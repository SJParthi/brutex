//! D-3134's `store.tix` line for a repaired entry is written only once the
//! repaired entry is on disk (D-3135).
//!
//! Its own test binary, because `telemetry::install` takes the process's one
//! sink: nothing else in this binary emits.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::fs;
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, TimeLookup};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// 2024-06-01 00:00 IST.
const JUNE_START: i64 = 1_717_180_200_000_000;
const DAY: i64 = 86_400_000_000;

fn path(timeframe: Timeframe) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe,
        month: YearMonth::new(2024, 6).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// A daily bar on June `day` (0-based), `hour` hours after IST midnight.
fn bar(day: i64, hour: i64) -> Bar {
    Bar {
        ts_micros: JUNE_START + day * DAY + hour * 3_600_000_000,
        open: 2_000_000,
        high: 2_000_100,
        low: 1_999_900,
        close: 2_000_050,
        volume: 10,
        open_interest: OI_NULL,
    }
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brutex-tix-log-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Flips a byte of the one daily entry (June's 30 days are bucket 0).
fn tear(root: &Path) {
    let tix = path(Timeframe::DAY_1)
        .with_file(FileKind::TimeIndex)
        .to_path_buf(root);
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(tix)
        .unwrap();
    let mut entry = [0u8; 16];
    file.read_exact_at(&mut entry, 64).unwrap();
    entry[3] ^= 0x5A;
    file.write_all_at(&entry, 64).unwrap();
}

fn repairs(log: &Path, keep: u8) -> usize {
    telemetry::tail(log, keep, &telemetry::Query::last(256))
        .records
        .iter()
        .filter(|line| {
            line.target == "store.tix"
                && line.message == "time index rebuilt from the bars"
                && line.field("scope") == Some(&telemetry::OwnedValue::Str("one torn entry".into()))
        })
        .count()
}

#[test]
fn a_repair_is_logged_only_when_the_repaired_entry_reached_disk() {
    let log = scratch("log");
    let sink = telemetry::install(&telemetry::Config::new(&log))
        .expect("nothing else in this binary installs a sink");
    let keep = sink.keep_files();

    // AN APPEND THAT RETIRES THE INDEX AFTER THE REPAIR: a second daily bar on
    // the last bar's IST day (D-2330). The torn entry is never written — the
    // `.tix` is removed — so no "repaired" line may claim it was.
    let root = scratch("retired");
    let mut writer = BarFile::open_or_create(&root, path(Timeframe::DAY_1), 7).unwrap();
    let three = [bar(2, 10), bar(3, 10), bar(4, 10)];
    assert!(matches!(
        writer.append(&three),
        Ok(Appended::Committed { .. })
    ));
    tear(&root);
    assert!(matches!(
        writer.append(&[bar(4, 14)]),
        Ok(Appended::Committed { .. })
    ));
    assert!(matches!(writer.time_lookup(), TimeLookup::Bisection(_)));
    drop(writer);
    assert_eq!(repairs(&log, keep), 0, "no entry was repaired on disk");

    // AN APPEND THAT LANDS: the repaired entry is written, and said once.
    let root = scratch("landed");
    let mut writer = BarFile::open_or_create(&root, path(Timeframe::DAY_1), 7).unwrap();
    assert!(matches!(
        writer.append(&three),
        Ok(Appended::Committed { .. })
    ));
    tear(&root);
    assert!(matches!(
        writer.append(&[bar(5, 10)]),
        Ok(Appended::Committed { .. })
    ));
    assert_eq!(writer.time_lookup(), TimeLookup::Indexed);
    drop(writer);
    assert_eq!(repairs(&log, keep), 1, "the one repair that reached disk");
}
