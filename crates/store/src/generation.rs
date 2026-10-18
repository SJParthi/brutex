//! Which generation of a bar month the store serves. D-3350, WP-13c.
//!
//! # Why this exists
//!
//! A month is append-only (`CLAUDE.md` §3 rule 8): `BarFile::append` takes a
//! batch that follows the last held bar, or one already held, and refuses a
//! stamp inside the held range that the month never held. So a session the
//! month lacks BEFORE its last bar can never be added by an append. Measured
//! on the operator's store (`state/gdfl-board/zerodha-holes.md`, DB-N20):
//! Zerodha NIFTY 1min lacks 2020-12-01..15 because 2020-12 already holds
//! 12-16 on, and the same 35 sessions plus seven Muhurats are missing from 190
//! of the 330 Zerodha 1min series holding bars (measured 10 Oct 2026, WP-13c).
//! Re-pulling them is refused by the store.
//!
//! [`crate::repair`] could already publish a complete successor of a month —
//! the held bars plus the filled sessions — under `bar-revisions-v1/<n>/`,
//! never touching the original. Nothing read it. This module is the one
//! authority that says which of a month's generations every reader opens.
//!
//! # The pointer
//!
//! `<root>/bar-generations-v1/bars/<vendor>/…/<yyyy-mm>.gen-v1`, the month's
//! own [`StorePath`] rendered under a separate root, so it is never under
//! `<root>/bars` (a test server's shadow links that directory to the real
//! store; nothing here may write through it). Append-only fixed records of
//! [`RECORD_LEN`] bytes, each naming one published revision; **the last valid
//! record is the generation served**. No record, or no file, is generation 0:
//! the original `.bin`.
//!
//! | Offset | Bytes | Field |
//! | --- | --- | --- |
//! | 0 | 8 | `BRXGEN01` |
//! | 8 | 4 | revision ordinal served, `u32` LE, `1..=`[`crate::repair::MAX_REVISIONS`] |
//! | 12 | 4 | the ordinal it replaced, `0` for the original |
//! | 16 | 8 | the revision header's `n_valid` |
//! | 24 | 8 | its `first_ts_micros` |
//! | 32 | 8 | its `last_ts_micros` |
//! | 40 | 8 | its header `generation` |
//! | 48 | 8 | when it was promoted, UTC micros |
//! | 56 | 4 | zero |
//! | 60 | 4 | CRC-32C of bytes 0..60 followed by the record's index, `u64` LE |
//!
//! # Atomic for every reader
//!
//! A revision is complete, synced and receipted before it is promoted
//! ([`crate::repair::publish`]). Promotion is ONE record written at the next
//! aligned offset and synced. A reader that resolved before it opens the old
//! generation, which stays byte-identical; a reader after it opens the new one;
//! a torn record fails its CRC and the reader serves the record before it,
//! with a `store.generation` warning — the promotion it would have made never
//! completed. Every `BarFile::open_existing` resolves here, so bars, windows,
//! exports, the census recount, calendars, gaps and day flags all read the same
//! generation.
//!
//! # Writers
//!
//! `BarFile::open_or_create` REFUSES a month this module says is served from a
//! revision ([`StoreError::Superseded`]): an append to the original would land
//! in a generation no reader opens. A month served from a revision grows only
//! by a further repair.
//!
//! # Cost
//!
//! A read is one `open` that is `ENOENT` for every month never repaired — the
//! whole store today — and otherwise one `fstat` and one 64-byte positional
//! read; a torn tail adds one read per skipped record, at most
//! [`MAX_RECORDS`]. Promotion is one 64-byte write and two syncs. No scan of
//! the store, of a directory, or of the month's bars. UNVERIFIED as a gated
//! measurement; the shadow-server timings are in the WP-13c report.

use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind};
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};

use crate::crc::crc32c_split;
use crate::file::{BarFile, StoreError};
use crate::flock::Flock;
use crate::header::Header;
use crate::path::{FileKind, StorePath};
use crate::repair::{self, RepairError, Revision, RevisionReader};

/// The directory under the store root every pointer lives below.
pub const ROOT: &str = "bar-generations-v1";
/// The pointer's extension, beside the month's own stem.
pub const EXTENSION: &str = "gen-v1";
/// Bytes of one pointer record, as a length.
const RECORD: usize = 64;
/// Bytes of one pointer record.
pub const RECORD_LEN: u64 = RECORD as u64;
/// The magic at byte 0 of every record.
pub const MAGIC: [u8; 8] = *b"BRXGEN01";
/// The most records one month's pointer may hold: one per revision ordinal.
pub const MAX_RECORDS: u64 = repair::MAX_REVISIONS as u64;
/// The longest pointer file read: every record, plus one torn tail.
const MAX_LEN: u64 = (MAX_RECORDS + 1) * RECORD_LEN;

/// One promotion: the revision a month is served from, and what it held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer {
    /// The revision served.
    pub revision: Revision,
    /// The ordinal it replaced; `0` is the original month.
    pub previous: u32,
    /// The revision header's committed record count.
    pub n_valid: u64,
    /// Its first committed stamp.
    pub first_ts_micros: i64,
    /// Its last committed stamp.
    pub last_ts_micros: i64,
    /// Its header generation.
    pub header_generation: u64,
    /// When the record was written, UTC micros.
    pub promoted_at_micros: i64,
    /// Which record of the pointer file this is.
    pub record: u64,
}

impl Pointer {
    /// Whether `header` is the header this pointer promoted. A revision that
    /// moved after promotion was written outside the repair protocol and is
    /// not served as if it were the promoted one.
    #[must_use]
    pub const fn describes(&self, header: &Header) -> bool {
        header.n_valid == self.n_valid
            && header.first_ts_micros == self.first_ts_micros
            && header.last_ts_micros == self.last_ts_micros
            && header.generation == self.header_generation
    }

    fn encode(&self) -> [u8; RECORD] {
        let mut bytes = [0u8; RECORD];
        let fields: [(usize, &[u8]); 8] = [
            (0, &MAGIC),
            (8, &self.revision.ordinal().to_le_bytes()),
            (12, &self.previous.to_le_bytes()),
            (16, &self.n_valid.to_le_bytes()),
            (24, &self.first_ts_micros.to_le_bytes()),
            (32, &self.last_ts_micros.to_le_bytes()),
            (40, &self.header_generation.to_le_bytes()),
            (48, &self.promoted_at_micros.to_le_bytes()),
        ];
        for (at, field) in fields {
            if let Some(slot) = bytes.get_mut(at..at + field.len()) {
                slot.copy_from_slice(field);
            }
        }
        let crc = seal(&bytes, self.record);
        if let Some(slot) = bytes.get_mut(60..64) {
            slot.copy_from_slice(&crc.to_le_bytes());
        }
        bytes
    }

    fn decode(bytes: &[u8; RECORD], record: u64) -> Option<Self> {
        let u32_at = |at: usize| -> Option<u32> {
            bytes
                .get(at..at + 4)
                .and_then(|b| b.try_into().ok())
                .map(u32::from_le_bytes)
        };
        let u64_at = |at: usize| -> Option<u64> {
            bytes
                .get(at..at + 8)
                .and_then(|b| b.try_into().ok())
                .map(u64::from_le_bytes)
        };
        let i64_at = |at: usize| -> Option<i64> {
            bytes
                .get(at..at + 8)
                .and_then(|b| b.try_into().ok())
                .map(i64::from_le_bytes)
        };
        if bytes.get(0..8) != Some(&MAGIC[..])
            || u32_at(56)? != 0
            || u32_at(60)? != seal(bytes, record)
        {
            return None;
        }
        Some(Self {
            revision: Revision::new(u32_at(8)?).ok()?,
            previous: u32_at(12)?,
            n_valid: u64_at(16)?,
            first_ts_micros: i64_at(24)?,
            last_ts_micros: i64_at(32)?,
            header_generation: u64_at(40)?,
            promoted_at_micros: i64_at(48)?,
            record,
        })
    }
}

/// The CRC-32C of a record's first 60 bytes followed by its index, so a
/// record copied to another slot does not verify there.
fn seal(bytes: &[u8; RECORD], record: u64) -> u32 {
    crc32c_split(bytes.get(0..60).unwrap_or(&[]), &record.to_le_bytes())
}

/// Which generation a month is served from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Served {
    /// The original `.bin` under the store root: no promotion was recorded.
    Original,
    /// A promoted repair revision.
    Revision(Pointer),
}

impl Served {
    /// The ordinal served: `0` for the original.
    #[must_use]
    pub const fn ordinal(self) -> u32 {
        match self {
            Self::Original => 0,
            Self::Revision(pointer) => pointer.revision.ordinal(),
        }
    }

    /// The root the served generation's files are rendered under: the store
    /// root itself, or `<root>/bar-revisions-v1/<n>`.
    #[must_use]
    pub fn root(self, root: &Path) -> PathBuf {
        match self {
            Self::Original => root.to_path_buf(),
            Self::Revision(pointer) => repair::revision_root(root, pointer.revision),
        }
    }
}

/// The pointer file of `path`'s month.
#[must_use]
pub fn pointer_path(root: &Path, path: StorePath<'_>) -> PathBuf {
    path.with_file(FileKind::Bars)
        .to_path_buf(&root.join(ROOT))
        .with_extension(EXTENSION)
}

/// The `.bin` a reader of `path` opens: the original, or the promoted
/// revision's. A cache keyed by a month's file must key by THIS path, so a
/// promotion is a different key and never a stale hit.
///
/// # Errors
///
/// Whatever [`served`] refuses.
///
/// # The form, too (SC-3, D-4965)
///
/// When the served generation holds the month SEALED, this is its `.sbar`,
/// the file the read door opens first (`BarFile::open_existing`), so a
/// month sealed after a cache entry was made is a new key — and a stamp of
/// the immutable sealed file — never a stale hit on the raw form's. One
/// `lstat` more than before; the raw `.bin` when no `.sbar` is there.
pub fn served_bars_path(root: &Path, path: StorePath<'_>) -> Result<PathBuf, StoreError> {
    let served = served(root, path)?;
    let served_root = served.root(root);
    let sealed = path
        .with_file(FileKind::SealedBars)
        .to_path_buf(&served_root);
    if fs::symlink_metadata(&sealed).is_ok_and(|meta| meta.is_file()) {
        return Ok(sealed);
    }
    Ok(path.with_file(FileKind::Bars).to_path_buf(&served_root))
}

/// Which generation of `path`'s month is served. Any non-bar path is served
/// from the original: only bar months have generations.
///
/// # Errors
///
/// [`StoreError::Generation`] for a pointer that is not a regular file, is
/// longer than [`MAX_RECORDS`] records, cannot be read, or holds records of
/// which none verifies — the month's served generation cannot then be told,
/// and serving the original instead would be the hidden fallback `CLAUDE.md`
/// §4 bans.
pub fn served(root: &Path, path: StorePath<'_>) -> Result<Served, StoreError> {
    if path.file() != FileKind::Bars {
        return Ok(Served::Original);
    }
    let pointer = pointer_path(root, path);
    let file = match crate::open_flags::open_read_no_follow(&pointer) {
        Ok(file) => file,
        Err(why) if why.kind() == ErrorKind::NotFound => return Ok(Served::Original),
        // A STORE ROOT THAT IS NOT A DIRECTORY holds no pointer and no month:
        // the original's own open names that, in the words every door uses.
        // A non-directory INSIDE the pointer tree is refused below.
        Err(why) if why.kind() == ErrorKind::NotADirectory && !root.is_dir() => {
            return Ok(Served::Original);
        }
        Err(why) => return Err(refused(&pointer, format!("cannot open it: {why}"))),
    };
    let meta = file
        .metadata()
        .map_err(|why| refused(&pointer, format!("cannot measure it: {why}")))?;
    if !meta.is_file() {
        return Err(refused(&pointer, "it is not a regular file".to_owned()));
    }
    let len = meta.len();
    if len > MAX_LEN {
        return Err(refused(
            &pointer,
            format!(
                "it is {len} bytes, past the {MAX_LEN} a pointer of {MAX_RECORDS} records can be"
            ),
        ));
    }
    let whole = len / RECORD_LEN;
    let mut skipped = 0u64;
    let mut at = whole;
    while at > 0 {
        at -= 1;
        let mut bytes = [0u8; RECORD];
        file.read_exact_at(&mut bytes, at * RECORD_LEN)
            .map_err(|why| refused(&pointer, format!("cannot read record {at}: {why}")))?;
        if let Some(found) = Pointer::decode(&bytes, at) {
            if skipped > 0 || len % RECORD_LEN != 0 {
                note_skipped(
                    &pointer,
                    skipped,
                    len % RECORD_LEN,
                    found.revision.ordinal(),
                );
            }
            return Ok(Served::Revision(found));
        }
        skipped += 1;
    }
    if whole == 0 {
        if len > 0 {
            note_skipped(&pointer, 0, len, 0);
        }
        return Ok(Served::Original);
    }
    Err(refused(
        &pointer,
        format!(
            "none of its {whole} record(s) verifies, so which generation the month serves cannot be told; nothing is served in its place"
        ),
    ))
}

fn refused(pointer: &Path, reason: String) -> StoreError {
    StoreError::Generation {
        path: pointer.to_path_buf(),
        reason,
    }
}

/// A pointer read that stepped over a torn tail or an unverifiable record.
/// Said once per read, never silently: the generation served is the last one
/// whose record verifies, which is the last promotion that completed.
fn note_skipped(pointer: &Path, records: u64, torn_bytes: u64, serving: u32) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(
            "store.generation",
            "pointer read stepped over records that do not verify",
        )
        .with(
            "pointer",
            telemetry::Value::Str(&pointer.display().to_string()),
        )
        .with("records_skipped", telemetry::Value::Uint(records))
        .with("torn_tail_bytes", telemetry::Value::Uint(torn_bytes))
        .with(
            "serving_revision",
            telemetry::Value::Uint(u64::from(serving)),
        ),
    );
}

/// What a promotion did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Promoted {
    /// A record was written; readers now open this revision.
    Promoted(Pointer),
    /// The revision was already the one served; nothing was written.
    Already(Pointer),
}

impl Promoted {
    /// The pointer now in force.
    #[must_use]
    pub const fn pointer(self) -> Pointer {
        match self {
            Self::Promoted(pointer) | Self::Already(pointer) => pointer,
        }
    }
}

/// Make `revision` the generation every reader of `path`'s month opens.
///
/// The revision must be complete ([`RevisionReader::open`] verifies its
/// receipt), and it must have been published FROM the generation served now —
/// its receipt's source header equals the served month's header — so a chain
/// of repairs can never skip one and drop the bars it added. Idempotent: the
/// revision already served is [`Promoted::Already`] and writes nothing.
///
/// # Errors
///
/// [`RepairError::StaleSource`] for a revision made from another generation;
/// [`RepairError::RevisionLimit`] for a pointer already holding
/// [`MAX_RECORDS`] records; the receipt's and the served month's own
/// refusals; [`StoreError::Locked`] (inside [`RepairError::Store`]) when another
/// promoter holds the pointer; and named I/O failures. A failure after the
/// record write began carries `publication_may_be_visible: true`.
pub fn promote(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    revision: Revision,
) -> Result<Promoted, RepairError> {
    if path.file() != FileKind::Bars {
        return Err(StoreError::NotABarPath { found: path.file() }.into());
    }
    let candidate = RevisionReader::open(root, path, symbol_id, revision)?;
    // THE ORIGINAL MONTH IS HELD SHARED UNTIL THE POINTER IS WRITTEN, so no
    // writer can append to it between the check below and the promotion: a
    // writer holding it refuses this promotion (`StoreError::Locked`), and one
    // that takes it afterwards finds the pointer under its own lock and is
    // refused (`BarFile::open_or_create`). Both locks are taken non-blocking,
    // so holding two cannot deadlock. Released when this returns.
    let _month = repair::shared_lock(path.with_file(FileKind::Lock).to_path_buf(root))?;
    let pointer_file = pointer_path(root, path);
    let directory = pointer_file
        .parent()
        .ok_or_else(|| RepairError::Incomplete(pointer_file.clone()))?
        .to_path_buf();
    // NO LINK BELOW THE ROOT IS FOLLOWED, as the bar writer's door (CE-62):
    // a linked pointer directory would send the promotion somewhere else.
    refuse_links(root, &pointer_file)?;
    let created = missing_dirs(root, &directory);
    io_at(
        fs::create_dir_all(&directory),
        &directory,
        "create pointer directories",
        false,
    )?;
    for made in &created {
        sync_dir(made.parent().unwrap_or(root), false)?;
    }
    let handle = io_at(
        no_follow(
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false),
        )
        .open(&pointer_file),
        &pointer_file,
        "open pointer",
        false,
    )?;
    // ONE PROMOTER AT A TIME, refused rather than waited for.
    let lock = match Flock::try_lock(handle, pointer_file.clone()) {
        Ok(lock) => lock,
        Err(std::fs::TryLockError::WouldBlock) => {
            return Err(StoreError::Locked { path: pointer_file }.into());
        }
        Err(std::fs::TryLockError::Error(why)) => {
            return io_at(Err(why), &pointer_file, "lock pointer", false);
        }
    };
    // UNDER THE LOCK: what is served now, and the header of that generation.
    let now = served(root, path)?;
    if let Served::Revision(pointer) = now
        && pointer.revision == revision
    {
        release(lock, &pointer_file, false)?;
        return Ok(Promoted::Already(pointer));
    }
    let current = BarFile::open_existing(root, path, symbol_id)?.header();
    if candidate.source_header() != current {
        release(lock, &pointer_file, false)?;
        return Err(RepairError::StaleSource);
    }
    let len = io_at(lock.metadata(), &pointer_file, "measure pointer", false)?.len();
    let record = len.div_ceil(RECORD_LEN);
    if record >= MAX_RECORDS {
        release(lock, &pointer_file, false)?;
        return Err(RepairError::RevisionLimit);
    }
    let header = candidate.header();
    let pointer = Pointer {
        revision,
        previous: now.ordinal(),
        n_valid: header.n_valid,
        first_ts_micros: header.first_ts_micros,
        last_ts_micros: header.last_ts_micros,
        header_generation: header.generation,
        promoted_at_micros: now_micros(),
        record,
    };
    io_at(
        lock.write_all_at(&pointer.encode(), record * RECORD_LEN),
        &pointer_file,
        "write pointer record",
        true,
    )?;
    io_at(lock.sync_all(), &pointer_file, "sync pointer", true)?;
    sync_dir(&directory, true)?;
    release(lock, &pointer_file, true)?;
    Ok(Promoted::Promoted(pointer))
}

/// `O_NOFOLLOW` on the hosts `crate::open_flags` has a verified value for.
fn no_follow(options: &mut OpenOptions) -> &mut OpenOptions {
    #[cfg(any(
        target_os = "macos",
        all(
            any(target_os = "linux", target_os = "android"),
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(crate::open_flags::O_NOFOLLOW);
    }
    options
}

/// [`StoreError::Symlinked`] for the first component of `leaf` below `root`
/// that is a symbolic link, root-most first; an absent one ends the walk.
fn refuse_links(root: &Path, leaf: &Path) -> Result<(), RepairError> {
    let Ok(below) = leaf.strip_prefix(root) else {
        return Ok(());
    };
    let mut at = root.to_path_buf();
    for part in below.components() {
        at.push(part);
        match fs::symlink_metadata(&at) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(StoreError::Symlinked { path: at }.into());
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    Ok(())
}

/// The directories between `root` and `dir` that do not exist yet,
/// root-most first, so each new entry's parent can be synced.
fn missing_dirs(root: &Path, dir: &Path) -> Vec<PathBuf> {
    let mut missing = Vec::new();
    let mut cursor = Some(dir);
    while let Some(here) = cursor.filter(|&here| here != root && here.starts_with(root)) {
        if !matches!(here.try_exists(), Ok(false)) {
            break;
        }
        missing.push(here.to_path_buf());
        cursor = here.parent();
    }
    missing.reverse();
    missing
}

fn sync_dir(dir: &Path, visible: bool) -> Result<(), RepairError> {
    let handle = io_at(File::open(dir), dir, "open directory for sync", visible)?;
    io_at(handle.sync_all(), dir, "sync directory", visible)
}

fn release(lock: Flock<File>, path: &Path, visible: bool) -> Result<(), RepairError> {
    io_at(
        lock.release().map_err(io::Error::from),
        path,
        "release pointer lock",
        visible,
    )
}

fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_micros()).unwrap_or(i64::MAX)
        })
}

fn io_at<T>(
    result: io::Result<T>,
    path: &Path,
    operation: &'static str,
    visible: bool,
) -> Result<T, RepairError> {
    result.map_err(|error| RepairError::Io {
        path: path.to_path_buf(),
        operation,
        kind: error.kind(),
        publication_may_be_visible: visible,
    })
}

/// [`StoreError::Superseded`] when `path`'s month is served from a revision:
/// the writer door's refusal, before anything is created or locked.
///
/// # Errors
///
/// The refusal, or whatever [`served`] refuses.
pub(crate) fn refuse_superseded(root: &Path, path: StorePath<'_>) -> Result<(), StoreError> {
    match served(root, path)? {
        Served::Original => Ok(()),
        Served::Revision(pointer) => Err(StoreError::Superseded {
            path: path.with_file(FileKind::Bars).to_path_buf(root),
            ordinal: pointer.revision.ordinal(),
        }),
    }
}

#[cfg(test)]
#[path = "generation_tests.rs"]
mod tests;
