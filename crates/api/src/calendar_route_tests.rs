#![cfg(test)]
//! Generated storage observations exercise the actual calendar handler.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "finite generated fixtures and exact route assertions"
)]

use super::*;
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
        fs::create_dir_all(&masters).expect("empty offline masters");
        let site = Loaded::new(Site::load(&masters, &root));
        Self { root, site }
    }

    fn day(&self, vendor: Vendor, segment: Segment, symbol: &str, date: u8) {
        let month = YearMonth::new(2025, 5).expect("fixture month");
        let day = Day::new(2025, 5, date).expect("fixture date");
        let ts = i64::from(day.days_from_epoch()) * 86_400_000_000 + 21_600_000_000;
        let path = StorePath::new(PathParts {
            vendor,
            exchange: "NSE",
            segment: segment.as_str(),
            symbol,
            contract: None,
            timeframe: Timeframe::DAY_1,
            month,
            file: FileKind::Bars,
        })
        .expect("fixture path");
        let hash = brutex_core::universe::fnv1a(symbol).to_le_bytes();
        let symbol_id = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
        let mut file = store::file::BarFile::open_or_create(&self.root, path, symbol_id)
            .expect("generated daily file");
        file.append(&[store::format::Bar {
            ts_micros: ts,
            open: 100,
            high: 110,
            low: 90,
            close: 105,
            volume: 1,
            open_interest: i64::MIN,
        }])
        .expect("one generated observation");
        drop(file);

        let path = manifest_path(&self.root, vendor);
        let bytes = if path.exists() {
            fs::read(&path).expect("existing manifest")
        } else {
            Vec::new()
        };
        let mut manifest = Manifest::open_image(vendor, &bytes).expect("manifest");
        manifest
            .record(Entry {
                key: EntryKey {
                    contract: None,
                    exchange: Exchange::Nse,
                    segment,
                    symbol: Symbol::new(symbol).expect("symbol"),
                    timeframe: Timeframe::DAY_1,
                    month,
                },
                rows: 1,
                first_ts_micros: ts,
                last_ts_micros: ts,
            })
            .expect("one observation in the census");
        fs::create_dir_all(path.parent().expect("manifest parent")).expect("manifest directory");
        fs::write(path, manifest.image()).expect("publish complete fixture census");
    }

    async fn get(&self, query: &str) -> (axum::http::StatusCode, Value) {
        let uri = format!("/calendar.json?{query}").parse().expect("uri");
        let (status, headers, body) =
            calendar_json(axum::extract::State(Loaded::clone(&self.site)), uri).await;
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        (status, serde_json::from_str(&body).expect("calendar JSON"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn calendar_reads_current_feed_and_cash_identity_after_startup() {
    let fixture = Fixture::new("calendar-route-fresh");
    let (status, before) = fixture.get("feed=dhan&symbol=ADANIENT").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(before["sessions"], 0);
    fixture.day(Vendor::Dhan, Segment::Cash, "ADANIENT", 2);
    fixture.day(Vendor::Dhan, Segment::Index, "NIFTY", 2);
    fixture.day(Vendor::Zerodha, Segment::Index, "NIFTY", 5);

    let expected_day = Day::new(2025, 5, 2).expect("date").days_from_epoch();
    let (status, cash) = fixture.get("feed=dhan&symbol=ADANIENT").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(cash["sessions"], 1);
    assert_eq!(cash["days"][0]["day"], expected_day);
    let (status, exchange) = fixture.get("feed=dhan").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(exchange["sessions"], 1);
    assert_eq!(exchange["days"][0]["day"], expected_day);
    let mut from: Vec<_> = exchange["derivedFrom"]
        .as_array()
        .expect("sources")
        .iter()
        .map(|name| name.as_str().expect("source name"))
        .collect();
    from.sort_unstable();
    assert_eq!(from, ["ADANIENT", "NIFTY"]);
    let (_, zerodha) = fixture.get("feed=zerodha&symbol=NIFTY").await;
    assert_eq!(zerodha["sessions"], 1);
    assert_eq!(
        zerodha["days"][0]["day"],
        Day::new(2025, 5, 5).expect("date").days_from_epoch()
    );
    let (status, unknown) = fixture.get("feed=notafeed").await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert!(unknown.to_string().contains("notafeed"));
}

#[tokio::test]
async fn calendar_refuses_a_symbol_held_under_multiple_identities() {
    let fixture = Fixture::new("calendar-route-ambiguous");
    fixture.day(Vendor::Dhan, Segment::Cash, "NIFTY", 2);
    fixture.day(Vendor::Dhan, Segment::Index, "NIFTY", 5);
    let (status, body) = fixture.get("feed=dhan&symbol=NIFTY").await;
    assert_eq!(status, axum::http::StatusCode::CONFLICT);
    assert_eq!(body["status"], "refused");
    assert!(
        body["refusal"]
            .as_str()
            .expect("reason")
            .contains("ambiguous")
    );
    assert!(body.get("sessions").is_none());
}
