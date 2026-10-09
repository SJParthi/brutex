//! `/bars.json` and `/bars/window.json` -- one month of bars, and one page
//! of bars across a range of months, with the browser-exactness rule both
//! share.
//!
//! # Moved, not rewritten
//!
//! Every item below was cut out of `server.rs` verbatim by the WP-01 seam and
//! is mounted there as a child module, so it reads that file's private
//! helpers through `super::` exactly as it did when it lived in it, and
//! `server.rs` imports each name back. The only edits are visibility
//! qualifiers (`pub(super)`) on items another module now has to reach.
//! No route answers differently because of the move.

use super::{Addressed, Loaded, param, timeframe_param};
use crate::{bars, ingest, render};
use std::fmt::Write as _;

/// The bars of one month as a JSON array, narrowed to a half-open microsecond
/// window.
///
/// `None` on either bound means no bound, so the un-windowed call is the whole
/// month exactly as it always was. The window is half-open — `>= from`,
/// `< to` — because the caller builds `to` as the start of the day AFTER the
/// one asked for, which is how an inclusive day is expressed without spelling
/// `23:59:59.999999` and hoping the last bar sits under it.
///
/// Extracted from [`bars_json`] because that function is at clippy's line
/// ceiling, and because the emit is the part worth reading on its own: it is
/// the only place a bar becomes bytes on this route.
pub(super) fn bars_array(
    rows: &[store::format::Bar],
    from: Option<i64>,
    to: Option<i64>,
) -> (String, Vec<String>) {
    let inside = |bar: &store::format::Bar| {
        from.is_none_or(|at| bar.ts_micros >= at) && to.is_none_or(|at| bar.ts_micros < at)
    };
    let mut out = String::with_capacity(rows.len() * 96 + 32);
    let mut withheld = Vec::new();
    out.push('[');
    let mut written = 0usize;
    for bar in rows.iter().filter(|bar| inside(bar)) {
        // A VALUE A BROWSER CANNOT HOLD IS WITHHELD BY NAME (CE-60, D-2687).
        if let Some(why) = inexact_bar(bar) {
            withheld.push(why);
            continue;
        }
        if written > 0 {
            out.push(',');
        }
        written = written.saturating_add(1);
        // `lightweight-charts` takes UTC seconds. The store holds micros.
        let _ = write!(
            out,
            r#"{{"t":{},"o":{},"h":{},"l":{},"c":{},"v":{},"oi":{}}}"#,
            bar.ts_micros / 1_000_000,
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume,
            if bar.open_interest == store::format::OI_NULL {
                "null".to_owned()
            } else {
                bar.open_interest.to_string()
            }
        );
    }
    out.push(']');
    (out, withheld)
}

/// The largest magnitude a browser's JSON number holds exactly: 2^53 − 1.
const BROWSER_EXACT: i64 = (1 << 53) - 1;

/// Whether `value` survives `JSON.parse` as itself.
const fn browser_exact(value: i64) -> bool {
    value.unsigned_abs() <= BROWSER_EXACT.unsigned_abs()
}

/// The fault naming the first field of `bar` a browser's JSON number cannot
/// hold exactly, or `None` when every field is exact.
///
/// The store admits any non-negative `i64` count and price, and both bar
/// routes write them as bare numbers: `9007199254740993` would reach a page as
/// `9007199254740992`, and `/db` and `/markets` check nothing. The other routes
/// either quote such values or refuse them; these two keep their wire shape and
/// withhold the bar, named in the `faults` they already carry, so every page is
/// covered by one rule at the server (CE-60, D-2687).
fn inexact_bar(bar: &store::format::Bar) -> Option<String> {
    let interest = (bar.open_interest != store::format::OI_NULL).then_some(bar.open_interest);
    [
        ("o", Some(bar.open)),
        ("h", Some(bar.high)),
        ("l", Some(bar.low)),
        ("c", Some(bar.close)),
        ("v", Some(bar.volume)),
        ("oi", interest),
    ]
    .into_iter()
    .find_map(|(field, value)| {
        value
            .filter(|&held| !browser_exact(held))
            .map(|held| inexact_fault(bar.ts_micros, field, held))
    })
}

/// [`inexact_bar`]'s sentence, shared with the window's change fields.
fn inexact_fault(ts_micros: i64, field: &str, value: i64) -> String {
    format!(
        "the bar at {ts_micros} UTC micros carries `{field}` = {value}, outside \
         ±(2^53 − 1), which a browser's JSON number cannot hold exactly; the bar \
         is withheld rather than sent to be rounded"
    )
}

/// An optional ISO day as midnight IST in UTC microseconds.
///
/// IST is UTC+05:30, so an IST day BEGINS `IST_OFFSET_SECS` before the UTC
/// instant of the same date. Getting that sign wrong shifts every window by
/// five and a half hours, which on a 09:15–15:29 session silently drops the
/// morning and admits the previous evening.
///
/// An absent or empty bound is unbounded. A supplied malformed bound refuses
/// by field name: returning the whole month would answer a different request.
fn ist_midnight_micros(field: &'static str, text: &str) -> Result<Option<i64>, String> {
    if text.is_empty() {
        return Ok(None);
    }
    let day = ingest::parse_day(field, text).map_err(|why| why.to_string())?;
    Ok(Some(
        (i64::from(day.days_from_epoch()) * 86_400 - pull::session::IST_OFFSET_SECS) * 1_000_000,
    ))
}

/// Inclusive civil-day bounds represented as a half-open timestamp window.
/// Validate before opening the addressed source; a reversed range is a request
/// error, while a valid range outside the file may still have no rows.
pub(super) fn day_window_bounds(query: &str) -> Result<(Option<i64>, Option<i64>), String> {
    let from = ist_midnight_micros("from", &param(query, "from"))?;
    let to = ist_midnight_micros("to", &param(query, "to"))?;
    if from.zip(to).is_some_and(|(from, to)| from > to) {
        return Err("from must be on or before to".to_owned());
    }
    // Four-digit ISO years fit in i64 microseconds, including the next midnight.
    Ok((from, to.map(|at| at + 86_400 * 1_000_000)))
}

/// One instrument-month of bars, as JSON, for the chart.
///
/// `?feed=<wire>&exchange=NSE&segment=INDEX&symbol=NIFTY&month=YYYY-MM`
///
/// # Paisa all the way out
///
/// Prices leave here as the `i64` paisa they are stored as. The browser divides
/// by 100 exactly once, where a canvas needs a number to draw — `CLAUDE.md` §7
/// says a float has no business near a price, and this endpoint keeps that true
/// right up to the pixel.
///
/// # A positional read per bar, and the whole month is one pass
///
/// `BarFile::read_record` reaches bar N by arithmetic — the header length plus
/// `N × record_stride`, one multiply and one add — and then reads exactly one
/// record's bytes. The records before it are not touched, so the *work* of
/// reaching the last bar of a month is the work of reaching the first.
/// `store::geometry::addressing_is_arithmetic_and_flat` asserts that across a
/// long walk: every index's offset is exactly one stride past the one before
/// it, at index 0 and at 400,000 alike.
///
/// This heading read **"O(1) per bar"** and the sentence under it said "the
/// cost of the last bar equals the cost of the first". The arithmetic is
/// constant and that is what is named above. The `pread` under it has never
/// been timed here, and a page cache is not a bound — so the claim is the
/// shape, not the nanoseconds, and it says which.
///
/// A month is at most 375 × ~22 bars and is sent whole: the chart pans and
/// zooms locally after that, with no request per viewport change.
///
/// `open_interest` is `i64::MIN` when the vendor sent none — the null sentinel
/// §7 reserves. It becomes JSON `null` rather than a number, because zero means
/// zero and writing 0 for "not sent" is a lie in the data.
pub(super) async fn bars_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let query = uri.query().unwrap_or("");
    // A fresh header array per return, because `HeaderName` is not `Copy` and
    // one binding cannot be moved into three arms.
    let json = || {
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )]
    };
    let refuse = |why: String| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            json(),
            format!(r#"{{"error":{}}}"#, render::json_string(&why)),
        )
    };

    let asked = match Addressed::parse(query) {
        Ok(asked) => asked,
        Err(why) => return refuse(why),
    };

    // AN OPTIONAL DAY WINDOW, AND WITHOUT IT THIS ENDPOINT COULD ONLY ANSWER
    // "THE WHOLE MONTH".
    //
    // `/db` shows one row per stored bar and reads this endpoint once per
    // instrument-month. Asking it for a SINGLE DAY was therefore ~549 requests
    // of a full month each — the page's own measurement is 81 bytes a bar, so a
    // one-minute month is ~670 KB and the day the operator asked for is ~2% of
    // it. The browser downloaded, parsed and threw away the other 98%, which is
    // what "the page is stuck" was.
    //
    // The bound is INCLUSIVE at both ends and stated in IST days, because that
    // is what the operator picked and what `Day` spells. Missing bounds remain
    // unbounded; malformed or reversed bounds refuse before source-file I/O.
    let (from_micros, to_micros) = match day_window_bounds(query) {
        Ok(bounds) => bounds,
        Err(why) => return refuse(why),
    };
    let file = match asked.open(&site, asked.month) {
        Ok(file) => file,
        Err(why) => return refuse(why),
    };

    // THE WINDOW IS ADDRESSED, NOT FILTERED, and it used to be filtered.
    //
    // This read the WHOLE month — `page(&file, 0, n_valid)` — and handed the
    // day bounds to `bars_array` afterwards. So a one-day query against a
    // one-minute month read ~8,250 records, one `pread` each, to return ~375.
    // An O(1) audit measured the shape as "reads every bar, filters after", on
    // an endpoint `/db` calls once per instrument-month.
    //
    // Bars in a file are strictly increasing by timestamp — the writer refuses
    // otherwise — so the start of the window is addressable. Through the
    // month's `.tix` time index that is one 16-byte entry read (D-2329); a
    // month with no usable index falls back, loudly, to the D-1434 bisection
    // in `log2(n_valid)` reads — fourteen at 8,250 bars instead of 8,250.
    //
    // The END is still found by reading forward and stopping, rather than by a
    // second bisection: the rows have to be read to be returned, so a second
    // search would buy nothing. What it saves is everything BEFORE the window.
    //
    // A bisection that refuses uses the full read path with the same validated
    // date filter. The page reader retains healthy rows and reports source
    // faults; an optimisation failure never removes the requested bounds.
    let held = usize::try_from(file.header().n_valid).unwrap_or(usize::MAX);
    // A BISECTION THAT LANDS PAST THE END FALLS BACK TO THE WHOLE MONTH.
    //
    // `first_at_or_after` bisects on each record's stamp and assumes the file is
    // strictly increasing -- which the writer enforces, and which `file.rs`'s own
    // `initialise` doc records ONE exception to: a header can name records whose
    // bytes are zeros from a newly allocated extent, and "nothing in the record
    // can detect it. An all-zero `Bar` satisfies `ohlc_is_sane`."
    //
    // An all-zero bar stamps at 0, which is below every real `from`, so the
    // bisection walks upward THROUGH the zero region and returns `n_valid`. That
    // gave `take = 0`, `page` read nothing, `faults` was empty, and the endpoint
    // answered **200 with `[]`** for a day that has 375 bars -- turning a correct
    // answer into a silent empty one, with the fault-reporting path unable to
    // fire because no record was read. The code this replaced did not depend on
    // monotonicity and was correct on such a file.
    //
    // Landing at or past the end is therefore treated as "the bisection could
    // not help" rather than as "the window is empty": read the month and let
    // `bars_array` filter, which is exactly the previous behaviour. A window
    // genuinely past the last bar costs one wasted pass and still answers `[]`
    // -- the same answer, arrived at by reading rather than by assuming.
    let begins = from_micros
        .and_then(|at| file.first_at_or_after(at).ok())
        .and_then(|index| usize::try_from(index).ok())
        .filter(|&index| index < held)
        .unwrap_or(0);
    // THE END IS BISECTED TOO, and the first version said it did not need to be.
    //
    // That commit argued "those rows have to be read to be returned, so a second
    // bisection would buy nothing". Both halves are wrong: nothing stopped at
    // `to`, and `bars_array` DISCARDS every row past it. So a one-day query
    // still read from the window's start to the end of the month -- the whole
    // month for day 1, half of it on average. An adversarial pass measured it:
    // 8,250 records read to return 375 on the first day, unchanged from before.
    //
    // A second lookup is one more entry read (fourteen record reads on the
    // bisection path) and it bounds what the first one only started. Same failure rule as the start: a bisection that cannot
    // answer leaves the bound where it was, because the window is an optional
    // narrowing and `bars_array` filters correctly either way.
    let ends = to_micros
        .and_then(|at| file.first_at_or_after(at).ok())
        .and_then(|index| usize::try_from(index).ok())
        .map_or(held, |index| index.clamp(begins, held));
    let (rows, mut faults) = bars::page(&file, begins, ends.saturating_sub(begins));

    let (out, mut withheld) = bars_array(&rows, from_micros, to_micros);
    faults.append(&mut withheld);

    // A FAULTY RECORD IS NOT SILENTLY SKIPPED. `page` returns what it could
    // read and what it could not; dropping the second half would draw a chart
    // with a hole in it and no way to know.
    if !faults.is_empty() {
        return (
            axum::http::StatusCode::PARTIAL_CONTENT,
            json(),
            format!(
                r#"{{"bars":{out},"faults":{}}}"#,
                render::json_string(&faults.join("; "))
            ),
        );
    }
    (axum::http::StatusCode::OK, json(), out)
}

/// One page of bars across a RANGE of months, answered in ONE request.
///
/// # The 2,187 requests this replaces
///
/// `/bars.json` serves ONE instrument-month, which is the right shape for a
/// chart of one month and the wrong shape for a grid over a store. The grid
/// fanned out one request per matched instrument-month — measured on a store
/// holding 2019-12..2026-08 for three instruments at nine rungs, that is
/// **2,187 requests**, past what a browser will open, and every one of them
/// failed `net::ERR_INSUFFICIENT_RESOURCES`.
///
/// Paging by time was then fixed in the browser by reading the census counts
/// and opening only the file the page sits in — one request, any depth. What
/// that could NOT fix is a sort on a price column: no index answers "the fifty
/// largest closes", so the grid fetched every row of every month and sorted
/// them in JavaScript. Measured: **76 requests and 9.4 seconds**, and the cost
/// was never the network — it was parsing 623,498 rows into objects to throw
/// away all but fifty.
///
/// This route answers the same question by reading the files it already has on
/// local disk and returning the fifty. `bars::window` seeks when the order is
/// the store's own and scans when it is not, and says which it did.
///
/// # Why the extremes ride along
///
/// The grid scales its magnitude bars against the widest range in the QUERY so
/// that turning a page cannot change what a full bar means. That property was
/// lost when the browser stopped reading every month. `extremes=1` restores it
/// for the cost of the scan it already takes — one number, computed where the
/// bytes are.
/// Everything `/bars/window.json` reads off the query string.
///
/// Parsed as one step so the handler is dispatch rather than a wall of
/// `match … return refuse`. Every field REFUSES rather than defaulting where a
/// default would answer a different question than the one asked.
pub(super) struct WindowAsk {
    pub(super) vendor: brutex_core::vendor::Vendor,
    pub(super) timeframe: store::path::Timeframe,
    pub(super) contract: Option<brutex_core::instrument::Contract>,
    pub(super) from: store::path::YearMonth,
    pub(super) to: store::path::YearMonth,
    pub(super) sort: bars::SortKey,
    pub(super) desc: bool,
    pub(super) offset: usize,
    pub(super) limit: usize,
    pub(super) want_extremes: bool,
}

impl WindowAsk {
    /// Reads one, or names the first thing wrong with it.
    pub(super) fn parse(query: &str) -> Result<Self, String> {
        let Some(vendor) = ingest::parse_vendor(&param(query, "feed")) else {
            return Err(format!(
                "{:?} is not a feed this build can read",
                param(query, "feed")
            ));
        };
        let month_of = |key: &str| {
            let raw = param(query, key);
            raw.split_once('-')
                .and_then(|(y, m)| {
                    store::path::YearMonth::new(y.parse().ok()?, m.parse().ok()?).ok()
                })
                .ok_or_else(|| format!("{raw:?} is not a YYYY-MM month for {key:?}"))
        };
        let from = month_of("from")?;
        let to = month_of("to")?;
        let timeframe = timeframe_param(query)?;
        let raw_contract = param(query, "contract");
        let contract = if raw_contract.is_empty() {
            None
        } else {
            Some(
                brutex_core::instrument::Contract::parse(&raw_contract).ok_or_else(|| {
                    format!("{raw_contract:?} is not a contract segment this store can name")
                })?,
            )
        };
        let sort = bars::SortKey::parse(&param(query, "sort")).ok_or_else(|| {
            format!(
                "{:?} is not a column this grid sorts on. Accepted: ts, o, h, l, c, v, oi.",
                param(query, "sort")
            )
        })?;
        // A NON-NUMBER IS A REFUSAL, NOT A ZERO. `offset=banana` silently
        // reading from the top would answer a different question than the one
        // asked, and the pager would look right while showing the wrong rows.
        let number = |key: &str, fallback: usize| {
            let raw = param(query, key);
            if raw.is_empty() {
                Ok(fallback)
            } else {
                raw.parse::<usize>()
                    .map_err(|_| format!("{raw:?} is not a whole number for {key:?}"))
            }
        };
        // ABSENT MEANS DESCENDING, which is what the grid opens on and what a
        // reader of a store asks for first: the newest row. Anything else that
        // is not `asc` or `desc` is refused: `dir=ASC` once answered newest
        // first with a 200 (Z1-slice14-F3, D-1762).
        let desc = match param(query, "dir").as_str() {
            "" | "desc" => true,
            "asc" => false,
            other => {
                return Err(format!(
                    "{other:?} is not a direction. Accepted: asc, desc."
                ));
            }
        };
        // Same rule: `extremes=yes` once dropped the extremes silently.
        let want_extremes = match param(query, "extremes").as_str() {
            "" | "0" | "false" => false,
            "1" | "true" => true,
            other => {
                return Err(format!(
                    "{other:?} is not an extremes flag. Accepted: 0, 1, false, true."
                ));
            }
        };
        Ok(Self {
            vendor,
            timeframe,
            contract,
            from,
            to,
            sort,
            desc,
            offset: number("offset", 0)?,
            // A LIMIT OUTSIDE 1..=MAX IS REFUSED, NOT CLAMPED. `limit=5000`
            // answered 1,000 rows under a 200 and a pager stepping by what it
            // asked skipped 4,000 of them; `limit=0` answered an empty page
            // beside a non-zero total (P1-01-01, P1-02-02, D-1765).
            limit: match number("limit", bars::PAGE_BARS)? {
                limit @ 1..=bars::MAX_WINDOW_LIMIT => limit,
                other => {
                    return Err(format!(
                        "{other} is not a page size this grid serves. Accepted: 1 to {}.",
                        bars::MAX_WINDOW_LIMIT
                    ));
                }
            },
            want_extremes,
        })
    }
}

pub(super) async fn bars_window_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let query = uri.query().unwrap_or("");
    let json = || {
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )]
    };
    let refuse = |why: String| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            json(),
            format!(r#"{{"error":{}}}"#, render::json_string(&why)),
        )
    };

    let asked = match WindowAsk::parse(query) {
        Ok(asked) => asked,
        Err(why) => return refuse(why),
    };
    let window = match bars::window(
        &site.store_root,
        asked.vendor,
        &param(query, "exchange"),
        &param(query, "segment"),
        &param(query, "symbol"),
        asked.timeframe,
        asked.contract,
        asked.from,
        asked.to,
        asked.sort,
        asked.desc,
        asked.offset,
        asked.limit,
        asked.want_extremes,
    ) {
        Ok(window) => window,
        Err(why) => return refuse(why),
    };
    let (sort, want_extremes) = (asked.sort, asked.want_extremes);

    let (body, withheld) = render_window(&window, sort.scans() || want_extremes);
    // PARTIAL CONTENT WHEN A RECORD WOULD NOT READ, the same status
    // `/bars.json` answers with, for the same reason: the rows are real and
    // the set is not whole, and one status must not mean both.
    let status = if window.faults.is_empty() && withheld == 0 {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::PARTIAL_CONTENT
    };
    (status, json(), body)
}

/// A [`bars::Window`] as the JSON the grid reads.
///
/// `scanned` IS ON THE WIRE BECAUSE THE TWO PATHS COST DIFFERENT THINGS. A
/// reader that cannot tell a seek from a scan cannot tell why one sort is
/// instant and another is not, and would reasonably call the slow one a bug.
///
/// `months_missing` is on it for the same reason: a sparse store is legal, and
/// a reader must be able to tell a gap from a refusal — `CLAUDE.md` §4.
///
/// Returns the body and how many bars or extremes were withheld because a
/// browser's JSON number cannot hold one of their values (CE-60, D-2687); the
/// caller answers 206 when that is not zero, as for a record that would not
/// read. Each is named in `faults`.
pub(super) fn render_window(window: &bars::Window, scanned: bool) -> (String, usize) {
    let mut rows = String::with_capacity(window.bars.len() * 128 + 32);
    let mut withheld: Vec<String> = Vec::new();
    rows.push('[');
    let mut written = 0usize;
    for row in &window.bars {
        let bar = &row.bar;
        let change = [("chg", row.chg), ("oichg", row.oichg)]
            .into_iter()
            .find_map(|(field, bps)| {
                bps.filter(|&held| !browser_exact(held))
                    .map(|held| inexact_fault(bar.ts_micros, field, held))
            });
        if let Some(why) = inexact_bar(bar).or(change) {
            withheld.push(why);
            continue;
        }
        if written > 0 {
            rows.push(',');
        }
        written = written.saturating_add(1);
        // EXACTLY ONE OF EACH PAIR IS NON-NULL, on every row, the same contract
        // `/store.json`'s `chg_bps`/`chg_why` already keeps. An unknown change
        // is never `0` — zero is a real bar that closed where the last one did.
        let value = |bps: Option<i64>| bps.map_or_else(|| "null".to_owned(), |n| n.to_string());
        let why = |bps: Option<i64>, reason: &str| {
            if bps.is_some() {
                "null".to_owned()
            } else {
                render::json_string(reason)
            }
        };
        let _ = write!(
            rows,
            r#"{{"t":{},"o":{},"h":{},"l":{},"c":{},"v":{},"oi":{},"chg":{},"chg_why":{},"oichg":{},"oichg_why":{}}}"#,
            bar.ts_micros / 1_000_000,
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume,
            if bar.open_interest == store::format::OI_NULL {
                "null".to_owned()
            } else {
                bar.open_interest.to_string()
            },
            value(row.chg),
            why(row.chg, row.chg_why),
            value(row.oichg),
            why(row.oichg, row.oichg_why)
        );
    }
    rows.push(']');

    let extremes = window.extremes.map_or_else(
        || "null".to_owned(),
        |top| {
            if browser_exact(top.range) && browser_exact(top.volume) {
                format!(r#"{{"range":{},"volume":{}}}"#, top.range, top.volume)
            } else {
                withheld.push(format!(
                    "the window's extremes (range {}, volume {}) lie outside \
                     ±(2^53 − 1), which a browser's JSON number cannot hold \
                     exactly; withheld as null",
                    top.range, top.volume
                ));
                "null".to_owned()
            }
        },
    );
    let mut faults: Vec<&str> = window.faults.iter().map(String::as_str).collect();
    faults.extend(withheld.iter().map(String::as_str));
    let body = format!(
        r#"{{"total":{},"months_read":{},"months_missing":{},"scanned":{scanned},"extremes":{extremes},"faults":{},"bars":{rows}}}"#,
        window.total,
        window.months_read,
        window.months_missing,
        if faults.is_empty() {
            "null".to_owned()
        } else {
            render::json_string(&faults.join("; "))
        }
    );
    (body, withheld.len())
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;
    use crate::server::Site;
    use crate::server::tests::{masters, store_root};
    use brutex_core::vendor::Vendor;

    /// **`/bars.json` NARROWS TO A DAY WINDOW, AND WITHOUT IT THE PAGE COULD
    /// ONLY ASK FOR A MONTH.**
    ///
    /// `/db` reads this endpoint once per instrument-month and shows one row
    /// per stored bar. Asking it for a SINGLE DAY was ~549 requests of a full
    /// month each; at the page's own measured 81 bytes a bar that is about
    /// 670 KB per one-minute month, of which the chosen day is roughly 2%. The
    /// browser downloaded, parsed and discarded the other 98%. The operator
    /// reported it as the page being stuck; it was arithmetic.
    ///
    /// Three properties, and each is a separate way to get this wrong:
    /// the window is **inclusive at both ends** (a `to` day whose bars fell
    /// outside would silently lose the day the operator asked for); an
    /// **absent** bound means no bound (or every existing caller breaks); and a
    /// **malformed** bound refuses rather than silently widening the request.
    #[tokio::test]
    async fn bars_json_narrows_to_the_day_window_and_refuses_a_malformed_one() {
        let root = store_root("barsday");
        // THE MONTH AND THE STAMPS AGREE. `19_723` is 1970-01-01 plus 19,723
        // days, which is 2024-01-01, so the path says January 2024 and the bars
        // are in it. A path naming one month over bars stamped in another would
        // still have passed this filter — and would have quietly encoded that
        // the two need not match.
        let month = store::path::YearMonth::new(2024, 1).expect("a legal month");
        // Three days, one bar each, at midday IST so no boundary is grazed.
        let day_micros = |d: i64| ((19_723 + d) * 86_400 - 19_800 + 6 * 3_600) * 1_000_000;
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: store::path::Timeframe::MINUTE_1,
            month,
            file: store::path::FileKind::Bars,
        })
        .expect("a legal path");
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the id is the cross-check `open` folds; any 32 bits serve"
        )]
        let symbol_id = brutex_core::universe::fnv1a("NIFTY") as u32;
        let mut file =
            store::file::BarFile::open_or_create(&root, path, symbol_id).expect("a bar file");
        let rows: Vec<store::format::Bar> = (0..3)
            .map(|d| store::format::Bar {
                ts_micros: day_micros(d),
                open: 100 + d,
                high: 110 + d,
                low: 90 + d,
                close: 105 + d,
                volume: 1,
                open_interest: i64::MIN,
            })
            .collect();
        file.append(&rows).expect("three legal bars");
        drop(file);

        let site = Site::serving(&masters("barsday", None, None), &root);
        let loaded = std::sync::Arc::new(site);
        let read = |q: &str| {
            let site = std::sync::Arc::clone(&loaded);
            let uri: axum::http::Uri = format!(
                "/bars.json?feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY\
                 &timeframe=1min&month=2024-01{q}"
            )
            .parse()
            .expect("a uri");
            async move { bars_json(axum::extract::State(site), uri).await }
        };

        let (_, _, all) = read("").await;
        assert_eq!(all.matches("\"t\":").count(), 3, "no window is every bar");

        // ONE DAY, THE MIDDLE ONE. Both neighbours must be excluded, which is
        // what proves the window is a window and not a floor.
        let (_, _, one) = read("&from=2024-01-02&to=2024-01-02").await;
        assert_eq!(
            one.matches("\"t\":").count(),
            1,
            "exactly the day asked for: {one}"
        );
        assert!(one.contains("\"o\":101"), "and it is the MIDDLE day: {one}");

        // A FLOOR ALONE, so `from` does not silently imply a ceiling.
        let (_, _, tail) = read("&from=2024-01-02").await;
        assert_eq!(tail.matches("\"t\":").count(), 2, "the day and after it");

        // A malformed supplied bound cannot silently widen to the whole month.
        let (code, _, junk) = read("&from=not-a-day").await;
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{junk}");
        assert!(junk.contains("from"), "named date refusal: {junk}");
        assert_eq!(junk.matches("\"t\":").count(), 0, "no bar response: {junk}");
    }
}
