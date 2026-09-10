//! Invented numeric observations test comparisons, never saved market authority.
#![expect(clippy::unwrap_used, reason = "exact numeric comparison fixtures")]
use super::*;
use crate::boolean_search_record::tests::{generated_policy, generated_policy_with_ceilings};
use runner::admission::{
    AdmissionEvidenceValuesV1, AdmissionReasonV1, CompletenessV1, HypothesisDecisionV1,
    ObservedI64V1,
};
use runner::search_allocation_v1::{self as spending, Refusal};

fn project_values(
    policy: &runner::admission::AdmissionPolicyV1,
    family_probabilities: [[u64; 2]; 2],
    row: QualificationRow,
    allocation: Allocation,
) -> Result<SearchRow, String> {
    super::project_values(
        ProjectionRule::SharedCeilingsV2,
        policy,
        family_probabilities,
        row,
        allocation,
    )
}

fn values() -> AdmissionEvidenceValuesV1 {
    use ObservedI64V1::Measured as I;
    use ObservedU64V1::Measured as U;
    AdmissionEvidenceValuesV1 {
        support_hits: U(100),
        independent_sessions: U(10),
        trades: U(10),
        max_mae_paisa: U(1),
        worst_reward_risk_ppm: U(1_000_000),
        win_rate_ppm: U(600_000),
        wilson_win_rate_ppm: U(0),
        return_drawdown_ppm: U(1_000_000),
        weakest_period_return_paisa: I(0),
        pbo_ppm: U(0),
        fwer_p_value_ppm: U(12_500),
        spa_p_value_ppm: U(12_500),
        decided_folds: U(2),
        ambiguous_fill_rate_ppm: U(0),
        gap_affected_rate_ppm: U(0),
        session_concentration_ppm: U(100_000),
        largest_trade_profit_share_ppm: U(100_000),
        execution_complete: CompletenessV1::Complete,
        data_complete: CompletenessV1::Complete,
        calendar_complete: CompletenessV1::Complete,
        population_complete: CompletenessV1::Complete,
        drawdown_paisa: U(1),
        worst_trade_loss_paisa: U(1),
        losing_trade_rate_ppm: U(400_000),
        losing_trades: U(4),
        pessimistic_profit_paisa: I(2),
        winning_trades: U(6),
        average_win_paisa: U(1),
        average_loss_paisa: U(1),
        profit_factor_ppm: U(1_000_000),
        consecutive_losing_streak: U(1),
        consecutive_winning_streak: U(1),
        bootstrap_draws: U(1000),
        bootstrap_strategies: U(2),
        bootstrap_periods: U(63),
        pbo_contributing_folds: U(2),
        pbo_unrankable_folds: U(0),
        profitable_oos_folds: U(1),
        oos_pessimistic_return_paisa: I(1),
        white_reality_p_value_ppm: U(12_500),
        romano_wolf_p_value_ppm: U(12_500),
        white_reality_decision: HypothesisDecisionV1::RejectedNull,
        romano_wolf_decision: HypothesisDecisionV1::RejectedNull,
        full_precision_statistics_complete: CompletenessV1::Complete,
    }
}

fn generated_row(values: &AdmissionEvidenceValuesV1) -> QualificationRow {
    let verdict = generated_policy()
        .evaluate_research_projection(*values)
        .unwrap()
        .verdict();
    QualificationRow {
        original: brutex_core::blake3::hash(b"generated original coordinate"),
        later: brutex_core::blake3::hash(b"generated later coordinate"),
        family: 1,
        coordinate: 17,
        values: *values,
        verdict,
        romano: [0, 1, 640, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        folds: None,
    }
}

#[test]
fn search_projection_changes_only_exact_probability_fields_and_retains_full_source() {
    let original = generated_row(&values());
    assert_eq!(original.verdict.status(), AdmissionStatusV1::Admitted);
    let result = project_values(
        &generated_policy(),
        [[1, 640], [1, 1280]],
        original.clone(),
        spending::allocate(0, 7, 50_000).unwrap(),
    )
    .unwrap();
    assert_eq!(result.source, original);
    assert_eq!(
        result.projection.policy_digest(),
        generated_policy().digest()
    );
    let mut expected = original.values;
    expected.fwer_p_value_ppm = ObservedU64V1::Measured(25_000);
    expected.romano_wolf_p_value_ppm = ObservedU64V1::Measured(25_000);
    expected.white_reality_p_value_ppm = ObservedU64V1::Measured(25_000);
    expected.spa_p_value_ppm = ObservedU64V1::Measured(12_500);
    assert_eq!(result.projection.values(), expected);
    assert_eq!(
        result.projection.verdict().status(),
        AdmissionStatusV1::Admitted
    );
    assert_eq!(
        result
            .probabilities
            .map(|p| (p.numerator(), p.denominator())),
        [(1, 40), (1, 40), (1, 80)]
    );
    assert_eq!(
        result.projection.canonical_bytes(),
        generated_policy()
            .evaluate_research_projection(expected)
            .unwrap()
            .canonical_bytes()
    );
    let legacy = super::project_values(
        ProjectionRule::LegacyV1,
        &generated_policy(),
        [[1, 640], [1, 1280]],
        original,
        spending::allocate(0, 7, 50_000).unwrap(),
    )
    .unwrap();
    assert_eq!(
        legacy.projection.canonical_bytes(),
        result.projection.canonical_bytes()
    );
}

#[test]
fn equality_passes_but_fractional_ppm_excess_never_rounds_into_admission() {
    for (n, d, ppm, accepted) in [
        (1, 320, 50_000, true),
        (1_000_001, 320_000_000, 50_001, false),
        (1, 321, 49_845, true),
    ] {
        let mut row = generated_row(&values());
        row.romano[1] = n;
        row.romano[2] = d;
        let result = project_values(
            &generated_policy(),
            [[n, d], [n, d]],
            row,
            spending::allocate(0, 0, 50_000).unwrap(),
        )
        .unwrap();
        assert_eq!(
            result.projection.values().fwer_p_value_ppm,
            ObservedU64V1::Measured(ppm)
        );
        assert_eq!(
            result.projection.values().white_reality_p_value_ppm,
            ObservedU64V1::Measured(ppm)
        );
        assert_eq!(
            result.projection.values().spa_p_value_ppm,
            ObservedU64V1::Measured(ppm)
        );
        assert_eq!(result.projection.verdict().is_admitted(), accepted);
        assert_eq!(
            result.projection.values().romano_wolf_decision,
            if accepted {
                HypothesisDecisionV1::RejectedNull
            } else {
                HypothesisDecisionV1::DidNotReject
            }
        );
        if !accepted {
            assert!(
                result
                    .projection
                    .verdict()
                    .failed()
                    .contains(AdmissionReasonV1::Fwer)
            );
        }
    }
}

#[test]
fn missing_fold_and_execution_authority_is_never_replaced_by_numeric_success() {
    let mut source = values();
    source.decided_folds = ObservedU64V1::Unmeasured;
    source.profitable_oos_folds = ObservedU64V1::Unmeasured;
    source.oos_pessimistic_return_paisa = ObservedI64V1::Unmeasured;
    source.full_precision_statistics_complete = CompletenessV1::Unmeasured;
    let original = generated_row(&source);
    let result = project_values(
        &generated_policy(),
        [[1, 640], [1, 640]],
        original.clone(),
        spending::allocate(0, 0, 50_000).unwrap(),
    )
    .unwrap();
    assert_eq!(result.source, original);
    assert_eq!(
        result.projection.values().decided_folds,
        ObservedU64V1::Unmeasured
    );
    assert_eq!(
        result.projection.values().profitable_oos_folds,
        ObservedU64V1::Unmeasured
    );
    assert_eq!(
        result.projection.values().oos_pessimistic_return_paisa,
        ObservedI64V1::Unmeasured
    );
    assert_eq!(
        result
            .projection
            .values()
            .full_precision_statistics_complete,
        CompletenessV1::Unmeasured
    );
    assert_eq!(
        result.projection.verdict().status(),
        AdmissionStatusV1::Unmeasured
    );
    assert!(
        result
            .projection
            .verdict()
            .unmeasured()
            .contains(AdmissionReasonV1::DecidedFolds)
    );
    source.data_complete = CompletenessV1::Refused;
    let result = project_values(
        &generated_policy(),
        [[1, 640], [1, 640]],
        generated_row(&source),
        spending::allocate(0, 0, 50_000).unwrap(),
    )
    .unwrap();
    assert_eq!(
        result.projection.verdict().status(),
        AdmissionStatusV1::Refused
    );
    assert_eq!(
        result.projection.values().data_complete,
        CompletenessV1::Refused
    );
}

#[test]
fn unreachable_draw_resolution_keeps_evidence_and_draws_without_positive_admission() {
    let allocation = spending::allocate(2, 0, 50_000).unwrap();
    assert_eq!(allocation.require_draws(1000), Err(Refusal::Resolution));
    let mut row = generated_row(&values());
    row.romano[2] = 1001;
    let result =
        project_values(&generated_policy(), [[1, 1001], [1, 1001]], row, allocation).unwrap();
    assert_eq!(
        result.projection.values().bootstrap_draws,
        ObservedU64V1::Measured(1000)
    );
    assert_eq!(
        result.projection.verdict().status(),
        AdmissionStatusV1::Rejected
    );
    assert_eq!(
        result
            .probabilities
            .map(|p| (p.numerator(), p.denominator())),
        [(96, 1001); 3]
    );
    assert_eq!(
        result.projection.values().fwer_p_value_ppm,
        ObservedU64V1::Measured(95_905)
    );
}

#[test]
fn full_probability_one_is_retained_and_all_three_families_can_independently_reject() {
    for position in 0..3 {
        let mut row = generated_row(&values());
        let mut family = [[1, 640]; 2];
        match position {
            0 => row.romano[2] = 1,
            1 => *family.first_mut().unwrap() = [1, 1],
            _ => *family.last_mut().unwrap() = [1, 1],
        }
        let result = project_values(
            &generated_policy(),
            family,
            row.clone(),
            spending::allocate(0, 0, 50_000).unwrap(),
        )
        .unwrap();
        assert_eq!(result.source, row);
        assert_eq!(result.probabilities.get(position).unwrap().numerator(), 1);
        assert_eq!(result.probabilities.get(position).unwrap().denominator(), 1);
        assert_eq!(
            result.projection.verdict().status(),
            AdmissionStatusV1::Rejected
        );
    }
}

#[test]
fn malformed_probabilities_and_exact_arithmetic_failure_refuse_without_partial_projection() {
    for (n, d) in [(0, 0), (2, 1)] {
        for position in 0..3 {
            let mut row = generated_row(&values());
            let mut family = [[1, 640]; 2];
            match position {
                0 => {
                    row.romano[1] = n;
                    row.romano[2] = d;
                }
                1 => *family.first_mut().unwrap() = [n, d],
                _ => *family.last_mut().unwrap() = [n, d],
            }
            assert_eq!(
                project_values(
                    &generated_policy(),
                    family,
                    row,
                    spending::allocate(0, 0, 50_000).unwrap()
                )
                .err()
                .unwrap(),
                "Probability"
            );
        }
    }
    let mut row = generated_row(&values());
    row.romano[1] = u64::MAX - 1;
    row.romano[2] = u64::MAX;
    assert_eq!(
        project_values(
            &generated_policy(),
            [[1, 640]; 2],
            row,
            spending::allocate(1_000_000_000_000_000, 0, 1).unwrap()
        )
        .err()
        .unwrap(),
        "Arithmetic"
    );
}

#[test]
fn inconsistent_detached_source_verdict_cannot_be_promoted() {
    // Defensive observed-value fault injection, not a constructible authenticated
    // Qualification: the supplied verdict disagrees with otherwise passing values.
    let mut rejected = values();
    rejected.support_hits = ObservedU64V1::Measured(0);
    rejected.independent_sessions = ObservedU64V1::Measured(0);
    let mut row = generated_row(&values());
    row.verdict = generated_policy()
        .evaluate_research_projection(rejected)
        .unwrap()
        .verdict();
    assert_eq!(row.verdict.status(), AdmissionStatusV1::Rejected);
    assert_eq!(
        project_values(
            &generated_policy(),
            [[1, 640]; 2],
            row,
            spending::allocate(0, 0, 50_000).unwrap()
        )
        .err()
        .unwrap(),
        "search-wide correction unexpectedly relaxed original qualification"
    );
}

#[test]
fn stricter_shared_search_allowance_cannot_be_relaxed_by_individual_policy_ceilings() {
    // The actual writer uses min(FWER, Romano), which is 2.5% here.
    // White/SPA still permit 5% individually. Their 4% corrected probability
    // escaped V1's shared guard, which checked only Romano. V2 must reject it.
    let policy = generated_policy_with_ceilings([50_000, 50_000, 50_000, 25_000]);
    let row = generated_row(&values());
    assert_eq!(row.verdict.status(), AdmissionStatusV1::Admitted);
    let ceilings = policy.values();
    let allocation = spending::allocate(
        0,
        0,
        ceilings
            .max_fwer_p_value_ppm
            .min(ceilings.max_romano_wolf_p_value_ppm),
    )
    .unwrap();
    assert!(!allocation.compare(1, 400).unwrap());
    let legacy = super::project_values(
        ProjectionRule::LegacyV1,
        &policy,
        [[1, 400]; 2],
        row.clone(),
        allocation,
    )
    .unwrap();
    assert_eq!(
        legacy.projection.verdict().status(),
        AdmissionStatusV1::Admitted
    );
    assert_eq!(legacy.projection.policy_digest(), policy.digest());
    let result = project_values(&policy, [[1, 400]; 2], row, allocation).unwrap();
    assert_eq!(
        result.projection.verdict().status(),
        AdmissionStatusV1::Rejected
    );
    assert!(
        result
            .projection
            .verdict()
            .failed()
            .contains(AdmissionReasonV1::WhiteRealityPValue)
    );
    assert!(
        result
            .projection
            .verdict()
            .failed()
            .contains(AdmissionReasonV1::Spa)
    );
}

#[test]
fn legacy_probability_guard_remains_exact_for_inconsistent_detached_allocations() {
    // Deliberately inconsistent detached alpha: an actual V1 plan would derive
    // 50,000 here. Preserve the old defensive guard; this is not a production
    // configuration or a claim that valid legacy batches hit this error.
    let mut original = generated_row(&values());
    original.romano[1] = 1;
    original.romano[2] = 400;
    assert_eq!(
        super::project_values(
            ProjectionRule::LegacyV1,
            &generated_policy(),
            [[1, 640]; 2],
            original,
            spending::allocate(0, 0, 25_000).unwrap(),
        )
        .err()
        .unwrap(),
        "search-wide correction unexpectedly relaxed original qualification"
    );
}

#[test]
fn exact_probability_projection_refuses_unrepresentable_integer_boundaries() {
    let wide_denominator = spending::allocate(10_000_000_000, 0, 50_000)
        .unwrap()
        .threshold();
    assert!(wide_denominator.denominator() > u128::from(u64::MAX));
    assert!(exact(wide_denominator).is_err());
    assert_eq!(upper_ppm(wide_denominator).unwrap(), 1);
    let allocation = spending::allocate(0, 0, 50_000).unwrap();
    for (numerator, denominator, expected) in [(0, 1, 0), (1, 16, 1_000_000), (1, 48, 333_334)] {
        let fraction = allocation
            .scaled_probability(numerator, denominator)
            .unwrap();
        assert!(exact(fraction).is_ok());
        assert_eq!(upper_ppm(fraction).unwrap(), expected);
    }
}

#[test]
fn shared_probability_cap_preserves_every_other_policy_value_and_never_loosens_a_ceiling() {
    let original = generated_policy_with_ceilings([50_000, 25_000, 10_000, 40_000]);
    let original_bytes = original.canonical_bytes();
    for (cap, expected) in [
        (0, [0, 0, 0, 0]),
        (1, [1, 1, 1, 1]),
        (25_000, [25_000, 25_000, 10_000, 25_000]),
        (50_000, [50_000, 25_000, 10_000, 40_000]),
        (u64::MAX, [50_000, 25_000, 10_000, 40_000]),
    ] {
        let capped = original.with_search_probability_ceiling(cap);
        let expected = generated_policy_with_ceilings(expected);
        assert_eq!(capped.canonical_bytes(), expected.canonical_bytes());
        assert_eq!(capped.digest(), expected.digest());
        assert_eq!(original.canonical_bytes(), original_bytes);
        assert_eq!(
            capped
                .with_search_probability_ceiling(cap)
                .canonical_bytes(),
            capped.canonical_bytes(),
            "reapplying the same cap is idempotent"
        );
    }
}

#[test]
fn unequal_family_ceilings_keep_exact_boundary_failures_as_saved_rejections() {
    for ceilings in [
        [50_000, 50_000, 50_000, 25_000],
        [25_000, 50_000, 50_000, 50_000],
    ] {
        let policy = generated_policy_with_ceilings(ceilings);
        for family in 0..3 {
            for (n, d, admitted) in [
                (1, 640, true),
                (1_000_001, 640_000_000, false),
                (1, 641, true),
            ] {
                let mut original = generated_row(&values());
                let mut families = [[1, 640]; 2];
                match family {
                    0 => {
                        original.romano[1] = n;
                        original.romano[2] = d;
                    }
                    1 => *families.first_mut().unwrap() = [n, d],
                    _ => *families.last_mut().unwrap() = [n, d],
                }
                let result = project_values(
                    &policy,
                    families,
                    original.clone(),
                    spending::allocate(0, 0, 25_000).unwrap(),
                )
                .unwrap();
                assert_eq!(result.source, original);
                assert_eq!(result.projection.verdict().is_admitted(), admitted);
                assert_eq!(
                    result.projection.policy_digest(),
                    generated_policy_with_ceilings([25_000; 4]).digest()
                );
                if !admitted {
                    let reason = match family {
                        0 => AdmissionReasonV1::Fwer,
                        1 => AdmissionReasonV1::WhiteRealityPValue,
                        _ => AdmissionReasonV1::Spa,
                    };
                    assert!(result.projection.verdict().failed().contains(reason));
                }
            }
        }
    }
}

#[test]
fn zero_shared_allowance_rejects_positive_probability_without_aborting_the_search() {
    let source = generated_row(&values());
    let result = project_values(
        &generated_policy(),
        [[1, 640]; 2],
        source.clone(),
        spending::allocate(0, 0, 0).unwrap(),
    )
    .unwrap();
    assert_eq!(result.source, source);
    assert_eq!(
        result.projection.verdict().status(),
        AdmissionStatusV1::Rejected
    );
    assert_eq!(
        result.projection.policy_digest(),
        generated_policy_with_ceilings([0; 4]).digest()
    );
    assert!(
        result
            .projection
            .verdict()
            .failed()
            .contains(AdmissionReasonV1::Fwer)
    );
}
