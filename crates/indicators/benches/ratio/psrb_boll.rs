//! `C-PSRB-BOLL-01` and `C-PSRB-BOLL-02` (PSRB-BOLL, D-8595): one Bollinger
//! step costs the same after any history and at any period.
//!
//! A step is what a strategy does with each candle: one `Bollinger::fold`, one
//! `window()`, and the population band tests `above_upper` and `below_lower`
//! at z 2.000. `C-PSRB-BOLL-01` times it after 2,000, 20,000 and 200,000
//! candles at n 20; `C-PSRB-BOLL-02` at n 20, 200 and 2,000 after 20,000
//! candles, so the ring's length is the only thing that changes. Each sample is
//! one step timed alone and holds two clock reads, so it is an upper bound on
//! the step.
//!
//! The measuring functions are the library's
//! `indicators::bollinger::measure_step` and `measure_period`, which the
//! module's own tests run at tiny sizes. This file sorts each round (the crate
//! holds no sort, gate 11), reads its ranks, prints and judges: for each size
//! the best round's p50, p99 and max with n, and the p99 of the median and of
//! the worst round; then the best rounds' p99 within the harness's ceiling of
//! the first size's and inside the budget, the minimum over trials as the
//! harness's `cost_ps` takes it. Every candle is invented.

use indicators::bollinger::{
    Refusal, StepCost, flat_within, measure_period, measure_step, summarize,
};

use super::CEILING_PERMILLE;

/// The p99 budget of one step, in nanoseconds: 2 µs (PARTS 4.6; its target of
/// 400 ns is an ESTIMATE until a machine measures it).
const BUDGET_NS: u64 = 2_000;

/// The harness's ceiling as the judge takes it, tied to `CEILING_PERMILLE`.
const CEILING: u64 = 3_000;
const _: () = assert!(CEILING_PERMILLE == 3_000 && CEILING == 3_000);

/// Rounds per size, odd so that the median round is a round.
const ROUNDS: u64 = 11;

/// Steps timed alone per round.
const SAMPLES: u64 = 20_000;

/// `C-PSRB-BOLL-01`'s histories, at n 20.
const HISTORIES: [u64; 3] = [2_000, 20_000, 200_000];

/// `C-PSRB-BOLL-02`'s periods, after 20,000 candles.
const PERIODS: [u16; 3] = [20, 200, 2_000];

/// One size's label and its measurement: `rounds` vectors of unsorted
/// nanosecond timings, or the refusal that stopped it.
type Size = (String, Result<Vec<Vec<u64>>, Refusal>);

/// The slot's row. The first id names the history row and the second the
/// period row, each the first word of every line its row prints. Both rows
/// always run.
pub(crate) fn row(floor: u128, ids: &[&str]) -> bool {
    let mut ok = ids.len() == 2;
    ok &= ids.first().is_some_and(|id| after_any_history(floor, id));
    ok &= ids.get(1).is_some_and(|id| at_any_period(floor, id));
    ok
}

/// `C-PSRB-BOLL-01`: the step after 2,000, 20,000 and 200,000 candles, n 20.
fn after_any_history(floor: u128, id: &str) -> bool {
    let sizes = HISTORIES.map(|history| {
        (
            format!("after {history} candles, n 20"),
            measure_step(history, SAMPLES, ROUNDS),
        )
    });
    judged(id, "history", floor, sizes)
}

/// `C-PSRB-BOLL-02`: the step at n 20, 200 and 2,000, after 20,000 candles.
fn at_any_period(floor: u128, id: &str) -> bool {
    let sizes = PERIODS.map(|period| {
        (
            format!("n {period}, after 20000 candles"),
            measure_period(period, SAMPLES, ROUNDS),
        )
    });
    judged(id, "period", floor, sizes)
}

/// Prints every size's line, then judges the best rounds.
fn judged(id: &str, axis: &str, floor: u128, sizes: [Size; 3]) -> bool {
    let mut best = Vec::with_capacity(sizes.len());
    for (size, measured) in sizes {
        match measured {
            Ok(rounds) => best.push(read(id, &size, floor, rounds)),
            Err(why) => {
                println!("{id} {size}: the step was refused ({why:?})   FAILED");
                return false;
            }
        }
    }
    let ok = flat_within(&best, CEILING, BUDGET_NS);
    let verdict = ["BREACH", "ok"]
        .get(usize::from(ok))
        .copied()
        .unwrap_or_default();
    println!(
        "{id} the best rounds' p99 across the {axis} sizes: within {}.{:03}x of the first and at most {BUDGET_NS} ns   {verdict}",
        CEILING / 1_000,
        CEILING % 1_000
    );
    ok
}

/// Sorts each round, prints the size's line and returns its best round's cost.
fn read(id: &str, size: &str, floor: u128, rounds: Vec<Vec<u64>>) -> StepCost {
    let mut costs: Vec<StepCost> = rounds
        .into_iter()
        .map(|mut round| {
            round.sort_unstable();
            summarize(&round)
        })
        .collect();
    costs.sort_unstable_by_key(|cost| cost.p99_ns);
    let best = costs.first().copied().unwrap_or_default();
    let median = costs.get(costs.len() / 2).map_or(0, |cost| cost.p99_ns);
    let worst = costs.last().map_or(0, |cost| cost.p99_ns);
    let floors = u128::from(best.p99_ns) * 1_000_000 / floor.max(1);
    println!(
        "{id} {size}: best round p50 {} ns, p99 {} ns, max {} ns (n {}); p99 of the median round {median} ns, of the worst {worst} ns; the best p99 is {}.{:03} floors",
        best.p50_ns,
        best.p99_ns,
        best.max_ns,
        best.samples,
        floors / 1_000,
        floors % 1_000
    );
    best
}
