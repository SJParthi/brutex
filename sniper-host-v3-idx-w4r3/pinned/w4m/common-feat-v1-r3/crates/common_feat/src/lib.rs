//! `common_feat` v1: THE common feature module of Brutex (REQ 72; REQ72-PLAN v4 section A, signed by MAIN
//! 2026-11-13 15:50:46 IST with the binding amendments A1-A4), the minimum slice the ADANIENT sniper reads
//! (REQ 76f; SNIPER-REQUIREMENTS sections 11 and 13; SNIPER-ADANIENT-PLAN 1.1 "`common_feat` v1, minimum slice").
//!
//! # What it is
//!
//! Pure integer functions over candles. Std only, no I/O, no float (the workspace denies `float_arithmetic`).
//! Prices are `i64` paisa; sums and products `i128`; volumes `u64`; `div_euclid` everywhere unless a function names
//! another rounding. Every reading is RAW: an exact rational ([`num::Ratio`]) or the module's fixed-point state at
//! `S = 10^6` ([`S`]), never a bit, so a threshold ladder (RSI 70, 71 ... 100) is one integer comparison a level
//! (REQ72-PLAN A.6) and the bit "value >= q" is decided exactly on the state (SNIPER-ADANIENT-PLAN 3.1, F24).
//!
//! # The laws it carries (REQ72-PLAN A.3)
//!
//! - K1: a candle reading is known at its candle's END; a higher-timeframe value is the one of its last COMPLETED
//!   candle at the decision instant ([`mtf`]).
//! - K6: a zero denominator is "not ready", never a value (flat RSI window, zero range, zero mean deviation).
//! - K13: corporate actions: factors only from events dated on or before the decision session ([`ca`]); CONT streams
//!   reset at a factor ex-date; a back-adjusted series is undone before any reading ([`ca::undo_factor`]).
//! - K14: every "from the open" boundary reads the dated session table's open ([`cal`], [`fold`]).
//! - K15: patterns read same-session candles only ([`anatomy`]).
//!
//! # Cost
//!
//! O(1) a candle: every state is a fixed ring, a running sum or a monotone deque; the named bounded exceptions are
//! CCI's mean absolute deviation (n <= 64 terms) and %R's deque (amortised O(1), worst n pops in one call). Each fold
//! returns its deterministic step count ([`steps::Steps`]), so a host can assert identical per-event counts at 1x,
//! 10x and 100x data (SNIPER-ADANIENT-PLAN 8.4, the HARD gate).

pub mod anatomy;
pub mod ca;
pub mod cal;
pub mod fold;
pub mod ind;
pub mod levels;
pub mod mtf;
pub mod num;
pub mod rvol;
pub mod series;
pub mod steps;

/// The fixed-point scale of every reading held as a state (REQ72-DESIGN A.4: `S = 10^6`, `indicators::SCALE`).
pub const S: i128 = 1_000_000;

/// The build's law tag, printed by every host on every result. `red` builds print `common-feat-v1-RED`, and a host
/// refuses a headline or search run built with it ([`refuse_if_red`]).
#[cfg(not(any(
    feature = "red",
    feature = "m_mtf_forming",
    feature = "m_pattern_cross",
    feature = "m_cidx_0915",
    feature = "m_ca_future",
    feature = "m_rvol_self",
    feature = "m_rsi_flat_ready",
    feature = "m_mtf_reset_cross",
    feature = "m_halt_ignored",
    feature = "m_cidx_floor",
    feature = "m_host"
)))]
pub const LAW: &str = "common-feat-v1";
/// The build's law tag under a red or mutant build.
#[cfg(any(
    feature = "red",
    feature = "m_mtf_forming",
    feature = "m_pattern_cross",
    feature = "m_cidx_0915",
    feature = "m_ca_future",
    feature = "m_rvol_self",
    feature = "m_rsi_flat_ready",
    feature = "m_mtf_reset_cross",
    feature = "m_halt_ignored",
    feature = "m_cidx_floor",
    feature = "m_host"
))]
pub const LAW: &str = "common-feat-v1-RED";

/// True when this build is the red build or carries a mutant.
pub const RED: bool = !matches!(LAW.as_bytes(), b"common-feat-v1");

/// A host calls this before any headline or search run.
///
/// # Errors
///
/// Refuses (with the reason) when the build is red or carries a named mutant.
pub const fn refuse_if_red() -> Result<(), &'static str> {
    if RED {
        Err("common_feat built with --features red or a named mutant: no headline or search run")
    } else {
        Ok(())
    }
}
