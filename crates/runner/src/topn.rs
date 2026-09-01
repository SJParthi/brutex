//! One authoritative, bounded Top-N ordering over a completed candidate population.
//!
//! # Why this is separate from evidence ranking
//!
//! [`crate::rank`] answers which frequent masks have the strongest forward
//! evidence.  The operator's final comparison asks a different question over
//! eleven exit-cell measurements: less loss and drawdown, more pessimistic
//! profit and wins, and stronger ratios.  Cutting on the first ordering before
//! measuring the second cannot produce a global Top-N under the second.
//!
//! This module owns only the final ordering.  A caller first feeds every fully
//! evaluated candidate into [`Extrema`], seals that population, then feeds the
//! same records through [`Keeper`].  The two-pass shape is required because a
//! normalised score depends on the minimum and maximum over the whole
//! population.  It is also what lets a disk-backed caller keep constant RAM:
//! candidate records may be streamed in both passes and the keeper holds at
//! most [`MAX_TOP`] rows.
//!
//! # What this does not claim
//!
//! A complete brute-force search is not O(1): every eligible candidate must be
//! generated, priced and offered, otherwise an unseen candidate may be the
//! winner.  Keeper state is bounded by 25 and every insertion examines at most
//! 25 rows, but total time is O(candidates) after pricing and a durable
//! two-pass record is O(candidates) on disk.  A caller that supplied a capped
//! evidence prefix has a deterministic Top-N *of that prefix*, never a global
//! Top-N; completion is a receipt owned by the caller, not inferred here.

use core::cmp::Ordering;

use crate::portfolio::StrategyDigest;
use costs::fill::Direction;

/// Largest authoritative list this policy retains.
pub const MAX_TOP: usize = 25;

/// Fixed-point denominator used by every normalised criterion.
pub const SCORE_SCALE: u64 = 1_000_000;

/// Bytes in the canonical version-one final-ranking policy record.
///
/// This is an explicit wire contract, not `size_of::<RankingPolicyV1>()`.
pub const RANKING_POLICY_CANONICAL_LEN_V1: usize = 76;

const RANKING_POLICY_DOMAIN_V1: [u8; 16] = *b"brutex-topn-v1\0\0";
const RANKING_POLICY_VERSION_V1: u32 = 1;
const RANKING_POLICY_PAYLOAD_LEN_V1: u32 = 52;

const CRITERIA: usize = 11;
const CRITERIA_U32: u32 = 11;
const NEUTRAL: u64 = SCORE_SCALE / 2;

/// Non-negative runtime weights for the eleven final criteria.
///
/// Fields follow the operator-facing comparison table.  A zero switches one
/// criterion off.  All eleven zero is refused by [`RankingPolicyV1::new`]
/// because it leaves no ranking policy at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Weights {
    /// Prefer less maximum drawdown.
    pub drawdown: u32,
    /// Prefer a smaller worst-trade loss magnitude.
    pub worst_loss: u32,
    /// Prefer a smaller losing-trade percentage.
    pub losing_rate: u32,
    /// Prefer fewer losing trades.
    pub losing_trades: u32,
    /// Prefer a smaller gross-loss to gross-win ratio.
    pub loss_ratio: u32,
    /// Prefer larger pessimistic profit.
    pub pessimistic_profit: u32,
    /// Prefer more winning trades.
    pub winning_trades: u32,
    /// Prefer a larger winning-trade percentage.
    pub win_rate: u32,
    /// Prefer a larger reward-to-risk ratio.
    pub reward_to_risk: u32,
    /// Prefer a larger average win.
    pub average_win: u32,
    /// Prefer a smaller average-loss magnitude.
    pub average_loss: u32,
}

impl Weights {
    /// Equal weight on all eleven criteria.
    #[must_use]
    pub const fn equal() -> Self {
        Self {
            drawdown: 1,
            worst_loss: 1,
            losing_rate: 1,
            losing_trades: 1,
            loss_ratio: 1,
            pessimistic_profit: 1,
            winning_trades: 1,
            win_rate: 1,
            reward_to_risk: 1,
            average_win: 1,
            average_loss: 1,
        }
    }

    const fn values(self) -> [u32; CRITERIA] {
        [
            self.drawdown,
            self.worst_loss,
            self.losing_rate,
            self.losing_trades,
            self.loss_ratio,
            self.pessimistic_profit,
            self.winning_trades,
            self.win_rate,
            self.reward_to_risk,
            self.average_win,
            self.average_loss,
        ]
    }
}

/// Raw final measurements for one fully priced candidate.
///
/// Losses are magnitudes, so their domains are non-negative and the ordering
/// cannot accidentally reward a more negative signed value.  Ratios are parts
/// per million.  `None` is an undefined denominator rather than zero; policy V1
/// scores it at the neutral midpoint and counts the candidate as unmeasured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metrics {
    /// Largest peak-to-trough loss magnitude, in paisa.
    pub drawdown: u64,
    /// Largest single losing-trade magnitude, in paisa.
    pub worst_loss: u64,
    /// Losing trades as parts per million of all trades.
    pub losing_rate_ppm: u64,
    /// Number of losing trades.
    pub losing_trades: u64,
    /// Gross-loss magnitude divided by gross win, in ppm; undefined with no win.
    pub loss_ratio_ppm: Option<u64>,
    /// Total under pessimistic one-minute fills, in paisa.
    pub pessimistic_profit: i64,
    /// Number of winning trades.
    pub winning_trades: u64,
    /// Winning trades as parts per million of all trades.
    pub win_rate_ppm: u64,
    /// Reward divided by risk, in ppm; undefined with no loss denominator.
    pub reward_to_risk_ppm: Option<u64>,
    /// Average winning-trade magnitude, in paisa.
    pub average_win: u64,
    /// Average losing-trade magnitude, in paisa.
    pub average_loss: u64,
    /// Statistical lower-bound assurance, in ppm, used by the tie-break.
    pub assurance_ppm: u64,
}

impl Metrics {
    fn values(self) -> [Option<i128>; CRITERIA] {
        [
            Some(i128::from(self.drawdown)),
            Some(i128::from(self.worst_loss)),
            Some(i128::from(self.losing_rate_ppm)),
            Some(i128::from(self.losing_trades)),
            self.loss_ratio_ppm.map(i128::from),
            Some(i128::from(self.pessimistic_profit)),
            Some(i128::from(self.winning_trades)),
            Some(i128::from(self.win_rate_ppm)),
            self.reward_to_risk_ppm.map(i128::from),
            Some(i128::from(self.average_win)),
            Some(i128::from(self.average_loss)),
        ]
    }

    fn has_unmeasured(self) -> bool {
        self.values().into_iter().any(|value| value.is_none())
    }
}

/// One canonical candidate offered to the final ranking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// Complete identity of this semantic strategy member.
    ///
    /// The digest is caller-owned because this kernel cannot reconstruct the
    /// instrument, timeframe, exit-cell key or evaluation-policy digest from
    /// final metrics. It is also the last total-order tie-break, so two exit
    /// cells sharing one mask and direction can never collapse into one row.
    pub strategy_digest: StrategyDigest,
    /// Six append-only vocabulary words identifying the condition combination.
    pub mask_words: [u64; 6],
    /// Explicit trade direction.
    pub direction: Direction,
    /// Whether the locked institutional admission stack accepted the candidate.
    pub admitted: bool,
    /// The eleven comparison measurements and assurance tie-break.
    pub metrics: Metrics,
}

/// Exact ordered-population proof shared by the extrema and selection passes.
///
/// Equal counts are insufficient: omission plus duplication or an in-range
/// replacement can keep every extrema and counter unchanged. This digest binds
/// every fixed candidate field in arrival order, while the counters make an
/// operator-readable reconciliation available without decoding the digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationProof {
    /// Every candidate observed, admitted or refused.
    pub considered: u64,
    /// Candidates admitted by the locked institutional policy.
    pub admitted: u64,
    /// Candidates refused by that policy.
    pub refused: u64,
    /// Candidates carrying at least one undefined final ratio.
    pub unmeasured: u64,
    /// BLAKE3 of the complete ordered fixed-field population.
    pub ordered_digest: [u8; 32],
}

/// Streaming builder for one exact population pass.
///
/// State is constant with respect to population length. The caller may stream
/// a disk ledger through this value without retaining any candidate row.
pub struct PopulationPass {
    hasher: brutex_core::blake3::Hasher,
    considered: u64,
    admitted: u64,
    refused: u64,
    unmeasured: u64,
}

impl PopulationPass {
    /// Starts one V1 ordered-population proof.
    #[must_use]
    pub fn new() -> Self {
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(b"brutex-topn-population-v1\0");
        Self {
            hasher,
            considered: 0,
            admitted: 0,
            refused: 0,
            unmeasured: 0,
        }
    }

    /// Adds one candidate in canonical population order.
    pub fn observe(&mut self, candidate: &Candidate) {
        self.considered = self.considered.saturating_add(1);
        if candidate.admitted {
            self.admitted = self.admitted.saturating_add(1);
        } else {
            self.refused = self.refused.saturating_add(1);
        }
        if candidate.metrics.has_unmeasured() {
            self.unmeasured = self.unmeasured.saturating_add(1);
        }

        self.hasher.update(&candidate.strategy_digest.bytes());
        for word in candidate.mask_words {
            self.hasher.update(&word.to_le_bytes());
        }
        self.hasher.update(&[match candidate.direction {
            Direction::Long => 1,
            Direction::Short => 2,
        }]);
        self.hasher.update(&[u8::from(candidate.admitted)]);
        hash_metrics(&mut self.hasher, candidate.metrics);
    }

    /// Seals this pass's exact counts and ordered digest.
    #[must_use]
    pub fn finish(self) -> PopulationProof {
        PopulationProof {
            considered: self.considered,
            admitted: self.admitted,
            refused: self.refused,
            unmeasured: self.unmeasured,
            ordered_digest: self.hasher.finalize(),
        }
    }
}

impl Default for PopulationPass {
    fn default() -> Self {
        Self::new()
    }
}

fn hash_metrics(hasher: &mut brutex_core::blake3::Hasher, metrics: Metrics) {
    for value in [
        metrics.drawdown,
        metrics.worst_loss,
        metrics.losing_rate_ppm,
        metrics.losing_trades,
    ] {
        hasher.update(&value.to_le_bytes());
    }
    hash_optional(hasher, metrics.loss_ratio_ppm);
    hasher.update(&metrics.pessimistic_profit.to_le_bytes());
    for value in [metrics.winning_trades, metrics.win_rate_ppm] {
        hasher.update(&value.to_le_bytes());
    }
    hash_optional(hasher, metrics.reward_to_risk_ppm);
    for value in [
        metrics.average_win,
        metrics.average_loss,
        metrics.assurance_ppm,
    ] {
        hasher.update(&value.to_le_bytes());
    }
}

fn hash_optional(hasher: &mut brutex_core::blake3::Hasher, value: Option<u64>) {
    if let Some(value) = value {
        hasher.update(&[1]);
        hasher.update(&value.to_le_bytes());
    } else {
        hasher.update(&[0]);
        hasher.update(&0_u64.to_le_bytes());
    }
}

/// Why a ranking policy or selection request was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No row count was requested.
    TopIsZero,
    /// The requested row count exceeded [`MAX_TOP`].
    TopAboveMaximum,
    /// All eleven weights were zero, so there was no score to order on.
    EveryWeightIsZero,
    /// A second-pass candidate fell outside the extrema sealed by the first pass.
    CandidateOutsidePopulation,
    /// The second pass did not reproduce the first pass byte for byte and row
    /// for row in the same canonical order.
    PopulationMismatch,
}

/// Version-one authoritative final ranking policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RankingPolicyV1 {
    weights: Weights,
}

impl RankingPolicyV1 {
    /// Builds a policy from runtime weights.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::EveryWeightIsZero`] when no criterion carries weight.
    pub fn new(weights: Weights) -> Result<Self, Refusal> {
        if weights.values().into_iter().all(|weight| weight == 0) {
            return Err(Refusal::EveryWeightIsZero);
        }
        Ok(Self { weights })
    }

    /// The exact runtime weights this policy applies.
    #[must_use]
    pub const fn weights(self) -> Weights {
        self.weights
    }

    /// Canonical fixed-size V1 bytes for this complete ranking policy.
    ///
    /// The record carries a domain, version, payload length, criterion count,
    /// a zero reserve and all eleven weights in their stable criterion order.
    /// It never depends on Rust layout, padding or enum representation.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; RANKING_POLICY_CANONICAL_LEN_V1] {
        let mut bytes = [0_u8; RANKING_POLICY_CANONICAL_LEN_V1];
        if let Some(slot) = bytes.get_mut(..16) {
            slot.copy_from_slice(&RANKING_POLICY_DOMAIN_V1);
        }
        if let Some(slot) = bytes.get_mut(16..20) {
            slot.copy_from_slice(&RANKING_POLICY_VERSION_V1.to_le_bytes());
        }
        if let Some(slot) = bytes.get_mut(20..24) {
            slot.copy_from_slice(&RANKING_POLICY_PAYLOAD_LEN_V1.to_le_bytes());
        }
        if let Some(slot) = bytes.get_mut(24..28) {
            slot.copy_from_slice(&CRITERIA_U32.to_le_bytes());
        }
        // bytes 28..32 are the V1 reserve and remain zero.
        for (index, weight) in self.weights.values().into_iter().enumerate() {
            let start = 32_usize.saturating_add(index.saturating_mul(4));
            let end = start.saturating_add(4);
            if let Some(slot) = bytes.get_mut(start..end) {
                slot.copy_from_slice(&weight.to_le_bytes());
            }
        }
        bytes
    }

    /// Domain-separated BLAKE3 of [`Self::canonical_bytes`].
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        brutex_core::blake3::hash(&self.canonical_bytes())
    }

    fn score(self, candidate: &Candidate, extrema: &Extrema) -> Result<u64, Refusal> {
        let values = candidate.metrics.values();
        let weights = self.weights.values();
        let mut weighted = 0_u128;
        let mut total_weight = 0_u128;
        for (((range, value), weight), preference) in extrema
            .ranges
            .iter()
            .zip(values)
            .zip(weights)
            .zip(PREFERENCES)
        {
            let weight = u128::from(weight);
            if weight == 0 {
                continue;
            }
            let part = range.normalise(value, preference)?;
            weighted = weighted.saturating_add(u128::from(part).saturating_mul(weight));
            total_weight = total_weight.saturating_add(weight);
        }
        let score = weighted.checked_div(total_weight).unwrap_or(0);
        Ok(u64::try_from(score).unwrap_or(SCORE_SCALE))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Range {
    low: Option<i128>,
    high: Option<i128>,
}

impl Range {
    fn observe(&mut self, value: i128) {
        self.low = Some(self.low.map_or(value, |known| known.min(value)));
        self.high = Some(self.high.map_or(value, |known| known.max(value)));
    }

    fn normalise(self, value: Option<i128>, preference: Preference) -> Result<u64, Refusal> {
        let Some(value) = value else {
            return Ok(NEUTRAL);
        };
        let (Some(low), Some(high)) = (self.low, self.high) else {
            return Err(Refusal::CandidateOutsidePopulation);
        };
        if value < low || value > high {
            return Err(Refusal::CandidateOutsidePopulation);
        }
        if low == high {
            return Ok(NEUTRAL);
        }
        let width = high.saturating_sub(low);
        let favourable = match preference {
            Preference::Lower => high.saturating_sub(value),
            Preference::Higher => value.saturating_sub(low),
        };
        let scaled = favourable
            .saturating_mul(i128::from(SCORE_SCALE))
            .checked_div(width)
            .unwrap_or(i128::from(NEUTRAL));
        u64::try_from(scaled).map_err(|_| Refusal::CandidateOutsidePopulation)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Preference {
    Lower,
    Higher,
}

const PREFERENCES: [Preference; CRITERIA] = [
    Preference::Lower,
    Preference::Lower,
    Preference::Lower,
    Preference::Lower,
    Preference::Lower,
    Preference::Higher,
    Preference::Higher,
    Preference::Higher,
    Preference::Higher,
    Preference::Higher,
    Preference::Lower,
];

/// First-pass extrema over the complete eligible population.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Extrema {
    ranges: [Range; CRITERIA],
    observed: u64,
    admitted: u64,
}

impl Extrema {
    /// Adds one fully evaluated candidate to the first pass.
    pub fn observe(&mut self, candidate: &Candidate) {
        self.observed = self.observed.saturating_add(1);
        if !candidate.admitted {
            return;
        }
        self.admitted = self.admitted.saturating_add(1);
        for (range, value) in self.ranges.iter_mut().zip(candidate.metrics.values()) {
            if let Some(value) = value {
                range.observe(value);
            }
        }
    }

    /// Candidates observed in the first pass.
    #[must_use]
    pub const fn observed(self) -> u64 {
        self.observed
    }

    /// Institutionally admitted candidates that define the score ranges.
    #[must_use]
    pub const fn admitted(self) -> u64 {
        self.admitted
    }
}

/// One candidate with its authoritative fixed-point score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RankedCandidate {
    /// The original canonical candidate.
    pub candidate: Candidate,
    /// Weighted score in `0..=`[`SCORE_SCALE`].
    pub score: u64,
}

impl Ord for RankedCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.candidate
            .admitted
            .cmp(&other.candidate.admitted)
            .then_with(|| self.score.cmp(&other.score))
            .then_with(|| {
                self.candidate
                    .metrics
                    .pessimistic_profit
                    .cmp(&other.candidate.metrics.pessimistic_profit)
            })
            .then_with(|| {
                self.candidate
                    .metrics
                    .assurance_ppm
                    .cmp(&other.candidate.metrics.assurance_ppm)
            })
            // Smaller canonical masks and direction bytes preserve the
            // vocabulary-facing order. The complete strategy digest then
            // distinguishes two exit cells whose visible fields are equal.
            .then_with(|| other.candidate.mask_words.cmp(&self.candidate.mask_words))
            .then_with(|| other.candidate.direction.cmp(&self.candidate.direction))
            .then_with(|| {
                other
                    .candidate
                    .strategy_digest
                    .cmp(&self.candidate.strategy_digest)
            })
    }
}

impl PartialOrd for RankedCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Exact counts accompanying one bounded final list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    /// Strongest first under [`RankingPolicyV1`].
    pub rows: Vec<RankedCandidate>,
    /// Requested bound, retained even when the population is smaller.
    pub requested: usize,
    /// Every second-pass candidate offered.
    pub considered: u64,
    /// Offered candidates whose institutional verdict admitted them.
    pub admitted: u64,
    /// Offered candidates whose institutional verdict refused them.
    pub refused: u64,
    /// Offered candidates carrying at least one undefined ratio.
    pub unmeasured: u64,
}

impl Selection {
    /// The first ten rows of this same ordering.
    ///
    /// A Top-10 is therefore a prefix of the authoritative Top-25 and never a
    /// second calculation that can disagree with it.
    #[must_use]
    pub fn top_ten(&self) -> &[RankedCandidate] {
        let end = self.rows.len().min(10);
        self.rows.get(..end).unwrap_or(&[])
    }
}

/// Second-pass fixed-capacity keeper.
pub struct Keeper {
    requested: usize,
    policy: RankingPolicyV1,
    extrema: Extrema,
    // Strongest first. Its length never exceeds `MAX_TOP`, so the insertion
    // walk and shift are bounded by a compile-time constant rather than by the
    // candidate population.
    held: Vec<RankedCandidate>,
    considered: u64,
    admitted: u64,
    refused: u64,
    unmeasured: u64,
}

/// A second-pass keeper that refuses unless it reproduces the first pass's
/// complete ordered-population proof.
pub struct VerifiedKeeper {
    keeper: Keeper,
    expected: PopulationProof,
    pass: PopulationPass,
}

impl VerifiedKeeper {
    /// Builds a bounded selector tied to one exact first-pass proof.
    ///
    /// # Errors
    ///
    /// Returns every refusal from [`Keeper::new`].
    pub fn new(
        requested: usize,
        policy: RankingPolicyV1,
        extrema: Extrema,
        expected: PopulationProof,
    ) -> Result<Self, Refusal> {
        Ok(Self {
            keeper: Keeper::new(requested, policy, extrema)?,
            expected,
            pass: PopulationPass::new(),
        })
    }

    /// Offers one second-pass row to both the proof and bounded selection.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::CandidateOutsidePopulation`] when a measurable field
    /// is outside the first-pass extrema.
    pub fn offer(&mut self, candidate: Candidate) -> Result<(), Refusal> {
        self.pass.observe(&candidate);
        self.keeper.offer(candidate)
    }

    /// Seals the Top-N only when both passes match exactly.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::PopulationMismatch`] for any omission, duplication,
    /// replacement or reordering, even when row counts and extrema agree.
    pub fn finish(self) -> Result<Selection, Refusal> {
        if self.pass.finish() != self.expected {
            return Err(Refusal::PopulationMismatch);
        }
        Ok(self.keeper.finish())
    }
}

impl Keeper {
    /// Builds a second-pass keeper over already sealed extrema.
    ///
    /// # Errors
    ///
    /// Refuses zero or a request above [`MAX_TOP`].
    pub fn new(
        requested: usize,
        policy: RankingPolicyV1,
        extrema: Extrema,
    ) -> Result<Self, Refusal> {
        if requested == 0 {
            return Err(Refusal::TopIsZero);
        }
        if requested > MAX_TOP {
            return Err(Refusal::TopAboveMaximum);
        }
        Ok(Self {
            requested,
            policy,
            extrema,
            held: Vec::with_capacity(requested),
            considered: 0,
            admitted: 0,
            refused: 0,
            unmeasured: 0,
        })
    }

    /// Offers one member of the exact same population observed in pass one.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::CandidateOutsidePopulation`] if its measurable value
    /// falls outside the sealed extrema, which detects a mismatched second pass
    /// instead of silently normalising against the wrong run.
    pub fn offer(&mut self, candidate: Candidate) -> Result<(), Refusal> {
        let ranked = if candidate.admitted {
            Some(RankedCandidate {
                score: self.policy.score(&candidate, &self.extrema)?,
                candidate,
            })
        } else {
            None
        };
        self.considered = self.considered.saturating_add(1);
        if candidate.admitted {
            self.admitted = self.admitted.saturating_add(1);
        } else {
            self.refused = self.refused.saturating_add(1);
        }
        if candidate.metrics.has_unmeasured() {
            self.unmeasured = self.unmeasured.saturating_add(1);
        }
        let Some(ranked) = ranked else {
            return Ok(());
        };

        let mut insertion = self.held.len();
        for (index, held) in self.held.iter().enumerate() {
            if ranked > *held {
                insertion = index;
                break;
            }
        }

        if insertion >= self.requested {
            return Ok(());
        }
        if self.held.len() == self.requested {
            let _ = self.held.pop();
        }
        self.held.insert(insertion, ranked);
        Ok(())
    }

    /// Finishes the exact strongest-first list and its reconciliation counts.
    #[must_use]
    pub fn finish(self) -> Selection {
        Selection {
            rows: self.held,
            requested: self.requested,
            considered: self.considered,
            admitted: self.admitted,
            refused: self.refused,
            unmeasured: self.unmeasured,
        }
    }
}

/// Runs both population passes over a resident slice.
///
/// A disk-backed caller uses [`Extrema`] and [`Keeper`] directly; this helper is
/// the small-fixture and already-resident form and is also the reference used by
/// tests.
///
/// # Errors
///
/// Refuses an invalid requested bound or a mismatched second pass.
pub fn select(
    candidates: &[Candidate],
    policy: RankingPolicyV1,
    requested: usize,
) -> Result<Selection, Refusal> {
    if requested == 0 {
        return Err(Refusal::TopIsZero);
    }
    if requested > MAX_TOP {
        return Err(Refusal::TopAboveMaximum);
    }
    if candidates.is_empty() {
        return Ok(Selection {
            rows: Vec::new(),
            requested,
            considered: 0,
            admitted: 0,
            refused: 0,
            unmeasured: 0,
        });
    }
    let mut extrema = Extrema::default();
    let mut first_pass = PopulationPass::new();
    for candidate in candidates {
        extrema.observe(candidate);
        first_pass.observe(candidate);
    }
    let mut keeper = VerifiedKeeper::new(requested, policy, extrema, first_pass.finish())?;
    for candidate in candidates {
        keeper.offer(*candidate)?;
    }
    keeper.finish()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]
mod tests {
    use super::{
        Candidate, Extrema, Keeper, MAX_TOP, Metrics, PopulationPass, RankingPolicyV1, Refusal,
        Selection, StrategyDigest, VerifiedKeeper, Weights, select,
    };
    use costs::fill::Direction;

    fn metrics(value: i64) -> Metrics {
        let magnitude = value.unsigned_abs();
        Metrics {
            drawdown: magnitude.saturating_add(30),
            worst_loss: magnitude.saturating_add(20),
            losing_rate_ppm: magnitude.saturating_mul(1_000).min(1_000_000),
            losing_trades: magnitude,
            loss_ratio_ppm: Some(magnitude.saturating_mul(10_000)),
            pessimistic_profit: value,
            winning_trades: 100_u64.saturating_sub(magnitude.min(100)),
            win_rate_ppm: 1_000_000_u64.saturating_sub(magnitude.saturating_mul(1_000)),
            reward_to_risk_ppm: Some(1_000_000_u64.saturating_add(magnitude)),
            average_win: 100_u64.saturating_add(magnitude),
            average_loss: magnitude,
            assurance_ppm: 900_000_u64.saturating_sub(magnitude.min(900_000)),
        }
    }

    fn candidate(bit: u32, value: i64) -> Candidate {
        let mut words = [0_u64; 6];
        let word = usize::try_from(bit / 64).unwrap_or(0);
        let shift = bit % 64;
        if let Some(slot) = words.get_mut(word) {
            *slot = 1_u64.checked_shl(shift).unwrap_or(0);
        }
        Candidate {
            strategy_digest: StrategyDigest::new([u8::try_from(bit).unwrap_or(u8::MAX); 32]),
            mask_words: words,
            direction: if bit.is_multiple_of(2) {
                Direction::Long
            } else {
                Direction::Short
            },
            admitted: !bit.is_multiple_of(3),
            metrics: metrics(value),
        }
    }

    fn policy() -> RankingPolicyV1 {
        RankingPolicyV1::new(Weights::equal()).expect("equal weights rank")
    }

    fn ranked(candidates: &[Candidate], n: usize) -> Selection {
        select(candidates, policy(), n).expect("fixture is one population")
    }

    #[test]
    fn population_edges_are_exact_and_top_ten_is_one_prefix() {
        for size in [0_usize, 1, 9, 10, 24, 25, 26, 80] {
            let rows: Vec<_> = (0..u32::try_from(size).unwrap_or(0))
                .map(|bit| candidate(bit, i64::from(bit)))
                .collect();
            let selected = ranked(&rows, MAX_TOP);
            assert_eq!(selected.considered, u64::try_from(size).unwrap_or(u64::MAX));
            assert_eq!(
                selected.admitted.saturating_add(selected.refused),
                selected.considered
            );
            assert_eq!(
                selected.rows.len(),
                usize::try_from(selected.admitted)
                    .unwrap_or(usize::MAX)
                    .min(MAX_TOP)
            );
            assert_eq!(
                selected.top_ten().len(),
                usize::try_from(selected.admitted)
                    .unwrap_or(usize::MAX)
                    .min(10)
            );
        }
        assert_eq!(select(&[], policy(), 0), Err(Refusal::TopIsZero));
        assert_eq!(
            select(&[], policy(), MAX_TOP.saturating_add(1)),
            Err(Refusal::TopAboveMaximum)
        );
    }

    #[test]
    fn top_ten_is_exactly_the_prefix_of_the_same_top_twenty_five() {
        let rows: Vec<_> = (0..80_u32)
            .map(|bit| candidate(bit, i64::from(bit).saturating_sub(40)))
            .collect();
        let twenty_five = ranked(&rows, 25);
        let ten = ranked(&rows, 10);
        assert_eq!(twenty_five.top_ten(), ten.rows.as_slice());
    }

    #[test]
    fn arrival_order_and_fixed_chunks_cannot_move_a_tie() {
        let tied = metrics(0);
        let ascending: Vec<_> = (0..20_u32)
            .map(|bit| Candidate {
                metrics: tied,
                admitted: true,
                ..candidate(bit, 0)
            })
            .collect();
        let mut descending = ascending.clone();
        descending.reverse();
        let first = ranked(&ascending, 10);
        let second = ranked(&descending, 10);
        assert_eq!(first.rows, second.rows);
        assert!(first.rows.windows(2).all(|pair| {
            pair.first()
                .zip(pair.get(1))
                .is_none_or(|(a, b)| a.candidate.mask_words <= b.candidate.mask_words)
        }));
    }

    #[test]
    fn equal_extrema_and_undefined_ratios_are_neutral_and_counted() {
        let mut one = candidate(1, 0);
        one.metrics.loss_ratio_ppm = None;
        one.metrics.reward_to_risk_ppm = None;
        let two = Candidate {
            mask_words: candidate(2, 0).mask_words,
            ..one
        };
        let selected = ranked(&[one, two], 2);
        assert_eq!(selected.unmeasured, 2);
        assert!(selected.rows.iter().all(|row| row.score == 500_000));
    }

    #[test]
    fn institutional_refusal_excludes_even_a_better_raw_score() {
        let admitted = Candidate {
            admitted: true,
            ..candidate(1, -1_000)
        };
        let refused = Candidate {
            admitted: false,
            ..candidate(2, 1_000)
        };
        let selected = ranked(&[refused, admitted], 2);
        assert_eq!(selected.rows.len(), 1);
        assert_eq!(selected.admitted, 1);
        assert_eq!(selected.refused, 1);
        assert_eq!(
            selected.rows.first().map(|row| row.candidate),
            Some(admitted)
        );
    }

    #[test]
    fn refused_extremes_cannot_distort_admitted_scores() {
        let admitted_low = Candidate {
            admitted: true,
            ..candidate(1, -10)
        };
        let admitted_high = Candidate {
            admitted: true,
            ..candidate(2, 10)
        };
        let refused_extreme = Candidate {
            admitted: false,
            metrics: Metrics {
                drawdown: u64::MAX,
                worst_loss: u64::MAX,
                losing_rate_ppm: u64::MAX,
                losing_trades: u64::MAX,
                loss_ratio_ppm: Some(u64::MAX),
                pessimistic_profit: i64::MAX,
                winning_trades: u64::MAX,
                win_rate_ppm: u64::MAX,
                reward_to_risk_ppm: Some(u64::MAX),
                average_win: u64::MAX,
                average_loss: u64::MAX,
                assurance_ppm: u64::MAX,
            },
            ..candidate(3, 0)
        };
        let admitted_only = ranked(&[admitted_low, admitted_high], 2);
        let with_refused = ranked(&[admitted_low, refused_extreme, admitted_high], 2);
        assert_eq!(with_refused.rows, admitted_only.rows);
        assert_eq!(with_refused.refused, 1);
    }

    #[test]
    fn runtime_weights_change_the_order_without_changing_the_population() {
        let profit = Candidate {
            metrics: Metrics {
                pessimistic_profit: 10_000,
                drawdown: 1_000,
                ..metrics(0)
            },
            ..candidate(1, 0)
        };
        let calm = Candidate {
            metrics: Metrics {
                pessimistic_profit: 100,
                drawdown: 1,
                ..metrics(0)
            },
            ..candidate(2, 0)
        };
        let profit_policy = RankingPolicyV1::new(Weights {
            pessimistic_profit: 1,
            drawdown: 0,
            worst_loss: 0,
            losing_rate: 0,
            losing_trades: 0,
            loss_ratio: 0,
            winning_trades: 0,
            win_rate: 0,
            reward_to_risk: 0,
            average_win: 0,
            average_loss: 0,
        })
        .expect("one live weight");
        let calm_policy = RankingPolicyV1::new(Weights {
            drawdown: 1,
            pessimistic_profit: 0,
            worst_loss: 0,
            losing_rate: 0,
            losing_trades: 0,
            loss_ratio: 0,
            winning_trades: 0,
            win_rate: 0,
            reward_to_risk: 0,
            average_win: 0,
            average_loss: 0,
        })
        .expect("one live weight");
        let rows = [profit, calm];
        assert_eq!(
            select(&rows, profit_policy, 2)
                .expect("rank")
                .rows
                .first()
                .map(|row| row.candidate.mask_words),
            Some(profit.mask_words)
        );
        assert_eq!(
            select(&rows, calm_policy, 2)
                .expect("rank")
                .rows
                .first()
                .map(|row| row.candidate.mask_words),
            Some(calm.mask_words)
        );
        assert_eq!(
            RankingPolicyV1::new(Weights {
                drawdown: 0,
                worst_loss: 0,
                losing_rate: 0,
                losing_trades: 0,
                loss_ratio: 0,
                pessimistic_profit: 0,
                winning_trades: 0,
                win_rate: 0,
                reward_to_risk: 0,
                average_win: 0,
                average_loss: 0,
            }),
            Err(Refusal::EveryWeightIsZero)
        );
    }

    #[test]
    fn extreme_integer_domains_do_not_wrap_or_escape_the_score_scale() {
        let low = Candidate {
            metrics: Metrics {
                drawdown: u64::MAX,
                worst_loss: u64::MAX,
                losing_rate_ppm: u64::MAX,
                losing_trades: u64::MAX,
                loss_ratio_ppm: Some(u64::MAX),
                pessimistic_profit: i64::MIN,
                winning_trades: 0,
                win_rate_ppm: 0,
                reward_to_risk_ppm: Some(0),
                average_win: 0,
                average_loss: u64::MAX,
                assurance_ppm: 0,
            },
            ..candidate(1, 0)
        };
        let high = Candidate {
            metrics: Metrics {
                drawdown: 0,
                worst_loss: 0,
                losing_rate_ppm: 0,
                losing_trades: 0,
                loss_ratio_ppm: Some(0),
                pessimistic_profit: i64::MAX,
                winning_trades: u64::MAX,
                win_rate_ppm: u64::MAX,
                reward_to_risk_ppm: Some(u64::MAX),
                average_win: u64::MAX,
                average_loss: 0,
                assurance_ppm: u64::MAX,
            },
            ..candidate(2, 0)
        };
        let selected = ranked(&[low, high], 2);
        assert!(selected.rows.iter().all(|row| row.score <= 1_000_000));
    }

    #[test]
    fn a_mismatched_second_pass_is_refused_instead_of_misnormalised() {
        let first = candidate(1, 10);
        let outside = candidate(2, 11);
        let mut extrema = Extrema::default();
        extrema.observe(&first);
        let mut keeper = Keeper::new(1, policy(), extrema).expect("bound");
        assert_eq!(
            keeper.offer(outside),
            Err(Refusal::CandidateOutsidePopulation)
        );
    }

    #[test]
    fn a_second_pass_omission_duplicate_replacement_or_reorder_never_seals() {
        let population = [candidate(1, -1), candidate(2, 0), candidate(3, 1)];
        let mut extrema = Extrema::default();
        let mut first = PopulationPass::new();
        for row in &population {
            extrema.observe(row);
            first.observe(row);
        }
        let proof = first.finish();

        let run = |rows: &[Candidate]| {
            let mut keeper = VerifiedKeeper::new(3, policy(), extrema, proof).expect("policy");
            for row in rows {
                keeper.offer(*row).expect("inside extrema");
            }
            keeper.finish()
        };

        assert!(run(&population).is_ok(), "the exact replay must seal");
        assert_eq!(
            run(&population[..2]),
            Err(Refusal::PopulationMismatch),
            "an omitted tail kept plausible extrema but must not seal"
        );
        assert_eq!(
            run(&[population[0], population[1], population[1]]),
            Err(Refusal::PopulationMismatch),
            "a duplicate replacing one row kept the count but must not seal"
        );
        let replacement = Candidate {
            strategy_digest: StrategyDigest::new([99; 32]),
            ..population[2]
        };
        assert_eq!(
            run(&[population[0], population[1], replacement]),
            Err(Refusal::PopulationMismatch),
            "an in-range replacement must be visible even when extrema agree"
        );
        assert_eq!(
            run(&[population[1], population[0], population[2]]),
            Err(Refusal::PopulationMismatch),
            "canonical population order is part of the proof"
        );
    }

    #[test]
    fn complete_strategy_digest_breaks_equal_mask_direction_exit_cell_ties() {
        let first = Candidate {
            admitted: true,
            strategy_digest: StrategyDigest::new([1; 32]),
            metrics: metrics(0),
            ..candidate(2, 0)
        };
        let second = Candidate {
            strategy_digest: StrategyDigest::new([2; 32]),
            ..first
        };
        let selected = ranked(&[second, first], 2);
        assert_eq!(
            selected
                .rows
                .iter()
                .map(|row| row.candidate.strategy_digest.bytes())
                .collect::<Vec<_>>(),
            vec![[1; 32], [2; 32]],
            "two semantic exit cells may never collapse behind mask+direction"
        );
    }

    #[test]
    fn bounded_keeper_matches_a_full_sort_for_every_small_arrival_permutation() {
        let base = [
            candidate(0, -3),
            candidate(1, 7),
            candidate(2, 2),
            candidate(3, 2),
        ];
        let reference = ranked(&base, 3).rows;
        for a in 0..4 {
            for b in 0..4 {
                if b == a {
                    continue;
                }
                for c in 0..4 {
                    if c == a || c == b {
                        continue;
                    }
                    for d in 0..4 {
                        if d == a || d == b || d == c {
                            continue;
                        }
                        let order = [
                            *base.get(a).expect("a"),
                            *base.get(b).expect("b"),
                            *base.get(c).expect("c"),
                            *base.get(d).expect("d"),
                        ];
                        assert_eq!(ranked(&order, 3).rows, reference);
                    }
                }
            }
        }
    }

    #[test]
    fn ranking_policy_bytes_are_versioned_fixed_and_layout_independent() {
        let bytes = policy().canonical_bytes();
        assert_eq!(bytes.len(), super::RANKING_POLICY_CANONICAL_LEN_V1);
        assert_eq!(&bytes[..16], b"brutex-topn-v1\0\0");
        assert_eq!(&bytes[16..20], &1_u32.to_le_bytes());
        assert_eq!(&bytes[20..24], &52_u32.to_le_bytes());
        assert_eq!(&bytes[24..28], &11_u32.to_le_bytes());
        assert_eq!(&bytes[28..32], &[0_u8; 4]);
        for encoded in bytes[32..].chunks_exact(4) {
            assert_eq!(encoded, &1_u32.to_le_bytes());
        }
        assert_eq!(policy().digest(), brutex_core::blake3::hash(&bytes));
    }

    #[test]
    fn every_runtime_weight_changes_the_canonical_policy_identity() {
        let variants = [
            Weights {
                drawdown: 2,
                ..Weights::equal()
            },
            Weights {
                worst_loss: 2,
                ..Weights::equal()
            },
            Weights {
                losing_rate: 2,
                ..Weights::equal()
            },
            Weights {
                losing_trades: 2,
                ..Weights::equal()
            },
            Weights {
                loss_ratio: 2,
                ..Weights::equal()
            },
            Weights {
                pessimistic_profit: 2,
                ..Weights::equal()
            },
            Weights {
                winning_trades: 2,
                ..Weights::equal()
            },
            Weights {
                win_rate: 2,
                ..Weights::equal()
            },
            Weights {
                reward_to_risk: 2,
                ..Weights::equal()
            },
            Weights {
                average_win: 2,
                ..Weights::equal()
            },
            Weights {
                average_loss: 2,
                ..Weights::equal()
            },
        ];
        let base = policy().digest();
        let mut seen = std::collections::HashSet::new();
        for weights in variants {
            let digest = RankingPolicyV1::new(weights)
                .expect("one changed weight")
                .digest();
            assert_ne!(digest, base);
            assert!(seen.insert(digest), "each criterion occupies its own slot");
        }
    }
}
