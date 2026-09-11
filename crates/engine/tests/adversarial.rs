#![allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes -- a \
              test that cannot panic cannot fail. `expect` and not \
              `unreachable!`, because `unreachable!` expands to a panic inside \
              the crate under test and leaves a coverage region no green run \
              can execute, while `expect` panics inside the standard library \
              and leaves none."
)]

//! THE SWEEP, ATTACKED FROM OUTSIDE THE CRATE, ON THE CROSS PRODUCT OF ITS EXTREMES.
//!
//! # Why this file exists
//!
//! `crates/engine` had **56 unit tests and no integration test at all**, so its
//! public surface had never been driven from outside the crate. A unit test can
//! reach private state and can be written to the implementation it is testing;
//! an integration test sees exactly what `crates/runner` and `crates/cli` see,
//! which is the surface that actually has to hold.
//!
//! # What is attacked, and why a cross product rather than a list
//!
//! Every extreme SHAPE of column crossed with every extreme LADDER setting. The
//! interesting failures in a level-wise search are not at one extreme, they are
//! where two meet — an impossible threshold against an empty column, a budget of
//! one against a column where every candidate is frequent, a threshold exactly
//! equal to the bar count against bars that all hit. A list of single extremes
//! walks past all of those; the product does not.
//!
//! # The invariants asserted on EVERY cell
//!
//! Not "it returns something". Five properties that must hold for any input at
//! all, and each one is a defect if it fails:
//!
//! 1. **It does not panic.** The whole matrix runs; a panic fails the test.
//! 2. **The accounting reconciles.** `Sweep::reconciles` is the engine's own
//!    statement that generated = duplicates + excluded + pruned + infrequent +
//!    frequent at every level. A level that loses a candidate has lost a result.
//! 3. **The verdict is coherent.** `completed()` is exactly `halted.is_none()`.
//!    A sweep cannot be both whole and truncated.
//! 4. **Nothing frequent is below the threshold.** Every kept itemset must hit
//!    at least `min_hits` bars — the one thing the word "frequent" means.
//! 5. **Nothing outside the live set is ever set.** A mask carrying a position
//!    the caller did not offer is the engine inventing a condition.
//!
//! # What this file deliberately does NOT do
//!
//! It asserts no specific depth, count or timing. Those are properties of a
//! fixture, and a test that pinned them here would fail on any legitimate change
//! to the vocabulary while proving nothing about the search. The five above hold
//! for every input this engine can be handed, which is what makes them
//! invariants rather than expectations.

use engine::Ladder;
use engine::column::set_positions;
use vocab::ConditionMask;

/// The positions a caller may legally offer — the vocabulary's own live set.
///
/// Read from `vocab::table::LIVE` rather than written down, so this file cannot
/// drift from the table the way a hand-kept list would.
fn live() -> Vec<u32> {
    set_positions(&vocab::table::LIVE).collect()
}

/// A mask with every live position set. The worst case for a level-wise search:
/// nothing is ever infrequent, so anti-monotonicity has nothing to prune with.
fn full() -> ConditionMask {
    vocab::table::LIVE
}

/// A mask with one live position set, chosen by index into the live set.
fn one(nth: usize) -> ConditionMask {
    let all = live();
    let mut m = ConditionMask::ZERO;
    if let Some(p) = all.get(nth % all.len().max(1))
        && let Ok(idx) = u16::try_from(*p)
    {
        m = vocab::table::set_exact(m, idx).unwrap_or(m);
    }
    m
}

/// Every extreme column shape, named so a failure says which one broke.
fn shapes() -> Vec<(&'static str, Vec<ConditionMask>)> {
    vec![
        ("empty column", vec![]),
        ("one bar, no bits", vec![ConditionMask::ZERO]),
        ("one bar, every live bit", vec![full()]),
        ("eight bars, no bits", vec![ConditionMask::ZERO; 8]),
        // NOT the explosion case, and the comment used to say it was. Every bar
        // carrying every bit makes support == bars, which D-0080 excludes as
        // AlwaysTrue BEFORE k=1: the ladder is empty, depth 0, and every cell
        // built on this shape passes vacuously. Kept because that exclusion is
        // itself worth asserting, and renamed so it cannot be mistaken again.
        (
            "eight bars, every live bit -- ALL always-true, all excluded",
            vec![full(); 8],
        ),
        // THE REAL EXPLOSION. One empty bar drops every position to support
        // bars-1: frequent at min_hits 1, and not always-true, so nothing is
        // excluded and anti-monotonicity has nothing to prune with.
        (
            "eight bars, every live bit except one empty bar",
            [vec![full(); 7], vec![ConditionMask::ZERO]].concat(),
        ),
        (
            "eight bars, one bit each, all different",
            (0..8).map(one).collect(),
        ),
        ("eight bars, one bit each, all the same", vec![one(0); 8]),
        // Half full, half empty: support is exactly half for every position,
        // which is the boundary `min_hits == bars / 2` lands on.
        (
            "four full then four empty",
            [vec![full(); 4], vec![ConditionMask::ZERO; 4]].concat(),
        ),
    ]
}

/// Every extreme ladder setting. `bars` is the column length, so the thresholds
/// that sit exactly ON the column can be built per shape.
fn ladders(bars: u64) -> Vec<(String, Ladder)> {
    let mut out = Vec::new();
    // `with_min_hits(0)` is documented to raise to one; `u64::MAX` is a
    // threshold no column can satisfy. Both must be handled, not guarded against.
    for (name, m) in [
        ("min_hits 0".to_owned(), 0_u64),
        ("min_hits 1".to_owned(), 1),
        ("min_hits 2".to_owned(), 2),
        (
            format!("min_hits bars-1 = {}", bars.saturating_sub(1)),
            bars.saturating_sub(1),
        ),
        (format!("min_hits bars = {bars}"), bars),
        (
            format!("min_hits bars+1 = {}", bars.saturating_add(1)),
            bars.saturating_add(1),
        ),
        ("min_hits u64::MAX".to_owned(), u64::MAX),
    ] {
        // Ceilings and pair budgets at their own extremes, including zero, which
        // `with_ceiling` documents as raised to one.
        for (cname, c) in [
            ("ceiling 0", 0_usize),
            ("ceiling 1", 1),
            ("ceiling 4096", 4096),
        ] {
            for (pname, p) in [("pairs 0", 0_u64), ("pairs 1", 1), ("pairs 1<<20", 1 << 20)] {
                out.push((
                    format!("{name} / {cname} / {pname}"),
                    Ladder::with_min_hits(m).with_ceiling(c).with_pair_budget(p),
                ));
            }
        }
    }
    out
}

/// THE MATRIX. Every shape against every ladder, five invariants on each cell.
#[test]
fn every_extreme_column_against_every_extreme_ladder_holds_the_five_invariants() {
    let live = live();
    assert!(
        !live.is_empty(),
        "the live set is empty, so this whole matrix would pass vacuously"
    );

    let mut cells = 0_u32;
    for (shape_name, bits) in shapes() {
        let bars = u64::try_from(bits.len()).expect("a fixture column fits u64");
        for (ladder_name, ladder) in ladders(bars) {
            let where_ = format!("[{shape_name}] x [{ladder_name}]");
            let applied = ladder.min_hits();
            let sweep = ladder.walk(&bits, &live);
            cells += 1;

            // 2. The accounting reconciles at every level.
            for (n, level) in sweep.levels.iter().enumerate() {
                assert!(
                    level.reconciles(),
                    "{where_}: level {n} accounting does not balance -- a candidate \
                     was generated and is in none of the five outcome buckets"
                );
            }

            // 3. The verdict is coherent: whole XOR truncated, never both.
            assert_eq!(
                sweep.completed(),
                sweep.halted.is_none(),
                "{where_}: a sweep reported itself both whole and halted"
            );

            // 4. Nothing kept is below the threshold the ladder actually applied.
            for item in sweep.all_frequent() {
                assert!(
                    item.hits >= applied,
                    "{where_}: kept an itemset hitting {} bars against a \
                     threshold of {applied} -- \"frequent\" means nothing if this \
                     can happen",
                    item.hits
                );
                // 5. And it names no position the caller did not offer.
                for p in set_positions(&item.mask) {
                    assert!(
                        live.contains(&p),
                        "{where_}: kept a mask carrying position {p}, which is \
                         not in the live set the caller supplied"
                    );
                }
            }

            // A column with no bars cannot yield a frequent set, whatever the
            // threshold: there is nothing for a candidate to hit.
            if bars == 0 {
                assert_eq!(
                    sweep.all_frequent().count(),
                    0,
                    "{where_}: found a frequent combination in a column of no bars"
                );
            }

            // A threshold above the bar count cannot be met by anything, because
            // support is counted against bars and cannot exceed them.
            if applied > bars {
                assert_eq!(
                    sweep.all_frequent().count(),
                    0,
                    "{where_}: {applied} hits were claimed on a {bars}-bar column"
                );
            }
        }
    }
    // The matrix must actually be a matrix. A generator that silently produced
    // nothing would pass every assertion above without testing anything, which
    // is the shape `CLAUDE.md` §4 bans.
    assert_eq!(
        cells, 567,
        "9 shapes x 7 thresholds x 3 ceilings x 3 pair budgets = 567 cells"
    );
}

/// A THRESHOLD OF ZERO IS RAISED TO ONE, AND THAT IS LOAD-BEARING.
///
/// `Ladder::with_min_hits` documents the raise and gives the reason: at
/// `min_hits == 0` every candidate satisfies `hits >= 0`, including one that hits
/// nothing at all — so the frontier can never empty and the walk cannot
/// terminate by extinction, which is the only way `CLAUDE.md` §6 lets it stop.
#[test]
fn a_zero_threshold_is_raised_rather_than_taken_literally() {
    assert_eq!(
        Ladder::with_min_hits(0).min_hits(),
        1,
        "a threshold of zero makes every candidate frequent forever"
    );
    let swept = Ladder::with_min_hits(0)
        .with_ceiling(64)
        .with_pair_budget(1 << 16)
        .walk(&[ConditionMask::ZERO; 4], &live());
    assert_eq!(
        swept.all_frequent().count(),
        0,
        "a column with no bits set has nothing frequent at any threshold -- if \
         zero had been taken literally, every live position would be here"
    );
}

/// THE BUDGETS REFUSE RATHER THAN TRUNCATE SILENTLY.
///
/// The worst case for a level-wise search is a column where every candidate is
/// frequent: anti-monotonicity has nothing to prune with, so the frontier grows
/// at every level instead of shrinking. Against a budget of one it must stop AND
/// SAY SO. A partial ranking returned as a whole answer is the fallback that
/// hides a failure `CLAUDE.md` §4 bans.
#[test]
fn a_column_that_cannot_be_pruned_halts_by_name_rather_than_lying() {
    // One empty bar, so support is bars-1 everywhere: frequent, and NOT
    // always-true, so D-0080 does not excuse the engine from walking it.
    let bits = [vec![full(); 15], vec![ConditionMask::ZERO]].concat();
    let swept = Ladder::with_min_hits(1)
        .with_ceiling(1)
        .with_pair_budget(1)
        .walk(&bits, &live());
    assert!(
        !swept.completed(),
        "a one-candidate budget against a column where everything is frequent \
         cannot have finished the ladder"
    );
    assert!(
        swept.halted.is_some(),
        "and it must name the halt rather than return quietly short"
    );
    for (n, level) in swept.levels.iter().enumerate() {
        assert!(
            level.reconciles(),
            "level {n} accounting must balance even on the level that breached"
        );
    }
}

/// A CONDITION TRUE ON EVERY BAR IS EXCLUDED BEFORE k=1, AND THAT IS WHY THE
/// OBVIOUS "WORST CASE" FIXTURE IS THE MOST TRIVIAL ONE.
///
/// # How this test came to exist
///
/// The matrix above carried a shape called "every live bit on every bar",
/// written as the explosion case: nothing infrequent, so nothing to prune with.
/// It is the opposite. Support then equals the bar count, and D-0080 excludes a
/// position at `support == bars` as **always-true** before the ladder starts —
/// so all 238 live positions are excluded at k=1, the frontier is empty, the
/// walk completes at depth 0, and **every cell built on that shape asserted
/// nothing**. A halting test written against it failed for that reason and not
/// because the engine was wrong.
///
/// The exclusion is correct and worth having: a condition that holds on every
/// bar separates no bars from any other, so it cannot contribute to a
/// combination that means anything. What was wrong was a fixture that mistook
/// "maximum bits set" for "maximum work", and the difference is one empty bar.
#[test]
fn a_position_true_on_every_bar_is_excluded_and_one_empty_bar_undoes_that() {
    let live = live();
    let ladder = || {
        Ladder::with_min_hits(1)
            .with_ceiling(4096)
            .with_pair_budget(1 << 20)
    };

    // Every bar, every bit: support == bars, so every position is always-true.
    let all_true = ladder().walk(&vec![full(); 8], &live);
    let excluded: u64 = all_true.levels.iter().map(|l| l.excluded).sum();
    assert_eq!(
        excluded,
        u64::try_from(live.len()).expect("the live set fits u64"),
        "every live position must be excluded as always-true"
    );
    assert_eq!(
        all_true.all_frequent().count(),
        0,
        "and nothing survives to be combined"
    );
    assert_eq!(all_true.depth(), 0, "so the ladder never leaves the ground");

    // One empty bar drops support to bars-1. Nothing is always-true now, so the
    // same positions are frequent and the walk has real work to do.
    let one_gap = ladder().walk(
        &[vec![full(); 7], vec![ConditionMask::ZERO]].concat(),
        &live,
    );
    assert_eq!(
        one_gap.levels.iter().map(|l| l.excluded).sum::<u64>(),
        0,
        "one bar short of every bar is not always-true, so nothing is excluded"
    );
    assert!(
        one_gap.depth() >= 1,
        "and the ladder must actually climb: one empty bar is the whole \
         difference between the most trivial column and the most expensive one"
    );
}
