//! IA-2's tests of the importer: the stop's close rules (IA-GR-05, D-4853),
//! the close's clock time and the run's end line (DB-A50), one cause per
//! refusal (IA-GR-16), and the journal append lock with the ordinal counted
//! under it (D-4905, FM-093, FM-094). Every tick is invented: prices that
//! cannot be mistaken for a vendor's.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::*;
use crate::gdfl_run::stop::StopControl;
use crate::session::Day;

const NINE_FIFTEEN: u32 = 9 * 3_600 + 15 * 60;

fn day(d: u8) -> Day {
    Day::new(2024, 4, d).unwrap()
}

fn scratch(tag: &str) -> PathBuf {
    crate::gdfl_fixtures::scratch(&format!("ia2-{tag}"))
}

/// An invented index file: four seconds of made-up prices.
fn file(symbol: &str) -> TickFile {
    TickFile {
        symbol: symbol.to_owned(),
        contract: None,
        name: format!("{symbol}.invented"),
        ticks: [0, 61, 122, 183]
            .into_iter()
            .map(|at| Tick {
                sod: NINE_FIFTEEN + at,
                ltp: 777_700 + i64::from(at),
                ltq: 0,
                oi: None,
            })
            .collect(),
    }
}

fn run_over(store: &Path, to: Day) -> Run<'_> {
    Run {
        kind: ImportKind::Indices,
        from: day(1),
        to,
        only: &[],
        store_root: store,
    }
}

/// Every NIFTY and BANKNIFTY file of every day asked.
fn both(_on: Day, sink: &mut dyn FnMut(TickFile)) -> DayRead {
    sink(file("NIFTY"));
    sink(file("BANKNIFTY"));
    DayRead {
        held: true,
        ..DayRead::default()
    }
}

fn journal(store: &Path) -> String {
    std::fs::read_to_string(journal_path(store)).unwrap_or_default()
}

fn lines_of(journal: &str, day: Day) -> Vec<&str> {
    let key = format!("indices {day} *");
    journal.lines().filter(|line| line.contains(&key)).collect()
}

/// **IA-GR-05 (D-4853): A STOP CLOSES THE DAY BY WHAT WAS WRITTEN, AND A
/// RESUME CONVERGES.** On NIFTY and BANKNIFTY, one worker, three days:
///
/// - a stop after the second day's first file closes that day `incomplete
///   stage=run reason=stopped` (its `begin` was written), reads no third day,
///   and ends the run `Stopped` naming the day and who asked;
/// - a resume reads the stopped day again and closes it `done`, and the
///   third; the store answers what it holds already (D-2803);
/// - a stop before a day's first file writes no line for that day at all;
/// - a stop after a day's last file lets the day close `done`, and the run
///   stops before the next.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "four stop cases, each read back from one journal"
)]
fn a_stop_closes_the_day_by_what_was_written_and_a_resume_converges() {
    // ── Mid-day, after `begin` ─────────────────────────────────────────────
    let store = scratch("stop-mid");
    // Checks: 1 the run's start, 2 day 1's top, 3-4 its two files, 5 after
    // it, 6 day 2's top, 7 its first file, 8 its second: the stop is asked
    // there, so day 2's first file is filed and its second is not taken.
    let stop = StopControl::tripping_at(store.join("imports/gdfl.stop"), 1_790_000_101, 8);
    let options = RunOptions {
        run: Some(1_790_000_101),
        stop: Some(&stop),
        workers: Workers::new(1).unwrap(),
        ..RunOptions::default()
    };
    let mut read = Vec::new();
    let got = drive_from(&run_over(&store, day(3)), options, |on, sink| {
        read.push(on);
        both(on, sink)
    })
    .unwrap_err();
    assert_eq!(read, vec![day(1), day(2)], "the third day is never read");
    let ImportRefusal::Stopped {
        day: at, by, done, ..
    } = &got
    else {
        panic!("{got:?}");
    };
    assert_eq!((*at, by.as_str()), (Some(day(2)), "operator"));
    assert_eq!(done.days_imported, 1, "day 2 is not imported: it is open");
    assert_eq!(got.code(), "stopped");
    assert_eq!(import_cause(&got), Cause::open(Stage::Run, "stopped"));
    let text = journal(&store);
    let first = lines_of(&text, day(1));
    assert!(
        first
            .last()
            .unwrap()
            .starts_with("done indices 2024-04-01 * "),
        "{text}"
    );
    let second = lines_of(&text, day(2));
    assert_eq!(
        second.first(),
        Some(&"begin indices 2024-04-02 *"),
        "{text}"
    );
    let closed = second.last().unwrap();
    assert!(
        closed.starts_with("incomplete indices 2024-04-02 * ")
            && closed.contains(" stage=run reason=stopped at=")
            && closed.ends_with(" run=1790000101"),
        "{closed}"
    );
    assert!(lines_of(&text, day(3)).is_empty(), "{text}");
    // DB-A50: the run's end, one more `run` line, the journal's last.
    let last = text.lines().last().unwrap();
    assert!(
        last.starts_with("run 1790000101 ended=")
            && last.contains(" outcome=stopped ")
            && last.ends_with(&format!(" definition={BAR_DEFINITION}")),
        "{last}"
    );
    // The journal still loads: the end line is a `run` line every reader
    // reads, and the stopped day is open for the resume.
    let loaded = Journal::load(journal_path(&store)).unwrap();
    assert!(!loaded.done.contains("indices 2024-04-02 *"));

    // ── The resume converges ──────────────────────────────────────────────
    let options = RunOptions {
        run: Some(1_790_000_102),
        ..RunOptions::default()
    };
    let report = drive_from(&run_over(&store, day(3)), options, both).unwrap();
    assert_eq!(
        (report.days_skipped, report.days_imported),
        (1, 2),
        "day 1 is skipped, day 2 is read again, day 3 is new"
    );
    let text = journal(&store);
    assert!(
        lines_of(&text, day(2))
            .last()
            .unwrap()
            .starts_with("done indices 2024-04-02 * "),
        "{text}"
    );
    let again = drive_from(
        &run_over(&store, day(3)),
        RunOptions {
            run: Some(1_790_000_103),
            ..RunOptions::default()
        },
        both,
    )
    .unwrap();
    assert_eq!(again.days_imported, 0, "converged: nothing left to read");

    // ── Before the day's first file: no line at all ───────────────────────
    let store = scratch("stop-before");
    // Checks: 1 start, 2 day 1's top, 3-4 its files, 5 after it, 6 day 2's
    // top, 7 its first file: asked there, so nothing of day 2 is taken.
    let stop = StopControl::tripping_at(store.join("imports/gdfl.stop"), 1_790_000_201, 7);
    let options = RunOptions {
        run: Some(1_790_000_201),
        stop: Some(&stop),
        workers: Workers::new(1).unwrap(),
        ..RunOptions::default()
    };
    let got = drive_from(&run_over(&store, day(3)), options, both).unwrap_err();
    assert!(
        matches!(&got, ImportRefusal::Stopped { day: Some(d), .. } if *d == day(2)),
        "{got:?}"
    );
    let text = journal(&store);
    assert!(
        lines_of(&text, day(2)).is_empty(),
        "no begin, so no close: {text}"
    );

    // ── After the day's last file: the day closes done, the run stops ─────
    let store = scratch("stop-after");
    // Check 5 is the one after day 1.
    let stop = StopControl::tripping_at(store.join("imports/gdfl.stop"), 1_790_000_301, 5);
    let options = RunOptions {
        run: Some(1_790_000_301),
        stop: Some(&stop),
        workers: Workers::new(1).unwrap(),
        ..RunOptions::default()
    };
    let got = drive_from(&run_over(&store, day(3)), options, both).unwrap_err();
    assert!(
        matches!(&got, ImportRefusal::Stopped { day: Some(d), done, .. } if *d == day(1) && done.days_imported == 1),
        "{got:?}"
    );
    let text = journal(&store);
    assert!(
        lines_of(&text, day(1)).last().unwrap().starts_with("done "),
        "{text}"
    );
    assert!(lines_of(&text, day(2)).is_empty(), "{text}");
    assert!(stop.checks() >= 4, "the stop was asked at every boundary");
}

/// **A STOP FILE OF THIS RUN STOPS IT; ONE OF ANOTHER RUN DOES NOT.** The
/// file is the one D-4827 names, written as `gdfl-stop` writes it; who asked
/// and when travel into the refusal (D-4893).
#[test]
fn a_stop_file_stops_only_the_run_it_names() {
    let store = scratch("stop-file");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    let path = store.join("imports/gdfl.stop");
    let other = crate::gdfl_run::stop::StopRequest {
        run: 42,
        requested: 1_790_000_000,
        by: "cli".to_owned(),
    };
    crate::gdfl_run::stop::write_stop(&path, &other).unwrap();
    let stop = StopControl::new(path.clone(), 1_790_000_401);
    assert!(!stop.requested(), "another run's stop file");
    let mine = crate::gdfl_run::stop::StopRequest {
        run: 1_790_000_401,
        requested: 1_790_000_500,
        by: "api".to_owned(),
    };
    let _ = std::fs::remove_file(&path);
    crate::gdfl_run::stop::write_stop(&path, &mine).unwrap();
    let options = RunOptions {
        run: Some(1_790_000_401),
        stop: Some(&stop),
        workers: Workers::new(1).unwrap(),
        ..RunOptions::default()
    };
    let got = drive_from(&run_over(&store, day(2)), options, both).unwrap_err();
    let ImportRefusal::Stopped {
        day, by, requested, ..
    } = &got
    else {
        panic!("{got:?}");
    };
    assert_eq!(
        (*day, by.as_str(), *requested),
        (None, "api", 1_790_000_500)
    );
    assert!(
        journal(&store).is_empty(),
        "a run stopped before any day writes no line"
    );
}

/// **DB-A50: EVERY CLOSE OF A LEDGER RUN SAYS WHEN, AND THE RUN SAYS IT
/// ENDED; A BARE READER'S JOURNAL STAYS A FUNCTION OF ITS INPUTS.** `at=` is
/// written before `run=`, which stays each line's last field; a refused close
/// identical but for `at=` and `run=` is not written again (D-2803).
#[test]
fn a_ledger_runs_closes_carry_a_clock_time_and_its_end_a_line() {
    let store = scratch("at");
    let options = RunOptions {
        run: Some(1_790_000_601),
        ..RunOptions::default()
    };
    drive_from(&run_over(&store, day(1)), options, both).unwrap();
    let text = journal(&store);
    let done = lines_of(&text, day(1));
    let close = done.last().unwrap();
    let at: u64 = close
        .split(' ')
        .find_map(|f| f.strip_prefix("at="))
        .unwrap()
        .parse()
        .unwrap();
    assert!(at > 1_700_000_000, "{close}");
    assert!(close.ends_with(" run=1790000601"), "{close}");
    assert!(
        text.lines().last().unwrap().contains(" outcome=finished "),
        "{text}"
    );

    let bare = scratch("at-bare");
    drive_from(&run_over(&bare, day(1)), RunOptions::default(), both).unwrap();
    let text = journal(&bare);
    assert!(!text.contains(" at="), "{text}");
    assert!(!text.contains(" ended="), "{text}");
}

/// **IA-GR-16: EVERY IMPORT REFUSAL HAS ONE PINNED CAUSE.** The table below
/// is the design's §5.8 rows for `ImportRefusal`; `import_cause` is
/// exhaustive (a new variant fails to compile until it is decided), no import
/// refusal closes a day, and a stop is the run's.
#[test]
fn every_import_refusal_has_one_pinned_cause() {
    let report = || Box::new(Report::default());
    let pinned: Vec<(ImportRefusal, Stage)> = vec![
        (
            ImportRefusal::KindUnknown {
                word: String::new(),
            },
            Stage::Run,
        ),
        (
            ImportRefusal::RangeBackwards {
                from: day(2),
                to: day(1),
            },
            Stage::Run,
        ),
        (ImportRefusal::VolumeOverflow { sod: 0 }, Stage::File),
        (ImportRefusal::Fold { why: String::new() }, Stage::File),
        (
            ImportRefusal::Journal { why: String::new() },
            Stage::Journal,
        ),
        (
            ImportRefusal::FilterName {
                name: String::new(),
            },
            Stage::File,
        ),
        (ImportRefusal::StampPastTheDay { sod: 0 }, Stage::File),
        (ImportRefusal::Filing { why: String::new() }, Stage::File),
        (ImportRefusal::DayNotHeld { day: day(1) }, Stage::Vendor),
        (
            ImportRefusal::Provenance { why: String::new() },
            Stage::File,
        ),
        (
            ImportRefusal::DayLedger { why: String::new() },
            Stage::Journal,
        ),
        (
            ImportRefusal::Lineage { why: String::new() },
            Stage::Journal,
        ),
        (
            ImportRefusal::Durability {
                kind: ImportKind::Indices,
                day: day(1),
                why: String::new(),
                done: report(),
            },
            Stage::Run,
        ),
        (
            ImportRefusal::FilterRenamed {
                name: String::new(),
                filed: String::new(),
            },
            Stage::Run,
        ),
        (
            ImportRefusal::UnderlyingSplit { held: Vec::new() },
            Stage::Run,
        ),
        (
            ImportRefusal::Stopped {
                kind: ImportKind::Indices,
                day: None,
                by: "cli".to_owned(),
                requested: 0,
                done: report(),
            },
            Stage::Run,
        ),
    ];
    for (refusal, stage) in &pinned {
        let cause = import_cause(refusal);
        assert_eq!(
            (cause.stage, cause.code, cause.closes),
            (*stage, refusal.code(), false),
            "{refusal:?}"
        );
    }
    // A census refusal's cause is the census's.
    let census = ImportRefusal::CensusRefused {
        kind: ImportKind::Indices,
        day: day(1),
        room: Err(String::new()),
        why: String::new(),
        done: report(),
    };
    assert_eq!(import_cause(&census).stage, Stage::Census);
    // The pin holds every stage's word, `run` added last.
    assert_eq!(Stage::ALL.last().map(|s| s.as_str()), Some("run"));
}

/// **FM-093, D-4905: A JOURNAL ORDINAL IS ITS OWN LINE UNDER TWO WRITERS.**
/// Two journals of one store (two kinds' runs) each append 300 `begin` lines
/// at once; every ordinal `append` answers names the line that writer wrote,
/// and no line is torn or glued to another.
#[test]
fn a_journal_ordinal_is_its_own_begin_line_under_two_writers() {
    let store = scratch("ordinal");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    let path = journal_path(&store);
    let written: Vec<Vec<(u64, String)>> = std::thread::scope(|scope| {
        let handles: Vec<_> = ["indices", "stocks"]
            .into_iter()
            .map(|kind| {
                let path = path.clone();
                scope.spawn(move || {
                    let journal = Journal::load(path).unwrap();
                    (0..300_u32)
                        .map(|i| {
                            let line = format!("begin {kind} 2024-04-01 X{i}");
                            (journal.append(&line).unwrap(), line)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 600, "no line lost, torn or glued");
    for (ordinal, line) in written.iter().flatten() {
        assert_eq!(
            lines[usize::try_from(*ordinal).unwrap() - 1],
            line,
            "ordinal {ordinal}"
        );
    }
}

/// **FM-094: THE APPEND COST IS FLAT WITH A SECOND WRITER.** Timed, not
/// asserted (run with `--ignored --nocapture`): p50, p99 and max of one
/// append while a second journal appends between every two of them, on a
/// journal of 1,000 and of 20,000 lines. Before IA-2 each such append re-read
/// the whole file; now it reads only the other writer's bytes.
#[test]
#[ignore = "a timing, printed for docs/06-limits.md and the package report"]
fn journal_append_cost_is_flat_with_a_second_writer() {
    for size in [1_000_u32, 20_000] {
        let store = scratch(&format!("flat-{size}"));
        std::fs::create_dir_all(store.join("imports")).unwrap();
        let path = journal_path(&store);
        let mut seed = String::new();
        for i in 0..size {
            let _ = writeln!(seed, "begin indices 2024-04-01 S{i}");
        }
        std::fs::write(&path, seed).unwrap();
        let mine = Journal::load(path.clone()).unwrap();
        let other = Journal::load(path).unwrap();
        let mut took = Vec::with_capacity(300);
        for i in 0..300_u32 {
            other
                .append(&format!("begin stocks 2024-04-01 O{i}"))
                .unwrap();
            let began = std::time::Instant::now();
            mine.append(&format!("begin indices 2024-04-01 M{i}"))
                .unwrap();
            took.push(began.elapsed());
        }
        took.sort_unstable();
        println!(
            "FM-094 journal of {size} lines, a second writer between appends: n=300 p50={:?} \
             p99={:?} max={:?}",
            took[150], took[296], took[299]
        );
    }
}

/// **ONE VERB TABLE (design §5.4).** The journal's verbs with their space
/// and the words `census_store` reads by are one list: a verb added to one
/// and not the other fails here.
#[test]
fn the_journal_has_one_verb_table() {
    let spaced: Vec<String> = VERB_WORDS.iter().map(|w| format!("{w} ")).collect();
    assert_eq!(super::journal::VERBS.to_vec(), spaced);
}

/// **FM-128: A NUL TAIL AFTER A POWER CUT IS CLOSED AND NAMED, NEVER READ AS
/// DATA.** 37 NUL bytes after the last line (and the start of a line followed
/// by NULs) are a torn append: the run goes on, the fragment is closed with
/// the torn mark, and every later load reads past it. A NUL inside a line is
/// still foreign.
#[test]
fn a_nul_tail_after_a_power_cut_is_closed_and_named() {
    assert!(super::journal::torn_fragment(&"\0".repeat(37)));
    assert!(super::journal::torn_fragment("begi\0\0\0"));
    assert!(super::journal::torn_fragment("\0\0 (torn)"));
    assert!(!super::journal::torn_fragment("do\0ne indices"));
    assert!(!super::journal::torn_fragment("xyz\0\0"));
    let store = scratch("nul-tail");
    drive_from(&run_over(&store, day(1)), RunOptions::default(), both).unwrap();
    let path = journal_path(&store);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.extend_from_slice(&[0_u8; 37]);
    std::fs::write(&path, &bytes).unwrap();
    let report = drive_from(&run_over(&store, day(2)), RunOptions::default(), both).unwrap();
    assert_eq!((report.days_skipped, report.days_imported), (1, 1));
    let text = journal(&store);
    assert!(
        text.contains(&format!("{} (torn)\n", "\0".repeat(37))),
        "{text:?}"
    );
    assert!(
        Journal::load(path).is_ok(),
        "every later load reads past it"
    );
}

/// **IA-2 (design §5.4 "What a reader keeps of each line"; D-4851, D-4882,
/// round 6 SF-6): A READER KEEPS EACH KEY'S HOLE REASON, ITS FIRST LANDING
/// AND ITS LAST `done`.** Per key, in O(1) a line: the `reason=` of the last
/// `incomplete` line; the definition and `run=` of the first close whose
/// `seconds=` is above zero (never replaced); the definition and `run=` of
/// the last `done` close (a `refused` or `recounted` line leaves it). A
/// close written before D-3317 carries no `run=` and is kept as such. The
/// incremental reader keeps them as the whole load does. Every line is
/// invented.
#[test]
fn the_journal_keeps_each_keys_hole_reason_first_landing_and_last_done() {
    let store = scratch("history");
    let path = journal_path(&store);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let key = "options 2024-05-14 NIFTY";
    let old = "indices 2019-01-02 *";
    let mut text = String::new();
    for line in [
        "run 1790000100 source=unrecorded commit=unrecorded started=1790000100 definition=3"
            .to_owned(),
        format!("begin {key}"),
        format!(
            "incomplete {key} definition=3 files=0 seconds=0 failures=2 stage=file \
             reason=filing at=1790000101 run=1790000100"
        ),
        format!(
            "incomplete {key} definition=3 files=16 seconds=1827 failures=0 stage=run \
             reason=stopped at=1790000201 run=1790000200"
        ),
        format!("done {old} definition=2 files=2 seconds=300 failures=0"),
        format!(
            "done {key} definition=4 files=16 seconds=22500 failures=0 at=1790000301 \
             run=1790000300"
        ),
        format!(
            "refused {key} definition=4 stage=census reason=census_refused failures=0 \
             run=1790000400"
        ),
    ] {
        writeln!(text, "{line}").unwrap();
    }
    std::fs::write(&path, &text).unwrap();

    let read = Journal::read_only(path.clone()).unwrap();
    let got = read.history(key);
    assert_eq!(got.hole_reason, Some("stopped"), "the LAST incomplete's");
    assert_eq!(
        got.first_landing,
        Some(ClosedBy {
            definition: Some(3),
            run: Some(1_790_000_200),
        }),
        "the first close with seconds above zero, not the seconds=0 one"
    );
    assert_eq!(
        got.last_done,
        Some(ClosedBy {
            definition: Some(4),
            run: Some(1_790_000_300),
        }),
        "a later refused close leaves the last done"
    );
    let legacy = read.history(old);
    assert_eq!(legacy.hole_reason, None);
    let unrun = Some(ClosedBy {
        definition: Some(2),
        run: None,
    });
    assert_eq!(legacy.first_landing, unrun, "a close with no run= says so");
    assert_eq!(legacy.last_done, unrun);
    assert_eq!(
        read.history("indices 2024-05-15 *"),
        KeyHistory::default(),
        "a key the journal never closed"
    );
    the_incremental_reader_keeps_the_same_history(&store, (key, old), unrun);
}

/// The second half of
/// [`the_journal_keeps_each_keys_hole_reason_first_landing_and_last_done`],
/// on its journal: the incremental reader, refreshed over three more lines,
/// moves the last done to a later `done`, leaves it for a `recounted` line,
/// keeps the first landing, and names a later `incomplete`'s reason as the
/// hole's.
fn the_incremental_reader_keeps_the_same_history(
    store: &Path,
    (key, old): (&str, &str),
    unrun: Option<ClosedBy>,
) {
    let path = journal_path(store);
    let mut reader = crate::gdfl_run::JournalReader::open(store).unwrap();
    let mut more = String::new();
    for line in [
        format!(
            "done {key} definition=4 files=16 seconds=22500 failures=0 at=1790000501 \
             run=1790000500"
        ),
        format!("recounted {key} definition=4 months=1 at=1790000601 run=1790000600"),
        format!(
            "incomplete {old} definition=4 files=2 seconds=10 failures=1 stage=file \
             reason=disk_full at=1790000701 run=1790000700"
        ),
    ] {
        writeln!(more, "{line}").unwrap();
    }
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    std::io::Write::write_all(&mut file, more.as_bytes()).unwrap();
    drop(file);
    reader.refresh().unwrap();
    let on = Day::new(2024, 5, 14).unwrap();
    let got = reader.history(ImportKind::Options, on, "NIFTY");
    assert_eq!(
        got.last_done,
        Some(ClosedBy {
            definition: Some(4),
            run: Some(1_790_000_500),
        }),
        "the newest done; the recount is not a done close"
    );
    assert_eq!(got.first_landing.unwrap().run, Some(1_790_000_200));
    assert_eq!(got.hole_reason, Some("stopped"));
    let legacy = reader.history(ImportKind::Indices, Day::new(2019, 1, 2).unwrap(), "*");
    assert_eq!(legacy.hole_reason, Some("disk_full"));
    assert_eq!(
        legacy.first_landing, unrun,
        "the first landing is never replaced"
    );
    assert_eq!(legacy.last_done, unrun);
}

/// **IA-2 (D-2814, FM-128; design §5.4 round 3 SF-10): EVERY IMPORT LEDGER
/// CLOSES A TORN OR ZEROED TAIL WITH THE MARK, AS THE JOURNAL DOES.** The
/// day, lineage, lots and scope ledgers share one append: a fragment a
/// killed append left, and a tail of NUL bytes a power cut left, are each
/// closed with ` (torn)` before the next line, so a reader knows the line
/// for torn and the next line is whole. Before IA-2 repair 2 the shared
/// append closed both with a bare newline, the NUL tail read as a line.
#[test]
fn every_import_ledger_closes_a_torn_or_zeroed_tail_with_the_mark() {
    let store = scratch("ledger-torn");
    let ledger = store.join("imports").join("gdfl.lineage");
    std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    for (tail, closed) in [
        (b"lot NIFTY 2024-0".to_vec(), "lot NIFTY 2024-0 (torn)"),
        (b"\0\0\0\0\0\0\0\0".to_vec(), "\0\0\0\0\0\0\0\0 (torn)"),
    ] {
        let mut bytes = b"whole one\n".to_vec();
        bytes.extend_from_slice(&tail);
        std::fs::write(&ledger, &bytes).unwrap();
        append_lines(&ledger, &["next one".to_owned()]).unwrap();
        let text = std::fs::read_to_string(&ledger).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines, ["whole one", closed, "next one"], "{text:?}");
    }
    // A whole tail gets no mark; nothing to append opens nothing.
    append_lines(&ledger, &["third".to_owned()]).unwrap();
    append_lines(&ledger, &[]).unwrap();
    let text = std::fs::read_to_string(&ledger).unwrap();
    assert!(text.ends_with("next one\nthird\n"), "{text:?}");
    assert_eq!(text.matches(" (torn)").count(), 1, "{text:?}");
}

/// **IA-2 REPAIR 2: WHAT THE NEW KEEPING COSTS.** Timed, not asserted (run
/// with `--ignored --nocapture`): p50, p99 and max of one `read_line` of a
/// closing line while 100,000 keys are kept (each key closed `incomplete`
/// and then `done`, so every map holds 100,000 entries), of one
/// `history` probe, and of one locked, synced ledger append on a ledger of
/// 1,000 lines. Every line is invented.
#[test]
#[ignore = "a timing, printed for docs/06-limits.md and the package report"]
fn ia2_repair2_timings() {
    fn pct(mut v: Vec<std::time::Duration>) -> String {
        v.sort_unstable();
        let n = v.len();
        format!(
            "n={n} p50={:?} p99={:?} max={:?}",
            v[n / 2],
            v[n * 99 / 100],
            v[n - 1]
        )
    }
    let keys = 100_000_u32;
    let lines: Vec<String> = (0..keys)
        .flat_map(|i| {
            let key = format!("options 2024-05-{:02} N{i}", 1 + i % 28);
            [
                format!(
                    "incomplete {key} definition=4 files=1 seconds={} failures=1 stage=file \
                     reason=filing at=1790000001 run={}",
                    i % 2,
                    1_790_000_000 + u64::from(i)
                ),
                format!(
                    "done {key} definition=4 files=1 seconds=60 failures=0 at=1790000002 run={}",
                    1_790_100_000 + u64::from(i)
                ),
            ]
        })
        .collect();
    let mut journal = Journal::default();
    let mut took = Vec::with_capacity(lines.len());
    for line in &lines {
        let fields: Vec<&str> = line.split(' ').collect();
        let began = std::time::Instant::now();
        journal.read_line(&fields).unwrap();
        took.push(began.elapsed());
    }
    println!(
        "IA-2 r2 read_line of a closing line, 100,000 keys kept: {}",
        pct(took)
    );
    let mut took = Vec::with_capacity(100_000);
    for i in 0..100_000_u32 {
        let key = format!("options 2024-05-{:02} N{i}", 1 + i % 28);
        let began = std::time::Instant::now();
        let got = journal.history(&key);
        took.push(began.elapsed());
        assert!(got.last_done.is_some());
    }
    println!("IA-2 r2 history probe, 100,000 keys: {}", pct(took));

    let store = scratch("r2-append");
    let ledger = store.join("imports").join("gdfl.lineage");
    std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    let mut seed = String::with_capacity(16_000);
    for i in 0..1_000 {
        writeln!(seed, "alias seed{i}").unwrap();
    }
    std::fs::write(&ledger, seed).unwrap();
    let mut took = Vec::with_capacity(300);
    for i in 0..300 {
        let line = vec![format!("alias timed{i}")];
        let began = std::time::Instant::now();
        append_lines(&ledger, &line).unwrap();
        took.push(began.elapsed());
    }
    println!(
        "IA-2 r2 locked ledger append (lock, last byte, write, sync): {}",
        pct(took)
    );
    // The re-key's lock check before its first month moves: one uncontended
    // take and release of the same journal lock.
    let lock = journal_lock_path(&ledger);
    let mut took = Vec::with_capacity(1_000);
    for _ in 0..1_000 {
        let began = std::time::Instant::now();
        crate::gdfl_run::locks::take_path_within(&lock, crate::gdfl_run::locks::LOCK_WAIT)
            .map(drop)
            .unwrap();
        took.push(began.elapsed());
    }
    println!(
        "IA-2 r2 re-key lock check (take and release, uncontended): {}",
        pct(took)
    );
}
