//! One contract-day's 1-second rows from the store (`bars/gdfl/NSE/FNO/<index>/<folder>/1s`, generation-aware door): valid
//! prints only (volume > 0, 0 < LOW <= HIGH), the derivatives halts cut, as the idx1s tool and its checkers read them.
//! (The corrected seconds of DP-NEWPX-IDXOPT, from the raw ticks, replace this source in the production pack: open item.)

use std::path::Path;

use brutex_core::vendor::Vendor;
use common_fill::tape::Row;
use idx_cal::day::{self, Day};
use idx_cal::storeio::{self, at_sod};
use store::path::Timeframe;

/// One second.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sec {
    /// Start, UTC micros.
    pub ts: i64,
    /// HIGH, paisa.
    pub high: i64,
    /// LOW.
    pub low: i64,
    /// CLOSE.
    pub close: i64,
    /// Volume, units.
    pub vol: u64,
}

impl Row for Sec {
    fn ts(&self) -> i64 {
        self.ts
    }
    fn high(&self) -> i64 {
        self.high
    }
    fn low(&self) -> i64 {
        self.low
    }
    fn vol(&self) -> u64 {
        self.vol
    }
}

fn hms(s: &str) -> i64 {
    let p: Vec<i64> = s.split(':').filter_map(|x| x.parse().ok()).collect();
    p.first().copied().unwrap_or(0) * 3600 + p.get(1).copied().unwrap_or(0) * 60 + p.get(2).copied().unwrap_or(0)
}

/// The derivatives halt window of a day (from, resume), UTC micros (the idx1s tool's session facts; K14).
#[must_use]
pub fn halt_der(d: Day) -> Option<(i64, i64)> {
    let w = match day::fmt(d).as_str() {
        "2020-03-13" => ("09:21:45", "10:21:00"),
        "2020-03-23" => ("09:58:01", "10:57:00"),
        "2021-02-24" => ("11:40:00", "15:45:00"),
        _ => return None,
    };
    Some((at_sod(d, hms(w.0)), at_sod(d, hms(w.1))))
}

/// The index halt window (the index seconds' own; the idx1s tool's `HALT_IDX`).
#[must_use]
pub fn halt_idx(d: Day) -> Option<(i64, i64)> {
    let w = match day::fmt(d).as_str() {
        "2020-03-13" => ("09:21:10", "10:21:01"),
        "2020-03-23" => ("09:58:09", "10:57:01"),
        "2021-02-24" => ("11:40:00", "15:45:00"),
        _ => return None,
    };
    Some((at_sod(d, hms(w.0)), at_sod(d, hms(w.1))))
}

/// The dated F&O end of a day (exclusive, UTC micros): 17:00:00 on 2021-02-24 (outage day), 15:40:00 from 2026-08-03, else
/// 15:30:00 (S1's FNO-END-ROWS for regular days).
#[must_use]
pub fn fno_end(d: Day) -> i64 {
    if day::fmt(d) == "2021-02-24" {
        at_sod(d, 17 * 3600)
    } else {
        at_sod(d, idx_cal::fno_end::regular_end(d))
    }
}

/// The 15:10:00 square-off start, or the halt's resume when 15:10 falls inside a halt.
#[must_use]
pub fn sq0(d: Day) -> i64 {
    let s = at_sod(d, 15 * 3600 + 10 * 60);
    match halt_der(d) {
        Some((a, b)) if s >= a && s < b => b,
        _ => s,
    }
}

/// The option rows of a contract folder on day `d` in `[09:15:00, F&O end)`. `Ok(None)` when the month file is missing.
///
/// # Errors
/// A refused month or a store read error.
pub fn option_day(root: &Path, index: &str, folder: &str, d: Day) -> Result<Option<Vec<Sec>>, String> {
    let to = (fno_end(d) - at_sod(d, 0)) / 1_000_000;
    let Some(bars) = storeio::day_bars(root, Vendor::Gdfl, "FNO", index, Some(folder), Timeframe::SECOND_1, d, 9 * 3600 + 15 * 60, to)? else { return Ok(None) };
    let h = halt_der(d);
    Ok(Some(
        bars.iter()
            .filter(|b| b.volume > 0 && b.high > 0 && b.low > 0 && b.high >= b.low)
            .filter(|b| !h.is_some_and(|(a, z)| b.ts_micros >= a && b.ts_micros < z))
            .map(|b| Sec { ts: b.ts_micros, high: b.high, low: b.low, close: b.close, vol: u64::try_from(b.volume).unwrap_or(0) })
            .collect(),
    ))
}

/// The option rows of a contract folder on day `d` in `[from_sod, to_sod)` only (the busiest rule reads the opening seconds).
///
/// # Errors
/// A refused month or a store read error.
pub fn option_window(root: &Path, index: &str, folder: &str, d: Day, from_sod: i64, to_sod: i64) -> Result<Option<Vec<Sec>>, String> {
    let Some(bars) = storeio::day_bars(root, Vendor::Gdfl, "FNO", index, Some(folder), Timeframe::SECOND_1, d, from_sod, to_sod)? else { return Ok(None) };
    let h = halt_der(d);
    Ok(Some(
        bars.iter()
            .filter(|b| b.volume > 0 && b.high > 0 && b.low > 0 && b.high >= b.low)
            .filter(|b| !h.is_some_and(|(a, z)| b.ts_micros >= a && b.ts_micros < z))
            .map(|b| Sec { ts: b.ts_micros, high: b.high, low: b.low, close: b.close, vol: u64::try_from(b.volume).unwrap_or(0) })
            .collect(),
    ))
}

/// The index's GDFL seconds on day `d` in `[09:15:00, 15:30:00)` (+90 min on 2021-02-24), the index halt cut (the spot).
///
/// # Errors
/// A refused month or a store read error.
pub fn spot_day(root: &Path, index: &str, d: Day) -> Result<Vec<Sec>, String> {
    let to = 15 * 3600 + 30 * 60 + if day::fmt(d) == "2021-02-24" { 90 * 60 } else { 0 };
    let bars = storeio::day_bars(root, Vendor::Gdfl, "INDEX", index, None, Timeframe::SECOND_1, d, 9 * 3600 + 15 * 60, to)?.unwrap_or_default();
    let h = halt_idx(d);
    Ok(bars
        .iter()
        .filter(|b| !h.is_some_and(|(a, z)| b.ts_micros >= a && b.ts_micros < z))
        .map(|b| Sec { ts: b.ts_micros, high: b.high, low: b.low, close: b.close, vol: u64::try_from(b.volume.max(0)).unwrap_or(0) })
        .collect())
}

/// The last row strictly before `t`.
#[must_use]
pub fn last_before(rows: &[Sec], t: i64) -> Option<&Sec> {
    let i = rows.partition_point(|r| r.ts < t);
    i.checked_sub(1).and_then(|j| rows.get(j))
}
