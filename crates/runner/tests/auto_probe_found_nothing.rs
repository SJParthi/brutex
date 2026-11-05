//! BASE-COV-LIBS-LS2 (5 Nov 2026): the auto threshold search's bisection
//! narrows from above on a probe that completes and finds nothing, and never
//! answers with it.
//!
//! The series is a deterministic random walk (a fixed linear congruential
//! stream, steps up to 400 paisa a minute, wicks of 20 to 119 paisa) on six
//! generated sessions, so one session is swept after the evaluator's warm-up.
//! Under a ceiling of one, its halving phase completes with nothing frequent
//! at the top, is refused at half the rows, and the bisection between them
//! meets a probe above every condition's support: it completes at depth zero,
//! the search moves its upper bound there, and later probes find the answer.
//!
//! Extreme cases: a ceiling of one; a probe that completes with nothing
//! frequent inside the bisection (the out-of-the-box case: the halving phase
//! answers nothing at its top, so only the bisection can find a threshold);
//! the combination of a refusal at depth one followed by a refusal at depth
//! two before it.

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
use std::cell::RefCell;

fn evaluator() -> Evaluator {
    Evaluator::new(
        Widths::pinned().expect("both pinned widths are valid"),
        Availability::Absent,
        Thresholds::CLASSICAL,
    )
}

/// Six generated sessions' minutes carrying a random walk from one fixed
/// linear congruential stream: each minute moves its close by up to 400 paisa
/// either way from its open, and hangs a wick of 20 to 119 paisa each side.
fn random_walk() -> Vec<indicators::Candle> {
    let mut state = 2_u64;
    let mut price = 2_500_000_i64;
    runner::synthetic::sessions(6)
        .into_iter()
        .map(|bar| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let step = i64::try_from(state >> 40).expect("24 bits fit") % 2_001 - 1_000;
            let open = price;
            let close = price + step * 400 / 1_000;
            price = close;
            let wick = i64::try_from((state >> 20) % 100).expect("fits") + 20;
            indicators::Candle::new(
                bar.ts_micros,
                open,
                open.max(close) + wick,
                open.min(close) - wick,
                close,
                bar.volume,
                bar.open_interest,
            )
        })
        .collect()
}

#[test]
fn a_bisection_probe_that_finds_nothing_narrows_the_search_and_is_not_the_answer() {
    let column = Column::build(&random_walk(), &mut evaluator());
    assert_eq!(column.census().swept, 374, "one session after the warm-up");
    let sweeper = Sweeper::new(
        engine::Ladder::with_min_hits(1)
            .with_ceiling(1)
            .with_support_lanes(1),
    );
    let probes = RefCell::new(Vec::new());
    let threshold = RefCell::new(0_u64);
    let auto = sweeper
        .auto_prepared_reporting(&column, &|event| {
            match event {
                AutoProbeEvent::Started(ladder) => *threshold.borrow_mut() = ladder.min_hits(),
                AutoProbeEvent::Finished(sweep) => probes.borrow_mut().push((
                    *threshold.borrow(),
                    sweep.completed(),
                    sweep.depth(),
                )),
                AutoProbeEvent::Level { .. } => {}
            }
            Ok(())
        })
        .expect("a reporter that accepts everything");
    // (threshold, completed, depth), probe by probe: the halving phase, then
    // the bisection between its refusal and its last completing threshold.
    assert_eq!(
        probes.into_inner(),
        vec![
            (373, true, 0),
            (186, false, 1),
            (279, false, 2),
            (326, true, 0),
            (302, true, 1),
            (290, true, 2),
            (284, true, 2),
            (281, true, 2),
            (280, true, 2),
        ]
    );
    // The depth-zero probe at 326 moved the upper bound and is not the
    // answer; the lowest completing threshold above the refusal is.
    assert_eq!(auto.min_hits, Some(280));
    assert!(auto.affordable);
    assert_eq!(auto.outcome.sweep.depth(), 2);
    assert_eq!(auto.outcome.sweep.min_hits, 280);
    assert_eq!(auto.refused_below, Some(279));
    assert_eq!(auto.attempts, 9);
}
