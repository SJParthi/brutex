//! The membership probe refuses an over-long argument before hashing it.
//!
//! # Why this is a test and not a bench
//!
//! `universe::of_equity` is public, takes an unguarded `&str`, and hashes it
//! once per table — TWICE when this was written, SIX times since D-0089 added
//! the four published NIFTY tiers, which is a further reason the guard matters
//! and not a reason to revisit the ceiling: a longer argument is refused
//! before ANY of the six hashes, so the short side of the ratio grew three
//! times over and the long side did not move. `fnv1a` walks the whole argument
//! with no bound of its own, so the cost of a membership probe used to be the
//! CALLER's string length rather than the table's size. Measured against gate
//! 8's 3.0x ceiling before the guard existed:
//!
//! | input | per call | ratio |
//! |---|---|---|
//! | 8 B | 8,256 ps | baseline |
//! | 24 B | 14,333 ps | 1.736x |
//! | 1 KiB | 1,806,225 ps | **218.777x** |
//! | 4 MiB | 7,663,937,500 ps | **928,287x** — 7.66 ms in ONE call |
//!
//! The per-byte slope agreed across the last two (882 ps/byte at 1 KiB,
//! 914 ps/byte at 4 MiB), a clean linear fit over a 4,096x span of input.
//!
//! A bench would have caught the ratio; only a test states the INVARIANT,
//! which is what the guard actually rests on: every member of every table is
//! a `Symbol`, and `Symbol::new` refuses anything past `SYMBOL_CAPACITY`, so
//! a longer string has no member it could equal. Answering `false` without
//! hashing is the same answer arrived at sooner.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::hint::black_box;
use std::time::Instant;

/// A 4 MiB argument must not cost more than a short one.
///
/// Asserted as a RATIO of integer nanoseconds, never a float: `CLAUDE.md`
/// section 7 bans the float for prices and the workspace lint is stricter and
/// bans the cast everywhere, so a comparison that is exact integer arithmetic
/// stays exact integer arithmetic. The bound is deliberately loose — 30x — so
/// this cannot flake on a loaded machine while still being 30,000x tighter
/// than the 928,287x that was measured before the guard.
#[test]
fn an_over_long_argument_is_refused_before_it_is_hashed() {
    const ROUNDS: u32 = 100;
    const CEILING: u128 = 30;

    let long = "A".repeat(4 * 1024 * 1024);
    let short = "RELIANCE";

    // The short probe first, so a cold cache penalises the SHORT side — which
    // makes the assertion harder to pass, not easier.
    let at = Instant::now();
    for _ in 0..ROUNDS {
        black_box(brutex_core::universe::of_equity(black_box(short)));
    }
    let small = at.elapsed().as_nanos().max(1);

    let at = Instant::now();
    for _ in 0..ROUNDS {
        black_box(brutex_core::universe::of_equity(black_box(&long)));
    }
    let big = at.elapsed().as_nanos().max(1);

    assert!(
        big <= small.saturating_mul(CEILING),
        "a 4 MiB probe took {big} ns against {small} ns for 8 bytes — more \
         than {CEILING}x, so the argument is being hashed rather than refused \
         at the length check, and `of_equity` is O(n) in its caller's string"
    );
}

/// And the answer is still correct, which is the point the cost rests on.
#[test]
fn the_guard_does_not_change_any_answer() {
    let member = brutex_core::universe::of_equity("RELIANCE");
    assert_ne!(
        member,
        brutex_core::universe::Universe::NONE,
        "RELIANCE is in the NIFTY Total Market and must still be found"
    );

    // Longer than SYMBOL_CAPACITY, so no `Symbol` can equal it and NONE is the
    // only correct answer — the guard returns it sooner, not differently.
    let too_long = "A".repeat(brutex_core::symbol::SYMBOL_CAPACITY + 1);
    assert_eq!(
        brutex_core::universe::of_equity(&too_long),
        brutex_core::universe::Universe::NONE,
        "a string too long to be a Symbol has no member it could equal"
    );

    // At the boundary exactly, an argument of SYMBOL_CAPACITY bytes is a legal symbol
    // shape and has to be probed rather than short-circuited.
    let at_cap = "A".repeat(brutex_core::symbol::SYMBOL_CAPACITY);
    assert_eq!(
        brutex_core::universe::of_equity(&at_cap),
        brutex_core::universe::Universe::NONE,
        "not a member, but it must have been probed rather than short-circuited"
    );

    // THE COMMENT ABOVE USED TO CLAIM MORE THAN IT COULD SHOW, and an audit was right to
    // flag it: it said "an off-by-one here would silently stop finding the longest real
    // symbols". Measured, that is false. `SYMBOL_CAPACITY` is 24 and the longest member of
    // the NIFTY Total Market is **10 bytes** — `AARTIDRUGS`, `ABBOTINDIA` and three more.
    // So `>` becoming `>=` rejects only a 24-byte argument, and no 24-byte argument is
    // ever a member: the assertion above expects `NONE`, which is also exactly what a
    // fired guard returns. The off-by-one is invisible through `of_equity`, and no
    // contortion of this test can make it visible.
    //
    // What IS worth asserting is the property that makes the off-by-one harmless, because
    // that property is not a law of nature. If `SYMBOL_CAPACITY` were ever lowered toward
    // the real maximum, a `>=` would begin losing members — and this is the assertion that
    // would notice.
    let longest = brutex_core::universe::NIFTY_TOTAL_MARKET
        .iter()
        .map(|s| s.len())
        .max()
        .unwrap_or(0);
    assert!(
        longest > 0,
        "no member was measured, so the bound below is compared against nothing"
    );
    assert!(
        longest < brutex_core::symbol::SYMBOL_CAPACITY,
        "the longest member is {longest} bytes and SYMBOL_CAPACITY is {}. Once a real \
         symbol reaches the cap, an off-by-one in the length guard starts losing members \
         instead of being harmless — and the boundary case above cannot see that, because \
         a fired guard and a genuine miss both answer NONE.",
        brutex_core::symbol::SYMBOL_CAPACITY
    );
}
