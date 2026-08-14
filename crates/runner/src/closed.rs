//! The combinations that carry information nothing larger already carries.
//!
//! # The redundancy the sweep produces by construction
//!
//! If `{A,B}` fires on exactly 500 bars and `{A,B,C}` fires on **exactly the
//! same 500**, then `{A,B}` tells a reader nothing `{A,B,C}` does not — every
//! bar it fires on, the larger one fires on too, with a further condition
//! attached for free. `{A,B}` is not **closed**.
//!
//! Apriori emits both, and it must: `{A,B}` is needed to generate `{A,B,C}`.
//! But keeping both in the ANSWER is duplication, and a sweep at low support
//! produces an enormous amount of it — the frequent-itemset literature is
//! explicit that a frequent set of size k implies `2^k − 2` other frequent
//! subsets, which is why the FIMI benchmark's binding constraint was the size of
//! its output file rather than the speed of its search.
//!
//! # Lossless, which is the whole reason to prefer it
//!
//! The closed sets are a **complete** summary: every frequent itemset and its
//! exact support can be recovered from them, because a non-closed set's support
//! is by definition the support of its smallest closed superset. So this is
//! compression with nothing thrown away — unlike a top-N cut, which is a
//! deliberate choice to look at less.
//!
//! [`crate::rank`] bounds what is RETAINED; this bounds what is REDUNDANT. They
//! compose: the closed set is smaller, and the top of it is smaller still.
//!
//! # Immediate supersets are enough, and that is a theorem rather than a hope
//!
//! A set `X` is closed when no superset shares its support. Checking only the
//! supersets one bit larger is sufficient: support is anti-monotone, so for any
//! `X ⊂ Y ⊂ Z`, `sup(X) ≥ sup(Y) ≥ sup(Z)`. If some larger `Z` had
//! `sup(Z) = sup(X)`, every `Y` between them is squeezed to the same value —
//! including one exactly one bit above `X`. So a violation, if it exists, is
//! always visible one level up.
//!
//! And that superset is always PRESENT to be seen: if `sup(Y) = sup(X)` and `X`
//! cleared `min_hits`, then `Y` cleared it too, so the sweep kept `Y`.

use std::collections::{HashMap, HashSet};

use engine::column::set_positions;
use engine::{Itemset, Sweep};
use vocab::ConditionMask;

/// What survived the redundancy check, and what it cost to find out.
#[derive(Clone, Debug, Default)]
pub struct Closed {
    /// The closed frequent itemsets, in the sweep's own canonical order.
    pub kept: Vec<Itemset>,
    /// Every frequent itemset the sweep produced.
    ///
    /// Reported beside `kept` because a compression ratio is the only way to
    /// read whether this was worth doing on a given column, and `kept.len()`
    /// alone cannot say.
    pub considered: u64,
}

impl Closed {
    /// How many frequent itemsets were redundant.
    #[must_use]
    pub fn redundant(&self) -> u64 {
        let kept = u64::try_from(self.kept.len()).unwrap_or(u64::MAX);
        self.considered.saturating_sub(kept)
    }
}

/// How many frequent itemsets are redundant, WITHOUT building the kept list.
///
/// [`crate::significance::effective_trials`] needs only this count, and calling
/// [`closed`] for it allocated a whole `Vec<Itemset>` of the answer and dropped
/// it. An audit measured the render that does so at 978,542 ns on the twelve-
/// level fixture, effectively all of it here.
///
/// The walk is the same and so is the cost in lookups; what is gone is the
/// allocation of a result nobody reads.
#[must_use]
pub fn redundant_count(sweep: &Sweep) -> u64 {
    let mut redundant: HashSet<ConditionMask> = HashSet::new();
    for (lower, upper) in sweep.levels.iter().zip(sweep.levels.iter().skip(1)) {
        let below: HashMap<ConditionMask, u64> =
            lower.frequent.iter().map(|i| (i.mask, i.hits)).collect();
        for larger in &upper.frequent {
            for bit in set_positions(&larger.mask) {
                let smaller = larger.mask.without_bit(bit);
                if below.get(&smaller) == Some(&larger.hits) {
                    redundant.insert(smaller);
                }
            }
        }
    }
    u64::try_from(redundant.len()).unwrap_or(u64::MAX)
}

/// The closed frequent itemsets of a sweep.
///
/// # Cost
///
/// One pass per level building a `mask → hits` map, then for each itemset at
/// level `k+1` one lookup per set bit. That is `O(Σ |F_k| · k)` lookups against
/// `O(Σ |F_k|)` insertions — linear in the answer's size times the depth, and
/// never quadratic in the frontier the way a pairwise superset search would be.
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn closed(sweep: &Sweep) -> Closed {
    let considered = sweep.levels.iter().fold(0_u64, |n, l| {
        n.saturating_add(u64::try_from(l.frequent.len()).unwrap_or(0))
    });

    // A set is disqualified by a superset one bit larger with identical support.
    // Collected first, then applied, because a level is read while the level
    // above it is being walked and mutating during that would be a scan.
    let mut redundant: HashSet<ConditionMask> = HashSet::new();
    // `zip` with `skip(1)` rather than `windows(2)` and a `let..else`: that
    // pattern's else arm cannot fire, and an arm no run reaches is the coverage
    // hole §9 refuses.
    for (lower, upper) in sweep.levels.iter().zip(sweep.levels.iter().skip(1)) {
        let below: HashMap<ConditionMask, u64> =
            lower.frequent.iter().map(|i| (i.mask, i.hits)).collect();
        for larger in &upper.frequent {
            for bit in set_positions(&larger.mask) {
                let smaller = larger.mask.without_bit(bit);
                // Equal support means the extra condition costs nothing: every
                // bar the smaller one fires on, the larger one fires on too.
                if below.get(&smaller) == Some(&larger.hits) {
                    redundant.insert(smaller);
                }
            }
        }
    }

    let kept: Vec<Itemset> = sweep
        .all_frequent()
        .filter(|i| !redundant.contains(&i.mask))
        .copied()
        .collect();
    Closed { kept, considered }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::closed;
    use crate::{Sweeper, synthetic};
    use engine::Ladder;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    fn swept() -> engine::Sweep {
        let bars = synthetic::sessions(8);
        Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))
            .run(&bars, &mut evaluator())
            .sweep
    }

    #[test]
    fn nothing_kept_has_a_superset_of_identical_support() {
        // The defining property, checked directly rather than through the
        // algorithm that produced it.
        let sweep = swept();
        let c = closed(&sweep);
        let by_mask: std::collections::HashMap<[u64; 6], u64> = sweep
            .all_frequent()
            .map(|i| (i.mask.words(), i.hits))
            .collect();

        let mut checked = 0_u32;
        for kept in &c.kept {
            for bit in 0..vocab::ConditionMask::BITS {
                if kept.mask.get(bit) {
                    continue;
                }
                let bigger = kept.mask.with_bit(bit);
                if let Some(&hits) = by_mask.get(&bigger.words()) {
                    assert_ne!(
                        hits, kept.hits,
                        "a kept set has a superset with identical support, which \
                         is exactly what closure forbids"
                    );
                    checked = checked.saturating_add(1);
                }
            }
        }
        assert!(checked > 0, "the fixture must exercise real supersets");
    }

    #[test]
    fn every_discarded_set_is_recoverable_from_a_kept_one() {
        // LOSSLESS is the claim, and this is what it means: a discarded set's
        // support equals that of some kept superset, so nothing about it is
        // unknown after the discard.
        let sweep = swept();
        let c = closed(&sweep);
        let kept: std::collections::HashMap<[u64; 6], u64> =
            c.kept.iter().map(|i| (i.mask.words(), i.hits)).collect();

        let mut recovered = 0_u32;
        for i in sweep.all_frequent() {
            if kept.contains_key(&i.mask.words()) {
                continue;
            }
            // Some kept superset must carry this exact support.
            let found = c
                .kept
                .iter()
                .any(|k| k.mask.intersect(&i.mask) == i.mask && k.hits == i.hits);
            assert!(
                found,
                "a discarded set has no kept superset of equal support -- the \
                 discard lost information, which closure must never do"
            );
            recovered = recovered.saturating_add(1);
        }
        assert!(recovered > 0, "the fixture must discard something");
    }

    #[test]
    fn the_compression_is_reported_and_the_counts_reconcile() {
        let sweep = swept();
        let c = closed(&sweep);
        let total = sweep.all_frequent().count();

        assert_eq!(usize::try_from(c.considered).unwrap_or(usize::MAX), total);
        assert!(c.kept.len() <= total, "closure never invents a set");
        assert_eq!(
            usize::try_from(c.redundant()).unwrap_or(usize::MAX),
            total.saturating_sub(c.kept.len()),
            "kept plus redundant must be everything, or one of them is wrong"
        );
        assert!(
            c.redundant() > 0,
            "this fixture is 3,689 combinations over twelve levels; if NOTHING \
             is redundant the check is not running"
        );
    }

    #[test]
    fn an_empty_sweep_closes_to_nothing_without_panicking() {
        let empty = engine::Sweep::default();
        let c = closed(&empty);
        assert!(c.kept.is_empty());
        assert_eq!(c.considered, 0);
        assert_eq!(c.redundant(), 0);
    }

    #[test]
    fn closure_is_idempotent_across_runs() {
        // §3 rule 5. `HashSet` iteration is randomised per process, so the
        // output order must come from the sweep's canonical order and not from
        // the set used to mark redundancy.
        let sweep = swept();
        let first: Vec<[u64; 6]> = closed(&sweep).kept.iter().map(|i| i.mask.words()).collect();
        for _ in 0..4 {
            let again: Vec<[u64; 6]> = closed(&sweep).kept.iter().map(|i| i.mask.words()).collect();
            assert_eq!(first, again, "two closures of one sweep disagreed");
        }
    }
}
