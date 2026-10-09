//! WHAT THE GDFL IMPORT SAYS IN THE LOG WHEN THE CENSUS REFUSES, AND ON EVERY
//! DAY IT CLOSES (D-3317; DB-G01, DB-G10, DB-G56, DB-025).
//!
//! Until D-3317 a census refusal was one `error` "refused" line followed by an
//! `info` "day imported" for the same day, and the run went on to file the
//! next day's bars. Now the first census refusal is ONE `error` line, "census
//! refused; run stopped", the day is never logged as imported, and the run
//! does not finish: it stops. Every other day closes with one summary line
//! whose level and `outcome` say how it closed, and "run started" names the
//! origin the caller measured.
//!
//! Its own test binary because `telemetry::install` is a process singleton.
//! Every value here is invented at run time; no vendor row is quoted.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail; one install per binary makes this one test"
)]

use std::path::{Path, PathBuf};

use pull::gdfl_cm::CmRefusal;
use pull::gdfl_import::{
    DayRead, ImportKind, ImportRefusal, Origin, Refused, Run, RunOptions, SourceKind, Tick,
    TickFile, drive, drive_from, journal_path,
};
use pull::session::Day;

const NINE_FIFTEEN: u32 = 9 * 3_600 + 15 * 60;
const CLOSE: u32 = 15 * 3_600 + 30 * 60;

fn day(d: u8) -> Day {
    Day::new(2024, 4, d).expect("a real April 2024 day")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-gdfl-census-stop-{tag}-{}",
        std::process::id()
    ));
    let _best_effort = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch root");
    dir
}

/// One invented file of `symbol`: traded rows through the session.
fn file(symbol: &str, ltq: u64) -> TickFile {
    TickFile {
        symbol: symbol.to_owned(),
        contract: None,
        name: format!("{symbol}.invented"),
        ticks: [NINE_FIFTEEN, NINE_FIFTEEN + 61, CLOSE - 1, CLOSE + 90]
            .into_iter()
            .map(|sod| Tick {
                sod,
                ltp: 10_000 + i64::from(sod % 7),
                ltq,
                oi: None,
            })
            .collect(),
    }
}

/// A refusal of the vendor's own bytes by a named reader rule: one that
/// closes a day done with refusals (D-3317 item 2).
fn vendor_refusal(name: &str) -> Refused {
    Refused::cm(name, &CmRefusal::PriceRefused { line: 2 })
}

/// The smallest zip there is: an end record and nothing else.
const EMPTY_ZIP: [u8; 22] = [
    0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// The log lines of this binary's sink.
fn log_lines(logs: &Path) -> Vec<String> {
    std::fs::read_to_string(telemetry::current_path(logs))
        .expect("the log")
        .lines()
        .filter(|line| line.contains("\"gdfl.import\""))
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_census_refusal_is_one_error_line_and_every_closed_day_says_how_it_closed() {
    let logs = scratch("logs");
    let config = telemetry::Config::new(&logs).with_min_level(telemetry::Level::Trace);
    let sink = telemetry::install(&config).expect("the only install in this binary");

    // ── A: the census refuses the first day; the run stops there ───────────
    let stopped = scratch("stopped");
    std::fs::create_dir_all(stopped.join("manifest/gdfl.man")).expect("a census that is a dir");
    let run = Run {
        kind: ImportKind::Stocks,
        from: day(1),
        to: day(2),
        only: &[],
        store_root: &stopped,
    };
    let mut read = Vec::new();
    let got = drive(&run, |on, sink| {
        read.push(on);
        sink(file("RELIANCE", 3));
        DayRead {
            held: true,
            ..DayRead::default()
        }
    })
    .unwrap_err();
    assert_eq!(read, vec![day(1)], "the second day is never read");
    let ImportRefusal::CensusRefused { day: at, done, .. } = &got else {
        panic!("{got:?}");
    };
    assert_eq!((*at, done.days_imported), (day(1), 0));
    let journal = std::fs::read_to_string(journal_path(&stopped)).expect("the journal");
    assert!(
        journal
            .lines()
            .last()
            .is_some_and(|l| l.starts_with("incomplete stocks 2024-04-01 * ")
                && l.contains(" stage=census ")),
        "{journal}"
    );

    // ── B: a measured origin; a day done with refusals and a refused day ───
    let archive = scratch("archive");
    std::fs::write(archive.join("2024.zip"), EMPTY_ZIP).expect("an archive");
    let origin = Origin::measure(SourceKind::Zip, &archive, Some("feedc0de"));
    assert_eq!(origin.fault, None, "{origin:?}");
    let ledger = scratch("ledger");
    let run = Run {
        kind: ImportKind::Indices,
        from: day(1),
        to: day(2),
        only: &[],
        store_root: &ledger,
    };
    let options = RunOptions {
        origin: Some(&origin),
        retry_refused: false,
    };
    let report = drive_from(&run, options, |on, sink| {
        if on == day(1) {
            sink(file("NIFTY", 0));
        }
        DayRead {
            held: true,
            refused: vec![vendor_refusal("NIFTY BANK.invented")],
            ..DayRead::default()
        }
    })
    .expect("the run finishes");
    assert_eq!(report.closed_with_refusals, vec![day(1)]);
    assert_eq!(report.closed_refused, vec![day(2)]);
    assert_eq!(report.days_imported, 2);

    sink.sync().expect("the log is durable before it is read");
    let lines = log_lines(&logs);
    let with = |parts: &[&str]| -> Vec<&String> {
        lines
            .iter()
            .filter(|line| parts.iter().all(|part| line.contains(part)))
            .collect()
    };
    // A: one error line naming the stop, and nothing that says imported.
    let stop = with(&["\"census refused; run stopped\"", "\"stocks\""]);
    assert_eq!(stop.len(), 1, "{lines:#?}");
    assert!(
        stop[0].contains("\"error\"") && stop[0].contains("\"2024-04-01\""),
        "{}",
        stop[0]
    );
    assert!(
        stop[0].contains("nothing of the batch was published"),
        "{}",
        stop[0]
    );
    assert!(
        with(&["\"day imported\"", "\"stocks\""]).is_empty(),
        "a census-refused day is never logged as imported: {lines:#?}"
    );
    assert!(
        with(&["\"run finished\"", "\"stocks\""]).is_empty(),
        "a stopped run did not finish: {lines:#?}"
    );
    assert_eq!(
        with(&["\"run started\"", "\"stocks\"", "\"unrecorded\""]).len(),
        1,
        "an origin nobody measured is named unrecorded: {lines:#?}"
    );
    // B: the origin in "run started", and each day's close.
    let started = with(&["\"run started\"", "\"indices\""]);
    assert_eq!(started.len(), 1, "{lines:#?}");
    for part in [
        "\"zip\"",
        "\"feedc0de\"",
        &format!("\"{}\"", origin.digest_hex()),
        "\"archive_bytes\"",
        "\"archive_cd_blake3\"",
    ] {
        assert!(started[0].contains(part), "{} lacks {part}", started[0]);
    }
    let imported = with(&["\"day imported\"", "\"indices\"", "\"2024-04-01\""]);
    assert_eq!(imported.len(), 1, "{lines:#?}");
    assert!(
        imported[0].contains("\"done_with_refusals\"") && imported[0].contains("\"info\""),
        "{}",
        imported[0]
    );
    let refused = with(&["\"day refused\"", "\"indices\"", "\"2024-04-02\""]);
    assert_eq!(refused.len(), 1, "{lines:#?}");
    assert!(
        refused[0].contains("\"warn\"") && refused[0].contains("\"refused\""),
        "{}",
        refused[0]
    );
    assert_eq!(with(&["\"run finished\"", "\"indices\""]).len(), 1);
    assert_eq!(
        sink.health().dropped,
        0,
        "nothing dropped, so the counts are exact"
    );

    for dir in [logs, stopped, archive, ledger] {
        let _best_effort = std::fs::remove_dir_all(dir);
    }
}
