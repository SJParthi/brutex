//! Explicit, bounded historical bar revisions; never a replacement of a month.
//!
//! See `crates/store/REPAIR.md` for the publication protocol, format, invariants,
//! and the integration deliberately left to the calendar and ingest owners.
//! All month filenames still come from [`StorePath`]. The separate revision
//! root is outside the ordinary catalog's `bars/` tree. No rename, deletion,
//! overlay copy, or change to append semantics is involved.
//!
//! **Served by every reader once promoted (D-3350, WP-13c).** A published
//! revision is inert until [`crate::generation::promote`] appends the month's
//! pointer record; from then on `BarFile::open_existing` opens it for every
//! reader and the writer door refuses the original. [`repair_month`] is the
//! whole repair: the served month's bars plus the offered ones, a held bar never
//! dropped and never changed, published at the next free ordinal and promoted.

use std::fmt;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::file::{Admission, BarFile, StoreError, survey};
use crate::flock::Flock;
use crate::format::Bar;
use crate::header::Header;
use crate::path::{FileKind, StorePath};

/// Maximum rows in both the source snapshot and the complete replacement.
/// This is a resource ceiling, not an assertion about calendar completeness.
///
/// **The most bars one month can hold on the finest rung the store files**:
/// 31 days of one-second bars ([`crate::path::Timeframe::SECOND_1`]), one per
/// second, which the month admission (D-0915) already bounds any month to.
/// It was 100,000, which no one-second month of a full session calendar fits
/// under — 23 sessions of 22,500 seconds are 517,500 bars, and the GDFL
/// NIFTY 2018-11 month is 22,949,312 bytes on the operator's store (10 Oct
/// 2026), 409,224 records past its 32,768-byte header region — so every
/// repair of one was refused `RowLimit` and the GDFL days a month lacks could
/// never be back-filled (DB-N26, WP-12d). At 56 bytes a bar the ceiling
/// bounds one repair's merged rows to 150 MB.
pub const MAX_ROWS: usize = 31 * 86_400;
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

/// `<root>/bar-revisions-v1/<n>`: the root a revision's month files are
/// rendered under by the unmodified [`StorePath`].
#[must_use]
pub fn revision_root(root: &Path, revision: Revision) -> PathBuf {
    revision.root(root)
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
    /// An offered bar at a stamp the served month holds carries other values.
    /// The held bar is never changed by a repair; the month is refused whole,
    /// naming the first such bar, and nothing is written (D-3350).
    HeldBarDiffers {
        /// The stamp both carry.
        ts_micros: i64,
        /// What the served month holds.
        held: Box<Bar>,
        /// What was offered.
        offered: Box<Bar>,
    },
    /// The offered bars share no stamp with the served month, so nothing
    /// proves they are on the price basis its held bars are on (a vendor that
    /// back-adjusts as of the pull date, D-3323). Widen the window to include
    /// a held session (D-3350).
    NoBasisWitness,
    /// A sibling indexed by record (an overlay or greeks file) exists beside
    /// the month: inserting bars would shift every row it describes.
    IndexedSibling(PathBuf),
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
            Self::RowLimit => write!(f, "repair exceeds the {MAX_ROWS}-row ceiling"),
            Self::StaleSource => f.write_str("repair source header changed; merge again"),
            Self::MissingTimestamp(ts) => write!(f, "repair omits source timestamp {ts}"),
            Self::UnsealedSource => f.write_str("repair requires checksum-protected source bars"),
            Self::Conflict => f.write_str("repair revision already holds a different request"),
            Self::Incomplete(path) => {
                write!(f, "incomplete repair at {}; preserved", path.display())
            }
            Self::InvalidReceipt(path) => write!(f, "invalid repair receipt at {}", path.display()),
            Self::HeldBarDiffers {
                ts_micros,
                held,
                offered,
            } => write!(
                f,
                "repair refused: the held bar at {ts_micros} would change (held o={} h={} l={} c={} v={} oi={}; offered o={} h={} l={} c={} v={} oi={}); a repair never changes a held bar, and nothing was written",
                held.open,
                held.high,
                held.low,
                held.close,
                held.volume,
                held.open_interest,
                offered.open,
                offered.high,
                offered.low,
                offered.close,
                offered.volume,
                offered.open_interest
            ),
            Self::NoBasisWitness => f.write_str(
                "repair refused: no offered bar shares a stamp with the month's held bars, so nothing proves the vendor's price basis is unchanged since they were stored; widen the window to include a held session",
            ),
            Self::IndexedSibling(path) => write!(
                f,
                "repair refused: {} is indexed by record and inserting bars would shift every row it describes",
                path.display()
            ),
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
        // Every refusal below releases it through the guard's explicit unlock.
        let lock_path = path.with_file(FileKind::Lock).to_path_buf(&revision_root);
        let lock = shared_lock(lock_path.clone())?;
        let bars = BarFile::open_existing(&revision_root, path, symbol_id)?;
        check_header(bars.header())?;
        if source.symbol_id != symbol_id
            || source.timeframe_secs != path.timeframe().secs()
            || encoded(bars.header())?.as_slice() != revision_bytes
        {
            return Err(RepairError::InvalidReceipt(receipt_path));
        }
        check_header(source)?;
        io_at(
            lock.release().map_err(io::Error::from),
            &lock_path,
            "release shared lock",
            false,
        )?;
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
    // THE SAME PATH-DERIVED ADMISSION `BarFile::append` asks (D-0915): a
    // revision is a write boundary too, and a merged bar outside the month or
    // off the rung's grid would otherwise publish through it.
    Admission::new(path.month(), path.timeframe().secs()).admit(merged)?;
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

    let lock_path = path.with_file(FileKind::Lock).to_path_buf(root);
    let source_lock = shared_lock(lock_path.clone())?;
    let source = BarFile::open_existing(root, path, symbol_id)?;
    check_header(source.header())?;
    if source.header() != expected_source {
        return Err(RepairError::StaleSource);
    }
    retain_timestamps(&source, merged)?;
    let published = write_revision(root, path, symbol_id, revision, expected_source, merged)?;
    // Released explicitly, never by close: a descriptor duplicated into a child
    // another thread spawned would otherwise keep the source locked. D-0693.
    // A refusal here comes after a written revision, so it says the
    // publication may be visible, because it is.
    io_at(
        source_lock.release().map_err(io::Error::from),
        &lock_path,
        "release shared lock",
        true,
    )?;
    Ok(published)
}

/// How [`repair_month`] compares an offered bar with the held bar at the same
/// stamp. Fixed by the caller from what the instrument IS, never from the bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// Open interest is a measurement for this instrument. When it is not —
    /// an index or a share (D-3322) — a held `0` and an offered null are the
    /// same absence, and the held one is kept.
    pub open_interest_applies: bool,
    /// The vendor adjusts its prices as of the pull date (D-3323): the
    /// offered bars must share at least one stamp with the held ones and agree
    /// there, or nothing proves the two pulls are on one basis.
    pub basis_witness: bool,
}

/// What [`repair_month`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonthRepair {
    /// Every offered bar is already held, with the same values. Nothing was
    /// written; the month is served as it was.
    NothingNew {
        /// The generation served: `0` is the original.
        ordinal: u32,
        /// Its committed records.
        n_valid: u64,
    },
    /// A new generation is served: the held bars and the added ones.
    Repaired {
        /// The generation it replaced: `0` is the original.
        from: u32,
        /// The revision now served.
        to: Revision,
        /// Bars the replaced generation held, every one of them kept.
        held: u64,
        /// Bars added.
        added: u64,
        /// The stamps of the bars added, ascending: what the repair filled,
        /// for a caller's ledger.
        added_at: Vec<i64>,
        /// The new generation's committed header.
        header: Header,
        /// Whether this call wrote the revision or found it already complete
        /// (a crash between publication and promotion, resumed).
        published: Published,
        /// The promotion.
        promoted: crate::generation::Promoted,
        /// Ordinals stepped over because they hold another request or an
        /// unfinished one, each with why. Never reused, never overwritten.
        skipped: Vec<(u32, String)>,
    },
}

/// Repair one month: serve a new generation holding every bar the served
/// generation holds plus every offered bar it lacks. D-3350, WP-13c.
///
/// * **Never in place.** The served generation's files are not opened for
///   writing; the new one is published under `bar-revisions-v1/<n>` by
///   [`publish`] and made the served one by [`crate::generation::promote`].
/// * **Never drops a held bar**, and never changes one: an offered bar at a
///   held stamp must carry the held values ([`Policy`]), or the month is
///   refused whole, naming the bar ([`RepairError::HeldBarDiffers`]).
/// * **Idempotent and resumable.** Offered bars that are all held answer
///   [`MonthRepair::NothingNew`]. A revision published and not promoted (a
///   crash between the two) is found complete at its ordinal and promoted. An
///   ordinal that holds another request or an unfinished one is stepped over
///   and named, never reused.
///
/// A month that does not exist is [`StoreError::Missing`]: an ordinary append
/// creates it, and a repair has nothing to keep.
///
/// # Cost
///
/// O(held + offered) to read and merge, bounded by [`MAX_ROWS`], plus what
/// [`publish`] states; one probe per stepped-over ordinal, at most
/// [`MAX_REVISIONS`]. Not O(1), and not on a read path.
///
/// # Errors
///
/// Every refusal of [`publish`] and [`crate::generation::promote`], the
/// month's own read refusals, [`RepairError::HeldBarDiffers`],
/// [`RepairError::NoBasisWitness`] and [`RepairError::IndexedSibling`].
pub fn repair_month(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    offered: &[Bar],
    policy: Policy,
) -> Result<MonthRepair, RepairError> {
    check_kind(path)?;
    if offered.len() > MAX_ROWS {
        return Err(RepairError::RowLimit);
    }
    survey(offered)?;
    Admission::new(path.month(), path.timeframe().secs()).admit(offered)?;
    let served = crate::generation::served(root, path)?;
    let served_root = served.root(root);
    for kind in [FileKind::Overlay, FileKind::Greeks] {
        let sibling = path.with_file(kind).to_path_buf(&served_root);
        if exists(&sibling)? {
            return Err(RepairError::IndexedSibling(sibling));
        }
    }
    let (header, merged, added_at, witnessed) = {
        let source = BarFile::open_existing(root, path, symbol_id)?;
        let header = source.header();
        check_header(header)?;
        let held = usize::try_from(header.n_valid).map_err(|_| RepairError::RowLimit)?;
        let mut merged = Vec::with_capacity(held.saturating_add(offered.len()));
        let mut offer = offered.iter().peekable();
        let mut witnessed = 0u64;
        let mut added_at = Vec::new();
        for index in 0..header.n_valid {
            let bar = source.read_record(index)?;
            while let Some(early) = offer.next_if(|o| o.ts_micros < bar.ts_micros) {
                added_at.push(early.ts_micros);
                merged.push(*early);
            }
            if let Some(same) = offer.next_if(|o| o.ts_micros == bar.ts_micros) {
                if !agrees(&bar, same, policy) {
                    return Err(RepairError::HeldBarDiffers {
                        ts_micros: bar.ts_micros,
                        held: Box::new(bar),
                        offered: Box::new(*same),
                    });
                }
                witnessed += 1;
            }
            merged.push(bar);
        }
        for late in offer {
            added_at.push(late.ts_micros);
            merged.push(*late);
        }
        (header, merged, added_at, witnessed)
    };
    let added = added_at.len() as u64;
    if added == 0 {
        return Ok(MonthRepair::NothingNew {
            ordinal: served.ordinal(),
            n_valid: header.n_valid,
        });
    }
    if policy.basis_witness && header.n_valid > 0 && witnessed == 0 {
        return Err(RepairError::NoBasisWitness);
    }
    let mut skipped = Vec::new();
    for ordinal in served.ordinal().saturating_add(1)..=MAX_REVISIONS {
        let revision = Revision::new(ordinal)?;
        match publish(root, path, symbol_id, revision, header, &merged) {
            Ok(published) => {
                let promoted = crate::generation::promote(root, path, symbol_id, revision)?;
                let now = promoted.pointer();
                let header = RevisionReader::open(root, path, symbol_id, now.revision)?.header();
                return Ok(MonthRepair::Repaired {
                    from: served.ordinal(),
                    to: revision,
                    held: (merged.len() as u64).saturating_sub(added),
                    added,
                    added_at,
                    header,
                    published,
                    promoted,
                    skipped,
                });
            }
            // ANOTHER REQUEST OR AN UNFINISHED ONE HOLDS THIS ORDINAL. It is
            // never reopened or overwritten; the next is tried, and this one
            // is named to the caller.
            Err(
                why @ (RepairError::Conflict
                | RepairError::Incomplete(_)
                | RepairError::InvalidReceipt(_)),
            ) => skipped.push((ordinal, why.to_string())),
            Err(why) => return Err(why),
        }
    }
    Err(RepairError::RevisionLimit)
}

/// Whether an offered bar says what the held bar at its stamp says.
const fn agrees(held: &Bar, offered: &Bar, policy: Policy) -> bool {
    let interest = held.open_interest == offered.open_interest
        || (!policy.open_interest_applies
            && (held.open_interest == 0 || held.open_interest == crate::format::OI_NULL)
            && (offered.open_interest == 0 || offered.open_interest == crate::format::OI_NULL));
    held.ts_micros == offered.ts_micros
        && held.open == offered.open
        && held.high == offered.high
        && held.low == offered.low
        && held.close == offered.close
        && held.volume == offered.volume
        && interest
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
    for kind in [
        FileKind::Bars,
        FileKind::Checksums,
        FileKind::Lock,
        FileKind::TimeIndex,
    ] {
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

/// The lock at `path`, taken shared and non-blocking; a missing lock file is
/// refused, never created.
pub(crate) fn shared_lock(path: PathBuf) -> Result<Flock<File>, RepairError> {
    let file = io_at(File::open(&path), &path, "open required lock", false)?;
    match Flock::try_lock_shared(file, path.clone()) {
        Ok(held) => Ok(held),
        Err(TryLockError::WouldBlock) => Err(StoreError::Locked { path }.into()),
        Err(TryLockError::Error(error)) => io_at(Err(error), &path, "lock shared", false),
    }
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

#[cfg(test)]
#[path = "repair_tests.rs"]
mod tests;
