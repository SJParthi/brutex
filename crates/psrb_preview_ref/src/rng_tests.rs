#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;

#[test]
fn the_generator_is_fixed_by_its_seed() {
    let mut a = SplitMix64::new(7);
    let mut b = SplitMix64::new(7);
    for _ in 0..100 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
    // SplitMix64's published first output for seed 0.
    assert_eq!(SplitMix64::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
}

#[test]
fn draws_stay_in_their_ranges() {
    let mut r = SplitMix64::new(1);
    assert_eq!(r.below(0), 0);
    assert_eq!(r.below(1), 0);
    for _ in 0..1_000 {
        assert!(r.below(10) < 10);
        let v = r.between(5, -5);
        assert!((-5..=5).contains(&v));
        assert_eq!(r.between(3, 3), 3);
    }
    assert!((i64::MIN..=i64::MAX).contains(&r.between(i64::MIN, i64::MAX)));
    assert!(!r.chance(0, 5));
    assert!(r.chance(5, 5));
    // A bound just above a power of two exercises the rejection branch.
    let mut hits = 0;
    for _ in 0..10_000 {
        hits += u64::from(r.below((1 << 63) + 1) > (1 << 62));
    }
    assert!(hits > 0);
}
