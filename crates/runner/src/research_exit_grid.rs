//! Additive exact TRAINING resolution for the approved runtime research scope.
//!
//! This type shares the legacy level resolver and pricing policy. Its family
//! and hash domain are separate: cash never becomes a legacy index or a V6
//! statistical-admission record. Input membership is a present snapshot.

use super::{
    ExecutionSeriesV1, ExitGridErrorV1, ExitGridPolicyV1, Hasher, Ladder, Ppm, RatioPairV1,
    ResolvedGridLevelsV1, ResolvedLaddersV1, hash, printed_ohlcv_cost_model_id_v1, put_i64,
    put_levels, put_u64, put_usize,
};
use crate::research_family::ResearchFamilyV1;
use brutex_core::instrument::InstrumentKey;

/// Exact resolved grid for an index or eligible cash family, in its own domain.
///
/// No public constructor accepts levels or digests. Only the shared policy
/// resolver over the named TRAINING series can construct this value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchResolvedExitGridV1 {
    pub(super) family: ResearchFamilyV1,
    pub(super) instrument: InstrumentKey,
    pub(super) feed_digest: [u8; 32],
    pub(super) commit_digest: [u8; 32],
    pub(super) calendar_digest: [u8; 32],
    pub(super) policy: ExitGridPolicyV1,
    pub(super) policy_digest: [u8; 32],
    pub(super) training_digest: [u8; 32],
    pub(super) training_bars: u64,
    pub(super) training_first_ts_micros: i64,
    pub(super) training_last_ts_micros: i64,
    pub(super) levels: ResolvedGridLevelsV1,
    pub(super) digest: [u8; 32],
}

impl ExitGridPolicyV1 {
    /// Resolve an explicitly eligible cash/index family through the same
    /// TRAINING samples, exact percentiles, ratios and resource checks as V1.
    ///
    /// # Errors
    /// Refuses unsupported family keys and every ordinary V1 resolution error.
    /// This proves execution-grid scope, not institutional statistical approval.
    pub fn resolve_research_attested(
        &self,
        series: ExecutionSeriesV1<'_>,
    ) -> Result<ResearchResolvedExitGridV1, ExitGridErrorV1> {
        let instrument = *series.instrument();
        let family = ResearchFamilyV1::new(instrument)
            .map_err(|_| ExitGridErrorV1::UnsupportedInstrument)?;
        let levels = self.resolve_levels(series.bars())?;
        let training_bars = u64::try_from(series.bars().len())
            .map_err(|_| ExitGridErrorV1::ArithmeticOverflow("training bar count"))?;
        let first = series
            .bars()
            .first()
            .ok_or(ExitGridErrorV1::EmptyExecutionSeries)?
            .ts_micros;
        let last = series
            .bars()
            .last()
            .ok_or(ExitGridErrorV1::EmptyExecutionSeries)?
            .ts_micros;
        let mut resolved = ResearchResolvedExitGridV1 {
            family,
            instrument,
            feed_digest: hash(series.feed().as_bytes()),
            commit_digest: hash(series.commit().as_bytes()),
            calendar_digest: series.calendar_digest(),
            policy: self.clone(),
            policy_digest: self.digest(),
            training_digest: crate::identity::data_digest(series.bars()),
            training_bars,
            training_first_ts_micros: first,
            training_last_ts_micros: last,
            levels,
            digest: [0; 32],
        };
        resolved.digest = digest_resolved(&resolved);
        Ok(resolved)
    }
}

impl ResearchResolvedExitGridV1 {
    /// Full eligible family capability, including the cash membership snapshot.
    #[must_use]
    pub const fn family(&self) -> ResearchFamilyV1 {
        self.family
    }
    /// Complete canonical instrument key.
    #[must_use]
    pub const fn instrument(&self) -> InstrumentKey {
        self.instrument
    }
    /// Exact source feed identity.
    #[must_use]
    pub const fn feed_digest(&self) -> [u8; 32] {
        self.feed_digest
    }
    /// Exact source build identity.
    #[must_use]
    pub const fn commit_digest(&self) -> [u8; 32] {
        self.commit_digest
    }
    /// Exact source calendar identity.
    #[must_use]
    pub const fn calendar_digest(&self) -> [u8; 32] {
        self.calendar_digest
    }
    /// Frozen explicit execution policy.
    #[must_use]
    pub const fn policy(&self) -> &ExitGridPolicyV1 {
        &self.policy
    }
    /// Digest of every execution-policy field.
    #[must_use]
    pub const fn policy_digest(&self) -> [u8; 32] {
        self.policy_digest
    }
    /// Exact TRAINING execution bytes.
    #[must_use]
    pub const fn training_digest(&self) -> [u8; 32] {
        self.training_digest
    }
    /// Actual count of TRAINING execution rows.
    #[must_use]
    pub const fn training_bars(&self) -> u64 {
        self.training_bars
    }
    /// First actual TRAINING timestamp.
    #[must_use]
    pub const fn training_first_ts_micros(&self) -> i64 {
        self.training_first_ts_micros
    }
    /// Last actual TRAINING timestamp.
    #[must_use]
    pub const fn training_last_ts_micros(&self) -> i64 {
        self.training_last_ts_micros
    }
    /// Side whose adverse and favorable ranges resolved this grid.
    #[must_use]
    pub const fn side(&self) -> crate::excursion::Side {
        self.policy.side()
    }
    /// Exact observed stop ladder.
    #[must_use]
    pub fn stop_levels_ppm(&self) -> &[Ppm] {
        &self.levels.stops
    }
    /// Exact observed target ladder.
    #[must_use]
    pub fn target_levels_ppm(&self) -> &[Ppm] {
        &self.levels.targets
    }
    /// Exact observed trailing ladder.
    #[must_use]
    pub fn trail_levels_ppm(&self) -> &[Ppm] {
        &self.levels.trails
    }
    /// Exact policy-admitted stop/target coordinate pairs.
    #[must_use]
    pub fn ratio_pairs(&self) -> &[RatioPairV1] {
        &self.levels.ratio_pairs
    }
    /// Exact forced-stop coordinate, when the explicit policy requires it.
    #[must_use]
    pub const fn forced_stop_index(&self) -> Option<usize> {
        self.levels.forced_stop_index
    }
    /// Complete expected cell population, including zero-trade coordinates.
    #[must_use]
    pub const fn cell_count(&self) -> u64 {
        self.levels.cell_count
    }
    /// New domain identity, never an alias of an old index resolution.
    #[must_use]
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// Rebuild exact ladder wrappers without rereading market inputs.
    ///
    /// # Errors
    /// Refuses invalid resolved ladders or a changed resolution identity.
    pub fn ladders(&self) -> Result<ResolvedLaddersV1, ExitGridErrorV1> {
        self.require_runtime_integrity()?;
        Ok(ResolvedLaddersV1 {
            stops: Ladder::new(self.levels.stops.clone())
                .ok_or(ExitGridErrorV1::InvalidResolvedLadder("stop"))?,
            targets: Ladder::new(self.levels.targets.clone())
                .ok_or(ExitGridErrorV1::InvalidResolvedLadder("target"))?,
            trails: Ladder::new(self.levels.trails.clone())
                .ok_or(ExitGridErrorV1::InvalidResolvedLadder("trail"))?,
        })
    }

    /// Recompute complete resolution identity and family consistency.
    #[must_use]
    pub fn digest_is_valid(&self) -> bool {
        self.family.instrument() == self.instrument
            && self.policy_digest == self.policy.digest()
            && self.digest == digest_resolved(self)
    }

    pub(super) fn require_runtime_integrity(&self) -> Result<(), ExitGridErrorV1> {
        if !self.digest_is_valid() {
            return Err(ExitGridErrorV1::ResolutionDigestMismatch);
        }
        if self.policy.cost_model_id() != printed_ohlcv_cost_model_id_v1() {
            return Err(ExitGridErrorV1::UnsupportedCostModelId);
        }
        Ok(())
    }
}

fn digest_resolved(value: &ResearchResolvedExitGridV1) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(b"brutex.runner.research-resolved-exit-grid.v1\0");
    hash.update(&value.family.encode());
    hash.update(&super::instrument_digest_v1(&value.instrument));
    hash.update(&value.feed_digest);
    hash.update(&value.commit_digest);
    hash.update(&value.calendar_digest);
    hash.update(&value.policy.digest());
    hash.update(&value.policy_digest);
    hash.update(&value.training_digest);
    put_u64(&mut hash, value.training_bars);
    put_i64(&mut hash, value.training_first_ts_micros);
    put_i64(&mut hash, value.training_last_ts_micros);
    put_levels(&mut hash, &value.levels.stops);
    put_levels(&mut hash, &value.levels.targets);
    put_levels(&mut hash, &value.levels.trails);
    put_usize(&mut hash, value.levels.ratio_pairs.len());
    for pair in &value.levels.ratio_pairs {
        put_usize(&mut hash, pair.stop_index());
        put_usize(&mut hash, pair.target_index());
    }
    put_usize(&mut hash, value.levels.ratio_bitmap.len());
    for admitted in &value.levels.ratio_bitmap {
        hash.update(&[u8::from(*admitted)]);
    }
    match value.levels.forced_stop_index {
        None => hash.update(&[0]),
        Some(index) => {
            hash.update(&[1]);
            put_usize(&mut hash, index);
        }
    }
    put_u64(&mut hash, value.levels.cell_count);
    hash.finalize()
}

#[cfg(test)]
#[path = "research_exit_grid_tests.rs"]
mod tests;
