//! AF-02 (ingest-6): import records tell the truth about filtered days.
//!
//! A day the journal closed only under a symbol filter (`BANKNIFTY,NIFTY`, the
//! 277-name key) has no `*` row, and `merged` used to read that as "the
//! journal does not name this day" and add a log-only `*` row, `refused`, for
//! the very refusals the filtered close already counts: a phantom refused day
//! on `/imports.json`, counted again by `/audit.json`, and served by
//! `/dayflags.json` as every underlying's import outcome. A log-only row is now
//! added only for a kind-day no journal line of any filter names (DB-025's
//! shape), and a run whose lifecycle is in no directory this route reads says
//! where it looked.
//!
//! EVERY FIXTURE IS INVENTED: the filters name real underlyings because the
//! rule is about names, but every ticker is a `ZZ` one no exchange lists, and
//! every count is a small round number.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]

use super::*;

fn ask(query: &str) -> Asked {
    asked(query).unwrap_or_else(|why| panic!("{query}: {why}"))
}

/// One importer `refused` event as the `cli` ring holds it. No run id: the
/// importer's events carry none (`gdfl_import::note_failure`).
fn refusal(seq: u32, kind: &str, day: &str, about: &str, why: &str, stage: &str) -> String {
    let stage = if stage.is_empty() {
        String::new()
    } else {
        format!(",\"stage\":\"{stage}\"")
    };
    format!(
        "{{\"seq\":{seq},\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":{},\"level\":\"error\",\
         \"target\":\"gdfl.import\",\"msg\":\"refused\",\"fields\":{{\"kind\":\"{kind}\",\
         \"day\":\"{day}\",\"about\":\"{about}\",\"why\":\"{why}\"{stage}}}}}\n",
        1_791_171_078_967_u64 + u64::from(seq)
    )
}

/// `n` distinct vendor refusals of `kind` on `day`, from `seq` on.
fn refusals(seq: u32, n: u32, kind: &str, day: &str) -> String {
    (0..n)
        .map(|i| {
            refusal(
                seq + i,
                kind,
                day,
                &format!("ZZTEST{i}APR2690000CE"),
                "line 4: not ten well-formed fields",
                "vendor",
            )
        })
        .collect()
}

/// A store root of its own with `journal` as its journal and `ring` as the
/// `cli` ring's one file (none when empty).
fn store(name: &str, journal: &str, ring: &str) -> std::path::PathBuf {
    let root =
        crate::server::import_journal::tests::journal(&format!("imports-af02-{name}"), journal);
    if !ring.is_empty() {
        let dir = root.join("logs").join("cli");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(dir.join("events.ndjson"), ring).expect("write the cli ring");
    }
    root
}

/// The 277-name key's shape: `names` comma-joined, sorted as the importer
/// writes it. Invented names around the ones a test asks about.
fn long_filter(with: &[&str], total: usize) -> String {
    let mut names: Vec<String> = with.iter().map(|name| (*name).to_owned()).collect();
    let mut i = 0;
    while names.len() < total {
        names.push(format!("ZZ{i:03}"));
        i += 1;
    }
    names.sort();
    names.join(",")
}

/// A `begin` and `done` pair under `filter`.
fn closed(kind: &str, day: &str, filter: &str, stats: &str) -> String {
    format!(
        "begin {kind} {day} {filter}\ndone {kind} {day} {filter} definition=2 files=7 \
         seconds=70 late=0 late_unresolved=0 max_back_s=0 {stats}\n"
    )
}

/// The journal of the operator's 2026-04-16: closed clean under
/// `BANKNIFTY,NIFTY`, then with ten vendor refusals under a three-name list.
fn filtered_day() -> String {
    let mut journal = closed("options", "2026-04-16", "BANKNIFTY,NIFTY", "failures=0");
    journal.push_str(&closed(
        "options",
        "2026-04-16",
        "BANKNIFTY,NIFTY,ZZTEST",
        "failures=10 vendor_refused=10 reason=malformed_row run=1791943060",
    ));
    journal
}

fn rows(got: &Value) -> &Vec<Value> {
    got["rows"].as_array().expect("rows")
}

/// **A DAY CLOSED UNDER A SYMBOL FILTER WITH LOGGED REFUSALS IS NOT A REFUSED
/// ROW** (ingest-6): the day is listed by its two journal closes, the ten
/// refusals sit beside them as the run's evidence, and no `*` row is made up
/// for them. Before AF-02 the answer held a third row, `*` `refused` from the
/// log, and `counts.refused` was 1.
#[test]
fn a_day_closed_under_a_symbol_filter_with_logged_refusals_is_not_a_refused_row() {
    let root = store(
        "symbol-filter",
        &filtered_day(),
        &refusals(1, 10, "options", "2026-04-16"),
    );
    let got = answer(
        &root,
        &ask("feed=gdfl&kind=options&from=2026-04-16&to=2026-04-16"),
    )
    .expect("reads");
    let rows = rows(&got);
    assert_eq!(rows.len(), 2, "{got:#}");
    assert!(
        rows.iter()
            .all(|row| row["filter"] != "*" && row["outcome_from"] == "journal"),
        "{got:#}"
    );
    assert_eq!(got["counts"]["refused"], 0, "{got:#}");
    assert_eq!(got["counts"]["done_with_refusals"], 1);
    assert_eq!(got["counts"]["done"], 1);
    let with = rows
        .iter()
        .find(|row| row["outcome"] == "done_with_refusals")
        .expect("the three-name close");
    assert_eq!(with["filter"], "BANKNIFTY,NIFTY,ZZTEST");
    assert_eq!(with["vendor_refused"], 10);
    assert_eq!(with["run"], 1_791_943_060_u64);
    assert_eq!(
        with["logged"].as_array().expect("logged").len(),
        10,
        "the refusals are the close's evidence"
    );
    let refused = answer(&root, &ask("feed=gdfl&outcome=refused")).expect("reads");
    assert!(rows_of_len(&refused, 0), "{refused:#}");
    assert_eq!(refused["counts"]["refused"], 0);
}

fn rows_of_len(got: &Value, n: usize) -> bool {
    got["rows"].as_array().is_some_and(|rows| rows.len() == n)
}

/// **THE AUDIT SUMMARY COUNTS THAT DAY ONCE** (ingest-6): `/audit.json`'s
/// counts come from the same walk, so the day is one `done_with_refusals` and
/// no `refused`, and no not-done day is named.
#[test]
fn the_audit_summary_counts_that_day_once() {
    let root = store(
        "summary-once",
        &filtered_day(),
        &refusals(1, 10, "options", "2026-04-16"),
    );
    let got = summary(&root);
    assert_eq!(got["counts"]["refused"], 0, "{got:#}");
    assert_eq!(got["counts"]["done_with_refusals"], 1);
    assert_eq!(got["counts"]["done"], 1);
    assert_eq!(got["newest_not_done"], Value::Null, "{got:#}");
    // THE TWO ROUTES AGREE, count by count.
    let whole = answer(&root, &ask("feed=gdfl")).expect("reads");
    assert_eq!(got["counts"], whole["counts"]);
}

/// **A DAY WITH NO JOURNAL LINE UNDER ANY FILTER IS STILL READ FROM THE LOG**
/// (DB-025 kept): the options day the calendar declined, and a stocks day the
/// journal names only for options, each get their log-only `*` row; a kind
/// is matched as well as a day.
#[test]
fn a_day_with_no_journal_line_under_any_filter_is_still_read_from_the_log() {
    let mut ring = refusal(
        1,
        "options",
        "2018-11-07",
        "calendar",
        "the calendar says OpenLengthUnmeasured, not a regular session",
        "",
    );
    ring.push_str(&refusals(2, 3, "options", "2026-04-16"));
    ring.push_str(&refusal(
        9,
        "stocks",
        "2026-04-16",
        "ZZSTOCK.NSE_EQ.csv",
        "line 3: not ten well-formed fields",
        "vendor",
    ));
    let root = store("still-logged", &filtered_day(), &ring);
    // THE RANGE NAMED: unasked, the walk is the journal's own span, which
    // here starts after 2018.
    let got = answer(
        &root,
        &ask("feed=gdfl&outcome=refused&order=asc&from=2018-01-01&to=2026-12-31"),
    )
    .expect("reads");
    let rows = rows(&got);
    assert_eq!(rows.len(), 2, "{got:#}");
    assert_eq!(rows[0]["day"], "2018-11-07");
    assert_eq!(rows[0]["kind"], "options");
    assert_eq!(rows[0]["filter"], "*");
    assert_eq!(rows[0]["outcome_from"], "log");
    assert_eq!(rows[0]["line"], Value::Null);
    assert_eq!(rows[0]["stage"], "calendar");
    assert_eq!(rows[1]["day"], "2026-04-16");
    assert_eq!(rows[1]["kind"], "stocks", "a kind is matched with its day");
    assert_eq!(rows[1]["outcome_from"], "log");
    assert_eq!(got["counts"]["refused"], 2);
}

/// **A RUN LOGGED ELSEWHERE SAYS WHERE IT LOOKED** (ingest-6 part 3): a run
/// whose `cli` was given `BRUTEX_LOG_DIR` logged its lifecycle to
/// `logs/cli-batch`, which this route does not read. Its answer names the
/// directories read and the one beside them that was not, instead of a bare
/// `events_total: 0`.
#[test]
fn a_run_logged_elsewhere_says_where_it_looked() {
    let mut journal = "run 1791943060 source=zip archive=/zz/a.zip archive_bytes=7 commit=abc \
         started=1791943060 archive_cd_blake3=00\n"
        .to_owned();
    journal.push_str(&filtered_day());
    let root = store("elsewhere", &journal, "");
    let batch = root.join("logs").join("cli-batch");
    std::fs::create_dir_all(&batch).expect("mkdir");
    std::fs::write(
        batch.join("events.ndjson"),
        "{\"seq\":1,\"run\":1791943060,\"ts\":\"2026-10-06T08:57:38.499Z\",\
         \"ms\":1791943060000,\"level\":\"info\",\"target\":\"gdfl.import.lifecycle\",\
         \"msg\":\"run claimed\",\"fields\":{}}\n",
    )
    .expect("write");
    let got = answer(&root, &ask("feed=gdfl&run=1791943060")).expect("reads");
    let lifecycle = &got["run"]["lifecycle"];
    assert_eq!(lifecycle["events_total"], 0, "{got:#}");
    let says = lifecycle["says"]
        .as_str()
        .expect("a sentence, not a bare zero");
    assert!(
        says.starts_with("no lifecycle event in logs/cli, "),
        "{says}"
    );
    assert!(says.contains("logs/gdfl/options"), "{says}");
    assert!(says.contains("logs/cli-batch"), "{says}");
    assert_eq!(lifecycle["not_read"], json!(["logs/cli-batch"]));
    // The run itself was found: its journal line is there.
    assert_eq!(got["run"]["journal_lines"].as_array().unwrap().len(), 1);
}

/// **A STORE WITH NO `logs` DIRECTORY AT ALL** still says where it looked,
/// and names nothing as unread.
#[test]
fn a_run_on_a_store_with_no_logs_says_where_it_looked() {
    let root = store("no-logs", &filtered_day(), "");
    let got = answer(&root, &ask("feed=gdfl&run=1791943060")).expect("reads");
    let lifecycle = &got["run"]["lifecycle"];
    assert_eq!(lifecycle["not_read"], json!([]), "{got:#}");
    assert!(
        lifecycle["says"]
            .as_str()
            .is_some_and(|says| says.starts_with("no lifecycle event in logs/cli")),
        "{got:#}"
    );
}

/// THE EXTREMES (operator's rule 4), each in one store:
///
/// * an EMPTY journal: a logged refusal is the only row, read from the log;
/// * a ONE-NAME filter and a day closed under BOTH `*` and a list: no
///   log-only row for either;
/// * a day BEGUN under a list and never closed (killed mid-write): `open`,
///   and its refusals make no `refused` row beside it;
/// * DUPLICATE names in a filter;
/// * a 277-name filter;
/// * a day whose refusal events have ROLLED OUT of the ring: the close stays
///   `done_with_refusals`, nothing is invented.
#[test]
fn every_filter_shape_suppresses_the_log_only_row_and_nothing_else() {
    // EMPTY.
    let root = store("empty", "", &refusals(1, 2, "options", "2026-04-16"));
    let got = answer(&root, &ask("feed=gdfl&from=2026-04-16&to=2026-04-16")).expect("reads");
    assert_eq!(rows(&got).len(), 1, "{got:#}");
    assert_eq!(rows(&got)[0]["outcome_from"], "log");
    assert_eq!(rows(&got)[0]["failures"], 2);
    assert_eq!(got["counts"]["refused"], 1);

    let wide = long_filter(&["BANKNIFTY", "NIFTY", "NIFTYNXT50"], 277);
    let mut journal = String::new();
    journal.push_str(&closed("options", "2026-04-13", "NIFTY", "failures=0"));
    journal.push_str(&closed("options", "2026-04-15", "*", "failures=0"));
    journal.push_str(&closed(
        "options",
        "2026-04-15",
        "BANKNIFTY,NIFTY",
        "failures=0",
    ));
    journal.push_str("begin options 2026-04-17 BANKNIFTY,NIFTY\n");
    journal.push_str(&closed(
        "options",
        "2026-04-20",
        "NIFTY,NIFTY,ZZTEST",
        "failures=0",
    ));
    journal.push_str(&closed(
        "options",
        "2026-04-21",
        &wide,
        "failures=3 vendor_refused=3 reason=malformed_row",
    ));
    // ROLLED OUT: a close with refusals and no event left in the ring.
    journal.push_str(&closed(
        "options",
        "2026-04-22",
        "BANKNIFTY,NIFTY",
        "failures=4 vendor_refused=4 reason=malformed_row",
    ));
    let mut ring = String::new();
    for (at, day) in [
        (1, "2026-04-13"),
        (10, "2026-04-15"),
        (20, "2026-04-17"),
        (30, "2026-04-20"),
        (40, "2026-04-21"),
    ] {
        ring.push_str(&refusals(at, 3, "options", day));
    }
    let root = store("shapes", &journal, &ring);
    let got = answer(
        &root,
        &ask("feed=gdfl&kind=options&from=2026-04-13&to=2026-04-22&order=asc"),
    )
    .expect("reads");
    let rows = rows(&got);
    assert!(
        rows.iter().all(|row| row["outcome_from"] == "journal"),
        "{got:#}"
    );
    let days: Vec<(&str, &str)> = rows
        .iter()
        .map(|row| {
            (
                row["day"].as_str().unwrap(),
                row["outcome"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        days,
        [
            ("2026-04-13", "done"),
            ("2026-04-15", "done"),
            ("2026-04-15", "done"),
            ("2026-04-17", "open"),
            ("2026-04-20", "done"),
            ("2026-04-21", "done_with_refusals"),
            ("2026-04-22", "done_with_refusals"),
        ]
    );
    assert_eq!(got["counts"]["refused"], 0, "{got:#}");
    assert_eq!(got["counts"]["open"], 1);
    let summary = summary(&root);
    assert_eq!(summary["counts"]["refused"], 0, "{summary:#}");
}

/// THE MERGE'S PROBE PER LOGGED DAY IS ONE RANGE OF THAT DAY'S KEYS, NOT A
/// WALK OF THE JOURNAL: a journal of many filtered days and as many logged
/// ones answers with exactly one row per close and no log-only row, whatever
/// the filters' length.
#[test]
fn a_month_of_filtered_closes_with_refusals_holds_no_phantom() {
    let wide = long_filter(&["BANKNIFTY", "NIFTY"], 277);
    let mut journal = String::new();
    let mut ring = String::new();
    let mut seq = 1;
    for d in 1..=28 {
        let day = format!("2026-02-{d:02}");
        journal.push_str(&closed("options", &day, "BANKNIFTY,NIFTY", "failures=0"));
        journal.push_str(&closed(
            "options",
            &day,
            &wide,
            "failures=2 vendor_refused=2 reason=malformed_row",
        ));
        ring.push_str(&refusals(seq, 2, "options", &day));
        seq += 2;
    }
    let root = store("month", &journal, &ring);
    let got = answer(&root, &ask("feed=gdfl&from=2026-02-01&to=2026-02-28")).expect("reads");
    assert_eq!(got["rows_in_range"], 56, "{}", got["counts"]);
    assert_eq!(got["counts"]["refused"], 0);
    assert_eq!(got["counts"]["done_with_refusals"], 28);
    assert_eq!(got["counts"]["done"], 28);
}
