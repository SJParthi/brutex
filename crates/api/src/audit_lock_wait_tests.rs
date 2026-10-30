//! THE PULL JOURNAL'S APPEND LOCK WAITS A BOUNDED TIME (AG-06, code-5,
//! FM-162; D-6244).
//!
//! Every append (`append`, `append_member`, `open_run`, `append_failure`,
//! `close_run`, `close_unopened`) took the journal's append lock with
//! `try_lock` and was refused at once while another writer's append held it
//! for its few milliseconds. The record it carried was lost: a run's opening
//! record among them, so a run that then died read "clean" at the next start.
//! Each test here fails on the `try_lock` build (feat/db-fix before AG-06)
//! and passes with the bounded wait, except the ones that pin a bound's
//! refusal, which pin what the wait must still refuse.

#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use super::*;
use pull::session::Day;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

thread_local! {
    /// Armed by [`refuse_next_write`]: the kind the next journal record write
    /// on this thread fails with. Consumed by that one write.
    static REFUSE_WRITE: std::cell::Cell<Option<std::io::ErrorKind>> =
        const { std::cell::Cell::new(None) };
}

/// Makes the next journal record write on this thread fail with `kind`.
fn refuse_next_write(kind: std::io::ErrorKind) {
    REFUSE_WRITE.with(|armed| armed.set(Some(kind)));
}

/// The seam [`super::journal_write`] asks before it writes.
pub(super) fn take_write_refusal() -> Option<std::io::ErrorKind> {
    REFUSE_WRITE.with(std::cell::Cell::take)
}

/// A store root nobody else in this process shares, emptied first.
fn scratch(name: &str) -> PathBuf {
    let dir = crate::scratch::path(&format!("ag06-{name}"));
    let _ignored = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn window() -> Window {
    let day = Day::new(2025, 7, 1).expect("a real date");
    Window::new(day, day).expect("forwards")
}

fn at(secs: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
}

fn opened(source: &str) -> Record {
    Record::opened(Scope::Spot, at(1), source, window(), 2, 0xa5)
}

fn closed(source: &str) -> Record {
    Record::refused(Scope::Spot, Outcome::Stored, at(2), source, "done")
}

fn member(i: usize) -> Record {
    Record::member_failure(Scope::Spot, at(3), &format!("M{i}"), window(), "a gap")
}

/// A journal with one record, so its file exists to be locked.
fn journal_with_one_record(name: &str) -> (PathBuf, Journal) {
    let root = scratch(name);
    let journal = Journal::at(&root);
    journal.append(&closed("seed")).expect("the seed record");
    (root, journal)
}

/// Another writer: a second open file description holding the append lock.
fn hold(journal: &Journal) -> store::flock::Flock<std::fs::File> {
    store::flock::Flock::try_lock(
        std::fs::OpenOptions::new()
            .read(true)
            .append(true)
            .open(&journal.path)
            .expect("the journal opens"),
        journal.path.clone(),
    )
    .expect("nothing else holds it")
}

/// What another writer sees: a fresh description asking without waiting.
fn lock_is_free(journal: &Journal) -> bool {
    store::flock::Flock::try_lock(
        std::fs::File::open(&journal.path).expect("the journal opens"),
        journal.path.clone(),
    )
    .is_ok()
}

/// Every record in the journal, decoded, oldest first.
fn outcomes(journal: &Journal) -> Vec<(Outcome, String)> {
    let n = journal.look().records();
    if n == 0 {
        return Vec::new();
    }
    let mut rows = Vec::new();
    let mut skip = 0;
    while skip < n {
        let page = journal.page(n, skip, n - skip).expect("a page");
        assert!(!page.is_empty(), "a page of a non-empty rest");
        skip += u64::try_from(page.len()).expect("small");
        rows.extend(page);
    }
    rows.reverse();
    rows.into_iter()
        .map(|row| {
            let r = row.decoded.expect("every record decodes");
            (r.outcome, r.source)
        })
        .collect()
}

/// The time the bound may overrun by: one pause, plus scheduling slack on a
/// machine shared by a dozen builds. Generous on purpose: the exact
/// accounting is pinned with an injected clock in `store::flock`.
const SLACK: Duration = Duration::from_millis(750);

#[test]
fn an_opening_append_waits_for_a_held_lock_and_lands() {
    let (root, journal) = journal_with_one_record("waits");
    let holder = hold(&journal);
    let released = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        holder.release().expect("released");
    });
    let started = Instant::now();
    let thread = journal
        .open_run(&opened("waited"))
        .expect("the opening waits for the other writer's append and lands");
    let took = started.elapsed();
    released.join().expect("the holder thread");
    assert_eq!(thread.last(), 1, "the opening is record 1, behind the seed");
    assert_eq!(
        outcomes(&journal),
        vec![
            (Outcome::Stored, "seed".to_owned()),
            (Outcome::Opened, "waited".to_owned()),
        ]
    );
    assert!(
        took >= Duration::from_millis(40),
        "it waited for the holder: {took:?}"
    );
    assert!(took < APPEND_LOCK_WAIT, "within the bound: {took:?}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_append_held_past_the_bound_is_refused_naming_the_bound() {
    let (root, journal) = journal_with_one_record("past-bound");
    let holder = hold(&journal);
    let head = opened("never");
    let started = Instant::now();
    let why = journal
        .open_run(&head)
        .expect_err("a holder that never releases refuses the opening");
    let took = started.elapsed();
    assert!(took >= APPEND_LOCK_WAIT, "it waited the bound: {took:?}");
    assert!(
        took < APPEND_LOCK_WAIT + store::flock::MAX_PAUSE + SLACK,
        "and no longer, never a hang: {took:?}"
    );
    for part in [
        "cannot take the journal append lock",
        &format!(
            "{} ms bound (APPEND_LOCK_WAIT)",
            APPEND_LOCK_WAIT.as_millis()
        ),
        &format!("a {} record ({})", head.kind.label(), head.outcome.label()),
        "still held after waiting",
        "held by another process",
        "refused rather than interleaved",
    ] {
        assert!(why.contains(part), "{part:?} missing from: {why}");
    }
    assert_eq!(journal.look().records(), 1, "nothing of the refusal landed");
    holder.release().expect("released");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_refusal_names_a_holder_in_this_process_when_it_is_known() {
    // The holder is named through `store::flock`'s one registry: here the
    // index build's own label, held on a named thread of this process.
    let (root, journal) = journal_with_one_record("holder-named");
    let path = journal.path.clone();
    let builder = std::thread::Builder::new()
        .name("ag06-index-build".to_owned())
        .spawn(move || {
            store::flock::Flock::try_lock_patiently(
                std::fs::File::open(&path).expect("the journal opens"),
                path.clone(),
                Duration::ZERO,
                INDEX_BUILD_DOING,
            )
            .expect("free")
        })
        .expect("a named thread")
        .join()
        .expect("the builder");
    let why = journal
        .appended_within(Duration::from_millis(30), &member(7), |_| {
            member(7).image().to_vec()
        })
        .expect_err("held");
    for part in [
        "held by this process",
        "thread ag06-index-build",
        "doing the journal index build",
    ] {
        assert!(why.contains(part), "{part:?} missing from: {why}");
    }
    let m = member(7);
    let named_record = format!("a {} record ({})", m.kind.label(), m.outcome.label());
    assert!(
        why.contains(&named_record),
        "{named_record:?} missing from: {why}"
    );
    builder.release().expect("released");
    // And an append that lands leaves no holder behind it.
    journal.append(&closed("after")).expect("lands");
    let after = store::flock::holder_note(&journal.path);
    assert!(
        !after.contains("held by this process"),
        "no stale holder: {after}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Whether the journal's append lock is held now, by anyone: one attempt from
/// a description of its own, let go at once when it was free.
fn held_now(journal: &Journal) -> bool {
    !lock_is_free(journal)
}

#[test]
fn the_holder_is_named_through_every_in_process_hand_off() {
    // AG-06 repair 1, the verifier's blocking item 2. A registry kept beside
    // the guard in `api::audit` dropped its entry AFTER the unlock had handed
    // the lock to the next appender, so it erased the successor's entry and
    // named nobody for the whole next hold: a refusal then blamed another
    // process. Here appenders in this process hand the lock to each other
    // while a sampler asks who holds it whenever it is held. A sample that
    // reads "not this process" is checked twice more, 1 ms apart; one still
    // wrong both times while the lock stays held is the defect. Fails on the
    // build before the repair (the journal's appends never reached
    // `store::flock`'s registry, so every sample read "another process").
    let (root, journal) = journal_with_one_record("hand-off");
    let journal = Arc::new(journal);
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let appenders: Vec<_> = (0..4)
        .map(|w| {
            let journal = Arc::clone(&journal);
            std::thread::spawn(move || {
                (0..120)
                    .filter(|i| journal.append(&closed(&format!("h{w}-{i}"))).is_err())
                    .count()
            })
        })
        .collect();
    let sampler = {
        let (journal, done) = (Arc::clone(&journal), Arc::clone(&done));
        std::thread::spawn(move || {
            let (mut held, mut unnamed) = (0_u64, 0_u64);
            while !done.load(std::sync::atomic::Ordering::Relaxed) {
                if !held_now(&journal) {
                    continue;
                }
                held += 1;
                let wrong =
                    || !store::flock::holder_note(&journal.path).contains("held by this process");
                if !wrong() {
                    continue;
                }
                let still = (0..2).all(|_| {
                    std::thread::sleep(Duration::from_millis(1));
                    held_now(&journal) && wrong()
                });
                if still {
                    unnamed += 1;
                }
            }
            (held, unnamed)
        })
    };
    let refused: usize = appenders
        .into_iter()
        .map(|h| h.join().expect("an appender"))
        .sum();
    done.store(true, std::sync::atomic::Ordering::Relaxed);
    let (held, unnamed) = sampler.join().expect("the sampler");
    assert_eq!(refused, 0, "no append refused");
    assert!(
        held >= 50,
        "the sampler saw the lock held often enough to judge: {held}"
    );
    assert_eq!(
        unnamed, 0,
        "of {held} samples that met the lock held, {unnamed} named no holder in this process \
         while one held it"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_zero_bound_refuses_at_once() {
    let (root, journal) = journal_with_one_record("zero");
    let holder = hold(&journal);
    let started = Instant::now();
    let why = journal
        .appended_within(Duration::ZERO, &closed("z"), |_| {
            closed("z").image().to_vec()
        })
        .expect_err("held, and no patience");
    let took = started.elapsed();
    assert!(why.contains("within its 0 ms bound"), "{why}");
    assert!(why.contains("(1 attempt(s)"), "one attempt: {why}");
    assert!(took < store::flock::MAX_PAUSE + SLACK, "{took:?}");
    holder.release().expect("released");
    let _ = std::fs::remove_dir_all(&root);
}

/// EIGHT APPENDERS AT ONCE LOSE NO RECORD UNNAMED (BASE-REDS-1023 repair 3,
/// TST-01 of the assurance attack of 29 Oct 2026). Each of 800 records is
/// appended ONCE, as every production caller of the plain append does: none
/// asks again (`RunLedger::open`, `RunLedger::close`, the spot route's
/// blocked record, `recorded_fact`, the member-failure receipt, `fno_opened`
/// and the autopilot's tick record; `docs/06-limits.md` L-BRED23-16 lists
/// them with what each does with the refusal). Each answer is [`judged`]: a
/// success is on disk exactly once, and a refusal waited the whole bound and
/// names it, which is what its caller surfaces; a refusal sooner (the
/// `try_lock` build's, which refused at once) is a loss and fails here. The
/// refusals are counted and printed with the waits' p50, p99 and max: under
/// load the bound can be met, and that rate is a stated limit, never hidden
/// by a retry. Round 2 asked again after a refusal, which no caller of this
/// append does.
#[test]
fn eight_concurrent_appenders_lose_no_record() {
    let (root, journal) = journal_with_one_record("eight");
    let appended = eight_appenders(&journal, "w");
    assert_eq!(
        appended.landed.len() + appended.refused.len(),
        800,
        "every record answered once"
    );
    assert_exactly(&journal, 1, &appended);
    eprintln!("{}", wait_line("eight", 1, &appended));
    let _ = std::fs::remove_dir_all(&root);
}

/// What eight appenders behind one barrier got for 100 records each, every
/// answer [`judged`]: the sources that landed, the sources the bound
/// refused, and each call's wait.
struct Appended {
    landed: Vec<String>,
    refused: Vec<String>,
    waits: Vec<Duration>,
}

/// Eight appenders behind one barrier, 100 plain appends each, each record
/// asked once; a refusal that is not the bound's panics as a loss.
fn eight_appenders(journal: &Journal, tag: &str) -> Appended {
    let barrier = Barrier::new(8);
    let answers: Vec<(String, bool, Duration)> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..8)
            .map(|w| {
                let barrier = &barrier;
                s.spawn(move || {
                    barrier.wait();
                    (0..100)
                        .map(|i| {
                            let source = format!("{tag}{w}-{i}");
                            let asked = Instant::now();
                            let answer = journal.append(&closed(&source));
                            let waited = asked.elapsed();
                            let landed = judged(answer, waited)
                                .expect("a refusal is the bound's, never a loss");
                            (source, landed, waited)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|h| h.join().expect("an appender"))
            .collect()
    });
    let (landed, refused): (Vec<_>, Vec<_>) = answers.iter().partition(|(_, l, _)| *l);
    Appended {
        landed: landed.into_iter().map(|(s, _, _)| s.clone()).collect(),
        refused: refused.into_iter().map(|(s, _, _)| s.clone()).collect(),
        waits: answers.iter().map(|(_, _, w)| *w).collect(),
    }
}

/// The sources of the newest `k` of `records`, read in pages as the journal
/// serves them.
fn newest_sources(journal: &Journal, records: u64, k: u64) -> Vec<String> {
    let mut out = Vec::new();
    let mut skip = 0;
    while skip < k {
        let page = journal.page(records, skip, k - skip).expect("a page");
        assert!(!page.is_empty(), "a page of a non-empty rest");
        skip += u64::try_from(page.len()).expect("small");
        out.extend(
            page.into_iter()
                .map(|row| row.decoded.expect("every record decodes").source),
        );
    }
    out
}

/// The journal holds its `held` records and then exactly the ones that
/// landed, each once and whole; so no refused record is on disk.
fn assert_exactly(journal: &Journal, held: usize, appended: &Appended) {
    let landed = u64::try_from(appended.landed.len()).expect("small");
    let records = journal.look().records();
    assert_eq!(
        records,
        u64::try_from(held).expect("small") + landed,
        "the held records and every answered success, nothing else"
    );
    assert_eq!(
        std::fs::metadata(&journal.path).expect("metadata").len(),
        records * RECORD_LEN_U64,
        "no record torn"
    );
    let mut newest = newest_sources(journal, records, landed);
    newest.sort();
    let mut want = appended.landed.clone();
    want.sort();
    assert_eq!(
        newest, want,
        "each answered success once, behind the held records"
    );
}

/// The measurement line of one run of [`eight_appenders`]: n, p50, p99 and
/// max of the waits in microseconds (nearest rank), and the refusals.
fn wait_line(what: &str, held: usize, appended: &Appended) -> String {
    let mut us: Vec<u128> = appended.waits.iter().map(Duration::as_micros).collect();
    us.sort_unstable();
    let at = |p: usize| us[(us.len() * p).div_ceil(100).max(1) - 1];
    format!(
        "APPEND_WAIT {what} held={held} n={} p50_us={} p99_us={} max_us={} refused={}",
        us.len(),
        at(50),
        at(99),
        at(100),
        appended.refused.len()
    )
}

/// TST-01 (3): the wait and the refusals of eight appenders, 100 records
/// each, over a journal already holding 1,000, 10,000 and 100,000 records
/// (1x, 10x, 100x), each answer [`judged`] and the journal checked as in
/// [`eight_concurrent_appenders_lose_no_record`]. Prints one `APPEND_WAIT`
/// line per size: those are the figures of `docs/06-limits.md`
/// L-BRED23-16, with the load average of each run beside them. What holds
/// at any load is asserted: no record lost unnamed at any size. The first
/// append of each size also builds the journal's index from scratch under
/// the lock (L-AG06-02), so the max holds that build.
#[test]
fn eight_appenders_at_one_ten_and_a_hundred_times_the_records_held() {
    for held in [1_000_usize, 10_000, 100_000] {
        let root = scratch(&format!("held-{held}"));
        let journal = Journal::at(&root);
        let images: Vec<u8> = (0..held)
            .flat_map(|i| closed(&format!("held-{i}")).image())
            .collect();
        std::fs::create_dir_all(journal.path.parent().expect("the audit directory"))
            .expect("the audit directory");
        std::fs::write(&journal.path, &images).expect("the held records");
        let appended = eight_appenders(&journal, "m");
        assert_exactly(&journal, held, &appended);
        eprintln!("{}", wait_line("measure", held, &appended));
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// 64 appenders at once, behind one barrier, with `bound`; answers each one's
/// answer after checking that every answered success is on disk, nothing
/// else is, and every refusal names its bound.
fn sixty_four_at_once(name: &str, bound: Duration) -> Vec<Result<u64, String>> {
    let (root, journal) = journal_with_one_record(name);
    let journal = Arc::new(journal);
    let barrier = Arc::new(Barrier::new(64));
    let answers: Vec<Result<u64, String>> = (0..64)
        .map(|w| {
            let (journal, barrier) = (Arc::clone(&journal), Arc::clone(&barrier));
            std::thread::spawn(move || {
                let record = closed(&format!("w{w}"));
                barrier.wait();
                journal.appended_within(bound, &record, |_| record.image().to_vec())
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|h| h.join().expect("an appender"))
        .collect();
    let landed = answers.iter().filter(|a| a.is_ok()).count();
    for why in answers.iter().filter_map(|a| a.as_ref().err()) {
        assert!(
            why.contains(&format!(
                "{} ms bound (APPEND_LOCK_WAIT)",
                bound.as_millis()
            )),
            "named: {why}"
        );
    }
    assert_eq!(
        journal.look().records(),
        1 + u64::try_from(landed).expect("small"),
        "every answered success is on disk and nothing else is"
    );
    let _ = std::fs::remove_dir_all(&root);
    answers
}

#[test]
fn sixty_four_appenders_at_once_each_land_or_are_refused_by_name() {
    // With the production bound, which is also `store::flock`'s ceiling: the
    // queue in this process means the 64th waits for 63 holds, and whether
    // that fits in one second is the disk's to say (docs/06-limits.md
    // L-AG06-01). What is proved here is that none is lost silently: each
    // lands at an ordinal of its own or is refused naming its bound, and the
    // journal holds exactly the ones that landed. That all 64 land when the
    // holds are short is `store::flock`'s
    // `sixty_four_patient_lockers_all_take_the_lock_in_turn`.
    let answers = sixty_four_at_once("sixty-four-bound", APPEND_LOCK_WAIT);
    let mut ordinals: Vec<u64> = answers
        .iter()
        .filter_map(|a| a.as_ref().ok())
        .copied()
        .collect();
    assert!(!ordinals.is_empty(), "the first in the queue lands");
    ordinals.sort_unstable();
    let landed = ordinals.len();
    ordinals.dedup();
    assert_eq!(ordinals.len(), landed, "no two at one ordinal");
    assert_eq!(
        ordinals,
        (1..=u64::try_from(landed).expect("small")).collect::<Vec<u64>>(),
        "the ones that landed follow the seed, one after another"
    );
}

#[test]
fn a_reader_never_waits_on_the_append_bound() {
    let (root, journal) = journal_with_one_record("reader");
    let holder = hold(&journal);
    let started = Instant::now();
    let log = journal.look();
    let page = journal.page(log.records(), 0, 1).expect("a page");
    let took = started.elapsed();
    assert_eq!(page.len(), 1);
    assert!(
        took < APPEND_LOCK_WAIT / 4,
        "a reader takes no append lock and never waits on it: {took:?}"
    );
    holder.release().expect("released");
    let _ = std::fs::remove_dir_all(&root);
}

/// The code-4 skeptic's barrier experiment, at the journal: one walk opens
/// and closes (an archive `fno_answer`: its opening record, then its NOT
/// STARTED close) while another opens (a Zerodha `fno_opened`), both started
/// on one barrier, 300 times on a fresh journal each. On the `try_lock` build
/// it lost a record on every contended run; here it loses none. `fno_answer`
/// and `fno_opened` are private to `server.rs` (batch-1 AF-04's file), so the
/// three journal calls they make (`Journal::append`) are made directly.
///
/// EACH RECORD IS ASKED ONCE (BASE-REDS-1023 repair 3, TST-01), as those two
/// callers do: neither asks again, and each puts a refusal into its answer
/// ("Opened in the journal: NO — ..." and the "Recorded" fact). Under load
/// (24 Oct 2026, load about 25, other builds fsyncing the drive) 7 of 900
/// appends were refused after the other walk's fsynced append held the lock
/// past `APPEND_LOCK_WAIT`: each such refusal must have waited the whole
/// bound and name it ([`judged`]), and is then counted as refused, not lost;
/// a refusal sooner (the `try_lock` build refused at once) is a loss and
/// fails here. Every answered success is on disk and no refused record is.
#[test]
fn the_code_4_barrier_experiment_loses_no_record_in_300_runs() {
    let mut refused = 0_usize;
    for run in 0..300 {
        let root = scratch(&format!("barrier-{run}"));
        let journal = Journal::at(&root);
        let barrier = Barrier::new(2);
        let (a, b) = std::thread::scope(|s| {
            let a = s.spawn(|| {
                barrier.wait();
                [
                    once(&journal, &opened("gdfl")),
                    once(&journal, &closed("gdfl")),
                ]
            });
            let b = s.spawn(|| {
                barrier.wait();
                [once(&journal, &opened("zerodha"))]
            });
            (a.join().expect("walk A"), b.join().expect("walk B"))
        });
        let landed = a.iter().chain(&b).filter(|l| **l).count();
        assert_eq!(
            usize::try_from(journal.look().records()).expect("small"),
            landed,
            "run {run}: every answered success is on disk and no refused record is"
        );
        refused += 3 - landed;
        let _ = std::fs::remove_dir_all(&root);
    }
    eprintln!("CODE4_BARRIER runs=300 appends=900 refused_by_the_bound={refused}");
}

/// Whether a refusal that came `waited` after its ask is the bound doing its
/// job: it waited at least `APPEND_LOCK_WAIT` and its text names the bound.
/// A refusal sooner than the bound is the `try_lock` build's loss.
fn a_refusal_is_the_bound(waited: Duration, why: &str) -> Result<(), String> {
    if waited < APPEND_LOCK_WAIT {
        return Err(format!(
            "refused after {waited:?}, inside the {APPEND_LOCK_WAIT:?} bound: {why}"
        ));
    }
    if !(why.contains("(APPEND_LOCK_WAIT)") && why.contains("refused rather than interleaved")) {
        return Err(format!("a refusal that does not name its bound: {why}"));
    }
    Ok(())
}

/// One plain append's answer, `waited` after its ask, as its caller gets it:
/// `Ok(true)` it landed, `Ok(false)` the bound refused it by name (what the
/// caller surfaces), `Err` a loss ([`a_refusal_is_the_bound`]).
fn judged(answer: Result<(), String>, waited: Duration) -> Result<bool, String> {
    match answer {
        Ok(()) => Ok(true),
        Err(why) => a_refusal_is_the_bound(waited, &why).map(|()| false),
    }
}

/// Appends `record` once and answers whether it landed; a loss panics.
fn once(journal: &Journal, record: &Record) -> bool {
    let asked = Instant::now();
    let answer = journal.append(record);
    judged(answer, asked.elapsed()).expect("a refusal is the bound's, never a loss")
}

/// What `appended_within` writes for `record`, whatever its ordinal.
fn image_of(record: &Record) -> impl FnOnce(u64) -> Vec<u8> + '_ {
    move |_ordinal| record.image().to_vec()
}

/// A refusal inside the bound is a loss and one that waited the bound out is
/// the bound's, named, with nothing of its record on disk; no caller of the
/// plain append asks again, so neither does this test (BASE-REDS-1023
/// repair 3, TST-01; round 2's `..._is_asked_again` asked again).
#[test]
fn a_refusal_inside_the_bound_is_a_loss_and_one_that_waited_it_out_is_named() {
    let named = "/j: cannot take the journal append lock for a x record (y) within its 1000 ms bound (APPEND_LOCK_WAIT): held; so this record was refused rather than interleaved";
    // The `try_lock` build's shape: refused at once, or anywhere inside the bound.
    assert!(a_refusal_is_the_bound(Duration::ZERO, named).is_err());
    assert!(
        a_refusal_is_the_bound(
            APPEND_LOCK_WAIT.saturating_sub(Duration::from_nanos(1)),
            named
        )
        .is_err()
    );
    // A whole bound waited, but the text names neither, or the bound alone.
    assert!(a_refusal_is_the_bound(APPEND_LOCK_WAIT, "/j: held").is_err());
    assert!(
        a_refusal_is_the_bound(
            APPEND_LOCK_WAIT,
            "/j: within its 1000 ms bound (APPEND_LOCK_WAIT): held"
        )
        .is_err()
    );
    assert!(a_refusal_is_the_bound(APPEND_LOCK_WAIT, named).is_ok());
    assert_eq!(judged(Ok(()), Duration::ZERO), Ok(true));
    assert_eq!(judged(Err(named.to_owned()), APPEND_LOCK_WAIT), Ok(false));
    assert!(judged(Err(named.to_owned()), Duration::ZERO).is_err());
    assert!(judged(Err("/j: held".to_owned()), APPEND_LOCK_WAIT).is_err());
    // On a real journal: a zero-bound refusal from a held lock is a loss, and
    // an append that meets a hold longer than the bound is refused by name
    // after the whole bound, nothing of it written.
    let (root, journal) = journal_with_one_record("named");
    let holder = hold(&journal);
    let quick = closed("quick");
    let asked = Instant::now();
    let refused = journal
        .appended_within(Duration::ZERO, &quick, image_of(&quick))
        .expect_err("held: a zero bound refuses");
    assert!(
        a_refusal_is_the_bound(asked.elapsed(), &refused).is_err(),
        "{refused}"
    );
    let (answer, waited) = std::thread::scope(|s| {
        let waiter = s.spawn(|| {
            let asked = Instant::now();
            let answer = journal.append(&closed("patient"));
            (answer, asked.elapsed())
        });
        std::thread::sleep(APPEND_LOCK_WAIT * 3 / 2);
        let answered = waiter.join().expect("the waiter");
        holder.release().expect("released");
        answered
    });
    assert_eq!(
        judged(answer, waited),
        Ok(false),
        "refused by the bound, named"
    );
    assert_eq!(journal.look().records(), 1, "the seed only");
    // The lock is free again: the next record lands at once.
    let after = closed("after");
    assert_eq!(
        journal.appended_within(APPEND_LOCK_WAIT, &after, image_of(&after)),
        Ok(1)
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// code-5 itself: a run whose opening met another writer's append, and whose
/// process then died before its close, reads INTERRUPTED at the next start,
/// not "clean".
#[test]
fn a_run_whose_opening_met_a_held_lock_and_then_died_reads_interrupted() {
    let (root, journal) = journal_with_one_record("code-5");
    let holder = hold(&journal);
    let released = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        holder.release().expect("released");
    });
    let opened_ok = journal.open_run(&opened("cut-short")).is_ok();
    released.join().expect("the holder thread");
    assert!(opened_ok, "the opening waited and landed");
    // The process dies here: no member failure, no close.
    let found = crate::pullrun::restart_scan(&journal);
    assert!(
        found.interrupted().is_some(),
        "the cut-short run is found, not answered clean: {found:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// One append call of [`every_append_kind_waits_for_a_held_lock`].
type Call<'a> = &'a dyn Fn(&mut Thread) -> Result<(), String>;

#[test]
fn every_append_kind_waits_for_a_held_lock() {
    let (root, journal) = journal_with_one_record("every-kind");
    let mut thread = journal.open_run(&opened("run")).expect("opens");
    let calls: [Call<'_>; 5] = [
        &|_| journal.append(&closed("append")),
        &|_| journal.append_member(&member(1), "a gap"),
        &|t| journal.append_failure(t, &member(2), "a gap"),
        &|t| journal.close_run(t, &closed("close")),
        &|_| journal.close_unopened(&opened("late"), &closed("late close")),
    ];
    for (i, call) in calls.iter().enumerate() {
        let holder = hold(&journal);
        let released = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            holder.release().expect("released");
        });
        let before = journal.look().records();
        call(&mut thread).unwrap_or_else(|why| panic!("call {i} waited and landed: {why}"));
        released.join().expect("the holder thread");
        assert!(journal.look().records() > before, "call {i} wrote");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_whole_note_lock_waits_too() {
    let (root, journal) = journal_with_one_record("notes");
    let notes = journal.notes_path();
    std::fs::write(&notes, b"").expect("the notes file");
    let holder = store::flock::Flock::try_lock(
        std::fs::OpenOptions::new()
            .append(true)
            .open(&notes)
            .expect("opens"),
        notes.clone(),
    )
    .expect("free");
    let released = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        holder.release().expect("released");
    });
    let whole = "x".repeat(NOTE_CAPACITY * 3);
    let record = Record::member_failure(Scope::Spot, at(3), "M", window(), &whole);
    journal.append_member(&record, &whole).expect("lands");
    released.join().expect("the holder thread");
    let n = journal.look().records();
    let entry = journal.page(n, 0, 1).expect("a page").remove(0);
    assert_eq!(
        entry.whole_note,
        Some(Ok(whole)),
        "the whole reason waited for its lock and was kept, not cut"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn disk_full_on_the_append_after_the_lock_is_refused_by_name_and_releases_the_lock() {
    let (root, journal) = journal_with_one_record("disk-full");
    let before = std::fs::read(&journal.path).expect("the journal");
    refuse_next_write(std::io::ErrorKind::StorageFull);
    let why = journal
        .append(&closed("full"))
        .expect_err("the disk is full");
    assert!(why.contains("cannot append the record"), "{why}");
    assert!(why.contains("truncated back to 256 bytes"), "{why}");
    assert_eq!(std::fs::read(&journal.path).expect("the journal"), before);
    assert!(
        lock_is_free(&journal),
        "the lock was released with the refusal"
    );
    journal
        .append(&closed("next"))
        .expect("the next append lands");
    assert_eq!(journal.look().records(), 2);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_torn_tail_after_the_wait_is_refused_not_repaired() {
    let (root, journal) = journal_with_one_record("torn");
    let holder = hold(&journal);
    let mut torn = std::fs::OpenOptions::new()
        .append(true)
        .open(&journal.path)
        .expect("opens");
    std::io::Write::write_all(&mut torn, &[0xEE; 100]).expect("a killed appender's tail");
    let before = std::fs::read(&journal.path).expect("the journal");
    let released = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        holder.release().expect("released");
    });
    let why = journal
        .append(&closed("after-tear"))
        .expect_err("waits, then refuses the torn tail by name");
    released.join().expect("the holder thread");
    assert!(why.contains("100 trailing byte(s)"), "{why}");
    assert_eq!(std::fs::read(&journal.path).expect("the journal"), before);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_journal_replaced_while_an_append_waits_is_refused_by_name() {
    let (root, journal) = journal_with_one_record("replaced");
    let holder = hold(&journal);
    let moved_to = root.join("audit").join("pull.journal.moved");
    let outcome = std::thread::scope(|s| {
        let waiter = s.spawn(|| journal.append(&closed("waiter")));
        // The waiter opens the journal at once and then waits on the lock.
        std::thread::sleep(Duration::from_millis(200));
        std::fs::rename(&journal.path, &moved_to).expect("moved aside");
        std::fs::write(&journal.path, b"").expect("a new journal at the path");
        std::thread::sleep(Duration::from_millis(20));
        holder.release().expect("released");
        waiter.join().expect("the waiter")
    });
    let why = outcome.expect_err("the locked file is no longer the journal");
    assert!(
        why.contains("replaced or moved while this append waited"),
        "{why}"
    );
    assert_eq!(
        std::fs::metadata(&moved_to).expect("moved").len(),
        RECORD_LEN_U64,
        "nothing appended to the file moved aside"
    );
    assert_eq!(
        std::fs::metadata(&journal.path).expect("new").len(),
        0,
        "nor to the new one behind a refusal"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Set in the child [`a_holder_killed_while_holding_frees_the_lock_to_the_waiter`]
/// spawns: the journal it holds the lock of.
const CHILD_HOLDS: &str = "BRUTEX_AG06_CHILD_HOLDS";

#[test]
fn a_holder_killed_while_holding_frees_the_lock_to_the_waiter() {
    if let Ok(path) = std::env::var(CHILD_HOLDS) {
        // THE CHILD: take the lock, say so, and hold it until killed.
        let path = PathBuf::from(path);
        let held = Journal { path: path.clone() };
        let _guard = hold(&held);
        std::fs::write(path.with_extension("held"), b"1").expect("the marker");
        std::thread::sleep(Duration::from_secs(30));
        panic!("the parent should have killed this child");
    }
    let (root, journal) = journal_with_one_record("killed");
    let marker = journal.path.with_extension("held");
    let mut child = std::process::Command::new(std::env::current_exe().expect("this binary"))
        .args([
            "--exact",
            "audit::lock_wait_tests::a_holder_killed_while_holding_frees_the_lock_to_the_waiter",
        ])
        .env(CHILD_HOLDS, &journal.path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("the holder child");
    let started = Instant::now();
    while !marker.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "the child never held"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!lock_is_free(&journal), "the child holds the lock");
    let answer = std::thread::scope(|s| {
        let waiter = s.spawn(|| journal.append(&closed("after-kill")));
        std::thread::sleep(Duration::from_millis(100));
        child.kill().expect("killed");
        let _status = child.wait().expect("reaped");
        waiter.join().expect("the waiter")
    });
    answer.expect("the kernel freed the dead holder's lock and the waiter took it");
    assert_eq!(journal.look().records(), 2);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_refusal_text_pins_the_bound_constant() {
    // The constant the docs name: one second, from the measured holder p99
    // (docs/06-limits.md, AG-06), and no more than `store::flock`'s one
    // ceiling, which would cut it (a compile-time assertion in audit.rs too).
    assert_eq!(APPEND_LOCK_WAIT, Duration::from_secs(1));
    assert!(APPEND_LOCK_WAIT <= store::flock::MAX_PATIENCE);
    let source = include_str!("audit.rs");
    assert_eq!(
        source.matches("Flock::try_lock_patiently(").count(),
        3,
        "the journal's and the notes file's append locks wait, and the index build registers"
    );
    for doing in ["APPEND_DOING,", "NOTE_DOING,", "INDEX_BUILD_DOING,"] {
        assert_eq!(
            source.matches(doing).count(),
            1,
            "{doing} names one lock site"
        );
    }
    assert!(
        !source.contains("\nstatic ") && !source.contains("LazyLock") && !source.contains("Mutex<"),
        "no second registry of holders in audit.rs: the one registry is store::flock's"
    );
    assert!(
        !source.contains("thread::sleep"),
        "no second wait loop in audit.rs: the one loop is store::flock's"
    );
}
