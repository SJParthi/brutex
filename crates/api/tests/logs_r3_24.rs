//! R3-24's three confirmed findings, each through `/logs.json`'s public body
//! (`api::logs::logs_json_over`), the one entry point the route and the
//! C-LG-01 bench share.
//!
//! # Why an integration test
//!
//! Each test here names only what `feat/db-fix` already exported before
//! R3-24: `logs_json_over`, the sink's public `Config` and `Event`, and the
//! importer's `journal_path`. So this same file compiles on the base, where
//! each test FAILS for the reason its finding states, and on R3-24, where it
//! passes. The unit tests in `logs.rs` and `sink.rs` prove the same
//! properties in more depth through R3-24's own names, which do not exist on
//! the base.
//!
//! The last two tests are R3-24 repair round 1's (20 Oct): each failed on
//! the package's first head, `dbfa9074a`, where the two defects the second
//! check found were made. They name only what the base exported too; on the
//! base the torn-line test passes (the span search did not exist) and the
//! per-request test fails (the state stream did not exist).
//!
//! Every fixture is invented, in a directory the test creates under the
//! system's temporary directory; no line here is a vendor's.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "a failed fixture or assertion must fail the test by name"
)]

use std::path::{Path, PathBuf};

/// A fresh, empty directory of this test's own.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("r324-logs-{name}-{}", std::process::id()));
    let _ignored = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// `/logs.json`'s body over one served directory and an empty store.
fn logs(served: &Path, store: &Path, query: &str) -> serde_json::Value {
    serde_json::from_str(&api::logs::logs_json_over(Some(served), store, query)).expect("JSON")
}

/// The records of an answer, as `(target, message)`.
fn named(got: &serde_json::Value) -> Vec<(String, String)> {
    got["records"]
        .as_array()
        .expect("records")
        .iter()
        .map(|record| {
            (
                record["target"].as_str().expect("a target").to_owned(),
                record["message"].as_str().expect("a message").to_owned(),
            )
        })
        .collect()
}

/// **DB-G88: A STATE CHANGE OUTLIVES THE REFUSALS THAT ROLL THE RING.**
/// `autopilot paused` is one of the ten state changes the 12 Oct audit found
/// only in the rolling ring. A pull's refusals then roll that ring past it
/// (here a 32 KiB x 2 ring and 2 MiB of refusals, the ratio of the 8 MiB x 8
/// ring the finding measured rolling in 8 to 12 minutes). On the base the
/// line was never audited, so asking for it after the roll answers nothing;
/// with R3-24 it is in the never-rolling state stream and is found.
#[test]
fn a_state_change_outlives_the_refusals_that_roll_the_ring() {
    let served = scratch("g88");
    let store = scratch("g88-store");
    {
        let sink = telemetry::Sink::open(
            &telemetry::Config::new(&served)
                .with_max_file_bytes(32 * 1024)
                .with_keep_files(2),
        )
        .expect("opens");
        let _ = sink.emit(&telemetry::Event::warn("autopilot", "paused").with("epoch", 7u64));
        let why = "a refusal's reason, long enough to be a real one ".repeat(8);
        for _ in 0..4_000 {
            let _ = sink.emit(
                &telemetry::Event::error("pull.spot", "member did not land")
                    .with("instrument", "NIFTY")
                    .with("day", "2026-10-01")
                    .with("why", why.as_str()),
            );
        }
    }
    // PREMISE: no file of the ring holds the state change any more.
    for entry in std::fs::read_dir(&served).expect("the directory") {
        let path = entry.expect("an entry").path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with("events") && name.ends_with(".ndjson") {
            let text = std::fs::read_to_string(&path).expect("a ring file");
            assert!(
                !text.contains("\"autopilot\""),
                "premise: the ring rolled the state change away ({name})"
            );
        }
    }
    let got = logs(&served, &store, "target=autopilot&limit=20");
    assert_eq!(
        named(&got),
        [("autopilot".to_owned(), "paused".to_owned())],
        "the state change is found after the ring rolled: {got:#}"
    );
    let _ignored = std::fs::remove_dir_all(&served);
    let _ignored = std::fs::remove_dir_all(&store);
}

/// **DB-023: `target=` ALONE READS EVERY AUDITED LINE, NOT THE NEWEST 4 MiB.**
/// One `pull.spot` refusal, then 4.5 MiB of other lines, all inside the
/// default 8 MiB x 8 ring. On the base `target=` alone filtered the newest
/// 4 MiB of the ring and stopped (`hit_scan_cap`), so the refusal was not
/// found; with R3-24 it goes through the audit index.
#[test]
fn a_target_alone_finds_a_refusal_older_than_the_newest_4_mib() {
    let served = scratch("023");
    let store = scratch("023-store");
    {
        let sink = telemetry::Sink::open(&telemetry::Config::new(&served)).expect("opens");
        let _ = sink.emit(
            &telemetry::Event::error("pull.spot", "member did not land")
                .with("instrument", "NIFTY")
                .with("day", "2026-10-01"),
        );
        let wide = "w".repeat(1024);
        // 8,000 lines of about 650 bytes each (the sink keeps a value to its
        // ceiling): past the 4 MiB the base read, inside the 64 MiB the ring
        // keeps.
        for _ in 0..8_000 {
            let _ = sink.emit(&telemetry::Event::info("api.bars", "busy").with("w", wide.as_str()));
        }
    }
    let ring: u64 = std::fs::read_dir(&served)
        .expect("the directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("events") && n.ends_with(".ndjson"))
        })
        .map(|path| std::fs::metadata(path).expect("a ring file").len())
        .sum();
    assert!(
        ring > 4 * 1024 * 1024 + 256 * 1024,
        "premise: the refusal stands more than 4 MiB back ({ring} bytes)"
    );
    let got = logs(&served, &store, "target=pull.spot&limit=20");
    assert_eq!(
        named(&got),
        [("pull.spot".to_owned(), "member did not land".to_owned())],
        "target alone finds the older refusal: {got:#}"
    );
    let _ignored = std::fs::remove_dir_all(&served);
    let _ignored = std::fs::remove_dir_all(&store);
}

/// **DB-A48: A "DAY IMPORTED" LINE THE JOURNAL CONTRADICTS IS MARKED.** The
/// import journal closed 2020-01-02 `incomplete` and holds no line for
/// 2022-03-07, yet the old CLI log says "day imported" for both. On the base
/// `/logs.json` served both lines with no mark; with R3-24 each carries the
/// journal's outcome beside it, and 2020-01-03, which the journal closed
/// `done`, carries none.
#[test]
fn a_day_imported_line_the_journal_contradicts_is_marked() {
    let root = scratch("a48");
    let journal = pull::gdfl_import::journal_path(&root);
    std::fs::create_dir_all(journal.parent().expect("imports/")).expect("mkdir");
    std::fs::write(
        &journal,
        "begin options 2020-01-02 *\n\
         incomplete options 2020-01-02 * definition=2 files=12 seconds=333 failures=1 late=0 late_unresolved=0 max_back_s=0\n\
         begin options 2020-01-03 *\n\
         done options 2020-01-03 * definition=2 files=11 seconds=222 failures=0 late=0 late_unresolved=0 max_back_s=0\n",
    )
    .expect("the journal");
    {
        let sink = telemetry::Sink::open(&telemetry::Config::new(root.join("logs").join("cli")))
            .expect("opens");
        for day in ["2020-01-02", "2020-01-03", "2022-03-07"] {
            let _ = sink.emit(
                &telemetry::Event::info("gdfl.import", "day imported")
                    .with("kind", "options")
                    .with("day", day),
            );
        }
    }
    let got: serde_json::Value =
        serde_json::from_str(&api::logs::logs_json_over(None, &root, "limit=10")).expect("JSON");
    let conflict_of = |day: &str| -> serde_json::Value {
        got["records"]
            .as_array()
            .expect("records")
            .iter()
            .find(|record| record["fields"]["day"] == day && record["message"] == "day imported")
            .unwrap_or_else(|| panic!("{day}: {got:#}"))
            .get("conflict")
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    };
    let incomplete = conflict_of("2020-01-02");
    assert_eq!(
        incomplete["journal_outcome"], "incomplete",
        "the journal closed 2020-01-02 incomplete: {got:#}"
    );
    assert_eq!(incomplete["journal_line"], 2);
    assert_eq!(
        conflict_of("2022-03-07")["journal_outcome"],
        "no journal line"
    );
    assert!(conflict_of("2020-01-03").is_null(), "done is no conflict");
    let _ignored = std::fs::remove_dir_all(&root);
}

/// One `pull.spot` refusal per `n`, each on day 2020-01-02.
fn refusals(sink: &telemetry::Sink, n: usize) {
    for i in 0..n {
        let _ = sink.emit(
            &telemetry::Event::error("pull.spot", "member did not land")
                .with("instrument", "NIFTY")
                .with("day", "2020-01-02")
                .with("n", u64::try_from(i).expect("small")),
        );
    }
}

/// **R3-24 REPAIR ROUND 1: A TORN LINE NEVER CUTS A SPAN SHORT.** A real
/// sink writes `k` refusals, a kill leaves a torn fragment at the end of the
/// audit file, the next open closes it with its newline and writes `40 - k`
/// more. A span alone (`from=0&to=<tomorrow>`) and a key with the span must
/// each answer all 40, for every `k`, and an answer is called complete only
/// when it is. On the package's first head (`dbfa9074a`) the span alone
/// answered `k` lines for `k` = 21, 32 and 37 (the second check's layouts)
/// and still said "every match the streams hold is on this page".
#[test]
fn a_torn_line_never_cuts_a_span_short() {
    const TOTAL: usize = 40;
    let to = telemetry::now_millis() + 86_400_000;
    let mut short = Vec::new();
    for k in 1..TOTAL {
        let served = scratch(&format!("torn-{k}"));
        let store = scratch(&format!("torn-store-{k}"));
        {
            let sink = telemetry::Sink::open(&telemetry::Config::new(&served)).expect("opens");
            refusals(&sink, k);
        }
        {
            use std::io::Write as _;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(telemetry::audit_current_path(&served))
                .expect("the audit file");
            file.write_all(b"{\"seq\":99999,\"ts\":\"2026-10-0")
                .expect("a torn fragment");
        }
        {
            let sink = telemetry::Sink::open(&telemetry::Config::new(&served)).expect("reopens");
            refusals(&sink, TOTAL - k);
        }
        for query in [
            format!("from=0&to={to}&limit=500"),
            format!("day=2020-01-02&from=0&to={to}&limit=500"),
        ] {
            let got = logs(&served, &store, &query);
            let records = got["records"].as_array().map_or(0, Vec::len);
            let said = got["keyed"]["index"]["said"].as_str().unwrap_or_default();
            if records != TOTAL {
                short.push(format!("k={k} {query}: {records} line(s), said {said:?}"));
            }
        }
        let _ignored = std::fs::remove_dir_all(&served);
        let _ignored = std::fs::remove_dir_all(&store);
    }
    assert!(
        short.is_empty(),
        "a span lost lines after a torn line: {short:#?}"
    );
}

/// **R3-24 REPAIR ROUND 1: A LINE WRITTEN PER REQUEST NEVER REACHES THE
/// NEVER-ROLLING STREAM.** Fifty of each line the server writes per bar
/// request, per CSV export or per minute, at the level it writes it, beside
/// one pull run's start. The state stream holds the run's start alone; the
/// rolling audit stream holds all 201. On the first head the 100 export
/// lines (`export abandoned; work stopped`, `export stopped short`) reached
/// the state stream, which never rolls.
#[test]
fn a_line_written_per_request_never_reaches_the_never_rolling_stream() {
    let served = scratch("per-request");
    {
        let sink = telemetry::Sink::open(&telemetry::Config::new(&served)).expect("opens");
        let _ = sink.emit(&telemetry::Event::info("pull.run", "started"));
        for rows in 0..50u64 {
            for event in [
                telemetry::Event::info("api.bars", "request abandoned; work stopped"),
                telemetry::Event::info("api.speed", "minute ended"),
                telemetry::Event::info("api.bars", "export abandoned; work stopped"),
                telemetry::Event::warn("api.bars", "export stopped short"),
            ] {
                let _ = sink.emit(&event.with("route", "/bars/export.csv").with("rows", rows));
            }
        }
    }
    let lines = |path: &Path| -> Vec<String> {
        std::fs::read_to_string(path)
            .expect("a stream")
            .lines()
            .map(|line| {
                let record: serde_json::Value = serde_json::from_str(line).expect("a record");
                record["msg"].as_str().expect("a message").to_owned()
            })
            .collect()
    };
    // `<log dir>/audit.state/`, spelt out so this file still names only what
    // the base exported (`telemetry::run_dirs::state_stream_of` is R3-24's).
    let state = telemetry::audit_current_path(&served.join("audit.state"));
    assert_eq!(lines(&state), ["started"], "the never-rolling stream");
    assert_eq!(
        lines(&telemetry::audit_current_path(&served)).len(),
        201,
        "the rolling audit stream keeps every line"
    );
    let _ignored = std::fs::remove_dir_all(&served);
}
