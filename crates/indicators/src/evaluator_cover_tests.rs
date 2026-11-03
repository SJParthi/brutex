#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail; `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind"
)]
//! BASE-COV-LIBS-LS3 (attempt 5): the parts of `evaluator.rs` no input to the
//! public API reaches, each driven directly or from a state built here, where
//! the fields are visible.
//!
//! Two of them are conjuncts the code keeps on purpose although another
//! condition already implies them (see `Evaluator::warmed_up`: "a redundant
//! conjunct that documents a requirement is cheaper than rediscovering the
//! requirement after the assignment changes"). A test that breaks the other
//! condition shows each one deciding on its own, which is what stops a later
//! edit deleting it unseen. Extreme cases: the first bar of a day and a bar
//! later in it, each with the session seeded and not; a fully warm evaluator
//! with the previous session taken away and put back; the base tag of no base
//! and of both bases; and (the combinations) an unseeded state on its own day
//! whose truth AND availability both drop the session-open pair, and a warm
//! state whose prev-5 ladder and yesterday both still hold while the previous
//! session is gone.

use super::{Evaluator, tolerance_base_tag};
use crate::Candle;
use crate::evaluator::Widths;
use crate::pattern::Thresholds;
use crate::vwap::Availability;
use vocab::tolerance::Base;

const MINUTE_MICROS: i64 = 60 * 1_000_000;
const DAY_MICROS: i64 = 24 * 60 * MINUTE_MICROS;
/// 09:15 IST as microseconds past midnight UTC.
const OPEN_UTC_MICROS: i64 = (555 - 330) * MINUTE_MICROS;

/// A deterministic, sane bar on `day` at `minute` past the open.
fn bar(day: i64, minute: i64) -> Candle {
    let close = 2_500_000 + day * 400 + minute * 3 + ((minute % 7) - 3) * 25;
    Candle::new(
        day * DAY_MICROS + OPEN_UTC_MICROS + minute * MINUTE_MICROS,
        close - 10,
        close + 60,
        close - 70,
        close,
        1_000,
        crate::OI_NULL,
    )
}

fn evaluator() -> Evaluator {
    Evaluator::new(
        Widths::pinned().expect("pinned widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
    )
}

/// **THE THREE BASE TAGS ARE DISTINCT, AND NO BASE IS ZERO.** The canonical
/// spec bytes carry a tolerance's base as this tag; no `Widths` the evaluator
/// accepts has a tolerance without a base, so the `None` tag is pinned here.
#[test]
fn the_base_tags_are_distinct_and_no_base_is_zero() {
    assert_eq!(
        [
            tolerance_base_tag(None),
            tolerance_base_tag(Some(Base::SessionRange)),
            tolerance_base_tag(Some(Base::CprWidth)),
        ],
        [0, 1, 2]
    );
}

/// **A BAR ON ITS STATE'S OWN DAY, BEFORE THE SESSION WAS SEEDED, CLAIMS NO
/// SESSION OPEN** — neither the truth nor the availability of 276/277. The
/// day check alone cannot say this: the state is on the bar's day. Through
/// the public API a state on a bar's day has always been seeded by an earlier
/// bar of it, so the unseeded one is built here.
#[test]
fn an_unseeded_state_on_the_bars_own_day_claims_no_session_open() {
    let mut seeded = evaluator();
    seeded.step(&bar(10, 0)).expect("a sane first bar");
    let mut unseeded = seeded;
    unseeded.seeded = false;

    let (_, known) = seeded.step_known(&bar(10, 1)).expect("a sane bar");
    assert_eq!(
        (known.get(276), known.get(277)),
        (true, true),
        "seeded: known"
    );

    let (truth, known) = unseeded.step_known(&bar(10, 1)).expect("a sane bar");
    assert_eq!(
        (
            truth.get(276),
            truth.get(277),
            known.get(276),
            known.get(277)
        ),
        (false, false, false, false),
        "unseeded on its own day: the pair is neither true nor known"
    );
    // The bar seeded it: the next bar of the day reads the session open again.
    let (_, known) = unseeded.step_known(&bar(10, 2)).expect("a sane bar");
    assert_eq!((known.get(276), known.get(277)), (true, true));
}

/// **WARM NEEDS THE PREVIOUS SESSION, EVEN WITH THE LADDER FULL AND
/// YESTERDAY SET.** Taken away, the evaluator is not warm; put back, it is.
#[test]
fn warm_needs_the_previous_session_even_with_the_ladder_full() {
    let mut e = evaluator();
    for day in 0..8 {
        for minute in 0..375 {
            e.step(&bar(day, minute)).expect("a sane bar");
        }
    }
    assert!(e.warmed_up(), "eight sessions warm the evaluator");
    assert!(e.prev5.filled() >= 5, "the ladder is full");
    assert!(e.yesterday.is_some(), "yesterday is set");
    let previous = e.previous.take();
    assert!(!e.warmed_up(), "no previous session: not warm");
    e.previous = previous;
    assert!(e.warmed_up());
}
