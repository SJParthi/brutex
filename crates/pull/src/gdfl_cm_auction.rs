//! The closing auction's prints, kept apart from trades (D-3320 item 3).
//!
//! # What is an auction print
//!
//! From 2026-08-03 a closing-auction-eligible share's continuous trading
//! ends at 15:15 (charter §7, NSE/CMTR/73845, NSE/CMTR/74466). The auction
//! then runs through its order-matching phase to 15:35: that timing rests on
//! a broker's dated implementation notice and is UNVERIFIED against an NSE
//! page or circular (charter §9, "The auction's phases";
//! [`super::AUCTION_BAND_END`]). In the GDFL capital-market files such a
//! share carries, in that band, a row whose bid and ask are both zero and
//! whose LTQ is positive: the auction's print (measured on 7 Oct 2026; its
//! price equals Zerodha's day close, D-3320). The match itself is stamped
//! from 15:28 at one price, and is often stated with a book beside it: when
//! every traded row of the match band is at one price, the booked ones are
//! prints too (SC-CLOSE-6, [`super::AUCTION_MATCH_FROM`]). The reader
//! ([`super::decode_with`]) diverts every such row of an eligible share out
//! of the rows the fold sees and into [`super::CmFile::auction`]; this module
//! files them. A print is never folded into a one-second bar, and none is
//! dropped (the operator's answer, D-3320: a silent drop is the fault
//! `CLAUDE.md` §4 bans).
//!
//! Identical consecutive rows (one second, one price, one quantity) are one
//! [`Print`] with their count in `rows`: whether two such rows are two fills
//! or one stated twice is UNVERIFIED (charter; HDFCBANK carries two), so the
//! quantity is the row's own and is never summed.
//!
//! # The `.auc` sidecar
//!
//! One file per instrument-month, beside the month's one-second bar file and
//! never inside it: `bars/gdfl/NSE/CASH/<SYM>/1s/<YYYY-MM>.auc` beside
//! `<YYYY-MM>.bin`. Every integer is little-endian. Two format versions
//! exist; this module is their byte authority until WP-99 transcribes both
//! into `docs/02-store-format.md` §8 (D-3320 item 3).
//!
//! ## Format version 1 (no checksum)
//!
//! | Bytes | Field | Value |
//! |---|---|---|
//! | 0..8 | magic | ASCII `BRTXAUC1` |
//! | 8..10 | version | `u16`, 1 |
//! | 10..12 | record length | `u16`, 32 |
//! | 12..16 | reserved | `u32`, 0 |
//!
//! then zero or more 32-byte records, day after day, and within a day in the
//! order the vendor's file states them:
//!
//! | Bytes | Field | Type |
//! |---|---|---|
//! | 0..8 | `t_micros`: the UTC microsecond of the print's second | `i64` |
//! | 8..16 | `price_paisa` | `i64` |
//! | 16..24 | `quantity`: the row's LTQ | `u64` |
//! | 24..28 | `rows`: identical consecutive rows merged | `u32` |
//! | 28..32 | reserved | `u32`, 0 |
//!
//! Version 1 checks only its shape: the header, a whole number of records,
//! the reserved fields zero. A flipped bit in a price, a quantity, a row
//! count or a time is not detectable in it, so a reader labels every
//! version-1 file **unchecked** ([`Sidecar::checked`]).
//!
//! ## Format version 2 (checksummed; every new file since AG-04)
//!
//! | Bytes | Field | Value |
//! |---|---|---|
//! | 0..8 | magic | ASCII `BRTXAUC1` (the family's magic, unchanged) |
//! | 8..10 | version | `u16`, 2 |
//! | 10..12 | record length | `u16`, 32 |
//! | 12..16 | header checksum | `u32`, CRC-32C of bytes 0..12 |
//!
//! then the records, each the version-1 record with its last four bytes
//! replaced:
//!
//! | Bytes | Field | Type |
//! |---|---|---|
//! | 0..28 | as version 1 | |
//! | 28..32 | record checksum: CRC-32C of bytes 0..28 followed by the record's zero-based ordinal in the file as a `u64` | `u32` |
//!
//! The ordinal is bound into the checksum exactly as the `.prv` record's is
//! (`store::provenance`), so a flipped bit anywhere in a record, and two
//! records swapped or one moved to another position, fail the check and are
//! refused by name with the ordinal. CRC-32C is `store::crc`'s.
//!
//! ## What both versions refuse, by name, never repaired
//!
//! A file shorter than its header; a magic, version or record length this
//! build does not write; a version-2 header whose checksum does not match;
//! a length that is not the header plus whole records (a torn tail, named by
//! the ordinal of the torn record); a version-1 reserved field set; a
//! version-2 record whose checksum does not match (named by its ordinal); a
//! record whose IST day is earlier than the record before it (out of order,
//! named by its ordinal). An empty file (a first append that never wrote its
//! header) is refused as that. Repairing a refused file is IA-16's (FM-099),
//! never this module's.
//!
//! # Append-only, idempotent (`CLAUDE.md` §3 rules 5 and 8)
//!
//! A day's prints are appended once, after every held record. Offered again,
//! byte for byte the records already held for that day, they write nothing.
//! A day whose records differ from the held ones, or a day older than one the
//! month already holds, is refused by name: it is a new generation of the
//! month (WP-80), never an in-place change or an insertion.
//!
//! **A version-1 file stays version 1.** An append to it writes version-1
//! records after version-1 records, so no file mixes the two and no held
//! byte is rewritten; only a NEW file is version 2. A month re-imported as a
//! new generation (WP-80) is a new file, and so version 2.
//!
//! **An append is all or nothing.** The day's records are written in one
//! call under the file's exclusive lock; if that write fails (a full disk),
//! the bytes it managed to write are cut back to the length the file had
//! when the lock was taken and the failure is named. No reader sees them:
//! a reader takes the shared lock ([`read`]). That is the append undone
//! before it was ever visible, not an in-place change of a held record.
//! A process killed inside the write leaves a torn tail: every reader
//! refuses it by name, and the next append cuts it back to the last whole
//! record that verifies before it writes (IA-16, IA-AUC-01, FM-099). Only a
//! partial record, whole records of zero bytes, or a partial header of a
//! file with no record are cut; any other damage stays refused.
//!
//! **A new file's directory is synced.** When an append creates the file
//! (or finds it empty), the file's directory, and the parent of every
//! directory the append had to create, are synced after the records, so a
//! crash cannot lose the entry of a file whose bytes were reported written.
//!
//! # Cost
//!
//! One append reads the month's sidecar whole (a few records a day, at most
//! a month of days: bounded by the month, not by the store), takes the
//! file's exclusive advisory lock (waiting at most [`LOCK_WAIT`] for a
//! reader), writes the day's records once and syncs them. Not a
//! constant-time claim; it is O(records in the month) per appended day, off
//! every read path. One [`read`] is one open, a shared lock held for one
//! read of the file, and one pass over its records: O(records in the month),
//! a handful a day, with one CRC-32C of 36 bytes per version-2 record.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use brutex_core::instrument::InstrumentKey;
use brutex_core::vendor::Vendor;
use store::crc::{crc32c, crc32c_split};
use store::flock::Flock;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{CmRefusal, CmRow};
use crate::session::{Day, IST_OFFSET_SECS};

/// The sidecar's magic, `BRTXAUC1`, shared by both versions.
pub const MAGIC: [u8; 8] = *b"BRTXAUC1";

/// The format version this module writes into every NEW sidecar: 2, the
/// checksummed one (AG-04, store-3).
pub const VERSION: u16 = 2;

/// The first format version, with no checksum: read, labelled unchecked,
/// and appended to in its own kind, never rewritten.
pub const VERSION_1: u16 = 1;

/// The header's length in bytes.
pub const HEADER_LEN: usize = 16;

/// One record's length in bytes.
pub const RECORD_LEN: usize = 32;

/// The bytes of a record its version-2 checksum covers, before the ordinal.
const RECORD_BODY: usize = 28;

/// The sidecar's file extension.
pub const EXTENSION: &str = "auc";

/// The longest a reader waits for an append's exclusive lock, and an append
/// for a reader's shared one, before refusing by name. A reader holds its
/// lock for one read of a few hundred bytes and an append for one write and
/// one sync, so a wait this long is a stuck holder, said so.
pub const LOCK_WAIT: Duration = Duration::from_secs(2);

/// One auction print as the reader diverted it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Print {
    /// The second of the IST day it is stamped at.
    pub sod: u32,
    /// Its price in paisa.
    pub price: i64,
    /// Its quantity: the row's LTQ, never a sum.
    pub quantity: u64,
    /// How many identical consecutive rows stated it.
    pub rows: u32,
    /// The one-based line of its first row in the file.
    pub line: u32,
}

/// Adds one diverted row: a row identical to the last print (one second, one
/// price, one quantity) raises its count; any other row is a new print.
pub(crate) fn merge(prints: &mut Vec<Print>, row: CmRow) {
    if let Some(last) = prints.last_mut()
        && last.sod == row.sod
        && last.price == row.ltp
        && last.quantity == row.ltq
    {
        last.rows = last.rows.saturating_add(1);
        return;
    }
    prints.push(Print {
        sod: row.sod,
        price: row.ltp,
        quantity: row.ltq,
        rows: 1,
        line: row.line,
    });
}

/// One record of a sidecar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    /// The UTC microsecond of the print's second.
    pub t_micros: i64,
    /// The price in paisa.
    pub price_paisa: i64,
    /// The row's LTQ.
    pub quantity: u64,
    /// Identical consecutive rows merged into it.
    pub rows: u32,
}

/// Copies `part` into `out` at `at`, where it fits.
fn put(out: &mut [u8], at: usize, part: &[u8]) {
    if let Some(slot) = out.get_mut(at..at + part.len()) {
        slot.copy_from_slice(part);
    }
}

/// `N` bytes of `bytes` at `at`, zeros where `bytes` is shorter.
fn take<const N: usize>(bytes: &[u8], at: usize) -> [u8; N] {
    bytes
        .get(at..at + N)
        .and_then(|slice| slice.try_into().ok())
        .unwrap_or([0; N])
}

/// A version-2 record's checksum: CRC-32C of its first 28 bytes followed by
/// its ordinal, little-endian.
fn record_crc(bytes: &[u8], ordinal: u64) -> u32 {
    crc32c_split(
        bytes.get(..RECORD_BODY).unwrap_or_default(),
        &ordinal.to_le_bytes(),
    )
}

/// The IST day number of a UTC microsecond.
const fn ist_day(t_micros: i64) -> i64 {
    (t_micros.div_euclid(1_000_000) + IST_OFFSET_SECS).div_euclid(86_400)
}

impl Record {
    /// `print` of `day` as a record.
    #[must_use]
    pub fn of(day: Day, print: &Print) -> Self {
        Self {
            t_micros: (i64::from(day.days_from_epoch()) * 86_400 + i64::from(print.sod)
                - IST_OFFSET_SECS)
                * 1_000_000,
            price_paisa: print.price,
            quantity: print.quantity,
            rows: print.rows,
        }
    }

    /// The record's 32 bytes in format version 1: the reserved field zero.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; RECORD_LEN] {
        self.encode(VERSION_1, 0)
    }

    /// The record's 32 bytes in format `version` at `ordinal`, its
    /// zero-based position in the file. Version 2's last four bytes are its
    /// checksum over the first 28 and the ordinal; version 1's are zero and
    /// the ordinal is not written.
    #[must_use]
    pub fn encode(&self, version: u16, ordinal: u64) -> [u8; RECORD_LEN] {
        let mut out = [0_u8; RECORD_LEN];
        put(&mut out, 0, &self.t_micros.to_le_bytes());
        put(&mut out, 8, &self.price_paisa.to_le_bytes());
        put(&mut out, 16, &self.quantity.to_le_bytes());
        put(&mut out, 24, &self.rows.to_le_bytes());
        if version != VERSION_1 {
            let crc = record_crc(&out, ordinal);
            put(&mut out, 28, &crc.to_le_bytes());
        }
        out
    }

    /// A version-1 record read back, or `None` when `bytes` is not one.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Self::decode(bytes, VERSION_1, 0).ok()
    }

    /// The record `bytes` holds at `ordinal` in a file of `version`.
    ///
    /// # Errors
    ///
    /// Not 32 bytes; version 1's reserved field set; version 2's checksum,
    /// which covers the ordinal, not matching. Named, never repaired.
    pub fn decode(bytes: &[u8], version: u16, ordinal: u64) -> Result<Self, String> {
        if bytes.len() != RECORD_LEN {
            return Err(format!(
                "{} bytes, not one {RECORD_LEN}-byte record",
                bytes.len()
            ));
        }
        let tail = take::<4>(bytes, RECORD_BODY);
        if version == VERSION_1 {
            if tail != [0; 4] {
                return Err("its reserved field is not zero".to_owned());
            }
        } else if u32::from_le_bytes(tail) != record_crc(bytes, ordinal) {
            return Err(
                "its checksum does not match: a flipped bit, a torn write, or a record moved \
                 from another position"
                    .to_owned(),
            );
        }
        Ok(Self {
            t_micros: i64::from_le_bytes(take::<8>(bytes, 0)),
            price_paisa: i64::from_le_bytes(take::<8>(bytes, 8)),
            quantity: u64::from_le_bytes(take::<8>(bytes, 16)),
            rows: u32::from_le_bytes(take::<4>(bytes, 24)),
        })
    }
}

/// The 16 header bytes of format `version`: version 1's last four reserved
/// (zero), version 2's the CRC-32C of the first twelve.
#[must_use]
pub fn header_of(version: u16) -> [u8; HEADER_LEN] {
    let mut out = [0_u8; HEADER_LEN];
    put(&mut out, 0, &MAGIC);
    put(&mut out, 8, &version.to_le_bytes());
    put(
        &mut out,
        10,
        &u16::try_from(RECORD_LEN).unwrap_or(0).to_le_bytes(),
    );
    if version != VERSION_1 {
        let crc = crc32c(out.get(..12).unwrap_or_default());
        put(&mut out, 12, &crc.to_le_bytes());
    }
    out
}

/// The header every new sidecar is written with: [`VERSION`]'s.
#[must_use]
pub fn header() -> [u8; HEADER_LEN] {
    header_of(VERSION)
}

/// Every record of one sidecar, and the format version it was written in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidecar {
    /// The format version: [`VERSION_1`] or [`VERSION`].
    pub version: u16,
    /// Every record, in file order.
    pub records: Vec<Record>,
}

impl Sidecar {
    /// Whether every record was checked against a checksum: `false` for a
    /// version-1 file, which carries none.
    #[must_use]
    pub const fn checked(&self) -> bool {
        self.version != VERSION_1
    }

    /// Why a reader cannot vouch for the records' values, or `None` where
    /// it can.
    #[must_use]
    pub const fn unchecked_why(&self) -> Option<&'static str> {
        if self.checked() {
            None
        } else {
            Some(
                "version 1 carries no checksum: its prices, quantities, row counts and times \
                 are served as written and a flipped bit in them cannot be detected",
            )
        }
    }
}

/// Why a sidecar's bytes were refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The zero-based ordinal of the record at fault, where one is.
    pub ordinal: Option<u64>,
    /// What is wrong, in words.
    pub why: String,
}

impl Refused {
    fn file(why: String) -> Self {
        Self { ordinal: None, why }
    }

    fn record(ordinal: usize, why: &str) -> Self {
        Self {
            ordinal: Some(u64::try_from(ordinal).unwrap_or(u64::MAX)),
            why: format!("record {ordinal}: {why}"),
        }
    }
}

impl core::fmt::Display for Refused {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.why)
    }
}

/// Every record of a sidecar's bytes, either version.
///
/// # Errors
///
/// [`Refused`], naming what the module header lists, with the record's
/// ordinal where one record is at fault.
pub fn parse(bytes: &[u8]) -> Result<Sidecar, Refused> {
    if bytes.is_empty() {
        return Err(Refused::file(
            "0 bytes: the file exists and holds no header: its first append is in progress, \
             or failed and was undone (that append's refusal names why)"
                .to_owned(),
        ));
    }
    let (head, body) = bytes.split_at_checked(HEADER_LEN).ok_or_else(|| {
        Refused::file(format!(
            "{} bytes, shorter than the {HEADER_LEN}-byte header: a truncated header",
            bytes.len()
        ))
    })?;
    if take::<8>(head, 0) != MAGIC {
        return Err(Refused::file("the magic is not BRTXAUC1".to_owned()));
    }
    let version = u16::from_le_bytes(take::<2>(head, 8));
    if version != VERSION_1 && version != VERSION {
        return Err(Refused::file(format!(
            "format version {version} is not one this build reads ({VERSION_1} or {VERSION})"
        )));
    }
    if head != header_of(version) {
        return Err(Refused::file(if version == VERSION_1 {
            "the version-1 header is not BRTXAUC1 with 32-byte records and a zero reserved field"
                .to_owned()
        } else {
            "the version-2 header's checksum does not match, or its record length is not 32"
                .to_owned()
        }));
    }
    let whole = body.len() / RECORD_LEN;
    if body.len() % RECORD_LEN != 0 {
        return Err(Refused {
            ordinal: Some(u64::try_from(whole).unwrap_or(u64::MAX)),
            why: format!(
                "{} bytes after the header, not a whole number of {RECORD_LEN}-byte records: \
                 record {whole} is torn ({} of its {RECORD_LEN} bytes)",
                body.len(),
                body.len() % RECORD_LEN
            ),
        });
    }
    let mut records = Vec::with_capacity(whole);
    let mut last_day = i64::MIN;
    for (ordinal, chunk) in body.chunks(RECORD_LEN).enumerate() {
        let record = Record::decode(chunk, version, u64::try_from(ordinal).unwrap_or(u64::MAX))
            .map_err(|why| Refused::record(ordinal, &why))?;
        let day = ist_day(record.t_micros);
        if day < last_day {
            return Err(Refused::record(
                ordinal,
                "its IST day is earlier than the record before it: out of order, which no \
                 append writes",
            ));
        }
        last_day = day;
        records.push(record);
    }
    Ok(Sidecar { version, records })
}

/// The sidecar at `path`, read under a shared lock so a read beside an
/// append sees the file before the append or after it, never between;
/// `Ok(None)` where there is no file.
///
/// # Errors
///
/// [`Refused`]: the file cannot be opened, locked within [`LOCK_WAIT`] or
/// read, or [`parse`] refuses its bytes. Each names the path.
pub fn read(path: &Path) -> Result<Option<Sidecar>, Refused> {
    let named = |why: String| Refused::file(format!("{}: {why}", path.display()));
    let file = match File::open(path) {
        Ok(file) => file,
        Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(named(why.to_string())),
    };
    let lock = hold(&file, path, Hold::Shared, LOCK_WAIT).map_err(&named)?;
    let mut bytes = Vec::new();
    let mut reader: &File = &lock;
    let got = reader.read_to_end(&mut bytes);
    lock.release().map_err(|why| named(why.to_string()))?;
    got.map_err(|why| named(why.to_string()))?;
    parse(&bytes).map(Some).map_err(|refused| Refused {
        why: format!("{}: {}", path.display(), refused.why),
        ..refused
    })
}

/// Which lock [`hold`] takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hold {
    Shared,
    Exclusive,
}

/// Takes `kind` of lock on `file`, waiting at most `bound` for the holder of
/// the other kind.
fn hold<'a>(
    file: &'a File,
    path: &Path,
    kind: Hold,
    bound: Duration,
) -> Result<Flock<&'a File>, String> {
    let started = Instant::now();
    loop {
        let tried = match kind {
            Hold::Shared => Flock::try_lock_shared(file, path.to_path_buf()),
            Hold::Exclusive => Flock::try_lock(file, path.to_path_buf()),
        };
        match tried {
            Ok(lock) => return Ok(lock),
            Err(std::fs::TryLockError::WouldBlock) if started.elapsed() < bound => {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(std::fs::TryLockError::WouldBlock) => {
                return Err(format!(
                    "the lock is held by another reader or writer past {} ms",
                    bound.as_millis()
                ));
            }
            Err(std::fs::TryLockError::Error(why)) => {
                return Err(format!("cannot be locked: {why}"));
            }
        }
    }
}

/// `key`'s one-second bar month of `day`'s month, as the store names it.
fn month_bars(key: &InstrumentKey, day: Day) -> Result<StorePath<'_>, String> {
    let month = YearMonth::new(day.year(), day.month()).map_err(|why| why.to_string())?;
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: key.exchange.as_str(),
        segment: key.segment.as_str(),
        symbol: key.underlying.as_str(),
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    })
    .map_err(|why| why.to_string())
}

/// The sidecar beside `key`'s one-second month file of `day`'s month, in the
/// GENERATION EVERY READER OPENS (IA-17b, D-4875 (5)): beside the original's
/// `.bin` while the month is served as the original, and beside the served
/// revision's (`bar-revisions-v1/<n>/...`) once a repair is promoted, the
/// way `BarFile::open_existing` resolves the bars
/// (`store::generation::served`). So a reader of a repaired CASH month reads
/// the `.auc` its generation was promoted with, never the original's, which
/// the repair left byte-identical.
///
/// For an original month the path is exactly the one this function always
/// answered: `bars/gdfl/NSE/CASH/<SYM>/1s/<YYYY-MM>.auc`.
///
/// # Errors
///
/// What the store's path builder refuses (an instrument or a month it cannot
/// file), and a generation pointer the store refuses
/// (`StoreError::Generation`: the served generation cannot be told, and the
/// original is not read in its place), named.
///
/// # Cost
///
/// One pointer probe: an `open` that answers not-found for a month never
/// repaired, else one read of the pointer's last record. O(1).
pub fn path(store_root: &Path, key: &InstrumentKey, day: Day) -> Result<PathBuf, String> {
    let bars = month_bars(key, day)?;
    let served = store::generation::served(store_root, bars).map_err(|why| why.to_string())?;
    Ok(bars
        .to_path_buf(&served.root(store_root))
        .with_extension(EXTENSION))
}

/// The ORIGINAL month's sidecar, the only one an append ever writes
/// ([`append`]): never inside a revision root.
fn original_path(store_root: &Path, key: &InstrumentKey, day: Day) -> Result<PathBuf, String> {
    Ok(month_bars(key, day)?
        .to_path_buf(store_root)
        .with_extension(EXTENSION))
}

/// What one append did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Appended {
    /// Records offered for the day.
    pub offered: usize,
    /// Records written; 0 when the day's records were already held.
    pub written: usize,
}

/// Why an append wrote nothing, typed so the import can tell a day the
/// month takes only with a new generation from a sidecar it must refuse
/// (IA-17b).
#[derive(Debug, Clone, PartialEq, Eq)]
enum NotFiled {
    /// The sidecar holds a day after this one: an insertion, which only the
    /// month's next generation can carry.
    Older(String),
    /// The month is served from a repair revision, whose files are never
    /// appended to (D-3350, append-only history).
    Revision(String),
    /// Anything else: the path, the file, its lock or its bytes; a day whose
    /// records differ; a write that failed (undone).
    Other(String),
}

impl NotFiled {
    fn words(self) -> String {
        match self {
            Self::Older(why) | Self::Revision(why) | Self::Other(why) => why,
        }
    }
}

/// Files `day`'s `prints` of `key` in the month's sidecar (see the module
/// header for the bytes and the rules).
///
/// **NEVER INTO A SERVED REVISION (IA-17b; CLAUDE.md §3 rule 8, D-3350).** A
/// month served from a repair revision is refused by name: its `.auc` is the
/// one its generation was promoted with ([`path`]), and the prints of a day
/// it lacks go with the month's next generation ([`write_revision`]), never
/// into the published revision's root and never into the original's, which
/// no reader of that month opens.
///
/// # Errors
///
/// [`CmRefusal::AuctionSidecar`] naming why: the path, the file, its lock or
/// its bytes; a day older than one the month holds; a day whose records
/// differ from those held for it; a month served from a revision; a write
/// that failed (undone) or a directory that could not be synced.
pub fn append(
    store_root: &Path,
    key: &InstrumentKey,
    day: Day,
    prints: &[Print],
) -> Result<Appended, CmRefusal> {
    append_typed(store_root, key, day, prints)
        .map_err(|why| CmRefusal::AuctionSidecar { why: why.words() })
}

/// One day's prints the import did not write, carried with the day's bars to
/// the month's next generation ([`DayPrints::WithGeneration`], IA-17b).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carried {
    /// The share.
    pub key: InstrumentKey,
    /// The day the prints are of.
    pub day: Day,
    /// The prints, as the reader diverted them.
    pub prints: Vec<Print>,
    /// Why they were not written to the month's sidecar.
    pub why: String,
}

/// Where the import's prints of one day went ([`file_or_carry`], IA-17b).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DayPrints {
    /// Filed in the month's sidecar, or held there already byte for byte
    /// (`written` 0).
    Appended(Appended),
    /// NOT WRITTEN ANYWHERE: the month takes this day's prints only with its
    /// next generation, so they travel with the day's bars to the gathered
    /// repair, whose step writes the new generation's whole `.auc`
    /// ([`write_revision`]). Why, in words.
    WithGeneration(String),
}

/// [`append`] for the import (IA-17b, D-4875 (5)): a day the month's sidecar
/// can only take as an insertion (it holds a later day), or a day of a month
/// served from a revision that its served `.auc` does not already hold, is
/// [`DayPrints::WithGeneration`] instead of a refusal, so the prints go with
/// the bars and are never dropped and never written in place. A served
/// revision that already holds the day's records byte for byte answers
/// `written` 0, as the original does.
///
/// # Errors
///
/// [`append`]'s, but for the two cases above; a served `.auc` that cannot be
/// read, and one holding different records for the day.
///
/// # Cost
///
/// [`append`]'s: one pointer probe and one read of the month's sidecar
/// (records in the month), and one write for an append.
pub fn file_or_carry(
    store_root: &Path,
    key: &InstrumentKey,
    day: Day,
    prints: &[Print],
) -> Result<DayPrints, CmRefusal> {
    let refuse = |why: String| CmRefusal::AuctionSidecar { why };
    match append_typed(store_root, key, day, prints) {
        Ok(appended) => Ok(DayPrints::Appended(appended)),
        Err(NotFiled::Older(why)) => Ok(DayPrints::WithGeneration(why)),
        Err(NotFiled::Revision(why)) => {
            let served = path(store_root, key, day).map_err(refuse)?;
            let held = read(&served).map_err(|refused| refuse(refused.why))?;
            let offered: Vec<Record> = prints.iter().map(|p| Record::of(day, p)).collect();
            let held_day = held
                .map(|sidecar| records_of_day(&sidecar.records, day))
                .unwrap_or_default();
            if held_day.is_empty() {
                return Ok(DayPrints::WithGeneration(why));
            }
            if held_day == offered {
                return Ok(DayPrints::Appended(Appended {
                    offered: offered.len(),
                    written: 0,
                }));
            }
            Err(refuse(format!(
                "{}: {day} holds {} records and {} different ones were offered; a new \
                 generation, never an in-place change",
                served.display(),
                held_day.len(),
                offered.len()
            )))
        }
        Err(other) => Err(refuse(other.words())),
    }
}

/// `day`'s IST date as UTC microseconds, from its first (inclusive) to the
/// next day's first (exclusive).
fn span_of_day(day: Day) -> (i64, i64) {
    let start = (i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS) * 1_000_000;
    (start, start + 86_400 * 1_000_000)
}

/// The records of `records` stamped on `day`'s IST date, in file order.
fn records_of_day(records: &[Record], day: Day) -> Vec<Record> {
    let (start, end) = span_of_day(day);
    records
        .iter()
        .filter(|r| r.t_micros >= start && r.t_micros < end)
        .copied()
        .collect()
}

/// [`append`], its refusal typed.
fn append_typed(
    store_root: &Path,
    key: &InstrumentKey,
    day: Day,
    prints: &[Print],
) -> Result<Appended, NotFiled> {
    let refuse = NotFiled::Other;
    let offered: Vec<Record> = prints.iter().map(|p| Record::of(day, p)).collect();
    let appended = Appended {
        offered: offered.len(),
        written: 0,
    };
    if offered.is_empty() {
        return Ok(appended);
    }
    let bars = month_bars(key, day).map_err(refuse)?;
    match store::generation::served(store_root, bars) {
        Ok(store::generation::Served::Original) => {}
        Ok(served) => {
            return Err(NotFiled::Revision(format!(
                "{} {day}: the month is served from revision {}, whose files are never appended \
                 to; the day's prints go with the month's next generation",
                key.underlying,
                served.ordinal()
            )));
        }
        Err(why) => {
            return Err(refuse(format!(
                "{} {day}: the month's served generation cannot be told: {why}",
                key.underlying
            )));
        }
    }
    let path = original_path(store_root, key, day).map_err(refuse)?;
    // THE DIRECTORIES THIS APPEND MAKES, so their entries are synced with
    // the file's: the parent of each holds its entry.
    let mut made: Vec<PathBuf> = Vec::new();
    if let Some(parent) = path.parent() {
        let mut at = Some(parent);
        while let Some(dir) = at.filter(|dir| !dir.as_os_str().is_empty() && !dir.exists()) {
            made.push(dir.to_path_buf());
            at = dir.parent();
        }
        std::fs::create_dir_all(parent)
            .map_err(|why| refuse(format!("{}: {why}", parent.display())))?;
    }
    let file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(&path)
        .map_err(|why| refuse(format!("{}: {why}", path.display())))?;
    let lock = hold(&file, &path, Hold::Exclusive, LOCK_WAIT).map_err(|why| {
        refuse(format!(
            "{} is busy or cannot be locked: {why}",
            path.display()
        ))
    })?;
    let written = write_day(&lock, &path, day, &offered, &made);
    lock.release().map_err(|why| {
        refuse(format!(
            "{}: the lock was not released: {why}",
            path.display()
        ))
    })?;
    let (written, version) = written?;
    note_filed(key, day, &path, offered.len(), written, version);
    Ok(Appended {
        written,
        ..appended
    })
}

/// Under the lock: reads the held records, decides, and appends in the
/// file's own version. Returns the records written and that version.
fn write_day(
    file: &File,
    path: &Path,
    day: Day,
    offered: &[Record],
    made: &[PathBuf],
) -> Result<(usize, u16), NotFiled> {
    write_day_held(file, path, day, offered, made).map_err(|(older, why)| {
        if older {
            NotFiled::Older(why)
        } else {
            NotFiled::Other(why)
        }
    })
}

/// The body of [`write_day`]: every refusal is `(false, words)` but the
/// older-day one, `(true, words)`.
fn write_day_held(
    file: &File,
    path: &Path,
    day: Day,
    offered: &[Record],
    made: &[PathBuf],
) -> Result<(usize, u16), (bool, String)> {
    let other = |why: String| (false, why);
    let mut bytes = Vec::new();
    let mut reader = file;
    reader
        .read_to_end(&mut bytes)
        .map_err(|why| other(format!("{}: {why}", path.display())))?;
    // A TORN TAIL A KILLED APPEND LEFT IS CUT BACK BY THE NEXT (IA-16,
    // IA-AUC-01, FM-099): every reader refused it, so no reader ever saw it;
    // only bytes after the last whole record that verifies are cut, under
    // the exclusive lock, and never a record the file held whole.
    if let Some(keep) = torn_tail(&bytes) {
        let had = bytes.len();
        file.set_len(u64::try_from(keep).unwrap_or(u64::MAX))
            .and_then(|()| file.sync_data())
            .map_err(|why| {
                other(format!(
                    "{}: its torn tail ({} of {had} bytes) could not be cut back: {why}",
                    path.display(),
                    had - keep
                ))
            })?;
        bytes.truncate(keep);
        note_torn_cut(path, had, keep);
    }
    let fresh = bytes.is_empty();
    let held = if fresh {
        Sidecar {
            version: VERSION,
            records: Vec::new(),
        }
    } else {
        parse(&bytes).map_err(|why| other(format!("{}: {why}", path.display())))?
    };
    let (_, end) = span_of_day(day);
    let held_day = records_of_day(&held.records, day);
    if !held_day.is_empty() {
        if held_day == offered {
            return Ok((0, held.version));
        }
        return Err(other(format!(
            "{}: {day} holds {} records and {} different ones were offered; a new generation, \
             never an in-place change",
            path.display(),
            held_day.len(),
            offered.len()
        )));
    }
    if held.records.last().is_some_and(|last| last.t_micros >= end) {
        return Err((
            true,
            format!(
                "{}: the month holds a day after {day}; {day} is a new generation, never an \
                 insertion",
                path.display()
            ),
        ));
    }
    let mut out = Vec::with_capacity(HEADER_LEN + offered.len() * RECORD_LEN);
    if fresh {
        out.extend_from_slice(&header_of(held.version));
    }
    let first = held.records.len();
    for (at, record) in offered.iter().enumerate() {
        let ordinal = u64::try_from(first + at).unwrap_or(u64::MAX);
        out.extend_from_slice(&record.encode(held.version, ordinal));
    }
    if let Err(why) = put_bytes(file, &out).and_then(|()| file.sync_data()) {
        let before = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let undone = file.set_len(before).and_then(|()| file.sync_data());
        return Err(other(match undone {
            Ok(()) => format!(
                "{}: {why}; the append was undone, the file is its {before} bytes again and \
                 nothing is half-written",
                path.display()
            ),
            Err(also) => format!(
                "{}: {why}; undoing the append to {before} bytes failed too ({also}): the file \
                 may end in a torn tail every reader refuses",
                path.display()
            ),
        }));
    }
    if fresh {
        let mut dirs: Vec<&Path> = path.parent().into_iter().collect();
        dirs.extend(made.iter().filter_map(|dir| dir.parent()));
        for dir in dirs {
            sync_dir(dir).map_err(|why| {
                other(format!(
                    "{}: written and synced, but its directory {} was not synced: {why}",
                    path.display(),
                    dir.display()
                ))
            })?;
        }
    }
    Ok((offered.len(), held.version))
}

/// THE NEW GENERATION'S WHOLE `.auc` (IA-17b; D-4875 (5), D-4898): the step
/// a CASH month's gathered repair hands `store::repair::repair_month_with`,
/// run with the new revision's root after it is published and before it is
/// promoted.
///
/// It writes `<revision_root>/bars/gdfl/NSE/CASH/<SYM>/1s/<YYYY-MM>.auc`:
/// every record of the sidecar of the generation served NOW ([`path`], the
/// original or the revision being replaced), and the `carried` days' prints
/// ([`DayPrints::WithGeneration`]), in IST day order, in format [`VERSION`]
/// (a new generation is a new file; records of a version-1 sidecar are
/// carried and checked from here on). A carried day the served sidecar
/// already holds byte for byte is kept once; one it holds differently is
/// refused, so the repair stops before the promotion and nothing is served
/// with two readings of one day. No served sidecar and nothing carried is no
/// file: a month before the closing auction's era writes none.
///
/// **Repeatable, and only where it is handed.** The file is written whole
/// beside itself (`.tmp`), synced, renamed over the target and its directory
/// synced, so an attempt a crash tore is replaced whole by the retry, and a
/// target already holding exactly these bytes is left alone. Nothing is
/// written outside `revision_root`: the served generation's sidecar and the
/// original's are only read, and a revision another repair published beside
/// this one is never assumed to be the one served (verify-IA-17-r1
/// follow-up 4).
///
/// Returns the records written, or `None` when there was nothing to write.
///
/// # Errors
///
/// The served sidecar's refusal ([`read`]), a carried day that differs from
/// the one held, and any host refusal of the write, each in words naming the
/// path.
///
/// # Cost
///
/// O(records of the month): one read of the served sidecar, one write and two
/// syncs. Runs once per repaired CASH month, never on a read path.
pub fn write_revision(
    store_root: &Path,
    revision_root: &Path,
    key: &InstrumentKey,
    month_of: Day,
    carried: &[(Day, &[Print])],
) -> Result<Option<usize>, String> {
    let served = path(store_root, key, month_of)?;
    let held = read(&served).map_err(|refused| refused.why)?;
    let (held_version, mut records) = held.map_or((VERSION, Vec::new()), |sidecar| {
        (sidecar.version, sidecar.records)
    });
    for &(day, prints) in carried {
        let offered: Vec<Record> = prints.iter().map(|p| Record::of(day, p)).collect();
        if offered.is_empty() {
            continue;
        }
        let held_day = records_of_day(&records, day);
        if held_day == offered {
            continue;
        }
        if !held_day.is_empty() {
            return Err(format!(
                "{}: {day} holds {} records and {} different ones were carried with the new \
                 generation; one day is never served with two readings",
                served.display(),
                held_day.len(),
                offered.len()
            ));
        }
        // IN IST DAY ORDER, each day's records in the order they were
        // stated: the held records are in day order (`parse` refuses any
        // other), so the carried day goes in after the last record of an
        // earlier day, walking back from the end past the later days (at
        // most a month of records). A later day's record is one at or after
        // the next IST midnight: no record of `day` itself is left by here.
        let (_, end) = span_of_day(day);
        let mut at = records.len();
        while at > 0
            && records
                .get(at - 1)
                .is_some_and(|record| record.t_micros >= end)
        {
            at -= 1;
        }
        let _replaced: Vec<Record> = records.splice(at..at, offered).collect();
    }
    if records.is_empty() {
        return Ok(None);
    }
    let mut out = Vec::with_capacity(HEADER_LEN + records.len() * RECORD_LEN);
    out.extend_from_slice(&header());
    for (at, record) in records.iter().enumerate() {
        out.extend_from_slice(&record.encode(VERSION, u64::try_from(at).unwrap_or(u64::MAX)));
    }
    let target = month_bars(key, month_of)?
        .to_path_buf(revision_root)
        .with_extension(EXTENSION);
    let named = |why: io::Error| format!("{}: {why}", target.display());
    if std::fs::read(&target).is_ok_and(|bytes| bytes == out) {
        return Ok(Some(records.len()));
    }
    let dir = target
        .parent()
        .ok_or_else(|| format!("{}: no directory", target.display()))?;
    std::fs::create_dir_all(dir).map_err(named)?;
    // THE ONE DOOR FOR A TEMPORARY (gate 28, RA-290): never a name already
    // there opened and cut, which could be a hard link of another file. This
    // process's own name, so two writers never share one; a dead writer's
    // leftover under it is cleared by the door and said.
    let tmp = target.with_extension(format!("{EXTENSION}.{}.tmp", std::process::id()));
    let mut staged = store::staging::create(&tmp).map_err(|why| why.to_string())?;
    let written = put_whole(&mut staged.file, &out)
        .and_then(|()| staged.file.sync_all())
        .and_then(|()| std::fs::rename(&tmp, &target));
    drop(staged.file);
    if let Err(why) = written {
        // Nothing half-written is left behind a refusal that says so.
        let _removed = std::fs::remove_file(&tmp);
        return Err(named(why));
    }
    sync_dir(dir).map_err(named)?;
    note_revision_written(
        key,
        month_of,
        (&target, staged.cleared),
        records.len(),
        held_version,
    );
    Ok(Some(records.len()))
}

/// Writes `bytes` whole into a fresh file.
fn put_whole(file: &mut File, bytes: &[u8]) -> io::Result<()> {
    #[cfg(test)]
    if let Some(after) = seam::FAIL_AFTER.with(std::cell::Cell::take) {
        file.write_all(bytes.get(..after).unwrap_or(bytes))?;
        return Err(io::Error::new(
            io::ErrorKind::StorageFull,
            "no space left on device (injected)",
        ));
    }
    file.write_all(bytes)
}

/// One `info` line per written revision sidecar (IA-17b): the share, the
/// month, the records, the version the served one was in, and the bytes of
/// a dead writer's leftover the staging door cleared (0 for none).
fn note_revision_written(
    key: &InstrumentKey,
    month_of: Day,
    (path, cleared): (&Path, Option<u64>),
    records: usize,
    from_version: u16,
) {
    let month = format!("{:04}-{:02}", month_of.year(), month_of.month());
    let path_text = path.display().to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(
            crate::gdfl_import::TARGET,
            "auction file written for a new generation",
        )
        .with("symbol", key.underlying.as_str())
        .with("month", month.as_str())
        .with("records", records)
        .with("from_version", usize::from(from_version))
        .with("version", usize::from(VERSION))
        .with("sidecar", path_text.as_str())
        .with("leftover_cleared_bytes", cleared.unwrap_or(0))
        .with("code", "auction_revision_written"),
    );
}

/// Writes `bytes` at the file's end in one call.
fn put_bytes(file: &File, bytes: &[u8]) -> io::Result<()> {
    #[cfg(test)]
    if let Some(after) = seam::FAIL_AFTER.with(std::cell::Cell::take) {
        let mut writer = file;
        writer.write_all(bytes.get(..after).unwrap_or(bytes))?;
        return Err(io::Error::new(
            io::ErrorKind::StorageFull,
            "no space left on device (injected)",
        ));
    }
    let mut writer = file;
    writer.write_all(bytes)
}

/// Syncs the directory `dir`, so the entries it holds survive a crash.
fn sync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(test)]
    seam::SYNCED.with(|synced| synced.borrow_mut().push(dir.to_path_buf()));
    File::open(dir)?.sync_all()
}

/// The test seams: a write that fails after so many bytes, as a full disk
/// does, and the directories synced, in order. Per thread, so tests running
/// at once never see each other's.
#[cfg(test)]
pub(crate) mod seam {
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;

    thread_local! {
        /// The next append's write stops after this many bytes and fails.
        pub(crate) static FAIL_AFTER: Cell<Option<usize>> = const { Cell::new(None) };
        /// Every directory synced on this thread.
        pub(crate) static SYNCED: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    }
}

/// One `info` line per filed day (D-3320): the share, the day, the prints
/// offered and written, the sidecar and its format version.
/// The length to cut a sidecar's bytes back to when they end in a TORN TAIL
/// (IA-16, IA-AUC-01): a killed append's partial record, or whole records of
/// zero bytes (a length that reached the disk before its bytes; a record of
/// zeros is no print, its time is 1970), after records that all verify. A
/// partial header of a file that holds no record is cut to nothing, so the
/// next append writes a fresh file. `None` for bytes that end whole, and for
/// any damage other than the tail: a flipped bit, a bad header, a record
/// that fails its check in place stays refused by name, never repaired.
/// O(bytes): one parse of the kept prefix.
fn torn_tail(bytes: &[u8]) -> Option<usize> {
    if bytes.is_empty() {
        return None;
    }
    if bytes.len() < HEADER_LEN {
        let partial_header = [VERSION, VERSION_1]
            .into_iter()
            .any(|version| header_of(version).starts_with(bytes));
        return partial_header.then_some(0);
    }
    let whole = HEADER_LEN + (bytes.len() - HEADER_LEN) / RECORD_LEN * RECORD_LEN;
    let mut keep = whole;
    while keep > HEADER_LEN
        && bytes
            .get(keep - RECORD_LEN..keep)
            .is_some_and(|record| record.iter().all(|&byte| byte == 0))
    {
        keep -= RECORD_LEN;
    }
    (keep < bytes.len() && bytes.get(..keep).is_some_and(|kept| parse(kept).is_ok()))
        .then_some(keep)
}

/// Says a torn tail was cut back before an append (IA-16, IA-AUC-01), with
/// the lengths, at `warn`: a state change of the sidecar a search finds.
fn note_torn_cut(path: &Path, had: usize, kept: usize) {
    let path_text = path.display().to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(
            crate::gdfl_import::TARGET,
            "auction sidecar torn tail cut back",
        )
        .with("sidecar", path_text.as_str())
        .with("bytes", had)
        .with("kept", kept)
        .with("code", "auction_torn_tail"),
    );
}

fn note_filed(
    key: &InstrumentKey,
    day: Day,
    path: &Path,
    offered: usize,
    written: usize,
    version: u16,
) {
    let day_text = day.to_string();
    let path_text = path.display().to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(crate::gdfl_import::TARGET, "auction prints filed")
            .with("symbol", key.underlying.as_str())
            .with("day", day_text.as_str())
            .with("prints", offered)
            .with("written", written)
            .with("version", usize::from(version))
            .with("sidecar", path_text.as_str()),
    );
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]
mod cloud_mutant_tests {
    //! Mutant kills for [`merge`] (cloud mutation round, 2026-10-16). Every
    //! price and quantity is invented.

    use super::{CmRow, Print, merge};

    const fn row(line: u32, sod: u32, ltp: i64, ltq: u64) -> CmRow {
        CmRow {
            line,
            sod,
            ltp,
            ltq,
        }
    }

    /// Only a row identical to the last print in ALL THREE of second, price
    /// and quantity raises its count; a row differing in any one of them is
    /// a new print, and the merged print keeps its FIRST row's line. Kills
    /// each `&&` of the merge test read as `||` and each `==` read as `!=`.
    #[test]
    fn a_row_merges_only_when_second_price_and_quantity_all_match() {
        let first = row(10, 55_687, 123_405, 7);
        for (other, what) in [
            (row(11, 55_688, 123_405, 7), "another second"),
            (row(11, 55_687, 123_410, 7), "another price"),
            (row(11, 55_687, 123_405, 8), "another quantity"),
        ] {
            let mut prints = Vec::new();
            merge(&mut prints, first);
            merge(&mut prints, other);
            assert_eq!(prints.len(), 2, "{what} is a new print");
            assert_eq!(
                prints[1],
                Print {
                    sod: other.sod,
                    price: other.ltp,
                    quantity: other.ltq,
                    rows: 1,
                    line: 11,
                },
                "{what}"
            );
            assert_eq!(prints[0].rows, 1, "{what} leaves the first alone");
        }
        let mut prints = Vec::new();
        merge(&mut prints, first);
        merge(&mut prints, row(11, 55_687, 123_405, 7));
        merge(&mut prints, row(12, 55_687, 123_405, 7));
        assert_eq!(
            prints,
            [Print {
                sod: 55_687,
                price: 123_405,
                quantity: 7,
                rows: 3,
                line: 10,
            }],
            "three identical rows are one print stated three times, its quantity never summed"
        );
    }

    /// The count saturates rather than wrapping to zero.
    #[test]
    fn a_merged_count_saturates() {
        let mut prints = vec![Print {
            sod: 1,
            price: 2,
            quantity: 3,
            rows: u32::MAX,
            line: 4,
        }];
        merge(&mut prints, row(5, 1, 2, 3));
        assert_eq!(prints[0].rows, u32::MAX);
        assert_eq!(prints.len(), 1);
    }

    /// A directory where the month's sidecar belongs is refused by the
    /// sidecar's own name, and nothing is written inside it. Invented share.
    #[test]
    #[allow(
        clippy::unwrap_used,
        clippy::panic,
        reason = "a test that cannot panic cannot fail"
    )]
    fn a_directory_at_the_sidecar_name_is_refused_by_name() {
        let root = crate::gdfl_fixtures::scratch("CloudSidecarDir");
        let key = super::InstrumentKey::cash(brutex_core::instrument::Exchange::Nse, "ZZAUCTION")
            .unwrap();
        let day = super::Day::new(2026, 8, 4).unwrap();
        let sidecar = super::path(&root, &key, day).unwrap();
        std::fs::create_dir_all(&sidecar).unwrap();
        let print = Print {
            sod: 55_687,
            price: 140_150,
            quantity: 400,
            rows: 1,
            line: 3,
        };
        let why = match super::append(&root, &key, day, &[print]) {
            Err(super::CmRefusal::AuctionSidecar { why }) => why,
            other => panic!("{other:?}"),
        };
        assert!(
            why.starts_with(&format!("{}: ", sidecar.display())),
            "{why}"
        );
        assert!(std::fs::read_dir(&sidecar).unwrap().next().is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A header that is not its version's own is refused in that version's
    /// words: a version-1 header with a reserved byte set is named against
    /// version 1's shape, a version-2 header with its checksum broken against
    /// version 2's. Kills the message choice's `==` read as `!=`, which
    /// swapped the two.
    #[test]
    #[allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]
    fn a_bad_header_is_refused_in_its_own_versions_words() {
        let mut first = super::header_of(super::VERSION_1);
        first[12] = 1;
        assert_eq!(
            super::parse(&first).unwrap_err().why,
            "the version-1 header is not BRTXAUC1 with 32-byte records and a zero reserved field"
        );
        let mut second = super::header_of(super::VERSION);
        second[12] ^= 0xFF;
        assert_eq!(
            super::parse(&second).unwrap_err().why,
            "the version-2 header's checksum does not match, or its record length is not 32"
        );
        // The premise: each header unbroken is read, with no record.
        for version in [super::VERSION_1, super::VERSION] {
            let read = super::parse(&super::header_of(version)).unwrap();
            assert_eq!((read.version, read.records.len()), (version, 0));
        }
    }

    /// Only a sidecar that is not there reads as no sidecar: one that cannot
    /// be opened for another reason (a regular file where its directory
    /// belongs) is refused naming the path. Kills the absence guard read as
    /// `true`, which answered every open failure as an absent file.
    #[test]
    #[allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]
    fn only_a_missing_sidecar_reads_as_none() {
        let root = crate::gdfl_fixtures::scratch("CloudSidecarRead");
        assert!(matches!(super::read(&root.join("Absent.auc")), Ok(None)));
        let file = root.join("File");
        std::fs::write(&file, b"x").unwrap();
        let under = file.join("Under.auc");
        let refused = super::read(&under).unwrap_err();
        assert_eq!(refused.ordinal, None);
        assert!(
            refused.why.starts_with(&format!("{}: ", under.display())),
            "{}",
            refused.why
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A fresh sidecar's new directories are synced in their parents up to
    /// the directory that already existed, and no higher: the store root is
    /// synced (it holds the first directory the append made) and nothing
    /// above it is. Kills the walk's `&&` read as `||`, which took every
    /// existing ancestor up to `/` for a directory the append made.
    #[test]
    #[allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]
    fn a_fresh_sidecar_syncs_no_directory_above_the_one_it_was_made_in() {
        let root = crate::gdfl_fixtures::scratch("CloudSidecarSync");
        let key = super::InstrumentKey::cash(brutex_core::instrument::Exchange::Nse, "ZZAUCTION")
            .unwrap();
        let day = super::Day::new(2026, 8, 4).unwrap();
        let print = Print {
            sod: 55_687,
            price: 140_150,
            quantity: 400,
            rows: 1,
            line: 3,
        };
        super::seam::SYNCED.with(|synced| synced.borrow_mut().clear());
        super::append(&root, &key, day, &[print]).unwrap();
        let synced = super::seam::SYNCED.with(|synced| synced.borrow().clone());
        assert!(synced.contains(&root), "{synced:?}");
        assert!(
            synced.iter().all(|dir| dir.starts_with(&root)),
            "{synced:?}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A sidecar lock another handle holds past the bound is refused once
    /// the bound passes, naming it. Run on its own thread under a ten-second
    /// watchdog, so a wait that never ends fails here instead of hanging the
    /// run (the wait guard read as `true`).
    #[test]
    #[allow(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "a test that cannot panic cannot fail"
    )]
    fn a_sidecar_lock_held_past_the_bound_is_refused_in_time() {
        let root = crate::gdfl_fixtures::scratch("CloudSidecarHeld");
        let path = root.join("Held.auc");
        std::fs::write(&path, b"").unwrap();
        let holder = std::fs::File::open(&path).unwrap();
        holder.lock().unwrap();
        let (send, answer) = std::sync::mpsc::channel();
        let at = path.clone();
        std::thread::spawn(move || {
            let file = std::fs::File::open(&at).unwrap();
            let bound = std::time::Duration::from_millis(50);
            let got = super::hold(&file, &at, super::Hold::Shared, bound).map(drop);
            let _gone_when_the_test_ended = send.send(got);
        });
        let got = answer
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("refused within ten seconds");
        assert_eq!(
            got,
            Err("the lock is held by another reader or writer past 50 ms".to_owned())
        );
        holder.unlock().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A fresh sidecar whose directory can be written but not opened (mode
    /// 0o300) is written and synced, and then the append refuses naming the
    /// directory that was not synced: the record is on disk, its entry's
    /// durability is not vouched for. Run where permission bits bind
    /// (D-0995).
    #[test]
    #[allow(
        clippy::unwrap_used,
        clippy::panic,
        reason = "a test that cannot panic cannot fail"
    )]
    fn a_sidecar_directory_that_cannot_be_synced_is_named() {
        crate::support::where_permission_binds(
            "gdfl_cm::auction::cloud_mutant_tests::a_sidecar_directory_that_cannot_be_synced_is_named",
            || {
                use std::os::unix::fs::PermissionsExt as _;
                let root = crate::gdfl_fixtures::scratch("CloudSidecarNoSync");
                let key =
                    super::InstrumentKey::cash(brutex_core::instrument::Exchange::Nse, "ZZAUCTION")
                        .unwrap();
                let day = super::Day::new(2026, 8, 4).unwrap();
                let path = super::path(&root, &key, day).unwrap();
                let dir = path.parent().unwrap().to_path_buf();
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o300)).unwrap();
                let print = Print {
                    sod: 55_687,
                    price: 140_150,
                    quantity: 400,
                    rows: 1,
                    line: 3,
                };
                let refused = super::append(&root, &key, day, &[print]);
                std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
                let why = match refused {
                    Err(super::CmRefusal::AuctionSidecar { why }) => why,
                    other => panic!("{other:?}"),
                };
                assert!(
                    why.starts_with(&format!(
                        "{}: written and synced, but its directory {} was not synced: ",
                        path.display(),
                        dir.display()
                    )),
                    "{why}"
                );
                let held = super::parse(&std::fs::read(&path).unwrap()).unwrap();
                assert_eq!(held.records.len(), 1, "the record is on disk");
                std::fs::remove_dir_all(root).unwrap();
            },
        );
    }
}

#[cfg(test)]
#[path = "gdfl_cm_auction_v2_tests.rs"]
mod v2_tests;

#[cfg(test)]
#[path = "gdfl_cm_auction_revision_tests.rs"]
mod revision_tests;
