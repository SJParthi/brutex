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
//! `(front end)`, `stage:<name>`); a name never asked answers an empty row
//! list, which is the truth, not an error.
//!
//! Cost: O(rows x buckets) counter reads, at most 256 x 272, three times. It
//! does not grow with the requests served or with the store; measured in this
//! package's report (p50, p99, max at n = 200, 1 and 60 clients).

/// The JSON the route answers with, at `now_ms`, for `query`.
///
/// # Errors
///
/// The serialiser's refusal, as a JSON error body: the handler answers it
/// `500`, never a `200` that reads like a report.
pub fn answer(table: &crate::speed::Speed, query: &str, now_ms: u64) -> Result<String, String> {
    let mut report = table.report(now_ms);
    let only = crate::server::param(query, "route");
    if !only.is_empty() {
        report.rows.retain(|row| row.route == only);
    }
    serde_json::to_string(&report).map_err(|why| {
        format!(
            "{{\"error\":\"the speed report would not serialise: {}\"}}",
            why.to_string().replace(['"', '\\'], "'")
        )
    })
}

/// The handler.
pub async fn speed_json(
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 2],
    String,
) {
    let (status, body) = match answer(
        crate::speed::global(),
        uri.query().unwrap_or(""),
        crate::speed::now_ms(),
    ) {
        Ok(body) => (axum::http::StatusCode::OK, body),
        Err(body) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, body),
    };
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
