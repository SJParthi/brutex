//! The patient exclusive lock and its one wait loop (AG-06, code-5, FM-162;
//! D-6244). Real locks for the wait and the expiry; an injected clock and
//! pause for the accounting a real clock cannot be made to show: a zero
//! bound, a clock stepping backwards, a garbage bound, the pause schedule.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::{
    Flock, Impatient, MAX_PATIENCE, MAX_PAUSE, holder_note, key_of, patiently, registry, whole,
};
use std::fs::{self, File, TryLockError};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A lock file nobody else names.
fn lock_file(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-store-patient-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).expect("a scratch directory");
    let lock = dir.join("patient.lock");
    File::create(&lock).expect("the lock file");
    lock
}

fn open(path: &PathBuf) -> File {
    File::options()
        .read(true)
        .write(true)
        .open(path)
        .expect("opens")
}

/// How many tickets this process's queue for `path` has given out; zero
/// when it has none.
fn tickets(path: &std::path::Path) -> u64 {
    registry()
        .as_ref()
        .and_then(|held| held.get(&key_of(path)))
        .and_then(|entry| entry.queue.as_ref())
        .map_or(0, |turns| whole(&turns.state).next)
}

/// Whether the one registry still records anything for `path`: a holder, a
/// reader or a queue.
fn recorded(path: &std::path::Path) -> bool {
    registry()
        .as_ref()
        .is_some_and(|held| held.contains_key(&key_of(path)))
}

/// Scheduling slack on a shared machine; the exact accounting is pinned
/// below with an injected clock.
const SLACK: Duration = Duration::from_millis(750);

#[test]
fn an_exclusive_patient_lock_waits_then_takes() {
    let path = lock_file("waits");
    let holder = Flock::try_lock(open(&path), path.clone()).expect("free");
    let released = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        holder.release().expect("released");
    });
    let started = Instant::now();
    let taken = Flock::try_lock_patiently(
        open(&path),
        path.clone(),
        Duration::from_secs(1),
        "a test append",
    )
    .expect("the holder released within the bound");
    let took = started.elapsed();
    released.join().expect("the holder");
    assert!(took >= Duration::from_millis(40), "it waited: {took:?}");
    assert!(
        took < Duration::from_secs(1),
        "and took it inside the bound"
    );
    assert!(
        matches!(open(&path).try_lock(), Err(TryLockError::WouldBlock)),
        "it holds the lock exclusively"
    );
    taken.release().expect("released");
    assert!(open(&path).try_lock().is_ok(), "and releases it");
}

#[test]
fn an_exclusive_patient_lock_expires_and_names_its_wait() {
    let path = lock_file("expires");
    let _holder = Flock::try_lock(open(&path), path.clone()).expect("free");
    let bound = Duration::from_millis(120);
    let started = Instant::now();
    let refused = Flock::try_lock_patiently(open(&path), path.clone(), bound, "a test append")
        .expect_err("the holder never releases");
    let took = started.elapsed();
    let Impatient::Held {
        bound: named,
        waited,
        attempts,
        ahead,
        holder,
    } = &refused
    else {
        panic!("contention, not a host refusal: {refused}");
    };
    assert_eq!(*named, bound);
    assert!(*waited >= bound, "{waited:?}");
    assert!(*attempts > 1, "it asked again: {attempts}");
    assert_eq!(*ahead, 0, "nobody in this process was ahead of it");
    let holder = holder
        .as_deref()
        .expect("a refusal that knows its path names the holder");
    assert!(
        holder.contains("held by another process"),
        "an unregistered holder is not claimed for this process: {holder}"
    );
    assert!(
        took >= bound && took < bound + MAX_PAUSE + SLACK,
        "{took:?}"
    );
    let text = refused.to_string();
    assert!(text.contains("of a 120 ms bound"), "{text}");
    assert!(text.contains("still held after waiting"), "{text}");
    assert!(text.contains(holder), "the text carries the holder: {text}");
    assert!(text.contains(&format!("doubling to {} ms", MAX_PAUSE.as_millis())));
}

#[test]
fn a_zero_bound_is_one_attempt_and_no_pause() {
    let mut pauses = 0;
    let mut attempts = 0;
    let got = patiently(
        Duration::ZERO,
        Instant::now,
        |_| pauses += 1,
        || {
            attempts += 1;
            Err(TryLockError::WouldBlock)
        },
    );
    assert!(matches!(got, Err(Impatient::Held { attempts: 1, .. })));
    assert_eq!((attempts, pauses), (1, 0));
}

#[test]
fn a_free_lock_is_taken_at_the_first_attempt() {
    let mut pauses = 0;
    let got = patiently(
        Duration::from_secs(1),
        Instant::now,
        |_| pauses += 1,
        || Ok(()),
    );
    assert!(got.is_ok());
    assert_eq!(pauses, 0);
}

#[test]
fn a_host_refusal_is_not_waited_on() {
    let mut pauses = 0;
    let got = patiently(
        Duration::from_secs(1),
        Instant::now,
        |_| pauses += 1,
        || Err(TryLockError::Error(std::io::Error::other("EBADF"))),
    );
    let Err(Impatient::Host(why)) = got else {
        panic!("a host refusal: {got:?}");
    };
    assert_eq!(why.to_string(), "EBADF");
    assert_eq!(pauses, 0);
}

#[test]
fn the_pauses_double_from_one_ms_to_the_patience_step_and_the_last_is_cut() {
    let mut slept = Vec::new();
    // A clock that never moves: only the slept pauses count.
    let frozen = Instant::now();
    let got = patiently(
        Duration::from_millis(50),
        || frozen,
        |pause| slept.push(pause.as_millis()),
        || Err(TryLockError::WouldBlock),
    );
    assert_eq!(slept, vec![1, 2, 4, 8, 16, 16, 3]);
    let Err(Impatient::Held {
        waited, attempts, ..
    }) = got
    else {
        panic!("{got:?}");
    };
    assert_eq!(waited, Duration::from_millis(50), "a test append");
    assert_eq!(attempts, 8);
}

#[test]
fn a_clock_stepping_backwards_cannot_extend_the_wait() {
    let base = Instant::now() + Duration::from_hours(1);
    let mut step = 0_u64;
    let mut slept = Duration::ZERO;
    let got = patiently(
        Duration::from_millis(100),
        || {
            step += 1;
            // Every reading is a second EARLIER than the one before.
            base.checked_sub(Duration::from_secs(step))
                .expect("in range")
        },
        |pause| slept += pause,
        || Err(TryLockError::WouldBlock),
    );
    assert_eq!(slept, Duration::from_millis(100), "the wait is the bound");
    assert!(
        matches!(got, Err(Impatient::Held { waited, .. }) if waited == Duration::from_millis(100))
    );
}

#[test]
fn a_clock_jumping_forwards_ends_the_wait_early_never_late() {
    let base = Instant::now();
    let mut readings = 0_u32;
    let mut pauses = 0;
    let got = patiently(
        Duration::from_millis(100),
        || {
            readings += 1;
            // The second reading is an hour later (a suspended laptop).
            if readings == 1 {
                base
            } else {
                base + Duration::from_hours(1)
            }
        },
        |_| pauses += 1,
        || Err(TryLockError::WouldBlock),
    );
    assert!(matches!(got, Err(Impatient::Held { attempts: 1, .. })));
    assert_eq!(pauses, 0);
}

#[test]
fn a_garbage_bound_is_cut_to_the_ceiling() {
    let frozen = Instant::now();
    let mut slept = Duration::ZERO;
    let mut attempts_seen = 0_u32;
    let got = patiently(
        Duration::MAX,
        || frozen,
        |pause| slept += pause,
        || {
            attempts_seen += 1;
            Err(TryLockError::WouldBlock)
        },
    );
    assert_eq!(slept, MAX_PATIENCE);
    let Err(Impatient::Held {
        bound, attempts, ..
    }) = got
    else {
        panic!("{got:?}");
    };
    assert_eq!(bound, MAX_PATIENCE);
    assert_eq!(attempts, attempts_seen);
    // 1+2+4+8 ms, then 16 ms pauses: bounded, about 1 s / 16 ms of them.
    assert!(attempts < 80, "{attempts}");
}

#[test]
fn a_holder_whose_file_is_replaced_still_hands_the_waiter_the_file_it_opened() {
    // The lock is on the open file description the waiter opened. A file
    // renamed over its path does not move the waiter: once the holder lets go
    // the waiter holds the OLD file. A caller whose file must still be the
    // one at the path checks that itself (`api::audit` compares device and
    // inode after taking the lock, AG-06).
    use std::os::unix::fs::MetadataExt as _;
    let path = lock_file("replaced");
    let holder = Flock::try_lock(open(&path), path.clone()).expect("free");
    let waiter_file = open(&path);
    let old = waiter_file.metadata().expect("old").ino();
    let fresh = path.with_extension("fresh");
    File::create(&fresh).expect("a new file");
    fs::rename(&fresh, &path).expect("replaced");
    let released = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        holder.release().expect("released");
    });
    let taken = Flock::try_lock_patiently(
        waiter_file,
        path.clone(),
        Duration::from_secs(1),
        "a test append",
    )
    .expect("taken");
    released.join().expect("the holder");
    assert_eq!(taken.metadata().expect("locked").ino(), old);
    assert_ne!(fs::metadata(&path).expect("new").ino(), old);
}

#[test]
fn sixty_four_patient_lockers_all_take_the_lock_in_turn() {
    let path = lock_file("sixty-four");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(64));
    let inside = std::sync::Arc::new(AtomicU64::new(0));
    let handles: Vec<_> = (0..64)
        .map(|_| {
            let (path, barrier, inside) = (
                path.clone(),
                std::sync::Arc::clone(&barrier),
                std::sync::Arc::clone(&inside),
            );
            std::thread::spawn(move || {
                barrier.wait();
                let held = Flock::try_lock_patiently(
                    open(&path),
                    path.clone(),
                    MAX_PATIENCE,
                    "a test append",
                )?;
                assert_eq!(inside.fetch_add(1, Ordering::SeqCst), 0, "exclusive");
                std::thread::sleep(Duration::from_millis(2));
                inside.fetch_sub(1, Ordering::SeqCst);
                held.release().expect("released");
                Ok::<(), Impatient>(())
            })
        })
        .collect();
    let refused = handles
        .into_iter()
        .map(|h| h.join().expect("a locker"))
        .filter(Result::is_err)
        .count();
    assert_eq!(refused, 0);
}

#[test]
fn eight_lockers_appending_in_a_loop_are_never_starved() {
    // The case a poller alone loses: each thread takes the lock again the
    // moment it lets go, so a sleeping poller wakes to a held lock. The
    // queue hands the lock to the one that asked first.
    let path = lock_file("starve");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let (path, barrier) = (path.clone(), std::sync::Arc::clone(&barrier));
            std::thread::spawn(move || {
                barrier.wait();
                let mut refused = 0_u32;
                for _ in 0..100 {
                    match Flock::try_lock_patiently(
                        open(&path),
                        path.clone(),
                        Duration::from_secs(1),
                        "a test append",
                    ) {
                        Ok(held) => {
                            std::thread::sleep(Duration::from_millis(1));
                            held.release().expect("released");
                        }
                        Err(_) => refused += 1,
                    }
                }
                refused
            })
        })
        .collect();
    let refused: u32 = handles
        .into_iter()
        .map(|h| h.join().expect("a locker"))
        .sum();
    assert_eq!(refused, 0, "800 takes, none starved past the bound");
}

#[test]
fn lockers_in_one_process_take_turns_in_the_order_they_asked() {
    let path = lock_file("order");
    let first =
        Flock::try_lock_patiently(open(&path), path.clone(), Duration::ZERO, "a test append")
            .expect("free, nobody ahead");
    let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let handles: Vec<_> = (0..4_u32)
        .map(|i| {
            let (mine, order) = (path.clone(), std::sync::Arc::clone(&order));
            // Each asks only once the one before it is queued.
            let queued_before = tickets(&path);
            let handle = std::thread::spawn(move || {
                let held = Flock::try_lock_patiently(
                    open(&mine),
                    mine.clone(),
                    Duration::from_secs(1),
                    "a test append",
                )
                .expect("its turn comes");
                order.lock().expect("order").push(i);
                held.release().expect("released");
            });
            while tickets(&path) == queued_before {
                std::thread::yield_now();
            }
            handle
        })
        .collect();
    first.release().expect("released");
    for h in handles {
        h.join().expect("a locker");
    }
    assert_eq!(*order.lock().expect("order"), vec![0, 1, 2, 3]);
    assert!(!recorded(&path), "an empty queue is forgotten");
}

#[test]
fn a_locker_that_gives_up_hands_its_place_on() {
    let path = lock_file("gives-up");
    let first =
        Flock::try_lock_patiently(open(&path), path.clone(), Duration::ZERO, "a test append")
            .expect("free");
    // Behind the holder: one that gives up after 50 ms, then one that waits.
    let quitter = {
        let path = path.clone();
        std::thread::spawn(move || {
            Flock::try_lock_patiently(
                open(&path),
                path.clone(),
                Duration::from_millis(50),
                "a test append",
            )
            .map(drop)
        })
    };
    let refused = quitter.join().expect("the quitter");
    let Err(Impatient::Held {
        attempts, ahead, ..
    }) = refused
    else {
        panic!("it gave up: {refused:?}");
    };
    assert_eq!(
        (attempts, ahead),
        (0, 1),
        "it never reached the lock; one was ahead"
    );
    let patient = {
        let path = path.clone();
        std::thread::spawn(move || {
            Flock::try_lock_patiently(
                open(&path),
                path.clone(),
                Duration::from_secs(1),
                "a test append",
            )
            .map(drop)
        })
    };
    std::thread::sleep(Duration::from_millis(30));
    first.release().expect("released");
    patient
        .join()
        .expect("the patient one")
        .expect("the place given up is skipped, not waited on");
    assert!(!recorded(&path), "an empty queue is forgotten");
}

#[test]
fn a_zero_bound_behind_a_locker_in_this_process_refuses_at_once() {
    let path = lock_file("zero-behind");
    let first =
        Flock::try_lock_patiently(open(&path), path.clone(), Duration::ZERO, "a test append")
            .expect("free");
    let started = Instant::now();
    let refused =
        Flock::try_lock_patiently(open(&path), path.clone(), Duration::ZERO, "a test append")
            .map(drop)
            .expect_err("one is ahead");
    assert!(started.elapsed() < MAX_PAUSE + SLACK);
    assert!(matches!(
        refused,
        Impatient::Held {
            ahead: 1,
            attempts: 0,
            ..
        }
    ));
    first.release().expect("released");
}

#[test]
fn a_patient_holder_is_named_while_it_holds_and_forgotten_when_it_lets_go() {
    // The repair's first half: the holder is named through R3-08's one
    // registry and `holder_note`, with what its caller said it is doing, and
    // only while it holds the lock.
    let path = lock_file("named");
    let held = std::thread::Builder::new()
        .name("ag06-holder".to_owned())
        .spawn({
            let path = path.clone();
            move || {
                Flock::try_lock_patiently(
                    open(&path),
                    path.clone(),
                    Duration::ZERO,
                    "a test append",
                )
                .expect("free")
            }
        })
        .expect("a named thread")
        .join()
        .expect("the taker");
    let note = holder_note(&path);
    for part in [
        "held by this process",
        "thread ag06-holder",
        "doing a test append",
    ] {
        assert!(note.contains(part), "{part:?} missing from: {note}");
    }
    held.release().expect("released");
    let after = holder_note(&path);
    assert!(!after.contains("held by this process"), "{after}");
    assert!(!recorded(&path), "nothing of it stays in the registry");
}

#[test]
fn a_successors_name_survives_its_predecessors_release() {
    // The repair's second half (AG-06 repair 1, the verifier's blocking item
    // 2): a registration kept beside the guard was erased by the
    // predecessor's late end, so the registry named nobody while the
    // successor held the lock. The guard itself registers after `flock(2)`
    // grants the lock and forgets before it unlocks, and the turn passes on
    // only after the unlock, so the successor's entry is never erased.
    let path = lock_file("successor");
    let first = Flock::try_lock_patiently(open(&path), path.clone(), Duration::ZERO, "the first")
        .expect("free");
    let (took, took_seen) = std::sync::mpsc::channel::<()>();
    let (go, go_seen) = std::sync::mpsc::channel::<()>();
    let successor = std::thread::Builder::new()
        .name("ag06-successor".to_owned())
        .spawn({
            let path = path.clone();
            move || {
                let held = Flock::try_lock_patiently(
                    open(&path),
                    path.clone(),
                    MAX_PATIENCE,
                    "the second",
                )
                .expect("its turn comes");
                took.send(()).expect("told");
                go_seen.recv().expect("let go");
                held.release().expect("released");
            }
        })
        .expect("a named thread");
    while tickets(&path) < 2 {
        std::thread::yield_now();
    }
    first.release().expect("released");
    took_seen.recv().expect("the successor holds it");
    let note = holder_note(&path);
    assert!(
        note.contains("thread ag06-successor") && note.contains("doing the second"),
        "the successor is named while it holds the lock: {note}"
    );
    assert!(!note.contains("the first"), "{note}");
    assert!(
        matches!(open(&path).try_lock(), Err(TryLockError::WouldBlock)),
        "and it does hold it"
    );
    go.send(()).expect("go");
    successor.join().expect("the successor");
    assert!(!holder_note(&path).contains("held by this process"));
    assert!(
        !recorded(&path),
        "an empty queue and no holder are forgotten"
    );
}

#[test]
fn no_reader_here_ever_finds_the_lock_held_and_its_holder_unnamed() {
    // AG-06 repair 2 (the verifier's blocking item 2, its sampler made
    // exact): eight patient lockers hand one lock to each other 1,600 times
    // while a sampler, holding the registry's lock as every reader here
    // does, asks the kernel whether the lock is held and the registry
    // whether a holder here is named. Before the repair the grant and the
    // record were two steps (and the forgetting and the unlock two more), so
    // the sampler met the lock held with nobody named at the instants
    // between them. After it, both pairs are one step under the registry's
    // lock and no sample can.
    const LOCKERS: usize = 8;
    const TAKES: usize = 200;
    let path = lock_file("sampled");
    let done = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let lockers: Vec<_> = (0..LOCKERS)
        .map(|_| {
            let (path, done) = (path.clone(), std::sync::Arc::clone(&done));
            std::thread::spawn(move || {
                let file = open(&path);
                let mut refused = 0_usize;
                for _ in 0..TAKES {
                    match Flock::try_lock_patiently(&file, path.as_path(), MAX_PATIENCE, "a take") {
                        Ok(held) => {
                            // Held across a yield, so the sampler meets it
                            // held as well as in hand-off.
                            std::thread::yield_now();
                            held.release().expect("released");
                        }
                        Err(_) => refused += 1,
                    }
                }
                done.fetch_add(1, Ordering::SeqCst);
                refused
            })
        })
        .collect();
    let probe = open(&path);
    let key = key_of(&path);
    let (mut held_samples, mut unnamed) = (0_u64, 0_u64);
    while done.load(Ordering::SeqCst) < LOCKERS {
        let registry = registry();
        let held = match probe.try_lock() {
            Ok(()) => {
                probe.unlock().expect("the probe lets go");
                false
            }
            Err(TryLockError::WouldBlock) => true,
            Err(TryLockError::Error(why)) => panic!("the probe was refused: {why}"),
        };
        let named = registry
            .as_ref()
            .and_then(|entries| entries.get(&key))
            .is_some_and(|entry| entry.writer.is_some());
        drop(registry);
        if held {
            held_samples += 1;
            if !named {
                unnamed += 1;
            }
        }
        std::thread::yield_now();
    }
    println!("AG06-SAMPLER held_samples={held_samples} unnamed={unnamed}");
    let refused: usize = lockers
        .into_iter()
        .map(|locker| locker.join().expect("a locker"))
        .sum();
    assert_eq!(refused, 0, "every take landed within the ceiling");
    assert!(
        held_samples >= 100,
        "the sampler met the lock held often enough to mean something: {held_samples}"
    );
    assert_eq!(
        unnamed, 0,
        "of {held_samples} samples that met the lock held by this process, {unnamed} found \
         nobody named"
    );
    assert!(!recorded(&path), "nothing of it stays in the registry");
}

#[test]
fn a_refusal_behind_a_holder_in_this_process_names_it() {
    let path = lock_file("named-refusal");
    let first = Flock::try_lock_patiently(
        open(&path),
        path.clone(),
        Duration::ZERO,
        "the journal index build",
    )
    .expect("free");
    let refused = Flock::try_lock_patiently(
        open(&path),
        path.clone(),
        Duration::from_millis(30),
        "a test append",
    )
    .map(drop)
    .expect_err("one holds it");
    let text = refused.to_string();
    assert!(
        text.contains("held by this process") && text.contains("doing the journal index build"),
        "{text}"
    );
    assert!(
        matches!(
            refused,
            Impatient::Held {
                attempts: 0,
                ahead: 1,
                ..
            }
        ),
        "it waited in this process's queue: {refused:?}"
    );
    first.release().expect("released");
}

#[test]
fn one_wait_loop_one_pause_one_ceiling() {
    // AG-06 repair 1: R3-08's patient shared lock and AG-06's patient
    // exclusive lock share one loop, one pause, one ceiling and one registry;
    // a second of any of them is what the repair removed.
    let source = include_str!("flock.rs");
    let production = source
        .split("#[cfg(test)]\n#[path")
        .next()
        .expect("the production half");
    assert_eq!(
        production.matches("pause.saturating_mul(2)").count(),
        1,
        "one loop doubles a pause"
    );
    assert_eq!(
        production.matches("Duration::from_millis(16)").count(),
        1,
        "one pause constant"
    );
    assert_eq!(
        production.matches("Duration::from_secs(1)").count(),
        1,
        "one ceiling"
    );
    assert_eq!(
        production.matches("\nstatic ").count(),
        1,
        "one registry of holders and waiters"
    );
    assert!(
        !production.contains("std::thread::sleep("),
        "no lock sleeps outside the one loop"
    );
    assert_eq!(
        production.matches(" patiently(").count(),
        3,
        "the loop's definition and its two callers, the shared lock and the exclusive one"
    );
}
