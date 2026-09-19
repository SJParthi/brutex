#![cfg(test)]
//! Defensive internal corruption injection, not a constructible public input.
use super::*;
use crate::excursion::Side;
use crate::exit_grid_policy::{
    ExecutionResolutionV1, ExitGridSelectorV1, ForcedStopV1, RangeResolutionV1, RatioLimitsV1,
    RationalPercentileV1, RungPlanV1,
};
use brutex_core::instrument::Exchange;

#[test]
fn internally_corrupted_resolution_cannot_reach_ladders_or_training_attestation()
-> Result<(), Box<dyn std::error::Error>> {
    let key = InstrumentKey::cash(Exchange::Nse, "RELIANCE")?;
    let bars = crate::synthetic::sessions(8);
    let series = ExecutionSeriesV1::new(&key, "generated-test", "generated-build", [7; 32], &bars)?;
    let rank = RationalPercentileV1::new(1, 2)?;
    let policy = ExitGridPolicyV1::new(
        ExecutionResolutionV1::OneMinuteOhlcv,
        RangeResolutionV1::PpmCeiling,
        Side::Long,
        RungPlanV1::new(vec![rank], vec![rank], vec![rank], 1)?,
        RatioLimitsV1::new(1, 1_000_000, 1)?,
        1000,
        ExitGridSelectorV1::GuaranteedFloor,
        printed_ohlcv_cost_model_id_v1(),
        ForcedStopV1::Disabled,
        1,
        1,
    )?;
    let original = policy.resolve_research_attested(series)?;
    let mut evaluator = indicators::evaluator::Evaluator::with_calendar(
        indicators::evaluator::Widths::pinned()?,
        indicators::vwap::Availability::Present,
        indicators::pattern::Thresholds::CLASSICAL,
        indicators::evaluator::Calendar::all_regular(),
    );
    let column = indicators::column::Column::build(&bars, &mut evaluator);
    let horizon = crate::outcome::Horizon::bars(5).ok_or("horizon")?;
    assert!(original.ladders().is_ok());
    assert!(original.attest_training(series, &column, horizon).is_ok());
    let mutations: [fn(&mut ResearchResolvedExitGridV1); 5] = [
        |value| value.digest = [0; 32],
        |value| value.policy_digest = [0; 32],
        |value| value.training_bars += 1,
        |value| value.calendar_digest = [0; 32],
        |value| value.levels.ratio_bitmap.clear(),
    ];
    for mutation in mutations {
        let mut altered = original.clone();
        mutation(&mut altered);
        assert!(!altered.digest_is_valid());
        assert_eq!(
            altered.ladders(),
            Err(ExitGridErrorV1::ResolutionDigestMismatch)
        );
        assert!(altered.attest_training(series, &column, horizon).is_err());
    }
    let mut foreign = original;
    foreign.family = ResearchFamilyV1::new(InstrumentKey::cash(Exchange::Nse, "ADANIENT")?)?;
    // Even a recomputed checksum cannot repair a crosswired typed key.
    foreign.digest = digest_resolved(&foreign);
    assert!(!foreign.digest_is_valid());
    assert_eq!(
        foreign.ladders(),
        Err(ExitGridErrorV1::ResolutionDigestMismatch)
    );
    Ok(())
}
