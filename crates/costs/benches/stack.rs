//! Package CHG-1's charge stack, timed: p50, p99 and max per executed order
//! and per client-security-day, and the ratios that would show a cost that
//! grows with the trade.
//!
//! # Method
//!
//! A sample is one batch of [`BATCH`] calls timed by one pair of
//! `Instant::now()` reads; the per-call figure is the batch time over
//! [`BATCH`], in picoseconds, so the clock's own cost is spread over the batch
//! rather than charged to one call. [`SAMPLES`] samples per measurement, after
//! [`WARMUP`] untimed ones; p50, p99 and max are read off the sorted samples.
//! `[profile.bench]` inherits release, so this is the shipped arithmetic.
//!
//! # What would falsify the O(1) claim
//!
//! * **The size of a trade must not cost anything.** One share at ₹1 and a
//!   crore of shares at ₹1,00,000 are the same multiplications.
//! * **The thousandth order must cost what the first did.** A day keeps
//!   running totals; if adding an order walked the earlier ones, a day that
//!   already holds a million orders would be slower to add to.
//! * **Settling a day of a million orders must cost what settling one does.**
//! * **A client-day of two million securities must cost what one of one does**
//!   to add to and to settle (C-CHG-06, `crates/costs/benches/stack.rs`).
//! * **The backtest ledger (`costs::ledger::DayLedger`) adds its 2,000,001st
//!   order, and a crore of shares, at the cost of its first** (C-CHG-07), **and
//!   closes 2,000,000 orders at the cost of two, in each mode** (C-CHG-08).
//! * **One order's lines from a ledger of 2,000,000 orders cost what they
//!   cost from a ledger of two, in each mode, and a delivery sell's share of
//!   the day's DP costs the same among 2,000,000 sells as alone** (C-CHG-09).
//! * **An option exercise on the ledger, which now carries the ₹20 brokerage
//!   of sources row B-05, costs the same for the 1,000,001st as for the
//!   first and for ₹1,000 crore as for ₹1, and a day of 1,000,000 exercises
//!   closes at the cost of one** (C-CHG-10, D-5132).
//!
//! The ledger's steps are also timed ONE call per sample and printed beside
//! a sample of two clock reads alone, so the per-call p99 and max are shown
//! undiluted by the batch mean; those figures carry the clock's own cost and
//! are reported, not gated (D-5131).
//!
//! Each is a ratio of medians held under [`CEILING_PERMILLE`]; a breach exits
//! non-zero. The absolute figures are printed and are NOT gated: they belong
//! to the machine that ran them.
//!
//! No benchmarking framework, for the reason `crates/costs/benches/ratio.rs`
//! gives: it would be a dependency.

#![allow(clippy::inconsistent_digit_grouping)]

use std::hint::black_box;
use std::time::Instant;

use brutex_core::instrument::Exchange;
use brutex_core::price::Paisa;
use costs::day::TradeDay;
use costs::dhan_calc::quote;
use costs::ledger::{DayLedger, ExecutedOrder};
use costs::rate::OrderSide;
use costs::schedule::{Mode, Product, RateCard, RateSelector, Settlement, TradeKind};
use costs::stack::{ClientDay, ClientSecurityDay, Order};

/// A ratio of medians above this is a failure. Thousandths: 3.0x, the
/// shared-CI number `docs/04-invariants.md` uses for every ratio row.
const CEILING_PERMILLE: u128 = 3_000;

/// Calls per timed sample.
const BATCH: u32 = 16;

/// Timed samples per measurement.
const SAMPLES: usize = 20_000;

/// Untimed samples first.
const WARMUP: usize = 2_000;

/// p50, p99 and max per call, in picoseconds.
#[derive(Clone, Copy)]
struct Spread {
    p50: u128,
    p99: u128,
    max: u128,
}

/// Times `op`, [`BATCH`] calls per sample.
fn spread<T>(op: impl FnMut() -> T) -> Spread {
    spread_of(BATCH, op)
}

/// Times `op` ONE call per sample: the per-call p99 and max that a batch
/// mean dilutes (one slow call is averaged with fifteen fast ones), bounded
/// below by the clock's own resolution and cost — so reported, never gated.
fn spread_unbatched<T>(op: impl FnMut() -> T) -> Spread {
    spread_of(1, op)
}

/// Times `op`, `batch` calls per sample, per call.
fn spread_of<T>(batch: u32, mut op: impl FnMut() -> T) -> Spread {
    for _ in 0..WARMUP {
        for _ in 0..batch {
            black_box(op());
        }
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..batch {
            black_box(op());
        }
        samples.push(start.elapsed().as_nanos() * 1_000 / u128::from(batch));
    }
    samples.sort_unstable();
    let at = |permille: usize| samples.get(SAMPLES * permille / 1_000).copied();
    Spread {
        p50: at(500).unwrap_or(u128::MAX),
        p99: at(990).unwrap_or(u128::MAX),
        max: samples.last().copied().unwrap_or(u128::MAX),
    }
}

/// `1234567` picoseconds as `1234.6 ns`.
fn ns(ps: u128) -> String {
    format!("{}.{} ns", ps / 1_000, (ps % 1_000) / 100)
}

fn report(label: &str, s: Spread) {
    println!(
        "  {label:<58} p50 {:>10}  p99 {:>10}  max {:>11}",
        ns(s.p50),
        ns(s.p99),
        ns(s.max)
    );
}

/// Prints one ratio of medians and returns whether it held.
fn ratio(label: &str, base: Spread, at: Spread) -> bool {
    if base.p50 == 0 {
        println!("  {label:<58} UNMEASURABLE — the baseline timed at zero");
        return false;
    }
    let permille = at.p50 * 1_000 / base.p50;
    let ok = permille <= CEILING_PERMILLE;
    println!(
        "  {label:<58} ratio {}.{:03}x  {}",
        permille / 1_000,
        permille % 1_000,
        if ok { "ok" } else { "BREACH" }
    );
    ok
}

fn order(side: OrderSide, quantity: i64, price: i64) -> Option<Order> {
    match Order::new(side, quantity, Paisa::from_raw(price)) {
        Ok(order) => Some(order),
        Err(reason) => {
            println!("  BENCH SETUP FAILED: {reason}");
            None
        }
    }
}

/// Everything the measurements share, built once.
struct Fixture {
    day: TradeDay,
    fresh: ClientSecurityDay,
    crowded: ClientSecurityDay,
    small_buy: Order,
    small_sell: Order,
    huge_buy: Order,
    huge_sell: Order,
}

const INTRADAY: TradeKind = TradeKind::Equity(Product::Intraday);

/// The fixture, or a loud failure: the bench refuses to measure something it
/// could not set up, rather than measuring something else.
fn fixture() -> Option<Fixture> {
    let day = TradeDay::new(2026, 10, 15).ok()?;
    let card = RateCard::resolve(
        INTRADAY,
        Exchange::Nse,
        day,
        RateSelector::CurrentOnEveryDate,
    )
    .ok()?;
    let small_buy = order(OrderSide::Buy, 1, 1_00)?;
    let small_sell = order(OrderSide::Sell, 1, 1_01)?;
    let fresh = ClientSecurityDay::with_card(card);
    let mut crowded = fresh;
    for _ in 0..1_000_000 {
        crowded.add(small_buy).ok()?;
        crowded.add(small_sell).ok()?;
    }
    Some(Fixture {
        day,
        fresh,
        crowded,
        small_buy,
        small_sell,
        huge_buy: order(OrderSide::Buy, 10_000_000, 100_000_00)?,
        huge_sell: order(OrderSide::Sell, 10_000_000, 100_100_00)?,
    })
}

/// C-CHG-01 — one executed order costs the same whatever its size.
fn an_orders_cost_does_not_track_its_size(f: &Fixture) -> bool {
    let add = |order: Order| {
        let fresh = f.fresh;
        spread(move || {
            let mut d = black_box(fresh);
            d.add(black_box(order))
        })
    };
    let small = add(f.small_buy);
    let huge = add(f.huge_buy);
    report("per order: add, one share at ₹1", small);
    report("per order: add, a crore of shares at ₹1,00,000", huge);
    ratio("C-CHG-01 add: a crore of shares vs one share", small, huge)
}

/// C-CHG-02 — the 2,000,001st order costs what the first did.
fn an_orders_cost_does_not_track_the_orders_before_it(f: &Fixture) -> bool {
    let (fresh, crowded, buy) = (f.fresh, f.crowded, f.small_buy);
    let first = spread(move || {
        let mut d = black_box(fresh);
        d.add(black_box(buy))
    });
    let later = spread(move || {
        let mut d = black_box(crowded);
        d.add(black_box(buy))
    });
    report("per order: add, to a day already holding 2,000,000", later);
    ratio(
        "C-CHG-02 add: the 2,000,001st order vs the first",
        first,
        later,
    )
}

/// C-CHG-03 — a whole client-security-day (open, two orders, settle) costs
/// the same whatever its size.
fn a_days_cost_does_not_track_its_size(f: &Fixture) -> bool {
    let day = f.day;
    let whole_day = move |buy: Order, sell: Order| {
        spread(move || {
            let mut d = ClientSecurityDay::open(
                black_box(INTRADAY),
                black_box(Exchange::Nse),
                black_box(day),
                RateSelector::CurrentOnEveryDate,
            )?;
            d.add(buy)?;
            d.add(sell)?;
            d.settle()
        })
    };
    let small = whole_day(f.small_buy, f.small_sell);
    let huge = whole_day(f.huge_buy, f.huge_sell);
    report(
        "per client-security-day: open + 2 orders + settle, small",
        small,
    );
    report(
        "per client-security-day: open + 2 orders + settle, huge",
        huge,
    );
    let gated = ratio("C-CHG-03 day: huge vs small", small, huge);
    // The widest day i64 allows: ₹4 crore a unit, a hundred crore units a
    // side, a turnover of 8 x 10^18 paisa. Its rate products pass 2^64, where
    // i128 division leaves the hardware's 64-bit fast path for the
    // software routine. That routine's cost is bounded by the 128-bit width,
    // not by the magnitude, so the cost is still constant; it is a
    // different constant, so it is printed and NOT gated (D-5118).
    let (Some(wide_buy), Some(wide_sell)) = (
        order(OrderSide::Buy, 1_000_000_000, 40_000_000_00),
        order(OrderSide::Sell, 1_000_000_000, 40_000_000_00),
    ) else {
        return false;
    };
    // It must SETTLE: timing a refusal would time the wrong thing.
    let settled = ClientSecurityDay::open(
        INTRADAY,
        Exchange::Nse,
        f.day,
        RateSelector::CurrentOnEveryDate,
    )
    .and_then(|mut d| {
        d.add(wide_buy)?;
        d.add(wide_sell)?;
        d.settle()
    });
    if let Err(reason) = settled {
        println!("  BENCH SETUP FAILED: the widest day refused: {reason}");
        return false;
    }
    {
        let widest = whole_day(wide_buy, wide_sell);
        report("per client-security-day: the widest i64 allows", widest);
        let _ = ratio(
            "  (reported, not gated) day: widest vs small",
            small,
            widest,
        );
    }
    gated
}

/// C-CHG-04 — settling a day of two million orders costs what settling two
/// does: the day keeps totals and never walks its orders.
fn settling_does_not_track_how_many_orders_the_day_holds(f: &Fixture) -> bool {
    let mut one_trip = f.fresh;
    if one_trip
        .add(f.small_buy)
        .and(one_trip.add(f.small_sell))
        .is_err()
    {
        println!("  BENCH SETUP FAILED: one round trip");
        return false;
    }
    let crowded = f.crowded;
    let two = spread(move || black_box(one_trip).settle());
    let many = spread(move || black_box(crowded).settle());
    report("settle: a day of 2 orders", two);
    report("settle: a day of 2,000,000 orders", many);
    ratio("C-CHG-04 settle: 2,000,000 orders vs 2", two, many)
}

/// C-CHG-05 — a calculator quote costs the same whatever its size.
fn a_calculator_quote_does_not_track_its_size() -> bool {
    let quoted = |quantity: i64, buy: i64, sell: i64| {
        spread(move || {
            quote(
                black_box(INTRADAY),
                Exchange::Nse,
                black_box(quantity),
                Some(Paisa::from_raw(buy)),
                Some(Paisa::from_raw(sell)),
            )
        })
    };
    let small = quoted(1, 1_00, 1_01);
    let huge = quoted(10_000_000, 100_000_00, 100_100_00);
    report("Dhan calculator quote, small", small);
    report("Dhan calculator quote, huge", huge);
    ratio("C-CHG-05 Dhan quote: huge vs small", small, huge)
}

/// C-CHG-06 — adding a settled security to a client-day, and settling the
/// client-day, cost the same whether it already holds one security or two
/// million: the client-day keeps totals and rounds its STT and stamp duty once.
fn a_client_days_cost_does_not_track_how_many_securities_it_holds(f: &Fixture) -> bool {
    let mut one_trip = f.fresh;
    let settled = one_trip
        .add(f.small_buy)
        .and(one_trip.add(f.small_sell))
        .and_then(|_| one_trip.settle());
    let Ok(settled) = settled else {
        println!("  BENCH SETUP FAILED: one round trip");
        return false;
    };
    let empty = ClientDay::new();
    let mut crowded = empty;
    for _ in 0..2_000_000 {
        if crowded.add(&settled).is_err() {
            println!("  BENCH SETUP FAILED: a crowded client-day");
            return false;
        }
    }
    let first = spread(move || {
        let mut client = black_box(empty);
        client
            .add(black_box(&settled))
            .and_then(|()| client.settle())
    });
    let later = spread(move || {
        let mut client = black_box(crowded);
        client
            .add(black_box(&settled))
            .and_then(|()| client.settle())
    });
    report("client-day: add + settle, the first security", first);
    report("client-day: add + settle, the 2,000,001st security", later);
    ratio(
        "C-CHG-06 client-day: 2,000,001st security vs the first",
        first,
        later,
    )
}

/// A ledger order of `qty` units worth `value` paisa.
const fn executed(side: OrderSide, qty: i64, value: i64) -> ExecutedOrder {
    ExecutedOrder {
        side,
        qty,
        value_paisa: Paisa::from_raw(value),
        broker_square_off: false,
    }
}

/// An empty ledger and one holding 2,000,000 small orders, or a loud failure.
fn ledgers(f: &Fixture) -> Option<(DayLedger, DayLedger)> {
    let fresh = DayLedger::new(INTRADAY, Exchange::Nse, f.day).ok()?;
    let mut crowded = fresh;
    for _ in 0..1_000_000 {
        crowded.add(executed(OrderSide::Buy, 1, 1_00)).ok()?;
        crowded.add(executed(OrderSide::Sell, 1, 1_01)).ok()?;
    }
    Some((fresh, crowded))
}

/// C-CHG-07 — the backtest ledger's add costs the same for the 2,000,001st
/// order as for the first, and for a crore of shares as for one.
fn a_ledger_add_does_not_track_the_orders_before_it_or_its_size(f: &Fixture) -> bool {
    let Some((fresh, crowded)) = ledgers(f) else {
        println!("  BENCH SETUP FAILED: the ledgers");
        return false;
    };
    let small = executed(OrderSide::Buy, 1, 1_00);
    let huge = executed(OrderSide::Buy, 10_000_000, 1_000_000_000_000);
    let add = |day: DayLedger, order: ExecutedOrder| {
        spread(move || {
            let mut d = black_box(day);
            d.add(black_box(order))
        })
    };
    let first = add(fresh, small);
    let later = add(crowded, small);
    let large = add(fresh, huge);
    report("ledger add: the first order, one share at ₹1", first);
    report("ledger add: the 2,000,001st order", later);
    report("ledger add: a crore of shares at ₹1,00,000", large);
    let held = ratio(
        "C-CHG-07 ledger add: the 2,000,001st order vs the first",
        first,
        later,
    );
    ratio(
        "C-CHG-07 ledger add: a crore of shares vs one",
        first,
        large,
    ) && held
}

/// C-CHG-08 — closing a ledger of 2,000,000 orders costs what closing one of
/// two does, in each mode: the close prices the totals and never the orders.
fn a_ledger_close_does_not_track_how_many_orders_it_holds(f: &Fixture) -> bool {
    let Some((fresh, crowded)) = ledgers(f) else {
        println!("  BENCH SETUP FAILED: the ledgers");
        return false;
    };
    let mut two = fresh;
    if two
        .add(executed(OrderSide::Buy, 1, 1_00))
        .and(two.add(executed(OrderSide::Sell, 1, 1_01)))
        .is_err()
    {
        println!("  BENCH SETUP FAILED: one round trip");
        return false;
    }
    let mut ok = true;
    for (mode, name) in [
        (Mode::Precise, "precise"),
        (Mode::DhanCalculator, "Dhan calculator"),
    ] {
        let close = |day: DayLedger| {
            spread(move || black_box(day).close(black_box(mode), RateSelector::CurrentOnEveryDate))
        };
        let small = close(two);
        let many = close(crowded);
        report(&format!("ledger close ({name}): 2 orders"), small);
        report(&format!("ledger close ({name}): 2,000,000 orders"), many);
        ok &= ratio(
            &format!("C-CHG-08 ledger close ({name}): 2,000,000 orders vs 2"),
            small,
            many,
        );
    }
    ok
}

/// C-CHG-09 — one order's own lines, priced against its day's netting, cost
/// the same on a day of 2,000,000 orders as on a day of two, in each mode,
/// and a one-sided delivery day's DP share costs the same for 2,000,000 sells
/// as for one: the lines read the day's totals and never its orders.
fn a_ledgers_order_lines_do_not_track_how_many_orders_it_holds(f: &Fixture) -> bool {
    let Some((fresh, crowded)) = ledgers(f) else {
        println!("  BENCH SETUP FAILED: the ledgers");
        return false;
    };
    let mut two = fresh;
    let sell = executed(OrderSide::Sell, 1, 1_01);
    if two
        .add(executed(OrderSide::Buy, 1, 1_00))
        .and(two.add(sell))
        .is_err()
    {
        println!("  BENCH SETUP FAILED: one round trip");
        return false;
    }
    let lines = |day: DayLedger, order: ExecutedOrder, mode: Mode| {
        spread(move || {
            black_box(day).order_lines_unrounded(
                black_box(order),
                black_box(mode),
                RateSelector::CurrentOnEveryDate,
            )
        })
    };
    let mut ok = true;
    for (mode, name) in [
        (Mode::Precise, "precise"),
        (Mode::DhanCalculator, "Dhan calculator"),
    ] {
        let small = lines(two, sell, mode);
        let many = lines(crowded, sell, mode);
        report(&format!("order lines ({name}): a day of 2 orders"), small);
        report(&format!("order lines ({name}): a day of 2,000,000"), many);
        ok &= ratio(
            &format!("C-CHG-09 order lines ({name}): 2,000,000 orders vs 2"),
            small,
            many,
        );
    }
    // A one-sided delivery day: the sell's share of the day's one DP.
    let Ok(held) = DayLedger::new(TradeKind::Equity(Product::Delivery), Exchange::Nse, f.day)
    else {
        println!("  BENCH SETUP FAILED: the delivery ledger");
        return false;
    };
    let (mut one, mut sells) = (held, held);
    let mut set_up = one.add(sell).is_ok();
    for _ in 0..2_000_000 {
        set_up &= sells.add(sell).is_ok();
    }
    if !set_up {
        println!("  BENCH SETUP FAILED: the delivery sells");
        return false;
    }
    let alone = lines(one, sell, Mode::Precise);
    let shared = lines(sells, sell, Mode::Precise);
    report(
        "order lines (precise): DP share, the day's only sell",
        alone,
    );
    report(
        "order lines (precise): DP share, 1 of 2,000,000 sells",
        shared,
    );
    ok &= ratio(
        "C-CHG-09 order lines (precise DP share): 2,000,000 sells vs 1",
        alone,
        shared,
    );
    ok
}

/// C-CHG-10 — recording an option exercise on the backtest ledger costs the
/// same for the 1,000,001st exercise as for the first, and for ₹1,000 crore
/// of intrinsic value as for ₹1, and closing a day of 1,000,000 exercises
/// costs what closing a day of one does: an exercise is four checked
/// additions — its count, its value, and the ₹20 brokerage of sources row
/// B-05 rounded and unrounded — and the close reads the totals (D-5132).
fn a_ledger_exercise_does_not_track_the_exercises_before_it_or_its_size(f: &Fixture) -> bool {
    let Ok(fresh) = DayLedger::new(TradeKind::Options, Exchange::Nse, f.day) else {
        println!("  BENCH SETUP FAILED: the options ledger");
        return false;
    };
    let small = Paisa::from_raw(1_00);
    let huge = Paisa::from_raw(1_000_000_000_000);
    let (mut one, mut crowded) = (fresh, fresh);
    let mut set_up = one.exercise(small, Settlement::Cash).is_ok();
    for _ in 0..1_000_000 {
        set_up &= crowded.exercise(small, Settlement::Cash).is_ok();
    }
    if !set_up {
        println!("  BENCH SETUP FAILED: the exercises");
        return false;
    }
    let exercise = |day: DayLedger, value: Paisa| {
        spread(move || {
            let mut d = black_box(day);
            d.exercise(black_box(value), Settlement::Cash)
        })
    };
    let first = exercise(fresh, small);
    let later = exercise(crowded, small);
    let large = exercise(fresh, huge);
    report("ledger exercise: the first, ₹1 of intrinsic value", first);
    report("ledger exercise: the 1,000,001st", later);
    report("ledger exercise: ₹1,000 crore of intrinsic value", large);
    // Reported, not gated, as the ledger's other unbatched steps are.
    report(
        "unbatched: ledger exercise, the 1,000,001st",
        spread_unbatched(move || {
            let mut d = black_box(crowded);
            d.exercise(black_box(small), Settlement::Cash)
        }),
    );
    let close = |day: DayLedger| {
        spread(move || {
            black_box(day).close(black_box(Mode::Precise), RateSelector::CurrentOnEveryDate)
        })
    };
    let closed_one = close(one);
    let closed_many = close(crowded);
    report("ledger close (precise): 1 exercise", closed_one);
    report("ledger close (precise): 1,000,000 exercises", closed_many);
    let held = ratio(
        "C-CHG-10 ledger exercise: the 1,000,001st vs the first",
        first,
        later,
    );
    let sized = ratio("C-CHG-10 ledger exercise: ₹1,000 crore vs ₹1", first, large);
    ratio(
        "C-CHG-10 ledger close (precise): 1,000,000 exercises vs 1",
        closed_one,
        closed_many,
    ) && held
        && sized
}

/// Reported, not gated: the ledger's three steps timed ONE call per sample,
/// so the p99 and max are per call rather than per 16-call mean. Each figure
/// carries the clock's own read (two `Instant::now()` per sample), so it is
/// an upper bound on the call, and the method is named in the output.
fn the_ledgers_steps_one_call_per_sample(f: &Fixture) -> bool {
    let Some((fresh, crowded)) = ledgers(f) else {
        println!("  BENCH SETUP FAILED: the ledgers");
        return false;
    };
    let order = executed(OrderSide::Sell, 1, 1_01);
    report(
        "unbatched: ledger add, the 2,000,001st order",
        spread_unbatched(move || {
            let mut d = black_box(crowded);
            d.add(black_box(order))
        }),
    );
    report(
        "unbatched: ledger close (precise), 2,000,000 orders",
        spread_unbatched(move || {
            black_box(crowded).close(black_box(Mode::Precise), RateSelector::CurrentOnEveryDate)
        }),
    );
    report(
        "unbatched: order lines (precise), 2,000,000 orders",
        spread_unbatched(move || {
            black_box(crowded).order_lines_unrounded(
                black_box(order),
                black_box(Mode::Precise),
                RateSelector::CurrentOnEveryDate,
            )
        }),
    );
    report(
        "unbatched: two clock reads and nothing else",
        spread_unbatched(|| black_box(fresh.orders())),
    );
    true
}

/// Reported, not gated: opening a day under the dated selector, which adds
/// the regime's compile-time-bounded walks to the card.
fn the_dated_selector_cost(f: &Fixture) {
    let day = f.day;
    let dated = spread(move || {
        ClientSecurityDay::open(
            TradeKind::Options,
            Exchange::Nse,
            black_box(day),
            RateSelector::Dated,
        )
    });
    report("open a dated options day (regime lookups)", dated);
}

fn main() {
    println!("CHG-1 charge stack — {SAMPLES} samples of {BATCH} calls each, per call:");
    let Some(f) = fixture() else {
        println!("  BENCH SETUP FAILED: the fixture");
        std::process::exit(1);
    };
    let mut ok = true;
    ok &= an_orders_cost_does_not_track_its_size(&f);
    ok &= an_orders_cost_does_not_track_the_orders_before_it(&f);
    ok &= a_days_cost_does_not_track_its_size(&f);
    ok &= settling_does_not_track_how_many_orders_the_day_holds(&f);
    ok &= a_calculator_quote_does_not_track_its_size();
    ok &= a_client_days_cost_does_not_track_how_many_securities_it_holds(&f);
    ok &= a_ledger_add_does_not_track_the_orders_before_it_or_its_size(&f);
    ok &= a_ledger_close_does_not_track_how_many_orders_it_holds(&f);
    ok &= a_ledgers_order_lines_do_not_track_how_many_orders_it_holds(&f);
    ok &= a_ledger_exercise_does_not_track_the_exercises_before_it_or_its_size(&f);
    ok &= the_ledgers_steps_one_call_per_sample(&f);
    the_dated_selector_cost(&f);
    if !ok {
        println!("CHG-1 STACK BENCH FAILED: a ratio breached its ceiling");
        std::process::exit(1);
    }
    println!("OK — the charge stack costs the same whatever it is asked.");
}
