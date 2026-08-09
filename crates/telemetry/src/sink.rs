//! The writer: where the bytes go, what it costs, and what happens when it
//! cannot.
//!
//! # What one `emit` actually costs
//!
//! Stated as operations, not as a timing, because no timing was taken —
//! `CLAUDE.md` §3 rule 6.
//!
//! | step | cost |
//! |---|---|
//! | level check | one relaxed atomic load and a comparison. A filtered event returns here having touched nothing else: no clock, no lock, no buffer. |
//! | clock | one `SystemTime::now`, a vDSO read on the platforms this builds for rather than a full syscall |
//! | lock | one uncontended `Mutex` acquire |
//! | render | one pass over the event's own bytes into a buffer the sink owns and reuses. **Zero allocations in steady state**: the buffer is cleared, not freed. |
//! | rotation check | one integer comparison against a running byte count the sink already holds — never a `metadata` call |
//! | write | one `write` syscall of the whole line |
//! | unlock | |
//!
//! Nothing in that list is a function of how many events came before, how
//! large the file is, or how many files there are. That is the sense in which
//! this is O(1) per `CLAUDE.md` §3 rule 4, and it is the honest sense: the
//! render step is linear in the size of the *one* event being written, which
//! is unavoidable and is bounded by the ceilings in [`crate::event`], and a
//! syscall is not free.
//!
//! **"How large the file is" is the row that could have been a lie, and it is
//! the row with a test.**
//! `telemetry::sink::the_roll_decision_reads_the_running_count_and_never_the_files_size`
//! grows the current file behind this sink's back to sixty-four times the
//! bound and asserts the next `emit` does **not** roll — which it would if the
//! rotation check reached for `metadata` — and then that the running count
//! still rolls on its own. The remaining rows are single operations by
//! inspection, and
//! `telemetry::sink::the_file_rolls_at_the_bound_and_the_set_never_grows_past_the_count`
//! holds "how many files there are".
//!
//! **No timing was taken and none is claimed**, which the first line of this
//! section already said and this one keeps saying: every entry in the table is
//! a count of operations. This crate carries no bench. Turning the counts into
//! a measurement needs a `crates/telemetry/benches/ratio.rs` of the shape
//! `crates/store` and `crates/pull` already have — which CI gate 14 wants
//! anyway — and nothing here should be read as a figure until it exists.
//!
//! Once per [`Config::max_file_bytes`] a rotation happens instead, inside the
//! same lock: at most `2 * keep_files` renames and one `open`. That is a
//! constant, it is amortised over ~33,000 events at the default bound, and it
//! is the one place the lock is held across more I/O than a single write.
//! Named here rather than left to be discovered.
//!
//! # What is lost on a kill
//!
//! Nothing is buffered in user space. There is no `BufWriter`: every event is
//! one `write_all` straight to the descriptor, so an event whose `emit`
//! returned [`Emitted::Written`] is in the kernel's page cache and survives
//! `SIGKILL`, a panic and an abort.
//!
//! It does **not** survive a power cut or a kernel panic, because nothing here
//! calls `fsync`. That is deliberate, and it is the one place this crate and
//! `api::audit` differ on purpose: the journal `fsync`s every 256-byte record
//! because it records the outcome of a run an operator will act on and there
//! are a few hundred of them. This is the fine-grained stream, and an `fsync`
//! per event would put a millisecond of disk latency in the middle of every
//! request handler — logging would then be the slowest thing the server does,
//! which is one way a logger takes down the thing it observes. [`Sink::sync`]
//! is there for a caller that wants a barrier at a moment of its own choosing.
//!
//! # When it cannot write
//!
//! `CLAUDE.md` §4 forbids a fallback that hides a failure. It also has to be
//! true that a full disk does not take down a backfill that is otherwise
//! working. Both, resolved explicitly:
//!
//! * **The caller is never killed.** [`Sink::emit`] does not panic, does not
//!   return an `Err` the caller must handle, and does not block on anything
//!   but its own mutex. A poisoned mutex — another thread panicked mid-write —
//!   is recovered from rather than propagated, because refusing to log
//!   *because an earlier log failed* is the worst of both behaviours.
//! * **The failure is never hidden.** Three surfaces, none of which a
//!   consumer that is looking can miss:
//!   1. `emit` returns [`Emitted::Dropped`] for that event.
//!   2. [`Sink::health`] carries a running count of dropped events and the
//!      most recent failure in its own words. This is what a page renders.
//!   3. The **first** failure, and only the first, goes to `stderr`.
//!
//! The trade-off, stated: between the first failure and the moment somebody
//! reads [`Sink::health`], those events are gone and cannot be recovered. They
//! are not queued in memory, on purpose — an unbounded backlog of a log that
//! cannot be written is precisely how a logging subsystem kills its host. Once
//! on `stderr`, and counted forever afterwards.
//!
//! # Rotation, and why this rotates when the journal does not
//!
//! `api::audit` explicitly refuses to rotate, citing `CLAUDE.md` §3 rule 8:
//! history is append-only. That is right *for the journal*. It is one
//! 256-byte record per operator-initiated pull, a hundred thousand of them is
//! 25.6 MB, and every one is a fact somebody may want in a year.
//!
//! This file is the other thing: per-request, per-window, per-retry events at
//! a few hundred bytes each. A twelve-hour backfill over the stated ~11,200
//! windows, at even five events a window, is ~56,000 lines; at debug level
//! over every HTTP request it is orders of magnitude more. Unbounded, that
//! fills a disk — and a full disk stops the backfill, so the logger would have
//! taken down the thing it observes.
//!
//! The two therefore compose rather than compete: **the journal is the history
//! and is never rotated; this is the stream and is deliberately bounded.** The
//! bound is [`DEFAULT_MAX_FILE_BYTES`] × [`DEFAULT_KEEP_FILES`] = 64 MiB,
//! stated in bytes so nobody has to guess, and what falls off the end is
//! deleted rather than compressed — a compressor would be a dependency, and
//! the events worth keeping forever are the ones the journal already has.

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use crate::clock::now_millis;
use crate::encode::line;
use crate::event::Event;
use crate::level::Level;
use crate::record::Record;

/// The name every file in the set starts with.
pub const BASENAME: &str = "events";

/// The extension every file in the set ends with.
pub const EXTENSION: &str = "ndjson";

/// How large the file being appended to may get before it rolls.
///
/// 8 MiB. At a ~250-byte line that is roughly 33,000 events — more than a
/// whole twelve-hour backfill emits at info level, so in ordinary operation
/// the newest file holds the entire run and a tail reader never crosses a
/// file boundary.
pub const DEFAULT_MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// How many files are kept, the one being written included.
///
/// Eight. With the bound above that is a **64 MiB ceiling on this crate's
/// entire footprint**, forever: negligible beside the 40 GB lake, and small
/// enough that no operator has to think about it. It is also ~260,000 events
/// of history, which is several backfills.
pub const DEFAULT_KEEP_FILES: u8 = 8;

/// The smallest file bound this crate accepts.
///
/// A bound below one line's worth would roll on nearly every event and turn
/// rotation — a constant amortised over thousands of writes — into part of the
/// per-event cost. Refused at [`Sink::open`] rather than clamped, because a
/// clamp is a setting that silently did something other than what it said.
pub const MIN_FILE_BYTES: u64 = 1024;

/// What happened to one event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emitted {
    /// The bytes reached the file.
    Written,
    /// Quieter than the sink's floor. Not a failure: nothing was meant to
    /// happen, and nothing did.
    Filtered,
    /// It could not be written, and the reason is in [`Sink::health`].
    Dropped,
    /// No sink is installed. Only [`crate::emit`] returns this.
    NotInstalled,
}

impl Emitted {
    /// Whether the event reached the file.
    #[must_use]
    pub const fn is_written(self) -> bool {
        matches!(self, Self::Written)
    }
}

/// Where a sink's bytes actually go.
///
/// # Why this is a trait and not a `File`
///
/// `api::audit::Journal::append` has two arms no test drives — a failing
/// `write_all` and a failing `fsync` — and says so, naming
/// `crates/store/src/file.rs` as the place this workspace already solved the
/// shape with a trait, and recording that the fix was available and not built.
/// It is built here, because the arm in question is the one this crate's whole
/// failure policy rests on: "a write that cannot land is reported and not
/// swallowed" is worth nothing if no test can make a write fail. A full disk
/// is not a state a developer's machine enters on request; a
/// [`Target`] that refuses is.
///
/// The cost is one indirect call per event, against a syscall in the same
/// critical section. It does not change the shape of the per-event cost.
pub trait Target: Send + core::fmt::Debug {
    /// Appends one whole line.
    ///
    /// # Errors
    ///
    /// Whatever the destination said.
    fn append(&mut self, bytes: &[u8]) -> std::io::Result<()>;

    /// Makes everything already appended durable.
    ///
    /// # Errors
    ///
    /// Whatever the destination said.
    fn sync(&self) -> std::io::Result<()>;

    /// Points at `path` instead, after the set has rolled.
    ///
    /// # Errors
    ///
    /// Whatever the destination said.
    fn reopen(&mut self, path: &Path) -> std::io::Result<()>;
}

/// A [`Target`] that is a file on disk. What every real sink uses.
#[derive(Debug)]
pub struct FileTarget {
    file: File,
}

impl FileTarget {
    /// Opens `path` for appending, creating it if it is not there.
    ///
    /// # Errors
    ///
    /// Whatever the filesystem said.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .map(|file| Self { file })
    }

    /// How many bytes the file already holds. One `metadata` call, at open
    /// time only — never on the write path.
    fn len(&self) -> u64 {
        self.file.metadata().map_or(0, |m| m.len())
    }
}

impl Target for FileTarget {
    fn append(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.file.write_all(bytes)
    }

    fn sync(&self) -> std::io::Result<()> {
        self.file.sync_all()
    }

    fn reopen(&mut self, path: &Path) -> std::io::Result<()> {
        self.file = OpenOptions::new().append(true).create(true).open(path)?;
        Ok(())
    }
}

/// Where the files go and how large the set may get.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The directory the set lives in. Created if it is not there.
    pub dir: PathBuf,
    /// How large the file being appended to may get before it rolls.
    pub max_file_bytes: u64,
    /// How many files are kept, the one being written included.
    pub keep_files: u8,
    /// The quietest level that is written at all.
    pub min_level: Level,
}

impl Config {
    /// The defaults, in `dir`.
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            keep_files: DEFAULT_KEEP_FILES,
            min_level: Level::Info,
        }
    }

    /// The same configuration with a different file bound.
    #[must_use]
    pub const fn with_max_file_bytes(mut self, bytes: u64) -> Self {
        self.max_file_bytes = bytes;
        self
    }

    /// The same configuration keeping a different number of files.
    #[must_use]
    pub const fn with_keep_files(mut self, files: u8) -> Self {
        self.keep_files = files;
        self
    }

    /// The same configuration with a different floor.
    #[must_use]
    pub const fn with_min_level(mut self, level: Level) -> Self {
        self.min_level = level;
        self
    }

    /// The ceiling this configuration puts on the whole set, in bytes.
    ///
    /// Saturating, so a configuration that would overflow the answer reports
    /// the largest number rather than a small wrong one.
    #[must_use]
    pub const fn ceiling_bytes(&self) -> u64 {
        self.max_file_bytes.saturating_mul(self.keep_files as u64)
    }

    /// Why this configuration cannot work, when it cannot.
    ///
    /// Checked before anything touches the disk — and, in [`crate::install`],
    /// before the process-wide slot is even looked at, so that a configuration
    /// this crate would never accept is refused for the same reason whether or
    /// not a sink is already installed.
    pub(crate) fn refusal(&self) -> Option<String> {
        if self.max_file_bytes < MIN_FILE_BYTES {
            return Some(format!(
                "a file bound of {} bytes is below the {MIN_FILE_BYTES}-byte floor: it \
                 would roll on nearly every event",
                self.max_file_bytes
            ));
        }
        if self.keep_files == 0 {
            return Some(
                "keep_files is 0, which keeps no file at all — there would be nowhere to write"
                    .to_owned(),
            );
        }
        None
    }
}

/// The directory a telemetry set lives in, beneath a store root.
///
/// Beside `audit/`, not inside it: the journal there is a history that is
/// never rewritten and this is a stream that is deliberately bounded. Two
/// different rules, two different directories — the same argument
/// `api::audit::journal_path` makes about `manifest/`.
#[must_use]
pub fn dir_beneath_store(root: &Path) -> PathBuf {
    root.join("telemetry")
}

/// The file being appended to.
#[must_use]
pub fn current_path(dir: &Path) -> PathBuf {
    dir.join(format!("{BASENAME}.{EXTENSION}"))
}

/// The `n`th rolled file, newest at 1.
#[must_use]
pub fn rotated_path(dir: &Path, n: u8) -> PathBuf {
    dir.join(format!("{BASENAME}.{n}.{EXTENSION}"))
}

/// Every file in the set, newest first, whether or not each exists.
///
/// The order a tail reader walks. Bounded by `keep_files`, which is a `u8`, so
/// this allocates at most 255 paths and does so once per query.
#[must_use]
pub fn paths_newest_first(dir: &Path, keep_files: u8) -> Vec<PathBuf> {
    let mut out = vec![current_path(dir)];
    for n in 1..keep_files.max(1) {
        out.push(rotated_path(dir, n));
    }
    out
}

/// What the sink has done, and what it has failed to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Health {
    /// The file being appended to.
    pub path: PathBuf,
    /// Events whose bytes reached the file.
    pub written: u64,
    /// Events that could not be written and are gone.
    ///
    /// **This is the number a page must show.** A non-zero value here is the
    /// only trace left of events that happened and were never recorded.
    pub dropped: u64,
    /// Times the file rolled.
    pub rotations: u64,
    /// Times a roll failed, leaving the current file past its bound.
    pub rotation_failures: u64,
    /// The most recent failure, in the failure's own words.
    pub last_error: Option<String>,
    /// How large the current file is, from the sink's own running count — no
    /// `metadata` call and no scan.
    pub current_bytes: u64,
    /// The sequence number the next event will carry.
    pub next_seq: u64,
}

impl Health {
    /// Whether an operator has to be told about this sink.
    #[must_use]
    pub const fn is_loud(&self) -> bool {
        self.dropped > 0 || self.rotation_failures > 0
    }
}

/// What is behind the lock.
#[derive(Debug)]
struct Inner {
    target: Box<dyn Target>,
    /// A running count, kept in step with every write. Never a `metadata`
    /// call: asking the filesystem how large the file is, per event, would be
    /// a syscall to learn a number this already knows.
    bytes: u64,
    /// The sequence number of the last event written.
    seq: u64,
    /// Rendered here and reused. Cleared, never freed.
    buf: Vec<u8>,
}

/// The event stream's writer.
///
/// Thread-safe and meant to be shared: one per process behind
/// [`crate::install`], or one per test behind an `Arc`.
pub struct Sink {
    dir: PathBuf,
    max_file_bytes: u64,
    keep_files: u8,
    min_level: AtomicU8,
    inner: Mutex<Inner>,
    written: AtomicU64,
    dropped: AtomicU64,
    rotations: AtomicU64,
    rotation_failures: AtomicU64,
    reported: AtomicBool,
    last_error: Mutex<Option<String>>,
}

impl core::fmt::Debug for Sink {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Deliberately not the health and deliberately not the lock: a `Debug`
        // that takes a mutex can deadlock the thing printing it.
        f.debug_struct("Sink")
            .field("dir", &self.dir)
            .field("max_file_bytes", &self.max_file_bytes)
            .field("keep_files", &self.keep_files)
            .field("dropped", &self.dropped.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl Sink {
    /// Opens the set in `config.dir`, creating the directory if it is not
    /// there.
    ///
    /// The sequence number resumes from the last readable line of the current
    /// file, so a restart continues the stream rather than starting a second
    /// one at zero. That read is one block from the end and nothing more; see
    /// [`resume_seq`].
    ///
    /// # Errors
    ///
    /// The failure in its own words, prefixed with the path. **Opening is the
    /// one operation here allowed to fail loudly**: a caller that cannot
    /// create its log directory has been told before anything depends on the
    /// logger, which is a wholly different situation from a disk that fills up
    /// eleven hours into a backfill.
    ///
    /// A bound below [`MIN_FILE_BYTES`] or a `keep_files` of zero is refused
    /// by name rather than clamped.
    pub fn open(config: &Config) -> Result<Self, String> {
        if let Some(why) = config.refusal() {
            return Err(why);
        }
        std::fs::create_dir_all(&config.dir).map_err(|e| {
            format!(
                "{}: cannot create the telemetry directory — {e}",
                config.dir.display()
            )
        })?;
        let path = current_path(&config.dir);
        let target = FileTarget::open(&path)
            .map_err(|e| format!("{}: cannot open the event stream — {e}", path.display()))?;
        let bytes = target.len();
        Ok(Self::around(
            config,
            Box::new(target),
            bytes,
            resume_seq(&path),
        ))
    }

    /// The same sink around somewhere other than a file.
    ///
    /// The seam [`Target`] exists for. It starts empty and at sequence zero:
    /// a destination this crate did not open is one it cannot measure or read
    /// a sequence number back out of.
    ///
    /// # Errors
    ///
    /// A configuration that cannot work, in its own words.
    pub fn with_target(config: &Config, target: Box<dyn Target>) -> Result<Self, String> {
        if let Some(why) = config.refusal() {
            return Err(why);
        }
        Ok(Self::around(config, target, 0, 0))
    }

    /// The common construction.
    fn around(config: &Config, target: Box<dyn Target>, bytes: u64, seq: u64) -> Self {
        Self {
            dir: config.dir.clone(),
            max_file_bytes: config.max_file_bytes,
            keep_files: config.keep_files,
            min_level: AtomicU8::new(config.min_level.rank()),
            inner: Mutex::new(Inner {
                target,
                bytes,
                seq,
                buf: Vec::with_capacity(512),
            }),
            written: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            rotations: AtomicU64::new(0),
            rotation_failures: AtomicU64::new(0),
            reported: AtomicBool::new(false),
            last_error: Mutex::new(None),
        }
    }

    /// The file being appended to.
    #[must_use]
    pub fn path(&self) -> PathBuf {
        current_path(&self.dir)
    }

    /// The directory the whole set lives in.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Every file in the set, newest first.
    #[must_use]
    pub fn paths(&self) -> Vec<PathBuf> {
        paths_newest_first(&self.dir, self.keep_files)
    }

    /// How many files this sink keeps.
    #[must_use]
    pub const fn keep_files(&self) -> u8 {
        self.keep_files
    }

    /// The quietest level currently written.
    ///
    /// One relaxed load. The stored rank is always one this crate wrote — the
    /// only writer is [`Sink::set_min_level`], which takes a [`Level`] — so
    /// the `unwrap_or` is a backstop no caller can drive.
    #[must_use]
    pub fn min_level(&self) -> Level {
        Level::of_rank(self.min_level.load(Ordering::Relaxed)).unwrap_or(Level::Trace)
    }

    /// Changes the floor, for every thread, without a lock.
    ///
    /// Deliberately adjustable at runtime: an operator watching a backfill go
    /// wrong should be able to turn debug on without restarting the process
    /// that is halfway through it.
    pub fn set_min_level(&self, level: Level) {
        self.min_level.store(level.rank(), Ordering::Relaxed);
    }

    /// Writes one event.
    ///
    /// Never panics and never propagates a failure; the module documentation
    /// says how a failure is surfaced instead.
    pub fn emit(&self, event: &Event<'_>) -> Emitted {
        if !event.level().at_least(self.min_level()) {
            return Emitted::Filtered;
        }
        let at = now_millis();
        // A POISONED LOCK IS RECOVERED FROM, NOT PROPAGATED. Poisoning means
        // another thread panicked while holding this mutex. What is behind it
        // is a destination, a byte count and a scratch buffer, none of which a
        // panic can leave in a state that makes the next line wrong — and
        // refusing to log because an earlier log panicked is the one behaviour
        // guaranteed to lose the events that explain the panic.
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let inner = &mut *guard;
        inner.seq = inner.seq.saturating_add(1);
        inner.buf.clear();
        line(&mut inner.buf, inner.seq, at, event);
        let span = u64::try_from(inner.buf.len()).unwrap_or(u64::MAX);
        if inner.bytes > 0
            && inner.bytes.saturating_add(span) > self.max_file_bytes
            && let Err(why) = self.roll(inner)
        {
            // The roll failed, so the current file will exceed its bound.
            // The event is still written: losing it because a RENAME failed
            // would be the worse of the two, and a bound that has been
            // exceeded is visible in `health()`.
            self.rotation_failures.fetch_add(1, Ordering::Relaxed);
            self.report(&why);
        }
        let landed = inner.target.append(&inner.buf);
        match landed {
            Ok(()) => {
                inner.bytes = inner.bytes.saturating_add(span);
                drop(guard);
                self.written.fetch_add(1, Ordering::Relaxed);
                Emitted::Written
            }
            Err(e) => {
                drop(guard);
                let why = format!(
                    "{}: cannot append the event — {e}",
                    current_path(&self.dir).display()
                );
                self.dropped.fetch_add(1, Ordering::Relaxed);
                self.report(&why);
                Emitted::Dropped
            }
        }
    }

    /// Makes everything already written durable.
    ///
    /// Not called per event; the module documentation says why. A caller that
    /// wants a barrier — before a deliberate shutdown, or straight after the
    /// event that explains a failure — calls this.
    ///
    /// # Errors
    ///
    /// The failure in its own words. Unlike [`Sink::emit`] this one *is*
    /// returned, because a caller that asked for durability has to be told it
    /// did not get it.
    pub fn sync(&self) -> Result<(), String> {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let outcome = guard.target.sync();
        drop(guard);
        outcome.map_err(|e| {
            let why = format!(
                "{}: the events were written and not synced — {e}",
                current_path(&self.dir).display()
            );
            self.report(&why);
            why
        })
    }

    /// What it has done, and what it has failed to do.
    #[must_use]
    pub fn health(&self) -> Health {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let (bytes, next_seq) = (guard.bytes, guard.seq.saturating_add(1));
        drop(guard);
        let last = self
            .last_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        Health {
            path: current_path(&self.dir),
            written: self.written.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            rotations: self.rotations.load(Ordering::Relaxed),
            rotation_failures: self.rotation_failures.load(Ordering::Relaxed),
            last_error: last,
            current_bytes: bytes,
            next_seq,
        }
    }

    /// Rolls the set: the current file becomes `.1`, each `.n` becomes
    /// `.n + 1`, and the oldest is deleted.
    ///
    /// At most `2 * keep_files` syscalls, once per [`Config::max_file_bytes`].
    /// A file that is not there is not an error — the set is sparse until it
    /// has rolled `keep_files` times.
    fn roll(&self, inner: &mut Inner) -> Result<(), String> {
        let keep = self.keep_files.max(1);
        let named = |path: &Path, what: &str, e: &std::io::Error| {
            format!("{}: {what} — {e}", path.display())
        };
        if keep > 1 {
            // The oldest goes first, or the shift below overwrites a file it
            // has not yet moved.
            let oldest = rotated_path(&self.dir, keep.saturating_sub(1));
            remove_if_present(&oldest)
                .map_err(|e| named(&oldest, "cannot delete the oldest file", &e))?;
            for n in (1..keep.saturating_sub(1)).rev() {
                let from = rotated_path(&self.dir, n);
                let to = rotated_path(&self.dir, n.saturating_add(1));
                rename_if_present(&from, &to).map_err(|e| named(&from, "cannot roll", &e))?;
            }
            let current = current_path(&self.dir);
            rename_if_present(&current, &rotated_path(&self.dir, 1))
                .map_err(|e| named(&current, "cannot roll the current file", &e))?;
        } else {
            // ONE FILE KEPT MEANS NO HISTORY AT ALL. The current file is
            // deleted rather than rolled, which is what `keep_files = 1` asks
            // for; it is stated here so it cannot be mistaken for a bug.
            let current = current_path(&self.dir);
            remove_if_present(&current)
                .map_err(|e| named(&current, "cannot delete the current file", &e))?;
        }
        let path = current_path(&self.dir);
        inner
            .target
            .reopen(&path)
            .map_err(|e| named(&path, "cannot open the next file", &e))?;
        inner.bytes = 0;
        self.rotations.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Records a failure, and says it out loud exactly once.
    fn report(&self, why: &str) {
        *self
            .last_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(why.to_owned());
        if self
            .reported
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            // ONCE. A logger that cannot write and says so on every event
            // turns one failure into a second denial of service, on the one
            // stream still working. The count in `health()` is the running
            // total; this is the notice that there is a count to look at.
            eprintln!(
                "telemetry: {why}\ntelemetry: this is the ONLY notice; the running count of \
                 lost events is Sink::health().dropped"
            );
        }
    }
}

/// Deletes a path, treating "it was not there" as success.
fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Renames a path, treating "it was not there" as success.
fn rename_if_present(from: &Path, to: &Path) -> std::io::Result<()> {
    match std::fs::rename(from, to) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// The sequence number to carry on from, read from one block at the end.
///
/// Never fails: an absent file, an unreadable one, and a file whose last line
/// will not decode all mean the same thing here — this build cannot tell what
/// came before, so it starts at zero. That is a **stated limit and not a
/// silent one**: a wiped or corrupted tail restarts the numbering, and the
/// `ms` field, which is never reused, is what orders events across such a
/// restart.
fn resume_seq(path: &Path) -> u64 {
    /// One block at the end. 64 KiB holds ~250 lines at the typical width, so
    /// the last complete line is inside it unless one line is larger than the
    /// block, which the ceilings in `crate::event` make impossible.
    const BLOCK: u64 = 64 * 1024;
    let Ok(meta) = std::fs::metadata(path) else {
        return 0;
    };
    let len = meta.len();
    if len == 0 {
        return 0;
    }
    let from = len.saturating_sub(BLOCK);
    let Ok(bytes) = crate::tail::read_at(path, from, len.saturating_sub(from)) else {
        return 0;
    };
    bytes
        .split(|&b| b == b'\n')
        .rev()
        .find_map(|line| Record::decode(line).ok())
        .map_or(0, |record| record.seq)
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{
        Config, DEFAULT_KEEP_FILES, DEFAULT_MAX_FILE_BYTES, Emitted, MIN_FILE_BYTES, Sink, Target,
        current_path, dir_beneath_store, paths_newest_first, rotated_path,
    };
    use crate::event::{Event, MAX_MESSAGE_BYTES, MAX_STR_VALUE_BYTES};
    use crate::level::Level;
    use crate::record::Record;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A directory no other live process will name, emptied first.
    ///
    /// The process id is in the name for the reason `api::scratch` gives: a
    /// fixed name in the shared temporary directory is a fixture two
    /// concurrent test binaries both claim, and this suite deletes what it
    /// creates.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("brutex-telemetry-{}-{name}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);
        dir
    }

    fn lines_of(path: &Path) -> Vec<Record> {
        let bytes = std::fs::read(path).unwrap_or_default();
        bytes
            .split(|&b| b == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| {
                Record::decode(line).unwrap_or_else(|e| {
                    panic!(
                        "{}: {e} in {}",
                        path.display(),
                        String::from_utf8_lossy(line)
                    )
                })
            })
            .collect()
    }

    /// A destination that refuses, so the failure policy can be driven by a
    /// test rather than by a full disk.
    #[derive(Debug, Default)]
    struct Brittle {
        refusing: AtomicBool,
    }

    impl Brittle {
        fn refuse(&self) {
            self.refusing.store(true, Ordering::Relaxed);
        }

        fn no(&self) -> std::io::Result<()> {
            if self.refusing.load(Ordering::Relaxed) {
                Err(std::io::Error::new(
                    std::io::ErrorKind::StorageFull,
                    "no space left on device",
                ))
            } else {
                Ok(())
            }
        }
    }

    impl Target for Arc<Brittle> {
        fn append(&mut self, _bytes: &[u8]) -> std::io::Result<()> {
            self.no()
        }

        fn sync(&self) -> std::io::Result<()> {
            self.no()
        }

        fn reopen(&mut self, _path: &Path) -> std::io::Result<()> {
            self.no()
        }
    }

    #[test]
    fn the_stated_ceiling_is_sixty_four_mebibytes_and_it_is_arithmetic_not_a_claim() {
        let config = Config::new("/nowhere");
        assert_eq!(config.max_file_bytes, DEFAULT_MAX_FILE_BYTES);
        assert_eq!(config.keep_files, DEFAULT_KEEP_FILES);
        assert_eq!(config.ceiling_bytes(), 64 * 1024 * 1024);
        assert_eq!(config.min_level, Level::Info);
        // A configuration that would overflow the answer saturates rather than
        // reporting a small wrong number.
        assert_eq!(
            Config::new("/nowhere")
                .with_max_file_bytes(u64::MAX)
                .with_keep_files(255)
                .ceiling_bytes(),
            u64::MAX
        );
        assert_eq!(
            Config::new("/nowhere")
                .with_min_level(Level::Trace)
                .min_level,
            Level::Trace
        );
    }

    #[test]
    fn the_paths_are_the_ones_a_reader_walks_newest_first() {
        let dir = Path::new("/tmp/x");
        assert_eq!(current_path(dir), Path::new("/tmp/x/events.ndjson"));
        assert_eq!(rotated_path(dir, 3), Path::new("/tmp/x/events.3.ndjson"));
        assert_eq!(
            paths_newest_first(dir, 3),
            vec![
                PathBuf::from("/tmp/x/events.ndjson"),
                PathBuf::from("/tmp/x/events.1.ndjson"),
                PathBuf::from("/tmp/x/events.2.ndjson"),
            ]
        );
        assert_eq!(paths_newest_first(dir, 0).len(), 1, "never fewer than one");
        assert_eq!(
            dir_beneath_store(Path::new("/root")),
            Path::new("/root/telemetry"),
            "beside audit/, not inside it"
        );
    }

    #[test]
    fn a_configuration_that_could_not_work_is_refused_by_name_rather_than_clamped() {
        let dir = scratch("refuse");
        let too_small = Config::new(&dir).with_max_file_bytes(MIN_FILE_BYTES - 1);
        let said = Sink::open(&too_small).expect_err("a bound below one line is refused");
        assert!(said.contains("below the"), "{said}");
        assert!(said.contains("roll on nearly every event"), "{said}");

        let no_files = Config::new(&dir).with_keep_files(0);
        let said = Sink::open(&no_files).expect_err("keeping no file is refused");
        assert!(said.contains("keep_files is 0"), "{said}");
        assert!(
            !dir.exists(),
            "a refused configuration created nothing on disk"
        );
        // The same refusals guard the seam, so a caller cannot slip past them
        // by supplying its own destination.
        assert!(Sink::with_target(&too_small, Box::new(Arc::new(Brittle::default()))).is_err());
        assert!(Sink::with_target(&no_files, Box::new(Arc::new(Brittle::default()))).is_err());
    }

    /// AN UNWRITABLE PATH IS REPORTED BEFORE ANYTHING DEPENDS ON THE LOGGER.
    #[test]
    fn opening_where_a_directory_cannot_exist_says_so_and_names_the_path() {
        let dir = scratch("blocked");
        std::fs::create_dir_all(&dir).expect("a directory");
        // A FILE where the telemetry directory has to be. `create_dir_all`
        // cannot succeed, and the refusal has to name the path or an operator
        // has nothing to act on.
        let blocked = dir.join("telemetry");
        std::fs::write(&blocked, b"in the way").expect("wrote");
        let said = Sink::open(&Config::new(&blocked)).expect_err("cannot be a directory");
        assert!(
            said.contains(&blocked.display().to_string()),
            "the refusal names the path: {said}"
        );
        assert!(said.contains("cannot create"), "{said}");

        // And a path that IS a directory cannot be opened as the event file.
        let as_dir = dir.join("as-dir");
        std::fs::create_dir_all(as_dir.join("events.ndjson")).expect("a directory in the way");
        let said = Sink::open(&Config::new(&as_dir)).expect_err("cannot open a directory");
        assert!(said.contains("cannot open the event stream"), "{said}");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_event_reaches_the_file_and_the_health_counts_it() {
        let dir = scratch("write");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        assert_eq!(sink.path(), current_path(&dir));
        assert_eq!(sink.dir(), dir);
        assert_eq!(sink.keep_files(), DEFAULT_KEEP_FILES);
        assert_eq!(sink.paths().len(), usize::from(DEFAULT_KEEP_FILES));

        let event = Event::info("api.server", "listening").with("port", 8080u32);
        assert_eq!(sink.emit(&event), Emitted::Written);
        assert!(sink.emit(&event).is_written());

        let health = sink.health();
        assert_eq!(health.written, 2);
        assert_eq!(health.dropped, 0);
        assert_eq!(health.rotations, 0);
        assert_eq!(health.rotation_failures, 0);
        assert_eq!(health.last_error, None);
        assert!(!health.is_loud());
        assert_eq!(health.next_seq, 3, "one-based, so the next one is third");

        let records = lines_of(&sink.path());
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].seq, 1, "the first event is seq 1");
        assert_eq!(records[1].seq, 2);
        assert!(records[0].matches(&event));
        assert_eq!(
            health.current_bytes,
            std::fs::metadata(sink.path()).unwrap().len(),
            "the running byte count matches the file, with no metadata call on \
             the write path"
        );
        assert!(sink.sync().is_ok());
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A QUIETER EVENT COSTS A LOAD AND A COMPARISON, AND NOTHING ELSE.
    #[test]
    fn an_event_below_the_floor_is_filtered_and_never_reaches_the_file() {
        let dir = scratch("floor");
        let sink = Sink::open(&Config::new(&dir).with_min_level(Level::Warn)).expect("opens");
        assert_eq!(sink.min_level(), Level::Warn);
        assert_eq!(sink.emit(&Event::info("t", "quiet")), Emitted::Filtered);
        assert_eq!(sink.emit(&Event::debug("t", "quieter")), Emitted::Filtered);
        assert_eq!(sink.emit(&Event::warn("t", "loud")), Emitted::Written);
        assert_eq!(sink.emit(&Event::error("t", "louder")), Emitted::Written);
        assert_eq!(lines_of(&sink.path()).len(), 2);
        assert_eq!(sink.health().written, 2, "a filtered event is not written");
        assert_eq!(sink.health().dropped, 0, "and it is not dropped either");

        // The floor moves at runtime, for every thread, without a restart.
        sink.set_min_level(Level::Trace);
        assert_eq!(sink.min_level(), Level::Trace);
        assert_eq!(sink.emit(&Event::trace("t", "now kept")), Emitted::Written);
        assert_eq!(lines_of(&sink.path()).len(), 3);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// ROTATION HAPPENS AT THE STATED BOUND AND KEEPS THE STATED NUMBER.
    ///
    /// Both halves matter and they fail differently: a bound that is never
    /// reached fills the disk, and a set that is never trimmed does the same
    /// more slowly. Asserted as file sizes and a file count, not as "it
    /// rotated".
    #[test]
    fn the_file_rolls_at_the_bound_and_the_set_never_grows_past_the_count() {
        let dir = scratch("roll");
        let config = Config::new(&dir)
            .with_max_file_bytes(2048)
            .with_keep_files(3);
        let sink = Sink::open(&config).expect("opens");
        assert_eq!(config.ceiling_bytes(), 6144);

        for i in 0..400u32 {
            assert_eq!(
                sink.emit(&Event::info("roll", "one more").with("i", i)),
                Emitted::Written
            );
        }

        // NO FILE IS PAST THE BOUND.
        for path in sink.paths() {
            let len = std::fs::metadata(&path).map_or(0, |m| m.len());
            assert!(
                len <= 2048,
                "{} is {len} bytes, past the bound",
                path.display()
            );
        }
        // AND THERE IS NO FOURTH FILE.
        assert!(
            !rotated_path(&dir, 3).exists(),
            "keep_files = 3 means three files, and the fourth was deleted"
        );
        assert_eq!(sink.paths().into_iter().filter(|p| p.exists()).count(), 3);
        assert!(sink.health().rotations > 0, "it actually rolled");
        assert_eq!(sink.health().rotation_failures, 0);

        // THE NEWEST FILE HOLDS THE NEWEST EVENTS, which is what makes the
        // tail reader's walk order correct.
        let current = lines_of(&current_path(&dir));
        let first_rolled = lines_of(&rotated_path(&dir, 1));
        let second_rolled = lines_of(&rotated_path(&dir, 2));
        assert!(!current.is_empty() && !first_rolled.is_empty());
        assert!(
            current[0].seq > first_rolled[first_rolled.len() - 1].seq,
            "events.ndjson holds later events than events.1.ndjson"
        );
        assert!(
            first_rolled[0].seq > second_rolled[second_rolled.len() - 1].seq,
            "and events.1 holds later events than events.2"
        );
        // The sequence is unbroken across the boundary: nothing was lost in
        // the roll itself.
        assert_eq!(first_rolled[first_rolled.len() - 1].seq + 1, current[0].seq);
        assert_eq!(sink.health().written, 400);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// The rotation check reads the sink's own running count, never the file.
    ///
    /// This module's header lists what one `emit` costs and says of that step
    /// "one integer comparison against a running byte count the sink already
    /// holds — **never** a `metadata` call", and `FileTarget::len` says the same
    /// thing from the other side: "at open time only — never on the write path".
    /// Nothing held either sentence. Every rotation test above drives the count
    /// and the file size together, so a `metadata()` on the write path would
    /// pass all of them.
    ///
    /// So this pulls the two apart: the file is grown behind the sink's back to
    /// sixty-four times the bound, and the next `emit` must **not** roll. Then
    /// it keeps emitting, and the count must roll on its own — otherwise the
    /// test could be satisfied by a sink that had simply stopped rotating.
    #[test]
    fn the_roll_decision_reads_the_running_count_and_never_the_files_size() {
        use std::io::Write as _;

        let dir = scratch("running-count");
        let sink = Sink::open(
            &Config::new(&dir)
                .with_max_file_bytes(MIN_FILE_BYTES)
                .with_keep_files(2),
        )
        .expect("opens");

        assert_eq!(sink.emit(&Event::info("count", "one")), Emitted::Written);
        let after_one = sink.health().current_bytes;
        assert!(
            after_one > 0 && after_one < MIN_FILE_BYTES,
            "one line is under the bound: {after_one}"
        );

        // GROWN BEHIND THE SINK'S BACK. Same path, same append mode, so the
        // sink's own writes still land after these bytes.
        let mut behind = std::fs::OpenOptions::new()
            .append(true)
            .open(current_path(&dir))
            .expect("the current file exists");
        behind
            .write_all(&vec![b'x'; 64 * 1024])
            .expect("grow it past the bound");
        drop(behind);

        let on_disk = std::fs::metadata(current_path(&dir)).expect("stat").len();
        assert!(
            on_disk > MIN_FILE_BYTES * 8,
            "the file is now far past the bound: {on_disk}"
        );

        assert_eq!(sink.emit(&Event::info("count", "two")), Emitted::Written);
        assert_eq!(
            sink.health().rotations,
            0,
            "a `metadata` call on the write path would have rolled on a \
             {on_disk}-byte file against a {MIN_FILE_BYTES}-byte bound"
        );
        assert!(
            sink.health().current_bytes < on_disk,
            "the count is the sink's own, not the file's"
        );

        // AND THE COUNT STILL ROLLS, so this cannot be passed by not rotating.
        for i in 0..64u32 {
            assert_eq!(
                sink.emit(&Event::info("count", "more").with("i", i)),
                Emitted::Written
            );
        }
        assert!(
            sink.health().rotations > 0,
            "the running count is still what triggers a roll"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keeping_one_file_keeps_one_file_and_the_one_file_is_still_bounded() {
        let dir = scratch("keep-one");
        let sink = Sink::open(
            &Config::new(&dir)
                .with_max_file_bytes(1024)
                .with_keep_files(1),
        )
        .expect("opens");
        for i in 0..200u32 {
            assert!(sink.emit(&Event::info("t", "m").with("i", i)).is_written());
        }
        assert!(!rotated_path(&dir, 1).exists(), "no history is kept");
        assert!(std::fs::metadata(current_path(&dir)).unwrap().len() <= 1024);
        assert!(sink.health().rotations > 0);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A WRITE THAT CANNOT LAND IS COUNTED AND NAMED, NEVER SWALLOWED.
    ///
    /// Three surfaces, asserted separately because a consumer may only be
    /// looking at one of them: the return value, the running count, and the
    /// reason in the sink's own words. And the caller is still alive at the
    /// end, which is the other half of the rule.
    #[test]
    fn a_write_that_cannot_land_is_counted_and_named_rather_than_silently_lost() {
        let dir = scratch("full-disk");
        let brittle = Arc::new(Brittle::default());
        let sink =
            Sink::with_target(&Config::new(&dir), Box::new(Arc::clone(&brittle))).expect("opens");

        assert_eq!(sink.emit(&Event::info("t", "before")), Emitted::Written);
        assert_eq!(sink.health().dropped, 0);
        assert_eq!(sink.health().last_error, None);

        brittle.refuse();

        for _ in 0..7 {
            assert_eq!(
                sink.emit(&Event::error("t", "after")),
                Emitted::Dropped,
                "a refused write is reported to its caller as dropped"
            );
        }
        let health = sink.health();
        assert_eq!(health.dropped, 7, "every dropped event is counted, exactly");
        assert_eq!(health.written, 1, "and none of them is counted as written");
        assert!(health.is_loud(), "the sink says it is unhealthy");
        let said = health.last_error.expect("a reason in its own words");
        assert!(
            said.contains(&current_path(&dir).display().to_string()),
            "the reason names the path: {said}"
        );
        assert!(
            said.contains("no space left on device"),
            "and repeats what the destination said: {said}"
        );

        // `sync` is the one call that DOES return its failure, because a
        // caller asking for durability has to be told it did not get it.
        let said = sink.sync().expect_err("a refused sync is returned");
        assert!(said.contains("written and not synced"), "{said}");

        // THE CALLER IS STILL ALIVE, and recovers the moment the destination
        // does. Nothing latched, nothing poisoned.
        brittle.refusing.store(false, Ordering::Relaxed);
        assert_eq!(sink.emit(&Event::info("t", "recovered")), Emitted::Written);
        assert_eq!(sink.health().written, 2);
        assert_eq!(
            sink.health().dropped,
            7,
            "the count is a total, not a gauge"
        );
    }

    /// A ROLL THAT CANNOT HAPPEN IS REPORTED, AND THE EVENT IS STILL WRITTEN.
    #[test]
    fn a_roll_that_fails_is_counted_and_the_event_is_written_anyway() {
        let dir = scratch("roll-fails");
        let brittle = Arc::new(Brittle::default());
        let sink = Sink::with_target(
            &Config::new(&dir)
                .with_max_file_bytes(1024)
                .with_keep_files(2),
            Box::new(Arc::clone(&brittle)),
        )
        .expect("opens");

        // THE LINE HAS TO BE MORE THAN HALF THE BOUND, and a long message
        // alone cannot get there. `emit` rolls when
        // `bytes + span > max_file_bytes`, so "every event after the first
        // rolls" needs `2 * span > 1024`, i.e. a line over 512 bytes.
        //
        // A 600-character message does NOT produce one: `MAX_MESSAGE_BYTES` is
        // 256, so the message is cut to 256 and the whole line lands near 380.
        // That is why this test asked for 4 rotations and got 2 — the premise
        // in the comment was arithmetic that the message ceiling had already
        // made false. The bound cannot come down to meet it either;
        // `MIN_FILE_BYTES` is 1024 and `Config::validate` refuses less.
        //
        // So the line is fattened where there is still room: two string fields
        // at `MAX_STR_VALUE_BYTES` each, which are not cut and add ~128 bytes
        // apiece on top of the 256-byte message.
        let fat = "x".repeat(MAX_MESSAGE_BYTES);
        let wide = "y".repeat(MAX_STR_VALUE_BYTES);
        let emit_fat = |sink: &Sink| {
            sink.emit(
                &Event::info("t", &fat)
                    .with("pad_a", wide.as_str())
                    .with("pad_b", wide.as_str()),
            )
        };

        assert!(emit_fat(&sink).is_written());
        // MEASURED, NOT ASSUMED. The sink's own running count is the span of
        // the one line written so far, so this checks the premise every
        // assertion below rests on instead of trusting a number in a comment.
        let span = sink.health().current_bytes;
        assert!(
            span.saturating_mul(2) > 1024,
            "the line must be over half the bound or nothing rolls: span {span}"
        );

        for _ in 0..4 {
            assert!(emit_fat(&sink).is_written());
        }
        assert_eq!(sink.health().rotations, 4, "it was rolling happily");
        assert_eq!(sink.health().rotation_failures, 0);

        brittle.refuse();
        // Every one of these attempts a roll, and every roll now fails at
        // `reopen`. The event is attempted anyway — losing it because a RENAME
        // failed would be the worse of the two — and is Dropped here only
        // because the append refuses as well.
        for _ in 0..5 {
            assert_eq!(emit_fat(&sink), Emitted::Dropped);
        }
        let health = sink.health();
        assert_eq!(
            health.rotation_failures, 5,
            "a roll that could not happen is counted, every time: {health:?}"
        );
        assert_eq!(health.dropped, 5, "and every event is still accounted for");
        assert_eq!(health.rotations, 4, "no failed roll was counted as a roll");
        assert!(health.is_loud());
        assert!(
            health
                .last_error
                .as_deref()
                .is_some_and(|said| said.contains("cannot append the event")),
            "the most recent failure is the most recent one: {health:?}"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// N THREADS PRODUCE EXACTLY N WHOLE LINES.
    ///
    /// The failure this rules out is interleaving: two writes whose bytes land
    /// inside one another produce a file where neither line decodes and the
    /// count is still right. So the assertions are (a) every line decodes,
    /// (b) the count is exact, and (c) the sequence numbers are exactly
    /// `1..=N` with no duplicate and no gap — which is only true if the number
    /// and the bytes were assigned under the same lock.
    #[test]
    fn many_threads_produce_exactly_one_whole_line_each_and_no_interleaving() {
        const THREADS: u64 = 16;
        const EACH: u64 = 250;
        let dir = scratch("threads");
        let sink = Arc::new(Sink::open(&Config::new(&dir)).expect("opens"));

        std::thread::scope(|scope| {
            for t in 0..THREADS {
                let sink = Arc::clone(&sink);
                scope.spawn(move || {
                    for i in 0..EACH {
                        // A long payload carrying quotes and a newline,
                        // because a short line can fit in one atomic write by
                        // luck and prove nothing.
                        let payload = format!(
                            "thread {t} item {i} \"quoted\"\nwith a newline and a backslash \
                             \\ and padding {}",
                            "x".repeat(64)
                        );
                        assert!(
                            sink.emit(
                                &Event::info("thread", &payload)
                                    .with("thread", t)
                                    .with("item", i)
                            )
                            .is_written()
                        );
                    }
                });
            }
        });

        let records = lines_of(&current_path(&dir));
        assert_eq!(
            u64::try_from(records.len()).unwrap(),
            THREADS * EACH,
            "every line is whole, and there are exactly as many as were emitted"
        );
        let mut seen: Vec<u64> = records.iter().map(|r| r.seq).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            u64::try_from(seen.len()).unwrap(),
            THREADS * EACH,
            "no sequence number was issued twice"
        );
        assert_eq!(seen.first(), Some(&1));
        assert_eq!(seen.last(), Some(&(THREADS * EACH)));

        // Every (thread, item) pair appears exactly once, so no event was lost
        // and none was written twice.
        let mut pairs: Vec<(i64, i64)> = records
            .iter()
            .filter_map(|r| Some((r.field("thread")?.as_i64()?, r.field("item")?.as_i64()?)))
            .collect();
        pairs.sort_unstable();
        pairs.dedup();
        assert_eq!(u64::try_from(pairs.len()).unwrap(), THREADS * EACH);
        assert_eq!(sink.health().written, THREADS * EACH);
        assert_eq!(sink.health().dropped, 0);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A RESTART CONTINUES THE STREAM RATHER THAN STARTING A SECOND ONE.
    #[test]
    fn a_reopened_sink_carries_on_from_the_last_line_it_can_read() {
        let dir = scratch("resume");
        {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            for _ in 0..5 {
                assert!(sink.emit(&Event::info("t", "m")).is_written());
            }
            assert_eq!(sink.health().next_seq, 6);
        }
        let again = Sink::open(&Config::new(&dir)).expect("re-opens");
        assert_eq!(
            again.health().next_seq,
            6,
            "the numbering continues where it stopped"
        );
        assert!(again.emit(&Event::info("t", "m")).is_written());
        let records = lines_of(&current_path(&dir));
        assert_eq!(records.len(), 6);
        assert_eq!(records[5].seq, 6);

        // A file whose tail will not decode restarts at zero, which is a
        // stated limit rather than a silent one.
        std::fs::write(current_path(&dir), b"not this format at all\n").expect("clobbered");
        let third = Sink::open(&Config::new(&dir)).expect("re-opens");
        assert_eq!(third.health().next_seq, 1);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_debug_of_the_sink_takes_no_lock_and_still_says_the_useful_things() {
        let dir = scratch("debug");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        let said = format!("{sink:?}");
        assert!(said.contains("Sink"), "{said}");
        assert!(said.contains("dropped"), "{said}");
        assert!(said.contains("keep_files"), "{said}");
        let _ignored = std::fs::remove_dir_all(&dir);
    }
}
