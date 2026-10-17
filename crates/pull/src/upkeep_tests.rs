//! Tests of the upkeep run (R3-17, D-5831): the journal's lines read back
//! strictly, one run of a kind at a time, Stop by flag and by request, resume
//! from a chain, the torn tail, a journal that stops taking lines, workers
//! that build every item once, the status a page reads, and the months a
//! scope holds. Every store and every bar is invented here.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    reason = "a test that cannot panic cannot fail; a builder is handed its item by reference"
)]

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{
    Ask, Begun, Entry, How, Journal, KEY_CAP, Kind, Lacks, Lines, MAX_WORKERS, Month, Outcome,
    SYNC_EVERY, Scope, State, Status, TAIL_BYTES, VOID, WHY_CAP, drive, journal_name, key_refused,
    months, one_line, one_word, parse, plan_digest, request_stop, run, status, stop_name,
    whole_lines,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A scratch store of this test's own, removed when it drops.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-pull-upkeep-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn journal(&self, kind: Kind) -> String {
        fs::read_to_string(self.0.join(journal_name(Vendor::Gdfl, kind))).unwrap_or_default()
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A plan of `n` invented items, each its key and its index.
fn keys(n: usize) -> Vec<(String, usize)> {
    (0..n)
        .map(|k| (format!("bars/gdfl/item-{k:03}.bin"), k))
        .collect()
}

/// The keys of a plan, in order.
fn names(plan: &[(String, usize)]) -> Vec<String> {
    plan.iter().map(|(key, _)| key.clone()).collect()
}

/// A plan's digest, as its `run` line states it.
fn digest(plan: &[(String, usize)]) -> u64 {
    plan_digest(plan.iter().map(|(key, _)| key.as_str()))
}

fn clean(key: &str, _: &usize) -> Outcome {
    Outcome::clean("built", format!("{key} built"))
}

/// An ask for `kind` with a fixed clock, so the run id is known.
fn ask<'a>(
    kind: Kind,
    workers: usize,
    stop: &'a AtomicBool,
    now: &'a (dyn Fn() -> u64 + Sync),
) -> Ask<'a> {
    Ask {
        vendor: Vendor::Gdfl,
        kind,
        workers,
        resume: None,
        commit: Some("0123456789abcdef"),
        stop,
        now,
    }
}

fn entries(text: &str) -> Vec<Entry> {
    text.lines().filter_map(parse).collect()
}

/// Every key the journal names on an `item` line of run `id`, in file order.
fn items_of(text: &str, id: u64) -> Vec<String> {
    entries(text)
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Item { id: of, key, .. } if of == id => Some(key),
            _ => None,
        })
        .collect()
}

// ═══ words and lines ════════════════════════════════════════════════════════

/// Each kind's word is its own, read back to the same kind; only the three
/// indexes are per-month, each with its sidecar; a per-month kind starts
/// between one and the cap of workers, any other kind one.
#[test]
fn every_kind_has_its_word_its_sidecar_and_its_worker_count() {
    let words: Vec<&str> = Kind::ALL.iter().map(|kind| kind.word()).collect();
    assert_eq!(words, ["tix", "srt", "msum", "recount", "rekey"]);
    for kind in Kind::ALL {
        assert_eq!(Kind::parse(kind.word()), Some(kind));
    }
    assert_eq!(Kind::parse("sort"), None);
    assert_eq!(Kind::parse(""), None);
    assert_eq!(Kind::DayIndex.sidecar(), Some(FileKind::TimeIndex));
    assert_eq!(Kind::SortIndex.sidecar(), Some(FileKind::SortIndex));
    assert_eq!(Kind::MonthSummary.sidecar(), Some(FileKind::MonthSummary));
    assert_eq!(Kind::Recount.sidecar(), None);
    assert_eq!(Kind::Rekey.sidecar(), None);
    for kind in [Kind::DayIndex, Kind::SortIndex, Kind::MonthSummary] {
        assert!(kind.per_month());
        assert_eq!(kind.workers(0), 1);
        assert_eq!(kind.workers(1), 1);
        assert_eq!(kind.workers(8), 8);
        assert_eq!(kind.workers(MAX_WORKERS), MAX_WORKERS);
        assert_eq!(kind.workers(MAX_WORKERS + 1), MAX_WORKERS);
    }
    for kind in [Kind::Recount, Kind::Rekey] {
        assert!(!kind.per_month());
        assert_eq!(kind.workers(8), 1);
        assert_eq!(kind.workers(0), 1);
    }
    assert_eq!(
        journal_name(Vendor::Gdfl, Kind::SortIndex),
        "audit/upkeep.gdfl.srt.log"
    );
    assert_eq!(
        stop_name(Vendor::Zerodha, Kind::Recount),
        "audit/upkeep.zerodha.recount.stop"
    );
    assert_eq!(How::Done.word(), "done");
    assert_eq!(How::Stopped.word(), "stopped");
    assert_eq!(How::Failed.word(), "failed");
}

/// The three lines this module writes are read back exactly; a void line, a
/// line of another format, an unknown head, a missing or misnamed field, an
/// extra word on a `run` line and a flag that is not 0 or 1 are no entry.
#[test]
fn a_journal_line_is_read_strictly() {
    let run = "run 7 v1 vendor=gdfl kind=srt commit=abc workers=4 asked=8 items=10 skipped=2 \
               plan=00000000000000ff resume=3 at=99";
    assert_eq!(
        parse(run),
        Some(Entry::Run {
            id: 7,
            resume: Some(3),
            items: 10,
            skipped: 2,
            workers: 4,
            at: 99,
        })
    );
    let fresh = run.replace("resume=3", "resume=-");
    assert!(matches!(
        parse(&fresh),
        Some(Entry::Run { resume: None, .. })
    ));
    let item = "item 7 n=2 ok=1 of=10 t=5 clean=0 word=REFUSED key=bars/x.bin why=held by a writer";
    assert_eq!(
        parse(item),
        Some(Entry::Item {
            id: 7,
            n: 2,
            ok: 1,
            of: 10,
            t: 5,
            clean: false,
            key: "bars/x.bin".to_owned(),
        })
    );
    assert!(matches!(
        parse(&item.replace("clean=0", "clean=1")),
        Some(Entry::Item { clean: true, .. })
    ));
    let end = "end 7 how=stopped n=2 ok=1 of=10 t=6 why=a Stop was asked";
    assert_eq!(
        parse(end),
        Some(Entry::End {
            id: 7,
            how: How::Stopped,
            n: 2,
            ok: 1,
            of: 10,
            t: 6,
        })
    );
    for how in ["done", "failed"] {
        assert!(parse(&end.replace("stopped", how)).is_some(), "{how}");
    }
    assert_eq!(parse(end).map(|entry| entry.id()), Some(7));
    for bad in [
        String::new(),
        format!("{run}{VOID}"),
        run.replace(" v1 ", " v2 "),
        format!("{run} extra"),
        run.replace("run 7", "run x"),
        run.replace("workers=4", "workers=four"),
        run.replace("vendor=", "feed="),
        run.replace("kind=", "what="),
        run.replace("commit=", "stamp="),
        run.replace("asked=8", "asked=x"),
        run.replace("items=10", "items=x"),
        run.replace("skipped=2", "skipped=x"),
        run.replace("plan=", "digest="),
        run.replace("resume=3", "resume=x"),
        run.replace("at=99", "at=x"),
        run.replace(" at=99", ""),
        item.replace("clean=0", "clean=2"),
        item.replace("n=2", "n=x"),
        item.replace("ok=1", "ok=x"),
        item.replace("of=10", "of=x"),
        item.replace("t=5", "t=x"),
        item.replace("word=", "said="),
        item.replace("key=", "path="),
        item.replace("why=", "because="),
        item.replace("clean=", "tidy="),
        end.replace("how=stopped", "how=paused"),
        end.replace("n=2", "n=x"),
        end.replace("ok=1", "ok=x"),
        end.replace("of=10", "of=x"),
        end.replace("t=6", "t=x"),
        end.replace("why=", "because="),
        "begin 7 n=1".to_owned(),
        "run".to_owned(),
    ] {
        assert_eq!(parse(&bad), None, "{bad}");
    }
}

/// A reason is one line, cut at the cap on a character boundary with the cut
/// said; a word is one word; a key is refused empty, too long or holding a
/// space or a control character.
#[test]
fn reasons_words_and_keys_keep_a_line_whole() {
    assert_eq!(one_line("a\nb\rc"), "a b c");
    let long = "é".repeat(WHY_CAP);
    let cut = one_line(&long);
    assert!(
        cut.ends_with(&format!("...(+{} bytes)", long.len() - WHY_CAP)),
        "{cut}"
    );
    assert!(cut.len() < long.len());
    assert_eq!(one_line(&"x".repeat(WHY_CAP)), "x".repeat(WHY_CAP));
    let odd = format!("{}é", "x".repeat(WHY_CAP - 1));
    assert_eq!(
        one_line(&odd),
        format!("{}...(+2 bytes)", "x".repeat(WHY_CAP - 1))
    );
    assert_eq!(one_word("not built"), "not_built");
    assert_eq!(one_word("a\tb"), "a_b");
    assert_eq!(
        one_word("a\u{7}b"),
        "a_b",
        "a control character that is no space"
    );
    assert_eq!(one_word(""), "-");
    assert_eq!(one_word(&"w".repeat(40)).len(), 32);
    assert!(key_refused("bars/gdfl/x.bin").is_none());
    assert!(key_refused(&"k".repeat(KEY_CAP)).is_none());
    assert!(key_refused("").is_some());
    assert!(
        key_refused(&"k".repeat(KEY_CAP + 1))
            .unwrap()
            .contains("bytes")
    );
    assert!(key_refused("a b").unwrap().contains("space"));
    assert!(key_refused("a\u{7}b").is_some());
    assert!(key_refused("a\nb").is_some());
    let plan = keys(3);
    let mut swapped = plan.clone();
    swapped.swap(0, 1);
    assert_eq!(digest(&plan), digest(&keys(3)));
    assert_ne!(digest(&plan), digest(&swapped));
    assert_ne!(digest(&plan), digest(&keys(2)));
    assert_eq!(
        digest(&[]),
        brutex_core::universe::fnv1a(""),
        "the empty plan digests the empty text"
    );
}

/// A tail cut off its file drops its first piece, a fragment no newline ends
/// is never a line, and a piece that is not text is skipped.
#[test]
fn a_tail_yields_only_whole_lines() {
    let lines: Vec<&str> = whole_lines(b"one\ntwo\nthr", false).collect();
    assert_eq!(lines, ["one", "two"]);
    let lines: Vec<&str> = whole_lines(b"one\ntwo\n", true).collect();
    assert_eq!(lines, ["two"]);
    let lines: Vec<&str> = whole_lines(b"\xff\xfe\nok\n", false).collect();
    assert_eq!(lines, ["ok"]);
    assert_eq!(whole_lines(b"", false).count(), 0);
}

// ═══ a run ══════════════════════════════════════════════════════════════════

/// One run over five items: each built once, in plan order, one `item` line
/// each carrying the running totals, a `run` line and an `end` line that say
/// what the run was, and a status that reads the end.
#[test]
fn a_run_builds_every_item_once_and_says_so_line_by_line() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 1_000;
    let plan = keys(5);
    let failing = |key: &str, &at: &usize| {
        if at == 3 {
            Outcome::failed("REFUSED", "held by a writer")
        } else {
            clean(key, &at)
        }
    };
    let ran = run(
        &root.0,
        &ask(Kind::SortIndex, 1, &stop, &now),
        &plan,
        &failing,
    )
    .unwrap();
    assert_eq!(ran.id, 1_000);
    assert_eq!(ran.resumed, None);
    assert_eq!(ran.workers, 1);
    assert_eq!(ran.skipped, 0);
    assert_eq!(ran.how, How::Done);
    assert_eq!(ran.built(), 5);
    assert_eq!(ran.clean(), 4);
    assert_eq!(ran.left(), 0);
    assert!(!ran.whole(), "an item that failed leaves the run not whole");
    assert_eq!(ran.torn, 0);
    assert_eq!(ran.outcomes[3].as_ref().unwrap().word, "REFUSED");
    assert_eq!(
        ran.outcomes[0].as_ref().unwrap().why,
        format!("{} built", plan[0].0)
    );
    let text = root.journal(Kind::SortIndex);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 7, "{text}");
    assert!(
        lines[0].starts_with(
            "run 1000 v1 vendor=gdfl kind=srt commit=0123456789abcdef workers=1 asked=1 items=5 \
             skipped=0 plan="
        ),
        "{}",
        lines[0]
    );
    assert!(lines[0].ends_with(" resume=- at=1000"), "{}", lines[0]);
    assert!(lines[0].contains(&format!("plan={:016x}", digest(&plan))));
    assert_eq!(items_of(&text, 1_000), names(&plan));
    assert!(
        lines[4].starts_with("item 1000 n=4 ok=3 of=5 t="),
        "{}",
        lines[4]
    );
    assert!(
        lines[4].contains(" clean=0 word=REFUSED key=bars/gdfl/item-003.bin why=held by a writer")
    );
    assert!(
        lines[6].starts_with("end 1000 how=done n=5 ok=4 of=5 t="),
        "{}",
        lines[6]
    );
    assert!(lines[6].ends_with("why=every item of the run was built"));
    let found = status(&root.0, Vendor::Gdfl, Kind::SortIndex)
        .unwrap()
        .unwrap();
    assert_eq!(found.id, 1_000);
    assert_eq!(found.state, State::Ended(How::Done));
    assert_eq!((found.n, found.ok, found.of), (5, 4, 5));
    assert_eq!(found.eta(), None, "an ended run has no time left");
    // A RUN OF ANOTHER KIND HAS ITS OWN JOURNAL AND ITS OWN STATUS.
    assert_eq!(status(&root.0, Vendor::Gdfl, Kind::DayIndex).unwrap(), None);
    let whole = run(&root.0, &ask(Kind::DayIndex, 1, &stop, &now), &plan, &clean).unwrap();
    assert!(whole.whole());
    assert_eq!(whole.id, 1_000, "ids are per journal");
}

/// The id is the clock's seconds, or one more than the newest id the journal
/// holds when the clock is not later, so no two runs share one whatever the
/// clock says. An unstamped build says so on its `run` line.
#[test]
fn a_run_id_is_never_reused_whatever_the_clock_says() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let early = || 50;
    let plan = keys(1);
    let first = run(
        &root.0,
        &ask(Kind::MonthSummary, 1, &stop, &early),
        &plan,
        &clean,
    )
    .unwrap();
    assert_eq!(first.id, 50);
    let mut unstamped = ask(Kind::MonthSummary, 1, &stop, &early);
    unstamped.commit = None;
    let second = run(&root.0, &unstamped, &plan, &clean).unwrap();
    assert_eq!(second.id, 51);
    let later = || 500;
    let third = run(
        &root.0,
        &ask(Kind::MonthSummary, 1, &stop, &later),
        &plan,
        &clean,
    )
    .unwrap();
    assert_eq!(third.id, 500);
    let text = root.journal(Kind::MonthSummary);
    assert!(
        text.contains("run 51 v1 vendor=gdfl kind=msum commit=unstamped "),
        "{text}"
    );
}

/// A key the journal cannot hold, or a key twice, refuses the run before its
/// journal exists: nothing is built and nothing written.
#[test]
fn a_plan_the_journal_cannot_hold_is_refused_before_anything_is_written() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 1;
    let built = AtomicUsize::new(0);
    let counting = |key: &str, &at: &usize| {
        built.fetch_add(1, Ordering::Relaxed);
        clean(key, &at)
    };
    let spaced = vec![("bars/a.bin".to_owned(), 0), ("bars/b c.bin".to_owned(), 1)];
    let why = run(
        &root.0,
        &ask(Kind::SortIndex, 1, &stop, &now),
        &spaced,
        &counting,
    )
    .unwrap_err();
    assert!(
        why.contains("space") && why.contains("nothing was written"),
        "{why}"
    );
    let twice = vec![("bars/a.bin".to_owned(), 0), ("bars/a.bin".to_owned(), 1)];
    let why = run(
        &root.0,
        &ask(Kind::SortIndex, 1, &stop, &now),
        &twice,
        &counting,
    )
    .unwrap_err();
    assert!(why.contains("twice"), "{why}");
    assert_eq!(built.load(Ordering::Relaxed), 0);
    assert!(
        !root
            .0
            .join(journal_name(Vendor::Gdfl, Kind::SortIndex))
            .exists()
    );
    // An empty plan is a run of nothing: its two lines, and done.
    let empty = run(
        &root.0,
        &ask(Kind::SortIndex, 8, &stop, &now),
        &[],
        &counting,
    )
    .unwrap();
    assert_eq!((empty.workers, empty.how, empty.built()), (1, How::Done, 0));
    assert!(empty.whole());
}

/// **ONE RUN OF A KIND AT A TIME.** While a run holds the journal's lock a
/// second run of the same vendor and kind is refused by name and builds
/// nothing, and the status reads the holder as running, with a time left
/// extrapolated from its rate.
#[test]
fn a_second_run_of_a_kind_is_refused_while_the_first_holds_it() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 10;
    let held = Journal::open(&root.0, Vendor::Gdfl, Kind::SortIndex).unwrap();
    let why = run(
        &root.0,
        &ask(Kind::SortIndex, 1, &stop, &now),
        &keys(2),
        &clean,
    )
    .unwrap_err();
    assert!(
        why.contains("another srt upkeep run of gdfl holds")
            && why.contains("one run of a kind at a time"),
        "{why}"
    );
    drop(held);
    // A run that died mid-way: a `run` line and two `item` lines, no `end`.
    let path = root.0.join(journal_name(Vendor::Gdfl, Kind::SortIndex));
    fs::write(
        &path,
        "run 9 v1 vendor=gdfl kind=srt commit=x workers=1 asked=1 items=4 skipped=0 \
         plan=0000000000000000 resume=- at=9\n\
         item 9 n=1 ok=1 of=4 t=10 clean=1 word=built key=k0 why=w\n\
         item 9 n=2 ok=2 of=4 t=20 clean=1 word=built key=k1 why=w\n",
    )
    .unwrap();
    let died = status(&root.0, Vendor::Gdfl, Kind::SortIndex)
        .unwrap()
        .unwrap();
    assert_eq!(died.state, State::Died);
    assert_eq!((died.id, died.n, died.of, died.t), (9, 2, 4, 20));
    assert_eq!(died.eta(), None, "a dead run has no time left");
    let held = Journal::open(&root.0, Vendor::Gdfl, Kind::SortIndex).unwrap();
    let running = status(&root.0, Vendor::Gdfl, Kind::SortIndex)
        .unwrap()
        .unwrap();
    assert_eq!(running.state, State::Running);
    assert_eq!(running.eta(), Some(20), "20 s for two of four: 20 s more");
    drop(held);
    let begun = Status {
        id: 1,
        state: State::Running,
        n: 0,
        ok: 0,
        of: 9,
        t: 3,
    };
    assert_eq!(begun.eta(), None, "no rate before the first item");
    // A `run` line alone: nothing built yet.
    fs::write(
        &path,
        "run 12 v1 vendor=gdfl kind=srt commit=x workers=1 asked=1 items=4 skipped=0 \
         plan=0000000000000000 resume=- at=12\n",
    )
    .unwrap();
    let fresh = status(&root.0, Vendor::Gdfl, Kind::SortIndex)
        .unwrap()
        .unwrap();
    assert_eq!(
        (fresh.id, fresh.n, fresh.of, fresh.state),
        (12, 0, 4, State::Died)
    );
}

/// **STOP BY THE CALLER'S FLAG.** The run takes no item once the flag is
/// set, the item in hand is finished, and it ends `stopped` with what is left
/// said; a RESUME then builds exactly what was left, and a resume of that
/// resume builds what IT left, skipping what either closed clean.
#[test]
fn a_stopped_run_resumes_along_its_chain() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 100;
    let plan = keys(10);
    let built = AtomicUsize::new(0);
    let stopping = |key: &str, &at: &usize| {
        if built.fetch_add(1, Ordering::Relaxed) + 1 == 3 {
            stop.store(true, Ordering::Release);
        }
        clean(key, &at)
    };
    let first = run(
        &root.0,
        &ask(Kind::SortIndex, 1, &stop, &now),
        &plan,
        &stopping,
    )
    .unwrap();
    assert_eq!(first.how, How::Stopped);
    assert_eq!((first.built(), first.left()), (3, 7));
    assert!(
        first.why.contains("7 item(s) left for a resume of run 100"),
        "{}",
        first.why
    );
    assert!(!first.whole());
    let text = root.journal(Kind::SortIndex);
    assert!(
        text.contains("end 100 how=stopped n=3 ok=3 of=10 "),
        "{text}"
    );
    let found = status(&root.0, Vendor::Gdfl, Kind::SortIndex)
        .unwrap()
        .unwrap();
    assert_eq!(found.state, State::Ended(How::Stopped));
    // The resume: two of the remaining seven, then a stop again.
    stop.store(false, Ordering::Release);
    built.store(0, Ordering::Relaxed);
    let stopping_at_two = |key: &str, &at: &usize| {
        if built.fetch_add(1, Ordering::Relaxed) + 1 == 2 {
            stop.store(true, Ordering::Release);
        }
        if at == 4 {
            Outcome::failed("REFUSED", "held")
        } else {
            clean(key, &at)
        }
    };
    let mut resume = ask(Kind::SortIndex, 1, &stop, &now);
    resume.resume = Some(100);
    let second = run(&root.0, &resume, &plan, &stopping_at_two).unwrap();
    assert_eq!(second.id, 101);
    assert_eq!(second.resumed, Some(100));
    assert_eq!(second.skipped, 3);
    assert_eq!((second.built(), second.clean()), (2, 1));
    assert_eq!(second.how, How::Stopped);
    let text = root.journal(Kind::SortIndex);
    assert_eq!(items_of(&text, 101), names(&plan[3..5]));
    assert!(text.contains(" items=7 skipped=3 "), "{text}");
    assert!(text.contains(" resume=100 at=100"), "{text}");
    // The resume of the resume: the failed item 4 and the five never reached.
    stop.store(false, Ordering::Release);
    resume.resume = Some(101);
    let third = run(&root.0, &resume, &plan, &clean).unwrap();
    assert_eq!(third.id, 102);
    assert_eq!(
        third.skipped, 4,
        "items 0-3 closed clean by runs 100 and 101"
    );
    assert_eq!(third.how, How::Done);
    assert!(third.whole());
    let text = root.journal(Kind::SortIndex);
    assert_eq!(items_of(&text, 102), names(&plan[4..]));
    // EVERY ITEM OF THE PLAN WAS CLOSED CLEAN EXACTLY ONCE across the chain.
    let mut closed: Vec<String> = entries(&text)
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Item {
                clean: true, key, ..
            } => Some(key),
            _ => None,
        })
        .collect();
    closed.sort();
    assert_eq!(closed, names(&plan));
    // A fourth resume of the whole chain has nothing left.
    resume.resume = Some(102);
    let nothing = run(&root.0, &resume, &plan, &clean).unwrap();
    assert_eq!(
        (nothing.skipped, nothing.built(), nothing.how),
        (10, 0, How::Done)
    );
    // A fresh run is a chain of its own: its resume skips only what IT
    // closed, never the items the unrelated chain above closed clean.
    built.store(2, Ordering::Relaxed);
    let fresh = run(
        &root.0,
        &ask(Kind::SortIndex, 1, &stop, &now),
        &plan,
        &stopping,
    )
    .unwrap();
    assert_eq!((fresh.id, fresh.built(), fresh.how), (104, 1, How::Stopped));
    stop.store(false, Ordering::Release);
    resume.resume = Some(104);
    let own = run(&root.0, &resume, &plan, &clean).unwrap();
    assert_eq!(
        (own.skipped, own.built(), own.how),
        (1, 9, How::Done),
        "only run 104's one clean item is skipped"
    );
}

/// A resume of a run the journal does not hold is refused, builds nothing,
/// and leaves the journal as it was; a resume of a chain whose `run` lines
/// were edited into a cycle ends.
#[test]
fn a_resume_of_no_such_run_is_refused() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 5;
    let plan = keys(2);
    run(&root.0, &ask(Kind::Recount, 1, &stop, &now), &plan, &clean).unwrap();
    let before = root.journal(Kind::Recount);
    let mut resume = ask(Kind::Recount, 1, &stop, &now);
    resume.resume = Some(4);
    let why = run(&root.0, &resume, &plan, &clean).unwrap_err();
    assert!(why.contains("no run 4 in this journal"), "{why}");
    assert_eq!(root.journal(Kind::Recount), before);
    // Two runs each naming the other: the chain is the two, once each.
    let path = root.0.join(journal_name(Vendor::Gdfl, Kind::Rekey));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        "run 1 v1 vendor=gdfl kind=rekey commit=x workers=1 asked=1 items=2 skipped=0 \
         plan=0 resume=2 at=1\n\
         item 1 n=1 ok=1 of=2 t=0 clean=1 word=built key=bars/gdfl/item-000.bin why=w\n\
         a line no run of this module wrote\n\
         run 2 v1 vendor=gdfl kind=rekey commit=x workers=1 asked=1 items=2 skipped=0 \
         plan=0 resume=1 at=2\n",
    )
    .unwrap();
    resume.kind = Kind::Rekey;
    resume.resume = Some(2);
    let ran = run(&root.0, &resume, &plan, &clean).unwrap();
    assert_eq!((ran.skipped, ran.built()), (1, 1));
}

/// **STOP BY REQUEST.** `request_stop` names the running run, which sees it
/// before its next item and ends `stopped`; a request naming another run —
/// a stale one left from before — stops nothing, now or later.
#[test]
fn a_stop_request_names_its_run_and_a_stale_one_stops_nothing() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 700;
    let plan = keys(6);
    // A STALE REQUEST, for a run long gone.
    request_stop(&root.0, Vendor::Gdfl, Kind::DayIndex, 699).unwrap();
    let asking = |key: &str, &at: &usize| {
        if at == 1 {
            let path = request_stop(&root.0, Vendor::Gdfl, Kind::DayIndex, 700).unwrap();
            assert_eq!(path, root.0.join(stop_name(Vendor::Gdfl, Kind::DayIndex)));
        }
        clean(key, &at)
    };
    let ran = run(
        &root.0,
        &ask(Kind::DayIndex, 1, &stop, &now),
        &plan,
        &asking,
    )
    .unwrap();
    assert_eq!(ran.how, How::Stopped);
    assert_eq!(ran.built(), 2);
    assert!(
        !stop.load(Ordering::Acquire),
        "the caller's flag is the caller's"
    );
    assert_eq!(
        fs::read_to_string(root.0.join(stop_name(Vendor::Gdfl, Kind::DayIndex))).unwrap(),
        "stop 700\n"
    );
    // The request names run 700 and stays where it is: the next run, 701,
    // is not stopped by it.
    let next = run(&root.0, &ask(Kind::DayIndex, 1, &stop, &now), &plan, &clean).unwrap();
    assert_eq!((next.id, next.how, next.built()), (701, How::Done, 6));
    // A request file that is not text is no request.
    fs::write(
        root.0.join(stop_name(Vendor::Gdfl, Kind::DayIndex)),
        b"\xff\xfe stop",
    )
    .unwrap();
    let after = run(&root.0, &ask(Kind::DayIndex, 1, &stop, &now), &plan, &clean).unwrap();
    assert_eq!(after.how, How::Done);
    // A request whose directory cannot be made is refused by name.
    let gone = root.0.join("no-such-store");
    let why = request_stop(&gone, Vendor::Gdfl, Kind::DayIndex, 1).unwrap_err();
    assert!(
        why.starts_with(&format!("{}: ", gone.join("audit").display())),
        "the audit directory itself is named: {why}"
    );
    // A staging name that is a directory is refused, and the request with it.
    let staging = root
        .0
        .join(stop_name(Vendor::Gdfl, Kind::SortIndex))
        .with_extension("stop.new");
    fs::create_dir_all(&staging).unwrap();
    let why = request_stop(&root.0, Vendor::Gdfl, Kind::SortIndex, 1).unwrap_err();
    assert!(why.contains("stop.new"), "{why}");
}

/// **WORKERS.** Eight workers over two hundred items build each exactly once,
/// the outcomes come back in plan order, and the journal's running totals
/// rise by one a line whatever order the workers finished in.
#[test]
fn eight_workers_build_every_item_once_and_the_totals_rise_by_one() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 3;
    let plan = keys(200);
    let counts: Vec<AtomicUsize> = (0..plan.len()).map(|_| AtomicUsize::new(0)).collect();
    let counting = |key: &str, &at: &usize| {
        counts[at].fetch_add(1, Ordering::Relaxed);
        assert_eq!(key, plan[at].0);
        if at.is_multiple_of(7) {
            Outcome::failed("REFUSED", "seventh")
        } else {
            clean(key, &at)
        }
    };
    let ran = run(
        &root.0,
        &ask(Kind::SortIndex, 8, &stop, &now),
        &plan,
        &counting,
    )
    .unwrap();
    assert_eq!(ran.workers, 8);
    assert!(
        counts
            .iter()
            .all(|count| count.load(Ordering::Relaxed) == 1)
    );
    for (at, outcome) in ran.outcomes.iter().enumerate() {
        let outcome = outcome.as_ref().unwrap();
        assert_eq!(outcome.clean, !at.is_multiple_of(7), "{at}");
    }
    let text = root.journal(Kind::SortIndex);
    let mut n = 0;
    let mut ok = 0;
    for entry in entries(&text) {
        if let Entry::Item {
            n: line_n,
            ok: line_ok,
            of,
            clean,
            ..
        } = entry
        {
            n += 1;
            ok += u64::from(clean);
            assert_eq!((line_n, line_ok, of), (n, ok, 200));
        }
    }
    assert_eq!(n, 200);
    let mut seen = items_of(&text, 3);
    seen.sort();
    assert_eq!(seen, names(&plan));
    // NEVER MORE WORKERS THAN ITEMS.
    let few = run(
        &root.0,
        &ask(Kind::SortIndex, 8, &stop, &now),
        &keys(3),
        &clean,
    )
    .unwrap();
    assert_eq!(few.workers, 3);
    // AND ONE for a kind that writes shared files.
    let one = run(&root.0, &ask(Kind::Rekey, 8, &stop, &now), &plan, &clean).unwrap();
    assert_eq!(one.workers, 1);
    assert!(
        root.journal(Kind::Rekey)
            .contains(" workers=1 asked=8 items=200 ")
    );
}

/// **A TORN TAIL IS ENDED, NEVER JOINED.** A fragment a killed run left is
/// ended as a void line before anything is appended, counted, and read as
/// no entry; the next `run` line starts a line of its own.
#[test]
fn a_torn_tail_is_ended_as_a_void_line() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 8;
    let path = root.0.join(journal_name(Vendor::Gdfl, Kind::SortIndex));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "item 7 n=1 ok=1 of=4 t=0 clean=1 wo").unwrap();
    let ran = run(
        &root.0,
        &ask(Kind::SortIndex, 1, &stop, &now),
        &keys(1),
        &clean,
    )
    .unwrap();
    assert_eq!(ran.torn, 35, "the fragment's bytes");
    let text = fs::read_to_string(&path).unwrap();
    let first = text.lines().next().unwrap();
    assert_eq!(first, format!("item 7 n=1 ok=1 of=4 t=0 clean=1 wo{VOID}"));
    assert_eq!(parse(first), None);
    assert!(text.lines().nth(1).unwrap().starts_with("run 8 v1 "));
    // A journal that ends whole has nothing to end.
    let again = run(
        &root.0,
        &ask(Kind::SortIndex, 1, &stop, &now),
        &keys(1),
        &clean,
    )
    .unwrap();
    assert_eq!(again.torn, 0);
    // A reason that ends like a void line is written so it does not read as one.
    let voidish = |_: &str, _: &usize| Outcome::clean("built", format!("ends{VOID}"));
    run(
        &root.0,
        &ask(Kind::SortIndex, 1, &stop, &now),
        &keys(1),
        &voidish,
    )
    .unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains(&format!("why=ends{VOID}.\n")), "{text}");
    assert!(
        entries(&text)
            .iter()
            .filter(|e| matches!(e, Entry::Item { .. }))
            .count()
            == 3,
        "{text}"
    );
}

/// A journal far longer than the tail a status reads still gives the newest
/// run's id and totals: every `item` line carries them.
#[test]
fn the_status_of_a_long_run_comes_from_its_last_line() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 40;
    let long = |_: &str, _: &usize| Outcome::clean("built", "w".repeat(900));
    let plan = keys(usize::try_from(TAIL_BYTES / 900).unwrap() * 3);
    run(&root.0, &ask(Kind::SortIndex, 2, &stop, &now), &plan, &long).unwrap();
    let length = fs::metadata(root.0.join(journal_name(Vendor::Gdfl, Kind::SortIndex)))
        .unwrap()
        .len();
    assert!(length > 2 * TAIL_BYTES);
    let found = status(&root.0, Vendor::Gdfl, Kind::SortIndex)
        .unwrap()
        .unwrap();
    let of = u64::try_from(plan.len()).unwrap();
    assert_eq!(
        (found.id, found.n, found.of, found.state),
        (40, of, of, State::Ended(How::Done))
    );
    // A second run resumed from the first reads its whole journal and skips all.
    let mut resume = ask(Kind::SortIndex, 2, &stop, &now);
    resume.resume = Some(40);
    let second = run(&root.0, &resume, &plan, &long).unwrap();
    assert_eq!(second.skipped, plan.len());
}

/// The first piece of a tail that cut the journal may be the end of a
/// longer line, and is never read as a line of its own: here the cut falls
/// exactly where a reason's text spells a line of run 999, and the next run
/// id still follows the journal's real newest, 5.
#[test]
fn a_line_the_tail_cuts_is_never_read_as_one() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 1;
    let path = root.0.join(journal_name(Vendor::Gdfl, Kind::SortIndex));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let spelled = "item 999 n=1 ok=1 of=1 t=0 clean=1 word=built key=k why=w\n";
    assert!(
        parse(spelled.trim_end()).is_some(),
        "it reads as a line alone"
    );
    let head = format!("end 5 how=done n=0 ok=0 of=0 t=0 why=a reason that reads {spelled}");
    // The last line is sized so the tail starts where the spelled line does.
    let start = "end 5 how=done n=0 ok=0 of=0 t=0 why=";
    let pad = usize::try_from(TAIL_BYTES).unwrap() - spelled.len() - start.len() - 1;
    fs::write(&path, format!("{head}{start}{}\n", "w".repeat(pad))).unwrap();
    let ran = run(
        &root.0,
        &ask(Kind::SortIndex, 1, &stop, &now),
        &keys(1),
        &clean,
    )
    .unwrap();
    assert_eq!(ran.id, 6);
}

/// The journal's name must be a regular file of one name: a directory, a
/// link and a second name of another file are each refused by name, nothing
/// written; a store root that is not there is refused, never made.
#[test]
fn a_journal_that_is_not_a_plain_file_is_refused() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 1;
    let path = root.0.join(journal_name(Vendor::Gdfl, Kind::SortIndex));
    fs::create_dir_all(&path).unwrap();
    let why = run(
        &root.0,
        &ask(Kind::SortIndex, 1, &stop, &now),
        &keys(1),
        &clean,
    )
    .unwrap_err();
    assert!(why.contains("upkeep.gdfl.srt.log"), "{why}");
    let why = status(&root.0, Vendor::Gdfl, Kind::SortIndex).unwrap_err();
    assert!(
        why.contains("not a regular file") || why.contains("upkeep.gdfl.srt.log"),
        "{why}"
    );
    let target = root.0.join("elsewhere.log");
    fs::write(&target, "").unwrap();
    let linked = root.0.join(journal_name(Vendor::Gdfl, Kind::DayIndex));
    std::os::unix::fs::symlink(&target, &linked).unwrap();
    let why = run(
        &root.0,
        &ask(Kind::DayIndex, 1, &stop, &now),
        &keys(1),
        &clean,
    )
    .unwrap_err();
    assert!(why.contains("never a link"), "{why}");
    let why = status(&root.0, Vendor::Gdfl, Kind::DayIndex).unwrap_err();
    assert!(why.contains("never a link"), "{why}");
    let second = root.0.join(journal_name(Vendor::Gdfl, Kind::MonthSummary));
    fs::hard_link(&target, &second).unwrap();
    let why = run(
        &root.0,
        &ask(Kind::MonthSummary, 1, &stop, &now),
        &keys(1),
        &clean,
    )
    .unwrap_err();
    assert!(why.contains("2 names"), "{why}");
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "",
        "nothing written through either name"
    );
    let gone = root.0.join("not-a-store");
    let why = run(
        &gone,
        &ask(Kind::SortIndex, 1, &stop, &now),
        &keys(1),
        &clean,
    )
    .unwrap_err();
    assert!(
        why.starts_with(&format!("{}: ", gone.join("audit").display()))
            && why.ends_with("; nothing was written"),
        "{why}"
    );
    assert!(!gone.exists(), "the root is never made");
    // A journal with no line a reader knows has no status.
    let blank = root.0.join(journal_name(Vendor::Gdfl, Kind::Recount));
    fs::write(&blank, "nothing this module wrote\n").unwrap();
    assert_eq!(status(&root.0, Vendor::Gdfl, Kind::Recount).unwrap(), None);
}

/// Lines that stop being taken partway: the journal of a real run, then a
/// refusal after `lines` of them, or of the `end` line alone.
struct Failing {
    inner: Journal,
    lines: usize,
    end_fails: bool,
}

impl Lines for Failing {
    fn line(&mut self, text: &str) -> Result<(), String> {
        if text.starts_with("end ") && self.end_fails {
            return Err("no space left on device (end)".to_owned());
        }
        if self.lines == 0 {
            return Err("no space left on device".to_owned());
        }
        self.lines -= 1;
        self.inner.line(text)
    }

    fn sync(&mut self) -> Result<(), String> {
        Lines::sync(&mut self.inner)
    }
}

/// What `run` settles before `drive`, for a fresh run.
fn begun(id: u64, plan: &[(String, usize)]) -> Begun<'_, usize> {
    Begun {
        id,
        plan,
        done: None,
        torn: 0,
    }
}

/// **A JOURNAL THAT STOPS TAKING LINES STOPS THE RUN.** No item is counted
/// that no line records: the run ends `failed`, naming the refusal, with the
/// items whose lines were written and nothing else; an `end` line refused is
/// said too; a `run` line refused builds nothing at all.
#[test]
fn a_journal_that_stops_taking_lines_fails_the_run_and_counts_only_what_it_recorded() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 60;
    let plan = keys(6);
    let failing = |lines: usize, end_fails: bool| Failing {
        inner: Journal::open(&root.0, Vendor::Gdfl, Kind::SortIndex).unwrap(),
        lines,
        end_fails,
    };
    let asked = ask(Kind::SortIndex, 1, &stop, &now);
    // The `run` line refused: an error, nothing built.
    let built = AtomicUsize::new(0);
    let counting = |key: &str, &at: &usize| {
        built.fetch_add(1, Ordering::Relaxed);
        clean(key, &at)
    };
    let mut lines = failing(0, false);
    let why = drive(&mut lines, &root.0, &asked, begun(60, &plan), &counting).unwrap_err();
    assert!(why.contains("no space left"), "{why}");
    drop(lines);
    assert_eq!(built.load(Ordering::Relaxed), 0);
    // The third item's line refused: two counted, the run failed.
    let mut lines = failing(3, false);
    let ran = drive(&mut lines, &root.0, &asked, begun(60, &plan), &counting).unwrap();
    drop(lines);
    assert_eq!(ran.how, How::Failed);
    assert!(
        ran.why
            .contains("the journal stopped taking lines: no space left"),
        "{}",
        ran.why
    );
    assert_eq!(ran.built(), 2);
    assert!(
        ran.outcomes[2].is_none(),
        "built, but no line says so: left for a resume"
    );
    assert_eq!(ran.left(), 4);
    assert_eq!(
        built.load(Ordering::Relaxed),
        3,
        "the third was built, then refused"
    );
    assert!(
        ran.why
            .contains("— and the end line was not written: no space left"),
        "{}",
        ran.why
    );
    let text = root.journal(Kind::SortIndex);
    assert_eq!(
        text.lines().count(),
        3,
        "the run line and two item lines: {text}"
    );
    // The `end` line refused: said beside why the run ended.
    let mut lines = failing(usize::MAX, true);
    let ran = drive(&mut lines, &root.0, &asked, begun(61, &plan), &clean).unwrap();
    drop(lines);
    assert_eq!(ran.how, How::Failed);
    assert!(
        ran.why
            .contains("every item of the run was built — and the end line was not written"),
        "{}",
        ran.why
    );
    assert_eq!(ran.built(), 6);
    // Eight workers and a refusal: no worker records past it.
    let wide = keys(100);
    let mut lines = failing(11, false);
    let eight = ask(Kind::SortIndex, 8, &stop, &now);
    let ran = drive(&mut lines, &root.0, &eight, begun(62, &wide), &clean).unwrap();
    assert_eq!(ran.how, How::Failed);
    assert_eq!(ran.built(), 10, "ten item lines after the run line");
}

/// A flush refused at the [`SYNC_EVERY`]-th item fails the run like a
/// refused line, and the item it would have made durable is left.
#[test]
fn a_flush_refused_mid_run_fails_the_run() {
    struct SyncOnce {
        inner: Journal,
        syncs: usize,
    }
    impl Lines for SyncOnce {
        fn line(&mut self, text: &str) -> Result<(), String> {
            self.inner.line(text)
        }
        fn sync(&mut self) -> Result<(), String> {
            self.syncs += 1;
            if self.syncs == 2 {
                Err("flush refused at the 64th item".to_owned())
            } else {
                Lines::sync(&mut self.inner)
            }
        }
    }
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 61;
    let plan = keys(usize::try_from(SYNC_EVERY).unwrap() + 5);
    let mut lines = SyncOnce {
        inner: Journal::open(&root.0, Vendor::Gdfl, Kind::SortIndex).unwrap(),
        syncs: 0,
    };
    let asked = ask(Kind::SortIndex, 1, &stop, &now);
    let ran = drive(&mut lines, &root.0, &asked, begun(61, &plan), &clean).unwrap();
    assert_eq!(ran.how, How::Failed);
    assert!(
        ran.why.contains("flush refused at the 64th item"),
        "{}",
        ran.why
    );
    assert_eq!(ran.built(), usize::try_from(SYNC_EVERY).unwrap() - 1);
}

// ═══ the months a scope holds ═══════════════════════════════════════════════

fn symbol_id(symbol: &str) -> u32 {
    u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX)).unwrap()
}

fn store_path<'a>(
    segment: &'a str,
    symbol: &'a str,
    contract: Option<&str>,
    rung: Timeframe,
    month: YearMonth,
) -> StorePath<'a> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract: contract.map(|c| Contract::parse(c).unwrap()),
        timeframe: rung,
        month,
        file: FileKind::Bars,
    })
    .unwrap()
}

/// Writes `count` invented one-second bars into one month.
fn put(root: &Path, at: StorePath<'_>, symbol: &str, count: i64, contract: bool) {
    let mut writer = BarFile::open_or_create(root, at, symbol_id(symbol)).unwrap();
    let open = at.month().ist_bounds_micros().0 + (86_400 + 555 * 60) * 1_000_000;
    let rows: Vec<Bar> = (0..count)
        .map(|k| Bar {
            ts_micros: open + k * i64::from(at.timeframe().secs()) * 1_000_000,
            open: 1_000_000 + k % 11,
            high: 1_000_050 + k % 11,
            low: 999_950 + k % 11,
            close: 1_000_010 + (k * 3) % 7,
            volume: k % 4,
            open_interest: if contract { 5 + k % 3 } else { OI_NULL },
        })
        .collect();
    writer.append(&rows).unwrap();
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).unwrap()
}

/// The walk lists every month file in path order with the sidecars it has
/// none of; it narrows by prefix, rung and span; it never follows a link
/// and counts what is not a store name; a vendor with no bars is refused.
#[test]
fn the_walk_lists_every_month_and_what_it_lacks() {
    let root = Root::new();
    let one = Timeframe::SECOND_1;
    let minute = Timeframe::MINUTE_1;
    // A big spot month the writer indexes whole, and a small one it gives
    // nothing (D-4933).
    put(
        &root.0,
        store_path("INDEX", "NIFTY", None, one, ym(2019, 3)),
        "NIFTY",
        1_100,
        false,
    );
    put(
        &root.0,
        store_path("INDEX", "NIFTY", None, one, ym(2019, 4)),
        "NIFTY",
        10,
        false,
    );
    put(
        &root.0,
        store_path("INDEX", "NIFTY", None, minute, ym(2019, 4)),
        "NIFTY",
        10,
        false,
    );
    put(
        &root.0,
        store_path(
            "FNO",
            "NIFTY",
            Some("2019-03-28-1150000-CE"),
            one,
            ym(2019, 3),
        ),
        "NIFTY",
        10,
        true,
    );
    let big = store_path("INDEX", "NIFTY", None, one, ym(2019, 3));
    fs::remove_file(big.with_file(FileKind::SortIndex).to_path_buf(&root.0)).unwrap();
    // What the walk leaves alone.
    let base = root.0.join("bars/gdfl/NSE");
    fs::write(base.join("INDEX/NIFTY/README"), "").unwrap();
    fs::create_dir_all(base.join("INDEX/nifty lower")).unwrap();
    std::os::unix::fs::symlink(base.join("INDEX/NIFTY"), base.join("INDEX/LINKED")).unwrap();
    fs::write(base.join("INDEX/NIFTY/1s/2019-13.bin"), "").unwrap();
    fs::write(base.join("INDEX/NIFTY/1s/notes.txt"), "").unwrap();
    fs::write(base.join("INDEX/NIFTY/1s/2019-3.bin"), "").unwrap();
    fs::create_dir_all(base.join("INDEX/NIFTY/2s")).unwrap();
    fs::create_dir_all(base.join("FNO/NIFTY/2019-03-28-1150000-CE/7m")).unwrap();
    std::os::unix::fs::symlink(
        base.join("INDEX/NIFTY/1s/2019-03.bin"),
        base.join("INDEX/NIFTY/1s/2019-05.bin"),
    )
    .unwrap();
    fs::create_dir_all(root.0.join("bars/gdfl/XYZ")).unwrap();
    fs::create_dir_all(base.join("NOTASEG")).unwrap();
    // A file named like a rung, under a symbol and under a contract: strays.
    fs::write(base.join("INDEX/NIFTY/1day"), "").unwrap();
    fs::write(base.join("FNO/NIFTY/2019-03-28-1150000-CE/1day"), "").unwrap();
    // A link beside a month that is not a month, and a link named like a
    // contract: never followed.
    std::os::unix::fs::symlink("nowhere", base.join("INDEX/NIFTY/1s/link.tix")).unwrap();
    std::os::unix::fs::symlink(
        base.join("FNO/NIFTY/2019-03-28-1150000-CE"),
        base.join("FNO/NIFTY/2019-03-28-1170000-CE"),
    )
    .unwrap();
    let walked = months(&root.0, Vendor::Gdfl, &Scope::default()).unwrap();
    let listed: Vec<(&str, Vec<Kind>)> = walked
        .months
        .iter()
        .map(|month| (month.key.as_str(), month.lacks.kinds().collect()))
        .collect();
    assert_eq!(
        listed,
        [
            (
                "bars/gdfl/NSE/FNO/NIFTY/2019-03-28-1150000-CE/1s/2019-03.bin",
                vec![Kind::DayIndex, Kind::SortIndex, Kind::MonthSummary]
            ),
            (
                "bars/gdfl/NSE/INDEX/NIFTY/1min/2019-04.bin",
                vec![Kind::DayIndex, Kind::SortIndex, Kind::MonthSummary]
            ),
            (
                "bars/gdfl/NSE/INDEX/NIFTY/1s/2019-03.bin",
                vec![Kind::SortIndex]
            ),
            (
                "bars/gdfl/NSE/INDEX/NIFTY/1s/2019-04.bin",
                vec![Kind::DayIndex, Kind::SortIndex, Kind::MonthSummary]
            ),
        ]
    );
    assert_eq!(walked.links, 4, "{walked:?}");
    assert_eq!(walked.strays, 11, "{walked:?}");
    assert_eq!(walked.strays_named.len(), 11);
    let first = &walked.months[2];
    assert_eq!(first.path(Vendor::Gdfl).unwrap(), big);
    assert_eq!(
        walked.months[0]
            .path(Vendor::Gdfl)
            .unwrap()
            .to_path_buf(&root.0),
        root.0.join(&walked.months[0].key)
    );
    // NARROWED: by prefix, by rung, by span.
    let index = Scope {
        prefix: vec!["NSE".to_owned(), "INDEX".to_owned()],
        ..Scope::default()
    };
    assert_eq!(
        months(&root.0, Vendor::Gdfl, &index).unwrap().months.len(),
        3
    );
    let contract = Scope {
        prefix: ["NSE", "FNO", "NIFTY", "2019-03-28-1150000-CE"]
            .map(str::to_owned)
            .to_vec(),
        ..Scope::default()
    };
    let only = months(&root.0, Vendor::Gdfl, &contract).unwrap().months;
    assert_eq!(only.len(), 1);
    assert!(only[0].contract.is_some());
    let other = Scope {
        prefix: ["NSE", "FNO", "NIFTY", "2019-03-28-1160000-CE"]
            .map(str::to_owned)
            .to_vec(),
        ..Scope::default()
    };
    assert!(
        months(&root.0, Vendor::Gdfl, &other)
            .unwrap()
            .months
            .is_empty()
    );
    let spot_and_contracts = Scope {
        prefix: ["NSE", "FNO", "NIFTY"].map(str::to_owned).to_vec(),
        ..Scope::default()
    };
    assert_eq!(
        months(&root.0, Vendor::Gdfl, &spot_and_contracts)
            .unwrap()
            .months
            .len(),
        1
    );
    let seconds = Scope {
        rung: Some(one),
        ..Scope::default()
    };
    assert_eq!(
        months(&root.0, Vendor::Gdfl, &seconds)
            .unwrap()
            .months
            .len(),
        3
    );
    let april = Scope {
        from: Some(ym(2019, 4)),
        to: Some(ym(2019, 4)),
        ..Scope::default()
    };
    // Named, not counted: March holds two months as well.
    assert_eq!(
        months(&root.0, Vendor::Gdfl, &april)
            .unwrap()
            .months
            .into_iter()
            .map(|month| month.key)
            .collect::<Vec<_>>(),
        [
            "bars/gdfl/NSE/INDEX/NIFTY/1min/2019-04.bin",
            "bars/gdfl/NSE/INDEX/NIFTY/1s/2019-04.bin"
        ]
    );
    let march = Scope {
        to: Some(ym(2019, 3)),
        ..Scope::default()
    };
    assert_eq!(
        months(&root.0, Vendor::Gdfl, &march)
            .unwrap()
            .months
            .into_iter()
            .map(|month| month.key)
            .collect::<Vec<_>>(),
        [
            "bars/gdfl/NSE/FNO/NIFTY/2019-03-28-1150000-CE/1s/2019-03.bin",
            "bars/gdfl/NSE/INDEX/NIFTY/1s/2019-03.bin"
        ]
    );
    // A symbol the scope does not name, and a contract under a symbol that
    // holds only spot rungs: nothing.
    let other_symbol = Scope {
        prefix: ["NSE", "INDEX", "BANKNIFTY"].map(str::to_owned).to_vec(),
        ..Scope::default()
    };
    assert!(
        months(&root.0, Vendor::Gdfl, &other_symbol)
            .unwrap()
            .months
            .is_empty()
    );
    let spot_contract = Scope {
        prefix: ["NSE", "INDEX", "NIFTY", "2019-03-28-1150000-CE"]
            .map(str::to_owned)
            .to_vec(),
        ..Scope::default()
    };
    assert!(
        months(&root.0, Vendor::Gdfl, &spot_contract)
            .unwrap()
            .months
            .is_empty()
    );
    let none = Scope {
        prefix: vec!["BSE".to_owned()],
        ..Scope::default()
    };
    assert!(
        months(&root.0, Vendor::Gdfl, &none)
            .unwrap()
            .months
            .is_empty()
    );
    let why = months(&root.0, Vendor::Zerodha, &Scope::default()).unwrap_err();
    assert!(why.contains("holds no zerodha bars"), "{why}");
}

/// A month whose sidecar name is held by something that is not absent — a
/// link, a directory — is not listed as lacking it: the builder will name
/// what is there.
#[test]
fn a_sidecar_name_held_by_anything_is_not_lacking() {
    let root = Root::new();
    let at = store_path("INDEX", "NIFTY", None, Timeframe::SECOND_1, ym(2020, 1));
    put(&root.0, at, "NIFTY", 10, false);
    fs::create_dir_all(at.with_file(FileKind::TimeIndex).to_path_buf(&root.0)).unwrap();
    std::os::unix::fs::symlink(
        "nowhere",
        at.with_file(FileKind::SortIndex).to_path_buf(&root.0),
    )
    .unwrap();
    let walked = months(&root.0, Vendor::Gdfl, &Scope::default()).unwrap();
    assert_eq!(walked.months.len(), 1);
    let lacks = walked.months[0].lacks;
    assert_eq!(lacks.kinds().collect::<Vec<_>>(), [Kind::MonthSummary]);
    assert!(
        lacks.has(Kind::MonthSummary) && !lacks.has(Kind::DayIndex) && !lacks.has(Kind::SortIndex)
    );
    assert!(!Lacks::of(Kind::ALL).has(Kind::Recount) && !Lacks::of(Kind::ALL).has(Kind::Rekey));
    assert_eq!(Lacks::of([Kind::Recount, Kind::Rekey]), Lacks::default());
    assert_eq!(Lacks::of(Kind::ALL).kinds().count(), 3);
    // A kind named twice is still lacked: a set, never a toggle.
    assert_eq!(
        Lacks::of([Kind::SortIndex, Kind::SortIndex])
            .kinds()
            .collect::<Vec<_>>(),
        [Kind::SortIndex]
    );
    let month: &Month = &walked.months[0];
    assert_eq!(month.symbol, "NIFTY");
    assert_eq!(
        (month.exchange.as_str(), month.segment.as_str()),
        ("NSE", "INDEX")
    );
    assert_eq!(month.rung, Timeframe::SECOND_1);
    assert_eq!(month.month, ym(2020, 1));
}

/// More strays than are named are counted past the names.
#[test]
fn strays_past_the_named_are_counted() {
    let root = Root::new();
    let dir = root.0.join("bars/gdfl/NSE/INDEX/NIFTY/1s");
    fs::create_dir_all(&dir).unwrap();
    for k in 0..(crate::gdfl_rekey::NAMED + 5) {
        fs::write(dir.join(format!("stray-{k:03}.bin")), "").unwrap();
    }
    let walked = months(&root.0, Vendor::Gdfl, &Scope::default()).unwrap();
    assert_eq!(
        walked.strays,
        u64::try_from(crate::gdfl_rekey::NAMED + 5).unwrap()
    );
    assert_eq!(walked.strays_named.len(), crate::gdfl_rekey::NAMED);
    assert!(walked.months.is_empty());
}

/// A directory in scope that cannot be listed refuses the walk by name.
#[test]
fn a_directory_that_cannot_be_listed_refuses_the_walk() {
    let root = Root::new();
    let base = root.0.join("bars/gdfl/NSE/INDEX");
    fs::create_dir_all(&base).unwrap();
    // A rung directory under a symbol that is a FILE named like a rung is a
    // stray; a symbol directory that vanishes between the listing and the
    // walk cannot be listed.
    let mut file = fs::File::create(base.join("NIFTY")).unwrap();
    file.write_all(b"not a directory").unwrap();
    let walked = months(&root.0, Vendor::Gdfl, &Scope::default()).unwrap();
    assert_eq!(walked.strays, 1);
    let why = super::listed(&base.join("NIFTY")).unwrap_err();
    assert!(why.contains("cannot list"), "{why}");
}

/// Spins until `flag` is set, failing the test after ten seconds.
fn wait_for(flag: &AtomicBool) {
    let started = std::time::Instant::now();
    while !flag.load(Ordering::Acquire) {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(10),
            "a rendezvous of the test never came"
        );
        std::thread::yield_now();
    }
}

/// Lines whose first `item` line is refused, saying so through `refused`.
struct RefuseFirstItem<'a> {
    inner: Journal,
    refused: &'a AtomicBool,
}

impl Lines for RefuseFirstItem<'_> {
    fn line(&mut self, text: &str) -> Result<(), String> {
        if text.starts_with("item ") && !self.refused.load(Ordering::Acquire) {
            self.refused.store(true, Ordering::Release);
            return Err("no space left on device (first item)".to_owned());
        }
        self.inner.line(text)
    }

    fn sync(&mut self) -> Result<(), String> {
        Lines::sync(&mut self.inner)
    }
}

/// **AN ITEM BUILT WHILE ANOTHER WORKER'S LINE IS REFUSED IS NOT COUNTED.**
/// Two workers each hold an item; the first's line is refused while the
/// second is still building; the second then finds the refusal and records
/// nothing, so neither item is counted and a resume builds both.
#[test]
fn an_item_built_while_another_line_is_refused_is_not_counted() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 70;
    let plan = keys(3);
    let (second_started, refused) = (AtomicBool::new(false), AtomicBool::new(false));
    let racing = |key: &str, &at: &usize| {
        if at == 0 {
            wait_for(&second_started);
        } else {
            second_started.store(true, Ordering::Release);
            wait_for(&refused);
        }
        clean(key, &at)
    };
    let mut lines = RefuseFirstItem {
        inner: Journal::open(&root.0, Vendor::Gdfl, Kind::SortIndex).unwrap(),
        refused: &refused,
    };
    let asked = ask(Kind::SortIndex, 2, &stop, &now);
    let ran = drive(&mut lines, &root.0, &asked, begun(70, &plan), &racing).unwrap();
    assert_eq!(ran.how, How::Failed);
    assert_eq!(ran.workers, 2);
    assert_eq!(ran.built(), 0, "{:?}", ran.outcomes);
    assert_eq!(ran.left(), 3);
    assert!(ran.why.contains("first item"), "{}", ran.why);
}

/// A Stop seen once the last item was taken leaves nothing: the run is done,
/// and whole.
#[test]
fn a_stop_seen_after_the_last_item_leaves_the_run_done() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 80;
    let plan = keys(3);
    let last = |key: &str, &at: &usize| {
        if at == 2 {
            stop.store(true, Ordering::Release);
        }
        clean(key, &at)
    };
    let ran = run(&root.0, &ask(Kind::SortIndex, 1, &stop, &now), &plan, &last).unwrap();
    assert_eq!((ran.how, ran.built(), ran.left()), (How::Done, 3, 0));
    assert!(ran.whole());
}

/// A flush refused right after the `run` line builds nothing and is an
/// error.
#[test]
fn a_flush_refused_after_the_run_line_builds_nothing() {
    struct FirstSync {
        inner: Journal,
    }
    impl Lines for FirstSync {
        fn line(&mut self, text: &str) -> Result<(), String> {
            self.inner.line(text)
        }
        fn sync(&mut self) -> Result<(), String> {
            Err("flush refused at the run line".to_owned())
        }
    }
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 90;
    let built = AtomicUsize::new(0);
    let counting = |key: &str, &at: &usize| {
        built.fetch_add(1, Ordering::Relaxed);
        clean(key, &at)
    };
    let mut lines = FirstSync {
        inner: Journal::open(&root.0, Vendor::Gdfl, Kind::SortIndex).unwrap(),
    };
    let plan = keys(2);
    let asked = ask(Kind::SortIndex, 1, &stop, &now);
    let why = drive(&mut lines, &root.0, &asked, begun(90, &plan), &counting).unwrap_err();
    assert!(why.contains("flush refused at the run line"), "{why}");
    assert_eq!(built.load(Ordering::Relaxed), 0);
}

/// A month whose served generation cannot be told — its pointer's name held
/// by a directory — refuses the walk by name: what it lacks cannot be said.
#[test]
fn a_month_whose_generation_cannot_be_told_refuses_the_walk() {
    let root = Root::new();
    let at = store_path("INDEX", "NIFTY", None, Timeframe::SECOND_1, ym(2020, 2));
    put(&root.0, at, "NIFTY", 10, false);
    fs::create_dir_all(store::generation::pointer_path(&root.0, at)).unwrap();
    let why = months(&root.0, Vendor::Gdfl, &Scope::default()).unwrap_err();
    assert!(why.contains("2020-02.bin"), "{why}");
}

/// The tail a status reads holds the longest line a run can write — a key
/// of [`KEY_CAP`] bytes and a reason cut at [`WHY_CAP`] — several times over.
#[test]
fn a_status_reads_the_longest_line_a_run_can_write() {
    let root = Root::new();
    let key = "k".repeat(KEY_CAP);
    let why = one_line(&"w".repeat(WHY_CAP + 500));
    let item = format!("item 3 n=1 ok=1 of=2 t=9 clean=1 word=built key={key} why={why}");
    assert!(item.len() > 1_500, "{}", item.len());
    assert!(
        u64::try_from(item.len() * 8).unwrap() < TAIL_BYTES,
        "eight of the longest lines fit the tail"
    );
    let path = root.0.join(journal_name(Vendor::Gdfl, Kind::SortIndex));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        format!(
            "run 3 v1 vendor=gdfl kind=srt commit=x workers=1 asked=1 items=2 skipped=0 \
             plan=0 resume=- at=3\n{item}\n"
        ),
    )
    .unwrap();
    assert_eq!(parse(&item).map(|entry| entry.id()), Some(3));
    let found = status(&root.0, Vendor::Gdfl, Kind::SortIndex)
        .unwrap()
        .unwrap();
    assert_eq!((found.id, found.n, found.of, found.t), (3, 1, 2, 9));
    assert_eq!(found.state, State::Died);
}

/// The host's own refusals, each said by name: a `flock` refused outright
/// (not held by another run), a lock probe refused, a lock the host would not
/// release — which fails a run that was otherwise done — and a month whose
/// parts the store cannot name.
#[test]
fn the_hosts_own_refusals_are_said_by_name() {
    let path = PathBuf::from("/store/audit/upkeep.gdfl.srt.log");
    let refused = std::io::Error::other("the host refused");
    let said = super::not_locked(
        &path,
        Vendor::Gdfl,
        Kind::SortIndex,
        std::fs::TryLockError::Error(refused),
    );
    assert_eq!(
        said,
        "/store/audit/upkeep.gdfl.srt.log: flock: the host refused; nothing was written"
    );
    let busy = super::not_locked(
        &path,
        Vendor::Gdfl,
        Kind::SortIndex,
        std::fs::TryLockError::WouldBlock,
    );
    assert!(
        busy.starts_with("another srt upkeep run of gdfl holds"),
        "{busy}"
    );
    assert!(!super::held_by_a_run(Ok(())).unwrap());
    assert!(super::held_by_a_run(Err(std::fs::TryLockError::WouldBlock)).unwrap());
    let probe = super::held_by_a_run(Err(std::fs::TryLockError::Error(std::io::Error::other(
        "probe refused",
    ))));
    assert_eq!(probe.unwrap_err().to_string(), "probe refused");
    let ran = |how: How| super::Ran {
        id: 1,
        resumed: None,
        workers: 1,
        skipped: 0,
        outcomes: vec![Some(Outcome::clean("built", "w"))],
        how,
        why: "every item of the run was built".to_owned(),
        torn: 0,
        millis: 0,
        flushes: 0,
    };
    assert_eq!(super::released(ran(How::Done), Ok(())), ran(How::Done));
    let left = super::released(
        ran(How::Done),
        Err(store::flock::Unreleased {
            path: path.clone(),
            why: std::io::Error::other("unlock refused"),
        }),
    );
    assert_eq!(left.how, How::Failed);
    assert!(
        left.why.starts_with(
            "every item of the run was built — and the journal's lock was not released: "
        ),
        "{}",
        left.why
    );
    assert!(left.why.contains("unlock refused"), "{}", left.why);
    assert!(!left.whole());
    let nameless = Month {
        exchange: String::new(),
        segment: "INDEX".to_owned(),
        symbol: "NIFTY".to_owned(),
        contract: None,
        rung: Timeframe::SECOND_1,
        month: ym(2020, 1),
        key: "bars/gdfl//INDEX/NIFTY/1s/2020-01.bin".to_owned(),
        lacks: Lacks::default(),
    };
    let why = nameless.path(Vendor::Gdfl).unwrap_err();
    assert!(
        why.starts_with("bars/gdfl//INDEX/NIFTY/1s/2020-01.bin is not a store path: "),
        "{why}"
    );
}

/// **A RUN FLUSHES ITS JOURNAL** after its `run` line, after every
/// [`SYNC_EVERY`]-th item and after its `end` line, and no more often: what a
/// kill can lose is at most the lines since the last flush (L-R317-06).
#[test]
fn a_run_flushes_its_journal_at_the_run_line_every_64_items_and_the_end() {
    let root = Root::new();
    let stop = AtomicBool::new(false);
    let now = || 99;
    let every = usize::try_from(SYNC_EVERY).unwrap();
    for (items, flushes) in [
        (0, 2),
        (1, 2),
        (every - 1, 2),
        (every, 3),
        (2 * every + 2, 4),
    ] {
        let ran = run(
            &root.0,
            &ask(Kind::SortIndex, 1, &stop, &now),
            &keys(items),
            &clean,
        )
        .unwrap();
        assert_eq!(ran.flushes, flushes, "{items} item(s)");
        assert_eq!(ran.built(), items);
    }
}
