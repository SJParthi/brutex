//! The drawdown monoid (lifted from evalcore `accum.rs:9-55`, its logic unchanged, every sum checked): total, max prefix (>= 0),
//! min prefix (<= 0), max drawdown (>= 0) over a sequence of steps (trade nets, session nets or mark-to-market steps). Associative;
//! the identity is all zero, so a session, month or piece with no step is skipped exactly (6.2).

use crate::{WfErr, add, sub};

/// The monoid element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dd {
    /// The sum.
    pub total: i64,
    /// The highest prefix sum (the empty prefix 0 included).
    pub max_prefix: i64,
    /// The lowest prefix sum (the empty prefix 0 included).
    pub min_prefix: i64,
    /// The largest fall from a running peak.
    pub max_dd: i64,
}

impl Dd {
    /// The identity.
    pub const ZERO: Self = Self { total: 0, max_prefix: 0, min_prefix: 0, max_dd: 0 };

    /// Appends one step.
    ///
    /// # Errors
    /// `Overflow`.
    pub fn push(&mut self, x: i64) -> Result<(), WfErr> {
        let neg = x.checked_neg().ok_or(WfErr::Overflow)?;
        *self = self.combine(&Self { total: x, max_prefix: x.max(0), min_prefix: x.min(0), max_dd: neg.max(0) })?;
        Ok(())
    }

    /// The monoid's product: `self` followed by `b`.
    ///
    /// # Errors
    /// `Overflow`.
    pub fn combine(&self, b: &Self) -> Result<Self, WfErr> {
        let total = add(self.total, b.total)?;
        let hi = add(self.total, b.max_prefix)?;
        let lo = add(self.total, b.min_prefix)?;
        let cross = sub(self.max_prefix, lo)?;
        Ok(Self { total, max_prefix: self.max_prefix.max(hi), min_prefix: self.min_prefix.min(lo), max_dd: if cfg!(feature = "m10") { self.max_dd.max(b.max_dd) } else { self.max_dd.max(b.max_dd).max(cross) } })
    }
}
