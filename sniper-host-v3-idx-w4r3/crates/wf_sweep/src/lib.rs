//! `wf_sweep`: the two `common_wf` ADDITIONS of decision D-S14 (SNIPER-ADANIENT-PLAN v2 section 8.3), STAGED here.
//!
//! - [`acc::SweepAcc`]: the learning cells of `common_wf`'s `Acc` (year parts, months, the order-free fingerprint, the
//!   per-part trade floors, the year-part session trees of section 7) for C columns at once, built for one window set and its
//!   learning hull; `insert` / `remove` in ANY order; `key(window, column, floor)` gives the REQ 73 key parts on demand.
//!   It refuses any trade outside its learning hull ([`SweepErr::TestMonthInSweep`]), so a test month can never reach a pick
//!   (L-INV by type, P20).
//! - [`lists::Lists::offer_learn`]: a tie run of n consecutive codes with one key, offered once, exactly equal to n single
//!   offers (OR-EQ), the tie counters advanced by n (P18: no silent dedupe).
//!
//! **Why staged here.** MAIN's 15:36:02 signature scopes `common_wf` v1 to its core, built by its own builder in
//! `scratch/common-wf-v1`; decision D-S14 adds these two parts "which that scope does not name". This builder may not write in
//! that tree. The code is written to be lifted verbatim into `common_wf` (its `dd` module is the same monoid as
//! `common_wf::dd::Dd`, lifted from evalcore `accum.rs:9-55`), where SWEEP-EQ and OR-EQ then run against `common_wf`'s own
//! `Acc` and `offer`. common_wf v1 is LINKED (its SHA256SUMS landed 2026-11-13 17:10:25 IST): the tree element is `common_wf::dd::Dd`, and
//! SWEEP-EQ runs against `common_wf::Acc` + `crit_val` (`tests_wf.rs`) beside the independent plain recomputation (`tests.rs`).
//!
//! Rules: std only, no I/O, no float; every sum checked (a refusal, never a wrap). `red` = the lenient reading; `m_s1`, `m_s6`,
//! `m_s14` are the named mutants.
#![cfg_attr(test, allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod acc;
pub mod dd;
pub mod lists;
pub mod views;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_fast;
#[cfg(test)]
mod tests_wf;

/// Money: i64 paisa, net after every charge.
pub type Paisa = i64;
/// A month key: year * 12 + (month - 1) (as `common_wf`).
pub type Mk = i32;
/// The most columns (share levels) one accumulator carries (crate v2 `MAXL`).
pub const MAXC: usize = 8;

/// True in a red or mutant build.
pub const RED: bool = cfg!(any(feature = "red", feature = "m_s1", feature = "m_s6", feature = "m_s14", feature = "m_k17"));

/// Every search door calls this first.
///
/// # Errors
/// Refuses a red or mutant build.
pub const fn refuse_if_red() -> Result<(), SweepErr> {
    if RED { Err(SweepErr::RedBuild) } else { Ok(()) }
}

/// Every refusal, named (counted by the host).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SweepErr {
    /// A trade outside the learning hull (P20): a test month can never reach a pick.
    TestMonthInSweep,
    /// A checked sum left its type.
    Overflow,
    /// A row the outcome table does not hold.
    UnknownRow,
    /// A second trade in one session leaf (FIRST mode holds one trade a session and side; EVERY mode is walked by
    /// `common_wf`'s `Acc` on the shortlist only).
    LeafOccupied,
    /// A removal of a trade that is not held.
    NotHeld,
    /// A window whose learning months are not whole calendar years after the hull's first month (the per-window year units,
    /// common_wf DV1), or outside the hull.
    BadWindow(String),
    /// A views row refused by its rule.
    BadView(String),
    /// More columns than [`MAXC`].
    TooManyColumns,
    /// A search door of a red or mutant build.
    RedBuild,
}

impl std::fmt::Display for SweepErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SweepErr {}

/// The `Overflow` refusal, built only when it is returned (speed build: `ok_or(SweepErr::Overflow)` built and dropped the
/// error on every successful call; `ok_or_else(overflow)` never builds it on the success path).
#[inline]
pub(crate) const fn overflow() -> SweepErr {
    SweepErr::Overflow
}

/// The `UnknownRow` refusal, built only when it is returned (see [`overflow`]).
#[inline]
pub(crate) const fn unknown_row() -> SweepErr {
    SweepErr::UnknownRow
}

/// Checked i64 addition.
#[inline]
pub(crate) fn add(a: i64, b: i64) -> Result<i64, SweepErr> {
    a.checked_add(b).ok_or_else(overflow)
}

/// Checked i64 subtraction.
#[inline]
pub(crate) fn sub(a: i64, b: i64) -> Result<i64, SweepErr> {
    a.checked_sub(b).ok_or_else(overflow)
}

/// The 64-bit mixer of the order-free learning fingerprint (as `common_wf::mix`).
#[must_use]
pub const fn mix(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}
