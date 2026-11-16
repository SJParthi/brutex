//! CHANGE C11 (crate v2.1; SNIPER s16, operator 14 Nov "Yes, closest count"; MAIN Q20 and the C11 confirmation in
//! MAIN-SIGNED-IDXOPT-20261114): `limits::size_units_closest`, the CLOSEST whole count, red tests first. Written before the function,
//! against a stub that returns crate v2's FLOOR count (`size_units`) with no reason: the half-way, 2 x, free-cash, no-reference and
//! extreme cases below fail on it and pass after it. The floor function `size_units` is unchanged (its v2 tests and vectors stay green).
//! Every figure is an integer (paisa, units); synthetic cases say so.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::too_many_lines, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap, clippy::similar_names, clippy::many_single_char_names, clippy::cast_lossless, reason = "tests")]

use crate::limits::{SizeWhy, exposure, size_units, size_units_closest};
use crate::tests_v2::Rng;
use crate::{FillErr, Paisa, Units};

/// A lot of 50 units at Rs 20.00 (2,000 paisa): one lot costs Rs 1,000.00 = 100,000 paisa (synthetic).
const UNIT: Units = 50;
const PX: Paisa = 2_000;

fn sz(target: Paisa, free: Option<Paisa>) -> (Units, SizeWhy) {
    size_units_closest(target, Some(PX), UNIT, free).unwrap()
}

/// SNIPER s16: "Exactly half way: the lower count (never above when both are equally close)": 1.5 lots -> 1 lot, 2.5 lots -> 2 lots.
#[test]
fn c11_exactly_half_way_takes_the_lower_count() {
    assert_eq!(sz(150_000, None), (50, SizeWhy::HalfWay));
    assert_eq!(sz(250_000, None), (100, SizeWhy::HalfWay));
    assert_eq!(sz(1_050_000, None), (500, SizeWhy::HalfWay), "10.5 lots -> 10");
    assert!(!SizeWhy::HalfWay.refused());
}

/// Just below half way (1.49 lots) the lower count; just above (1.51 lots) the upper, slightly ABOVE the target (s16: "even when that is
/// slightly above it"); one paisa either side of half way decides it.
#[test]
fn c11_just_below_and_just_above_half_way() {
    assert_eq!(sz(149_000, None), (50, SizeWhy::Closest));
    assert_eq!(sz(151_000, None), (100, SizeWhy::Closest));
    assert_eq!(sz(149_999, None), (50, SizeWhy::Closest));
    assert_eq!(sz(150_001, None), (100, SizeWhy::Closest));
    assert_eq!(sz(200_000, None), (100, SizeWhy::Closest), "exactly 2 lots");
}

/// MAIN Q20 (14 Nov): one lot costing EXACTLY twice the target is half way between 0 and 1 lot: the lower count, 0, refused by name and
/// counted (`exact_2x`).
#[test]
fn c11_one_lot_costing_exactly_twice_the_target_is_refused_exact_2x() {
    assert_eq!(sz(50_000, None), (0, SizeWhy::Exact2x));
    assert!(SizeWhy::Exact2x.refused());
    assert_eq!(SizeWhy::Exact2x.code(), "exact_2x");
    // The 2 x refusal comes before the free-cash cap (no cash is asked of a refused trade).
    assert_eq!(sz(50_000, Some(0)), (0, SizeWhy::Exact2x));
}

/// One lot costing MORE than twice the target (just over, and far over): 0, refused `over_2x`.
#[test]
fn c11_one_lot_costing_more_than_twice_the_target_is_refused_over_2x() {
    assert_eq!(sz(49_999, None), (0, SizeWhy::Over2x));
    assert_eq!(sz(1, None), (0, SizeWhy::Over2x));
    assert!(SizeWhy::Over2x.refused());
    assert_eq!(SizeWhy::Over2x.code(), "over_2x");
}

/// One lot costing between 1 x and 2 x the target (strictly): ONE lot (closer than 0), and exactly 1 x: one lot.
#[test]
fn c11_one_lot_costing_between_one_and_two_x_the_target_buys_one_lot() {
    assert_eq!(sz(50_001, None), (50, SizeWhy::Closest), "one lot at 1.99999 x the target");
    assert_eq!(sz(75_000, None), (50, SizeWhy::Closest), "one lot at 1.33 x");
    assert_eq!(sz(99_999, None), (50, SizeWhy::Closest));
    assert_eq!(sz(100_000, None), (50, SizeWhy::Closest), "exactly one lot");
}

/// Base Q7: when free cash cannot pay the closest count, the largest count that fits, named with the closest count beside
/// ("cash-short: n of m lots"); not even one lot fits: 0, refused `cash_short_zero`. Free cash exactly enough: the closest count.
#[test]
fn c11_free_cash_short_takes_the_largest_count_that_fits() {
    assert_eq!(sz(1_000_000, Some(350_000)), (150, SizeWhy::CashShort { closest: 500 }), "10 lots wanted, 3 fit");
    assert_eq!(sz(1_000_000, Some(99_999)), (0, SizeWhy::CashShortZero { closest: 500 }));
    assert_eq!(sz(1_000_000, Some(0)), (0, SizeWhy::CashShortZero { closest: 500 }));
    assert_eq!(sz(1_000_000, Some(1_000_000)), (500, SizeWhy::Closest), "exactly enough");
    assert_eq!(sz(1_000_000, None), (500, SizeWhy::Closest), "no cap");
    // The closest count slightly ABOVE the target (1.6 lots -> 2) with free cash = the target: 2 lots do not fit, 1 does.
    assert_eq!(sz(160_000, Some(160_000)), (50, SizeWhy::CashShort { closest: 100 }));
    // A half-way count short of cash: cash decides (the half-way lower count 1 does not fit 99,999).
    assert_eq!(sz(150_000, Some(99_999)), (0, SizeWhy::CashShortZero { closest: 50 }));
    assert!(!SizeWhy::CashShort { closest: 1 }.refused() && SizeWhy::CashShortZero { closest: 1 }.refused());
    assert_eq!((SizeWhy::CashShort { closest: 1 }.code(), SizeWhy::CashShortZero { closest: 1 }.code()), ("cash_short", "cash_short_zero"));
    // Free cash below 0 must never reach the sizing law (OB25): refused by name, never read as 0.
    assert_eq!(size_units_closest(1_000_000, Some(PX), UNIT, Some(-1)).err(), Some(FillErr::NegativeFreeCash));
}

/// No reference price (none traded before T, `size_ref` None), a reference of 0 or below: 0, refused `no_reference` (S11); a target of 0
/// or below: 0, refused `no_target`; a lot of 0: refused `LotMismatch` (OB8), never read as 1.
#[test]
fn c11_no_reference_no_target_and_a_lot_of_zero() {
    assert_eq!(size_units_closest(1_000_000, None, UNIT, None).unwrap(), (0, SizeWhy::NoReference));
    assert_eq!(size_units_closest(1_000_000, Some(0), UNIT, None).unwrap(), (0, SizeWhy::NoReference));
    assert_eq!(size_units_closest(1_000_000, Some(-5), UNIT, None).unwrap(), (0, SizeWhy::NoReference));
    assert_eq!(size_units_closest(0, Some(PX), UNIT, None).unwrap(), (0, SizeWhy::NoTarget));
    assert_eq!(size_units_closest(-1, Some(PX), UNIT, None).unwrap(), (0, SizeWhy::NoTarget));
    assert_eq!(size_units_closest(1_000_000, Some(PX), 0, None).err(), Some(FillErr::LotMismatch));
    assert_eq!((SizeWhy::NoReference.code(), SizeWhy::NoTarget.code()), ("no_reference", "no_target"));
    assert!(SizeWhy::NoReference.refused() && SizeWhy::NoTarget.refused());
    assert!(!SizeWhy::Closest.refused());
    assert_eq!((SizeWhy::Closest.code(), SizeWhy::HalfWay.code()), ("closest", "half_way"));
}

/// i64 / u64 extremes: no overflow, no wrap, every boundary exact (u128 products: a lot of up to (2^63 - 1) x (2^64 - 1) paisa).
#[test]
fn c11_i64_and_u64_extremes() {
    let m = i64::MAX;
    assert_eq!(size_units_closest(m, Some(1), 1, None).unwrap(), (m as u64, SizeWhy::Closest), "i64::MAX one-paisa units");
    assert_eq!(size_units_closest(m, Some(m), 1, None).unwrap(), (1, SizeWhy::Closest), "one unit exactly the target");
    assert_eq!(size_units_closest(m, Some(m), 2, None).unwrap(), (0, SizeWhy::Exact2x), "one lot exactly 2 x i64::MAX");
    assert_eq!(size_units_closest(m, Some(1), u64::MAX - 1, None).unwrap(), (0, SizeWhy::Exact2x), "lot 2^64 - 2 = 2 x i64::MAX");
    assert_eq!(size_units_closest(m, Some(1), u64::MAX, None).unwrap(), (0, SizeWhy::Over2x), "lot 2^64 - 1 > 2 x i64::MAX");
    assert_eq!(size_units_closest(m, Some(m), u64::MAX, None).unwrap(), (0, SizeWhy::Over2x), "the largest lot");
    assert_eq!(size_units_closest(1, Some(1), u64::MAX, None).unwrap(), (0, SizeWhy::Over2x));
    // i64::MAX = 2 x 4,611,686,018,427,387,903 + 1: half way between two 2-paisa units: the lower.
    assert_eq!(size_units_closest(m, Some(2), 1, None).unwrap(), (4_611_686_018_427_387_903, SizeWhy::HalfWay));
    assert_eq!(size_units_closest(m, Some(2), 1, Some(m)).unwrap(), (4_611_686_018_427_387_903, SizeWhy::HalfWay), "fits i64::MAX free cash");
    assert_eq!(size_units_closest(m, Some(1), 1, Some(m - 1)).unwrap(), ((m - 1) as u64, SizeWhy::CashShort { closest: m as u64 }));
    assert_eq!(size_units_closest(i64::MIN, Some(1), 1, None).unwrap(), (0, SizeWhy::NoTarget));
    assert_eq!(size_units_closest(m, Some(i64::MIN), 1, None).unwrap(), (0, SizeWhy::NoReference));
    assert_eq!(size_units_closest(m, Some(1), 1, Some(i64::MIN)).err(), Some(FillErr::NegativeFreeCash));
    assert_eq!(size_units_closest(m, Some(m), u64::MAX, Some(i64::MIN)).err(), Some(FillErr::NegativeFreeCash), "a negative free cash is a host error: refused first, whatever the trade");
    assert_eq!(size_units_closest(m, None, 0, Some(-1)).err(), Some(FillErr::LotMismatch), "a lot of 0 first");
}

/// The naive reference, written apart from the function: the counts max(0, q - 2) ..= q + 2 (q = target / lot; the distance
/// |n x lot - target| only grows away from target / lot, so the window holds the closest) by their distance, ties to the lower, in i128
/// with abs (the function uses u128 and the remainder test); then the s16 refusals and the free-cash cap by a BINARY search for the
/// largest count at or below the closest that the free cash pays (the function divides).
fn naive(target: i128, refpx: i128, unit: i128, free: Option<i128>) -> (u64, &'static str, u64) {
    let lot = refpx * unit;
    let q = target / lot;
    let lo = (q - 2).max(0);
    let mut best = lo;
    for n in lo..=q + 2 {
        if (n * lot - target).abs() < (best * lot - target).abs() {
            best = n;
        }
    }
    if best == 0 {
        return (0, if 2 * target == lot { "exact_2x" } else { "over_2x" }, 0);
    }
    // The loop keeps the LOWER of two equally close counts, so a tie is always with the count above.
    let tie = (best * lot - target).abs() == ((best + 1) * lot - target).abs();
    if let Some(f) = free
        && best * lot > f
    {
        // The largest n in [0, best] with n x lot <= f (binary search; n = 0 always pays).
        let (mut a, mut b) = (0_i128, best);
        while a < b {
            let mid = (a + b + 1) / 2;
            if mid * lot <= f { a = mid } else { b = mid - 1 }
        }
        let n = a;
        return ((n * unit) as u64, if n == 0 { "cash_short_zero" } else { "cash_short" }, (best * unit) as u64);
    }
    ((best * unit) as u64, if tie { "half_way" } else { "closest" }, (best * unit) as u64)
}

/// The function equals the naive reference on 300,000 random cases (small and large values, every lot size the index and stock options
/// use, free cash on and off, short and long), and on an exhaustive small grid (every target 1..=400 paisa x reference 1..=40 x unit 1..=3,
/// free cash none or 0..=420 in steps of 7).
#[test]
fn c11_equals_the_naive_reference_random_and_exhaustive() {
    let check = |target: i64, refpx: i64, unit: u64, free: Option<i64>| {
        let (u, why) = size_units_closest(target, Some(refpx), unit, free).unwrap();
        let (nu, ncode, nclosest) = naive(i128::from(target), i128::from(refpx), i128::from(unit), free.map(i128::from));
        assert_eq!((u, why.code()), (nu, ncode), "target {target} ref {refpx} unit {unit} free {free:?}");
        if let SizeWhy::CashShort { closest } | SizeWhy::CashShortZero { closest } = why {
            assert_eq!(closest, nclosest, "closest beside, target {target}");
        }
    };
    let mut g = Rng(0x0c11_5eed);
    let units = [1_u64, 15, 25, 30, 35, 40, 50, 75, 100, 250, 1_800, 900];
    for i in 0..300_000_u64 {
        let target = match i % 3 {
            0 => 1 + g.below(10_000_000) as i64,
            1 => 100_000_000 + g.below(2_000_000_000) as i64,
            _ => 1 + g.below(1_000) as i64,
        };
        let refpx = match i % 4 {
            0 => 1 + g.below(1_000) as i64,
            1 => 1 + g.below(500_000) as i64,
            2 => 1 + g.below(50) as i64,
            _ => 1 + g.below(20_000_000) as i64,
        };
        let unit = units[g.below(units.len() as u64) as usize];
        let free = match g.below(3) {
            0 => None,
            1 => Some(g.below(target as u64 * 2 + 1) as i64),
            _ => Some(target + g.below(1_000) as i64),
        };
        check(target, refpx, unit, free);
    }
    for target in 1..=400_i64 {
        for refpx in 1..=40_i64 {
            for unit in 1..=3_u64 {
                check(target, refpx, unit, None);
                let mut f = 0;
                while f <= 420 {
                    check(target, refpx, unit, Some(f));
                    f += 7;
                }
            }
        }
    }
}

/// The floor function `size_units` is crate v2's, unchanged (its vectors E1 124 / E2 342 are in tests_v2 and tests_g0); beside it the
/// closest count is the floor count or ONE unit more, never anything else, on the same exposure (`exposure` = size_units' own
/// floor(slot x leverage)): one unit more exactly when the remainder is MORE than half a lot.
#[test]
fn c11_floor_unchanged_and_closest_is_floor_or_one_unit_more() {
    // Fixed floor readings (crate v2's arithmetic): Rs 1 lakh x 5 / Rs 4,016.10 = 124.5 -> 124; lots of 75 at Rs 200: 3,333 -> 33 lots.
    assert_eq!(size_units(10_000_000, 5, 1, 401_610, 1), 124);
    assert_eq!(size_units(50_000_000, 1, 1, 20_000, 75), 2_475);
    assert_eq!(size_units(0, 5, 1, 401_610, 1), 0);
    assert_eq!(size_units(10_000_000, 5, 0, 401_610, 1), 0);
    assert_eq!(size_units(10_000_000, 5, 1, 0, 1), 0);
    assert_eq!(size_units(10_000_000, 5, 1, 401_610, 0), 124, "a lot of 0 is read as 1 by the floor function (crate v2, unchanged)");
    // The closest count on the same exposure: 124.5 shares -> half way -> 124 (= floor); 124.6 -> 125 (floor + 1).
    assert_eq!(size_units_closest(exposure(10_000_000, 5, 1).unwrap(), Some(401_610), 1, None).unwrap().0, 124);
    let mut g = Rng(0x0c11_f100);
    for _ in 0..200_000 {
        let slot = 1 + g.below(10_000_000_000) as i64;
        let (num, den) = [(1_u32, 1_u32), (5, 1), (4, 1), (1, 2), (3, 2)][g.below(5) as usize];
        let refpx = 1 + g.below(5_000_000) as i64;
        let unit = [1_u64, 25, 50, 75, 15, 1_800][g.below(6) as usize];
        let fl = size_units(slot, num, den, refpx, unit);
        let e = exposure(slot, num, den).unwrap();
        let (cl, why) = size_units_closest(e, Some(refpx), unit, None).unwrap();
        let lot = i128::from(refpx) * i128::from(unit);
        let r = i128::from(e) % lot;
        if fl == 0 && 2 * r <= lot {
            assert_eq!(cl, 0, "slot {slot} ref {refpx} unit {unit}");
            assert!(why.refused());
        } else if 2 * r > lot {
            assert_eq!(cl, fl + unit, "slot {slot} ref {refpx} unit {unit}: one unit more");
        } else {
            assert_eq!(cl, fl, "slot {slot} ref {refpx} unit {unit}: the floor count");
        }
    }
}

/// `exposure` (C11's helper, the floor(amount x num / den) both sizing functions use): crate v2's arithmetic; None for a denominator of
/// 0, an amount below 0 or a result outside i64 (refused, never wrapped).
#[test]
fn c11_exposure_helper() {
    assert_eq!(exposure(10_000_000, 5, 1), Some(50_000_000));
    assert_eq!(exposure(3, 1, 2), Some(1), "floor");
    assert_eq!(exposure(0, 5, 1), Some(0));
    assert_eq!(exposure(1, 5, 0), None);
    assert_eq!(exposure(-1, 5, 1), None);
    assert_eq!(exposure(i64::MAX, 5, 1), None);
    assert_eq!(exposure(i64::MAX, 1, 1), Some(i64::MAX));
    assert_eq!(exposure(i64::MAX, u32::MAX, u32::MAX), Some(i64::MAX));
}
