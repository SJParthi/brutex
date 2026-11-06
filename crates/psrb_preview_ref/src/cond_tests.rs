#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::indicators::supertrend::Reading;
use crate::market::Bar;
use crate::rng::SplitMix64;

fn prev(h: i64, l: i64, c: i64) -> AcceptedBar {
    AcceptedBar::from_hlc(h, l, c).unwrap()
}

/// PWC 4.3 fixtures CPR-1, CPR-2, CPR-2b (C1) and PIV-1, PIV-2 (C2).
#[test]
fn t_f20_f21_the_cpr_and_pivot_fixtures() {
    assert!(c1_narrow_cpr(&prev(100_100, 99_800, 100_100), 10));
    assert!(!c1_narrow_cpr(&prev(100_100, 99_800, 100_100), 9));
    assert!(c1_narrow_cpr(&prev(101_000, 99_000, 100_150), 10));
    assert!(!c1_narrow_cpr(&prev(101_000, 99_000, 100_151), 10));
    assert!(c1_narrow_cpr(&prev(101_000, 99_000, 100_151), 20));
    let piv1 = prev(100_100, 99_800, 100_100);
    assert!(c2_beyond_r1_s1(&piv1, 100_201, Side::Long) && !c2_beyond_r1_s1(&piv1, 100_200, Side::Long));
    assert!(c2_beyond_r1_s1(&piv1, 99_899, Side::Short) && !c2_beyond_r1_s1(&piv1, 99_900, Side::Short));
    let piv2 = prev(10_007, 10_000, 10_001);
    assert!(!c2_beyond_r1_s1(&piv2, 10_005, Side::Long) && c2_beyond_r1_s1(&piv2, 10_006, Side::Long));
    assert!(c2_beyond_r1_s1(&piv2, 9_998, Side::Short) && !c2_beyond_r1_s1(&piv2, 9_999, Side::Short));
    assert_eq!(Side::Long.label(), "LONG");
    assert_eq!(Side::Short.label(), "SHORT");
}

/// X-F20: C1 at its edges.
#[test]
fn x_f20_c1_extremes() {
    assert!(c1_narrow_cpr(&prev(10_100, 9_900, 10_000), 20));
    assert!(!c1_narrow_cpr(&prev(11_000, 9_000, 10_800), 20));
    // a tie is narrow, one unit over is not: 10_000 x |x| = 20 x T
    assert!(c1_narrow_cpr(&prev(100_100, 99_800, 100_100), 10));
    assert!(c1_narrow_cpr(&prev(5, 5, 5), 20));
    // a negative inner term uses its absolute value: |x| = 300 over T = 299,700 is wider
    // than 10 bp
    assert!(!c1_narrow_cpr(&prev(100_100, 99_800, 99_800), 10));
    assert!(c1_narrow_cpr(&prev(100_100, 99_800, 99_800), 11));
    assert!(c1_narrow_cpr(&prev(2, 1, 1), 10_000));
    // combo: near i64::MAX / 3, exact in i128
    let big = i64::MAX / 3;
    assert!(c1_narrow_cpr(&prev(big, big - 1, big - 1), 1));
    // combo: the tie reached through a negative inner term: x = -3, T = 1,500
    assert!(c1_narrow_cpr(&prev(502, 499, 499), 20));
    assert!(!c1_narrow_cpr(&prev(502, 499, 499), 19));
}

/// X-F21: C2 at its edges.
#[test]
fn x_f21_c2_extremes() {
    let flat = prev(100, 100, 100);
    assert!(!c2_beyond_r1_s1(&flat, 100, Side::Long) && !c2_beyond_r1_s1(&flat, 100, Side::Short));
    assert!(c2_beyond_r1_s1(&flat, 101, Side::Long) && c2_beyond_r1_s1(&flat, 99, Side::Short));
    assert!(c2_beyond_r1_s1(&prev(110, 90, 100), 1, Side::Short));
    let big = i64::MAX;
    assert!(!c2_beyond_r1_s1(&prev(big, big - 10, big - 5), big, Side::Long) || c2_beyond_r1_s1(&prev(big, big - 10, big - 5), big, Side::Long));
    assert!(c2_beyond_r1_s1(&prev(big, big - 10, big - 5), 1, Side::Short));
    // the proof test: never both sides (2T - 3L >= 2T - 3H)
    let mut r = SplitMix64::new(9);
    for _ in 0..10_000 {
        let l = i64::try_from(r.below(1_000)).unwrap() + 1;
        let h = l + i64::try_from(r.below(100)).unwrap();
        let c = l + i64::try_from(r.below(u64::try_from(h - l + 1).unwrap())).unwrap();
        let p = prev(h, l, c);
        let close = i64::try_from(r.below(1_200)).unwrap() + 1;
        assert!(!(c2_beyond_r1_s1(&p, close, Side::Long) && c2_beyond_r1_s1(&p, close, Side::Short)));
    }
}

fn st(trend: Trend, settled: bool) -> Reading {
    Reading { trend, line: 0, upper: 0, lower: 0, settled }
}

/// T-F22-3: settled x trend x side.
#[test]
fn t_f22_supertrend_side_permutations() {
    for settled in [true, false] {
        for trend in [Trend::Up, Trend::Down] {
            for side in Side::BOTH {
                let want = settled && (trend == Trend::Up) == (side == Side::Long);
                assert_eq!(c3_supertrend_side(Some(&st(trend, settled)), side), want);
            }
        }
    }
    assert!(!c3_supertrend_side(None, Side::Long));
}

/// T-F24..F27: the reading conditions on both sides, warm and cold, flat and not.
#[test]
fn t_f23_f27_reading_conditions() {
    use crate::indicators::{bollinger, cci, ema, rsi, willr};
    let w = bollinger::Window { n: 2, sum: 3, sum_sq: 5, last: 2 };
    assert!(c4_bollinger_beyond(Some(&w), 900, Side::Long) && !c4_bollinger_beyond(Some(&w), 1_000, Side::Long));
    assert!(!c4_bollinger_beyond(Some(&w), 0, Side::Short) && !c4_bollinger_beyond(None, 0, Side::Long));
    let up = rsi::Reading { period: 14, gain: 1_000_000, loss: 0 };
    let down = rsi::Reading { period: 14, gain: 0, loss: 1_000_000 };
    let thin = rsi::Reading { period: 14, gain: 13_999, loss: 0 };
    assert!(c5_rsi_beyond(Some(&up), 700_000, 300_000, Side::Long) && !c5_rsi_beyond(Some(&up), 700_000, 300_000, Side::Short));
    assert!(c5_rsi_beyond(Some(&down), 700_000, 300_000, Side::Short) && !c5_rsi_beyond(Some(&thin), 700_000, 300_000, Side::Long));
    assert!(!c5_rsi_beyond(None, 700_000, 300_000, Side::Long));
    let top = willr::Reading { highest: 110, lowest: 100, close: 110 };
    let flat = willr::Reading { highest: 100, lowest: 100, close: 100 };
    assert!(c6_willr_level(Some(&top), -850_000, -150_000, Side::Long) && !c6_willr_level(Some(&top), -850_000, -150_000, Side::Short));
    assert!(!c6_willr_level(Some(&flat), -850_000, -150_000, Side::Long) && !c6_willr_level(None, -1, -1, Side::Short));
    let low = willr::Reading { highest: 110, lowest: 100, close: 100 };
    assert!(c6_willr_level(Some(&low), -150_000, -850_000, Side::Short));
    let hot = cci::Reading { n: 20, deviation: 100, total_deviation: 200 };
    let cold = cci::Reading { n: 20, deviation: -100, total_deviation: 200 };
    let none = cci::Reading { n: 20, deviation: 0, total_deviation: 0 };
    assert!(c7_cci_beyond(Some(&hot), 200_000, -200_000, Side::Long) && !c7_cci_beyond(Some(&hot), 200_000, -200_000, Side::Short));
    assert!(c7_cci_beyond(Some(&cold), 200_000, -200_000, Side::Short) && !c7_cci_beyond(Some(&none), 0, 0, Side::Long));
    assert!(!c7_cci_beyond(None, 0, 0, Side::Short));
    let mut e = ema::Ema::new(2);
    let b = Bar::flat(100, 1);
    e.fold(&b);
    assert!(!c8_ema_side(&e, 101, Side::Long));
    e.fold(&b);
    assert!(c8_ema_side(&e, 101, Side::Long) && !c8_ema_side(&e, 100, Side::Long) && !c8_ema_side(&e, 100, Side::Short));
    assert!(c8_ema_side(&e, 99, Side::Short) && !c8_ema_side(&e, 99, Side::Long));
}
