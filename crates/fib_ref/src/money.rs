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
#[path = "money_tests.rs"]
mod tests;
