//! `C-PSRB-CCI-01` and `C-PSRB-CCI-02` (PSRB-CCI, D-8655): one CCI step costs the same however long the
//! history, and one fold the same whatever the period; and, unjudged, what a reading costs across periods.
//!
//! `C-PSRB-CCI-01`'s step is one `Cci::fold`, `Cci::reading` and both comparisons of the reading, above +200
//! and below -200: `indicators::cci::measure_step` times it at period 20 after 2,000, 20,000 and 200,000
//! candles. `C-PSRB-CCI-02` times `Cci::fold` ALONE (`indicators::cci::measure_period`) at periods 20, 200
//! and 2,000 after 20,000 candles, so the ring's length is the only thing that changes. A reading is O(period)
//! by design (D-8653, `docs/06-limits.md` L-PSRB-CCI-02), so no judged row times it across periods; one line
//! without an id puts that slope on record instead, the step with the reading at the same three periods
//! (`indicators::cci::measure_reading_period`), and judges nothing.
//!
//! `ROUNDS` rounds of `SAMPLES` steps per size, one clock read either side of each step, so every figure is an
//! upper bound on the step. `indicators::cci_tests::the_bench_measures_every_row_at_tiny_sizes` runs all
//! three at tiny sizes; this file only sorts (gate 11 keeps every sort out of the crate), prints and judges.
//! Printed per size: the best round's p50, p99 and max with n, and the median and worst rounds' p99. Judged,
//! by the crate's minimum-over-trials rule (`cost_ps` in `crates/indicators/benches/ratio.rs`): each size's
//! best-round p99 within the harness's `CEILING_PERMILLE` of the first size's, and inside `BUDGET_NS`,
//! PARTS-WILLR-CCI.md 6.6's 2 µs (its 400 ns target is an ESTIMATE). Every candle is invented and nothing is
//! written.

use indicators::cci::{
    Refusal, StepCost, flat_within, measure_period, measure_reading_period, measure_step, summarize,
};

use super::CEILING_PERMILLE;

/// Timed steps per round.
const SAMPLES: u64 = 20_000;

/// Rounds per size. The best is judged; the median and the worst are printed beside it.
const ROUNDS: u64 = 9;

/// The p99 budget of one step, in nanoseconds: PARTS-WILLR-CCI.md 6.6's 2 µs.
const BUDGET_NS: u64 = 2_000;

/// `C-PSRB-CCI-01`'s histories, at period 20.
const HISTORIES: [u64; 3] = [2_000, 20_000, 200_000];

/// `C-PSRB-CCI-02`'s periods, after 20,000 candles; the unjudged reading line's too.
const PERIODS: [u16; 3] = [20, 200, 2_000];

/// The slot's row: the first id names the history row and the second the period row, and each is the first
/// word of its lines. Any other count of ids fails the row. The reading line follows, without an id.
pub(crate) fn row(floor: u128, ids: &[&str]) -> bool {
    println!(
        "  PSRB-CCI: {} id(s) for its 2 rows; the machine's floor is {floor} ps; budget {BUDGET_NS} ns a step",
        ids.len()
    );
    let judges: [fn(&str) -> bool; 2] = [
        one_step_however_long_the_history,
        one_fold_whatever_the_period,
    ];
    let mut ok = ids.len() == judges.len();
    for (id, judge) in ids.iter().zip(judges) {
        ok &= judge(id);
    }
    the_reading_across_periods();
    ok
}

/// `C-PSRB-CCI-01`: the step at period 20 after each history.
fn one_step_however_long_the_history(id: &str) -> bool {
    let costs = HISTORIES.map(|history| {
        let label = format!("history {history}, period 20");
        best_round(id, &label, measure_step(history, SAMPLES, ROUNDS))
    });
    verdict(id, &costs)
}

/// `C-PSRB-CCI-02`: the fold alone after 20,000 candles at each period.
fn one_fold_whatever_the_period(id: &str) -> bool {
    let costs = PERIODS.map(|period| {
        let label = format!("fold alone, period {period}, history 20000");
        best_round(id, &label, measure_period(period, SAMPLES, ROUNDS))
    });
    verdict(id, &costs)
}

/// The unjudged line: the step with the reading at each period, its best rounds' p99 printed on one line.
fn the_reading_across_periods() {
    let costs = PERIODS.map(|period| {
        let label = format!("reading at period {period}, history 20000");
        best_round(
            "  (unjudged)",
            &label,
            measure_reading_period(period, SAMPLES, ROUNDS),
        )
    });
    let figures: Vec<String> = costs
        .iter()
        .map(|cost| format!("{} ns", cost.p99_ns))
        .collect();
    println!(
        "  reading at periods 20, 200, 2000: O(period) by design, L-PSRB-CCI-02, not judged flat: best-round p99 {}",
        figures.join(", ")
    );
}

/// Sorts each round, reads it by rank, prints the best, median and worst rounds, and returns the best. Every
/// line names the measurement's refusal, `None` when there was none; a refusal leaves no round, so the best
/// reads p99 0 and the verdict fails.
fn best_round(id: &str, label: &str, measured: Result<Vec<Vec<u64>>, Refusal>) -> StepCost {
    let refusal = measured.as_ref().err().copied();
    let mut costs: Vec<StepCost> = measured
        .unwrap_or_default()
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
        "{id} {label}: best of {} rounds p50 {} ns, p99 {} ns, max {} ns, n {}; median round p99 {} ns; worst round p99 {} ns; refusal {refusal:?}",
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

/// Judges one row's best rounds with `indicators::cci::flat_within` and prints each size's p99 against the
/// first size's, and the verdict.
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
