//! Money in paise, exact integer division and the rupee printer.
//!
//! Every price and amount is a paise integer (`i64`), every sum an `i128`, and
//! every division names its rounding: half-up for a charge line (COSTS.md
//! section 5), floor or ceil where the kit says "never kinder".

/// Paise in one rupee.
pub const RUPEE: i64 = 100;
/// Rs 1,00,000 in paise.
pub const LAKH: i64 = 100_000 * RUPEE;
/// Rs 1,00,00,000 in paise.
pub const CRORE: i64 = 100 * LAKH;

/// `n / d` rounded toward negative infinity, for `d > 0`.
///
/// # Panics
/// When `d <= 0`: every caller divides by a positive count or price.
pub fn floor_div(n: i128, d: i128) -> i128 {
    assert!(d > 0, "floor_div needs a positive divisor, got {d}");
    n.div_euclid(d)
}

/// `n / d` rounded toward positive infinity, for `d > 0`.
///
/// # Panics
/// When `d <= 0`.
pub fn ceil_div(n: i128, d: i128) -> i128 {
    assert!(d > 0, "ceil_div needs a positive divisor, got {d}");
    -(-n).div_euclid(d)
}

/// `n / d` rounded half-up (an exact half goes up), for `n >= 0`, `d > 0`:
/// COSTS.md section 5's "half-up to the paisa".
///
/// # Panics
/// When `n < 0` or `d <= 0`: a charge is never negative.
pub fn half_up(n: i128, d: i128) -> i128 {
    assert!(
        n >= 0 && d > 0,
        "half_up takes n >= 0 and d > 0, got {n} / {d}"
    );
    (2 * n + d) / (2 * d)
}

/// A paise amount rounded half-up to the whole rupee, still in paise
/// ("rounded off to the nearest rupee, 50 paise up").
pub fn to_rupee(paise: i128) -> i128 {
    half_up(paise, i128::from(RUPEE)) * i128::from(RUPEE)
}

/// Prints paise as rupees with two decimals: `-120.05`, `0.00`, `57941` as
/// `579.41`. Used only for the golden files' readable lines.
pub fn rs(paise: i128) -> String {
    let sign = if paise < 0 { "-" } else { "" };
    let abs = paise.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

/// Saturating narrow of an exact `i128` to `i64` for storage of figures the
/// kit bounds far below `i64::MAX` (a day's money); saturation is printed by
/// the caller as a refusal, never silently.
pub fn narrow(v: i128) -> Option<i64> {
    i64::try_from(v).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_and_ceil_cover_signs_and_exact_cases() {
        // eight extremes
        assert_eq!(floor_div(7, 2), 3);
        assert_eq!(floor_div(-7, 2), -4);
        assert_eq!(floor_div(-8, 2), -4);
        assert_eq!(floor_div(0, 5), 0);
        assert_eq!(ceil_div(7, 2), 4);
        assert_eq!(ceil_div(-7, 2), -3);
        assert_eq!(ceil_div(8, 2), 4);
        assert_eq!(ceil_div(0, 9), 0);
        // the largest magnitudes the money path can hold
        assert_eq!(floor_div(i128::from(i64::MAX), 1), i128::from(i64::MAX));
        assert_eq!(ceil_div(i128::from(i64::MIN), 1), i128::from(i64::MIN));
        // combination: floor(x) + ceil(-x) == 0 for every x
        for x in -20..=20 {
            assert_eq!(floor_div(x, 3) + ceil_div(-x, 3), 0);
        }
    }

    #[test]
    #[should_panic(expected = "positive divisor")]
    fn floor_div_refuses_a_zero_divisor() {
        let _ = floor_div(1, 0);
    }

    #[test]
    #[should_panic(expected = "positive divisor")]
    fn ceil_div_refuses_a_negative_divisor() {
        let _ = ceil_div(1, -3);
    }

    #[test]
    fn half_up_rounds_exact_halves_up_and_nothing_else() {
        assert_eq!(half_up(5, 10), 1); // 0.5 -> 1
        assert_eq!(half_up(4, 10), 0); // 0.4 -> 0
        assert_eq!(half_up(15, 10), 2); // 1.5 -> 2
        assert_eq!(half_up(25, 10), 3); // 2.5 -> 3, never banker's
        assert_eq!(half_up(0, 7), 0);
        assert_eq!(half_up(6, 7), 1);
        assert_eq!(half_up(3, 7), 0);
        assert_eq!(half_up(7, 7), 1);
        // COSTS.md CN01's exchange line: 32.663505 rupees -> 3266 paise
        assert_eq!(half_up(100_500_000 * 32_501, 1_000_000_000), 3266);
        // and the SEBI line 150.75 paise -> 151
        assert_eq!(half_up(100_500_000 * 1_500, 1_000_000_000), 151);
    }

    #[test]
    #[should_panic(expected = "half_up takes")]
    fn half_up_refuses_a_negative_numerator() {
        let _ = half_up(-1, 2);
    }

    #[test]
    fn to_rupee_and_rs_print_the_kit_figures() {
        assert_eq!(to_rupee(12_563), 12_600); // 125.63 -> 126
        assert_eq!(to_rupee(12_449), 12_400); // 124.49 -> 124
        assert_eq!(to_rupee(12_450), 12_500); // 124.50 -> 125, 50 paise up
        assert_eq!(to_rupee(0), 0);
        assert_eq!(rs(57_941), "579.41");
        assert_eq!(rs(-12_005), "-120.05");
        assert_eq!(rs(0), "0.00");
        assert_eq!(rs(7), "0.07");
        assert_eq!(rs(-7), "-0.07");
        assert_eq!(LAKH, 10_000_000);
        assert_eq!(CRORE, 1_000_000_000);
    }

    #[test]
    fn narrow_saturates_by_refusing() {
        assert_eq!(narrow(5), Some(5));
        assert_eq!(narrow(i128::from(i64::MAX)), Some(i64::MAX));
        assert_eq!(narrow(i128::from(i64::MAX) + 1), None);
        assert_eq!(narrow(i128::from(i64::MIN) - 1), None);
    }
}
