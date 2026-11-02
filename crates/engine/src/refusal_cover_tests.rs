//! BASE-COV-LIBS (D-7329): every memory refusal of the walk, reached by the
//! real path it takes. Exhausting this machine's allocator is no test, so a
//! test-only arm of [`crate::reserved`] asks for `usize::MAX` on the n-th call
//! of this thread: `try_reserve_exact` then answers the genuine
//! `TryReserveError`, and everything after it is shipping code.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use crate::column::Column;
use crate::{BATCH_PER_LANE, Breach, Ladder, highest_position};
use std::cell::Cell;
use vocab::ConditionMask;

std::thread_local! {
    /// How many more calls of `reserved` succeed before one is refused.
    static LEFT: Cell<Option<u32>> = const { Cell::new(None) };
}

/// Whether this call of `reserved` is the one to refuse.
pub(crate) fn this_reservation_is_refused() -> bool {
    LEFT.with(|left| match left.get() {
        Some(0) => {
            left.set(None);
            true
        }
        Some(n) => {
            left.set(Some(n - 1));
            false
        }
        None => false,
    })
}

/// Runs `body` with the `n`-th reservation (counted from 0) of this thread
/// refused; answers whether that one came.
fn refusing<T>(n: u32, body: impl FnOnce() -> T) -> (T, bool) {
    LEFT.with(|left| left.set(Some(n)));
    let out = body();
    let fired = LEFT.with(Cell::take).is_none();
    (out, fired)
}

/// A row of every live position and an empty one: every pair is frequent at
/// one hit, so a join passes a whole batch to `drain`.
fn every_live() -> (Vec<ConditionMask>, Vec<u32>) {
    let live: Vec<u32> = (0..ConditionMask::BITS)
        .filter(|&position| u16::try_from(position).is_ok_and(vocab::table::is_live))
        .collect();
    let full = live
        .iter()
        .copied()
        .fold(ConditionMask::ZERO, ConditionMask::with_bit);
    (vec![full, ConditionMask::ZERO], live)
}

fn ladder() -> Ladder {
    Ladder::with_min_hits(1)
        .with_ceiling(BATCH_PER_LANE * 2 + 1)
        .with_support_lanes(1)
}

const NO_REPORT: &dyn Fn(&crate::Frontier, usize, u64) = &|_, _, _| {};

/// **THE RETAINED ENTRY POINTS**: the column's copy refused, and the level
/// table's: each is a Memory halt at depth 0, naming the bars it was given.
#[test]
fn the_retained_walk_refuses_its_column_and_its_levels_by_name() {
    let (rows, live) = every_live();
    let (sweep, fired) = refusing(0, || ladder().walk_reporting(&rows, &live, NO_REPORT));
    assert!(fired);
    let halt = sweep.halted.expect("the refusal halts the walk");
    assert_eq!((halt.k, halt.breach), (0, Breach::Memory));
    assert_eq!(sweep.bars, 2);

    let column = Column::try_from_rows(&rows).unwrap();
    let (sweep, fired) = refusing(0, || ladder().walk_column(&column, &live, NO_REPORT));
    assert!(fired);
    assert_eq!(sweep.halted.unwrap().breach, Breach::Memory);
    assert_eq!(sweep.bars, 2);
}

/// **THE STREAMED ENTRY POINTS**, each reservation of each one.
#[test]
fn the_streamed_walks_refuse_their_column_and_their_levels_by_name() {
    let (rows, live) = every_live();
    let column = Column::try_from_rows(&rows).unwrap();
    let (out, fired) = refusing(0, || {
        ladder().walk_column_streamed(&column, &live, &mut |_, _, _| {})
    });
    assert!(fired);
    assert_eq!(out.halted.unwrap().breach, Breach::Memory);

    let (out, fired) = refusing(0, || {
        ladder().walk_streamed(&rows, &live, &mut |_, _, _| {})
    });
    assert!(fired);
    assert_eq!(out.halted.unwrap().breach, Breach::Memory);
    assert_eq!(out.bars, 2);

    for n in [0, 1] {
        let (out, fired) = refusing(n, || {
            ladder().walk_streamed_with_retirement(
                &rows,
                &live,
                &mut |_, _, _| panic!("a refused walk reports no level"),
                &mut |_, _| panic!("a refused walk retires no level"),
            )
        });
        assert!(fired, "reservation {n}");
        let halted = out.halted.unwrap();
        assert_eq!((halted.k, halted.breach), (0, Breach::Memory), "{n}");
        assert_eq!(out.bars, 2, "{n}");
    }
}

/// **A REFUSAL ANYWHERE IN THE WALK IS A HALT, NEVER A SHORT ANSWER.** The
/// walk is run once with each of its reservations refused in turn, the
/// candidates of a batch included, so both `drain` calls are reached with its
/// count vector refused: whichever one it is, the walk says Memory, and every
/// level it reports reconciles (no candidate is neither frequent nor
/// infrequent). The first `n` past the last reservation is the walk whole.
#[test]
fn every_reservation_of_a_real_walk_refused_in_turn_is_a_memory_halt() {
    let (rows, live) = every_live();
    let column = Column::try_from_rows(&rows).unwrap();
    let whole = ladder().walk_column(&column, &live, NO_REPORT);
    let mut refused = 0_u32;
    for n in 0..256 {
        let (sweep, fired) = refusing(n, || ladder().walk_column(&column, &live, NO_REPORT));
        if !fired {
            assert_eq!(
                sweep.halted.map(|h| h.breach),
                whole.halted.map(|h| h.breach)
            );
            assert_eq!(sweep.levels.len(), whole.levels.len());
            assert!(refused >= 6, "only {refused} reservations were reached");
            return;
        }
        refused += 1;
        let halt = sweep.halted.unwrap_or_else(|| panic!("{n}: no halt"));
        assert_eq!(halt.breach, Breach::Memory, "{n}");
        for level in &sweep.levels {
            assert!(level.reconciles(), "{n}: level {} lost candidates", level.k);
        }
    }
    panic!("a walk of two levels took more than 256 reservations");
}

/// COMBINED: a refusal with no live positions and no rows at all. The empty
/// walk reserves too, and a refusal of it is still a halt, not an empty
/// success.
#[test]
fn an_empty_walk_refused_is_a_halt_and_not_an_empty_success() {
    let (sweep, fired) = refusing(0, || ladder().walk_reporting(&[], &[], NO_REPORT));
    assert!(fired);
    assert_eq!(sweep.halted.unwrap().breach, Breach::Memory);
    assert_eq!(sweep.bars, 0);
}

/// The empty mask has no highest position; each single bit is its own.
#[test]
fn the_highest_position_of_nothing_is_none_at_both_ends_of_the_mask() {
    assert_eq!(highest_position(&ConditionMask::ZERO), None);
    for position in [0, 63, 64, ConditionMask::BITS - 1] {
        let one = ConditionMask::ZERO.with_bit(position);
        assert_eq!(highest_position(&one), Some(position));
    }
}
