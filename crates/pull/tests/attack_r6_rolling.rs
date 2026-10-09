//! ATTACK ROUND 6, the decoded-bar door: a batch that crosses a month.
//!
//! `split_window` caps a rolling-option chunk at 45 days and, since D-0320 and
//! D-1370, lets that chunk cross a month boundary. The spot door files such a
//! batch one month per file (`months_in`); the decoded door `from_rows`
//! addressed ONE month from the first and last bar and refused a batch whose
//! bars span two, so a contract run across a month end never landed, and every
//! rerun refused the same way (D-3136).
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::cast_possible_truncation
)]

use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::fetch::BarRequest;
use pull::ingest::Plan;
use pull::session::{Day, Window};
use pull::vendor::{Granularity, Listing, TimestampEncoding};
use store::path::Timeframe;

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("brutex-attack-r6-{tag}-{}", std::process::id()));
        let _best_effort = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch root");
        Self(dir)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir_all(&self.0);
    }
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real day")
}

fn plan(request: &BarRequest) -> Plan<'_> {
    Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::TrueDataIndex,
        request,
        encoding: TimestampEncoding::IsoDateTimeOffset,
        scale: pull::http::DECODED_PRICE_SCALE,
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
    }
}

/// A bar at `hh:mm` IST on `d`.
fn bar(d: Day, hh: i64, mm: i64) -> store::format::Bar {
    let midnight_utc = i64::from(d.days_from_epoch()) * 86_400 - 19_800;
    store::format::Bar {
        ts_micros: (midnight_utc + hh * 3_600 + mm * 60) * 1_000_000,
        open: 2_500_000,
        high: 2_500_000,
        low: 2_500_000,
        close: 2_500_000,
        volume: 1,
        open_interest: store::format::OI_NULL,
    }
}

fn committed(root: &Path, year: u16, month: u8) -> Vec<store::format::Bar> {
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: store::path::YearMonth::new(year, month).unwrap(),
        file: store::path::FileKind::Bars,
    })
    .unwrap();
    let Ok(file) = store::file::BarFile::open_existing(
        root,
        path,
        brutex_core::universe::fnv1a("NIFTY") as u32,
    ) else {
        return Vec::new();
    };
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

#[test]
fn a_decoded_batch_across_a_month_end_is_filed_one_month_per_file() {
    // Monday 2025-06-30 15:29 IST and Tuesday 2025-07-01 09:15 IST, inside a
    // capped chunk that starts in June and ends in July.
    let request = BarRequest {
        instrument_id: String::new(),
        listing: Listing::Index,
        window: Window::new(day(2025, 6, 20), day(2025, 7, 10)).expect("a window"),
        granularity: Granularity::Minute1,
    };
    let scratch = Scratch::new("month-span");
    let bars = [bar(day(2025, 6, 30), 15, 29), bar(day(2025, 7, 1), 9, 15)];
    let done = pull::ingest::from_rows(&bars, &[], "NIFTY", "attack", &scratch.0, plan(&request));
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.bars_committed, 2);
    assert!(done.balances(), "{done:?}");
    assert_eq!(committed(&scratch.0, 2025, 6), [bars[0]]);
    assert_eq!(committed(&scratch.0, 2025, 7), [bars[1]]);
    // One census row per month filed, both recorded in one cycle.
    let pending: Vec<_> = done.pending.into_iter().collect();
    assert_eq!(pending.len(), 2, "{pending:?}");
    assert!(pull::ingest::record_held(&scratch.0, Vendor::Zerodha, &pending).is_none());

    // A RERUN CHANGES NOTHING and refuses nothing.
    let again = pull::ingest::from_rows(&bars, &[], "NIFTY", "attack", &scratch.0, plan(&request));
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert_eq!(again.bars_committed, 0);
}

/// ROUND 7 (D-3700). A member refused at the address stage names the endpoint
/// it came from, as the session-stage refusal and the landed path do.
#[test]
fn an_address_stage_refusal_names_its_origin() {
    let request = BarRequest {
        instrument_id: String::new(),
        listing: Listing::Index,
        window: Window::new(day(2025, 6, 20), day(2025, 7, 10)).expect("a window"),
        granularity: Granularity::Minute1,
    };
    // Its own root: the two tests run at once in one process, and a shared
    // tag let either one's `Drop` remove the other's store mid-run (D-4612).
    let scratch = Scratch::new("address-stage");
    let bars = [bar(day(2025, 6, 30), 15, 29)];
    let done = pull::ingest::from_rows(
        &bars,
        &[],
        "NIFTY",
        "attack",
        &scratch.0,
        Plan {
            exchange: "BSE",
            ..plan(&request)
        },
    );
    assert_eq!(done.failures.len(), 1, "{:?}", done.failures);
    assert!(
        done.failures[0].why.ends_with("(from attack)"),
        "{:?}",
        done.failures
    );
}
