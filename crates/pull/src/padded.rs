//! Vendor-padded minutes: defined, counted, and kept out of a derived
//! bucket's open (INT-15, R4-27).
//!
//! # What a padded minute is
//!
//! Zerodha fills a quiet minute of a cash share with a candle of its own: no
//! volume, and open, high, low and close all the close of the minute before.
//! Nobody traded in it. Measured offline on 13 Oct 2026 over every Zerodha
//! cash minute against GDFL's ticks: 2,158,944 flat zero-volume minutes of
//! 140,870,615 (1.53%) had no GDFL trade at all
//! (`state/data-census-20261013/crosscheck_all.json`, field `only_z_flat`).
//! Stored as they arrive, they are bars like any other, and a derived bucket
//! that begins with one opens at a price nobody paid in that bucket.
//!
//! **The definition** (the docs/05 entry of R4-27): a VENDOR-PADDED MINUTE is
//! a cash-equity one-minute bar with volume zero whose open, high, low and
//! close all equal the close of the bar before it **in the same session
//! window** of the same day. Therefore:
//!
//! * the first bar of a day (the 09:15 open) is never padded, whatever it
//!   looks like: it has no bar before it in its session;
//! * the first bar after a halt is never padded: a halt ends one window, and
//!   the bar before it is in the window that closed (`calendar::Session`
//!   holds up to [`crate::calendar::MAX_WINDOWS`] windows a day);
//! * a bar outside every window of its day is never padded;
//! * an index minute is never padded: an index does not trade, so its flat
//!   minute is the level not moving, not a vendor's filler. The rule is
//!   applied by SEGMENT (cash), not by vendor name; GDFL never derives
//!   (`ingest::derives_for`), so in the derive path today it reaches Zerodha's
//!   minutes only.
//!
//! # What it is not
//!
//! A padded minute is COUNTED and is NOT A TRADE. It is kept in the store as
//! the vendor sent it (nothing here rewrites a bar), it is counted per day in
//! the month summary (`store::month_summary`, version 2), and a derived
//! bucket reads it as a minute in which nothing traded: the bucket opens on
//! its first unpadded minute, its high and low come from its unpadded minutes,
//! and its volume is unchanged (a padded minute adds zero, so volume
//! conditions see zero for it). What a bucket of ONLY padded minutes becomes
//! is the fold rule the decision chose, measured both ways
//! ([`crate::fold::PaddedFold`]).
//!
//! # Cost
//!
//! [`is_padded`] is one comparison of four prices and a volume: O(1) a bar.
//! [`Walk::step`] adds one window lookup over at most
//! [`crate::calendar::MAX_WINDOWS`] windows. [`per_day`] is one pass over a
//! month's minutes with one calendar lookup a day.

use store::format::Bar;
use store::month_summary::{MonthSummary, NotCounted};

use crate::calendar::DayKind;

/// Whether `bar` is a vendor-padded minute after `prev`, the bar before it.
///
/// `same_session` is the caller's answer to "is `prev` in the same session
/// window of the same day as `bar`": [`Walk`] computes it from the calendar.
/// A bar without a bar before it is never padded, so there is no `Option`
/// here: the caller simply does not ask.
///
/// # Examples
///
/// ```
/// # use pull::padded::is_padded;
/// # use store::format::{Bar, OI_NULL};
/// let traded = Bar { ts_micros: 0, open: 101, high: 104, low: 99, close: 103,
///                    volume: 7, open_interest: OI_NULL };
/// let flat = Bar { ts_micros: 60_000_000, open: 103, high: 103, low: 103,
///                  close: 103, volume: 0, open_interest: OI_NULL };
/// assert!(is_padded(&traded, &flat, true));
/// // Across a session break it is the first bar of its window: never padded.
/// assert!(!is_padded(&traded, &flat, false));
/// // A flat minute WITH volume traded, at one price.
/// assert!(!is_padded(&traded, &Bar { volume: 1, ..flat }, true));
/// ```
#[must_use]
pub const fn is_padded(prev: &Bar, bar: &Bar, same_session: bool) -> bool {
    same_session
        && bar.volume == 0
        && bar.open == prev.close
        && bar.high == prev.close
        && bar.low == prev.close
        && bar.close == prev.close
}

/// The index of the window of `session` holding minute-of-day `minute`, or
/// `None` for a minute outside every window or a day that is not an open
/// session. At most [`crate::calendar::MAX_WINDOWS`] comparisons.
#[must_use]
pub fn window_of(session: DayKind, minute: u16) -> Option<usize> {
    let DayKind::Open(s) = session else {
        return None;
    };
    s.windows
        .iter()
        .take(usize::from(s.count))
        .position(|window| window.holds(minute))
}

/// The session windows of `session` as `(first minute, last minute)` of the
/// day, inclusive, the form `store::month_summary::MonthSummary::padded_on`
/// takes. Empty for a day that is not an open session.
#[must_use]
pub fn windows_of(session: DayKind) -> Vec<(u16, u16)> {
    let DayKind::Open(s) = session else {
        return Vec::new();
    };
    s.windows
        .iter()
        .take(usize::from(s.count))
        .map(|window| (window.from, window.to))
        .collect()
}

/// The bar before, as [`Walk`] remembers it.
#[derive(Debug, Clone, Copy)]
struct Seen {
    bar: Bar,
    day: i64,
    window: Option<usize>,
}

/// The one per-bar walk the derived fold and the full read both take, so the
/// two cannot answer differently: each bar is judged against the bar walked
/// before it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Walk {
    previous: Option<Seen>,
}

impl Walk {
    /// A walk that has seen no bar.
    #[must_use]
    pub const fn new() -> Self {
        Self { previous: None }
    }

    /// Judges `bar`, on IST epoch day `day` and in window `window` of that
    /// day's session ([`window_of`]), against the bar stepped before it, and
    /// remembers it. O(1).
    pub fn step(&mut self, bar: &Bar, day: i64, window: Option<usize>) -> bool {
        let padded = self.previous.is_some_and(|before| {
            let same_session = window.is_some() && before.day == day && before.window == window;
            is_padded(&before.bar, bar, same_session)
        });
        self.previous = Some(Seen {
            bar: *bar,
            day,
            window,
        });
        padded
    }
}

/// One IST day's padded minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaddedDay {
    /// The IST day, as days since the epoch.
    pub day: i64,
    /// Bars stamped on the day.
    pub bars: u32,
    /// Of them, vendor-padded minutes.
    pub padded: u32,
}

/// Why padded minutes were not counted for a series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Uncounted {
    /// The series is not a cash share's: the definition does not apply.
    NotCash(crate::vendor::Venue),
    /// The month summary cannot say, by name.
    Summary(NotCounted),
    /// A stamp whose IST day leaves the `i64` range.
    Overflow(i64),
}

impl core::fmt::Display for Uncounted {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotCash(venue) => write!(
                f,
                "not_cash: padded minutes are defined for a cash share's minutes only, \
                 and this series is {venue}"
            ),
            Self::Summary(why) => write!(f, "{why}"),
            Self::Overflow(ts) => write!(f, "overflow: stamp {ts} has no IST day"),
        }
    }
}

impl core::error::Error for Uncounted {}

/// The IST epoch day of a stamp and its minute of that day.
fn ist_minute(ts_micros: i64) -> Option<(i64, u16)> {
    const MINUTE: i64 = 60_000_000;
    let local = ts_micros.checked_add(crate::session::IST_OFFSET_SECS * 1_000_000)?;
    let minutes = local.div_euclid(MINUTE);
    let day = minutes.div_euclid(1_440);
    let minute = u16::try_from(minutes.rem_euclid(1_440)).ok()?;
    Some((day, minute))
}

/// The FULL READ: every bar of a month of a cash share's minutes, judged one
/// by one, counted per IST day that holds a bar, in day order.
///
/// The session of each day is the one the derived fold uses
/// ([`crate::fold::session_of`]: the calendar decides which days trade, the
/// venue their hours, a special session its recorded windows), looked up once
/// a day.
///
/// # Errors
///
/// [`Uncounted::NotCash`] for any venue but [`crate::vendor::Venue::NseCash`];
/// [`Uncounted::Overflow`] for a stamp with no IST day.
pub fn per_day(
    minutes: &[Bar],
    venue: crate::vendor::Venue,
    cash_schedule: Option<&crate::cash_auction::Schedule>,
    runtime: crate::calendar::Runtime<'_>,
) -> Result<Vec<PaddedDay>, Uncounted> {
    if venue != crate::vendor::Venue::NseCash {
        return Err(Uncounted::NotCash(venue));
    }
    let mut out: Vec<PaddedDay> = Vec::new();
    let mut walk = Walk::new();
    let mut session = (i64::MIN, DayKind::Unmeasured);
    for bar in minutes {
        let (day, minute) = ist_minute(bar.ts_micros).ok_or(Uncounted::Overflow(bar.ts_micros))?;
        if session.0 != day {
            session = (
                day,
                crate::fold::session_of(day, venue, cash_schedule, runtime),
            );
        }
        let padded = walk.step(bar, day, window_of(session.1, minute));
        match out.last_mut() {
            Some(last) if last.day == day => {
                last.bars = last.bars.saturating_add(1);
                last.padded = last.padded.saturating_add(u32::from(padded));
            }
            _ => out.push(PaddedDay {
                day,
                bars: 1,
                padded: u32::from(padded),
            }),
        }
    }
    Ok(out)
}

/// The same counts as [`per_day`], read off a month summary instead of the
/// bars: one [`MonthSummary::padded_on`] a day with bars, with that day's
/// windows from the same session lookup. No bar is read.
///
/// # Errors
///
/// [`Uncounted::NotCash`] as [`per_day`]; [`Uncounted::Summary`] when the
/// summary cannot count (a version 1 summary, a month that is not one
/// minute, a month with invalid stamps), by name and never as zero.
pub fn per_day_from_summary(
    summary: &MonthSummary,
    venue: crate::vendor::Venue,
    cash_schedule: Option<&crate::cash_auction::Schedule>,
    runtime: crate::calendar::Runtime<'_>,
) -> Result<Vec<PaddedDay>, Uncounted> {
    if venue != crate::vendor::Venue::NseCash {
        return Err(Uncounted::NotCash(venue));
    }
    let mut out = Vec::new();
    for day in summary.first_day()..=summary.last_day() {
        let bars = summary.bars_on(day);
        if bars == 0 {
            continue;
        }
        let session = crate::fold::session_of(day, venue, cash_schedule, runtime);
        let padded = summary
            .padded_on(day, &windows_of(session))
            .map_err(Uncounted::Summary)?;
        out.push(PaddedDay { day, bars, padded });
    }
    Ok(out)
}

#[cfg(test)]
#[path = "padded_tests.rs"]
mod tests;
