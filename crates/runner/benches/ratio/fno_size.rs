//! FNO-SIZE's seven bench rows, C-FNO-SIZE-01..07 (state/fno-size-20261103/PARTS-FNO-SIZE.md
//! section 8): a thin wrapper over `runner::fno_size::measure`, whose functions the library's
//! tests run at tiny sizes.
//!
//! Every size prints its p50, p99, max and sample count, each line led by its row's id. A
//! per-unit row is judged by this bench's two-sided ratio ceiling between its 1x and 100x p99
//! and by its budget at every size; a fixed-work row by its p99 over p50 at every size, the
//! same ceiling across its sizes and its budget. Every budget is an ESTIMATE until measured on
//! the Mac, and figures measured in the cloud are advisory.

use runner::fno_size::Policy;
use runner::fno_size::measure::{
    CYCLES_PER_SAMPLE, DUPLICATES_BUDGET_PS, EDGE_INSERT_BUDGET_PS, EDGE_RESET_BUDGET_PS,
    JUDGING_BUDGET_PS, NETWORK_BUDGET_PS, PLAN_BUDGET_PS, RISK_CYCLE_BUDGET_PS,
    RISK_RELEASE_BUDGET_PS, SCORE_BUDGET_PS, SCORES_PER_SAMPLE, Series, measure_duplicates,
    measure_edge_insert, measure_edge_reset, measure_judging, measure_network, measure_plan,
    measure_risk_cycle, measure_risk_release, measure_score, steady, within,
};

use super::ratio;

/// One size's order statistics, picoseconds per unit, nearest rank.
struct Stats {
    p50: u128,
    p99: u128,
    max: u128,
    n: usize,
}

/// The order statistics of `series`. Sorting is a bench's business: the library sorts nothing.
fn stats(series: &Series) -> Stats {
    let mut sorted = series.samples.clone();
    sorted.sort_unstable();
    let n = sorted.len();
    let rank = |percent: usize| {
        let at = (n * percent).div_ceil(100).saturating_sub(1);
        sorted.get(at).copied().unwrap_or(0)
    };
    Stats {
        p50: rank(50),
        p99: rank(99),
        max: sorted.last().copied().unwrap_or(0),
        n,
    }
}

/// The word a verdict prints.
fn word(ok: bool) -> &'static str {
    ["OVER", "ok"]
        .get(usize::from(ok))
        .copied()
        .unwrap_or("OVER")
}

/// Prints one size's line and returns its statistics.
fn show(id: &str, what: &str, size: &str, series: &Series) -> Stats {
    let s = stats(series);
    println!(
        "{id} {what} at {size}: p50 {} ps, p99 {} ps, max {} ps, n {}, refused {}",
        s.p50, s.p99, s.max, s.n, series.refused
    );
    s
}

/// A per-unit row: its p99 at 100x within the ceiling of its p99 at 1x, inside its budget at
/// every size, and nothing refused.
fn per_unit(id: &str, what: &str, sizes: &[(&str, Series); 3], budget_ps: u128) -> bool {
    let all: Vec<Stats> = sizes
        .iter()
        .map(|(size, series)| show(id, what, size, series))
        .collect();
    let first = all.first().map_or(0, |s| s.p99);
    let last = all.last().map_or(0, |s| s.p99);
    let flat = ratio(&format!("{id} {what}, p99 1x -> 100x"), first, last);
    let inside = all.iter().all(|s| within(s.p99, budget_ps));
    let clean = sizes.iter().all(|(_, series)| series.refused == 0);
    println!(
        "{id} {what}: p99 budget {budget_ps} ps (ESTIMATE) {}, refusals {}",
        word(inside),
        word(clean)
    );
    flat & inside & clean
}

/// A fixed-work row: p99 at most 1.2 times p50 at every size, the same cost per unit from 1x
/// to 100x within the ceiling, inside its budget, and nothing refused.
fn fixed_work(id: &str, what: &str, sizes: &[(&str, Series); 3], budget_ps: u128) -> bool {
    let all: Vec<Stats> = sizes
        .iter()
        .map(|(size, series)| show(id, what, size, series))
        .collect();
    let still = all.iter().all(|s| steady(s.p50, s.p99));
    let first = all.first().map_or(0, |s| s.p99);
    let last = all.last().map_or(0, |s| s.p99);
    let flat = ratio(&format!("{id} {what}, p99 1x -> 100x"), first, last);
    let inside = all.iter().all(|s| within(s.p99, budget_ps));
    let clean = sizes.iter().all(|(_, series)| series.refused == 0);
    println!(
        "{id} {what}: p99/p50 at most 1.2 {}, p99 budget {budget_ps} ps (ESTIMATE) {}, refusals {}",
        word(still),
        word(inside),
        word(clean)
    );
    still & flat & inside & clean
}

/// C-FNO-SIZE-01: `plan` per candidate, SIZE-SPLIT-ALL, SIZE-RISK and SIZE-FIXED-20-SEEDED,
/// one batch of 218 after 10, 100 and 1,000 batches through the same plan.
fn plan_row(id: &str) -> bool {
    println!(
        "{id} method: one plan of 218 candidates a sample, 1,000 samples a size, ps per candidate"
    );
    let mut ok = true;
    for policy in [Policy::SplitAll, Policy::Risk, Policy::FixedSeeded] {
        let sizes = [
            ("10 batches", measure_plan(policy, 218, 10, 1_000)),
            ("100 batches", measure_plan(policy, 218, 100, 1_000)),
            ("1,000 batches", measure_plan(policy, 218, 1_000, 1_000)),
        ];
        ok &= per_unit(
            id,
            &format!("plan, {}", policy.name()),
            &sizes,
            PLAN_BUDGET_PS,
        );
    }
    ok
}

/// C-FNO-SIZE-02: one score, on tables of 1k, 10k and 100k learning trades.
fn score_row(id: &str) -> bool {
    println!(
        "{id} method: {SCORES_PER_SAMPLE} scores a sample, 2,000 samples a size, ps per score"
    );
    let sizes = [
        ("1k trades", measure_score(1_000, 2_000)),
        ("10k trades", measure_score(10_000, 2_000)),
        ("100k trades", measure_score(100_000, 2_000)),
    ];
    per_unit(id, "EdgeTable::score", &sizes, SCORE_BUDGET_PS)
}

/// C-FNO-SIZE-03: RANKED's network on 256 rows, 1, 10 and 100 networks a sample.
fn network_row(id: &str) -> bool {
    println!(
        "{id} method: 1, 10 or 100 networks of 256 rows a sample, 100 samples a size, ps per network"
    );
    let sizes = [
        ("1x", measure_network(256, 1, 100)),
        ("10x", measure_network(256, 10, 100)),
        ("100x", measure_network(256, 100, 100)),
    ];
    fixed_work(id, "RANKED network, 256 rows", &sizes, NETWORK_BUDGET_PS)
}

/// C-FNO-SIZE-04: (a) the risk book's cycle with 255 others in force, after 1k, 10k and 100k
/// cycles; (b) the worst release, behind 255 released rows, 1, 10 and 100 a sample.
fn risk_row(id: &str) -> bool {
    println!(
        "{id} method (a): {CYCLES_PER_SAMPLE} cycles a sample, 2,000 samples a size, ps per cycle"
    );
    let cycle = [
        ("1k cycles", measure_risk_cycle(1_000, 2_000)),
        ("10k cycles", measure_risk_cycle(10_000, 2_000)),
        ("100k cycles", measure_risk_cycle(100_000, 2_000)),
    ];
    let a = per_unit(id, "(a) RiskBook cycle", &cycle, RISK_CYCLE_BUDGET_PS);
    println!(
        "{id} method (b): 1, 10 or 100 worst releases a sample on one book, each with the settle and admission that put it back, 200 samples a size, ps per repetition"
    );
    let release = [
        ("1x", measure_risk_release(1, 200)),
        ("10x", measure_risk_release(10, 200)),
        ("100x", measure_risk_release(100, 200)),
    ];
    let b = fixed_work(
        id,
        "(b) RiskBook worst release",
        &release,
        RISK_RELEASE_BUDGET_PS,
    );
    a & b
}

/// C-FNO-SIZE-05: (a) one insertion into a table already reset, 1k, 10k and 100k trades;
/// (b) the reset alone on a 375-rung ladder, 1, 10 and 100 a sample.
fn edge_row(id: &str) -> bool {
    println!(
        "{id} method (a): every trade inserted once a sample, 200 samples a size, ps per trade"
    );
    let insert = [
        ("1k trades", measure_edge_insert(1_000, 200)),
        ("10k trades", measure_edge_insert(10_000, 200)),
        ("100k trades", measure_edge_insert(100_000, 200)),
    ];
    let a = per_unit(
        id,
        "(a) EdgeTable insertion",
        &insert,
        EDGE_INSERT_BUDGET_PS,
    );
    println!(
        "{id} method (b): 1, 10 or 100 resets of 256 x 2 x 376 cells a sample, 40 samples a size, ps per reset"
    );
    let reset = [
        ("1x", measure_edge_reset(375, 1, 40)),
        ("10x", measure_edge_reset(375, 10, 40)),
        ("100x", measure_edge_reset(375, 100, 40)),
    ];
    let b = fixed_work(
        id,
        "(b) EdgeTable reset, 375 rungs",
        &reset,
        EDGE_RESET_BUDGET_PS,
    );
    a & b
}

/// C-FNO-SIZE-06: one month boundary, per trade moved, months of 100, 1k and 10k trades.
fn judging_row(id: &str) -> bool {
    println!("{id} method: one month boundary a sample, 300 samples a size, ps per trade moved");
    let sizes = [
        ("100 a month", measure_judging(100, 300)),
        ("1k a month", measure_judging(1_000, 300)),
        ("10k a month", measure_judging(10_000, 300)),
    ];
    per_unit(id, "EdgeTable::judging", &sizes, JUDGING_BUDGET_PS)
}

/// C-FNO-SIZE-07: `plan` on 256 candidates on one stock, 1, 10 and 100 a sample.
fn duplicates_row(id: &str) -> bool {
    println!(
        "{id} method: 1, 10 or 100 plans of 256 candidates on one stock a sample, 50 samples a size, ps per plan"
    );
    let sizes = [
        ("1x", measure_duplicates(256, 1, 50)),
        ("10x", measure_duplicates(256, 10, 50)),
        ("100x", measure_duplicates(256, 100, 50)),
    ];
    fixed_work(
        id,
        "plan, the duplicate check's worst batch",
        &sizes,
        DUPLICATES_BUDGET_PS,
    )
}

/// The seven rows, each id printed as the first word of its lines.
pub(crate) fn row(ids: &[&str]) -> bool {
    let [plan, score, network, risk, edge, judging, duplicates] = ids else {
        println!("  FNO-SIZE: seven bench ids expected, {} given", ids.len());
        return false;
    };
    let rows = [
        plan_row(plan),
        score_row(score),
        network_row(network),
        risk_row(risk),
        edge_row(edge),
        judging_row(judging),
        duplicates_row(duplicates),
    ];
    rows.iter().all(|ok| *ok)
}
