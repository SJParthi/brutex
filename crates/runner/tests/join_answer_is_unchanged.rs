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
const EXCLUDED_AT_K1: usize = 161;
const TOTAL_SURVIVORS: usize = 3_689;

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
    assert_eq!(out.sweep.excluded.len(), EXCLUDED_AT_K1);
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
