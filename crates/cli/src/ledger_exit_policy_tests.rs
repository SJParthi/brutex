#![cfg(test)]
//! Generated fixtures prove runtime resolution and identity wiring, not returns.
use super::*;
use crate::audited_range_command::StrictConfig;
use crate::candidate_universe::boolean_candidate_v1 as candidate;
use std::collections::HashSet;

fn debug(why: impl std::fmt::Debug) -> String {
    format!("{why:?}")
}

// Independent reconstruction of the policy before runtime grid wiring.
fn original_five(side: Side) -> Result<ExitGridPolicyV1, String> {
    let ladder = [1, 2, 3, 4, 5]
        .into_iter()
        .map(|step| RationalPercentileV1::new(step, 5).map_err(debug))
        .collect::<Result<Vec<_>, _>>()?;
    ExitGridPolicyV1::new(
        ExecutionResolutionV1::OneMinuteOhlcv,
        RangeResolutionV1::PpmCeiling,
        side,
        RungPlanV1::new(ladder.clone(), ladder.clone(), ladder, 5).map_err(debug)?,
        RatioLimitsV1::new(300, i64::MAX, u64::MAX).map_err(debug)?,
        16_384,
        ExitGridSelectorV1::GuaranteedFloor,
        printed_ohlcv_cost_model_id_v1(),
        ForcedStopV1::Disabled,
        u64::MAX,
        u64::MAX,
    )
    .map_err(debug)
}

#[test]
fn absent_and_explicit_five_preserve_original_policy_and_digest() -> Result<(), String> {
    for side in [Side::Long, Side::Short] {
        let original = original_five(side)?;
        for raw in [None, Some("5")] {
            let policy = exit_policy_with(side, raw)?;
            assert_eq!(policy, original);
            assert_eq!(policy.digest(), original.digest());
        }
    }
    Ok(())
}

struct ClearKnobs;
impl Drop for ClearKnobs {
    fn drop(&mut self) {
        crate::knobs::clear_all();
    }
}

#[test]
fn every_admitted_runtime_resolution_binds_exact_axes_without_changing_risk() -> Result<(), String>
{
    let _serial = crate::knobs::serially();
    let _clear = ClearKnobs;
    crate::knobs::clear_all();
    let ceiling = crate::rungs_within_cell_budget();
    let mut digests = HashSet::new();
    for count in 2..=ceiling {
        let raw = count.to_string();
        assert!(crate::audited_range_command::request_value(
            "BRUTEX_GRID_RUNGS",
            &raw
        ));
        crate::knobs::set("BRUTEX_GRID_RUNGS", &raw);
        for side in [Side::Long, Side::Short] {
            let policy = exit_policy(side)?;
            let denominator = u32::try_from(count).map_err(debug)?;
            let expected = (1..=denominator)
                .map(|step| RationalPercentileV1::new(step, denominator).map_err(debug))
                .collect::<Result<Vec<_>, _>>()?;
            assert_eq!(policy.rungs().stop(), expected);
            assert_eq!(policy.rungs().target(), expected);
            assert_eq!(policy.rungs().trail(), expected);
            assert_eq!(policy.rungs().max_levels_per_axis(), count);
            let original = original_five(side)?;
            assert_eq!(policy.side(), side);
            assert_eq!(policy.ratios(), original.ratios());
            assert_eq!(policy.max_cells(), 16_384);
            assert_eq!(policy.selector(), original.selector());
            assert_eq!(policy.forced_stop(), original.forced_stop());
            assert_eq!(policy.cost_model_id(), original.cost_model_id());
            assert_eq!(
                policy.execution_resolution(),
                original.execution_resolution()
            );
            assert_eq!(policy.range_resolution(), original.range_resolution());
            assert_eq!(policy.max_ambiguous_bars(), original.max_ambiguous_bars());
            assert_eq!(policy.max_gap_fills(), original.max_gap_fills());
            if count == 2 {
                // Independently constructed public-policy source-only census,
                // before this shared CLI wiring change; no outcome-derived cap.
                let expected_digest = match side {
                    Side::Long => [
                        0xf8, 0x78, 0x98, 0xd8, 0xff, 0xc2, 0xe4, 0x1c, 0xb7, 0x7e, 0xea, 0x60,
                        0x43, 0xff, 0xb0, 0x09, 0x29, 0xde, 0x70, 0x6e, 0x8a, 0xfa, 0x92, 0xb6,
                        0x75, 0x78, 0x8f, 0xdc, 0x27, 0x1d, 0xe0, 0x6d,
                    ],
                    Side::Short => [
                        0x84, 0xec, 0xcb, 0xfa, 0x98, 0x09, 0xbf, 0x4a, 0x77, 0xab, 0xac, 0xaf,
                        0xea, 0xee, 0x28, 0x1d, 0xa0, 0x12, 0x35, 0x3f, 0xbb, 0xbf, 0x66, 0x1f,
                        0x95, 0xbc, 0x14, 0x67, 0x37, 0xfa, 0x94, 0xd7,
                    ],
                };
                assert_eq!(policy.digest(), expected_digest);
            }
            assert!(
                digests.insert(policy.digest()),
                "each side/resolution is distinct"
            );
        }
    }
    assert_eq!(digests.len(), (ceiling - 1) * 2);
    Ok(())
}

#[test]
fn invalid_runtime_resolution_refuses_without_default_or_clamp() -> Result<(), String> {
    let _serial = crate::knobs::serially();
    let _clear = ClearKnobs;
    crate::knobs::clear_all();
    let above = crate::rungs_within_cell_budget()
        .checked_add(1)
        .ok_or("bound overflow")?;
    for raw in [
        String::new(),
        " ".to_owned(),
        "0".to_owned(),
        "1".to_owned(),
        "-2".to_owned(),
        "2.0".to_owned(),
        "2x".to_owned(),
        "rung".to_owned(),
        "184467440737095516160".to_owned(),
        above.to_string(),
    ] {
        assert!(!crate::audited_range_command::request_value(
            "BRUTEX_GRID_RUNGS",
            &raw
        ));
        for side in [Side::Long, Side::Short] {
            assert_eq!(
                exit_policy_with(side, Some(&raw)),
                Err("BRUTEX_GRID_RUNGS refused by strict runtime bounds".to_owned())
            );
            // The HTTP knob store deliberately treats an empty override as removal.
            // Empty environment values are covered by the pure resolution above.
            if !raw.trim().is_empty() {
                crate::knobs::set("BRUTEX_GRID_RUNGS", &raw);
                assert_eq!(exit_policy(side), exit_policy_with(side, Some(&raw)));
            }
        }
    }
    assert_eq!(
        exit_policy_with(Side::Long, Some(" 2 "))?,
        exit_policy_with(Side::Long, Some("2"))?
    );
    Ok(())
}

#[test]
fn generated_source_descriptor_binds_both_runtime_exit_policies_independent_of_catalog()
-> Result<(), String> {
    let fixture = candidate::tests::Fixture::new()?;
    fixture.prepare("NIFTY")?;
    let inputs = StrictConfig::from_values(
        Some(fixture.root.as_os_str().to_owned()),
        Some("4194304".into()),
        Some("40000".into()),
    )
    .map_err(debug)?;
    let programs = [vocab::expression::Expression::parse("40").map_err(debug)?];
    let other = [vocab::expression::Expression::parse("30").map_err(debug)?];
    let long = exit_policy_with(Side::Long, None)?;
    let short = exit_policy_with(Side::Short, None)?;
    let long_two = exit_policy_with(Side::Long, Some("2"))?;
    let short_two = exit_policy_with(Side::Short, Some("2"))?;
    let mut request = candidate::Request {
        store: &fixture.root,
        output: &fixture.output,
        vendor: Vendor::Zerodha,
        underlying: "NIFTY",
        rung: "1min",
        from: (2025, 5),
        to: (2025, 5),
        horizon: Horizon::bars(5).ok_or("horizon")?,
        programs: &programs,
        long: &long,
        short: &short,
        inputs: &inputs,
        bounds: candidate::Bounds {
            programs: 1,
            coordinates: 32_768,
            trades: 200_000,
            bytes: 33_554_432,
        },
        widths: Widths::pinned().map_err(debug)?,
        thresholds: Thresholds::CLASSICAL,
    };
    let stamp = "generated-runtime-grid-source-preflight";
    let original = candidate::fingerprint(&request, stamp)?;
    let explicit_long = exit_policy_with(Side::Long, Some("5"))?;
    let explicit_short = exit_policy_with(Side::Short, Some("5"))?;
    request.long = &explicit_long;
    request.short = &explicit_short;
    let explicit = candidate::fingerprint(&request, stamp)?;
    assert_eq!(explicit.identity, original.identity);
    assert_eq!(explicit.descriptor, original.descriptor);
    for (long, short) in [
        (&long_two, &short),
        (&long, &short_two),
        (&long_two, &short_two),
    ] {
        request.long = long;
        request.short = short;
        let changed = candidate::fingerprint(&request, stamp)?;
        assert_ne!(changed.identity, original.identity);
        assert_ne!(changed.descriptor, original.descriptor);
        assert_eq!(
            changed.source, original.source,
            "the raw source did not change"
        );
    }
    request.long = &long;
    request.short = &short;
    request.programs = &other;
    let catalog_changed = candidate::fingerprint(&request, stamp)?;
    assert_ne!(catalog_changed.identity, original.identity);
    assert_eq!(catalog_changed.descriptor, original.descriptor);
    original.require_current()?;
    Ok(())
}
