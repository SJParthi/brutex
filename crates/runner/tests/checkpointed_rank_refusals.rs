//! BASE-COV-LIBS-LS2: the checkpointed-rank adapter refuses a restored sweep
//! that is not a terminal sequence, and a signal column whose rows no longer
//! number the bars it swept.
//!
//! `runner::rank_checkpointed_sweep` re-ranks a sweep restored from a
//! checkpoint. Its own tests in `lib.rs` cover a missing terminal, a bar count
//! off by one, an empty sweep and a level whose generation count moved; these
//! are the two refusals they leave: a level numbered out of order, and a column
//! projected onto fills, whose row count no longer equals the bars it swept.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "the exception every test module in this workspace takes: a fixture \
              that does not hold must fail loudly, and the mutated level is the \
              fixture's own last one."
)]

use indicators::column::Column;
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::rank::Lens;

fn evaluator() -> Evaluator {
    Evaluator::new(
        Widths::pinned().expect("both pinned widths are valid"),
        Availability::Absent,
        Thresholds::CLASSICAL,
    )
}

#[test]
fn a_restored_sweep_out_of_sequence_or_over_a_projected_column_is_refused() {
    let bars = runner::synthetic::sessions(8);
    let signal = Column::build(&bars, &mut evaluator());
    let forward = runner::outcome::forward(&bars, &signal, runner::outcome::Horizon::DEFAULT);
    let sweeper = runner::Sweeper::new(
        engine::Ladder::with_min_hits(600)
            .with_ceiling(50_000)
            .with_support_lanes(1),
    );
    let original = sweeper.run_prepared(&signal).sweep;
    assert!(original.levels.len() >= 2, "the fixture sweeps past k = 1");
    let rank = |column: Column, sweep: engine::Sweep| {
        runner::rank_checkpointed_sweep(column, None, &forward, sweep, 10, Lens::Detectability)
            .map(|_| ())
    };
    assert_eq!(rank(signal.clone(), original.clone()), Ok(()));
    let refused = Err("restored sweep and signal column/accounting disagree".to_owned());

    // The terminal level renumbered: every level still reconciles its own
    // counts, and the sequence 1, 2, ... is broken at its end.
    let mut renumbered = original.clone();
    let last = renumbered.levels.len() - 1;
    renumbered.levels[last].k += 1;
    assert!(renumbered.levels.iter().all(engine::Frontier::reconciles));
    assert_eq!(rank(signal.clone(), renumbered), refused, "renumbered");

    // The same column projected onto fills with its first row dropped: its
    // rows are one fewer, its census still counts every bar it swept, and a
    // sweep restored with the row count agrees with the rows and not the census.
    let rows = signal.sources().len();
    let onto: Vec<Option<usize>> = signal
        .sources()
        .iter()
        .enumerate()
        .map(|(row, &source)| (row > 0).then_some(source))
        .collect();
    let (projected, dropped) = signal
        .reproject(&onto, bars.len())
        .expect("a non-decreasing map inside the series");
    assert_eq!(dropped, 1);
    assert_eq!(projected.bits().len(), rows - 1);
    assert_eq!(projected.census().swept, signal.census().swept);
    let mut recounted = original;
    recounted.bars = u64::try_from(rows - 1).expect("a small fixture");
    assert_eq!(rank(projected, recounted), refused, "projected");
}
