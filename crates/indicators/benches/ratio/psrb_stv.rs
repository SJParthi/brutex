//! `C-PSRB-STV-01` and `C-PSRB-STV-02`: one step of the two-band Supertrend
//! (`indicators::dual_band_supertrend`, PSRB-STV) costs the same however many
//! candles came before it and whatever its ATR period.
//!
//! The measuring functions are the module's own; this file sorts each round
//! (gate 11 keeps a sort out of the crate's production code), prints and judges.
//! For each size it prints the best round's p50, p99 and max with n, and the
//! median and worst rounds' p99. It judges the best round, the crate's
//! minimum-over-trials rule: every size's best p99 within `CEILING_PERMILLE` of
//! `ratio.rs` (3.0x) of the first size's, and inside a 2 µs budget. The helpers
//! are this part's own copy (PARTS 2.4): a shared one would make one PSRB part
//! wait on another.

use indicators::dual_band_supertrend::{
    Refusal, StepCost, flat_within, measure_period, measure_step, summarize,
};

/// Timed steps per round.
const SAMPLES: u64 = 10_000;

/// Rounds per size. Odd, so the median round is one of them.
const ROUNDS: u64 = 7;

/// The budget for one step's p99, in nanoseconds: 2 µs.
const BUDGET_NS: u64 = 2_000;

/// `C-PSRB-STV-01`'s sizes: candles folded before the timed steps.
const HISTORIES: [u64; 3] = [2_000, 20_000, 200_000];

/// `C-PSRB-STV-02`'s sizes: ATR periods, each after 20,000 candles.
const PERIODS: [u16; 3] = [10, 100, 1_000];

/// One size's timings as the module returns them: rounds of unsorted steps.
type Measured = Result<Vec<Vec<u64>>, Refusal>;

/// Measures and judges both rows, printing each id it is given as the first word
/// of every line about it; true when both hold.
pub(crate) fn row(floor: u128, ids: &[&str]) -> bool {
    let [step_id, period_id] = ids else {
        println!("  C-PSRB-STV REFUSED: the row judges two ids and was given {ids:?}");
        return false;
    };
    let ceiling = u64::try_from(super::CEILING_PERMILLE).unwrap_or(0);
    let verdicts = [
        judge(
            step_id,
            "fold + reading(), period 10, factor 3,000, candles folded",
            (floor, ceiling),
            HISTORIES.map(|history| (history, measure_step(history, SAMPLES, ROUNDS))),
        ),
        judge(
            period_id,
            "fold + reading() after 20,000 candles, factor 3,000, ATR period",
            (floor, ceiling),
            PERIODS.map(|period| (u64::from(period), measure_period(period, SAMPLES, ROUNDS))),
        ),
    ];
    !verdicts.contains(&false)
}

/// Prints one row's sizes and its verdict. A refused measurement prints its
/// refusal and fails the row.
fn judge(id: &str, axis: &str, (floor, ceiling): (u128, u64), sizes: [(u64, Measured); 3]) -> bool {
    let mut kept = Vec::with_capacity(sizes.len());
    for (size, measured) in sizes {
        match measured {
            Ok(mut rounds) => kept.push(report(id, axis, size, &mut rounds)),
            Err(why) => println!("{id} {axis} {size}: REFUSED {why:?}"),
        }
    }
    let ok = kept.len() == HISTORIES.len() && flat_within(&kept, ceiling, BUDGET_NS);
    let p99: Vec<String> = kept.iter().map(|cost| cost.p99_ns.to_string()).collect();
    println!(
        "{id} best-round p99 {} ns: within {}.{:03}x of the first size and {BUDGET_NS} ns \
         (machine floor {floor} ps)   {}",
        p99.join(" -> "),
        ceiling / 1_000,
        ceiling % 1_000,
        if ok { "ok" } else { "BREACH" }
    );
    ok
}

/// Sorts every round, prints one size's line, and returns the round with the
/// lowest p99.
fn report(id: &str, axis: &str, size: u64, rounds: &mut [Vec<u64>]) -> StepCost {
    let mut costs: Vec<StepCost> = rounds
        .iter_mut()
        .map(|round| {
            round.sort_unstable();
            summarize(round)
        })
        .collect();
    costs.sort_unstable_by_key(|cost| cost.p99_ns);
    let lowest = costs.first().copied().unwrap_or_default();
    let median = costs.get(costs.len() / 2).copied().unwrap_or_default();
    let worst = costs.last().copied().unwrap_or_default();
    println!(
        "{id} {axis} = {size}: best round p50 {} ns, p99 {} ns, max {} ns (n {}); median \
         round p99 {} ns; worst round p99 {} ns",
        lowest.p50_ns, lowest.p99_ns, lowest.max_ns, lowest.samples, median.p99_ns, worst.p99_ns
    );
    lowest
}
