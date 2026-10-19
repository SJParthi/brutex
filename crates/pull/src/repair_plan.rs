//! What a versioned repair of one series' months would do — read-only, and the
//! one answer the `/pull/repair.json` route and `cli zerodha-repair` both give.
//! D-3350, WP-13c; findings DB-N20, DB-N22. Short sessions: R3-18 (DB-A12,
//! CMP-19).
//!
//! # What it answers
//!
//! For every month of the asked window: the generation every reader opens
//! (`0` is the original), the bars it holds, the sessions the calendar of
//! record owes inside the window ([`crate::calendar::exchange_kind_of`], the
//! authority `/calendar.json` reads), how many hold a bar, and each session
//! that does not, classed by what can fill it:
//!
//! - [`Missing::Front`] — before the month's first held bar;
//! - [`Missing::Interior`] — between its held bars;
//! - [`Missing::Tail`] — after its last held bar;
//! - [`Missing::Absent`] — the month holds nothing at all;
//! - [`Missing::BeforeFirst`] — before the first bar of the SERIES' first
//!   held month: a listing day, the start of the feed's history, or a hole an
//!   earlier refused answer left. The bars cannot tell which, so it is counted
//!   apart and never planned as a repair; a repair over that month lands
//!   whatever the feed serves.
//!
//! The first two are what the append-only store refuses for ever and only a
//! versioned repair fills (`POST /pull/repair`, `cli zerodha-repair`); tail and
//! absent an ordinary pull fills. A day the calendar cannot attest is named
//! apart, never counted as owed.
//!
//! # Short sessions (R3-18)
//!
//! **A session that holds a bar is not therefore whole.** Until R3-18 a
//! session counted as held when it held ONE bar, so a day missing 15:15–15:28
//! was invisible here: of the 14 months behind the 213 minutes Zerodha NIFTY
//! 1min lacks (DB-A12), the plan named 2, and the documented store run would
//! have left 12 of them short for ever. At the minute rung every held session
//! is now also counted against the minutes it owes ([`owed`]), and one that
//! holds fewer is a [`Short`], classed by the same five words by where its
//! missing minutes lie: between held bars is `interior`; before the month's
//! first held bar `front` (or `before_first` in the series' first month);
//! after its last `tail`. The most demanding class wins
//! (`interior` > `front` > `tail` > `before_first`).
//!
//! What a session owes is decided by ONE function for the plan and for the
//! stored-month gap ledger (`super::gap_backfill`): the calendar of record's
//! session, and on a regular day the venue's dated hours — for the cash venue
//! from 2026-08-03 the minutes owed under EVERY closing-auction eligibility,
//! so an eligible share's 15:15 close is never read as 15 missing minutes and
//! no share's eligibility is guessed (D-3320); a recorded halt's minutes
//! (`calendar::HALTS`) are owed by no one, so every share's 59 silent minutes
//! on 2020-03-13 are the halt, not a hole. A session whose length no record
//! states is named in [`Month::length_unjudged`], never counted short.
//!
//! **A share's short session may be minutes nobody traded.** An index is
//! computed every minute; a share trades only when somebody trades it, and
//! the fold already counts its no-trade buckets `partial` (D-3321, DB-G36).
//! The store alone cannot tell such a minute from a vendor hole, so a share's
//! short session is still listed and planned — a repair lands only what the
//! feed serves — and labelled ([`absence_may_be_no_trade`]) wherever it is
//! shown.
//! A rung other than `1min` is judged by holding a bar, as before: a derived
//! rung is refolded from the minutes a repair restores (D-3321), and its
//! short buckets are the derive ledger's `partial` (D-3335).
//!
//! # Cost
//!
//! Per month: one generation probe and one open; per day of the window: two
//! time-indexed lookups, and at the minute rung two more per trading window
//! (at most [`crate::calendar::MAX_WINDOWS`]) and at most two record reads;
//! per series: one listing of its rung directory, to find its first held
//! month. O(days asked + months held), never the month's bars. A lookup or a
//! read that fails refuses the month by name rather than reading the session
//! as empty. Nothing is written, created or locked beyond a reader's shared
//! lock.

use std::path::Path;

use brutex_core::instrument::{Exchange, Segment};
use store::file::{BarFile, StoreError};
use store::path::{PathParts, StorePath, Timeframe, YearMonth};

use crate::calendar::{CasEligibility, DayKind, MAX_WINDOWS, Session, exchange_kind_of};
use crate::session::{Day, IST_OFFSET_SECS, Window};
use crate::vendor::{SessionKind, Venue};

/// What can fill a session a month lacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Missing {
    /// Before the month's first held bar: refused by the append-only store,
    /// filled only by a versioned repair.
    Front,
    /// Between the month's held bars: likewise.
    Interior,
    /// After the month's last held bar: an ordinary pull appends it.
    Tail,
    /// The month holds nothing: an ordinary pull creates it.
    Absent,
    /// Before the first bar of the series' first held month: a listing, the
    /// start of the feed's history, or a hole an earlier refused answer left.
    /// Counted apart and never planned as a repair.
    BeforeFirst,
}

impl Missing {
    /// The code a page and a terminal print, and a ledger line's `fill=`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Front => "front",
            Self::Interior => "interior",
            Self::Tail => "tail",
            Self::Absent => "absent",
            Self::BeforeFirst => "before_first",
        }
    }

    /// Whether only a versioned repair can fill it.
    #[must_use]
    pub const fn needs_repair(self) -> bool {
        matches!(self, Self::Front | Self::Interior)
    }

    /// How demanding a fill is, for a short session whose missing minutes
    /// fall in more than one place: the most demanding wins.
    const fn rank(self) -> u8 {
        match self {
            Self::Interior => 4,
            Self::Front => 3,
            Self::Tail => 2,
            Self::BeforeFirst => 1,
            Self::Absent => 0,
        }
    }

    /// The more demanding of two fills.
    const fn worst(self, other: Self) -> Self {
        if other.rank() > self.rank() {
            other
        } else {
            self
        }
    }
}

/// A session that holds bars, but fewer of its owed minutes than the
/// calendar owes (R3-18). Only the minute rung is judged this way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Short {
    /// Which session.
    pub day: Day,
    /// Minutes the session owes ([`owed`]).
    pub owed: u32,
    /// Of those, the minutes holding a bar.
    pub held: u32,
    /// What can fill the rest: the most demanding class of where they lie.
    pub fill: Missing,
}

impl Short {
    /// Minutes owed and not held.
    #[must_use]
    pub const fn missing(self) -> u32 {
        self.owed.saturating_sub(self.held)
    }
}

/// The trading windows one day owes at the minute rung, as `[from, to)`
/// minutes past IST midnight, and how many of them are real.
pub type Windows = ([(u32, u32); MAX_WINDOWS], usize);

/// What one day owes a series at the minute rung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owes {
    /// The exchange did not trade.
    Closed,
    /// No calendar of record attests the day.
    Unattested,
    /// A session traded and no record states its minutes: judged only by
    /// holding a bar, never counted short.
    Unsized,
    /// These windows.
    Windows(Windows),
}

/// What `number` (days since the epoch) owes a series of `segment` at the
/// minute rung: the ONE answer the repair plan and the stored-month gap ledger
/// share (R3-18).
///
/// The calendar of record decides the day ([`exchange_kind_of`]). A special
/// session owes its recorded windows (D-3319). A regular session owes the
/// venue's dated hours, and where those depend on the instrument — the cash
/// venue's closing-auction eligibility from 2026-08-03 (D-3320) — the minutes
/// owed under EVERY eligibility: the latest open and the earliest close, so no
/// share's flag is guessed and no eligible share's 15:15 close reads as a
/// hole. A recorded halt's minutes ([`crate::calendar::halt_on`]) are owed
/// by no one. A segment with no venue table, hours no row verifies, or a halt
/// that would leave more than [`MAX_WINDOWS`] windows, is [`Owes::Unsized`].
///
/// O(1): one calendar answer, at most two dated-table walks of fixed length
/// and one walk of the three-entry halt table. Proof:
/// `crates/pull/tests/zerodha_gaps.rs`
/// (`what_a_day_owes_costs_the_same_on_every_date_of_the_window`).
#[must_use]
pub fn owed(segment: &str, number: u32) -> Owes {
    let session = match exchange_kind_of(i64::from(number)) {
        DayKind::Closed => return Owes::Closed,
        DayKind::Unmeasured => return Owes::Unattested,
        DayKind::OpenLengthUnmeasured => return Owes::Unsized,
        DayKind::Open(session) => session,
    };
    let windows = if session == Session::full() {
        match regular(segment, number) {
            Some(window) => ([window, (0, 0)], 1),
            None => return Owes::Unsized,
        }
    } else {
        let mut windows = [(0, 0); MAX_WINDOWS];
        let count = usize::from(session.count).min(MAX_WINDOWS);
        for (slot, window) in windows.iter_mut().zip(session.windows).take(count) {
            *slot = (u32::from(window.from), u32::from(window.to) + 1);
        }
        (windows, count)
    };
    // A RECORDED HALT OWES NOTHING (calendar::HALTS): its whole minutes,
    // reopening auction included, had no continuous trading on any segment,
    // so a share's 59 minutes without a candle on 2020-03-13 are the halt,
    // not a hole. The calendar's own window, never a measured guess.
    match crate::calendar::halt_on(i64::from(number)) {
        None => Owes::Windows(windows),
        Some(halt) => without(
            windows,
            (u32::from(halt.halted.from), u32::from(halt.halted.to) + 1),
        )
        .map_or(Owes::Unsized, Owes::Windows),
    }
}

/// A regular session's owed `[from, to)` for `segment`: the venue's dated
/// hours under every closing-auction eligibility (the latest open and the
/// earliest close), or `None` where no row verifies them.
fn regular(segment: &str, number: u32) -> Option<(u32, u32)> {
    let day = Day::from_days(number).ok()?;
    let venue = Venue::for_segment(Exchange::parse("NSE").ok()?, Segment::parse(segment).ok()?)?;
    let mut span: Option<(u32, u32)> = None;
    for cas in [CasEligibility::Eligible, CasEligibility::NotEligible] {
        let hours = venue.share_hours_on(day, cas).ok()?;
        if hours.kind() != SessionKind::Continuous {
            return None;
        }
        let (open, close) = (hours.open_minute(), hours.close_minute());
        span = Some(span.map_or((open, close), |(from, to)| (from.max(open), to.min(close))));
    }
    span.filter(|(from, to)| from < to)
}

/// `windows` less the minutes `[from, to)`: `None` when what is left needs
/// more than [`MAX_WINDOWS`] windows.
fn without((windows, count): Windows, (from, to): (u32, u32)) -> Option<Windows> {
    let mut out = [(0, 0); MAX_WINDOWS];
    let mut used = 0_usize;
    for &(open, close) in windows.iter().take(count) {
        for piece in [(open, close.min(from)), (open.max(to), close)] {
            if piece.0 < piece.1 {
                *out.get_mut(used)? = piece;
                used += 1;
            }
        }
    }
    Some((out, used))
}

/// Whether a minute without a bar may be a minute nobody traded, for
/// `segment`: a share trades only when somebody trades it, an index is
/// computed every minute. The fold's rule (`fold.rs`, `no_trade_is_absence`,
/// D-3321, DB-G36), asked here so the plan and the ledger can say so: the
/// store alone cannot tell a no-trade minute from a vendor hole, so such a
/// short session is still listed, labelled.
#[must_use]
pub fn absence_may_be_no_trade(segment: &str) -> bool {
    matches!(
        (Exchange::parse("NSE"), Segment::parse(segment)),
        (Ok(exchange), Ok(segment)) if Venue::for_segment(exchange, segment) == Some(Venue::NseCash)
    )
}

/// One month of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Month {
    /// Which month.
    pub month: YearMonth,
    /// The generation served: `0` is the original month.
    pub generation: u32,
    /// Bars the served generation holds.
    pub bars: u64,
    /// Its first and last held stamps, UTC micros, when it holds any.
    pub span: Option<(i64, i64)>,
    /// Sessions of record inside the asked window.
    pub sessions: u32,
    /// Of those, the sessions holding at least one bar.
    pub held: u32,
    /// Each session holding none, and what can fill it.
    pub missing: Vec<(Day, Missing)>,
    /// Each session holding a bar but fewer of its owed minutes than the
    /// calendar owes, at the minute rung (R3-18).
    pub short: Vec<Short>,
    /// Sessions holding a bar whose owed minutes no record states, at the
    /// minute rung: held, and never judged whole or short.
    pub length_unjudged: Vec<Day>,
    /// Days inside the window the calendar of record cannot attest.
    pub unattested: Vec<Day>,
    /// Why the month could not be read, when it could not: nothing about it
    /// is then claimed.
    pub refused: Option<String>,
}

impl Month {
    /// Whether only a versioned repair can make this month whole.
    #[must_use]
    pub fn needs_repair(&self) -> bool {
        self.missing.iter().any(|(_, why)| why.needs_repair())
            || self.short.iter().any(|short| short.fill.needs_repair())
    }
}

/// The store's symbol id for `symbol`: the cross-check every bar header
/// carries, the low 32 bits of its FNV-1a (`ingest::identify`).
#[must_use]
pub fn symbol_id(symbol: &str) -> u32 {
    u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX)).unwrap_or(u32::MAX)
}

/// The plan for `parts`' series over `window`, one entry per month the window
/// touches, oldest first. `parts.month` is ignored; every other part names the
/// series.
#[must_use]
pub fn plan(root: &Path, parts: PathParts<'_>, window: Window) -> Vec<Month> {
    let symbol_id = symbol_id(parts.symbol);
    let first_held = first_held_month(root, parts);
    let mut months = Vec::new();
    let mut at = window.from();
    while let Ok(month) = at.year_month() {
        let end = at.end_of_month().min(window.to());
        months.push(plan_month(
            root,
            PathParts { month, ..parts },
            symbol_id,
            (at, end, first_held == Some(month)),
        ));
        if end >= window.to() {
            break;
        }
        let Ok(next) = Day::from_days(end.days_from_epoch().saturating_add(1)) else {
            break;
        };
        at = next;
    }
    months
}

/// The earliest month the series' rung directory holds a bar file for: one
/// listing, `None` when the directory is not there or names no month.
pub(crate) fn first_held_month(root: &Path, parts: PathParts<'_>) -> Option<YearMonth> {
    held_months(root, parts).first().copied()
}

/// Every month the series' rung directory holds a bar file for, ascending:
/// one listing, empty when the directory is not there or names no month.
/// `parts.month` only completes the path; every held month is answered.
#[must_use]
pub fn held_months(root: &Path, parts: PathParts<'_>) -> Vec<YearMonth> {
    let Some(dir) = StorePath::new(parts)
        .ok()
        .map(|path| path.to_path_buf(root))
        .and_then(|file| file.parent().map(Path::to_path_buf))
    else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut months: Vec<YearMonth> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let stem = name.to_str()?.strip_suffix(".bin")?;
            let (year, month) = stem.split_once('-')?;
            let digits = |text: &str, width: usize| {
                (text.len() == width && text.bytes().all(|b| b.is_ascii_digit()))
                    .then(|| text.parse::<u16>().ok())
                    .flatten()
            };
            YearMonth::new(digits(year, 4)?, u8::try_from(digits(month, 2)?).ok()?).ok()
        })
        .collect();
    months.sort_unstable();
    months.dedup();
    months
}

/// One month of the plan: `parts.month` over `from..=to`, a window inside it;
/// `series_first` when it is the series' first held month.
pub(crate) fn plan_month(
    root: &Path,
    parts: PathParts<'_>,
    symbol_id: u32,
    (from, to, series_first): (Day, Day, bool),
) -> Month {
    let mut out = Month {
        month: parts.month,
        generation: 0,
        bars: 0,
        span: None,
        sessions: 0,
        held: 0,
        missing: Vec::new(),
        short: Vec::new(),
        length_unjudged: Vec::new(),
        unattested: Vec::new(),
        refused: None,
    };
    let path = match StorePath::new(parts) {
        Ok(path) => path,
        Err(why) => {
            out.refused = Some(why.to_string());
            return out;
        }
    };
    let file = match store::generation::served(root, path)
        .and_then(|served| BarFile::open_existing(root, path, symbol_id).map(|file| (served, file)))
    {
        Ok((served, file)) => {
            out.generation = served.ordinal();
            out.bars = file.header().n_valid;
            if out.bars > 0 {
                out.span = Some((file.header().first_ts_micros, file.header().last_ts_micros));
            }
            Some(file)
        }
        Err(StoreError::Missing { .. }) => None,
        Err(why) => {
            out.refused = Some(why.to_string());
            return out;
        }
    };
    let minutes = parts.timeframe == Timeframe::MINUTE_1;
    for number in from.days_from_epoch()..=to.days_from_epoch() {
        let Ok(day) = Day::from_days(number) else {
            continue;
        };
        match exchange_kind_of(i64::from(number)) {
            DayKind::Closed => continue,
            DayKind::Unmeasured => {
                out.unattested.push(day);
                continue;
            }
            DayKind::Open(_) | DayKind::OpenLengthUnmeasured => {}
        }
        out.sessions += 1;
        let base = day_base(number);
        let held = match &file {
            None => Ok(false),
            Some(file) => count(file, out.bars, base, base + DAY_MICROS).map(|(n, _)| n > 0),
        };
        match held {
            Ok(true) => out.held += 1,
            Ok(false) => {
                let why = match out.span {
                    None => Missing::Absent,
                    Some(span) => class(span, (base, base + DAY_MICROS - 1), series_first),
                };
                out.missing.push((day, why));
                continue;
            }
            Err(why) => {
                out.refused = Some(format!("{day}: {why}"));
                return out;
            }
        }
        let (Some(file), Some(span), true) = (&file, out.span, minutes) else {
            continue;
        };
        match owed(parts.segment, number) {
            Owes::Windows(windows) => match judge(file, out.bars, number, windows) {
                Ok(Some(found)) => {
                    out.short.push(Short {
                        day,
                        owed: found.owed,
                        held: found.held,
                        fill: found.fill(span, series_first),
                    });
                }
                Ok(None) => {}
                Err(why) => {
                    out.refused = Some(format!("{day}: {why}"));
                    return out;
                }
            },
            Owes::Unsized | Owes::Unattested | Owes::Closed => out.length_unjudged.push(day),
        }
    }
    out
}

/// A day in UTC micros.
const DAY_MICROS: i64 = 86_400 * 1_000_000;

/// A minute in UTC micros.
const MINUTE_MICROS: i64 = 60 * 1_000_000;

/// IST midnight of `number` (days since the epoch), as UTC micros.
pub(crate) fn day_base(number: u32) -> i64 {
    (i64::from(number) * 86_400 - IST_OFFSET_SECS) * 1_000_000
}

/// The records of `file` stamped in `[from, to)`: how many, and the index of
/// the first. Two time-indexed lookups, no record read.
pub(crate) fn count(file: &BarFile, bars: u64, from: i64, to: i64) -> Result<(u64, u64), String> {
    let at = |ts: i64| {
        file.first_at_or_after(ts)
            .map(|index| index.min(bars))
            .map_err(|why| why.to_string())
    };
    let (first, end) = (at(from)?, at(to)?);
    Ok((end.saturating_sub(first), first))
}

/// What can fill an owed span `[first, last]` (UTC micros, both held minutes
/// of the day) that a month spanning `span` lacks entirely.
const fn class((first, last): (i64, i64), (from, to): (i64, i64), series_first: bool) -> Missing {
    if to < first && series_first {
        Missing::BeforeFirst
    } else if to < first {
        Missing::Front
    } else if from > last {
        Missing::Tail
    } else {
        Missing::Interior
    }
}

/// One short session as [`judge`] found it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Found {
    /// Minutes owed.
    pub(crate) owed: u32,
    /// Of those, the minutes holding a bar.
    pub(crate) held: u32,
    /// The first and the last owed minute, UTC micros.
    pub(crate) bounds: (i64, i64),
    /// The first and the last held owed minute, UTC micros, when any is.
    pub(crate) held_span: Option<(i64, i64)>,
    /// Whether an owed minute between the first and the last held one is
    /// missing.
    pub(crate) gap_inside: bool,
}

impl Found {
    /// What can fill it, given the month's held span.
    pub(crate) fn fill(self, span: (i64, i64), series_first: bool) -> Missing {
        let Some((first, last)) = self.held_span else {
            return class(span, self.bounds, series_first);
        };
        let mut fill = Missing::Absent;
        if self.gap_inside {
            fill = fill.worst(Missing::Interior);
        }
        if first > self.bounds.0 {
            fill = fill.worst(class(
                span,
                (self.bounds.0, first - MINUTE_MICROS),
                series_first,
            ));
        }
        if last < self.bounds.1 {
            fill = fill.worst(class(
                span,
                (last + MINUTE_MICROS, self.bounds.1),
                series_first,
            ));
        }
        fill
    }
}

/// Whether the session of day `number` holds fewer of its owed `windows`'
/// minutes than it owes: `None` when it holds them all. At most two lookups
/// per window and two record reads.
pub(crate) fn judge(
    file: &BarFile,
    bars: u64,
    number: u32,
    (windows, count_of): Windows,
) -> Result<Option<Found>, String> {
    let base = day_base(number);
    let (mut owed, mut held) = (0_u32, 0_u32);
    let mut bounds: Option<(i64, i64)> = None;
    // Per window: (first index, records) of its held minutes.
    let mut runs: [(u64, u64); MAX_WINDOWS] = [(0, 0); MAX_WINDOWS];
    for (slot, &(from, to)) in runs.iter_mut().zip(windows.iter()).take(count_of) {
        if to <= from {
            continue;
        }
        let width = to - from;
        let (start, end) = (
            base + i64::from(from) * MINUTE_MICROS,
            base + i64::from(to) * MINUTE_MICROS,
        );
        bounds = Some(bounds.map_or((start, end - MINUTE_MICROS), |(first, _)| {
            (first, end - MINUTE_MICROS)
        }));
        let (records, first) = count(file, bars, start, end)?;
        // A minute holds at most one bar; a stamp off the minute grid cannot
        // make a window hold more minutes than it has.
        let records = records.min(u64::from(width));
        *slot = (first, records);
        owed = owed.saturating_add(width);
        held = held.saturating_add(u32::try_from(records).unwrap_or(width));
    }
    let Some(bounds) = bounds else {
        return Ok(None);
    };
    if held >= owed {
        return Ok(None);
    }
    let read = |index: u64| {
        file.read_record(index)
            .map(|bar| bar.ts_micros)
            .map_err(|why| why.to_string())
    };
    let first_run = runs.iter().take(count_of).find(|(_, n)| *n > 0);
    let last_run = runs.iter().take(count_of).rev().find(|(_, n)| *n > 0);
    let held_span = match (first_run, last_run) {
        (Some(&(a, _)), Some(&(b, n))) => Some((read(a)?, read(b + n - 1)?)),
        _ => None,
    };
    // Owed minutes from the first held one to the last: a held count below
    // that is a hole between held bars.
    let gap_inside = held_span.is_some_and(|(first, last)| {
        let between: i64 = windows
            .iter()
            .take(count_of)
            .map(|&(from, to)| {
                let start = (base + i64::from(from) * MINUTE_MICROS).max(first);
                let end = (base + i64::from(to) * MINUTE_MICROS).min(last + MINUTE_MICROS);
                ((end - start) / MINUTE_MICROS).max(0)
            })
            .sum();
        i64::from(held) < between
    });
    Ok(Some(Found {
        owed,
        held,
        bounds,
        held_span,
        gap_inside,
    }))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test-only assertions"
)]
mod tests {
    use super::{Missing, plan};
    use crate::session::{Day, IST_OFFSET_SECS, Window};
    use brutex_core::vendor::Vendor;
    use store::format::{Bar, OI_NULL};
    use store::path::{FileKind, PathParts, Timeframe, YearMonth};

    fn parts() -> PathParts<'static> {
        PathParts {
            vendor: Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "PLANIDX",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2020, 12).unwrap(),
            file: FileKind::Bars,
        }
    }

    fn bar(day: Day, minute: i64) -> Bar {
        let secs = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60;
        Bar {
            ts_micros: secs * 1_000_000,
            open: 500_000,
            high: 500_010,
            low: 499_990,
            close: 500_001,
            volume: 0,
            open_interest: OI_NULL,
        }
    }

    /// The DB-N20 month, on a fixture: November 2020 is held, and December
    /// 2020 holds the 16th on. The plan names December's eleven sessions
    /// before it as `front` — a repair's — and the window's later sessions it
    /// lacks as `tail`, an ordinary pull's.
    #[test]
    fn a_month_with_a_leading_hole_is_planned_as_a_repair() {
        let root = std::env::temp_dir().join(format!("brutex-PLAN-{}", std::process::id()));
        let _fresh = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let id = super::symbol_id("PLANIDX");
        let november = PathParts {
            month: YearMonth::new(2020, 11).unwrap(),
            ..parts()
        };
        let mut file = store::file::BarFile::open_or_create(
            &root,
            store::path::StorePath::new(november).unwrap(),
            id,
        )
        .unwrap();
        file.append(&[bar(Day::new(2020, 11, 27).unwrap(), 0)])
            .unwrap();
        drop(file);
        let path = store::path::StorePath::new(parts()).unwrap();
        let mut file = store::file::BarFile::open_or_create(&root, path, id).unwrap();
        let held: Vec<Bar> = [16u8, 17, 18, 21, 22, 23]
            .iter()
            .map(|&d| bar(Day::new(2020, 12, d).unwrap(), 0))
            .collect();
        file.append(&held).unwrap();
        drop(file);
        let window = Window::new(
            Day::new(2020, 12, 1).unwrap(),
            Day::new(2020, 12, 31).unwrap(),
        )
        .unwrap();
        let months = plan(&root, parts(), window);
        assert_eq!(months.len(), 1);
        let december = &months[0];
        assert_eq!((december.generation, december.bars), (0, 6));
        assert_eq!(december.sessions, 22, "December 2020's sessions of record");
        assert_eq!(december.held, 6);
        let front: Vec<String> = december
            .missing
            .iter()
            .filter(|(_, why)| *why == Missing::Front)
            .map(|(day, _)| day.to_string())
            .collect();
        assert_eq!(front.len(), 11);
        assert_eq!(front.first().map(String::as_str), Some("2020-12-01"));
        assert_eq!(front.last().map(String::as_str), Some("2020-12-15"));
        assert!(december.needs_repair());
        let tail = december
            .missing
            .iter()
            .filter(|(_, why)| *why == Missing::Tail)
            .count();
        assert_eq!(tail, 5, "24, 28, 29, 30 and 31 follow the last held bar");
        assert!(december.unattested.is_empty());

        // A month that holds nothing is an ordinary pull's, never a repair's.
        let january = Window::new(
            Day::new(2021, 1, 1).unwrap(),
            Day::new(2021, 1, 31).unwrap(),
        )
        .unwrap();
        let absent = &plan(&root, parts(), january)[0];
        assert!(!absent.missing.is_empty());
        assert!(
            absent
                .missing
                .iter()
                .all(|(_, why)| *why == Missing::Absent)
        );
        assert!(!absent.needs_repair());

        // THE SERIES' FIRST HELD MONTH: its sessions before November's first
        // bar may be a listing or the start of the feed's history, which the
        // bars cannot tell from a hole. Counted apart, never planned.
        let first = &plan(
            &root,
            parts(),
            Window::new(
                Day::new(2020, 11, 1).unwrap(),
                Day::new(2020, 11, 30).unwrap(),
            )
            .unwrap(),
        )[0];
        assert_eq!(first.held, 1);
        assert_eq!(
            first.missing.len(),
            19,
            "every session of November before the 27th"
        );
        assert!(
            first
                .missing
                .iter()
                .all(|(_, why)| *why == Missing::BeforeFirst)
        );
        assert!(!first.needs_repair());
        assert_eq!(Missing::BeforeFirst.code(), "before_first");
        let _cleanup = std::fs::remove_dir_all(&root);
    }

    /// **A RECORDED HALT IS OWED BY NO ONE (R3-18).** 2020-03-13's halt
    /// (`calendar::HALTS`, 09:22-10:20 whole minutes) splits the regular
    /// session in two for an index and a share alike, 316 minutes owed, the
    /// count every share's 2020-03-13 holds; the 2021-02-24 outage's windows
    /// are the session's own, 220 minutes. A halt that would leave three
    /// windows is refused as unsized, never truncated. A share's absent
    /// minute may be no trade; an index's may not; a segment with no venue
    /// is neither.
    #[test]
    fn a_recorded_halt_is_owed_by_no_one() {
        use super::{Owes, absence_may_be_no_trade, owed, without};
        let day = |y, m, d| Day::new(y, m, d).unwrap().days_from_epoch();
        let sum = |owes: Owes| match owes {
            Owes::Windows((w, c)) => w[..c].iter().map(|(a, b)| b - a).sum::<u32>(),
            other => panic!("{other:?}"),
        };
        for segment in ["INDEX", "CASH"] {
            assert_eq!(
                owed(segment, day(2020, 3, 13)),
                Owes::Windows(([(555, 562), (621, 930)], 2)),
                "{segment}"
            );
            assert_eq!(sum(owed(segment, day(2020, 3, 13))), 316);
            assert_eq!(sum(owed(segment, day(2021, 2, 24))), 220);
            assert_eq!(sum(owed(segment, day(2020, 3, 12))), 375);
        }
        assert_eq!(owed("INDEX", day(2020, 3, 14)), Owes::Closed);
        assert_eq!(owed("FUT", day(2020, 3, 12)), Owes::Unsized);
        assert_eq!(owed("INDEX", day(2030, 1, 1)), Owes::Unattested);
        assert_eq!(
            without(([(555, 600), (700, 800)], 2), (560, 570)),
            None,
            "three windows are refused"
        );
        assert_eq!(
            without(([(555, 930), (0, 0)], 1), (500, 1_000)),
            Some(([(0, 0), (0, 0)], 0)),
            "a halt over the whole session leaves nothing owed"
        );
        assert!(absence_may_be_no_trade("CASH"));
        assert!(!absence_may_be_no_trade("INDEX"));
        assert!(!absence_may_be_no_trade("NOT-A-SEGMENT"));
    }

    /// One entry per month the window touches, each clipped to the window.
    #[test]
    fn the_plan_has_one_entry_per_month_clipped_to_the_window() {
        let root = std::env::temp_dir().join(format!("brutex-PLAN-SPAN-{}", std::process::id()));
        let _fresh = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let window = Window::new(
            Day::new(2020, 11, 25).unwrap(),
            Day::new(2021, 1, 5).unwrap(),
        )
        .unwrap();
        let months = plan(&root, parts(), window);
        let names: Vec<String> = months.iter().map(|m| m.month.to_string()).collect();
        assert_eq!(names, ["2020-11", "2020-12", "2021-01"]);
        assert_eq!(
            months[0].sessions, 3,
            "25, 26 and 27 November; the 30th was an exchange holiday"
        );
        assert_eq!(months[1].sessions, 22, "December 2020, less the 25th");
        assert_eq!(months[2].sessions, 3, "1, 4 and 5 January 2021");
        assert!(months.iter().all(|m| m.bars == 0 && m.refused.is_none()));
        let _cleanup = std::fs::remove_dir_all(&root);
    }
}
