//! `POST /page-report`: a page says it failed (R3-14).
//!
//! # What was missing
//!
//! A page that threw, or drew an answer it could not read, failed in the
//! operator's browser and nowhere else: the server saw a `200` for the data
//! and the log said nothing, so "the page broke at 14:02" had no record
//! anyone could search.
//!
//! # The rule
//!
//! * One report is a form body, `page=<path>&what=<what failed>`, at most
//!   [`MAX_REPORT_BYTES`] (1 KiB) whole. A longer one is refused `413`, an
//!   empty or malformed one `400` naming the field, before anything is
//!   written.
//! * At most [`REPORTS_PER_WINDOW`] reports a window are written, as
//!   `api.page.access page reported a failure` at `warn`, which the audit
//!   stream keeps (an `access` target, `telemetry::is_audited`). Past it a
//!   report is counted and answered `429`; the next window's first written
//!   report carries how many were kept out before it, and every answer carries
//!   the running totals. A page in a loop cannot roll the audit stream.
//! * Like every `POST`, it is admitted only from this server's own page
//!   (`server::same_origin_writes_only`), and a read-only or unstamped server
//!   refuses it as it refuses every write (that gate decides by method).
//!
//! Cost: O(1) a report — one parse of at most 1 KiB (a longer body is refused
//! before it is read for fields:
//! `api::page_report_route::tests::a_page_report_is_rationed_and_audited`), one
//! lock, one line.

use std::sync::atomic::{AtomicU64, Ordering};

/// The largest report body, bytes.
pub const MAX_REPORT_BYTES: usize = 1_024;

/// The longest `page` path, bytes.
pub const MAX_PAGE_BYTES: usize = 256;

/// Reports one window writes.
pub const REPORTS_PER_WINDOW: u64 = 30;

/// Reports written since the start.
static WRITTEN: AtomicU64 = AtomicU64::new(0);

/// Reports kept out by the ration since the start.
static KEPT_OUT: AtomicU64 = AtomicU64::new(0);

/// The ration, one window for every page.
static RATION: std::sync::Mutex<crate::logs::Ration> =
    std::sync::Mutex::new(crate::logs::Ration::new(REPORTS_PER_WINDOW));

/// A report, read and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The page's path, `/db`.
    pub page: String,
    /// What failed, as the page said it.
    pub what: String,
}

/// Why a report was refused before anything was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The body is longer than [`MAX_REPORT_BYTES`].
    TooLong(usize),
    /// A field is missing, empty, too long or carries a control character.
    Field(&'static str, &'static str),
}

impl Refusal {
    /// The status it answers with.
    #[must_use]
    pub const fn status(&self) -> u16 {
        match self {
            Self::TooLong(_) => 413,
            Self::Field(..) => 400,
        }
    }

    /// The sentence it answers with.
    #[must_use]
    pub fn sentence(&self) -> String {
        match self {
            Self::TooLong(n) => format!(
                "a page report is at most {MAX_REPORT_BYTES} bytes; this one was {n}: nothing \
                 was written"
            ),
            Self::Field(name, why) => format!("the report's {name}= {why}: nothing was written"),
        }
    }
}

/// Reads one report body.
///
/// # Errors
///
/// [`Refusal`] when the body is too long or a field is missing, empty, too
/// long or carries a control character.
pub fn read(body: &str) -> Result<Report, Refusal> {
    if body.len() > MAX_REPORT_BYTES {
        return Err(Refusal::TooLong(body.len()));
    }
    let page = crate::server::param(body, "page");
    let what = crate::server::param(body, "what");
    if page.is_empty() {
        return Err(Refusal::Field("page", "is missing or empty"));
    }
    if !page.starts_with('/') {
        return Err(Refusal::Field("page", "is not a path of this server"));
    }
    if page.len() > MAX_PAGE_BYTES {
        return Err(Refusal::Field("page", "is longer than 256 bytes"));
    }
    if what.trim().is_empty() {
        return Err(Refusal::Field("what", "is missing or empty"));
    }
    if page.chars().any(char::is_control) {
        return Err(Refusal::Field("page", "carries a control character"));
    }
    if what
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(Refusal::Field("what", "carries a control character"));
    }
    Ok(Report { page, what })
}

/// What one report did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// Whether its line was written.
    pub written: bool,
    /// What the log did with its line: `None` when the ration kept it out
    /// before the log was asked (R3-14 repair 1: this was ignored, and a line
    /// the log dropped was answered `202 written`).
    pub log: Option<telemetry::Emitted>,
    /// Reports written since the start, this one included.
    pub written_total: u64,
    /// Reports kept out since the start, this one included.
    pub kept_out_total: u64,
}

/// Writes `report` under the ration, at `now_ms`, and says what it did.
pub fn note(report: &Report, now_ms: u64) -> Outcome {
    let admit = RATION
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .admit(now_ms);
    if !admit.write {
        let kept_out_total = KEPT_OUT.fetch_add(1, Ordering::Relaxed).saturating_add(1);
        return Outcome {
            written: false,
            log: None,
            written_total: WRITTEN.load(Ordering::Relaxed),
            kept_out_total,
        };
    }
    let mut line = telemetry::Event::warn("api.page.access", "page reported a failure")
        .with("page", telemetry::Value::Str(&report.page))
        .with("what", telemetry::Value::Str(&report.what));
    if let Some(kept_out) = admit.summary {
        line = line.with("kept_out_before", telemetry::Value::Uint(kept_out));
    }
    // THE LOG'S ANSWER IS READ (R3-14 repair 1): only a line that reached
    // the file is counted written and answered `202 written`.
    let log = telemetry::emit(&line);
    let written = log.is_written();
    let written_total = if written {
        WRITTEN.fetch_add(1, Ordering::Relaxed).saturating_add(1)
    } else {
        WRITTEN.load(Ordering::Relaxed)
    };
    Outcome {
        written,
        log: Some(log),
        written_total,
        kept_out_total: KEPT_OUT.load(Ordering::Relaxed),
    }
}

/// The status and the sentence one report's [`Outcome`] earns: `202` only for
/// a line the log wrote; `429` when this route's own ration kept it out;
/// `202` with `written: false` when the log's own floor or flood ration held
/// it back (counted there, and said by its next line); `503` when the log
/// could not take it at all, which is a failure the page must see.
#[must_use]
pub fn status_of(done: &Outcome) -> (u16, &'static str) {
    match done.log {
        None => (
            429,
            "counted, not written: the page-report ration of this minute is spent",
        ),
        Some(telemetry::Emitted::Written) => {
            (202, "written to the log, which the audit stream keeps")
        }
        Some(telemetry::Emitted::Filtered) => (
            202,
            "counted, not written: the log's own floor or flood ration held the line back \
             and says so in its next line",
        ),
        Some(telemetry::Emitted::Dropped | telemetry::Emitted::NotInstalled) => (
            503,
            "NOT WRITTEN: the log could not take the line (dropped, or no log is open); \
             /logs names why",
        ),
    }
}

/// The answer to one body at `now_ms`: a status and its JSON.
#[must_use]
pub fn answer(body: &str, now_ms: u64) -> (u16, String) {
    match read(body) {
        Err(refusal) => (
            refusal.status(),
            serde_json::json!({ "written": false, "error": refusal.sentence() }).to_string(),
        ),
        Ok(report) => {
            let done = note(&report, now_ms);
            let (status, why) = status_of(&done);
            (
                status,
                serde_json::json!({
                    "written": done.written,
                    "why": why,
                    "written_total": done.written_total,
                    "kept_out_total": done.kept_out_total,
                    "per_window": REPORTS_PER_WINDOW,
                })
                .to_string(),
            )
        }
    }
}

/// The handler.
pub async fn page_report(
    body: String,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 2],
    String,
) {
    let (status, json) = answer(&body, crate::speed::now_ms());
    (
        axum::http::StatusCode::from_u16(status)
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/json; charset=utf-8",
            ),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        json,
    )
}

#[cfg(test)]
#[path = "page_report_route_tests.rs"]
mod tests;
