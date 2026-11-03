#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail; `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind"
)]
//! BASE-COV-LIBS-LS3 (attempt 5): the sides of `trend.rs` no test above
//! reached.
//!
//! * **A period of zero is warm before it has a value.** `TrendThresholds`
//!   takes any period and `Ema::warm` is `folded >= period`, so a zero period
//!   is warm on a fresh state, while `Ema::value` is still `None` until the
//!   first price is folded. Every gate that asks for warm AND a value then
//!   decides on the value alone: the averages (0-5) and the trailing stop
//!   (64, 65) are neither true nor known on the first candle. Nothing refuses a
//!   zero period, and nothing divides by it (`Ema::fold` and `Atr::fold` stop
//!   at a non-positive denominator), so this is an input the type admits.
//! * **`SwingDetector::confirm`'s two defensive arms**, which its own comment
//!   says no call reaches (it runs only once the ring is full). They publish
//!   nothing, and that is driven here on a ring with an empty slot.
//!
//! Extreme cases: periods of zero for both averages and the ATR at once (the
//! combination), a negative period, the first candle and the second; an
//! empty ring, and a ring whose middle is filled while its oldest slot is not.

use super::{FRACTAL, SwingDetector, TrendState, TrendThresholds};
use crate::Candle;
use vocab::tolerance::pinned_fib;

const MINUTE_MICROS: i64 = 60 * 1_000_000;

fn candle(minute: i64, close: i64) -> Candle {
    Candle::new(
        minute * MINUTE_MICROS,
        close,
        close + 50,
        close - 50,
        close,
        10,
        crate::OI_NULL,
    )
}

/// The trend positions a warm average or stop speaks for.
const GATED: [u32; 8] = [0, 1, 2, 3, 4, 5, 64, 65];

fn gated(mask: vocab::ConditionMask) -> Vec<bool> {
    GATED.iter().map(|p| mask.get(*p)).collect()
}

#[test]
fn a_period_of_zero_is_warm_before_it_has_a_value_and_speaks_for_nothing_yet() {
    let fib = pinned_fib().expect("pinned");
    for period in [0_i128, -1] {
        let mut t = TrendState::new(TrendThresholds {
            ema_fast: period,
            ema_slow: period,
            atr_period: period,
            supertrend_mult: 3_000,
        });
        assert_eq!(
            gated(t.known(fib)),
            [false; 8],
            "{period}: fresh, nothing known"
        );
        let truth = t.step(&candle(0, 2_000_000), fib).expect("a sane candle");
        assert_eq!(
            gated(truth),
            [false; 8],
            "{period}: first candle, nothing true"
        );
        // One price folded: every gate now has its value, and speaks.
        let known = t.known(fib);
        assert_eq!(
            gated(known),
            [true; 8],
            "{period}: after one candle, all known"
        );
    }
}

#[test]
fn a_ring_with_an_empty_slot_publishes_no_swing() {
    let mut empty = SwingDetector::new();
    empty.confirm();
    assert_eq!((empty.swing_high(), empty.swing_low()), (None, None));

    let mut middle_only = SwingDetector::new();
    *middle_only
        .ring
        .get_mut(FRACTAL)
        .expect("the middle is in the ring") = Some((2_000_500, 2_000_400));
    middle_only.confirm();
    assert_eq!(
        (middle_only.swing_high(), middle_only.swing_low()),
        (None, None),
        "a middle with no neighbours is not a swing"
    );
}
