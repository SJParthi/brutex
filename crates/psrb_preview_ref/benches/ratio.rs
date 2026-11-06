//! The O(1) ratio bench of `psrb_preview_ref` (TEST-PLAN section 3, the project's form):
//! the per-candle step (the seven reading states, the gate and every rule set's
//! conjunction) and the per-second step (the resting stop's scan), each at 1x, 10x and
//! 100x of its history, p99 judged against 3.000x.
//!
//! `cargo bench` passes `--bench`: full measurement, and a breach exits 1. Without it
//! (`cargo test`) one short SMOKE pass prints every line and never gates.

use psrb_preview_ref::benchkit::{Dist, per_candle, per_second, row};

fn main() {
    let full = std::env::args().any(|a| a == "--bench");
    let (base, reps) = if full { (2_000, 4_000) } else { (20, 50) };
    let sizes = [base, base * 10, base * 100];
    let candle: [Dist; 3] = sizes.map(|s| per_candle(s, reps));
    let second: [Dist; 3] = sizes.map(|s| per_second(s, reps));
    let mut all_ok = true;
    for (label, dists) in [("B-candle per candle (bank + gate + 3 rule sets)", &candle), ("B-second per second (stop scan)", &second)] {
        let (line, ok) = row(label, dists);
        println!("{line}");
        all_ok &= ok;
    }
    println!("mode {} sizes {sizes:?} reps {reps}", if full { "full" } else { "smoke (never gating)" });
    if full && !all_ok {
        std::process::exit(1);
    }
}
