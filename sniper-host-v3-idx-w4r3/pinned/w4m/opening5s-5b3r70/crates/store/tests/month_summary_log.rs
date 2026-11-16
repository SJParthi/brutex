//! The `store.sum` lines reach the log through the writer's own step: a
//! month's FIRST summary is a named write, a month that turns dense again and
//! is read whole is a named write, and the per-commit extension between them
//! writes none. R3-06 repair (rule 3: every state change is logged), D-4980.
//!
//! Its own test binary, because `telemetry::install` takes the process's one
//! sink and this test counts every line written into it. Every bar is
//! invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::path::PathBuf;

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use telemetry::{Level, OwnedValue, Query};

const SYMBOL: u32 = 7;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brutex-sum-log-{tag}-{}", std::process::id()));
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
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    })
    .unwrap()
}

/// One-second bars at `open + offset` seconds for each offset, invented
/// prices nobody could take for a real print.
fn bars(month: YearMonth, offsets: impl Iterator<Item = i64>) -> Vec<Bar> {
    // 09:15 IST on the month's 4th.
    let open = month.ist_bounds_micros().0 + (3 * 86_400 + 555 * 60) * 1_000_000;
    offsets
        .map(|s| Bar {
            ts_micros: open + s * 1_000_000,
            open: 7_700_000 + s,
            high: 7_700_100 + s,
            low: 7_699_900 + s,
            close: 7_700_050 + s,
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
fn a_first_summary_and_a_month_dense_again_are_logged_and_extension_is_silent() {
    let log = scratch("log");
    let root = scratch("root");
    let sink = telemetry::install(&telemetry::Config::new(&log).with_min_level(Level::Trace))
        .expect("nothing else in this test binary installs a sink");
    let month = YearMonth::new(2033, 3).unwrap();
    let mut writer = BarFile::open_or_create(&root, path(month), SYMBOL).unwrap();

    // 1. FIRST: 1,100 consecutive seconds, past the 1,024 a summary needs.
    writer.append(&bars(month, 0..1_100)).unwrap();
    let sum = path(month)
        .with_file(FileKind::MonthSummary)
        .to_path_buf(&root);
    assert!(
        sum.is_file(),
        "the first commit past the threshold keeps a summary"
    );
    // 2. SPARSE: 300 bars ten seconds apart. Held 1,400 of 4,091 spanned
    //    seconds, under half, so the summary is extended and drops its
    //    seconds. Upkeep, silent.
    writer
        .append(&bars(month, (0..300).map(|j| 1_100 + 10 * j)))
        .unwrap();
    // 3. DENSE AGAIN: 3,000 consecutive seconds. Held 4,400 of 7,100, over
    //    half, so the month is read whole to keep its seconds: named.
    writer.append(&bars(month, 4_100..7_100)).unwrap();
    // 4. EXTENDED, still dense: silent.
    writer.append(&bars(month, 7_100..7_200)).unwrap();
    let header = writer.header();
    let bars_path = writer.path().to_path_buf();
    drop(writer);
    let kept = store::month_summary::read_beside(&bars_path, &header, month).unwrap();
    assert!(
        kept.tracks_seconds(),
        "the dense month keeps its seconds again"
    );

    let found = telemetry::tail(
        &log,
        sink.keep_files(),
        &Query::last(256).from_target("store.sum"),
    );
    assert_eq!(found.records.len(), 2, "{:?}", found.records);
    // NEWEST FIRST is how `tail` answers, so each line is found by its reason.
    let with = |reason: &str| {
        found
            .records
            .iter()
            .find(|line| field(&line.fields, "reason").starts_with(reason))
            .unwrap_or_else(|| panic!("a {reason} line: {:?}", found.records))
    };
    let (first, again) = (with("absent"), with("dense_again"));
    assert!(
        first.seq < again.seq,
        "the first write is logged before the rebuild"
    );
    for line in [first, again] {
        assert_eq!(line.level, Level::Info, "{line:?}");
        assert_eq!(line.message, "month summary written whole");
        assert!(
            field(&line.fields, "file").ends_with("2033-03.sum"),
            "{line:?}"
        );
    }
    assert!(
        field(&first.fields, "reason").starts_with("absent"),
        "{first:?}"
    );
    assert_eq!(field(&first.fields, "n_valid"), "1100");
    assert!(
        field(&again.fields, "reason").starts_with("dense_again"),
        "{again:?}"
    );
    assert_eq!(field(&again.fields, "n_valid"), "4400");

    let _ = fs::remove_dir_all(&log);
    let _ = fs::remove_dir_all(&root);
}
