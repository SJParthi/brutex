//! The advisory-lock guard: a lock this process holds is released by
//! `File::unlock`, never by closing a descriptor. D-0693.
//!
//! # The rule the standard library does not state
//!
//! `File::lock`, `lock_shared`, `try_lock` and `try_lock_shared` are `flock(2)`
//! locks, and a `flock` lock belongs to the **open file description**, not to
//! the descriptor that took it. Every descriptor that refers to that
//! description holds the same lock, and closing one of them releases nothing
//! while another is still open.
//!
//! On Linux `std::process::Command` duplicates the whole descriptor table into
//! the child, and a `CLOEXEC` descriptor is closed only when the child execs.
//! So a thread that spawns a child while another thread holds a lock leaves a
//! second reference to the locked description in that child until its exec.
//! A holder that releases only by dropping its `File` then leaves the lock
//! alive on the child's reference, and the next `try_lock` or
//! `try_lock_shared` of the same file refuses with `WouldBlock` — in this
//! process or in any other.
//!
//! That is not a hypothesis. CI hit it: the `cli` test
//! `checkpoint_counter_reseeding_and_changed_batch_boundaries_refuse` failed
//! with "checkpoint payload is busy or cannot be read" because the checkpoint
//! writer released its payload lock by drop and the reader's shared lock came
//! straight after. macOS applies `CLOEXEC` inside `posix_spawn` atomically, so
//! the race never reproduces there with a real spawn. `File::try_clone` makes
//! the same second reference deterministically, and every test that proves
//! this module uses it.
//!
//! An explicit `File::unlock` releases the lock on the description itself,
//! however many descriptors still refer to it. [`Flock`] is the one place that
//! call is made: on the success path by [`Flock::release`], which returns the
//! refusal to its caller, and on every other path by `Drop`, which cannot
//! return one and logs it instead.
//!
//! # Why this lives in `store`
//!
//! Every crate that takes a production file lock — `store`, `pull`, `api` and
//! `cli` — already depends on this one, so the guard adds no arrow to the crate
//! graph. `store` also owns the busiest lock user, [`crate::file::BarFile`]'s
//! month lock, and already emits through `telemetry`, which the drop note
//! needs.
//!
//! # What it deliberately does not do
//!
//! It implements no `io::Read` and no `io::Write`. `File::flush` is a no-op, so
//! a delegating `flush` would carry a mutant no test can kill. A caller reads
//! and writes through `Deref`/`DerefMut`, or through a duplicate descriptor
//! taken before the lock (`Flock::lock(file.try_clone()?, path)`) when it must
//! also move or wrap the `File` while the lock is held. That duplicate names the
//! same open file description, so its guard takes and releases the same lock
//! the original handle sees.

use std::borrow::{Borrow, BorrowMut};
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt;
use std::fs::{File, TryLockError};
use std::hash::{Hash as _, Hasher as _};
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The words every refusal of a month that is being written carries, so a
/// reader can tell "ask again" from "damaged" (R3-08, NEW-21, RA-117).
///
/// A page read that met a writer used to be refused at once and the routes
/// called it unreadable — the word a damaged file earns — so a healthy month
/// an import worker held for an instant read on `/db` as broken, and a
/// window's highest and lowest could miss it with nothing saying why. These
/// words are the contract between the store's refusal
/// (`StoreError::Locked`'s sentence) and the server that answers it: the
/// server answers "being written" with `Retry-After`, never "unreadable".
pub const BEING_WRITTEN: &str = "being written, ask again";

/// THE ONE CEILING of every patient lock in this module, shared or exclusive,
/// whatever it asked for: **1 s**. For a shared lock it is the most one
/// [`with_patience`] call may wait in total, across every shared lock it
/// takes; for an exclusive lock ([`Flock::try_lock_patiently`]) it cuts the
/// bound that one call is given. A bound, so a patient reader can never be
/// parked for longer than this by a writer that holds a month for minutes,
/// and a garbage bound cannot park a writer for ever behind a stuck holder
/// (AG-06 reuses it rather than keep a second ceiling).
pub const MAX_PATIENCE: Duration = Duration::from_secs(1);

/// The longest single pause between two attempts of any patient lock, shared
/// or exclusive: **16 ms**. The pauses of the one wait loop ([`patiently`])
/// go 1, 2, 4, 8, 16, 16 … ms, so a lock let go a moment after the first
/// refusal is taken within a millisecond or two, and a long wait costs one
/// `flock` call per 16 ms.
pub const MAX_PAUSE: Duration = Duration::from_millis(16);

thread_local! {
    /// How much waiting the shared locks taken by
    /// [`Flock::try_lock_shared_patiently`] on this thread have LEFT: zero
    /// (refuse at once) unless a caller set it through [`with_patience`], and
    /// spent as it is waited, so one request that opens many held months
    /// waits for all of them together no longer than its patience — never its
    /// patience once per month.
    static PATIENCE: Cell<Duration> = const { Cell::new(Duration::ZERO) };
}

/// Runs `work` with this thread patient for up to `wait` IN TOTAL (at most
/// [`MAX_PATIENCE`]) whenever it takes a shared lock through
/// [`Flock::try_lock_shared_patiently`], and restores the previous patience
/// afterwards, also when `work` panics.
///
/// A budget, not a per-lock timeout: the time a patient lock waits is taken
/// out of it, so a window scan over a hundred months an importer holds waits
/// `wait` once, then every later held month is refused at once by name.
///
/// Opt-in on purpose. The server's page reads run on its blocking pools and
/// are patient there (`api::detail`); the importer, the scrub and the CLI keep
/// the immediate refusal they were written against, so no write path is
/// slowed by a reader's courtesy.
pub fn with_patience<R>(wait: Duration, work: impl FnOnce() -> R) -> R {
    /// Puts the previous patience back when dropped.
    struct Restore(Duration);
    impl Drop for Restore {
        fn drop(&mut self) {
            PATIENCE.with(|patience| patience.set(self.0));
        }
    }
    let _restore = Restore(PATIENCE.with(|patience| patience.replace(wait.min(MAX_PATIENCE))));
    work()
}

/// How much more a shared lock taken on this thread may wait now.
#[must_use]
pub fn patience() -> Duration {
    PATIENCE.with(Cell::get)
}

/// Takes `waited` out of this thread's patience.
fn spend_patience(waited: Duration) {
    PATIENCE.with(|patience| patience.set(patience.get().saturating_sub(waited)));
}

/// The first words of the note a month's writer leaves in the lock file it
/// holds ([`Flock::try_lock_noted`]), so a reader can tell a note from any
/// other bytes.
pub const NOTE_MARK: &str = "brutex month lock held:";

/// The most bytes of a holder's note a refused reader reads back. A note is
/// one line of about two hundred bytes; anything longer is cut, never read
/// whole.
pub const NOTE_MAX: usize = 512;

/// The exclusive holder of a lock in THIS process.
#[derive(Debug, Clone)]
struct Holder {
    /// The thread that took it, by name, or its id when it has none.
    thread: String,
    /// When it was taken, in whole milliseconds after the Unix epoch.
    since_unix_ms: u128,
    /// What the holder is doing, when its taker said
    /// ([`Flock::try_lock_patiently`]: "a pull audit journal append", "the
    /// journal index build"); `None` for a month's writer, whose lock path
    /// already says what it writes.
    doing: Option<&'static str>,
}

/// What THIS process holds on one lock path, and who waits for it: its
/// writer, if any, how many readers, and the queue of patient exclusive
/// lockers ([`Flock::try_lock_patiently`]) while one is outstanding. One
/// registry for all three, so a holder and its waiters are one entry (AG-06).
#[derive(Debug, Default)]
struct Held {
    writer: Option<Holder>,
    readers: u32,
    queue: Option<std::sync::Arc<Turns>>,
}

impl Held {
    /// Whether the entry records nothing any more and may be removed.
    fn is_idle(&self) -> bool {
        self.writer.is_none() && self.readers == 0 && self.queue.is_none()
    }
}

/// Which side of a lock a guard registered, so the guard's end forgets
/// exactly what its start recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Writer,
    Reader,
}

/// Every lock this process holds through [`Flock::try_lock_noted`] (month
/// writers), [`Flock::try_lock_shared_patiently`] (month readers) or
/// [`Flock::try_lock_patiently`] (patient exclusive writers, the pull audit
/// journal's appends among them), and every patient exclusive locker's queue,
/// keyed by a hash of the lock's path (no allocation per probe). One lock and
/// one hash probe when a guard starts and one when it ends: O(1) expected per
/// acquisition. Only those three constructors register, so the results
/// files' and every other lock's path pays nothing.
///
/// The registry and the kernel's lock move TOGETHER for every reader of this
/// map: a guard's `flock(2)` grant and its record are one step under this
/// lock ([`take_held`]), and so are its forgetting and its unlock
/// ([`let_go`]). So while a guard of this process holds the lock, this map
/// names it, at every instant a reader here can see — through every hand-off
/// — and it never names one that has let go; and a holder's end can never
/// erase its successor's entry (AG-06 repair 1: a registry kept beside the
/// guard erased it; repair 2: the record followed the grant and preceded the
/// unlock, and a reader in between read "another process"). Both `flock`
/// calls made under this lock are non-blocking. The bookkeeping is proved by
/// `crate::flock::tests::readers_in_this_process_are_counted_while_they_hold_the_month`,
/// `crate::flock::patient_tests::a_successors_name_survives_its_predecessors_release`
/// and `crate::flock::patient_tests::no_reader_here_ever_finds_the_lock_held_and_its_holder_unnamed`;
/// its cost apart from the acquisition is measured by AG-06 within the
/// journal append (`docs/06-limits.md` L-AG06-10) and by no bench alone
/// (L-R308-02).
static HOLDERS: Mutex<Option<HashMap<u64, Held>>> = Mutex::new(None);

/// The registry, whoever panicked while holding it: every update is one
/// statement on whole counters, so the poison says only that unrelated code
/// panicked.
fn registry() -> std::sync::MutexGuard<'static, Option<HashMap<u64, Held>>> {
    HOLDERS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The registry key of a lock path.
fn key_of(path: &Path) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

/// The thread asking, by name, or its id when it has none.
fn this_thread() -> String {
    let current = std::thread::current();
    current
        .name()
        .map_or_else(|| format!("{:?}", current.id()), str::to_owned)
}

/// Milliseconds after the Unix epoch now; zero on a clock set before it.
fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis())
}

/// Asks for the lock ONCE through `attempt` (a non-blocking `flock(2)` call)
/// and, when it is granted, records that this thread holds `key` on `side`,
/// doing `doing` when it is a writer that said — AS ONE STEP: the registry's
/// lock is held across the attempt and the record. So no reader of the
/// registry in this process ([`holder_note`]) can find the lock granted to
/// this process and its holder not yet named (AG-06 repair 2: the record used
/// to follow the grant, and a refusal read in between said "another
/// process"). The attempt never blocks, so the registry is held for one
/// system call; a refused attempt records nothing.
fn take_held(
    key: u64,
    side: Side,
    doing: Option<&'static str>,
    attempt: impl FnOnce() -> Result<(), TryLockError>,
) -> Result<(), TryLockError> {
    // The thread's name is read before the registry is locked, so the lock
    // covers only the attempt and the record.
    let thread = (side == Side::Writer).then(this_thread);
    let mut held = registry();
    attempt()?;
    let entry = held
        .get_or_insert_with(HashMap::new)
        .entry(key)
        .or_default();
    match thread {
        Some(thread) => {
            entry.writer = Some(Holder {
                thread,
                since_unix_ms: now_unix_ms(),
                doing,
            });
        }
        None => entry.readers = entry.readers.saturating_add(1),
    }
    drop(held);
    Ok(())
}

/// Forgets what [`take_held`] recorded under `key` for `side` and then lets
/// the lock go through `unlock` — AS ONE STEP, under the registry's lock, so
/// no reader of the registry in this process can find the lock still held by
/// this process and its holder already forgotten (AG-06 repair 2). An entry
/// that holds nothing more, and has no patient locker waiting, is removed, so
/// the map is bounded by the locks held or awaited now. Answers the unlock's
/// result; `unlock` never blocks, so the registry is held for one system
/// call.
fn let_go(key: u64, side: Side, unlock: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
    let mut held = registry();
    forget_in(&mut held, key, side);
    let unlocked = unlock();
    drop(held);
    unlocked
}

/// The registry half of [`let_go`].
fn forget_in(held: &mut Option<HashMap<u64, Held>>, key: u64, side: Side) {
    let Some(holders) = held.as_mut() else {
        return;
    };
    let empty = holders.get_mut(&key).is_some_and(|entry| {
        match side {
            Side::Writer => entry.writer = None,
            Side::Reader => entry.readers = entry.readers.saturating_sub(1),
        }
        entry.is_idle()
    });
    if empty {
        holders.remove(&key);
    }
}

/// The note a writer leaves in the month lock it holds: who it is (pid, the
/// program and its verb, the thread), since when, and the lock by the path it
/// opened it at — which names the store it writes (R3-08; rules-audit RA-298).
fn note_of(path: &Path) -> String {
    static PROCESS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let process = PROCESS.get_or_init(|| {
        let mut args = std::env::args_os();
        let program = args
            .next()
            .map(|program| {
                Path::new(&program)
                    .file_name()
                    .map_or_else(|| program.clone(), std::ffi::OsStr::to_os_string)
            })
            .map_or_else(
                || "an unnamed program".to_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
        // The verb only: the rest of a command line can be long, and no
        // argument past the verb is needed to say which run holds a month.
        let verb = args
            .next()
            .map(|verb| verb.to_string_lossy().chars().take(48).collect::<String>())
            .unwrap_or_default();
        if verb.is_empty() {
            program
        } else {
            format!("{program} {verb}")
        }
    });
    format!(
        "{NOTE_MARK} pid {}, {process}, thread {}, since {} ms after the Unix epoch, as {}\n",
        std::process::id(),
        this_thread(),
        now_unix_ms(),
        path.display()
    )
}

/// The note another process's writer left in the lock at `path`, read back
/// through one non-blocking, bounded read; `None` when the file holds none.
fn note_in(path: &Path) -> Result<Option<String>, String> {
    use std::io::Read as _;
    let file = crate::safe_open::open_read(path).map_err(|refusal| refusal.to_string())?;
    let mut bytes = Vec::with_capacity(NOTE_MAX);
    file.take(NOTE_MAX as u64)
        .read_to_end(&mut bytes)
        .map_err(|why| why.to_string())?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(text
        .strip_prefix(NOTE_MARK)
        .map(|rest| rest.lines().next().unwrap_or("").trim().to_owned()))
}

/// Who holds the lock at `path`, in words a page can show (R3-08;
/// rules-audit RA-298; AG-06 for the patient exclusive locks):
///
/// - a writer in THIS process: the pid, its thread, what it is doing when its
///   taker said so ([`Flock::try_lock_patiently`]), and since when;
/// - readers in this process: how many;
/// - a writer in ANOTHER process that left its note in the lock file
///   ([`Flock::try_lock_noted`]): the note — its pid, program and verb,
///   thread, since when, and the lock path it writes, which names its store;
/// - otherwise: that another process holds it and left no note — a reader
///   (readers write nothing), or a writer that cannot (an older build), since
///   `flock(2)` itself names no holder.
///
/// Read when the sentence is made, so a holder that let go a moment before
/// can read as the next case down; never invented. Deterministic for one
/// hold, so the same month refused twice in one answer reads as one sentence
/// and is named once (`api::fault_log::push_once`).
#[must_use]
pub fn holder_note(path: &Path) -> String {
    let pid = std::process::id();
    let (writer, readers) = registry()
        .as_ref()
        .and_then(|holders| holders.get(&key_of(path)))
        .map_or((None, 0), |held| (held.writer.clone(), held.readers));
    if let Some(holder) = writer {
        let doing = holder
            .doing
            .map_or_else(String::new, |doing| format!(", doing {doing}"));
        return format!(
            "held by this process (pid {pid}, thread {}{doing}) since {} ms after the Unix \
             epoch, lock {}",
            holder.thread,
            holder.since_unix_ms,
            path.display()
        );
    }
    if readers > 0 {
        return format!(
            "held by {readers} reader(s) in this process (pid {pid}), lock {}",
            path.display()
        );
    }
    match note_in(path) {
        // THE LAST WRITER'S NOTE, SAID AS THAT (killed mid-write): a writer
        // clears its note before it lets go, but one killed while it held the
        // month cannot, and its note stays until the next writer takes the
        // month. Readers write none. So the note names the holder unless that
        // writer has since ended and readers, or a writer that writes no note,
        // hold the month now — said, never left for the reader to assume.
        Ok(Some(note)) => format!(
            "held by another process; the note the last writer left in the lock reads: {note} \
             (a killed writer leaves its note behind: if that pid has ended, the holder is a \
             reader, which leaves none, or a writer that writes none)"
        ),
        Ok(None) => format!(
            "held by another process that left no note in the lock: a reader (readers write \
             nothing) or a writer that writes none; flock(2) does not name its holder, and this \
             process (pid {pid}) holds nothing on {}",
            path.display()
        ),
        Err(why) => format!(
            "held by another process; its note in {} did not read: {why}",
            path.display()
        ),
    }
}

/// A `flock(2)` lock this process holds on one open file description.
///
/// It is released by `File::unlock`, never by close, so a duplicate descriptor
/// left in a child — or made by `try_clone` — cannot keep it alive.
///
/// `F` is the file, owned (`File`) or borrowed (`&File`, `&mut File`). `P` is
/// the path the lock is named by in a refusal: owned by default, borrowed
/// (`&Path`) where a per-call guard should allocate nothing.
#[derive(Debug)]
#[must_use = "dropping a Flock unlocks immediately"]
pub struct Flock<F: Borrow<File>, P: AsRef<Path> = PathBuf> {
    /// The file whose open file description holds the lock.
    file: F,
    /// The path a refusal to release names.
    path: P,
    /// Whether `Drop` still owes the unlock. Cleared as the guard ends
    /// (`Flock::end`, which [`Flock::release`] calls first), before it
    /// unlocks, so a refused release is reported once, by the release, and
    /// never again by `Drop`.
    held: bool,
    /// The holder registry's key and side while this guard holds a lock
    /// taken by [`Flock::try_lock_noted`], [`Flock::try_lock_shared_patiently`]
    /// or [`Flock::try_lock_patiently`], so a refused party can name it
    /// ([`holder_note`]); `None` for every other lock, which is never
    /// registered.
    holding: Option<(u64, Side)>,
    /// `Some` when this guard wrote its holder's note into the lock file
    /// ([`Flock::try_lock_noted`]) and owes it cleared before it unlocks: the
    /// lock file's times from before the note, put back as it is cleared, so
    /// a released month lock is the file it was — its bytes and its
    /// modification time — and a check that a refused or finished write
    /// touched nothing in the month still holds (R3-08).
    noted: Option<std::fs::FileTimes>,
    /// This process's turn for the lock's path when the guard was taken by
    /// [`Flock::try_lock_patiently`]: handed to the next patient locker only
    /// AFTER the unlock, because a field drops after `Drop::drop` (and after
    /// [`Flock::release`]'s unlock) has run (AG-06). `None` for every other
    /// lock.
    _turn: Option<Turn>,
}

impl<F: Borrow<File>, P: AsRef<Path>> Flock<F, P> {
    /// Blocks until the exclusive lock is taken.
    ///
    /// # Errors
    ///
    /// The host's refusal, exactly as `File::lock` returns it.
    pub fn lock(file: F, path: P) -> io::Result<Self> {
        file.borrow().lock()?;
        Ok(Self::taken(file, path))
    }

    /// Blocks until a shared lock is taken.
    ///
    /// # Errors
    ///
    /// The host's refusal, exactly as `File::lock_shared` returns it.
    pub fn lock_shared(file: F, path: P) -> io::Result<Self> {
        file.borrow().lock_shared()?;
        Ok(Self::taken(file, path))
    }

    /// Takes the exclusive lock, or refuses without waiting.
    ///
    /// # Errors
    ///
    /// `TryLockError::WouldBlock` while another description holds the lock, or
    /// the host's refusal, exactly as `File::try_lock` returns them.
    pub fn try_lock(file: F, path: P) -> Result<Self, TryLockError> {
        file.borrow().try_lock()?;
        Ok(Self::taken(file, path))
    }

    /// [`Self::try_lock`] for a MONTH's `.lock` sibling, which holds no data:
    /// the holder is registered in this process and leaves its note in the
    /// lock file — pid, program and verb, thread, since when, and the lock by
    /// the path it opened, which names its store — so a reader it refuses, in
    /// this process or another, names who holds the month (R3-08; rules-audit
    /// RA-298). The note is cleared before the lock is let go.
    ///
    /// Only for a lock file that is nothing but a lock: the note truncates and
    /// rewrites the file it is given. A note that cannot be written (a file
    /// opened read-only, a refused write) leaves the lock held and the month
    /// writable — the note is a courtesy to readers, never a condition of the
    /// write — and is said once per process on `store.flock`, so a reader that
    /// then reads "left no note" is not the only place it shows.
    ///
    /// One `fstat` and two `pwrite`-class calls on taking, and a truncate and
    /// one `futimens` on letting go: O(1) per acquisition. What the note holds,
    /// and that letting go keeps the file's bytes and time, is proved by
    /// `crate::flock::tests::a_released_noted_lock_keeps_its_bytes_and_its_modification_time`;
    /// the cost is UNVERIFIED: no bench times a noted acquisition
    /// (`docs/06-limits.md` L-R308-02).
    ///
    /// # Errors
    ///
    /// As [`Self::try_lock`].
    pub fn try_lock_noted(file: F, path: P) -> Result<Self, TryLockError> {
        use std::os::unix::fs::FileExt as _;
        let key = key_of(path.as_ref());
        take_held(key, Side::Writer, None, || file.borrow().try_lock())?;
        // THE TIMES FROM BEFORE THE NOTE, to put back as it is cleared; a
        // file whose times do not read keeps whatever the clear leaves.
        let times = file.borrow().metadata().map_or_else(
            |_| std::fs::FileTimes::new(),
            |meta| {
                let mut times = std::fs::FileTimes::new();
                if let Ok(modified) = meta.modified() {
                    times = times.set_modified(modified);
                }
                if let Ok(accessed) = meta.accessed() {
                    times = times.set_accessed(accessed);
                }
                times
            },
        );
        let note = note_of(path.as_ref());
        let written = file
            .borrow()
            .set_len(0)
            .and_then(|()| file.borrow().write_all_at(note.as_bytes(), 0));
        if let Err(why) = &written {
            note_unnoted(path.as_ref(), why);
        }
        Ok(Self {
            file,
            path,
            held: true,
            holding: Some((key, Side::Writer)),
            noted: written.is_ok().then_some(times),
            _turn: None,
        })
    }

    /// Takes a shared lock, or refuses without waiting.
    ///
    /// # Errors
    ///
    /// `TryLockError::WouldBlock` while another description holds the lock
    /// exclusively, or the host's refusal, exactly as `File::try_lock_shared`
    /// returns them.
    pub fn try_lock_shared(file: F, path: P) -> Result<Self, TryLockError> {
        file.borrow().try_lock_shared()?;
        Ok(Self::taken(file, path))
    }

    /// Takes a shared lock on a MONTH's `.lock` at once when no writer holds
    /// it; when one does, asks again for up to this thread's [`patience`]
    /// (zero unless a caller set it with [`with_patience`]), pausing 1, 2, 4
    /// … up to 16 ms between attempts, then refuses (R3-08, NEW-21). The time
    /// it waited is taken out of the thread's patience, so the months one
    /// request opens share one budget. The reader is counted in this
    /// process's registry while it holds the month, so a writer it refuses
    /// can say a reader here holds it ([`holder_note`]).
    ///
    /// With no patience it is exactly [`Self::try_lock_shared`]: one attempt.
    /// The wait is bounded by [`MAX_PATIENCE`] whatever was asked, and it
    /// polls rather than blocks in `flock(2)`, because a blocking shared lock
    /// cannot be given a deadline. The polling is [`patiently`], the one wait
    /// loop the exclusive [`Self::try_lock_patiently`] uses too (AG-06).
    ///
    /// # Errors
    ///
    /// `TryLockError::WouldBlock` when a writer still holds the lock as the
    /// patience runs out, or the host's refusal at once.
    pub fn try_lock_shared_patiently(file: F, path: P) -> Result<Self, TryLockError> {
        let started = Instant::now();
        let key = key_of(path.as_ref());
        let taken = patiently(patience(), Instant::now, std::thread::sleep, || {
            take_held(key, Side::Reader, None, || file.borrow().try_lock_shared())
        });
        spend_patience(started.elapsed());
        taken.map_err(|refused| match refused {
            Impatient::Held { .. } => TryLockError::WouldBlock,
            Impatient::Host(why) => TryLockError::Error(why),
        })?;
        Ok(Self {
            file,
            path,
            held: true,
            holding: Some((key, Side::Reader)),
            noted: None,
            _turn: None,
        })
    }

    /// Releases the lock on the success path, and says so if it cannot.
    ///
    /// `Drop` performs the same unlock on every other path, but it has nowhere
    /// to send a refusal except the log. A caller that is about to report
    /// success calls this instead, so a lock that could not be released turns
    /// the success into a named failure rather than into a line nobody reads.
    ///
    /// # Errors
    ///
    /// [`Unreleased`], naming the lock's path and the host's refusal. The guard
    /// is consumed either way and `Drop` does not try again.
    pub fn release(mut self) -> Result<(), Unreleased> {
        match self.end() {
            Some(Err(why)) => Err(Unreleased {
                path: self.path.as_ref().to_path_buf(),
                why,
            }),
            Some(Ok(())) | None => Ok(()),
        }
    }

    /// The guard for a lock the caller has just taken, unregistered.
    const fn taken(file: F, path: P) -> Self {
        Self {
            file,
            path,
            held: true,
            holding: None,
            noted: None,
            _turn: None,
        }
    }

    /// Every way a guard ends: clears the holder's note while the lock is
    /// still held, then forgets the registration AND unlocks as one step
    /// under the registry's lock ([`let_go`]), so no reader of the registry
    /// in this process finds the lock held here with its holder forgotten
    /// (AG-06 repair 2). Runs once, whichever end comes first: answers the
    /// unlock's result, or `None` when the guard has already ended.
    fn end(&mut self) -> Option<io::Result<()>> {
        if let Some(times) = self.noted.take() {
            let file = self.file.borrow();
            if let Err(why) = file.set_len(0).and_then(|()| file.set_times(times)) {
                note_unnoted(self.path.as_ref(), &why);
            }
        }
        if !std::mem::replace(&mut self.held, false) {
            return None;
        }
        let file = self.file.borrow();
        Some(match self.holding.take() {
            Some((key, side)) => let_go(key, side, || unlock(file)),
            None => unlock(file),
        })
    }
}

impl<F: Borrow<File>, P: AsRef<Path>> Deref for Flock<F, P> {
    type Target = File;

    fn deref(&self) -> &File {
        self.file.borrow()
    }
}

impl<F: BorrowMut<File>, P: AsRef<Path>> DerefMut for Flock<F, P> {
    fn deref_mut(&mut self) -> &mut File {
        self.file.borrow_mut()
    }
}

impl<F: Borrow<File>, P: AsRef<Path>> Drop for Flock<F, P> {
    /// Every path that did not call [`Flock::release`]: an early return, a
    /// refusal, an owner that dropped. The unlock is explicit here for the
    /// reason the module header gives, and a refusal is logged because a drop
    /// cannot return one.
    fn drop(&mut self) {
        if let Some(Err(why)) = self.end() {
            note_unreleased(self.path.as_ref(), &why);
        }
    }
}

/// A lock whose unlock the host refused.
///
/// Returned by [`Flock::release`]. The lock may still be held by the open file
/// description, so a later attempt to take it can refuse until that
/// description is unlocked again or every descriptor referring to it has
/// closed. For a guard that owned its `File`, those descriptors are the
/// guard's own, which closes as the guard is consumed, and any duplicate of
/// it. For a guard over a borrowed file, the guard closes nothing: the owner's
/// descriptor is one of them, so the lock can outlast the guard while the
/// owner stays open, until a later guard over the same owner releases it or
/// the owner closes.
#[derive(Debug)]
pub struct Unreleased {
    /// The file the lock was taken on.
    pub path: PathBuf,
    /// The host's refusal to unlock it.
    pub why: io::Error,
}

impl fmt::Display for Unreleased {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the advisory lock on {} could not be released, and it may stay held until its \
             open file description is unlocked again or every descriptor of it has closed: {}",
            self.path.display(),
            self.why
        )
    }
}

impl std::error::Error for Unreleased {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.why)
    }
}

impl From<Unreleased> for io::Error {
    /// Keeps the host's kind, and carries the whole sentence so the path
    /// survives into a caller that reports only an `io::Error`.
    fn from(unreleased: Unreleased) -> Self {
        Self::new(unreleased.why.kind(), unreleased.to_string())
    }
}

/// The one `File::unlock` this crate makes on a guard's behalf.
///
/// Under `cfg(test)` a one-shot, per-thread refusal of a chosen kind can be
/// armed first, because no real file refuses an unlock on request and the
/// refusal arms of [`Flock::release`] and `Drop`, and of the callers in this
/// crate that turn a refused release into their own error, must still be
/// driven.
fn unlock(file: &File) -> io::Result<()> {
    #[cfg(test)]
    if let Some(kind) = tests::take_unlock_refusal() {
        return Err(io::Error::new(
            kind,
            "unlock refused by the flock test seam",
        ));
    }
    file.unlock()
}

/// A holder's note that could not be written or cleared
/// ([`Flock::try_lock_noted`]): said ONCE per process on `store.flock`, so a
/// lock file that refuses every note is one line, never one per month.
fn note_unnoted(path: &Path, why: &io::Error) {
    static SAID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if SAID.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(
            "store.flock",
            "a month lock's holder note was not written or cleared (said once per process); \
             a reader it refuses says the holder left no note, or reads a stale one",
        )
        .with("file", telemetry::Value::Str(&path.display().to_string()))
        .with("why", telemetry::Value::Str(&why.to_string())),
    );
}

/// A guard dropped without a release, whose unlock the host refused.
///
/// `Warn`, not `Error`: the kernel still frees the lock once every descriptor
/// of its open file description has closed. For a guard that owned its `File`
/// that is its own descriptor, which closes as the guard drops, and any
/// duplicate of it. For a guard over a borrowed file the drop closes nothing,
/// and the owner's descriptor can hold the lock while the owner stays open,
/// until a later guard over the same owner releases it or the owner closes.
/// The line records a refused unlock; the lock may outlive the guard while a
/// borrowed owner or a duplicate descriptor remains open. Without either,
/// the owned file's last close frees the lock as the guard drops. One event
/// per refused unlock, never per acquisition, so the normal
/// path costs nothing beyond the `flock` call itself.
fn note_unreleased(path: &Path, why: &io::Error) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("store.flock", "advisory lock not released by its guard")
            .with("file", telemetry::Value::Str(&path.display().to_string()))
            .with("why", telemetry::Value::Str(&why.to_string())),
    );
}

// ---------------------------------------------------------------------------
// THE PATIENT EXCLUSIVE LOCK (AG-06, code-5, FM-162; D-6244).
//
// A writer whose lock another writer holds for one short append used to be
// refused at once (`try_lock`), and the record it carried was lost: the pull
// audit journal's opening record of a run, so the restart scan later answered
// "clean" for a run that ran and died. This waits a BOUNDED time instead and
// refuses by name when the bound runs out. It is R3-08's patience made
// exclusive, not a second mechanism: the one wait loop ([`patiently`], which
// the patient shared lock calls too), the one pause ([`MAX_PAUSE`]), the one
// ceiling ([`MAX_PATIENCE`]) and the one registry of this process's holders
// and waiters ([`HOLDERS`], read by [`holder_note`]).
// ---------------------------------------------------------------------------

/// Why a patient lock was not taken.
#[derive(Debug)]
pub enum Impatient {
    /// Another holder kept the lock for the whole wait.
    Held {
        /// The bound the wait was given, after [`MAX_PATIENCE`] cut it.
        bound: Duration,
        /// How long this call waited, by the monotonic clock or by its own
        /// slept pauses, whichever is longer.
        waited: Duration,
        /// How many times the lock was asked for, the first included. Zero
        /// when the bound ran out while patient lockers ahead of this one in
        /// this process still held or awaited their turn.
        attempts: u32,
        /// How many patient lockers of the same path were ahead of this one
        /// in this process when it asked.
        ahead: u64,
        /// Who held it as the bound ran out, in [`holder_note`]'s words, when
        /// the refusal knows the lock's path ([`Flock::try_lock_patiently`]);
        /// `None` from the bare loop ([`patiently`]), which knows no path.
        holder: Option<String>,
    },
    /// The host refused the lock for a reason other than another holder, at
    /// the first attempt that met it. Never waited on: it is not contention.
    Host(io::Error),
}

impl fmt::Display for Impatient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Held {
                bound,
                waited,
                attempts,
                ahead,
                holder,
            } => {
                write!(
                    f,
                    "still held after waiting {} ms of a {} ms bound ({attempts} attempt(s), \
                     {ahead} ahead of it in this process, pausing 1 ms doubling to {} ms)",
                    waited.as_millis(),
                    bound.as_millis(),
                    MAX_PAUSE.as_millis(),
                )?;
                match holder {
                    Some(holder) => write!(f, "; {holder}"),
                    None => Ok(()),
                }
            }
            Self::Host(why) => write!(f, "the host refused the lock: {why}"),
        }
    }
}

impl std::error::Error for Impatient {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Held { .. } => None,
            Self::Host(why) => Some(why),
        }
    }
}

impl<F: Borrow<File>, P: AsRef<Path>> Flock<F, P> {
    /// Takes the exclusive lock at once when nobody holds it; when another
    /// holder does, waits for up to `bound` (cut to [`MAX_PATIENCE`]), then
    /// refuses naming the bound, the wait, the attempts, how many lockers of
    /// this path were ahead of it in this process, and who holds it
    /// ([`holder_note`]). While it holds the lock the holder is registered in
    /// this process, `doing` what its caller said ("a pull audit journal
    /// append"), so a locker it refuses names it.
    ///
    /// # Two waits, one bound
    ///
    /// **In this process, a queue.** Patient lockers of one path take turns
    /// in the order they asked (a ticket each in the path's [`HOLDERS`]
    /// entry, woken by a condition variable when the one ahead lets go). A
    /// poller alone starves under sustained contention: the lock is free only
    /// for the microseconds between one holder's release and the next one's
    /// take, and a writer appending in a loop takes it again before a
    /// sleeping poller wakes. Measured: 8 threads × 100 journal appends
    /// refused 102 of 800 with a 1 s bound before the queue, none after it
    /// (AG-06, `docs/06-limits.md`).
    ///
    /// **Against another process, a poll.** The one at the head of the queue
    /// asks `flock(2)` without blocking through [`patiently`], the loop the
    /// patient shared lock uses, because a blocking lock cannot be given a
    /// deadline. A holder killed while it holds the lock is no hang: the
    /// kernel frees a dead process's locks and the next attempt takes it.
    ///
    /// A zero bound is one attempt when nobody in this process is ahead, and
    /// a refusal at once when someone is.
    ///
    /// # Cost
    ///
    /// Uncontended: one registry lock and hash probe for the ticket, one
    /// `flock` call, one for the holder, and one each as it ends. Under
    /// contention the time spent is at most the bound plus one attempt,
    /// whatever the queue's length; a waiter is woken once per holder ahead
    /// of it.
    ///
    /// # Errors
    ///
    /// [`Impatient::Held`] when the lock was still held, or this process's
    /// queue still ahead, as the bound ran out; [`Impatient::Host`] at once
    /// for any other refusal.
    pub fn try_lock_patiently(
        file: F,
        path: P,
        bound: Duration,
        doing: &'static str,
    ) -> Result<Self, Impatient> {
        let bound = bound.min(MAX_PATIENCE);
        let started = Instant::now();
        let (turn, ahead) = Turn::wait(path.as_ref(), bound)?;
        let left = bound.saturating_sub(started.elapsed());
        let key = key_of(path.as_ref());
        // GRANTED AND NAMED IN ONE STEP ([`take_held`]), and forgotten and
        // unlocked in one step ([`let_go`]), so no reader of the registry in
        // this process finds the lock held here and its holder unnamed — not
        // even at the instant of a hand-off (AG-06 repair 2) — and a
        // predecessor's end cannot erase this entry: it ended before this lock
        // was granted.
        patiently(left, Instant::now, std::thread::sleep, || {
            take_held(key, Side::Writer, Some(doing), || file.borrow().try_lock())
        })
        .map_err(|refused| match refused {
            Impatient::Held { attempts, .. } => Impatient::Held {
                bound,
                waited: started.elapsed(),
                attempts,
                ahead,
                holder: Some(holder_note(path.as_ref())),
            },
            host @ Impatient::Host(_) => host,
        })?;
        Ok(Self {
            file,
            path,
            held: true,
            holding: Some((key, Side::Writer)),
            noted: None,
            _turn: Some(turn),
        })
    }
}

/// One lock path's queue of patient exclusive lockers in this process, kept
/// in the path's [`HOLDERS`] entry while a ticket of it is outstanding.
#[derive(Debug, Default)]
struct Turns {
    /// The tickets: the next to give out, the one now served, and those
    /// whose waiters gave up before their turn.
    state: Mutex<TurnState>,
    /// Signalled whenever `serving` moves.
    moved: std::sync::Condvar,
}

/// The ticket counters of one [`Turns`].
#[derive(Debug, Default)]
struct TurnState {
    /// The ticket the next locker to ask is given.
    next: u64,
    /// The ticket whose holder may take the lock now.
    serving: u64,
    /// Tickets given up before they were served, skipped when reached.
    gone: std::collections::BTreeSet<u64>,
}

/// A queue's counters, whoever panicked while holding them: every update is
/// one statement, so the poison says only that some unrelated code panicked.
fn whole<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One ticket of one path's queue, handed on when dropped.
#[derive(Debug)]
struct Turn {
    /// The registry key of the path the queue is for.
    key: u64,
    /// The queue.
    turns: std::sync::Arc<Turns>,
    /// This locker's ticket.
    ticket: u64,
}

impl Turn {
    /// Takes a ticket for `path` and waits until it is served, for up to
    /// `bound`; answers the turn and how many were ahead when it asked.
    fn wait(path: &Path, bound: Duration) -> Result<(Self, u64), Impatient> {
        let started = Instant::now();
        let key = key_of(path);
        let (turns, ticket, ahead) = {
            let mut held = registry();
            let entry = held
                .get_or_insert_with(HashMap::new)
                .entry(key)
                .or_default();
            let turns = std::sync::Arc::clone(entry.queue.get_or_insert_with(Default::default));
            let mut state = whole(&turns.state);
            let ticket = state.next;
            state.next = state.next.saturating_add(1);
            let ahead = ticket.saturating_sub(state.serving);
            drop(state);
            drop(held);
            (turns, ticket, ahead)
        };
        let turn = Self { key, turns, ticket };
        let state = whole(&turn.turns.state);
        let (state, _timed_out) = turn
            .turns
            .moved
            .wait_timeout_while(state, bound, |state| state.serving != ticket)
            .unwrap_or_else(PoisonError::into_inner);
        let served = state.serving == ticket;
        drop(state);
        if served {
            return Ok((turn, ahead));
        }
        // Dropping `turn` gives the ticket up (or hands it on, if it was
        // served in between).
        drop(turn);
        Err(Impatient::Held {
            bound,
            waited: started.elapsed(),
            attempts: 0,
            ahead,
            holder: Some(holder_note(path)),
        })
    }
}

impl Drop for Turn {
    fn drop(&mut self) {
        let mut held = registry();
        let mut state = whole(&self.turns.state);
        if state.serving == self.ticket {
            state.serving = state.serving.saturating_add(1);
            loop {
                let serving = state.serving;
                if !state.gone.remove(&serving) {
                    break;
                }
                state.serving = serving.saturating_add(1);
            }
            self.turns.moved.notify_all();
        } else {
            state.gone.insert(self.ticket);
        }
        let idle = state.serving == state.next;
        drop(state);
        // AN EMPTY QUEUE IS FORGOTTEN, and its entry with it once nothing
        // else is recorded there. No ticket can be taken meanwhile: taking
        // one needs the registry lock this holds.
        if idle && let Some(holders) = held.as_mut() {
            let empty = holders.get_mut(&self.key).is_some_and(|entry| {
                entry.queue = None;
                entry.is_idle()
            });
            if empty {
                holders.remove(&self.key);
            }
        }
    }
}

/// THE ONE WAIT LOOP of every patient lock in this module, shared
/// ([`Flock::try_lock_shared_patiently`]) and exclusive
/// ([`Flock::try_lock_patiently`]). `attempt` asks for the lock once; `now`
/// and `sleep` are the clock and the pause, passed in so a test can step the
/// clock backwards or count the pauses without sleeping.
///
/// The bound is cut to [`MAX_PATIENCE`]. The pauses go 1 ms doubling to
/// [`MAX_PAUSE`]. It stops when the wait reaches the bound, measured as the
/// LONGER of the clock's elapsed time and the sum of the pauses it slept, so
/// a clock that steps backwards (or stands still) cannot extend the wait:
/// every pause adds to the sum, and the last pause is cut to what is left of
/// the bound. A host refusal that is not contention returns at once.
pub(crate) fn patiently(
    bound: Duration,
    mut now: impl FnMut() -> Instant,
    mut sleep: impl FnMut(Duration),
    mut attempt: impl FnMut() -> Result<(), TryLockError>,
) -> Result<(), Impatient> {
    let bound = bound.min(MAX_PATIENCE);
    let started = now();
    let mut slept = Duration::ZERO;
    let mut pause = Duration::from_millis(1);
    let mut attempts = 0_u32;
    loop {
        attempts = attempts.saturating_add(1);
        match attempt() {
            Ok(()) => return Ok(()),
            Err(TryLockError::Error(why)) => return Err(Impatient::Host(why)),
            Err(TryLockError::WouldBlock) => {
                let waited = now().saturating_duration_since(started).max(slept);
                let left = bound.saturating_sub(waited);
                if left.is_zero() {
                    return Err(Impatient::Held {
                        bound,
                        waited,
                        attempts,
                        ahead: 0,
                        holder: None,
                    });
                }
                let step = pause.min(left);
                sleep(step);
                slept = slept.saturating_add(step);
                pause = pause.saturating_mul(2).min(MAX_PAUSE);
            }
        }
    }
}

#[cfg(test)]
#[path = "flock_patient_tests.rs"]
mod patient_tests;

#[cfg(test)]
#[path = "flock_tests.rs"]
pub(crate) mod tests;
