//! Search-wide projection of existing exact saved statistical observations.
//! No bootstrap is rerun and no missing policy field becomes measured here.
use crate::boolean_evidence::{Qualification, QualificationRow};
use crate::boolean_qualified_journal::Link;
use crate::boolean_search_record::{ProjectionRule, Summary, debug, display};
use brutex_core::blake3::Hasher;
use runner::admission::research_projection::{ResearchAdmissionProjectionV1, hypothesis_decision};
use runner::admission::{AdmissionExactProbabilityV2, AdmissionStatusV1, ObservedU64V1};
use runner::search_allocation_v1::{Allocation, Fraction};

/// Full original observation plus the separately declared search-wide verdict.
pub struct SearchRow {
    /// Unmodified original/later qualification details, with all source links.
    pub source: QualificationRow,
    /// Complete common-policy evaluation after the additional search correction.
    pub projection: ResearchAdmissionProjectionV1,
    /// Exact search-scaled Romano-Wolf, White and SPA probabilities.
    pub probabilities: [Fraction; 3],
}

pub(crate) fn project(
    source: &Qualification,
    row: QualificationRow,
    allocation: Allocation,
    rule: ProjectionRule,
) -> Result<SearchRow, String> {
    project_values(
        rule,
        &source.policy(),
        source.family_probabilities(),
        row,
        allocation,
    )
}

fn project_values(
    rule: ProjectionRule,
    policy: &runner::admission::AdmissionPolicyV1,
    family_probabilities: [[u64; 2]; 2],
    row: QualificationRow,
    allocation: Allocation,
) -> Result<SearchRow, String> {
    let policy = effective_policy(policy, allocation, rule);
    let [white, spa] = family_probabilities;
    let probabilities = [
        allocation
            .scaled_probability(row.romano[1], row.romano[2])
            .map_err(debug)?,
        allocation
            .scaled_probability(white[0], white[1])
            .map_err(debug)?,
        allocation
            .scaled_probability(spa[0], spa[1])
            .map_err(debug)?,
    ];
    let mut values = row.values;
    values.fwer_p_value_ppm = ObservedU64V1::Measured(upper_ppm(probabilities[0])?);
    values.romano_wolf_p_value_ppm = values.fwer_p_value_ppm;
    values.romano_wolf_decision = hypothesis_decision(exact(probabilities[0])?);
    values.white_reality_p_value_ppm = ObservedU64V1::Measured(upper_ppm(probabilities[1])?);
    values.white_reality_decision = hypothesis_decision(exact(probabilities[1])?);
    values.spa_p_value_ppm = ObservedU64V1::Measured(upper_ppm(probabilities[2])?);
    let projection = policy.evaluate_research_projection(values).map_err(debug)?;
    if projection.verdict().status() == AdmissionStatusV1::Admitted
        && (row.verdict.status() != AdmissionStatusV1::Admitted
            || (rule == ProjectionRule::LegacyV1
                && !allocation
                    .compare(row.romano[1], row.romano[2])
                    .map_err(debug)?))
    {
        return Err("search-wide correction unexpectedly relaxed original qualification".into());
    }
    Ok(SearchRow {
        source: row,
        projection,
        probabilities,
    })
}

fn effective_policy(
    original: &runner::admission::AdmissionPolicyV1,
    allocation: Allocation,
    rule: ProjectionRule,
) -> runner::admission::AdmissionPolicyV1 {
    match rule {
        ProjectionRule::LegacyV1 => *original,
        ProjectionRule::SharedCeilingsV2
        | ProjectionRule::ScopedSharedCeilingsV3
        | ProjectionRule::IndexConsistencyV4 => {
            original.with_search_probability_ceiling(allocation.alpha_ppm())
        }
    }
}

fn exact(p: Fraction) -> Result<AdmissionExactProbabilityV2, String> {
    AdmissionExactProbabilityV2::new(
        u64::try_from(p.numerator()).map_err(display)?,
        u64::try_from(p.denominator()).map_err(display)?,
    )
    .map_err(debug)
}
fn upper_ppm(p: Fraction) -> Result<u64, String> {
    u64::try_from(
        p.numerator()
            .checked_mul(1_000_000)
            .ok_or("search probability ppm overflow")?
            .div_ceil(p.denominator()),
    )
    .map_err(display)
}

pub(crate) fn summarize(
    source: &Qualification,
    allocation: Allocation,
    rule: ProjectionRule,
) -> Result<Summary, String> {
    let mut digest = Hasher::new();
    digest.update(rule.summary_domain());
    digest.update(&source.identity());
    digest.update(&source.completion_digest());
    digest.update(&allocation.digest());
    digest.update(&(source.row_count() as u64).to_le_bytes());
    let mut counts = [0_u64; 4];
    for start in (0..source.row_count()).step_by(256) {
        for row in source.rows(source.completion_digest(), start, 256)? {
            let projected = project(source, row, allocation, rule)?;
            digest.update(&projected.source.original);
            digest.update(&projected.source.later);
            digest.update(&projected.projection.canonical_bytes());
            let [admitted, rejected, unmeasured, refused] = &mut counts;
            let count = match projected.projection.verdict().status() {
                AdmissionStatusV1::Admitted => admitted,
                AdmissionStatusV1::Rejected => rejected,
                AdmissionStatusV1::Unmeasured => unmeasured,
                AdmissionStatusV1::Refused => refused,
            };
            *count = count
                .checked_add(1)
                .ok_or("search projection count overflow")?;
        }
    }
    source.require_current()?;
    Ok(Summary {
        child: Link {
            identity: source.identity(),
            pin: source.completion_digest(),
        },
        count: source.row_count() as u64,
        counts,
        projection: digest.finalize(),
        allocation: allocation.digest(),
    })
}

/// One authenticated complete rung, with bounded access to all saved reasons.
pub struct RungReader<'a> {
    pub(crate) parent: Option<&'a crate::boolean_search_reader::Reader>,
    pub(crate) campaign: Option<crate::boolean_evidence::QualifiedCampaign>,
    pub(crate) batch_replay_nodes: u64,
    pub(crate) source: Qualification,
    pub(crate) allocation: Allocation,
    pub(crate) summary: Summary,
    pub(crate) projection_rule: ProjectionRule,
}
impl RungReader<'_> {
    /// Exact policy for the recorded arithmetic version. V1 retains its original
    /// ceilings; V2 caps the four family ceilings at the shared allowance.
    #[must_use]
    pub fn policy(&self) -> runner::admission::AdmissionPolicyV1 {
        effective_policy(&self.source.policy(), self.allocation, self.projection_rule)
    }

    /// Stored arithmetic version; historical V1 results are never relabelled V2.
    #[must_use]
    pub fn projection_version(&self) -> u8 {
        self.projection_rule.version()
    }

    /// Recheck the exact parent search, batch campaign and qualification child.
    /// # Errors
    /// Refuses missing or changed evidence anywhere in the retained ancestry.
    pub fn require_current(&self) -> Result<(), String> {
        if let Some(parent) = self.parent {
            parent.require_current()?;
        }
        if let Some(campaign) = &self.campaign {
            campaign.require_current()?;
        }
        self.source.require_current()
    }
    /// Serialized evidence admitted across the retained search, campaign and child.
    /// This excludes allocator overhead and total process resident memory.
    /// # Errors
    /// Refuses arithmetic overflow rather than reporting a partial ancestry count.
    pub fn admitted_bytes(&self) -> Result<u64, String> {
        self.source
            .admitted_bytes()
            .checked_add(
                self.parent
                    .map_or(0, crate::boolean_search_reader::Reader::admitted_bytes),
            )
            .and_then(|n| {
                n.checked_add(self.campaign.as_ref().map_or(
                    0,
                    crate::boolean_evidence::QualifiedCampaign::admitted_bytes,
                ))
            })
            .ok_or_else(|| "search detail ancestry byte count overflow".into())
    }
    /// Declared grammar work charged by parent history and this selected batch replay.
    /// # Errors
    /// Refuses arithmetic overflow rather than reporting only part of the work.
    pub fn replay_nodes(&self) -> Result<u64, String> {
        self.batch_replay_nodes
            .checked_add(
                self.parent
                    .map_or(0, crate::boolean_search_reader::Reader::replay_nodes),
            )
            .ok_or_else(|| "search detail replay count overflow".into())
    }
    /// Exact retained qualification child, including its trade/session/fold links.
    #[must_use]
    pub const fn source(&self) -> &Qualification {
        &self.source
    }
    /// Search-wide batch/timeframe allocation, fixed before later work.
    #[must_use]
    pub const fn allocation(&self) -> Allocation {
        self.allocation
    }
    /// Saved full-rung counts and projection digest, rechecked on open.
    #[must_use]
    pub const fn summary(&self) -> Summary {
        self.summary
    }
    /// Every coordinate in a bounded page; no winner-only projection.
    /// # Errors
    /// Refuses source replacement, wrong child pin or an invalid page.
    pub fn rows(
        &self,
        pin: [u8; 32],
        start: usize,
        limit: usize,
    ) -> Result<Vec<SearchRow>, String> {
        self.require_current()?;
        let rows = self
            .source
            .rows(pin, start, limit)?
            .into_iter()
            .map(|row| project(&self.source, row, self.allocation, self.projection_rule))
            .collect::<Result<Vec<_>, _>>()?;
        self.require_current()?;
        Ok(rows)
    }
}

#[cfg(test)]
#[path = "boolean_search_projection_tests.rs"]
mod tests;
