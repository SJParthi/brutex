#![cfg(test)]
//! Defensive internal corruption injection, not a constructible public input.
use super::*;
use crate::excursion::Side;
use crate::exit_grid_policy::{
    ExecutionResolutionV1, ExitGridSelectorV1, ForcedStopV1, RangeResolutionV1, RatioLimitsV1,
    RationalPercentileV1, RungPlanV1, printed_ohlcv_cost_model_id_v3,
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
        printed_ohlcv_cost_model_id_v3(),
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

/// A training slice whose prices breach the arithmetic envelope is refused at
/// attestation, by the envelope, with the bound it breached (G18-runner-16,
/// D-2063). Every price of the generated sessions is scaled by 10^9: the
/// ranges in ppm, and so every resolved ladder, are unchanged, while the
/// aggregate paisa bound `4 x bars x max high` passes `i64::MAX`. Before this
/// test no slice ever reached the envelope, so an envelope that always passed,
/// or extremes read as `(0, 1)`, went unnoticed.
#[test]
fn a_training_slice_past_the_arithmetic_envelope_is_refused_at_attestation()
-> Result<(), Box<dyn std::error::Error>> {
    const SCALE: i64 = 1_000_000_000;
    let key = InstrumentKey::cash(Exchange::Nse, "RELIANCE")?;
    let bars = crate::synthetic::sessions(8)
        .iter()
        .map(|b| {
            indicators::Candle::new(
                b.ts_micros,
                b.open * SCALE,
                b.high * SCALE,
                b.low * SCALE,
                b.close * SCALE,
                1,
                b.open_interest,
            )
        })
        .collect::<Vec<_>>();
    let highest = bars.iter().map(|b| b.high).max().ok_or("bars")?;
    let count = i128::try_from(bars.len())?;
    assert!(
        i128::from(highest) * count * 4 > i128::from(i64::MAX),
        "premise: the slice breaches the aggregate paisa bound"
    );
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
        printed_ohlcv_cost_model_id_v3(),
        ForcedStopV1::Disabled,
        1,
        1,
    )?;
    let resolved = policy.resolve_research_attested(series)?;
    let mut evaluator = indicators::evaluator::Evaluator::with_calendar(
        indicators::evaluator::Widths::pinned()?,
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        indicators::evaluator::Calendar::all_regular(),
    );
    let column = indicators::column::Column::build(&bars, &mut evaluator);
    let horizon = crate::outcome::Horizon::bars(5).ok_or("horizon")?;
    assert_eq!(
        resolved.attest_training(series, &column, horizon).err(),
        Some(ExitGridErrorV1::ArithmeticEnvelopeExceeded(
            "aggregate paisa accumulator"
        ))
    );
    Ok(())
}

/// An attested training slice prints its identity-bearing fields, not an
/// empty string (G18-runner-17, D-2063).
#[test]
fn an_attested_training_slice_debug_prints_its_identity() -> Result<(), Box<dyn std::error::Error>>
{
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
        printed_ohlcv_cost_model_id_v3(),
        ForcedStopV1::Disabled,
        1,
        1,
    )?;
    let resolved = policy.resolve_research_attested(series)?;
    let mut evaluator = indicators::evaluator::Evaluator::with_calendar(
        indicators::evaluator::Widths::pinned()?,
        indicators::vwap::Availability::Present,
        indicators::pattern::Thresholds::CLASSICAL,
        indicators::evaluator::Calendar::all_regular(),
    );
    let column = indicators::column::Column::build(&bars, &mut evaluator);
    let horizon = crate::outcome::Horizon::bars(5).ok_or("horizon")?;
    let attested = resolved.attest_training(series, &column, horizon)?;
    let printed = format!("{attested:?}");
    for field in [
        "AttestedTrainingV1",
        "resolution_digest",
        "bars",
        "column",
        "horizon",
        "column_digest",
        "evaluation_spec",
        "..",
    ] {
        assert!(printed.contains(field), "missing {field:?}: {printed}");
    }
    Ok(())
}
