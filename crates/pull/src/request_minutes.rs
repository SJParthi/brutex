//! Request-wide spot-minute evidence, separate from per-month derivation.
//! A monotone row cursor and one visit per civil day: O(rows + days + gaps).
//! No expected-minute enumeration, synthetic candles, or disk-history inference.
//!
//! # What reaches the receipt, the log and the ledger (D-3335, D-3341)
//!
//! The audit finds runs of missing scheduled minutes and days it cannot
//! attest, and [`report`] hands them on three ways:
//!
//! - **the ledger**, `<store>/audit/<vendor>.gaps` (D-3335's Zerodha ledger
//!   is `audit/zerodha.gaps`): one `gap` line per day that missed a minute
//!   or was withheld, with the day's owed, held and missing minutes, its runs
//!   and its longest run. Durable, per day, and the only surface that keeps
//!   every day: the rolling log covered 8 to 12 minutes of a pull (DB-G88);
//! - **the receipt**: one [`Failure`] per (month, reason), counting its
//!   runs, days and minutes and quoting the first sentence. A reason seen
//!   once is its own sentence, unchanged;
//! - **the log**: one `pull.request_minutes` event per (instrument, month,
//!   reason), with `count`, `first_day` and `last_day` (D-3341 item 2). One
//!   event per gap was 4,847 errors in one log window, and each receipt
//!   failure was a `pull.spot` "member did not land" line besides.
//!
//! The derive's `derive` lines are written to the same ledger by
//! `ingest::derive` through [`append_ledger`] and [`derive_line`].
//!
//! # Cost
//!
//! The audit is one pass; the aggregation one pass over its findings with at
//! most a handful of groups per month; the ledger one append of the member's
//! lines, written once and synced once. UNVERIFIED as a measured bound: no
//! bench times this. `docs/06-limits.md`, D-1493.

use std::io::Write as _;
use std::path::Path;

use super::{Failure, Ingested, Plan};
use crate::calendar::{DayKind, MAX_WINDOWS, Session};
use crate::fetch::RawRow;
use crate::session::{Day, IST_OFFSET_SECS};
use crate::vendor::{Granularity, SessionKind, TimestampEncoding, Venue};
use brutex_core::instrument::{Exchange, Segment};

/// Why a finding is named: the ledger's `why=` and the event's `code`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Why {
    /// The vendor answered no candle for scheduled minutes (D-3335).
    VendorEmpty,
    /// A traded session whose length no record states: the day is withheld
    /// (D-3335, D-3344).
    WithheldUnsized,
    /// A day no calendar or venue table can attest.
    Unverified,
    /// A day past the end of the calendar of record
    /// ([`crate::calendar::SOURCED_LAST_DAY`]): nothing judges it, and the
    /// run says so in one line rather than once per instrument (DB-B25).
    PastRecord,
    /// Broker stamps out of order: nothing about the window is attested.
    Unordered,
    /// Rows the vendor sent on this day were refused — by name, a row at a
    /// time (D-3351), or by the store, which refused the day's month inside
    /// its held range — and the day is short in the store by them (D-3335's
    /// `refused_day`).
    RefusedDay,
    /// The store refused this day's rows because the month already holds
    /// later days: a span before the month's first held bar that only a
    /// versioned repair can fill (D-3335's `refused_front`, D-3350).
    RefusedFront,
    /// A versioned repair added this day's bars (D-3350): the line supersedes
    /// the day's earlier `refused_*` line, last line of a key winning.
    Repaired,
    /// The STORE holds fewer of the day's owed minutes than the session owes,
    /// found from the stored month and the calendar with no vendor call
    /// (R3-18, `ingest::gap_backfill`): a month landed before this ledger
    /// existed, or whose answer's line was lost. What the vendor answered is
    /// not known, so no vendor reason is claimed; the line's `fill=` says what
    /// can fill it.
    StoredShort,
}

impl Why {
    /// The code, never renamed or reused.
    const fn code(self) -> &'static str {
        match self {
            Self::VendorEmpty => "vendor_empty",
            Self::WithheldUnsized => "withheld_unmeasured_session",
            Self::Unverified => "calendar_unverified",
            Self::PastRecord => "calendar_past_record",
            Self::Unordered => "unordered_stamps",
            Self::RefusedDay => "refused_day",
            Self::RefusedFront => "refused_front",
            Self::Repaired => "repaired",
            Self::StoredShort => "stored_short",
        }
    }
}

/// What a landing refused of an answer it was handed, for the ledger: the
/// IST-local epoch second of every refused row, and the days the store
/// refused before a month's first held bar. D-3351, WP-13c.
///
/// The ledger audits the VENDOR's rows, so a minute refused after the vendor
/// sent it — a row named and refused, or a month the append-only store
/// refused — read as held, and the 35 sessions the store refused on the
/// operator's store had no line at all (DB-N22). Each source keeps its own
/// list in time order; nothing here is sorted.
#[derive(Debug, Clone, Default)]
pub(super) struct Refusals {
    /// One time-ordered list per source: rows refused by name, rows of a
    /// month the store refused.
    pub(super) seconds: Vec<Vec<i64>>,
    /// IST day numbers before a refused month's first held bar, in order.
    pub(super) fronts: Vec<u32>,
    /// IST day numbers a repair added bars on, in order (D-3350).
    pub(super) repaired: Vec<u32>,
}

/// One pointer per time-ordered list of refused seconds, each advanced only
/// forward, so a membership question asked in time order is O(1) amortised.
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
struct Cursor<'a> {
    lists: Vec<(&'a [i64], usize)>,
}

impl<'a> Cursor<'a> {
    fn new(lists: &'a [Vec<i64>]) -> Self {
        Self {
            lists: lists.iter().map(|list| (list.as_slice(), 0)).collect(),
        }
    }

    /// Whether `second` is refused. Asked with `second` non-decreasing.
    fn refused(&mut self, second: i64) -> bool {
        let mut hit = false;
        for (list, at) in &mut self.lists {
            while list.get(*at).is_some_and(|&s| s < second) {
                *at += 1;
            }
            hit |= list.get(*at) == Some(&second);
        }
        hit
    }

    /// Whether any refused second lies in `[from, to)`. Asked with `from`
    /// non-decreasing; leaves every pointer at or past `from`.
    fn any_in(&mut self, from: i64, to: i64) -> bool {
        let mut hit = false;
        for (list, at) in &mut self.lists {
            while list.get(*at).is_some_and(|&s| s < from) {
                *at += 1;
            }
            hit |= list.get(*at).is_some_and(|&s| s < to);
        }
        hit
    }
}

/// One thing the audit names: a run of missing minutes, or a day or window
/// it cannot attest.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    why: Why,
    /// The day it is about, when it is about one.
    day: Option<Day>,
    /// The reason within its code, without the day: findings of one month
    /// group under (code, detail), so two different reasons never share a
    /// count.
    detail: &'static str,
    /// Missing scheduled minutes, for a gap; zero otherwise.
    minutes: u64,
    /// The sentence, as the receipt has always carried it.
    text: String,
}

/// One audited day's `gap` line (D-3335).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DayGaps {
    day: Day,
    /// Minutes the session owed.
    owed: u32,
    /// Minutes the answer held, of those owed (for an unsized day, the
    /// answer's rows on it).
    held: u32,
    /// Minutes owed and not held.
    missing: u32,
    /// The first missing minute, as minutes past IST midnight.
    first_missing: Option<u32>,
    /// Runs of consecutive missing minutes.
    runs: u32,
    /// The longest run, in minutes, and where it began.
    longest: u32,
    longest_at: Option<u32>,
    why: Why,
}

impl DayGaps {
    const fn new(day: Day, why: Why) -> Self {
        Self {
            day,
            owed: 0,
            held: 0,
            missing: 0,
            first_missing: None,
            runs: 0,
            longest: 0,
            longest_at: None,
            why,
        }
    }
}

/// What the audit of one answer found.
#[derive(Debug, Default)]
struct Audit {
    findings: Vec<Finding>,
    days: Vec<DayGaps>,
}

/// Audits one answer and hands its findings to the receipt, the log and the
/// ledger. See the module header.
///
/// `done` is the member's receipt from `from_members`; the aggregated
/// failures are appended to it. A ledger that cannot be written is a named
/// failure and a `pull.file` "not filed" line, never silence.
pub(super) fn report(
    rows: &[RawRow],
    instrument: &str,
    store_root: &Path,
    plan: Plan<'_>,
    done: &mut Ingested,
    refusals: &Refusals,
) {
    let Some(venue) = audited_venue(plan) else {
        return;
    };
    let audit = audit_refused(rows, plan, venue, refusals);
    for group in aggregate(&audit.findings, plan.vendor) {
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::error("pull.request_minutes", "request minute coverage incomplete")
                .with("instrument", telemetry::Value::Str(instrument))
                // `feed`, `instrument` and `month` are three of the audit
                // stream's index dimensions (D-3341 item 3).
                .with("feed", telemetry::Value::Str(plan.vendor.as_str()))
                .with("month", telemetry::Value::Str(&group.month))
                .with("code", telemetry::Value::Str(group.why.code()))
                .with("count", telemetry::Value::Uint(u64::from(group.count)))
                .with("first_day", telemetry::Value::Str(&group.first_day))
                .with("last_day", telemetry::Value::Str(&group.last_day))
                .with("reason", telemetry::Value::Str(&group.text)),
        );
        done.failures.push(group.receipt(instrument));
    }
    if audit.days.is_empty() {
        return;
    }
    let key = instrument_key(plan, instrument);
    let run = run_ordinal();
    let lines: Vec<String> = audit
        .days
        .iter()
        .map(|day| gap_line(&key, day, run))
        .collect();
    if let Err(why) = append_ledger(store_root, plan.vendor, &lines) {
        super::note_not_filed(instrument, "gap ledger", &why);
        done.failures.push(failure(
            instrument,
            format!(
                "the minute-gap ledger was not written ({} day line(s)): {why}; the bars and the receipt are unaffected, and every gap above is on the receipt and the log",
                lines.len()
            ),
        ));
    }
}

/// A receipt failure. The one constructor here, so a field added to
/// [`Failure`] is one edit in this file. It is about no month or day until a
/// caller says which (DB-G45, D-3341): [`Group::receipt`] and
/// [`record_derived`] do.
fn failure(instrument: &str, why: String) -> Failure {
    Failure::new(instrument, why)
}

fn audited_venue(plan: Plan<'_>) -> Option<Venue> {
    if plan.request.granularity != Granularity::Minute1
        || plan.contract.is_some()
        || plan.request.listing == crate::vendor::Listing::Derivative
    {
        return None;
    }
    let venue = match (Exchange::parse(plan.exchange), Segment::parse(plan.segment)) {
        (Ok(exchange), Ok(segment)) => Venue::for_segment(exchange, segment),
        _ => None,
    };
    venue.filter(|venue| matches!(venue, Venue::NseIndex | Venue::NseCash))
}

/// The windows one day owes, as `[from, to)` minutes past IST midnight, and
/// how many of them are real.
type Owed = ([(u32, u32); MAX_WINDOWS], usize);

/// What one day of the request owes, or why it owes nothing the audit can
/// check.
enum DayPlan {
    /// The exchange did not trade.
    Closed,
    /// These windows.
    Owed(Owed),
    /// A traded session whose length no record states.
    Unsized,
    /// A day nothing can attest, and why.
    Unverified(&'static str, String),
    /// A day past the end of the calendar of record, and what the calendar
    /// says of it (DB-B25).
    PastRecord(&'static str),
}

/// The session a day owes, by the same calendar the landing and the derive
/// read: the runtime calendar, with a special session's recorded window in
/// place of an unsized answer ([`crate::session::with_recorded_window`],
/// D-3319). Until D-3319 every special session was "exceptional or
/// unmeasured" here and its minutes went unaudited.
fn day_plan(day: Day, number: u32, plan: Plan<'_>, venue: Venue) -> DayPlan {
    let at = i64::from(number);
    match crate::session::with_recorded_window(at, plan.calendar.kind_of(at)) {
        DayKind::Closed => DayPlan::Closed,
        DayKind::Open(session) if session == Session::full() => regular_plan(day, plan, venue),
        DayKind::Open(session) => DayPlan::Owed(special_windows(session)),
        DayKind::OpenLengthUnmeasured => DayPlan::Unsized,
        // PAST THE RECORD'S LAST DAY, NAMED AS THAT (DB-B25). It was the same
        // "calendar authority missing" finding as any other unknown day, once
        // per instrument: 332 identical failures a day when the record ended.
        DayKind::Unmeasured if at > crate::calendar::SOURCED_LAST_DAY => {
            DayPlan::PastRecord(plan.calendar.unverified_reason(at))
        }
        DayKind::Unmeasured => DayPlan::Unverified(
            plan.calendar.unverified_reason(at),
            plan.calendar.unverified_reason(at).to_owned(),
        ),
    }
}

/// A regular day: the venue's dated hours, and for a share on or after
/// 2026-08-03 its dated continuous close.
fn regular_plan(day: Day, plan: Plan<'_>, venue: Venue) -> DayPlan {
    let hours = match venue.hours_on(day) {
        Ok(hours) if hours.kind() == SessionKind::Continuous => hours,
        other => {
            return DayPlan::Unverified("venue hours", format!("venue hours {other:?}"));
        }
    };
    let close = if venue == Venue::NseCash && crate::vendor::cash_auction_eligibility_required(day)
    {
        match plan
            .cash_schedule
            .ok_or_else(|| "dated cash eligibility missing".to_owned())
            .and_then(|schedule| schedule.close(day))
        {
            Ok(close) => u32::from(close),
            Err(why) => return DayPlan::Unverified("dated cash eligibility", why),
        }
    } else {
        hours.close_minute()
    };
    DayPlan::Owed(([(hours.open_minute(), close), (0, 0)], 1))
}

/// A special session's recorded windows, as `[from, to)` minutes.
fn special_windows(session: Session) -> Owed {
    let mut owed = [(0, 0); MAX_WINDOWS];
    let count = usize::from(session.count).min(MAX_WINDOWS);
    for (slot, window) in owed.iter_mut().zip(session.windows).take(count) {
        *slot = (u32::from(window.from), u32::from(window.to) + 1);
    }
    (owed, count)
}

#[cfg(test)]
fn audit_venue(rows: &[RawRow], plan: Plan<'_>, venue: Venue) -> Audit {
    audit_refused(rows, plan, venue, &Refusals::default())
}

/// The audit of `rows` as the STORE holds them: a refused row's minute counts
/// as missing, and a day short by refusals is `refused_day` (or
/// `refused_front`) on the ledger, with no receipt finding of its own — the
/// landing already named each refusal (D-3351).
fn audit_refused(rows: &[RawRow], plan: Plan<'_>, venue: Venue, refusals: &Refusals) -> Audit {
    let mut audit = Audit::default();
    if rows
        .windows(2)
        .any(|pair| matches!(pair, [a, b] if a.timestamp > b.timestamp))
    {
        audit.findings.push(Finding {
            why: Why::Unordered,
            day: None,
            detail: "unordered stamps",
            minutes: 0,
            text: "request minute coverage UNVERIFIED: unordered broker timestamps".to_owned(),
        });
        return audit;
    }
    let mut refused = Cursor::new(&refusals.seconds);
    let mut days_refused = Cursor::new(&refusals.seconds);
    let mut fronts = refusals.fronts.iter().copied().peekable();
    let mut repaired = refusals.repaired.iter().copied().peekable();
    let mut stamps = rows
        .iter()
        .map(|row| local_seconds(row.timestamp, plan.encoding))
        // A REFUSED ROW IS NOT HELD (D-3351): its exact second is dropped
        // before the floor, so a kept row in the same minute still holds it.
        .filter(|&second| !refused.refused(i64::try_from(second).unwrap_or(i64::MAX)))
        // FLOORED TO ITS MINUTE (c4a-4, D-1493). Every comparison below is
        // in whole minutes: `next` steps by 60 from the open, and `missing`
        // counts `(to - from) / 60`. A stamp such as 09:15:59 compared raw
        // opened a "gap" of 0 minutes before it and moved `next` to 09:16:59,
        // so a real missing 09:16 before a 09:17:00 row was reported as 0
        // missing minutes. `ingest::from_window` refuses an off-grid stamp
        // before calling here, so production input is already aligned; this
        // makes the count right without relying on that order of calls.
        .map(|second| second.div_euclid(60) * 60)
        .peekable();
    for number in
        plan.request.window.from().days_from_epoch()..=plan.request.window.to().days_from_epoch()
    {
        let Ok(day) = Day::from_days(number) else {
            audit.findings.push(Finding {
                why: Why::Unverified,
                day: None,
                detail: "invalid day",
                minutes: 0,
                text: format!("request minute coverage UNVERIFIED: invalid day {number}"),
            });
            continue;
        };
        let base = i128::from(number) * 86_400;
        let (windows, count) = match day_plan(day, number, plan, venue) {
            DayPlan::Closed => continue,
            DayPlan::Owed(owed) => owed,
            DayPlan::Unverified(detail, why) => {
                audit.findings.push(unverified_finding(day, detail, &why));
                continue;
            }
            DayPlan::PastRecord(reason) => {
                audit.findings.push(past_record_finding(day, reason));
                continue;
            }
            DayPlan::Unsized => {
                withheld_day(day, base, &mut stamps, &mut audit);
                continue;
            }
        };
        let why = short_why(number, base, &mut fronts, &mut days_refused);
        let findings = &mut audit.findings;
        let mut sink = Vec::new();
        // A REFUSED DAY'S RUNS ARE NOT RECEIPT FINDINGS: the landing named
        // each refusal already, and a "coverage gap" sentence would blame the
        // vendor for minutes it sent. They are counted into the ledger line.
        let found = if why == Why::VendorEmpty {
            findings
        } else {
            &mut sink
        };
        let mut line = DayGaps::new(day, why);
        tally(
            found,
            &mut line,
            windows.get(..count.min(MAX_WINDOWS)).unwrap_or(&[]),
            base,
            &mut stamps,
        );
        // A DAY A REPAIR FILLED IS SAID, short or not (D-3350), so its earlier
        // `refused_*` line is superseded rather than left standing.
        while repaired.next_if(|&day| day < number).is_some() {}
        let filled = repaired.next_if_eq(&number).is_some();
        if line.missing > 0 {
            audit.days.push(line);
        } else if filled {
            line.why = Why::Repaired;
            audit.days.push(line);
        }
    }
    audit
}

/// Counts one day's owed `windows` (`[from, to)` minutes past IST midnight)
/// against `stamps` (IST-local seconds floored to the minute, ascending, from
/// `base`, the day's IST midnight) into `line`: its owed and held minutes and
/// its runs of missing ones, each run also a finding in `found`. The one
/// counter the vendor audit and the stored-month ledger share (R3-18).
fn tally(
    found: &mut Vec<Finding>,
    line: &mut DayGaps,
    windows: &[(u32, u32)],
    base: i128,
    stamps: &mut std::iter::Peekable<impl Iterator<Item = i128>>,
) {
    for &(open, close) in windows {
        line.owed = line.owed.saturating_add(close.saturating_sub(open));
        let mut next = base + i128::from(open) * 60;
        let close = base + i128::from(close) * 60;
        while stamps.peek().is_some_and(|stamp| *stamp < next) {
            stamps.next();
        }
        while let Some(&stamp) = stamps.peek() {
            if stamp >= close {
                break;
            }
            if stamp > next {
                missing(found, line, base, next, stamp);
            }
            next = stamp + 60;
            stamps.next();
        }
        if next < close {
            missing(found, line, base, next, close);
        }
    }
    line.held = line.owed.saturating_sub(line.missing);
}

/// The `gap` line for one day of a STORED month the store holds short, found
/// with no vendor call (R3-18): `stamps` are the UTC micros of the day's held
/// minutes, ascending; `windows` what the day owes
/// (`repair_plan::owed`); `fill` the repair plan's word for what can fill
/// it, written as `fill=` after D-3335's fields with `source=store`. With
/// the line, its missing minutes; `None` when the day holds every owed
/// minute.
pub(super) fn stored_line(
    key: &str,
    day: Day,
    windows: &[(u32, u32)],
    stamps: impl Iterator<Item = i64>,
    fill: &str,
) -> Option<(String, u32)> {
    let base = i128::from(day.days_from_epoch()) * 86_400;
    let mut line = DayGaps::new(day, Why::StoredShort);
    let mut stamps = stamps
        .map(|micros| {
            (i128::from(micros.div_euclid(1_000_000)) + i128::from(IST_OFFSET_SECS)).div_euclid(60)
                * 60
        })
        .peekable();
    tally(&mut Vec::new(), &mut line, windows, base, &mut stamps);
    (line.missing > 0).then(|| {
        (
            format!(
                "{} fill={fill} source=store",
                gap_line(key, &line, run_ordinal())
            ),
            line.missing,
        )
    })
}

/// THE DAY IS WITHHELD AND SAID SO, the rest of the answer is audited
/// (D-3344). Its rows are counted for the ledger: the landing refuses them by
/// the same calendar.
fn withheld_day(
    day: Day,
    base: i128,
    stamps: &mut std::iter::Peekable<impl Iterator<Item = i128>>,
    audit: &mut Audit,
) {
    let mut held = 0_u32;
    while let Some(&stamp) = stamps.peek() {
        if stamp >= base + 86_400 {
            break;
        }
        if stamp >= base {
            held = held.saturating_add(1);
        }
        stamps.next();
    }
    let mut line = DayGaps::new(day, Why::WithheldUnsized);
    line.held = held;
    audit.days.push(line);
    audit.findings.push(Finding {
        why: Why::WithheldUnsized,
        day: Some(day),
        detail: "unsized session",
        minutes: 0,
        text: format!(
            "request minute coverage UNVERIFIED on {day}: a traded session whose length no record states; {held} minute row(s) withheld with the day, the rest of the window audited"
        ),
    });
}

/// WHY A DAY IS SHORT, decided before its runs are counted (D-3351): the
/// store refused it before its month's first held bar, or rows of it were
/// refused, or — the rest — the vendor sent no candle. Asked in day order.
fn short_why(
    number: u32,
    base: i128,
    fronts: &mut std::iter::Peekable<impl Iterator<Item = u32>>,
    days_refused: &mut Cursor<'_>,
) -> Why {
    while fronts.next_if(|&front| front < number).is_some() {}
    let base_second = i64::try_from(base).unwrap_or(i64::MAX);
    if fronts.next_if_eq(&number).is_some() {
        Why::RefusedFront
    } else if days_refused.any_in(base_second, base_second.saturating_add(86_400)) {
        Why::RefusedDay
    } else {
        Why::VendorEmpty
    }
}

fn local_seconds(timestamp: i64, encoding: TimestampEncoding) -> i128 {
    match encoding {
        TimestampEncoding::EpochMillisUtc => {
            i128::from(timestamp).div_euclid(1_000) + i128::from(IST_OFFSET_SECS)
        }
        TimestampEncoding::EpochSecondsUtc | TimestampEncoding::IsoDateTimeOffset => {
            i128::from(timestamp) + i128::from(IST_OFFSET_SECS)
        }
        TimestampEncoding::IstDateTimeText | TimestampEncoding::IsoDateTimeText => {
            i128::from(timestamp)
        }
    }
}

/// One run of missing minutes: a finding for the receipt and the log, and
/// counted into its day's ledger line.
fn missing(findings: &mut Vec<Finding>, line: &mut DayGaps, base: i128, from: i128, to: i128) {
    let first = (from - base) / 60;
    let end = (to - base) / 60;
    let minutes = (to - from) / 60;
    let run = u32::try_from(minutes).unwrap_or(u32::MAX);
    let at = u32::try_from(first).unwrap_or(u32::MAX);
    line.missing = line.missing.saturating_add(run);
    line.runs = line.runs.saturating_add(1);
    line.first_missing = line.first_missing.or(Some(at));
    if run > line.longest {
        line.longest = run;
        line.longest_at = Some(at);
    }
    let day = line.day;
    findings.push(Finding {
        why: Why::VendorEmpty,
        day: Some(day),
        detail: "missing minutes",
        minutes: u64::try_from(minutes).unwrap_or(u64::MAX),
        text: format!(
            "request minute coverage gap on {day}: {:02}:{:02}–{:02}:{:02} IST (end exclusive), {minutes} missing scheduled minutes; no bars fabricated",
            first / 60,
            first % 60,
            end / 60,
            end % 60,
        ),
    });
}

/// One (month, code, detail) group of findings: one receipt failure and one
/// event.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Group {
    why: Why,
    /// `YYYY-MM`, or `-` for a finding about no day.
    month: String,
    /// Findings in the group.
    count: u32,
    /// `YYYY-MM-DD` of the first and last day, or `-`.
    first_day: String,
    last_day: String,
    /// The receipt's sentence.
    text: String,
    /// The month the group is about, as the receipt files it (DB-G45): the
    /// month of its days, [`None`] for findings about no day.
    about_month: Option<store::path::YearMonth>,
    /// The one day the group is about, when all its findings are on one day.
    about_day: Option<Day>,
}

impl Group {
    /// This group's receipt failure, filed under the month and, when it is
    /// about one day, the day its findings concern — never the request
    /// window's first month (DB-G45, D-3341). Consumes the sentence.
    fn receipt(self, instrument: &str) -> Failure {
        let failure = failure(instrument, self.text);
        let failure = match self.about_month {
            Some(month) => failure.in_month(month),
            None => failure,
        };
        match self.about_day {
            Some(day) => failure.on_day(day),
            None => failure,
        }
    }
}

/// The finding for one `day` nothing can attest, with why.
fn unverified_finding(day: Day, detail: &'static str, why: &str) -> Finding {
    Finding {
        why: Why::Unverified,
        day: Some(day),
        detail,
        minutes: 0,
        text: format!("request minute coverage UNVERIFIED on {day}: {why}"),
    }
}

/// The finding for one `day` past the calendar of record (DB-B25).
fn past_record_finding(day: Day, reason: &'static str) -> Finding {
    Finding {
        why: Why::PastRecord,
        day: Some(day),
        detail: reason,
        minutes: 0,
        text: past_record_text(&day.to_string(), reason),
    }
}

/// The sentence of a finding about `days` past the calendar of record. It
/// leads with [`crate::calendar::PAST_RECORD_FINDING`], the words a run
/// recognises these findings by when it folds them into one line (DB-B25).
fn past_record_text(days: &str, reason: &str) -> String {
    let last = Day::from_days(u32::try_from(crate::calendar::SOURCED_LAST_DAY).unwrap_or(0))
        .map_or_else(|_| String::from("its last day"), |day| day.to_string());
    format!(
        "{} (it ends on {last}): {days}: {reason}; bars the vendor sent for it are kept as landed, and no completeness is claimed until the record is extended",
        crate::calendar::PAST_RECORD_FINDING
    )
}

/// One group per (month, code, detail), in first-occurrence order. A group
/// of one finding carries its sentence unchanged; a larger one counts its
/// findings, days and minutes and quotes the first.
///
/// # Cost
///
/// One pass over the findings. The open groups are those of the current
/// month, and a month has at most four codes, so the walk for a group is
/// bounded; findings arrive in day order, so a month's groups are closed
/// when the next month begins.
fn aggregate(findings: &[Finding], vendor: brutex_core::vendor::Vendor) -> Vec<Group> {
    struct Open<'a> {
        why: Why,
        month: Option<store::path::YearMonth>,
        detail: &'static str,
        count: u32,
        days: u32,
        minutes: u64,
        first_day: Option<Day>,
        last_day: Option<Day>,
        first: &'a str,
    }
    let close = |open: Open<'_>| -> Group {
        let month = open
            .month
            .map_or_else(|| NONE.to_string(), |m| m.to_string());
        let first_day = open
            .first_day
            .map_or_else(|| NONE.to_string(), |d| d.to_string());
        let last_day = open
            .last_day
            .map_or_else(|| NONE.to_string(), |d| d.to_string());
        let text = if open.count == 1 {
            open.first.to_owned()
        } else {
            match open.why {
                Why::VendorEmpty => format!(
                    "request minute coverage gap in {month}: {} run(s) of missing scheduled minutes on {} day(s), {first_day}..{last_day}, {} minute(s) in all; each day is a line of audit/{}.gaps (D-3335); no bars fabricated; the first: {}",
                    open.count,
                    open.days,
                    open.minutes,
                    vendor.as_str(),
                    open.first
                ),
                Why::PastRecord => past_record_text(
                    &format!("{} day(s) of {month}, {first_day}..{last_day}", open.days),
                    open.detail,
                ),
                Why::WithheldUnsized
                | Why::Unverified
                | Why::Unordered
                | Why::RefusedDay
                | Why::RefusedFront
                | Why::Repaired
                | Why::StoredShort => format!(
                    "request minute coverage UNVERIFIED in {month}: {} finding(s) on {} day(s), {first_day}..{last_day}; the first: {}",
                    open.count, open.days, open.first
                ),
            }
        };
        Group {
            why: open.why,
            month,
            count: open.count,
            first_day,
            last_day,
            text,
            about_month: open.month,
            about_day: if open.first_day == open.last_day {
                open.first_day
            } else {
                None
            },
        }
    };
    let mut done: Vec<Group> = Vec::new();
    let mut open: Vec<Open<'_>> = Vec::with_capacity(4);
    for finding in findings {
        let month = finding.day.and_then(|day| day.year_month().ok());
        if open.first().is_some_and(|group| group.month != month) {
            done.extend(open.drain(..).map(close));
        }
        let mut grouped = false;
        for group in &mut open {
            if group.why == finding.why && group.detail == finding.detail {
                group.count = group.count.saturating_add(1);
                group.minutes = group.minutes.saturating_add(finding.minutes);
                if finding.day != group.last_day {
                    group.days = group.days.saturating_add(1);
                    group.last_day = finding.day;
                }
                grouped = true;
                break;
            }
        }
        if !grouped {
            open.push(Open {
                why: finding.why,
                month,
                detail: finding.detail,
                count: 1,
                days: 1,
                minutes: finding.minutes,
                first_day: finding.day,
                last_day: finding.day,
                first: &finding.text,
            });
        }
    }
    done.extend(open.drain(..).map(close));
    done
}

/// The census's instrument text, `<EXCHANGE>-<SEGMENT>-<SYMBOL>`, the key
/// every ledger line names (D-3335, the text `/store.json` lists).
fn instrument_key(plan: Plan<'_>, instrument: &str) -> String {
    ledger_key(plan.exchange, plan.segment, instrument)
}

/// A ledger line's instrument key, `<exchange>-<segment>-<symbol>`: the one
/// spelling the pull's lines and the stored-month lines share (R3-18).
pub(super) fn ledger_key(exchange: &str, segment: &str, instrument: &str) -> String {
    format!("{exchange}-{segment}-{instrument}")
}

/// The run every ledger line written now belongs to: the telemetry sink's
/// claimed run, the key `/logs?run=` groups a pull's events by, so a ledger
/// line and the run's events name one run. Zero where no run is claimed or
/// no sink is installed, which is the sink's own word for "no run".
fn run_ordinal() -> u64 {
    telemetry::global().map_or(0, telemetry::Sink::run)
}

/// The ledger's and the group's mark for "none": one hyphen (D-3335's
/// `<HH:MM|->`).
const NONE: char = '-';

/// `HH:MM`, or `-` for none.
fn clock(minute: Option<u32>) -> String {
    minute.map_or_else(
        || NONE.to_string(),
        |minute| format!("{:02}:{:02}", minute / 60, minute % 60),
    )
}

/// One `gap` line, D-3335's fields in D-3335's order.
fn gap_line(key: &str, day: &DayGaps, run: u64) -> String {
    format!(
        "gap {key} {} v=1 rung=1min owed={} held={} missing={} first_missing={} runs={} longest={} longest_at={} why={} run={run}",
        day.day,
        day.owed,
        day.held,
        day.missing,
        clock(day.first_missing),
        day.runs,
        day.longest,
        clock(day.longest_at),
        day.why.code(),
    )
}

/// One `derive` line for a derived rung's month (D-3335, D-3321 item 3).
///
/// `days`, `buckets`, `withheld` and `partial` are the fold's
/// ([`crate::fold::Tally`]): what the rung's month SHOULD hold given its
/// source minutes. The census row says what the file DOES hold, so a reader
/// that finds the census below `buckets` knows the month is behind its
/// source (DB-G12) without folding anything.
pub(super) fn derive_line(parts: &store::path::PathParts<'_>, tally: crate::fold::Tally) -> String {
    let key = match parts.contract {
        Some(contract) => format!(
            "{}-{}-{}-{contract}",
            parts.exchange, parts.segment, parts.symbol
        ),
        None => format!("{}-{}-{}", parts.exchange, parts.segment, parts.symbol),
    };
    format!(
        "derive {key} {} {} v=1 days={} buckets={} withheld={} partial={} run={}",
        parts.timeframe.as_str(),
        parts.month,
        tally.days,
        tally.buckets,
        tally.withheld,
        tally.partial,
        run_ordinal()
    )
}

/// Appends one derived month's `derive` lines (D-3335), or answers the named
/// failure for a ledger that could not be written, logged as `pull.file` "not
/// filed". The derived bars and their census rows are already written either
/// way.
pub(super) fn record_derived(
    store_root: &Path,
    vendor: brutex_core::vendor::Vendor,
    instrument: &str,
    month: store::path::YearMonth,
    lines: &[String],
) -> Option<Failure> {
    let why = append_ledger(store_root, vendor, lines).err()?;
    super::note_not_filed(instrument, "derive ledger", &why);
    Some(
        failure(
            instrument,
            format!(
                "{month}: the derive ledger was not written ({} line(s)): {why}; the derived bars and their census rows are unaffected",
                lines.len()
            ),
        )
        .in_month(month),
    )
}

/// Appends `lines` to `<store>/audit/<vendor>.gaps` in one write, and syncs
/// it.
///
/// # The rules
///
/// D-3317's: one line per record, fields separated by one space, every field
/// after the key `name=value`, the last line of a key wins. A line holding a
/// newline, a carriage return or a tab would break that and is refused, as
/// is the whole batch with it: a key the census writes never holds one.
///
/// # It is a record of runs, not of state
///
/// Every audit that finds something appends its lines, so a re-run that finds
/// the same gap appends the same line again under its own run, the way
/// `audit/pull.journal` records every run. Bars and census are unaffected and
/// stay byte-identical across re-runs (`CLAUDE.md` §3 rule 5); this file is
/// the evidence of what each run was answered. A day whose answer was whole
/// writes no line.
///
/// # Errors
///
/// The host's words for a directory, open, write or sync that failed, or the
/// refusal of a line that would break the line rules.
pub(super) fn append_ledger(
    store_root: &Path,
    vendor: brutex_core::vendor::Vendor,
    lines: &[String],
) -> Result<(), String> {
    append_named(store_root, vendor, "gaps", lines)
}

/// [`append_ledger`] for `<store>/audit/<vendor>.<name>`, under the same line
/// rules: the gap ledger is `gaps`, the repair journal `repairs` (D-3350).
/// Taken under the ledger's exclusive lock ([`append_named_picking`]), so it
/// never interleaves with another writer's batch.
///
/// # Errors
///
/// The host's words for a directory, open, lock, write or sync that failed,
/// or the refusal of a line that would break the line rules.
pub(super) fn append_named(
    store_root: &Path,
    vendor: brutex_core::vendor::Vendor,
    name: &str,
    lines: &[String],
) -> Result<(), String> {
    if lines.is_empty() {
        return Ok(());
    }
    append_named_picking(store_root, vendor, name, |_ledger| {
        Ok(std::borrow::Cow::Borrowed(lines))
    })
    .map(drop)
}

/// Appends to `<store>/audit/<vendor>.<name>` the lines `pick` chooses
/// **while this process holds the ledger's exclusive `flock`**, and answers
/// the ledger's length after the append and whether a torn last line was
/// closed first. R3-18 rework (verify-R3-18-r1).
///
/// # Why the lines are chosen under the lock
///
/// A writer that decides what to write from an earlier read of the ledger
/// (`gap_backfill`: "write no line for a `(key, day)` the ledger already
/// names") is wrong the moment another writer appends between that read and
/// its own append: the last line of a key wins, so a store-found line written
/// after a vendor's line for the same day hides the vendor's reason, and two
/// such writers each append every line. `pick` is handed the locked ledger to
/// read what was added since its earlier read; every writer of the file —
/// [`append_ledger`], [`append_named`] and this — takes the same lock, so
/// nothing can be appended between `pick`'s read and the write.
///
/// # A torn last line is closed, never extended
///
/// A writer killed mid-append can leave a last line with no line break. The
/// next line appended straight after it would be glued onto the fragment and
/// both would read as one malformed line. When the ledger's last byte is not
/// a line break, one is written first, the fragment stays a line of its own
/// (every reader names it as a fault), and a `pull.ledger` warning says so.
///
/// # What the lock does not cover
///
/// `flock` is advisory: a writer that does not take it (a build older than
/// this one) is not held off. Every writer in this tree takes it.
///
/// # Errors
///
/// The host's words for a directory, open, lock, read, write or sync that
/// failed; something other than a regular file at the name; `pick`'s own
/// refusal; or the refusal of a line that would break the line rules. Nothing
/// is written on any of them.
pub(crate) fn append_named_picking<'l>(
    store_root: &Path,
    vendor: brutex_core::vendor::Vendor,
    name: &str,
    pick: impl FnOnce(&mut std::fs::File) -> Result<std::borrow::Cow<'l, [String]>, String>,
) -> Result<(u64, bool), String> {
    use std::os::unix::fs::{FileExt as _, OpenOptionsExt as _};
    let dir = store_root.join("audit");
    std::fs::create_dir_all(&dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let path = dir.join(format!("{}.{name}", vendor.as_str()));
    let named = |why: std::io::Error| format!("{}: {why}", path.display());
    // NEVER THROUGH A LINK, NEVER WAITING ON A PIPE: a link at the name is
    // refused by the open (`O_NOFOLLOW`), a pipe opens at once (`O_NONBLOCK`)
    // and is refused by the handle's own `fstat` below.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .custom_flags(store::open_flags::O_NOFOLLOW_NONBLOCK)
        .open(&path)
        .map_err(|why| {
            format!(
                "{}: {why}; a ledger is appended only as a regular file, never through a link",
                path.display()
            )
        })?;
    if !file.metadata().map_err(named)?.is_file() {
        return Err(format!(
            "{}: not a regular file, so no line is appended to it",
            path.display()
        ));
    }
    let mut ledger = store::flock::Flock::lock(file, path.as_path()).map_err(named)?;
    let lines = pick(&mut ledger)?;
    if lines.is_empty() {
        let end = ledger.metadata().map_err(named)?.len();
        ledger.release().map_err(|why| why.to_string())?;
        return Ok((end, false));
    }
    let mut text =
        String::with_capacity(lines.iter().map(|line| line.len() + 1).sum::<usize>() + 1);
    let len = ledger.metadata().map_err(named)?.len();
    let mut last = [0_u8];
    let torn = match len.checked_sub(1) {
        Some(at) => {
            ledger.read_exact_at(&mut last, at).map_err(named)?;
            last != *b"\n"
        }
        None => false,
    };
    if torn {
        text.push('\n');
    }
    for line in lines.iter() {
        if line.contains(['\n', '\r', '\t']) {
            return Err(format!(
                "a ledger line holds a line break or a tab and was refused with its batch: {line:?}"
            ));
        }
        text.push_str(line);
        text.push('\n');
    }
    ledger
        .write_all(text.as_bytes())
        .and_then(|()| ledger.sync_data())
        .map_err(named)?;
    ledger.release().map_err(|why| why.to_string())?;
    if torn {
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::warn(
                "pull.ledger",
                "a torn last line was closed before the append",
            )
            .with("path", telemetry::Value::Str(&path.display().to_string()))
            .with("at", telemetry::Value::Uint(len)),
        );
    }
    let written = u64::try_from(text.len()).unwrap_or(u64::MAX);
    Ok((len.saturating_add(written), torn))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test-only assertions"
)]
mod tests {
    use super::*;
    use crate::fetch::BarRequest;
    use crate::session::Window;

    fn row(timestamp: i64) -> RawRow {
        RawRow {
            timestamp,
            open: 100,
            high: 100,
            low: 100,
            close: 100,
            volume: 1,
            open_interest: None,
        }
    }

    fn request_on(from: Day, to: Day) -> BarRequest {
        BarRequest {
            instrument_id: String::new(),
            listing: crate::vendor::Listing::Index,
            window: Window::new(from, to).expect("a window"),
            granularity: Granularity::Minute1,
        }
    }

    fn plan(request: &BarRequest) -> Plan<'_> {
        Plan {
            calendar: crate::calendar::Runtime::default(),
            cash_schedule: None,
            columns: crate::csv::Columns::TrueDataIndex,
            request,
            encoding: TimestampEncoding::EpochSecondsUtc,
            scale: crate::vendor::PriceScale::Paisa,
            vendor: brutex_core::vendor::Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            contract: None,
        }
    }

    /// Epoch second of `minute` past IST midnight on `day`.
    fn at(day: Day, minute: i64) -> i64 {
        i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60
    }

    fn texts(audit: &Audit) -> Vec<&str> {
        audit.findings.iter().map(|f| f.text.as_str()).collect()
    }

    /// **A STAMP INSIDE A MINUTE COUNTS AS THAT MINUTE (c4a-4, D-1493).**
    ///
    /// 09:15:59 then 09:17:00 then every minute to the close: one gap, the
    /// 09:16 minute, reported as one missing minute. Before the floor this
    /// produced two "0 missing scheduled minutes" gaps and no correct one.
    #[test]
    fn a_stamp_inside_a_minute_counts_as_that_minute_and_a_gap_counts_right() {
        let day = Day::new(2022, 10, 3).expect("a real day");
        let request = request_on(day, day);
        let plan = plan(&request);
        let open = at(day, 555);
        let mut rows = vec![row(open + 59), row(open + 120)];
        rows.extend((3..375).map(|minute| row(open + minute * 60)));
        let audit = audit_venue(&rows, plan, Venue::NseIndex);
        assert_eq!(audit.findings.len(), 1, "{:?}", texts(&audit));
        let only = &audit.findings[0].text;
        assert!(
            only.contains("09:16–09:17") && only.contains(" 1 missing scheduled minutes"),
            "{only}"
        );
        assert_eq!(audit.days.len(), 1);
        let line = audit.days[0];
        assert_eq!(
            (line.owed, line.held, line.missing, line.runs, line.longest),
            (375, 374, 1, 1, 1)
        );
        assert_eq!(line.first_missing, Some(556));

        // A WHOLE SESSION, EACH STAMP 59 SECONDS LATE: no gap at all.
        let late: Vec<RawRow> = (0..375)
            .map(|minute| row(open + minute * 60 + 59))
            .collect();
        let whole = audit_venue(&late, plan, Venue::NseIndex);
        assert!(
            whole.findings.is_empty() && whole.days.is_empty(),
            "every minute is held"
        );

        // AND THE ALIGNED SESSION IS UNCHANGED.
        let aligned: Vec<RawRow> = (0..375).map(|minute| row(open + minute * 60)).collect();
        assert!(
            audit_venue(&aligned, plan, Venue::NseIndex)
                .findings
                .is_empty()
        );

        // AN EMPTY RESPONSE IS STILL THE WHOLE SESSION MISSING.
        let none = audit_venue(&[], plan, Venue::NseIndex);
        assert_eq!(none.findings.len(), 1);
        assert!(
            none.findings[0]
                .text
                .contains("375 missing scheduled minutes"),
            "{:?}",
            texts(&none)
        );
        assert_eq!(none.days[0].held, 0);
        assert_eq!(none.days[0].missing, 375);
    }

    /// **A MUHURAT IS AUDITED AGAINST ITS RECORDED WINDOW (D-3319, DB-012).**
    ///
    /// A 60-day Zerodha minute answer from 2020-10-17 holds the Muhurat of
    /// 2020-11-14, a Saturday the calendar of record traded 18:15-19:14. The
    /// audit owes every regular session its 375 minutes and the Muhurat its
    /// sixty, names no day UNVERIFIED, and finds nothing on an answer that
    /// holds them all: the request plan around the special session is whole.
    #[test]
    fn a_sixty_day_window_holding_a_muhurat_owes_every_session_and_the_window() {
        let from = Day::new(2020, 10, 17).unwrap();
        let to = Day::new(2020, 12, 15).unwrap();
        let muhurat = Day::new(2020, 11, 14).unwrap();
        let request = request_on(from, to);
        let plan = plan(&request);
        let mut rows = Vec::new();
        let mut sessions = 0;
        for number in from.days_from_epoch()..=to.days_from_epoch() {
            let day = Day::from_days(number).unwrap();
            match day_plan(day, number, plan, Venue::NseIndex) {
                DayPlan::Owed((windows, count)) => {
                    for &(open, close) in windows.iter().take(count) {
                        rows.extend(
                            (i64::from(open)..i64::from(close)).map(|minute| row(at(day, minute))),
                        );
                    }
                    if day == muhurat {
                        assert_eq!(count, 1);
                        assert_eq!(windows[0], (1_095, 1_155), "18:15-19:14");
                    } else {
                        sessions += 1;
                    }
                }
                DayPlan::Closed => {}
                DayPlan::Unsized | DayPlan::Unverified(..) | DayPlan::PastRecord(..) => {
                    panic!("{day} is attested")
                }
            }
        }
        assert_eq!(sessions, 40, "D-3344 measured 41 sessions with the Muhurat");
        let whole = audit_venue(&rows, plan, Venue::NseIndex);
        assert!(whole.findings.is_empty(), "{:?}", texts(&whole));
        assert!(whole.days.is_empty());

        // Without the Muhurat's sixty minutes, that day alone is named, as a
        // gap of its window and nothing else.
        let without: Vec<RawRow> = rows
            .iter()
            .copied()
            .filter(|r| (r.timestamp - at(muhurat, 1_095)).abs() >= 3_600)
            .collect();
        let short = audit_venue(&without, plan, Venue::NseIndex);
        assert_eq!(
            texts(&short),
            [
                "request minute coverage gap on 2020-11-14: 18:15–19:15 IST (end exclusive), 60 missing scheduled minutes; no bars fabricated"
            ]
        );
        assert_eq!(short.days.len(), 1);
        assert_eq!(
            gap_line("NSE-INDEX-NIFTY", &short.days[0], 7),
            "gap NSE-INDEX-NIFTY 2020-11-14 v=1 rung=1min owed=60 held=0 missing=60 first_missing=18:15 runs=1 longest=60 longest_at=18:15 why=vendor_empty run=7"
        );
    }

    /// **A DAY WITH NO RECORDED WINDOW IS WITHHELD BY NAME, AND ONLY IT.**
    /// No such day exists in the calendar today (every Muhurat has its window
    /// since D-3319), so the classification is driven directly: an unsized
    /// day's rows are counted against it and the days around it are audited.
    #[test]
    fn an_unsized_day_is_withheld_and_named_in_the_ledger() {
        let day = Day::new(2020, 11, 14).unwrap();
        let mut line = DayGaps::new(day, Why::WithheldUnsized);
        line.held = 60;
        assert_eq!(
            gap_line("NSE-CASH-ABB", &line, 0),
            "gap NSE-CASH-ABB 2020-11-14 v=1 rung=1min owed=0 held=60 missing=0 first_missing=- runs=0 longest=0 longest_at=- why=withheld_unmeasured_session run=0"
        );
        assert_eq!(Why::VendorEmpty.code(), "vendor_empty");
        assert_eq!(Why::Unverified.code(), "calendar_unverified");
        assert_eq!(Why::PastRecord.code(), "calendar_past_record");
        assert_eq!(Why::Unordered.code(), "unordered_stamps");
    }

    /// **A DISASTER-RECOVERY SATURDAY OWES ITS TWO WINDOWS, NOT THE HALT.**
    /// 2024-03-02 traded 09:15-09:59 and 11:30-12:29 (105 minutes). Rows in
    /// both windows audit clean; a stamp in the halt between them is neither
    /// owed nor a gap, and a missing 11:30 is one run of one minute.
    #[test]
    fn a_two_window_session_is_audited_window_by_window() {
        let day = Day::new(2024, 3, 2).unwrap();
        let request = request_on(day, day);
        let plan = plan(&request);
        let mut rows: Vec<RawRow> = (555..600).map(|m| row(at(day, m))).collect();
        rows.push(row(at(day, 650)));
        rows.extend((690..750).map(|m| row(at(day, m))));
        assert!(
            audit_venue(&rows, plan, Venue::NseIndex)
                .findings
                .is_empty()
        );
        rows.retain(|r| r.timestamp != at(day, 690));
        let short = audit_venue(&rows, plan, Venue::NseIndex);
        assert_eq!(short.findings.len(), 1, "{:?}", texts(&short));
        assert!(short.findings[0].text.contains("11:30–11:31"));
        assert_eq!(
            (
                short.days[0].owed,
                short.days[0].held,
                short.days[0].missing
            ),
            (105, 104, 1)
        );
    }

    /// **ONE FAILURE AND ONE EVENT PER (MONTH, REASON), COUNTED (D-3341).**
    /// Five gaps over two months are two groups: July's one gap keeps its own
    /// sentence, and August's four are one line that counts runs, days and
    /// minutes and quotes the first. Two different UNVERIFIED reasons in one
    /// month never share a count.
    #[test]
    fn findings_aggregate_per_month_and_reason() {
        let finding = |why, day: Day, detail, minutes, text: &str| Finding {
            why,
            day: Some(day),
            detail,
            minutes,
            text: text.to_owned(),
        };
        let july = Day::new(2025, 7, 31).unwrap();
        let aug1 = Day::new(2025, 8, 1).unwrap();
        let aug4 = Day::new(2025, 8, 4).unwrap();
        let findings = vec![
            finding(Why::VendorEmpty, july, "missing minutes", 15, "gap a"),
            finding(Why::VendorEmpty, aug1, "missing minutes", 375, "gap b"),
            finding(Why::VendorEmpty, aug4, "missing minutes", 2, "gap c"),
            finding(Why::Unverified, aug4, "X", 0, "unverified x"),
            finding(Why::VendorEmpty, aug4, "missing minutes", 13, "gap d"),
            finding(Why::Unverified, aug4, "Y", 0, "unverified y"),
        ];
        let groups = aggregate(&findings, brutex_core::vendor::Vendor::Zerodha);
        assert_eq!(groups.len(), 4, "{groups:#?}");
        assert_eq!(groups[0].text, "gap a");
        assert_eq!((groups[0].month.as_str(), groups[0].count), ("2025-07", 1));
        assert_eq!(groups[1].count, 3);
        assert_eq!(
            (groups[1].first_day.as_str(), groups[1].last_day.as_str()),
            ("2025-08-01", "2025-08-04")
        );
        assert!(
            groups[1].text.starts_with(
                "request minute coverage gap in 2025-08: 3 run(s) of missing scheduled minutes on 2 day(s), 2025-08-01..2025-08-04, 390 minute(s) in all; each day is a line of audit/zerodha.gaps"
            ) && groups[1].text.ends_with("the first: gap b"),
            "{}",
            groups[1].text
        );
        assert_eq!(groups[2].text, "unverified x");
        assert_eq!(groups[3].text, "unverified y");
        assert!(aggregate(&[], brutex_core::vendor::Vendor::Zerodha).is_empty());

        // EACH RECEIPT FAILURE IS FILED UNDER ITS OWN MONTH (DB-G45): July's
        // one gap under 2025-07 and its day; August's three gaps over two days
        // under 2025-08 and no single day; the two UNVERIFIED findings on
        // 2025-08-04 under that day. Never the request window's first month.
        let receipts: Vec<Failure> = groups.into_iter().map(|g| g.receipt("NIFTY")).collect();
        let filed: Vec<(Option<String>, Option<String>)> = receipts
            .iter()
            .map(|f| (f.month.map(|m| m.to_string()), f.day.map(|d| d.to_string())))
            .collect();
        assert_eq!(
            filed,
            vec![
                (Some("2025-07".into()), Some("2025-07-31".into())),
                (Some("2025-08".into()), None),
                (Some("2025-08".into()), Some("2025-08-04".into())),
                (Some("2025-08".into()), Some("2025-08-04".into())),
            ]
        );
        assert_eq!(receipts[0].why, "gap a");
        assert_eq!(receipts[0].instrument, "NIFTY");
    }

    /// **THE LEDGER IS APPENDED, ONE LINE PER RECORD, AND REFUSES A BROKEN
    /// LINE WHOLE.**
    #[test]
    fn the_ledger_appends_lines_and_refuses_a_line_break() {
        let root = std::env::temp_dir().join(format!("brutex-LEDGER-UNIT-{}", std::process::id()));
        let _fresh = std::fs::remove_dir_all(&root);
        let vendor = brutex_core::vendor::Vendor::Zerodha;
        append_ledger(&root, vendor, &[]).unwrap();
        assert!(
            !root.join("audit").exists(),
            "nothing to write writes nothing"
        );
        append_ledger(&root, vendor, &["gap A".to_owned(), "gap B".to_owned()]).unwrap();
        append_ledger(&root, vendor, &["derive C".to_owned()]).unwrap();
        let path = root.join("audit").join("zerodha.gaps");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "gap A\ngap B\nderive C\n"
        );
        let refused = append_ledger(&root, vendor, &["OK".to_owned(), "bad\nline".to_owned()]);
        assert!(refused.unwrap_err().contains("line break"));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "gap A\ngap B\nderive C\n",
            "a refused batch writes none of its lines"
        );
        // A store root that is a file cannot hold the ledger, and says so.
        let file_root = root.join("NOT-A-DIR");
        std::fs::write(&file_root, b"x").unwrap();
        assert!(append_ledger(&file_root, vendor, &["gap X".to_owned()]).is_err());
        let _cleanup = std::fs::remove_dir_all(&root);
    }

    /// **EVERY APPEND TAKES THE LEDGER'S LOCK, CLOSES A TORN LAST LINE AND
    /// NEVER WRITES THROUGH A LINK** (R3-18 rework, verify-R3-18-r1).
    #[test]
    fn the_ledger_append_waits_for_the_lock_closes_a_torn_line_and_refuses_a_link() {
        use std::io::Write as _;
        let root = std::env::temp_dir().join(format!("brutex-LEDGER-LOCK-{}", std::process::id()));
        let _fresh = std::fs::remove_dir_all(&root);
        let vendor = brutex_core::vendor::Vendor::Zerodha;
        let audit = root.join("audit");
        std::fs::create_dir_all(&audit).unwrap();
        let path = audit.join("zerodha.gaps");

        // A TORN LAST LINE: closed by one line break, then the batch.
        std::fs::write(&path, "gap TORN 2019-09-2").unwrap();
        append_ledger(&root, vendor, &["gap A".to_owned()]).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "gap TORN 2019-09-2\ngap A\n"
        );
        // A whole last line gets no extra break.
        append_ledger(&root, vendor, &["gap B".to_owned()]).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "gap TORN 2019-09-2\ngap A\ngap B\n"
        );

        // THE LOCK: an append waits while another holder writes.
        let mut holder = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        holder.lock().unwrap();
        std::thread::scope(|scope| {
            let run = scope.spawn(|| append_ledger(&root, vendor, &["gap C".to_owned()]));
            std::thread::sleep(std::time::Duration::from_millis(300));
            assert!(!run.is_finished(), "the append waited for the lock");
            write!(holder, "gap HELD").unwrap();
            holder.unlock().unwrap();
            run.join().unwrap().unwrap();
        });
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "gap TORN 2019-09-2\ngap A\ngap B\ngap HELD\ngap C\n",
            "the holder's unfinished line was closed, never glued to"
        );

        // A LINK AT THE NAME is refused, and its target is untouched.
        let target = root.join("elsewhere");
        std::fs::write(&target, "x\n").unwrap();
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&target, &path).unwrap();
        let refused = append_ledger(&root, vendor, &["gap D".to_owned()]).unwrap_err();
        assert!(refused.contains("never through a link"), "{refused}");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "x\n");
        let _cleanup = std::fs::remove_dir_all(&root);
    }

    /// The derive line names the census key, the rung and the month.
    #[test]
    fn a_derive_line_carries_the_folds_four_counts() {
        let parts = store::path::PathParts {
            vendor: brutex_core::vendor::Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "BANKNIFTY",
            contract: None,
            timeframe: store::path::Timeframe::MINUTE_5,
            month: store::path::YearMonth::new(2024, 3).unwrap(),
            file: store::path::FileKind::Bars,
        };
        let tally = crate::fold::Tally {
            days: 21,
            buckets: 1_596,
            withheld: 2,
            partial: 0,
        };
        assert_eq!(
            derive_line(&parts, tally),
            "derive NSE-INDEX-BANKNIFTY 5min 2024-03 v=1 days=21 buckets=1596 withheld=2 partial=0 run=0"
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]
mod cloud_mutant_tests {
    //! Mutant kills and coverage for the request-minute audit (cloud
    //! mutation round, 2026-10-16). Every stamp and price is invented.

    use super::*;
    use crate::calendar::Runtime;
    use crate::fetch::BarRequest;
    use crate::session::Window;

    fn row(timestamp: i64) -> RawRow {
        RawRow {
            timestamp,
            open: 100,
            high: 100,
            low: 100,
            close: 100,
            volume: 1,
            open_interest: None,
        }
    }

    fn request_on(from: Day, to: Day) -> BarRequest {
        BarRequest {
            instrument_id: String::new(),
            listing: crate::vendor::Listing::Index,
            window: Window::new(from, to).unwrap(),
            granularity: Granularity::Minute1,
        }
    }

    fn plan_at<'a>(request: &'a BarRequest, exchange: &'a str, segment: &'a str) -> Plan<'a> {
        Plan {
            calendar: Runtime::default(),
            cash_schedule: None,
            columns: crate::csv::Columns::TrueDataIndex,
            request,
            encoding: TimestampEncoding::EpochSecondsUtc,
            scale: crate::vendor::PriceScale::Paisa,
            vendor: brutex_core::vendor::Vendor::Zerodha,
            exchange,
            segment,
            contract: None,
        }
    }

    /// Only an NSE index or cash minute request for a spot series is
    /// audited: an exchange or segment the parsers refuse, another venue, a
    /// derivative listing, a contract or a daily request is not.
    #[test]
    fn only_an_nse_spot_minute_request_is_audited() {
        let day = Day::new(2022, 10, 3).unwrap();
        let request = request_on(day, day);
        assert_eq!(
            audited_venue(plan_at(&request, "NSE", "INDEX")),
            Some(Venue::NseIndex)
        );
        assert_eq!(
            audited_venue(plan_at(&request, "NSE", "CASH")),
            Some(Venue::NseCash)
        );
        for (exchange, segment) in [("ZZZ", "INDEX"), ("NSE", "ZZZ"), ("BSE", "INDEX")] {
            assert_eq!(
                audited_venue(plan_at(&request, exchange, segment)),
                None,
                "{exchange} {segment}"
            );
        }
        let daily = BarRequest {
            granularity: Granularity::Day1,
            ..request_on(day, day)
        };
        assert_eq!(audited_venue(plan_at(&daily, "NSE", "INDEX")), None);
        let derivative = BarRequest {
            listing: crate::vendor::Listing::Derivative,
            ..request_on(day, day)
        };
        assert_eq!(audited_venue(plan_at(&derivative, "NSE", "INDEX")), None);
    }

    /// Every stamp encoding lands on the IST-local second: UTC milliseconds
    /// floored to their second then shifted, UTC seconds and offset text
    /// shifted, IST text as written.
    #[test]
    fn every_stamp_encoding_lands_on_the_ist_local_second() {
        let utc = 1_700_000_000_i64;
        let ist = i128::from(utc) + 19_800;
        assert_eq!(
            local_seconds(utc * 1_000 + 999, TimestampEncoding::EpochMillisUtc),
            ist
        );
        assert_eq!(
            local_seconds(-1, TimestampEncoding::EpochMillisUtc),
            i128::from(19_800 - 1),
            "a millisecond before the epoch floors to the second before it"
        );
        assert_eq!(local_seconds(utc, TimestampEncoding::EpochSecondsUtc), ist);
        assert_eq!(
            local_seconds(utc, TimestampEncoding::IsoDateTimeOffset),
            ist
        );
        assert_eq!(
            local_seconds(utc, TimestampEncoding::IstDateTimeText),
            i128::from(utc)
        );
        assert_eq!(
            local_seconds(utc, TimestampEncoding::IsoDateTimeText),
            i128::from(utc)
        );
    }

    /// A withheld day counts its own rows only: a row of the day before is
    /// consumed and not counted, its first second and its last are counted,
    /// and the next day's first row is left for the next day.
    #[test]
    fn a_withheld_day_counts_only_its_own_rows_and_leaves_the_next_days() {
        let day = Day::new(2031, 3, 9).unwrap();
        let base = i128::from(day.days_from_epoch()) * 86_400;
        let stamps = vec![
            base - 60,
            base,
            base + 60,
            base + 86_399,
            base + 86_400,
            base + 90_000,
        ];
        let mut rest = stamps.into_iter().peekable();
        let mut audit = Audit::default();
        withheld_day(day, base, &mut rest, &mut audit);
        assert_eq!(
            rest.next(),
            Some(base + 86_400),
            "the next day's row is left"
        );
        assert_eq!(audit.days.len(), 1);
        let line = audit.days[0];
        assert_eq!(
            (line.day, line.held, line.owed, line.missing, line.runs),
            (day, 3, 0, 0, 0)
        );
        assert_eq!(line.why, Why::WithheldUnsized);
        let [finding] = audit.findings.as_slice() else {
            panic!("{:?}", audit.findings);
        };
        assert_eq!(
            (finding.why, finding.day, finding.detail, finding.minutes),
            (Why::WithheldUnsized, Some(day), "unsized session", 0)
        );
        assert!(
            finding
                .text
                .starts_with("request minute coverage UNVERIFIED on 2031-03-09:")
                && finding
                    .text
                    .contains("3 minute row(s) withheld with the day"),
            "{}",
            finding.text
        );
    }

    /// A traded session whose length no record states (a Muhurat the
    /// circular names and no record has sized) is withheld whole and said,
    /// through the audit: one `withheld_unmeasured_session` line holding
    /// the day's rows, one finding, and no vendor-gap finding. The day is
    /// looked for on the calendar rather than named, so a later record of
    /// its window moves the test to the next such day.
    #[test]
    fn an_unsized_session_is_withheld_whole_through_the_audit() {
        let found =
            (crate::calendar::SOURCED_FIRST_DAY..=crate::calendar::SOURCED_LAST_DAY).find(|&at| {
                crate::session::with_recorded_window(at, Runtime::default().kind_of(at))
                    == DayKind::OpenLengthUnmeasured
            });
        let Some(number) = found else {
            // The calendar sizes every traded day: the arm has no subject,
            // which is what the walk above just proved for every day.
            assert!(found.is_none());
            return;
        };
        let day = Day::from_days(u32::try_from(number).unwrap()).unwrap();
        let request = request_on(day, day);
        let plan = plan_at(&request, "NSE", "INDEX");
        let evening = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + 18 * 3_600;
        let rows = [row(evening), row(evening + 60), row(evening + 120)];
        let audit = audit_venue(&rows, plan, Venue::NseIndex);
        assert_eq!(audit.days.len(), 1);
        assert_eq!(
            (audit.days[0].why, audit.days[0].held),
            (Why::WithheldUnsized, 3)
        );
        assert_eq!(audit.findings.len(), 1, "{:?}", audit.findings);
        assert_eq!(audit.findings[0].why, Why::WithheldUnsized);
        assert_eq!(
            gap_line("NSE-INDEX-NIFTY", &audit.days[0], 4),
            format!(
                "gap NSE-INDEX-NIFTY {day} v=1 rung=1min owed=0 held=3 missing=0 first_missing=- runs=0 longest=0 longest_at=- why=withheld_unmeasured_session run=4"
            )
        );
    }

    /// Two UNVERIFIED findings of one month are one counted sentence naming
    /// both days and quoting the first; its receipt is filed under the
    /// month and, spanning two days, under no single day.
    #[test]
    fn a_month_of_unverified_findings_is_one_counted_sentence() {
        let (d1, d2) = (Day::new(2031, 3, 3).unwrap(), Day::new(2031, 3, 4).unwrap());
        let findings = vec![
            unverified_finding(d1, "venue hours", "First reason"),
            unverified_finding(d2, "venue hours", "Second reason"),
        ];
        let groups = aggregate(&findings, brutex_core::vendor::Vendor::Zerodha);
        let [group] = groups.as_slice() else {
            panic!("{groups:?}");
        };
        let march = store::path::YearMonth::new(2031, 3).unwrap();
        assert_eq!(
            (
                group.why,
                group.count,
                group.month.clone(),
                group.first_day.as_str(),
                group.last_day.as_str()
            ),
            (
                Why::Unverified,
                2,
                march.to_string(),
                "2031-03-03",
                "2031-03-04"
            )
        );
        assert_eq!(
            group.text,
            "request minute coverage UNVERIFIED in 2031-03: 2 finding(s) on 2 day(s), 2031-03-03..2031-03-04; the first: request minute coverage UNVERIFIED on 2031-03-03: First reason"
        );
        assert_eq!(group.about_day, None);
        assert_eq!(
            group.about_month,
            Some(store::path::YearMonth::new(2031, 3).unwrap())
        );
    }

    /// Runs `f` on its own thread and fails the test if it does not answer
    /// within two seconds: a pointer that stops advancing is a failure here,
    /// not a hang.
    fn within<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        let (send, answer) = std::sync::mpsc::channel();
        std::thread::spawn(move || send.send(f()));
        answer
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("answered within two seconds")
    }

    /// `refused` finds a second in any list, asked in time order, each
    /// pointer stepping only past seconds below the one asked; equal seconds
    /// are found again. Kills every mutant of its step and its test.
    #[test]
    fn a_refused_second_is_found_in_any_list_asked_in_order() {
        let answers = within(|| {
            let lists = vec![vec![10, 20, 30], vec![15]];
            let mut cursor = Cursor::new(&lists);
            [5, 10, 10, 15, 16, 20, 29, 30, 31].map(|second| cursor.refused(second))
        });
        assert_eq!(
            answers,
            [false, true, true, true, false, true, false, true, false]
        );
    }

    /// `any_in` answers whether any refused second lies in `[from, to)`:
    /// the end excluded, the start included, across lists. Kills every
    /// mutant of its step and its two bounds.
    #[test]
    fn any_refused_second_in_a_half_open_span_is_found() {
        let answers = within(|| {
            let lists = vec![vec![10, 20], vec![35]];
            let mut cursor = Cursor::new(&lists);
            [
                (0, 10),
                (0, 11),
                (11, 20),
                (20, 21),
                (21, 35),
                (30, 36),
                (36, 99),
            ]
            .map(|(from, to)| cursor.any_in(from, to))
        });
        assert_eq!(answers, [false, true, false, true, false, true, false]);
    }

    fn whole_session(day: Day) -> Vec<RawRow> {
        let open = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + 555 * 60;
        (0..375).map(|minute| row(open + minute * 60)).collect()
    }

    /// A minute refused by name after the vendor sent it is missing from the
    /// store: its day is a `refused_day` ledger line short by it, with no
    /// receipt finding (the landing named the refusal). Kills the refused
    /// second's filter and `short_why`'s day test.
    #[test]
    fn a_minute_refused_by_name_is_a_refused_day_and_no_receipt_finding() {
        let day = Day::new(2022, 10, 3).unwrap();
        let rows = whole_session(day);
        let refused = rows[45].timestamp + IST_OFFSET_SECS;
        let refusals = Refusals {
            seconds: vec![vec![refused]],
            ..Refusals::default()
        };
        let audit = within(move || {
            audit_refused(
                &rows,
                plan_at(&request_on(day, day), "NSE", "INDEX"),
                Venue::NseIndex,
                &refusals,
            )
        });
        assert!(audit.findings.is_empty(), "{:?}", audit.findings);
        let [line] = audit.days.as_slice() else {
            panic!("{:?}", audit.days);
        };
        assert_eq!(
            (
                line.why,
                line.owed,
                line.held,
                line.missing,
                line.first_missing
            ),
            (Why::RefusedDay, 375, 374, 1, Some(600))
        );
    }

    /// A day the store refused before its month's first held bar is a
    /// `refused_front` line; the next day, whole, writes none; a repair that
    /// filled a whole day says so with a `repaired` line. Kills the fronts'
    /// and the repairs' day-order steps.
    #[test]
    fn a_front_day_is_refused_front_and_a_repaired_whole_day_says_repaired() {
        let (d1, d2, d3) = (
            Day::new(2022, 10, 3).unwrap(),
            Day::new(2022, 10, 4).unwrap(),
            Day::new(2022, 10, 6).unwrap(),
        );
        let mut rows = whole_session(d2);
        rows.extend(whole_session(Day::new(2022, 10, 5).unwrap()));
        rows.extend(whole_session(d3));
        let refusals = Refusals {
            seconds: Vec::new(),
            fronts: vec![d1.days_from_epoch() - 9, d1.days_from_epoch()],
            repaired: vec![d1.days_from_epoch() - 2, d3.days_from_epoch()],
        };
        let audit = within(move || {
            let request = request_on(d1, d3);
            audit_refused(
                &rows,
                plan_at(&request, "NSE", "INDEX"),
                Venue::NseIndex,
                &refusals,
            )
        });
        assert!(audit.findings.is_empty(), "{:?}", audit.findings);
        let lines: Vec<(Day, Why, u32, u32)> = audit
            .days
            .iter()
            .map(|line| (line.day, line.why, line.held, line.missing))
            .collect();
        assert_eq!(
            lines,
            [(d1, Why::RefusedFront, 0, 375), (d3, Why::Repaired, 375, 0)]
        );
    }

    /// A day with no candle at all in the middle of a window is that day's
    /// own gap, every one of its 375 minutes, said as the vendor's empty
    /// answer in one finding and one day line; the whole days either side
    /// owe nothing and fabricate nothing.
    #[test]
    fn a_missing_day_in_the_middle_of_a_window_is_its_own_whole_gap() {
        let (d1, d2, d3) = (
            Day::new(2022, 10, 10).unwrap(),
            Day::new(2022, 10, 11).unwrap(),
            Day::new(2022, 10, 12).unwrap(),
        );
        let mut rows = whole_session(d1);
        rows.extend(whole_session(d3));
        let audit = within(move || {
            let request = request_on(d1, d3);
            audit_refused(
                &rows,
                plan_at(&request, "NSE", "INDEX"),
                Venue::NseIndex,
                &Refusals::default(),
            )
        });
        let [finding] = audit.findings.as_slice() else {
            panic!("{:?}", audit.findings);
        };
        assert_eq!(
            (finding.why, finding.day, finding.minutes),
            (Why::VendorEmpty, Some(d2), 375)
        );
        assert_eq!(
            finding.text,
            "request minute coverage gap on 2022-10-11: 09:15–15:30 IST (end exclusive), 375 missing scheduled minutes; no bars fabricated"
        );
        let lines: Vec<(Day, Why, u32, u32)> = audit
            .days
            .iter()
            .map(|line| (line.day, line.why, line.held, line.missing))
            .collect();
        assert_eq!(lines, [(d2, Why::VendorEmpty, 0, 375)]);
    }

    fn at_minute(day: Day, minute: i64) -> i64 {
        i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60
    }

    /// `report` hands an audited answer's gap to the receipt (one failure,
    /// filed under its month and day) and to the ledger (one `gap` line),
    /// and an unaudited answer to neither; a ledger that cannot be written is
    /// a named receipt failure of its own. Kills `report` read as `()`.
    #[test]
    fn report_files_the_receipt_and_the_ledger_line_or_names_the_ledger_it_could_not_write() {
        let day = Day::new(2022, 10, 3).unwrap();
        let request = request_on(day, day);
        let rows: Vec<RawRow> = (5..375)
            .map(|minute| row(at_minute(day, 555 + minute)))
            .collect();
        let root = crate::gdfl_fixtures::scratch("CloudReport");
        let mut done = Ingested::default();
        report(
            &rows,
            "NIFTY",
            &root,
            plan_at(&request, "NSE", "INDEX"),
            &mut done,
            &Refusals::default(),
        );
        let [failure] = done.failures.as_slice() else {
            panic!("{:?}", done.failures);
        };
        assert_eq!(failure.instrument, "NIFTY");
        assert!(
            failure
                .why
                .starts_with("request minute coverage gap on 2022-10-03: 09:15–09:20 IST"),
            "{}",
            failure.why
        );
        assert_eq!(
            (failure.day, failure.month),
            (Some(day), day.year_month().ok())
        );
        let ledger = std::fs::read_to_string(root.join("audit").join("zerodha.gaps")).unwrap();
        let lines: Vec<&str> = ledger.lines().collect();
        assert_eq!(lines.len(), 1, "{ledger}");
        assert!(
            lines[0].starts_with(
                "gap NSE-INDEX-NIFTY 2022-10-03 v=1 rung=1min owed=375 held=370 missing=5 first_missing=09:15 runs=1 longest=5 longest_at=09:15 why=vendor_empty run="
            ),
            "{}",
            lines[0]
        );
        // An answer the audit does not name is neither a failure nor a line.
        let mut untouched = Ingested::default();
        report(
            &rows,
            "NIFTY",
            &root,
            plan_at(&request, "BSE", "INDEX"),
            &mut untouched,
            &Refusals::default(),
        );
        assert!(untouched.failures.is_empty());
        assert_eq!(
            std::fs::read_to_string(root.join("audit").join("zerodha.gaps")).unwrap(),
            ledger
        );
        // A ledger whose directory is a file: the gap is still on the receipt,
        // and the unwritten ledger is a failure of its own.
        let blocked = crate::gdfl_fixtures::scratch("CloudReportBlocked");
        std::fs::write(blocked.join("audit"), b"not a directory").unwrap();
        let mut named = Ingested::default();
        report(
            &rows,
            "NIFTY",
            &blocked,
            plan_at(&request, "NSE", "INDEX"),
            &mut named,
            &Refusals::default(),
        );
        assert_eq!(named.failures.len(), 2, "{:?}", named.failures);
        assert!(
            named.failures[1]
                .why
                .starts_with("the minute-gap ledger was not written (1 day line(s)):"),
            "{}",
            named.failures[1].why
        );
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(blocked).unwrap();
    }

    /// An unmeasured day after the calendar of record ends is a past-record
    /// finding; one before it begins is an UNVERIFIED one. Kills the guard's
    /// comparison mutated, and the guard forced either way.
    #[test]
    fn a_day_past_the_record_and_a_day_before_it_are_named_apart() {
        let after = Day::new(2027, 1, 4).unwrap();
        let before = Day::new(2017, 12, 29).unwrap();
        for (day, why) in [(after, Why::PastRecord), (before, Why::Unverified)] {
            let request = request_on(day, day);
            let audit = audit_venue(&[], plan_at(&request, "NSE", "INDEX"), Venue::NseIndex);
            let [finding] = audit.findings.as_slice() else {
                panic!("{day}: {:?}", audit.findings);
            };
            assert_eq!((finding.why, finding.day), (why, Some(day)), "{day}");
            assert!(audit.days.is_empty(), "{day}");
        }
        // The past-record sentence leads with the words a run folds these
        // findings by, names the record's last day and the day, and says the
        // vendor's bars are kept as landed.
        let request = request_on(after, after);
        let audit = audit_venue(&[], plan_at(&request, "NSE", "INDEX"), Venue::NseIndex);
        let text = &audit.findings[0].text;
        let reason = Runtime::default().unverified_reason(i64::from(after.days_from_epoch()));
        assert_eq!(
            text,
            &format!(
                "{} (it ends on 2026-12-31): 2027-01-04: {reason}; bars the vendor sent for it are kept as landed, and no completeness is claimed until the record is extended",
                crate::calendar::PAST_RECORD_FINDING
            )
        );
    }

    /// Of two missing runs of equal length the FIRST is the day's longest:
    /// a later run must be strictly longer to take its place. Kills the
    /// longest-run test's `>` read as `>=`.
    #[test]
    fn of_two_equal_runs_the_first_is_the_longest() {
        let day = Day::new(2022, 10, 3).unwrap();
        let request = request_on(day, day);
        let rows: Vec<RawRow> = (555..930)
            .filter(|minute| ![600, 601, 700, 701].contains(minute))
            .map(|minute| row(at_minute(day, minute)))
            .collect();
        let audit = audit_venue(&rows, plan_at(&request, "NSE", "INDEX"), Venue::NseIndex);
        let [line] = audit.days.as_slice() else {
            panic!("{:?}", audit.days);
        };
        assert_eq!(
            (
                line.missing,
                line.runs,
                line.longest,
                line.longest_at,
                line.first_missing
            ),
            (4, 2, 2, Some(600), Some(600))
        );
    }

    /// From 2026-08-03 an index still owes 09:15 to 15:30 and needs no cash
    /// schedule; a share eligible for the closing auction owes 09:15 to
    /// 15:15 by its dated flag; a share with no flag is UNVERIFIED. Kills the
    /// cash test's `&&` read as `||` and `==` read as `!=`, and the full
    /// session's guard forced false.
    #[test]
    fn after_the_auction_change_the_index_owes_the_full_day_and_a_share_its_dated_close() {
        let day = Day::new(2026, 8, 4).unwrap();
        let request = request_on(day, day);
        let index: Vec<RawRow> = (555..930)
            .map(|minute| row(at_minute(day, minute)))
            .collect();
        let audit = audit_venue(&index, plan_at(&request, "NSE", "INDEX"), Venue::NseIndex);
        assert!(
            audit.findings.is_empty() && audit.days.is_empty(),
            "{:?}",
            audit.findings
        );
        let mut schedule = crate::cash_auction::Schedule::default();
        schedule.insert(day, true).unwrap();
        let share: Vec<RawRow> = (555..915)
            .map(|minute| row(at_minute(day, minute)))
            .collect();
        let plan = Plan {
            cash_schedule: Some(&schedule),
            ..plan_at(&request, "NSE", "CASH")
        };
        let audit = audit_venue(&share, plan, Venue::NseCash);
        assert!(
            audit.findings.is_empty() && audit.days.is_empty(),
            "{:?}",
            audit.findings
        );
        let short: Vec<RawRow> = (555..914)
            .map(|minute| row(at_minute(day, minute)))
            .collect();
        let audit = audit_venue(&short, plan, Venue::NseCash);
        assert_eq!(audit.days.len(), 1);
        assert_eq!(
            (
                audit.days[0].owed,
                audit.days[0].missing,
                audit.days[0].first_missing
            ),
            (360, 1, Some(914))
        );
        let unflagged = audit_venue(&share, plan_at(&request, "NSE", "CASH"), Venue::NseCash);
        let [finding] = unflagged.findings.as_slice() else {
            panic!("{:?}", unflagged.findings);
        };
        assert_eq!(
            (finding.why, finding.detail),
            (Why::Unverified, "dated cash eligibility")
        );
    }

    /// A derive ledger that writes is no failure and holds the lines; one
    /// that cannot be written is a named failure filed under its month.
    /// Kills `record_derived` read as `None`.
    #[test]
    fn record_derived_names_an_unwritten_ledger_under_its_month() {
        let month = store::path::YearMonth::new(2031, 3).unwrap();
        let lines = vec![
            "derive NSE-INDEX-NIFTY 5min 2031-03 v=1 days=1 buckets=75 withheld=0 partial=0 run=0"
                .to_owned(),
        ];
        let root = crate::gdfl_fixtures::scratch("CloudDerived");
        let vendor = brutex_core::vendor::Vendor::Zerodha;
        assert_eq!(record_derived(&root, vendor, "NIFTY", month, &lines), None);
        assert_eq!(
            std::fs::read_to_string(root.join("audit").join("zerodha.gaps")).unwrap(),
            format!("{}\n", lines[0])
        );
        std::fs::remove_dir_all(root.join("audit")).unwrap();
        std::fs::write(root.join("audit"), b"not a directory").unwrap();
        let failure = record_derived(&root, vendor, "NIFTY", month, &lines).unwrap();
        assert_eq!(
            (failure.instrument.as_str(), failure.month, failure.day),
            ("NIFTY", Some(month), None)
        );
        assert!(
            failure.why.starts_with(&format!(
                "{month}: the derive ledger was not written (1 line(s)):"
            )),
            "{}",
            failure.why
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A pipe at the ledger's name opens at once (`O_NONBLOCK`) and is
    /// refused by the handle's own status, by name: nothing is appended
    /// and the append never waits for a reader. `mkfifo` is the external
    /// program, run from a test only.
    #[test]
    fn a_pipe_at_the_ledger_name_is_refused_and_never_waited_on() {
        let root = crate::gdfl_fixtures::scratch("CloudLedgerPipe");
        let audit = root.join("audit");
        std::fs::create_dir_all(&audit).unwrap();
        let pipe = audit.join("zerodha.gaps");
        let made = std::process::Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "mkfifo made {}", pipe.display());
        let (tx, rx) = std::sync::mpsc::channel();
        let at = root.clone();
        std::thread::spawn(move || {
            let vendor = brutex_core::vendor::Vendor::Zerodha;
            let _gone_when_the_test_ended =
                tx.send(append_ledger(&at, vendor, &["gap A".to_owned()]));
        });
        let why = rx
            .recv_timeout(std::time::Duration::from_mins(1))
            .expect("refused at once, never waited on")
            .unwrap_err();
        assert_eq!(
            why,
            format!(
                "{}: not a regular file, so no line is appended to it",
                pipe.display()
            )
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
