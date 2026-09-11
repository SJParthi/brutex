//! Statistics V3 over an actual evaluated-or-extinct stored family capability.
//! A naturally empty family never needs a fabricated nonempty Pre-Admission V1.

use super::StoredPopulationV6RouteV1;
use super::family_v6::StoredFamilyV6;
use crate::population_observations_v1::{
    ObservationAuthorityCommitV2, ProducedObservationAuthorityV2,
    produce_natural_extinction_observation_v2,
};
use crate::population_statistics_v3::{
    ProducedPopulationStatisticsV3, produce_all_extinct_population_statistics_v3,
    produce_evaluated_population_statistics_v3, produce_mixed_population_statistics_v3,
};

/// Every variant must agree with its freshly authenticated Candidate ledger.
/// An empty evaluated variant or a nonempty extinct variant is a crosswire.
fn require_variant(family: &StoredFamilyV6, name: &str) -> Result<(), String> {
    family.require_current()?;
    let has_rows = family.candidate_audit().row_count() != 0;
    if has_rows != family.evaluated().is_some() {
        return Err(format!(
            "Step 3 {name} stored V6 family variant disagrees with Candidate row count"
        ));
    }
    Ok(())
}

fn extinction(
    family: &StoredFamilyV6,
    name: &str,
    route: &StoredPopulationV6RouteV1<'_>,
) -> Result<(ProducedObservationAuthorityV2, ObservationAuthorityCommitV2), String> {
    require_variant(family, name)?;
    if family.evaluated().is_some() {
        return Err(format!(
            "Step 3 {name} evaluated family cannot mint an extinction observation"
        ));
    }
    let (produced, commit) = family.pre_admission_v2();
    let authority = produce_natural_extinction_observation_v2(produced, commit)
        .map_err(|why| format!("Step 3 {name} natural-extinction observation refused: {why}"))?;
    family.require_current()?;
    let committed = authority
        .append_and_reopen(route.observation_root, route.observation_bounds)
        .map_err(|why| {
            format!("Step 3 {name} Observation V2 receipt-last commit refused: {why}")
        })?;
    family.require_current()?;
    Ok((authority, committed))
}

/// Dispatch from the typed family variants, retaining exact source checks at
/// every observation publication and before the statistics product escapes.
pub(super) fn produce(
    nifty: &StoredFamilyV6,
    banknifty: &StoredFamilyV6,
    route: &StoredPopulationV6RouteV1<'_>,
) -> Result<ProducedPopulationStatisticsV3, String> {
    require_variant(nifty, "NIFTY")?;
    require_variant(banknifty, "BANKNIFTY")?;
    let produced = match (nifty.evaluated(), banknifty.evaluated()) {
        (Some(nifty), Some(banknifty)) => {
            let nifty = nifty.candidate_pre_admission();
            let banknifty = banknifty.candidate_pre_admission();
            produce_evaluated_population_statistics_v3(
                nifty.observations(),
                nifty.pre_admission_audit(),
                banknifty.observations(),
                banknifty.pre_admission_audit(),
                route.procedure,
            )
            .map_err(|why| format!("Step 3 Statistics V3 evaluated pair refused: {why}"))?
        }
        (Some(nifty), None) => {
            let nifty = nifty.candidate_pre_admission();
            let (extinct, commit) = extinction(banknifty, "BANKNIFTY", route)?;
            produce_mixed_population_statistics_v3(
                nifty.observations(),
                nifty.pre_admission_audit(),
                &extinct,
                &commit,
                route.procedure,
            )
            .map_err(|why| format!("Step 3 Statistics V3 mixed pair refused: {why}"))?
        }
        (None, Some(banknifty)) => {
            let banknifty = banknifty.candidate_pre_admission();
            let (extinct, commit) = extinction(nifty, "NIFTY", route)?;
            produce_mixed_population_statistics_v3(
                banknifty.observations(),
                banknifty.pre_admission_audit(),
                &extinct,
                &commit,
                route.procedure,
            )
            .map_err(|why| format!("Step 3 Statistics V3 mixed pair refused: {why}"))?
        }
        (None, None) => {
            let (nifty_extinct, nifty_commit) = extinction(nifty, "NIFTY", route)?;
            let (banknifty_extinct, banknifty_commit) = extinction(banknifty, "BANKNIFTY", route)?;
            produce_all_extinct_population_statistics_v3(
                &nifty_extinct,
                &nifty_commit,
                &banknifty_extinct,
                &banknifty_commit,
            )
            .map_err(|why| format!("Step 3 Statistics V3 all-extinct pair refused: {why}"))?
        }
    };
    nifty.require_current()?;
    banknifty.require_current()?;
    Ok(produced)
}
