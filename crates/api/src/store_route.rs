//! `/store.json` -- one feed's census on the wire, its validator, and the
//! three census headers `/store.json` and `/instruments.json` stamp.
//!
//! # Moved, not rewritten
//!
//! Every item below was cut out of `server.rs` verbatim by the WP-01 seam and
//! is mounted there as a child module, so it reads that file's private
//! helpers through `super::` exactly as it did when it lived in it, and
//! `server.rs` imports each name back. The only edits are visibility
//! qualifiers (`pub(super)`) on items another module now has to reach.
//! No route answers differently because of the move.

use super::{
    Loaded, Site, Unknown, census_now, hex32, month_before, month_change, no_such_feed_json,
    note_header, param, store_wire,
};
use crate::{census, ingest, render};
use brutex_core::vendor::Vendor;
use std::fmt::Write as _;

/// The state of the selected feed's census, as one stable word.
///
/// `held` · `absent` · `unreadable` — [`census::Census::name`]'s words, which
/// are `/audit.json`'s words. A feed with no census row in this build reads
/// `absent`, exactly as `audit_json::store_block` renders it, so the two
/// surfaces cannot drift into two vocabularies for one fact.
pub const CENSUS_STATE_HEADER: &str = "x-brutex-census-state";

/// The sentence [`census::VendorCensus::note`] produces for that state.
pub const CENSUS_NOTE_HEADER: &str = "x-brutex-census-note";

/// What loading the census had to step over, or empty when it stepped over
/// nothing. `/audit.json` writes `null` here; a header has no `null`, and an
/// **always-present, sometimes-empty** value keeps "nothing was stepped over"
/// and "this build does not say" apart — an absent header is the second.
pub const CENSUS_DEGRADED_HEADER: &str = "x-brutex-census-degraded";

/// The three census facts, stamped on a JSON response that cannot carry them
/// in its body.
///
/// # Why headers and not a field
///
/// `/store.json` answers a JSON **array** and `/db` reads it as one —
/// `rows = Array.isArray(j) ? j : []`. Wrapping the rows in an object to make
/// room for a status would turn every existing reader into a reader of `[]`,
/// which is the very outcome being fixed. A header is additive: an array stays
/// an array, and a reader that wants the reason asks for it by name.
///
/// # What a page must read
///
/// `r.headers.get('x-brutex-census-state')` — `unreadable` means the array is
/// EMPTY BECAUSE THE COUNTER IS DAMAGED, not because the store is. The sentence
/// to show is `x-brutex-census-note`. The response also carries a non-200 for
/// that state, so a reader that only checks `r.ok` stops calling it empty.
/// D-0124.
pub(super) fn census_headers(census: Option<&census::VendorCensus>) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert(
        axum::http::HeaderName::from_static(CENSUS_STATE_HEADER),
        axum::http::HeaderValue::from_static(match census.map(|c| &c.state) {
            Some(census::Census::Held { .. }) => "held",
            Some(census::Census::Unreadable { .. }) => "unreadable",
            // A feed with no row is `absent`, which is what
            // `audit_json::store_block` already answers for the same input.
            Some(census::Census::Absent) | None => "absent",
        }),
    );
    headers.insert(
        axum::http::HeaderName::from_static(CENSUS_NOTE_HEADER),
        note_header(&census.map_or_else(
            || String::from("no census row for this feed in this build"),
            census::VendorCensus::note,
        )),
    );
    headers.insert(
        axum::http::HeaderName::from_static(CENSUS_DEGRADED_HEADER),
        note_header(
            &census
                .and_then(census::VendorCensus::degraded)
                .unwrap_or_default(),
        ),
    );
    headers
}

/// Whether the selected feed's counter refused to load.
///
/// The one state where a body of `[]` and a body of every row this store holds
/// are produced by the same code path, and the reason a status code moves.
pub(super) fn census_is_unreadable(census: Option<&census::VendorCensus>) -> bool {
    matches!(
        census.map(|c| &c.state),
        Some(census::Census::Unreadable { .. })
    )
}

/// The census row for one key — the counters and the closes in **one probe**.
///
/// `pull::manifest::Manifest::held` is one hash probe into a table the load
/// walk built once, so this reads `rows` and both closes for the cost the old
/// `rows_for` paid for `rows` alone. Proven by
/// `api::server::a_row_and_its_neighbour_cost_two_probes_and_no_file`.
pub(super) fn held_row(
    census: &census::VendorCensus,
    key: &pull::manifest::EntryKey,
) -> Option<pull::manifest::Held> {
    match census.state {
        census::Census::Held { ref manifest } => manifest.held(key),
        census::Census::Absent | census::Census::Unreadable { .. } => None,
    }
}

/// One row of `/store.json`, appended to `out`.
///
/// Written as `"chg_bps": <n>, "chg_why": null` **or** `"chg_bps": null,
/// "chg_why": "<code>"` — both keys always present, exactly one of them a
/// value. A key that is sometimes absent makes "unknown" and "the field does
/// not exist in this build" the same thing on the wire, and the browser cannot
/// tell them apart.
fn write_change(out: &mut String, key: &str, change: Result<i64, Unknown>) {
    match change {
        Ok(bps) => {
            let _ = write!(out, r#","{key}_bps":{bps},"{key}_why":null"#);
        }
        Err(why) => {
            let _ = write!(
                out,
                r#","{key}_bps":null,"{key}_why":{}"#,
                render::json_string(why.code())
            );
        }
    }
}

/// The body of `/store.json` for one feed.
///
/// Split out of the handler so every arm is reachable from a test without a
/// socket — the same argument this module's own header makes about `main.rs`.
///
/// # Cost
///
/// **Two hash probes per row and no syscall**: this month's row, and the same
/// series at [`month_before`]. Crossing a month crosses a manifest *entry*, not
/// a file — no `open`, no `stat`, no `pread`, because the census is already
/// resident. Proven by
/// `api::server::a_row_and_its_neighbour_cost_two_probes_and_no_file`.
///
/// What is **not** constant is the caller: `census_now` re-reads and re-walks
/// every manifest per request, which is O(entries) and was before this existed.
/// `docs/06-limits.md` §41 records it rather than this comment hiding it.
pub(super) fn store_body(
    censuses: &[census::VendorCensus],
    entries: &[(census::Series, store::path::YearMonth)],
    feed: Vendor,
) -> String {
    // ONE LOOKUP FOR THE WHOLE RESPONSE, not one per row. `Vendor::ALL` is four
    // long so the old per-row `find` was cheap, but it was O(vendors) inside an
    // O(entries) loop for a value that cannot change between iterations — and
    // a feed with no census at all is now one comparison rather than one per
    // entry that could never match.
    let Some(census) = censuses.iter().find(|c| c.vendor == feed) else {
        return String::from("[]");
    };
    // THE ANSWER SAYS WHO PRODUCED IT. Until this field existed no byte of a
    // `/store.json` body named the feed, so a reader that got the wrong one had
    // no way to notice — which is half of why substituting Dhan for an unknown
    // feed went unseen for as long as it did. The other half is now a `400`;
    // see `store_json`.
    //
    // ENCODED ONCE, not once per row. `Vendor::as_str` is a fixed token, and
    // allocating its quoted form again for each of the 43,422 entries
    // `docs/06-limits.md` §34 projects is a per-row cost with no reader.
    let feed_json = render::json_string(feed.as_str());
    let mut out = String::from("[");
    let mut n = 0usize;
    for (series, month) in entries {
        let Some(row) = held_row(census, &series.at(*month)) else {
            continue;
        };
        if n > 0 {
            out.push(',');
        }
        n += 1;
        // THE ROW'S OWN RUNG, NOT A LITERAL.
        //
        // This wrote `"timeframe":"1min"` on every row, unconditionally, while
        // `Series` has carried the real `Timeframe` since D-0055 and the store
        // has filed under `1day/` and `1min/` accordingly. The consequences all
        // landed on the reader, and all of them looked like data faults:
        //
        //   * `/db`'s TF column was a sort control over one repeated value —
        //     20,516 identical cells that could never differ.
        //   * `/db`'s completeness divided DAILY bars by a 375-bar MINUTE
        //     session, so a complete 1day month read as ~99.7% missing.
        //   * the Markets chart asked for `…/1min/2021-08.bin` for an
        //     instrument stored under `1day/`, and answered "That read failed"
        //     about a file that was never supposed to exist.
        //
        // The store was right the whole time. `Timeframe::as_str` is the SAME
        // word the path segment uses — `1min`, `1day` — so a reader that keys
        // off this string and a reader that keys off the directory cannot
        // disagree again.
        // THE WINDOW THIS MONTH ACTUALLY COVERS, not just how many bars it
        // holds. The census has carried `first_ts_micros` and `last_ts_micros`
        // per entry since it was written; nothing put them on the wire, so a
        // reader could say "21 bars" and not "09 Aug 2021 to 31 Aug 2021".
        // Deriving them from the month name would be a guess — a month holding
        // one bar covers one day, and which day is a fact only the entry has.
        //
        // Microseconds, unconverted. The browser owns the IST rendering, the
        // same way it owns the decimal point on basis points: one encoding on
        // the wire, one place that formats it. `CLAUDE.md` §7's reasoning
        // applied to time rather than money.
        let _ = write!(
            out,
            r#"{{"feed":{feed_json},"instrument":{},"month":{},"timeframe":{},"rows":{},"first_ts":{},"last_ts":{}"#,
            render::json_string(&series.to_string()),
            render::json_string(&month.to_string()),
            render::json_string(series.timeframe.as_str()),
            row.entry.rows,
            row.entry.first_ts_micros,
            row.entry.last_ts_micros
        );
        write_change(
            &mut out,
            "chg",
            month_change(series.segment, Some(row.closes)),
        );
        // THE PREVIOUS MONTH IS A PROBE, NOT A SCAN, AND NOT AN ASSUMPTION. A
        // month the census does not hold is `no_earlier_month` — never zero,
        // and never this month's number repeated.
        let before = month_before(*month).and_then(|at| held_row(census, &series.at(at)));
        write_change(
            &mut out,
            "prev_chg",
            month_change(series.segment, before.map(|held| held.closes)),
        );
        out.push('}');
    }
    out.push(']');
    out
}

/// One census body's validator, for conditional requests.
///
/// # Why this endpoint of all of them
///
/// `/store.json` is the whole census and it GROWS WITH THE STORE. Measured on a
/// running binary mid-pull: 329 KB, then 377,735 bytes minutes later, for ONE
/// feed. The console polls it every five seconds while a run is in flight —
/// `watchStore(5000)` — so an unchanged store was still costing a full transfer
/// and a full `JSON.parse` on the reader's main thread, every tick, for the
/// length of a run.
///
/// A poll that changes nothing should cost nothing. With a validator it costs a
/// 304 and an empty body.
///
/// # Why the digest and not a counter
///
/// A generation counter would be cheaper to compute and WRONG: it advances when
/// the store is re-read, not when the answer changes, so a re-read that found
/// the same bytes would still force a full transfer. The digest is over the
/// bytes actually being sent, so the validator means what a validator is
/// supposed to mean — this is the same answer you already have.
///
/// `brutex_core::blake3` is the repository's own primitive, the one `CLAUDE.md`
/// §3 rule 3 already uses for run identity. No new dependency.
///
/// # Cost
///
/// O(body) once per immutable census snapshot, vendor and wire format. The
/// wire cache retains both bytes and this digest, so unchanged GETs reuse the
/// encoding and conditional GETs need no new body pass. HEAD omits both.
pub(super) fn census_etag(body: &str) -> String {
    // `hex32` AND NOT A SECOND LOOP. This file already renders 32 bytes as hex
    // at its foot, under a test that pins the width — writing the same fold
    // again here is the duplicate-spelling this repository keeps removing, and
    // it would drift the first time either was touched.
    format!("\"{}\"", hex32(brutex_core::blake3::hash(body.as_bytes())))
}

/// Whether the reader already holds this exact answer.
///
/// `If-None-Match` is a COMMA-SEPARATED LIST and a reader may legitimately send
/// several, so this compares each in turn rather than the header as one string.
/// `*` is deliberately not honoured: it means "if any representation exists",
/// which for an endpoint that always has one would answer 304 forever.
fn none_match_hit(headers: &axum::http::HeaderMap, etag: &str) -> bool {
    headers
        .get(axum::http::header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|sent| sent.split(',').any(|one| one.trim() == etag))
}

pub(super) async fn store_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    asked_with: axum::http::HeaderMap,
    uri: axum::http::Uri,
    method: axum::http::Method,
) -> axum::response::Response {
    use axum::response::IntoResponse as _;
    let head = method == axum::http::Method::HEAD;
    let result =
        match crate::detail::run(move || store_response(&site, &asked_with, &uri, &method)).await {
            Ok(response) => response,
            Err(why) => (
                if matches!(why, crate::detail::RunError::Saturated) {
                    axum::http::StatusCode::TOO_MANY_REQUESTS
                } else {
                    axum::http::StatusCode::SERVICE_UNAVAILABLE
                },
                census_response_headers(matches!(why, crate::detail::RunError::Saturated)),
                serde_json::json!({"error":format!("census read unavailable: {why:?}")})
                    .to_string()
                    .into(),
            ),
        };
    let mut response = result.into_response();
    if head {
        // An exact empty body makes Axum invent Content-Length: 0, which would
        // misdescribe the GET representation. Unknown size permits omission.
        *response.body_mut() =
            axum::body::Body::from_stream(axum::body::Body::empty().into_data_stream());
        response
            .headers_mut()
            .remove(axum::http::header::CONTENT_LENGTH);
    }
    response
}

fn census_response_headers(retry: bool) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    if retry {
        headers.insert(
            axum::http::header::RETRY_AFTER,
            axum::http::HeaderValue::from_static("1"),
        );
    }
    headers
}

pub(super) fn store_response(
    site: &Site,
    asked_with: &axum::http::HeaderMap,
    uri: &axum::http::Uri,
    method: &axum::http::Method,
) -> (
    axum::http::StatusCode,
    axum::http::HeaderMap,
    axum::body::Bytes,
) {
    let query = uri.query().unwrap_or("");
    // REFUSED BY NAME, NEVER SUBSTITUTED. The `.unwrap_or(Vendor::Dhan)` that
    // stood here was unreachable for an ABSENT feed — `parse_vendor` answers
    // the empty string with Dhan itself — so the only input it ever caught was
    // a feed somebody NAMED and this build does not read. See the doc block
    // above for what answering it as Dhan cost.
    let asked = param(query, "feed");
    let Some(feed) = ingest::parse_vendor(&asked) else {
        // The census headers are deliberately NOT stamped: they describe a
        // feed's counter, and there is no feed here to have one. Only the
        // content type, so a browser parses the refusal as what it is.
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
        );
        return (
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            no_such_feed_json(&asked).into(),
        );
    };

    // FRESH, NOT THE STARTUP SNAPSHOT. See `census_now`.
    let format = match param(query, "encoding").as_str() {
        "" => store_wire::Format::Expanded,
        store_wire::ENCODING => store_wire::Format::Compact,
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                census_response_headers(false),
                serde_json::json!({"error":"unrecognised census encoding"})
                    .to_string()
                    .into(),
            );
        }
    };
    let (censuses, entries) = census_now(site);
    let census = censuses.iter().find(|c| c.vendor == feed);
    // 503, FOR THE REASON `health` GIVES: a monitor reads the status code and
    // nothing else. The body stays an array so the readers that only want rows
    // are untouched.
    let code = if census_is_unreadable(census) {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    } else {
        axum::http::StatusCode::OK
    };
    let mut headers = census_headers(census);

    // HEAD is a header read, not a full census encoding whose body Axum drops.
    // Representation headers that require generating a body are omitted.
    if method == axum::http::Method::HEAD {
        return (code, headers, axum::body::Bytes::new());
    }
    let encoded = match site
        .census_wire
        .get(&censuses, feed, format, || match format {
            store_wire::Format::Expanded => Ok(store_body(&censuses, &entries, feed)),
            store_wire::Format::Compact => store_wire::compact(&censuses, &entries, feed),
        }) {
        Ok(encoded) => encoded,
        Err(why) => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                headers,
                serde_json::json!({"error":why}).to_string().into(),
            );
        }
    };
    let etag = &encoded.etag;
    // `from_str` cannot fail on this value — it is quotes and hex — but the
    // header map takes a `Result` and a silently dropped validator would mean a
    // reader never gets a 304 and never knows why. Refusing to guess: if it
    // could not be built the response simply carries no validator, which is the
    // pre-existing behaviour rather than a new failure.
    if let Ok(value) = axum::http::HeaderValue::from_str(etag) {
        headers.insert(axum::http::header::ETAG, value);
    }

    // A MATCH IS A 304 AND NO BODY — but ONLY on a 200.
    //
    // `census_is_unreadable` answers 503 with an empty array, and two different
    // damaged reads produce the same bytes and therefore the same tag. Serving
    // 304 for that would tell a reader "nothing changed" about a state whose
    // whole point is that the counter is broken — and `census_headers` carries
    // the reason in `x-brutex-census-note`, which a 304 body would still deliver
    // but a caching reader might not re-read. A failure keeps its status.
    if code == axum::http::StatusCode::OK && none_match_hit(asked_with, etag) {
        return (
            axum::http::StatusCode::NOT_MODIFIED,
            headers,
            axum::body::Bytes::new(),
        );
    }

    (code, headers, encoded.body)
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
    use crate::server::percentage_tests::{census_of, month, priced, series, series_at};
    use brutex_core::instrument::Segment;
    use pull::manifest::Closes;
    use store::path::Timeframe;

    /// The one row `store_body` produced, so an assertion names a field rather
    /// than a substring of the whole array.
    fn only_row(body: &str) -> String {
        let inner = body
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .expect("a JSON array");
        assert!(
            !inner.contains("},{"),
            "this helper is for a single-row body: {body}"
        );
        inner.to_owned()
    }

    /// **A STORE ROW NAMES THE RUNG IT IS STORED AT, NEVER A FIXED WORD.**
    ///
    /// `store_body` wrote `"timeframe":"1min"` as a literal on every row while
    /// `Series` carried the real `Timeframe`. Every test in this module built
    /// its series at `MINUTE_1`, so all 321 of them agreed with the literal and
    /// none of them could tell the difference — the writer was reverted to the
    /// literal and the suite stayed green.
    ///
    /// This test builds a `DAY_1` series specifically so it cannot: it asserts
    /// the row says `1day` AND that it does not say `1min`, which is what makes
    /// reverting the writer a failure rather than a wash.
    #[test]
    fn a_store_row_names_the_rung_it_is_stored_at_and_never_a_fixed_word() {
        let daily = series_at(Segment::Index, "NIFTY", Timeframe::DAY_1);
        let at = month(2026, 7);
        let census = census_of(&[(daily, at, 23, priced(10_000_000, 10_125_000))]);
        let body = only_row(&store_body(&[census], &[(daily, at)], Vendor::Groww));
        assert!(
            body.contains(r#""timeframe":"1day""#),
            "a 1day series must say 1day: {body}"
        );
        assert!(
            !body.contains(r#""timeframe":"1min""#),
            "a 1day series must not say 1min: {body}"
        );

        // The other rung, from the same writer, so the assertion above is
        // reading the field and not a word that happens to be everywhere.
        let minute = series_at(Segment::Index, "NIFTY", Timeframe::MINUTE_1);
        let census = census_of(&[(minute, at, 8_250, priced(10_000_000, 10_125_000))]);
        let body = only_row(&store_body(&[census], &[(minute, at)], Vendor::Groww));
        assert!(
            body.contains(r#""timeframe":"1min""#),
            "a 1min series must say 1min: {body}"
        );
    }

    /// **A MONTH WITH NO CLOSE RECORDED IS UNKNOWN, AND NEVER ZERO.**
    ///
    /// Every version-1 census entry reads back this way, and so does every
    /// month this build committed before it read a bar file — D-0067. On the
    /// wire it is `null` and a code, so the browser cannot render it as a
    /// number by accident.
    #[test]
    fn a_month_with_no_close_recorded_is_unknown_and_never_zero() {
        assert_eq!(
            month_change(Segment::Index, Some(Closes::UNKNOWN)),
            Err(Unknown::NotRecorded)
        );
        let nifty = series(Segment::Index, "NIFTY");
        let at = month(2026, 7);
        let body = store_body(
            &[census_of(&[(nifty, at, 8_250, Closes::UNKNOWN)])],
            &[(nifty, at)],
            Vendor::Groww,
        );
        let row = only_row(&body);
        assert!(
            row.contains(r#""chg_bps":null,"chg_why":"not_recorded""#),
            "unknown on the wire is null and a reason: {row}"
        );
        assert!(
            !row.contains(r#""chg_bps":0"#),
            "and it is never a zero: {row}"
        );
    }

    /// **AN INSTRUMENT'S FIRST MONTH HAS A CHANGE AND NO PREVIOUS CHANGE.**
    ///
    /// This is the property that decided the reading. Because `Δ %` is the
    /// month's own first close to its own last close, it is defined for every
    /// held month including the first — and only the "previous" column has a
    /// missing-neighbour case. That case is `no_earlier_month`, spelled out,
    /// and it is not `0.00%`.
    #[test]
    fn an_instruments_first_month_has_a_change_and_no_previous_change() {
        let nifty = series(Segment::Index, "NIFTY");
        let first = month(2026, 6);
        let second = month(2026, 7);
        let census = census_of(&[
            (nifty, first, 8_250, priced(10_000, 10_125)),
            (nifty, second, 8_250, priced(10_000, 10_500)),
        ]);
        let body = store_body(
            &census_slice(&census),
            &[(nifty, second), (nifty, first)],
            Vendor::Groww,
        );

        assert!(
            body.contains(r#""chg_bps":500,"chg_why":null,"prev_chg_bps":125,"prev_chg_why":null"#),
            "July renders its own +5.00% and June's +1.25% beside it: {body}"
        );
        assert!(
            body.contains(r#""chg_bps":125,"chg_why":null,"prev_chg_bps":null,"prev_chg_why":"no_earlier_month""#),
            "June has its own number and NO previous — not a zero: {body}"
        );
        assert!(
            !body.contains(r#""prev_chg_bps":0"#),
            "a missing neighbour is never rendered as no change: {body}"
        );
    }

    /// One census as the slice `store_body` takes.
    fn census_slice(census: &census::VendorCensus) -> [census::VendorCensus; 1] {
        [census.clone()]
    }

    /// **A ROW AND ITS NEIGHBOUR COST TWO PROBES AND NO FILE.**
    ///
    /// Two things are asserted, and each is the falsifiable half of a claim the
    /// doc comments make.
    ///
    /// **No file.** The census is built in memory and its `path` names a
    /// directory that does not exist. Every number below therefore came out of
    /// the manifest. A change that reached for a bar file to price a month
    /// would fail here rather than pass slowly, which is the difference between
    /// this design and the 17.5 GB route D-0067 measured and rejected.
    ///
    /// **One month back, never a search.** June and August are held and July is
    /// not. August's previous change is `no_earlier_month` — it does NOT walk
    /// back to June. That is what makes the lookup a probe: a scan would find
    /// June, print a number spanning two months, and label it as one.
    #[test]
    fn a_row_and_its_neighbour_cost_two_probes_and_no_file() {
        let nifty = series(Segment::Index, "NIFTY");
        let june = month(2026, 6);
        let august = month(2026, 8);
        let census = census_of(&[
            (nifty, june, 8_250, priced(10_000, 10_125)),
            (nifty, august, 8_250, priced(10_000, 10_200)),
        ]);
        assert!(
            !census.path.exists(),
            "the fixture must not have a file behind it, or it proves nothing"
        );
        let body = store_body(
            &census_slice(&census),
            &[(nifty, august), (nifty, june)],
            Vendor::Groww,
        );
        assert!(
            body.contains(r#""chg_bps":200,"chg_why":null,"prev_chg_bps":null,"prev_chg_why":"no_earlier_month""#),
            "August reads July, finds nothing, and stops — it does not reach June: {body}"
        );
        assert!(
            !body.contains(r#""chg_bps":200,"chg_why":null,"prev_chg_bps":125"#),
            "reaching past the gap would print a June number on an August row: {body}"
        );
    }

    /// **EVERY ROW CARRIES A NUMBER OR A NAMED REASON, AND NEVER BOTH.**
    ///
    /// The wire shape is total: both keys of each pair are present on every
    /// row, and exactly one of them is non-`null`. A key that is sometimes
    /// absent makes "unknown" and "this build has no such field" the same thing
    /// in the browser, and a row carrying both would be a number with an
    /// excuse — which is the shape `CLAUDE.md` §4 bans.
    #[test]
    fn every_row_carries_a_number_or_a_named_reason_and_never_both() {
        let nifty = series(Segment::Index, "NIFTY");
        let reliance = series(Segment::Cash, "RELIANCE");
        let june = month(2026, 6);
        let july = month(2026, 7);
        let census = census_of(&[
            (nifty, june, 8_250, priced(10_000, 10_125)),
            (nifty, july, 8_250, Closes::UNKNOWN),
            (reliance, june, 8_250, priced(295_050, 310_025)),
            (reliance, july, 8_250, priced(310_025, 62_005)),
        ]);
        let body = store_body(
            &census_slice(&census),
            &[
                (nifty, july),
                (reliance, july),
                (nifty, june),
                (reliance, june),
            ],
            Vendor::Groww,
        );

        // Four rows, four of each key, and no key ever missing.
        for field in [
            "\"chg_bps\":",
            "\"chg_why\":",
            "\"prev_chg_bps\":",
            "\"prev_chg_why\":",
        ] {
            assert_eq!(
                body.matches(field).count(),
                4,
                "{field} appears on every row exactly once: {body}"
            );
        }
        // A number and a reason never co-occur: every `_bps` that is a number
        // is followed by a `_why` of null, and vice versa.
        assert_eq!(
            body.matches("_bps\":null").count() + body.matches("_why\":null").count(),
            8,
            "one of each pair is null on every row, and only one: {body}"
        );

        assert!(
            body.contains(
                r#""chg_bps":null,"chg_why":"not_recorded","prev_chg_bps":125,"prev_chg_why":null"#
            ),
            "an unpriced index month is unknown and its priced neighbour is not: {body}"
        );
        // THE 80% FALL D-0018 IS ABOUT. ₹3,100.25 to ₹620.05 is a 1:5 split
        // read as an -8,000 bp crash, and it is exactly the number this page
        // must never print.
        assert!(
            body.contains(r#""chg_bps":null,"chg_why":"corporate_action_unverified","prev_chg_bps":null,"prev_chg_why":"corporate_action_unverified""#),
            "the equity is refused in both columns, by name: {body}"
        );
        assert!(
            !body.contains("-8000"),
            "the fabricated crash never reaches the wire: {body}"
        );
    }

    /// **A FEED WITH NO CENSUS IS AN EMPTY ARRAY, NOT A ROW OF ZEROES.**
    ///
    /// The page renders "nothing stored for this feed" off an empty array and
    /// names where the rows actually are. It has no way to distinguish a zeroed
    /// row from a real one, so there must never be a zeroed row.
    #[test]
    fn a_feed_with_no_census_is_an_empty_array() {
        let nifty = series(Segment::Index, "NIFTY");
        let at = month(2026, 7);
        let census = census_of(&[(nifty, at, 8_250, priced(10_000, 10_125))]);
        assert_eq!(
            store_body(&census_slice(&census), &[(nifty, at)], Vendor::Dhan),
            "[]",
            "a feed this store has never held answers with nothing"
        );
        assert_eq!(
            store_body(&[], &[(nifty, at)], Vendor::Groww),
            "[]",
            "and so does a store with no censuses at all"
        );
        // A MONTH THE CENSUS DOES NOT HOLD IS SKIPPED, not emitted empty.
        assert_eq!(
            store_body(
                &census_slice(&census),
                &[(nifty, month(2026, 5))],
                Vendor::Groww
            ),
            "[]"
        );
    }

    #[test]
    fn a_census_validator_answers_for_the_bytes_and_not_for_the_read() {
        let one = census_etag("[]");
        let same = census_etag("[]");
        let other = census_etag(r#"[{"instrument":"BANKNIFTY"}]"#);

        assert_eq!(
            one, same,
            "the same bytes are the same answer — a validator that changed on a \
             re-read would force a full transfer for an unchanged census, which \
             is the whole cost this exists to remove"
        );
        assert_ne!(one, other, "different bytes are a different answer");
        assert_eq!(one.len(), 66, "a quoted 32-byte digest: {one}");
        assert!(
            one.starts_with('"') && one.ends_with('"'),
            "an ETag is a quoted-string by RFC 9110: {one}"
        );
    }

    #[test]
    fn a_reader_holding_the_answer_is_recognised_and_a_wildcard_is_not() {
        let etag = census_etag("[]");
        let with = |value: &str| {
            let mut map = axum::http::HeaderMap::new();
            map.insert(
                axum::http::header::IF_NONE_MATCH,
                axum::http::HeaderValue::from_str(value).expect("a header value"),
            );
            map
        };

        assert!(none_match_hit(&with(&etag), &etag), "the exact tag");
        assert!(
            none_match_hit(&with(&format!("\"deadbeef\", {etag}")), &etag),
            "If-None-Match is a LIST and a reader may legitimately send several"
        );
        assert!(
            !none_match_hit(&with("\"deadbeef\""), &etag),
            "a tag for another answer is not this answer"
        );
        assert!(
            !none_match_hit(&with("*"), &etag),
            "`*` means \"if any representation exists\", which for an endpoint \
             that always has one would answer 304 for ever"
        );
        assert!(
            !none_match_hit(&axum::http::HeaderMap::new(), &etag),
            "a reader holding nothing gets the body"
        );
    }
}
