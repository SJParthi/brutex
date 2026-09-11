//! Deterministic Bonferroni allocation over a predeclared finite search scope.
//!
//! Each batch/timeframe cell is one complete multiple-testing family. Its
//! component procedure must itself control family error at the assigned level.
//! No independence across cells is needed for the finite union bound. Bootstrap
//! inputs retain their approximation/consistency assumptions: allocation is
//! not a distribution-free finite-sample guarantee and does not validate an
//! adaptively chosen grid. See Romano/Wolf (2005), section 1:
//! <https://doi.org/10.1198/016214504000000539>, and Tian/Ramdas (2021), section 2:
//! <https://arxiv.org/abs/1910.04900>.
//!
//! This pure module neither owns nor spends a durable slot. The caller must
//! commit the finite scope and exact hypothesis/procedure binding before work,
//! return the same slot on retry, and refuse a changed binding. A fresh process,
//! output directory, seed or batch boundary does not replenish the budget.

/// Invalid scope, probability or unrepresentable exact arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Zero scope width or an ordinal outside its predeclared scope.
    Scope,
    /// Alpha exceeds one, or a supplied probability is malformed.
    Probability,
    /// Checked exact arithmetic cannot represent this request.
    Arithmetic,
}

/// A reduced exact probability, with no rounded ppm intermediate.
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

/// One immutable finite batch/timeframe allocation, not persistence authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Allocation {
    batch: u64,
    batches: u64,
    rung: u64,
    rungs: u64,
    alpha_ppm: u64,
    families: u128,
    threshold: Fraction,
}
impl Allocation {
    /// Zero-based predeclared batch ordinal.
    #[must_use]
    pub const fn batch(self) -> u64 {
        self.batch
    }
    /// Complete predeclared batch count, including unexecuted batches.
    #[must_use]
    pub const fn batches(self) -> u64 {
        self.batches
    }
    /// Zero-based predeclared timeframe ordinal.
    #[must_use]
    pub const fn rung(self) -> u64 {
        self.rung
    }
    /// Complete predeclared timeframe count.
    #[must_use]
    pub const fn rungs(self) -> u64 {
        self.rungs
    }
    /// Shared finite search error budget supplied by the caller.
    #[must_use]
    pub const fn alpha_ppm(self) -> u64 {
        self.alpha_ppm
    }
    /// Equal family threshold: alpha divided by all declared batch/rung cells.
    #[must_use]
    pub const fn threshold(self) -> Fraction {
        self.threshold
    }
    /// Exact slot/scope/alpha identity. Child evidence belongs in the caller's binding.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        let mut h = brutex_core::blake3::Hasher::new();
        h.update(b"brutex-finite-family-allocation-v1\0");
        for n in [
            self.batch,
            self.batches,
            self.rung,
            self.rungs,
            self.alpha_ppm,
        ] {
            h.update(&n.to_le_bytes());
        }
        h.finalize()
    }
    /// Whether an exact family-adjusted probability reaches this slot's threshold.
    /// # Errors
    /// Malformed probability or overflowing exact cross multiplication.
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
    /// Full finite-search Bonferroni projection, capped at one; original p stays unchanged.
    /// # Errors
    /// Malformed probability or overflowing exact multiplication.
    pub fn scaled_probability(self, numerator: u64, denominator: u64) -> Result<Fraction, Refusal> {
        let p = probability(numerator, denominator)?;
        let numerator = p
            .numerator
            .checked_mul(self.families)
            .ok_or(Refusal::Arithmetic)?;
        Ok(reduced(numerator.min(p.denominator), p.denominator))
    }
    /// Minimum draws permitting the nonzero `(exceedances+1)/(draws+1)` floor
    /// to reach this threshold. `None` means a zero alpha admits no such probability.
    #[must_use]
    pub fn minimum_draws(self) -> Option<u128> {
        (self.threshold.numerator != 0).then(|| {
            self.threshold
                .denominator
                .div_ceil(self.threshold.numerator)
                - 1
        })
    }
}

/// Allocate equal alpha to a fixed finite batch/timeframe cell. Ordinals are zero-based.
/// No count is inferred from successful outcomes; refused/unexecuted cells keep their share.
/// # Errors
/// Invalid scope, alpha outside `[0,1]`, or unrepresentable exact denominator.
pub fn allocate(
    batch_ordinal: u64,
    total_batches: u64,
    rung: u64,
    total_rungs: u64,
    alpha_ppm: u64,
) -> Result<Allocation, Refusal> {
    if total_batches == 0
        || total_rungs == 0
        || batch_ordinal >= total_batches
        || rung >= total_rungs
    {
        return Err(Refusal::Scope);
    }
    if alpha_ppm > 1_000_000 {
        return Err(Refusal::Probability);
    }
    let families = u128::from(total_batches)
        .checked_mul(u128::from(total_rungs))
        .ok_or(Refusal::Arithmetic)?;
    let denominator = families.checked_mul(1_000_000).ok_or(Refusal::Arithmetic)?;
    Ok(Allocation {
        batch: batch_ordinal,
        batches: total_batches,
        rung,
        rungs: total_rungs,
        alpha_ppm,
        families,
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
#[path = "family_allocation_v1_tests.rs"]
mod tests;
