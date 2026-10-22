#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]
//! The quarantine's routes and its readers by name: `/bars.json` answers a
//! month quarantined whole by name, `/bars/window.json` names it and serves
//! the rest, `/instruments.json` flags the series, and the plan and the press
//! run the one verb. R4-18. Every price is invented.

use std::fs;
use std::path::{Path, PathBuf};

use axum::http::StatusCode;
use brutex_core::vendor::Vendor;
use serde_json::Value;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::quarantine::{Cause, Record, Scope};

use super::super::{Loaded, Site};
use super::{arguments, quarantine_apply, quarantine_plan_json};

const SYMBOL: &str = "QFIXQ";

struct Fixture {
    root: PathBuf,
    site: Loaded,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
        let masters = root.join("masters");
        fs::create_dir_all(&masters).expect("offline empty master root");
        let site = Loaded::new(Site::load(&masters, &root));
        Self { root, site }
    }

    fn month(&self, month: YearMonth, level: i64, n: i64) -> Vec<Bar> {
        let rows = rows(month, level, n);
        let mut file = store::file::BarFile::open_or_create(
            &self.root,
            path(month),
            pull::ingest::repair_plan::symbol_id(SYMBOL),
        )
        .expect("a month");
        file.append(&rows).expect("rows");
        rows
    }

    fn quarantine_whole(&self, month: YearMonth, bars: u64) {
        store::quarantine::record(
            &self.root,
            &Record {
                path: path(month),
                scope: Scope::WholeMonth,
                cause: Cause::BeforeListing,
                stamps: &[],
                bars,
                revision: 0,
                source_digest: [7; 32],
                who: "test",
                reason: "invented: before listing",
            },
        )
        .expect("recorded");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _cleanup = fs::remove_dir_all(&self.root);
    }
}

fn path(month: YearMonth) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol: SYMBOL,
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    })
    .expect("a path")
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).expect("a month")
}

/// `n` minutes from 09:15 IST on the 5th of `month`.
fn rows(month: YearMonth, level: i64, n: i64) -> Vec<Bar> {
    let day = pull::session::Day::new(month.year(), month.month(), 5).expect("a day");
    let open = i64::from(day.days_from_epoch()) * 86_400_000_000 + 13_500_000_000;
    (0..n)
        .map(|i| Bar {
            ts_micros: open + i * 60_000_000,
            open: level,
            high: level + 4,
            low: level - 4,
            close: level + 1,
            volume: 10 + i,
            open_interest: OI_NULL,
        })
        .collect()
}

async fn get(site: &Loaded, uri: &str) -> (StatusCode, Value) {
    for _ in 0..400 {
        let parsed: axum::http::Uri = uri.parse().expect("URI");
        let (status, _, body) = if uri.starts_with("/bars/window.json") {
            super::super::bars_route::bars_window_json(axum::extract::State(Loaded::clone(site)), parsed).await
        } else if uri.starts_with("/bars.json") {
            super::super::bars_route::bars_json(axum::extract::State(Loaded::clone(site)), parsed).await
        } else {
            quarantine_plan_json(axum::extract::State(Loaded::clone(site)), parsed).await
        };
        if status != StatusCode::TOO_MANY_REQUESTS {
            return (status, serde_json::from_str(&body).expect("JSON"));
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("the pool stayed full for two seconds");
}

fn files_under(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                out.extend(files_under(&path));
            } else {
                out.push((path.clone(), fs::read(&path).expect("bytes")));
            }
        }
    }
    out.sort();
    out
}

#[tokio::test]
async fn bars_json_answers_a_quarantined_month_by_name() {
    let fixture = Fixture::new("r418-bars-by-name");
    fixture.month(ym(2018, 3), 99_500, 3);
    let uri = format!("/bars.json?feed=zerodha&exchange=NSE&segment=CASH&symbol={SYMBOL}&month=2018-03");
    let (status, before) = get(&fixture.site, &uri).await;
    assert_eq!(status, StatusCode::OK, "{before}");
    assert_eq!(before.as_array().map(Vec::len), Some(3));
    fixture.quarantine_whole(ym(2018, 3), 3);
    let (status, body) = get(&fixture.site, &uri).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "month_quarantined");
    assert_eq!(body["quarantine"]["scope"], "whole_month");
    assert_eq!(body["quarantine"]["cause"], "before_listing");
    assert_eq!(body["quarantine"]["bars"], 3);
    assert_eq!(body["quarantine"]["ledger"], "audit/zerodha.quarantine.ledger");
    assert!(body["quarantine"]["ledger_at"].as_u64().is_some_and(|at| at > 0));
    assert!(body.get("bars").is_none(), "never its bars: {body}");
    // A torn index answers by name, never the bars.
    let index = store::quarantine::index_path(&fixture.root, Vendor::Zerodha);
    let bytes = fs::read(&index).expect("index");
    fs::write(&index, &bytes[..bytes.len() / 2]).expect("torn");
    let (status, torn) = get(&fixture.site, &uri).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{torn}");
    assert_eq!(torn["code"], "quarantine_index_torn");
}

#[tokio::test]
async fn a_window_names_its_quarantined_month_and_serves_the_rest() {
    let fixture = Fixture::new("r418-window");
    let february = fixture.month(ym(2018, 2), 410, 2);
    fixture.month(ym(2018, 3), 99_500, 3);
    fixture.month(ym(2018, 4), 420, 2);
    fixture.quarantine_whole(ym(2018, 3), 3);
    let uri = format!(
        "/bars/window.json?feed=zerodha&exchange=NSE&segment=CASH&symbol={SYMBOL}&from=2018-02&to=2018-04&limit=50"
    );
    let (status, body) = get(&fixture.site, &uri).await;
    assert!(status.is_success(), "{status} {body}");
    let text = body.to_string();
    assert!(text.contains("2018-03: quarantined, not served"), "{text}");
    assert!(!text.contains("99500"), "no quarantined bar is served: {text}");
    assert!(text.contains(&february[0].ts_micros.to_string()) || text.contains("410"), "{text}");
}

#[test]
fn instruments_json_flags_a_quarantined_series() {
    let fixture = Fixture::new("r418-instruments");
    fixture.month(ym(2018, 3), 99_500, 3);
    fixture.quarantine_whole(ym(2018, 3), 3);
    let key = brutex_core::instrument::InstrumentKey::cash(brutex_core::instrument::Exchange::Nse, SYMBOL)
        .expect("a share");
    let summary = store::quarantine::Summary::load(&fixture.root, Vendor::Zerodha);
    let flag = super::super::instruments_route::quarantine_of(&summary, &key);
    let value: Value = serde_json::from_str(&flag).expect("JSON");
    assert_eq!(value["months"], 1);
    assert_eq!(value["whole_months"], 1);
    assert_eq!(value["bars"], 3);
    assert_eq!(value["causes"], serde_json::json!(["before_listing"]));
    let other = brutex_core::instrument::InstrumentKey::cash(brutex_core::instrument::Exchange::Nse, "QFIXR")
        .expect("a share");
    assert_eq!(super::super::instruments_route::quarantine_of(&summary, &other), "null");
    let unreadable = Err(store::quarantine::QuarantineError::Reason("x".to_owned()));
    assert_eq!(
        super::super::instruments_route::quarantine_of(&unreadable, &key),
        r#"{"unknown":"quarantine_reason"}"#
    );
}

#[test]
fn the_question_is_refused_by_name_before_the_verb_runs() {
    assert!(arguments("symbol=IRFC&from=2018-01-01&to=2020-12-31&cause=before_listing", false).is_ok());
    for bad in [
        "",
        "symbol=IRFC&from=2018-01-01&to=2020-12-31",
        "symbol=IRFC&from=2018-01-01&cause=before_listing",
        "symbol=IRFC&from=2018-01-01&to=2020-12-31&cause=before_listing&bogus=1",
        "symbol=IRFC&from=2018-01-01&to=2020-12-31&cause=before_listing&reason=a%09b",
    ] {
        assert!(arguments(bad, false).is_err(), "{bad:?}");
    }
    let long = format!("symbol=IRFC&from=2018-01-01&to=2020-12-31&cause=before_listing&reason={}", "x".repeat(201));
    assert!(arguments(&long, true).is_err(), "a reason past 200 bytes");
    assert!(
        arguments("symbol=IRFC&from=2018-01-01&to=2020-12-31&cause=before_listing", true).is_err(),
        "a press names its reason"
    );
    let args = arguments("symbol=IRFC&from=2018-01-01&to=2020-12-31&cause=zero_price&reason=r&segment=CASH", true)
        .expect("ok");
    assert_eq!(args.last().map(String::as_str), Some("--apply"));
}

fn plan(root: &Path, stamps: &[i64]) {
    let dir = root.join("audit").join("series-identity");
    fs::create_dir_all(&dir).expect("dir");
    let listed = stamps.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
    fs::write(
        dir.join(format!("zerodha-CASH-{SYMBOL}-00aa00aa00aa00aa.plan")),
        format!(
            "brutex series-identity quarantine plan v2\nkey\trung\tmonth\tscope\tcause\taction\tbars\ttimestamps_utc_micros\nNSE-CASH-{SYMBOL}\t1min\t2018-03\ttimestamps\tforeign_level\tquarantine\t{}\t{listed}\n",
            stamps.len()
        ),
    )
    .expect("plan");
}

#[tokio::test]
async fn the_plan_writes_nothing_and_the_press_applies_once() {
    let fixture = Fixture::new("r418-press");
    let held = fixture.month(ym(2018, 3), 520, 5);
    plan(&fixture.root, &[held[1].ts_micros]);
    let before = files_under(&fixture.root);
    let (status, body) = get(
        &fixture.site,
        &format!("/pull/quarantine.json?symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["applied"], false);
    assert!(body["report"].as_str().unwrap().contains("plan: 1 bar(s)"), "{body}");
    assert_eq!(files_under(&fixture.root), before, "the plan wrote nothing");
    let form = format!("symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level&reason=invented");
    let (status, _, text) = quarantine_apply(axum::extract::State(Loaded::clone(&fixture.site)), form.clone()).await;
    assert_eq!(status, StatusCode::OK, "{text}");
    let body: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(body["applied"], true);
    assert!(body["report"].as_str().unwrap().contains("DROPPED 1 of 5"), "{body}");
    let (_, _, again) = quarantine_apply(axum::extract::State(Loaded::clone(&fixture.site)), form).await;
    assert!(again.contains("already_dropped"), "{again}");
    // A SECOND PRESS WHILE ONE RUNS is refused by name, never queued.
    super::PRESSING.store(true, std::sync::atomic::Ordering::Release);
    let (status, _, busy) = quarantine_apply(
        axum::extract::State(Loaded::clone(&fixture.site)),
        format!("symbol={SYMBOL}&from=2018-03-01&to=2018-03-31&cause=foreign_level&reason=r"),
    )
    .await;
    super::PRESSING.store(false, std::sync::atomic::Ordering::Release);
    assert_eq!(status, StatusCode::CONFLICT, "{busy}");
    assert!(busy.contains("quarantine_running"));
}
