//! The combination ladder, walked upward until a level produces nothing.
//!
//! `docs/00-charter.md` §6 specifies this in one paragraph, and this module is
//! that paragraph and nothing more: at each level, join the previous frequent
//! frontier with itself, prune any candidate whose (k−1)-subsets are not all
//! frequent, evaluate the survivors against the bar-bit column, keep those with
//! at least `min_hits` hits, recurse, and **stop when a level produces nothing**.
//!
//! # There is no `k`
//!
//! [`Ladder`] carries no depth field. Not a default, not an override, not a
//! private one. `CLAUDE.md` §6 requires the absence rather than a default
//! because in the predecessor repository the flag defaulted to a dynamic token
//! while the frontier mask was 64 bits wide against a 74-condition vocabulary —
//! so every real run tripped the width guard and silently fell back to a
//! hardcoded `k = [1, 2]`. A parameter that can be set can be set wrongly and
//! silently. The guard against that recurring is not this comment: it is
//! [`WIDTH_IS_SUFFICIENT`], a const assertion that fails the **build** if the
//! vocabulary ever outgrows the mask.
//!
//! # What is deliberately absent
//!
//! **Ranking.** `docs/00-charter.md` §6 says "rank the survivors" and names no
//! statistic; no other document names one either. Under `CLAUDE.md` §3.1 that
//! makes the ranking metric `UNVERIFIED`, so this module returns frequent
//! itemsets with their exact hit counts and **refuses to rank them at all**. Support is a
//! frequency, not an edge — see [`Frontier::frequent`].
//!
//! That includes not ordering by support, and the wording here used to imply otherwise.
//! `sort_canonically` keys on `(mask.words(), hits)`; the word array is unique per mask
//! within a level, so the `hits` tiebreak can never fire and the emitted order is
//! **canonical mask order**, not support order. It is deterministic — which is what §3.5
//! needs — and it is not a ranking. A reader who wants the strongest first must sort, and
//! must first decide what "strongest" means, which is the decision §3.1 blocks.
//!
//! # Cost
//!
//! `CLAUDE.md` §3.4 requires each *per-operation* cost to be constant, not the
//! whole sweep. The four operations this module performs are:
//!
//! | operation | cost | why |
//! |---|---|---|
//! | mask evaluation | O(1) | [`vocab::ConditionMask::hits`] — 6 ANDs, 6 XORs, 5 ORs, 1 compare, branchless |
//! | condition lookup | O(1) | direct index into `vocab`'s fixed table |
//! | duplicate rejection | O(1) | one `HashSet` probe on a `Hash + Eq` mask |
//! | result append | O(1) amortised | `Vec::push`. Reserved at k=1; not above it |
//!
//! The reservation detail is stated because the row used to claim "capacity reserved per
//! level" and only k=1 reserves: `next_level` builds `out` and `seen` with `Vec::new` and
//! `HashSet::new`. The BOUND is unaffected -- amortised O(1) is amortised O(1) -- but the
//! stated reason was not what the code does, and an audit that read the code rather than
//! the table found it. Reserving above k=1 would mean sizing for the join's `|F|²/2`
//! candidates, which is the allocation `docs/06-limits.md` is about, not a cheap win.
//!
//! Support counting is O(bars) *by definition* — it is the measurement, not an
//! operation on a bar — and the level join is O(|F|²) in the frontier, which is
//! Apriori's documented shape. Both are stated in [`Ladder::walk`] rather than
//! hidden, per §3.6.

// Gate 16 requires every crate root to forbid unsafe, and this one did not --
// the only crate of the eleven that did not. Nothing here needs it: the whole
// module is integer arithmetic over `[u64; 6]` masks and two collections.
#![forbid(unsafe_code)]

/// The bar column transposed into one bitmap per position, and the support count
/// that reads only the bitmaps a candidate names. Same answers as [`support`],
/// far fewer bytes moved -- see the module doc for the arithmetic.
pub mod column;

use std::collections::HashSet;
use vocab::ConditionMask;

use crate::column::Column;

/// Fails the build if the vocabulary ever outgrows the mask.
///
/// This is the guard that the predecessor repository did not have. There, the
/// frontier was 64 bits against 74 conditions and the overflow was discovered
/// only by reading output that had silently stopped at `k = 2`. Here it is a
/// compile error.
pub const WIDTH_IS_SUFFICIENT: () = assert!(
    vocab::table::COUNT <= ConditionMask::BITS as usize,
    "the condition table has more positions than ConditionMask has bits; widen \
     ConditionMask::WORDS in the same change that adds the position"
);

/// Why a position never entered the ladder.
///
/// `docs/05-decisions.md` D-0080 requires that a position excluded before k=1 be
/// **named in the run output** rather than silently dropped, so that a reader of
/// a result can tell "this condition was never tried" apart from "this condition
/// was tried and lost".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// Support is exactly 0. The position is false on every loaded bar, so every
    /// combination containing it has support 0 and the whole subtree is dead.
    AlwaysFalse,
    /// Support is exactly the bar count. The position is true on every loaded
    /// bar, so it partitions nothing: every combination containing it has the
    /// same support as that combination without it.
    AlwaysTrue,
    /// The position is not `Live` in the vocabulary — retired or void.
    NotLive,
}

/// A position that was excluded before the ladder started, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Excluded {
    /// The condition bit index.
    pub position: u32,
    /// Its measured support over the loaded bars, or `None` when none was taken.
    ///
    /// `None` is [`Why::NotLive`] and only that. §3.6 forbids naming a measurement that
    /// was not made, and this field used to read `0` there -- a number indistinguishable
    /// from a position genuinely absent on every bar. Two of the three `NotLive` causes
    /// cannot be measured even in principle: `p >= ConditionMask::BITS` has no bit to
    /// build a mask from, so `with_bit(p)` is a no-op and the empty mask hits every bar.
    pub support: Option<u64>,
    /// The reason, for the run output.
    pub reason: Why,
}

/// One frequent combination and its exact hit count.
///
/// # Size, because a document quotes it
///
/// `docs/06-limits.md` §5 tabulates what a level would cost to hold, and its numbers are
/// per-`Itemset`. It quoted **32 bytes**, which was the predecessor's mask width, and the
/// figures downstream of it were wrong by 75%. The assertion below derives the size from
/// `ConditionMask` plus one `u64` rather than writing a number down, and
/// `vocab::mask` pins `ConditionMask` at `WORDS * 8` -- so 48 + 8 = **56 bytes**, and a
/// mask widening moves the document's arithmetic through a failing build rather than
/// silently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Itemset {
    /// The combination. A bar matches iff `bar_bits.hits(&mask)`.
    pub mask: ConditionMask,
    /// How many loaded bars matched. Never below the ladder's `min_hits`.
    pub hits: u64,
}

const _: () = assert!(
    core::mem::size_of::<Itemset>()
        == core::mem::size_of::<ConditionMask>() + core::mem::size_of::<u64>()
);

/// Everything one level of the ladder produced.
#[derive(Clone, Debug, Default)]
pub struct Frontier {
    /// The level. `k == 1` is single conditions.
    pub k: u32,
    /// Combinations meeting `min_hits`, in a deterministic order: ascending by
    /// the mask's word array, which is a total order over the whole 384-bit
    /// space and is therefore stable across processes and machines.
    ///
    /// **This is ordered by support only where support ties are broken by that
    /// word order.** It is not a ranking by edge, profitability or any other
    /// outcome — no document defines one, so this module does not invent one
    /// (`CLAUDE.md` §3.1).
    pub frequent: Vec<Itemset>,
    /// Candidates the join produced at this level, counting each time it was
    /// produced. The join reaches the same k-set from several pairs, so this is
    /// larger than the number of distinct candidates by exactly [`Self::duplicates`].
    pub generated: u64,
    /// Candidates the join produced that had already been seen at this level.
    /// Rejected in O(1) by one `HashSet` probe and never evaluated twice.
    /// Measured by `C-E-04` — the walk row folds duplicate rejection into its
    /// per-bar cost, and `crates/engine/benches/ratio.rs` is where it runs.
    ///
    /// This field exists because it was missing: the first version of this struct
    /// reported `generated`, `pruned` and `infrequent`, and at k=3 they came to 8
    /// against a `generated` of 10. Two candidates were unaccounted for — the
    /// duplicates — and a reader could not tell whether they had been dropped by
    /// design or lost by a bug. [`Frontier::reconciles`] now makes that a test
    /// failure rather than something to notice.
    pub duplicates: u64,
    /// Positions this level refused before measuring anything else — the D-0080
    /// support-0 / support-1 exclusions. Nonzero only at k=1, because a position
    /// is excluded once and its whole subtree dies with it.
    ///
    /// This is the FIFTH outcome, and it was the second one found missing. After
    /// `duplicates` was added, k=1 still failed [`Frontier::reconciles`]: 8
    /// generated against 4 accounted. The four unaccounted were the excluded
    /// positions. A level has five exits, not four.
    pub excluded: u64,
    /// Candidates the subset prune removed **without evaluating them against a
    /// single bar**. This is the number that shows the prune is doing its job.
    pub pruned: u64,
    /// Candidates evaluated against the bars and found too rare.
    pub infrequent: u64,
}

impl Frontier {
    /// Every candidate the join produced ended in exactly one of four places.
    ///
    /// `generated == duplicates + pruned + infrequent + frequent`. A level that
    /// does not satisfy this has lost a candidate somewhere, and no report built
    /// on it can be believed.
    #[must_use]
    pub fn reconciles(&self) -> bool {
        let accounted = self
            .duplicates
            .saturating_add(self.excluded)
            .saturating_add(self.pruned)
            .saturating_add(self.infrequent)
            .saturating_add(len_u64(self.frequent.len()));
        accounted == self.generated
    }
}

/// A level that would have enumerated more candidates than the ladder may hold.
///
/// # Why this exists, and why it is not the depth parameter §6 forbids
///
/// It will be read as one, so: a depth parameter says "stop at k=N" and returns a
/// TRUNCATED answer that looks complete. This says "level k wanted more than the
/// ceiling and I refused" and returns the levels below it, which are complete,
/// beside a named reason. `CLAUDE.md` §4 asks for exactly that — degrade loudly
/// and name the reason, or refuse, never both silently — and §6 forbids the
/// other thing. A caller cannot set this to get a shallower ANSWER; it can only
/// set how much memory a single level may occupy before the walk gives up.
///
/// # The hole this closes
///
/// `CLAUDE.md` §6 replaces a depth parameter with extinction: the walk stops
/// where the frequent frontier empties, justified by anti-monotonicity. That
/// argument is sound and it is **not a termination guarantee**. If a set of P
/// positions co-occurs on at least `min_hits` bars, then every subset of it is
/// frequent, [`every_subset_is_frequent`] never prunes, and the frontier at
/// level k is `C(P, k)`. At P = 40 that is 137,846,528,820 itemsets at level 20
/// — 7.7 TB — and correlated bits on a range-bound session are ordinary, not
/// pathological. Before this type the only thing between that and the operator
/// was the allocator, and the failure was a process abort with no message, no
/// partial result and no event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Halt {
    /// The level that breached. Its [`Frontier`] is in [`Sweep::levels`] and is
    /// **partial** — it holds what was evaluated before the ceiling was reached.
    pub k: u32,
    /// Distinct candidates admitted across **every level so far**, not just the
    /// one that breached.
    ///
    /// # This counted one level, and one level was the wrong thing to count
    ///
    /// The first version bounded `seen.len()` per level and this field held that
    /// number. A per-level cap is not a total cap: `Sweep::levels` retains every
    /// level, so the peak is `depth × ceiling` rather than `ceiling`. The gap was
    /// not theoretical — `Ladder::with_min_hits(2)` over 3,000 synthetic bars
    /// with 238 live positions **OOM-killed the process** on the first real
    /// column anyone drove through it, while every per-level check passed.
    ///
    /// The budget is now cumulative, so one number bounds the whole run.
    pub candidates: usize,
    /// The ceiling that was in force, echoed so the result is self-describing.
    pub ceiling: usize,
}

/// The outcome of a whole sweep.
#[derive(Clone, Debug, Default)]
pub struct Sweep {
    /// One entry per level walked, in order. Its length **is** the depth
    /// reached, which is a result and never an input.
    pub levels: Vec<Frontier>,
    /// Positions excluded before k=1, named per D-0080.
    pub excluded: Vec<Excluded>,
    /// Bars loaded. Support fractions are over this.
    pub bars: u64,
    /// The threshold actually applied, echoed so a result is self-describing.
    pub min_hits: u64,
    /// `None` when the ladder went extinct, which is the answer §6 asks for.
    /// `Some` when a level breached the candidate ceiling and the walk stopped
    /// short — see [`Halt`]. A reader that ignores this field reads a partial
    /// sweep as a complete one, which is why [`Sweep::completed`] exists.
    pub halted: Option<Halt>,
}

impl Sweep {
    /// The depth the ladder reached before a level came back empty.
    ///
    /// Zero when no single condition met `min_hits`.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.levels
            .iter()
            .filter(|l| !l.frequent.is_empty())
            .count()
    }

    /// Every frequent combination found at every level.
    pub fn all_frequent(&self) -> impl Iterator<Item = &Itemset> {
        self.levels.iter().flat_map(|l| l.frequent.iter())
    }

    /// Did the ladder go extinct, which is the answer `CLAUDE.md` §6 asks for?
    ///
    /// False means a level breached the candidate ceiling and the walk stopped
    /// short, so the deepest level held is **partial** and combinations exist
    /// that this sweep did not enumerate. A reader that treats a halted sweep as
    /// a complete one is making the claim §3 rule 6 forbids.
    #[must_use]
    pub const fn completed(&self) -> bool {
        self.halted.is_none()
    }
}

/// Distinct candidates a whole walk may admit before it refuses.
///
/// **This was per LEVEL and that was the defect.** `Sweep::levels` retains every
/// level, so a per-level cap left the peak at `depth × ceiling` rather than
/// `ceiling`, and the arithmetic below — which reads "one GiB" — described one
/// level rather than the run. The gap was not theoretical: a first real column,
/// 3,000 bars at `min_hits(2)`, OOM-killed the process while every per-level
/// check passed. The budget is cumulative now, so the number below bounds the
/// whole walk and means what it always claimed to.
///
/// # Where the number comes from
///
/// Bytes, not taste. A level holds each distinct candidate twice: once in `seen`
/// as a [`ConditionMask`] key (48 bytes) and once, if it survives, in `out` as an
/// [`Itemset`] (56 bytes). `hashbrown` carries roughly one slot in eight spare
/// plus a control byte, so 128 bytes per candidate across both is a safe
/// round-up. `2^23 · 128 B = 1 GiB`, which is the largest single level this
/// crate will build on an ordinary machine without the operator having said so.
///
/// It is deliberately far above anything a healthy sweep reaches. With all 238
/// live positions frequent at k=1 — the worst case the vocabulary permits — the
/// distinct-candidate counts are `C(238,2) = 28,203` and `C(238,3) = 2,215,180`,
/// both under this. `C(238,4) = 130,344,865` is fifteen times over it, and that
/// level is the one an adversarial audit measured at 4.69 years of support
/// counting and 24.9 GB of peak memory. So the ceiling first bites exactly where
/// the walk stops being a computation and starts being a hang.
///
/// Both bounds are pinned by
/// `engine::tests::the_default_ceiling_is_the_one_its_arithmetic_describes`,
/// which asserts them in `const` blocks — so moving this constant to a value that
/// puts `C(238,3)` outside it, or `C(238,4)` inside it, fails the **build** and
/// not a test run. The paragraph above is therefore checked arithmetic rather
/// than a comment, which is the whole of what CI gate 12 asks for.
pub const DEFAULT_CEILING: usize = 1 << 23;

/// The ladder. **Carries no depth field**, by `CLAUDE.md` §6.
#[derive(Clone, Copy, Debug)]
pub struct Ladder {
    min_hits: u64,
    ceiling: usize,
}

impl Ladder {
    /// A ladder that keeps combinations hit at least `min_hits` times.
    ///
    /// `docs/00-charter.md` §6 names `min_hits` as the threshold, so it is
    /// sourced rather than invented. It is the *only* knob, and it cannot
    /// silently mean something other than what it says: a count, not a fraction.
    #[must_use]
    pub const fn with_min_hits(min_hits: u64) -> Self {
        // Zero is raised to one, and this is not a convenience.
        //
        // At `min_hits == 0` every candidate satisfies `hits >= 0`, including one with
        // support ZERO. So no level ever comes back empty, the ladder climbs until the
        // mask runs out of bits, and the walk becomes a full powerset enumeration that
        // does not depend on the bars at all — 2^234 candidates, and extinction, the
        // mechanism §6 puts in place of a depth parameter, is simply off.
        //
        // Worse, it contradicts this module's own D-0080 guard, which excludes a
        // POSITION whose support is exactly zero on the argument that a condition
        // matching nothing partitions nothing. Accepting a COMBINATION that matches
        // nothing, in the same walk, is the same claim answered both ways.
        //
        // One is the floor because a frequent set means "occurred", and the smallest
        // number of occurrences that is an occurrence is one.
        let floor = if min_hits == 0 { 1 } else { min_hits };
        Self {
            min_hits: floor,
            ceiling: DEFAULT_CEILING,
        }
    }

    /// The same ladder with a different per-level candidate ceiling.
    ///
    /// A ceiling of zero is raised to one, for the reason
    /// [`Ladder::with_min_hits`] raises `min_hits`: a ceiling of zero refuses
    /// before admitting anything, so every level past k=1 would halt at once and
    /// report a breach that describes the caller rather than the data.
    ///
    /// **This is not a depth control.** See [`Halt`] for why the distinction is
    /// real and not a wording choice.
    #[must_use]
    pub const fn with_ceiling(self, ceiling: usize) -> Self {
        Self {
            min_hits: self.min_hits,
            ceiling: if ceiling == 0 { 1 } else { ceiling },
        }
    }

    /// The per-level candidate ceiling this ladder will actually apply.
    #[must_use]
    pub const fn ceiling(&self) -> usize {
        self.ceiling
    }

    /// The threshold this ladder will actually apply.
    ///
    /// May differ from what was passed: see [`Ladder::with_min_hits`] for why zero is
    /// raised to one. A caller recording a run's parameters should record this, not its
    /// own argument.
    #[must_use]
    pub const fn min_hits(&self) -> u64 {
        self.min_hits
    }

    /// Walk the ladder over a bar-bit column, upward, until a level is empty.
    ///
    /// `bar_bits` is one mask per loaded bar, computed once — §6's "compute the
    /// condition bits per bar, once".
    ///
    /// # Cost, stated rather than implied (§3.6)
    ///
    /// Measured by `C-E-04`, in `crates/engine/benches/ratio.rs`.
    ///
    /// Per candidate the work is one `HashSet` probe (O(1)), up to `k` subset
    /// probes (O(k), and `k` is bounded by the vocabulary width), and one
    /// support count, which walks the bars. Support counting is O(bars) because
    /// it *is* the measurement. The join is O(|F|²) in the previous frontier,
    /// which is Apriori's documented shape and not a hidden scan.
    ///
    /// # Termination
    ///
    /// Each level's masks have `popcount == k`, strictly increasing, and
    /// `popcount` cannot exceed [`ConditionMask::BITS`]. So the loop terminates
    /// even if every candidate were frequent.
    #[must_use]
    pub fn walk(self, bar_bits: &[ConditionMask], live: &[u32]) -> Sweep {
        let bars = len_u64(bar_bits.len());
        let mut sweep = Sweep {
            bars,
            min_hits: self.min_hits,
            ..Sweep::default()
        };

        // ── the layout, chosen ONCE for the whole walk ───────────────────────
        //
        // `column.rs` has existed since b4220b6 and NOTHING CALLED IT. This
        // function took the row-major slice and counted support against it at
        // both of its call sites, so every candidate re-read the entire column
        // -- 58.7 MB at 1,222,791 bars -- while a transposed copy that reads only
        // the `k` bitmaps a candidate names sat one module away, benchmarked and
        // unreachable. `C-E-06` measured the two layouts at 94x, 19x and 9x apart
        // at k=1, k=4 and k=8 and the faster one was never on the path it was
        // written for. That is the same defect as the vocabulary and the ladder
        // never having been joined, one layer down.
        //
        // The transpose is paid once, before k=1. It costs one pass over the bars
        // setting at most `ConditionMask::BITS` bits each -- a constant per bar,
        // so it cannot change the per-bar bound `C-E-04` pins -- and it is repaid
        // by the very first level, which counts support once per live position.
        //
        // The signature still takes the row-major slice: that is what a caller
        // has, and which layout the sweep counts against is this function's
        // business rather than its caller's.
        let column = Column::transpose(bar_bits);

        // ── k=1, and the D-0080 exclusion guard ──────────────────────────────
        // Every position is measured BEFORE the ladder starts, and one at
        // support 0 or at support == bars is named in the output rather than
        // dropped. A support-0 position poisons its whole subtree; a support-1
        // position partitions nothing.
        let mut first: Vec<Itemset> = Vec::with_capacity(live.len());
        // Both counted inside the loop below, so every entry in `live` increments exactly
        // one bucket and `Frontier::reconciles` becomes an invariant of the loop rather
        // than an identity one residual makes true by construction.
        let mut duplicates = 0_u64;
        let mut infrequent = 0_u64;
        let mut offered: HashSet<u32> = HashSet::with_capacity(live.len());
        for &p in live {
            // The caller's list is checked rather than trusted, and each rejection is
            // NAMED. Three ways it can be wrong, all silent before this:
            //
            //   * an index past the mask width. `with_bit` is a no-op there, so the
            //     candidate became the EMPTY mask — which every bar matches — and the
            //     position was reported as a measured always-true condition.
            //   * a void or retired position. `Why::NotLive` existed and was constructed
            //     nowhere in the workspace, so a tombstone was reported as if it had been
            //     measured and found false. §3.8 keeps those indices reserved forever;
            //     sweeping one is sweeping a name, not a condition.
            //   * a duplicate. The same position twice produces the same singleton twice,
            //     and at k=2 a pair of identical bits whose union has popcount 1, which
            //     the join silently drops — so the frontier width no longer matches the
            //     list the caller handed in.
            // Dedup FIRST. It used to run after the liveness check, so a repeated dead
            // position was pushed to `excluded` once per occurrence -- D-0080 requires an
            // excluded position be NAMED, and naming one three times reports three
            // exclusions where there is one position.
            if !offered.insert(p) {
                duplicates = duplicates.saturating_add(1);
                continue;
            }
            let live_position = u16::try_from(p).is_ok_and(vocab::table::is_live);
            if p >= ConditionMask::BITS || !live_position {
                sweep.excluded.push(Excluded {
                    position: p,
                    support: None,
                    reason: Why::NotLive,
                });
                continue;
            }
            let m = ConditionMask::default().with_bit(p);
            let hits = column.support(&m);
            if hits == 0 {
                sweep.excluded.push(Excluded {
                    position: p,
                    support: Some(hits),
                    reason: Why::AlwaysFalse,
                });
            } else if hits == bars {
                sweep.excluded.push(Excluded {
                    position: p,
                    support: Some(hits),
                    reason: Why::AlwaysTrue,
                });
            } else if hits >= self.min_hits {
                first.push(Itemset { mask: m, hits });
            } else {
                // Counted here, not derived afterwards. `infrequent` used to be
                // `generated - frequent - excluded`, which absorbed every silently
                // dropped duplicate and reported it as a condition that HAD been
                // measured against the bars and found too rare. It had never been
                // measured at all.
                infrequent = infrequent.saturating_add(1);
            }
        }
        sort_canonically(&mut first);
        let generated = len_u64(live.len());
        // `duplicates` was hardcoded `0` here, under a comment claiming none were
        // possible because "`live` is deduplicated". `live` is deduplicated BY THIS LOOP,
        // out of a caller list that may contain anything -- so the claim described the
        // output of the code below rather than its input, and the count was wrong whenever
        // it mattered.
        let mut current = Frontier {
            k: 1,
            frequent: first,
            generated,
            duplicates,
            excluded: len_u64(sweep.excluded.len()),
            pruned: 0,
            infrequent,
        };

        // ── k=2 upward, until a level produces nothing ───────────────────────
        // `current` is held by value rather than read back out of `sweep.levels`,
        // so the level is moved into the record exactly once and no clone is
        // needed. The empty level that ends the walk is recorded too — a reader
        // of the output can see that the ladder died rather than was stopped.
        let mut k: u32 = 1;
        // Distinct candidates admitted across every level so far. The budget is
        // cumulative because the peak is -- `sweep.levels` keeps them all.
        let mut admitted: usize = 0;
        while !current.frequent.is_empty() {
            // `saturating_add`, not `checked_add`, and the difference is a branch
            // no test can reach. A level's masks all have `popcount == k` and a
            // popcount cannot exceed `ConditionMask::BITS`, so `k` never passes
            // 385 and the overflow arm of a `checked_add` is dead code — a
            // `break` that cannot execute while this loop is correct, and so a
            // `break` nothing can prove. Saturation agrees with `checked_add` on
            // every reachable value of `k` and adds no arm to defend.
            k = k.saturating_add(1);
            // The budget is CUMULATIVE across levels, not per level. See `Halt`:
            // a per-level cap left the peak at `depth * ceiling`, and the process
            // died rather than refused.
            let (next, halt, added) = self.next_level(&column, &current, k, admitted);
            admitted = admitted.saturating_add(added);
            sweep.levels.push(current);
            current = next;
            // A HALTED LEVEL IS PARTIAL, so climbing off it would build k+1 from
            // an incomplete frontier and label the result complete. Anti-monotonicity
            // only licenses the prune when the previous level is the WHOLE frequent
            // set; from a truncated one the subset test rejects candidates that are
            // frequent, and nothing downstream could tell. So the walk stops, the
            // partial level is recorded below, and `Sweep::halted` names why.
            if halt.is_some() {
                sweep.halted = halt;
                break;
            }
        }
        sweep.levels.push(current);
        sweep
    }

    /// Join the frontier with itself, subset-prune, evaluate what is left.
    fn next_level(
        self,
        column: &Column,
        prev: &Frontier,
        k: u32,
        admitted: usize,
    ) -> (Frontier, Option<Halt>, usize) {
        // O(1) membership for the subset prune, and O(1) duplicate rejection.
        // `ConditionMask` derives `Hash + Eq`, so the key is the mask itself and
        // no separate index is needed.
        let frequent_prev: HashSet<ConditionMask> = prev.frequent.iter().map(|i| i.mask).collect();
        // Pre-sized, because gate 11 rule 3 is right that an unsized map is a
        // rehash the caller did not ask for. The floor is the previous level's
        // survivor count: the join emits at least that many candidates before any
        // are pruned. The true count is |F|^2/2, and reserving THAT is the
        // allocation `docs/06-limits.md` §5 is about -- so this reserves the floor
        // rather than the ceiling, and says which.
        let mut seen: HashSet<ConditionMask> = HashSet::with_capacity(frequent_prev.len());
        // PRE-SIZED, and it was not. `Vec::new()` here meant the survivor vector
        // grew by doubling through every level, while its neighbour above carried
        // a comment explaining why pre-sizing matters -- gate 11 rule 3, "an
        // unsized map is a rehash the caller did not ask for", applied to one of
        // the two collections and not the other. An audit found the gap.
        //
        // The floor, not the ceiling, for the reason `seen` uses the same one:
        // the join emits at least `|F|` candidates before any are pruned, and
        // reserving the true `|F|^2/2` is the allocation `docs/06-limits.md` §5 is
        // about. Capped at the ladder's own ceiling so a huge frontier cannot
        // reserve past what a level is allowed to hold anyway.
        let mut out: Vec<Itemset> = Vec::with_capacity(frequent_prev.len().min(self.ceiling));
        let mut generated: u64 = 0;
        let mut duplicates: u64 = 0;
        let mut pruned: u64 = 0;
        let mut infrequent: u64 = 0;
        let mut halted: Option<Halt> = None;

        'join: for (a_idx, a) in prev.frequent.iter().enumerate() {
            for b in prev.frequent.iter().skip(a_idx.saturating_add(1)) {
                let cand = a.mask.union(&b.mask);
                // A union of two distinct (k-1)-sets is a k-set only when they
                // share exactly k-2 bits. Checking popcount is the same test and
                // needs no canonical-prefix bookkeeping.
                if cand.popcount() != k {
                    continue;
                }
                // THE CEILING, AND IT IS CHECKED BEFORE ANY COUNTER MOVES.
                //
                // Placement is the whole correctness argument. Guarding `out.len()`
                // would guard the wrong number: `seen` is filled BEFORE the support
                // test, so a level whose survivors all fall under `min_hits` still
                // allocates every distinct candidate it enumerated -- the level that
                // goes extinct is the level that allocates most. `seen` is the
                // allocation, so `seen` is what is bounded.
                //
                // Checking here, rather than after `seen.insert`, keeps
                // `Frontier::reconciles` an invariant: nothing half-processed is
                // ever counted. The cost is that a level which has admitted exactly
                // `ceiling` distinct candidates halts even if every remaining pair
                // would have been a duplicate. That is conservative in the safe
                // direction and it is stated rather than hidden.
                if admitted.saturating_add(seen.len()) >= self.ceiling {
                    halted = Some(Halt {
                        k,
                        candidates: admitted.saturating_add(seen.len()),
                        ceiling: self.ceiling,
                    });
                    break 'join;
                }
                generated = generated.saturating_add(1);
                // Duplicate rejection: O(1). The same k-set arises from several
                // pairs and must be evaluated once.
                if !seen.insert(cand) {
                    duplicates = duplicates.saturating_add(1);
                    continue;
                }
                // Subset prune, justified by anti-monotonicity: a bar matches a
                // mask iff every bit is set, so adding a bit can only remove
                // hits. If any (k-1)-subset is infrequent the k-set cannot be
                // frequent, and it is never evaluated against a single bar.
                if !every_subset_is_frequent(&cand, &frequent_prev) {
                    pruned = pruned.saturating_add(1);
                    continue;
                }
                let hits = column.support(&cand);
                if hits >= self.min_hits {
                    out.push(Itemset { mask: cand, hits });
                } else {
                    infrequent = infrequent.saturating_add(1);
                }
            }
        }
        sort_canonically(&mut out);
        (
            Frontier {
                k,
                frequent: out,
                generated,
                duplicates,
                excluded: 0,
                pruned,
                infrequent,
            },
            halted,
            seen.len(),
        )
    }
}

/// How many bars the mask matches.
///
/// One [`ConditionMask::hits`] per bar and nothing else — no allocation, no
/// early exit, no branch on the answer.
#[must_use]
pub fn support(bar_bits: &[ConditionMask], mask: &ConditionMask) -> u64 {
    bar_bits.iter().fold(
        0_u64,
        |n, b| if b.hits(mask) { n.saturating_add(1) } else { n },
    )
}

/// True when every (k−1)-subset of `cand` is in the previous frequent frontier.
///
/// Walks the whole 384-bit width rather than the set bits, because
/// [`ConditionMask`] exposes no bit iterator. The loop bound is
/// Measured by `C-E-02`: the per-bar cost is flat from k=1 to k=8, which is what
/// says walking the full width costs the same whatever the candidate requires.
/// `crates/engine/benches/ratio.rs`.
///
/// [`ConditionMask::BITS`], a compile-time constant, so this is O(1) under
/// §3.4 — but it is 384 probes where 5 would do, and that is a real cost stated
/// rather than hidden. A `set_bits()` accessor on `ConditionMask` would make it
/// O(k); it does not exist and adding one is a change to `crates/vocab`.
fn every_subset_is_frequent(cand: &ConditionMask, frequent: &HashSet<ConditionMask>) -> bool {
    let mut b: u32 = 0;
    while b < ConditionMask::BITS {
        if cand.get(b) && !frequent.contains(&cand.without_bit(b)) {
            return false;
        }
        b = b.saturating_add(1);
    }
    true
}

/// Order by the mask's words, then by hits.
///
/// `[u64; WORDS]` has a total order, so this is stable across processes and
/// machines — which `CLAUDE.md` §3.5 requires, since a `HashSet`'s iteration
/// order is randomised per process and must never reach the output.
fn sort_canonically(v: &mut [Itemset]) {
    v.sort_unstable_by_key(|i| (i.mask.words(), i.hits));
}

/// `usize` count as `u64` without a cast lint or a panic.
/// A collection's length, as a `u64`.
///
/// # This was a fold, and the fold was a rule breach
///
/// It read `it.fold(0, |n, _| n + 1)` over an iterator — O(n) where `.len()` is
/// O(1). An adversarial audit measured the difference over a 1,222,791-element
/// slice: **612,083 ns against 0 ns**. It ran once per walk rather than once per
/// candidate, so the cost was 0.6 ms and not a hot loss, but `CLAUDE.md` §3.4
/// does not grade a scan by how often it happens — "a change that makes one of
/// them scan fails the bench gate", and no bench row covered this one.
///
/// `try_from` rather than a cast, because `cast_possible_truncation` is denied
/// workspace-wide; the `Err` arm cannot be reached on any 64-bit target and
/// lives inside `core`, so it leaves no uncoverable region in this crate.
fn len_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bars whose bits are given as position lists.
    fn bars(spec: &[&[u32]]) -> Vec<ConditionMask> {
        spec.iter()
            .map(|bits| {
                bits.iter()
                    .fold(ConditionMask::default(), |m, &b| m.with_bit(b))
            })
            .collect()
    }

    /// **E-02 — the completeness proof.** Apriori's kept set equals brute force's,
    /// exhaustively, over every bar column of a small vocabulary.
    ///
    /// # Why this test is the one that matters
    ///
    /// Anti-monotonicity is the ARGUMENT that pruning is safe: `support(A) >=
    /// support(A + b)`, so a frequent set can never have an infrequent subset, so
    /// discarding the supersets of an infrequent set cannot discard anything
    /// frequent. `support_never_increases_as_bits_are_added` proves the premise.
    ///
    /// It does not prove the IMPLEMENTATION. A join that skipped a pair, a prune
    /// that tested the wrong subsets, a dedup that swallowed a distinct
    /// combination — each of those loses frequent sets while every
    /// anti-monotonicity test still passes, because the property holds of the
    /// relation whatever the code does with it.
    ///
    /// So this enumerates all 2^p - 1 non-empty combinations directly, counts each
    /// one's support with `support`, and requires the two kept sets to be EQUAL as
    /// sets. Not "Apriori found at least as many" and not "the counts agree" —
    /// equal, so a missing combination and an invented one both fail.
    ///
    /// Exhaustive over inputs, not sampled: 4 positions and 6 bars is 2^4 = 16
    /// possible bar values and every one of `2^24` columns would be too many, so
    /// the columns are enumerated as all 4^6 = 4,096 assignments of six bars drawn
    /// from four hand-picked bit patterns, at three thresholds. 12,288 sweeps.
    #[test]
    fn the_apriori_kept_set_equals_the_brute_force_kept_set() {
        const P: u32 = 4;
        let live: Vec<u32> = (0..P).collect();

        // Four bit patterns over four positions, chosen to include the empty bar
        // (which supports nothing), two overlapping pairs, and a full bar.
        let shapes: [&[u32]; 4] = [&[], &[0, 1], &[1, 2], &[0, 1, 2, 3]];

        let mut columns = 0_u32;
        for assignment in 0..4_usize.pow(6) {
            let column: Vec<ConditionMask> = (0..6)
                .map(|slot| {
                    let pick = (assignment / 4_usize.pow(slot)) % 4;
                    shapes
                        .get(pick)
                        .copied()
                        .unwrap_or(&[])
                        .iter()
                        .fold(ConditionMask::default(), |m, &b| m.with_bit(b))
                })
                .collect();

            for min_hits in [1_u64, 2, 4] {
                // Brute force: every non-empty combination of the four positions,
                // counted directly. No pruning, no join, no ladder.
                let mut brute: Vec<ConditionMask> = Vec::new();
                for subset in 1_u32..(1 << P) {
                    let mask =
                        live.iter()
                            .enumerate()
                            .fold(ConditionMask::default(), |m, (i, &bit)| {
                                if subset & (1 << i) == 0 {
                                    m
                                } else {
                                    m.with_bit(bit)
                                }
                            });
                    if support(&column, &mask) >= min_hits {
                        brute.push(mask);
                    }
                }

                let sweep = Ladder::with_min_hits(min_hits).walk(&column, &live);
                let mut kept: Vec<ConditionMask> = sweep.all_frequent().map(|i| i.mask).collect();

                // A position excluded as always-true or always-false never enters
                // the ladder, by design (`Why::AlwaysTrue` / `Why::AlwaysFalse`),
                // so brute force must drop the same combinations rather than the
                // comparison reporting a difference the design intends.
                let excluded: Vec<u32> = sweep.excluded.iter().map(|e| e.position).collect();
                brute.retain(|m| !excluded.iter().any(|&b| m.get(b)));

                kept.sort_unstable_by_key(ConditionMask::words);
                brute.sort_unstable_by_key(ConditionMask::words);

                // Counted before the comparison rather than inside its failure
                // message. A message argument only runs when the assertion fails,
                // so `kept.len()` there is an expression no passing run executes;
                // hoisted, the numbers are the same and the message keeps them.
                let kept_count = kept.len();
                let brute_count = brute.len();
                assert_eq!(
                    kept, brute,
                    "column {assignment} at min_hits {min_hits}: the ladder kept \
                     {kept_count} set(s) and brute force kept {brute_count}. \
                     Apriori is only sound if these are EQUAL -- a shortfall is a \
                     missed combination and a surplus is an invented one, and \
                     excluded positions ({excluded:?}) are removed from both sides \
                     before comparing.",
                );
                columns = columns.saturating_add(1);
            }
        }
        assert_eq!(
            columns, 12_288,
            "every column at every threshold must have been compared"
        );
    }

    /// **E-07 — §3 rule 5.** The same inputs produce the same output, twice over.
    ///
    /// Idempotence is not a property of the algorithm here, it is a property of the
    /// implementation: a `HashSet` iterated for output order would produce a
    /// different ranking on every process, because Rust's default hasher is seeded
    /// per process. `the_order_of_the_output_does_not_depend_on_hash_iteration_order`
    /// checks that within one run. This checks it across two independent walks,
    /// which is the form §3 rule 5 actually states — "Same inputs, same outputs,
    /// byte for byte. Reruns are safe."
    ///
    /// # Why positions 3 and 9 are in the fixture
    ///
    /// They are the exclusions, and without them this test did not check the half
    /// of `render` that prints them. 9 is set on every bar and 3 on none, so the
    /// D-0080 guard names both, `Sweep::excluded` is non-empty, and the closure's
    /// exclusion loop actually runs. Before they were added the loop body never
    /// executed once: two walks that disagreed **only** in their exclusions —
    /// a different order, a different reason, a missing entry — rendered
    /// identically and this test reported them equal.
    #[test]
    fn a_rerun_with_identical_inputs_produces_an_identical_sweep() {
        let column = bars(&[
            &[0, 1, 5, 9],
            &[0, 1, 9],
            &[1, 5, 9],
            &[0, 5, 9],
            &[0, 1, 5, 9],
            &[2, 9],
            &[0, 9],
            &[1, 9],
        ]);
        let live = [0_u32, 1, 2, 3, 5, 9];

        let first = Ladder::with_min_hits(2).walk(&column, &live);
        let second = Ladder::with_min_hits(2).walk(&column, &live);

        // Rendered to text and compared as text, because that is what "byte for
        // byte" means for a result a human or a file will see. Comparing the
        // structs would miss an ordering difference inside a field that happens to
        // hold a set.
        let render = |s: &Sweep| -> String {
            use core::fmt::Write as _;
            let mut out = format!("bars {} min_hits {}\n", s.bars, s.min_hits);
            for level in &s.levels {
                let _ = writeln!(
                    out,
                    "k {} generated {} duplicates {} excluded {} pruned {} infrequent {}",
                    level.k,
                    level.generated,
                    level.duplicates,
                    level.excluded,
                    level.pruned,
                    level.infrequent
                );
                for set in &level.frequent {
                    let _ = writeln!(out, "  {:?} hits {}", set.mask.words(), set.hits);
                }
            }
            for e in &s.excluded {
                let shown = e
                    .support
                    .map_or_else(|| "unmeasured".to_owned(), |n| n.to_string());
                let _ = writeln!(out, "excluded {} {shown} {:?}", e.position, e.reason);
            }
            out
        };

        let text = render(&first);
        assert_eq!(
            text,
            render(&second),
            "two walks over identical inputs rendered differently, so the output \
             depends on something outside the inputs -- CLAUDE.md §3 rule 5"
        );
        // The comparison above is only as wide as what `render` prints. These two
        // lines are the proof that it prints the exclusions at all: position 3 is
        // false on every bar and 9 is true on every bar, so both must appear with
        // their measured support and their reason.
        assert!(
            text.contains("excluded 3 0 AlwaysFalse") && text.contains("excluded 9 8 AlwaysTrue"),
            "`render` did not name the two excluded positions, so a rerun could \
             differ in its exclusions and still compare equal:\n{text}"
        );
        assert!(
            first.levels.iter().all(Frontier::reconciles),
            "a rerun that agrees with itself is worthless if neither walk was sound"
        );
    }

    /// **V-06 — bits are computed once, and the sweep CANNOT recompute them.**
    ///
    /// The invariant is "bits are computed exactly once per slice". The obvious
    /// proof is a counting spy: wrap the indicator layer, run a sweep, assert the
    /// call count equals the bar count. That proves the code as written does not
    /// recompute. It does not stop the next edit from doing so.
    ///
    /// This proves the stronger thing. `Ladder::walk` takes `&[ConditionMask]` —
    /// bits that already exist — and `crates/engine` declares one dependency,
    /// `vocab`, which holds the mask type and the bit table but computes nothing
    /// from a candle. `crates/indicators` is the only crate that turns a bar into a
    /// bit, and this crate cannot see it. So there is no expression in the sweep
    /// that could recompute a condition, and no counting is required.
    ///
    /// Checked against the manifest rather than asserted in prose, so adding the
    /// arrow fails this test rather than passing review.
    /// Every crate this manifest declares a dependency on, however it is spelled.
    ///
    /// # Why this is a parser and not a substring scan
    ///
    /// It WAS a substring scan over the raw text between `[dependencies]` and the
    /// next `\n[`, and that scan was wrong in both directions at once.
    ///
    /// False positive: comments are inside the table. A commit adding the sentence
    /// "`crates/indicators` had always written it the other way" to a comment in
    /// that table turned this test red while the dependency list had not moved.
    /// That is the same defect `vocab::mask::hits_does_the_same_work_for_every_input`
    /// was fixed for one commit earlier -- text in a comment is text.
    ///
    /// False negative, and far worse: cargo accepts four spellings of one
    /// dependency and the scan could see exactly one.
    ///
    /// ```text
    /// [dependencies]
    /// store = { path = "../store" }        the only form the scan saw
    /// store.path = "../store"              a dotted key, same meaning
    ///
    /// [dependencies.store]                 a table header, same meaning
    /// path = "../store"
    ///
    /// [dev-dependencies]                   links into every test and bench
    /// [target.'cfg(unix)'.dependencies]    links on that target
    /// ```
    ///
    /// An audit confirmed by running it that one `[dependencies.store]` stanza
    /// defeats this test, gate 22 clause A, gate 9, gate 9b, gate 21 clause A and
    /// the four graph tests in `crates/core/tests/graph.rs` -- nine dependency
    /// guarantees sharing one parser shape, and one bypass for all of them.
    ///
    /// The one limit, stated: a `#` inside a quoted value would be read as a
    /// comment. No manifest in this workspace has one, and a dependency name
    /// cannot contain one.
    fn declared_dependencies(manifest: &str) -> Vec<String> {
        /// Are this table's KEYS dependency names?
        fn keys_are_dependencies(table: &str) -> bool {
            matches!(
                table.rsplit('.').next().unwrap_or(""),
                "dependencies" | "dev-dependencies" | "build-dependencies"
            )
        }
        /// Does this table's HEADER name a dependency, as `[dependencies.store]` does?
        fn named_by_header(table: &str) -> Option<&str> {
            let mut segments = table.rsplit('.');
            let leaf = segments.next()?;
            let parent = segments.next()?;
            matches!(
                parent,
                "dependencies" | "dev-dependencies" | "build-dependencies"
            )
            .then_some(leaf)
        }

        let mut found: Vec<String> = Vec::new();
        let mut table = String::new();
        for raw in manifest.lines() {
            let line = raw.split_once('#').map_or(raw, |(code, _)| code).trim();
            if line.is_empty() {
                continue;
            }
            if let Some(inner) = line.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
                table = inner
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .trim()
                    .to_owned();
                if let Some(name) = named_by_header(&table) {
                    found.push(name.to_owned());
                }
                continue;
            }
            if !keys_are_dependencies(&table) {
                continue;
            }
            let key = line.split('=').next().unwrap_or("").trim();
            let name = key.split('.').next().unwrap_or("").trim();
            if !name.is_empty() {
                found.push(name.to_owned());
            }
        }
        found.sort();
        found.dedup();
        found
    }

    #[test]
    fn the_sweep_cannot_compute_a_condition_bit() {
        // Exact, not "contains no forbidden name". A list of things that must be
        // absent is only as good as the list; an equality says what IS there, so a
        // dependency nobody thought to forbid fails it too.
        assert_eq!(
            declared_dependencies(include_str!("../Cargo.toml")),
            ["vocab"],
            "`crates/engine` must declare `vocab` and nothing else. V-06's whole \
             argument is that the sweep cannot recompute a condition bit because it \
             cannot reach the code that computes one -- `crates/indicators` is the \
             only crate that turns a bar into a bit. A new arrow here needs a \
             decisions entry AND a different proof of V-06."
        );

        // The parser is checked against the four spellings it exists for, because a
        // parser nothing tests is the previous version of this test.
        assert_eq!(
            declared_dependencies("[dependencies]\nstore = { path = \"../store\" }"),
            ["store"],
            "the inline-table spelling"
        );
        assert_eq!(
            declared_dependencies("[dependencies]\nstore.path = \"../store\""),
            ["store"],
            "the dotted-key spelling"
        );
        assert_eq!(
            declared_dependencies("[dependencies.store]\npath = \"../store\""),
            ["store"],
            "the table-header spelling — the bypass that defeated nine guarantees"
        );
        assert_eq!(
            declared_dependencies("[target.'cfg(unix)'.dev-dependencies]\nstore = \"1\""),
            ["store"],
            "a dev-dependency behind a target predicate still links into every test"
        );
        assert_eq!(
            declared_dependencies("[dependencies]\n# store = { path = \"../store\" }"),
            [] as [&str; 0],
            "a commented-out dependency is not a dependency"
        );

        // And the entry point takes bits, not bars. A signature change to accept
        // candles would make recomputation constructible again.
        let src = include_str!("lib.rs");
        assert!(
            src.contains("pub fn walk(self, bar_bits: &[ConditionMask], live: &[u32])"),
            "`walk`'s signature changed. It must take already-computed masks: a \
             sweep that accepted bars could compute a bit per candidate, which is \
             exactly what V-06 forbids."
        );
    }

    #[test]
    fn the_type_carries_no_depth_field() {
        // CLAUDE.md §6: "There is no k parameter. Not a default, not a token,
        // not an environment override. The type does not carry the field."
        //
        // THE SIZE WAS THE WHOLE TEST, AND THE SIZE WAS ONLY EVER A PROXY. It
        // read "a Ladder is exactly one u64, so there is nowhere for a depth to
        // hide", which was true while one field existed and stopped being an
        // argument the moment a second one could be justified. A size check
        // notices that a field ARRIVED; it can never ask what the field does.
        //
        // The size is still pinned, derived from the two fields rather than
        // written as a literal, so a THIRD field fails here and has to be
        // argued for in this comment before it can compile.
        assert_eq!(
            core::mem::size_of::<Ladder>(),
            core::mem::size_of::<u64>() + core::mem::size_of::<usize>()
        );
    }

    /// The behavioural half of §6, and the half a size assertion cannot reach.
    ///
    /// A depth parameter CHANGES the depth — that is what makes it one, and what
    /// made the predecessor's silent fallback to `k = [1, 2]` invisible. A memory
    /// ceiling does not: across every ceiling that does not bite, the walk reaches
    /// the same depth and returns the same combinations, byte for byte.
    #[test]
    fn the_ceiling_cannot_choose_a_depth() {
        let b = bars(&[&[0, 1], &[0, 1], &[0, 1], &[2], &[2], &[0]]);
        let roomy = Ladder::with_min_hits(2).walk(&b, &[0, 1, 2]);
        let tight = Ladder::with_min_hits(2)
            .with_ceiling(64)
            .walk(&b, &[0, 1, 2]);

        assert!(
            roomy.completed() && tight.completed(),
            "neither should bite"
        );
        assert_eq!(roomy.depth(), tight.depth(), "a ceiling is not a depth");
        let wide: Vec<Itemset> = roomy.all_frequent().copied().collect();
        let narrow: Vec<Itemset> = tight.all_frequent().copied().collect();
        assert_eq!(wide, narrow, "and it changes no combination either");
    }

    /// The hole this closes: a frontier that never empties.
    ///
    /// Four positions that co-occur on most bars. Every subset of a frequent set
    /// is frequent, so `every_subset_is_frequent` never prunes and extinction —
    /// §6's entire replacement for a depth parameter — never happens. Before the
    /// ceiling the only thing under this was the allocator.
    #[test]
    fn a_level_that_would_outgrow_the_ceiling_halts_loudly() {
        let b = bars(&[&[0, 1, 2, 3], &[0, 1, 2, 3], &[0, 1, 2, 3], &[4]]);
        let s = Ladder::with_min_hits(1)
            .with_ceiling(2)
            .walk(&b, &[0, 1, 2, 3, 4]);

        assert!(
            !s.completed(),
            "a partial sweep must never report itself whole"
        );
        // `unwrap_or_default` and not `expect`: this crate denies both
        // `expect_used` and `panic` in tests as well as in shipping code, and the
        // assertion above already proves the `Some`. A defaulted `Halt` is all
        // zeroes, so every field assertion below still fails loudly if it were not.
        let halt = s.halted.unwrap_or_default();
        assert_eq!(halt.ceiling, 2, "the ceiling in force is echoed");
        assert_eq!(halt.candidates, 2, "and so is what it admitted");
        assert_eq!(halt.k, 2, "the level that breached is named");
        // The partial level is KEPT, not discarded: everything below it is
        // complete and a caller paid for it.
        assert!(s.levels.iter().any(|l| l.k == halt.k));
    }

    /// A halted level has lost no candidate — it simply stopped admitting them.
    #[test]
    fn a_halted_level_still_reconciles() {
        let b = bars(&[&[0, 1, 2, 3], &[0, 1, 2, 3], &[0, 1, 2, 3], &[4]]);
        let s = Ladder::with_min_hits(1)
            .with_ceiling(2)
            .walk(&b, &[0, 1, 2, 3, 4]);
        assert!(s.halted.is_some(), "the fixture must actually breach");
        for level in &s.levels {
            assert!(level.reconciles(), "level {} lost a candidate", level.k);
        }
    }

    /// Extinction is silent, and that silence is the positive result.
    #[test]
    fn a_ladder_that_goes_extinct_reports_no_halt() {
        let b = bars(&[&[0, 1], &[0, 1], &[0, 1], &[2], &[2], &[0]]);
        let s = Ladder::with_min_hits(2).walk(&b, &[0, 1, 2]);
        assert!(s.completed());
        assert_eq!(s.halted, None, "nothing breached, so nothing is named");
    }

    /// The sentinel `unwrap_or_default` leans on, pinned.
    ///
    /// `a_level_that_would_outgrow_the_ceiling_halts_loudly` reads its `Halt`
    /// through `unwrap_or_default`, which is only sound because a defaulted
    /// `Halt` is a value no real breach can produce — `k` is at least 2 and both
    /// counts are at least 1 whenever one is constructed. Without this test the
    /// `Default` impl is also a function no test enters, and llvm-cov counts it.
    #[test]
    fn a_defaulted_halt_is_a_value_no_breach_can_produce() {
        let d = Halt::default();
        assert_eq!(d.k, 0, "no level is k=0");
        assert_eq!(d.candidates, 0, "a breach admitted at least one candidate");
        assert_eq!(d.ceiling, 0, "and ran under a ceiling of at least one");
        assert_ne!(
            d,
            Halt {
                k: 2,
                candidates: 2,
                ceiling: 2
            }
        );
    }

    /// A silent depth cap is caught, which `the_ceiling_cannot_choose_a_depth`
    /// alone did not do.
    ///
    /// An adversarial audit injected `if k > 4 { break; }` into `walk` — a
    /// hardcoded, silent truncation returning a sweep whose `completed()` is
    /// still true, which is precisely the §6-forbidden thing and precisely the
    /// predecessor's `k = [1, 2]` failure. **All 37 tests passed.** The fixture
    /// in that test reaches depth 2, so it cannot observe a cap at 3 or above.
    ///
    /// Eight positions co-occurring on three bars makes every subset frequent,
    /// so extinction happens at k=9 and the ladder must report depth 8 with
    /// exactly `2^8 - 1 = 255` subsets. Any cap below 8 changes both numbers.
    ///
    /// **Position 6 is not among them, and that is not arbitrary.** It is a
    /// retired tombstone, so `walk` excludes it before k=1 under D-0080 and the
    /// first draft of this fixture reached depth 7 with seven live bits while
    /// claiming eight. The ninth position exists only so the other eight are not
    /// set on *every* bar — at support == bars they would all be excluded as
    /// `AlwaysTrue` instead — and it contributes its own k=1 singleton, which is
    /// why the count is 256 rather than 255.
    #[test]
    fn a_silently_capped_depth_would_be_caught() {
        let deep: &[u32] = &[0, 1, 2, 3, 4, 5, 7, 8];
        let b = bars(&[deep, deep, deep, &[9], &[9]]);
        let s = Ladder::with_min_hits(2).walk(&b, &[0, 1, 2, 3, 4, 5, 7, 8, 9]);

        assert!(s.completed(), "nothing should breach the default ceiling");
        assert_eq!(s.depth(), 8, "eight co-occurring live bits must reach k=8");
        assert_eq!(
            s.all_frequent().count(),
            256,
            "every non-empty subset of the eight, plus the ninth's singleton"
        );
        assert!(
            s.levels.last().is_some_and(|l| l.frequent.is_empty()),
            "the ladder must die of extinction, not of a cap"
        );
    }

    /// The transpose stays wired into the sweep.
    ///
    /// # Why this is a source check and not a behavioural one
    ///
    /// The two layouts are answer-equivalent by construction — that is the whole
    /// claim of `the_two_layouts_agree_on_every_candidate` — so **no test of the
    /// output can tell them apart.** An audit proved it: reverting the k=1 site
    /// to the row-major free function, which un-does the cheaper walk entirely,
    /// survives every behavioural test AND gate 8. (The spelling of that call is
    /// deliberately not repeated in this paragraph — it would be counted below.)
    /// `C-E-04` is a ratio row that read `ok` before the wiring and `ok` after,
    /// and `C-E-06` benchmarks the two layouts against each other directly rather
    /// than through `walk`. Nothing in the repository would have noticed.
    ///
    /// So the guard has to be structural, which is the shape
    /// `the_sweep_cannot_compute_a_condition_bit` already uses. The needles are
    /// assembled with `concat!` because a literal spelling of them would appear
    /// in this file and count itself.
    #[test]
    fn the_sweep_counts_support_against_the_transposed_column() {
        let src = include_str!("lib.rs");
        assert_eq!(
            src.matches(concat!("column.", "support(&")).count(),
            2,
            "both support sites in the sweep -- k=1 and the level join -- must \
             read the transposed column. One of them has gone back to the \
             row-major walk, which is 9x more bytes moved per candidate and which \
             no behavioural test can see."
        );
        assert!(
            src.contains(concat!("Column::", "transpose(bar_bits)")),
            "the walk must build the transposed layout once, before k=1"
        );
    }

    /// The budget spans the whole walk, and a per-level cap would not have.
    ///
    /// # The arithmetic that makes this test able to fail
    ///
    /// Eight co-occurring live bits plus a ninth that partitions them. The k=1
    /// frontier is nine positions, so the join at k=2 admits `C(9,2) = 36`
    /// distinct candidates — `seen` counts before the frequency test, so the
    /// eight pairs containing the ninth position are admitted and then found
    /// infrequent. 28 survive. k=3 admits `C(8,3) = 56`; k=4 would admit
    /// `C(8,4) = 70`.
    ///
    /// **No single level reaches 100.** A per-level ceiling of 100 therefore
    /// never fires, the walk runs to extinction at depth 8, and it holds 247
    /// candidates in total on the way. That is precisely the shape that
    /// OOM-killed a real 3,000-bar run while every per-level check passed.
    ///
    /// Cumulatively: 36 after k=2, 92 after k=3, and the budget is spent eight
    /// candidates into k=4.
    #[test]
    fn the_budget_is_cumulative_and_a_per_level_cap_would_miss_it() {
        let deep: &[u32] = &[0, 1, 2, 3, 4, 5, 7, 8];
        let b = bars(&[deep, deep, deep, &[9], &[9]]);
        let live = [0, 1, 2, 3, 4, 5, 7, 8, 9];

        let capped = Ladder::with_min_hits(2).with_ceiling(100).walk(&b, &live);
        assert!(
            !capped.completed(),
            "a per-level cap of 100 never fires here -- only a total one does"
        );
        let halt = capped.halted.unwrap_or_default();
        assert_eq!(halt.ceiling, 100);
        assert_eq!(halt.candidates, 100, "the TOTAL is what breached");
        assert_eq!(halt.k, 4, "36 at k=2, 92 at k=3, spent early in k=4");

        // And the same ladder with room runs to extinction, so the budget is a
        // refusal and never a depth.
        let roomy = Ladder::with_min_hits(2)
            .with_ceiling(10_000)
            .walk(&b, &live);
        assert!(roomy.completed());
        assert_eq!(roomy.depth(), 8);
    }

    #[test]
    fn a_zero_ceiling_is_raised_to_one() {
        // Same reason zero `min_hits` is raised: a ceiling of zero refuses before
        // admitting anything, so every level past k=1 reports a breach that
        // describes the caller rather than the data.
        assert_eq!(Ladder::with_min_hits(1).with_ceiling(0).ceiling(), 1);
        assert_eq!(Ladder::with_min_hits(1).with_ceiling(7).ceiling(), 7);
    }

    #[test]
    fn the_default_ceiling_is_the_one_its_arithmetic_describes() {
        assert_eq!(Ladder::with_min_hits(1).ceiling(), DEFAULT_CEILING);
        assert_eq!(DEFAULT_CEILING, 8_388_608, "2^23, one GiB at 128 B each");
        // The doc's claim that a healthy sweep never reaches the ceiling, pinned
        // at COMPILE time rather than run time. Both operands are constants, so a
        // runtime assertion would only ever restate what the compiler already
        // knew; a `const` block fails the BUILD if the ceiling is ever moved to a
        // value that puts C(238,3) outside it or C(238,4) inside it, which is the
        // arithmetic the doc block on `DEFAULT_CEILING` argues from.
        const { assert!(2_215_180 < DEFAULT_CEILING, "C(238,3) must fit") };
        const { assert!(130_344_865 > DEFAULT_CEILING, "C(238,4) must not") };
    }

    #[test]
    fn depth_is_reached_by_extinction_and_not_by_a_caller() {
        // 0 and 1 co-occur on 3 bars, 2 only ever alone. So k=1 keeps three
        // positions, k=2 keeps {0,1} only, and k=3 must come back empty.
        let b = bars(&[&[0, 1], &[0, 1], &[0, 1], &[2], &[2], &[0]]);
        let s = Ladder::with_min_hits(2).walk(&b, &[0, 1, 2]);
        assert_eq!(s.depth(), 2, "the ladder should die at k=3");
        assert_eq!(s.levels.len(), 3, "the empty level is recorded, not hidden");
        assert!(s.levels.last().is_some_and(|l| l.frequent.is_empty()));
    }

    /// The emitted order is canonical MASK order, and it is not support order.
    ///
    /// The module doc said this crate "refuses to order them by anything but support",
    /// which reads as a promise that the output IS support-ordered. It is not, and cannot
    /// be: `sort_canonically` keys on `(mask.words(), hits)`, `seen` makes every mask
    /// unique within a level, so the `[u64; 6]` primary key is unique and the `hits`
    /// tiebreak can never fire.
    ///
    /// That is the right behaviour -- §3.1 blocks inventing a ranking metric, and support
    /// is a frequency rather than an edge -- but the doc had to say what the code does.
    /// This pins it, so the corrected wording is a mechanism rather than a second sentence.
    ///
    /// A high bit sorts FIRST whenever the low words are zero, which is the opposite of
    /// what a reader expects, and the fixture is chosen to show exactly that.
    #[test]
    fn the_emitted_order_is_canonical_mask_order_and_not_support_order() {
        // bit 279 lives in word 4, so its word 0 is zero and it sorts ahead of bits 0..3.
        let b = bars(&[&[0, 1, 279], &[1, 2, 279], &[1, 2, 279], &[1], &[3]]);
        let s = Ladder::with_min_hits(1).walk(&b, &[0, 1, 2, 3, 279]);
        let first = s.levels.first();
        assert!(first.is_some(), "k=1 always runs");
        let level = first.cloned().unwrap_or_default();

        let words: Vec<[u64; 6]> = level.frequent.iter().map(|i| i.mask.words()).collect();
        let mut sorted = words.clone();
        sorted.sort_unstable();
        assert_eq!(
            words, sorted,
            "the frequent sets must come out in ascending word order, which is what makes a \
             rerun byte-identical under §3.5"
        );

        let supports: Vec<u64> = level.frequent.iter().map(|i| i.hits).collect();
        assert_eq!(
            supports,
            vec![3, 1, 4, 2, 1],
            "bit 279 first with support 3, then bits 0..3 with 1, 4, 2, 1. If this ever \
             reads as sorted, the ordering contract changed and the module doc is stale."
        );
        let mut ascending = supports.clone();
        ascending.sort_unstable();
        assert_ne!(
            supports, ascending,
            "the output is NOT support-ordered, and the module doc must not imply it is"
        );
    }

    /// Every offer at k=1 lands in exactly one bucket, and a duplicate is not "too rare".
    ///
    /// # Three witnesses, two of them from an adversarial audit
    ///
    /// `live = [0, 0, 1, 1, 1]` over bars `[{0,1}, {0,1}, {0}, {1}]` at `min_hits = 1`.
    /// The engine reported `generated 5, duplicates 0, infrequent 3, frequent 2`. The
    /// truth is two frequent singletons, ZERO infrequent, and three duplicate offers.
    /// `infrequent` was `generated - frequent - excluded`, so the three dropped duplicates
    /// landed in it and were reported as conditions that had been measured against the
    /// bars and found too rare. Not one of them was ever measured.
    ///
    /// `live = [0, 1, D, D, D]` with `D` a tombstone. The engine emitted three identical
    /// `Excluded` rows and `excluded 3`, because the liveness check ran BEFORE the dedup
    /// probe. D-0080 asks for an excluded position to be named -- once, because there is
    /// one position.
    ///
    /// The third witness is mine and covers the bucket the other two leave at zero: a
    /// position genuinely below the threshold. Without it a fix that simply stopped
    /// counting `infrequent` at all would pass.
    #[test]
    fn every_offer_at_k1_lands_in_exactly_one_bucket() {
        /// k=1 always runs, so `levels` is never empty -- said with an assertion rather
        /// than an `expect`, and cloned so the three witnesses below each read one line.
        fn level_one(s: &Sweep) -> Frontier {
            let first = s.levels.first();
            assert!(first.is_some(), "k=1 always runs, so levels is never empty");
            first.cloned().unwrap_or_default()
        }

        // `assert` then `unwrap_or`, not `expect`: the workspace denies `expect_used`, and
        // the fallback is itself a non-live index so a broken table cannot make this test
        // pass by accident.
        let dead = (0..vocab::table::NEXT_FREE)
            .find(|b| !vocab::table::is_live(*b))
            .map(u32::from);
        assert!(
            dead.is_some(),
            "§3.8 keeps retired indices reserved forever, so at least one is not live"
        );
        let dead = dead.unwrap_or(u32::from(vocab::table::NEXT_FREE));

        // Witness 1 — three duplicate offers, and nothing is infrequent.
        let b = bars(&[&[0, 1], &[0, 1], &[0], &[1]]);
        let s = Ladder::with_min_hits(1).walk(&b, &[0, 0, 1, 1, 1]);
        let k1 = level_one(&s);
        assert_eq!(
            (
                k1.generated,
                k1.duplicates,
                k1.infrequent,
                k1.excluded,
                k1.frequent.len()
            ),
            (5, 3, 0, 0, 2),
            "five offers: three repeats and two frequent singletons. Nothing here was \
         measured and found too rare."
        );
        assert!(k1.reconciles());

        // Witness 2 — a repeated tombstone is one exclusion, named once, unmeasured.
        let s = Ladder::with_min_hits(1).walk(&b, &[0, 1, dead, dead, dead]);
        let k1 = level_one(&s);
        assert_eq!(
            s.excluded.iter().filter(|e| e.position == dead).count(),
            1,
            "position {dead} is one position and D-0080 asks for it to be named once"
        );
        assert_eq!(
            (k1.generated, k1.duplicates, k1.excluded, k1.infrequent),
            (5, 2, 1, 0)
        );
        assert_eq!(
            s.excluded
                .iter()
                .find(|e| e.position == dead)
                .map(|e| e.support),
            Some(None),
            "a tombstone's support was never measured, and §3.6 forbids reporting a 0 \
         that cannot be told apart from a position absent on every bar"
        );
        assert!(k1.reconciles());

        // Witness 3 — the infrequent bucket, so a fix that zeroed it would not pass.
        let b = bars(&[&[0, 1], &[0, 1], &[0], &[1], &[0]]);
        let s = Ladder::with_min_hits(4).walk(&b, &[0, 1]);
        let k1 = level_one(&s);
        assert_eq!(
            (
                k1.generated,
                k1.duplicates,
                k1.infrequent,
                k1.excluded,
                k1.frequent.len()
            ),
            (2, 0, 1, 0, 1),
            "bit 0 hits 4 of 5 and clears min_hits; bit 1 hits 3 and does not"
        );
        assert!(k1.reconciles());
    }

    #[test]
    fn a_position_true_on_every_bar_is_excluded_and_named() {
        // D-0080: support exactly 1.000 partitions nothing, and the exclusion
        // must be NAMED in the output rather than silently dropped.
        let b = bars(&[&[0, 9], &[1, 9], &[0, 9]]);
        let s = Ladder::with_min_hits(1).walk(&b, &[0, 1, 9]);
        let got = s.excluded.iter().find(|e| e.position == 9);
        assert_eq!(
            got,
            Some(&Excluded {
                position: 9,
                support: Some(3),
                reason: Why::AlwaysTrue
            }),
            "position 9 is set on all three bars"
        );
        assert!(
            s.all_frequent().all(|i| !i.mask.get(9)),
            "an excluded position must not appear in any frequent set"
        );
    }

    #[test]
    fn a_position_false_on_every_bar_is_excluded_and_named() {
        let b = bars(&[&[0], &[1], &[0]]);
        let s = Ladder::with_min_hits(1).walk(&b, &[0, 1, 7]);
        assert_eq!(
            s.excluded.iter().find(|e| e.position == 7),
            Some(&Excluded {
                position: 7,
                support: Some(0),
                reason: Why::AlwaysFalse
            })
        );
    }

    #[test]
    fn the_subset_prune_actually_removes_candidates() {
        // {0,1} and {0,2} are frequent, {1,2} is not. So {0,1,2} is generated by
        // the join and must be pruned WITHOUT being evaluated.
        let b = bars(&[&[0, 1], &[0, 1], &[0, 2], &[0, 2], &[1], &[2]]);
        let s = Ladder::with_min_hits(2).walk(&b, &[0, 1, 2]);
        let k3 = s.levels.iter().find(|l| l.k == 3);
        assert!(
            k3.is_some_and(|l| l.pruned > 0),
            "the prune must fire, not merely exist"
        );
        assert!(k3.is_some_and(|l| l.frequent.is_empty()));
    }

    #[test]
    fn support_never_increases_as_bits_are_added() {
        // Anti-monotonicity, the property every prune above rests on. Checked
        // over the real ladder rather than asserted in a comment.
        let b = bars(&[&[0, 1, 2], &[0, 1], &[0], &[1, 2], &[2]]);
        let s = Ladder::with_min_hits(1).walk(&b, &[0, 1, 2]);
        for set in s.all_frequent() {
            let mut bit: u32 = 0;
            while bit < ConditionMask::BITS {
                if set.mask.get(bit) {
                    let smaller = set.mask.without_bit(bit);
                    assert!(
                        support(&b, &smaller) >= set.hits,
                        "removing a bit must not reduce support"
                    );
                }
                bit = bit.saturating_add(1);
            }
        }
    }

    #[test]
    fn every_frequent_set_meets_the_threshold_exactly_as_stated() {
        let b = bars(&[&[0, 1], &[0, 1], &[0, 1], &[0], &[1]]);
        let s = Ladder::with_min_hits(3).walk(&b, &[0, 1]);
        assert!(s.all_frequent().all(|i| i.hits >= 3));
        assert!(s.all_frequent().all(|i| i.hits == support(&b, &i.mask)));
    }

    #[test]
    fn one_k_set_is_evaluated_once_however_many_pairs_produce_it() {
        // {0,1,2} arises from three different pairs of 2-sets. Duplicate
        // rejection is what keeps it one evaluation.
        //
        // The trailing `&[3]` bar is load-bearing and was missing when this test
        // was first written: without it, 0, 1 and 2 are each set on every bar, so
        // the D-0080 guard excluded all three as AlwaysTrue and k=1 was empty.
        // The test failed for the right reason and the fixture was the bug.
        let b = bars(&[&[0, 1, 2], &[0, 1, 2], &[0, 1, 2], &[3]]);
        let s = Ladder::with_min_hits(1).walk(&b, &[0, 1, 2, 3]);
        let k3 = s.levels.iter().find(|l| l.k == 3);
        assert!(
            k3.is_some_and(|l| l.frequent.len() == 1),
            "exactly one 3-set survives"
        );
        assert!(
            k3.is_some_and(|l| l.generated >= 3),
            "the join really does produce it 3 times"
        );
    }

    #[test]
    fn the_order_of_the_output_does_not_depend_on_hash_iteration_order() {
        // §3.5 byte-for-byte idempotence. HashSet iteration is randomised per
        // process, so the same input must still yield the same sequence.
        let b = bars(&[&[0, 1, 2], &[0, 1], &[1, 2], &[0, 2], &[0, 1, 2]]);
        let first = Ladder::with_min_hits(1).walk(&b, &[0, 1, 2]);
        for _ in 0..8 {
            let again = Ladder::with_min_hits(1).walk(&b, &[0, 1, 2]);
            let a: Vec<_> = first.all_frequent().copied().collect();
            let c: Vec<_> = again.all_frequent().copied().collect();
            assert_eq!(a, c, "two walks of one input disagreed");
        }
    }

    #[test]
    fn a_high_bit_survives_the_whole_ladder() {
        // The predecessor's bug was a frontier narrower than the vocabulary, so the
        // highest LIVE positions must survive the whole ladder.
        //
        // This used 273 and 383, and both were wrong once `walk` began validating the
        // list: 273 is VOID — one of the 39 forming-pivot rows D-0080 excluded — and 383
        // is past the table entirely. Both were silently swept before, and 383 was the
        // worse of the two: `with_bit` is a no-op past the width, so it became the EMPTY
        // mask, which every bar matches, and was reported as a measured always-true
        // condition. A bit nobody can set, reported as one every bar sets.
        //
        // 274 and 275 are the two most recently appended positions and the highest live
        // ones, which makes this the stronger test: it proves an append reaches the
        // ladder. The `&[]` bar keeps 274 off at least one bar so the D-0080 AlwaysTrue
        // guard does not exclude it before the ladder starts.
        let b = bars(&[&[274, 275], &[274, 275], &[274], &[]]);
        let s = Ladder::with_min_hits(2).walk(&b, &[274, 275]);
        assert_eq!(
            s.depth(),
            2,
            "a 2-set of the two highest LIVE bits must be found"
        );
        assert!(s.all_frequent().any(|i| i.mask.get(274) && i.mask.get(275)));
    }

    #[test]
    fn no_bar_and_no_position_are_both_survivable() {
        let empty: Vec<ConditionMask> = Vec::new();
        let s = Ladder::with_min_hits(1).walk(&empty, &[0, 1]);
        assert_eq!(s.bars, 0);
        assert_eq!(s.depth(), 0);
        let b = bars(&[&[0]]);
        let t = Ladder::with_min_hits(1).walk(&b, &[]);
        assert_eq!(t.depth(), 0);
    }

    #[test]
    fn a_threshold_above_the_bar_count_finds_nothing_and_says_so() {
        let b = bars(&[&[0, 1], &[0, 1]]);
        let s = Ladder::with_min_hits(99).walk(&b, &[0, 1]);
        assert_eq!(s.depth(), 0);
        assert_eq!(s.min_hits, 99, "the result echoes the threshold it applied");
    }
}

#[cfg(test)]
mod accounting {
    use super::*;

    fn bars(spec: &[&[u32]]) -> Vec<ConditionMask> {
        spec.iter()
            .map(|b| {
                b.iter()
                    .fold(ConditionMask::default(), |m, &x| m.with_bit(x))
            })
            .collect()
    }

    /// Every candidate the join produced must land in exactly one bucket.
    ///
    /// This test exists because the operator asked why a level showed candidates
    /// "pruned", and reconciling the answer showed that `generated` (10) exceeded
    /// `pruned + infrequent + frequent` (8) at k=3. The two missing candidates
    /// were duplicates, dropped correctly and reported nowhere. The defect was in
    /// the report, not the ladder — and a report nobody can reconcile is a report
    /// nobody should trust.
    #[test]
    fn no_candidate_goes_missing_from_the_report() {
        for spec in [
            &[
                &[0u32, 1, 2][..],
                &[0, 1, 2],
                &[0, 1],
                &[1, 2],
                &[0, 2],
                &[3],
            ][..],
            &[&[0, 1][..], &[0, 2], &[1, 2], &[0, 1, 2], &[4], &[5, 6]][..],
            &[
                &[0, 1, 2, 3][..],
                &[0, 1, 2],
                &[0, 1, 3],
                &[0, 2, 3],
                &[1, 2, 3],
                &[7],
            ][..],
        ] {
            let b = bars(spec);
            let live: Vec<u32> = (0..8).collect();
            for min in [1_u64, 2, 3] {
                let s = Ladder::with_min_hits(min).walk(&b, &live);
                for l in &s.levels {
                    // Counted before the assertion, not inside its message: a
                    // message argument runs only on failure, so `l.frequent.len()`
                    // there was an expression no passing run executed. `excluded`
                    // joins the sum for the same reason it is in `reconciles` --
                    // a message that omits a term it is reconciling cannot be
                    // checked against the number it reports.
                    let frequent = l.frequent.len();
                    assert!(
                        l.reconciles(),
                        "k={} lost candidates: generated {} but accounted \
                         {}+{}+{}+{}+{} as duplicates, excluded, pruned, \
                         infrequent and frequent",
                        l.k,
                        l.generated,
                        l.duplicates,
                        l.excluded,
                        l.pruned,
                        l.infrequent,
                        frequent,
                    );
                }
            }
        }
    }

    /// Pruning is not stopping: a pruned candidate is skipped, the level continues,
    /// and the ladder climbs past it.
    #[test]
    fn a_pruned_candidate_does_not_end_the_level_or_the_ladder() {
        // {0,1},{0,2},{1,2},{0,1,2} all frequent, plus {3,4} to keep the frontier
        // wide. k=3 prunes nothing here and k=4 must still be attempted.
        let b = bars(&[&[0, 1, 2], &[0, 1, 2], &[0, 1, 2], &[3, 4], &[3, 4], &[5]]);
        let s = Ladder::with_min_hits(2).walk(&b, &[0, 1, 2, 3, 4, 5]);
        let k3 = s.levels.iter().find(|l| l.k == 3);
        assert!(
            k3.is_some_and(|l| !l.frequent.is_empty()),
            "k=3 found {{0,1,2}}"
        );
        assert!(
            s.levels.iter().any(|l| l.k == 4),
            "k=4 must be ATTEMPTED even though k=3 pruned candidates"
        );
        assert_eq!(
            s.depth(),
            3,
            "depth is where it ran out, not where it pruned"
        );
    }
}

#[cfg(test)]
mod caller_input {
    use super::*;

    fn bars(spec: &[&[u32]]) -> Vec<ConditionMask> {
        spec.iter()
            .map(|bits| {
                bits.iter()
                    .fold(ConditionMask::default(), |m, &b| m.with_bit(b))
            })
            .collect()
    }

    /// A position past the mask width is refused and NAMED, not swept.
    ///
    /// `with_bit` is a no-op past the width, so such a position became the EMPTY mask —
    /// which every bar matches — and was reported as a measured always-true condition.
    /// A bit nobody can set, reported as one every bar sets.
    #[test]
    fn an_out_of_range_position_is_refused_and_named() {
        let b = bars(&[&[0], &[1], &[0]]);
        let s = Ladder::with_min_hits(1).walk(&b, &[0, 1, 9_999]);
        let named = s.excluded.iter().find(|e| e.position == 9_999);
        assert_eq!(
            named.map(|e| e.reason),
            Some(Why::NotLive),
            "9999 is past the mask and must be named, not swept"
        );
        assert!(
            s.all_frequent().all(|i| !i.mask.get(9_999)),
            "a refused position must not reach any frequent set"
        );
    }

    /// A void or retired position is refused and named.
    ///
    /// `Why::NotLive` existed and was constructed nowhere in the workspace, so a
    /// tombstone was reported as if it had been measured and found false. §3.8 keeps
    /// those indices reserved forever; sweeping one sweeps a name, not a condition.
    #[test]
    fn a_tombstone_or_void_position_is_refused_and_named() {
        let b = bars(&[&[0], &[1], &[0]]);
        // 6, 19 and 25 are the three retired positions; 240 is one of the 39 void rows.
        let s = Ladder::with_min_hits(1).walk(&b, &[0, 1, 6, 19, 25, 240]);
        for dead in [6_u32, 19, 25, 240] {
            let named = s.excluded.iter().find(|e| e.position == dead);
            assert_eq!(
                named.map(|e| e.reason),
                Some(Why::NotLive),
                "position {dead} is not live and must be named"
            );
        }
        assert!(
            s.all_frequent()
                .all(|i| vocab::table::only_live(i.mask) == i.mask)
        );
    }

    /// A duplicated position is offered once.
    ///
    /// The same position twice produces the same singleton twice, and at k=2 a pair of
    /// identical bits whose union has popcount 1 — which the join silently drops, so the
    /// frontier no longer matches the list the caller handed in.
    #[test]
    fn a_duplicated_position_is_offered_once() {
        let b = bars(&[&[0, 1], &[0, 1], &[0], &[1]]);
        let once = Ladder::with_min_hits(1).walk(&b, &[0, 1]);
        let twice = Ladder::with_min_hits(1).walk(&b, &[0, 1, 0, 1, 1]);
        let a: Vec<_> = once.all_frequent().copied().collect();
        let c: Vec<_> = twice.all_frequent().copied().collect();
        assert_eq!(a, c, "duplicates in the list changed the result");
    }

    /// `min_hits = 0` is raised to 1, so extinction still happens.
    ///
    /// At zero every candidate satisfies `hits >= 0`, including one with support ZERO, so
    /// no level ever empties and the walk becomes a full powerset enumeration independent
    /// of the bars — with extinction, the mechanism §6 puts in place of a depth
    /// parameter, simply off. It also contradicted this module's own D-0080 guard, which
    /// excludes a POSITION of support zero on the argument that it partitions nothing.
    #[test]
    fn a_zero_threshold_is_raised_to_one_and_the_ladder_still_dies() {
        let b = bars(&[&[0, 1], &[0, 1], &[0], &[2], &[]]);
        let l = Ladder::with_min_hits(0);
        assert_eq!(
            l.min_hits(),
            1,
            "zero must be raised, and reported as raised"
        );
        let s = l.walk(&b, &[0, 1, 2]);
        assert!(
            s.levels.last().is_some_and(|x| x.frequent.is_empty()),
            "the ladder must still reach an empty level"
        );
        // Read once, into a name the message can interpolate. Calling `depth()` in
        // the failure message put the only call to it on a path no passing run
        // takes.
        let depth = s.depth();
        assert!(depth <= 3, "depth {depth} is a powerset, not an extinction");
        // And no frequent set may have support zero.
        assert!(s.all_frequent().all(|i| i.hits >= 1));
    }

    /// The threshold a run applied is what the result reports.
    #[test]
    fn the_result_reports_the_threshold_actually_applied() {
        let b = bars(&[&[0], &[1]]);
        let s = Ladder::with_min_hits(0).walk(&b, &[0, 1]);
        assert_eq!(s.min_hits, 1, "the result must echo the raised threshold");
    }
}
