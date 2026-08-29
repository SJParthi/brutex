//! The bar column, transposed: one bitmap per condition position.
//!
//! # Why a second layout for the same data
//!
//! [`crate::support`] holds the column row-major — one 48-byte [`ConditionMask`] per
//! bar — and answers `support(A)` by walking every bar and asking `hits`. That is
//! correct, it is O(bars) by definition, and its per-bar cost is flat in `k`
//! (measured: `C-E-02`). It has one property that does not show up in a per-bar
//! ratio: **every candidate re-reads the whole column.**
//!
//! At 1,222,791 bars that is 58.7 MB per candidate. The same data stored as one
//! bitmap per position is 149 KB per position, 36.4 MB for all 238 — and
//! `support(A)` then reads only the `k` bitmaps the candidate names. At k=4 that is
//! 596 KB instead of 58.7 MB, **96 times fewer bytes moved**, and the arithmetic is
//! the same AND that `hits` already does.
//!
//! This is the standard vertical format for frequent-itemset mining. It fits here
//! unusually well because `hits` is already pure bitwise over `[u64; WORDS]`, so the
//! transpose changes the LAYOUT and not the semantics: anti-monotonicity, the
//! level-wise join and the subset prune are untouched, and
//! `the_two_layouts_agree_on_every_candidate` is what says so rather than this
//! paragraph.
//!
//! # What it does not change
//!
//! Support is still O(bars). Nothing can make it otherwise -- it IS the measurement,
//! and a column of N bars cannot be counted in fewer than N steps. What changes is
//! the constant, and the constant is what decides whether a level takes minutes or
//! days.
//!
//! # Padding
//!
//! A bitmap holds `ceil(bars / 64)` words, so the last word has `bars % 64` real
//! bits and up to 63 padding bits. **Padding is zero and stays zero.** That is safe
//! because this module only ever ANDs bitmaps together: a zero can never become a
//! one, so a padding bit can never be counted as a hit. If an OR or a NOT is ever
//! added here, the tail needs an explicit mask and this comment stops being true.

use vocab::ConditionMask;

/// One `u64` of a bitmap covers this many bars.
const BARS_PER_WORD: usize = 64;

/// Every position set in `mask`, low to high.
///
/// `ConditionMask` exposes no bit iterator -- `every_subset_is_frequent` says as much
/// and pays 384 probes for the lack of one. This walks the words instead, clearing
/// the lowest set bit each step, so it costs one iteration per SET bit rather than
/// one per possible bit. It lives here rather than in `crates/vocab` because
/// `words()` is already public and this needs no new API surface there.
/// Made `pub` so `crates/runner` can walk a mask's set bits without the 384
/// probes a `get`-loop would cost. It reads no bar and touches no filesystem, so
/// gate 22 is unaffected: the boundary that gate defends is what this crate may
/// DEPEND on, not what it may expose.
pub fn set_positions(mask: &ConditionMask) -> impl Iterator<Item = u32> {
    mask.words()
        .into_iter()
        .enumerate()
        .flat_map(|(index, word)| {
            let mut remaining = word;
            let base = u32::try_from(index).unwrap_or(0).saturating_mul(64);
            core::iter::from_fn(move || {
                if remaining == 0 {
                    return None;
                }
                let bit = remaining.trailing_zeros();
                // Clear the lowest set bit. `wrapping_sub` and not `- 1` because
                // `remaining` is non-zero here and clippy cannot see that.
                remaining &= remaining.wrapping_sub(1);
                Some(base.saturating_add(bit))
            })
        })
}

/// A bar column stored as one bitmap per condition position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    /// How many bars the column holds. The denominator every support is over.
    bars: u64,
    /// Words in one position's bitmap. `ceil(bars / 64)`.
    stride: usize,
    /// `ConditionMask::BITS` bitmaps end to end; position `p` occupies
    /// `p * stride .. (p + 1) * stride`.
    bits: Vec<u64>,
}

impl Column {
    /// Transposes a row-major column into one bitmap per position.
    ///
    /// Allocates `BITS * ceil(bars / 64)` words once and makes no other allocation.
    /// At 1,222,791 bars the stride is 19,107 words, so the figure is
    /// `384 * 19_107 * 8` = **58.7 MB**. Proved by
    /// `engine::column::the_allocation_is_bits_by_stride_and_not_the_live_count`.
    ///
    /// THE FIRST FACTOR IS `BITS` AND NOT THE LIVE POSITION COUNT, deliberately. Only
    /// **328** of the 384 are live, and sizing to those would be 50.1 MB -- 8.6 MB less.
    /// It is not done, because [`Self::bitmap`] addresses a bitmap as
    /// `position * stride`, and that multiply is the whole reason a lookup is O(1). To
    /// pack out the dead positions the type would have to carry a live-position map and
    /// pay an indirection per access, trading a constant-time address for a smaller
    /// allocation on the hottest path in the sweep. The 8.6 MB is the price of the
    /// multiply, and it is a price and not an oversight.
    ///
    /// **The price SHRINKS as the vocabulary fills, and these numbers moved.** They read
    /// 238 live / 36.4 MB / 22.3 MB, correct when the table held 280 positions and 104
    /// were free. D-0244 and D-0246 took it to 365 with 19 free, so what the multiply
    /// buys is now paid for with 19 empty positions instead of 104. The test beside this
    /// derives the live count from `vocab::table::LIVE` rather than repeating it, because
    /// it previously compared a literal `238` against arithmetic on the same literal and
    /// stayed green through both growths.
    ///
    /// Pre-sized rather than grown, which is what gate 11 rule 3 asks of every
    /// collection on an O(1) path.
    #[must_use]
    pub fn transpose(bar_bits: &[ConditionMask]) -> Self {
        let bars = bar_bits.len();
        let stride = bars.div_ceil(BARS_PER_WORD);
        let positions = usize::try_from(ConditionMask::BITS).unwrap_or(0);
        let mut bits = vec![0_u64; positions.saturating_mul(stride)];
        for (bar, mask) in bar_bits.iter().enumerate() {
            let word = bar / BARS_PER_WORD;
            let bit = 1_u64 << (bar % BARS_PER_WORD);
            for position in set_positions(mask) {
                let at = usize::try_from(position)
                    .unwrap_or(0)
                    .saturating_mul(stride)
                    .saturating_add(word);
                if let Some(slot) = bits.get_mut(at) {
                    *slot |= bit;
                }
            }
        }
        Self {
            bars: u64::try_from(bars).unwrap_or(u64::MAX),
            stride,
            bits,
        }
    }

    /// How many bars this column holds.
    #[must_use]
    pub const fn bars(&self) -> u64 {
        self.bars
    }

    /// How many bars carry every bit `candidate` requires.
    ///
    /// The same answer [`crate::support`] gives, by the same rule -- a bar hits iff it
    /// carries every required bit -- reading `k` bitmaps instead of the whole column.
    ///
    /// An EMPTY candidate requires nothing and so hits every bar, which is the same
    /// convention `hits` uses: `(bits & 0) == 0` is true for every bar. Returning
    /// `bars` here rather than 0 is what keeps the two layouts in agreement, and
    /// `the_two_layouts_agree_on_every_candidate` covers it explicitly.
    #[must_use]
    pub fn support(&self, candidate: &ConditionMask) -> u64 {
        // THE EMPTY MASK, ANSWERED IN O(1) AND NOT BY FALLING THROUGH THE LOOP.
        // A mask requiring nothing is matched by every bar; `popcount` is a
        // register operation, so this costs nothing and it keeps the loop below
        // able to assume at least one AND happens -- which is what makes the
        // padding argument hold.
        if candidate.popcount() == 0 {
            return self.bars;
        }

        // THE POSITIONS, DERIVED ONCE, ON THE STACK.
        //
        // This was `let rest: Vec<u32> = required.collect()` -- a HEAP ALLOCATION
        // PER CALL, and since 7461f57 wired this function into `Ladder::walk` that
        // is one allocation per candidate per level, which is billions in a real
        // sweep. `CLAUDE.md` §3.4 requires a constant per-operation cost and gate
        // 11 rule 3 refuses an unsized collection on an O(1) path; an allocation
        // is neither constant nor sized. A fixed array indexed by `zip` is both,
        // costs 3,072 bytes of stack -- 384 `usize`, not the 1,536 this comment
        // claimed for months, which is the figure for `[u32; 384]` and not what
        // is declared below -- and cannot be larger because a mask cannot
        // name more positions than it has bits.
        // BASE OFFSETS, DERIVED ONCE -- not a bitmap slice per (word, position).
        //
        // The first version called `self.bitmap(position)` inside the word loop.
        // That is a `try_from`, a `saturating_mul`, a `saturating_add`, a range
        // bounds check and a slice construction, `stride * k` times: at 1,222,791
        // bars the stride is 19,107, so a k=8 candidate re-derived the same eight
        // slices 152,856 times. An audit measured the waste at 2.19x-3.25x on the
        // single hottest path in the sweep.
        //
        // The address of position `p`'s word `w` is `p * stride + w`. Only the
        // first term depends on the position, so it is computed once per candidate
        // and the inner loop is one add and one bounds-checked read.
        let mut bases = [0_usize; ConditionMask::BITS as usize];
        let mut count = 0_usize;
        for (slot, position) in bases.iter_mut().zip(set_positions(candidate)) {
            *slot = usize::try_from(position)
                .unwrap_or(0)
                .saturating_mul(self.stride);
            count = count.saturating_add(1);
        }

        let mut hits = 0_u32;
        for word in 0..self.stride {
            // NO SHORT-CIRCUIT, AND ITS REMOVAL IS THE POINT.
            //
            // This loop used to `break` when the accumulator reached zero, which
            // made the cost of a support count depend on the ANSWER: a candidate
            // that misses early was cheaper than one that matches. `C-E-03` is the
            // row that exists to refuse exactly that -- "the per-bar cost does not
            // depend on the answer" -- and an audit measured the live function at
            // 6.384x against its 3.0x ceiling while `C-E-03` read 1.000x, because
            // the row was still measuring the row-major function the sweep no
            // longer calls. A sweep whose runtime tracks the market rather than
            // the bar count cannot be budgeted, which is the whole reason the
            // invariant is written down.
            //
            // Starting from the FIRST named bitmap rather than `u64::MAX` is what
            // keeps padding zero: every real bitmap has zeroes in the tail bits, so
            // an AND of real bitmaps does too. The `popcount == 0` guard above is
            // what guarantees there is a first one.
            let mut acc = u64::MAX;
            for base in bases.iter().take(count) {
                acc &= self
                    .bits
                    .get(base.saturating_add(word))
                    .copied()
                    .unwrap_or(0);
            }
            hits = hits.saturating_add(acc.count_ones());
        }
        u64::from(hits)
    }

    /// The support count AND the identity of the bars it counted, in ONE pass.
    ///
    /// # Why this exists, and what it measured
    ///
    /// [`crate::Ladder::walk`]'s `seen` set rejects a duplicate by its MASK -- which
    /// bits a candidate names. Two candidates that name different bits and select the
    /// SAME BARS are not duplicates to it, and they are the same hypothesis to
    /// everything downstream: the same trades, the same mean, the same t-statistic.
    ///
    /// MEASURED, on the 60min zerodha NIFTY run of 2019-12..2026-08 that reached k=8:
    /// the report's `top 10000 best by |t|` held **221 distinct `(hits, n, mean)`
    /// signatures and 95 distinct t-statistics**. Ranks 5 through 9 were one finding
    /// five times, each copy differing only in which `close_below_pivot_r*_band` was
    /// bolted on -- a bit that changed the mask and not one bar. A single trade set,
    /// `hits=232 n=137 mean=355`, occupied **1,781 of the 10,000 rows**.
    ///
    /// That is not a ranking. It is one finding, printed until the page is full.
    ///
    /// # And it moves the significance bar, which is the part that changes an answer
    ///
    /// The same run counted `84,430,203` hypotheses and set its Bonferroni floor at
    /// `t = 6.19` from that count. A hypothesis that is not distinct is not a trial,
    /// so a trial count inflated by redundancy raises the bar every REAL finding must
    /// clear. The count this function makes available is the honest denominator.
    ///
    /// # Why it is free
    ///
    /// [`Self::support`] already computes `acc` -- the AND of the named bitmaps -- for
    /// every word, and `acc` IS the hit set. The fold below reads that register and
    /// touches no memory the support count was not already touching, so this costs the
    /// same bytes moved as [`Self::support`] and adds two register operations per word.
    ///
    /// # The cost does not depend on the answer
    ///
    /// Every word is folded, including the zero words of a sparse hit set. Skipping
    /// zeroes would make a candidate that misses early cheaper than one that matches,
    /// which is the property `C-E-03` exists to refuse -- see the no-short-circuit
    /// comment in [`Self::support`], which this loop inherits for the same reason.
    #[must_use]
    pub fn support_fingerprinted(&self, candidate: &ConditionMask) -> (u64, HitSet) {
        // THE EMPTY MASK SELECTS EVERY BAR, and it needs a fingerprint like any
        // other candidate rather than falling through the loop -- which would fold
        // over zero words and hand back the seed, the fingerprint of a hit set that
        // selects NOTHING. Those two are opposites, so they must not collide.
        // `HitSet::EVERY_BAR` is a distinct reserved value and never a fold output.
        if candidate.popcount() == 0 {
            return (self.bars, HitSet::EVERY_BAR);
        }

        let mut bases = [0_usize; ConditionMask::BITS as usize];
        let mut count = 0_usize;
        for (slot, position) in bases.iter_mut().zip(set_positions(candidate)) {
            *slot = usize::try_from(position)
                .unwrap_or(0)
                .saturating_mul(self.stride);
            count = count.saturating_add(1);
        }

        let mut hits = 0_u32;
        let mut lo = FINGERPRINT_SEED_LO;
        let mut hi = FINGERPRINT_SEED_HI;
        for word in 0..self.stride {
            let mut acc = u64::MAX;
            for base in bases.iter().take(count) {
                acc &= self
                    .bits
                    .get(base.saturating_add(word))
                    .copied()
                    .unwrap_or(0);
            }
            hits = hits.saturating_add(acc.count_ones());
            // TWO INDEPENDENT FOLDS, and the rotation is what makes them ORDERED.
            // Multiply-xorshift alone is commutative over a set of words, so two
            // different hit sets holding the same words in a different order would
            // fold to the same value. The rotate makes each step depend on how many
            // words came before it, so word order is part of the identity -- and
            // word order is bar order.
            lo = (lo ^ acc).wrapping_mul(FINGERPRINT_MIX_LO);
            lo ^= lo >> 29;
            hi = (hi.rotate_left(23) ^ acc).wrapping_mul(FINGERPRINT_MIX_HI);
        }
        (u64::from(hits), HitSet { lo, hi })
    }
}

/// The identity of a SET OF BARS: 128 bits folded over the bitmap the candidate
/// selects, by [`Column::support_fingerprinted`].
///
/// # Why 128 bits and not 64
///
/// Because the birthday bound is the whole argument, and at this scale 64 is not
/// enough. A collision does not merely miscount -- it DISCARDS a genuine finding by
/// declaring it a duplicate of an unrelated one, silently, which is the failure mode
/// `CLAUDE.md` §4 bans outright.
///
/// The run that motivated this weighed `84,097,159` combinations. At 64 bits the
/// birthday probability over N = 8.4e7 is about `N^2 / 2^65` -- roughly **2 in 10,000**,
/// which is small but is a coin flip against a real answer that nobody would ever see
/// land. At 128 bits the same expression is `N^2 / 2^129`, about `1e-23`: not a promise
/// that it cannot happen, but a number small enough to write down and defend.
///
/// This type is deliberately NOT `Ord`. There is no meaningful order over hit-set
/// identities -- sorting by one would impose a ranking that no document defines, which
/// is the same reason [`crate::Frontier::frequent`] refuses to sort by edge.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct HitSet {
    lo: u64,
    hi: u64,
}

impl HitSet {
    /// The hit set of a candidate that requires nothing and therefore selects every
    /// bar. Reserved, and unreachable as a fold output: the fold always ends with a
    /// `wrapping_mul` by an odd constant, so it can only produce zero in `hi` from a
    /// zero input to that multiply, and `lo`'s final xorshift cannot produce the low
    /// half either. `the_empty_and_the_impossible_never_collide` is what says so.
    pub const EVERY_BAR: Self = Self { lo: 0, hi: 0 };

    /// The two halves, for a caller that must persist or transmit an identity.
    ///
    /// Exposed as a pair rather than as fields so the fold's internals stay private:
    /// a future change to the mixing constants must not be a breaking change to
    /// anything that merely stores what it was handed.
    #[must_use]
    pub const fn halves(self) -> (u64, u64) {
        (self.lo, self.hi)
    }
}

/// Seeds for the two folds. FIXED, for the reason [`crate::MASK_HASH_SEED`] is fixed:
/// §3.5 requires the same inputs to give the same outputs on every run and every
/// machine, and a seed drawn from the environment would break that.
const FINGERPRINT_SEED_LO: u64 = 0xcbf2_9ce4_8422_2325;
/// The second fold's seed. Different from the first, so the two halves of a
/// fingerprint are not the same function of the same input.
const FINGERPRINT_SEED_HI: u64 = 0x9e37_79b9_7f4a_7c15;
/// The low fold's multiplier. Odd, so the multiply is a bijection on `u64` and cannot
/// fold two distinct inputs together on its own -- the same property
/// [`crate::MASK_HASH_MIX`] is chosen for.
const FINGERPRINT_MIX_LO: u64 = 0x100_0000_01b3;
/// The high fold's multiplier. Also odd, and different, so a candidate cannot produce
/// two equal halves except by coincidence.
const FINGERPRINT_MIX_HI: u64 = 0xff51_afd7_ed55_8ccd;

// NO `#[expect(clippy::unwrap_used, clippy::expect_used)]` here, and its absence is
// deliberate rather than an omission. `mod tests` in `lib.rs` needs those exemptions;
// this module's tests do not use either macro -- every fallible step is `unwrap_or` with
// a value that would itself fail the assertion. Adding the exemption anyway made clippy
// say `this lint expectation is unfulfilled`, which is exactly what `expect` is for over
// `allow`: it reports an exemption nobody needs.
#[cfg(test)]
mod tests {
    use super::{Column, HitSet, set_positions};
    use vocab::ConditionMask;

    /// A deterministic bit source. No `rand`: §3.5 wants the same inputs to give the
    /// same outputs on every run, and a seeded LCG written here is reproducible in a
    /// way a thread-local generator is not.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            // Numerical Recipes' constants. Any full-period LCG does; what matters is
            // that it is written down rather than drawn from the environment.
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0
        }
        fn below(&mut self, n: u32) -> u32 {
            if n == 0 {
                0
            } else {
                (self.next() >> 33) as u32 % n
            }
        }
    }

    /// A column of `bars` masks, each carrying `set` positions drawn from `live`.
    fn column(seed: u64, bars: usize, live: &[u32], set: usize) -> Vec<ConditionMask> {
        let mut rng = Lcg(seed);
        (0..bars)
            .map(|_| {
                let mut m = ConditionMask::ZERO;
                for _ in 0..set {
                    let pick = live
                        .get(rng.below(u32::try_from(live.len()).unwrap_or(1)) as usize)
                        .copied()
                        .unwrap_or(0);
                    m = m.with_bit(pick);
                }
                m
            })
            .collect()
    }

    /// `set_positions` yields exactly the bits that are set, in order.
    #[test]
    fn set_positions_yields_every_set_bit_and_nothing_else() {
        for bits in [
            vec![],
            vec![0_u32],
            vec![63, 64],
            vec![0, 1, 62, 63, 64, 65, 127, 128, 383],
            (0..ConditionMask::BITS).step_by(7).collect(),
        ] {
            let mask = bits.iter().fold(ConditionMask::ZERO, |m, b| m.with_bit(*b));
            let got: Vec<u32> = set_positions(&mask).collect();
            let mut want = bits.clone();
            want.sort_unstable();
            want.dedup();
            assert_eq!(got, want, "for {bits:?}");
            assert_eq!(
                u32::try_from(got.len()).unwrap_or(0),
                mask.popcount(),
                "the count must agree with popcount, which is computed differently"
            );
        }
    }

    /// **The two layouts agree on every candidate.**
    ///
    /// This is the whole justification for the module existing: the transpose is a
    /// change of LAYOUT, so any disagreement with [`crate::support`] is a bug in the
    /// transpose rather than a design choice. Compared against the row-major walk over
    /// every candidate the ladder could build at k=1..4, on columns whose bar counts
    /// straddle the 64-bar word boundary in both directions.
    #[test]
    fn the_two_layouts_agree_on_every_candidate() {
        // Positions spanning five of the six mask words, including both sides of the
        // 63->64 boundary, so a word-index slip in either layout shows up.
        const LIVE: [u32; 8] = [0, 1, 63, 64, 127, 192, 233, 279];
        // Bar counts either side of a multiple of 64: the tail word is where a padding
        // bit would be counted as a hit if padding were not zero.
        const BARS: [usize; 7] = [1, 63, 64, 65, 127, 128, 300];

        let mut checked = 0_u32;
        for (seed, bars) in BARS.iter().enumerate() {
            for set in [1_usize, 3, 8] {
                let rows = column(seed as u64 * 7 + 1, *bars, &LIVE, set);
                let vertical = Column::transpose(&rows);
                assert_eq!(
                    vertical.bars(),
                    u64::try_from(*bars).unwrap_or(0),
                    "the bar count must survive the transpose"
                );
                // Every candidate at k=1 and k=2, plus a k=3 and a k=4, and the empty
                // mask -- which requires nothing and therefore hits every bar.
                let mut candidates = vec![ConditionMask::ZERO];
                for a in LIVE {
                    candidates.push(ConditionMask::ZERO.with_bit(a));
                    for b in LIVE {
                        candidates.push(ConditionMask::ZERO.with_bit(a).with_bit(b));
                    }
                }
                candidates.push(ConditionMask::ZERO.with_bit(0).with_bit(64).with_bit(233));
                candidates.push(
                    ConditionMask::ZERO
                        .with_bit(1)
                        .with_bit(63)
                        .with_bit(127)
                        .with_bit(279),
                );
                for candidate in candidates {
                    let row_major = crate::support(&rows, &candidate);
                    let bitmaps = vertical.support(&candidate);
                    // Read once, into a name the message interpolates. Spelled
                    // inside the message, `candidate.words()` is an expression only
                    // a FAILING run evaluates -- and this comparison is the module's
                    // whole justification, so it is the one assertion that must
                    // never fail. `crates/engine/src/lib.rs` hoists `kept_count` out
                    // of E-02's comparison for the same reason; the numbers are the
                    // same and the message keeps them.
                    let shown = candidate.words();
                    assert_eq!(
                        bitmaps, row_major,
                        "layouts disagree at {bars} bar(s), {set} bit(s) per bar, \
                         candidate {shown:?}"
                    );
                    checked = checked.saturating_add(1);
                }
            }
        }
        assert!(
            checked > 1_000,
            "only {checked} comparisons ran, which is too few to have covered the \
             boundaries this test names"
        );
    }

    /// An empty candidate hits every bar, and an impossible one hits none.
    ///
    /// Both are conventions rather than arithmetic, so both are pinned. `hits` is
    /// `(bits & mask) == mask`, which is trivially true for an empty mask -- so the
    /// transpose must return `bars`, not 0, and it is the one case that does not go
    /// through a bitmap at all.
    #[test]
    fn the_empty_candidate_hits_everything_and_a_dead_bit_hits_nothing() {
        let rows = column(9, 200, &[3, 70, 150], 2);
        let vertical = Column::transpose(&rows);
        assert_eq!(vertical.support(&ConditionMask::ZERO), 200);
        assert_eq!(crate::support(&rows, &ConditionMask::ZERO), 200);

        // A position no bar carries: its bitmap is all zero, so the AND is zero.
        let dead = ConditionMask::ZERO.with_bit(300);
        assert_eq!(vertical.support(&dead), 0);
        assert_eq!(crate::support(&rows, &dead), 0);
    }

    /// A column with no bars answers 0 for everything, including the empty candidate.
    /// The allocation is `BITS * stride`, and the doc block on [`Column::transpose`]
    /// states the byte figure it implies.
    ///
    /// Guards the claim in two directions at once. A change that sized the allocation to
    /// the LIVE position count would shrink it to 328 strides and fail here, which is the
    /// trade the doc block argues against; and a change to `WORDS` or to `BARS_PER_WORD`
    /// moves the figure, so the number in the prose cannot rot while the code moves. The
    /// arithmetic is written out rather than recomputed from the same expression the code
    /// uses, because a test that repeats the implementation asserts nothing.
    #[test]
    fn the_allocation_is_bits_by_stride_and_not_the_live_count() {
        let bars = 1_222_791_usize;
        let stride = 19_107_usize;
        assert_eq!(
            bars.div_ceil(64),
            stride,
            "ceil(1222791 / 64) is 19107; if this moved, BARS_PER_WORD moved"
        );
        // COMPARED AS `u32`, so no conversion is needed and no `expect` appears. The
        // test module here carries none of the panic exemptions `lib.rs`'s does, and
        // that is deliberate -- see the note above `mod tests`.
        assert_eq!(ConditionMask::BITS, 384, "the mask is 6 words of 64 bits");
        let words = 384_usize * stride;
        assert_eq!(words, 7_337_088, "384 x 19107");
        assert_eq!(
            words * 8,
            58_696_704,
            "58.7 MB, which is the figure `transpose`'s doc block states"
        );
        // AND THE LIVE COUNT WOULD BE SMALLER, which is the trade being refused.
        //
        // DERIVED FROM THE TABLE, NOT WRITTEN DOWN. This read `let live =
        // 238_usize;` and the assertion below was computed from it. The
        // vocabulary has grown twice since — D-0244 and D-0246 took it to 323
        // live — and the test stayed GREEN the whole time, because it was
        // comparing a literal against arithmetic on the same literal. A test
        // that certifies its own stale constant is worse than no test: it makes
        // the figure look checked.
        //
        // AND THEN IT WENT STALE IN THE OTHER DIRECTION, which is the failure
        // the paragraph above describes happening to the paragraph above. Once
        // `live` was read from the table, the LITERAL it is compared against
        // became the thing that could rot — and it did: the five weekday rows
        // took the table to 328 while this still said 323, so `cargo test -p
        // engine` failed against a vocabulary nobody had changed here. Reading
        // one side from the source does not make the other side self-checking;
        // it moves which side has to be maintained. `vocab::table::
        // the_live_mask_is_the_table` is the row that owns this number, and it
        // says 328 -- "323 + the five weekday bits".
        let live = usize::try_from(vocab::table::LIVE.popcount()).unwrap_or(0);
        assert_eq!(live, 328, "the live count, read from the table");
        assert!(
            live * stride * 8 < words * 8,
            "sizing to the live positions would be smaller -- that is why the doc \
             block has to say why it is not done"
        );
        assert_eq!(
            words * 8 - live * stride * 8,
            8_559_936,
            "the price of addressing a bitmap as position * stride. It was 22.3 MB \
             at 238 live and is 8.6 MB at 328: the waste SHRINKS as the vocabulary \
             fills, because what is being paid for is the UNALLOCATED headroom, \
             and there are now 14 free positions instead of 104"
        );
    }

    #[test]
    fn an_empty_column_supports_nothing() {
        let vertical = Column::transpose(&[]);
        assert_eq!(vertical.bars(), 0);
        assert_eq!(vertical.support(&ConditionMask::ZERO), 0);
        assert_eq!(vertical.support(&ConditionMask::ZERO.with_bit(5)), 0);
    }

    /// The fixture generator cannot divide by zero, and does not draw when asked
    /// for nothing.
    ///
    /// # What this is guarding, and why nothing else here reaches it
    ///
    /// `Lcg::below` is the only arithmetic in this module's fixtures, and `% n` on
    /// `n == 0` is not a wrong answer -- it is a panic, in a test helper, which
    /// reports as a failing assertion in whatever test happened to call it. The
    /// `n == 0` arm is the guard against that, and every other test here hands
    /// `column` a non-empty `live` slice, so the guard has never once run: the one
    /// branch written specifically to keep the fixtures from crashing was the one
    /// branch nothing exercised.
    ///
    /// The second half is the part that is easy to get wrong. The guard returns
    /// early WITHOUT calling `next`, so asking for a draw below zero must not
    /// advance the stream -- otherwise a caller that happens to pass an empty slice
    /// perturbs every later draw, and two fixtures written to the same seed stop
    /// agreeing. §3 rule 5 is the reason the generator is written down here at all
    /// rather than taken from the environment, and a guard that silently consumed a
    /// draw would undo that. Compared against a second generator on the same seed
    /// that was never asked, which is the only way to see it.
    #[test]
    fn asking_the_fixture_generator_for_a_draw_below_zero_yields_zero_without_consuming_one() {
        let mut guarded = Lcg(7);
        assert_eq!(
            guarded.below(0),
            0,
            "below(0) has no value it could return but 0, and `% 0` is a panic"
        );
        assert_eq!(
            guarded.below(0),
            0,
            "and it stays 0, however often it is asked"
        );

        let mut untouched = Lcg(7);
        assert_eq!(
            guarded.below(1_000),
            untouched.below(1_000),
            "the two zero-width draws advanced the stream, so an empty `live` slice \
             would change every later draw and two fixtures on one seed would stop \
             agreeing"
        );

        // AND THE GUARD IS REACHED THROUGH THE REAL CALLER, not only by hand.
        // `column` computes the argument as `live.len()`, so an empty slice is what
        // puts a zero there. The masks it then builds fall back on position 0 for
        // every bar -- `live.get(0)` is `None` and `unwrap_or(0)` supplies the bit --
        // so the column is degenerate but well formed, and both layouts must still
        // agree about it.
        let rows = column(11, 130, &[], 3);
        assert_eq!(
            rows.len(),
            130,
            "the generator must still produce every bar"
        );
        let vertical = Column::transpose(&rows);
        assert_eq!(
            vertical.support(&ConditionMask::ZERO.with_bit(0)),
            130,
            "with no live position to draw from, every bar falls back to bit 0"
        );
        assert_eq!(
            vertical.support(&ConditionMask::ZERO.with_bit(1)),
            0,
            "and no other position was ever set"
        );
        assert_eq!(
            vertical.support(&ConditionMask::ZERO.with_bit(0)),
            crate::support(&rows, &ConditionMask::ZERO.with_bit(0)),
            "the two layouts must agree on the degenerate column as well"
        );
    }

    /// The padding bits in the last word are zero, and stay uncounted.
    ///
    /// 65 bars means the second word holds one real bar and 63 padding bits. If padding
    /// were set, a candidate that every bar satisfies would report 128 rather than 65.
    #[test]
    fn padding_in_the_tail_word_is_never_counted() {
        let bars = 65;
        let rows: Vec<ConditionMask> = (0..bars).map(|_| ConditionMask::ZERO.with_bit(7)).collect();
        let vertical = Column::transpose(&rows);
        assert_eq!(
            vertical.support(&ConditionMask::ZERO.with_bit(7)),
            u64::try_from(bars).unwrap_or(0),
            "every bar carries bit 7, so support is the bar count and not the word count \
             times 64"
        );
    }

    /// The whole point of the type: DIFFERENT BITS, SAME BARS, ONE IDENTITY.
    ///
    /// Bit 9 is set on exactly the bars bit 3 is set on, so a candidate naming
    /// `{3}`, one naming `{9}` and one naming `{3,9}` all select the same bars.
    /// `Ladder::walk`'s mask-keyed `seen` calls those three distinct candidates,
    /// and they are one hypothesis.
    #[test]
    fn masks_that_differ_but_select_the_same_bars_share_a_fingerprint() {
        let bars = 300;
        let rows: Vec<ConditionMask> = (0..bars)
            .map(|bar| {
                let mut m = ConditionMask::ZERO.with_bit(1);
                if bar % 3 == 0 {
                    // The redundant pair, always set together -- the shape that put
                    // ranks 5 through 9 of the real run on the same trades.
                    m = m.with_bit(3).with_bit(9);
                }
                m
            })
            .collect();
        let vertical = Column::transpose(&rows);

        let only_three = vertical.support_fingerprinted(&ConditionMask::ZERO.with_bit(3));
        let only_nine = vertical.support_fingerprinted(&ConditionMask::ZERO.with_bit(9));
        let both = vertical.support_fingerprinted(&ConditionMask::ZERO.with_bit(3).with_bit(9));

        assert_eq!(only_three.0, 100, "every third bar of 300");
        assert_eq!(
            (only_three.1, only_nine.1),
            (only_nine.1, both.1),
            "three masks, one hit set, therefore one identity -- this is the equality \
             that collapses a 10,000-row report to its real findings"
        );

        // And a mask selecting a DIFFERENT set must not collide with them.
        let elsewhere = vertical.support_fingerprinted(&ConditionMask::ZERO.with_bit(1));
        assert_eq!(elsewhere.0, u64::try_from(bars).unwrap_or(0));
        assert_ne!(
            elsewhere.1, only_three.1,
            "300 bars and 100 bars are not the same hypothesis"
        );
    }

    /// The count this function returns is the count [`Column::support`] returns.
    ///
    /// Sharing a loop is not the same as sharing an answer, and a fingerprint that
    /// came with a wrong support would be worse than no fingerprint: it would be
    /// silently wrong in the ranking as well as the dedup.
    #[test]
    fn the_fingerprinted_count_is_the_plain_count() {
        const LIVE: [u32; 8] = [0, 1, 63, 64, 127, 192, 233, 279];
        const BARS: [usize; 7] = [1, 63, 64, 65, 127, 128, 300];

        let mut checked = 0_u32;
        for (seed, bars) in BARS.iter().enumerate() {
            for set in [1_usize, 3, 8] {
                let rows = column(seed as u64 * 11 + 1, *bars, &LIVE, set);
                let vertical = Column::transpose(&rows);
                let mut candidates = vec![ConditionMask::ZERO];
                for a in LIVE {
                    candidates.push(ConditionMask::ZERO.with_bit(a));
                    for b in LIVE {
                        candidates.push(ConditionMask::ZERO.with_bit(a).with_bit(b));
                    }
                }
                for candidate in &candidates {
                    let (counted, _) = vertical.support_fingerprinted(candidate);
                    assert_eq!(
                        counted,
                        vertical.support(candidate),
                        "the two functions walk the same words and must agree at \
                         {bars} bars, set {set}"
                    );
                    checked = checked.saturating_add(1);
                }
            }
        }
        assert!(
            checked > 1_000,
            "the loop must actually have run: {checked}"
        );
    }

    /// A fingerprint is a function of the hit set and nothing else -- not of the
    /// call, the process or the order the candidates arrived in. §3.5.
    #[test]
    fn the_same_hit_set_folds_the_same_way_every_time() {
        let rows = column(7, 300, &[0, 1, 63, 64, 127], 3);
        let first = Column::transpose(&rows);
        let second = Column::transpose(&rows);
        let candidate = ConditionMask::ZERO.with_bit(1).with_bit(64);
        assert_eq!(
            first.support_fingerprinted(&candidate),
            second.support_fingerprinted(&candidate),
            "two columns from the same rows are the same column"
        );
        assert_eq!(
            first.support_fingerprinted(&candidate),
            first.support_fingerprinted(&candidate),
            "and asking twice does not move it"
        );
    }

    /// Word ORDER is part of the identity, which is what the rotate buys.
    ///
    /// Two hit sets holding the same words in a different order are different sets
    /// of bars. A commutative fold would call them equal and silently merge two
    /// unrelated findings.
    #[test]
    fn the_fold_is_ordered_so_moving_bars_moves_the_identity() {
        // 128 bars: two full words. Bit 5 on the first word's bars in one column,
        // on the second word's bars in the other -- same popcount, same word
        // VALUES, opposite order.
        let front: Vec<ConditionMask> = (0..128)
            .map(|bar| {
                if bar < 64 {
                    ConditionMask::ZERO.with_bit(5)
                } else {
                    ConditionMask::ZERO
                }
            })
            .collect();
        let back: Vec<ConditionMask> = (0..128)
            .map(|bar| {
                if bar >= 64 {
                    ConditionMask::ZERO.with_bit(5)
                } else {
                    ConditionMask::ZERO
                }
            })
            .collect();
        let candidate = ConditionMask::ZERO.with_bit(5);
        let a = Column::transpose(&front).support_fingerprinted(&candidate);
        let b = Column::transpose(&back).support_fingerprinted(&candidate);
        assert_eq!(a.0, b.0, "both select 64 bars");
        assert_ne!(
            a.1, b.1,
            "the first 64 bars and the last 64 bars are not the same trades, so they \
             must not be the same identity"
        );
    }

    /// The empty mask selects every bar, and the reserved value it returns is not
    /// reachable by folding -- so "everything" can never be mistaken for a real
    /// candidate's hit set, nor for the "nothing" the bare seed would denote.
    #[test]
    fn the_empty_and_the_impossible_never_collide() {
        let bars = 300;
        let rows = column(3, bars, &[0, 1, 63, 64, 127], 2);
        let vertical = Column::transpose(&rows);

        let (counted, identity) = vertical.support_fingerprinted(&ConditionMask::ZERO);
        assert_eq!(
            counted,
            u64::try_from(bars).unwrap_or(0),
            "a candidate requiring nothing is matched by every bar"
        );
        assert_eq!(identity, HitSet::EVERY_BAR);
        assert_eq!(identity.halves(), (0, 0));

        // No real candidate -- including one that happens to select every bar by
        // naming a bit every bar carries -- may fold to the reserved value.
        let all_bars: Vec<ConditionMask> =
            (0..bars).map(|_| ConditionMask::ZERO.with_bit(7)).collect();
        let dense = Column::transpose(&all_bars);
        let (dense_count, dense_identity) =
            dense.support_fingerprinted(&ConditionMask::ZERO.with_bit(7));
        assert_eq!(dense_count, u64::try_from(bars).unwrap_or(0));
        assert_ne!(
            dense_identity,
            HitSet::EVERY_BAR,
            "selecting every bar BY NAMING A BIT is a real hypothesis and must fold, \
             not take the reserved value"
        );

        for candidate in [
            ConditionMask::ZERO.with_bit(0),
            ConditionMask::ZERO.with_bit(63),
            ConditionMask::ZERO.with_bit(1).with_bit(64),
        ] {
            assert_ne!(
                vertical.support_fingerprinted(&candidate).1,
                HitSet::EVERY_BAR,
                "the reserved value is reserved"
            );
        }
    }

    /// A hit set of NOTHING is a real answer -- a candidate whose bits never co-occur
    /// -- and it must have an identity of its own rather than borrowing the empty
    /// mask's, which means the opposite.
    #[test]
    fn selecting_no_bars_is_its_own_identity() {
        // Bit 2 on the even bars, bit 4 on the odd ones: never together.
        let rows: Vec<ConditionMask> = (0..128)
            .map(|bar| {
                if bar % 2 == 0 {
                    ConditionMask::ZERO.with_bit(2)
                } else {
                    ConditionMask::ZERO.with_bit(4)
                }
            })
            .collect();
        let vertical = Column::transpose(&rows);
        let (counted, identity) =
            vertical.support_fingerprinted(&ConditionMask::ZERO.with_bit(2).with_bit(4));
        assert_eq!(counted, 0, "the two bits never co-occur");
        assert_ne!(
            identity,
            HitSet::EVERY_BAR,
            "no bars and every bar are opposites and must not share an identity"
        );
        let (_, halves) = (identity, identity.halves());
        assert_ne!(halves, (0, 0), "and it is not the reserved pair either");
    }
}
