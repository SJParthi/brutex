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
                .saturating_add(cell.targeted)
                .saturating_add(cell.timed_out),
            cell.trades,
            "every trade ends by exactly one of stop, target or time"
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
    let placements: Vec<pbo::Placement> = folds
        .folds
        .iter()
        .map(|f| pbo::Placement {
            candidates: usize::try_from(f.considered).unwrap_or(1).max(1),
            winner_rank: usize::from(!f.out_of_sample.worst_case_positive()),
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
