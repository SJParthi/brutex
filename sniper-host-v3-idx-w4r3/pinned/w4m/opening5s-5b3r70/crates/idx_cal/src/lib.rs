//! idx_cal: build step S1 of SNIPER-IDXOPT-PLAN-v3.1 (index options, NIFTY / BANKNIFTY), read only on every input.
//!
//! * [`cal`]: DP-IDXCAL, the dated NSE expiry calendar from the F&O bhavcopy listings, with true expiry days, re-dated
//!   series joined by contract identity and the G-CAL truth assertion.
//! * [`bhav`]: the raw bhavcopy zips (contract identity, `UndrlygPric`).
//! * [`storeio`]: the store's read-only door (copied).
//! * [`ticks`] / [`crc`]: the GDFL tick store reader (copied from feat_io, one split).
//! * [`fno_end`]: the dated F&O-end rows.

pub mod bhav;
pub mod cal;
pub mod crc;
pub mod day;
pub mod listing;
pub mod storeio;
pub mod ticks;

/// The dated F&O session end (plan s3.1 (6), B5; the robot's `options.rs` `session_end` :233-241): 15:30:00, or
/// 15:40:00 from 2026-08-03 (measured in the store's option prints; NSE circular not on disk), as seconds of day. The
/// special sessions' own segment ends come from `SESSIONS-SPECIAL-v1.tsv` (the caller's), never from here.
pub mod fno_end {
    use crate::day::{self, Day};

    /// 15:30:00.
    pub const CLOSE_S: i64 = 15 * 3600 + 30 * 60;
    /// 15:40:00.
    pub const CLOSE_CAS_S: i64 = 15 * 3600 + 40 * 60;

    /// The first day of the 15:40 era.
    #[must_use]
    pub const fn era_start() -> Day {
        day::from_civil(2026, 8, 3)
    }

    /// The regular-session F&O end of day `d` (seconds of day, exclusive).
    #[must_use]
    pub const fn regular_end(d: Day) -> i64 {
        if d >= era_start() { CLOSE_CAS_S } else { CLOSE_S }
    }

    /// The index close (entries stop strictly before it in every era).
    pub const INDEX_CLOSE_S: i64 = CLOSE_S;
}
