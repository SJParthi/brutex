#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail; `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind"
)]
//! BASE-COV-LIBS-LS3 (attempt 5): two sides of `orb.rs` no input to the public
//! API reaches, driven from states built here, where the fields are visible.
//!
//! * A state on its bar's own day that is not live starts the session again.
//!   `Orb::new` starts at day `i64::MIN`, which no bar carries, so through the
//!   public API "not live" always comes with "another day"; this is the state
//!   a derived `Default` would have produced (see
//!   `default_is_the_empty_state_and_not_a_zeroed_one`), and it must not
//!   continue a session it never saw.
//! * A closed window whose span leaves `i64` publishes no availability: every
//!   window an evaluable bar can close has positive extremes, so its span
//!   always fits.
//!
//! Extreme cases: the second bar of a day after a stale first one; a window of
//! `i64::MAX` over `-1`; a window of one paisa; and (the combination) the
//! overflowing window beside a sound one, which keeps its own availability.

use super::{ORB_FIRST, Orb, RELATIONS_PER_WINDOW, Window};
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

    let mut stale = Orb::new();
    stale.step(&first, fib).expect("a sane bar");
    stale.live = false;
    stale.step(&second, fib).expect("a sane bar");

    let mut alone = Orb::new();
    alone.step(&second, fib).expect("a sane bar");
    assert_eq!(stale, alone, "the stale first bar is not in the session");
}

fn closed(high: i64, low: i64) -> Window {
    Window {
        high,
        low,
        seeded: true,
        closed: true,
    }
}

#[test]
fn a_closed_window_whose_span_leaves_i64_publishes_no_availability() {
    let fib = pinned_fib().expect("pinned");
    let window_positions = |known: vocab::ConditionMask, offset: u16| -> Vec<bool> {
        let base = ORB_FIRST + offset * RELATIONS_PER_WINDOW;
        (base..base + RELATIONS_PER_WINDOW)
            .map(|p| known.get(u32::from(p)))
            .collect()
    };

    let overflowing = Orb {
        windows: [
            closed(i64::MAX, -1),
            closed(2_000_100, 2_000_000),
            Window::new(),
            Window::new(),
        ],
        ..Orb::new()
    };
    let known = overflowing.known(fib);
    assert_eq!(
        window_positions(known, 0),
        [false; 5],
        "the overflowing window"
    );
    assert_eq!(
        window_positions(known, 1),
        [true; 5],
        "the sound window beside it keeps its own availability"
    );

    let narrow = Orb {
        windows: [
            closed(2_000_001, 2_000_000),
            Window::new(),
            Window::new(),
            Window::new(),
        ],
        ..Orb::new()
    };
    assert_eq!(
        window_positions(narrow.known(fib), 0),
        [true; 5],
        "one paisa is a span"
    );
}
