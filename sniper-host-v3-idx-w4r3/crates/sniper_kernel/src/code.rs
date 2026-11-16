//! The canonical combo code (SNIPER-ADANIENT-PLAN 3.4): `common_wf`'s `Code`, 256 bits, the mixed-radix number of the value
//! indices in the sealed dials order, M-CCI the least significant digit, so consecutive CCI levels have consecutive codes (the
//! tie runs of 8.3). The full product with HELD and LATE is at most 3.99 x 10^75 < 2^256, so every combo has one code.

use std::cmp::Ordering;

/// A 256-bit code, little-endian 64-bit limbs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Code(pub [u64; 4]);

impl Ord for Code {
    fn cmp(&self, o: &Self) -> Ordering {
        for i in (0..4).rev() {
            let (a, b) = (self.0.get(i).copied().unwrap_or(0), o.0.get(i).copied().unwrap_or(0));
            match a.cmp(&b) {
                Ordering::Equal => {}
                x => return x,
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for Code {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Code {
    /// `self * m + a`, or `None` past 2^256.
    #[must_use]
    pub fn mul_add(self, m: u64, a: u64) -> Option<Self> {
        let mut out = [0_u64; 4];
        let mut carry = u128::from(a);
        for (i, o) in out.iter_mut().enumerate() {
            let x = u128::from(self.0.get(i).copied().unwrap_or(0)) * u128::from(m) + carry;
            *o = u64::try_from(x & u128::from(u64::MAX)).ok()?;
            carry = x >> 64;
        }
        if carry != 0 { None } else { Some(Self(out)) }
    }

    /// The mixed-radix code of `digits` = (value index, radix) LEAST significant first (M-CCI first). `None` when a digit is
    /// not below its radix or the code leaves 256 bits.
    #[must_use]
    pub fn mixed(digits: &[(u64, u64)]) -> Option<Self> {
        let mut acc = Self::default();
        for &(d, r) in digits.iter().rev() {
            if d >= r {
                return None;
            }
            acc = acc.mul_add(r, d)?;
        }
        Some(acc)
    }

    /// `self + x`, or `None` past 2^256.
    #[must_use]
    pub fn plus(self, x: u64) -> Option<Self> {
        self.mul_add(1, x)
    }

    /// `(self / m, self % m)` for a small divisor (`None` for 0): the inverse of [`Self::mul_add`].
    #[must_use]
    pub fn divmod(self, m: u64) -> Option<(Self, u64)> {
        if m == 0 {
            return None;
        }
        let mut out = [0_u64; 4];
        let mut rem = 0_u128;
        for i in (0..4).rev() {
            let cur = (rem << 64) | u128::from(self.0.get(i).copied().unwrap_or(0));
            let q = cur / u128::from(m);
            rem = cur % u128::from(m);
            *out.get_mut(i)? = u64::try_from(q).ok()?;
        }
        Some((Self(out), u64::try_from(rem).ok()?))
    }

    /// The digits of a mixed-radix code, LEAST significant first (the inverse of [`Self::mixed`]); `None` when the code holds
    /// more than the radices can (a code from another dials file).
    #[must_use]
    pub fn digits(self, radices: &[u64]) -> Option<Vec<u64>> {
        let mut acc = self;
        let mut out = Vec::with_capacity(radices.len());
        for &r in radices {
            let (q, d) = acc.divmod(r)?;
            out.push(d);
            acc = q;
        }
        if acc == Self::default() { Some(out) } else { None }
    }

    /// 64 hex digits, most significant first.
    #[must_use]
    pub fn hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for i in (0..4).rev() {
            s.push_str(&format!("{:016x}", self.0.get(i).copied().unwrap_or(0)));
        }
        s
    }
}
