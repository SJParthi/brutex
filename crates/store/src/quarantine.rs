//! The quarantine of named bars: an append-only ledger that is the truth, and
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
//! next append ends it with a newline first (`pull::alerts`' torn-tail rule
//! without the overwrite: no byte of the ledger is ever changed).
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
//! index of the wrong length (a torn copy, a truncation) answers
//! [`QuarantineError::Torn`] by name, never "clear", and is rebuilt from the
//! ledger by the next [`record`] or by [`rebuild`].
//!
//! # Writers
//!
//! One at a time, under `<root>/audit/<feed>.quarantine.lock` taken
//! patiently: a second writer waits, and when the bound runs out it is
//! refused naming the holder ([`QuarantineError::Held`]).
//!
//! # Cost
//!
//! [`probe`]: one `open` (ENOENT for a feed never quarantined), one `fstat`
//! and one 512-byte positional read — O(1). [`Snapshot::load`]: one read of
//! the whole index (524,800 bytes), once per cached body; each lookup after
//! it O(1). [`record`]: one ledger append and one index image — O(index) on
//! the write path, never on a read. [`rebuild`] and [`ledger_stamps`]: one
//! pass over the ledger, on the write path only.

use std::collections::HashMap;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _, Write as _};
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

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
/// How long a second writer waits for the first before it is refused.
pub const WRITER_PATIENCE: Duration = Duration::from_secs(10);
/// The index header's magic.
pub const MAGIC: [u8; 8] = *b"BRXQIX01";
/// The ledger's first line.
pub const LEDGER_HEAD: &str = "brutex quarantine ledger v1";

const SLOT: usize = 64;
const BUCKET_BYTES: usize = SLOT * 8;

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
    /// `quarantine_slot_damaged`: a slot on the key's run fails its CRC.
    Damaged {
        /// The index.
        path: PathBuf,
        /// The slot.
        slot: u64,
    },
    /// `quarantine_index_full`: every slot of the key's bucket holds another
    /// key. Nothing was indexed (the ledger line stands).
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
            Self::Torn { .. } => "quarantine_index_torn",
            Self::Damaged { .. } => "quarantine_slot_damaged",
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
            Self::Damaged { path, slot } => write!(
                f,
                "quarantine_slot_damaged: slot {slot} of {} fails its checksum; the quarantine is not known until it is rebuilt from its ledger",
                path.display()
            ),
            Self::Full { path } => write!(
                f,
                "quarantine_index_full: every slot of this key's bucket in {} holds another month; the ledger line stands, the index was not written",
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

fn io_at<T>(result: io::Result<T>, path: &Path, operation: &'static str) -> Result<T, QuarantineError> {
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
    root.join(DIR).join(format!("{}.quarantine.ledger", feed.as_str()))
}

fn lock_path(root: &Path, feed: Vendor) -> PathBuf {
    root.join(DIR).join(format!("{}.quarantine.lock", feed.as_str()))
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
    Held { hash: [u8; 16], month: u32, entry: Option<Entry> },
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

/// Answers whether `path`'s month is quarantined. ONE positional read.
///
/// # Errors
///
/// [`QuarantineError::Torn`] for an index of the wrong length,
/// [`QuarantineError::Damaged`] for a slot on the key's run that fails its
/// checksum, and a host refusal. Never [`Probe::Clear`] for an index it
/// cannot trust.
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
    let (file, len) = match crate::safe_open::open_read_len(&index) {
        Ok(found) => found,
        Err(why) if why.is_missing() => return Ok(Probe::Clear),
        Err(why) => {
            return Err(QuarantineError::Io {
                path: index,
                operation: "open the quarantine index",
                kind: refusal_kind(&why),
            });
        }
    };
    if len != INDEX_LEN {
        return Err(QuarantineError::Torn { path: index, len });
    }
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
                return entry.map(Probe::Quarantined).ok_or(QuarantineError::Damaged {
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
    /// As [`probe`], for any slot of the index.
    pub fn load(root: &Path, feed: Vendor) -> Result<Self, QuarantineError> {
        let index = index_path(root, feed);
        let image = match read_image(&index)? {
            Some(image) => image,
            None => return Ok(Self::default()),
        };
        let mut months = HashMap::new();
        for (slot, bytes) in image
            .get(HEADER_LEN as usize..)
            .unwrap_or(&[])
            .chunks_exact(SLOT)
            .enumerate()
        {
            match decode_slot(bytes, slot as u64) {
                Slot::Free => {}
                Slot::Damaged
                | Slot::Held { entry: None, .. } => {
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
    let mut image = Vec::with_capacity(INDEX_LEN as usize);
    io_at(file.read_to_end(&mut image), index, "read the quarantine index")?;
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
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
        out.push(char::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
        out.push(char::from(b"0123456789abcdef"[usize::from(byte & 15)]));
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
    /// The index was rebuilt from the ledger first (it was missing a valid
    /// image: torn, damaged).
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
/// image (temporary file, sync, rename, directory sync). One writer at a
/// time; a [`Scope::Dropping`] intent is ledgered and never indexed.
///
/// # Errors
///
/// [`QuarantineError::Reason`], [`QuarantineError::Held`],
/// [`QuarantineError::Linked`], [`QuarantineError::Full`] (the ledger line
/// stands, named) and host refusals.
pub fn record(root: &Path, act: &Record<'_>) -> Result<Recorded, QuarantineError> {
    record_bounded(root, act, WRITER_PATIENCE)
}

fn record_bounded(
    root: &Path,
    act: &Record<'_>,
    bound: Duration,
) -> Result<Recorded, QuarantineError> {
    check_reason(act.reason)?;
    if act.who.contains(['\t', '\n', '\r']) || act.who.is_empty() {
        return Err(QuarantineError::Reason(
            "the writer's name is empty or holds a tab or a line break".to_owned(),
        ));
    }
    let feed = act.path.vendor();
    let lock = writer_lock(root, feed, bound)?;
    let done = record_held(root, act);
    let released = lock.release();
    let done = done?;
    if released.is_err() {
        return Err(QuarantineError::Io {
            path: lock_path(root, feed),
            operation: "release the quarantine lock",
            kind: io::ErrorKind::Other,
        });
    }
    note_recorded(act, &done);
    Ok(done)
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
            .with("ledger_at", telemetry::Value::Uint(done.ledger_at)),
    );
}

fn audit_dir(root: &Path) -> Result<PathBuf, QuarantineError> {
    let dir = root.join(DIR);
    match fs::symlink_metadata(&dir) {
        Ok(meta) if meta.file_type().is_symlink() => Err(QuarantineError::Linked(dir)),
        Ok(_) => Ok(dir),
        Err(why) if why.kind() == io::ErrorKind::NotFound => {
            match fs::create_dir(&dir) {
                Err(why) if why.kind() != io::ErrorKind::AlreadyExists => {
                    io_at(Err(why), &dir, "create the audit directory")
                }
                _ => Ok(dir),
            }
        }
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

fn record_held(root: &Path, act: &Record<'_>) -> Result<Recorded, QuarantineError> {
    let feed = act.path.vendor();
    let key = series_key(act.path);
    let line = ledger_line(&key, act, now_micros());
    let (ledger_at, ended_torn) = append_line(&ledger_path(root, feed), &line)?;
    if act.scope == Scope::Dropping {
        return Ok(Recorded {
            ledger_at,
            ended_torn,
            rebuilt: false,
        });
    }
    let index = index_path(root, feed);
    let (mut image, rebuilt) = match read_image(&index) {
        Ok(Some(image)) if image_trusted(&image) => (image, false),
        Ok(None) => (empty_image(), false),
        // TORN OR DAMAGED: rebuilt from the ledger, which already holds the
        // line just written, so the rebuild indexes it too.
        Ok(Some(_)) | Err(QuarantineError::Torn { .. } | QuarantineError::Damaged { .. }) => {
            (image_from_ledger(root, feed)?, true)
        }
        Err(other) => return Err(other),
    };
    if !rebuilt {
        let entry = Entry {
            scope: act.scope,
            cause: act.cause,
            bars: act.bars,
            ledger_at,
            revision: act.revision,
            written_at: now_micros(),
        };
        upsert(&mut image, &hash_of(&key, act.path.month()), month_code(act.path.month()), entry, &index)?;
    }
    publish_image(root, feed, &image)?;
    Ok(Recorded {
        ledger_at,
        ended_torn,
        rebuilt,
    })
}

fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_micros()).unwrap_or(i64::MAX))
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

/// Appends one line; a ledger whose last byte is not a newline (a torn
/// append) is ended with one first. Answers the line's offset.
fn append_line(path: &Path, line: &str) -> Result<(u64, bool), QuarantineError> {
    let mut file = io_at(
        OpenOptions::new().read(true).append(true).create(true).open(path),
        path,
        "open the quarantine ledger",
    )?;
    let len = io_at(file.metadata(), path, "measure the quarantine ledger")?.len();
    let mut ended_torn = false;
    let mut at = len;
    if len == 0 {
        let head = format!("{LEDGER_HEAD}\n");
        io_at(file.write_all(head.as_bytes()), path, "write the ledger head")?;
        at = head.len() as u64;
    } else {
        let mut last = [0u8; 1];
        io_at(file.read_exact_at(&mut last, len - 1), path, "read the ledger tail")?;
        if last != *b"\n" {
            io_at(file.write_all(b"\n"), path, "end a torn ledger line")?;
            ended_torn = true;
            at = len + 1;
        }
    }
    io_at(file.write_all(line.as_bytes()), path, "append to the quarantine ledger")?;
    io_at(file.sync_all(), path, "sync the quarantine ledger")?;
    Ok((at, ended_torn))
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
    /// Complete lines that fail their checksum or do not parse, by offset.
    pub damaged: Vec<u64>,
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
            io_at(file.read_to_end(&mut bytes), &path, "read the quarantine ledger")?;
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

fn parse_ledger(bytes: &[u8]) -> Ledger {
    let mut ledger = Ledger::default();
    let mut at = 0u64;
    let mut rest = bytes;
    while !rest.is_empty() {
        let Some(end) = rest.iter().position(|byte| *byte == b'\n') else {
            ledger.torn_tail = true;
            break;
        };
        let (raw, after) = rest.split_at(end);
        let text = std::str::from_utf8(raw).unwrap_or("");
        if at != 0 && !text.is_empty() {
            match parse_line(text, at) {
                Some(line) => ledger.lines.push(line),
                None => ledger.damaged.push(at),
            }
        } else if at == 0 && text != LEDGER_HEAD {
            ledger.damaged.push(at);
        }
        at += end as u64 + 1;
        rest = after.get(1..).unwrap_or(&[]);
    }
    ledger
}

fn parse_line(text: &str, at: u64) -> Option<Line> {
    let (body, crc) = text.rsplit_once('\t')?;
    if u32::from_str_radix(crc, 16).ok()? != crc32c(body.as_bytes()) {
        return None;
    }
    let fields: Vec<&str> = body.split('\t').collect();
    let [written_at, _feed, key, month, scope, cause, bars, revision, digest, who, reason, stamps] =
        fields.as_slice()
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
/// As [`read_ledger`].
pub fn ledger_stamps(root: &Path, path: StorePath<'_>) -> Result<Vec<i64>, QuarantineError> {
    let ledger = read_ledger(root, path.vendor())?;
    let key = series_key(path);
    let month = path.month().to_string();
    let mut stamps: Vec<i64> = ledger
        .lines
        .iter()
        .filter(|line| line.scope == Scope::Dropped && line.key == key && line.month == month)
        .flat_map(|line| line.stamps.iter().copied())
        .collect();
    stamps.sort_unstable();
    stamps.dedup();
    Ok(stamps)
}

fn empty_image() -> Vec<u8> {
    let mut image = vec![0u8; INDEX_LEN as usize];
    let fields: [(usize, &[u8]); 5] = [
        (0, &MAGIC),
        (8, &1u32.to_le_bytes()),
        (12, &(SLOT as u32).to_le_bytes()),
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
    let header_ok = image.get(..8) == Some(&MAGIC[..])
        && u32::from_le_bytes(field(image, 60)) == crc32c(image.get(..60).unwrap_or(&[]));
    header_ok
        && image
            .get(HEADER_LEN as usize..)
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

/// Puts `entry` in the key's slot: the matching one, or the first free one
/// of its bucket. A drop onto a month already dropped adds its bars.
fn upsert(
    image: &mut [u8],
    hash: &[u8; 16],
    month: u32,
    mut entry: Entry,
    index: &Path,
) -> Result<(), QuarantineError> {
    let home = home_of(hash);
    for slot in home..home + BUCKET {
        let at = (HEADER_LEN + slot * SLOT_LEN) as usize;
        let bytes = image.get(at..at + SLOT).unwrap_or(&[]);
        match decode_slot(bytes, slot) {
            Slot::Held {
                hash: held,
                month: was,
                entry: Some(before),
            } if held == *hash && was == month => {
                if before.scope == Scope::Dropped && entry.scope == Scope::Dropped {
                    entry.bars = before.bars.saturating_add(entry.bars);
                }
            }
            Slot::Free => {}
            Slot::Held { .. } => continue,
            Slot::Damaged => {
                return Err(QuarantineError::Damaged {
                    path: index.to_path_buf(),
                    slot,
                });
            }
        }
        let encoded = encode_slot(hash, month, &entry, slot);
        if let Some(dst) = image.get_mut(at..at + SLOT) {
            dst.copy_from_slice(&encoded);
        }
        let entries = image
            .get(HEADER_LEN as usize..)
            .unwrap_or(&[])
            .chunks_exact(SLOT)
            .filter(|bytes| bytes.iter().any(|byte| *byte != 0))
            .count() as u64;
        if let Some(dst) = image.get_mut(32..40) {
            dst.copy_from_slice(&entries.to_le_bytes());
        }
        seal_header(image);
        return Ok(());
    }
    Err(QuarantineError::Full {
        path: index.to_path_buf(),
    })
}

/// The index image the ledger describes: every `dropped` and `whole_month`
/// line, in order.
fn image_from_ledger(root: &Path, feed: Vendor) -> Result<Vec<u8>, QuarantineError> {
    let ledger = read_ledger(root, feed)?;
    let index = index_path(root, feed);
    let mut image = empty_image();
    for line in &ledger.lines {
        if line.scope == Scope::Dropping {
            continue;
        }
        let Some(month) = parse_month(&line.month) else {
            continue;
        };
        let entry = Entry {
            scope: line.scope,
            cause: line.cause,
            bars: line.bars,
            ledger_at: line.at,
            revision: line.revision,
            written_at: line.written_at,
        };
        upsert(&mut image, &hash_of(&line.key, month), month_code(month), entry, &index)?;
    }
    Ok(image)
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
    io_at(staged.file.write_all(image), &staging, "write the index image")?;
    io_at(staged.file.sync_all(), &staging, "sync the index image")?;
    io_at(fs::rename(&staging, &index), &index, "publish the index image")?;
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
    /// Complete lines that did not check.
    pub damaged: usize,
}

/// Rebuilds the feed's index from its ledger and publishes it: the repair
/// of a torn or damaged index. Under the writer lock.
///
/// # Errors
///
/// As [`record`].
pub fn rebuild(root: &Path, feed: Vendor) -> Result<Rebuilt, QuarantineError> {
    let lock = writer_lock(root, feed, WRITER_PATIENCE)?;
    let done = (|| {
        let ledger = read_ledger(root, feed)?;
        let image = image_from_ledger(root, feed)?;
        let months = image
            .get(HEADER_LEN as usize..)
            .unwrap_or(&[])
            .chunks_exact(SLOT)
            .filter(|bytes| bytes.iter().any(|byte| *byte != 0))
            .count();
        publish_image(root, feed, &image)?;
        Ok(Rebuilt {
            lines: ledger.lines.len(),
            months,
            torn_tail: ledger.torn_tail,
            damaged: ledger.damaged.len(),
        })
    })();
    let _released = lock.release();
    done
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
    /// Builds the summary from the feed's ledger: every `dropped` and
    /// `whole_month` line, the newest per (series, month) deciding its scope.
    ///
    /// # Errors
    ///
    /// As [`read_ledger`].
    pub fn load(root: &Path, feed: Vendor) -> Result<Self, QuarantineError> {
        let ledger = read_ledger(root, feed)?;
        let mut months: HashMap<(String, String), Entry> = HashMap::new();
        for line in &ledger.lines {
            if line.scope == Scope::Dropping {
                continue;
            }
            let entry = Entry {
                scope: line.scope,
                cause: line.cause,
                bars: line.bars,
                ledger_at: line.at,
                revision: line.revision,
                written_at: line.written_at,
            };
            months
                .entry((line.key.clone(), line.month.clone()))
                .and_modify(|held| {
                    let bars = if held.scope == Scope::Dropped && entry.scope == Scope::Dropped {
                        held.bars.saturating_add(entry.bars)
                    } else {
                        entry.bars
                    };
                    *held = Entry { bars, ..entry };
                })
                .or_insert(entry);
        }
        let mut instruments: HashMap<String, Instrument> = HashMap::new();
        for ((key, _), entry) in &months {
            let instrument = key.splitn(4, '/').take(3).collect::<Vec<_>>().join("/");
            let row = instruments.entry(instrument).or_default();
            row.months += 1;
            row.whole_months += u64::from(entry.scope == Scope::WholeMonth);
            row.bars = row.bars.saturating_add(entry.bars);
            if !row.causes.contains(&entry.cause) {
                row.causes.push(entry.cause);
                row.causes.sort_unstable();
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
        self.instruments.get(&format!("{exchange}/{segment}/{symbol}"))
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

/// Sets aside the bars stamped `named` in `path`'s month: the whole month by
/// record when every held bar is named, otherwise a new generation without
/// them through [`crate::repair::drop_month`]. LEDGERED FIRST: the intent
/// line is synced before the door publishes, the `dropped` line and the
/// index after the promotion. Idempotent: a second apply of the same plan
/// writes nothing; a run killed between the promotion and its `dropped` line
/// is finished from its intent ([`Applied::Resumed`]).
///
/// # Cost
///
/// O(held) to read the month and O(ledger) to read the ledger, plus the
/// door's; the write path only.
///
/// # Errors
///
/// [`crate::repair::RepairError::UnheldStamp`] for a named stamp the month
/// does not hold and no ledger line set aside (nothing written); every
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
    let probed = match probe(root, path) {
        Err(QuarantineError::Torn { .. } | QuarantineError::Damaged { .. }) => {
            rebuild(root, path.vendor())?;
            probe(root, path)?
        }
        other => other?,
    };
    let source = crate::file::BarFile::open_existing(root, path, symbol_id)?;
    let n_valid = source.header().n_valid;
    let mut held_named = Vec::with_capacity(named.len());
    let mut unheld = Vec::new();
    {
        let mut wanted = named.iter().copied().peekable();
        for index in 0..n_valid {
            let bar = source.read_record(index)?;
            while let Some(early) = wanted.next_if(|at| *at < bar.ts_micros) {
                unheld.push(early);
            }
            if wanted.next_if(|at| *at == bar.ts_micros).is_some() {
                held_named.push(bar);
            }
        }
        unheld.extend(wanted);
    }
    drop(source);
    if let Probe::Quarantined(entry) = probed
        && entry.scope == Scope::WholeMonth
        && held_named.len() as u64 == n_valid
    {
        return Ok(Applied::Nothing {
            why: "already_whole",
        });
    }
    let mut resumable = None;
    if !unheld.is_empty() {
        let ledger = read_ledger(root, path.vendor())?;
        let key = series_key(path);
        let month = path.month().to_string();
        let mine: Vec<&Line> = ledger
            .lines
            .iter()
            .filter(|line| line.key == key && line.month == month)
            .collect();
        let set_aside = |at: i64, scope: Scope| {
            mine.iter()
                .any(|line| line.scope == scope && line.stamps.binary_search(&at).is_ok())
        };
        for at in &unheld {
            if !set_aside(*at, Scope::Dropped) && !set_aside(*at, Scope::Dropping) {
                return Err(crate::repair::RepairError::UnheldStamp(*at).into());
            }
        }
        // AN INTENT WITH NO `dropped` LINE whose bars the served month no
        // longer holds: a run killed after the promotion. Finished now.
        resumable = mine
            .iter()
            .rev()
            .find(|line| {
                line.scope == Scope::Dropping
                    && line.stamps.iter().all(|at| !set_aside(*at, Scope::Dropped))
                    && line.stamps.iter().all(|at| unheld.binary_search(at).is_ok())
            })
            .map(|line| (*line).clone());
    }
    if held_named.is_empty() {
        let Some(intent) = resumable else {
            return Ok(Applied::Nothing {
                why: "already_dropped",
            });
        };
        let served = crate::generation::served(root, path)?;
        let mut digest = [0u8; 32];
        for (at, pair) in digest.iter_mut().zip(intent.digest.as_bytes().chunks(2)) {
            *at = std::str::from_utf8(pair)
                .ok()
                .and_then(|text| u8::from_str_radix(text, 16).ok())
                .unwrap_or(0);
        }
        let done = record(
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
        return Ok(Applied::Resumed {
            bars: intent.bars,
            ledger_at: done.ledger_at,
        });
    }
    let stamps: Vec<i64> = held_named.iter().map(|bar| bar.ts_micros).collect();
    let digest = digest_of(&held_named);
    if held_named.len() as u64 == n_valid {
        let done = record(
            root,
            &Record {
                path,
                scope: Scope::WholeMonth,
                cause: why.cause,
                stamps: &[],
                bars: n_valid,
                revision: 0,
                source_digest: digest,
                who: why.who,
                reason: why.reason,
            },
        )?;
        return Ok(Applied::WholeMonth {
            bars: n_valid,
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
    record(root, &act(Scope::Dropping, 0))?;
    let drop = crate::repair::drop_month(root, path, symbol_id, &stamps, why.reason)?;
    let done = record(root, &act(Scope::Dropped, drop.to.ordinal()))?;
    Ok(Applied::Dropped {
        drop,
        ledger_at: done.ledger_at,
    })
}

#[cfg(test)]
#[path = "quarantine_tests.rs"]
mod tests;
