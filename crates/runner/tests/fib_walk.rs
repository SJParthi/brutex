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

/// One bar, refused unless it is evaluable: a fixture typo must never pass as
/// a corrupt bar.
fn bar(second: u32, open: i64, high: i64, low: i64, close: i64, volume: i64) -> Candle {
    let candle = corrupt(second, open, high, low, close, volume);
    assert_eq!(candle.check_evaluable(), Ok(()), "fixture bar at {second}");
    candle
}

/// One bar as given, evaluable or not.
fn corrupt(second: u32, open: i64, high: i64, low: i64, close: i64, volume: i64) -> Candle {
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

// -------------------------------------------------------------- targets --

/// A take-profit at one R: `P + 1,005`.
const ONE_R: TakeProfit = TakeProfit::RMultiple { milli_r: 1_000 };

#[test]
fn a_minute_target_fills_only_when_the_minute_trades_through_it() {
    let day = minute_day(&[
        bar(at(10, 0, 0), P, P + 100, P - 50, P, 1_000),
        bar(at(10, 1, 0), P, 101_005, P, P, 1_000),
        bar(at(10, 2, 0), P, 101_010, P, P, 1_000),
    ]);
    let variant = with_stop(ExitVariant {
        take_profit: ONE_R,
        ..ExitVariant::REFERENCE
    });
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert_eq!(out.exit_reason, ExitReason::TakeProfit);
    assert_eq!(
        out.exit_second,
        at(10, 2, 0),
        "a touch is not a trade-through"
    );
    assert_eq!(
        out.exit_value_paisa,
        i128::from(100 * 101_005),
        "at the limit"
    );
    assert!(!has(&out, TradeFlag::TargetBookedAtWorst));
}

#[test]
fn a_minute_limit_off_the_tick_grid_rests_at_the_next_tick_above() {
    // Half an R is 502.5 paisa: the limit rests at Rs 1,005.05.
    let day = minute_day(&[
        bar(at(10, 0, 0), P, P + 100, P - 50, P, 1_000),
        bar(at(10, 1, 0), P, 100_505, P, P, 1_000),
        bar(at(10, 2, 0), P, 100_506, P, P, 1_000),
    ]);
    let variant = ExitVariant {
        take_profit: TakeProfit::RMultiple { milli_r: 500 },
        ..STOP_ONLY
    };
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert_eq!(out.exit_second, at(10, 2, 0));
    assert_eq!(out.exit_value_paisa, i128::from(100 * 100_505));
}

#[test]
fn a_second_target_is_booked_at_the_touching_second_s_low() {
    let day = second_day(&[
        bar(at(10, 0, 5), 100_950, 101_010, 100_900, 100_950, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    let variant = with_stop(ExitVariant {
        take_profit: ONE_R,
        ..ExitVariant::REFERENCE
    });
    let out = second_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert_eq!(out.exit_reason, ExitReason::TakeProfit);
    assert_eq!(out.exit_value_paisa, i128::from(100 * 100_900));
    assert!(has(&out, TradeFlag::TargetBookedAtWorst));
}

#[test]
fn a_target_and_a_stop_in_one_bar_is_the_stop_at_the_bar_s_low() {
    let variant = with_stop(ExitVariant {
        take_profit: ONE_R,
        ..ExitVariant::REFERENCE
    });
    let both = bar(at(10, 1, 0), P, 101_100, 98_000, P, 1_000);
    let day = minute_day(&[bar(at(10, 0, 0), P, P + 100, P - 50, P, 1_000), both]);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert_eq!(out.exit_reason, ExitReason::StopLoss);
    assert_eq!(out.exit_value_paisa, i128::from(100 * 98_000));
    assert!(has(&out, TradeFlag::FiredStopLoss));
    assert!(has(&out, TradeFlag::FiredTakeProfit));
    let day = second_day(&[
        bar(at(10, 1, 0), P, 101_100, 98_000, P, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    let out = second_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert_eq!(out.exit_reason, ExitReason::StopLoss);
    assert_eq!(out.exit_value_paisa, i128::from(100 * 98_000));
}

#[test]
fn a_target_touched_before_the_first_fill_leaves_the_entry_unfilled() {
    let both = BookMark {
        ask: true,
        bid: true,
        flagged_print: false,
    };
    let mut day = Day::seconds(vec![
        bar(at(10, 0, 0), P, 101_100, P, P, 1_000),
        quiet(at(10, 0, 1), P, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    day.marks = Some(vec![BookMark { ask: false, ..both }, both, both]);
    let variant = ExitVariant {
        take_profit: ONE_R,
        ..STOP_ONLY
    };
    let out = second_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert_eq!(
        out.status,
        SlotStatus::EntryUnfilled(UnfilledReason::TargetBeforeFill)
    );
}

#[test]
fn a_fill_beyond_the_target_runs_both_readings_and_is_flagged() {
    let day = second_day(&[
        bar(at(10, 0, 1), 100_500, 100_600, 100_400, 100_500, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    // A target of 0.05 R (Rs 1,000.5025) sits below the fill at Rs 1,001.00.
    let variant = ExitVariant {
        take_profit: TakeProfit::RMultiple { milli_r: 50 },
        ..ExitVariant::REFERENCE
    };
    let fires = second_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert!(has(&fires, TradeFlag::EntryBeyondTarget));
    assert_eq!(fires.exit_reason, ExitReason::TakeProfit);
    assert_eq!(fires.exit_second, at(10, 0, 0), "it fires on the first bar");
    assert_eq!(fires.exit_value_paisa, i128::from(100 * (P - 50)));
    let void = walk_one(
        &day,
        &WalkProfile {
            beyond_target: BeyondTarget::TargetVoid,
            ..second_profile()
        },
        long_plan(at(10, 0, 0)),
        variant,
        FillVariant::STAGE_A,
        TradeSize::Small,
    );
    assert!(has(&void, TradeFlag::EntryBeyondTarget));
    assert_eq!(void.exit_reason, ExitReason::Forced, "the target is void");
}

// --------------------------------------------------------------- trails --

#[test]
fn the_swing_trail_waits_for_its_confirming_close_and_applies_from_the_next_bar() {
    let day = minute_day(&[
        bar(at(10, 0, 0), P, P + 100, 99_950, P + 50, 1_000),
        bar(at(10, 1, 0), P + 100, 100_200, 100_060, P + 100, 1_000),
        bar(at(10, 2, 0), P + 100, 100_150, 100_020, P + 100, 1_000),
        bar(at(10, 3, 0), P + 100, 100_300, 100_080, P + 100, 1_000),
        bar(at(10, 4, 0), P + 100, 100_250, 100_010, P + 100, 1_000),
    ]);
    let variant = ExitVariant {
        trailing_stop: TrailingStop::Swing {
            width: 1,
            buffer_paisa: 0,
        },
        ..ExitVariant::REFERENCE
    };
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert_eq!(out.exit_reason, ExitReason::TrailingStopLoss);
    assert_eq!(out.exit_second, at(10, 4, 0));
    assert_eq!(out.exit_value_paisa, i128::from(100 * 100_010));
    // A buffer below the swing keeps the 10:04 low above the trail.
    let buffered = ExitVariant {
        trailing_stop: TrailingStop::Swing {
            width: 1,
            buffer_paisa: 6,
        },
        ..ExitVariant::REFERENCE
    };
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), buffered);
    assert_eq!(out.exit_reason, ExitReason::Forced);
}

#[test]
fn a_swing_across_a_withheld_candle_is_a_hole_never_a_trail() {
    // FM-F045: lows of 100, (withheld), 90, 95 above a base.
    let day = minute_day(&[
        bar(at(10, 0, 0), 100_100, 100_200, 100_050, 100_100, 1_000),
        bar(at(10, 1, 0), 100_150, 100_200, 100_100, 100_150, 1_000),
        bar(at(10, 3, 0), 100_150, 100_200, 100_090, 100_150, 1_000),
        bar(at(10, 4, 0), 100_150, 100_200, 100_095, 100_150, 1_000),
        quiet(at(10, 6, 0), 100_150, 1_000),
    ]);
    let variant = ExitVariant {
        trailing_stop: TrailingStop::Swing {
            width: 1,
            buffer_paisa: 0,
        },
        ..ExitVariant::REFERENCE
    };
    let exits = set_of(Search::Long, vec![variant]);
    let index = day.index(&minute_profile(), &exits).expect("indexes");
    assert_eq!(index.trail_holes(), 1);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert!(has(&out, TradeFlag::TrailHole));
    assert_eq!(out.exit_reason, ExitReason::Forced, "no swing at 90");
}

#[test]
fn a_give_back_level_made_by_a_bar_applies_from_the_next_bar() {
    let variant = ExitVariant {
        trailing_take_profit: TrailingTakeProfit::GiveBack { permille: 500 },
        ..ExitVariant::REFERENCE
    };
    let day = minute_day(&[
        bar(at(10, 0, 0), P, P + 100, 99_950, P, 1_000),
        bar(at(10, 1, 0), 100_700, 101_100, 100_600, 101_000, 1_000),
        bar(at(10, 2, 0), 101_000, 102_100, 100_600, 102_000, 1_000),
        bar(at(10, 3, 0), 101_500, 101_500, 101_000, 101_200, 1_000),
    ]);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert_eq!(out.exit_reason, ExitReason::TrailingTakeProfit);
    assert_eq!(
        out.exit_second,
        at(10, 3, 0),
        "10:02's own level waits for 10:03"
    );
    assert_eq!(out.exit_value_paisa, i128::from(100 * 101_000));
    // A new-high minute whose low crosses the PRIOR level exits on it.
    let day = minute_day(&[
        bar(at(10, 0, 0), P, P + 100, 99_950, P, 1_000),
        bar(at(10, 1, 0), 100_700, 101_100, 100_600, 101_000, 1_000),
        bar(at(10, 2, 0), 101_000, 102_100, 100_500, 102_000, 1_000),
    ]);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert_eq!(out.exit_second, at(10, 2, 0));
    assert_eq!(out.exit_value_paisa, i128::from(100 * 100_500));
}

#[test]
fn an_r_step_earned_in_a_bar_is_not_hit_inside_that_bar() {
    let variant = ExitVariant {
        trailing_stop: TrailingStop::RStep { milli_r: 1_000 },
        ..ExitVariant::REFERENCE
    };
    let day = minute_day(&[
        bar(at(10, 0, 0), P, P + 100, 99_950, P, 1_000),
        bar(at(10, 1, 0), P, 101_100, 99_990, P, 1_000),
        bar(at(10, 2, 0), P, 100_500, 99_995, P, 1_000),
    ]);
    let out = minute_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert_eq!(out.exit_reason, ExitReason::TrailingStopLoss);
    assert_eq!(out.exit_second, at(10, 2, 0));
    assert_eq!(out.exit_value_paisa, i128::from(100 * 99_995));
}

// ----------------------------------------------------------- exit fills --

/// A long of 100 entered at 10:00:00, stopped at 10:01:00 by a one-share
/// second followed by three lower seconds.
fn spill_day(window_volume: i64) -> Day {
    second_day(&[
        bar(at(10, 1, 0), P, P, 98_990, 98_990, 1),
        bar(at(10, 1, 1), 98_990, 98_990, 98_980, 98_980, 30),
        bar(at(10, 1, 2), 98_980, 98_980, 98_970, 98_970, 30),
        bar(at(10, 1, 3), 98_970, 98_970, 98_960, 98_960, 100),
        quiet(at(15, 9, 0), P, window_volume),
    ])
}

#[test]
fn a_stop_spills_by_volume_under_one_reading_and_leaves_whole_under_the_other() {
    let day = spill_day(1_000);
    let spill = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(
        spill.exit_value_paisa,
        i128::from(98_990 + 30 * 98_980 + 30 * 98_970 + 39 * 98_960)
    );
    assert_eq!(spill.exit_spill_seconds, 3);
    let own = spill.own_exit.expect("one order, its spill inside it");
    assert_eq!((own.filled_qty, own.last_fill_second), (100, at(10, 1, 3)));
    let whole = walk_one(
        &day,
        &WalkProfile {
            exit_fill: ExitFill::FirstTouchWhole,
            ..second_profile()
        },
        long_plan(at(10, 0, 0)),
        STOP_ONLY,
        FillVariant::STAGE_A,
        TradeSize::Small,
    );
    assert_eq!(whole.exit_value_paisa, i128::from(100 * 98_990));
    assert_eq!(whole.exit_spill_seconds, 0);
}

#[test]
fn a_spill_still_open_at_15_09_goes_to_the_forced_rules() {
    let thin = second_day(&[
        bar(at(10, 1, 0), P, P, 98_990, 98_990, 1),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    let out = second_slot(&thin, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(out.status, SlotStatus::Priced);
    assert!(has(&out, TradeFlag::ExitSpillToForced));
    assert_eq!(out.own_exit.expect("own").filled_qty, 1);
    assert_eq!(out.forced_exit.expect("forced").filled_qty, 99);
    let starved = second_day(&[
        bar(at(10, 1, 0), P, P, 98_990, 98_990, 1),
        quiet(at(15, 9, 0), P, 98),
    ]);
    let out = second_slot(&starved, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(
        out.status,
        SlotStatus::Unpriceable(UnpricedReason::ForcedThin)
    );
}

#[test]
fn a_time_exit_partly_filled_spills_and_the_rest_is_forced_in_three_orders() {
    let day = second_day(&[
        quiet(at(15, 8, 0), P, 30),
        quiet(at(15, 8, 30), P, 20),
        bar(at(15, 9, 0), P, P + 10, 99_700, P, 1_000),
    ]);
    let variant = ExitVariant {
        time_exit: TimeExit::Clock {
            second: at(15, 8, 0),
        },
        ..STOP_ONLY
    };
    let out = second_slot(&day, long_plan(at(10, 0, 0)), variant);
    assert_eq!(out.status, SlotStatus::Priced);
    assert_eq!(out.exit_reason, ExitReason::Time);
    assert!(has(&out, TradeFlag::FiredTime));
    assert!(has(&out, TradeFlag::ExitSpillToForced));
    assert!(has(&out, TradeFlag::DhanSquareOff));
    let own = out.own_exit.expect("the time exit's order");
    let forced = out.forced_exit.expect("the square-off's order");
    assert_eq!((own.filled_qty, own.last_fill_second), (50, at(15, 8, 30)));
    assert_eq!(own.exit_value_paisa, i128::from(50 * (P - 10)));
    assert_eq!(
        (forced.filled_qty, forced.exit_value_paisa),
        (50, i128::from(50 * 99_700))
    );
    assert_eq!(
        out.exit_value_paisa,
        own.exit_value_paisa + forced.exit_value_paisa
    );
}

#[test]
fn an_exit_during_a_spilling_entry_cancels_the_rest_and_r_uses_the_filled_qty() {
    // FM-F036: 300 wanted, 100 filled, a stop in the next second.
    let mut plan = long_plan(at(10, 0, 0));
    plan.entry_ref = 33_333;
    plan.base_edge = 33_000;
    plan.trigger_edge = 33_333;
    let day = Day::seconds(vec![
        bar(at(10, 0, 0), 33_330, 33_340, 33_320, 33_330, 100),
        bar(at(10, 0, 1), 33_330, 33_330, 32_990, 33_000, 1_000),
        quiet(at(15, 9, 0), 33_000, 1_000),
    ]);
    let out = second_slot(&day, plan, STOP_ONLY);
    assert_eq!(out.planned_qty, 300);
    assert_eq!(out.filled_qty, 100);
    assert!(has(&out, TradeFlag::EntryCancelledByExit));
    assert_eq!(out.exit_value_paisa, i128::from(100 * 32_990));
    assert_eq!(
        r_long(&out),
        RMultiple {
            num: 100 * (32_990 - 33_340),
            den: 100 * 338
        }
    );
}

// --------------------------------------------------------------- halts --

/// A long entered at 10:00:00, quiet to 11:39:59, a halt over
/// `[11:40, 12:00)`, then `after` from 12:00:00.
fn halt_day(after: Candle) -> Day {
    let mut day = second_day(&[
        quiet(at(11, 39, 59), P, 1_000),
        quiet(at(11, 50, 0), P + 2_000, 1_000),
        after,
        quiet(at(12, 0, 1), P - 500, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    day.halts = vec![HaltRange {
        start_second: at(11, 40, 0),
        end_second: at(12, 0, 0),
    }];
    day
}

#[test]
fn an_entry_before_a_halt_is_kept_unpriced_and_one_after_it_is_not_tradable() {
    let day = halt_day(quiet(at(12, 0, 0), P - 500, 1_000));
    let out = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(out.status, SlotStatus::Unpriceable(UnpricedReason::Halt));
    assert!(has(&out, TradeFlag::Halt));
    assert_eq!(out.filled_qty, 100);
    assert_eq!(
        out.mae_to_horizon_paisa,
        (P + 100) - (P - 510),
        "seen through the halt"
    );
    assert_eq!(
        out.mfe_to_horizon_paisa, 0,
        "the halted print is never read"
    );
    let late = second_slot(&day, long_plan(at(11, 45, 0)), STOP_ONLY);
    assert_eq!(late.status, SlotStatus::NotTradable);
    assert_eq!(late.status.name(), "not_tradable_after_halt_start");
}

#[test]
fn under_the_resume_reading_a_gapped_stop_fills_at_the_first_print_after_the_halt() {
    let day = halt_day(bar(at(12, 0, 0), 98_500, 98_600, 98_400, 98_500, 1_000));
    let resume = WalkProfile {
        halt_rule: HaltRule::ResumeAtFirstPrint,
        ..second_profile()
    };
    let out = walk_one(
        &day,
        &resume,
        long_plan(at(10, 0, 0)),
        STOP_ONLY,
        FillVariant::STAGE_A,
        TradeSize::Small,
    );
    assert_eq!(out.exit_reason, ExitReason::StopLoss);
    assert_eq!(out.exit_second, at(12, 0, 0));
    assert_eq!(out.exit_value_paisa, i128::from(100 * 98_400));
    assert!(has(&out, TradeFlag::GapThroughStop));
    assert!(has(&out, TradeFlag::CircuitHalt));
    // No fill inside the halted range: an entry window over it fills after.
    let mut plan = long_plan(at(11, 39, 59));
    plan.entry_window_end = at(12, 30, 0);
    let late = walk_one(
        &day,
        &resume,
        plan,
        STOP_ONLY,
        FillVariant::STAGE_A,
        TradeSize::Small,
    );
    assert_eq!(late.first_entry_second, at(11, 39, 59));
    let mut plan = long_plan(at(11, 40, 30));
    plan.entry_window_end = at(12, 30, 0);
    let after = walk_one(
        &day,
        &resume,
        plan,
        STOP_ONLY,
        FillVariant::STAGE_A,
        TradeSize::Small,
    );
    assert_eq!(after.first_entry_second, at(12, 0, 0), "11:50 is halted");
}

// ------------------------------------------------------------ feed data --

/// A long entered at 10:09:58 with GDFL silent over 10:10 to 10:19 and a
/// resume at 10:20:00 beyond the stop; `traded` says whether Zerodha traded
/// those minutes. The committed past's longest silence is 30 seconds.
fn hole_day(traded: bool) -> Day {
    let mut day = Day::seconds(vec![
        bar(at(10, 9, 58), P, P + 100, P - 50, P + 50, 10_000),
        quiet(at(10, 9, 59), P, 1_000),
        bar(at(10, 20, 0), 98_500, 98_600, 98_400, 98_500, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    day.max_gap = 30;
    if traded {
        for minute in 10..20 {
            day.zerodha.push(quiet(at(10, minute, 0), P, 500));
        }
    }
    day
}

#[test]
fn a_silence_over_a_traded_zerodha_minute_is_an_execution_feed_hole() {
    let out = second_slot(&hole_day(true), long_plan(at(10, 9, 58)), STOP_ONLY);
    assert_eq!(
        out.status,
        SlotStatus::Unpriceable(UnpricedReason::ExecutionFeedHole)
    );
    assert_eq!(out.r_multiple(Search::Long), None, "no R, never the resume");
    let mut plan = long_plan(at(10, 10, 0));
    plan.entry_window_end = at(10, 11, 0);
    let unfilled = second_slot(&hole_day(true), plan, STOP_ONLY);
    assert_eq!(
        unfilled.status,
        SlotStatus::EntryUnfilled(UnfilledReason::FeedHole)
    );
    assert_eq!(unfilled.status.name(), "entry_unfilled_feed_hole");
}

#[test]
fn a_silence_zerodha_shares_is_flagged_and_the_walk_waits_for_the_next_second() {
    let out = second_slot(&hole_day(false), long_plan(at(10, 9, 58)), STOP_ONLY);
    assert_eq!(out.status, SlotStatus::Priced);
    assert!(has(&out, TradeFlag::SilenceInTrade));
    assert_eq!(out.exit_value_paisa, i128::from(100 * 98_400));
    let mut plan = long_plan(at(10, 10, 0));
    plan.entry_window_end = at(10, 11, 0);
    let unfilled = second_slot(&hole_day(false), plan, STOP_ONLY);
    assert_eq!(
        unfilled.status,
        SlotStatus::EntryUnfilled(UnfilledReason::WindowExpired)
    );
}

#[test]
fn a_zero_low_second_inside_a_trade_is_corrupt_and_never_an_exit_at_zero() {
    let day = second_day(&[
        corrupt(at(10, 0, 5), 100, 100, 0, 100, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    let exits = set_of(Search::Long, vec![STOP_ONLY]);
    assert_eq!(
        day.index(&second_profile(), &exits)
            .expect("indexes")
            .corrupt_bars(),
        1
    );
    let out = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(
        out.status,
        SlotStatus::Unpriceable(UnpricedReason::CorruptBarInTrade)
    );
    assert_eq!(out.exit_value_paisa, 0);
}

#[test]
fn a_flagged_print_fill_is_kept_and_an_outlier_second_is_flagged() {
    let both = BookMark {
        ask: true,
        bid: true,
        flagged_print: false,
    };
    let mut day = second_day(&[quiet(at(15, 9, 0), P, 1_000)]);
    day.marks = Some(vec![
        BookMark {
            flagged_print: true,
            ..both
        },
        both,
    ]);
    let out = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(out.status, SlotStatus::Priced);
    assert!(has(&out, TradeFlag::FlaggedPrintFill));
    assert!(!has(&out, TradeFlag::CrossFeedOutlier));
    // The 10:00 Zerodha minute's high moved below the fill: an outlier.
    day.zerodha[0].high = P + 90;
    let out = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert!(has(&out, TradeFlag::CrossFeedOutlier));
}

#[test]
fn a_touch_by_a_second_beyond_the_converted_minute_is_unwitnessed() {
    let mut day = second_day(&[
        bar(at(10, 1, 0), P, P, 98_990, 98_990, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    let out = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert!(
        !has(&out, TradeFlag::TouchUnwitnessed),
        "the minute saw 98,990"
    );
    day.zerodha[1].low = 99_000;
    let out = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert!(has(&out, TradeFlag::TouchUnwitnessed));
}

#[test]
fn worse_of_both_is_refused_until_prec_15_and_seconds_only_ignores_the_minute() {
    let mut day = second_day(&[
        bar(at(10, 1, 0), P, P, 99_000, 99_000, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    day.zerodha[1].low = 98_995;
    let exits = set_of(Search::Long, vec![STOP_ONLY]);
    let worse = WalkProfile {
        cross_feed: CrossFeed::WorseOfBoth,
        ..second_profile()
    };
    let refused = day.index(&worse, &exits).expect_err("not built");
    assert_eq!(refused, WalkRefusal::CrossFeedNotBuilt);
    assert_eq!(refused.name(), "cross_feed_not_built");
    let out = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(
        out.exit_reason,
        ExitReason::Forced,
        "no second touched the stop"
    );
}

// ---------------------------------------------------------------- clock --

#[test]
fn the_snapshot_edge_and_the_clock_lag_decide_the_first_admitted_second() {
    let day = Day::seconds(vec![
        quiet(at(10, 0, 0), P, 1_000),
        quiet(at(10, 0, 1), P + 100, 1_000),
        quiet(at(10, 0, 2), P + 200, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    let opens = second_slot(&day, long_plan(at(10, 0, 0)), STOP_ONLY);
    assert_eq!(opens.first_entry_second, at(10, 0, 0));
    let closes = WalkProfile {
        snapshot_edge: SnapshotEdge::StampClosesInterval,
        ..second_profile()
    };
    let out = walk_one(
        &day,
        &closes,
        long_plan(at(10, 0, 0)),
        STOP_ONLY,
        FillVariant::STAGE_A,
        TradeSize::Small,
    );
    assert_eq!(
        out.first_entry_second,
        at(10, 0, 1),
        "the decision's stamp never fills"
    );
    let exits = set_of(Search::Long, vec![STOP_ONLY]);
    let mut shifted = day.clone();
    shifted.lag = ClockLag::Measured(120);
    let refused = shifted
        .index(&second_profile(), &exits)
        .expect_err("shifted");
    assert_eq!(refused, WalkRefusal::ClockShifted { lag: 120 });
    assert_eq!(refused.name(), "clock_shifted");
    shifted.lag = ClockLag::Measured(2);
    let restamp = WalkProfile {
        clock_rule: ClockRule::Restamp,
        ..second_profile()
    };
    let out = walk_one(
        &shifted,
        &restamp,
        long_plan(at(10, 0, 0)),
        STOP_ONLY,
        FillVariant::STAGE_A,
        TradeSize::Small,
    );
    assert_eq!(out.first_entry_second, at(10, 0, 2));
    shifted.lag = ClockLag::Measured(-2);
    let out = walk_one(
        &shifted,
        &restamp,
        long_plan(at(10, 0, 0)),
        STOP_ONLY,
        FillVariant::STAGE_A,
        TradeSize::Small,
    );
    assert_eq!(
        out.first_entry_second,
        at(10, 0, 0),
        "a negative lag starts at c"
    );
    shifted.lag = ClockLag::Unmeasured;
    assert_eq!(
        shifted
            .index(&second_profile(), &exits)
            .expect_err("unmeasured"),
        WalkRefusal::ClockLagUnmeasured
    );
    shifted.lag = ClockLag::TrustUnchecked;
    let index = shifted.index(&second_profile(), &exits).expect("labelled");
    assert!(index.seconds_trust_unchecked());
    assert!(!index.book_observed());
}

// ------------------------------------------------------- exact factors --

#[test]
fn a_converted_level_that_would_round_either_way_is_compared_exactly() {
    // A level of 1,000 decision paisa is 2,333.33 raw under the pair (3, 7):
    // a second's high of 2,333 does not reach it; 2,334 does.
    let mut plan = long_plan(at(10, 0, 0));
    plan.entry_ref = 900;
    plan.base_edge = 800;
    plan.trigger_edge = 900;
    plan.fib_zero = 700;
    plan.fib_one = 1_000;
    let mut day = Day::seconds(vec![
        bar(at(10, 0, 0), 2_100, 2_110, 2_090, 2_100, 10_000),
        bar(at(10, 0, 5), 2_300, 2_333, 2_300, 2_300, 10_000),
        bar(at(10, 0, 6), 2_330, 2_334, 2_320, 2_330, 10_000),
        quiet(at(15, 9, 0), 2_100, 10_000),
    ]);
    day.raw_tick = 1;
    let factor = WalkProfile {
        price_map: PriceMap::Factor {
            zerodha_close: 3,
            gdfl_close: 7,
        },
        ..second_profile()
    };
    let variant = ExitVariant {
        take_profit: TakeProfit::FibRung {
            rung_permille: 1_000,
        },
        ..ExitVariant::REFERENCE
    };
    let out = walk_one(
        &day,
        &factor,
        plan,
        variant,
        FillVariant::STAGE_A,
        TradeSize::Small,
    );
    assert_eq!(out.planned_qty, 4_761);
    assert_eq!(out.decision_qty, 11_111);
    assert_eq!(out.exit_reason, ExitReason::TakeProfit);
    assert_eq!(out.exit_second, at(10, 0, 6));
    assert_eq!((out.risk_num, out.risk_den), (105 * 7, 3));
    // The stop: 800 x 7 / 3 = 1,866.67, less one raw tick = 1,865.67.
    let stop_day = |low: i64| {
        let mut d = Day::seconds(vec![
            bar(at(10, 0, 0), 2_100, 2_110, 2_090, 2_100, 10_000),
            bar(at(10, 0, 5), 2_000, 2_000, low, 2_000, 10_000),
            quiet(at(15, 9, 0), 2_100, 10_000),
        ]);
        d.raw_tick = 1;
        d
    };
    let walk = |low| {
        walk_one(
            &stop_day(low),
            &factor,
            plan,
            STOP_ONLY,
            FillVariant::STAGE_A,
            TradeSize::Small,
        )
        .exit_reason
    };
    assert_eq!(walk(1_866), ExitReason::Forced);
    assert_eq!(walk(1_865), ExitReason::StopLoss);
}

#[test]
fn under_the_pair_one_four_the_deep_risk_is_four_decision_risks_and_the_stop_one_raw_tick_out() {
    let factor = WalkProfile {
        price_map: PriceMap::Factor {
            zerodha_close: 1,
            gdfl_close: 4,
        },
        ..second_profile()
    };
    let day = |low: i64| {
        Day::seconds(vec![
            bar(at(10, 0, 0), 400_000, 400_100, 399_900, 400_000, 1_000),
            bar(at(10, 0, 5), 396_000, 396_000, low, 396_000, 1_000),
            quiet(at(15, 9, 0), 400_000, 1_000),
        ])
    };
    let walk = |low| {
        walk_one(
            &day(low),
            &factor,
            long_plan(at(10, 0, 0)),
            STOP_ONLY,
            FillVariant::STAGE_A,
            TradeSize::Small,
        )
    };
    let out = walk(395_995);
    assert_eq!((out.risk_num, out.risk_den), (4 * 1_005, 1));
    assert_eq!(out.risk_per_share_paisa, 1_005);
    assert_eq!(out.planned_qty, 25);
    assert_eq!(out.exit_reason, ExitReason::StopLoss);
    assert_eq!(walk(395_996).exit_reason, ExitReason::Forced);
}

#[test]
fn the_deep_quantity_is_spent_in_raw_shares_and_a_price_above_the_size_is_refused() {
    let factor = WalkProfile {
        price_map: PriceMap::Factor {
            zerodha_close: 60_680,
            gdfl_close: 242_700,
        },
        ..second_profile()
    };
    let mut plan = long_plan(at(10, 0, 0));
    plan.entry_ref = 60_680;
    plan.base_edge = 60_000;
    plan.trigger_edge = 60_680;
    let day = Day::seconds(vec![
        quiet(at(10, 0, 0), 242_700, 10_000),
        quiet(at(15, 9, 0), 242_700, 10_000),
    ]);
    let out = walk_one(
        &day,
        &factor,
        plan,
        STOP_ONLY,
        FillVariant::STAGE_A,
        TradeSize::Large,
    );
    assert_eq!(out.planned_qty, 206, "not 824");
    assert_eq!(out.decision_qty, 823, "floor(5,00,00,000 / 60,680)");
    assert_eq!(out.filled_qty, 206);
    let mut dear = long_plan(at(10, 0, 0));
    dear.entry_ref = 20_000_000;
    dear.base_edge = 19_000_000;
    let day = minute_day(&[quiet(at(10, 0, 0), 20_000_000, 1_000)]);
    let out = minute_slot(&day, dear, STOP_ONLY);
    assert_eq!(
        out.status,
        SlotStatus::Refused(SlotRefusal::SizeBelowOneShare)
    );
    assert_eq!(out.filled_qty, 0);
    let mut inverted = long_plan(at(10, 0, 0));
    inverted.base_edge = P + TICK;
    let out = minute_slot(&day, inverted, STOP_ONLY);
    assert_eq!(
        out.status,
        SlotStatus::Refused(SlotRefusal::NonpositivePlannedRisk)
    );
}

// ------------------------------------------- the reference and its seeds --

#[test]
fn the_reference_walk_closes_only_by_the_forced_exit_and_measures_its_peaks() {
    let day = second_day(&[
        quiet(at(10, 30, 0), P + 400, 1_000),
        bar(at(11, 0, 0), P + 800, P + 900, P + 700, P + 800, 1_000),
        bar(at(12, 0, 0), P + 800, P + 900, P + 600, P + 800, 1_000),
        quiet(at(15, 9, 0), P, 1_000),
    ]);
    let out = walk_one(
        &day,
        &second_profile(),
        long_plan(at(10, 0, 0)),
        ExitVariant::REFERENCE,
        FillVariant::REFERENCE,
        TradeSize::Small,
    );
    assert_eq!(out.exit_reason, ExitReason::Forced);
    assert_eq!(out.mfe_second, at(11, 0, 0));
    assert_eq!(out.last_peak_second, at(12, 0, 0));
    assert_eq!(out.mfe_to_exit_paisa, 800);
    assert_eq!(out.mfe_to_horizon_paisa, 800);
}

fn samples(values: &[(u32, u32)]) -> Vec<Sample> {
    values
        .iter()
        .map(|&(month, value)| Sample { month, value })
        .collect()
}

#[test]
fn seed_rule_is_the_lower_nearest_rank_median() {
    for values in [
        vec![3_u32],
        vec![1, 9],
        vec![1, 4, 9],
        vec![2, 3, 5, 8, 13, 21],
    ] {
        let s: Vec<Sample> = values
            .iter()
            .map(|&value| Sample { month: 0, value })
            .collect();
        let none: [Sample; 0] = [];
        let seeds = SeedSamples {
            far_edge_excursion_ticks: &none,
            run_up_milli_r: &s,
            giveback_permille: &none,
            pullback_candles: &none,
            poke_through_paisa: &none,
            swing_size_milli_r: &none,
            last_peak_clock_seconds: &none,
            time_to_best_seconds: &none,
        };
        let set = seed_exit_set(Search::Long, 0, 1, &seeds).expect("seeds");
        let median = vocab::fib_cube::lower_nearest_rank(&values, 1, 2).expect("a median");
        assert!(
            set.variants()
                .iter()
                .any(|v| v.take_profit == TakeProfit::RMultiple { milli_r: median }),
            "{values:?}"
        );
    }
}

/// Every distribution with one learning value and, when `garbage`, wild
/// values at months at and after W0 = 6.
fn seed_samples(garbage: bool) -> [Vec<Sample>; 8] {
    let late = |v| {
        if garbage {
            vec![(6, v), (9, v)]
        } else {
            Vec::new()
        }
    };
    let one = |early: &[(u32, u32)], late_value| {
        let mut all = early.to_vec();
        all.extend(late(late_value));
        all.sort_by_key(|&(_, v)| v);
        samples(&all)
    };
    [
        one(&[(0, 2), (1, 4), (2, 6)], 90),
        one(&[(0, 900), (5, 1_500), (3, 2_400)], 99_000),
        one(&[(0, 300), (1, 400)], 999),
        one(&[(0, 1), (1, 2), (2, 3)], 50),
        one(&[(0, 15), (1, 25)], 9_000),
        one(&[(0, 600)], 77_000),
        one(&[(0, at(14, 0, 0)), (1, at(14, 30, 0))], at(15, 0, 0)),
        one(&[(0, 600), (4, 1_200)], 20_000),
    ]
}

fn seeded(garbage: bool) -> ExitSet {
    let s = seed_samples(garbage);
    let seeds = SeedSamples {
        far_edge_excursion_ticks: &s[0],
        run_up_milli_r: &s[1],
        giveback_permille: &s[2],
        pullback_candles: &s[3],
        poke_through_paisa: &s[4],
        swing_size_milli_r: &s[5],
        last_peak_clock_seconds: &s[6],
        time_to_best_seconds: &s[7],
    };
    seed_exit_set(Search::Long, 6, 6, &seeds).expect("seeds")
}

#[test]
fn the_seed_reads_only_the_learning_months_and_garbage_after_w0_changes_no_byte() {
    let clean = seeded(false);
    let dirty = seeded(true);
    assert_eq!(clean.bytes(), dirty.bytes());
    assert_eq!(clean.digest(), dirty.digest());
    // 3 stops x 2 targets x 2 give-backs x 5 trails x 3 time exits.
    assert_eq!(clean.count(), 180);
    assert_eq!(clean.widths(), &[1, 2]);
    let first = clean.variants()[0];
    assert_eq!(first, ExitVariant::REFERENCE);
    assert!(
        clean
            .variants()
            .iter()
            .any(|v| v.stop == StopLoss::BeyondBase { extra_ticks: 4 })
    );
    assert!(
        clean
            .variants()
            .iter()
            .any(|v| v.time_exit == TimeExit::Hold { seconds: 600 })
    );
    let fills =
        seed_fill_set(Search::Long, 6, &samples(&[(0, 200), (1, 400), (7, 999)])).expect("fills");
    assert_eq!(fills.variants().len(), 4);
    assert!(
        fills
            .variants()
            .iter()
            .any(|f| f.participation == Participation::Cap { permille: 200 })
    );
}

// ------------------------------------------- variants, passes, increments --

/// Five variants of every component kind.
fn five() -> ExitSet {
    set_of(
        Search::Long,
        vec![
            STOP_ONLY,
            ExitVariant {
                take_profit: ONE_R,
                ..STOP_ONLY
            },
            ExitVariant {
                trailing_take_profit: TrailingTakeProfit::GiveBack { permille: 400 },
                ..ExitVariant::REFERENCE
            },
            ExitVariant {
                trailing_stop: TrailingStop::Swing {
                    width: 1,
                    buffer_paisa: 0,
                },
                ..STOP_ONLY
            },
            ExitVariant {
                time_exit: TimeExit::Hold { seconds: 600 },
                ..ExitVariant::REFERENCE
            },
        ],
    )
}

/// A busy minute day with three plans.
fn busy() -> (Day, Vec<Plan>) {
    let mut bars = vec![quiet(at(9, 59, 0), P - 50, 1_000)];
    let mut price = P;
    for minute in 0..60 {
        let swing = [40, -30, 70, -90, 120, -60][minute as usize % 6];
        price += swing;
        bars.push(bar(
            at(10, minute, 0),
            price,
            price + 80,
            price - 70,
            price,
            1_000,
        ));
    }
    bars.push(quiet(at(15, 9, 0), P, 1_000));
    let plans = [at(10, 0, 0), at(10, 7, 0), at(10, 31, 0)]
        .into_iter()
        .map(long_plan)
        .collect();
    (Day::minutes(bars), plans)
}

#[test]
fn output_bytes_independent_of_budget() {
    let (day, plans) = busy();
    let exits = five();
    let index = day.index(&minute_profile(), &exits).expect("indexes");
    let sizes = [TradeSize::Small, TradeSize::Large];
    let mut runs = Vec::new();
    for budget in [1, 2, 7] {
        runs.push(
            walk_all(
                &index,
                &plans,
                &exits,
                FillVariant::STAGE_A,
                &sizes,
                PassBudget::new(budget).expect("a budget"),
            )
            .expect("walks"),
        );
    }
    assert_eq!(runs[0], runs[1]);
    assert_eq!(runs[0], runs[2]);
    assert_eq!(runs[0].len(), 15);
    assert_eq!(exits.passes(PassBudget::new(2).expect("two")).len(), 3);
    // Each pair equals its single-variant walk byte for byte.
    for walked in &runs[0] {
        let single = walk_pairs(
            &index,
            &plans,
            &exits,
            FillVariant::STAGE_A,
            &sizes,
            &[(walked.plan, walked.variant)],
        )
        .expect("walks");
        assert_eq!(&single[0], walked);
    }
    assert!(PassBudget::new(0).is_none());
}

#[test]
fn an_added_component_needs_no_code_and_an_incremental_rerun_walks_only_the_missing_pairs() {
    let (day, plans) = busy();
    let base = set_of(Search::Long, vec![STOP_ONLY]);
    let grown = base
        .extended(
            7,
            &[ExitVariant {
                trailing_take_profit: TrailingTakeProfit::GiveBack { permille: 250 },
                ..STOP_ONLY
            }],
        )
        .expect("an appended version");
    assert_eq!(grown.version(), 2);
    assert_eq!(grown.get(0), base.get(0), "an id keeps its variant");
    let index = day.index(&minute_profile(), &grown).expect("indexes");
    let sizes = [TradeSize::Small];
    let wanted: Vec<(u32, u16)> = (0..3).flat_map(|p| [(p, 0), (p, 1)]).collect();
    let have: Vec<(u32, u16)> = (0..3).map(|p| (p, 0)).collect();
    let missing = missing_pairs(&wanted, &have);
    assert_eq!(missing, vec![(0, 1), (1, 1), (2, 1)]);
    let fresh = walk_pairs(
        &index,
        &plans,
        &grown,
        FillVariant::STAGE_A,
        &sizes,
        &missing,
    )
    .expect("walks");
    let full = walk_pairs(
        &index,
        &plans,
        &grown,
        FillVariant::STAGE_A,
        &sizes,
        &wanted,
    )
    .expect("walks");
    let full_new: Vec<&Walked> = full.iter().filter(|w| w.variant == 1).collect();
    assert_eq!(fresh.iter().collect::<Vec<_>>(), full_new);
    let old = walk_pairs(&index, &plans, &base, FillVariant::STAGE_A, &sizes, &have);
    assert!(old.is_ok());
}

#[test]
fn the_walk_refuses_a_deep_fill_at_stage_a_an_unknown_id_and_the_other_direction() {
    let (day, plans) = busy();
    let exits = five();
    let index = day.index(&minute_profile(), &exits).expect("indexes");
    let sizes = [TradeSize::Small];
    assert_eq!(
        walk_pairs(
            &index,
            &plans,
            &exits,
            FillVariant::REFERENCE,
            &sizes,
            &[(0, 0)]
        ),
        Err(WalkRefusal::FillVariantDeepOnly)
    );
    assert_eq!(
        walk_pairs(
            &index,
            &plans,
            &exits,
            FillVariant::STAGE_A,
            &sizes,
            &[(0, 9)]
        ),
        Err(WalkRefusal::VariantUnknown { id: 9 })
    );
    assert!(matches!(
        walk_pairs(
            &index,
            &plans,
            &exits,
            FillVariant::STAGE_A,
            &sizes,
            &[(7, 0)]
        ),
        Err(WalkRefusal::PlanInvalid { plan: 7, .. })
    ));
    let mut short = plans[0];
    short.direction = Search::Short;
    assert_eq!(
        walk_pairs(
            &index,
            &[short],
            &exits,
            FillVariant::STAGE_A,
            &sizes,
            &[(0, 0)]
        ),
        Err(WalkRefusal::DirectionMismatch { plan: 0 })
    );
}

#[test]
fn a_walk_is_deterministic() {
    let (day, plans) = busy();
    let exits = five();
    let index = day.index(&minute_profile(), &exits).expect("indexes");
    let run = || {
        walk_all(
            &index,
            &plans,
            &exits,
            FillVariant::STAGE_A,
            &[TradeSize::Small],
            PassBudget::new(3).expect("three"),
        )
        .expect("walks")
    };
    assert_eq!(run(), run());
}

// --------------------------------------------------------------- mirror --

/// A deterministic generator for the seeded property tests (splitmix64).
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + i64::try_from(self.below(u64::try_from(hi - lo + 1).unwrap())).unwrap()
    }

    fn coin(&mut self) -> bool {
        self.below(2) == 0
    }
}

/// The mirror of a price about `2P`: a short on mirrored bars is the long.
const K: i64 = 2 * P;

fn mirror_bar(c: &Candle) -> Candle {
    Candle::new(
        c.ts_micros,
        K - c.open,
        K - c.low,
        K - c.high,
        K - c.close,
        c.volume,
        c.open_interest,
    )
}

/// A random day at `resolution` from 10:00 with gaps, and a forced window.
fn random_day(rng: &mut Seeded, resolution: Resolution) -> Day {
    let mut bars = vec![quiet(at(9, 59, 0), P - rng.range(0, 300), 1_000)];
    let mut price = P;
    let step = if resolution == Resolution::Minute {
        60
    } else {
        1
    };
    let mut second = at(10, 0, 0);
    for _ in 0..rng.range(20, 90) {
        let open = price;
        price += rng.range(-250, 250);
        let high = open.max(price) + rng.range(0, 150);
        let low = open.min(price) - rng.range(0, 150);
        let volume = rng.range(1, 160);
        bars.push(bar(second, open, high, low, price, volume));
        second += step * u32::try_from(rng.range(1, 3)).unwrap();
    }
    for window in [0, 17, 40, 59] {
        if rng.coin() {
            let w = if resolution == Resolution::Minute {
                at(15, 9, 0)
            } else {
                at(15, 9, window)
            };
            if bars.last().is_some_and(|b| b.ts_micros >= ts(w)) {
                continue;
            }
            let open = price;
            bars.push(bar(
                w,
                open,
                open + rng.range(0, 90),
                open - rng.range(0, 90),
                open,
                rng.range(1, 400),
            ));
        }
    }
    let mut day = match resolution {
        Resolution::Minute => Day::minutes(bars),
        Resolution::Second => Day::seconds(bars),
    };
    if resolution == Resolution::Second && rng.coin() {
        let marks = (0..day.bars.len())
            .map(|_| BookMark {
                ask: rng.below(8) != 0,
                bid: rng.below(8) != 0,
                flagged_print: rng.below(10) == 0,
            })
            .collect();
        day.marks = Some(marks);
    }
    if rng.below(4) == 0 {
        let start = at(10, 0, 0) + u32::try_from(rng.range(30, 900)).unwrap();
        day.halts = vec![HaltRange {
            start_second: start,
            end_second: start + u32::try_from(rng.range(1, 200)).unwrap(),
        }];
    }
    if rng.coin() {
        day.max_gap = u32::try_from(rng.range(0, 4)).unwrap();
    }
    day
}

/// The mirrored day: every price about `2P`, the book's sides swapped.
fn mirror_day(day: &Day) -> Day {
    let mut out = day.clone();
    out.bars = day.bars.iter().map(mirror_bar).collect();
    out.zerodha = day.zerodha.iter().map(mirror_bar).collect();
    out.trail = day
        .trail
        .iter()
        .map(|c| FibCandle {
            candle: mirror_bar(&c.candle),
            ..*c
        })
        .collect();
    out.marks = day.marks.as_ref().map(|marks| {
        marks
            .iter()
            .map(|m| BookMark {
                ask: m.bid,
                bid: m.ask,
                flagged_print: m.flagged_print,
            })
            .collect()
    });
    out
}

fn random_variant(rng: &mut Seeded) -> ExitVariant {
    let rungs = indicators::fib::RUNGS;
    ExitVariant {
        stop: match rng.below(3) {
            0 => StopLoss::Absent,
            _ => StopLoss::BeyondBase {
                extra_ticks: u32::try_from(rng.range(0, 6)).unwrap(),
            },
        },
        take_profit: match rng.below(4) {
            0 => TakeProfit::Absent,
            1 => TakeProfit::RMultiple {
                milli_r: u32::try_from(rng.range(50, 3_000)).unwrap(),
            },
            2 => TakeProfit::FibRung {
                rung_permille: u16::try_from(rungs[rng.below(11) as usize]).unwrap(),
            },
            _ => TakeProfit::NextRung,
        },
        trailing_take_profit: match rng.below(2) {
            0 => TrailingTakeProfit::Absent,
            _ => TrailingTakeProfit::GiveBack {
                permille: u16::try_from(rng.range(1, 1_000)).unwrap(),
            },
        },
        trailing_stop: match rng.below(3) {
            0 => TrailingStop::Absent,
            1 => TrailingStop::Swing {
                width: u16::try_from(rng.range(1, 2)).unwrap(),
                buffer_paisa: u32::try_from(rng.range(0, 20)).unwrap(),
            },
            _ => TrailingStop::RStep {
                milli_r: u32::try_from(rng.range(100, 2_000)).unwrap(),
            },
        },
        time_exit: match rng.below(3) {
            0 => TimeExit::Absent,
            1 => TimeExit::Clock {
                second: at(10, 0, 0) + u32::try_from(rng.range(0, 3_600)).unwrap(),
            },
            _ => TimeExit::Hold {
                seconds: u32::try_from(rng.range(1, 900)).unwrap(),
            },
        },
    }
}

fn random_plan(rng: &mut Seeded) -> Plan {
    let mut plan = long_plan(at(10, 0, 0) + 60 * u32::try_from(rng.range(0, 30)).unwrap());
    plan.style = [EntryStyle::Close, EntryStyle::Break, EntryStyle::Retest][rng.below(3) as usize];
    plan.entry_window_end = plan.decision_second + 60 * u32::try_from(rng.range(1, 20)).unwrap();
    plan.base_edge = P - rng.range(50, 1_500);
    plan.trigger_edge = P + rng.range(-200, 200);
    plan.fib_zero = P - rng.range(0, 2_000);
    plan.fib_one = P + rng.range(1, 2_000);
    plan
}

fn mirror_plan(plan: &Plan) -> Plan {
    Plan {
        direction: Search::Short,
        entry_ref: K - plan.entry_ref,
        base_edge: K - plan.base_edge,
        trigger_edge: K - plan.trigger_edge,
        fib_zero: K - plan.fib_zero,
        fib_one: K - plan.fib_one,
        ..*plan
    }
}

fn random_profile(rng: &mut Seeded, resolution: Resolution) -> WalkProfile {
    WalkProfile {
        resolution,
        exit_fill: if rng.coin() {
            ExitFill::SpillByVolume
        } else {
            ExitFill::FirstTouchWhole
        },
        snapshot_edge: if rng.coin() {
            SnapshotEdge::StampOpensInterval
        } else {
            SnapshotEdge::StampClosesInterval
        },
        halt_rule: if rng.coin() {
            HaltRule::WorstSeenThroughHalt
        } else {
            HaltRule::ResumeAtFirstPrint
        },
        beyond_target: if rng.coin() {
            BeyondTarget::FiresOnFirstBar
        } else {
            BeyondTarget::TargetVoid
        },
        ..minute_profile()
    }
}

fn random_fill(rng: &mut Seeded, resolution: Resolution) -> FillVariant {
    if resolution == Resolution::Minute {
        return FillVariant::STAGE_A;
    }
    FillVariant {
        window: match rng.below(3) {
            0 => EntryWindow::NextClose,
            1 => EntryWindow::UntilForced,
            _ => EntryWindow::Deadline {
                seconds: u32::try_from(rng.range(1, 400)).unwrap(),
            },
        },
        participation: if rng.coin() {
            Participation::NoCap
        } else {
            Participation::Cap {
                permille: u16::try_from(rng.range(1, 1_000)).unwrap(),
            }
        },
    }
}

/// Exited shares of a slot: its own order's and its forced order's.
fn exited(outcome: &SlotOutcome) -> i128 {
    let qty = |o: Option<runner::fib::types::ExitOrder>| o.map_or(0, |o| o.filled_qty);
    i128::from(qty(outcome.own_exit) + qty(outcome.forced_exit))
}

#[test]
fn a_short_on_mirrored_bars_is_the_long_mirrored_exactly() {
    let mut rng = Seeded(0x00F1_B5ED);
    let mut statuses = std::collections::HashSet::with_capacity(32);
    let mut reasons = std::collections::HashSet::with_capacity(8);
    for trial in 0..600 {
        let resolution = if trial % 2 == 0 {
            Resolution::Minute
        } else {
            Resolution::Second
        };
        let day = random_day(&mut rng, resolution);
        let plan = random_plan(&mut rng);
        let variant = random_variant(&mut rng);
        let profile = random_profile(&mut rng, resolution);
        let fill = random_fill(&mut rng, resolution);
        let size = if rng.coin() {
            TradeSize::Small
        } else {
            TradeSize::Large
        };
        let long = walk_one(&day, &profile, plan, variant, fill, size);
        let short = walk_one(
            &mirror_day(&day),
            &profile,
            mirror_plan(&plan),
            variant,
            fill,
            size,
        );
        let context = format!("trial {trial}: {plan:?} {variant:?} {profile:?} {fill:?}");
        assert_eq!(long.status, short.status, "{context}");
        assert_eq!(long.exit_reason, short.exit_reason, "{context}");
        assert_eq!(long.flags, short.flags, "{context}");
        assert_eq!(
            (long.filled_qty, long.decision_qty, long.planned_qty),
            (short.filled_qty, short.decision_qty, short.planned_qty),
            "{context}"
        );
        assert_eq!(
            (long.risk_per_share_paisa, long.risk_num, long.risk_den),
            (short.risk_per_share_paisa, short.risk_num, short.risk_den),
            "{context}"
        );
        assert_eq!(
            (
                long.first_entry_second,
                long.last_entry_second,
                long.exit_second
            ),
            (
                short.first_entry_second,
                short.last_entry_second,
                short.exit_second
            ),
            "{context}"
        );
        assert_eq!(
            (
                long.mfe_to_exit_paisa,
                long.mae_to_exit_paisa,
                long.mfe_to_horizon_paisa,
                long.mae_to_horizon_paisa
            ),
            (
                short.mfe_to_exit_paisa,
                short.mae_to_exit_paisa,
                short.mfe_to_horizon_paisa,
                short.mae_to_horizon_paisa
            ),
            "{context}"
        );
        assert_eq!(
            (
                long.mfe_second,
                long.last_peak_second,
                long.entry_wait_seconds,
                long.exit_spill_seconds,
                long.max_participation_permille
            ),
            (
                short.mfe_second,
                short.last_peak_second,
                short.entry_wait_seconds,
                short.exit_spill_seconds,
                short.max_participation_permille
            ),
            "{context}"
        );
        let k = i128::from(K);
        assert_eq!(
            short.entry_value_paisa,
            k * i128::from(long.filled_qty) - long.entry_value_paisa,
            "{context}"
        );
        assert_eq!(
            short.exit_value_paisa,
            k * exited(&long) - long.exit_value_paisa,
            "{context}"
        );
        assert_eq!(
            long.r_multiple(Search::Long),
            short.r_multiple(Search::Short),
            "{context}"
        );
        statuses.insert(long.status.name());
        reasons.insert(long.exit_reason.name());
    }
    // The trials reach the walker's outcomes, not one corner of them.
    assert!(statuses.len() >= 6, "{statuses:?}");
    assert_eq!(reasons.len(), 6, "{reasons:?}");
}

// ---------------------------------------------------------------- codec --

use runner::fib::codec::{
    CodecRefusal, DECISION_RECORD_BYTES, DecisionRecord, HEADER_BYTES, Header, MEASURED_ROWS,
    Measurement, OUTCOME_FIXED_BYTES, OUTCOME_RECORD_V1, OUTCOME_RECORD_V2, OutcomeRecord,
    RecordKind, SLOT_V1_BYTES, SLOT_V2_BYTES, estimate_bytes, outcome_stride,
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn decision() -> DecisionRecord {
    let mut measurements = [Measurement {
        raw: 0,
        position_num: 0,
        position_den: 0,
    }; MEASURED_ROWS];
    for (k, m) in measurements.iter_mut().enumerate() {
        let k = k as u64;
        m.raw = -7 - i64::try_from(k).unwrap();
        m.position_num = 2 * k + 1;
        m.position_den = 2 * (k + 20);
    }
    let mut coords = [0_u8; vocab::fib_cube::ROW_COUNT];
    for (k, c) in coords.iter_mut().enumerate() {
        *c = u8::try_from(k % 3).unwrap();
    }
    DecisionRecord {
        ordinal: 41,
        stock_slot: 207,
        ist_day: DAY,
        plan: long_plan(at(10, 0, 0)),
        kind: indicators::fibflag::types::BaseKind::Micro3,
        coords,
        measurements,
    }
}

/// A priced long with both exit orders, and an unfilled slot.
fn outcome_record() -> OutcomeRecord {
    let day = second_day(&[
        quiet(at(15, 8, 0), P, 30),
        bar(at(15, 9, 0), P, P + 10, 99_700, P, 1_000),
    ]);
    let variant = ExitVariant {
        time_exit: TimeExit::Clock {
            second: at(15, 8, 0),
        },
        ..STOP_ONLY
    };
    let priced = second_slot(&day, long_plan(at(10, 0, 0)), variant);
    let unfilled = second_slot(&day, long_plan(at(12, 0, 0)), variant);
    OutcomeRecord {
        ordinal: 41,
        variant: 3,
        fill: 1,
        zerodha_close: 60_680,
        gdfl_close: 242_700,
        clock_lag: ClockLag::Measured(-2),
        slots: vec![priced, unfilled],
    }
}

#[test]
fn record_stride_equals_declared_layout() {
    assert_eq!(MEASURED_ROWS, 13);
    assert_eq!(HEADER_BYTES, 53);
    assert_eq!(DECISION_RECORD_BYTES, 405);
    assert_eq!(OUTCOME_FIXED_BYTES, 29);
    assert_eq!(SLOT_V1_BYTES, 167);
    assert_eq!(SLOT_V2_BYTES, 225);
    assert_eq!(decision().encode().len(), DECISION_RECORD_BYTES);
    let record = outcome_record();
    for version in [OUTCOME_RECORD_V1, OUTCOME_RECORD_V2] {
        let bytes = record.encode(version).expect("encodes");
        assert_eq!(Some(bytes.len()), outcome_stride(version, 2));
    }
    assert_eq!(outcome_stride(OUTCOME_RECORD_V2 + 1, 2), None);
    assert_eq!(
        record.encode(OUTCOME_RECORD_V2 + 1),
        Err(CodecRefusal::Version {
            version: OUTCOME_RECORD_V2 + 1
        })
    );
}

#[test]
fn the_records_bytes_are_pinned() {
    let header = Header::outcomes(
        OUTCOME_RECORD_V2,
        Resolution::Second,
        2,
        7,
        [0xAB; vocab::fib_cube::DIGEST_BYTES],
    )
    .expect("a header");
    // "BTXFIBO1", version 2, resolution 1, two slots, stride 479, version 7.
    assert_eq!(
        hex(&header.encode()),
        format!(
            "{}{}{}{}{}{}{}",
            "4254584649424f31",
            "0200",
            "01",
            "0200",
            "df010000",
            "07000000",
            "ab".repeat(32)
        )
    );
    let decision = hex(&brutex_core::blake3::hash(&decision().encode()));
    let outcome = hex(&brutex_core::blake3::hash(
        &outcome_record().encode(OUTCOME_RECORD_V2).expect("encodes"),
    ));
    let outcome_v1 = hex(&brutex_core::blake3::hash(
        &outcome_record().encode(OUTCOME_RECORD_V1).expect("encodes"),
    ));
    assert_eq!(
        decision,
        "03998d091aaeefe0e58384e050da5d7199b22fae0c7863f900d6aeb0cd2685d5"
    );
    assert_eq!(
        outcome,
        "8434ee0996726db30a9aa49bee7bb4b3ecf19ac3fc1511312d45fe48c5890522"
    );
    assert_eq!(
        outcome_v1,
        "bf35fcf707e6277f00fcb87d18cdc47d0b9806dac32837a517390fe95104e4fc"
    );
}

#[test]
fn every_record_round_trips_and_a_v1_record_reads_its_orders_merged() {
    let d = decision();
    assert_eq!(DecisionRecord::decode(&d.encode()), Ok(d));
    let record = outcome_record();
    let v2 = Header::outcomes(OUTCOME_RECORD_V2, Resolution::Second, 2, 7, [1; 32]).expect("v2");
    let decoded = OutcomeRecord::decode(&v2, &record.encode(OUTCOME_RECORD_V2).expect("v2"))
        .expect("decodes");
    assert_eq!(decoded.record, record);
    assert!(!decoded.orders_merged);
    assert!(record.slots[0].own_exit.is_some() && record.slots[0].forced_exit.is_some());
    let v1 = Header::outcomes(OUTCOME_RECORD_V1, Resolution::Second, 2, 7, [1; 32]).expect("v1");
    let old = OutcomeRecord::decode(&v1, &record.encode(OUTCOME_RECORD_V1).expect("v1"))
        .expect("decodes");
    assert!(old.orders_merged, "v1 stays decodable, its orders merged");
    let mut merged = record.clone();
    for slot in &mut merged.slots {
        slot.own_exit = None;
        slot.forced_exit = None;
    }
    assert_eq!(old.record, merged);
    for header in [v1, v2, Header::decisions(1, [9; 32])] {
        assert_eq!(Header::decode(&header.encode()), Ok(header));
    }
    let lags = [
        ClockLag::Unmeasured,
        ClockLag::TrustUnchecked,
        ClockLag::Measured(7),
    ];
    for lag in lags {
        let mut r = record.clone();
        r.clock_lag = lag;
        let back =
            OutcomeRecord::decode(&v2, &r.encode(OUTCOME_RECORD_V2).expect("v2")).expect("decodes");
        assert_eq!(back.record.clock_lag, lag);
    }
}

#[test]
fn every_status_and_reason_survives_the_codec() {
    let record = outcome_record();
    let v2 = Header::outcomes(OUTCOME_RECORD_V2, Resolution::Second, 2, 7, [1; 32]).expect("v2");
    let statuses = [
        SlotStatus::Priced,
        SlotStatus::NotTradable,
        SlotStatus::Refused(SlotRefusal::SizeBelowOneShare),
        SlotStatus::Refused(SlotRefusal::NonpositivePlannedRisk),
        SlotStatus::EntryUnfilled(UnfilledReason::WindowExpired),
        SlotStatus::EntryUnfilled(UnfilledReason::TargetBeforeFill),
        SlotStatus::EntryUnfilled(UnfilledReason::FeedHole),
        SlotStatus::EntryUnfilled(UnfilledReason::NeverTriggered),
        SlotStatus::Unpriceable(UnpricedReason::ForcedThin),
        SlotStatus::Unpriceable(UnpricedReason::ForcedLockedLong),
        SlotStatus::Unpriceable(UnpricedReason::ForcedLockedShort),
        SlotStatus::Unpriceable(UnpricedReason::ForcedCheap),
        SlotStatus::Unpriceable(UnpricedReason::ExecutionFeedHole),
        SlotStatus::Unpriceable(UnpricedReason::CorruptBarInTrade),
        SlotStatus::Unpriceable(UnpricedReason::Halt),
    ];
    let reasons = [
        ExitReason::StopLoss,
        ExitReason::TrailingStopLoss,
        ExitReason::TrailingTakeProfit,
        ExitReason::TakeProfit,
        ExitReason::Time,
        ExitReason::Forced,
    ];
    let mut names = std::collections::HashSet::with_capacity(statuses.len());
    for (k, status) in statuses.into_iter().enumerate() {
        let mut r = record.clone();
        r.slots[0].status = status;
        r.slots[1].exit_reason = reasons[k % reasons.len()];
        let back =
            OutcomeRecord::decode(&v2, &r.encode(OUTCOME_RECORD_V2).expect("v2")).expect("decodes");
        assert_eq!(back.record, r);
        assert!(names.insert(status.name()), "{} named twice", status.name());
    }
    assert_eq!(SlotStatus::Priced.entered(), true);
    assert!(SlotStatus::Unpriceable(UnpricedReason::Halt).entered());
    assert!(!SlotStatus::NotTradable.entered());
}

#[test]
fn a_damaged_record_is_refused_by_name() {
    let header =
        Header::outcomes(OUTCOME_RECORD_V2, Resolution::Second, 2, 7, [1; 32]).expect("v2");
    let mut bytes = header.encode();
    bytes[0] = b'X';
    assert_eq!(Header::decode(&bytes), Err(CodecRefusal::Magic));
    assert_eq!(CodecRefusal::Magic.name(), "record_magic");
    let short = &header.encode()[..HEADER_BYTES - 1];
    assert_eq!(
        Header::decode(short),
        Err(CodecRefusal::Length {
            expected: HEADER_BYTES,
            got: HEADER_BYTES - 1
        })
    );
    let mut bytes = header.encode();
    bytes[8] = 9;
    assert_eq!(
        Header::decode(&bytes),
        Err(CodecRefusal::Version { version: 9 })
    );
    let mut bytes = Header::decisions(1, [0; 32]).encode();
    bytes[8] = 2;
    assert_eq!(
        Header::decode(&bytes),
        Err(CodecRefusal::Version { version: 2 })
    );
    let mut bytes = header.encode();
    bytes[10] = 7;
    assert_eq!(
        Header::decode(&bytes),
        Err(CodecRefusal::Code {
            field: "resolution",
            code: 7
        })
    );
    assert_eq!(
        Header::outcomes(9, Resolution::Minute, 1, 1, [0; 32]),
        Err(CodecRefusal::Version { version: 9 })
    );
    let mut d = decision().encode();
    d[14] = 5;
    assert_eq!(
        DecisionRecord::decode(&d),
        Err(CodecRefusal::Code {
            field: "direction",
            code: 5
        })
    );
    let mut d = decision().encode();
    d[15] = 5;
    assert!(matches!(
        DecisionRecord::decode(&d),
        Err(CodecRefusal::Code {
            field: "entry_style",
            ..
        })
    ));
    let mut d = decision().encode();
    d[14 + 54] = 5;
    assert!(matches!(
        DecisionRecord::decode(&d),
        Err(CodecRefusal::Code {
            field: "base_kind",
            ..
        })
    ));
    let record = outcome_record().encode(OUTCOME_RECORD_V2).expect("v2");
    let damage = |at: usize, value: u8| {
        let mut bytes = record.clone();
        bytes[at] = value;
        OutcomeRecord::decode(&header, &bytes).map(|_| ())
    };
    let lag_at = 4 + 2 + 2 + 8 + 8;
    assert!(matches!(
        damage(lag_at, 9),
        Err(CodecRefusal::Code {
            field: "clock_lag",
            ..
        })
    ));
    let slot = OUTCOME_FIXED_BYTES;
    assert!(matches!(
        damage(slot, 9),
        Err(CodecRefusal::Code {
            field: "status",
            ..
        })
    ));
    for (kind, field) in [
        (1, "unfilled_reason"),
        (2, "unpriced_reason"),
        (3, "slot_refusal"),
    ] {
        let mut bytes = record.clone();
        bytes[slot] = kind;
        bytes[slot + 1] = 99;
        assert!(
            matches!(
                OutcomeRecord::decode(&header, &bytes),
                Err(CodecRefusal::Code { field: f, .. }) if f == field
            ),
            "{field}"
        );
    }
    assert!(matches!(
        damage(slot + 2, 99),
        Err(CodecRefusal::Code {
            field: "exit_reason",
            ..
        })
    ));
    assert!(matches!(
        OutcomeRecord::decode(&header, &record[1..]),
        Err(CodecRefusal::Length { .. })
    ));
    let mut wide = outcome_record();
    wide.slots = vec![wide.slots[0]; usize::from(u16::MAX) + 1];
    assert_eq!(wide.encode(OUTCOME_RECORD_V2), Err(CodecRefusal::SlotCount));
    for (refusal, name) in [
        (CodecRefusal::SlotCount, "record_slot_count"),
        (CodecRefusal::Version { version: 0 }, "record_version"),
        (
            CodecRefusal::Length {
                expected: 0,
                got: 1,
            },
            "record_length",
        ),
        (
            CodecRefusal::Code {
                field: "x",
                code: 0,
            },
            "record_code",
        ),
    ] {
        assert_eq!(refusal.name(), name);
    }
}

#[test]
fn estimate_uses_header_stride() {
    let header =
        Header::outcomes(OUTCOME_RECORD_V2, Resolution::Second, 2, 7, [1; 32]).expect("v2");
    let stride = u128::try_from(OUTCOME_FIXED_BYTES + 2 * SLOT_V2_BYTES).unwrap();
    assert_eq!(u128::from(header.stride), stride);
    assert_eq!(estimate_bytes(&header, 10), 53 + 10 * stride);
    let later = Header {
        stride: header.stride + 100,
        ..header
    };
    assert_eq!(estimate_bytes(&later, 10), 53 + 10 * (stride + 100));
    assert_eq!(Header::decisions(1, [0; 32]).kind, RecordKind::Decisions);
    assert_eq!(Header::decisions(1, [0; 32]).stride, 405);
}

#[test]
fn adding_a_variant_appends_outcomes_only() {
    let (day, plans) = busy();
    let base = set_of(Search::Long, vec![STOP_ONLY]);
    let grown = base
        .extended(
            7,
            &[ExitVariant {
                take_profit: ONE_R,
                ..STOP_ONLY
            }],
        )
        .expect("grown");
    let file = |exits: &ExitSet, id: u16| -> Vec<u8> {
        let index = day.index(&minute_profile(), exits).expect("indexes");
        let pairs: Vec<(u32, u16)> = (0..3).map(|p| (p, id)).collect();
        let walked = walk_pairs(
            &index,
            &plans,
            exits,
            FillVariant::STAGE_A,
            &[TradeSize::Small],
            &pairs,
        )
        .expect("walks");
        let mut bytes = Vec::new();
        for w in walked {
            let record = OutcomeRecord {
                ordinal: w.plan,
                variant: w.variant,
                fill: 0,
                zerodha_close: 1,
                gdfl_close: 1,
                clock_lag: ClockLag::Measured(0),
                slots: w.slots,
            };
            bytes.extend(record.encode(OUTCOME_RECORD_V2).expect("v2"));
        }
        bytes
    };
    let before = file(&base, 0);
    let after = file(&grown, 0);
    assert_eq!(
        before, after,
        "variant 0's file is untouched by the new variant"
    );
    let added = file(&grown, 1);
    assert_eq!(
        added.len(),
        3 * outcome_stride(OUTCOME_RECORD_V2, 1).unwrap()
    );
    assert_ne!(added, before);
    let decisions: Vec<u8> = plans
        .iter()
        .map(|plan| {
            DecisionRecord {
                plan: *plan,
                ..decision()
            }
            .encode()
        })
        .collect::<Vec<_>>()
        .concat();
    assert_eq!(
        decisions.len(),
        3 * DECISION_RECORD_BYTES,
        "no decision record grows"
    );
}

#[test]
fn outcomes_never_feed_a_coordinate() {
    let record = decision();
    let plan = record.plan;
    let before = minute_day(&[bar(at(10, 0, 0), P, P + 100, P - 50, P, 1_000)]);
    let mut after = before.clone();
    for b in &mut after.bars {
        if b.ts_micros >= ts(plan.decision_second) {
            *b = mirror_bar(b);
        }
    }
    after.trail = after.bars.iter().map(candle_of).collect();
    let walked = minute_slot(&before, plan, STOP_ONLY);
    let perturbed = minute_slot(&after, plan, STOP_ONLY);
    assert_ne!(walked, perturbed, "the perturbation reached the outcome");
    // The decision record is built from the plan and the cell alone: no bar
    // after the decision can reach it.
    assert_eq!(record.encode(), decision().encode());
}
