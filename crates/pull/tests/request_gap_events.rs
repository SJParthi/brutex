//! ONE REQUEST-MINUTE GAP IS ONE LOG LINE. OD-2, D-2371, invariant AFG-71.
//!
//! `ingest::from_window` turns each line of `request_minutes::audit` into a
//! receipt `Failure` and emits one `pull.request_minutes` event for it. Until
//! D-2371 the audit ALSO emitted a `pull.file` "not filed" event for the same
//! line, under the placeholder instrument "requested window", so each gap was
//! two events while `docs/06-limits.md` said one.
//!
//! Its own test binary because `telemetry::install` is a process singleton:
//! no sibling test can write a gap line into this log and inflate the count.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::PathBuf;

use pull::fetch::RawRow;

/// 2025-07-01 09:15:00 IST as a UTC epoch second. A Tuesday.
const OPEN_UTC: i64 = 1_751_341_500;

fn row(minute: i64) -> RawRow {
    RawRow {
        timestamp: OPEN_UTC + minute * 60,
        open: 2_550_000,
        high: 2_550_040,
        low: 2_549_960,
        close: 2_550_010,
        volume: 400,
        open_interest: Some(500_000),
    }
}

fn request() -> pull::fetch::BarRequest {
    let day = pull::session::Day::new(2025, 7, 1).expect("a real day");
    pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Index,
        window: pull::session::Window::new(day, day).expect("a legal window"),
        granularity: pull::vendor::Granularity::Minute1,
    }
}

fn plan(request: &pull::fetch::BarRequest) -> pull::ingest::Plan<'_> {
    pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::TrueDataIndex,
        request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: brutex_core::vendor::Vendor::TrueData,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
    }
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brutex-gap-events-{tag}-{}", std::process::id()));
    let _best_effort = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch root");
    dir
}

/// The log lines naming a request-coverage line.
fn coverage_lines(log: &str) -> Vec<&str> {
    log.lines()
        .filter(|line| line.contains("request minute coverage"))
        .collect()
}

#[test]
fn each_request_minute_gap_is_exactly_one_error_event() {
    let logs = scratch("gap-receipt");
    let config = telemetry::Config::new(&logs).with_min_level(telemetry::Level::Trace);
    let sink = telemetry::install(&config).expect("the only install in this binary");
    let req = request();

    // Minutes 0..5 and 60..75 absent: two gaps, two receipt failures.
    let rows = (0..375)
        .filter(|m| *m >= 5 && !(60..75).contains(m))
        .map(row)
        .collect();
    let store = scratch("store");
    let done = pull::ingest::from_window(
        &pull::fetch::RawWindow {
            rows,
            skipped: pull::fetch::DecodeSkips::default(),
        },
        "NIFTY",
        "test",
        &store,
        plan(&req),
    );
    let gaps = done
        .failures
        .iter()
        .filter(|f| f.why.starts_with("request minute coverage gap"))
        .count();
    assert_eq!(gaps, 2, "the premise: two gaps on the receipt");

    // An empty response is the whole session missing: one more gap.
    let empty = scratch("empty");
    let none = pull::ingest::from_window(
        &pull::fetch::RawWindow {
            rows: Vec::new(),
            skipped: pull::fetch::DecodeSkips::default(),
        },
        "NIFTY",
        "test",
        &empty,
        plan(&req),
    );
    assert!(
        none.failures
            .iter()
            .any(|f| f.why.contains("375 missing scheduled minutes"))
    );

    sink.sync().expect("the log is durable before it is read");
    let log = std::fs::read_to_string(telemetry::current_path(&logs)).expect("the log");
    let lines = coverage_lines(&log);
    assert_eq!(
        lines.len(),
        3,
        "one event per gap, not two (pull.file + pull.request_minutes): {lines:#?}"
    );
    for line in &lines {
        assert!(line.contains("\"pull.request_minutes\""), "{line}");
        assert!(
            line.contains("\"error\""),
            "a receipt failure logs at error: {line}"
        );
        assert!(
            line.contains("\"NIFTY\""),
            "names the real instrument: {line}"
        );
        assert!(!line.contains("requested window"), "no placeholder: {line}");
    }
    assert_eq!(
        sink.health().dropped,
        0,
        "nothing dropped, so the count is exact"
    );

    for dir in [logs, store, empty] {
        let _best_effort = std::fs::remove_dir_all(dir);
    }
}
