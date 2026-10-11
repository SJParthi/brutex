//! `/calendar.json` built from the calendar of record, per feed and rung
//! (D-3336).
//!
//! # What was wrong
//!
//! `/calendar.json` was a vote. Every spot series a feed held derived its own
//! calendar from its bars, and the union was answered as the exchange's. On
//! the operator's store that was 343 derivations: 6.7 s (DB-G48) to 36.9 s
//! (measured cold on a shadow of that store, 9 Oct 2026) for 4.8 to 7.4 MB,
//! re-derived on every Zerodha landing, and a G-sec index (`NIFTYGS10YR`) that
//! publishes on Saturdays and Sundays alone voted 132 weekend days of 2015 and
//! 2016 into it, so 2015 read 357 sessions and 2016 289. GDFL holds only `1s`
//! files, which nothing read, so its calendar was `{"sessions":0,"days":[]}`
//! and read as "no sessions" (DB-015, DB-G29, DB-G48, DB-G85).
//!
//! # What it is now
//!
//! **The status of a day is the calendar of record's**:
//! [`pull::calendar::exchange_kind_of`] with its special sessions, its halts
//! and the closing auction (D-2805, D-3319, D-3320). That record runs from
//! 2018-01-01 to 2026-10-04. **Only outside it** does a stored bar speak, and
//! only the two spot indices the engine sweeps, `NIFTY` and `BANKNIFTY`, may:
//! a day either holds a bar on is an observed session of unrecorded length
//! (`U`); a day both were read for and neither holds is closed by observation
//! (`.`). No other series votes, so a G-sec or bond index cannot make a
//! weekend a session, and an index that is not one of the two cannot close a
//! day.
//!
//! **What a feed holds is a separate string**, `held`, one digit per day: how
//! many of the series read hold a bar that day, `-` where none was read. A
//! session the feed holds no bar on (`2022-03-07` in GDFL's index files, a
//! day its import refused, D-3318) is a session with a `0` beside it, and it is
//! listed by date in `sessionsNotHeld`, never silently dropped from the
//! calendar.
//!
//! # The day codes (D-3336 §3)
//!
//! | code | day |
//! |---|---|
//! | `.` | closed |
//! | `R` | regular session |
//! | `X` | regular session from 2026-08-03: a closing-auction-eligible share's continuous trading ends at 15:15 (charter §7, §9); an index stays continuous to 15:29 (D-3320) |
//! | `P` | special session with recorded windows (Muhurat, DR drill, Budget weekend, the 2021-02-24 outage) |
//! | `U` | session of unmeasured length |
//! | `H` | session with a suspected halt (2020-03-13, 2020-03-23: UNVERIFIED, no NSE circular recorded) |
//! | `-` | outside the calendar's span, and no observation either |
//!
//! # Owed bars per rung
//!
//! A regular session owes 375 one-minute bars and 22,500 one-second index
//! values; a CAS-eligible share from 2026-08-03 owes 360 minutes (Zerodha's
//! landing ends at 15:14, D-3320); a derived rung owes the buckets
//! [`pull::fold`] makes from a complete session, computed by folding one; a
//! special session owes what its windows hold; a halted minute is neither owed
//! nor missing. Trade-driven series (one-second shares and options) owe no
//! count (D-3336 §2), so `casEligible` at `1s` is `null`.
//!
//! # Cost
//!
//! Building one answer is one pass over the days of its span (about 4,300 for
//! Zerodha's 2015-2026) and, per special day, one pass over its 1,440 minutes;
//! the derived-rung counts fold at most 375 synthetic minutes per session
//! shape. It runs once per feed generation: the route keeps the answer under
//! the feed's manifest stamp and a request that finds it is one map probe.

use pull::calendar::{DayKind, Session};
use store::path::Timeframe;

/// A closed day.
pub const CLOSED: u8 = b'.';
/// A regular session before the closing auction existed.
pub const REGULAR: u8 = b'R';
/// A regular session on which eligible shares have a closing auction.
pub const AUCTION: u8 = b'X';
/// A special session with recorded windows.
pub const SPECIAL: u8 = b'P';
/// A session whose length is not recorded.
pub const UNSIZED: u8 = b'U';
/// A session with a suspected halt.
pub const HALTED: u8 = b'H';
/// Outside the calendar, and observed by nothing either.
pub const OUTSIDE: u8 = b'-';

/// The longest list of days one answer names before it says it stopped.
///
/// A bound on the body, not on the counts: every list's full count travels
/// beside it, and `truncated` says when a list was cut. 512 dates is about
/// two years of sessions, more than any feed is short by on its reference
/// indices.
pub const MAX_LISTED: usize = 512;

/// IST is UTC+5:30, so IST midnight of epoch day `d` is `d·86,400 − 19,800`
/// seconds since the epoch.
const IST_OFFSET_SECS: i64 = 19_800;

/// The legend, in the order of [the table above](self).
const LEGEND: [(u8, &str); 7] = [
    (CLOSED, "closed"),
    (REGULAR, "regular session"),
    (
        AUCTION,
        "regular session; a closing-auction-eligible share trades continuously to 15:15 and \
         auctions 15:15-15:29 (charter §7, §9); an index stays continuous (D-3320)",
    ),
    (SPECIAL, "special session with recorded windows"),
    (UNSIZED, "session of unmeasured length"),
    (
        HALTED,
        "session with a suspected market-wide halt (UNVERIFIED: no NSE circular recorded)",
    ),
    (
        OUTSIDE,
        "outside the calendar's span, and observed by nothing",
    ),
];

/// Whether `code` is a day the exchange traded.
#[must_use]
pub const fn is_session(code: u8) -> bool {
    matches!(code, REGULAR | AUCTION | SPECIAL | UNSIZED | HALTED)
}

/// `epoch_day` as a civil day, or `None` before 1970 or past what a
/// [`pull::session::Day`] carries.
fn civil(epoch_day: i64) -> Option<pull::session::Day> {
    u32::try_from(epoch_day)
        .ok()
        .and_then(|days| pull::session::Day::from_days(days).ok())
}

/// `epoch_day` as `YYYY-MM-DD`, or the bare number when it is not a civil day.
#[must_use]
pub fn iso(epoch_day: i64) -> String {
    civil(epoch_day).map_or_else(
        || epoch_day.to_string(),
        |day| format!("{:04}-{:02}-{:02}", day.year(), day.month(), day.day()),
    )
}

/// A minute of the day as `HH:MM`.
fn clock(minute: u16) -> String {
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

/// A second of the day as `HH:MM:SS`.
fn clock_secs(second: u32) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        second / 3_600,
        second % 3_600 / 60,
        second % 60
    )
}

/// Whether eligible shares have a closing auction on `epoch_day`.
fn auction_day(epoch_day: i64) -> bool {
    civil(epoch_day).is_some_and(pull::vendor::cash_auction_eligibility_required)
}

/// The day code the calendar of record gives `epoch_day`, or `None` outside
/// the record's span.
///
/// # Cost
///
/// [`pull::calendar::exchange_kind_of`], one walk of the three-entry halt
/// table and one of the fifteen-entry special-session table: O(1).
#[must_use]
pub fn record_code(epoch_day: i64) -> Option<u8> {
    match pull::calendar::exchange_kind_of(epoch_day) {
        DayKind::Unmeasured => None,
        DayKind::Closed => Some(CLOSED),
        DayKind::OpenLengthUnmeasured => Some(UNSIZED),
        DayKind::Open(session) => Some(
            if pull::calendar::halt_on(epoch_day).is_some_and(|halt| halt.evidence.is_suspected()) {
                HALTED
            } else if pull::calendar::special_session(epoch_day).is_some()
                || session != Session::full()
            {
                SPECIAL
            } else if auction_day(epoch_day) {
                AUCTION
            } else {
                REGULAR
            },
        ),
    }
}

/// The minute opens `epoch_day` owes at the exchange session, a recorded halt's
/// whole minutes taken out: empty for a closed day, `None` where the record
/// does not size the day.
///
/// One pass over the day's 1,440 minutes; asked only for special and halt
/// days, never per day of a span.
#[must_use]
pub fn owed_minutes(epoch_day: i64) -> Option<Vec<u16>> {
    match pull::calendar::exchange_kind_of(epoch_day) {
        DayKind::Closed => Some(Vec::new()),
        DayKind::Unmeasured | DayKind::OpenLengthUnmeasured => None,
        DayKind::Open(session) => {
            let halted = pull::calendar::halt_on(epoch_day).map(|halt| halt.halted);
            Some(
                (0..pull::calendar::MINUTES_PER_DAY)
                    .filter(|minute| {
                        session.expects(*minute) && !halted.is_some_and(|w| w.holds(*minute))
                    })
                    .collect(),
            )
        }
    }
}

/// The minute opens a regular session owes: 09:15 through 15:29.
fn regular_minutes() -> Vec<u16> {
    (pull::calendar::OPEN_MINUTE..=pull::calendar::LAST_MINUTE).collect()
}

/// The minute opens a closing-auction-eligible share's regular session owes
/// from 2026-08-03: 09:15 through 15:14 (D-3320, D-3336 §1).
fn cas_minutes() -> Vec<u16> {
    (pull::calendar::OPEN_MINUTE..pull::calendar::CAS_CONTINUOUS_CLOSE).collect()
}

/// How many seconds of `[from, to)` lie inside `[cut_from, cut_to)`.
const fn overlap(from: u32, to: u32, cut_from: u32, cut_to: u32) -> u32 {
    let start = if from > cut_from { from } else { cut_from };
    let end = if to < cut_to { to } else { cut_to };
    end.saturating_sub(start)
}

/// The bars `minutes` of `epoch_day` owe at `rung`, for a series on `venue`.
///
/// - `1s`: sixty seconds per minute, less every second of a recorded halt
///   `venue`'s own one-second series spends in it ([`pull::calendar::Halt::seconds`]).
/// - `1min`: one per minute.
/// - a wider rung: the buckets [`pull::fold::fold_from_bars`] makes from those
///   minutes, one synthetic bar each, on the fold's own grid. The fold is the
///   one authority for where a bucket starts; restating its anchor here would
///   be a second one.
///
/// `None` when the fold refuses the width.
#[must_use]
pub fn owed_at(
    epoch_day: i64,
    minutes: &[u16],
    rung: Timeframe,
    venue: pull::vendor::Venue,
) -> Option<u64> {
    if rung == Timeframe::SECOND_1 {
        let halt = pull::calendar::halt_on(epoch_day).map(|halt| halt.seconds(venue));
        return Some(
            minutes
                .iter()
                .map(|minute| {
                    let from = u32::from(*minute) * 60;
                    let cut = halt.map_or(0, |seconds| {
                        overlap(from, from + 60, seconds.from, seconds.resume)
                    });
                    u64::from(60_u32.saturating_sub(cut))
                })
                .sum(),
        );
    }
    if rung == Timeframe::MINUTE_1 {
        return u64::try_from(minutes.len()).ok();
    }
    let bucket = pull::fold::Bucket::of_secs(rung.secs())?;
    let midnight = epoch_day
        .checked_mul(86_400)?
        .checked_sub(IST_OFFSET_SECS)?;
    let bars: Vec<store::format::Bar> = minutes
        .iter()
        .filter_map(|minute| {
            let secs = midnight.checked_add(i64::from(*minute) * 60)?;
            Some(store::format::Bar {
                ts_micros: secs.checked_mul(1_000_000)?,
                open: 1,
                high: 1,
                low: 1,
                close: 1,
                volume: 0,
                open_interest: store::format::OI_NULL,
            })
        })
        .collect();
    let folded = pull::fold::fold_from_bars(&bars, bucket, pull::fold::Bucket::MINUTE).ok()?;
    u64::try_from(folded.len()).ok()
}

/// One series' reading: for each day of its span, how many bars it holds, or
/// `None` where that day was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    /// The series' name, as the answer lists it.
    pub name: String,
    /// The first day of `counts`.
    first: i64,
    /// One entry per day from `first`.
    counts: Vec<Option<u32>>,
}

impl Reading {
    /// A reading from a [`pull::calendar::Calendar`], for a test to build one
    /// by hand: a day it calls open holds a bar, a day it calls closed was
    /// read and holds none, and an unmeasured day was not read.
    #[cfg(test)]
    #[must_use]
    pub fn from_calendar(name: &str, calendar: &pull::calendar::Calendar) -> Self {
        let first = calendar.first_day();
        let counts = (first..=calendar.last_day())
            .map(|day| match calendar.kind_of(day) {
                DayKind::Open(_) | DayKind::OpenLengthUnmeasured => Some(1),
                DayKind::Closed => Some(0),
                DayKind::Unmeasured => None,
            })
            .collect();
        Self {
            name: name.to_owned(),
            first,
            counts,
        }
    }

    /// A reading from per-month day counts, in ascending month order: a month
    /// whose counts are `None` was held and could not be read.
    #[must_use]
    pub fn from_months(
        name: &str,
        months: &[(
            store::path::YearMonth,
            Option<crate::calendar_of::MonthDays>,
        )],
    ) -> Self {
        let mut first: Option<i64> = None;
        let mut counts: Vec<Option<u32>> = Vec::new();
        for (month, days) in months {
            let Ok(one) = pull::session::Day::new(month.year(), month.month(), 1) else {
                continue;
            };
            let start = i64::from(one.days_from_epoch());
            let end = i64::from(one.end_of_month().days_from_epoch());
            let origin = *first.get_or_insert(start);
            // A GAP BETWEEN TWO HELD MONTHS IS NOT READ: its days stay `None`.
            for (offset, day) in (start..=end).enumerate() {
                let Ok(at) = usize::try_from(day.saturating_sub(origin)) else {
                    continue;
                };
                if counts.len() <= at {
                    counts.resize(at.saturating_add(1), None);
                }
                if let Some(slot) = counts.get_mut(at) {
                    *slot = days.as_ref().map(|held| held.count(offset));
                }
            }
        }
        Self {
            name: name.to_owned(),
            first: first.unwrap_or(0),
            counts,
        }
    }

    /// What this reading says of `epoch_day`.
    #[must_use]
    pub fn at(&self, epoch_day: i64) -> Option<u32> {
        epoch_day
            .checked_sub(self.first)
            .and_then(|at| usize::try_from(at).ok())
            .and_then(|at| self.counts.get(at).copied())
            .flatten()
    }

    /// The first and last day this reading holds a bar on.
    #[must_use]
    pub fn held_span(&self) -> Option<(i64, i64)> {
        let held = |count: &Option<u32>| count.is_some_and(|n| n > 0);
        let first = self.counts.iter().position(held)?;
        let last = self.counts.iter().rposition(held)?;
        Some((
            self.first
                .saturating_add(i64::try_from(first).unwrap_or(i64::MAX)),
            self.first
                .saturating_add(i64::try_from(last).unwrap_or(i64::MAX)),
        ))
    }
}

/// Who the answer is about.
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// The feed's exchange calendar, held by its reference indices.
    Exchange,
    /// One stored series.
    Series {
        /// `NSE` or `BSE`.
        exchange: &'a str,
        /// `INDEX`, `CASH` or `FNO`.
        segment: &'a str,
        /// The stored name.
        symbol: &'a str,
    },
}

/// What one answer is asked for.
#[derive(Debug, Clone, Copy)]
pub struct Asked<'a> {
    /// The feed's wire name.
    pub feed: &'a str,
    /// The rung whose files attest the span and the holdings.
    pub rung: Timeframe,
    /// Whether the caller named the rung, or the feed's default was taken.
    pub rung_asked: bool,
    /// The series, or the exchange.
    pub target: Target<'a>,
    /// The feed's census generation the answer was built from.
    pub generation: Option<u64>,
}

/// The target as JSON.
fn target_json(target: Target<'_>) -> serde_json::Value {
    match target {
        Target::Exchange => serde_json::json!({ "kind": "exchange" }),
        Target::Series {
            exchange,
            segment,
            symbol,
        } => serde_json::json!({
            "kind": "series",
            "exchange": exchange,
            "segment": segment,
            "symbol": symbol,
        }),
    }
}

/// The answer for a feed and rung that holds nothing to build a calendar from,
/// with the reason: never an empty list that reads as "no sessions".
#[must_use]
pub fn withheld(
    feed: &str,
    rung: Option<Timeframe>,
    target: Target<'_>,
    generation: Option<u64>,
    reason: &str,
) -> String {
    serde_json::json!({
        "feed": feed,
        "rung": rung.map(Timeframe::as_str),
        "target": target_json(target),
        "status": "withheld",
        "reason": reason,
        "generation": generation,
        "from": null,
        "to": null,
        "firstDay": null,
        "lastDay": null,
        "days": "",
        "held": "",
        "sessions": 0,
        "calendarSessions": 0,
        "sessionsNotHeld": [],
        "withheld": [],
        "derivedFrom": [],
        "disagreements": [],
        "basis": basis_json(None, 0, &[]),
    })
    .to_string()
}

/// Where the answer's day statuses come from.
fn basis_json(rung: Option<Timeframe>, observed: u64, by: &[String]) -> serde_json::Value {
    serde_json::json!({
        "record": {
            "from": iso(pull::calendar::SOURCED_FIRST_DAY),
            "to": iso(pull::calendar::SOURCED_LAST_DAY),
            "source": "pull::calendar::exchange_kind_of: the measured 2019-12-02..2026-09-04 window, \
                       NSE's holiday circulars either side (D-2805), recorded special-session \
                       windows (D-3319), halts and the closing auction (D-3320)",
        },
        "observed": {
            "by": by,
            "rung": rung.map(Timeframe::as_str),
            "days": observed,
            "rule": "outside the record only: a day NIFTY or BANKNIFTY holds a bar on is a \
                     session of unrecorded length (U); a day both were read for and neither \
                     holds is closed (.); no other series votes",
        },
    })
}

/// The first day eligible shares have a closing auction, by bisection over
/// the record's span: `None` if it starts after the span.
fn auction_start() -> Option<i64> {
    let (mut low, mut high) = (
        pull::calendar::SOURCED_FIRST_DAY,
        pull::calendar::SOURCED_LAST_DAY,
    );
    if !auction_day(high) {
        return None;
    }
    while low < high {
        let mid = low + (high - low) / 2;
        if auction_day(mid) {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    Some(low)
}

/// Owed bars at `rung` for one day's minutes, as JSON: a number, or `null`.
fn owed_json(day: i64, minutes: &[u16], rung: Timeframe) -> serde_json::Value {
    owed_at(day, minutes, rung, pull::vendor::Venue::NseIndex)
        .map_or(serde_json::Value::Null, serde_json::Value::from)
}

/// The regular-session row of `owed` at `rung`.
fn owed_table(rung: Timeframe) -> serde_json::Value {
    // ANY ORDINARY WEEKDAY OF THE RECORD WILL DO: the counts depend on the
    // session's minutes and the fold's grid, and a regular day carries neither
    // a halt nor a window of its own. 2024-01-01 is a regular Monday.
    const ORDINARY: i64 = 19_723;
    let cas = if rung == Timeframe::SECOND_1 {
        // A ONE-SECOND SHARE IS TRADE-DRIVEN: it owes no count (D-3336 §2).
        serde_json::Value::Null
    } else {
        owed_json(ORDINARY, &cas_minutes(), rung)
    };
    serde_json::json!({
        "rung": rung.as_str(),
        "regular": owed_json(ORDINARY, &regular_minutes(), rung),
        "casEligible": cas,
    })
}

/// Every special or halt day of `from..=to` the record sizes, as JSON.
fn specials_json(from: i64, to: i64, rung: Timeframe) -> Vec<serde_json::Value> {
    let mut days: Vec<i64> = pull::calendar::SPECIAL_SESSIONS
        .iter()
        .map(|special| special.day)
        .chain(pull::calendar::HALTS.iter().map(|halt| halt.day))
        .filter(|day| (from..=to).contains(day))
        .collect();
    days.sort_unstable();
    days.dedup();
    days.into_iter()
        .filter_map(|day| {
            let minutes = owed_minutes(day)?;
            let code = record_code(day)?;
            let special = pull::calendar::special_session(day);
            let DayKind::Open(session) = pull::calendar::exchange_kind_of(day) else {
                return None;
            };
            let windows: Vec<serde_json::Value> = session
                .windows
                .iter()
                .take(usize::from(session.count))
                .map(|window| serde_json::json!({ "from": clock(window.from), "to": clock(window.to) }))
                .collect();
            // THE INDEX DENOMINATOR ON THE OUTAGE DAY IS UNPROVED (D-0420):
            // SEBI records unavailable NIFTY computation and no common
            // publication window, so an intraday index count is `null`. A
            // daily bar is still one bar.
            let index_unproved =
                day == pull::calendar::SYSTEMS_OUTAGE_DAY && rung != Timeframe::DAY_1;
            let owed = owed_json(day, &minutes, rung);
            let minute_owed = owed_json(day, &minutes, Timeframe::MINUTE_1);
            Some(serde_json::json!({
                "day": day,
                "date": iso(day),
                "weekday": pull::calendar::weekday_of(day).short(),
                "code": char::from(code).to_string(),
                "kind": special.map_or("suspected halt", |s| s.kind.label()),
                "windows": windows,
                "owed": owed,
                "indexOwed": if index_unproved { serde_json::Value::Null } else { owed.clone() },
                "minuteOwed": minute_owed,
                "minuteIndexOwed": if day == pull::calendar::SYSTEMS_OUTAGE_DAY {
                    serde_json::Value::Null
                } else {
                    minute_owed.clone()
                },
                "evidence": special.map_or("", |s| s.evidence),
            }))
        })
        .collect()
}

/// Every recorded halt of `from..=to`, as JSON.
fn halts_json(from: i64, to: i64) -> Vec<serde_json::Value> {
    pull::calendar::HALTS
        .iter()
        .filter(|halt| (from..=to).contains(&halt.day))
        .map(|halt| {
            let seconds = |s: pull::calendar::HaltSeconds| {
                serde_json::json!({ "from": clock_secs(s.from), "resume": clock_secs(s.resume) })
            };
            serde_json::json!({
                "day": halt.day,
                "date": iso(halt.day),
                "halted": { "from": clock(halt.halted.from), "to": clock(halt.halted.to) },
                "reopening": halt.reopening.map(|w| serde_json::json!({ "from": clock(w.from), "to": clock(w.to) })),
                "seconds": {
                    "index": seconds(halt.index),
                    "cash": seconds(halt.cash),
                    "derivatives": seconds(halt.derivatives),
                },
                "suspected": halt.evidence.is_suspected(),
                "evidence": halt.evidence.text(),
                "rule": "a halted minute is neither owed nor missing (D-3336 §1)",
            })
        })
        .collect()
}

/// Days of one list, bounded by [`MAX_LISTED`].
#[derive(Default)]
struct Listed {
    /// The days, as JSON, at most [`MAX_LISTED`].
    days: Vec<serde_json::Value>,
    /// How many there were.
    count: u64,
}

impl Listed {
    /// Count `day`, and list it while there is room.
    fn push(&mut self, day: i64, code: u8) {
        self.count = self.count.saturating_add(1);
        if self.days.len() < MAX_LISTED {
            self.days.push(serde_json::json!({
                "day": day,
                "date": iso(day),
                "weekday": pull::calendar::weekday_of(day).short(),
                "code": char::from(code).to_string(),
            }));
        }
    }

    /// The list and its count.
    fn json(self) -> serde_json::Value {
        let truncated = u64::try_from(self.days.len()).unwrap_or(u64::MAX) < self.count;
        serde_json::json!({ "count": self.count, "truncated": truncated, "days": self.days })
    }
}

/// The days of `from..=to` as a dense status string, the observed-day count,
/// and the observed weekend sessions.
///
/// `attest` speaks only where the record does not.
fn statuses(from: i64, to: i64, attest: &[Reading]) -> (String, u64, Listed) {
    let mut days = String::with_capacity(usize::try_from(to - from + 1).unwrap_or(0));
    let mut observed = 0_u64;
    let mut weekends = Listed::default();
    for day in from..=to {
        let code = record_code(day).unwrap_or_else(|| {
            let mut read = false;
            let mut held = false;
            for reading in attest {
                match reading.at(day) {
                    Some(0) => read = true,
                    Some(_) => held = true,
                    None => {}
                }
            }
            if held {
                observed = observed.saturating_add(1);
                if pull::calendar::weekday_of(day).is_weekend() {
                    weekends.push(day, UNSIZED);
                }
                UNSIZED
            } else if read {
                CLOSED
            } else {
                OUTSIDE
            }
        });
        days.push(char::from(code));
    }
    (days, observed, weekends)
}

/// What one pass over an answer's days finds beside their statuses.
#[derive(Default)]
struct Walk {
    /// One digit per day: how many holders hold a bar; `-` where none read it.
    held: String,
    /// Sessions at least one holder holds a bar on.
    sessions: u64,
    /// Every session of the span.
    calendar_sessions: u64,
    /// Sessions read and held by no holder.
    not_held: Listed,
    /// Sessions some holders hold and others do not.
    partly: Listed,
    /// Sessions no holder's reading covers.
    unread: Listed,
    /// Closed days a holder holds a bar on.
    on_closed: Listed,
    /// The `-` days, as `{from, to}` runs.
    withheld: Vec<serde_json::Value>,
}

/// One pass over `from..=to` with their status `days`, against `holders`.
fn walk(from: i64, to: i64, days: &str, holders: &[Reading]) -> Walk {
    let mut walked = Walk {
        held: String::with_capacity(days.len()),
        ..Walk::default()
    };
    let mut run: Option<(i64, i64)> = None;
    for (day, code) in (from..=to).zip(days.bytes()) {
        let mut read = false;
        let mut holding = 0_usize;
        for reading in holders {
            match reading.at(day) {
                Some(0) => read = true,
                Some(_) => {
                    read = true;
                    holding = holding.saturating_add(1);
                }
                None => {}
            }
        }
        walked.held.push(if read {
            char::from_digit(u32::try_from(holding.min(9)).unwrap_or(9), 10).unwrap_or('9')
        } else {
            char::from(OUTSIDE)
        });
        if is_session(code) {
            walked.calendar_sessions = walked.calendar_sessions.saturating_add(1);
            if holding > 0 {
                walked.sessions = walked.sessions.saturating_add(1);
                if holding < holders.len() {
                    walked.partly.push(day, code);
                }
            } else if read {
                walked.not_held.push(day, code);
            } else {
                walked.unread.push(day, code);
            }
        } else if code == CLOSED && holding > 0 {
            walked.on_closed.push(day, code);
        }
        if code == OUTSIDE {
            run = Some(run.map_or((day, day), |(start, _)| (start, day)));
        } else if let Some((start, end)) = run.take() {
            walked
                .withheld
                .push(serde_json::json!({ "from": start, "to": end }));
        }
    }
    if let Some((start, end)) = run {
        walked
            .withheld
            .push(serde_json::json!({ "from": start, "to": end }));
    }
    walked
}

/// The full answer for `asked`.
///
/// `holders` say what the feed holds (the series asked for, or the reference
/// indices); `attest` are the reference indices, which alone speak outside the
/// record. `unreadable` names every month a reading could not read.
#[must_use]
pub fn answer(
    asked: &Asked<'_>,
    holders: &[Reading],
    attest: &[Reading],
    unreadable: &[String],
) -> String {
    let span = holders.iter().filter_map(Reading::held_span).fold(
        None,
        |acc: Option<(i64, i64)>, (first, last)| {
            Some(acc.map_or((first, last), |(a, b)| (a.min(first), b.max(last))))
        },
    );
    let Some((from, to)) = span else {
        let names: Vec<&str> = holders.iter().map(|r| r.name.as_str()).collect();
        return withheld(
            asked.feed,
            Some(asked.rung),
            asked.target,
            asked.generation,
            &format!(
                "{} holds no {} bar for {}: there is no span to give a calendar for",
                asked.feed,
                asked.rung.as_str(),
                if names.is_empty() {
                    "the series asked".to_owned()
                } else {
                    names.join(" or ")
                },
            ),
        );
    };
    let (days, observed, weekends) = statuses(from, to, attest);
    let walked = walk(from, to, &days, holders);
    let names: Vec<String> = holders.iter().map(|r| r.name.clone()).collect();
    let attesters: Vec<String> = attest.iter().map(|r| r.name.clone()).collect();
    let legend: serde_json::Map<String, serde_json::Value> = LEGEND
        .iter()
        .map(|(code, what)| {
            (
                char::from(*code).to_string(),
                serde_json::Value::from(*what),
            )
        })
        .collect();
    serde_json::json!({
        "feed": asked.feed,
        "rung": asked.rung.as_str(),
        "rungAsked": asked.rung_asked,
        "target": target_json(asked.target),
        "status": "ok",
        "reason": null,
        "generation": asked.generation,
        "from": from,
        "to": to,
        "firstDay": from,
        "lastDay": to,
        "fromDate": iso(from),
        "toDate": iso(to),
        "days": days,
        "held": walked.held,
        "heldBy": names,
        "legend": legend,
        "heldLegend": "one digit per day: how many of heldBy hold a bar that day; - where none was read",
        "sessions": walked.sessions,
        "calendarSessions": walked.calendar_sessions,
        "sessionsNotHeld": walked.not_held.json(),
        "sessionsPartlyHeld": walked.partly.json(),
        "sessionsUnread": walked.unread.json(),
        "heldOnClosedDays": walked.on_closed.json(),
        "observedWeekendSessions": weekends.json(),
        "owed": owed_table(asked.rung),
        "minuteOwed": owed_table(Timeframe::MINUTE_1),
        "special": specials_json(from, to, asked.rung),
        "halts": halts_json(from, to),
        "cas": {
            "from": auction_start().map(iso),
            "continuousClose": clock(pull::calendar::CAS_CONTINUOUS_CLOSE),
            "auction": format!(
                "{}-{}",
                clock(pull::calendar::CAS_CONTINUOUS_CLOSE),
                clock(pull::calendar::LAST_MINUTE)
            ),
            "eligibility": "per share, from the dated NSE security master's ElgbltyClsgAuctnSsn \
                            flag (charter §9); unknown is never read as not eligible",
            "index": "continuous to 15:29; never a closing-auction phase (D-3320)",
        },
        "basis": basis_json(Some(asked.rung), observed, &attesters),
        "unreadable": unreadable,
        "withheld": walked.withheld,
        "derivedFrom": attesters,
        "disagreements": [],
    })
    .to_string()
}

/// `body`, a held-span answer of [`answer`], widened to the calendar of
/// record's whole span (`/calendar.json?span=record`, DB-B42): the record's
/// day codes for every day of `SOURCED_FIRST_DAY..=SOURCED_LAST_DAY` the feed
/// holds no bar on, with `held` a `-` there, so a page asking about a day the
/// store does not reach reads the record (2026-10-02 closed) rather than
/// guessing a weekday is a session. What the feed HOLDS is unchanged: `held`,
/// `heldBy`, `sessions` and the held span (`heldSpan`) are the store's facts.
///
/// `None` when `body` is not an `ok` answer with a day string (a withheld or
/// refused answer is answered as it is).
///
/// # Cost
///
/// One parse of the kept answer and one pass over the days added: at most the
/// record's 3,199 days. O(1) per request in the store's size.
#[must_use]
pub fn widen_to_record(body: &str, rung: Option<Timeframe>) -> Option<String> {
    let mut value: serde_json::Value = serde_json::from_str(body).ok()?;
    if value.get("status").and_then(serde_json::Value::as_str) != Some("ok") {
        return None;
    }
    let from = value.get("from")?.as_i64()?;
    let to = value.get("to")?.as_i64()?;
    let days = value.get("days")?.as_str()?.to_owned();
    let held = value.get("held")?.as_str()?.to_owned();
    let rung = rung
        .or_else(|| {
            let word = value.get("rung").and_then(serde_json::Value::as_str)?;
            Timeframe::KNOWN
                .iter()
                .copied()
                .find(|known| known.as_str() == word)
        })
        .unwrap_or(Timeframe::MINUTE_1);
    let first = from.min(pull::calendar::SOURCED_FIRST_DAY);
    let last = to.max(pull::calendar::SOURCED_LAST_DAY);
    let code_of = |day: i64| record_code(day).unwrap_or(OUTSIDE);
    let mut added_sessions = 0_u64;
    let mut before = Listed::default();
    let mut after = Listed::default();
    let mut widened_days = String::with_capacity(usize::try_from(last - first + 1).unwrap_or(0));
    let mut widened_held = String::with_capacity(widened_days.capacity());
    for day in first..from {
        let code = code_of(day);
        if is_session(code) {
            added_sessions = added_sessions.saturating_add(1);
            before.push(day, code);
        }
        widened_days.push(char::from(code));
        widened_held.push(char::from(OUTSIDE));
    }
    widened_days.push_str(&days);
    widened_held.push_str(&held);
    for day in to.saturating_add(1)..=last {
        let code = code_of(day);
        if is_session(code) {
            added_sessions = added_sessions.saturating_add(1);
            after.push(day, code);
        }
        widened_days.push(char::from(code));
        widened_held.push(char::from(OUTSIDE));
    }
    let unread = merged_list(before, value.get("sessionsUnread"), after);
    let object = value.as_object_mut()?;
    let calendar_sessions = object
        .get("calendarSessions")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
        .saturating_add(added_sessions);
    let mut set = |key: &str, field: serde_json::Value| {
        let _replaced = object.insert(key.to_owned(), field);
    };
    set("from", first.into());
    set("to", last.into());
    set("firstDay", first.into());
    set("lastDay", last.into());
    set("fromDate", iso(first).into());
    set("toDate", iso(last).into());
    set("days", widened_days.into());
    set("held", widened_held.into());
    set("calendarSessions", calendar_sessions.into());
    set("sessionsUnread", unread);
    set("special", specials_json(first, last, rung).into());
    set("halts", halts_json(first, last).into());
    set("span", "record".into());
    set(
        "heldSpan",
        serde_json::json!({ "from": iso(from), "to": iso(to), "firstDay": from, "lastDay": to }),
    );
    set(
        "spanNote",
        format!(
            "widened to the calendar of record ({} to {}): a day outside the held span {} to {} \
             carries the record's own code and a held digit of -, so a session there is owed \
             and unread, never assumed and never held",
            iso(pull::calendar::SOURCED_FIRST_DAY),
            iso(pull::calendar::SOURCED_LAST_DAY),
            iso(from),
            iso(to)
        )
        .into(),
    );
    Some(value.to_string())
}

/// **AN ANSWER THAT WAS ASKED FOR `span=record` AND IS NOT WIDENED SAYS SO**
/// (WP-61 repair). A withheld, building or refused answer was returned as it
/// stood, with no `span` field, so a reader that asked for the record's span
/// could not tell a widened answer from one that was not. The body gains
/// `span: "held"` and a `spanNote` with the reason; every other field is
/// untouched.
///
/// `None` when `body` is not a JSON object (it is then answered as it is).
///
/// # Cost
///
/// One parse of the kept answer: O(1) per request in the store's size.
#[must_use]
pub fn not_widened(body: &str) -> Option<String> {
    let mut value: serde_json::Value = serde_json::from_str(body).ok()?;
    let status = value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("no status")
        .to_owned();
    let object = value.as_object_mut()?;
    let _was = object.insert("span".to_owned(), "held".into());
    let _note = object.insert(
        "spanNote".to_owned(),
        format!(
            "span=record was asked and this answer is NOT widened to the calendar of record: \
             only a finished answer (status ok, with its day string) is widened, and this one \
             is {status:?}. No day outside what it states is claimed open or closed"
        )
        .into(),
    );
    Some(value.to_string())
}

/// `before`, the list `middle` already holds, and `after`, as one bounded list
/// in day order.
fn merged_list(
    before: Listed,
    middle: Option<&serde_json::Value>,
    after: Listed,
) -> serde_json::Value {
    let middle_count = middle
        .and_then(|list| list.get("count"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let middle_days: Vec<serde_json::Value> = middle
        .and_then(|list| list.get("days"))
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let count = before
        .count
        .saturating_add(middle_count)
        .saturating_add(after.count);
    let days: Vec<serde_json::Value> = before
        .days
        .into_iter()
        .chain(middle_days)
        .chain(after.days)
        .take(MAX_LISTED)
        .collect();
    let truncated = u64::try_from(days.len()).unwrap_or(u64::MAX) < count;
    serde_json::json!({ "count": count, "truncated": truncated, "days": days })
}

#[cfg(test)]
#[path = "calendar_record_tests.rs"]
mod tests;
