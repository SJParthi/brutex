#![cfg(test)]
//! Independent integer arithmetic and finite telescoping regression fixtures.
#![expect(clippy::unwrap_used, reason = "finite exact arithmetic assertions")]
use super::*;

fn pair(value: Fraction) -> (u128, u128) {
    (value.numerator(), value.denominator())
}

#[test]
fn fixed_slots_retries_and_digest_bind_the_entire_numeric_contract() {
    let mut unique = std::collections::BTreeSet::new();
    for batch in 0..32 {
        for rung in 0..8 {
            let a = allocate(batch, rung, 50_000).unwrap();
            assert_eq!(a, allocate(batch, rung, 50_000).unwrap());
            assert_eq!(
                (a.batch(), a.rung(), a.rungs(), a.alpha_ppm()),
                (batch, rung, 8, 50_000)
            );
            assert!(unique.insert(a.digest()));
            let mut bytes = b"brutex-countable-search-allocation-v1\0".to_vec();
            for value in [batch, rung, 8, 50_000] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            assert_eq!(a.digest(), brutex_core::blake3::hash(&bytes));
            assert_ne!(a.digest(), allocate(batch, rung, 50_001).unwrap().digest());
            let expected = 160 * (u128::from(batch) + 1) * (u128::from(batch) + 2);
            assert_eq!(pair(a.threshold()), (1, expected));
        }
    }
    // Retrying a prior slot after arbitrary future allocations never recycles
    // a skipped share or depends on the order these pure functions are called.
    let first = allocate(0, 7, 50_000).unwrap();
    assert_eq!(allocate(500_000, 2, 50_000).unwrap().batch(), 500_000);
    assert_eq!(first, allocate(0, 7, 50_000).unwrap());
}

#[test]
fn independently_accumulated_finite_prefixes_telescope_without_exceeding_alpha() {
    // LCM(1..=16) permits a completely independent integer sum of the first
    // fifteen rational batch shares, without the production reduction helper.
    const UNIT: u128 = 720_720;
    for alpha in [0_u64, 1, 73_127, 1_000_000] {
        let denominator = UNIT * 8 * 1_000_000;
        let mut sum = 0_u128;
        for batch in 0..15_u64 {
            for rung in 0..8 {
                let p = allocate(batch, rung, alpha).unwrap().threshold();
                assert_eq!(denominator % p.denominator(), 0);
                sum += p.numerator() * (denominator / p.denominator());
            }
            let n = u128::from(batch) + 1;
            assert_eq!(sum * (n + 1), u128::from(alpha) * UNIT * 8 * n);
            assert!(sum <= u128::from(alpha) * UNIT * 8);
        }
    }
}

#[test]
fn exact_threshold_draw_floor_and_scaled_probability_share_one_boundary() {
    let a = allocate(0, 7, 50_000).unwrap();
    assert_eq!(pair(a.threshold()), (1, 320));
    assert_eq!(a.compare(1, 320), Ok(true));
    assert_eq!(a.compare(2, 640), Ok(true));
    assert_eq!(a.compare(1, 319), Ok(false));
    assert_eq!(a.compare(1, 321), Ok(true));
    assert_eq!(a.minimum_draws(), Some(319));
    assert_eq!(a.require_draws(318), Err(Refusal::Resolution));
    assert_eq!(a.require_draws(319), Ok(()));
    assert_eq!(a.require_draws(320), Ok(()));
    assert_eq!(pair(a.scaled_probability(1, 320).unwrap()), (1, 20));
    assert_eq!(pair(a.scaled_probability(1, 640).unwrap()), (1, 40));
    assert_eq!(pair(a.scaled_probability(1, 16).unwrap()), (1, 1));
    assert_eq!(pair(a.scaled_probability(1, 1).unwrap()), (1, 1));
    assert_eq!(pair(a.scaled_probability(0, 91).unwrap()), (0, 1));
    let zero = allocate(11, 0, 0).unwrap();
    assert_eq!(pair(zero.threshold()), (0, 1));
    assert_eq!(zero.minimum_draws(), None);
    assert_eq!(zero.require_draws(u64::MAX), Err(Refusal::Resolution));
    assert_eq!(zero.compare(0, 1), Ok(true));
    assert_eq!(zero.compare(1, u64::MAX), Ok(false));
    let full = allocate(0, 0, 1_000_000).unwrap();
    assert_eq!(pair(full.threshold()), (1, 16));
    assert_eq!(full.require_draws(0), Err(Refusal::Resolution));
    assert_eq!(full.require_draws(14), Err(Refusal::Resolution));
    assert_eq!(full.require_draws(15), Ok(()));
    assert_eq!(full.require_draws(u64::MAX), Ok(()));
}

#[test]
fn probability_lattice_matches_direct_unreduced_integer_comparison() {
    for batch in 0..12_u64 {
        for alpha in [0_u64, 1, 73_127, 1_000_000] {
            let a = allocate(batch, batch % 8, alpha).unwrap();
            let weight = 8 * (u128::from(batch) + 1) * (u128::from(batch) + 2);
            for denominator in 1..=63_u64 {
                for numerator in 0..=denominator {
                    let want = u128::from(numerator) * 1_000_000 * weight
                        <= u128::from(alpha) * u128::from(denominator);
                    assert_eq!(a.compare(numerator, denominator), Ok(want));
                    let scaled = a.scaled_probability(numerator, denominator).unwrap();
                    assert_eq!(
                        scaled.numerator() * u128::from(denominator),
                        (u128::from(numerator) * weight).min(u128::from(denominator))
                            * scaled.denominator()
                    );
                }
            }
        }
    }
}

#[test]
fn nonintegral_draw_threshold_is_rounded_up_before_subtracting_one() {
    let a = allocate(4, 3, 73_127).unwrap();
    let denominator = 240_000_000_u128;
    let floor = denominator / 73_127;
    assert_ne!(denominator % 73_127, 0);
    assert_eq!(a.minimum_draws(), Some(floor));
    let enough = u64::try_from(floor).unwrap();
    assert_eq!(a.require_draws(enough - 1), Err(Refusal::Resolution));
    assert_eq!(a.require_draws(enough), Ok(()));
    assert_eq!(a.compare(1, enough), Ok(false));
    assert_eq!(a.compare(1, enough + 1), Ok(true));
}

#[test]
fn malformed_scope_and_probabilities_have_explicit_refusals() {
    for rung in [8, 9, u64::MAX] {
        assert_eq!(allocate(0, rung, 50_000), Err(Refusal::Scope));
    }
    for alpha in [1_000_001, u64::MAX] {
        assert_eq!(allocate(0, 0, alpha), Err(Refusal::Probability));
    }
    // The documented precedence is scope before alpha, then arithmetic.
    assert_eq!(allocate(u64::MAX, 8, u64::MAX), Err(Refusal::Scope));
    assert_eq!(allocate(u64::MAX, 0, u64::MAX), Err(Refusal::Probability));
    let a = allocate(3, 5, 50_000).unwrap();
    for (n, d) in [(0, 0), (1, 0), (2, 1), (u64::MAX, u64::MAX - 1)] {
        assert_eq!(a.compare(n, d), Err(Refusal::Probability));
        assert_eq!(a.scaled_probability(n, d), Err(Refusal::Probability));
    }
    assert_eq!(a.compare(u64::MAX, u64::MAX), Ok(false));
    assert_eq!(
        pair(a.scaled_probability(u64::MAX, u64::MAX).unwrap()),
        (1, 1)
    );
}

#[test]
fn machine_and_draw_ceilings_refuse_without_wrapping_or_reusing_a_slot() {
    // Distinct failures of the adjacent-ordinal product, eight-rung weight,
    // and ppm denominator. All are public inputs; no private corruption.
    for batch in [u64::MAX, u64::MAX / 2, 10_000_000_000_000_000] {
        assert_eq!(allocate(batch, 0, 1), Err(Refusal::Arithmetic));
    }
    let large = allocate(1_000_000_000_000_000, 7, 1).unwrap();
    assert_eq!(large.batch(), 1_000_000_000_000_000);
    assert_eq!(large.require_draws(u64::MAX), Err(Refusal::Resolution));
    assert_eq!(
        large.compare(u64::MAX - 1, u64::MAX),
        Err(Refusal::Arithmetic)
    );
    assert_eq!(
        large.scaled_probability(u64::MAX - 1, u64::MAX),
        Err(Refusal::Arithmetic)
    );
    assert_eq!(large.compare(0, u64::MAX), Ok(true));
    assert_eq!(pair(large.scaled_probability(0, u64::MAX).unwrap()), (0, 1));
    assert_eq!(allocate(0, 0, 50_000).unwrap().minimum_draws(), Some(319));
}
