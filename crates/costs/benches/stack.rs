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
use costs::rate::OrderSide;
use costs::schedule::{Product, RateCard, RateSelector, TradeKind};
use costs::stack::{ClientSecurityDay, Order};

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
fn spread<T>(mut op: impl FnMut() -> T) -> Spread {
    for _ in 0..WARMUP {
        for _ in 0..BATCH {
            black_box(op());
        }
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..BATCH {
            black_box(op());
        }
        samples.push(start.elapsed().as_nanos() * 1_000 / u128::from(BATCH));
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
    let card = RateCard::resolve(INTRADAY, Exchange::Nse, day, RateSelector::default()).ok()?;
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
                RateSelector::default(),
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
    ratio("C-CHG-03 day: huge vs small", small, huge)
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
    the_dated_selector_cost(&f);
    if !ok {
        println!("CHG-1 STACK BENCH FAILED: a ratio breached its ceiling");
        std::process::exit(1);
    }
    println!("OK — the charge stack costs the same whatever it is asked.");
}
