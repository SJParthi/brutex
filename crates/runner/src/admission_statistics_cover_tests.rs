#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::type_complexity,
    reason = "tests state their expectations"
)]
//! BASE-COV-LIBS: every refusal of the detached Statistics V2 and V3 field
//! reconciliation and of the walk-forward authorities' canonical fields, each
//! driven by the one field that breaks it, and every accessor read back.
//!
//! Extreme cases: a zero identity in each of the thirteen V2 and nine V3
//! positions; `u64::MAX` bootstrap draws (the denominator overflows); zero
//! draws, strategies and a single period; every finite resample probability
//! with a zero numerator and with a denominator one off; a fold count of zero;
//! decided and profitable folds one past their bound; combinations: every
//! identity zero at once (the first is named) and a zero fold count together
//! with a zero identity (the identity is named, the earlier check); and, out of
//! the box, an authority id that is the derived id of a DIFFERENT walk.

use super::{
    AdmissionExactProbabilityV2, AdmissionStatisticsDraftV2, AdmissionStatisticsDraftV3,
    AdmissionStatisticsFieldsV2, AdmissionStatisticsFieldsV3, AdmissionStatisticsIdentityV2,
    AdmissionStatisticsIdentityV3, AdmissionStatisticsProbabilityV2, AdmissionStatisticsRefusalV2,
    AdmissionStatisticsRefusalV3, AnchoredWalkForwardAuthorityV2, AnchoredWalkForwardAuthorityV3,
    AnchoredWalkForwardRefusalV2, AnchoredWalkForwardRefusalV3, canonical_wilson_projection_v2,
};

fn p(numerator: u64, denominator: u64) -> AdmissionExactProbabilityV2 {
    AdmissionExactProbabilityV2::new(numerator, denominator).unwrap()
}

/// The module tests' own V2 fixture, rebuilt here (theirs is private to them).
fn v2() -> AdmissionStatisticsDraftV2 {
    let (wilson_lower_bits, wilson_win_rate_ppm) = canonical_wilson_projection_v2(30, 50);
    AdmissionStatisticsDraftV2 {
        candidate_semantic_id: [1; 32],
        population_search_id: [2; 32],
        ranking_validation_policy_digest: [13; 32],
        source_policy_digest: [3; 32],
        finalization_family_digest: [4; 32],
        statistics_audit_id: [5; 32],
        statistics_completion_digest: [6; 32],
        observation_statistics_link_id: [7; 32],
        cscv_policy_digest: [8; 32],
        cscv_split_family_digest: [9; 32],
        white_family_digest: [10; 32],
        spa_family_digest: [11; 32],
        romano_wolf_family_digest: [12; 32],
        trades: 50,
        wins: 30,
        wilson_lower_bits,
        wilson_win_rate_ppm,
        cscv_split_count: 12,
        pbo_contributing_splits: 10,
        pbo_unrankable_splits: 2,
        pbo_probability: p(1, 10),
        white_probability: p(50, 1_001),
        spa_probability: p(49, 1_001),
        familywise_romano_wolf_probability: p(49, 1_001),
        candidate_romano_wolf_probability: p(50, 1_001),
        bootstrap_draws: 1_000,
        bootstrap_strategies: 100,
        bootstrap_periods: 300,
    }
}

fn v3() -> AdmissionStatisticsDraftV3 {
    let l = v2();
    AdmissionStatisticsDraftV3 {
        candidate_semantic_id: l.candidate_semantic_id,
        statistics_audit_id: l.statistics_audit_id,
        statistics_completion_digest: l.statistics_completion_digest,
        observation_statistics_link_id: l.observation_statistics_link_id,
        cscv_policy_digest: l.cscv_policy_digest,
        cscv_split_family_digest: l.cscv_split_family_digest,
        white_family_digest: l.white_family_digest,
        spa_family_digest: l.spa_family_digest,
        romano_wolf_family_digest: l.romano_wolf_family_digest,
        trades: l.trades,
        wins: l.wins,
        wilson_lower_bits: l.wilson_lower_bits,
        wilson_win_rate_ppm: l.wilson_win_rate_ppm,
        cscv_split_count: l.cscv_split_count,
        pbo_contributing_splits: l.pbo_contributing_splits,
        pbo_unrankable_splits: l.pbo_unrankable_splits,
        pbo_probability: l.pbo_probability,
        white_probability: l.white_probability,
        spa_probability: l.spa_probability,
        familywise_romano_wolf_probability: l.familywise_romano_wolf_probability,
        candidate_romano_wolf_probability: l.candidate_romano_wolf_probability,
        bootstrap_draws: l.bootstrap_draws,
        bootstrap_strategies: l.bootstrap_strategies,
        bootstrap_periods: l.bootstrap_periods,
    }
}

#[test]
fn a_v2_statistics_draft_is_refused_by_its_first_broken_field() {
    use AdmissionStatisticsIdentityV2 as I;
    use AdmissionStatisticsProbabilityV2 as P;
    use AdmissionStatisticsRefusalV2 as R;
    let check = |edit: &dyn Fn(&mut AdmissionStatisticsDraftV2)| {
        let mut draft = v2();
        edit(&mut draft);
        AdmissionStatisticsFieldsV2::verify_detached(&draft).err()
    };
    let fields = AdmissionStatisticsFieldsV2::verify_detached(&v2()).unwrap();
    assert_eq!(fields.values(), v2());
    assert_eq!(fields.candidate_semantic_id(), [1; 32]);
    assert_eq!(fields.population_search_id(), [2; 32]);
    assert_eq!(fields.ranking_validation_policy_digest(), [13; 32]);
    assert_eq!(fields.source_policy_digest(), [3; 32]);
    assert_eq!(fields.finalization_family_digest(), [4; 32]);
    assert_eq!(fields.statistics_audit_id(), [5; 32]);

    let zero: [(fn(&mut AdmissionStatisticsDraftV2), I); 13] = [
        (|d| d.candidate_semantic_id = [0; 32], I::CandidateSemantic),
        (|d| d.population_search_id = [0; 32], I::PopulationSearch),
        (
            |d| d.ranking_validation_policy_digest = [0; 32],
            I::RankingValidationPolicy,
        ),
        (|d| d.source_policy_digest = [0; 32], I::SourcePolicy),
        (
            |d| d.finalization_family_digest = [0; 32],
            I::FinalizationFamily,
        ),
        (|d| d.statistics_audit_id = [0; 32], I::StatisticsAudit),
        (
            |d| d.statistics_completion_digest = [0; 32],
            I::StatisticsCompletion,
        ),
        (
            |d| d.observation_statistics_link_id = [0; 32],
            I::ObservationStatisticsLink,
        ),
        (|d| d.cscv_policy_digest = [0; 32], I::CscvPolicy),
        (|d| d.cscv_split_family_digest = [0; 32], I::CscvSplitFamily),
        (|d| d.white_family_digest = [0; 32], I::WhiteFamily),
        (|d| d.spa_family_digest = [0; 32], I::SpaFamily),
        (
            |d| d.romano_wolf_family_digest = [0; 32],
            I::RomanoWolfFamily,
        ),
    ];
    for (edit, name) in zero {
        assert_eq!(check(&edit), Some(R::ZeroIdentity(name)), "{name:?}");
    }
    // Every identity zero and every count broken at once: the first identity.
    assert_eq!(
        check(&|d| {
            for (edit, _) in zero {
                edit(d);
            }
            d.wins = u64::MAX;
            d.bootstrap_draws = 0;
        }),
        Some(R::ZeroIdentity(I::CandidateSemantic))
    );
    assert_eq!(check(&|d| d.wins = 51), Some(R::WinningTradesExceedTrades));
    assert_eq!(
        check(&|d| d.wilson_win_rate_ppm += 1),
        Some(R::WilsonProjection)
    );
    assert_eq!(check(&|d| d.bootstrap_draws = 0), Some(R::BootstrapShape));
    assert_eq!(
        check(&|d| d.bootstrap_strategies = 0),
        Some(R::BootstrapShape)
    );
    assert_eq!(check(&|d| d.bootstrap_periods = 1), Some(R::BootstrapShape));
    assert_eq!(
        check(&|d| d.bootstrap_draws = u64::MAX),
        Some(R::DrawDenominatorOverflow)
    );
    assert_eq!(
        check(&|d| d.white_probability = p(0, 1_001)),
        Some(R::ZeroFiniteResampleNumerator(P::WhiteReality))
    );
    assert_eq!(
        check(&|d| d.candidate_romano_wolf_probability = p(0, 1_001)),
        Some(R::ZeroFiniteResampleNumerator(P::CandidateRomanoWolf))
    );
    assert_eq!(
        check(&|d| d.spa_probability = p(49, 1_000)),
        Some(R::FiniteResampleDenominator {
            field: P::Spa,
            expected: 1_001,
            actual: 1_000,
        })
    );
    assert_eq!(
        check(&|d| d.familywise_romano_wolf_probability = p(49, 1_002)),
        Some(R::FiniteResampleDenominator {
            field: P::FamilywiseRomanoWolf,
            expected: 1_001,
            actual: 1_002,
        })
    );
    assert_eq!(
        check(&|d| d.pbo_contributing_splits = 0),
        Some(R::CscvCounts)
    );
    assert_eq!(check(&|d| d.pbo_unrankable_splits = 3), Some(R::CscvCounts));
    assert_eq!(
        check(&|d| d.pbo_unrankable_splits = u64::MAX),
        Some(R::CscvCounts)
    );
    assert_eq!(
        check(&|d| d.pbo_probability = p(1, 11)),
        Some(R::CscvCounts)
    );
    assert_eq!(
        check(&|d| d.familywise_romano_wolf_probability = p(51, 1_001)),
        Some(R::RomanoWolfOrdering)
    );
}

#[test]
fn a_v3_statistics_draft_is_refused_by_its_first_broken_field() {
    use AdmissionStatisticsIdentityV3 as I;
    use AdmissionStatisticsProbabilityV2 as P;
    use AdmissionStatisticsRefusalV3 as R;
    let check = |edit: &dyn Fn(&mut AdmissionStatisticsDraftV3)| {
        let mut draft = v3();
        edit(&mut draft);
        AdmissionStatisticsFieldsV3::verify_detached(&draft).err()
    };
    let fields = AdmissionStatisticsFieldsV3::verify_detached(&v3()).unwrap();
    assert_eq!(fields.values(), v3());
    assert_eq!(fields.candidate_semantic_id(), [1; 32]);
    assert_eq!(fields.statistics_audit_id(), [5; 32]);

    let zero: [(fn(&mut AdmissionStatisticsDraftV3), I); 9] = [
        (|d| d.candidate_semantic_id = [0; 32], I::CandidateSemantic),
        (|d| d.statistics_audit_id = [0; 32], I::StatisticsAudit),
        (
            |d| d.statistics_completion_digest = [0; 32],
            I::StatisticsCompletion,
        ),
        (
            |d| d.observation_statistics_link_id = [0; 32],
            I::ObservationStatisticsLink,
        ),
        (|d| d.cscv_policy_digest = [0; 32], I::CscvPolicy),
        (|d| d.cscv_split_family_digest = [0; 32], I::CscvSplitFamily),
        (|d| d.white_family_digest = [0; 32], I::WhiteFamily),
        (|d| d.spa_family_digest = [0; 32], I::SpaFamily),
        (
            |d| d.romano_wolf_family_digest = [0; 32],
            I::RomanoWolfFamily,
        ),
    ];
    for (edit, name) in zero {
        assert_eq!(check(&edit), Some(R::ZeroIdentity(name)), "{name:?}");
    }
    assert_eq!(check(&|d| d.wins = 51), Some(R::WinningTradesExceedTrades));
    assert_eq!(
        check(&|d| d.wilson_lower_bits ^= 1),
        Some(R::WilsonProjection)
    );
    assert_eq!(check(&|d| d.bootstrap_draws = 0), Some(R::BootstrapShape));
    assert_eq!(
        check(&|d| d.bootstrap_strategies = 0),
        Some(R::BootstrapShape)
    );
    assert_eq!(check(&|d| d.bootstrap_periods = 1), Some(R::BootstrapShape));
    assert_eq!(
        check(&|d| d.bootstrap_draws = u64::MAX),
        Some(R::DrawDenominatorOverflow)
    );
    assert_eq!(
        check(&|d| d.spa_probability = p(0, 1_001)),
        Some(R::ZeroFiniteResampleNumerator(P::Spa))
    );
    assert_eq!(
        check(&|d| d.white_probability = p(50, 2_000)),
        Some(R::FiniteResampleDenominator {
            field: P::WhiteReality,
            expected: 1_001,
            actual: 2_000,
        })
    );
    assert_eq!(
        check(&|d| d.pbo_contributing_splits = 0),
        Some(R::CscvCounts)
    );
    assert_eq!(check(&|d| d.cscv_split_count = 13), Some(R::CscvCounts));
    assert_eq!(check(&|d| d.pbo_probability = p(1, 9)), Some(R::CscvCounts));
    assert_eq!(
        check(&|d| d.familywise_romano_wolf_probability = p(51, 1_001)),
        Some(R::RomanoWolfOrdering)
    );
}

/// The walk-forward authorities of V2 and V3 hold the same fields and rules:
/// one body of tests for both.
macro_rules! walk_forward_tests {
    ($module:ident, $authority:ident, $refusal:ident) => {
        mod $module {
            use super::*;

            fn walk(fold_count: u64, decided: u64, profitable: u64, oos: i64) -> $authority {
                $authority {
                    validation_policy_digest: [1; 32],
                    validation_family_digest: [2; 32],
                    walk_facts_digest: [3; 32],
                    authority_id: [0; 32],
                    fold_count,
                    decided_folds: decided,
                    profitable_oos_folds: profitable,
                    aggregate_oos_paisa: oos,
                }
                .with_derived_authority_id()
            }

            fn check(value: $authority) -> Result<$authority, $refusal> {
                $authority::from_canonical_fields(value)
            }

            #[test]
            fn canonical_fields_keep_their_identities_counts_and_outcomes_in_order() {
                let good = walk(5, 4, 2, 100);
                assert_eq!(check(good), Ok(good));
                assert_eq!(good.validation_policy_digest(), [1; 32]);
                assert_eq!(good.validation_family_digest(), [2; 32]);
                assert_eq!(good.walk_facts_digest(), [3; 32]);
                assert_eq!(good.fold_count(), 5);
                assert_ne!(good.authority_id, [0; 32]);

                let zeroed: [fn(&mut $authority); 4] = [
                    |w| w.validation_policy_digest = [0; 32],
                    |w| w.validation_family_digest = [0; 32],
                    |w| w.walk_facts_digest = [0; 32],
                    |w| w.authority_id = [0; 32],
                ];
                for edit in zeroed {
                    let mut w = good;
                    edit(&mut w);
                    assert_eq!(check(w), Err($refusal::ZeroIdentity));
                }
                // A zero identity and a zero fold count at once: the identity.
                let mut both = walk(0, 0, 0, 0);
                both.walk_facts_digest = [0; 32];
                assert_eq!(check(both), Err($refusal::ZeroIdentity));

                for (folds, decided, profitable) in [(0, 0, 0), (3, 4, 0), (5, 2, 3)] {
                    assert_eq!(
                        check(walk(folds, decided, profitable, 0)),
                        Err($refusal::CountHierarchy),
                        "{folds} {decided} {profitable}"
                    );
                }
                for (decided, profitable, oos) in [
                    // Nothing decided, yet an out-of-sample figure.
                    (0, 0, 7),
                    (0, 0, -7),
                    // Nothing profitable, yet a gain.
                    (3, 0, 1),
                    // Every decided fold profitable, yet no gain.
                    // One fold decided, profitable, and no gain: the smallest such walk.
                    (1, 1, 0),
                    (3, 3, 0),
                    (3, 3, i64::MIN),
                ] {
                    assert_eq!(
                        check(walk(5, decided, profitable, oos)),
                        Err($refusal::OutcomeHierarchy),
                        "{decided} {profitable} {oos}"
                    );
                }
                for (decided, profitable, oos) in [
                    (0, 0, 0),
                    (3, 0, 0),
                    (3, 0, -9),
                    (3, 3, 1),
                    (3, 1, i64::MAX),
                    (5, 5, i64::MAX),
                ] {
                    let w = walk(5, decided, profitable, oos);
                    assert_eq!(check(w), Ok(w), "{decided} {profitable} {oos}");
                }
                // Out of the box: the derived id of a different walk.
                let mut foreign = good;
                foreign.authority_id = walk(5, 4, 2, 101).authority_id;
                assert_eq!(check(foreign), Err($refusal::AuthorityIdentityMismatch));
            }
        }
    };
}

walk_forward_tests!(
    walk_v2,
    AnchoredWalkForwardAuthorityV2,
    AnchoredWalkForwardRefusalV2
);
walk_forward_tests!(
    walk_v3,
    AnchoredWalkForwardAuthorityV3,
    AnchoredWalkForwardRefusalV3
);
