//! C-FNO-PORT-01 (FNO-PORTFOLIO, D-8180): admit and release with the other
//! nineteen slots held, timed at 1x, 10x and 100x the sessions behind them.
//! `runner::fno_portfolio::measure_admit_release` drives the account and times
//! each call; this file prints p50, p99 and max at each size and fails the row,
//! through its harness `crates/runner/benches/ratio.rs`, which exits non-zero,
//! when one cycle's p99 (one release and the admission into its slot) grows
//! past the harness's ceiling at 10x or 100x, or when the cycle's p99 or
//! either call's passes the budget.
//!
//! The ratio is judged on the cycle, not on each call alone: a release is a few
//! tens of nanoseconds, near the timer's own cost, so its p99 alone is mostly
//! the machine's noise, and a call whose work grew with the history would grow
//! the cycle it is in. Each size runs [`TRIALS`] times, the sizes interleaved,
//! and the trial with the smallest p99 is the one judged: an interrupt, a
//! migration or a clock still ramping can only make a trial slower, the reason
//! the harness's own `per_unit_min` keeps a minimum (D-0690). Every input is
//! invented by the fixture, and nothing is read or written.

use runner::fno_portfolio::{Samples, measure_admit_release};

use super::ratio;

/// Sessions behind the timed ones at 1x; 10x and 100x take ten and a hundred
/// times as many.
const BASE_SESSIONS: u32 = 10;

/// Sessions timed at every size: 40 sessions of 80 cycles, 3,200 samples of
/// each call.
const TIMED_SESSIONS: u32 = 40;

/// Entries one session of the fixture admits: twenty to fill the slots, then
/// one a cycle.
const ENTRIES_PER_SESSION: u64 = 100;

/// Trials of every size; the one with the smallest p99 is judged.
const TRIALS: usize = 5;

/// The p99 budget of one admission, one release and one cycle, in
/// nanoseconds: PARTS section 11's target, an ESTIMATE until the Mac
/// measures it.
const BUDGET_NS: u64 = 300;

/// p50, p99 and max of one size's samples, in nanoseconds.
type Quantiles = (u64, u64, u64);

/// The three series one run gives: the cycles, the admissions, the releases.
const SERIES: [&str; 3] = ["cycle", "admit", "release"];

/// C-FNO-PORT-01's row. Each id is printed as the first word of a line.
pub(crate) fn row(ids: &[&str]) -> bool {
    for id in ids {
        println!(
            "{id} admit and release with the other 19 slots held, at 1x, 10x and 100x the \
             sessions behind, the fastest p99 of {TRIALS} trials (FNO-PORTFOLIO, D-8180)"
        );
    }
    let sizes = [BASE_SESSIONS, BASE_SESSIONS * 10, BASE_SESSIONS * 100];
    // best[size][series], the series in SERIES order.
    let mut best = [[(0, u64::MAX, 0); 3]; 3];
    for _ in 0..TRIALS {
        for (sessions, kept) in sizes.iter().zip(&mut best) {
            let expected = u64::from(sessions + TIMED_SESSIONS) * ENTRIES_PER_SESSION;
            let samples = match measure_admit_release(*sessions, TIMED_SESSIONS) {
                Ok(samples) if samples.admitted == expected => samples,
                other => {
                    println!(
                        "  C-FNO-PORT-01 REFUSED: the fixture did not run as built behind \
                         {sessions} sessions: {other:?}"
                    );
                    return false;
                }
            };
            for (kept, trial) in kept.iter_mut().zip(series(&samples)) {
                keep_fastest(kept, quantiles(trial));
            }
        }
    }
    let [
        [cycle_1, admit_1, release_1],
        [cycle_10, admit_10, release_10],
        [cycle_100, admit_100, release_100],
    ] = best;
    let cycle = [cycle_1, cycle_10, cycle_100];
    let admit = [admit_1, admit_10, admit_100];
    let release = [release_1, release_10, release_100];
    let flat = ratios(cycle);
    let mut within = true;
    for (name, sizes) in SERIES.iter().zip([cycle, admit, release]) {
        within &= judge(name, sizes);
    }
    flat & within
}

/// One run's three series: each cycle's release plus admission, the
/// admissions, the releases.
fn series(run: &Samples) -> [Vec<u64>; 3] {
    let cycles = run
        .admit_ns
        .iter()
        .zip(&run.release_ns)
        .map(|(admit, release)| admit.saturating_add(*release))
        .collect();
    [cycles, run.admit_ns.clone(), run.release_ns.clone()]
}

/// Keeps whichever of two trials has the smaller p99.
fn keep_fastest(best: &mut Quantiles, trial: Quantiles) {
    if trial.1 < best.1 {
        *best = trial;
    }
}

/// The cycle's p99 at 10x and at 100x, each held to the ceiling against 1x.
fn ratios(cycle: [Quantiles; 3]) -> bool {
    let [(_, base, _), (_, ten, _), (_, hundred, _)] = cycle;
    let ten = ratio(
        "C-FNO-PORT-01 cycle p99, 1x -> 10x",
        picoseconds(base),
        picoseconds(ten),
    );
    let hundred = ratio(
        "C-FNO-PORT-01 cycle p99, 1x -> 100x",
        picoseconds(base),
        picoseconds(hundred),
    );
    ten & hundred
}

/// One series' three sizes printed, and its largest p99 held to the budget.
fn judge(name: &str, sizes: [Quantiles; 3]) -> bool {
    for (label, (p50, p99, max)) in ["1x", "10x", "100x"].iter().zip(sizes) {
        println!(
            "  C-FNO-PORT-01 {name:<7} at {label:>4}: p50 {p50} ns, p99 {p99} ns, max {max} ns"
        );
    }
    let worst = sizes
        .iter()
        .map(|&(_, p99, _)| p99)
        .max()
        .unwrap_or(u64::MAX);
    let within = worst <= BUDGET_NS;
    println!(
        "  C-FNO-PORT-01 {name:<7} p99 at most {worst} ns, budget {BUDGET_NS} ns   {}",
        if within { "ok" } else { "OVER BUDGET" }
    );
    within
}

/// p50, p99 and max of one series, in nanoseconds.
fn quantiles(mut series: Vec<u64>) -> Quantiles {
    series.sort_unstable();
    let at = |permille: usize| {
        series
            .get(series.len() * permille / 1_000)
            .copied()
            .unwrap_or(0)
    };
    (at(500), at(990), series.last().copied().unwrap_or(0))
}

/// Nanoseconds as the picoseconds the harness's ratio reads.
fn picoseconds(ns: u64) -> u128 {
    u128::from(ns) * 1_000
}
