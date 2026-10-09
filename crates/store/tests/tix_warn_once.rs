//! A month whose time lookup bisects is named in the log ONCE per kind of
//! reason in a process, however many handles ask. D-3340, WP-15; DB-064,
//! DB-G81, DB-G88.
//!
//! Before D-3340 every read handle that decided "bisect" wrote its own
//! `store.tix` warning, and every HTTP request opens its own handle: the
//! shadow of the live store logged one warning per `/spot.json` chunk, 423 of
//! them for 414 requests, and the live 64 MiB window held 101 at once.
//!
//! Its own test binary, because `telemetry::install` takes the process's one
//! sink and this test must count every line written into it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, Reindexed, TimeLookup};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::time_index::Why;
use telemetry::{Level, OwnedValue, Query};

const SYMBOL: u32 = 7;
const MICROS: i64 = 1_000_000;

fn path(month: YearMonth) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "BANKNIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    })
    .unwrap()
}

/// One session of invented one-minute bars from 09:15 IST on the 4th.
fn session(month: YearMonth) -> Vec<Bar> {
    let open = month.ist_bounds_micros().0 + (3 * 86_400 + 555 * 60) * MICROS;
    (0..375)
        .map(|k| Bar {
            ts_micros: open + k * 60 * MICROS,
            open: 5_100_000,
            high: 5_100_100,
            low: 5_099_900,
            close: 5_100_050,
            volume: 0,
            open_interest: OI_NULL,
        })
        .collect()
}

fn text(value: &OwnedValue) -> String {
    match value {
        OwnedValue::Str(text) => text.clone(),
        other => format!("{other:?}"),
    }
}

fn field(fields: &[(String, OwnedValue)], key: &str) -> String {
    fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| text(value))
        .unwrap_or_default()
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brutex-tix-once-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_in_place(path: &Path, at: u64, bytes: &[u8]) {
    use std::os::unix::fs::FileExt as _;
    let file = fs::OpenOptions::new().write(true).open(path).unwrap();
    file.write_all_at(bytes, at).unwrap();
}

/// Every `store.tix` line the sink holds: the warnings counted by file name
/// and reason, and the number of rebuild lines. Any other level is a failure.
fn read_back(log: &Path, keep_files: u8) -> (BTreeMap<(String, String), usize>, usize) {
    let found = telemetry::tail(log, keep_files, &Query::last(256).from_target("store.tix"));
    let mut warned: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut rebuilt = 0;
    for line in &found.records {
        match line.level {
            Level::Warn => {
                assert_eq!(line.message, "time lookup falls back to bisection");
                let file = field(&line.fields, "file");
                let name = Path::new(&file)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or(file);
                *warned
                    .entry((name, field(&line.fields, "reason")))
                    .or_default() += 1;
                assert!(
                    field(&line.fields, "repeats").contains("not logged again"),
                    "{line:?}"
                );
            }
            Level::Info => {
                assert_eq!(line.message, "time index rebuilt from the bars");
                rebuilt += 1;
            }
            other => panic!("unexpected store.tix line at {other:?}: {line:?}"),
        }
    }
    (warned, rebuilt)
}

#[test]
fn a_month_that_bisects_is_named_once_per_kind_of_reason_however_many_handles_ask() {
    let log = scratch("log");
    let root = scratch("root");
    let sink =
        telemetry::install(&telemetry::Config::new(&log).with_min_level(telemetry::Level::Trace))
            .expect("nothing else in this test binary installs a sink");

    let a = YearMonth::new(2019, 3).unwrap();
    let b = YearMonth::new(2019, 4).unwrap();
    let tix = |month| {
        path(month)
            .with_file(FileKind::TimeIndex)
            .to_path_buf(&root)
    };
    for month in [a, b] {
        let mut writer = BarFile::open_or_create(&root, path(month), SYMBOL).unwrap();
        assert!(matches!(
            writer.append(&session(month)),
            Ok(Appended::Committed { .. })
        ));
        drop(writer);
        fs::remove_file(tix(month)).unwrap();
    }
    let probe = |month: YearMonth| session(month)[200].ts_micros + 1;

    // Month A, no index: five handles, as five requests would open.
    for _ in 0..5 {
        let reader = BarFile::open_existing(&root, path(a), SYMBOL).unwrap();
        assert_eq!(reader.time_lookup(), TimeLookup::Bisection(Why::Absent));
        assert_eq!(reader.first_at_or_after(probe(a)), Ok(201));
    }
    // Month B, no index: three handles.
    for _ in 0..3 {
        let reader = BarFile::open_existing(&root, path(b), SYMBOL).unwrap();
        assert_eq!(reader.first_at_or_after(probe(b)), Ok(201));
        assert_eq!(reader.time_lookup(), TimeLookup::Bisection(Why::Absent));
    }
    // Month A again, now for ANOTHER kind of reason — a new fact, named.
    fs::write(tix(a), [0u8; 10]).unwrap();
    let shorter = Why::Header("it is shorter than its header");
    for _ in 0..4 {
        let reader = BarFile::open_existing(&root, path(a), SYMBOL).unwrap();
        assert_eq!(reader.time_lookup(), TimeLookup::Bisection(shorter.clone()));
        assert_eq!(reader.first_at_or_after(probe(a)), Ok(201));
    }
    // Month B gets its index; then an entry is damaged under a handle that
    // already confirmed it, and three lookups meet it.
    assert_eq!(
        BarFile::index_existing(&root, path(b), SYMBOL)
            .unwrap()
            .outcome,
        Reindexed::Built(Why::Absent)
    );
    let reader = BarFile::open_existing(&root, path(b), SYMBOL).unwrap();
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    let geometry = store::time_index::Geometry::new(b, 60, SYMBOL);
    let (bucket, _) = store::time_index::split(geometry.slot_of(probe(b)).unwrap());
    let entry = store::time_index::entry_offset(bucket);
    // One bit of the entry's occupancy flipped: its checksum, which covers
    // its bytes and its position, no longer holds.
    let held = fs::read(tix(b)).unwrap()[usize::try_from(entry).unwrap()];
    write_in_place(&tix(b), entry, &[held ^ 1]);
    for _ in 0..3 {
        assert_eq!(reader.first_at_or_after(probe(b)), Ok(201));
    }
    drop(reader);

    let (warned, rebuilt) = read_back(&log, sink.keep_files());
    let expected: BTreeMap<(String, String), usize> = [
        ("2019-03.tix", Why::Absent.to_string()),
        ("2019-03.tix", shorter.to_string()),
        ("2019-04.tix", Why::Absent.to_string()),
        ("2019-04.tix", Why::Entry { bucket }.to_string()),
    ]
    .into_iter()
    .map(|(name, reason)| ((name.to_owned(), reason), 1))
    .collect();
    assert_eq!(
        warned, expected,
        "twelve handles and three failed lookups, four facts: each named once"
    );
    assert_eq!(rebuilt, 1, "the one index built says so once");

    let _ = fs::remove_dir_all(&log);
    let _ = fs::remove_dir_all(&root);
}
