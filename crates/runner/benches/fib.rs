//! The FIB bench target of `crates/runner` (FIB-1; D-6318): the harness's own
//! self-tests (FM-F001, FM-F002), then every FIB row this crate owns, each with
//! its class, axis, three scales spanning at least 100x, p50, p99 and max per
//! call, its structural work counter and its ratio row.
//!
//! The harness is `fib_rows/harness.rs`; its tests also run under
//! `cargo test` through `tests/fib_harness.rs`. Rows not built yet say so.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "a bench that cannot measure has nothing to report"
)]

#[path = "fib_rows/harness.rs"]
mod harness;

#[path = "fib_rows/charges.rs"]
mod charges_rows;
#[path = "fib_rows/gate.rs"]
mod gate_rows;
#[path = "fib_rows/identity.rs"]
mod identity_rows;
#[path = "fib_rows/lattice.rs"]
mod lattice_rows;
#[path = "fib_rows/rank.rs"]
mod rank_rows;
#[path = "fib_rows/walk.rs"]
mod walk_rows;

fn main() {
    let mut out = String::new();
    out.push_str("crates/runner — FIB bench (FIB-1 harness)\n");
    if let Err(why) = harness::self_tests() {
        println!("{out}harness self-test FAILED: {why}");
        std::process::exit(1);
    }
    out.push_str("harness self-tests: spike detected as WORST-CASE, passed as AMORTISED; row without class refused; axis and span refused\n");
    let host = harness::host(&mut out);
    out.push('\n');
    let mut ok = true;
    ok &= identity_rows::report(&host, &mut out);
    ok &= rank_rows::report(&host, &mut out);
    ok &= walk_rows::report(&host, &mut out);
    ok &= lattice_rows::report(&host, &mut out);
    ok &= gate_rows::report(&host, &mut out);
    ok &= charges_rows::report(&host, &mut out);
    print!("{out}");
    if !ok {
        println!("\na FIB row FAILED or was refused; see above.");
        std::process::exit(1);
    }
    println!("\nok — every FIB row of crates/runner measured, judged and reported.");
}
