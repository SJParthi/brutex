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

#![allow(clippy::inconsistent_digit_grouping, clippy::too_many_lines)]

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

fn main() {
    println!("CHG-1 charge stack — {SAMPLES} samples of {BATCH} calls each, per call:");
    let Ok(day) = TradeDay::new(2026, 10, 15) else {
        println!("  BENCH SETUP FAILED: the trade day");
        std::process::exit(1);
    };
    let intraday = TradeKind::Equity(Product::Intraday);
    let Ok(card) = RateCard::resolve(intraday, Exchange::Nse, day, RateSelector::default()) else {
        println!("  BENCH SETUP FAILED: the rate card");
        std::process::exit(1);
    };
    let (Some(small_buy), Some(small_sell), Some(huge_buy), Some(huge_sell)) = (
        order(OrderSide::Buy, 1, 1_00),
        order(OrderSide::Sell, 1, 1_01),
        order(OrderSide::Buy, 10_000_000, 100_000_00),
        order(OrderSide::Sell, 10_000_000, 100_100_00),
    ) else {
        std::process::exit(1);
    };

    // Per executed order: one `add` on a day that already holds orders.
    let fresh = ClientSecurityDay::with_card(card);
    let add_small = spread(|| {
        let mut d = black_box(fresh);
        d.add(black_box(small_buy))
    });
    let add_huge = spread(|| {
        let mut d = black_box(fresh);
        d.add(black_box(huge_buy))
    });
    let mut crowded = fresh;
    for _ in 0..1_000_000 {
        if crowded.add(small_buy).and(crowded.add(small_sell)).is_err() {
            println!("  BENCH SETUP FAILED: a million orders");
            std::process::exit(1);
        }
    }
    let add_crowded = spread(|| {
        let mut d = black_box(crowded);
        d.add(black_box(small_buy))
    });
    report("per order: add, one share at ₹1", add_small);
    report("per order: add, a crore of shares at ₹1,00,000", add_huge);
    report(
        "per order: add, to a day already holding 2,000,000",
        add_crowded,
    );

    // Per client-security-day: open (resolve the card), two orders, settle.
    let whole_day = |buy: Order, sell: Order| {
        move || {
            let mut d = ClientSecurityDay::open(
                black_box(intraday),
                black_box(Exchange::Nse),
                black_box(day),
                RateSelector::default(),
            )?;
            d.add(buy)?;
            d.add(sell)?;
            d.settle()
        }
    };
    let day_small = spread(whole_day(small_buy, small_sell));
    let day_huge = spread(whole_day(huge_buy, huge_sell));
    report(
        "per client-security-day: open + 2 orders + settle, small",
        day_small,
    );
    report(
        "per client-security-day: open + 2 orders + settle, huge",
        day_huge,
    );

    // Settlement alone, of one round trip and of two million orders.
    let mut one_trip = fresh;
    if one_trip
        .add(small_buy)
        .and(one_trip.add(small_sell))
        .is_err()
    {
        std::process::exit(1);
    }
    let settle_one = spread(|| black_box(one_trip).settle());
    let settle_crowded = spread(|| black_box(crowded).settle());
    report("settle: a day of 2 orders", settle_one);
    report("settle: a day of 2,000,000 orders", settle_crowded);

    // The dated selector: the regime walks on top of the same stack.
    let dated_option_day = spread(|| {
        ClientSecurityDay::open(
            TradeKind::Options,
            Exchange::Nse,
            black_box(day),
            RateSelector::Dated,
        )
    });
    report(
        "open a dated options day (regime lookups)",
        dated_option_day,
    );

    // The calculator replica, per quote.
    let dhan_small = spread(|| {
        quote(
            black_box(intraday),
            Exchange::Nse,
            black_box(1),
            Some(Paisa::from_raw(1_00)),
            Some(Paisa::from_raw(1_01)),
        )
    });
    let dhan_huge = spread(|| {
        quote(
            black_box(intraday),
            Exchange::Nse,
            black_box(10_000_000),
            Some(Paisa::from_raw(100_000_00)),
            Some(Paisa::from_raw(100_100_00)),
        )
    });
    report("Dhan calculator quote, small", dhan_small);
    report("Dhan calculator quote, huge", dhan_huge);

    println!("ratios of medians, ceiling 3.000x:");
    let mut ok = true;
    ok &= ratio(
        "C-CHG-01 add: a crore of shares vs one share",
        add_small,
        add_huge,
    );
    ok &= ratio(
        "C-CHG-02 add: the 2,000,001st order vs the first",
        add_small,
        add_crowded,
    );
    ok &= ratio("C-CHG-03 day: huge vs small", day_small, day_huge);
    ok &= ratio(
        "C-CHG-04 settle: 2,000,000 orders vs 2",
        settle_one,
        settle_crowded,
    );
    ok &= ratio("C-CHG-05 Dhan quote: huge vs small", dhan_small, dhan_huge);
    if !ok {
        println!("CHG-1 STACK BENCH FAILED: a ratio breached its ceiling");
        std::process::exit(1);
    }
    println!("OK — the charge stack costs the same whatever it is asked.");
}
