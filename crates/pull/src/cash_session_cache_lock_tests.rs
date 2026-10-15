//! AF-03 (p99-2): READERS SHARE A DAY'S LOCK AND AN INSTALLER HOLDS IT ALONE.
//!
//! Every reader of the dated masters took the installer's exclusive,
//! non-blocking lock and held it through the gzip decode, so four identical
//! `/gaps.json` requests in parallel gave one right answer and three
//! `expected 0` answers naming "operation would block", and a GDFL import or
//! `cli` reading the same day refused beside it. Each test here fails on that
//! code and passes on the shared hold with a named, bounded wait.
//!
//! Fixtures are invented; no vendor row is copied.

#![allow(
    clippy::expect_used,
    reason = "tests fail immediately when fixture setup fails"
)]

use super::*;
use flate2::{Compression, write::GzEncoder};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

const CSV: &str = "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n1363,HINDALCO,EQ,INE038A01020,1\n16921,20MICRONS,EQ,INE144J01027,0\n";

/// One reader's whole answer: each day with its two identities' flags.
type Answer = Vec<(u32, bool, bool)>;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "brutex-af03-lock-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("unique test directory");
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn day(date: u8) -> Day {
    Day::new(2026, 8, date).expect("test date")
}

fn gzip(text: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(text).expect("compress fixture");
    encoder.finish().expect("finish gzip")
}

fn seed(root: &Path, date: u8) -> Vec<u8> {
    let bytes = gzip(CSV.as_bytes());
    install(root, day(date), &bytes).expect("valid fixture installs");
    bytes
}

/// What `/gaps.json` and the pull read: `prepare_local_observed`, on its own
/// runtime, as each request thread runs it.
fn observe(root: &Path, days: &[Day]) -> Result<Answer, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|why| why.to_string())?;
    let mut cache = HashMap::new();
    runtime.block_on(prepare_local_observed(root, days, &mut cache))?;
    let mut answer = Vec::with_capacity(cache.len());
    for (key, daily) in &cache {
        answer.push((
            *key,
            daily.eligibility("HINDALCO", "INE038A01020")?,
            daily.eligibility("20MICRONS", "INE144J01027")?,
        ));
    }
    answer.sort_unstable();
    Ok(answer)
}

/// What `/bars.json`, recovery, a GDFL import and `cli` read.
fn lifecycle(root: &Path, on: Day) -> Result<Answer, String> {
    let master = read_local_lifecycle(root, on)?;
    Ok(vec![(
        on.days_from_epoch(),
        master.eligibility("HINDALCO", "INE038A01020")?,
        master.eligibility("20MICRONS", "INE144J01027")?,
    )])
}

/// Runs `readers` threads behind one barrier and returns their answers.
fn in_parallel<T: Send + 'static>(
    readers: usize,
    read: impl Fn(usize) -> T + Send + Sync + 'static,
) -> Vec<T> {
    let barrier = Arc::new(Barrier::new(readers));
    let read = Arc::new(read);
    let handles: Vec<_> = (0..readers)
        .map(|nth| {
            let barrier = Arc::clone(&barrier);
            let read = Arc::clone(&read);
            thread::spawn(move || {
                barrier.wait();
                read(nth)
            })
        })
        .collect();
    handles
        .into_iter()
        .map(|handle| handle.join().expect("reader thread"))
        .collect()
}

/// **EIGHT READERS OF ONE DAY ALL GET THE SAME ANSWER.** The finding's
/// reproduction, in-process: eight identical reads behind a barrier, twenty
/// rounds, while another process's lifecycle reader (`cli` `stored.rs`, a
/// GDFL import) holds one of the days shared the whole time. The exclusive
/// read lock refused every one of them on that day.
#[test]
fn eight_readers_of_one_day_all_get_the_same_answer() {
    let temp = Temp::new();
    for date in [3, 4, 5] {
        seed(&temp.0, date);
    }
    let days = [day(3), day(4), day(5)];
    let solo = observe(&temp.0, &days).expect("one reader alone");
    assert_eq!(solo.len(), 3);
    assert!(solo.iter().all(|&(_, hindalco, micro)| hindalco && !micro));
    let other = File::open(lock_path(&temp.0, day(4))).expect("the day's lock");
    other
        .try_lock_shared()
        .expect("another process reads the day");
    for round in 0..20 {
        let root = temp.0.clone();
        let answers = in_parallel(8, move |_| observe(&root, &days));
        for answer in answers {
            assert_eq!(answer.as_ref(), Ok(&solo), "round {round}");
        }
    }
    other.unlock().expect("the other reader leaves");
}

/// **A READER BESIDE AN INSTALLER WAITS, THEN ANSWERS — OR NAMES THE WAIT.**
/// The installer holds the day alone with its payload written and its
/// receipt not yet: a reader that did not wait would refuse ("would block")
/// or read a half-published entry ("incomplete"). Both readers wait, then
/// answer the whole master. Past the bound the refusal names the installer
/// and the bound, never "would block".
#[test]
fn a_reader_beside_an_installer_waits_then_answers_or_names_the_wait() {
    let temp = Temp::new();
    let bytes = gzip(CSV.as_bytes());
    let (path, file) = open_lock(&temp.0, day(6)).expect("the day's lock");
    let installer = Flock::try_lock(&file, path.as_path()).expect("the installer holds the day");
    let (payload, metadata) = paths(&temp.0, day(6));
    fs::write(&payload, &bytes).expect("payload published first");
    let barrier = Arc::new(Barrier::new(3));
    let spawn = |observed: bool| {
        let barrier = Arc::clone(&barrier);
        let root = temp.0.clone();
        thread::spawn(move || {
            barrier.wait();
            let started = Instant::now();
            let answer = if observed {
                observe(&root, &[day(6)])
            } else {
                lifecycle(&root, day(6))
            };
            (answer, started.elapsed())
        })
    };
    let lifecycle_reader = spawn(false);
    let observed_reader = spawn(true);
    barrier.wait();
    thread::sleep(Duration::from_millis(150));
    fs::write(&metadata, receipt(day(6), &bytes)).expect("receipt last");
    installer.release().expect("the installer releases");
    let whole = vec![(day(6).days_from_epoch(), true, false)];
    for reader in [lifecycle_reader, observed_reader] {
        let (answer, waited) = reader.join().expect("reader thread");
        assert_eq!(answer.as_ref(), Ok(&whole), "waited {waited:?}");
        assert!(waited < LOCK_WAIT, "within the bound: {waited:?}");
    }

    let held = Flock::try_lock(&file, path.as_path()).expect("the installer holds it again");
    let started = Instant::now();
    let why = read_local_lifecycle_within(&temp.0, day(6), Duration::from_millis(40))
        .expect_err("held past the bound");
    assert!(started.elapsed() >= Duration::from_millis(40), "it waited");
    assert!(
        why.contains("unavailable")
            && why.contains("an installer held it exclusively")
            && why.contains("40 ms bound")
            && why.contains("not read as absent"),
        "{why}"
    );
    assert!(!why.contains("would block"), "{why}");
    held.release().expect("released");
}

/// **AN INSTALLER BESIDE READERS INSTALLS ONCE.** Four installers of the same
/// bytes race eight readers for four fresh days while another process's
/// reader holds each day shared for the first 60 ms. Every installer waits
/// and succeeds (the entry is written once: `create_new` refuses a second
/// writer, so four `Ok`s prove one write and three re-checks); every reader
/// answers the whole master or names it absent, never "incomplete", never
/// "would block".
#[test]
fn an_installer_beside_readers_installs_once() {
    let temp = Temp::new();
    let bytes = Arc::new(gzip(CSV.as_bytes()));
    for date in [10, 11, 12, 13] {
        let (path, file) = open_lock(&temp.0, day(date)).expect("the day's lock");
        let reader = Flock::try_lock_shared(&file, path.as_path()).expect("a reader holds it");
        let root = temp.0.clone();
        let shared = Arc::clone(&bytes);
        let racing = thread::spawn(move || {
            in_parallel(12, move |nth| match nth % 3 {
                0 => install(&root, day(date), &shared).map(|()| Vec::new()),
                1 => lifecycle(&root, day(date)),
                _ => observe(&root, &[day(date)]),
            })
        });
        thread::sleep(Duration::from_millis(60));
        reader.release().expect("the reader leaves");
        let whole = vec![(day(date).days_from_epoch(), true, false)];
        for (nth, answer) in racing.join().expect("race").into_iter().enumerate() {
            match (nth % 3, answer) {
                (0, installed) => assert_eq!(installed, Ok(Vec::new()), "installer {nth}"),
                (_, Ok(answer)) => assert_eq!(answer, whole, "reader {nth}"),
                (_, Err(why)) => assert!(
                    why.contains("is absent") || why.contains("missing locally"),
                    "reader {nth} may only find the day not yet installed: {why}"
                ),
            }
        }
        let (payload, metadata) = paths(&temp.0, day(date));
        assert_eq!(fs::read(&payload).expect("payload"), *bytes);
        assert_eq!(
            fs::read(&metadata).expect("receipt"),
            receipt(day(date), &bytes).into_bytes()
        );
        assert_eq!(lifecycle(&temp.0, day(date)), Ok(whole));
    }
    let entries = fs::read_dir(&temp.0).expect("cache").count();
    assert_eq!(entries, 12, "a lock, a payload and a receipt per day, once");
}

/// **A SHARED READER NEVER BLOCKS THE LIFECYCLE READER.** The `/gaps.json`
/// reader's hold, taken exactly as `prepare_*` takes it, leaves the lifecycle
/// reader's single attempt (a zero bound) answering; and a loop of observed
/// reads beside 200 zero-bound lifecycle reads never refuses one.
#[test]
fn a_shared_reader_never_blocks_the_lifecycle_reader() {
    let temp = Temp::new();
    seed(&temp.0, 3);
    let whole = vec![(day(3).days_from_epoch(), true, false)];
    let (path, file) = open_lock(&temp.0, day(3)).expect("the day's lock");
    let reader = hold(&file, &path, Hold::Shared, Duration::ZERO).expect("a reader holds it");
    let master = read_local_lifecycle_within(&temp.0, day(3), Duration::ZERO)
        .expect("one attempt answers beside a reader");
    assert!(
        master
            .eligibility("HINDALCO", "INE038A01020")
            .expect("held")
    );
    assert_eq!(observe(&temp.0, &[day(3)]), Ok(whole.clone()));
    reader.release().expect("released");

    let root = temp.0.clone();
    let observing = thread::spawn(move || {
        (0..200)
            .map(|_| observe(&root, &[day(3)]))
            .collect::<Vec<_>>()
    });
    for nth in 0..200 {
        let read = read_local_lifecycle_within(&temp.0, day(3), Duration::ZERO)
            .and_then(|master| master.eligibility("HINDALCO", "INE038A01020"));
        assert_eq!(read, Ok(true), "lifecycle read {nth}");
    }
    for answer in observing.join().expect("observer") {
        assert_eq!(answer.as_ref(), Ok(&whole));
    }
}

/// **SIXTY-FOUR PARALLEL READERS**, half of each kind, one day: all answer,
/// all alike.
#[test]
fn sixty_four_parallel_readers_all_answer_alike() {
    let temp = Temp::new();
    seed(&temp.0, 7);
    let whole = vec![(day(7).days_from_epoch(), true, false)];
    let root = temp.0.clone();
    let answers = in_parallel(64, move |nth| {
        if nth % 2 == 0 {
            observe(&root, &[day(7)])
        } else {
            lifecycle(&root, day(7))
        }
    });
    assert_eq!(answers.len(), 64);
    for answer in answers {
        assert_eq!(answer.as_ref(), Ok(&whole));
    }
}

/// **THE EXTREMES REFUSE BY NAME.** A missing master, a zero-length payload
/// and a corrupt gzip under matching receipts, and a lock path that is a
/// symbolic link or a directory: each refuses with its own reason on both
/// read paths, nothing is created or replaced, and no answer says
/// "would block".
#[cfg(unix)]
#[test]
fn missing_empty_corrupt_and_nonregular_evidence_refuses_by_name() {
    let temp = Temp::new();
    let missing = observe(&temp.0, &[day(17)]).expect_err("missing master");
    assert!(missing.contains("missing locally"), "{missing}");
    assert!(
        lifecycle(&temp.0, day(17))
            .expect_err("missing master")
            .contains("absent")
    );
    for (date, payload_bytes) in [(18, Vec::new()), (19, b"\x1f\x8b not gzip".to_vec())] {
        let (_, file) = open_lock(&temp.0, day(date)).expect("the day's lock");
        drop(file);
        let (payload, metadata) = paths(&temp.0, day(date));
        fs::write(&payload, &payload_bytes).expect("owned fixture");
        fs::write(&metadata, receipt(day(date), &payload_bytes)).expect("matching receipt");
        for why in [
            observe(&temp.0, &[day(date)]).expect_err("refused"),
            lifecycle(&temp.0, day(date)).expect_err("refused"),
        ] {
            assert!(why.contains("invalid NSE cash"), "{date}: {why}");
            assert!(!why.contains("would block"), "{why}");
        }
        assert_eq!(fs::read(&payload).expect("retained"), payload_bytes);
    }
    seed(&temp.0, 20);
    let lock = lock_path(&temp.0, day(20));
    let elsewhere = temp.0.join("elsewhere.lock");
    fs::rename(&lock, &elsewhere).expect("relocate owned lock");
    std::os::unix::fs::symlink(&elsewhere, &lock).expect("symlink fixture");
    for why in [
        observe(&temp.0, &[day(20)]).expect_err("symlinked lock"),
        lifecycle(&temp.0, day(20)).expect_err("symlinked lock"),
        install(&temp.0, day(20), &gzip(CSV.as_bytes())).expect_err("symlinked lock"),
    ] {
        assert!(why.contains("not a regular file"), "{why}");
    }
    fs::remove_file(&lock).expect("owned symlink");
    fs::create_dir(&lock).expect("directory fixture");
    for why in [
        observe(&temp.0, &[day(20)]).expect_err("directory lock"),
        lifecycle(&temp.0, day(20)).expect_err("directory lock"),
    ] {
        assert!(why.contains("not a regular file"), "{why}");
    }
}

/// **A HOLDER THAT IS KILLED RELEASES THE DAY.** The kernel drops a `flock`
/// with the last descriptor of its open file description. In-process: a lock
/// taken on a raw descriptor and closed without an unlock. Cross-process:
/// this test re-runs itself as a child that takes the day alone and sleeps;
/// a reader names the wait while the child lives and answers once it is
/// killed.
#[cfg(unix)]
#[test]
fn a_killed_holder_releases_the_day() {
    if let Some(held) = std::env::var_os("BRUTEX_AF03_HOLD") {
        let held = PathBuf::from(held);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&held)
            .expect("the parent's lock file");
        file.try_lock().expect("the child holds the day alone");
        fs::write(held.with_extension("ready"), b"held").expect("ready marker");
        thread::sleep(Duration::from_mins(2));
        // Never reached when the parent kills it; a nonzero exit if it is not.
        std::process::exit(3);
    }
    let temp = Temp::new();
    seed(&temp.0, 21);
    let lock = lock_path(&temp.0, day(21));
    let raw = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock)
        .expect("lock");
    raw.try_lock().expect("a raw exclusive hold");
    assert!(read_local_lifecycle_within(&temp.0, day(21), Duration::ZERO).is_err());
    drop(raw);
    assert!(
        read_local_lifecycle_within(&temp.0, day(21), Duration::ZERO).is_ok(),
        "closing the descriptor released it"
    );

    let mut child = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            "cash_session_cache::lock_tests::a_killed_holder_releases_the_day",
            "--test-threads",
            "1",
        ])
        .env("BRUTEX_AF03_HOLD", &lock)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("child holder");
    let ready = lock.with_extension("ready");
    let started = Instant::now();
    while !ready.exists() {
        assert!(
            started.elapsed() < Duration::from_mins(1),
            "the child took the day"
        );
        thread::sleep(Duration::from_millis(10));
    }
    let why = read_local_lifecycle_within(&temp.0, day(21), Duration::from_millis(30))
        .expect_err("the child holds it");
    assert!(why.contains("an installer held it exclusively"), "{why}");
    child.kill().expect("kill the holder");
    child.wait().expect("reap the holder");
    let master = read_local_lifecycle_within(&temp.0, day(21), Duration::from_secs(2))
        .expect("the kernel released the killed holder's lock");
    assert!(
        master
            .eligibility("HINDALCO", "INE038A01020")
            .expect("held")
    );
}
