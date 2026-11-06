//! Exact integer money: rounding rules and the rupee text (F61 `report::rupees`).
//!
//! Every amount is paisa in `i64` at rest and `i128` in products and sums
//! (CLAUDE.md section 7). Nothing here touches a floating-point value.

/// `floor(n / d)`; `d` must be positive (a non-positive divisor gives 0).
#[must_use]
pub const fn floor_div(n: i128, d: i128) -> i128 {
    if d <= 0 { 0 } else { n.div_euclid(d) }
}

/// `ceil(n / d)`; `d` must be positive (a non-positive divisor gives 0).
#[must_use]
pub const fn ceil_div(n: i128, d: i128) -> i128 {
    if d <= 0 { 0 } else { -((-n).div_euclid(d)) }
}

/// `n / d` rounded to the nearest integer, an exact half upwards (toward plus infinity).
#[must_use]
pub const fn half_up(n: i128, d: i128) -> i128 {
    if d <= 0 { 0 } else { (2 * n + d).div_euclid(2 * d) }
}

/// Paise rounded to the nearest rupee, 50 paise up, returned in paise.
#[must_use]
pub const fn to_rupee(paise: i128) -> i128 {
    half_up(paise, 100) * 100
}

/// Groups the decimal digits of a whole number the Indian way: the last three, then pairs.
fn indian_group(digits: &str) -> String {
    let n = digits.len();
    if n <= 3 {
        return digits.to_owned();
    }
    let (head, tail) = digits.split_at(n - 3);
    let mut out = String::with_capacity(n + n / 2);
    let lead = head.len() % 2;
    for (i, ch) in head.chars().enumerate() {
        if i > 0 && (i + 2 - lead) % 2 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.push(',');
    out.push_str(tail);
    out
}

/// Paisa as rupees with two decimals and Indian digit grouping, by integer division only:
/// `200_000_000` is `"20,00,000.00"`, `-99` is `"-0.99"`.
#[must_use]
pub fn rupees(paise: i128) -> String {
    let sign = if paise < 0 { "-" } else { "" };
    let a = paise.unsigned_abs();
    let whole = indian_group(&(a / 100).to_string());
    format!("{sign}{whole}.{:02}", a % 100)
}

#[cfg(test)]
#[path = "money_tests.rs"]
mod tests;
