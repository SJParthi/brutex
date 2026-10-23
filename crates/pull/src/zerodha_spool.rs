//! The Zerodha spool: a run's verdict records, appended as each window or
//! day is verified, until one `verify-merge` process merges them into one
//! verdict generation per (segment, timeframe, day) (IA-9; D-4881, D-4921;
//! `docs/02-store-format.md` §35, "The Zerodha spool").
//!
//! # Why a spool, and not a generation per window
//!
//! A verdict generation is a whole day image, sorted and synced three times
//! (`store::verdict::commit`, L-IA-4). Written once per landed window, a full
//! backfill would write about 3.9M images (about 46 GB, an extrapolation,
//! design §6.3); spooled, the same records are about 0.80 GB written once
//! and merged into about 39,600 generations. So a run appends here, O(1) per
//! record, and the merge writes each touched day once.
//!
//! # The files
//!
//! ```text
//! <store>/verify/spool/<run>/<SEG>/<tf>/<yyyy-mm>.vsp          the records
//! <store>/verify/spool/<run>/<SEG>/<tf>/<yyyy-mm>.vsp.merged   the merge log
//! <store>/verify/merge.lock                                    one merger at a time
//! <store>/verify/live/<run>.lock                               the run is still spooling
//! ```
//!
//! `<run>` is the run's id from the store-wide runs ledger (D-4880): never
//! given twice, so an orphan spool and a new run never share a directory.
//!
//! # The rules (D-4881)
//!
//! * **A 64-byte header** (`BRUTEXVS`, version, stride 160, feed, segment,
//!   timeframe, mode, month, run, created, CRC-32C over bytes 0..60), written
//!   and synced, its directory synced, before any record. A file shorter than
//!   the header, or whose header is all zero, is unborn: the next append
//!   writes the header again, and the merge reads it as holding nothing.
//! * **160-byte records, each with its own CRC-32C at 156.** A verdict record
//!   (kind 1) is the verdict store's record layout (§35) with the day and the
//!   exchange in its reserved bytes and, at offset 120, the receipt it was
//!   compared with instead of a detail offset. A detail record (kind 2)
//!   carries one detail entry (at most [`DETAIL_PAYLOAD`] bytes) of the
//!   verdict record before it.
//! * **One appender, at its own offset** (round 5, B2; critic IA-9-2). Only
//!   the run whose id names the directory appends, through ONE
//!   [`SpoolWriter`] per run: a map from spool file to its handle and the
//!   offset after its last synced record, behind one mutex, so two tasks of
//!   one run appending to one file are serialised and never interleave. An
//!   append is one `write_all_at` at that offset and one `sync_data`; a failed
//!   write leaves the offset where it was, so the next append writes over the
//!   torn bytes (the `store/src/provenance.rs` rule).
//! * **The one reader is the merge** ([`read_spool`]): it takes the
//!   ⌊(len − 64) ÷ 160⌋ whole records; bytes past them are a torn append; a
//!   LAST whole record that fails its CRC is an append that never finished its
//!   sync. Both are ignored and named (offset and bytes). Any EARLIER record
//!   that fails its CRC was damaged in place: the spool is refused by name
//!   ([`SpoolError::Damaged`]), never skipped silently.
//! * **A run that is still spooling is never merged** (IA-9 repair 1). The
//!   writer takes `verify/live/<run>.lock` exclusively at its first append
//!   and holds it until it is dropped (the run's end, or its process's
//!   death, which the kernel turns into a release). A merger try-locks each
//!   run's lock before it reads one of the run's spools: a run whose lock is
//!   held is LIVE, its spools are left as they are and the run is named
//!   ([`live_probe`]); a run whose lock it takes is held by the merger for
//!   the pass, so a writer of that run resuming meanwhile waits for the pass
//!   ([`LIVE_WAIT`]) and never appends to a spool the merger is reading or
//!   removing. Measured before the lock: a `verify-merge` started while a
//!   cold run spooled committed 38,096 of its 39,024 records and lost the
//!   rest with every exit 0. The lock files are never removed: one empty file
//!   per run, so a lock is never unlinked under a holder.
//! * **The merge log** (`.vsp.merged`, 16-byte records {day, generation,
//!   state, exchange, CRC}) says which days are merged, so a re-merge after a
//!   crash writes no duplicate generation; its own torn tail is written over
//!   and an earlier bad record refuses the spool by name (RA-319). Each merge
//!   of a spool first records how many verdict records it read (a `seen`
//!   record): a committed day is skipped only when that commit covered every
//!   record the spool now holds for the day, so records a resumed run appends
//!   to a partly merged spool are merged later, never skipped as merged. When
//!   a spool is merged whole its log is removed FIRST and the spool only
//!   after, so a log never outlives its spool to answer for a new one.
//! * **Every walk accepts names by exact pattern** (RA-315): a `.DS_Store`,
//!   `._name`, `.Spotlight-V100`, `.fseventsd` or any other name that is not
//!   a run, a segment, a timeframe or a spool is counted as stray, never read,
//!   never an error and never deleted.
//!
//! # Cost
//!
//! An append: one `write_all_at` and one `sync_data` per window per file,
//! O(records of the window); the handle map is O(1) expected per lookup and
//! bounded by the files one run touches. [`read_spool`] is O(records of the
//! spool), once, in the merge (L-IA-17). A merge-log step is one hash probe
//! of the log's index of each day's last record, one 16-byte write and one
//! `sync_data` (C-ZV-03); [`read_log`] builds that index once per spool
//! merged, O(records of the log), and refuses a log past [`MAX_LOG_BYTES`].

use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind};
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::vendor::Vendor;
use store::crc::crc32c;
use store::path::{Timeframe, YearMonth};
use store::verdict::{self, Code, Detail, Facts, Key, Mode};

use crate::session::Day;

/// The magic at byte 0 of every spool.
pub const MAGIC: [u8; 8] = *b"BRUTEXVS";
/// The spool format version this build writes and reads.
pub const VERSION: u16 = 1;
/// The header's length.
pub const HEADER_LEN: u64 = 64;
/// One record's length: the verdict store's record stride.
pub const RECORD_LEN: u64 = verdict::RECORD_STRIDE as u64;
/// A merge-log record's length.
pub const LOG_RECORD_LEN: u64 = 16;
/// The most payload bytes one detail record carries.
pub const DETAIL_PAYLOAD: usize = 128;
/// The most details one verdict record carries in a spool (IA-9 repair 1):
/// its count is written in the record (bytes 150..152), so a detail an append
/// never finished is seen, and a writer resuming finds its last whole record
/// in at most this many plus one reads.
pub const MAX_DETAILS: usize = 32;
/// The longest spool the merge reads: 256 MiB, about 1.6M records. One run's
/// records of one (segment, timeframe, month) are about 343 instruments × 23
/// days at the cold rate (about 1.3 MB); a longer file is refused by its length
/// before a byte is allocated, never read into memory whole.
pub const MAX_SPOOL_BYTES: u64 = 256 << 20;
/// The directory, under the store root, that holds every run's spool.
pub const SPOOL_DIR: &str = "verify/spool";
/// The merge lock, under the store root (design §5.3, round 4).
pub const MERGE_LOCK: &str = "verify/merge.lock";
/// A spool's file extension.
pub const SPOOL_EXT: &str = "vsp";
/// A merge log's suffix after the spool's name.
pub const LOG_SUFFIX: &str = ".merged";
/// The directory, under the store root, of every run's live lock.
pub const LIVE_DIR: &str = "verify/live";
/// How long a writer waits for a run's live lock that a merger holds (it is
/// merging this run's spools of an earlier process) before it refuses by
/// name: a run that resumes under its old id while its orphan spools merge.
pub const LIVE_WAIT: std::time::Duration = std::time::Duration::from_secs(30);
/// The pause between two tries of a live lock.
const LIVE_STEP: std::time::Duration = std::time::Duration::from_millis(50);

/// `<root>/verify/live/<run>.lock`: held by run `run`'s writer while it
/// spools, by a merger while it merges the run's spools.
#[must_use]
pub fn live_lock_path(root: &Path, run: u64) -> PathBuf {
    root.join(LIVE_DIR).join(format!("{run}.lock"))
}

/// What a merger's probe of one run's live lock found.
#[derive(Debug)]
pub enum Live {
    /// The run is not spooling: the merger holds its lock until dropped.
    Taken(store::flock::Flock<File>),
    /// The run is still spooling: its holder, in words.
    Spooling(String),
}

/// Tries run `run`'s live lock once, never waiting: [`Live::Taken`] when no
/// writer of the run holds it (O(1): one open, one `flock`).
///
/// # Errors
///
/// The lock could not be opened or the host refused the `flock` call: the
/// caller names it and leaves the run's spools waiting.
pub fn live_probe(root: &Path, run: u64) -> Result<Live, SpoolError> {
    let path = live_lock_path(root, run);
    let file = open_live(root, &path)?;
    match store::flock::Flock::try_lock(file, path.clone()) {
        Ok(lock) => Ok(Live::Taken(lock)),
        Err(std::fs::TryLockError::WouldBlock) => {
            Ok(Live::Spooling(store::flock::holder_note(&path)))
        }
        Err(std::fs::TryLockError::Error(why)) => Err(SpoolError::io(&path, "lock", why)),
    }
}

/// Opens (creating) a live lock file and its folder.
fn open_live(root: &Path, path: &Path) -> Result<File, SpoolError> {
    let dir = root.join(LIVE_DIR);
    fs::create_dir_all(&dir).map_err(|why| SpoolError::io(&dir, "create", why))?;
    crate::gdfl_run::locks::open_lock_path(path, LIVE_DIR).map_err(|why| SpoolError::Io {
        path: path.to_path_buf(),
        action: "open",
        source: io::Error::other(why.to_string()),
    })
}

/// Record kind 1: a verdict record.
const KIND_VERDICT: u8 = 1;
/// Record kind 2: a detail of the verdict record before it.
const KIND_DETAIL: u8 = 2;

/// A merge-log state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogState {
    /// The merger is about to commit the day; `generation` is the one the
    /// year index named before it (0 = none).
    Intent,
    /// The day is committed; `generation` is the one published.
    Committed,
    /// The spool was refused by name; nothing of it is merged until a
    /// `--retry`.
    Refused,
    /// A merge read the spool: `day` holds how many verdict records it read
    /// (`generation` and `exchange` 0). Every intent and commit after it
    /// covers those records and no later one.
    Seen,
}

impl LogState {
    /// The byte on disk.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Intent => 1,
            Self::Committed => 2,
            Self::Refused => 3,
            Self::Seen => 4,
        }
    }

    /// The state a byte names, or `None` for one no writer writes.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Intent),
            2 => Some(Self::Committed),
            3 => Some(Self::Refused),
            4 => Some(Self::Seen),
            _ => None,
        }
    }
}

/// Which spool: one run's records of one segment, timeframe and month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpoolId {
    /// The run, from the runs ledger.
    pub run: u64,
    /// The segment.
    pub segment: Segment,
    /// The timeframe of every record in it.
    pub timeframe: Timeframe,
    /// The month every record's day falls in.
    pub month: YearMonth,
}

impl SpoolId {
    /// `<root>/verify/spool/<run>/<SEG>/<tf>/<yyyy-mm>.vsp`.
    #[must_use]
    pub fn path(&self, root: &Path) -> PathBuf {
        root.join(SPOOL_DIR)
            .join(self.run.to_string())
            .join(self.segment.as_str())
            .join(self.timeframe.as_str())
            .join(format!("{}.{SPOOL_EXT}", self.month))
    }

    /// The merge log beside the spool.
    #[must_use]
    pub fn log_path(&self, root: &Path) -> PathBuf {
        let mut path = self.path(root).into_os_string();
        path.push(LOG_SUFFIX);
        PathBuf::from(path)
    }
}

/// One verdict as a run spools it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpoolRecord {
    /// The exchange whose calendar the day answers to.
    pub exchange: Exchange,
    /// The day.
    pub day: Day,
    /// The instrument key.
    pub key: Key,
    /// What was found.
    pub facts: Facts,
    /// The receipt the record was compared with, as the run numbers them
    /// (1 for its first; 0 for none: a cold record, or a day no answer
    /// covered). The merge orders one key's records of one day by it.
    pub receipt: u32,
    /// Details, each at most [`DETAIL_PAYLOAD`] bytes.
    pub details: Vec<Detail>,
}

/// A spool's header, decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Which spool.
    pub id: SpoolId,
    /// The pass that wrote it: [`Mode::ZerodhaMerge`] for a pull run's,
    /// [`Mode::ZerodhaCold`] for a cold verification's.
    pub mode: Mode,
    /// When it was born (unix seconds).
    pub created: u64,
}

/// Every refusal of the spool, each named.
#[derive(Debug)]
pub enum SpoolError {
    /// `spool_damaged`: a record before the last whole one fails its CRC, or
    /// a detail record does not follow a verdict record of its key.
    Damaged {
        /// The spool.
        path: PathBuf,
        /// The record's ordinal.
        record: u64,
        /// What is wrong with it.
        why: &'static str,
    },
    /// `spool_header_refused`: a born header that fails its CRC, names
    /// another magic or version, or does not name this spool.
    Header {
        /// The spool.
        path: PathBuf,
        /// What is wrong with it.
        why: String,
    },
    /// `merge_log_damaged`: a merge-log record before the last fails its CRC,
    /// or the log is longer than [`MAX_LOG_BYTES`].
    LogDamaged {
        /// The merge log.
        path: PathBuf,
        /// The record's ordinal (the cap's, for a log past it).
        record: u64,
        /// What is wrong with it.
        why: &'static str,
    },
    /// `record_refused`: a record this format cannot hold (a detail past
    /// [`DETAIL_PAYLOAD`], a day outside the spool's month, a run past
    /// `u32::MAX`, a bit the verdict format does not name).
    Record {
        /// What is wrong with it.
        why: String,
    },
    /// `not_regular`: something other than a regular file is at a spool's or
    /// a log's name (a link, a pipe, a directory): never followed.
    NotRegular {
        /// The name.
        path: PathBuf,
    },
    /// The host refused an action.
    Io {
        /// The file.
        path: PathBuf,
        /// What was being done.
        action: &'static str,
        /// The host's words.
        source: io::Error,
    },
    /// `run_live_busy`: the run's live lock stayed held past [`LIVE_WAIT`]
    /// (a merger merging this run's spools of an earlier process, or a
    /// second writer of the run): nothing was appended.
    Busy {
        /// The run.
        run: u64,
        /// Who holds the lock, in words.
        holder: String,
    },
    /// `spool_full`: the append would take the spool past
    /// [`MAX_SPOOL_BYTES`], the length the merge reads: nothing was written,
    /// so the spool stays mergeable whole.
    Full {
        /// The spool.
        path: PathBuf,
        /// Its length now.
        len: u64,
        /// The bytes the append would add.
        adding: u64,
    },
}

impl SpoolError {
    /// The refusal's code, never renamed or reused.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Damaged { .. } => "spool_damaged",
            Self::Header { .. } => "spool_header_refused",
            Self::LogDamaged { .. } => "merge_log_damaged",
            Self::Record { .. } => "record_refused",
            Self::NotRegular { .. } => "not_regular",
            Self::Io { .. } => "io",
            Self::Busy { .. } => "run_live_busy",
            Self::Full { .. } => "spool_full",
        }
    }

    fn io(path: &Path, action: &'static str, source: io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            action,
            source,
        }
    }

    /// A refused open in the host's own words (IA-9 repair 2): only a name
    /// that is not a regular file is [`Self::NotRegular`]; any other refusal
    /// (`EMFILE` when a pass holds every descriptor, `EACCES`, a name gone)
    /// keeps the host's error, so a full descriptor table is never reported
    /// as a link or a pipe at a spool's name.
    fn refused(at: &Path, why: store::safe_open::ReadRefusal) -> Self {
        match why {
            store::safe_open::ReadRefusal::NotRegular { path, .. } => Self::NotRegular { path },
            store::safe_open::ReadRefusal::Io {
                path,
                action,
                source,
            } => Self::Io {
                path,
                action,
                source,
            },
            // A refusal of a kind added later keeps its words too.
            other => Self::io(at, "open", io::Error::other(other.to_string())),
        }
    }
}

impl std::fmt::Display for SpoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Damaged { path, record, why } => write!(
                f,
                "spool_damaged: {} record {record} {why}; the spool is refused by name and nothing of it is merged",
                path.display()
            ),
            Self::Header { path, why } => {
                write!(f, "spool_header_refused: {}: {why}", path.display())
            }
            Self::LogDamaged { path, record, why } => write!(
                f,
                "merge_log_damaged: {} record {record} {why}; the spool is refused by name",
                path.display()
            ),
            Self::Record { why } => write!(f, "record_refused: {why}"),
            Self::NotRegular { path } => write!(
                f,
                "not_regular: {} is not a regular file (never followed)",
                path.display()
            ),
            Self::Io {
                path,
                action,
                source,
            } => write!(f, "io: {action} {}: {source}", path.display()),
            Self::Busy { run, holder } => write!(
                f,
                "run_live_busy: run {run}'s live lock {LIVE_DIR}/{run}.lock stayed held for {} s ({holder}); nothing was appended",
                LIVE_WAIT.as_secs()
            ),
            Self::Full { path, len, adding } => write!(
                f,
                "spool_full: {} holds {len} bytes and {adding} more would pass the {MAX_SPOOL_BYTES} bytes a merge reads; nothing was appended, the spool stays mergeable whole",
                path.display()
            ),
        }
    }
}

impl std::error::Error for SpoolError {}

// ---------------------------------------------------------------- the header --

/// The position of `timeframe` in [`Timeframe::KNOWN`], plus one: the header's
/// timeframe code. Its seconds are written beside it and must agree.
fn timeframe_code(timeframe: Timeframe) -> u8 {
    Timeframe::KNOWN
        .iter()
        .zip(1_u8..)
        .find(|(known, _)| **known == timeframe)
        .map_or(0, |(_, code)| code)
}

/// The 64 header bytes of `id`.
#[must_use]
pub fn encode_header(id: &SpoolId, mode: Mode, created: u64) -> [u8; 64] {
    let mut out = [0_u8; 64];
    put(&mut out, 0, &MAGIC);
    put(&mut out, 8, &VERSION.to_le_bytes());
    put(
        &mut out,
        10,
        &u16::try_from(RECORD_LEN).unwrap_or(0).to_le_bytes(),
    );
    put(
        &mut out,
        12,
        &[
            verdict::feed_code(Vendor::Zerodha).unwrap_or(0),
            verdict::segment_code(id.segment),
            timeframe_code(id.timeframe),
            mode.code(),
        ],
    );
    put(&mut out, 16, &id.month.year().to_le_bytes());
    put(&mut out, 18, &[id.month.month()]);
    put(&mut out, 20, &id.run.to_le_bytes());
    put(&mut out, 28, &created.to_le_bytes());
    put(&mut out, 36, &id.timeframe.secs().to_le_bytes());
    let crc = crc32c(out.get(..60).unwrap_or_default());
    put(&mut out, 60, &crc.to_le_bytes());
    out
}

/// A header read back, or why not; `Ok(None)` for an unborn header (all
/// zero).
///
/// # Errors
///
/// [`SpoolError::Header`] for a CRC, magic, version, stride, feed or field
/// that does not read.
pub fn decode_header(bytes: &[u8], path: &Path) -> Result<Option<Header>, SpoolError> {
    let refuse = |why: &str| SpoolError::Header {
        path: path.to_path_buf(),
        why: why.to_owned(),
    };
    if bytes.len() < 64 || bytes.iter().take(64).all(|b| *b == 0) {
        return Ok(None);
    }
    if crc32c(bytes.get(..60).unwrap_or_default()) != u32_at(bytes, 60) {
        return Err(refuse("its CRC-32C does not match"));
    }
    if bytes.get(..8) != Some(MAGIC.as_slice()) {
        return Err(refuse("it does not begin BRUTEXVS"));
    }
    if u16_at(bytes, 8) != VERSION {
        return Err(refuse("its version is not 1, the version this build reads"));
    }
    if u64::from(u16_at(bytes, 10)) != RECORD_LEN {
        return Err(refuse("its stride is not 160"));
    }
    if verdict::feed_of(u8_at(bytes, 12)) != Some(Vendor::Zerodha) {
        return Err(refuse("it names a feed other than zerodha"));
    }
    let segment =
        verdict::segment_of(u8_at(bytes, 13)).ok_or_else(|| refuse("it names no segment"))?;
    let timeframe = Timeframe::from_secs(u32_at(bytes, 36))
        .ok()
        .filter(|tf| timeframe_code(*tf) == u8_at(bytes, 14))
        .ok_or_else(|| refuse("its timeframe code and seconds disagree"))?;
    let mode = Mode::from_code(u8_at(bytes, 15))
        .filter(|m| matches!(m, Mode::ZerodhaMerge | Mode::ZerodhaCold))
        .ok_or_else(|| refuse("its mode is not a Zerodha pass"))?;
    let month = YearMonth::new(u16_at(bytes, 16), u8_at(bytes, 18))
        .map_err(|_| refuse("it names no month"))?;
    Ok(Some(Header {
        id: SpoolId {
            run: u64_at(bytes, 20),
            segment,
            timeframe,
            month,
        },
        mode,
        created: u64_at(bytes, 28),
    }))
}

// --------------------------------------------------------------- the records --

/// The 160 bytes of a verdict record, and one per detail.
///
/// # Errors
///
/// [`SpoolError::Record`] for a detail past [`DETAIL_PAYLOAD`] bytes, a run
/// past `u32::MAX` or a flag bit the verdict format does not name.
pub fn encode_record(record: &SpoolRecord, run: u64) -> Result<Vec<u8>, SpoolError> {
    let refuse = |why: String| SpoolError::Record { why };
    let run = u32::try_from(run).map_err(|_| refuse(format!("run {run} is past u32::MAX")))?;
    if verdict::flags::unnamed(record.facts.flags) != 0 {
        return Err(refuse(format!(
            "flags {:#x} carry a bit the verdict format does not name",
            record.facts.flags
        )));
    }
    if record.details.len() > MAX_DETAILS {
        return Err(refuse(format!(
            "{} details: a spooled record carries at most {MAX_DETAILS}",
            record.details.len()
        )));
    }
    let day = record.day.days_from_epoch();
    let exchange = verdict::exchange_code(record.exchange);
    let mut out = Vec::with_capacity(verdict::RECORD_STRIDE * (1 + record.details.len()));
    let mut bytes = [0_u8; 160];
    let facts = &record.facts;
    put(&mut bytes, 0, &record.key.hash().to_le_bytes());
    put(&mut bytes, 8, record.key.text().as_bytes());
    put(&mut bytes, 72, &[facts.code.code(), facts.rule_set]);
    put(&mut bytes, 74, &facts.reason.to_le_bytes());
    put(&mut bytes, 76, &run.to_le_bytes());
    put(&mut bytes, 80, &facts.flags.to_le_bytes());
    for (at, value) in [
        (88, facts.source_units),
        (92, facts.store_units),
        (96, facts.missing),
        (100, facts.extra),
        (104, facts.value_diff),
        (108, facts.volume_diff),
        (112, facts.oi_diff),
        (116, facts.first_diff),
        (120, record.receipt),
        (124, facts.month_generation),
        (128, facts.first_row),
        (132, facts.rows),
        (136, facts.first_unit),
        (140, facts.last_unit),
    ] {
        put(&mut bytes, at, &value.to_le_bytes());
    }
    // THE DETAILS THAT FOLLOW, COUNTED IN THE RECORD (IA-9 repair 1).
    put(
        &mut bytes,
        150,
        &u16::try_from(record.details.len())
            .unwrap_or(u16::MAX)
            .to_le_bytes(),
    );
    seal(&mut bytes, day, exchange, KIND_VERDICT);
    out.extend_from_slice(&bytes);
    for detail in &record.details {
        if detail.kind == 0 || detail.payload.len() > DETAIL_PAYLOAD {
            return Err(refuse(format!(
                "detail kind {} of {} bytes: a spool detail is a kind above 0 and at most {DETAIL_PAYLOAD} bytes",
                detail.kind,
                detail.payload.len()
            )));
        }
        let mut bytes = [0_u8; 160];
        put(&mut bytes, 0, &record.key.hash().to_le_bytes());
        put(&mut bytes, 8, &detail.kind.to_le_bytes());
        put(
            &mut bytes,
            10,
            &u16::try_from(detail.payload.len())
                .unwrap_or(0)
                .to_le_bytes(),
        );
        put(&mut bytes, 12, &detail.payload);
        seal(&mut bytes, day, exchange, KIND_DETAIL);
        out.extend_from_slice(&bytes);
    }
    Ok(out)
}

/// Writes the day, the exchange and the kind, then the CRC (a verdict
/// record's detail count at 150 is written before it).
fn seal(bytes: &mut [u8; 160], day: u32, exchange: u8, kind: u8) {
    put(bytes, 144, &day.to_le_bytes());
    put(bytes, 148, &[exchange, kind]);
    let crc = crc32c(bytes.get(..156).unwrap_or_default());
    put(bytes, 156, &crc.to_le_bytes());
}

/// One decoded record: a verdict record with its run and the details it
/// declares, or a detail.
enum Decoded {
    Verdict(SpoolRecord, u32, usize),
    Detail(u64, Detail),
}

/// Decodes one 160-byte record, or names why it is not one.
fn decode_record(bytes: &[u8]) -> Result<Decoded, &'static str> {
    if crc32c(bytes.get(..156).unwrap_or_default()) != u32_at(bytes, 156) {
        return Err("fails its CRC-32C");
    }
    // 150..152 is a verdict record's detail count, zero in a detail.
    let declared = usize::from(u16_at(bytes, 150));
    if bytes
        .get(152..156)
        .is_none_or(|r| r.iter().any(|b| *b != 0))
        || declared > MAX_DETAILS
        || (declared > 0 && u8_at(bytes, 149) != KIND_VERDICT)
    {
        return Err("has reserved bytes that are not zero");
    }
    let day = Day::from_days(u32_at(bytes, 144)).map_err(|_| "names no day")?;
    let exchange = verdict::exchange_of(u8_at(bytes, 148)).ok_or("names no exchange")?;
    let hash = u64_at(bytes, 0);
    match u8_at(bytes, 149) {
        KIND_VERDICT => {
            let field = bytes.get(8..72).unwrap_or_default();
            let len = field.iter().take_while(|b| **b != 0).count();
            let text = std::str::from_utf8(field.get(..len).unwrap_or_default())
                .map_err(|_| "holds a key that is not UTF-8")?;
            let key = Key::instrument(text).map_err(|_| "holds no instrument key")?;
            if key.hash() != hash {
                return Err("holds a key whose hash is not its text's");
            }
            let code = Code::from_code(u8_at(bytes, 72)).ok_or("names no verdict code")?;
            let facts = Facts {
                code,
                rule_set: u8_at(bytes, 73),
                reason: u16_at(bytes, 74),
                flags: u64_at(bytes, 80),
                source_units: u32_at(bytes, 88),
                store_units: u32_at(bytes, 92),
                missing: u32_at(bytes, 96),
                extra: u32_at(bytes, 100),
                value_diff: u32_at(bytes, 104),
                volume_diff: u32_at(bytes, 108),
                oi_diff: u32_at(bytes, 112),
                first_diff: u32_at(bytes, 116),
                month_generation: u32_at(bytes, 124),
                first_row: u32_at(bytes, 128),
                rows: u32_at(bytes, 132),
                first_unit: u32_at(bytes, 136),
                last_unit: u32_at(bytes, 140),
            };
            Ok(Decoded::Verdict(
                SpoolRecord {
                    exchange,
                    day,
                    key,
                    facts,
                    receipt: u32_at(bytes, 120),
                    details: Vec::new(),
                },
                u32_at(bytes, 76),
                declared,
            ))
        }
        KIND_DETAIL => {
            let kind = u16_at(bytes, 8);
            let len = usize::from(u16_at(bytes, 10));
            if kind == 0 || len > DETAIL_PAYLOAD {
                return Err("holds a detail no writer writes");
            }
            let payload = bytes.get(12..12 + len).unwrap_or_default().to_vec();
            Ok(Decoded::Detail(hash, Detail { kind, payload }))
        }
        _ => Err("names no record kind"),
    }
}

// ---------------------------------------------------------------- the writer --

/// One spool file a run has open: its handle, the offset after its last
/// synced record, and the most bytes the file can hold (its length when it
/// was opened, then the end of the last write tried), so a later append
/// knows whether bytes past `end` must be cut first.
#[derive(Debug)]
struct Open {
    file: File,
    end: u64,
    len: u64,
}

/// What one append wrote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Appended {
    /// Verdict records written.
    pub records: u64,
    /// 160-byte records written, details included.
    pub slots: u64,
    /// Spool files written to.
    pub files: u64,
    /// Spools born (a header written) by this append.
    pub born: u64,
}

/// What a writer holds behind its one mutex: its open files and, from its
/// first append on, the run's live lock.
#[derive(Debug)]
struct Held {
    files: HashMap<SpoolId, Open>,
    live: Option<store::flock::Flock<File>>,
}

/// A run's one spool writer (critic IA-9-2): every append of the run goes
/// through it, so one file has one appender and one offset.
///
/// # The run is live while its writer lives (IA-9 repair 1)
///
/// From its first append until it is dropped, the writer holds the run's
/// live lock (`verify/live/<run>.lock`), and no merger reads, commits or
/// removes a spool of a run whose lock is held: the run is named live and
/// its spools wait. So a caller DROPS the writer when its run ends, before
/// it merges in the same process or starts a `verify-merge` child (D-4921),
/// or the merge leaves that run's spools waiting and names it.
#[derive(Debug)]
pub struct SpoolWriter {
    root: PathBuf,
    run: u64,
    mode: Mode,
    created: u64,
    open: Mutex<Held>,
}

impl SpoolWriter {
    /// The writer of run `run` under `root`, writing `mode`'s spools, born at
    /// `created` (unix seconds).
    #[must_use]
    pub fn new(root: &Path, run: u64, mode: Mode, created: u64) -> Self {
        Self {
            root: root.to_path_buf(),
            run,
            mode,
            created,
            open: Mutex::new(Held {
                files: HashMap::with_capacity(16),
                live: None,
            }),
        }
    }

    /// Takes the run's live lock: at once when it is free, else tried every
    /// [`LIVE_STEP`] for at most [`LIVE_WAIT`] (a merger is merging this
    /// run's spools of an earlier process), then refused by name.
    fn take_live(&self) -> Result<store::flock::Flock<File>, SpoolError> {
        let path = live_lock_path(&self.root, self.run);
        let started = std::time::Instant::now();
        loop {
            let file = open_live(&self.root, &path)?;
            match store::flock::Flock::try_lock_noted(file, path.clone()) {
                Ok(lock) => return Ok(lock),
                Err(std::fs::TryLockError::WouldBlock) if started.elapsed() < LIVE_WAIT => {
                    std::thread::sleep(LIVE_STEP);
                }
                Err(std::fs::TryLockError::WouldBlock) => {
                    return Err(SpoolError::Busy {
                        run: self.run,
                        holder: store::flock::holder_note(&path),
                    });
                }
                Err(std::fs::TryLockError::Error(why)) => {
                    return Err(SpoolError::io(&path, "lock", why));
                }
            }
        }
    }

    /// The run.
    #[must_use]
    pub const fn run(&self) -> u64 {
        self.run
    }

    /// Appends `records` of `segment` at `timeframe`: one write and one
    /// sync per spool file they fall in. Nothing is written for a record the
    /// format cannot hold; the whole call is refused first.
    ///
    /// # Errors
    ///
    /// [`SpoolError::Record`] for a record no spool can hold (checked before
    /// any byte is written), and every refusal opening, writing or syncing a
    /// file. A failed write leaves the file's offset unchanged, so the next
    /// append writes over the torn bytes.
    pub fn append(
        &self,
        segment: Segment,
        timeframe: Timeframe,
        records: &[SpoolRecord],
    ) -> Result<Appended, SpoolError> {
        let mut grouped: HashMap<SpoolId, (Vec<u8>, u64, u64)> = HashMap::with_capacity(4);
        for record in records {
            let month = record.day.year_month().map_err(|why| SpoolError::Record {
                why: format!("day {}: {why}", record.day),
            })?;
            let id = SpoolId {
                run: self.run,
                segment,
                timeframe,
                month,
            };
            let bytes = encode_record(record, self.run)?;
            let slots = bytes.len() as u64 / RECORD_LEN;
            let entry = grouped.entry(id).or_insert_with(|| (Vec::new(), 0, 0));
            entry.0.extend_from_slice(&bytes);
            entry.1 += 1;
            entry.2 += slots;
        }
        let mut held = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        // THE RUN IS LIVE FROM ITS FIRST APPEND ON: no merger touches its
        // spools until this writer is dropped (IA-9 repair 1).
        if held.live.is_none() {
            held.live = Some(self.take_live()?);
        }
        let open = &mut held.files;
        let mut done = Appended::default();
        for (id, (bytes, records, slots)) in grouped {
            let at = if let Some(found) = open.get(&id) {
                found.end
            } else {
                let (file, end, len, born) = self.open_spool(&id)?;
                done.born += u64::from(born);
                open.insert(id, Open { file, end, len });
                end
            };
            let path = id.path(&self.root);
            // A SPOOL PAST THE MERGE'S READ CAP COULD NEVER BE MERGED: the
            // append is refused by name before a byte is written.
            let adding = bytes.len() as u64;
            if at.saturating_add(adding) > MAX_SPOOL_BYTES {
                return Err(SpoolError::Full {
                    path,
                    len: at,
                    adding,
                });
            }
            let Some(entry) = open.get_mut(&id) else {
                continue;
            };
            // NO BYTE OF WHAT IS WRITTEN OVER OUTLIVES THE APPEND (IA-9
            // repair 2): past `at` can lie an ignored last group a resumed
            // run found (a verdict record and up to 32 details), or the torn
            // bytes of a write of this writer that failed; an append shorter
            // than them would leave their rest after it, a detail with no
            // record of its key before it, which refuses the whole spool as
            // damaged and loses every intact record in it. So the file is
            // cut to `at` and the cut synced before the first new byte, and
            // only once the append is taken: one refused `spool_full` above
            // cuts nothing. Only this run's writer touches its spool, under
            // its live lock, so no merger reads it while it is cut.
            if entry.len > at {
                entry
                    .file
                    .set_len(at)
                    .map_err(|why| SpoolError::io(&path, "cut", why))?;
                entry
                    .file
                    .sync_all()
                    .map_err(|why| SpoolError::io(&path, "sync", why))?;
            }
            // The most the file can hold after this write, whole or torn.
            entry.len = at + adding;
            entry
                .file
                .write_all_at(&bytes, at)
                .map_err(|why| SpoolError::io(&path, "write", why))?;
            entry
                .file
                .sync_data()
                .map_err(|why| SpoolError::io(&path, "sync", why))?;
            entry.end = at + bytes.len() as u64;
            done.records += records;
            done.slots += slots;
            done.files += 1;
        }
        Ok(done)
    }

    /// Opens (or makes) one spool: a new file is born with its header, synced,
    /// its directory synced; an unborn one gets its header again; one this
    /// run made before resumes after its whole records. Also returns the
    /// file's length, so the first append cuts what lies past the resume
    /// point.
    fn open_spool(&self, id: &SpoolId) -> Result<(File, u64, u64, bool), SpoolError> {
        let path = id.path(&self.root);
        let dir = path.parent().unwrap_or(&self.root).to_path_buf();
        fs::create_dir_all(&dir).map_err(|why| SpoolError::io(&dir, "create", why))?;
        let (file, len) = match fs::symlink_metadata(&path) {
            Err(why) if why.kind() == ErrorKind::NotFound => {
                let file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(|why| SpoolError::io(&path, "create", why))?;
                (file, 0)
            }
            Err(why) => return Err(SpoolError::io(&path, "stat", why)),
            Ok(meta) if !meta.file_type().is_file() => {
                return Err(SpoolError::NotRegular { path });
            }
            Ok(_) => store::safe_open::open_read_write(&path)
                .map_err(|why| SpoolError::refused(&path, why))?,
        };
        let mut head = [0_u8; 64];
        let read = if len >= HEADER_LEN {
            file.read_exact_at(&mut head, 0)
                .map_err(|why| SpoolError::io(&path, "read", why))?;
            decode_header(&head, &path)?
        } else {
            None
        };
        match read {
            Some(header) if header.id == *id => {
                let whole = (len - HEADER_LEN) / RECORD_LEN;
                // A RUN RESUMED IN A NEW PROCESS (a restart mid-run) finds
                // its spool's last append as the crash left it. A last record
                // that never finished its sync, and a last verdict record
                // lacking a detail it declares, are written over, exactly as
                // the merge ignores them; appending after them would turn
                // them into an EARLIER bad record, which refuses the whole
                // spool as damaged (D-4881). One read, O(1).
                let end = resume_at(&file, HEADER_LEN + whole * RECORD_LEN, &path)?;
                // Bytes past `end` are cut by the first append taken (IA-9
                // repair 2), never here: an append refused `spool_full`
                // writes nothing at all.
                Ok((file, end, len, false))
            }
            Some(header) => Err(SpoolError::Header {
                path,
                why: format!(
                    "it is run {}'s spool of {} {} {}, not this run's: only the run whose id names a spool appends to it",
                    header.id.run,
                    header.id.segment.as_str(),
                    header.id.timeframe.as_str(),
                    header.id.month
                ),
            }),
            None => {
                file.write_all_at(&encode_header(id, self.mode, self.created), 0)
                    .map_err(|why| SpoolError::io(&path, "write", why))?;
                file.sync_all()
                    .map_err(|why| SpoolError::io(&path, "sync", why))?;
                // THE NAME AND EVERY FOLDER A BIRTH MADE SURVIVE A CRASH: the
                // spool's folder, its segment's, its run's and `verify/spool`.
                for folder in dir.ancestors().take(4) {
                    sync_dir(folder)?;
                }
                // An unborn file can hold bytes past its header: the first
                // append cuts them.
                Ok((file, HEADER_LEN, len.max(HEADER_LEN), true))
            }
        }
    }
}

/// Syncs a directory, so a name it gained survives a crash.
fn sync_dir(dir: &Path) -> Result<(), SpoolError> {
    File::open(dir)
        .and_then(|d| d.sync_all())
        .map_err(|why| SpoolError::io(dir, "sync", why))
}

// ---------------------------------------------------------------- the reader --

/// One spool as the merge reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpoolRead {
    /// Its header; `None` for an unborn spool, which holds nothing.
    pub header: Option<Header>,
    /// Every verdict record, with its details, in append order.
    pub records: Vec<SpoolRecord>,
    /// The run each record names (all the header's, in a spool a writer
    /// wrote).
    pub runs: Vec<u32>,
    /// A torn tail, ignored: its offset and bytes.
    pub torn: Option<(u64, u64)>,
    /// A last append that never finished its sync, ignored: its offset (a
    /// last whole record that fails its CRC, or a last verdict record with
    /// fewer details after it than it declares, from that record on).
    pub unsynced: Option<u64>,
    /// The bytes [`Self::unsynced`] ignores (0 with none).
    pub unsynced_bytes: u64,
}

/// Reads a whole spool under the length-based tail rule (D-4881).
///
/// # Errors
///
/// [`SpoolError::Damaged`] for an earlier record that fails its CRC or a
/// detail with no record before it; [`SpoolError::Header`] for a header that
/// does not read or does not match `expect`; [`SpoolError::NotRegular`] and
/// the host's.
pub fn read_spool(path: &Path, expect: &SpoolId) -> Result<SpoolRead, SpoolError> {
    let (file, len) =
        store::safe_open::open_read_len(path).map_err(|why| SpoolError::refused(path, why))?;
    let mut empty = SpoolRead {
        header: None,
        records: Vec::new(),
        runs: Vec::new(),
        torn: None,
        unsynced: None,
        unsynced_bytes: 0,
    };
    if len < HEADER_LEN {
        return Ok(empty);
    }
    if len > MAX_SPOOL_BYTES {
        return Err(SpoolError::Header {
            path: path.to_path_buf(),
            why: format!(
                "its length {len} is past the {MAX_SPOOL_BYTES}-byte cap a spool is read under"
            ),
        });
    }
    let size = usize::try_from(len).map_err(|_| SpoolError::Header {
        path: path.to_path_buf(),
        why: format!("its length {len} does not fit this host's memory"),
    })?;
    let mut bytes = vec![0_u8; size];
    file.read_exact_at(&mut bytes, 0)
        .map_err(|why| SpoolError::io(path, "read", why))?;
    let Some(header) = decode_header(&bytes, path)? else {
        // AN UNBORN HEADER WITH BYTES AFTER IT: the next append starts the
        // spool again at 64, over them; they are named, never read as records.
        if len > HEADER_LEN {
            empty.torn = Some((HEADER_LEN, len - HEADER_LEN));
        }
        return Ok(empty);
    };
    if header.id != *expect {
        return Err(SpoolError::Header {
            path: path.to_path_buf(),
            why: format!(
                "it names run {} {} {} {}, not the spool at its own path",
                header.id.run,
                header.id.segment.as_str(),
                header.id.timeframe.as_str(),
                header.id.month
            ),
        });
    }
    empty.header = Some(header);
    let whole = (len - HEADER_LEN) / RECORD_LEN;
    let end = HEADER_LEN + whole * RECORD_LEN;
    if end < len {
        empty.torn = Some((end, len - end));
    }
    let mut out = empty;
    let body = bytes
        .get(64..usize::try_from(end).unwrap_or(0))
        .unwrap_or_default();
    take_records(body, (whole, end), header.id.month, path, &mut out)?;
    Ok(out)
}

/// The whole records of a spool's `body` (its `whole` records, ending at
/// byte `end`) into `out`, each detail on the verdict record it follows,
/// under the tail rule (D-4881) and the detail count (IA-9 repair 1).
fn take_records(
    body: &[u8],
    (whole, end): (u64, u64),
    month: YearMonth,
    path: &Path,
    out: &mut SpoolRead,
) -> Result<(), SpoolError> {
    // THE DETAILS THE LAST VERDICT RECORD STILL OWES, and where it began
    // (IA-9 repair 1): a record is whole only with every detail it declares.
    let mut owed = 0_usize;
    let mut group = 0_u64;
    for (index, chunk) in body.chunks_exact(verdict::RECORD_STRIDE).enumerate() {
        let ordinal = index as u64;
        let damaged = |why: &'static str| SpoolError::Damaged {
            path: path.to_path_buf(),
            record: ordinal,
            why,
        };
        match decode_record(chunk) {
            Ok(Decoded::Verdict(record, run, declared)) => {
                if owed > 0 {
                    return Err(damaged("follows a record that lacks a detail it declares"));
                }
                if record.day.year_month().ok() != Some(month) {
                    return Err(damaged("names a day outside its spool's month"));
                }
                out.records.push(record);
                out.runs.push(run);
                owed = declared;
                group = ordinal;
            }
            Ok(Decoded::Detail(hash, detail)) => match out.records.last_mut() {
                Some(owner) if owner.key.hash() == hash && owed > 0 => {
                    owner.details.push(detail);
                    owed -= 1;
                }
                Some(owner) if owner.key.hash() == hash => {
                    return Err(damaged("is a detail its record does not declare"));
                }
                _ => return Err(damaged("is a detail with no record of its key before it")),
            },
            // THE LAST WHOLE RECORD MAY BE AN APPEND THAT NEVER FINISHED ITS
            // SYNC: ignored and named. Anything earlier was synced before a
            // later record was written, so a bad one is damage.
            Err(_) if ordinal + 1 == whole => {
                out.unsynced = Some(HEADER_LEN + ordinal * RECORD_LEN);
                out.unsynced_bytes = RECORD_LEN;
            }
            Err(why) => return Err(damaged(why)),
        }
    }
    // A LAST RECORD WITHOUT EVERY DETAIL IT DECLARES is an append that never
    // finished: ignored from that record on and named, never merged without
    // its details (the second check's follow-up: before the count, a torn
    // detail was merged away in silence).
    if owed > 0 {
        out.records.pop();
        out.runs.pop();
        out.unsynced = Some(HEADER_LEN + group * RECORD_LEN);
        out.unsynced_bytes = end - (HEADER_LEN + group * RECORD_LEN);
    }
    Ok(())
}

/// Where a resumed writer appends: after the last whole record of a spool
/// whose whole records end at `end`, so neither a last record that never
/// finished its sync nor a last verdict record lacking a detail it declares
/// becomes an earlier record (which would refuse the spool as damaged).
/// One read of at most [`MAX_DETAILS`] + 2 records, O(1).
fn resume_at(file: &File, end: u64, path: &Path) -> Result<u64, SpoolError> {
    let whole = (end - HEADER_LEN) / RECORD_LEN;
    let window = whole.min(MAX_DETAILS as u64 + 2);
    if window == 0 {
        return Ok(end);
    }
    let from = end - window * RECORD_LEN;
    let mut tail = vec![0_u8; usize::try_from(window * RECORD_LEN).unwrap_or(0)];
    file.read_exact_at(&mut tail, from)
        .map_err(|why| SpoolError::io(path, "read", why))?;
    let mut details = 0_usize;
    let mut after = end;
    for (back, chunk) in tail.chunks_exact(verdict::RECORD_STRIDE).rev().enumerate() {
        let at = end - (back as u64 + 1) * RECORD_LEN;
        match decode_record(chunk) {
            // A LAST RECORD THAT NEVER FINISHED ITS SYNC is written over.
            Err(_) if back == 0 => after = at,
            Ok(Decoded::Detail(..)) => details += 1,
            // A LAST VERDICT RECORD LACKING A DETAIL it declares is written
            // over from its start.
            Ok(Decoded::Verdict(_, _, declared)) if details < declared => return Ok(at),
            // Whole, or damage the merge refuses by name: after it.
            _ => return Ok(after),
        }
    }
    Ok(after)
}

// ------------------------------------------------------------- the merge log --

/// One merge-log record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogRecord {
    /// The day (days since 1970-01-01; 0 for a refusal).
    pub day: u32,
    /// For an intent, the generation the year index named before the merge;
    /// for a commit, the generation published.
    pub generation: u32,
    /// What happened.
    pub state: LogState,
    /// The exchange's code (0 for a refusal).
    pub exchange: u8,
}

impl LogRecord {
    /// The 16 bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; 16] {
        let mut out = [0_u8; 16];
        put(&mut out, 0, &self.day.to_le_bytes());
        put(&mut out, 4, &self.generation.to_le_bytes());
        put(&mut out, 8, &[self.state.code(), self.exchange]);
        let crc = crc32c(out.get(..12).unwrap_or_default());
        put(&mut out, 12, &crc.to_le_bytes());
        out
    }

    /// The record 16 bytes hold, or `None` when its CRC or state is wrong.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 16 || crc32c(bytes.get(..12)?) != u32_at(bytes, 12) {
            return None;
        }
        if bytes.get(10..12)?.iter().any(|b| *b != 0) {
            return None;
        }
        Some(Self {
            day: u32_at(bytes, 0),
            generation: u32_at(bytes, 4),
            state: LogState::from_code(u8_at(bytes, 8))?,
            exchange: u8_at(bytes, 9),
        })
    }
}

/// A merge log as read: its good records, and where the next one goes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogRead {
    /// Every trusted record, in order.
    pub records: Vec<LogRecord>,
    /// Where the next record is written: after the last trusted one, over a
    /// torn tail or a last record that never finished its sync.
    pub next: u64,
    /// The last record of each (exchange, day), refusals and `seen` records
    /// apart, with the verdict records the spool held when it was written (the
    /// last `seen` before it), kept as each record is learned: a day's
    /// merge-log step is one hash probe whatever the log holds (C-ZV-03),
    /// never a walk back through it.
    last: HashMap<(u8, u32), (LogRecord, u32)>,
    /// The verdict records the last `seen` record says a merge read.
    covered: u32,
    /// Whether a refusal is on record, kept the same way.
    refused: bool,
}

impl LogRead {
    /// Whether a refusal is on record. O(1).
    #[must_use]
    pub const fn refused(&self) -> bool {
        self.refused
    }

    /// The last record of `(exchange, day)`, if any. One hash probe.
    #[must_use]
    pub fn last_of(&self, exchange: u8, day: u32) -> Option<LogRecord> {
        self.last.get(&(exchange, day)).map(|(record, _)| *record)
    }

    /// The verdict records of the spool that the last record of `(exchange,
    /// day)` covers: records at a place past it arrived after that merge
    /// read the spool. One hash probe.
    #[must_use]
    pub fn covered_of(&self, exchange: u8, day: u32) -> Option<u32> {
        self.last.get(&(exchange, day)).map(|(_, covered)| *covered)
    }

    /// The verdict records the last `seen` record says a merge read (0 for a
    /// log with none). O(1).
    #[must_use]
    pub const fn covered(&self) -> u32 {
        self.covered
    }

    /// Takes in one trusted record, in log order.
    fn learn(&mut self, record: LogRecord) {
        match record.state {
            LogState::Refused => self.refused = true,
            LogState::Seen => self.covered = record.day,
            LogState::Intent | LogState::Committed => {
                self.last
                    .insert((record.exchange, record.day), (record, self.covered));
            }
        }
        self.records.push(record);
    }
}

/// The longest merge log read: 64 MiB, 4,194,304 records. A month's whole
/// merge writes at most two records a day (an intent and a commit) and one
/// `seen` record per merge of the spool, so a log
/// this long is never written by this format; one that is, is refused by name
/// before a byte is allocated, never read in part.
pub const MAX_LOG_BYTES: u64 = 64 << 20;

/// Reads a merge log under its tail rule; an absent log is empty.
///
/// # Errors
///
/// [`SpoolError::LogDamaged`] for a bad record before the last or a log
/// past [`MAX_LOG_BYTES`], [`SpoolError::NotRegular`] and the host's.
pub fn read_log(path: &Path) -> Result<LogRead, SpoolError> {
    let (file, len) = match store::safe_open::open_read_len(path) {
        Ok(found) => found,
        Err(why) if why.is_missing() => return Ok(LogRead::default()),
        Err(why) => return Err(SpoolError::refused(path, why)),
    };
    // A LOG PAST THE CAP IS REFUSED, NEVER READ IN PART: reading its first
    // 64 MiB would merge against a log whose later records were never seen.
    let size = usize::try_from(len)
        .ok()
        .filter(|_| len <= MAX_LOG_BYTES)
        .ok_or_else(|| SpoolError::LogDamaged {
            path: path.to_path_buf(),
            record: MAX_LOG_BYTES / LOG_RECORD_LEN,
            why: "is past the 64 MiB a merge log is read under",
        })?;
    let mut bytes = vec![0_u8; size];
    file.read_exact_at(&mut bytes, 0)
        .map_err(|why| SpoolError::io(path, "read", why))?;
    let whole = bytes.len() / 16;
    let mut out = LogRead {
        records: Vec::with_capacity(whole),
        last: HashMap::with_capacity(whole),
        ..LogRead::default()
    };
    for (index, chunk) in bytes.chunks_exact(16).enumerate() {
        match LogRecord::decode(chunk) {
            Some(record) => out.learn(record),
            None if index + 1 == whole => {}
            None => {
                return Err(SpoolError::LogDamaged {
                    path: path.to_path_buf(),
                    record: index as u64,
                    why: "fails its CRC",
                });
            }
        }
    }
    out.next = out.records.len() as u64 * LOG_RECORD_LEN;
    Ok(out)
}

/// Appends one record at `log.next` and syncs it; `log` learns it. The first
/// record of a log also syncs its directory.
///
/// # Errors
///
/// [`SpoolError::NotRegular`] and the host's.
pub fn append_log(path: &Path, log: &mut LogRead, record: LogRecord) -> Result<(), SpoolError> {
    let file = match fs::symlink_metadata(path) {
        Err(why) if why.kind() == ErrorKind::NotFound => OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|why| SpoolError::io(path, "create", why))?,
        Err(why) => return Err(SpoolError::io(path, "stat", why)),
        Ok(meta) if !meta.file_type().is_file() => {
            return Err(SpoolError::NotRegular {
                path: path.to_path_buf(),
            });
        }
        Ok(_) => {
            store::safe_open::open_read_write(path)
                .map_err(|why| SpoolError::refused(path, why))?
                .0
        }
    };
    file.write_all_at(&record.encode(), log.next)
        .map_err(|why| SpoolError::io(path, "write", why))?;
    file.sync_data()
        .map_err(|why| SpoolError::io(path, "sync", why))?;
    if log.next == 0
        && let Some(dir) = path.parent()
    {
        sync_dir(dir)?;
    }
    log.next += LOG_RECORD_LEN;
    log.learn(record);
    Ok(())
}

// ----------------------------------------------------------------- listing --

/// Every spool waiting under `<root>/verify/spool`, and every name that is
/// not one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Waiting {
    /// The spools, in no promised order.
    pub spools: Vec<SpoolId>,
    /// Names the walk read past, by path: not a run, a segment, a timeframe,
    /// a spool or its log (a `.DS_Store`, a `._name`, an editor's file).
    pub stray: Vec<PathBuf>,
}

/// Lists every spool waiting: O(entries of the spool tree), which is the
/// spools waiting and their directories (L-IA-17).
///
/// # Errors
///
/// The host's refusal reading a directory that exists.
pub fn waiting(root: &Path) -> Result<Waiting, SpoolError> {
    let mut out = Waiting::default();
    let top = root.join(SPOOL_DIR);
    for (run_name, run_path) in entries(&top, &mut out.stray, true)? {
        let Some(run) = run_name
            .parse::<u64>()
            .ok()
            .filter(|_| all_digits(&run_name))
        else {
            out.stray.push(run_path);
            continue;
        };
        for (seg_name, seg_path) in entries(&run_path, &mut out.stray, true)? {
            let Some(segment) = segment_named(&seg_name) else {
                out.stray.push(seg_path);
                continue;
            };
            for (tf_name, tf_path) in entries(&seg_path, &mut out.stray, true)? {
                let Some(timeframe) = timeframe_named(&tf_name) else {
                    out.stray.push(tf_path);
                    continue;
                };
                // A MERGE LOG WHOSE SPOOL IS GONE answers for nothing: it is
                // named stray, never read and never removed (IA-9 repair 1;
                // a merge removes the log before the spool, so only a hand or
                // a damaged tree leaves one).
                let mut months: HashSet<YearMonth> = HashSet::with_capacity(4);
                let mut logs: Vec<(YearMonth, PathBuf)> = Vec::new();
                for (file_name, file_path) in entries(&tf_path, &mut out.stray, false)? {
                    match spool_month(&file_name) {
                        Some((month, false)) => {
                            months.insert(month);
                            out.spools.push(SpoolId {
                                run,
                                segment,
                                timeframe,
                                month,
                            });
                        }
                        Some((month, true)) => logs.push((month, file_path)),
                        None => out.stray.push(file_path),
                    }
                }
                out.stray.extend(
                    logs.into_iter()
                        .filter(|(month, _)| !months.contains(month))
                        .map(|(_, path)| path),
                );
            }
        }
    }
    Ok(out)
}

/// The names in `dir` that are directories (`dirs`) or regular files (not
/// `dirs`); anything else is stray. An absent directory has none.
fn entries(
    dir: &Path,
    stray: &mut Vec<PathBuf>,
    dirs: bool,
) -> Result<Vec<(String, PathBuf)>, SpoolError> {
    let listing = match fs::read_dir(dir) {
        Ok(listing) => listing,
        Err(why) if why.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(SpoolError::io(dir, "list", why)),
    };
    let mut out = Vec::with_capacity(8);
    for entry in listing {
        let entry = entry.map_err(|why| SpoolError::io(dir, "list", why))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|why| SpoolError::io(&path, "stat", why))?;
        let name = entry.file_name().to_str().map(str::to_owned);
        match name {
            Some(name) if (dirs && kind.is_dir()) || (!dirs && kind.is_file()) => {
                out.push((name, path));
            }
            _ => stray.push(path),
        }
    }
    Ok(out)
}

fn all_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// The segment a directory name is, exactly.
fn segment_named(name: &str) -> Option<Segment> {
    [Segment::Index, Segment::Cash, Segment::Fno]
        .into_iter()
        .find(|segment| segment.as_str() == name)
}

/// The timeframe a directory name is, exactly.
fn timeframe_named(name: &str) -> Option<Timeframe> {
    Timeframe::KNOWN
        .iter()
        .copied()
        .find(|tf| tf.as_str() == name)
}

/// `<yyyy-mm>.vsp` → (month, false); `<yyyy-mm>.vsp.merged` → (month, true);
/// any other name → `None`.
fn spool_month(name: &str) -> Option<(YearMonth, bool)> {
    let (stem, log) = match name.strip_suffix(LOG_SUFFIX) {
        Some(stem) => (stem, true),
        None => (name, false),
    };
    let month = stem.strip_suffix(".vsp")?;
    let (year, number) = month.split_once('-')?;
    if year.len() != 4 || number.len() != 2 || !all_digits(year) || !all_digits(number) {
        return None;
    }
    let month = YearMonth::new(year.parse().ok()?, number.parse().ok()?).ok()?;
    Some((month, log))
}

// --------------------------------------------------------------- bytes --

fn put(out: &mut [u8], at: usize, bytes: &[u8]) {
    if let Some(slot) = out.get_mut(at..at + bytes.len()) {
        slot.copy_from_slice(bytes);
    }
}

fn take<const N: usize>(bytes: &[u8], at: usize) -> [u8; N] {
    bytes
        .get(at..at + N)
        .and_then(|b| b.try_into().ok())
        .unwrap_or([0; N])
}

fn u8_at(bytes: &[u8], at: usize) -> u8 {
    bytes.get(at).copied().unwrap_or(0)
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(take::<2>(bytes, at))
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(take::<4>(bytes, at))
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(take::<8>(bytes, at))
}
