//! BASE-COV-PULL-PS1 (D-7335): a stored answer's gap line that is not a
//! repair, and a named ledger handed no lines. Every stamp and price below is
//! invented.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::calendar::Runtime;
use crate::fetch::BarRequest;
use crate::session::Window;

/// One invented minute row at `timestamp`.
const fn row(timestamp: i64) -> RawRow {
    RawRow {
        timestamp,
        open: 100,
        high: 100,
        low: 100,
        close: 100,
        volume: 1,
        open_interest: None,
    }
}

/// An audited NSE index minute plan over `request`.
fn plan(request: &BarRequest) -> Plan<'_> {
    Plan {
        calendar: Runtime::default(),
        cash_schedule: None,
        columns: crate::csv::Columns::TrueDataIndex,
        request,
        encoding: TimestampEncoding::EpochSecondsUtc,
        scale: crate::vendor::PriceScale::Paisa,
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
        run: 0,
    }
}

/// **A STORED ANSWER'S GAP THAT IS NOT A REPAIR CARRIES NO STORE ORIGIN**:
/// an answer read back from the store (`from_store`) whose day the vendor
/// left short is a `vendor_empty` line with no `origin=store` mark, because
/// that mark names a repaired day and this one was not repaired. Extremes at
/// once: the store origin and the first five minutes missing.
#[test]
fn a_stored_answers_gap_that_is_not_a_repair_carries_no_store_origin() {
    let day = Day::new(2022, 10, 3).unwrap();
    let request = BarRequest {
        instrument_id: String::new(),
        listing: crate::vendor::Listing::Index,
        window: Window::new(day, day).unwrap(),
        granularity: Granularity::Minute1,
    };
    let at =
        |minute: i64| i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60;
    let rows: Vec<RawRow> = (5..375).map(|minute| row(at(555 + minute))).collect();
    let root = crate::gdfl_fixtures::scratch("ps1-report-store");
    let mut done = Ingested::default();
    report(
        &rows,
        "NIFTY",
        &root,
        plan(&request),
        &mut done,
        (&Refusals::default(), true),
    );
    assert_eq!(done.failures.len(), 1, "{:?}", done.failures);
    let ledger = std::fs::read_to_string(root.join("audit").join("zerodha.gaps")).unwrap();
    let lines: Vec<&str> = ledger.lines().collect();
    assert_eq!(lines.len(), 1, "{ledger}");
    assert!(lines[0].contains(" why=vendor_empty "), "{}", lines[0]);
    assert!(!lines[0].contains("origin=store"), "{}", lines[0]);
    std::fs::remove_dir_all(root).unwrap();
}

/// **A NAMED LEDGER HANDED NO LINES WRITES NOTHING AND OPENS NOTHING**: not
/// the audit folder, not the file, and it answers `Ok`. Extremes at once: a
/// store root that holds nothing at all, and a ledger name no caller uses.
#[test]
fn a_named_ledger_handed_no_lines_writes_nothing() {
    let root = crate::gdfl_fixtures::scratch("ps1-append-empty");
    append_named(
        &root,
        brutex_core::vendor::Vendor::Zerodha,
        "zq-invented",
        &[],
    )
    .unwrap();
    assert!(!root.join("audit").exists(), "nothing was made");
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    std::fs::remove_dir_all(root).unwrap();
}
