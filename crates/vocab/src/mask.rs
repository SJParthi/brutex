//! A 384-bit condition mask, because the table no longer fits in a `u128`.
//!
//! `docs/03-vocabulary.md` shipped 74 conditions in a `u128` with 54 positions
//! of headroom. The table in [`crate::table`] defines 280 positions, so every
//! configuration of that headroom overflows. Four words replace one, and the
//! claim this module has to carry is that widening does **not** cost the
//! per-operation bound `CLAUDE.md` §3 rule 4 asks for.
//!
//! No allocation. No `Vec`. No loop anywhere on the hit path.

/// How many 64-bit words the mask carries.
///
/// **Widening is a one-line change here and nowhere else.** It was `[u64; 4]`
/// until the forming-day pivot family pushed the table past 256 positions; the
/// name carries no width so the next widening does not rename 77 references.
/// It costs one AND, one XOR and one OR per word in [`ConditionMask::hits`] —
/// a larger constant, still a constant, still no branch.
pub const WORDS: usize = 6;

/// A set of condition bits, [`ConditionMask::BITS`] wide.
///
/// Word 0 holds bits 0–63, word 1 holds 64–127, and so on, so bit *b* lives at
/// word `b >> 6`, offset `b & 63`. That layout keeps the shipped bits 0–127 in
/// words 0 and 1 exactly where the `u128` had them, which is what makes the
/// widening a format version rather than a renumbering (`CLAUDE.md` §3.8).
/// Proven by `vocab::mask::the_shipped_128_bits_keep_their_positions`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ConditionMask([u64; WORDS]);

// Forty-eight bytes, checked by the compiler rather than by a comment. A mask
// is copied per candidate in the sweep, so its width is a cost and not a
// detail. `tests/mask.rs` asserts the same thing at run time, because a
// `const` assertion that someone deletes leaves no failing test behind.
const _: () = assert!(core::mem::size_of::<ConditionMask>() == WORDS * 8);
const _: () = assert!(core::mem::align_of::<ConditionMask>() == 8);

impl ConditionMask {
    /// The empty set: no condition required, no condition held.
    pub const ZERO: Self = Self([0; WORDS]);

    /// How many condition positions fit. Not how many the table defines --
    /// that is [`crate::table::COUNT`], and it is smaller on purpose.
    pub const BITS: u32 = {
        // `as` would trip clippy::cast_possible_truncation and a runtime
        // `try_from` is not const. WORDS is a small literal, so the assertion
        // below is the whole of the safety argument and it is checked at compile
        // time.
        assert!(WORDS <= 64, "a mask wider than 4,096 bits needs a u64 BITS");
        #[allow(clippy::cast_possible_truncation)]
        let w = WORDS as u32;
        w * 64
    };

    /// Build from raw words, low word first.
    #[must_use]
    pub const fn from_words(w: [u64; WORDS]) -> Self {
        Self(w)
    }

    /// The raw words, low word first.
    #[must_use]
    pub const fn words(&self) -> [u64; WORDS] {
        self.0
    }

    /// This set with bit `b` added.
    ///
    /// A position at or above [`Self::BITS`] is ignored rather than panicking:
    /// a vocabulary index is validated once, where the table is read
    /// ([`crate::table::set_exact`]), and not again on every evaluation. The
    /// per-word match is what keeps the array index a constant, so the
    /// compiler proves the access in bounds instead of the program checking it.
    #[must_use]
    pub const fn with_bit(mut self, b: u32) -> Self {
        let bit = 1u64 << (b & 63);
        match b >> 6 {
            0 => self.0[0] |= bit,
            1 => self.0[1] |= bit,
            2 => self.0[2] |= bit,
            3 => self.0[3] |= bit,
            4 => self.0[4] |= bit,
            5 => self.0[5] |= bit,
            _ => {}
        }
        self
    }

    /// This set with bit `b` removed. Out-of-range positions are ignored, for
    /// the reason [`Self::with_bit`] gives.
    #[must_use]
    pub const fn without_bit(mut self, b: u32) -> Self {
        let bit = 1u64 << (b & 63);
        match b >> 6 {
            0 => self.0[0] &= !bit,
            1 => self.0[1] &= !bit,
            2 => self.0[2] &= !bit,
            3 => self.0[3] &= !bit,
            4 => self.0[4] &= !bit,
            5 => self.0[5] &= !bit,
            _ => {}
        }
        self
    }

    /// Is bit `b` set? A position at or above [`Self::BITS`] is never set.
    #[must_use]
    pub const fn get(&self, b: u32) -> bool {
        let bit = 1u64 << (b & 63);
        match b >> 6 {
            0 => self.0[0] & bit != 0,
            1 => self.0[1] & bit != 0,
            2 => self.0[2] & bit != 0,
            3 => self.0[3] & bit != 0,
            4 => self.0[4] & bit != 0,
            5 => self.0[5] & bit != 0,
            _ => false,
        }
    }

    /// **The hit test.** Does this bar's bit set satisfy every bit `candidate`
    /// requires -- that is, `(bar & candidate) == candidate`?
    ///
    /// Deliberately branchless, and that is the whole point of the function.
    /// An early-exit loop would return sooner on a candidate that fails in
    /// word 0, which makes the cost depend on the data; a constant-time
    /// guarantee that only holds on average is not the guarantee `CLAUDE.md`
    /// §3 rule 4 asks for. This is always [`WORDS`] ANDs, [`WORDS`] XORs,
    /// `WORDS - 1` ORs and one compare -- six, six and five today -- for every
    /// input, with no loop and no early return.
    ///
    /// The counts are stated against [`WORDS`] rather than as literals because
    /// they were literals, they said "four", and the mask had been six words
    /// wide since it outgrew 256 positions. A count copied from the code stops
    /// being true the moment the code changes.
    /// Proven by `vocab::mask::hits_does_the_same_work_for_every_input`, which
    /// counts the operators in this body against `WORDS`, and by
    /// `vocab::mask::adding_a_required_bit_can_only_remove_hits`.
    ///
    /// The relation is **anti-monotone**: adding a required bit can only
    /// remove hits. Every pruning guarantee in the sweep rests on that one
    /// property, so it is tested here rather than assumed in the engine.
    #[must_use]
    pub const fn hits(&self, candidate: &Self) -> bool {
        let d0 = (self.0[0] & candidate.0[0]) ^ candidate.0[0];
        let d1 = (self.0[1] & candidate.0[1]) ^ candidate.0[1];
        let d2 = (self.0[2] & candidate.0[2]) ^ candidate.0[2];
        let d3 = (self.0[3] & candidate.0[3]) ^ candidate.0[3];
        let d4 = (self.0[4] & candidate.0[4]) ^ candidate.0[4];
        let d5 = (self.0[5] & candidate.0[5]) ^ candidate.0[5];
        (d0 | d1 | d2 | d3 | d4 | d5) == 0
    }

    /// How many bits are set.
    #[must_use]
    pub const fn popcount(&self) -> u32 {
        self.0[0].count_ones()
            + self.0[1].count_ones()
            + self.0[2].count_ones()
            + self.0[3].count_ones()
            + self.0[4].count_ones()
            + self.0[5].count_ones()
    }

    /// Every bit set in either set.
    #[must_use]
    pub const fn union(&self, other: &Self) -> Self {
        Self([
            self.0[0] | other.0[0],
            self.0[1] | other.0[1],
            self.0[2] | other.0[2],
            self.0[3] | other.0[3],
            self.0[4] | other.0[4],
            self.0[5] | other.0[5],
        ])
    }

    /// Every bit set in both sets.
    #[must_use]
    pub const fn intersect(&self, other: &Self) -> Self {
        Self([
            self.0[0] & other.0[0],
            self.0[1] & other.0[1],
            self.0[2] & other.0[2],
            self.0[3] & other.0[3],
            self.0[4] & other.0[4],
            self.0[5] & other.0[5],
        ])
    }

    /// True when no bit is set.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        (self.0[0] | self.0[1] | self.0[2] | self.0[3] | self.0[4] | self.0[5]) == 0
    }
}

#[cfg(test)]
mod tests {

    /// THE MASK ALGEBRA IS EXERCISED IN EVERY WORD, NOT ONLY THE LOW ONES.
    ///
    /// `cargo-mutants` over this crate found **thirteen survivors here**, and
    /// every one was in a word the tests never reached:
    ///
    /// * `union` words 4 and 5 — `|` mutates to `^` and nothing notices
    /// * `intersect` word 5 — `&` mutates to `|` or `^` and nothing notices
    /// * `is_empty` — all five `|` in the fold, to `&` or `^`
    ///
    /// The cause is structural rather than careless. The table defines 280
    /// positions, so a mask built from real bits never sets anything above word
    /// 4, and word 5 (bits 320..383) is unallocated. But these three operations
    /// run over all six words whatever the vocabulary holds, so a defect in the
    /// part no position reaches is a defect no test reaches either — until the
    /// table grows into it, at which point the sweep is silently wrong.
    ///
    /// This drives every word deliberately, including the unallocated one.
    #[test]
    fn every_word_of_the_mask_algebra_is_exercised_including_the_unallocated_one() {
        // One bit in each of the six words, chosen so `b >> 6` walks 0..=5.
        let per_word: [u32; WORDS] = [1, 65, 129, 193, 257, 321];

        for (word, &bit) in per_word.iter().enumerate() {
            assert_eq!(
                bit >> 6,
                u32::try_from(word).unwrap_or(u32::MAX),
                "bit {bit} must live in word {word}"
            );

            // ── is_empty: one bit anywhere is not empty ──────────────────────
            // Kills `|`->`&` in the fold: an AND of six words is zero whenever
            // ANY word is zero, so a single-bit mask would read as empty.
            let one = ConditionMask::default().with_bit(bit);
            assert!(
                !one.is_empty(),
                "a mask with bit {bit} set (word {word}) is not empty"
            );

            // ── union: a SHARED bit must survive ─────────────────────────────
            // Kills `|`->`^`: exclusive-or clears a bit set on both sides, so
            // the union of a mask with itself would lose it.
            let joined = one.union(&one);
            assert!(
                joined.get(bit),
                "union must keep a bit both sides set, in word {word}"
            );
            assert_eq!(
                joined.popcount(),
                1,
                "and must invent none beside it, in word {word}"
            );

            // ── intersect: only the SHARED bit ───────────────────────────────
            // Kills `&`->`|` and `&`->`^`. `other` carries a DIFFERENT bit in
            // the same word, so an OR would keep two and an XOR would keep the
            // two unshared ones and drop the shared one.
            let other = ConditionMask::default().with_bit(bit).with_bit(bit ^ 2);
            let both = one.intersect(&other);
            assert!(
                both.get(bit),
                "intersect keeps the bit both sides set, in word {word}"
            );
            assert_eq!(
                both.popcount(),
                1,
                "intersect keeps ONLY the shared bit, in word {word}"
            );
        }

        // ── is_empty's fold: one case per operator, and the arithmetic matters ──
        //
        // `is_empty` is `(w0 | w1 | w2 | w3 | w4 | w5) == 0` — FIVE operators,
        // and a mutation replaces exactly one. A first version of this test set
        // one bit per word at the SAME position in each, and four mutants
        // survived it: with `w0 ^ w1` both being `1 << 1`, that pair cancels to
        // zero, but the four remaining `|` still OR in the other non-zero words,
        // so the answer stays right and the defect stays hidden.
        //
        // To kill the operator at position k, exactly the two words it joins
        // must be EQUAL and non-zero with every other word zero. Then `^` folds
        // the pair to zero, the rest contribute nothing, and `is_empty` wrongly
        // answers true.
        for k in 0..WORDS - 1 {
            let low = u32::try_from(k).unwrap_or(0) * 64 + 7;
            let pair = ConditionMask::default().with_bit(low).with_bit(low + 64);
            assert_eq!(pair.popcount(), 2, "two bits, in words {k} and {}", k + 1);
            assert!(
                !pair.is_empty(),
                "words {k} and {} both hold bit 7; a fold that cancels them \
                 reads this as empty, and it is not",
                k + 1
            );
        }

        // A bit in every word at once, which no single mutation should hide.
        let all = per_word
            .iter()
            .fold(ConditionMask::default(), |m, &b| m.with_bit(b));
        assert_eq!(
            all.popcount(),
            u32::try_from(WORDS).unwrap_or(u32::MAX),
            "one bit per word"
        );
        assert!(
            !all.is_empty(),
            "and a mask that full is certainly not empty"
        );

        assert!(
            ConditionMask::ZERO.is_empty(),
            "and the empty mask still reads empty, which is the other half"
        );
    }
    use super::*;

    #[test]
    fn a_mask_hits_itself() {
        let m = ConditionMask::ZERO.with_bit(3).with_bit(200);
        assert!(m.hits(&m));
    }

    #[test]
    fn the_empty_candidate_hits_everything() {
        assert!(ConditionMask::ZERO.hits(&ConditionMask::ZERO));
        assert!(ConditionMask::ZERO.with_bit(1).hits(&ConditionMask::ZERO));
    }

    #[test]
    fn a_missing_bit_in_any_word_is_a_miss() {
        // Both edges of all six words, and 7 -- the bit the bar holds -- so the
        // `held` branch below is a case this loop takes rather than an arm
        // nobody enters. The list stopped at 255 while the mask was six words
        // wide, which left words 4 and 5 out of the one test that walks a
        // required-and-missing bit through every word, and left the `b == 7`
        // branch dead: 7 was never in it.
        for b in [0u32, 7, 63, 64, 127, 128, 191, 192, 255, 256, 319, 320, 383] {
            let bar = ConditionMask::ZERO.with_bit(7);
            let want = ConditionMask::ZERO.with_bit(7).with_bit(b);
            if b == 7 {
                assert!(bar.hits(&want));
            } else {
                assert!(
                    !bar.hits(&want),
                    "bit {b} should have been required and missing"
                );
            }
        }
    }

    /// Anti-monotonicity: `bar.hits(wider)` implies `bar.hits(base)`.
    ///
    /// The property EVERY pruning guarantee rests on. §6's argument is that a
    /// k-combination cannot be frequent unless all of its (k-1)-subsets are, so if this
    /// fails the ladder is unsound and no amount of enumeration repairs it. `hits`'s own
    /// doc comment cites this test as the proof, and gate 12 accepts it.
    ///
    /// # It could not fail, and an audit demonstrated that
    ///
    /// The previous version asserted `bar.hits(&base)` INSIDE the loop, having already
    /// asserted the same thing unconditionally two lines above. The consequent did not
    /// depend on the loop variable, so the loop was incapable of failing whatever `hits`
    /// did. An adversarial audit replaced the all-bits-present fold with an
    /// any-bit-present one and this test stayed green.
    ///
    /// The repair is to assert the IMPLICATION rather than its consequent, and to state it
    /// over several bars and several bases. One bar with a single required bit cannot
    /// distinguish "requires every bit" from "requires any bit" -- with one bit the two
    /// agree, which is why a single-bit base was exactly the wrong fixture.
    #[test]
    fn adding_a_required_bit_can_only_remove_hits() {
        /// Bars spanning word 0, the 63->64 boundary, the last live word, and empty.
        const BARS: [&[u32]; 5] = [&[1, 2, 130], &[0, 63, 64], &[5], &[], &[63, 64, 127, 279]];
        /// Bases of several widths, including empty -- which every bar hits.
        const BASES: [&[u32]; 5] = [&[], &[1], &[1, 2], &[5], &[63, 64]];

        let build = |bits: &[u32]| bits.iter().fold(ConditionMask::ZERO, |m, b| m.with_bit(*b));
        for bar_bits in BARS {
            let bar = build(bar_bits);
            for base_bits in BASES {
                let base = build(base_bits);
                for added in 0..ConditionMask::BITS {
                    let wider = base.with_bit(added);
                    assert!(
                        !bar.hits(&wider) || bar.hits(&base),
                        "bar {bar_bits:?} hits base+{added} but not base {base_bits:?}. \
                         Adding a required bit made a MISS into a HIT, so support is not \
                         anti-monotone and every subset prune in the ladder is unsound."
                    );
                }
            }
        }
    }

    /// The branchless claim, tested as a claim about the SOURCE and not about
    /// a clock. A timing test on shared CI measures the runner, and gate 8 is
    /// where a ratio belongs; what can be proved here is that the function
    /// contains no loop and no early return, which is what makes the work the
    /// same for every input. The behavioural half -- that a candidate failing
    /// in word 0 and one failing in word 3 both produce a decision -- is
    /// `a_missing_bit_in_any_word_is_a_miss` above.
    #[test]
    fn hits_does_the_same_work_for_every_input() {
        // The fold count, named once. It was written `WORDS - 1` inside the
        // failure messages below, and an expression there is only ever
        // evaluated when the assertion fails -- a region `cargo llvm-cov`
        // counts and a passing run can never close. A `const` interpolates by
        // name and says the same thing.
        const FOLDS: usize = WORDS - 1;

        // The anchor is the WHOLE signature and it begins with a real newline,
        // which is the point. `split_once` takes the first match, and this file
        // contains the string being searched for twice -- once as the definition
        // and once here. In this literal the leading newline is the two
        // characters `\` and `n`, so the pattern matches the definition and
        // never this line. A prefix like `pub const fn hits(` would match here
        // too, and if `hits` were ever deleted the extraction would silently
        // read this test instead of refusing.
        const ANCHOR: &str = "\n    pub const fn hits(&self, candidate: &Self) -> bool {\n";

        let src = include_str!("mask.rs");
        let body = src
            .split_once(ANCHOR)
            .and_then(|(_, rest)| rest.split_once("\n    }"))
            .map(|(body, _)| body);
        assert!(
            body.is_some(),
            "`hits` no longer has the signature `{ANCHOR}` closing at one indent, \
             so this test read nothing and proves nothing"
        );
        // Not `let Some(body) = .. else { unreachable!(..) }`: that `else` is a
        // panic inside this crate that a correct build can never enter, so it
        // is a coverage region no test can close. The assertion above is the
        // check, and an empty body fails every count below rather than passing
        // quietly -- `matches(..).count()` on "" is 0 and WORDS is 6.
        let extracted = body.unwrap_or_default();

        // Comments are stripped before anything is inspected, and this line is
        // load-bearing. Every check below reads TEXT, and text inside a comment
        // is text. An adversarial audit beat the previous version of this test
        // by moving the word-5 term into a comment and folding `d4` twice: the
        // operator counts were satisfied by the comment while `hits` ignored
        // bits 320..383 outright, so any candidate requiring one of them would
        // have hit every bar. `hits` contains no string literal, so splitting
        // each line at the first `//` cannot cut a comment marker out of one.
        let body: String = extracted
            .lines()
            .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
            .collect::<Vec<_>>()
            .join("\n");

        // `&&` and `||` are on this list because the same audit beat the
        // previous version a second way, with
        // `(d0 | d1 | d2 | d3 | d4) == 0 && (d5 | d5) == 0` -- every count
        // satisfied, every banned word absent, and the second comparison
        // evaluated only when the first says maybe. Short-circuiting IS a
        // branch; it just is not spelled `if`.
        for banned in [
            "for ", "while ", "loop ", "return", "if ", "match ", "&&", "||",
        ] {
            assert!(
                !body.contains(banned),
                "`hits` contains `{banned}`, so its cost now depends on the data. \
                 It must be {WORDS} ANDs, {WORDS} XORs, {FOLDS} ORs and one compare."
            );
        }

        // The function, written out from `WORDS` and compared line for line.
        //
        // This replaced three `matches(..).count()` assertions, and the reason is
        // the project's own rule that removing the second copy beats testing that
        // two copies agree. A count is a PROXY for the shape: it says six ANDs
        // appear somewhere, which a comment can satisfy and a doubled term can
        // satisfy. These say what each line IS. Both attacks that beat the counts
        // die here without either being enumerated, and so does a word added to
        // `WORDS` and not to the body -- the loop simply demands a line that is
        // not there.
        for w in 0..WORDS {
            let line = format!("let d{w} = (self.0[{w}] & candidate.0[{w}]) ^ candidate.0[{w}];");
            assert!(
                body.lines().any(|l| l.trim() == line),
                "`hits` must contain exactly `{line}`, so that word {w} is read, \
                 masked and compared like every other: {body}"
            );
        }

        // The decision is ONE expression naming every difference. The ban list
        // above is a list, and a list is only as complete as whoever wrote it;
        // this is the complement, and it does not need to anticipate the trick.
        let terms = (0..WORDS).map(|w| format!("d{w}")).collect::<Vec<_>>();
        let fold = format!("({}) == 0", terms.join(" | "));
        let last = body
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or_default()
            .trim();
        assert_eq!(
            last, fold,
            "`hits` must decide with exactly `{fold}`: {body}"
        );

        // And nothing else is in there. The two checks above say what must be
        // present; this says that is ALL that is present, which is what stops a
        // branch being inserted between two of the differences.
        let lines = body.lines().filter(|l| !l.trim().is_empty()).count();
        assert_eq!(
            lines,
            WORDS + 1,
            "`hits` is {lines} lines of code and must be exactly {}: {WORDS} \
             differences and one fold, nothing between them: {body}",
            WORDS + 1
        );
    }

    #[test]
    fn bits_land_in_the_word_the_layout_promises() {
        assert_eq!(ConditionMask::ZERO.with_bit(0).words(), [1, 0, 0, 0, 0, 0]);
        assert_eq!(ConditionMask::ZERO.with_bit(64).words(), [0, 1, 0, 0, 0, 0]);
        assert_eq!(
            ConditionMask::ZERO.with_bit(128).words(),
            [0, 0, 1, 0, 0, 0]
        );
        assert_eq!(
            ConditionMask::ZERO.with_bit(192).words(),
            [0, 0, 0, 1, 0, 0]
        );
    }

    /// Every position lands in the word the layout promises, is visible at its
    /// own index, and is visible at no other.
    ///
    /// `with_bit` and `get` each pick a word with a six-arm match and the offset
    /// with `b & 63`, so each has the same two mistakes available: touching the
    /// wrong word, which answers for a position 64 away, and aliasing, where
    /// bit 64 reports the bit set at 0. `bits_land_in_the_word_the_layout_promises`
    /// above names four positions and `get` was only ever asked about three of
    /// its six words -- words 2, 4 and 5 were never read by any unit test, so an
    /// arm returning the wrong word's bit would have passed. This walks all
    /// [`ConditionMask::BITS`] of them.
    #[test]
    fn every_position_lands_in_its_own_word_and_is_visible_nowhere_else() {
        for b in 0..ConditionMask::BITS {
            let word_of_b = b >> 6;
            let offset = b & 63;
            let m = ConditionMask::ZERO.with_bit(b);
            assert!(m.get(b), "bit {b} was set and `get` denies it");
            assert_eq!(m.popcount(), 1, "setting bit {b} set a second bit as well");
            for (index, word) in m.words().into_iter().enumerate() {
                let holds_it = u32::try_from(index).is_ok_and(|w| w == word_of_b);
                let want = if holds_it { 1u64 << offset } else { 0 };
                assert_eq!(
                    word, want,
                    "bit {b} belongs in word {word_of_b} at offset {offset}, and \
                     word {index} reads {word:#018x}"
                );
            }
            for other in 0..ConditionMask::BITS {
                assert_eq!(
                    m.get(other),
                    other == b,
                    "only bit {b} is set, and `get` disagrees about bit {other}"
                );
            }
        }
    }

    /// Clearing reaches every word too, and clears exactly one position.
    ///
    /// Starting from all ones is what makes the negative half cheap: after one
    /// removal exactly one position is gone, whichever word it was in, so a
    /// `&= !bit` against the wrong word shows up as a `popcount` that is right
    /// and a `get` that is wrong -- or the other way round.
    /// `without_bit_removes_only_that_bit` above names eight positions; the mask
    /// has [`ConditionMask::BITS`].
    #[test]
    fn without_bit_clears_one_position_in_every_word() {
        let full = ConditionMask::from_words([u64::MAX; WORDS]);
        for b in 0..ConditionMask::BITS {
            let less = full.without_bit(b);
            assert!(!less.get(b), "bit {b} survived removal from a full mask");
            assert_eq!(
                less.popcount(),
                ConditionMask::BITS - 1,
                "removing bit {b} took another position with it"
            );
            assert_eq!(
                less.union(&ConditionMask::ZERO.with_bit(b)),
                full,
                "putting bit {b} back has to restore the full mask exactly"
            );
            assert!(
                !less.hits(&ConditionMask::ZERO.with_bit(b)),
                "a candidate requiring bit {b} must miss a bar that lost it"
            );
        }
    }

    #[test]
    fn out_of_range_bits_are_ignored_not_panicked() {
        // BITS-relative, not a literal: 256 was out of range at four words and
        // is an ordinary position at six. A test whose premise is a width has
        // to be written against the width.
        let past = ConditionMask::BITS;
        let m = ConditionMask::ZERO.with_bit(past).with_bit(u32::MAX);
        assert!(m.is_empty());
        assert!(!m.get(past));
        assert!(!m.get(u32::MAX));
        // And removing one is the same non-event, rather than clearing the
        // bit that `b & 63` would have aliased to.
        let full = ConditionMask::from_words([u64::MAX; WORDS]);
        assert_eq!(full.without_bit(past), full);
        assert_eq!(full.without_bit(u32::MAX), full);
    }

    #[test]
    fn without_bit_removes_only_that_bit() {
        let m = ConditionMask::ZERO.with_bit(5).with_bit(70).with_bit(200);
        let less = m.without_bit(70);
        assert!(less.get(5) && !less.get(70) && less.get(200));
        // Every word can be cleared, not just the one a happy test picks.
        for b in [0u32, 63, 64, 127, 128, 191, 192, 255] {
            let one = ConditionMask::ZERO.with_bit(b);
            assert!(one.without_bit(b).is_empty(), "bit {b} survived removal");
        }
    }

    #[test]
    fn popcount_counts_every_word() {
        let m = ConditionMask::ZERO
            .with_bit(0)
            .with_bit(64)
            .with_bit(128)
            .with_bit(192);
        assert_eq!(m.popcount(), 4);
    }

    #[test]
    fn union_and_intersect_reach_every_word() {
        let a = ConditionMask::ZERO.with_bit(0).with_bit(64);
        let b = ConditionMask::ZERO.with_bit(64).with_bit(128).with_bit(192);
        assert_eq!(a.union(&b).popcount(), 4);
        assert_eq!(a.intersect(&b), ConditionMask::ZERO.with_bit(64));
        assert!(ConditionMask::ZERO.is_empty());
        assert!(!a.is_empty());
    }

    #[test]
    fn from_words_and_words_round_trip() {
        let w = [1u64, 2, 3, 4, 0, 0];
        assert_eq!(ConditionMask::from_words(w).words(), w);
        assert_eq!(ConditionMask::default(), ConditionMask::ZERO);
    }

    #[test]
    fn the_shipped_128_bits_keep_their_positions() {
        // Widening must not renumber anything, or every stored result changes
        // meaning silently (`CLAUDE.md` §3.8).
        for b in 0..128u32 {
            let m = ConditionMask::ZERO.with_bit(b);
            let w = m.words();
            let as_u128 = u128::from(w[0]) | (u128::from(w[1]) << 64);
            assert_eq!(as_u128, 1u128 << b, "bit {b} moved when the mask widened");
            assert_eq!(w[2] | w[3], 0);
        }
    }
}
