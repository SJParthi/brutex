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

use super::{Addressed, Loaded, param, params, timeframe_param};
use crate::{bars, ingest, render};
use std::fmt::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};

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
#[cfg(test)]
pub(super) fn bars_array(
    rows: &[store::format::Bar],
    from: Option<i64>,
    to: Option<i64>,
) -> (String, Vec<String>) {
    bars_array_of(rows, from, to, None)
}

/// [`bars_array`] for a series of `segment`: an `INDEX` or `CASH` bar's open
/// interest is not applicable and is served `null` with `oichg_why`
/// `not_a_derivative`, whatever was stored, and an `INDEX` bar's volume
/// carries `v_why` `index_no_volume` (D-3322; DB-036, DB-G28, DB-G100). A
/// Zerodha index or share month stores `0` there, which is a measurement
/// nobody took; GDFL stores the null. Both now read the same.
///
/// A derivative's row is unchanged, byte for byte, so a one-second option
/// month is not made larger by a reason it never carries.
pub(super) fn bars_array_of(
    rows: &[store::format::Bar],
    from: Option<i64>,
    to: Option<i64>,
    segment: Option<brutex_core::instrument::Segment>,
) -> (String, Vec<String>) {
    use brutex_core::instrument::Segment;
    let inside = |bar: &store::format::Bar| {
        from.is_none_or(|at| bar.ts_micros >= at) && to.is_none_or(|at| bar.ts_micros < at)
    };
    let tail = match segment {
        Some(Segment::Index) => r#","oichg_why":"not_a_derivative","v_why":"index_no_volume""#,
        Some(Segment::Cash) => r#","oichg_why":"not_a_derivative""#,
        Some(Segment::Fno) | None => "",
    };
    let oi_applies = tail.is_empty();
    let mut out = String::with_capacity(rows.len() * (96 + tail.len()) + 32);
    let mut withheld = Vec::new();
    out.push('[');
    let mut written = 0usize;
    for bar in rows.iter().filter(|bar| inside(bar)) {
        // A VALUE A BROWSER CANNOT HOLD IS WITHHELD BY NAME (CE-60, D-2687).
        if let Some(why) = inexact_bar(bar, oi_applies) {
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
            r#"{{"t":{},"o":{},"h":{},"l":{},"c":{},"v":{},"oi":"#,
            bar.ts_micros / 1_000_000,
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume,
        );
        // WRITTEN IN PLACE, NOT THROUGH A `String` PER BAR: a one-second
        // month is about 2.7 million rows, and the open interest was one heap
        // allocation each (`cloud/attack-o1-20261016`).
        if !oi_applies || bar.open_interest == store::format::OI_NULL {
            out.push_str("null");
        } else {
            let _ = write!(out, "{}", bar.open_interest);
        }
        out.push_str(tail);
        out.push('}');
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
fn inexact_bar(bar: &store::format::Bar, oi_applies: bool) -> Option<String> {
    // AN OPEN INTEREST SERVED AS `null` IS NOT SENT, so it cannot be rounded.
    let interest =
        (oi_applies && bar.open_interest != store::format::OI_NULL).then_some(bar.open_interest);
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
    // OFF THE ASYNC WORKERS (DB-066). A month of one-second bars is a 35 MB
    // read and render; on a Tokio worker it stalled every other route that
    // shared it. The blocking pool admits it, and a full pool answers 429 by
    // name rather than queueing behind it.
    // STOPPED WHEN THE CLIENT GOES (DB-N15): the read looks at the flag
    // between chunks of rows.
    let answer = cancellable(false, move |cancel| {
        let answered = month_answer(&site, &asked, from_micros, to_micros, cancel);
        note_cancelled("/bars.json", &answered);
        answered
    })
    .await;
    match answer {
        Ok(Ok((status, body))) => (status, json(), body),
        // A KEY THE DECODER REFUSES IS GONE, as on the window (D-3331, WP-12c).
        Ok(Err(why)) if pull::gdfl_lineage::is_contract_refused(&why) => (
            axum::http::StatusCode::GONE,
            json(),
            format!(r#"{{"error":{}}}"#, render::json_string(&why)),
        ),
        Ok(Err(why)) => refuse(why),
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                "a month of bars",
                crate::detail::MAX_STORE_READ_CONCURRENT,
                &why,
            );
            (status, json(), body)
        }
    }
}

/// [`bars_json`]'s blocking half: open the month, locate the day window and
/// render it. Returns the status and body, or the refusal sentence.
fn month_answer(
    site: &Loaded,
    asked: &Addressed,
    from_micros: Option<i64>,
    to_micros: Option<i64>,
    cancel: &bars::Cancel,
) -> Result<(axum::http::StatusCode, String), String> {
    let file = asked.open(site, asked.month)?;
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
    let (rows, mut faults) =
        bars::page_cancellable(&file, begins, ends.saturating_sub(begins), cancel)?;

    let segment = brutex_core::instrument::Segment::parse(&asked.segment).ok();
    let (out, withheld) = bars_array_of(&rows, from_micros, to_micros, segment);
    for one in withheld {
        faults.push(one);
    }

    // A FAULTY RECORD IS NOT SILENTLY SKIPPED. `page` returns what it could
    // read and what it could not; dropping the second half would draw a chart
    // with a hole in it and no way to know.
    //
    // COALESCED AND CAPPED, NEVER ONE SENTENCE PER RECORD (DB-B75): a month
    // whose `.crc` was lost answered 188,619,461 bytes here, every record its
    // own copy of the path. `faulted_records` keeps the whole count.
    if !faults.is_empty() {
        let records = faults.records();
        return Ok((
            axum::http::StatusCode::PARTIAL_CONTENT,
            format!(
                r#"{{"bars":{out},"faulted_records":{records},"faults":{}}}"#,
                faults_json(
                    &faults
                        .sentences()
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                )
            ),
        ));
    }
    Ok((axum::http::StatusCode::OK, out))
}

/// One page of bars across a RANGE of months and one or more series,
/// answered in ONE request.
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
/// # Several series, ordered on the server (D-3326)
///
/// `/db` then asked each contract of a chain separately with the same offset
/// and limit, and re-sorted the union of the pages it held: page 2 of an
/// 80-series close sort overlapped the true page 2 on 22 of 50 rows, the
/// total was the first series' alone, and a CE+PE pair was stacked rather
/// than merged by time (DB-002, DB-003, DB-G82, DB-G92, DB-G93). `key=` names
/// every series of one request, repeated up to [`bars::MAX_SERIES`] times, and
/// the server merges and pages them; `series_totals` says what each holds.
///
/// # Why the extremes ride along
///
/// The grid scales its magnitude bars against the widest range in the QUERY so
/// that turning a page cannot change what a full bar means. `extremes=1`
/// restores it for the cost of a scan, and now says where each extreme is.
/// Everything `/bars/window.json` reads off the query string.
///
/// Parsed as one step so the handler is dispatch rather than a wall of
/// `match … return refuse`. Every field REFUSES rather than defaulting where a
/// default would answer a different question than the one asked.
#[expect(
    clippy::struct_excessive_bools,
    reason = "the four yes/no parameters of the query string, each its own field \
              as `bars::WindowQuery` has them"
)]
pub(super) struct WindowAsk {
    pub(super) vendor: brutex_core::vendor::Vendor,
    pub(super) timeframe: store::path::Timeframe,
    pub(super) series: Vec<bars::SeriesKey>,
    pub(super) from: store::path::YearMonth,
    pub(super) to: store::path::YearMonth,
    pub(super) sort: bars::SortKey,
    pub(super) desc: bool,
    pub(super) offset: Option<usize>,
    pub(super) limit: usize,
    pub(super) want_extremes: bool,
    pub(super) at: Option<i64>,
    pub(super) filter: bars::Filter,
    pub(super) lookback: bool,
    pub(super) coverage: bool,
}

/// A two-way flag, refused by name when it is neither.
fn flag(query: &str, key: &str, absent: bool) -> Result<bool, String> {
    match param(query, key).as_str() {
        "" => Ok(absent),
        "0" | "false" => Ok(false),
        "1" | "true" => Ok(true),
        other => Err(format!(
            "{other:?} is not a {key} flag. Accepted: 0, 1, false, true."
        )),
    }
}

/// `HH:MM:SS` (or `HH:MM`, read as `:00`) as seconds of the day, or a
/// refusal naming `key`.
fn time_of_day(key: &str, text: &str) -> Result<u32, String> {
    let fields: Vec<&str> = text.split(':').collect();
    let number = |field: Option<&&str>, top: u32| {
        field
            .filter(|field| field.len() == 2 && field.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|field| field.parse::<u32>().ok())
            .filter(|&value| value < top)
    };
    let parsed = match fields.len() {
        2 => number(fields.first(), 24)
            .zip(number(fields.get(1), 60))
            .map(|(h, m)| h * 3_600 + m * 60),
        3 => number(fields.first(), 24)
            .zip(number(fields.get(1), 60))
            .zip(number(fields.get(2), 60))
            .map(|((h, m), sec)| h * 3_600 + m * 60 + sec),
        _ => None,
    };
    parsed.ok_or_else(|| format!("{text:?} is not an IST time of day HH:MM:SS for {key:?}"))
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
        let series = Self::series(query)?;
        let sort = bars::SortKey::parse(&param(query, "sort")).ok_or_else(|| {
            format!(
                "{:?} is not a column this grid sorts on. Accepted: ts, o, h, l, c, v, oi, tod.",
                param(query, "sort")
            )
        })?;
        // A NON-NUMBER IS A REFUSAL, NOT A ZERO. `offset=banana` silently
        // reading from the top would answer a different question than the one
        // asked, and the pager would look right while showing the wrong rows.
        let number = |key: &str| {
            let raw = param(query, key);
            if raw.is_empty() {
                Ok(None)
            } else {
                raw.parse::<usize>()
                    .map(Some)
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
        let at = Self::instant(query, sort)?;
        Ok(Self {
            vendor,
            timeframe,
            series,
            from,
            to,
            sort,
            desc,
            offset: number("offset")?,
            // A LIMIT OUTSIDE 1..=MAX IS REFUSED, NOT CLAMPED. `limit=5000`
            // answered 1,000 rows under a 200 and a pager stepping by what it
            // asked skipped 4,000 of them; `limit=0` answered an empty page
            // beside a non-zero total (P1-01-01, P1-02-02, D-1765).
            limit: match number("limit")?.unwrap_or(bars::PAGE_BARS) {
                limit @ 1..=bars::MAX_WINDOW_LIMIT => limit,
                other => {
                    return Err(format!(
                        "{other} is not a page size this grid serves. Accepted: 1 to {}.",
                        bars::MAX_WINDOW_LIMIT
                    ));
                }
            },
            want_extremes,
            at,
            filter: Self::filter(query)?,
            lookback: flag(query, "lookback", true)?,
            coverage: flag(query, "coverage", false)?,
        })
    }

    /// The series: every `key=`, or the one `exchange`, `segment`, `symbol`
    /// and `contract` name.
    fn series(query: &str) -> Result<Vec<bars::SeriesKey>, String> {
        let keys = params(query, "key");
        if keys.is_empty() {
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
            return Ok(vec![bars::SeriesKey::new(
                &param(query, "exchange"),
                &param(query, "segment"),
                &param(query, "symbol"),
                contract,
            )]);
        }
        // ONE WAY TO NAME A SERIES PER REQUEST. Both at once would leave the
        // reader to guess which one the answer is about.
        if ["symbol", "contract"]
            .iter()
            .any(|key| !param(query, key).is_empty())
        {
            return Err(
                "name the series with key=, or with exchange, segment, symbol and contract, \
                 not both"
                    .to_owned(),
            );
        }
        if keys.len() > bars::MAX_SERIES {
            return Err(format!(
                "too_many_series: {} keys were named and one window reads at most {}",
                keys.len(),
                bars::MAX_SERIES
            ));
        }
        let mut series = Vec::with_capacity(keys.len());
        for key in &keys {
            let parsed = bars::SeriesKey::parse(key)
                .ok_or_else(|| format!("{key:?} is not a census instrument key"))?;
            // `all(!=)`, not the membership call gate 11 refuses.
            if !series.iter().all(|held| *held != parsed) {
                return Err(format!("{key:?} is named twice"));
            }
            series.push(parsed);
        }
        Ok(series)
    }

    /// `at=<unix seconds>` as UTC micros; refused with any order but time.
    fn instant(query: &str, sort: bars::SortKey) -> Result<Option<i64>, String> {
        let raw = param(query, "at");
        if raw.is_empty() {
            return Ok(None);
        }
        if sort.scans() {
            return Err("at= finds an instant in time order, so it needs sort=ts".to_owned());
        }
        raw.parse::<i64>()
            .ok()
            .and_then(|secs| secs.checked_mul(1_000_000))
            .map(Some)
            .ok_or_else(|| format!("{raw:?} is not a whole number of unix seconds for \"at\""))
    }

    /// `from_day`, `to_day`, `tod_from` and `tod_to`.
    fn filter(query: &str) -> Result<bars::Filter, String> {
        let from = ist_midnight_micros("from_day", &param(query, "from_day"))?;
        let to = ist_midnight_micros("to_day", &param(query, "to_day"))?;
        if from.zip(to).is_some_and(|(from, to)| from > to) {
            return Err("from_day must be on or before to_day".to_owned());
        }
        let tod_from = param(query, "tod_from");
        let tod_to = param(query, "tod_to");
        let tod = if tod_from.is_empty() && tod_to.is_empty() {
            None
        } else {
            let lo = if tod_from.is_empty() {
                0
            } else {
                time_of_day("tod_from", &tod_from)?
            };
            let hi = if tod_to.is_empty() {
                86_399
            } else {
                time_of_day("tod_to", &tod_to)?
            };
            if lo > hi {
                return Err(
                    "tod_from must be at or before tod_to: a window across midnight is not served"
                        .to_owned(),
                );
            }
            Some((lo, hi))
        };
        Ok(bars::Filter {
            from,
            to: to.map(|at| at + 86_400 * 1_000_000),
            tod,
        })
    }

    /// The query the window reads.
    fn query(self) -> bars::WindowQuery {
        bars::WindowQuery {
            vendor: self.vendor,
            timeframe: self.timeframe,
            series: self.series,
            from: self.from,
            to: self.to,
            sort: self.sort,
            desc: self.desc,
            offset: self.offset,
            limit: self.limit,
            want_extremes: self.want_extremes,
            at: self.at,
            filter: self.filter,
            lookback: self.lookback,
            coverage: self.coverage,
        }
    }
}

/// Window scans — a non-time sort, a fold of the extremes, or the O(days)
/// work of a time-of-day bound or `coverage=1` — that may be queued or
/// running at once on the blocking pool.
///
/// A pool of its own, apart from `crate::detail::run_store_read`'s, so a seek
/// never waits behind a scan (D-3326, DB-066): a scan reads up to
/// [`bars::MAX_SCAN_BARS`] records, a seek a page.
pub(super) const MAX_SCAN_CONCURRENT: usize = 2;

static SCANS_ACTIVE: AtomicUsize = AtomicUsize::new(0);

/// One admitted scan; dropping it returns the slot.
struct ScanPermit;

impl Drop for ScanPermit {
    fn drop(&mut self) {
        SCANS_ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Runs a window scan on the blocking pool, in the scan pool of
/// [`MAX_SCAN_CONCURRENT`] slots, or refuses before queueing.
async fn run_scan<T, F>(work: F) -> Result<T, crate::detail::RunError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    SCANS_ACTIVE
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
            (active < MAX_SCAN_CONCURRENT).then_some(active.saturating_add(1))
        })
        .map_err(|_| crate::detail::RunError::Saturated)?;
    let permit = ScanPermit;
    tokio::task::spawn_blocking(move || {
        let _held = permit;
        work()
    })
    .await
    .map_err(|why| crate::detail::RunError::Join(why.to_string()))
}

/// Runs `work` off the async workers — in the scan pool when `heavy`, else
/// the store-read pool, each admitted under its cap and refused by name when
/// full — with a cancellation flag this future owns (DB-N15).
///
/// When the client goes away the server drops the request's future, and with
/// it the guard, which sets the flag; `work` looks at it between months and
/// every few thousand rows and stops by name. An answer that arrives disarms
/// the guard first, so a finished request cancels nothing.
///
/// # Errors
///
/// The pool's refusal, or a join failure.
pub(super) async fn cancellable<T, F>(heavy: bool, work: F) -> Result<T, crate::detail::RunError>
where
    T: Send + 'static,
    F: FnOnce(&bars::Cancel) -> T + Send + 'static,
{
    let (cancel, guard) = bars::Cancel::pair();
    let job = move || work(&cancel);
    let answered = if heavy {
        run_scan(job).await
    } else {
        crate::detail::run_store_read(job).await
    };
    guard.disarm();
    answered
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
    let scanned = asked.sort.scans() || asked.want_extremes;
    // THE SCAN POOL ALSO HOLDS THE WORK THAT GROWS WITH DAYS. A time-of-day
    // bound locates every day's window and `coverage=1` counts every held
    // month's days: O(days), measured at 0.9 to 1.7 s for a first request
    // over NIFTY's 97 one-second months (most without a `.tix`), and 0.6 ms
    // once the month's runs are memoised. Either would hold a seek slot for
    // that second, so it waits where the sorts wait (D-3326, DB-066).
    let heavy = scanned || asked.filter.tod.is_some() || asked.coverage;
    let root = site.store_root.clone();
    let query = asked.query();
    // STOPPED WHEN THE CLIENT GOES (DB-N15): the window looks at the flag
    // between its plan and its read, and every few thousand rows of a read.
    let work = move |cancel: &bars::Cancel| {
        let window = bars::window_cancellable(&root, &query, cancel);
        note_cancelled("/bars/window.json", &window);
        Ok::<_, String>(render_window(&window?, scanned))
    };
    // OFF THE ASYNC WORKERS, AND A SCAN APART FROM A SEEK (DB-066).
    let answered = cancellable(heavy, work).await;
    let (body, withheld, faulted) = match answered {
        Ok(Ok(rendered)) => rendered,
        // A SERIES THE DECODER REFUSES IS GONE, not malformed: 410, with the
        // refusal the re-key recorded (D-3331; WP-12c, DB-A18).
        Ok(Err(why)) if pull::gdfl_lineage::is_contract_refused(&why) => {
            return (
                axum::http::StatusCode::GONE,
                json(),
                format!(r#"{{"error":{}}}"#, render::json_string(&why)),
            );
        }
        Ok(Err(why)) => return refuse(why),
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                if heavy {
                    "a bar window scan"
                } else {
                    "a bar window"
                },
                if heavy {
                    MAX_SCAN_CONCURRENT
                } else {
                    crate::detail::MAX_STORE_READ_CONCURRENT
                },
                &why,
            );
            return (status, json(), body);
        }
    };
    // PARTIAL CONTENT WHEN A RECORD WOULD NOT READ, the same status
    // `/bars.json` answers with, for the same reason: the rows are real and
    // the set is not whole, and one status must not mean both.
    let status = if !faulted && withheld == 0 {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::PARTIAL_CONTENT
    };
    (status, json(), body)
}

/// `api.bars`, INFO, once per request whose client went away mid-read: the
/// route, and that the work stopped. Silent for every answered request.
fn note_cancelled<T>(route: &str, answered: &Result<T, String>) {
    if let Err(why) = answered
        && why.starts_with("cancelled")
    {
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::info("api.bars", "request abandoned; work stopped")
                .with("route", telemetry::Value::Str(route)),
        );
    }
}

/// Faults one answer names in full; past it the rest are counted.
///
/// **The body is bounded whatever went wrong.** The readers name a month
/// once however many of its rows would not read, and this is the second
/// bound, for a window of many series: `MAX_FAULTS_NAMED` reasons of at most
/// `MAX_FAULT_BYTES` each, then one count.
const MAX_FAULTS_NAMED: usize = 64;

/// Bytes of one fault an answer carries; a longer one is cut and says so.
const MAX_FAULT_BYTES: usize = 512;

/// The `faults` field: `null` for none, else the reasons joined, capped at
/// [`MAX_FAULTS_NAMED`] with the count of the rest.
fn faults_json(faults: &[&str]) -> String {
    if faults.is_empty() {
        return "null".to_owned();
    }
    let mut named: Vec<String> = faults
        .iter()
        .take(MAX_FAULTS_NAMED)
        .map(|fault| {
            if fault.len() <= MAX_FAULT_BYTES {
                (*fault).to_owned()
            } else {
                let mut cut = MAX_FAULT_BYTES;
                while !fault.is_char_boundary(cut) {
                    cut -= 1;
                }
                format!(
                    "{} [cut: {} more bytes]",
                    fault.get(..cut).unwrap_or_default(),
                    fault.len() - cut
                )
            }
        })
        .collect();
    if faults.len() > MAX_FAULTS_NAMED {
        named.push(format!(
            "and {} more fault(s) not listed here",
            faults.len() - MAX_FAULTS_NAMED
        ));
    }
    render::json_string(&named.join("; "))
}

/// What a value sort served from sort indexes and what it read, as JSON, or
/// `null` for a time-ordered page (D-3349).
fn sort_index_json(report: Option<&bars::SortReport>) -> String {
    report.map_or_else(
        || "null".to_owned(),
        |report| {
            let list = |items: &[String]| {
                let joined: Vec<String> = items.iter().map(|item| render::json_string(item)).collect();
                format!("[{}]", joined.join(","))
            };
            format!(
                r#"{{"column":{},"by_time":{},"indexed_rows":{},"indexed_runs":{},"cut_runs":{},"scanned_rows":{},"scanned_months":{},"scanned_why":{},"unread_rows":{},"unread_months":{},"unread_why":{},"build":{}}}"#,
                render::json_string(report.column),
                report.by_time,
                report.indexed_rows,
                report.indexed_runs,
                report.cut_runs,
                report.scanned_rows,
                report.scanned_months,
                list(&report.scanned_why),
                report.unread_rows,
                report.unread_months,
                list(&report.unread_why),
                list(&report.build),
            )
        },
    )
}

/// One extreme as JSON: `{"v", "t", "key"}`, `t` in UTC seconds like a bar's.
fn extreme_json(extreme: Option<bars::Extreme>, keys: &[bars::SeriesTotal]) -> String {
    extreme.map_or_else(
        || "null".to_owned(),
        |extreme| {
            format!(
                r#"{{"v":{},"t":{},"key":{}}}"#,
                extreme.v,
                extreme.t.div_euclid(1_000_000),
                keys.get(extreme.series)
                    .map_or_else(|| "null".to_owned(), |held| render::json_string(&held.key))
            )
        },
    )
}

/// The window's extremes as JSON, or `null` with the reason pushed to
/// `withheld` when a value would not survive a browser's JSON number.
fn extremes_json(window: &bars::Window, withheld: &mut Vec<String>) -> String {
    let Some(top) = window.extremes else {
        return "null".to_owned();
    };
    let values = [
        Some(top.range),
        Some(top.volume),
        top.close_max.map(|e| e.v),
        top.close_min.map(|e| e.v),
        top.high_max.map(|e| e.v),
        top.low_min.map(|e| e.v),
        top.range_max.map(|e| e.v),
        top.volume_max.map(|e| e.v),
    ];
    if values
        .into_iter()
        .flatten()
        .any(|value| !browser_exact(value))
    {
        withheld.push(format!(
            "the window's extremes (range {}, volume {}) lie outside \
             ±(2^53 − 1), which a browser's JSON number cannot hold \
             exactly; withheld as null",
            top.range, top.volume
        ));
        return "null".to_owned();
    }
    let keys = &window.series;
    format!(
        r#"{{"range":{},"volume":{},"close_max":{},"close_min":{},"high_max":{},"low_min":{},"range_max":{},"volume_max":{}}}"#,
        top.range,
        top.volume,
        extreme_json(top.close_max, keys),
        extreme_json(top.close_min, keys),
        extreme_json(top.high_max, keys),
        extreme_json(top.low_min, keys),
        extreme_json(top.range_max, keys),
        extreme_json(top.volume_max, keys),
    )
}

/// One window row as JSON, or the sentence withholding it because a browser's
/// JSON number cannot hold one of its values (CE-60, D-2687).
fn render_row(
    window: &bars::Window,
    row: &bars::WindowBar,
    several: bool,
) -> Result<String, String> {
    let bar = &row.bar;
    let change = [("chg", row.chg), ("oichg", row.oichg)]
        .into_iter()
        .find_map(|(field, bps)| {
            bps.filter(|&held| !browser_exact(held))
                .map(|held| inexact_fault(bar.ts_micros, field, held))
        });
    if let Some(why) = inexact_bar(bar, row.oi_applies).or(change) {
        return Err(why);
    }
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
    let word = |text: Option<&str>| text.map_or_else(|| "null".to_owned(), render::json_string);
    let day = bars::ist_day(bar.ts_micros);
    let mut out = String::with_capacity(256);
    let _ = write!(
        out,
        r#"{{"t":{},"o":{},"h":{},"l":{},"c":{},"v":{},"v_why":{},"oi":{},"chg":{},"chg_why":{},"oichg":{},"oichg_why":{},"month":{},"phase":{}"#,
        bar.ts_micros / 1_000_000,
        bar.open,
        bar.high,
        bar.low,
        bar.close,
        bar.volume,
        word(row.v_why),
        if !row.oi_applies || bar.open_interest == store::format::OI_NULL {
            "null".to_owned()
        } else {
            bar.open_interest.to_string()
        },
        value(row.chg),
        why(row.chg, row.chg_why),
        value(row.oichg),
        why(row.oichg, row.oichg_why),
        render::json_string(day.get(..7).unwrap_or(&day)),
        word(row.phase),
    );
    if several {
        let _ = write!(
            out,
            r#","key":{}"#,
            window
                .series
                .get(row.series)
                .map_or_else(|| "null".to_owned(), |held| render::json_string(&held.key))
        );
    }
    out.push('}');
    Ok(out)
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
/// Returns the body, how many bars or extremes were withheld because a
/// browser's JSON number cannot hold one of their values (CE-60, D-2687), and
/// whether the window named a fault; the caller answers 206 for either.
pub(super) fn render_window(window: &bars::Window, scanned: bool) -> (String, usize, bool) {
    let mut rows = String::with_capacity(window.bars.len() * 200 + 32);
    let mut withheld: Vec<String> = Vec::new();
    let several = window.series.len() > 1;
    rows.push('[');
    let mut written = 0usize;
    for row in &window.bars {
        match render_row(window, row, several) {
            Ok(text) => {
                if written > 0 {
                    rows.push(',');
                }
                written = written.saturating_add(1);
                rows.push_str(&text);
            }
            Err(why) => withheld.push(why),
        }
    }
    rows.push(']');

    let extremes = extremes_json(window, &mut withheld);
    let mut faults: Vec<&str> = window.faults.iter().map(String::as_str).collect();
    faults.extend(withheld.iter().map(String::as_str));
    let list = |items: &mut dyn Iterator<Item = String>| {
        let joined: Vec<String> = items.collect();
        format!("[{}]", joined.join(","))
    };
    let missing = list(&mut window.missing_months.iter().map(|m| render::json_string(m)));
    let totals = list(&mut window.series.iter().map(|held| {
        format!(
            r#"{{"key":{},"total":{}}}"#,
            render::json_string(&held.key),
            held.total
        )
    }));
    let coverage = list(&mut window.coverage.iter().map(|month| {
        format!(
            r#"{{"key":{},"month":{},"rows":{},"days_covered":{}}}"#,
            window
                .series
                .get(month.series)
                .map_or_else(|| "null".to_owned(), |held| render::json_string(&held.key)),
            render::json_string(&month.month.to_string()),
            month.rows,
            month.days_covered
        )
    }));
    // THE KEY A SERIES CONTINUES AS (D-3331; WP-12c, DB-A17): `null` when no
    // asked series is superseded, so every older answer reads the same.
    let superseded = if window.superseded.is_empty() {
        "null".to_owned()
    } else {
        list(&mut window.superseded.iter().map(|one| {
            format!(
                r#"{{"key":{},"by":{},"kind":{},"recorded":{}}}"#,
                render::json_string(&one.key),
                render::json_string(&one.by),
                render::json_string(one.kind),
                one.recorded
            )
        }))
    };
    // AND THE KEYS A SERIES' COPIED BARS CAME FROM, so the copy is audited
    // from the key it landed in: `null` for every other series.
    let copied_in = if window.supersedes.is_empty() {
        "null".to_owned()
    } else {
        list(&mut window.supersedes.iter().map(|one| {
            format!(
                r#"{{"key":{},"of":{},"kind":{},"bars":{},"blake3":{}}}"#,
                render::json_string(&one.key),
                render::json_string(&one.of),
                render::json_string(one.kind),
                one.bars,
                render::json_string(&one.blake3)
            )
        }))
    };
    let body = format!(
        r#"{{"total":{},"offset":{},"months_read":{},"months_missing":{},"missing_months":{missing},"months_outside_life":{},"scanned":{scanned},"scan_rows":{},"sort_index":{},"at_index":{},"series_totals":{totals},"superseded_by":{superseded},"supersedes":{copied_in},"extremes":{extremes},"extremes_why":{},"coverage":{coverage},"faulted_records":{},"faults":{},"bars":{rows}}}"#,
        window.total,
        window.offset,
        window.months_read,
        window.months_missing,
        window.months_outside_life,
        window.scan_rows,
        sort_index_json(window.sort_index.as_ref()),
        window
            .at_index
            .map_or_else(|| "null".to_owned(), |index| index.to_string()),
        window
            .extremes_why
            .as_deref()
            .map_or_else(|| "null".to_owned(), render::json_string),
        window.faulted_records,
        faults_json(&faults)
    );
    (body, withheld.len(), !window.faults.is_empty())
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

    /// **THE `faults` FIELD IS BOUNDED WHATEVER WENT WRONG.** None is `null`;
    /// a few are joined as they are; 100,000 are the first
    /// `MAX_FAULTS_NAMED` and a count of the rest; one of a megabyte is cut
    /// and says how much was cut. The field was `faults.join("; ")` of
    /// everything — 59 MB for a month another process held.
    #[test]
    fn the_faults_field_is_bounded_whatever_went_wrong() {
        assert_eq!(faults_json(&[]), "null");
        assert_eq!(faults_json(&["a", "b"]), r#""a; b""#);
        let one = "2025-03: another writer holds the month";
        let many = vec![one; 100_000];
        let field = faults_json(&many);
        assert_eq!(field.matches(one).count(), MAX_FAULTS_NAMED);
        assert!(
            field.contains(&format!(
                "and {} more fault(s) not listed here",
                100_000 - MAX_FAULTS_NAMED
            )),
            "{}",
            &field[field.len() - 80..]
        );
        let bound = MAX_FAULTS_NAMED * (MAX_FAULT_BYTES + 40) + 80;
        assert!(field.len() < bound, "{} bytes", field.len());
        // 1,000,001 BYTES WHOSE 512TH IS INSIDE A CHARACTER: cut at 511.
        let long = format!("x{}", "é".repeat(500_000));
        let field = faults_json(&[long.as_str(); 200]);
        assert!(field.len() < bound, "{} bytes", field.len());
        assert!(field.contains("[cut: 999490 more bytes]"), "a cut is named");
    }

    /// **A `/bars.json` ROW ALLOCATES NOTHING OF ITS OWN**
    /// (`cloud/attack-o1-20261016`).
    ///
    /// The open interest went through `"null".to_owned()` or
    /// `open_interest.to_string()` once per bar, about 2.7 million heap
    /// allocations for a one-second month. The bytes are pinned for all three
    /// segment shapes, and the allocation's absence is asserted on the source
    /// of `bars_array_of`, whose needles are assembled at run time so this
    /// test cannot match itself.
    #[test]
    fn a_bars_row_is_written_in_place_with_the_same_bytes() {
        use brutex_core::instrument::Segment;
        let bar = |open_interest: i64| store::format::Bar {
            ts_micros: 1_704_079_800_000_000,
            open: 2_150_000,
            high: 2_151_000,
            low: 2_149_000,
            close: 2_150_500,
            volume: 75,
            open_interest,
        };
        let rows = [bar(1_234), bar(store::format::OI_NULL)];
        let head =
            r#"{"t":1704079800,"o":2150000,"h":2151000,"l":2149000,"c":2150500,"v":75,"oi":"#;
        assert_eq!(
            bars_array_of(&rows, None, None, Some(Segment::Fno)).0,
            format!("[{head}1234}},{head}null}}]")
        );
        let index = r#","oichg_why":"not_a_derivative","v_why":"index_no_volume"}"#;
        assert_eq!(
            bars_array_of(&rows, None, None, Some(Segment::Index)).0,
            format!("[{head}null{index},{head}null{index}]")
        );
        let cash = r#","oichg_why":"not_a_derivative"}"#;
        assert_eq!(
            bars_array_of(&rows, None, None, Some(Segment::Cash)).0,
            format!("[{head}null{cash},{head}null{cash}]")
        );
        let source = include_str!("bars_route.rs");
        let start = source
            .find(&format!("{}{}", "fn bars_array", "_of("))
            .expect("bars_array_of exists");
        let body = &source[start..];
        let body = &body[..body.find("\n}\n").expect("bars_array_of ends")];
        for needle in [
            format!("{}{}", ".to_", "owned()"),
            format!("{}{}", ".to_", "string()"),
        ] {
            assert!(
                !body.contains(&needle),
                "bars_array_of holds {needle:?}: a per-bar allocation is back"
            );
        }
    }
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

    /// **A FULL SCAN POOL REFUSES A SCAN BY NAME, AND A SEEK IS STILL
    /// ANSWERED.** D-3326, DB-066, DB-G80: every window ran on a Tokio worker
    /// with no admission, so a few 97-month sorts could hold every worker and
    /// queue every route behind them. A sort or an extremes fold now waits in
    /// a pool of [`MAX_SCAN_CONCURRENT`] slots apart from the seek pool; when
    /// it is full the scan answers 429 naming that bound, and a time-ordered
    /// page in the same instant is served from the other pool.
    #[tokio::test]
    async fn a_full_scan_pool_refuses_a_scan_by_name_and_a_seek_is_still_answered() {
        let root = store_root("scanpool");
        let month = store::path::YearMonth::new(2024, 1).expect("a legal month");
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
        // 2024-01-02 09:15 IST onward, one invented bar a minute.
        let open = (19_724 * 86_400 - 19_800 + 9 * 3_600 + 15 * 60) * 1_000_000;
        let rows: Vec<store::format::Bar> = (0..5)
            .map(|n| store::format::Bar {
                ts_micros: open + n * 60_000_000,
                open: 1_000 + n,
                high: 1_010 + n,
                low: 990 + n,
                close: 1_005 + n,
                volume: 0,
                open_interest: i64::MIN,
            })
            .collect();
        file.append(&rows).expect("five legal bars");
        drop(file);
        let loaded = std::sync::Arc::new(Site::serving(&masters("scanpool", None, None), &root));
        let ask = |sort: &str| {
            let uri: axum::http::Uri = format!(
                "/bars/window.json?feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY\
                 &timeframe=1min&from=2024-01&to=2024-01&limit=5&sort={sort}"
            )
            .parse()
            .expect("a uri");
            bars_window_json(axum::extract::State(std::sync::Arc::clone(&loaded)), uri)
        };
        // EVERY SLOT TAKEN, whatever another test holds at this instant.
        SCANS_ACTIVE.fetch_add(MAX_SCAN_CONCURRENT, Ordering::AcqRel);
        let (scan, _, refused) = ask("c").await;
        let (seek, _, served) = ask("ts").await;
        // A TIME-OF-DAY BOUND AND A COVERAGE COUNT ARE O(days): they wait
        // where the sorts wait, not in a seek's slot.
        let (timed, _, timed_body) = ask("ts&tod_from=09:15:00&tod_to=09:16:59").await;
        let (covered, _, covered_body) = ask("ts&coverage=1").await;
        SCANS_ACTIVE.fetch_sub(MAX_SCAN_CONCURRENT, Ordering::AcqRel);
        for (status, body) in [(timed, &timed_body), (covered, &covered_body)] {
            assert_eq!(status, axum::http::StatusCode::TOO_MANY_REQUESTS, "{body}");
            assert!(body.contains("a bar window scan not admitted"), "{body}");
        }
        assert_eq!(scan, axum::http::StatusCode::TOO_MANY_REQUESTS, "{refused}");
        assert!(
            refused.contains("a bar window scan not admitted")
                && refused.contains(&format!("at most {MAX_SCAN_CONCURRENT} run at once")),
            "the refusal names the pool and its bound: {refused}"
        );
        // A seek may itself meet a full seek pool shared with other tests;
        // that is a 429 naming the OTHER pool, never the scan's.
        assert!(
            !served.contains("a bar window scan"),
            "a seek never waits on the scan pool: {served}"
        );
        if seek == axum::http::StatusCode::OK {
            assert_eq!(served.matches("\"t\":").count(), 5, "{served}");
        } else {
            assert_eq!(seek, axum::http::StatusCode::TOO_MANY_REQUESTS, "{served}");
        }
        // AND WITH THE SLOTS FREE AGAIN THE SAME SCAN IS ANSWERED.
        for _ in 0..400 {
            let (status, _, body) = ask("c").await;
            if status == axum::http::StatusCode::OK {
                assert_eq!(body.matches("\"t\":").count(), 5, "{body}");
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        panic!("the scan pool stayed full for two seconds after the slots were returned");
    }

    /// **AN ABANDONED REQUEST STOPS ITS BLOCKING WORK** (DB-N15). The future
    /// a route awaits owns the cancellation guard; dropping it — what the
    /// server does when the client closes the connection, and what
    /// `JoinHandle::abort` does here — sets the flag the blocking work reads.
    /// Before, the work ran to its end on a blocking thread nobody waited
    /// for: a 97-month scan kept reading for 6.5 s after the client left.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_abandoned_request_stops_its_blocking_work() {
        for _ in 0..400 {
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let (done_tx, done_rx) = std::sync::mpsc::channel();
            let task = tokio::spawn(cancellable(false, move |cancel: &bars::Cancel| {
                let _ = started_tx.send(());
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                while !cancel.is_cancelled() && std::time::Instant::now() < deadline {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                let _ = done_tx.send(cancel.is_cancelled());
            }));
            if started_rx
                .recv_timeout(std::time::Duration::from_millis(200))
                .is_err()
            {
                // THE POOL WAS FULL (another test's reads): the work never
                // started and the future answered 429. Asked again.
                let _ = task.await;
                continue;
            }
            task.abort();
            assert_eq!(
                done_rx.recv_timeout(std::time::Duration::from_secs(10)),
                Ok(true),
                "the blocking work saw the client go"
            );
            return;
        }
        panic!("the store-read pool stayed full");
    }

    /// An abandoned request writes ONE `api.bars` line naming its route; an
    /// answered one, and one refused for any other reason, write none.
    #[test]
    fn an_abandoned_request_is_named_once() {
        let from = crate::emitted::mark();
        note_cancelled::<()>(
            "/bars/window.json",
            &Err("cancelled: the client that asked went away".to_owned()),
        );
        note_cancelled::<()>("/bars.json", &Ok(()));
        note_cancelled::<()>("/bars.json", &Err("scan_budget_exceeded: no".to_owned()));
        let lines = crate::emitted::landed(from, "api.bars", "request abandoned; work stopped");
        let mine: Vec<_> = lines
            .iter()
            .filter(|line| crate::emitted::says(line, "route", "/bars"))
            .collect();
        assert_eq!(mine.len(), 1, "{lines:?}");
        assert!(crate::emitted::says(mine[0], "route", "/bars/window.json"));
        assert_eq!(mine[0].level, telemetry::Level::Info);
    }

    /// An answered request cancels nothing: the guard is disarmed before the
    /// future ends.
    #[tokio::test]
    async fn an_answered_request_cancels_nothing() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(None));
        let held = std::sync::Arc::clone(&seen);
        for _ in 0..400 {
            let held = std::sync::Arc::clone(&held);
            let answered = cancellable(true, move |cancel: &bars::Cancel| {
                if let Ok(mut slot) = held.lock() {
                    *slot = Some(cancel.clone());
                }
            })
            .await;
            if answered.is_ok() {
                let kept = seen
                    .lock()
                    .expect("the slot")
                    .clone()
                    .expect("the work ran");
                assert!(!kept.is_cancelled());
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        panic!("the scan pool stayed full");
    }
}
