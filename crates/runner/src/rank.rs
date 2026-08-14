//! The best combinations, in bounded memory.
//!
//! # The wall this exists to remove
//!
//! A sweep retains every survivor. Measured on this machine: **212 bytes per
//! retained combination**, 61,125,295 combinations at 13.0 GB, and the arithmetic
//! from there is unforgiving —
//!
//! | combinations | retained |
//! |---|---|
//! | 61 million | 13 GB |
//! | 1 billion | 212 GB |
//! | 100 billion | 21,200 GB |
//!
//! No machine holds the last row, and the frequent-itemset literature says so in
//! its own words: the FIMI benchmark could not run below a threshold because
//! *the output file* exceeded its limit, not because the search was too slow.
//! The binding constraint is what you keep.
//!
//! # What this does instead
//!
//! Keeps `k` and throws the rest away as it goes. Memory becomes a function of
//! how many results you want to LOOK at, not of how many exist — 10,000 kept is
//! 2.1 MB whether the sweep enumerated a million or a hundred billion.
//!
//! # Ranked by |t|, and why the absolute value
//!
//! A combination that reliably precedes a **fall** is as tradeable as one that
//! precedes a rise; only the direction of the position differs. Ranking by `t`
//! signed would discard every short setup, so the order is on |t| and the sign
//! is carried in [`crate::outcome::Edge::mean_paisa`] for a reader to see.
//!
//! This is a stated choice, recorded in `docs/05-decisions.md`, not a
//! derivation — as with the horizon and the measure it sits beside.
//!
//! # It ranks by evidence, and the bar for that evidence is elsewhere
//!
//! A high |t| here is **not** a finding. [`crate::significance`] computes what t
//! must clear given how many hypotheses the run tested, and on a sweep of
//! sixty-one million that bar is above 6. This module orders candidates; it does
//! not bless them, and the report prints both numbers side by side so the
//! difference is impossible to miss.

use core::cmp::Ordering;
use std::collections::BinaryHeap;

use engine::Sweep;
use indicators::column::Column;
use vocab::ConditionMask;

use crate::outcome::{Edge, Forward, edge};

/// One combination and what it did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scored {
    /// The combination.
    pub mask: ConditionMask,
    /// How many swept bars it fired on, from the sweep.
    pub hits: u64,
    /// What happened after those bars.
    pub edge: Edge,
}

/// Ordered by |t|, then by mask so ties are stable across processes.
///
/// `f64` is not `Ord`, and the usual escape — comparing with `partial_cmp` and
/// unwrapping — is a panic on a NaN. `total_cmp` gives a genuine total order
/// over every `f64` including NaN, so the heap needs no fallible arm and
/// `CLAUDE.md` §3 rule 5's byte-for-byte reproducibility holds without one.
impl Ord for Scored {
    fn cmp(&self, other: &Self) -> Ordering {
        self.edge
            .t
            .abs()
            .total_cmp(&other.edge.t.abs())
            .then_with(|| self.mask.words().cmp(&other.mask.words()))
    }
}

impl PartialOrd for Scored {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Scored {}

/// The kept combinations, and how many were weighed to find them.
#[derive(Clone, Debug, Default)]
pub struct Ranked {
    /// Best first, by |t|.
    pub top: Vec<Scored>,
    /// Every combination the sweep produced — the number this kept `top` out of.
    ///
    /// Reported because `top.len()` alone cannot distinguish "the sweep found
    /// four combinations" from "the sweep found sixty-one million and this is
    /// the best four", and `CLAUDE.md` §4 does not allow those to look alike.
    pub considered: u64,
}

/// The best `keep` combinations by |t|, in memory proportional to `keep`.
///
/// # Why a heap and not a sort
///
/// Sorting needs every scored combination resident at once, which is the 21 TB
/// this module exists to avoid. A bounded min-heap holds `keep` and discards on
/// arrival: the smallest |t| currently held is the admission price, and anything
/// below it is dropped without ever being stored.
///
/// # Cost
///
/// Per combination: one [`edge`] pass over the column, then O(log keep) to
/// admit or O(1) to reject. Memory is O(keep) and independent of how many
/// combinations exist, which is the whole point.
///
/// The edge pass is O(column) per combination and that is not a defect —
/// measuring what a combination did requires looking at the bars it fired on,
/// and there is no way to know that without visiting them. It is O(1) per
/// (bar, combination), which is what `CLAUDE.md` §3 rule 4 requires.
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn rank(sweep: &Sweep, column: &Column, forward: &Forward, keep: usize) -> Ranked {
    let mut heap: BinaryHeap<core::cmp::Reverse<Scored>> = BinaryHeap::with_capacity(keep);
    let mut considered: u64 = 0;

    for itemset in sweep.all_frequent() {
        considered = considered.saturating_add(1);
        if keep == 0 {
            continue;
        }
        let scored = Scored {
            mask: itemset.mask,
            hits: itemset.hits,
            edge: edge(column, forward, &itemset.mask),
        };
        if heap.len() < keep {
            heap.push(core::cmp::Reverse(scored));
            continue;
        }
        // The admission price is the weakest currently held. Comparing before
        // pushing is what keeps the heap at `keep` rather than letting it grow
        // and trimming afterwards -- the trim-after form allocates the whole
        // result set, which is the thing this module refuses to do.
        let weakest_is_weaker = heap.peek().is_some_and(|core::cmp::Reverse(w)| *w < scored);
        if weakest_is_weaker {
            heap.pop();
            heap.push(core::cmp::Reverse(scored));
        }
    }

    let mut top: Vec<Scored> = heap.into_iter().map(|core::cmp::Reverse(s)| s).collect();
    // Best first. `sort_unstable_by` with the same total order the heap used, so
    // the output is identical across processes -- a `HashSet`-derived order
    // would not be, and §3 rule 5 forbids that reaching the output.
    top.sort_unstable_by(|a, b| b.cmp(a));
    Ranked { top, considered }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{Scored, rank};
    use crate::outcome::{Edge, Horizon, forward};
    use crate::{Sweeper, synthetic};
    use engine::Ladder;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use vocab::ConditionMask;

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    fn scored(t: f64, bit: u32) -> Scored {
        Scored {
            mask: ConditionMask::default().with_bit(bit),
            hits: 1,
            edge: Edge {
                n: 2,
                mean_paisa: t,
                t,
            },
        }
    }

    #[test]
    fn the_order_is_on_the_absolute_t_so_a_short_setup_is_not_discarded() {
        let up = scored(3.0, 1);
        let down = scored(-9.0, 2);
        assert!(
            down > up,
            "a combination that reliably precedes a FALL is as tradeable as one \
             that precedes a rise; ranking on signed t would throw every short \
             setup away"
        );
    }

    #[test]
    fn ties_break_on_the_mask_so_the_order_is_stable_across_processes() {
        let a = scored(4.0, 1);
        let b = scored(4.0, 2);
        assert_ne!(a.cmp(&b), core::cmp::Ordering::Equal);
        assert_eq!(a.cmp(&a), core::cmp::Ordering::Equal);
    }

    #[test]
    fn a_nan_t_is_ordered_rather_than_panicking() {
        // `partial_cmp().unwrap()` is the usual way to put an f64 in a heap and
        // it is a panic waiting for a degenerate sample. `total_cmp` orders NaN
        // rather than refusing to.
        let nan = scored(f64::NAN, 1);
        let real = scored(5.0, 2);
        assert_ne!(nan.cmp(&real), core::cmp::Ordering::Equal);
        assert_eq!(nan.cmp(&nan), core::cmp::Ordering::Equal);
    }

    #[test]
    fn keeping_zero_weighs_everything_and_retains_nothing() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))
            .run(&bars, &mut evaluator());
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, Horizon::DEFAULT);

        let r = rank(&out.sweep, &column, &f, 0);
        assert!(r.top.is_empty());
        assert_eq!(
            r.considered,
            u64::try_from(out.sweep.all_frequent().count()).unwrap_or(u64::MAX),
            "every combination is still counted, because 'kept none of four' and \
             'kept none of sixty-one million' are different facts"
        );
    }

    #[test]
    fn the_kept_set_is_the_best_by_absolute_t_and_bounded_by_keep() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))
            .run(&bars, &mut evaluator());
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, Horizon::DEFAULT);
        let total = out.sweep.all_frequent().count();
        assert!(total > 100, "the fixture must produce enough to rank");

        let keep = 25;
        let r = rank(&out.sweep, &column, &f, keep);
        assert_eq!(r.top.len(), keep, "the heap must fill and stay bounded");
        assert_eq!(usize::try_from(r.considered).unwrap_or(usize::MAX), total);

        // Best first, and every kept one is at least as strong as the next.
        // `zip` with `skip(1)` rather than `windows(2)` and a `let..else`: the
        // else arm of that pattern cannot fire, and an arm no run reaches is
        // the coverage hole §9 refuses.
        for (a, b) in r.top.iter().zip(r.top.iter().skip(1)) {
            assert!(
                a.edge.t.abs() >= b.edge.t.abs(),
                "the output must be ordered best first"
            );
        }

        // AND NOTHING DISCARDED WAS BETTER THAN WHAT WAS KEPT, which is the only
        // property that makes a bounded heap equivalent to sorting everything.
        let worst_kept = r.top.last().map_or(0.0, |s: &Scored| s.edge.t.abs());
        let better_than_worst = out
            .sweep
            .all_frequent()
            .filter(|i| crate::outcome::edge(&column, &f, &i.mask).t.abs() > worst_kept)
            .count();
        assert!(
            better_than_worst <= keep,
            "{better_than_worst} combinations beat the weakest kept one, but \
             only {keep} were kept -- the heap dropped something it should have \
             admitted"
        );
    }

    #[test]
    fn asking_for_more_than_exists_returns_everything_and_no_padding() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))
            .run(&bars, &mut evaluator());
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, Horizon::DEFAULT);
        let total = out.sweep.all_frequent().count();

        let r = rank(&out.sweep, &column, &f, total.saturating_mul(2));
        assert_eq!(r.top.len(), total, "no padding, no duplication");
    }

    #[test]
    fn ranking_is_idempotent_across_runs() {
        // §3 rule 5. A `BinaryHeap`'s iteration order is not sorted, so the
        // final sort is what makes this true -- and it is what a `HashSet`-
        // derived order would break.
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))
            .run(&bars, &mut evaluator());
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, Horizon::DEFAULT);

        let first = rank(&out.sweep, &column, &f, 20);
        for _ in 0..4 {
            let again = rank(&out.sweep, &column, &f, 20);
            let a: Vec<_> = first.top.iter().map(|s| s.mask.words()).collect();
            let b: Vec<_> = again.top.iter().map(|s| s.mask.words()).collect();
            assert_eq!(a, b, "two rankings of one input disagreed");
        }
    }
}
