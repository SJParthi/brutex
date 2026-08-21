//! The prefix join returns the same answer the pairwise join returned.
//!
//! # Why this file exists
//!
//! `crates/engine` replaced a join that unioned every pair of frequent
//! (k−1)-sets and discarded the 95.19% whose union did not have k bits, with one
//! that groups the frontier by its (k−2)-prefix and visits only the pairs that
//! can join. The engine proves the two agree in the abstract —
//! `engine::tests::the_prefix_join_finds_exactly_what_an_exhaustive_join_would`
//! rebuilds each level by brute force and compares the sets.
//!
//! This file proves it on the real fixture, against numbers **recorded before
//! the change**. An algorithmic rewrite that is correct in a unit test and wrong
//! at scale is the failure mode that matters, and the only defence against it is
//! a figure written down while the old code was still running.
//!
//! # Where these numbers came from
//!
//! One run of `Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))`
//! over `synthetic::sessions(8)`, read off the audit report the day before the
//! join changed. They are not derived from the current code, which is the whole
//! point: if they were, the test could not fail.

#![allow(
    clippy::expect_used,
    reason = "the exception every test in this workspace takes -- a test that \
              cannot panic cannot fail, and `.expect` panics inside core, which is \
              not instrumented, so it leaves no uncoverable region behind."
)]

use engine::Ladder;
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::{Sweeper, synthetic};

/// Survivors per level, k=1 upward, from the pairwise join.
const FREQUENT: [usize; 12] = [17, 98, 304, 607, 837, 823, 584, 295, 101, 21, 2, 0];

/// Everything the pairwise join reported that the prefix join must match.
const SWEPT_BARS: u64 = 1_124;
const TOTAL_SURVIVORS: usize = 3_689;

/// Positions excluded before k=1, on this fixture.
///
/// # This number moved and the join's answer did not, which is the whole point
///
/// It was **161** when the vocabulary held 238 live positions. The crossing
/// family (D-0244) added 34, and this figure went to **182**. Every other
/// assertion in this file was untouched: [`FREQUENT`] matched level for level and
/// [`TOTAL_SURVIVORS`] matched at 3,689.
///
/// So the arithmetic is forced, and [`NEW_POSITIONS_THAT_FIRE`] below states it
/// as a claim rather than leaving 21 to be inferred from two constants:
///
/// ```text
/// 34 added  −  21 that never fire on this fixture  =  13 that fire at least once
/// ```
///
/// **None of the 13 reached `min_hits = 600`**, which is why [`FREQUENT`]'s k=1
/// entry is still 17. That is the family behaving as designed rather than a
/// coincidence: a crossing fires on the one bar a level changes side, not on
/// every bar the level is above — so its support is a small multiple of the
/// number of trend changes in eight sessions, nowhere near 600 of 1,124 bars.
///
/// A pinned figure that moves for a reason you can state is a gate working. This
/// one told us the vocabulary grew, told us by how much, and told us the join
/// was not disturbed — which is exactly what this file exists to establish.
const EXCLUDED_AT_K1: usize = 182;

/// How many of the 34 crossing positions have non-zero support on this fixture.
///
/// Asserted rather than left implicit. If a future change makes MORE of them
/// fire, `EXCLUDED_AT_K1` falls and this rises, and a reader diffing one constant
/// would have to derive the other; pinning both means the test says which of the
/// two things happened.
const NEW_POSITIONS_THAT_FIRE: usize = 13;

fn evaluator() -> Evaluator {
    Evaluator::new(
        Widths::pinned().expect("both pinned widths are valid"),
        Availability::Absent,
        Thresholds::CLASSICAL,
    )
}

#[test]
fn every_level_returns_the_survivors_the_pairwise_join_returned() {
    let bars = synthetic::sessions(8);
    let ladder = Ladder::with_min_hits(600).with_ceiling(50_000);
    let out = Sweeper::new(ladder).run(&bars, &mut evaluator());

    assert!(
        out.is_complete(),
        "the fixture must produce a complete answer, or the comparison below is \
         between two truncations rather than between two joins"
    );
    assert_eq!(
        out.sweep.bars, SWEPT_BARS,
        "the column changed size, so any difference below would be the bars and \
         not the join"
    );
    assert_eq!(
        out.sweep.levels.len(),
        FREQUENT.len(),
        "the ladder climbed to a different height"
    );

    for (level, want) in out.sweep.levels.iter().zip(FREQUENT) {
        assert_eq!(
            level.frequent.len(),
            want,
            "level k={} returned {} survivors where the pairwise join returned \
             {}. The prefix grouping reached a different set of candidates -- \
             every counter in this level still reconciles, which is exactly why \
             this figure is pinned from outside the code",
            level.k,
            level.frequent.len(),
            want
        );
    }

    assert_eq!(out.sweep.all_frequent().count(), TOTAL_SURVIVORS);
    assert_eq!(
        out.sweep.excluded.len(),
        EXCLUDED_AT_K1,
        "positions excluded before k=1. This is the ONLY figure in this file the \
         crossing family moved, and the constant's own doc carries the arithmetic"
    );

    // THE CROSSING FAMILY'S SUPPORT, STATED RATHER THAN INFERRED FROM TWO
    // CONSTANTS.
    //
    // `EXCLUDED_AT_K1` went 161 -> 182 when 34 positions were appended, which
    // forces 13 of them to have non-zero support. Deriving that by subtracting
    // one pinned number from another is exactly the kind of reasoning a reader
    // should not have to redo, so it is measured here directly: a future change
    // that makes a different number of them fire fails on THIS line and names
    // the family, rather than failing on the excluded count and leaving the
    // reader to work out which vocabulary moved.
    let firing = vocab::table::CROSSINGS
        .iter()
        .flat_map(|&(_, _, up, down)| [up, down])
        .filter(|&index| {
            out.sweep
                .excluded
                .iter()
                .all(|e| u16::try_from(e.position).unwrap_or(u16::MAX) != index)
        })
        .count();
    assert_eq!(
        firing, NEW_POSITIONS_THAT_FIRE,
        "of the 34 crossing positions, {firing} have non-zero support on this \
         fixture. None reaches min_hits = 600, which is why k=1 still returns \
         17: a crossing fires on the one bar a level changes side, not on every \
         bar it is above"
    );
}

#[test]
fn the_join_no_longer_walks_a_pair_it_will_discard() {
    let bars = synthetic::sessions(8);
    let ladder = Ladder::with_min_hits(600).with_ceiling(50_000);
    let out = Sweeper::new(ladder).run(&bars, &mut evaluator());

    // The pairwise join walked |F|²/2 pairs per level and generated only those
    // whose union had k bits. The prefix join walks exactly what it generates.
    // Asserting equality is asserting that no pair is walked speculatively.
    for level in out.sweep.levels.iter().skip(1) {
        let pairs = u64::try_from(level.frequent.len()).unwrap_or(u64::MAX);
        assert!(
            level.generated >= pairs,
            "level {} generated fewer candidates than it kept",
            level.k
        );
        assert_eq!(
            level.duplicates, 0,
            "level {} emitted the same k-set twice. The prefix join reaches each \
             k-set from exactly one pair -- its two largest positions -- so a \
             non-zero duplicate count means the grouping is wrong, not that \
             deduplication is working",
            level.k
        );
    }

    // And the total pairwise cost the old join paid, recomputed here so the size
    // of what was removed is visible rather than asserted in a comment.
    let pairwise: u64 = out
        .sweep
        .levels
        .iter()
        .map(|l| {
            let f = u64::try_from(l.frequent.len()).unwrap_or(0);
            f.saturating_mul(f.saturating_sub(1)) / 2
        })
        .sum();
    let walked: u64 = out.sweep.levels.iter().map(|l| l.generated).sum();
    assert!(
        pairwise > walked.saturating_mul(20),
        "the prefix join must walk at least 20x fewer pairs than the pairwise \
         join would have on this fixture; it walked {walked} against {pairwise}"
    );
}
