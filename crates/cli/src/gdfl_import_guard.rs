//! The door of `cli gdfl-import` (AF-01, D-4977): what is asked before the
//! importer may write its first journal line, and what is said after it
//! returns. Nothing here reads or files a bar; `pull::gdfl_import` does, and
//! this module is not allowed to change it.
//!
//! # Before any day
//!
//! 1. **The source root is what its source needs** (`source_root_unknown`,
//!    FM-060). A tick store has `FORMAT.md` at its root; a directory of
//!    options archives holds at least one `<yyyy>.zip`; a capital-market
//!    archive is a readable zip holding at least one day zip of the run's
//!    tree. A typo in the variable, an empty staging folder or a second
//!    volume that has gone used to be read as "every day not held", exit 0.
//! 2. **One run of a kind at a time** (`run_busy`, FM-064's door half). The
//!    lock `telemetry::run_dirs::for_gdfl` names for the kind on the store's
//!    census layout (`imports/gdfl.<kind>.lock` on v4, `imports/gdfl.lock` on
//!    v3) is taken with `flock` without waiting, opened with `O_NOFOLLOW`
//!    (a link at the name is refused, never followed) and `O_NONBLOCK` (a
//!    pipe at the name cannot hold the open). The holder writes its record
//!    into it; a second run of the kind is refused naming that record. The
//!    kernel frees the lock of a killed holder, and the next run says the
//!    holder died: the record is still there, the lock is free.
//! 3. **What the last run of the same key read** (FM-290/FM-007's door half):
//!    the newest line of `imports/gdfl.sources` for the same kind, source,
//!    journal key and source path, read from the tail (at most
//!    [`TAIL_WINDOW`] bytes, never the whole file).
//!
//! # After the run
//!
//! - A run that owed days and read no instrument file is `no_file_read`,
//!   never OK.
//! - A zip source whose digest differs from the last run of the key, with days
//!   skipped unread, is `source_changed_since_last_run`. A run on a changed
//!   archive that skipped no day hides none and is not failed for it, but its
//!   line is written `source_changed_unchecked` and is never the baseline a
//!   later run compares with: a change is resolved only by restoring the
//!   archive the done days were read from, or by the accept and restate doors
//!   (IA-6, R3-22/R4-21), never by running some other range first. A tick
//!   store's identity is not comparable and says so.
//! - Every run, whatever its end, probes the store root and the source root
//!   once (two `stat` calls): a root that is gone is named `store_lost` or
//!   `source_lost` and the run is `FAILED`, instead of leaving only per-day
//!   "Permission denied" journal failures, or days closed "not in source"
//!   with exit 0 after the source's volume went mid-run (FM-086's door half).
//!   The probe is keyed on the root being absent, never on
//!   `ErrorKind::PermissionDenied`.
//! - The run's line is appended to `imports/gdfl.sources` (format:
//!   `docs/02-store-format.md` §41), the holder record is cleared and the
//!   lock released, each refusal named.
//!
//! # What it does not do
//!
//! The journal append lock, a run id chosen under a lock (FM-126) and the
//! census verbs taking the same lock (FM-058) are IA-2's. A per-file
//! comparison against `.prv`'s `source_crc32` is IA-6's (flag 7,
//! `source_changed_since_import`, test IA-GV-24), and landing a corrected day
//! as a new generation is R3-22's restate door (R4-21). The receipt names
//! both where they would apply.
//!
//! The process's own start time is not recorded: `std` exposes none and every
//! crate root forbids `unsafe`. The record carries the run's start (taken in
//! the same process before any day) and the moment the lock was taken. For
//! the same reason a contender cannot ask whether the pid a record names is
//! alive: in the moment between a new holder's `flock` and its record's
//! write, the record a contender reads can still be the dead holder's, and
//! `run_busy` says the record is the holder's "as written", never more.

use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, ErrorKind};
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::{FileExt as _, MetadataExt as _, OpenOptionsExt as _};
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::gdfl_archive::{day_zip_name, zip_entries};
use pull::gdfl_cm::{CmKind, folder_day};
use pull::gdfl_import::{ImportKind, Origin, Report, SourceKind};
use pull::ingest::census_store::{Location, locate};
use store::flock::Flock;
use store::open_flags::O_NOFOLLOW_NONBLOCK;
use telemetry::run_dirs::{self, GdflLayout};

/// Another run of the kind holds its lock.
pub(crate) const RUN_BUSY: &str = "run_busy";
/// The run lock could not be opened or taken: a link, a directory or a pipe
/// at its name, or the host's refusal.
pub(crate) const RUN_LOCK_REFUSED: &str = "run_lock_refused";
/// The run lock or its holder record could not be released.
pub(crate) const RUN_LOCK_UNRELEASED: &str = "run_lock_unreleased";
/// The source root is not what its source needs.
pub(crate) const SOURCE_ROOT_UNKNOWN: &str = "source_root_unknown";
/// The run owed days and read no instrument file.
pub(crate) const NO_FILE_READ: &str = "no_file_read";
/// The store root is gone. The word `pull::alerts::KINDS` gives kind 2.
pub(crate) const STORE_LOST: &str = "store_lost";
/// The source root is gone. The word `store::verdict::DayState::SourceLost`
/// writes.
pub(crate) const SOURCE_LOST: &str = "source_lost";
/// A zip source's digest changed since the key's last run, with days skipped.
pub(crate) const SOURCE_CHANGED: &str = "source_changed_since_last_run";
/// The outcome word of a run whose source differed from the key's baseline
/// (or whose baseline is behind a line of [`SOURCE_CHANGED`] older than the
/// window) but which skipped no day unread, so it hid none and is not failed
/// for it. Not a failure code: a word in the ledger's outcome field, so the
/// line is never the baseline a later run compares with.
pub(crate) const SOURCE_CHANGED_UNCHECKED: &str = "source_changed_unchecked";
/// `imports/gdfl.sources` could not be read or appended.
pub(crate) const SOURCES_LEDGER: &str = "sources_ledger_unwritten";

/// The ledger of what each run read, beneath `<store>/imports`.
pub(crate) const SOURCES: &str = "gdfl.sources";
/// The first field of every ledger line of format version 1.
pub(crate) const SOURCES_V1: &str = "v1";
/// The most bytes of the ledger's tail one run reads (1 MiB): about five
/// thousand lines of the usual length. A key whose last line is further back
/// is reported as having no earlier line in the window, by name.
pub(crate) const TAIL_WINDOW: u64 = 1 << 20;
/// The first word of a holder record of format version 1.
pub(crate) const HOLDER_V1: &str = "gdfl-lock v1";
/// The most bytes of a holder record that are read.
pub(crate) const RECORD_CAP: usize = 4096;
/// The most bytes of the verb line a holder record keeps.
pub(crate) const VERB_CAP: usize = 512;
/// A tick store's specification, which its root holds.
pub(crate) const FORMAT: &str = "FORMAT.md";
/// The telemetry target of every door event.
const TARGET: &str = "cli.gdfl_import.door";

/// What the door refused or found, by code and in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// The code: one of this module's constants.
    pub(crate) code: &'static str,
    /// What happened, naming paths and values.
    pub(crate) words: String,
}

impl Finding {
    /// A finding, logged once at `error` as it is made. Its words pass
    /// through [`plain`] here, the one place every refusal and finding of the
    /// door is made, so words it carries from elsewhere (an origin's fault,
    /// the census's refusal, a host error naming a path) cannot forge a
    /// receipt line or move the cursor (GD-11).
    pub(crate) fn new(code: &'static str, words: String) -> Self {
        let words = if words.chars().any(escaped_by_plain) {
            plain(&words)
        } else {
            words
        };
        let _noted =
            telemetry::emit(&telemetry::Event::error(TARGET, code).with("why", words.as_str()));
        Self { code, words }
    }
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.words)
    }
}

// ═══ 1. The source root ═════════════════════════════════════════════════════

/// What a source root must be for the source and kind of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// A tick store: a directory with `FORMAT.md` at its root.
    TickStore,
    /// A directory holding the vendor's `<yyyy>.zip` options archives.
    NfoYears,
    /// The capital-market outer zip, holding day zips of this tree.
    CmArchive(CmKind),
}

impl Shape {
    /// The shape a run of `kind` from `source` reads.
    pub(crate) const fn of(source: SourceKind, kind: ImportKind) -> Self {
        match (source, kind) {
            (SourceKind::TickStore, _) => Self::TickStore,
            (SourceKind::Zip, ImportKind::Options) => Self::NfoYears,
            (SourceKind::Zip, ImportKind::Stocks) => Self::CmArchive(CmKind::Stocks),
            (SourceKind::Zip, ImportKind::Indices) => Self::CmArchive(CmKind::Indices),
        }
    }

    /// What the shape is, in the words a refusal shows as "expected".
    const fn expected(self) -> &'static str {
        match self {
            Self::TickStore => "a GDFL tick store: a directory holding FORMAT.md at its root",
            Self::NfoYears => "a directory holding the vendor's yearly options zips, <yyyy>.zip",
            Self::CmArchive(CmKind::Indices) => {
                "the capital-market outer zip, holding day zips of the INDICES tree"
            }
            Self::CmArchive(CmKind::Stocks) => {
                "the capital-market outer zip, holding day zips of the STOCKS tree"
            }
        }
    }
}

/// Whether `name` is a member name the outer archive files as a day zip, and
/// of which tree: the rule `pull::gdfl_archive`'s day table applies, written
/// with that module's public parts (the name must be exactly the one
/// `day_zip_name` gives for the day its file names).
fn day_zip_tree(name: &str) -> Option<CmKind> {
    let file = name.rsplit_once('/')?.1;
    let (kind, day) = folder_day(file.strip_suffix(".zip")?).ok()?;
    (day_zip_name(kind, day) == name).then_some(kind)
}

/// Whether a directory entry's name is a year archive's, `<yyyy>.zip`.
fn year_zip(name: &std::ffi::OsStr) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 8
        && bytes
            .get(..4)
            .is_some_and(|year| year.iter().all(u8::is_ascii_digit))
        && bytes.ends_with(b".zip")
}

/// What is wrong with `root` for `shape`, or `None` when it is what the shape
/// needs. One metadata read for a tick store; one directory listing, stopped
/// at the first year archive, for the options archives; one read of the
/// central directory (the archive reader's own walk; `pull::gdfl_archive`
/// refuses one over its 64 MiB cap) for the outer zip. Measured costs:
/// `docs/06-limits.md` L-AF01-1 to L-AF01-4.
pub(crate) fn root_fault(shape: Shape, root: &Path) -> Option<String> {
    let found = match fs::metadata(root) {
        Err(why) if why.kind() == ErrorKind::NotFound => return Some("does not exist".to_owned()),
        Err(why) => return Some(format!("cannot be read ({why})")),
        Ok(found) => found,
    };
    match shape {
        Shape::TickStore | Shape::NfoYears if !found.is_dir() => {
            Some("is not a directory".to_owned())
        }
        Shape::TickStore => match fs::metadata(root.join(FORMAT)) {
            Ok(format) if format.is_file() => None,
            Ok(_) => Some(format!("its {FORMAT} is not a file")),
            Err(why) if why.kind() == ErrorKind::NotFound => {
                Some(format!("holds no {FORMAT} at its root"))
            }
            Err(why) => Some(format!("its {FORMAT} cannot be read ({why})")),
        },
        Shape::NfoYears => {
            let entries = match fs::read_dir(root) {
                Ok(entries) => entries,
                Err(why) => return Some(format!("cannot be listed ({why})")),
            };
            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(why) => return Some(format!("cannot be listed ({why})")),
                };
                if year_zip(&entry.file_name())
                    && fs::metadata(entry.path()).is_ok_and(|m| m.is_file())
                {
                    return None;
                }
            }
            Some("holds no <yyyy>.zip".to_owned())
        }
        Shape::CmArchive(_) if !found.is_file() => Some("is not a file".to_owned()),
        Shape::CmArchive(tree) => {
            let file = match File::open(root) {
                Ok(file) => file,
                Err(why) => return Some(format!("cannot be opened ({why})")),
            };
            match zip_entries(&file, 0, found.len()) {
                Err(why) => Some(format!("is not a readable zip ({why})")),
                Ok(entries) if entries.iter().any(|e| day_zip_tree(&e.name) == Some(tree)) => None,
                Ok(entries) => Some(format!(
                    "holds no day zip of its tree ({} members, none of them a name \
                     pull::gdfl_archive::day_zip_name gives)",
                    entries.len()
                )),
            }
        }
    }
}

/// The source root, refused with [`SOURCE_ROOT_UNKNOWN`] naming the
/// variable, the path, what was found and what was expected, and the origin's
/// own fault when it measured one.
///
/// # Errors
///
/// [`SOURCE_ROOT_UNKNOWN`].
pub(crate) fn source_root(
    shape: Shape,
    var: &str,
    root: &Path,
    origin: &Origin,
) -> Result<(), Finding> {
    let Some(found) = root_fault(shape, root) else {
        return Ok(());
    };
    let mut words = format!(
        "{var}={} {found}; expected {}",
        shown(root),
        shape.expected()
    );
    if let Some(fault) = &origin.fault {
        let _ = write!(words, "; the origin measured: {fault}");
    }
    Err(Finding::new(SOURCE_ROOT_UNKNOWN, words))
}

// ═══ 2. The run lock ════════════════════════════════════════════════════════

/// Who takes the run lock, as its record names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Holder {
    /// This process.
    pub(crate) pid: u32,
    /// When the run started, UNIX seconds (`Origin::started`).
    pub(crate) run_started: u64,
    /// The kind the run imports.
    pub(crate) kind: ImportKind,
    /// The verb line, escaped and cut to [`VERB_CAP`] bytes.
    pub(crate) verb: String,
}

impl Holder {
    /// The holder record, one line, taken at `lock_taken` (UNIX
    /// microseconds) on `layout` (`docs/02-store-format.md` §41).
    pub(crate) fn record(&self, layout: GdflLayout, lock_taken: u64) -> String {
        format!(
            "{HOLDER_V1} pid={} run_started={} lock_taken_us={lock_taken} kind={} layout={} verb={}\n",
            self.pid,
            self.run_started,
            self.kind.as_str(),
            layout_word(layout),
            self.verb
        )
    }
}

/// The verb line as a holder record keeps it: escaped, and cut to
/// [`VERB_CAP`] bytes with the cut said.
pub(crate) fn verb_of(args: &[&str]) -> String {
    let mut line = String::from("gdfl-import");
    for word in args {
        line.push(' ');
        line.push_str(word);
    }
    let mut escaped = escape(line.as_bytes());
    if escaped.len() > VERB_CAP {
        let mut cut = VERB_CAP;
        while !escaped.is_char_boundary(cut) {
            cut -= 1;
        }
        let rest = escaped.len() - cut;
        escaped.truncate(cut);
        let _ = write!(escaped, "...(+{rest} bytes)");
    }
    escaped
}

/// `v3` or `v4`.
const fn layout_word(layout: GdflLayout) -> &'static str {
    match layout {
        GdflLayout::V3 => "v3",
        GdflLayout::V4 => "v4",
    }
}

/// UNIX microseconds now; 0 for a clock before 1970, which names itself in
/// the record rather than stopping the run.
fn now_micros() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_micros()).unwrap_or(u64::MAX))
}

/// The census layout of `store`, which decides the lock: the census's own
/// answer (`pull::ingest::census_store::locate`), never a guess.
///
/// # Errors
///
/// [`STORE_LOST`] for a store root that is gone, else [`RUN_LOCK_REFUSED`]
/// with the census's words.
pub(crate) fn layout_of(store: &Path) -> Result<GdflLayout, Finding> {
    match locate(store, Vendor::Gdfl) {
        Ok(Location::Single(_)) => Ok(GdflLayout::V3),
        Ok(Location::Partitioned(_)) => Ok(GdflLayout::V4),
        Err(why) => Err(lost_store(store).unwrap_or_else(|| {
            Finding::new(
                RUN_LOCK_REFUSED,
                format!("the census layout that decides the run lock cannot be read: {why}"),
            )
        })),
    }
}

/// What is at `path`, in a word, for a refusal: never followed.
fn what_is_at(path: &Path) -> String {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => "a symbolic link (never followed)".to_owned(),
        Ok(m) if m.is_dir() => "a directory".to_owned(),
        Ok(m) if m.is_file() => "a regular file".to_owned(),
        Ok(_) => "a special file (a pipe, a socket or a device)".to_owned(),
        Err(why) => format!("unmeasurable ({why})"),
    }
}

/// Reads at most [`RECORD_CAP`] bytes from the start of `file`, as text
/// [`escape`]d: the bytes at the lock's name are whatever was left there (a
/// record of another format, random bytes, a terminal control sequence), and
/// the receipt prints them as one line that can forge none.
fn record_of(file: &File) -> io::Result<String> {
    let mut buf = vec![0; RECORD_CAP];
    let mut filled = 0;
    while let Some(rest) = buf.get_mut(filled..).filter(|rest| !rest.is_empty()) {
        let n = file.read_at(rest, filled as u64)?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    buf.truncate(filled);
    let kept = buf.trim_ascii_end();
    Ok(escape(kept))
}

/// The named field of a holder record, or `?`.
fn field<'a>(record: &'a str, name: &str) -> &'a str {
    record
        .split(' ')
        .find_map(|word| word.strip_prefix(name)?.strip_prefix('='))
        .unwrap_or("?")
}

/// The run lock this process holds, and what it found when it took it.
#[derive(Debug)]
pub(crate) struct RunLock {
    /// The held lock; `None` once released.
    flock: Option<Flock<File>>,
    /// The lock's name beneath the store, `imports/gdfl.<kind>.lock` or
    /// `imports/gdfl.lock`.
    pub(crate) name: &'static str,
    /// The census layout that chose it.
    pub(crate) layout: GdflLayout,
    /// The record a holder left without releasing: it died (the kernel freed
    /// its lock, the record stayed).
    pub(crate) dead: Option<String>,
}

impl RunLock {
    /// Takes the lock of `holder.kind` on `store`'s layout without waiting,
    /// and writes the holder's record into it.
    ///
    /// # Errors
    ///
    /// [`RUN_BUSY`] naming the holder's pid and start, [`STORE_LOST`] for a
    /// store root that is gone, and [`RUN_LOCK_REFUSED`] for a link, a
    /// directory or a special file at the lock's name, or the host's refusal.
    pub(crate) fn take(store: &Path, holder: &Holder) -> Result<Self, Finding> {
        let layout = layout_of(store)?;
        let dir = run_dirs::for_gdfl(holder.kind.as_str(), layout).ok_or_else(|| {
            Finding::new(
                RUN_LOCK_REFUSED,
                format!(
                    "{} has no run lock in telemetry::run_dirs",
                    holder.kind.as_str()
                ),
            )
        })?;
        let path = store.join(dir.lock);
        let refused = |what: String| {
            lost_store(store).unwrap_or_else(|| {
                Finding::new(RUN_LOCK_REFUSED, format!("{}: {what}", shown(&path)))
            })
        };
        // `imports/` alone is made, never the store root above it: a root
        // whose volume went is `store_lost`, never born again empty on the
        // mount point's parent.
        if let Some(parent) = path.parent() {
            match fs::create_dir(parent) {
                Ok(()) => {}
                Err(why) if why.kind() == ErrorKind::AlreadyExists => {}
                Err(why) => return Err(refused(format!("its folder: {why}"))),
            }
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o644)
            .custom_flags(O_NOFOLLOW_NONBLOCK)
            .open(&path)
            .map_err(|why| {
                refused(if fs::symlink_metadata(&path).is_err() {
                    format!("cannot be created ({why})")
                } else {
                    format!("is {} and cannot be opened ({why})", what_is_at(&path))
                })
            })?;
        let found = file.metadata().map_err(|why| refused(why.to_string()))?;
        if !found.is_file() {
            return Err(refused(format!(
                "is {}, not a regular file",
                what_is_at(&path)
            )));
        }
        // A second name of the lock file (a hard link) is refused before a
        // byte is written: the record's `set_len(0)` and write would truncate
        // and overwrite whatever file the other name is (X1).
        if found.nlink() != 1 {
            return Err(refused(format!(
                "has {} names (a hard link to another file); never truncated or written \
                 through, so that file is left as it was",
                found.nlink()
            )));
        }
        let reader = file.try_clone().map_err(|why| refused(why.to_string()))?;
        let flock = match Flock::try_lock(file, path.clone()) {
            Ok(flock) => flock,
            Err(TryLockError::WouldBlock) => {
                let record = record_of(&reader).unwrap_or_else(|why| format!("unreadable ({why})"));
                // The holder's kind is the one its record names: on v3 one
                // lock serialises every kind, so it need not be this run's.
                let words = if record.is_empty() {
                    format!(
                        "another run holds {}; its holder has not written its record yet",
                        dir.lock
                    )
                } else {
                    format!(
                        "another {} run holds {}: pid {} started {} (UNIX s), as its record was \
                         last written (a holder that took the lock from a dead one rewrites it \
                         within the same moment); record: {record}",
                        field(&record, "kind"),
                        dir.lock,
                        field(&record, "pid"),
                        field(&record, "run_started"),
                    )
                };
                return Err(Finding::new(RUN_BUSY, words));
            }
            Err(TryLockError::Error(why)) => return Err(refused(format!("flock: {why}"))),
        };
        drop(reader);
        let dead = record_of(&flock).map_err(|why| refused(why.to_string()))?;
        let dead = (!dead.is_empty()).then_some(dead);
        if let Some(record) = &dead {
            let _noted = telemetry::emit(
                &telemetry::Event::warn(TARGET, "previous holder died")
                    .with("lock", dir.lock)
                    .with("record", record.as_str()),
            );
        }
        let record = holder.record(layout, now_micros());
        flock
            .set_len(0)
            .and_then(|()| flock.write_all_at(record.as_bytes(), 0))
            .and_then(|()| flock.sync_data())
            .map_err(|why| refused(format!("the holder record: {why}")))?;
        Ok(Self {
            flock: Some(flock),
            name: dir.lock,
            layout,
            dead,
        })
    }

    /// Clears the record and releases the lock, saying so if either fails.
    ///
    /// # Errors
    ///
    /// [`RUN_LOCK_UNRELEASED`], naming the lock and the host's refusal.
    pub(crate) fn release(mut self) -> Result<(), Finding> {
        let Some(flock) = self.flock.take() else {
            return Ok(());
        };
        let cleared = flock.set_len(0);
        let released = flock.release();
        match (cleared, released) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(why), _) => Err(Finding::new(
                RUN_LOCK_UNRELEASED,
                format!(
                    "{}: the holder record was not cleared ({why}); the next run will read this run as died",
                    self.name
                ),
            )),
            (Ok(()), Err(why)) => Err(Finding::new(RUN_LOCK_UNRELEASED, why.to_string())),
        }
    }
}

impl Drop for RunLock {
    /// A run that returns without [`RunLock::release`] (an early refusal)
    /// still clears its record: only a holder that died leaves one. The
    /// lock itself is unlocked by `Flock`'s own drop.
    fn drop(&mut self) {
        if let Some(flock) = &self.flock {
            let _cleared = flock.set_len(0);
        }
    }
}

// ═══ 3. The sources ledger ══════════════════════════════════════════════════

/// The ledger's path beneath a store root.
pub(crate) fn sources_path(store: &Path) -> PathBuf {
    store.join("imports").join(SOURCES)
}

/// `bytes` with `\`, tab, newline, carriage return and every byte that is not
/// printable ASCII escaped, so a path or key is one tab-free field.
pub(crate) fn escape(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'\t' => out.push_str("\\t"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            0x20..=0x7e => out.push(char::from(b)),
            _ => {
                let _ = write!(out, "\\x{b:02x}");
            }
        }
    }
    out
}

/// A path as the receipt and the refusals show it: [`escape`]d, so a name
/// holding a newline, a tab or a terminal control byte cannot forge a line of
/// the receipt or move the operator's cursor.
pub(crate) fn shown(path: &Path) -> String {
    escape(path.as_os_str().as_bytes())
}

/// Words the door did not write itself (a refusal of another crate, an
/// origin's fault, an error that carries a path through `display()`) as one
/// line of the receipt that can forge none (GD-11): every control character
/// (C0, DEL and C1, so a newline, a carriage return and ESC included), every
/// bidirectional override and the Unicode line and paragraph separators are
/// escaped (`\n`, `\t`, `\r`, `\xNN`, `\u{NNNN}`); every other character,
/// `\` included, is kept. Keeping `\` makes it idempotent, so words already
/// carrying a [`shown`] path pass through unchanged and every refusal can go
/// through it at the one place it is made ([`Finding::new`]) and again where
/// it is printed. A path is still printed by [`shown`], which also escapes
/// `\` and every non-ASCII byte and so is the exact spelling; in words that
/// passed only through this a literal `\n` and an escaped newline read
/// alike, which is an ambiguity, never a forged line or a moved cursor.
pub(crate) fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_ascii_control() => {
                let _ = write!(out, "\\x{:02x}", u32::from(c));
            }
            c if c.is_control() || unprintable_format(c) => {
                let _ = write!(out, "\\u{{{:04x}}}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out
}

/// Whether [`plain`] changes `c`: a control character, or one of
/// [`unprintable_format`].
fn escaped_by_plain(c: char) -> bool {
    c.is_control() || unprintable_format(c)
}

/// A character that is not a control but moves or reorders what a terminal
/// or a page shows: the bidirectional marks, embeddings, overrides and
/// isolates, and the line and paragraph separators some viewers break on.
const fn unprintable_format(c: char) -> bool {
    matches!(
        c,
        '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
    )
}

/// The journal key's filter part for `only`, as `pull::gdfl_import` writes
/// it: the sorted, deduplicated names joined by `,`, or `*`.
pub(crate) fn filter_key(only: &[String]) -> String {
    if only.is_empty() {
        return "*".to_owned();
    }
    let names: std::collections::BTreeSet<&str> = only.iter().map(String::as_str).collect();
    names.into_iter().collect::<Vec<_>>().join(",")
}

/// A digest as lowercase hex.
pub(crate) fn hex(digest: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for b in digest {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// One line of `imports/gdfl.sources` v1 (`docs/02-store-format.md` §41).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    /// When the run started, UNIX seconds.
    pub(crate) started: u64,
    /// The import kind's word.
    pub(crate) kind: String,
    /// `tickstore` or `zip`.
    pub(crate) source: String,
    /// The origin's digest in hex, or `unmeasured`.
    pub(crate) digest: String,
    /// `ok`, or the codes and the run's own end joined by `+`.
    pub(crate) outcome: String,
    /// Days skipped unread as already done.
    pub(crate) days_skipped: usize,
    /// Instrument files read.
    pub(crate) files: usize,
    /// The journal key's filter part, escaped.
    pub(crate) key: String,
    /// The source path, escaped.
    pub(crate) path: String,
}

/// The fields a ledger line is matched on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Key<'a> {
    /// The kind's word.
    pub(crate) kind: &'a str,
    /// The source's word.
    pub(crate) source: &'a str,
    /// The filter key, escaped.
    pub(crate) filter: &'a str,
    /// The source path, escaped.
    pub(crate) path: &'a str,
}

/// The digest written for an origin that could not be measured.
pub(crate) const UNMEASURED: &str = "unmeasured";

impl Entry {
    /// The line, ending in a newline.
    pub(crate) fn line(&self) -> String {
        format!(
            "{SOURCES_V1}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            self.started,
            self.kind,
            self.source,
            self.digest,
            self.outcome,
            self.days_skipped,
            self.files,
            self.key,
            self.path
        )
    }

    /// A line without its newline, or `None` when it is not one of v1.
    pub(crate) fn parse(line: &str) -> Option<Self> {
        let mut f = line.split('\t');
        if f.next()? != SOURCES_V1 {
            return None;
        }
        let entry = Self {
            started: f.next()?.parse().ok()?,
            kind: f.next()?.to_owned(),
            source: f.next()?.to_owned(),
            digest: f.next()?.to_owned(),
            outcome: f.next()?.to_owned(),
            days_skipped: f.next()?.parse().ok()?,
            files: f.next()?.parse().ok()?,
            key: f.next()?.to_owned(),
            path: f.next()?.to_owned(),
        };
        let digest_ok = entry.digest == UNMEASURED
            || (entry.digest.len() == 64 && entry.digest.bytes().all(|b| b.is_ascii_hexdigit()));
        (f.next().is_none() && digest_ok).then_some(entry)
    }

    /// Whether this line is a run of `key`.
    fn is(&self, key: Key<'_>) -> bool {
        self.kind == key.kind
            && self.source == key.source
            && self.key == key.filter
            && self.path == key.path
    }
}

/// What the tail of the ledger says of a key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Tail {
    /// The ledger does not exist yet.
    pub(crate) absent: bool,
    /// The newest line of the key with a measured digest that did not name an
    /// unresolved change: the baseline.
    pub(crate) last: Option<Entry>,
    /// The bytes after the last newline: a line torn by a killed append,
    /// skipped for comparison.
    pub(crate) torn: usize,
    /// Lines in the window that are not v1 lines, skipped.
    pub(crate) unreadable: usize,
    /// The window began inside the file: older lines were not read.
    pub(crate) cut: bool,
    /// Newer lines of the key that named a change they could not resolve
    /// ([`SOURCE_CHANGED`]), passed over: the change they named is still
    /// unresolved, so they are never the baseline a later run compares with.
    pub(crate) unresolved: usize,
}

/// Opens the ledger to append, never through a link and never blocking on a
/// pipe, refusing anything but a regular file. It is opened to read as well,
/// for the one byte [`ends_torn`] reads from this same file at the append (a
/// file opened to append alone refuses that read with `EBADF`); the tail the
/// door compares with is read through the store's one reading door,
/// `store::safe_open` (R3-16), in [`tail_for`].
fn open_ledger(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .mode(0o644)
        .custom_flags(O_NOFOLLOW_NONBLOCK)
        .open(path)?;
    let found = file.metadata()?;
    if !found.is_file() {
        return Err(io::Error::other(format!(
            "{} is {}, not a regular file",
            shown(path),
            what_is_at(path)
        )));
    }
    // A second name (a hard link) would carry the run's line into whatever
    // file it is, as the lock's would (GD-10): refused before a byte.
    if found.nlink() != 1 {
        return Err(io::Error::other(format!(
            "{} has {} names (a hard link to another file); no line is written through it",
            shown(path),
            found.nlink()
        )));
    }
    Ok(file)
}

/// The tail of `ledger` as it bears on `key`: at most [`TAIL_WINDOW`] bytes
/// read once from the end, lines scanned newest first. Opened through
/// `store::safe_open::open_read_len` (R3-16, D-4951): a link at the name is
/// never followed, a pipe never waited on, and the length comes from the same
/// `fstat` that proved the name a regular file.
///
/// # Errors
///
/// The store door's refusal (a link or a special file at the ledger's name
/// included, said as what it is), or the host's refusal to read it.
pub(crate) fn tail_for(ledger: &Path, key: Key<'_>) -> Result<Tail, String> {
    let (file, len) = match store::safe_open::open_read_len(ledger) {
        Ok(opened) => opened,
        Err(why) if why.is_missing() => {
            return Ok(Tail {
                absent: true,
                ..Tail::default()
            });
        }
        Err(why) => return Err(escape(why.to_string().as_bytes())),
    };
    let start = len.saturating_sub(TAIL_WINDOW);
    let mut buf = vec![0; usize::try_from(len - start).unwrap_or(0)];
    file.read_exact_at(&mut buf, start)
        .map_err(|why| format!("{}: {why}", shown(ledger)))?;
    Ok(scan(&buf, start > 0, key))
}

/// The scan of one window: `cut` when it began inside the file.
pub(crate) fn scan(window: &[u8], cut: bool, key: Key<'_>) -> Tail {
    let mut tail = Tail {
        cut,
        ..Tail::default()
    };
    let mut body = window;
    if cut {
        // The first line began before the window. A window with no newline
        // at all is the tail of one line longer than the window, torn or
        // never ended: the whole window is that torn line, named as one
        // (never read as an empty, clean tail), and the next append starts
        // a line of its own after it.
        let Some(first) = body.iter().position(|&b| b == b'\n') else {
            tail.torn = body.len();
            return tail;
        };
        body = body.get(first + 1..).unwrap_or(&[]);
    }
    let whole = body
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |at| at + 1);
    tail.torn = body.len() - whole;
    for line in body
        .get(..whole)
        .unwrap_or(&[])
        .split(|&b| b == b'\n')
        .rev()
    {
        if line.is_empty() {
            continue;
        }
        match std::str::from_utf8(line).ok().and_then(Entry::parse) {
            None => tail.unreadable += 1,
            Some(entry) if entry.is(key) && entry.digest != UNMEASURED => {
                // A line that named a change, or that ran on a changed archive
                // and skipped no day so checked none, is not a baseline: a
                // later run on the same changed archive compares with the
                // line before the change, so the change stays named until the
                // archive the done days came from is back or the accept and
                // restate doors land them; never accepted by running again,
                // over this range or any other.
                if entry
                    .outcome
                    .split('+')
                    .any(|word| word == SOURCE_CHANGED || word == SOURCE_CHANGED_UNCHECKED)
                {
                    tail.unresolved += 1;
                    continue;
                }
                tail.last = Some(entry);
                break;
            }
            Some(_) => {}
        }
    }
    tail
}

/// Writes `line` as one write, after a newline when the ledger's last line is
/// torn, so the torn bytes stay a line of their own.
///
/// # Errors
///
/// The writer's refusal.
pub(crate) fn append_to(out: &mut dyn io::Write, torn: bool, line: &str) -> io::Result<()> {
    let mut bytes = Vec::with_capacity(line.len() + 1);
    if torn {
        bytes.push(b'\n');
    }
    bytes.extend_from_slice(line.as_bytes());
    out.write_all(&bytes)?;
    out.flush()
}

/// [`SOURCES_LEDGER`] for a refused append, naming a full disk as one.
pub(crate) fn unwritten(ledger: &Path, why: &io::Error) -> Finding {
    let disk = if why.kind() == ErrorKind::StorageFull {
        "the disk is full: "
    } else {
        ""
    };
    Finding::new(SOURCES_LEDGER, format!("{}: {disk}{why}", shown(ledger)))
}

/// Whether the file's last byte ends a line: `false` for an empty file.
/// Read once, at the append, from the open file itself: the open-time tail
/// is stale by then, because a run of another kind (v4 runs kinds side by
/// side, each under its own lock, all appending to the one ledger) or a
/// killed append can have changed the end since.
fn ends_torn(file: &File, len: u64) -> io::Result<bool> {
    if len == 0 {
        return Ok(false);
    }
    let mut last = [0];
    file.read_exact_at(&mut last, len - 1)?;
    Ok(last != *b"\n")
}

/// Appends `line` to `ledger` and makes it durable, after a newline when the
/// ledger's last byte, read at the append ([`ends_torn`]), does not end a
/// line; a ledger that was empty at the append (this append created it, or
/// it held nothing) has its directory entry made durable too, so the first
/// run's line survives a power loss with its name.
///
/// # Errors
///
/// [`SOURCES_LEDGER`], naming a full disk as one.
pub(crate) fn append(ledger: &Path, line: &str) -> Result<(), Finding> {
    let unwritten = |why: io::Error| unwritten(ledger, &why);
    let mut file = open_ledger(ledger).map_err(unwritten)?;
    let len = file.metadata().map_err(unwritten)?.len();
    let torn = ends_torn(&file, len).map_err(unwritten)?;
    append_to(&mut file, torn, line).map_err(unwritten)?;
    file.sync_data().map_err(unwritten)?;
    if len == 0 {
        sync_parent(ledger).map_err(unwritten)?;
    }
    Ok(())
}

/// Makes the directory entry of `path` durable: one `fsync` of its parent.
fn sync_parent(path: &Path) -> io::Result<()> {
    match path.parent() {
        Some(dir) => File::open(dir)?.sync_all(),
        None => Ok(()),
    }
}

// ═══ 4. What the run's source is, against the last run of its key ══════════

/// The run's source identity against the key's last line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Identity {
    /// A tick store: its digest is `FORMAT.md`'s.
    NotComparable,
    /// The origin could not be measured.
    Unmeasured,
    /// No earlier line of the key in the window.
    First,
    /// The same digest as the key's last run.
    Same {
        /// When that run started.
        since: u64,
    },
    /// A different digest from the key's last run.
    Changed {
        /// That run's digest.
        old: String,
        /// This run's.
        new: String,
        /// When that run started.
        since: u64,
        /// Later runs of the key that already named a change and left it
        /// unresolved ([`Tail::unresolved`]).
        unresolved: usize,
    },
    /// Every line of the key in the window named an unresolved change, and
    /// the line they were compared with is older than the window.
    Unresolved {
        /// Those lines.
        unresolved: usize,
    },
}

/// The identity of a run reading `source` measured as `digest`, against
/// `tail`.
pub(crate) fn identity(source: SourceKind, digest: Option<&[u8; 32]>, tail: &Tail) -> Identity {
    if source == SourceKind::TickStore {
        return Identity::NotComparable;
    }
    let Some(digest) = digest else {
        return Identity::Unmeasured;
    };
    let new = hex(digest);
    match &tail.last {
        None if tail.unresolved > 0 => Identity::Unresolved {
            unresolved: tail.unresolved,
        },
        None => Identity::First,
        Some(last) if last.digest == new => Identity::Same {
            since: last.started,
        },
        Some(last) => Identity::Changed {
            old: last.digest.clone(),
            new,
            since: last.started,
            unresolved: tail.unresolved,
        },
    }
}

/// The words the receipt shows for a tick store's identity.
pub(crate) const NOT_COMPARABLE: &str = "source identity not comparable: the tick-store digest is \
     FORMAT.md's and the .bts writer skips existing outputs (FM-007)";

// ═══ 5. The end of a run ════════════════════════════════════════════════════

/// [`STORE_LOST`] when the store root is gone.
fn lost_store(store: &Path) -> Option<Finding> {
    gone(store).then(|| {
        Finding::new(
            STORE_LOST,
            format!(
                "the store root {} is gone: its volume may have been unplugged; a per-day \
                 journal or durability failure after it is its consequence, not its cause",
                shown(store)
            ),
        )
    })
}

/// Whether `path` is absent: `NotFound`, and only that (a refused look is
/// not a lost root, FM-086's `volume_access_revoked` is another code).
fn gone(path: &Path) -> bool {
    matches!(fs::metadata(path), Err(why) if why.kind() == ErrorKind::NotFound)
}

/// The roots probed once at the end of every run: each one that is gone,
/// named.
pub(crate) fn probe_roots(store: &Path, source: &Path) -> Vec<Finding> {
    let mut lost: Vec<Finding> = lost_store(store).into_iter().collect();
    if gone(source) {
        lost.push(Finding::new(
            SOURCE_LOST,
            format!(
                "the source root {} is gone: its volume may have been unplugged or the folder \
                 moved while the run read it",
                shown(source)
            ),
        ));
    }
    lost
}

/// The days of a run whose done data were not read again: the days skipped
/// as already done, and the done days whose listing alone was read again for
/// a wider scope (`Report::days_widened`, WP-12f, D-4878), whose members the
/// earlier close covered were not re-read either. Either kind of day can
/// hide a changed archive, so both count as skipped unread.
pub(crate) const fn unread(report: &Report) -> usize {
    report.days_skipped + report.days_widened
}

/// [`NO_FILE_READ`] for a run that read no instrument file and skipped done
/// days unread when the last line of its key ([`Tail::last`]) ended
/// [`NO_FILE_READ`] too (verify-AF-01-r2 follow-up). The importer closes a
/// held day whose files the filter did not want as done with no file, so the
/// identical next run skips that day and owes nothing: without this it
/// exited `OK` and wrote `ok`, and the run that read nothing was accepted by
/// running it again. The line this run writes ends [`NO_FILE_READ`] as well,
/// so it stays named until a run of the key reads a file. A run of the key
/// over other, properly done days after such a line is named too: the ledger
/// line holds no range, so the door cannot tell those days apart, and it
/// says so rather than pass a run that read nothing.
pub(crate) fn nothing_read_again(report: &Report, tail: &Tail) -> Option<Finding> {
    if report.files > 0 || unread(report) == 0 {
        return None;
    }
    let last = tail.last.as_ref()?;
    if !last.outcome.split('+').any(|word| word == NO_FILE_READ) {
        return None;
    }
    Some(Finding::new(
        NO_FILE_READ,
        format!(
            "the run read no instrument file and skipped {} days as already done, and the last \
             run of this key (started {} UNIX s) ended {NO_FILE_READ}: a day that run closed \
             without reading a file is done in the journal, so running again reads nothing and \
             is not OK. The filter matched no file in those days (a renamed or absent symbol) \
             or the source holds none of its files; the ledger line holds no range, so days \
             done by an earlier run of this key are not told apart from them",
            unread(report),
            last.started
        ),
    ))
}

/// What the door finds of a run that returned: `report` when it ran to its
/// end. The roots are probed at the end of EVERY run, whatever it found and
/// however it ended (two `stat` calls, docs/06 L-AF01-3): a source whose
/// volume went after some files were read leaves its remaining days closed
/// "not in source" with no failure, which nothing else names.
pub(crate) fn verdict(
    report: Option<&Report>,
    identity: &Identity,
    roots: (&Path, &Path),
) -> Vec<Finding> {
    let mut found = Vec::new();
    if let Some(report) = report {
        let owed = report.days_imported + report.days_missing.len();
        if report.files == 0 && owed > 0 {
            found.push(Finding::new(
                NO_FILE_READ,
                format!(
                    "the run owed {owed} days ({} imported, {} not in the source) and read no \
                     instrument file ({} not wanted); a run that reads nothing is never OK",
                    report.days_imported,
                    report.days_missing.len(),
                    report.files_skipped
                ),
            ));
        }
        if let Identity::Changed {
            old,
            new,
            since,
            unresolved,
        } = identity
            && unread(report) > 0
        {
            let again = if *unresolved > 0 {
                format!(
                    " {unresolved} later runs of this key named this change already; it stays \
                     named until the skipped days are landed, never accepted by running again, \
                     over this range or any other."
                )
            } else {
                String::new()
            };
            found.push(Finding::new(
                SOURCE_CHANGED,
                format!(
                    "the archive's digest was {old} at the run of {since} (UNIX s) and is {new} \
                     now; {} days were skipped unread as already done.{again} The digest covers \
                     every member's size and CRC-32, so which skipped days changed is not known \
                     here: comparing each day's files against .prv's source_crc32 is IA-6's \
                     source_changed_since_import (IA-GV-24) and landing a corrected day as a new \
                     generation is R3-22's restate door (R4-21): neither is built",
                    unread(report)
                ),
            ));
        }
        if let Identity::Unresolved { unresolved } = identity
            && unread(report) > 0
        {
            found.push(Finding::new(
                SOURCE_CHANGED,
                format!(
                    "{unresolved} runs of this key in the ledger's last {TAIL_WINDOW} bytes named \
                     a changed archive and none resolved it; the digest they were compared with \
                     is older than that window. {} days were skipped unread as already done",
                    unread(report)
                ),
            ));
        }
    }
    found.extend(probe_roots(roots.0, roots.1));
    found
}

/// Whether a run's line must carry [`SOURCE_CHANGED_UNCHECKED`]: its source
/// differed from the key's baseline (or the baseline is behind unresolved
/// lines older than the window), and no [`SOURCE_CHANGED`] was found because
/// it skipped no day unread. Such a line is never a baseline.
pub(crate) fn unchecked(identity: &Identity, found: &[Finding]) -> bool {
    matches!(
        identity,
        Identity::Changed { .. } | Identity::Unresolved { .. }
    ) && !found.iter().any(|f| f.code == SOURCE_CHANGED)
}

/// The ledger's outcome word for a run: `ok`, or every code, then
/// [`SOURCE_CHANGED_UNCHECKED`] when `unchecked`, then the run's own end,
/// joined by `+`.
pub(crate) fn outcome(found: &[Finding], unchecked: bool, end: Option<&str>) -> String {
    let words: Vec<&str> = found
        .iter()
        .map(|f| f.code)
        .chain(unchecked.then_some(SOURCE_CHANGED_UNCHECKED))
        .chain(end)
        .collect();
    if words.is_empty() {
        "ok".to_owned()
    } else {
        words.join("+")
    }
}

/// The source path a ledger key names: the root resolved once by the host
/// (`realpath`), so one archive reached through a link, a doubled slash or a
/// `..` is one key and is compared with its own last run (verify-AF-01-r1
/// X7). Resolved after the root check, so a failure here means the root went
/// in between, or a link in its chain cannot be read.
///
/// # Errors
///
/// [`SOURCE_LOST`] for a root that is gone, else [`SOURCE_ROOT_UNKNOWN`]
/// naming the variable, the path and the host's refusal.
pub(crate) fn canonical_source(var: &str, path: &Path) -> Result<PathBuf, Finding> {
    fs::canonicalize(path).map_err(|why| {
        if gone(path) {
            Finding::new(
                SOURCE_LOST,
                format!(
                    "the source root {} went between its check and the run lock: {why}",
                    shown(path)
                ),
            )
        } else {
            Finding::new(
                SOURCE_ROOT_UNKNOWN,
                format!(
                    "{var}={} cannot be resolved to one path for the sources ledger's key: {why}",
                    shown(path)
                ),
            )
        }
    })
}

/// The door of one run: its source root checked, its lock held, and the tail
/// of its key read.
#[derive(Debug)]
pub(crate) struct Door {
    lock: RunLock,
    ledger: PathBuf,
    tail: Tail,
    identity: Identity,
    entry: Entry,
    roots: (PathBuf, PathBuf),
    fault: Option<String>,
}

/// What a door is opened with.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Opening<'a> {
    /// The run's kind.
    pub(crate) kind: ImportKind,
    /// The variable that named the source root.
    pub(crate) var: &'a str,
    /// The store root.
    pub(crate) store: &'a Path,
    /// The run's measured origin (its source kind and path included).
    pub(crate) origin: &'a Origin,
    /// The run's filter.
    pub(crate) only: &'a [String],
    /// The verb's words after `gdfl-import`.
    pub(crate) args: &'a [&'a str],
}

impl Door {
    /// Checks the source root, takes the run lock and reads the tail, in that
    /// order: nothing is written before the root is known, and the ledger is
    /// read under the lock.
    ///
    /// # Errors
    ///
    /// Every refusal of [`source_root`] and [`RunLock::take`], and
    /// [`SOURCES_LEDGER`] for a ledger that cannot be read.
    pub(crate) fn open(at: Opening<'_>) -> Result<Self, Finding> {
        let origin = at.origin;
        source_root(
            Shape::of(origin.source, at.kind),
            at.var,
            &origin.path,
            origin,
        )?;
        let canonical = canonical_source(at.var, &origin.path)?;
        let holder = Holder {
            pid: std::process::id(),
            run_started: origin.started,
            kind: at.kind,
            verb: verb_of(at.args),
        };
        let lock = RunLock::take(at.store, &holder)?;
        let entry = Entry {
            started: origin.started,
            kind: at.kind.as_str().to_owned(),
            source: origin.source.as_str().to_owned(),
            digest: origin
                .digest
                .as_ref()
                .map_or_else(|| UNMEASURED.to_owned(), hex),
            outcome: String::new(),
            days_skipped: 0,
            files: 0,
            key: escape(filter_key(at.only).as_bytes()),
            path: escape(canonical.as_os_str().as_bytes()),
        };
        let ledger = sources_path(at.store);
        let key = Key {
            kind: &entry.kind,
            source: &entry.source,
            filter: &entry.key,
            path: &entry.path,
        };
        let tail = tail_for(&ledger, key).map_err(|why| Finding::new(SOURCES_LEDGER, why))?;
        let identity = identity(origin.source, origin.digest.as_ref(), &tail);
        Ok(Self {
            lock,
            ledger,
            tail,
            identity,
            entry,
            roots: (at.store.to_path_buf(), origin.path.clone()),
            // The origin's fault carries the source root through `display()`
            // (pull's words, never escaped there): printed as [`plain`] text.
            fault: origin.fault.as_deref().map(plain),
        })
    }

    /// Ends the run: prints the door's lines on the receipt, finds what the
    /// run's end means, appends the run's line, releases the lock, and prints
    /// one exit line per finding. `end` is the run's own end when it was not
    /// clean (`failed` or `refused`). Returns whether the door found
    /// anything, which makes the exit `FAILED`.
    pub(crate) fn close(
        mut self,
        report: Option<&Report>,
        end: Option<&str>,
        out: &mut String,
    ) -> bool {
        let _ = writeln!(
            out,
            "  run lock            {} ({}; pid {})",
            self.lock.name,
            layout_word(self.lock.layout),
            std::process::id()
        );
        if let Some(record) = &self.lock.dead {
            let _ = writeln!(
                out,
                "  previous holder     died: pid {} started {} (record present, lock free): {record}",
                field(record, "pid"),
                field(record, "run_started")
            );
        }
        if let Some(fault) = &self.fault {
            let _ = writeln!(out, "  source fault        {fault}");
        }
        self.print_identity(out);
        if self.tail.torn > 0 || self.tail.unreadable > 0 {
            let _ = writeln!(
                out,
                "  sources ledger      {}: a torn last line of {} bytes and {} unreadable lines, skipped for comparison",
                shown(&self.ledger),
                self.tail.torn,
                self.tail.unreadable
            );
        }
        let mut found = verdict(report, &self.identity, (&self.roots.0, &self.roots.1));
        if let Some(again) = report.and_then(|report| nothing_read_again(report, &self.tail))
            && !found.iter().any(|f| f.code == NO_FILE_READ)
        {
            found.push(again);
        }
        if let Some(report) = report {
            self.entry.days_skipped = unread(report);
            self.entry.files = report.files;
            if report.days_widened > 0 {
                let _ = writeln!(
                    out,
                    "  days widened        {} (done days whose listing alone was read again for a \
                     wider scope, WP-12f; their done members were not re-read, so the door counts \
                     them skipped unread)",
                    report.days_widened
                );
            }
        }
        let unchecked = unchecked(&self.identity, &found);
        if unchecked {
            let _ = writeln!(
                out,
                "  source change       unresolved and not checked by this run: it skipped no day \
                 unread, so it hid none and is not failed for it, but its line is written \
                 {SOURCE_CHANGED_UNCHECKED} and is never the baseline; the next run of this key \
                 that skips a done day fails {SOURCE_CHANGED} until the archive the done days \
                 came from is back, or IA-6's per-file comparison (IA-GV-24) and R3-22's restate \
                 door (R4-21) land them"
            );
        }
        self.entry.outcome = outcome(&found, unchecked, end);
        if gone(&self.roots.0) {
            // No ledger to append to: the loss is named, never skipped quietly.
            if !found.iter().any(|f| f.code == STORE_LOST) {
                found.extend(lost_store(&self.roots.0));
            }
        } else if let Err(why) = append(&self.ledger, &self.entry.line()) {
            found.push(why);
        }
        if let Err(why) = self.lock.release() {
            found.push(why);
        }
        for finding in &found {
            let _ = writeln!(out, "GDFL IMPORT FAILED: {finding}");
        }
        !found.is_empty()
    }

    /// The receipt's `source identity` line: what this run's source is
    /// against the last run of its key.
    fn print_identity(&self, out: &mut String) {
        let _ = match &self.identity {
            Identity::NotComparable => writeln!(out, "  source identity     {NOT_COMPARABLE}"),
            Identity::Unmeasured => writeln!(
                out,
                "  source identity     unmeasured (see the source fault); not compared"
            ),
            Identity::First => writeln!(
                out,
                "  source identity     {} (no earlier run of this key in {})",
                self.entry.digest,
                self.window_words()
            ),
            Identity::Same { since } => writeln!(
                out,
                "  source identity     {} (the same as the run of {since})",
                self.entry.digest
            ),
            Identity::Changed {
                old,
                since,
                unresolved,
                ..
            } if *unresolved > 0 => writeln!(
                out,
                "  source identity     {} (CHANGED: {old} at the run of {since}; {unresolved} later runs named it, unresolved)",
                self.entry.digest
            ),
            Identity::Changed { old, since, .. } => writeln!(
                out,
                "  source identity     {} (CHANGED: {old} at the run of {since})",
                self.entry.digest
            ),
            Identity::Unresolved { unresolved } => writeln!(
                out,
                "  source identity     {} (UNRESOLVED: {unresolved} runs in {} named a changed archive; the line they compared with is older)",
                self.entry.digest,
                self.window_words()
            ),
        };
    }

    /// The window the tail read covered, in words.
    fn window_words(&self) -> String {
        if self.tail.absent {
            format!("{} (absent until this run)", shown(&self.ledger))
        } else if self.tail.cut {
            format!("the last {TAIL_WINDOW} bytes of {}", shown(&self.ledger))
        } else {
            shown(&self.ledger)
        }
    }
}

#[cfg(test)]
#[path = "gdfl_import_guard_tests.rs"]
mod tests;
