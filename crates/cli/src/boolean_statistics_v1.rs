//! Full-population statistics for an explicitly requested Boolean catalog.
//!
//! This is a new authority, never a legacy mask/statistics receipt. Every
//! coordinate participates, including refused executions and zero-trade rows.
//! The existing Romano–Wolf procedure cannot measure a family containing a
//! constant-return candidate; that absence is retained and cannot pass admission.
//! Preparation and integrity reads are bounded input-dependent work, not O(1).
//! The memory bound covers additional statistics working buffers. Retained
//! candidate trades, periods and source guards remain outside that bound; it
//! is not a whole-process or whole-campaign memory ceiling.

use std::path::{Path, PathBuf};

use brutex_core::blake3::{Hasher, hash};
use runner::bootstrap::{RomanoWolfAdjustedReceiptV1, SpaReceiptV1, WhiteRealityCheckReceiptV1};
use runner::research_family::{ResearchFamilyV1, ResearchScopeV1};

use crate::population_observations_v1::{CscvLayoutReceiptV1, derive_layout};
use crate::population_statistics_v2::PopulationStatisticsProcedureV2;
use crate::sweep_evidence::{Completion, Operation};

use super::{BooleanCoordinateV1, CommittedBooleanFamilyV1, display, persistence};

#[path = "boolean_admission_v1.rs"]
pub(crate) mod admission;
#[path = "boolean_statistics_numeric_v1.rs"]
mod numeric;
#[path = "boolean_statistics_reader.rs"]
pub(crate) mod reader;
#[path = "boolean_statistics_wire_v1.rs"]
mod wire;

/// Physical admission, separate from every statistical decision threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Bounds {
    pub families: u64,
    pub candidates: u64,
    pub observations: u64,
    pub bootstrap_work: u64,
    pub split_work: u64,
    /// Additional numeric and publication buffers, excluding retained sources.
    pub memory_bytes: u64,
    pub bytes: u64,
}

impl Bounds {
    fn words(self) -> [u64; 7] {
        [
            self.families,
            self.candidates,
            self.observations,
            self.bootstrap_work,
            self.split_work,
            self.memory_bytes,
            self.bytes,
        ]
    }
}

/// A missing procedure result is not a zero probability or a rejected null.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RomanoWolfAvailabilityV1 {
    Measured,
    ConstantReturnCandidate,
    NumericalRefusal,
}

impl RomanoWolfAvailabilityV1 {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::ConstantReturnCandidate => {
                "unavailable: complete family contains a constant-return candidate"
            }
            Self::NumericalRefusal => "unavailable: shared Romano-Wolf numeric procedure refused",
        }
    }
    const fn code(self) -> u64 {
        match self {
            Self::Measured => 1,
            Self::ConstantReturnCandidate => 2,
            Self::NumericalRefusal => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CandidateStatisticsV1 {
    pub family: usize,
    pub coordinate: usize,
    pub identity: [u8; 32],
    pub trades: u64,
    pub wins: u64,
    pub return_paisa: i64,
    pub wilson_lower_bits: u64,
    pub period_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SplitStatisticsV1 {
    train_mask: u64,
    test_mask: u64,
    bottom_half: bool,
    rankable: bool,
    scores_digest: [u8; 32],
}

/// Full precision results over the complete, canonically ordered cohort.
pub(crate) struct MeasurementsV1 {
    pub white: WhiteRealityCheckReceiptV1,
    pub spa: SpaReceiptV1,
    pub romano: Option<RomanoWolfAdjustedReceiptV1>,
    pub romano_availability: RomanoWolfAvailabilityV1,
    pub candidates: Vec<CandidateStatisticsV1>,
    pub contributing_splits: u64,
    pub bottom_half_splits: u64,
    splits: Vec<SplitStatisticsV1>,
}

struct Group {
    sources: Vec<CommittedBooleanFamilyV1>,
    scope: ResearchScopeV1,
    cohort: [u8; 32],
    layout: CscvLayoutReceiptV1,
    candidates: usize,
}

/// Durable successor capability retaining the original market-source guards.
pub(crate) struct CommittedBooleanStatisticsV1 {
    group: Group,
    identity: [u8; 32],
    completion: [u8; 32],
    directory: PathBuf,
    payload: [u8; 32],
    bytes: u64,
    measurements: MeasurementsV1,
}

impl CommittedBooleanStatisticsV1 {
    pub(crate) fn require_current(&self) -> Result<(), String> {
        self.group.require_current()?;
        persistence::verify(
            &self.directory,
            self.identity,
            self.payload,
            self.bytes,
            self.completion,
        )?;
        self.group.require_current()
    }
    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    pub(crate) const fn completion_digest(&self) -> [u8; 32] {
        self.completion
    }
    pub(crate) fn measurements(&self) -> &MeasurementsV1 {
        &self.measurements
    }
    pub(crate) fn sources(&self) -> &[CommittedBooleanFamilyV1] {
        &self.group.sources
    }
    pub(crate) fn families(&self) -> &[ResearchFamilyV1] {
        self.group.scope.families()
    }
    pub(crate) const fn layout(&self) -> CscvLayoutReceiptV1 {
        self.group.layout
    }
}

/// Persist before the numeric work, then publish only the exact verified body.
pub(crate) fn produce(
    root: &Path,
    sources: Vec<CommittedBooleanFamilyV1>,
    procedure: PopulationStatisticsProcedureV2,
    bounds: Bounds,
) -> Result<CommittedBooleanStatisticsV1, String> {
    let group = Group::new(sources, bounds)?;
    admit(&group, procedure, bounds)?;
    let identity = identity(&group, procedure, bounds);
    let attempt = crate::sweep_evidence::begin(root, identity, Operation::BooleanStatistics)?;
    let prepared = numeric::measure(&group, procedure).and_then(|measured| {
        wire::encode(&group, identity, procedure, bounds, &measured).map(|body| (measured, body))
    });
    let (measurements, body) = match prepared {
        Ok(value) => value,
        Err(why) => return Err(super::refuse(attempt, why)),
    };
    let pending =
        match persistence::prepare_in_namespace(root, "boolean-statistics-v1", identity, &body) {
            Ok(value) => value,
            Err(why) => return Err(super::refuse(attempt, why)),
        };
    let payload = hash(&body);
    let bytes = body.len() as u64;
    let completion = crate::finish_stored_month(
        attempt,
        Completion::Completed,
        || {
            group.require_current()?;
            pending.verify_body(payload, bytes)
        },
        || pending.finish(identity, payload, bytes),
    )?;
    let committed = CommittedBooleanStatisticsV1 {
        group,
        identity,
        completion,
        directory: pending.directory().to_path_buf(),
        payload,
        bytes,
        measurements,
    };
    committed.require_current()?;
    Ok(committed)
}

impl Group {
    fn new(mut sources: Vec<CommittedBooleanFamilyV1>, bounds: Bounds) -> Result<Self, String> {
        if bounds.words().contains(&0)
            || sources.is_empty()
            || sources.len() as u64 > bounds.families
        {
            return Err(
                "Boolean statistics scope or physical limits are empty/exceeded".to_owned(),
            );
        }
        let mut keys = Vec::new();
        keys.try_reserve_exact(sources.len()).map_err(display)?;
        for source in &sources {
            source.require_current()?;
            keys.push(source.family().instrument());
        }
        let scope = ResearchScopeV1::new(&keys, usize::try_from(bounds.families).map_err(display)?)
            .map_err(display)?;
        sources.sort_unstable_by_key(|source| scope.position(&source.family().instrument()));
        let first = sources
            .first()
            .ok_or("Boolean statistics first source missing")?;
        let cohort = first.cohort_digest();
        let sessions = first.sessions();
        if sessions.len() < 2 || sessions.windows(2).any(|pair| pair.first() >= pair.get(1)) {
            return Err("Boolean statistics sessions must be ordered, distinct and contain at least two periods".to_owned());
        }
        let layout = derive_layout(sessions.len())?;
        let mut candidates = 0_usize;
        for source in &sources {
            if source.cohort_digest() != cohort
                || source.sessions() != sessions
                || source.programs() != first.programs()
                || source.rows().is_empty()
            {
                return Err("Boolean statistics families disagree on complete catalog, sessions or evaluation context".to_owned());
            }
            candidates = candidates
                .checked_add(source.rows().len())
                .ok_or("Boolean statistics candidate count overflow")?;
        }
        Ok(Self {
            sources,
            scope,
            cohort,
            layout,
            candidates,
        })
    }
    fn require_current(&self) -> Result<(), String> {
        for source in &self.sources {
            source.require_current()?;
        }
        Ok(())
    }
}

fn admit(
    group: &Group,
    procedure: PopulationStatisticsProcedureV2,
    bounds: Bounds,
) -> Result<(), String> {
    let candidates = u64::try_from(group.candidates).map_err(display)?;
    let body_bytes = wire::required_bytes(group)?;
    admit_shape(
        candidates,
        group.layout.period_count(),
        group.layout.split_count(),
        procedure,
        bounds,
        body_bytes,
    )?;
    Ok(())
}

fn admit_shape(
    candidates: u64,
    periods: u64,
    splits: u64,
    procedure: PopulationStatisticsProcedureV2,
    bounds: Bounds,
    body_bytes: u64,
) -> Result<(), String> {
    let observations = candidates
        .checked_mul(periods)
        .ok_or("Boolean statistics observation overflow")?;
    let bootstrap_work = observations
        .checked_mul(procedure.draws())
        .and_then(|n| n.checked_mul(3))
        .ok_or("Boolean statistics bootstrap work overflow")?;
    let split_work = observations
        .checked_mul(splits)
        .ok_or("Boolean statistics split work overflow")?;
    // Shared RW retains every sampled index. Include that allocation explicitly,
    // in addition to our full return matrix, output rows and fixed body buffer.
    let bootstrap_memory = procedure
        .draws()
        .checked_mul(periods)
        .and_then(|n| n.checked_mul(8))
        .and_then(|n| {
            procedure
                .draws()
                .checked_mul(32)
                .and_then(|rows| n.checked_add(rows))
        })
        .ok_or("Boolean statistics bootstrap memory overflow")?;
    let memory = observations
        .checked_mul(8)
        .and_then(|n| n.checked_add(bootstrap_memory))
        .and_then(|n| {
            candidates
                .checked_mul(1024)
                .and_then(|rows| n.checked_add(rows))
        })
        .and_then(|n| splits.checked_mul(640).and_then(|rows| n.checked_add(rows)))
        .and_then(|n| {
            body_bytes
                .checked_mul(2)
                .and_then(|body| n.checked_add(body))
        })
        .ok_or("Boolean statistics memory overflow")?;
    if candidates == 0
        || periods < 2
        || splits == 0
        || bounds.words().contains(&0)
        || candidates > bounds.candidates
        || observations > bounds.observations
        || bootstrap_work > bounds.bootstrap_work
        || split_work > bounds.split_work
        || memory > bounds.memory_bytes
        || body_bytes > bounds.bytes
        || procedure.draws().checked_add(1).is_none()
    {
        return Err("Boolean statistics complete-family work/memory admission refused".to_owned());
    }
    for count in [
        candidates,
        periods,
        splits,
        procedure.draws(),
        procedure.block_length(),
        memory,
    ] {
        usize::try_from(count).map_err(display)?;
    }
    Ok(())
}

fn identity(group: &Group, procedure: PopulationStatisticsProcedureV2, bounds: Bounds) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(b"brutex-boolean-statistics-v1\0");
    hash.update(&group.scope.digest());
    hash.update(&group.cohort);
    hash.update(&group.layout.digest());
    for value in [
        procedure.draws(),
        procedure.seed(),
        procedure.block_length(),
    ]
    .into_iter()
    .chain(bounds.words())
    {
        hash.update(&value.to_le_bytes());
    }
    for source in &group.sources {
        hash.update(&source.family().encode());
        hash.update(&source.identity());
        hash.update(&source.completion_digest());
    }
    hash.finalize()
}

#[cfg(test)]
#[path = "boolean_statistics_tests.rs"]
mod tests;
