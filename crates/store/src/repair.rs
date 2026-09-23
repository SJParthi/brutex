//! Explicit, bounded historical bar revisions; never a replacement of a month.
//!
//! See `crates/store/REPAIR.md` for the publication protocol, format, invariants,
//! and the integration deliberately left to the calendar and ingest owners.
//! All month filenames still come from [`StorePath`]. The separate revision
//! root is outside the ordinary catalog's `bars/` tree. No current pointer,
//! rename, deletion, overlay copy, or change to append semantics is involved.

use std::fmt;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::file::{BarFile, StoreError, survey};
use crate::format::Bar;
use crate::header::Header;
use crate::path::{FileKind, StorePath};

/// Maximum rows in both the source snapshot and the complete replacement.
/// This is a resource ceiling, not an assertion about calendar completeness.
pub const MAX_ROWS: usize = 100_000;
/// Maximum explicit revision ordinal per logical month. There is no scan for
/// a free ordinal and exhausted ordinals are never reused for different data.
pub const MAX_REVISIONS: u32 = 1_024;
const MAGIC: &[u8; 8] = b"BRXREPV1";
const RECEIPT_LEN: usize = 144;

/// An explicit revision ordinal, independent of the bar file format version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revision(u32);

impl Revision {
    /// Construct a bounded ordinal.
    ///
    /// # Errors
    /// Refuses zero and ordinals above [`MAX_REVISIONS`].
    pub const fn new(ordinal: u32) -> Result<Self, RepairError> {
        if ordinal == 0 || ordinal > MAX_REVISIONS {
            Err(RepairError::RevisionLimit)
        } else {
            Ok(Self(ordinal))
        }
    }

    /// The stable ordinal used in the on-disk path.
    #[must_use]
    pub const fn ordinal(self) -> u32 {
        self.0
    }

    fn root(self, root: &Path) -> PathBuf {
        root.join("bar-revisions-v1").join(self.0.to_string())
    }
}

/// Named refusals from the repair boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairError {
    /// A regular store validation or read failed.
    Store(StoreError),
    /// The ordinal is outside the fixed namespace.
    RevisionLimit,
    /// The source or offered row count exceeds [`MAX_ROWS`].
    RowLimit,
    /// The source has advanced since the caller prepared the merged rows.
    StaleSource,
    /// A source timestamp was omitted from the complete merged series.
    MissingTimestamp(i64),
    /// The original is not checksum protected, so verified repair is refused.
    UnsealedSource,
    /// A published ordinal already holds a different request.
    Conflict,
    /// Reserved or unexplained files have no valid completion receipt.
    Incomplete(PathBuf),
    /// The receipt is damaged, foreign, or disagrees with the bar header.
    InvalidReceipt(PathBuf),
    /// A filesystem operation failed. Once receipt writing starts the complete
    /// revision may be visible even if crash durability was not established.
    Io {
        /// The exact file or directory involved.
        path: PathBuf,
        /// The operation that failed.
        operation: &'static str,
        /// The host refusal.
        kind: io::ErrorKind,
        /// True means callers must not report that publication did not happen.
        publication_may_be_visible: bool,
    },
}

impl fmt::Display for RepairError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => error.fmt(f),
            Self::RevisionLimit => f.write_str("repair revision ordinal outside 1..=1024"),
            Self::RowLimit => f.write_str("repair exceeds the 100000-row ceiling"),
            Self::StaleSource => f.write_str("repair source header changed; merge again"),
            Self::MissingTimestamp(ts) => write!(f, "repair omits source timestamp {ts}"),
            Self::UnsealedSource => f.write_str("repair requires checksum-protected source bars"),
            Self::Conflict => f.write_str("repair revision already holds a different request"),
            Self::Incomplete(path) => {
                write!(f, "incomplete repair at {}; preserved", path.display())
            }
            Self::InvalidReceipt(path) => write!(f, "invalid repair receipt at {}", path.display()),
            Self::Io {
                path,
                operation,
                kind,
                publication_may_be_visible,
            } => write!(
                f,
                "{operation} {}: {kind:?}; publication may be visible: {publication_may_be_visible}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for RepairError {}

impl From<StoreError> for RepairError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}

/// Whether this call created the revision or verified an exact completed retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Published {
    /// Data, checksums, receipt, and directory entries were synced.
    Created,
    /// Exact bytes and source header matched; durability was re-established.
    Reused,
}

/// A pinned revision reader. No append method or mutable bar handle is exposed.
#[derive(Debug)]
pub struct RevisionReader {
    bars: BarFile,
    source: Header,
}

impl RevisionReader {
    /// Open exactly one published revision, without creating any files.
    ///
    /// # Errors
    /// Refuses missing/torn receipts, missing locks, changed revision headers,
    /// unsealed or oversized files, and the normal [`BarFile`] open failures.
    /// Block CRC validation remains lazy and constant per record read.
    pub fn open(
        root: &Path,
        path: StorePath<'_>,
        symbol_id: u32,
        revision: Revision,
    ) -> Result<Self, RepairError> {
        check_kind(path)?;
        let revision_root = revision.root(root);
        let physical = path.to_path_buf(&revision_root);
        let receipt_path = physical.with_extension("repair-v1");
        let receipt = read_receipt(&receipt_path)?;
        let (prefix, headers) = receipt.split_at(16);
        let (magic, ordinal) = prefix.split_at(8);
        let expected_ordinal = u64::from(revision.0).to_le_bytes();
        if magic != MAGIC || ordinal != expected_ordinal {
            return Err(RepairError::InvalidReceipt(receipt_path));
        }
        let (source_bytes, revision_bytes) = headers.split_at(64);
        let source = Header::decode(source_bytes)
            .map_err(|_| RepairError::InvalidReceipt(receipt_path.clone()))?;
        // Unlike the legacy opener, revisions must never admit a missing lock.
        let _lock = shared_lock(path.with_file(FileKind::Lock).to_path_buf(&revision_root))?;
        let bars = BarFile::open_existing(&revision_root, path, symbol_id)?;
        check_header(bars.header())?;
        if source.symbol_id != symbol_id
            || source.timeframe_secs != path.timeframe().secs()
            || encoded(bars.header())?.as_slice() != revision_bytes
        {
            return Err(RepairError::InvalidReceipt(receipt_path));
        }
        check_header(source)?;
        Ok(Self { bars, source })
    }

    /// The original snapshot's header, retained in the completion receipt.
    #[must_use]
    pub const fn source_header(&self) -> Header {
        self.source
    }

    /// The revision's committed bar header.
    #[must_use]
    pub const fn header(&self) -> Header {
        self.bars.header()
    }

    /// Read through the existing fixed-stride and block-CRC contract.
    ///
    /// # Errors
    /// Returns the same indexed-read refusals as [`BarFile::read_record`].
    pub fn read_record(&self, index: u64) -> Result<Bar, StoreError> {
        self.bars.read_record(index)
    }

    /// Locate a timestamp through the existing chronological read contract.
    ///
    /// # Errors
    /// Returns the underlying bar read or integrity refusal.
    pub fn first_at_or_after(&self, timestamp: i64) -> Result<u64, StoreError> {
        self.bars.first_at_or_after(timestamp)
    }
}

/// UNVERIFIED performance: no named cost test or measured latency bound is established here.
/// Publish a complete chronological merge as an explicit revision.
///
/// `expected_source` is the original reader's header when the merge was made.
/// Every original timestamp must survive; values at those timestamps may be
/// corrected. Calendar, month membership, provenance, and correction policy
/// must be validated by the caller. The source stays shared-locked throughout.
/// A completed byte-exact retry succeeds even if the original later advances.
/// Work is O(source rows + merged rows), bounded by [`MAX_ROWS`]; reads retain
/// their existing O(1) per-record cost. This never selects a current revision.
///
/// # Errors
/// Refuses invalid batches before reserving anything; stale, unsealed, corrupt,
/// or unlocked sources; missing source timestamps; differing completed retries;
/// unfinished ordinals; and named I/O failures. Failed reservations are retained
/// for inspection and never overwritten. Choose another ordinal after review.
pub fn publish(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    revision: Revision,
    expected_source: Header,
    merged: &[Bar],
) -> Result<Published, RepairError> {
    check_kind(path)?;
    if merged.len() > MAX_ROWS {
        return Err(RepairError::RowLimit);
    }
    survey(merged)?;
    check_header(expected_source)?;
    let revision_root = revision.root(root);
    let physical = path.to_path_buf(&revision_root);
    let receipt_path = physical.with_extension("repair-v1");
    let reservation = physical.with_extension("reserved-v1");
    // Completed retries never rewrite any evidence, including their receipt.
    if exists(&reservation)? {
        let reader = RevisionReader::open(root, path, symbol_id, revision)?;
        if reader.source_header() != expected_source
            || reader.header().n_valid != merged.len() as u64
        {
            return Err(RepairError::Conflict);
        }
        for (index, offered) in merged.iter().enumerate() {
            if reader.read_record(index as u64)? != *offered {
                return Err(RepairError::Conflict);
            }
        }
        sync_revision(root, path, &revision_root, &receipt_path)?;
        return Ok(Published::Reused);
    }

    let _source_lock = shared_lock(path.with_file(FileKind::Lock).to_path_buf(root))?;
    let source = BarFile::open_existing(root, path, symbol_id)?;
    check_header(source.header())?;
    if source.header() != expected_source {
        return Err(RepairError::StaleSource);
    }
    retain_timestamps(&source, merged)?;
    write_revision(root, path, symbol_id, revision, expected_source, merged)
}

fn retain_timestamps(source: &BarFile, merged: &[Bar]) -> Result<(), RepairError> {
    let mut offered = merged.iter().peekable();
    let mut previous = None;
    for index in 0..source.records() {
        let bar = source.read_record(index)?;
        // Legacy files need not have been written by today's batch validator.
        survey(std::slice::from_ref(&bar))?;
        if previous.is_some_and(|ts| ts >= bar.ts_micros) {
            return Err(RepairError::Store(StoreError::BatchNotOrdered {
                at: index,
                previous: previous.unwrap_or(bar.ts_micros),
                next: bar.ts_micros,
            }));
        }
        previous = Some(bar.ts_micros);
        while offered
            .peek()
            .is_some_and(|row| row.ts_micros < bar.ts_micros)
        {
            offered.next();
        }
        if offered
            .next()
            .is_none_or(|row| row.ts_micros != bar.ts_micros)
        {
            return Err(RepairError::MissingTimestamp(bar.ts_micros));
        }
    }
    Ok(())
}

fn write_revision(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    revision: Revision,
    expected_source: Header,
    merged: &[Bar],
) -> Result<Published, RepairError> {
    let revision_root = revision.root(root);
    let physical = path.to_path_buf(&revision_root);
    let receipt_path = physical.with_extension("repair-v1");
    let reservation = physical.with_extension("reserved-v1");
    let directory = physical
        .parent()
        .ok_or_else(|| RepairError::Incomplete(physical.clone()))?;
    io_at(
        fs::create_dir_all(directory),
        directory,
        "create revision directories",
        false,
    )?;
    // create_new is the one-writer reservation. It is never removed, including
    // after an interrupted write. No competing publisher can reopen this pair.
    let reserved = io_at(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&reservation),
        &reservation,
        "reserve revision",
        false,
    )?;
    io_at(reserved.sync_all(), &reservation, "sync reservation", false)?;
    for kind in [FileKind::Bars, FileKind::Checksums, FileKind::Lock] {
        let sibling = path.with_file(kind).to_path_buf(&revision_root);
        if exists(&sibling)? {
            return Err(RepairError::Incomplete(sibling));
        }
    }
    if exists(&receipt_path)? {
        return Err(RepairError::Incomplete(receipt_path));
    }
    let mut writer = BarFile::open_or_create(&revision_root, path, symbol_id)?;
    writer.append(merged)?;
    // Verify every written block before publishing its receipt.
    for (index, offered) in merged.iter().enumerate() {
        if writer.read_record(index as u64)? != *offered {
            return Err(RepairError::Conflict);
        }
    }
    // Make every new directory entry durable, not just the final month parent.
    sync_ancestors(directory, root, false)?;
    let mut receipt = Vec::with_capacity(RECEIPT_LEN);
    receipt.extend_from_slice(MAGIC);
    receipt.extend_from_slice(&u64::from(revision.0).to_le_bytes());
    receipt.extend_from_slice(&encoded(expected_source)?);
    receipt.extend_from_slice(&encoded(writer.header())?);
    let mut marker = io_at(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&receipt_path),
        &receipt_path,
        "create receipt",
        false,
    )?;
    io_at(
        marker.write_all(&receipt),
        &receipt_path,
        "write receipt",
        true,
    )?;
    io_at(marker.sync_all(), &receipt_path, "sync receipt", true)?;
    sync_ancestors(directory, root, true)?;
    Ok(Published::Created)
}

fn check_kind(path: StorePath<'_>) -> Result<(), RepairError> {
    if path.file() != FileKind::Bars {
        return Err(StoreError::NotABarPath { found: path.file() }.into());
    }
    Ok(())
}

fn check_header(header: Header) -> Result<(), RepairError> {
    if header.n_valid > MAX_ROWS as u64 {
        return Err(RepairError::RowLimit);
    }
    if !header.checksums_present() {
        return Err(RepairError::UnsealedSource);
    }
    Ok(())
}

fn encoded(header: Header) -> Result<[u8; 64], RepairError> {
    header
        .commit()
        .map(|commit| commit.bytes)
        .map_err(|_| RepairError::Conflict)
}

fn read_receipt(path: &Path) -> Result<[u8; RECEIPT_LEN], RepairError> {
    let mut file = match File::open(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(RepairError::Incomplete(path.to_path_buf()));
        }
        result => io_at(result, path, "open receipt", false)?,
    };
    if io_at(file.metadata(), path, "measure receipt", false)?.len() != RECEIPT_LEN as u64 {
        return Err(RepairError::InvalidReceipt(path.to_path_buf()));
    }
    let mut bytes = [0; RECEIPT_LEN];
    io_at(file.read_exact(&mut bytes), path, "read receipt", false)?;
    Ok(bytes)
}

fn shared_lock(path: PathBuf) -> Result<File, RepairError> {
    let file = io_at(File::open(&path), &path, "open required lock", false)?;
    match file.try_lock_shared() {
        Ok(()) => (),
        Err(TryLockError::WouldBlock) => return Err(StoreError::Locked { path }.into()),
        Err(TryLockError::Error(error)) => return io_at(Err(error), &path, "lock shared", false),
    }
    Ok(file)
}

fn exists(path: &Path) -> Result<bool, RepairError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        result => io_at(result, path, "inspect revision", false).map(|_| true),
    }
}

fn sync_revision(
    root: &Path,
    path: StorePath<'_>,
    revision_root: &Path,
    receipt: &Path,
) -> Result<(), RepairError> {
    for sibling in [
        path.to_path_buf(revision_root),
        path.with_file(FileKind::Checksums)
            .to_path_buf(revision_root),
        receipt.to_path_buf(),
    ] {
        sync_file(&sibling, true)?;
    }
    sync_ancestors(
        receipt
            .parent()
            .ok_or_else(|| RepairError::Incomplete(receipt.to_path_buf()))?,
        root,
        true,
    )
}

fn sync_ancestors(mut directory: &Path, root: &Path, visible: bool) -> Result<(), RepairError> {
    loop {
        sync_file(directory, visible)?;
        if directory == root {
            return Ok(());
        }
        directory = directory
            .parent()
            .ok_or_else(|| RepairError::Incomplete(directory.to_path_buf()))?;
    }
}

fn sync_file(path: &Path, visible: bool) -> Result<(), RepairError> {
    let file = io_at(File::open(path), path, "open for sync", visible)?;
    io_at(file.sync_all(), path, "sync", visible)
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
