//! `GET /speed.json`: how fast every route of this server has answered, kept
//! by the server itself (R3-14; RA-014).
//!
//! The figures are [`crate::speed`]'s: for every route asked since the start,
//! and every import or checking stage counted, the requests, how they ended
//! (answered, refused, busy, failed, cancelled), and p50, p99 and the max —
//! since the start, over the last whole minute and over the last hour of whole
//! minutes — and the first, cold, request after the start.
//!
//! `?route=<path>` answers that one row (an exact name: `/bars.json`,
//! `(front end)`, `stage:<name>`).
//!
//! # A name with no row is a `404` that says so (R3-14 repair 4)
//!
//! It answered `200` with an empty row list, which reads exactly like a route
//! that was asked and measured nothing — the silent answer the standing rules
//! refuse. A row exists from the moment its route is first asked (or its
//! stage first counted) in this process, so a name with no row is one of
//! three things, and the body names all three instead of guessing: not asked
//! since `started_ms`, counted under [`crate::speed::OVER_CAP`] because the
//! table was full (the body carries that count), or not a route at all. The
//! names the table does keep are listed, in the order first asked, so the
//! caller can see what it could have asked for.
//!
//! Cost: O(rows x buckets) counter reads, at most 256 x 272, three times. It
//! does not grow with the requests served or with the store; measured in this
//! package's report (p50, p99, max at n = 200, 1 and 60 clients). The `404`
//! adds one pass over at most 256 names.

use axum::http::StatusCode;

/// The status and JSON the route answers with, at `now_ms`, for `query`.
///
/// `200` and the report; `404` and a JSON error body when `route=` names no
/// row (see the module's note); `500` and a JSON error body when the
/// serialiser refuses, never a `200` that reads like a report.
#[must_use]
pub fn answer(table: &crate::speed::Speed, query: &str, now_ms: u64) -> (StatusCode, String) {
    let mut report = table.report(now_ms);
    let only = crate::server::param(query, "route");
    if !only.is_empty() {
        // One pass over at most 256 rows: the row asked for, and the others.
        let (asked, others): (Vec<_>, Vec<_>) = std::mem::take(&mut report.rows)
            .into_iter()
            .partition(|row| row.route == only);
        if asked.is_empty() {
            let kept: Vec<&str> = others.iter().map(|row| row.route.as_str()).collect();
            let body = serde_json::json!({
                "error": format!("no speed row is named '{only}'"),
                "why": format!(
                    "a row exists from the first time its route is asked, or its stage \
                     counted, in this process; '{only}' has not been since started_ms, or was \
                     counted under '{}' because the table was full ({} requests there), or is \
                     not a route of this server",
                    crate::speed::OVER_CAP,
                    report.over_cap
                ),
                "started_ms": report.started_ms,
                "now_ms": report.now_ms,
                "over_cap": report.over_cap,
                "rows_kept": kept,
            });
            return (StatusCode::NOT_FOUND, body.to_string());
        }
        report.rows = asked;
    }
    match serde_json::to_string(&report) {
        Ok(body) => (StatusCode::OK, body),
        Err(why) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "{{\"error\":\"the speed report would not serialise: {}\"}}",
                why.to_string().replace(['"', '\\'], "'")
            ),
        ),
    }
}

/// The handler.
pub async fn speed_json(
    uri: axum::http::Uri,
) -> (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 2],
    String,
) {
    let (status, body) = answer(
        crate::speed::global(),
        uri.query().unwrap_or(""),
        crate::speed::now_ms(),
    );
    (
        status,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/json; charset=utf-8",
            ),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        body,
    )
}
