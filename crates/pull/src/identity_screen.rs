//! The landing side of [`crate::series_identity`]: a broker's minute answer
//! judged, row by row, against the day bars the store already holds for the
//! same series. WP-13d; DB-B07, DB-B08, DB-B10, DB-B60, DB-B64.
//!
//! # Where it stands
//!
//! After the grid screen ([`super::screen_candles`]) and before the member is
//! built, so a refused minute never reaches the session filter, the fold, the
//! append or a derived rung. It is refused the way that screen refuses: by
//! name, that row only, one failure and one `pull.file` line per (cause, day),
//! and its second handed to the gap ledger, which then says why the store
//! lacks that minute (D-3351).
//!
//! # No table
//!
//! Nothing here names an instrument, a vendor, a token or a listing date. The
//! day rung of the same key is the only witness: whichever vendor served it,
//! whatever the series is. A series with no day rung is not judged, and says
//! so on `/logs` under `pull.identity`.
//!
//! # Cost
//!
//! Per answer: one open and at most 31 record reads for each month the answer
//! touches; one hash-free pass over the rows, each a binary search over those
//! day bars. Only when a row falls before every day bar of the touched months
//! — or they hold none — one listing of the series' day directory, bounded by
//! the months of its history (141 for 2015-01..2026-09), to find its first
//! held month. Not O(1) in that one case; named for `docs/06-limits.md`.
//! **UNVERIFIED as a measurement**: no bench times the landing door
//! (`CLAUDE.md` §3 rule 6).

use std::collections::BTreeSet;
use std::path::Path;

use store::file::{BarFile, StoreError};
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{Identity, Plan, identify, repair_plan, stamp_day, stamp_local_second};
use crate::fetch::{RawRow, RawWindow, Refused, RowCause, to_paisa};
use crate::series_identity::{Cause, DayBook, FirstDay, Tally, Verdict, audit, ist_day};

/// One stored series judged the way the landing door judges an answer:
/// read-only, and the list a quarantine of its months would work from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeriesScan {
    /// The day months the series holds, ascending.
    pub day_months: Vec<YearMonth>,
    /// The minute months it holds, ascending.
    pub minute_months: Vec<YearMonth>,
    /// The IST day of its first served day bar, when the day rung holds one.
    pub first_day: Option<i64>,
    /// Each minute month's verdicts, ascending by month.
    pub months: Vec<(YearMonth, Tally)>,
    /// Months of either rung that would not open, each with why. Their
    /// minutes are in no count above.
    pub unread: Vec<(YearMonth, String)>,
}

/// Every stored 1min bar of `parts`' series judged against its own stored
/// 1day bars ([`crate::series_identity::audit`]). `parts.timeframe`,
/// `parts.month` and `parts.file` are ignored; every other part names the
/// series. Nothing is written, created or locked beyond a reader's shared
/// lock, and each month is read as every reader opens it (its served
/// generation, D-3350).
///
/// # Cost
///
/// Two directory listings, then one read of every bar of both rungs:
/// O(bars the series holds). A scan, not a read path.
#[must_use]
pub fn scan_series(store_root: &Path, parts: PathParts<'_>) -> SeriesScan {
    let symbol_id = repair_plan::symbol_id(parts.symbol);
    let rung = |timeframe: Timeframe, month: YearMonth| PathParts {
        timeframe,
        month,
        file: FileKind::Bars,
        ..parts
    };
    let mut scan = SeriesScan {
        day_months: repair_plan::held_months(store_root, rung(Timeframe::DAY_1, parts.month)),
        minute_months: repair_plan::held_months(store_root, rung(Timeframe::MINUTE_1, parts.month)),
        ..SeriesScan::default()
    };
    let mut days: Vec<Bar> = Vec::new();
    for month in &scan.day_months {
        match month_days(store_root, rung(Timeframe::DAY_1, *month), symbol_id) {
            Ok(bars) => days.extend(bars),
            Err(why) => scan.unread.push((*month, format!("1day: {why}"))),
        }
    }
    let book = DayBook::new(&days, FirstDay::NoDayRung);
    if let FirstDay::On(first) = book.first() {
        scan.first_day = Some(first);
    }
    for month in &scan.minute_months {
        match month_days(store_root, rung(Timeframe::MINUTE_1, *month), symbol_id) {
            Ok(minutes) => scan.months.push((*month, audit(&minutes, &book))),
            Err(why) => scan.unread.push((*month, format!("1min: {why}"))),
        }
    }
    scan
}

/// The rows of `kept` that are this series on their day, and the rest refused
/// by name. `kept` is what the grid screen kept of `raw`, in `raw`'s order.
///
/// A window that is not intraday is returned as it came: the day rung is the
/// witness, not the accused. So is an answer whose instrument does not parse;
/// the landing refuses that by name itself.
pub(super) fn screen(
    raw: &RawWindow,
    kept: Vec<RawRow>,
    instrument: &str,
    store_root: &Path,
    plan: Plan<'_>,
) -> (Vec<RawRow>, Refused) {
    let intraday = matches!(
        plan.request.granularity.grid(),
        crate::vendor::Grid::Intraday(_)
    );
    if !intraday || kept.is_empty() {
        return (kept, Refused::default());
    }
    let Ok(identity) = identify(instrument, plan.exchange, plan.segment) else {
        return (kept, Refused::default());
    };
    let book = match day_book(&kept, store_root, plan, &identity) {
        Ok(book) => book,
        Err(why) => {
            note(instrument, &Tally::default(), book_first(None), Some(&why));
            return (kept, Refused::default());
        }
    };
    let mut tally = Tally::default();
    let mut refused = Refused::default();
    let mut out = Vec::with_capacity(kept.len());
    // `kept` is a subsequence of `raw.rows`, so one cursor finds each row's
    // position in the answer: O(rows) for the whole pass.
    let mut cursor = 0usize;
    for row in kept {
        while raw
            .rows
            .get(cursor)
            .is_some_and(|at| at.timestamp != row.timestamp)
        {
            cursor += 1;
        }
        let judged = stamp_day(row.timestamp, plan.encoding).and_then(|day| {
            let low = to_paisa(row.low, plan.scale)?;
            let high = to_paisa(row.high, plan.scale)?;
            Some((i64::from(day.days_from_epoch()), low, high))
        });
        // A stamp or a price this build cannot hold is the landing's own
        // refusal, in its own words: not judged here, and not counted.
        let Some((day, low, high)) = judged else {
            out.push(row);
            continue;
        };
        let verdict = book.judge(day, low, high);
        tally.count(verdict, day, row.timestamp);
        let Verdict::Refused(cause) = verdict else {
            out.push(row);
            continue;
        };
        refused.note(
            row_cause(cause),
            (cursor, row.timestamp),
            stamp_local_second(row.timestamp, plan.encoding),
            || sentence(cause, cursor, low, high),
        );
    }
    if tally.refused() + tally.suspect_level + tally.unchecked() > 0 {
        note(instrument, &tally, book.first(), None);
    }
    (out, refused)
}

/// The receipt's cause for a verdict's.
const fn row_cause(cause: Cause) -> RowCause {
    match cause {
        Cause::BeforeFirstDayBar { .. } => RowCause::BeforeFirstDayBar,
        Cause::ForeignLevel { .. } => RowCause::ForeignLevel,
    }
}

/// The first refused row of a (cause, day), in words a receipt can carry.
fn sentence(cause: Cause, row: usize, low: i64, high: i64) -> String {
    match cause {
        Cause::BeforeFirstDayBar { first } => format!(
            "row {row}: a minute on a day before the first day bar this series was served \
             (IST day {first}): not listed yet, or the day pass has not reached it; pull the \
             day rung over that span first. Minute refused before fold or append (WP-13d)"
        ),
        Cause::ForeignLevel { day_low, day_high } => format!(
            "row {row}: a minute ranging {low}..{high} paisa lies wholly outside its own \
             served day's range {day_low}..{day_high} by more than {} bps: another \
             instrument's bars or another price basis under this key. Minute refused before \
             fold or append (WP-13d)",
            crate::series_identity::FOREIGN_BAND_BPS
        ),
    }
}

/// `first`, or the day rung's absence, for [`note`].
const fn book_first(first: Option<i64>) -> FirstDay {
    match first {
        Some(day) => FirstDay::On(day),
        None => FirstDay::NoDayRung,
    }
}

/// The one `pull.identity` line of an answer that was not wholly held: `warn`
/// when a minute was refused or is suspect, or the day rung would not open;
/// `info` when minutes were only left unjudged.
fn note(instrument: &str, tally: &Tally, first: FirstDay, unread: Option<&str>) {
    let loud = unread.is_some() || tally.refused() + tally.suspect_level > 0;
    let message = if unread.is_some() {
        "minutes not judged against their served day: the day rung would not open"
    } else if tally.refused() > 0 {
        "minutes refused: not this series on their own served day"
    } else if tally.suspect_level > 0 {
        "minutes landed outside their own served day's range: suspect, not refused"
    } else {
        "minutes landed unjudged: the day rung holds no bar for their day"
    };
    let level = if loud {
        telemetry::Level::Warn
    } else {
        telemetry::Level::Info
    };
    let first_day = match first {
        FirstDay::On(day) => day,
        FirstDay::NoDayRung => -1,
    };
    let count = telemetry::Value::Uint;
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(level, "pull.identity", message)
            .with("instrument", telemetry::Value::Str(instrument))
            .with("held", count(tally.held))
            .with("before_first_day_bar", count(tally.before_first_day_bar))
            .with("foreign_level", count(tally.foreign_level))
            .with("suspect_level", count(tally.suspect_level))
            .with("no_day_rung", count(tally.no_day_rung))
            .with("no_day_bar", count(tally.no_day_bar))
            .with("day_bar_unusable", count(tally.day_bar_unusable))
            .with("first_day", telemetry::Value::Int(first_day))
            .with("why", telemetry::Value::Str(unread.unwrap_or(""))),
    );
}

/// The day bars of every month `rows` touch, and the series' first served day
/// when a row falls before all of them.
fn day_book(
    rows: &[RawRow],
    store_root: &Path,
    plan: Plan<'_>,
    identity: &Identity,
) -> Result<DayBook, String> {
    let mut months: BTreeSet<YearMonth> = BTreeSet::new();
    let mut earliest: Option<i64> = None;
    for row in rows {
        let Some(day) = stamp_day(row.timestamp, plan.encoding) else {
            continue;
        };
        let at = i64::from(day.days_from_epoch());
        earliest = Some(earliest.map_or(at, |held: i64| held.min(at)));
        if let Ok(month) = day.year_month() {
            months.insert(month);
        }
    }
    let parts = |month: YearMonth| PathParts {
        vendor: plan.vendor,
        exchange: identity.exchange.as_str(),
        segment: identity.segment.as_str(),
        symbol: identity.symbol.as_str(),
        contract: plan.contract,
        timeframe: Timeframe::DAY_1,
        month,
        file: FileKind::Bars,
    };
    let mut days: Vec<Bar> = Vec::new();
    for month in &months {
        days.extend(month_days(store_root, parts(*month), identity.symbol_id)?);
    }
    let book = DayBook::new(&days, FirstDay::NoDayRung);
    let before_all = match (book.first(), earliest) {
        (FirstDay::NoDayRung, _) => true,
        (FirstDay::On(first), Some(row)) => row < first,
        (FirstDay::On(_), None) => false,
    };
    let Some(any) = months.first().copied().filter(|_| before_all) else {
        return Ok(book);
    };
    // The one listing: the series' first held day month, wherever it is.
    let Some(first_month) = repair_plan::first_held_month(store_root, parts(any)) else {
        return Ok(book);
    };
    let first = month_days(store_root, parts(first_month), identity.symbol_id)?
        .first()
        .map_or(FirstDay::NoDayRung, |bar| {
            FirstDay::On(ist_day(bar.ts_micros))
        });
    Ok(DayBook::new(&days, first))
}

/// One month of a rung, as every reader opens it; nothing when the month is
/// not there.
fn month_days(store_root: &Path, parts: PathParts<'_>, symbol_id: u32) -> Result<Vec<Bar>, String> {
    let path = StorePath::new(parts).map_err(|why| why.to_string())?;
    let file = match BarFile::open_existing(store_root, path, symbol_id) {
        Ok(file) => file,
        Err(StoreError::Missing { .. }) => return Ok(Vec::new()),
        Err(why) => return Err(why.to_string()),
    };
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).map_err(|why| why.to_string()))
        .collect()
}
