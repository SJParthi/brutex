#![cfg(test)]
#![allow(clippy::expect_used, clippy::panic)]
//! [`Flock`] releases by `File::unlock`, never by close. Every proof below
//! keeps a duplicate descriptor open — `File::try_clone`, the deterministic
//! model of the reference a spawned child holds until its exec — because a
//! guard that released only by close passes every one of these checks the
//! moment the duplicate is closed first. D-0693.

use super::{Flock, Unreleased};
use std::cell::Cell;
use std::error::Error as _;
use std::fs::{self, File, TryLockError};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

thread_local! {
    /// Armed by [`refuse_next_unlock`], consumed by the next unlock on this
    /// thread and by nothing else.
    static REFUSE_NEXT_UNLOCK: Cell<bool> = const { Cell::new(false) };
    /// Every unlock this thread has asked the seam about, refused or not.
    static UNLOCKS: Cell<u64> = const { Cell::new(0) };
}

/// Makes the next `Flock` unlock on this thread fail, once.
///
/// Per-thread, so a refusal armed by one test can never land in a guard
/// another test is dropping on a neighbouring thread.
pub(crate) fn refuse_next_unlock() {
    REFUSE_NEXT_UNLOCK.with(|armed| armed.set(true));
}

/// The seam `super::unlock` asks before it unlocks: counts the call, and
/// disarms and reports a refusal armed by [`refuse_next_unlock`].
pub(super) fn take_unlock_refusal() -> bool {
    UNLOCKS.with(|count| count.set(count.get() + 1));
    REFUSE_NEXT_UNLOCK.with(|armed| armed.replace(false))
}

/// How many unlocks this thread has made through the seam.
fn unlocks() -> u64 {
    UNLOCKS.with(Cell::get)
}

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A scratch directory holding one lock file, removed on drop.
struct Scratch {
    dir: PathBuf,
    lock: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "brutex-store-flock-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _stale = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch directory");
        let lock = dir.join("flock.lock");
        File::create(&lock).expect("the lock file");
        Self { dir, lock }
    }

    /// A NEW open file description of the lock file.
    fn open(&self) -> File {
        File::options()
            .read(true)
            .write(true)
            .open(&self.lock)
            .expect("the lock file opens")
    }

    /// What another holder sees: a fresh description asking for the
    /// exclusive lock without waiting.
    fn probe(&self) -> Result<(), TryLockError> {
        let held = Flock::try_lock(self.open(), self.lock.as_path())?;
        held.release().expect("the probe releases what it took");
        Ok(())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _cleanup = fs::remove_dir_all(&self.dir);
    }
}

/// The four ways a guard can be taken.
#[derive(Debug, Clone, Copy)]
enum Take {
    Lock,
    LockShared,
    TryLock,
    TryLockShared,
}

const TAKES: [Take; 4] = [
    Take::Lock,
    Take::LockShared,
    Take::TryLock,
    Take::TryLockShared,
];

impl Take {
    fn on<F: std::borrow::Borrow<File>, P: AsRef<Path>>(self, file: F, path: P) -> Flock<F, P> {
        match self {
            Self::Lock => Flock::lock(file, path).expect("lock"),
            Self::LockShared => Flock::lock_shared(file, path).expect("lock_shared"),
            Self::TryLock => Flock::try_lock(file, path).expect("try_lock"),
            Self::TryLockShared => Flock::try_lock_shared(file, path).expect("try_lock_shared"),
        }
    }
}

fn would_block(result: &Result<(), TryLockError>) -> bool {
    matches!(result, Err(TryLockError::WouldBlock))
}

/// AN OWNED GUARD RELEASES THE LOCK WHILE A DUPLICATE OF ITS FILE IS OPEN,
/// whether it is dropped or released.
///
/// Fails if `Drop::drop` does nothing or if `unlock` stops unlocking: the
/// duplicate then keeps the lock alive and the probe refuses.
#[test]
fn an_owned_lock_is_released_despite_a_duplicated_descriptor() {
    let scratch = Scratch::new("owned");
    for take in TAKES {
        for release in [false, true] {
            let guard = take.on(scratch.open(), scratch.lock.clone());
            assert!(
                would_block(&scratch.probe()),
                "{take:?}: the premise, a held lock refuses a second description"
            );
            let child = guard.try_clone().expect("the duplicate a child would hold");
            if release {
                guard.release().expect("an ordinary unlock succeeds");
            } else {
                drop(guard);
            }
            scratch.probe().unwrap_or_else(|why| {
                panic!(
                    "{take:?} (release: {release}): the lock survived on the duplicate \
                     descriptor — {why}"
                )
            });
            drop(child);
        }
    }
}

/// A BORROWED GUARD RELEASES THE LOCK WHILE THE OWNER AND A DUPLICATE ARE BOTH
/// STILL OPEN.
///
/// This is the case close can never cover: the guard owns no descriptor at
/// all, so only `File::unlock` frees the lock. Also exercises `DerefMut`, by
/// writing and reading back through a guard over `&mut File`.
#[test]
fn a_borrowed_lock_is_released_despite_a_duplicated_descriptor() {
    let scratch = Scratch::new("borrowed");
    let mut owner = scratch.open();
    let child = owner.try_clone().expect("the duplicate a child would hold");
    for take in TAKES {
        for release in [false, true] {
            let guard = take.on(&owner, scratch.lock.as_path());
            assert!(would_block(&scratch.probe()), "{take:?}: the premise");
            if release {
                guard.release().expect("an ordinary unlock succeeds");
            } else {
                drop(guard);
            }
            scratch.probe().unwrap_or_else(|why| {
                panic!("{take:?} (release: {release}) over &File kept the lock — {why}")
            });

            let mut guard = take.on(&mut owner, scratch.lock.as_path());
            assert!(would_block(&scratch.probe()), "{take:?}: the premise");
            guard.set_len(0).expect("truncate through Deref");
            guard.seek(SeekFrom::Start(0)).expect("rewind");
            guard.write_all(b"flock").expect("write through DerefMut");
            guard.seek(SeekFrom::Start(0)).expect("rewind");
            let mut back = String::new();
            guard
                .read_to_string(&mut back)
                .expect("read through DerefMut");
            assert_eq!(back, "flock", "the guard hands out the file it locked");
            if release {
                guard.release().expect("an ordinary unlock succeeds");
            } else {
                drop(guard);
            }
            scratch.probe().unwrap_or_else(|why| {
                panic!("{take:?} (release: {release}) over &mut File kept the lock — {why}")
            });
        }
    }
    drop(child);
    drop(owner);
}

/// A REFUSED RELEASE IS RETURNED, NAMING THE FILE, AND IS NOT REPEATED.
///
/// The refusal is injected, because no real file refuses an unlock on request.
/// It never drops a held guard under an injected fault, so it writes nothing
/// to the process-wide sink `crate::emits` counts.
#[test]
fn a_refused_release_is_returned_naming_the_file() {
    let scratch = Scratch::new("refused");
    let guard = Flock::lock(scratch.open(), scratch.lock.clone()).expect("lock");
    let before = unlocks();
    refuse_next_unlock();
    let refused: Unreleased = guard
        .release()
        .expect_err("the seam refused the unlock and release must say so");
    assert_eq!(
        unlocks() - before,
        1,
        "the release asked once, and Drop did not ask again after it"
    );
    assert_eq!(
        refused.path, scratch.lock,
        "the refusal names the lock file"
    );

    let sentence = refused.to_string();
    let named = scratch.lock.display().to_string();
    assert!(
        sentence.contains(&named),
        "the sentence names the file: {sentence}"
    );
    assert!(
        sentence.contains("could not be released before its descriptor closed"),
        "the sentence says what failed: {sentence}"
    );
    assert!(
        sentence.contains("unlock refused by the flock test seam"),
        "the sentence carries the host's refusal: {sentence}"
    );
    assert_eq!(
        refused.source().map(ToString::to_string).as_deref(),
        Some("unlock refused by the flock test seam"),
        "the host's refusal is the source"
    );

    let lifted = io::Error::from(refused);
    assert_eq!(
        lifted.kind(),
        io::ErrorKind::Other,
        "the host's kind survives"
    );
    assert!(
        lifted.to_string().contains(&named),
        "an io::Error still names the file: {lifted}"
    );

    // The guard's own descriptor closed with the release and nothing
    // duplicated it, so the kernel has freed the lock.
    scratch
        .probe()
        .expect("with no duplicate, closing the only descriptor freed the lock");
}

/// A CONTENDED LOCK IS REFUSED WITHOUT WAITING, AND A REFUSAL HOLDS NOTHING.
#[test]
fn a_contended_lock_is_refused_and_takes_nothing() {
    let scratch = Scratch::new("contended");
    let exclusive = Flock::try_lock(scratch.open(), scratch.lock.clone()).expect("free");
    assert!(matches!(
        Flock::try_lock(scratch.open(), scratch.lock.as_path()),
        Err(TryLockError::WouldBlock)
    ));
    assert!(matches!(
        Flock::try_lock_shared(scratch.open(), scratch.lock.as_path()),
        Err(TryLockError::WouldBlock)
    ));
    exclusive.release().expect("release");

    let first = Flock::try_lock_shared(scratch.open(), scratch.lock.clone()).expect("shared");
    let second = Flock::try_lock_shared(scratch.open(), scratch.lock.clone())
        .expect("two shared guards coexist");
    assert!(would_block(&scratch.probe()), "readers exclude a writer");
    first.release().expect("release");
    second.release().expect("release");
    scratch.probe().expect("every refusal above took nothing");
}
