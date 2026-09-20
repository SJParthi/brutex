#![cfg(test)]
//! Actual window responses over finite generated records and owned failures.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::*;
use axum::http::StatusCode;
use serde_json::Value;
use std::fs;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

const QUERY: &str = "feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&from=2025-05&to=2025-07";

struct Fixture {
    root: PathBuf,
    site: Loaded,
    paths: Vec<PathBuf>,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
        let masters = root.join("masters");
        fs::create_dir_all(&masters).expect("offline empty master root");
        let site = Loaded::new(Site::load(&masters, &root));
        let mut fixture = Self {
            root,
            site,
            paths: Vec::new(),
        };
        for (month, closes, interest) in [(5, [100, 110], [OI_NULL, 0]), (6, [200, 180], [100, 50])]
        {
            let date = Day::new(2025, month, 2).expect("generated date");
            let path = StorePath::new(PathParts {
                vendor: Vendor::Dhan,
                exchange: "NSE",
                segment: "INDEX",
                symbol: "NIFTY",
                contract: None,
                timeframe: Timeframe::MINUTE_1,
                month: YearMonth::new(2025, month).expect("month"),
                file: FileKind::Bars,
            })
            .expect("generated path");
            let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
            let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
            let mut file = store::file::BarFile::open_or_create(&fixture.root, path, symbol)
                .expect("owned generated bar writer");
            let rows: Vec<_> = closes
                .iter()
                .copied()
                .enumerate()
                .map(|(index, close)| {
                    let ordinal =
                        i64::from(month - 5) * 2 + i64::try_from(index).expect("two rows") + 1;
                    Bar {
                        ts_micros: i64::from(date.days_from_epoch()) * 86_400_000_000
                            + 13_500_000_000
                            + i64::try_from(index).expect("row") * 60_000_000,
                        open: close,
                        high: close + ordinal * 10,
                        low: close - ordinal * 10,
                        close,
                        volume: ordinal * 10,
                        open_interest: interest[index],
                    }
                })
                .collect();
            file.append(&rows).expect("complete generated records");
            fixture.paths.push(path.to_path_buf(&fixture.root));
        }
        fixture
    }

    async fn get_raw(&self, query: &str) -> (StatusCode, Value) {
        let uri = format!("/bars/window.json?{query}").parse().expect("URI");
        let (status, headers, body) =
            bars_window_json(axum::extract::State(Loaded::clone(&self.site)), uri).await;
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        (status, serde_json::from_str(&body).expect("window JSON"))
    }

    async fn get(&self, options: &str) -> (StatusCode, Value) {
        self.get_raw(&format!("{QUERY}&{options}")).await
    }

    async fn get_month(&self, options: &str) -> (StatusCode, Value) {
        let uri = format!(
            "/bars.json?feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&month=2025-05&{options}"
        )
        .parse()
        .expect("month URI");
        let (status, headers, body) =
            bars_json(axum::extract::State(Loaded::clone(&self.site)), uri).await;
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        (status, serde_json::from_str(&body).expect("month JSON"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn single_month_day_bounds_refuse_typos_and_reversed_ranges_before_open() {
    let fixture = Fixture::new("month-date-refusals");
    for (options, field) in [
        ("from=not-a-day", "from"),
        ("to=not-a-day", "to"),
        ("from=2025-02-29", "from"),
        ("to=2025-04-31", "to"),
        ("from=2025-13-02", "from"),
        ("to=2025-00-02", "to"),
        ("from=2025-05-02&to=bad", "to"),
        ("from=bad&to=2025-05-02", "from"),
        ("from=2025-05-03&to=2025-05-02", "from"),
    ] {
        let (status, response) = fixture.get_month(options).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{options}: {response}");
        assert!(
            response["error"]
                .as_str()
                .expect("named refusal")
                .contains(field)
        );
        assert!(response.get("bars").is_none());
    }
    let path = &fixture.paths[0];
    let held = path.with_extension("held");
    fs::rename(path, &held).expect("withhold owned source temporarily");
    let (status, response) = fixture.get_month("from=bad").await;
    fs::rename(held, path).expect("restore exact source");
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        response["error"]
            .as_str()
            .expect("date refusal")
            .contains("from")
    );
    assert!(
        !response["error"]
            .as_str()
            .expect("date refusal")
            .contains("cannot open")
    );
}

#[tokio::test]
async fn single_month_day_windows_preserve_valid_rows_nulls_and_source_bytes() {
    let fixture = Fixture::new("month-date-windows");
    let originals: Vec<_> = fixture
        .paths
        .iter()
        .flat_map(|path| [path.clone(), path.with_extension("crc")])
        .map(|path| {
            let bytes = fs::read(&path).expect("owned source or checksum");
            (path, bytes)
        })
        .collect();
    let (status, full) = fixture.get_month("").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(full.as_array().expect("bars").len(), 2);
    assert!(full[0]["oi"].is_null());
    assert_eq!(full[1]["oi"], 0);
    assert_eq!(full[0]["o"], 100);
    assert_eq!(full[1]["c"], 110);
    for options in [
        "from=&to=",
        "from=2025-05-02&to=2025-05-02",
        "from=2025-05-01&to=2025-05-02",
        "from=2025-05-02",
        "to=2025-05-02",
    ] {
        let (status, response) = fixture.get_month(options).await;
        assert_eq!(status, StatusCode::OK, "{options}: {response}");
        assert_eq!(response, full, "{options}");
    }
    for options in [
        "from=2025-05-03",
        "to=2025-05-01",
        "from=2025-05-03&to=2025-05-04",
    ] {
        let (status, response) = fixture.get_month(options).await;
        assert_eq!(status, StatusCode::OK, "{options}: {response}");
        assert_eq!(response, serde_json::json!([]), "{options}");
    }
    for (path, bytes) in originals {
        assert_eq!(fs::read(path).expect("unchanged source"), bytes);
    }
}

#[test]
fn single_day_window_checks_both_ist_midnights_at_microsecond_precision() {
    // 2025-05-02 00:00 IST is 2025-05-01 18:30 UTC. These are literal UTC
    // instants, independent of the parser's civil-day conversion arithmetic.
    let first = 1_746_124_200_000_000;
    let end = 1_746_210_600_000_000;
    let bounds = day_window_bounds("from=2025-05-02&to=2025-05-02").expect("same day");
    assert_eq!(bounds, (Some(first), Some(end)));
    let rows: Vec<_> = [first - 1, first, end - 1, end]
        .into_iter()
        .map(|ts_micros| Bar {
            ts_micros,
            open: 100,
            high: 110,
            low: 90,
            close: 100,
            volume: 1,
            open_interest: OI_NULL,
        })
        .collect();
    let response: Value =
        serde_json::from_str(&bars_array(&rows, bounds.0, bounds.1)).expect("exact window");
    assert_eq!(response.as_array().expect("bars").len(), 2);
    assert_eq!(response[0]["t"], first / 1_000_000);
    assert_eq!(response[1]["t"], (end - 1) / 1_000_000);
    assert!(response[0]["oi"].is_null());
}

#[tokio::test]
async fn window_pages_preserve_integer_values_month_gaps_and_change_provenance() {
    let fixture = Fixture::new("window-route-pages");
    let originals: Vec<_> = fixture
        .paths
        .iter()
        .map(|path| fs::read(path).expect("source"))
        .collect();
    let (status, page) = fixture.get("dir=asc&offset=1&limit=2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["total"], 4);
    assert_eq!(page["months_read"], 2);
    assert_eq!(page["months_missing"], 1);
    assert_eq!(page["scanned"], false);
    assert!(page["faults"].is_null());
    assert!(page["extremes"].is_null());
    assert_eq!(page["bars"][0]["c"], 110);
    assert_eq!(page["bars"][0]["oi"], 0);
    assert_eq!(page["bars"][0]["chg"], 1_000);
    assert!(page["bars"][0]["chg_why"].is_null());
    assert_eq!(page["bars"][0]["oichg_why"], "oi_null_before");
    assert_eq!(page["bars"][1]["c"], 200);
    assert!(page["bars"][1]["chg"].is_null());
    assert_eq!(page["bars"][1]["chg_why"], "first_bar_in_file");
    let (_, scan) = fixture.get("sort=c&extremes=true&limit=2").await;
    assert_eq!(scan["scanned"], true);
    assert_eq!(scan["extremes"]["range"], 80);
    assert_eq!(scan["extremes"]["volume"], 40);
    assert_eq!(scan["bars"][0]["c"], 200);
    assert_eq!(scan["bars"][1]["c"], 180);
    assert_eq!(scan["bars"][1]["oichg"], -5_000);
    assert!(scan["bars"][1]["oichg_why"].is_null());
    for sort in ["ts", "o", "h", "l", "c", "v", "oi"] {
        let (status, sorted) = fixture.get(&format!("sort={sort}&limit=4")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(sorted["scanned"], sort != "ts");
        let mut closes: Vec<_> = sorted["bars"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|bar| bar["c"].as_i64().expect("integer close"))
            .collect();
        closes.sort_unstable();
        assert_eq!(closes, [100, 110, 180, 200]);
    }
    let (_, first) = fixture.get("dir=asc&limit=1").await;
    assert!(first["bars"][0]["oi"].is_null());
    assert_eq!(first["bars"][0]["oichg_why"], "oi_null");
    for options in ["offset=4", "limit=0", "sort=v&offset=99"] {
        let (status, empty) = fixture.get(options).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(empty["bars"], serde_json::json!([]));
        assert_eq!(empty["total"], 4);
    }
    let (status, contract) = fixture
        .get("contract=2025-06-26-2500000-CE&extremes=true")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(contract["total"], 0);
    assert_eq!(contract["months_missing"], 3);
    assert_eq!(
        contract["extremes"],
        serde_json::json!({"range": 0, "volume": 0})
    );
    for (path, expected) in fixture.paths.iter().zip(originals) {
        assert_eq!(fs::read(path).expect("read-only route"), expected);
    }
}

#[tokio::test]
async fn window_corrupt_months_are_faults_instead_of_silent_missing_months() {
    let fixture = Fixture::new("window-route-corruption");
    let path = &fixture.paths[1];
    let original = fs::read(path).expect("original June authority");
    fs::write(path, b"unreadable generated month").expect("truncate owned authority");
    for options in ["dir=asc", "sort=c&extremes=1"] {
        let (status, partial) = fixture.get(options).await;
        assert_eq!(status, StatusCode::PARTIAL_CONTENT, "{partial}");
        assert_eq!(partial["months_read"], 1);
        assert_eq!(partial["months_missing"], 1);
        assert_eq!(partial["total"], 2);
        assert_eq!(partial["bars"].as_array().expect("healthy rows").len(), 2);
        assert!(
            partial["faults"]
                .as_str()
                .expect("named unreadable file")
                .contains("2025-06")
        );
    }
    let (status, all_unreadable) = fixture
        .get_raw(&QUERY.replace("from=2025-05", "from=2025-06"))
        .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(all_unreadable["months_read"], 0);
    assert_eq!(all_unreadable["months_missing"], 1);
    assert_eq!(all_unreadable["bars"], serde_json::json!([]));
    assert!(all_unreadable["faults"].is_string());
    fs::write(path, &original).expect("restore exact bytes");
    let (status, recovered) = fixture.get("dir=asc").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(recovered["total"], 4);
    assert!(recovered["faults"].is_null());
    assert_eq!(fs::read(path).expect("read-only recovery"), original);
}

#[tokio::test]
async fn window_unavailable_store_roots_cannot_be_reported_as_empty_history() {
    let fixture = Fixture::new("window-route-root");
    let moved = fixture.root.with_extension("owned-detached-root");
    assert!(!moved.exists());
    fs::rename(&fixture.root, &moved).expect("detach owned generated store");
    let (status, absent) = fixture.get("").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        absent["error"]
            .as_str()
            .expect("refusal")
            .contains("store root")
    );
    fs::write(&fixture.root, b"owned obstruction").expect("store path is no longer a directory");
    let (status, blocked) = fixture.get("").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        blocked["error"]
            .as_str()
            .expect("refusal")
            .contains("not a directory")
    );
    fs::remove_file(&fixture.root).expect("remove only owned obstruction");
    fs::rename(moved, &fixture.root).expect("reattach same owned store");
    let (status, recovered) = fixture.get("").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(recovered["total"], 4);
}

#[tokio::test]
async fn window_malformed_inputs_refuse_instead_of_answering_another_page() {
    let fixture = Fixture::new("window-route-inputs");
    for (key, value) in [
        ("feed", "unknown"),
        ("from", "bad"),
        ("to", "2025-13"),
        ("from", "2025-08"),
        ("symbol", "bad-symbol"),
    ] {
        let query = QUERY
            .split('&')
            .map(|pair| {
                if pair.starts_with(&format!("{key}=")) {
                    format!("{key}={value}")
                } else {
                    pair.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("&");
        let (status, refused) = fixture.get_raw(&query).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {refused}");
        assert!(
            refused["error"]
                .as_str()
                .is_some_and(|reason| !reason.is_empty())
        );
        assert!(refused.get("bars").is_none());
    }
    for options in [
        "offset=banana",
        "limit=-1",
        "sort=unknown",
        "contract=bad",
        "timeframe=17min",
    ] {
        let (status, refused) = fixture.get(options).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{options}: {refused}");
        assert!(
            refused["error"]
                .as_str()
                .is_some_and(|reason| !reason.is_empty())
        );
    }
}
