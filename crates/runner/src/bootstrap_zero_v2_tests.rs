#![cfg(test)]
//! Generated numeric evidence only; no market or finite-sample validity claim.
#![expect(clippy::unwrap_used, reason = "bounded numeric assertions")]
#![allow(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    reason = "independent statistical reference arithmetic; bounded three-period fixtures"
)]
use super::*;
use crate::bootstrap::{spa_receipt_v1, white_reality_check_receipt_v1};
const BOUNDS: Bounds = Bounds {
    max_work: 1_000_000,
    max_bytes: 64 * 1024 * 1024,
};
fn item<T>(slice: &[T], index: usize) -> &T {
    slice.get(index).unwrap()
}

#[test]
fn cold_source_audit_reproduces_full_identity_and_statistics_without_resampling() {
    let full = vec![vec![0; 4], vec![2, 5, 8, 11], vec![-11, -8, -5, -2]];
    let receipt = evaluate(&full, 37, 11, 2, BOUNDS).unwrap();
    let audit = audit_sources(&full, 37, 11, 2, BOUNDS).unwrap();
    assert_eq!(audit.family_digest(), receipt.family_digest());
    assert_eq!(audit.shared_digest(), receipt.shared_digest());
    for (source, row) in audit.statistics().iter().zip(receipt.rows()) {
        assert_eq!(
            *source,
            row.shared()
                .map(|value| value.observed_statistic().to_bits())
        );
    }
    let zeros = audit_sources(&[vec![0; 4]], 1, 0, 1, BOUNDS).unwrap();
    assert_eq!(zeros.statistics(), [None]);
    assert_eq!(zeros.shared_digest(), None);
    for (returns, draws, block) in [
        (vec![], 1, 1),
        (vec![vec![0]], 1, 1),
        (vec![vec![1, 2], vec![1, 2, 3]], 1, 1),
        (full.clone(), 0, 1),
        (full.clone(), 1, 0),
    ] {
        assert_eq!(
            audit_sources(&returns, draws, 0, block, BOUNDS),
            Err(Refusal::Shape)
        );
    }
    assert_eq!(
        audit_sources(&full, usize::MAX, 0, 1, BOUNDS),
        Err(Refusal::Arithmetic)
    );
    assert_eq!(
        audit_sources(&[vec![1; 4]], 1, 0, 1, BOUNDS),
        Err(Refusal::NonzeroConstant { strategy: 0 })
    );
    assert_eq!(
        audit_sources(&[vec![i64::MAX, i64::MAX - 1]], 1, 0, 1, BOUNDS),
        Err(Refusal::Numerical)
    );
    assert_eq!(
        audit_sources(
            &full,
            37,
            11,
            2,
            Bounds {
                max_bytes: 1,
                ..BOUNDS
            }
        ),
        Err(Refusal::Bound)
    );
}

#[test]
fn zero_and_nonpositive_rows_remain_mapped_with_conservative_one() {
    let active = vec![vec![2, 5, 8, 11], vec![-11, -8, -5, -2], vec![-3, -1, 1, 3]];
    let full = vec![
        vec![0; 4],
        item(&active, 0).clone(),
        vec![0; 4],
        item(&active, 1).clone(),
        item(&active, 2).clone(),
    ];
    let shared = romano_wolf_adjusted_p_values_v1(&active, 37, 11, 2).unwrap();
    let r = evaluate(&full, 37, 11, 2, BOUNDS).unwrap();
    assert_eq!(
        (
            r.strategies(),
            r.periods(),
            r.draws(),
            r.seed(),
            r.block(),
            r.bounds()
        ),
        (5, 4, 37, 11, 2, BOUNDS)
    );
    assert_eq!(r.shared_digest(), Some(shared.family_digest()));
    assert!(r.candidate(5).is_none());
    for (i, row) in r.rows().iter().enumerate() {
        assert_eq!(row.strategy(), i);
        assert_eq!(row.p_value().denominator(), 38);
    }
    for i in [0, 2] {
        let row = r.candidate(i).unwrap();
        assert_eq!(row.classification(), Classification::ConservativeZero);
        assert_eq!(row.p_value().numerator(), 38);
        assert_eq!(row.shared(), None);
    }
    for (i, at) in [(1, 0), (3, 1), (4, 2)] {
        assert_eq!(
            r.candidate(i).unwrap().shared(),
            shared.candidate(at).copied()
        );
    }
    assert_eq!(
        r.candidate(1).unwrap().classification(),
        Classification::MeasuredPositive
    );
    assert_eq!(
        r.candidate(1).unwrap().p_value().numerator(),
        shared.candidate(0).unwrap().adjusted_p_value().numerator() as u64
    );
    for i in [3, 4] {
        assert_eq!(
            r.candidate(i).unwrap().classification(),
            Classification::ConservativeNonpositive
        );
        assert_eq!(r.candidate(i).unwrap().p_value().numerator(), 38);
    }
    assert!(
        romano_wolf_adjusted_p_values_v1(&full, 37, 11, 2).is_none(),
        "V1 remains unchanged"
    );
}
#[test]
fn all_zero_is_complete_non_rejection_but_nonzero_constants_still_refuse() {
    let r = evaluate(&[vec![0; 4], vec![0; 4]], 19, 0, 7, BOUNDS).unwrap();
    assert_eq!(r.shared_digest(), None);
    assert_eq!(r.strategies(), 2);
    for row in r.rows() {
        assert_eq!(
            row.p_value(),
            Probability {
                numerator: 20,
                denominator: 20
            }
        );
        assert_eq!(row.classification(), Classification::ConservativeZero);
    }
    for value in [1, -1, i64::MAX, i64::MIN] {
        assert_eq!(
            evaluate(&[vec![0; 4], vec![value; 4]], 19, 0, 7, BOUNDS),
            Err(Refusal::NonzeroConstant { strategy: 1 })
        );
    }
    assert_eq!(
        evaluate(&[vec![i64::MAX, i64::MAX - 1]], 19, 0, 7, BOUNDS),
        Err(Refusal::Numerical)
    );
}
#[test]
fn full_identity_binds_zeros_order_every_value_procedure_and_physical_admission() {
    let input = vec![vec![0; 4], vec![1, 3, -2, 4], vec![-5, 0, 2, 0]];
    let r = evaluate(&input, 23, 0, 2, BOUNDS).unwrap();
    assert_eq!(r, evaluate(&input, 23, 0, 2, BOUNDS).unwrap());
    assert_ne!(r.family_digest(), r.shared_digest().unwrap());
    let mut reordered = input.clone();
    reordered.swap(0, 1);
    let mut changed = input.clone();
    *changed.get_mut(1).and_then(|row| row.get_mut(1)).unwrap() += 1;
    let mut extra = input.clone();
    extra.push(vec![0; 4]);
    for other in [&reordered, &changed, &extra] {
        assert_ne!(
            r.family_digest(),
            evaluate(other, 23, 0, 2, BOUNDS).unwrap().family_digest()
        );
    }
    for (draws, seed, block) in [(24, 0, 2), (23, 1, 2), (23, 0, 3)] {
        assert_ne!(
            r.family_digest(),
            evaluate(&input, draws, seed, block, BOUNDS)
                .unwrap()
                .family_digest()
        );
    }
    assert_ne!(
        r.family_digest(),
        evaluate(
            &input,
            23,
            0,
            2,
            Bounds {
                max_bytes: BOUNDS.max_bytes - 1,
                ..BOUNDS
            }
        )
        .unwrap()
        .family_digest()
    );
}
#[test]
fn shape_denominator_and_exact_work_storage_caps_refuse_before_allocation() {
    let input = vec![vec![0; 3], vec![1, 2, 3]];
    let (work, bytes) = requirements(2, 3, 17).unwrap();
    let exact = Bounds {
        max_work: u64::try_from(work).unwrap(),
        max_bytes: u64::try_from(bytes).unwrap(),
    };
    assert!(evaluate(&input, 17, 0, 2, exact).is_ok());
    assert_eq!(
        evaluate(
            &input,
            17,
            0,
            2,
            Bounds {
                max_work: exact.max_work - 1,
                ..exact
            }
        ),
        Err(Refusal::Bound)
    );
    assert_eq!(
        evaluate(
            &input,
            17,
            0,
            2,
            Bounds {
                max_bytes: exact.max_bytes - 1,
                ..exact
            }
        ),
        Err(Refusal::Bound)
    );
    for rows in [
        vec![],
        vec![vec![]],
        vec![vec![0]],
        vec![vec![0; 2], vec![0; 3]],
    ] {
        assert_eq!(evaluate(&rows, 17, 0, 2, BOUNDS), Err(Refusal::Shape));
    }
    assert_eq!(evaluate(&input, 0, 0, 2, BOUNDS), Err(Refusal::Shape));
    assert_eq!(evaluate(&input, 17, 0, 0, BOUNDS), Err(Refusal::Shape));
    assert_eq!(
        evaluate(&input, usize::MAX, 0, 2, BOUNDS),
        Err(Refusal::Arithmetic)
    );
    assert_eq!(
        requirements(usize::MAX, usize::MAX, usize::MAX),
        Err(Refusal::Arithmetic)
    );
}
#[test]
fn negative_variable_rows_still_compete_in_positive_resample_maxima() {
    let positive = vec![-2, 0, 3];
    let negative = vec![-8, 1, 3];
    let mut witnessed = false;
    for seed in 0..64 {
        let full = evaluate(
            &[positive.clone(), negative.clone(), vec![0; 3]],
            31,
            seed,
            1,
            BOUNDS,
        )
        .unwrap();
        let reduced = evaluate(std::slice::from_ref(&positive), 31, seed, 1, BOUNDS).unwrap();
        assert!(
            full.candidate(0).unwrap().p_value().numerator()
                >= reduced.candidate(0).unwrap().p_value().numerator()
        );
        witnessed |= full.candidate(0).unwrap().p_value().numerator()
            > reduced.candidate(0).unwrap().p_value().numerator();
    }
    assert!(
        witnessed,
        "dropping negative observed candidates would change positive adjusted probabilities"
    );
}

fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut n = *state;
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    n ^ (n >> 31)
}
fn draws(count: usize, seed: u64, block: u64) -> Vec<Vec<usize>> {
    let mut state = seed;
    let mut output = Vec::new();
    let chance = if block <= 1 {
        0
    } else {
        1_000_000 - 1_000_000 / block
    };
    for _ in 0..count {
        let mut at = usize::try_from(next(&mut state) % 3).unwrap();
        let mut row = Vec::new();
        for _ in 0..3 {
            row.push(at);
            at = if next(&mut state) % 1_000_000 < chance {
                (at + 1) % 3
            } else {
                usize::try_from(next(&mut state) % 3).unwrap()
            };
        }
        output.push(row);
    }
    output
}
fn full_zero_reference(rows: &[Vec<i64>], draw_count: usize, seed: u64, block: u64) -> Vec<u64> {
    let means: Vec<f64> = rows
        .iter()
        .map(|r| r.iter().map(|&n| n as f64).sum::<f64>() / 3.0)
        .collect();
    let errors: Vec<f64> = rows
        .iter()
        .zip(&means)
        .map(|(r, mean)| (r.iter().map(|&n| (n as f64 - mean).powi(2)).sum::<f64>() / 6.0).sqrt())
        .collect();
    let observed: Vec<f64> = means
        .iter()
        .zip(&errors)
        .map(|(mean, se)| if *se == 0.0 { 0.0 } else { mean / se })
        .collect();
    let mut order: Vec<_> = (0..rows.len()).collect();
    order.sort_by(|&a, &b| {
        item(&observed, b)
            .total_cmp(item(&observed, a))
            .then(a.cmp(&b))
    });
    let draws = draws(draw_count, seed, block);
    let mut answer = vec![0; rows.len()];
    let mut cumulative = 0;
    for (rank, &own) in order.iter().enumerate() {
        let count = draws
            .iter()
            .filter(|indices| {
                let max = order
                    .get(rank..)
                    .unwrap()
                    .iter()
                    .map(|&other| {
                        if *item(&errors, other) == 0.0 {
                            0.0
                        } else {
                            (indices
                                .iter()
                                .map(|&i| *item(item(rows, other), i) as f64)
                                .sum::<f64>()
                                / 3.0
                                - item(&means, other))
                                / item(&errors, other)
                        }
                    })
                    .fold(f64::NEG_INFINITY, f64::max);
                max > *item(&observed, own)
            })
            .count();
        let initial = if *item(&observed, own) <= 0.0 {
            draw_count + 1
        } else {
            count + 1
        };
        cumulative = cumulative.max(initial);
        *answer.get_mut(own).unwrap() = cumulative as u64;
    }
    answer
}
#[test]
fn complete_small_integer_families_match_explicit_zero_floored_full_suffix_reference() {
    let mut vectors = Vec::new();
    for a in [-2, 0, 3] {
        for b in [-2, 0, 3] {
            for c in [-2, 0, 3] {
                if a != b || b != c {
                    vectors.push(vec![a, b, c]);
                }
            }
        }
    }
    let mut checked = 0;
    for left in &vectors {
        for right in &vectors {
            for (seed, block) in [(0, 1), (11, 3)] {
                let rows = vec![vec![0; 3], left.clone(), right.clone(), vec![0; 3]];
                let receipt = evaluate(&rows, 19, seed, block, BOUNDS).unwrap();
                let expected = full_zero_reference(&rows, 19, seed, block as u64);
                for (row, numerator) in receipt.rows().iter().zip(expected) {
                    assert_eq!(
                        row.p_value().numerator(),
                        numerator,
                        "{rows:?} seed={seed} block={block}"
                    );
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 1152);
}

/// The three separate calls, made as the index-stop qualification made them.
fn separately(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
    bounds: Bounds,
) -> Result<FamilyEvaluation, FamilyRefusal> {
    let romano_wolf =
        evaluate(returns, draws, seed, block, bounds).map_err(FamilyRefusal::RomanoWolf)?;
    let white =
        white_reality_check_receipt_v1(returns, draws, seed, block).ok_or(FamilyRefusal::White)?;
    let spa = spa_receipt_v1(returns, draws, seed, block).ok_or(FamilyRefusal::Spa)?;
    Ok(FamilyEvaluation {
        romano_wolf,
        white,
        spa,
    })
}

#[test]
fn one_walk_reproduces_all_three_separate_calls_byte_for_byte() {
    let mut state = 0x5eed;
    let mut generated: Vec<Vec<i64>> = (0..30_i64)
        .map(|row| match row % 6 {
            0 => vec![0; 60],
            _ => (0..60)
                .map(|_| i64::try_from(next(&mut state) % 21).unwrap() - 10 + row % 5 - 2)
                .collect(),
        })
        .collect();
    // An exact duplicate: a tie in every statistic.
    generated.push(item(&generated, 1).clone());
    for returns in [
        vec![
            vec![0; 4],
            vec![2, 5, 8, 11],
            vec![0; 4],
            vec![-11, -8, -5, -2],
            vec![-3, -1, 1, 3],
        ],
        vec![vec![0; 4], vec![0; 4]],
        vec![vec![1, 3, -2, 4], vec![-5, 0, 2, 0], vec![0; 4]],
        vec![vec![-2, 0], vec![0, 0], vec![3, -1]],
        generated,
    ] {
        for (draws, seed, block) in [(1, 0, 1), (19, 11, 2), (101, 3, 7)] {
            let shared = evaluate_with_family_tests(&returns, draws, seed, block, BOUNDS);
            assert_eq!(shared, separately(&returns, draws, seed, block, BOUNDS));
            let shared = shared.unwrap();
            assert_eq!(
                shared.romano_wolf(),
                &evaluate(&returns, draws, seed, block, BOUNDS).unwrap()
            );
            assert_eq!(
                Some(shared.white()),
                white_reality_check_receipt_v1(&returns, draws, seed, block)
            );
            assert_eq!(
                Some(shared.spa()),
                spa_receipt_v1(&returns, draws, seed, block)
            );
        }
    }
}

#[test]
fn every_small_integer_family_is_reproduced_by_the_one_walk() {
    let mut vectors = Vec::new();
    for a in [-2, 0, 3] {
        for b in [-2, 0, 3] {
            for c in [-2, 0, 3] {
                if a != b || b != c {
                    vectors.push(vec![a, b, c]);
                }
            }
        }
    }
    let mut checked = 0;
    for left in &vectors {
        for right in &vectors {
            for (seed, block) in [(0, 1), (11, 3)] {
                let rows = vec![vec![0; 3], left.clone(), right.clone(), vec![0; 3]];
                assert_eq!(
                    evaluate_with_family_tests(&rows, 19, seed, block, BOUNDS),
                    separately(&rows, 19, seed, block, BOUNDS),
                    "{rows:?} seed={seed} block={block}"
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 1152);
}

#[test]
fn every_refusal_is_the_one_the_separate_calls_reach_first() {
    let input = vec![vec![0; 3], vec![1, 2, 3]];
    let (work, bytes) = requirements(2, 3, 17).unwrap();
    let exact = Bounds {
        max_work: u64::try_from(work).unwrap(),
        max_bytes: u64::try_from(bytes).unwrap(),
    };
    let cases: Vec<(Vec<Vec<i64>>, usize, usize, Bounds)> = vec![
        (vec![], 17, 2, BOUNDS),
        (vec![vec![]], 17, 2, BOUNDS),
        (vec![vec![0]], 17, 2, BOUNDS),
        (vec![vec![0; 2], vec![0; 3]], 17, 2, BOUNDS),
        (input.clone(), 0, 2, BOUNDS),
        (input.clone(), 17, 0, BOUNDS),
        (input.clone(), usize::MAX, 2, BOUNDS),
        (
            input.clone(),
            17,
            2,
            Bounds {
                max_work: exact.max_work - 1,
                ..exact
            },
        ),
        (
            input.clone(),
            17,
            2,
            Bounds {
                max_bytes: exact.max_bytes - 1,
                ..exact
            },
        ),
        (vec![vec![0; 4], vec![1; 4]], 17, 2, BOUNDS),
        (vec![vec![i64::MAX, i64::MAX - 1]], 17, 2, BOUNDS),
    ];
    for (returns, draws, block, bounds) in cases {
        let refused = evaluate_with_family_tests(&returns, draws, 0, block, bounds);
        assert!(refused.is_err(), "{returns:?} was not refused");
        assert_eq!(refused, separately(&returns, draws, 0, block, bounds));
    }
    assert!(evaluate_with_family_tests(&input, 17, 0, 2, exact).is_ok());
}

#[test]
fn every_shared_walk_refusal_names_the_separate_call_it_stands_for() {
    assert_eq!(
        family_refusal(FamilyTestsRefusalV1::White),
        FamilyRefusal::White
    );
    assert_eq!(
        family_refusal(FamilyTestsRefusalV1::Spa),
        FamilyRefusal::Spa
    );
    for why in [
        FamilyTestsRefusalV1::Rows,
        FamilyTestsRefusalV1::RomanoWolf,
        FamilyTestsRefusalV1::Pass,
    ] {
        assert_eq!(
            family_refusal(why),
            FamilyRefusal::RomanoWolf(Refusal::Numerical)
        );
    }
}
