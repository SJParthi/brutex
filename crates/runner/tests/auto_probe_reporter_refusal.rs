//! BASE-COV-LIBS-LS2: the automatic threshold search ends with the reason its
//! level reporter gave. The reporter is asked once per probe level until it
//! refuses; the probe's later levels are walked without asking it again, and
//! the search returns that first reason instead of a threshold.
//!
//! `runner::Sweeper::auto_prepared_reporting` is otherwise tested in `lib.rs`
//! with reporters that accept every event (lib.rs is LSA's, so the refusal is
//! tested here).

#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes: a fixture \
              that does not hold must fail loudly."
)]

use indicators::column::Column;
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::{AutoProbeEvent, Sweeper};
use std::cell::Cell;

fn evaluator() -> Evaluator {
    Evaluator::new(
        Widths::pinned().expect("both pinned widths are valid"),
        Availability::Absent,
        Thresholds::CLASSICAL,
    )
}

#[test]
fn a_reporter_that_refuses_a_level_ends_the_auto_search_with_its_reason() {
    let bars = runner::synthetic::sessions(8);
    let column = Column::build(&bars, &mut evaluator());
    let sweeper = Sweeper::new(
        engine::Ladder::with_min_hits(600)
            .with_ceiling(50_000)
            .with_support_lanes(1),
    );

    // The premise, from a reporter that accepts everything: some probe walks
    // a level with frequent sets and then a further level. Count the level
    // events up to the first such level, and whether one followed it.
    let levels_seen = Cell::new(0_u32);
    let first_frequent = Cell::new(None::<u32>);
    let followed = Cell::new(false);
    let accepted = sweeper.auto_prepared_reporting(&column, &|event| {
        match event {
            AutoProbeEvent::Level { frontier, .. } => {
                levels_seen.set(levels_seen.get() + 1);
                if first_frequent.get().is_some() {
                    followed.set(true);
                } else if !frontier.frequent.is_empty() {
                    first_frequent.set(Some(levels_seen.get()));
                }
            }
            AutoProbeEvent::Finished(_) if !followed.get() => first_frequent.set(None),
            _ => {}
        }
        Ok(())
    });
    assert!(accepted.is_ok());
    let at = first_frequent
        .get()
        .expect("a probe walks a level with frequent sets and then another level");
    assert!(followed.get());

    // The same search with a reporter that refuses the first level holding
    // frequent sets: it is asked exactly that many times, its later levels in
    // that probe are walked without asking it, and its reason is the answer.
    let asked = Cell::new(0_u32);
    let refused = sweeper.auto_prepared_reporting(&column, &|event| match event {
        AutoProbeEvent::Level { frontier, .. } => {
            asked.set(asked.get() + 1);
            if frontier.frequent.is_empty() {
                Ok(())
            } else {
                Err(format!("level reporter refused at call {}", asked.get()))
            }
        }
        _ => Ok(()),
    });
    assert_eq!(
        refused.map(|auto| auto.attempts),
        Err(format!("level reporter refused at call {at}"))
    );
    assert_eq!(asked.get(), at, "a refused probe asks its reporter no more");
}
