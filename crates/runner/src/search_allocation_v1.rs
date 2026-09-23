//! Fixed countable alpha spending across successive batches and eight rungs.
//!
//! Batch `b` and each rung receive `alpha / (8 * (b + 1) * (b + 2))`.
//! The first `n` complete batches spend exactly `alpha * n / (n + 1)`:
//! `1 / ((b + 1) * (b + 2)) = 1 / (b + 1) - 1 / (b + 2)`.
//! Thus every finite prefix, and the countable limit, stays within alpha.
//!
//! This is a deterministic numeric allocation, not a durable spending token.
//! The caller must fix and persist search order, family/program/source and
//! procedure identity before examining outcomes. Retries reuse the same slot;
//! refused, skipped and unexecuted slots never replenish another slot. Changing
//! directories, seeds or batch sizes must not restart the spending sequence.
//! Each complete rung family still needs a valid multiple-testing procedure.
//! The union bound does not require independence across slots; it does not
//! establish the calibration assumptions of a bootstrap or adaptive selection.
//! Finite machine arithmetic and finite bootstrap draws can explicitly refuse
//! later ordinals even though the mathematical sequence is countable.

/// The original eight intraday rungs; daily data remains context only.
pub const RUNGS: u64 = 8;

/// Invalid input, unrepresentable arithmetic or insufficient draw resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A rung is outside the fixed zero-based `0..8` range.
    Scope,
    /// Alpha exceeds one, or a supplied probability is malformed.
    Probability,
    /// A checked exact product cannot be represented in `u128`.
    Arithmetic,
    /// The nonzero bootstrap floor cannot reach this slot's threshold.
    Resolution,
}

/// A reduced exact probability; no rounded ppm intermediate is stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fraction {
    numerator: u128,
    denominator: u128,
}
impl Fraction {
    /// Exact numerator.
    #[must_use]
    pub const fn numerator(self) -> u128 {
        self.numerator
    }
    /// Positive exact denominator.
    #[must_use]
    pub const fn denominator(self) -> u128 {
        self.denominator
    }
}

/// One fixed ordinal/rung allocation, without persistence or admission authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Allocation {
    batch: u64,
    rung: u64,
    alpha_ppm: u64,
    weight: u128,
    threshold: Fraction,
}
impl Allocation {
    /// Zero-based ordinal in the fixed search sequence, including skipped slots.
    #[must_use]
    pub const fn batch(self) -> u64 {
        self.batch
    }
    /// Zero-based intraday rung ordinal.
    #[must_use]
    pub const fn rung(self) -> u64 {
        self.rung
    }
    /// Fixed number of intraday rungs.
    #[must_use]
    pub const fn rungs(self) -> u64 {
        RUNGS
    }
    /// Original shared research error budget in ppm.
    #[must_use]
    pub const fn alpha_ppm(self) -> u64 {
        self.alpha_ppm
    }
    /// Exact per-slot threshold `alpha / (8 * (b + 1) * (b + 2))`.
    #[must_use]
    pub const fn threshold(self) -> Fraction {
        self.threshold
    }
    /// Versioned slot identity; the caller separately binds the immutable search.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        let mut h = brutex_core::blake3::Hasher::new();
        h.update(b"brutex-countable-search-allocation-v1\0");
        for value in [self.batch, self.rung, RUNGS, self.alpha_ppm] {
            h.update(&value.to_le_bytes());
        }
        h.finalize()
    }
    /// Compare a family-adjusted exact probability with this slot's threshold.
    /// Equality passes; no ppm flooring can turn an excess into acceptance.
    /// # Errors
    /// A malformed probability or an unrepresentable exact cross-product.
    pub fn compare(self, numerator: u64, denominator: u64) -> Result<bool, Refusal> {
        let p = probability(numerator, denominator)?;
        let left = p
            .numerator
            .checked_mul(self.threshold.denominator)
            .ok_or(Refusal::Arithmetic)?;
        let right = p
            .denominator
            .checked_mul(self.threshold.numerator)
            .ok_or(Refusal::Arithmetic)?;
        Ok(left <= right)
    }
    /// The exact probability divided by this slot's share, capped at one.
    /// Compare this diagnostic with the original alpha, preserving the raw p.
    /// # Errors
    /// A malformed probability or an unrepresentable exact product.
    pub fn scaled_probability(self, numerator: u64, denominator: u64) -> Result<Fraction, Refusal> {
        let p = probability(numerator, denominator)?;
        let numerator = p
            .numerator
            .checked_mul(self.weight)
            .ok_or(Refusal::Arithmetic)?;
        Ok(reduced(numerator.min(p.denominator), p.denominator))
    }
    /// Minimum draws permitting `(exceedances + 1) / (draws + 1)` to reach
    /// this threshold. Zero alpha has no attainable nonzero floor.
    #[must_use]
    pub fn minimum_draws(self) -> Option<u128> {
        (self.threshold.numerator != 0).then(|| {
            self.threshold
                .denominator
                .div_ceil(self.threshold.numerator)
                - 1
        })
    }
    /// Refuse a procedure whose finite nonzero bootstrap floor cannot pass.
    /// This does not increase draws, round alpha up or recycle this slot.
    /// # Errors
    /// `Resolution` for zero alpha or fewer than the exact minimum draws.
    pub fn require_draws(self, draws: u64) -> Result<(), Refusal> {
        let minimum = self.minimum_draws().ok_or(Refusal::Resolution)?;
        if u128::from(draws) < minimum {
            return Err(Refusal::Resolution);
        }
        Ok(())
    }
}

/// Allocate a fixed share of one search-wide budget. There is no total-batch
/// estimate or outcome-dependent redistribution; all eight rungs retain shares.
/// # Errors
/// Invalid rung/alpha or a denominator outside checked `u128` representation.
pub fn allocate(batch: u64, rung: u64, alpha_ppm: u64) -> Result<Allocation, Refusal> {
    if rung >= RUNGS {
        return Err(Refusal::Scope);
    }
    if alpha_ppm > 1_000_000 {
        return Err(Refusal::Probability);
    }
    // Widen before adding: even u64::MAX + 2 is representable in u128.
    let next = u128::from(batch) + 1;
    let weight = next
        .checked_mul(next + 1)
        .and_then(|v| v.checked_mul(u128::from(RUNGS)))
        .ok_or(Refusal::Arithmetic)?;
    let denominator = weight.checked_mul(1_000_000).ok_or(Refusal::Arithmetic)?;
    Ok(Allocation {
        batch,
        rung,
        alpha_ppm,
        weight,
        threshold: reduced(u128::from(alpha_ppm), denominator),
    })
}
fn probability(numerator: u64, denominator: u64) -> Result<Fraction, Refusal> {
    if denominator == 0 || numerator > denominator {
        return Err(Refusal::Probability);
    }
    Ok(reduced(u128::from(numerator), u128::from(denominator)))
}
fn reduced(numerator: u128, denominator: u128) -> Fraction {
    let (mut a, mut b) = (numerator, denominator);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    Fraction {
        numerator: numerator / a,
        denominator: denominator / a,
    }
}

#[cfg(test)]
#[path = "search_allocation_v1_tests.rs"]
mod tests;
