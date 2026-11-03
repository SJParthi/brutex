#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]
//! BASE-COV-PULL-PS1 (D-7335): the arms of the importer's day loop no test
//! reached at tip 2e7a0b6d3b56 (Route B), each driven through the input that
//! reaches it: a scope ledger or a journal that will not take a line, a store
//! root that is a dangling link or cannot be made, a source lost after a file
//! landed, a store that takes no writes, a stop seen at a day's top, the reach
//! back over another filter's keys, a listing that cannot be given, a member
//! whose day is gone, and the day loop's own warnings, read back from a sink
//! in a child process. Every symbol (`ZQ...`), price and digest is invented
//! at run time; no vendor row is quoted.

use std::path::{Path, PathBuf};

use super::*;
use crate::gdfl_fixtures::{put, scratch};
use crate::gdfl_import::{Durability, Observer, SourceKind, Tick, Workers};
use crate::ingest::FailKind;

/// 09:15:00 IST, the session's first second.
const NINE_FIFTEEN: u32 = 9 * 3_600 + 15 * 60;

/// The ledger id of the run whose key another run holds, in the sink test.
const SINK_RUN: u64 = 1_790_100_101;

/// The id of the run that holds the sink's key there.
const HOLDER: u64 = 1_790_100_199;

// ── helpers ────────────────────────────────────────────────────────────────

/// A day of April 2024: the 1st to the 5th are regular sessions, the 6th is a
/// Saturday.
fn april(day: u8) -> Day {
    Day::new(2024, 4, day).unwrap()
}

/// The run of `kind` over `from..=to` into `store`, every symbol wanted.
fn run_over(kind: ImportKind, store: &Path, (from, to): (Day, Day)) -> Run<'_> {
    Run {
        kind,
        from,
        to,
        only: &[],
        store_root: store,
    }
}

/// One worker, and every write made durable as it is written: a run whose
/// files are filed on the calling thread and whose day takes no batch.
fn quiet<'a>() -> RunOptions<'a> {
    RunOptions {
        workers: Workers::new(1).unwrap(),
        durability: Durability::EachWrite,
        ..RunOptions::default()
    }
}

/// An invented share's day file: two traded seconds at prices no listed
/// share prints.
fn share_file(symbol: &str) -> TickFile {
    TickFile {
        symbol: symbol.to_owned(),
        contract: None,
        name: format!("{symbol}.invented"),
        ticks: vec![
            Tick {
                sod: NINE_FIFTEEN,
                ltp: 777_700,
                ltq: 7,
                oi: None,
            },
            Tick {
                sod: NINE_FIFTEEN + 2,
                ltp: 777_800,
                ltq: 3,
                oi: None,
            },
        ],
    }
}

/// An invented index's day file: four seconds a minute apart, no volume.
fn index_file(symbol: &str) -> TickFile {
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

/// What a source answers for a day it holds, beside its files.
fn held_day() -> DayRead {
    DayRead {
        held: true,
        ..DayRead::default()
    }
}

/// The scope member a share is filed under.
fn cash(share: &str) -> String {
    format!("NSE-CASH-{share}")
}

/// The scope member an index is filed under.
fn index_member(index: &str) -> String {
    format!("NSE-INDEX-{index}")
}

/// The recorded scope of these shares.
fn cash_scope(shares: &[&str]) -> scope::Record {
    scope::Record::of(shares.iter().copied().map(cash).collect())
}

/// The members of `set`, in order.
fn sorted(set: &HashSet<String>) -> Vec<String> {
    let mut all: Vec<String> = set.iter().cloned().collect();
    all.sort();
    all
}

/// A closing line as this build writes one: `head` (its verb and its key),
/// the D-2822 counts under the current definition, `tail` (its `scope=` and
/// any field after it), then `run=1`.
fn closing(head: &str, tail: &str) -> String {
    format!(
        "{head} definition={BAR_DEFINITION} files=1 seconds=1 failures=0 late=0 \
         late_unresolved=0 max_back_s=0{tail} run=1\n"
    )
}

/// `key` closed `done` under the scope digested `digest`.
fn done_under(key: &str, digest: &str) -> String {
    closing(&format!("done {key}"), &format!(" scope={digest}"))
}

/// Writes `lines` as the store's journal.
fn put_journal(store: &Path, lines: &[String]) {
    let _journal = put(store, "imports/gdfl.journal", lines.concat().as_bytes());
}

/// Records each of `records` in the store's scope ledger, as a run of `kind`
/// records its own.
fn record_scopes(store: &Path, kind: ImportKind, records: &[&scope::Record]) {
    let mut ledger = scope::Ledger::load(scope::scopes_path(store)).unwrap();
    for record in records {
        ledger.record(kind.as_str(), record).unwrap();
    }
}

/// The store's journal, or nothing when it holds none.
fn journal_text(store: &Path) -> String {
    std::fs::read_to_string(journal_path(store)).unwrap_or_default()
}

/// The store's scope ledger, or nothing when it holds none.
fn scopes_text(store: &Path) -> String {
    std::fs::read_to_string(scope::scopes_path(store)).unwrap_or_default()
}

/// The journal's last line naming `key` (a `begin` or a close), if any.
fn last_line_of(store: &Path, key: &str) -> Option<String> {
    journal_text(store)
        .lines()
        .rev()
        .find(|line| {
            line.split(' ')
                .skip(1)
                .take(3)
                .collect::<Vec<_>>()
                .join(" ")
                == key
        })
        .map(str::to_owned)
}

/// Files `files` through the day's filer as a source hands them on: one job
/// each, held back by its instrument-month.
fn file_these(day_filer: &mut DayFiler<'_>, day: Day, files: Vec<TickFile>) {
    let month = day.year_month().unwrap();
    let jobs: Vec<Handed> = files
        .into_iter()
        .map(|file| Handed {
            file,
            facts: FileFacts::default(),
        })
        .collect();
    let _reads = day_filer.file_all(
        &jobs,
        &|one: &Handed| Some((one.file.symbol.clone(), one.file.contract, month)),
        &|one: &Handed| Ok((one.clone(), None)),
    );
}

/// Drives `run` under the recorded scope `now` beside the swept surface
/// `swept` through `read_day`, with the real census.
fn drive_scoped<F>(
    run: &Run<'_>,
    (now, swept): (&scope::Record, &scope::Record),
    options: RunOptions<'_>,
    read_day: F,
) -> Result<Report, ImportRefusal>
where
    F: FnMut(Day, &mut DayFiler<'_>) -> (DayRead, DayExtras),
{
    drive_core(
        run,
        options,
        (&mut StoreCensus::default(), Some((now, swept))),
        read_day,
    )
}

/// The context of a run that records no scope, as `drive_days` builds it.
fn bare_ctx<'a>(run: &'a Run<'a>, journal: &'a Journal, options: RunOptions<'a>) -> Ctx<'a> {
    Ctx {
        run,
        journal,
        options,
        id: 7,
        started: 0,
        announced: std::cell::Cell::new(false),
        scoped: None,
        held_back: std::cell::RefCell::new(HashMap::new()),
        store_id: std::cell::Cell::new(None),
    }
}

/// The context of a run recording `record` beside the swept surface `swept`,
/// its scope ledger read from the run's store as `drive_days` reads it.
fn scoped_ctx<'a>(
    run: &'a Run<'a>,
    journal: &'a Journal,
    (record, swept): (&'a scope::Record, &scope::Record),
    options: RunOptions<'a>,
) -> Ctx<'a> {
    Ctx {
        scoped: Some(Scoped {
            record,
            swept: std::sync::Arc::clone(swept.set()),
            ledger: std::cell::RefCell::new(
                scope::Ledger::load(scope::scopes_path(run.store_root)).unwrap(),
            ),
            beyond: std::cell::RefCell::new(HashMap::new()),
            shares: std::cell::RefCell::new(HashMap::new()),
        }),
        ..bare_ctx(run, journal, options)
    }
}

/// The run's own key of `day`, asked as the range's loop asks it (`early`
/// false) or as the reach back does (`early` true).
fn own(key: &str, day: Day, early: bool) -> Asked<'_> {
    Asked {
        key,
        day,
        remembered: key,
        names: None,
        early,
    }
}

/// A store whose journal closes 1 and 2 April `done` under a scope its scope
/// ledger does not hold: the store has no ledger at all.
fn unrecorded_store(tag: &str) -> PathBuf {
    let store = scratch(tag);
    let unrecorded = cash_scope(&["ZQX"]);
    put_journal(
        &store,
        &[
            done_under(&format!("stocks {} *", april(1)), unrecorded.digest()),
            done_under(&format!("stocks {} *", april(2)), unrecorded.digest()),
        ],
    );
    store
}

/// The host's refusal of a write to a read-only volume, as the store's door
/// names it.
fn read_only() -> FailKind {
    FailKind::Io {
        kind: std::io::ErrorKind::ReadOnlyFilesystem,
        publication_may_be_visible: false,
    }
}

/// Plants `kind` as the store's answer to the next append of `share`'s file
/// under `store` (the filing's inject seam).
fn plant(store: &Path, share: &str, kind: FailKind) {
    crate::gdfl_import::filing::inject::plant(store, &format!("{share}.invented"), kind, false);
}

/// The nanoseconds since the epoch: a name no earlier run used.
fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// An observer that, once the run's day has passed, stands a directory at the
/// journal's name, so the run's end line meets a journal that will not take
/// it; it keeps what the journal said then and whether the run ended with its
/// report.
#[derive(Debug)]
struct BreakJournal {
    journal: PathBuf,
    said: std::sync::Mutex<String>,
    ended_ok: std::sync::atomic::AtomicBool,
}

impl BreakJournal {
    /// The observer of a run into `store`.
    fn at(store: &Path) -> Self {
        Self {
            journal: journal_path(store),
            said: std::sync::Mutex::new(String::new()),
            ended_ok: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

impl Observer for BreakJournal {
    fn begun(&self, _kind: ImportKind, _run: u64) {}
    fn day_begun(&self, _kind: ImportKind, _day: Day, _at: (usize, usize)) {}
    fn day_passed(&self, _kind: ImportKind, _day: Day, _report: &Report) {
        let text = std::fs::read_to_string(&self.journal).unwrap();
        std::fs::remove_file(&self.journal).unwrap();
        std::fs::create_dir(&self.journal).unwrap();
        *self.said.lock().unwrap() = text;
    }
    fn finished(&self, _kind: ImportKind, end: Result<&Report, &ImportRefusal>) {
        self.ended_ok
            .store(end.is_ok(), std::sync::atomic::Ordering::SeqCst);
    }
}

/// The text of field `key` of `record`, or `None`.
fn text(record: &telemetry::Record, key: &str) -> Option<String> {
    record
        .field(key)
        .and_then(telemetry::OwnedValue::as_str)
        .map(str::to_owned)
}

/// The count of field `key` of `record`, or `None`.
fn count(record: &telemetry::Record, key: &str) -> Option<u64> {
    record.field(key).and_then(telemetry::OwnedValue::as_u64)
}

/// The flag of field `key` of `record`, or `None`.
fn flag(record: &telemetry::Record, key: &str) -> Option<bool> {
    record.field(key).and_then(telemetry::OwnedValue::as_bool)
}

// ── what a done day owes (WP-12f, D-4878) ──────────────────────────────────

/// `Ctx::held` WITHOUT A SCOPE IS `false` (D-4878 item 10): a run that records
/// no scope cannot tell whether a whole-tree close read a share, so the share's
/// restatement stands; the same question under a recorded swept surface that
/// files the share answers `true`, and `false` for one it does not.
#[test]
fn a_run_with_no_scope_never_says_a_close_read_a_share() {
    let store = scratch("ps1-held-unscoped");
    let journal = Journal::load(journal_path(&store)).unwrap();
    let run = run_over(ImportKind::Stocks, &store, (april(1), april(1)));
    let bare = bare_ctx(&run, &journal, quiet());
    assert!(
        !bare.held("", "ZQA"),
        "no swept surface is known without a scope"
    );
    assert!(!bare.held("an-unknown-scope", "ZQA"));
    let (record, swept) = (cash_scope(&["ZQA", "ZQB"]), cash_scope(&["ZQA"]));
    let scoped = scoped_ctx(&run, &journal, (&record, &swept), quiet());
    assert!(scoped.held("", "ZQA"), "the swept surface files ZQA");
    assert!(!scoped.held("", "ZQB"), "and not ZQB");
    std::fs::remove_dir_all(store).unwrap();
}

/// A DONE-WITH-REFUSALS KEY READ AGAIN WHOLE OWES NOTHING (WP-12f check 3):
/// under `retry_refused` the run's own key inside its range is read again by
/// `admit`, so `owed` answers `Nothing`; the same close asked as a day before
/// the range, never read whole, still owes the member its scope never covered,
/// and so does the key in a run without the flag.
#[test]
fn a_retried_done_day_owes_nothing_only_where_the_run_reads_it_whole() {
    let store = scratch("ps1-owed-retried");
    let narrow = cash_scope(&["ZQA"]);
    let key = format!("stocks {} *", april(1));
    put_journal(
        &store,
        &[closing(
            &format!("done {key}"),
            &format!(
                " scope={} vendor_refused=1 reason=ticker_mismatch",
                narrow.digest()
            ),
        )],
    );
    record_scopes(&store, ImportKind::Stocks, &[&narrow]);
    let journal = Journal::load(journal_path(&store)).unwrap();
    assert!(journal.with_refusals.contains(&key), "closed with refusals");
    let wide = cash_scope(&["ZQA", "ZQB"]);
    let run = run_over(ImportKind::Stocks, &store, (april(1), april(1)));
    let retrying = RunOptions {
        retry_refused: true,
        ..quiet()
    };
    let ctx = scoped_ctx(&run, &journal, (&wide, &narrow), retrying);
    assert!(
        matches!(ctx.owed(&own(&key, april(1), false)), Owed::Nothing),
        "read again whole by admit, so it owes nothing"
    );
    let Owed::Probe(early) = ctx.owed(&own(&key, april(1), true)) else {
        panic!("a day before the range is never read whole");
    };
    assert_eq!(sorted(&early), [cash("ZQB")]);
    let plain = scoped_ctx(&run, &journal, (&wide, &narrow), quiet());
    let Owed::Probe(beyond) = plain.owed(&own(&key, april(1), false)) else {
        panic!("without the flag the close still owes ZQB");
    };
    assert_eq!(sorted(&beyond), [cash("ZQB")]);
    std::fs::remove_dir_all(store).unwrap();
}

/// AN EXAMINED DAY OWES ONLY THE OPEN MEMBER KEYS ITS SCOPE HOLDS (D-4878): a
/// day examined under this run's scope whose member keys are ZQB (`done`) and
/// ZQZ (`incomplete`, opened by a wider run) owes this run nothing while its
/// scope does not hold ZQZ, and owes ZQZ alone to a run whose scope holds it
/// and that examined the day.
#[test]
fn an_examined_day_owes_no_member_key_its_scope_does_not_hold() {
    let store = scratch("ps1-owed-pending");
    let key = format!("stocks {} *", april(1));
    let narrow = cash_scope(&["ZQA"]);
    put_journal(
        &store,
        &[
            done_under(&key, narrow.digest()),
            closing(&format!("done {key}+{}", cash("ZQB")), ""),
            closing(
                &format!("incomplete {key}+{}", cash("ZQZ")),
                " stage=file reason=fold",
            ),
        ],
    );
    let (now, wider) = (
        cash_scope(&["ZQA", "ZQB"]),
        cash_scope(&["ZQA", "ZQB", "ZQZ"]),
    );
    record_scopes(&store, ImportKind::Stocks, &[&narrow, &now]);
    let mut ledger = scope::Ledger::load(scope::scopes_path(&store)).unwrap();
    ledger.examine(&key, now.digest(), (1, 2)).unwrap();
    let journal = Journal::load(journal_path(&store)).unwrap();
    let run = run_over(ImportKind::Stocks, &store, (april(1), april(1)));
    let ctx = scoped_ctx(&run, &journal, (&now, &narrow), quiet());
    assert!(
        matches!(ctx.owed(&own(&key, april(1), false)), Owed::Nothing),
        "ZQB is done and ZQZ is no member of this scope"
    );
    ledger.examine(&key, wider.digest(), (2, 1)).unwrap();
    let ctx = scoped_ctx(&run, &journal, (&wider, &narrow), quiet());
    let Owed::Pending(members) = ctx.owed(&own(&key, april(1), false)) else {
        panic!("ZQZ's own key is still open");
    };
    assert_eq!(members, [cash("ZQZ")]);
    std::fs::remove_dir_all(store).unwrap();
}

/// A CLOSE NAMING A SCOPE THE LEDGER DOES NOT HOLD IS READ AS THE SWEPT
/// SURFACE (D-4878): it owes this run's members beyond the swept surface,
/// never every member as if it had covered none, and that set is computed once
/// a run for every day the digest names.
#[test]
fn a_close_naming_an_unrecorded_scope_owes_what_lies_beyond_the_swept_surface() {
    let store = unrecorded_store("ps1-owed-unrecorded");
    let journal = Journal::load(journal_path(&store)).unwrap();
    let (now, swept) = (cash_scope(&["ZQA", "ZQB"]), cash_scope(&["ZQA"]));
    let run = run_over(ImportKind::Stocks, &store, (april(1), april(2)));
    let ctx = scoped_ctx(&run, &journal, (&now, &swept), quiet());
    let (first, second) = (
        format!("stocks {} *", april(1)),
        format!("stocks {} *", april(2)),
    );
    let Owed::Probe(one) = ctx.owed(&own(&first, april(1), false)) else {
        panic!("a close under an unrecorded scope owes the members beyond the swept surface");
    };
    let Owed::Probe(two) = ctx.owed(&own(&second, april(2), false)) else {
        panic!("so does the second day's");
    };
    assert_eq!(sorted(&one), [cash("ZQB")], "beyond the swept surface only");
    assert!(std::sync::Arc::ptr_eq(&one, &two), "one set for both days");
    assert_eq!(ctx.scoped.as_ref().unwrap().beyond.borrow().len(), 1);
    std::fs::remove_dir_all(store).unwrap();
}

// ── the close and admit (D-3317, D-4878, D-4885) ───────────────────────────

/// A SCOPE LEDGER THAT WILL NOT TAKE THE RUN'S MEMBER LIST LEAVES THE DAY
/// CLOSED NOWHERE (D-4878): a close names its scope only after the list is in
/// the ledger, so a ledger that refuses it is one more named failure of the
/// day, which the report counts `incomplete` while the journal writes no
/// closing line: the day stays begun and the next run reads it again.
#[test]
fn a_scope_ledger_that_refuses_the_runs_list_leaves_the_day_unclosed() {
    let store = scratch("ps1-close-ledger");
    let (now, swept) = (cash_scope(&["ZQA"]), cash_scope(&["ZQA"]));
    let run = run_over(ImportKind::Stocks, &store, (april(1), april(1)));
    let ledger = scope::scopes_path(&store);
    let report = drive_scoped(&run, (&now, &swept), quiet(), |on, filer| {
        assert!(matches!(filer.ask, Ask::All), "{:?}", filer.ask);
        std::fs::create_dir_all(&ledger).unwrap();
        file_these(filer, on, vec![share_file("ZQA")]);
        (held_day(), DayExtras::default())
    })
    .unwrap();
    let key = format!("stocks {} *", april(1));
    assert_eq!(report.closed_incomplete, [april(1)]);
    assert_eq!(report.days_imported, 1);
    let [refused] = report.failures.as_slice() else {
        panic!("{:?}", report.failures);
    };
    assert_eq!(refused.instrument, format!("{} scope ledger", april(1)));
    assert!(
        refused.why.starts_with("the import journal: ")
            && refused.why.contains(&ledger.display().to_string()),
        "{}",
        refused.why
    );
    assert_eq!(
        report.failures_by_cause.get(&("journal", "journal")),
        Some(&1)
    );
    assert_eq!(
        last_line_of(&store, &key),
        Some(format!("begin {key}")),
        "begun, and closed nowhere"
    );
    let journal = Journal::load(journal_path(&store)).unwrap();
    assert!(journal.open.contains(&key), "the next run reads it again");
    std::fs::remove_dir_all(store).unwrap();
}

/// ROW 3 OF `admit` (D-4885): a day closed `done` under a definition no longer
/// current is skipped unread and counted in `restated_skipped` by a run that
/// asks not to restate, which writes nothing; a run that does not ask reads it
/// again and names it restated and resumed.
#[test]
fn a_restated_day_is_skipped_unread_only_when_the_run_asks() {
    let store = scratch("ps1-skip-restated");
    let key = format!("stocks {} *", april(1));
    put_journal(
        &store,
        &[format!(
            "done {key} definition=1 files=1 seconds=1 failures=0 late=0 late_unresolved=0 \
             max_back_s=0 run=1\n"
        )],
    );
    let before = journal_text(&store);
    let run = run_over(ImportKind::Stocks, &store, (april(1), april(1)));
    let skip_asked = RunOptions {
        skip_restated: true,
        ..quiet()
    };
    let first_run = drive_from(&run, skip_asked, |_, _| -> DayRead {
        panic!("a restated day is not read when the run asks not to restate")
    })
    .unwrap();
    assert_eq!(
        (
            first_run.restated_skipped,
            first_run.days_skipped,
            first_run.days_imported
        ),
        (1, 0, 0)
    );
    assert!(first_run.restated.is_empty() && first_run.resumed.is_empty());
    assert_eq!(journal_text(&store), before, "nothing written");
    let second_run = drive_from(&run, quiet(), |_, sink| {
        sink(share_file("ZQA"));
        held_day()
    })
    .unwrap();
    assert_eq!(second_run.restated_skipped, 0);
    assert_eq!(second_run.restated, [april(1)]);
    assert_eq!(second_run.resumed, [april(1)]);
    assert_eq!(second_run.days_imported, 1);
    std::fs::remove_dir_all(store).unwrap();
}

/// A KEY WITH NO DAY WORD NAMES NO RESTATED DAY (D-4878 item 10): the auction
/// era's admission check reads the key's day from it, so a key that holds no
/// space reads no share of any day, while the whole-tree key of a share's day
/// closed under definition 3 stands for every share of it.
#[test]
fn a_key_with_no_day_word_reads_no_restated_share() {
    let store = scratch("ps1-no-day-word");
    let on = Day::new(2026, 8, 4).unwrap();
    put_journal(
        &store,
        &[format!(
            "done stocks {on} * definition=3 files=1 seconds=1 failures=0 late=0 \
             late_unresolved=0 max_back_s=0 run=1\n"
        )],
    );
    let journal = Journal::load(journal_path(&store)).unwrap();
    let never: ScopeHeld<'_> = &|_: &str, _: &str| -> bool {
        unreachable!("a day every share of which stands asks no scope")
    };
    assert!(
        restated_past_append(
            &journal,
            never,
            ImportKind::Stocks,
            &format!("stocks {on} *"),
            on
        ),
        "the whole-tree key under 3 stands for every share of its day"
    );
    assert!(
        !restated_past_append(&journal, never, ImportKind::Stocks, "stocks", on),
        "a key with no day word names no day"
    );
    std::fs::remove_dir_all(store).unwrap();
}

// ── the words of a close (D-3317, D-4878) ──────────────────────────────────

/// THE WORDS OF A MEMBER'S INCOMPLETE CLOSE, AND THE WORSE OF TWO CLOSES
/// (D-4878): a member key's `incomplete` close is the member's, never the
/// day's; a day read again for its members closes as the worst of them, done
/// best and refused worst.
#[test]
fn a_members_incomplete_close_is_its_own_and_the_worse_close_wins() {
    let member = Closed::Member {
        member: "NSE-CASH-ZQB",
        key: "stocks 2024-04-01 *+NSE-CASH-ZQB",
    };
    assert_eq!(
        summary_words(member, Outcome::Incomplete),
        "member incomplete"
    );
    assert_eq!(
        summary_words(Closed::Day, Outcome::Incomplete),
        "day incomplete"
    );
    for (a, b, worst) in [
        (
            Outcome::DoneWithRefusals,
            Outcome::Incomplete,
            Outcome::Incomplete,
        ),
        (
            Outcome::Incomplete,
            Outcome::DoneWithRefusals,
            Outcome::Incomplete,
        ),
        (
            Outcome::Done,
            Outcome::DoneWithRefusals,
            Outcome::DoneWithRefusals,
        ),
        (Outcome::Refused, Outcome::Incomplete, Outcome::Refused),
    ] {
        assert_eq!(worse(a, b), worst, "{a:?} and {b:?}");
    }
}

/// A DAY READ AGAIN FOR THREE MEMBERS IS NAMED ONCE IN EACH LIST (D-4878,
/// D-6900 keep item 2): a member closed done with refusals and two closed
/// `incomplete` on a store hole put the day once in `closed_with_refusals`,
/// once in `closed_incomplete` and once in `holes_made`; each member key is
/// told as the member's, and the day once, as the worst of its members'
/// closes.
#[test]
fn a_day_read_again_for_three_members_is_named_once_and_told_by_its_worst_close() {
    let store = scratch("ps1-three-members");
    let narrow = cash_scope(&["ZQA"]);
    let now = cash_scope(&["ZQA", "ZQB", "ZQC", "ZQD"]);
    let key = format!("stocks {} *", april(1));
    put_journal(&store, &[done_under(&key, narrow.digest())]);
    record_scopes(&store, ImportKind::Stocks, &[&narrow]);
    for share in ["ZQC", "ZQD"] {
        plant(&store, share, FailKind::NotHeld);
    }
    let _earlier = tests::summaries_taken();
    let run = run_over(ImportKind::Stocks, &store, (april(1), april(1)));
    let report = drive_scoped(&run, (&now, &narrow), quiet(), |on, filer| {
        let ask = filer.ask.clone();
        match ask {
            Ask::Probe(beyond, _) => (
                held_day(),
                DayExtras {
                    present: sorted(&beyond),
                    ..DayExtras::default()
                },
            ),
            Ask::One(member) => {
                let share = crate::gdfl_cm::member_share(&member);
                file_these(filer, on, vec![share_file(share)]);
                let refused = if share == "ZQB" {
                    vec![Refused::at(
                        Cause::vendor("ticker_mismatch"),
                        failure("ZQB.invented", "an invented ticker refusal".to_owned()),
                    )]
                } else {
                    Vec::new()
                };
                (
                    DayRead {
                        held: true,
                        refused,
                        ..DayRead::default()
                    },
                    DayExtras::default(),
                )
            }
            Ask::All => panic!("the day was closed done: read again for its new members only"),
        }
    })
    .unwrap();
    let day_one = april(1);
    assert_eq!(report.closed_with_refusals, [day_one]);
    assert_eq!(
        report.closed_incomplete,
        [day_one],
        "two member keys closed incomplete: the day is named once"
    );
    assert_eq!(
        report.holes_made,
        [(day_one, FailKind::NotHeld.code().to_owned())],
        "two holes on one day: the day is named once"
    );
    assert_eq!(
        (
            report.days_widened,
            report.members_reopened,
            report.days_imported
        ),
        (1, 3, 1)
    );
    let told: Vec<(String, Option<String>)> = tests::summaries_taken()
        .into_iter()
        .map(|one| (one.words, one.text.get("member").cloned()))
        .collect();
    assert_eq!(
        told,
        [
            ("member imported".to_owned(), Some(cash("ZQB"))),
            ("member incomplete".to_owned(), Some(cash("ZQC"))),
            ("member incomplete".to_owned(), Some(cash("ZQD"))),
            ("day incomplete".to_owned(), None),
        ]
    );
    for share in ["ZQC", "ZQD"] {
        let member_key = format!("{key}+{}", cash(share));
        assert!(
            last_line_of(&store, &member_key).is_some_and(|line| line
                .starts_with(&format!("incomplete {member_key} "))
                && line.contains(&format!(" reason={} ", FailKind::NotHeld.code()))),
            "{}",
            journal_text(&store)
        );
    }
    std::fs::remove_dir_all(store).unwrap();
}

/// A DAY WITH NO SESSION HAS NO DAY LEDGER (D-3335): a member key read again
/// on a day the calendar holds as no session (a Saturday an earlier build
/// closed `done`) writes no day-ledger line, while the same member read on a
/// session day writes its index's line; neither day names a failure.
#[test]
fn a_member_read_on_a_day_with_no_session_writes_no_day_ledger_line() {
    let store = scratch("ps1-flags-no-session");
    let (friday, saturday) = (april(5), april(6));
    let narrow = scope::Record::of(vec![index_member("ZQA")]);
    let now = scope::Record::of(vec![index_member("ZQA"), index_member("ZQB")]);
    put_journal(
        &store,
        &[
            done_under(&format!("indices {friday} *"), narrow.digest()),
            done_under(&format!("indices {saturday} *"), narrow.digest()),
        ],
    );
    record_scopes(&store, ImportKind::Indices, &[&narrow]);
    let run = run_over(ImportKind::Indices, &store, (friday, saturday));
    let report = drive_scoped(&run, (&now, &narrow), quiet(), |on, filer| {
        let ask = filer.ask.clone();
        match ask {
            Ask::Probe(beyond, _) => (
                held_day(),
                DayExtras {
                    present: sorted(&beyond),
                    ..DayExtras::default()
                },
            ),
            Ask::One(member) => {
                assert_eq!(member, index_member("ZQB"));
                file_these(filer, on, vec![index_file("ZQB")]);
                (held_day(), DayExtras::default())
            }
            Ask::All => panic!("both days were closed done: read again for ZQB only"),
        }
    })
    .unwrap();
    assert_eq!(report.days_widened, 2, "both days read again for ZQB");
    assert_eq!(report.failures_total, 0, "{:?}", report.failures);
    assert_eq!(report.flags_lines, 1, "the Friday's line alone");
    let flags = std::fs::read_to_string(quality::dayflags_path(&store)).unwrap();
    let lines: Vec<&str> = flags.lines().collect();
    let [line] = lines.as_slice() else {
        panic!("{flags}");
    };
    assert!(
        line.starts_with(&format!("flags indices {friday} ZQB v=")),
        "{line}"
    );
    assert!(
        last_line_of(
            &store,
            &format!("indices {saturday} *+{}", index_member("ZQB"))
        )
        .is_some(),
        "the Saturday's member key was read and closed"
    );
    std::fs::remove_dir_all(store).unwrap();
}

// ── the run's end (DB-A50, D-4853, FM-125) ─────────────────────────────────

/// THE DAY LOOP'S OWN WARNINGS REACH A SINK WITH THEIR FIELDS (D-4878, IA-16,
/// DB-A50, the leftover DB-A41 of round 3): a run whose key another run of
/// this process holds says so once, by name, and leaves the key alone; a skip
/// that is no hole says nothing, while a hole skip says its reason and whether
/// a repair fills it; a close naming an unrecorded scope is said once a run
/// however many days name it; and an end line the journal refuses is said at
/// `error` with the run, its outcome and why. Run in a child process, where
/// the one sink of a process can be installed.
#[test]
fn the_day_loops_warnings_reach_a_sink_with_their_fields() {
    crate::support::where_permission_binds(
        "gdfl_import::day::ps1_tests::the_day_loops_warnings_reach_a_sink_with_their_fields",
        the_day_loops_warnings_reach_a_sink_with_their_fields_body,
    );
}

/// The body of `the_day_loops_warnings_reach_a_sink_with_their_fields`, run in
/// its child, where the sink is installed.
fn the_day_loops_warnings_reach_a_sink_with_their_fields_body() {
    let home = scratch("ps1-day-sink");
    let logs =
        telemetry::install(&telemetry::Config::new(&home).with_min_level(telemetry::Level::Trace))
            .expect("the only install in this process");

    // A run key another run holds, and an end line the journal refuses.
    assert!(logs.claim_run(HOLDER), "no run held the key before");
    let store = scratch("ps1-day-sink-store");
    let breaking = BreakJournal::at(&store);
    let options = RunOptions {
        run: Some(SINK_RUN),
        observer: Some(&breaking),
        ..quiet()
    };
    let report = drive_from(
        &run_over(ImportKind::Indices, &store, (april(1), april(1))),
        options,
        |_, sink| {
            sink(index_file("ZQA"));
            held_day()
        },
    )
    .unwrap();
    assert_eq!(report.days_imported, 1);
    assert_eq!(
        logs.run(),
        HOLDER,
        "the run neither took nor gave back a key it did not hold"
    );
    logs.release_run(HOLDER);

    // Each skip: a done day's, a restated day's, and two holes'.
    let bare = scratch("ps1-day-sink-bare");
    let journal = Journal::load(journal_path(&bare)).unwrap();
    let run = run_over(ImportKind::Stocks, &bare, (april(1), april(4)));
    let ctx = bare_ctx(&run, &journal, quiet());
    for (on, skip) in [
        (april(1), Skip::Done),
        (april(2), Skip::Restated),
        (april(3), Skip::Hole(REPAIRABLE_HOLES[0])),
        (april(4), Skip::Unrepairable(HOLE_REASONS[3])),
    ] {
        note_hole_skipped(&ctx, (&format!("stocks {on} *"), on), skip);
    }

    // A close naming an unrecorded scope, on two days.
    let unrecorded = unrecorded_store("ps1-day-sink-unrecorded");
    let closes = Journal::load(journal_path(&unrecorded)).unwrap();
    let (now, swept) = (cash_scope(&["ZQA", "ZQB"]), cash_scope(&["ZQA"]));
    let scoped_run = run_over(ImportKind::Stocks, &unrecorded, (april(1), april(2)));
    let scoped = scoped_ctx(&scoped_run, &closes, (&now, &swept), quiet());
    for on in [april(1), april(2)] {
        let key = format!("stocks {on} *");
        assert!(matches!(scoped.owed(&own(&key, on, false)), Owed::Probe(_)));
    }

    let records = telemetry::tail(
        &home,
        logs.keep_files(),
        &telemetry::Query::last(telemetry::MAX_LIMIT).from_target(TARGET),
    )
    .records;
    let of = |message: &str| -> Vec<&telemetry::Record> {
        records
            .iter()
            .filter(|record| record.message == message)
            .collect()
    };
    let coded = |code: &str| -> Vec<&telemetry::Record> {
        records
            .iter()
            .filter(|record| text(record, "code").as_deref() == Some(code))
            .collect()
    };
    let key_held = of("run key held by another run");
    let [held] = key_held.as_slice() else {
        panic!("{key_held:?}");
    };
    assert_eq!(
        (count(held, "run"), count(held, "holder")),
        (Some(SINK_RUN), Some(HOLDER))
    );
    let run_finished = of("run finished");
    let [finished] = run_finished.as_slice() else {
        panic!("{run_finished:?}");
    };
    assert_eq!(
        (finished.run, count(finished, "run")),
        (HOLDER, Some(SINK_RUN)),
        "the run's events carry the holder's key, never one it did not take"
    );
    let not_written = of("run end line not written");
    let [unwritten] = not_written.as_slice() else {
        panic!("{not_written:?}");
    };
    assert_eq!(
        (count(unwritten, "run"), text(unwritten, "outcome")),
        (Some(SINK_RUN), Some("finished".to_owned()))
    );
    assert!(
        text(unwritten, "why").is_some_and(|why| why.contains("gdfl.journal")),
        "{unwritten:?}"
    );
    let mut skipped: Vec<(Option<String>, Option<bool>)> = coded("hole_skipped")
        .into_iter()
        .map(|record| (text(record, "day"), flag(record, "repairable")))
        .collect();
    skipped.sort();
    assert_eq!(
        skipped,
        [
            (Some(april(3).to_string()), Some(true)),
            (Some(april(4).to_string()), Some(false)),
        ],
        "a done day and a restated day are no holes: nothing said of them"
    );
    let unrecorded_said = coded("scope_unrecorded");
    let [said] = unrecorded_said.as_slice() else {
        panic!("{unrecorded_said:?}");
    };
    assert_eq!(text(said, "kind"), Some("stocks".to_owned()));
}

/// THE RUN'S END LINE, REFUSED, LEAVES THE RUN'S END AS IT WAS (DB-A50): a
/// journal that took the run's `run` line and its day's close and then will
/// not take the end line (a directory now stands at its name) changes nothing
/// of how the run ends: its whole report, told to the observer.
#[test]
fn a_journal_that_refuses_the_end_line_leaves_the_runs_end_as_it_was() {
    let store = scratch("ps1-end-line");
    let breaking = BreakJournal::at(&store);
    let options = RunOptions {
        run: Some(1_790_100_201),
        observer: Some(&breaking),
        ..quiet()
    };
    let report = drive_from(
        &run_over(ImportKind::Indices, &store, (april(1), april(1))),
        options,
        |_, sink| {
            sink(index_file("ZQA"));
            held_day()
        },
    )
    .unwrap();
    assert_eq!((report.days_imported, report.failures_total), (1, 0));
    assert!(
        breaking.ended_ok.load(std::sync::atomic::Ordering::SeqCst),
        "the observer was told the report"
    );
    let said = breaking.said.lock().unwrap().clone();
    assert!(
        said.starts_with("run 1790100201 ")
            && said.contains(&format!("\ndone indices {} * ", april(1))),
        "{said}"
    );
    assert!(!said.contains(" ended="), "{said}");
    assert!(
        journal_path(&store).is_dir(),
        "the end line was written nowhere"
    );
    std::fs::remove_dir_all(store).unwrap();
}

/// A STOP ASKED AFTER THE RUN'S START IS SEEN AT THE FIRST DAY'S TOP (D-4853):
/// the stop is asked as the run starts and again before each day, so a stop
/// that came after the first ask ends the run `Stopped` with no day, before
/// anything of the first day is asked or written.
#[test]
fn a_stop_asked_after_the_runs_start_ends_it_before_its_first_day() {
    let store = scratch("ps1-stop-top");
    let stop = crate::gdfl_run::stop::StopControl::tripping_at(
        store.join("imports").join("gdfl.stop"),
        1_790_100_301,
        2,
    );
    let options = RunOptions {
        run: Some(1_790_100_301),
        stop: Some(&stop),
        ..quiet()
    };
    let got = drive_from(
        &run_over(ImportKind::Stocks, &store, (april(1), april(2))),
        options,
        |_, _| -> DayRead { panic!("no day is begun") },
    )
    .unwrap_err();
    let ImportRefusal::Stopped {
        kind,
        day,
        by,
        requested,
        done,
    } = &got
    else {
        panic!("{got:?}");
    };
    assert_eq!(
        (*kind, *day, by.as_str(), *requested),
        (ImportKind::Stocks, None, "operator", 0)
    );
    assert_eq!(**done, Report::default(), "nothing was done");
    assert_eq!(
        stop.checks(),
        2,
        "asked as the run started, then at the first day's top"
    );
    assert!(!journal_path(&store).exists(), "no line: no day was begun");
    std::fs::remove_dir_all(store).unwrap();
}

// ── the store root (IA-16 repair 1; FM-125, RA-284, D-4852) ────────────────

/// A ROOT BELOW A FILE IS NO FRESH STORE (FM-125, D-4852): `fresh_root` makes
/// a root only where the host finds nothing, and below a file it answers "not
/// a directory", so nothing is made and the journal's load refuses the run by
/// name, the file untouched.
#[test]
fn a_root_below_a_file_is_never_made_and_the_journal_refuses_it() {
    let base = scratch("ps1-root-below-file");
    let file = put(&base, "plain", b"not a folder");
    let root = file.join("S");
    let run = run_over(ImportKind::Stocks, &root, (april(1), april(1)));
    assert_eq!(fresh_root(&run), Ok(()), "left to the journal's load");
    let got = drive(&run, |_, _| -> DayRead {
        panic!("a refused run reads no day")
    })
    .unwrap_err();
    let ImportRefusal::Journal { why } = &got else {
        panic!("{got:?}");
    };
    assert!(
        why.starts_with(&format!("{}: ", journal_path(&root).display())),
        "{why}"
    );
    assert_eq!(
        std::fs::read(file).unwrap(),
        b"not a folder",
        "the file is untouched"
    );
    std::fs::remove_dir_all(base).unwrap();
}

/// A RELATIVE ROOT WHOSE PARENT IS THE WORKING FOLDER (FM-125, D-4852): a
/// one-component root's parent is the empty path, which a run takes for the
/// working folder; a root that cannot be made there (`<absent>/.`: the host
/// finds nothing at it and will not make it, its folder being absent) is
/// refused `store_lost` in the host's words, never as a root without a parent,
/// and nothing is made in the working folder.
#[test]
fn a_relative_root_that_cannot_be_made_is_refused_and_nothing_is_made() {
    let absent = format!("zq-ps1-absent-{}-{}", std::process::id(), nanos());
    let root = PathBuf::from(format!("{absent}/."));
    assert_eq!(
        root.parent(),
        Some(Path::new("")),
        "its parent is the working folder"
    );
    let got = drive(
        &run_over(ImportKind::Stocks, &root, (april(1), april(1))),
        |_, _| -> DayRead { panic!("a refused run reads no day") },
    )
    .unwrap_err();
    let ImportRefusal::StoreLost {
        kind,
        day,
        why,
        done,
    } = &got
    else {
        panic!("{got:?}");
    };
    assert_eq!((*kind, *day), (ImportKind::Stocks, april(1)));
    assert!(
        why.starts_with(&format!(
            "{} is not a store root a run can begin: ",
            root.display()
        )),
        "{why}"
    );
    assert!(
        !why.contains("its parent is not a directory"),
        "the working folder is a parent: {why}"
    );
    assert_eq!(**done, Report::default());
    assert!(
        !Path::new(&absent).exists(),
        "nothing was made in the working folder"
    );
}

/// A DANGLING LINK NAMED WITH A TRAILING SLASH IS NO FRESH STORE (FM-125,
/// D-4852): the host finds nothing at it (the slash follows the link to
/// nothing), then will not make it (the link itself is there); the root is not
/// a directory after that refusal, so the run is refused `store_lost` and the
/// link's target is never made. Linux's answers; Darwin's `mkdir` on such a
/// name is UNVERIFIED here, so the test is Linux's.
#[cfg(target_os = "linux")]
#[test]
fn a_dangling_link_named_with_a_trailing_slash_is_refused_and_its_target_never_made() {
    let base = scratch("ps1-root-dangling-slash");
    let target = base.join("nowhere");
    std::os::unix::fs::symlink(&target, base.join("link")).unwrap();
    let root = base.join("link/");
    let run = run_over(ImportKind::Stocks, &root, (april(1), april(1)));
    let got = fresh_root(&run).unwrap_err();
    let ImportRefusal::StoreLost {
        kind,
        day,
        why,
        done,
    } = &got
    else {
        panic!("{got:?}");
    };
    assert_eq!((*kind, *day), (ImportKind::Stocks, april(1)));
    assert!(
        why.starts_with(&format!(
            "{} is not a store root a run can begin: ",
            root.display()
        )) && why.contains("File exists"),
        "{why}"
    );
    assert_eq!(**done, Report::default());
    assert!(!target.exists(), "the link's target was never made");
    std::fs::remove_dir_all(base).unwrap();
}

/// A STORE ROOT THAT IS A DANGLING LINK HAS NO IDENTITY AT THE RUN'S START
/// (RA-284, D-4852): the first day's root check learns it then (none: the link
/// still points nowhere) and begins the day; the root asked once more before
/// the day's ledger, census and close is unreachable, so the run ends
/// `store_lost` on that day and no writer makes the link's target.
#[test]
fn a_store_root_that_is_a_dangling_link_is_learned_at_its_first_day_and_lost_before_its_close() {
    let base = scratch("ps1-root-dangling");
    let target = base.join("nowhere");
    let root = base.join("S");
    std::os::unix::fs::symlink(&target, &root).unwrap();
    let mut read = Vec::new();
    let got = drive_from(
        &run_over(ImportKind::Stocks, &root, (april(1), april(2))),
        quiet(),
        |on, _| {
            read.push(on);
            held_day()
        },
    )
    .unwrap_err();
    assert_eq!(read, [april(1)], "the first day is begun and read");
    let ImportRefusal::StoreLost {
        kind,
        day,
        why,
        done,
    } = &got
    else {
        panic!("{got:?}");
    };
    assert_eq!((*kind, *day), (ImportKind::Stocks, april(1)));
    assert!(
        why.starts_with(&format!(
            "after the day's files and before its ledger, census and closing line: the loss \
             probe could not reach {}",
            root.display()
        )),
        "{why}"
    );
    assert_eq!(done.days_imported, 0);
    assert!(!target.exists(), "no writer made the link's target");
    std::fs::remove_dir_all(base).unwrap();
}

// ── the reach back over other filters' keys (D-4878 repair 2, DB-B76) ──────

/// A RUN ENDED BY A LENDER'S MEMBER ENDS BEFORE ITS RANGE (D-4851, D-4878):
/// the store refuses ZQB's file of a day before the range, read for the key
/// `ZQA,ZQB` closed `done`; the member key closes `incomplete`, the run ends
/// `store_not_writable` on that day, and no day of the range is begun.
#[test]
fn a_run_ended_by_a_lenders_member_begins_no_day_of_its_range() {
    let store = scratch("ps1-reach-ends");
    let lender = format!("stocks {} ZQA,ZQB", april(1));
    let (narrow, now) = (cash_scope(&["ZQA"]), cash_scope(&["ZQA", "ZQB"]));
    put_journal(&store, &[done_under(&lender, narrow.digest())]);
    record_scopes(&store, ImportKind::Stocks, &[&narrow]);
    plant(&store, "ZQB", read_only());
    let run = run_over(ImportKind::Stocks, &store, (april(2), april(3)));
    let got = drive_scoped(&run, (&now, &narrow), quiet(), |on, filer| {
        let ask = filer.ask.clone();
        match ask {
            Ask::Probe(beyond, names) => {
                assert_eq!(
                    names.as_deref().map(sorted),
                    Some(vec!["ZQA".to_owned(), "ZQB".to_owned()])
                );
                (
                    held_day(),
                    DayExtras {
                        present: sorted(&beyond),
                        ..DayExtras::default()
                    },
                )
            }
            Ask::One(member) => {
                file_these(
                    filer,
                    on,
                    vec![share_file(crate::gdfl_cm::member_share(&member))],
                );
                (held_day(), DayExtras::default())
            }
            Ask::All => panic!("no day of the range is begun"),
        }
    })
    .unwrap_err();
    let ImportRefusal::StoreNotWritable {
        kind,
        day,
        why,
        done,
    } = &got
    else {
        panic!("{got:?}");
    };
    assert_eq!((*kind, *day), (ImportKind::Stocks, april(1)));
    assert!(why.contains("planted by the test"), "{why}");
    assert_eq!((done.days_before_range, done.days_widened), (1, 1));
    assert_eq!(done.closed_incomplete, [april(1)]);
    let member_key = format!("{lender}+{}", cash("ZQB"));
    assert!(
        last_line_of(&store, &member_key)
            .is_some_and(|line| line.starts_with(&format!("incomplete {member_key} "))),
        "{}",
        journal_text(&store)
    );
    assert!(
        !journal_text(&store).contains(&april(2).to_string()),
        "no day of the range was begun"
    );
    std::fs::remove_dir_all(store).unwrap();
}

/// A KEY ANOTHER FILTER CLOSED, EXAMINED FOR THIS FILTER, OWES ITS OPEN MEMBER
/// KEYS (D-4878 repair 2): a day before the range closed `done` under `ZQA`,
/// examined for `*` under this scope, whose member key ZQB is `incomplete`, is
/// listed for ZQB alone (not for every member beyond its close's scope) under
/// the lender's names, read first under that key's own member key, and
/// examined again by no one.
#[test]
fn a_lenders_open_member_key_is_read_first_for_the_run_that_examined_it() {
    let store = scratch("ps1-reach-pending");
    let lender = format!("stocks {} ZQA", april(1));
    let (narrow, now) = (cash_scope(&["ZQA"]), cash_scope(&["ZQA", "ZQB", "ZQC"]));
    put_journal(
        &store,
        &[
            done_under(&lender, narrow.digest()),
            closing(
                &format!("incomplete {lender}+{}", cash("ZQB")),
                " stage=file reason=fold",
            ),
        ],
    );
    record_scopes(&store, ImportKind::Stocks, &[&narrow, &now]);
    scope::Ledger::load(scope::scopes_path(&store))
        .unwrap()
        .examine(&format!("{lender}/*"), now.digest(), (1, 1))
        .unwrap();
    let ledger_before = scopes_text(&store);
    let _earlier = tests::summaries_taken();
    let mut probed = Vec::new();
    let run = run_over(ImportKind::Stocks, &store, (april(2), april(2)));
    let report = drive_scoped(&run, (&now, &narrow), quiet(), |on, filer| {
        let ask = filer.ask.clone();
        match ask {
            Ask::Probe(pending, names) => {
                probed.push((sorted(&pending), names.as_deref().map(sorted)));
                (
                    held_day(),
                    DayExtras {
                        present: sorted(&pending),
                        ..DayExtras::default()
                    },
                )
            }
            Ask::One(member) => {
                file_these(
                    filer,
                    on,
                    vec![share_file(crate::gdfl_cm::member_share(&member))],
                );
                (held_day(), DayExtras::default())
            }
            Ask::All => (held_day(), DayExtras::default()),
        }
    })
    .unwrap();
    assert_eq!(
        probed,
        [(vec![cash("ZQB")], Some(vec!["ZQA".to_owned()]))],
        "the open member key alone, under the lender's names"
    );
    assert_eq!(
        (
            report.days_before_range,
            report.days_widened,
            report.members_reopened
        ),
        (1, 1, 0),
        "read first, and no examination counted"
    );
    assert_eq!(
        report.resumed,
        [april(1)],
        "the member key an earlier run left incomplete"
    );
    let member_key = format!("{lender}+{}", cash("ZQB"));
    assert!(
        last_line_of(&store, &member_key)
            .is_some_and(|line| line.starts_with(&format!("done {member_key} "))),
        "{}",
        journal_text(&store)
    );
    assert_eq!(
        scopes_text(&store),
        ledger_before,
        "a pending key is examined again by no one"
    );
    let reach: Vec<_> = tests::summaries_taken()
        .into_iter()
        .filter(|told| told.text.get("code").map(String::as_str) == Some("scope_reach_back"))
        .collect();
    let [said] = reach.as_slice() else {
        panic!("{reach:?}");
    };
    assert_eq!(said.text.get("key"), Some(&lender));
    assert_eq!(said.text.get("first_owed"), Some(&april(1).to_string()));
    std::fs::remove_dir_all(store).unwrap();
}

/// A DAY TWO LENDERS OWE IS COUNTED BEFORE THE RANGE ONCE (D-4878, DB-B76):
/// the day before the range closed `done` under `ZQA,ZQB` and under `ZQC,ZQD`
/// owes each key the member its names hold and its scope never covered; both
/// are read, each under its own member key and examined for this filter, and
/// the day is one day before the range, said once.
#[test]
fn a_day_two_lenders_owe_is_counted_before_the_range_once() {
    let store = scratch("ps1-two-lenders");
    let (first, second) = (
        format!("stocks {} ZQA,ZQB", april(1)),
        format!("stocks {} ZQC,ZQD", april(1)),
    );
    let (by_first, by_second) = (cash_scope(&["ZQA"]), cash_scope(&["ZQC"]));
    let now = cash_scope(&["ZQA", "ZQB", "ZQC", "ZQD"]);
    put_journal(
        &store,
        &[
            done_under(&first, by_first.digest()),
            done_under(&second, by_second.digest()),
        ],
    );
    record_scopes(&store, ImportKind::Stocks, &[&by_first, &by_second]);
    let _earlier = tests::summaries_taken();
    let run = run_over(ImportKind::Stocks, &store, (april(2), april(2)));
    let report = drive_scoped(&run, (&now, &by_first), quiet(), |on, filer| {
        let ask = filer.ask.clone();
        match ask {
            Ask::Probe(beyond, names) => {
                let names = names.expect("each lender of this day names its shares");
                let present = sorted(&beyond)
                    .into_iter()
                    .filter(|one| names.contains(crate::gdfl_cm::member_share(one)))
                    .collect();
                (
                    held_day(),
                    DayExtras {
                        present,
                        ..DayExtras::default()
                    },
                )
            }
            Ask::One(member) => {
                file_these(
                    filer,
                    on,
                    vec![share_file(crate::gdfl_cm::member_share(&member))],
                );
                (held_day(), DayExtras::default())
            }
            Ask::All => (held_day(), DayExtras::default()),
        }
    })
    .unwrap();
    assert_eq!(
        (
            report.days_before_range,
            report.days_widened,
            report.members_reopened
        ),
        (1, 2, 2),
        "two keys read again, one day before the range"
    );
    for (lender, share) in [(&first, "ZQB"), (&second, "ZQD")] {
        let member_key = format!("{lender}+{}", cash(share));
        assert!(
            last_line_of(&store, &member_key)
                .is_some_and(|line| line.starts_with(&format!("done {member_key} "))),
            "{}",
            journal_text(&store)
        );
    }
    let reach: Vec<String> = tests::summaries_taken()
        .into_iter()
        .filter(|told| told.text.get("code").map(String::as_str) == Some("scope_reach_back"))
        .filter_map(|told| told.text.get("key").cloned())
        .collect();
    assert_eq!(
        reach,
        [first.as_str()],
        "said once, naming the first key that owed it"
    );
    std::fs::remove_dir_all(store).unwrap();
}

// ── the listing and the member keys of a widened day (D-4878) ──────────────

/// A LISTING THAT CANNOT BE ASKED LEAVES A DONE DAY UNEXAMINED (D-4878): a day
/// the source no longer holds, and one whose listing it refuses, are each one
/// named `scope_reopen_unreadable` failure of the run, in the source's words
/// when it gave any; neither day is read, counted, written or examined, so the
/// next run asks again.
#[test]
fn a_listing_the_source_cannot_give_is_named_and_leaves_the_day_unexamined() {
    let store = scratch("ps1-probe-unreadable");
    let (narrow, now) = (cash_scope(&["ZQA"]), cash_scope(&["ZQA", "ZQB"]));
    put_journal(
        &store,
        &[
            done_under(&format!("stocks {} *", april(1)), narrow.digest()),
            done_under(&format!("stocks {} *", april(2)), narrow.digest()),
        ],
    );
    record_scopes(&store, ImportKind::Stocks, &[&narrow]);
    let (journal_before, ledger_before) = (journal_text(&store), scopes_text(&store));
    let run = run_over(ImportKind::Stocks, &store, (april(1), april(2)));
    let report = drive_scoped(&run, (&now, &narrow), quiet(), |on, filer| {
        assert!(matches!(filer.ask, Ask::Probe(..)), "{:?}", filer.ask);
        let read = if on == april(1) {
            DayRead::default()
        } else {
            DayRead {
                held: true,
                refused: vec![Refused::at(
                    Cause::open(Stage::Vendor, "listing_unreadable"),
                    failure("listing", "an invented listing refusal".to_owned()),
                )],
                ..DayRead::default()
            }
        };
        (read, DayExtras::default())
    })
    .unwrap();
    let named: Vec<(String, String)> = report
        .failures
        .iter()
        .map(|one| (one.instrument.clone(), one.why.clone()))
        .collect();
    assert_eq!(
        named,
        [
            (
                format!("{} listing", april(1)),
                "the journal closed this day done and the source holds no file of it now, so \
                 the members a wider scope added cannot be read"
                    .to_owned()
            ),
            (
                format!("{} listing", april(2)),
                "an invented listing refusal".to_owned()
            ),
        ]
    );
    assert_eq!(
        report
            .failures_by_cause
            .get(&("vendor", "scope_reopen_unreadable")),
        Some(&2)
    );
    assert_eq!(
        (
            report.days_widened,
            report.days_imported,
            report.days_skipped
        ),
        (0, 0, 0)
    );
    assert_eq!(
        journal_text(&store),
        journal_before,
        "nothing of either day written"
    );
    assert_eq!(scopes_text(&store), ledger_before, "neither day examined");
    std::fs::remove_dir_all(store).unwrap();
}

/// A MEMBER KEY WHOSE DAY THE SOURCE NO LONGER HOLDS CLOSES NOTHING OF THE DAY
/// (D-4878): listed with a file of ZQB, the day held no file when ZQB's own
/// key read it; that key is closed `refused day_not_held`, the day is named
/// missing, and it is neither counted imported nor told.
#[test]
fn a_member_whose_day_is_gone_when_read_closes_nothing_of_the_day() {
    let store = scratch("ps1-member-gone");
    let (narrow, now) = (cash_scope(&["ZQA"]), cash_scope(&["ZQA", "ZQB"]));
    let key = format!("stocks {} *", april(1));
    put_journal(&store, &[done_under(&key, narrow.digest())]);
    record_scopes(&store, ImportKind::Stocks, &[&narrow]);
    let _earlier = tests::summaries_taken();
    let run = run_over(ImportKind::Stocks, &store, (april(1), april(1)));
    let report = drive_scoped(&run, (&now, &narrow), quiet(), |_, filer| {
        let ask = filer.ask.clone();
        match ask {
            Ask::Probe(beyond, _) => (
                held_day(),
                DayExtras {
                    present: sorted(&beyond),
                    ..DayExtras::default()
                },
            ),
            Ask::One(member) => {
                assert_eq!(member, cash("ZQB"));
                (DayRead::default(), DayExtras::default())
            }
            Ask::All => panic!("the day was closed done: read again for ZQB only"),
        }
    })
    .unwrap();
    assert_eq!(report.days_missing, [april(1)]);
    assert_eq!(
        (
            report.days_widened,
            report.members_reopened,
            report.days_imported
        ),
        (1, 1, 0)
    );
    assert!(
        tests::summaries_taken().is_empty(),
        "no member close and no day is told"
    );
    let member_key = format!("{key}+{}", cash("ZQB"));
    assert!(
        last_line_of(&store, &member_key).is_some_and(|line| line.starts_with(&format!(
            "refused {member_key} definition={BAR_DEFINITION} stage=vendor reason=day_not_held \
             failures=0 run="
        ))),
        "{}",
        journal_text(&store)
    );
    std::fs::remove_dir_all(store).unwrap();
}

/// `note_examined` WITHOUT A SCOPE RECORDS NOTHING (D-4878): a run that
/// records no scope has no ledger to examine a day in, so the call counts no
/// member, writes no `run` line and leaves the journal absent.
#[test]
fn an_examination_without_a_scope_counts_and_writes_nothing() {
    let store = scratch("ps1-examined-unscoped");
    let journal = Journal::load(journal_path(&store)).unwrap();
    let run = run_over(ImportKind::Stocks, &store, (april(1), april(1)));
    let ctx = bare_ctx(&run, &journal, quiet());
    let key = format!("stocks {} *", april(1));
    let mut report = Report::default();
    note_examined(&ctx, &own(&key, april(1), false), 3, &mut report);
    assert_eq!(report, Report::default(), "no member counted");
    assert!(!ctx.announced.get(), "no run line");
    assert!(!journal_path(&store).exists());
    std::fs::remove_dir_all(store).unwrap();
}

/// A SCOPE LEDGER THAT WILL NOT TAKE AN EXAMINATION IS A NAMED FAILURE OF THE
/// RUN (D-4878): the `run` line is written first, the examination is not, and
/// the next run, the ledger writable again, lists the day again and examines
/// it.
#[test]
fn an_examination_the_ledger_refuses_is_named_and_asked_again_next_run() {
    let store = scratch("ps1-examined-refused");
    let (narrow, now) = (cash_scope(&["ZQA"]), cash_scope(&["ZQA", "ZQB"]));
    let key = format!("stocks {} *", april(1));
    put_journal(&store, &[done_under(&key, narrow.digest())]);
    record_scopes(&store, ImportKind::Stocks, &[&narrow]);
    let ledger = scope::scopes_path(&store);
    let recorded = scopes_text(&store);
    let run = run_over(ImportKind::Stocks, &store, (april(1), april(1)));
    let refused = drive_scoped(&run, (&now, &narrow), quiet(), |_, filer| {
        assert!(matches!(filer.ask, Ask::Probe(..)), "{:?}", filer.ask);
        std::fs::remove_file(&ledger).unwrap();
        std::fs::create_dir(&ledger).unwrap();
        (held_day(), DayExtras::default())
    })
    .unwrap();
    let [named] = refused.failures.as_slice() else {
        panic!("{:?}", refused.failures);
    };
    assert_eq!(named.instrument, format!("{} scope ledger", april(1)));
    assert!(
        named.why.contains(&ledger.display().to_string()),
        "{}",
        named.why
    );
    assert_eq!(
        refused.failures_by_cause.get(&("journal", "scope_ledger")),
        Some(&1)
    );
    assert_eq!((refused.days_widened, refused.members_reopened), (1, 0));
    assert!(
        journal_text(&store)
            .lines()
            .any(|line| line.starts_with("run ")),
        "the run line is written before the ledger is asked"
    );

    std::fs::remove_dir(&ledger).unwrap();
    std::fs::write(&ledger, recorded).unwrap();
    let mut listed = 0;
    let again = drive_scoped(&run, (&now, &narrow), quiet(), |_, filer| {
        assert!(matches!(filer.ask, Ask::Probe(..)), "{:?}", filer.ask);
        listed += 1;
        (held_day(), DayExtras::default())
    })
    .unwrap();
    assert_eq!(listed, 1, "listed again");
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert!(ledger.is_file());
    assert!(
        scopes_text(&store).contains(&format!("examined {key} scope={} run=", now.digest())),
        "{}",
        scopes_text(&store)
    );
    std::fs::remove_dir_all(store).unwrap();
}

// ── a lost source and a store that takes no writes (IA-16; D-4851, D-4852) ─

/// A SOURCE LOST AFTER A FILE LANDED CLOSES THE DAY `incomplete stage=run
/// reason=source_lost` AND ENDS THE RUN (D-4852, FM-086, INFRA-024): the day's
/// file landed, then the source's root went away and a file it could not reach
/// was refused; the loss probe names the root, the day's close takes the run's
/// cause before the file's, and no later day is read or named not held.
#[test]
fn a_source_lost_after_a_file_landed_closes_the_day_incomplete_and_ends_the_run() {
    let root = scratch("ps1-source-lost");
    let (store, source) = (root.join("S"), root.join("source"));
    std::fs::create_dir(&source).unwrap();
    let origin = Origin {
        source: SourceKind::Zip,
        path: source.clone(),
        archive_bytes: 0,
        digest: None,
        commit: None,
        started: 1_790_100_000,
        fault: None,
    };
    let options = RunOptions {
        origin: Some(&origin),
        ..quiet()
    };
    let mut read = Vec::new();
    let got = drive_from(
        &run_over(ImportKind::Stocks, &store, (april(1), april(2))),
        options,
        |on, sink| {
            read.push(on);
            sink(share_file("ZQA"));
            std::fs::remove_dir(&source).unwrap();
            DayRead {
                held: true,
                refused: vec![Refused::at(
                    Cause::open(Stage::Vendor, "tick_store_unavailable"),
                    failure(
                        "ZQB.invented",
                        "the invented tick store went away mid-day".to_owned(),
                    ),
                )],
                ..DayRead::default()
            }
        },
    )
    .unwrap_err();
    assert_eq!(read, [april(1)], "no day after the loss is read");
    let ImportRefusal::SourceLost {
        kind,
        day,
        why,
        done,
    } = &got
    else {
        panic!("{got:?}");
    };
    assert_eq!((*kind, *day), (ImportKind::Stocks, april(1)));
    assert!(
        why.starts_with(&format!(
            "the loss probe could not reach the source's root {}",
            source.display()
        )),
        "{why}"
    );
    assert_eq!(done.closed_incomplete, [april(1)]);
    assert_eq!(done.files, 1, "the day's file landed before the loss");
    assert!(done.days_missing.is_empty(), "no day is named not held");
    let key = format!("stocks {} *", april(1));
    assert!(
        last_line_of(&store, &key)
            .is_some_and(|line| line.starts_with(&format!("incomplete {key} "))
                && line.contains(" stage=run reason=source_lost ")),
        "{}",
        journal_text(&store)
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A STORE THAT TAKES NO WRITES ENDS THE RUN ONCE THE DAY IS CLOSED (FM-129,
/// D-4851): the host refuses the day's file for a read-only volume (the inject
/// seam's answer, the real path after it), the day closes `incomplete`, the
/// run ends `store_not_writable` naming the fix, and the next day is never
/// read.
#[test]
fn a_store_that_takes_no_writes_ends_the_run_once_the_day_is_closed() {
    let store = scratch("ps1-not-writable");
    plant(&store, "ZQA", read_only());
    let mut read = Vec::new();
    let got = drive_from(
        &run_over(ImportKind::Stocks, &store, (april(1), april(2))),
        quiet(),
        |on, sink| {
            read.push(on);
            sink(share_file("ZQA"));
            held_day()
        },
    )
    .unwrap_err();
    assert_eq!(read, [april(1)], "the next day is never read");
    let ImportRefusal::StoreNotWritable {
        kind,
        day,
        why,
        done,
    } = &got
    else {
        panic!("{got:?}");
    };
    assert_eq!((*kind, *day), (ImportKind::Stocks, april(1)));
    assert!(
        why.contains("planted by the test") && why.contains("chflags nouchg"),
        "the store's words and the fix: {why}"
    );
    assert_eq!(done.closed_incomplete, [april(1)]);
    let key = format!("stocks {} *", april(1));
    assert!(
        last_line_of(&store, &key)
            .is_some_and(|line| line.starts_with(&format!("incomplete {key} "))
                && line.contains(" stage=file ")),
        "{}",
        journal_text(&store)
    );
    assert!(!journal_text(&store).contains(&april(2).to_string()));
    std::fs::remove_dir_all(store).unwrap();
}
