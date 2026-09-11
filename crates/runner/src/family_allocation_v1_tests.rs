//! Exact finite-scope allocation arithmetic regressions.
#![expect(clippy::unwrap_used, reason = "finite exact arithmetic assertions")]
use super::*;

#[test]
fn every_predeclared_cell_shares_one_budget_and_retries_do_not_renumber() {
    let mut identities = std::collections::BTreeSet::new();
    let mut total = 0;
    for batch in 0..3 {
        for rung in 0..8 {
            let a = allocate(batch, 3, rung, 8, 75_000).unwrap();
            assert_eq!(a, allocate(batch, 3, rung, 8, 75_000).unwrap());
            assert_eq!(
                (a.batch(), a.batches(), a.rung(), a.rungs(), a.alpha_ppm()),
                (batch, 3, rung, 8, 75_000)
            );
            assert_eq!(
                (a.threshold().numerator(), a.threshold().denominator()),
                (1, 320)
            );
            total += a.threshold().numerator();
            assert!(identities.insert(a.digest()));
        }
    }
    assert_eq!(total * 1_000_000, 75_000 * 320);
    assert_ne!(
        allocate(0, 3, 0, 8, 75_000).unwrap().digest(),
        allocate(0, 2, 0, 8, 75_000).unwrap().digest()
    );
    assert_ne!(
        allocate(0, 3, 0, 8, 75_000).unwrap().digest(),
        allocate(0, 3, 0, 8, 50_000).unwrap().digest()
    );
}
#[test]
fn exact_boundary_scaling_and_draw_resolution_never_round_through_ppm() {
    let a = allocate(2, 3, 7, 8, 75_000).unwrap();
    assert_eq!(a.compare(1, 320), Ok(true));
    assert_eq!(a.compare(1, 319), Ok(false));
    assert_eq!(a.compare(2, 640), Ok(true));
    assert_eq!(a.minimum_draws(), Some(319));
    let p = a.scaled_probability(1, 640).unwrap();
    assert_eq!((p.numerator(), p.denominator()), (3, 80));
    assert_eq!(
        a.scaled_probability(1, 1).unwrap(),
        Fraction {
            numerator: 1,
            denominator: 1
        }
    );
    assert_eq!(
        a.scaled_probability(0, 17).unwrap(),
        Fraction {
            numerator: 0,
            denominator: 1
        }
    );
    let zero = allocate(0, 1, 0, 8, 0).unwrap();
    assert_eq!(zero.minimum_draws(), None);
    assert_eq!(zero.compare(1, 1_000_000), Ok(false));
    assert_eq!(zero.compare(0, 1), Ok(true));
}
#[test]
fn complete_small_probability_lattice_matches_unreduced_integer_comparison() {
    for batches in 1..=4 {
        for rungs in 1..=8 {
            for alpha in [0, 1, 50_000, 1_000_000] {
                let a = allocate(0, batches, rungs - 1, rungs, alpha).unwrap();
                for denominator in 1..=31 {
                    for numerator in 0..=denominator {
                        assert_eq!(
                            a.compare(numerator, denominator).unwrap(),
                            u128::from(numerator)
                                * 1_000_000
                                * u128::from(batches)
                                * u128::from(rungs)
                                <= u128::from(alpha) * u128::from(denominator)
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn malformed_scope_probabilities_and_unrepresentable_products_refuse() {
    for (batch, batches, rung, rungs) in [(0, 0, 0, 8), (1, 1, 0, 8), (0, 1, 0, 0), (0, 1, 8, 8)] {
        assert_eq!(
            allocate(batch, batches, rung, rungs, 50_000),
            Err(Refusal::Scope)
        );
    }
    assert_eq!(allocate(0, 1, 0, 8, 1_000_001), Err(Refusal::Probability));
    assert_eq!(
        allocate(0, u64::MAX, 0, u64::MAX, 1),
        Err(Refusal::Arithmetic)
    );
    let a = allocate(0, 1, 0, 8, 50_000).unwrap();
    for (n, d) in [(0, 0), (2, 1)] {
        assert_eq!(a.compare(n, d), Err(Refusal::Probability));
        assert_eq!(a.scaled_probability(n, d), Err(Refusal::Probability));
    }
    let large = allocate(0, u64::MAX, 0, 1_000_000, 1).unwrap();
    assert_eq!(
        large.compare(u64::MAX - 1, u64::MAX),
        Err(Refusal::Arithmetic)
    );
    assert_eq!(
        large.scaled_probability(u64::MAX - 1, u64::MAX),
        Err(Refusal::Arithmetic)
    );
}
