#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::rng::SplitMix64;

#[test]
fn rounding_rules() {
    assert_eq!(floor_div(7, 2), 3);
    assert_eq!(floor_div(-7, 2), -4);
    assert_eq!(ceil_div(7, 2), 4);
    assert_eq!(ceil_div(-7, 2), -3);
    assert_eq!(ceil_div(6, 2), 3);
    assert_eq!(half_up(5, 2), 3);
    assert_eq!(half_up(-5, 2), -2);
    assert_eq!(half_up(4, 3), 1);
    assert_eq!(to_rupee(12_550), 12_600);
    assert_eq!(to_rupee(12_549), 12_500);
    for f in [floor_div, ceil_div, half_up] {
        assert_eq!(f(5, 0), 0);
        assert_eq!(f(5, -1), 0);
    }
}

/// X-F61: the rupee text at its edges.
#[test]
fn x_f61_rupees_extremes() {
    let cases: [(i128, &str); 12] = [
        (0, "0.00"),
        (-1, "-0.01"),
        (5, "0.05"),
        (123_456, "1,234.56"),
        (200_000_000, "20,00,000.00"),
        (-1_000_000_000, "-1,00,00,000.00"), // combo: negative and a crore
        (-99, "-0.99"),                      // combo: negative under one rupee
        (99_999, "999.99"),
        (100_000, "1,000.00"),
        (i128::from(i64::MIN), "-92,23,37,20,36,85,47,758.08"), // oob
        (i128::MAX, "17,01,41,18,34,60,46,92,31,73,16,87,30,37,15,88,41,057.27"),
        (i128::MIN, "-17,01,41,18,34,60,46,92,31,73,16,87,30,37,15,88,41,057.28"),
    ];
    for (p, want) in cases {
        assert_eq!(rupees(p), want, "{p}");
    }
}

/// The naive digit model: every digit by repeated division, grouping by position.
fn digits_model(p: i128) -> String {
    let neg = p < 0;
    let mut a = p.unsigned_abs();
    let paise = a % 100;
    a /= 100;
    let mut groups: Vec<String> = Vec::new();
    let mut first = true;
    loop {
        let width = if first { 1_000 } else { 100 };
        let part = a % width;
        a /= width;
        if a == 0 {
            groups.push(part.to_string());
            break;
        }
        groups.push(if first { format!("{part:03}") } else { format!("{part:02}") });
        first = false;
    }
    groups.reverse();
    format!("{}{}.{paise:02}", if neg { "-" } else { "" }, groups.join(","))
}

/// P-F61-1: 10,000 values against the digit model.
#[test]
fn p_f61_rupees_vs_digit_model() {
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        for _ in 0..2_500 {
            let down = u32::try_from(r.below(64)).unwrap();
            let up = u32::try_from(r.below(63)).unwrap();
            let v = i128::from(r.next_u64() >> down) << up;
            let v = if r.chance(1, 2) { -v } else { v };
            assert_eq!(rupees(v), digits_model(v), "{v}");
        }
    }
}
