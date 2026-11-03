#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail; `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind"
)]
//! BASE-COV-LIBS-LS3 (attempt 5): the three sides of the gap ladder no test
//! above reached, each driven by the leg or the band that reaches it.
//!
//! * The truth emission handed the PIVOT band: `Tolerance::covers` sees two
//!   spans and cannot tell the families apart, so it covers a close sitting
//!   on a rung; `vocab::table::set_near` knows the rung is a Fibonacci one and
//!   refuses the band, and the rung stays unset.
//! * A leg whose length leaves `i64`: availability stays empty.
//! * A leg of no length: no band can cover anything on it, so availability
//!   stays empty.
//!
//! Prices are paisa of invented levels. Extreme cases: a close exactly on a
//! rung, a leg spanning `-1..i64::MAX`, a leg of zero length up and down, a
//! one-paisa leg; combinations: the pivot band on a sound leg with the close
//! on the midpoint, and an overflowing leg asked for both truth and
//! availability.

use super::{Direction, GapFib, GapLeg};
use vocab::ConditionMask;
use vocab::tolerance::{pinned_fib, pinned_pivot};

fn with_leg(x1: i64, x2: i64, direction: Direction) -> GapFib {
    GapFib {
        leg: Some(GapLeg { x1, x2, direction }),
        ..GapFib::new()
    }
}

#[test]
fn a_rung_handed_the_pivot_band_is_refused_by_the_vocabulary_and_stays_unset() {
    let fib = pinned_fib().expect("the Fibonacci band is pinned");
    let pivot = pinned_pivot().expect("the pivot band is pinned");
    let gap = with_leg(2_000_000, 2_100_000, Direction::Up);
    let leg = gap.leg.expect("a leg");
    let mid = leg.midpoint().expect("a midpoint");
    let range = leg.length().expect("a length");
    assert!(
        pivot.covers(mid, mid, range),
        "the band itself covers the rung"
    );

    assert_ne!(
        gap.bits(mid, fib),
        ConditionMask::ZERO,
        "the right band sets the rung"
    );
    assert_eq!(
        gap.bits(mid, pivot),
        ConditionMask::ZERO,
        "the wrong family's band is refused at every rung"
    );
}

#[test]
fn a_leg_whose_length_leaves_i64_is_unavailable_and_sets_nothing() {
    let fib = pinned_fib().expect("pinned");
    let wide = with_leg(-1, i64::MAX, Direction::Up);
    assert_eq!(
        wide.leg.and_then(|l| l.length()),
        None,
        "the length leaves i64"
    );
    assert_eq!(wide.known(fib), ConditionMask::ZERO);
    assert_eq!(wide.bits(0, fib), ConditionMask::ZERO);
}

#[test]
fn a_leg_of_no_length_is_unavailable_at_every_rung() {
    let fib = pinned_fib().expect("pinned");
    for direction in [Direction::Up, Direction::Down] {
        let flat = with_leg(2_000_000, 2_000_000, direction);
        assert_eq!(flat.leg.and_then(|l| l.length()), Some(0));
        assert_eq!(flat.known(fib), ConditionMask::ZERO, "{direction:?}");
    }
    // The sound half: one paisa of leg is a span, and every rung on it is
    // available.
    let narrow = with_leg(2_000_000, 2_000_001, Direction::Up);
    assert_eq!(
        narrow.known(fib).popcount(),
        u32::try_from(GapFib::positions().len()).expect("eleven rungs")
    );
}
