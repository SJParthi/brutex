#![cfg(test)]
//! `/spot.json` over generated records (D-2807). No row here is market data.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::spot::{MAX_SPOT_TIMES, Spot, spot_json, underlying_segment, verdict};
use super::*;
use axum::http::StatusCode;
use serde_json::{Value, json};
use std::fs;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// 2025-05-02 09:15 IST, in UTC seconds.
fn open_second() -> i64 {
    i64::from(
        Day::new(2025, 5, 2)
            .expect("generated day")
            .days_from_epoch(),
    ) * 86_400
        + 13_500
}

struct Fixture {
    root: PathBuf,
    site: Loaded,
    index_path: PathBuf,
}

fn write(root: &Path, segment: &str, symbol: &str, bars: &[(i64, i64)]) -> PathBuf {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2025, 5).expect("month"),
        file: FileKind::Bars,
    })
    .expect("generated path");
    let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
    let id = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
    let mut file =
        store::file::BarFile::open_or_create(root, path, id).expect("owned generated writer");
    let rows: Vec<Bar> = bars
        .iter()
        .map(|&(secs, close)| Bar {
            ts_micros: secs * 1_000_000,
            open: close,
            high: close + 5,
            low: close - 5,
            close,
            volume: 0,
            open_interest: OI_NULL,
        })
        .collect();
    file.append(&rows).expect("generated records");
    path.to_path_buf(root)
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
        let masters = root.join("masters");
        fs::create_dir_all(&masters).expect("offline empty master root");
        let site = Loaded::new(Site::load(&masters, &root));
        let t0 = open_second();
        // A GAP AT +120 s, so "no bar at t" and "the bar before t" differ.
        let index_path = write(
            &root,
            "INDEX",
            "NIFTY",
            &[(t0, 2_400_000), (t0 + 60, 2_401_050), (t0 + 180, 2_399_900)],
        );
        write(&root, "CASH", "RELIANCE", &[(t0, 141_025)]);
        Self {
            root,
            site,
            index_path,
        }
    }

    async fn get(&self, query: &str) -> (StatusCode, Value) {
        let uri = format!("/spot.json?{query}").parse().expect("URI");
        let (status, headers, body) =
            spot_json(axum::extract::State(Loaded::clone(&self.site)), uri).await;
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        (status, serde_json::from_str(&body).expect("spot JSON"))
    }

    async fn at(&self, symbol: &str, times: &[i64]) -> (StatusCode, Value) {
        let t: Vec<String> = times.iter().map(i64::to_string).collect();
        self.get(&format!(
            "feed=dhan&exchange=NSE&symbol={symbol}&timeframe=1min&month=2025-05&t={}",
            t.join(",")
        ))
        .await
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn an_index_underlying_reads_index_and_every_other_underlying_reads_cash() {
    for name in brutex_core::universe::FNO_INDEX_UNDERLYINGS {
        assert_eq!(underlying_segment(name), "INDEX", "{name}");
    }
    for name in ["RELIANCE", "BAJAJ-AUTO", "NIFTYBEES"] {
        assert_eq!(underlying_segment(name), "CASH", "{name}");
    }
}

#[test]
fn an_absence_is_stated_only_when_the_records_either_side_bracket_it() {
    let t = 1_000;
    assert_eq!(verdict(None, None, t), Spot::NoBar);
    assert_eq!(verdict(Some(999), Some(1_001), t), Spot::NoBar);
    assert_eq!(verdict(None, Some(1_001), t), Spot::NoBar);
    assert_eq!(verdict(Some(1), None, t), Spot::NoBar);
    // A ZERO STAMP IS THE ZERO-FILLED EXTENT, never a real bar.
    assert_eq!(verdict(Some(0), Some(1_001), t), Spot::OrderBroken);
    assert_eq!(verdict(Some(1_000), None, t), Spot::OrderBroken);
    assert_eq!(verdict(Some(1_001), None, t), Spot::OrderBroken);
    assert_eq!(verdict(Some(999), Some(1_000), t), Spot::OrderBroken);
    assert_eq!(verdict(None, Some(0), t), Spot::OrderBroken);
    assert_eq!(Spot::OrderBroken.why(), Some("spot_order_broken"));
    assert_eq!(Spot::At(7).why(), None);
    assert_eq!(Spot::At(7).close(), Some(7));
    assert_eq!(Spot::NoBar.close(), None);
}

#[tokio::test]
async fn the_close_is_the_bar_stamped_exactly_at_t_and_never_a_nearest_one() {
    let fixture = Fixture::new("spot-exact");
    let t0 = open_second();
    let (status, body) = fixture
        .at(
            "NIFTY",
            &[t0, t0 + 60, t0 + 90, t0 + 120, t0 + 180, t0 - 60, t0 + 240],
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["segment"], "INDEX");
    assert_eq!(body["symbol"], "NIFTY");
    assert_eq!(body["exchange"], "NSE");
    assert_eq!(body["timeframe"], "1min");
    assert_eq!(body["month"], "2025-05");
    assert!(body["faults"].is_null());
    assert_eq!(
        body["spots"],
        json!([
            {"t": t0, "close": 2_400_000, "why": null},
            {"t": t0 + 60, "close": 2_401_050, "why": null},
            // BETWEEN TWO BARS and AT THE GAP: neither the earlier close nor
            // the later one, a named absence.
            {"t": t0 + 90, "close": null, "why": "no_spot_bar"},
            {"t": t0 + 120, "close": null, "why": "no_spot_bar"},
            {"t": t0 + 180, "close": 2_399_900, "why": null},
            {"t": t0 - 60, "close": null, "why": "no_spot_bar"},
            {"t": t0 + 240, "close": null, "why": "no_spot_bar"},
        ])
    );
}

#[tokio::test]
async fn a_stock_option_reads_its_cash_series_and_an_unheld_one_says_so_by_name() {
    let fixture = Fixture::new("spot-cash");
    let t0 = open_second();
    let (status, body) = fixture.at("RELIANCE", &[t0]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["segment"], "CASH");
    assert_eq!(
        body["spots"],
        json!([{"t": t0, "close": 141_025, "why": null}])
    );
    // FINNIFTY is an index underlying with no file here; TCS a share with none.
    for (symbol, segment) in [("FINNIFTY", "INDEX"), ("TCS", "CASH")] {
        let (status, body) = fixture.at(symbol, &[t0, t0 + 60]).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["segment"], segment);
        assert!(body["faults"].is_null());
        assert_eq!(
            body["spots"],
            json!([
                {"t": t0, "close": null, "why": "no_spot_file"},
                {"t": t0 + 60, "close": null, "why": "no_spot_file"},
            ])
        );
    }
}

#[tokio::test]
async fn malformed_questions_are_refused_by_name_before_any_file_is_read() {
    let fixture = Fixture::new("spot-refusals");
    let base = "feed=dhan&exchange=NSE&symbol=NIFTY&timeframe=1min&month=2025-05&t=1";
    let too_many = vec!["1"; MAX_SPOT_TIMES + 1].join(",");
    let at_cap = vec!["1"; MAX_SPOT_TIMES].join(",");
    for (query, word) in [
        (base.replace("feed=dhan", "feed=nobody"), "feed"),
        (base.replace("exchange=NSE", "exchange="), "exchange"),
        (base.replace("symbol=NIFTY", "symbol="), "symbol"),
        (base.replace("timeframe=1min", "timeframe="), "timeframe"),
        (base.replace("timeframe=1min", "timeframe=1hour"), "rung"),
        (base.replace("month=2025-05", "month=2025-13"), "month"),
        (base.replace("month=2025-05", "month=May"), "month"),
        (base.replace("&t=1", ""), "t is required"),
        (base.replace("t=1", "t=1,x"), "\"x\""),
        (base.replace("t=1", "t=9223372036855"), "UTC second"),
        (base.replace("t=1", &format!("t={too_many}")), "chunks"),
        (base.replace("symbol=NIFTY", "symbol=A%2FB"), "symbol"),
    ] {
        let (status, body) = fixture.get(&query).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {body}");
        assert!(
            body["error"].as_str().expect("named").contains(word),
            "{query}: {body}"
        );
        assert!(body.get("spots").is_none());
    }
    let (status, body) = fixture
        .get(&base.replace("t=1", &format!("t={at_cap}")))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["spots"].as_array().expect("answers").len(),
        MAX_SPOT_TIMES
    );
}

#[tokio::test]
async fn an_unreadable_file_or_record_is_partial_content_with_the_fault_named() {
    let fixture = Fixture::new("spot-unreadable");
    let t0 = open_second();
    let original = fs::read(&fixture.index_path).expect("generated file");
    // A RECORD'S OWN BYTES CHANGED: its block no longer matches its seal.
    let stamp = (t0 * 1_000_000).to_le_bytes();
    let at = original
        .windows(8)
        .rposition(|w| w == stamp)
        .expect("the first record's stamp is in the file");
    let mut damaged = original.clone();
    damaged[at] ^= 0x01;
    fs::write(&fixture.index_path, &damaged).expect("damage owned record");
    let (status, body) = fixture.at("NIFTY", &[t0]).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT, "{body}");
    assert_eq!(
        body["spots"],
        json!([{"t": t0, "close": null, "why": "spot_unreadable"}])
    );
    let fault = body["faults"].as_str().expect("named fault");
    assert!(
        fault.contains("2025-05.bin") && fault.contains(&format!("t={t0}:")),
        "{fault}"
    );
    // THE WHOLE FILE UNREADABLE: every answer says so, and the reason is named.
    fs::write(&fixture.index_path, b"not a bar file").expect("truncate owned file");
    let (status, body) = fixture.at("NIFTY", &[t0, t0 + 60]).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT, "{body}");
    assert_eq!(
        body["spots"],
        json!([
            {"t": t0, "close": null, "why": "spot_unreadable"},
            {"t": t0 + 60, "close": null, "why": "spot_unreadable"},
        ])
    );
    assert!(body["faults"].is_string());
    fs::write(&fixture.index_path, &original).expect("restore exact bytes");
    let (status, body) = fixture.at("NIFTY", &[t0]).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["spots"][0]["close"], 2_400_000);
    assert_eq!(
        fs::read(&fixture.index_path).expect("a GET writes nothing"),
        original
    );
}
