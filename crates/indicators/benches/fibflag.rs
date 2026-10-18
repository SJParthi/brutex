//! The FIB bench target of `crates/indicators` (FIB-1; D-6318), through the shared FIB
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

#[path = "fib_rows/detect.rs"]
mod detect_rows;
#[path = "fib_rows/hist.rs"]
mod hist_rows;
#[path = "fib_rows/trigger.rs"]
mod trigger_rows;

fn main() {
    let mut out = String::new();
    out.push_str("crates/indicators — FIB bench (FIB-1 harness)\n");
    let host = harness::host(&mut out);
    out.push('\n');
    let mut ok = true;
    ok &= hist_rows::report(&host, &mut out);
    ok &= detect_rows::report(&host, &mut out);
    ok &= trigger_rows::report(&host, &mut out);
    print!("{out}");
    if !ok {
        println!("\na FIB row FAILED or was refused; see above.");
        std::process::exit(1);
    }
    println!("\nok — every FIB row of crates/indicators measured, judged and reported.");
}
