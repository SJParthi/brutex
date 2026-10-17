//! One upkeep run: a store builder carried over a list of items as a TRACKED
//! run — a run id, an append-only journal, N workers, Stop and resume. R3-17,
//! D-4997.
//!
//! # Why it exists
//!
//! The store's catch-up doors — the day index (`.tix`), the sort index
//! (`.srt`), the month summary (`.sum`), the census recount and the GDFL
//! re-key — were each a hand-run verb over one series and one span, one month
//! at a time, with no record a page could read while it ran, no way to stop
//! it but a kill, and no way to go on from where a kill left it. This module
//! is the part every one of them shares; what one item IS and how it is
//! built stays with the caller (`cli upkeep`), which hands [`run`] the item
//! keys in order and one function that builds one of them.
//!
//! # One file per vendor and kind, under `<store>/audit/`, append-only
//!
//! [`journal_name`] — `audit/upkeep.<vendor>.<kind>.log` — holds one line per
//! event, each whole once its newline is written:
//!
//! ```text
//! run <id> v1 vendor=<v> kind=<k> commit=<sha|unstamped> workers=<n> asked=<n> items=<n> skipped=<n> plan=<16 hex> resume=<id|-> at=<unix s>
//! item <id> n=<done> ok=<clean> of=<items> t=<s> clean=<0|1> word=<word> key=<key> why=<sentence>
//! end <id> how=<done|stopped|failed> n=<done> ok=<clean> of=<items> t=<s> why=<sentence>
//! ```
//!
//! Every `item` line carries the run's running totals, so the LAST whole line
//! of the file says where the newest run stands: [`status`] reads a bounded
//! tail ([`TAIL_BYTES`]) and nothing else, whatever the run has written. A
//! line torn by a kill or a full disk is ended, before anything is appended,
//! as a void line (the fragment, then [`VOID`], then the newline), exactly as
//! `cli scrub-blocks` ends its own; no reader reads a void line.
//!
//! # One run at a time per vendor and kind
//!
//! The run holds an exclusive `flock` on its journal for its whole life. A
//! second run of the same vendor and kind is refused by name at once; a run
//! of another kind, or of another vendor, goes on beside it. While a run
//! holds the lock, [`status`] says it is running; a journal whose last line
//! is not an `end` and whose lock is free belongs to a run that DIED.
//!
//! What it shares with an import is not decided here: the caller takes the
//! import's own run lock before it calls [`run`], and every builder takes the
//! month's exclusive store lock, so an import and an upkeep never write one
//! month at once (`cli upkeep`, `store::file::StoreError::Locked`).
//!
//! # Stop
//!
//! Two ways, both seen between items and never inside one: the caller's own
//! flag ([`Ask::stop`], for a run inside the app) and a request file,
//! [`stop_name`], naming the run ([`request_stop`], for `cli upkeep stop`). A
//! request for another run id is not this run's and is ignored, so a stale
//! file never stops a later run and nothing has to remove it. Each worker
//! finishes the item it holds, takes no other, and the run ends `stopped`.
//!
//! # Resume
//!
//! [`Ask::resume`] names an earlier run. The new run gets an id of its own and
//! its `run` line says `resume=<id>`; every key a run of that chain (the run
//! named, the one it resumed, and so on) closed CLEAN is skipped, and the
//! rest are built: what a kill or a Stop left, what failed, and anything the
//! caller's plan holds now that it did not. Reading the chain is one pass
//! over the journal, done once per resume and never on a request path.
//!
//! # Workers
//!
//! [`Kind::workers`]: up to [`MAX_WORKERS`] for a per-month kind, ONE for the
//! recount and the re-key, which write one census and one ledger that a
//! second worker would race. Workers take the next item from one counter, so
//! no item is built twice and the order they finish in is the machine's; the
//! outcomes come back in plan order whatever it was ([`Ran::outcomes`]), and
//! what a builder writes for one month is that month's alone, so the bytes of
//! the store do not depend on the count (`cli upkeep`'s
//! `the_sort_index_is_the_same_bytes_at_1_and_8_workers`).
//!
//! # Cost
//!
//! Per item: one `item` line (one `write`), a `sync_data` every
//! [`SYNC_EVERY`] items and at the end, and one open of the stop file per
//! worker per item (`ENOENT` when no stop was asked). A kill can lose at most
//! the unsynced lines; their items are built again on resume, which every
//! builder here keeps (`kept`) when it finds them done. [`status`] is one
//! bounded tail read and one `flock` probe. The months walk ([`months`]) is
//! O(directory entries) under the scope, with one pointer open and three
//! `lstat`s per month file.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{BufRead as _, BufReader, ErrorKind, Read as _, Seek as _, SeekFrom, Write as _};
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use store::flock::Flock;
use store::open_flags::O_NOFOLLOW_NONBLOCK;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// The directory every upkeep file lives in, relative to the store root.
pub const DIR: &str = "audit";

/// The most workers one run starts.
pub const MAX_WORKERS: usize = 64;

/// The most journal bytes [`status`] reads, from the end. An `item` line is a
/// key of at most [`KEY_CAP`] bytes, a reason of at most [`WHY_CAP`] and a
/// few short fields, so this always holds several whole lines.
pub const TAIL_BYTES: u64 = 16 * 1024;

/// What ends a torn line: the fragment, then this, then the newline.
pub const VOID: &str = " #void";

/// The most bytes of an item key. A store path is far shorter
/// (`store::path::MAX_LEN`).
pub const KEY_CAP: usize = 512;

/// The most bytes of a reason a line keeps; the rest is cut and the cut said.
pub const WHY_CAP: usize = 1_024;

/// Items between two flushes of the journal to the disk.
pub const SYNC_EVERY: u64 = 64;

/// The most bytes of a stop request that are read.
const STOP_CAP: u64 = 64;

/// The first word after the id on every `run` line: the journal's format.
const FORMAT: &str = "v1";

/// What an upkeep run builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The `.tix` time index of a month (`cli tix-rebuild`).
    DayIndex,
    /// The `.srt` sort index of a month (`cli srt-build`).
    SortIndex,
    /// The `.sum` month summary of a month (`cli msum-build`).
    MonthSummary,
    /// The census rows of one month's days, from the bar files' headers
    /// (`cli census-recount`).
    Recount,
    /// The GDFL option keys of one underlying, judged and moved or retired
    /// (`cli gdfl-rekey`).
    Rekey,
}

impl Kind {
    /// Every kind, in the order the plan lists them.
    pub const ALL: [Self; 5] = [
        Self::DayIndex,
        Self::SortIndex,
        Self::MonthSummary,
        Self::Recount,
        Self::Rekey,
    ];

    /// The kind's word: in commands, file names and journal lines.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::DayIndex => "tix",
            Self::SortIndex => "srt",
            Self::MonthSummary => "msum",
            Self::Recount => "recount",
            Self::Rekey => "rekey",
        }
    }

    /// The kind a word names, if it names one.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.word() == word)
    }

    /// Whether one item is one month file and nothing else, so two items can
    /// be built at once: the three indexes. The recount writes the census and
    /// the import journal, the re-key the census and the lineage ledger, and
    /// a second worker would race the first on those files.
    #[must_use]
    pub const fn per_month(self) -> bool {
        matches!(self, Self::DayIndex | Self::SortIndex | Self::MonthSummary)
    }

    /// The month sidecar this kind builds, for a per-month kind.
    #[must_use]
    pub const fn sidecar(self) -> Option<FileKind> {
        match self {
            Self::DayIndex => Some(FileKind::TimeIndex),
            Self::SortIndex => Some(FileKind::SortIndex),
            Self::MonthSummary => Some(FileKind::MonthSummary),
            Self::Recount | Self::Rekey => None,
        }
    }

    /// The workers a run of this kind starts when `asked` were asked for:
    /// between one and [`MAX_WORKERS`] for a per-month kind, one otherwise.
    #[must_use]
    pub fn workers(self, asked: usize) -> usize {
        if self.per_month() {
            asked.clamp(1, MAX_WORKERS)
        } else {
            1
        }
    }
}

/// The journal of `vendor`'s runs of `kind`, relative to the store root.
#[must_use]
pub fn journal_name(vendor: Vendor, kind: Kind) -> String {
    format!("{DIR}/upkeep.{}.{}.log", vendor.as_str(), kind.word())
}

/// The stop request of `vendor`'s runs of `kind`, relative to the store root.
#[must_use]
pub fn stop_name(vendor: Vendor, kind: Kind) -> String {
    format!("{DIR}/upkeep.{}.{}.stop", vendor.as_str(), kind.word())
}

/// What building one item came to, in the builder's own words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Whether the item ended as the run wants it: built, kept, or needing
    /// nothing. A clean item is skipped by a resume; any other is built again.
    pub clean: bool,
    /// One word for the line: `built`, `kept`, `small`, `REFUSED`, ...
    pub word: String,
    /// What happened, in one sentence.
    pub why: String,
}

impl Outcome {
    /// A clean outcome.
    #[must_use]
    pub fn clean(word: &str, why: impl Into<String>) -> Self {
        Self {
            clean: true,
            word: word.to_owned(),
            why: why.into(),
        }
    }

    /// An outcome that is not clean.
    #[must_use]
    pub fn failed(word: &str, why: impl Into<String>) -> Self {
        Self {
            clean: false,
            word: word.to_owned(),
            why: why.into(),
        }
    }
}

/// What one run is asked to do, beside its items and its builder.
pub struct Ask<'a> {
    /// The feed whose store is kept.
    pub vendor: Vendor,
    /// What is built.
    pub kind: Kind,
    /// The workers asked for; [`Kind::workers`] decides how many start.
    pub workers: usize,
    /// The earlier run this one resumes.
    pub resume: Option<u64>,
    /// The build's commit, or `None` for an unstamped build.
    pub commit: Option<&'a str>,
    /// The caller's own Stop: set, and the run takes no further item.
    pub stop: &'a AtomicBool,
    /// UNIX seconds now: the run id and the `run` line's `at=`.
    pub now: &'a (dyn Fn() -> u64 + Sync),
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// Every item it was given was built, clean or not.
    Done,
    /// A Stop was asked and seen; items are left.
    Stopped,
    /// The journal could not be written; the run stopped taking items.
    Failed,
}

impl How {
    /// The word on the `end` line.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }

    /// The `how` a word names.
    fn parse(word: &str) -> Option<Self> {
        [Self::Done, Self::Stopped, Self::Failed]
            .into_iter()
            .find(|how| how.word() == word)
    }
}

/// What one run did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ran {
    /// The run's id.
    pub id: u64,
    /// The run it resumed.
    pub resumed: Option<u64>,
    /// The workers it started.
    pub workers: usize,
    /// Items of the plan a run of the resumed chain had already closed clean:
    /// not built again.
    pub skipped: usize,
    /// Per item of the plan, in plan order: what building it came to, or
    /// `None` for an item skipped (resumed) or not reached (stopped, failed).
    pub outcomes: Vec<Option<Outcome>>,
    /// How it ended.
    pub how: How,
    /// Why it ended so, for a stop or a failure.
    pub why: String,
    /// Bytes of a torn tail ended as a void line before the run began.
    pub torn: u64,
    /// Milliseconds the items took, wall clock: the run's own measurement of
    /// its rate (items per second), on the host and the load it ran under.
    pub millis: u64,
}

impl Ran {
    /// Items built this run.
    #[must_use]
    pub fn built(&self) -> usize {
        self.outcomes.iter().flatten().count()
    }

    /// Items built this run that ended clean.
    #[must_use]
    pub fn clean(&self) -> usize {
        self.outcomes
            .iter()
            .flatten()
            .filter(|outcome| outcome.clean)
            .count()
    }

    /// Items neither skipped nor built: what a Stop or a failure left.
    #[must_use]
    pub fn left(&self) -> usize {
        self.outcomes
            .len()
            .saturating_sub(self.skipped)
            .saturating_sub(self.built())
    }

    /// Whether the run walked its whole plan and every item ended clean.
    #[must_use]
    pub fn whole(&self) -> bool {
        self.how == How::Done && self.clean() == self.built() && self.left() == 0
    }
}

/// One line of the journal, as [`parse`] reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A run began.
    Run {
        /// Its id.
        id: u64,
        /// The run it resumed.
        resume: Option<u64>,
        /// Items it was given to build (the plan less the skipped).
        items: u64,
        /// Items of the plan it skipped.
        skipped: u64,
        /// The workers it started.
        workers: u64,
        /// When it began, UNIX seconds.
        at: u64,
    },
    /// One item was built.
    Item {
        /// The run's id.
        id: u64,
        /// Items built so far by the run, this one included.
        n: u64,
        /// Of those, the clean ones.
        ok: u64,
        /// Items the run was given.
        of: u64,
        /// Seconds since the run began.
        t: u64,
        /// Whether this item ended clean.
        clean: bool,
        /// The item's key.
        key: String,
    },
    /// A run ended.
    End {
        /// Its id.
        id: u64,
        /// How.
        how: How,
        /// Items built.
        n: u64,
        /// Of those, the clean ones.
        ok: u64,
        /// Items it was given.
        of: u64,
        /// Seconds the items took.
        t: u64,
    },
}

impl Entry {
    /// The run the line belongs to.
    #[must_use]
    pub const fn id(&self) -> u64 {
        match self {
            Self::Run { id, .. } | Self::Item { id, .. } | Self::End { id, .. } => *id,
        }
    }
}

/// `name=<number>`, exactly.
fn number(word: Option<&str>, name: &str) -> Option<u64> {
    word?.strip_prefix(name)?.strip_prefix('=')?.parse().ok()
}

/// `name=<text>`, exactly.
fn text<'a>(word: Option<&'a str>, name: &str) -> Option<&'a str> {
    word?.strip_prefix(name)?.strip_prefix('=')
}

/// One journal line, read STRICTLY: exactly the words this module writes, in
/// its order, each number a number. A void line, a line of another format and
/// a line two writers' bytes were joined into are no entry at all.
#[must_use]
pub fn parse(line: &str) -> Option<Entry> {
    if line.ends_with(VOID) {
        return None;
    }
    let mut words = line.split(' ');
    let head = words.next()?;
    let id = words.next()?.parse().ok()?;
    match head {
        "run" => {
            if words.next()? != FORMAT {
                return None;
            }
            let _vendor = text(words.next(), "vendor")?;
            let _kind = text(words.next(), "kind")?;
            let _commit = text(words.next(), "commit")?;
            let workers = number(words.next(), "workers")?;
            let _asked = number(words.next(), "asked")?;
            let items = number(words.next(), "items")?;
            let skipped = number(words.next(), "skipped")?;
            let _plan = text(words.next(), "plan")?;
            let resume = match text(words.next(), "resume")? {
                "-" => None,
                other => Some(other.parse().ok()?),
            };
            let at = number(words.next(), "at")?;
            words.next().is_none().then_some(Entry::Run {
                id,
                resume,
                items,
                skipped,
                workers,
                at,
            })
        }
        "item" => {
            let n = number(words.next(), "n")?;
            let ok = number(words.next(), "ok")?;
            let of = number(words.next(), "of")?;
            let t = number(words.next(), "t")?;
            let clean = match text(words.next(), "clean")? {
                "1" => true,
                "0" => false,
                _ => return None,
            };
            let _word = text(words.next(), "word")?;
            let key = text(words.next(), "key")?;
            let _why = text(words.next(), "why")?;
            Some(Entry::Item {
                id,
                n,
                ok,
                of,
                t,
                clean,
                key: key.to_owned(),
            })
        }
        "end" => {
            let how = How::parse(text(words.next(), "how")?)?;
            let n = number(words.next(), "n")?;
            let ok = number(words.next(), "ok")?;
            let of = number(words.next(), "of")?;
            let t = number(words.next(), "t")?;
            let _why = text(words.next(), "why")?;
            Some(Entry::End {
                id,
                how,
                n,
                ok,
                of,
                t,
            })
        }
        _ => None,
    }
}

/// A reason as a line keeps it: one line, at most [`WHY_CAP`] bytes, the cut
/// said.
#[must_use]
pub fn one_line(text: &str) -> String {
    let mut line = text.replace(['\n', '\r'], " ");
    if line.len() > WHY_CAP {
        let mut cut = WHY_CAP;
        while !line.is_char_boundary(cut) {
            cut -= 1;
        }
        let rest = line.len() - cut;
        line.truncate(cut);
        let _ = write!(line, "...(+{rest} bytes)");
    }
    line
}

/// Why `key` cannot be an item key, or `None` when it can: not empty, at
/// most [`KEY_CAP`] bytes, and no space or control character, so a line's
/// fields stay one word each and a key never breaks a line.
#[must_use]
pub fn key_refused(key: &str) -> Option<String> {
    if key.is_empty() {
        return Some("an empty item key".to_owned());
    }
    if key.len() > KEY_CAP {
        return Some(format!(
            "the item key `{}...` is {} bytes, more than {KEY_CAP}",
            key.chars().take(64).collect::<String>().escape_debug(),
            key.len()
        ));
    }
    key.chars()
        .any(|c| c.is_whitespace() || c.is_control())
        .then(|| {
            format!(
                "the item key `{}` holds a space or a control character",
                key.escape_debug()
            )
        })
}

/// The plan's digest on the `run` line: FNV-1a of its keys, one per line.
#[must_use]
pub fn plan_digest<'a>(keys: impl IntoIterator<Item = &'a str>) -> u64 {
    let mut joined = String::new();
    for key in keys {
        joined.push_str(key);
        joined.push('\n');
    }
    brutex_core::universe::fnv1a(&joined)
}

/// The journal, open for append and read, its lock held.
struct Journal {
    flock: Flock<File>,
    path: PathBuf,
}

impl Journal {
    /// Opens `vendor`'s journal of `kind` under `root`, creating it and the
    /// audit directory when absent (never the root above them), and takes its
    /// exclusive lock without waiting.
    ///
    /// The open never follows a final link and never waits on a pipe; what it
    /// opened must be a regular file with one name. Anything else is refused
    /// by name and nothing is written.
    fn open(root: &Path, vendor: Vendor, kind: Kind) -> Result<Self, String> {
        let path = root.join(journal_name(vendor, kind));
        if let Some(dir) = path.parent() {
            match fs::create_dir(dir) {
                Ok(()) => {}
                Err(why) if why.kind() == ErrorKind::AlreadyExists => {}
                Err(why) => return Err(format!("{}: {why}; nothing was written", dir.display())),
            }
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .custom_flags(O_NOFOLLOW_NONBLOCK)
            .open(&path)
            .map_err(|why| refusal(&path, &why))?;
        plain(&file, &path)?;
        let flock = match Flock::try_lock(file, path.clone()) {
            Ok(flock) => flock,
            Err(TryLockError::WouldBlock) => {
                return Err(format!(
                    "another {} upkeep run of {} holds {}: one run of a kind at a time; \
                     nothing was written",
                    kind.word(),
                    vendor.as_str(),
                    path.display()
                ));
            }
            Err(TryLockError::Error(why)) => {
                return Err(format!(
                    "{}: flock: {why}; nothing was written",
                    path.display()
                ));
            }
        };
        Ok(Self { flock, path })
    }

    fn named(&self, why: &std::io::Error) -> String {
        format!("{}: {why}", self.path.display())
    }

    /// Ends a tail no newline ends, as a void line, and returns the bytes of
    /// the fragment it ended (zero when the file is empty or ends whole).
    fn end_torn_tail(&mut self) -> Result<u64, String> {
        let (tail, _) = tail_of(&mut self.flock, &self.path)?;
        // THE FRAGMENT IS WHAT FOLLOWS THE LAST NEWLINE (all of a tail with
        // none): empty when the file ends whole.
        let torn = tail
            .rsplit(|byte| *byte == b'\n')
            .next()
            .map_or(0, <[u8]>::len);
        if torn == 0 {
            return Ok(0);
        }
        let mut end = Vec::with_capacity(VOID.len() + 1);
        end.extend_from_slice(VOID.as_bytes());
        end.push(b'\n');
        self.flock
            .write_all(&end)
            .and_then(|()| self.flock.sync_data())
            .map_err(|why| self.named(&why))?;
        Ok(u64::try_from(torn).unwrap_or(u64::MAX))
    }

    /// The newest run id among the tail's whole lines, or zero.
    fn newest_id(&mut self) -> Result<u64, String> {
        let (tail, cut) = tail_of(&mut self.flock, &self.path)?;
        Ok(whole_lines(&tail, cut)
            .filter_map(parse)
            .map(|entry| entry.id())
            .max()
            .unwrap_or(0))
    }

    /// Every entry of the file, in order: one pass, for a resume.
    fn entries(&mut self) -> Result<Vec<Entry>, String> {
        self.flock
            .seek(SeekFrom::Start(0))
            .map_err(|why| self.named(&why))?;
        let mut out = Vec::new();
        for line in BufReader::new(&*self.flock).split(b'\n') {
            let line = line.map_err(|why| self.named(&why))?;
            if let Some(entry) = std::str::from_utf8(&line).ok().and_then(parse) {
                out.push(entry);
            }
        }
        Ok(out)
    }

    /// One line, in one write, newline included.
    fn line(&mut self, text: &str) -> Result<(), String> {
        let mut bytes = Vec::with_capacity(text.len() + 2);
        bytes.extend_from_slice(text.as_bytes());
        if text.ends_with(VOID) {
            // No written line reads as a void one, whatever a reason ended in.
            bytes.push(b'.');
        }
        bytes.push(b'\n');
        self.flock.write_all(&bytes).map_err(|why| self.named(&why))
    }

    fn sync(&self) -> Result<(), String> {
        self.flock.sync_data().map_err(|why| self.named(&why))
    }
}

/// Why `path` did not open, by its cause.
fn refusal(path: &Path, why: &std::io::Error) -> String {
    let link = fs::symlink_metadata(path).is_ok_and(|at| at.file_type().is_symlink());
    if link {
        format!(
            "{}: {why} (it must be a regular file, never a link); nothing was written",
            path.display()
        )
    } else {
        format!("{}: {why}; nothing was written", path.display())
    }
}

/// Refuses what was opened at `path` unless it is a regular file of one name:
/// a second name (a hard link) would carry every line into another file.
fn plain(file: &File, path: &Path) -> Result<(), String> {
    let meta = file
        .metadata()
        .map_err(|why| format!("{}: {why}", path.display()))?;
    if !meta.is_file() {
        return Err(format!(
            "{} is not a regular file; nothing was written",
            path.display()
        ));
    }
    if meta.nlink() != 1 {
        return Err(format!(
            "{} has {} names (a hard link); nothing was written through it",
            path.display(),
            meta.nlink()
        ));
    }
    Ok(())
}

/// The last [`TAIL_BYTES`] of `file`, and whether that cut off an earlier
/// part of it.
fn tail_of(file: &mut File, path: &Path) -> Result<(Vec<u8>, bool), String> {
    let named = |why: std::io::Error| format!("{}: {why}", path.display());
    let len = file.metadata().map_err(named)?.len();
    let from = len.saturating_sub(TAIL_BYTES);
    file.seek(SeekFrom::Start(from)).map_err(named)?;
    let mut bytes = Vec::new();
    file.take(TAIL_BYTES)
        .read_to_end(&mut bytes)
        .map_err(named)?;
    Ok((bytes, from > 0))
}

/// The whole lines of a tail, in order: the first is dropped when the tail
/// was `cut` (it may be a line's end), and a last piece no newline ends is
/// never a line.
fn whole_lines(tail: &[u8], cut: bool) -> impl Iterator<Item = &str> {
    let mut pieces: Vec<&[u8]> = tail.split(|byte| *byte == b'\n').collect();
    // `split` gives one piece past the last newline: empty when the tail
    // ends whole, a torn fragment otherwise. Neither is a line.
    let _past = pieces.pop();
    pieces
        .into_iter()
        .skip(usize::from(cut))
        .filter_map(|piece| std::str::from_utf8(piece).ok())
}

/// Whether a stop request at `path` names run `id`.
fn stop_asked_for(path: &Path, id: u64) -> bool {
    let Ok(file) = OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW_NONBLOCK)
        .open(path)
    else {
        return false;
    };
    let mut text = String::new();
    if file.take(STOP_CAP).read_to_string(&mut text).is_err() {
        return false;
    }
    text.trim_end() == format!("stop {id}")
}

/// Asks run `id` of `vendor`'s `kind` to stop: writes the request file
/// [`stop_name`] naming it, whole — a new file renamed over the name, never
/// edited in place. The run sees it before its next item; a request for a run
/// that is not running stops nothing, now or later.
///
/// # Errors
///
/// The staging file's refusal (`store::staging::create`), or the host's
/// refusal to write, flush or rename it.
pub fn request_stop(root: &Path, vendor: Vendor, kind: Kind, id: u64) -> Result<PathBuf, String> {
    let path = root.join(stop_name(vendor, kind));
    if let Some(dir) = path.parent() {
        match fs::create_dir(dir) {
            Ok(()) => {}
            Err(why) if why.kind() == ErrorKind::AlreadyExists => {}
            Err(why) => return Err(format!("{}: {why}", dir.display())),
        }
    }
    let staging = path.with_extension("stop.new");
    let mut staged = store::staging::create(&staging).map_err(|why| why.to_string())?;
    let named = |why: std::io::Error| format!("{}: {why}", staging.display());
    staged
        .file
        .write_all(format!("stop {id}\n").as_bytes())
        .and_then(|()| staged.file.sync_all())
        .map_err(named)?;
    fs::rename(&staging, &path).map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(path)
}

/// Where the newest run of a journal stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// The newest run's id.
    pub id: u64,
    /// Whether it is running, ended, or died.
    pub state: State,
    /// Items built so far.
    pub n: u64,
    /// Of those, the clean ones.
    pub ok: u64,
    /// Items it was given.
    pub of: u64,
    /// Seconds since it began, as its last line said.
    pub t: u64,
}

impl Status {
    /// Seconds left at the rate so far, for a running run that has built at
    /// least one item: an extrapolation, said as one.
    #[must_use]
    pub fn eta(&self) -> Option<u64> {
        (self.state == State::Running && self.n > 0)
            .then(|| self.t.saturating_mul(self.of.saturating_sub(self.n)) / self.n)
    }
}

/// The newest run's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Its lock is held.
    Running,
    /// Its `end` line says how.
    Ended(How),
    /// No `end` line and its lock is free: it was killed, or its host went.
    Died,
}

/// Where `vendor`'s newest run of `kind` stands, from the journal's last whole
/// line and one probe of its lock; `None` when no run has written a line.
///
/// # Errors
///
/// A journal that cannot be opened or read, or that is not a regular file.
pub fn status(root: &Path, vendor: Vendor, kind: Kind) -> Result<Option<Status>, String> {
    let path = root.join(journal_name(vendor, kind));
    let mut file = match OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW_NONBLOCK)
        .open(&path)
    {
        Ok(file) => file,
        Err(why) if why.kind() == ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(refusal(&path, &why)),
    };
    plain(&file, &path)?;
    let (tail, cut) = tail_of(&mut file, &path)?;
    let Some(last) = whole_lines(&tail, cut).filter_map(parse).last() else {
        return Ok(None);
    };
    let running = match file.try_lock_shared() {
        Ok(()) => {
            let _ = file.unlock();
            false
        }
        Err(TryLockError::WouldBlock) => true,
        Err(TryLockError::Error(why)) => return Err(format!("{}: flock: {why}", path.display())),
    };
    let (id, n, ok, of, t, ended) = match last {
        Entry::Run { id, items, .. } => (id, 0, 0, items, 0, None),
        Entry::Item {
            id, n, ok, of, t, ..
        } => (id, n, ok, of, t, None),
        Entry::End {
            id,
            how,
            n,
            ok,
            of,
            t,
        } => (id, n, ok, of, t, Some(how)),
    };
    let state = match (ended, running) {
        (Some(how), _) => State::Ended(how),
        (None, true) => State::Running,
        (None, false) => State::Died,
    };
    Ok(Some(Status {
        id,
        state,
        n,
        ok,
        of,
        t,
    }))
}

/// The keys every run of `id`'s chain closed clean, read from `entries`.
///
/// # Errors
///
/// No `run` line for `id`: there is nothing to resume, and starting from the
/// beginning in silence is not what a resume asked.
fn done_by_chain(entries: &[Entry], id: u64) -> Result<HashSet<String>, String> {
    let parents: HashMap<u64, Option<u64>> = entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Run { id, resume, .. } => Some((*id, *resume)),
            Entry::Item { .. } | Entry::End { .. } => None,
        })
        .collect();
    if !parents.contains_key(&id) {
        return Err(format!(
            "no run {id} in this journal: nothing to resume, and nothing was written"
        ));
    }
    let mut chain = HashSet::with_capacity(parents.len());
    let mut at = Some(id);
    // A run already in the chain ends it: a hand-edited cycle cannot loop.
    while let Some(run) = at.filter(|run| chain.insert(*run)) {
        at = parents.get(&run).copied().flatten();
    }
    Ok(entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Item {
                id,
                clean: true,
                key,
                ..
            } if chain.contains(id) => Some(key.clone()),
            Entry::Run { .. } | Entry::Item { .. } | Entry::End { .. } => None,
        })
        .collect())
}

/// Where a run's lines go: its journal, or, in a test, a journal that stops
/// taking them partway.
trait Lines: Send {
    /// Appends one whole line.
    fn line(&mut self, text: &str) -> Result<(), String>;
    /// Flushes what was appended to the disk.
    fn sync(&mut self) -> Result<(), String>;
}

impl Lines for Journal {
    fn line(&mut self, text: &str) -> Result<(), String> {
        Self::line(self, text)
    }

    fn sync(&mut self) -> Result<(), String> {
        Self::sync(self)
    }
}

/// What the workers share, under one lock: the lines and the totals they
/// carry, and the outcomes in plan order.
struct Shared<'a> {
    lines: &'a mut dyn Lines,
    n: u64,
    ok: u64,
    outcomes: Vec<Option<Outcome>>,
    fault: Option<String>,
}

impl Shared<'_> {
    /// Records one built item: its line, carrying the totals it brings, and
    /// its outcome. Whether the line was taken: when it is refused, or another
    /// worker's was, nothing of the item is counted, so a resume builds it
    /// again.
    fn record(
        &mut self,
        at: usize,
        outcome: Outcome,
        line: impl FnOnce(u64, u64, &Outcome) -> String,
    ) -> bool {
        if self.fault.is_some() {
            return false;
        }
        let (n, ok) = (self.n + 1, self.ok + u64::from(outcome.clean));
        let written = self.lines.line(&line(n, ok, &outcome)).and_then(|()| {
            if n.is_multiple_of(SYNC_EVERY) {
                self.lines.sync()
            } else {
                Ok(())
            }
        });
        match written {
            Ok(()) => {
                (self.n, self.ok) = (n, ok);
                let _ = self.outcomes.get_mut(at).map(|slot| slot.replace(outcome));
                true
            }
            Err(why) => {
                self.fault = Some(why);
                false
            }
        }
    }
}

/// The `run` line: the id, the format, what is built and for which feed, the
/// commit, the workers started and asked, the items given and skipped, the
/// plan's digest, the run resumed and the clock.
fn run_line(
    id: u64,
    ask: &Ask<'_>,
    digest: u64,
    (workers, of, skipped): (usize, u64, usize),
) -> String {
    format!(
        "run {id} {FORMAT} vendor={} kind={} commit={} workers={workers} asked={} items={of} \
         skipped={skipped} plan={digest:016x} resume={} at={}",
        ask.vendor.as_str(),
        ask.kind.word(),
        ask.commit.unwrap_or("unstamped"),
        ask.workers,
        ask.resume
            .map_or_else(|| "-".to_owned(), |resumed| resumed.to_string()),
        (ask.now)()
    )
}

/// How a run ended and why, from what the workers left: a refused line, the
/// items left, and whether a Stop was seen. A Stop seen once the last item
/// was taken left nothing: the run is done.
fn ending(fault: Option<&str>, left: u64, stopped: bool, id: u64) -> (How, String) {
    match fault {
        Some(why) => (
            How::Failed,
            format!("the journal stopped taking lines: {why}"),
        ),
        None if stopped && left > 0 => (
            How::Stopped,
            format!("a Stop was asked; {left} item(s) left for a resume of run {id}"),
        ),
        None => (How::Done, "every item of the run was built".to_owned()),
    }
}

/// What [`run`] settled before the first line: the id, the plan, the keys
/// the resumed chain closed clean, and the torn bytes it ended.
struct Begun<'a, T> {
    id: u64,
    plan: &'a [(String, T)],
    /// `None` for a fresh run: nothing is skipped.
    done: Option<HashSet<String>>,
    torn: u64,
}

/// Runs `build` over every item of `plan` the resumed chain did not close
/// clean, on [`Kind::workers`] workers, as one tracked run of `ask.kind` for
/// `ask.vendor` under `root`. Each item of the plan is its key, the word the
/// journal names it by, and what the caller builds from; `build` is handed
/// both.
///
/// # Errors
///
/// Before any item: a key [`key_refused`] refuses or that appears twice, a
/// journal that cannot be opened, locked or read, another run of the kind
/// holding it, a resume of a run the journal does not hold, or a `run` line
/// that cannot be written. Nothing is built then. Once items are built, a
/// journal that stops taking lines ends the run [`How::Failed`] instead, with
/// what was built; so does a lock the host will not release.
pub fn run<T: Sync>(
    root: &Path,
    ask: &Ask<'_>,
    plan: &[(String, T)],
    build: &(dyn Fn(&str, &T) -> Outcome + Sync),
) -> Result<Ran, String> {
    let mut seen = HashSet::with_capacity(plan.len());
    for (key, _) in plan {
        if let Some(why) = key_refused(key) {
            return Err(format!("{why}; nothing was written"));
        }
        if !seen.insert(key.as_str()) {
            return Err(format!(
                "the item key `{key}` is in the plan twice; nothing was written"
            ));
        }
    }
    let mut journal = Journal::open(root, ask.vendor, ask.kind)?;
    let torn = journal.end_torn_tail()?;
    let done = match ask.resume {
        Some(resumed) => Some(done_by_chain(&journal.entries()?, resumed)?),
        None => None,
    };
    let id = (ask.now)().max(journal.newest_id()?.saturating_add(1));
    let begun = Begun {
        id,
        plan,
        done,
        torn,
    };
    let mut ran = drive(&mut journal, root, ask, begun, build)?;
    if let Err(why) = journal.flock.release() {
        ran.how = How::Failed;
        ran.why = format!(
            "{} — and the journal's lock was not released: {why}",
            ran.why
        );
    }
    Ok(ran)
}

/// The run itself, once its id is settled: the `run` line, the workers, the
/// `end` line.
fn drive<T: Sync>(
    lines: &mut dyn Lines,
    root: &Path,
    ask: &Ask<'_>,
    begun: Begun<'_, T>,
    build: &(dyn Fn(&str, &T) -> Outcome + Sync),
) -> Result<Ran, String> {
    let Begun {
        id,
        plan,
        done,
        torn,
    } = begun;
    let todo: Vec<(usize, &(String, T))> = plan
        .iter()
        .enumerate()
        .filter(|(_, (key, _))| done.as_ref().is_none_or(|done| !done.contains(key)))
        .collect();
    let skipped = plan.len() - todo.len();
    let workers = ask.kind.workers(ask.workers).min(todo.len().max(1));
    let of = u64::try_from(todo.len()).unwrap_or(u64::MAX);
    let digest = plan_digest(plan.iter().map(|(key, _)| key.as_str()));
    lines.line(&run_line(id, ask, digest, (workers, of, skipped)))?;
    lines.sync()?;
    let stop_file = root.join(stop_name(ask.vendor, ask.kind));
    let started = Instant::now();
    let next = AtomicUsize::new(0);
    let stopped = AtomicBool::new(false);
    let shared = Mutex::new(Shared {
        lines,
        n: 0,
        ok: 0,
        outcomes: vec![None; plan.len()],
        fault: None,
    });
    let faulted = AtomicBool::new(false);
    let worker = || {
        loop {
            // NO ITEM IS BUILT THAT NO LINE CAN RECORD: once one line was
            // refused, every worker stops at its next look.
            if faulted.load(Ordering::Acquire) {
                return;
            }
            if ask.stop.load(Ordering::Acquire) || stop_asked_for(&stop_file, id) {
                stopped.store(true, Ordering::Release);
                return;
            }
            let Some(&(at, (key, item))) = todo.get(next.fetch_add(1, Ordering::AcqRel)) else {
                return;
            };
            let outcome = build(key, item);
            let line = |n: u64, ok: u64, outcome: &Outcome| {
                format!(
                    "item {id} n={n} ok={ok} of={of} t={} clean={} word={} key={key} why={}",
                    started.elapsed().as_secs(),
                    u8::from(outcome.clean),
                    one_word(&outcome.word),
                    one_line(&outcome.why)
                )
            };
            let mut shared = shared.lock().unwrap_or_else(PoisonError::into_inner);
            if !shared.record(at, outcome, line) {
                faulted.store(true, Ordering::Release);
                return;
            }
        }
    };
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(worker);
        }
    });
    let shared = shared.into_inner().unwrap_or_else(PoisonError::into_inner);
    let millis = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let seconds = millis / 1_000;
    let (how, why) = ending(
        shared.fault.as_deref(),
        of.saturating_sub(shared.n),
        stopped.load(Ordering::Acquire),
        id,
    );
    let end = format!(
        "end {id} how={} n={} ok={} of={of} t={seconds} why={}",
        how.word(),
        shared.n,
        shared.ok,
        one_line(&why)
    );
    let (how, why) = match shared.lines.line(&end).and_then(|()| shared.lines.sync()) {
        Ok(()) => (how, why),
        Err(fault) => (
            How::Failed,
            format!("{why} — and the end line was not written: {fault}"),
        ),
    };
    Ok(Ran {
        id,
        resumed: ask.resume,
        workers,
        skipped,
        outcomes: shared.outcomes,
        how,
        why,
        torn,
        millis,
    })
}

/// A builder's word as one word of a line.
fn one_word(word: &str) -> String {
    let word: String = word
        .chars()
        .map(|c| {
            if c.is_whitespace() || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .take(32)
        .collect();
    if word.is_empty() {
        "-".to_owned()
    } else {
        word
    }
}

// ═══ The months a scope holds ═══════════════════════════════════════════════

/// One stored month file of one series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Month {
    /// The exchange segment of its path.
    pub exchange: String,
    /// The segment segment of its path.
    pub segment: String,
    /// The symbol segment of its path.
    pub symbol: String,
    /// The contract, for a contract month.
    pub contract: Option<Contract>,
    /// The rung.
    pub rung: Timeframe,
    /// The month.
    pub month: YearMonth,
    /// Its `.bin`, relative to the store root: the item key.
    pub key: String,
    /// The sidecars the month has none of, beside the generation every
    /// reader opens.
    pub lacks: Lacks,
}

impl Month {
    /// The month's bar path in `vendor`'s store.
    ///
    /// # Errors
    ///
    /// None for a month [`months`] found: its path was proven when it was.
    pub fn path(&self, vendor: Vendor) -> Result<StorePath<'_>, String> {
        StorePath::new(PathParts {
            vendor,
            exchange: &self.exchange,
            segment: &self.segment,
            symbol: &self.symbol,
            contract: self.contract,
            timeframe: self.rung,
            month: self.month,
            file: FileKind::Bars,
        })
        .map_err(|why| format!("{} is not a store path: {why}", self.key))
    }
}

/// The per-month kinds a month has no sidecar of: one bit each, so asking is
/// one mask, never a scan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Lacks(u8);

impl Lacks {
    /// The bit of `kind`, or none for a kind that keeps no sidecar.
    const fn bit(kind: Kind) -> u8 {
        match kind {
            Kind::DayIndex => 1,
            Kind::SortIndex => 2,
            Kind::MonthSummary => 4,
            Kind::Recount | Kind::Rekey => 0,
        }
    }

    /// The set of `kinds`.
    #[must_use]
    pub fn of(kinds: impl IntoIterator<Item = Kind>) -> Self {
        Self(
            kinds
                .into_iter()
                .fold(0, |bits, kind| bits | Self::bit(kind)),
        )
    }

    /// Whether the month lacks `kind`'s sidecar.
    #[must_use]
    pub const fn has(self, kind: Kind) -> bool {
        self.0 & Self::bit(kind) != 0
    }

    /// The kinds lacked, in [`Kind::ALL`] order.
    pub fn kinds(self) -> impl Iterator<Item = Kind> {
        Kind::ALL.into_iter().filter(move |kind| self.has(*kind))
    }
}

/// Which part of a vendor's store [`months`] walks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    /// Path segments under `bars/<vendor>/`: none for the whole feed, then
    /// the exchange, the segment, the symbol and one contract, each narrowing.
    pub prefix: Vec<String>,
    /// Only this rung.
    pub rung: Option<Timeframe>,
    /// Only months from this one.
    pub from: Option<YearMonth>,
    /// Only months up to this one.
    pub to: Option<YearMonth>,
}

impl Scope {
    fn holds(&self, rung: Timeframe, month: YearMonth) -> bool {
        self.rung.is_none_or(|want| want == rung)
            && self.from.is_none_or(|from| month >= from)
            && self.to.is_none_or(|to| month <= to)
    }

    /// Whether the walk may go into the entry named `name` at `depth` (0 is
    /// the exchange).
    fn admits(&self, depth: usize, name: &str) -> bool {
        self.prefix.get(depth).is_none_or(|want| want == name)
    }
}

/// What a walk found and what it left alone.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Walked {
    /// The month files in scope, in path order.
    pub months: Vec<Month>,
    /// Symbolic links met: never followed.
    pub links: u64,
    /// Entries that are not part of a store path, counted.
    pub strays: u64,
    /// The first [`crate::gdfl_rekey::NAMED`] of them.
    pub strays_named: Vec<PathBuf>,
}

impl Walked {
    fn stray(&mut self, path: PathBuf) {
        self.strays += 1;
        if self.strays_named.len() < crate::gdfl_rekey::NAMED {
            self.strays_named.push(path);
        }
    }
}

/// The entries of `dir` in name order, each with whether it is a directory
/// (never through a link) or a link.
fn listed(dir: &Path) -> Result<Vec<(PathBuf, String, Listed)>, String> {
    let named = |why: std::io::Error| format!("cannot list {}: {why}", dir.display());
    // IN NAME ORDER, so two walks of one store list it alike (§3 rule 5): an
    // ordered map of one directory's entries, never on a request path.
    let mut out = BTreeMap::new();
    for entry in fs::read_dir(dir).map_err(named)? {
        let entry = entry.map_err(named)?;
        let kind = entry.file_type().map_err(named)?;
        let what = if kind.is_symlink() {
            Listed::Link
        } else if kind.is_dir() {
            Listed::Dir
        } else {
            Listed::Other
        };
        let _ = out.insert(
            entry.file_name().to_string_lossy().into_owned(),
            (entry.path(), what),
        );
    }
    Ok(out
        .into_iter()
        .map(|(name, (path, what))| (path, name, what))
        .collect())
}

/// What a listed entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Listed {
    Dir,
    Link,
    Other,
}

/// A `YYYY-MM` stem in the store's own spelling.
fn month_named(stem: &str) -> Option<YearMonth> {
    let (year, month) = stem.split_once('-')?;
    if year.len() != 4 || month.len() != 2 {
        return None;
    }
    let month = YearMonth::new(year.parse().ok()?, month.parse().ok()?).ok()?;
    (month.to_string() == stem).then_some(month)
}

/// Every month file under `bars/<vendor>/` the scope holds, in path order,
/// with the sidecars each has none of.
///
/// A link is never followed and counted; a name that is not part of a store
/// path is counted and the first ones named; a month file whose parts the
/// store cannot name is a stray. A month's sidecars are looked for beside the
/// generation a reader opens (`store::generation::served`): a repaired month
/// keeps its indexes beside its revision.
///
/// # Errors
///
/// A directory in scope that cannot be listed, or a month whose served
/// generation cannot be told.
pub fn months(root: &Path, vendor: Vendor, scope: &Scope) -> Result<Walked, String> {
    let mut walked = Walked::default();
    let base = root.join(store::path::STORE_ROOT).join(vendor.as_str());
    if !base.is_dir() {
        return Err(format!(
            "{} is not a directory: the store holds no {} bars",
            base.display(),
            vendor.as_str()
        ));
    }
    for (exchange_path, exchange, what) in listed(&base)? {
        if !scope.admits(0, &exchange) {
            continue;
        }
        if !into(
            &mut walked,
            what,
            &exchange_path,
            Exchange::parse(&exchange).is_ok(),
        ) {
            continue;
        }
        for (segment_path, segment, what) in listed(&exchange_path)? {
            if !scope.admits(1, &segment) {
                continue;
            }
            if !into(
                &mut walked,
                what,
                &segment_path,
                Segment::parse(&segment).is_ok(),
            ) {
                continue;
            }
            for (symbol_path, symbol, what) in listed(&segment_path)? {
                if !scope.admits(2, &symbol) {
                    continue;
                }
                let named = Symbol::new(&symbol).is_ok_and(|s| s.as_str() == symbol);
                if !into(&mut walked, what, &symbol_path, named) {
                    continue;
                }
                let at = Series {
                    root,
                    vendor,
                    exchange: &exchange,
                    segment: &segment,
                    symbol: &symbol,
                };
                walk_symbol(&at, &symbol_path, scope, &mut walked)?;
            }
        }
    }
    Ok(walked)
}

/// Whether the walk goes into a listed entry: a directory whose name is a
/// part; a link is counted, anything else a stray.
fn into(walked: &mut Walked, what: Listed, path: &Path, named: bool) -> bool {
    match (what, named) {
        (Listed::Dir, true) => true,
        (Listed::Link, _) => {
            walked.links += 1;
            false
        }
        (Listed::Dir | Listed::Other, _) => {
            walked.stray(path.to_path_buf());
            false
        }
    }
}

/// One series' place in the store.
struct Series<'a> {
    root: &'a Path,
    vendor: Vendor,
    exchange: &'a str,
    segment: &'a str,
    symbol: &'a str,
}

/// Everything below one symbol directory: rungs, or contracts then rungs.
fn walk_symbol(
    at: &Series<'_>,
    dir: &Path,
    scope: &Scope,
    walked: &mut Walked,
) -> Result<(), String> {
    for (path, name, what) in listed(dir)? {
        let rung = Timeframe::KNOWN
            .iter()
            .copied()
            .find(|rung| rung.as_str() == name);
        let contract = Contract::parse(&name).filter(|c| c.as_str() == name);
        match (rung, contract) {
            (Some(rung), _) => {
                // A rung directly under the symbol is the spot series; a
                // scope that names a contract does not hold it.
                if scope.prefix.len() <= 3 && into(walked, what, &path, true) {
                    walk_rung(at, None, rung, &path, scope, walked)?;
                }
            }
            (None, Some(contract)) => {
                if !scope.admits(3, &name) || !into(walked, what, &path, true) {
                    continue;
                }
                for (rung_path, rung_name, rung_what) in listed(&path)? {
                    let rung = Timeframe::KNOWN
                        .iter()
                        .copied()
                        .find(|rung| rung.as_str() == rung_name);
                    match rung {
                        Some(rung) if into(walked, rung_what, &rung_path, true) => {
                            walk_rung(at, Some(contract), rung, &rung_path, scope, walked)?;
                        }
                        Some(_) => {}
                        None => {
                            let _ = into(walked, rung_what, &rung_path, false);
                        }
                    }
                }
            }
            (None, None) => {
                let _ = into(walked, what, &path, false);
            }
        }
    }
    Ok(())
}

/// Every month file in one rung directory.
fn walk_rung(
    at: &Series<'_>,
    contract: Option<Contract>,
    rung: Timeframe,
    dir: &Path,
    scope: &Scope,
    walked: &mut Walked,
) -> Result<(), String> {
    for (path, name, what) in listed(dir)? {
        let Some(stem) = name.strip_suffix(FileKind::Bars.extension()) else {
            // A month's own siblings are expected here and are not strays.
            if what == Listed::Link {
                walked.links += 1;
            } else if !FileKind::ALL
                .iter()
                .any(|kind| name.ends_with(kind.extension()))
            {
                walked.stray(path);
            }
            continue;
        };
        let parts = month_named(stem).and_then(|month| {
            StorePath::new(PathParts {
                vendor: at.vendor,
                exchange: at.exchange,
                segment: at.segment,
                symbol: at.symbol,
                contract,
                timeframe: rung,
                month,
                file: FileKind::Bars,
            })
            .ok()
            .map(|store_path| (month, store_path))
        });
        let (true, Some((month, store_path))) = (what == Listed::Other, parts) else {
            let _ = into(walked, what, &path, false);
            continue;
        };
        if !scope.holds(rung, month) {
            continue;
        }
        let served = store::generation::served(at.root, store_path)
            .map_err(|why| format!("{}: {why}", path.display()))?
            .root(at.root);
        let lacks = Lacks::of(Kind::ALL.into_iter().filter(|kind| {
            kind.sidecar().is_some_and(|file| {
                fs::symlink_metadata(store_path.with_file(file).to_path_buf(&served))
                    .is_err_and(|why| why.kind() == ErrorKind::NotFound)
            })
        }));
        let key = path
            .strip_prefix(at.root)
            .map_or_else(|_| path.clone(), Path::to_path_buf)
            .to_string_lossy()
            .into_owned();
        walked.months.push(Month {
            exchange: at.exchange.to_owned(),
            segment: at.segment.to_owned(),
            symbol: at.symbol.to_owned(),
            contract,
            rung,
            month,
            key,
            lacks,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "upkeep_tests.rs"]
mod tests;
