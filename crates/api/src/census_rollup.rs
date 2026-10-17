//! `/census/rollup.json` -- the coverage roll-up per month of one selection,
//! read off the facet index the census moves forward, never off the census
//! list itself (R3-09; NEW-05, RA-008, DB-G80).
//!
//! # Why this route exists
//!
//! /db's Coverage grid counted its months by downloading the WHOLE census of
//! the feed (`/store.json?encoding=census-tuples-v1`: 53,131,576 bytes and
//! 602,732 rows for GDFL on 12 Oct 2026) and folding it in the browser. What
//! the grid shows per month is a handful of numbers: how many
//! instrument-months the selection holds at each rung, their bars, their
//! first and last stamp, the days those span and the days the calendar
//! owes. Those numbers are kept per (month, rung) in the facet index
//! ([`super::facets::MonthCell`]) by the same `apply` that moves every other
//! node of it forward by the entries the census appends, so this answer is
//! read off memory: its size is the selection's months times its rungs,
//! whatever the census holds.
//!
//! ```text
//! GET /census/rollup.json?feed=gdfl&segment=FNO                     the segment's months
//! GET /census/rollup.json?feed=gdfl&segment=FNO&underlying=NIFTY    one underlying's months
//! GET /census/rollup.json?...&exchange=NSE                          with the exchange named
//! ```
//!
//! # What is counted, and what is not claimed
//!
//! * `instrument_months`, `rows`, `first_ts`, `last_ts`: exact, the census's
//!   own numbers summed (`MonthCell::apply`).
//! * `days_owed`: the sessions the calendar of record holds in the month
//!   (`pull::calendar::sourced_kind_of`, the NSE trading-holiday circulars of
//!   D-2805; a Muhurat of unmeasured length is a session). It is what a series
//!   listed for the whole month is owed: an option contract listed or expiring
//!   inside the month is owed fewer, and the answer does not say how many.
//!   `null` where any day of the month is outside the record -- never a count
//!   that silently begins at the record's edge.
//! * `days_held`: the sessions from the IST day of the earliest first bar to
//!   the IST day of the latest last bar the census records for the cell. The
//!   census records each instrument-month's first and last bar, not which days
//!   between them hold bars, so a hole inside the span is NOT seen here (the
//!   day band and `/gaps.json` see it). The answer states this basis.
//!
//! # Cost, and its bound
//!
//! The index is moved forward (or, cold, built) under its lock exactly as
//! `/census/facets.json` moves it; warmed at start and after every
//! background re-read (`census_cache::start`), so a request reads one map
//! (one probe per exchange for an underlying) and writes one row per
//! (month, rung) it holds, plus at most 31 calendar lookups per month. The
//! cells of one answer are capped at [`CELLS_MOST`] and refused by name past
//! it (`rollup_too_many`), never served slowly: 4,096 cells is 34 years of
//! months at all ten rungs. Named for `docs/06-limits.md`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use store::path::{Timeframe, YearMonth};

use super::facets::{Cells, EXCHANGES, MonthCell, SEGMENTS};
use super::{Loaded, Site, no_such_feed_json, param, vendor_census};
use crate::{census, ingest, render};

/// The most (month, rung) cells one answer carries. See the module header.
pub(crate) const CELLS_MOST: usize = 4_096;

/// The parameters this route reads; any other is refused by name.
const NAMES: [&str; 4] = ["feed", "segment", "exchange", "underlying"];

/// The `days_owed` basis, stated in every answer.
pub(crate) const DAYS_OWED_BASIS: &str = "sessions the calendar of record holds in the month \
     (pull::calendar::sourced_kind_of: the NSE trading-holiday circulars, D-2805; a Muhurat of \
     unmeasured length is a session): what a series listed the whole month is owed; a contract \
     listed or expiring inside the month is owed fewer; null where a day of the month is outside \
     the record";

/// The `days_held` basis, stated in every answer.
pub(crate) const DAYS_HELD_BASIS: &str = "sessions from the IST day of the earliest first bar to \
     the IST day of the latest last bar the census records for the cell; the census records each \
     instrument-month's first and last bar, not which days between hold bars, so a hole inside \
     the span is not seen here (the day band and /gaps.json see it); null where a day of the span \
     is outside the record";

/// The order the answer lists its rows in, stated in every answer.
const ORDER: &str = "months newest first; within a month, finest rung first";

/// What a request asked for, read and checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Asked {
    feed: Vendor,
    segment: Segment,
    exchange: Option<Exchange>,
    underlying: Option<Symbol>,
}

/// `{"error": <code>, "why": <sentence>}`.
fn error_json(code: &str, why: &str) -> String {
    format!(
        r#"{{"error":{},"why":{}}}"#,
        render::json_string(code),
        render::json_string(why)
    )
}

/// What a caller sent, capped for a sentence.
fn echo(word: &str) -> String {
    super::store_route::since_echo(word)
}

/// Reads the query, or names the parameter it cannot read: an unknown
/// parameter, an empty or unknown feed, a segment that is not one, an
/// exchange without an underlying.
fn asked(query: &str) -> Result<Asked, (axum::http::StatusCode, String)> {
    let bad = |why: String| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            error_json("bad_request", &why),
        )
    };
    // NO PARAMETER IS IGNORED SILENTLY (the operator's rule 3): one this route
    // does not read is refused by name, so a caller who spelled `segement` or
    // asked a `since` of a roll-up learns it was not applied.
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let name = pair.split_once('=').map_or(pair, |(name, _)| name);
        if !NAMES.contains(&name) {
            return Err(bad(format!(
                "parameter {:?} is not one this route reads; it reads {}",
                echo(name),
                NAMES.join(", ")
            )));
        }
    }
    let feed_word = param(query, "feed");
    let Some(feed) = (!feed_word.is_empty())
        .then(|| ingest::parse_vendor(&feed_word))
        .flatten()
    else {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            no_such_feed_json(&echo(&feed_word)),
        ));
    };
    let segment_word = param(query, "segment");
    let segment = Segment::parse(&segment_word).map_err(|why| {
        bad(format!(
            "segment {:?}: {}; one of {} is required",
            echo(&segment_word),
            echo(&why.to_string()),
            SEGMENTS.map(Segment::as_str).join(", ")
        ))
    })?;
    let underlying = match param(query, "underlying").as_str() {
        "" => None,
        word => Some(Symbol::new(word).map_err(|why| {
            bad(format!(
                "underlying {:?}: {}",
                echo(word),
                echo(&why.to_string())
            ))
        })?),
    };
    let exchange = match param(query, "exchange").as_str() {
        "" => None,
        word => Some(Exchange::parse(word).map_err(|why| {
            bad(format!(
                "exchange {:?}: {}",
                echo(word),
                echo(&why.to_string())
            ))
        })?),
    };
    if exchange.is_some() && underlying.is_none() {
        return Err(bad(
            "exchange needs an underlying: the segment's roll-up is every exchange's".to_owned(),
        ));
    }
    Ok(Asked {
        feed,
        segment,
        exchange,
        underlying,
    })
}

/// The IST epoch day of a stamp in microseconds.
const fn ist_day(micros: i64) -> i64 {
    micros
        .div_euclid(1_000_000)
        .saturating_add(pull::session::IST_OFFSET_SECS)
        .div_euclid(86_400)
}

/// Sessions in `first ..= last` by the calendar of record, or `None` when a
/// day of it is outside the record (or the range is empty). At most one
/// lookup per day; every caller passes days of one month.
fn sessions(first: i64, last: i64) -> Option<u32> {
    if first > last {
        return None;
    }
    let mut count = 0_u32;
    for day in first..=last {
        match pull::calendar::sourced_kind_of(day) {
            pull::calendar::DayKind::Open(_) | pull::calendar::DayKind::OpenLengthUnmeasured => {
                count = count.saturating_add(1);
            }
            pull::calendar::DayKind::Closed => {}
            pull::calendar::DayKind::Unmeasured => return None,
        }
    }
    Some(count)
}

/// The first and last epoch day of `month`, or `None` past the calendar's
/// types (never, for a month a census key names).
fn month_days(month: YearMonth) -> Option<(i64, i64)> {
    let first = pull::session::Day::new(month.year(), month.month(), 1).ok()?;
    Some((
        i64::from(first.days_from_epoch()),
        i64::from(first.end_of_month().days_from_epoch()),
    ))
}

/// The sessions the calendar owes in `month`. See [`DAYS_OWED_BASIS`].
pub(crate) fn days_owed(month: YearMonth) -> Option<u32> {
    let (first, last) = month_days(month)?;
    sessions(first, last)
}

/// The sessions `cell`'s stamps span inside `month`. See [`DAYS_HELD_BASIS`].
pub(crate) fn days_held(month: YearMonth, cell: &MonthCell) -> Option<u32> {
    let (from, to) = month_days(month)?;
    let (first, last) = (cell.first?, cell.last?);
    sessions(ist_day(first).max(from), ist_day(last).min(to))
}

/// `null` or the number.
fn number<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(|| "null".to_owned(), |n| n.to_string())
}

/// One rung's totals over the months it holds.
#[derive(Debug, Default)]
struct Rung {
    months: u64,
    keys: u64,
    rows: u64,
    first: Option<i64>,
    last: Option<i64>,
    held: Option<u64>,
    owed: Option<u64>,
}

/// The body: the selection, the stated bases and order, the per-rung totals
/// and one row per (month, rung), newest month first.
fn body(
    asked: &Asked,
    exchange: Option<Exchange>,
    cells: &Cells,
    generation: u64,
    state: &str,
    degraded: Option<&str>,
) -> String {
    let mut owed_of: BTreeMap<YearMonth, Option<u32>> = BTreeMap::new();
    let mut rungs: BTreeMap<Timeframe, Rung> = BTreeMap::new();
    let mut months = String::with_capacity(cells.len().saturating_mul(160));
    // NEWEST MONTH FIRST, FINEST RUNG FIRST WITHIN IT: the cells are kept
    // oldest month first, so a stable sort on the month alone, reversed,
    // keeps each month's rungs in their own order. At most `CELLS_MOST`.
    let mut ordered: Vec<(&(YearMonth, Timeframe), &MonthCell)> = cells.iter().collect();
    ordered.sort_by_key(|&(&(month, _), _)| std::cmp::Reverse(month));
    for (n, (&(month, rung), cell)) in ordered.into_iter().enumerate() {
        let owed = *owed_of.entry(month).or_insert_with(|| days_owed(month));
        let held = days_held(month, cell);
        let total = rungs.entry(rung).or_insert_with(|| Rung {
            held: Some(0),
            owed: Some(0),
            ..Rung::default()
        });
        total.months = total.months.saturating_add(1);
        total.keys = total.keys.saturating_add(cell.keys);
        total.rows = total.rows.saturating_add(cell.rows);
        total.first = match (total.first, cell.first) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        total.last = total.last.max(cell.last);
        // A TOTAL OVER A MONTH THE CALENDAR CANNOT COUNT IS NOT A TOTAL:
        // `null` from the first such month on, never a sum that skipped it.
        total.held = total.held.zip(held).map(|(a, b)| a + u64::from(b));
        total.owed = total.owed.zip(owed).map(|(a, b)| a + u64::from(b));
        if n > 0 {
            months.push(',');
        }
        let _ = write!(
            months,
            r#"{{"month":"{month}","timeframe":{},"instrument_months":{},"rows":{},"first_ts":{},"last_ts":{},"days_held":{},"days_owed":{}}}"#,
            render::json_string(rung.as_str()),
            cell.keys,
            cell.rows,
            number(cell.first),
            number(cell.last),
            number(held),
            number(owed),
        );
    }
    let mut out = format!(
        r#"{{"schema":1,"feed":{},"segment":{},"exchange":{},"underlying":{},"state":{},"degraded":{},"generation":{generation},"order":{},"basis":{{"days_owed":{},"days_held":{}}},"rungs":["#,
        render::json_string(asked.feed.as_str()),
        render::json_string(asked.segment.as_str()),
        exchange.map_or_else(|| "null".to_owned(), |e| render::json_string(e.as_str())),
        asked
            .underlying
            .map_or_else(|| "null".to_owned(), |s| render::json_string(s.as_str())),
        render::json_string(state),
        degraded.map_or_else(|| "null".to_owned(), render::json_string),
        render::json_string(ORDER),
        render::json_string(DAYS_OWED_BASIS),
        render::json_string(DAYS_HELD_BASIS),
    );
    for (n, (rung, total)) in rungs.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"timeframe":{},"months_held":{},"instrument_months":{},"rows":{},"first_ts":{},"last_ts":{},"days_held":{},"days_owed":{}}}"#,
            render::json_string(rung.as_str()),
            total.months,
            total.keys,
            total.rows,
            number(total.first),
            number(total.last),
            number(total.held),
            number(total.owed),
        );
    }
    out.push_str(r#"],"months":["#);
    out.push_str(&months);
    out.push_str("]}");
    out
}

/// Which exchange holds `symbol` in the index, when the request named none:
/// one probe per exchange. `Err` names none, or every one that does.
fn resolve(
    index: &super::facets::Index,
    symbol: Symbol,
    asked: Option<Exchange>,
) -> Result<Exchange, Vec<Exchange>> {
    if let Some(exchange) = asked {
        return if index.holds(exchange, symbol) {
            Ok(exchange)
        } else {
            Err(Vec::new())
        };
    }
    let holders: Vec<Exchange> = EXCHANGES
        .into_iter()
        .filter(|&exchange| index.holds(exchange, symbol))
        .collect();
    match holders.as_slice() {
        [one] => Ok(*one),
        _ => Err(holders),
    }
}

/// `404 not_held`: the census holds no such underlying in the segment (on
/// the exchange asked, where one was).
fn not_held(
    index: &super::facets::Index,
    asked: &Asked,
    symbol: Symbol,
) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::NOT_FOUND,
        error_json(
            "not_held",
            &format!(
                "{}{} is not an underlying the {} census holds in {} (generation {})",
                asked
                    .exchange
                    .map_or_else(String::new, |e| format!("{} ", e.as_str())),
                symbol.as_str(),
                asked.feed.as_str(),
                asked.segment.as_str(),
                index.generation()
            ),
        ),
    )
}

/// The answer to one request: status, headers, body.
fn answered(
    index: &super::facets::Index,
    asked: &Asked,
    state: &str,
    degraded: Option<&str>,
) -> (axum::http::StatusCode, String) {
    let (exchange, cells) = match asked.underlying {
        None => (None, index.segment_cells()),
        Some(symbol) => match resolve(index, symbol, asked.exchange) {
            // HELD ON THAT EXCHANGE, read by the probe `resolve` just made:
            // never the segment's cells in its place.
            Ok(exchange) => match index.underlying_cells(exchange, symbol) {
                Some(cells) => (Some(exchange), cells),
                None => return not_held(index, asked, symbol),
            },
            Err(holders) if holders.len() > 1 => {
                return (
                    axum::http::StatusCode::CONFLICT,
                    error_json(
                        "exchange_ambiguous",
                        &format!(
                            "{} is an underlying of {} {} on {}; name one with exchange=",
                            symbol.as_str(),
                            asked.feed.as_str(),
                            asked.segment.as_str(),
                            holders
                                .iter()
                                .map(|e| e.as_str())
                                .collect::<Vec<_>>()
                                .join(" and ")
                        ),
                    ),
                );
            }
            Err(_) => return not_held(index, asked, symbol),
        },
    };
    if cells.len() > CELLS_MOST {
        return (
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            error_json(
                "rollup_too_many",
                &format!(
                    "the selection holds {} (month, rung) cells; one answer carries at most \
                     {CELLS_MOST}: name an underlying",
                    cells.len()
                ),
            ),
        );
    }
    (
        axum::http::StatusCode::OK,
        body(asked, exchange, cells, index.generation(), state, degraded),
    )
}

/// JSON, never cached by a browser: the roll-up moves with the census.
fn json_headers(headers: &mut axum::http::HeaderMap) {
    let _type = headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    let _cache = headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
}

/// The whole answer for `query`, on the thread that calls it.
pub(super) fn answer(
    site: &Site,
    query: &str,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    let asked = match asked(query) {
        Ok(asked) => asked,
        Err((code, body)) => {
            let mut headers = axum::http::HeaderMap::new();
            json_headers(&mut headers);
            return (code, headers, body);
        }
    };
    let (_stamps, now) = vendor_census(site, asked.feed);
    let census = now.census();
    let behind = super::store_route::behind(site, asked.feed);
    let mut headers = super::store_route::census_headers_with(census, Some(&behind));
    json_headers(&mut headers);
    // HOW CURRENT: a roll-up built from the previous read while a background
    // re-read runs says so, and how old it is (R3-09).
    super::census_cache::freshness_headers(&mut headers, now.freshness());
    if super::store_route::census_is_unreadable(census) {
        let why = census.map_or_else(String::new, census::VendorCensus::note);
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            headers,
            error_json("census_unreadable", &why),
        );
    }
    let state = census.map_or("absent", |census| census.state_word(behind.days));
    let degraded = census.and_then(census::VendorCensus::degraded);
    // AN INDEX BEING BUILT IS NAMED, NOT WAITED FOR (R3-09, RA-046): the
    // first roll-up after a restart waited 5.6 s on average for the warm's
    // FNO build over 20 restarts of the shadow. It now answers "warming" at
    // once, and while a rebuild runs, from the previous index, saying so.
    let built =
        super::facets::index_or_previous(site, asked.feed, asked.segment, now.slot(), |index| {
            (
                answered(index, &asked, state, degraded.as_deref()),
                index.generation(),
            )
        });
    let (((code, body), generation), how) = match built {
        Ok(built) => built,
        Err(since_ms) => {
            let (code, body) =
                super::facets::warming_answer(&mut headers, asked.feed, asked.segment, since_ms);
            return (code, headers, body);
        }
    };
    super::census_cache::freshness_headers(&mut headers, how.freshness(now.freshness()));
    let _generation = headers.insert(
        axum::http::HeaderName::from_static(super::store_route::CENSUS_PART_GENERATION_HEADER),
        axum::http::HeaderValue::from(generation),
    );
    (code, headers, body)
}

/// `GET /census/rollup.json`: [`answer`], off the async workers (DB-G80): a
/// cold index is built under its lock, and a vendor not yet warmed is read
/// once. A full pool is a named `429` with `Retry-After`.
pub(super) async fn rollup_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    let query = uri.query().unwrap_or("").to_owned();
    match crate::detail::run_store_read(move || answer(&site, &query)).await {
        Ok(answered) => answered,
        Err(why) => {
            let saturated = matches!(why, crate::detail::RunError::Saturated);
            let mut headers = axum::http::HeaderMap::new();
            json_headers(&mut headers);
            if saturated {
                let _retry = headers.insert(
                    axum::http::header::RETRY_AFTER,
                    axum::http::HeaderValue::from_static("1"),
                );
            }
            (
                if saturated {
                    axum::http::StatusCode::TOO_MANY_REQUESTS
                } else {
                    axum::http::StatusCode::SERVICE_UNAVAILABLE
                },
                headers,
                error_json("census_read_unavailable", &format!("{why:?}")),
            )
        }
    }
}
