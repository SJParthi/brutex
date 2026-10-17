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

/// The longest any thread may be patient, in total, across every shared lock
/// one [`with_patience`] call takes, whatever it asked for. A bound, so a
/// patient reader can never be parked for longer than this by a writer that
/// holds a month for minutes.
pub const MAX_PATIENCE: Duration = Duration::from_secs(1);

/// The longest single pause between two attempts of a patient reader.
const MAX_PAUSE: Duration = Duration::from_millis(16);

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

/// The exclusive holder of a month lock in THIS process.
#[derive(Debug, Clone)]
struct Holder {
    /// The thread that took it, by name, or its id when it has none.
    thread: String,
    /// When it was taken, in whole milliseconds after the Unix epoch.
    since_unix_ms: u128,
}

/// What THIS process holds on one month lock: its writer, if any, and how
/// many readers.
#[derive(Debug, Default)]
struct Held {
    writer: Option<Holder>,
    readers: u32,
}

/// Which side of a month lock a guard registered, so the guard's end forgets
/// exactly what its start recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Writer,
    Reader,
}

/// Every month lock this process holds through [`Flock::try_lock_noted`]
/// (writers) or [`Flock::try_lock_shared_patiently`] (readers), keyed by a
/// hash of the lock's path (no allocation per probe). One lock and one hash
/// probe when a guard starts and one when it ends: O(1) expected per
/// acquisition. Only those two constructors register, so the journals', the
/// results files' and every other lock's path pays nothing.
static HOLDERS: Mutex<Option<HashMap<u64, Held>>> = Mutex::new(None);

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

/// Records that this thread now holds `path` on `side`.
fn hold(path: &Path, side: Side) -> u64 {
    let key = key_of(path);
    let writer = (side == Side::Writer).then(|| Holder {
        thread: this_thread(),
        since_unix_ms: now_unix_ms(),
    });
    let mut held = HOLDERS.lock().unwrap_or_else(PoisonError::into_inner);
    let entry = held
        .get_or_insert_with(HashMap::new)
        .entry(key)
        .or_default();
    match writer {
        Some(holder) => entry.writer = Some(holder),
        None => entry.readers = entry.readers.saturating_add(1),
    }
    key
}

/// Forgets what [`hold`] recorded under `key` for `side`; an entry that holds
/// nothing more is removed, so the map is bounded by the locks held now.
fn let_go(key: u64, side: Side) {
    let mut held = HOLDERS.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(holders) = held.as_mut() else {
        return;
    };
    let empty = holders.get_mut(&key).is_some_and(|entry| {
        match side {
            Side::Writer => entry.writer = None,
            Side::Reader => entry.readers = entry.readers.saturating_sub(1),
        }
        entry.writer.is_none() && entry.readers == 0
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

/// Who holds the month lock at `path`, in words a page can show (R3-08;
/// rules-audit RA-298):
///
/// - a writer in THIS process: the pid, its thread and since when;
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
    let (writer, readers) = HOLDERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .and_then(|holders| holders.get(&key_of(path)))
        .map_or((None, 0), |held| (held.writer.clone(), held.readers));
    if let Some(holder) = writer {
        return format!(
            "held by this process (pid {pid}, thread {}) since {} ms after the Unix epoch, \
             lock {}",
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
        // writer has since ended and readers hold the month now — said, never
        // left for the reader to assume.
        Ok(Some(note)) => format!(
            "held by another process; the note the last writer left in the lock reads: {note} \
             (a writer killed while it held the month leaves its note behind, so if that pid \
             has ended the holder is a reader, which leaves none)"
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
    /// Whether `Drop` still owes the unlock. Cleared by [`Flock::release`]
    /// before it unlocks, so a refused release is reported once, by the
    /// release, and never again by `Drop`.
    held: bool,
    /// The holder registry's key and side while this guard holds a MONTH
    /// lock taken by [`Flock::try_lock_noted`] or
    /// [`Flock::try_lock_shared_patiently`], so a refused party can name it
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
    /// one `futimens` on letting go: O(1) per
    /// acquisition.
    ///
    /// # Errors
    ///
    /// As [`Self::try_lock`].
    pub fn try_lock_noted(file: F, path: P) -> Result<Self, TryLockError> {
        use std::os::unix::fs::FileExt as _;
        file.borrow().try_lock()?;
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
        let holding = Some((hold(path.as_ref(), Side::Writer), Side::Writer));
        Ok(Self {
            file,
            path,
            held: true,
            holding,
            noted: written.is_ok().then_some(times),
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
    /// cannot be given a deadline.
    ///
    /// # Errors
    ///
    /// `TryLockError::WouldBlock` when a writer still holds the lock as the
    /// patience runs out, or the host's refusal at once.
    pub fn try_lock_shared_patiently(file: F, path: P) -> Result<Self, TryLockError> {
        let started = Instant::now();
        let deadline = started + patience();
        let mut pause = Duration::from_millis(1);
        let taken = loop {
            match file.borrow().try_lock_shared() {
                Ok(()) => break Ok(()),
                Err(TryLockError::WouldBlock) => {
                    let left = deadline.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        break Err(TryLockError::WouldBlock);
                    }
                    std::thread::sleep(pause.min(left));
                    pause = pause.saturating_mul(2).min(MAX_PAUSE);
                }
                Err(host) => break Err(host),
            }
        };
        spend_patience(started.elapsed());
        taken?;
        let holding = Some((hold(path.as_ref(), Side::Reader), Side::Reader));
        Ok(Self {
            file,
            path,
            held: true,
            holding,
            noted: None,
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
        self.held = false;
        self.forget();
        unlock(self.file.borrow()).map_err(|why| Unreleased {
            path: self.path.as_ref().to_path_buf(),
            why,
        })
    }

    /// The guard for a lock the caller has just taken, unregistered.
    const fn taken(file: F, path: P) -> Self {
        Self {
            file,
            path,
            held: true,
            holding: None,
            noted: None,
        }
    }

    /// Clears the holder's note while the lock is still held, and forgets the
    /// registration: what every way a guard ends owes before it unlocks. Each
    /// part runs once, whichever end comes first.
    fn forget(&mut self) {
        if let Some(times) = self.noted.take() {
            let file = self.file.borrow();
            if let Err(why) = file.set_len(0).and_then(|()| file.set_times(times)) {
                note_unnoted(self.path.as_ref(), &why);
            }
        }
        if let Some((key, side)) = self.holding.take() {
            let_go(key, side);
        }
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
        self.forget();
        if self.held
            && let Err(why) = unlock(self.file.borrow())
        {
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

fn note_unreleased(path: &Path, why: &io::Error) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("store.flock", "advisory lock not released by its guard")
            .with("file", telemetry::Value::Str(&path.display().to_string()))
            .with("why", telemetry::Value::Str(&why.to_string())),
    );
}

#[cfg(test)]
#[path = "flock_tests.rs"]
pub(crate) mod tests;
