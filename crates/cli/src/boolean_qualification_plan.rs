//! One predeclared catalog over selected rungs, before later-price evaluation.
//! Eight statistical allocation units remain reserved; omitted units are unspent.
//!
//! A plan is neither statistical completion nor source authority. Its producer
//! requires both live strict preparations; callers retain them through terminal
//! publication. A detached decoder cannot create that producer value. Failed,
//! paused and zero-return units keep their original allocation. Separate plans
//! do not claim a shared grammar-wide budget or permit fresh alpha on retry.
use crate::boolean_campaign::{Prepared, RungScope};
use crate::population_statistics_v2::PopulationStatisticsProcedureV2;
use brutex_core::blake3::Hasher;
use pull::session::Day;
use runner::bootstrap_zero_v2::Bounds;
use runner::exit_grid_policy::expression_execution::later_period::validation::LaterSessionWindowV1;
use runner::family_allocation_v1::{self as allocation, Allocation};

const HEADER: usize = 1024;
const WINDOW: usize = 16;
const MAGIC: &[u8; 8] = b"BRBQPL01";

#[derive(Clone, Debug, PartialEq, Eq)]
struct Facts {
    rungs: RungScope,
    index_policy: Option<[u8; 32]>,
    // Exact all-eight identities/descriptors already bind every source receipt.
    identities: [[u8; 32]; 8],
    catalogs: [[u8; 32]; 8],
    sources: [[u8; 32]; 8],
    count: u64,
    horizon: u64,
    training: [u64; 2],
    later: [u64; 2],
    procedure: PopulationStatisticsProcedureV2,
    limits: [u64; 2],
    minimums: [u64; 2],
    alphas: [u64; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Description {
    facts: Facts,
    windows: Vec<LaterSessionWindowV1>,
    descriptor: [u8; 32],
    units: [[u8; 32]; 8],
}

/// Created only from the two live, full-scope strict preparations, before work.
pub(crate) struct Plan {
    description: Description,
}
#[cfg(test)]
pub(crate) struct GeneratedBindings<'a> {
    pub(crate) families: &'a [runner::research_family::ResearchFamilyV1],
    pub(crate) catalogs: [[u8; 32]; 8],
    pub(crate) sources: [[u8; 32]; 8],
}
pub(crate) fn ordered_identities(ids: impl ExactSizeIterator<Item = [u8; 32]>) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(b"brutex-boolean-qualification-ordered-source-identities-v1\0");
    hash.update(&(ids.len() as u64).to_le_bytes());
    for identity in ids {
        hash.update(&identity);
    }
    hash.finalize()
}
impl Plan {
    pub(crate) const fn index_policy(&self) -> Option<[u8; 32]> {
        self.description.facts.index_policy
    }
    #[cfg(test)]
    pub(crate) fn with_index_consistency(mut self) -> Result<Self, String> {
        self.description.facts.index_policy = Some(crate::index_consistency::Policy::V1.digest());
        self.description = Description::new(self.description.facts)?;
        Ok(self)
    }
    pub(crate) const fn rungs(&self) -> RungScope {
        self.description.facts.rungs
    }
    pub(crate) const fn scope_digest(&self) -> [u8; 32] {
        self.description.facts.identities[5]
    }
    pub(crate) const fn policy_digest(&self) -> [u8; 32] {
        self.description.facts.identities[6]
    }
    #[cfg(test)]
    pub(crate) fn generated_fixture(
        policy: &runner::admission::AdmissionPolicyV1,
        procedure: PopulationStatisticsProcedureV2,
        training: ((u16, u8), (u16, u8)),
        later: ((u16, u8), (u16, u8)),
        programs: &[runner::expression::Expression],
        limits: [u64; 2],
        bindings: &GeneratedBindings<'_>,
    ) -> Result<Self, String> {
        let values = policy.values();
        let mut identities = std::array::from_fn(|index| {
            brutex_core::blake3::hash(format!("generated qualification source {index}").as_bytes())
        });
        identities[4] = crate::boolean_campaign::program_digest(programs);
        let keys: Vec<_> = bindings
            .families
            .iter()
            .copied()
            .map(runner::research_family::ResearchFamilyV1::instrument)
            .collect();
        identities[5] =
            runner::research_family::ResearchScopeV1::new(&keys, bindings.families.len())
                .map_err(display)?
                .digest();
        identities[6] = policy.digest();
        let facts = Facts {
            rungs: RungScope::ALL,
            index_policy: None,
            identities,
            catalogs: bindings.catalogs,
            sources: bindings.sources,
            count: programs.len() as u64,
            horizon: 5,
            training: month_days(training)?,
            later: month_days(later)?,
            procedure,
            limits,
            minimums: [values.min_decided_folds, values.min_profitable_oos_folds],
            alphas: [
                values.max_fwer_p_value_ppm,
                values.max_romano_wolf_p_value_ppm,
            ],
        };
        Ok(Self {
            description: Description::new(facts)?,
        })
    }
    pub(crate) fn new(training: &Prepared, later: &Prepared) -> Result<Self, String> {
        training.require_current()?;
        later.require_current()?;
        require_matching(training, later)?;
        let policy = training.policy().values();
        let (max_bytes, max_records) = training.source_limits();
        let mut catalogs = [[0; 32]; 8];
        let mut sources = [[0; 32]; 8];
        for (index, (catalog, source)) in catalogs.iter_mut().zip(&mut sources).enumerate() {
            if training.rungs().contains(index) {
                *catalog = training.expectedcatalog_digest(index)?;
                *source = later.expectedsource_digest(index)?;
            }
        }
        let facts = Facts {
            rungs: training.rungs(),
            index_policy: Some(crate::index_consistency::Policy::V1.digest()),
            identities: [
                training.identity(),
                training.descriptor_digest(),
                later.identity(),
                later.descriptor_digest(),
                training.program_digest(),
                training.scope_digest(),
                training.policy().digest(),
                training.preparation_policy_digest(),
            ],
            catalogs,
            sources,
            count: training.program_count(),
            horizon: u64::from(training.horizon()),
            training: month_days(training.span())?,
            later: month_days(later.span())?,
            procedure: training.procedure(),
            limits: [max_bytes, max_records],
            minimums: [policy.min_decided_folds, policy.min_profitable_oos_folds],
            alphas: [
                policy.max_fwer_p_value_ppm,
                policy.max_romano_wolf_p_value_ppm,
            ],
        };
        let description = Description::new(facts)?;
        training.require_current()?;
        later.require_current()?;
        Ok(Self { description })
    }
    pub(crate) const fn descriptor(&self) -> [u8; 32] {
        self.description.descriptor
    }
    pub(crate) fn expected_catalogs(&self, rung: usize) -> Result<[u8; 32], String> {
        self.description.require_selected(rung)?;
        self.description
            .facts
            .catalogs
            .get(rung)
            .copied()
            .ok_or_else(|| "planned original rung absent".into())
    }
    pub(crate) fn expected_sources(&self, rung: usize) -> Result<[u8; 32], String> {
        self.description.require_selected(rung)?;
        self.description
            .facts
            .sources
            .get(rung)
            .copied()
            .ok_or_else(|| "planned later rung absent".into())
    }
    pub(crate) const fn units(&self) -> &[[u8; 32]; 8] {
        &self.description.units
    }
    pub(crate) fn requested(&self) -> Result<LaterSessionWindowV1, String> {
        self.description.requested()
    }
    pub(crate) fn training_requested(&self) -> Result<LaterSessionWindowV1, String> {
        LaterSessionWindowV1::new(
            i64::try_from(self.description.facts.training[0]).map_err(display)?,
            i64::try_from(self.description.facts.training[1]).map_err(display)?,
        )
    }
    pub(crate) fn windows(&self) -> &[LaterSessionWindowV1] {
        &self.description.windows
    }
    pub(crate) fn allocation(&self, rung: usize) -> Result<Allocation, String> {
        self.description.allocation(rung)
    }
    pub(crate) fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        self.description.encode()
    }
    pub(crate) fn numerical_bounds(&self) -> Result<Bounds, String> {
        self.description.facts.numerical_bounds()
    }
    pub(crate) const fn procedure(&self) -> PopulationStatisticsProcedureV2 {
        self.description.facts.procedure
    }
    pub(crate) const fn source_limits(&self) -> [u64; 2] {
        self.description.facts.limits
    }
    /// Low draw resolution is visible, never silently increased or a fake rejection.
    pub(crate) fn resolution_reachable(&self) -> Result<bool, String> {
        self.description.resolution_reachable()
    }
}

/// Detached fixed-format observation; cannot supply a live `Plan` to a producer.
pub(crate) struct ObservedPlan {
    description: Description,
}
impl ObservedPlan {
    pub(crate) const fn index_policy(&self) -> Option<[u8; 32]> {
        self.description.facts.index_policy
    }
    pub(crate) const fn rungs(&self) -> RungScope {
        self.description.facts.rungs
    }
    pub(crate) const fn source_descriptors(&self) -> [[u8; 32]; 2] {
        [
            self.description.facts.identities[1],
            self.description.facts.identities[3],
        ]
    }
    pub(crate) fn expected_catalogs(&self, rung: usize) -> Result<[u8; 32], String> {
        self.description.require_selected(rung)?;
        self.description
            .facts
            .catalogs
            .get(rung)
            .copied()
            .ok_or_else(|| "planned original rung absent".into())
    }
    pub(crate) fn expected_sources(&self, rung: usize) -> Result<[u8; 32], String> {
        self.description.require_selected(rung)?;
        self.description
            .facts
            .sources
            .get(rung)
            .copied()
            .ok_or_else(|| "planned later rung absent".into())
    }
    pub(crate) const fn scope_digest(&self) -> [u8; 32] {
        self.description.facts.identities[5]
    }
    pub(crate) const fn policy_digest(&self) -> [u8; 32] {
        self.description.facts.identities[6]
    }
    pub(crate) const fn program_digest(&self) -> [u8; 32] {
        self.description.facts.identities[4]
    }
    pub(crate) const fn program_count(&self) -> u64 {
        self.description.facts.count
    }
    pub(crate) const fn horizon(&self) -> u64 {
        self.description.facts.horizon
    }
    pub(crate) const fn minimum_folds(&self) -> [u64; 2] {
        self.description.facts.minimums
    }
    pub(crate) const fn alpha_ceilings(&self) -> [u64; 2] {
        self.description.facts.alphas
    }
    pub(crate) fn training_requested(&self) -> Result<LaterSessionWindowV1, String> {
        LaterSessionWindowV1::new(
            i64::try_from(self.description.facts.training[0]).map_err(display)?,
            i64::try_from(self.description.facts.training[1]).map_err(display)?,
        )
    }
    pub(crate) fn decode(bytes: &[u8], max_bytes: u64, max_folds: u64) -> Result<Self, String> {
        if bytes.len() < HEADER || bytes.len() as u64 > max_bytes {
            return Err("qualification plan exceeds header/byte admission".into());
        }
        let mut reader = Decoder { bytes, at: 0 };
        let version = match &reader.take::<8>()? {
            b"BRBQPL01" => 1,
            b"BRBQPL02" => 2,
            b"BRBQPL03" => 3,
            _ => return Err("qualification plan version differs".into()),
        };
        if reader.u64()? != version || reader.u64()? != 1 || reader.u64()? != 8 {
            return Err("qualification plan version or finite scope differs".into());
        }
        let mut identities = [[0; 32]; 8];
        for identity in &mut identities {
            *identity = reader.take()?;
        }
        let mut catalogs = [[0; 32]; 8];
        let mut sources = [[0; 32]; 8];
        for identity in catalogs.iter_mut().chain(&mut sources) {
            *identity = reader.take()?;
        }
        let count = reader.u64()?;
        let horizon = reader.u64()?;
        let training = [reader.u64()?, reader.u64()?];
        let later = [reader.u64()?, reader.u64()?];
        let procedure =
            PopulationStatisticsProcedureV2::new(reader.u64()?, reader.u64()?, reader.u64()?)?;
        let limits = [reader.u64()?, reader.u64()?];
        let minimums = [reader.u64()?, reader.u64()?];
        let alphas = [reader.u64()?, reader.u64()?];
        let folds = reader.u64()?;
        let work = reader.u64()?;
        let memory = reader.u64()?;
        let mut index_policy = None;
        let rungs = if version == 3 {
            let rungs = RungScope::from_mask(u8::try_from(reader.u64()?).map_err(display)?)?;
            let policy = reader.take::<32>()?;
            if policy != crate::index_consistency::Policy::V1.digest()
                || reader.take::<40>()? != [0; 40]
            {
                return Err("qualification index policy or padding differs".into());
            }
            index_policy = Some(policy);
            rungs
        } else if version == 2 {
            let rungs = RungScope::from_mask(u8::try_from(reader.u64()?).map_err(display)?)?;
            if rungs == RungScope::ALL || reader.take::<72>()? != [0; 72] {
                return Err("scoped qualification plan selection/padding differs".into());
            }
            rungs
        } else {
            if reader.take::<80>()? != [0; 80] {
                return Err("qualification plan padding differs".into());
            }
            RungScope::ALL
        };
        if folds > max_folds || wire_size(folds)? != bytes.len() {
            return Err("qualification plan count/length/padding differs".into());
        }
        let facts = Facts {
            rungs,
            index_policy,
            identities,
            catalogs,
            sources,
            count,
            horizon,
            training,
            later,
            procedure,
            limits,
            minimums,
            alphas,
        };
        facts.admit(max_bytes)?;
        if facts.folds() != folds
            || facts.numerical_bounds()?
                != (Bounds {
                    max_work: work,
                    max_bytes: memory,
                })
        {
            return Err("qualification plan derived numerical admission differs".into());
        }
        let description = Description::new(facts)?;
        for expected in &description.windows {
            if reader.u64()? != u64::try_from(expected.first_day()).map_err(display)?
                || reader.u64()? != u64::try_from(expected.last_day()).map_err(display)?
            {
                return Err("qualification plan civil partition differs".into());
            }
        }
        Ok(Self { description })
    }
    pub(crate) const fn descriptor(&self) -> [u8; 32] {
        self.description.descriptor
    }
    pub(crate) const fn units(&self) -> &[[u8; 32]; 8] {
        &self.description.units
    }
    pub(crate) fn requested(&self) -> Result<LaterSessionWindowV1, String> {
        self.description.requested()
    }
    pub(crate) fn windows(&self) -> &[LaterSessionWindowV1] {
        &self.description.windows
    }
    pub(crate) fn allocation(&self, rung: usize) -> Result<Allocation, String> {
        self.description.allocation(rung)
    }
    pub(crate) fn numerical_bounds(&self) -> Result<Bounds, String> {
        self.description.facts.numerical_bounds()
    }
    pub(crate) const fn procedure(&self) -> PopulationStatisticsProcedureV2 {
        self.description.facts.procedure
    }
    pub(crate) const fn source_limits(&self) -> [u64; 2] {
        self.description.facts.limits
    }
    pub(crate) fn resolution_reachable(&self) -> Result<bool, String> {
        self.description.resolution_reachable()
    }
}

fn require_matching(training: &Prepared, later: &Prepared) -> Result<(), String> {
    if training.rungs() != later.rungs()
        || training.program_digest() != later.program_digest()
        || training.program_count() != later.program_count()
        || training.scope_digest() != later.scope_digest()
        || training.horizon() != later.horizon()
        || training.policy() != later.policy()
        || training.procedure() != later.procedure()
        || training.source_limits() != later.source_limits()
        || training.preparation_policy_digest() != later.preparation_policy_digest()
    {
        return Err(
            "qualification training/later catalog, scope, horizon or resolved policy differs"
                .into(),
        );
    }
    Ok(())
}
fn month_days(span: ((u16, u8), (u16, u8))) -> Result<[u64; 2], String> {
    let ((fy, fm), (ty, tm)) = span;
    let first = Day::new(fy, fm, 1).map_err(display)?;
    let last = Day::new(ty, tm, 1).map_err(display)?.end_of_month();
    if first > last {
        return Err("qualification month span is reversed".into());
    }
    Ok([
        u64::from(first.days_from_epoch()),
        u64::from(last.days_from_epoch()),
    ])
}
impl Facts {
    fn folds(&self) -> u64 {
        self.minimums[0].max(self.minimums[1])
    }
    fn alpha(&self) -> u64 {
        self.alphas[0].min(self.alphas[1])
    }
    fn numerical_bounds(&self) -> Result<Bounds, String> {
        Ok(Bounds {
            max_work: self.limits[1]
                .checked_mul(
                    self.procedure
                        .draws()
                        .checked_add(1)
                        .ok_or("qualification draw denominator overflow")?,
                )
                .ok_or("qualification work overflow")?,
            max_bytes: self.limits[0],
        })
    }
    fn admit(&self, max_bytes: u64) -> Result<(), String> {
        for (index, (catalog, source)) in self.catalogs.iter().zip(&self.sources).enumerate() {
            if !self.rungs.contains(index) && (*catalog != [0; 32] || *source != [0; 32]) {
                return Err("unselected qualification timeframe carries source evidence".into());
            }
        }
        for [first, last] in [self.training, self.later] {
            let first = Day::from_days(u32::try_from(first).map_err(display)?).map_err(display)?;
            let last = Day::from_days(u32::try_from(last).map_err(display)?).map_err(display)?;
            if first.day() != 1 || last != last.end_of_month() {
                return Err("qualification source spans require complete declared months".into());
            }
        }
        let [start, end] = self.later;
        if self.count == 0
            || self.horizon == 0
            || self.horizon > u64::from(u32::MAX)
            || self.training[0] > self.training[1]
            || self.training[1] >= start
            || start > end
            || self.minimums.contains(&0)
            || self.alphas.iter().any(|&n| n > 1_000_000)
            || self.limits.contains(&0)
            || self.folds() > end - start + 1
            || self.folds() > self.limits[1]
        {
            return Err(
                "qualification requires nonempty, wholly later, physically admitted civil folds"
                    .into(),
            );
        }
        let bytes = (wire_size(self.folds())? as u64)
            .checked_add(
                self.folds()
                    .checked_mul(16)
                    .ok_or("qualification fold storage overflow")?,
            )
            .ok_or("qualification storage overflow")?;
        if bytes > self.limits[0].min(max_bytes) {
            return Err("qualification plan buffers exceed byte admission".into());
        }
        self.numerical_bounds()?;
        Ok(())
    }
}
impl Description {
    fn require_selected(&self, rung: usize) -> Result<(), String> {
        if self.facts.rungs.contains(rung) {
            Ok(())
        } else {
            Err("qualification timeframe is outside the declared selection".into())
        }
    }
    fn new(facts: Facts) -> Result<Self, String> {
        facts.admit(facts.limits[0])?;
        let [start, end] = facts.later;
        let days = end - start + 1;
        let count = facts.folds();
        let mut windows = Vec::new();
        windows
            .try_reserve_exact(usize::try_from(count).map_err(display)?)
            .map_err(display)?;
        let mut first = start;
        for index in 0..count {
            let width = days / count + u64::from(index < days % count);
            let last = first + width - 1;
            windows.push(LaterSessionWindowV1::new(
                i64::try_from(first).map_err(display)?,
                i64::try_from(last).map_err(display)?,
            )?);
            first = last + 1;
        }
        let mut description = Self {
            facts,
            windows,
            descriptor: [0; 32],
            units: [[0; 32]; 8],
        };
        let mut hash = Hasher::new();
        hash.update(b"brutex-boolean-qualification-plan-v1\0");
        hash.update(&description.encode()?);
        description.descriptor = hash.finalize();
        for (index, (unit, rung)) in description
            .units
            .iter_mut()
            .zip(crate::ledger_all::LEDGER_RUNGS)
            .enumerate()
        {
            let mut hash = Hasher::new();
            hash.update(b"brutex-boolean-qualification-unit-v1\0");
            hash.update(&description.descriptor);
            hash.update(
                &allocation::allocate(0, 1, index as u64, 8, description.facts.alpha())
                    .map_err(display)?
                    .digest(),
            );
            hash.update(&(rung.len() as u64).to_le_bytes());
            hash.update(rung.as_bytes());
            *unit = hash.finalize();
        }
        Ok(description)
    }
    fn requested(&self) -> Result<LaterSessionWindowV1, String> {
        LaterSessionWindowV1::new(
            i64::try_from(self.facts.later[0]).map_err(display)?,
            i64::try_from(self.facts.later[1]).map_err(display)?,
        )
    }
    fn allocation(&self, rung: usize) -> Result<Allocation, String> {
        self.require_selected(rung)?;
        allocation::allocate(
            0,
            1,
            u64::try_from(rung).map_err(display)?,
            8,
            self.facts.alpha(),
        )
        .map_err(display)
    }
    fn resolution_reachable(&self) -> Result<bool, String> {
        Ok(self
            .allocation(
                self.facts
                    .rungs
                    .indices()
                    .next()
                    .ok_or("qualification selection empty")?,
            )?
            .minimum_draws()
            .is_some_and(|minimum| u128::from(self.facts.procedure.draws()) >= minimum))
    }
    fn encode(&self) -> Result<Vec<u8>, String> {
        let facts = &self.facts;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(wire_size(facts.folds())?)
            .map_err(display)?;
        let version = if facts.index_policy.is_some() {
            3_u64
        } else if facts.rungs == RungScope::ALL {
            1_u64
        } else {
            2
        };
        bytes.extend_from_slice(match version {
            1 => MAGIC,
            2 => b"BRBQPL02",
            _ => b"BRBQPL03",
        });
        for value in [version, 1, 8] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for identity in facts
            .identities
            .iter()
            .chain(&facts.catalogs)
            .chain(&facts.sources)
        {
            bytes.extend_from_slice(identity);
        }
        let bounds = facts.numerical_bounds()?;
        for value in [facts.count, facts.horizon]
            .into_iter()
            .chain(facts.training)
            .chain(facts.later)
            .chain([
                facts.procedure.draws(),
                facts.procedure.seed(),
                facts.procedure.block_length(),
            ])
            .chain(facts.limits)
            .chain(facts.minimums)
            .chain(facts.alphas)
            .chain([facts.folds(), bounds.max_work, bounds.max_bytes])
        {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        if version != 1 {
            bytes.extend_from_slice(&u64::from(facts.rungs.mask()).to_le_bytes());
        }
        if let Some(policy) = facts.index_policy {
            bytes.extend_from_slice(&policy);
        }
        bytes.resize(HEADER, 0);
        for window in &self.windows {
            bytes.extend_from_slice(&window.first_day().to_le_bytes());
            bytes.extend_from_slice(&window.last_day().to_le_bytes());
        }
        Ok(bytes)
    }
}
fn wire_size(folds: u64) -> Result<usize, String> {
    usize::try_from(folds)
        .map_err(display)?
        .checked_mul(WINDOW)
        .and_then(|n| n.checked_add(HEADER))
        .ok_or_else(|| "qualification plan size overflow".into())
}
struct Decoder<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Decoder<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self
            .at
            .checked_add(N)
            .ok_or("qualification plan offset overflow")?;
        let found = self
            .bytes
            .get(self.at..end)
            .ok_or("truncated qualification plan")?;
        self.at = end;
        found.try_into().map_err(display)
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take()?))
    }
}
fn display(why: impl std::fmt::Debug) -> String {
    format!("{why:?}")
}

#[cfg(test)]
mod tests {
    //! Mechanical descriptor tests use invented hashes, never market authority.
    #![expect(clippy::unwrap_used, reason = "finite descriptor fixture assertions")]
    use super::*;
    fn facts() -> Facts {
        Facts {
            rungs: RungScope::ALL,
            index_policy: None,
            identities: std::array::from_fn(|index| {
                brutex_core::blake3::hash(&(index as u64).to_le_bytes())
            }),
            catalogs: [[3; 32]; 8],
            sources: [[4; 32]; 8],
            count: 2,
            horizon: 5,
            training: month_days(((2023, 12), (2024, 1))).unwrap(),
            later: month_days(((2024, 2), (2024, 3))).unwrap(),
            procedure: PopulationStatisticsProcedureV2::new(1_000, 1, 2).unwrap(),
            limits: [1_048_576, 10_000],
            minimums: [3, 2],
            alphas: [50_000, 25_000],
        }
    }
    #[test]
    fn daily_policy_plan_is_versioned_and_legacy_bytes_never_gain_a_requirement() {
        let old = Description::new(facts()).unwrap();
        let bytes = old.encode().unwrap();
        assert!(
            ObservedPlan::decode(&bytes, 1_048_576, 10_000)
                .unwrap()
                .index_policy()
                .is_none()
        );
        let mut ids = std::collections::HashSet::new();
        for mask in 1..=u8::MAX {
            let mut facts = facts();
            facts.rungs = RungScope::from_mask(mask).unwrap();
            facts.index_policy = Some(crate::index_consistency::Policy::V1.digest());
            for index in 0..8 {
                if !facts.rungs.contains(index) {
                    *facts.catalogs.get_mut(index).unwrap() = [0; 32];
                    *facts.sources.get_mut(index).unwrap() = [0; 32];
                }
            }
            let new = Description::new(facts).unwrap();
            let bytes = new.encode().unwrap();
            assert_eq!(bytes.get(..8).unwrap(), b"BRBQPL03");
            let saved = ObservedPlan::decode(&bytes, 1_048_576, 10_000).unwrap();
            assert_eq!(
                saved.index_policy(),
                Some(crate::index_consistency::Policy::V1.digest())
            );
            assert_eq!(saved.rungs().mask(), mask);
            assert!(ids.insert(saved.descriptor()));
            let mut wrong = bytes;
            *wrong.get_mut(952).unwrap() ^= 1;
            assert!(ObservedPlan::decode(&wrong, 1_048_576, 10_000).is_err());
        }
        assert_eq!(old.encode().unwrap(), bytes);
    }
    #[test]
    fn qualification_partition_covers_every_civil_day_before_later_observations() {
        let mut source = facts();
        for count in 1..=60 {
            source.minimums = [count, 1];
            let plan = Description::new(source.clone()).unwrap();
            assert_eq!(plan.windows.len() as u64, count);
            assert_eq!(
                plan.windows.first().unwrap().first_day(),
                i64::try_from(source.later[0]).unwrap()
            );
            assert_eq!(
                plan.windows.last().unwrap().last_day(),
                i64::try_from(source.later[1]).unwrap()
            );
            let mut days = 0;
            for pair in plan.windows.windows(2) {
                assert_eq!(
                    pair.first().unwrap().last_day() + 1,
                    pair.last().unwrap().first_day()
                );
            }
            for window in &plan.windows {
                days += window.last_day() - window.first_day() + 1;
            }
            assert_eq!(
                days, 60,
                "leap February plus March, with no removed nontrading day"
            );
        }
        source.minimums = [7, 9];
        let plan = Description::new(source).unwrap();
        assert_eq!(plan.windows.len(), 9, "both existing policy floors bind");
        let lengths: Vec<_> = plan
            .windows
            .iter()
            .map(|window| window.last_day() - window.first_day() + 1)
            .collect();
        assert_eq!(lengths, [7, 7, 7, 7, 7, 7, 6, 6, 6]);
    }
    #[test]
    fn qualification_identity_binds_both_full_sources_and_every_declared_term() {
        let original = facts();
        let plan = Description::new(original.clone()).unwrap();
        assert_eq!(plan, Description::new(original.clone()).unwrap());
        let mut variants = Vec::new();
        for index in 0..8 {
            let mut changed = original.clone();
            *changed.identities.get_mut(index).unwrap() = [19; 32];
            variants.push(changed);
            let mut changed = original.clone();
            *changed.catalogs.get_mut(index).unwrap() = [20; 32];
            variants.push(changed);
            let mut changed = original.clone();
            *changed.sources.get_mut(index).unwrap() = [21; 32];
            variants.push(changed);
        }
        for edit in [
            |f: &mut Facts| f.count += 1,
            |f: &mut Facts| f.horizon += 1,
            |f: &mut Facts| f.training = month_days(((2023, 11), (2024, 1))).unwrap(),
            |f: &mut Facts| f.later = month_days(((2024, 2), (2024, 4))).unwrap(),
            |f: &mut Facts| {
                f.procedure = PopulationStatisticsProcedureV2::new(1_001, 1, 2).unwrap();
            },
            |f: &mut Facts| {
                f.procedure = PopulationStatisticsProcedureV2::new(1_000, 2, 2).unwrap();
            },
            |f: &mut Facts| {
                f.procedure = PopulationStatisticsProcedureV2::new(1_000, 1, 3).unwrap();
            },
            |f: &mut Facts| f.limits[0] += 1,
            |f: &mut Facts| f.limits[1] += 1,
            |f: &mut Facts| f.minimums[0] += 1,
            |f: &mut Facts| f.minimums[1] -= 1,
            |f: &mut Facts| f.alphas[0] += 1,
            |f: &mut Facts| f.alphas[1] += 1,
        ] {
            let mut changed = original.clone();
            edit(&mut changed);
            variants.push(changed);
        }
        for changed in variants {
            let other = Description::new(changed).unwrap();
            assert_ne!(plan.descriptor, other.descriptor);
            assert!(plan.units.iter().zip(other.units).all(|(a, b)| *a != b));
        }
        assert_eq!(
            plan.units
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            8
        );
    }
    #[test]
    fn qualification_zero_and_unreachable_alpha_keep_all_eight_allocated_units() {
        let mut source = facts();
        source.procedure = PopulationStatisticsProcedureV2::new(1, 0, 1).unwrap();
        let low = Plan {
            description: Description::new(source.clone()).unwrap(),
        };
        assert!(!low.resolution_reachable().unwrap());
        for rung in 0..8 {
            let allocation = low.allocation(rung).unwrap();
            assert_eq!(allocation.batches(), 1);
            assert_eq!(allocation.rungs(), 8);
            assert_eq!(allocation.rung(), rung as u64);
            assert_eq!(allocation.alpha_ppm(), 25_000);
            assert_eq!(allocation.minimum_draws(), Some(319));
            assert!(!allocation.compare(1, 2).unwrap());
        }
        assert!(low.allocation(8).is_err());
        source.alphas = [0, 25_000];
        let zero = Description::new(source).unwrap();
        assert!(!zero.resolution_reachable().unwrap());
        assert_eq!(zero.allocation(0).unwrap().minimum_draws(), None);
        assert_ne!(zero.descriptor, low.descriptor());
        assert_eq!(zero.windows, low.windows());
    }

    #[test]
    fn subset_plan_binds_selected_sources_and_preserves_reserved_eight_unit_alpha() {
        let legacy = Description::new(facts()).unwrap();
        let legacy_bytes = legacy.encode().unwrap();
        let mut identities = std::collections::HashSet::new();
        for mask in 1..=u8::MAX {
            let mut selected = facts();
            selected.rungs = RungScope::from_mask(mask).unwrap();
            for index in 0..8 {
                if !selected.rungs.contains(index) {
                    *selected.catalogs.get_mut(index).unwrap() = [0; 32];
                    *selected.sources.get_mut(index).unwrap() = [0; 32];
                }
            }
            let plan = Description::new(selected.clone()).unwrap();
            assert!(identities.insert(plan.descriptor));
            let raw = plan.encode().unwrap();
            let observed = ObservedPlan::decode(&raw, selected.limits[0], 3).unwrap();
            assert_eq!(observed.rungs(), selected.rungs);
            assert_eq!(observed.description, plan);
            for index in 0..8 {
                if selected.rungs.contains(index) {
                    assert_eq!(
                        plan.allocation(index).unwrap(),
                        legacy.allocation(index).unwrap()
                    );
                    assert_eq!(
                        observed.expected_catalogs(index).unwrap(),
                        *selected.catalogs.get(index).unwrap()
                    );
                } else {
                    assert!(observed.allocation(index).is_err());
                    assert!(observed.expected_catalogs(index).is_err());
                    assert!(observed.expected_sources(index).is_err());
                }
            }
            if selected.rungs == RungScope::ALL {
                assert_eq!(raw, legacy_bytes);
            } else {
                assert_eq!(raw.get(..8).unwrap(), b"BRBQPL02");
                let index = (0..8)
                    .find(|&index| !selected.rungs.contains(index))
                    .unwrap();
                *selected.catalogs.get_mut(index).unwrap() = [9; 32];
                assert!(Description::new(selected).is_err());
                let mut changed = raw;
                changed.get_mut(944..952).unwrap().fill(0);
                assert!(ObservedPlan::decode(&changed, 1_048_576, 3).is_err());
            }
        }
        assert_eq!(identities.len(), 255);
    }
    #[test]
    fn qualification_plan_refuses_overlap_invalid_dates_and_exact_physical_caps() {
        let source = facts();
        let mut exact = source.clone();
        exact.limits[0] = HEADER as u64 + source.folds() * 32;
        Description::new(exact.clone()).unwrap();
        exact.limits[0] -= 1;
        assert!(
            Description::new(exact)
                .unwrap_err()
                .contains("byte admission")
        );
        for edit in [
            |f: &mut Facts| f.count = 0,
            |f: &mut Facts| f.horizon = u64::from(u32::MAX) + 1,
            |f: &mut Facts| f.training.swap(0, 1),
            |f: &mut Facts| f.later[0] = f.training[1],
            |f: &mut Facts| f.later.swap(0, 1),
            |f: &mut Facts| f.training[0] = u64::MAX,
            |f: &mut Facts| f.training[0] += 1,
            |f: &mut Facts| f.later[1] -= 1,
            |f: &mut Facts| f.minimums = [61, 1],
            |f: &mut Facts| f.minimums = [0, 1],
            |f: &mut Facts| f.alphas = [1_000_001, 1],
            |f: &mut Facts| f.limits[1] = 2,
            |f: &mut Facts| f.limits[1] = u64::MAX,
            |f: &mut Facts| {
                f.procedure = PopulationStatisticsProcedureV2::new(u64::MAX, 1, 1).unwrap();
            },
        ] {
            let mut changed = source.clone();
            edit(&mut changed);
            assert!(Description::new(changed).is_err());
        }
        assert!(month_days(((2024, 13), (2024, 1))).is_err());
        assert!(month_days(((2024, 3), (2024, 1))).is_err());
        assert!(wire_size(u64::MAX).is_err());
    }
    #[test]
    fn qualification_detached_codec_is_exact_bounded_and_cannot_hide_foreign_partition() {
        let plan = Plan {
            description: Description::new(facts()).unwrap(),
        };
        let raw = plan.canonical_bytes().unwrap();
        let byte_cap = HEADER as u64 + plan.windows().len() as u64 * 32;
        let observed = ObservedPlan::decode(&raw, byte_cap, 3).unwrap();
        assert_eq!(observed.descriptor(), plan.descriptor());
        assert_eq!(observed.units(), plan.units());
        for rung in 0..8 {
            assert_eq!(
                observed.expected_catalogs(rung).unwrap(),
                plan.expected_catalogs(rung).unwrap()
            );
            assert_eq!(
                observed.expected_sources(rung).unwrap(),
                plan.expected_sources(rung).unwrap()
            );
        }
        assert!(observed.expected_catalogs(8).is_err());
        assert!(observed.expected_sources(8).is_err());
        assert!(plan.expected_catalogs(8).is_err());
        assert!(plan.expected_sources(8).is_err());
        assert_eq!(observed.requested().unwrap(), plan.requested().unwrap());
        assert_eq!(observed.windows(), plan.windows());
        assert_eq!(observed.allocation(7).unwrap(), plan.allocation(7).unwrap());
        assert_eq!(
            observed.numerical_bounds().unwrap(),
            plan.numerical_bounds().unwrap()
        );
        assert_eq!(observed.procedure(), plan.procedure());
        assert_eq!(observed.source_limits(), plan.source_limits());
        assert_eq!(
            observed.resolution_reachable().unwrap(),
            plan.resolution_reachable().unwrap()
        );
        assert_eq!(observed.description.encode().unwrap(), raw);
        assert!(ObservedPlan::decode(&raw, byte_cap - 1, 3).is_err());
        assert!(ObservedPlan::decode(&raw, byte_cap, 2).is_err());
        for index in [0, 8, 16, 24, 920, 928, 936, 944, 1023, 1024, 1032] {
            let mut changed = raw.clone();
            *changed.get_mut(index).unwrap() ^= 1;
            assert!(
                ObservedPlan::decode(&changed, byte_cap, 3).is_err(),
                "field at {index}"
            );
        }
        for width in [0, 8, HEADER - 1, raw.len() - 1] {
            assert!(ObservedPlan::decode(raw.get(..width).unwrap(), byte_cap, 3).is_err());
        }
        let mut extra = raw.clone();
        extra.push(0);
        assert!(ObservedPlan::decode(&extra, byte_cap, 3).is_err());
        let mut foreign = raw;
        *foreign.get_mut(32).unwrap() ^= 1;
        let other = ObservedPlan::decode(&foreign, byte_cap, 3).unwrap();
        assert_ne!(
            other.descriptor(),
            observed.descriptor(),
            "outer owner must compare its pinned expected descriptor"
        );
    }
    #[test]
    fn qualification_ordered_source_bindings_preserve_every_identity_and_family_position() {
        let first = [1; 32];
        let second = [2; 32];
        let complete = ordered_identities([first, second].into_iter());
        assert_eq!(complete, ordered_identities([first, second].into_iter()));
        for changed in [
            ordered_identities([second, first].into_iter()),
            ordered_identities([first].into_iter()),
            ordered_identities([first, second, second].into_iter()),
            ordered_identities([first, [3; 32]].into_iter()),
            ordered_identities([].into_iter()),
        ] {
            assert_ne!(complete, changed);
        }
    }
}
