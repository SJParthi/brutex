//! `C-PSRB-RSI-01` and `C-PSRB-RSI-02` (PSRB-RSI, D-8584): one Wilder RSI step costs
//! the same however long the history and whatever the period.
//!
//! A step is one `Rsi::fold`, `Rsi::reading` and both comparisons of the reading against
//! a level. `indicators::rsi::measure_step` times it at period 14 after 2,000, 20,000 and
//! 200,000 candles (`C-PSRB-RSI-01`), and `indicators::rsi::measure_period` at periods 14,
//! 140 and 1,400 after 20,000 (`C-PSRB-RSI-02`): `ROUNDS` rounds of `SAMPLES` steps per
//! size, one clock read either side of each step, so every figure is an upper bound on
//! the step. `indicators::rsi::tests::the_bench_measures_both_rows_at_tiny_sizes` runs
//! both at tiny sizes; this file only sorts (gate 11 keeps every sort out of the crate),
//! prints and judges.
//!
//! Printed per size: the best round's p50, p99 and max with n, and the median and worst
//! rounds' p99. Judged, by the crate's minimum-over-trials rule (`cost_ps` in
//! `crates/indicators/benches/ratio.rs`): each size's best-round p99 within the harness's
//! `CEILING_PERMILLE` of the first size's, and inside `BUDGET_NS`, PARTS.md 3.6's 2 µs
//! (its 300 ns target is an ESTIMATE). Every candle is invented and nothing is written.

use indicators::rsi::{Refusal, StepCost, flat_within, measure_period, measure_step, summarize};

use super::CEILING_PERMILLE;

/// Timed steps per round.
const SAMPLES: u64 = 20_000;

/// Rounds per size. The best is judged; the median and the worst are printed beside it.
const ROUNDS: u64 = 9;

/// The p99 budget of one step, in nanoseconds: PARTS.md 3.6's 2 µs.
const BUDGET_NS: u64 = 2_000;

/// `C-PSRB-RSI-01`'s histories, at period 14.
const HISTORIES: [u64; 3] = [2_000, 20_000, 200_000];

/// `C-PSRB-RSI-02`'s periods, after 20,000 candles.
const PERIODS: [u16; 3] = [14, 140, 1_400];

/// The slot's row: the first id names the history row and the second the period row, and
/// each is the first word of its lines. Any other count of ids fails the row.
pub(crate) fn row(floor: u128, ids: &[&str]) -> bool {
    println!(
        "  PSRB-RSI: {} id(s) for its 2 rows; the machine's floor is {floor} ps; budget {BUDGET_NS} ns a step",
        ids.len()
    );
    let judges: [fn(&str) -> bool; 2] = [
        one_step_however_long_the_history,
        one_step_whatever_the_period,
    ];
    let mut ok = ids.len() == judges.len();
    for (id, judge) in ids.iter().zip(judges) {
        ok &= judge(id);
    }
    ok
}

/// `C-PSRB-RSI-01`: the step at period 14 after each history.
fn one_step_however_long_the_history(id: &str) -> bool {
    let costs = HISTORIES.map(|history| {
        let label = format!("history {history}, period 14");
        best_round(id, &label, measure_step(history, SAMPLES, ROUNDS))
    });
    verdict(id, &costs)
}

/// `C-PSRB-RSI-02`: the step after 20,000 candles at each period.
fn one_step_whatever_the_period(id: &str) -> bool {
    let costs = PERIODS.map(|period| {
        let label = format!("period {period}, history 20000");
        best_round(id, &label, measure_period(period, SAMPLES, ROUNDS))
    });
    verdict(id, &costs)
}

/// Sorts each round, reads it by rank, prints the best, median and worst rounds, and
/// returns the best. A refusal prints its reason and leaves no sample, which the verdict
/// fails.
fn best_round(id: &str, label: &str, measured: Result<Vec<Vec<u64>>, Refusal>) -> StepCost {
    let rounds = measured.unwrap_or_else(|why| {
        println!("{id} {label}: REFUSED {why:?}");
        Vec::new()
    });
    let mut costs: Vec<StepCost> = rounds
        .into_iter()
        .map(|mut round| {
            round.sort_unstable();
            summarize(&round)
        })
        .collect();
    costs.sort_unstable_by_key(|cost| cost.p99_ns);
    let best = costs.first().copied().unwrap_or_default();
    let median = costs.get(costs.len() / 2).copied().unwrap_or_default();
    let worst = costs.last().copied().unwrap_or_default();
    println!(
        "{id} {label}: best of {} rounds p50 {} ns, p99 {} ns, max {} ns, n {}; median round p99 {} ns; worst round p99 {} ns",
        costs.len(),
        best.p50_ns,
        best.p99_ns,
        best.max_ns,
        best.samples,
        median.p99_ns,
        worst.p99_ns
    );
    best
}

/// Judges one row's best rounds with `indicators::rsi::flat_within` and prints each
/// size's p99 against the first size's, and the verdict.
fn verdict(id: &str, costs: &[StepCost]) -> bool {
    let ceiling = u64::try_from(CEILING_PERMILLE).unwrap_or(0);
    let ok = flat_within(costs, ceiling, BUDGET_NS);
    let first = costs.first().map_or(0, |cost| cost.p99_ns);
    let ratios: Vec<String> = costs
        .iter()
        .map(|cost| {
            let permille = cost
                .p99_ns
                .saturating_mul(1_000)
                .checked_div(first)
                .unwrap_or(0);
            format!("{}.{:03}x", permille / 1_000, permille % 1_000)
        })
        .collect();
    println!(
        "{id}: best-round p99 against the first size {}; ceiling {}.{:03}x, budget {BUDGET_NS} ns   {}",
        ratios.join(" -> "),
        ceiling / 1_000,
        ceiling % 1_000,
        ["BREACH", "ok"]
            .get(usize::from(ok))
            .copied()
            .unwrap_or_default()
    );
    ok
}
