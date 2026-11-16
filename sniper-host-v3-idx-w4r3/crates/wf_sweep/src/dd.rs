//! The drawdown monoid: `common_wf::dd::Dd` itself (LINKED since common_wf v1's SHA256SUMS landed, 2026-11-13 17:10:25 IST), so
//! the sweep tree composes with the module's own law; only two adapters live here. Mutant `m_s6` (the tree composes right to
//! left) lives in the tree's node product (`acc.rs`).

pub use common_wf::dd::Dd;

use crate::SweepErr;

/// One step as a monoid element.
///
/// # Errors
/// `Overflow`.
pub fn one(x: i64) -> Result<Dd, SweepErr> {
    let mut d = Dd::ZERO;
    d.push(x).map_err(|_| SweepErr::Overflow)?;
    Ok(d)
}

/// `a` followed by `b`, through the module's product.
///
/// # Errors
/// `Overflow`.
pub fn combine(a: &Dd, b: &Dd) -> Result<Dd, SweepErr> {
    a.combine(b).map_err(|_| SweepErr::Overflow)
}
