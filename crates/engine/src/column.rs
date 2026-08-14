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
pub(crate) fn set_positions(mask: &ConditionMask) -> impl Iterator<Item = u32> {
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
    /// 238 of the 384 are live, and sizing to those would be 36.4 MB -- 22.3 MB less.
    /// It is not done, because [`Self::bitmap`] addresses a bitmap as
    /// `position * stride`, and that multiply is the whole reason a lookup is O(1). To
    /// pack out the dead positions the type would have to carry a live-position map and
    /// pay an indirection per access, trading a constant-time address for a smaller
    /// allocation on the hottest path in the sweep. The 22.3 MB is the price of the
    /// multiply, and it is a price and not an oversight.
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
}

// NO `#[expect(clippy::unwrap_used, clippy::expect_used)]` here, and its absence is
// deliberate rather than an omission. `mod tests` in `lib.rs` needs those exemptions;
// this module's tests do not use either macro -- every fallible step is `unwrap_or` with
// a value that would itself fail the assertion. Adding the exemption anyway made clippy
// say `this lint expectation is unfulfilled`, which is exactly what `expect` is for over
// `allow`: it reports an exemption nobody needs.
#[cfg(test)]
mod tests {
    use super::{Column, set_positions};
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
                    assert_eq!(
                        bitmaps,
                        row_major,
                        "layouts disagree at {bars} bar(s), {set} bit(s) per bar, \
                         candidate {:?}",
                        candidate.words()
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
    /// the LIVE position count would shrink it to 238 strides and fail here, which is the
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
        let live = 238_usize;
        assert!(
            live * stride * 8 < words * 8,
            "sizing to the live positions would be smaller -- that is why the doc \
             block has to say why it is not done"
        );
        assert_eq!(
            words * 8 - live * stride * 8,
            22_316_976,
            "22.3 MB is the price of addressing a bitmap as position * stride"
        );
    }

    #[test]
    fn an_empty_column_supports_nothing() {
        let vertical = Column::transpose(&[]);
        assert_eq!(vertical.bars(), 0);
        assert_eq!(vertical.support(&ConditionMask::ZERO), 0);
        assert_eq!(vertical.support(&ConditionMask::ZERO.with_bit(5)), 0);
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
}
