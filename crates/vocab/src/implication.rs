//! Which condition positions say something another already said.
//!
//! # The row that made this necessary, and it is measured
//!
//! A live sweep's leading combination was, decoded from its mask:
//!
//! ```text
//! {15, 65, 82, 84, 182, 186, 185}
//!   close_above_pdl            (15)
//!   close_below_supertrend     (65)
//!   close_above_pivot_s2_band  (82)
//!   close_above_pivot_s3_band  (84)   <- implied by 82
//!   close_above_pivot_s4_band  (182)  <- implied by 82
//!   close_above_pivot_s5_band  (186)  <- implied by 82
//!   close_below_pivot_r5_band  (185)
//! ```
//!
//! Seven conditions carrying **four** facts. It fired on 200,813 bars of
//! 609,722 and won 52% — a coin flip on a near-tautology, sitting at the head
//! of the ranked table because nothing in the sweep knew that three of its bits
//! were restatements of a fourth.
//!
//! # Why Apriori cannot do this, and why `closed` cannot either
//!
//! Apriori prunes on **support anti-monotonicity**: a candidate dies only when
//! one of its (k−1) subsets is INFREQUENT. `above_s2` and `above_s3` are both
//! frequent — spectacularly so — and their union is frequent too, so the prune
//! has no reason to fire and is right not to. It is a statement about counts and
//! this is a statement about meaning.
//!
//! [`crate`]'s sibling `runner::closed` removes a set only when a superset ONE
//! BIT LARGER has IDENTICAL support. It therefore deletes `{above_s2}` and
//! KEEPS `{above_s2, above_s3, above_s4, above_s5}` — the maximal form, which is
//! lossless and is exactly why the reported row reads long.
//!
//! # The chain, and it is exact rather than approximate
//!
//! `crates/indicators/src/daily.rs:206-215` computes the ten pivot levels in
//! `i128` from `P = ⌊(H+L+C)/3⌋` and `rr = H − L`. Every adjacent difference
//! telescopes to `H − P ≥ 0`, `P − L ≥ 0` or `rr ≥ 0`, and `daily.rs:181-189`
//! refuses `high < low` and a close outside `[L, H]`. So on EVERY session:
//!
//! ```text
//! s5 ≤ s4 ≤ s3 ≤ s2 ≤ s1 ≤ P ≤ r1 ≤ r2 ≤ r3 ≤ r4 ≤ r5
//! ```
//!
//! **And the band half-width is read ONCE, outside the loop** —
//! `daily.rs:579` `let half = levels.band_half();` — so all ten levels share one
//! `half ≥ 0`, and the test at `daily.rs:604` is `close > level + half`. A
//! shared offset preserves the order, so `close > s2 + half` implies
//! `close > s3 + half` for every rung below. That is why this is an exact
//! implication and not a usually-true one: had `half` been per-level, two bands
//! could overlap and the chain would break.
//!
//! # What is deliberately NOT here
//!
//! The opening-range windows look ordered — a 60-minute range contains a
//! 5-minute one — and they are **not** admitted. A window still forming emits
//! nothing (`crates/indicators/src/orb.rs:277-281`) while the shorter one
//! already answers, so between minute 5 and minute 15 the implication is simply
//! false. Nor are the Fibonacci ladders, whose rungs are mutually exclusive
//! rather than implied, nor `near_pivot_*`, whose bands can both fire at a
//! boundary where `r2 − r1` equals `2·half`. An approximate implication pruned
//! as exact would discard a real distinction, which is worse than the redundancy
//! it removes.
//!
//! # Cost
//!
//! [`implies`] is a `match` over two `u16` and at most one range comparison —
//! straight-line, no loop, no allocation, and independent of the vocabulary's
//! size. `CLAUDE.md` §3 rule 4's bound is unaffected: this is not one of the
//! five operations it names, and it adds a constant to the join's per-candidate
//! work rather than a scan. **UNVERIFIED as a measured bound**; read off the
//! source per §3 rule 6.

/// The pivot chain, low to high, as `(above_bit, below_bit)` per rung.
///
/// Written out rather than derived from names: a name is not evidence of a
/// position, and the ORDER is the whole content of this table. The rung for the
/// pivot itself is `above_cpr_tc` / `below_cpr_bc`, which
/// `crates/indicators/src/daily.rs:491-493` proves is the same predicate shape
/// at `P` — `cpr_span` returns `(P − half, P + half)` because `tc = 2P − bc`.
///
/// Index in this array IS the rung's rank. Nothing reads the names.
///
/// **Test-only, and that is the honest shape.** [`pivot_rung`] is the production
/// path — a `const fn` cannot iterate and the workspace denies indexing, so the
/// chain is written there as a match. This array exists to hold that match
/// accountable: `the_written_out_match_equals_the_chain_it_stands_in_for` walks
/// both and refuses any drift. Shipping it unread would be a second definition
/// of one fact.
#[cfg(test)]
const PIVOT_CHAIN: [(u16, u16); 11] = [
    (186, 187), // s5, the lowest
    (182, 183), // s4
    (84, 85),   // s3
    (82, 83),   // s2
    (80, 81),   // s1
    (60, 61),   // P, as above_cpr_tc / below_cpr_bc
    (74, 75),   // r1
    (76, 77),   // r2
    (78, 79),   // r3
    (180, 181), // r4
    (184, 185), // r5, the highest
];

/// Where `position` sits on the pivot chain, and on which side.
///
/// `None` for every position that is not on the chain, which is most of them.
const fn pivot_rung(position: u16) -> Option<(usize, bool)> {
    // A `const fn` cannot iterate, and the workspace denies indexing, so this is
    // written as a match over the twenty-two positions. The const assertion
    // below holds it equal to `PIVOT_CHAIN` so the two cannot drift.
    match position {
        186 => Some((0, true)),
        187 => Some((0, false)),
        182 => Some((1, true)),
        183 => Some((1, false)),
        84 => Some((2, true)),
        85 => Some((2, false)),
        82 => Some((3, true)),
        83 => Some((3, false)),
        80 => Some((4, true)),
        81 => Some((4, false)),
        60 => Some((5, true)),
        61 => Some((5, false)),
        74 => Some((6, true)),
        75 => Some((6, false)),
        76 => Some((7, true)),
        77 => Some((7, false)),
        78 => Some((8, true)),
        79 => Some((8, false)),
        180 => Some((9, true)),
        181 => Some((9, false)),
        184 => Some((10, true)),
        185 => Some((10, false)),
        _ => None,
    }
}

/// Does `a` being true guarantee `b` is true?
///
/// Exact, never approximate. A `true` here licenses the engine to refuse a
/// candidate containing both, so anything short of a proof belongs outside this
/// function.
///
/// `implies(x, x)` is false: a position does not restate itself, and a
/// candidate can never hold one position twice.
#[must_use]
pub const fn implies(a: u16, b: u16) -> bool {
    if a == b {
        return false;
    }
    match (pivot_rung(a), pivot_rung(b)) {
        // Same side of the chain: `above` at a higher rung implies `above` at
        // every lower one; `below` at a lower rung implies `below` at every
        // higher one.
        (Some((rank_a, true)), Some((rank_b, true))) => rank_a > rank_b,
        (Some((rank_a, false)), Some((rank_b, false))) => rank_a < rank_b,
        // Opposite sides are a BAND, and a band is information: `above s2` with
        // `below s1` says price sits between them. Never implied, never pruned.
        _ => false,
    }
}

/// Can `a` and `b` both be true on one bar?
///
/// `false` means the pair is impossible, so a candidate holding both fires on
/// no bar at all. Those already die on the support test; refusing them earlier
/// only saves counting a support that is known to be zero.
#[must_use]
pub const fn compatible(a: u16, b: u16) -> bool {
    if a == b {
        return true;
    }
    match (pivot_rung(a), pivot_rung(b)) {
        // `above X` and `below Y` with X at or above Y is impossible: the close
        // would have to exceed `level(X) + half` and fall short of
        // `level(Y) − half` at once, and `level(X) ≥ level(Y)` with `half ≥ 0`.
        (Some((rank_a, true)), Some((rank_b, false))) => rank_a < rank_b,
        (Some((rank_a, false)), Some((rank_b, true))) => rank_b < rank_a,
        _ => true,
    }
}

/// Is this pair worth enumerating at all?
///
/// One call, both questions. A pair is dead when either position restates the
/// other or the two can never hold together.
#[must_use]
pub const fn pair_is_informative(a: u16, b: u16) -> bool {
    !implies(a, b) && !implies(b, a) && compatible(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The match and the array must name the same twenty-two positions.
    #[test]
    fn the_written_out_match_equals_the_chain_it_stands_in_for() {
        for (rank, (above, below)) in PIVOT_CHAIN.iter().enumerate() {
            assert_eq!(
                pivot_rung(*above),
                Some((rank, true)),
                "position {above} must be rung {rank} above"
            );
            assert_eq!(
                pivot_rung(*below),
                Some((rank, false)),
                "position {below} must be rung {rank} below"
            );
        }
        // And nothing else is on the chain.
        let mut on_chain = 0;
        for position in 0..370_u16 {
            if pivot_rung(position).is_some() {
                on_chain += 1;
            }
        }
        assert_eq!(on_chain, 22, "eleven rungs, two sides each");
    }

    /// The measured row: three of its seven bits restate a fourth.
    #[test]
    fn the_measured_leading_combination_carries_four_facts_not_seven() {
        // close_above_pivot_s2_band implies s3, s4 and s5.
        for lower in [84_u16, 182, 186] {
            assert!(
                implies(82, lower),
                "above_s2 (82) must imply above at the lower rung {lower}"
            );
            assert!(
                !implies(lower, 82),
                "and the implication must not run backwards"
            );
        }
        // The other three bits of that row are not on the chain at all, so
        // nothing about them is inferred.
        for free in [15_u16, 65, 185] {
            assert!(!implies(82, free));
            assert!(!implies(free, 82));
        }
        // `below_r5` (185) is on the chain but on the other side, and `above_s2`
        // with `below_r5` is a BAND -- both survive.
        assert!(pair_is_informative(82, 185), "a band is information");
    }

    /// A band is never pruned, in either direction, at every rung pair.
    #[test]
    fn opposite_sides_are_a_band_and_survive() {
        for (rank_a, (above, _)) in PIVOT_CHAIN.iter().enumerate() {
            for (rank_b, (_, below)) in PIVOT_CHAIN.iter().enumerate() {
                assert!(!implies(*above, *below));
                assert!(!implies(*below, *above));
                // Compatible only when the `above` rung is strictly below the
                // `below` rung -- that is what makes it a band rather than a
                // contradiction.
                assert_eq!(
                    compatible(*above, *below),
                    rank_a < rank_b,
                    "above rung {rank_a} with below rung {rank_b}"
                );
            }
        }
    }

    /// Same-side ordering: exactly one direction implies, never both.
    #[test]
    fn one_direction_implies_and_the_other_never_does() {
        for (rank_high, (above_high, below_high)) in PIVOT_CHAIN.iter().enumerate() {
            for (rank_low, (above_low, below_low)) in PIVOT_CHAIN.iter().enumerate() {
                if rank_high == rank_low {
                    continue;
                }
                assert_eq!(implies(*above_high, *above_low), rank_high > rank_low);
                assert_eq!(implies(*below_high, *below_low), rank_high < rank_low);
                // Never symmetric -- that would make every pair dead.
                assert!(!(implies(*above_high, *above_low) && implies(*above_low, *above_high)));
            }
        }
    }

    /// Nothing off the chain is touched, and that is most of the vocabulary.
    #[test]
    fn positions_off_the_chain_are_never_pruned() {
        for a in 0..370_u16 {
            for b in 0..370_u16 {
                if pivot_rung(a).is_some() && pivot_rung(b).is_some() {
                    continue;
                }
                assert!(!implies(a, b), "{a} must not imply {b}");
                assert!(compatible(a, b), "{a} and {b} must stay compatible");
            }
        }
    }

    /// A position never restates itself, and is always compatible with itself.
    ///
    /// # Why `pair_is_informative(x, x)` is TRUE and that is right
    ///
    /// This test first asserted the opposite, on the reasoning that a position
    /// beside itself carries nothing. It does not follow: `implies` is false in
    /// both directions and `compatible` is true, so the pair is "informative" by
    /// construction — and the engine never asks. The prefix join draws `b` from
    /// `block.iter().skip(offset + 1)`, so `a` and `b` are distinct entries and
    /// a self-pair is unreachable. Asserting `false` here would have pinned a
    /// value no caller can observe, which is a test asserting nothing about the
    /// program.
    #[test]
    fn a_position_is_neither_implied_by_nor_incompatible_with_itself() {
        for position in 0..370_u16 {
            assert!(!implies(position, position), "{position} restates itself");
            assert!(compatible(position, position));
        }
    }

    /// The opening range is deliberately absent: a forming window emits nothing
    /// while a shorter one already answers, so the implication is false between
    /// minute 5 and minute 15.
    #[test]
    fn the_opening_range_is_not_on_the_chain_and_must_not_be() {
        for position in 86..=105_u16 {
            assert!(
                pivot_rung(position).is_none(),
                "ORB position {position} must not be treated as ordered"
            );
        }
    }
}
