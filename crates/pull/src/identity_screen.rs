//! The landing side of [`crate::series_identity`]: a broker's answer judged,
//! row by row, against what the store already holds for the same series.
//! WP-13d; DB-B07, DB-B08, DB-B10, DB-B60, DB-B64. R4-17: RT-DATA-02, ING-01,
//! Z5, Z6.
//!
//! # Where it stands
//!
//! After the grid screen ([`super::screen_candles`]) and before the member is
//! built, so a refused row never reaches the session filter, the fold, the
//! append or a derived rung. It is refused the way that screen refuses: by
//! name, that row only, one failure and one `pull.file` line per (cause, day),
//! and its second handed to the gap ledger, which then says why the store
//! lacks it (D-3351).
//!
//! # Its witnesses
//!
//! For a minute, the day rung of the same key, whichever vendor served it; for
//! a CASH share, besides, the listing day the receipted NSE security master
//! states for its exact symbol and ISIN ([`crate::listing`], D-4990) and the
//! minute kept before it in the same continuous run
//! ([`crate::series_identity::JumpRun`]). A day bar has one witness only, the
//! master: the day rung is never judged by range (D-4992). Nothing here names
//! an instrument, a vendor or a token.
//!
//! # Cost
//!
//! Per answer: one open and at most 31 record reads for each month a minute
//! answer touches; one hash-free pass over the rows, each a binary search over
//! those day bars and a fixed number of compares; for a CASH share, one
//! expected-O(1) listing probe ([`crate::listing::Listing::span`]). Only when a
//! minute falls before every day bar of the touched months — or they hold none
//! — one listing of the series' day directory, bounded by the months of its
//! history (141 for 2015-01..2026-09), to find its first held month. Not O(1)
//! in that one case; named for `docs/06-limits.md`. The listing probe is timed
//! by `crates/pull/benches/ratio/identity.rs`.

use std::collections::BTreeSet;
use std::path::Path;

use store::file::{BarFile, StoreError};
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{Identity, Plan, identify, repair_plan, stamp_day, stamp_local_second};
use crate::fetch::{RawRow, RawWindow, Refused, RowCause, to_paisa};
use crate::series_identity::{
    self, Cause, DayBook, FirstDay, MinuteJudge, Rules, Tally, Verdict, ist_day, prices,
};

/// One stored bar a landing would refuse, or a suspect minute it would land
/// and name: the rows a quarantine plan is made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlaggedBar {
    /// Its rung.
    pub timeframe: Timeframe,
    /// Its month.
    pub month: YearMonth,
    /// The bar as stored.
    pub bar: Bar,
    /// [`Cause::code`], or [`series_identity::SUSPECT_CODE`].
    pub code: &'static str,
    /// Whether a landing refuses it: false for a suspect minute.
    pub refused: bool,
}

/// One stored series judged the way the landing door judges an answer:
/// read-only, and the list a quarantine of its months would work from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesScan {
    /// The day months the series holds, ascending.
    pub day_months: Vec<YearMonth>,
    /// The minute months it holds, ascending.
    pub minute_months: Vec<YearMonth>,
    /// The IST day of its first served day bar, when the day rung holds one.
    pub first_day: Option<i64>,
    /// Where its listing came from: [`crate::listing::Source::code`], or
    /// `no_listing` for a series no listing applies to.
    pub listing: &'static str,
    /// The master's listing day, as IST days since the epoch, when a refusal
    /// rests on it.
    pub listed_on: Option<i64>,
    /// Each minute month's verdicts, ascending by month.
    pub months: Vec<(YearMonth, Tally)>,
    /// Each day month's verdicts under the day-rung arm, ascending by month.
    pub day_verdicts: Vec<(YearMonth, Tally)>,
    /// Every bar a landing would refuse and every suspect minute, day bars
    /// first, each rung ascending.
    pub flagged: Vec<FlaggedBar>,
    /// Every judged in-session jump of a CASH share, in basis points of the
    /// smaller price: the distribution [`series_identity::JUMP_BPS`] is
    /// measured against. Empty for every other series.
    pub jumps: Vec<i64>,
    /// Months of either rung that would not open, each with why. Their
    /// bars are in no count above.
    pub unread: Vec<(YearMonth, String)>,
}

/// Every stored bar of `parts`' series judged as the landing door judges an
/// answer: each 1day bar by the day-rung arm, each 1min bar against the
/// series' own stored 1day bars and, for a CASH share, its listing and its
/// run ([`series_identity::MinuteJudge`]). `parts.timeframe`, `parts.month`
/// and `parts.file` are ignored; every other part names the series. Nothing is
/// written, created or locked beyond a reader's shared lock, and each month is
/// read as every reader opens it (its served generation, D-3350).
///
/// # Cost
///
/// Two directory listings and one listing-table load per store, then one read
/// of every bar of both rungs: O(bars the series holds). A scan, not a read
/// path.
#[must_use]
pub fn scan_series(store_root: &Path, parts: PathParts<'_>) -> SeriesScan {
    let rung = |timeframe: Timeframe, month: YearMonth| PathParts {
        timeframe,
        month,
        file: FileKind::Bars,
        ..parts
    };
    let cash = parts.segment == "CASH" && parts.contract.is_none();
    let table = cash.then(|| crate::listing::for_store(store_root));
    let isin = brutex_core::universe::nse_isin(parts.symbol);
    let held = table.as_deref().and_then(|listing| {
        listing.span(
            parts.exchange,
            parts.symbol,
            isin.as_ref().map(brutex_core::isin::Isin::as_str),
        )
    });
    let listed_on = held
        .and_then(|found| found.listed_on())
        .map(|day| i64::from(day.days_from_epoch()));
    let mut series = SeriesScan {
        day_months: repair_plan::held_months(store_root, rung(Timeframe::DAY_1, parts.month)),
        minute_months: repair_plan::held_months(store_root, rung(Timeframe::MINUTE_1, parts.month)),
        first_day: None,
        listing: match (held, cash) {
            (Some(found), _) => found.source.code(),
            (None, true) => crate::listing::Source::InferredFromVendorSpan.code(),
            (None, false) => NO_LISTING,
        },
        listed_on,
        months: Vec::new(),
        day_verdicts: Vec::new(),
        flagged: Vec::new(),
        jumps: Vec::new(),
        unread: Vec::new(),
    };
    let rules = Rules {
        listed_on,
        jumps: cash,
        zero_held: parts.segment == "INDEX" && parts.contract.is_none(),
    };
    let symbol_id = repair_plan::symbol_id(parts.symbol);
    let days = scan_days(
        &mut series,
        &|month| month_days(store_root, rung(Timeframe::DAY_1, month), symbol_id),
        rules,
    );
    let book = DayBook::new(&days, FirstDay::NoDayRung);
    if let FirstDay::On(first) = book.first() {
        series.first_day = Some(first);
    }
    let mut judge = MinuteJudge::new(&book, rules, 60);
    for month in series.minute_months.clone() {
        match month_days(store_root, rung(Timeframe::MINUTE_1, month), symbol_id) {
            Ok(minutes) => scan_minutes(&mut series, &mut judge, (month, rules), &minutes),
            Err(why) => series.unread.push((month, format!("1min: {why}"))),
        }
    }
    series
}

/// Every held day month judged by the day-rung arm; answers the day bars
/// read, for the book the minutes are judged against.
fn scan_days(
    series: &mut SeriesScan,
    read: &dyn Fn(YearMonth) -> Result<Vec<Bar>, String>,
    rules: Rules,
) -> Vec<Bar> {
    let mut days: Vec<Bar> = Vec::new();
    for month in series.day_months.clone() {
        let bars = match read(month) {
            Ok(bars) => bars,
            Err(why) => {
                series.unread.push((month, format!("1day: {why}")));
                continue;
            }
        };
        let mut tally = Tally::default();
        for bar in &bars {
            let day = ist_day(bar.ts_micros);
            let cause = series_identity::judge_day(day, prices(bar), rules);
            tally.count(
                cause.map_or(Verdict::Held, Verdict::Refused),
                day,
                bar.ts_micros,
            );
            if let Some(cause) = cause {
                series.flagged.push(FlaggedBar {
                    timeframe: Timeframe::DAY_1,
                    month,
                    bar: *bar,
                    code: cause.code(),
                    refused: true,
                });
            } else if held_zero(bar, rules) {
                series
                    .flagged
                    .push(held_zero_flag(Timeframe::DAY_1, month, bar));
            }
        }
        series.day_verdicts.push((month, tally));
        days.extend(bars);
    }
    days
}

/// Whether `bar` holds a zero an index's rules hold rather than refuse
/// ([`Rules::zero_held`]): landed, so a scan names it with a label.
fn held_zero(bar: &Bar, rules: Rules) -> bool {
    rules.zero_held && series_identity::non_positive(bar.open, bar.high, bar.low, bar.close)
}

/// The labelled flag of a held index zero: the plan names it, nothing
/// refuses it.
const fn held_zero_flag(timeframe: Timeframe, month: YearMonth, bar: &Bar) -> FlaggedBar {
    FlaggedBar {
        timeframe,
        month,
        bar: *bar,
        code: Cause::NonPositivePrice.code(),
        refused: false,
    }
}

/// One minute month judged in time order by `judge`, whose run carries over
/// from the month before.
fn scan_minutes(
    series: &mut SeriesScan,
    judge: &mut MinuteJudge<'_>,
    (month, rules): (YearMonth, Rules),
    minutes: &[Bar],
) {
    let mut tally = Tally::default();
    for bar in minutes {
        let day = ist_day(bar.ts_micros);
        let (verdict, jump) = judge.judge(bar.ts_micros.div_euclid(1_000_000), day, prices(bar));
        series.jumps.extend(jump);
        tally.count(verdict, day, bar.ts_micros);
        let code = match verdict {
            Verdict::Refused(cause) => Some((cause.code(), true)),
            Verdict::Suspect { .. } => Some((series_identity::SUSPECT_CODE, false)),
            Verdict::Held | Verdict::Unchecked(_) if held_zero(bar, rules) => {
                Some((Cause::NonPositivePrice.code(), false))
            }
            Verdict::Held | Verdict::Unchecked(_) => None,
        };
        if let Some((code, refused)) = code {
            series.flagged.push(FlaggedBar {
                timeframe: Timeframe::MINUTE_1,
                month,
                bar: *bar,
                code,
                refused,
            });
        }
    }
    series.months.push((month, tally));
}

/// The rows of `kept` that are this series on their day, and the rest refused
/// by name. `kept` is what the grid screen kept of `raw`, in `raw`'s order.
///
/// # Two arms (R4-17)
///
/// An INTRADAY window is judged row by row: a price at or below zero, a CASH
/// minute before its master's listing day, a minute that is not this series on
/// its own served day (WP-13d), and a CASH minute that jumps from the minute
/// kept before it inside one continuous run ([`series_identity::JUMP_BPS`]).
/// When the day rung of a touched month will not open, only the range rule
/// is skipped (every range verdict `no_day_rung`, one warn naming why); the
/// other three need no day bar and still refuse by name (ZI-11).
///
/// Any other window — the day rung — is judged ONLY for a price at or below
/// zero and for a CASH day before its listing, never by range: the day rung is
/// the witness the minute ranges are judged against, not the accused (D-4992).
/// An answer whose instrument does not parse is returned as it came; the
/// landing refuses that by name itself.
pub(super) fn screen(
    raw: &RawWindow,
    kept: Vec<RawRow>,
    instrument: &str,
    store_root: &Path,
    plan: Plan<'_>,
) -> (Vec<RawRow>, Refused) {
    if kept.is_empty() {
        return (kept, Refused::default());
    }
    let Ok(identity) = identify(instrument, plan.exchange, plan.segment) else {
        return (kept, Refused::default());
    };
    let table = cash_listing(store_root, &identity, plan);
    let span = table
        .as_deref()
        .and_then(|listing| span_of(listing, &identity));
    let listed_on = span
        .and_then(|span| span.listed_on())
        .map(|day| i64::from(day.days_from_epoch()));
    let witness = Witness {
        instrument,
        plan,
        listing: match (span, table.is_some()) {
            (Some(span), _) => span.source.code(),
            // A SHARE IN NO MASTER ON DISK (DHFL, JETAIRWAYS): its span is the
            // vendor's own, labelled, and refuses nothing (D-4990).
            (None, true) => crate::listing::Source::InferredFromVendorSpan.code(),
            (None, false) => NO_LISTING,
        },
    };
    // AN INDEX'S ZERO IS ITS OWN (NIFTY50DIVPOINT restarts from zero, D-4992).
    let zero_held = identity.segment.as_str() == "INDEX" && plan.contract.is_none();
    match plan.request.granularity.grid() {
        crate::vendor::Grid::Intraday(step) => {
            let rules = Rules {
                listed_on,
                // CASH MINUTES ONLY: the population JUMP_BPS is measured over.
                jumps: table.is_some() && step == 60,
                zero_held,
            };
            minute_arm(
                raw,
                kept,
                &witness,
                (store_root, &identity),
                rules,
                i64::from(step),
            )
        }
        _ => day_arm(
            raw,
            kept,
            &witness,
            Rules {
                listed_on,
                jumps: false,
                zero_held,
            },
        ),
    }
}

/// What every refusal of one answer names beside itself.
struct Witness<'a> {
    instrument: &'a str,
    plan: Plan<'a>,
    /// [`crate::listing::Source::code`] of the span in force, or [`NO_LISTING`].
    listing: &'static str,
}

/// The listing label of a series no listing applies to, or none was found for.
const NO_LISTING: &str = "no_listing";

/// The listing table of the store, for a CASH share and nothing else.
fn cash_listing(
    store_root: &Path,
    identity: &Identity,
    plan: Plan<'_>,
) -> Option<std::sync::Arc<crate::listing::Listing>> {
    (identity.segment.as_str() == "CASH" && plan.contract.is_none())
        .then(|| crate::listing::for_store(store_root))
}

/// The share's span, by its exact NSE ISIN.
fn span_of<'a>(
    listing: &'a crate::listing::Listing,
    identity: &Identity,
) -> Option<crate::listing::Span<'a>> {
    let symbol = identity.symbol.as_str();
    let isin = brutex_core::universe::nse_isin(symbol);
    listing.span(
        identity.exchange.as_str(),
        symbol,
        isin.as_ref().map(brutex_core::isin::Isin::as_str),
    )
}

/// One kept row's IST day and its four prices in paisa, or `None` when the
/// stamp or a price cannot be held: the landing's own refusal, in its own
/// words, not judged here and not counted.
fn held(row: &RawRow, plan: Plan<'_>) -> Option<(i64, [i64; 4])> {
    let day = stamp_day(row.timestamp, plan.encoding)?;
    let paisa = |raw: i64| to_paisa(raw, plan.scale);
    Some((
        i64::from(day.days_from_epoch()),
        [
            paisa(row.open)?,
            paisa(row.high)?,
            paisa(row.low)?,
            paisa(row.close)?,
        ],
    ))
}

/// Whether a price at or below zero is refused for this plan: every index,
/// share and future, never an option (an option's zero is not this rule's).
fn prices_judged(plan: Plan<'_>) -> bool {
    !plan.contract.is_some_and(|contract| contract.is_option())
}

/// The day-rung arm: a non-positive price and a day before the listing, and
/// nothing else (D-4992).
fn day_arm(
    raw: &RawWindow,
    kept: Vec<RawRow>,
    witness: &Witness<'_>,
    rules: Rules,
) -> (Vec<RawRow>, Refused) {
    let plan = witness.plan;
    let judged = prices_judged(plan);
    let mut tally = Tally::default();
    let mut refused = Refused::default();
    let mut out = Vec::with_capacity(kept.len());
    let mut cursor = 0usize;
    for row in kept {
        cursor = position(raw, cursor, &row);
        let Some((day, ohlc)) = held(&row, plan) else {
            out.push(row);
            continue;
        };
        let cause = series_identity::judge_day(day, ohlc, rules)
            .filter(|cause| judged || !matches!(cause, Cause::NonPositivePrice));
        let Some(cause) = cause else {
            tally.count(Verdict::Held, day, row.timestamp);
            out.push(row);
            continue;
        };
        tally.count(Verdict::Refused(cause), day, row.timestamp);
        refused.note(
            row_cause(cause),
            (cursor, row.timestamp),
            stamp_local_second(row.timestamp, plan.encoding),
            || sentence(cause, cursor, ohlc, witness.listing),
        );
    }
    if tally.refused() > 0 {
        note(witness, &tally, FirstDay::NoDayRung, None);
    }
    (out, refused)
}

/// The position of `row` in the answer, from `cursor` on: `kept` is a
/// subsequence of `raw.rows`, so one cursor finds every row in O(rows) for the
/// whole pass.
fn position(raw: &RawWindow, mut cursor: usize, row: &RawRow) -> usize {
    while raw
        .rows
        .get(cursor)
        .is_some_and(|at| at.timestamp != row.timestamp)
    {
        cursor += 1;
    }
    cursor
}

/// The minute arm: every row judged by [`MinuteJudge`] against the served
/// day bars of the months the answer touches.
fn minute_arm(
    raw: &RawWindow,
    kept: Vec<RawRow>,
    witness: &Witness<'_>,
    (store_root, identity): (&Path, &Identity),
    rules: Rules,
    step: i64,
) -> (Vec<RawRow>, Refused) {
    let plan = witness.plan;
    // A DAY RUNG THAT WILL NOT OPEN SKIPS THE RANGE RULES ONLY (R4-17 repair
    // 1): every minute is judged against a book with no day bar, so each range
    // verdict is `no_day_rung` and nothing is refused BY RANGE, while a price
    // at or below zero, a CASH minute before its listing and a CASH
    // in-session jump — none of which reads the day rung — are still refused
    // by name. The warn names why the rung would not open.
    let (book, unread) = match day_book(&kept, store_root, plan, identity) {
        Ok(book) => (book, None),
        Err(why) => (DayBook::new(&[], FirstDay::NoDayRung), Some(why)),
    };
    let judged = prices_judged(plan);
    let mut judge = MinuteJudge::new(&book, rules, step);
    let mut tally = Tally::default();
    let mut refused = Refused::default();
    let mut out = Vec::with_capacity(kept.len());
    let mut cursor = 0usize;
    for row in kept {
        cursor = position(raw, cursor, &row);
        let Some((day, ohlc)) = held(&row, plan) else {
            out.push(row);
            continue;
        };
        let second = stamp_local_second(row.timestamp, plan.encoding);
        let verdict = match judge.judge(second, day, ohlc) {
            // AN OPTION'S ZERO IS NOT THIS RULE'S: judged by range as before.
            (Verdict::Refused(Cause::NonPositivePrice), _) if !judged => {
                book.judge(day, ohlc[2], ohlc[1])
            }
            (verdict, _) => verdict,
        };
        tally.count(verdict, day, row.timestamp);
        let Verdict::Refused(cause) = verdict else {
            out.push(row);
            continue;
        };
        refused.note(row_cause(cause), (cursor, row.timestamp), second, || {
            sentence(cause, cursor, ohlc, witness.listing)
        });
    }
    if unread.is_some() || tally.refused() + tally.suspect_level + tally.unchecked() > 0 {
        note(witness, &tally, book.first(), unread.as_deref());
    }
    (out, refused)
}

/// The receipt's cause for a verdict's.
const fn row_cause(cause: Cause) -> RowCause {
    match cause {
        Cause::BeforeFirstDayBar { .. } => RowCause::BeforeFirstDayBar,
        Cause::ForeignLevel { .. } => RowCause::ForeignLevel,
        Cause::NonPositivePrice => RowCause::NonPositivePrice,
        Cause::BeforeListing { .. } => RowCause::BeforeListing,
        Cause::InSessionJump { .. } => RowCause::InSessionJump,
    }
}

/// The first refused row of a (cause, day), in words a receipt can carry.
fn sentence(cause: Cause, row: usize, [open, high, low, close]: [i64; 4], listing: &str) -> String {
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
        Cause::NonPositivePrice => format!(
            "row {row}: open {open}, high {high}, low {low}, close {close} paisa: a price at or \
             below zero is no price. Bar refused before fold or append (R4-17, ING-01)"
        ),
        Cause::BeforeListing { listed_on } => format!(
            "row {row}: a bar on a day before this security's listing day (IST day \
             {listed_on}) in the NSE security master ({listing}): not this security's bar. \
             Bar refused before fold or append (R4-17, D-4990)"
        ),
        Cause::InSessionJump { reference, bps } => format!(
            "row {row}: an open of {open} paisa after a close of {reference} inside one \
             continuous run is a jump of {bps} bps of the smaller, beyond {} bps: another \
             price basis interleaved under this key. Minute refused before fold or append \
             (R4-17, RT-DATA-02)",
            crate::series_identity::JUMP_BPS
        ),
    }
}

/// The `pull.identity` lines of an answer that was not wholly held: one
/// `warn` when the day rung would not open, naming why; one `warn` per
/// (cause, day) refused or suspect, each naming its count — after that warn
/// too, since a rung that would not open stops only the range rules; one
/// `info` when minutes were only left unjudged with every day rung read.
fn note(witness: &Witness<'_>, tally: &Tally, first: FirstDay, unread: Option<&str>) {
    let first_day = match first {
        FirstDay::On(day) => day,
        FirstDay::NoDayRung => -1,
    };
    let count = telemetry::Value::Uint;
    let base = |level, message| {
        telemetry::Event::new(level, "pull.identity", message)
            .with("instrument", telemetry::Value::Str(witness.instrument))
            .with("listing", telemetry::Value::Str(witness.listing))
            .with("first_day", telemetry::Value::Int(first_day))
    };
    if let Some(why) = unread {
        let _dropped_when_filtered = telemetry::emit(
            &base(
                telemetry::Level::Warn,
                "minutes not judged against their served day: the day rung would not open; \
                 only the rules that need no day bar were applied",
            )
            .with("why", telemetry::Value::Str(why))
            .with("no_day_rung", count(tally.no_day_rung)),
        );
    }
    for group in &tally.flagged {
        let message = if group.refused {
            "rows refused: not this series on this day"
        } else {
            "minutes landed outside their own served day's range: suspect, not refused"
        };
        let _dropped_when_filtered = telemetry::emit(
            &base(telemetry::Level::Warn, message)
                .with("cause", telemetry::Value::Str(group.code))
                .with("day", telemetry::Value::Int(group.day))
                .with("rows", count(group.minutes)),
        );
    }
    if tally.flagged.is_empty() && unread.is_none() {
        let _dropped_when_filtered = telemetry::emit(
            &base(
                telemetry::Level::Info,
                "minutes landed unjudged: the day rung holds no bar for their day",
            )
            .with("held", count(tally.held))
            .with("no_day_rung", count(tally.no_day_rung))
            .with("no_day_bar", count(tally.no_day_bar))
            .with("day_bar_unusable", count(tally.day_bar_unusable)),
        );
    }
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
