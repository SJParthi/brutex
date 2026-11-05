//! BASE-COV-LIBS-LSA (D-8569): the reservations of this crate that only a
//! refusing allocator fails, reached by the real path they take.
//!
//! Exhausting the host's allocator is no test, and a test allocator needs the
//! unsafe code this workspace denies. So a test-only line beside each of these
//! reservations asks [`asked`] for its size: on the n-th seamed reservation of
//! the thread, armed by [`refusing`], the answer is `usize::MAX`, so
//! `try_reserve_exact` answers the genuine `TryReserveError` (a capacity no
//! allocator holds) and everything after it is shipping code. Nothing of it is
//! compiled into a build. It is the door `engine::reserved` keeps for the walk
//! (D-7329), and the seams of D-6017 and D-8561 keep for the store's host
//! refusals.
//!
//! The seamed reservations, in the order one resolution, view, replay or
//! preparation meets them:
//!
//! * `exit_grid_policy::observed_ranges`: the observed stops, targets and
//!   trails, one reservation each;
//! * `exit_grid_policy::admitted_ratio_pairs`: the admitted ratio pairs;
//! * `resolved_grid_view::ResolvedGridViewV1::ladders`: the copy of each
//!   exact expression ladder (stops, targets, trails);
//! * `resolved_grid_view::ResolvedGridViewV1::coordinate_row_offsets`;
//! * `grid::replay_universe_v1`: the replay candidates;
//! * `grid::replay_universe_from_candidates_v1`: the evidence rows, then the
//!   one-row scratch;
//! * `signal_candle_stop::days_before`: the per-day prefix table (the rows'
//!   call, then the periods').

use std::cell::Cell;

std::thread_local! {
    /// How many more seamed reservations of this thread succeed before one is
    /// refused; `None` when no test has armed one.
    static LEFT: Cell<Option<u32>> = const { Cell::new(None) };
}

/// The size a seamed reservation asks for: `elements`, or `usize::MAX` for
/// the one reservation a test armed with [`refusing`].
pub(crate) fn asked(elements: usize) -> usize {
    let refused = LEFT.with(|left| match left.get() {
        Some(0) => {
            left.set(None);
            true
        }
        Some(n) => {
            left.set(Some(n - 1));
            false
        }
        None => false,
    });
    if refused { usize::MAX } else { elements }
}

/// Runs `body` with the `n`-th seamed reservation (counted from 0) of this
/// thread refused, and answers whether that reservation came.
pub(crate) fn refusing<T>(n: u32, body: impl FnOnce() -> T) -> (T, bool) {
    LEFT.with(|left| left.set(Some(n)));
    let out = body();
    let fired = LEFT.with(Cell::take).is_none();
    (out, fired)
}

/// THE DOOR ITSELF: unarmed it asks for what it is given; armed at `n` it
/// lets `n` reservations through, refuses the next one alone and is spent;
/// armed past the reservations a body makes, it reports that none came and
/// leaves nothing armed for the next test on the thread.
///
/// Extreme cases: no arming at all; the first reservation refused; the third;
/// sizes of zero and `usize::MAX` (the refused answer cannot be told from a
/// real request of that size, so the test reads the arming, not the size);
/// combinations: an arming the body never reaches followed by an unarmed
/// call, and a spent arming followed by more reservations; out of the box, an
/// arming at `u32::MAX` with a body that reserves nothing.
#[test]
fn the_armed_reservation_alone_is_refused_and_the_arming_is_spent() {
    assert_eq!(asked(7), 7, "unarmed: the size asked");
    assert_eq!(asked(0), 0);

    let (sizes, fired) = refusing(0, || [asked(3), asked(4)]);
    assert!(fired);
    assert_eq!(sizes, [usize::MAX, 4], "the first refused, the next asked");

    let (sizes, fired) = refusing(2, || [asked(1), asked(2), asked(0), asked(5)]);
    assert!(fired);
    assert_eq!(sizes, [1, 2, usize::MAX, 5], "only the third");

    let (sizes, fired) = refusing(3, || [asked(1), asked(2)]);
    assert!(!fired, "two reservations never reach the fourth");
    assert_eq!(sizes, [1, 2]);
    assert_eq!(asked(9), 9, "and the arming did not outlive its body");

    let ((), fired) = refusing(u32::MAX, || ());
    assert!(!fired);
    assert_eq!(asked(usize::MAX), usize::MAX, "a real request of any size");
}
