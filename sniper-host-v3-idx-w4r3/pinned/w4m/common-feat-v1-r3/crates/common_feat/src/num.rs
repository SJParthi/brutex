//! Exact integer arithmetic: rationals, the floor and ceiling at `S`, and the integer square root.

use crate::S;

/// An exact rational `num / den` with `den > 0`. Every RAW reading that is a ratio (RSI, CCI, %R, body share ...)
/// is one of these, so "reading >= level" is decided by cross-multiplication, never by a rounded value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ratio {
    /// The numerator.
    pub num: i128,
    /// The denominator, always `> 0`.
    pub den: i128,
}

impl Ratio {
    /// `num / den`, or `None` when `den == 0` (K6: a zero denominator is "not ready", never a value). A negative
    /// denominator is folded into the numerator's sign.
    #[must_use]
    pub const fn new(num: i128, den: i128) -> Option<Self> {
        if den == 0 {
            None
        } else if den < 0 {
            Some(Self { num: -num, den: -den })
        } else {
            Some(Self { num, den })
        }
    }

    /// `floor(value x S)`: the reading in millionths, rounded toward minus infinity (the exact 1e-6 floor bucket).
    #[must_use]
    pub const fn floor_s(self) -> i128 {
        (self.num * S).div_euclid(self.den)
    }

    /// `ceil(value x S)`.
    #[must_use]
    pub const fn ceil_s(self) -> i128 {
        -((-self.num * S).div_euclid(self.den))
    }

    /// `value >= q` for an integer level `q`, exactly.
    #[must_use]
    pub const fn ge_int(self, q: i128) -> bool {
        self.num >= q * self.den
    }

    /// `value <= q` for an integer level `q`, exactly.
    #[must_use]
    pub const fn le_int(self, q: i128) -> bool {
        self.num <= q * self.den
    }

    /// `value >= a / b` exactly (`b > 0`).
    #[must_use]
    pub const fn ge(self, a: i128, b: i128) -> bool {
        self.num * b >= a * self.den
    }

    /// `value <= a / b` exactly (`b > 0`).
    #[must_use]
    pub const fn le(self, a: i128, b: i128) -> bool {
        self.num * b <= a * self.den
    }
}

/// `floor(sqrt(x))` for `x >= 0` (Newton on `u128`), `0` for a negative `x`. Display only: no bit reads it (the
/// Bollinger bits square both sides instead).
#[must_use]
pub fn isqrt(x: i128) -> i128 {
    let Ok(v) = u128::try_from(x) else {
        return 0;
    };
    if v < 2 {
        return x;
    }
    // A start at or above the root: 2^ceil(bits / 2).
    let bits = 128 - v.leading_zeros();
    let mut r: u128 = 1u128 << bits.div_ceil(2);
    loop {
        let next = r.midpoint(v / r);
        if next >= r {
            break;
        }
        r = next;
    }
    while r * r > v {
        r -= 1;
    }
    while (r + 1) * (r + 1) <= v {
        r += 1;
    }
    i128::try_from(r).unwrap_or(i128::MAX)
}

/// `x` (paisa) times `num / den`, snapped to the NEAREST multiple of `tick` paisa, a half rounded up. This is the one
/// rounding of [`crate::ca::undo_factor`]: a back-adjusted price `z = round(raw x f)` maps back to the raw tick grid.
#[must_use]
pub fn mul_snap(x: i64, num: i128, den: i128, tick: i64) -> i64 {
    if den <= 0 || tick <= 0 {
        return x;
    }
    let t = i128::from(tick);
    // floor((x num / den) / t + 1/2) * t  =  floor((2 x num + t den) / (2 t den)) * t
    let q = (2 * i128::from(x) * num + t * den).div_euclid(2 * t * den);
    i64::try_from(q * t).unwrap_or(x)
}

/// A value held at `S` (paisa x 10^6) printed as rupees with two decimals, rounded half away from zero (display only).
#[must_use]
pub fn rupees_s(v: i128) -> String {
    // paisa x S -> rupee cents x ... : rupees = v / (100 S); two decimals = v / S paisa.
    let paisa_x100 = round_div(v * 100, S); // hundredths of a paisa
    let neg = paisa_x100 < 0;
    let a = paisa_x100.abs();
    // a is in 1e-4 rupees; two decimals = round(a / 100)
    let cents = round_div(a, 100);
    format!("{}{}.{:02}", if neg { "-" } else { "" }, cents / 100, cents % 100)
}

/// A value held at `S` in natural units (RSI points, CCI points ...) printed with `dp` decimals, half away from zero.
#[must_use]
pub fn units_s(v: i128, dp: u32) -> String {
    let scale = 10i128.pow(dp);
    let x = round_div(v * scale, S);
    let neg = x < 0;
    let a = x.abs();
    if dp == 0 {
        return format!("{}{}", if neg { "-" } else { "" }, a);
    }
    format!("{}{}.{:0w$}", if neg { "-" } else { "" }, a / scale, a % scale, w = dp as usize)
}

/// Paisa printed as rupees with two decimals.
#[must_use]
pub fn rupees(p: i64) -> String {
    let neg = p < 0;
    let a = i128::from(p).abs();
    format!("{}{}.{:02}", if neg { "-" } else { "" }, a / 100, a % 100)
}

/// `round(a / b)`, half away from zero, `b > 0`.
#[must_use]
pub const fn round_div(a: i128, b: i128) -> i128 {
    if a >= 0 { (2 * a + b).div_euclid(2 * b) } else { -((-2 * a + b).div_euclid(2 * b)) }
}
