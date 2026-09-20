#![cfg(test)]
//! Scrub responses over generated counters and their actual stored files.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "finite owned fixtures and exact JSON assertions"
)]

use super::*;
use axum::http::StatusCode;
use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
use serde_json::Value;
use std::fs;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

struct Fixture {
    root: PathBuf,
    site: Loaded,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
        let masters = root.join("masters");
        fs::create_dir_all(&masters).expect("owned offline masters");
        let site = Loaded::new(Site::load(&masters, &root));
        Self { root, site }
    }

    fn publish(&self, entries: &[Entry]) -> Vec<u8> {
        let mut manifest = Manifest::open_image(Vendor::Dhan, &[]).expect("empty counter");
        for entry in entries {
            manifest.record(*entry).expect("generated census entry");
        }
        let path = manifest_path(&self.root, Vendor::Dhan);
        fs::create_dir_all(path.parent().expect("counter parent")).expect("owned vendor root");
        let image = manifest.image().clone();
        fs::write(path, &image).expect("publish complete generated counter");
        image
    }

    fn stored(&self, entry: Entry) -> PathBuf {
        let path = StorePath::new(PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment: "INDEX",
            symbol: entry.key.symbol.as_str(),
            contract: None,
            timeframe: entry.key.timeframe,
            month: entry.key.month,
            file: FileKind::Bars,
        })
        .expect("fixture address");
        let hash = brutex_core::universe::fnv1a(entry.key.symbol.as_str()).to_le_bytes();
        let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
        let mut file = store::file::BarFile::open_or_create(&self.root, path, symbol)
            .expect("owned bar writer");
        file.append(&[store::format::Bar {
            ts_micros: entry.first_ts_micros,
            open: 100,
            high: 110,
            low: 90,
            close: 105,
            volume: 1,
            open_interest: store::format::OI_NULL,
        }])
        .expect("generated bar");
        path.to_path_buf(&self.root)
    }

    async fn get(&self, feed: &str) -> (StatusCode, Value) {
        let uri = format!("/verify.json?feed={feed}").parse().expect("URI");
        let (status, headers, body) =
            verify_json(axum::extract::State(Loaded::clone(&self.site)), uri).await;
        assert_eq!(
            headers[axum::http::header::CONTENT_TYPE],
            "application/json; charset=utf-8"
        );
        (
            status,
            serde_json::from_str(&body).expect("verification JSON"),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn entry(symbol: &str) -> Entry {
    let date = Day::new(2025, 5, 2).expect("fixture date");
    let ts = i64::from(date.days_from_epoch()) * 86_400_000_000 + 21_600_000_000;
    Entry {
        key: EntryKey {
            contract: None,
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: Symbol::new(symbol).expect("generated symbol"),
            timeframe: Timeframe::DAY_1,
            month: YearMonth::new(2025, 5).expect("month"),
        },
        rows: 1,
        first_ts_micros: ts,
        last_ts_micros: ts,
    }
}

#[tokio::test]
async fn scrub_route_uses_current_files_and_never_repairs_disagreements() {
    let fixture = Fixture::new("scrub-route-current");
    let (status, absent) = fixture.get("dhan").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(absent["verified"], false);
    assert_eq!(fixture.get("unknown").await.0, StatusCode::BAD_REQUEST);
    let (status, default_feed) = fixture.get("").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(default_feed["feed"], "dhan");
    let truthful = entry("NIFTY");
    let path = fixture.stored(truthful);
    let original = fs::read(&path).expect("original bar bytes");
    fixture.publish(&[truthful]);
    let (status, healthy) = fixture.get("dhan").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(healthy["feed"], "dhan");
    assert_eq!(healthy["verified"], true);
    assert_eq!(healthy["seen"], 1);
    assert_eq!(healthy["agreed"], 1);
    assert_eq!(healthy["findings"], serde_json::json!([]));
    assert!(healthy["refused"].is_null());
    assert_eq!(fixture.get("").await.1, healthy);
    assert_eq!(
        fixture.get("zerodha").await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );

    let held = path.with_extension("held");
    fs::rename(&path, &held).expect("hide only the owned fixture");
    let (status, missing) = fixture.get("dhan").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(missing["missing"], 1);
    assert_eq!(missing["verified"], false);
    assert!(!path.exists(), "scrub cannot refill a missing file");
    fs::rename(held, &path).expect("restore owned bar file");

    fs::write(&path, b"unreadable generated bar").expect("owned corruption");
    let (status, unreadable) = fixture.get("dhan").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(unreadable["unreadable"], 1);
    assert_eq!(unreadable["missing"], 0);
    assert_eq!(unreadable["verified"], false);
    assert_eq!(
        fs::read(&path).expect("unchanged corruption"),
        b"unreadable generated bar"
    );
    fs::write(&path, &original).expect("restore exact bar bytes");

    let mut wrong_count = truthful;
    wrong_count.rows = 2;
    let counter = fixture.publish(&[wrong_count]);
    let (status, rows) = fixture.get("dhan").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rows["rows"], 1);
    assert_eq!(rows["verified"], false);
    assert_eq!(
        fs::read(manifest_path(&fixture.root, Vendor::Dhan)).expect("unrepaired census"),
        counter
    );

    let mut wrong_bounds = truthful;
    wrong_bounds.first_ts_micros += 60_000_000;
    wrong_bounds.last_ts_micros = wrong_bounds.first_ts_micros;
    fixture.publish(&[wrong_bounds]);
    let (status, bounds) = fixture.get("dhan").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bounds["bounds"], 1);
    assert_eq!(bounds["verified"], false);
    fixture.publish(&[truthful]);
    assert_eq!(fixture.get("dhan").await.1["verified"], true);
    assert_eq!(fs::read(path).expect("read-only bar file"), original);
}

#[tokio::test]
async fn scrub_route_bounds_named_findings_without_truncating_the_failure_count() {
    let fixture = Fixture::new("scrub-route-bounded-findings");
    let entries: Vec<_> = (0..53)
        .map(|index| entry(&format!("CHECK{index}")))
        .collect();
    let original = fixture.publish(&entries);
    let (status, report) = fixture.get("dhan").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["verified"], false);
    assert_eq!(report["seen"], 53);
    assert_eq!(report["missing"], 53);
    assert_eq!(report["agreed"], 0);
    assert_eq!(report["undrawn"], 3);
    assert_eq!(
        report["findings"].as_array().expect("named findings").len(),
        50
    );
    assert_eq!(fixture.get("dhan").await.1, report);
    assert_eq!(
        fs::read(manifest_path(&fixture.root, Vendor::Dhan)).expect("read-only counter"),
        original
    );
}

#[tokio::test]
async fn scrub_route_refuses_a_corrupted_counter_instead_of_claiming_an_empty_success() {
    let fixture = Fixture::new("scrub-route-corrupt-counter");
    let truthful = entry("NIFTY");
    fixture.stored(truthful);
    let original = fixture.publish(&[truthful]);
    assert_eq!(fixture.get("dhan").await.1["verified"], true);
    let path = manifest_path(&fixture.root, Vendor::Dhan);
    fs::write(&path, b"not a census").expect("owned counter corruption");
    let (status, report) = fixture.get("dhan").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(report["verified"], false);
    assert_eq!(report["seen"], 0);
    assert!(
        report["refused"]
            .as_str()
            .expect("named refusal")
            .contains("could not be read")
    );
    assert_eq!(fs::read(&path).expect("unchanged counter"), b"not a census");
    fs::write(path, original).expect("restore exact counter");
    assert_eq!(fixture.get("dhan").await.1["verified"], true);
}
