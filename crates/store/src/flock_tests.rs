#![cfg(test)]
#![allow(clippy::expect_used, clippy::panic)]
//! [`Flock`] releases by `File::unlock`, never by close. The two proofs below
//! that a guard releases its lock, owned and borrowed, keep a duplicate
//! descriptor open — `File::try_clone`, the deterministic model of the
//! reference a spawned child holds until its exec — because a guard that
//! released only by close passes their checks the moment the duplicate is
//! closed first. The others prove the mode each constructor takes, what a
//! refused release reports and leaves held, and contention. D-0693.

use super::{Flock, Unreleased};
use std::cell::Cell;
use std::error::Error as _;
use std::fs::{self, File, TryLockError};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

thread_local! {
    /// Armed by [`refuse_unlock`]: how many unlocks on this thread to let
    /// through first, and the kind the refusal then carries. Consumed by
    /// that one unlock and by nothing else.
    static REFUSE_UNLOCK: Cell<Option<(u64, io::ErrorKind)>> = const { Cell::new(None) };
    /// Every unlock this thread has asked the seam about, refused or not.
    static UNLOCKS: Cell<u64> = const { Cell::new(0) };
}

/// Makes the next `Flock` unlock on this thread fail, once, with kind
/// `Other`.
///
/// Per-thread, so a refusal armed by one test can never land in a guard
/// another test is dropping on a neighbouring thread.
pub(crate) fn refuse_next_unlock() {
    refuse_unlock(0, io::ErrorKind::Other);
}

/// Lets `after` unlocks on this thread through, then makes the next one fail,
/// once, with an `io::Error` of `kind`.
///
/// The kind is the caller's so a test can tell a kept kind from a hardcoded
/// `Other`, and `after` is there because a production call such as
/// `repair::publish` unlocks other guards before the release under test.
pub(crate) fn refuse_unlock(after: u64, kind: io::ErrorKind) {
    REFUSE_UNLOCK.with(|armed| armed.set(Some((after, kind))));
}

/// The seam `super::unlock` asks before it unlocks: counts the call, and
/// disarms and reports the kind of a refusal armed by [`refuse_unlock`] once
/// the unlocks it lets through have passed.
pub(super) fn take_unlock_refusal() -> Option<io::ErrorKind> {
    UNLOCKS.with(|count| count.set(count.get() + 1));
    REFUSE_UNLOCK.with(|armed| match armed.get() {
        Some((0, kind)) => {
            armed.set(None);
            Some(kind)
        }
        Some((after, kind)) => {
            armed.set(Some((after - 1, kind)));
            None
        }
        None => None,
    })
}

/// The part of [`Unreleased`]'s sentence that says how long the lock may last.
///
/// True of an owned guard and of a borrowed one. The open file description
/// keeps a lock nobody unlocked until the last descriptor referring to it
/// closes, and for a borrowed guard that includes the owner's, which the guard
/// never closes. A later unlock of the same description frees it sooner, as
/// `a_lock_a_refused_release_left_is_freed_by_a_later_release_over_the_same_owner`
/// shows, so the sentence says "may", not "stays".
const UNRELEASED_SAYS: &str = "could not be released, and it may stay held until its open \
                               file description is unlocked again or every descriptor of it \
                               has closed";

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

    /// What another reader sees: a fresh description asking for a shared
    /// lock without waiting.
    fn shared_probe(&self) -> Result<(), TryLockError> {
        let held = Flock::try_lock_shared(self.open(), self.lock.as_path())?;
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

/// While a guard taken by `take` over `over` is held: an exclusive probe is
/// refused, and a shared probe is granted exactly when `take` is shared.
fn holds_the_mode_it_names(scratch: &Scratch, take: Take, over: &str) {
    assert!(
        would_block(&scratch.probe()),
        "{take:?} over {over}: a second description's exclusive lock is refused \
         beside any guard"
    );
    let reader = scratch.shared_probe();
    if matches!(take, Take::LockShared | Take::TryLockShared) {
        reader.unwrap_or_else(|why| {
            panic!(
                "{take:?} over {over} took the exclusive lock: a second description's \
                 shared lock was refused — {why}"
            )
        });
    } else {
        assert!(
            would_block(&reader),
            "{take:?} over {over} took a shared lock: a second description's shared \
             lock was granted"
        );
    }
}

/// EACH CONSTRUCTOR TAKES THE MODE IT NAMES, OVER AN OWNED FILE AND A
/// BORROWED ONE.
///
/// The two duplicate tests above probe only with an exclusive `try_lock`,
/// which a shared holder refuses exactly as an exclusive one does, so they
/// cannot tell the two modes apart: `lock_shared` taking the exclusive lock
/// passed them and every other store test. Here, while each guard is held, a
/// second description asks for a shared lock without waiting. It is granted
/// beside `lock_shared` and `try_lock_shared` and refused beside `lock` and
/// `try_lock`, and an exclusive `try_lock` is refused beside all four, for
/// `F` = `File`, `&File` and `&mut File`.
///
/// Every probe is a `try_` call, and the lock is proved free before each
/// guard is taken, so neither a constructor that took the wrong mode nor a
/// release that left the lock held can leave `lock` or `lock_shared` waiting:
/// either fails the test rather than hanging it.
#[test]
fn each_constructor_takes_the_mode_it_names_owned_or_borrowed() {
    let scratch = Scratch::new("mode");
    let mut owner = scratch.open();
    for take in TAKES {
        let free = |over: &str| {
            scratch.probe().unwrap_or_else(|why| {
                panic!("{take:?} over {over}: the last guard's release left the lock held — {why}")
            });
        };

        free("File");
        let guard = take.on(scratch.open(), scratch.lock.clone());
        holds_the_mode_it_names(&scratch, take, "File");
        guard.release().expect("an ordinary unlock succeeds");

        free("&File");
        let guard = take.on(&owner, scratch.lock.as_path());
        holds_the_mode_it_names(&scratch, take, "&File");
        guard.release().expect("an ordinary unlock succeeds");

        free("&mut File");
        let guard = take.on(&mut owner, scratch.lock.as_path());
        holds_the_mode_it_names(&scratch, take, "&mut File");
        guard.release().expect("an ordinary unlock succeeds");
    }
    drop(owner);
    scratch
        .probe()
        .expect("every guard above was released, and no probe kept what it took");
}

/// A REFUSED RELEASE IS RETURNED, NAMING THE FILE, AND IS NOT REPEATED.
///
/// The refusal is injected, because no real file refuses an unlock on request.
/// It never drops a held guard under an injected fault, so it writes nothing
/// to the process-wide sink `crate::emits` counts.
///
/// The injected kind is `PermissionDenied`, not the `Other` an
/// `io::Error::other` carries, so a `From<Unreleased>` that replaced the
/// host's kind with `Other` fails here rather than passing by coincidence.
#[test]
fn a_refused_release_is_returned_naming_the_file() {
    let scratch = Scratch::new("refused");
    let guard = Flock::lock(scratch.open(), scratch.lock.clone()).expect("lock");
    let before = unlocks();
    refuse_unlock(0, io::ErrorKind::PermissionDenied);
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
        sentence.contains(UNRELEASED_SAYS),
        "the sentence says what failed and how long the lock lasts: {sentence}"
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
    assert_eq!(
        refused.why.kind(),
        io::ErrorKind::PermissionDenied,
        "the premise: the seam refused with the kind it was armed with"
    );

    let lifted = io::Error::from(refused);
    assert_eq!(
        lifted.kind(),
        io::ErrorKind::PermissionDenied,
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

/// A BORROWED GUARD WHOSE RELEASE IS REFUSED LEAVES THE LOCK HELD WHILE ITS
/// OWNER IS OPEN AND NOTHING UNLOCKS IT AGAIN, AND ITS REFUSAL SAYS NOTHING
/// CLOSED.
///
/// Dropping or consuming a guard over `&File` closes nothing: the owner keeps
/// the descriptor open, and the lock stays on its open file description until
/// the owner closes it or that description is unlocked again. A refusal that
/// said the lock "could not be released before its descriptor closed" told the
/// reader of a close that never happened, and so the wrong lifetime. The
/// sentence is checked on this guard, where nothing else unlocks the
/// description and the owner's close frees the lock.
///
/// Released rather than dropped, so it writes nothing to the process-wide sink
/// `crate::emits` counts.
#[test]
fn a_refused_release_of_a_borrowed_guard_holds_the_lock_until_its_owner_closes() {
    let scratch = Scratch::new("refused-borrowed");
    let owner = scratch.open();
    let guard = Flock::lock(&owner, scratch.lock.as_path()).expect("lock");
    refuse_unlock(0, io::ErrorKind::PermissionDenied);
    let refused = guard
        .release()
        .expect_err("the seam refused the unlock and release must say so");

    let sentence = refused.to_string();
    assert!(
        sentence.contains(UNRELEASED_SAYS),
        "the sentence says how long the lock lasts: {sentence}"
    );
    assert!(
        !sentence.contains("descriptor closed:"),
        "the sentence claims a close that never happened for a borrowed \
         guard: {sentence}"
    );
    assert!(
        would_block(&scratch.probe()),
        "the guard is gone and the owner is open, so the lock is still held"
    );
    drop(owner);
    scratch
        .probe()
        .expect("the owner's close was the last descriptor, and it freed the lock");
}

/// A LOCK A REFUSED RELEASE LEFT HELD IS FREED BY A LATER RELEASE OVER THE
/// SAME OWNER, WHILE THE OWNER IS STILL OPEN.
///
/// So the refusal cannot say the lock stays held until the owner closes the
/// file. `cli`'s observation `ReadLease` takes a guard over one cached owner
/// for each projection, and the next projection's release unlocks the same
/// description.
///
/// Released rather than dropped, so it writes nothing to the process-wide sink
/// `crate::emits` counts.
#[test]
fn a_lock_a_refused_release_left_is_freed_by_a_later_release_over_the_same_owner() {
    let scratch = Scratch::new("refused-relocked");
    let owner = scratch.open();
    let guard = Flock::lock(&owner, scratch.lock.as_path()).expect("lock");
    refuse_unlock(0, io::ErrorKind::PermissionDenied);
    let refused = guard
        .release()
        .expect_err("the seam refused the unlock and release must say so");
    assert!(
        refused.to_string().contains(UNRELEASED_SAYS),
        "the sentence says the lock may stay held: {refused}"
    );
    assert!(
        would_block(&scratch.probe()),
        "the premise: the refused release left the lock held"
    );
    Flock::lock(&owner, scratch.lock.as_path())
        .expect("the owner's own description takes its lock again")
        .release()
        .expect("and releases it");
    scratch
        .probe()
        .expect("the owner is still open, and the later release freed the lock");
    drop(owner);
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
