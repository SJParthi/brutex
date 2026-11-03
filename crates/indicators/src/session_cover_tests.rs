#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail; `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind"
)]
//! BASE-COV-LIBS-LS3 (attempt 5): the one side of `session.rs` no input to the
//! public API reaches. `SessionState::new` starts at day `i64::MIN`, which no
//! bar carries, so through the public API "not live" always comes with
//! "another day"; a state on its bar's own day that is not live is built here,
//! and it must start the session again rather than continue one it never saw.
//! Extreme cases: a stale first bar whose high is above and whose low is below
//! the second's, so either extreme surviving would show; and (the combination)
//! the same state stepped on, where the restarted session then carries both
//! later bars and nothing before them.

use super::SessionState;
use crate::Candle;
use vocab::tolerance::pinned_fib;

const MINUTE_MICROS: i64 = 60 * 1_000_000;
/// 09:15 IST on day 100.
const OPEN: i64 = 100 * 24 * 60 * MINUTE_MICROS + (555 - 330) * MINUTE_MICROS;

fn candle(minute: i64, high: i64, low: i64) -> Candle {
    Candle::new(
        OPEN + minute * MINUTE_MICROS,
        low,
        high,
        low,
        high,
        10,
        crate::OI_NULL,
    )
}

#[test]
fn a_state_on_its_bars_own_day_that_is_not_live_starts_the_session_again() {
    let fib = pinned_fib().expect("pinned");
    let first = candle(0, 2_000_900, 2_000_000);
    let second = candle(1, 2_000_500, 2_000_400);
    let third = candle(2, 2_000_600, 2_000_300);

    let mut continuing = SessionState::default();
    continuing.step(&first, None, fib).expect("a sane bar");
    let mut stale = continuing;
    stale.live = false;

    continuing.step(&second, None, fib).expect("a sane bar");
    assert_eq!(
        continuing.day_extremes(),
        Some((2_000_900, 2_000_000)),
        "a live state keeps the first bar's extremes"
    );
    stale.step(&second, None, fib).expect("a sane bar");
    assert_eq!(
        stale.day_extremes(),
        Some((2_000_500, 2_000_400)),
        "the stale first bar is not in the restarted session"
    );
    stale.step(&third, None, fib).expect("a sane bar");
    assert_eq!(stale.day_extremes(), Some((2_000_600, 2_000_300)));
}
