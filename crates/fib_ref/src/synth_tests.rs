use super::*;

#[test]
fn splitmix_is_the_reference_sequence() {
    // the first outputs of SplitMix64 seeded with 0 (widely published)
    let mut r = SplitMix64::new(0);
    assert_eq!(r.next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(r.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(r.next_u64(), 0x06C4_5D18_8009_454F);
    let mut a = SplitMix64::new(42);
    let mut b = SplitMix64::new(42);
    for _ in 0..100 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
}

#[test]
fn range_below_and_chance_stay_in_bounds() {
    let mut r = SplitMix64::new(7);
    for _ in 0..1000 {
        let x = r.range(-3, 5);
        assert!((-3..=5).contains(&x));
        assert!(r.below(10) < 10);
    }
    assert_eq!(r.range(5, 5), 5);
    assert_eq!(r.range(9, 2), 9);
    assert_eq!(r.below(0), 0);
    assert!(!r.chance(0));
    assert!(r.chance(1000));
    let full = r.range(i64::MIN, i64::MAX);
    let _ = full;
}

#[test]
fn seconds_stay_inside_their_minute_and_hit_its_extremes() {
    let mut r = SplitMix64::new(11);
    let minutes = random_minutes(&mut r, 100_000, 5, 50, 0, 0, 4, 3);
    assert_eq!(minutes.len(), 50);
    for m in &minutes {
        assert!(m.l <= m.o.min(m.c) && m.h >= m.o.max(m.c));
        for permille in [0, 300, 1000] {
            let secs = seconds_for(&mut r, m, 5, permille, 100);
            assert!(secs.len() >= 3);
            assert_eq!(secs[0].o, m.o);
            assert_eq!(secs.last().map(|s| s.c), Some(m.c));
            assert_eq!(secs.iter().map(|s| s.h).max(), Some(m.h));
            assert_eq!(secs.iter().map(|s| s.l).min(), Some(m.l));
            assert!(secs.windows(2).all(|w| w[0].t < w[1].t));
            assert!(secs.iter().all(|s| s.l <= s.h && s.v >= 1));
        }
    }
    let all = seconds_for_all(&mut r, &minutes[..3], 5, 500, 10);
    assert!(all.len() >= 9);
}

#[test]
fn scripts_place_prices_on_the_grid() {
    let mut r = SplitMix64::new(3);
    let m = scripted(&mut r, 10_000, 5, &[(0, 4, -2, 3), (3, 3, 3, 3)], 10);
    assert_eq!(
        (m[0].m, m[0].o, m[0].h, m[0].l, m[0].c),
        (10, 10_000, 10_020, 9_990, 10_015)
    );
    assert_eq!(m[0].g, Some(10_015));
    assert_eq!(m[1].m, 11);
    // a walk that would go below zero stays at one step
    let low = random_minutes(&mut r, 5, 5, 20, 0, -10, 1, 1);
    assert!(low.iter().all(|x| x.l >= 5 && x.c >= 5));
}
