//! C-FNO-CLOCK-01 (FNO-CLOCK, D-8225): one minute boundary of the clock
//! calibration, at 1x, 10x and 100x a full session's seconds. A step reads one
//! shifted minute window through the per-day sparse table and tallies its four
//! prices, in the order a shift's sweep reads its minutes. The measuring
//! function is `cli::fno_clock::measure_minute_boundary`, which the library's
//! own tests run at tiny sizes; this file only times it, prints p50, p99, max
//! and n at each size, and fails when a p99 ratio is above the crate's ceiling
//! or a p99 is over the step's budget in instruction floors.
//!
//! The 10x and 100x grids are invented: a session holds 22,500 seconds, so
//! they exist to show the step does not grow with the grid it reads.

use cli::fno_clock::{Spread, measure_minute_boundary};
use pull::calendar::Session;

use super::{budget, ratio};

/// Samples at each size.
const SAMPLES: usize = 2_000;

/// Steps in one sample: a sample is tens of microseconds of work, short
/// enough that a preemption lands in well under one sample in a hundred, so
/// the p99 reads the step rather than the scheduler. Measured on the cloud
/// host on 3 Nov 2026: 200 samples of 1,000 steps (about 200 µs each) read a
/// 10x p99 of 3.6 times its p50 and a max of 2.3 µs a step.
const STEPS: usize = 100;

/// Instruction floors one step's p99 may cost: PARTS section 11's 1 µs, an
/// ESTIMATE until measured, held in floors as this crate's other budget rows
/// are, so it moves with the machine: 1,600 floors is 987 ns at the cloud
/// host's 617 ps floor of 3 Nov 2026, and 832 ns at the 520 ps the Mac
/// measured for C-CLI-05. The p99s measured on the cloud host were 553 to
/// 1,338 floors in the bench profile (four runs at 1x, 10x and 100x) and 140 to
/// 191 in the instrumented opt-level-0 build, so the worst is 1.20 times
/// under it: a tighter headroom than this crate's other budgets, because the
/// figure is the design's target, not a multiple of a measurement
/// (`docs/06-limits.md` L-FNO-CLOCK-07).
const ALLOWED: u128 = 1_600;

/// The FNO-CLOCK row: prints each of `ids` first on its line, then measures.
pub(crate) fn row(floor: u128, ids: &[&str]) -> bool {
    for id in ids {
        println!(
            "{id} clock calibration, one minute boundary at 1x, 10x and 100x a session's seconds (FNO-CLOCK, D-8225)"
        );
    }
    the_clock_minute_boundary_is_flat(floor)
}

/// C-FNO-CLOCK-01: the step's p99 at 10x and 100x a session's seconds against
/// its p99 at 1x, inside the crate's ceiling, and every p99 inside the budget.
pub(crate) fn the_clock_minute_boundary_is_flat(floor: u128) -> bool {
    let session = usize::from(Session::full().bars()) * 60;
    let mut spreads = Vec::with_capacity(3);
    for (scale, seconds) in [
        ("1x", session),
        ("10x", session * 10),
        ("100x", session * 100),
    ] {
        let spread = Spread::of(&measure_minute_boundary(seconds, SAMPLES, STEPS));
        println!(
            "  C-FNO-CLOCK-01 {scale:>4} ({seconds} seconds): p50 {} ps, p99 {} ps, max {} ps, n {} (method: {SAMPLES} samples of {STEPS} consecutive minute windows, floor {floor} ps)",
            spread.p50, spread.p99, spread.max, spread.n
        );
        spreads.push((scale, spread));
    }
    let p99 = |at: usize| spreads.get(at).map_or(0, |(_, spread)| spread.p99);
    let mut ok = ratio("C-FNO-CLOCK-01  step p99, 1x -> 10x", p99(0), p99(1))
        & ratio("C-FNO-CLOCK-01  step p99, 1x -> 100x", p99(0), p99(2));
    for (scale, spread) in &spreads {
        ok &= budget(
            &format!("C-FNO-CLOCK-01  step p99 at {scale} against the floor"),
            floor,
            spread.p99,
            ALLOWED,
        );
    }
    ok
}
