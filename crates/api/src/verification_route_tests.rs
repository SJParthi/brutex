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
        self.query(&format!("feed={feed}")).await
    }

    async fn query(&self, query: &str) -> (StatusCode, Value) {
        let uri = format!("/verify.json?{query}").parse().expect("URI");
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

/// sobs-10, D-4454: EVERY SCRUB IS ONE EVENT ON THE LOG, and the event
/// carries the verdict the reply carries: Error for a refused question, Info
/// for a counter that checked out, Warn with the first finding for one that
/// disagreed. A feed that is not one is refused before any scrub and writes
/// none.
#[tokio::test]
async fn every_scrub_writes_its_verdict_to_the_log_once() {
    let _installed = crate::emitted::sink();
    let run = telemetry::reserve_run_id().expect("the shared sink reserves an id");
    let fixture = Fixture::new("scrub-route-logged");
    let truthful = entry("SOBSTEN");
    telemetry::in_run(run, async {
        assert_eq!(fixture.get("dhan").await.0, StatusCode::SERVICE_UNAVAILABLE);
        fixture.stored(truthful);
        fixture.publish(&[truthful]);
        assert_eq!(fixture.get("dhan").await.1["verified"], true);
        let mut wrong = truthful;
        wrong.rows = 2;
        fixture.publish(&[wrong]);
        assert_eq!(fixture.get("dhan").await.1["rows"], 1);
        assert_eq!(fixture.get("unknown").await.0, StatusCode::BAD_REQUEST);
    })
    .await;
    let scrubs: Vec<telemetry::Record> = crate::emitted::run_story(run)
        .into_iter()
        .filter(|record| record.target == "api.verify")
        .collect();
    assert_eq!(
        scrubs.len(),
        3,
        "one event per scrub and none for the refused feed: {scrubs:?}"
    );
    let (refused, clean, disagreed) = (&scrubs[0], &scrubs[1], &scrubs[2]);
    for scrub in [refused, clean, disagreed] {
        assert_eq!(scrub.message, "scrub");
        assert!(crate::emitted::says(scrub, "feed", "dhan"), "{scrub:?}");
    }
    assert_eq!(refused.level, telemetry::Level::Error);
    assert!(
        crate::emitted::says(refused, "say", "not verified"),
        "{refused:?}"
    );
    assert!(crate::emitted::counts(refused, "seen", 0));

    assert_eq!(clean.level, telemetry::Level::Info);
    assert_eq!(
        clean
            .field("verified")
            .and_then(telemetry::OwnedValue::as_bool),
        Some(true)
    );
    assert!(crate::emitted::counts(clean, "seen", 1));
    assert!(crate::emitted::counts(clean, "agreed", 1));

    assert_eq!(disagreed.level, telemetry::Level::Warn);
    assert_eq!(
        disagreed
            .field("verified")
            .and_then(telemetry::OwnedValue::as_bool),
        Some(false)
    );
    assert!(crate::emitted::counts(disagreed, "rows", 1));
    assert!(crate::emitted::counts(disagreed, "agreed", 0));
    assert!(
        crate::emitted::says(disagreed, "first", "SOBSTEN"),
        "the first finding names the entry: {disagreed:?}"
    );
}

/// **One answer opens at most a page of files, says where the next page
/// starts, and is never `verified` unless it covered every held entry.**
/// W1-api5-7, W1-api6-0, D-4435.
#[tokio::test]
async fn scrub_route_checks_one_page_and_names_the_rest() {
    let fixture = Fixture::new("scrub-route-paged");
    let entries: Vec<_> = (0..5).map(|index| entry(&format!("PAGE{index}"))).collect();
    for one in &entries {
        fixture.stored(*one);
    }
    fixture.publish(&entries);

    let (status, whole) = fixture.get("dhan").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(whole["verified"], true, "{whole}");
    assert_eq!(whole["held"], 5);
    assert_eq!(whole["offset"], 0);
    assert_eq!(whole["limit"], crate::verify::MAX_VERIFY_PAGE);
    assert!(whole["next_offset"].is_null());
    assert_eq!(whole["seen"], 5);

    let (status, first) = fixture.query("feed=dhan&limit=2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["seen"], 2, "a page opens only its own entries");
    assert_eq!(first["agreed"], 2);
    assert_eq!(first["held"], 5);
    assert_eq!(first["next_offset"], 2);
    assert_eq!(first["verified"], false, "a clean page is not the store");
    let said = first["say"].as_str().expect("a sentence");
    assert!(said.starts_with("not verified — 2 entry(s)"), "{said}");
    assert!(said.contains("entries 1 to 2 of the 5 held"), "{said}");
    assert!(said.contains("next page starts at offset=2"), "{said}");

    let (status, last) = fixture.query("feed=dhan&offset=4&limit=2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(last["seen"], 1);
    assert!(last["next_offset"].is_null());
    assert_eq!(
        last["verified"], false,
        "the last page alone is not the store"
    );
    assert!(
        last["say"]
            .as_str()
            .expect("a sentence")
            .contains("this is the last page")
    );

    // A page walk sees each held entry exactly once.
    let mut seen = 0;
    let mut at = Some(0_u64);
    while let Some(offset) = at {
        let (status, page) = fixture
            .query(&format!("feed=dhan&offset={offset}&limit=2"))
            .await;
        assert_eq!(status, StatusCode::OK);
        seen += page["seen"].as_u64().expect("a count");
        at = page["next_offset"].as_u64();
    }
    assert_eq!(seen, 5);

    for refused in [
        "feed=dhan&offset=5",
        "feed=dhan&offset=99",
        "feed=dhan&limit=0",
        "feed=dhan&limit=1025",
        "feed=dhan&limit=many",
        "feed=dhan&offset=-1",
    ] {
        let (status, body) = fixture.query(refused).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}: {body}");
        assert!(body["refused"].is_string(), "{refused} names why: {body}");
        assert_eq!(body["feed"], "dhan");
    }
    assert_eq!(
        fixture.query("feed=dhan&limit=1024").await.1["verified"],
        true,
        "the ceiling itself is a page"
    );

    // A new census snapshot is a new list: the sixth entry is paged at once.
    let mut more = entries.clone();
    more.push(entry("PAGE5"));
    fixture.publish(&more);
    let (status, grown) = fixture.query("feed=dhan").await;
    assert_eq!(status, StatusCode::OK, "{grown}");
    assert_eq!(grown["held"], 6);
    assert_eq!(grown["seen"], 6);
    assert_eq!(grown["missing"], 1, "the new month has no file");

    // An empty counter is one empty page, not a refusal and not a pass.
    fixture.publish(&[]);
    let (status, empty) = fixture.get("dhan").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty["held"], 0);
    assert_eq!(empty["verified"], false);
}

/// **However many entries the counter holds, one answer opens at most
/// `MAX_VERIFY_PAGE` files.** W1-api5-7, W1-api6-0, D-4435.
#[tokio::test]
async fn scrub_route_opens_no_more_than_a_page_of_a_larger_counter() {
    let fixture = Fixture::new("scrub-route-capped");
    let entries: Vec<_> = (0..1_100)
        .map(|index| entry(&format!("CAP{index}")))
        .collect();
    fixture.publish(&entries);
    let (status, report) = fixture.get("dhan").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["held"], 1_100);
    assert_eq!(report["seen"], crate::verify::MAX_VERIFY_PAGE);
    assert_eq!(report["missing"], crate::verify::MAX_VERIFY_PAGE);
    assert_eq!(report["next_offset"], crate::verify::MAX_VERIFY_PAGE);
    let (_, rest) = fixture.query("feed=dhan&offset=1024").await;
    assert_eq!(rest["seen"], 76);
    assert!(rest["next_offset"].is_null());
}

/// What one `/verify.json` page costs at the ceiling, every file present and
/// agreeing, and what the once-per-snapshot list costs at 10^5 log entries.
/// A measurement, run on purpose; the numbers are in `docs/06-limits.md`'s
/// D-4435 section. W1-api5-7, W1-api6-0.
#[test]
#[ignore = "a latency measurement, run on purpose: see crate::latency"]
fn latency_scrub_page_at_the_ceiling() -> Result<(), String> {
    let fixture = Fixture::new("scrub-route-latency");
    let entries: Vec<_> = (0..crate::verify::MAX_VERIFY_PAGE)
        .map(|index| entry(&format!("LAT{index}")))
        .collect();
    for one in &entries {
        fixture.stored(*one);
    }
    fixture.publish(&entries);
    let page = crate::latency::Timed::run(200, || {
        let (code, body) = verify_reading(&fixture.site, Vendor::Dhan, "dhan", 0, 1_024);
        if code == StatusCode::OK && body.contains("\"verified\":true") {
            Ok(())
        } else {
            Err(body)
        }
    })?;
    println!(
        "{}",
        page.line("/verify.json page of 1024 held files, all agreeing")
    );

    let mut manifest = Manifest::open_image(Vendor::Dhan, &[]).map_err(|why| why.to_string())?;
    for index in 0..100_000 {
        manifest
            .record(entry(&format!("LOG{index}")))
            .map_err(|why| why.to_string())?;
    }
    let walk = crate::latency::Timed::run(50, || {
        if manifest.newest().len() == 100_000 {
            Ok(())
        } else {
            Err("every key once".to_owned())
        }
    })?;
    println!(
        "{}",
        walk.line("Manifest::newest over 10^5 log entries (memo miss)")
    );
    Ok(())
}
