//! `sniper_kernel`: the sweep kernel of the ADANIENT sniper (SNIPER-ADANIENT-PLAN v2, section 8; MAIN signed v1 at
//! 2026-11-13 16:12:10 IST with the binding amendments S1-S11).
//!
//! | Module | Role (plan section) |
//! |---|---|
//! | [`ladder`] | a dial's levels as DATA rows (`GE=range:lo:hi:step`, the mirror column for the short side), the exact bucket of a RAW reading (3.1, F24), the OPEN last rung (S8) |
//! | [`line`] | the LINE engine (8.3): one outer combo's kept set walked over every (RSI, %R) line and every CCI level, FIRST mode (the session's first qualifying candle), counted tie runs |
//! | [`bits`] | the outer dials' kept sets as bitsets over the candidate rows (g7's sparse AND, 8.3) |
//! | [`code`] | the canonical 256-bit combo code (3.4): the mixed-radix number of the value indices, M-CCI least significant |
//! | [`steps`] | deterministic per-event step counters and their exact p50 / p99 / max (8.4, the HARD gate) |
//! | [`scale`] | the 1x / 10x / 100x synthetic replication inside the same calendar (8.4) |
//!
//! The kernel never sees money, months or verdicts: it hands every insert, removal and offer to a [`line::Sink`], which is
//! `wf_sweep::SweepAcc` + `wf_sweep::Lists` in the host (the `common_wf` additions of decision D-S14). So no key law lives here.
//!
//! Rules: std only, no dependency, no I/O, no float. The cargo feature `red` swaps in the OLD reading (CUBE's suffix reading,
//! which counts every qualifying candle of a session: B1's defect); `m_s2`, `m_s5`, `m_s19` are the named mutants. A host
//! refuses a search run built with any of them ([`refuse_if_red`]).
#![cfg_attr(test, allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod bits;
pub mod code;
pub mod ladder;
pub mod line;
pub mod scale;
pub mod steps;
#[cfg(test)]
mod tests;

/// True in a red or mutant build.
pub const RED: bool = cfg!(any(feature = "red", feature = "m_s2", feature = "m_s5", feature = "m_s19"));

/// The kernel's law tag, printed on every result.
#[must_use]
pub const fn law() -> &'static str {
    if cfg!(feature = "red") {
        "sniper-kernel-v1-RED"
    } else if RED {
        "sniper-kernel-v1-MUTANT"
    } else {
        "sniper-kernel-v1"
    }
}

/// Every search door calls this first.
///
/// # Errors
/// Refuses a red or mutant build.
pub const fn refuse_if_red() -> Result<(), &'static str> {
    if RED { Err("sniper_kernel built with --features red or a named mutant: no search run") } else { Ok(()) }
}
