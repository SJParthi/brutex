//! The owned, row-major bar column used by the live sweep.
//!
//! Support counting is necessarily O(bars): counting how many bars match requires
//! looking at the bars. The operation bounded by `AGENTS.md` golden rule 4 is the
//! **mask evaluation on one bar**, and that operation must not grow with candidate
//! depth.
//!
//! [`ConditionMask::hits`] is that operation. It compares the mask's fixed six words
//! with no loop and no early return, so [`Column::support`] calls it exactly once per
//! bar whether the candidate names one condition or all 384 positions. The previous
//! representation stored one bitmap per condition and ANDed the `k` bitmaps a
//! candidate named. It moved fewer bytes, but its live per-bar work was Theta(k),
//! directly contradicting the rule the row-major hit test was written to satisfy.
//!
//! This type therefore owns the rows it is given. A caller that repeats a walk can
//! build it once and reuse it, while every walk still gets the fixed-width evaluation
//! path. `the_fixed_width_column_agrees_with_the_vertical_reference` proves the
//! representation change leaves support counts and hit-set fingerprints unchanged.
//!
//! Fingerprints still group hits into 64-bar words. A final partial word starts at
//! zero and only real bars can set a bit, so all padding bits remain zero.

use vocab::ConditionMask;

/// One packed fingerprint word covers this many bars.
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

/// An owned row-major bar column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    /// One fixed-width condition mask per bar.
    rows: Vec<ConditionMask>,
}

impl Column {
    /// Copies already-computed row masks into a reusable support column.
    ///
    /// The allocation is exactly `bar_bits.len() * size_of::<ConditionMask>()` bytes:
    /// 58.7 MB at 1,222,791 bars. No per-candidate allocation follows. Owning the
    /// rows lets threshold searches reuse one column without tying the public type to
    /// the lifetime of the evaluator's output.
    #[must_use]
    pub fn from_rows(bar_bits: &[ConditionMask]) -> Self {
        Self {
            rows: bar_bits.to_vec(),
        }
    }

    /// How many bars this column holds.
    #[must_use]
    pub fn bars(&self) -> u64 {
        u64::try_from(self.rows.len()).unwrap_or(u64::MAX)
    }

    /// How many bars carry every bit `candidate` requires.
    ///
    /// Exactly one fixed-six-word [`ConditionMask::hits`] call per bar. There is no
    /// candidate-position iterator, no early exit and no work proportional to
    /// `candidate.popcount()`. An empty candidate naturally hits every row because
    /// `(bits & 0) == 0`.
    #[must_use]
    pub fn support(&self, candidate: &ConditionMask) -> u64 {
        self.rows.iter().fold(0_u64, |hits, row| {
            hits.saturating_add(u64::from(row.hits(candidate)))
        })
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
    /// Every row still takes one fixed-width hit test. The hit booleans are packed into
    /// consecutive 64-bar words so this returns the exact identity the former vertical
    /// representation returned, including zero padding in a partial tail word.
    #[must_use]
    pub fn support_fingerprinted(&self, candidate: &ConditionMask) -> (u64, HitSet) {
        // THE EMPTY MASK SELECTS EVERY BAR, and it needs a fingerprint like any
        // other candidate rather than falling through the loop -- which would fold
        // over zero words and hand back the seed, the fingerprint of a hit set that
        // selects NOTHING. Those two are opposites, so they must not collide.
        // `HitSet::EVERY_BAR` is a distinct reserved value and never a fold output.
        if candidate.popcount() == 0 {
            return (self.bars(), HitSet::EVERY_BAR);
        }

        let mut hits = 0_u64;
        let mut lo = FINGERPRINT_SEED_LO;
        let mut hi = FINGERPRINT_SEED_HI;
        for rows in self.rows.chunks(BARS_PER_WORD) {
            let word = rows.iter().enumerate().fold(0_u64, |packed, (bit, row)| {
                packed | (u64::from(row.hits(candidate)) << bit)
            });
            hits = hits.saturating_add(u64::from(word.count_ones()));
            // TWO INDEPENDENT FOLDS, and the rotation is what makes them ORDERED.
            // Multiply-xorshift alone is commutative over a set of words, so two
            // different hit sets holding the same words in a different order would
            // fold to the same value. The rotate makes each step depend on how many
            // words came before it, so word order is part of the identity -- and
            // word order is bar order.
            lo = (lo ^ word).wrapping_mul(FINGERPRINT_MIX_LO);
            lo ^= lo >> 29;
            hi = (hi.rotate_left(23) ^ word).wrapping_mul(FINGERPRINT_MIX_HI);
        }
        (hits, HitSet { lo, hi })
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
    use super::{
        BARS_PER_WORD, Column, FINGERPRINT_MIX_HI, FINGERPRINT_MIX_LO, FINGERPRINT_SEED_HI,
        FINGERPRINT_SEED_LO, HitSet, set_positions,
    };
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

    /// The former position-bitmap algorithm, retained only as an independent
    /// differential oracle for the row-major shipping path.
    ///
    /// This deliberately pays O(candidate popcount × bars): that is the cost the
    /// production implementation must not regain. Keeping it inside `#[cfg(test)]`
    /// lets boundary and fingerprint tests prove result identity without putting the
    /// forbidden shape back on the live path.
    fn vertical_reference(rows: &[ConditionMask], candidate: &ConditionMask) -> (u64, HitSet) {
        if candidate.is_empty() {
            return (
                u64::try_from(rows.len()).unwrap_or(u64::MAX),
                HitSet::EVERY_BAR,
            );
        }

        let mut hit_words = vec![u64::MAX; rows.len().div_ceil(BARS_PER_WORD)];
        for position in set_positions(candidate) {
            for (word, chunk) in rows.chunks(BARS_PER_WORD).enumerate() {
                let bitmap = chunk.iter().enumerate().fold(0_u64, |packed, (bit, row)| {
                    packed | (u64::from(row.get(position)) << bit)
                });
                if let Some(acc) = hit_words.get_mut(word) {
                    *acc &= bitmap;
                }
            }
        }

        let mut hits = 0_u64;
        let mut lo = FINGERPRINT_SEED_LO;
        let mut hi = FINGERPRINT_SEED_HI;
        for word in hit_words {
            hits = hits.saturating_add(u64::from(word.count_ones()));
            lo = (lo ^ word).wrapping_mul(FINGERPRINT_MIX_LO);
            lo ^= lo >> 29;
            hi = (hi.rotate_left(23) ^ word).wrapping_mul(FINGERPRINT_MIX_HI);
        }
        (hits, HitSet { lo, hi })
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

    /// **The fixed-width live path agrees with the former vertical algorithm.**
    ///
    /// Compared against both the free row-major count and an independent test-only
    /// reconstruction of the old position bitmaps. It drives every candidate the
    /// ladder could build at k=1..4 on columns whose bar counts straddle the 64-bar
    /// fingerprint boundary in both directions. Equality covers both the support
    /// count and the durable hit-set identity.
    #[test]
    fn the_fixed_width_column_agrees_with_the_vertical_reference() {
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
                let owned = Column::from_rows(&rows);
                assert_eq!(
                    owned.bars(),
                    u64::try_from(*bars).unwrap_or(0),
                    "the bar count must survive the owned copy"
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
                    let fixed = owned.support_fingerprinted(&candidate);
                    let vertical = vertical_reference(&rows, &candidate);
                    // Read once, into a name the message interpolates. Spelled
                    // inside the message, `candidate.words()` is an expression only
                    // a FAILING run evaluates -- and this comparison is the module's
                    // whole justification, so it is the one assertion that must
                    // never fail. `crates/engine/src/lib.rs` hoists `kept_count` out
                    // of E-02's comparison for the same reason; the numbers are the
                    // same and the message keeps them.
                    let shown = candidate.words();
                    assert_eq!(
                        fixed.0, row_major,
                        "the live count disagrees with the row-major reference at \
                         {bars} bar(s), {set} bit(s) per bar, \
                         candidate {shown:?}"
                    );
                    assert_eq!(
                        fixed, vertical,
                        "the live count or fingerprint disagrees with the former \
                         vertical algorithm at {bars} bar(s), {set} bit(s) per bar, \
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
    /// owned column must return `bars`, not 0.
    #[test]
    fn the_empty_candidate_hits_everything_and_a_dead_bit_hits_nothing() {
        let rows = column(9, 200, &[3, 70, 150], 2);
        let owned = Column::from_rows(&rows);
        assert_eq!(owned.support(&ConditionMask::ZERO), 200);
        assert_eq!(crate::support(&rows, &ConditionMask::ZERO), 200);

        // A position no row carries cannot satisfy the fixed-width hit test.
        let dead = ConditionMask::ZERO.with_bit(300);
        assert_eq!(owned.support(&dead), 0);
        assert_eq!(crate::support(&rows, &dead), 0);
    }

    /// The owned allocation is exactly one fixed-width mask per bar.
    ///
    /// This binds the 58.7 MB figure in [`Column::from_rows`] to the mask's compiler-
    /// checked 48-byte representation. It also refuses the old `BITS × stride`
    /// allocation shape from returning unnoticed.
    #[test]
    fn the_allocation_is_one_fixed_width_mask_per_bar() {
        let bars = 1_222_791_usize;
        let mask_bytes = core::mem::size_of::<ConditionMask>();
        assert_eq!(mask_bytes, 48, "the mask is six u64 words");
        assert_eq!(
            bars.saturating_mul(mask_bytes),
            58_693_968,
            "1,222,791 rows × 48 bytes is the one owned allocation"
        );

        let rows = vec![ConditionMask::ZERO; 65];
        let owned = Column::from_rows(&rows);
        assert_eq!(owned.rows.len(), rows.len());
        assert_eq!(core::mem::size_of_val(owned.rows.as_slice()), 65 * 48);
    }

    /// The shipping support body cannot quietly regain a candidate-position loop.
    ///
    /// Result tests cannot distinguish the forbidden vertical algorithm from the
    /// fixed-width one because the differential test above proves their answers are
    /// identical. This structural guard pins the distinction that matters: one call
    /// to the six-word `hits` operation inside one row fold, with no popcount or
    /// `set_positions` dependency in the body.
    #[test]
    fn the_live_support_body_is_one_fixed_width_hit_test() {
        let source = include_str!("column.rs");
        let body = source
            .split("pub fn support(&self, candidate: &ConditionMask) -> u64 {")
            .nth(1)
            .and_then(|tail| tail.split("pub fn support_fingerprinted").next())
            .unwrap_or("");
        assert_eq!(
            body.matches("row.hits(candidate)").count(),
            1,
            "the live support fold must perform exactly one fixed-width hit test per row"
        );
        assert!(
            !body.contains("set_positions(candidate)") && !body.contains("popcount()"),
            "candidate depth must not control live support work"
        );
    }

    #[test]
    fn an_empty_column_supports_nothing() {
        let owned = Column::from_rows(&[]);
        assert_eq!(owned.bars(), 0);
        assert_eq!(owned.support(&ConditionMask::ZERO), 0);
        assert_eq!(owned.support(&ConditionMask::ZERO.with_bit(5)), 0);
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
        let owned = Column::from_rows(&rows);
        assert_eq!(
            owned.support(&ConditionMask::ZERO.with_bit(0)),
            130,
            "with no live position to draw from, every bar falls back to bit 0"
        );
        assert_eq!(
            owned.support(&ConditionMask::ZERO.with_bit(1)),
            0,
            "and no other position was ever set"
        );
        assert_eq!(
            owned.support(&ConditionMask::ZERO.with_bit(0)),
            crate::support(&rows, &ConditionMask::ZERO.with_bit(0)),
            "the live column and reference count must agree on the degenerate input"
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
        let owned = Column::from_rows(&rows);
        let (counted, _) = owned.support_fingerprinted(&ConditionMask::ZERO.with_bit(7));
        assert_eq!(
            counted,
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
        let owned = Column::from_rows(&rows);

        let only_three = owned.support_fingerprinted(&ConditionMask::ZERO.with_bit(3));
        let only_nine = owned.support_fingerprinted(&ConditionMask::ZERO.with_bit(9));
        let both = owned.support_fingerprinted(&ConditionMask::ZERO.with_bit(3).with_bit(9));

        assert_eq!(only_three.0, 100, "every third bar of 300");
        assert_eq!(
            (only_three.1, only_nine.1),
            (only_nine.1, both.1),
            "three masks, one hit set, therefore one identity -- this is the equality \
             that collapses a 10,000-row report to its real findings"
        );

        // And a mask selecting a DIFFERENT set must not collide with them.
        let elsewhere = owned.support_fingerprinted(&ConditionMask::ZERO.with_bit(1));
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
                let owned = Column::from_rows(&rows);
                let mut candidates = vec![ConditionMask::ZERO];
                for a in LIVE {
                    candidates.push(ConditionMask::ZERO.with_bit(a));
                    for b in LIVE {
                        candidates.push(ConditionMask::ZERO.with_bit(a).with_bit(b));
                    }
                }
                for candidate in &candidates {
                    let (counted, _) = owned.support_fingerprinted(candidate);
                    assert_eq!(
                        counted,
                        owned.support(candidate),
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
        let first = Column::from_rows(&rows);
        let second = Column::from_rows(&rows);
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
        let a = Column::from_rows(&front).support_fingerprinted(&candidate);
        let b = Column::from_rows(&back).support_fingerprinted(&candidate);
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
        let owned = Column::from_rows(&rows);

        let (counted, identity) = owned.support_fingerprinted(&ConditionMask::ZERO);
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
        let dense = Column::from_rows(&all_bars);
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
                owned.support_fingerprinted(&candidate).1,
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
        let owned = Column::from_rows(&rows);
        let (counted, identity) =
            owned.support_fingerprinted(&ConditionMask::ZERO.with_bit(2).with_bit(4));
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
