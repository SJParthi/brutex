//! R4-28 repair 5 (D-7191; `docs/04-invariants.md` R428-31): A WHOLE-PLAN
//! REFUSAL OF A FUTURES PULL IS A `pull.futures` EVENT.
//!
//! `fnowork::futures_plan` refuses a whole plan three ways: no receipted
//! snapshot at all (`no_snapshot`), a newest snapshot dated after today
//! (`snapshot_from_the_future`), and an index it cannot read
//! (`futures_index`). Each reached the receipt and `/audit` and no event, so
//! `/logs` showed a futures pull that planned nothing as no pull at all. This
//! binary asks for each refusal and reads the lines off the file the sink
//! wrote.
//!
//! Its own test binary because `telemetry::install` is a process singleton.
//! Every row here is invented: the underlying `ZQIDX` and a token in the eight
//! millions. The header is Kite's published column list, a schema and not a
//! row.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::path::PathBuf;

use pull::master_snapshots::{Book, Snapshot, Stamp};
use pull::session::{Day, Window};

/// A new, empty folder of this test's own. Left for the main session's
/// clean-up: nothing in this binary deletes a file.
fn scratch(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!(
        "brutex-r428-futures-refusals-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real day")
}

/// A book in memory over one invented snapshot taken on `on`.
fn one_snapshot(on: Day) -> Book {
    let csv = format!(
        "{}\n8400101,32812,ZQIDX26JANFUT,\"ZQIDX\",0,2026-01-27,0,0.05,75,FUT,NFO-FUT,NFO\n",
        pull::vendor::KITE_MASTER_HEADER
    );
    let snapshot = Snapshot::parse(
        Stamp {
            day: on,
            ordinal: 1,
        },
        &csv,
    )
    .expect("it parses");
    Book::of(vec![snapshot]).expect("a book")
}

/// The plan of ZQIDX's futures over 2026-01-01..09, asked on `today`.
fn plan(book: &Book, today: Day) -> String {
    pull::fnowork::futures_plan(
        book,
        pull::vendor::ZERODHA_CONTINUOUS,
        "ZQIDX",
        Window::new(day(2026, 1, 1), day(2026, 1, 9)).expect("an ordered window"),
        today,
        day(2026, 1, 9),
    )
    .expect_err("each plan here is refused whole")
}

#[test]
fn each_whole_plan_refusal_is_a_futures_error_event_naming_it() {
    let logs = scratch("logs");
    let config = telemetry::Config::new(&logs).with_min_level(telemetry::Level::Trace);
    let sink = telemetry::install(&config).expect("the only install in this binary");

    let none = plan(
        &Book::of(Vec::new()).expect("an empty book"),
        day(2026, 1, 12),
    );
    let ahead = plan(&one_snapshot(day(2026, 1, 20)), day(2026, 1, 12));
    let memory = plan(&one_snapshot(day(2026, 1, 2)), day(2026, 1, 12));
    assert!(none.starts_with("no_snapshot: "), "{none}");
    assert!(ahead.starts_with("snapshot_from_the_future: "), "{ahead}");
    assert!(
        memory.starts_with("futures_index: a book built in memory has no snapshots directory"),
        "{memory}"
    );

    sink.sync().expect("the log is durable before it is read");
    let log = std::fs::read_to_string(telemetry::current_path(&logs)).expect("the log");
    let lines: Vec<&str> = log
        .lines()
        .filter(|line| line.contains("\"target\":\"pull.futures\""))
        .collect();
    assert_eq!(lines.len(), 3, "one line per refused plan: {lines:#?}");
    for (line, code) in lines.iter().zip([
        "no_snapshot: ",
        "snapshot_from_the_future: ",
        "futures_index: ",
    ]) {
        let why = format!("\"why\":\"{code}");
        for fragment in [
            "\"level\":\"error\"",
            "\"msg\":\"no future of the underlying was planned\"",
            "\"underlying\":\"ZQIDX\"",
            why.as_str(),
        ] {
            assert!(line.contains(fragment), "{fragment} in {line}");
        }
        assert!(!line.contains("\"cut\":true"), "nothing cut: {line}");
    }
    assert_eq!(sink.health().dropped, 0, "nothing dropped by the sink");
}
