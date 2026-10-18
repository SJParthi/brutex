//! The bench rows of the walker (FIB-5; D-6380): ONE BAR STEP OF ONE SLOT,
//! the walker's unit of work, at three active-slot counts and three holds,
//! at both resolutions.
//!
//! Every slot carries all five exit components (a stop, a target, a
//! give-back, a trailing stop and, on the hold rows, a time exit), each
//! evaluated on every bar and none reached before its hold ends, so every
//! step does the full comparison set. Slots walk in production order — each
//! to its exit, then the next — over a ring of synthetic stock-days whose
//! indexes together exceed this host's last-level cache (FM-F002); a finished
//! slot restarts on the next day of the ring. The structural work counter is
//! the stepper's own: one per bar stepped plus one per swing absorbed.
//!
//! Synthetic: prices near Rs 500.00 on invented days; nothing is a market
//! fact.

use std::fmt::Write as _;
use std::sync::OnceLock;
use std::time::Instant;

use indicators::fibflag::types::{EntryStyle, FibCandle};
use indicators::{Candle, OI_NULL};
use runner::fib::types::{
    BeyondTarget, ClockRule, CrossFeed, ExitFill, HaltRule, PriceMap, Resolution, SnapshotEdge,
    TradeSize, WalkProfile,
};
use runner::fib::walk::exits::{
    ExitSet, ExitVariant, FillVariant, StopLoss, TakeProfit, TimeExit, TrailingStop,
    TrailingTakeProfit,
};
use runner::fib::walk::{
    BookEvidence, ClockLag, DayIndex, DayInput, Plan, Stepper, TrailCandles, index_day,
};
use vocab::fib_cube::Search;

use crate::harness::{Axis, Class, Host, Row, run};

/// The first invented day.
const DAY0: i64 = 19_723;
/// Deep days in the ring: about 6.5 MB each.
const SECOND_DAYS: usize = 8;
/// Stage-A days in the ring: about 0.4 MB each.
const MINUTE_DAYS: usize = 100;
/// 09:15:00 IST.
const OPEN: u32 = 33_300;
/// 15:10:00 IST: the index keeps nothing from here.
const AUTO_EXIT: u32 = 54_600;
/// The deep hold scales: 10 seconds, 1,000 seconds, and 09:15:00 to 15:09:00.
const DEEP_HOLDS: [u64; 3] = [10, 1_000, 21_240];
/// The stage-A hold scales, in minutes: the longest is 09:15 to 15:09.
const MINUTE_HOLDS: [u64; 3] = [3, 35, 354];

fn ts(day: usize, second: u32) -> i64 {
    ((DAY0 + i64::try_from(day).unwrap()) * 86_400 + i64::from(second) - 19_800) * 1_000_000
}

/// splitmix64.
fn mix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One synthetic stock-day: bars every second (or minute) from 09:15:00 to
/// 15:09:59, a random walk around Rs 500.00, its Zerodha minutes and its
/// one-minute candles.
fn synthetic(day: usize, resolution: Resolution) -> (Vec<Candle>, Vec<Candle>, Vec<FibCandle>) {
    let mut seed = 0xF1B5_0000 + day as u64;
    let step = if resolution == Resolution::Minute {
        60
    } else {
        1
    };
    let mut price: i64 = 50_000;
    let mut bars = Vec::new();
    let mut second = OPEN;
    while second < AUTO_EXIT {
        let open = price;
        price += i64::try_from(mix(&mut seed) % 61).unwrap() - 30;
        let high = open.max(price) + i64::try_from(mix(&mut seed) % 20).unwrap();
        let low = open.min(price) - i64::try_from(mix(&mut seed) % 20).unwrap();
        let volume = 1 + i64::try_from(mix(&mut seed) % 1_000).unwrap();
        bars.push(Candle::new(
            ts(day, second),
            open,
            high,
            low,
            price,
            volume,
            OI_NULL,
        ));
        second += step;
    }
    let mut minutes: Vec<Candle> = Vec::new();
    for b in &bars {
        let start = b.ts_micros - b.ts_micros.rem_euclid(60_000_000);
        match minutes.last_mut() {
            Some(m) if m.ts_micros == start => {
                m.high = m.high.max(b.high);
                m.low = m.low.min(b.low);
                m.close = b.close;
                m.volume += b.volume;
            }
            _ => minutes.push(Candle::new(
                start, b.open, b.high, b.low, b.close, b.volume, OI_NULL,
            )),
        }
    }
    let trail = minutes
        .iter()
        .map(|m| FibCandle {
            candle: *m,
            tf_minutes: 1,
            complete: true,
            missing_minutes: 0,
            close_ts_micros: m.ts_micros + 60_000_000,
        })
        .collect();
    (bars, minutes, trail)
}

/// Every component present and none reached: a stop and a trail far below, a
/// target and an R-step far above, a give-back to the entry reference.
const COMPOSITE: ExitVariant = ExitVariant {
    stop: StopLoss::BeyondBase { extra_ticks: 0 },
    take_profit: TakeProfit::RMultiple { milli_r: 1_000_000 },
    trailing_take_profit: TrailingTakeProfit::GiveBack { permille: 1_000 },
    trailing_stop: TrailingStop::RStep { milli_r: 1_000_000 },
    time_exit: TimeExit::Absent,
};

/// The composite with the swing trail, its buffer far below every swing.
const SWING: ExitVariant = ExitVariant {
    trailing_stop: TrailingStop::Swing {
        width: 1,
        buffer_paisa: 40_000,
    },
    ..COMPOSITE
};

/// The composite with a hold of `seconds`.
const fn hold(seconds: u32) -> ExitVariant {
    ExitVariant {
        time_exit: TimeExit::Hold { seconds },
        ..COMPOSITE
    }
}

/// The exit set of one resolution's rows: ids 0 composite, 1 swing, 2..4 the
/// holds.
fn exit_set(resolution: Resolution) -> ExitSet {
    let holds = match resolution {
        Resolution::Second => DEEP_HOLDS,
        Resolution::Minute => MINUTE_HOLDS.map(|m| m * 60),
    };
    let mut variants = vec![COMPOSITE, SWING];
    variants.extend(holds.map(|h| hold(u32::try_from(h).unwrap())));
    ExitSet::new(Search::Long, 1, 0, variants).unwrap()
}

/// One resolution's ring of days, its exit set and the build times.
struct Ring {
    exits: ExitSet,
    days: Vec<DayIndex>,
    build_ns: Vec<u128>,
}

fn ring(resolution: Resolution) -> Ring {
    let exits = exit_set(resolution);
    let count = match resolution {
        Resolution::Second => SECOND_DAYS,
        Resolution::Minute => MINUTE_DAYS,
    };
    let profile = WalkProfile {
        resolution,
        price_map: PriceMap::Identity,
        exit_fill: ExitFill::SpillByVolume,
        snapshot_edge: SnapshotEdge::StampOpensInterval,
        halt_rule: HaltRule::WorstSeenThroughHalt,
        cross_feed: CrossFeed::SecondsOnly,
        beyond_target: BeyondTarget::FiresOnFirstBar,
        clock_rule: ClockRule::RefuseShifted,
    };
    let mut days = Vec::with_capacity(count);
    let mut build_ns = Vec::with_capacity(count);
    for day in 0..count {
        let (bars, minutes, trail) = synthetic(day, resolution);
        let trails = [TrailCandles {
            tf_minutes: 1,
            candles: &trail,
        }];
        let input = DayInput {
            bars: &bars,
            book: BookEvidence::NotObserved,
            halts: &[],
            zerodha_minutes: if resolution == Resolution::Second {
                &minutes
            } else {
                &[]
            },
            raw_tick: 5,
            decision_tick: 5,
            max_second_gap: u32::MAX,
            clock_lag: ClockLag::Measured(0),
            trail: &trails,
        };
        let start = Instant::now();
        let index = index_day(&profile, &input, &exits).unwrap();
        build_ns.push(start.elapsed().as_nanos());
        days.push(index);
    }
    Ring {
        exits,
        days,
        build_ns,
    }
}

fn deep_ring() -> &'static Ring {
    static RING: OnceLock<Ring> = OnceLock::new();
    RING.get_or_init(|| ring(Resolution::Second))
}

fn minute_ring() -> &'static Ring {
    static RING: OnceLock<Ring> = OnceLock::new();
    RING.get_or_init(|| ring(Resolution::Minute))
}

/// A long decided at 09:15:00 with its entry reference far below the market,
/// so no component is reached before its hold ends.
const PLAN: Plan = Plan {
    direction: Search::Long,
    style: EntryStyle::Close,
    tf_minutes: 1,
    decision_second: OPEN,
    entry_window_end: OPEN + 60,
    entry_ref: 30_000,
    base_edge: 10_000,
    trigger_edge: 30_000,
    fib_zero: 10_000,
    fib_one: 90_000,
    trail: 0,
};

/// The walker's state for one row: slots stepping one at a time over a ring.
pub(crate) struct Walking {
    ring: &'static Ring,
    variant: ExitVariant,
    slots: Vec<Stepper<'static>>,
    at: usize,
    turns: usize,
    work: u64,
}

fn stepper(ring: &'static Ring, day: usize, variant: &ExitVariant) -> Stepper<'static> {
    Stepper::new(
        &ring.days[day % ring.days.len()],
        &PLAN,
        0,
        variant,
        FillVariant::STAGE_A,
        TradeSize::Small,
    )
    .unwrap()
}

fn walking(ring: &'static Ring, id: u16, slots: u64) -> Walking {
    let variant = *ring.exits.get(id).unwrap();
    let slots = (0..slots as usize)
        .map(|j| stepper(ring, j, &variant))
        .collect();
    Walking {
        ring,
        variant,
        slots,
        at: 0,
        turns: 0,
        work: 0,
    }
}

/// One bar step of the current slot; a finished slot restarts on the next
/// day of the ring and the walk moves to the next slot.
fn step(state: &mut Walking, _i: u64) -> u64 {
    let slot = &mut state.slots[state.at];
    let before = slot.work();
    let more = slot.step();
    state.work += slot.work() - before;
    if !more {
        state.turns += 1;
        *slot = stepper(state.ring, state.at + state.turns, &state.variant);
        state.at = (state.at + 1) % state.slots.len();
    }
    state.work
}

/// One whole pair walked BY EVENTS (FM-F030): the current slot runs to its
/// end with [`Stepper::run_events`], restarts on the next day of the ring, and
/// the walk moves to the next slot.
fn pair_by_events(state: &mut Walking, _i: u64) -> u64 {
    let slot = &mut state.slots[state.at];
    slot.run_events();
    state.work += slot.work();
    state.turns += 1;
    *slot = stepper(state.ring, state.at + state.turns, &state.variant);
    state.at = (state.at + 1) % state.slots.len();
    state.work
}

fn work(state: &Walking) -> u64 {
    state.work
}

fn bytes(state: &Walking) -> u64 {
    let days: u64 = state.ring.days.iter().map(DayIndex::bytes).sum();
    days + (state.slots.len() * std::mem::size_of::<Stepper<'static>>()) as u64
}

fn hold_id(holds: [u64; 3], scale: u64) -> u16 {
    2 + u16::try_from(holds.iter().position(|h| *h == scale).unwrap()).unwrap()
}

/// The rows, run and reported. False when any row failed.
pub(crate) fn report(host: &Host, out: &mut String) -> bool {
    let deep_slots = Row {
        id: "C-F5-01",
        name: "deep: one bar step of one slot, every component live, full-day hold",
        class: Some(Class::WorstCase),
        axis: Some(Axis::Data {
            unit: "active slots per stock-day (8 synthetic days of 21,300 seconds)",
            production: None,
        }),
        scales: [1, 16, 256],
        setup: |slots| walking(deep_ring(), 0, slots),
        op: step,
        work,
        bytes,
    };
    let deep_holds = Row {
        id: "C-F5-02",
        name: "deep: one bar step of one slot, every component live, 16 slots",
        class: Some(Class::WorstCase),
        axis: Some(Axis::Data {
            unit: "hold in seconds (10, 1,000, 09:15:00 to 15:09:00)",
            production: Some(DEEP_HOLDS[2]),
        }),
        scales: DEEP_HOLDS,
        setup: |hold| walking(deep_ring(), hold_id(DEEP_HOLDS, hold), 16),
        op: step,
        work,
        bytes,
    };
    let minute_slots = Row {
        id: "C-F5-03",
        name: "stage A: one bar step of one slot, every component live, full-day hold",
        class: Some(Class::WorstCase),
        axis: Some(Axis::Data {
            unit: "active slots per stock-day (100 synthetic days of 355 minutes)",
            production: None,
        }),
        scales: [1, 16, 256],
        setup: |slots| walking(minute_ring(), 0, slots),
        op: step,
        work,
        bytes,
    };
    let minute_holds = Row {
        id: "C-F5-04",
        name: "stage A: one bar step of one slot, every component live, 16 slots",
        class: Some(Class::WorstCase),
        axis: Some(Axis::Data {
            unit: "hold in minutes (3, 35, 09:15 to 15:09)",
            production: Some(MINUTE_HOLDS[2]),
        }),
        scales: MINUTE_HOLDS,
        setup: |hold| walking(minute_ring(), hold_id(MINUTE_HOLDS, hold), 16),
        op: step,
        work,
        bytes,
    };
    let deep_swing = Row {
        id: "C-F5-05",
        name: "deep: one bar step of one slot with the swing trail, full-day hold",
        class: Some(Class::Amortised { period: 60 }),
        axis: Some(Axis::Data {
            unit: "active slots per stock-day (8 synthetic days of 21,300 seconds)",
            production: None,
        }),
        scales: [1, 16, 256],
        setup: |slots| walking(deep_ring(), 1, slots),
        op: step,
        work,
        bytes,
    };
    let deep_events = Row {
        id: "C-F5-07",
        name: "deep: one whole pair walked by events, every component live, full-day hold",
        class: Some(Class::Bounded {
            bound: "O(log bars x (1 + events)) per pair: touches, swings read, new highs while a give-back or an R-step trails",
        }),
        axis: Some(Axis::Data {
            unit: "active slots per stock-day (8 synthetic days of 21,300 seconds)",
            production: None,
        }),
        scales: [1, 16, 256],
        setup: |slots| walking(deep_ring(), 0, slots),
        op: pair_by_events,
        work,
        bytes,
    };
    let mut ok = true;
    for row in [
        deep_slots,
        deep_holds,
        minute_slots,
        minute_holds,
        deep_swing,
        deep_events,
    ] {
        ok &= !run(&row, host, out).fails();
    }
    report_builds(out);
    ok
}

/// C-F5-06: the per-stock-day index builds the rings timed, as information.
fn report_builds(out: &mut String) {
    for (name, ring) in [
        ("deep, 21,300 seconds", deep_ring()),
        ("stage A, 355 minutes", minute_ring()),
    ] {
        let mut ns = ring.build_ns.clone();
        ns.sort_unstable();
        let _ = writeln!(
            out,
            "C-F5-06 DayIndex::build of one stock-day ({name}), BOUNDED O(bars x log bars + seconds of the day + candles x widths), once per stock-day: {} builds, min {} ns, p50 {} ns, max {} ns; {} bytes per index",
            ns.len(),
            ns[0],
            ns[ns.len() / 2],
            ns[ns.len() - 1],
            ring.days[0].bytes()
        );
    }
    let _ = writeln!(out);
}
