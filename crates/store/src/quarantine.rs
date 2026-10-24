//! The quarantine of named bars (D-7290): an append-only ledger that is the truth, and
//! an index every reader probes in ONE positional read. R4-18; findings
//! ING-01 (a, b, e), RT-DATA-02 (c, d), Z5, Z6.
//!
//! # Why this exists
//!
//! A share's stored Zerodha series can hold bars that are not that share's:
//! rows from before it listed (IRFC from 2018-03 at a level another security
//! traded at, KFINTECH, MAZDOCK), a day row whose every price is zero
//! (MAZDOCK 1day 2018-01..07), and minutes of another price basis
//! interleaved with its own (PIIND 1min 2018-12..2020-01). R4-17's
//! `cli series-identity` names them in a quarantine plan and writes no bar.
//! This module is what acts on that plan without ever mutating a month in
//! place (`CLAUDE.md` §3 rule 8):
//!
//! * a month whose EVERY bar is named is recorded here and answered BY NAME
//!   by every reader — no empty generation is published for it;
//! * a month where only SOME bars are named gets a new generation without
//!   them through [`crate::repair::drop_month`] (the old generation stays
//!   byte-identical), and is recorded here so a reader can say what was
//!   dropped, why, and where the ledger line is.
//!
//! # The ledger — `<root>/audit/<feed>.quarantine.ledger`
//!
//! Text, one line per act, never rewritten:
//!
//! ```text
//! brutex quarantine ledger v1
//! <written_at_utc_micros>\t<feed>\t<key>\t<yyyy-mm>\t<scope>\t<cause>\t<bars>\t<revision>\t<source_digest>\t<who>\t<reason>\t<stamps>\t<crc32c>
//! ```
//!
//! `scope` is `dropping` (written BEFORE the door publishes: the intent a
//! restart resumes from), `dropped` (after the promotion) or `whole_month`.
//! `stamps` lists every named bar's UTC micros, comma-separated, or `-` for a
//! whole month. `source_digest` is the BLAKE3 of the named bars as held
//! (stamp, open, high, low, close, volume, open interest; little-endian
//! `i64`s), so what was set aside stays provable. The last field is the
//! CRC-32C of everything before its tab. A line with no newline is a TORN
//! append (a writer killed mid-line): it is never read as a line, and the
//! next append ends it with [`TORN_END`] (`\ttorn` and a newline) first
//! (`pull::alerts`' torn-tail rule without the overwrite: no byte of the
//! ledger is ever changed). A line ending `\ttorn` is that ENDED torn append:
//! never a line and never damage, because the kill it records left no act —
//! the writer it cut off never returned. Every OTHER complete line that fails
//! its checksum or does not parse is DAMAGE: the act it recorded is not known,
//! so no index is rebuilt from the ledger, no listing counts it and no apply
//! writes past it until the ledger is restored; each is refused by name,
//! [`QuarantineError::LedgerDamaged`], with its byte offset (R4-18 repair 2).
//!
//! # The index — `<root>/audit/<feed>.quarantine`
//!
//! Derived from the ledger, never the truth: a 64-byte header, then
//! [`CAPACITY`] `+` [`BUCKET`] `- 1` fixed 64-byte slots, open-addressed by
//! the BLAKE3 of (series key, month). A key's home slot is its hash modulo
//! [`CAPACITY`]; it lives in the first free or matching slot of the
//! [`BUCKET`] slots from there, and the trailing `BUCKET - 1` overflow slots
//! mean that run never wraps. So a probe is ONE `pread` of
//! [`BUCKET`] `x 64` bytes, whatever the store holds.
//!
//! | Offset | Bytes | Slot field |
//! | --- | --- | --- |
//! | 0 | 16 | BLAKE3 of the series key and month, first 16 bytes |
//! | 16 | 4 | month, `yyyy * 100 + mm`, `u32` LE |
//! | 20 | 1 | scope: 1 whole month, 2 dropped |
//! | 21 | 1 | cause: 1 before listing, 2 foreign level, 3 zero price |
//! | 24 | 8 | bars named (cumulative over drops) |
//! | 32 | 8 | byte offset of the newest ledger line for it |
//! | 40 | 4 | the revision ordinal served without them (0: none) |
//! | 44 | 8 | written at, UTC micros |
//! | 60 | 4 | CRC-32C of bytes 0..60 followed by the slot's index, `u64` LE |
//!
//! An all-zero slot is free. Every publish of the index writes a whole new
//! image to a staging file, syncs it and renames it over the old one, so a
//! reader sees the old index or the new one and never half of either. An
//! index of the wrong length (a torn copy, a truncation) or whose header does
//! not check (no magic, another version, a bad checksum: a zeroed image too)
//! answers [`QuarantineError::Torn`] or [`QuarantineError::HeaderDamaged`] by
//! name, never "clear", and is rebuilt from the ledger by the next writer or
//! by [`rebuild`].
//!
//! The header (version 2, R4-18 repair 2):
//!
//! | Offset | Bytes | Header field |
//! | --- | --- | --- |
//! | 0 | 8 | [`MAGIC`] |
//! | 8 | 4 | [`INDEX_VERSION`], `u32` LE |
//! | 12 | 4 | slot bytes, 64 |
//! | 16 | 8 | [`CAPACITY`] |
//! | 24 | 8 | [`BUCKET`] |
//! | 32 | 8 | slots held |
//! | 40 | 8 | LEDGER BYTES DESCRIBED: every complete line before this offset is in the image |
//! | 60 | 4 | CRC-32C of bytes 0..60 |
//!
//! THE INDEX NEVER LAGS ITS LEDGER UNSAID. A reader compares the ledger
//! bytes the image describes with the ledger's length (one `lstat`), and only
//! an image that describes the WHOLE ledger is ever the answer. One behind it
//! holds an act ledgered and not indexed: a writer's act in flight, an act
//! whose publish was refused or cut off by a kill after its ledger line, or a
//! ledger restored or cut. While the next writer is bringing the index level,
//! a reader cannot tell an act in flight from a failed one, so it never takes
//! the lag for the first: while a writer holds the lock it waits for that
//! writer's image within its thread's patience ([`crate::flock::patience`]:
//! none for the CLI and the importer, some for the server's page reads) and is
//! refused by name when that runs out ([`QuarantineError::BeingWritten`]);
//! with no writer it is refused by name too ([`QuarantineError::Stale`]). A
//! writer that finds the image behind (or ahead of) the ledger rebuilds it
//! from the ledger under the lock before its own act, so the next writer of
//! any month of the feed finishes what a failed one left.
//!
//! # Writers
//!
//! One at a time, under `<root>/audit/<feed>.quarantine.lock` taken
//! patiently: a second writer waits, and when the bound runs out it is
//! refused naming the holder ([`QuarantineError::Held`]). [`apply_month`]
//! holds it for its whole sequence (probe, intent, door, `dropped` line), so
//! two applies of one feed never interleave. A refusal writes NOTHING: a
//! damaged ledger, a full bucket and a bad reason are refused before the
//! ledger line; only a host refusal after it can leave a line unindexed, and
//! that is what [`QuarantineError::Stale`] names until the next writer.
//!
//! An intent (`dropping`) is indexed too, as a drop of no bars: from the
//! moment a drop is ledgered every reader knows the month is named, and the
//! doors that add or restate bars ([`set_aside`]) read its stamps.
//!
//! # Cost
//!
//! [`probe`]: one `open` (ENOENT for a feed never quarantined), one `fstat`,
//! one `lstat` of the ledger and two positional reads (the 64-byte header and
//! the 512-byte bucket) — O(1); when the image and the ledger disagree, a
//! second look at once and one non-blocking `flock`, and while a writer holds
//! the lock the same again after each pause, for at most the thread's
//! patience (never past [`crate::flock::MAX_PATIENCE`]). [`Snapshot::load`]:
//! one read of the whole index (524,800 bytes), once per cached body; each
//! lookup after it O(1). [`record`]: one ledger append and one index image —
//! O(index) on the write path, never on a read, plus one pass over the ledger
//! when the image must be rebuilt. [`rebuild`], [`ledger_stamps`],
//! [`set_aside`] for a month a drop names, and [`apply_month`]: one pass over
//! the ledger, on the write path only.

use std::collections::HashMap;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _, Write as _};
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use brutex_core::vendor::Vendor;

use crate::crc::{crc32c, crc32c_split};
use crate::flock::{Flock, Impatient};
use crate::format::Bar;
use crate::path::{FileKind, STORE_ROOT, StorePath, YearMonth};

/// The directory under the store root the ledger and the index live in.
pub const DIR: &str = "audit";
/// Home slots in the index.
pub const CAPACITY: u64 = 8_192;
/// Slots one probe reads, and the most slots a key may lie from its home.
pub const BUCKET: u64 = 8;
/// Bytes of one slot.
pub const SLOT_LEN: u64 = 64;
/// Bytes of the index header.
pub const HEADER_LEN: u64 = 64;
/// Slots in the file: the home slots and the overflow that keeps a bucket
/// from wrapping.
pub const SLOTS: u64 = CAPACITY + BUCKET - 1;
/// The only length an index may have.
pub const INDEX_LEN: u64 = HEADER_LEN + SLOTS * SLOT_LEN;
/// The longest reason a record carries, in bytes.
pub const MAX_REASON: usize = 200;
/// How long a second writer waits for the first before it is refused: the
/// store's one bound on a patient lock, [`crate::flock::MAX_PATIENCE`]
/// (1 s), which `Flock::try_lock_patiently` clamps every wait to. It was
/// written as 10 s, a wait the clamp never allowed (R4-18 repair 1).
pub const WRITER_PATIENCE: Duration = crate::flock::MAX_PATIENCE;
/// The index header's magic.
pub const MAGIC: [u8; 8] = *b"BRXQIX01";
/// The index header's version. 2 records the ledger bytes the image
/// describes (R4-18 repair 2); an image of any other version is rebuilt.
pub const INDEX_VERSION: u32 = 2;
/// The ledger's first line.
pub const LEDGER_HEAD: &str = "brutex quarantine ledger v1";
/// What a writer appends after a torn last line before its own: the fragment
/// then reads as an ENDED torn append, never as a line and never as damage.
pub const TORN_END: &str = "\ttorn\n";
/// The end of an ended torn append, as [`parse_ledger`] reads it.
const TORN_MARK: &[u8] = b"\ttorn";

const SLOT: usize = 64;
const SLOT_U32: u32 = 64;
const HEADER: usize = 64;
const BUCKET_BYTES: usize = SLOT * 8;
/// [`INDEX_LEN`] as a length in memory.
const INDEX_BYTES: usize = HEADER + (8_192 + 8 - 1) * SLOT;
const _: () = assert!(
    INDEX_BYTES as u64 == INDEX_LEN,
    "the index length in two units"
);

/// Why a bar is quarantined. Fixed by the plan that named it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Cause {
    /// Dated before the share's listing day: another security's rows under a
    /// reused token (Z5).
    BeforeListing,
    /// A level that is neither the share's own day nor GDFL's minute (Z6,
    /// RT-DATA-02).
    ForeignLevel,
    /// A price at or below zero (ING-01: MAZDOCK's day rows).
    ZeroPrice,
}

impl Cause {
    /// Every cause, in code order.
    pub const ALL: [Self; 3] = [Self::BeforeListing, Self::ForeignLevel, Self::ZeroPrice];

    /// The word the plan, the ledger and every answer carry.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::BeforeListing => "before_listing",
            Self::ForeignLevel => "foreign_level",
            Self::ZeroPrice => "zero_price",
        }
    }

    /// The cause a word names, or `None`.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|cause| cause.code() == word)
    }

    const fn byte(self) -> u8 {
        match self {
            Self::BeforeListing => 1,
            Self::ForeignLevel => 2,
            Self::ZeroPrice => 3,
        }
    }

    const fn of_byte(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(Self::BeforeListing),
            2 => Some(Self::ForeignLevel),
            3 => Some(Self::ZeroPrice),
            _ => None,
        }
    }
}

/// What a ledger line records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The intent, written before the door publishes. Never indexed.
    Dropping,
    /// A generation without the named bars is served.
    Dropped,
    /// Every bar of the month is named: nothing of it is served.
    WholeMonth,
}

impl Scope {
    /// The ledger's word.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Dropping => "dropping",
            Self::Dropped => "dropped",
            Self::WholeMonth => "whole_month",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        [Self::Dropping, Self::Dropped, Self::WholeMonth]
            .into_iter()
            .find(|scope| scope.code() == word)
    }

    const fn byte(self) -> u8 {
        match self {
            Self::Dropping => 0,
            Self::WholeMonth => 1,
            Self::Dropped => 2,
        }
    }

    const fn of_byte(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(Self::WholeMonth),
            2 => Some(Self::Dropped),
            _ => None,
        }
    }
}

/// One month's quarantine as the index holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Whole month, or dropped from a served generation.
    pub scope: Scope,
    /// Why.
    pub cause: Cause,
    /// Bars named, over every drop of the month.
    pub bars: u64,
    /// Byte offset of the newest ledger line for the month.
    pub ledger_at: u64,
    /// The revision served without them; `0` for a whole month.
    pub revision: u32,
    /// When it was written, UTC micros.
    pub written_at: i64,
}

/// What a probe found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
    /// No quarantine names the month.
    Clear,
    /// The month's quarantine.
    Quarantined(Entry),
}

/// Every refusal of this module, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuarantineError {
    /// `quarantine_index_torn`: the index is not [`INDEX_LEN`] bytes. Rebuilt
    /// from the ledger by the next [`record`] or [`rebuild`]; never "clear".
    Torn {
        /// The index.
        path: PathBuf,
        /// Its length.
        len: u64,
    },
    /// `quarantine_index_torn` too: the index is [`INDEX_LEN`] bytes and its
    /// header does not check (no [`MAGIC`], another [`INDEX_VERSION`] or
    /// geometry, a bad checksum: a zeroed image). Never "clear".
    HeaderDamaged {
        /// The index.
        path: PathBuf,
    },
    /// `quarantine_slot_damaged`: a slot on the key's run fails its CRC.
    Damaged {
        /// The index.
        path: PathBuf,
        /// The slot.
        slot: u64,
    },
    /// `quarantine_index_stale`: the image describes `built_from` bytes of
    /// the ledger, which holds `ledger_len`, and no writer holds the lock: an
    /// act is ledgered and not indexed (a publish refused or killed after its
    /// ledger line), or the ledger was restored or cut. Never "clear"; the
    /// next writer of the feed rebuilds the index first.
    Stale {
        /// The index.
        path: PathBuf,
        /// The ledger bytes the image describes.
        built_from: u64,
        /// The ledger's length now.
        ledger_len: u64,
    },
    /// `quarantine_being_written`: the image describes `built_from` bytes of
    /// the ledger, which holds `ledger_len`, and a writer held the feed's lock
    /// for all `waited_ms` this reader could wait (its thread's patience,
    /// [`crate::flock::patience`]: none for the CLI and the importer). The
    /// writer may be writing its own act or bringing the index level with
    /// one a failed publish left unindexed; a reader cannot tell which, so an
    /// image behind its ledger is never the answer (R4-18 repair 2). Its
    /// sentence carries [`crate::flock::BEING_WRITTEN`]: ask again.
    BeingWritten {
        /// The index.
        path: PathBuf,
        /// The ledger bytes the image describes.
        built_from: u64,
        /// The ledger's length at the last look.
        ledger_len: u64,
        /// How long this reader waited, ms.
        waited_ms: u64,
    },
    /// `quarantine_ledger_damaged`: complete ledger lines that fail their
    /// checksum or do not parse, by byte offset. The act each recorded is not
    /// known, so nothing is rebuilt from the ledger, counted from it or
    /// written past it.
    LedgerDamaged {
        /// The ledger.
        path: PathBuf,
        /// Every damaged line's byte offset, ascending.
        offsets: Vec<u64>,
    },
    /// `quarantine_index_full`: every slot of the key's bucket holds another
    /// key. Nothing was written: refused before the ledger line.
    Full {
        /// The index.
        path: PathBuf,
    },
    /// `quarantine_held`: another writer kept the lock past
    /// [`WRITER_PATIENCE`].
    Held {
        /// The lock.
        path: PathBuf,
        /// Who holds it (`store::flock::holder_note`).
        holder: String,
    },
    /// `quarantine_reason`: empty, longer than [`MAX_REASON`] bytes, or
    /// holding a tab or a line break.
    Reason(String),
    /// `quarantine_linked`: the audit directory is a symbolic link.
    Linked(PathBuf),
    /// `quarantine_io`: a host refusal.
    Io {
        /// The file.
        path: PathBuf,
        /// What was being done.
        operation: &'static str,
        /// The host's kind.
        kind: io::ErrorKind,
    },
}

impl QuarantineError {
    /// The code a page and a log carry.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Torn { .. } | Self::HeaderDamaged { .. } => "quarantine_index_torn",
            Self::Damaged { .. } => "quarantine_slot_damaged",
            Self::Stale { .. } => "quarantine_index_stale",
            Self::BeingWritten { .. } => "quarantine_being_written",
            Self::LedgerDamaged { .. } => "quarantine_ledger_damaged",
            Self::Full { .. } => "quarantine_index_full",
            Self::Held { .. } => "quarantine_held",
            Self::Reason(_) => "quarantine_reason",
            Self::Linked(_) => "quarantine_linked",
            Self::Io { .. } => "quarantine_io",
        }
    }
}

impl fmt::Display for QuarantineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Torn { path, len } => write!(
                f,
                "quarantine_index_torn: {} is {len} bytes, not {INDEX_LEN}; the quarantine is not known until it is rebuilt from its ledger",
                path.display()
            ),
            Self::HeaderDamaged { path } => write!(
                f,
                "quarantine_index_torn: the header of {} does not check (no magic, another version or geometry, or a bad checksum); the quarantine is not known until it is rebuilt from its ledger",
                path.display()
            ),
            Self::Damaged { path, slot } => write!(
                f,
                "quarantine_slot_damaged: slot {slot} of {} fails its checksum; the quarantine is not known until it is rebuilt from its ledger",
                path.display()
            ),
            Self::Stale {
                path,
                built_from,
                ledger_len,
            } => write!(
                f,
                "quarantine_index_stale: {} describes {built_from} bytes of its ledger, which holds {ledger_len}, and no writer is at work: an act is ledgered and not indexed (a publish refused or killed after its ledger line), or the ledger was restored or cut; the quarantine is not known until the next writer rebuilds the index from its ledger",
                path.display()
            ),
            Self::BeingWritten {
                path,
                built_from,
                ledger_len,
                waited_ms,
            } => write!(
                f,
                "quarantine_being_written: {} describes {built_from} bytes of its ledger, which holds {ledger_len}, and a writer held the quarantine lock for all {waited_ms} ms this reader could wait: an act is being written, or the index is being brought level with one a failed publish left unindexed, and an image behind its ledger is never the answer; {}",
                path.display(),
                crate::flock::BEING_WRITTEN
            ),
            Self::LedgerDamaged { path, offsets } => write!(
                f,
                "quarantine_ledger_damaged: {} complete line(s) of {} fail their checksum or do not parse, at byte {}; the act each recorded is not known, so nothing is rebuilt from the ledger, counted from it or written past it until it is restored",
                offsets.len(),
                path.display(),
                offsets
                    .iter()
                    .take(8)
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
                    + if offsets.len() > 8 { ", ..." } else { "" }
            ),
            Self::Full { path } => write!(
                f,
                "quarantine_index_full: every slot of this key's bucket in {} holds another month; nothing was written",
                path.display()
            ),
            Self::Held { path, holder } => write!(
                f,
                "quarantine_held: {} is held by another writer ({holder}); nothing was written",
                path.display()
            ),
            Self::Reason(why) => write!(f, "quarantine_reason: {why}"),
            Self::Linked(path) => write!(
                f,
                "quarantine_linked: {} is a symbolic link; a quarantine is written only inside the store",
                path.display()
            ),
            Self::Io {
                path,
                operation,
                kind,
            } => write!(f, "quarantine_io: {operation} {}: {kind:?}", path.display()),
        }
    }
}

impl std::error::Error for QuarantineError {}

fn io_at<T>(
    result: io::Result<T>,
    path: &Path,
    operation: &'static str,
) -> Result<T, QuarantineError> {
    result.map_err(|why| QuarantineError::Io {
        path: path.to_path_buf(),
        operation,
        kind: why.kind(),
    })
}

/// `<root>/audit/<feed>.quarantine`.
#[must_use]
pub fn index_path(root: &Path, feed: Vendor) -> PathBuf {
    root.join(DIR).join(format!("{}.quarantine", feed.as_str()))
}

/// `<root>/audit/<feed>.quarantine.ledger`.
#[must_use]
pub fn ledger_path(root: &Path, feed: Vendor) -> PathBuf {
    root.join(DIR)
        .join(format!("{}.quarantine.ledger", feed.as_str()))
}

fn lock_path(root: &Path, feed: Vendor) -> PathBuf {
    root.join(DIR)
        .join(format!("{}.quarantine.lock", feed.as_str()))
}

/// The series a month belongs to, as the ledger and the index name it:
/// `<exchange>/<segment>/<symbol>[/<contract>]/<rung>`, from the one path
/// renderer. No table of instruments.
#[must_use]
pub fn series_key(path: StorePath<'_>) -> String {
    let rendered = path.with_file(FileKind::Bars).to_string();
    let prefix_len = STORE_ROOT.len() + 1 + path.vendor().as_str().len() + 1;
    let end = rendered.rfind('/').unwrap_or(rendered.len());
    rendered
        .get(prefix_len..end)
        .map_or_else(|| rendered.clone(), str::to_owned)
}

/// The month as the index stores it.
#[must_use]
pub const fn month_code(month: YearMonth) -> u32 {
    month.year() as u32 * 100 + month.month() as u32
}

fn hash_of(key: &str, month: YearMonth) -> [u8; 16] {
    let mut text = Vec::with_capacity(key.len() + 8);
    text.extend_from_slice(key.as_bytes());
    text.push(0);
    text.extend_from_slice(&month_code(month).to_le_bytes());
    let full = brutex_core::blake3::hash(&text);
    let mut out = [0u8; 16];
    out.copy_from_slice(full.get(..16).unwrap_or(&[0u8; 16]));
    out
}

fn home_of(hash: &[u8; 16]) -> u64 {
    let mut low = [0u8; 8];
    low.copy_from_slice(hash.get(..8).unwrap_or(&[0u8; 8]));
    u64::from_le_bytes(low) % CAPACITY
}

fn slot_crc(bytes: &[u8], slot: u64) -> u32 {
    crc32c_split(bytes.get(..60).unwrap_or(&[]), &slot.to_le_bytes())
}

fn field<const N: usize>(bytes: &[u8], at: usize) -> [u8; N] {
    let mut out = [0u8; N];
    if let Some(src) = bytes.get(at..at + N) {
        out.copy_from_slice(src);
    }
    out
}

/// What one slot's 64 bytes say.
enum Slot {
    Free,
    Held {
        hash: [u8; 16],
        month: u32,
        entry: Option<Entry>,
    },
    Damaged,
}

fn decode_slot(bytes: &[u8], slot: u64) -> Slot {
    if bytes.iter().all(|byte| *byte == 0) {
        return Slot::Free;
    }
    if u32::from_le_bytes(field(bytes, 60)) != slot_crc(bytes, slot) {
        return Slot::Damaged;
    }
    let hash = field::<16>(bytes, 0);
    let month = u32::from_le_bytes(field(bytes, 16));
    let entry = Scope::of_byte(bytes.get(20).copied().unwrap_or(0))
        .zip(Cause::of_byte(bytes.get(21).copied().unwrap_or(0)))
        .map(|(scope, cause)| Entry {
            scope,
            cause,
            bars: u64::from_le_bytes(field(bytes, 24)),
            ledger_at: u64::from_le_bytes(field(bytes, 32)),
            revision: u32::from_le_bytes(field(bytes, 40)),
            written_at: i64::from_le_bytes(field(bytes, 44)),
        });
    Slot::Held { hash, month, entry }
}

fn encode_slot(hash: &[u8; 16], month: u32, entry: &Entry, slot: u64) -> [u8; SLOT] {
    let mut bytes = [0u8; SLOT];
    let fields: [(usize, &[u8]); 7] = [
        (0, hash),
        (16, &month.to_le_bytes()),
        (24, &entry.bars.to_le_bytes()),
        (32, &entry.ledger_at.to_le_bytes()),
        (40, &entry.revision.to_le_bytes()),
        (44, &entry.written_at.to_le_bytes()),
        (20, &[entry.scope.byte(), entry.cause.byte()]),
    ];
    for (at, value) in fields {
        if let Some(dst) = bytes.get_mut(at..at + value.len()) {
            dst.copy_from_slice(value);
        }
    }
    let crc = slot_crc(&bytes, slot);
    if let Some(dst) = bytes.get_mut(60..64) {
        dst.copy_from_slice(&crc.to_le_bytes());
    }
    bytes
}

/// Answers whether `path`'s month is quarantined. O(1): the header and the
/// key's bucket, two positional reads, and the ledger's length.
///
/// # Errors
///
/// [`QuarantineError::Torn`] for an index of the wrong length,
/// [`QuarantineError::HeaderDamaged`] for one whose header does not check,
/// [`QuarantineError::Stale`] for one that does not describe the whole
/// ledger while no writer is at work, [`QuarantineError::BeingWritten`] for
/// one that still does not as the thread's patience runs out while a writer
/// holds the lock, [`QuarantineError::Damaged`] for a slot
/// on the key's run that fails its checksum, and a host refusal. Never
/// [`Probe::Clear`] for an index it cannot trust.
pub fn probe(root: &Path, path: StorePath<'_>) -> Result<Probe, QuarantineError> {
    probe_at(root, path.vendor(), &series_key(path), path.month())
}

/// [`probe`] by feed, series key and month.
///
/// # Errors
///
/// As [`probe`].
pub fn probe_at(
    root: &Path,
    feed: Vendor,
    key: &str,
    month: YearMonth,
) -> Result<Probe, QuarantineError> {
    let index = index_path(root, feed);
    let Some(file) = level_read(root, feed, &index, || open_index(&index))? else {
        return Ok(Probe::Clear);
    };
    let hash = hash_of(key, month);
    let home = home_of(&hash);
    let mut run = [0u8; BUCKET_BYTES];
    io_at(
        file.read_exact_at(&mut run, HEADER_LEN + home * SLOT_LEN),
        &index,
        "read the quarantine index",
    )?;
    find_in_run(&run, home, &hash, month_code(month), &index)
}

/// The feed's index opened for a reader: `None` when there is none, else the
/// file and the ledger bytes its header says the image describes. One
/// `open`, one `fstat` and one 64-byte positional read.
fn open_index(index: &Path) -> Result<Option<(File, u64)>, QuarantineError> {
    let (file, len) = match crate::safe_open::open_read_len(index) {
        Ok(found) => found,
        Err(why) if why.is_missing() => return Ok(None),
        Err(why) => {
            return Err(QuarantineError::Io {
                path: index.to_path_buf(),
                operation: "open the quarantine index",
                kind: refusal_kind(&why),
            });
        }
    };
    if len != INDEX_LEN {
        return Err(QuarantineError::Torn {
            path: index.to_path_buf(),
            len,
        });
    }
    let mut header = [0u8; HEADER];
    io_at(
        file.read_exact_at(&mut header, 0),
        index,
        "read the quarantine index header",
    )?;
    let built_from = described(&header).ok_or_else(|| QuarantineError::HeaderDamaged {
        path: index.to_path_buf(),
    })?;
    Ok(Some((file, built_from)))
}

/// The ledger bytes a header says its image describes, or `None` when the
/// header does not check: no [`MAGIC`], another [`INDEX_VERSION`], slot size,
/// [`CAPACITY`] or [`BUCKET`], or a bad checksum (a zeroed image).
fn described(header: &[u8]) -> Option<u64> {
    let checks = header.get(..8) == Some(&MAGIC[..])
        && u32::from_le_bytes(field(header, 8)) == INDEX_VERSION
        && u32::from_le_bytes(field(header, 12)) == SLOT_U32
        && u64::from_le_bytes(field(header, 16)) == CAPACITY
        && u64::from_le_bytes(field(header, 24)) == BUCKET
        && u32::from_le_bytes(field(header, 60)) == crc32c(header.get(..60).unwrap_or(&[]));
    checks.then(|| u64::from_le_bytes(field(header, 40)))
}

/// One look at the feed's index against its ledger.
enum Look<T> {
    /// The image describes the whole ledger: what was read of it, `None`
    /// when there is no index and no ledger.
    Level(Option<T>),
    /// It does not: the ledger bytes it describes, the ledger's length, and
    /// whether a writer held the lock at this instant.
    Behind {
        built_from: u64,
        ledger_len: u64,
        writing: bool,
    },
}

/// `open` once, then the ledger's length (one `lstat`), and one
/// non-blocking shared `flock` of the writer's lock only when the two
/// disagree.
fn look<T>(
    root: &Path,
    feed: Vendor,
    open: &mut impl FnMut() -> Result<Option<(T, u64)>, QuarantineError>,
) -> Result<Look<T>, QuarantineError> {
    let opened = open()?;
    let built_from = opened.as_ref().map_or(0, |(_, built)| *built);
    let ledger_len = ledger_len(root, feed)?;
    if ledger_len == built_from {
        return Ok(Look::Level(opened.map(|(found, _)| found)));
    }
    Ok(Look::Behind {
        built_from,
        ledger_len,
        writing: ledger_len > built_from && writer_active(root, feed),
    })
}

/// What `open` reads of the feed's index (it answers what it read and the
/// ledger bytes the image describes), ONLY from an image that describes the
/// whole ledger. AN IMAGE BEHIND ITS LEDGER IS NEVER THE ANSWER (R4-18
/// repair 2): a writer holding the lock may be writing its own act, or
/// bringing the index level with one a failed publish left unindexed, and a
/// reader cannot tell which — taking the lag for the first answered "clear"
/// for a month the second recorded whole.
///
/// Each attempt looks twice, the second at once: a writer may rename a new
/// image between the read of the index and the ledger's length. While a
/// writer holds the lock the reader asks again through the store's one wait
/// loop ([`crate::flock::patiently`], pausing 1 ms doubling to 16 ms) for at
/// most its thread's patience ([`crate::flock::patience`]: none, so refused
/// at once, unless its caller set some, as the server's page reads do), and
/// what it waited is spent from that patience, so the months one request
/// probes share one budget.
///
/// # Errors
///
/// [`QuarantineError::Stale`] when the image lags its ledger and no writer
/// holds the lock, [`QuarantineError::BeingWritten`] when one still does as
/// the patience runs out, and `open`'s and the ledger's refusals.
fn level_read<T>(
    root: &Path,
    feed: Vendor,
    index: &Path,
    mut open: impl FnMut() -> Result<Option<(T, u64)>, QuarantineError>,
) -> Result<Option<T>, QuarantineError> {
    let started = Instant::now();
    let mut behind = (0, 0);
    let mut answer: Result<Option<T>, QuarantineError> = Err(QuarantineError::Stale {
        path: index.to_path_buf(),
        built_from: 0,
        ledger_len: 0,
    });
    let waited = crate::flock::patiently(
        crate::flock::patience(),
        Instant::now,
        std::thread::sleep,
        || {
            let mut writing = false;
            for _ in 0..2 {
                match look(root, feed, &mut open) {
                    Ok(Look::Behind {
                        built_from,
                        ledger_len,
                        writing: now,
                    }) => {
                        behind = (built_from, ledger_len);
                        writing = now;
                    }
                    Ok(Look::Level(found)) => {
                        answer = Ok(found);
                        return Ok(());
                    }
                    Err(why) => {
                        answer = Err(why);
                        return Ok(());
                    }
                }
            }
            answer = Err(QuarantineError::Stale {
                path: index.to_path_buf(),
                built_from: behind.0,
                ledger_len: behind.1,
            });
            if writing {
                Err(std::fs::TryLockError::WouldBlock)
            } else {
                Ok(())
            }
        },
    );
    let spent = started.elapsed();
    crate::flock::spend_patience(spent);
    match waited {
        Ok(()) => answer,
        Err(_still_written) => Err(QuarantineError::BeingWritten {
            path: index.to_path_buf(),
            built_from: behind.0,
            ledger_len: behind.1,
            waited_ms: u64::try_from(spent.as_millis()).unwrap_or(u64::MAX),
        }),
    }
}

/// The ledger's length, `0` when there is none. One `lstat`; a ledger that is
/// not a regular file is refused.
fn ledger_len(root: &Path, feed: Vendor) -> Result<u64, QuarantineError> {
    let path = ledger_path(root, feed);
    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_file() => Ok(meta.len()),
        Ok(_) => Err(QuarantineError::Io {
            path,
            operation: "inspect the quarantine ledger (not a regular file)",
            kind: io::ErrorKind::InvalidInput,
        }),
        Err(why) if why.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(why) => io_at(Err(why), &path, "inspect the quarantine ledger"),
    }
}

/// Whether a writer holds the feed's quarantine lock at this instant: one
/// shared `flock(2)` asked without waiting, and let go at once. A lock that
/// cannot be opened or asked is no writer: the caller then refuses.
fn writer_active(root: &Path, feed: Vendor) -> bool {
    let path = lock_path(root, feed);
    let Ok(file) = crate::safe_open::open_read(&path) else {
        return false;
    };
    match Flock::try_lock_shared(file, path) {
        Ok(held) => {
            let _released = held.release();
            false
        }
        Err(std::fs::TryLockError::WouldBlock) => true,
        Err(std::fs::TryLockError::Error(_)) => false,
    }
}

fn refusal_kind(why: &crate::safe_open::ReadRefusal) -> io::ErrorKind {
    match why {
        crate::safe_open::ReadRefusal::Io { source, .. } => source.kind(),
        _ => io::ErrorKind::InvalidInput,
    }
}

fn find_in_run(
    run: &[u8],
    home: u64,
    hash: &[u8; 16],
    month: u32,
    index: &Path,
) -> Result<Probe, QuarantineError> {
    for (offset, bytes) in run.chunks_exact(SLOT).enumerate() {
        let slot = home + offset as u64;
        match decode_slot(bytes, slot) {
            Slot::Free => return Ok(Probe::Clear),
            Slot::Damaged => {
                return Err(QuarantineError::Damaged {
                    path: index.to_path_buf(),
                    slot,
                });
            }
            Slot::Held {
                hash: held,
                month: at,
                entry,
            } if held == *hash && at == month => {
                return entry
                    .map(Probe::Quarantined)
                    .ok_or(QuarantineError::Damaged {
                        path: index.to_path_buf(),
                        slot,
                    });
            }
            Slot::Held { .. } => {}
        }
    }
    Ok(Probe::Clear)
}

/// The whole index read once, for a body that lists many series (`/store.json`,
/// `/instruments.json`). One read of [`INDEX_LEN`] bytes; each lookup after
/// it is one hash-map probe.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    months: HashMap<([u8; 16], u32), Entry>,
}

impl Snapshot {
    /// Reads the feed's index whole. A feed never quarantined is an empty
    /// snapshot.
    ///
    /// # Errors
    ///
    /// As [`probe`], for the header and any slot of the index.
    pub fn load(root: &Path, feed: Vendor) -> Result<Self, QuarantineError> {
        let index = index_path(root, feed);
        // ONLY A LEVEL IMAGE, as `probe_at` (`level_read`).
        let image = level_read(root, feed, &index, || {
            let Some(image) = read_image(&index)? else {
                return Ok(None);
            };
            let built_from = described(&image).ok_or_else(|| QuarantineError::HeaderDamaged {
                path: index.clone(),
            })?;
            Ok(Some((image, built_from)))
        })?;
        let Some(image) = image else {
            return Ok(Self::default());
        };
        let mut months = HashMap::new();
        for (slot, bytes) in image
            .get(HEADER..)
            .unwrap_or(&[])
            .chunks_exact(SLOT)
            .enumerate()
        {
            match decode_slot(bytes, slot as u64) {
                Slot::Free => {}
                Slot::Damaged | Slot::Held { entry: None, .. } => {
                    return Err(QuarantineError::Damaged {
                        path: index,
                        slot: slot as u64,
                    });
                }
                Slot::Held {
                    hash,
                    month,
                    entry: Some(entry),
                } => {
                    months.insert((hash, month), entry);
                }
            }
        }
        Ok(Self { months })
    }

    /// The month's quarantine, if any.
    #[must_use]
    pub fn get(&self, path: StorePath<'_>) -> Option<Entry> {
        self.get_at(&series_key(path), path.month())
    }

    /// [`Self::get`] by series key and month.
    #[must_use]
    pub fn get_at(&self, key: &str, month: YearMonth) -> Option<Entry> {
        self.months
            .get(&(hash_of(key, month), month_code(month)))
            .copied()
    }

    /// Months quarantined in the snapshot.
    #[must_use]
    pub fn len(&self) -> usize {
        self.months.len()
    }

    /// Whether nothing is quarantined.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.months.is_empty()
    }
}

/// The index's bytes, `None` when there is no index, refused when torn.
fn read_image(index: &Path) -> Result<Option<Vec<u8>>, QuarantineError> {
    let (mut file, len) = match crate::safe_open::open_read_len(index) {
        Ok(found) => found,
        Err(why) if why.is_missing() => return Ok(None),
        Err(why) => {
            return Err(QuarantineError::Io {
                path: index.to_path_buf(),
                operation: "open the quarantine index",
                kind: refusal_kind(&why),
            });
        }
    };
    if len != INDEX_LEN {
        return Err(QuarantineError::Torn {
            path: index.to_path_buf(),
            len,
        });
    }
    let mut image = Vec::with_capacity(INDEX_BYTES);
    io_at(
        file.read_to_end(&mut image),
        index,
        "read the quarantine index",
    )?;
    if image.len() as u64 != INDEX_LEN {
        return Err(QuarantineError::Torn {
            path: index.to_path_buf(),
            len: image.len() as u64,
        });
    }
    Ok(Some(image))
}

/// The BLAKE3 of `bars`, as the ledger records the bars it names.
#[must_use]
pub fn digest_of(bars: &[Bar]) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(bars.len() * 56);
    for bar in bars {
        for value in [
            bar.ts_micros,
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume,
            bar.open_interest,
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    brutex_core::blake3::hash(&bytes)
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            for nibble in [byte >> 4, byte & 15] {
                out.push(char::from_digit(u32::from(nibble), 16).unwrap_or('0'));
            }
            out
        })
}

/// One act, as [`record`] writes it.
#[derive(Debug, Clone, Copy)]
pub struct Record<'a> {
    /// The month.
    pub path: StorePath<'a>,
    /// What is recorded.
    pub scope: Scope,
    /// Why.
    pub cause: Cause,
    /// The named bars' stamps, ascending; empty for a whole month.
    pub stamps: &'a [i64],
    /// Bars named.
    pub bars: u64,
    /// The revision served without them; `0` when none.
    pub revision: u32,
    /// [`digest_of`] the named bars as held.
    pub source_digest: [u8; 32],
    /// Who did it: the verb or the route.
    pub who: &'a str,
    /// Why, in the operator's words.
    pub reason: &'a str,
}

/// What [`record`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recorded {
    /// The byte offset of the ledger line written.
    pub ledger_at: u64,
    /// A torn last line was ended before this one.
    pub ended_torn: bool,
    /// The index was rebuilt from the ledger first (it was missing, torn,
    /// damaged, of another version, or behind or ahead of its ledger).
    pub rebuilt: bool,
}

/// Checks a reason: 1 to [`MAX_REASON`] bytes, no tab, no line break.
///
/// # Errors
///
/// [`QuarantineError::Reason`] naming which rule.
pub fn check_reason(reason: &str) -> Result<(), QuarantineError> {
    if reason.trim().is_empty() {
        return Err(QuarantineError::Reason(
            "a quarantine names its reason; none was given".to_owned(),
        ));
    }
    if reason.len() > MAX_REASON {
        return Err(QuarantineError::Reason(format!(
            "the reason is {} bytes, past the {MAX_REASON} a record carries",
            reason.len()
        )));
    }
    if reason.contains(['\t', '\n', '\r']) {
        return Err(QuarantineError::Reason(
            "the reason holds a tab or a line break".to_owned(),
        ));
    }
    Ok(())
}

/// Appends `act` to the ledger FIRST, then publishes its slot in a new index
/// image (temporary file, sync, rename, directory sync) that describes the
/// whole ledger. One writer at a time. An image behind its ledger (or torn,
/// or damaged) is rebuilt from the ledger first; a [`Scope::Dropping`] intent
/// is indexed as a drop of no bars.
///
/// # Errors
///
/// [`QuarantineError::Reason`], [`QuarantineError::Held`],
/// [`QuarantineError::Linked`], [`QuarantineError::LedgerDamaged`] and
/// [`QuarantineError::Full`] (each before anything is written), and host
/// refusals.
pub fn record(root: &Path, act: &Record<'_>) -> Result<Recorded, QuarantineError> {
    record_bounded(root, act, WRITER_PATIENCE)
}

fn record_bounded(
    root: &Path,
    act: &Record<'_>,
    bound: Duration,
) -> Result<Recorded, QuarantineError> {
    check_act(act)?;
    let feed = act.path.vendor();
    let lock = writer_lock(root, feed, bound)?;
    let done = record_locked(root, act);
    let released = release(root, feed, lock);
    let done = done?;
    released?;
    note_recorded(act, &done);
    Ok(done)
}

/// [`record`] by a caller that already holds the feed's writer lock.
fn record_in(root: &Path, act: &Record<'_>) -> Result<Recorded, QuarantineError> {
    check_act(act)?;
    let done = record_locked(root, act)?;
    note_recorded(act, &done);
    Ok(done)
}

/// An act's reason, writer's name and stamps: refused before anything is
/// written.
fn check_act(act: &Record<'_>) -> Result<(), QuarantineError> {
    check_reason(act.reason)?;
    if act.who.contains(['\t', '\n', '\r']) || act.who.is_empty() {
        return Err(QuarantineError::Reason(
            "the writer's name is empty or holds a tab or a line break".to_owned(),
        ));
    }
    if !ascending(act.stamps) {
        return Err(QuarantineError::Reason(
            "the named stamps are not strictly ascending".to_owned(),
        ));
    }
    Ok(())
}

/// Strictly ascending: what every reader of a line's stamps searches by.
fn ascending(stamps: &[i64]) -> bool {
    stamps
        .windows(2)
        .all(|pair| matches!(pair, [a, b] if a < b))
}

/// Lets the writer lock go, naming a release the host refused.
fn release(root: &Path, feed: Vendor, lock: Flock<File>) -> Result<(), QuarantineError> {
    lock.release().map_err(|_| QuarantineError::Io {
        path: lock_path(root, feed),
        operation: "release the quarantine lock",
        kind: io::ErrorKind::Other,
    })
}

fn note_recorded(act: &Record<'_>, done: &Recorded) {
    let key = series_key(act.path);
    let month = act.path.month().to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info("store.quarantine", "quarantine recorded")
            .with("feed", telemetry::Value::Str(act.path.vendor().as_str()))
            .with("key", telemetry::Value::Str(&key))
            .with("month", telemetry::Value::Str(&month))
            .with("scope", telemetry::Value::Str(act.scope.code()))
            .with("cause", telemetry::Value::Str(act.cause.code()))
            .with("bars", telemetry::Value::Uint(act.bars))
            .with("revision", telemetry::Value::Uint(u64::from(act.revision)))
            .with("digest", telemetry::Value::Str(&hex(&act.source_digest)))
            .with("who", telemetry::Value::Str(act.who))
            .with("reason", telemetry::Value::Str(act.reason))
            .with("ledger_at", telemetry::Value::Uint(done.ledger_at))
            .with("ended_torn", telemetry::Value::Bool(done.ended_torn))
            .with("rebuilt", telemetry::Value::Bool(done.rebuilt)),
    );
}

/// One event for an index rebuilt from its ledger, and why it had to be.
fn note_rebuilt(feed: Vendor, why: &str, done: &Rebuilt) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(
            "store.quarantine",
            "quarantine index rebuilt from its ledger",
        )
        .with("feed", telemetry::Value::Str(feed.as_str()))
        .with("why", telemetry::Value::Str(why))
        .with("lines", telemetry::Value::Uint(done.lines as u64))
        .with("months", telemetry::Value::Uint(done.months as u64))
        .with("ended_torn", telemetry::Value::Uint(done.ended as u64))
        .with("torn_tail", telemetry::Value::Bool(done.torn_tail)),
    );
}

/// One event for a ledger that holds damaged lines: every refusal that reads
/// it says so on `/logs`, by offset.
fn note_damaged(feed: Vendor, refused: &QuarantineError) {
    if let QuarantineError::LedgerDamaged { offsets, .. } = refused {
        let first = offsets.first().copied().unwrap_or(0);
        let text = refused.to_string();
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::error("store.quarantine", "quarantine ledger damaged")
                .with("feed", telemetry::Value::Str(feed.as_str()))
                .with(
                    "damaged_lines",
                    telemetry::Value::Uint(offsets.len() as u64),
                )
                .with("first_at", telemetry::Value::Uint(first))
                .with("why", telemetry::Value::Str(&text)),
        );
    }
}

fn audit_dir(root: &Path) -> Result<PathBuf, QuarantineError> {
    let dir = root.join(DIR);
    match fs::symlink_metadata(&dir) {
        Ok(meta) if meta.file_type().is_symlink() => Err(QuarantineError::Linked(dir)),
        Ok(_) => Ok(dir),
        Err(why) if why.kind() == io::ErrorKind::NotFound => match fs::create_dir(&dir) {
            Err(why) if why.kind() != io::ErrorKind::AlreadyExists => {
                io_at(Err(why), &dir, "create the audit directory")
            }
            _ => Ok(dir),
        },
        Err(why) => io_at(Err(why), &dir, "inspect the audit directory"),
    }
}

fn writer_lock(root: &Path, feed: Vendor, bound: Duration) -> Result<Flock<File>, QuarantineError> {
    audit_dir(root)?;
    let path = lock_path(root, feed);
    let file = io_at(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path),
        &path,
        "open the quarantine lock",
    )?;
    match Flock::try_lock_patiently(file, path.clone(), bound, "quarantine record") {
        Ok(held) => Ok(held),
        Err(Impatient::Held { holder, .. }) => Err(QuarantineError::Held {
            holder: holder.unwrap_or_else(|| crate::flock::holder_note(&path)),
            path,
        }),
        Err(other) => Err(QuarantineError::Io {
            path,
            operation: "lock the quarantine",
            kind: impatient_kind(&other),
        }),
    }
}

fn impatient_kind(why: &Impatient) -> io::ErrorKind {
    match why {
        Impatient::Host(error) => error.kind(),
        Impatient::Held { .. } => io::ErrorKind::WouldBlock,
    }
}

/// The act, under the writer lock. EVERY REFUSAL BEFORE THE FIRST BYTE: the
/// image made level with the ledger (rebuilt from it when it is behind, torn
/// or damaged, which refuses a damaged ledger line by name), the key's slot
/// found (a full bucket refused) and the line placed. Then the ledger line in
/// one write, synced; then the image naming it and describing the whole
/// ledger. Only a host refusal of that last publish leaves the line
/// unindexed, and every reader then names it ([`QuarantineError::Stale`])
/// until the next writer, which rebuilds first.
fn record_locked(root: &Path, act: &Record<'_>) -> Result<Recorded, QuarantineError> {
    let feed = act.path.vendor();
    let key = series_key(act.path);
    let ledger = ledger_path(root, feed);
    let index = index_path(root, feed);
    let mut file = open_ledger(&ledger)?;
    let tail = tail_of(&file, &ledger)?;
    let (mut image, rebuilt) = level_image(root, feed, tail.len)?;
    let hash = hash_of(&key, act.path.month());
    let month = month_code(act.path.month());
    let (slot, before) = find_slot(&image, &hash, month, &index)?;
    let line = ledger_line(&key, act, now_micros());
    let ledger_at = write_tail(&mut file, &ledger, &tail, &line)?;
    let (scope, bars) = indexed(act.scope, act.bars);
    let entry = Entry {
        scope,
        cause: act.cause,
        bars,
        ledger_at,
        revision: act.revision,
        written_at: now_micros(),
    };
    put(&mut image, slot, &hash, month, before, entry);
    set_described(&mut image, ledger_at + line.len() as u64);
    publish_image(root, feed, &image)?;
    Ok(Recorded {
        ledger_at,
        ended_torn: tail.ends_torn,
        rebuilt,
    })
}

/// How a ledger line is indexed: an intent (`dropping`) is a drop of no
/// bars, so every reader knows from the intent on that the month is named,
/// and the `dropped` line that finishes it adds the bars.
const fn indexed(scope: Scope, bars: u64) -> (Scope, u64) {
    match scope {
        Scope::Dropping => (Scope::Dropped, 0),
        other => (other, bars),
    }
}

/// The ledger opened to append, created when it is new.
fn open_ledger(path: &Path) -> Result<File, QuarantineError> {
    io_at(
        OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path),
        path,
        "open the quarantine ledger",
    )
}

/// Where the next line lands, read under the writer lock: the ledger's
/// length, and what is written before the line in the same write.
struct Tail {
    /// The ledger's length.
    len: u64,
    /// [`TORN_END`] after a torn last line, then the head when the ledger is
    /// new or its head itself was torn.
    prefix: String,
    /// The last line was torn and is ended now.
    ends_torn: bool,
}

/// The ledger's [`Tail`]. A ledger that does not start with its head (and
/// is not a torn head: a strict prefix of it) is damaged at byte 0 and
/// refused: nothing is written past it.
fn tail_of(file: &File, path: &Path) -> Result<Tail, QuarantineError> {
    let len = io_at(file.metadata(), path, "measure the quarantine ledger")?.len();
    let mut prefix = String::new();
    let mut ends_torn = false;
    if len > 0 {
        let mut last = [0u8; 1];
        io_at(
            file.read_exact_at(&mut last, len - 1),
            path,
            "read the ledger tail",
        )?;
        if last != *b"\n" {
            prefix.push_str(TORN_END);
            ends_torn = true;
        }
    }
    let head = format!("{LEDGER_HEAD}\n");
    let head_len = head.len() as u64;
    let mut first = vec![0u8; usize::try_from(len.min(head_len)).unwrap_or(0)];
    io_at(
        file.read_exact_at(&mut first, 0),
        path,
        "read the ledger head",
    )?;
    if first.as_slice() != head.as_bytes() {
        let torn_head = len < head_len && head.as_bytes().starts_with(&first);
        if !torn_head {
            return Err(QuarantineError::LedgerDamaged {
                path: path.to_path_buf(),
                offsets: vec![0],
            });
        }
        prefix.push_str(&head);
    }
    Ok(Tail {
        len,
        prefix,
        ends_torn,
    })
}

/// Writes `tail`'s prefix and `line` in ONE write, syncs, and answers the
/// line's offset.
fn write_tail(
    file: &mut File,
    path: &Path,
    tail: &Tail,
    line: &str,
) -> Result<u64, QuarantineError> {
    let mut bytes = String::with_capacity(tail.prefix.len() + line.len());
    bytes.push_str(&tail.prefix);
    bytes.push_str(line);
    io_at(
        file.write_all(bytes.as_bytes()),
        path,
        "append to the quarantine ledger",
    )?;
    io_at(file.sync_all(), path, "sync the quarantine ledger")?;
    Ok(tail.len + tail.prefix.len() as u64)
}

fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_micros()).unwrap_or(i64::MAX)
        })
}

fn ledger_line(key: &str, act: &Record<'_>, written_at: i64) -> String {
    let stamps = if act.stamps.is_empty() {
        "-".to_owned()
    } else {
        act.stamps
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(",")
    };
    let body = format!(
        "{written_at}\t{}\t{key}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{stamps}",
        act.path.vendor().as_str(),
        act.path.month(),
        act.scope.code(),
        act.cause.code(),
        act.bars,
        act.revision,
        hex(&act.source_digest),
        act.who,
        act.reason,
    );
    let crc = crc32c(body.as_bytes());
    format!("{body}\t{crc:08x}\n")
}

/// Appends one line with no index publish: what a writer killed between its
/// ledger fsync and its index rename leaves. The tests' seam only.
#[cfg(test)]
fn append_line(path: &Path, line: &str) -> Result<(u64, bool), QuarantineError> {
    let mut file = open_ledger(path)?;
    let tail = tail_of(&file, path)?;
    let at = write_tail(&mut file, path, &tail, line)?;
    Ok((at, tail.ends_torn))
}

/// One valid ledger line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Its byte offset.
    pub at: u64,
    /// When it was written.
    pub written_at: i64,
    /// The series key.
    pub key: String,
    /// The month, `yyyy-mm`.
    pub month: String,
    /// What it records.
    pub scope: Scope,
    /// Why.
    pub cause: Cause,
    /// Bars named.
    pub bars: u64,
    /// The revision served without them.
    pub revision: u32,
    /// The named bars' digest, hex.
    pub digest: String,
    /// Who.
    pub who: String,
    /// Why, in words.
    pub reason: String,
    /// The named stamps; empty for a whole month.
    pub stamps: Vec<i64>,
}

/// The ledger read whole: its valid lines, and how many were not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ledger {
    /// Every line that parses and checks, in order.
    pub lines: Vec<Line>,
    /// A last fragment with no newline: a torn append, never read as a line.
    pub torn_tail: bool,
    /// Complete lines that fail their checksum or do not parse, by offset:
    /// DAMAGE, which every reader of the ledger refuses by name
    /// ([`QuarantineError::LedgerDamaged`]).
    pub damaged: Vec<u64>,
    /// Torn appends a later writer ended with [`TORN_END`], by offset: never
    /// lines and never damage.
    pub ended: Vec<u64>,
    /// The bytes read: what an image built from these lines describes.
    pub len: u64,
}

/// Reads the feed's ledger. O(ledger), the write path's and the verb's.
///
/// # Errors
///
/// A host refusal.
pub fn read_ledger(root: &Path, feed: Vendor) -> Result<Ledger, QuarantineError> {
    let path = ledger_path(root, feed);
    let bytes = match crate::safe_open::open_read(&path) {
        Ok(mut file) => {
            let mut bytes = Vec::new();
            io_at(
                file.read_to_end(&mut bytes),
                &path,
                "read the quarantine ledger",
            )?;
            bytes
        }
        Err(why) if why.is_missing() => return Ok(Ledger::default()),
        Err(why) => {
            return Err(QuarantineError::Io {
                path,
                operation: "open the quarantine ledger",
                kind: refusal_kind(&why),
            });
        }
    };
    Ok(parse_ledger(&bytes))
}

/// The ledger's lines. EVERY complete line is one of four things: an ended
/// torn append (ends `\ttorn`), the head (the first line that is not one),
/// an act that parses and checks, or damage — a line that is not valid
/// UTF-8, is empty, fails its checksum or does not parse is never skipped
/// without a word.
fn parse_ledger(bytes: &[u8]) -> Ledger {
    let mut ledger = Ledger {
        len: bytes.len() as u64,
        ..Ledger::default()
    };
    let mut at = 0u64;
    let mut rest = bytes;
    let mut headed = false;
    while !rest.is_empty() {
        let Some(end) = rest.iter().position(|byte| *byte == b'\n') else {
            ledger.torn_tail = true;
            break;
        };
        let (raw, after) = rest.split_at(end);
        if raw.ends_with(TORN_MARK) {
            ledger.ended.push(at);
        } else if !headed {
            headed = true;
            if raw != LEDGER_HEAD.as_bytes() {
                ledger.damaged.push(at);
            }
        } else {
            match std::str::from_utf8(raw)
                .ok()
                .and_then(|text| parse_line(text, at))
            {
                Some(line) => ledger.lines.push(line),
                None => ledger.damaged.push(at),
            }
        }
        at += end as u64 + 1;
        rest = after.get(1..).unwrap_or(&[]);
    }
    ledger
}

fn parse_line(text: &str, at: u64) -> Option<Line> {
    let (body, crc) = text.rsplit_once('\t')?;
    if crc.len() != 8 || u32::from_str_radix(crc, 16).ok()? != crc32c(body.as_bytes()) {
        return None;
    }
    let fields: Vec<&str> = body.split('\t').collect();
    let [
        written_at,
        _feed,
        key,
        month,
        scope,
        cause,
        bars,
        revision,
        digest,
        who,
        reason,
        stamps,
    ] = fields.as_slice()
    else {
        return None;
    };
    let stamps = if *stamps == "-" {
        Vec::new()
    } else {
        stamps
            .split(',')
            .map(str::parse::<i64>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?
    };
    // A month that is no month, or stamps out of order, is no act a writer
    // here wrote: damage, never a line read on a guess.
    parse_month(month)?;
    if !ascending(&stamps) {
        return None;
    }
    Some(Line {
        at,
        written_at: written_at.parse().ok()?,
        key: (*key).to_owned(),
        month: (*month).to_owned(),
        scope: Scope::parse(scope)?,
        cause: Cause::parse(cause)?,
        bars: bars.parse().ok()?,
        revision: revision.parse().ok()?,
        digest: (*digest).to_owned(),
        who: (*who).to_owned(),
        reason: (*reason).to_owned(),
        stamps,
    })
}

/// Every stamp the ledger records as DROPPED from `path`'s month (lines of
/// scope `dropped`), ascending and unique. O(ledger).
///
/// # Errors
///
/// As [`read_ledger`], and [`QuarantineError::LedgerDamaged`]: a damaged
/// line may have named stamps of this month.
pub fn ledger_stamps(root: &Path, path: StorePath<'_>) -> Result<Vec<i64>, QuarantineError> {
    stamps_of(root, path, &[Scope::Dropped])
}

/// Every stamp a line of one of `scopes` names in `path`'s month, ascending
/// and unique, from a ledger with no damaged line. O(ledger).
fn stamps_of(
    root: &Path,
    path: StorePath<'_>,
    scopes: &[Scope],
) -> Result<Vec<i64>, QuarantineError> {
    let ledger = checked_ledger(root, path.vendor())?;
    let key = series_key(path);
    let month = path.month().to_string();
    let mut stamps: Vec<i64> = ledger
        .lines
        .iter()
        .filter(|line| scopes.contains(&line.scope) && line.key == key && line.month == month)
        .flat_map(|line| line.stamps.iter().copied())
        .collect();
    stamps.sort_unstable();
    stamps.dedup();
    Ok(stamps)
}

/// The feed's ledger, refused by name when any line of it is damaged (an
/// event on `/logs` says so too).
fn checked_ledger(root: &Path, feed: Vendor) -> Result<Ledger, QuarantineError> {
    let ledger = read_ledger(root, feed)?;
    refuse_damage(&ledger, root, feed)?;
    Ok(ledger)
}

/// [`QuarantineError::LedgerDamaged`] for a ledger with a damaged line.
fn refuse_damage(ledger: &Ledger, root: &Path, feed: Vendor) -> Result<(), QuarantineError> {
    if ledger.damaged.is_empty() {
        return Ok(());
    }
    let refused = QuarantineError::LedgerDamaged {
        path: ledger_path(root, feed),
        offsets: ledger.damaged.clone(),
    };
    note_damaged(feed, &refused);
    Err(refused)
}

/// What the quarantine holds of one month, for a door that ADDS or RESTATES
/// bars ([`crate::repair::repair_month`], [`crate::repair::restate_month`]):
/// a stamp the quarantine set aside is never served again by a re-pull.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetAside {
    /// No quarantine names the month: one probe, no ledger read.
    Nothing,
    /// The month is recorded whole: nothing of it is served, and no door
    /// adds to it.
    Whole(Entry),
    /// Every stamp a drop of the month names, finished or intended,
    /// ascending and unique.
    Stamps(Vec<i64>),
}

/// The month's [`SetAside`]. O(1) for a month no quarantine names (one
/// [`probe`]); O(ledger) for one a drop names.
///
/// # Errors
///
/// [`probe`]'s refusals (an index not trusted is never "nothing"), and
/// [`read_ledger`]'s and [`QuarantineError::LedgerDamaged`] for a month a
/// drop names.
pub fn set_aside(root: &Path, path: StorePath<'_>) -> Result<SetAside, QuarantineError> {
    match probe(root, path)? {
        Probe::Clear => Ok(SetAside::Nothing),
        Probe::Quarantined(entry) if entry.scope == Scope::WholeMonth => Ok(SetAside::Whole(entry)),
        Probe::Quarantined(_) => {
            stamps_of(root, path, &[Scope::Dropped, Scope::Dropping]).map(SetAside::Stamps)
        }
    }
}

fn empty_image() -> Vec<u8> {
    let mut image = vec![0u8; INDEX_BYTES];
    let fields: [(usize, &[u8]); 5] = [
        (0, &MAGIC),
        (8, &INDEX_VERSION.to_le_bytes()),
        (12, &SLOT_U32.to_le_bytes()),
        (16, &CAPACITY.to_le_bytes()),
        (24, &BUCKET.to_le_bytes()),
    ];
    for (at, value) in fields {
        if let Some(dst) = image.get_mut(at..at + value.len()) {
            dst.copy_from_slice(value);
        }
    }
    seal_header(&mut image);
    image
}

fn seal_header(image: &mut [u8]) {
    let crc = crc32c(image.get(..60).unwrap_or(&[]));
    if let Some(dst) = image.get_mut(60..64) {
        dst.copy_from_slice(&crc.to_le_bytes());
    }
}

/// Whether an image's header and every slot check.
fn image_trusted(image: &[u8]) -> bool {
    described(image).is_some()
        && image
            .get(HEADER..)
            .unwrap_or(&[])
            .chunks_exact(SLOT)
            .enumerate()
            .all(|(slot, bytes)| {
                matches!(
                    decode_slot(bytes, slot as u64),
                    Slot::Free | Slot::Held { entry: Some(_), .. }
                )
            })
}

/// One month's entry after `newer` lands on `held`: the ONE rule the index
/// ([`put`], so [`rebuild`] too) and [`Summary::load`] share. A drop onto a
/// drop adds its bars (an intent adds none, and keeps the revision served);
/// a drop onto a month recorded whole keeps the whole record, because a
/// whole record means the reader serves none of the month and a later
/// partial drop must never re-serve the rest; anything else is the newer
/// line.
fn merged(held: Entry, newer: Entry) -> Entry {
    match (held.scope, newer.scope) {
        (Scope::Dropped, Scope::Dropped) => Entry {
            bars: held.bars.saturating_add(newer.bars),
            revision: if newer.revision == 0 {
                held.revision
            } else {
                newer.revision
            },
            ..newer
        },
        (Scope::WholeMonth, Scope::Dropped) => held,
        _ => newer,
    }
}

/// The byte offset of `slot` in an image.
fn slot_at(slot: u64) -> usize {
    usize::try_from(HEADER_LEN + slot * SLOT_LEN).unwrap_or(usize::MAX)
}

/// The slot `hash`'s month lives in, or the first free one of its bucket,
/// and what it holds there. O([`BUCKET`]).
///
/// # Errors
///
/// [`QuarantineError::Full`] when every slot of the bucket holds another
/// month, [`QuarantineError::Damaged`] for one that does not check.
fn find_slot(
    image: &[u8],
    hash: &[u8; 16],
    month: u32,
    index: &Path,
) -> Result<(u64, Option<Entry>), QuarantineError> {
    let home = home_of(hash);
    for slot in home..home + BUCKET {
        let at = slot_at(slot);
        match decode_slot(image.get(at..at + SLOT).unwrap_or(&[]), slot) {
            Slot::Free => return Ok((slot, None)),
            Slot::Held {
                hash: held,
                month: was,
                entry: Some(before),
            } if held == *hash && was == month => return Ok((slot, Some(before))),
            Slot::Held { entry: Some(_), .. } => {}
            Slot::Held { entry: None, .. } | Slot::Damaged => {
                return Err(QuarantineError::Damaged {
                    path: index.to_path_buf(),
                    slot,
                });
            }
        }
    }
    Err(QuarantineError::Full {
        path: index.to_path_buf(),
    })
}

/// Writes `entry`, merged onto what the slot held ([`merged`]), into `slot`,
/// counts a slot newly taken in the header, and seals it.
fn put(
    image: &mut [u8],
    slot: u64,
    hash: &[u8; 16],
    month: u32,
    before: Option<Entry>,
    entry: Entry,
) {
    let entry = before.map_or(entry, |held| merged(held, entry));
    let at = slot_at(slot);
    if let Some(dst) = image.get_mut(at..at + SLOT) {
        dst.copy_from_slice(&encode_slot(hash, month, &entry, slot));
    }
    if before.is_none() {
        let held = u64::from_le_bytes(field(image, 32)).saturating_add(1);
        if let Some(dst) = image.get_mut(32..40) {
            dst.copy_from_slice(&held.to_le_bytes());
        }
    }
    seal_header(image);
}

/// Records in the header the ledger bytes the image describes, and seals it.
fn set_described(image: &mut [u8], ledger_len: u64) {
    if let Some(dst) = image.get_mut(40..48) {
        dst.copy_from_slice(&ledger_len.to_le_bytes());
    }
    seal_header(image);
}

/// The index entry a ledger line makes ([`indexed`]).
fn entry_of(line: &Line) -> Entry {
    let (scope, bars) = indexed(line.scope, line.bars);
    Entry {
        scope,
        cause: line.cause,
        bars,
        ledger_at: line.at,
        revision: line.revision,
        written_at: line.written_at,
    }
}

/// The index image `ledger` describes: every line, in order, through the one
/// merge rule. A damaged line refuses the image by name: an image that left
/// out the act it recorded would answer "clear" for a month it names.
fn image_of(ledger: &Ledger, root: &Path, feed: Vendor) -> Result<Vec<u8>, QuarantineError> {
    refuse_damage(ledger, root, feed)?;
    let index = index_path(root, feed);
    let mut image = empty_image();
    for line in &ledger.lines {
        // `parse_line` refused every month that is no month.
        let Some(month) = parse_month(&line.month) else {
            continue;
        };
        let hash = hash_of(&line.key, month);
        let code = month_code(month);
        let (slot, before) = find_slot(&image, &hash, code, &index)?;
        put(&mut image, slot, &hash, code, before, entry_of(line));
    }
    set_described(&mut image, ledger.len);
    Ok(image)
}

/// Where the published image stands against a ledger of `ledger_len` bytes.
enum Level {
    /// It checks and describes exactly those bytes.
    Current(Vec<u8>),
    /// It must be rebuilt, and why.
    Behind(String),
}

fn level(root: &Path, feed: Vendor, ledger_len: u64) -> Result<Level, QuarantineError> {
    let index = index_path(root, feed);
    let why = match read_image(&index) {
        Ok(Some(image)) if image_trusted(&image) && described(&image) == Some(ledger_len) => {
            return Ok(Level::Current(image));
        }
        Ok(None) if ledger_len == 0 => return Ok(Level::Current(empty_image())),
        Ok(Some(image)) => match described(&image) {
            None => "its header did not check".to_owned(),
            Some(built) if built != ledger_len => {
                format!("it described {built} of the ledger's {ledger_len} bytes")
            }
            Some(_) => "a slot failed its checksum".to_owned(),
        },
        Ok(None) => format!("there was none and the ledger holds {ledger_len} bytes"),
        Err(QuarantineError::Torn { len, .. }) => format!("it was {len} bytes, not {INDEX_LEN}"),
        Err(other) => return Err(other),
    };
    Ok(Level::Behind(why))
}

/// The image level with a ledger of `ledger_len` bytes, for a writer under
/// the lock: the published one when current, else one rebuilt from the
/// ledger (`true`; a damaged line refused by name).
fn level_image(
    root: &Path,
    feed: Vendor,
    ledger_len: u64,
) -> Result<(Vec<u8>, bool), QuarantineError> {
    match level(root, feed, ledger_len)? {
        Level::Current(image) => Ok((image, false)),
        Level::Behind(why) => {
            let ledger = read_ledger(root, feed)?;
            let image = image_of(&ledger, root, feed)?;
            note_rebuilt(feed, &why, &rebuilt_of(&ledger, &image));
            Ok((image, true))
        }
    }
}

fn parse_month(text: &str) -> Option<YearMonth> {
    let (year, month) = text.split_once('-')?;
    YearMonth::new(year.parse().ok()?, month.parse().ok()?).ok()
}

fn publish_image(root: &Path, feed: Vendor, image: &[u8]) -> Result<(), QuarantineError> {
    let index = index_path(root, feed);
    let dir = audit_dir(root)?;
    let staging = dir.join(format!(
        ".{}.quarantine.{}.staging",
        feed.as_str(),
        std::process::id()
    ));
    let mut staged = crate::staging::create(&staging).map_err(|why| QuarantineError::Io {
        path: staging.clone(),
        operation: "create the index staging file",
        kind: staging_kind(&why),
    })?;
    io_at(
        staged.file.write_all(image),
        &staging,
        "write the index image",
    )?;
    io_at(staged.file.sync_all(), &staging, "sync the index image")?;
    io_at(
        fs::rename(&staging, &index),
        &index,
        "publish the index image",
    )?;
    io_at(
        File::open(&dir).and_then(|handle| handle.sync_all()),
        &dir,
        "sync the audit directory",
    )
}

fn staging_kind(why: &crate::staging::StagingError) -> io::ErrorKind {
    match why {
        crate::staging::StagingError::Io { source, .. } => source.kind(),
        _ => io::ErrorKind::AlreadyExists,
    }
}

/// What [`rebuild`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rebuilt {
    /// Valid ledger lines read.
    pub lines: usize,
    /// Months indexed.
    pub months: usize,
    /// The ledger ended in a torn line.
    pub torn_tail: bool,
    /// Torn appends a later writer ended ([`TORN_END`]): no act, no damage.
    pub ended: usize,
}

fn rebuilt_of(ledger: &Ledger, image: &[u8]) -> Rebuilt {
    Rebuilt {
        lines: ledger.lines.len(),
        months: usize::try_from(u64::from_le_bytes(field(image, 32))).unwrap_or(usize::MAX),
        torn_tail: ledger.torn_tail,
        ended: ledger.ended.len(),
    }
}

/// Rebuilds the feed's index from its ledger and publishes it: the repair
/// of a torn, damaged or stale index. Under the writer lock.
///
/// # Errors
///
/// As [`record`]; [`QuarantineError::LedgerDamaged`] publishes nothing, so
/// every reader keeps answering "not known".
pub fn rebuild(root: &Path, feed: Vendor) -> Result<Rebuilt, QuarantineError> {
    let lock = writer_lock(root, feed, WRITER_PATIENCE)?;
    let done = rebuild_locked(root, feed, "asked");
    let released = release(root, feed, lock);
    let done = done?;
    released?;
    Ok(done)
}

fn rebuild_locked(root: &Path, feed: Vendor, why: &str) -> Result<Rebuilt, QuarantineError> {
    let ledger = read_ledger(root, feed)?;
    let image = image_of(&ledger, root, feed)?;
    publish_image(root, feed, &image)?;
    let done = rebuilt_of(&ledger, &image);
    note_rebuilt(feed, why, &done);
    Ok(done)
}

/// Brings the feed's index level with its ledger under the writer lock:
/// `None` when the published image already describes the whole ledger, else
/// the rebuild (an act ledgered and not indexed, a torn, damaged or older
/// image). The verb calls it before its first act, so its output names a
/// rebuild; [`apply_month`] calls it too.
///
/// # Errors
///
/// As [`rebuild`].
pub fn catch_up(root: &Path, feed: Vendor) -> Result<Option<Rebuilt>, QuarantineError> {
    let lock = writer_lock(root, feed, WRITER_PATIENCE)?;
    let done = catch_up_locked(root, feed);
    let released = release(root, feed, lock);
    let done = done?;
    released?;
    Ok(done)
}

fn catch_up_locked(root: &Path, feed: Vendor) -> Result<Option<Rebuilt>, QuarantineError> {
    match level(root, feed, ledger_len(root, feed)?)? {
        Level::Current(_) => Ok(None),
        Level::Behind(why) => rebuild_locked(root, feed, &why).map(Some),
    }
}

/// One instrument's quarantine over every rung and month, as a listing row
/// carries it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Instrument {
    /// Distinct (rung, month) pairs set aside, whole or by bars.
    pub months: u64,
    /// Of them, months recorded whole.
    pub whole_months: u64,
    /// Bars named over all of them.
    pub bars: u64,
    /// Every cause, in code order.
    pub causes: Vec<Cause>,
}

/// The feed's quarantine by instrument and by (series, month), read from the
/// ledger ONCE per cached body (`/store.json`, `/instruments.json`); each
/// lookup after it is one hash probe. O(ledger) to build.
#[derive(Debug, Clone, Default)]
pub struct Summary {
    instruments: HashMap<String, Instrument>,
    months: HashMap<(String, String), Entry>,
}

impl Summary {
    /// Builds the summary from the feed's ledger: every line through the one
    /// merge rule the index uses ([`merged`]; an intent a drop of no bars),
    /// so a listing and a probe never say two things of one month. Each
    /// instrument's causes are every cause any of its lines named.
    ///
    /// # Errors
    ///
    /// As [`read_ledger`], and [`QuarantineError::LedgerDamaged`]: a listing
    /// that left out the act a damaged line recorded would under-count
    /// without a word, so the feed's rows say the code instead.
    pub fn load(root: &Path, feed: Vendor) -> Result<Self, QuarantineError> {
        let ledger = checked_ledger(root, feed)?;
        let mut months: HashMap<(String, String), Entry> = HashMap::new();
        let mut causes: HashMap<String, Vec<Cause>> = HashMap::new();
        for line in &ledger.lines {
            let entry = entry_of(line);
            months
                .entry((line.key.clone(), line.month.clone()))
                .and_modify(|held| *held = merged(*held, entry))
                .or_insert(entry);
            let named = causes.entry(instrument_of(&line.key)).or_default();
            if !named.contains(&line.cause) {
                named.push(line.cause);
                named.sort_unstable();
            }
        }
        let mut instruments: HashMap<String, Instrument> = HashMap::new();
        for ((key, _), entry) in &months {
            let instrument = instrument_of(key);
            let row = instruments.entry(instrument.clone()).or_default();
            row.months += 1;
            row.whole_months += u64::from(entry.scope == Scope::WholeMonth);
            row.bars = row.bars.saturating_add(entry.bars);
            if row.causes.is_empty() {
                row.causes = causes.get(&instrument).cloned().unwrap_or_default();
            }
        }
        Ok(Self {
            instruments,
            months,
        })
    }

    /// The instrument's quarantine: `exchange/segment/symbol`.
    #[must_use]
    pub fn instrument(&self, exchange: &str, segment: &str, symbol: &str) -> Option<&Instrument> {
        if self.instruments.is_empty() {
            return None;
        }
        self.instruments
            .get(&format!("{exchange}/{segment}/{symbol}"))
    }

    /// The month's quarantine.
    #[must_use]
    pub fn month(&self, path: StorePath<'_>) -> Option<Entry> {
        if self.months.is_empty() {
            return None;
        }
        self.months
            .get(&(series_key(path), path.month().to_string()))
            .copied()
    }

    /// Whether the feed has no quarantine at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.months.is_empty()
    }
}

/// The instrument a series key belongs to: `exchange/segment/symbol`.
fn instrument_of(key: &str) -> String {
    key.splitn(4, '/').take(3).collect::<Vec<_>>().join("/")
}

/// What [`apply_month`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// Nothing was written: every named bar is already set aside.
    Nothing {
        /// Why: `already_whole` or `already_dropped`.
        why: &'static str,
    },
    /// Every bar of the month is named: recorded whole, nothing published.
    WholeMonth {
        /// Bars the month holds.
        bars: u64,
        /// The ledger line.
        ledger_at: u64,
    },
    /// A generation without the named bars is served.
    Dropped {
        /// The door's account.
        drop: crate::repair::MonthDrop,
        /// The `dropped` ledger line.
        ledger_at: u64,
    },
    /// A drop a killed run ledgered and promoted, finished from its intent:
    /// the `dropped` line and the index were written now.
    Resumed {
        /// Bars the intent named.
        bars: u64,
        /// The `dropped` ledger line.
        ledger_at: u64,
    },
}

/// Every refusal of [`apply_month`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    /// The door or the month refused.
    Repair(crate::repair::RepairError),
    /// The ledger or the index refused.
    Quarantine(QuarantineError),
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Repair(why) => why.fmt(f),
            Self::Quarantine(why) => why.fmt(f),
        }
    }
}

impl std::error::Error for ApplyError {}

impl From<QuarantineError> for ApplyError {
    fn from(value: QuarantineError) -> Self {
        Self::Quarantine(value)
    }
}

impl From<crate::repair::RepairError> for ApplyError {
    fn from(value: crate::repair::RepairError) -> Self {
        Self::Repair(value)
    }
}

impl From<crate::file::StoreError> for ApplyError {
    fn from(value: crate::file::StoreError) -> Self {
        Self::Repair(value.into())
    }
}

/// Who and why, for [`apply_month`].
#[derive(Debug, Clone, Copy)]
pub struct Why<'a> {
    /// The cause the plan measured.
    pub cause: Cause,
    /// The verb or route acting.
    pub who: &'a str,
    /// The operator's reason.
    pub reason: &'a str,
}

/// What one apply reads of a month: its bars at the named stamps, how many
/// it holds, and the intents a killed run owes it.
struct Month {
    /// The served month's bars at named stamps, ascending.
    held_named: Vec<Bar>,
    /// Bars the served month holds.
    n_valid: u64,
    /// Intents a killed run OWES: a `dropping` line no `dropped` line names
    /// a stamp of, whose every stamp the served month no longer holds (the
    /// door promoted, the run died before its `dropped` line). Oldest first.
    owed: Vec<Line>,
}

/// Judges `named` against the month and its ledger lines. Refuses a damaged
/// ledger ([`QuarantineError::LedgerDamaged`]) and a named stamp the month
/// does not hold that no ledger line set aside
/// ([`crate::repair::RepairError::UnheldStamp`]); writes nothing. One pass
/// over the month for the named stamps and every unfinished intent's, one
/// over the ledger: O(held + named + ledger).
fn judge(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    named: &[i64],
) -> Result<Month, ApplyError> {
    let ledger = checked_ledger(root, path.vendor())?;
    let key = series_key(path);
    let month = path.month().to_string();
    let mine: Vec<&Line> = ledger
        .lines
        .iter()
        .filter(|line| line.key == key && line.month == month)
        .collect();
    let named_by = |at: i64, scope: Scope| {
        mine.iter()
            .any(|line| line.scope == scope && line.stamps.binary_search(&at).is_ok())
    };
    let unfinished: Vec<&Line> = mine
        .iter()
        .copied()
        .filter(|line| {
            line.scope == Scope::Dropping
                && line.stamps.iter().all(|at| !named_by(*at, Scope::Dropped))
        })
        .collect();
    let mut wanted = named.to_vec();
    wanted.extend(
        unfinished
            .iter()
            .flat_map(|line| line.stamps.iter().copied()),
    );
    wanted.sort_unstable();
    wanted.dedup();
    let (held, n_valid) = held_at(root, path, symbol_id, &wanted)?;
    let holds = |at: i64| held.binary_search_by_key(&at, |bar| bar.ts_micros).is_ok();
    if let Some(at) = named.iter().find(|at| {
        !holds(**at) && !named_by(**at, Scope::Dropped) && !named_by(**at, Scope::Dropping)
    }) {
        return Err(crate::repair::RepairError::UnheldStamp(*at).into());
    }
    let held_named = held
        .iter()
        .filter(|bar| named.binary_search(&bar.ts_micros).is_ok())
        .copied()
        .collect();
    let owed = unfinished
        .iter()
        .filter(|line| line.stamps.iter().all(|at| !holds(*at)))
        .map(|line| (*line).clone())
        .collect();
    Ok(Month {
        held_named,
        n_valid,
        owed,
    })
}

/// The served month's bars at `wanted` stamps (ascending), and its committed
/// count. One pass, O(held + wanted).
fn held_at(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    wanted: &[i64],
) -> Result<(Vec<Bar>, u64), ApplyError> {
    let source = crate::file::BarFile::open_existing(root, path, symbol_id)?;
    let n_valid = source.header().n_valid;
    let mut held = Vec::with_capacity(wanted.len());
    let mut next = wanted.iter().copied().peekable();
    for index in 0..n_valid {
        if next.peek().is_none() {
            break;
        }
        let bar = source.read_record(index)?;
        while next.next_if(|at| *at < bar.ts_micros).is_some() {}
        if next.next_if(|at| *at == bar.ts_micros).is_some() {
            held.push(bar);
        }
    }
    Ok((held, n_valid))
}

/// Writes the `dropped` line and the index a killed run owed, from its
/// intent: the generation served now is the one it promoted. Under the lock.
fn finish_resume(
    root: &Path,
    path: StorePath<'_>,
    intent: &Line,
    why: Why<'_>,
) -> Result<u64, ApplyError> {
    let served = crate::generation::served(root, path)?;
    let mut digest = [0u8; 32];
    for (at, pair) in digest.iter_mut().zip(intent.digest.as_bytes().chunks(2)) {
        *at = std::str::from_utf8(pair)
            .ok()
            .and_then(|text| u8::from_str_radix(text, 16).ok())
            .unwrap_or(0);
    }
    let done = record_in(
        root,
        &Record {
            path,
            scope: Scope::Dropped,
            cause: intent.cause,
            stamps: &intent.stamps,
            bars: intent.bars,
            revision: served.ordinal(),
            source_digest: digest,
            who: why.who,
            reason: why.reason,
        },
    )?;
    Ok(done.ledger_at)
}

/// Sets aside the bars stamped `named` in `path`'s month: the whole month by
/// record when every held bar is named, otherwise a new generation without
/// them through [`crate::repair::drop_month`]. LEDGERED FIRST: the intent
/// line is synced before the door publishes, the `dropped` line and the
/// index after the promotion. Idempotent: a second apply of the same plan
/// writes nothing. ONE WRITER FOR THE WHOLE SEQUENCE: the feed's writer lock
/// is held from the first read to the last line, so two applies never
/// interleave (a second waits [`WRITER_PATIENCE`], then is refused naming the
/// holder). Before its own act it brings the index level with the ledger and
/// finishes every intent a killed run owes the month, whatever subset this
/// apply names ([`Applied::Resumed`] when that is all there was to do).
///
/// # Cost
///
/// O(held) to read the month and O(ledger) to read the ledger, twice (once
/// before the lock, so a refusal writes nothing, not even the lock file, and
/// once under it), plus the door's; the write path only.
///
/// # Errors
///
/// [`crate::repair::RepairError::UnheldStamp`] for a named stamp the month
/// does not hold and no ledger line set aside, and
/// [`QuarantineError::LedgerDamaged`] (each with nothing written); every
/// refusal of the door, of [`record`] and of the month's reads.
pub fn apply_month(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    named: &[i64],
    why: Why<'_>,
) -> Result<Applied, ApplyError> {
    check_reason(why.reason)?;
    let mut named = named.to_vec();
    named.sort_unstable();
    named.dedup();
    if named.is_empty() {
        return Err(crate::repair::RepairError::NothingNamed.into());
    }
    // A REFUSAL WRITES NOTHING: judged once, read only, before the lock.
    judge(root, path, symbol_id, &named)?;
    let feed = path.vendor();
    let lock = writer_lock(root, feed, WRITER_PATIENCE)?;
    let done = apply_locked(root, path, symbol_id, &named, why);
    let released = release(root, feed, lock);
    let done = done?;
    released?;
    Ok(done)
}

/// [`apply_month`] under the feed's writer lock.
fn apply_locked(
    root: &Path,
    path: StorePath<'_>,
    symbol_id: u32,
    named: &[i64],
    why: Why<'_>,
) -> Result<Applied, ApplyError> {
    catch_up_locked(root, path.vendor())?;
    let probed = probe(root, path)?;
    let month = judge(root, path, symbol_id, named)?;
    // WHAT A KILLED RUN OWES FIRST, oldest first, whatever this apply names:
    // its `dropped` line, so the index and the listings count its bars.
    let mut resumed: Option<Applied> = None;
    for intent in &month.owed {
        let ledger_at = finish_resume(root, path, intent, why)?;
        let bars = match resumed {
            Some(Applied::Resumed { bars, .. }) => bars.saturating_add(intent.bars),
            _ => intent.bars,
        };
        resumed = Some(Applied::Resumed { bars, ledger_at });
    }
    let nothing = |why: &'static str| resumed.clone().unwrap_or(Applied::Nothing { why });
    // A MONTH QUARANTINED WHOLE STAYS WHOLE, whatever subset a later plan
    // names: every bar it holds is already set aside, so publishing a
    // generation without some of them would only re-serve the rest. A stamp
    // neither held nor ledgered is still refused (`judge`); an intent a
    // killed run owes is still finished above (its `dropped` line never lifts
    // the whole record: see `merged`).
    if let Probe::Quarantined(entry) = probed
        && entry.scope == Scope::WholeMonth
    {
        return Ok(nothing("already_whole"));
    }
    if month.held_named.is_empty() {
        return Ok(nothing("already_dropped"));
    }
    let stamps: Vec<i64> = month.held_named.iter().map(|bar| bar.ts_micros).collect();
    let digest = digest_of(&month.held_named);
    if month.held_named.len() as u64 == month.n_valid {
        let done = record_in(
            root,
            &Record {
                path,
                scope: Scope::WholeMonth,
                cause: why.cause,
                stamps: &[],
                bars: month.n_valid,
                revision: 0,
                source_digest: digest,
                who: why.who,
                reason: why.reason,
            },
        )?;
        return Ok(Applied::WholeMonth {
            bars: month.n_valid,
            ledger_at: done.ledger_at,
        });
    }
    let act = |scope, revision| Record {
        path,
        scope,
        cause: why.cause,
        stamps: &stamps,
        bars: stamps.len() as u64,
        revision,
        source_digest: digest,
        who: why.who,
        reason: why.reason,
    };
    record_in(root, &act(Scope::Dropping, 0))?;
    let drop = crate::repair::drop_month(root, path, symbol_id, &stamps, why.reason)?;
    let done = record_in(root, &act(Scope::Dropped, drop.to.ordinal()))?;
    Ok(Applied::Dropped {
        drop,
        ledger_at: done.ledger_at,
    })
}

#[cfg(test)]
#[path = "quarantine_tests.rs"]
mod tests;
