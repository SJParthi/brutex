//! A versioned conservative extension of the existing adjusted RW procedure.
//!
//! All exact-zero session vectors remain present with a named conservative
//! probability of one. Nonzero constants refuse; nonconstant rows use the
//! unchanged shared bootstrap. Nonpositive observed statistics are forced to
//! one. For each positive observed threshold t, `max(0,M)>t` iff `M>t`:
//! removing point-zero vectors from numerical suffix maxima changes no positive
//! row. Their position after positive rows cannot alter the cumulative maximum.
//! Keeping nonpositive variable rows in the numerical family is essential.
//!
//! This is a sample-path identity with a conservative zero-floored reference,
//! not a new finite-sample inference theorem. Bootstrap consistency and the
//! underlying hypothesis construction remain prerequisites. An observed zero
//! series is not a claim that future returns are identically zero. The base
//! algorithm and its limits are Romano/Wolf (2016), algorithms 3.1 and 4.1:
//! <https://www.econ.uzh.ch/apps/workingpapers/wp/econwp219.pdf>.
//! Existing V1 bytes, receipts and behavior remain unchanged.

use crate::bootstrap::{
    FamilyTestsRefusalV1, RomanoWolfAdjustedCandidateV1, RomanoWolfAdjustedReceiptV1, SpaReceiptV1,
    WhiteRealityCheckReceiptV1, family_tests_v1, romano_wolf_adjusted_p_values_v1,
};

/// Additional requested numerical storage and complete resampling-work caps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// Maximum `strategies * periods * (draws + 1)` offered observations.
    pub max_work: u64,
    /// Conservative requested buffer-byte cap, excluding caller-owned inputs,
    /// allocator metadata and whole-process/runtime memory.
    pub max_bytes: u64,
}
/// Refusal is distinct from a complete family containing no rejection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Empty/misaligned/too-short input or zero draws/block length.
    Shape,
    /// An exact nonzero constant cannot be studentized.
    NonzeroConstant {
        /// Original caller index.
        strategy: usize,
    },
    /// Work or additional storage exceeds the explicit physical admission.
    Bound,
    /// Exact denominator, count or size arithmetic overflowed.
    Arithmetic,
    /// Fallible allocation failed.
    Allocation,
    /// The unchanged shared numerical method refused nonconstant inputs.
    Numerical,
}
/// Why a full-family row has its exact final probability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Classification {
    /// A positive observed statistic with the shared adjusted result.
    MeasuredPositive,
    /// Every exact input return was zero; probability one is a conservative rule.
    ConservativeZero,
    /// A variable row with a nonpositive observed statistic; probability one.
    ConservativeNonpositive,
}
/// Exact probability; a conservative one is `(draws+1)/(draws+1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Probability {
    numerator: u64,
    denominator: u64,
}
impl Probability {
    /// Exact numerator, without a ppm conversion.
    #[must_use]
    pub const fn numerator(self) -> u64 {
        self.numerator
    }
    /// Exact positive denominator.
    #[must_use]
    pub const fn denominator(self) -> u64 {
        self.denominator
    }
}
/// Full caller-position result with optional original nonconstant-subfamily facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    strategy: usize,
    classification: Classification,
    probability: Probability,
    shared: Option<RomanoWolfAdjustedCandidateV1>,
}
impl Candidate {
    /// Index in the complete caller family, including zeros.
    #[must_use]
    pub const fn strategy(self) -> usize {
        self.strategy
    }
    /// Explicit distinction between measured and conservative probabilities.
    #[must_use]
    pub const fn classification(self) -> Classification {
        self.classification
    }
    /// Conservative full-family adjusted probability.
    #[must_use]
    pub const fn p_value(self) -> Probability {
        self.probability
    }
    /// Unchanged shared numerical facts. Its strategy/rank refer to the
    /// nonconstant subfamily, not the full caller position.
    #[must_use]
    pub const fn shared(self) -> Option<RomanoWolfAdjustedCandidateV1> {
        self.shared
    }
}
/// Complete new-version result; every input row and procedure term is bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    rows: Vec<Candidate>,
    draws: usize,
    periods: usize,
    seed: u64,
    block: usize,
    bounds: Bounds,
    family_digest: [u8; 32],
    shared_digest: Option<[u8; 32]>,
}
/// Source-only cold audit; no resampling or probability is produced here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceAudit {
    statistics: Vec<Option<u64>>,
    family: [u8; 32],
    shared: Option<[u8; 32]>,
}
impl SourceAudit {
    /// Full original ordered matrix/procedure/bound identity.
    #[must_use]
    pub const fn family_digest(&self) -> [u8; 32] {
        self.family
    }
    /// Exact nonconstant subfamily identity, absent only for all-zero inputs.
    #[must_use]
    pub const fn shared_digest(&self) -> Option<[u8; 32]> {
        self.shared
    }
    /// Caller-ordered shared observed-statistic bits; inner None means exact zero.
    #[must_use]
    pub fn statistics(&self) -> &[Option<u64>] {
        &self.statistics
    }
}
/// Reproduce full identities and observed statistics through the same V1 kernel,
/// without reproducing bootstrap exceedance counts. O(strategies*periods) work;
/// the same conservative physical admission applies before active-row cloning.
/// # Errors
/// The same malformed, constant, numerical and physical refusals as `evaluate`.
pub fn audit_sources(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
    bounds: Bounds,
) -> Result<SourceAudit, Refusal> {
    let periods = returns.first().map_or(0, Vec::len);
    if periods < 2 || draws == 0 || block == 0 || returns.iter().any(|row| row.len() != periods) {
        return Err(Refusal::Shape);
    }
    u64::try_from(draws.checked_add(1).ok_or(Refusal::Arithmetic)?)
        .map_err(|_| Refusal::Arithmetic)?;
    let (work, bytes) = requirements(returns.len(), periods, draws)?;
    if work > u128::from(bounds.max_work) || bytes > u128::from(bounds.max_bytes) {
        return Err(Refusal::Bound);
    }
    let mut statistics = Vec::new();
    statistics
        .try_reserve_exact(returns.len())
        .map_err(|_| Refusal::Allocation)?;
    let mut active = Vec::new();
    active
        .try_reserve_exact(returns.len())
        .map_err(|_| Refusal::Allocation)?;
    for (strategy, series) in returns.iter().enumerate() {
        let first = *series.first().ok_or(Refusal::Shape)?;
        if series.iter().all(|&value| value == first) {
            if first != 0 {
                return Err(Refusal::NonzeroConstant { strategy });
            }
            statistics.push(None);
            continue;
        }
        let summary = crate::bootstrap::summarise(series);
        if summary.standard_error <= 0.0 || !summary.standard_error.is_finite() {
            return Err(Refusal::Numerical);
        }
        let observed = crate::bootstrap::studentized(summary.mean, summary.standard_error);
        if !observed.is_finite() {
            return Err(Refusal::Numerical);
        }
        statistics.push(Some(observed.to_bits()));
        let mut copied = Vec::new();
        copied
            .try_reserve_exact(periods)
            .map_err(|_| Refusal::Allocation)?;
        copied.extend_from_slice(series);
        active.push(copied);
    }
    let shared = if active.is_empty() {
        None
    } else {
        Some(
            crate::bootstrap::romano_wolf_family_digest_v1(&active, draws, seed, block)
                .ok_or(Refusal::Arithmetic)?,
        )
    };
    Ok(SourceAudit {
        statistics,
        family: digest(returns, draws, seed, block, bounds)?,
        shared,
    })
}
impl Receipt {
    /// Complete caller-ordered rows. No zero or nonpositive candidate is omitted.
    #[must_use]
    pub fn rows(&self) -> &[Candidate] {
        &self.rows
    }
    /// O(1) lookup by the complete caller position.
    #[must_use]
    pub fn candidate(&self, strategy: usize) -> Option<&Candidate> {
        self.rows.get(strategy)
    }
    /// Complete population cardinality.
    #[must_use]
    pub fn strategies(&self) -> usize {
        self.rows.len()
    }
    /// Shared bootstrap draw count.
    #[must_use]
    pub const fn draws(&self) -> usize {
        self.draws
    }
    /// Full aligned period count.
    #[must_use]
    pub const fn periods(&self) -> usize {
        self.periods
    }
    /// Exact supplied deterministic seed.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }
    /// Shared stationary-bootstrap block length.
    #[must_use]
    pub const fn block(&self) -> usize {
        self.block
    }
    /// Explicit physical admission used by the producer.
    #[must_use]
    pub const fn bounds(&self) -> Bounds {
        self.bounds
    }
    /// New-domain identity of every original ordered return and procedure/bound.
    #[must_use]
    pub const fn family_digest(&self) -> [u8; 32] {
        self.family_digest
    }
    /// Shared method's nonconstant-subfamily identity, absent for an all-zero family.
    #[must_use]
    pub const fn shared_digest(&self) -> Option<[u8; 32]> {
        self.shared_digest
    }
}

/// Evaluate all input rows with explicit zero/nonpositive conservative outcomes.
/// The active clone is bounded before allocation. It retains variable negative
/// rows because they can contribute positive centered resample maxima.
/// # Errors
/// Invalid shape, nonzero constants, numerical failure, allocation or physical cap.
pub fn evaluate(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
    bounds: Bounds,
) -> Result<Receipt, Refusal> {
    let plan = admit(returns, draws, seed, block, bounds)?;
    let mut active = Vec::new();
    active
        .try_reserve_exact(returns.len())
        .map_err(|_| Refusal::Allocation)?;
    let mut positions = Vec::new();
    positions
        .try_reserve_exact(returns.len())
        .map_err(|_| Refusal::Allocation)?;
    for (strategy, series) in returns.iter().enumerate() {
        let first = *series.first().ok_or(Refusal::Shape)?;
        if series.iter().all(|&v| v == first) {
            if first != 0 {
                return Err(Refusal::NonzeroConstant { strategy });
            }
        } else {
            let mut copy = Vec::new();
            copy.try_reserve_exact(plan.periods)
                .map_err(|_| Refusal::Allocation)?;
            copy.extend_from_slice(series);
            active.push(copy);
            positions.push(strategy);
        }
    }
    let shared = if active.is_empty() {
        None
    } else {
        Some(
            romano_wolf_adjusted_p_values_v1(&active, draws, seed, block)
                .ok_or(Refusal::Numerical)?,
        )
    };
    assemble(returns, plan, &positions, shared.as_ref())
}

/// [`evaluate`]'s receipt together with White's and Hansen's receipts for the
/// same complete family, all three measured from ONE walk over the draws.
///
/// Byte for byte what `evaluate`,
/// [`crate::bootstrap::white_reality_check_receipt_v1`] and
/// [`crate::bootstrap::spa_receipt_v1`] return for the same inputs. The exact
/// zero rows stay in White's and SPA's family and out of Romano--Wolf's, as
/// those separate calls treat them, and the walk is
/// [`crate::bootstrap::family_tests_v1`]. The Romano--Wolf rows are named by
/// position rather than cloned, so `bounds` -- unchanged, because it is part of
/// the family digest -- over-counts the buffers this path holds.
///
/// # Errors
///
/// The refusal of the first of the three separate calls that refuses, in the
/// order the index-stop qualification has always made them: `evaluate`'s own,
/// then White's, then SPA's.
pub fn evaluate_with_family_tests(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
    bounds: Bounds,
) -> Result<FamilyEvaluation, FamilyRefusal> {
    let plan = admit(returns, draws, seed, block, bounds).map_err(FamilyRefusal::RomanoWolf)?;
    let mut positions = Vec::new();
    positions
        .try_reserve_exact(returns.len())
        .map_err(|_| FamilyRefusal::RomanoWolf(Refusal::Allocation))?;
    for (strategy, series) in returns.iter().enumerate() {
        let first = *series
            .first()
            .ok_or(FamilyRefusal::RomanoWolf(Refusal::Shape))?;
        if series.iter().all(|&v| v == first) {
            if first != 0 {
                return Err(FamilyRefusal::RomanoWolf(Refusal::NonzeroConstant {
                    strategy,
                }));
            }
        } else {
            positions.push(strategy);
        }
    }
    let tests = family_tests_v1(returns, &positions, draws, seed, block).map_err(family_refusal)?;
    let romano_wolf = assemble(returns, plan, &positions, tests.romano_wolf())
        .map_err(FamilyRefusal::RomanoWolf)?;
    Ok(FamilyEvaluation {
        romano_wolf,
        white: tests.white(),
        spa: tests.spa(),
    })
}

/// [`evaluate`]'s receipt with White's and Hansen's from the same draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyEvaluation {
    romano_wolf: Receipt,
    white: WhiteRealityCheckReceiptV1,
    spa: SpaReceiptV1,
}
impl FamilyEvaluation {
    /// Exactly [`evaluate`]'s receipt for the same inputs.
    #[must_use]
    pub const fn romano_wolf(&self) -> &Receipt {
        &self.romano_wolf
    }
    /// Exactly White's separate receipt for the complete family.
    #[must_use]
    pub const fn white(&self) -> WhiteRealityCheckReceiptV1 {
        self.white
    }
    /// Exactly Hansen's separate receipt for the complete family.
    #[must_use]
    pub const fn spa(&self) -> SpaReceiptV1 {
        self.spa
    }
}

/// Which of the three separate calls refuses first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FamilyRefusal {
    /// [`evaluate`] refuses, for this reason.
    RomanoWolf(Refusal),
    /// White's separate receipt refuses the complete family.
    White,
    /// Hansen's separate receipt refuses the complete family.
    Spa,
}

/// Names the separate call a shared-walk refusal stands for. A row or walk
/// failure belongs to this procedure's numerical step, which `evaluate`
/// reports as [`Refusal::Numerical`].
const fn family_refusal(why: FamilyTestsRefusalV1) -> FamilyRefusal {
    match why {
        FamilyTestsRefusalV1::White => FamilyRefusal::White,
        FamilyTestsRefusalV1::Spa => FamilyRefusal::Spa,
        FamilyTestsRefusalV1::Rows
        | FamilyTestsRefusalV1::RomanoWolf
        | FamilyTestsRefusalV1::Pass => FamilyRefusal::RomanoWolf(Refusal::Numerical),
    }
}

/// The procedure terms every row shares, admitted before any row is read.
#[derive(Clone, Copy)]
struct Plan {
    draws: usize,
    seed: u64,
    block: usize,
    bounds: Bounds,
    periods: usize,
    denominator: u64,
}

/// Shape, exact denominator and physical admission, refused in that order.
fn admit(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
    bounds: Bounds,
) -> Result<Plan, Refusal> {
    let periods = returns.first().map_or(0, Vec::len);
    if periods < 2 || draws == 0 || block == 0 || returns.iter().any(|r| r.len() != periods) {
        return Err(Refusal::Shape);
    }
    let denominator = u64::try_from(draws.checked_add(1).ok_or(Refusal::Arithmetic)?)
        .map_err(|_| Refusal::Arithmetic)?;
    let (work, bytes) = requirements(returns.len(), periods, draws)?;
    if work > u128::from(bounds.max_work) || bytes > u128::from(bounds.max_bytes) {
        return Err(Refusal::Bound);
    }
    Ok(Plan {
        draws,
        seed,
        block,
        bounds,
        periods,
        denominator,
    })
}

/// Every caller row, conservative by default, with the shared Romano--Wolf
/// facts of each nonconstant row laid over its caller position.
fn assemble(
    returns: &[Vec<i64>],
    plan: Plan,
    positions: &[usize],
    shared: Option<&RomanoWolfAdjustedReceiptV1>,
) -> Result<Receipt, Refusal> {
    let denominator = plan.denominator;
    let mut rows = Vec::new();
    rows.try_reserve_exact(returns.len())
        .map_err(|_| Refusal::Allocation)?;
    for strategy in 0..returns.len() {
        rows.push(Candidate {
            strategy,
            classification: Classification::ConservativeZero,
            probability: Probability {
                numerator: denominator,
                denominator,
            },
            shared: None,
        });
    }
    if let Some(receipt) = shared {
        for (at, &strategy) in positions.iter().enumerate() {
            let original = *receipt.candidate(at).ok_or(Refusal::Numerical)?;
            let row = rows.get_mut(strategy).ok_or(Refusal::Numerical)?;
            row.shared = Some(original);
            if original.observed_statistic() > 0.0 {
                row.classification = Classification::MeasuredPositive;
                row.probability.numerator = u64::try_from(original.adjusted_p_value().numerator())
                    .map_err(|_| Refusal::Arithmetic)?;
            } else {
                row.classification = Classification::ConservativeNonpositive;
            }
        }
    }
    Ok(Receipt {
        rows,
        draws: plan.draws,
        periods: plan.periods,
        seed: plan.seed,
        block: plan.block,
        bounds: plan.bounds,
        family_digest: digest(returns, plan.draws, plan.seed, plan.block, plan.bounds)?,
        shared_digest: shared.map(RomanoWolfAdjustedReceiptV1::family_digest),
    })
}
fn requirements(strategies: usize, periods: usize, draws: usize) -> Result<(u128, u128), Refusal> {
    let (s, n, b) = (strategies as u128, periods as u128, draws as u128);
    let values = s.checked_mul(n).ok_or(Refusal::Arithmetic)?;
    let work = values
        .checked_mul(b.checked_add(1).ok_or(Refusal::Arithmetic)?)
        .ok_or(Refusal::Arithmetic)?;
    // Input clones + index matrix + row/index/performance/output scratch.
    // 512 bytes/strategy conservatively exceeds the sum of shared V1 structs;
    // it does not claim allocator bookkeeping or resident-set accounting.
    let bytes = values
        .checked_mul(8)
        .and_then(|v| {
            b.checked_mul(n)?
                .checked_mul(size_of::<usize>() as u128)?
                .checked_add(v)
        })
        .and_then(|v| {
            b.checked_mul((size_of::<Vec<usize>>() + 8) as u128)?
                .checked_add(v)
        })
        .and_then(|v| {
            s.checked_mul(512 + size_of::<Candidate>() as u128)?
                .checked_add(v)
        })
        .and_then(|v| v.checked_add(1024))
        .ok_or(Refusal::Arithmetic)?;
    Ok((work, bytes))
}
fn digest(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
    bounds: Bounds,
) -> Result<[u8; 32], Refusal> {
    let mut h = brutex_core::blake3::Hasher::new();
    h.update(b"brutex-rw-conservative-zero-v2\0");
    for n in [
        u64::try_from(draws).map_err(|_| Refusal::Arithmetic)?,
        seed,
        u64::try_from(block).map_err(|_| Refusal::Arithmetic)?,
        bounds.max_work,
        bounds.max_bytes,
        u64::try_from(returns.len()).map_err(|_| Refusal::Arithmetic)?,
    ] {
        h.update(&n.to_le_bytes());
    }
    for (i, row) in returns.iter().enumerate() {
        h.update(
            &u64::try_from(i)
                .map_err(|_| Refusal::Arithmetic)?
                .to_le_bytes(),
        );
        h.update(
            &u64::try_from(row.len())
                .map_err(|_| Refusal::Arithmetic)?
                .to_le_bytes(),
        );
        for n in row {
            h.update(&n.to_le_bytes());
        }
    }
    Ok(h.finalize())
}

#[cfg(test)]
#[path = "bootstrap_zero_v2_tests.rs"]
mod tests;
