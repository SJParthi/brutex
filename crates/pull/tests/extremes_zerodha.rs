//! Extreme cases of the Zerodha (Kite) minute import, for the cloud extremes
//! round of 10 Nov 2026 (`EXTREMES-REPORT.md` at the repository root).
//!
//! Two doors, both pure of the network: `pull::http::decode_body` over a Kite
//! JSON body with the shipped Zerodha descriptor, and `pull::ingest::
//! from_window` over decoded rows into a scratch store, the way
//! `tests/zerodha_repair.rs` drives it. What is added here:
//!
//! * an empty answer, and positional rows of the wrong width or cell type;
//! * a `null` volume in a positional (Kite) row;
//! * an answer whose candles are out of order;
//! * one candle sent twice, once with and once without its open-interest
//!   cell;
//! * an answer across the IST year boundary, and one across 29 February;
//! * weekend and holiday candles inside the measured calendar, before it and
//!   after it;
//! * an index candle priced at zero.
//!
//! Every candle is invented at run time. A test named `finding_*` is
//! `#[ignore]`d because it FAILS on the branch it was written against; its
//! doc comment is the finding.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::fs;
use std::path::{Path, PathBuf};

use pull::fetch::{RawRow, RawWindow};
use pull::session::{Day, IST_OFFSET_SECS, Window};
use pull::vendor::Listing;
use store::file::{BarFile, StoreError};
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

// ===========================================================================
// Fixtures
// ===========================================================================

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("brutex-xz-{tag}-{}-{serial}", std::process::id()));
        let _best_effort = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch root");
        Self(dir)
    }

    fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir_all(&self.0);
    }
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// The UTC epoch second of `minute` past IST midnight on `d`.
fn stamp(d: Day, minute: i64) -> i64 {
    i64::from(d.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60
}

/// An invented index candle at `minute` past IST midnight on `d`, in paisa.
fn candle(d: Day, minute: i64) -> RawRow {
    let base = 2_000_000 + (minute % 89) * 7 + i64::from(d.day());
    RawRow {
        timestamp: stamp(d, minute),
        open: base,
        high: base + 40,
        low: base - 30,
        close: base + 5,
        volume: 0,
        open_interest: None,
    }
}

/// 09:15 IST, in minutes past midnight.
const OPEN: i64 = 555;

/// The 375 minutes of a regular session on `d`.
fn session(d: Day) -> Vec<RawRow> {
    (OPEN..OPEN + 375).map(|m| candle(d, m)).collect()
}

fn request(from: Day, to: Day) -> pull::fetch::BarRequest {
    pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: Listing::Index,
        window: Window::new(from, to).unwrap(),
        granularity: pull::vendor::Granularity::Minute1,
    }
}

fn plan(request: &pull::fetch::BarRequest) -> pull::ingest::Plan<'_> {
    pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::Gdfl,
        request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
        run: 0,
    }
}

const SYMBOL: &str = "NIFTY";

/// Lands `rows` as one Zerodha NIFTY answer over `from..=to`.
fn land(root: &Path, from: Day, to: Day, rows: Vec<RawRow>) -> pull::ingest::Ingested {
    let req = request(from, to);
    pull::ingest::from_window(&RawWindow { rows }, SYMBOL, "XZ TEST", root, plan(&req))
}

/// Every one-minute bar of `month`, or none when the month was never made.
fn minutes_of(root: &Path, month: (u16, u8)) -> Vec<Bar> {
    let path = StorePath::new(PathParts {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol: SYMBOL,
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(month.0, month.1).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    let id = u32::try_from(brutex_core::universe::fnv1a(SYMBOL) & u64::from(u32::MAX)).unwrap();
    match BarFile::open_existing(root, path, id) {
        Ok(file) => (0..file.records())
            .map(|i| file.read_record(i).unwrap())
            .collect(),
        Err(StoreError::Missing { .. }) => Vec::new(),
        Err(other) => panic!("the store refused {month:?}: {other}"),
    }
}

/// The shipped Zerodha HTTP descriptor.
fn zerodha() -> pull::vendor::HttpSpec {
    let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Zerodha.descriptor().transport
    else {
        panic!("Zerodha is an HTTP feed");
    };
    spec
}

/// A Kite historical answer holding `candles`, each already JSON.
fn kite(candles: &[String]) -> String {
    format!(
        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
        candles.join(",")
    )
}

/// One positional Kite candle with the given cells after the stamp.
fn cell_row(minute: i64, cells: &str) -> String {
    format!(
        "[\"2024-06-03T{:02}:{:02}:00+0530\",{cells}]",
        minute / 60,
        minute % 60
    )
}

fn decode(body: &str, listing: Listing) -> Result<RawWindow, pull::fetch::FetchError> {
    pull::http::decode_body(body, &zerodha(), listing)
}

// ===========================================================================
// The answer's shape
// ===========================================================================

/// An answer of no candle decodes to no row, lands no bar, and the day it
/// owed is named in the gap ledger rather than passing as a quiet day.
#[test]
fn an_empty_answer_lands_no_bar_and_names_the_day_it_owed() {
    let decoded = decode(&kite(&[]), Listing::Index).expect("an empty answer decodes");
    assert!(decoded.rows.is_empty());
    let s = Scratch::new("Empty");
    let d = day(2024, 6, 3);
    let got = land(s.root(), d, d, Vec::new());
    assert_eq!(got.bars_committed, 0);
    assert!(minutes_of(s.root(), (2024, 6)).is_empty());
    let ledger =
        fs::read_to_string(s.root().join("audit").join("zerodha.gaps")).unwrap_or_default();
    assert!(
        ledger
            .lines()
            .any(|l| l.starts_with("gap ") && l.contains("2024-06-03")),
        "the owed day is named: {ledger}"
    );
}

/// A positional row of the wrong width, or with a price that is not a JSON
/// number, refuses the whole window; a `null` price skips its row alone.
#[test]
fn a_positional_row_of_the_wrong_shape_refuses_the_window_and_a_null_price_skips_its_row() {
    let good = cell_row(OPEN, "22000.5,22010,21995,22005.25,0");
    for bad in [
        cell_row(OPEN + 1, "22000.5,22010,21995,22005.25"),
        cell_row(OPEN + 1, "22000.5,22010,21995,22005.25,0,0,7"),
        cell_row(OPEN + 1, "\"22000.5\",22010,21995,22005.25,0"),
        cell_row(OPEN + 1, "true,22010,21995,22005.25,0"),
        cell_row(OPEN + 1, "[22000.5],22010,21995,22005.25,0"),
        cell_row(OPEN + 1, "-22000.5,22010,21995,22005.25,0"),
        "\"2024-06-03T09:16:00+0530\"".to_owned(),
    ] {
        let body = kite(&[good.clone(), bad.clone()]);
        assert!(
            decode(&body, Listing::Index).is_err(),
            "the window is refused for {bad}"
        );
    }
    let null_price = cell_row(OPEN + 1, "null,22010,21995,22005.25,0");
    let body = kite(&[good, null_price]);
    let decoded = decode(&body, Listing::Index).expect("a null price is a skipped row");
    assert_eq!(
        decoded.rows.len(),
        1,
        "the null-priced row alone is skipped"
    );
}

/// FINDING (Zerodha decode, a `null` volume).
///
/// Input: a Kite positional candle for an EQUITY whose volume cell is
/// `null`: `["2024-06-03T09:15:00+0530",1500.5,1501,1499.5,1500,null]`.
///
/// Expected: refused (or at least counted and named), as the same `null`
/// is in the two other decode shapes: `decode_objects` and the parallel
/// arrays both pass the volume through `one_volume`, which refuses a
/// non-number. A share's volume is a count of shares that changed hands, and
/// `one_volume`'s own comment calls a made-up zero on an equity "a lie".
///
/// Actual: decoded as volume `0`, for every listing, with no count and no
/// event: `crates/pull/src/http.rs:3810-3811` (`serde_json::Value::Null =>
/// 0` in `decode_positional`), the shape Kite answers in.
///
/// Proposed fix: send the positional volume cell through `one_volume` like
/// the other shapes, so a `null` refuses the window on a share; if a `null`
/// must be tolerated on an index, record it as zero there only and count it
/// the way `one_volume` counts a corrected negative.
#[test]
#[ignore = "finding: a null Kite volume is stored as zero (EXTREMES-REPORT.md)"]
fn finding_a_null_volume_in_a_kite_candle_is_decoded_as_zero() {
    let body = kite(&[cell_row(OPEN, "1500.5,1501,1499.5,1500,null")]);
    let got = decode(&body, Listing::Equity);
    assert!(
        got.is_err(),
        "a null volume on a share must not become a traded zero: {:?}",
        got.map(|w| w.rows)
    );
}

// ===========================================================================
// Order and duplicates
// ===========================================================================

/// An answer whose candles are out of order lands NONE of its bars, and the
/// refusal is named: never a partial or re-sorted month.
#[test]
fn an_out_of_order_answer_lands_none_of_its_bars_and_is_named() {
    let s = Scratch::new("Unordered");
    let d = day(2024, 6, 4);
    let mut rows = session(d);
    rows.swap(100, 101);
    let got = land(s.root(), d, d, rows);
    assert_eq!(got.bars_committed, 0, "{got:?}");
    assert!(minutes_of(s.root(), (2024, 6)).is_empty());
    assert!(!got.failures.is_empty(), "the unordered answer is named");
}

/// FINDING (Zerodha door, one candle in two shapes).
///
/// Input: one Kite answer for NIFTY carrying the 09:20 candle twice with the
/// same open, high, low, close and volume: once as a six-cell row and once as
/// a seven-cell row whose open-interest cell is `0`.
///
/// Expected: one bar at 09:20. An index carries no open interest (the door
/// withholds any it is sent and files `OI_NULL`), so the two rows are one
/// candle sent twice, which the door lands once like any identical repeat.
///
/// Actual: see the assertion. The duplicate screen compares whole `RawRow`s
/// BEFORE the open interest is withheld (`crates/pull/src/ingest.rs`
/// 1489-1528, `screen_candles`, ahead of `fetch::land_rows`' withholding at
/// `crates/pull/src/fetch.rs` 1207-1211), so the two differ in a field the
/// series does not keep, and the minute is refused as conflicting.
///
/// Proposed fix: compare candles in the duplicate screen on the fields the
/// series keeps (withhold the index's or share's open interest first), so a
/// repeat that differs only in a withheld cell is an identical repeat.
#[test]
#[ignore = "finding: a candle repeated with and without its OI cell is refused (EXTREMES-REPORT.md)"]
fn finding_one_index_candle_sent_with_and_without_its_oi_cell_is_refused() {
    let six = cell_row(OPEN + 5, "22000.5,22010,21995,22005.25,0");
    let seven = cell_row(OPEN + 5, "22000.5,22010,21995,22005.25,0,0");
    let mut candles: Vec<String> = (0..5)
        .map(|m| cell_row(OPEN + m, "22000,22010,21990,22005,0"))
        .collect();
    candles.push(six);
    candles.push(seven);
    let decoded = decode(&kite(&candles), Listing::Index).expect("both rows decode");
    assert_eq!(decoded.rows.len(), 7);
    let s = Scratch::new("SixSeven");
    let d = day(2024, 6, 3);
    let rows: Vec<RawRow> = decoded
        .rows
        .iter()
        .map(|r| RawRow {
            timestamp: r.timestamp,
            ..*r
        })
        .collect();
    let got = land(s.root(), d, d, rows);
    let bars = minutes_of(s.root(), (2024, 6));
    let at_0920 = stamp(d, OPEN + 5) * 1_000_000;
    assert!(
        bars.iter().any(|b| b.ts_micros == at_0920),
        "the 09:20 candle lands once; failures {:?}",
        got.failures
    );
}

// ===========================================================================
// The year boundary and the leap day
// ===========================================================================

/// One answer across 31 December 2024 and 1 January 2025 files each session
/// whole in its own IST month.
#[test]
fn an_answer_across_the_year_boundary_files_each_session_in_its_own_month() {
    let s = Scratch::new("YearEnd");
    let (last, first) = (day(2024, 12, 31), day(2025, 1, 1));
    let mut rows = session(last);
    rows.extend(session(first));
    let got = land(s.root(), last, first, rows);
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    let december = minutes_of(s.root(), (2024, 12));
    let january = minutes_of(s.root(), (2025, 1));
    assert_eq!(december.len(), 375);
    assert_eq!(january.len(), 375);
    assert_eq!(december[0].ts_micros, stamp(last, OPEN) * 1_000_000);
    assert_eq!(january[374].ts_micros, stamp(first, OPEN + 374) * 1_000_000);
}

/// One answer across 28 February to 1 March 2024 files the leap day in
/// February.
#[test]
fn an_answer_across_the_leap_day_files_it_in_february() {
    let s = Scratch::new("Leap");
    let (from, leap, to) = (day(2024, 2, 28), day(2024, 2, 29), day(2024, 3, 1));
    let mut rows = session(from);
    rows.extend(session(leap));
    rows.extend(session(to));
    let got = land(s.root(), from, to, rows);
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    let february = minutes_of(s.root(), (2024, 2));
    assert_eq!(february.len(), 750);
    assert_eq!(february[749].ts_micros, stamp(leap, OPEN + 374) * 1_000_000);
    assert_eq!(minutes_of(s.root(), (2024, 3)).len(), 375);
}

// ===========================================================================
// Closed days, inside and outside the measured calendar
// ===========================================================================

/// What one answer of a regular session and an hour of candles on a closed
/// day leaves in the minute store: `(closed day, bars of the closed day
/// stored, bars of the open day stored, failures named)`.
fn closed_day_outcome(open: Day, closed: Day) -> (Day, usize, usize, usize) {
    let s = Scratch::new("Closed");
    let mut rows = session(open);
    rows.extend((OPEN..OPEN + 60).map(|m| candle(closed, m)));
    let got = land(s.root(), open, closed, rows);
    let mut held = minutes_of(s.root(), (open.year(), open.month()));
    if closed.month() != open.month() {
        held.extend(minutes_of(s.root(), (closed.year(), closed.month())));
    }
    let closed_from = stamp(closed, 0) * 1_000_000;
    let on_closed = held.iter().filter(|b| b.ts_micros >= closed_from).count();
    (
        closed,
        on_closed,
        held.len() - on_closed,
        got.failures.len(),
    )
}

/// Inside the measured calendar (2019-12-02 to 2026-09-04) a closed day's
/// candles are dropped and its open neighbour lands whole: the case the
/// finding below fails outside it.
#[test]
fn candles_on_a_closed_day_inside_the_measured_calendar_are_not_stored() {
    let (_, on_closed, on_open, _) = closed_day_outcome(day(2024, 5, 31), day(2024, 6, 1));
    assert_eq!((on_closed, on_open), (0, 375));
}

/// FINDING (Zerodha landing, closed days outside the measured calendar).
///
/// Input: one NIFTY answer holding a regular session and an hour of candles
/// stamped on a day the SOURCED calendar (the ingest calendar of record,
/// 2018-01-01 to 2026-12-31 from NSE's circulars) knows is closed but the
/// MEASURED calendar (2019-12-02 to 2026-09-04) does not cover: Saturday
/// 2018-03-03, Saturday 2026-09-12, and the Gandhi Jayanti holiday
/// 2026-10-02.
///
/// Expected: the closed day's candles are dropped and counted, as they are
/// inside the measured span (2024-06-01 above), and the open day lands with
/// its derived rungs.
///
/// Actual: see the assertion. `fetch::land_rows` judges the row with the
/// measured `kind_of` (through `Window::verdict`), which answers `Unmeasured`
/// there, and keeps it counted as `unclassified_kept`
/// (`crates/pull/src/fetch.rs:1071-1075`, `session::on_unclassified_day` at
/// `crates/pull/src/session.rs:1304`). The minute month is written with the
/// closed day's bars; the derived rungs, judged on the sourced calendar,
/// then call them a "store defect" and withhold every derived bucket of the
/// answer, the open day's included (2018-03-02's 2min to 60min are lost).
///
/// Proposed fix: judge a row's day at landing with the same calendar the
/// derivation uses (`sourced_kind_of`, with recorded windows), keeping
/// `unclassified_kept` for days NO calendar covers; or, at least, refuse the
/// closed day's rows by name before append.
#[test]
#[ignore = "finding: closed-day candles outside the measured calendar are stored (EXTREMES-REPORT.md)"]
fn finding_candles_on_closed_days_outside_the_measured_calendar_are_stored() {
    let outcomes: Vec<(Day, usize, usize, usize)> = [
        (day(2018, 3, 2), day(2018, 3, 3)),
        (day(2026, 9, 11), day(2026, 9, 12)),
        (day(2026, 10, 1), day(2026, 10, 2)),
    ]
    .iter()
    .map(|&(open, closed)| closed_day_outcome(open, closed))
    .collect();
    assert!(
        outcomes
            .iter()
            .all(|&(_, on_closed, on_open, _)| on_closed == 0 && on_open == 375),
        "(closed day, its bars stored, open day's bars stored, failures) = {outcomes:?}"
    );
}

/// FINDING (Zerodha door, a NIFTY candle priced at zero).
///
/// Input: a NIFTY minute answer for 2024-06-05 whose 201st candle has open,
/// high, low and close all `0`.
///
/// Expected: the zero candle is refused by name and the minute is left
/// empty: NIFTY 50 is a capitalisation-weighted index that never prints zero,
/// and a stored zero reads to every indicator as a 100% crash.
///
/// Actual: stored. The identity screen exempts EVERY index from its price
/// floor (`zero_held`, `crates/pull/src/identity_screen.rs:355-356`, "AN
/// INDEX'S ZERO IS ITS OWN (NIFTY50DIVPOINT restarts from zero, D-4992)"),
/// and the store admits a zero price (`crates/store/src/format.rs:800`), so
/// nothing between the vendor's zero and the swept series refuses it.
///
/// Proposed fix: scope the zero exemption to the indices that can be zero
/// (the dividend-point series), keyed by symbol, and refuse a zero on every
/// other index; the swept NIFTY and BANKNIFTY above all.
#[test]
#[ignore = "finding: a zero-priced NIFTY candle is stored (EXTREMES-REPORT.md)"]
fn finding_a_nifty_candle_priced_at_zero_is_stored() {
    let s = Scratch::new("ZeroIndex");
    let d = day(2024, 6, 5);
    let mut rows = session(d);
    let zero_at = rows[200].timestamp;
    rows[200] = RawRow {
        timestamp: zero_at,
        open: 0,
        high: 0,
        low: 0,
        close: 0,
        volume: 0,
        open_interest: None,
    };
    let got = land(s.root(), d, d, rows);
    let held = minutes_of(s.root(), (2024, 6));
    let zeros = held.iter().filter(|b| b.low == 0).count();
    assert_eq!(
        zeros, 0,
        "{zeros} zero-priced NIFTY bar(s) reached the store; failures {:?}",
        got.failures
    );
}
