//! What a walk RETAINS, which is not the same question as how far it walks.
//!
//! # The measurement this module exists for
//!
//! A full-range sweep — `zerodha NIFTY 2019-12 .. 2026-08` — reached **7.4 GB
//! resident and was killed at exit 137**, and the cause was measured rather
//! than guessed: it is not the combination ladder. Every one of the sixteen
//! ladders in two eight-rung runs had finished or halted **within two and a
//! half minutes**, at a peak of 8.91 GB across all eight concurrent rungs with
//! 6.35 GB of that RETAINED. What ran afterwards — ranking 15 to 17 million
//! retained survivors per rung, eight rungs at once — is where the process
//! died.
//!
//! `crate::DEFAULT_CEILING`'s own table already names the term: of the five
//! summands in a level's live set, `56 · Σ_{j<k} S_j` is *"the retained
//! survivors of every earlier level"*, and its comment ends **"it exists
//! because `Sweep::levels` keeps every survivor for a ranker that wants only
//! the top `keep`."** This module is the other half of that sentence.
//!
//! # Two things are bounded here and they are DIFFERENT bounds
//!
//! [`Streamed`] bounds what the ENGINE holds. A walk that streams keeps two
//! levels alive at once — the one being joined from and the one being built —
//! instead of every level to the end, and records each finished level as a
//! [`Tally`], which is a [`crate::Frontier`] with its survivor vector replaced
//! by that vector's length. The survivors themselves are handed to the caller
//! at the level boundary and then dropped.
//!
//! [`Best`] bounds what a CALLER holds. It is a fixed-capacity retention over
//! [`crate::Itemset`], ordered by `(hits, mask.words())` descending, and it is
//! offered as a ready-made sink for callers that do not want to write one. A
//! caller with a better score than support — `runner::rank` scores by forward
//! edge, which this crate cannot compute and must not pretend to — feeds its
//! own heap from the same boundary instead.
//!
//! # NEITHER IS A DEPTH PARAMETER, and the distinction is testable
//!
//! `CLAUDE.md` §6 bans a `k` on the sweep, and a retention cap will be read as
//! one, so: the ladder still walks upward from k=1 and its normal completion is
//! still the frequent frontier becoming empty. Its existing candidate or pair
//! budget may refuse first, loudly, exactly as on the retaining path. Nothing
//! here is consulted by the join, by the subset prune, by the support test or
//! by either budget. A cap of zero and a cap past the widest level walk the
//! **same ladder to the same depth and produce the same [`Tally`] for every
//! level** — which is not an argument, it is
//! `engine::keep::a_cap_of_zero_and_a_generous_cap_walk_the_same_ladder`.
//!
//! # The narrowing is REPORTED, never absorbed
//!
//! `CLAUDE.md` §4 bans a fallback that hides a failure. A cap that refuses a
//! survivor has genuinely narrowed the answer, so both numbers are carried out
//! of the walk beside the result: [`Streamed::streamed`] is every survivor the
//! engine handed over and dropped, and [`Best::discarded`] is every one the cap
//! refused. `engine::keep::every_offer_is_either_held_or_counted_as_discarded`
//! pins the identity that makes them readable — offered = held + discarded.

use crate::{Excluded, Frontier, Halt, Itemset};

/// One level's five exits, WITHOUT the survivors themselves.
///
/// # Why a second type rather than an emptied [`Frontier`]
///
/// A `Frontier` whose `frequent` had been emptied would still answer
/// [`Frontier::reconciles`], and it would answer FALSE — the five-exit identity
/// counts survivors, so a level drained of them reads as a level that lost
/// candidates. `runner::report` renders exactly that as `LOST A CANDIDATE`, a
/// defect banner. Handing a caller a value that reports a bug where there is
/// none is the failure wearing a success's clothes `CLAUDE.md` §4 bans, said
/// backwards.
///
/// So the survivors become a COUNT and the type says so in its name. Every
/// other field is carried across unchanged and [`Self::reconciles`] asks the
/// same question `Frontier::reconciles` asks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// The level. `k == 1` is single conditions.
    pub k: u32,
    /// How many combinations met `min_hits`. This is `frequent.len()` of the
    /// level the walk built, and it is the only thing kept from that vector.
    pub survivors: u64,
    /// Candidates the join produced at this level. See [`Frontier::generated`].
    pub generated: u64,
    /// Candidates already seen at this level. See [`Frontier::duplicates`].
    pub duplicates: u64,
    /// Positions refused before anything else was measured. See
    /// [`Frontier::excluded`].
    pub excluded: u64,
    /// Candidates the subset prune removed unevaluated. See
    /// [`Frontier::pruned`].
    pub pruned: u64,
    /// Candidates evaluated against the bars and found too rare. See
    /// [`Frontier::infrequent`].
    pub infrequent: u64,
}

impl Tally {
    /// The five exits of a level the walk has just finished with.
    #[must_use]
    pub fn of(level: &Frontier) -> Self {
        Self {
            k: level.k,
            survivors: crate::len_u64(level.frequent.len()),
            generated: level.generated,
            duplicates: level.duplicates,
            excluded: level.excluded,
            pruned: level.pruned,
            infrequent: level.infrequent,
        }
    }

    /// Every candidate the join produced ended in exactly one of five places.
    ///
    /// The same identity [`Frontier::reconciles`] states, asked of a level whose
    /// survivors are gone and whose COUNT of them is not.
    #[must_use]
    pub fn reconciles(&self) -> bool {
        let accounted = self
            .duplicates
            .saturating_add(self.excluded)
            .saturating_add(self.pruned)
            .saturating_add(self.infrequent)
            .saturating_add(self.survivors);
        accounted == self.generated
    }
}

/// The outcome of a walk that did not retain its survivors.
///
/// Field for field this is [`crate::Sweep`] with `Vec<Frontier>` replaced by
/// `Vec<Tally>` and one number added. A depth-25 walk therefore holds
/// `25 × size_of::<Tally>()` bytes of level record whatever the frontier widths
/// were, against the `56 · Σ_j |F_j|` a retaining walk holds — the term
/// `crate::DEFAULT_CEILING`'s table names as the one that matters.
/// `engine::keep::a_streamed_walk_retains_no_survivor_at_all` measures the
/// difference on a walked ladder rather than asserting it.
#[derive(Clone, Debug, Default)]
pub struct Streamed {
    /// One entry per level walked, in order. Its length **is** the depth
    /// reached, which is a result and never an input.
    pub levels: Vec<Tally>,
    /// Positions excluded before k=1, named per D-0080. Bounded by the
    /// vocabulary rather than by the data, so this one IS retained whole.
    pub excluded: Vec<Excluded>,
    /// Bars loaded. Support fractions are over this.
    pub bars: u64,
    /// The threshold actually applied, echoed so a result is self-describing.
    pub min_hits: u64,
    /// `None` when the ladder went extinct. See [`crate::Sweep::halted`].
    pub halted: Option<Halt>,
    /// **Every survivor this walk handed to the sink and then dropped.**
    ///
    /// This is the loud number `CLAUDE.md` §4 asks for. A caller that ignored
    /// the sink is holding none of these combinations, and the difference
    /// between that and a sweep which found nothing is exactly this field. It
    /// is NOT a defect count and NOT a truncation of the search: every one of
    /// them was enumerated, evaluated against the bars and offered.
    pub streamed: u64,
}

impl Streamed {
    /// The depth the ladder reached before a level came back empty.
    ///
    /// Zero when no single condition met `min_hits`. The same question
    /// [`crate::Sweep::depth`] answers, over counts rather than vectors.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.levels.iter().filter(|l| l.survivors > 0).count()
    }

    /// Did the ladder go extinct, which is the answer `CLAUDE.md` §6 asks for?
    ///
    /// False means a level breached a budget and the walk stopped short. See
    /// [`crate::Sweep::completed`].
    #[must_use]
    pub const fn completed(&self) -> bool {
        self.halted.is_none()
    }
}

/// Does `a` rank below `b`, and is that order derived from the CANDIDATE?
///
/// `(hits, mask.words())` ascending. `[u64; 6]` has a total order, so two
/// itemsets compare equal only when they are the same itemset — a mask is
/// unique within a level and its popcount names the level, so it is unique
/// across the whole walk. A strict total order has exactly one sorted sequence
/// whatever order its elements arrived in, which is what makes this retention
/// reproducible under `CLAUDE.md` §3 rule 5.
///
/// An insertion-order tiebreak would not be. `crate::drain` spreads support
/// counting across every core and `runner` walks eight rungs at once, so
/// arrival order is a property of the schedule and the schedule is not part of
/// the answer.
fn ranks_below(a: &Itemset, b: &Itemset) -> bool {
    (a.hits, a.mask.words()) < (b.hits, b.mask.words())
}

/// [`ranks_below`] asked of two slots, by index.
///
/// # Why it takes indices and answers `false` off the end
///
/// So the out-of-range answer is REACHABLE from a test. Every caller below is a
/// sift step whose indices are already in range, so that arm cannot fire in
/// production — and an arm no fixture can enter is the coverage hole
/// `CLAUDE.md` §9 refuses and, worse, a decision nobody has ever seen made.
/// `crate::cannot_grow` takes its growth amount as a parameter for the same
/// reason and says so in its own doc.
///
/// `false` and not a panic: "there is nothing at that index" is honestly "the
/// one at `lower` does not rank below it", and it stops a sift rather than
/// ending a run. It is also what makes the sift's `swap` calls sound — a `true`
/// answer is proof that both slots exist.
fn weaker(held: &[Itemset], lower: usize, upper: usize) -> bool {
    match (held.get(lower), held.get(upper)) {
        (Some(l), Some(u)) => ranks_below(l, u),
        _ => false,
    }
}

/// The best `cap` itemsets offered, in memory proportional to `cap`.
///
/// # What it is, in one line
///
/// A binary min-heap of fixed capacity: the weakest retained itemset sits at
/// the root, so an offer is one comparison to reject and a sift to admit.
///
/// # Why the heap is written out rather than taken from `std`
///
/// `std::collections::BinaryHeap` is the obvious tool and is what
/// `runner::rank` uses. CI gate 11 rule 4 counts that type's name in non-test
/// source against a per-file allowlist, and this crate's entry is pinned at
/// what it measures today. Raising it is a `.github/workflows/ci.yml` edit, and
/// the reason gate 11 states for keeping its escape hatch in the gate and never
/// in the code is exactly the reason a session that adds a construct must not
/// also widen the list that would have refused it.
///
/// So the sift is spelled out here, which costs about forty lines and buys one
/// thing the standard heap could not have given anyway: [`Self::discarded`], a
/// count of what the cap refused, which a plain heap has no place to keep.
///
/// **This is not a depth control.** See the module header: a cap of zero walks
/// the same ladder as a cap of a million.
#[derive(Clone, Debug, Default)]
pub struct Best {
    cap: usize,
    held: Vec<Itemset>,
    discarded: u64,
}

impl Best {
    /// A retention that will hold at most `cap` itemsets.
    ///
    /// Zero is a legal cap and is not raised to one, unlike
    /// [`crate::Ladder::with_min_hits`]'s threshold. A threshold of zero
    /// switches OFF the extinction that ends the walk; a retention of zero
    /// simply keeps nothing, counts everything it was offered in
    /// [`Self::discarded`], and changes no other number anywhere.
    ///
    /// The vector is reserved to `cap` here, because that is what the type
    /// promises to hold and growing to it by doubling would overshoot the bound
    /// the caller asked for.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            cap,
            held: Vec::with_capacity(cap),
            discarded: 0,
        }
    }

    /// The cap this retention was built with.
    #[must_use]
    pub const fn cap(&self) -> usize {
        self.cap
    }

    /// How many itemsets are held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// Is nothing held?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// **Every itemset the cap refused**, across every offer.
    ///
    /// A caller that reports a top list without this number is claiming the
    /// list is the whole frequent set. It is the count `CLAUDE.md` §4 requires
    /// beside a narrowed answer, and it counts an EVICTION as well as an
    /// outright rejection: an itemset admitted at one moment and pushed out by
    /// a stronger one later is just as absent from the result.
    #[must_use]
    pub const fn discarded(&self) -> u64 {
        self.discarded
    }

    /// Offer one itemset.
    ///
    /// Held when the retention is not yet full, or when it ranks above the
    /// weakest held. Otherwise refused, and counted.
    pub fn offer(&mut self, candidate: Itemset) {
        if self.held.len() < self.cap {
            self.held.push(candidate);
            self.sift_up(self.held.len().saturating_sub(1));
            return;
        }
        // FULL. Exactly one itemset leaves — the candidate itself when it ranks
        // no higher than the weakest held, or that weakest one when it does.
        // Both are one combination refused, so the counter moves either way and
        // moves once.
        self.discarded = self.discarded.saturating_add(1);
        if self
            .held
            .first()
            .is_some_and(|weakest| ranks_below(weakest, &candidate))
        {
            let _ = self.take_root();
            self.held.push(candidate);
            self.sift_up(self.held.len().saturating_sub(1));
        }
    }

    /// Offer every survivor of one completed level.
    ///
    /// The level arrives in canonical mask order, which this ignores: the
    /// retention's own order is total, so what it holds does not depend on the
    /// order it was offered in.
    /// `engine::keep::the_order_of_arrival_does_not_change_what_is_kept`
    /// permutes a level and requires the same rows out.
    pub fn offer_level(&mut self, level: &Frontier) {
        for itemset in &level.frequent {
            self.offer(*itemset);
        }
    }

    /// Everything held, **strongest first**.
    ///
    /// Draining the heap yields ascending order, so the vector is reversed
    /// once. The cost is `cap log cap` comparisons over what the caller itself
    /// bounded, never over the frequent set — the same trade `runner::rank`
    /// makes when it orders its own `keep` rows for reproducibility.
    #[must_use]
    pub fn into_ordered(mut self) -> Vec<Itemset> {
        let mut out: Vec<Itemset> = Vec::with_capacity(self.held.len());
        while let Some(weakest) = self.take_root() {
            out.push(weakest);
        }
        out.reverse();
        out
    }

    /// Remove and return the weakest held itemset.
    fn take_root(&mut self) -> Option<Itemset> {
        let last = self.held.len().checked_sub(1)?;
        self.held.swap(0, last);
        let out = self.held.pop();
        self.sift_down(0);
        out
    }

    /// Restore the heap property upward from `from`.
    ///
    /// # `checked_sub` rather than a guard on the index, and that is a mutation
    ///
    /// This read `while at > 0 { let parent = at.saturating_sub(1) / 2; .. }`,
    /// and `cargo mutants` reported the guard as a SURVIVOR: at the root the
    /// saturating subtraction yields the root again, `weaker` compares a slot
    /// with itself, the strict order answers false, and the loop returns. So
    /// `>=` and `>` behave identically and no test could tell them apart --
    /// an equivalent mutant, which is a guard that is not carrying its weight.
    ///
    /// `checked_sub` removes the second spelling instead of testing for it: the
    /// root has no parent, and that is now the loop's own condition rather than
    /// a comparison beside it.
    fn sift_up(&mut self, from: usize) {
        let mut at = from;
        while let Some(above) = at.checked_sub(1) {
            let parent = above / 2;
            if !weaker(&self.held, at, parent) {
                return;
            }
            // A `true` from `weaker` is proof both slots exist.
            self.held.swap(at, parent);
            at = parent;
        }
    }

    /// Restore the heap property downward from `from`.
    ///
    /// # The step count is BOUNDED IN THE LOOP, and that bound is not decoration
    ///
    /// Each step moves to a CHILD, whose index is strictly greater than its
    /// parent's and which `weaker` has just proved is inside the vector, so a
    /// sift cannot take more steps than the heap holds entries -- far fewer,
    /// since the index at least doubles.
    ///
    /// The bound is written into the loop rather than argued in this comment
    /// because `cargo mutants` showed what the argument is worth: with a bare
    /// `loop`, inverting the comparison below made the walk swap a slot with
    /// itself for ever and the run was reported as a TIMEOUT rather than a
    /// failure. A timeout is a test that did not finish, not a test that
    /// passed. Bounded, the same mutation leaves a heap whose order is wrong,
    /// which `engine::keep::the_order_of_arrival_does_not_change_what_is_kept`
    /// sees on the very next offer.
    fn sift_down(&mut self, from: usize) {
        let mut at = from;
        for _ in 0..self.held.len() {
            let left = at.saturating_mul(2).saturating_add(1);
            let right = left.saturating_add(1);
            let mut weakest = at;
            if weaker(&self.held, left, weakest) {
                weakest = left;
            }
            if weaker(&self.held, right, weakest) {
                weakest = right;
            }
            if weakest == at {
                return;
            }
            self.held.swap(at, weakest);
            at = weakest;
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{Best, Streamed, Tally, ranks_below, weaker};
    use crate::{Frontier, Itemset, Ladder, column::Column};
    use vocab::ConditionMask;

    /// One itemset at a named strength, on a bit of its own.
    fn item(hits: u64, bit: u32) -> Itemset {
        Itemset {
            mask: ConditionMask::default().with_bit(bit),
            hits,
        }
    }

    /// Forty itemsets, every one distinct under `(hits, mask.words())`.
    fn forty() -> Vec<Itemset> {
        (0..40_u32).map(|i| item(u64::from(i), i)).collect()
    }

    /// Does every parent rank at or below both of its children?
    ///
    /// The min-heap property, asked of the whole vector rather than of the one
    /// slot a sift just touched -- a sift that fixed its own path and broke
    /// another would pass the narrower question.
    fn is_a_min_heap(best: &Best) -> bool {
        (0..best.held.len()).all(|at| {
            !weaker(&best.held, at.saturating_mul(2).saturating_add(1), at)
                && !weaker(&best.held, at.saturating_mul(2).saturating_add(2), at)
        })
    }

    /// An index past the end answers `false`, and that arm is REACHABLE.
    ///
    /// It cannot fire from a sift -- every index a sift passes is in range --
    /// which is exactly why `weaker` takes indices rather than two itemsets. An
    /// arm no fixture can enter is a coverage hole and a decision nobody has
    /// ever seen made.
    #[test]
    fn an_index_past_the_end_ranks_below_nothing() {
        let held = vec![item(1, 0), item(2, 1)];
        assert!(weaker(&held, 0, 1), "1 hit ranks below 2");
        assert!(!weaker(&held, 1, 0), "and 2 does not rank below 1");
        assert!(!weaker(&held, 9, 0), "nothing at 9 ranks below the root");
        assert!(
            !weaker(&held, 0, 9),
            "and the root ranks below nothing at 9"
        );
        assert!(!weaker(&held, 9, 9), "nor does nothing rank below nothing");
        // EQUAL HITS FALL THROUGH TO THE MASK, which is what makes the order
        // total and therefore reproducible. Without this the two would compare
        // equal and which one survived a cut would depend on arrival.
        let tie = vec![item(5, 0), item(5, 1)];
        assert!(weaker(&tie, 0, 1), "bit 0's word sorts below bit 1's");
        assert!(!weaker(&tie, 1, 0), "and the comparison is antisymmetric");

        // STRICT, AND THAT IS THE WHOLE PROPERTY. `cargo mutants` turned the
        // `<` in `ranks_below` into `<=` and no test noticed: with `<=` an
        // itemset ranks below ITSELF, the root of a full heap is evicted and
        // reinserted for every equal offer, and -- worse -- `sift_up`'s guard
        // stops being an equivalence. A strict order is what makes two equal
        // candidates interchangeable, which is what makes the cut reproducible
        // under `CLAUDE.md` §3 rule 5.
        let one = item(5, 0);
        assert!(
            !ranks_below(&one, &one),
            "nothing ranks below itself; the order is strict"
        );
        assert!(
            !weaker(&tie, 0, 0),
            "and neither does a slot against itself"
        );
    }

    /// Offered = held + discarded, at every cap, with nothing unaccounted.
    ///
    /// `CLAUDE.md` §4 wants a narrowing stated rather than absorbed, and this is
    /// the identity that makes the statement checkable: a caller can always tell
    /// how much of the frequent set it is NOT looking at.
    #[test]
    fn every_offer_is_either_held_or_counted_as_discarded() {
        for cap in [0_usize, 1, 7, 39, 40, 100] {
            let mut best = Best::with_capacity(cap);
            for it in forty() {
                best.offer(it);
            }
            assert_eq!(best.cap(), cap, "the cap is echoed unchanged");
            assert_eq!(
                best.len().min(cap),
                best.len(),
                "nothing above the cap may be held"
            );
            assert_eq!(best.len(), cap.min(40), "and the cap is actually filled");
            assert_eq!(
                u64::try_from(best.len()).unwrap_or(u64::MAX) + best.discarded(),
                40,
                "every offer ends in exactly one of two places"
            );
            assert_eq!(
                best.is_empty(),
                cap == 0,
                "empty exactly when it holds none"
            );
            assert!(
                is_a_min_heap(&best),
                "the heap property holds after the run"
            );
        }
    }

    /// A cap of zero keeps nothing and refuses everything, loudly.
    #[test]
    fn a_cap_of_zero_holds_nothing_and_says_so() {
        let mut best = Best::with_capacity(0);
        for it in forty() {
            best.offer(it);
        }
        assert!(best.is_empty());
        assert_eq!(best.discarded(), 40);
        assert!(
            best.into_ordered().is_empty(),
            "and draining an empty retention is not a special case"
        );
    }

    /// What survives the cut is the strongest, and it comes out strongest first.
    #[test]
    fn the_strongest_are_kept_and_ordered_strongest_first() {
        let mut best = Best::with_capacity(7);
        for it in forty() {
            best.offer(it);
        }
        let out = best.into_ordered();
        let want: Vec<Itemset> = (33..40_u32).rev().map(|i| item(u64::from(i), i)).collect();
        assert_eq!(out, want, "the top seven by hits, descending");
    }

    /// Equal support at the capacity boundary is settled by the mask, not by
    /// which candidate happened to arrive before the retention filled.
    ///
    /// The broader permutation test below varies support too, so an
    /// implementation that compared only `hits` could still pass it: its top
    /// nine have distinct scores and never need the second half of the key.
    /// This fixture removes that escape hatch. Every candidate has nine hits,
    /// the cap cuts through the tie, and three different arrival orders must
    /// retain the same four masks in the same strongest-first order.
    #[test]
    fn equal_hits_at_the_cut_are_broken_by_the_candidate_mask() {
        let ascending: Vec<Itemset> = (0..10_u32).map(|bit| item(9, bit)).collect();
        let mut descending = ascending.clone();
        descending.reverse();
        let shuffled: Vec<Itemset> = [5_u32, 0, 9, 2, 7, 1, 8, 3, 6, 4]
            .into_iter()
            .map(|bit| item(9, bit))
            .collect();

        let run = |order: &[Itemset]| {
            let mut best = Best::with_capacity(4);
            for candidate in order {
                best.offer(*candidate);
            }
            (best.discarded(), best.into_ordered())
        };
        let expected: Vec<Itemset> = (6..10_u32).rev().map(|bit| item(9, bit)).collect();
        let first = run(&ascending);

        assert_eq!(first.0, 6, "six candidates fall outside a top-four cut");
        assert_eq!(first.1, expected, "the four greatest masks win the tie");
        assert_eq!(first, run(&descending), "reverse arrival changes nothing");
        assert_eq!(first, run(&shuffled), "shuffled arrival changes nothing");
    }

    /// The retention is a SET decision, not an arrival-order one.
    ///
    /// `CLAUDE.md` §3 rule 5 is the whole reason the order falls through to the
    /// mask words: `crate::drain` spreads support counting across every core and
    /// `runner` walks eight rungs at once, so anything keyed on when a candidate
    /// showed up is keyed on the schedule.
    #[test]
    fn the_order_of_arrival_does_not_change_what_is_kept() {
        let ascending = forty();
        let mut descending = ascending.clone();
        descending.reverse();
        // A stride permutation, so the third ordering is neither of the first
        // two reversed and a sift that only ever walked one way is caught.
        //
        // Built rather than indexed. `ascending` IS `item(i, i)` for i in
        // 0..40, so the strided member can be constructed directly -- and a
        // `.get(at).unwrap_or_else(..)` here left a closure no input could
        // enter, which is the coverage hole this crate refuses in production
        // and should not tolerate in a fixture either. 17 is coprime with 40,
        // so `17i mod 40` visits every index exactly once.
        let strided: Vec<Itemset> = (0..40_u32)
            .map(|i| {
                let at = i.wrapping_mul(17) % 40;
                item(u64::from(at), at)
            })
            .collect();
        assert_eq!(strided.len(), ascending.len());

        let run = |order: &[Itemset]| {
            let mut best = Best::with_capacity(9);
            for it in order {
                best.offer(*it);
                assert!(
                    is_a_min_heap(&best),
                    "the heap property holds at every step"
                );
            }
            (best.discarded(), best.into_ordered())
        };

        let first = run(&ascending);
        assert_eq!(
            first,
            run(&descending),
            "reversed arrival, identical answer"
        );
        assert_eq!(first, run(&strided), "strided arrival, identical answer");
        assert_eq!(first.0, 31, "and thirty-one of the forty were refused");
    }

    /// A [`Tally`] answers the five-exit identity the [`Frontier`] answers.
    #[test]
    fn a_tally_reconciles_exactly_when_its_frontier_does() {
        let level = Frontier {
            k: 3,
            frequent: vec![item(9, 0), item(8, 1)],
            generated: 10,
            duplicates: 1,
            excluded: 2,
            pruned: 3,
            infrequent: 2,
        };
        let tally = Tally::of(&level);
        assert_eq!(tally.k, 3);
        assert_eq!(tally.survivors, 2, "the vector becomes its length");
        assert_eq!(tally.generated, 10);
        assert_eq!(tally.duplicates, 1);
        assert_eq!(tally.excluded, 2);
        assert_eq!(tally.pruned, 3);
        assert_eq!(tally.infrequent, 2);
        assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");

        let lost = Frontier {
            generated: 99,
            ..Frontier::default()
        };
        assert!(
            !lost.reconciles() && !Tally::of(&lost).reconciles(),
            "and a level that lost a candidate is still caught after the \
             survivors are gone -- which is the whole reason this is a second \
             type rather than an emptied Frontier"
        );
    }

    /// The derived shapes a caller will actually use.
    #[test]
    fn the_result_types_answer_depth_completion_and_the_derives() {
        let mut streamed = Streamed::default();
        assert_eq!(streamed.depth(), 0, "no level, no depth");
        assert!(streamed.completed(), "and nothing halted it");
        streamed.levels.push(Tally {
            k: 1,
            survivors: 4,
            ..Tally::default()
        });
        streamed.levels.push(Tally {
            k: 2,
            survivors: 0,
            ..Tally::default()
        });
        assert_eq!(
            streamed.depth(),
            1,
            "the empty level that ends the walk is recorded and is not depth"
        );
        streamed.halted = Some(crate::Halt::default());
        assert!(!streamed.completed(), "a halted walk is not a complete one");

        // The derives are part of the surface: a caller that clones a result or
        // prints one in a refusal must not be the first to find out.
        let copy = streamed.clone();
        assert_eq!(copy.levels, streamed.levels);
        assert!(!format!("{streamed:?}").is_empty());
        assert!(!format!("{:?}", Best::default()).is_empty());
        assert_eq!(
            Best::default().cap(),
            0,
            "a default retention holds nothing"
        );
        assert!(Best::default().clone().is_empty());
    }

    /// **A cap is not a depth control**, which is `CLAUDE.md` §6's whole point.
    ///
    /// Two walks over one column, one feeding a retention of zero and one a
    /// retention wider than anything the ladder can produce. Every level, every
    /// counter and the depth itself must be identical: the cap bounds what the
    /// CALLER holds and is read by nothing inside the walk.
    #[test]
    fn a_cap_of_zero_and_a_generous_cap_walk_the_same_ladder() {
        let live: Vec<u32> = (0..8).collect();
        let spec: Vec<Vec<u32>> = (0..64_u32)
            .map(|bar| (0..8_u32).filter(|b| bar % (b + 2) != 0).collect())
            .collect();
        let masks: Vec<ConditionMask> = spec
            .iter()
            .map(|bits| {
                bits.iter()
                    .fold(ConditionMask::default(), |m, &b| m.with_bit(b))
            })
            .collect();
        let column = Column::from_rows(&masks);

        let walk = |cap: usize| {
            let mut best = Best::with_capacity(cap);
            let out = Ladder::with_min_hits(1).walk_column_streamed(
                &column,
                &live,
                &mut |level, _, _| best.offer_level(level),
            );
            (out, best)
        };

        let (tight, none_kept) = walk(0);
        let (loose, all_kept) = walk(1 << 20);

        assert_eq!(tight.levels, loose.levels, "the same levels, exit for exit");
        assert_eq!(tight.depth(), loose.depth(), "and the same depth");
        assert_eq!(tight.bars, loose.bars);
        assert_eq!(tight.min_hits, loose.min_hits);
        assert_eq!(tight.halted, loose.halted);
        assert_eq!(tight.streamed, loose.streamed);
        assert!(
            tight.depth() > 2,
            "the fixture must climb, or it proves nothing"
        );

        assert!(none_kept.is_empty(), "a cap of zero keeps nothing");
        assert_eq!(
            none_kept.discarded(),
            tight.streamed,
            "and refuses exactly what it was offered -- the loud number"
        );
        assert_eq!(
            u64::try_from(all_kept.len()).unwrap_or(u64::MAX),
            loose.streamed,
            "while a cap past the frontier keeps every one of them"
        );
        assert_eq!(all_kept.discarded(), 0, "and refuses none");
    }

    /// The engine holds no survivor after a streamed walk, and says how many.
    ///
    /// [`Streamed`] has no `Vec<Itemset>` anywhere in it, so this measures the
    /// only thing there is to measure: the retaining walk's own held bytes
    /// against a level record that is `size_of::<Tally>()` per level whatever
    /// the frontier widths were.
    #[test]
    fn a_streamed_walk_retains_no_survivor_at_all() {
        let live: Vec<u32> = (0..8).collect();
        let spec: Vec<Vec<u32>> = (0..64_u32)
            .map(|bar| (0..8_u32).filter(|b| bar % (b + 2) != 0).collect())
            .collect();
        let masks: Vec<ConditionMask> = spec
            .iter()
            .map(|bits| {
                bits.iter()
                    .fold(ConditionMask::default(), |m, &b| m.with_bit(b))
            })
            .collect();
        let column = Column::from_rows(&masks);

        let retained = Ladder::with_min_hits(1).walk_column(&column, &live, &|_, _, _| {});
        let held_bytes: usize = retained
            .levels
            .iter()
            .map(|l| l.frequent.capacity().saturating_mul(size_of::<Itemset>()))
            .sum();

        let streamed =
            Ladder::with_min_hits(1).walk_column_streamed(&column, &live, &mut |_, _, _| {});
        let record_bytes = streamed.levels.len().saturating_mul(size_of::<Tally>());

        assert!(
            held_bytes > 0,
            "the retaining walk must actually hold something, or there is \
             nothing to have saved"
        );
        assert!(
            record_bytes < held_bytes,
            "a level record of {record_bytes} B must be smaller than the \
             {held_bytes} B of survivors it replaces"
        );
        assert_eq!(
            streamed.streamed,
            u64::try_from(retained.all_frequent().count()).unwrap_or(u64::MAX),
            "and every survivor the retaining walk KEPT was handed over and \
             counted rather than silently dropped"
        );
        assert_eq!(
            size_of::<Tally>(),
            size_of::<Itemset>(),
            "A WHOLE LEVEL NOW COSTS WHAT ONE SURVIVOR USED TO -- 56 bytes, \
             fixed however wide the frontier was. Pinned against `Itemset` \
             rather than against the number, so a mask widening moves both and \
             a field added to either fails the build rather than drifting."
        );
    }
}
