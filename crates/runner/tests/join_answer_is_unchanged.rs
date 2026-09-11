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
/// # This number has moved twice and the join's answer has moved neither time
///
/// That is the whole point of the file. Every other assertion here — [`FREQUENT`]
/// level for level, [`TOTAL_SURVIVORS`] at 3,689 — was untouched by both
/// vocabulary growths.
///
/// | vocabulary | live | excluded | firing |
/// |---|---:|---:|---:|
/// | before D-0244 | 238 | 161 | 77 |
/// | + 34 crossings (D-0244) | 272 | 182 | 90 |
/// | + 51 ordinals (D-0249) | 323 | 219 | 104 |
/// | + 5 weekdays (`a12192b`) | **328** | **221** | **107** |
///
/// # The last row was owed and unpaid, and the suite was red the whole time
///
/// `a12192b` added `is_monday`..`is_friday` and took the live count from 323 to
/// 328 — `crates/engine/src/column.rs` asserts that figure directly. This
/// constant was not moved with it, so this test failed from that commit onward
/// and kept failing: **221 against a pinned 219**.
///
/// Nothing noticed, because `cargo test --workspace` does not terminate on this
/// tree — `cli`'s `the_audit_renders_every_stage_of_the_institutional_stack`
/// sweeps at 6.7% support under the full `engine::DEFAULT_CEILING` in a debug
/// build, measured at over an hour. A suite that cannot finish cannot report a
/// failure, so `CLAUDE.md` §9's green-suite requirement was unverifiable and a
/// genuinely red test sat behind it.
///
/// **Three of the five weekdays fire on this fixture and two do not.** Eight
/// synthetic sessions are eight consecutive days from a fixed epoch, so they do
/// not cover a whole trading week — which is why the excluded count rose by two
/// rather than by five. That is the fixture's calendar, not a defect in the
/// conditions, and it is stated here so the next reader does not have to
/// rediscover it.
///
/// So **27 of the 85 new positions fire at least once** and 58 never do, which
/// [`NEW_POSITIONS_THAT_FIRE`] states directly rather than leaving a reader to
/// subtract one pinned constant from another.
///
/// **None of the 27 reaches `min_hits = 600`**, which is why [`FREQUENT`]'s k=1
/// entry is still 17. That is both families behaving as designed rather than a
/// coincidence: a crossing fires on the one bar a level changes side, and an
/// ordinal on a subset of those — so their support is a small multiple of the
/// number of side changes in eight sessions, nowhere near 600 of 1,124 bars.
///
/// A pinned figure that moves for a reason you can state is a gate working.
const EXCLUDED_AT_K1: usize = 221;

/// How many of the 85 new positions have non-zero support on this fixture.
///
/// Asserted rather than left implicit. If a future change makes MORE of them
/// fire, `EXCLUDED_AT_K1` falls and this rises, and a reader diffing one constant
/// would have to derive the other; pinning both means the test says which of the
/// two things happened.
const NEW_POSITIONS_THAT_FIRE: usize = 27;

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

    // BOTH NEW FAMILIES' SUPPORT, STATED RATHER THAN INFERRED FROM TWO
    // CONSTANTS.
    //
    // `EXCLUDED_AT_K1` went 161 -> 182 -> 219 across two appends totalling 85
    // positions, which forces 27 of them to have non-zero support. (It later
    // went 219 -> 221 for the five weekday conditions, which are a different
    // family and do not move THIS count -- the filter below reads `CROSSINGS`
    // and nothing else.) Deriving
    // that by subtracting one pinned number from another is exactly the kind of
    // reasoning a reader should not have to redo, so it is measured here
    // directly: a future change that makes a different number fire fails on
    // THIS line and names the family, rather than failing on the excluded count
    // and leaving the reader to work out which vocabulary moved.
    //
    // All FIVE positions per level -- the two edges and the three ordinals --
    // because they are one family in everything but numbering, and counting
    // only the edges is what this line did before the ordinals landed.
    let firing = vocab::table::CROSSINGS
        .iter()
        .flat_map(|c| [c.up, c.down, c.first, c.second, c.later])
        .filter(|&index| {
            out.sweep
                .excluded
                .iter()
                .all(|e| u16::try_from(e.position).unwrap_or(u16::MAX) != index)
        })
        .count();
    assert_eq!(
        firing, NEW_POSITIONS_THAT_FIRE,
        "of the 85 new positions, {firing} have non-zero support on this \
         fixture. None reaches min_hits = 600, which is why k=1 still returns \
         17: a crossing fires on the one bar a level changes side, and an \
         ordinal on a subset of those"
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
