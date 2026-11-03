#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail; `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind"
)]
//! BASE-COV-LIBS-LS3 (attempt 5): the parts of `vwap.rs` no input to the
//! public API reaches.
//!
//! `Refused::into_corrupt` is the evaluator's one mapping of a VWAP refusal
//! (D-8565); the evaluator never hands this module a bar `Candle::check`
//! would refuse, so the record-fault half is proved here, directly. And a state
//! on its bar's own day that is not live resets rather than continuing:
//! `for_slice` starts at day `i64::MIN`, which no bar carries, so only a state
//! built here is on the bar's day without being live. Extreme cases: every
//! record fault `Candle::check_evaluable` names, the accumulator refusal, a
//! stale accumulator from an earlier bar of the same day, and (the
//! combination) that stale state also carrying a contributing count.

use super::{Availability, Refused, Vwap};
use crate::{Candle, Corrupt};

const MINUTE_MICROS: i64 = 60 * 1_000_000;
/// 09:15 IST on day 100, as microseconds since the epoch.
const DAY_100_OPEN: i64 = 100 * 24 * 60 * MINUTE_MICROS + (555 - 330) * MINUTE_MICROS;

/// **EACH REFUSAL KEEPS ITS OWN NAME THROUGH THE EVALUATOR.** A record fault
/// reaches the caller as the fault it was; the accumulator's refusal as the
/// accumulator's.
#[test]
fn each_refusal_keeps_its_own_name_through_the_evaluator() {
    for why in [
        Corrupt::HighBelowLow,
        Corrupt::RangeOverflows,
        Corrupt::PriceOutsideRange,
        Corrupt::NegativeVolume,
        Corrupt::PriceNotPositive,
        Corrupt::AccumulatorTooLarge,
    ] {
        assert_eq!(Refused::Corrupt(why).into_corrupt(), why);
    }
    assert_eq!(
        Refused::AccumulatorTooLarge.into_corrupt(),
        Corrupt::AccumulatorTooLarge
    );
    // Through the module itself: a bar `check_evaluable` refuses comes back as
    // a record fault, by the same mapping.
    let broken = Candle::new(DAY_100_OPEN, 100, 50, 90, 100, 1, crate::OI_NULL);
    let refused = Vwap::for_slice(Availability::Present)
        .fold(&broken)
        .expect_err("high below low is not a bar");
    assert_eq!(refused.into_corrupt(), Corrupt::HighBelowLow);
}

/// **A STATE ON ITS BAR'S OWN DAY THAT IS NOT LIVE STARTS THE SESSION AGAIN**,
/// so an accumulator left from an earlier bar never leaks into it.
#[test]
fn a_state_on_its_bars_own_day_that_is_not_live_starts_the_session_again() {
    let first = Candle::new(DAY_100_OPEN, 1_000, 1_100, 900, 1_000, 500, crate::OI_NULL);
    let second = Candle::new(
        DAY_100_OPEN + MINUTE_MICROS,
        2_000,
        2_100,
        1_900,
        2_000,
        500,
        crate::OI_NULL,
    );

    let mut continuing = Vwap::for_slice(Availability::Present);
    continuing.fold(&first).expect("a sane bar");
    let mut stale = continuing;
    stale.live = false;
    continuing.fold(&second).expect("a sane bar");
    stale.fold(&second).expect("a sane bar");

    let mut alone = Vwap::for_slice(Availability::Present);
    alone.fold(&second).expect("a sane bar");
    assert_eq!(stale, alone, "the earlier bar of the day is gone");
    assert_ne!(
        continuing.value(),
        alone.value(),
        "a live state carries the earlier bar: the reset is what differs"
    );
}
