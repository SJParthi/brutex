//! Month recovery after a power cut: the door that brings back a month left
//! refused at its last commit whose records check out, writing the safe part
//! as a new file and saying what it dropped. R3-20, D-7093; NEW-23, RA-276,
//! RA-280, RA-330, FM-062; the store door critic D-4911 (c) proposed.
//!
//! # The state it recovers from (D-3353)
//!
//! Inside a GDFL import day's [`crate::file::DurableBatch`] no barrier orders
//! a month's records before its header slot on the drive, so a power cut (not
//! a killed process) can leave a month the batch appended to with its slot
//! on the platter and its records, or its checksums, not. Such a month is
//! refused by its own checks (`crate::file`'s module header says so) and,
//! until this door, stayed refused for ever: `repair::repair_month` reads the
//! month it repairs, and no door could read this one. The bars it held before
//! the batch were behind that refusal too.
//!
//! # What the door does
//!
//! 1. **Plans, writing nothing** ([`plan`]). It opens the ORIGINAL month and
//!    reads every committed record through the verified read path. A month
//!    that opens and reads clean is [`Plan::Clean`]. A refused one is asked
//!    for its last good commit: each header slot that still decodes, newest
//!    first, is tried as the month's header, and the first whose every
//!    record verifies against the block checksums, is in order, inside the
//!    month and on its grid, and agrees with the slot's own first and last
//!    stamp is the cut ([`Basis::Commit`]). When no slot does, the longest
//!    prefix of whole blocks whose checksums verify is the cut
//!    ([`Basis::Blocks`]). A power cut tears what the batch wrote, which
//!    starts at the tail block of the month's last durable commit, and in
//!    any order between two flushes; so a failed block below the tail block
//!    of the oldest surviving commit, with a whole block after it that
//!    verifies, is damage inside the month's history, which no cut answers
//!    ([`RecoverError::NotTheTail`]).
//! 2. **Writes the safe part as a new file** ([`recover`]): the kept records
//!    are published as a repair revision under `bar-revisions-v1/<n>/` by
//!    [`crate::repair::publish`] and made the served generation by
//!    [`crate::generation::promote`], the two doors every repair takes, so
//!    nothing of the original month is opened for writing, truncated or
//!    renamed (`CLAUDE.md` §3 rule 8). Those two doors check their source by
//!    opening it (`BarFile::open_existing`), and no door opens a refused
//!    month, so the door opens it at the cut for them: on its own thread,
//!    inside [`within`], and only for the exact bytes the plan read (the
//!    file's length and the CRC-32C of its header region,
//!    [`Cut::fits`]). Every other reader, on every other thread, keeps the
//!    month's refusal until the promotion makes the revision what they open.
//! 3. **Says what it dropped**: the most bars a surviving slot published,
//!    the bars kept, the bars dropped, and the IST days from the last kept
//!    bar's (which may have lost later bars) to the last claimed bar's
//!    ([`Dropped`]). The importer reads those days again: the cli's
//!    `month-recover` writes the read-again lines for the days the import
//!    journal shows done, and an open day is read again anyway.
//! 4. **Records it** ([`Record`]): one 256-byte record appended to
//!    `<root>/bar-recoveries-v1/bars/…/<yyyy-mm>.recover-v1`, after the
//!    promotion, naming the revision, the cut, the claim and the days. A
//!    second run reads it and writes nothing ([`Outcome::Already`]); a run
//!    cut off between the promotion and the record finds its own promotion
//!    (the served revision was published from this cut of the original) and
//!    writes the record it owes ([`Recovered::resumed`]).
//!
//! # What it never does
//!
//! It never runs by itself: the cli's `month-recover` runs it on the
//! operator's yes, from the store-step list (D-6900). It never recovers a
//! month held sealed, served from a revision it did not promote, unsealed (no
//! checksum can prove a record), with an overlay or greeks sibling (they are
//! indexed by record, as `repair::repair_month` refuses them), or that a
//! writer holds. It never keeps a record no checksum proves, so a cut can
//! drop bars a durable commit held (a tail block whose entry the batch
//! re-sealed over records that never reached the disk): those bars are
//! named among the dropped and read again, never kept on trust.
//!
//! # Cost
//!
//! A plan reads the month once per candidate commit through the verified
//! read path (at most [`crate::format::MAX_SLOTS`] slots and one block
//! prefix), O(the month's bytes), and holds the kept records in memory for
//! the publication, at most [`crate::repair::row_ceiling`] of them (the
//! repair's own bound, `docs/06-limits.md` L-IA-26). Nothing scales with the
//! months stored: one pointer read, one record-file read and the month's own
//! files. A one-time door, never a per-request read (`docs/06-limits.md`
//! L-IA-40).

use std::cell::RefCell;
use std::fmt;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, ErrorKind};
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};

use crate::crc::{crc32c, crc32c_split};
use crate::file::{Admission, BarFile, StoreError, survey};
use crate::flock::Flock;
use crate::format::Bar;
use crate::generation::{self, Promoted, Served};
use crate::header::Header;
use crate::layout::Layout;
use crate::path::{FileKind, StorePath, YearMonth};
use crate::repair::{self, Published, RepairError, Revision, RevisionReader};

/// The directory under the store root every recovery record lives below.
pub const ROOT: &str = "bar-recoveries-v1";
/// The record file's extension, beside the month's own stem.
pub const EXTENSION: &str = "recover-v1";
/// The magic at byte 0 of every record.
pub const MAGIC: [u8; 8] = *b"BRXRCV01";
/// The record format's version.
pub const VERSION: u16 = 1;
/// Bytes of one record, as a length.
const RECORD: usize = 256;
/// Bytes of one record.
pub const RECORD_LEN: u64 = RECORD as u64;
/// The most records one month's file may hold. Each is one run of the door
/// that wrote, and a month is recovered once: a second run reads the record
/// and writes nothing, so the bound is far past any honest history.
pub const MAX_RECORDS: u64 = 64;
/// The longest record file read: every record, plus one torn tail.
const MAX_LEN: u64 = (MAX_RECORDS + 1) * RECORD_LEN;
/// The header region's length, as the bar geometry fixes it.
const REGION_LEN: u64 = 32_768;
/// One IST day, in microseconds.
const DAY_MICROS: i64 = 86_400_000_000;

/// Why the recovered month's records stop where they do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// A header slot's commit whose every record checks out: the month's
    /// last good commit.
    Commit {
        /// The commit's generation.
        generation: u64,
    },
    /// No slot's commit checks out; the longest prefix of whole blocks whose
    /// checksums verify. The records are a prefix of the commit `generation`
    /// names (the oldest surviving slot that published all of them), and the
    /// cut is no commit of its own.
    Blocks {
        /// Whole blocks kept.
        blocks: u64,
        /// The oldest surviving commit that published every kept record.
        generation: u64,
    },
}

impl Basis {
    /// A stable word for a log line, a ledger field or a JSON value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Commit { .. } => "commit",
            Self::Blocks { .. } => "blocks",
        }
    }

    /// The generation the kept records were committed under.
    #[must_use]
    pub const fn generation(self) -> u64 {
        match self {
            Self::Commit { generation } | Self::Blocks { generation, .. } => generation,
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::Commit { .. } => 1,
            Self::Blocks { .. } => 2,
        }
    }

    const fn value(self) -> u64 {
        match self {
            Self::Commit { generation } => generation,
            Self::Blocks { blocks, .. } => blocks,
        }
    }
}

impl fmt::Display for Basis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Commit { generation } => write!(f, "the commit of generation {generation}"),
            Self::Blocks { blocks, generation } => write!(
                f,
                "the first {blocks} whole block(s) whose checksums verify (a prefix of generation {generation}; no slot's commit checks out)"
            ),
        }
    }
}

/// The month opened at its last good commit: the header the door serves the
/// recovery from, and the bytes it was planned against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cut {
    /// The original month's bar file.
    bars: PathBuf,
    /// Its length when planned.
    len: u64,
    /// CRC-32C of its header region (at most 32,768 bytes) when planned.
    region_crc: u32,
    /// The header the recovered month is cut from.
    header: Header,
    /// Why the records stop there.
    basis: Basis,
}

impl Cut {
    /// The header the recovered month is cut from: the source header of the
    /// revision the door publishes.
    #[must_use]
    pub const fn header(&self) -> Header {
        self.header
    }

    /// Why the records stop there.
    #[must_use]
    pub const fn basis(&self) -> Basis {
        self.basis
    }

    /// The bar file's length the cut was planned against.
    #[must_use]
    pub const fn bin_len(&self) -> u64 {
        self.len
    }

    /// The CRC-32C of the header region the cut was planned against.
    #[must_use]
    pub const fn region_crc(&self) -> u32 {
        self.region_crc
    }

    /// Whether the cut was planned against these bytes: the same length and
    /// the same header region. A month whose bytes moved since its plan is
    /// not opened at the cut (`crate::file`'s `read_header`), so it refuses
    /// exactly as it did.
    #[must_use]
    pub fn fits(&self, len: u64, region: &[u8]) -> bool {
        self.len == len && self.region_crc == crc32c(region)
    }
}

thread_local! {
    /// The cut this thread's read door applies, inside [`within`] only.
    static SCOPE: RefCell<Option<Cut>> = const { RefCell::new(None) };
}

/// The cut the read door applies to `bars` on this thread: `Some` only inside
/// [`within`] for that very file. One thread-local borrow per open.
pub(crate) fn scoped(bars: &Path) -> Option<Cut> {
    SCOPE.with(|scope| {
        scope
            .borrow()
            .as_ref()
            .filter(|cut| cut.bars == bars)
            .cloned()
    })
}

/// Puts back the scope a [`within`] replaced, even when the step unwinds.
struct Restore(Option<Cut>);

impl Drop for Restore {
    fn drop(&mut self) {
        let before = self.0.take();
        SCOPE.with(|scope| scope.replace(before));
    }
}

/// Runs `step` with `cut` applied by this thread's read door to the cut's
/// month and to nothing else. Not public: only this door opens a refused
/// month, and only for the step it names.
fn within<T>(cut: &Cut, step: impl FnOnce() -> T) -> T {
    let before = SCOPE.with(|scope| scope.replace(Some(cut.clone())));
    let _restore = Restore(before);
    step()
}

/// One commit the plan tried and why its records do not check out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tried {
    /// The commit's generation.
    pub generation: u64,
    /// The records it names.
    pub n_valid: u64,
    /// Why it is not the cut, in the store's own words.
    pub refused: String,
}

/// What a recovery drops: the bars past the cut that a surviving slot
/// published, and the IST days they fall in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dropped {
    /// Bars the recovered month holds.
    pub kept: u64,
    /// The stamp of the last of them.
    pub last_kept_ts: i64,
    /// The most bars any surviving header slot published.
    pub claimed: u64,
    /// That slot's generation.
    pub claimed_generation: u64,
    /// That slot's last stamp.
    pub claimed_last_ts: i64,
    /// `claimed - kept`.
    pub bars: u64,
    /// The IST day of the month of the last kept bar: the first day that may
    /// have lost bars (the bars after its last kept one are unproven).
    pub first_day: u8,
    /// The IST day of the month of the last claimed bar.
    pub last_day: u8,
}

impl Dropped {
    /// Every IST day of the month a re-read must cover, first to last.
    pub fn days(&self) -> impl Iterator<Item = u8> {
        self.first_day..=self.last_day
    }
}

/// What the plan found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// The month as served opens and every committed record verifies:
    /// there is nothing to recover and nothing is written.
    Clean {
        /// The generation served: `0` is the original.
        ordinal: u32,
        /// Its committed records.
        n_valid: u64,
    },
    /// This door already recovered the month: its record names the
    /// revision served. Nothing is written.
    Recovered(Record),
    /// The month is refused, and this is its last good commit.
    Cut {
        /// The month's own refusal, in the store's words.
        refused: String,
        /// Where the recovered month's records stop.
        cut: Cut,
        /// What the recovery drops.
        dropped: Dropped,
        /// The commits tried before the cut, newest first.
        tried: Vec<Tried>,
        /// `Some` when this cut's revision is already served (a run cut off
        /// between its promotion and its record): the recovery writes only
        /// the record it owes.
        promoted: Option<generation::Pointer>,
    },
}

/// One recovery, as its record file keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    /// The revision the recovered month is served from.
    pub revision: Revision,
    /// Why its records stop where they do.
    pub basis: Basis,
    /// The header it was cut from (the revision's source header).
    pub source: Header,
    /// What it dropped.
    pub dropped: Dropped,
    /// The original bar file's length when planned.
    pub bin_len: u64,
    /// The CRC-32C of the original's header region when planned.
    pub region_crc: u32,
    /// When the recovery was recorded, UTC micros.
    pub recovered_at_micros: i64,
    /// Whether the caller has named the dropped days for the importer's
    /// re-read ([`note_reread`]): `false` in the record the recovery writes,
    /// `true` in the copy appended once the caller's lines are written, so a
    /// caller cut off between the two writes its lines again on the next run.
    pub reread_named: bool,
    /// Which record of the file this is.
    pub record: u64,
}

impl Record {
    fn encode(&self) -> Result<[u8; RECORD], RecoverError> {
        let source = self
            .source
            .commit()
            .map_err(|why| RecoverError::Record {
                path: PathBuf::new(),
                reason: format!("the cut header does not encode: {why}"),
            })?
            .bytes;
        let mut bytes = [0u8; RECORD];
        let fields: [(usize, &[u8]); 17] = [
            (0, &MAGIC),
            (8, &VERSION.to_le_bytes()),
            (10, &[self.basis.code()]),
            (11, &[self.dropped.first_day]),
            (12, &[self.dropped.last_day]),
            (13, &[u8::from(self.reread_named)]),
            (16, &self.revision.ordinal().to_le_bytes()),
            (20, &self.region_crc.to_le_bytes()),
            (24, &self.bin_len.to_le_bytes()),
            (32, &self.basis.value().to_le_bytes()),
            (40, &source),
            (104, &self.dropped.last_kept_ts.to_le_bytes()),
            (112, &self.dropped.claimed.to_le_bytes()),
            (120, &self.dropped.claimed_generation.to_le_bytes()),
            (128, &self.dropped.claimed_last_ts.to_le_bytes()),
            (136, &self.dropped.bars.to_le_bytes()),
            (144, &self.recovered_at_micros.to_le_bytes()),
        ];
        for (at, field) in fields {
            if let Some(slot) = bytes.get_mut(at..at + field.len()) {
                slot.copy_from_slice(field);
            }
        }
        let crc = seal(&bytes, self.record);
        if let Some(slot) = bytes.get_mut(RECORD - 4..) {
            slot.copy_from_slice(&crc.to_le_bytes());
        }
        Ok(bytes)
    }

    fn decode(bytes: &[u8; RECORD], record: u64) -> Option<Self> {
        let word = |at: usize| -> Option<[u8; 8]> { bytes.get(at..at + 8)?.try_into().ok() };
        let quad = |at: usize| -> Option<[u8; 4]> { bytes.get(at..at + 4)?.try_into().ok() };
        if bytes.get(..8)? != MAGIC
            || u16::from_le_bytes(bytes.get(8..10)?.try_into().ok()?) != VERSION
            || u32::from_le_bytes(quad(RECORD - 4)?) != seal(bytes, record)
        {
            return None;
        }
        let source = Header::decode(bytes.get(40..104)?).ok()?;
        let value = u64::from_le_bytes(word(32)?);
        let basis = match *bytes.get(10)? {
            1 => Basis::Commit { generation: value },
            2 => Basis::Blocks {
                blocks: value,
                generation: source.generation,
            },
            _ => return None,
        };
        let kept = source.n_valid;
        let reread_named = match *bytes.get(13)? {
            0 => false,
            1 => true,
            _ => return None,
        };
        Some(Self {
            revision: Revision::new(u32::from_le_bytes(quad(16)?)).ok()?,
            basis,
            source,
            dropped: Dropped {
                kept,
                last_kept_ts: i64::from_le_bytes(word(104)?),
                claimed: u64::from_le_bytes(word(112)?),
                claimed_generation: u64::from_le_bytes(word(120)?),
                claimed_last_ts: i64::from_le_bytes(word(128)?),
                bars: u64::from_le_bytes(word(136)?),
                first_day: *bytes.get(11)?,
                last_day: *bytes.get(12)?,
            },
            bin_len: u64::from_le_bytes(word(24)?),
            region_crc: u32::from_le_bytes(quad(20)?),
            recovered_at_micros: i64::from_le_bytes(word(144)?),
            reread_named,
            record,
        })
    }

    /// Whether `other` records the same recovery, its flag and position
    /// aside.
    fn same_recovery(&self, other: &Self) -> bool {
        Self {
            reread_named: false,
            record: 0,
            ..*self
        } == Self {
            reread_named: false,
            record: 0,
            ..*other
        }
    }
}

/// CRC-32C of a record's first 252 bytes followed by its index, so a record
/// copied to another position does not verify there.
fn seal(bytes: &[u8; RECORD], record: u64) -> u32 {
    crc32c_split(
        bytes.get(..RECORD - 4).unwrap_or(&[]),
        &record.to_le_bytes(),
    )
}

/// What [`recover`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The month opens and reads clean: nothing was written.
    Clean {
        /// The generation served.
        ordinal: u32,
        /// Its committed records.
        n_valid: u64,
    },
    /// Already recovered by this door: nothing was written.
    Already(Record),
    /// The month is recovered.
    Recovered(Recovered),
}

/// A month this call recovered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovered {
    /// The month's own refusal before the recovery.
    pub refused: String,
    /// The commits tried before the cut.
    pub tried: Vec<Tried>,
    /// Whether the revision was written by this call or found complete (a
    /// run cut off between publication and promotion, resumed).
    pub published: Published,
    /// The promotion.
    pub promoted: Promoted,
    /// The record written.
    pub record: Record,
    /// Ordinals stepped over because they hold another request or an
    /// unfinished one, each with why. Never reused, never overwritten.
    pub skipped: Vec<(u32, String)>,
    /// `true` when this call found its own promotion already made (a run cut
    /// off before its record) and wrote only the record.
    pub resumed: bool,
}

/// Named refusals of the recovery door.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoverError {
    /// The store refused the month or a file of it, in its own words.
    Store(StoreError),
    /// The repair doors refused the publication or the promotion.
    Repair(RepairError),
    /// The month is not a raw original month this door recovers: held
    /// sealed, served from a revision this door did not promote, or with a
    /// sibling indexed by record.
    NotRaw {
        /// The file that says so.
        path: PathBuf,
        /// Why, in words.
        why: String,
    },
    /// The month's header carries no block checksums: no record of it can
    /// be proved, so no cut can be either.
    Unsealed {
        /// The bar file.
        path: PathBuf,
    },
    /// No commit of the month checks out and no whole block verifies: there
    /// is nothing safe to keep.
    NoCommit {
        /// The month's own refusal.
        refused: String,
        /// Every commit tried, and why.
        tried: Vec<Tried>,
    },
    /// The first block that fails lies below the tail block of the oldest
    /// surviving commit and whole blocks past it verify: damage inside the
    /// month's history, not the tail a power cut tears.
    NotTheTail {
        /// The month's own refusal.
        refused: String,
        /// The first block that fails.
        block: u64,
        /// Whole blocks past it that verify.
        verified_after: u64,
    },
    /// The month's recovery record file refused.
    Record {
        /// The record file.
        path: PathBuf,
        /// Why.
        reason: String,
    },
    /// A filesystem operation failed.
    Io {
        /// The exact file or directory.
        path: PathBuf,
        /// The operation.
        operation: &'static str,
        /// The host's refusal.
        kind: ErrorKind,
    },
}

impl RecoverError {
    /// A stable word for a log line, a ledger field or an exit message.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Store(StoreError::Locked { .. }) => "month_busy",
            Self::Store(StoreError::Missing { .. }) => "month_missing",
            Self::Store(_) => "store_refused",
            Self::Repair(_) => "repair_refused",
            Self::NotRaw { .. } => "not_raw",
            Self::Unsealed { .. } => "unsealed",
            Self::NoCommit { .. } => "nothing_verifies",
            Self::NotTheTail { .. } => "not_the_tail",
            Self::Record { .. } => "record_refused",
            Self::Io { .. } => "io",
        }
    }
}

impl fmt::Display for RecoverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = self.word();
        match self {
            Self::Store(why) => write!(f, "{word}: {why}"),
            Self::Repair(why) => write!(f, "{word}: {why}"),
            Self::NotRaw { path, why } => write!(
                f,
                "{word}: {} {why}; this door recovers a raw original month a power cut left refused, and nothing was written",
                path.display()
            ),
            Self::Unsealed { path } => write!(
                f,
                "{word}: {} carries no block checksums, so no record of it can be proved and no cut can be either; nothing was written",
                path.display()
            ),
            Self::NoCommit { refused, tried } => {
                write!(
                    f,
                    "{word}: the month is refused ({refused}) and no commit of it checks out"
                )?;
                for one in tried {
                    write!(
                        f,
                        "; generation {} ({} bars): {}",
                        one.generation, one.n_valid, one.refused
                    )?;
                }
                f.write_str(
                    "; no whole block verifies either, so there is nothing safe to keep: restore the month from the backup or re-import it, and nothing was written",
                )
            }
            Self::NotTheTail {
                refused,
                block,
                verified_after,
            } => write!(
                f,
                "{word}: the month is refused ({refused}); block {block} is the first that fails and {verified_after} whole block(s) past it verify, which is damage inside the month's history and not the torn tail a power cut leaves; no cut answers it (scrub-blocks names the block), and nothing was written"
            ),
            Self::Record { path, reason } => {
                write!(f, "{word}: {}: {reason}", path.display())
            }
            Self::Io {
                path,
                operation,
                kind,
            } => write!(f, "{word}: {operation} {}: {kind:?}", path.display()),
        }
    }
}

impl std::error::Error for RecoverError {}

impl From<StoreError> for RecoverError {
    fn from(why: StoreError) -> Self {
        Self::Store(why)
    }
}

impl From<RepairError> for RecoverError {
    fn from(why: RepairError) -> Self {
        Self::Repair(why)
    }
}

/// The record file of `path`'s month.
#[must_use]
pub fn record_path(root: &Path, path: StorePath<'_>) -> PathBuf {
    path.with_file(FileKind::Bars)
        .to_path_buf(&root.join(ROOT))
        .with_extension(EXTENSION)
}

/// The last record of `path`'s month that verifies, if any, and how many
/// records past it (or bytes of a torn tail) were stepped over.
///
/// # Errors
///
/// [`RecoverError::Record`] for a file that is not a regular file, is longer
/// than [`MAX_RECORDS`] records, cannot be read, or holds bytes of which no
/// record verifies.
pub fn read_record(root: &Path, path: StorePath<'_>) -> Result<Option<Record>, RecoverError> {
    let at = record_path(root, path);
    let file = match crate::open_flags::open_read_no_follow(&at) {
        Ok(file) => file,
        Err(why) if why.kind() == ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(record_refused(&at, format!("cannot open it: {why}"))),
    };
    last_record(&file, &at)
}

fn record_refused(path: &Path, reason: String) -> RecoverError {
    RecoverError::Record {
        path: path.to_path_buf(),
        reason,
    }
}

/// The last record of an open record file that verifies.
fn last_record(file: &File, at: &Path) -> Result<Option<Record>, RecoverError> {
    let meta = file
        .metadata()
        .map_err(|why| record_refused(at, format!("cannot measure it: {why}")))?;
    if !meta.is_file() {
        return Err(record_refused(at, "it is not a regular file".to_owned()));
    }
    let len = meta.len();
    if len > MAX_LEN {
        return Err(record_refused(
            at,
            format!("it is {len} bytes, past the {MAX_LEN} a file of {MAX_RECORDS} records can be"),
        ));
    }
    let whole = len / RECORD_LEN;
    let mut index = whole;
    while index > 0 {
        index -= 1;
        let mut bytes = [0u8; RECORD];
        file.read_exact_at(&mut bytes, index * RECORD_LEN)
            .map_err(|why| record_refused(at, format!("cannot read record {index}: {why}")))?;
        if let Some(found) = Record::decode(&bytes, index) {
            return Ok(Some(found));
        }
    }
    if len == 0 {
        return Ok(None);
    }
    Err(record_refused(
        at,
        format!(
            "none of its {whole} whole record(s) of {len} bytes verifies, so what this door did to the month cannot be told; nothing is assumed in its place"
        ),
    ))
}

/// What the plan reads of a month, with the records the cut keeps.
struct Planned {
    plan: Plan,
    rows: Vec<Bar>,
}

/// Plans `path`'s recovery, writing nothing.
///
/// # Errors
///
/// [`RecoverError::Store`] for a month that is missing, held by a writer, or
/// whose served generation cannot be told; [`RecoverError::NotRaw`],
/// [`RecoverError::Unsealed`], [`RecoverError::NoCommit`],
/// [`RecoverError::NotTheTail`] as the module says; a record file that
/// refuses.
pub fn plan(root: &Path, path: StorePath<'_>, symbol_id: u32) -> Result<Plan, RecoverError> {
    let _month = month_lock(root, path)?;
    planned(root, path, symbol_id).map(|planned| planned.plan)
}

/// The month's own lock, shared and non-blocking: a writer holding the month
/// refuses the door ([`StoreError::Locked`]), and no writer can take it while
/// the door plans and publishes. A month with no lock file never had a
/// writer, so no batch can have torn it; it is refused by name.
fn month_lock(root: &Path, path: StorePath<'_>) -> Result<Flock<File>, RecoverError> {
    if path.file() != FileKind::Bars {
        return Err(StoreError::NotABarPath { found: path.file() }.into());
    }
    let lock_path = path.with_file(FileKind::Lock).to_path_buf(root);
    let bars = path.to_path_buf(root);
    let handle = match crate::open_flags::open_read_no_follow(&lock_path) {
        Ok(handle) => handle,
        Err(why) if why.kind() == ErrorKind::NotFound && !bars.exists() => {
            return Err(StoreError::Missing {
                path: bars,
                action: crate::file::Action::Open,
            }
            .into());
        }
        Err(why) => {
            return Err(RecoverError::Io {
                path: lock_path,
                operation: "open the month's lock",
                kind: why.kind(),
            });
        }
    };
    match Flock::try_lock_shared(handle, lock_path.clone()) {
        Ok(held) => Ok(held),
        Err(TryLockError::WouldBlock) => Err(StoreError::Locked { path: lock_path }.into()),
        Err(TryLockError::Error(why)) => Err(RecoverError::Io {
            path: lock_path,
            operation: "lock the month shared",
            kind: why.kind(),
        }),
    }
}

/// [`plan`] under the month's lock, keeping the records the cut keeps.
fn planned(root: &Path, path: StorePath<'_>, symbol_id: u32) -> Result<Planned, RecoverError> {
    let served = generation::served(root, path)?;
    let original = root;
    if let Served::Revision(pointer) = served {
        if let Some(record) = read_record(root, path)?
            && record.revision == pointer.revision
        {
            return Ok(Planned {
                plan: Plan::Recovered(record),
                rows: Vec::new(),
            });
        }
        // OUR OWN PROMOTION, NOT YET RECORDED: the revision replaced the
        // original and was published from the cut this plan finds in the
        // original's bytes, which never change after a recovery. An original
        // this door cannot cut (clean, sealed, refused for another reason) is
        // not this door's promotion: the served revision is judged below as
        // any other.
        if pointer.previous == 0
            && let Ok(Diagnosis::Cut(mut found)) = diagnose(original, path, symbol_id)
        {
            let source =
                RevisionReader::open(root, path, symbol_id, pointer.revision)?.source_header();
            if source == found.cut.header {
                if let Plan::Cut { promoted, .. } = &mut found.plan {
                    *promoted = Some(pointer);
                }
                return Ok(Planned {
                    plan: found.plan,
                    rows: found.rows,
                });
            }
        }
        // ANY OTHER REVISION: the month as served is what a reader opens.
        return match read_whole(&BarFile::open_existing(root, path, symbol_id)?) {
            Ok(rows) => Ok(Planned {
                plan: Plan::Clean {
                    ordinal: pointer.revision.ordinal(),
                    n_valid: rows.len() as u64,
                },
                rows: Vec::new(),
            }),
            Err(why) => Err(RecoverError::NotRaw {
                path: generation::pointer_path(root, path),
                why: format!(
                    "is served from repair revision {}, which this door did not promote, and it is refused ({why}); a promoted revision was synced whole before its promotion, so its refusal is not a power cut's",
                    pointer.revision.ordinal()
                ),
            }),
        };
    }
    match diagnose(original, path, symbol_id)? {
        Diagnosis::Clean { n_valid } => Ok(Planned {
            plan: Plan::Clean {
                ordinal: 0,
                n_valid,
            },
            rows: Vec::new(),
        }),
        Diagnosis::Cut(found) => Ok(Planned {
            plan: found.plan,
            rows: found.rows,
        }),
    }
}

/// What the original month's bytes say.
enum Diagnosis {
    Clean { n_valid: u64 },
    Cut(Box<Found>),
}

/// A cut, with its plan and its records.
struct Found {
    cut: Cut,
    plan: Plan,
    rows: Vec<Bar>,
}

/// Every committed record of `file`, through the verified read path, checked
/// in order, inside the month and on its grid, and against the header's own
/// first and last stamps.
fn read_whole(file: &BarFile) -> Result<Vec<Bar>, StoreError> {
    let header = file.header();
    let mut rows = Vec::with_capacity(usize::try_from(header.n_valid).unwrap_or(0));
    for index in 0..header.n_valid {
        rows.push(file.read_record(index)?);
    }
    Ok(rows)
}

/// The kept records' own checks, past the block checksums: in order, inside
/// the month and on its grid, and the header's first and last stamps theirs.
fn check_rows(path: StorePath<'_>, header: Header, rows: &[Bar]) -> Result<(), String> {
    if rows.is_empty() {
        return Err("it names no record, so there is nothing to keep".to_owned());
    }
    survey(rows).map_err(|why| why.to_string())?;
    Admission::new(path.month(), path.timeframe().secs())
        .admit(rows)
        .map_err(|why| why.to_string())?;
    let (first, last) = (
        rows.first().map_or(0, |bar| bar.ts_micros),
        rows.last().map_or(0, |bar| bar.ts_micros),
    );
    if first != header.first_ts_micros || last != header.last_ts_micros {
        return Err(format!(
            "its records run {first}..{last} and its header says {}..{}",
            header.first_ts_micros, header.last_ts_micros
        ));
    }
    Ok(())
}

/// One header slot that still decodes, where its generation puts it.
#[derive(Debug, Clone, Copy)]
struct Slot {
    header: Header,
}

/// The original month's raw facts: its length, its region's checksum, its
/// surviving slots and its sidecar's length.
struct Raw {
    bars: PathBuf,
    len: u64,
    region_crc: u32,
    slots: Vec<Slot>,
    sidecar_len: u64,
}

fn io_at(path: &Path, operation: &'static str) -> impl FnOnce(io::Error) -> RecoverError {
    let path = path.to_path_buf();
    move |why| RecoverError::Io {
        path,
        operation,
        kind: why.kind(),
    }
}

/// Reads the original month's length, header region and sidecar length, and
/// decodes each slot that sits where its own generation puts it.
fn raw(root: &Path, path: StorePath<'_>) -> Result<Raw, RecoverError> {
    let bars = path.to_path_buf(root);
    let (file, len) = crate::safe_open::open_read_len(&bars).map_err(|why| RecoverError::Io {
        path: bars.clone(),
        operation: "open the bar file",
        kind: why_kind(&why),
    })?;
    let want = usize::try_from(len.min(REGION_LEN)).unwrap_or(0);
    let mut region = vec![0u8; want];
    file.read_exact_at(&mut region, 0)
        .map_err(io_at(&bars, "read the header region"))?;
    let stride = usize::try_from(Layout::CURRENT.slot_stride()).unwrap_or(usize::MAX);
    let slot_count = Layout::CURRENT.slot_count();
    let slots = region
        .chunks(stride)
        .zip(0u64..)
        .filter_map(|(chunk, index)| {
            let header = Header::decode(chunk).ok()?;
            (header.generation % slot_count == index).then_some(Slot { header })
        })
        .collect();
    let sidecar = path.with_file(FileKind::Checksums).to_path_buf(root);
    let sidecar_len = match fs::symlink_metadata(&sidecar) {
        Ok(meta) => meta.len(),
        Err(why) if why.kind() == ErrorKind::NotFound => 0,
        Err(why) => return Err(io_at(&sidecar, "measure the checksum sidecar")(why)),
    };
    Ok(Raw {
        bars,
        len,
        region_crc: crc32c(&region),
        slots,
        sidecar_len,
    })
}

fn why_kind(why: &crate::safe_open::ReadRefusal) -> ErrorKind {
    match why {
        crate::safe_open::ReadRefusal::Io { source, .. } => source.kind(),
        _ => ErrorKind::InvalidInput,
    }
}

/// The original month as its bytes stand: clean, or refused with its cut.
fn diagnose(root: &Path, path: StorePath<'_>, symbol_id: u32) -> Result<Diagnosis, RecoverError> {
    refuse_not_raw(root, path)?;
    let refused = match BarFile::open_existing_at(root, path, symbol_id) {
        Ok(file) => match read_whole(&file) {
            Ok(rows) => {
                return Ok(Diagnosis::Clean {
                    n_valid: rows.len() as u64,
                });
            }
            Err(why) => why.to_string(),
        },
        // A MONTH THAT IS NOT THERE OR IS BUSY is the store's refusal, never a
        // cut: there is nothing torn to recover.
        Err(why @ (StoreError::Missing { .. } | StoreError::Locked { .. })) => {
            return Err(why.into());
        }
        Err(why) => why.to_string(),
    };
    let raw = raw(root, path)?;
    if raw
        .slots
        .iter()
        .any(|slot| !slot.header.checksums_present())
    {
        return Err(RecoverError::Unsealed { path: raw.bars });
    }
    let mut month = Month {
        root,
        path,
        symbol_id,
        layout: raw
            .slots
            .first()
            .and_then(|slot| Layout::for_version(slot.header.format_version).ok())
            .unwrap_or(Layout::CURRENT),
        claim: raw
            .slots
            .iter()
            .max_by_key(|slot| slot.header.n_valid)
            .copied(),
        refused,
        tried: Vec::new(),
        raw,
    };
    month
        .raw
        .slots
        .sort_by_key(|slot| std::cmp::Reverse(slot.header.generation));
    if let Some((cut, rows)) = month.commit_cut() {
        return month
            .found(cut, rows)
            .map(|found| Diagnosis::Cut(Box::new(found)));
    }
    let (cut, rows) = month.blocks_cut()?;
    month
        .found(cut, rows)
        .map(|found| Diagnosis::Cut(Box::new(found)))
}

/// A refused original month, as the plan reads it.
struct Month<'a> {
    root: &'a Path,
    path: StorePath<'a>,
    symbol_id: u32,
    layout: Layout,
    /// The surviving slot naming the most records: what was published.
    claim: Option<Slot>,
    /// The month's own refusal.
    refused: String,
    /// The commits tried, newest first.
    tried: Vec<Tried>,
    raw: Raw,
}

impl Month<'_> {
    /// A cut at `header`, against the bytes the plan read.
    fn cut(&self, header: Header, basis: Basis) -> Cut {
        Cut {
            bars: self.raw.bars.clone(),
            len: self.raw.len,
            region_crc: self.raw.region_crc,
            header,
            basis,
        }
    }

    /// The newest surviving commit whose every record checks out, with its
    /// records; each one that does not is kept in `tried` with why.
    fn commit_cut(&mut self) -> Option<(Cut, Vec<Bar>)> {
        let capacity = self.layout.capacity_for(self.raw.len);
        let slots = self.raw.slots.clone();
        for slot in slots {
            let header = slot.header;
            let refused = if header.n_valid == 0 {
                "it names no record, so there is nothing to keep".to_owned()
            } else if header.n_valid > capacity {
                format!(
                    "the file's {} bytes hold {capacity} whole record(s), so its records did not all reach the disk",
                    self.raw.len
                )
            } else {
                let cut = self.cut(
                    header,
                    Basis::Commit {
                        generation: header.generation,
                    },
                );
                match verified(self.root, self.path, self.symbol_id, &cut) {
                    Ok(rows) => return Some((cut, rows)),
                    Err(why) => why,
                }
            };
            self.tried.push(Tried {
                generation: header.generation,
                n_valid: header.n_valid,
                refused,
            });
        }
        None
    }

    /// NO COMMIT CHECKS OUT: the longest prefix of whole blocks that does,
    /// never past what a surviving slot published, and refused when the first
    /// block that fails is inside a durable history and a later one verifies.
    fn blocks_cut(&mut self) -> Result<(Cut, Vec<Bar>), RecoverError> {
        let Some(claimed) = self.claim else {
            return Err(self.nothing());
        };
        let per_block = self.layout.records_per_block();
        let whole = self
            .layout
            .capacity_for(self.raw.len)
            .min(self.raw.sidecar_len / 4 * per_block)
            .min(claimed.header.n_valid)
            / per_block;
        if whole == 0 {
            return Err(self.nothing());
        }
        let probe = self.cut(
            Header {
                n_valid: whole * per_block,
                ..claimed.header
            },
            Basis::Blocks {
                blocks: whole,
                generation: claimed.header.generation,
            },
        );
        let (root, path, symbol_id) = (self.root, self.path, self.symbol_id);
        let (rows, failed) = within(&probe, || {
            blocks_prefix(root, path, symbol_id, whole, per_block)
        })?;
        // WHAT THE OLDEST SURVIVING COMMIT PUBLISHED, no later append touched
        // below its tail block: when that commit was durable, a block below
        // that fails while a later whole block verifies is damage, not a
        // write the batch lost. At or past it the batch's own writes (its
        // records and the entries it re-sealed) reach the drive in any order,
        // so a verified block past a failed one is a power cut's state.
        let history = self
            .raw
            .slots
            .iter()
            .min_by_key(|slot| slot.header.generation)
            .map_or(0, |slot| slot.header.n_valid / per_block);
        if let Some(block) = failed.filter(|&block| block < history) {
            let verified_after = within(&probe, || {
                verified_blocks_after(root, path, symbol_id, block, whole, per_block)
            })?;
            if verified_after > 0 {
                return Err(RecoverError::NotTheTail {
                    refused: self.refused.clone(),
                    block,
                    verified_after,
                });
            }
        }
        let blocks = failed.unwrap_or(whole);
        let kept = blocks * per_block;
        let first = rows.first().map(|bar| bar.ts_micros);
        let last = usize::try_from(kept)
            .ok()
            .and_then(|kept| kept.checked_sub(1))
            .and_then(|at| rows.get(at))
            .map(|bar| bar.ts_micros);
        let (Some(first), Some(last)) = (first, last) else {
            return Err(self.nothing());
        };
        // THE OLDEST SURVIVING COMMIT THAT PUBLISHED EVERY KEPT RECORD.
        let generation = self
            .raw
            .slots
            .iter()
            .filter(|slot| slot.header.n_valid >= kept)
            .map(|slot| slot.header.generation)
            .min()
            .unwrap_or(claimed.header.generation);
        let cut = self.cut(
            Header {
                generation,
                n_valid: kept,
                first_ts_micros: first,
                last_ts_micros: last,
                ..claimed.header
            },
            Basis::Blocks { blocks, generation },
        );
        // THE CUT IS OPENED ONCE MORE AS THE DOORS WILL OPEN IT, so what is
        // kept is what the publication's own source check reads.
        match verified(root, path, symbol_id, &cut) {
            Ok(rows) => Ok((cut, rows)),
            Err(why) => {
                self.tried.push(Tried {
                    generation,
                    n_valid: kept,
                    refused: why,
                });
                Err(self.nothing())
            }
        }
    }

    /// Nothing safe to keep.
    fn nothing(&self) -> RecoverError {
        RecoverError::NoCommit {
            refused: self.refused.clone(),
            tried: self.tried.clone(),
        }
    }

    /// The plan of a cut, with what it drops.
    fn found(self, cut: Cut, rows: Vec<Bar>) -> Result<Found, RecoverError> {
        let header = cut.header;
        let claimed = self.claim.map_or(header, |slot| slot.header);
        let kept = header.n_valid;
        // A CUT THAT DROPS NOTHING ANSWERS A REFUSAL NO POWER CUT LEFT: every
        // record the newest commit names checks out, so the month's own
        // refusal is about something else and stays the month's.
        if claimed.n_valid <= kept {
            return Err(RecoverError::NoCommit {
                refused: format!(
                    "{}; every record its newest surviving commit names checks out, so the refusal is not a torn tail",
                    self.refused
                ),
                tried: self.tried,
            });
        }
        let month = self.path.month();
        let dropped = Dropped {
            kept,
            last_kept_ts: header.last_ts_micros,
            claimed: claimed.n_valid,
            claimed_generation: claimed.generation,
            claimed_last_ts: claimed.last_ts_micros,
            bars: claimed.n_valid - kept,
            first_day: day_of(month, header.last_ts_micros),
            last_day: day_of(month, claimed.last_ts_micros),
        };
        Ok(Found {
            plan: Plan::Cut {
                refused: self.refused,
                cut: cut.clone(),
                dropped,
                tried: self.tried,
                promoted: None,
            },
            cut,
            rows,
        })
    }
}

/// The refusals before any byte is judged: a sealed form, an indexed sibling.
fn refuse_not_raw(root: &Path, path: StorePath<'_>) -> Result<(), RecoverError> {
    for (kind, why) in [
        (
            FileKind::SealedBars,
            "is held in its sealed form, written once and checked whole, so no power cut tears it",
        ),
        (
            FileKind::Overlay,
            "is an overlay indexed by record beside the month; a cut would leave rows describing bars the month no longer holds",
        ),
        (
            FileKind::Greeks,
            "is a greeks file indexed by record beside the month; a cut would leave rows describing bars the month no longer holds",
        ),
    ] {
        let sibling = path.with_file(kind).to_path_buf(root);
        match fs::symlink_metadata(&sibling) {
            Ok(_) => {
                return Err(RecoverError::NotRaw {
                    path: sibling,
                    why: why.to_owned(),
                });
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(io_at(&sibling, "inspect a sibling")(error)),
        }
    }
    Ok(())
}

/// The month opened at `cut` on this thread, every kept record read through
/// the verified read path and checked.
fn verified(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    cut: &Cut,
) -> Result<Vec<Bar>, String> {
    within(cut, || {
        let file =
            BarFile::open_existing_at(root, path, symbol_id).map_err(|why| why.to_string())?;
        if file.header() != cut.header {
            return Err("the month's bytes moved since they were read".to_owned());
        }
        let rows = read_whole(&file).map_err(|why| why.to_string())?;
        check_rows(path, cut.header, &rows)?;
        Ok(rows)
    })
}

/// The records of the first `whole` blocks up to the first block that fails,
/// and that block. Inside [`within`] a probe header naming `whole` blocks.
fn blocks_prefix(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    whole: u64,
    per_block: u64,
) -> Result<(Vec<Bar>, Option<u64>), RecoverError> {
    let file = BarFile::open_existing_at(root, path, symbol_id)?;
    let mut rows = Vec::with_capacity(usize::try_from(whole * per_block).unwrap_or(0));
    for index in 0..whole * per_block {
        match file.read_record(index) {
            Ok(bar) => rows.push(bar),
            Err(_) => return Ok((rows, Some(index / per_block))),
        }
    }
    Ok((rows, None))
}

/// How many whole blocks past `failed`, below `whole`, verify: one record
/// read per block, which verifies that block alone.
fn verified_blocks_after(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    failed: u64,
    whole: u64,
    per_block: u64,
) -> Result<u64, RecoverError> {
    let file = BarFile::open_existing_at(root, path, symbol_id)?;
    Ok((failed + 1..whole)
        .filter(|block| file.read_record(block * per_block).is_ok())
        .count() as u64)
}

/// The IST day of `month` a stamp falls on, `1..=31`.
fn day_of(month: YearMonth, ts_micros: i64) -> u8 {
    let (from, _) = month.ist_bounds_micros();
    let day = (ts_micros - from).div_euclid(DAY_MICROS) + 1;
    u8::try_from(day.clamp(1, 31)).unwrap_or(1)
}

/// Recovers `path`'s month: plans it, publishes the kept records as a new
/// revision, promotes it and appends the record. Idempotent: a month already
/// recovered, or clean, writes nothing.
///
/// # Errors
///
/// Every refusal of [`plan`], [`crate::repair::publish`] and
/// [`crate::generation::promote`]; [`RecoverError::Record`] and
/// [`RecoverError::Io`] for the record file. A failure after the promotion
/// leaves the month recovered and unrecorded: the same call again writes the
/// record it owes.
pub fn recover(root: &Path, path: StorePath<'_>, symbol_id: u32) -> Result<Outcome, RecoverError> {
    let _month = month_lock(root, path)?;
    let Planned { plan, rows } = planned(root, path, symbol_id)?;
    let (refused, cut, dropped, tried, promoted) = match plan {
        Plan::Clean { ordinal, n_valid } => return Ok(Outcome::Clean { ordinal, n_valid }),
        Plan::Recovered(record) => return Ok(Outcome::Already(record)),
        Plan::Cut {
            refused,
            cut,
            dropped,
            tried,
            promoted,
        } => (refused, cut, dropped, tried, promoted),
    };
    let records = RecordFile::open(root, path)?;
    // ONE DOOR AT A TIME per month: the record file's own lock is taken
    // before anything is written, and what another door recorded meanwhile
    // is the answer.
    if let Some(record) = records.last()?
        && let Served::Revision(pointer) = generation::served(root, path)?
        && record.revision == pointer.revision
    {
        records.release()?;
        return Ok(Outcome::Already(record));
    }
    // A PROMOTION ALREADY MADE BY THIS CUT is resumed: only its record is
    // owed. Otherwise the kept records are published and promoted, each
    // door opening the month at the cut on this thread only.
    let (publication, promotion, resumed) = if let Some(pointer) = promoted {
        let publication = Publication {
            revision: pointer.revision,
            published: Published::Reused,
            skipped: Vec::new(),
        };
        (publication, Promoted::Already(pointer), true)
    } else {
        let publication = publish(root, path, symbol_id, &cut, &rows)?;
        let revision = publication.revision;
        let promotion = within(&cut, || {
            generation::promote(root, path, symbol_id, revision)
        })?;
        (publication, promotion, false)
    };
    let Publication {
        revision,
        published,
        skipped,
    } = publication;
    let record = records.append(Record {
        revision,
        basis: cut.basis,
        source: cut.header,
        dropped,
        bin_len: cut.len,
        region_crc: cut.region_crc,
        recovered_at_micros: now_micros(),
        reread_named: false,
        record: 0,
    })?;
    records.release()?;
    Ok(Outcome::Recovered(Recovered {
        refused,
        tried,
        published,
        promoted: promotion,
        record,
        skipped,
        resumed,
    }))
}

/// Says the caller has named `record`'s dropped days for the importer's
/// re-read: appends a copy of the month's last record with
/// [`Record::reread_named`] set. Idempotent: a last record already flagged is
/// returned and nothing is written.
///
/// # Errors
///
/// [`RecoverError::Record`] when the month's last record is not `record`'s
/// recovery (another recovery, or none), and the record file's own refusals.
pub fn note_reread(
    root: &Path,
    path: StorePath<'_>,
    record: &Record,
) -> Result<Record, RecoverError> {
    let records = RecordFile::open(root, path)?;
    let last = records.last()?;
    let noted = match last {
        Some(last) if last.same_recovery(record) && last.reread_named => last,
        Some(last) if last.same_recovery(record) => records.append(Record {
            reread_named: true,
            ..last
        })?,
        _ => {
            return Err(record_refused(
                &records.path,
                "its last record is not this recovery's, so the re-read cannot be noted against it"
                    .to_owned(),
            ));
        }
    };
    records.release()?;
    Ok(noted)
}

/// The revision the kept records were published at, and how.
struct Publication {
    revision: Revision,
    published: Published,
    /// Ordinals stepped over, each with why.
    skipped: Vec<(u32, String)>,
}

/// Publishes the kept records at the first ordinal that takes them: a
/// complete earlier publication of the same cut is reused, and an ordinal
/// holding another request or an unfinished one is stepped over and named,
/// as `repair::repair_month` steps over them.
fn publish(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    cut: &Cut,
    rows: &[Bar],
) -> Result<Publication, RecoverError> {
    let mut skipped = Vec::new();
    for ordinal in 1..=repair::MAX_REVISIONS {
        let revision = Revision::new(ordinal)?;
        match within(cut, || {
            repair::publish(root, path, symbol_id, revision, cut.header, rows)
        }) {
            Ok(published) => {
                return Ok(Publication {
                    revision,
                    published,
                    skipped,
                });
            }
            Err(
                why @ (RepairError::Conflict
                | RepairError::Incomplete(_)
                | RepairError::InvalidReceipt(_)),
            ) => skipped.push((ordinal, why.to_string())),
            Err(why) => return Err(why.into()),
        }
    }
    Err(RepairError::RevisionLimit.into())
}

/// A month's record file, held exclusively for the life of the value.
struct RecordFile {
    path: PathBuf,
    lock: Flock<File>,
    directory: PathBuf,
}

impl RecordFile {
    /// Opens (creating it and its directories when absent) and locks the
    /// record file, refusing a link below the root and another holder.
    fn open(root: &Path, month: StorePath<'_>) -> Result<Self, RecoverError> {
        let path = record_path(root, month);
        let directory = path
            .parent()
            .ok_or_else(|| record_refused(&path, "it has no directory".to_owned()))?
            .to_path_buf();
        refuse_links(root, &path)?;
        let missing = missing_dirs(root, &directory);
        fs::create_dir_all(&directory).map_err(io_at(&directory, "create record directories"))?;
        for made in &missing {
            sync_dir(made.parent().unwrap_or(root))?;
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        no_follow(&mut options);
        let handle = options
            .open(&path)
            .map_err(io_at(&path, "open the record file"))?;
        let lock = match Flock::try_lock(handle, path.clone()) {
            Ok(lock) => lock,
            Err(TryLockError::WouldBlock) => {
                return Err(StoreError::Locked { path }.into());
            }
            Err(TryLockError::Error(why)) => {
                return Err(io_at(&path, "lock the record file")(why));
            }
        };
        Ok(Self {
            path,
            lock,
            directory,
        })
    }

    fn last(&self) -> Result<Option<Record>, RecoverError> {
        last_record(&self.lock, &self.path)
    }

    /// Appends `record` at the next aligned offset (a torn tail is stepped
    /// over, never overwritten), syncs it and its directory.
    fn append(&self, mut record: Record) -> Result<Record, RecoverError> {
        let len = self
            .lock
            .metadata()
            .map_err(io_at(&self.path, "measure the record file"))?
            .len();
        let index = len.div_ceil(RECORD_LEN);
        if index >= MAX_RECORDS {
            return Err(record_refused(
                &self.path,
                format!("it already holds {index} record(s), the most a month's file holds"),
            ));
        }
        record.record = index;
        let bytes = record.encode().map_err(|why| match why {
            RecoverError::Record { reason, .. } => record_refused(&self.path, reason),
            other => other,
        })?;
        self.lock
            .write_all_at(&bytes, index * RECORD_LEN)
            .map_err(io_at(&self.path, "write the record"))?;
        self.lock
            .sync_all()
            .map_err(io_at(&self.path, "sync the record"))?;
        sync_dir(&self.directory)?;
        Ok(record)
    }

    fn release(self) -> Result<(), RecoverError> {
        let Self { path, lock, .. } = self;
        lock.release().map_err(|why| RecoverError::Io {
            path,
            operation: "release the record file's lock",
            kind: io::Error::from(why).kind(),
        })
    }
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
fn refuse_links(root: &Path, leaf: &Path) -> Result<(), RecoverError> {
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

fn sync_dir(dir: &Path) -> Result<(), RecoverError> {
    let handle = File::open(dir).map_err(io_at(dir, "open a directory for sync"))?;
    handle.sync_all().map_err(io_at(dir, "sync a directory"))
}

fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_micros()).unwrap_or(i64::MAX)
        })
}

#[cfg(test)]
#[path = "recover_tests.rs"]
mod tests;
