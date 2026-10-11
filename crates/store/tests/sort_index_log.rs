//! The two `store.srt` lines reach the log through the production calls that
//! own them, and the writer's ordinary upkeep writes none. D-3349, WP-39.
//!
//! Its own test binary, because `telemetry::install` takes the process's one
//! sink and this test counts every line written into it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::path::PathBuf;

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, Resorted};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use telemetry::{Level, OwnedValue, Query};

const SYMBOL: u32 = 5;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brutex-srt-log-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn path(month: YearMonth) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    })
    .unwrap()
}

/// One invented session of one-minute bars from 09:15 IST on the 4th.
fn session(month: YearMonth) -> Vec<Bar> {
    let open = month.ist_bounds_micros().0 + (3 * 86_400 + 555 * 60) * 1_000_000;
    (0..375)
        .map(|k| Bar {
            ts_micros: open + k * 60_000_000,
            open: 6_300_000 + k,
            high: 6_300_100 + k,
            low: 6_299_900 + k,
            close: 6_300_050 + k,
            volume: 0,
            open_interest: OI_NULL,
        })
        .collect()
}

fn field(fields: &[(String, OwnedValue)], key: &str) -> String {
    fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| match value {
            OwnedValue::Str(text) => text.clone(),
            OwnedValue::Int(n) => n.to_string(),
            OwnedValue::Uint(n) => n.to_string(),
            other => format!("{other:?}"),
        })
        .unwrap_or_default()
}

#[test]
fn a_lagging_index_warns_a_rebuild_informs_and_upkeep_is_silent() {
    let log = scratch("log");
    let root = scratch("root");
    let sink = telemetry::install(&telemetry::Config::new(&log).with_min_level(Level::Trace))
        .expect("nothing else in this test binary installs a sink");

    // UPKEEP: two months written, appended to, reopened. Silent.
    let quiet = YearMonth::new(2032, 7).unwrap();
    for _ in 0..2 {
        let mut writer = BarFile::open_or_create(&root, path(quiet), SYMBOL).unwrap();
        let _ = writer.append(&session(quiet));
    }
    // A LAG: the `.srt` name is held by a directory, so no run can land.
    let lagging = YearMonth::new(2032, 8).unwrap();
    let srt = path(lagging)
        .with_file(FileKind::SortIndex)
        .to_path_buf(&root);
    fs::create_dir_all(&srt).unwrap();
    let mut writer = BarFile::open_or_create(&root, path(lagging), SYMBOL).unwrap();
    assert!(matches!(
        writer.append(&session(lagging)),
        Ok(Appended::Committed { .. })
    ));
    drop(writer);
    // A REBUILD through the door, named.
    fs::remove_file(
        path(quiet)
            .with_file(FileKind::SortIndex)
            .to_path_buf(&root),
    )
    .unwrap();
    let rebuilt = BarFile::sort_index_existing(&root, path(quiet), SYMBOL).unwrap();
    assert!(matches!(rebuilt.outcome, Resorted::Built(_)));

    let found = telemetry::tail(
        &log,
        sink.keep_files(),
        &Query::last(256).from_target("store.srt"),
    );
    assert_eq!(found.records.len(), 2, "{:?}", found.records);
    let warn = found
        .records
        .iter()
        .find(|line| line.level == Level::Warn)
        .expect("the lag is a warning");
    assert_eq!(warn.message, "sort index lags the bars");
    assert!(
        field(&warn.fields, "file").ends_with("2032-08.srt"),
        "{warn:?}"
    );
    assert!(
        field(&warn.fields, "reason").contains("not a regular file"),
        "{warn:?}"
    );
    assert_eq!(field(&warn.fields, "n_valid"), "375");
    let info = found
        .records
        .iter()
        .find(|line| line.level == Level::Info)
        .expect("the rebuild is an info line");
    assert_eq!(info.message, "sort index written whole");
    assert!(
        field(&info.fields, "file").ends_with("2032-07.srt"),
        "{info:?}"
    );
    assert!(
        field(&info.fields, "reason").contains("no .srt"),
        "{info:?}"
    );

    let _ = fs::remove_dir_all(&log);
    let _ = fs::remove_dir_all(&root);
}
