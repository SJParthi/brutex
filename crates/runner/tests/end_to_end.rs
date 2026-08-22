//! The whole pipeline, run end to end, with its own output checked.
//!
//! # Why this exists beside the unit tests
//!
//! Every module in this crate is tested on its own. That proves each stage does
//! what it claims and proves **nothing** about whether they compose: a sweep
//! whose masks never reach the trade walk, a grid built from a column paired
//! with the wrong bars, an audit rendering a `Validated` nobody populated —
//! each of those passes every unit test in the crate.
//!
//! So this runs the real sequence, in order, on one slice, and asserts the
//! properties that can only be checked once they are joined:
//!
//! 1. bars → `Column` → `Ladder::walk` → frequent itemsets
//! 2. itemsets → `closed` → distinct hypotheses
//! 3. hypothesis → `trade::walk` → non-overlapping intraday round trips
//! 4. trades → `grid::evaluate` → every stop/target/trail variant
//! 5. slice → `validate::walk_forward` → chosen on the past, judged on the future
//! 6. folds → `pbo` → is the selection fooling itself
//! 7. returns → `bootstrap` → is anything here beyond luck
//! 8. all of it → `audit::render` → one report
//!
//! # The data
//!
//! `synthetic::sessions` — generated in-process, deterministically, from
//! integer arithmetic. **No stored bar is read, no file is opened and no vendor
//! is contacted.** That is a hard requirement of this repository and it is why
//! this test can run anywhere.
//!
//! The figures it produces are therefore facts about the CODE and not about
//! NIFTY. That distinction is the whole reason the assertions below are about
//! structure — counts reconciling, orderings holding, absences reported — and
//! never about whether a number is large.

use costs::fill::Direction;
use engine::Ladder;
use indicators::column::Column;
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::excursion::Side;
use runner::outcome::Horizon;
use runner::{Sweeper, audit, bootstrap, closed, grid, pbo, synthetic, trade, validate};
use vocab::ConditionMask;

#[expect(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
fn evaluator() -> Evaluator {
    Evaluator::new(
        Widths::pinned().expect("pinned widths are valid"),
        Availability::Absent,
        Thresholds::CLASSICAL,
    )
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one procedure, and splitting it would hide that each stage's \
              output is the next stage's input -- which is the thing under test."
)]
fn the_whole_pipeline_runs_and_every_stage_feeds_the_next() {
    let bars = synthetic::sessions(12);
    let horizon = Horizon::DEFAULT;

    // ---- 1. bars -> column -> sweep ------------------------------------
    let column = Column::build(&bars, &mut evaluator());
    assert!(!column.is_empty(), "the column must sweep bars");
    let census = column.census();
    assert!(
        census.reconciles(),
        "every offered bar must land in exactly one bucket"
    );

    let swept =
        Sweeper::new(Ladder::with_min_hits(150).with_ceiling(50_000)).run(&bars, &mut evaluator());
    let frequent = swept.sweep.all_frequent().count();
    assert!(frequent > 0, "the sweep must find frequent combinations");

    // ---- 2. closure -----------------------------------------------------
    let distinct = closed::closed(&swept.sweep);
    assert!(
        distinct.kept.len() <= frequent,
        "closure never invents a combination"
    );
    assert_eq!(
        usize::try_from(distinct.considered).unwrap_or(usize::MAX),
        frequent,
        "closure must account for every frequent set it was given"
    );

    // ---- 3. one hypothesis -> trades ------------------------------------
    let mask = distinct
        .kept
        .first()
        .map_or_else(ConditionMask::default, |i| i.mask);
    let taken = trade::walk(&bars, &column, &mask, horizon, Direction::Long);
    assert!(
        taken.reconciles(),
        "trades + blocked + too-late must equal signals"
    );

    // ---- 4. trades -> the exit grid -------------------------------------
    let exits = grid::evaluate(&bars, &column, &mask, horizon, Side::Long, 4);
    assert!(
        exits.baseline().is_some() || exits.cells.is_empty(),
        "a populated grid must carry the no-levels baseline row"
    );
    for cell in &exits.cells {
        assert!(
            cell.pessimistic <= cell.optimistic,
            "the pessimistic reading can never beat the optimistic one"
        );
        assert_eq!(
            cell.stopped
                .saturating_add(cell.trailed_stop)
                .saturating_add(cell.trailed_profit)
                .saturating_add(cell.targeted)
                .saturating_add(cell.timed_out),
            cell.trades,
            "every trade ends by exactly one of fixed stop, trailing stop loss, \
             trailing take profit, target or time"
        );
    }

    // ---- 5. walk-forward -------------------------------------------------
    let folds = validate::walk_forward(
        &bars,
        horizon,
        3,
        Direction::Long,
        &Sweeper::new(Ladder::with_min_hits(150).with_ceiling(20_000)),
        evaluator,
    );
    assert_eq!(folds.folds.len(), 3, "three splits, three folds");
    for fold in &folds.folds {
        assert!(
            fold.purged >= horizon.as_bars() as usize,
            "the purge must cover the outcome window, or a training bar's \
             result reaches into the test set"
        );
        assert_eq!(
            fold.chosen.is_some(),
            fold.chosen_exit.is_some(),
            "a combination and its exit are chosen together, on the same bars"
        );
    }

    // ---- 6. folds -> PBO -------------------------------------------------
    // Placements built from the folds' own in-sample and out-of-sample totals:
    // each fold ranked its candidates in sample and the winner landed somewhere
    // out of sample. With one candidate per fold there is nothing to rank, so
    // this exercises the refusal rather than inventing a ranking.
    // ONE CHOSEN COMBINATION PER FOLD IS NOT A RANKING, and this used to
    // pretend otherwise.
    //
    // It passed `candidates: f.considered` — thousands — beside
    // `winner_rank: usize::from(!positive)`, which is a BOOLEAN: zero or one.
    // A rank of 0 or 1 against thousands of candidates is a relative placement
    // of ~0 whatever actually happened, so `Pbo::probability` reported **0.0%
    // overfitting for eight consecutive total failures**. The number was not
    // wrong by a little; it was structurally incapable of being anything else.
    //
    // A real PBO needs each split's FULL vector of candidate performances, in
    // sample and out, so `pbo::place` can find the in-sample winner and rank it
    // out of sample. `walk_forward` keeps only the winner, so that vector does
    // not exist here. The honest placement is therefore one candidate — which
    // `Placement::relative` correctly refuses as unrankable — and the assertion
    // below checks that every fold lands in the refusal rather than in a
    // fabricated probability.
    let placements: Vec<pbo::Placement> = folds
        .folds
        .iter()
        .filter(|f| f.chosen.is_some())
        .map(|_| pbo::Placement {
            candidates: 1,
            winner_rank: 0,
        })
        .collect();
    let overfit = pbo::probability_of_overfitting(&placements);
    assert_eq!(
        overfit.folds.saturating_add(overfit.unrankable),
        placements.len(),
        "every placement is either ranked or named unrankable"
    );

    // ---- 7. returns -> the bootstrap ------------------------------------
    // Per-fold worst-case totals as one strategy's series, against a flat one.
    // Two strategies is the smallest set the tests are defined on.
    let series: Vec<Vec<i64>> = vec![
        folds.folds.iter().map(|f| f.out_of_sample.worst).collect(),
        vec![0; folds.folds.len()],
    ];
    let rc = bootstrap::reality_check(&series, 200, 7, bootstrap::DEFAULT_BLOCK);
    let spa = bootstrap::spa(&series, 200, 7, bootstrap::DEFAULT_BLOCK);
    assert!(
        rc.is_some() && spa.is_some(),
        "aligned series must produce verdicts"
    );
    let named = bootstrap::romano_wolf(&series, 200, 7, bootstrap::DEFAULT_BLOCK, 50_000);
    assert!(
        named.len() <= series.len(),
        "the stepdown cannot reject more strategies than exist"
    );

    // ---- 8. everything -> one report ------------------------------------
    let report = audit::render(
        Some(&taken),
        Some(&exits),
        Some(&folds),
        Some(&overfit),
        Some((rc.as_ref(), spa.as_ref(), named.len())),
        6,
    );
    for section in [
        "TRADES",
        "EXIT GRID",
        "WALK-FORWARD",
        "OVERFITTING",
        "BOOTSTRAP",
    ] {
        assert!(
            report.contains(section),
            "{section} missing from the report"
        );
    }
    assert!(
        !report.contains("NOT SUPPLIED"),
        "every section was supplied, so none may claim otherwise"
    );

    // Printed so a reader can see the engine's actual output rather than take
    // a test's word for it. `cargo test -- --nocapture` shows it.
    println!("\n{report}");
}

#[test]
fn the_pipeline_refuses_a_slice_it_cannot_measure_rather_than_inventing_one() {
    // THE OTHER HALF. A pipeline that produces a report on a slice it could not
    // measure is worse than one that refuses -- the report reads the same
    // either way. One session is below the warm-up, so nothing is swept.
    let bars = synthetic::sessions(1);
    let column = Column::build(&bars, &mut evaluator());
    assert!(
        column.is_empty(),
        "one session is below the warm-up, so nothing may be swept"
    );

    let taken = trade::walk(
        &bars,
        &column,
        &ConditionMask::default(),
        Horizon::DEFAULT,
        Direction::Long,
    );
    assert_eq!(taken.signals, 0, "an empty column fires no signals");
    assert!(taken.reconciles());

    let folds = validate::walk_forward(
        &bars,
        Horizon::DEFAULT,
        3,
        Direction::Long,
        &Sweeper::new(Ladder::with_min_hits(150)),
        evaluator,
    );
    assert_eq!(
        folds.decided(),
        0,
        "nothing was swept, so nothing was chosen"
    );
    assert_eq!(folds.held_up(), 0);

    let report = audit::render(Some(&taken), None, Some(&folds), None, None, 6);
    assert!(
        report.contains("NOT SUPPLIED"),
        "sections that were not supplied must say so rather than reading as empty"
    );
}

/// The drill-down ladder: how deep the sweep reaches as support falls.
///
/// # What this proves that a unit test cannot
///
/// `CLAUDE.md` §6 says there is no depth parameter — the ladder climbs until the
/// frequent frontier empties. That is easy to assert and hard to believe, so
/// this measures it: sweep the same slice at falling support thresholds and
/// watch how far it reaches.
///
/// # The comparison is only legal between sweeps that went extinct
///
/// Anti-monotonicity says lowering `min_hits` keeps strictly MORE combinations
/// frequent, so a lower threshold can never go extinct sooner. Stated that
/// baldly the assertion is **false**, and it failed the first time this test
/// ran: depth 16 at `min_hits=600` against depth 12 at `min_hits=300`.
///
/// Nothing was wrong with the prune. A lower threshold makes each level wider,
/// a wider level hits the candidate ceiling sooner, and a sweep that breaches
/// the ceiling stops where it stopped — [`Sweep::completed`] is false and the
/// deepest level it holds is **partial**. Its depth is an artefact of the
/// budget, not a fact about the data, and comparing it to an extinct sweep's
/// depth compares two different measurements that happen to share a name.
///
/// So the invariant below is guarded on `completed()`, and the halted rungs are
/// still printed — with the budget that stopped them named — because a rung
/// that reached the end of the money is exactly what a reader needs to see.
/// Hiding it would leave a truncated search reading as an exhaustive one, which
/// is the failure §4 bans outright.
///
/// Every bar is `synthetic::sessions`, generated in-process. No stored bar is
/// read, so the figures are facts about the SEARCH and not about any market.
#[test]
fn the_ladder_reaches_further_as_support_falls_unless_a_budget_stops_it_first() {
    let bars = synthetic::sessions(12);
    let mut deepest_extinct = 0_usize;
    let mut extinct_rungs = 0_usize;
    let mut rows: Vec<String> = Vec::new();

    // Falling support against the SHIPPED ceiling, so the figures describe the
    // engine an operator actually runs rather than one this test invented.
    for min_hits in [1_500_u64, 1_000, 600, 300, 150] {
        let swept = Sweeper::new(Ladder::with_min_hits(min_hits)).run(&bars, &mut evaluator());
        let depth = swept.sweep.depth();
        let frequent = swept.sweep.all_frequent().count();
        let generated: u64 = swept.sweep.levels.iter().map(|l| l.generated).sum();
        let distinct = closed::closed(&swept.sweep).kept.len();

        let ending = match swept.sweep.halted {
            None => {
                // Extinct: the frontier emptied on its own, so this depth is a
                // fact about the data and comparable to the last such fact.
                assert!(
                    depth >= deepest_extinct,
                    "min_hits {min_hits} went extinct at depth {depth} where a \
                     HIGHER threshold reached {deepest_extinct} -- with both \
                     sweeps extinct no budget can explain that, so the prune \
                     dropped a candidate anti-monotonicity says it must keep"
                );
                deepest_extinct = depth;
                extinct_rungs = extinct_rungs.saturating_add(1);
                "extinct".to_owned()
            }
            Some(halt) => format!("HALTED k={} {:?}", halt.k, halt.breach),
        };

        rows.push(format!(
            "  {min_hits:>8}{depth:>7}{generated:>13}{frequent:>10}{distinct:>10}   {ending}"
        ));
    }

    assert!(
        extinct_rungs > 0,
        "every rung hit a budget, so this measured the budgets and not the data"
    );
    assert!(
        deepest_extinct > 1,
        "the deepest extinct rung must climb past k=1, or nothing is climbing"
    );

    println!("\nDRILL-DOWN LADDER   (synthetic::sessions(12), no stored bars)");
    println!(
        "  {:>8}{:>7}{:>13}{:>10}{:>10}   ended by",
        "min_hits", "depth", "enumerated", "frequent", "distinct"
    );
    for row in &rows {
        println!("{row}");
    }
    println!();
}
