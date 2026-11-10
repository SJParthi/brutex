//! The o1p99 attack on `crates/costs` (cloud lane `cloud/o1p99-attack-20261110`).
//!
//! `LotTable::lot_on` says its cost is "one hash probe and one binary search
//! of at most `MAX_LOT_ROWS_PER_UNDERLYING` rows: bounded, whatever the table
//! holds", and `costs::lot::table_tests::a_lookup_is_flat_in_the_table`
//! measures it in TABLE size. Nothing bounds the KEY: the probe hashes the
//! caller's whole string and a miss copies it into the refusal, while every
//! row the table can hold is capped at `MAX_LOT_TEXT_BYTES` (2,048). This file
//! reads the lookup per call at keys of 1, 10^2, 10^4 and 10^6 bytes and
//! prints p50 and p99. See `o1p99/harness.rs` and `O1-ATTACK-REPORT.md`.
//!
//! * `O1P99-COSTS-01` `LotTable::lot_on` by key bytes (a miss that starts
//!   like a real underlying). PRINTED, NOT GATED: it grows linearly in the
//!   key (finding O1P99-F-COSTS-01, `tests/o1p99_findings.rs`).
//! * `O1P99-COSTS-02` `LotTable::lot_on` for the real underlyings (hits),
//!   every compiled underlying walked in turn: GATED against the first
//!   underlying alone, so a lookup that grew with the underlying's position
//!   in the table would show.

use std::hint::black_box;

#[allow(
    dead_code,
    reason = "the harness is copied into every crate's o1p99 bench and not every bench calls every helper"
)]
#[path = "o1p99/harness.rs"]
mod harness;

use costs::day::TradeDay;
use costs::lot::{LotTable, compiled_rows};
use harness::{FACTOR_PERMILLE, measure, verdict};

/// A setup failure this bench cannot measure past.
fn refuse(why: &str) -> ! {
    println!("BENCH SETUP FAILED — {why}");
    std::process::exit(1)
}

/// `O1P99-COSTS-01`: printed only.
fn lookup_by_key_bytes(clock: u128, table: &LotTable, day: TradeDay) {
    const SIZES: [u64; 4] = [1, 100, 10_000, 1_000_000];
    let mut keys: Vec<String> = SIZES
        .iter()
        .map(|&n| {
            let n = usize::try_from(n).unwrap_or(usize::MAX);
            let mut s: String = "NIFTYX".chars().take(n).collect();
            while s.len() < n {
                s.push('X');
            }
            s
        })
        .collect();
    let tails = measure(
        &mut keys,
        5,
        400,
        1,
        clock,
        |_| {},
        |k, _| u64::from(table.lot_on(black_box(k.as_str()), day).is_ok()),
    );
    let _printed = verdict(
        "O1P99-COSTS-01",
        "LotTable::lot_on miss by key bytes (printed; see O1P99-F-COSTS-01)",
        &SIZES,
        &tails,
        u128::MAX,
    );
}

/// `O1P99-COSTS-02`.
fn lookup_hits_are_flat(clock: u128, table: &LotTable, day: TradeDay) -> bool {
    let mut names: Vec<String> = compiled_rows()
        .iter()
        .map(|r| r.underlying.clone())
        .collect();
    names.dedup();
    let Some(first) = names.first().cloned() else {
        refuse("the compiled lot table holds no row")
    };
    let count = u64::try_from(names.len()).unwrap_or(0);
    println!("O1P99|O1P99-COSTS-02|the compiled table names {count} underlyings");
    let mut states: Vec<Vec<String>> = vec![vec![first], names];
    let sizes = [1, count];
    let tails = measure(
        &mut states,
        5,
        2_000,
        16,
        clock,
        |_| {},
        |ns, i| {
            let at = usize::try_from(i % u64::try_from(ns.len()).unwrap_or(1)).unwrap_or(0);
            let name = ns.get(at).map_or("", String::as_str);
            u64::from(table.lot_on(black_box(name), day).is_ok())
        },
    );
    verdict(
        "O1P99-COSTS-02",
        "LotTable::lot_on hit, one underlying -> every underlying",
        &sizes,
        &tails,
        FACTOR_PERMILLE,
    )
}

fn main() {
    let clock = harness::clock_ps();
    println!(
        "o1p99 attack — crates/costs, clock {} ns a pair",
        harness::ns(clock)
    );
    let table = match LotTable::from_rows(compiled_rows()) {
        Ok(t) => t,
        Err(e) => refuse(&format!(
            "the compiled lot rows do not build a table: {e:?}"
        )),
    };
    let Ok(day) = TradeDay::new(2024, 12, 10) else {
        refuse("2024-12-10 is a trade day")
    };
    lookup_by_key_bytes(clock, &table, day);
    let ok = lookup_hits_are_flat(clock, &table, day);
    if ok {
        println!("every gated o1p99 row held its ceiling");
    } else {
        println!("AN o1p99 ROW GREW PAST ITS CEILING — see the VERDICT lines marked GROWS.");
        std::process::exit(1);
    }
}
