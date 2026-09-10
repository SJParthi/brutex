//! Exact admission/terminal boundaries around the shared threshold search.
#![allow(
    clippy::expect_used,
    reason = "test fixture failures must fail the test"
)]

use engine::Ladder;
use indicators::column::Column;
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::{AutoProbeEvent, Sweeper};
use std::cell::{Cell, RefCell};

fn column() -> Column {
    let bars = runner::synthetic::sessions(8);
    let mut evaluator = Evaluator::new(
        Widths::pinned().expect("valid pinned widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
    );
    Column::build(&bars, &mut evaluator)
}

#[test]
fn a_refused_probe_start_prevents_every_level_and_finish() {
    let input = column();
    assert!(input.census().swept > 1);
    let events = Cell::new(0);
    let result = Sweeper::new(Ladder::with_min_hits(1).with_ceiling(64)).auto_prepared_reporting(
        &input,
        &|event| {
            events.set(events.get() + 1);
            assert!(matches!(event, AutoProbeEvent::Started(_)));
            Err("identity reservation was not durable".to_owned())
        },
    );
    assert_eq!(
        result.expect_err("no admitted probe"),
        "identity reservation was not durable"
    );
    assert_eq!(events.get(), 1);
}

#[test]
fn exact_probe_boundaries_preserve_the_original_auto_answer() {
    let input = column();
    let sweeper = Sweeper::new(Ladder::with_min_hits(1).with_ceiling(128));
    let ordinary = sweeper.auto_prepared(&input);
    let active = Cell::new(false);
    let starts = Cell::new(0_u32);
    let finishes = Cell::new(0_u32);
    let counters = RefCell::new(Vec::new());
    let reported = sweeper
        .auto_prepared_reporting(&input, &|event| {
            match event {
                AutoProbeEvent::Started(ladder) => {
                    assert!(!active.replace(true));
                    assert_eq!(ladder.ceiling(), 128);
                    assert!(ladder.min_hits() > 0);
                    starts.set(starts.get() + 1);
                    counters.borrow_mut().clear();
                }
                AutoProbeEvent::Level {
                    frontier,
                    admitted,
                    pairs,
                } => {
                    assert!(active.get());
                    counters.borrow_mut().push((frontier.k, admitted, pairs));
                }
                AutoProbeEvent::Finished(sweep) => {
                    assert!(active.replace(false));
                    finishes.set(finishes.get() + 1);
                    assert_eq!(counters.borrow().len(), sweep.levels.len());
                }
            }
            Ok(())
        })
        .expect("durable reporter accepted every event");
    assert!(!active.get());
    assert_eq!(starts.get(), reported.attempts);
    assert_eq!(finishes.get(), reported.attempts);
    assert_eq!(ordinary.min_hits, reported.min_hits);
    assert_eq!(ordinary.refused_below, reported.refused_below);
    assert_eq!(ordinary.affordable, reported.affordable);
    assert_eq!(ordinary.attempts, reported.attempts);
    assert_eq!(
        format!("{:?}", ordinary.outcome),
        format!("{:?}", reported.outcome)
    );
}

#[test]
fn a_refused_probe_finish_prevents_the_next_start() {
    let input = column();
    let starts = Cell::new(0);
    let result = Sweeper::new(Ladder::with_min_hits(1).with_ceiling(64)).auto_prepared_reporting(
        &input,
        &|event| match event {
            AutoProbeEvent::Started(_) => {
                starts.set(starts.get() + 1);
                Ok(())
            }
            AutoProbeEvent::Level { .. } => Ok(()),
            AutoProbeEvent::Finished(_) => Err("terminal seal was not durable".to_owned()),
        },
    );
    assert_eq!(
        result.expect_err("failed terminal"),
        "terminal seal was not durable"
    );
    assert_eq!(starts.get(), 1);
}
