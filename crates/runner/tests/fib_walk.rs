//! The walker's rules, each proved by a test that fails without the rule
//! (FIB-5; D-6380 to D-6385). Every fixture is synthetic: prices near
//! Rs 1,000.00 on an invented day, volumes chosen to make the rule visible.
//! Nothing here is a market fact.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    reason = "a test that cannot panic cannot fail"
)]

use indicators::fibflag::types::{EntryStyle, FibCandle};
use indicators::{Candle, OI_NULL};
use runner::fib::types::{
    BeyondTarget, ClockRule, CrossFeed, ExitFill, ExitReason, HaltRule, PriceMap, RMultiple,
    Resolution, SlotOutcome, SlotRefusal, SlotStatus, SnapshotEdge, TradeFlag, TradeSize,
    UnfilledReason, UnpricedReason, WalkProfile,
};
use runner::fib::walk::exits::{
    EntryWindow, ExitSet, ExitVariant, FillVariant, Participation, PassBudget, Sample, SeedSamples,
    StopLoss, TakeProfit, TimeExit, TrailingStop, TrailingTakeProfit, seed_exit_set, seed_fill_set,
};
use runner::fib::walk::{
    BookEvidence, BookMark, ClockLag, DayIndex, DayInput, HaltRange, Plan, TrailCandles,
    WalkRefusal, Walked, missing_pairs, walk_all, walk_pairs,
};
use vocab::fib_cube::Search;

/// An invented IST day number.
const DAY: i64 = 19_723;
/// Rs 1,000.00, the fixtures' centre price.
const P: i64 = 100_000;
/// The fixtures' tick, decision and raw.
const TICK: i64 = 5;

/// Seconds after IST midnight of `h:m:s`.
const fn at(h: u32, m: u32, s: u32) -> u32 {
    h * 3_600 + m * 60 + s
}

/// The UTC microseconds of an IST second of [`DAY`].
fn ts(second: u32) -> i64 {
    (DAY * 86_400 + i64::from(second) - 19_800) * 1_000_000
}

/// One bar.
fn bar(second: u32, open: i64, high: i64, low: i64, close: i64, volume: i64) -> Candle {
    Candle::new(ts(second), open, high, low, close, volume, OI_NULL)
}

/// A quiet bar around `price`: ten paisa either side, so never flat.
fn quiet(second: u32, price: i64, volume: i64) -> Candle {
    bar(second, price, price + 10, price - 10, price, volume)
}

/// One complete one-minute trail candle from a minute bar.
fn candle_of(minute: &Candle) -> FibCandle {
    FibCandle {
        candle: *minute,
        tf_minutes: 1,
        complete: true,
        missing_minutes: 0,
        close_ts_micros: minute.ts_micros + 60_000_000,
    }
}

/// The deciding readings, stage A.
fn minute_profile() -> WalkProfile {
    WalkProfile {
        resolution: Resolution::Minute,
        price_map: PriceMap::Identity,
        exit_fill: ExitFill::SpillByVolume,
        snapshot_edge: SnapshotEdge::StampOpensInterval,
        halt_rule: HaltRule::WorstSeenThroughHalt,
        cross_feed: CrossFeed::SecondsOnly,
        beyond_target: BeyondTarget::FiresOnFirstBar,
        clock_rule: ClockRule::RefuseShifted,
    }
}

/// The same readings, deep, factor one.
fn second_profile() -> WalkProfile {
    WalkProfile {
        resolution: Resolution::Second,
        ..minute_profile()
    }
}

/// One stock-day's inputs, owned.
#[derive(Clone)]
struct Day {
    bars: Vec<Candle>,
    marks: Option<Vec<BookMark>>,
    halts: Vec<HaltRange>,
    zerodha: Vec<Candle>,
    trail: Vec<FibCandle>,
    max_gap: u32,
    lag: ClockLag,
    raw_tick: i64,
}

impl Day {
    /// Minute bars; their own trail candles.
    fn minutes(bars: Vec<Candle>) -> Self {
        let trail = bars.iter().map(candle_of).collect();
        Self {
            bars,
            marks: None,
            halts: Vec::new(),
            zerodha: Vec::new(),
            trail,
            max_gap: u32::MAX,
            lag: ClockLag::Measured(0),
            raw_tick: TICK,
        }
    }

    /// Second bars, with a quiet Zerodha minute over each second's minute.
    fn seconds(bars: Vec<Candle>) -> Self {
        let mut zerodha: Vec<Candle> = Vec::new();
        for b in &bars {
            let minute = b.ts_micros - b.ts_micros.rem_euclid(60_000_000);
            match zerodha.last_mut() {
                Some(z) if z.ts_micros == minute => {
                    z.high = z.high.max(b.high);
                    z.low = z.low.min(b.low);
                    z.volume += b.volume.max(1);
                }
                _ => zerodha.push(Candle::new(
                    minute,
                    b.open,
                    b.high,
                    b.low,
                    b.close,
                    b.volume.max(1),
                    OI_NULL,
                )),
            }
        }
        Self {
            bars,
            marks: None,
            halts: Vec::new(),
            zerodha,
            trail: Vec::new(),
            max_gap: u32::MAX,
            lag: ClockLag::Measured(0),
            raw_tick: TICK,
        }
    }

    fn index(&self, profile: &WalkProfile, exits: &ExitSet) -> Result<DayIndex, WalkRefusal> {
        let trails = [TrailCandles {
            tf_minutes: 1,
            candles: &self.trail,
        }];
        let book = match &self.marks {
            Some(marks) => BookEvidence::Observed(marks),
            None => BookEvidence::NotObserved,
        };
        let input = DayInput {
            bars: &self.bars,
            book,
            halts: &self.halts,
            zerodha_minutes: &self.zerodha,
            raw_tick: self.raw_tick,
            decision_tick: TICK,
            max_second_gap: self.max_gap,
            clock_lag: self.lag,
            trail: &trails,
        };
        DayIndex::build(profile, &input, exits)
    }
}

/// A one-minute CLOSE long decided at `decision`: entry reference [`P`], base
/// edge Rs 990.00 (planned stop Rs 989.95, decision risk 1,005 paisa), day
/// range Rs 995.00 to Rs 1,010.00.
fn long_plan(decision: u32) -> Plan {
    Plan {
        direction: Search::Long,
        style: EntryStyle::Close,
        tf_minutes: 1,
        decision_second: decision,
        entry_window_end: decision + 60,
        entry_ref: P,
        base_edge: 99_000,
        trigger_edge: P,
        fib_zero: 99_500,
        fib_one: 101_000,
        trail: 0,
    }
}

/// A variant from its stop and a change of the reference.
fn with_stop(variant: ExitVariant) -> ExitVariant {
    ExitVariant {
        stop: StopLoss::BeyondBase { extra_ticks: 0 },
        ..variant
    }
}

/// The stop loss alone.
const STOP_ONLY: ExitVariant = ExitVariant {
    stop: StopLoss::BeyondBase { extra_ticks: 0 },
    ..ExitVariant::REFERENCE
};

fn set_of(direction: Search, variants: Vec<ExitVariant>) -> ExitSet {
    ExitSet::new(direction, 1, 0, variants).expect("a valid set")
}

/// One slot of one plan under one variant.
fn walk_one(
    day: &Day,
    profile: &WalkProfile,
    plan: Plan,
    variant: ExitVariant,
    fill: FillVariant,
    size: TradeSize,
) -> SlotOutcome {
    let exits = set_of(plan.direction, vec![variant]);
    let index = day.index(profile, &exits).expect("the day indexes");
    let walked = walk_pairs(&index, &[plan], &exits, fill, &[size], &[(0, 0)]).expect("walks");
    walked[0].slots[0]
}

/// One minute slot at the small size.
fn minute_slot(day: &Day, plan: Plan, variant: ExitVariant) -> SlotOutcome {
    walk_one(
        day,
        &minute_profile(),
        plan,
        variant,
        FillVariant::STAGE_A,
        TradeSize::Small,
    )
}

/// One second slot at the small size, with reading 1 and no cap.
fn second_slot(day: &Day, plan: Plan, variant: ExitVariant) -> SlotOutcome {
    walk_one(
        day,
        &second_profile(),
        plan,
        variant,
        FillVariant::STAGE_A,
        TradeSize::Small,
    )
}

fn has(outcome: &SlotOutcome, flag: TradeFlag) -> bool {
    outcome.flags.contains(flag)
}

/// The R of a long outcome.
fn r_long(outcome: &SlotOutcome) -> RMultiple {
    outcome.r_multiple(Search::Long).expect("a priced long")
}

fn minus_one() -> RMultiple {
    RMultiple::whole(-1)
}

/// The day's minute bars from 09:59 with a quiet 15:09 minute.
fn minute_day(after: &[Candle]) -> Day {
    let mut bars = vec![quiet(at(9, 59, 0), P - 50, 1_000)];
    bars.extend_from_slice(after);
    bars.push(quiet(at(15, 9, 0), P, 1_000));
    Day::minutes(bars)
}

// ---------------------------------------------------------------- stops --

#[test]
fn a_gap_through_the_stop_exits_at_the_bar_low_with_r_below_minus_one() {
    let day = minute_day(&[
        bar(at(10, 0, 0), P, P + 100, P - 50, P + 50, 1_000),
        bar(at(10, 1, 0), 98_500, 98_600, 98_000, 98_100, 1_000),
    ]);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(out.status, SlotStatus::Priced);
    assert_eq!(out.exit_reason, ExitReason::StopLoss);
    assert_eq!(out.filled_qty, 100);
    assert_eq!(out.entry_value_paisa, i128::from(100 * (P + 100)));
    assert_eq!(out.exit_value_paisa, i128::from(100 * 98_000));
    assert!(has(&out, TradeFlag::GapThroughStop));
    assert!(has(&out, TradeFlag::FiredStopLoss));
    let r = r_long(&out);
    assert_eq!(
        r,
        RMultiple {
            num: -210_000,
            den: 100_500
        }
    );
    assert_eq!(r.cmp_exact(minus_one()), Some(core::cmp::Ordering::Less));
}

#[test]
fn entry_and_stop_in_one_minute_stop_in_that_minute_at_its_low() {
    let day = minute_day(&[bar(at(10, 0, 0), P, P + 100, 98_900, 99_000, 1_000)]);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(out.exit_reason, ExitReason::StopLoss);
    assert_eq!(out.first_entry_second, at(10, 0, 0));
    assert_eq!(out.exit_second, at(10, 0, 0));
    assert_eq!(out.exit_value_paisa, i128::from(100 * 98_900));
    assert!(
        !has(&out, TradeFlag::GapThroughStop),
        "it opened above the stop"
    );
}

#[test]
fn entry_and_stop_in_one_second_stop_in_that_second_at_its_low() {
    let day = Day::seconds(vec![
        bar(at(10, 0, 0), P, P + 100, 98_900, 99_000, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    let out = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(out.exit_reason, ExitReason::StopLoss);
    assert_eq!(out.exit_second, at(10, 0, 0));
    assert_eq!(out.entry_value_paisa, i128::from(100 * (P + 100)));
    assert_eq!(out.exit_value_paisa, i128::from(100 * 98_900));
    let own = out.own_exit.expect("one own order");
    assert_eq!((own.filled_qty, own.last_fill_second), (100, at(10, 0, 0)));
    assert_eq!(out.forced_exit, None);
}

#[test]
fn an_entry_minute_wholly_below_the_stop_is_stopped_with_a_finite_negative_r() {
    let day = minute_day(&[bar(at(10, 0, 0), 98_850, 98_900, 98_800, 98_850, 1_000)]);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(out.status, SlotStatus::Priced, "kept in statistics");
    assert!(has(&out, TradeFlag::EntryBeyondStop));
    assert_eq!(
        r_long(&out),
        RMultiple {
            num: -10_000,
            den: 100_500
        }
    );
}

#[test]
fn an_entry_one_paisa_above_the_stop_exiting_one_percent_up_is_about_one_r() {
    // FM-F033: R's denominator is the PLANNED risk (entry_ref less the planned
    // stop), never the filled risk of six paisa, which would read 165R.
    let entry = 98_996;
    let day = minute_day(&[
        bar(at(10, 0, 0), entry, entry, entry, entry, 1_000),
        bar(at(10, 1, 0), 99_990, 99_995, 99_986, 99_990, 1_000),
    ]);
    let variant = ExitVariant {
        time_exit: TimeExit::Clock {
            second: at(10, 1, 0),
        },
        ..STOP_ONLY
    };
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert_eq!(out.exit_reason, ExitReason::Time);
    assert_eq!(out.exit_value_paisa, i128::from(100 * 99_986));
    assert_eq!(
        r_long(&out),
        RMultiple {
            num: 99_000,
            den: 100_500
        }
    );
    assert_eq!(out.risk_per_share_paisa, 1_005);
}

#[test]
fn a_stop_absent_still_measures_r_from_the_planned_stop() {
    let day = minute_day(&[
        bar(at(10, 0, 0), P, P + 100, 98_000, P, 1_000),
        quiet(at(10, 1, 0), P + 500, 1_000),
    ]);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), ExitVariant::REFERENCE);
    assert_eq!(
        out.exit_reason,
        ExitReason::Forced,
        "no stop order: only forced"
    );
    assert_eq!(out.risk_num, 1_005);
    assert_eq!(out.risk_den, 1);
    assert_eq!(out.exit_value_paisa, i128::from(100 * (P - 10)));
    assert_eq!(
        r_long(&out),
        RMultiple {
            num: -11_000,
            den: 100_500
        }
    );
}

#[test]
fn a_tiny_structural_stop_walks_unchanged_and_is_judged_only_by_the_gate() {
    // 0.15% of the entry reference: no floor, no refusal (17:15 item 8).
    let mut plan = long_plan(at(10, 0, 0));
    plan.base_edge = P - 150 + TICK;
    let day = minute_day(&[
        bar(at(10, 0, 0), P, P + 20, P - 20, P, 1_000),
        bar(at(10, 1, 0), P, P + 20, P - 160, P, 1_000),
    ]);
    let out = minute_slot(&day, plan, STOP_ONLY);
    assert_eq!(out.risk_per_share_paisa, 150);
    assert_eq!(out.exit_reason, ExitReason::StopLoss);
    assert_eq!(out.exit_value_paisa, i128::from(100 * (P - 160)));
}

// --------------------------------------------------------------- forced --

/// A long entered at 10:00:00 on seconds, then whatever `tail` holds.
fn second_day(tail: &[Candle]) -> Day {
    let mut bars = vec![bar(at(10, 0, 0), P, P + 100, P - 50, P + 50, 10_000)];
    bars.extend_from_slice(tail);
    Day::seconds(bars)
}

#[test]
fn a_trade_open_at_15_09_59_with_no_print_in_the_window_is_unpriceable() {
    let day = second_day(&[quiet(at(15, 8, 59), P, 10_000)]);
    let out = second_slot(&day, long_plan(at(10, 0, 0)), ExitVariant::REFERENCE);
    assert_eq!(
        out.status,
        SlotStatus::Unpriceable(UnpricedReason::ForcedThin)
    );
    assert!(has(&out, TradeFlag::DhanSquareOff));
    assert_eq!(out.r_multiple(Search::Long), None, "never priced");
}

#[test]
fn a_print_only_at_15_10_00_is_never_read_and_the_trade_stays_unpriceable() {
    let day = second_day(&[quiet(at(15, 10, 0), P, 10_000)]);
    let exits = set_of(Search::Long, vec![ExitVariant::REFERENCE]);
    let index = day.index(&second_profile(), &exits).expect("indexes");
    assert_eq!(index.dropped_after_auto_exit(), 1);
    assert_eq!(index.bars(), 1);
    let out = second_slot(&day, long_plan(at(10, 0, 0)), ExitVariant::REFERENCE);
    assert_eq!(
        out.status,
        SlotStatus::Unpriceable(UnpricedReason::ForcedThin)
    );
}

#[test]
fn prints_in_the_window_with_too_little_volume_are_unpriceable() {
    let day = second_day(&[quiet(at(15, 9, 0), P, 60), quiet(at(15, 9, 30), P, 39)]);
    let out = second_slot(&day, long_plan(at(10, 0, 0)), ExitVariant::REFERENCE);
    assert_eq!(out.planned_qty, 100);
    assert_eq!(
        out.status,
        SlotStatus::Unpriceable(UnpricedReason::ForcedThin)
    );
    let enough = second_day(&[quiet(at(15, 9, 0), P, 60), quiet(at(15, 9, 30), P, 40)]);
    let out = second_slot(&enough, long_plan(at(10, 0, 0)), ExitVariant::REFERENCE);
    assert_eq!(
        out.status,
        SlotStatus::Priced,
        "the summed volume holds 100"
    );
}

#[test]
fn the_forced_price_is_the_lowest_low_of_the_window_not_the_last_second() {
    let day = second_day(&[
        bar(at(15, 9, 0), P, P + 10, 99_500, P, 100),
        bar(at(15, 9, 20), P, P + 10, 99_200, P, 100),
        bar(at(15, 9, 59), P, P + 10, 99_800, P, 100),
    ]);
    let out = second_slot(&day, long_plan(at(10, 0, 0)), ExitVariant::REFERENCE);
    assert_eq!(out.status, SlotStatus::Priced);
    assert_eq!(out.exit_reason, ExitReason::Forced);
    assert_eq!(out.exit_value_paisa, i128::from(100 * 99_200));
    let forced = out.forced_exit.expect("the forced order");
    assert_eq!(forced.filled_qty, 100);
    assert_eq!(forced.last_fill_second, at(15, 9, 59));
    assert!(has(&out, TradeFlag::DhanSquareOff));
    assert_eq!(out.mae_to_exit_paisa, (P + 100) - 99_200);
    assert_eq!(out.mae_to_horizon_paisa, (P + 100) - 99_200);
}

#[test]
fn a_short_s_forced_price_is_the_highest_high_of_the_window() {
    let mut plan = long_plan(at(10, 0, 0));
    plan.direction = Search::Short;
    plan.base_edge = 101_000;
    plan.fib_zero = 100_500;
    plan.fib_one = 99_000;
    let day = Day::seconds(vec![
        bar(at(10, 0, 0), P, P + 50, P - 100, P - 50, 10_000),
        bar(at(15, 9, 0), P, 100_500, P - 10, P, 100),
        bar(at(15, 9, 20), P, 100_800, P - 10, P, 100),
        bar(at(15, 9, 59), P, 100_200, P - 10, P, 100),
    ]);
    let out = second_slot(&day, plan, ExitVariant::REFERENCE);
    assert_eq!(out.status, SlotStatus::Priced);
    assert_eq!(
        out.entry_value_paisa,
        i128::from(100 * (P - 100)),
        "a sell at the low"
    );
    assert_eq!(
        out.exit_value_paisa,
        i128::from(100 * 100_800),
        "a buy at the window's high"
    );
}

#[test]
fn a_one_sided_window_is_locked_and_a_thin_one_thin() {
    let mut plan = long_plan(at(10, 0, 0));
    plan.direction = Search::Short;
    plan.base_edge = 101_000;
    plan.fib_zero = 100_500;
    plan.fib_one = 99_000;
    let bars = vec![
        bar(at(10, 0, 0), P, P + 50, P - 100, P - 50, 10_000),
        quiet(at(15, 9, 0), P, 1_000),
        quiet(at(15, 9, 1), P, 1_000),
    ];
    let both = BookMark {
        ask: true,
        bid: true,
        flagged_print: false,
    };
    let bid_only = BookMark { ask: false, ..both };
    let mut day = Day::seconds(bars);
    day.marks = Some(vec![both, bid_only, bid_only]);
    let out = second_slot(&day, plan, ExitVariant::REFERENCE);
    assert_eq!(
        out.status,
        SlotStatus::Unpriceable(UnpricedReason::ForcedLockedShort)
    );
    assert_eq!(out.status.name(), "unpriceable_locked_short");
    day.marks = Some(vec![both, both, both]);
    day.bars[1].volume = 10;
    day.bars[2].volume = 10;
    let out = second_slot(&day, plan, ExitVariant::REFERENCE);
    assert_eq!(
        out.status,
        SlotStatus::Unpriceable(UnpricedReason::ForcedThin)
    );
    assert_eq!(out.status.name(), "unpriceable_thin");
    let mut long_day = second_day(&[quiet(at(15, 9, 0), P, 1_000)]);
    let ask_only = BookMark { bid: false, ..both };
    long_day.marks = Some(vec![both, ask_only]);
    let out = second_slot(&long_day, long_plan(at(10, 0, 0)), ExitVariant::REFERENCE);
    assert_eq!(
        out.status,
        SlotStatus::Unpriceable(UnpricedReason::ForcedLockedLong)
    );
}

#[test]
fn a_minute_forced_exit_is_the_15_09_minute_s_low_and_carries_the_square_off() {
    let day = minute_day(&[bar(at(10, 0, 0), P, P + 100, P - 50, P, 1_000)]);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), ExitVariant::REFERENCE);
    assert_eq!(out.status, SlotStatus::Priced);
    assert_eq!(out.exit_reason, ExitReason::Forced);
    assert_eq!(out.exit_value_paisa, i128::from(100 * (P - 10)));
    assert_eq!(out.exit_second, at(15, 9, 0));
    assert!(has(&out, TradeFlag::DhanSquareOff));
}

#[test]
fn a_minute_walk_never_reads_15_10_or_later_and_a_missing_15_09_is_cheap_unpriceable() {
    let bars = vec![
        quiet(at(9, 59, 0), P - 50, 1_000),
        bar(at(10, 0, 0), P, P + 100, P - 50, P, 1_000),
        quiet(at(15, 6, 0), P, 1_000),
        quiet(at(15, 12, 0), P + 9_000, 1_000),
    ];
    let day = Day::minutes(bars);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), ExitVariant::REFERENCE);
    assert_eq!(
        out.status,
        SlotStatus::Unpriceable(UnpricedReason::ForcedCheap)
    );
    assert_eq!(out.status.name(), "forced_exit_unpriceable_cheap");
    assert_eq!(out.mfe_to_horizon_paisa, 0, "the 15:12 bar is never read");
}

#[test]
fn a_decision_whose_window_is_wholly_missing_is_entry_unfilled() {
    // FM-F039: a 15-minute plan decided at 10:00 with 10:00 to 10:44 missing.
    let mut plan = long_plan(at(10, 0, 0));
    plan.tf_minutes = 1;
    plan.entry_window_end = at(10, 15, 0);
    let day = minute_day(&[quiet(at(10, 45, 0), P, 1_000)]);
    let out = minute_slot(&day, plan, STOP_ONLY);
    assert_eq!(
        out.status,
        SlotStatus::EntryUnfilled(UnfilledReason::WindowExpired)
    );
    assert_eq!(out.filled_qty, 0);
}

#[test]
fn a_missing_minute_and_a_missing_second_are_waited_for() {
    let mut plan = long_plan(at(10, 0, 0));
    plan.entry_window_end = at(10, 15, 0);
    let day = minute_day(&[quiet(at(10, 1, 0), P, 1_000)]);
    let out = minute_slot(&day, plan, STOP_ONLY);
    assert_eq!(out.first_entry_second, at(10, 1, 0));
    assert_eq!(out.entry_wait_seconds, 60);
    let day = Day::seconds(vec![
        quiet(at(10, 0, 3), P, 10_000),
        quiet(at(15, 9, 0), P, 10_000),
    ]);
    let out = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(out.first_entry_second, at(10, 0, 3));
    assert_eq!(out.entry_wait_seconds, 3);
}

// ---------------------------------------------------------------- fills --

#[test]
fn an_entry_spills_over_three_seconds_smaller_than_the_quantity() {
    let day = Day::seconds(vec![
        bar(at(10, 0, 0), P, P + 10, P - 10, P, 30),
        bar(at(10, 0, 1), P, P + 20, P - 10, P, 30),
        bar(at(10, 0, 2), P, P + 30, P - 10, P, 50),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    let out = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(out.filled_qty, 100);
    assert_eq!(
        out.entry_value_paisa,
        i128::from(30 * (P + 10) + 30 * (P + 20) + 40 * (P + 30))
    );
    assert_eq!(out.first_entry_second, at(10, 0, 0));
    assert_eq!(out.last_entry_second, at(10, 0, 2));
    assert_eq!(out.max_participation_permille, 1_000);
    assert!(!has(&out, TradeFlag::PartialEntry));
}

#[test]
fn reading_one_cancels_the_remainder_at_the_next_close_and_reading_two_fills_on() {
    let bars = vec![
        bar(at(10, 0, 0), P, P + 10, P - 10, P, 30),
        bar(at(10, 1, 0), P, P + 20, P - 10, P, 100),
        quiet(at(15, 9, 0), P, 1_000),
    ];
    let day = Day::seconds(bars);
    let one = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(one.filled_qty, 30);
    assert!(has(&one, TradeFlag::PartialEntry));
    let two = walk_one(
        &day,
        &second_profile(),
        long_plan(at(10, 0, 0)),
        STOP_ONLY,
        FillVariant::REFERENCE,
        TradeSize::Small,
    );
    assert_eq!(two.filled_qty, 100);
    assert_eq!(
        two.entry_value_paisa,
        i128::from(30 * (P + 10) + 70 * (P + 20))
    );
    let deadline = FillVariant {
        window: EntryWindow::Deadline { seconds: 30 },
        participation: Participation::NoCap,
    };
    let three = walk_one(
        &day,
        &second_profile(),
        long_plan(at(10, 0, 0)),
        STOP_ONLY,
        deadline,
        TradeSize::Small,
    );
    assert_eq!(three.filled_qty, 30);
}

#[test]
fn a_participation_cap_takes_its_share_of_each_second() {
    let day = Day::seconds(vec![
        bar(at(10, 0, 0), P, P + 10, P - 10, P, 99),
        bar(at(10, 0, 1), P, P + 20, P - 10, P, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    let cap = FillVariant {
        window: EntryWindow::UntilForced,
        participation: Participation::Cap { permille: 250 },
    };
    let out = walk_one(
        &day,
        &second_profile(),
        long_plan(at(10, 0, 0)),
        STOP_ONLY,
        cap,
        TradeSize::Small,
    );
    // floor(250 x 99 / 1000) = 24, then 76 of the next second's 250.
    assert_eq!(
        out.entry_value_paisa,
        i128::from(24 * (P + 10) + 76 * (P + 20))
    );
    assert_eq!(out.max_participation_permille, 242);
}

#[test]
fn a_book_with_no_ask_supplies_nothing_and_the_fill_moves_to_the_next_second() {
    let both = BookMark {
        ask: true,
        bid: true,
        flagged_print: false,
    };
    let mut day = Day::seconds(vec![
        bar(at(10, 0, 0), P, P + 10, P - 10, P, 1_000),
        bar(at(10, 0, 1), P, P + 20, P - 10, P, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    day.marks = Some(vec![BookMark { ask: false, ..both }, both, both]);
    let out = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(out.first_entry_second, at(10, 0, 1));
    assert_eq!(out.entry_value_paisa, i128::from(100 * (P + 20)));
    assert!(has(&out, TradeFlag::OneSidedBook));
}

#[test]
fn a_stage_a_buy_into_flat_day_high_minutes_is_unfilled() {
    let day = minute_day(&[
        bar(at(10, 0, 0), P + 200, P + 200, P + 200, P + 200, 1_000),
        quiet(at(10, 1, 0), P, 1_000),
    ]);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(
        out.status,
        SlotStatus::EntryUnfilled(UnfilledReason::WindowExpired)
    );
    assert!(has(&out, TradeFlag::LockedMinuteSuspected));
}

#[test]
fn a_stop_into_flat_day_low_minutes_exits_at_the_first_unlocked_minute() {
    let day = minute_day(&[
        bar(at(10, 0, 0), P, P + 100, P - 50, P, 1_000),
        bar(at(10, 1, 0), 98_000, 98_000, 98_000, 98_000, 1_000),
        bar(at(10, 2, 0), 97_900, 97_900, 97_900, 97_900, 1_000),
        bar(at(10, 3, 0), 97_950, 98_000, 97_850, 97_950, 1_000),
    ]);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(out.exit_reason, ExitReason::StopLoss);
    assert_eq!(out.exit_second, at(10, 3, 0));
    assert_eq!(out.exit_value_paisa, i128::from(100 * 97_850));
    assert!(has(&out, TradeFlag::LockedMinuteSuspected));
    assert_eq!(out.exit_spill_seconds, 2);
}

#[test]
fn single_minute_legs_are_the_cost_crate_s_printed_extreme_fills() {
    use brutex_core::price::Paisa;
    use costs::fill::{Anchor, Bar, Direction, fills_at};
    let entry = bar(at(10, 0, 0), P, P + 100, P - 50, P, 1_000);
    let exit = bar(at(10, 1, 0), 98_500, 98_600, 98_000, 98_100, 1_000);
    let day = minute_day(&[entry, exit]);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    let leg = |c: &Candle| {
        Bar::new(
            Paisa::from_raw(c.open),
            Paisa::from_raw(c.high),
            Paisa::from_raw(c.low),
        )
        .expect("a valid bar")
    };
    let fills = fills_at(
        leg(&entry),
        leg(&exit),
        Direction::Long,
        Anchor::PrintedExtreme,
    )
    .expect("fills");
    assert_eq!(i128::from(fills.buy().raw()) * 100, out.entry_value_paisa);
    assert_eq!(i128::from(fills.sell().raw()) * 100, out.exit_value_paisa);
}
