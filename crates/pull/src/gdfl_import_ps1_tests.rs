#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]
//! BASE-COV-PULL-PS1 (D-7335): the arms of the importer's top level no test
//! reached at tip 2e7a0b6d3b56 (Route B), each driven by the input that
//! reaches it: a probe of a stocks day whose old-symbol stem the stored
//! scope refuses, beside a member two files hold; a lineage ledger that will
//! not take a day's lines; a warm-up whose worker threads cannot start (in a
//! child process where the host refuses every thread); and a lineage rescan
//! whose first day has a session before it in the source.
//!
//! Every option name (`ZQALPHA...`), share (`ZQPROBE`), premium and quantity
//! is invented at run time; no vendor row is quoted. The one real symbol,
//! `TATAMOTORS`, is the compiled symbol-change row's own
//! (`brutex_core::alias::SYMBOL_CHANGES`, `TATAMOTORS -> TMPV` from
//! 2025-10-24): `CmScope::filing` refuses a stem only by the compiled
//! `alias::LINEAGE`, so no invented name can be refused that way.

use std::sync::Arc;

use super::*;
use crate::gdfl_cm::{CmScope, DayListing};
use crate::gdfl_fixtures::{bts, csv, put, row, scratch};
use crate::gdfl_nfo::NfoTickStore;

fn day(year: u16, month: u8, date: u8) -> Day {
    Day::new(year, month, date).unwrap()
}

/// 09:20:00 IST.
const NINE_TWENTY: u32 = 9 * 3_600 + 20 * 60;

/// Two invented calls of one invented underlying, expiring Thu 28 Mar 2024.
const CALLS: [&str; 2] = ["ZQALPHA28MAR24500CE", "ZQALPHA28MAR24520CE"];

/// The lot every [`CALLS`] file's rows show: the greatest common divisor of
/// their traded quantities, 50 and 75.
const LOT: u64 = 25;

/// An options tick store at `root` holding `on` alone: one file per
/// [`CALLS`], two traded rows each (quantities 50 and 75, so a lot of
/// [`LOT`]) at premiums no contract of that name ever printed.
fn options_world(root: &Path, on: Day) -> NfoTickStore {
    let nfo = NfoTickStore::new(root);
    let folder = crate::gdfl_nfo::day_folder_name(on);
    let files: Vec<(String, Vec<u8>)> = CALLS
        .iter()
        .map(|ticker| {
            let stem = format!("{ticker}.NFO");
            let rows = [
                row(&stem, on, NINE_TWENTY, "7.77", 50, 1_000),
                row(&stem, on, NINE_TWENTY + 60, "7.78", 75, 1_050),
            ];
            (format!("{folder}\\Options\\{stem}.csv"), csv(&rows))
        })
        .collect();
    let entries: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
        .collect();
    let at = nfo.day_path(on);
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(&at, bts(&entries)).unwrap();
    nfo
}

// ── cm_present: a refused stem and a twin member (2655, 2657:21 False) ─────

/// A probe of a stocks day (WP-12f, D-4878) asks an old symbol's stem, which
/// the stored scope refuses on a day after its change, by the key that
/// refusal names: `TATAMOTORS` is filed as `TMPV` from 24 Oct 2025, so a run
/// that wants every name, or `TMPV`, is owed the member, and one that names
/// `TATAMOTORS` alone, or a lending filter that names neither, is not. A
/// member two files hold (a case twin) is answered once, in listing order.
#[test]
fn a_probe_asks_a_refused_stem_by_the_key_its_refusal_names_and_each_member_once() {
    let on = day(2026, 1, 5);
    let scope = Arc::new(CmScope::stored(["ZQPROBE"]).unwrap());
    let mut listing = DayListing::<()>::scoped(CmKind::Stocks, on, scope);
    let folder = listing.folder().to_owned();
    for name in ["TATAMOTORS.NSE.csv", "ZQPROBE.NSE.csv", "ZQPROBE.NSE.CSV"] {
        listing.push(&format!("{folder}/{name}"), 1, 0, ());
    }
    // The premise: the old stem is refused that day, and the refusal files
    // it under the newest symbol its change reaches.
    let refused = listing.filing("TATAMOTORS.NSE").unwrap_err();
    assert_eq!(refused.filed_as(), Some("TMPV"));
    let (old, invented) = (
        "NSE-CASH-TATAMOTORS".to_owned(),
        "NSE-CASH-ZQPROBE".to_owned(),
    );
    let wanted: HashSet<String> = [old.clone(), invented.clone()].into_iter().collect();
    let store = Path::new("a-probe-reads-no-store");
    let run = |only| Run {
        kind: ImportKind::Stocks,
        from: on,
        to: on,
        only,
        store_root: store,
    };
    let (by_key, by_stem) = (["TMPV".to_owned()], ["TATAMOTORS".to_owned()]);
    assert_eq!(
        cm_present((&run(&[]), None), &listing, &wanted),
        [old.clone(), invented.clone()],
        "every name: both members, the twin once"
    );
    assert_eq!(
        cm_present((&run(&by_key), None), &listing, &wanted),
        [old],
        "the key the refusal names"
    );
    assert!(
        cm_present((&run(&by_stem), None), &listing, &wanted).is_empty(),
        "the stem's own spelling is not what a refused stem is asked by"
    );
    let lender: HashSet<String> = std::iter::once("ZQPROBE".to_owned()).collect();
    assert_eq!(
        cm_present((&run(&[]), Some(&lender)), &listing, &wanted),
        [invented],
        "a close lent by a filter of other names owes no member filed as TMPV"
    );
}

// ── fold_lineage: a ledger that refuses a day's lines (3127-3131) ──────────

/// A lineage ledger that will not take a day's lines fails that day by name
/// (D-3331): the day's extras carry one failure at stage `journal`, code
/// `lineage`, naming the ledger in the host's words, and count nothing
/// written. The ledger is a link to a file in a folder that is not there: it
/// reads as empty when the run loads it, and its append is refused (ENOENT),
/// as root too. The run's first fold has lines to write: the alias table.
#[test]
fn a_lineage_ledger_that_will_not_take_a_days_lines_fails_the_day_by_name() {
    let root = scratch("ps1-lineage-unwritable");
    let store = root.join("S");
    let ledger = crate::gdfl_lineage::lineage_path(&store);
    std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(root.join("gone").join("gdfl.lineage"), &ledger).unwrap();
    // The premises: the first fold of a run writes the alias table, so the
    // day has lines; and the host refuses to open the ledger to append.
    assert!(!crate::expiry_alias::alias_records().unwrap().is_empty());
    let host = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ledger)
        .unwrap_err();
    let on = day(2024, 3, 5);
    let mut lineage = lineage::LineageRun::load(&store, on).unwrap();
    let source = NfoTickStore::new(&root.join("no-source"));
    let listing = crate::gdfl_nfo::NfoDay::new(on, 0);
    let run = Run {
        kind: ImportKind::Options,
        from: on,
        to: on,
        only: &[],
        store_root: &store,
    };
    let mut extras = DayExtras::default();
    fold_lineage(
        &source,
        &run,
        (on, &listing),
        &mut lineage,
        &[],
        &mut extras,
    );
    let [staged] = extras.failures.as_slice() else {
        panic!("one failure, not {:?}", extras.failures);
    };
    assert_eq!(staged.cause, Cause::open(Stage::Journal, "lineage"));
    assert_eq!(
        staged.failure,
        Failure {
            instrument: format!("{on} lineage"),
            why: format!("the lineage ledger: {}: {host}", ledger.display()),
            month: on.year_month().ok(),
            day: Some(on),
        }
    );
    assert_eq!(extras.lineage, lineage::Written::default());
    assert!(!root.join("gone").exists(), "nothing was made for the link");
    std::fs::remove_dir_all(root).unwrap();
}

// ── warm_files_on: no worker thread starts (3266-3274) ─────────────────────

/// A stack size no 64-bit host can map, handed to every thread start through
/// `RUST_MIN_STACK`: 2^60 bytes, past every user address space (x86-64's is
/// at most 2^57 bytes with five-level paging, arm64's 2^52), so glibc's
/// `pthread_create` cannot map the stack and answers EAGAIN, which `std`
/// reads as `WouldBlock`.
#[cfg(target_os = "linux")]
const UNMAPPABLE_STACK: &str = "1152921504606846976";

/// Runs `body` in a child of this test binary in which no thread can start:
/// its `RUST_MIN_STACK` is [`UNMAPPABLE_STACK`], the size every thread start
/// without one of its own asks for. The harness, refused a thread for the
/// test (`WouldBlock`), runs it on the main thread.
/// `crate::support::where_permission_binds` gives its child no variable of
/// its own; this is that helper with one variable more and no change of
/// user (no permission bit is involved). Like it, it asserts that the child
/// passed and ran exactly one test, and a coverage run's `LLVM_PROFILE_FILE`
/// reaches the child.
#[cfg(target_os = "linux")]
fn where_no_thread_starts(test: &str, body: impl FnOnce()) {
    const CHILD: &str = "BRUTEX_PS1_NO_THREAD_CHILD";
    if std::env::var_os(CHILD).is_some() {
        body();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().expect("this binary"))
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .env("RUST_MIN_STACK", UNMAPPABLE_STACK)
        .output()
        .expect("the child test process starts");
    let (stdout, stderr) = (
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    // Formatted unconditionally, and both checks taken without a short
    // circuit, so no line or branch side is reached only when the child fails.
    let said = format!("child {test} failed or ran no test:\n{stdout}\n{stderr}");
    let ran = (output.status.success(), stdout.contains("1 passed"));
    assert_eq!(ran, (true, true), "{said}");
}

/// One file a warm-up read, as the lineage is handed it: its ticker, its
/// contract-day and its place in the listing.
#[cfg(target_os = "linux")]
type ContractShape = (
    crate::gdfl_nfo::OptionTicker,
    crate::gdfl_lineage::ContractDay,
    usize,
);

/// [`ContractShape`] of each of `contracts`, in order.
#[cfg(target_os = "linux")]
fn contract_days(contracts: &[lineage::DayContract]) -> Vec<ContractShape> {
    contracts
        .iter()
        .map(|contract| (contract.ticker.clone(), contract.day, contract.file))
        .collect()
}

/// A warm-up whose worker threads cannot start (WP-16) reads its files on
/// the calling thread instead and answers exactly what one thread answers:
/// both files' contract-days, in listing order, each with the lot its rows
/// show. Run in a child process where the host refuses every thread start,
/// which the body checks first. Linux only: the refusal is glibc's
/// `pthread_create`, reasoned for x86-64 and arm64 Linux.
#[cfg(target_os = "linux")]
#[test]
fn a_warm_up_whose_workers_cannot_start_reads_its_files_on_this_thread() {
    where_no_thread_starts(
        "gdfl_import::ps1_tests::a_warm_up_whose_workers_cannot_start_reads_its_files_on_this_thread",
        a_warm_up_whose_workers_cannot_start_reads_its_files_on_this_thread_body,
    );
}

/// The test above, in the child where no thread starts.
#[cfg(target_os = "linux")]
fn a_warm_up_whose_workers_cannot_start_reads_its_files_on_this_thread_body() {
    let refused = std::thread::Builder::new().spawn(|| {}).err();
    assert_eq!(
        refused.as_ref().map(std::io::Error::kind),
        Some(std::io::ErrorKind::WouldBlock),
        "the premise: this process cannot start a thread"
    );
    let root = scratch("ps1-warm-up-no-thread");
    let monday = day(2024, 3, 4);
    let nfo = options_world(&root, monday);
    let listing = nfo.day(monday).unwrap().unwrap();
    let wanted: Vec<_> = listing.entries().iter().enumerate().collect();
    let alone = warm_files(&nfo, &listing, &wanted);
    let on_workers = warm_files_on(&nfo, 4, &listing, &wanted);
    assert_eq!(contract_days(&on_workers), contract_days(&alone));
    let inferred = crate::gdfl_lineage::LotEvidence::Inferred(LOT);
    assert_eq!(
        on_workers
            .iter()
            .map(|contract| (contract.file, contract.day.lot))
            .collect::<Vec<_>>(),
        [(0, inferred), (1, inferred)],
        "both files were read, each with its lot"
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ── nfo_day_with: a lineage rescan's warm-up (3423-3424) ───────────────────

/// A lineage rescan (`cli gdfl-dayflags options --lineage`) folds in the
/// sessions before its first day that the source holds (D-3331), their files
/// read one after another on the rescan's own thread: the session before is
/// read though the rescanned day itself is gone from the source, and its two
/// calls' lots are recorded from that session, beside the alias table. The
/// day is named missing, and nothing is filed.
#[test]
fn a_lineage_rescan_folds_in_the_session_before_its_first_day() {
    let root = scratch("ps1-rescan-warm-up");
    let (monday, tuesday) = (day(2024, 3, 4), day(2024, 3, 5));
    let nfo = options_world(&root.join("ts"), monday);
    let store = root.join("S");
    put(
        &store,
        "imports/gdfl.journal",
        format!("done options {tuesday} *\n").as_bytes(),
    );
    let done = rescan_nfo(&nfo, (tuesday, tuesday), &store, true).unwrap();
    assert_eq!(
        done,
        Rescan {
            days_missing: vec![tuesday],
            lineage_lines: crate::expiry_alias::alias_records().unwrap().len(),
            lot_lines: CALLS.len(),
            ..Rescan::default()
        }
    );
    let lots: Vec<String> = CALLS
        .iter()
        .map(|ticker| {
            let decoded = decode_ticker(ticker, monday).unwrap();
            crate::gdfl_lineage::LotRecord {
                underlying: decoded.underlying,
                contract: decoded.contract,
                from: monday,
                lot: LOT,
            }
            .line()
        })
        .collect();
    let written = std::fs::read_to_string(crate::gdfl_lineage::lots_path(&store)).unwrap();
    assert_eq!(written.lines().collect::<Vec<_>>(), lots);
    assert!(!store.join("bars").exists(), "a rescan files nothing");
    std::fs::remove_dir_all(root).unwrap();
}
