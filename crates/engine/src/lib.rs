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

use crate::column::{Column, set_positions};

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
    /// **This is canonical MASK order, not support order.** The wording here used
    /// to say the opposite — "ordered by support, ties broken by word order" —
    /// and the code has never done that: `sort_canonically` keys on
    /// `(mask.words(), hits)`, so the word array is the PRIMARY key and `hits` is
    /// a tiebreak that can never fire, because a mask is unique within a level.
    /// The module header states it correctly; this field doc contradicted it.
    ///
    /// It is not a ranking by edge, profitability or any other outcome — no
    /// document defines one, so this module does not invent one
    /// (`CLAUDE.md` §3.1). A reader who wants the strongest first must sort, and
    /// must first decide what "strongest" means.
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
    /// Every candidate the join produced ended in exactly one of five places.
    ///
    /// `generated == duplicates + excluded + pruned + infrequent + frequent`. A
    /// level that does not satisfy this has lost a candidate somewhere, and no
    /// report built on it can be believed.
    ///
    /// `excluded` is the one that surprises a reader, and it is why this list is
    /// five long rather than four: at k=1 a position rejected by D-0080 as
    /// always-true or always-false was still *generated*, so it must still be
    /// accounted for. A summary that shows the other four and omits this one
    /// reads as though the difference had vanished.
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
///
/// # Two budgets, because one measured the wrong thing
///
/// The first version bounded distinct candidates alone. An audit then measured
/// the whole threshold range with a counting allocator and a watchdog and found
/// **memory never binds**: peak heap topped out at 1.59 GB, 3.3% of the machine,
/// while every run below 11% support was killed at 1,500 seconds still inside one
/// join. The candidate ceiling counts what a level HOLDS, and `popcount != k`
/// rejects almost every pair before it is counted — so it saw 38 million units of
/// a four-hundred-billion-unit job. [`Breach`] says which budget was spent.
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
    /// Pairs the join had iterated when the walk stopped.
    ///
    /// # Why bytes were not enough
    ///
    /// [`Self::candidates`] bounds what a level HOLDS. This bounds what it DOES,
    /// and an audit measured how far apart those are: at `min_hits = 2` and k=6
    /// the frontier was 937,181 wide, so the join had `|F|²/2 ≈ 4.39 × 10¹¹`
    /// pairs to walk — while `generated` reached only 38 million, because the
    /// `popcount != k` filter rejects almost all of them before they are counted.
    /// **Five orders of magnitude of work the candidate ceiling cannot see.**
    ///
    /// The same audit measured the consequence end to end: peak heap never
    /// exceeded 1.59 GB — 3.3% of that machine — while runs below 11% support sat
    /// for over 1,500 seconds and were killed. Memory was never what bound; time
    /// was, and nothing was counting it.
    pub pairs: u64,
    /// The pair budget that was in force.
    pub pair_budget: u64,
    /// Which budget was spent.
    pub breach: Breach,
}

/// Which of the two budgets a walk spent.
///
/// They measure different things and a sweep can hit either first: one bounds
/// the bytes a level holds, the other the work it does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Breach {
    /// Distinct candidates admitted reached the ceiling. Bounds memory.
    #[default]
    Candidates,
    /// The join's pair iterations reached the budget. Bounds time.
    Pairs,
    /// **The machine refused the allocation.** Not a number anyone chose.
    ///
    /// The ceiling above is a constant, and a constant is a human input — the
    /// thing `CLAUDE.md` §6 removes from depth and which was reaching depth
    /// through the side door anyway. At `1 << 23` a real column halted at k=9
    /// with the pair budget 99.95% unused; the ladder was not extinct, it was
    /// capped, and raising the constant only moves the cap somewhere else a
    /// person picked.
    ///
    /// This is the bound that needs no person. Every growth of the candidate
    /// set goes through [`HashSet::try_reserve`], which returns rather than
    /// aborts, so the sweep expands until the allocator genuinely refuses and
    /// then halts naming that. It uses what a 4 GB machine has and what a 48 GB
    /// machine has, discovers which at runtime, and asks nobody.
    Memory,
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
/// round-up. `2^26 · 128 B = 8 GiB`, which is the largest single level this
/// crate will build on an ordinary machine without the operator having said so.
///
/// **This paragraph said `2^23 · 128 B = 1 GiB` after the constant moved to
/// `2^26`, understating the bound it justifies by eight times.** An adversarial
/// audit caught it. Measured against the real figure: at `min_hits = 50` this
/// engine went extinct holding **5.0 GB**, which is the arithmetic above being
/// approximately right rather than an accident.
///
/// # The headroom argument this doc used to make no longer holds
///
/// With all 238 live positions frequent at k=1 — the worst case the vocabulary
/// permits — the distinct-candidate counts are `C(238,2) = 28,203` and
/// `C(238,3) = 2,218,636`, both far under this. `C(238,4) = 130,344,865` used to
/// be **fifteen times** over the ceiling; against `2^26` it is **1.94 times**
/// over. The comfortable margin the old text argued from is gone, and that is
/// the deliberate consequence of D-0138: the ceiling was capping DEPTH, which
/// §6 forbids, so it was raised until the allocator rather than a constant is
/// what stops the walk. [`Breach::Memory`] is now the bound that carries the
/// weight this paragraph used to.
///
/// Both bounds are pinned by
/// `engine::tests::the_default_ceiling_is_the_one_its_arithmetic_describes`,
/// which asserts them in `const` blocks — so moving this constant to a value that
/// puts `C(238,3)` outside it, or `C(238,4)` inside it, fails the **build** and
/// not a test run. The paragraph above is therefore checked arithmetic rather
/// than a comment, which is the whole of what CI gate 12 asks for.
/// # 2^27, and the arithmetic is this machine's rather than a preference
///
/// The bound is RAM and nothing else. **Measured on the operator's machine at
/// `2^25`: k=14, 24 s, 4.9 GB of 48** — so a candidate costs about 146 bytes
/// once the `seen` set's own overhead is counted, not the 56 the `Itemset`
/// struct suggests. From that one measurement the ladder is linear:
///
/// | ceiling | candidates | RAM |
/// |---|---|---|
/// | `2^25` | 33.5 M | 4.9 GB (measured) |
/// | `2^26` | 67.1 M | ~9.8 GB |
/// | **`2^27`** | **134.2 M** | **~19.6 GB** |
/// | `2^28` | 268.4 M | ~39.2 GB — swaps on a 48 GB machine |
///
/// `2^27` is the largest power of two that leaves the machine room to hold the
/// bars, the column and the operating system alongside it. `2^28` fits only if
/// nothing else does.
///
/// # Raising the PAIR budget instead buys nothing
///
/// The join is prefix-grouped and its own comment states the consequence: *"No
/// popcount filter, because the grouping already IS that filter"*. Every pair
/// yields exactly one distinct candidate, so pairs walked and candidates
/// enumerated are ONE quantity. A pair budget of `2^40` would permit a trillion
/// candidates, and a trillion retained survivors is 160 TB. The ceiling is the
/// only lever and RAM is where it stops.
pub const DEFAULT_CEILING: usize = 1 << 27;

/// Pairs one level's join may iterate before the walk refuses.
///
/// # This bounds TIME, and the ceiling above bounds bytes
///
/// They are not interchangeable and an audit measured how far apart they are. At
/// `min_hits = 2` on a 1,124-bar column the frontier at k=6 was 937,181 wide, so
/// the join had `|F|²/2 ≈ 4.39 × 10¹¹` pairs to walk. `generated` reached only
/// 38 million, because `popcount != k` rejects almost every pair before it is
/// counted — so the candidate ceiling saw 38 million units of a job that was four
/// hundred billion. The walk sat in that one loop for 456 seconds.
///
/// The same audit ran the whole threshold range end to end with a counting
/// allocator and a watchdog. **Peak heap never exceeded 1.59 GB — 3.3% of that
/// machine — while every run below 11% support was killed at 1,500 seconds.**
/// Memory was never the binding constraint. Time was, and nothing counted it.
///
/// `2^34` is 17.2 billion pair iterations. The per-pair cost is a mask union and
/// a popcount, measured by `C-E-08` in `crates/engine/benches/ratio.rs`, so the
/// budget is stated in the unit the bench measures rather than in seconds, which
/// would be a claim about a machine rather than about the work.
/// # IT CANNOT BIND AT THE SHIPPED CEILING, and that is recorded rather than
/// left for the next reader to discover
///
/// This budget exists because time was the binding constraint and nothing
/// counted it. At the shipped constants it still does not, and the arithmetic
/// is short: the prefix join makes `duplicates` a **measured zero** — every
/// pair contributes exactly one distinct candidate — so `seen` grows one entry
/// per pair walked. [`Ladder::exhausted`] tests `admitted + seen.len() >=
/// ceiling` in the same loop that counts pairs, and [`DEFAULT_CEILING`] is
/// `2^27` against this `2^34`. The ceiling is **128x smaller**, so it trips
/// 128 pairs-worth of work before this budget is approached, and every
/// default-configured halt reports [`Breach::Candidates`] — a MEMORY reason —
/// including runs that spent their whole time in the join.
///
/// **THIS PARAGRAPH SAID `2^26` AND `256x` AND THE CONSTANT IS `2^27`.** The
/// conclusion is unchanged — the ceiling still trips first and this budget
/// still cannot fire at the defaults — but the factor was wrong by two, and an
/// arithmetic argument whose inputs do not match the constants it names is the
/// same class of defect D-0302 built a gate for. Corrected by D-0304, which
/// also records what nothing here says: **no production caller sets the
/// ceiling at all.** `Ladder::with_ceiling` is called from tests and benches
/// only, so every real run halts at a `DEFAULT_CEILING` sized in this file's
/// own table against *"a 48 GB machine"* — a static assumption about hardware
/// that the operator's machine may not share, deciding how far the ladder is
/// allowed to walk before it stops enumerating combinations.
///
/// `engine::the_pair_budget_refuses_where_the_ceiling_cannot` proves the budget
/// works; note that it must call `with_pair_budget(1)` to reach it. Nothing
/// proves it fires at the default, because it cannot.
///
/// UNVERIFIED whether the right correction is a larger ceiling, a smaller
/// budget, or dropping one of the two as redundant now that pairs and distinct
/// candidates are the same quantity. That is a `docs/05-decisions.md` choice
/// about engine behaviour and is deliberately not made here; what is fixed here
/// is the silence about it.
pub const DEFAULT_PAIR_BUDGET: u64 = 1 << 34;

/// The ladder. **Carries no depth field**, by `CLAUDE.md` §6.
#[derive(Clone, Copy, Debug)]
pub struct Ladder {
    min_hits: u64,
    ceiling: usize,
    pair_budget: u64,
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
            pair_budget: DEFAULT_PAIR_BUDGET,
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
            pair_budget: self.pair_budget,
        }
    }

    /// The per-level candidate ceiling this ladder will actually apply.
    #[must_use]
    pub const fn ceiling(&self) -> usize {
        self.ceiling
    }

    /// The same ladder with a different pair-iteration budget.
    ///
    /// Zero is raised to one, for the reason the other two knobs are: a budget of
    /// zero refuses before doing anything, so every level past k=1 would report a
    /// breach that describes the caller rather than the data.
    ///
    /// **Not a depth control either.** See [`Halt`]: it returns a refusal beside
    /// the complete levels rather than a truncated answer that reads as whole.
    #[must_use]
    pub const fn with_pair_budget(self, pair_budget: u64) -> Self {
        Self {
            min_hits: self.min_hits,
            ceiling: self.ceiling,
            pair_budget: if pair_budget == 0 { 1 } else { pair_budget },
        }
    }

    /// The pair-iteration budget this ladder will actually apply.
    #[must_use]
    pub const fn pair_budget(&self) -> u64 {
        self.pair_budget
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
        // Cumulative on BOTH axes. `pairs` was per-level, which is the exact
        // defect fixed for `admitted` in 5b791da reintroduced on the time axis:
        // a walk of depth 12 could spend twelve budgets and record no Halt.
        let mut pairs_walked: u64 = 0;
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
            let (next, halt, added, walked) =
                self.next_level(&column, &current, k, admitted, pairs_walked);
            admitted = admitted.saturating_add(added);
            pairs_walked = pairs_walked.saturating_add(walked);
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

    /// Whether the walk may allocate one more distinct candidate.
    ///
    /// # THE BOUND IS CUMULATIVE, AND A PER-LEVEL ONE HAS ALREADY FAILED
    ///
    /// `admitted` accumulates across every level of the walk, and it is tempting
    /// to call that a mistake: `seen` is built fresh inside [`Self::next_level`]
    /// and dropped when that level ends, so the bytes THIS SET holds are one
    /// level's worth. An audit reached exactly that conclusion and changed the
    /// test to `seen.len() + grow_by > ceiling`.
    ///
    /// **It is wrong, and `the_budget_is_cumulative_and_a_per_level_cap_would_
    /// miss_it` caught it.** `seen` is not the only allocation. Every SURVIVOR
    /// of every level is retained in the frontier and none of them is dropped —
    /// that is the memory that accumulates, and a per-level cap cannot see it.
    /// The test's own fixture is built to prove it: no single level reaches 100
    /// candidates while the walk holds 247 in total, so a per-level ceiling of
    /// 100 never fires and the walk runs to extinction. Its doc records what
    /// that shape did in production — *"precisely the shape that OOM-killed a
    /// real 3,000-bar run while every per-level check passed"*.
    ///
    /// So the ceiling stays cumulative. What it costs is stated rather than
    /// hidden: it also bounds the total number of candidates a walk may ever
    /// examine, which is why raising [`DEFAULT_PAIR_BUDGET`] buys nothing and
    /// why the reach of a single walk is bounded by RAM at roughly 134 million
    /// on a 48 GB machine. Billions of candidates in one walk would need
    /// billions of survivors retained, and there is no machine that holds them.
    ///
    /// Order matters: the ceiling is checked first so a caller that set one gets
    /// the breach it asked for, rather than a memory report from an allocator
    /// that was never going to refuse.
    fn exhausted(
        &self,
        seen: &mut HashSet<ConditionMask>,
        admitted: usize,
        grow_by: usize,
    ) -> Option<Breach> {
        if admitted.saturating_add(seen.len()) >= self.ceiling {
            return Some(Breach::Candidates);
        }
        if cannot_grow(seen, grow_by) {
            return Some(Breach::Memory);
        }
        None
    }

    fn next_level(
        self,
        column: &Column,
        prev: &Frontier,
        k: u32,
        admitted: usize,
        pairs_walked: u64,
    ) -> (Frontier, Option<Halt>, usize, u64) {
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

        // THE JOIN, GROUPED BY (k−2)-PREFIX — and the whole cost of this level.
        //
        // # What it replaces, and what that cost
        //
        // This was `for a in F { for b in F[a+1..] }` with a popcount filter:
        // every pair of survivors unioned, then discarded unless the union
        // happened to have exactly k bits. The filter is correct and it is
        // cheap per pair, and it was still the dominant expense of the sweep,
        // because almost every pair fails it. Measured over one 1,124-bar run:
        //
        // | k | \|F\| | pairs walked | survived | wasted |
        // |---|---|---|---|---|
        // | 5 | 607 | 183,921 | 9,345 | 94.9% |
        // | 6 | 837 | 349,866 | 13,421 | 96.2% |
        // | 7 | 823 | 338,253 | 13,059 | 96.1% |
        // | **all** | | **1,141,847** | **54,915** | **95.19%** |
        //
        // # Why the prefix is the whole trick
        //
        // Two distinct (k−1)-sets union to a k-set exactly when they share
        // k−2 positions. The pairs that do are precisely the pairs that agree
        // on every position but their HIGHEST — so grouping the frontier by
        // "the itemset minus its highest position" puts every joinable pair in
        // one group and no joinable pair across two.
        //
        // Completeness, which is the only thing that matters: for a frequent
        // k-set S = {p₁ < … < p_k}, the subsets S∖{p_k} and S∖{p_{k−1}} are
        // both frequent by anti-monotonicity, both have prefix {p₁…p_{k−2}},
        // and their union is S. So S is enumerated. Uniqueness: any pair
        // producing S must contribute S's two largest positions as its two
        // highest bits, which is that same pair and no other — so `duplicates`
        // becomes a MEASURED zero rather than an assumed one, and `seen` is
        // kept precisely to keep measuring it.
        //
        // # The grouping is built, not assumed
        //
        // `sort_canonically` orders by `mask.words()`, which is numeric word
        // order — it groups by the HIGHEST bit, the opposite of what this
        // needs, and prefix blocks are NOT contiguous under it. Relying on it
        // would silently drop candidates. So the key is computed and sorted on
        // explicitly here: |F| log |F| per level against the |F|²/2 it removes.
        let mut keyed: Vec<(ConditionMask, ConditionMask)> =
            Vec::with_capacity(prev.frequent.len());
        keyed.extend(
            prev.frequent
                .iter()
                .map(|it| (without_highest(&it.mask), it.mask)),
        );
        keyed.sort_unstable_by_key(|(prefix, mask)| (prefix.words(), mask.words()));

        let mut pairs: u64 = 0;
        'join: for block in keyed.chunk_by(|a, b| a.0 == b.0) {
            for (offset, (_, a)) in block.iter().enumerate() {
                // THE PAIR BUDGET, CHECKED ONCE PER OUTER ROW so it costs nothing
                // per pair. The count is exact rather than estimated because the
                // inner loop below increments it.
                //
                // This is the budget that bounds TIME. The candidate ceiling
                // bounds bytes, and the two are five orders of magnitude apart on
                // a wide frontier -- see `Halt::pairs`. Without this, the only
                // symptom of a `min_hits` set too low is a process that never
                // returns, which is the opposite of the loud refusal
                // `CLAUDE.md` §4 requires.
                if pairs_walked.saturating_add(pairs) >= self.pair_budget {
                    halted = Some(Halt {
                        k,
                        candidates: admitted.saturating_add(seen.len()),
                        ceiling: self.ceiling,
                        pairs: pairs_walked.saturating_add(pairs),
                        pair_budget: self.pair_budget,
                        breach: Breach::Pairs,
                    });
                    break 'join;
                }
                for (_, b) in block.iter().skip(offset.saturating_add(1)) {
                    pairs = pairs.saturating_add(1);
                    // No popcount filter, because the grouping already IS that
                    // filter: `a` and `b` share a prefix of k−2 positions and
                    // differ in their highest, so the union has exactly k. A
                    // branch here could never be taken, and an unreachable
                    // branch is a coverage hole -- the property is proved by
                    // `every_generated_candidate_has_exactly_k_bits` instead.
                    let cand = a.union(b);
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
                    //
                    // THE TWO MEMORY BOUNDS ARE ASKED AS ONE QUESTION, and that is
                    // what makes the halt reachable from a test. Written as two
                    // separate `if` blocks, the allocator arm was eleven regions
                    // no fixture could execute -- a refusal path nobody had ever
                    // seen fire, which is the shape of every defect an audit
                    // found today. `exhausted` takes the growth amount, so a test
                    // hands it `usize::MAX` and the whole arm runs.
                    if let Some(breach) = self.exhausted(&mut seen, admitted, 1) {
                        halted = Some(Halt {
                            k,
                            candidates: admitted.saturating_add(seen.len()),
                            ceiling: self.ceiling,
                            pairs: pairs_walked.saturating_add(pairs),
                            pair_budget: self.pair_budget,
                            breach,
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
            pairs,
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

/// Would growing the candidate set by `by` be refused by the allocator?
///
/// # Why this is a function and not two lines at the call site
///
/// So it can be TESTED. A genuine out-of-memory branch is unreachable from any
/// fixture — the machine has to actually run out — and an unreachable branch is
/// the coverage hole `CLAUDE.md` §9 refuses and, worse, a refusal path nobody
/// has ever seen fire. Taking `by` as a parameter makes both answers reachable:
/// `try_reserve(usize::MAX)` fails on capacity overflow *without allocating*, so
/// the failure arm is provable in a unit test on an empty set, in microseconds,
/// on any machine.
///
/// # The honest limit, which is the operating system's and not this code's
///
/// `try_reserve` reports what the ALLOCATOR refuses. On a system that
/// overcommits — macOS and Linux both do by default — a reservation can succeed
/// and the process still be killed later when the pages are touched. So this
/// catches an honest refusal and does not catch an overcommit death, and the
/// candidate ceiling remains as the bound that does. Two bounds, different
/// failure modes, and neither is claimed to be the other.
///
/// `len() == capacity()` first, so the reserve call is made only when growth is
/// actually due rather than on every candidate.
fn cannot_grow(seen: &mut HashSet<ConditionMask>, by: usize) -> bool {
    seen.len() == seen.capacity() && seen.try_reserve(by).is_err()
}

/// The itemset minus its highest set position — the join's grouping key.
///
/// Two (k−1)-sets union to a k-set exactly when they agree on every position
/// but their highest, so this value is equal for precisely the pairs the join
/// should visit and unequal for every pair it should not.
///
/// O(1): the scan is over [`ConditionMask`]'s six words, a compile-time
/// constant, not over the set bits — and it runs once per frontier entry per
/// level, never once per pair, so it is off the join's hot path entirely.
///
/// **UNVERIFIED as a measured figure.** The bound is read off the loop's
/// constant limit. That is sound as an argument and is not a measurement, and
/// no row in `crates/engine/benches/ratio.rs` covers this function. Gate 12
/// caught this block on the commit that introduced it.
///
/// # The empty mask
///
/// Returns the mask unchanged when nothing is set. The walk never supplies one
/// — a frontier holds (k−1)-sets and k ≥ 2 there, so popcount is at least one —
/// but the arm is real code and is covered by a direct unit test rather than
/// left as a branch no run reaches. At k=2 the prefix of every 1-set IS the
/// empty mask, which puts all of them in a single block and reproduces the
/// exhaustive pairing that level requires.
fn without_highest(m: &ConditionMask) -> ConditionMask {
    let words = m.words();
    for (index, word) in words.iter().enumerate().rev() {
        if *word != 0 {
            // `leading_zeros` is 0..=63 for a non-zero word, so the subtraction
            // cannot wrap; `saturating_sub` says so without a lint exception.
            let bit = 63_u32.saturating_sub(word.leading_zeros());
            let base = u32::try_from(index).unwrap_or(0).saturating_mul(64);
            return m.without_bit(base.saturating_add(bit));
        }
    }
    *m
}

/// True when every (k−1)-subset of `cand` is in the previous frequent frontier.
///
/// Walks the SET bits, k of them, not the whole 384-bit width.
///
/// # What this actually bought, which is less than it looks
///
/// The previous form scanned all 384 positions as
/// `if cand.get(b) && !frequent.contains(..)`. Its doc called that "384 probes
/// where k would do" and a bench comparison was written expecting a large win.
/// **It was 8%.** `&&` short-circuits, so the number of `HashSet` lookups was
/// already k — the 384 were cheap bit tests, not probes, and the doc's own
/// wording had oversold the cost of the thing it was complaining about.
///
/// Kept because 8% of the sweep is real and the code is strictly less work, not
/// because the estimate was right. `crates/vocab` still exposes no bit
/// iterator; `column::set_positions` is `crates/engine`'s own, which is why this
/// needed no change outside the crate.
///
/// # The measurement this used to cite was of a different function
///
/// This block named `C-E-02` as its proof. `C-E-02` benches the free `support`
/// function's per-bar cost and never calls this one — so the citation was for
/// the wrong subject, and the only per-candidate loop in the sweep other than
/// support counting had no bench row at all. Found by an adversarial audit.
///
/// **UNVERIFIED as a measured figure.** The O(1) bound above is read off the
/// loop's constant limit, which is sound as an argument and is not a
/// measurement. No row in `crates/engine/benches/ratio.rs` covers this function
/// yet, and naming one that does not would be worse than admitting none does.
///
/// It also matters more since the prefix join landed: with the join no longer
/// walking a million pairs, this 384-probe loop is now a materially larger
/// share of what a level costs than it was when the citation was written.
fn every_subset_is_frequent(cand: &ConditionMask, frequent: &HashSet<ConditionMask>) -> bool {
    set_positions(cand).all(|b| frequent.contains(&cand.without_bit(b)))
}

/// Order by the mask's words, then by hits.
///
/// `[u64; WORDS]` has a total order, so this is stable across processes and
/// machines — which `CLAUDE.md` §3.5 requires, since a `HashSet`'s iteration
/// order is randomised per process and must never reach the output.
fn sort_canonically(v: &mut [Itemset]) {
    v.sort_unstable_by_key(|i| (i.mask.words(), i.hits));
}

/// A collection's length as a `u64`, without a cast lint or a panic.
///
/// Two summary lines sat stacked here since 6082bea — `usize count as u64
/// without a cast lint or a panic.` above `A collection's length, as a u64.` —
/// a merge artifact that rustdoc renders as one run-on sentence. Found by an
/// adversarial audit; merged into the single line above.
///
/// **UNVERIFIED as a measured figure.** The 612,083 ns below was taken by an
/// audit on one machine and is not reproduced by any bench in
/// `crates/engine/benches/ratio.rs`, so it is recorded as what it is — an
/// observation that motivated a change — rather than as a bound this file
/// claims to hold.
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

/// Live positions in the shipped vocabulary, read from the table.
///
/// At module scope so the two `const` blocks in the ceiling test can use it
/// without tripping `items_after_statements`, and so a reader sees the three
/// derived numbers together rather than buried in a test body.
#[cfg(test)]
#[allow(
    clippy::cast_possible_truncation,
    reason = "a popcount of a 384-bit mask cannot exceed 384."
)]
const LIVE_POSITIONS: usize = vocab::table::LIVE.popcount() as usize;

/// `C(live, 3)` -- the widest a k=3 level can be if every live position is
/// frequent. The ceiling must sit above this or a healthy sweep halts.
#[cfg(test)]
const WORST_K3: usize = LIVE_POSITIONS * (LIVE_POSITIONS - 1) * (LIVE_POSITIONS - 2) / 6;

/// `C(live, 4)`. The ceiling must sit BELOW this, or it is bounding nothing.
#[cfg(test)]
const WORST_K4: usize = WORST_K3 * (LIVE_POSITIONS - 3) / 4;

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

    /// THREE MUTANTS NOTHING WAS STANDING BETWEEN.
    ///
    /// `cargo-mutants` over this file on 2026-08-18 tested 34 mutants and three
    /// survived, all of them here:
    ///
    /// * `replace Frontier::reconciles -> bool with true`
    /// * `replace Ladder::min_hits -> u64 with 1`
    /// * `delete field bars from struct Sweep expression in Ladder::walk`
    ///
    /// Each is a value the suite READ and never ASSERTED. `reconciles` was
    /// called in tests only ever expecting `true`, so a version that can never
    /// say `false` passed every one. `min_hits` was only ever set to 1 in the
    /// paths that then read it back. And `bars` was carried through the whole
    /// sweep without one assertion that it equals the column it was walked over.
    ///
    /// `CLAUDE.md` §9 blocks a build on a surviving mutant in a touched module.
    /// These are what that clause is for: three accessors that could return a
    /// constant and nothing would have noticed.
    #[test]
    fn the_frontier_summary_refuses_counters_that_do_not_add_up() {
        // `generated` must equal duplicates + excluded + pruned + infrequent +
        // the frequent set's own length. `excluded` is in that list because a
        // position D-0080 rejects at k=1 was still generated.
        let mut f = Frontier {
            k: 1,
            generated: 10,
            duplicates: 2,
            excluded: 3,
            pruned: 1,
            infrequent: 4,
            ..Frontier::default()
        };
        assert!(
            f.reconciles(),
            "2 + 3 + 1 + 4 + 0 frequent is 10 generated, which balances"
        );

        // Move exactly one counter. A `reconciles` that cannot return false
        // survives every assertion above and dies here.
        f.infrequent = 5;
        assert!(
            !f.reconciles(),
            "11 accounted against 10 generated must NOT reconcile — a summary \
             that always balances hides the difference it exists to show"
        );

        f.infrequent = 3;
        assert!(!f.reconciles(), "9 accounted against 10 generated is a gap");
    }

    #[test]
    fn the_min_hits_getter_reports_the_threshold_actually_applied() {
        // Deliberately not 1: `with_min_hits` raises zero TO one, so a getter
        // stuck at 1 is indistinguishable from a correct one on the clamped
        // path. 600 separates them.
        assert_eq!(Ladder::with_min_hits(600).min_hits(), 600);
        assert_eq!(
            Ladder::with_min_hits(0).min_hits(),
            1,
            "zero is raised to one, and the getter reports what was APPLIED \
             rather than what was asked for"
        );
        assert_eq!(Ladder::with_min_hits(2).min_hits(), 2);
    }

    #[test]
    fn a_sweep_records_the_bar_count_it_was_walked_over() {
        // `Sweep::bars` is what every support fraction downstream is taken over,
        // and `runner::Outcome::trustworthy` compares it against the census. A
        // sweep that reported 0 bars would make both meaningless, and nothing
        // here asserted it until this test.
        for len in [1_usize, 3, 7] {
            let column = bars(&vec![&[0_u32, 1][..]; len]);
            let sweep = Ladder::with_min_hits(1).walk(&column, &[0, 1]);
            assert_eq!(
                sweep.bars, len as u64,
                "a {len}-bar column must be recorded as {len} bars"
            );
        }

        let empty = Ladder::with_min_hits(1).walk(&[], &[0]);
        assert_eq!(empty.bars, 0, "an empty column is zero bars, not unset");
    }

    /// THE PROPERTY THE DELETED `popcount != k` FILTER USED TO ENFORCE.
    ///
    /// The prefix join drops the textbook `popcount != k` skip and justifies the
    /// deletion in a comment: `a` and `b` share a prefix of k−2 positions and
    /// differ only in their highest, so `a.union(b)` has exactly k bits and the
    /// branch could never be taken. That reasoning is correct and it was
    /// **unproven** — the comment cited this test by name and no such test had
    /// ever been written. `git log -S` over the whole history finds the
    /// identifier in exactly one commit: the one that removed the filter.
    ///
    /// So this is the assertion the deletion was authorised by. It walks the
    /// same exhaustive column space `the_apriori_kept_set_equals_the_brute_force_kept_set`
    /// uses — every assignment of six bars drawn from four bit patterns, at three
    /// thresholds — and checks that every itemset a level emits carries exactly
    /// that level's `k` bits. A join that paired across prefixes, or a
    /// `without_highest` that cleared the wrong bit, would put a k−1 or k+1
    /// itemset in a level and this refuses it.
    #[test]
    fn every_generated_candidate_has_exactly_k_bits() {
        const P: u32 = 4;
        let live: Vec<u32> = (0..P).collect();
        let shapes: [&[u32]; 4] = [&[], &[0, 1], &[1, 2], &[0, 1, 2, 3]];

        let mut checked = 0_u64;
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

            for min_hits in 1..=3_u64 {
                let sweep = Ladder::with_min_hits(min_hits).walk(&column, &live);
                for level in &sweep.levels {
                    for set in &level.frequent {
                        // Read once, into names both the comparison and the message
                        // use. `set.mask.popcount()` was spelled a SECOND time inside
                        // the message, and a message argument only runs when the
                        // assertion fails -- so that second call sat on a path no
                        // passing run executes. Same reason `kept_count` and
                        // `brute_count` are hoisted out of E-02's comparison below,
                        // and `depth` out of the extinction assertion in
                        // `mod caller_input`. Hoisted, the numbers are identical and
                        // the message keeps every one of them.
                        let bits = set.mask.popcount();
                        let k = level.k;
                        assert_eq!(
                            bits, k,
                            "level k={k} emitted a {bits}-bit itemset at \
                             min_hits={min_hits}, assignment={assignment}: the prefix \
                             join is the only thing standing in for the popcount \
                             filter it replaced"
                        );
                        checked = checked.saturating_add(1);
                    }
                }
            }
        }
        assert!(
            checked > 0,
            "the space produced no itemset at all, so this asserted nothing — \
             a test that cannot fail is the defect this file bans"
        );
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
    ///
    /// # And why 240 is in it
    ///
    /// 3 and 9 are both MEASURED exclusions, so `Excluded::support` was `Some` on
    /// every entry the render ever saw and the `None` half of the field's spelling
    /// went unrendered. §3 rule 6 forbids naming a measurement nobody took, which
    /// is the whole reason that field is an `Option` and the whole reason `render`
    /// prints the word `unmeasured` rather than a zero — and a spelling no test
    /// reads is a spelling a rerun can change silently. 240 is one of the void
    /// rows, so it is [`Why::NotLive`], its support is `None`, and the third
    /// branch of the exclusion line is now part of what "byte for byte" covers.
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
        let live = [0_u32, 1, 2, 3, 5, 9, 240];

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
        // The comparison above is only as wide as what `render` prints. These three
        // lines are the proof that it prints the exclusions at all: position 3 is
        // false on every bar and 9 is true on every bar, so both must appear with
        // their measured support and their reason -- and 240 is not live, so it must
        // appear with the word that stands in for a measurement nobody took. A
        // render that printed `0` there instead would be claiming 240 was tested
        // against the bars and found absent, which is exactly what §3 rule 6 and the
        // `Option` on `Excluded::support` exist to prevent.
        assert!(
            text.contains("excluded 3 0 AlwaysFalse")
                && text.contains("excluded 9 8 AlwaysTrue")
                && text.contains("excluded 240 unmeasured NotLive"),
            "`render` did not name the three excluded positions, so a rerun could \
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
        // A KEY WITH NO NAME IS NOT A DEPENDENCY EITHER, and this is the only guard
        // between a malformed line and a phantom entry. The scanner takes everything
        // left of the first `=` as the name, so a line that opens with one -- junk, a
        // half-finished edit, a continuation nobody closed -- yields the empty
        // string. Without the `!name.is_empty()` filter that empty string joins
        // `found`, and the exact-equality assertion at the top of this test then
        // reads `["", "vocab"]`: a dependency with no name, failing a check whose
        // whole value is that it says what IS there.
        assert_eq!(
            declared_dependencies("[dependencies]\n= \"1\"\nvocab = \"0.1.0\""),
            ["vocab"],
            "a nameless key must be dropped, not admitted as a dependency called \"\""
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
            core::mem::size_of::<u64>()
                + core::mem::size_of::<usize>()
                + core::mem::size_of::<u64>()
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

        // The PAIR BUDGET is held to the same rule. It bounds time where the
        // ceiling bounds bytes, and neither may pick a depth.
        let paired = Ladder::with_min_hits(2)
            .with_pair_budget(1_000_000)
            .walk(&b, &[0, 1, 2]);
        assert!(paired.completed(), "a budget this roomy cannot bite");
        assert_eq!(
            paired.depth(),
            roomy.depth(),
            "a pair budget is not a depth"
        );
        let by_pairs: Vec<Itemset> = paired.all_frequent().copied().collect();
        assert_eq!(wide, by_pairs, "nor does it change a combination");
    }

    /// The budget that bounds TIME, and the one the candidate ceiling cannot see.
    ///
    /// An audit measured the gap: at `min_hits = 2`, k=6, a 937,181-wide frontier
    /// gives the join `4.39 × 10¹¹` pairs to walk while `generated` reaches only
    /// 38 million — because `popcount != k` rejects almost every pair before it is
    /// counted. The ceiling saw 38 million units of a four-hundred-billion-unit
    /// job, and the walk sat in that loop for 456 seconds. Peak heap across the
    /// whole threshold range never passed 1.59 GB, 3.3% of the machine: **memory
    /// was never what bound.**
    #[test]
    fn the_pair_budget_refuses_where_the_ceiling_cannot() {
        let deep: &[u32] = &[0, 1, 2, 3, 4, 5, 7, 8];
        let b = bars(&[deep, deep, deep, &[9], &[9]]);
        let live = [0, 1, 2, 3, 4, 5, 7, 8, 9];

        // One pair is all it may walk, so the very first join row breaches.
        let s = Ladder::with_min_hits(2).with_pair_budget(1).walk(&b, &live);
        assert!(!s.completed(), "a one-pair budget cannot finish a join");
        let halt = s.halted.unwrap_or_default();
        assert_eq!(halt.breach, Breach::Pairs, "time, not bytes, was spent");
        assert_eq!(halt.pair_budget, 1);
        assert!(
            halt.candidates < engine_ceiling(),
            "the CANDIDATE ceiling was nowhere near spent -- that is the point"
        );
        for level in &s.levels {
            assert!(level.reconciles(), "level {} lost a candidate", level.k);
        }
    }

    /// The default ceiling, named once so the test above reads clearly.
    fn engine_ceiling() -> usize {
        DEFAULT_CEILING
    }

    /// Every pair the join reports having visited, it really visited.
    ///
    /// # The defect this exists for, which nothing else in the crate can see
    ///
    /// An audit injected `if generated >= 500 { break 'join; }` into the inner
    /// join loop. It lost **55.7% of the frequent sets** — 454 of 1024 — and
    /// still reported `completed() == true` with an empty final level, so it
    /// satisfied even the strongest assertion in the depth guard. `cargo test -p
    /// engine` passed 39/39 **and `cargo mutants` reported 3 caught, 0 missed**,
    /// because the mutation tool generated only `>=` → `<` for the injected
    /// threshold, which fires immediately and is caught.
    ///
    /// It is strictly worse than a depth cap: the walk does not stop, so every
    /// level above the truncation is subset-pruned against a **partial** frontier
    /// — which `walk`'s own comment says would "build k+1 from an incomplete
    /// frontier and label the result complete". That is the predecessor's
    /// `k = [1, 2]` failure wearing the disguise of a clean extinction.
    ///
    /// # Why this test can see it when nothing else can
    ///
    /// It is scale-free and self-consistent: `generated` is recomputed at each
    /// level from the previous level **alone** — the number of unordered pairs
    /// whose union has popcount k — and compared with what the level reported.
    /// An early break out of the join makes the reported number strictly smaller.
    /// Nothing else in the algorithm can, so there is no fixture size to get
    /// wrong and no threshold to sit beneath.
    #[test]
    fn the_join_visits_every_pair_it_reports() {
        // Ten co-occurring live bits plus one odd, so the ladder runs deep enough
        // for a truncation to have somewhere to hide.
        let deep: &[u32] = &[0, 1, 2, 3, 4, 5, 7, 8, 9, 10];
        let b = bars(&[deep, deep, deep, &[11], &[11]]);
        let s = Ladder::with_min_hits(2).walk(&b, &[0, 1, 2, 3, 4, 5, 7, 8, 9, 10, 11]);
        assert!(s.completed(), "the fixture must not breach a budget");

        // Level k's `generated` must equal the pairs of level k-1 whose union has
        // popcount k. Recomputed here from the frontier, not from the counter.
        // Zipped against its own tail rather than walked as `windows(2)`. A window
        // is a SLICE, so reading its two ends costs a `first()`/`last()` pair and an
        // arm for the `None` a width-2 window cannot produce -- a `continue` no run
        // can take, which is a branch the suite can never show working. Zipping the
        // levels with `skip(1)` yields exactly the same consecutive pairs already
        // unwrapped, so the pair is a value instead of a slice and there is no dead
        // arm left to reason about.
        for (prev, level) in s.levels.iter().zip(s.levels.iter().skip(1)) {
            // Recomputed for the PREFIX join: the pairs it walks are exactly the
            // pairs sharing a (k−2)-prefix. The older form counted every pair
            // whose union had popcount k, which is the same SET of k-sets
            // reached through every producing pair rather than through one --
            // 360 pairs where 120 sets exist. Counting producing pairs against a
            // join that no longer walks them would fail on a correct engine.
            let mut expected: u64 = 0;
            for (i, a) in prev.frequent.iter().enumerate() {
                for c in prev.frequent.iter().skip(i.saturating_add(1)) {
                    if without_highest(&a.mask) == without_highest(&c.mask) {
                        expected = expected.saturating_add(1);
                    }
                }
            }
            assert_eq!(
                level.generated, expected,
                "level {} reported {} candidates but its own frontier yields {} \
                 -- the join returned early and every level above it was pruned \
                 against a partial frontier",
                level.k, level.generated, expected
            );
        }

        // AND EVERY LEVEL MUST RECONCILE, which catches the other half.
        //
        // The pair-recount above sees a join that STOPPED. It cannot see a cap on
        // SURVIVORS -- `out.truncate(n)` after the join leaves `generated` exactly
        // right while silently dropping frequent sets, so an audit reported it as
        // invisible at every threshold. `Frontier::reconciles` is what sees it:
        // `generated` must equal duplicates + excluded + pruned + infrequent +
        // frequent, and a truncated survivor list makes that sum too small.
        let mut survivors = 0_u64;
        for level in &s.levels {
            // Read once, into a name the message interpolates and the running total
            // consumes. `level.frequent.len()` was spelled only inside the failure
            // message, where it runs on the panic path alone -- the same hoist E-02
            // makes for `kept_count`.
            let frequent = len_u64(level.frequent.len());
            assert!(
                level.reconciles(),
                "level {} does not reconcile: {} generated against \
                 {} duplicates + {} pruned + {} infrequent + {frequent} frequent. A \
                 survivor list was truncated after the join.",
                level.k,
                level.generated,
                level.duplicates,
                level.pruned,
                level.infrequent,
            );
            survivors = survivors.saturating_add(frequent);
        }

        // AND THE FIXTURE MUST HAVE PRODUCED A FRONTIER TO CHECK IN THE FIRST PLACE.
        //
        // Every assertion above this point lives inside a `for` over `s.levels`. A
        // walk that returned no level -- or one, which makes the zipped pair loop
        // empty too -- runs neither body, and this test then passes having asserted
        // nothing at all: §4's banned test, arriving by accident rather than by
        // authorship. The same hole is why
        // `every_generated_candidate_has_exactly_k_bits` counts what it checked and
        // why E-02 pins its column count.
        //
        // The numbers are the fixture's, not a floor. Ten co-occurring bits give a
        // frequent set for every non-empty subset of them -- 2^10 - 1 = 1023 -- and
        // 11, true on the other two bars, adds its singleton at k=1 for 1024. The
        // ladder therefore runs k=1..10 and dies at k=11: 11 levels. That 1024 is the
        // same total the header quotes when it says the injected `break 'join` lost
        // 454 of them, so a truncation this exact count would miss is one the header
        // has never seen.
        let depth = s.levels.len();
        assert_eq!(
            (depth, survivors),
            (11, 1024),
            "the fixture produced {depth} level(s) and {survivors} frequent set(s), so \
             the two loops above walked a frontier that is not the one this test was \
             written against"
        );
    }

    /// A cap keyed on ANY scale is caught, not just one keyed on `k`.
    ///
    /// # Why the single-point fixture was not enough
    ///
    /// `a_silently_capped_depth_would_be_caught` uses 5 bars, 9 offered positions,
    /// a deepest level of k=8 and a widest frontier of `C(8,4) = 70`. An audit
    /// binary-probed every boundary and found **five different caps that survive
    /// 39/39**: `k > 8`, `frequent.len() > 100`, `bar_bits.len() > 5000`,
    /// `live.len() > 200`, and `bar_bits.len() > 1000 && k >= 3`. Production is
    /// 238 live positions over 1.2 million bars, where `C(238,2) = 28,203` — the
    /// frontier margin alone is 400×.
    ///
    /// A single fixture can only ever see a cap below its own numbers. This one
    /// is parametric: `P` co-occurring live positions for `P` in 4..=12 sweeps
    /// `live.len()` across 5..13, frontier width across 6..924 and depth across
    /// 4..12 — so a cap keyed on any of them fires at some `P` and not at others,
    /// which is exactly what a single point cannot detect.
    #[test]
    fn a_cap_keyed_on_any_scale_would_be_caught() {
        // Live positions only: 6 is a retired tombstone that D-0080 excludes.
        const POOL: [u32; 12] = [0, 1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 12];
        const ODD: u32 = 13;

        for p in 4_usize..=12 {
            let deep: Vec<u32> = POOL.iter().take(p).copied().collect();
            let b = bars(&[&deep, &deep, &deep, &[ODD], &[ODD]]);
            let mut live = deep.clone();
            live.push(ODD);
            let s = Ladder::with_min_hits(2).walk(&b, &live);

            let width = u32::try_from(p).unwrap_or(0);
            assert!(s.completed(), "P={p} must not breach a budget");
            assert_eq!(s.depth(), p, "P={p}: extinction is at k={p}");
            assert_eq!(
                s.all_frequent().count(),
                (1_usize << width).saturating_sub(1).saturating_add(1),
                "P={p}: every non-empty subset of the {p}, plus the odd singleton"
            );
            assert!(
                s.levels.last().is_some_and(|l| l.frequent.is_empty()),
                "P={p}: the ladder must die of extinction, not of a cap"
            );
        }
    }

    /// The walk leaves a loop early in exactly three places, and each is a budget.
    ///
    /// # Why this has to be structural, and no test of behaviour will do
    ///
    /// `a_cap_keyed_on_any_scale_would_be_caught` sweeps `P` from 4 to 12, so it
    /// catches a cap keyed on depth or on frontier width — both verified by
    /// injection. It **cannot** catch one keyed above its own numbers:
    /// `if live.len() > 200 { break; }` survives all 44 tests, because the
    /// widest fixture offers 13 positions and production offers 238.
    ///
    /// That gap cannot be closed by a bigger fixture. A cap at any threshold a
    /// fixture does not cross is invisible to every behavioural test, and running
    /// the real 238 × 1.2 M shape in a unit test is the expense the whole design
    /// exists to avoid. So the guard counts **exits** instead of observing
    /// outcomes: an added `break` changes this number whatever it is keyed on.
    ///
    /// The three that are allowed, and why each is not a depth control:
    ///
    /// | Where | Leaves | Because |
    /// |---|---|---|
    /// | `walk`'s k-loop | the ladder | a level breached a budget; `Sweep::halted` names it |
    /// | `next_level`, outer row | the join | the PAIR budget — bounds time |
    /// | `next_level`, inner pair | the join | the CANDIDATE budget — bounds bytes |
    ///
    /// Every one records a [`Halt`], so none can truncate silently. A fourth exit
    /// has to be argued for here before it can compile.
    #[test]
    fn the_walk_has_exactly_three_early_exits_and_each_is_a_budget() {
        let src = include_str!("lib.rs");
        // Code lines only: this file discusses `break` at length in prose, and
        // text in a comment is text -- the lesson four guards in this workspace
        // have already learnt the hard way.
        // EVERY WAY OUT, not just `break`. An audit pointed out three the first
        // draft could not see: an early `return` from `next_level`, a labelled
        // `continue 'join` that skips the rest of a row, and anything at all in
        // `column.rs` -- which the sweep calls per candidate and which the guard
        // was not reading.
        //
        // `return` is counted only in the SHIPPING region: the test module below
        // is full of ordinary returns and closures, and scanning it would pin a
        // number that moves whenever a test is added.
        let shipping = src.split("#[cfg(test)]").next().unwrap_or(src);
        let count_exits = |text: &str| -> usize {
            text.lines()
                .map(|l| l.split_once("//").map_or(l, |(code, _)| code))
                .filter(|code| {
                    code.split_whitespace().any(|w| {
                        let w = w.trim_end_matches(';');
                        w == "break" || w == "return" || w == "continue"
                    })
                })
                .count()
        };
        let exits = count_exits(shipping);
        // The transposed column is on the sweep's hot path and had no guard at
        // all. Its only early exit is the empty-mask short answer.
        let column_src = include_str!("column.rs");
        let column_exits = count_exits(column_src.split("#[cfg(test)]").next().unwrap_or(""));
        assert_eq!(
            column_exits, 2,
            "crates/engine/src/column.rs may leave early in exactly two places: \
             `set_positions`' iterator returning `None` when a word is exhausted, \
             which is how an iterator ends, and `support`'s `popcount == 0` short \
             answer for the empty mask. Any THIRD exit truncates a support count, \
             which silently changes every hit total the ladder reads and which no \
             behavioural test can see -- the two layouts are answer-equivalent by \
             construction."
        );
        assert_eq!(
            exits, 10,
            "the shipping region of this file may leave a loop early in exactly \
             ten places, and every one is accounted for:\n\
             \x20 3 BUDGET EXITS, each recording a `Halt` -- the k-loop on a \
             breach, the join's outer row on the pair budget, the join's inner \
             pair on whichever memory bound `exhausted` names;\n\
             \x20 2 EXHAUSTION RETURNS inside `exhausted` -- the ceiling, which \
             a caller set, and the allocator, which nobody set;\n\
             \x20 4 FILTER SKIPS, which advance rather than truncate -- a \
             duplicate position and a non-live one at k=1, a duplicate \
             candidate, a subset-pruned candidate;\n\
             \x20 1 KEY RETURN -- `without_highest` handing back the join's \
             grouping key once it has found the top word.\n\
             It was NINE until `every_subset_is_frequent` became a one-line \
             `set_positions(cand).all(..)`, which deleted its `return false`. \
             Before that it was nine for a changed reason: the prefix join \
             removed the `popcount != k` skip and added the `without_highest` \
             return, and the two cancelled exactly. A count that holds for a new \
             reason is only honest if the reason is rewritten with it, and a \
             count that MOVES is only safe if the exit it lost is named.\n\
             A tenth is how a silent truncation arrives. `break` alone was not \
             enough: an audit defeated the first draft with an early `return` and \
             with a labelled `continue 'join`, neither of which carries the token \
             it counted. And a cap keyed above any fixture's scale \
             (`live.len() > 200` survives all 44 behavioural tests) is invisible \
             to everything except a count like this one."
        );

        // AND THE LOOP CONDITIONS THEMSELVES, because an exit does not need a
        // `break` to exist. An audit turned
        //     while !current.frequent.is_empty() {
        // into
        //     while !current.frequent.is_empty() && k < 13 {
        // -- a silent depth cap that leaves the token count at three, passes every
        // behavioural test, passes clippy, and changes no coverage. A guard that
        // counts exits has to pin where the loop is allowed to stop as well.
        //
        // SEARCHED IN CODE, NOT IN THE FILE, and the first draft was not.
        //
        // It called `src.contains(..)` on the whole file. The mutation above is
        // spelled out in this very paragraph, so `contains` found it in the
        // COMMENT and the guard passed on a mutated tree -- verified: the cap
        // survived all 45 tests. Assembling the needle with `concat!` was not
        // enough, because the prose is the thing that matches.
        //
        // The exit count above already strips comments; these must too. Sixth
        // time in this workspace, and the first where the guard warning about the
        // defect contained it.
        let code = src
            .lines()
            .map(|l| l.split_once("//").map_or(l, |(before, _)| before))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            code.contains(concat!("while !current.", "frequent.is_empty() {")),
            "the k-loop must stop on extinction ALONE. A second conjunct in its \
             condition is a depth cap that carries no `break` and is therefore \
             invisible to the count above."
        );
        assert!(
            code.contains(concat!("for (_, b) in block.", "iter().skip(")),
            "the join's inner loop must run over the whole remaining block. A \
             `.take(n)` or a narrowed range truncates the level without a `break`."
        );
        assert!(
            code.contains(concat!("for block in keyed.", "chunk_by(")),
            "the join's outer loop must visit EVERY prefix block. Skipping a block \
             drops every k-set whose two largest positions live in it, and does so \
             without a `break`, a `continue` or a counter moving -- the level would \
             reconcile perfectly against a frontier that is quietly short."
        );
    }

    #[test]
    fn a_zero_pair_budget_is_raised_to_one() {
        assert_eq!(
            Ladder::with_min_hits(1).with_pair_budget(0).pair_budget(),
            1
        );
        assert_eq!(
            Ladder::with_min_hits(1).with_pair_budget(9).pair_budget(),
            9
        );
        assert_eq!(
            Ladder::with_min_hits(1).pair_budget(),
            DEFAULT_PAIR_BUDGET,
            "the default is the documented one"
        );
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
        assert_eq!(d.pairs, 0, "and had iterated no pairs");
        assert_eq!(d.breach, Breach::Candidates, "the default arm");
        assert_ne!(
            d,
            Halt {
                k: 2,
                candidates: 2,
                ceiling: 2,
                pairs: 2,
                pair_budget: 2,
                breach: Breach::Pairs,
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
        assert_eq!(
            DEFAULT_CEILING, 134_217_728,
            "2^27. The bound is RAM and nothing else. MEASURED on the operator's \
             machine at 2^25: k=14, 24 s, 4.9 GB of 48 -- about 146 bytes per \
             candidate once the `seen` set's own overhead is counted, not the 56 \
             the `Itemset` struct suggests. Linear from there: 2^26 is ~9.8 GB, \
             2^27 is ~19.6 GB, 2^28 is ~39.2 GB and swaps on a 48 GB machine. \
             2^27 is the largest power of two that leaves room for the bars, the \
             column and the operating system beside it. Raising the PAIR budget \
             instead would buy nothing: the join is prefix-grouped, so every pair \
             yields exactly one distinct candidate and the two are ONE quantity."
        );
        // The doc's claim that a healthy sweep never reaches the ceiling, pinned
        // at COMPILE time rather than run time. Both operands are constants, so a
        // runtime assertion would only ever restate what the compiler already
        // knew; a `const` block fails the BUILD if the ceiling is ever moved to a
        // value that puts C(238,3) outside it or C(238,4) inside it, which is the
        // arithmetic the doc block on `DEFAULT_CEILING` argues from.
        // DERIVED FROM THE LIVE COUNT, NOT FROM A LITERAL, AND THAT MATTERS NOW.
        //
        // These read `2_218_636` and `130_344_865` -- C(238,3) and C(238,4) at a
        // vocabulary of 238 live positions. The vocabulary has grown twice since
        // (D-0244, D-0246) and is 323 live, where C(323,3) is 5,559,461: a
        // ceiling lowered anywhere into [2,218,637, 5,559,461] would have passed
        // both literal assertions while the real k=3 worst case overflowed it.
        //
        // The guard is only as good as the number it is computed from, so it is
        // computed from the table -- the same shape `WIDTH_IS_SUFFICIENT` uses
        // at the top of this file, and for the same reason.
        const {
            assert!(
                WORST_K3 < DEFAULT_CEILING,
                "C(live,3) must fit. The pin read 2,215,180 for months against a \
                 true 2,218,636 -- 3,456 short, and therefore a guard that \
                 admitted values it advertised as rejecting. Found by an \
                 adversarial audit and not by this assertion, which is why it is \
                 now derived."
            );
        };
        const {
            assert!(
                WORST_K4 > DEFAULT_CEILING,
                "C(live,4) must NOT fit, or the ceiling is not bounding anything"
            );
        };
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
        // {0,1,2} is reachable from three different pairs of 2-sets -- {0,1}+{0,2},
        // {0,1}+{1,2} and {0,2}+{1,2}. It must be EVALUATED once.
        //
        // The pairwise join reached it three times and leaned on `seen` to reject
        // two. The prefix join walks only the pair that agrees on everything but
        // its highest position -- {0,1}+{0,2} -- so it is reached once and the
        // duplicate rejection has nothing to reject. Both satisfy the name; the
        // second is the stronger property, so the assertions below pin THAT and
        // keep `duplicates` as the witness rather than the mechanism.
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
            k3.is_some_and(|l| l.generated == 1),
            "the prefix join must REACH it once, not reach it three times and \
             discard two"
        );
        assert!(
            k3.is_some_and(|l| l.duplicates == 0),
            "and the duplicate counter is the witness: a prefix join that emitted \
             the same k-set twice would be enumerating pairs it has no business \
             walking"
        );
    }

    /// The prefix join returns exactly what an exhaustive pairwise join returns.
    ///
    /// This is the only test that matters for the change that introduced it.
    /// Everything else here measures the join's COST; this measures its ANSWER,
    /// against an oracle written the slow, obvious way -- every pair, popcount
    /// filter, deduplicate, subset-prune, count support. If the prefix grouping
    /// ever drops a joinable pair, the two sets diverge and this fails, whatever
    /// the counters say and however perfectly each level reconciles.
    #[test]
    fn the_prefix_join_finds_exactly_what_an_exhaustive_join_would() {
        // Wide enough that most pairs do NOT share a prefix, so a grouping bug
        // has somewhere to lose candidates rather than being masked by a frontier
        // small enough that every pair happens to be in one block.
        let wide: &[u32] = &[0, 1, 2, 3, 4, 5, 6, 7];
        let b = bars(&[wide, wide, wide, &[0, 1, 2], &[3, 4, 5], &[8], &[8]]);
        let live: Vec<u32> = (0..=8).collect();
        let min_hits = 2;
        let s = Ladder::with_min_hits(min_hits).walk(&b, &live);
        assert!(s.completed(), "the fixture must not breach a budget");
        assert!(
            s.depth() >= 3,
            "and must climb far enough to be worth checking"
        );

        // The oracle: rebuild each level from the one below by brute force.
        let mut oracle: Vec<ConditionMask> = s
            .levels
            .first()
            .map(|l| l.frequent.iter().map(|i| i.mask).collect())
            .unwrap_or_default();
        for level in s.levels.iter().skip(1) {
            let prev: HashSet<ConditionMask> = oracle.iter().copied().collect();
            let mut next: HashSet<ConditionMask> = HashSet::new();
            for (i, a) in oracle.iter().enumerate() {
                for c in oracle.iter().skip(i.saturating_add(1)) {
                    let cand = a.union(c);
                    if cand.popcount() == level.k
                        && every_subset_is_frequent(&cand, &prev)
                        && support(&b, &cand) >= min_hits
                    {
                        next.insert(cand);
                    }
                }
            }
            let mut got: Vec<ConditionMask> = level.frequent.iter().map(|i| i.mask).collect();
            let mut want: Vec<ConditionMask> = next.into_iter().collect();
            got.sort_unstable_by_key(ConditionMask::words);
            want.sort_unstable_by_key(ConditionMask::words);
            assert_eq!(
                got, want,
                "level {} disagrees with an exhaustive join -- the prefix grouping \
                 reached a different set of k-sets, which no counter in this file \
                 would notice",
                level.k
            );
            oracle = want;
        }
    }

    /// The sweep, at production SHAPE, against a brute force sharing no code.
    ///
    /// # Why the source-text pin was not enough
    ///
    /// `the_walk_has_exactly_three_early_exits_and_each_is_a_budget` counts exit
    /// tokens per LINE and pins loop headers by prefix. An adversarial audit
    /// defeated it **nine** ways without moving either number:
    ///
    /// | Truncation | Why the count did not move |
    /// |---|---|
    /// | `if halt.is_some() \|\| (bars > 100 && k >= 3)` | folded into the existing `break`'s condition |
    /// | `frequent.len() > 4096 \|\|` in `every_subset_is_frequent` | folded into the existing `return false`'s condition |
    /// | `.take(1024)` on the join's outer row loop | that header was not pinned at all |
    /// | `.step_by(stride)` on the inner loop | the pin is `contains`, so any suffix passes |
    /// | the `'join` loop wrapped in `if column.bars() <= 1000` | an `if` carries no exit token |
    /// | `.then_some(())?` in an `Option` wrapper | `?` is a way out and is not one of the three words |
    /// | a `Ladder { min_hits: self.min_hits * 8, ..self }` into the join | the join is untouched; its INPUT is not |
    /// | `self.stride.min(4)` in `Column::support` | `column.rs`'s headers were not pinned |
    /// | `out.truncate(925)` | `reconciles()` was asserted only where the widest level is 252 |
    ///
    /// Every one passed 45/45. Three also passed `--fail-under-regions 100`,
    /// because a threshold no fixture crosses leaves no unexecuted region.
    ///
    /// The only thing that sees all nine is recomputing the answer at a scale
    /// above every other fixture here, from the other layout. A counter cannot
    /// catch a cap on the quantity the counter itself reports.
    #[test]
    fn a_production_shape_sweep_equals_its_brute_force() {
        /// Sixteen co-occurring live positions: frontier width to 10,090, where
        /// the parametric fixture stops at 924.
        const DEEP: [u32; 16] = [0, 1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        /// 1,200 bars — four times the largest column any other test builds.
        const BARS: usize = 1_200;
        /// Chosen so the ladder dies of `min_hits`, not of running out of bits.
        const MIN_HITS: u64 = 700;

        let column: Vec<ConditionMask> = (0..BARS)
            .map(|i| {
                DEEP.iter()
                    .enumerate()
                    .fold(ConditionMask::default(), |m, (j, &p)| {
                        // Each position is absent on its own residue class, so the
                        // sixteen supports differ and the lattice is not one block.
                        if i % (j + 7) == 0 { m } else { m.with_bit(p) }
                    })
            })
            .collect();
        let live: Vec<u32> = DEEP.to_vec();
        let sweep = Ladder::with_min_hits(MIN_HITS).walk(&column, &live);

        // 1. IT DIED OF EXTINCTION. A cap reusing an existing `break` leaves a
        //    NON-EMPTY top level behind and still reports `completed`.
        assert!(sweep.halted.is_none(), "no budget may breach at this shape");
        assert!(sweep.completed());
        assert!(
            sweep.levels.last().is_some_and(|l| l.frequent.is_empty()),
            "a completed sweep must end on an EMPTY level: the ladder stopped \
             because the frontier emptied, not because something told it to"
        );

        // 2. EVERY LEVEL ACCOUNTS FOR EVERY CANDIDATE, at a width of 10,090.
        for level in &sweep.levels {
            // The counters are read into locals rather than passed as lazy format
            // arguments: an argument only evaluated when the assertion FAILS is a
            // region no passing run executes, and the 100% floor counts it.
            let kept = level.frequent.len();
            assert!(
                level.reconciles(),
                "level {} generated {} and accounts for {} + {} + {} + {} + {kept}",
                level.k,
                level.generated,
                level.duplicates,
                level.excluded,
                level.pruned,
                level.infrequent,
            );
        }

        // 3. THE KEPT SET IS THE BRUTE-FORCE SET. Counted with `support` over the
        //    ROW-MAJOR column, the layout the sweep does NOT use -- so a
        //    truncation inside `Column::support` is a disagreement and not a
        //    shared mistake. All 65,535 non-empty subsets, no Apriori anywhere.
        let mut brute: HashSet<ConditionMask> = HashSet::new();
        for subset in 1_u32..(1 << 16) {
            let mask = DEEP
                .iter()
                .enumerate()
                .fold(ConditionMask::default(), |m, (i, &b)| {
                    if subset & (1 << i) == 0 {
                        m
                    } else {
                        m.with_bit(b)
                    }
                });
            if support(&column, &mask) >= MIN_HITS {
                brute.insert(mask);
            }
        }
        let kept: HashSet<ConditionMask> = sweep.all_frequent().map(|i| i.mask).collect();
        let dropped = brute.difference(&kept).count();
        let invented = kept.difference(&brute).count();
        let total = brute.len();
        assert_eq!(
            (dropped, invented),
            (0, 0),
            "the ladder dropped {dropped} frequent sets the brute force found and \
             invented {invented} it did not, out of {total} -- a silent truncation"
        );
    }

    /// A frontier holding the same mask twice is still counted once.
    ///
    /// # Why this reaches for `next_level` directly
    ///
    /// The prefix join reaches every k-set from exactly one pair, so no walk
    /// this crate can perform will ever increment `duplicates` — which left the
    /// duplicate arm as code no test could reach through `walk`. An unreachable
    /// branch is not a safety net; it is an untested one, and the 100% floor in
    /// `CLAUDE.md` §9 is right to refuse it.
    ///
    /// The arm is reachable, by the one input that should reach it: a MALFORMED
    /// frontier. `{0,1}` twice plus `{0,2}` all share the prefix `{0}`, so the
    /// block yields the pair `({0,1}, {0,2})` twice and the second is rejected
    /// rather than evaluated against the bars a second time. That the join
    /// survives a frontier it should never be handed is worth pinning on its own
    /// -- it is the difference between wasted work and a double-counted level.
    #[test]
    fn the_same_mask_twice_in_a_frontier_is_still_evaluated_once() {
        let b = bars(&[&[0, 1, 2], &[0, 1, 2], &[0, 1, 2], &[3]]);
        let column = Column::transpose(&b);
        let one = |bits: &[u32]| Itemset {
            mask: bits
                .iter()
                .fold(ConditionMask::default(), |m, &x| m.with_bit(x)),
            hits: 3,
        };
        // Deliberately malformed: {0,1} appears twice.
        let prev = Frontier {
            k: 2,
            frequent: vec![one(&[0, 1]), one(&[0, 1]), one(&[0, 2])],
            generated: 0,
            duplicates: 0,
            excluded: 0,
            pruned: 0,
            infrequent: 0,
        };
        let (level, halted, _, _) = Ladder::with_min_hits(1).next_level(&column, &prev, 3, 0, 0);

        assert!(halted.is_none(), "the fixture must not breach a budget");
        assert_eq!(
            level.duplicates, 1,
            "the repeated mask produces {{0,1,2}} twice and the second must be \
             rejected, not evaluated against the bars again"
        );
        assert!(
            level.reconciles(),
            "and the level must still account for every candidate it generated"
        );
    }

    #[test]
    fn exhausted_names_the_ceiling_first_and_the_allocator_second() {
        let mut seen: HashSet<ConditionMask> = HashSet::new();
        // Ceiling wins when both could fire: a caller that set one must get the
        // breach it asked for, not a memory report from an allocator that was
        // never going to refuse.
        // A ceiling of ZERO, because the test is now `seen.len() + grow > ceiling`
        // and `seen` is empty here: at a ceiling of one, growing by one is
        // exactly at the bound and passes. Zero is the only ceiling an empty set
        // can breach, and it is the honest fixture for "the ceiling answers
        // first".
        let tight = Ladder::with_min_hits(1).with_ceiling(1);
        assert_eq!(
            tight.exhausted(&mut seen, 1, usize::MAX),
            Some(Breach::Candidates)
        );
        // With room in the ceiling, the allocator is what answers.
        let roomy = Ladder::with_min_hits(1);
        assert_eq!(
            roomy.exhausted(&mut seen, 0, usize::MAX),
            Some(Breach::Memory),
            "an allocation the machine cannot satisfy is a halt naming MEMORY, \
             and it is reachable here without a machine that is out of it"
        );
        // And an ordinary candidate passes both.
        assert_eq!(roomy.exhausted(&mut seen, 0, 1), None);
    }

    #[test]
    fn the_allocator_refusing_to_grow_is_a_halt_and_not_a_panic() {
        // Both answers, on an empty set, in microseconds. `try_reserve(usize::MAX)`
        // fails on CAPACITY OVERFLOW without asking the OS for anything, so the
        // refusal arm is provable without a machine that is actually out of
        // memory -- which is the only reason this is a function rather than two
        // lines inlined at the call site.
        let mut seen: HashSet<ConditionMask> = HashSet::new();
        assert_eq!(
            seen.len(),
            seen.capacity(),
            "a fresh set is exactly full at zero"
        );
        assert!(
            cannot_grow(&mut seen, usize::MAX),
            "a reservation the allocator cannot satisfy must REPORT, not abort -- \
             `CLAUDE.md` §4 wants the reason named, and a panic names nothing a \
             caller can read"
        );
        assert!(
            !cannot_grow(&mut seen, 1),
            "and an ordinary growth must be allowed through"
        );
        // Once it has room, the check costs a compare and reserves nothing.
        assert!(seen.capacity() >= 1);
        assert!(
            !cannot_grow(&mut seen, 1),
            "not full, so no reserve is attempted"
        );
    }

    #[test]
    fn without_highest_clears_the_top_bit_and_leaves_an_empty_mask_alone() {
        let empty = ConditionMask::default();
        assert_eq!(
            without_highest(&empty),
            empty,
            "nothing set means nothing to clear -- the walk never supplies this, \
             so it is proved here rather than left as a branch no run reaches"
        );
        // Highest is cleared, not lowest, and not merely any one bit.
        let m = ConditionMask::default()
            .with_bit(3)
            .with_bit(70)
            .with_bit(200);
        assert_eq!(
            without_highest(&m),
            ConditionMask::default().with_bit(3).with_bit(70),
            "the key must drop the TOP position, across word boundaries"
        );
        // A single bit reduces to empty, which is what puts every 1-set of the
        // k=1 frontier into one block and makes k=2 the exhaustive level it is.
        assert_eq!(
            without_highest(&ConditionMask::default().with_bit(5)),
            empty
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
