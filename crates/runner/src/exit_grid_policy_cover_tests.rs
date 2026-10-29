#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests state their expectations"
)]
//! BASE-COV-LIBS: the exit-grid policy's three constructors refuse every
//! zero bound, inverted or non-positive ratio and non-positive forced stop by
//! name, and a built policy reads back every decision it was given.
//!
//! Extreme cases: a zero level bound with otherwise valid axes; `i64::MIN`,
//! zero and negative ratio bounds; an inverted ratio interval; `i64::MAX` as
//! both bounds (the widest single point); a zero pair ceiling; `u64::MAX`
//! cells; a forced stop of zero, `-1` and `i64::MIN` in both forms;
//! combinations: zero cells with a zero cost model (cells named, the first
//! check) and an inverted ratio with a zero pair ceiling (the ratio named);
//! and, out of the box, the largest positive forced stop, which is accepted.

use super::*;

fn p(numerator: u32, denominator: u32) -> RationalPercentileV1 {
    RationalPercentileV1::new(numerator, denominator).unwrap()
}

fn rungs() -> RungPlanV1 {
    RungPlanV1::new(
        vec![p(1, 4), p(1, 2)],
        vec![p(3, 4), p(1, 1)],
        vec![p(1, 4)],
        4,
    )
    .unwrap()
}

fn ratios() -> RatioLimitsV1 {
    RatioLimitsV1::new(100, 1_000, 16).unwrap()
}

fn policy(
    max_cells: u64,
    cost_model_id: [u8; 32],
    forced_stop: ForcedStopV1,
) -> Result<ExitGridPolicyV1, ExitGridErrorV1> {
    ExitGridPolicyV1::new(
        ExecutionResolutionV1::OneMinuteOhlcv,
        RangeResolutionV1::PpmFloor,
        crate::excursion::Side::Short,
        rungs(),
        ratios(),
        max_cells,
        ExitGridSelectorV1::EdgeThenPessimistic,
        cost_model_id,
        forced_stop,
        7,
        5,
    )
}

#[test]
fn a_rung_plan_needs_a_positive_level_bound() {
    assert_eq!(
        RungPlanV1::new(vec![p(1, 2)], vec![p(1, 2)], vec![p(1, 2)], 0).err(),
        Some(ExitGridErrorV1::ZeroLimit("max_levels_per_axis"))
    );
    // A zero bound with every axis empty too: the bound is named first.
    assert_eq!(
        RungPlanV1::new(vec![], vec![], vec![], 0).err(),
        Some(ExitGridErrorV1::ZeroLimit("max_levels_per_axis"))
    );
    let plan = RungPlanV1::new(vec![p(1, 2)], vec![p(1, 2)], vec![p(1, 2)], usize::MAX).unwrap();
    assert_eq!(plan.max_levels_per_axis(), usize::MAX);
}

#[test]
fn ratio_limits_are_a_positive_ordered_interval_with_a_pair_ceiling() {
    for (min, max) in [
        (0, 100),
        (-1, 100),
        (i64::MIN, i64::MAX),
        (200, 199),
        (i64::MAX, 1),
    ] {
        assert_eq!(
            RatioLimitsV1::new(min, max, 4).err(),
            Some(ExitGridErrorV1::InvalidRatioLimits {
                min_hundredths: min,
                max_hundredths: max,
            }),
            "{min} {max}"
        );
    }
    // Inverted and a zero ceiling at once: the interval is named.
    assert_eq!(
        RatioLimitsV1::new(5, 4, 0).err(),
        Some(ExitGridErrorV1::InvalidRatioLimits {
            min_hundredths: 5,
            max_hundredths: 4,
        })
    );
    assert_eq!(
        RatioLimitsV1::new(100, 100, 0).err(),
        Some(ExitGridErrorV1::ZeroLimit("max_ratio_pairs"))
    );
    let point = RatioLimitsV1::new(i64::MAX, i64::MAX, u64::MAX).unwrap();
    assert_eq!(point.max_pairs(), u64::MAX);
}

#[test]
fn a_policy_refuses_zero_cells_and_a_non_positive_forced_stop_by_name() {
    let id = printed_ohlcv_cost_model_id_v2();
    assert_eq!(
        policy(0, id, ForcedStopV1::Disabled).err(),
        Some(ExitGridErrorV1::ZeroLimit("max_cells"))
    );
    assert_eq!(
        policy(0, [0; 32], ForcedStopV1::Disabled).err(),
        Some(ExitGridErrorV1::ZeroLimit("max_cells"))
    );
    for ppm in [0, -1, i64::MIN] {
        for forced in [
            ForcedStopV1::IncludeExactObserved(ppm),
            ForcedStopV1::RequireExactObserved(ppm),
        ] {
            assert_eq!(
                policy(1, id, forced).err(),
                Some(ExitGridErrorV1::InvalidForcedStop(ppm)),
                "{forced:?}"
            );
        }
    }
    assert!(policy(1, id, ForcedStopV1::RequireExactObserved(i64::MAX)).is_ok());
    assert!(policy(1, id, ForcedStopV1::IncludeExactObserved(1)).is_ok());
}

#[test]
fn a_built_policy_reads_back_every_decision_it_was_given() {
    let id = printed_ohlcv_cost_model_id_v2();
    let built = policy(u64::MAX, id, ForcedStopV1::IncludeExactObserved(250)).unwrap();
    assert_eq!(
        built.execution_resolution(),
        ExecutionResolutionV1::OneMinuteOhlcv
    );
    assert_eq!(built.range_resolution(), RangeResolutionV1::PpmFloor);
    assert_eq!(built.side(), crate::excursion::Side::Short);
    assert_eq!(built.rungs(), &rungs());
    assert_eq!(built.ratios(), ratios());
    assert_eq!(built.max_cells(), u64::MAX);
    assert_eq!(built.selector(), ExitGridSelectorV1::EdgeThenPessimistic);
    assert_eq!(built.cost_model_id(), id);
    assert_eq!(built.max_ambiguous_bars(), 7);
    assert_eq!(built.max_gap_fills(), 5);
}
