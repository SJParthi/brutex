//! Saved comparison observers. No bytes mint training, OOS or selection authority.
use super::super::super::persistence::Observation;
use super::super::reader::{Record, Statistics, StatisticsRow, StatisticsSummary, charge, page};
use super::{display, probability};
use brutex_core::blake3::Hasher;
use runner::admission::research_projection::{hypothesis_decision, wilson_ppm};
use runner::admission::{
    AdmissionEvidenceValuesV1, AdmissionPolicyV1, AdmissionVerdictV1, CompletenessV1, ObservedU64V1,
};
use std::path::Path;

/// Exact observed candidate comparison with every stable value and reason bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionRow {
    /// Full candidate identity, also present in the linked statistics row.
    pub identity: [u8; 32],
    /// Canonical index in the full statistics population.
    pub source_index: usize,
    /// All44 observed fields; unavailable fields retain their explicit tags.
    pub values: AdmissionEvidenceValuesV1,
    /// Complete common-policy comparison and all reason partitions.
    pub verdict: AdmissionVerdictV1,
}

/// A saved admission observation retaining authenticated saved ancestors.
pub struct Admission {
    observation: Observation,
    statistics: Statistics,
    policy: AdmissionPolicyV1,
    rows: Vec<AdmissionRow>,
}

struct Decoded {
    statistics: [u8; 32],
    completion: [u8; 32],
    policy: AdmissionPolicyV1,
    rows: Vec<AdmissionRow>,
}

impl Admission {
    pub(crate) fn all_rows(&self) -> &[AdmissionRow] {
        &self.rows
    }
    /// Authenticate admission, statistics and all candidate ancestors under one
    /// serialized-byte ceiling, then verify every saved comparison against them.
    /// # Errors
    /// Refuses missing/corrupt/foreign evidence, inconsistent values or budgets.
    pub fn open(root: &Path, identity: [u8; 32], max_bytes: u64) -> Result<Self, String> {
        let (observation, decoded) =
            Observation::open(root, "boolean-admission-v1", identity, max_bytes, |raw| {
                decode(raw, identity)
            })?;
        let remaining = charge(max_bytes, observation.body_bytes())?;
        let statistics = Statistics::open(root, decoded.statistics, remaining)?;
        if statistics.completion_digest() != decoded.completion
            || statistics.summary().candidates != decoded.rows.len()
        {
            return Err("Boolean admission referenced statistics differs".to_owned());
        }
        let result = Self {
            observation,
            statistics,
            policy: decoded.policy,
            rows: decoded.rows,
        };
        result.check_values()?;
        result.require_current()?;
        Ok(result)
    }
    /// Exact admission identity.
    #[must_use]
    pub fn identity(&self) -> [u8; 32] {
        self.observation.identity()
    }
    /// Immutable completion pin required on pages.
    #[must_use]
    pub fn completion_digest(&self) -> [u8; 32] {
        self.observation.completion_digest()
    }
    /// Full authenticated statistics ancestor, with candidate navigation links.
    #[must_use]
    pub const fn statistics(&self) -> &Statistics {
        &self.statistics
    }
    /// Complete validated policy, including all39 canonical settings.
    #[must_use]
    pub const fn policy(&self) -> AdmissionPolicyV1 {
        self.policy
    }
    /// Full population count; no winner-only projection.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
    /// Saved admission body bytes, excluding linked ancestors.
    #[must_use]
    pub fn body_bytes(&self) -> u64 {
        self.observation.body_bytes()
    }
    /// Recheck all saved evidence generations, without raw-market re-attestation.
    /// # Errors
    /// Any replaced/deleted/changed saved body, receipt or owner refuses.
    pub fn require_current(&self) -> Result<(), String> {
        self.observation
            .with_current(|| self.statistics.require_current())
    }
    /// Return up to256 complete comparison rows pinned to this completion.
    /// # Errors
    /// Refuses wrong pins, bad offsets/limits or changed saved ancestors.
    pub fn rows(
        &self,
        pin: [u8; 32],
        start: usize,
        limit: usize,
    ) -> Result<Vec<AdmissionRow>, String> {
        if pin != self.completion_digest() {
            return Err("Boolean admission completion pin differs".to_owned());
        }
        self.observation.with_current(|| {
            self.statistics
                .with_current(|| page(&self.rows, start, limit))
        })
    }
    fn check_values(&self) -> Result<(), String> {
        let family = self
            .statistics
            .all_rows()
            .iter()
            .filter_map(|row| row.romano)
            .find(|row| row[1] == 0);
        for (index, (saved, measured)) in
            self.rows.iter().zip(self.statistics.all_rows()).enumerate()
        {
            if saved.source_index != index || saved.identity != measured.identity {
                return Err("Boolean admission candidate order or identity differs".to_owned());
            }
            let expected = self.statistics.with_source(measured, |row| {
                let mut values = super::base_values(row)?;
                apply_statistics(&mut values, measured, self.statistics.summary(), family)?;
                Ok(values)
            })?;
            if saved.values != expected {
                return Err(
                    "Boolean admission values differ from authenticated saved ancestors".to_owned(),
                );
            }
        }
        Ok(())
    }
}

fn decode(raw: &[u8], identity: [u8; 32]) -> Result<Decoded, String> {
    let header = raw.get(..512).ok_or("Boolean admission manifest missing")?;
    let mut input = Record::new(header);
    if input.bytes::<8>()? != *b"BTXBAM01" || input.bytes::<32>()? != identity {
        return Err("Boolean admission manifest identity differs".to_owned());
    }
    let statistics = input.bytes()?;
    let completion = input.bytes()?;
    let policy = AdmissionPolicyV1::from_canonical_bytes(&input.bytes::<310>()?)
        .map_err(|why| format!("Boolean admission policy: {why:?}"))?;
    let max_bytes = input.word()?;
    let count = input.size()?;
    input.padding()?;
    let expected = count
        .checked_mul(1024)
        .and_then(|n| n.checked_add(512))
        .ok_or("Boolean admission cardinality overflow")?;
    if count == 0 || expected != raw.len() || raw.len() as u64 > max_bytes {
        return Err("Boolean admission count or saved budget differs".to_owned());
    }
    let mut hash = Hasher::new();
    hash.update(b"brutex-boolean-admission-v1\0");
    hash.update(&statistics);
    hash.update(&completion);
    hash.update(&policy.digest());
    hash.update(&max_bytes.to_le_bytes());
    if hash.finalize() != identity {
        return Err("Boolean admission inputs do not reproduce identity".to_owned());
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(count).map_err(display)?;
    for (index, bytes) in raw
        .get(512..)
        .ok_or("Boolean admission rows missing")?
        .chunks_exact(1024)
        .enumerate()
    {
        let mut input = Record::new(bytes);
        let identity = input.bytes()?;
        let source_index = input.size()?;
        let projection =
            runner::admission::research_projection_codec::decode(&policy, &input.bytes::<448>()?)?;
        input.padding()?;
        if source_index != index {
            return Err("Boolean admission row ordering differs".to_owned());
        }
        rows.push(AdmissionRow {
            identity,
            source_index,
            values: projection.values(),
            verdict: projection.verdict(),
        });
    }
    Ok(Decoded {
        statistics,
        completion,
        policy,
        rows,
    })
}

fn apply_statistics(
    values: &mut AdmissionEvidenceValuesV1,
    row: &StatisticsRow,
    summary: &StatisticsSummary,
    family: Option<[u64; 8]>,
) -> Result<(), String> {
    values.wilson_win_rate_ppm = ObservedU64V1::Measured(
        wilson_ppm(row.wins, row.trades, row.wilson_lower_bits)
            .ok_or("Boolean Wilson source differs")?,
    );
    let white = probability(summary.white[2], summary.white[3])?;
    values.white_reality_p_value_ppm = ObservedU64V1::Measured(white.ppm());
    values.white_reality_decision = hypothesis_decision(white);
    values.spa_p_value_ppm =
        ObservedU64V1::Measured(probability(summary.spa[2], summary.spa[3])?.ppm());
    values.bootstrap_draws = ObservedU64V1::Measured(summary.procedure[0]);
    values.bootstrap_strategies = ObservedU64V1::Measured(summary.candidates as u64);
    values.bootstrap_periods = ObservedU64V1::Measured(summary.periods);
    values.pbo_contributing_folds = ObservedU64V1::Measured(summary.contributing_splits);
    values.pbo_unrankable_folds = ObservedU64V1::Measured(
        (summary.splits as u64)
            .checked_sub(summary.contributing_splits)
            .ok_or("Boolean split count differs")?,
    );
    if summary.contributing_splits > 0 {
        values.pbo_ppm = ObservedU64V1::Measured(
            probability(summary.bottom_half_splits, summary.contributing_splits)?.ppm(),
        );
    }
    if let Some(romano) = row.romano {
        let family = family.ok_or("Boolean RW family rank missing")?;
        values.fwer_p_value_ppm = ObservedU64V1::Measured(probability(family[6], family[7])?.ppm());
        let adjusted = probability(romano[6], romano[7])?;
        values.romano_wolf_p_value_ppm = ObservedU64V1::Measured(adjusted.ppm());
        values.romano_wolf_decision = hypothesis_decision(adjusted);
        values.full_precision_statistics_complete = CompletenessV1::Complete;
    }
    Ok(())
}
