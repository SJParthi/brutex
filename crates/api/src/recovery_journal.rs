//! Restart-safe progress for exact, caller-identified recovery work units.
//!
//! V1 is an append-only sequence of 1,024-byte records. Integers are little
//! endian; CRC-32C covers bytes `0..1020`. The layout is deliberately separate
//! from the existing pull audit journal, whose version and stride do not move.
//!
//! | Bytes | Meaning |
//! |---|---|
//! | 0..4 | `BXRJ` |
//! | 4..6 | version, 1 |
//! | 6 | status, 0..6 in [`Status`] order |
//! | 7 | reserved, zero |
//! | 8..40 | caller-supplied, domain-separated work key |
//! | 40..44 / 44..48 | attempts / unchanged attempts |
//! | 48..56 / 56..64 | committed records / diagnostics |
//! | 64..66 / 66..68 | HTTP status / body length in bytes |
//! | 68..80 | reserved, zero |
//! | 80..848 | exact UTF-8 request body, followed by zero padding |
//! | 848..856 / 856..864 | missing / unverified coverage quantities |
//! | 864..1020 | reserved, zero |
//! | 1020..1024 | CRC-32C |
//!
//! The coordinator owns canonical form validation, key derivation, transitions,
//! and retry policy. This module never rewrites a form or resets a counter. It
//! refuses a different body under an existing key, both on append and replay.
//! A stored `InFlight` record remains `InFlight` after restart: deciding how to
//! reconcile that interrupted attempt belongs to the coordinator.
//!
//! Opening takes an exclusive advisory lock for the handle's entire lifetime,
//! replays with a bounded read buffer, and refuses corruption or a torn tail
//! without truncation. The containing directory must already exist. Appending
//! publishes to the index only after `sync_all` succeeds. Any uncertain I/O
//! poisons the handle; dropping and reopening is required. In particular, a
//! failed sync does NOT prove that the record is absent on disk. Reopen checks
//! and syncs the whole surviving file before exposing its recovered index.
//!
//! One append writes a fixed-size record and uses expected/amortized O(1)
//! indexed state updates. Restart is O(journal records); memory is O(distinct
//! work units), plus a fixed replay buffer. Disk latency, hash-table resize,
//! and worst-case hash collisions are not claimed to be constant-time.

use std::collections::HashMap;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use store::crc::crc32c;

const RECORD_LEN: usize = 1_024;
const RECORD_LEN_U64: u64 = 1_024;
const MAGIC: [u8; 4] = *b"BXRJ";
const VERSION: u16 = 1;
const MAX_BODY_BYTES: usize = 768;
const BODY_OFFSET: usize = 80;
const MISSING_OFFSET: usize = 848;
const UNVERIFIED_OFFSET: usize = 856;
const RESERVED_OFFSET: usize = 864;
const CRC_OFFSET: usize = 1_020;
const REPLAY_BUFFER_BYTES: usize = RECORD_LEN * 64;
/// Maximum events a read-only status poll may request; larger limits refuse.
pub(crate) const MAX_TAIL_RECORDS: usize = 256;
const _: () = assert!(BODY_OFFSET + MAX_BODY_BYTES == MISSING_OFFSET);
const _: () = assert!(MISSING_OFFSET + 8 == UNVERIFIED_OFFSET);
const _: () = assert!(UNVERIFIED_OFFSET + 8 == RESERVED_OFFSET);
const _: () = assert!(RESERVED_OFFSET <= CRC_OFFSET);
const _: () = assert!(CRC_OFFSET + 4 == RECORD_LEN);
const _: () = assert!(RECORD_LEN as u64 == RECORD_LEN_U64);

/// Persisted coordinator state, not a claim inferred from HTTP success.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Status {
    /// Planned but not admitted to a vendor request yet (wire code 0).
    Queued,
    /// The attempt was durably reserved before its request (wire code 1).
    InFlight,
    /// The coordinator's explicit verification completed (wire code 2).
    Verified,
    /// Evidence remains insufficient (wire code 3).
    Unverified,
    /// The coordinator's retry budget is spent (wire code 4).
    Exhausted,
    /// Work cannot safely proceed (wire code 5).
    Blocked,
    /// The coordinator proved no acquisition was required (wire code 6).
    NotApplicable,
}

impl Status {
    const fn code(self) -> u8 {
        match self {
            Self::Queued => 0,
            Self::InFlight => 1,
            Self::Verified => 2,
            Self::Unverified => 3,
            Self::Exhausted => 4,
            Self::Blocked => 5,
            Self::NotApplicable => 6,
        }
    }

    fn decode(code: u8) -> io::Result<Self> {
        match code {
            0 => Ok(Self::Queued),
            1 => Ok(Self::InFlight),
            2 => Ok(Self::Verified),
            3 => Ok(Self::Unverified),
            4 => Ok(Self::Exhausted),
            5 => Ok(Self::Blocked),
            6 => Ok(Self::NotApplicable),
            _ => Err(invalid_data(format!("unknown recovery status {code}"))),
        }
    }
}

/// One complete replacement state for an immutable, exact work unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Record {
    /// Domain-separated key supplied by the coordinator.
    pub(crate) key: [u8; 32],
    /// Canonical single-symbol spot form, nonempty and at most 768 UTF-8 bytes.
    pub(crate) body: String,
    /// Explicitly classified work state.
    pub(crate) status: Status,
    /// Attempts already reserved, including any interrupted attempt.
    pub(crate) attempts: u32,
    /// The coordinator's unchanged-progress counter.
    pub(crate) unchanged: u32,
    /// Actual committed source records, as reported by the coordinator.
    pub(crate) committed: u64,
    /// The coordinator's diagnostic count.
    pub(crate) diagnostics: u64,
    /// Missing coverage quantity measured by the coordinator, not diagnostics.
    pub(crate) missing: u64,
    /// Unverified coverage quantity measured by the coordinator, not diagnostics.
    pub(crate) unverified: u64,
    /// Last HTTP status, or zero when there was no response.
    pub(crate) http_status: u16,
}

impl Record {
    fn image(&self) -> io::Result<[u8; RECORD_LEN]> {
        if self.body.is_empty() || self.body.len() > MAX_BODY_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "recovery body must contain 1..=768 UTF-8 bytes; nothing was truncated",
            ));
        }
        let body_len = u16::try_from(self.body.len()).map_err(invalid_input)?;
        let mut image = [0; RECORD_LEN];
        put(&mut image, 0, &MAGIC)?;
        put(&mut image, 4, &VERSION.to_le_bytes())?;
        put(&mut image, 6, &[self.status.code()])?;
        put(&mut image, 8, &self.key)?;
        put(&mut image, 40, &self.attempts.to_le_bytes())?;
        put(&mut image, 44, &self.unchanged.to_le_bytes())?;
        put(&mut image, 48, &self.committed.to_le_bytes())?;
        put(&mut image, 56, &self.diagnostics.to_le_bytes())?;
        put(&mut image, 64, &self.http_status.to_le_bytes())?;
        put(&mut image, 66, &body_len.to_le_bytes())?;
        put(&mut image, BODY_OFFSET, self.body.as_bytes())?;
        put(&mut image, MISSING_OFFSET, &self.missing.to_le_bytes())?;
        put(
            &mut image,
            UNVERIFIED_OFFSET,
            &self.unverified.to_le_bytes(),
        )?;
        let checksum = checksum(&image);
        put(&mut image, CRC_OFFSET, &checksum.to_le_bytes())?;
        Ok(image)
    }

    fn decode(image: &[u8; RECORD_LEN]) -> io::Result<Self> {
        if take::<4>(image, 0)? != MAGIC {
            return Err(invalid_data("not a recovery journal record (magic)"));
        }
        if u32::from_le_bytes(take(image, CRC_OFFSET)?) != checksum(image) {
            return Err(invalid_data("recovery record CRC-32C mismatch"));
        }
        let version = u16::from_le_bytes(take(image, 4)?);
        if version != VERSION {
            return Err(invalid_data(format!(
                "unknown recovery record version {version}; expected {VERSION}"
            )));
        }
        let [code] = take(image, 6)?;
        let status = Status::decode(code)?;
        let body_len = usize::from(u16::from_le_bytes(take(image, 66)?));
        if body_len == 0 || body_len > MAX_BODY_BYTES {
            return Err(invalid_data(
                "recovery record body length is outside 1..=768",
            ));
        }
        let end = BODY_OFFSET + body_len;
        if take::<1>(image, 7)? != [0]
            || image.iter().skip(68).take(12).any(|byte| *byte != 0)
            || image
                .iter()
                .skip(end)
                .take(MISSING_OFFSET - end)
                .any(|byte| *byte != 0)
            || image
                .iter()
                .skip(RESERVED_OFFSET)
                .take(CRC_OFFSET - RESERVED_OFFSET)
                .any(|byte| *byte != 0)
        {
            return Err(invalid_data(
                "recovery record reserved bytes or body padding are nonzero",
            ));
        }
        let bytes = image
            .get(BODY_OFFSET..end)
            .ok_or_else(|| invalid_data("recovery body exceeds its record"))?;
        let body = std::str::from_utf8(bytes)
            .map_err(|_| invalid_data("recovery record body is not UTF-8"))?
            .to_owned();
        Ok(Self {
            key: take(image, 8)?,
            body,
            status,
            attempts: u32::from_le_bytes(take(image, 40)?),
            unchanged: u32::from_le_bytes(take(image, 44)?),
            committed: u64::from_le_bytes(take(image, 48)?),
            diagnostics: u64::from_le_bytes(take(image, 56)?),
            missing: u64::from_le_bytes(take(image, MISSING_OFFSET)?),
            unverified: u64::from_le_bytes(take(image, UNVERIFIED_OFFSET)?),
            http_status: u16::from_le_bytes(take(image, 64)?),
        })
    }
}

/// A locked progress file and the index recovered from its durable records.
///
/// Callers may inspect `latest` and `order`; change them only through `append`.
/// The sole file handle (including its exclusive lock) lives in `io` until
/// this object is dropped. No clone or independent unlocked append is exposed.
#[derive(Debug)]
pub(crate) struct Journal {
    /// Most recently synced state for each work key.
    pub(crate) latest: HashMap<[u8; 32], Record>,
    /// Distinct work keys in first-appearance order, stable across restart.
    pub(crate) order: Vec<[u8; 32]>,
    io: Box<dyn JournalIo>,
    bytes: u64,
    poisoned: bool,
    named_file: Option<(PathBuf, u64, u64)>,
}

impl Journal {
    /// Open or create a journal in an existing directory and replay it once.
    ///
    /// # Errors
    /// A busy lock, inaccessible path, unsupported record, corrupted history,
    /// torn tail, allocation failure, or file/directory sync failure. Nothing
    /// is truncated, and a failed open exposes no partially recovered index.
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        Self::open_at(path, true, false)
            .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))
    }

    /// Reopen a recorded identity without ever creating its missing history.
    pub(crate) fn open_existing(path: &Path) -> io::Result<Self> {
        Self::open_at(path, false, false)
            .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))
    }

    /// Claim a new filename without opening or modifying an existing generation.
    pub(crate) fn create_new(path: &Path) -> io::Result<Self> {
        Self::open_at(path, false, true)
            .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))
    }

    fn open_at(path: &Path, create: bool, create_new: bool) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(create)
            .create_new(create_new)
            .open(path)?;
        file.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => io::Error::new(
                io::ErrorKind::WouldBlock,
                "recovery journal is already exclusively locked; no work was admitted",
            ),
            TryLockError::Error(error) => error,
        })?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(invalid_data("recovery journal is not a regular file"));
        }
        let bytes = metadata.len();
        let index = replay(
            &mut BufReader::with_capacity(REPLAY_BUFFER_BYTES, &mut file),
            bytes,
        )?;
        if file.metadata()?.len() != bytes {
            return Err(invalid_data(
                "recovery journal length changed during locked replay",
            ));
        }
        // A complete record can survive an earlier failed sync. Make that
        // record durable before the recovered index permits further work.
        file.sync_all()?;
        // Sync the containing directory too, so a newly created journal's
        // directory entry is not merely an unsynced promise. The parent must
        // already exist: recursive directory creation is the caller's job.
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        File::open(parent)?.sync_all()?;
        Ok(Self {
            latest: index.latest,
            order: index.order,
            io: Box::new(file),
            bytes,
            poisoned: false,
            named_file: Some((path.to_owned(), metadata.dev(), metadata.ino())),
        })
    }

    /// Persist exactly one state, then publish it to the in-memory index.
    ///
    /// # Errors
    /// Invalid bodies and changed key/body associations refuse before I/O and
    /// do not poison the handle. Any length/read/write/sync uncertainty poisons
    /// it and leaves both public indexes unchanged. A poisoned handle refuses
    /// every later append; an error is never permission to retry it blindly.
    pub(crate) fn append(&mut self, record: Record) -> io::Result<()> {
        if self.poisoned {
            return Err(io::Error::other(
                "recovery journal is poisoned after uncertain I/O; drop and reopen it",
            ));
        }
        let image = record.image()?;
        reserve_unit(
            &mut self.latest,
            &mut self.order,
            &record,
            io::ErrorKind::InvalidInput,
        )?;
        let next_bytes = self.bytes.checked_add(RECORD_LEN_U64).ok_or_else(|| {
            io::Error::other("recovery journal length cannot fit its next record")
        })?;
        // Index capacity was reserved above: successful durable writes are
        // followed only by moving already-owned data into those containers.
        if let Err(error) = self.persist(&image) {
            self.poisoned = true;
            return Err(error);
        }
        publish(&mut self.latest, &mut self.order, record);
        self.bytes = next_bytes;
        Ok(())
    }

    fn persist(&mut self, image: &[u8; RECORD_LEN]) -> io::Result<()> {
        self.check_named_file()?;
        if self.io.length()? != self.bytes {
            return Err(invalid_data(
                "locked recovery journal length changed; refused to append after foreign bytes",
            ));
        }
        self.io.write_all(image).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("recovery append may be partial: {error}"),
            )
        })?;
        self.io.durable_sync().map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("recovery record written but durability is uncertain: {error}"),
            )
        })?;
        self.check_named_file()
    }

    fn check_named_file(&self) -> io::Result<()> {
        if let Some((path, device, inode)) = &self.named_file {
            let metadata = std::fs::metadata(path).map_err(|why| {
                io::Error::new(
                    why.kind(),
                    format!(
                        "recovery journal filename is unavailable; append was not published: {why}"
                    ),
                )
            })?;
            if !metadata.is_file() || metadata.dev() != *device || metadata.ino() != *inode {
                return Err(invalid_data(
                    "recovery journal filename changed file identity; append was not published",
                ));
            }
        }
        Ok(())
    }
}

/// Read at most `limit` recent events, newest first, without a writer lock.
///
/// This is not a current inventory or a whole-journal validation: earlier
/// records are not read, repeated keys remain separate events, and the newest
/// event for a key may be outside this bounded window. No file is created or
/// changed. A complete, unchanged length before and after reading bounds the
/// snapshot; a later append naturally belongs to the next poll.
/// A live writer's complete bytes can be visible before its sync finishes:
/// recent events are therefore not a durability receipt either. Only the
/// writer's successfully published index (or a successful reopen) proves that.
///
/// # Errors
/// More than [`MAX_TAIL_RECORDS`] requested, an unreadable file, any ragged
/// tail, a corrupt selected record, or a length change during the read. A
/// concurrent writer is never reported as a clean empty/finished inventory.
pub(crate) fn tail(path: &Path, limit: usize) -> io::Result<Vec<Record>> {
    if limit > MAX_TAIL_RECORDS {
        return Err(invalid_input(format!(
            "recovery tail limit exceeds {MAX_TAIL_RECORDS} events"
        )));
    }
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(invalid_data("recovery journal is not a regular file"));
    }
    let bytes = metadata.len();
    let count = whole_records(bytes)?;
    let wanted = usize::try_from(count.min(u64::try_from(limit).map_err(invalid_input)?))
        .map_err(invalid_data)?;
    let records = read_tail(
        &mut BufReader::with_capacity(REPLAY_BUFFER_BYTES, &mut file),
        count,
        wanted,
    )?;
    let after = file.metadata()?.len();
    unchanged_snapshot(bytes, after)?;
    Ok(records)
}

fn unchanged_snapshot(bytes: u64, after: u64) -> io::Result<()> {
    whole_records(after)?;
    if after != bytes {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "recovery journal changed during the recent-events read; retry the snapshot",
        ));
    }
    Ok(())
}

fn read_tail(
    reader: &mut (impl Read + Seek),
    count: u64,
    wanted: usize,
) -> io::Result<Vec<Record>> {
    let first = count - u64::try_from(wanted).map_err(invalid_input)?;
    reader.seek(SeekFrom::Start(first * RECORD_LEN_U64))?;
    let mut records = Vec::new();
    records.try_reserve(wanted).map_err(io::Error::other)?;
    let mut image = [0; RECORD_LEN];
    for _ in 0..wanted {
        reader.read_exact(&mut image)?;
        records.push(Record::decode(&image)?);
    }
    records.reverse();
    Ok(records)
}

fn whole_records(bytes: u64) -> io::Result<u64> {
    if !bytes.is_multiple_of(RECORD_LEN_U64) {
        return Err(invalid_data(format!(
            "recovery journal has {} trailing bytes after {} whole records; no truncation or repair was attempted",
            bytes % RECORD_LEN_U64,
            bytes / RECORD_LEN_U64,
        )));
    }
    Ok(bytes / RECORD_LEN_U64)
}

/// Private fault-injection seam; production owns exactly one locked `File`.
trait JournalIo: Write + Send + Sync + std::fmt::Debug {
    fn length(&self) -> io::Result<u64>;
    fn durable_sync(&mut self) -> io::Result<()>;
}

impl JournalIo for File {
    fn length(&self) -> io::Result<u64> {
        self.metadata().map(|metadata| metadata.len())
    }

    fn durable_sync(&mut self) -> io::Result<()> {
        self.sync_all()
    }
}

/// Validated, locked readback only. It is not a new durability or coverage receipt.
#[derive(Default, Debug)]
pub(crate) struct Index {
    pub(crate) latest: HashMap<[u8; 32], Record>,
    pub(crate) order: Vec<[u8; 32]>,
}

/// Read every existing record without opening for write, creating, or syncing.
/// A live writer refuses the snapshot; callers must not infer an empty inventory.
/// Cost is O(records) time and O(distinct work units) memory.
pub(crate) fn snapshot(path: &Path) -> io::Result<Index> {
    let mut file = File::open(path)?;
    file.try_lock_shared().map_err(|error| match error {
        TryLockError::WouldBlock => io::Error::new(
            io::ErrorKind::WouldBlock,
            "recovery journal has a writer; read-only inventory is unavailable",
        ),
        TryLockError::Error(error) => error,
    })?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(invalid_data("recovery journal is not a regular file"));
    }
    let index = replay(
        &mut BufReader::with_capacity(REPLAY_BUFFER_BYTES, &mut file),
        metadata.len(),
    )?;
    unchanged_snapshot(metadata.len(), file.metadata()?.len())?;
    Ok(index)
}

fn replay(reader: &mut impl Read, bytes: u64) -> io::Result<Index> {
    let count = whole_records(bytes)?;
    let mut index = Index::default();
    let mut image = [0; RECORD_LEN];
    for ordinal in 0..count {
        reader.read_exact(&mut image).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("cannot read recovery record {ordinal}: {error}"),
            )
        })?;
        let record = Record::decode(&image).map_err(|error| {
            io::Error::new(error.kind(), format!("recovery record {ordinal}: {error}"))
        })?;
        reserve_unit(
            &mut index.latest,
            &mut index.order,
            &record,
            io::ErrorKind::InvalidData,
        )?;
        publish(&mut index.latest, &mut index.order, record);
    }
    Ok(index)
}

fn reserve_unit(
    latest: &mut HashMap<[u8; 32], Record>,
    order: &mut Vec<[u8; 32]>,
    record: &Record,
    mismatch: io::ErrorKind,
) -> io::Result<()> {
    if let Some(previous) = latest.get(&record.key) {
        if previous.body != record.body {
            return Err(io::Error::new(
                mismatch,
                "recovery key is already bound to a different exact request body",
            ));
        }
    } else {
        latest.try_reserve(1).map_err(io::Error::other)?;
        order.try_reserve(1).map_err(io::Error::other)?;
    }
    Ok(())
}

fn publish(latest: &mut HashMap<[u8; 32], Record>, order: &mut Vec<[u8; 32]>, record: Record) {
    let key = record.key;
    if latest.insert(key, record).is_none() {
        order.push(key);
    }
}

fn put(image: &mut [u8; RECORD_LEN], offset: usize, bytes: &[u8]) -> io::Result<()> {
    image
        .get_mut(offset..offset + bytes.len())
        .ok_or_else(|| invalid_data("recovery field exceeds its fixed record"))?
        .copy_from_slice(bytes);
    Ok(())
}

fn take<const N: usize>(image: &[u8; RECORD_LEN], offset: usize) -> io::Result<[u8; N]> {
    image
        .get(offset..offset + N)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| invalid_data("recovery field exceeds its fixed record"))
}

fn checksum(image: &[u8; RECORD_LEN]) -> u32 {
    // CRC_OFFSET is proved below RECORD_LEN by the compile-time assertion.
    let (covered, _) = image.split_at(CRC_OFFSET);
    crc32c(covered)
}

fn invalid_data(message: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn invalid_input(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "brutex-recovery-journal-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
            ));
            std::fs::create_dir(&directory).expect("unique scratch directory");
            Self(directory)
        }

        fn path(&self) -> PathBuf {
            self.0.join("recovery-v1.journal")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn record(key: u8) -> Record {
        Record {
            key: [key; 32],
            body: "target=fno&vendor=zerodha&cash_identity=zerodha_cross_checked&member=GVT%26D&granularity=1min&from=2019-12-01&to=2026-09-04".to_owned(),
            status: Status::Queued,
            attempts: 0,
            unchanged: 0,
            committed: 0,
            diagnostics: 0,
            missing: 0,
            unverified: 0,
            http_status: 0,
        }
    }

    fn reseal(image: &mut [u8; RECORD_LEN]) {
        let crc = checksum(image);
        image[CRC_OFFSET..].copy_from_slice(&crc.to_le_bytes());
    }

    #[test]
    fn exact_scope_and_every_field_round_trip_without_reinterpretation() {
        let scratch = Scratch::new();
        let mut expected = record(53);
        expected.status = Status::Unverified;
        expected.attempts = u32::MAX;
        expected.unchanged = 17;
        expected.committed = u64::MAX;
        expected.diagnostics = 0x0102_0304_0506_0708;
        expected.missing = 0x1213_1415_1617_1819;
        expected.unverified = 0x2122_2324_2526_2728;
        expected.http_status = 503;
        let mut journal = Journal::open(&scratch.path()).expect("create journal");
        assert!(journal.latest.is_empty());
        assert!(journal.order.is_empty());
        journal.append(expected.clone()).expect("durable append");
        assert_eq!(journal.latest.get(&expected.key), Some(&expected));
        assert_eq!(journal.order, vec![expected.key]);
        let bytes = std::fs::read(scratch.path()).expect("persisted bytes");
        assert_eq!(bytes.len(), 1_024);
        assert_eq!(&bytes[0..8], b"BXRJ\x01\x00\x03\x00");
        assert_eq!(&bytes[8..40], &[53; 32]);
        assert_eq!(&bytes[40..44], &[255; 4]);
        assert_eq!(&bytes[44..48], &[17, 0, 0, 0]);
        assert_eq!(&bytes[48..56], &[255; 8]);
        assert_eq!(&bytes[56..64], &[8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(&bytes[64..66], &[247, 1]);
        assert_eq!(
            usize::from(u16::from_le_bytes([bytes[66], bytes[67]])),
            expected.body.len()
        );
        assert!(bytes[68..80].iter().all(|byte| *byte == 0));
        assert_eq!(
            &bytes[80..80 + expected.body.len()],
            expected.body.as_bytes()
        );
        assert!(
            bytes[80 + expected.body.len()..848]
                .iter()
                .all(|byte| *byte == 0)
        );
        assert_eq!(
            &bytes[848..856],
            &[0x19, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12]
        );
        assert_eq!(
            &bytes[856..864],
            &[0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21]
        );
        assert!(bytes[864..1_020].iter().all(|byte| *byte == 0));
        assert_eq!(&bytes[1_020..], &crc32c(&bytes[..1_020]).to_le_bytes());
        drop(journal);
        let recovered = Journal::open(&scratch.path()).expect("reopen");
        assert_eq!(recovered.latest.get(&expected.key), Some(&expected));
        assert_eq!(recovered.order, vec![expected.key]);
    }

    #[test]
    fn every_status_has_a_fixed_encoding_and_unknown_codes_refuse() {
        for (code, status) in [
            Status::Queued,
            Status::InFlight,
            Status::Verified,
            Status::Unverified,
            Status::Exhausted,
            Status::Blocked,
            Status::NotApplicable,
        ]
        .into_iter()
        .enumerate()
        {
            let mut original = record(1);
            original.status = status;
            original.http_status = u16::MAX;
            original.unchanged = u32::MAX;
            let image = original.image().expect("encode");
            assert_eq!(usize::from(image[6]), code);
            assert_eq!(Record::decode(&image).expect("decode"), original);
        }
        for code in 7..=u8::MAX {
            let mut image = record(1).image().expect("encode");
            image[6] = code;
            reseal(&mut image);
            assert!(
                Record::decode(&image)
                    .expect_err("unknown status")
                    .to_string()
                    .contains("status")
            );
        }
    }

    #[test]
    fn reopen_preserves_in_flight_and_terminal_budgets_and_first_seen_order() {
        let scratch = Scratch::new();
        let mut journal = Journal::open(&scratch.path()).expect("create");
        let mut first = record(7);
        journal.append(first.clone()).expect("queue first");
        let mut second = record(9);
        second.status = Status::InFlight;
        second.attempts = 2;
        second.unchanged = 1;
        journal.append(second.clone()).expect("reserve second");
        first.status = Status::Exhausted;
        first.attempts = 4;
        first.unchanged = 3;
        first.committed = 19;
        first.diagnostics = 16;
        journal.append(first.clone()).expect("exhaust first");
        let snapshot = journal.latest.clone();
        drop(journal);
        let mut reopened = Journal::open(&scratch.path()).expect("replay");
        assert_eq!(reopened.latest, snapshot);
        assert_eq!(reopened.order, vec![first.key, second.key]);
        assert_eq!(reopened.bytes, 3 * RECORD_LEN_U64);
        reopened
            .append(second.clone())
            .expect("duplicate state is an audit event");
        assert_eq!(reopened.order, vec![first.key, second.key]);
        assert_eq!(reopened.latest.len(), 2);
        assert_eq!(reopened.bytes, 4 * RECORD_LEN_U64);
    }

    #[test]
    fn coverage_updates_preserve_equal_attempts_and_keep_quantities_separate() {
        let scratch = Scratch::new();
        let mut journal = Journal::open(&scratch.path()).expect("create");
        let mut event = record(1);
        event.status = Status::Unverified;
        event.attempts = 2;
        event.unchanged = 1;
        event.diagnostics = 4;
        event.missing = 30;
        event.unverified = 5;
        journal.append(event.clone()).expect("initial observation");
        event.diagnostics = 2;
        event.missing = 0;
        event.unverified = 10;
        journal
            .append(event.clone())
            .expect("readback without another attempt");
        assert_eq!(journal.latest.get(&event.key), Some(&event));
        assert_eq!(journal.order, vec![event.key]);
        drop(journal);
        let reopened = Journal::open(&scratch.path()).expect("replay");
        assert_eq!(reopened.latest.get(&event.key), Some(&event));
        let recent = tail(&scratch.path(), 2).expect("recent observations");
        assert_eq!(recent[0].attempts, 2);
        assert_eq!(recent[1].attempts, 2);
        assert_eq!(
            (
                recent[0].diagnostics,
                recent[0].missing,
                recent[0].unverified
            ),
            (2, 0, 10)
        );
        assert_eq!(
            (
                recent[1].diagnostics,
                recent[1].missing,
                recent[1].unverified
            ),
            (4, 30, 5)
        );
    }

    #[test]
    fn bodies_at_capacity_are_exact_and_invalid_inputs_do_not_poison() {
        let scratch = Scratch::new();
        let mut journal = Journal::open(&scratch.path()).expect("create");
        for body in [
            String::new(),
            "x".repeat(769),
            format!("{}é", "x".repeat(767)),
        ] {
            let mut invalid = record(1);
            invalid.body = body;
            assert_eq!(
                journal.append(invalid).expect_err("length refused").kind(),
                io::ErrorKind::InvalidInput
            );
            assert!(!journal.poisoned);
            assert_eq!(std::fs::metadata(scratch.path()).expect("file").len(), 0);
            assert!(journal.latest.is_empty());
            assert!(journal.order.is_empty());
        }
        let mut maximum = record(1);
        maximum.body = format!("{}é", "x".repeat(766));
        journal.append(maximum.clone()).expect("768 bytes exactly");
        drop(journal);
        let recovered = Journal::open(&scratch.path()).expect("reopen max body");
        assert_eq!(recovered.latest.get(&maximum.key), Some(&maximum));
    }

    #[test]
    fn key_cannot_silently_rebind_to_another_scope_on_append_or_replay() {
        let scratch = Scratch::new();
        let first = record(2);
        let mut changed = first.clone();
        changed.body = first.body.replace("granularity=1min", "granularity=1day");
        let mut journal = Journal::open(&scratch.path()).expect("create");
        journal.append(first.clone()).expect("first");
        let before = std::fs::read(scratch.path()).expect("bytes");
        assert_eq!(
            journal
                .append(changed.clone())
                .expect_err("scope conflict")
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(!journal.poisoned);
        assert_eq!(journal.latest.get(&first.key), Some(&first));
        assert_eq!(std::fs::read(scratch.path()).expect("bytes"), before);
        drop(journal);
        let mut illegal_history = before;
        illegal_history.extend_from_slice(&changed.image().expect("valid bytes, wrong binding"));
        std::fs::write(scratch.path(), &illegal_history).expect("fixture");
        assert_eq!(
            Journal::open(&scratch.path())
                .expect_err("replay scope conflict")
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            std::fs::read(scratch.path()).expect("unchanged"),
            illegal_history
        );
    }

    #[test]
    fn partial_tail_refuses_without_truncating_any_byte() {
        for tail in [1, 17, RECORD_LEN - 1] {
            let scratch = Scratch::new();
            let mut bytes = record(1).image().expect("whole record").to_vec();
            bytes.extend(std::iter::repeat_n(0xab, tail));
            std::fs::write(scratch.path(), &bytes).expect("torn fixture");
            let error = Journal::open(&scratch.path()).expect_err("torn tail");
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(error.to_string().contains("trailing bytes"));
            assert_eq!(
                std::fs::read(scratch.path()).expect("preserved tail"),
                bytes
            );
        }
    }

    #[test]
    fn corrupt_magic_crc_version_status_and_lengths_refuse_the_entire_open() {
        let original = record(1).image().expect("encode");
        let mut corruptions = Vec::new();
        let mut magic = original;
        magic[0] = b'?';
        reseal(&mut magic);
        corruptions.push((magic, "magic"));
        let mut crc = original;
        crc[50] ^= 1;
        corruptions.push((crc, "CRC-32C"));
        for version in [0u16, 2, u16::MAX] {
            let mut image = original;
            image[4..6].copy_from_slice(&version.to_le_bytes());
            reseal(&mut image);
            corruptions.push((image, "version"));
        }
        let mut status = original;
        status[6] = 99;
        reseal(&mut status);
        corruptions.push((status, "status"));
        for length in [0u16, 769, u16::MAX] {
            let mut image = original;
            image[66..68].copy_from_slice(&length.to_le_bytes());
            reseal(&mut image);
            corruptions.push((image, "body length"));
        }
        let mut utf8 = original;
        utf8[BODY_OFFSET] = 0xff;
        reseal(&mut utf8);
        corruptions.push((utf8, "UTF-8"));
        for (image, message) in corruptions {
            let scratch = Scratch::new();
            let mut bytes = original.to_vec();
            bytes.extend_from_slice(&image);
            std::fs::write(scratch.path(), &bytes).expect("fixture");
            let error = Journal::open(&scratch.path()).expect_err("corrupt second record");
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(error.to_string().contains(message), "{error}");
            assert!(error.to_string().contains("record 1"), "{error}");
            assert_eq!(std::fs::read(scratch.path()).expect("unchanged"), bytes);
        }
    }

    #[test]
    fn every_reserved_and_padding_byte_is_checked_even_with_a_valid_crc() {
        let original = record(1);
        let bytes = original.image().expect("encode");
        for offset in std::iter::once(7)
            .chain(68..80)
            .chain(BODY_OFFSET + original.body.len()..MISSING_OFFSET)
            .chain(RESERVED_OFFSET..CRC_OFFSET)
        {
            let mut image = bytes;
            image[offset] = 1;
            reseal(&mut image);
            assert!(
                Record::decode(&image)
                    .expect_err("reserved byte")
                    .to_string()
                    .contains("reserved"),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn checksum_covers_every_non_checksum_byte_and_checksum_damage_is_refused() {
        let original = record(1).image().expect("encode");
        for offset in 0..RECORD_LEN {
            let mut damaged = original;
            damaged[offset] ^= 1;
            assert!(Record::decode(&damaged).is_err(), "unchecked byte {offset}");
        }
    }

    #[test]
    fn exclusive_lock_lasts_for_the_handle_and_releases_on_drop() {
        let scratch = Scratch::new();
        let mut first = Journal::open(&scratch.path()).expect("first handle");
        first.append(record(1)).expect("first append");
        assert_eq!(
            Journal::open(&scratch.path())
                .expect_err("second handle refused")
                .kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(
            std::fs::metadata(scratch.path()).expect("length").len(),
            RECORD_LEN_U64
        );
        drop(first);
        let reopened = Journal::open(&scratch.path()).expect("lock released");
        assert_eq!(reopened.latest.len(), 1);
    }

    #[test]
    fn live_writer_remains_locked_while_tail_returns_bounded_newest_first_events() {
        let scratch = Scratch::new();
        let mut journal = Journal::open(&scratch.path()).expect("writer");
        assert!(tail(&scratch.path(), 10).expect("empty events").is_empty());
        for attempt in 0..7 {
            let mut event = record(1);
            event.attempts = attempt;
            journal.append(event).expect("append");
        }
        let recent = tail(&scratch.path(), 3).expect("read with writer alive");
        assert_eq!(
            recent
                .iter()
                .map(|event| event.attempts)
                .collect::<Vec<_>>(),
            vec![6, 5, 4]
        );
        assert_eq!(recent.len(), 3);
        assert_eq!(
            Journal::open(&scratch.path())
                .expect_err("reader did not release writer's lock")
                .kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(journal.latest.len(), 1);
        assert_eq!(
            tail(&scratch.path(), MAX_TAIL_RECORDS)
                .expect("limit exceeds actual events")
                .len(),
            7
        );
        assert!(tail(&scratch.path(), 0).expect("zero requested").is_empty());
        assert_eq!(
            tail(&scratch.path(), usize::MAX)
                .expect_err("bounded request")
                .kind(),
            io::ErrorKind::InvalidInput
        );
        let mut next = record(1);
        next.attempts = 7;
        journal.append(next.clone()).expect("writer still usable");
        assert_eq!(tail(&scratch.path(), 1).expect("fresh poll"), vec![next]);
    }

    #[test]
    fn tail_does_not_replay_corrupt_prefix_and_refuses_corruption_inside_its_window() {
        let scratch = Scratch::new();
        let first = record(1);
        let last = record(2);
        let mut bytes = first.image().expect("first").to_vec();
        bytes[50] ^= 1;
        bytes.extend_from_slice(&last.image().expect("last"));
        std::fs::write(scratch.path(), &bytes).expect("fixture");
        assert_eq!(
            tail(&scratch.path(), 1).expect("only reads last"),
            vec![last]
        );
        assert_eq!(
            tail(&scratch.path(), 2)
                .expect_err("selected corrupt record")
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            Journal::open(&scratch.path())
                .expect_err("full replay still refuses")
                .kind(),
            io::ErrorKind::InvalidData
        );
        bytes.push(0);
        std::fs::write(scratch.path(), &bytes).expect("torn active-writer shape");
        for limit in [0, 1, MAX_TAIL_RECORDS] {
            assert!(
                tail(&scratch.path(), limit)
                    .expect_err("no false clean tail")
                    .to_string()
                    .contains("trailing bytes")
            );
        }
        assert_eq!(std::fs::read(scratch.path()).expect("read-only"), bytes);
    }

    #[test]
    fn tail_does_not_create_missing_files() {
        let scratch = Scratch::new();
        assert_eq!(
            tail(&scratch.path(), 1).expect_err("missing").kind(),
            io::ErrorKind::NotFound
        );
        assert!(!scratch.path().exists());
    }

    #[test]
    fn existing_open_and_snapshot_never_create_and_new_identity_never_replaces() {
        let scratch = Scratch::new();
        assert_eq!(
            Journal::open_existing(&scratch.path())
                .expect_err("missing history")
                .kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            snapshot(&scratch.path())
                .expect_err("read-only missing history")
                .kind(),
            io::ErrorKind::NotFound
        );
        assert!(!scratch.path().exists());
        let mut journal = Journal::create_new(&scratch.path()).expect("new identity");
        let row = record(1);
        journal.append(row.clone()).expect("first event");
        let bytes = std::fs::read(scratch.path()).expect("saved bytes");
        assert_eq!(
            Journal::create_new(&scratch.path())
                .expect_err("existing identity")
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            snapshot(&scratch.path()).expect_err("live writer").kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(std::fs::read(scratch.path()).expect("unchanged"), bytes);
        drop(journal);
        let inventory = snapshot(&scratch.path()).expect("read-only complete replay");
        assert_eq!(inventory.latest, HashMap::from([(row.key, row.clone())]));
        assert_eq!(inventory.order, vec![row.key]);
        assert_eq!(std::fs::read(scratch.path()).expect("read-only"), bytes);
        let reopened = Journal::open_existing(&scratch.path()).expect("existing resume");
        assert_eq!(reopened.latest.get(&row.key), Some(&row));
    }

    #[test]
    fn deleted_or_replaced_filename_refuses_append_and_preserves_the_old_index() {
        for replacement in [false, true] {
            let scratch = Scratch::new();
            let mut journal = Journal::create_new(&scratch.path()).expect("new journal");
            let first = record(1);
            journal.append(first.clone()).expect("first event");
            let preserved = scratch.0.join("original-preserved.bin");
            std::fs::rename(scratch.path(), &preserved).expect("external move fixture");
            let original = std::fs::read(&preserved).expect("original bytes");
            if replacement {
                std::fs::write(scratch.path(), record(3).image().expect("replacement"))
                    .expect("different inode");
            }
            assert!(
                journal
                    .append(record(2))
                    .expect_err("named history changed")
                    .to_string()
                    .contains("filename")
            );
            assert!(journal.poisoned);
            assert_eq!(journal.latest, HashMap::from([(first.key, first)]));
            assert_eq!(
                std::fs::read(&preserved).expect("old inode not appended"),
                original
            );
            if replacement {
                assert_eq!(
                    std::fs::read(scratch.path()).expect("replacement untouched"),
                    record(3).image().expect("replacement")
                );
            } else {
                assert!(
                    !scratch.path().exists(),
                    "missing identity was not recreated"
                );
            }
        }
    }

    #[test]
    fn snapshot_checks_the_entire_prefix_and_refuses_ragged_history_without_changes() {
        let scratch = Scratch::new();
        let mut image = record(1).image().expect("record").to_vec();
        image[50] ^= 1;
        image.extend_from_slice(&record(2).image().expect("valid tail"));
        std::fs::write(scratch.path(), &image).expect("corrupt prefix fixture");
        assert_eq!(
            tail(&scratch.path(), 1).expect("bounded event only").len(),
            1
        );
        assert_eq!(
            snapshot(&scratch.path())
                .expect_err("complete prefix validation")
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(std::fs::read(scratch.path()).expect("preserved"), image);
        image = record(1).image().expect("record").to_vec();
        image.push(0);
        std::fs::write(scratch.path(), &image).expect("ragged fixture");
        assert_eq!(
            snapshot(&scratch.path())
                .expect_err("partial append")
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(std::fs::read(scratch.path()).expect("preserved"), image);
    }

    #[test]
    fn tail_snapshot_growth_shrinkage_and_ragged_growth_never_report_clean() {
        unchanged_snapshot(2 * RECORD_LEN_U64, 2 * RECORD_LEN_U64)
            .expect("unchanged complete snapshot");
        for after in [0, RECORD_LEN_U64, 3 * RECORD_LEN_U64] {
            assert_eq!(
                unchanged_snapshot(2 * RECORD_LEN_U64, after)
                    .expect_err("changed complete length")
                    .kind(),
                io::ErrorKind::WouldBlock,
            );
        }
        assert_eq!(
            unchanged_snapshot(2 * RECORD_LEN_U64, 2 * RECORD_LEN_U64 + 1)
                .expect_err("writer left partial record")
                .kind(),
            io::ErrorKind::InvalidData,
        );
    }

    #[test]
    fn tail_seeks_directly_and_reads_only_requested_fixed_stride_records() {
        struct Measured {
            bytes: io::Cursor<Vec<u8>>,
            read: usize,
            seeks: Vec<u64>,
        }
        impl Read for Measured {
            fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
                let read = self.bytes.read(bytes)?;
                self.read += read;
                Ok(read)
            }
        }
        impl Seek for Measured {
            fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
                let offset = self.bytes.seek(position)?;
                self.seeks.push(offset);
                Ok(offset)
            }
        }
        let mut bytes = Vec::new();
        for key in 0..100 {
            bytes.extend_from_slice(&record(key).image().expect("encode"));
        }
        let mut reader = Measured {
            bytes: io::Cursor::new(bytes),
            read: 0,
            seeks: Vec::new(),
        };
        let records = read_tail(&mut reader, 100, 3).expect("bounded tail");
        assert_eq!(reader.read, 3 * RECORD_LEN);
        assert_eq!(reader.seeks, vec![97 * RECORD_LEN_U64]);
        assert_eq!(
            records.iter().map(|record| record.key).collect::<Vec<_>>(),
            vec![[99; 32], [98; 32], [97; 32]]
        );
    }

    #[test]
    fn nonexistent_parent_is_not_silently_created() {
        let scratch = Scratch::new();
        let path = scratch.0.join("absent").join("recovery.journal");
        assert_eq!(
            Journal::open(&path).expect_err("parent missing").kind(),
            io::ErrorKind::NotFound
        );
        assert!(!scratch.0.join("absent").exists());
        assert!(Journal::open(&scratch.0).is_err());
    }

    #[test]
    fn replay_crosses_buffer_boundaries_keeps_only_latest_and_handles_short_reads() {
        let mut bytes = Vec::new();
        let mut expected = HashMap::new();
        for attempt in 0..150u32 {
            let key = u8::try_from(attempt % 3).expect("small key");
            let mut event = record(key);
            event.status = Status::InFlight;
            event.attempts = attempt;
            expected.insert(event.key, event.clone());
            bytes.extend_from_slice(&event.image().expect("image"));
        }
        let scratch = Scratch::new();
        std::fs::write(scratch.path(), &bytes).expect("large fixture");
        let journal = Journal::open(&scratch.path()).expect("multi-buffer replay");
        assert_eq!(journal.latest, expected);
        assert_eq!(journal.order, vec![[0; 32], [1; 32], [2; 32]]);
        let mut too_short = bytes.as_slice();
        let error = replay(
            &mut too_short,
            u64::try_from(bytes.len()).expect("length") + RECORD_LEN_U64,
        )
        .expect_err("unexpected EOF");
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
        assert!(error.to_string().contains("record 150"));
    }

    #[derive(Debug, Default)]
    struct Memory {
        bytes: Vec<u8>,
        write_budget: Option<usize>,
        fail_sync: bool,
        fail_length: bool,
        writes: usize,
        syncs: usize,
    }

    #[derive(Debug)]
    struct FaultIo(Arc<Mutex<Memory>>);

    impl Write for FaultIo {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let mut memory = self.0.lock().expect("fixture lock");
            memory.writes += 1;
            if memory.write_budget == Some(0) {
                return Err(io::Error::other("injected write failure"));
            }
            let accepted = memory.write_budget.unwrap_or(bytes.len()).min(bytes.len());
            memory.bytes.extend_from_slice(&bytes[..accepted]);
            if let Some(budget) = memory.write_budget.as_mut() {
                *budget -= accepted;
            }
            Ok(accepted)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl JournalIo for FaultIo {
        fn length(&self) -> io::Result<u64> {
            let memory = self.0.lock().expect("fixture lock");
            if memory.fail_length {
                return Err(io::Error::other("injected metadata failure"));
            }
            Ok(u64::try_from(memory.bytes.len()).expect("fixture length"))
        }

        fn durable_sync(&mut self) -> io::Result<()> {
            let mut memory = self.0.lock().expect("fixture lock");
            memory.syncs += 1;
            if memory.fail_sync {
                return Err(io::Error::other("injected sync failure"));
            }
            Ok(())
        }
    }

    fn faulty() -> (Journal, Arc<Mutex<Memory>>) {
        let memory = Arc::new(Mutex::new(Memory::default()));
        let journal = Journal {
            latest: HashMap::new(),
            order: Vec::new(),
            io: Box::new(FaultIo(Arc::clone(&memory))),
            bytes: 0,
            poisoned: false,
            named_file: None,
        };
        (journal, memory)
    }

    #[test]
    fn partial_and_zero_byte_write_failures_do_not_publish_and_poison_the_handle() {
        for budget in [0, 1, 31, RECORD_LEN - 1] {
            let (mut journal, memory) = faulty();
            let old = record(1);
            journal.append(old.clone()).expect("committed first event");
            let mut next = record(2);
            next.status = Status::InFlight;
            next.attempts = 1;
            memory.lock().expect("fixture").write_budget = Some(budget);
            let error = journal
                .append(next.clone())
                .expect_err("injected write failure");
            assert!(error.to_string().contains("partial"));
            assert_eq!(journal.latest.len(), 1);
            assert_eq!(journal.latest.get(&old.key), Some(&old));
            assert_eq!(journal.order, vec![old.key]);
            assert_eq!(journal.bytes, RECORD_LEN_U64);
            assert!(journal.poisoned);
            let mut state = memory.lock().expect("fixture");
            assert_eq!(state.bytes.len(), RECORD_LEN + budget);
            assert_eq!(state.syncs, 1);
            state.write_budget = None;
            let writes = state.writes;
            drop(state);
            assert!(
                journal
                    .append(next)
                    .expect_err("poisoned retry")
                    .to_string()
                    .contains("poisoned")
            );
            assert_eq!(memory.lock().expect("fixture").writes, writes);
            let scratch = Scratch::new();
            std::fs::write(scratch.path(), &memory.lock().expect("fixture").bytes)
                .expect("persist crash image");
            if budget == 0 {
                assert_eq!(
                    Journal::open(&scratch.path())
                        .expect("valid old prefix")
                        .latest
                        .len(),
                    1
                );
            } else {
                assert_eq!(
                    Journal::open(&scratch.path())
                        .expect_err("partial tail")
                        .kind(),
                    io::ErrorKind::InvalidData
                );
            }
        }
    }

    #[test]
    fn sync_failure_preserves_old_index_and_reopen_recovers_the_uncertain_record() {
        let (mut journal, memory) = faulty();
        let mut next = record(1);
        journal.append(next.clone()).expect("queue");
        let previous = journal.latest.clone();
        next.status = Status::InFlight;
        next.attempts = 3;
        next.unchanged = 2;
        memory.lock().expect("fixture").fail_sync = true;
        let error = journal.append(next.clone()).expect_err("sync failure");
        assert!(error.to_string().contains("durability is uncertain"));
        assert_eq!(journal.latest, previous);
        assert_eq!(journal.order, vec![next.key]);
        assert_eq!(journal.bytes, RECORD_LEN_U64);
        assert!(journal.poisoned);
        let mut state = memory.lock().expect("fixture");
        assert_eq!(state.bytes.len(), 2 * RECORD_LEN);
        assert_eq!(state.syncs, 2);
        state.fail_sync = false;
        let writes = state.writes;
        let bytes = state.bytes.clone();
        drop(state);
        assert!(journal.append(next.clone()).is_err());
        assert_eq!(memory.lock().expect("fixture").writes, writes);
        let scratch = Scratch::new();
        std::fs::write(scratch.path(), bytes).expect("surviving complete crash image");
        let mut reopened = Journal::open(&scratch.path()).expect("validate and sync on reopen");
        assert_eq!(reopened.latest.get(&next.key), Some(&next));
        assert_eq!(reopened.order, vec![next.key]);
        next.status = Status::Blocked;
        reopened
            .append(next.clone())
            .expect("fresh handle can append");
        assert_eq!(reopened.latest.get(&next.key), Some(&next));
    }

    #[test]
    fn sync_failure_cannot_publish_a_new_key_or_its_order_entry() {
        let (mut journal, memory) = faulty();
        let first = record(1);
        journal.append(first.clone()).expect("old durable event");
        memory.lock().expect("fixture").fail_sync = true;
        assert!(journal.append(record(2)).is_err());
        assert!(journal.poisoned);
        assert_eq!(journal.latest.len(), 1);
        assert_eq!(journal.latest.get(&first.key), Some(&first));
        assert_eq!(journal.order, vec![first.key]);
        assert_eq!(journal.bytes, RECORD_LEN_U64);
        assert_eq!(memory.lock().expect("fixture").bytes.len(), 2 * RECORD_LEN);
    }

    #[test]
    fn metadata_failure_or_foreign_length_change_poison_without_a_write() {
        for foreign in [false, true] {
            let (mut journal, memory) = faulty();
            let first = record(1);
            journal.append(first.clone()).expect("first event");
            let mut state = memory.lock().expect("fixture");
            if foreign {
                state.bytes.push(0);
            } else {
                state.fail_length = true;
            }
            let writes = state.writes;
            drop(state);
            assert!(journal.append(record(2)).is_err());
            assert!(journal.poisoned);
            assert_eq!(journal.latest.get(&first.key), Some(&first));
            assert_eq!(journal.order, vec![first.key]);
            assert_eq!(memory.lock().expect("fixture").writes, writes);
            assert!(journal.append(record(2)).is_err());
            assert_eq!(memory.lock().expect("fixture").writes, writes);
        }
    }
}
