//! `GET /feed-checks.json`, `GET /tick-checks.json` and
//! `GET /calendar-days.json`: the append-only outputs of the data checks,
//! served to `/ingest` (DATA-1w, D-5190 to D-5194).
//!
//! # What each serves
//!
//! - **`/feed-checks.json`**: DATA-1's `cli feed-reconcile` months, the
//!   Zerodha minutes, the Zerodha day bar and the GDFL seconds of every cash
//!   share side by side, one row per stock-day: its status, its exact paisa
//!   factor pair, `adjusted`, the `ex_date` verdict, the Zerodha minute flags
//!   and the short volume (measured, not judged: no ENV-1 envelope exists).
//!   - no `month`: the months held, newest version each, with their counts;
//!   - `month=YYYY-MM`: that month's counts and its shares;
//!   - `month` and `symbol`: that share's day rows;
//!   - `month`, `version` and `line`: the source row behind a number.
//! - **`/tick-checks.json`**: DATA-2's `cli tick-to-second-check` years, each
//!   stored GDFL share-day rebuilt from its raw source under rule T and rule A
//!   and compared with the store.
//!   - no `year` and no `month`: the years held, with their counts;
//!   - `year=YYYY`: that year's counts and its months;
//!   - `month=YYYY-MM`: that month's shares; with `symbol`, its day rows;
//!   - `year`, `version` and `line`: the source row behind a number.
//! - **`/calendar-days.json`**: CAL-1's tradable-day rule,
//!   `pull::calendar::tradable_day`, read from the compile-time tables:
//!   - no `year`: every year of the calendar of record with its counts by
//!     reason (`pull::calendar::year_exclusions`, two lookups in the running
//!     sums), a year the record does not wholly hold named and refused;
//!   - `year=YYYY` (and `reason=` one reason's name): its excluded days, each
//!     with the notice, circular or evidence it rests on. Ordinary weekends
//!     are counted, never listed, as `cli calendar-days` prints them.
//!
//! # Paging: nothing typed (operator, 16 Oct 2026 17:15 IST)
//!
//! Every list is paged by `limit` and `offset`, two REQUIRED named fields with
//! no default. A missing one, a zero `limit`, a `limit` above the list's
//! length or an `offset` at or past its end is refused by name, never answered
//! with an empty page. Every answer states `total`, `offset`, `limit` and
//! `returned`. A field a route does not read is refused by name.
//!
//! # Where the files are
//!
//! `BRUTEX_FEED_RECONCILE_DIR` and `BRUTEX_TICK_SECOND_DIR` name the folders
//! when set. Unset, they are the store's audit area, `<store>/audit/
//! feed-reconcile` (where DATA-1's full run writes) and `<store>/audit/
//! tick-to-second`. Every answer names the folder it read and which of the
//! two chose it, so an empty answer is never mistaken for an empty check.
//!
//! # Cost
//!
//! Per request: one `stat` of the folder, one map probe per month or year on
//! the page, O(limit) rows. The first ask of a version parses it once,
//! O(its bytes) (`feed_checks_files`). The calendar is O(1) per year and
//! O(days of the year) for its list, a constant. Measured in
//! `docs/06-limits.md` L-D1W-01.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::feed_checks_files::{
    self as files, FeedMonth, FeedRow, FeedStock, FeedTally, Place, Refusal, TickRow, TickTally,
    TickYear, Words, count_at,
};
use super::{Loaded, param};
use pull::calendar::{self, NotTradable};
use pull::session::Day;

/// The environment variable that names the feed-reconcile folder.
pub(crate) const FEED_DIR_ENV: &str = "BRUTEX_FEED_RECONCILE_DIR";
/// The environment variable that names the tick-to-second folder.
pub(crate) const TICK_DIR_ENV: &str = "BRUTEX_TICK_SECOND_DIR";

/// The three routes.
pub(crate) const FEED_ROUTE: &str = "/feed-checks.json";
/// See [`FEED_ROUTE`].
pub(crate) const TICK_ROUTE: &str = "/tick-checks.json";
/// See [`FEED_ROUTE`].
pub(crate) const CALENDAR_ROUTE: &str = "/calendar-days.json";

/// A page of a list: both fields required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Page {
    /// Rows asked for, at least 1.
    pub(crate) limit: usize,
    /// The first row asked for, 0 = the first.
    pub(crate) offset: usize,
}

/// Refuses every field of `query` that `known` does not name.
fn known_fields(query: &str, route: &str, known: &[&str]) -> Result<(), Refusal> {
    for field in query.split('&').filter(|field| !field.is_empty()) {
        let name = field.split('=').next().unwrap_or(field);
        if !known.contains(&name) {
            let shown: String = name.chars().take(32).collect();
            return Err(Refusal::Asked(format!(
                "{route} does not read the field {shown:?}: it reads {}",
                known.join(", ")
            )));
        }
    }
    Ok(())
}

/// Whether `query` names `name` at all.
fn named(query: &str, name: &str) -> bool {
    query
        .split('&')
        .any(|field| field.split('=').next() == Some(name))
}

/// A field holding a whole number written as one: digits only, no sign, no
/// leading zero, no more than 19 of them.
fn number(query: &str, name: &str) -> Result<Option<usize>, Refusal> {
    if !named(query, name) {
        return Ok(None);
    }
    let text = param(query, name);
    let shown: String = text.chars().take(24).collect();
    let canonical = !text.is_empty()
        && text.len() <= 19
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    canonical
        .then(|| text.parse::<usize>().ok())
        .flatten()
        .map(Some)
        .ok_or_else(|| Refusal::Asked(format!("{name}={shown:?} is not a whole number")))
}

/// The page a query asks for.
///
/// # Errors
///
/// A missing `limit` or `offset`, one that is not a whole number, or a zero
/// `limit`, each named.
pub(crate) fn page_of(query: &str) -> Result<Page, Refusal> {
    let limit = number(query, "limit")?;
    let offset = number(query, "offset")?;
    let (Some(limit), Some(offset)) = (limit, offset) else {
        return Err(Refusal::Asked(
            "limit and offset are both required, and neither has a default: ask for \
             limit=<rows>&offset=<first row>"
                .to_owned(),
        ));
    };
    if limit == 0 {
        return Err(Refusal::Asked(
            "limit=0 asks for no rows; a page holds at least one".to_owned(),
        ));
    }
    Ok(Page { limit, offset })
}

/// The rows `page` covers of a list of `total` named `what`, or why not.
///
/// # Errors
///
/// An empty list ([`Refusal::Absent`]), a `limit` above `total`, or an
/// `offset` at or past its end ([`Refusal::Asked`]), each naming `total`.
pub(crate) fn window(page: Page, total: usize, what: &str) -> Result<(usize, usize), Refusal> {
    if total == 0 {
        return Err(Refusal::Absent(format!("there are no {what}")));
    }
    if page.limit > total {
        return Err(Refusal::Asked(format!(
            "limit={} is above the {total} {what} there are; ask for at most {total}",
            page.limit
        )));
    }
    if page.offset >= total {
        return Err(Refusal::Asked(format!(
            "offset={} is past the end: there are {total} {what}, numbered 0 to {}",
            page.offset,
            total - 1
        )));
    }
    Ok((
        page.offset,
        page.offset.saturating_add(page.limit).min(total),
    ))
}

/// An answer being built: an object whose fields are set by name.
type Fields = Map<String, Value>;

/// The field `key` of `fields`, made `null` first when it is not there.
fn field<'a>(fields: &'a mut Fields, key: &str) -> &'a mut Value {
    fields.entry(key).or_insert(Value::Null)
}

/// The head every check answer opens with: what it is, the folder it read,
/// what chose that folder, and the words its counts are defined by.
fn head(check: &str, dir: &Path, from: &str, definitions: Value) -> Fields {
    Fields::from_iter([
        ("check".to_owned(), json!(check)),
        ("dir".to_owned(), json!(dir.display().to_string())),
        ("dir_from".to_owned(), json!(from)),
        ("definitions".to_owned(), definitions),
    ])
}

/// The paging fields of an answer.
fn paged(page: Page, total: usize, range: (usize, usize)) -> Value {
    json!({
        "total": total,
        "offset": page.offset,
        "limit": page.limit,
        "returned": range.1 - range.0,
    })
}

/// The folder a check's files are read from, and what chose it.
pub(crate) fn check_dir(store_root: &Path, env: &str, leaf: &str) -> (PathBuf, String) {
    match std::env::var_os(env).filter(|value| !value.is_empty()) {
        Some(value) => (
            PathBuf::from(value),
            format!("the environment variable {env}"),
        ),
        None => (
            store_root.join("audit").join(leaf),
            format!("the store's audit area ({env} is not set)"),
        ),
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// Each word with its count, words with none left out.
fn by_word(words: &Words, counts: &[u32]) -> Value {
    let mut out = Map::new();
    for (id, word) in words.all().iter().enumerate() {
        let count = u16::try_from(id).map_or(0, |id| count_at(counts, id));
        if count > 0 {
            out.insert(word.clone(), json!(count));
        }
    }
    Value::Object(out)
}

/// `true` when `query` names `name` with a value; the value, decoded.
fn text_field(query: &str, name: &str) -> Option<String> {
    named(query, name).then(|| param(query, name))
}

/// A share's key as a request names it, or why it cannot be one.
fn symbol_field(query: &str) -> Result<Option<String>, Refusal> {
    let Some(symbol) = text_field(query, "symbol") else {
        return Ok(None);
    };
    if symbol.is_empty()
        || symbol.len() > 64
        || symbol
            .bytes()
            .any(|b| b.is_ascii_control() || b == b',' || b == b'/')
    {
        let shown: String = symbol.chars().take(32).collect();
        return Err(Refusal::Asked(format!(
            "symbol={shown:?} cannot be a stored share key"
        )));
    }
    Ok(Some(symbol))
}

/// `month=YYYY-MM`, when named.
fn month_field(query: &str) -> Result<Option<store::path::YearMonth>, Refusal> {
    let Some(text) = text_field(query, "month") else {
        return Ok(None);
    };
    files::month_of(&text).map(Some).ok_or_else(|| {
        let shown: String = text.chars().take(24).collect();
        Refusal::Asked(format!("month={shown:?} is not a month written YYYY-MM"))
    })
}

/// `year=YYYY`, when named: four digits, nothing else.
fn year_field(query: &str) -> Result<Option<u16>, Refusal> {
    let Some(text) = text_field(query, "year") else {
        return Ok(None);
    };
    let shown: String = text.chars().take(24).collect();
    (text.len() == 4 && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse::<u16>().ok())
        .flatten()
        .filter(|year| *year >= 1970)
        .map(Some)
        .ok_or_else(|| Refusal::Asked(format!("year={shown:?} is not a year written YYYY")))
}

/// What the calendar of record says of `day`: `null` when it takes trades,
/// the reason's name otherwise.
fn tradable(day: Day) -> Value {
    match calendar::tradable_day(i64::from(day.days_from_epoch())) {
        Ok(()) => Value::Null,
        Err(reason) => json!(reason.name()),
    }
}

/// The source row at `line` of a version, from the rows it parsed into:
/// the rows of a version are consecutive lines, so the row is found by one
/// subtraction, never a search.
fn row_at<T>(rows: &[T], line: usize, place: impl Fn(&T) -> Place) -> Option<&T> {
    let first = usize::try_from(place(rows.first()?).line).ok()?;
    let row = rows.get(line.checked_sub(first)?)?;
    (usize::try_from(place(row).line).ok() == Some(line)).then_some(row)
}

/// `version` and `line`, both required together.
fn line_fields(query: &str, held: u32) -> Result<Option<usize>, Refusal> {
    let line = number(query, "line")?;
    let version = number(query, "version")?;
    match (line, version) {
        (None, None) => Ok(None),
        (Some(line), Some(version)) => {
            if u32::try_from(version).ok() != Some(held) {
                return Err(Refusal::Absent(format!(
                    "version {version} is not the newest version held, {held}: a line number \
                     names a row of one version, so ask again from the newest"
                )));
            }
            Ok(Some(line))
        }
        _ => Err(Refusal::Asked(
            "line and version are asked together: a line number names a row of one version"
                .to_owned(),
        )),
    }
}

// ═══ /feed-checks.json ═══

const FEED_FIELDS: [&str; 6] = ["month", "symbol", "limit", "offset", "version", "line"];

fn feed_tally(month: &FeedMonth, tally: &FeedTally) -> Value {
    let flag = |name: &str| {
        month
            .flag(name)
            .map_or(0, |bit| count_at(&tally.flags, bit))
    };
    json!({
        "rows": tally.rows,
        "stock_days": tally.days,
        "by_status": by_word(&month.statuses, &tally.status),
        "adjusted_days": flag("adjusted"),
        "ex_date_days": flag("ex_date"),
        "ex_date_verdicts": by_word(&month.ex_dates, &tally.ex_date),
        "zerodha_minute_short_days": tally.short_positive,
        "short_judgements": by_word(&month.judgements, &tally.judgement),
        "flags": by_word(&month.flag_names, &tally.flags),
    })
}

fn feed_head(dir: &Path, from: &str) -> Fields {
    head(
        "feed-reconcile (DATA-1, cli feed-reconcile)",
        dir,
        from,
        json!({
                "stock_days": "rows of one day; a share whose month could not be read has one row, day `-`",
                "adjusted_days": "days flagged `adjusted`: 1 lies outside the interval of the day's median factor pair",
                "ex_date_days": "days flagged `ex_date`: two consecutive matched days have disjoint intervals; every one is unsourced (no corporate-action source is merged)",
                "zerodha_minute_short_days": "days whose short_volume (Zerodha day bar minus its minute sum minus the GDFL volume outside Zerodha's printed minutes) is above zero: MEASURED, NOT JUDGED -- no ENV-1 envelope exists, so every computed day's short_judgement is unjudged",
                "factor": "the exact paisa pair (median_z, median_g) of the day's lower-median matched minute; never a float",
        }),
    )
}

fn feed_month_json(month: &FeedMonth) -> Value {
    json!({
        "month": month.month.to_string(),
        "version": month.version,
        "file": month.file.display().to_string(),
        "bytes": month.bytes,
        "blake3": hex(&month.digest),
        "load_micros": month.load_micros,
        "stocks": month.stocks.len(),
        "adjusted_stocks": month.adjusted_stocks,
        "ex_date_stocks": month.ex_date_stocks,
        "counts": feed_tally(month, &month.tally),
        "writer_summary": month.summary,
        "run": month.header,
    })
}

fn feed_stock_json(month: &FeedMonth, stock: &FeedStock) -> Value {
    let rows = month.rows_of(stock);
    json!({
        "symbol": stock.name,
        "first_line": rows.first().map(|row| row.place.line),
        "last_line": rows.last().map(|row| row.place.line),
        "counts": feed_tally(month, &stock.tally),
    })
}

fn feed_row_json(month: &FeedMonth, row: &FeedRow) -> Value {
    let day = (row.day > 0)
        .then(|| Day::new(month.month.year(), month.month.month(), row.day).ok())
        .flatten();
    let flags: Vec<&str> = (0..64_u16)
        .filter(|bit| row.flags & (1 << bit) != 0)
        .map(|bit| month.flag_names.name(bit))
        .collect();
    let adjusted = month
        .flag("adjusted")
        .is_some_and(|bit| row.flags & (1 << bit) != 0);
    json!({
        "day": day.map_or_else(|| "-".to_owned(), |day| day.to_string()),
        "line": row.place.line,
        "day_kind": month.day_kinds.name(row.day_kind),
        "tradable": day.map_or(Value::Null, tradable),
        "status": month.status_texts.name(row.status_text),
        "applied_lag": row.lag,
        "lag_source": month.lag_sources.name(row.lag_source),
        "factor": row.median.map(|(z, g)| json!({ "zerodha_paisa": z, "gdfl_paisa": g })),
        "adjusted": adjusted,
        "ex_date": month.ex_dates.name(row.ex_date),
        "flags": flags,
        "short_volume": row.short.map(|short| short.to_string()),
        "short_judgement": month.judgements.name(row.judgement),
    })
}

/// A check folder that does not exist, named as such: a 404 that says no
/// check has been written there, never the words of an empty check.
fn missing_folder(exists: bool, dir: &Path, check: &str, writer: &str) -> Result<(), Refusal> {
    if exists {
        return Ok(());
    }
    Err(Refusal::Absent(format!(
        "`{}` does not exist: no {check} check has been written there yet (`{writer}` writes \
         it; D-5193 names the folder)",
        dir.display()
    )))
}

/// The whole `/feed-checks.json` answer over the store at `root`.
///
/// # Errors
///
/// [`Refusal`], by name.
pub(crate) fn feed_answer(root: &Path, query: &str) -> Result<Value, Refusal> {
    known_fields(query, FEED_ROUTE, &FEED_FIELDS)?;
    let month = month_field(query)?;
    let symbol = symbol_field(query)?;
    let (dir, from) = check_dir(root, FEED_DIR_ENV, "feed-reconcile");
    let listing = files::feed_listing(&dir)?;
    missing_folder(
        listing.exists(),
        &dir,
        "feed-reconcile",
        "cli feed-reconcile",
    )?;
    let mut answer = feed_head(&dir, &from);
    let Some(month) = month else {
        if symbol.is_some() || named(query, "line") || named(query, "version") {
            return Err(Refusal::Asked(
                "symbol, line and version name a part of one month: ask with month=YYYY-MM"
                    .to_owned(),
            ));
        }
        let page = page_of(query)?;
        let total = listing.newest.len();
        let range = window(
            page,
            total,
            &format!("feed-reconcile months in `{}`", dir.display()),
        )?;
        let rows: Vec<Value> = listing
            .page(range)
            .iter()
            .map(|(month, version)| match files::feed_month(&dir, *month, *version) {
                Ok(held) => feed_month_json(&held),
                Err(why) => json!({ "month": month.to_string(), "version": version, "error": why.text() }),
            })
            .collect();
        *field(&mut answer, "page") = paged(page, total, range);
        *field(&mut answer, "months") = Value::Array(rows);
        return Ok(Value::Object(answer));
    };
    let Some(version) = listing.newest.get(&month).copied() else {
        return Err(Refusal::Absent(format!(
            "`{}` holds no feed-reconcile version of {month}: that month has not been checked",
            dir.display()
        )));
    };
    let held = files::feed_month(&dir, month, version)?;
    *field(&mut answer, "month") = feed_month_json(&held);
    if let Some(line) = line_fields(query, held.version)? {
        if symbol.is_some() || named(query, "limit") || named(query, "offset") {
            return Err(Refusal::Asked(
                "a source row is asked by month, version and line alone".to_owned(),
            ));
        }
        let row = row_at(&held.rows, line, |row| row.place).ok_or_else(|| {
            Refusal::Absent(format!(
                "line {line} of `{}` is not a row: its rows are lines {} to {}",
                held.file.display(),
                held.rows.first().map_or(0, |row| row.place.line),
                held.rows.last().map_or(0, |row| row.place.line)
            ))
        })?;
        *field(&mut answer, "row") = feed_row_json(&held, row);
        *field(&mut answer, "source") = source_json(&held.file, row.place, &held.columns)?;
        return Ok(Value::Object(answer));
    }
    let page = page_of(query)?;
    if let Some(symbol) = symbol {
        let stock = held.stock(&symbol).ok_or_else(|| {
            Refusal::Absent(format!(
                "{month} version {} holds no row of `{symbol}`",
                held.version
            ))
        })?;
        let rows = held.rows_of(stock);
        let range = window(page, rows.len(), &format!("rows of {symbol} in {month}"))?;
        *field(&mut answer, "stock") = feed_stock_json(&held, stock);
        *field(&mut answer, "page") = paged(page, rows.len(), range);
        *field(&mut answer, "days") = Value::Array(
            rows.get(range.0..range.1)
                .unwrap_or(&[])
                .iter()
                .map(|row| feed_row_json(&held, row))
                .collect(),
        );
        return Ok(Value::Object(answer));
    }
    let range = window(page, held.stocks.len(), &format!("shares in {month}"))?;
    *field(&mut answer, "page") = paged(page, held.stocks.len(), range);
    *field(&mut answer, "stocks") = Value::Array(
        held.stocks
            .get(range.0..range.1)
            .unwrap_or(&[])
            .iter()
            .map(|stock| feed_stock_json(&held, stock))
            .collect(),
    );
    Ok(Value::Object(answer))
}

fn source_json(file: &Path, place: Place, columns: &[String]) -> Result<Value, Refusal> {
    let cells = files::source_row(file, place, columns)?;
    Ok(json!({
        "file": file.display().to_string(),
        "line": place.line,
        "cells": cells.into_iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
    }))
}

// ═══ /tick-checks.json ═══

const TICK_FIELDS: [&str; 7] = [
    "year", "month", "symbol", "limit", "offset", "version", "line",
];

fn tick_tally(year: &TickYear, tally: &TickTally) -> Value {
    let rule = |i: usize| {
        let [compared, high, low] = tally.minutes.get(i).copied().unwrap_or_default();
        json!({ "compared": compared, "high_match": high, "low_match": low })
    };
    let exact_t = year
        .compares
        .find("exact_T")
        .map_or(0, |id| count_at(&tally.compare, id));
    json!({
        "rows": tally.rows,
        "share_days": tally.share_days,
        "by_status": by_word(&year.statuses, &tally.status),
        "by_compare": by_word(&year.compares, &tally.compare),
        "by_role": by_word(&year.roles, &tally.role),
        "by_session": by_word(&year.sessions, &tally.session),
        "by_zerodha_basis": by_word(&year.bases, &tally.basis),
        "rule_t_exact": { "share_days": exact_t, "of": tally.share_days },
        "minute_match": { "t": rule(0), "a": rule(1), "r": rule(2) },
        "fill": { "compared": tally.fill.first(), "worse_under_a": tally.fill.get(1) },
    })
}

fn tick_head(dir: &Path, from: &str) -> Fields {
    head(
        "tick-to-second (DATA-2, cli tick-to-second-check)",
        dir,
        from,
        json!({
                "share_days": "one per stored share-day: the row the year's index names for it (an `extra` row a `landing` row follows is listed and not counted)",
                "rule_t_exact": "share-days whose `compare` is exact_T: the store equals rule T (trades only) in every field and the digest, of every share-day",
                "minute_match": "Zerodha one-minute bars whose high (and low) the rebuilt seconds match exactly, summed over share-days, under rule T, rule A (every row's price) and rule R: on the RAW basis (DATA-1's factor is not applied, D-5188); a share's minutes count only on days Zerodha has them (zerodha_basis)",
                "fill": "stored seconds compared for the fill measure, and those whose rule-A high is above, or low below, the stored bar",
        }),
    )
}

fn tick_year_json(year: &TickYear) -> Value {
    json!({
        "year": year.year,
        "version": year.version,
        "file": year.file.display().to_string(),
        "index": year.index_file.display().to_string(),
        "bytes": year.bytes,
        "csv_blake3": hex(&year.csv_digest),
        "index_blake3": hex(&year.index_digest),
        "segments_through": year.segments_through,
        "load_micros": year.load_micros,
        "months": year.held_months().map(|month| month.month).collect::<Vec<_>>(),
        "counts": tick_tally(year, &year.tally),
    })
}

fn tick_row_json(year: &TickYear, row: &TickRow) -> Value {
    let rule = |i: usize| row.minutes.get(i).copied().unwrap_or_default();
    json!({
        "day": row.day.to_string(),
        "line": row.place.line,
        "tradable": tradable(row.day),
        "role": year.roles.name(row.role),
        "source": year.sources.name(row.source),
        "status": year.statuses.name(row.status),
        "compare": year.compares.name(row.compare),
        "session": year.sessions.name(row.session),
        "zerodha_basis": year.bases.name(row.basis),
        "counted": row.counted,
        "store_seconds": row.store_seconds,
        "minutes": { "t": rule(0), "a": rule(1), "r": rule(2) },
        "fill": row.fill,
    })
}

/// The tick-to-second year `year`, or why it is not held.
fn held_year(dir: &Path, year: u16) -> Result<std::sync::Arc<TickYear>, Refusal> {
    let listing = files::tick_listing(dir)?;
    missing_folder(
        listing.exists(),
        dir,
        "tick-to-second",
        "cli tick-to-second-check",
    )?;
    let version = listing.newest.get(&year).copied().ok_or_else(|| {
        Refusal::Absent(format!(
            "`{}` holds no tick-to-second index of {year}: that year has not been checked",
            dir.display()
        ))
    })?;
    files::tick_year(dir, year, version)
}

/// The `month=` part of `/tick-checks.json`: one month's shares, or with
/// `symbol` that share's day rows.
fn tick_month_answer(
    query: &str,
    dir: &Path,
    mut answer: Fields,
    month: store::path::YearMonth,
    symbol: Option<String>,
) -> Result<Value, Refusal> {
    if named(query, "line") || named(query, "version") {
        return Err(Refusal::Asked(
            "a source row is asked by year, version and line".to_owned(),
        ));
    }
    let held = held_year(dir, month.year())?;
    *field(&mut answer, "year") = tick_year_json(&held);
    let page = page_of(query)?;
    let of_month = held.month(month.month()).ok_or_else(|| {
        Refusal::Absent(format!(
            "{} version {} holds no row of {month}",
            held.year, held.version
        ))
    })?;
    *field(&mut answer, "month") = json!({
        "month": month.to_string(),
        "shares": of_month.stocks.len(),
        "counts": tick_tally(&held, &of_month.tally),
    });
    if let Some(symbol) = symbol {
        let stock = held
            .symbols
            .find(&symbol)
            .and_then(|id| of_month.stock(u32::from(id)))
            .ok_or_else(|| {
                Refusal::Absent(format!(
                    "{} version {} holds no row of `{symbol}` in {month}",
                    held.year, held.version
                ))
            })?;
        let rows = held.rows_of(stock);
        let range = window(page, rows.len(), &format!("rows of {symbol} in {month}"))?;
        *field(&mut answer, "stock") =
            json!({ "symbol": symbol, "counts": tick_tally(&held, &stock.tally) });
        *field(&mut answer, "page") = paged(page, rows.len(), range);
        *field(&mut answer, "days") = Value::Array(
            rows.get(range.0..range.1)
                .unwrap_or(&[])
                .iter()
                .filter_map(|at| held.row(*at))
                .map(|row| tick_row_json(&held, row))
                .collect(),
        );
        return Ok(Value::Object(answer));
    }
    let range = window(page, of_month.stocks.len(), &format!("shares in {month}"))?;
    *field(&mut answer, "page") = paged(page, of_month.stocks.len(), range);
    *field(&mut answer, "stocks") = Value::Array(
        of_month
            .stocks
            .get(range.0..range.1)
            .unwrap_or(&[])
            .iter()
            .map(|stock| {
                let rows = held.rows_of(stock);
                json!({
                    "symbol": held.symbols.name(u16::try_from(stock.symbol).unwrap_or(u16::MAX)),
                    "first_line": rows.first().and_then(|at| held.row(*at)).map(|row| row.place.line),
                    "last_line": rows.last().and_then(|at| held.row(*at)).map(|row| row.place.line),
                    "counts": tick_tally(&held, &stock.tally),
                })
            })
            .collect(),
    );
    Ok(Value::Object(answer))
}

/// The whole `/tick-checks.json` answer over the store at `root`.
///
/// # Errors
///
/// [`Refusal`], by name.
pub(crate) fn tick_answer(root: &Path, query: &str) -> Result<Value, Refusal> {
    known_fields(query, TICK_ROUTE, &TICK_FIELDS)?;
    let year = year_field(query)?;
    let month = month_field(query)?;
    let symbol = symbol_field(query)?;
    let (dir, from) = check_dir(root, TICK_DIR_ENV, "tick-to-second");
    let mut answer = tick_head(&dir, &from);
    if let (Some(year), Some(month)) = (year, month)
        && month.year() != year
    {
        return Err(Refusal::Asked(format!(
            "month={month} is not a month of year={year}"
        )));
    }
    if let Some(month) = month {
        return tick_month_answer(query, &dir, answer, month, symbol);
    }
    if symbol.is_some() {
        return Err(Refusal::Asked(
            "symbol names a share of one month: ask with month=YYYY-MM".to_owned(),
        ));
    }
    let Some(year) = year else {
        if named(query, "line") || named(query, "version") {
            return Err(Refusal::Asked(
                "a source row is asked by year, version and line".to_owned(),
            ));
        }
        let listing = files::tick_listing(&dir)?;
        missing_folder(
            listing.exists(),
            &dir,
            "tick-to-second",
            "cli tick-to-second-check",
        )?;
        let page = page_of(query)?;
        let total = listing.newest.len();
        let range = window(
            page,
            total,
            &format!("tick-to-second years in `{}`", dir.display()),
        )?;
        let rows: Vec<Value> = listing
            .page(range)
            .iter()
            .map(
                |(year, version)| match files::tick_year(&dir, *year, *version) {
                    Ok(held) => tick_year_json(&held),
                    Err(why) => json!({ "year": year, "version": version, "error": why.text() }),
                },
            )
            .collect();
        *field(&mut answer, "page") = paged(page, total, range);
        *field(&mut answer, "years") = Value::Array(rows);
        return Ok(Value::Object(answer));
    };
    let held = held_year(&dir, year)?;
    *field(&mut answer, "year") = tick_year_json(&held);
    if let Some(line) = line_fields(query, held.version)? {
        if named(query, "limit") || named(query, "offset") {
            return Err(Refusal::Asked(
                "a source row is asked by year, version and line alone".to_owned(),
            ));
        }
        let row = row_at(&held.rows, line, |row| row.place).ok_or_else(|| {
            Refusal::Absent(format!(
                "line {line} of `{}` is not a row: its rows are lines {} to {}",
                held.file.display(),
                held.rows.first().map_or(0, |row| row.place.line),
                held.rows.last().map_or(0, |row| row.place.line)
            ))
        })?;
        *field(&mut answer, "row") = tick_row_json(&held, row);
        *field(&mut answer, "source") = source_json(&held.file, row.place, &held.columns)?;
        return Ok(Value::Object(answer));
    }
    let page = page_of(query)?;
    // AT MOST TWELVE: the months of one year.
    let months: Vec<_> = held.held_months().collect();
    let range = window(page, months.len(), &format!("months in {year}"))?;
    *field(&mut answer, "page") = paged(page, months.len(), range);
    *field(&mut answer, "months") = Value::Array(
        months
            .get(range.0..range.1)
            .unwrap_or(&[])
            .iter()
            .map(|month| {
                json!({
                    "month": format!("{year}-{:02}", month.month),
                    "shares": month.stocks.len(),
                    "counts": tick_tally(&held, &month.tally),
                })
            })
            .collect(),
    );
    Ok(Value::Object(answer))
}

// ═══ /calendar-days.json ═══

const CALENDAR_FIELDS: [&str; 4] = ["year", "reason", "limit", "offset"];

/// The years the calendar of record touches, in order.
fn record_years() -> (u16, u16) {
    let (first, last) = calendar::CalendarVersion::newest().record();
    let year = |day: i64| {
        u32::try_from(day)
            .ok()
            .and_then(|days| Day::from_days(days).ok())
            .map_or(0, Day::year)
    };
    (year(first), year(last))
}

fn holidays_json(holidays: calendar::YearHolidays) -> Value {
    match holidays {
        calendar::YearHolidays::Recorded(number) => {
            json!({ "record": "circular", "circular": number })
        }
        calendar::YearHolidays::Measured => {
            json!({ "record": "measured", "note": "the whole year lies in the measured window; its closed days were measured from the store" })
        }
        calendar::YearHolidays::Owed => {
            json!({ "record": "owed", "note": "no NSE holiday circular is recorded; days outside the measured window are Unmeasured" })
        }
    }
}

fn calendar_year_json(year: u16) -> Value {
    let record = calendar::year_record(year);
    let mut out = Fields::from_iter([
        ("year".to_owned(), json!(year)),
        ("holidays".to_owned(), holidays_json(record.holidays)),
        ("notices".to_owned(), json!(record.notices)),
        ("untimed_notices".to_owned(), json!(record.untimed_notices)),
    ]);
    match calendar::year_exclusions(year) {
        Ok(counted) => {
            let counts = counted.counts;
            let mut by_reason = Map::new();
            for reason in NotTradable::ALL {
                let count = counts.count(reason);
                if count > 0 {
                    by_reason.insert(reason.name().to_owned(), json!(count));
                }
            }
            *field(&mut out, "days") = json!(counts.days());
            *field(&mut out, "tradable") = json!(counts.tradable);
            *field(&mut out, "excluded_sessions") = json!(counts.excluded_sessions());
            *field(&mut out, "weekends") = json!(counts.count(NotTradable::Weekend));
            *field(&mut out, "holidays_closed") = json!(counts.count(NotTradable::Holiday));
            *field(&mut out, "unexplained") = json!(counts.unexplained());
            *field(&mut out, "by_reason") = Value::Object(by_reason);
        }
        Err(why) => *field(&mut out, "refused") = json!(why.to_string()),
    }
    Value::Object(out)
}

/// The address inside a source sentence, when it holds one.
fn address_in(text: &str) -> Option<String> {
    let at = text.find("https://")?;
    let rest = text.get(at..)?;
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    rest.get(..end).map(str::to_owned)
}

fn calendar_day_json(day: i64, reason: NotTradable) -> Value {
    let record = calendar::record_of(day);
    let civil = u32::try_from(day)
        .ok()
        .and_then(|days| Day::from_days(days).ok())
        .map_or_else(|| day.to_string(), |civil| civil.to_string());
    let notice = calendar::session_circular(day);
    let source = notice.map(|notice| notice.source);
    json!({
        "day": civil,
        "weekday": record.weekday.short(),
        "reason": reason.name(),
        "label": reason.label(),
        "unexplained": reason.is_unexplained(),
        "holiday": record.holiday.map(|holiday| json!({ "name": holiday.name, "circular": holiday.circular })),
        "notice": source,
        "notice_url": source.and_then(address_in),
        "special_evidence": record.special.map(|special| special.evidence),
        "halt": record.halt.map(|halt| json!({
            "suspected": halt.evidence.is_suspected(),
            "evidence": halt.evidence.text(),
        })),
    })
}

/// The whole `/calendar-days.json` answer.
///
/// # Errors
///
/// [`Refusal`], by name.
pub(crate) fn calendar_answer(query: &str) -> Result<Value, Refusal> {
    known_fields(query, CALENDAR_ROUTE, &CALENDAR_FIELDS)?;
    let year = year_field(query)?;
    let reason = match text_field(query, "reason") {
        None => None,
        Some(name) => Some(
            NotTradable::ALL
                .into_iter()
                .find(|reason| reason.name() == name && *reason != NotTradable::Weekend)
                .ok_or_else(|| {
                    let shown: String = name.chars().take(32).collect();
                    Refusal::Asked(format!(
                        "reason={shown:?} is not a listed reason: one of {} (Weekend is \
                         counted, never listed)",
                        NotTradable::ALL
                            .iter()
                            .filter(|reason| **reason != NotTradable::Weekend)
                            .map(|reason| reason.name())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                })?,
        ),
    };
    let page = page_of(query)?;
    let (first, last) = record_years();
    let mut answer = Fields::from_iter([
        (
            "check".to_owned(),
            json!("tradable-day rule (CAL-1, pull::calendar::tradable_day)"),
        ),
        ("record_years".to_owned(), json!([first, last])),
        (
            "rule".to_owned(),
            json!(
                "a day takes a trade and enters an own-past statistic only when it is a regular weekday 09:15-15:29 session with no special session, no notice of one and no market-wide halt; every other day is excluded, its data stays stored and labelled; ordinary closed weekends are counted, never listed"
            ),
        ),
    ]);
    let Some(year) = year else {
        if reason.is_some() {
            return Err(Refusal::Asked(
                "reason names the days of one year: ask with year=YYYY".to_owned(),
            ));
        }
        let total = if first == 0 {
            0
        } else {
            usize::from(last.saturating_sub(first)) + 1
        };
        let range = window(page, total, "years in the calendar of record")?;
        let years: Vec<Value> = (range.0..range.1)
            .filter_map(|at| u16::try_from(at).ok())
            .map(|at| calendar_year_json(first.saturating_add(at)))
            .collect();
        *field(&mut answer, "page") = paged(page, total, range);
        *field(&mut answer, "years") = Value::Array(years);
        return Ok(Value::Object(answer));
    };
    if year < first || year > last {
        return Err(Refusal::Absent(format!(
            "{year} lies outside the calendar of record, {first} to {last}"
        )));
    }
    let counted = calendar_year_json(year);
    if let Some(why) = counted.get("refused").and_then(Value::as_str) {
        return Err(Refusal::Absent(why.to_owned()));
    }
    *field(&mut answer, "year") = counted;
    let (Ok(start), Ok(end)) = (Day::new(year, 1, 1), Day::new(year, 12, 31)) else {
        return Err(Refusal::Asked(format!(
            "{year} is not a year this calendar spells"
        )));
    };
    // AT MOST 366 DAYS, each one O(1) walk of the compile-time tables: the
    // list of one year is a constant's work, whatever the store holds.
    let mut days = Vec::new();
    for day in i64::from(start.days_from_epoch())..=i64::from(end.days_from_epoch()) {
        if let Err(why) = calendar::tradable_day(day)
            && why != NotTradable::Weekend
            && reason.is_none_or(|asked| asked == why)
        {
            days.push((day, why));
        }
    }
    let what = reason.map_or_else(
        || format!("excluded days in {year} besides weekends"),
        |reason| format!("{} days in {year}", reason.name()),
    );
    let range = window(page, days.len(), &what)?;
    *field(&mut answer, "page") = paged(page, days.len(), range);
    *field(&mut answer, "days") = Value::Array(
        days.get(range.0..range.1)
            .unwrap_or(&[])
            .iter()
            .map(|(day, why)| calendar_day_json(*day, *why))
            .collect(),
    );
    Ok(Value::Object(answer))
}

// ═══ The handlers ═══

type Answered = (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

fn json_header() -> [(axum::http::HeaderName, &'static str); 1] {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

fn served(route: &'static str, answer: Result<Value, Refusal>) -> Answered {
    match answer {
        Ok(body) => (axum::http::StatusCode::OK, json_header(), body.to_string()),
        Err(why) => {
            let status = match why {
                Refusal::Asked(_) => axum::http::StatusCode::BAD_REQUEST,
                Refusal::Absent(_) => axum::http::StatusCode::NOT_FOUND,
                Refusal::Refused(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            };
            (
                status,
                json_header(),
                json!({ "error": why.text(), "route": route }).to_string(),
            )
        }
    }
}

/// The answer of work run on the store-read pool, or the pool's refusal.
fn pooled(
    route: &'static str,
    ran: Result<Result<Value, Refusal>, crate::detail::RunError>,
) -> Answered {
    match ran {
        Ok(answer) => served(route, answer),
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                route,
                crate::detail::MAX_STORE_READ_CONCURRENT,
                &why,
            );
            (status, json_header(), body)
        }
    }
}

/// `GET /feed-checks.json`.
pub(super) async fn feed_checks_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> Answered {
    let query = uri.query().unwrap_or("").to_owned();
    let root = site.store_root.clone();
    let ran = crate::detail::run_store_read(move || feed_answer(&root, &query)).await;
    pooled(FEED_ROUTE, ran)
}

/// `GET /tick-checks.json`.
pub(super) async fn tick_checks_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> Answered {
    let query = uri.query().unwrap_or("").to_owned();
    let root = site.store_root.clone();
    let ran = crate::detail::run_store_read(move || tick_answer(&root, &query)).await;
    pooled(TICK_ROUTE, ran)
}

/// `GET /calendar-days.json`. Reads no store: the tables are compiled in.
pub(super) async fn calendar_days_json(uri: axum::http::Uri) -> Answered {
    let query = uri.query().unwrap_or("").to_owned();
    served(CALENDAR_ROUTE, calendar_answer(&query))
}

#[cfg(test)]
#[path = "feed_checks_tests.rs"]
mod tests;
