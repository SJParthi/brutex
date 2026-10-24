//! `cli backup --to <dir> [--plan] [--mib-per-s N]` and
//! `cli backup-drill --from <dir> [--months N]`: the store's independent
//! second copy and its restore drill (AUT-06, DB-B73; D-7137, D-7175..D-7181).
//!
//! # Why
//!
//! One drive held the purchased vendor archives and the whole store, and no
//! copy of either existed: `/disk.json` said `"recorded": false` on every read
//! and every page carried "NO COPY of this store is recorded". Under D-7135
//! there is never a fresh-from-scratch rebuild, so a verified second copy with
//! a restore drill is the store's only disaster path. The runbook's manual
//! step (`rsync -a --delete-after` and a hand-written receipt) deleted on the
//! target and recorded nothing a later run could check; this verb replaces it.
//!
//! # What is copied (D-7175)
//!
//! What the store already LISTS, never a glob of the drive:
//!
//! * **every month every census names** (`census_store::read_vendor`, each
//!   vendor's newest entries): each sibling of the month the store keeps
//!   ([`FileKind::ALL`] but the `.lock`, which holds no data), its generation
//!   pointer under `bar-generations-v1/`, and every revision under
//!   `bar-revisions-v1/<n>/` with its `.repair-v1` receipt — the served
//!   generation and every older one;
//! * **the listed directories** [`LISTED`]: the census files (`manifest/`),
//!   the journals (`imports/`), the ledgers (`audit/`), the operations area
//!   (`ops/`, less this verb's own two logs, which change with every run), the
//!   verdict store (`verify/`), the reference tables (`reference/`) and the
//!   session masters (`session-masters/`);
//! * **the GDFL source archives by name and size**: every distinct
//!   `archive=<path>` the GDFL import journal's `run` lines name — the
//!   capital-market outer zip, the folder of yearly options zips, and the tick
//!   store's root, so the tick store's day files, its version files and its
//!   `tick-versions-v1/` are copied too (D-7180).
//!
//! Every other top-level name of the store root (`logs/`, `telemetry/`,
//! `results/`, ...) is NOT copied and is NAMED in every answer, plan or run,
//! so what the copy leaves out is said, never discovered. A month on disk that
//! no census names is not copied either: `cli census-recount` is what brings
//! it into the census.
//!
//! # On the target (D-7176)
//!
//! Everything lives under `<dir>/brutex-backup-v1/`:
//!
//! ```text
//! manifest.log                         append-only, one record per copy
//! backup.lock                          one run at a time per target
//! runs/<run>/store/<store path>        the bytes of a store file
//! runs/<run>/archives/<absolute path>  the bytes of a source archive file
//! ```
//!
//! A copy is written as `<name>.tmp`, fsynced, renamed to `<name>` (refused
//! if `<name>` exists) and its folder fsynced, then its record appended. Its
//! name carries the run that wrote it, and run ids only grow, so a changed
//! source is a NEW name and the old copy stays: nothing on the target is ever
//! overwritten or deleted. The manifest's lines:
//!
//! ```text
//! brutex-backup-manifest v1
//! run <run> started=<unix_s> commit=<sha|unstamped> store=<path>
//! file run=<run> class=<class> len=<n> mtime=<ns> blake3=<hex> path=<path>[ unit=<month>]
//! busy <month> run=<run>
//! missing <month> run=<run>
//! unreadable <path> run=<run> why=<sentence>
//! skipped <path> run=<run> why=<word>
//! unlisted <vendor> run=<run> why=<sentence>
//! stopped <run> why=<sentence>
//! end <run> state=<complete|partial> files=<n> bytes=<n> copied_files=<n> copied_bytes=<n> unchanged=<n> busy=<n> missing=<n> unreadable=<n>
//! ```
//!
//! Paths are percent-encoded as the GDFL journal encodes them (every byte
//! outside `A-Z a-z 0-9 - . _ ~ /` written `%XX`). A `file` record's `run=`
//! names the run whose folder holds its bytes. A store `path` is relative to
//! the store root; an archive's is absolute.
//!
//! # Unchanged, new or changed
//!
//! A store file is unchanged when its length and modification time (to the
//! nanosecond) equal its newest record's; a length that equals with a time
//! that does not is read and hashed, and a digest that equals the record's is
//! a new record over the copy already held, not a second copy. An archive is
//! unchanged when its name and size equal its newest record's (D-7175). A
//! change that keeps both the length and the nanosecond time is not seen: the
//! drill and S-17's scrub of the copy read the bytes back
//! (`docs/06-limits.md`).
//!
//! # A month a writer holds
//!
//! A month is copied under its SHARED lock (the original's `.lock` and every
//! revision's), exactly the lock `BarFile::open_existing` takes, so no writer
//! appends while it is read. A month a writer holds is a `busy` line, tried
//! once more after every other unit, and left for the next run if still held:
//! never copied half-written. The rate limit is paid after the month is
//! released, never with it held (rules audit RA-128).
//!
//! # Refusals, before anything is written
//!
//! `target_missing`, `not_a_directory`, `inside_store`, `same_device` (the
//! target's `st_dev` is the store root's), `inside_archive` (the target lies
//! inside a source archive this verb copies), `target_path_too_long` (the
//! receipt line could not hold it), `receipt_unwritable` (the receipt log is
//! no regular file), `nothing_listed` (the store lists nothing: a wrong
//! root), `insufficient_space` (less free space than the plan's bytes to
//! copy) or `space_unknown`, `journal_unreadable`, `target_busy` (another run
//! holds the target), and `manifest_unreadable` (a line this build cannot
//! read). A run the target stops is `target_failed` or `target_unwritable`.
//!
//! # The receipt (D-7177)
//!
//! A run that reaches its end appends ONE line to `<store>/ops/backups.log`,
//! the line `docs/36-ops-runbook.md` §4 defines and `api`'s `last_backup`
//! already reads, its first two words unchanged:
//!
//! ```text
//! copied=<UTC> dest=<dir> raw_bytes=<n> store_bytes=<n> by=cli-backup run=<run> state=<complete|partial> files=<n> bytes=<n> copied_files=<n> copied_bytes=<n> unchanged=<n> busy=<n> missing=<n> unreadable=<n> commit=<sha|unstamped>
//! ```
//!
//! `files` and `bytes` are what the target holds (every path's newest copy);
//! `raw_bytes` and `store_bytes` split them into archives and store files. A
//! second run copies nothing and says `copied_files=0`. A run stopped by the
//! target (a full disk, a lost drive) appends no receipt: its copies so far
//! are durable and recorded, and the next run finishes the rest.
//!
//! # The drill (D-7178)
//!
//! `backup-drill` restores `N` months (3 by default), chosen from the manifest
//! across kinds (the store path's segment) and rotated by the drill's id, into
//! a NEW directory under the system's temporary folder — refused inside the
//! store or the target — checks every restored file against its manifest
//! digest, and compares it byte for byte with the store's file under the
//! month's shared lock. A store file whose length or time moved since the
//! copy is compared with the digest only and counted `moved`. It appends one
//! line to `<store>/ops/backup-drills.log`, never `ops/backups.log` (whose
//! newest line must stay a receipt), and writes nothing else in the store.
//!
//! # Cost
//!
//! Per file: one `lstat`, one probe of a map built once from the manifest,
//! and one record appended (`cli::backup::tests::the_per_file_step_probes_the_manifest_once`);
//! a copy is O(the file's bytes) on top. Per month: a bounded number of
//! `lstat`s (twelve siblings, the pointer, the revisions up to
//! `store::repair::MAX_REVISIONS`). Per run: one manifest read, one census
//! read per vendor and one journal read. A full copy is O(bytes) by nature, a
//! one-time pass (`docs/06-limits.md`).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, BufRead as _, Read as _, Write as _};
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::{FileExt as _, MetadataExt as _, OpenOptionsExt as _};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use brutex_core::vendor::Vendor;
use pull::ingest::census_store;
use pull::manifest::Entry;
use rayon::prelude::*;
use store::flock::Flock;
use store::path::{FileKind, PathParts, StorePath};

use crate::{FAILED, MISUSED, OK};

/// The audited target: every state change of both verbs is an event here.
pub(crate) const TARGET: &str = "cli.backup.lifecycle";

/// The target of the per-unit notes (a busy, missing or unreadable unit).
pub(crate) const NOTES: &str = "cli.backup";

/// The folder under `--to` that holds the manifest, the lock and the copies.
pub(crate) const HOME: &str = "brutex-backup-v1";

/// The append-only manifest, under [`HOME`].
pub(crate) const MANIFEST: &str = "manifest.log";

/// A manifest's first line.
const MANIFEST_HEAD: &str = "brutex-backup-manifest v1";

/// The one-run-at-a-time lock, under [`HOME`].
const LOCK: &str = "backup.lock";

/// The folder of run folders, under [`HOME`].
const RUNS: &str = "runs";

/// The receipt log, relative to the store root: `api`'s `BACKUP_RECEIPTS`.
pub const RECEIPTS: &str = "ops/backups.log";

/// The drill log, relative to the store root (D-7178).
pub const DRILLS: &str = "ops/backup-drills.log";

/// The directories the store keeps, each copied whole: `(class, directory)`.
pub(crate) const LISTED: [(&str, &str); 7] = [
    ("census", "manifest"),
    ("journal", "imports"),
    ("ledger", "audit"),
    ("ops", "ops"),
    ("verify", "verify"),
    ("reference", "reference"),
    ("session-masters", "session-masters"),
];

/// The top-level names copied through the census's months rather than walked.
const BY_MONTH: [&str; 3] = ["bars", store::generation::ROOT, "bar-revisions-v1"];

/// The class of every file of a month.
const MONTH: &str = "month";

/// The class of every source archive file.
const ARCHIVE: &str = "archive";

/// The longest line this module appends to a store log: `api`'s
/// `MAX_RECEIPT_LINE`, past which `last_backup` takes a line for no receipt
/// (`api::disk_route` holds both equal in a test).
pub(crate) const MAX_LINE: usize = 1_024;

/// The words that end a torn tail: no reader reads a line that ends in them.
const VOID: &str = " #void";

/// Bytes read and written at a time.
const CHUNK: usize = 1 << 20;

/// Records appended between two manifest syncs, at most.
const SYNC_RECORDS: u64 = 256;

/// Time between two manifest syncs, at most.
const SYNC_EVERY: Duration = Duration::from_secs(2);

/// The most names a list in an answer carries; the manifest carries all.
const NAMED: usize = 16;

/// The default number of months a drill restores.
const DRILL_MONTHS: u64 = 3;

// ---------------------------------------------------------------- encoding

/// `bytes` as one field value: every byte outside `A-Z a-z 0-9 - . _ ~ /`
/// written `%XX`, upper-case hex — the GDFL journal's rule (D-3317), so a
/// value never holds a space or a newline and loses no byte.
pub(crate) fn encoded(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        if is_plain(byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

const fn is_plain(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/')
}

/// `text` decoded, or `None` for a byte [`encoded`] never leaves plain or a
/// broken escape.
#[must_use]
pub fn decoded(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        if byte == b'%' {
            let pair = bytes.get(at + 1..at + 3)?;
            let digit = |b: u8| char::from(b).to_digit(16);
            let (Some(high), Some(low)) = (digit(*pair.first()?), digit(*pair.last()?)) else {
                return None;
            };
            out.push(u8::try_from(high * 16 + low).ok()?);
            at += 3;
        } else if is_plain(byte) {
            out.push(byte);
            at += 1;
        } else {
            return None;
        }
    }
    Some(out)
}

fn hex(digest: &[u8; 32]) -> String {
    digest
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

fn unhex(text: &str) -> Option<[u8; 32]> {
    let bytes = text.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0_u8; 32];
    for (slot, pair) in out.iter_mut().zip(bytes.chunks(2)) {
        let digit = |b: Option<&u8>| b.and_then(|b| char::from(*b).to_digit(16));
        let (high, low) = (digit(pair.first())?, digit(pair.last())?);
        *slot = u8::try_from(high * 16 + low).ok()?;
    }
    Some(out)
}

/// The map key of a recorded path: the first 16 bytes of its BLAKE3. 128 bits
/// keep a few million paths apart with a collision chance under 2^-80.
fn key(path: &[u8]) -> u128 {
    let digest = brutex_core::blake3::hash(path);
    let mut half = [0_u8; 16];
    for (slot, byte) in half.iter_mut().zip(digest) {
        *slot = byte;
    }
    u128::from_le_bytes(half)
}

fn mtime_ns(meta: &fs::Metadata) -> i64 {
    meta.mtime()
        .saturating_mul(1_000_000_000)
        .saturating_add(meta.mtime_nsec())
}

fn now_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// `secs` as the receipt's `YYYY-MM-DDTHH:MM:SSZ`.
pub(crate) fn utc(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).unwrap_or(i64::MAX);
    let (year, month, day) = telemetry::civil_from_days(days);
    let within = secs % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        within / 3_600,
        within % 3_600 / 60,
        within % 60
    )
}

fn shown(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Up to [`NAMED`] names of a list, and how many there are.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Named {
    /// How many.
    pub(crate) count: u64,
    /// The first [`NAMED`] of them.
    pub(crate) first: Vec<String>,
}

impl Named {
    fn add(&mut self, name: String) {
        self.count += 1;
        if self.first.len() < NAMED {
            self.first.push(name);
        }
    }

    fn text(&self) -> String {
        let more = self.count.saturating_sub(self.first.len() as u64);
        let mut out = self.first.join(", ");
        if more > 0 {
            let _ = write!(out, " and {more} more (every one is a manifest line)");
        }
        out
    }
}

// ---------------------------------------------------------------- manifest

/// The newest copy of one path, as the manifest records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Held {
    len: u64,
    mtime: i64,
    run: u64,
    digest: [u8; 32],
    archive: bool,
}

/// One `file` record, read whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Record {
    pub(crate) run: u64,
    pub(crate) class: String,
    pub(crate) len: u64,
    pub(crate) mtime: i64,
    pub(crate) digest: [u8; 32],
    pub(crate) path: Vec<u8>,
    pub(crate) unit: Option<Vec<u8>>,
}

/// `line` as a `file` record, or `None`: exactly the words this module writes,
/// in its order, and not a word more.
pub(crate) fn record_of(line: &str) -> Option<Record> {
    let mut words = line.split(' ');
    if words.next()? != "file" {
        return None;
    }
    let mut field = |name: &str| -> Option<&str> {
        words
            .next()?
            .strip_prefix(name)?
            .strip_prefix('=')
            .filter(|value| !value.is_empty())
    };
    let run = field("run")?.parse().ok()?;
    let class = field("class")?.to_owned();
    let len = field("len")?.parse().ok()?;
    let mtime = field("mtime")?.parse().ok()?;
    let digest = unhex(field("blake3")?)?;
    let path = decoded(field("path")?)?;
    let unit = match words.next() {
        None => None,
        Some(word) => Some(decoded(word.strip_prefix("unit=")?)?),
    };
    if words.next().is_some() || path.is_empty() || (class == MONTH) != unit.is_some() {
        return None;
    }
    Some(Record {
        run,
        class,
        len,
        mtime,
        digest,
        path,
        unit,
    })
}

fn record_line(
    run: u64,
    class: &str,
    len: u64,
    mtime: i64,
    digest: &[u8; 32],
    path: &[u8],
    unit: Option<&[u8]>,
) -> String {
    let mut line = format!(
        "file run={run} class={class} len={len} mtime={mtime} blake3={} path={}",
        hex(digest),
        encoded(path)
    );
    if let Some(unit) = unit {
        let _ = write!(line, " unit={}", encoded(unit));
    }
    line
}

/// What a target's manifest holds: the newest copy of every path, in a map
/// built once per run and probed once per file.
#[derive(Debug, Default)]
pub(crate) struct Book {
    held: HashMap<u128, Held>,
    /// The newest run id any line names.
    pub(crate) newest_run: u64,
    /// Paths held.
    pub(crate) files: u64,
    /// Their bytes.
    pub(crate) bytes: u64,
    /// The archives' share of [`Self::bytes`].
    pub(crate) raw_bytes: u64,
    /// The store files' share of [`Self::bytes`].
    pub(crate) store_bytes: u64,
    /// Whether the file ends in a fragment a killed run left.
    torn: bool,
    /// How many probes were made: one per file a run lists.
    pub(crate) probes: u64,
    /// The newest month records by path, kept for a drill only.
    months: Option<HashMap<Vec<u8>, Record>>,
}

impl Book {
    fn probe(&mut self, path: &[u8]) -> Option<Held> {
        self.probes += 1;
        self.held.get(&key(path)).copied()
    }

    fn insert(&mut self, path: &[u8], held: Held) {
        if let Some(old) = self.held.insert(key(path), held) {
            self.bytes = self.bytes.saturating_sub(old.len);
            if old.archive {
                self.raw_bytes = self.raw_bytes.saturating_sub(old.len);
            } else {
                self.store_bytes = self.store_bytes.saturating_sub(old.len);
            }
        } else {
            self.files += 1;
        }
        self.bytes = self.bytes.saturating_add(held.len);
        if held.archive {
            self.raw_bytes = self.raw_bytes.saturating_add(held.len);
        } else {
            self.store_bytes = self.store_bytes.saturating_add(held.len);
        }
        self.newest_run = self.newest_run.max(held.run);
    }

    /// The manifest at `path`, read once, or an empty book when there is none.
    ///
    /// # Errors
    ///
    /// A manifest that is not a regular file, cannot be read, does not begin
    /// with [`MANIFEST_HEAD`], or holds a whole line this build cannot read:
    /// what is held could not be told, and copying past it could claim copies
    /// that are not there.
    pub(crate) fn load(path: &Path, keep_months: bool) -> Result<Self, String> {
        let mut book = Self::default();
        let file = match store::open_flags::open_read_no_follow(path) {
            Ok(file) => file,
            Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(book),
            Err(why) => return Err(format!("{}: {why}", path.display())),
        };
        let meta = file
            .metadata()
            .map_err(|why| format!("{}: {why}", path.display()))?;
        if !meta.is_file() {
            return Err(format!("{} is not a regular file", path.display()));
        }
        // PRE-SIZED FROM THE FILE: no record is shorter than the shortest line
        // `record_line` writes, so neither map rehashes while the book loads.
        let shortest = record_line(0, "x", 0, 0, &[0; 32], b"x", None).len() + 1;
        let most = usize::try_from(meta.len()).unwrap_or(usize::MAX) / shortest;
        book.held.reserve(most);
        if keep_months {
            book.months = Some(HashMap::with_capacity(most));
        }
        let mut reader = io::BufReader::new(file);
        let mut line = Vec::new();
        let mut first = true;
        loop {
            line.clear();
            let read = reader
                .read_until(b'\n', &mut line)
                .map_err(|why| format!("{}: {why}", path.display()))?;
            if read == 0 {
                break;
            }
            if line.last() != Some(&b'\n') {
                // A FRAGMENT NOBODY FINISHED: a killed run's last append. Not a
                // line; the next run ends it before it appends.
                book.torn = true;
                break;
            }
            let text = std::str::from_utf8(line.get(..line.len() - 1).unwrap_or(&[]))
                .map_err(|_| format!("{} holds a line that is not UTF-8", path.display()))?;
            if first {
                first = false;
                if text != MANIFEST_HEAD {
                    return Err(format!(
                        "{} does not begin with {MANIFEST_HEAD:?}: it is not a manifest this \
                         build wrote, so nothing it holds is taken as copied",
                        path.display()
                    ));
                }
                continue;
            }
            book.read_line(text).ok_or_else(|| {
                format!(
                    "{} holds a line this build cannot read, so what the target holds cannot \
                     be told and nothing was copied: {:?}",
                    path.display(),
                    text.chars().take(200).collect::<String>()
                )
            })?;
        }
        Ok(book)
    }

    fn read_line(&mut self, text: &str) -> Option<()> {
        if text.ends_with(VOID) {
            return Some(());
        }
        let mut words = text.split(' ');
        match words.next()? {
            "file" => {
                let record = record_of(text)?;
                let archive = record.class == ARCHIVE;
                self.insert(
                    &record.path,
                    Held {
                        len: record.len,
                        mtime: record.mtime,
                        run: record.run,
                        digest: record.digest,
                        archive,
                    },
                );
                if record.class == MONTH
                    && let Some(months) = self.months.as_mut()
                {
                    months.insert(record.path.clone(), record);
                }
            }
            "run" | "end" | "stopped" => {
                let run: u64 = words.next()?.parse().ok()?;
                self.newest_run = self.newest_run.max(run);
            }
            "busy" | "missing" | "unreadable" | "skipped" | "unlisted" => {
                words.next()?;
                let run = words.next()?.strip_prefix("run=")?.parse().ok()?;
                self.newest_run = self.newest_run.max(run);
            }
            _ => return None,
        }
        Some(())
    }

    /// The newest month records grouped by month, for a drill.
    fn units(&self) -> BTreeMap<Vec<u8>, Vec<Record>> {
        let mut out: BTreeMap<Vec<u8>, Vec<Record>> = BTreeMap::new();
        for record in self.months.iter().flat_map(HashMap::values) {
            if let Some(unit) = &record.unit {
                out.entry(unit.clone()).or_default().push(record.clone());
            }
        }
        out
    }
}

/// The manifest, open for appending under the run's lock.
struct Ledger {
    file: File,
    path: PathBuf,
    unsynced: u64,
    synced_at: Instant,
    /// A write that did not finish (a full disk mid-line): the next line ends
    /// it as a void line first, so the two are never joined into one.
    torn: bool,
}

impl Ledger {
    /// Opens (or creates, with its first line) the manifest at `path`, ending
    /// a torn tail as a void line first.
    fn open(path: &Path, torn: bool) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .custom_flags(store::open_flags::O_NOFOLLOW)
            .open(path)?;
        if file.metadata()?.len() == 0 {
            file.write_all(format!("{MANIFEST_HEAD}\n").as_bytes())?;
        } else if torn {
            file.write_all(format!("{VOID}\n").as_bytes())?;
        }
        file.sync_data()?;
        Ok(Self {
            file,
            path: path.to_path_buf(),
            unsynced: 0,
            synced_at: Instant::now(),
            torn: false,
        })
    }

    fn line(&mut self, text: &str) -> io::Result<()> {
        let mut bytes = Vec::with_capacity(VOID.len() + text.len() + 2);
        if self.torn {
            bytes.extend_from_slice(VOID.as_bytes());
            bytes.push(b'\n');
        }
        bytes.extend_from_slice(text.as_bytes());
        bytes.push(b'\n');
        self.torn = true;
        self.file.write_all(&bytes)?;
        self.torn = false;
        self.unsynced += 1;
        if self.unsynced >= SYNC_RECORDS || self.synced_at.elapsed() >= SYNC_EVERY {
            self.sync()?;
        }
        Ok(())
    }

    fn sync(&mut self) -> io::Result<()> {
        self.file.sync_data()?;
        self.unsynced = 0;
        self.synced_at = Instant::now();
        Ok(())
    }
}

// ---------------------------------------------------------------- listing

/// One file the walk found, as it will be recorded.
#[derive(Debug, Clone)]
struct Found {
    class: &'static str,
    /// As recorded: relative to the store root, or absolute for an archive.
    path: Vec<u8>,
    /// Where it is read.
    source: PathBuf,
    len: u64,
    mtime: i64,
    unit: Option<Vec<u8>>,
}

impl Found {
    fn archive(&self) -> bool {
        self.class == ARCHIVE
    }
}

/// What the walk could not list or read, each named.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Faults {
    /// Months a writer held.
    pub(crate) busy: Named,
    /// Months a census names and the disk does not; archives a journal names
    /// and the disk does not.
    pub(crate) missing: Named,
    /// Files and folders that could not be read.
    pub(crate) unreadable: Named,
    /// Entries that are not regular files or folders (a link, a FIFO): not
    /// followed, not copied.
    pub(crate) skipped: Named,
    /// Vendors whose census would not load: their months are not listed.
    pub(crate) unlisted: Named,
    /// Census entries that name no legal store path.
    pub(crate) unaddressed: Named,
}

/// A census month, as listed.
#[derive(Debug, Clone)]
struct Month {
    vendor: Vendor,
    entry: Entry,
    unit: String,
}

fn path_of(vendor: Vendor, entry: &Entry) -> Option<StorePath<'_>> {
    StorePath::new(PathParts {
        vendor,
        exchange: entry.key.exchange.as_str(),
        segment: entry.key.segment.as_str(),
        symbol: entry.key.symbol.as_str(),
        contract: entry.key.contract,
        timeframe: entry.key.timeframe,
        month: entry.key.month,
        file: FileKind::Bars,
    })
    .ok()
}

/// Every month every census names, once, in the censuses' own order: the copy
/// is a one-time pass whose order nothing reads, so no sort is paid.
fn months(root: &Path, faults: &mut Faults) -> Vec<Month> {
    let mut listed: Vec<(Vendor, Entry)> = Vec::new();
    for vendor in Vendor::ALL {
        match census_store::read_vendor(root, vendor) {
            Ok(reads) => listed.extend(
                reads
                    .iter()
                    .flat_map(|read| read.census.newest())
                    .map(|entry| (vendor, entry)),
            ),
            Err(why) => faults
                .unlisted
                .add(format!("{}: {}", vendor.as_str(), one_line(&why))),
        }
    }
    // PRE-SIZED FROM THE CENSUSES, so the set never grows mid-walk.
    let mut seen = HashSet::with_capacity(listed.len());
    let mut out = Vec::with_capacity(listed.len());
    for (vendor, entry) in listed {
        let Some(unit) = path_of(vendor, &entry).map(|path| path.to_string()) else {
            faults.unaddressed.add(format!("{:?}", entry.key));
            continue;
        };
        if seen.insert(unit.clone()) {
            out.push(Month {
                vendor,
                entry,
                unit,
            });
        }
    }
    out
}

/// The files and locks of one month: every sibling but the `.lock`, the
/// generation pointer, and every revision with its receipt.
#[derive(Debug, Default)]
struct MonthFiles {
    files: Vec<PathBuf>,
    locks: Vec<PathBuf>,
    /// Why the generation pointer could not be read: its bytes are still
    /// copied, but a revision past a gap in the ordinals would not be found,
    /// so the month is named unreadable rather than called whole.
    pointer_fault: Option<String>,
}

fn month_files(root: &Path, path: StorePath<'_>) -> MonthFiles {
    let mut out = MonthFiles::default();
    let siblings = |under: &Path, out: &mut MonthFiles| {
        for kind in FileKind::ALL {
            let file = path.with_file(kind).to_path_buf(under);
            if kind == FileKind::Lock {
                out.locks.push(file);
            } else {
                out.files.push(file);
            }
        }
    };
    siblings(root, &mut out);
    out.files.push(store::generation::pointer_path(root, path));
    // EVERY REVISION, SERVED OR OLDER: ordinals are taken in order from 1 and
    // never reused (`store::repair::repair_month_with`), so the first ordinal
    // holding nothing ends the walk — past the pointer's newest, which a gap
    // could otherwise hide.
    let newest = match store::generation::served(root, path) {
        Ok(store::generation::Served::Revision(pointer)) => pointer.revision.ordinal(),
        Ok(store::generation::Served::Original) => 0,
        Err(why) => {
            out.pointer_fault = Some(format!(
                "its generation pointer cannot be read ({why}); a revision past a gap in \
                 its ordinals would not be found"
            ));
            0
        }
    };
    for ordinal in 1..=store::repair::MAX_REVISIONS {
        let Ok(revision) = store::repair::Revision::new(ordinal) else {
            break;
        };
        let under = store::repair::revision_root(root, revision);
        let bars = path.to_path_buf(&under);
        let receipt = bars.with_extension("repair-v1");
        let sealed = path.with_file(FileKind::SealedBars).to_path_buf(&under);
        let present = [&bars, &receipt, &sealed]
            .iter()
            .any(|file| fs::symlink_metadata(file).is_ok());
        if !present && ordinal > newest {
            break;
        }
        if present {
            siblings(&under, &mut out);
            out.files.push(receipt);
        }
    }
    out
}

/// What [`stat`] found at a path.
enum Stat {
    File(u64, i64),
    Absent,
    Other(String),
}

fn stat(path: &Path) -> Stat {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => Stat::File(meta.len(), mtime_ns(&meta)),
        Ok(meta) if meta.file_type().is_symlink() => Stat::Other("link".to_owned()),
        Ok(_) => Stat::Other("not a regular file".to_owned()),
        Err(why) if why.kind() == io::ErrorKind::NotFound => Stat::Absent,
        Err(why) => Stat::Other(why.to_string()),
    }
}

fn relative(root: &Path, path: &Path) -> Vec<u8> {
    path.strip_prefix(root)
        .unwrap_or(path)
        .as_os_str()
        .as_bytes()
        .to_vec()
}

/// The found files of one month, measured now, or `None` when the month has
/// no file at all (`missing`).
fn month_found(
    root: &Path,
    month: &Month,
    files: &MonthFiles,
    faults: &mut Faults,
) -> Option<Vec<Found>> {
    let mut found = Vec::new();
    for file in &files.files {
        match stat(file) {
            Stat::File(len, mtime) => found.push(Found {
                class: MONTH,
                path: relative(root, file),
                source: file.clone(),
                len,
                mtime,
                unit: Some(month.unit.as_bytes().to_vec()),
            }),
            Stat::Absent => {}
            Stat::Other(why) => faults
                .skipped
                .add(format!("{} ({why})", shown(&relative(root, file)))),
        }
    }
    (!found.is_empty()).then_some(found)
}

/// Every regular file under `dir`, depth first in name order, never following
/// a link. An absent `dir` holds nothing.
fn walk(
    dir: &Path,
    class: &'static str,
    rel_root: Option<&Path>,
    exclude: Option<&[PathBuf; 2]>,
    faults: &mut Faults,
) -> Vec<Found> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    let recorded = |path: &Path| match rel_root {
        Some(root) => relative(root, path),
        None => path.as_os_str().as_bytes().to_vec(),
    };
    while let Some(at) = stack.pop() {
        let entries = match fs::read_dir(&at) {
            Ok(entries) => entries,
            Err(why) if why.kind() == io::ErrorKind::NotFound && at == dir => continue,
            Err(why) => {
                faults
                    .unreadable
                    .add(format!("{} ({why})", shown(&recorded(&at))));
                continue;
            }
        };
        let mut names: Vec<PathBuf> = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => names.push(entry.path()),
                Err(why) => faults
                    .unreadable
                    .add(format!("{} ({why})", shown(&recorded(&at)))),
            }
        }
        // Depth first, in the order the folder lists them: folders are pushed
        // in reverse. The copy reads no order, so none is sorted for.
        let mut folders = Vec::new();
        for path in names {
            if exclude.is_some_and(|[receipts, drills]| *receipts == path || *drills == path) {
                continue;
            }
            match fs::symlink_metadata(&path) {
                Ok(meta) if meta.is_dir() => folders.push(path),
                Ok(meta) if meta.is_file() => out.push(Found {
                    class,
                    path: recorded(&path),
                    source: path,
                    len: meta.len(),
                    mtime: mtime_ns(&meta),
                    unit: None,
                }),
                Ok(meta) => {
                    let why = if meta.file_type().is_symlink() {
                        "link"
                    } else {
                        "not a regular file"
                    };
                    faults
                        .skipped
                        .add(format!("{} ({why})", shown(&recorded(&path))));
                }
                Err(why) => faults
                    .unreadable
                    .add(format!("{} ({why})", shown(&recorded(&path)))),
            }
        }
        stack.extend(folders.into_iter().rev());
    }
    out
}

/// Every distinct archive the GDFL journal's `run` lines name, in the order
/// first named; `unrecorded` is left out, and a run line whose archive cannot
/// be read is named in `faults.unreadable`.
pub(crate) fn archives(root: &Path, faults: &mut Faults) -> Result<Vec<PathBuf>, String> {
    let journal = pull::gdfl_import::journal_path(root);
    let file = match store::open_flags::open_read_no_follow(&journal) {
        Ok(file) => file,
        Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{}: {why}", journal.display())),
    };
    let mut out = Vec::new();
    let mut reader = io::BufReader::new(file);
    let mut line = Vec::new();
    let mut number = 0_u64;
    loop {
        line.clear();
        let read = reader
            .read_until(b'\n', &mut line)
            .map_err(|why| format!("{}: {why}", journal.display()))?;
        // A FRAGMENT AFTER THE LAST NEWLINE IS NO LINE: a killed append's.
        if read == 0 || line.pop() != Some(b'\n') {
            break;
        }
        number += 1;
        if !line.starts_with(b"run ") {
            continue;
        }
        // A RUN LINE WHOSE ARCHIVE CANNOT BE READ IS NAMED, never skipped in
        // silence: the archive it names would go uncopied unseen.
        let value = std::str::from_utf8(&line).ok().and_then(|text| {
            text.split(' ')
                .find_map(|word| word.strip_prefix("archive="))
        });
        match value {
            Some("unrecorded") => {}
            Some(value) => {
                match decoded(value).map(|path| PathBuf::from(std::ffi::OsStr::from_bytes(&path))) {
                    Some(path) if path.is_absolute() => {
                        out.push(path);
                    }
                    _ => faults.unreadable.add(format!(
                        "{} line {number} (its archive= is no absolute path this build can decode)",
                        journal.display()
                    )),
                }
            }
            None => faults.unreadable.add(format!(
                "{} line {number} (a run line with no readable archive=)",
                journal.display()
            )),
        }
    }
    // ONCE EACH, by a set pre-sized from the lines that named one.
    let mut seen = HashSet::with_capacity(out.len());
    out.retain(|path| seen.insert(path.clone()));
    Ok(out)
}

/// The archive files one journal-named archive holds: the file itself, or
/// every regular file under the folder.
fn archive_found(archive: &Path, faults: &mut Faults) -> Vec<Found> {
    if archive
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        faults.unreadable.add(format!(
            "{} (an archive path with `..` is not copied)",
            archive.display()
        ));
        return Vec::new();
    }
    match fs::symlink_metadata(archive) {
        Ok(meta) if meta.is_file() => vec![Found {
            class: ARCHIVE,
            path: archive.as_os_str().as_bytes().to_vec(),
            source: archive.to_path_buf(),
            len: meta.len(),
            mtime: mtime_ns(&meta),
            unit: None,
        }],
        Ok(meta) if meta.is_dir() => walk(archive, ARCHIVE, None, None, faults),
        Ok(_) => {
            faults.skipped.add(format!(
                "{} (not a regular file or folder)",
                archive.display()
            ));
            Vec::new()
        }
        Err(why) if why.kind() == io::ErrorKind::NotFound => {
            faults
                .missing
                .add(format!("{} (archive)", archive.display()));
            Vec::new()
        }
        Err(why) => {
            faults
                .unreadable
                .add(format!("{} ({why})", archive.display()));
            Vec::new()
        }
    }
}

/// The top-level names of the store root this verb does not copy, in the
/// order the root lists them.
fn not_covered(root: &Path) -> Vec<String> {
    fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            !BY_MONTH.iter().any(|by| by == name) && !LISTED.iter().any(|(_, dir)| dir == name)
        })
        .collect()
}

// ---------------------------------------------------------------- the run

/// How the verb was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ask {
    /// The target folder.
    pub(crate) to: PathBuf,
    /// Count and measure only.
    pub(crate) plan: bool,
    /// At most this many bytes a second; zero is unthrottled.
    pub(crate) bytes_per_s: u64,
}

/// What the verb reads of its machine, passed in so every arm has a test.
pub(crate) struct Env<'a> {
    /// The build's commit.
    pub(crate) commit: Option<&'a str>,
    /// The `st_dev` of a path.
    pub(crate) device: &'a dyn Fn(&Path) -> io::Result<u64>,
    /// The bytes free under a path.
    pub(crate) free: &'a dyn Fn(&Path) -> Result<u64, String>,
    /// Faults a test injects; none in the binary.
    pub(crate) inject: Inject,
}

/// Faults a test injects into a run; [`Inject::default`] in the binary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Inject {
    /// After this many copies, the next one is cut half way, as a kill would
    /// cut it, with a torn record behind it.
    pub(crate) kill_after: Option<u64>,
    /// The bytes the target takes before it answers "no space left".
    pub(crate) space: Option<u64>,
}

/// One class's counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    /// Files listed.
    pub(crate) listed: u64,
    /// Their bytes.
    pub(crate) listed_bytes: u64,
    /// Files unchanged since their newest copy.
    pub(crate) unchanged: u64,
    /// Files with no copy yet.
    pub(crate) new: u64,
    /// Files changed since their newest copy.
    pub(crate) changed: u64,
    /// Bytes of the new and changed files: what a copy writes.
    pub(crate) copy_bytes: u64,
    /// Files copied by this run.
    pub(crate) copied: u64,
    /// Bytes copied by this run.
    pub(crate) copied_bytes: u64,
    /// Files whose time moved and whose bytes did not: recorded, not copied.
    pub(crate) recorded_only: u64,
}

/// Why a run was refused or stopped: a word and a sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Refusal {
    /// The refusal's name.
    pub(crate) word: &'static str,
    /// What was wrong, in words.
    pub(crate) why: String,
}

fn refused(word: &'static str, why: String) -> Refusal {
    Refusal { word, why }
}

/// What a run did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Ran {
    /// The run id; zero for a plan.
    pub(crate) run: u64,
    /// The target folder, canonical.
    pub(crate) dest: PathBuf,
    /// Per class.
    pub(crate) classes: BTreeMap<&'static str, Counts>,
    /// What could not be listed or copied.
    pub(crate) faults: Faults,
    /// The store's top-level names this verb does not copy.
    pub(crate) not_covered: Vec<String>,
    /// The bytes free on the target when measured.
    pub(crate) free: u64,
    /// The target's book after the run.
    pub(crate) files: u64,
    /// Its bytes.
    pub(crate) bytes: u64,
    /// The receipt appended, for a run.
    pub(crate) receipt: Option<String>,
    /// Probes of the manifest's map this run made.
    pub(crate) probes: u64,
}

impl Ran {
    fn total(&self) -> Counts {
        self.classes
            .values()
            .fold(Counts::default(), |sum, c| Counts {
                listed: sum.listed + c.listed,
                listed_bytes: sum.listed_bytes.saturating_add(c.listed_bytes),
                unchanged: sum.unchanged + c.unchanged,
                new: sum.new + c.new,
                changed: sum.changed + c.changed,
                copy_bytes: sum.copy_bytes.saturating_add(c.copy_bytes),
                copied: sum.copied + c.copied,
                copied_bytes: sum.copied_bytes.saturating_add(c.copied_bytes),
                recorded_only: sum.recorded_only + c.recorded_only,
            })
    }

    /// Whether every listed unit was copied or found unchanged.
    pub(crate) fn complete(&self) -> bool {
        self.faults.busy.count == 0
            && self.faults.missing.count == 0
            && self.faults.unreadable.count == 0
            && self.faults.unlisted.count == 0
    }
}

/// How a listed file compares with its newest copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Unchanged,
    New,
    Changed,
    /// Same length, another time: hashed before it is copied.
    Touched(Held),
}

fn verdict(book: &mut Book, found: &Found) -> Verdict {
    match book.probe(&found.path) {
        None => Verdict::New,
        Some(held) if found.archive() => {
            if held.len == found.len {
                Verdict::Unchanged
            } else {
                Verdict::Changed
            }
        }
        Some(held) if held.len != found.len => Verdict::Changed,
        Some(held) if held.mtime == found.mtime => Verdict::Unchanged,
        Some(held) => Verdict::Touched(held),
    }
}

fn tally(counts: &mut Counts, found: &Found, verdict: Verdict) {
    counts.listed += 1;
    counts.listed_bytes = counts.listed_bytes.saturating_add(found.len);
    match verdict {
        Verdict::Unchanged => counts.unchanged += 1,
        Verdict::New => counts.new += 1,
        Verdict::Changed | Verdict::Touched(_) => counts.changed += 1,
    }
    if verdict != Verdict::Unchanged {
        counts.copy_bytes = counts.copy_bytes.saturating_add(found.len);
    }
}

/// Every lock of `locks` that exists, held shared; `Err(Ok(lock))` when a
/// writer holds one, `Err(Err(why))` when one cannot be opened. An absent
/// lock is no writer: the store makes one only for a writer.
fn hold(locks: &[PathBuf]) -> Result<Vec<Flock<File>>, Result<PathBuf, String>> {
    let mut held = Vec::new();
    for lock in locks {
        let file = match File::open(lock) {
            Ok(file) => file,
            Err(why) if why.kind() == io::ErrorKind::NotFound => continue,
            Err(why) => return Err(Err(format!("{}: {why}", lock.display()))),
        };
        match Flock::try_lock_shared(file, lock.clone()) {
            Ok(flock) => held.push(flock),
            Err(TryLockError::WouldBlock) => return Err(Ok(lock.clone())),
            Err(TryLockError::Error(why)) => {
                return Err(Err(format!("{}: {why}", lock.display())));
            }
        }
    }
    Ok(held)
}

/// The checks every form makes before it reads the store: the target exists,
/// is a folder, is not inside the store, is not on the store's device, and
/// fits in a receipt. Returns the target, canonical.
fn check_target(root: &Path, to: &Path, env: &Env<'_>) -> Result<PathBuf, Refusal> {
    match fs::metadata(to) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            return Err(refused(
                "not_a_directory",
                format!("{} is not a directory", to.display()),
            ));
        }
        Err(why) if why.kind() == io::ErrorKind::NotFound => {
            return Err(refused(
                "target_missing",
                format!(
                    "{} does not exist: is the second drive attached and mounted?",
                    to.display()
                ),
            ));
        }
        Err(why) => {
            return Err(refused(
                "target_missing",
                format!("{} cannot be read: {why}", to.display()),
            ));
        }
    }
    let dest = fs::canonicalize(to)
        .map_err(|why| refused("target_missing", format!("{}: {why}", to.display())))?;
    let store = fs::canonicalize(root)
        .map_err(|why| refused("store_unreadable", format!("{}: {why}", root.display())))?;
    if dest.starts_with(&store) {
        return Err(refused(
            "inside_store",
            format!(
                "{} is inside the store {}: a copy beside what it copies dies with it",
                dest.display(),
                store.display()
            ),
        ));
    }
    let (target_dev, store_dev) = ((env.device)(&dest), (env.device)(&store));
    match (target_dev, store_dev) {
        (Ok(target), Ok(of_store)) if target == of_store => {
            return Err(refused(
                "same_device",
                format!(
                    "{} is on the store's own device (st_dev {target}): a drive failure \
                     would take both; use a second drive",
                    dest.display()
                ),
            ));
        }
        (Ok(_), Ok(_)) => {}
        (Err(why), _) | (_, Err(why)) => {
            return Err(refused(
                "same_device",
                format!(
                    "the devices of {} and {} cannot be told: {why}",
                    dest.display(),
                    store.display()
                ),
            ));
        }
    }
    check_receipt(&store, &dest)?;
    Ok(dest)
}

/// Refuses a target whose receipt `api` could not read, or a receipt log
/// that is no regular file: both before anything is copied.
fn check_receipt(store: &Path, dest: &Path) -> Result<(), Refusal> {
    // THE LONGEST RECEIPT THIS TARGET COULD GET, every number at its widest:
    // a line `api` would refuse as no receipt is refused here, before a copy.
    let widest = Receipt {
        copied_at: now_s(),
        dest: dest.to_path_buf(),
        raw_bytes: u64::MAX,
        store_bytes: u64::MAX,
        run: u64::MAX,
        complete: false,
        files: u64::MAX,
        bytes: u64::MAX,
        copied_files: u64::MAX,
        copied_bytes: u64::MAX,
        unchanged: u64::MAX,
        busy: u64::MAX,
        missing: u64::MAX,
        unreadable: u64::MAX,
        commit: Some("0".repeat(40)),
    }
    .line()
    .len();
    if widest > MAX_LINE {
        return Err(refused(
            "target_path_too_long",
            format!(
                "{} makes a receipt of up to {widest} bytes, past the {MAX_LINE} a receipt line \
                 carries (docs/36-ops-runbook.md §4)",
                dest.display()
            ),
        ));
    }
    // THE RECEIPT'S FOLDER AND FILE, looked at before a byte is copied: an
    // `ops` that is no folder, or a log that is no regular file, would leave
    // a whole copy that no receipt could record.
    let log = store.join(RECEIPTS);
    for (path, want_dir) in [
        (log.parent().unwrap_or(store), true),
        (log.as_path(), false),
    ] {
        match fs::symlink_metadata(path) {
            Ok(meta) if (want_dir && !meta.is_dir()) || (!want_dir && !meta.is_file()) => {
                return Err(refused(
                    "receipt_unwritable",
                    format!(
                        "{} is not a {}, so no receipt could be appended; nothing was copied",
                        path.display(),
                        if want_dir { "folder" } else { "regular file" }
                    ),
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

/// A listed unit: a month (its locks and found files) or one file.
enum Unit {
    Month(Month),
    File(Found),
}

/// Every unit, in the order a run copies them: the months, then the listed
/// directories, then the archives; and the archives' roots.
fn units(root: &Path, faults: &mut Faults) -> Result<(Vec<Unit>, Vec<PathBuf>), Refusal> {
    let mut out: Vec<Unit> = months(root, faults).into_iter().map(Unit::Month).collect();
    let own = [root.join(RECEIPTS), root.join(DRILLS)];
    for (class, dir) in LISTED {
        out.extend(
            walk(&root.join(dir), class, Some(root), Some(&own), faults)
                .into_iter()
                .map(Unit::File),
        );
    }
    let archives = archives(root, faults).map_err(|why| refused("journal_unreadable", why))?;
    for archive in &archives {
        out.extend(archive_found(archive, faults).into_iter().map(Unit::File));
    }
    Ok((out, archives))
}

/// The verb's one entry: a plan, or a run.
///
/// # Errors
///
/// A refusal before anything was written, or the target's fault that stopped
/// a run (its copies so far are durable and recorded).
pub(crate) fn backup(root: &Path, ask: &Ask, env: &Env<'_>) -> Result<Ran, Refusal> {
    let dest = check_target(root, &ask.to, env)?;
    let home = dest.join(HOME);
    let manifest = home.join(MANIFEST);
    let mut faults = Faults::default();
    let (units, archive_roots) = units(root, &mut faults)?;
    // A STORE THAT LISTS NOTHING IS NOT BACKED UP AS "0 FILES": that is a root
    // pointed at the wrong folder, and a receipt would say a copy exists.
    if units.is_empty() {
        return Err(refused(
            "nothing_listed",
            format!(
                "{} lists nothing to copy: no census names a month, no listed folder holds a \
                 file and the journal names no archive{}; nothing was written",
                root.display(),
                if faults.unlisted.count > 0 {
                    format!(
                        " (censuses that would not load: {})",
                        faults.unlisted.text()
                    )
                } else {
                    String::new()
                }
            ),
        ));
    }
    // ONE HASH PROBE PER ANCESTOR OF THE TARGET, bounded by its path's depth,
    // never a scan of the archives the journal names.
    let roots: HashSet<&Path> = archive_roots.iter().map(PathBuf::as_path).collect();
    if let Some(archive) = dest.ancestors().find(|at| roots.contains(at)) {
        return Err(refused(
            "inside_archive",
            format!(
                "{} is inside the source archive {}, which this verb copies: the copy would \
                 copy itself",
                dest.display(),
                archive.display()
            ),
        ));
    }
    // THE PLAN: every unit measured against the book, read without a lock and
    // without writing a byte anywhere.
    let mut book =
        Book::load(&manifest, false).map_err(|why| refused("manifest_unreadable", why))?;
    let mut planned = Ran {
        dest: dest.clone(),
        not_covered: not_covered(root),
        ..Ran::default()
    };
    plan(root, &units, &mut book, &mut planned, faults.clone());
    let needed = planned.total().copy_bytes;
    planned.free = (env.free)(&dest).map_err(|why| refused("space_unknown", why))?;
    planned.files = book.files;
    planned.bytes = book.bytes;
    planned.probes = book.probes;
    if planned.free < needed {
        return Err(refused(
            "insufficient_space",
            format!(
                "{} has {} byte(s) free and the plan copies {needed} byte(s) ({} file(s)); \
                 nothing was written",
                dest.display(),
                planned.free,
                planned.total().new + planned.total().changed
            ),
        ));
    }
    if ask.plan {
        return Ok(planned);
    }
    copy(
        root,
        ask,
        env,
        (&dest, &home, &manifest),
        (units, faults),
        &planned,
    )
}

/// Fills `ran` with what a copy would do: per class, and the months a writer
/// holds now.
fn plan(root: &Path, units: &[Unit], book: &mut Book, ran: &mut Ran, mut faults: Faults) {
    for unit in units {
        match unit {
            Unit::Month(month) => {
                let Some(path) = path_of(month.vendor, &month.entry) else {
                    continue;
                };
                let files = month_files(root, path);
                if let Some(why) = &files.pointer_fault {
                    faults.unreadable.add(format!("{} ({why})", month.unit));
                }
                match hold(&files.locks) {
                    Ok(held) => drop(held),
                    Err(Ok(_)) => faults.busy.add(month.unit.clone()),
                    Err(Err(why)) => faults.unreadable.add(format!("{} ({why})", month.unit)),
                }
                match month_found(root, month, &files, &mut faults) {
                    Some(found) => {
                        for file in &found {
                            let verdict = verdict(book, file);
                            tally(ran.classes.entry(MONTH).or_default(), file, verdict);
                        }
                    }
                    None => faults.missing.add(month.unit.clone()),
                }
            }
            Unit::File(file) => {
                let verdict = verdict(book, file);
                tally(ran.classes.entry(file.class).or_default(), file, verdict);
            }
        }
    }
    ran.faults = faults;
}

/// Where one run writes, and how far it may go.
struct Copier<'a> {
    home: &'a Path,
    run: u64,
    inject: Inject,
    copied: u64,
    written: u64,
    known: HashSet<PathBuf>,
}

/// Why one copy did not finish.
enum Cut {
    /// The source could not be read: named, skipped, the run goes on.
    Source(String),
    /// The target refused a write: the run stops.
    Target(String),
    /// A test's kill.
    Killed,
}

fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

impl Copier<'_> {
    /// Where `found`'s bytes go in this run's folder.
    fn destination(&self, found: &Found) -> PathBuf {
        let mut out = self.home.join(RUNS).join(self.run.to_string());
        let path = Path::new(std::ffi::OsStr::from_bytes(&found.path));
        out.push(if found.archive() { "archives" } else { "store" });
        for part in path.components() {
            if let Component::Normal(part) = part {
                out.push(part);
            }
        }
        out
    }

    /// Makes every missing folder down to `dir`, each one's parent synced.
    fn ensure_dir(&mut self, dir: &Path) -> io::Result<()> {
        if self.known.contains(dir) {
            return Ok(());
        }
        let mut missing = Vec::new();
        let mut at = dir;
        loop {
            match fs::symlink_metadata(at) {
                Ok(meta) if meta.is_dir() => break,
                Ok(_) => {
                    return Err(io::Error::other(format!(
                        "{} is not a directory",
                        at.display()
                    )));
                }
                Err(why) if why.kind() == io::ErrorKind::NotFound => {
                    missing.push(at.to_path_buf());
                    at = at.parent().ok_or(why)?;
                }
                Err(why) => return Err(why),
            }
        }
        for folder in missing.iter().rev() {
            fs::create_dir(folder)?;
            if let Some(parent) = folder.parent() {
                sync_dir(parent)?;
            }
        }
        self.known.insert(dir.to_path_buf());
        Ok(())
    }

    fn room(&mut self, bytes: usize) -> Result<(), Cut> {
        self.written = self.written.saturating_add(bytes as u64);
        match self.inject.space {
            Some(space) if self.written > space => Err(Cut::Target(
                "No space left on device (os error 28, injected by a test)".to_owned(),
            )),
            _ => Ok(()),
        }
    }

    /// Copies `found` to a temporary name, fsyncs it and renames it to its
    /// name in this run's folder, refusing a name that exists. Returns its
    /// BLAKE3 and the folder to sync before its record is appended.
    fn copy(&mut self, found: &Found) -> Result<([u8; 32], PathBuf), Cut> {
        let dest = self.destination(found);
        let folder = dest
            .parent()
            .ok_or_else(|| Cut::Target(format!("{} has no folder", dest.display())))?
            .to_path_buf();
        self.ensure_dir(&folder)
            .map_err(|why| Cut::Target(format!("{}: {why}", folder.display())))?;
        let mut name = dest.as_os_str().to_owned();
        name.push(".tmp");
        let tmp = PathBuf::from(name);
        let mut source = store::open_flags::open_read_no_follow(&found.source)
            .map_err(|why| Cut::Source(format!("{}: {why}", found.source.display())))?;
        // A STAGING NAME OF THIS RUN ALONE: run ids only grow, so no other run
        // and no earlier one was ever handed it (`store::staging`, RA-290).
        let staged = store::staging::create(&tmp)
            .map_err(|why| Cut::Target(format!("{}: {why}", tmp.display())))?;
        let mut out = staged.file;
        if let Some(bytes) = staged.cleared {
            crate::note(
                &telemetry::Event::warn(NOTES, "backup staging leftover cleared")
                    .with("path", tmp.display().to_string().as_str())
                    .with("bytes", bytes),
            );
        }
        let mut hasher = brutex_core::blake3::Hasher::new();
        let mut buffer = vec![0_u8; CHUNK];
        let mut left = found.len;
        let kill = self.inject.kill_after == Some(self.copied);
        while left > 0 {
            let want = usize::try_from(left.min(CHUNK as u64)).unwrap_or(CHUNK);
            let chunk = buffer
                .get_mut(..want)
                .ok_or_else(|| Cut::Target("chunk".to_owned()))?;
            source.read_exact(chunk).map_err(|why| {
                Cut::Source(if why.kind() == io::ErrorKind::UnexpectedEof {
                    format!(
                        "{} shrank while it was copied: {} byte(s) were listed",
                        found.source.display(),
                        found.len
                    )
                } else {
                    format!("{}: {why}", found.source.display())
                })
            })?;
            hasher.update(chunk);
            if kill {
                // A KILL CUTS THE COPY HALF WAY: half a chunk reaches the
                // temporary name and nothing is renamed or recorded.
                let half = chunk.get(..want / 2).unwrap_or(&[]);
                let _ = out.write_all(half);
                return Err(Cut::Killed);
            }
            self.room(want)?;
            out.write_all(chunk)
                .map_err(|why| Cut::Target(format!("{}: {why}", tmp.display())))?;
            left -= want as u64;
        }
        if kill {
            return Err(Cut::Killed);
        }
        out.sync_all()
            .map_err(|why| Cut::Target(format!("{}: {why}", tmp.display())))?;
        drop(out);
        match fs::symlink_metadata(&dest) {
            Err(why) if why.kind() == io::ErrorKind::NotFound => {}
            Ok(_) => {
                return Err(Cut::Target(format!(
                    "{} already exists: a copy is never written over",
                    dest.display()
                )));
            }
            Err(why) => return Err(Cut::Target(format!("{}: {why}", dest.display()))),
        }
        fs::rename(&tmp, &dest).map_err(|why| {
            Cut::Target(format!("{} -> {}: {why}", tmp.display(), dest.display()))
        })?;
        self.copied += 1;
        Ok((hasher.finalize(), folder))
    }
}

/// The BLAKE3 of `source`'s first `len` bytes, or why it could not be read.
fn digest_of(source: &Path, len: u64) -> Result<[u8; 32], String> {
    let file = store::open_flags::open_read_no_follow(source)
        .map_err(|why| format!("{}: {why}", source.display()))?;
    let mut hasher = brutex_core::blake3::Hasher::new();
    let mut buffer = vec![0_u8; CHUNK];
    let mut reader = file.take(len);
    let mut read = 0_u64;
    loop {
        let n = reader
            .read(&mut buffer)
            .map_err(|why| format!("{}: {why}", source.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(buffer.get(..n).unwrap_or(&[]));
        read += n as u64;
    }
    if read != len {
        return Err(format!(
            "{} shrank while it was read: {len} byte(s) were listed",
            source.display()
        ));
    }
    Ok(hasher.finalize())
}

/// One run's state while it copies.
struct Copying<'a> {
    root: &'a Path,
    book: Book,
    ledger: Ledger,
    copier: Copier<'a>,
    ran: Ran,
    rate: (Instant, u64),
}

impl Copying<'_> {
    fn note_line(&mut self, line: &str) -> Result<(), Refusal> {
        self.ledger.line(line).map_err(|why| {
            refused(
                "target_unwritable",
                format!("{}: {why}", self.ledger.path.display()),
            )
        })
    }

    /// Copies one unit's files. A target fault stops the run.
    fn files(&mut self, found: &[Found]) -> Result<(), Refusal> {
        let run = self.copier.run;
        let mut records = Vec::new();
        let mut folders = HashSet::with_capacity(found.len());
        for file in found {
            let verdict = verdict(&mut self.book, file);
            let counts = self.ran.classes.entry(file.class).or_default();
            tally(counts, file, verdict);
            let copy = match verdict {
                Verdict::Unchanged => continue,
                Verdict::Touched(held) => match digest_of(&file.source, file.len) {
                    Ok(digest) if digest == held.digest => {
                        // THE SAME BYTES UNDER A NEWER TIME: a record over the
                        // copy already held, not a second copy.
                        counts.recorded_only += 1;
                        records.push((file, held.run, digest));
                        continue;
                    }
                    Ok(_) => true,
                    Err(why) => {
                        self.unreadable(&file.path, &why)?;
                        continue;
                    }
                },
                Verdict::New | Verdict::Changed => true,
            };
            if copy {
                match self.copier.copy(file) {
                    Ok((digest, folder)) => {
                        let counts = self.ran.classes.entry(file.class).or_default();
                        counts.copied += 1;
                        counts.copied_bytes = counts.copied_bytes.saturating_add(file.len);
                        folders.insert(folder);
                        records.push((file, run, digest));
                    }
                    Err(Cut::Source(why)) => self.unreadable(&file.path, &why)?,
                    Err(Cut::Target(why)) => return Err(refused("target_failed", why)),
                    Err(Cut::Killed) => {
                        // A KILL BETWEEN A COPY AND ITS RECORD, MID-APPEND: a
                        // torn record behind the half-written temporary.
                        let line = record_line(
                            run, file.class, file.len, file.mtime, &[0; 32], &file.path, None,
                        );
                        let torn = line.get(..line.len() / 2).unwrap_or("");
                        let _ = self.ledger.file.write_all(torn.as_bytes());
                        self.ledger.torn = true;
                        return Err(refused("killed", "killed by a test".to_owned()));
                    }
                }
            }
        }
        // EVERY FOLDER SYNCED BEFORE ANY RECORD NAMES A FILE IN IT: a record
        // never outlives its copy.
        for folder in &folders {
            sync_dir(folder)
                .map_err(|why| refused("target_failed", format!("{}: {why}", folder.display())))?;
        }
        for (file, holder, digest) in records {
            let line = record_line(
                holder,
                file.class,
                file.len,
                file.mtime,
                &digest,
                &file.path,
                file.unit.as_deref(),
            );
            self.note_line(&line)?;
            self.book.insert(
                &file.path,
                Held {
                    len: file.len,
                    mtime: file.mtime,
                    run: holder,
                    digest,
                    archive: file.archive(),
                },
            );
        }
        Ok(())
    }

    fn unreadable(&mut self, path: &[u8], why: &str) -> Result<(), Refusal> {
        let run = self.copier.run;
        self.ran
            .faults
            .unreadable
            .add(format!("{} ({})", shown(path), one_line(why)));
        crate::note(
            &telemetry::Event::warn(NOTES, "backup file unreadable")
                .with("path", shown(path).as_str())
                .with("why", why),
        );
        self.note_line(&format!(
            "unreadable {} run={run} why={}",
            encoded(path),
            encoded(why.as_bytes())
        ))
    }

    /// One month under its shared locks: `Ok(false)` when a writer holds it.
    fn month(&mut self, month: &Month) -> Result<bool, Refusal> {
        let run = self.copier.run;
        let Some(path) = path_of(month.vendor, &month.entry) else {
            return Ok(true);
        };
        let files = month_files(self.root, path);
        let held = match hold(&files.locks) {
            Ok(held) => held,
            Err(Ok(_)) => return Ok(false),
            Err(Err(why)) => {
                self.unreadable(month.unit.as_bytes(), &why)?;
                return Ok(true);
            }
        };
        // MEASURED UNDER THE LOCK: no writer moves the month from here on.
        let mut faults = Faults::default();
        let found = month_found(self.root, month, &files, &mut faults);
        for name in &faults.skipped.first {
            self.ran.faults.skipped.add(name.clone());
        }
        let Some(found) = found else {
            drop(held);
            self.ran.faults.missing.add(month.unit.clone());
            crate::note(
                &telemetry::Event::warn(NOTES, "backup month missing")
                    .with("month", month.unit.as_str()),
            );
            self.note_line(&format!(
                "missing {} run={run}",
                encoded(month.unit.as_bytes())
            ))?;
            return Ok(true);
        };
        let done = self.files(&found);
        drop(held);
        done?;
        if let Some(why) = &files.pointer_fault {
            self.unreadable(month.unit.as_bytes(), why)?;
        }
        self.throttle();
        Ok(true)
    }

    /// Every unit in order, then each busy month once more: still held, it is
    /// a `busy` line and the next run copies it.
    fn every(&mut self, units: &[Unit]) -> Result<(), Refusal> {
        let run = self.copier.run;
        let mut busy = Vec::new();
        for unit in units {
            match unit {
                Unit::Month(month) => {
                    if !self.month(month)? {
                        busy.push(month);
                    }
                }
                Unit::File(found) => {
                    self.files(std::slice::from_ref(found))?;
                    self.throttle();
                }
            }
        }
        for month in busy {
            if !self.month(month)? {
                self.ran.faults.busy.add(month.unit.clone());
                crate::note(
                    &telemetry::Event::warn(NOTES, "backup month busy")
                        .with("month", month.unit.as_str())
                        .with("run", run),
                );
                self.note_line(&format!(
                    "busy {} run={run}",
                    encoded(month.unit.as_bytes())
                ))?;
            }
        }
        Ok(())
    }

    fn throttle(&self) {
        let (started, bytes_per_s) = self.rate;
        if bytes_per_s == 0 {
            return;
        }
        let copied = self.ran.total().copied_bytes;
        let micros = u128::from(copied).saturating_mul(1_000_000) / u128::from(bytes_per_s);
        let owed = Duration::from_micros(u64::try_from(micros).unwrap_or(u64::MAX));
        if let Some(ahead) = owed.checked_sub(started.elapsed()) {
            std::thread::sleep(ahead);
        }
    }
}

fn copy(
    root: &Path,
    ask: &Ask,
    env: &Env<'_>,
    (dest, home, manifest): (&Path, &Path, &Path),
    (units, listed): (Vec<Unit>, Faults),
    planned: &Ran,
) -> Result<Ran, Refusal> {
    let unwritable = |why: io::Error, at: &Path| {
        refused("target_unwritable", format!("{}: {why}", at.display()))
    };
    match fs::create_dir(home) {
        Ok(()) => sync_dir(dest).map_err(|why| unwritable(why, dest))?,
        Err(why) if why.kind() == io::ErrorKind::AlreadyExists => {}
        Err(why) => return Err(unwritable(why, home)),
    }
    let lock_path = home.join(LOCK);
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .custom_flags(store::open_flags::O_NOFOLLOW)
        .open(&lock_path)
        .map_err(|why| unwritable(why, &lock_path))?;
    let _one = Flock::try_lock(lock_file, lock_path.clone()).map_err(|_| {
        refused(
            "target_busy",
            format!(
                "{} is held by another backup to this target; one run at a time, nothing was \
                 copied",
                lock_path.display()
            ),
        )
    })?;
    // UNDER THE LOCK THE BOOK IS READ AGAIN: a run that finished between the
    // plan and the lock is what this one builds on.
    let mut book =
        Book::load(manifest, false).map_err(|why| refused("manifest_unreadable", why))?;
    // ROOM FOR EVERY FILE THE PLAN LISTED: an insert during the copy never
    // rehashes the map, so no per-file step pays for the whole table (p99).
    book.held
        .reserve(usize::try_from(planned.total().listed).unwrap_or(usize::MAX));
    let run = now_s().max(book.newest_run.saturating_add(1));
    let ledger = Ledger::open(manifest, book.torn).map_err(|why| unwritable(why, manifest))?;
    let started = now_s();
    let mut state = Copying {
        root,
        book,
        ledger,
        copier: Copier {
            home,
            run,
            inject: env.inject,
            copied: 0,
            written: 0,
            known: HashSet::with_capacity(units.len()),
        },
        ran: Ran {
            run,
            dest: dest.to_path_buf(),
            not_covered: planned.not_covered.clone(),
            free: planned.free,
            ..Ran::default()
        },
        rate: (Instant::now(), ask.bytes_per_s),
    };
    state.ran.faults.unlisted = listed.unlisted.clone();
    state.ran.faults.unaddressed = listed.unaddressed.clone();
    state.ran.faults.skipped = listed.skipped.clone();
    state.ran.faults.unreadable = listed.unreadable.clone();
    state.ran.faults.missing = listed.missing.clone();
    crate::note(
        &telemetry::Event::info(TARGET, "backup started")
            .with("run", run)
            .with("dest", dest.display().to_string().as_str())
            .with("plan_copy_bytes", planned.total().copy_bytes),
    );
    state.note_line(&format!(
        "run {run} started={started} commit={} store={}",
        env.commit.unwrap_or("unstamped"),
        encoded(root.as_os_str().as_bytes())
    ))?;
    for name in &listed.unlisted.first {
        state.note_line(&format!(
            "unlisted {} run={run} why=census",
            encoded(name.as_bytes())
        ))?;
    }
    if let Err(mut stop) = state.every(&units) {
        // THE TARGET STOPPED THE RUN: what was copied is durable and recorded,
        // and the next run finishes the rest. No receipt: this was no copy. A
        // test's kill writes nothing after the cut, as a dead process would.
        if stop.word != "killed" {
            let line = format!("stopped {run} why={}", encoded(stop.why.as_bytes()));
            if let Err(why) = state.ledger.line(&line).and_then(|()| state.ledger.sync()) {
                let _ = write!(
                    stop.why,
                    "; its `stopped` line could not be appended to {} either ({why})",
                    state.ledger.path.display()
                );
            }
        }
        return Err(stop);
    }
    finish(root, env, state)
}

/// One receipt: the line a finished run appends to `ops/backups.log`
/// (D-7177). Its first two words are the runbook's `copied=` and `dest=`,
/// which `api`'s `last_backup` requires; every other word is `key=value`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    /// When the run finished, Unix seconds (written as a UTC instant).
    pub copied_at: u64,
    /// The target folder, canonical (written percent-encoded).
    pub dest: PathBuf,
    /// Bytes of the source archives the target holds.
    pub raw_bytes: u64,
    /// Bytes of the store files the target holds.
    pub store_bytes: u64,
    /// The run's id.
    pub run: u64,
    /// Whether every listed unit was copied or found unchanged.
    pub complete: bool,
    /// Paths the target holds a copy of (the manifest's file count).
    pub files: u64,
    /// Their bytes.
    pub bytes: u64,
    /// Files this run copied.
    pub copied_files: u64,
    /// Their bytes.
    pub copied_bytes: u64,
    /// Files found unchanged.
    pub unchanged: u64,
    /// Months a writer held at the end of the run.
    pub busy: u64,
    /// Months and archives named and not on disk.
    pub missing: u64,
    /// Files, folders and censuses that could not be read.
    pub unreadable: u64,
    /// The build's commit, or `None` for an unstamped build.
    pub commit: Option<String>,
}

impl Receipt {
    /// The line, without its newline.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "copied={} dest={} raw_bytes={} store_bytes={} by=cli-backup run={} {} commit={}",
            utc(self.copied_at),
            encoded(self.dest.as_os_str().as_bytes()),
            self.raw_bytes,
            self.store_bytes,
            self.run,
            self.words(),
            self.commit.as_deref().unwrap_or("unstamped"),
        )
    }

    /// The words the manifest's `end` line shares with the receipt.
    fn words(&self) -> String {
        format!(
            "state={} files={} bytes={} copied_files={} copied_bytes={} unchanged={} busy={} missing={} unreadable={}",
            if self.complete { "complete" } else { "partial" },
            self.files,
            self.bytes,
            self.copied_files,
            self.copied_bytes,
            self.unchanged,
            self.busy,
            self.missing,
            self.unreadable,
        )
    }
}

fn finish(root: &Path, env: &Env<'_>, mut state: Copying<'_>) -> Result<Ran, Refusal> {
    let run = state.copier.run;
    let total = state.ran.total();
    let faults = &state.ran.faults;
    let complete = state.ran.complete();
    let mut receipt = Receipt {
        copied_at: 0,
        dest: state.ran.dest.clone(),
        raw_bytes: state.book.raw_bytes,
        store_bytes: state.book.store_bytes,
        run,
        complete,
        files: state.book.files,
        bytes: state.book.bytes,
        copied_files: total.copied,
        copied_bytes: total.copied_bytes,
        unchanged: total.unchanged + total.recorded_only,
        busy: faults.busy.count,
        missing: faults.missing.count,
        unreadable: faults.unreadable.count + faults.unlisted.count,
        commit: env.commit.map(str::to_owned),
    };
    state.note_line(&format!("end {run} {}", receipt.words()))?;
    state.ledger.sync().map_err(|why| {
        refused(
            "target_unwritable",
            format!("{}: {why}", state.ledger.path.display()),
        )
    })?;
    receipt.copied_at = now_s();
    let receipt = receipt.line();
    append_line(root, RECEIPTS, &receipt).map_err(|why| refused("receipt_unwritable", why))?;
    state.ran.files = state.book.files;
    state.ran.bytes = state.book.bytes;
    state.ran.probes = state.book.probes;
    state.ran.receipt = Some(receipt);
    let event = if complete {
        telemetry::Event::info(TARGET, "backup finished")
    } else {
        telemetry::Event::error(TARGET, "backup finished")
    };
    crate::note(
        &event
            .with("run", run)
            .with("copied_files", total.copied)
            .with("copied_bytes", total.copied_bytes)
            .with("busy", state.ran.faults.busy.count)
            .with("missing", state.ran.faults.missing.count)
            .with("complete", u64::from(complete)),
    );
    Ok(state.ran)
}

/// Appends `line` to the store log `rel` (made with its folder if absent),
/// ending a torn tail first, and syncs it. The log must be a regular file.
///
/// # Errors
///
/// A log that is not a regular file, or a write the disk refused.
pub(crate) fn append_line(root: &Path, rel: &str, line: &str) -> Result<PathBuf, String> {
    let path = root.join(rel);
    let folder = path
        .parent()
        .ok_or_else(|| format!("{} has no folder", path.display()))?;
    match fs::symlink_metadata(folder) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return Err(format!("{} is not a directory", folder.display())),
        Err(why) if why.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(folder).map_err(|why| format!("{}: {why}", folder.display()))?;
            sync_dir(root).map_err(|why| format!("{}: {why}", root.display()))?;
        }
        Err(why) => return Err(format!("{}: {why}", folder.display())),
    }
    match fs::symlink_metadata(&path) {
        Ok(meta) if !meta.is_file() => {
            return Err(format!(
                "{} is not a regular file; nothing was appended",
                path.display()
            ));
        }
        _ => {}
    }
    let mut file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .custom_flags(store::open_flags::O_NOFOLLOW_NONBLOCK)
        .open(&path)
        .map_err(|why| format!("{}: {why}", path.display()))?;
    let len = file
        .metadata()
        .map_err(|why| format!("{}: {why}", path.display()))?
        .len();
    let mut text = String::with_capacity(line.len() + 2);
    if len > 0 {
        let mut last = [0_u8; 1];
        file.read_exact_at(&mut last, len - 1)
            .map_err(|why| format!("{}: {why}", path.display()))?;
        if last != *b"\n" {
            // A TORN TAIL IS ENDED: this line is never joined onto it.
            text.push('\n');
        }
    }
    text.push_str(line);
    text.push('\n');
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_data())
        .map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(path)
}

fn one_line(text: &str) -> String {
    text.replace(['\n', '\r'], " ")
}

// ---------------------------------------------------------------- the drill

/// How the drill was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DrillAsk {
    /// The target folder a backup wrote.
    pub(crate) from: PathBuf,
    /// How many months to restore.
    pub(crate) months: u64,
}

/// One month's restore.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Restored {
    /// The month, as the store names its bars.
    pub(crate) unit: String,
    /// Files restored.
    pub(crate) files: u64,
    /// Their bytes.
    pub(crate) bytes: u64,
    /// Files the store changed since the copy: compared with the digest only.
    pub(crate) moved: u64,
    /// What did not compare, each named.
    pub(crate) failures: Vec<String>,
}

/// What a drill did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Drilled {
    /// The drill's id.
    pub(crate) id: u64,
    /// `pass`, `fail` or `incomplete`.
    pub(crate) result: &'static str,
    /// The months restored.
    pub(crate) months: Vec<Restored>,
    /// Months a writer held, skipped.
    pub(crate) busy: Vec<String>,
    /// How many months the manifest holds.
    pub(crate) held: u64,
    /// The scratch root the months were restored into.
    pub(crate) scratch: PathBuf,
    /// The drill line appended.
    pub(crate) line: String,
}

/// The kind of a month: its store path's segment (`bars/<vendor>/<exchange>/<segment>/...`).
fn kind_of(unit: &[u8]) -> Vec<u8> {
    unit.split(|byte| *byte == b'/')
        .nth(3)
        .unwrap_or_default()
        .to_vec()
}

/// The months a drill tries, in order: one of each kind in turn, each kind's
/// months rotated by `seed`, every month once.
pub(crate) fn drill_order(units: &[Vec<u8>], seed: u64) -> Vec<Vec<u8>> {
    let mut kinds: BTreeMap<Vec<u8>, Vec<&Vec<u8>>> = BTreeMap::new();
    for unit in units {
        kinds.entry(kind_of(unit)).or_default().push(unit);
    }
    let longest = kinds.values().map(Vec::len).max().unwrap_or(0);
    let mut out = Vec::with_capacity(units.len());
    for round in 0..longest {
        for list in kinds.values() {
            if round < list.len() {
                let len = list.len() as u64;
                let at = usize::try_from((seed % len + round as u64) % len).unwrap_or(0);
                if let Some(unit) = list.get(at) {
                    out.push((*unit).clone());
                }
            }
        }
    }
    out
}

/// The locks of a month whose files are `records`: its own and each
/// revision's.
fn drill_locks(root: &Path, unit: &[u8], records: &[Record]) -> Vec<PathBuf> {
    let unit = Path::new(std::ffi::OsStr::from_bytes(unit)).with_extension("lock");
    let mut out = vec![root.join(&unit)];
    let mut revisions = HashSet::with_capacity(records.len());
    for record in records {
        let path = Path::new(std::ffi::OsStr::from_bytes(&record.path));
        let mut parts = path.components();
        if parts.next() == Some(Component::Normal("bar-revisions-v1".as_ref()))
            && let Some(Component::Normal(ordinal)) = parts.next()
            && revisions.insert(ordinal.to_os_string())
        {
            out.push(root.join("bar-revisions-v1").join(ordinal).join(&unit));
        }
    }
    out
}

/// Whether the two files hold the same bytes.
fn same_bytes(restored: &Path, held: &Path) -> Result<bool, String> {
    let open = |path: &Path| {
        store::open_flags::open_read_no_follow(path)
            .map_err(|why| format!("{}: {why}", path.display()))
    };
    let (mut left, mut right) = (open(restored)?, open(held)?);
    let (mut ours, mut theirs) = (vec![0_u8; CHUNK], vec![0_u8; CHUNK]);
    loop {
        let got = read_full(&mut left, &mut ours)
            .map_err(|why| format!("{}: {why}", restored.display()))?;
        let want = read_full(&mut right, &mut theirs)
            .map_err(|why| format!("{}: {why}", held.display()))?;
        if got != want || ours.get(..got) != theirs.get(..want) {
            return Ok(false);
        }
        if got == 0 {
            return Ok(true);
        }
    }
}

fn read_full(file: &mut File, buffer: &mut [u8]) -> io::Result<usize> {
    let mut at = 0;
    while at < buffer.len() {
        let n = file.read(buffer.get_mut(at..).unwrap_or(&mut []))?;
        if n == 0 {
            break;
        }
        at += n;
    }
    Ok(at)
}

/// Restores one month into `scratch` and compares it.
fn drill_month(
    root: &Path,
    home: &Path,
    scratch: &Path,
    unit: &[u8],
    records: &[Record],
) -> Option<Restored> {
    let locks = drill_locks(root, unit, records);
    let mut restored = Restored {
        unit: shown(unit),
        ..Restored::default()
    };
    let _held = match hold(&locks) {
        Ok(held) => held,
        Err(Ok(_)) => return None,
        Err(Err(why)) => {
            restored.failures.push(format!("the month's lock: {why}"));
            return Some(restored);
        }
    };
    for record in records {
        let rel = Path::new(std::ffi::OsStr::from_bytes(&record.path));
        let copy = home
            .join(RUNS)
            .join(record.run.to_string())
            .join("store")
            .join(rel);
        let into = scratch.join(rel);
        let fail = |restored: &mut Restored, why: String| {
            restored.failures.push(format!("{}: {why}", rel.display()));
        };
        if let Some(folder) = into.parent()
            && let Err(why) = fs::create_dir_all(folder)
        {
            fail(&mut restored, format!("the scratch folder: {why}"));
            continue;
        }
        let digest = match restore_one(&copy, &into, record.len) {
            Ok(digest) => digest,
            Err(why) => {
                fail(&mut restored, why);
                continue;
            }
        };
        restored.files += 1;
        restored.bytes = restored.bytes.saturating_add(record.len);
        if digest != record.digest {
            fail(
                &mut restored,
                format!(
                    "the copy's BLAKE3 {} is not the manifest's {}",
                    hex(&digest),
                    hex(&record.digest)
                ),
            );
            continue;
        }
        let in_store = root.join(rel);
        match stat(&in_store) {
            Stat::File(len, mtime) if len == record.len && mtime == record.mtime => {
                match same_bytes(&into, &in_store) {
                    Ok(true) => {}
                    Ok(false) => fail(
                        &mut restored,
                        "the restored bytes differ from the store's".to_owned(),
                    ),
                    Err(why) => fail(&mut restored, why),
                }
            }
            Stat::File(..) => restored.moved += 1,
            Stat::Absent => fail(&mut restored, "absent from the store".to_owned()),
            Stat::Other(why) => fail(&mut restored, format!("in the store: {why}")),
        }
    }
    Some(restored)
}

/// Copies `copy` to `into` (a new file), returning its BLAKE3.
fn restore_one(copy: &Path, into: &Path, len: u64) -> Result<[u8; 32], String> {
    let mut source = store::open_flags::open_read_no_follow(copy).map_err(|why| {
        if why.kind() == io::ErrorKind::NotFound {
            format!("the copy {} is absent from the target", copy.display())
        } else {
            format!("{}: {why}", copy.display())
        }
    })?;
    let actual = source
        .metadata()
        .map_err(|why| format!("{}: {why}", copy.display()))?
        .len();
    if actual != len {
        return Err(format!(
            "the copy {} holds {actual} byte(s) and the manifest says {len}",
            copy.display()
        ));
    }
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(into)
        .map_err(|why| format!("{}: {why}", into.display()))?;
    let mut hasher = brutex_core::blake3::Hasher::new();
    let mut buffer = vec![0_u8; CHUNK];
    loop {
        let n = source
            .read(&mut buffer)
            .map_err(|why| format!("{}: {why}", copy.display()))?;
        if n == 0 {
            break;
        }
        let chunk = buffer.get(..n).unwrap_or(&[]);
        hasher.update(chunk);
        out.write_all(chunk)
            .map_err(|why| format!("{}: {why}", into.display()))?;
    }
    Ok(hasher.finalize())
}

/// Restores up to `asked` months in [`drill_order`], a batch at a time, the
/// months of a batch side by side; a month a writer holds is skipped and the
/// next one tried. Sets the drill's result.
fn restore_months(
    store: &Path,
    home: &Path,
    units: &BTreeMap<Vec<u8>, Vec<Record>>,
    asked: u64,
    drilled: &mut Drilled,
) {
    let names: Vec<Vec<u8>> = units.keys().cloned().collect();
    let order = drill_order(&names, drilled.id);
    let wanted = usize::try_from(asked).unwrap_or(usize::MAX);
    let mut at = 0;
    while drilled.months.len() < wanted && at < order.len() {
        let need = wanted - drilled.months.len();
        let batch = order
            .get(at..order.len().min(at.saturating_add(need)))
            .unwrap_or(&[]);
        at += batch.len();
        // EACH MONTH OF A BATCH reads its own copies and writes its own scratch
        // files, so they run side by side.
        let scratch = &drilled.scratch;
        let outcomes: Vec<(Vec<u8>, Option<Restored>)> = batch
            .par_iter()
            .map(|unit| {
                let records = units.get(unit).map_or(&[][..], Vec::as_slice);
                (
                    unit.clone(),
                    drill_month(store, home, scratch, unit, records),
                )
            })
            .collect();
        for (unit, outcome) in outcomes {
            match outcome {
                Some(restored) => drilled.months.push(restored),
                None => drilled.busy.push(shown(&unit)),
            }
        }
    }
    let failed = drilled.months.iter().any(|m| !m.failures.is_empty());
    drilled.result = if failed {
        "fail"
    } else if drilled.months.len() < wanted {
        "incomplete"
    } else {
        "pass"
    };
}

/// One drill line: what `backup-drill` appends to `ops/backup-drills.log`
/// (D-7178), which `/disk.json` serves as the last drill. Its first word is
/// `drilled=<UTC>`; every word is `key=value`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrillLine {
    /// When the drill finished, Unix seconds (written as a UTC instant).
    pub drilled_at: u64,
    /// The target folder, canonical.
    pub from: PathBuf,
    /// The drill's id.
    pub drill: u64,
    /// `pass`, `fail` or `incomplete`.
    pub result: String,
    /// Months restored and compared.
    pub months: u64,
    /// Months asked for.
    pub asked: u64,
    /// Months the manifest holds.
    pub held: u64,
    /// Files restored.
    pub files: u64,
    /// Their bytes.
    pub bytes: u64,
    /// Files the store changed since the copy: compared with the digest only.
    pub moved: u64,
    /// Files that did not compare.
    pub failed: u64,
    /// Months a writer held, skipped.
    pub busy: u64,
    /// The scratch folder the months were restored into.
    pub scratch: PathBuf,
    /// The build's commit, or `None` for an unstamped build.
    pub commit: Option<String>,
    /// The months restored, by store path.
    pub units: Vec<String>,
}

impl DrillLine {
    /// The line, without its newline: the months by name as far as
    /// [`MAX_LINE`] allows, the rest counted as `more=`.
    #[must_use]
    pub fn line(&self) -> String {
        let mut line = format!(
            "drilled={} from={} drill={} result={} months={} asked={} held={} files={} bytes={} moved={} failed={} busy={} scratch={} commit={}",
            utc(self.drilled_at),
            encoded(self.from.as_os_str().as_bytes()),
            self.drill,
            self.result,
            self.months,
            self.asked,
            self.held,
            self.files,
            self.bytes,
            self.moved,
            self.failed,
            self.busy,
            encoded(self.scratch.as_os_str().as_bytes()),
            self.commit.as_deref().unwrap_or("unstamped"),
        );
        let mut listed = 0;
        for (n, unit) in self.units.iter().enumerate() {
            let word = format!(
                "{}{}",
                if n == 0 { " month=" } else { "," },
                encoded(unit.as_bytes())
            );
            // ROOM IS KEPT FOR ` more=<n>`, at most 26 bytes.
            if line.len() + word.len() + 26 > MAX_LINE {
                break;
            }
            line.push_str(&word);
            listed += 1;
        }
        if listed < self.units.len() {
            let _ = write!(line, " more={}", self.units.len() - listed);
        }
        line
    }
}

/// The drill line of `drilled`.
fn drill_line(drilled: &Drilled, from: &Path, asked: u64, commit: Option<&str>) -> String {
    let sum = |of: fn(&Restored) -> u64| drilled.months.iter().map(of).sum::<u64>();
    DrillLine {
        drilled_at: now_s(),
        from: from.to_path_buf(),
        drill: drilled.id,
        result: drilled.result.to_owned(),
        months: drilled.months.len() as u64,
        asked,
        held: drilled.held,
        files: sum(|m| m.files),
        bytes: sum(|m| m.bytes),
        moved: sum(|m| m.moved),
        failed: sum(|m| m.failures.len() as u64),
        busy: drilled.busy.len() as u64,
        scratch: drilled.scratch.clone(),
        commit: commit.map(str::to_owned),
        units: drilled.months.iter().map(|m| m.unit.clone()).collect(),
    }
    .line()
}

/// The drill's target and store root, canonical, once the target is a folder
/// holding a manifest.
fn drill_target(root: &Path, asked: &Path) -> Result<(PathBuf, PathBuf), Refusal> {
    match fs::metadata(asked) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            return Err(refused(
                "not_a_directory",
                format!("{} is not a directory", asked.display()),
            ));
        }
        Err(why) => {
            return Err(refused(
                "target_missing",
                format!(
                    "{} cannot be read ({why}): is the second drive attached and mounted?",
                    asked.display()
                ),
            ));
        }
    }
    let from = fs::canonicalize(asked)
        .map_err(|why| refused("target_missing", format!("{}: {why}", asked.display())))?;
    let store = fs::canonicalize(root)
        .map_err(|why| refused("store_unreadable", format!("{}: {why}", root.display())))?;
    let manifest = from.join(HOME).join(MANIFEST);
    if fs::symlink_metadata(&manifest).is_err() {
        return Err(refused(
            "no_manifest",
            format!(
                "{} holds no backup manifest ({}): run `cli backup --to {}` first",
                from.display(),
                manifest.display(),
                from.display()
            ),
        ));
    }
    Ok((from, store))
}

/// The drill: `ask.months` months restored into a new folder under
/// `scratch_parent` and compared with the store and the manifest.
///
/// # Errors
///
/// A refusal before any month was restored.
pub(crate) fn drill(
    root: &Path,
    ask: &DrillAsk,
    (scratch_parent, process): (&Path, u32),
    commit: Option<&str>,
) -> Result<Drilled, Refusal> {
    let (from, store) = drill_target(root, &ask.from)?;
    let home = from.join(HOME);
    let manifest = home.join(MANIFEST);
    // A BACKUP WRITING TO THIS TARGET holds its lock exclusively: a drill
    // never reads a manifest mid-run.
    let lock_path = home.join(LOCK);
    let _shared = match File::open(&lock_path) {
        Ok(file) => Some(
            Flock::try_lock_shared(file, lock_path.clone()).map_err(|_| {
                refused(
                    "target_busy",
                    format!("{} is held by a backup to this target", lock_path.display()),
                )
            })?,
        ),
        Err(_) => None,
    };
    let book = Book::load(&manifest, true).map_err(|why| refused("manifest_unreadable", why))?;
    let units = book.units();
    if units.is_empty() {
        return Err(refused(
            "no_month",
            format!("{} records no month to restore", manifest.display()),
        ));
    }
    let parent = fs::canonicalize(scratch_parent).map_err(|why| {
        refused(
            "scratch_unavailable",
            format!("{}: {why}", scratch_parent.display()),
        )
    })?;
    if parent.starts_with(&store) || parent.starts_with(&from) {
        return Err(refused(
            "scratch_inside",
            format!(
                "the scratch folder's parent {} is inside the store or the target; a drill \
                 restores beside neither",
                parent.display()
            ),
        ));
    }
    let id = now_s();
    let scratch = parent.join(format!("brutex-backup-drill-{id}-{process}"));
    fs::create_dir(&scratch).map_err(|why| {
        refused(
            "scratch_unavailable",
            format!("{}: {why}", scratch.display()),
        )
    })?;
    crate::note(
        &telemetry::Event::info(TARGET, "backup drill started")
            .with("drill", id)
            .with("from", from.display().to_string().as_str())
            .with("months", ask.months),
    );
    let mut drilled = Drilled {
        id,
        held: units.len() as u64,
        scratch,
        ..Drilled::default()
    };
    restore_months(&store, &home, &units, ask.months, &mut drilled);
    let failed: u64 = drilled.months.iter().map(|m| m.failures.len() as u64).sum();
    let line = drill_line(&drilled, &from, ask.months, commit);
    append_line(root, DRILLS, &line).map_err(|why| refused("drill_log_unwritable", why))?;
    drilled.line = line;
    let event = if drilled.result == "pass" {
        telemetry::Event::info(TARGET, "backup drill finished")
    } else {
        telemetry::Event::error(TARGET, "backup drill finished")
    };
    crate::note(
        &event
            .with("drill", id)
            .with("result", drilled.result)
            .with("months", drilled.months.len() as u64)
            .with("failed", failed),
    );
    Ok(drilled)
}

// ---------------------------------------------------------------- the verbs

/// The backup verb's usage, said with every refusal of its arguments.
const TAKES: &str = "backup takes --to <dir> [--plan] [--mib-per-s N]";

/// The drill verb's usage.
const DRILL_TAKES: &str = "backup-drill takes --from <dir> [--months N]";

/// `arguments` as an [`Ask`], or the misuse.
pub(crate) fn parse(arguments: &[&str]) -> Result<Ask, String> {
    let mut to = None;
    let mut plan = false;
    let mut mib = None;
    let mut words = arguments.iter();
    while let Some(word) = words.next() {
        match *word {
            "--to" if to.is_none() => {
                to = Some(
                    words
                        .next()
                        .filter(|v| !v.is_empty())
                        .map(PathBuf::from)
                        .ok_or_else(|| format!("--to takes a folder — {TAKES}"))?,
                );
            }
            "--plan" if !plan => plan = true,
            "--mib-per-s" if mib.is_none() => {
                mib = Some(
                    words
                        .next()
                        .and_then(|v| v.parse::<u64>().ok())
                        .ok_or_else(|| format!("--mib-per-s takes a whole number — {TAKES}"))?,
                );
            }
            other => return Err(format!("{other:?} — {TAKES}")),
        }
    }
    let to = to.ok_or_else(|| format!("--to is required — {TAKES}"))?;
    Ok(Ask {
        to,
        plan,
        bytes_per_s: mib.unwrap_or(0).saturating_mul(1024 * 1024),
    })
}

/// `arguments` as a [`DrillAsk`], or the misuse.
pub(crate) fn parse_drill(arguments: &[&str]) -> Result<DrillAsk, String> {
    let mut from = None;
    let mut months = None;
    let mut words = arguments.iter();
    while let Some(word) = words.next() {
        match *word {
            "--from" if from.is_none() => {
                from = Some(
                    words
                        .next()
                        .filter(|v| !v.is_empty())
                        .map(PathBuf::from)
                        .ok_or_else(|| format!("--from takes a folder — {DRILL_TAKES}"))?,
                );
            }
            "--months" if months.is_none() => {
                months = Some(
                    words
                        .next()
                        .and_then(|v| v.parse::<u64>().ok())
                        .filter(|n| *n > 0)
                        .ok_or_else(|| {
                            format!("--months takes a whole number above 0 — {DRILL_TAKES}")
                        })?,
                );
            }
            other => return Err(format!("{other:?} — {DRILL_TAKES}")),
        }
    }
    Ok(DrillAsk {
        from: from.ok_or_else(|| format!("--from is required — {DRILL_TAKES}"))?,
        months: months.unwrap_or(DRILL_MONTHS),
    })
}

fn class_lines(out: &mut String, ran: &Ran) {
    for (class, c) in &ran.classes {
        let _ = writeln!(
            out,
            "  {class:<16} listed={} bytes={} unchanged={} new={} changed={} to_copy_bytes={} copied={} copied_bytes={} recorded_only={}",
            c.listed,
            c.listed_bytes,
            c.unchanged,
            c.new,
            c.changed,
            c.copy_bytes,
            c.copied,
            c.copied_bytes,
            c.recorded_only
        );
    }
}

fn fault_lines(out: &mut String, faults: &Faults) {
    for (word, named, what) in [
        (
            "BUSY",
            &faults.busy,
            "month(s) a writer held: not copied, copied by the next run",
        ),
        ("MISSING", &faults.missing, "named and not on disk"),
        (
            "UNREADABLE",
            &faults.unreadable,
            "could not be read: not copied",
        ),
        (
            "UNLISTED",
            &faults.unlisted,
            "census(es) would not load: their months were not listed",
        ),
        (
            "NOT ADDRESSED",
            &faults.unaddressed,
            "census entr(ies) name no legal store path",
        ),
        (
            "SKIPPED",
            &faults.skipped,
            "entr(ies) are not regular files (a link, a FIFO): not followed",
        ),
    ] {
        if named.count > 0 {
            let _ = writeln!(out, "  {word:<15} {} {what}: {}", named.count, named.text());
        }
    }
}

/// The backup verb, with its store root and machine passed in.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    env: &Env<'_>,
    out: &mut String,
) -> u8 {
    let ask = match parse(arguments) {
        Ok(ask) => ask,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            return MISUSED;
        }
    };
    let ran = store()
        .map_err(|why| refused("store_unavailable", why))
        .and_then(|root| backup(&root, &ask, env).map(|ran| (root, ran)));
    let (root, ran) = match ran {
        Ok(done) => done,
        Err(refusal) => {
            crate::note(
                &telemetry::Event::error(TARGET, "backup refused")
                    .with("refusal", refusal.word)
                    .with("why", refusal.why.as_str()),
            );
            let _ = writeln!(out, "BACKUP REFUSED ({}): {}", refusal.word, refusal.why);
            return FAILED;
        }
    };
    let total = ran.total();
    if ask.plan {
        let _ = writeln!(
            out,
            "BACKUP PLAN -- to {}: {} file(s) listed, {} byte(s); to copy {} file(s), {} byte(s) ({} new, {} changed); {} unchanged; the target holds {} file(s), {} byte(s), and has {} byte(s) free -- the copy fits; nothing was written",
            ran.dest.display(),
            total.listed,
            total.listed_bytes,
            total.new + total.changed,
            total.copy_bytes,
            total.new,
            total.changed,
            total.unchanged,
            ran.files,
            ran.bytes,
            ran.free
        );
    } else {
        let _ = writeln!(
            out,
            "BACKUP -- run {} to {}: copied {} file(s), {} byte(s); {} unchanged; the target holds {} file(s), {} byte(s) -- {}",
            ran.run,
            ran.dest.join(HOME).display(),
            total.copied,
            total.copied_bytes,
            total.unchanged + total.recorded_only,
            ran.files,
            ran.bytes,
            if ran.complete() {
                "complete"
            } else {
                "PARTIAL: run again once what is named below is resolved"
            }
        );
    }
    class_lines(out, &ran);
    fault_lines(out, &ran.faults);
    if !ran.not_covered.is_empty() {
        let _ = writeln!(
            out,
            "  NOT COVERED     the store root's {} not copied by this verb (docs/36-ops-runbook.md §4): {}",
            ran.not_covered.len(),
            ran.not_covered.join(", ")
        );
    }
    if let Some(receipt) = &ran.receipt {
        let _ = writeln!(
            out,
            "  receipt         {}: {receipt}",
            root.join(RECEIPTS).display()
        );
        let _ = writeln!(
            out,
            "  manifest        {}",
            ran.dest.join(HOME).join(MANIFEST).display()
        );
    }
    if ask.plan || ran.complete() {
        OK
    } else {
        FAILED
    }
}

/// The drill verb, with its store root and scratch parent passed in.
pub(crate) fn drill_command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    scratch: (&Path, u32),
    commit: Option<&str>,
    out: &mut String,
) -> u8 {
    let ask = match parse_drill(arguments) {
        Ok(ask) => ask,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            return MISUSED;
        }
    };
    let drilled = store()
        .map_err(|why| refused("store_unavailable", why))
        .and_then(|root| drill(&root, &ask, scratch, commit).map(|d| (root, d)));
    let (root, drilled) = match drilled {
        Ok(done) => done,
        Err(refusal) => {
            crate::note(
                &telemetry::Event::error(TARGET, "backup drill refused")
                    .with("refusal", refusal.word)
                    .with("why", refusal.why.as_str()),
            );
            let _ = writeln!(
                out,
                "BACKUP DRILL REFUSED ({}): {}",
                refusal.word, refusal.why
            );
            return FAILED;
        }
    };
    let _ = writeln!(
        out,
        "BACKUP DRILL -- drill {} from {}: {}: {} of {} month(s) restored (the manifest holds {}) into {}",
        drilled.id,
        ask.from.display(),
        drilled.result.to_uppercase(),
        drilled.months.len(),
        ask.months,
        drilled.held,
        drilled.scratch.display()
    );
    for month in &drilled.months {
        let _ = writeln!(
            out,
            "  month           {} files={} bytes={} moved={} {}",
            month.unit,
            month.files,
            month.bytes,
            month.moved,
            if month.failures.is_empty() {
                "byte-equal with the store and the manifest digests"
            } else {
                "FAILED"
            }
        );
        for failure in &month.failures {
            let _ = writeln!(out, "  FAILED          {failure}");
        }
    }
    for unit in &drilled.busy {
        let _ = writeln!(
            out,
            "  BUSY            {unit}: a writer held it; not restored"
        );
    }
    let _ = writeln!(
        out,
        "  drill line      {}: {}",
        root.join(DRILLS).display(),
        drilled.line
    );
    let _ = writeln!(
        out,
        "  scratch root    {} (a new folder each drill, outside the store and the target, left for inspection)",
        drilled.scratch.display()
    );
    if drilled.result == "pass" { OK } else { FAILED }
}

/// The machine's own device and free-space answers.
fn device(path: &Path) -> io::Result<u64> {
    fs::metadata(path).map(|meta| meta.dev())
}

fn free(path: &Path) -> Result<u64, String> {
    pull::gdfl_import::volume_space(path).map(|space| space.free)
}

/// `cli backup` against the CLI's store root: a writer (`store_write`), so an
/// unstamped build is refused before it reads the store, unless `--plan`.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    let store = crate::store_write::root_for("backup", arguments);
    let env = Env {
        commit: crate::commit_stamp(),
        device: &device,
        free: &free,
        inject: Inject::default(),
    };
    command_with(arguments, &store, &env, out)
}

/// `cli backup-drill` against the CLI's store root, restoring under the
/// system's temporary folder: a writer of `ops/backup-drills.log`.
pub(crate) fn drill_command(arguments: &[&str], out: &mut String) -> u8 {
    let store = crate::store_write::root_for("backup-drill", arguments);
    drill_command_with(
        arguments,
        &store,
        (&std::env::temp_dir(), std::process::id()),
        crate::commit_stamp(),
        out,
    )
}

#[cfg(test)]
#[path = "backup_tests.rs"]
mod tests;
