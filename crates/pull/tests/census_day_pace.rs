//! THE IMPORT SAYS WHAT EACH DAY'S CENSUS PUBLISH COST, AND HOW IT READ THE
//! CENSUS (R3-19, D-6891).
//!
//! A run keeps its census decoded between its days, so a day's publish reads
//! one header region instead of the whole list. That is only visible if the
//! log says it: every "day imported" carries `census_ms`, and every census a
//! day published is one `pull.census` "census published" line saying `read`
//! (`kept` or `whole`) and `why`. A day that read its census whole is then
//! named on `/logs`, never only slow.
//!
//! Its own test binary because `telemetry::install` is a process singleton.
//! Every value here is invented at run time; no vendor row is quoted.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail; one install per binary makes this one test"
)]

use std::path::{Path, PathBuf};

use pull::gdfl_import::{DayRead, ImportKind, Run, RunOptions, Tick, TickFile, drive_from};
use pull::session::Day;

const NINE_FIFTEEN: u32 = 9 * 3_600 + 15 * 60;
const CLOSE: u32 = 15 * 3_600 + 30 * 60;

fn day(d: u8) -> Day {
    Day::new(2024, 4, d).expect("a real April 2024 day")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-census-day-pace-{tag}-{}",
        std::process::id()
    ));
    let _best_effort = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch root");
    dir
}

/// One invented file of `symbol`: four traded rows through the session, at
/// prices no real index has.
fn file(symbol: &str) -> TickFile {
    TickFile {
        symbol: symbol.to_owned(),
        contract: None,
        name: format!("{symbol}.invented"),
        ticks: [NINE_FIFTEEN, NINE_FIFTEEN + 61, CLOSE - 1, CLOSE - 2]
            .into_iter()
            .map(|sod| Tick {
                sod,
                ltp: 10_000 + i64::from(sod % 7),
                ltq: 0,
                oi: None,
            })
            .collect(),
    }
}

/// The log lines of this binary's sink.
fn log_lines(logs: &Path) -> Vec<String> {
    std::fs::read_to_string(telemetry::current_path(logs))
        .expect("the log")
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_day_says_its_census_time_and_how_the_census_was_read() {
    let logs = scratch("logs");
    let config = telemetry::Config::new(&logs).with_min_level(telemetry::Level::Trace);
    let sink = telemetry::install(&config).expect("the only install in this binary");

    let store = scratch("store");
    let run = Run {
        kind: ImportKind::Indices,
        from: day(1),
        to: day(4),
        only: &[],
        store_root: &store,
    };
    let report = drive_from(&run, RunOptions::default(), |_on, sink| {
        sink(file("NIFTY"));
        DayRead {
            held: true,
            ..DayRead::default()
        }
    })
    .expect("the run finishes");
    assert_eq!(report.days_imported, 4, "{report:?}");

    sink.sync().expect("the log is durable before it is read");
    let lines = log_lines(&logs);
    let with = |parts: &[&str]| -> Vec<&String> {
        lines
            .iter()
            .filter(|line| parts.iter().all(|part| line.contains(part)))
            .collect()
    };
    // Every day imported says its census time.
    let imported = with(&["\"gdfl.import\"", "\"day imported\""]);
    assert_eq!(imported.len(), 4, "{lines:#?}");
    for line in &imported {
        assert!(line.contains("\"census_ms\":"), "{line}");
    }
    // Every census a day published says how it read it: the first two days
    // read whole (no census yet, then the run's first read of it), the rest
    // from the run's memory.
    let published = with(&["\"pull.census\"", "\"census published\""]);
    assert_eq!(published.len(), 4, "{lines:#?}");
    for (at, (read, why)) in [
        ("whole", "first"),
        ("whole", "first"),
        ("kept", "-"),
        ("kept", "-"),
    ]
    .into_iter()
    .enumerate()
    {
        let line = published[at];
        assert!(
            line.contains(&format!("\"read\":\"{read}\""))
                && line.contains(&format!("\"why\":\"{why}\"")),
            "day {}: {line}",
            at + 1
        );
        assert!(
            line.contains("\"load_us\":") && line.contains("\"total_us\":"),
            "{line}"
        );
    }
    assert_eq!(
        sink.health().dropped,
        0,
        "nothing dropped, so the counts are exact"
    );

    for dir in [logs, store] {
        let _best_effort = std::fs::remove_dir_all(dir);
    }
}
