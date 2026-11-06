//! F07 `candles::fold_timeframe`, F08 `candles::close_instant`, F09
//! `candles::auction_eligible` and F10 the hole rule (PREVIEW-SPEC section 3.4).
//!
//! k-minute buckets on the 09:15-anchored grid, clipped to the stock-day's continuous
//! close. A padded minute (volume 0) is a stored minute with no trade: it is never
//! missing and never folded (`PADDED_FOLD = Absent`), so a bucket of only padded
//! minutes has no bar and is not a hole.

use crate::market::{Bar, Minute};
use crate::time::{CLOSE_AUCTION, CLOSE_FULL, Day, LAST_AUCTION_MINUTE, OPEN};

/// The six timeframes, in minutes, each its own run.
pub const TIMEFRAMES: [u32; 6] = [1, 3, 5, 15, 30, 60];

/// 2026-08-03: the first day closing-auction eligibility is decided per stock-day.
#[must_use]
pub fn auction_switch() -> Option<Day> {
    Day::from_ymd(2026, 8, 3)
}

/// F09: `None` before 2026-08-03 or on a day without stored minutes; else whether the
/// stock-day's last stored minute (padded or not) starts at or before 15:14.
#[must_use]
pub fn auction_eligible(day: Day, minutes: &[Minute]) -> Option<bool> {
    let switch = auction_switch()?;
    if day < switch {
        return None;
    }
    minutes.iter().map(|m| m.start).max().map(|last| last <= LAST_AUCTION_MINUTE)
}

/// The continuous close of a stock-day: 15:15 when auction-eligible, else 15:30.
#[must_use]
pub fn session_close(day: Day, minutes: &[Minute]) -> u32 {
    if auction_eligible(day, minutes) == Some(true) { CLOSE_AUCTION } else { CLOSE_FULL }
}

/// F08: the close instant of the bucket starting at `start` for k-minute candles: the
/// start plus `60 k` seconds, clipped to the continuous close. `None` for a bucket that
/// starts at or after the close, or before 09:15, or for `k = 0`.
#[must_use]
pub fn close_instant(start: u32, k: u32, close: u32) -> Option<u32> {
    if k == 0 || start < OPEN || start >= close {
        return None;
    }
    Some(start.saturating_add(60 * k).min(close))
}

/// One scheduled bucket that matters: it holds a bar, or it is a hole, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bucket {
    /// The stock-day.
    pub day: Day,
    /// The bucket's start, seconds of the IST day.
    pub start: u32,
    /// The candle close instant T.
    pub t: u32,
    /// The folded bar, `None` when no traded minute fell in the bucket.
    pub bar: Option<Bar>,
    /// A scheduled minute was missing, or the day was withheld, or its month refused.
    pub hole: bool,
}

/// Why a month's minutes cannot be folded at all (`FoldError`): every scheduled bucket
/// of every full session of that month becomes a hole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldError {
    /// A minute not stamped on a whole minute (day and stamp).
    OffGrid(Day, u32),
    /// A minute stamped twice, or out of order (day and stamp).
    Duplicate(Day, u32),
}

/// The fold's own check of one day's minutes: whole minutes, strictly rising.
///
/// # Errors
/// The first off-grid or repeated stamp.
pub fn check_minutes(day: Day, minutes: &[Minute]) -> Result<(), FoldError> {
    let mut previous: Option<u32> = None;
    for m in minutes {
        if m.start % 60 != 0 {
            return Err(FoldError::OffGrid(day, m.start));
        }
        if previous.is_some_and(|p| m.start <= p) {
            return Err(FoldError::Duplicate(day, m.start));
        }
        previous = Some(m.start);
    }
    Ok(())
}

/// Folds traded minutes into one bar: first open, highest high, lowest low, last close,
/// summed volume. `None` when no minute traded.
#[must_use]
pub fn fold_bar(minutes: &[Minute]) -> Option<Bar> {
    let mut traded = minutes.iter().filter(|m| !m.padded());
    let first = traded.next()?;
    let mut bar = first.bar;
    for m in traded {
        bar.high = bar.high.max(m.bar.high);
        bar.low = bar.low.min(m.bar.low);
        bar.close = m.bar.close;
        bar.volume = bar.volume.saturating_add(m.bar.volume);
    }
    Some(bar)
}

/// Counters the fold keeps per stock and timeframe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FoldCounts {
    /// Stored minutes outside 09:15 to the continuous close (never folded).
    pub outside_session: u64,
    /// Hole buckets.
    pub holes: u64,
}

/// F07 and F10 for one full-session stock-day: every bucket of the k-minute schedule
/// that holds a bar or is a hole. `blocked` marks a withheld day or a refused month:
/// then every scheduled bucket is a hole with no bar.
#[must_use]
pub fn fold_day(day: Day, minutes: &[Minute], k: u32, blocked: bool, counts: &mut FoldCounts) -> Vec<Bucket> {
    let close = session_close(day, minutes);
    let mut out = Vec::new();
    if k == 0 {
        return out;
    }
    let width = 60 * k;
    let in_session = |m: &&Minute| m.start >= OPEN && m.start < close;
    let outside = minutes.iter().filter(|m| !in_session(m)).count();
    counts.outside_session = counts.outside_session.saturating_add(u64::try_from(outside).unwrap_or(u64::MAX));
    let mut start = OPEN;
    while let Some(t) = close_instant(start, k, close) {
        let end = start + width;
        let inside: Vec<Minute> =
            minutes.iter().filter(in_session).filter(|m| m.start >= start && m.start < end).copied().collect();
        let scheduled = (t - start) / 60;
        let missing = u32::try_from(inside.len()).unwrap_or(u32::MAX) < scheduled;
        let hole = blocked || missing;
        let bar = if blocked { None } else { fold_bar(&inside) };
        if hole {
            counts.holes += 1;
        }
        if hole || bar.is_some() {
            out.push(Bucket { day, start, t, bar, hole });
        }
        start = end;
    }
    out
}

#[cfg(test)]
#[path = "candles_tests.rs"]
mod tests;
