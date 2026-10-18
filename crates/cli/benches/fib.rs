//! The FIB bench target of `crates/cli` (FIB-1; D-6318), through the shared FIB
//! harness (`crates/runner/benches/fib_rows/harness.rs`). Every row module
//! under `fib_rows/` is its owning package's; until a package lands its row
//! says it is not built yet.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "a bench that cannot measure has nothing to report"
)]

#[path = "../../runner/benches/fib_rows/harness.rs"]
mod harness;

#[path = "fib_rows/day.rs"]
mod day_rows;
#[path = "fib_rows/run.rs"]
mod run_rows;
#[path = "fib_rows/wire.rs"]
mod wire_rows;

fn main() {
    let mut out = String::new();
    out.push_str("crates/cli — FIB bench (FIB-1 harness)\n");
    let host = harness::host(&mut out);
    out.push('\n');
    let mut ok = true;
    ok &= day_rows::report(&host, &mut out);
    ok &= run_rows::report(&host, &mut out);
    ok &= wire_rows::report(&host, &mut out);
    print!("{out}");
    if !ok {
        println!("\na FIB row FAILED or was refused; see above.");
        std::process::exit(1);
    }
    println!("\nok — every FIB row of crates/cli measured, judged and reported.");
}
