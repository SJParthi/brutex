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
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
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

/// The most per-subsystem level overrides one sink will hold.
///
/// Bounded so the lookup is bounded: resolving a target walks this array at
/// most once, so the cost is a constant and not a function of how many
/// subsystems exist. Eight is more than the taxonomy has targets today.
pub const MAX_TARGET_LEVELS: usize = 8;

/// Largest integer JSON/JavaScript transports without rounding.
const MAX_SAFE_RUN_ID: u64 = 9_007_199_254_740_991;

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
    /// No sink is installed. [`crate::emit`] and [`crate::emit_for_run`] return
    /// this; methods on an already-open [`Sink`] cannot.
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
    /// Per-subsystem floors, longest dotted prefix wins.
    ///
    /// Empty is the default and costs nothing: with no overrides the hot path
    /// is the same single relaxed atomic load it always was.
    pub target_levels: Vec<(String, Level)>,
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
            target_levels: Vec::new(),
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
    /// The same configuration with a floor for ONE subsystem and everything
    /// beneath it.
    ///
    /// `("pull", Level::Debug)` covers `pull`, `pull.http` and
    /// `pull.http.retry` — and never `pullover`, because the prefix match
    /// requires the dot. The longest matching prefix wins, so
    /// `("pull", Debug)` beside `("pull.chunk", Trace)` gives the chunk path
    /// its own floor without lifting the rest of the crate to `Trace`.
    ///
    /// # Why this exists
    ///
    /// With one global floor an operator has two settings: `Info`, which hides
    /// the per-member detail that explains a failure, and `Debug`, which turns
    /// on every `api.request` line as well. On a 62,600-member backfill the
    /// second is not a choice — it rolls the run's own beginning out of the
    /// window. Per-subsystem is what makes "verbose for the pull, quiet for
    /// everything else" expressible.
    ///
    /// Silently ignored past [`MAX_TARGET_LEVELS`]: a bounded array is what
    /// keeps the lookup constant, and an operator who writes nine overrides
    /// gets the first eight rather than a refusal to start.
    pub fn with_target_level(mut self, target: impl Into<String>, level: Level) -> Self {
        if self.target_levels.len() < MAX_TARGET_LEVELS {
            self.target_levels.push((target.into(), level));
        }
        self
    }

    /// The quietest level ANY event could need to pass, across the global floor
    /// and every override.
    ///
    /// This is the number the hot path compares against, so that a config with
    /// no overrides costs exactly what it cost before this feature existed:
    /// one relaxed atomic load and a comparison.
    fn fast_floor(&self) -> Level {
        lowest_floor(self.min_level, &self.target_levels)
    }

    /// The same configuration with a different global floor.
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
        // THE BOUND THAT MAKES `level_for` O(1), CHECKED WHERE IT CANNOT BE
        // BYPASSED.
        //
        // `with_target_level` already refuses to push past the ceiling — but
        // `target_levels` is a PUBLIC field, so `Config { target_levels: v, .. }`
        // and `config.target_levels.push(..)` both walk straight around the
        // builder. `level_for` walks this table on every emit that clears the
        // fast floor, and `CLAUDE.md` §3 rule 4 wants that walk bounded by a
        // constant rather than by whatever a caller happened to assemble.
        //
        // A builder that enforces an invariant and a public field that does not
        // is an invariant enforced by politeness. This is the same check, at the
        // one place every construction path has to pass through.
        if self.target_levels.len() > MAX_TARGET_LEVELS {
            return Some(format!(
                "{} per-target overrides is past the {MAX_TARGET_LEVELS}-override \
                 ceiling: `Sink::level_for` walks this table on every event that \
                 clears the fast floor, and the walk is only constant-time \
                 because the table is bounded",
                self.target_levels.len()
            ));
        }
        None
    }
}

/// The quietest level ANY event could need, across a global floor and a set of
/// per-target overrides.
///
/// **ONE IMPLEMENTATION, CALLED BY BOTH WRITERS OF THE PAIR** — by
/// [`Config::fast_floor`] at construction and by [`Sink::set_min_level`] at
/// runtime. "The fast floor is the minimum of the global floor and every
/// override" is precisely the invariant [`Sink::floors`] packs into one word to
/// protect, and an invariant with two copies of its arithmetic has two chances
/// to drift. The two copies were real: this body was duplicated verbatim inside
/// `set_min_level`.
///
/// `min_by_key` RATHER THAN A HAND-WRITTEN COMPARISON, and the reason is a
/// measurement. `if level.rank() < lowest.rank()` mutates to `<=` with
/// **identical behaviour**: `rank` is injective, so equal ranks mean equal
/// levels and both arms return the same value. That is an equivalent mutant —
/// one no test can ever kill — and CI gate 18 would have reported it as a
/// surviving mutant forever. Moving the comparison into `std` removes the
/// mutable operator instead of suppressing the finding.
fn lowest_floor(global: Level, overrides: &[(String, Level)]) -> Level {
    core::iter::once(global)
        .chain(overrides.iter().map(|&(_, level)| level))
        .min_by_key(|level| level.rank())
        .unwrap_or(global)
}

/// The two floors as ONE word: the global floor in the high byte, the fast
/// floor in the low byte.
///
/// Through `to_be_bytes`/`from_be_bytes` rather than a shift and a cast because
/// `clippy::cast_possible_truncation` is denied workspace-wide and the shift
/// form needs exactly the truncating cast it names. The bytes form compiles to
/// the same instruction and cannot be wrong about which half is which.
const fn packed(global: Level, fast: Level) -> u16 {
    u16::from_be_bytes([global.rank(), fast.rank()])
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
    /// The `ms` of the last event written, and the floor the next one is
    /// clamped to.
    ///
    /// **This exists so that `ms` order IS file order.** The clock used to be
    /// read before the lock was taken, which put `seq` in lock-acquisition
    /// order and `ms` in pre-lock order — two orders that disagree the moment
    /// one thread is preempted between the two lines. `tail`'s `since` filter
    /// ends the whole walk at the first record older than the floor, on the
    /// stated grounds that "events are in time order", so a single inversion
    /// silently truncated the result and `Tail::missing` could not report it:
    /// `missing_between` returns `None` whenever a `since` filter is present.
    ///
    /// Clamping rather than merely moving the call also absorbs a clock STEPPED
    /// BACKWARDS by NTP, which moving it alone would not. The cost is one
    /// comparison and one store, both inside a critical section that already
    /// formats and appends the line.
    last_at: i64,
    /// Rendered here and reused. Cleared, never freed.
    buf: Vec<u8>,
}

impl Inner {
    /// The timestamp for the next line: never earlier than the last one's.
    ///
    /// Split out of [`Sink::emit`] so that a clock which STEPS BACKWARDS can be
    /// tested without owning one. `now_millis` reads the host clock, and a test
    /// cannot move the host clock — so the only way to prove the clamp is to
    /// hand it the reading directly. Proved by
    /// `a_backward_clock_cannot_move_ms_backwards`.
    ///
    /// One comparison and one store. O(1), and called with the lock already
    /// held — it adds a fixed pair of instructions to a critical section that
    /// already formats and appends a line. The flatness of the emit path
    /// carrying it is held by
    /// `telemetry::bench::the_tail_is_flat_in_the_size_of_the_file_too` (C-T-01b),
    /// the same proof the `run` stamp beside it names.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    fn stamp(&mut self, now: i64) -> i64 {
        self.last_at = now.max(self.last_at);
        self.last_at
    }
}

/// The event stream's writer.
///
/// Thread-safe and meant to be shared: one per process behind
/// [`crate::install`], or one per test behind an `Arc`.
pub struct Sink {
    dir: PathBuf,
    max_file_bytes: u64,
    keep_files: u8,
    /// Per-subsystem floors, longest dotted prefix wins. Empty is the default.
    ///
    /// Read-only after construction, so no lock and no atomic: the hot path
    /// borrows it and walks at most [`MAX_TARGET_LEVELS`] entries.
    target_levels: Vec<(String, Level)>,
    /// The run every event is stamped with, or zero for none.
    ///
    /// **A LOG SPANNING THREE BACKFILLS CANNOT BE SPLIT INTO THREE WITHOUT
    /// THIS**, and splitting it is the first thing a reader who did not run the
    /// job has to do. Held on the sink rather than threaded through every
    /// call site for the reason `emit` reads a global at all: a parameter on
    /// every function between `main` and a note helper is a parameter somebody
    /// forgets, and the one they forget is the site being diagnosed.
    ///
    /// One relaxed atomic load per event, which is the same cost as the level
    /// gate beside it. O(1), and it cannot grow — the flatness of the emit path
    /// with this load on it is held by
    /// `telemetry::bench::the_tail_is_flat_in_the_size_of_the_file_too` (C-T-01b),
    /// and the split it buys by
    /// `telemetry::tail::a_log_holding_several_runs_splits_back_into_them` (T-22).
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    run: AtomicU64,

    /// The last opaque correlation id reserved from this log's durable
    /// sequence space.
    ///
    /// Initialised from the last readable sequence number when the sink opens.
    /// A caller that reserves an id and then writes at least one event carrying
    /// it therefore makes the next process start strictly above every id still
    /// present in the log. If no such event lands, there is no old record for a
    /// later reuse to collide with. This is the exact attempt boundary used by
    /// the sweep monitor; it does not infer identity from a wall clock.
    reserved_run: AtomicU64,

    /// Set once a roll has failed, and never cleared.
    ///
    /// **THE WINDOW IS DESTROYED BY RE-ATTEMPTING, NOT BY FAILING ONCE.**
    /// `roll` unlinks the oldest file, renames the middle ones up, and only
    /// then moves the current file aside. Every one of those steps can fail,
    /// and each returns early — leaving `inner.bytes` past the bound, so the
    /// NEXT event meets the same roll condition and shifts the whole set again.
    /// And the next. Measured against a rename that could not complete, with
    /// `keep_files = 5`: file sizes went `[930, 927, 926, 926, 926]` to
    /// `[1862, 927, 926, 0, 0]` in two events — the retained history emptied
    /// one file per event while the sink reported only that rolling had failed.
    ///
    /// So the first failure stops rotation for the life of the sink. The
    /// current file then grows past its bound, which is the documented
    /// degradation and is visible in [`Health::current_bytes`] — and the events
    /// already on disk survive, which is the whole point of keeping them.
    rotation_broken: AtomicBool,

    /// **BOTH FLOORS, IN ONE WORD, BECAUSE TWO WORDS COULD DISAGREE.**
    ///
    /// High byte: the global floor, which is what [`Sink::min_level`] reports.
    /// Low byte: the fast floor, `min(global, every override)`, which is the
    /// ONLY thing [`Sink::emit`]'s first test reads — an event below it is
    /// rejected on one relaxed atomic load, exactly as before per-subsystem
    /// floors existed, so a sink with no overrides pays nothing for the
    /// feature. Only an event that passes it is worth resolving a specific
    /// target for.
    ///
    /// **THEY WERE TWO ATOMICS AND `set_min_level` WROTE THEM ONE AFTER THE
    /// OTHER.** Two callers racing interleave those four stores, and one of the
    /// orderings leaves the pair describing two *different* floors — for the
    /// life of the sink, or until somebody sets the level again. A
    /// barrier-synchronised probe on this tree, reading only at the instant no
    /// thread was writing, saw it at rounds 2023, 2597, 5275 and 6269 of four
    /// million: `min_level()` reporting `error` while a `trace` event was still
    /// admitted, and the reverse. Rare, real, and silent — the operator who
    /// lowered the floor gets no debug lines and a `min_level()` that agrees
    /// with what they asked for.
    ///
    /// One word cannot tear: every state a reader can observe is a word some
    /// `set_min_level` wrote, whole. **It costs `emit` nothing** — the fast
    /// path is one relaxed atomic load either way, and taking the low byte is a
    /// truncation the compiler folds into the comparison it already made.
    ///
    /// *A `Mutex` around the pair was the other candidate and is the larger
    /// change for the same result.* It would sit on the level-setting path
    /// rather than the emit path, so its cost is paid by a rare caller — but it
    /// adds a third lock to a type whose `Debug` already refuses to take the
    /// first two, and it fixes only the durable crossing: a reader that loads
    /// the two atomics separately can still see them mid-flight, and closing
    /// *that* means taking the lock in `emit`, which is the one place this
    /// crate cannot afford one. Packing removes both windows and adds no lock.
    /// See D-0101.
    floors: AtomicU16,
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
    /// `resume_seq`.
    ///
    /// A file that does not end in a newline is **terminated before the first
    /// append**, so the record the previous process was killed in the middle
    /// of stays one line of its own rather than being fused onto. See
    /// `terminate_torn_tail`, which also says what that does not recover.
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
        let mut target = FileTarget::open(&path)
            .map_err(|e| format!("{}: cannot open the event stream — {e}", path.display()))?;
        let found = target.len();
        let seq = resume_seq(&path);
        // THE TORN TAIL IS CLOSED BEFORE THE FIRST APPEND. See
        // `terminate_torn_tail`: without this the first event of the new
        // process fuses onto whatever the old one was killed in the middle of.
        let (bytes, torn) = terminate_torn_tail(&mut target, &path, found);
        let sink = Self::around(config, Box::new(target), bytes, seq);
        if let Some(why) = torn {
            sink.report(&why);
        }
        Ok(sink)
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
            target_levels: config.target_levels.clone(),
            run: AtomicU64::new(0),
            reserved_run: AtomicU64::new(seq),
            rotation_broken: AtomicBool::new(false),
            floors: AtomicU16::new(packed(config.min_level, config.fast_floor())),
            inner: Mutex::new(Inner {
                target,
                bytes,
                seq,
                // ZERO, NOT `now_millis()`. A resumed sink must not claim the
                // events already in the file happened at the moment it opened;
                // the first event written clamps against a floor of zero, which
                // any real clock clears.
                last_at: 0,
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
    /// The floor that applies to one target: the LONGEST matching dotted
    /// prefix among the overrides, or the global floor when none matches.
    ///
    /// `"pull"` covers `pull`, `pull.http` and `pull.http.retry`; it does NOT
    /// cover `pullover`, because the match requires the dot — the same rule
    /// [`crate::tail::Query::target`] already uses, so a filter written for
    /// the reader means the same thing as one written for the writer.
    ///
    /// Longest wins so `("pull", Debug)` and `("pull.chunk", Trace)` compose:
    /// the chunk path gets its own floor without lifting the rest of the
    /// crate. Bounded by [`MAX_TARGET_LEVELS`], so this is a constant number
    /// of prefix comparisons of a bounded-length target.
    pub fn level_for(&self, target: &str) -> Level {
        let mut best: Option<(usize, Level)> = None;
        for (prefix, level) in &self.target_levels {
            // NO LENGTH GUARD, BECAUSE THE DOT ALREADY IS ONE. A byte at
            // `prefix.len()` exists only when `target` is strictly longer, so
            // `target.len() > prefix.len()` was implied by the line below it
            // and never decided anything — CI gate 18 found it by mutating `>`
            // to `>=` and watching the whole suite stay green. Redundant code
            // removed rather than the finding suppressed.
            let covers = target == prefix
                || (target.starts_with(prefix.as_str())
                    && target.as_bytes().get(prefix.len()) == Some(&b'.'));
            // `>=`, SO THE LAST REGISTRATION OF A TARGET WINS.
            //
            // `with_target_level` does not reject a repeat, so two entries can
            // carry the same prefix — and until CI gate 18 mutated this
            // operator, nothing decided which of them applied. `>=` makes a
            // later call override an earlier one, which is the useful
            // direction: a caller building a config in layers expects the last
            // word to be the one that counts. Pinned by
            // `a_repeated_target_takes_its_last_registration`.
            if covers && best.is_none_or(|(len, _)| prefix.len() >= len) {
                best = Some((prefix.len(), *level));
            }
        }
        best.map_or_else(|| self.min_level(), |(_, level)| level)
    }

    /// The global floor, as one relaxed load.
    ///
    /// The stored rank is always one this crate wrote — the only writers are
    /// construction and [`Sink::set_min_level`], both of which take a
    /// [`Level`] — so the `unwrap_or` is a backstop no caller can drive.
    #[must_use]
    pub fn min_level(&self) -> Level {
        let [global, _fast] = self.floors.load(Ordering::Relaxed).to_be_bytes();
        Level::of_rank(global).unwrap_or(Level::Trace)
    }

    /// The lowest floor any target could have, as one relaxed load.
    ///
    /// The low half of [`Sink::floors`], and the whole of `emit`'s fast reject.
    /// The same backstop applies for the same reason.
    fn fast_floor(&self) -> Level {
        let [_global, fast] = self.floors.load(Ordering::Relaxed).to_be_bytes();
        Level::of_rank(fast).unwrap_or(Level::Trace)
    }

    /// Changes the floor, for every thread, without a lock.
    ///
    /// Deliberately adjustable at runtime: an operator watching a backfill go
    /// wrong should be able to turn debug on without restarting the process
    /// that is halfway through it.
    pub fn set_min_level(&self, level: Level) {
        // AND THE FAST FLOOR WITH IT, OR THE MOVE DOES NOTHING.
        //
        // `emit`'s first test is the fast floor, so leaving it behind here
        // makes a runtime lowering silently ineffective — the sink reports the
        // new `min_level()` and keeps filtering at the old one.
        // `an_event_below_the_floor_is_filtered_and_never_reaches_the_file`
        // caught exactly that: it lowers the floor to `Trace` and asserts the
        // next `trace` event is written.
        //
        // The fast floor is the MINIMUM of the global floor and every override,
        // so a subsystem pinned lower than the new global keeps its own floor
        // reachable.
        //
        // **AND BOTH IN ONE STORE.** This was two stores into two atomics, and
        // two callers racing interleaved them into a pair that described two
        // different floors — see `Sink::floors`, which is one word for exactly
        // this reason. There is no interleaving of a single relaxed store, so
        // the last writer's pair is the pair, whole.
        self.floors.store(
            packed(level, lowest_floor(level, &self.target_levels)),
            Ordering::Relaxed,
        );
    }

    /// Stamps every later event with `run`. Zero clears it.
    ///
    /// Set when a run begins and cleared when it ends, so events outside a run
    /// — a served request, a startup line — carry no run and say so by
    /// omission rather than by a zero a reader has to interpret.
    ///
    /// # This is a STORE, and a store is wrong when two runs overlap
    ///
    /// It is kept because a single-run process is the ordinary case and this is
    /// the honest primitive for it. When runs can overlap — `api::pullrun`
    /// spawns one chain per vendor and every one of them reaches a pull — the
    /// last writer wins and the first finisher clears the key for everybody, so
    /// `/logs?run=` groups events into stories that never happened. Use
    /// [`Self::claim_run`] there.
    pub fn set_run(&self, run: u64) {
        self.run.store(run, Ordering::Relaxed);
    }

    /// Takes the run key **only if nothing holds it**, answering whether it did.
    ///
    /// # The stories that never happened
    ///
    /// `set_run` is a bare store, and `api::server::broker_run` called it once
    /// per LEG while `api::pullrun::conduct` runs one chain per vendor
    /// concurrently. So two feeds in one press overwrote each other's key, every
    /// event after the second write carried the second feed's id — including the
    /// first feed's — and whichever finished first cleared the key to zero,
    /// after which the survivor's remaining events carried no run at all.
    ///
    /// A reader grouping by `run` therefore saw one story assembled from two
    /// feeds and a second story that stopped mid-sentence. `record.rs` calls
    /// this field *"the key a reader groups by"*, and it was the one field that
    /// could not be trusted for exactly the runs worth reading.
    ///
    /// # Why a claim rather than a task-local
    ///
    /// The obvious repair is per-task storage, and this crate cannot have it:
    /// `CLAUDE.md` §5 puts `telemetry` among the crates that **depend on
    /// nothing**, and a task-local means a runtime dependency. Threading the id
    /// through every `emit` is the other repair and is the one this sink exists
    /// to avoid — its own comment says a parameter on every function between
    /// the caller and a leaf is one somebody forgets, and the site they forget
    /// is the one being diagnosed.
    ///
    /// A claim needs neither. The OUTERMOST scope that knows a run has begun
    /// takes the key, every concurrent leg beneath it inherits that one id —
    /// which is correct, because they are one press — and a leg that finds the
    /// key already held does not take it and does not release it.
    ///
    /// # Cost
    ///
    /// One `compare_exchange`. O(1), lock-free, and exact: two threads racing
    /// to claim cannot both win, which a `load`-then-`store` cannot promise.
    ///
    /// Claiming zero is refused — zero is the absence of a run, so a caller
    /// asking for it is asking to hold nothing.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    pub fn claim_run(&self, run: u64) -> bool {
        run != 0
            && self
                .run
                .compare_exchange(0, run, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
    }

    /// Releases the run key, **only if this caller still holds it**.
    ///
    /// The other half of [`Self::claim_run`], and the half that stops a leg
    /// clearing a key it never took. A caller that lost the claim passes the id
    /// it wanted and nothing happens, which is the outcome it wants: somebody
    /// else's run is still in flight and its events must keep their key.
    ///
    /// # Cost
    ///
    /// One `compare_exchange`. O(1).
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    pub fn release_run(&self, run: u64) {
        let _lost_the_claim =
            self.run
                .compare_exchange(run, 0, Ordering::AcqRel, Ordering::Acquire);
    }

    /// The run every event is currently stamped with, or zero for none.
    #[must_use]
    pub fn run(&self) -> u64 {
        self.run.load(Ordering::Relaxed)
    }

    /// Reserves one non-zero run id from the log's resumed sequence space.
    ///
    /// [`None`] after exhausting JavaScript's exact-integer range; rounding on
    /// the browser boundary would let two distinct attempts compare equal, so
    /// exhaustion is a refusal rather than a rollover.
    #[must_use]
    pub fn reserve_run_id(&self) -> Option<u64> {
        self.reserved_run
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |held| {
                held.checked_add(1).filter(|next| *next <= MAX_SAFE_RUN_ID)
            })
            .ok()
            .and_then(|held| held.checked_add(1))
    }

    /// Whether an event at `level` from `target` would be written.
    ///
    /// **THE GATE, WITHOUT AN EVENT TO GATE.** [`Sink::emit`] is a function, so
    /// by the time it can look at the level the caller has already built the
    /// whole `Event` — a 12-slot array initialised on the stack, plus every
    /// argument expression evaluated. The level check is cheap; getting to it
    /// is not.
    ///
    /// Measured, one binary, one harness: a filtered event through
    /// `Sink::emit` cost **6,750 ps with no fields and 14,146 ps with three** —
    /// it DOUBLES with the number of fields at the call site. That is
    /// O(call-site fields), and `CLAUDE.md` §3 rule 4 asks for O(1).
    ///
    /// This is the same two checks `emit` runs, exposed so [`crate::emit_if`]
    /// can run them BEFORE the arguments are evaluated. It reads one relaxed
    /// atomic and, only when overrides exist, walks at most
    /// [`MAX_TARGET_LEVELS`] bounded prefixes — O(1) in everything, including
    /// how many fields the caller was about to attach. Proved by
    /// `telemetry::sink::a_filtered_event_never_evaluates_its_arguments`, which
    /// counts side effects rather than timing them: an argument that did not
    /// run cannot increment a counter.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    #[must_use]
    pub fn admits(&self, level: Level, target: &str) -> bool {
        if !level.at_least(self.fast_floor()) {
            return false;
        }
        self.target_levels.is_empty() || level.at_least(self.level_for(target))
    }

    /// Writes one event.
    ///
    /// Never panics and never propagates a failure; the module documentation
    /// says how a failure is surfaced instead.
    pub fn emit(&self, event: &Event<'_>) -> Emitted {
        self.emit_for_run(self.run.load(Ordering::Relaxed), event)
    }

    /// Writes one event under an explicit run id.
    ///
    /// Unlike [`Self::set_run`], this changes no shared state. Concurrent work
    /// can therefore stamp its own structural boundary without overwriting or
    /// clearing another run's key. Zero has the ordinary "outside a run"
    /// meaning used by [`Self::emit`].
    pub fn emit_for_run(&self, run: u64, event: &Event<'_>) -> Emitted {
        // THE FAST REJECT, UNCHANGED. One relaxed atomic load and a comparison
        // against the lowest floor any target could have. With no overrides
        // that value IS the global floor, so a sink that does not use the
        // feature pays exactly what it paid before the feature existed.
        //
        // The load is 16 bits wide now rather than 8 — both floors share one
        // word so they cannot be observed crossed, see `Sink::floors` — and
        // taking the low half is a truncation, not a branch.
        if !event.level().at_least(self.fast_floor()) {
            return Emitted::Filtered;
        }
        // AND ONLY THEN, THE SPECIFIC FLOOR. An event that cleared the lowest
        // bar may still be below its OWN subsystem's, which is the whole point
        // of a per-target level: `pull=debug` must not also lift `api.request`.
        // Bounded by `MAX_TARGET_LEVELS`, and skipped entirely when empty.
        if !self.target_levels.is_empty() && !event.level().at_least(self.level_for(event.target()))
        {
            return Emitted::Filtered;
        }
        // A POISONED LOCK IS RECOVERED FROM, NOT PROPAGATED. Poisoning means
        // another thread panicked while holding this mutex. What is behind it
        // is a destination, a byte count and a scratch buffer, none of which a
        // panic can leave in a state that makes the next line wrong — and
        // refusing to log because an earlier log panicked is the one behaviour
        // guaranteed to lose the events that explain the panic.
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let inner = &mut *guard;
        // THE CLOCK IS READ HERE, INSIDE THE LOCK, AND CLAMPED. Reading it
        // before the lock put `seq` in lock-acquisition order and `ms` in
        // pre-lock order; a thread preempted between the two lines then wrote a
        // line whose `ms` was lower than the line before it. `tail`'s `since`
        // filter ends the walk at the first record older than the floor because
        // it trusts time order, so one inversion silently truncated the answer.
        // See `Inner::last_at`. Proved deterministically by
        // `a_backward_clock_cannot_move_ms_backwards`, which feeds the clamp a
        // reading no real clock would give; `ms_never_goes_backwards_in_the_file`
        // races eight threads at it as a regression net, and being a race it
        // cannot prove an absence — recorded that way rather than as a proof.
        let at = inner.stamp(now_millis());
        inner.seq = inner.seq.saturating_add(1);
        inner.buf.clear();
        line(&mut inner.buf, inner.seq, at, run, event);
        let span = u64::try_from(inner.buf.len()).unwrap_or(u64::MAX);
        // CARRIED OUT OF THE CRITICAL SECTION, NOT REPORTED INSIDE IT.
        //
        // `report` writes to stderr, and stderr BLOCKS: piped to a reader that
        // has stopped reading, it fills the pipe buffer and the write parks
        // indefinitely. Doing that while this mutex is held would stall every
        // other thread that logs — turning a failed rename into a process-wide
        // freeze on the one path that is supposed to explain the failure.
        // `CLAUDE.md` §4 forbids a fallback that hides a failure; a fallback
        // that HANGS on one is worse.
        //
        // The counters stay inside: they are relaxed atomics and cannot block.
        // Only the notice moves. The other two `report` call sites already
        // dropped the guard first; this was the one that did not.
        let mut roll_failure: Option<String> = None;
        if !self.rotation_broken.load(Ordering::Relaxed)
            && inner.bytes > 0
            && inner.bytes.saturating_add(span) > self.max_file_bytes
            && let Err(why) = self.roll(inner)
        {
            // NEVER AGAIN, for this sink. See `rotation_broken`.
            self.rotation_broken.store(true, Ordering::Relaxed);
            // The roll failed, so the current file will exceed its bound.
            // The event is still written: losing it because a RENAME failed
            // would be the worse of the two, and a bound that has been
            // exceeded is visible in `health()`.
            self.rotation_failures.fetch_add(1, Ordering::Relaxed);
            roll_failure = Some(why);
        }
        let landed = inner.target.append(&inner.buf);
        match landed {
            Ok(()) => {
                inner.bytes = inner.bytes.saturating_add(span);
                drop(guard);
                if let Some(why) = roll_failure {
                    self.report(&why);
                }
                self.written.fetch_add(1, Ordering::Relaxed);
                Emitted::Written
            }
            Err(e) => {
                // THE FRAGMENT IS TERMINATED BEFORE THE LOCK IS RELEASED.
                //
                // `write_all` reports the error, not how many bytes reached the
                // file first. A disk that fills mid-line therefore leaves a
                // PARTIAL line with no newline — and the next event, appended at
                // that offset, fuses onto it. One unparseable line, and **two**
                // events lost where `dropped` counts one.
                //
                // Measured on a real 2 MB volume driven to genuine ENOSPC: 132
                // lines written, the file 14 bytes longer than the sink's own
                // count, the last byte not a newline; after space was freed,
                // three further events all returned `Written` and the reader
                // then found 134 records for 135 writes, with `dropped` still
                // saying 1.
                //
                // One byte closes it: the fragment becomes its own line, which
                // the reader counts as `malformed` — visible, and exactly one
                // event's worth — and the next event starts clean. If this
                // write fails too there is nothing further to lose; the file is
                // already unwritable and the next append will say so.
                let _terminated = inner.target.append(b"\n");
                drop(guard);
                // A ROLL THAT ALSO FAILED IS REPORTED FIRST, so the notice
                // names the failure that came first. `report` prints once per
                // sink, so with both failing this is the one an operator sees —
                // and the roll is the earlier and more explanatory of the two.
                if let Some(why) = roll_failure {
                    self.report(&why);
                }
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

/// Whether the file's last byte is anything but a newline.
///
/// One `open`, one `seek` and a one-byte `read`, at open time only — never on
/// the write path, which is the row of the cost table in this module's header
/// that says the rotation check never reaches for `metadata`.
///
/// **A read that fails answers "no".** This cannot tell a whole file from a
/// torn one without looking, and a guess in the other direction would append a
/// newline to a file whose last line is complete — turning a clean tail into a
/// blank line the reader then has to step over. Doing nothing is the answer
/// that cannot make an intact file worse.
///
/// That silence is not a fallback hiding a failure, because the same file is
/// unreadable to `crate::tail`, which names it in `Tail::errors` on every
/// query. A log nobody can read is already reported by the surface whose job
/// that is; this one does not report it a second time on a guess.
fn ends_mid_line(path: &Path, len: u64) -> bool {
    if len == 0 {
        return false;
    }
    let Ok(last) = crate::tail::read_at(path, len.saturating_sub(1), 1) else {
        return false;
    };
    // Compared as bytes rather than through `first()`, so there is no `None`
    // arm for a one-byte `read_exact` that cannot produce one — an arm no test
    // could ever reach is a region the coverage gate would carry forever.
    last != b"\n"
}

/// Closes a torn tail before anything is appended to it, and says so.
///
/// Returns the running byte count the sink starts from, and the notice naming
/// the tear when there was one.
///
/// # What this is for
///
/// `Sink::emit` already terminates a line its own `write_all` tore — a disk
/// that fills mid-line — because the next event appended at that offset would
/// otherwise fuse onto the fragment: one unparseable line, and **two** events
/// lost where `dropped` counts one. That fix cannot cover the other way a line
/// ends mid-write, because the process that would have written the byte is
/// gone: a `SIGKILL` between the iterations of a `write_all` that the kernel
/// split, or a power cut with the last page still in the cache.
///
/// `resume_seq` does not close it either, and reading it is what makes the gap
/// look shut: it finds the last COMPLETE line and carries on from there, so a
/// restart knows the fragment is there and appends past it anyway.
///
/// **Measured on this tree**, as the shape rather than on a real power cut —
/// three whole lines, a fragment appended by hand, reopen, emit one event:
/// the reader returned `records=3 malformed=1 partial_tail=false` while the
/// sink reported `written=1 dropped=0`. The new event was inside the corrupt
/// line, the writer believed it had landed, and nothing anywhere counted the
/// torn one. Two events behind one `malformed`, and a `written` that disagreed
/// with the file.
///
/// One byte at open closes it, exactly as one byte in `emit` closes the other
/// half: the fragment becomes a line of its own, which the reader counts in
/// [`crate::Tail::malformed`] — visible, and exactly one event's worth — and
/// the first event of the new process starts clean.
///
/// # What it does NOT recover, and does not pretend to
///
/// * **The torn event's own bytes.** They were never written. What is
///   recovered is the *next* event and the count.
/// * **Its sequence number.** `resume_seq` resumes from the last DECODABLE
///   line, so the number the torn event carried is handed to the next one and
///   `Tail::missing` reports no hole. That was true before this change and is
///   neither caused nor fixed by it; the honest reading of the file is "one
///   malformed line here", which is what the reader now says.
/// * **`Health::is_loud`.** There is no counter in [`Health`] for a record
///   torn by another process, and adding a field is a change to
///   `crates/api/src/logs.rs`, which builds a `Health` by exhaustive literal.
///   The notice reaches [`Health::last_error`] and `/logs.json`; the HTML
///   banner is keyed on `is_loud()` and stays quiet.
///
/// # Why it spends the one stderr notice
///
/// `Sink::report` prints once per sink and never again, so a torn tail at open
/// takes the notice a later write failure would have had. That is the right
/// way round: a write failure still reaches the page through `dropped` and
/// `is_loud()`, and this one does not reach the page at all. Loud where the
/// other surface is silent, per `CLAUDE.md` §4.
///
/// # Why the parameter is `&mut dyn Target`
///
/// `&mut dyn Target` RATHER THAN `&mut FileTarget`, for the reason [`Target`]
/// exists at all: the arm where the terminating byte itself cannot be written
/// is a full disk, and a developer's machine does not enter that state on
/// request. A refusing double does, and it is the same seam
/// `a_write_that_cannot_land_is_counted_and_named_rather_than_silently_lost`
/// already uses. The indirect call happens once, at open.
fn terminate_torn_tail(target: &mut dyn Target, path: &Path, len: u64) -> (u64, Option<String>) {
    if !ends_mid_line(path, len) {
        return (len, None);
    }
    match target.append(b"\n") {
        // The byte is part of the file now, so the running count owns it too —
        // a count that disagreed with the file would move the roll decision by
        // one byte for the life of the sink.
        Ok(()) => (
            len.saturating_add(1),
            Some(format!(
                "{}: the last line had no newline — a record torn by a kill or a power \
                 cut. It has been terminated, so it is ONE line the reader counts in \
                 Tail::malformed and the next event starts clean; the torn record's own \
                 bytes are gone and are not recoverable",
                path.display()
            )),
        ),
        // NOTHING FURTHER IS LOST BY CARRYING ON. The file was already
        // unwritable-or-worse and the very next append will say so in its own
        // words; refusing to open here would lose the events that explain it.
        Err(e) => (
            len,
            Some(format!(
                "{}: the last line had no newline and the terminating byte could not be \
                 written — {e}; the next event will fuse onto the torn record and both \
                 will read as one malformed line",
                path.display()
            )),
        ),
    }
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
        Config, DEFAULT_KEEP_FILES, DEFAULT_MAX_FILE_BYTES, Emitted, Health, Inner,
        MAX_TARGET_LEVELS, MIN_FILE_BYTES, Sink, Target, current_path, dir_beneath_store,
        paths_newest_first, rotated_path,
    };
    use crate::event::{Event, MAX_MESSAGE_BYTES, MAX_STR_VALUE_BYTES};
    use crate::level::{LEVELS, Level};
    use crate::record::Record;
    use crate::value::Value;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A directory no other live process will name, emptied first.
    ///
    /// The process id is in the name for the reason `api::scratch` gives: a
    /// fixed name in the shared temporary directory is a fixture two
    /// concurrent test binaries both claim.
    ///
    /// **It used to end "and this suite deletes what it creates", which was not
    /// true.** It emptied the directory it was ABOUT to use and left every
    /// directory from every earlier run behind — and with a fresh pid each run,
    /// nothing ever collided, so nothing was ever reclaimed. That sentence cost
    /// 2.5 GB across 9,958 directories on one machine in a day.
    /// [`crate::sweep_stale_scratch`] now makes it true of earlier runs.
    fn scratch(name: &str) -> PathBuf {
        // Clears what earlier RUNS left behind — see `sweep_stale_scratch`.
        // Emptying only the directory about to be used is what let 9,958
        // of them accumulate.
        crate::tests::sweep_stale_scratch();
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

    /// A destination that accepts everything and keeps nothing.
    ///
    /// [`Inner::stamp`] touches no destination, so the tests that exercise the
    /// clamp need an `Inner` without needing a file. Distinct from `Brittle`,
    /// which exists to REFUSE.
    #[derive(Debug)]
    struct NullTarget;

    impl Target for NullTarget {
        fn append(&mut self, _bytes: &[u8]) -> std::io::Result<()> {
            Ok(())
        }

        fn sync(&self) -> std::io::Result<()> {
            Ok(())
        }

        fn reopen(&mut self, _path: &Path) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// **`report` runs with the emit mutex RELEASED, proved by parking it.**
    ///
    /// The hazard is that `report` writes to stderr, and stderr blocks: piped
    /// to a reader that has stopped reading, the write parks until the pipe
    /// drains. Doing that under the emit mutex freezes every thread that logs.
    ///
    /// No portable test can block stderr. But `report`'s FIRST act is to take
    /// `last_error`, so holding that lock parks any thread inside `report` at a
    /// known point — the same park a full pipe would cause, reached by a door
    /// a test can actually close.
    ///
    /// With the notice still inside the critical section the emitting thread
    /// would hold `inner` while parked, and `try_lock` below would never
    /// succeed. That is exactly the freeze this checks for.
    #[test]
    fn a_failed_roll_reports_with_the_emit_lock_released() {
        use std::os::unix::fs::PermissionsExt as _;
        use std::sync::PoisonError;

        let dir = scratch("report-off-lock");
        let sink = Arc::new(
            Sink::open(
                &Config::new(&dir)
                    .with_max_file_bytes(MIN_FILE_BYTES)
                    .with_keep_files(3),
            )
            .expect("opens"),
        );

        let fat = "x".repeat(MAX_MESSAGE_BYTES);
        let wide = "y".repeat(MAX_STR_VALUE_BYTES);
        let emit_fat = move |sink: &Sink| {
            sink.emit(
                &Event::info("t", &fat)
                    .with("pad_a", wide.as_str())
                    .with("pad_b", wide.as_str()),
            )
        };

        // One natural roll first, so the failing one is the SECOND.
        for _ in 0..2 {
            assert!(emit_fat(&sink).is_written());
        }
        assert_eq!(sink.health().rotation_failures, 0, "nothing failed yet");

        // A read-only directory makes the rename refuse, which is a failed roll.
        let mut ro = std::fs::metadata(&dir).expect("the dir").permissions();
        ro.set_mode(0o555);
        std::fs::set_permissions(&dir, ro).expect("read-only");

        // PARK anything that reaches `report`.
        let held = sink
            .last_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        let writer = {
            let mine = Arc::clone(&sink);
            std::thread::spawn(move || emit_fat(&mine))
        };

        // The counter is bumped INSIDE the critical section, immediately before
        // the notice that now sits outside it — so once it moves, the writer is
        // either finishing its append or already parked in `report`.
        let mut spun = 0;
        while sink.rotation_failures.load(Ordering::Relaxed) == 0 && spun < 5_000 {
            std::thread::sleep(std::time::Duration::from_millis(1));
            spun += 1;
        }
        assert!(
            sink.rotation_failures.load(Ordering::Relaxed) > 0,
            "the premise: the roll had to fail for `report` to be reached"
        );

        // THE ASSERTION. Another thread must be able to take the emit mutex
        // while the writer is parked in `report`.
        let mut free = false;
        for _ in 0..5_000 {
            if sink.inner.try_lock().is_ok() {
                free = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // Release the park and let the writer finish before asserting, so a
        // failure reports rather than leaving a thread wedged.
        drop(held);
        let _outcome = writer.join().expect("the writer did not panic");
        let mut rw = std::fs::metadata(&dir).expect("the dir").permissions();
        rw.set_mode(0o755);
        std::fs::set_permissions(&dir, rw).expect("restore");

        assert!(
            free,
            "the emit mutex was still held while `report` was parked: a stderr \
             that blocks would freeze every thread that logs"
        );
    }

    /// **The override ceiling is enforced where the public field cannot dodge it.**
    ///
    /// `with_target_level` refuses to push past `MAX_TARGET_LEVELS`, but
    /// `Config::target_levels` is `pub`: a struct literal and a direct `push`
    /// both walk around the builder. `Sink::level_for` walks that table on every
    /// event clearing the fast floor, so an unbounded table is an unbounded
    /// per-event cost — the thing `CLAUDE.md` §3 rule 4 forbids.
    ///
    /// The refusal names the ceiling and the reason, rather than clamping
    /// silently: §4 bans a fallback that hides a failure.
    #[test]
    fn a_config_past_the_override_ceiling_is_refused_by_name() {
        let dir = scratch("too-many-overrides");

        // EXACTLY at the ceiling is fine — the boundary is `>`, not `>=`, and a
        // test that only used a huge number could not tell the two apart.
        let mut at = Config::new(&dir);
        at.target_levels = (0..MAX_TARGET_LEVELS)
            .map(|i| (format!("sub{i}"), Level::Debug))
            .collect();
        assert!(
            at.refusal().is_none(),
            "the ceiling itself is allowed, not refused"
        );
        assert!(
            Sink::open(&at).is_ok(),
            "and a sink at the ceiling opens normally"
        );

        // One past it is refused, by name.
        let mut past = Config::new(&dir);
        past.target_levels = (0..=MAX_TARGET_LEVELS)
            .map(|i| (format!("sub{i}"), Level::Debug))
            .collect();
        let said = past
            .refusal()
            .expect("one past the ceiling cannot be accepted");
        assert!(
            said.contains(&(MAX_TARGET_LEVELS + 1).to_string()),
            "the refusal names how many were offered: {said}"
        );
        assert!(
            said.contains("level_for"),
            "and names the walk that is only constant-time while bounded: {said}"
        );
        let refused = Sink::open(&past).expect_err("and `open` refuses it too");
        assert!(
            refused.contains("ceiling"),
            "the same reason reaches the caller: {refused}"
        );
    }

    /// **The clamp, against a clock that runs backwards.**
    ///
    /// `now_millis` reads the host clock and a test cannot move the host clock,
    /// so [`Inner::stamp`] takes the reading as an argument and this hands it a
    /// sequence no real clock would produce. Without the clamp the third call
    /// returns 999 and the file holds a line older than the one before it,
    /// which is exactly what ends `tail`'s `since` walk early.
    #[test]
    fn a_backward_clock_cannot_move_ms_backwards() {
        let mut inner = Inner {
            target: Box::new(NullTarget),
            bytes: 0,
            seq: 0,
            last_at: 0,
            buf: Vec::new(),
        };

        assert_eq!(inner.stamp(1_000), 1_000, "a fresh sink takes the clock");
        assert_eq!(inner.stamp(1_001), 1_001, "forward is taken verbatim");
        assert_eq!(
            inner.stamp(999),
            1_001,
            "a clock stepped BACKWARDS by NTP is clamped to the last stamp, \
             because `tail` ends its `since` walk at the first older record"
        );
        assert_eq!(inner.stamp(1_001), 1_001, "equal is not an inversion");
        assert_eq!(inner.stamp(1_002), 1_002, "and it moves on afterwards");
        assert_eq!(
            inner.last_at, 1_002,
            "the floor is the last value handed out, not the last one read"
        );
    }

    /// **The floor a resumed sink starts from is zero, not the current clock.**
    ///
    /// A sink reopened on an existing file must not claim the events already in
    /// it happened when it opened. Zero is below every real reading, so the
    /// first event after a resume takes the clock unchanged.
    #[test]
    fn a_resumed_sink_does_not_stamp_the_present_onto_the_past() {
        let dir = scratch("resume-stamp");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        let before = crate::clock::now_millis();
        assert_eq!(
            sink.emit(&Event::info("api.server", "first")),
            Emitted::Written
        );
        let written = lines_of(&current_path(&dir));
        assert_eq!(written.len(), 1);
        assert!(
            written[0].at_unix_millis >= before,
            "the first event after open takes the real clock, not a floor: \
             {} < {before}",
            written[0].at_unix_millis
        );
    }

    /// **Every line in the file is at least as new as the line before it, with
    /// eight threads racing.**
    ///
    /// This is the property `tail`'s `since` filter depends on, asserted end to
    /// end rather than on the helper alone. It is a race, so it cannot PROVE
    /// the absence of an inversion — it is a regression net, and the
    /// deterministic proof is `a_backward_clock_cannot_move_ms_backwards`.
    /// Recorded that way rather than claimed as more than it is.
    #[test]
    fn ms_never_goes_backwards_in_the_file() {
        let dir = scratch("ms-order");
        let sink = std::sync::Arc::new(Sink::open(&Config::new(&dir)).expect("opens"));

        let mut hands = Vec::new();
        for t in 0..8u32 {
            let mine = std::sync::Arc::clone(&sink);
            hands.push(std::thread::spawn(move || {
                for i in 0..40u32 {
                    let _ = mine.emit(&Event::info("api.request", "served").with("t", t * 100 + i));
                }
            }));
        }
        for h in hands {
            h.join().expect("no thread panicked");
        }

        let written = lines_of(&current_path(&dir));
        assert_eq!(written.len(), 8 * 40, "every event landed");
        for pair in written.windows(2) {
            assert!(
                pair[1].at_unix_millis >= pair[0].at_unix_millis,
                "seq {} has ms {} but seq {} before it has ms {} — an inversion \
                 like this ends `tail`'s `since` walk and returns nothing",
                pair[1].seq,
                pair[1].at_unix_millis,
                pair[0].seq,
                pair[0].at_unix_millis
            );
            assert!(
                pair[1].seq > pair[0].seq,
                "seq must be strictly increasing in file order"
            );
        }
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

    /// ONE SUBSYSTEM LOUD, THE REST QUIET — the whole reason per-target floors
    /// exist.
    ///
    /// With a single global floor an operator has two settings: `Info`, which
    /// hides the per-member detail that explains a failure, and `Debug`, which
    /// also turns on every `api.request` line. On a 62,600-member backfill the
    /// second is not a choice — it rolls the run's own beginning out of a
    /// 64 MiB window. This is the test that "verbose for the pull, quiet for
    /// everything else" is actually expressible.
    #[test]
    fn a_target_floor_lifts_one_subsystem_without_lifting_the_rest() {
        let dir = scratch("targetlevel");
        let sink = Sink::open(
            &Config::new(&dir)
                .with_min_level(Level::Info)
                .with_target_level("pull", Level::Debug),
        )
        .expect("opens");

        // The global floor is untouched and still reported as itself.
        assert_eq!(sink.min_level(), Level::Info);

        // The named subsystem, and everything BENEATH it, is now audible.
        assert_eq!(sink.emit(&Event::debug("pull", "member")), Emitted::Written);
        assert_eq!(
            sink.emit(&Event::debug("pull.member", "landed")),
            Emitted::Written,
            "a dotted child inherits its parent's floor"
        );
        assert_eq!(
            sink.emit(&Event::debug("pull.http.retry", "again")),
            Emitted::Written,
            "and so does a grandchild"
        );

        // EVERYTHING ELSE IS UNCHANGED. This is the half that makes the
        // feature worth having: `pull=debug` must not also lift api.request.
        assert_eq!(
            sink.emit(&Event::debug("api.request", "served")),
            Emitted::Filtered,
            "an unnamed subsystem keeps the GLOBAL floor"
        );
        // AND THE PREFIX MATCH REQUIRES THE DOT. `pull` must never swallow
        // `pullover` — a subsystem name that is a prefix of another one is an
        // accident waiting to happen, and the same rule `tail::Query` uses.
        assert_eq!(
            sink.emit(&Event::debug("pullover", "not ours")),
            Emitted::Filtered,
            "a prefix without the dot is a DIFFERENT subsystem"
        );

        assert_eq!(
            lines_of(&sink.path()).len(),
            3,
            "three written, two filtered"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **AN OVERRIDE CAN ALSO SILENCE, AND THAT IS THE HARDER DIRECTION.**
    ///
    /// Every other test here makes a subsystem LOUDER than the global floor.
    /// The reverse — raise one subsystem above the floor to shut it up — is the
    /// other half of what `Config::with_target_level`'s doc promises ("verbose
    /// for the pull, quiet for everything else"), and until this test nothing
    /// proved it worked.
    ///
    /// It is the harder direction because of the fast path. `fast_floor` is the
    /// **minimum** across the global floor and every override, so when an
    /// override is quieter than the floor the minimum is the floor itself, and
    /// `fast_floor` admits the event. Nothing is filtered until `level_for` is
    /// consulted afterwards. An `emit` that treated `fast_floor` as the whole
    /// decision — or that only consulted `level_for` when an override was
    /// lower — would pass every existing test in this file and write the very
    /// lines the operator asked to be rid of.
    ///
    /// This is the real operator request: on a 62,600-member backfill, run the
    /// pull at `debug` and silence the per-request log, which at `debug` is one
    /// line per asset fetch and drowns the run.
    #[test]
    fn a_target_floor_can_raise_one_subsystem_above_the_global_floor_and_silence_it() {
        let dir = scratch("silence");
        let sink = Sink::open(
            &Config::new(&dir)
                .with_min_level(Level::Debug)
                .with_target_level("api.request", Level::Error),
        )
        .expect("opens");

        assert_eq!(sink.min_level(), Level::Debug, "the global floor is Debug");
        assert_eq!(
            sink.level_for("api.request"),
            Level::Error,
            "and the override is QUIETER than it"
        );

        // The global floor still governs everything unnamed.
        assert_eq!(
            sink.emit(&Event::debug("pull.member", "landed")),
            Emitted::Written,
            "the rest of the workspace is audible at Debug"
        );

        // THE SILENCED SUBSYSTEM. Each of these passes `fast_floor` (Debug) and
        // must be stopped by `level_for` alone.
        for level in [Level::Debug, Level::Info, Level::Warn] {
            assert_eq!(
                sink.emit(&Event::new(level, "api.request", "served")),
                Emitted::Filtered,
                "{level} from a subsystem floored at Error must not be written,                  even though the GLOBAL floor would have admitted it"
            );
        }
        // ...and its children are silenced with it.
        assert_eq!(
            sink.emit(&Event::warn("api.request.slow", "took a while")),
            Emitted::Filtered,
            "a dotted child inherits the raised floor too"
        );
        // But the subsystem is not muted outright: at or above its own floor it
        // still speaks, which is the difference between quieting and losing.
        assert_eq!(
            sink.emit(&Event::error("api.request", "500")),
            Emitted::Written,
            "an Error from the silenced subsystem still reaches the file"
        );

        let lines = lines_of(&sink.path());
        assert_eq!(lines.len(), 2, "one pull line and one api.request error");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **THE TWO ROTATION BOUNDARIES, WHICH ARE BOTH `>` AND NOT `>=`.**
    ///
    /// The roll decision is `inner.bytes > 0 && inner.bytes + span >
    /// max_file_bytes`. Mutation testing found BOTH comparisons surviving —
    /// every existing rotation test drives the count well past the bound, so
    /// nothing pinned either edge, and either could have been `>=` for the life
    /// of the crate without a test noticing.
    ///
    /// They are different bugs and both are silent:
    ///
    /// * `bytes > 0` guards an EMPTY file. As `>=` a first event larger than
    ///   the whole bound would roll a zero-byte file out of the way before
    ///   writing — spending a rotation, and on a `keep_files` of 2 discarding
    ///   one of the two files the operator has, to make room in a file that was
    ///   already empty. The event does not fit either way; rolling first only
    ///   destroys history.
    /// * `bytes + span > max` decides an EXACT fit. As `>=` an event that fills
    ///   the file precisely to its bound would roll first and leave the previous
    ///   file one event short of its own ceiling, forever.
    #[test]
    fn a_first_event_larger_than_the_whole_bound_does_not_roll_an_empty_file() {
        let dir = scratch("rollempty");
        let sink = Sink::open(
            &Config::new(&dir)
                .with_max_file_bytes(MIN_FILE_BYTES)
                .with_keep_files(2),
        )
        .expect("opens");

        // A line genuinely larger than the whole bound. It has to be built from
        // the CEILINGS rather than one long string: `MAX_STR_VALUE_BYTES` caps a
        // field value at 128 bytes and `MAX_MESSAGE_BYTES` a message at 256, so
        // a single padded field silently stops growing — which is exactly how
        // the first version of this test passed while provoking nothing.
        let pad = "x".repeat(MAX_STR_VALUE_BYTES);
        let message = "m".repeat(MAX_MESSAGE_BYTES);
        let mut event = Event::error("roll", &message);
        for key in ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"] {
            event = event.with(key, Value::Str(&pad));
        }

        assert!(sink.emit(&event).is_written());
        let span = sink.health().current_bytes;
        // THE PREMISE, ASSERTED. Without this the test can quietly become a
        // test of nothing the moment a ceiling moves.
        assert!(
            span > MIN_FILE_BYTES,
            "the fixture must exceed the {MIN_FILE_BYTES}-byte bound to provoke \
             the guard at all; it is {span}"
        );

        assert_eq!(
            sink.health().rotations,
            0,
            "an EMPTY file must not be rolled: there is nothing to make room \
             for, and on keep_files=2 the roll would throw away half the history"
        );
        assert!(
            !rotated_path(&dir, 1).exists(),
            "and no rotated file was created"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// The other edge: an event that fits EXACTLY stays in the current file.
    ///
    /// The bound is DERIVED from a measured line rather than the line padded to
    /// a chosen bound — the first attempt did the latter and could not work,
    /// because `MAX_STR_VALUE_BYTES` truncates a field value at 128 bytes, so
    /// the padding silently stopped growing. Measuring one line on a probe sink
    /// and opening the real one at exactly twice that width makes the second
    /// event land precisely on the bound, with no arithmetic about the line
    /// format that this test has no business knowing.
    #[test]
    fn an_event_that_fills_the_file_exactly_to_its_bound_does_not_roll() {
        // Five padded fields, so one line comfortably clears MIN_FILE_BYTES / 2
        // and the derived bound is legal.
        let pad = "y".repeat(100);
        let event = || {
            Event::info("roll", "exact")
                .with("a", Value::Str(&pad))
                .with("b", Value::Str(&pad))
                .with("c", Value::Str(&pad))
                .with("d", Value::Str(&pad))
                .with("e", Value::Str(&pad))
        };

        let probe_dir = scratch("rollprobe");
        let span = {
            let probe = Sink::open(
                &Config::new(&probe_dir)
                    .with_max_file_bytes(MIN_FILE_BYTES * 64)
                    .with_keep_files(2),
            )
            .expect("opens");
            assert!(probe.emit(&event()).is_written());
            probe.health().current_bytes
        };
        assert!(
            span * 2 >= MIN_FILE_BYTES,
            "the derived bound {} must clear the {MIN_FILE_BYTES}-byte floor",
            span * 2
        );

        let dir = scratch("rollexact");
        let sink = Sink::open(
            &Config::new(&dir)
                .with_max_file_bytes(span * 2)
                .with_keep_files(2),
        )
        .expect("opens");

        assert!(sink.emit(&event()).is_written());
        assert_eq!(sink.health().current_bytes, span, "one line, measured");

        // THE EXACT FIT. `bytes + span == max`, which is not `> max`, so this
        // must NOT roll. Under `>=` it would, and the previous file would be
        // left one event short of its own ceiling forever.
        assert!(sink.emit(&event()).is_written());
        assert_eq!(
            sink.health().current_bytes,
            span * 2,
            "the fixture is only meaningful if it lands EXACTLY on the bound"
        );
        assert_eq!(
            sink.health().rotations,
            0,
            "a file filled exactly to its bound has not exceeded it"
        );

        // And the next event, which cannot fit, DOES roll — so this cannot be
        // passed by a sink that has stopped rotating altogether.
        assert!(sink.emit(&event()).is_written());
        assert_eq!(sink.health().rotations, 1, "the next event rolls");

        let _ignored = std::fs::remove_dir_all(&dir);
        let _ignored = std::fs::remove_dir_all(&probe_dir);
    }

    /// THE LONGEST MATCHING PREFIX WINS, so floors compose instead of fighting.
    #[test]
    fn the_most_specific_target_floor_is_the_one_that_applies() {
        let dir = scratch("longest");
        let sink = Sink::open(
            &Config::new(&dir)
                .with_min_level(Level::Error)
                .with_target_level("pull", Level::Debug)
                .with_target_level("pull.chunk", Level::Trace),
        )
        .expect("opens");

        assert_eq!(sink.level_for("pull.chunk"), Level::Trace, "longest wins");
        assert_eq!(
            sink.level_for("pull.member"),
            Level::Debug,
            "falls to `pull`"
        );
        assert_eq!(
            sink.level_for("api.request"),
            Level::Error,
            "falls to global"
        );

        // The chunk path can go to Trace WITHOUT lifting the rest of the crate,
        // which is the composition this rule buys.
        assert_eq!(
            sink.emit(&Event::trace("pull.chunk", "answered")),
            Emitted::Written
        );
        assert_eq!(
            sink.emit(&Event::trace("pull.member", "too quiet")),
            Emitted::Filtered,
            "`pull` is Debug, so a Trace beneath it is still filtered"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A SINK WITH NO OVERRIDES BEHAVES EXACTLY AS IT DID BEFORE THE FEATURE.
    ///
    /// The fast path is one relaxed atomic load against `min(global,
    /// overrides)`. With no overrides that value IS the global floor, so this
    /// pins that the feature costs nothing when unused — and that
    /// `set_min_level` moves BOTH numbers. The first version of this feature
    /// moved only the global one, so a runtime lowering was silently ignored
    /// while `min_level()` reported the new value.
    #[test]
    fn no_overrides_is_the_old_behaviour_and_the_floor_still_moves_at_runtime() {
        let dir = scratch("nooverride");
        let sink = Sink::open(&Config::new(&dir).with_min_level(Level::Warn)).expect("opens");
        assert_eq!(sink.level_for("anything"), Level::Warn);
        assert_eq!(sink.emit(&Event::info("t", "quiet")), Emitted::Filtered);

        sink.set_min_level(Level::Trace);
        assert_eq!(sink.level_for("anything"), Level::Trace);
        assert_eq!(
            sink.emit(&Event::trace("t", "now kept")),
            Emitted::Written,
            "lowering the floor must lower the FAST floor with it"
        );

        // And with an override present, a global raise must not silence a
        // subsystem pinned lower than it.
        let dir2 = scratch("nooverride2");
        let pinned = Sink::open(
            &Config::new(&dir2)
                .with_min_level(Level::Info)
                .with_target_level("pull", Level::Trace),
        )
        .expect("opens");
        pinned.set_min_level(Level::Error);
        assert_eq!(pinned.level_for("pull"), Level::Trace, "the pin survives");
        assert_eq!(
            pinned.emit(&Event::trace("pull", "still heard")),
            Emitted::Written
        );
        assert_eq!(
            pinned.emit(&Event::warn("api", "below the new global")),
            Emitted::Filtered
        );
        let _ignored = std::fs::remove_dir_all(&dir);
        let _ignored = std::fs::remove_dir_all(&dir2);
    }

    /// THE OVERRIDE LIST IS BOUNDED, WHICH IS WHAT KEEPS THE LOOKUP CONSTANT.
    #[test]
    fn target_floors_past_the_ceiling_are_dropped_rather_than_refused() {
        let mut config = Config::new(Path::new("/tmp/never-opened"));
        for n in 0..(crate::MAX_TARGET_LEVELS + 4) {
            config = config.with_target_level(format!("t{n}"), Level::Trace);
        }
        assert_eq!(
            config.target_levels.len(),
            crate::MAX_TARGET_LEVELS,
            "a bounded array is what makes resolving a target a constant, and \
             an operator who writes nine gets the first eight rather than a \
             refusal to start"
        );
    }

    /// A REPEATED TARGET TAKES ITS LAST REGISTRATION.
    ///
    /// Found by CI gate 18 on its first run: `>` versus `>=` in `level_for`
    /// survived the suite, which meant nothing decided what a repeated target
    /// meant. `with_target_level` does not reject a repeat, so the case is
    /// reachable and had to be chosen rather than left to the operator that
    /// happened to be written first.
    #[test]
    fn a_repeated_target_takes_its_last_registration() {
        let sink = Sink::open(
            &Config::new(scratch("repeat"))
                .with_min_level(Level::Error)
                .with_target_level("pull", Level::Warn)
                .with_target_level("pull", Level::Trace),
        )
        .expect("opens");
        assert_eq!(
            sink.level_for("pull"),
            Level::Trace,
            "the LAST registration wins — a config built in layers expects the \
             last word to count"
        );
        assert_eq!(
            sink.level_for("pull.member"),
            Level::Trace,
            "and its children inherit that same last word"
        );
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
        // The FIRST of these attempts a roll and fails at `reopen`. The event is
        // attempted anyway — losing it because a RENAME failed would be the
        // worse of the two — and is Dropped here only because the append
        // refuses as well.
        //
        // The four after it do NOT attempt a roll, and that is the fix rather
        // than a regression. This assertion used to read `rotation_failures ==
        // 5` — "counted, every time" — which was an accurate description of the
        // defect: every event re-entered `roll`, and `roll` unlinks the oldest
        // file and renames the rest UP before it reaches the step that failed.
        // Re-attempting therefore shifted the whole retained set once per
        // event. Measured against a rename that could not complete, at
        // `keep_files = 5`: `[930, 927, 926, 926, 926]` became
        // `[1862, 927, 926, 0, 0]` in two events.
        //
        // One failure, counted once, and then rotation stops for the life of
        // the sink. See `Sink::rotation_broken`.
        for _ in 0..5 {
            assert_eq!(emit_fat(&sink), Emitted::Dropped);
        }
        let health = sink.health();
        assert_eq!(
            health.rotation_failures, 1,
            "a roll is attempted ONCE and never re-attempted: {health:?}"
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

    /// A target that keeps the bytes it accepted, and can stop half-way.
    ///
    /// `write_all` reports an error without saying how much reached the file
    /// first, which is exactly what a disk filling mid-line does. Neither
    /// [`Brittle`] (which accepts nothing once refusing) nor a real filesystem
    /// (which needs a full volume) can produce that state on demand.
    #[derive(Debug, Default)]
    struct HalfWay {
        wrote: std::sync::Mutex<Vec<u8>>,
        /// Bytes to accept on the next append before refusing. `None` accepts all.
        cut_at: std::sync::Mutex<Option<usize>>,
    }

    impl Target for Arc<HalfWay> {
        fn append(&mut self, bytes: &[u8]) -> std::io::Result<()> {
            let cut = self.cut_at.lock().map_or(None, |mut c| c.take());
            let mut wrote = self
                .wrote
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(n) = cut {
                wrote.extend_from_slice(bytes.get(..n).unwrap_or(bytes));
                return Err(std::io::Error::new(
                    std::io::ErrorKind::StorageFull,
                    "no space left on device",
                ));
            }
            wrote.extend_from_slice(bytes);
            Ok(())
        }
        fn sync(&self) -> std::io::Result<()> {
            Ok(())
        }
        fn reopen(&mut self, _path: &Path) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// **A TORN WRITE DOES NOT SWALLOW THE NEXT EVENT.**
    ///
    /// `write_all` says it failed, never how many bytes landed first. A disk
    /// that fills mid-line leaves a partial line with no newline, and the next
    /// event — appended at that offset — FUSES onto it. One unparseable line,
    /// and **two** events lost where `dropped` counts one.
    ///
    /// Measured on a real 2 MB volume driven to genuine ENOSPC before the fix:
    /// 132 lines written, the file 14 bytes longer than the sink's own running
    /// count, the last byte not a newline; after space was freed the reader
    /// found 134 records for 135 writes and `dropped` still said 1.
    ///
    /// The fix is one byte — a newline after a failed append — so the fragment
    /// becomes its own line the reader counts as `malformed`, and the next event
    /// starts clean.
    #[test]
    fn a_torn_write_is_terminated_so_the_next_event_is_not_fused_onto_it() {
        let dir = scratch("torn");
        let half = Arc::new(HalfWay::default());
        let sink =
            Sink::with_target(&Config::new(&dir), Box::new(Arc::clone(&half))).expect("opens");

        assert!(sink.emit(&Event::info("torn", "first")).is_written());

        // The next append accepts 20 bytes and then refuses, mid-line.
        *half.cut_at.lock().expect("the lock") = Some(20);
        assert_eq!(
            sink.emit(&Event::info("torn", "torn-away")),
            Emitted::Dropped
        );

        assert!(sink.emit(&Event::info("torn", "after")).is_written());

        // THE DOUBLE'S OTHER TWO METHODS. `Target` has three, and a double that
        // implements one is not standing in for a target — `sync` and `reopen`
        // are on the path a real sink takes and were never exercised here, so a
        // change to either would have gone unnoticed through this fixture.
        assert!(sink.sync().is_ok(), "a barrier through the double succeeds");
        assert!(
            sink.emit(&Event::info("torn", "after the sync"))
                .is_written(),
            "and the sink is still usable after one"
        );

        // AND A ROLL, which is the third method. A double that never reopens is
        // not standing in for a target on the one path where rotation happens.
        let rolled = Sink::with_target(
            &Config::new(scratch("torn-roll"))
                .with_max_file_bytes(MIN_FILE_BYTES)
                .with_keep_files(2),
            Box::new(Arc::clone(&half)),
        )
        .expect("opens");
        let wide = "z".repeat(MAX_STR_VALUE_BYTES);
        let fat = "r".repeat(MAX_MESSAGE_BYTES);
        for _ in 0..8 {
            assert!(
                rolled
                    .emit(
                        &Event::info("torn", &fat)
                            .with("a", Value::Str(&wide))
                            .with("b", Value::Str(&wide))
                    )
                    .is_written()
            );
        }
        assert!(
            rolled.health().rotations > 0,
            "the double's `reopen` really was called: {:?}",
            rolled.health()
        );

        let bytes = half.wrote.lock().expect("the lock").clone();
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();

        // THE POINT: the survivor is on a line of its OWN. Without the fix the
        // fragment and this event share one unparseable line.
        assert!(
            lines
                .iter()
                .any(|line| line.contains("\"after\"") && Record::decode(line.as_bytes()).is_ok()),
            "the event after a torn write must be readable on its own line: {lines:#?}"
        );
        assert!(
            lines
                .iter()
                .any(|line| Record::decode(line.as_bytes()).is_err()),
            "and the fragment is still visible as one malformed line, not erased"
        );
        assert_eq!(sink.health().dropped, 1, "exactly one event was lost");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **EITHER HALF OF `is_loud` ALONE MAKES A SINK LOUD.**
    ///
    /// `is_loud` is `dropped > 0 || rotation_failures > 0`, and mutating the
    /// SECOND comparison survived this crate's suite: every existing test that
    /// had a rotation failure also had `dropped > 0`, so the first disjunct hid
    /// the second. A sink that failed every roll and dropped nothing would have
    /// reported itself healthy — and `/logs` renders exactly this flag.
    ///
    /// Each half is therefore asserted in isolation, which is the only shape
    /// that can tell them apart.
    #[test]
    fn each_half_of_is_loud_is_load_bearing_on_its_own() {
        let quiet = Health {
            path: PathBuf::from("events.ndjson"),
            written: 10,
            dropped: 0,
            rotations: 2,
            rotation_failures: 0,
            last_error: None,
            current_bytes: 100,
            next_seq: 11,
        };
        assert!(!quiet.is_loud(), "nothing wrong is not loud");

        let dropped_only = Health {
            dropped: 1,
            ..quiet.clone()
        };
        assert!(
            dropped_only.is_loud(),
            "a lost event alone must be loud, with no rotation failure beside it"
        );

        let rolls_only = Health {
            rotation_failures: 1,
            ..quiet.clone()
        };
        assert!(
            rolls_only.is_loud(),
            "and a failed roll alone must be loud, with NOTHING dropped — the \
             case every other test in this file accidentally masked"
        );
    }

    /// **A REOPENED SINK RESUMES ITS BYTE COUNT, OR ITS BOUND IS A LIE.**
    ///
    /// `Sink::open` seeds the running count from `FileTarget::len()`. Replacing
    /// that with `0` survived the whole suite: nothing asserted that a restart
    /// picks up where it left off. Under the mutation the current file is
    /// allowed to grow to its prior size PLUS the bound before rolling, so the
    /// crate's headline claim — a fixed ceiling on its own footprint — fails
    /// after the first restart, which is the ordinary case for a long-lived
    /// operator process.
    #[test]
    fn a_reopened_sink_resumes_the_byte_count_of_the_file_it_found() {
        let dir = scratch("resume-bytes");
        let before = {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            for n in 0..5u32 {
                assert!(sink.emit(&Event::info("t", "m").with("n", n)).is_written());
            }
            sink.health().current_bytes
        };
        assert!(before > 0, "the premise: something was written");

        let again = Sink::open(&Config::new(&dir)).expect("reopens");
        assert_eq!(
            again.health().current_bytes,
            before,
            "a reopened sink must measure the file it found, not start from zero \
             — otherwise the bound is only honoured until the first restart"
        );
        let on_disk = std::fs::metadata(current_path(&dir)).expect("stat").len();
        assert_eq!(
            again.health().current_bytes,
            on_disk,
            "and the number it resumes is the file's real length"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **ONLY "IT WAS NOT THERE" IS SUCCESS. EVERY OTHER ERROR IS A FAILURE.**
    ///
    /// `remove_if_present` and `rename_if_present` treat `NotFound` as success,
    /// because the set is sparse until it has rolled `keep_files` times.
    /// Replacing either guard with `true` — making EVERY error a success —
    /// survived the whole suite. A roll that could not unlink or could not
    /// rename would then return `Ok`, `rotation_failures` would stay 0, and
    /// `health()` would report a healthy sink while the set grew past its bound
    /// unchecked. That is the silent fallback `CLAUDE.md` §4 bans, in the one
    /// place whose entire job is bounding this crate's footprint.
    ///
    /// A DIRECTORY where a log file belongs is the portable way to make each
    /// call fail with something that is not `NotFound`: `remove_file` on a
    /// directory refuses on every platform this builds for, and a rename onto a
    /// non-empty directory refuses likewise. No `chflags`, no permission games,
    /// no root.
    #[test]
    fn a_rotation_error_that_is_not_absence_is_reported_rather_than_swallowed() {
        use std::os::unix::fs::PermissionsExt as _;

        // (a) THE UNLINK. A directory sits where the oldest file belongs.
        let dir = scratch("roll-unlink-refuses");
        let sink = Sink::open(
            &Config::new(&dir)
                .with_max_file_bytes(MIN_FILE_BYTES)
                .with_keep_files(2),
        )
        .expect("opens");
        let oldest = rotated_path(&dir, 1);
        std::fs::create_dir_all(oldest.join("not-empty")).expect("a directory in the way");

        let fat = "x".repeat(MAX_MESSAGE_BYTES);
        let wide = "y".repeat(MAX_STR_VALUE_BYTES);
        let emit_fat = |sink: &Sink| {
            sink.emit(
                &Event::info("t", &fat)
                    .with("pad_a", wide.as_str())
                    .with("pad_b", wide.as_str()),
            )
        };
        for _ in 0..4 {
            let _outcome = emit_fat(&sink);
        }
        let health = sink.health();
        assert_eq!(
            health.rotation_failures, 1,
            "an unlink that refused for a reason other than absence is a FAILED \
             roll, not a successful one: {health:?}"
        );
        assert!(
            health.is_loud(),
            "and the sink says so, because a page reads this flag"
        );
        assert!(
            health
                .last_error
                .as_deref()
                .is_some_and(|said| said.contains("cannot delete the oldest file")),
            "named by the step that refused: {health:?}"
        );
        let _ignored = std::fs::remove_dir_all(&dir);

        // (b) THE RENAME, ISOLATED FROM THE UNLINK.
        //
        // The oldest file is unlinked FIRST, so any fixture that makes the
        // unlink fail never reaches a rename — the first version of this half
        // put a directory in the way and passed for that wrong reason. The
        // isolation is: a read-only directory with the oldest slot ABSENT, so
        // `remove_if_present` returns `NotFound` -> `Ok` (which a read-only
        // directory permits, because nothing is removed) and the rename is the
        // first call that can refuse.
        let dir = scratch("roll-rename-refuses");
        let sink = Sink::open(
            &Config::new(&dir)
                .with_max_file_bytes(MIN_FILE_BYTES)
                .with_keep_files(3),
        )
        .expect("opens");
        // EXACTLY one natural roll: the first event fits, the second does not,
        // so `.1` becomes a real file and `.2` is never reached. A third event
        // would roll again and occupy `.2`, which is what made the first
        // version of this fixture fail its own premise.
        for _ in 0..2 {
            assert!(emit_fat(&sink).is_written());
        }
        assert!(rotated_path(&dir, 1).exists(), "the premise: .1 is a file");
        assert!(!rotated_path(&dir, 2).exists(), "and .2 is absent");
        assert_eq!(sink.health().rotation_failures, 0, "nothing has failed yet");

        let mut ro = std::fs::metadata(&dir).expect("the dir").permissions();
        ro.set_mode(0o555);
        std::fs::set_permissions(&dir, ro).expect("read-only");
        for _ in 0..3 {
            let _outcome = emit_fat(&sink);
        }
        let health = sink.health();
        let mut rw = std::fs::metadata(&dir).expect("the dir").permissions();
        rw.set_mode(0o755);
        std::fs::set_permissions(&dir, rw).expect("restore");

        assert_eq!(
            health.rotation_failures, 1,
            "a rename that refused is a FAILED roll: {health:?}"
        );
        assert!(
            health
                .last_error
                .as_deref()
                .is_some_and(|said| said.contains("cannot roll")),
            "and it is the RENAME that is named, not the unlink: {health:?}"
        );
        assert!(health.is_loud());
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **`is_written` IS FALSE FOR EVERY OUTCOME THAT IS NOT A WRITE.**
    ///
    /// Mutating it to `true` survived: every call site asserted the happy
    /// answer, so a predicate that always said "written" passed the suite. It
    /// is the value `emit`'s callers branch on and the one the emit-site proofs
    /// in three other crates rest on, so "always true" would have made those
    /// proofs vacuous too.
    #[test]
    fn is_written_is_true_only_for_a_write() {
        assert!(Emitted::Written.is_written());
        assert!(
            !Emitted::Filtered.is_written(),
            "a filtered event is not written"
        );
        assert!(
            !Emitted::Dropped.is_written(),
            "and a dropped one certainly is not"
        );
        assert!(
            !Emitted::NotInstalled.is_written(),
            "nor is one with nowhere to go"
        );
    }

    /// **`resume_seq` READS FAR ENOUGH BACK FOR A LINE OF ANY LEGAL WIDTH.**
    ///
    /// Its block is `64 * 1024`, and its doc says the last complete line is
    /// always inside it "unless one line is larger than the block, which the
    /// ceilings in `crate::event` make impossible". Mutating `*` to `+` makes
    /// the block **1088 bytes** — and the event ceilings permit a line far wider
    /// than that: a 256-byte message plus twelve 128-byte values is over 1800.
    /// The mutant survived because the reopen test wrote five short lines.
    ///
    /// A restart that cannot find a complete line resumes the sequence at ZERO,
    /// so every number in the file repeats — which is the one thing a sequence
    /// exists to prevent.
    #[test]
    fn a_restart_resumes_the_sequence_after_a_line_of_the_widest_legal_shape() {
        let dir = scratch("resume-wide");
        let wide = "w".repeat(MAX_STR_VALUE_BYTES);
        let message = "m".repeat(MAX_MESSAGE_BYTES);
        let span = {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            let mut event = Event::info("t", &message);
            for key in ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"] {
                event = event.with(key, Value::Str(&wide));
            }
            assert!(sink.emit(&event).is_written());
            sink.health().current_bytes
        };
        assert!(
            span > 1088,
            "the premise: one legal line is wider than the mutated block would \
             be, or this proves nothing. It is {span} bytes"
        );

        let again = Sink::open(&Config::new(&dir)).expect("reopens");
        assert_eq!(
            again.health().next_seq,
            2,
            "the sequence resumes after the one line already on disk; a block \
             too small to hold it would restart at 1 and repeat every number"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A FILTERED EVENT EVALUATES NOTHING — WHICH IS THE WHOLE POINT.**
    ///
    /// `Sink::emit` is a function, so a caller reaches it having already built
    /// the entire `Event` and evaluated every argument. Measured in one binary
    /// against `tracing`'s macro: a filtered event cost **6,750 ps with no
    /// fields and 14,146 ps with three** — it DOUBLES with the field count,
    /// while `tracing` stayed at 270 / 250 ps. That is O(call-site fields)
    /// against `CLAUDE.md` §3 rule 4's O(1), and it is the only measured O(1)
    /// violation on the write path. The bound this restores is the one
    /// `telemetry::bench::a_filtered_event_touches_nothing_and_stays_flat`
    /// (C-T-02) measures, and this test is what makes it true of a call site
    /// with fields on it rather than only of a bare one.
    ///
    /// `emit_if!` closes it by gating on [`Sink::admits`] before the arguments
    /// exist. This asserts the property by COUNTING SIDE EFFECTS rather than by
    /// timing: a counter incremented inside an argument expression can only
    /// move if that expression ran. Timing would measure this machine; a
    /// counter measures the semantics.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    #[test]
    fn a_filtered_event_never_evaluates_its_arguments() {
        let dir = scratch("emit-if");
        let sink = Sink::open(&Config::new(&dir).with_min_level(Level::Error)).expect("opens");

        let evaluated = std::sync::atomic::AtomicU64::new(0);
        let count = || {
            evaluated.fetch_add(1, Ordering::Relaxed);
            7_u64
        };

        // ONE SHAPE, DRIVEN BOTH WAYS. Written as two separate `if`s, each
        // would leave its other arm unexecuted forever — the branch that proves
        // the gate lets an event THROUGH and the branch that proves it holds one
        // back are the same code, and a test that only ever takes one of them is
        // half a test.
        let gated = |level: Level| -> Emitted {
            if sink.admits(level, "t") {
                sink.emit(&Event::new(level, "t", "m").with("n", Value::Uint(count())))
            } else {
                Emitted::Filtered
            }
        };

        // BELOW the floor: the argument must not run.
        assert!(
            !sink.admits(Level::Debug, "t"),
            "the premise: Debug is below Error"
        );
        let outcome = gated(Level::Debug);
        assert_eq!(outcome, Emitted::Filtered);
        assert_eq!(
            evaluated.load(Ordering::Relaxed),
            0,
            "a filtered event evaluated an argument — the cost is then a \
             function of how many the call site has, which is not O(1)"
        );

        // AT the floor: the SAME closure, and now the other arm runs.
        assert!(sink.admits(Level::Error, "t"));
        let outcome = gated(Level::Error);
        assert_eq!(outcome, Emitted::Written);
        assert_eq!(
            evaluated.load(Ordering::Relaxed),
            1,
            "and a written event evaluates it exactly once — not zero, not twice"
        );

        // `admits` agrees with `emit` on every rung, which is what makes the
        // gate safe to trust: a gate that disagreed would silently drop events.
        for level in LEVELS {
            let would = sink.admits(level, "t");
            let did = sink.emit(&Event::new(level, "t", "agree")).is_written();
            assert_eq!(
                would, did,
                "{level}: admits said {would} and emit did {did} — a gate that \
                 disagrees with the thing it gates loses events"
            );
        }
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// The run getter answers what the setter stored, including the clear.
    ///
    /// `Sink::run` was added with the stamp and had no test: the WRITE side was
    /// proven end-to-end by `telemetry::tail::a_log_holding_several_runs_splits_back_into_them`,
    /// and the read-back accessor an operator surface would call was never
    /// exercised. Small, but it is the shape this crate has recorded three
    /// times now — a thing built and reachable from nothing.
    /// **THE SECOND LEG OF A PRESS INHERITS THE KEY; IT DOES NOT TAKE IT.**
    ///
    /// The defect: `api::server::broker_run` stamped the run key with a bare
    /// `set_run`, once per LEG, while `api::pullrun::conduct` runs one chain per
    /// vendor CONCURRENTLY. Two feeds in one press overwrote each other, every
    /// event after the second write carried the second feed's id — including the
    /// first feed's — and whichever finished first cleared the key to zero,
    /// after which the survivor's remaining events carried no run at all.
    ///
    /// `record.rs` calls this field *"the key a reader groups by"*, and it was
    /// the one field that could not be trusted for exactly the runs worth
    /// reading: `/logs?run=` showed one story assembled from two feeds and a
    /// second that stopped mid-sentence.
    ///
    /// Four properties, and each fails independently.
    #[test]
    fn a_second_claim_does_not_steal_the_run_key_and_a_loser_cannot_clear_it() {
        let dir = scratch("run-claim");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        assert_eq!(sink.run(), 0, "a fresh sink belongs to no run");

        // 1. THE FIRST LEG TAKES IT.
        assert!(sink.claim_run(11), "an unheld key is takeable");
        assert_eq!(sink.run(), 11);

        // 2. THE SECOND DOES NOT — and the key does not move. This is the whole
        //    defect: `set_run` would have made this 22.
        assert!(!sink.claim_run(22), "a held key is not takeable");
        assert_eq!(
            sink.run(),
            11,
            "the concurrent sibling INHERITS the press's id rather than \
             overwriting it — they are one press"
        );

        // 3. A LOSER CANNOT CLEAR IT. This is the half that left the survivor's
        //    events with no run at all: whichever leg finished first called
        //    `set_run(0)` unconditionally.
        sink.release_run(22);
        assert_eq!(
            sink.run(),
            11,
            "a leg that never took the key must not clear it out from under the \
             legs still running"
        );

        // 4. THE HOLDER CAN, and only the holder.
        sink.release_run(11);
        assert_eq!(sink.run(), 0, "the press ended, so the key is free again");

        // AND ZERO IS NOT CLAIMABLE. Zero is the ABSENCE of a run, so claiming
        // it would be taking nothing while reporting success — after which the
        // caller would `release_run(0)` and clear whatever a later press held.
        assert!(!sink.claim_run(0), "zero is no run, not a run named zero");
        assert_eq!(sink.run(), 0);
    }

    #[test]
    fn the_run_getter_answers_what_was_stamped_and_what_was_cleared() {
        let dir = scratch("run-getter");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        assert_eq!(sink.run(), 0, "a fresh sink belongs to no run");

        sink.set_run(1_786_197_791_427);
        assert_eq!(sink.run(), 1_786_197_791_427);

        sink.set_run(0);
        assert_eq!(
            sink.run(),
            0,
            "and zero clears it rather than being a run id"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_explicit_run_stamps_only_its_event_and_never_steals_the_ambient_run() {
        let dir = scratch("explicit-run");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        sink.set_run(7);
        assert!(
            sink.emit_for_run(11, &Event::info("t", "explicit"))
                .is_written()
        );
        assert_eq!(sink.run(), 7, "the ambient owner was not overwritten");
        assert!(sink.emit(&Event::info("t", "ambient")).is_written());

        let bytes = std::fs::read(current_path(&dir)).expect("the two events landed");
        let records: Vec<Record> = bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| Record::decode(line).expect("the sink decodes its own line"))
            .collect();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].run, 11, "the explicit event took its own key");
        assert_eq!(records[1].run, 7, "the next event kept the ambient key");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reserved_run_ids_resume_strictly_above_every_id_that_reached_the_log() {
        let dir = scratch("reserved-run");
        let first = {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            let first = sink
                .reserve_run_id()
                .expect("the id space is not exhausted");
            assert_ne!(first, 0);
            assert!(
                sink.emit_for_run(first, &Event::info("t", "attempt started"))
                    .is_written()
            );
            first
        };
        let reopened = Sink::open(&Config::new(&dir)).expect("reopens");
        let second = reopened
            .reserve_run_id()
            .expect("the resumed id space is not exhausted");
        assert!(
            second > first,
            "a restart must not give a retained attempt its id again: {first} then {second}"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **THE RETAINED WINDOW SURVIVES A ROLL THAT CANNOT COMPLETE.**
    ///
    /// This is the property `Sink::rotation_broken` exists for, and it is
    /// asserted on the FILES rather than on a counter — a counter cannot tell
    /// you that history is still there.
    ///
    /// `roll` unlinks the oldest file and renames the rest UP *before* it
    /// reaches the step that fails, so every re-attempt shifts the whole set
    /// again. With rotation left to re-attempt, `keep_files` further events
    /// empty every retained file in turn. Measured before the fix, at
    /// `keep_files = 5`: `[930, 927, 926, 926, 926]` to `[1862, 927, 926, 0, 0]`
    /// in two events.
    ///
    /// **WHICH ASSERTION DISCRIMINATES, stated so this is not read as proving
    /// more than it does.** On a read-only directory the *first* step — the
    /// unlink — already refuses, so the shift never begins and the file sizes
    /// hold even without the fix. The assertion that fails without it is the
    /// re-attempt count: 20 rather than 1. That count is the mechanism, and the
    /// sizes are the consequence — they are asserted anyway so that a future
    /// fixture in which the chain fails half-way (a single unrenameable file,
    /// which needs `chflags` and is not portable) cannot regress silently.
    #[test]
    fn a_roll_that_cannot_complete_does_not_empty_the_history_one_file_per_event() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = scratch("roll-keeps-history");
        // A REAL sink on REAL files: the property is about bytes that survive on
        // disk, and `Brittle` is an in-memory target with no files to keep.
        let sink = Sink::open(
            &Config::new(&dir)
                .with_max_file_bytes(MIN_FILE_BYTES)
                .with_keep_files(4),
        )
        .expect("opens");

        let fat = "x".repeat(MAX_MESSAGE_BYTES);
        let wide = "y".repeat(MAX_STR_VALUE_BYTES);
        let emit_fat = |sink: &Sink| {
            sink.emit(
                &Event::info("t", &fat)
                    .with("pad_a", wide.as_str())
                    .with("pad_b", wide.as_str()),
            )
        };

        // Fill the set so there is a history to lose.
        for _ in 0..8 {
            assert!(emit_fat(&sink).is_written());
        }
        let sizes = |dir: &Path| -> Vec<u64> {
            (1..4)
                .map(|n| std::fs::metadata(rotated_path(dir, n)).map_or(0, |meta| meta.len()))
                .collect()
        };
        let kept = sizes(&dir);
        assert!(
            kept.iter().all(|&len| len > 0),
            "the premise: the window holds events before anything fails: {kept:?}"
        );

        // A READ-ONLY DIRECTORY: the rename and the unlink both refuse, which is
        // the real condition an operator hits on a mount gone read-only.
        let mut ro = std::fs::metadata(&dir).expect("the dir").permissions();
        ro.set_mode(0o555);
        std::fs::set_permissions(&dir, ro).expect("make it read-only");

        for _ in 0..20 {
            let _outcome = emit_fat(&sink);
        }

        let after = sizes(&dir);

        // Restore BEFORE asserting, so a failure still leaves a removable tree.
        let mut rw = std::fs::metadata(&dir).expect("the dir").permissions();
        rw.set_mode(0o755);
        std::fs::set_permissions(&dir, rw).expect("restore");

        assert_eq!(
            after, kept,
            "twenty events after the first failed roll, and every retained file \
             is byte-for-byte the size it was. Before the fix these emptied one \
             per event: {kept:?} -> {after:?}"
        );
        assert_eq!(
            sink.health().rotation_failures,
            1,
            "and the failure is reported once, not twenty times"
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

    /// **THE TWO FLOORS ARE ONE WORD, SO NOTHING CAN OBSERVE THEM CROSSED.**
    ///
    /// `set_min_level` used to store the global floor into one atomic and then
    /// compute and store the fast floor into another. Two callers racing
    /// interleave those four stores, and one ordering leaves the pair
    /// describing two *different* floors until somebody sets the level again.
    /// A barrier-synchronised probe on this tree — two setters, one observer,
    /// and reads taken only at the instant no thread was writing — saw it at
    /// rounds 2023, 2597, 5275 and 6269 of four million, in both directions:
    /// `min_level()` saying `error` while a `trace` event was still admitted,
    /// and `min_level()` saying `trace` while `trace` was filtered. Rare, real,
    /// and silent: an operator who lowers the floor mid-backfill gets no debug
    /// lines and a `min_level()` that agrees with what they asked for.
    ///
    /// **THIS TEST IS DETERMINISTIC, AND THAT IS THE POINT.** A loop racing two
    /// threads fails only sometimes — 74 rounds one run and 17,715 the next —
    /// so it reports the scheduler rather than the code, and a suite that
    /// sometimes goes red for no change is a suite nobody reads. The property
    /// that holds after the fix is structural rather than statistical: the only
    /// mutator is ONE store of ONE word, so the only states any reader can
    /// observe are the words `set_min_level` wrote, and this asserts every one
    /// of those words is consistent — high byte the global floor, low byte
    /// `min(global, every override)` — for all five levels, with an override
    /// below the floor and one above it, plus the word the constructor wrote.
    ///
    /// It reads `floors` directly, as ONE load, rather than through
    /// `min_level()` and `admits()`, which are two loads at two instants and
    /// could only ever be compared by racing them. **Splitting the pair back
    /// into two atomics does not make this test flaky — it stops it
    /// compiling**, which is the loudest failure a regression can be given.
    #[test]
    fn the_two_floors_live_in_one_word_and_are_never_observed_crossed() {
        let dir = scratch("floors-one-word");
        // One override BELOW the global floor and one ABOVE it, so the fast
        // floor is neither trivially the global floor nor trivially an
        // override: it is `min(global, Debug)` at every rung.
        let sink = Sink::with_target(
            &Config::new(&dir)
                .with_min_level(Level::Info)
                .with_target_level("pull", Level::Debug)
                .with_target_level("api.request", Level::Error),
            Box::new(Arc::new(Brittle::default())),
        )
        .expect("opens");

        // THE CONSTRUCTOR IS THE OTHER WRITER OF THE PAIR, and it is checked
        // before anything has been set: a sink born crossed is the same defect
        // arriving a different way.
        let [global, fast] = sink.floors.load(Ordering::Relaxed).to_be_bytes();
        assert_eq!(Level::of_rank(global), Some(Level::Info));
        assert_eq!(
            Level::of_rank(fast),
            Some(Level::Debug),
            "the fast floor a sink is born with is min(Info, Debug, Error)"
        );

        for level in LEVELS {
            sink.set_min_level(level);
            // ONE LOAD. Whatever a racing reader sees, it sees one of these.
            let [global, fast] = sink.floors.load(Ordering::Relaxed).to_be_bytes();

            // The expectation is derived through `Ord` rather than through
            // `rank`, so this is not the implementation checking itself: the
            // two agree only because `level.rs` pins that they do.
            let expected = [level, Level::Debug, Level::Error]
                .into_iter()
                .min()
                .expect("three levels");

            assert_eq!(
                Level::of_rank(global),
                Some(level),
                "the high byte is the global floor that was just set"
            );
            assert_eq!(
                Level::of_rank(fast),
                Some(expected),
                "and the low byte, in the SAME word, is min(global, overrides) \
                 — a pair that disagreed is the defect this packing removes"
            );
            assert_eq!(
                sink.min_level(),
                level,
                "and what the sink reports is that same high byte"
            );
            // TIED TO BEHAVIOUR, so this is not a test of a representation.
            // A subsystem pinned at Debug stays audible however high the
            // global floor goes — which needs the low byte to have moved with
            // the high one, in this store, not the next.
            assert!(
                sink.admits(Level::Debug, "pull"),
                "a subsystem pinned at Debug is still admitted with the global \
                 floor at {level}"
            );
            assert_eq!(
                sink.admits(Level::Debug, "unnamed"),
                Level::Debug.at_least(level),
                "while an unnamed subsystem is governed by the global floor"
            );
        }
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

    /// **A TORN TAIL FROM A KILL IS NOT FUSED ONTO BY THE NEXT PROCESS.**
    ///
    /// `emit` already terminates a line its own `write_all` tore. The other
    /// way a line ends mid-write has no such repair available, because the
    /// process that would have written the byte is gone: a kill between the
    /// iterations of a split `write_all`, or a power cut with the last page
    /// still in the cache.
    ///
    /// `resume_seq` reads the last COMPLETE line, so the restart knew the
    /// fragment was there and appended past it anyway. Run against this exact
    /// fixture with the fix disabled, the reader said
    /// `records=3 malformed=1 partial_tail=false` while the sink said
    /// `written=1 dropped=0`: the new event was inside the corrupt line, the
    /// writer believed it had landed, and nothing counted the torn one.
    ///
    /// **`malformed` is 1 both before and after, and that is the point rather
    /// than a weakness of the test.** Before, one `malformed` stood for two
    /// lost events and neither was named; after, it stands for exactly the one
    /// that was torn. What flips is everything around it: the file's last byte,
    /// the record count, which record is newest, and `last_error`.
    ///
    /// The second half is the mutation that matters as much: a guard that
    /// fired on EVERY open would put a blank line in front of every restart's
    /// first event, and no assertion above would notice.
    #[test]
    fn a_file_that_ends_mid_line_is_terminated_at_open_and_not_appended_onto() {
        let dir = scratch("torn-tail-on-open");
        {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            for _ in 0..3 {
                assert!(sink.emit(&Event::info("t", "whole")).is_written());
            }
        }
        // The shape a kill leaves: complete lines, then one that stops.
        let mut bytes = std::fs::read(current_path(&dir)).expect("the file");
        bytes.extend_from_slice(
            br#"{"seq":4,"ts":"x","ms":5,"level":"info","target":"t","msg":"tor"#,
        );
        std::fs::write(current_path(&dir), &bytes).expect("torn");

        let again = Sink::open(&Config::new(&dir)).expect("reopens");
        let closed = std::fs::read(current_path(&dir)).expect("the file");
        assert_eq!(
            closed.last(),
            Some(&b'\n'),
            "the tear is closed AT OPEN, before anything is emitted through it"
        );
        assert_eq!(
            again.health().current_bytes,
            u64::try_from(closed.len()).unwrap(),
            "and the running count owns the byte it wrote — a count that \
             disagreed with the file moves the roll decision for the life of \
             the sink"
        );
        // Named, and on the surface `/logs.json` renders.
        let health = again.health();
        assert!(
            health
                .last_error
                .as_deref()
                .is_some_and(|said| said.contains("no newline")),
            "the tear is named rather than repaired in silence: {health:?}"
        );

        assert!(again.emit(&Event::info("t", "after the tear")).is_written());
        let found = crate::tail::tail(&dir, again.keep_files(), &crate::tail::Query::last(10));
        assert_eq!(
            found.records.len(),
            4,
            "three whole lines and the new one — fused, the new event is inside \
             the corrupt line and this is 3: {found:?}"
        );
        assert_eq!(found.records[0].message, "after the tear");
        assert_eq!(
            found.malformed, 1,
            "and the torn record is ONE malformed line, counted, which is \
             exactly one event's worth"
        );
        // Stated rather than credited to the fix: the file ends on a line
        // boundary, so the reader has no fragment to report. It read false
        // before the fix too, because the FUSED line also ended in a newline —
        // which is precisely why `partial_tail` could not be the thing that
        // told anybody an event had been swallowed.
        assert!(!found.partial_tail, "{found:?}");

        // A FILE THAT ENDS PROPERLY IS NOT TOUCHED.
        let before = std::fs::metadata(current_path(&dir)).expect("stat").len();
        let third = Sink::open(&Config::new(&dir)).expect("reopens");
        assert_eq!(
            std::fs::metadata(current_path(&dir)).expect("stat").len(),
            before,
            "a whole file gains no byte on open"
        );
        assert_eq!(third.health().current_bytes, before);
        assert!(
            third.health().last_error.is_none(),
            "and nothing is reported about a file that was never torn"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **THE TERMINATING BYTE CAN ITSELF FAIL, AND THAT IS SAID RATHER THAN
    /// ASSUMED AWAY.**
    ///
    /// The arm is a full disk at the moment of restart, which is not a state a
    /// developer's machine enters on request — so it is driven through the
    /// [`Target`] seam, the same way the failed-append policy is. Two things
    /// are pinned: the running count does NOT claim a byte that was never
    /// written, and the notice says the next event will fuse rather than
    /// implying the tear was closed.
    #[test]
    fn a_tear_that_cannot_be_closed_is_named_and_the_count_claims_no_byte() {
        let dir = scratch("torn-tail-cannot-close");
        std::fs::create_dir_all(&dir).expect("the directory");
        let torn = b"{\"seq\":1,\"ms\"";
        std::fs::write(current_path(&dir), torn).expect("torn");

        let brittle = Arc::new(Brittle::default());
        brittle.refuse();
        let mut target = Arc::clone(&brittle);
        let (bytes, why) = super::terminate_torn_tail(
            &mut target,
            &current_path(&dir),
            u64::try_from(torn.len()).unwrap(),
        );
        assert_eq!(
            bytes,
            u64::try_from(torn.len()).unwrap(),
            "the count is the file's real length: nothing was appended"
        );
        let said = why.expect("a tear that could not be closed is still reported");
        assert!(said.contains("could not be written"), "{said}");
        assert!(
            said.contains("fuse"),
            "and it says what will happen next rather than implying a repair: {said}"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A TAIL THAT CANNOT BE EXAMINED IS LEFT EXACTLY AS IT WAS FOUND.**
    ///
    /// `ends_mid_line` has to look at the last byte, and looking can refuse.
    /// The wrong answer to "I could not tell" is to append anyway: on a file
    /// whose last line is complete that puts a blank line in front of the
    /// restart's first event, on every open, forever. So the unknown case does
    /// nothing at all, and this pins that it really does nothing — not one
    /// byte, and no claim in `last_error` about a file nobody could read.
    ///
    /// A **write-only** file is the portable way to reach it: `FileTarget`
    /// opens it for appending and `read_at`, which opens for reading, refuses.
    /// The same caveat the read-only-directory fixture above carries applies —
    /// as root the mode decides nothing and this proves nothing.
    #[test]
    fn a_tail_that_cannot_be_examined_is_left_exactly_as_it_was_found() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = scratch("torn-tail-unreadable");
        std::fs::create_dir_all(&dir).expect("the directory");
        let torn = b"{\"seq\":1,\"ms\"";
        std::fs::write(current_path(&dir), torn).expect("torn");
        let mut write_only = std::fs::metadata(current_path(&dir))
            .expect("stat")
            .permissions();
        write_only.set_mode(0o222);
        std::fs::set_permissions(current_path(&dir), write_only).expect("write-only");

        let sink = Sink::open(&Config::new(&dir)).expect("a log that cannot be read still opens");
        let health = sink.health();
        let on_disk = std::fs::metadata(current_path(&dir)).expect("stat").len();

        let mut restored = std::fs::metadata(current_path(&dir))
            .expect("stat")
            .permissions();
        restored.set_mode(0o644);
        std::fs::set_permissions(current_path(&dir), restored).expect("restore");

        assert_eq!(
            on_disk,
            u64::try_from(torn.len()).unwrap(),
            "not one byte was appended on a guess"
        );
        assert_eq!(health.current_bytes, on_disk);
        assert!(
            health.last_error.is_none(),
            "and nothing is claimed about a file nobody could look at: {health:?}"
        );
        assert_eq!(
            health.next_seq, 1,
            "`resume_seq` could not read it either, which is its own stated limit"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A real file whose descriptor will not be pointed anywhere else.
    ///
    /// `Brittle` refuses everything, so a sink around it cannot show what
    /// happens to the bytes AFTER a roll that renamed and could not reopen —
    /// the appends refuse too. This one keeps writing through the descriptor it
    /// already holds, which is exactly what a real [`FileTarget`] does, and
    /// exactly why the bytes land in the file the rename moved.
    #[derive(Debug)]
    struct WontReopen {
        file: std::fs::File,
    }

    impl Target for WontReopen {
        fn append(&mut self, bytes: &[u8]) -> std::io::Result<()> {
            use std::io::Write as _;
            self.file.write_all(bytes)
        }

        fn sync(&self) -> std::io::Result<()> {
            self.file.sync_all()
        }

        fn reopen(&mut self, _path: &Path) -> std::io::Result<()> {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "permission denied",
            ))
        }
    }

    /// **A ROLL THAT RENAMED AND THEN COULD NOT REOPEN WRITES INTO
    /// `events.1.ndjson`, BEHIND `Health::path`'S BACK.**
    ///
    /// `roll` unlinks, renames the set up, renames the current file to `.1`,
    /// and only then reopens. Every step before the reopen has a test naming
    /// it — `a_rotation_error_that_is_not_absence_is_reported_rather_than_swallowed`
    /// covers the unlink and the rename — and the reopen had none: the one test
    /// that reached it did so through `Brittle`, whose `append` refuses as
    /// well, so every event after the failure was `Dropped` and the state this
    /// leaves behind was never observed.
    ///
    /// The state is worth a test because it is the one degradation here that is
    /// not visible on the surface that reports it. `rotation_failures` counts
    /// the roll and `is_loud` says so, but `Health::path` still names
    /// `events.ndjson` — which by then does not exist, because the rename
    /// completed and the create that would have remade it did not. The
    /// descriptor the sink still holds names the renamed inode, so every later
    /// event lands in `events.1.ndjson` instead.
    ///
    /// Nothing is lost, and that is asserted too: the reader walks the set
    /// rather than the one path, so the events are all on the page. What is
    /// wrong is only where `Health` says they are.
    #[test]
    fn a_roll_that_renames_and_cannot_reopen_writes_into_the_file_it_rolled() {
        let dir = scratch("roll-reopen-refuses");
        std::fs::create_dir_all(&dir).expect("the directory");
        let file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(current_path(&dir))
            .expect("the current file");
        let sink = Sink::with_target(
            &Config::new(&dir)
                .with_max_file_bytes(MIN_FILE_BYTES)
                .with_keep_files(2),
            Box::new(WontReopen { file }),
        )
        .expect("opens");

        // Over half the bound, so the second event rolls. The arithmetic is
        // the one `a_roll_that_fails_is_counted_and_the_event_is_written_anyway`
        // spells out: the message ceiling alone cannot get there.
        let fat = "x".repeat(MAX_MESSAGE_BYTES);
        let wide = "y".repeat(MAX_STR_VALUE_BYTES);
        let emit_fat = || {
            sink.emit(
                &Event::info("t", &fat)
                    .with("pad_a", wide.as_str())
                    .with("pad_b", wide.as_str()),
            )
        };

        assert!(emit_fat().is_written());
        let span = sink.health().current_bytes;
        assert!(
            span.saturating_mul(2) > MIN_FILE_BYTES,
            "the line must be over half the bound or nothing rolls: span {span}"
        );
        assert!(
            current_path(&dir).exists(),
            "the premise: it is there first"
        );

        // The roll: unlink of the absent oldest succeeds, the rename succeeds,
        // the reopen refuses. The event is written anyway.
        assert!(emit_fat().is_written());
        // And no roll is re-attempted, so the third lands the same way.
        assert!(emit_fat().is_written());
        sink.sync().expect("the renamed descriptor remains durable");

        let health = sink.health();
        assert_eq!(
            health.rotation_failures, 1,
            "the reopen failure is a FAILED roll, counted once: {health:?}"
        );
        assert_eq!(health.rotations, 0, "and not counted as a roll as well");
        assert!(health.is_loud());
        assert!(
            health
                .last_error
                .as_deref()
                .is_some_and(|said| said.contains("cannot open the next file")),
            "named by the step that refused, not by the rename before it: {health:?}"
        );
        assert!(
            health.current_bytes > MIN_FILE_BYTES,
            "the current file grows past its bound, which is the documented \
             degradation: {health:?}"
        );

        // THE PART NOTHING ELSE SAYS. `Health` names a path that is not there.
        assert_eq!(health.path, current_path(&dir));
        assert!(
            !current_path(&dir).exists(),
            "the rename completed and the create that would have remade it did \
             not, so Health::path names a file that does not exist"
        );
        assert_eq!(
            lines_of(&rotated_path(&dir, 1)).len(),
            3,
            "and all three events are in the file the roll moved aside"
        );

        // Nothing is LOST by it: the reader walks the set, not the one path.
        let found = crate::tail::tail(&dir, sink.keep_files(), &crate::tail::Query::last(10));
        assert_eq!(found.records.len(), 3, "{found:?}");
        assert_eq!(found.malformed, 0);
        let _ignored = std::fs::remove_dir_all(&dir);
    }
}
