//! BASE-COV-LIBS-LS4: `engine::primitives::JoinProbe::screen_every_pair`
//! counts a pair the production screen refuses as a pair and never as a
//! survivor.
//!
//! The bench (C-E-08) only ever offers frontiers whose every k=3 union has
//! every 2-subset frequent, so the screen's refusing side never ran: a pair
//! whose union needs a subset the frontier does not hold. That side is the
//! Apriori subset prune itself (`CLAUDE.md` section 6), so it is pinned here.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use engine::Itemset;
use engine::primitives::JoinProbe;
use vocab::ConditionMask;

/// The frequent 2-set `{a, b}`, seen on one bar.
fn pair(a: u32, b: u32) -> Itemset {
    Itemset {
        mask: ConditionMask::default().with_bit(a).with_bit(b),
        hits: 1,
    }
}

fn screened(frontier: &[Itemset]) -> (u64, u64) {
    JoinProbe::try_new(frontier)
        .expect("a small frontier indexes")
        .screen_every_pair()
}

/// **A PAIR WHOSE UNION HAS AN INFREQUENT SUBSET IS COUNTED AND NOT KEPT.**
/// `{0,1}` and `{0,2}` share the prefix `{0}`; their union `{0,1,2}` needs
/// `{1,2}`.
#[test]
fn a_pair_whose_union_lacks_a_frequent_subset_is_counted_and_refused() {
    // 1. The missing subset: one pair, no survivor.
    assert_eq!(screened(&[pair(0, 1), pair(0, 2)]), (1, 0));
    // 2. CONTROL: with `{1,2}` frequent the same pair survives, and `{1,2}`
    //    is alone in its prefix block, so it adds no pair.
    assert_eq!(screened(&[pair(0, 1), pair(0, 2), pair(1, 2)]), (1, 1));
    // 3. Order does not matter: the index keys every mask by its prefix.
    assert_eq!(screened(&[pair(1, 2), pair(0, 2), pair(0, 1)]), (1, 1));
    assert_eq!(screened(&[pair(0, 2), pair(0, 1)]), (1, 0));
    // 4. No pair at all: one mask, and the empty frontier.
    assert_eq!(screened(&[pair(0, 1)]), (0, 0));
    assert_eq!(screened(&[]), (0, 0));
}

/// COMBINED: one prefix block of three masks where every union lacks a
/// subset, beside one where every union has them all. Each refused pair is
/// still a pair; only the whole block survives.
#[test]
fn refused_and_kept_pairs_in_one_frontier_are_each_counted_once() {
    // 5. Block {0}: {0,1} {0,2} {0,3}, and no {1,2} {1,3} {2,3}: three pairs,
    //    none kept.
    let refused = [pair(0, 1), pair(0, 2), pair(0, 3)];
    assert_eq!(screened(&refused), (3, 0));
    // 6. COMBINED: the same block with every 2-subset of {1,2,3} frequent:
    //    the three pairs of block {0} survive, and block {1} ({1,2} {1,3})
    //    adds a fourth pair whose union {1,2,3} needs {2,3}, also present.
    let mut whole = refused.to_vec();
    whole.extend([pair(1, 2), pair(1, 3), pair(2, 3)]);
    assert_eq!(screened(&whole), (4, 4));
    // 7. COMBINED: drop {2,3} alone. Every union that needs it is refused:
    //    {0,2,3} from block {0} and {1,2,3} from block {1}.
    whole.retain(|item| item.mask != pair(2, 3).mask);
    assert_eq!(screened(&whole), (4, 2));
    // 8. OUT OF THE BOX: positions at the far end of the mask, past the
    //    first word, prune the same way.
    let last = ConditionMask::BITS - 1;
    let far = [pair(0, last - 1), pair(0, last)];
    assert_eq!(screened(&far), (1, 0));
}
