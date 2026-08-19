//! Gate 8 for `crates/api` — answering one request costs the same whatever
//! the universe holds.
//!
//! # Why this file did not exist
//!
//! Gate 8 runs `cargo bench --workspace`, and until now only `crates/core` and
//! `crates/store` carried a bench. So the gate was green while the ONE crate
//! that answers requests had never been measured at all. `docs/06-limits.md`
//! §7c lists what gate 8 covers; `api` was not on it, and nothing said so out
//! loud.
//!
//! # What it measures
//!
//! `dashboard_html` recomputes six numbers on every hit of `/`. Every one of
//! them is a property of a universe that is loaded once, wrapped in an `Arc`
//! and never mutated again — so recomputing them per request is not merely
//! O(n), it is O(n) for an answer that cannot have changed.
//!
//! The measurement is per rendered page at 1×, 10× and 100× the instrument
//! count, against a real-scale ceiling: the two masters merged to 90,623
//! distinct instruments on 2026-08-01, which is what 100× stands in for.
//!
//! See `crates/store/benches/ratio.rs` for why there is no benchmarking
//! framework and why every ratio here is integer arithmetic.

use std::collections::HashMap;
use std::hint::black_box;
use std::time::Instant;

use api::merge::{Entry, Merged};
use api::server::{Read, dashboard_html};
use brutex_core::instrument::{Exchange, InstrumentKey, Kind, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::universe::Universe;
use brutex_core::vendor::{Vendor, VendorSet};

/// A ratio above this is a failure. Thousandths, so the comparison is integer.
///
/// 3.0× — `docs/04-invariants.md`, the shared-CI number.
const CEILING_PERMILLE: u128 = 3_000;

/// How many times each measurement is repeated before the minimum is taken.
///
/// Lower than the `core` harness's 60 on purpose: one call here renders a whole
/// page over up to 90,000 instruments, so 60 trials of that is minutes of wall
/// clock for no extra confidence. The minimum of 8 is already stable to well
/// inside the 3× ceiling this gate judges against.
const TRIALS: u32 = 8;

/// The baseline universe size, and the two multiples measured against it.
const BASE: usize = 900;

/// Times one closure, in picoseconds per call, as a minimum over [`TRIALS`].
fn cost_ps<T>(mut op: impl FnMut() -> T) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..TRIALS {
        let start = Instant::now();
        black_box(op());
        let ps = start.elapsed().as_nanos() * 1_000;
        if ps < best {
            best = ps;
        }
    }
    best
}

/// Prints one measurement and returns whether it stayed under the ceiling.
fn ratio(label: &str, base_ps: u128, at_ps: u128) -> bool {
    if base_ps == 0 {
        println!("  {label:<44} UNMEASURABLE — the 1x baseline timed at zero");
        return false;
    }
    let permille = at_ps * 1_000 / base_ps;
    let ok = permille <= CEILING_PERMILLE;
    println!(
        "  {label:<44} {:>11} ps -> {:>11} ps   ratio {}.{:03}x  {}",
        base_ps,
        at_ps,
        permille / 1_000,
        permille % 1_000,
        if ok { "ok" } else { "BREACH" }
    );
    ok
}

/// A synthetic symbol, or a loud exit.
///
/// `expect_used` is denied workspace-wide and a bench is not exempt from it --
/// neither `crates/core` nor `crates/store` suppresses a single lint in theirs.
/// The generated text cannot be invalid, so this arm is unreachable; if it ever
/// runs it names itself and stops, rather than unwinding out of a measurement.
fn symbol(i: usize) -> Symbol {
    let text = format!("S{i}");
    let Ok(symbol) = Symbol::new(&text) else {
        eprintln!("the bench generated a symbol the constructor refused: {text}");
        std::process::exit(1);
    };
    symbol
}

/// A universe of `n` synthetic instruments, spread across the tracked sets.
///
/// The mix matters: the counts under measurement filter on universe
/// membership, so a population that is all one universe would let a compiler
/// hoist work a real population needs.
fn universe_of(n: usize) -> Read {
    let mut by_key = HashMap::with_capacity(n);
    for i in 0..n {
        let key = InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Cash,
            underlying: symbol(i),
            kind: Kind::Equity,
        };
        let universe = match i % 3 {
            0 => Universe::TOTAL_MARKET,
            1 => Universe::TOTAL_MARKET.union(Universe::FNO),
            _ => Universe::NONE,
        };
        by_key.insert(
            key,
            Entry {
                vendors: VendorSet::EMPTY.with(Vendor::Groww).with(Vendor::Dhan),
                isin: None,
                conflict: None,
                universe,
            },
        );
    }
    Read::new(
        Merged {
            by_key,
            conflicts: Vec::new(),
            eligibility: Vec::new(),
        },
        Vec::new(),
        false,
        0,
    )
}

/// C-11 — rendering the dashboard costs the same at 1×, 10× and 100× the
/// instrument count.
fn dashboard_is_flat_in_universe_size() -> bool {
    let one = universe_of(BASE);
    let ten = universe_of(BASE * 10);
    let hundred = universe_of(BASE * 100);

    let base_ps = cost_ps(|| dashboard_html(&one));
    let mut ok = true;

    ok &= ratio(
        "C-11 dashboard, 10x universe",
        base_ps,
        cost_ps(|| dashboard_html(&ten)),
    );
    ok &= ratio(
        "C-11 dashboard, 100x universe",
        base_ps,
        cost_ps(|| dashboard_html(&hundred)),
    );
    ok
}

fn main() {
    println!("gate 8 — crates/api, ceiling {CEILING_PERMILLE} permille");

    let ok = dashboard_is_flat_in_universe_size();

    if ok {
        println!("all ratios within the ceiling");
    } else {
        println!();
        println!("A REQUEST COSTS MORE BECAUSE THE UNIVERSE IS LARGER.");
        println!("`Read` is loaded once and never mutated, so every number a");
        println!("page reports is fixed the moment it is loaded. Compute it");
        println!("there, not per request. CLAUDE.md section 3 rule 4.");
        std::process::exit(1);
    }
}
