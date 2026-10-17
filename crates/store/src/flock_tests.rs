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

/// The warning records a refused unlock, not a guaranteed surviving lock.
/// Check the note's qualification against owned, duplicated and borrowed files.
#[test]
fn a_refused_drop_unlock_may_outlive_the_guard_but_an_owned_last_close_frees_it() {
    let note = include_str!("flock.rs")
        .split_once("/// A guard dropped without a release, whose unlock the host refused.")
        .expect("the Drop note's doc")
        .1
        .split_once("fn note_unreleased(")
        .expect("the Drop note's function")
        .0
        .split_whitespace()
        .filter(|word| *word != "///")
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        note.contains("The line records a refused unlock; the lock may outlive the guard"),
        "the warning cannot assert that every refused unlock leaves a lock held: {note}"
    );
    let _sink_is_mine = crate::emits::hold_the_sink();
    let scratch = Scratch::new("refused-drop-lifetime");
    for duplicated in [false, true] {
        let guard = Flock::try_lock(scratch.open(), scratch.lock.clone()).expect("free lock");
        let duplicate = duplicated.then(|| guard.try_clone().expect("duplicate"));
        let before = unlocks();
        refuse_next_unlock();
        drop(guard);
        assert_eq!(unlocks() - before, 1, "the Drop unlock is refused once");
        if duplicated {
            assert!(
                would_block(&scratch.probe()),
                "the duplicate keeps the lock held"
            );
        } else {
            scratch
                .probe()
                .expect("the owned file's last close frees the lock");
        }
        drop(duplicate);
        scratch.probe().expect("no duplicate remains");
    }
    let owner = scratch.open();
    let guard = Flock::try_lock(&owner, scratch.lock.as_path()).expect("free borrowed lock");
    refuse_next_unlock();
    drop(guard);
    assert!(
        would_block(&scratch.probe()),
        "the borrowed owner's file is still open"
    );
    drop(owner);
    scratch
        .probe()
        .expect("the owner's last close frees the lock");
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

/// **A PATIENT READER WAITS FOR A WRITER, THEN IS REFUSED; AN IMPATIENT ONE
/// IS REFUSED AT ONCE** (R3-08, NEW-21). With no patience set the patient
/// call is the one attempt `try_lock_shared` always was. With patience it
/// keeps asking until the patience runs out, and only then refuses.
#[test]
fn a_patient_reader_waits_for_a_writer_then_is_refused() {
    use std::time::{Duration, Instant};
    let scratch = Scratch::new("patient-refused");
    let writer = Flock::try_lock(scratch.open(), scratch.lock.clone()).expect("free");

    let began = Instant::now();
    let impatient = Flock::try_lock_shared_patiently(scratch.open(), scratch.lock.as_path());
    let impatient_took = began.elapsed();
    assert!(matches!(impatient, Err(TryLockError::WouldBlock)));
    assert_eq!(
        super::patience(),
        Duration::ZERO,
        "no caller asked for patience"
    );

    let wait = Duration::from_millis(400);
    let began = Instant::now();
    let patient = super::with_patience(wait, || {
        assert_eq!(super::patience(), wait);
        Flock::try_lock_shared_patiently(scratch.open(), scratch.lock.as_path()).map(drop)
    });
    let patient_took = began.elapsed();
    assert!(matches!(patient, Err(TryLockError::WouldBlock)));
    assert!(
        patient_took >= wait,
        "it waited its whole patience: {patient_took:?}"
    );
    assert!(
        patient_took < wait + Duration::from_millis(400),
        "and no longer than one pause past it: {patient_took:?}"
    );
    assert!(
        impatient_took < patient_took / 2,
        "the impatient call did not wait: {impatient_took:?} against {patient_took:?}"
    );
    writer.release().expect("release");
    scratch.probe().expect("every refusal took nothing");
}

/// **A WRITER THAT LETS GO WITHIN THE PATIENCE IS WAITED FOR, AND THE READ
/// GOES ON** (R3-08, NEW-21): the page read that met an import worker an
/// instant before it finished its day is answered, not refused.
#[test]
fn a_patient_reader_takes_the_month_a_writer_lets_go_in_time() {
    use std::time::{Duration, Instant};
    let scratch = Scratch::new("patient-served");
    let writer = Flock::try_lock(scratch.open(), scratch.lock.clone()).expect("free");
    let letting_go = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(80));
        writer.release().expect("the writer lets go");
    });
    let began = Instant::now();
    let reader = super::with_patience(Duration::from_secs(1), || {
        Flock::try_lock_shared_patiently(scratch.open(), scratch.lock.clone())
    })
    .expect("the writer let go inside the patience");
    let took = began.elapsed();
    letting_go.join().expect("the writer thread");
    assert!(
        took >= Duration::from_millis(60),
        "it waited for the writer: {took:?}"
    );
    assert!(
        would_block(&scratch.probe()),
        "the reader holds its shared lock"
    );
    reader.release().expect("release");
    scratch.probe().expect("and gave it back");
}

/// **PATIENCE IS BOUNDED, SCOPED TO ITS CALL AND RESTORED, ALSO BY A PANIC**
/// (R3-08). Whatever a caller asks for, no thread waits past
/// [`super::MAX_PATIENCE`] for one lock; the patience a nested call sets ends
/// with it; and a panic inside `with_patience` leaves the thread as it was,
/// so a reused blocking thread never inherits another request's patience.
#[test]
fn patience_is_bounded_scoped_and_restored_by_a_panic() {
    use std::time::Duration;
    assert_eq!(super::patience(), Duration::ZERO);
    super::with_patience(Duration::from_mins(1), || {
        assert_eq!(super::patience(), super::MAX_PATIENCE, "bounded");
        super::with_patience(Duration::from_millis(5), || {
            assert_eq!(super::patience(), Duration::from_millis(5), "nested");
        });
        assert_eq!(
            super::patience(),
            super::MAX_PATIENCE,
            "the outer patience is back"
        );
    });
    assert_eq!(super::patience(), Duration::ZERO, "ended with its call");
    let unwound = std::panic::catch_unwind(|| {
        super::with_patience(Duration::from_millis(250), || panic!("work that panics"));
    });
    assert!(unwound.is_err());
    assert_eq!(super::patience(), Duration::ZERO, "restored by the unwind");
}

/// **ONE REQUEST'S PATIENCE IS ONE BUDGET, NEVER ONCE PER MONTH** (R3-08,
/// NEW-21). A window scan that opens three months a writer holds waits its
/// patience once, in total; the later months are refused at once, so a slot
/// is never held for patience times months.
#[test]
fn a_requests_patience_is_spent_across_every_month_it_opens() {
    use std::time::{Duration, Instant};
    let held: Vec<Scratch> = (0..3)
        .map(|n| Scratch::new(&format!("budget-{n}")))
        .collect();
    let writers: Vec<_> = held
        .iter()
        .map(|month| Flock::try_lock(month.open(), month.lock.clone()).expect("free"))
        .collect();
    let wait = Duration::from_millis(150);
    let began = Instant::now();
    let (refusals, left) = super::with_patience(wait, || {
        let refusals = held
            .iter()
            .map(|month| {
                matches!(
                    Flock::try_lock_shared_patiently(month.open(), month.lock.as_path()),
                    Err(TryLockError::WouldBlock)
                )
            })
            .filter(|refused| *refused)
            .count();
        (refusals, super::patience())
    });
    let took = began.elapsed();
    assert_eq!(refusals, 3, "every held month is refused");
    assert_eq!(left, Duration::ZERO, "the budget was spent");
    assert!(took >= wait, "it waited its patience once: {took:?}");
    assert!(
        took < wait * 2,
        "and not once per month (3 x {wait:?}): {took:?}"
    );
    for writer in writers {
        writer.release().expect("release");
    }
}

/// **A REFUSED READER NAMES THE WRITER IN THIS PROCESS: ITS PID, ITS THREAD
/// AND THE LOCK** (R3-08, RA-298). Only a month lock's own constructors
/// register; every way a guard ends (release, drop) forgets it; and the
/// store's refusal says "being written", never "unreadable".
#[test]
fn a_refused_reader_names_the_writer_in_this_process() {
    let scratch = Scratch::new("holder");
    let pid = std::process::id().to_string();
    let lock = scratch.lock.display().to_string();
    let nobody = super::holder_note(&scratch.lock);
    assert!(nobody.contains("left no note"), "{nobody}");
    assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");

    let path = scratch.lock.clone();
    let file = scratch.open();
    let (held_tx, held_rx) = std::sync::mpsc::channel();
    let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
    let writer = std::thread::Builder::new()
        .name("r3-08-holder".to_owned())
        .spawn(move || {
            let guard = Flock::try_lock_noted(file, path).expect("free");
            held_tx.send(()).expect("the test waits");
            let _ = go_rx.recv();
            drop(guard);
        })
        .expect("a writer thread");
    held_rx.recv().expect("the writer holds the lock");
    let named = super::holder_note(&scratch.lock);
    assert!(named.contains("held by this process"), "{named}");
    assert!(named.contains(&format!("pid {pid}")), "{named}");
    assert!(named.contains("thread r3-08-holder"), "{named}");
    assert!(named.contains(&lock), "{named}");
    let said = crate::file::StoreError::Locked {
        path: scratch.lock.clone(),
    }
    .to_string();
    assert!(said.starts_with(super::BEING_WRITTEN), "{said}");
    assert!(
        said.contains("r3-08-holder") && !said.contains("nreadable"),
        "{said}"
    );
    go_tx.send(()).expect("the writer lets go");
    writer.join().expect("the writer thread");
    let after = super::holder_note(&scratch.lock);
    assert!(after.contains("left no note"), "forgotten on drop: {after}");

    let released = Flock::try_lock_noted(scratch.open(), scratch.lock.clone()).expect("free");
    assert!(super::holder_note(&scratch.lock).contains("held by this process"));
    released.release().expect("release");
    assert!(
        super::holder_note(&scratch.lock).contains("left no note"),
        "forgotten on release"
    );
    for take in TAKES {
        let plain = take.on(scratch.open(), scratch.lock.clone());
        assert!(
            super::holder_note(&scratch.lock).contains("left no note"),
            "{take:?}: a lock that is not a month lock's is never registered"
        );
        drop(plain);
    }
}

/// **A WRITER LEAVES ITS NOTE IN THE MONTH LOCK, SO A READER IN ANOTHER
/// PROCESS NAMES IT; THE NOTE IS GONE WHEN THE WRITER LETS GO** (R3-08,
/// RA-298: a test server held a real month busy and no page said who).
///
/// `flock(2)` names no holder across processes. The writer's note does: the
/// test reads the bytes `try_lock_noted` wrote, then plays the other process
/// by writing a note into a lock this process holds through the unregistered
/// constructor, so the registry has nothing and only the file can answer.
#[test]
fn a_writers_note_names_it_to_a_reader_in_another_process() {
    let scratch = Scratch::new("note");
    let pid = std::process::id().to_string();
    let lock = scratch.lock.display().to_string();
    let writer = Flock::try_lock_noted(scratch.open(), scratch.lock.clone()).expect("free");
    let written = fs::read_to_string(&scratch.lock).expect("the lock reads");
    assert!(written.starts_with(super::NOTE_MARK), "{written}");
    for part in [
        format!("pid {pid}"),
        "thread ".to_owned(),
        "ms after the Unix epoch".to_owned(),
        format!("as {lock}"),
    ] {
        assert!(written.contains(&part), "{part}: {written}");
    }
    assert!(written.len() <= super::NOTE_MAX, "{}", written.len());
    drop(writer);
    assert_eq!(
        fs::metadata(&scratch.lock).expect("the lock").len(),
        0,
        "the note is cleared before the lock is let go"
    );

    // ANOTHER PROCESS'S WRITER, as the bytes it leaves: a pid that is not
    // this one, its program and verb, and the lock path IT opened, which
    // names a different store.
    let other = Flock::try_lock(scratch.open(), scratch.lock.clone()).expect("free");
    let theirs = format!(
        "{} pid 4242424, api serve, thread import-worker-3, since 1 ms after the Unix epoch, \
         as /elsewhere/runtime/store/bars/x.lock\n",
        super::NOTE_MARK
    );
    fs::write(&scratch.lock, &theirs).expect("their note");
    let named = super::holder_note(&scratch.lock);
    assert!(named.contains("held by another process"), "{named}");
    for part in [
        "pid 4242424",
        "api serve",
        "import-worker-3",
        "/elsewhere/runtime/store",
    ] {
        assert!(named.contains(part), "{part}: {named}");
    }
    let said = crate::file::StoreError::Locked {
        path: scratch.lock.clone(),
    }
    .to_string();
    assert!(
        said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
        "{said}"
    );

    // A NOTE PAST ITS BOUND is read only to the bound, never whole.
    let huge = format!(
        "{} pid 7, {}\n",
        super::NOTE_MARK,
        "x".repeat(super::NOTE_MAX * 4)
    );
    fs::write(&scratch.lock, &huge).expect("a long note");
    let cut = super::holder_note(&scratch.lock);
    assert!(cut.len() < super::NOTE_MAX + 200, "{}", cut.len());

    // BYTES THAT ARE NOT A NOTE are not read as one.
    fs::write(&scratch.lock, "not a note at all").expect("garbage");
    assert!(super::holder_note(&scratch.lock).contains("left no note"));
    other.release().expect("release");

    // A LOCK FILE THAT IS GONE says why its note did not read.
    let gone = scratch.dir.join("absent.lock");
    assert!(super::holder_note(&gone).contains("did not read"));
}

/// **A RELEASED MONTH LOCK IS THE FILE IT WAS: ITS BYTES AND ITS
/// MODIFICATION TIME** (R3-08). The note is written while the month is held
/// and cleared before it is let go, and the lock file's times are put back
/// as it is cleared, so a write that is refused after taking the lock (the
/// sort-index door's checks) or one that finishes leaves no trace in the
/// month: `crates/store/tests/sort_index.rs` compares every file's bytes,
/// inode and modification time before and after.
#[test]
fn a_released_noted_lock_keeps_its_bytes_and_its_modification_time() {
    let scratch = Scratch::new("times");
    let past = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    scratch
        .open()
        .set_times(fs::FileTimes::new().set_modified(past))
        .expect("an old modification time");
    let before = fs::metadata(&scratch.lock).expect("the lock");
    assert_eq!(before.modified().expect("mtime"), past);
    for release in [true, false] {
        let writer = Flock::try_lock_noted(scratch.open(), scratch.lock.clone()).expect("free");
        assert!(
            fs::read_to_string(&scratch.lock)
                .expect("the lock reads")
                .starts_with(super::NOTE_MARK),
            "noted while held"
        );
        if release {
            writer.release().expect("release");
        } else {
            drop(writer);
        }
        let after = fs::metadata(&scratch.lock).expect("the lock");
        assert_eq!(after.len(), 0, "cleared (release {release})");
        assert_eq!(
            after.modified().expect("mtime"),
            past,
            "the modification time is put back (release {release})"
        );
    }
}

/// **A NOTE A KILLED WRITER LEFT IS SAID TO BE THE LAST WRITER'S, NEVER
/// THE HOLDER'S FOR CERTAIN** (R3-08, killed mid-write). A writer killed
/// while it held the month cannot clear its note; a reader in another process
/// then holds the month and leaves none. A writer refused now must not read
/// the dead writer's pid as today's holder without saying so; the next writer
/// that takes the month overwrites the stale note and clears its own.
#[test]
fn a_note_a_killed_writer_left_is_said_to_be_the_last_writers() {
    let scratch = Scratch::new("stale");
    let dead = format!(
        "{} pid 9191919, api serve, thread import-worker-1, since 1 ms after the Unix epoch, \
         as /gone/store/bars/x.lock\n",
        super::NOTE_MARK
    );
    fs::write(&scratch.lock, &dead).expect("the note a killed writer left");
    // ANOTHER PROCESS'S READER: a shared lock no registry of this process
    // knows about, exactly what a reader elsewhere is to this one.
    let reader = Flock::try_lock_shared(scratch.open(), scratch.lock.clone()).expect("free");
    assert!(
        matches!(
            Flock::try_lock_noted(scratch.open(), scratch.lock.clone()),
            Err(TryLockError::WouldBlock)
        ),
        "a writer is refused while the reader holds the month"
    );
    let named = super::holder_note(&scratch.lock);
    for part in [
        "the note the last writer left",
        "pid 9191919",
        "killed",
        "the holder is a reader",
    ] {
        assert!(named.contains(part), "{part}: {named}");
    }
    assert_eq!(
        fs::read_to_string(&scratch.lock).expect("the lock reads"),
        dead,
        "a refused writer writes nothing"
    );
    reader.release().expect("release");
    let next = Flock::try_lock_noted(scratch.open(), scratch.lock.clone()).expect("free now");
    let note = fs::read_to_string(&scratch.lock).expect("the lock reads");
    assert!(
        note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
        "the next writer's note replaces the stale one: {note}"
    );
    next.release().expect("release");
    assert_eq!(fs::metadata(&scratch.lock).expect("the lock").len(), 0);
}

/// **A NOTE THAT CANNOT BE WRITTEN NEVER COSTS THE WRITE** (R3-08). A lock
/// file opened read-only takes the lock, writes no note, is still named in
/// this process, and leaves the file as it was.
#[test]
fn a_note_that_cannot_be_written_leaves_the_lock_held_and_the_file_untouched() {
    let scratch = Scratch::new("unnoted");
    let read_only = File::open(&scratch.lock).expect("read-only");
    let writer = Flock::try_lock_noted(read_only, scratch.lock.clone()).expect("free");
    assert!(would_block(&scratch.probe()), "the lock is held");
    assert_eq!(fs::metadata(&scratch.lock).expect("the lock").len(), 0);
    assert!(super::holder_note(&scratch.lock).contains("held by this process"));
    writer.release().expect("release");
    scratch.probe().expect("and let go");
}

/// **A READER IN THIS PROCESS IS COUNTED, SO A WRITER IT REFUSES CAN SAY
/// WHO** (R3-08, RA-298, FM-066's naming half): the patient shared lock
/// registers while held, and every reader that ends is forgotten.
#[test]
fn readers_in_this_process_are_counted_while_they_hold_the_month() {
    let scratch = Scratch::new("readers");
    let first =
        Flock::try_lock_shared_patiently(scratch.open(), scratch.lock.clone()).expect("free");
    let second =
        Flock::try_lock_shared_patiently(scratch.open(), scratch.lock.clone()).expect("shared");
    let named = super::holder_note(&scratch.lock);
    assert!(
        named.contains("held by 2 reader(s) in this process"),
        "{named}"
    );
    drop(first);
    assert!(super::holder_note(&scratch.lock).contains("held by 1 reader(s)"));
    second.release().expect("release");
    assert!(super::holder_note(&scratch.lock).contains("left no note"));
    let refused = super::with_patience(std::time::Duration::ZERO, || {
        let writer = Flock::try_lock(scratch.open(), scratch.lock.clone()).expect("free");
        let refused = Flock::try_lock_shared_patiently(scratch.open(), scratch.lock.clone());
        drop(writer);
        refused.map(drop)
    });
    assert!(matches!(refused, Err(TryLockError::WouldBlock)));
    assert!(
        super::holder_note(&scratch.lock).contains("left no note"),
        "a refused reader registers nothing"
    );
}
