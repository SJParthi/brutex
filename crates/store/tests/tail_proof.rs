//! The D-0688 tail proof runs again on every touch of the tail that follows a
//! touch of another block, counted off the log:
//! `store::tail_proof::every_touch_of_the_tail_after_another_block_runs_its_proof_again`.
//! The other two tests here read the invariant rows that describe it.
//!
//! # What the answers could not show
//!
//! A read handle remembers ONE verified block. A read that alternates between
//! a full block and a tail block an interrupted append sealed past the commit
//! therefore misses that memory on every touch of the tail, and AF-43 says
//! the proof runs again each time. The tests in `crate::file` see only what a
//! read returns. Equal answers cannot tell a proof run again from one
//! remembered, and a test that changes a proof input before a touch shows
//! only that a change to THAT input is seen: AF-47's leg changed the sidecar
//! entry, and a proof remembered against the entry's value passed it (AF-48).
//! A memory keyed on some other change to the file would pass a leg that
//! damages the records in the same way.
//!
//! What no memory can fake is the proof's own line. `block::verify_through`
//! writes one `store.block` warning, "tail block sealed past the commit by an
//! interrupted append", every time it admits such a block, and that call is
//! the proof. So this counts that line after every touch: one per touch of the
//! tail after another block, none for a touch of a full block, and none for a
//! second touch of the tail in a row, which the one-block memory still serves
//! for the price of a load.
//!
//! # Why this is its own test binary, and holds one test that emits
//!
//! `telemetry::install` writes a per-process `OnceLock` and refuses a second
//! call. `store`'s unit-test binary spends its install on
//! `store::emits::every_emit_in_this_crate_reaches_the_log_through_its_production_call`,
//! whose exact count would also swallow these lines. Cargo builds every file
//! under `tests/` into its own process, so the install below is a different
//! `OnceLock`, and nothing else in this binary emits: the other two tests
//! only read a document.
//!
//! # What it does not prove
//!
//! That the proof's answer is right: `crate::file` and `crate::unit` hold
//! that. This proves only that it ran, once per touch that needed it.
//!
//! # The row it made false
//!
//! AF-47 measured a memory that keeps the tail block keyed on the block
//! alone, and said in the present tense that its own test was then the only
//! store test that failed. That held at 224b6760 and stopped holding when
//! this file landed, because the count here fails under that memory too.
//! The second test checks that AF-47 scopes the measurement to its commit
//! and names the count, and that AF-48 says AF-47 was corrected.
//!
//! # The rows that named too little
//!
//! AF-43 and AF-47 each say the tail is proved again on every touch, and
//! each named only the test in `crate::file` beside it. Measured at
//! da28ae95, a memory keyed on the entry and the bar file's modification
//! time passes that test, and the count here is the one store test it fails.
//! The third test checks that every row making the claim names the count in
//! its test column.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// The symbol id the month is written and reopened under.
const SYMBOL: u32 = 26_000;

/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;

/// The open of the first one-minute bar of 2024-06-03, in microseconds.
const T0: i64 = 1_717_386_300_000_000;

/// The target and the sentence the proof writes when it admits the tail.
const TARGET: &str = "store.block";
const ADMITTED: &str = "tail block sealed past the commit by an interrupted append";

/// The `index`-th one-minute bar of the session, in paisa.
fn bar(index: i64) -> Bar {
    Bar {
        ts_micros: T0 + index * MINUTE,
        open: 2_345_600 + index,
        high: 2_345_900 + index,
        low: 2_345_100 + index,
        close: 2_345_700 + index,
        volume: 1_000 + index,
        open_interest: OI_NULL,
    }
}

/// The month under test.
fn bars_path() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).expect("2024-06"),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

/// A temporary path no other live process will name.
fn scratch(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "brutex-store-tail-proof-{tag}-{}",
        std::process::id()
    ))
}

/// How many times the proof has admitted a tail block, read off the disk.
fn admissions(dir: &Path, keep_files: u8) -> usize {
    let found = telemetry::tail(
        dir,
        keep_files,
        &telemetry::Query::last(telemetry::MAX_LIMIT).from_target(TARGET),
    );
    assert!(
        found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
        "the log reads whole: {found:?}"
    );
    found
        .records
        .iter()
        .filter(|line| line.message == ADMITTED)
        .inspect(|line| {
            assert_eq!(line.level, telemetry::Level::Warn, "{line:?}");
            assert!(
                line.fields
                    .contains(&("block".to_owned(), telemetry::OwnedValue::Int(1))),
                "the admitted block is block 1: {line:?}"
            );
            assert!(
                line.fields
                    .contains(&("sealed_through".to_owned(), telemetry::OwnedValue::Int(83))),
                "sealed over the 83 records the dead append wrote: {line:?}"
            );
        })
        .count()
}

/// EVERY TOUCH OF THE TAIL AFTER ANOTHER BLOCK RUNS ITS PROOF AGAIN.
///
/// Eighty committed and three lost to a crash between the seal and the
/// commit: block 0 is full, and block 1 is the tail, its entry sealed over 83
/// records. After each read the proof's line is counted. A touch of the tail
/// that follows a touch of block 0 adds exactly one, thirteen times over, so
/// a memory that serves the tail without calling the proof fails here,
/// whatever it is keyed on. A touch of block 0 adds none, and neither does a
/// second touch of the tail in a row, so the one-block memory still spares a
/// walk of the tail its proof.
#[test]
fn every_touch_of_the_tail_after_another_block_runs_its_proof_again() {
    let dir = scratch("log");
    let root = scratch("root");
    let _ignored = fs::remove_dir_all(&dir);
    let _ignored = fs::remove_dir_all(&root);
    let sink = telemetry::install(&telemetry::Config::new(&dir))
        .expect("nothing else in this test binary installs a sink");
    let keep = sink.keep_files();

    // THE CRASH, through the public door: the header region is put back to
    // what it was before the second append committed, so the seal of the
    // lost three is on disk and their commit is not.
    let month = bars_path();
    let path = month.to_path_buf(&root);
    let mut writer = BarFile::open_or_create(&root, month, SYMBOL).expect("the month opens");
    let held: Vec<Bar> = (0..80).map(bar).collect();
    assert!(matches!(
        writer.append(&held),
        Ok(Appended::Committed { n_valid: 80, .. })
    ));
    let region = fs::read(&path).expect("the month reads")[..32_768].to_vec();
    let lost: Vec<Bar> = (80..83).map(bar).collect();
    assert!(matches!(
        writer.append(&lost),
        Ok(Appended::Committed { n_valid: 83, .. })
    ));
    drop(writer);
    let mut bytes = fs::read(&path).expect("the month reads");
    bytes[..32_768].copy_from_slice(&region);
    fs::write(&path, &bytes).expect("the month writes");

    let reader = BarFile::open_existing(&root, month, SYMBOL).expect("the month opens");
    let mut seen = admissions(&dir, keep);
    assert_eq!(seen, 0, "nothing has touched the tail yet");
    let mut touch = |index: i64, proved: usize, why: &str| {
        let at = u64::try_from(index).expect("small");
        assert_eq!(reader.read_record(at), Ok(bar(index)), "record {index}");
        let now = admissions(&dir, keep);
        assert_eq!(now - seen, proved, "record {index}: {why}");
        seen = now;
    };

    touch(0, 0, "a full block is proved without the tail's line");
    touch(73, 1, "the first touch of the tail proves it");
    touch(
        79,
        0,
        "a second touch of the tail in a row is served by the memory",
    );
    for round in 0..6i64 {
        touch(round, 0, "a full block again");
        touch(73 + round, 1, "the tail after block 0 is proved again");
        touch(72 - round, 0, "a full block again");
        touch(79 - round, 1, "and again");
    }
    assert_eq!(
        seen, 13,
        "thirteen touches of the tail after block 0, each proved"
    );

    drop(reader);
    let _ignored = fs::remove_dir_all(&dir);
    let _ignored = fs::remove_dir_all(&root);
}

// ===========================================================================
// The rows: what AF-43, AF-47 and AF-48 say the test above fails under
// ===========================================================================

/// The invariants register. Read at compile time so a rename fails the build
/// rather than skipping the check.
const INVARIANTS: &str = include_str!("../../../docs/04-invariants.md");

/// The row of the register whose id is `id`, one line of the table.
fn row(id: &str) -> &'static str {
    let lead = format!("\n| {id} | ");
    let from = INVARIANTS
        .find(&lead)
        .unwrap_or_else(|| panic!("docs/04-invariants.md has no row {id}"))
        + 1;
    let rest = &INVARIANTS[from..];
    rest.find('\n').map_or(rest, |to| &rest[..to])
}

/// A failure naming the row and the claim, unless the row makes it.
#[track_caller]
fn says(text: &str, id: &str, claim: &str) {
    assert!(text.contains(claim), "{id} does not say `{claim}`");
}

/// The test column of the row whose id is `id`: the cell before the status.
fn tests_of(id: &str) -> &'static str {
    let body = row(id)
        .strip_suffix(" |")
        .unwrap_or_else(|| panic!("{id} does not close its row with ` |`"));
    let (rest, _status) = body
        .rsplit_once(" | ")
        .unwrap_or_else(|| panic!("{id} has no status cell"));
    let (_text, tests) = rest
        .rsplit_once(" | ")
        .unwrap_or_else(|| panic!("{id} has no test column"));
    tests
}

/// The count above, as a test column names it.
const COUNT: &str =
    "`store::tail_proof::every_touch_of_the_tail_after_another_block_runs_its_proof_again`";

/// The test in `crate::file` that checks the answers and the damaged inputs.
const FILE_TEST: &str =
    "`store::file::reads_of_a_month_an_interrupted_append_left_are_idempotent_and_write_nothing`";

/// **AF-47 DOES NOT CALL ITS TEST THE ONLY ONE ITS SECOND CACHE FAILS.**
///
/// AF-47 measured two caches at 224b6760, and under the second, a memory that
/// keeps the tail block keyed on the block alone, its own test was then the
/// only store test that failed. The row said so in the present tense. The
/// test above landed one commit later and fails under that memory too, so the
/// sentence was false from then on, while AF-48 said "both fail" of the same
/// memory. Neither row is on main, so AF-47 is corrected in place: it scopes
/// the measurement to the commit that took it and names the test above, and
/// AF-48 no longer says AF-47 is unedited. AF-48 scopes its own count the same
/// way: a memory keyed on the entry's value passed every store test at
/// 224b6760, 224 of them, and fails two since the test above landed. This
/// holds the two rows to each other and to the test above.
#[test]
fn af_47_and_af_48_agree_on_which_tests_a_memory_of_the_tail_block_fails() {
    let (af_47, af_48) = (row("AF-47"), row("AF-48"));
    says(af_48, "AF-48", "Keyed on the tail block alone: both fail.");
    says(af_48, "AF-48", "at 224b6760, every store test passed (224)");
    says(
        af_47,
        "AF-47",
        "At 224b6760, which added this leg, it was the only store test the \
         second cache failed.",
    );
    says(
        af_47,
        "AF-47",
        "`store::tail_proof::every_touch_of_the_tail_after_another_block_runs_its_proof_again` \
         (AF-48) fails under both caches",
    );
    assert!(
        !af_47.contains("it is the only store test that fails"),
        "AF-47 again calls its test the only store test the second cache \
         fails, and the test above fails under it too"
    );
    says(af_48, "AF-48", "AF-47 is corrected in place");
    assert!(
        !af_48.contains("AF-47 is not edited"),
        "AF-48 says AF-47 is not edited, and AF-47 was corrected in place"
    );
}

/// **EVERY ROW THAT SAYS THE TAIL IS PROVED AGAIN NAMES THE COUNT BESIDE IT.**
///
/// AF-43, AF-47 and AF-48 each say that reads alternating between a full
/// block and the tail run the proof again on every touch of the tail. The
/// test in `crate::file` checks the answers and refuses damaged inputs, and a
/// memory keyed on the tail's entry and the bar file's modification time
/// passes it: measured at da28ae95, the count above was then the one store
/// test that failed. A row whose test column named only the test in
/// `crate::file` claimed more than its tests check. So each of the three
/// names the count beside it, and AF-47, which is the row that corrects
/// AF-43, says why and no longer says AF-43 is unedited.
#[test]
fn every_row_that_says_the_tail_is_proved_again_names_the_count_in_its_test_column() {
    for id in ["AF-43", "AF-47", "AF-48"] {
        let tests = tests_of(id);
        assert!(
            tests.contains(COUNT),
            "{id}'s test column does not name the count: {tests}"
        );
        assert!(
            tests.contains(FILE_TEST),
            "{id}'s test column no longer names the test in `crate::file`: {tests}"
        );
    }
    let af_47 = row("AF-47");
    says(
        af_47,
        "AF-47",
        "a memory keyed on the entry and the bar file's modification time passes it",
    );
    assert!(
        !af_47.contains("AF-43 is not edited."),
        "AF-47 says AF-43 is not edited, and AF-43's test column names the count"
    );
}
