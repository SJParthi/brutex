//! BASE-COV-LIBS-LS2: the population door over a sweep that halts at its
//! ceiling. The levels it retired before the halt have no successor to decide
//! their closure, so their members stream as Unknown, the run says its
//! closure is incomplete, and the run as a whole is not complete.
//!
//! `runner::Sweeper::run_prepared_population_by_reporting` is tested in
//! `lib.rs` on a ladder that goes extinct, where every verdict is decided;
//! this is the halted case beside it (lib.rs is LSA's, so it is tested here).

#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes: a fixture \
              that does not hold must fail loudly."
)]

use indicators::column::Column;
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::{ClosureVerdict, PopulationMember, Sweeper};

fn evaluator() -> Evaluator {
    Evaluator::new(
        Widths::pinned().expect("both pinned widths are valid"),
        Availability::Absent,
        Thresholds::CLASSICAL,
    )
}

#[test]
fn a_halted_population_streams_unknown_closures_and_is_not_complete() {
    let bars = runner::synthetic::sessions(8);
    let column = Column::build(&bars, &mut evaluator());
    let halting = engine::Ladder::with_min_hits(600)
        .with_ceiling(5)
        .with_support_lanes(1);
    let mut members: Vec<PopulationMember> = Vec::new();
    let population = Sweeper::new(halting)
        .run_prepared_population_by_reporting(
            column,
            &|_, _, _| {},
            |member| -> Result<(), &'static str> {
                members.push(member);
                Ok(())
            },
        )
        .expect("the in-memory sink cannot refuse");
    assert!(
        population.outcome.sweep.halted.is_some(),
        "the premise: a halt"
    );
    assert!(!population.outcome.closure_complete);
    assert!(!population.is_complete());
    assert!(
        members
            .iter()
            .any(|member| member.closure == ClosureVerdict::Unknown),
        "a level retired without a successor has no decided closure"
    );
    assert_eq!(
        population.considered,
        u64::try_from(members.len()).expect("a small fixture")
    );
    assert!(population.closed.saturating_add(population.redundant) < population.considered);
}
