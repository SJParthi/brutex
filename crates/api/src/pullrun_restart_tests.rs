#![cfg(test)]
//! The restart scan's answer when more than one run is open, when a close
//! could end more than one run, and when a restated head lies behind a close
//! that may be its own (AG-01: the 14 Oct attack run's code-3, code-4,
//! code-7 and the restart half of code-2; D-4976).
//!
//! Split from `pullrun.rs`'s own test module so these read as one argument:
//! every run left open is named, a close the walk can only guess at is NOT
//! KNOWN and never the wrong run, and no answer covers an archive-folder leg.
//! Every fixture is invented: sources, windows and asks name no vendor row.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a whole-file test module; fixtures are bounded and a failed assertion is the test failing"
)]

use super::*;
use crate::audit::{Outcome, Record, Scope};

/// The window most fixtures ask for: 2024-10-04..2024-10-14, days since the
/// epoch.
const FROM: u32 = 20_000;
const TO: u32 = 20_010;

fn day(days: u32) -> pull::session::Day {
    pull::session::Day::from_days(days).expect("a day")
}

fn at(secs: u64) -> std::time::SystemTime {
    std::time::UNIX_EPOCH + core::time::Duration::from_secs(secs)
}

/// An opening record of `scope` for `source` over `from..to`, naming `ask`.
fn opened(secs: u64, scope: Scope, source: &str, from: u32, to: u32, ask: u64) -> Record {
    Record::opened(
        scope,
        at(secs),
        source,
        pull::session::Window::new(day(from), day(to)).expect("a window"),
        7,
        ask,
    )
}

/// A closing record of `scope` and `outcome`, as a press writes one.
fn closed(secs: u64, scope: Scope, outcome: Outcome, source: &str, from: u32, to: u32) -> Record {
    Record::refused(scope, outcome, at(secs), source, "closed")
        .with_window(pull::session::Window::new(day(from), day(to)).expect("a window"))
}

/// `head` written again by its run's thread: "continues the run opened at
/// record `of`".
fn restated(head: &Record, of: u64) -> Record {
    let mut again = head.clone();
    again.note = format!("continues the run opened at record {of}");
    again.note_bytes = u16::try_from(again.note.len()).expect("a short note");
    again
}

fn failure(secs: u64) -> Record {
    Record::member_failure(
        Scope::Spot,
        at(secs),
        "NIFTY",
        pull::session::Window::new(day(FROM), day(FROM)).expect("a window"),
        "a gap",
    )
}

fn numbered(records: Vec<Record>, first: u64) -> Vec<(u64, Record)> {
    records
        .into_iter()
        .enumerate()
        .map(|(offset, record)| (first + offset as u64, record))
        .collect()
}

/// A store root no earlier run named (a name and the time, so nothing has
/// to be removed first).
fn fresh_root(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after the epoch")
        .as_nanos();
    crate::scratch::path(&format!("ag01-{name}-{nanos}"))
}

/// A journal of exactly `records` under a [`fresh_root`], indexed in one
/// pass.
fn journal_of(name: &str, records: &[Record]) -> crate::audit::Journal {
    journal_at(&fresh_root(name), records)
}

/// [`journal_of`] at `root`.
fn journal_at(root: &std::path::Path, records: &[Record]) -> crate::audit::Journal {
    let path = crate::audit::journal_path(root);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
    let mut bytes = Vec::with_capacity(records.len() * crate::audit::RECORD_LEN);
    for record in records {
        bytes.extend_from_slice(&record.image());
    }
    std::fs::write(&path, bytes).expect("write");
    let journal = crate::audit::Journal::at(root);
    let built = journal.build_index().expect("the index builds");
    assert_eq!(built.records, records.len() as u64);
    journal
}

/// The answer's NOT KNOWN reason, or a failure naming what it was instead.
fn why_not_known(answer: &Restart) -> String {
    match answer {
        Restart::NotKnown(why) => why.clone(),
        Restart::Guessed(guessed) => guessed.why.clone(),
        other => panic!("expected NOT KNOWN, got {other:?}"),
    }
}

/// `/pull/run.json` for `answer`, parsed.
fn document(answer: &Restart) -> serde_json::Value {
    serde_json::from_str(&status_json(&Progress::default(), None, Some(answer)))
        .expect("valid JSON")
}

// ---------------------------------------------------------------------------
// code-3: every run left open is named
// ---------------------------------------------------------------------------

/// **THREE RUNS LEFT OPEN ARE ALL NAMED** (code-3): the NIFTY spot run of
/// one feed, a second feed's spot run, and an expired-F&O walk of BANKNIFTY,
/// cut short by one crash. The walk returned only the newest; the document,
/// its words and the ERROR line named one run of three.
#[test]
fn three_runs_left_open_are_all_named() {
    let _shared = crate::emitted::sink();
    let from = crate::emitted::mark();
    let records = vec![
        opened(100, Scope::Spot, "ag01-three-zerodha", FROM, TO, 0x21),
        failure(110),
        opened(120, Scope::Spot, "ag01-three-second", FROM, TO, 0x22),
        failure(130),
        opened(140, Scope::Fno, "ag01-three-BANKNIFTY", FROM, TO, 0x23),
        failure(150),
    ];
    let journal = journal_of("three", &records);
    let answer = restart_scan(&journal);
    let run = answer.interrupted().expect("a run is open");
    assert_eq!((run.ordinal, run.open_count), (4, 3));
    assert_eq!(
        run.also_open
            .iter()
            .map(|other| (
                other.ordinal,
                other.source.as_str(),
                other.ask,
                other.failures
            ))
            .collect::<Vec<_>>(),
        vec![
            (2, "ag01-three-second", Some(0x22), 1),
            (0, "ag01-three-zerodha", Some(0x21), 1),
        ],
        "newest first, each with its ask and its own failures"
    );
    let words = run.words();
    assert!(
        words.contains("3 runs read have no closing record")
            && words.contains("record 2 (a pull run")
            && words.contains("record 0 (a pull run")
            && words.contains("ag01-three-zerodha"),
        "{words}"
    );
    let doc = document(&answer);
    assert_eq!(doc["interrupted"]["ordinal"], 4);
    assert_eq!(doc["interrupted"]["openCount"], 3);
    assert_eq!(doc["interrupted"]["alsoOpen"][0]["ordinal"], 2);
    assert_eq!(doc["interrupted"]["alsoOpen"][1]["ordinal"], 0);
    assert_eq!(doc["interrupted"]["alsoOpen"][1]["scope"], "spot");
    assert_eq!(doc["interrupted"]["alsoOpen"][1]["ask"], "0000000000000021");
    assert_eq!(
        doc["interrupted"]["alsoOpen"][1]["mayBeClosed"],
        serde_json::Value::Null
    );
    // ONE ERROR LINE PER RUN, each with its own ordinal.
    let lines = crate::emitted::landed(from, "pull.run", "a pull run has no closing record")
        .into_iter()
        .filter(|line| crate::emitted::says(line, "source", "ag01-three-"))
        .collect::<Vec<_>>();
    let mut ordinals = lines
        .iter()
        .filter_map(|line| {
            line.field("ordinal")
                .and_then(telemetry::OwnedValue::as_u64)
        })
        .collect::<Vec<_>>();
    ordinals.sort_unstable();
    assert_eq!(ordinals, vec![0, 2, 4], "{lines:?}");
    assert!(
        lines
            .iter()
            .all(|line| crate::emitted::counts(line, "open_count", 3))
    );
}

/// **TWO CRASHES IN A ROW WITH DIFFERENT ASKS NAME BOTH** (code-3, the
/// skeptic's V3-3): Zerodha alone, no parallel feed. Run A (one ask) is cut
/// short; later run B over the same window and source, another ask, is cut
/// short too. A was hidden behind B.
#[test]
fn two_crashes_in_a_row_with_different_asks_name_both() {
    let read = numbered(
        vec![
            opened(100, Scope::Spot, "kite", FROM, TO, 0xA),
            failure(110),
            failure(111),
            opened(500, Scope::Spot, "kite", FROM, TO, 0xB),
            failure(510),
        ],
        0,
    );
    let answer = restart_in(&read);
    let run = answer.interrupted().expect("B is open");
    assert_eq!((run.ordinal, run.ask, run.open_count), (3, Some(0xB), 2));
    assert_eq!(
        run.also_open
            .iter()
            .map(|other| (
                other.ordinal,
                other.ask,
                other.failures,
                other.failures_whole
            ))
            .collect::<Vec<_>>(),
        vec![(0, Some(0xA), 2, true)]
    );
    let words = answer.words();
    assert!(
        words.contains(
            "record 0 (a pull run, 2024-10-04..2024-10-14, source \"kite\", ask 000000000000000a"
        ),
        "{words}"
    );
}

/// **ONE RUN OPEN READS BYTE FOR BYTE AS BEFORE** (code-3): the newest run's
/// fields and sentence are unchanged when it is the only one; the document
/// gains `openCount: 1` and an empty `alsoOpen` after its old keys.
#[test]
fn one_run_open_reads_as_before() {
    // A kill between the opening record and the first member: one record.
    let read = numbered(vec![opened(100, Scope::Spot, "kite", FROM, TO, 0x5)], 0);
    let answer = restart_in(&read);
    let run = answer.interrupted().expect("open");
    assert_eq!(
        (run.ordinal, run.failures, run.open_count, run.runs_unread),
        (0, 0, 1, 0)
    );
    assert!(run.also_open.is_empty());
    let words = run.words();
    assert!(
        !words.contains("runs read have no closing record"),
        "{words}"
    );
    assert!(!words.contains("older run record(s)"), "{words}");
    assert!(
        words.ends_with(
            "same ask, once it closes having asked the vendor (any close but NOT STARTED), \
             supersedes this one."
        ),
        "{words}"
    );
    let raw = status_json(&Progress::default(), None, Some(&answer));
    assert!(
        raw.contains(",\"openCount\":1,\"runRecordsUnread\":0,\"alsoOpen\":[]}"),
        "{raw}"
    );
    // The old keys come first, in their old order.
    let words_at = raw.find("\"words\":").expect("words");
    let count_at = raw.find("\"openCount\"").expect("count");
    assert!(words_at < count_at);
}

/// **THE OPEN LIST STOPS AT THE SCAN'S BOUND AND SAYS SO** (code-3).
///
/// * 65 runs opened, then 800 failures: the index reads the 64 newest run
///   records, so 64 runs are found and named, and the one run record behind
///   them is counted as not read.
/// * Exactly 64: all named, nothing unread.
/// * 70 opened inside the scan: 70 are counted, 64 named, 6 counted only.
#[test]
fn the_open_list_stops_at_the_scan_bound_and_says_so() {
    let many = |runs: u64, failures: u64| {
        let mut records = (0..runs)
            .map(|run| {
                opened(
                    100 + run,
                    Scope::Spot,
                    &format!("ag01-bound-{run}"),
                    FROM,
                    TO,
                    0x100 + run,
                )
            })
            .collect::<Vec<_>>();
        records.extend((0..failures).map(|offset| failure(1_000 + offset)));
        records
    };
    let journal = journal_of("bound-65", &many(65, INTERRUPTED_SCAN));
    let answer = restart_scan(&journal);
    let run = answer.interrupted().expect("open");
    assert_eq!(
        (
            run.ordinal,
            run.open_count,
            run.also_open.len(),
            run.runs_unread
        ),
        (64, 64, 63, 1)
    );
    assert_eq!(run.also_open.last().map(|other| other.ordinal), Some(1));
    assert!(
        !run.failures_whole,
        "its opening record lies before the scan"
    );
    let words = run.words();
    assert!(
        words.contains(
            "1 older run record(s) that the journal's index names lie before the 64 run records \
             read through it and were not read"
        ),
        "{words}"
    );
    assert_eq!(document(&answer)["interrupted"]["runRecordsUnread"], 1);

    let journal = journal_of("bound-64", &many(64, INTERRUPTED_SCAN));
    let run = restart_scan(&journal).interrupted().cloned().expect("open");
    assert_eq!(
        (
            run.ordinal,
            run.open_count,
            run.also_open.len(),
            run.runs_unread
        ),
        (63, 64, 63, 0)
    );
    assert!(!run.words().contains("older run record(s)"));

    let _shared = crate::emitted::sink();
    let from = crate::emitted::mark();
    let read = numbered(many(70, 0), 0);
    let run = restart_in(&read).interrupted().cloned().expect("open");
    // The runs counted and not named are on `/logs` too, as one line.
    let unnamed = crate::emitted::landed(
        from,
        "pull.run",
        "pull runs have no closing record, not named",
    );
    assert!(
        unnamed
            .iter()
            .any(|line| crate::emitted::counts(line, "unnamed", 6)
                && crate::emitted::counts(line, "open_count", 70)),
        "{unnamed:?}"
    );
    assert_eq!(
        (run.ordinal, run.open_count, run.also_open.len()),
        (69, 70, 63)
    );
    assert!(
        run.words()
            .contains("6 more run(s) read have no closing record and are counted, not named"),
        "{}",
        run.words()
    );
    assert_eq!(
        document(&Restart::Interrupted(run))["interrupted"]["alsoOpen"]
            .as_array()
            .map(Vec::len),
        Some(63)
    );
}

/// **TWO SERVERS WRITING ONE JOURNAL, BOTH KILLED** (code-3): each run's
/// thread restates its head when the other appended between, and both runs
/// are named, with every failure counted to its own run.
#[test]
fn two_writers_of_one_journal_cut_short_are_both_named() {
    let root = fresh_root("two-writers");
    let journal = journal_at(&root, &[]);
    let other = crate::audit::Journal::at(&root);
    let mut one = journal
        .open_run(&opened(100, Scope::Spot, "ag01-writer-one", FROM, TO, 0x1))
        .expect("opened");
    let mut two = other
        .open_run(&opened(101, Scope::Spot, "ag01-writer-two", FROM, TO, 0x2))
        .expect("opened");
    for offset in 0..3 {
        journal
            .append_failure(&mut one, &failure(110 + offset), "a gap")
            .expect("lands");
        other
            .append_failure(&mut two, &failure(120 + offset), "a gap")
            .expect("lands");
    }
    let answer = restart_scan(&journal);
    let run = answer.interrupted().expect("open");
    assert_eq!(
        (run.source.as_str(), run.failures, run.open_count),
        ("ag01-writer-two", 3, 2)
    );
    assert_eq!(
        run.also_open
            .iter()
            .map(|other| (other.ordinal, other.source.as_str(), other.failures))
            .collect::<Vec<_>>(),
        vec![(0, "ag01-writer-one", 3)]
    );
}

/// **A RECORD THAT WILL NOT DECODE AFTER AN OLDER OPEN RUN IS SAID ON THAT
/// RUN** (code-3): it may be that run's close, so the run is listed with the
/// doubt, and the newest run (after the damage) is reported as before.
#[test]
fn damage_after_an_older_open_run_is_said_on_it() {
    use std::io::{Seek as _, Write as _};
    let records = vec![
        opened(100, Scope::Spot, "ag01-damage-old", FROM, TO, 0x1),
        failure(110),
        failure(111),
        opened(200, Scope::Spot, "ag01-damage-new", 20_020, 20_030, 0x2),
        failure(210),
    ];
    let root = fresh_root("damage-older");
    let journal = journal_at(&root, &records);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(crate::audit::journal_path(&root))
        .expect("open");
    file.seek(std::io::SeekFrom::Start(
        2 * crate::audit::RECORD_LEN as u64,
    ))
    .expect("seek");
    file.write_all(&[0xff; crate::audit::RECORD_LEN])
        .expect("damage");
    drop(file);
    let answer = restart_scan(&journal);
    let run = answer.interrupted().expect("the newer run is open");
    assert_eq!(
        (run.ordinal, run.undecodable_before, run.open_count),
        (3, 1, 2)
    );
    assert_eq!(
        run.also_open
            .first()
            .map(|other| (other.ordinal, other.doubt)),
        Some((0, Some(CLOSE_UNDECODED)))
    );
    assert!(
        run.words()
            .contains("it may have been closed: a record after its opening record")
    );
    assert_eq!(
        document(&answer)["interrupted"]["alsoOpen"][0]["mayBeClosed"],
        CLOSE_UNDECODED
    );
}

/// **AN OLDER RUN KNOWN ONLY THROUGH A RESTATED HEAD, WITH THE INDEX
/// UNREADABLE, IS LISTED WITH ITS DOUBT** (code-3): its close may lie before
/// the scan, where nothing could be read, so it is named as possibly closed;
/// the newer run, opened inside the scan, is reported as before.
#[test]
fn an_older_run_whose_close_may_lie_before_an_unreadable_index_is_doubted() {
    let head = opened(100, Scope::Spot, "kite", FROM, TO, 0x5);
    let read = numbered(
        vec![
            restated(&head, 5),
            failure(110),
            opened(200, Scope::Spot, "kite", 20_020, 20_030, 0x6),
            failure(210),
        ],
        900,
    );
    let answer = judged(
        &read,
        Vec::new(),
        Some("the index is not there".to_owned()),
        Bounds {
            records: 904,
            scan_first: 104,
            runs_unread: 0,
        },
    );
    let run = answer.interrupted().expect("the newer run is open");
    assert_eq!((run.ordinal, run.open_count), (902, 2));
    assert_eq!(
        run.also_open
            .iter()
            .map(|other| (other.ordinal, other.doubt, other.failures_whole))
            .collect::<Vec<_>>(),
        vec![(5, Some(CLOSE_UNREAD), false)]
    );
    assert!(
        run.words()
            .contains("it may have been closed: it is known only through a restated head")
    );
}

// ---------------------------------------------------------------------------
// code-4: no guessed pairing
// ---------------------------------------------------------------------------

/// **A CLOSE THAT COULD END TWO WALKS IS NOT KNOWN** (code-4): two
/// expired-F&O walks of NIFTY over one window, on two feeds (the feed is in
/// the ask; the source is the underlying), and the older closes first, not
/// directly behind a record of its own. The walk handed the close to the
/// newest, so the walk that FINISHED was reported cut short and the one cut
/// short was hidden. Now: NOT KNOWN, naming both, never the wrong ordinal.
#[test]
fn a_close_that_could_end_two_walks_is_not_known() {
    let records = vec![
        opened(100, Scope::Fno, "NIFTY", FROM, TO, 0x6),
        opened(101, Scope::Fno, "NIFTY", FROM, TO, 0xd),
        opened(102, Scope::Spot, "kite", 20_020, 20_030, 0x1),
        closed(103, Scope::Spot, Outcome::Stored, "kite", 20_020, 20_030),
        closed(104, Scope::Fno, Outcome::NotStarted, "NIFTY", FROM, TO),
    ];
    let read = numbered(records.clone(), 0);
    let why = why_not_known(&restart_in(&read));
    assert!(
        why.contains("the closing record at record 4")
            && why.contains("a close written without its run's thread could end either run")
            && why.contains("opened at records 1, 0"),
        "{why}"
    );
    // The same journal on disk, through the scan and onto the document.
    let journal = journal_of("two-walks", &records);
    let answer = restart_scan(&journal);
    assert_eq!(answer.word(), "not_known", "{}", answer.words());
    let doc = document(&answer);
    assert_eq!(doc["interrupted"], serde_json::Value::Null);
    assert_eq!(doc["restart"]["state"], "not_known");

    // The skeptic's V2: an older walk of the same underlying, window and
    // source left open earlier, then two feeds' walks, an unrelated
    // BANKNIFTY walk that opens and closes, and the second walk's STORED
    // close. All three are named.
    let read = numbered(
        vec![
            opened(50, Scope::Fno, "NIFTY", FROM, TO, 0xd),
            opened(100, Scope::Fno, "NIFTY", FROM, TO, 0x2),
            opened(101, Scope::Fno, "NIFTY", FROM, TO, 0xd),
            opened(102, Scope::Fno, "BANKNIFTY", FROM, TO, 0x3),
            closed(103, Scope::Fno, Outcome::Stored, "BANKNIFTY", FROM, TO),
            closed(104, Scope::Fno, Outcome::Stored, "NIFTY", FROM, TO),
        ],
        0,
    );
    let why = why_not_known(&restart_in(&read));
    assert!(why.contains("opened at records 2, 1, 0"), "{why}");
}

/// **A CLOSE THAT DIRECTLY FOLLOWS ITS RUN STILL PAIRS** (code-4): no guess
/// where position proves the run, and none where only one run of the
/// close's scope, window and source is open. Position proves a SPOT run's
/// close (`close_run` writes it behind a record of its own run, under the
/// append lock), and an expired-F&O walk's only while it is the one walk of
/// its scope, window and source open, or every one open names one ask: an F&O
/// walk closes with a plain append (second check's blocking 1, below).
/// Repeated on BANKNIFTY and on an F&O stock.
#[test]
fn a_close_that_directly_follows_its_walk_still_pairs() {
    // Two spot runs of one feed and window, two asks; the newer closes
    // directly behind its own opening record: the older is cut short.
    let read = numbered(
        vec![
            opened(100, Scope::Spot, "kite", FROM, TO, 0x6),
            opened(101, Scope::Spot, "kite", FROM, TO, 0xd),
            closed(102, Scope::Spot, Outcome::Stored, "kite", FROM, TO),
        ],
        0,
    );
    let run = restart_in(&read)
        .interrupted()
        .cloned()
        .expect("the older spot run is open");
    assert_eq!((run.ordinal, run.ask, run.open_count), (0, Some(0x6), 1));
    for underlying in ["NIFTY", "BANKNIFTY", "RELIANCE"] {
        // One walk open and its close directly behind it.
        let read = numbered(
            vec![
                opened(100, Scope::Fno, underlying, FROM, TO, 0x6),
                closed(101, Scope::Fno, Outcome::Stored, underlying, FROM, TO),
            ],
            0,
        );
        assert_eq!(restart_in(&read), Restart::Clean, "{underlying}");
        // Two walks of ONE ask, a close directly behind the newer: the same
        // walk finished whichever it was, so nothing is open and nothing is
        // a guess.
        let read = numbered(
            vec![
                opened(100, Scope::Fno, underlying, FROM, TO, 0x6),
                opened(101, Scope::Fno, underlying, FROM, TO, 0x6),
                closed(102, Scope::Fno, Outcome::Stored, underlying, FROM, TO),
            ],
            0,
        );
        assert_eq!(restart_in(&read), Restart::Clean, "{underlying}");
        // One walk open and a close of it with records between: the only
        // candidate, so it pairs (the weak path, as before).
        let read = numbered(
            vec![
                opened(100, Scope::Fno, underlying, FROM, TO, 0x6),
                opened(101, Scope::Spot, "kite", 20_020, 20_030, 0x1),
                closed(102, Scope::Fno, Outcome::Stored, underlying, FROM, TO),
            ],
            0,
        );
        let run = restart_in(&read)
            .interrupted()
            .cloned()
            .expect("kite is open");
        assert_eq!((run.ordinal, run.open_count), (1, 1), "{underlying}");
    }
}

/// The guessed half of `answer`, or a failure naming what it was instead.
fn guessed_of(answer: &Restart) -> &Guessed {
    match answer {
        Restart::Guessed(guessed) => guessed,
        other => panic!("expected a guessed NOT KNOWN, got {other:?}"),
    }
}

/// **AN F&O WALK'S CLOSE BEHIND ANOTHER WALK'S OPENING RECORD IS NOT KNOWN**
/// (code-4, the attack's A1; second check's blocking 1). Walk G (ask 6)
/// opens, walk D (ask d) opens, and G's NOT STARTED close lands directly
/// behind D's opening record. By position the close was D's: G, the walk that
/// finished, was reported cut short, and D, the one that was, was hidden. An
/// F&O walk closes with a plain append, so position proves nothing while two
/// of its walks are open: NOT KNOWN, naming both, never record 0 alone, and
/// one of the two is surely open (a close ends one run). Repeated on
/// BANKNIFTY and on an F&O stock, on disk through the scan and onto the
/// document and `/logs`.
#[test]
fn an_fno_close_behind_another_walks_opening_is_not_known() {
    let _shared = crate::emitted::sink();
    for underlying in ["NIFTY", "BANKNIFTY", "RELIANCE"] {
        let source = format!("ag01-a1-{underlying}");
        let records = vec![
            opened(100, Scope::Fno, &source, FROM, TO, 0x6),
            opened(101, Scope::Fno, &source, FROM, TO, 0xd),
            closed(102, Scope::Fno, Outcome::NotStarted, &source, FROM, TO),
        ];
        let answer = restart_in(&numbered(records.clone(), 0));
        let guessed = guessed_of(&answer);
        assert!(
            guessed.why.contains("the closing record at record 2")
                && guessed.why.contains("opened at records 1, 0"),
            "{underlying}: {}",
            guessed.why
        );
        assert_eq!(guessed.guessed_runs, vec![0, 1], "{underlying}");
        assert!(guessed.sure.is_empty(), "{underlying}: {:?}", guessed.sure);
        assert_eq!(
            guessed
                .left_open
                .iter()
                .map(|counted| (counted.close, counted.runs.clone(), counted.left_open))
                .collect::<Vec<_>>(),
            vec![(2, vec![1, 0], 1)],
            "{underlying}"
        );
        assert_eq!(guessed.at_least_open(), 1, "{underlying}");
        let words = answer.words();
        assert!(
            words.starts_with("NOT KNOWN which pull runs were left open, though at least 1 run(s)")
                && words.contains("so 1 of them have no closing record; which is not known"),
            "{underlying}: {words}"
        );

        // On disk, through the scan, onto the document and the log.
        let from = crate::emitted::mark();
        let journal = journal_of(&format!("a1-{underlying}"), &records);
        let answer = restart_scan(&journal);
        assert_eq!(answer.word(), "not_known", "{}", answer.words());
        let doc = document(&answer);
        assert_eq!(doc["interrupted"], serde_json::Value::Null, "{underlying}");
        assert_eq!(doc["restart"]["state"], "not_known");
        assert_eq!(doc["restart"]["guessedRuns"], serde_json::json!([0, 1]));
        assert_eq!(doc["restart"]["leftOpen"][0]["close"], 2);
        assert_eq!(doc["restart"]["leftOpen"][0]["leftOpen"], 1);
        assert_eq!(doc["restart"]["leftOpen"][0]["scope"], "fno");
        assert_eq!(doc["restart"]["atLeastOpen"], 1);
        assert_eq!(
            doc["restart"]["reading"]["ordinal"], 0,
            "the walk's reading, labelled so"
        );
        let counted = crate::emitted::landed(
            from,
            "pull.run",
            "pull runs have no closing record, which of them is not known",
        )
        .into_iter()
        .filter(|line| crate::emitted::says(line, "source", &source))
        .collect::<Vec<_>>();
        assert_eq!(counted.len(), 1, "{underlying}: {counted:?}");
        assert!(crate::emitted::counts(&counted[0], "left_open", 1));
        assert!(crate::emitted::counts(&counted[0], "close", 2));
    }
    // The skeptic's V2 (three walks of one underlying, one close): two are
    // surely open.
    let read = numbered(
        vec![
            opened(50, Scope::Fno, "NIFTY", FROM, TO, 0xd),
            opened(100, Scope::Fno, "NIFTY", FROM, TO, 0x2),
            opened(101, Scope::Fno, "NIFTY", FROM, TO, 0xd),
            opened(102, Scope::Fno, "BANKNIFTY", FROM, TO, 0x3),
            closed(103, Scope::Fno, Outcome::Stored, "BANKNIFTY", FROM, TO),
            closed(104, Scope::Fno, Outcome::Stored, "NIFTY", FROM, TO),
        ],
        0,
    );
    let answer = restart_in(&read);
    assert_eq!(guessed_of(&answer).at_least_open(), 2);
    assert!(
        answer.words().contains("at least 2 run(s)"),
        "{}",
        answer.words()
    );
}

/// **A RUN SURELY OPEN BESIDE A GUESS IS NAMED AND LOGGED** (second check's
/// blocking 2, its probe P4). A NIFTY spot run is cut short; two NIFTY F&O
/// walks of two asks open, another feed's spot run opens and closes, and one
/// NOT STARTED close of the walks lands behind it. The guess made the answer
/// a bare NOT KNOWN: the spot run, open whichever way the close is read, was
/// on no document and had no ERROR line, and "NOT KNOWN whether a pull run
/// was left open" was said of records that surely leave two open.
#[test]
fn a_run_surely_open_beside_a_guess_is_named_and_logged() {
    let _shared = crate::emitted::sink();
    let records = vec![
        opened(100, Scope::Spot, "ag01-p4-kite", 20_020, 20_030, 0x1),
        failure(110),
        opened(120, Scope::Fno, "ag01-p4-NIFTY", FROM, TO, 0x6),
        opened(121, Scope::Fno, "ag01-p4-NIFTY", FROM, TO, 0xd),
        opened(130, Scope::Spot, "ag01-p4-other", FROM, TO, 0x9),
        closed(131, Scope::Spot, Outcome::Stored, "ag01-p4-other", FROM, TO),
        closed(
            140,
            Scope::Fno,
            Outcome::NotStarted,
            "ag01-p4-NIFTY",
            FROM,
            TO,
        ),
    ];
    let answer = restart_in(&numbered(records.clone(), 0));
    let guessed = guessed_of(&answer);
    assert_eq!(
        guessed
            .sure
            .iter()
            .map(|run| (run.ordinal, run.source.as_str(), run.failures))
            .collect::<Vec<_>>(),
        vec![(0, "ag01-p4-kite", 1)],
        "the spot run is open whichever way the close is read"
    );
    assert_eq!(guessed.guessed_runs, vec![2, 3]);
    assert_eq!(guessed.at_least_open(), 2);
    let words = answer.words();
    assert!(
        words.starts_with("NOT KNOWN which pull runs were left open, though at least 2 run(s)")
            && words.contains("Open whichever way the guesses are read")
            && words.contains("record 0 (a pull run")
            && words.contains("ag01-p4-kite")
            && words.contains("A guess names the run(s) opened at records 2, 3"),
        "{words}"
    );

    let from = crate::emitted::mark();
    let answer = restart_scan(&journal_of("p4", &records));
    let doc = document(&answer);
    assert_eq!(doc["restart"]["state"], "not_known");
    assert_eq!(doc["interrupted"], serde_json::Value::Null);
    assert_eq!(doc["restart"]["sureOpen"][0]["ordinal"], 0);
    assert_eq!(doc["restart"]["sureOpen"][0]["source"], "ag01-p4-kite");
    assert_eq!(doc["restart"]["atLeastOpen"], 2);
    // ONE ERROR LINE FOR THE SURE RUN, one for the count, and the WARN line.
    let sure = crate::emitted::landed(from, "pull.run", "a pull run has no closing record")
        .into_iter()
        .filter(|line| crate::emitted::says(line, "source", "ag01-p4-"))
        .collect::<Vec<_>>();
    assert_eq!(sure.len(), 1, "{sure:?}");
    assert!(crate::emitted::counts(&sure[0], "ordinal", 0));
    assert_eq!(sure[0].level, telemetry::Level::Error);
    let counted = crate::emitted::landed(
        from,
        "pull.run",
        "pull runs have no closing record, which of them is not known",
    )
    .into_iter()
    .filter(|line| crate::emitted::says(line, "source", "ag01-p4-NIFTY"))
    .collect::<Vec<_>>();
    assert_eq!(counted.len(), 1, "{counted:?}");
    assert_eq!(counted[0].level, telemetry::Level::Error);
    let warned = crate::emitted::landed(
        from,
        "pull.run",
        "whether a pull run was left open is not known",
    )
    .into_iter()
    // A string field is kept to `telemetry::MAX_STR_VALUE_BYTES` (512): the
    // WARN line carries the headline and the guess, and the sure run's own
    // ERROR line above carries its clause.
    .filter(|line| {
        crate::emitted::says(
            line,
            "reason",
            "though at least 2 run(s) were: the closing record at record 6 does not directly \
             follow",
        )
    })
    .count();
    assert_eq!(warned, 1);
}

/// **A COUNT IS CLAIMED ONLY WHERE NOTHING READ COULD MOVE IT** (second
/// check's blocking 2): a later close of the guessed runs' scope, window and
/// source could end, or supersede, any of them, so no count is said; a record
/// that would not decode after the oldest of their opening records may be a
/// close of one, so no count is said and no run it may have closed is sure;
/// and a guess with nothing beside it that could leave no run open still
/// reads "whether a pull run was left open".
#[test]
fn a_count_is_claimed_only_where_nothing_read_could_move_it() {
    // A later close of the same key: no count; the unrelated kite run is
    // still sure.
    let read = numbered(
        vec![
            opened(100, Scope::Fno, "NIFTY", FROM, TO, 0x6),
            opened(101, Scope::Fno, "NIFTY", FROM, TO, 0xd),
            opened(102, Scope::Spot, "kite", 20_020, 20_030, 0x1),
            closed(103, Scope::Fno, Outcome::Stored, "NIFTY", FROM, TO),
            opened(104, Scope::Fno, "NIFTY", FROM, TO, 0x6),
            closed(105, Scope::Fno, Outcome::Stored, "NIFTY", FROM, TO),
        ],
        0,
    );
    let answer = restart_in(&read);
    let guessed = guessed_of(&answer);
    assert!(guessed.left_open.is_empty(), "{:?}", guessed.left_open);
    assert_eq!(
        guessed
            .sure
            .iter()
            .map(|run| run.ordinal)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(guessed.at_least_open(), 1);

    // A record that would not decode after the oldest guessed run's opening
    // record: no count, and the runs it may have closed are not sure; the run
    // opened after it is.
    let read = numbered(
        vec![
            opened(100, Scope::Spot, "kite", 20_020, 20_030, 0x1),
            failure(101),
            opened(120, Scope::Fno, "NIFTY", FROM, TO, 0x6),
            opened(121, Scope::Fno, "NIFTY", FROM, TO, 0xd),
            failure(122),
            closed(131, Scope::Spot, Outcome::Stored, "other", FROM, TO),
            closed(140, Scope::Fno, Outcome::NotStarted, "NIFTY", FROM, TO),
            opened(150, Scope::Spot, "late", 20_040, 20_050, 0x9),
        ],
        0,
    );
    let damaged: Vec<Damaged> = vec![(4, "a torn record".into())];
    let read: Vec<(u64, Record)> = read.into_iter().filter(|(at, _)| *at != 4).collect();
    let answer = judged(
        &read,
        damaged,
        None,
        Bounds {
            records: 8,
            scan_first: 0,
            runs_unread: 0,
        },
    );
    let guessed = guessed_of(&answer);
    assert!(guessed.left_open.is_empty(), "{:?}", guessed.left_open);
    assert_eq!(
        guessed
            .sure
            .iter()
            .map(|run| run.ordinal)
            .collect::<Vec<_>>(),
        vec![7],
        "only the run opened after the damage is sure"
    );
    let reading = guessed.reading.as_ref().expect("a reading");
    assert_eq!(reading.undecodable_before, 1);

    // A restated run behind an unseen close, and nothing else: the reading
    // may leave no run open, so the headline is unchanged.
    let head = opened(100, Scope::Spot, "kite", FROM, TO, 0x5);
    let read = numbered(
        vec![
            closed(120, Scope::Spot, Outcome::Stored, "kite", FROM, TO),
            restated(&head, 5),
        ],
        900,
    );
    let answer = restart_in(&read);
    assert_eq!(guessed_of(&answer).at_least_open(), 0);
    assert!(
        answer
            .words()
            .starts_with("NOT KNOWN whether a pull run was left open: "),
        "{}",
        answer.words()
    );
}

/// **A GUESSED ANSWER NAMES EVERY RUN OF ITS READING** (third check of
/// AG-01). A NIFTY spot run opens, a record after it will not decode, two
/// NIFTY F&O walks of two asks open and one NOT STARTED close of theirs lands
/// directly behind the newer. The spot run is open in the walk's reading,
/// named by no guess, and the damaged record may be its close: it was in
/// `restart.reading` only, and the words never named it. It is named now,
/// with why it may be closed, under `doubtedOpen`, and it gets no ERROR line
/// (it may not be cut short). The run records the index names behind the
/// scan are said as an interrupted answer says them. Repeated on BANKNIFTY
/// and on an F&O stock.
#[test]
fn a_guessed_answer_names_every_run_of_its_reading() {
    let _shared = crate::emitted::sink();
    for underlying in ["NIFTY", "BANKNIFTY", "RELIANCE"] {
        let spot = format!("ag01-d-kite-{underlying}");
        let walks = format!("ag01-d-{underlying}");
        let read = numbered(
            vec![
                opened(100, Scope::Spot, &spot, 20_020, 20_030, 0x1),
                failure(101),
                opened(120, Scope::Fno, &walks, FROM, TO, 0x6),
                opened(121, Scope::Fno, &walks, FROM, TO, 0xd),
                closed(122, Scope::Fno, Outcome::NotStarted, &walks, FROM, TO),
            ],
            0,
        );
        let read: Vec<(u64, Record)> = read.into_iter().filter(|(at, _)| *at != 1).collect();
        let from = crate::emitted::mark();
        let answer = judged(
            &read,
            vec![(1, "a torn record".into())],
            None,
            Bounds {
                records: 5,
                scan_first: 0,
                runs_unread: 3,
            },
        );
        let guessed = guessed_of(&answer);
        assert!(guessed.sure.is_empty(), "{underlying}: {:?}", guessed.sure);
        assert_eq!(guessed.guessed_runs, vec![2, 3], "{underlying}");
        assert_eq!(
            guessed
                .doubted
                .iter()
                .map(|run| (run.ordinal, run.source.as_str(), run.doubt))
                .collect::<Vec<_>>(),
            vec![(0, spot.as_str(), Some(CLOSE_UNDECODED))],
            "{underlying}"
        );
        let words = answer.words();
        assert!(
            words.contains("named by no guess, though something read may have closed it")
                && words.contains(&format!(
                    "record 0 (a pull run, 2024-10-24..2024-11-03, source {spot:?}"
                ))
                && words.contains(CLOSE_UNDECODED)
                && words.contains("3 older run record(s) that the journal's index names"),
            "{underlying}: {words}"
        );
        let doc = document(&answer);
        assert_eq!(
            doc["restart"]["doubtedOpen"][0]["ordinal"], 0,
            "{underlying}"
        );
        assert_eq!(
            doc["restart"]["doubtedOpen"][0]["mayBeClosed"],
            CLOSE_UNDECODED
        );
        assert_eq!(doc["restart"]["sureOpen"], serde_json::json!([]));
        // No ERROR line for a run that may have been closed.
        let cut_short =
            crate::emitted::landed(from, "pull.run", "a pull run has no closing record")
                .into_iter()
                .filter(|line| crate::emitted::says(line, "source", &spot))
                .count();
        assert_eq!(cut_short, 0, "{underlying}");
    }
    // Nothing doubted beside a guess: the list is empty and the words have
    // no such sentence.
    let read = numbered(
        vec![
            opened(120, Scope::Fno, "NIFTY", FROM, TO, 0x6),
            opened(121, Scope::Fno, "NIFTY", FROM, TO, 0xd),
            closed(122, Scope::Fno, Outcome::NotStarted, "NIFTY", FROM, TO),
        ],
        0,
    );
    let answer = restart_in(&read);
    assert!(guessed_of(&answer).doubted.is_empty());
    assert!(
        !answer.words().contains("named by no guess"),
        "{}",
        answer.words()
    );
    assert_eq!(
        document(&answer)["restart"]["doubtedOpen"],
        serde_json::json!([])
    );
}

/// **A FALLBACK CLOSE NEVER SUPERSEDES ANOTHER ASK'S CUT-SHORT RUN** (code-4,
/// the skeptic's V4b): a Dhan walk is cut short, a Zerodha walk and a second
/// Dhan walk of the same NIFTY window open, the Zerodha walk's STORED close
/// lands with another record between, then a lone NOT STARTED close. It read
/// "clean": the STORED close was handed to the second Dhan walk and, with
/// its ask, superseded the first. Now NOT KNOWN, never clean.
#[test]
fn a_fallback_close_never_supersedes_another_asks_cut_short_run() {
    let dhan = 0x0d4a;
    let zerodha = 0x2e40;
    let read = numbered(
        vec![
            opened(100, Scope::Fno, "NIFTY", FROM, TO, dhan),
            opened(110, Scope::Fno, "NIFTY", FROM, TO, zerodha),
            opened(120, Scope::Fno, "NIFTY", FROM, TO, dhan),
            opened(125, Scope::Spot, "kite", 20_020, 20_030, 0x1),
            closed(130, Scope::Fno, Outcome::Stored, "NIFTY", FROM, TO),
            closed(140, Scope::Fno, Outcome::NotStarted, "NIFTY", FROM, TO),
        ],
        0,
    );
    let answer = restart_in(&read);
    assert_ne!(answer, Restart::Clean);
    let why = why_not_known(&answer);
    assert!(why.contains("opened at records 2, 1, 0"), "{why}");
    // The walk's own reading supersedes nothing by a guess: the first Dhan
    // walk is still open in it.
    let found = interrupted_in(&read).expect("open in the walk's reading");
    let open = core::iter::once(found.ordinal)
        .chain(found.also_open.iter().map(|other| other.ordinal))
        .collect::<Vec<_>>();
    assert!(open.contains(&0), "{open:?}");

    // Where every open run of the close names ONE ask, the same pull
    // finished whichever it was: the newest is closed and the rest are
    // superseded, as before, and nothing is a guess.
    let read = numbered(
        vec![
            opened(100, Scope::Fno, "NIFTY", FROM, TO, dhan),
            opened(120, Scope::Fno, "NIFTY", FROM, TO, dhan),
            opened(125, Scope::Spot, "kite", 20_020, 20_030, 0x1),
            closed(130, Scope::Fno, Outcome::Stored, "NIFTY", FROM, TO),
        ],
        0,
    );
    let run = restart_in(&read)
        .interrupted()
        .cloned()
        .expect("kite is open");
    assert_eq!((run.ordinal, run.open_count), (2, 1));
    // ... but a NOT STARTED close asked the vendor for nothing: which of the
    // two it ended is not known.
    let read = numbered(
        vec![
            opened(100, Scope::Fno, "NIFTY", FROM, TO, dhan),
            opened(120, Scope::Fno, "NIFTY", FROM, TO, dhan),
            opened(125, Scope::Spot, "kite", 20_020, 20_030, 0x1),
            closed(130, Scope::Fno, Outcome::NotStarted, "NIFTY", FROM, TO),
        ],
        0,
    );
    assert_eq!(restart_in(&read).word(), "not_known");
}

/// **A GUESS SETTLED LATER DECIDES NOTHING** (code-4): of two runs a close
/// could end, the other one closes later too, so both are closed whichever
/// way the first went: "clean", not NOT KNOWN.
#[test]
fn a_guess_both_of_whose_runs_close_later_is_settled() {
    let read = numbered(
        vec![
            opened(100, Scope::Fno, "NIFTY", FROM, TO, 0x6),
            opened(101, Scope::Fno, "NIFTY", FROM, TO, 0xd),
            opened(102, Scope::Spot, "kite", 20_020, 20_030, 0x1),
            closed(103, Scope::Spot, Outcome::Stored, "kite", 20_020, 20_030),
            closed(104, Scope::Fno, Outcome::NotStarted, "NIFTY", FROM, TO),
            failure(105),
            closed(106, Scope::Fno, Outcome::Stored, "NIFTY", FROM, TO),
        ],
        0,
    );
    assert_eq!(restart_in(&read), Restart::Clean);
}

/// **A GUESS WHOSE OTHER RUN WAS ONLY SUPERSEDED STILL DECIDES** (code-4):
/// a close could end a walk of one ask or a walk of another; a later walk of
/// the FIRST ask then closes and supersedes it. Had the guessed close been
/// the first walk's, the second walk is still open: NOT KNOWN, not the
/// unrelated run the walk's reading leaves open.
#[test]
fn a_guess_whose_other_run_was_only_superseded_still_decides() {
    let read = numbered(
        vec![
            opened(100, Scope::Fno, "NIFTY", FROM, TO, 0x6),
            opened(101, Scope::Fno, "NIFTY", FROM, TO, 0xd),
            opened(102, Scope::Spot, "kite", 20_020, 20_030, 0x1),
            closed(103, Scope::Fno, Outcome::Stored, "NIFTY", FROM, TO),
            opened(104, Scope::Fno, "NIFTY", FROM, TO, 0x6),
            closed(105, Scope::Fno, Outcome::Stored, "NIFTY", FROM, TO),
        ],
        0,
    );
    // The walk's own reading: the guessed close took walk 1, walk 4
    // superseded walk 0, and only the kite run is left.
    assert_eq!(
        interrupted_in(&read).map(|run| (run.ordinal, run.open_count)),
        Some((2, 1))
    );
    let why = why_not_known(&restart_in(&read));
    assert!(why.contains("opened at records 1, 0"), "{why}");
}

/// **OUT-OF-ORDER ORDINALS ARE NEVER READ AS A POSITION** (code-4): a close
/// whose ordinal does not follow the record read before it is not "beside"
/// anything, so two candidates are a guess, not the wrong run.
#[test]
fn out_of_order_ordinals_never_prove_a_pairing() {
    let read = vec![
        (5, opened(100, Scope::Spot, "kite", FROM, TO, 0x1)),
        (3, opened(110, Scope::Spot, "kite", FROM, TO, 0x2)),
        (
            6,
            closed(120, Scope::Spot, Outcome::Stored, "kite", FROM, TO),
        ),
    ];
    assert_eq!(restart_in(&read).word(), "not_known");
}

// ---------------------------------------------------------------------------
// code-7: no dropped live run
// ---------------------------------------------------------------------------

/// **AN UNSEEN CLOSE NEVER DROPS A LIVE RESTATED RUN** (code-7, the attack's
/// A3): `[close][continues the run opened at record 5][failure][failure]`.
/// The run restated itself AFTER a close of its scope, window and source
/// whose run was not read; it was dropped as closed and the answer was
/// "clean". Neither record names an ask, so this and a failure offered again
/// behind the run's own close are one shape: NOT KNOWN, never clean.
#[test]
fn an_unseen_close_never_drops_a_live_restated_run() {
    let head = opened(100, Scope::Spot, "kite", FROM, TO, 0x5);
    let read = numbered(
        vec![
            closed(120, Scope::Spot, Outcome::Stored, "kite", FROM, TO),
            restated(&head, 5),
            failure(130),
            failure(131),
        ],
        900,
    );
    let answer = restart_in(&read);
    assert_ne!(answer, Restart::Clean);
    let why = why_not_known(&answer);
    assert!(
        why.contains(
            "the run opened at record 5, read only through its head restated at record 901"
        ) && why.contains("closing record at record 900"),
        "{why}"
    );
    // The walk's reading keeps the run open, with both failures.
    assert_eq!(
        interrupted_in(&read).map(|run| (run.ordinal, run.failures)),
        Some((5, 2))
    );
    // Its own close later, through its thread, settles it: clean.
    let mut settled = read.clone();
    settled.push((904, restated(&head, 5)));
    settled.push((
        905,
        closed(140, Scope::Spot, Outcome::Stored, "kite", FROM, TO),
    ));
    assert_eq!(restart_in(&settled), Restart::Clean);
}

/// **AN UNSEEN CLOSE IS MET ONCE BY A RUN** (code-7): 800 restatements of one
/// run behind one such close, interleaved with another writer's records, are
/// one run with every failure counted and one guess, NOT KNOWN; they were
/// all dropped and read clean. A second run of the same scope, window and
/// source restated behind the same close meets it as well (it may be that
/// run's close just as well), so it is a guess of its own.
#[test]
fn an_unseen_close_is_consumed_once() {
    let head = opened(100, Scope::Spot, "kite", FROM, TO, 0x5);
    let mut records = vec![closed(120, Scope::Spot, Outcome::Stored, "kite", FROM, TO)];
    for offset in 0..800 {
        records.push(restated(&head, 5));
        records.push(failure(200 + offset));
        records.push(opened(
            200 + offset,
            Scope::Spot,
            "other",
            20_020,
            20_030,
            0x9,
        ));
        records.push(closed(
            200 + offset,
            Scope::Spot,
            Outcome::Stored,
            "other",
            20_020,
            20_030,
        ));
    }
    let read = numbered(records, 1_000);
    let walked = walk(&read);
    assert_eq!(walked.guessed.len(), 1, "{:?}", walked.guessed);
    assert_eq!(
        walked
            .open
            .as_ref()
            .map(|run| (run.ordinal, run.failures, run.open_count)),
        Some((5, 800, 1))
    );
    assert_eq!(restart_in(&read).word(), "not_known");

    // Two runs restated behind one close: two guesses, both live.
    let other = opened(100, Scope::Spot, "kite", FROM, TO, 0x7);
    let read = numbered(
        vec![
            closed(120, Scope::Spot, Outcome::Stored, "kite", FROM, TO),
            restated(&head, 5),
            failure(130),
            restated(&other, 7),
            failure(140),
        ],
        900,
    );
    let walked = walk(&read);
    assert_eq!(walked.guessed.len(), 2, "{:?}", walked.guessed);
    assert_eq!(walked.open.as_ref().map(|run| run.open_count), Some(2));
}

/// The restated-head shapes that were right stay right: a head restated
/// behind its OWN run's close (read) is no new run; a head of an open run
/// rejoins it whatever closed before; a duplicate close with no run left to
/// close closes nothing.
#[test]
fn restated_heads_that_were_right_stay_right() {
    let head = opened(100, Scope::Spot, "kite", FROM, TO, 0x5);
    let behind_own = numbered(
        vec![
            head.clone(),
            failure(110),
            closed(120, Scope::Spot, Outcome::Stored, "kite", FROM, TO),
            restated(&head, 0),
            failure(121),
        ],
        0,
    );
    assert_eq!(restart_in(&behind_own), Restart::Clean);
    let duplicate = numbered(
        vec![
            head.clone(),
            closed(120, Scope::Spot, Outcome::Stored, "kite", FROM, TO),
            closed(120, Scope::Spot, Outcome::Stored, "kite", FROM, TO),
        ],
        0,
    );
    assert_eq!(restart_in(&duplicate), Restart::Clean);
    assert_eq!(restart_in(&[]), Restart::Clean, "an empty journal");
}

// ---------------------------------------------------------------------------
// the journal itself: missing, unreadable, torn, past the scan
// ---------------------------------------------------------------------------

/// **THE JOURNAL'S OWN EXTREMES.** Missing: no run was ever journalled, so
/// "clean" (nothing to read is not a failure to read). Unreadable: NOT
/// KNOWN. A torn tail (a close cut mid-write): the run it would have closed
/// is open. The open run at record 801 of an 800-record scan: read through
/// the index, named.
#[test]
fn the_journals_own_extremes() {
    use std::os::unix::fs::PermissionsExt as _;
    let missing = fresh_root("missing");
    assert_eq!(
        restart_scan(&crate::audit::Journal::at(&missing)),
        Restart::Clean
    );

    let root = fresh_root("unreadable");
    let journal = journal_at(&root, &[opened(100, Scope::Spot, "kite", FROM, TO, 1)]);
    let file = crate::audit::journal_path(&root);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    let answer = restart_scan(&journal);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).expect("chmod");
    assert_eq!(answer.word(), "not_known", "{}", answer.words());

    let root = fresh_root("torn");
    let journal = journal_at(&root, &[opened(100, Scope::Spot, "ag01-torn", FROM, TO, 1)]);
    let file = crate::audit::journal_path(&root);
    let close = closed(120, Scope::Spot, Outcome::Stored, "ag01-torn", FROM, TO).image();
    let mut bytes = std::fs::read(&file).expect("read");
    bytes.extend_from_slice(&close[..crate::audit::RECORD_LEN / 2]);
    std::fs::write(&file, bytes).expect("write");
    let run = restart_scan(&journal)
        .interrupted()
        .cloned()
        .expect("the close never landed whole");
    assert_eq!((run.ordinal, run.source.as_str()), (0, "ag01-torn"));

    let mut records = vec![opened(100, Scope::Spot, "ag01-801", FROM, TO, 1)];
    records.extend((0..INTERRUPTED_SCAN).map(|offset| failure(200 + offset)));
    let journal = journal_of("record-801", &records);
    let run = restart_scan(&journal)
        .interrupted()
        .cloned()
        .expect("found through the index");
    assert_eq!(
        (run.ordinal, run.failures, run.open_count, run.runs_unread),
        (0, INTERRUPTED_SCAN, 1, 0)
    );
}

// ---------------------------------------------------------------------------
// code-2: the leg no answer covers
// ---------------------------------------------------------------------------

/// **THE RESTART WORDS NAME THE UNJOURNALLED ARCHIVE LEG** (code-2, its
/// restart half): an archive-folder spot leg writes no opening record
/// (`server::local_answer` lands it and writes only its close), so one cut
/// short is invisible to the scan. Every answer, and `/pull/run.json`'s
/// restart object, says so, naming the feeds from the vendor table.
#[test]
fn the_restart_words_name_the_unjournalled_archive_leg() {
    let folders = pull::vendor::Feed::ALL
        .into_iter()
        .filter(|feed| {
            matches!(
                feed.descriptor().transport.kind(),
                pull::vendor::SourceKind::Folder
            )
        })
        .collect::<Vec<_>>();
    assert!(
        folders.contains(&pull::vendor::Feed::Gdfl),
        "GDFL is an archive-folder feed today: {folders:?}"
    );
    let open = restart_in(&numbered(
        vec![opened(100, Scope::Spot, "kite", FROM, TO, 1)],
        0,
    ));
    for answer in [
        Restart::Clean,
        open,
        Restart::NotKnown("a reason".to_owned()),
    ] {
        let words = answer.words();
        assert!(
            words.contains("writes no opening record") && words.contains("never reported here"),
            "{words}"
        );
        for feed in &folders {
            assert!(words.contains(feed.display()), "{words}");
        }
        let doc = document(&answer);
        let wires = doc["restart"]["unjournalledFeeds"]
            .as_array()
            .expect("an array")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>();
        assert_eq!(
            wires,
            folders.iter().map(|feed| feed.wire()).collect::<Vec<_>>()
        );
    }
    // The run's own sentence is unchanged: the caveat is the answer's.
    let run = opened(100, Scope::Spot, "kite", FROM, TO, 1);
    let found = interrupted_in(&numbered(vec![run], 0)).expect("open");
    assert!(!found.words().contains("writes no opening record"));
}

// ---------------------------------------------------------------------------
// cost
// ---------------------------------------------------------------------------

/// **WHAT THE SCAN AND THE DOCUMENT COST WITH 64 RUNS OPEN** (timing
/// evidence, run by hand): `restart_scan` over an 8,001-record journal whose
/// newest 7,937 records are failures behind 64 opening records (800 read,
/// the 64 run records through the index), and `status_json` carrying that
/// answer. p50, p99 and max over n = 200 each, printed.
#[test]
#[ignore = "timing evidence, run by hand: cargo test -p api --release --lib restart_cost -- --ignored --nocapture"]
fn restart_cost_with_64_open_runs() {
    let spread = cost_spread;
    let mut records = (0..64_u64)
        .map(|run| {
            opened(
                100 + run,
                Scope::Spot,
                &format!("ag01-cost-{run}"),
                FROM,
                TO,
                run,
            )
        })
        .collect::<Vec<_>>();
    records.extend((0..8_001 - 64).map(|offset| failure(1_000 + offset)));
    let journal = journal_of("cost-64", &records);
    let answer = restart_scan(&journal);
    let run = answer.interrupted().expect("open");
    assert_eq!((run.open_count, run.also_open.len()), (64, 63));
    let time = cost_timed;
    spread(
        "restart_scan (8,001 records, 64 runs open)",
        time(&mut || assert!(restart_scan(&journal).interrupted().is_some())),
    );
    spread(
        "status_json (64 runs open)",
        time(&mut || {
            let _ = std::hint::black_box(status_json(
                &Progress::default(),
                Some(20_738),
                Some(&answer),
            ));
        }),
    );
    let one = journal_of(
        "cost-1",
        &core::iter::once(opened(100, Scope::Spot, "ag01-cost-one", FROM, TO, 1))
            .chain((0..8_000).map(|offset| failure(1_000 + offset)))
            .collect::<Vec<_>>(),
    );
    spread(
        "restart_scan (8,001 records, 1 run open)",
        time(&mut || assert!(restart_scan(&one).interrupted().is_some())),
    );
}

/// p50, p99 and max of `nanos`, printed as microseconds with `what` and n.
fn cost_spread(what: &str, mut nanos: Vec<u128>) {
    nanos.sort_unstable();
    let pick = |q: usize| nanos[(nanos.len() * q / 100).min(nanos.len() - 1)];
    println!(
        "{what}: n={} p50={}us p99={}us max={}us",
        nanos.len(),
        pick(50) / 1_000,
        pick(99) / 1_000,
        nanos[nanos.len() - 1] / 1_000
    );
}

/// The wall time of 200 sequential runs of `work`, in nanoseconds each.
fn cost_timed(work: &mut dyn FnMut()) -> Vec<u128> {
    (0..200)
        .map(|_| {
            let began = std::time::Instant::now();
            work();
            began.elapsed().as_nanos()
        })
        .collect()
}

/// **THE WALK'S WORST CASE AND THE LARGEST GUESSED ANSWER** (timing
/// evidence, run by hand, the second check's P5 shape on the final code):
/// every record read is a run record of one F&O key. 432 walks of 432 asks
/// open, then 432 closes of the key: every close but the first is a pairing
/// guess, and the counts pass looks at every guess's later records and
/// earlier guesses. Every walk ends, so the answer is clean. Then 432 walks
/// and one close: a guessed answer counting 431 surely open, and
/// `status_json` carrying it. p50, p99 and max over n = 200 each, printed.
#[test]
#[ignore = "timing evidence, run by hand: cargo test -p api --release --lib restart_cost -- --ignored --nocapture"]
fn restart_cost_of_the_walks_worst_case() {
    let (spread, time) = (cost_spread, cost_timed);
    let walks = |closes: u64| {
        let mut records = (0..432_u64)
            .map(|walk| opened(100 + walk, Scope::Fno, "ag01-cost-NIFTY", FROM, TO, walk))
            .collect::<Vec<_>>();
        records.extend((0..closes).map(|close| {
            closed(
                1_000 + close,
                Scope::Fno,
                Outcome::Stored,
                "ag01-cost-NIFTY",
                FROM,
                TO,
            )
        }));
        records
    };
    let paired = journal_of("cost-guess-432", &walks(432));
    assert_eq!(restart_scan(&paired), Restart::Clean);
    spread(
        "restart_scan (864 run records of one F&O key, 432 closes, every close a guess)",
        time(&mut || assert_eq!(restart_scan(&paired).word(), "clean")),
    );
    // A GUESSED ANSWER AT ITS LARGEST: 432 walks open and ONE close of the
    // key, so 431 are surely open, which is a count; the document carries
    // the reading, 64 guessed ordinals and the count's 432.
    let guessed = journal_of("cost-guess-1", &walks(1));
    let answer = restart_scan(&guessed);
    assert_eq!(guessed_of(&answer).at_least_open(), 431);
    spread(
        "restart_scan (433 run records of one F&O key, one close: a guessed answer)",
        time(&mut || assert_eq!(restart_scan(&guessed).word(), "not_known")),
    );
    spread(
        "status_json (that guessed answer)",
        time(&mut || {
            let _ = std::hint::black_box(status_json(
                &Progress::default(),
                Some(20_738),
                Some(&answer),
            ));
        }),
    );
    println!(
        "guessed answer: words {} bytes, document {} bytes",
        answer.words().len(),
        status_json(&Progress::default(), Some(20_738), Some(&answer)).len()
    );
}
