//! SIZE-FIXED-20-RANKED's order: a strict total order over one batch, applied by a fixed
//! network rather than a sort (D-8634).
//!
//! The keys, in order: (1) the learned edge, higher first, a score with no trade behind it after
//! every score with one; (2) the stop in R, smaller first; (3) the liquidity, larger first; (4)
//! the seeded key of tie-break (e), smaller first; (5) the stock position, smaller first; (6)
//! the recipe, smaller first. Distinct (stock, recipe) make it total. The edge only orders: a
//! negative edge is still offered.
//!
//! The network is Batcher's bitonic network over the next power of two at or above the batch
//! (at most 256), padded with entries that compare after every candidate: the same
//! compare-exchanges for every batch of that width, at most `128 x 8 x 9 / 2 = 4,608` at 256.
//! It is bounded by the universe and not O(1); `docs/06-limits.md` records it and
//! `crates/runner/benches/ratio.rs` row C-FNO-SIZE-03 measures it.

use core::cmp::Ordering;

use super::edge::MeanKey;
use super::{Candidate, EdgeTable, Fraction, StockId, UNIVERSE, put, read};

/// Tie-break (e)'s seeded key: FNO-PORTFOLIO's formula restated byte for byte, `SplitMix64`'s
/// step and finaliser over the seed, then the start, the stock and the recipe, each folded in
/// by exclusive or and followed by the mix. One seed draws one permutation in every policy.
#[must_use]
pub fn seeded(seed: u64, start: i64, stock: StockId, recipe: u32) -> u64 {
    let mut state = mix(seed);
    state = mix(state ^ start.cast_unsigned());
    state = mix(state ^ u64::from(stock.0));
    mix(state ^ u64::from(recipe))
}

/// `SplitMix64`'s step and finaliser.
#[must_use]
pub const fn mix(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A candidate's keys after the edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Rest {
    /// The stop's distance in R.
    pub(super) stop: Fraction,
    /// Session-to-date traded value.
    pub(super) liquidity: u128,
    /// Tie-break (e)'s key.
    pub(super) seeded: u64,
    /// The stock's position.
    pub(super) stock: u8,
    /// The recipe.
    pub(super) recipe: u32,
}

impl Rest {
    /// The keys of a padding entry; never compared, because a pad loses before its keys are read.
    const PAD: Self = Self {
        stop: Fraction { num: 0, den: 1 },
        liquidity: 0,
        seeded: 0,
        stock: 0,
        recipe: 0,
    };

    /// `candidate`'s keys under `seed`.
    fn of(candidate: &Candidate, seed: u64) -> Self {
        Self {
            stop: candidate.stop_in_r,
            liquidity: candidate.liquidity,
            seeded: seeded(seed, candidate.start, candidate.stock, candidate.recipe),
            stock: candidate.stock.0,
            recipe: candidate.recipe,
        }
    }
}

/// The strict total order of two candidates' keys: `Less` when `a` is offered first. Counts in
/// `reached` a comparison that falls through to the seeded key.
pub(super) fn compare(
    (a_mean, a): (&MeanKey, &Rest),
    (b_mean, b): (&MeanKey, &Rest),
    reached: &mut u32,
) -> Ordering {
    b_mean
        .against(a_mean)
        .then_with(|| a.stop.cmp(&b.stop))
        .then_with(|| b.liquidity.cmp(&a.liquidity))
        .then_with(|| {
            *reached += 1;
            a.seeded.cmp(&b.seeded)
        })
        .then_with(|| a.stock.cmp(&b.stock))
        .then_with(|| a.recipe.cmp(&b.recipe))
}

/// The batch in arrival order: `order[i] = i` for the first `n` entries.
pub(super) fn arrival(n: usize, order: &mut [u16; UNIVERSE]) {
    for (slot, index) in order.iter_mut().zip(0u16..).take(n) {
        *slot = index;
    }
}

/// RANKED's offering order of a validated batch of at most [`UNIVERSE`] candidates: the first
/// `batch.len()` entries of `order` are their indices in rank order. Returns how many
/// compare-exchanges reached the seeded key.
pub(super) fn rank(
    batch: &[Candidate],
    edge: &EdgeTable,
    seed: u64,
    order: &mut [u16; UNIVERSE],
) -> u32 {
    let mut means = [MeanKey::NONE; UNIVERSE];
    let mut rests = [Rest::PAD; UNIVERSE];
    for ((candidate, mean), rest) in batch.iter().zip(means.iter_mut()).zip(rests.iter_mut()) {
        *mean = MeanKey::of(&edge.score(candidate.stock, candidate.side, candidate.minute));
        *rest = Rest::of(candidate, seed);
    }
    let real = batch.len();
    let width = real.next_power_of_two();
    arrival(width, order);
    let mut reached = 0;
    let mut size = 2;
    while size <= width {
        let mut stride = size / 2;
        while stride > 0 {
            for low in 0..width {
                let high = low ^ stride;
                if high > low {
                    let (a, b) = (read(order, low, 0), read(order, high, 0));
                    let ordering = match (usize::from(a) < real, usize::from(b) < real) {
                        (false, false) => Ordering::Equal,
                        (false, true) => Ordering::Greater,
                        (true, false) => Ordering::Less,
                        (true, true) => compare(
                            (
                                &read(&means, usize::from(a), MeanKey::NONE),
                                &read(&rests, usize::from(a), Rest::PAD),
                            ),
                            (
                                &read(&means, usize::from(b), MeanKey::NONE),
                                &read(&rests, usize::from(b), Rest::PAD),
                            ),
                            &mut reached,
                        ),
                    };
                    let swap = if low & size == 0 {
                        ordering == Ordering::Greater
                    } else {
                        ordering == Ordering::Less
                    };
                    if swap {
                        put(order, low, b);
                        put(order, high, a);
                    }
                }
            }
            stride /= 2;
        }
        size *= 2;
    }
    reached
}
