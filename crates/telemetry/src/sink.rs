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
use crate::event::{Event, MAX_TARGET_BYTES};
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
/// footprint while rotation works**: negligible beside the 40 GB lake, and
/// small enough that no operator has to think about it. It used to say
/// "forever", and it is not: after the first failed roll `rotation_broken`
/// stops rotating for the life of the sink and the current file grows past
/// its bound, loudly, in [`Health`]. D-1324. It is also ~260,000 events
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

/// How many lines one `(target, message)` pair may write at `warn` in one
/// [`FLOOD_WINDOW_MS`] window before the rest of that window's lines are held
/// back and counted (DB-A43).
///
/// # Why a ration, and why at `warn` only
///
/// The 64 MiB ring is the only place a `warn` line lives, and one pair can
/// spend it in seconds: measured on the operator's store, 9 Oct 2026, one
/// wall-second held 29,415 `api.bars read refused` lines (one per refused
/// month of a `/db` burst) and the rolled file `events.3` covered 155 ms. Every
/// other line of that hour was evicted by one repeated sentence.
///
/// So a pair past this many lines in one window writes nothing more in that
/// window: each held line is counted in [`Health::rationed`], and the pair's
/// next line after the window writes ONE `held back by the flood ration` line
/// first, naming the message and how many were held. Nothing is silent; the
/// count is exact. A pair that floods and then falls silent leaves its last
/// window's count in [`Health::rationed`] only, because the summary rides on
/// the pair's next line. Said rather than hidden (`CLAUDE.md` §3 rule 6).
///
/// `error` is never rationed (a refusal names itself at that level), and an
/// audited line ([`is_audited`]) is never rationed either: the durable stream
/// must not lose what it exists to keep. `info` and below are not rationed:
/// they are a run's structural progress, which pages fold line by line.
///
/// 256 is above every ration a caller already applies (`api.request`'s
/// 200 failed-request lines a minute), so no caller's own bound is cut.
pub const DEFAULT_FLOOD_LINES: u32 = 256;

/// The flood ration's window, in milliseconds of the sink's own clamped clock.
pub const FLOOD_WINDOW_MS: i64 = 1_000;

/// The message of the one line that says a pair was held back.
pub const FLOOD_MESSAGE: &str = "held back by the flood ration";

/// The most pairs the ration follows at once. A pair past it is written
/// unrationed rather than held: the table is bounded so a lookup is a
/// constant, and an unfollowed pair loses nothing.
const FLOOD_PAIRS: usize = 128;

/// What happened to one event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emitted {
    /// The bytes reached the file.
    Written,
    /// Quieter than the sink's floor, or a `warn` line its pair's flood
    /// ration held back (counted in [`Health::rationed`] and said by the
    /// pair's next line; [`DEFAULT_FLOOD_LINES`]). Not a failure: the bound
    /// working, and nothing was lost without a count.
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
    /// Whether [`Sink::open`] also keeps the durable audit stream beside the
    /// rolling set: refusals and run lifecycle events, rolled at
    /// [`AUDIT_MAX_FILE_BYTES`] and kept [`AUDIT_KEEP_FILES`] files, with the
    /// `audit.idx` index (D-3341 item 3). On by default; a sink around a
    /// [`Target`] this crate did not open never keeps one.
    pub audit: bool,
    /// How many `warn` lines one `(target, message)` pair may write per
    /// [`FLOOD_WINDOW_MS`]; zero turns the ration off. [`DEFAULT_FLOOD_LINES`]
    /// by default (DB-A43).
    pub flood_lines: u32,
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
            audit: true,
            flood_lines: DEFAULT_FLOOD_LINES,
        }
    }

    /// The same configuration with the audit stream on or off.
    #[must_use]
    pub const fn with_audit(mut self, audit: bool) -> Self {
        self.audit = audit;
        self
    }

    /// The same configuration with a different flood ration: `lines` per
    /// `(target, message)` pair at `warn` per [`FLOOD_WINDOW_MS`], or none at
    /// all for zero.
    #[must_use]
    pub const fn with_flood_lines(mut self, lines: u32) -> Self {
        self.flood_lines = lines;
        self
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
        // AND EACH PREFIX IS BOUNDED TOO (OD-4, D-2373). The count bound above
        // makes `level_for` a constant number of comparisons; it did not make
        // each comparison constant, because a prefix had no length bound and
        // `starts_with` and `==` cost up to the prefix's length. A target is
        // recorded at most `MAX_TARGET_BYTES` long (`encode` caps it), so a
        // longer prefix names no target this sink writes faithfully: it is
        // refused by name rather than clamped, which `CLAUDE.md` §4 forbids.
        if let Some((prefix, _)) = self
            .target_levels
            .iter()
            .find(|(prefix, _)| prefix.len() > MAX_TARGET_BYTES)
        {
            return Some(format!(
                "a per-target override prefix of {} bytes is past the \
                 {MAX_TARGET_BYTES}-byte target ceiling: `Sink::level_for` compares \
                 it on every event that clears the fast floor, and each comparison \
                 is only constant-time because the prefix is bounded",
                prefix.len()
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
    /// Events whose clock reading was BEHIND the floor and was held up to it.
    ///
    /// The clamp keeps `ms` order equal to file order, and it used to do so
    /// silently: a floor resumed from a line stamped by a clock once set
    /// forward held every later event at that future instant, for as long as
    /// the error lasted, and nothing said so. Each held event is counted here,
    /// and an open that resumes a floor ahead of the clock names it in
    /// [`Self::last_error`]. audit-20261003 hunt-costs-4, D-1538.
    pub clock_held: u64,
    /// `warn` lines the flood ration held back ([`DEFAULT_FLOOD_LINES`],
    /// DB-A43). Each is also counted on the `held back by the flood ration`
    /// line its pair writes next; this is the running total, including a last
    /// window no later line has reported yet. Not a failure, so not loud: the
    /// ration is the bound working.
    pub rationed: u64,
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
    /// Whether the target may end in a fragment no newline closed.
    ///
    /// Set when an append failed AND the one-byte terminator after it failed
    /// too — a disk still full. The next append then leads with the newline,
    /// so the event it reports `Written` is a line of its own rather than the
    /// tail of the fragment. Cleared when an append lands. A newline led with
    /// when the fragment was empty is one blank line the reader steps over.
    /// audit-20261003 attacksweep-2, D-1539.
    torn: bool,
    /// The durable audit stream, when this sink keeps one (D-3341 item 3).
    /// Behind the same lock as the ring, so an audited line lands in both
    /// files in one order and under one writer.
    audit: Option<Audit>,
    /// The never-rolling state stream, kept beside the audit stream whenever
    /// that one is (R3-24): the who-did-what lines
    /// (`crate::run_dirs::state_events::is_state_change`), a second copy that
    /// no flood of refusals can roll away. Behind the same lock, so a line
    /// lands in all three files in one order.
    state: Option<Audit>,
    /// The flood ration's pairs (DB-A43). Behind the ring's lock, so a pair's
    /// count and the lines it admits are one order.
    flood: Flood,
    /// What the picture counts and the live copy (R3-24), kept as each line
    /// is written.
    counters: Counters,
}

/// One `(target, message)` pair's place in the flood ration.
#[derive(Debug, Clone, Copy)]
struct Pair {
    /// The `ms` its window opened at.
    start: i64,
    /// Lines written in that window.
    written: u32,
    /// Lines held back in that window, not yet reported by a line.
    held: u64,
}

/// What the flood ration decided for one line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rationed {
    /// Write it, first saying how many lines of its pair the last window held
    /// back (zero: nothing to say).
    Write { held_before: u64 },
    /// Hold it back: its pair has written its ration this window.
    Hold,
}

/// The pairs the flood ration follows, keyed by a hash of the pair so a
/// lookup allocates nothing. Two pairs whose hashes collide share a ration:
/// the worst that does is hold a line one window early, and it is counted.
#[derive(Debug, Default)]
struct Flood {
    pairs: std::collections::HashMap<u64, Pair>,
}

impl Flood {
    /// The decision for one `warn` line of `target` and `message` at `at`,
    /// with `limit` lines a window. O(1): one hash of the pair and one probe;
    /// at most once per [`FLOOD_PAIRS`] new pairs, one pass that drops the
    /// pairs with nothing to report.
    ///
    /// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
    /// "Cost claims of the /db fix round that name no proof").
    fn admit(&mut self, limit: u32, target: &str, message: &str, at: i64) -> Rationed {
        use std::hash::{Hash as _, Hasher as _};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        target.hash(&mut hasher);
        message.hash(&mut hasher);
        let key = hasher.finish();
        if !self.pairs.contains_key(&key) && self.pairs.len() >= FLOOD_PAIRS {
            self.pairs
                .retain(|_, pair| pair.held > 0 || at < pair.start.saturating_add(FLOOD_WINDOW_MS));
            if self.pairs.len() >= FLOOD_PAIRS {
                // A PAIR PAST THE TABLE IS WRITTEN, never held: an unfollowed
                // pair loses nothing.
                return Rationed::Write { held_before: 0 };
            }
        }
        let pair = self.pairs.entry(key).or_insert(Pair {
            start: at,
            written: 0,
            held: 0,
        });
        let mut held_before = 0;
        if at >= pair.start.saturating_add(FLOOD_WINDOW_MS) {
            held_before = pair.held;
            *pair = Pair {
                start: at,
                written: 0,
                held: 0,
            };
        }
        if pair.written < limit {
            pair.written = pair.written.saturating_add(1);
            Rationed::Write { held_before }
        } else {
            pair.held = pair.held.saturating_add(1);
            Rationed::Hold
        }
    }
}

/// How many minutes the `/logs` picture counts, newest last: one hour
/// (R3-24).
pub const PICTURE_MINUTES: usize = 60;

/// How many codes the `/logs` picture keeps by count (R3-24).
pub const PICTURE_CODES: usize = 16;

/// How many of the newest lines the sink keeps a copy of in memory, for the
/// live tail (R3-24). Each is a buffer reused in place, so at most this many
/// lines of at most the widest legal line (just over 41 KB) are held: about
/// 21 MB in the worst case, a few hundred KB at the lines' usual width.
pub const RECENT_LINES: usize = 512;

/// What the sink counted as it wrote, for the `/logs` picture (R3-24): kept
/// at write time in a fixed size, so the picture is read from counters and
/// never from the files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Picture {
    /// Per minute of the [`PICTURE_MINUTES`] ending at the minute asked
    /// for, oldest first: the minute's first `ms` and the lines written in
    /// it per level, by [`Level::rank`]. A minute nothing was written in is
    /// all zeros.
    pub minutes: Vec<(i64, [u64; 5])>,
    /// The codes written most (a line's `code`, else its `reason`), most
    /// first, at most [`PICTURE_CODES`].
    pub codes: Vec<(String, u64)>,
    /// How many times a new code took the place of the least-counted one
    /// once the table was full. Past zero a count may be over by at most the
    /// count it replaced (the space-saving rule): said, never hidden.
    pub codes_replaced: u64,
    /// The `ms` of the oldest line the ring still holds, as the sink saw
    /// each file begin; `None` for a ring with no line.
    pub oldest_ms: Option<i64>,
    /// Lines counted since the sink opened.
    pub counted: u64,
    /// When the sink opened, `ms`.
    pub since_ms: i64,
}

/// The counters behind a [`Picture`], and the live copy, behind the ring's
/// lock: every update is O(1) or bounded by a constant of this module.
#[derive(Debug)]
struct Counters {
    /// `(minute, counts)` by minute modulo [`PICTURE_MINUTES`]; a slot whose
    /// minute is not the one being counted is reset first.
    minutes: [(i64, [u64; 5]); PICTURE_MINUTES],
    /// The code table, at most [`PICTURE_CODES`].
    codes: Vec<(String, u64)>,
    /// See [`Picture::codes_replaced`].
    codes_replaced: u64,
    /// Each ring file's first line's `ms`, newest file first: a roll pushes
    /// an empty one at the front and drops the oldest.
    ring_firsts: std::collections::VecDeque<Option<i64>>,
    /// See [`Picture::counted`].
    counted: u64,
    /// See [`Picture::since_ms`].
    since_ms: i64,
    /// The newest lines, each a reused buffer, at most [`RECENT_LINES`].
    recent: Vec<Vec<u8>>,
    /// Where the next line goes in `recent`.
    recent_next: usize,
}

impl Counters {
    /// Counters for a sink opening on `dir`: each ring file's first line read
    /// once, at most `keep` small reads, so the picture's oldest line is
    /// right from the first answer.
    fn open(dir: &Path, keep: u8) -> Self {
        let mut ring_firsts: std::collections::VecDeque<Option<i64>> =
            paths_newest_first(dir, keep)
                .iter()
                .map(|path| head_ms(path))
                .collect();
        ring_firsts.resize(usize::from(keep.max(1)), None);
        Self::with_firsts(ring_firsts)
    }

    /// Counters for a sink around a target this crate did not open.
    fn empty(keep: u8) -> Self {
        let mut ring_firsts = std::collections::VecDeque::new();
        ring_firsts.resize(usize::from(keep.max(1)), None);
        Self::with_firsts(ring_firsts)
    }

    /// The common construction.
    fn with_firsts(ring_firsts: std::collections::VecDeque<Option<i64>>) -> Self {
        Self {
            minutes: [(i64::MIN, [0; 5]); PICTURE_MINUTES],
            codes: Vec::with_capacity(PICTURE_CODES),
            codes_replaced: 0,
            ring_firsts,
            counted: 0,
            since_ms: now_millis(),
            recent: Vec::new(),
            recent_next: 0,
        }
    }

    /// Counts one line written at `at`, at `level`, carrying `code`, and keeps
    /// its copy. O(1) but for the code table, a scan of at most
    /// [`PICTURE_CODES`] entries, and the copy, one `memcpy` into a buffer
    /// that already has the room once it has held a line as wide.
    fn count(&mut self, at: i64, level: Level, code: Option<&str>, line: &[u8]) {
        let minute = at.div_euclid(60_000);
        let width = i64::try_from(PICTURE_MINUTES).unwrap_or(60);
        let slot = usize::try_from(minute.rem_euclid(width)).unwrap_or(0);
        if let Some((held, counts)) = self.minutes.get_mut(slot) {
            if *held != minute {
                *held = minute;
                *counts = [0; 5];
            }
            if let Some(count) = counts.get_mut(usize::from(level.rank())) {
                *count = count.saturating_add(1);
            }
        }
        if let Some(first) = self.ring_firsts.front_mut()
            && first.is_none()
        {
            *first = Some(at);
        }
        self.counted = self.counted.saturating_add(1);
        if let Some(code) = code {
            self.count_code(code);
        }
        if self.recent.len() < RECENT_LINES {
            self.recent.push(line.to_vec());
        } else if let Some(held) = self.recent.get_mut(self.recent_next) {
            held.clear();
            held.extend_from_slice(line);
        }
        self.recent_next = self.recent_next.saturating_add(1) % RECENT_LINES;
    }

    /// One more line of `code`: its own count, a free entry, or the place of
    /// the least-counted code with that count and one (space-saving).
    fn count_code(&mut self, code: &str) {
        if let Some(entry) = self.codes.iter_mut().find(|(held, _)| held == code) {
            entry.1 = entry.1.saturating_add(1);
            return;
        }
        if self.codes.len() < PICTURE_CODES {
            self.codes.push((code.to_owned(), 1));
            return;
        }
        if let Some(least) = self.codes.iter_mut().min_by_key(|(_, count)| *count) {
            *least = (code.to_owned(), least.1.saturating_add(1));
            self.codes_replaced = self.codes_replaced.saturating_add(1);
        }
    }

    /// The ring rolled: a new current file, the oldest dropped.
    fn rolled(&mut self) {
        self.ring_firsts.push_front(None);
        let _dropped = self.ring_firsts.pop_back();
    }

    /// The picture at minute `now_minute` (`ms` / 60,000).
    fn picture(&self, now_minute: i64) -> Picture {
        let width = i64::try_from(PICTURE_MINUTES).unwrap_or(60);
        let minutes = (now_minute.saturating_sub(width - 1)..=now_minute)
            .map(|minute| {
                let slot = usize::try_from(minute.rem_euclid(width)).unwrap_or(0);
                let counts = self
                    .minutes
                    .get(slot)
                    .filter(|(held, _)| *held == minute)
                    .map_or([0; 5], |(_, counts)| *counts);
                (minute.saturating_mul(60_000), counts)
            })
            .collect();
        let mut codes = self.codes.clone();
        codes.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        Picture {
            minutes,
            codes,
            codes_replaced: self.codes_replaced,
            oldest_ms: self.ring_firsts.iter().rev().find_map(|first| *first),
            counted: self.counted,
            since_ms: self.since_ms,
        }
    }

    /// The newest lines' copies, newest first.
    fn recent(&self) -> Vec<Vec<u8>> {
        let held = self.recent.len();
        (1..=held)
            .filter_map(|back| {
                let at = (self.recent_next + RECENT_LINES - back) % RECENT_LINES;
                self.recent.get(at).cloned()
            })
            .collect()
    }
}

impl Inner {
    /// The durable copies of the line just rendered from `own_from` in
    /// `buf`, stamped `at` under `run`, whatever the ring said; the first
    /// failure, if one failed.
    ///
    /// THE AUDIT COPY: a refusal kept only when the ring could take it would
    /// be lost exactly when the disk is in trouble, which is when it matters
    /// most. The same bytes, less the newline led with for a torn ring
    /// fragment, which is the ring's and not this stream's. Only an audited
    /// event reaches this; any other pays one `Option` test and nothing else.
    /// D-3341 item 3.
    ///
    /// AND THE STATE STREAM'S, for a who-did-what line (R3-24): the same
    /// bytes again, in a stream that never rolls. Every other event pays the
    /// `audited` test it already paid.
    fn keep_durable(
        &mut self,
        audited: bool,
        own_from: usize,
        (at, run): (i64, u64),
        event: &Event<'_>,
    ) -> Option<String> {
        if !audited {
            return None;
        }
        let own = self.buf.get(own_from..).unwrap_or(&[]);
        let keys = EventKeys(event, run);
        let audit_failure = self
            .audit
            .as_mut()
            .and_then(|audit| audit.append(own, self.seq, event.level(), at, &keys));
        let state_failure = self
            .state
            .as_mut()
            .filter(|_| {
                crate::run_dirs::state_events::is_state_change(
                    event.level(),
                    event.target(),
                    event.message(),
                )
            })
            .and_then(|state| state.append(own, self.seq, event.level(), at, &keys));
        audit_failure.or(state_failure)
    }

    /// THE PICTURE AND THE LIVE COPY (R3-24): each line just written, counted
    /// and copied as it lands: the flood's own line first when one `led` the
    /// write (`(torn newline led, flood line led)`), then the event's.
    fn count_written(
        &mut self,
        (torn, flood): (bool, bool),
        own_from: usize,
        at: i64,
        event: &Event<'_>,
    ) {
        if flood {
            let held_line = self
                .buf
                .get(usize::from(torn)..own_from.saturating_sub(1))
                .unwrap_or(&[]);
            self.counters.count(at, Level::Warn, None, held_line);
        }
        let own = self.buf.get(own_from..).unwrap_or(&[]);
        let own = own.strip_suffix(b"\n").unwrap_or(own);
        let field = |key: &str| {
            event
                .fields()
                .iter()
                .find(|(name, _)| *name == key)
                .and_then(|&(_, value)| value.as_str())
        };
        let code = field("code").or_else(|| field("reason"));
        self.counters.count(at, event.level(), code, own);
    }

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
    fn stamp(&mut self, now: i64) -> (i64, bool) {
        let held = now < self.last_at;
        self.last_at = now.max(self.last_at);
        (self.last_at, held)
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
    /// See [`Health::clock_held`].
    clock_held: AtomicU64,
    /// See [`Config::flood_lines`]. Read-only after construction.
    flood_lines: u32,
    /// See [`Health::rationed`].
    rationed: AtomicU64,
    /// The exclusive lock on `<dir>/events.lock`, held for the life of the
    /// sink and released by the kernel when the file closes, or earlier by
    /// [`Sink::close`]. `None` for a sink around a target this crate did not
    /// open. See [`Sink::open`].
    held_lock: Option<File>,
    /// Set once by [`Sink::close`], under the ring's lock; never cleared. A
    /// closed sink writes nothing more: every later event is [`Emitted::Dropped`]
    /// and counted.
    closed: AtomicBool,
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
            .field(
                "holds_directory",
                &(self.held_lock.is_some() && !self.closed.load(Ordering::Relaxed)),
            )
            .finish_non_exhaustive()
    }
}

impl Sink {
    /// Opens the set in `config.dir`, creating the directory if it is not
    /// there.
    ///
    /// The sequence number resumes from the last readable line of the current
    /// file, so a restart continues the stream rather than starting a second
    /// one at zero. That read is one block from the end of the newest
    /// non-empty file and nothing more, and it resumes the `ms` floor too; see
    /// `resume_point`.
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
        let held = hold_directory(&config.dir)?;
        let path = current_path(&config.dir);
        let mut target = FileTarget::open(&path)
            .map_err(|e| format!("{}: cannot open the event stream — {e}", path.display()))?;
        let found = target.len();
        let resumed = resume_point(&config.dir, config.keep_files);
        // THE TORN TAIL IS CLOSED BEFORE THE FIRST APPEND. See
        // `terminate_torn_tail`: without this the first event of the new
        // process fuses onto whatever the old one was killed in the middle of.
        let (bytes, torn) = terminate_torn_tail(&mut target, &path, found);
        let mut sink = Self::around(
            config,
            Box::new(target),
            bytes,
            resumed.seq,
            resumed.last_at,
        );
        sink.held_lock = Some(held);
        // THE PICTURE'S OLDEST LINE IS THE RING'S FROM THE START (R3-24): each
        // ring file's first line, read once here.
        sink.inner
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .counters = Counters::open(&config.dir, config.keep_files);
        // THE AUDIT STREAM OPENS UNDER THE SAME DIRECTORY LOCK, so it has one
        // writer exactly as the ring does. A stream that cannot open is named
        // and the ring still serves: losing the long record must not cost the
        // short one (`CLAUDE.md` §4: loud, never silent). D-3341 item 3.
        if config.audit {
            let (audit, said) = Audit::open(&config.dir, config.keep_files);
            // THE STATE STREAM OPENS AFTER THE AUDIT STREAM, whose torn tail
            // is then closed, because a new one is seeded from it (R3-24).
            let (state, state_said) = Audit::open_state(&config.dir, config.keep_files);
            let inner = sink.inner.get_mut().unwrap_or_else(PoisonError::into_inner);
            inner.audit = Some(audit);
            inner.state = Some(state);
            for why in [said, state_said].into_iter().flatten() {
                sink.report(&why);
            }
        }
        // RUN IDS RESUME ABOVE EVERY RUN THE LOG CARRIES, not above the last
        // `seq` alone: a process that reserved two ids and wrote an event only
        // under the second left a `seq` below that id. hunt-costs-2, D-1536.
        sink.reserved_run = AtomicU64::new(resumed.seq.max(resumed.max_run));
        if let Some(why) = torn {
            sink.report(&why);
        }
        // A FLOOR AHEAD OF THE CLOCK IS NAMED, not carried silently.
        let now = now_millis();
        if resumed.last_at > now {
            sink.report(&format!(
                "{}: the last line on disk is stamped {} ms, ahead of the clock's {now} ms; \
                 every event is held at that stamp until the clock passes it, and \
                 Sink::health().clock_held counts them",
                path.display(),
                resumed.last_at
            ));
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
        Ok(Self::around(config, target, 0, 0, 0))
    }

    /// The common construction.
    fn around(
        config: &Config,
        target: Box<dyn Target>,
        bytes: u64,
        seq: u64,
        last_at: i64,
    ) -> Self {
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
                // THE LAST STAMP ON DISK, NOT `now_millis()` AND NOT ZERO.
                // Not the clock: a resumed sink must not claim the events
                // already in the file happened at the moment it opened. Not
                // zero either: a restart whose clock is behind the last line
                // would write an older line after a newer one, which ends
                // `tail`'s `since` walk early. See `resume_point`; zero only
                // when nothing on disk could be read. D-1325.
                last_at,
                buf: Vec::with_capacity(512),
                torn: false,
                audit: None,
                state: None,
                flood: Flood::default(),
                counters: Counters::empty(config.keep_files),
            }),
            written: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            rotations: AtomicU64::new(0),
            rotation_failures: AtomicU64::new(0),
            reported: AtomicBool::new(false),
            last_error: Mutex::new(None),
            clock_held: AtomicU64::new(0),
            flood_lines: config.flood_lines,
            rationed: AtomicU64::new(0),
            held_lock: None,
            closed: AtomicBool::new(false),
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
    /// of prefix comparisons, each bounded by [`MAX_TARGET_BYTES`] because
    /// `Config::refusal` refuses a longer prefix (OD-4, D-2373).
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
        // A CLOSED SINK WRITES NOTHING (R3-14): its directory may already
        // belong to the next process, so a late line (an abandoned thread's,
        // say) is dropped and counted, never appended to that process's file.
        // Read under the ring's lock, which `close` sets it under, so no line
        // lands after the close. See `Sink::close`.
        if self.closed.load(Ordering::Relaxed) {
            drop(guard);
            return self.after_close(event);
        }
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
        let (at, held) = inner.stamp(now_millis());
        if held {
            self.clock_held.fetch_add(1, Ordering::Relaxed);
        }
        // THE FLOOD RATION (DB-A43): a `warn` pair past its lines this window
        // is held back and counted; never an `error`, never an audited line.
        // One level test for every other event. See `DEFAULT_FLOOD_LINES`.
        let audited = is_audited(event.level(), event.target(), event.message());
        let mut held_before = 0u64;
        if self.flood_lines > 0 && event.level() == Level::Warn && !audited {
            match inner
                .flood
                .admit(self.flood_lines, event.target(), event.message(), at)
            {
                Rationed::Hold => {
                    drop(guard);
                    self.rationed.fetch_add(1, Ordering::Relaxed);
                    return Emitted::Filtered;
                }
                Rationed::Write { held_before: n } => held_before = n,
            }
        }
        inner.seq = inner.seq.saturating_add(1);
        inner.buf.clear();
        // A FRAGMENT THE LAST FAILURE COULD NOT CLOSE IS CLOSED FIRST. See
        // `Inner::torn`.
        let led = inner.torn;
        if led {
            inner.buf.push(b'\n');
        }
        // WHAT THE LAST WINDOW HELD BACK IS SAID BEFORE THIS LINE, as a line of
        // its own in the same write: its pair, its count and the window.
        if held_before > 0 {
            let said = Event::warn(event.target(), FLOOD_MESSAGE)
                .with("message", event.message())
                .with("held", held_before)
                .with("window_ms", FLOOD_WINDOW_MS);
            line(&mut inner.buf, inner.seq, at, run, &said);
            inner.seq = inner.seq.saturating_add(1);
        }
        let own_from = inner.buf.len();
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
        let audit_failure = inner.keep_durable(audited, own_from, (at, run), event);
        match landed {
            Ok(()) => {
                inner.bytes = inner.bytes.saturating_add(span);
                inner.torn = false;
                inner.count_written((led, held_before > 0), own_from, at, event);
                drop(guard);
                if let Some(why) = roll_failure {
                    self.report(&why);
                }
                if let Some(why) = audit_failure {
                    self.report(&why);
                }
                self.written
                    .fetch_add(1 + u64::from(held_before > 0), Ordering::Relaxed);
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
                // AND WHEN THAT BYTE FAILS TOO — a disk still full — the
                // fragment is remembered, and the next append leads with the
                // newline. Discarding this result let the next `Written` event
                // fuse onto the fragment. attacksweep-2, D-1539.
                inner.torn = inner.target.append(b"\n").is_err();
                drop(guard);
                // A ROLL THAT ALSO FAILED IS REPORTED FIRST, so the notice
                // names the failure that came first. `report` prints once per
                // sink, so with both failing this is the one an operator sees —
                // and the roll is the earlier and more explanatory of the two.
                if let Some(why) = roll_failure {
                    self.report(&why);
                }
                if let Some(why) = audit_failure {
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

    /// An event that came after [`Sink::close`]: dropped, counted and named.
    #[cold]
    fn after_close(&self, event: &Event<'_>) -> Emitted {
        self.dropped.fetch_add(1, Ordering::Relaxed);
        self.report(&format!(
            "{}: an event ({} / {}) came after the sink was closed at the process's end and \
             was not written",
            self.dir.display(),
            event.target(),
            event.message()
        ));
        Emitted::Dropped
    }

    /// Ends this sink's hold on its directory, for a process that is about to
    /// end: syncs what was written, marks the sink closed, closes the audit
    /// stream and releases `<dir>/events.lock`, in that order. Idempotent.
    ///
    /// # Why a process closes its sink before it frees its port (R3-14)
    ///
    /// The lock is otherwise held until the process's last instant, and a
    /// server's successor is started the moment the old one's port is free.
    /// When the port went first, a successor could start inside the gap,
    /// find this lock still held and run its whole life with no log and no
    /// audit stream. Closing the sink first means a free port means a free
    /// log directory.
    ///
    /// After the close every event is [`Emitted::Dropped`], counted in
    /// [`Health::dropped`] and named once on stderr, never written: the
    /// directory may already be the successor's. [`Sink::audit_health`] is
    /// `None` from then on.
    ///
    /// # Errors
    ///
    /// The sync's or the unlock's failure in its own words; the sink is
    /// closed and the lock's file handle dropped with the process either way.
    pub fn close(&self) -> Result<(), String> {
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        if self.closed.swap(true, Ordering::Relaxed) {
            return Ok(());
        }
        let synced = guard.target.sync();
        guard.audit = None;
        guard.state = None;
        drop(guard);
        let unlocked = self.held_lock.as_ref().map_or(Ok(()), File::unlock);
        synced.and(unlocked).map_err(|e| {
            let why = format!(
                "{}: the sink was closed, and its sync or its unlock failed — {e}",
                self.dir.display()
            );
            self.report(&why);
            why
        })
    }

    /// What the audit stream has done, and failed to do: `None` when this
    /// sink keeps none (a [`Target`] sink, or [`Config::audit`] off).
    ///
    /// One uncontended lock and a copy of a handful of counters: no
    /// `metadata` call and no read, for the reason [`Sink::health`] gives.
    #[must_use]
    pub fn audit_health(&self) -> Option<AuditHealth> {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        guard.audit.as_ref().map(Audit::health)
    }

    /// The `/logs` picture (R3-24): lines per minute by level over the last
    /// [`PICTURE_MINUTES`], the codes written most, and the ring's oldest
    /// line, from the counters this sink keeps as it writes. One lock and a
    /// copy of a fixed-size table: no file is opened, whatever the ring holds.
    #[must_use]
    pub fn picture(&self) -> Picture {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        guard.counters.picture(now_millis().div_euclid(60_000))
    }

    /// The newest lines this sink wrote, newest first, at most
    /// [`RECENT_LINES`], decoded (R3-24): the live tail, read from the copy
    /// the sink keeps as it writes and never from a file. The copies are
    /// taken under the lock and decoded after it is released. A copy that
    /// will not decode is counted in the second value and passed over.
    #[must_use]
    pub fn recent(&self) -> (Vec<Record>, u64) {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let lines = guard.counters.recent();
        drop(guard);
        let mut malformed = 0u64;
        let records = lines
            .iter()
            .filter_map(|line| {
                let decoded = Record::decode(line).ok();
                if decoded.is_none() {
                    malformed = malformed.saturating_add(1);
                }
                decoded
            })
            .collect();
        (records, malformed)
    }

    /// What the never-rolling state stream has done, and failed to do
    /// (R3-24): `None` exactly when [`Self::audit_health`] is. Its
    /// [`AuditHealth::never_rolls`] is set and its rotation and reaper counts
    /// stay zero: it rolls nothing.
    #[must_use]
    pub fn state_health(&self) -> Option<AuditHealth> {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        guard.state.as_ref().map(Audit::health)
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
            clock_held: self.clock_held.load(Ordering::Relaxed),
            rationed: self.rationed.load(Ordering::Relaxed),
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
        inner.counters.rolled();
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

/// Where a reopened sink carries on from: the sequence number AND the `ms`
/// stamp of the newest readable record in the set.
///
/// # Why the set and not only the current file
///
/// It read the current file alone, and an EMPTY current file is exactly what a
/// set holds straight after a roll that renamed the old file aside and then
/// failed to append (a full disk), or after the current file was removed: the
/// newest history is then in `.1`. The numbering restarted at zero on top of
/// it — so `Sink::reserve_run_id`, which resumes from this number, handed a
/// restarted process run ids still carried by events in the rolled files, and
/// `Tail::missing` stopped being able to see a gap across the restart. The
/// first NON-EMPTY file, newest first, is the evidence of where the stream
/// stands. D-1326.
///
/// # Why the stamp too
///
/// `Inner::last_at` is the floor `ms` is clamped to so that `ms` order IS file
/// order, which `tail`'s `since` filter ends its walk on. It started a resumed
/// sink at zero, so a process restarted with its clock behind the last line on
/// disk — NTP stepping a rebooted host back, a restored VM — wrote a line older
/// than the line before it, and a `since` query landing between the two
/// returned nothing at all: the walk met the new, older line first and stopped.
/// Resuming the floor from the same record the sequence comes from carries the
/// clamp across the restart. D-1325.
///
/// # Limits, stated
///
/// Never fails: a set with no non-empty file, an unreadable newest file, and a
/// newest file whose last block holds no line that decodes all mean the same
/// thing — this build cannot tell what came before, so it starts at zero with
/// no floor. That is a **stated limit and not a silent one**: a corrupted tail
/// restarts the numbering. The newest non-empty file is the only one asked;
/// a corrupt one is not skipped in favour of an older file, whose number could
/// be lower than lines this one still holds.
///
/// The cost is one `stat` per empty or absent file passed over (at most
/// `keep_files`) and one 64 KiB read, once, at open — never on the write path.
///
/// # The run floor, and its limit
///
/// `max_run` is the largest `run` among the lines of that same block that
/// decode. `Sink::reserve_run_id` resumes above it as well as above `seq`, so a
/// process that reserved several ids and wrote an event only under a later one
/// does not hand that id out again (hunt-costs-2, D-1536). **Only the last block
/// is read**: a run id carried only by lines further back than 64 KiB, and
/// above every `seq` since, is not seen. That is a stated limit in
/// `docs/06-limits.md`, not a guarantee over the whole set.
fn resume_point(dir: &Path, keep_files: u8) -> Resumed {
    paths_newest_first(dir, keep_files)
        .iter()
        .find_map(|path| {
            let len = std::fs::metadata(path).map_or(0, |meta| meta.len());
            (len > 0).then_some((path, len))
        })
        .and_then(|(path, len)| last_records(path, len))
        .unwrap_or_default()
}

/// What a reopened sink resumes from. See [`resume_point`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Resumed {
    seq: u64,
    last_at: i64,
    max_run: u64,
}

/// The last line of one file that decodes, and the largest run in the block,
/// read from one block at its end.
fn last_records(path: &Path, len: u64) -> Option<Resumed> {
    /// One block at the end. 64 KiB holds ~250 lines at the typical width, so
    /// the last complete line is inside it unless one line is larger than the
    /// block, which the ceilings in `crate::event` make impossible: the widest
    /// legal line is just over 41 KB (`tail::MAX_LINE_BYTES`).
    const BLOCK: u64 = 64 * 1024;
    let from = len.saturating_sub(BLOCK);
    let bytes = crate::tail::read_at(path, from, len.saturating_sub(from)).ok()?;
    let mut decoded = bytes
        .split(|&b| b == b'\n')
        .rev()
        .filter_map(|line| Record::decode(line).ok());
    let last = decoded.next()?;
    let max_run = decoded.fold(last.run, |most, record| most.max(record.run));
    Some(Resumed {
        seq: last.seq,
        last_at: last.at_unix_millis,
        max_run,
    })
}

/// Takes the directory's exclusive lock, or refuses naming who holds it.
///
/// **One sink per directory, across processes.** Two sinks on one set both
/// resumed the same `seq`, reserved the same run ids, and each rolled the
/// other's current file out from under it. The lock is `flock` on
/// `<dir>/events.lock`, released by the kernel when the holder closes it or
/// dies, so a crash leaves nothing to clean up. A refusal is the open's own
/// error, which `install` callers already print and carry on without a log
/// (hunt-costs-3, D-1537).
fn hold_directory(dir: &Path) -> Result<File, String> {
    let path = dir.join("events.lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|e| format!("{}: cannot open the sink's lock — {e}", path.display()))?;
    // ONE REFUSAL FOR BOTH WAYS THE LOCK IS NOT HELD (D-1464). `WouldBlock`
    // is another sink; any other error is a filesystem that cannot take the
    // lock at all, which no test can stage, so its own arm was four lines the
    // coverage gate could never see run. Converted to the `io::Error` std
    // defines for it, the cause is printed either way and the refusal is the
    // same: no sink writes a directory it could not lock.
    file.try_lock().map_err(std::io::Error::from).map_err(|e| {
        format!(
            "{}: another sink, in this process or another, holds this telemetry directory, \
             or its lock {} cannot be taken ({e}); refused rather than writing one event \
             stream from two writers",
            dir.display(),
            path.display()
        )
    })?;
    Ok(file)
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
/// `resume_point` does not close it either, and reading it is what makes the gap
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
/// * **Its sequence number.** `resume_point` resumes from the last DECODABLE
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

// ===========================================================================
// THE AUDIT STREAM (D-3341 item 3; DB-023, DB-G88)
// ===========================================================================
//
// # Why a second stream
//
// The ring above is bounded at 64 MiB, and during a pull it held 4 to 12
// minutes: one 8 MiB file rolled every 35 seconds, two thirds of it expected
// absences and per-day pull refusals (DB-023, DB-G88). A refusal logged at
// 14:03 was gone by 14:15, so the one question the log exists for, "why was
// this day refused", had no answer an hour later.
//
// The audit stream keeps only what that question needs: every event at
// `error` (a refusal names itself at that level throughout this workspace),
// and every run lifecycle event at `info` or above ([`is_audited`]). It is the
// same line, byte for byte, as the ring's, so one decoder reads both. It rolls
// at [`AUDIT_MAX_FILE_BYTES`] and keeps [`AUDIT_KEEP_FILES`] files: at most
// 1 GiB while rotation works, against the ring's 64 MiB, and holding far fewer
// kinds of line.
//
// # The index: a path probe, not a scan
//
// Beside the stream, `audit.idx/<dimension>/<value>.idx` holds one fixed
// 32-byte [`AuditPointer`] per audited event that carries that value: the
// event's `day`, its `month` (or its day's), its `feed` (or `vendor`), its
// `instrument` (or `symbol`), its `contract`, its `target`, its `run` (its
// top-level run, or a numeric `run` field) and its `code` (or `reason`): the
// last two added by WP-33c so a run and a refusal reason are each one probe
// (DB-A41, DB-A47). A reader
// asking "what was refused for NIFTY on 2022-03-07" opens ONE file named by the
// question and reads the pointers at its end; nothing is scanned and nothing is
// planned. `CLAUDE.md` §4: the path is the index.
//
// The field names are the ones the workspace already writes; this crate knows
// no vendor and no instrument, only which field a value sits in. An event that
// carries none of them is still in the stream and indexed by its target.
//
// # What it costs, stated
//
// An event that is not audited pays one `Option` test and, when the sink keeps
// a stream, [`is_audited`]: one level comparison and at most one pass over its
// message, which `crate::event` bounds. An audited event pays one more
// `write` of its line and at most eight index appends of 32 bytes, each through
// a posting file kept open between events ([`AUDIT_HANDLES`] of them; a miss
// is one `open`, and at most one `create_dir_all`). None of that grows with the
// stream, the index or the number of earlier events. Once per
// [`AUDIT_MAX_FILE_BYTES`] the stream rolls: at most `2 * AUDIT_KEEP_FILES`
// renames and two small writes, one more rename for the index generation it
// retires and one channel send, amortised over every line of a 64 MiB file
// and never a delete (D-4838, below).
// Nothing is `fsync`ed per event, for the reason the module header gives; the
// stream is long-lived, not power-cut-proof.
//
// **What is measured, and what is not.** The emit that triggers a roll is
// timed by `crates/telemetry/benches/ratio.rs` (C-AU-01) with the retired
// generation at 10, 1,000 and 100,000 posting files. An ordinary audited emit
// (the line and its index appends) is still UNVERIFIED as a measurement: no
// bench in this crate times one, and the claim above is a count of operations
// (`CLAUDE.md` §3 rule 6).
//
// # Retiring an index generation: renamed on the emit path, deleted off it
// (D-4838; IA-11r)
//
// A roll past [`AUDIT_KEEP_FILES`] ends one index generation: the one whose
// audit file the roll just dropped. Until IA-11r the roll deleted that tree
// itself, `remove_dir_all` under the sink's lock, so every emitter in the
// process waited for a delete whose cost is the number of posting files the
// generation held (F26). Now the roll RENAMES the generation from
// `audit.idx/<serial>` to `audit.idx.gone/<serial>`, one rename inside the
// sink's own directory whatever the tree holds, and hands the new path to the
// stream's reaper: one thread per sink ([`REAPER_THREAD`], the only thread
// this crate starts in production), which waits on a channel and deletes
// outside every lock of the sink. A reader finds a renamed generation absent
// from the instant of the rename, exactly as it found a deleted one
// ([`audit_postings_before`] passes over a generation that is not there).
//
// What waits is counted, never silent: [`AuditHealth::gone_waiting`] is the
// generations renamed aside and not yet deleted, [`AuditHealth::reaped`] the
// ones deleted, [`AuditHealth::reap_failures`] the deletes that failed (the
// entry then stays where it is until the next start, never retried in a
// loop), and [`AuditHealth::reaper_behind`] says so once more than
// [`AUDIT_KEEP_FILES`] wait. A process that exits in the middle of a delete
// leaves the rest of the tree in `audit.idx.gone/`; the next
// [`Sink::open`] on the directory lists that directory once and hands every
// entry to its reaper, and also retires the one generation a process killed
// between a roll's serial write and its rename could have left behind.
//
// The disk held meanwhile is the generations rolled since the reaper last
// finished: normally none or one, each at most eight 32-byte pointers per
// audited line of one 64 MiB file and a directory per distinct value
// (`docs/06-limits.md` L-IA-36). The roll's emit is timed at 10, 1,000 and
// 100,000 posting files, and the reaper's delete beside it, by
// `crates/telemetry/benches/ratio.rs` (C-AU-01, C-AU-02).
//
// # Seeding
//
// A stream that does not exist yet, beside a ring that does, starts by copying
// the ring's audited lines, oldest first: the refusals still on disk are kept
// before the ring rolls them away. Once, at open, bounded by the ring's own
// ceiling, and counted in [`AuditHealth::seeded`].
//
// # Retention, named (WP-33c, DB-A43)
//
// The stream holds at most [`AUDIT_MAX_FILE_BYTES`] x [`AUDIT_KEEP_FILES`] =
// 1 GiB while rotation works, and loses no audited line inside that bound: a
// line leaves only when its whole file rolls past the kept set, with its index
// generation. What it takes is bounded by what is audited: refusals at
// `error`, run lifecycles, and access records their writer rations (the api's
// `/db` reads: one sampled line and one count per route a minute, plus each
// refused read under the failed-request ration). At ~600 bytes a line that is
// some 1.8 million lines; how many days that is depends on how many refusals
// the operator's runs produce, so no day count is claimed here.

/// The audit stream's file stem: `audit.ndjson`, rolled to `audit.<n>.ndjson`.
pub const AUDIT_BASENAME: &str = "audit";

/// How large the audit file being appended to may get before it rolls: 64 MiB
/// (D-3341 item 3).
pub const AUDIT_MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// How many audit files are kept, the one being written included: 16, so the
/// stream holds at most 1 GiB while rotation works (D-3341 item 3).
pub const AUDIT_KEEP_FILES: u8 = 16;

/// The index directory beside the stream.
pub const AUDIT_INDEX_DIR: &str = "audit.idx";

/// Where a roll puts the index generation it is done with, beside
/// [`AUDIT_INDEX_DIR`] and never inside it, so the count of generations a
/// reader can reach is unchanged: `audit.idx.gone/<serial>`. The stream's
/// reaper deletes what it holds (D-4838).
const AUDIT_GONE_DIR: &str = "audit.idx.gone";

/// The name of the thread that deletes retired index generations: one per
/// sink that keeps an audit stream, and the only thread this crate starts
/// outside its tests (D-4838).
const REAPER_THREAD: &str = "telemetry-reaper";

/// One index record: serial, offset, seq, length, level and a mark.
pub const AUDIT_POINTER_BYTES: usize = 32;

/// [`AUDIT_POINTER_BYTES`] as a `u64`, for file arithmetic.
const AUDIT_POINTER_BYTES_U64: u64 = 32;

/// The dimensions the index keeps posting files for, in the order an event's
/// pointers are appended.
///
/// `run` and `code` were added by WP-33c (DB-A41, DB-A47): `run` is the
/// line's own run (its top-level `run`, else a numeric `run` field, which is
/// how the GDFL importer names its run on `run started` and `run finished`),
/// and `code` is a refusal's `code` field (else its `reason`), so every day
/// refused for one reason is one posting. A generation written before a
/// dimension existed does not index its earlier lines by it; [`audit_coverage`]
/// says from which offset each generation does.
///
/// `reason` and `kind` were added by R3-24: `reason` is a line's own `reason`
/// field, so a refusal whose `code` and `reason` differ is found by either,
/// and `kind` is its `kind` field (the GDFL importer's `indices`, `stocks` or
/// `options`), so the log can be asked by import kind. A generation opened by
/// an earlier build is covered by them from the offset this build opened it
/// at ([`AUDIT_DIMS_FILE`]), never read as holding nothing.
pub const AUDIT_DIMENSIONS: [&str; 10] = [
    "day",
    "month",
    "feed",
    "instrument",
    "contract",
    "target",
    "run",
    "code",
    "reason",
    "kind",
];

/// The file in each index generation holding the first and last event time
/// of its audit file (R3-24): 16 bytes, the first `ms` then the last, both
/// little-endian `i64`, the last [`i64::MAX`] while the file is still being
/// appended to. Written at the generation's first line and again when it
/// rolls, never per line, so a time-span question picks the files it reads
/// without opening the others.
pub const AUDIT_SPAN_FILE: &str = "span";

/// How large the never-rolling state stream may grow before its health says
/// so: 256 MiB. It is not a bound: the stream never rolls and never drops a
/// line, so what it holds is what the writers' state changes add, and past
/// this size the operator is told to look at what writes it (R3-24;
/// `docs/06-limits.md`, proposed in the R3-24 report).
pub const STATE_LOUD_BYTES: u64 = 256 * 1024 * 1024;

/// The first and last event time of one audit file, as its index generation
/// records them ([`AUDIT_SPAN_FILE`]) or, for a generation an earlier build
/// wrote, as its first and last lines say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditSpan {
    /// The `ms` of its first line.
    pub first_ms: i64,
    /// The `ms` of its last line, or `None` while it is the file being
    /// appended to: a line may land after any answer.
    pub last_ms: Option<i64>,
}

impl AuditSpan {
    /// Whether any line of the file can fall in `from..=to`, both `ms`.
    #[must_use]
    pub fn overlaps(&self, from: i64, to: i64) -> bool {
        self.first_ms <= to && self.last_ms.is_none_or(|last| last >= from)
    }
}

/// How far back one log directory's three streams reach (R3-24): the time of
/// the oldest line each still holds. `None` for a stream with no line, or
/// none that could be read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Reach {
    /// The rolling ring's oldest line: every event the sink wrote.
    pub ring_from: Option<i64>,
    /// The rolling audit stream's oldest line: refusals, access records and
    /// lifecycles, kept [`AUDIT_KEEP_FILES`] files.
    pub audit_from: Option<i64>,
    /// The never-rolling state stream's oldest line: who did what, kept for
    /// ever.
    pub state_from: Option<i64>,
}

impl Reach {
    /// The oldest audited line either durable stream holds: how far back the
    /// directory's audit history reaches.
    #[must_use]
    pub fn audited_from(&self) -> Option<i64> {
        match (self.audit_from, self.state_from) {
            (Some(audit), Some(state)) => Some(audit.min(state)),
            (one, other) => one.or(other),
        }
    }
}

/// The `ms` of the first line of `path` that decodes, reading at most one
/// line's ceiling from its start; `None` for a file that is not there, is
/// empty, or whose head does not decode.
fn head_ms(path: &Path) -> Option<i64> {
    use std::io::{BufRead as _, Read as _};
    let file = File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(
        file.take(u64::try_from(crate::tail::MAX_LINE_BYTES).unwrap_or(u64::MAX)),
    );
    let mut line = Vec::new();
    let _read = reader.read_until(b'\n', &mut line).ok()?;
    let line = line.strip_suffix(b"\n").unwrap_or(&line);
    Record::decode(line)
        .ok()
        .map(|record| record.at_unix_millis)
}

/// The span of the audit file of `serial`, in a stream whose current serial is
/// `current_serial`: its [`AUDIT_SPAN_FILE`] when that holds one, else its
/// first and last lines (one read at each end). `None` once the file has
/// rolled out of the kept set, or when it holds no line.
#[must_use]
pub fn audit_span(dir: &Path, current_serial: u64, serial: u64) -> Option<AuditSpan> {
    let path = audit_path_for(dir, current_serial, serial)?;
    let open = serial == current_serial;
    if let Ok(bytes) = std::fs::read(audit_generation_path(dir, serial).join(AUDIT_SPAN_FILE))
        && let Ok(image) = <[u8; 16]>::try_from(bytes.as_slice())
    {
        let half = |from: usize| {
            image
                .get(from..from.saturating_add(8))
                .and_then(|eight| <[u8; 8]>::try_from(eight).ok())
                .map_or(i64::MAX, i64::from_le_bytes)
        };
        let (first_ms, last) = (half(0), half(8));
        return Some(AuditSpan {
            first_ms,
            last_ms: (!open && last != i64::MAX).then_some(last),
        });
    }
    let first_ms = head_ms(&path)?;
    let last_ms = if open {
        None
    } else {
        let len = std::fs::metadata(&path).ok()?.len();
        last_records(&path, len).map(|last| last.last_at)
    };
    Some(AuditSpan { first_ms, last_ms })
}

/// How far back the streams of the log directory `dir` reach. One `open` per
/// ring file until the oldest present one is found and one line read from it;
/// the audit serial and at most [`AUDIT_KEEP_FILES`] span probes, each one
/// small read, stopping at the oldest kept generation that has a line; one
/// span probe of the state stream. Nothing scales with what the files hold.
#[must_use]
pub fn reach_of(dir: &Path, ring_keep: u8) -> Reach {
    let ring_from = paths_newest_first(dir, ring_keep)
        .iter()
        .rev()
        .find_map(|path| head_ms(path));
    let oldest_of = |stream: &Path| {
        let current = audit_serial(stream).ok()?;
        let oldest = current.saturating_sub(u64::from(AUDIT_KEEP_FILES.saturating_sub(1)));
        (oldest..=current)
            .find_map(|serial| audit_span(stream, current, serial).map(|s| s.first_ms))
    };
    Reach {
        ring_from,
        audit_from: oldest_of(dir),
        state_from: oldest_of(&crate::run_dirs::state_stream_of(dir)),
    }
}

/// The dimensions the first index build (D-3341 item 3) kept, from the start
/// of every generation it wrote. A generation with no [`AUDIT_DIMS_FILE`] was
/// written by that build: these six cover it whole, the rest not at all.
pub const AUDIT_FIRST_DIMENSIONS: [&str; 6] =
    ["day", "month", "feed", "instrument", "contract", "target"];

/// The file in each index generation naming, per dimension, the byte offset of
/// the generation's audit file from which its lines are indexed by it:
/// `<dimension> <offset>` per line. Written when a generation is opened by a
/// build that indexes a dimension the file does not name yet.
pub const AUDIT_DIMS_FILE: &str = "dims";

/// A target whose last segment is this is an access record its writer already
/// rations (the api's sampled `/db` reads, DB-A42), audited at `info` or
/// above.
pub const AUDIT_ACCESS_SEGMENT: &str = "access";

/// The words that mark a run's lifecycle in an event's message ("run started",
/// "command finished", "census refused; run stopped").
pub const AUDIT_LIFECYCLE_WORDS: [&str; 6] = [
    "started", "finished", "stopped", "ended", "paused", "resumed",
];

/// The longest posting name the index files, after encoding. A longer value is
/// counted in [`AuditHealth::unindexed`] and the event stays in the stream.
pub const AUDIT_MAX_POSTING_NAME: usize = 120;

/// Posting files kept open between events. Past it the set is emptied and
/// reopened on demand: a constant, so an append is never a search.
const AUDIT_HANDLES: usize = 64;

/// The mark closing every [`AuditPointer`] image.
const POINTER_MARK: [u8; 3] = *b"AXI";

/// The audit file being appended to.
#[must_use]
pub fn audit_current_path(dir: &Path) -> PathBuf {
    dir.join(format!("{AUDIT_BASENAME}.{EXTENSION}"))
}

/// The `n`th rolled audit file, newest at 1.
#[must_use]
pub fn audit_rolled_path(dir: &Path, n: u8) -> PathBuf {
    dir.join(format!("{AUDIT_BASENAME}.{n}.{EXTENSION}"))
}

/// The file holding the serial of the audit file being appended to.
#[must_use]
pub fn audit_serial_path(dir: &Path) -> PathBuf {
    dir.join(AUDIT_INDEX_DIR).join("serial")
}

/// The index generation of the audit file of `serial`: every pointer into
/// that file, and nothing else, so the generation is deleted with the file
/// and the index is bounded exactly as the stream is.
#[must_use]
pub fn audit_generation_path(dir: &Path, serial: u64) -> PathBuf {
    dir.join(AUDIT_INDEX_DIR).join(serial.to_string())
}

/// The posting file for `value` in `dimension` of the audit file of `serial`,
/// or `None` for a dimension the index does not keep or a value it cannot name
/// ([`AUDIT_MAX_POSTING_NAME`]).
#[must_use]
pub fn audit_posting_path(
    dir: &Path,
    serial: u64,
    dimension: &str,
    value: &str,
) -> Option<PathBuf> {
    if !AUDIT_DIMENSIONS.contains(&dimension) {
        return None;
    }
    let name = posting_name(value)?;
    Some(
        audit_generation_path(dir, serial)
            .join(dimension)
            .join(format!("{name}.idx")),
    )
}

/// `value` as a file name: every byte outside `A-Z a-z 0-9 - _` written `%XX`,
/// so no value can name a path outside its dimension or collide with another.
/// `None` when empty or longer than [`AUDIT_MAX_POSTING_NAME`] once encoded.
fn posting_name(value: &str) -> Option<String> {
    if value.is_empty() || value.len() > AUDIT_MAX_POSTING_NAME {
        return None;
    }
    let mut name = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' {
            name.push(char::from(byte));
        } else {
            name.push('%');
            name.push(hex_digit(byte >> 4));
            name.push(hex_digit(byte & 0x0f));
        }
    }
    (name.len() <= AUDIT_MAX_POSTING_NAME).then_some(name)
}

/// One upper-case hex digit.
fn hex_digit(nibble: u8) -> char {
    char::from(
        b"0123456789ABCDEF"
            .get(usize::from(nibble))
            .copied()
            .unwrap_or(b'0'),
    )
}

/// Whether an event belongs in the audit stream: every event at `error`, and
/// every event at `info` or above whose target ends in `lifecycle` or
/// [`AUDIT_ACCESS_SEGMENT`], whose message holds one of
/// [`AUDIT_LIFECYCLE_WORDS`] as a word, or that is a row of the state-change
/// table (`crate::run_dirs::state_events::TABLE`, R3-24).
///
/// `warn` is not a refusal here: the ring's flood was `warn` (an expected
/// absence logged once per month asked, DB-023), and a stream that took it
/// would roll as fast as the ring does. An access record is taken because its
/// writer bounds it before it is emitted (DB-A42); this crate cannot, as it
/// knows no route. A table row is taken whatever its words: "listening" and
/// "census compacted" are state changes no word rule can see, and until R3-24
/// they reached the rolling ring alone. O(message length), which
/// `crate::event` bounds, plus at most the table's length of comparisons.
#[must_use]
pub fn is_audited(level: Level, target: &str, message: &str) -> bool {
    if level.at_least(Level::Error) {
        return true;
    }
    if !level.at_least(Level::Info) {
        return false;
    }
    is_lifecycle(target, message)
        || target.rsplit('.').next() == Some(AUDIT_ACCESS_SEGMENT)
        || crate::run_dirs::state_events::row_of(target, message).is_some()
}

/// Whether `target` and `message` mark a run's lifecycle as the audit stream
/// reads one: a target ending `lifecycle`, or a message holding one of
/// [`AUDIT_LIFECYCLE_WORDS`] as a word. Shared by [`is_audited`] and the
/// state stream's own test, so the two cannot read a lifecycle differently.
#[must_use]
pub(crate) fn is_lifecycle(target: &str, message: &str) -> bool {
    target.rsplit('.').next() == Some("lifecycle")
        || message
            .split(|c: char| !c.is_ascii_alphabetic())
            .any(|word| AUDIT_LIFECYCLE_WORDS.contains(&word))
}

/// Where one audited line is: the serial of its file, its offset and length in
/// it, its `seq` (which a reader checks the line against) and its level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditPointer {
    /// The serial of the audit file the line was appended to.
    pub serial: u64,
    /// The line's first byte, in that file.
    pub offset: u64,
    /// The line's `seq`.
    pub seq: u64,
    /// The line's length, newline included.
    pub len: u32,
    /// The line's level.
    pub level: Level,
}

impl AuditPointer {
    /// Exactly the bytes the index appends.
    #[must_use]
    pub fn image(&self) -> [u8; AUDIT_POINTER_BYTES] {
        let mut out = [0u8; AUDIT_POINTER_BYTES];
        let parts: [&[u8]; 6] = [
            &self.serial.to_le_bytes(),
            &self.offset.to_le_bytes(),
            &self.seq.to_le_bytes(),
            &self.len.to_le_bytes(),
            &[self.level.rank()],
            &POINTER_MARK,
        ];
        let mut at = 0usize;
        for part in parts {
            for (dst, src) in out.iter_mut().skip(at).zip(part.iter()) {
                *dst = *src;
            }
            at = at.saturating_add(part.len());
        }
        out
    }

    /// One record of a posting file, or `None` when it is not one this crate
    /// wrote (a wrong mark or a level no build names).
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let le8 = |from: usize| -> Option<u64> {
            let eight = bytes.get(from..from.checked_add(8)?)?;
            Some(u64::from_le_bytes(<[u8; 8]>::try_from(eight).ok()?))
        };
        if bytes.len() != AUDIT_POINTER_BYTES || bytes.get(29..32) != Some(POINTER_MARK.as_slice())
        {
            return None;
        }
        Some(Self {
            serial: le8(0)?,
            offset: le8(8)?,
            seq: le8(16)?,
            len: u32::from_le_bytes(<[u8; 4]>::try_from(bytes.get(24..28)?).ok()?),
            level: Level::of_rank(*bytes.get(28)?)?,
        })
    }
}

/// What the audit stream has done, and what it failed to do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditHealth {
    /// The audit file being appended to.
    pub path: PathBuf,
    /// Its serial: how many times the stream has rolled since it began.
    pub serial: u64,
    /// Its size, from the stream's own running count.
    pub current_bytes: u64,
    /// Audited lines written by this process (seeded lines included).
    pub written: u64,
    /// Audited lines that could not be written and are gone from this stream
    /// (the ring may still hold them).
    pub dropped: u64,
    /// Times the stream rolled.
    pub rotations: u64,
    /// Times a roll failed. The first stops rotation for the life of the sink,
    /// as the ring's does, and the current file grows past its bound.
    pub rotation_failures: u64,
    /// Pointers appended to the index.
    pub index_records: u64,
    /// Pointers that could not be appended: the line is in the stream and a
    /// probe for that value will not find it.
    pub index_failures: u64,
    /// Values the index could not name ([`AUDIT_MAX_POSTING_NAME`]).
    pub unindexed: u64,
    /// Lines copied from the ring when the stream was first opened.
    pub seeded: u64,
    /// Index generations renamed aside to `audit.idx.gone/` and not deleted
    /// yet: the ones this process's rolls retired and the ones an earlier
    /// process left there, less those the reaper has deleted. An entry whose
    /// delete failed stays counted: it is still on disk (D-4838).
    pub gone_waiting: u64,
    /// Retired generations the reaper deleted in this process.
    pub reaped: u64,
    /// Deletes of a retired generation that failed. The entry stays in
    /// `audit.idx.gone/` for the next start, is not retried before it, and
    /// its failure is in [`Self::last_error`].
    pub reap_failures: u64,
    /// The reaper's most recent failed delete, in its own words: always
    /// here, whatever [`Self::last_error`] carries.
    pub reap_error: Option<String>,
    /// The most recent failure of the stream's writer, in its own words;
    /// when it has none, the reaper's ([`Self::reap_error`]).
    pub last_error: Option<String>,
    /// Whether this is the never-rolling state stream (R3-24): it keeps every
    /// line for ever, so its size is what an operator watches, and past
    /// [`STATE_LOUD_BYTES`] it is loud.
    pub never_rolls: bool,
}

impl AuditHealth {
    /// Whether an operator has to be told about the stream.
    #[must_use]
    pub const fn is_loud(&self) -> bool {
        self.dropped > 0
            || self.rotation_failures > 0
            || self.index_failures > 0
            || self.reap_failures > 0
            || self.reaper_behind()
            || self.grown_past_loud()
    }

    /// Whether the never-rolling state stream has grown past
    /// [`STATE_LOUD_BYTES`]. Always `false` for the rolling audit stream,
    /// whose size its roll bounds.
    #[must_use]
    pub const fn grown_past_loud(&self) -> bool {
        self.never_rolls && self.current_bytes > STATE_LOUD_BYTES
    }

    /// Whether more retired generations wait for the reaper than the stream
    /// keeps files ([`AUDIT_KEEP_FILES`]): the reaper is not keeping up with
    /// the rolls, or it could not be started, and the disk they hold is
    /// growing (D-4838; `docs/06-limits.md` L-IA-36).
    #[must_use]
    pub const fn reaper_behind(&self) -> bool {
        self.gone_waiting > AUDIT_KEEP_FILES as u64
    }
}

/// The text of a field an audited event carries, whether the event is being
/// written ([`EventKeys`]) or was read back from the ring to seed the stream
/// ([`Record`]).
trait Keyed {
    /// The event's target.
    fn target(&self) -> &str;
    /// The text of the field named `key`, when it is text.
    fn text(&self, key: &str) -> Option<&str>;
    /// The run the line belongs to: its top-level `run` when it has one, else
    /// a `run` field holding a whole number (the GDFL importer's `run started`
    /// and `run finished` name their run that way). `None` for neither.
    fn run(&self) -> Option<u64>;
}

/// An event being written, and the run it is stamped with, as [`Keyed`].
struct EventKeys<'e, 'a>(&'e Event<'a>, u64);

impl Keyed for EventKeys<'_, '_> {
    fn target(&self) -> &str {
        self.0.target()
    }

    fn text(&self, key: &str) -> Option<&str> {
        self.0
            .fields()
            .iter()
            .find(|(name, _)| *name == key)
            .and_then(|&(_, value)| value.as_str())
    }

    fn run(&self) -> Option<u64> {
        if self.1 != 0 {
            return Some(self.1);
        }
        self.0
            .fields()
            .iter()
            .find(|(name, _)| *name == "run")
            .and_then(|&(_, value)| match value {
                crate::value::Value::Uint(n) => Some(n),
                crate::value::Value::Int(n) => u64::try_from(n).ok(),
                crate::value::Value::Str(text) => whole_number(text),
                _ => None,
            })
            .filter(|run| *run != 0)
    }
}

impl Keyed for Record {
    fn target(&self) -> &str {
        &self.target
    }

    fn text(&self, key: &str) -> Option<&str> {
        self.field(key).and_then(crate::value::OwnedValue::as_str)
    }

    fn run(&self) -> Option<u64> {
        if self.run != 0 {
            return Some(self.run);
        }
        self.field("run")
            .and_then(|value| {
                value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(whole_number))
            })
            .filter(|run| *run != 0)
    }
}

/// `text` as a canonical whole number: digits only, no sign, no leading zero.
fn whole_number(text: &str) -> Option<u64> {
    text.parse::<u64>().ok().filter(|n| n.to_string() == text)
}

/// Whether `text` is a day as this workspace writes one: `YYYY-MM-DD`.
fn is_day_text(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(at, byte)| {
            if at == 4 || at == 7 {
                *byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
}

/// The posting values one event carries, one per [`AUDIT_DIMENSIONS`] entry.
fn postings_of(keys: &impl Keyed) -> [Option<std::borrow::Cow<'_, str>>; 10] {
    use std::borrow::Cow;
    let day = keys.text("day").filter(|text| is_day_text(text));
    let month = keys
        .text("month")
        .filter(|text| text.len() == 7 && is_day_text(&format!("{text}-01")))
        .or_else(|| day.and_then(|day| day.get(..7)));
    [
        day.map(Cow::Borrowed),
        month.map(Cow::Borrowed),
        keys.text("feed")
            .or_else(|| keys.text("vendor"))
            .map(Cow::Borrowed),
        keys.text("instrument")
            .or_else(|| keys.text("symbol"))
            .map(Cow::Borrowed),
        keys.text("contract").map(Cow::Borrowed),
        Some(Cow::Borrowed(keys.target())),
        keys.run().map(|run| Cow::Owned(run.to_string())),
        keys.text("code")
            .or_else(|| keys.text("reason"))
            .map(Cow::Borrowed),
        keys.text("reason").map(Cow::Borrowed),
        keys.text("kind").map(Cow::Borrowed),
    ]
}

/// Whether the audit stream in `dir` exists, and its serial when it does:
/// `Ok(None)` when neither its file nor its serial is there, which is a
/// stream that was never written, NOT one that holds nothing (DB-A46). A
/// reader says "no audit stream" for the first and "0 lines" only for the
/// second.
///
/// # Errors
///
/// As [`audit_serial`].
pub fn audit_state(dir: &Path) -> Result<Option<u64>, String> {
    // `try_exists`, NOT `exists`: `exists` answers `false` for a path that
    // could not be asked about, so the stream of a directory nobody may
    // search read as "never written" and its reader said "nothing audited"
    // where it had to say "could not look" (D-4862).
    let there = |path: PathBuf| {
        path.try_exists()
            .map_err(|e| format!("{}: cannot be looked for — {e}", path.display()))
    };
    if !there(audit_current_path(dir))? && !there(audit_serial_path(dir))? {
        return Ok(None);
    }
    audit_serial(dir).map(Some)
}

/// From which byte offset of the audit file of `serial` its lines are indexed
/// by `dimension`: `Some(0)` for the whole generation, `Some(n)` for the lines
/// from `n` on, `None` when none of them are (a generation written before the
/// dimension existed, or whose [`AUDIT_DIMS_FILE`] is unreadable).
///
/// A generation with no file was written by the first index build, which kept
/// [`AUDIT_FIRST_DIMENSIONS`] from its start and no other. One read of a file
/// of a few hundred bytes.
#[must_use]
pub fn audit_coverage(dir: &Path, serial: u64, dimension: &str) -> Option<u64> {
    let path = audit_generation_path(dir, serial).join(AUDIT_DIMS_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => dims_of(&text)
            .into_iter()
            .find(|(name, _)| name == dimension)
            .map(|(_, from)| from),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            AUDIT_FIRST_DIMENSIONS.contains(&dimension).then_some(0)
        }
        Err(_) => None,
    }
}

/// The `<dimension> <offset>` lines of a [`AUDIT_DIMS_FILE`], the first
/// naming of each dimension winning. A line this build cannot read is
/// skipped: it names nothing this reader could rely on.
fn dims_of(text: &str) -> Vec<(String, u64)> {
    let mut out: Vec<(String, u64)> = Vec::new();
    for line in text.lines() {
        let Some((name, from)) = line.split_once(' ') else {
            continue;
        };
        let Some(from) = whole_number(from) else {
            continue;
        };
        if !out.iter().any(|(held, _)| held == name) {
            out.push((name.to_owned(), from));
        }
    }
    out
}

/// The serial of the audit file being appended to, `0` before the stream has
/// ever rolled.
///
/// # Errors
///
/// A serial file that cannot be read, or holds anything but eight bytes.
pub fn audit_serial(dir: &Path) -> Result<u64, String> {
    let path = audit_serial_path(dir);
    match std::fs::read(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(format!("{}: {e}", path.display())),
        Ok(bytes) => <[u8; 8]>::try_from(bytes.as_slice())
            .map(u64::from_le_bytes)
            .map_err(|_| {
                format!(
                    "{}: {} byte(s), not the eight of a serial",
                    path.display(),
                    bytes.len()
                )
            }),
    }
}

/// The file a pointer of `serial` is in, when the current file is
/// `current_serial`, or `None` once it has rolled past [`AUDIT_KEEP_FILES`].
#[must_use]
pub fn audit_path_for(dir: &Path, current_serial: u64, serial: u64) -> Option<PathBuf> {
    let back = current_serial.checked_sub(serial)?;
    if back == 0 {
        return Some(audit_current_path(dir));
    }
    u8::try_from(back)
        .ok()
        .filter(|back| *back < AUDIT_KEEP_FILES)
        .map(|back| audit_rolled_path(dir, back))
}

/// What reading the line one pointer names found.
#[derive(Debug, Clone, PartialEq)]
pub enum AuditLine {
    /// The line, decoded, its `seq` the pointer's.
    Found(Record),
    /// The file it was in has rolled out of the kept set, or no longer holds
    /// that line.
    RolledOff,
    /// A file that is there could not be read, in the failure's own words.
    Unreadable(String),
}

/// The line `pointer` names: one positional read, decoded and checked against
/// the pointer's `seq`. A roll that raced the read moved the line one file
/// older, and that file is tried too. O(1): at most two reads of one line.
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
#[must_use]
pub fn audit_read(dir: &Path, current_serial: u64, pointer: &AuditPointer) -> AuditLine {
    for serial_now in [current_serial, current_serial.saturating_add(1)] {
        let Some(path) = audit_path_for(dir, serial_now, pointer.serial) else {
            continue;
        };
        match crate::tail::read_at(&path, pointer.offset, u64::from(pointer.len)) {
            Ok(bytes) => {
                let line = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
                if let Ok(record) = Record::decode(line)
                    && record.seq == pointer.seq
                {
                    return AuditLine::Found(record);
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::UnexpectedEof
                ) => {}
            Err(e) => return AuditLine::Unreadable(format!("{}: {e}", path.display())),
        }
    }
    AuditLine::RolledOff
}

/// The newest pointers of one posting file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Postings {
    /// Up to the number asked for, newest first.
    pub pointers: Vec<AuditPointer>,
    /// How many whole records the file holds.
    pub total: u64,
    /// Records among those read that would not decode, skipped.
    pub malformed: u64,
}

/// The newest `take` pointers `dimension = value` holds, newest first, across
/// every generation the stream keeps: per generation, newest first, one
/// `metadata` call and one read of at most the pointers still wanted, at the
/// posting file's end, whatever its length. At most [`AUDIT_KEEP_FILES`]
/// generations (and one more, for a roll whose serial was not yet recorded),
/// so a probe is a bounded number of path lookups, never a walk. A posting
/// that does not exist is an empty answer: nothing audited carried that value.
///
/// # Errors
///
/// The serial or a posting file that is there and cannot be read, or a
/// dimension or value the index cannot name.
pub fn audit_postings(
    dir: &Path,
    dimension: &str,
    value: &str,
    take: usize,
) -> Result<Postings, String> {
    let page = audit_postings_before(dir, dimension, value, take, None)?;
    Ok(Postings {
        pointers: page
            .pointers
            .into_iter()
            .map(|(_, pointer)| pointer)
            .collect(),
        total: page.total,
        malformed: page.malformed,
    })
}

/// Where one pointer sits in the index: the generation (the serial of the
/// audit file it points into) and its number in that generation's posting
/// file. A posting file is appended to and never rewritten while its
/// generation is kept, so a place is stable: the cursor a keyed `/logs.json`
/// page hands back (DB-A44).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PostingAt {
    /// The generation.
    pub serial: u64,
    /// The pointer's number in that generation's posting file, from 0.
    pub index: u64,
}

/// One page of a posting, read backwards from a place.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PostingsPage {
    /// Up to the number asked for, strictly older than the place asked from,
    /// newest first, each with its own place.
    pub pointers: Vec<(PostingAt, AuditPointer)>,
    /// How many whole records the posting holds across the kept generations.
    pub total: u64,
    /// Records among those read that would not decode, skipped (their places
    /// are passed over, never re-read).
    pub malformed: u64,
    /// Kept generations whose lines this dimension does not index from their
    /// start, with the offset it does index them from (`None`: not at all).
    /// A posting that misses a line there misses it because the line predates
    /// the dimension, not because nothing carried the value. Probed only for a
    /// dimension outside [`AUDIT_FIRST_DIMENSIONS`].
    pub uncovered: Vec<(u64, Option<u64>)>,
}

/// The newest `take` pointers `dimension = value` holds strictly older than
/// `before` (from the newest when `None`), newest first, each with its place.
///
/// Per generation, newest first: one `metadata` call and one read of at most
/// the pointers still wanted, at a computed offset in the posting file,
/// whatever its length. At most [`AUDIT_KEEP_FILES`] + 2 generations. For a
/// dimension a generation may not cover, one more small read per generation
/// ([`audit_coverage`]). O(generations + take), never a walk of a posting.
///
/// # Errors
///
/// The serial or a posting file that is there and cannot be read, or a
/// dimension or value the index cannot name.
pub fn audit_postings_before(
    dir: &Path,
    dimension: &str,
    value: &str,
    take: usize,
    before: Option<PostingAt>,
) -> Result<PostingsPage, String> {
    let current = audit_serial(dir)?;
    let mut out = PostingsPage::default();
    let oldest = current.saturating_sub(u64::from(AUDIT_KEEP_FILES));
    let newest = current.saturating_add(1);
    let check_coverage = !AUDIT_FIRST_DIMENSIONS.contains(&dimension);
    for serial in (oldest..=newest).rev() {
        let path = audit_posting_path(dir, serial, dimension, value)
            .ok_or_else(|| format!("the audit index keeps no posting for {dimension}={value:?}"))?;
        if check_coverage && audit_path_for(dir, current, serial).is_some_and(|file| file.exists())
        {
            let from = audit_coverage(dir, serial, dimension);
            if from != Some(0) {
                out.uncovered.push((serial, from));
            }
        }
        let len = match std::fs::metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{}: {e}", path.display())),
            Ok(meta) => meta.len(),
        };
        let total = len / AUDIT_POINTER_BYTES_U64;
        out.total = out.total.saturating_add(total);
        // THE PLACE ASKED FROM BOUNDS THIS GENERATION: newer generations are
        // passed over whole, the place's own one is read below its index.
        let end = match before {
            Some(at) if serial > at.serial => continue,
            Some(at) if serial == at.serial => total.min(at.index),
            _ => total,
        };
        let left = take.saturating_sub(out.pointers.len());
        let wanted = end.min(u64::try_from(left).unwrap_or(u64::MAX));
        if wanted == 0 {
            continue;
        }
        let first = end.saturating_sub(wanted);
        let bytes = crate::tail::read_at(
            &path,
            first.saturating_mul(AUDIT_POINTER_BYTES_U64),
            wanted.saturating_mul(AUDIT_POINTER_BYTES_U64),
        )
        .map_err(|e| format!("{}: {e}", path.display()))?;
        for (n, chunk) in bytes.chunks_exact(AUDIT_POINTER_BYTES).enumerate().rev() {
            let at = PostingAt {
                serial,
                index: first.saturating_add(u64::try_from(n).unwrap_or(u64::MAX)),
            };
            match AuditPointer::decode(chunk) {
                Some(pointer) => out.pointers.push((at, pointer)),
                None => out.malformed = out.malformed.saturating_add(1),
            }
        }
    }
    Ok(out)
}

/// The audit stream's writer, behind the sink's lock.
#[derive(Debug)]
struct Audit {
    dir: PathBuf,
    /// The file being appended to, or `None` when it could not be opened.
    file: Option<File>,
    /// Its running size.
    bytes: u64,
    /// Its serial.
    serial: u64,
    /// The roll bound: [`AUDIT_MAX_FILE_BYTES`], and smaller only in a test
    /// that has to see a roll without writing 64 MiB.
    max_file_bytes: u64,
    /// Set by the first failed roll, never cleared: see `Sink::rotation_broken`.
    rotation_broken: bool,
    /// Posting files open between events, at most [`AUDIT_HANDLES`].
    handles: std::collections::HashMap<PathBuf, File>,
    /// The counters [`Sink::audit_health`] copies out.
    health: AuditHealth,
    /// What the reaper counts, shared with its thread (D-4838).
    reap: std::sync::Arc<Reap>,
    /// The reaper's channel: a roll sends it each generation it renamed
    /// aside. `None` when the thread could not be started, which
    /// [`AuditHealth::last_error`] names; the entries then wait for the next
    /// start. Dropping the stream closes the channel and ends the thread.
    reaper: Option<std::sync::mpsc::Sender<Retired>>,
    /// Whether this stream rolls: the audit stream does, the state stream
    /// (`crate::run_dirs::STATE_STREAM_DIR`, R3-24) never does, so it starts
    /// no reaper and retires nothing.
    rolls: bool,
    /// The current generation's first line's `ms`, once it has one.
    span_first: Option<i64>,
    /// Its last line's `ms`, written into [`AUDIT_SPAN_FILE`] when it rolls.
    span_last: i64,
}

/// One retired index generation handed to the reaper: its path, and the
/// identity of the `audit.idx.gone/` it was handed over in (R3-16, RA-294).
#[derive(Debug)]
struct Retired {
    /// `audit.idx.gone/<name>`.
    path: PathBuf,
    /// The folder's own identity when the entry was handed over.
    gone: Held,
}

/// A directory's identity, `(device, inode)`, read without following a link.
/// A folder renamed away and another (or a link) put in its place has
/// another, so the reaper never deletes through a name that changed under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Held {
    /// `st_dev`.
    dev: u64,
    /// `st_ino`.
    ino: u64,
}

/// What a stream's reaper has done, counted where both the stream (under the
/// sink's lock) and the reaper's thread (outside it) can reach it. Atomics
/// and one small mutex for the words of a failure: neither side ever waits
/// for a delete to read or write them.
#[derive(Debug, Default)]
struct Reap {
    /// Entries in `audit.idx.gone/` handed over, or left for the next start,
    /// and not deleted.
    waiting: AtomicU64,
    /// Entries deleted.
    reaped: AtomicU64,
    /// Deletes that failed.
    failures: AtomicU64,
    /// The last failed delete, in its own words.
    said: Mutex<Option<String>>,
    /// Held by a test for as long as the reaper must not delete, so the test
    /// can look at what a roll alone did. Taken by the reaper around each
    /// delete; never taken by an emit.
    #[cfg(test)]
    gate: Mutex<()>,
}

impl Reap {
    /// Records one failure of the reaper's side, in its own words.
    fn fail(&self, why: String) {
        let _earlier = self.failures.fetch_add(1, Ordering::Relaxed);
        *self.said.lock().unwrap_or_else(PoisonError::into_inner) = Some(why);
    }
}

/// Whether `gone` (a sink's `audit.idx.gone`) is there, asked WITHOUT
/// following a link, and which directory it is: `Ok(None)` when nothing has
/// that name.
///
/// # Errors
///
/// When the name is taken by something that is not a directory (a symbolic
/// link, even one to a directory, or a file), or cannot be asked about: the
/// sink renames nothing into it and deletes nothing from it. The words are
/// the reason alone; the caller says what it could not do, and to what.
fn audit_gone_is_ours(gone: &Path) -> Result<Option<Held>, String> {
    use std::os::unix::fs::MetadataExt as _;
    match gone.symlink_metadata() {
        Ok(meta) if meta.is_dir() => Ok(Some(Held {
            dev: meta.dev(),
            ino: meta.ino(),
        })),
        Ok(_) => Err(
            "it is not a directory (a symbolic link or a file is in its place), so no retired \
             index generation is renamed into it and nothing in it is deleted; remove it"
                .to_owned(),
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Why the reaper may not delete `job` now, or `None` when the folder it was
/// handed over in is still that folder (R3-16, RA-294).
///
/// The start's look ([`audit_gone_is_ours`]) refused a link planted BEFORE
/// the listing; a folder renamed away and a link (or another folder) put in
/// its place AFTER the hand-over was still deleted through, because a
/// delete resolves the folder's name again. One `lstat` of the folder per
/// delete, compared with the identity read at the hand-over. A swap inside
/// the microseconds between this look and the delete's own open is not
/// closed: that needs a delete relative to a held directory handle, which
/// `std` offers only inside `remove_dir_all` and not to a caller
/// (`docs/06-limits.md`, named by R3-16).
fn changed_under(job: &Retired) -> Option<String> {
    let folder = job.path.parent().unwrap_or_else(|| Path::new(""));
    let now = match audit_gone_is_ours(folder) {
        Ok(Some(now)) if now == job.gone => return None,
        Ok(Some(_)) => "another directory has its name now".to_owned(),
        Ok(None) => "nothing has its name now".to_owned(),
        Err(why) => why,
    };
    Some(format!(
        "{}: is no longer the folder {} was retired into ({now}); nothing in it is deleted, \
         and the generation stays for the next start",
        folder.display(),
        job.path.display()
    ))
}

/// The reaper: deletes each retired index generation it is sent, until the
/// stream that owns the channel is dropped (D-4838).
///
/// It runs on its own thread and takes no lock of the sink, so an emit never
/// waits for it. One `remove_dir_all` per entry, whose cost is the entry's
/// posting files: `docs/06-limits.md` L-IA-36, timed by
/// `crates/telemetry/benches/ratio.rs` (C-AU-02). A delete that fails is
/// counted with its words and the entry is left where it is for the next
/// start; it is not tried again in this process, so a tree that cannot be
/// deleted costs one attempt, never a loop.
fn reaper(jobs: &std::sync::mpsc::Receiver<Retired>, reap: &Reap) {
    while let Ok(job) = jobs.recv() {
        #[cfg(test)]
        let _released_by_the_test = reap.gate.lock().unwrap_or_else(PoisonError::into_inner);
        // NEVER THROUGH A FOLDER THAT CHANGED UNDER IT (R3-16, RA-294).
        if let Some(why) = changed_under(&job) {
            reap.fail(why);
            continue;
        }
        let path = job.path;
        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            // GONE ALREADY is done: nothing of it waits.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                reap.fail(format!(
                    "{}: cannot delete the retired index generation — {e}; it stays for the \
                     next start",
                    path.display()
                ));
                continue;
            }
        }
        let _earlier = reap.reaped.fetch_add(1, Ordering::Relaxed);
        let _earlier = reap.waiting.fetch_sub(1, Ordering::Relaxed);
    }
}

/// What [`Audit::open_in`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opened {
    /// A stream that never existed: it is to be seeded.
    Fresh,
    /// A stream written before: its lines are never copied a second time.
    Existing,
    /// It could not be opened; the reason is in its health.
    Failed,
}

/// The state-change lines of several files, read in turn, each file from its
/// start: the line as written (without its newline) and decoded. A line that
/// does not decode is passed over; a file that cannot be opened is too, as
/// the audit stream's own seed passes it over.
struct StateLines {
    files: std::vec::IntoIter<PathBuf>,
    reader: Option<std::io::BufReader<File>>,
}

impl StateLines {
    /// The lines of `files`, in that order.
    fn new(files: Vec<PathBuf>) -> Self {
        Self {
            files: files.into_iter(),
            reader: None,
        }
    }
}

impl Iterator for StateLines {
    type Item = (Record, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        use std::io::BufRead as _;
        loop {
            let Some(reader) = self.reader.as_mut() else {
                let path = self.files.next()?;
                self.reader = File::open(&path).ok().map(std::io::BufReader::new);
                continue;
            };
            let mut line = Vec::new();
            if !matches!(reader.read_until(b'\n', &mut line), Ok(read) if read > 0) {
                self.reader = None;
                continue;
            }
            if line.last() == Some(&b'\n') {
                let _newline = line.pop();
            }
            if let Ok(record) = Record::decode(&line)
                && crate::run_dirs::state_events::is_state_change(
                    record.level,
                    &record.target,
                    &record.message,
                )
            {
                return Some((record, line));
            }
        }
    }
}

impl Audit {
    /// Opens the stream in `dir`, seeding it from the ring there when it is
    /// new, and says what went wrong, if anything did.
    fn open(dir: &Path, ring_keep: u8) -> (Self, Option<String>) {
        let (mut audit, said, opened) = Self::open_in(dir, true);
        match opened {
            Opened::Failed => return (audit, said),
            Opened::Fresh => audit.seed(ring_keep),
            Opened::Existing => {}
        }
        // THE REAPER STARTS LAST, with what an earlier process left it: a
        // stream that could not open returned above and starts no thread.
        let reaping = audit.start_reaper();
        (audit, said.or(reaping.err()))
    }

    /// Opens the never-rolling state stream of the log directory `parent`
    /// (R3-24): `<parent>/audit.state/`, laid out as an audit stream. A new
    /// one is seeded from what `parent` already holds ([`Self::seed_state`]).
    /// It rolls nothing, so it starts no reaper.
    fn open_state(parent: &Path, ring_keep: u8) -> (Self, Option<String>) {
        let dir = crate::run_dirs::state_stream_of(parent);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            let (stream, _, _) = Self::open_in(&dir, false);
            return stream.failed(format!(
                "{}: cannot create the state stream — {e}",
                dir.display()
            ));
        }
        let (mut state, said, opened) = Self::open_in(&dir, false);
        if opened == Opened::Fresh {
            state.seed_state(parent, ring_keep);
        }
        (state, said)
    }

    /// The opening both streams share: the serial, the file, a torn tail
    /// closed, the coverage marked and the span resumed. Says whether the
    /// stream is new (and so to be seeded), was there already, or could not
    /// be opened, in which case the reason is in the health and returned.
    fn open_in(dir: &Path, rolls: bool) -> (Self, Option<String>, Opened) {
        let current = audit_current_path(dir);
        let mut audit = Self {
            dir: dir.to_path_buf(),
            file: None,
            bytes: 0,
            serial: 0,
            max_file_bytes: AUDIT_MAX_FILE_BYTES,
            rotation_broken: false,
            handles: std::collections::HashMap::new(),
            health: AuditHealth {
                path: current.clone(),
                never_rolls: !rolls,
                ..AuditHealth::default()
            },
            reap: std::sync::Arc::new(Reap::default()),
            reaper: None,
            rolls,
            span_first: None,
            span_last: 0,
        };
        // A STREAM THAT HAS NEVER EXISTED, and only that, is seeded: a
        // current file or a serial on disk means a stream that was written
        // before, whose lines must not be copied a second time.
        let fresh = !current.exists() && !audit_serial_path(dir).exists();
        let failed = |audit: Self, why: String| {
            let (audit, said) = audit.failed(why);
            (audit, said, Opened::Failed)
        };
        audit.serial = match audit_serial(dir) {
            Ok(serial) => serial,
            Err(why) => return failed(audit, format!("the audit stream's serial: {why}")),
        };
        let mut target = match FileTarget::open(&current) {
            Ok(target) => target,
            Err(e) => {
                let why = format!("{}: cannot open the audit stream — {e}", current.display());
                return failed(audit, why);
            }
        };
        let found = target.len();
        // A TORN TAIL IS CLOSED BEFORE THE FIRST APPEND, as the ring's is.
        let (len, torn) = terminate_torn_tail(&mut target, &current, found);
        audit.file = Some(target.file);
        audit.bytes = len;
        // WHAT THIS BUILD INDEXES IS WRITTEN DOWN BEFORE ITS FIRST LINE, so a
        // reader can tell a posting that missed a line from a generation that
        // predates the dimension (WP-33c). Before the seed, so seeded lines
        // are covered.
        let marked = audit.mark_coverage();
        audit.resume_span();
        let opened = if fresh {
            Opened::Fresh
        } else {
            Opened::Existing
        };
        (audit, torn.or(marked.err()), opened)
    }

    /// Picks up the current generation's span where the last process left it
    /// (R3-24): its first line's time from [`AUDIT_SPAN_FILE`], or from the
    /// file's first line for a generation an earlier build wrote (and then
    /// records it), and its last line's time from the file's end. One small
    /// read at each end, once per open.
    fn resume_span(&mut self) {
        if self.bytes == 0 {
            return;
        }
        let current = audit_current_path(&self.dir);
        let recorded = audit_span(&self.dir, self.serial, self.serial).map(|span| span.first_ms);
        let first = recorded.or_else(|| head_ms(&current));
        self.span_last = last_records(&current, self.bytes).map_or(0, |last| last.last_at);
        if let Some(first) = first {
            self.span_first = Some(first);
            self.span_last = self.span_last.max(first);
            self.write_span(false);
        }
    }

    /// Writes the current generation's [`AUDIT_SPAN_FILE`]: its first line's
    /// time and, once it is `closed` by a roll, its last; [`i64::MAX`] while
    /// it is still appended to. One write of 16 bytes. A write that fails is
    /// counted with the index's failures and named: a reader then reads the
    /// file's own first and last lines, which is slower and never wrong.
    fn write_span(&mut self, closed: bool) {
        let Some(first) = self.span_first else {
            return;
        };
        let last = if closed { self.span_last } else { i64::MAX };
        let mut image = [0u8; 16];
        for (dst, src) in image
            .iter_mut()
            .zip(first.to_le_bytes().into_iter().chain(last.to_le_bytes()))
        {
            *dst = src;
        }
        let generation = audit_generation_path(&self.dir, self.serial);
        let path = generation.join(AUDIT_SPAN_FILE);
        if let Err(e) =
            std::fs::create_dir_all(&generation).and_then(|()| std::fs::write(&path, image))
        {
            self.health.index_failures = self.health.index_failures.saturating_add(1);
            self.health.last_error = Some(format!(
                "{}: cannot record the generation's time span — {e}",
                path.display()
            ));
        }
    }

    /// Copies what the log directory `parent` already holds of the state
    /// stream's lines into a new one, oldest first (R3-24): its audit
    /// stream's kept files and its ring, merged by `seq`, each line once.
    ///
    /// # Why both, and why merged
    ///
    /// The audit stream holds the lifecycles of every run since it began,
    /// long after the ring rolled them away; the ring holds the state changes
    /// no rule audited before R3-24 ("listening", "census compacted"). Each
    /// is written in `seq` order, so one pass over both, taking the lower
    /// `seq` each step and a line both hold once, gives one stream in the
    /// order the lines were written.
    ///
    /// # Cost
    ///
    /// Once, when the stream is new: one sequential read of at most the audit
    /// stream's ceiling ([`AUDIT_MAX_FILE_BYTES`] x [`AUDIT_KEEP_FILES`]) and
    /// the ring's, one line held at a time from each. Counted in
    /// [`AuditHealth::seeded`].
    fn seed_state(&mut self, parent: &Path, ring_keep: u8) {
        let audit_files: Vec<PathBuf> = (1..AUDIT_KEEP_FILES)
            .rev()
            .map(|n| audit_rolled_path(parent, n))
            .chain(std::iter::once(audit_current_path(parent)))
            .collect();
        let ring_files: Vec<PathBuf> = paths_newest_first(parent, ring_keep)
            .into_iter()
            .rev()
            .collect();
        let mut audited = StateLines::new(audit_files);
        let mut ring = StateLines::new(ring_files);
        let (mut left, mut right) = (audited.next(), ring.next());
        loop {
            let take_left = match (&left, &right) {
                (None, None) => break,
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (Some((one, one_line)), Some((other, other_line))) => {
                    // THE SAME LINE IN BOTH is one line: the ring's copy goes.
                    if one.seq == other.seq && one_line == other_line {
                        right = ring.next();
                        continue;
                    }
                    one.seq <= other.seq
                }
            };
            let taken = if take_left {
                std::mem::replace(&mut left, audited.next())
            } else {
                std::mem::replace(&mut right, ring.next())
            };
            if let Some((record, mut line)) = taken {
                line.push(b'\n');
                if self
                    .append(
                        &line,
                        record.seq,
                        record.level,
                        record.at_unix_millis,
                        &record,
                    )
                    .is_none()
                {
                    self.health.seeded = self.health.seeded.saturating_add(1);
                }
            }
        }
    }

    /// Starts the stream's reaper and hands it what is already waiting
    /// (D-4838): every entry `audit.idx.gone/` holds, listed once, and the
    /// one generation a process killed between a roll's serial write and its
    /// rename left in `audit.idx/`.
    ///
    /// One `read_dir` per start, over the generations rolled since a reaper
    /// last finished: `docs/06-limits.md` L-IA-36. A thread that cannot start
    /// is named and returned; the stream still writes, its rolls still rename
    /// aside, and what they rename waits, counted, for the next start.
    fn start_reaper(&mut self) -> Result<(), String> {
        self.start_reaper_with(|jobs, reap| {
            std::thread::Builder::new()
                .name(REAPER_THREAD.to_owned())
                .spawn(move || reaper(&jobs, &reap))
                .map(|_detached| ())
        })
    }

    /// [`Self::start_reaper`] with the thread's start handed in, so a test
    /// can state a start that fails, which no test can cause.
    fn start_reaper_with(
        &mut self,
        spawn: impl FnOnce(
            std::sync::mpsc::Receiver<Retired>,
            std::sync::Arc<Reap>,
        ) -> std::io::Result<()>,
    ) -> Result<(), String> {
        let (send, jobs) = std::sync::mpsc::channel::<Retired>();
        let started = spawn(jobs, std::sync::Arc::clone(&self.reap)).map_err(|e| {
            format!(
                "{}: cannot start the {REAPER_THREAD} thread — {e}; retired index generations \
                 wait in {AUDIT_GONE_DIR}/ for the next start",
                self.dir.display()
            )
        });
        match started {
            Ok(()) => self.reaper = Some(send),
            Err(ref why) => {
                self.reaper = None;
                self.health.last_error = Some(why.clone());
            }
        }
        // WHAT AN EARLIER PROCESS LEFT ASIDE, listed before anything this
        // start renames there, so no entry is handed over twice.
        let listed = self.hand_over_what_waits();
        if let Err(ref why) = listed {
            self.health.last_error = Some(why.clone());
        }
        // A PROCESS KILLED AFTER IT RECORDED A SERIAL AND BEFORE IT RENAMED
        // THE GENERATION THAT SERIAL ENDED left that one generation in
        // `audit.idx/`, where no later roll looks for it. Retired here: one
        // rename that finds nothing on every other start.
        let stranded = self
            .serial
            .checked_sub(u64::from(AUDIT_KEEP_FILES))
            .map_or(Ok(()), |gone| self.retire(gone));
        started.and(stranded).and(listed)
    }

    /// Lists `audit.idx.gone/` once and hands every retired generation in it
    /// to the reaper.
    ///
    /// # What is not handed over
    ///
    /// **The directory itself, when it is not one.** `read_dir` follows a
    /// symbolic link, so a link planted under this name would have had the
    /// reaper delete another directory's entries. Asked without following
    /// ([`audit_gone_is_ours`]), and refused by name: nothing in it is listed.
    ///
    /// **An entry that is not a directory.** A retired generation is a
    /// directory; a file there (a `.DS_Store` a file browser left) is not
    /// this sink's, and handing it over made a delete that fails at every
    /// start and a health that is loud for ever. It is left where it is,
    /// never counted as waiting or as a failed delete, and named apart in
    /// [`AuditHealth::last_error`] when the writer has no error of its own.
    fn hand_over_what_waits(&mut self) -> Result<(), String> {
        let gone = self.dir.join(AUDIT_GONE_DIR);
        let Some(held) = audit_gone_is_ours(&gone)
            .map_err(|why| format!("{}: cannot list — {why}", gone.display()))?
        else {
            return Ok(());
        };
        let cannot_list = |e: std::io::Error| format!("{}: cannot list — {e}", gone.display());
        let mut strays = 0u64;
        let mut first_stray = None;
        for entry in std::fs::read_dir(&gone).map_err(cannot_list)? {
            let entry = entry.map_err(cannot_list)?;
            if entry.file_type().map_err(cannot_list)?.is_dir() {
                self.hand_over(entry.path(), held);
            } else {
                strays = strays.saturating_add(1);
                let _kept = first_stray.get_or_insert_with(|| entry.path());
            }
        }
        if let Some(stray) = first_stray
            && self.health.last_error.is_none()
        {
            self.health.last_error = Some(format!(
                "{}: is not a retired index generation (it is not a directory), and {} such \
                 entr{} in {AUDIT_GONE_DIR}/ left where {}: nothing here made {}, so nothing \
                 here deletes {}",
                stray.display(),
                strays,
                if strays == 1 { "y is" } else { "ies are" },
                if strays == 1 { "it is" } else { "they are" },
                if strays == 1 { "it" } else { "them" },
                if strays == 1 { "it" } else { "them" },
            ));
        }
        Ok(())
    }

    /// Counts one entry of `audit.idx.gone/` as waiting and sends it to the
    /// reaper: one channel send, which never blocks. With no reaper the entry
    /// stays counted and waits for the next start.
    fn hand_over(&self, path: PathBuf, gone: Held) {
        let _earlier = self.reap.waiting.fetch_add(1, Ordering::Relaxed);
        // WITH NO THREAD (it could not be started) nothing is deleted on this
        // path instead: the entry stays, and the health counts it.
        if let Some(reaper) = self.reaper.as_ref() {
            let _waits_for_the_next_start_if_unsent = reaper.send(Retired { path, gone });
        }
    }

    /// Retires index generation `gone`: renames `audit.idx/<gone>` to
    /// `audit.idx.gone/<gone>` and hands it to the reaper (D-4838). A
    /// generation that is not there is nothing to retire.
    ///
    /// One `create_dir_all` of the sibling directory, one `rename` inside the
    /// sink's own directory and one channel send, whatever the generation
    /// holds; no delete. Proven by
    /// `telemetry::sink::tests::an_audit_roll_never_waits_for_a_delete`
    /// (IA-LG-01) and timed by `crates/telemetry/benches/ratio.rs` (C-AU-01).
    ///
    /// A name already taken in `audit.idx.gone/` (an entry an earlier stream
    /// of this directory left, which its delete could not remove) is not
    /// written over: the generation is renamed to a second, stamped name. A
    /// rename that fails both ways is the caller's failure, named.
    fn retire(&self, gone: u64) -> Result<(), String> {
        let generation = audit_generation_path(&self.dir, gone);
        let aside = self.dir.join(AUDIT_GONE_DIR);
        // A LINK UNDER THIS NAME IS NOT A PLACE TO RENAME INTO: the
        // generation would leave the sink's directory and the next start
        // would not find it. Refused by name; the generation stays in
        // `audit.idx/`, where a reader already takes it for rolled off.
        let _there = audit_gone_is_ours(&aside)
            .map_err(|why| format!("{}: cannot create — {why}", aside.display()))?;
        std::fs::create_dir_all(&aside)
            .map_err(|e| format!("{}: cannot create — {e}", aside.display()))?;
        // WHICH FOLDER the generation is renamed into, asked again now that
        // it is there: the reaper deletes only inside this one (RA-294).
        let held = audit_gone_is_ours(&aside)
            .map_err(|why| format!("{}: cannot create — {why}", aside.display()))?
            .ok_or_else(|| {
                format!(
                    "{}: cannot create — nothing has the name after it was made",
                    aside.display()
                )
            })?;
        let stamped = format!("{gone}.{}.{}", self.health.rotations, now_millis());
        let mut refused = None;
        for name in [gone.to_string(), stamped] {
            let to = aside.join(name);
            match std::fs::rename(&generation, &to) {
                Ok(()) => {
                    self.hand_over(to, held);
                    return Ok(());
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(e) => {
                    refused = Some(format!("{} -> {}: {e}", generation.display(), to.display()));
                }
            }
        }
        Err(format!(
            "cannot retire the index generation — {}",
            refused.unwrap_or_default()
        ))
    }

    /// Writes this build's coverage into the current generation's
    /// [`AUDIT_DIMS_FILE`]: every dimension it does not name yet, from the
    /// offset the stream stands at. A generation that has lines and no file was
    /// written by the first index build, so the dimensions it kept are written
    /// as covered from 0 and the others from here. One read and at most one
    /// append, at open and at each roll.
    fn mark_coverage(&mut self) -> Result<(), String> {
        let generation = audit_generation_path(&self.dir, self.serial);
        let path = generation.join(AUDIT_DIMS_FILE);
        let held = match std::fs::read_to_string(&path) {
            Ok(text) => Some(dims_of(&text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                let why = format!("{}: cannot read the index coverage — {e}", path.display());
                self.health.last_error = Some(why.clone());
                return Err(why);
            }
        };
        let older = self.bytes > 0 || generation.exists();
        let mut text = String::new();
        for dimension in AUDIT_DIMENSIONS {
            let from = match held {
                Some(ref held) if held.iter().any(|(name, _)| name == dimension) => continue,
                Some(_) => self.bytes,
                None if older && !AUDIT_FIRST_DIMENSIONS.contains(&dimension) => self.bytes,
                None => 0,
            };
            text.push_str(dimension);
            text.push(' ');
            text.push_str(&from.to_string());
            text.push('\n');
        }
        if text.is_empty() {
            return Ok(());
        }
        std::fs::create_dir_all(&generation)
            .and_then(|()| {
                OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&path)?
                    .write_all(text.as_bytes())
            })
            .map_err(|e| {
                let why = format!("{}: cannot record the index coverage — {e}", path.display());
                self.health.last_error = Some(why.clone());
                why
            })
    }

    /// The stream with no file, and the reason, recorded.
    fn failed(mut self, why: String) -> (Self, Option<String>) {
        self.health.last_error = Some(why.clone());
        (self, Some(why))
    }

    /// Copies the ring's audited lines, oldest first. See the section header.
    fn seed(&mut self, ring_keep: u8) {
        use std::io::BufRead as _;
        let ring = paths_newest_first(&self.dir, ring_keep);
        for path in ring.iter().rev() {
            let Ok(file) = File::open(path) else {
                continue;
            };
            for line in std::io::BufReader::new(file).split(b'\n') {
                let Ok(line) = line else {
                    break;
                };
                let Ok(record) = Record::decode(&line) else {
                    continue;
                };
                if !is_audited(record.level, &record.target, &record.message) {
                    continue;
                }
                let mut whole = line;
                whole.push(b'\n');
                if self
                    .append(
                        &whole,
                        record.seq,
                        record.level,
                        record.at_unix_millis,
                        &record,
                    )
                    .is_none()
                {
                    self.health.seeded = self.health.seeded.saturating_add(1);
                }
            }
        }
    }

    /// Appends one whole line, stamped `at`, and its pointers, or says why it
    /// could not.
    fn append(
        &mut self,
        line: &[u8],
        seq: u64,
        level: Level,
        at: i64,
        keys: &impl Keyed,
    ) -> Option<String> {
        let span = u64::try_from(line.len()).unwrap_or(u64::MAX);
        let mut said = None;
        if self.rolls
            && !self.rotation_broken
            && self.bytes > 0
            && self.bytes.saturating_add(span) > self.max_file_bytes
            && let Err(why) = self.roll()
        {
            self.rotation_broken = true;
            self.health.rotation_failures = self.health.rotation_failures.saturating_add(1);
            self.health.last_error = Some(why.clone());
            said = Some(why);
        }
        let Some(file) = self.file.as_mut() else {
            self.health.dropped = self.health.dropped.saturating_add(1);
            return said.or_else(|| self.health.last_error.clone());
        };
        if let Err(e) = file.write_all(line) {
            // A FRAGMENT IS CLOSED, as the ring closes one: one malformed line,
            // never two lines fused. If this byte fails too, the next open
            // closes it.
            let _closed = file.write_all(b"\n");
            let why = format!(
                "{}: cannot append an audited line — {e}",
                self.health.path.display()
            );
            self.health.dropped = self.health.dropped.saturating_add(1);
            self.health.last_error = Some(why.clone());
            return Some(why);
        }
        let pointer = AuditPointer {
            serial: self.serial,
            offset: self.bytes,
            seq,
            len: u32::try_from(line.len()).unwrap_or(u32::MAX),
            level,
        };
        self.bytes = self.bytes.saturating_add(span);
        self.health.written = self.health.written.saturating_add(1);
        // THE GENERATION'S SPAN (R3-24): its first line's time recorded once,
        // its last kept here and recorded when it rolls.
        self.span_last = self.span_last.max(at);
        if self.span_first.is_none() {
            self.span_first = Some(at);
            self.span_last = at;
            self.write_span(false);
        }
        let image = pointer.image();
        for (dimension, value) in AUDIT_DIMENSIONS.into_iter().zip(postings_of(keys)) {
            let Some(value) = value else {
                continue;
            };
            let Some(path) = audit_posting_path(&self.dir, self.serial, dimension, &value) else {
                self.health.unindexed = self.health.unindexed.saturating_add(1);
                continue;
            };
            if let Err(why) = self.post(path, &image) {
                self.health.index_failures = self.health.index_failures.saturating_add(1);
                self.health.last_error = Some(why.clone());
                said = Some(why);
            } else {
                self.health.index_records = self.health.index_records.saturating_add(1);
            }
        }
        said
    }

    /// Appends one pointer image to one posting file.
    fn post(&mut self, path: PathBuf, image: &[u8; AUDIT_POINTER_BYTES]) -> Result<(), String> {
        use std::collections::hash_map::Entry;
        if self.handles.len() >= AUDIT_HANDLES && !self.handles.contains_key(&path) {
            self.handles.clear();
        }
        let file = match self.handles.entry(path) {
            Entry::Occupied(open) => open.into_mut(),
            Entry::Vacant(slot) => {
                let path = slot.key();
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("{}: cannot create — {e}", parent.display()))?;
                }
                let file = OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(path)
                    .map_err(|e| format!("{}: cannot open — {e}", path.display()))?;
                slot.insert(file)
            }
        };
        file.write_all(image)
            .map_err(|e| format!("an audit index posting: cannot append a pointer — {e}"))
    }

    /// Rolls the stream: the current file becomes `.1`, each `.n` becomes
    /// `.n + 1`, the oldest is deleted, and the serial advances. The serial is
    /// written AFTER the renames, so a crash between them leaves it one behind
    /// and [`audit_read`]'s second try finds the line one file older.
    ///
    /// The index generation of the file just dropped is RENAMED ASIDE for the
    /// reaper, never deleted here ([`Self::retire`], D-4838), and only after
    /// the next file is open: a rename that fails is this roll's failure (the
    /// stream stops rolling, as after any failed roll), but the stream goes on
    /// in the file it just opened, with pointers that name it.
    fn roll(&mut self) -> Result<(), String> {
        let named = |path: &Path, what: &str, e: &std::io::Error| {
            format!("{}: {what} — {e}", path.display())
        };
        let oldest = audit_rolled_path(&self.dir, AUDIT_KEEP_FILES.saturating_sub(1));
        remove_if_present(&oldest)
            .map_err(|e| named(&oldest, "cannot delete the oldest audit file", &e))?;
        for n in (1..AUDIT_KEEP_FILES.saturating_sub(1)).rev() {
            let from = audit_rolled_path(&self.dir, n);
            let to = audit_rolled_path(&self.dir, n.saturating_add(1));
            rename_if_present(&from, &to).map_err(|e| named(&from, "cannot roll", &e))?;
        }
        let current = audit_current_path(&self.dir);
        rename_if_present(&current, &audit_rolled_path(&self.dir, 1))
            .map_err(|e| named(&current, "cannot roll the audit file", &e))?;
        // THE CLOSED FILE'S SPAN, with its last line's time (R3-24).
        self.write_span(true);
        let next = self.serial.saturating_add(1);
        let path = audit_serial_path(&self.dir);
        let staged = path.with_extension("next");
        std::fs::create_dir_all(self.dir.join(AUDIT_INDEX_DIR))
            .and_then(|()| std::fs::write(&staged, next.to_le_bytes()))
            .and_then(|()| std::fs::rename(&staged, &path))
            .map_err(|e| named(&path, "cannot record the audit serial", &e))?;
        self.serial = next;
        // Every posting handle is of a generation this roll closed.
        self.handles.clear();
        self.file = Some(
            OpenOptions::new()
                .append(true)
                .create(true)
                .open(&current)
                .map_err(|e| named(&current, "cannot open the next audit file", &e))?,
        );
        self.bytes = 0;
        self.span_first = None;
        self.health.rotations = self.health.rotations.saturating_add(1);
        // A NEW GENERATION IS COVERED BY EVERY DIMENSION FROM ITS FIRST LINE.
        // A coverage file that cannot be written does not stop the roll: the
        // reader then treats the generation as the first build's and says the
        // newer dimensions do not cover it, which is the safe reading.
        let _named_in_last_error = self.mark_coverage();
        // THE GENERATION OF THE FILE JUST DELETED GOES WITH IT, so the index
        // never outlives the lines it points at: renamed aside here, deleted
        // by the reaper off this path (D-4838). Never a delete under the
        // sink's lock, which this runs under.
        next.checked_sub(u64::from(AUDIT_KEEP_FILES))
            .map_or(Ok(()), |gone| self.retire(gone))
    }

    /// The counters, with the running size and serial filled in, and what
    /// the reaper has counted on its side. The reaper's words are always in
    /// `reap_error`, and in `last_error` too when the writer has none of its
    /// own, so neither failure hides the other.
    fn health(&self) -> AuditHealth {
        let reap_error = self
            .reap
            .said
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        AuditHealth {
            current_bytes: self.bytes,
            serial: self.serial,
            gone_waiting: self.reap.waiting.load(Ordering::Relaxed),
            reaped: self.reap.reaped.load(Ordering::Relaxed),
            reap_failures: self.reap.failures.load(Ordering::Relaxed),
            last_error: self
                .health
                .last_error
                .clone()
                .or_else(|| reap_error.clone()),
            reap_error,
            ..self.health.clone()
        }
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
    use crate::event::{Event, MAX_MESSAGE_BYTES, MAX_STR_VALUE_BYTES, MAX_TARGET_BYTES};
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
        crate::tests::where_permission_binds(
            "sink::tests::a_failed_roll_reports_with_the_emit_lock_released",
            a_failed_roll_reports_with_the_emit_lock_released_body,
        );
    }

    /// The test above, run where the mode bits bind (D-0995).
    fn a_failed_roll_reports_with_the_emit_lock_released_body() {
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
        //
        // SLEEP FIRST, THEN LOOK, and no line that only some iterations reach,
        // in both waits below. Looking first let the writer's speed decide
        // whether a loop body ran at all, so gate 20's exact uncovered-line
        // count for this file moved with thread timing (D-0801). Each wait is
        // one `any` over a closure whose every line runs on every run, however
        // many tries the other thread takes.
        let failed = (0..5_000).any(|_| {
            std::thread::sleep(std::time::Duration::from_millis(1));
            sink.rotation_failures.load(Ordering::Relaxed) > 0
        });
        assert!(
            failed,
            "the premise: the roll had to fail for `report` to be reached"
        );

        // THE ASSERTION. Another thread must be able to take the emit mutex
        // while the writer is parked in `report`.
        let free = (0..5_000).any(|_| {
            std::thread::sleep(std::time::Duration::from_millis(1));
            sink.inner.try_lock().is_ok()
        });

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
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
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

    /// **Each override prefix is bounded, not only their count.** OD-4,
    /// D-2373, invariant AFG-73.
    ///
    /// `level_for` compares every prefix against every target that clears the
    /// fast floor. The count ceiling bounds how many comparisons; until D-2373
    /// nothing bounded how long each one was, so a megabyte prefix made every
    /// such event cost a megabyte-wide `starts_with`. A prefix exactly
    /// `MAX_TARGET_BYTES` long is allowed; one byte longer is refused by name,
    /// through the public field and through the builder alike.
    #[test]
    fn an_override_prefix_past_the_target_ceiling_is_refused_by_name() {
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("long-override-prefix");

        let at = Config::new(&dir).with_target_level("p".repeat(MAX_TARGET_BYTES), Level::Debug);
        assert!(at.refusal().is_none(), "the ceiling itself is allowed");
        let sink = Sink::open(&at).expect("a sink at the ceiling opens");
        assert_eq!(sink.level_for(&"p".repeat(MAX_TARGET_BYTES)), Level::Debug);
        drop(sink);

        let built =
            Config::new(&dir).with_target_level("p".repeat(MAX_TARGET_BYTES + 1), Level::Debug);
        let mut literal = Config::new(&dir);
        literal
            .target_levels
            .push(("q".repeat(1 << 20), Level::Trace));
        for (past, len) in [(built, MAX_TARGET_BYTES + 1), (literal, 1 << 20)] {
            let said = past
                .refusal()
                .expect("a prefix past the ceiling is refused");
            assert!(said.contains(&len.to_string()), "names the length: {said}");
            assert!(
                said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),
                "names the ceiling and the walk it bounds: {said}"
            );
            let refused = Sink::open(&past).expect_err("and `open` refuses it too");
            assert_eq!(refused, said, "the same reason reaches the caller");
        }
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
            torn: false,
            audit: None,
            state: None,
            flood: super::Flood::default(),
            counters: super::Counters::empty(1),
        };

        assert_eq!(
            inner.stamp(1_000),
            (1_000, false),
            "a fresh sink takes the clock"
        );
        assert_eq!(
            inner.stamp(1_001),
            (1_001, false),
            "forward is taken verbatim"
        );
        assert_eq!(
            inner.stamp(999),
            (1_001, true),
            "a clock stepped BACKWARDS by NTP is clamped to the last stamp, \
             because `tail` ends its `since` walk at the first older record"
        );
        assert_eq!(
            inner.stamp(1_001),
            (1_001, false),
            "equal is not an inversion"
        );
        assert_eq!(
            inner.stamp(1_002),
            (1_002, false),
            "and it moves on afterwards"
        );
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
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
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
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
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
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
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
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
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
            clock_held: 0,
            rationed: 9,
        };
        assert!(
            !quiet.is_loud(),
            "nothing wrong is not loud, and a ration working is nothing wrong"
        );

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
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
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
        crate::tests::where_permission_binds(
            "sink::tests::a_rotation_error_that_is_not_absence_is_reported_rather_than_swallowed",
            a_rotation_error_that_is_not_absence_is_reported_rather_than_swallowed_body,
        );
    }

    /// The test above, run where the mode bits bind (D-0995).
    fn a_rotation_error_that_is_not_absence_is_reported_rather_than_swallowed_body() {
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

    /// **`resume_point` READS FAR ENOUGH BACK FOR A LINE OF ANY LEGAL WIDTH.**
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
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
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

    /// audit-20261003 hunt-costs-2 (D-1536). A process that reserves two ids
    /// and writes an event only for the second must not have the second handed
    /// out again after a restart. The counter was seeded from the last `seq`
    /// alone, which here is 1, so the restart reserved 2 — an id already in
    /// the log.
    #[test]
    fn a_restart_never_reserves_a_run_id_an_event_in_the_log_carries() {
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("reserved-run-skip");
        let second = {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            let first = sink.reserve_run_id().expect("an id");
            let second = sink.reserve_run_id().expect("an id");
            assert!(second > first);
            assert!(
                sink.emit_for_run(second, &Event::info("t", "only the second ran"))
                    .is_written()
            );
            second
        };
        let reopened = Sink::open(&Config::new(&dir)).expect("reopens");
        let next = reopened.reserve_run_id().expect("an id");
        assert!(
            next > second,
            "a restart handed out {next}, which the log already carries as {second}"
        );
        drop(reopened);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// audit-20261003 hunt-costs-3 (D-1537). Two sinks on one directory are
    /// refused across processes, not only within one: the second `open` names
    /// the holder's lock instead of resuming the same `seq`, reserving the same
    /// run id and rotating the first one's file out from under it. The lock is
    /// a kernel `flock`, which conflicts between two open file descriptions in
    /// one process exactly as between two processes, so this proves the
    /// cross-process case without spawning one.
    #[test]
    fn a_second_sink_on_a_held_directory_is_refused_by_name() {
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("two-sinks");
        let held = Sink::open(&Config::new(&dir)).expect("the first sink opens");
        let why = Sink::open(&Config::new(&dir))
            .expect_err("a second sink on a held directory must be refused");
        assert!(
            why.contains("another") && why.contains(&dir.display().to_string()),
            "the refusal names the directory and why: {why}"
        );
        drop(held);
        let reopened = Sink::open(&Config::new(&dir)).expect("released with the first sink");
        assert!(
            reopened
                .emit(&Event::info("t", "after the holder"))
                .is_written()
        );
        drop(reopened);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// R3-14: **A CLOSED SINK FREES ITS DIRECTORY AND WRITES NOTHING MORE.**
    /// A server's successor starts the moment the old one's port is free, and
    /// the old one closes its sink first, so the successor's `open` must
    /// succeed while the closed sink is still alive in the ending process.
    /// Before `close` existed the lock lived until the process's last instant
    /// and the successor ran with no log. A line after the close is dropped
    /// and counted, never appended to the successor's file; a second close is
    /// a no-op.
    #[test]
    fn a_closed_sink_frees_its_directory_and_writes_nothing_more() {
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("closed-sink");
        let ending = Sink::open(&Config::new(&dir)).expect("the ending process's sink");
        assert!(
            ending
                .emit(&Event::info("t", "before the close"))
                .is_written()
        );
        assert!(
            Sink::open(&Config::new(&dir)).is_err(),
            "the premise: an open sink holds its directory"
        );
        assert!(format!("{ending:?}").contains("holds_directory: true"));
        assert!(
            ending.audit_health().is_some(),
            "the premise: it keeps an audit stream"
        );
        ending.close().expect("a clean close");
        assert!(format!("{ending:?}").contains("holds_directory: false"));
        assert_eq!(ending.audit_health(), None, "the audit stream is closed");
        let successor = Sink::open(&Config::new(&dir))
            .expect("the directory is free while the closed sink lives");
        assert_eq!(
            ending.emit(&Event::error("t", "a late line from the ending process")),
            Emitted::Dropped
        );
        let health = ending.health();
        assert_eq!(health.dropped, 1, "the late line is counted");
        let said = health.last_error.expect("and named");
        assert!(said.contains("after the sink was closed"), "{said}");
        assert!(
            said.contains("a late line from the ending process"),
            "{said}"
        );
        assert!(
            successor
                .emit(&Event::info("t", "the successor's line"))
                .is_written()
        );
        ending.close().expect("a second close is a no-op");
        let text = std::fs::read_to_string(current_path(&dir)).expect("the log");
        assert!(text.contains("before the close") && text.contains("the successor's line"));
        assert!(
            !text.contains("a late line"),
            "nothing landed after the close: {text}"
        );
        drop(successor);
        drop(ending);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// R3-14: a close whose sync fails still closes, and returns the failure.
    #[test]
    fn a_close_whose_sync_fails_still_closes_and_says_so() {
        let dir = scratch("closed-brittle");
        let brittle = Arc::new(Brittle::default());
        let sink =
            Sink::with_target(&Config::new(&dir), Box::new(Arc::clone(&brittle))).expect("opens");
        brittle.refuse();
        let said = sink.close().expect_err("the refused sync is returned");
        assert!(
            said.contains("closed") && said.contains("no space left"),
            "{said}"
        );
        assert_eq!(sink.emit(&Event::info("t", "after")), Emitted::Dropped);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// audit-20261003 hunt-costs-4 (D-1538). A floor resumed from the log that
    /// is AHEAD of the clock — a clock once set forward by mistake — is no
    /// longer carried silently: the open names it in `last_error`, and every
    /// event whose clock reading was held up to the floor is counted.
    #[test]
    fn a_resumed_floor_ahead_of_the_clock_is_named_and_counted() {
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("future-floor");
        {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            assert!(sink.emit(&Event::info("t", "before")).is_written());
        }
        // The last line, re-stamped in 2100 as a clock set forward would have.
        let path = current_path(&dir);
        let text = std::fs::read_to_string(&path).expect("the log");
        let now = crate::clock::now_millis();
        let future = 4_102_444_800_000_i64;
        assert!(future > now, "the premise: 2100 is ahead of this clock");
        let patched = text.replacen(
            &format!(
                "\"ms\":{}",
                Record::decode(text.trim_end().as_bytes())
                    .expect("a record")
                    .at_unix_millis
            ),
            &format!("\"ms\":{future}"),
            1,
        );
        assert_ne!(patched, text, "the premise: the stamp was rewritten");
        std::fs::write(&path, patched).expect("rewrite");

        let reopened = Sink::open(&Config::new(&dir)).expect("reopens");
        let named = reopened.health().last_error.unwrap_or_default();
        assert!(
            named.contains("ahead of the clock") && named.contains(&future.to_string()),
            "the open names the floor it resumed: {named}"
        );
        assert_eq!(reopened.health().clock_held, 0);
        assert!(
            reopened
                .emit(&Event::info("t", "after restart"))
                .is_written()
        );
        assert_eq!(
            reopened.health().clock_held,
            1,
            "the event stamped at the floor rather than the clock is counted"
        );
        drop(reopened);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// audit-20261003 attacksweep-2 (D-1539). On a disk that is STILL full,
    /// the newline that closes a torn fragment fails too, and the next event
    /// reported `Written` was fused onto the fragment and unreadable. The sink
    /// now remembers the unterminated fragment and closes it first.
    #[test]
    fn a_fragment_the_full_disk_would_not_terminate_is_closed_before_the_next_event() {
        let full = Arc::new(StillFull::default());
        let sink = Sink::with_target(
            &Config::new(scratch("still-full")),
            Box::new(Arc::clone(&full)),
        )
        .expect("opens");
        assert!(sink.emit(&Event::info("full", "one")).is_written());

        *full.cut_at.lock().expect("the lock") = Some(20);
        full.full.store(true, Ordering::Relaxed);
        assert_eq!(sink.emit(&Event::info("full", "torn")), Emitted::Dropped);
        assert_eq!(sink.emit(&Event::info("full", "three")), Emitted::Dropped);

        full.full.store(false, Ordering::Relaxed);
        assert!(sink.emit(&Event::info("full", "four")).is_written());

        let bytes = full.wrote.lock().expect("the lock").clone();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.lines()
                .any(|line| line.contains("\"four\"") && Record::decode(line.as_bytes()).is_ok()),
            "the event reported Written must be readable on its own line: {text}"
        );
        drop(sink);
        // The double fails nothing but the append: its sync and reopen succeed,
        // so every `Dropped` above is the full disk's and no other step's.
        let mut target = Arc::clone(&full);
        assert!(Target::sync(&target).is_ok());
        assert!(Target::reopen(&mut target, Path::new("unused")).is_ok());
    }

    /// A target that accepts `cut_at` bytes of one append and then refuses
    /// every append, with nothing written, while `full` is set — a disk that
    /// stays full, which is what ENOSPC is.
    #[derive(Debug, Default)]
    struct StillFull {
        wrote: std::sync::Mutex<Vec<u8>>,
        cut_at: std::sync::Mutex<Option<usize>>,
        full: AtomicBool,
    }

    impl Target for Arc<StillFull> {
        fn append(&mut self, bytes: &[u8]) -> std::io::Result<()> {
            let mut wrote = self
                .wrote
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if self.full.load(Ordering::Relaxed) {
                let cut = self.cut_at.lock().map_or(None, |mut c| c.take());
                wrote.extend_from_slice(bytes.get(..cut.unwrap_or(0)).unwrap_or(bytes));
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

    #[test]
    fn reserved_run_ids_resume_strictly_above_every_id_that_reached_the_log() {
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
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
        crate::tests::where_permission_binds(
            "sink::tests::a_roll_that_cannot_complete_does_not_empty_the_history_one_file_per_event",
            a_roll_that_cannot_complete_does_not_empty_the_history_one_file_per_event_body,
        );
    }

    /// The test above, run where the mode bits bind (D-0995).
    fn a_roll_that_cannot_complete_does_not_empty_the_history_one_file_per_event_body() {
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
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
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
        // One sink per directory (D-1537): the holder is released first.
        drop(again);
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
    /// `resume_point` reads the last COMPLETE line, so the restart knew the
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
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
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
        // One sink per directory (D-1537): the holder is released first.
        drop(again);
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
    /// as root the mode decides nothing, which is why the body runs through
    /// `where_permission_binds` (D-0995).
    #[test]
    fn a_tail_that_cannot_be_examined_is_left_exactly_as_it_was_found() {
        crate::tests::where_permission_binds(
            "sink::tests::a_tail_that_cannot_be_examined_is_left_exactly_as_it_was_found",
            a_tail_that_cannot_be_examined_is_left_exactly_as_it_was_found_body,
        );
    }

    /// The test above, run where the mode bits bind (D-0995).
    fn a_tail_that_cannot_be_examined_is_left_exactly_as_it_was_found_body() {
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
            "`resume_point` could not read it either, which is its own stated limit"
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

    /// **A RESTART WHOSE CLOCK IS BEHIND THE LAST LINE KEEPS `ms` IN FILE ORDER.**
    ///
    /// The line on disk is stamped a day ahead of this host — the state a
    /// reboot leaves when NTP steps the clock back after the previous process
    /// wrote. A resumed sink used to clamp against a floor of zero, so its
    /// first line was a day OLDER than the line before it; `tail`'s `since`
    /// filter, which ends the walk at the first record older than its floor,
    /// then met that line first and returned nothing — although the older
    /// line on disk matched. D-1325.
    #[test]
    fn a_restart_behind_the_last_stamp_keeps_ms_in_file_order() {
        let dir = scratch("resume-floor");
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let ahead = crate::clock::now_millis() + 86_400_000;
        let mut line = Vec::new();
        crate::encode::line(
            &mut line,
            5,
            ahead,
            0,
            &Event::info("t", "before the restart"),
        );
        std::fs::write(current_path(&dir), &line).expect("write");

        let sink = Sink::open(&Config::new(&dir)).expect("reopens");
        assert!(
            sink.emit(&Event::info("t", "after the restart"))
                .is_written()
        );
        let written = lines_of(&current_path(&dir));
        assert_eq!(written.len(), 2);
        assert_eq!(written[1].seq, 6, "the sequence resumes as before");
        assert_eq!(
            written[1].at_unix_millis, ahead,
            "the floor resumed from the line on disk, so the clock behind it is clamped"
        );

        let found = crate::tail(
            &dir,
            sink.keep_files(),
            &crate::Query::last(10).since(ahead - 3_600_000),
        );
        let messages: Vec<&str> = found.records.iter().map(|r| r.message.as_str()).collect();
        assert_eq!(
            messages,
            vec!["after the restart", "before the restart"],
            "a `since` an hour before the line on disk finds both"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **AN EMPTY CURRENT FILE DOES NOT RESTART THE NUMBERING UNDER A ROLLED ONE.**
    ///
    /// A roll renames the current file to `.1` and opens a fresh one; if the
    /// append that follows fails, or the process dies first, the set is left
    /// with an empty current file and all of its history in `.1`. The resume
    /// read the current file alone, found nothing, and started at zero — so
    /// the restarted process re-issued sequence numbers AND run ids that events
    /// in `.1` still carry. `reserve_run_id` says a restart starts "strictly
    /// above every id still present in the log"; this is the case where it did
    /// not. D-1326.
    #[test]
    fn an_empty_current_file_resumes_from_the_rolled_one_beneath_it() {
        // A sink is dropped and reopened here: see `FORK_GATE` (D-1462).
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("resume-rolled");
        let config = Config::new(&dir).with_keep_files(4);
        let (last_seq, run) = {
            let sink = Sink::open(&config).expect("opens");
            let run = sink.reserve_run_id().expect("an id");
            for i in 0..5u32 {
                assert!(
                    sink.emit_for_run(run, &Event::info("t", "m").with("i", i))
                        .is_written()
                );
            }
            (sink.health().next_seq - 1, run)
        };
        // THE ROLL, by hand, exactly as `roll` leaves it before its append.
        std::fs::rename(current_path(&dir), rotated_path(&dir, 1)).expect("roll");
        std::fs::write(current_path(&dir), b"").expect("an empty current file");

        let sink = Sink::open(&config).expect("reopens");
        assert_eq!(
            sink.health().next_seq,
            last_seq + 1,
            "the numbering carries on from the rolled file"
        );
        let next = sink.reserve_run_id().expect("an id");
        assert!(
            next > run,
            "a restart never re-issues a run id still on disk"
        );
        assert!(next > last_seq);

        // And an ABSENT current file is the same state: nothing to read.
        drop(sink);
        std::fs::remove_file(current_path(&dir)).expect("remove");
        let sink = Sink::open(&config).expect("reopens over an absent file");
        assert_eq!(sink.health().next_seq, last_seq + 1);

        // A set with nothing in it at all still starts at one.
        let empty = scratch("resume-rolled-empty");
        std::fs::create_dir_all(&empty).expect("a scratch directory");
        std::fs::write(rotated_path(&empty, 1), b"").expect("an empty rolled file");
        let fresh = Sink::open(&Config::new(&empty)).expect("opens");
        assert_eq!(fresh.health().next_seq, 1);
        assert_eq!(fresh.reserve_run_id(), Some(1));
        let _ignored = std::fs::remove_dir_all(&dir);
        let _ignored = std::fs::remove_dir_all(&empty);
    }

    // -----------------------------------------------------------------------
    // THE AUDIT STREAM (D-3341 item 3; DB-023, DB-G88)
    // -----------------------------------------------------------------------

    use super::{
        AUDIT_DIMENSIONS, AUDIT_KEEP_FILES, AUDIT_POINTER_BYTES, AuditLine, AuditPointer,
        audit_current_path, audit_path_for, audit_posting_path, audit_postings, audit_read,
        audit_rolled_path, audit_serial, audit_serial_path, is_audited, posting_name,
    };

    /// The one record a posting probe finds for `dimension = value`.
    fn probed(dir: &Path, dimension: &str, value: &str) -> Vec<Record> {
        let postings = audit_postings(dir, dimension, value, 100).expect("a probe");
        let serial = audit_serial(dir).expect("a serial");
        postings
            .pointers
            .iter()
            .map(|pointer| match audit_read(dir, serial, pointer) {
                AuditLine::Found(record) => record,
                other => panic!("{dimension}={value}: {other:?}"),
            })
            .collect()
    }

    /// **A REFUSAL LANDS IN THE AUDIT STREAM AND IS FOUND BY A PATH PROBE ON
    /// EVERY KEY IT CARRIES**: its day, its month, its feed, its instrument,
    /// its contract and its target. The line is the ring's own, byte for byte.
    /// An event that is not a refusal or a lifecycle step never reaches it.
    #[test]
    fn a_refusal_is_kept_in_the_audit_stream_and_found_by_each_of_its_keys() {
        let dir = scratch("audit-probe");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        let refusal = Event::error("gdfl.import", "refused")
            .with("feed", "gdfl")
            .with("instrument", "NIFTY")
            .with("day", "2022-03-07")
            .with("contract", "2022-03-10-1650000-CE")
            .with(
                "why",
                "line 4820: LTP is not a decimal of at most two places",
            );
        assert!(sink.emit(&refusal).is_written());
        // NOT AUDITED: a warn (the ring's flood was warn), and an info that is
        // no run's lifecycle.
        assert!(
            sink.emit(&Event::warn("api.bars", "read refused").with("day", "2022-03-07"))
                .is_written()
        );
        assert!(
            sink.emit(&Event::info("pull.run", "ingested").with("day", "2022-03-07"))
                .is_written()
        );

        for (dimension, value) in [
            ("day", "2022-03-07"),
            ("month", "2022-03"),
            ("feed", "gdfl"),
            ("instrument", "NIFTY"),
            ("contract", "2022-03-10-1650000-CE"),
            ("target", "gdfl.import"),
        ] {
            let found = probed(&dir, dimension, value);
            assert_eq!(found.len(), 1, "{dimension}={value}: {found:?}");
            assert!(found[0].matches(&refusal), "{dimension}={value}");
        }
        assert!(probed(&dir, "target", "api.bars").is_empty());
        assert!(probed(&dir, "instrument", "BANKNIFTY").is_empty());

        // THE SAME BYTES AS THE RING'S LINE.
        let ring = std::fs::read_to_string(current_path(&dir)).expect("ring");
        let audit = std::fs::read_to_string(audit_current_path(&dir)).expect("audit");
        assert_eq!(audit.lines().count(), 1);
        assert_eq!(ring.lines().next(), audit.lines().next());

        let health = sink.audit_health().expect("a file sink keeps a stream");
        assert_eq!(health.written, 1);
        assert_eq!(health.index_records, 6);
        assert_eq!((health.dropped, health.index_failures), (0, 0));
        assert!(!health.is_loud());
        assert_eq!(health.current_bytes, audit.len() as u64);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **WHAT IS AUDITED, BY RULE**: every `error`; a lifecycle word or a
    /// `lifecycle` target at `info` or above; never a `warn` refusal, never a
    /// `debug` lifecycle step, and a word inside another word is not one.
    #[test]
    fn the_audit_rule_takes_errors_and_lifecycles_and_nothing_else() {
        assert!(is_audited(Level::Error, "pull.spot", "member did not land"));
        assert!(is_audited(Level::Info, "gdfl.import", "run started"));
        assert!(is_audited(Level::Info, "gdfl.import", "run finished"));
        assert!(is_audited(
            Level::Warn,
            "pull.run",
            "finished — BOOKS DO NOT BALANCE"
        ));
        assert!(is_audited(Level::Info, "cli.lifecycle", "command refused"));
        assert!(is_audited(Level::Warn, "autopilot", "paused"));
        assert!(!is_audited(Level::Warn, "api.bars", "read refused"));
        assert!(!is_audited(Level::Info, "pull.run", "ingested"));
        assert!(!is_audited(Level::Debug, "pull.chunk", "chunk finished"));
        assert!(!is_audited(Level::Info, "x", "unfinished business"));
        assert!(!is_audited(Level::Info, "x.lifecycles", "noted"));
        assert!(AUDIT_DIMENSIONS.contains(&"day"));
    }

    /// **A POSTING NAME CANNOT LEAVE ITS DIMENSION**: every byte outside
    /// `A-Z a-z 0-9 - _` is escaped, so `..`, `/` and a space are text, and a
    /// value too long to name is refused rather than cut into another's name.
    #[test]
    fn a_posting_name_is_escaped_and_never_a_path() {
        assert_eq!(posting_name("NIFTY 50").as_deref(), Some("NIFTY%2050"));
        assert_eq!(posting_name("../x").as_deref(), Some("%2E%2E%2Fx"));
        assert_eq!(posting_name("M&M").as_deref(), Some("M%26M"));
        assert_eq!(posting_name(""), None);
        assert_eq!(posting_name(&"A".repeat(121)), None);
        assert_eq!(
            posting_name(&"/".repeat(41)),
            None,
            "escaped past the bound"
        );
        let dir = Path::new("/store/telemetry");
        assert_eq!(
            audit_posting_path(dir, 3, "day", "2022-03-07"),
            Some(PathBuf::from(
                "/store/telemetry/audit.idx/3/day/2022-03-07.idx"
            ))
        );
        assert_eq!(audit_posting_path(dir, 0, "nowhere", "x"), None);
    }

    /// **A POINTER ROUND-TRIPS, AND A RECORD THIS CRATE DID NOT WRITE IS
    /// REFUSED**: a wrong mark, a level no build names, a short record.
    #[test]
    fn a_pointer_round_trips_and_a_foreign_record_is_refused() {
        let pointer = AuditPointer {
            serial: 3,
            offset: 1 << 40,
            seq: 77,
            len: 512,
            level: Level::Error,
        };
        let image = pointer.image();
        assert_eq!(AuditPointer::decode(&image), Some(pointer));
        let mut marked = image;
        marked[31] = b'Z';
        assert_eq!(AuditPointer::decode(&marked), None);
        let mut ranked = image;
        ranked[28] = 9;
        assert_eq!(AuditPointer::decode(&ranked), None);
        assert_eq!(
            AuditPointer::decode(&image[..AUDIT_POINTER_BYTES - 1]),
            None
        );
    }

    /// **THE STREAM ROLLS AT ITS BOUND, THE SERIAL MOVES, AND A POINTER STILL
    /// FINDS ITS LINE IN THE ROLLED FILE**; a line older than the kept set is
    /// `RolledOff`, never another line; a serial one behind the files (a crash
    /// between the renames and the serial's write) is found one file on.
    #[test]
    fn a_rolled_line_is_found_through_its_serial_and_a_dropped_one_is_said() {
        let dir = scratch("audit-roll");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        sink.inner
            .lock()
            .unwrap()
            .audit
            .as_mut()
            .expect("a stream")
            .max_file_bytes = 2048;
        for i in 0..40u32 {
            assert!(
                sink.emit(
                    &Event::error("pull.spot", "member did not land")
                        .with("instrument", "VBL")
                        .with("day", "2019-01-02")
                        .with("i", i)
                )
                .is_written()
            );
        }
        let health = sink.audit_health().expect("a stream");
        assert!(health.rotations >= 3, "{health:?}");
        // THE INDEX IS BOUNDED WITH THE STREAM: one generation per kept file.
        let generations = std::fs::read_dir(dir.join(super::AUDIT_INDEX_DIR))
            .unwrap()
            .filter(|entry| entry.as_ref().unwrap().path().is_dir())
            .count();
        assert!(
            generations <= usize::from(AUDIT_KEEP_FILES),
            "{generations}"
        );
        assert_eq!(audit_serial(&dir).expect("serial"), health.serial);
        assert_eq!(health.rotation_failures, 0);
        let found = probed(&dir, "instrument", "VBL");
        assert_eq!(found.len(), 40, "every line still in a kept file");
        let mut seen: Vec<u64> = found
            .iter()
            .map(|r| {
                r.field("i")
                    .and_then(crate::value::OwnedValue::as_u64)
                    .unwrap()
            })
            .collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..40).collect::<Vec<u64>>());

        // A SERIAL ONE BEHIND: the line moved one file older than it says.
        let first = audit_postings(&dir, "instrument", "VBL", 1)
            .expect("probe")
            .pointers[0];
        let behind = health.serial.saturating_sub(1);
        assert!(matches!(
            audit_read(&dir, behind, &first),
            AuditLine::Found(_)
        ));
        // ROLLED OFF: a serial the kept set no longer reaches.
        let ancient = AuditPointer { serial: 0, ..first };
        assert_eq!(
            audit_read(&dir, u64::from(AUDIT_KEEP_FILES) + 5, &ancient),
            AuditLine::RolledOff
        );
        assert_eq!(audit_path_for(&dir, 5, 6), None, "a serial from the future");
        assert_eq!(audit_path_for(&dir, 5, 5), Some(audit_current_path(&dir)));
        assert_eq!(audit_path_for(&dir, 5, 3), Some(audit_rolled_path(&dir, 2)));
        // A POINTER WHOSE SEQ DOES NOT MATCH THE LINE IS NOT THAT LINE.
        let wrong = AuditPointer {
            seq: first.seq + 1_000_000,
            ..first
        };
        assert_eq!(
            audit_read(&dir, health.serial, &wrong),
            AuditLine::RolledOff
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A NEW STREAM BESIDE AN OLD RING STARTS WITH THE RING'S REFUSALS**,
    /// oldest first and once: a reopen does not copy them again.
    #[test]
    fn a_new_audit_stream_is_seeded_from_the_ring_once() {
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("audit-seed");
        {
            let ring_only = Sink::open(&Config::new(&dir).with_audit(false)).expect("opens");
            assert!(ring_only.audit_health().is_none());
            for day in ["2022-03-04", "2022-03-07"] {
                let _ = ring_only.emit(&Event::error("gdfl.import", "refused").with("day", day));
                let _ =
                    ring_only.emit(&Event::info("gdfl.import", "day imported").with("day", day));
            }
            let _ = ring_only.emit(&Event::info("gdfl.import", "run finished"));
        }
        assert!(!audit_current_path(&dir).exists());
        {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            let health = sink.audit_health().expect("a stream");
            assert_eq!(health.seeded, 3, "{health:?}");
            assert_eq!(probed(&dir, "day", "2022-03-07").len(), 1);
            assert_eq!(probed(&dir, "target", "gdfl.import").len(), 3);
        }
        let again = Sink::open(&Config::new(&dir)).expect("reopens");
        assert_eq!(again.audit_health().expect("a stream").seeded, 0);
        assert_eq!(probed(&dir, "target", "gdfl.import").len(), 3);
        assert!(!audit_serial_path(&dir).exists(), "no roll, no serial");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A SINK AROUND A TARGET IT DID NOT OPEN KEEPS NO STREAM**, and an
    /// absent posting is an empty answer rather than an error.
    #[test]
    fn a_target_sink_keeps_no_stream_and_an_absent_posting_is_empty() {
        let sink = Sink::with_target(&Config::new("/nowhere"), Box::new(NullTarget)).expect("ok");
        assert!(sink.audit_health().is_none());
        let dir = scratch("audit-empty");
        let none = audit_postings(&dir, "day", "2022-03-07", 10).expect("absent is empty");
        assert_eq!((none.total, none.pointers.len()), (0, 0));
        assert!(audit_postings(&dir, "nowhere", "x", 10).is_err());
        assert_eq!(audit_serial(&dir), Ok(0));
        std::fs::create_dir_all(audit_serial_path(&dir).parent().unwrap()).unwrap();
        std::fs::write(audit_serial_path(&dir), b"abc").unwrap();
        assert!(audit_serial(&dir).is_err_and(|why| why.contains("3 byte(s)")));
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **AN INDEX GENERATION GOES WITH ITS FILE**: past
    /// [`AUDIT_KEEP_FILES`] rolls the oldest file and its pointers go
    /// together (the generation renamed aside for the reaper, D-4838), so a
    /// probe finds exactly the lines still kept and the index a reader can
    /// reach never outgrows the stream.
    #[test]
    fn the_oldest_index_generation_goes_with_the_oldest_file() {
        let dir = scratch("audit-generations");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        sink.inner
            .lock()
            .unwrap()
            .audit
            .as_mut()
            .expect("a stream")
            .max_file_bytes = 1;
        let lines = u64::from(AUDIT_KEEP_FILES) + 6;
        for i in 0..lines {
            let _ = sink.emit(
                &Event::error("pull.spot", "refused")
                    .with("instrument", "VBL")
                    .with("i", i),
            );
        }
        let health = sink.audit_health().expect("a stream");
        assert_eq!(health.rotations, lines - 1);
        assert_eq!(health.rotation_failures, 0, "{health:?}");
        assert!(!super::audit_generation_path(&dir, 0).exists());
        assert!(super::audit_generation_path(&dir, health.serial).exists());
        let found = probed(&dir, "instrument", "VBL");
        assert_eq!(
            found.len(),
            usize::from(AUDIT_KEEP_FILES),
            "one line per kept file"
        );
        assert_eq!(
            found[0]
                .field("i")
                .and_then(crate::value::OwnedValue::as_u64),
            Some(lines - 1),
            "newest first"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    // -----------------------------------------------------------------------
    // IA-11r, R3 (D-4838): A ROLL RENAMES THE RETIRED GENERATION ASIDE AND THE
    // REAPER DELETES IT OFF THE EMIT PATH
    // -----------------------------------------------------------------------

    /// A posting tree of `files` files in index generation `serial` of `dir`:
    /// a synthetic generation, one posting per value of one dimension.
    fn posting_tree(dir: &Path, serial: u64, files: usize) {
        let dimension = super::audit_generation_path(dir, serial).join("contract");
        std::fs::create_dir_all(&dimension).expect("a dimension directory");
        for n in 0..files {
            std::fs::write(dimension.join(format!("ZZ{n:07}.idx")), [0u8; 0]).expect("a posting");
        }
    }

    /// Every file under `root`, counted by walking it; 0 when it is not there.
    fn files_under(root: &Path) -> usize {
        let Ok(entries) = std::fs::read_dir(root) else {
            return 0;
        };
        entries
            .map(|entry| entry.expect("an entry").path())
            .map(|path| if path.is_dir() { files_under(&path) } else { 1 })
            .sum()
    }

    /// What `audit.idx.gone/` of `dir` holds, by name, sorted.
    fn gone_entries(dir: &Path) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(dir.join(super::AUDIT_GONE_DIR)) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    /// The stream's reaper counters, shared with its thread.
    fn reap_of(sink: &Sink) -> Arc<super::Reap> {
        Arc::clone(
            &sink
                .inner
                .lock()
                .unwrap()
                .audit
                .as_ref()
                .expect("a stream")
                .reap,
        )
    }

    /// Waits until the reaper has nothing left that it can delete: nothing
    /// waiting beyond the deletes that failed. Panics after two minutes.
    fn reaped(sink: &Sink) -> super::AuditHealth {
        for _ in 0..120_000 {
            let health = sink.audit_health().expect("a stream");
            if health.gone_waiting <= health.reap_failures {
                return health;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("the reaper did not finish: {:?}", sink.audit_health());
    }

    /// A stream in `dir` fifteen rolls old with nothing written since, so the
    /// next roll is the first that ends an index generation (generation 0),
    /// and that generation holds `files` posting files.
    fn fifteen_rolls_old(dir: &Path, files: usize) {
        std::fs::create_dir_all(dir.join(super::AUDIT_INDEX_DIR)).expect("the index directory");
        std::fs::write(audit_serial_path(dir), 15u64.to_le_bytes()).expect("a serial");
        std::fs::write(audit_current_path(dir), b"").expect("a stream");
        posting_tree(dir, 0, files);
    }

    /// **IA-LG-01: AN AUDIT ROLL NEVER WAITS FOR A DELETE** (D-4838; F26).
    /// With the reaper held, so that nothing can be deleted at all, eight
    /// threads emit through a stream whose every line rolls, and the first
    /// roll ends a generation of 10 and then of 1,000 posting files: every
    /// emit returns, the generation is whole under `audit.idx.gone/` (the roll
    /// renamed it and deleted nothing), a reader finds it absent from
    /// `audit.idx/` and reads only kept lines, and the health counts what
    /// waits. Released, the reaper deletes it all. 100,000 files are timed by
    /// the bench (C-AU-01, C-AU-02), not built here.
    #[test]
    fn an_audit_roll_never_waits_for_a_delete() {
        for files in [10usize, 1_000] {
            let dir = scratch(&format!("audit-reap-{files}"));
            fifteen_rolls_old(&dir, files);
            let sink = Arc::new(Sink::open(&Config::new(&dir)).expect("opens"));
            assert_eq!(sink.audit_health().expect("a stream").serial, 15);
            sink.inner
                .lock()
                .unwrap()
                .audit
                .as_mut()
                .expect("a stream")
                .max_file_bytes = 1;
            let reap = reap_of(&sink);
            let held = reap.gate.lock().unwrap();
            // THE FIRST LINE lands in the empty current file; every line after
            // it rolls first.
            let per_thread = 6u64;
            std::thread::scope(|scope| {
                for thread in 0..8u64 {
                    let sink = Arc::clone(&sink);
                    scope.spawn(move || {
                        for i in 0..per_thread {
                            assert!(
                                sink.emit(
                                    &Event::error("pull.spot", "refused")
                                        .with("instrument", "VBL")
                                        .with("thread", thread)
                                        .with("i", i),
                                )
                                .is_written(),
                                "an emit returned unwritten while the reaper was held"
                            );
                        }
                    });
                }
            });
            let lines = 8 * per_thread;
            let health = sink.audit_health().expect("a stream");
            assert_eq!(health.written, lines);
            assert_eq!(
                health.rotations,
                lines - 1,
                "every line after the first rolled"
            );
            assert_eq!(health.rotation_failures, 0, "{health:?}");
            assert_eq!(health.serial, 15 + lines - 1);
            // THE ROLL RENAMED, AND DELETED NOTHING: the reaper is held, yet
            // every roll finished; the generation the first roll ended is whole
            // where the reaper will find it, and absent where a reader looks.
            assert!(!super::audit_generation_path(&dir, 0).exists());
            let aside = dir.join(super::AUDIT_GONE_DIR);
            assert_eq!(files_under(&aside.join("0")), files, "generation 0, whole");
            // Generations 1..=15 never existed (nothing was indexed in them);
            // 16.. are this run's, one line each, retired as they aged out.
            let retired = lines - 1 - 15;
            assert_eq!(health.gone_waiting, 1 + retired, "{health:?}");
            assert_eq!((health.reaped, health.reap_failures), (0, 0));
            assert!(
                health.reaper_behind(),
                "more wait than the stream keeps files"
            );
            assert!(health.is_loud(), "and that is said");
            assert_eq!(gone_entries(&dir).len() as u64, 1 + retired);
            // A READER SEES ONLY KEPT GENERATIONS, never a retired one, and no
            // error for the ones that are gone.
            let found = probed(&dir, "instrument", "VBL");
            assert_eq!(
                found.len(),
                usize::from(AUDIT_KEEP_FILES),
                "one line a kept file"
            );
            assert!(
                audit_postings(&dir, "contract", "ZZ0000000", 10)
                    .expect("a probe of a retired generation's value")
                    .pointers
                    .is_empty()
            );
            // RELEASED, the reaper deletes every retired generation, off the
            // emit path, and counts each.
            drop(held);
            let health = reaped(&sink);
            assert_eq!(health.gone_waiting, 0, "{health:?}");
            assert_eq!(health.reaped, 1 + retired);
            assert_eq!(health.reap_failures, 0);
            assert!(!health.reaper_behind() && !health.is_loud());
            assert!(gone_entries(&dir).is_empty(), "{:?}", gone_entries(&dir));
            assert!(aside.is_dir(), "the directory itself stays");
            let _ignored = std::fs::remove_dir_all(&dir);
        }
    }

    /// **A PROCESS KILLED IN THE MIDDLE OF A DELETE FINISHES IT AT ITS NEXT
    /// START** (IA-LG-01): what `audit.idx.gone/` holds when a sink opens, a
    /// half-deleted tree and a whole one, is handed to the reaper and deleted,
    /// and counted while it waits. So is the one generation a process killed
    /// between a roll's serial write and its rename left in `audit.idx/`.
    #[test]
    fn a_delete_cut_short_is_finished_at_the_next_start() {
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("audit-reap-restart");
        fifteen_rolls_old(&dir, 0);
        // The serial says 20: generations 5..=20 are kept, and generation 4
        // was ended by the roll that wrote 20, which never renamed it.
        std::fs::write(audit_serial_path(&dir), 20u64.to_le_bytes()).expect("a serial");
        posting_tree(&dir, 4, 7);
        posting_tree(&dir, 5, 3);
        let aside = dir.join(super::AUDIT_GONE_DIR);
        // A tree a killed reaper left half deleted, and one it never began.
        std::fs::create_dir_all(aside.join("2").join("day")).expect("a half-deleted tree");
        std::fs::write(aside.join("2").join("day").join("2022-03-07.idx"), b"x").expect("a file");
        std::fs::create_dir_all(aside.join("3").join("code")).expect("a whole tree");
        std::fs::write(aside.join("3").join("code").join("ZZ.idx"), b"x").expect("a file");
        std::fs::write(aside.join("3").join("dims"), b"day 0\n").expect("a file");
        {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            let health = reaped(&sink);
            assert_eq!(
                health.reaped, 3,
                "the two left aside and the stranded one: {health:?}"
            );
            assert_eq!((health.gone_waiting, health.reap_failures), (0, 0));
            assert_eq!(health.last_error, None);
            assert!(gone_entries(&dir).is_empty(), "{:?}", gone_entries(&dir));
            assert!(
                !super::audit_generation_path(&dir, 4).exists(),
                "the stranded generation"
            );
            assert_eq!(
                files_under(&super::audit_generation_path(&dir, 5)),
                3,
                "a kept generation is never touched"
            );
        }
        // NOTHING LEFT: the next start lists an empty directory and reaps
        // nothing.
        let again = Sink::open(&Config::new(&dir)).expect("reopens");
        let health = again.audit_health().expect("a stream");
        assert_eq!((health.gone_waiting, health.reaped), (0, 0), "{health:?}");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A DELETE THAT FAILS IS COUNTED, NAMED, LEFT FOR THE NEXT START AND
    /// NOT RETRIED** (IA-LG-01): a retired tree the reaper cannot delete stays
    /// in `audit.idx.gone/`, `reap_failures` is one and stays one, its words
    /// are in the health, and the stream goes on writing and rolling. The next
    /// start, with the obstacle gone, deletes it.
    #[test]
    fn a_failed_delete_is_counted_named_and_left_for_the_next_start() {
        use std::os::unix::fs::PermissionsExt as _;
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("audit-reap-fails");
        fifteen_rolls_old(&dir, 0);
        let stuck = dir.join(super::AUDIT_GONE_DIR).join("0").join("day");
        std::fs::create_dir_all(&stuck).expect("a tree");
        std::fs::write(stuck.join("2022-03-07.idx"), b"x").expect("a file");
        let mode = |mode: u32| {
            std::fs::set_permissions(&stuck, std::fs::Permissions::from_mode(mode)).expect("chmod");
        };
        // A directory nothing can be unlinked from.
        mode(0o555);
        {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            let health = reaped(&sink);
            assert_eq!(health.reap_failures, 1, "{health:?}");
            assert_eq!(
                (health.gone_waiting, health.reaped),
                (1, 0),
                "still on disk, still counted"
            );
            let said = health.reap_error.clone().expect("the reaper's words");
            assert!(
                said.contains("cannot delete the retired index generation"),
                "{said}"
            );
            assert!(said.contains("stays for the next start"), "{said}");
            assert_eq!(
                health.last_error, health.reap_error,
                "the writer has no error of its own"
            );
            assert!(health.is_loud());
            // THE STREAM IS UNHARMED, and the failed delete is not tried again.
            assert!(sink.emit(&Event::error("x", "refused")).is_written());
            std::thread::sleep(std::time::Duration::from_millis(20));
            let later = sink.audit_health().expect("a stream");
            assert_eq!((later.reap_failures, later.written), (1, 1), "{later:?}");
            assert_eq!(gone_entries(&dir), ["0"]);
        }
        mode(0o755);
        let sink = Sink::open(&Config::new(&dir)).expect("reopens");
        let health = reaped(&sink);
        assert_eq!(
            (health.reaped, health.reap_failures, health.gone_waiting),
            (1, 0, 0)
        );
        assert!(gone_entries(&dir).is_empty());
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **WITH NO REAPER THREAD NOTHING IS DELETED ON THE EMIT PATH INSTEAD**
    /// (IA-LG-01): a thread that cannot start is named in the health, the
    /// rolls still rename each ended generation aside, what waits is counted,
    /// and past [`AUDIT_KEEP_FILES`] waiting the health says the reaper is
    /// behind. The next start deletes them.
    #[test]
    fn without_a_reaper_the_retired_generations_wait_counted() {
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("audit-reap-none");
        let lines = 2 * u64::from(AUDIT_KEEP_FILES) + 3;
        {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            {
                let mut guard = sink.inner.lock().unwrap();
                let audit = guard.audit.as_mut().expect("a stream");
                audit.max_file_bytes = 1;
                let said = audit
                    .start_reaper_with(|_jobs, _reap| Err(std::io::Error::other("no threads left")))
                    .expect_err("a start that fails");
                assert!(
                    said.contains("cannot start the telemetry-reaper thread"),
                    "{said}"
                );
                assert!(said.contains("no threads left"), "{said}");
            }
            for i in 0..lines {
                assert!(
                    sink.emit(&Event::error("x", "refused").with("i", i))
                        .is_written(),
                    "line {i}"
                );
            }
            let health = sink.audit_health().expect("a stream");
            assert_eq!(health.rotation_failures, 0, "{health:?}");
            // Generation g is ended by the roll to serial g + 16: of the
            // `lines - 1` rolls, the last `lines - 1 - 15` each ended one.
            let retired = lines - 1 - u64::from(AUDIT_KEEP_FILES) + 1;
            assert_eq!(
                (health.gone_waiting, health.reaped),
                (retired, 0),
                "{health:?}"
            );
            assert!(
                health.reaper_behind(),
                "{retired} wait, the stream keeps 16 files"
            );
            assert!(health.is_loud());
            assert!(
                health
                    .last_error
                    .as_deref()
                    .is_some_and(|why| why.contains("cannot start the telemetry-reaper thread")),
                "{health:?}"
            );
            assert_eq!(gone_entries(&dir).len() as u64, retired);
            assert_eq!(
                probed(&dir, "target", "x").len(),
                usize::from(AUDIT_KEEP_FILES)
            );
        }
        let sink = Sink::open(&Config::new(&dir)).expect("reopens");
        let health = reaped(&sink);
        assert_eq!(health.gone_waiting, 0, "{health:?}");
        assert!(gone_entries(&dir).is_empty());
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A NAME ALREADY TAKEN ASIDE IS NOT WRITTEN OVER** (D-4838): an entry
    /// an earlier stream left under the same serial keeps its name and what
    /// it holds, and the generation is renamed beside it under a stamped
    /// name, whole.
    #[test]
    fn a_name_already_taken_aside_is_kept_and_the_generation_renamed_beside_it() {
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("audit-reap-taken");
        fifteen_rolls_old(&dir, 4);
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        let reap = reap_of(&sink);
        let held = reap.gate.lock().unwrap();
        let aside = dir.join(super::AUDIT_GONE_DIR);
        // AFTER THE START'S LISTING: the same name, not empty, appears aside.
        std::fs::create_dir_all(aside.join("0").join("earlier")).expect("a taken name");
        sink.inner
            .lock()
            .unwrap()
            .audit
            .as_mut()
            .expect("a stream")
            .max_file_bytes = 1;
        for i in 0..2u32 {
            assert!(
                sink.emit(&Event::error("x", "refused").with("i", i))
                    .is_written()
            );
        }
        let health = sink.audit_health().expect("a stream");
        assert_eq!(
            (health.rotations, health.rotation_failures),
            (1, 0),
            "{health:?}"
        );
        let names = gone_entries(&dir);
        assert_eq!(names.len(), 2, "{names:?}");
        assert_eq!(names[0], "0", "the earlier entry keeps its name");
        assert!(
            names[1].starts_with("0."),
            "the generation, under a stamped name: {names:?}"
        );
        assert_eq!(files_under(&aside.join(&names[1])), 4, "and whole");
        assert!(
            aside.join("0").join("earlier").is_dir(),
            "the earlier entry untouched"
        );
        assert_eq!(
            health.gone_waiting, 1,
            "only what this stream handed over is counted"
        );
        drop(held);
        let _done = reaped(&sink);
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A RENAME THAT CANNOT HAPPEN IS THE ROLL'S FAILURE, NAMED** (D-4838):
    /// with `audit.idx.gone` unusable (nothing can be renamed into it, or a
    /// file stands where it has to be) the roll fails by name, rotation stops
    /// as after any failed roll, nothing is deleted in the rename's place,
    /// and the stream goes on in the file the roll opened, every line found.
    #[test]
    fn a_rename_aside_that_cannot_happen_fails_the_roll_by_name() {
        use std::os::unix::fs::PermissionsExt as _;
        let _gate = crate::tests::no_fork_in_flight();
        // A DIRECTORY NOTHING CAN BE RENAMED INTO.
        let blocked = scratch("audit-reap-blocked");
        fifteen_rolls_old(&blocked, 2);
        let aside = blocked.join(super::AUDIT_GONE_DIR);
        std::fs::create_dir_all(&aside).expect("the directory");
        let sink = Sink::open(&Config::new(&blocked)).expect("opens");
        std::fs::set_permissions(&aside, std::fs::Permissions::from_mode(0o555)).expect("chmod");
        sink.inner
            .lock()
            .unwrap()
            .audit
            .as_mut()
            .expect("a stream")
            .max_file_bytes = 1;
        for i in 0..3u32 {
            assert!(
                sink.emit(&Event::error("x", "refused").with("i", i))
                    .is_written()
            );
        }
        std::fs::set_permissions(&aside, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let health = sink.audit_health().expect("a stream");
        assert_eq!(health.rotation_failures, 1, "{health:?}");
        assert_eq!(
            health.rotations, 1,
            "the files rolled once, and never again"
        );
        let said = health.last_error.clone().expect("the roll's failure");
        assert!(
            said.contains("cannot retire the index generation"),
            "{said}"
        );
        assert!(said.contains(super::AUDIT_GONE_DIR), "{said}");
        assert_eq!(health.dropped, 0, "no line was lost to it");
        assert_eq!(health.gone_waiting, 0);
        assert!(
            super::audit_generation_path(&blocked, 0).exists(),
            "not renamed, not deleted"
        );
        assert_eq!(
            probed(&blocked, "target", "x").len(),
            3,
            "every line is found"
        );
        drop(sink);

        // AND WHERE THE DIRECTORY CANNOT EXIST AT ALL: a file in its place.
        let filed = scratch("audit-reap-filed");
        fifteen_rolls_old(&filed, 1);
        std::fs::write(filed.join(super::AUDIT_GONE_DIR), b"in the way").expect("a file");
        let sink = Sink::open(&Config::new(&filed)).expect("opens anyway");
        let listed = sink.audit_health().expect("a stream").last_error;
        assert!(
            listed
                .as_deref()
                .is_some_and(|why| why.contains("cannot list")),
            "the start's listing is named: {listed:?}"
        );
        sink.inner
            .lock()
            .unwrap()
            .audit
            .as_mut()
            .expect("a stream")
            .max_file_bytes = 1;
        for i in 0..2u32 {
            assert!(
                sink.emit(&Event::error("x", "refused").with("i", i))
                    .is_written()
            );
        }
        let health = sink.audit_health().expect("a stream");
        assert_eq!(health.rotation_failures, 1, "{health:?}");
        assert!(
            health
                .last_error
                .as_deref()
                .is_some_and(|why| why.contains("cannot create")),
            "{health:?}"
        );
        let _ignored = std::fs::remove_dir_all(&blocked);
        let _ignored = std::fs::remove_dir_all(&filed);
    }

    /// **A FILE IN `audit.idx.gone/` IS NOT A RETIRED GENERATION** (D-4838).
    /// A `.DS_Store` a file browser left there was handed to the reaper, its
    /// delete failed, and the health was loud at every start for ever. It is
    /// left where it is, counted neither as waiting nor as a failed delete,
    /// and named apart; the real generation beside it is still deleted. At
    /// the second start too, when nothing but the stray is there.
    #[test]
    fn a_stray_file_in_the_graveyard_is_named_and_never_a_failed_delete() {
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("audit-reap-stray");
        fifteen_rolls_old(&dir, 0);
        let gone = dir.join(super::AUDIT_GONE_DIR);
        std::fs::create_dir_all(gone.join("0").join("day")).expect("a retired generation");
        std::fs::write(gone.join("0").join("day").join("2022-03-07.idx"), b"x").expect("a file");
        std::fs::write(gone.join(".DS_Store"), b"not this sink's").expect("a stray");
        for start in 0..2u64 {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            let health = reaped(&sink);
            assert_eq!(
                (health.reap_failures, health.gone_waiting, health.reaped),
                (0, 0, 1 - start),
                "start {start}: {health:?}"
            );
            assert_eq!(health.reap_error, None, "no delete failed");
            assert!(!health.is_loud(), "start {start}: {health:?}");
            let said = health.last_error.clone().expect("the stray is named");
            assert!(said.contains(".DS_Store"), "{said}");
            assert!(said.contains("is not a retired index generation"), "{said}");
            assert!(said.contains("1 such entry is"), "{said}");
            assert_eq!(gone_entries(&dir), [".DS_Store"], "left where it is");
            assert!(sink.emit(&Event::error("x", "refused")).is_written());
        }
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A LINK AT THE LOGS' GONE FOLDER IS REFUSED, AND NOTHING UNDER IT IS
    /// TOUCHED** (R3-16, RA-294), whenever the link appears. One planted
    /// before the start is the next test's. Here `audit.idx.gone` is SWAPPED
    /// after a generation was handed to the reaper and before it deletes:
    /// the real folder renamed away and, in its place, a link to another
    /// directory or another directory, each holding an entry of the
    /// generation's name. The delete resolved the name again, so it emptied
    /// whatever was there now; it is refused by name instead, the other
    /// directory and the real generation keep every file, and once the real
    /// folder is back the next start deletes the generation.
    #[test]
    fn a_link_at_the_logs_gone_folder_is_refused_and_nothing_under_it_is_touched() {
        let _gate = crate::tests::no_fork_in_flight();
        for swap in ["link", "folder"] {
            let dir = scratch(&format!("audit-reap-swap-{swap}"));
            let elsewhere = scratch(&format!("audit-reap-swap-{swap}-elsewhere"));
            fifteen_rolls_old(&dir, 2);
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            sink.inner
                .lock()
                .unwrap()
                .audit
                .as_mut()
                .expect("a stream")
                .max_file_bytes = 1;
            let reap = reap_of(&sink);
            let held = reap.gate.lock().unwrap();
            for i in 0..4u32 {
                if !gone_entries(&dir).is_empty() {
                    break;
                }
                assert!(
                    sink.emit(&Event::error("x", "refused").with("i", i))
                        .is_written()
                );
            }
            let names = gone_entries(&dir);
            assert_eq!(names.len(), 1, "one generation handed over: {names:?}");
            let name = names[0].clone();

            // THE SWAP, while the reaper waits.
            let gone = dir.join(super::AUDIT_GONE_DIR);
            let moved_away = dir.join("gone-moved-away");
            std::fs::rename(&gone, &moved_away).expect("renamed away");
            let theirs = if swap == "link" {
                std::os::unix::fs::symlink(&elsewhere, &gone).expect("a link");
                elsewhere.join(&name)
            } else {
                gone.join(&name)
            };
            std::fs::create_dir_all(&theirs).expect("a directory");
            std::fs::write(theirs.join("someone-elses"), b"not this sink's").expect("a file");
            drop(held);

            let health = reaped(&sink);
            assert_eq!(
                (health.reap_failures, health.reaped),
                (1, 0),
                "{swap}: {health:?}"
            );
            let said = health.reap_error.clone().expect("the refusal");
            assert!(said.contains("is no longer the folder"), "{said}");
            assert!(said.contains(super::AUDIT_GONE_DIR), "{said}");
            assert!(said.contains("nothing in it is deleted"), "{said}");
            assert_eq!(
                std::fs::read(theirs.join("someone-elses")).expect("still there"),
                b"not this sink's",
                "{swap}: nothing under the new name was deleted"
            );
            assert!(
                files_under(&moved_away.join(&name)) > 0,
                "{swap}: the real generation is whole, left for the next start"
            );
            drop(sink);

            // THE REAL FOLDER BACK: the next start deletes the generation.
            if swap == "link" {
                std::fs::remove_file(&gone).expect("the test's link");
            } else {
                std::fs::remove_dir_all(&gone).expect("the test's folder");
            }
            std::fs::rename(&moved_away, &gone).expect("put back");
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            let health = reaped(&sink);
            assert_eq!(
                (health.reap_failures, health.reaped),
                (0, 1),
                "{swap}: {health:?}"
            );
            assert!(gone_entries(&dir).is_empty(), "{swap}");
            if swap == "link" {
                assert_eq!(
                    std::fs::read(theirs.join("someone-elses")).expect("still there"),
                    b"not this sink's"
                );
            }
            drop(sink);
            let _ignored = std::fs::remove_dir_all(&dir);
            let _ignored = std::fs::remove_dir_all(&elsewhere);
        }
    }

    /// **A LINK NAMED `audit.idx.gone` IS REFUSED, AND NOTHING BEHIND IT IS
    /// DELETED** (D-4838). The start's listing followed the link, so the
    /// reaper deleted every entry of the directory it pointed at; and a roll
    /// renamed its generation through it, out of the sink's directory. Both
    /// are refused by name: the other directory is untouched, the generation
    /// stays in `audit.idx/`, and no line is lost.
    #[test]
    fn a_linked_graveyard_is_refused_by_name_and_nothing_behind_it_is_deleted() {
        let _gate = crate::tests::no_fork_in_flight();
        let dir = scratch("audit-reap-linked");
        let elsewhere = scratch("audit-reap-linked-elsewhere");
        fifteen_rolls_old(&dir, 2);
        let kept = elsewhere.join("someone-elses").join("file");
        std::fs::create_dir_all(kept.parent().expect("a parent")).expect("a directory");
        std::fs::write(&kept, b"not this sink's").expect("a file");
        std::os::unix::fs::symlink(&elsewhere, dir.join(super::AUDIT_GONE_DIR)).expect("a link");

        let sink = Sink::open(&Config::new(&dir)).expect("opens anyway");
        let health = sink.audit_health().expect("a stream");
        let said = health.last_error.clone().expect("the listing is refused");
        assert!(said.contains("cannot list"), "{said}");
        assert!(said.contains("a symbolic link or a file"), "{said}");
        assert!(said.contains(super::AUDIT_GONE_DIR), "{said}");
        assert_eq!(health.gone_waiting, 0, "nothing behind the link is counted");

        // A ROLL DOES NOT RENAME THROUGH IT EITHER.
        sink.inner
            .lock()
            .unwrap()
            .audit
            .as_mut()
            .expect("a stream")
            .max_file_bytes = 1;
        for i in 0..3u32 {
            assert!(
                sink.emit(&Event::error("x", "refused").with("i", i))
                    .is_written()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        let health = sink.audit_health().expect("a stream");
        assert_eq!(health.rotation_failures, 1, "{health:?}");
        let said = health.last_error.clone().expect("the roll's failure");
        assert!(said.contains("cannot create"), "{said}");
        assert!(said.contains("a symbolic link or a file"), "{said}");
        assert_eq!(
            (health.gone_waiting, health.reaped, health.dropped),
            (0, 0, 0)
        );
        assert!(
            super::audit_generation_path(&dir, 0).exists(),
            "not renamed out of the sink's directory"
        );
        assert_eq!(
            std::fs::read(&kept).expect("still there"),
            b"not this sink's",
            "nothing behind the link was deleted"
        );
        assert_eq!(
            std::fs::read_dir(&elsewhere).expect("lists").count(),
            1,
            "and nothing was renamed into it"
        );
        assert_eq!(probed(&dir, "target", "x").len(), 3, "every line is found");
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
        let _ignored = std::fs::remove_dir_all(&elsewhere);
    }

    /// **A STREAM NOBODY MAY LOOK FOR IS NOT "NEVER WRITTEN"** (D-4862):
    /// `audit_state` answered `Ok(None)` for a directory without search
    /// permission, so a reader said "nothing audited" where it could not
    /// look. It is an error, in the failure's own words; a directory that
    /// can be searched and holds no stream is still `Ok(None)`.
    #[test]
    fn an_audit_stream_that_cannot_be_looked_for_is_an_error() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = scratch("audit-state-unsearchable");
        std::fs::create_dir_all(&dir).expect("a directory");
        assert_eq!(
            super::audit_state(&dir),
            Ok(None),
            "searchable, never written"
        );
        assert_eq!(super::audit_state(&dir.join("absent")), Ok(None));
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o000)).expect("chmod");
        let answer = super::audit_state(&dir);
        let bound = std::fs::metadata(audit_current_path(&dir)).is_err();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        assert!(
            bound,
            "mode 000 does not bind this process: run as a user it binds"
        );
        let said = answer.expect_err("it could not be looked for");
        assert!(said.contains("cannot be looked for"), "{said}");
        assert!(said.contains("Permission denied"), "{said}");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    // -----------------------------------------------------------------------
    // WP-33c: RUN AND CODE IN THE INDEX, COVERAGE, ABSENCE, PLACES, ACCESS
    // RECORDS AND THE FLOOD RATION
    // -----------------------------------------------------------------------

    /// **A RUN AND A REFUSAL REASON ARE EACH ONE PROBE** (DB-A41, DB-A47):
    /// the top-level run of a stamped line, the numeric `run` field the GDFL
    /// importer writes on `run started`, and a refusal's `code` (else its
    /// `reason`) each land in their own posting; a line with none of them is
    /// in neither, and a run of zero is no run.
    #[test]
    fn a_run_and_a_refusal_code_are_each_one_probe() {
        let dir = scratch("audit-run-code");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        assert!(
            sink.emit_for_run(4025, &Event::info("cli.lifecycle", "command started"))
                .is_written()
        );
        assert!(
            sink.emit(&Event::info("gdfl.import", "run started").with("run", 1_791_000_000_u64))
                .is_written()
        );
        assert!(
            sink.emit(
                &Event::error("gdfl.import", "refused")
                    .with("day", "2020-01-01")
                    .with("code", "census_full")
            )
            .is_written()
        );
        assert!(
            sink.emit(&Event::error("pull.spot", "refused").with("reason", "ZZ_REASON"))
                .is_written()
        );
        assert!(
            sink.emit(&Event::error("x", "refused").with("run", 0_u64))
                .is_written()
        );
        assert_eq!(probed(&dir, "run", "4025").len(), 1);
        let importer = probed(&dir, "run", "1791000000");
        assert_eq!(importer.len(), 1);
        assert_eq!(importer[0].message, "run started");
        let census = probed(&dir, "code", "census_full");
        assert_eq!(census.len(), 1);
        assert_eq!(census[0].target, "gdfl.import");
        assert_eq!(probed(&dir, "code", "ZZ_REASON").len(), 1);
        assert!(probed(&dir, "run", "0").is_empty(), "zero is no run");
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A GENERATION SAYS FROM WHERE EACH DIMENSION COVERS IT** (WP-33c): a
    /// new stream is covered by every dimension from 0; a generation the first
    /// index build wrote (lines, no `dims` file) is covered by its six from 0
    /// and by `run` and `code` only from the offset the newer build opened it
    /// at; a posting probe names that generation as uncovered, so a miss there
    /// is never read as "nothing carried the value".
    #[test]
    fn a_generation_names_the_offset_each_dimension_covers_it_from() {
        let dir = scratch("audit-coverage");
        {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            let _ = sink.emit(&Event::error("x", "refused").with("code", "A"));
        }
        for dimension in super::AUDIT_DIMENSIONS {
            assert_eq!(
                super::audit_coverage(&dir, 0, dimension),
                Some(0),
                "{dimension}"
            );
        }
        assert!(
            super::audit_postings_before(&dir, "code", "A", 10, None)
                .expect("a probe")
                .uncovered
                .is_empty()
        );

        // THE FIRST BUILD'S GENERATION: lines and postings, no `dims` file.
        let old = scratch("audit-coverage-old");
        {
            let sink = Sink::open(&Config::new(&old)).expect("opens");
            let _ = sink.emit(&Event::error("x", "refused").with("code", "B"));
        }
        std::fs::remove_file(super::audit_generation_path(&old, 0).join(super::AUDIT_DIMS_FILE))
            .expect("the newer build wrote one");
        assert_eq!(super::audit_coverage(&old, 0, "day"), Some(0));
        assert_eq!(super::audit_coverage(&old, 0, "run"), None);
        let before = std::fs::metadata(audit_current_path(&old))
            .expect("a stream")
            .len();
        {
            let sink = Sink::open(&Config::new(&old)).expect("reopens");
            let _ = sink.emit(&Event::error("x", "refused").with("code", "B"));
        }
        assert_eq!(super::audit_coverage(&old, 0, "day"), Some(0));
        assert_eq!(super::audit_coverage(&old, 0, "code"), Some(before));
        let page = super::audit_postings_before(&old, "code", "B", 10, None).expect("a probe");
        assert_eq!(
            page.pointers.len(),
            2,
            "the old build's line was posted by this one too"
        );
        assert_eq!(page.uncovered, vec![(0, Some(before))]);
        assert!(
            super::audit_postings_before(&old, "day", "2022-01-01", 10, None)
                .expect("a probe")
                .uncovered
                .is_empty(),
            "the first six cover every generation whole"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
        let _ignored = std::fs::remove_dir_all(&old);
    }

    /// **A STREAM THAT WAS NEVER WRITTEN IS NOT A STREAM HOLDING NOTHING**
    /// (DB-A46): `audit_state` answers `None` for a directory with no audit
    /// file and no serial, and the serial for one that has a stream.
    #[test]
    fn an_absent_stream_is_named_apart_from_an_empty_one() {
        let dir = scratch("audit-state");
        std::fs::create_dir_all(&dir).expect("mkdir");
        assert_eq!(super::audit_state(&dir), Ok(None));
        assert_eq!(
            audit_serial(&dir),
            Ok(0),
            "the serial alone cannot tell them apart"
        );
        {
            let _sink = Sink::open(&Config::new(&dir)).expect("opens");
        }
        assert_eq!(super::audit_state(&dir), Ok(Some(0)));
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A PLACE IN A POSTING IS A STABLE CURSOR** (DB-A44): reading on from
    /// the last place a page returned gives the next older pointers, never one
    /// twice and never one skipped, across a roll, and lines appended after the
    /// first page do not move what the next page holds.
    #[test]
    fn a_posting_is_paged_from_a_place_without_a_line_twice_or_skipped() {
        let dir = scratch("audit-places");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        sink.inner
            .lock()
            .unwrap()
            .audit
            .as_mut()
            .expect("a stream")
            .max_file_bytes = 600;
        for i in 0..12u32 {
            let _ = sink.emit(
                &Event::error("x", "refused")
                    .with("instrument", "VBL")
                    .with("i", i),
            );
        }
        assert!(sink.audit_health().expect("a stream").rotations >= 2);
        let first = super::audit_postings_before(&dir, "instrument", "VBL", 5, None).expect("p1");
        assert_eq!(first.pointers.len(), 5);
        assert_eq!(first.total, 12);
        // A LINE APPENDED NOW IS NEWER THAN EVERY PLACE ALREADY HANDED OUT.
        let _ = sink.emit(
            &Event::error("x", "refused")
                .with("instrument", "VBL")
                .with("i", 99),
        );
        let mut seen: Vec<u64> = Vec::new();
        let serial = audit_serial(&dir).expect("serial");
        let mut page = first;
        loop {
            for (_, pointer) in &page.pointers {
                let AuditLine::Found(record) = audit_read(&dir, serial, pointer) else {
                    panic!("{pointer:?}");
                };
                seen.push(
                    record
                        .field("i")
                        .and_then(crate::value::OwnedValue::as_u64)
                        .unwrap(),
                );
            }
            let Some(&(last, _)) = page.pointers.last() else {
                break;
            };
            page = super::audit_postings_before(&dir, "instrument", "VBL", 5, Some(last))
                .expect("the next page");
        }
        assert_eq!(
            seen,
            (0..12).rev().collect::<Vec<u64>>(),
            "newest first, each once"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **AN ACCESS RECORD IS AUDITED AT INFO, AND NOTHING ELSE NEW IS**
    /// (DB-A42): a target whose last segment is `access` is the api's rationed
    /// `/db` read record; `accesses` and a `warn` are not.
    #[test]
    fn an_access_record_is_audited_at_info() {
        assert!(is_audited(Level::Info, "api.access", "read served"));
        assert!(is_audited(Level::Info, "access", "read served"));
        assert!(!is_audited(Level::Debug, "api.access", "read served"));
        assert!(!is_audited(Level::Info, "api.accesses", "read served"));
        assert!(!is_audited(Level::Info, "api.access.detail", "read served"));
    }

    /// **A WARN FLOOD IS HELD TO ITS RATION, COUNTED, AND SAID** (DB-A43):
    /// one pair past its lines in a window writes nothing more in it; the held
    /// lines are counted in `rationed`; the pair's next line after the window
    /// is led by ONE line naming the message and the count. Another pair, an
    /// error and an audited warn are never held.
    #[test]
    fn a_warn_flood_is_held_to_its_ration_counted_and_said() {
        let dir = scratch("flood");
        let sink = Sink::open(&Config::new(&dir).with_flood_lines(3)).expect("opens");
        for _ in 0..10 {
            let _ = sink.emit(&Event::warn("api.bars", "read refused").with("month", "1999-02"));
        }
        assert!(
            sink.emit(&Event::warn("api.other", "read refused"))
                .is_written()
        );
        for _ in 0..5 {
            assert!(
                sink.emit(&Event::error("api.bars", "read refused"))
                    .is_written()
            );
        }
        for _ in 0..5 {
            assert!(sink.emit(&Event::warn("autopilot", "paused")).is_written());
        }
        let health = sink.health();
        assert_eq!(health.rationed, 7, "{health:?}");
        let held: Vec<Record> = lines_of(&current_path(&dir))
            .into_iter()
            .filter(|r| r.target == "api.bars" && r.level == Level::Warn)
            .collect();
        assert_eq!(held.len(), 3, "the ration, and no more");
        // THE NEXT WINDOW SAYS WHAT THE LAST ONE HELD, then writes the line.
        {
            let mut guard = sink.inner.lock().unwrap();
            guard.last_at = guard.last_at.saturating_add(super::FLOOD_WINDOW_MS);
        }
        assert!(
            sink.emit(&Event::warn("api.bars", "read refused").with("month", "1999-03"))
                .is_written()
        );
        let lines = lines_of(&current_path(&dir));
        let last_two = &lines[lines.len() - 2..];
        assert_eq!(last_two[0].message, super::FLOOD_MESSAGE);
        assert_eq!(
            last_two[0]
                .field("held")
                .and_then(crate::value::OwnedValue::as_u64),
            Some(7)
        );
        assert_eq!(
            last_two[0]
                .field("message")
                .and_then(crate::value::OwnedValue::as_str),
            Some("read refused")
        );
        assert_eq!(
            last_two[1]
                .field("month")
                .and_then(crate::value::OwnedValue::as_str),
            Some("1999-03")
        );
        assert_eq!(last_two[1].seq, last_two[0].seq + 1);
        assert_eq!(sink.health().written, lines.len() as u64);
        // OFF IS OFF.
        let open = scratch("flood-off");
        let unrationed = Sink::open(&Config::new(&open).with_flood_lines(0)).expect("opens");
        for _ in 0..10 {
            assert!(
                unrationed
                    .emit(&Event::warn("api.bars", "read refused"))
                    .is_written()
            );
        }
        assert_eq!(unrationed.health().rationed, 0);
        let _ignored = std::fs::remove_dir_all(&dir);
        let _ignored = std::fs::remove_dir_all(&open);
    }

    /// **THE RATION'S TABLE IS BOUNDED, AND A PAIR PAST IT IS WRITTEN, NOT
    /// HELD**: `FLOOD_PAIRS` pairs in one window fill it; the next new pair is
    /// written unrationed rather than held.
    #[test]
    fn a_pair_past_the_rations_table_is_written_not_held() {
        let mut flood = super::Flood::default();
        let names: Vec<String> = (0..super::FLOOD_PAIRS).map(|n| format!("t{n}")).collect();
        for name in &names {
            assert_eq!(
                flood.admit(1, name, "m", 0),
                super::Rationed::Write { held_before: 0 }
            );
            assert_eq!(flood.admit(1, name, "m", 0), super::Rationed::Hold);
        }
        for _ in 0..3 {
            assert_eq!(
                flood.admit(1, "past", "m", 0),
                super::Rationed::Write { held_before: 0 },
                "unfollowed, never held"
            );
        }
        // A LATER WINDOW REPORTS WHAT EACH FOLLOWED PAIR HELD.
        assert_eq!(
            flood.admit(1, "t0", "m", super::FLOOD_WINDOW_MS),
            super::Rationed::Write { held_before: 1 }
        );
    }

    /// **A RUN IS READ FROM EVERY FORM A LINE CARRIES IT IN**, written or
    /// seeded from the ring: a top-level run, a numeric field (unsigned or
    /// signed), a field of canonical digits; never a negative, a padded or a
    /// zero one.
    #[test]
    fn a_run_is_read_from_every_form_a_line_carries_it_in() {
        let dir = scratch("audit-run-forms");
        {
            let ring_only = Sink::open(&Config::new(&dir).with_audit(false)).expect("opens");
            let _ = ring_only.emit_for_run(9, &Event::error("x", "seeded top-level"));
            let _ = ring_only.emit(&Event::error("x", "seeded field").with("run", 11_u64));
            let _ = ring_only.emit(&Event::error("x", "seeded text").with("run", "13"));
        }
        let sink = Sink::open(&Config::new(&dir)).expect("opens with a stream");
        assert_eq!(sink.audit_health().expect("a stream").seeded, 3);
        let _ = sink.emit(&Event::error("x", "signed").with("run", 5_i64));
        let _ = sink.emit(&Event::error("x", "text").with("run", "7"));
        let _ = sink.emit(&Event::error("x", "negative").with("run", -3_i64));
        let _ = sink.emit(&Event::error("x", "padded").with("run", "007"));
        let _ = sink.emit(&Event::error("x", "a flag").with("run", true));
        for (run, message) in [
            ("9", "seeded top-level"),
            ("11", "seeded field"),
            ("13", "seeded text"),
            ("5", "signed"),
            ("7", "text"),
        ] {
            let found = probed(&dir, "run", run);
            assert_eq!(found.len(), 1, "run {run}");
            assert_eq!(found[0].message, message);
        }
        assert!(probed(&dir, "run", "-3").is_empty());
        assert!(probed(&dir, "run", "007").is_empty());
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **A COVERAGE FILE IS READ LINE BY LINE AND A DIMENSION IT LACKS IS
    /// ADDED FROM WHERE THE STREAM STANDS**: a line this build cannot read is
    /// skipped, the first naming of a dimension wins, a generation whose file
    /// is not a file covers nothing it can vouch for, and a coverage file that
    /// cannot be read or written is named in the stream's health.
    #[test]
    fn a_coverage_file_is_read_line_by_line_and_extended_where_it_stands() {
        assert_eq!(
            super::dims_of("day 0\nnonsense\nrun x\nrun 5\nrun 6\ncode 007\n"),
            vec![("day".to_owned(), 0), ("run".to_owned(), 5)]
        );
        // A FILE NAMING SOME DIMENSIONS: the rest are added from the offset
        // the stream stands at when the next build opens it.
        let dir = scratch("audit-dims-partial");
        {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            let _ = sink.emit(&Event::error("x", "refused"));
        }
        let dims = super::audit_generation_path(&dir, 0).join(super::AUDIT_DIMS_FILE);
        std::fs::write(&dims, "day 0\n").expect("a partial coverage file");
        let at = std::fs::metadata(audit_current_path(&dir))
            .expect("a stream")
            .len();
        {
            let _sink = Sink::open(&Config::new(&dir)).expect("reopens");
        }
        assert_eq!(super::audit_coverage(&dir, 0, "day"), Some(0));
        assert_eq!(super::audit_coverage(&dir, 0, "target"), Some(at));
        assert_eq!(super::audit_coverage(&dir, 0, "run"), Some(at));
        // A COVERAGE FILE THAT IS NOT A FILE vouches for nothing, and the
        // writer names it.
        std::fs::remove_file(&dims).expect("unlink");
        std::fs::create_dir_all(&dims).expect("a directory in its place");
        assert_eq!(super::audit_coverage(&dir, 0, "day"), None);
        {
            let sink = Sink::open(&Config::new(&dir)).expect("opens anyway");
            let health = sink.audit_health().expect("a stream");
            assert!(
                health
                    .last_error
                    .as_deref()
                    .is_some_and(|why| why.contains("cannot read the index coverage")),
                "{health:?}"
            );
        }
        // A GENERATION THAT CANNOT BE WRITTEN: the coverage is not recorded,
        // and the stream says so.
        let blocked = scratch("audit-dims-blocked");
        let generation = super::audit_generation_path(&blocked, 0);
        std::fs::create_dir_all(&generation).expect("mkdir");
        let read_only = |mode: u32| {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&generation, std::fs::Permissions::from_mode(mode))
                .expect("chmod");
        };
        read_only(0o555);
        let said = Sink::open(&Config::new(&blocked))
            .expect("opens anyway")
            .audit_health()
            .expect("a stream")
            .last_error;
        read_only(0o755);
        assert!(
            said.as_deref()
                .is_some_and(|why| why.contains("cannot record the index coverage")),
            "{said:?}"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
        let _ignored = std::fs::remove_dir_all(&blocked);
    }

    /// **A POINTER THAT IS NOT ONE IS COUNTED AND ITS PLACE PASSED OVER**: a
    /// posting with a foreign record in it pages past it, and says how many.
    #[test]
    fn a_pointer_that_is_not_one_is_counted_and_passed_over() {
        use std::io::Write as _;
        let dir = scratch("audit-malformed-pointer");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        let _ = sink.emit(&Event::error("x", "first").with("instrument", "VBL"));
        let path = audit_posting_path(&dir, 0, "instrument", "VBL").expect("a posting");
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("the posting")
            .write_all(&[7u8; AUDIT_POINTER_BYTES])
            .expect("a foreign record");
        let _ = sink.emit(&Event::error("x", "second").with("instrument", "VBL"));
        let page =
            super::audit_postings_before(&dir, "instrument", "VBL", 10, None).expect("a probe");
        assert_eq!(page.total, 3);
        assert_eq!(page.malformed, 1);
        assert_eq!(page.pointers.len(), 2);
        assert_eq!(page.pointers[0].0.index, 2, "the second line's place");
        assert_eq!(
            page.pointers[1].0.index, 0,
            "the foreign record's place passed over"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
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
mod state_tests {
    //! The never-rolling state stream, the generations' time spans and how
    //! far back a directory reaches (R3-24; DB-G88, DB-023).

    use super::{
        AUDIT_KEEP_FILES, AUDIT_MAX_FILE_BYTES, AUDIT_SPAN_FILE, AuditHealth, AuditLine, AuditSpan,
        Config, DEFAULT_KEEP_FILES, STATE_LOUD_BYTES, Sink, audit_current_path,
        audit_generation_path, audit_postings, audit_read, audit_serial, audit_span, reach_of,
    };
    use crate::event::Event;
    use crate::record::Record;
    use crate::run_dirs::state_stream_of;
    use std::path::{Path, PathBuf};

    fn scratch(name: &str) -> PathBuf {
        crate::tests::sweep_stale_scratch();
        let dir = std::env::temp_dir().join(format!(
            "brutex-telemetry-{}-state-{name}",
            std::process::id()
        ));
        let _ignored = std::fs::remove_dir_all(&dir);
        dir
    }

    /// Every line `dimension = value` holds in the stream at `dir`, oldest
    /// first, read through its pointers.
    fn every_line(dir: &Path, dimension: &str, value: &str) -> Vec<Record> {
        let postings = audit_postings(dir, dimension, value, usize::MAX).expect("a probe");
        let serial = audit_serial(dir).expect("a serial");
        let mut out: Vec<Record> = postings
            .pointers
            .iter()
            .filter_map(|pointer| match audit_read(dir, serial, pointer) {
                AuditLine::Found(record) => Some(record),
                AuditLine::RolledOff => None,
                AuditLine::Unreadable(why) => panic!("{dimension}={value}: {why}"),
            })
            .collect();
        out.reverse();
        out
    }

    /// Writes at least `refusals` bytes of refusals through a sink whose audit
    /// stream rolls at `audit_file_bytes`, with a state change first, one
    /// every 997 refusals and one last; answers the sink, its directory and
    /// the state changes' `seq`s.
    fn through_a_flood(
        name: &str,
        refusals: u64,
        audit_file_bytes: u64,
    ) -> (Sink, PathBuf, Vec<u64>) {
        let dir = scratch(name);
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        sink.inner
            .lock()
            .unwrap()
            .audit
            .as_mut()
            .expect("an audit stream")
            .max_file_bytes = audit_file_bytes;
        let state = |sink: &Sink, n: u64, wanted: &mut Vec<u64>| {
            assert!(
                sink.emit(&Event::warn("autopilot", "paused").with("epoch", n))
                    .is_written()
            );
            wanted.push(sink.health().next_seq - 1);
        };
        let mut wanted = Vec::new();
        state(&sink, 0, &mut wanted);
        let why = "a refusal's reason, long enough to be a real one ".repeat(8);
        let mut written = 0u64;
        let mut n = 0u64;
        while written < refusals {
            let before = sink.audit_health().expect("a stream");
            let _ = sink.emit(
                &Event::error("pull.spot", "member did not land")
                    .with("instrument", "NIFTY")
                    .with("day", "2026-10-01")
                    .with("why", why.as_str()),
            );
            let after = sink.audit_health().expect("a stream");
            written += if after.rotations > before.rotations {
                after.current_bytes
            } else {
                after.current_bytes - before.current_bytes
            };
            n += 1;
            if n.is_multiple_of(997) {
                state(&sink, n, &mut wanted);
            }
        }
        state(&sink, n, &mut wanted);
        (sink, dir, wanted)
    }

    /// The proof at a given size: the refusals outgrow the rolling audit
    /// stream twice over, the rolling stream loses the first state change,
    /// and the state stream still finds every one by key, never having
    /// rolled.
    fn survives(name: &str, refusals: u64, audit_file_bytes: u64) {
        let retention = u64::from(AUDIT_KEEP_FILES) * audit_file_bytes;
        assert!(
            refusals >= 2 * retention,
            "premise: twice what the stream keeps"
        );
        let (sink, dir, wanted) = through_a_flood(name, refusals, audit_file_bytes);
        let rolling: Vec<u64> = every_line(&dir, "target", "autopilot")
            .iter()
            .map(|record| record.seq)
            .collect();
        assert!(
            !rolling.contains(&wanted[0]),
            "premise: the rolling stream rolled the first state change away"
        );
        let state_dir = state_stream_of(&dir);
        let kept: Vec<u64> = every_line(&state_dir, "target", "autopilot")
            .iter()
            .map(|record| record.seq)
            .collect();
        assert_eq!(kept, wanted, "every state change, in order, once");
        let by_day = every_line(&state_dir, "day", "2026-10-01");
        assert!(by_day.is_empty(), "no refusal is a state change");
        let health = sink.state_health().expect("a state stream");
        assert!(health.never_rolls);
        assert_eq!((health.serial, health.rotations), (0, 0));
        assert_eq!(health.written, wanted.len() as u64);
        assert!(!health.is_loud(), "{health:?}");
        // THE ANSWER SAYS HOW FAR BACK IT REACHES: the state stream to the
        // first state change, further than the rolling stream.
        let reach = reach_of(&dir, DEFAULT_KEEP_FILES);
        let first = every_line(&state_dir, "target", "autopilot")[0].at_unix_millis;
        assert_eq!(reach.state_from, Some(first));
        assert!(reach.audit_from.expect("an audit stream") >= first);
        assert_eq!(reach.audited_from(), Some(first));
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **AN AUDIT LINE SURVIVES 2 GiB OF REFUSALS** (R3-24, DB-G88): every
    /// who-did-what line is still found by key after the refusals have rolled
    /// the audit stream past its whole retention twice.
    ///
    /// Scaled by 1,024 on both sides so it runs in a test: the audit stream
    /// rolls at 64 KiB instead of 64 MiB (16 files, 1 MiB kept instead of
    /// 1 GiB) and 2 MiB of refusals go through it instead of 2 GiB, the same
    /// ratio. `an_audit_line_survives_2_gib_of_refusals_at_full_size` is the
    /// same proof at the real bounds. Before R3-24 there was no state stream
    /// and the first state change was gone with its audit file.
    #[test]
    fn an_audit_line_survives_2_gib_of_refusals() {
        const SCALE: u64 = 1024;
        survives(
            "survives",
            2 * 1024 * 1024 * 1024 / SCALE,
            AUDIT_MAX_FILE_BYTES / SCALE,
        );
    }

    /// The same proof at the real bounds: 2 GiB of refusals through a 64 MiB
    /// x 16 audit stream. Ignored in the ordinary run because it writes 2 GiB;
    /// run with `cargo test -p telemetry --release --
    /// --ignored an_audit_line_survives_2_gib_of_refusals_at_full_size`.
    #[test]
    #[ignore = "writes 2 GiB: run by hand at the real bounds (R3-24 report)"]
    fn an_audit_line_survives_2_gib_of_refusals_at_full_size() {
        survives(
            "survives-full",
            2 * 1024 * 1024 * 1024,
            AUDIT_MAX_FILE_BYTES,
        );
    }

    /// The lines of a stream's current file, decoded.
    fn lines_of(dir: &Path) -> Vec<Record> {
        std::fs::read(audit_current_path(dir))
            .expect("a stream")
            .split(|&byte| byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| Record::decode(line).expect("a line"))
            .collect()
    }

    /// **A NEW STATE STREAM IS SEEDED FROM WHAT THE DIRECTORY HOLDS**: the
    /// lifecycles the audit stream kept and the state changes only the ring
    /// held (an earlier build audited no "listening"), merged by `seq`, a line
    /// both hold copied once, and never again on a later open.
    #[test]
    fn a_new_state_stream_is_seeded_from_the_audit_stream_and_the_ring() {
        let dir = scratch("seed");
        {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            let _ = sink.emit(&Event::info("cli.lifecycle", "command started"));
            let _ = sink.emit(&Event::error("pull.spot", "member did not land"));
            let _ = sink.emit(&Event::info("x", "quiet"));
        }
        {
            let ring_only = Sink::open(&Config::new(&dir).with_audit(false)).expect("opens");
            let _ = ring_only.emit(&Event::info("api.serve", "listening"));
        }
        {
            let sink = Sink::open(&Config::new(&dir)).expect("opens");
            let _ = sink.emit(&Event::info("cli.lifecycle", "command finished"));
        }
        // AS IF THE DIRECTORY WERE AN EARLIER BUILD'S: no state stream yet.
        std::fs::remove_dir_all(state_stream_of(&dir)).expect("a state stream");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        assert_eq!(sink.state_health().expect("a state stream").seeded, 3);
        let seeded: Vec<(String, String)> = lines_of(&state_stream_of(&dir))
            .into_iter()
            .map(|record| (record.target, record.message))
            .collect();
        let named = |target: &str, message: &str| (target.to_owned(), message.to_owned());
        assert_eq!(
            seeded,
            [
                named("cli.lifecycle", "command started"),
                named("api.serve", "listening"),
                named("cli.lifecycle", "command finished"),
            ]
        );
        let seqs: Vec<u64> = lines_of(&state_stream_of(&dir))
            .iter()
            .map(|record| record.seq)
            .collect();
        assert!(seqs.windows(2).all(|pair| pair[0] < pair[1]), "{seqs:?}");
        drop(sink);
        let again = Sink::open(&Config::new(&dir)).expect("reopens");
        assert_eq!(again.state_health().expect("a state stream").seeded, 0);
        assert_eq!(
            lines_of(&state_stream_of(&dir)).len(),
            3,
            "never copied twice"
        );
        drop(again);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **EACH AUDIT FILE KEEPS ITS FIRST AND LAST EVENT TIME** (R3-24): the
    /// file being appended to is open-ended, a rolled one closed at its last
    /// line; a generation an earlier build wrote, with no span file, reads its
    /// own first and last lines; a reopened stream records the span again.
    #[test]
    fn a_generation_keeps_its_first_and_last_event_time() {
        let dir = scratch("span");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        sink.inner
            .lock()
            .unwrap()
            .audit
            .as_mut()
            .expect("an audit stream")
            .max_file_bytes = 1;
        let _ = sink.emit(&Event::error("x", "one").with("instrument", "AAA"));
        let _ = sink.emit(&Event::error("x", "two").with("instrument", "AAA"));
        let _ = sink.emit(&Event::error("x", "three").with("instrument", "BBB"));
        let at = |instrument: &str| -> Vec<i64> {
            every_line(&dir, "instrument", instrument)
                .iter()
                .map(|record| record.at_unix_millis)
                .collect()
        };
        let (aaa, bbb) = (at("AAA"), at("BBB"));
        assert_eq!(audit_serial(&dir).expect("a serial"), 2);
        let first = audit_span(&dir, 2, 0).expect("the first file");
        assert_eq!(
            first,
            AuditSpan {
                first_ms: aaa[0],
                last_ms: Some(aaa[0])
            }
        );
        assert_eq!(
            audit_span(&dir, 2, 2),
            Some(AuditSpan {
                first_ms: bbb[0],
                last_ms: None
            }),
            "the file being appended to is open-ended"
        );
        assert!(first.overlaps(aaa[0], aaa[0]));
        assert!(!first.overlaps(aaa[0] + 1, i64::MAX));
        assert!(!first.overlaps(i64::MIN, aaa[0] - 1));
        let open = audit_span(&dir, 2, 2).expect("open");
        assert!(open.overlaps(i64::MAX - 1, i64::MAX));
        assert_eq!(
            std::fs::read(audit_generation_path(&dir, 0).join(AUDIT_SPAN_FILE))
                .expect("a span file")
                .len(),
            16
        );
        // A GENERATION AN EARLIER BUILD WROTE reads its own lines.
        std::fs::remove_file(audit_generation_path(&dir, 1).join(AUDIT_SPAN_FILE))
            .expect("a span file");
        assert_eq!(
            audit_span(&dir, 2, 1),
            Some(AuditSpan {
                first_ms: aaa[1],
                last_ms: Some(aaa[1])
            })
        );
        // A FILE ROLLED OUT OF THE KEPT SET has no span to read.
        assert_eq!(audit_span(&dir, u64::from(AUDIT_KEEP_FILES) + 5, 0), None);
        // A REOPENED STREAM records the current file's span again.
        drop(sink);
        std::fs::remove_file(audit_generation_path(&dir, 2).join(AUDIT_SPAN_FILE))
            .expect("a span file");
        let again = Sink::open(&Config::new(&dir)).expect("reopens");
        assert!(
            audit_generation_path(&dir, 2)
                .join(AUDIT_SPAN_FILE)
                .exists(),
            "resumed and recorded"
        );
        assert_eq!(
            audit_span(&dir, 2, 2).map(|span| span.first_ms),
            Some(bbb[0])
        );
        let reach = reach_of(&dir, DEFAULT_KEEP_FILES);
        assert_eq!(reach.audit_from, Some(aaa[0]));
        assert!(reach.ring_from.is_some_and(|ring| ring <= aaa[0]));
        assert_eq!(reach.state_from, None, "no refusal is a state change");
        assert_eq!(reach.audited_from(), Some(aaa[0]));
        drop(again);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// A directory with nothing in it reaches nowhere, and says so with
    /// `None` rather than a time.
    #[test]
    fn an_empty_directory_reaches_nowhere() {
        let dir = scratch("reach-empty");
        std::fs::create_dir_all(&dir).expect("a directory");
        assert_eq!(reach_of(&dir, DEFAULT_KEEP_FILES), super::Reach::default());
        assert_eq!(super::Reach::default().audited_from(), None);
        let only_state = super::Reach {
            state_from: Some(5),
            ..super::Reach::default()
        };
        assert_eq!(only_state.audited_from(), Some(5));
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// The state stream never rolls, so its size is what is watched: loud
    /// past [`STATE_LOUD_BYTES`], and the rolling stream never by size.
    #[test]
    fn the_state_stream_is_loud_only_past_its_size() {
        let at = |never_rolls: bool, current_bytes: u64| AuditHealth {
            current_bytes,
            never_rolls,
            ..AuditHealth::default()
        };
        assert!(!at(true, STATE_LOUD_BYTES).is_loud());
        assert!(at(true, STATE_LOUD_BYTES + 1).is_loud());
        assert!(at(true, STATE_LOUD_BYTES + 1).grown_past_loud());
        assert!(!at(false, u64::MAX).is_loud());
    }

    /// **A STATE STREAM THAT CANNOT BE MADE IS NAMED**, and the ring and the
    /// audit stream still write: a file where its directory goes.
    #[test]
    fn a_state_stream_that_cannot_be_made_is_named() {
        let dir = scratch("state-blocked");
        std::fs::create_dir_all(&dir).expect("a directory");
        std::fs::write(state_stream_of(&dir), b"in the way").expect("a file in its place");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        let health = sink.state_health().expect("named, not absent");
        assert!(
            health
                .last_error
                .as_deref()
                .is_some_and(|why| why.contains("cannot create the state stream")),
            "{health:?}"
        );
        assert!(
            sink.emit(&Event::warn("autopilot", "paused")).is_written(),
            "the ring still writes"
        );
        let after = sink.state_health().expect("a state stream");
        assert_eq!(after.dropped, 1, "the state line is counted as lost");
        assert!(after.is_loud());
        assert_eq!(sink.audit_health().expect("an audit stream").written, 1);
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
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
mod picture_tests {
    //! The `/logs` picture's counters and the live copy, kept as lines are
    //! written (R3-24).

    use super::{
        Config, Counters, MIN_FILE_BYTES, PICTURE_CODES, PICTURE_MINUTES, RECENT_LINES, Sink,
        current_path,
    };
    use crate::event::Event;
    use crate::level::Level;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        crate::tests::sweep_stale_scratch();
        let dir = std::env::temp_dir().join(format!(
            "brutex-telemetry-{}-picture-{name}",
            std::process::id()
        ));
        let _ignored = std::fs::remove_dir_all(&dir);
        dir
    }

    /// The counts of the newest minute that holds any, by level rank.
    fn newest_counts(sink: &Sink) -> [u64; 5] {
        sink.picture()
            .minutes
            .iter()
            .rev()
            .find(|(_, counts)| counts.iter().sum::<u64>() > 0)
            .map(|(_, counts)| *counts)
            .unwrap_or_default()
    }

    /// **EVERY LINE WRITTEN IS COUNTED BY MINUTE AND LEVEL, AND ITS CODE**,
    /// as it is written; a filtered event is not; the ring's oldest line is
    /// the first one written.
    #[test]
    fn the_picture_is_counted_as_lines_are_written() {
        let dir = scratch("counted");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        let _ = sink.emit(&Event::info("a", "one"));
        let _ = sink.emit(&Event::info("a", "two"));
        let _ = sink.emit(&Event::warn("a", "three").with("code", "late"));
        let _ = sink.emit(&Event::error("a", "four").with("code", "census_full"));
        let _ = sink.emit(&Event::error("a", "five").with("code", "census_full"));
        let _ = sink.emit(&Event::error("a", "six").with("reason", "bad_row"));
        let _ = sink.emit(&Event::debug("a", "filtered by the floor"));
        let picture = sink.picture();
        assert_eq!(picture.counted, 6);
        assert_eq!(picture.minutes.len(), PICTURE_MINUTES);
        let total: u64 = picture
            .minutes
            .iter()
            .map(|(_, c)| c.iter().sum::<u64>())
            .sum();
        assert_eq!(total, 6, "{picture:?}");
        // Within a test the six may straddle a minute; summed per level they
        // are exact.
        let by_level = picture.minutes.iter().fold([0u64; 5], |mut sum, (_, c)| {
            for (s, n) in sum.iter_mut().zip(c) {
                *s += n;
            }
            sum
        });
        assert_eq!(by_level, [0, 0, 2, 1, 3]);
        assert_eq!(
            picture.codes,
            [
                ("census_full".to_owned(), 2),
                ("bad_row".to_owned(), 1),
                ("late".to_owned(), 1)
            ]
        );
        assert_eq!(picture.codes_replaced, 0);
        let first = crate::tail(&dir, sink.keep_files(), &crate::tail::Query::last(10))
            .records
            .last()
            .expect("a line")
            .at_unix_millis;
        assert_eq!(picture.oldest_ms, Some(first));
        assert!(picture.since_ms <= first);
        assert!(newest_counts(&sink).iter().sum::<u64>() > 0);
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **THE PICTURE NEVER READS A FILE**: the ring's files deleted under
    /// the sink leave every count where it was. And a reopened sink starts
    /// its oldest line from the ring it finds, read once.
    #[test]
    fn the_picture_survives_its_files_and_a_reopen_reads_the_ring_once() {
        let dir = scratch("no-files");
        let sink = Sink::open(&Config::new(&dir)).expect("opens");
        let _ = sink.emit(&Event::error("a", "kept").with("code", "x"));
        let before = sink.picture();
        std::fs::remove_file(current_path(&dir)).expect("the ring file");
        assert_eq!(sink.picture(), before, "counted, never read back");
        let _ = sink.emit(&Event::error("a", "after").with("code", "x"));
        assert_eq!(sink.picture().counted, 2);
        assert_eq!(sink.picture().codes, [("x".to_owned(), 2)]);
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
        // A REOPEN over a ring that is there reads its oldest line once.
        let kept = scratch("reopen");
        let first = {
            let sink = Sink::open(&Config::new(&kept)).expect("opens");
            let _ = sink.emit(&Event::info("a", "written by the last process"));
            sink.picture().oldest_ms
        };
        let again = Sink::open(&Config::new(&kept)).expect("reopens");
        let reopened = again.picture();
        assert_eq!(reopened.counted, 0, "counts are this process's");
        assert_eq!(reopened.oldest_ms, first, "the ring's oldest, read at open");
        drop(again);
        let _ignored = std::fs::remove_dir_all(&kept);
    }

    /// **A ROLL MOVES THE OLDEST LINE ON**: with two files kept, the first
    /// line leaves when its file does, and the picture says so.
    #[test]
    fn a_roll_moves_the_oldest_line_on() {
        let dir = scratch("rolls");
        let sink = Sink::open(
            &Config::new(&dir)
                .with_max_file_bytes(MIN_FILE_BYTES)
                .with_keep_files(2),
        )
        .expect("opens");
        let _ = sink.emit(&Event::info("a", "first"));
        let first = sink.picture().oldest_ms.expect("a line");
        std::thread::sleep(std::time::Duration::from_millis(3));
        for n in 0..60u32 {
            let _ = sink.emit(&Event::info("a", "filler").with("n", n));
        }
        assert!(
            sink.health().rotations >= 2,
            "premise: the first file is gone"
        );
        let oldest = sink.picture().oldest_ms.expect("a line");
        assert!(oldest > first, "{oldest} after {first}");
        let on_disk = crate::tail(&dir, 2, &crate::tail::Query::last(1000))
            .records
            .last()
            .expect("a line")
            .at_unix_millis;
        assert!(oldest <= on_disk, "never newer than the ring's oldest line");
        drop(sink);
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    /// **THE CODE TABLE IS FIXED IN SIZE**: past [`PICTURE_CODES`] codes a
    /// new one takes the least-counted one's place with its count and one,
    /// and the replacement is counted.
    #[test]
    fn the_code_table_is_fixed_in_size_and_says_when_it_replaced() {
        let mut counters = Counters::empty(1);
        for n in 0..PICTURE_CODES {
            for _ in 0..=n {
                counters.count(60_000, Level::Error, Some(&format!("code{n:02}")), b"x");
            }
        }
        counters.count(60_000, Level::Error, Some("newcomer"), b"x");
        let picture = counters.picture(1);
        assert_eq!(picture.codes.len(), PICTURE_CODES);
        assert_eq!(picture.codes_replaced, 1);
        assert!(picture.codes.contains(&("newcomer".to_owned(), 2)));
        assert!(!picture.codes.iter().any(|(code, _)| code == "code00"));
        assert_eq!(
            picture.codes[0],
            (format!("code{:02}", PICTURE_CODES - 1), 16)
        );
        // A MINUTE OUT OF THE WINDOW counts nothing, and a slot reused for a
        // later minute starts from zero.
        assert!(
            counters
                .picture(1 + i64::try_from(PICTURE_MINUTES).unwrap())
                .minutes
                .iter()
                .all(|(_, c)| c.iter().sum::<u64>() == 0)
        );
        counters.count(
            (1 + i64::try_from(PICTURE_MINUTES).unwrap()) * 60_000,
            Level::Info,
            None,
            b"y",
        );
        let later = counters.picture(1 + i64::try_from(PICTURE_MINUTES).unwrap());
        assert_eq!(later.minutes.last().unwrap().1, [0, 0, 1, 0, 0]);
        assert_eq!(
            later.minutes[0].1, [0; 5],
            "the minute an hour back is not this one"
        );
    }

    /// **THE LIVE COPY IS THE NEWEST LINES, NEWEST FIRST, AT MOST
    /// [`RECENT_LINES`]**, read from memory: the ring's file deleted under
    /// the sink, the copy still answers.
    #[test]
    fn the_live_copy_is_the_newest_lines_newest_first() {
        let dir = scratch("recent");
        let sink = Sink::open(&Config::new(&dir).with_min_level(Level::Trace)).expect("opens");
        let total = u32::try_from(RECENT_LINES).unwrap() + 40;
        for n in 0..total {
            let _ = sink.emit(&Event::trace("a", "line").with("n", n));
        }
        std::fs::remove_file(current_path(&dir)).expect("the ring file");
        let (records, malformed) = sink.recent();
        assert_eq!(malformed, 0);
        assert_eq!(records.len(), RECENT_LINES);
        let n = |at: usize| records[at].field("n").and_then(crate::OwnedValue::as_u64);
        assert_eq!(n(0), Some(u64::from(total - 1)), "newest first");
        assert_eq!(
            n(RECENT_LINES - 1),
            Some(u64::from(total) - RECENT_LINES as u64)
        );
        // A FLOOD'S OWN LINE is copied and counted with the line it leads.
        let flood = scratch("recent-flood");
        let rationed = Sink::open(&Config::new(&flood).with_flood_lines(1)).expect("opens");
        let _ = rationed.emit(&Event::warn("f", "same"));
        let _ = rationed.emit(&Event::warn("f", "same"));
        std::thread::sleep(std::time::Duration::from_millis(1_050));
        let _ = rationed.emit(&Event::warn("f", "same"));
        let (copied, _) = rationed.recent();
        assert_eq!(copied.len(), 3, "{copied:?}");
        assert_eq!(copied[1].message, super::FLOOD_MESSAGE);
        assert_eq!(rationed.picture().counted, 3);
        drop(sink);
        drop(rationed);
        let _ignored = std::fs::remove_dir_all(&dir);
        let _ignored = std::fs::remove_dir_all(&flood);
    }

    /// A sink around a target this crate did not open counts too, from an
    /// empty ring.
    #[test]
    fn a_target_sink_counts_from_nothing() {
        let sink = Sink::with_target(
            &Config::new(scratch("target")),
            Box::new(super::tests_support::Discard),
        )
        .expect("a sink");
        assert_eq!(sink.picture().oldest_ms, None);
        let _ = sink.emit(&Event::info("a", "b"));
        assert_eq!(sink.picture().counted, 1);
        assert_eq!(sink.recent().0.len(), 1);
    }
}

#[cfg(test)]
mod tests_support {
    //! A target that keeps nothing, for a sink this crate did not open.

    /// Takes every write and keeps nothing.
    #[derive(Debug)]
    pub(super) struct Discard;

    impl super::Target for Discard {
        fn append(&mut self, _bytes: &[u8]) -> std::io::Result<()> {
            Ok(())
        }

        fn sync(&self) -> std::io::Result<()> {
            Ok(())
        }

        fn reopen(&mut self, _path: &std::path::Path) -> std::io::Result<()> {
            Ok(())
        }
    }
}
