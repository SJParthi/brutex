//! The mask, checked against a naive per-bit reference implementation.
//!
//! The unit tests beside [`vocab::ConditionMask`] check the cases a human thought
//! of. These check the four word operations against an implementation written
//! the slow, obvious way -- one `bool` per position and a loop -- over inputs nobody
//! chose by hand. A word-level bug that a hand-written case misses is a bug
//! the reference disagrees with.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use vocab::ConditionMask;

/// `WORDS * 8` bytes, refused at COMPILE time and not at run time. The crate
/// asserts this too; it is repeated here because a `const` assertion deleted
/// with the line it guards leaves nothing failing behind. Written against
/// [`vocab::mask::WORDS`] rather than a literal, so a widening does not need a
/// second edit here.
const _: () = assert!(std::mem::size_of::<ConditionMask>() == vocab::mask::WORDS * 8);

/// Sized from [`vocab::mask::WORDS`] rather than a literal, so a widening does
/// not silently leave the reference narrower than the thing it checks. It was
/// `[bool; 256]` and the mask grew past it — the reference stopped covering the
/// range it was there to cover, which is the one failure a reference must not
/// have.
const N: usize = vocab::mask::WORDS * 64;

/// The naive mask: one `bool` per position, no packing, no cleverness.
#[derive(Clone, Copy)]
struct Naive([bool; N]);

impl Naive {
    fn of(mask: ConditionMask) -> Self {
        let mut bits = [false; N];
        for (b, slot) in bits.iter_mut().enumerate() {
            let index = u32::try_from(b).expect("a position fits in a u32");
            *slot = mask.get(index);
        }
        Self(bits)
    }

    fn popcount(self) -> u32 {
        let mut n = 0;
        for held in self.0 {
            if held {
                n += 1;
            }
        }
        n
    }

    fn union(self, other: Self) -> Self {
        let mut out = [false; N];
        for (b, slot) in out.iter_mut().enumerate() {
            *slot = self.0[b] || other.0[b];
        }
        Self(out)
    }

    fn intersect(self, other: Self) -> Self {
        let mut out = [false; N];
        for (b, slot) in out.iter_mut().enumerate() {
            *slot = self.0[b] && other.0[b];
        }
        Self(out)
    }

    /// The superset relation, spelled out one position at a time.
    fn hits(self, candidate: Self) -> bool {
        let mut ok = true;
        for b in 0..N {
            if candidate.0[b] && !self.0[b] {
                ok = false;
            }
        }
        ok
    }

    fn to_mask(self) -> ConditionMask {
        let mut m = ConditionMask::ZERO;
        for (b, held) in self.0.iter().enumerate() {
            if *held {
                m = m.with_bit(u32::try_from(b).expect("a position fits in a u32"));
            }
        }
        m
    }
}

/// A deterministic integer generator. No floating point, no dependency, and
/// the same 400 masks on every machine and every rerun -- `CLAUDE.md` §3
/// rule 5 applies to a test as much as to a sweep.
struct Lcg(u64);

impl Lcg {
    fn next_u64(&mut self) -> u64 {
        // Numerical Recipes' 64-bit constants; wrapping is the definition.
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn next_mask(&mut self) -> ConditionMask {
        ConditionMask::from_words([
            self.next_u64(),
            self.next_u64(),
            self.next_u64(),
            self.next_u64(),
            0,
            0,
        ])
    }

    /// A sparse mask, because the interesting hit tests are the ones that
    /// nearly pass. Four uniform words almost never hit anything.
    fn next_sparse(&mut self) -> ConditionMask {
        let mut m = ConditionMask::ZERO;
        for _ in 0..6 {
            m = m.with_bit(u32::try_from(self.next_u64() % (N as u64)).expect("under the width"));
        }
        m
    }
}

#[test]
fn popcount_union_and_intersect_agree_with_the_naive_reference() {
    let mut rng = Lcg(0x5EED);
    for _ in 0..200 {
        let a = rng.next_mask();
        let b = rng.next_mask();
        let na = Naive::of(a);
        let nb = Naive::of(b);

        assert_eq!(a.popcount(), na.popcount(), "popcount disagrees");
        assert_eq!(
            a.union(&b),
            na.union(nb).to_mask(),
            "union disagrees for {a:?} and {b:?}",
        );
        assert_eq!(
            a.intersect(&b),
            na.intersect(nb).to_mask(),
            "intersect disagrees for {a:?} and {b:?}",
        );
        assert_eq!(a, na.to_mask(), "the round trip through bits lost a bit");
    }
}

#[test]
fn the_hit_test_agrees_with_the_naive_reference() {
    let mut rng = Lcg(0x00C0_FFEE);
    let mut hits = 0;
    for _ in 0..400 {
        let bar = rng.next_mask();
        let candidate = rng.next_sparse();
        let want = Naive::of(bar).hits(Naive::of(candidate));
        assert_eq!(
            bar.hits(&candidate),
            want,
            "hit test disagrees: bar {bar:?}, candidate {candidate:?}",
        );
        if want {
            hits += 1;
        }
    }
    assert!(
        hits > 0,
        "400 draws and not one hit: the generator is producing candidates no \
         bar can satisfy, so this test is comparing two `false`s",
    );
}

/// **Anti-monotonicity, across every position.** Adding a required bit can
/// only remove hits -- never add one. Every pruning guarantee in the sweep
/// rests on this and on nothing else.
#[test]
fn adding_a_required_bit_can_only_remove_hits_at_every_position() {
    let mut rng = Lcg(0xA117);
    for _ in 0..40 {
        let bar = rng.next_mask();
        let base = rng.next_sparse();
        let before = bar.hits(&base);
        for b in 0..ConditionMask::BITS {
            let wider = base.with_bit(b);
            let after = bar.hits(&wider);
            assert!(
                before || !after,
                "adding bit {b} to a candidate CREATED a hit, which breaks the \
                 subset-prune the sweep depends on",
            );
            if after {
                assert!(before);
            }
        }
    }
}

/// **Every index below the width is addressable, and the width itself is not.** The
/// second half is the one that matters: a position past the end must not
/// alias onto a real one via `b & 63`.
#[test]
fn every_word_addresses_its_range_and_the_mask_refuses_past_bits() {
    for b in 0..ConditionMask::BITS {
        let m = ConditionMask::ZERO.with_bit(b);
        assert_eq!(m.popcount(), 1, "bit {b} set more or less than one bit");
        assert!(m.get(b), "bit {b} did not read back");
        assert_eq!(
            m.words().iter().position(|w| *w != 0),
            Some(usize::try_from(b / 64).expect("a word index")),
            "bit {b} landed in the wrong word",
        );
    }
    for b in [
        ConditionMask::BITS,
        ConditionMask::BITS + 1,
        ConditionMask::BITS + 64,
        1_000_000,
        u32::MAX - 1,
        u32::MAX,
    ] {
        let m = ConditionMask::ZERO.with_bit(b);
        assert!(m.is_empty(), "position {b} is past the mask and was set");
        assert!(!m.get(b));
        assert_eq!(m.popcount(), 0);
    }
}

/// **The shipped 128 bits are where the `u128` left them.** This is the whole
/// argument that widening is a format version and not a renumbering: read the
/// low two words back as a `u128` and every shipped position is untouched.
#[test]
fn the_shipped_128_positions_stay_in_words_0_and_1() {
    for b in 0..128u32 {
        let w = ConditionMask::ZERO.with_bit(b).words();
        let as_u128 = u128::from(w[0]) | (u128::from(w[1]) << 64);
        assert_eq!(
            as_u128,
            1u128 << b,
            "position {b} moved when the mask widened"
        );
        assert_eq!(w[2] | w[3], 0, "position {b} leaked into the new words");
    }
    // And the two new words are genuinely new: nothing below 128 reaches them.
    let shipped = (0..128u32).fold(ConditionMask::ZERO, ConditionMask::with_bit);
    assert_eq!(shipped.words()[2], 0);
    assert_eq!(shipped.words()[3], 0);
    assert_eq!(shipped.popcount(), 128);
}

#[test]
fn the_mask_is_words_times_eight_bytes_at_run_time_too() {
    assert_eq!(std::mem::size_of::<ConditionMask>(), vocab::mask::WORDS * 8);
    assert_eq!(std::mem::align_of::<ConditionMask>(), 8);
}
