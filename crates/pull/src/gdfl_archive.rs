//! The vendor's capital-market zips read in place, the one [`CmSource`]
//! today: the outer archive's central directory indexed once, each stored
//! day zip read by offset, and each member inflated in memory and checked
//! against the size and CRC-32 its central record states. Nothing is
//! extracted to disk. D-0812, D-2800.

use std::path::Path;

use std::sync::Arc;

use crate::gdfl_cm::{CmKind, CmRefusal, CmScope, CmSource, DayListing, ListedFile};
use crate::gdfl_run::preflight::{Watch, WatchFault};
use crate::session::Day;

/// CRC-32 of `bytes` with the zip polynomial.
#[must_use]
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = flate2::Crc::new();
    crc.update(bytes);
    crc.sum()
}

/// Random access to an archive's bytes.
pub trait ReadAt {
    /// Fills `buf` with the bytes starting at `offset`.
    ///
    /// # Errors
    ///
    /// Whatever the read fails with, including a read past the end.
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()>;
}

/// A positional read (`pread`), as `store::file` reads: it moves no shared
/// cursor, so one `Archive<File>` read from many threads at once never has
/// one read land at another's offset (design §3.3 asks for `read_at`).
impl ReadAt for std::fs::File {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        std::os::unix::fs::FileExt::read_exact_at(self, buf, offset)
    }
}

impl ReadAt for Vec<u8> {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        let held = self
            .get(index(offset)..)
            .and_then(|rest| rest.get(..buf.len()))
            .ok_or(std::io::ErrorKind::UnexpectedEof)?;
        buf.copy_from_slice(held);
        Ok(())
    }
}

/// No end-of-central-directory record ends the bytes.
const NO_END_RECORD: &str = "no end-of-central-directory record ends it";
/// An end record says "see the ZIP64 record" with no ZIP64 locator before it.
const SATURATED_WITHOUT_LOCATOR: &str = "a saturated end record has no ZIP64 locator";
/// The ZIP64 locator points at bytes that are not a ZIP64 end record.
const NO_ZIP64_RECORD: &str = "the ZIP64 locator points at no ZIP64 end record";
/// A record, the central directory or a day zip runs past the bytes' end.
const PAST_THE_END: &str = "a record runs past the end of the archive";
/// The central directory is larger than [`CENTRAL_CAP`].
const CENTRAL_OVER_CAP: &str = "the central directory is over its cap";
/// A central-directory record does not start with its signature.
const NO_CENTRAL_SIGNATURE: &str = "a central-directory record has no signature";
/// A record's fixed part, name or extra field is cut short.
const CUT_SHORT: &str = "a zip record is cut short";
/// The records the declared count walks do not end at the declared directory
/// size: a count that stops early would leave records to be read as absent
/// members, and a comment that runs past the end is cut short.
const SIZE_MISMATCH: &str = "the records do not end at the central directory's declared size";
/// A saturated field has no value in a ZIP64 extra block.
const NO_ZIP64_VALUE: &str = "a saturated field has no ZIP64 value";
/// A day zip's local header does not start with its signature.
const NO_LOCAL_SIGNATURE: &str = "a day zip's local header has no signature";
/// A member's local header, inside its day zip, does not start with its
/// signature.
const NO_MEMBER_SIGNATURE: &str = "a member's local header has no signature";

/// The largest central directory read into memory, 64 MiB. The operator's
/// outer archive's is 493,028 bytes for 4,209 entries (its ZIP64 end record,
/// read with `xxd`), so the cap is two orders of magnitude above the one
/// directory that exists and refuses a corrupt size before allocating it.
const CENTRAL_CAP: u64 = 64 << 20;

/// The end record's fixed length.
const END_LEN: usize = 22;

/// How far from the end the end record can start: its fixed length plus the
/// longest comment a 16-bit length can declare, 22 + 65,535. Written as one
/// literal: a sum here could be mutated to a product that only searches
/// further back, which no end record can need.
const TAIL_MAX: u64 = 65_557;

const END_SIGNATURE: u64 = 0x0605_4b50;
const LOCATOR_SIGNATURE: u64 = 0x0706_4b50;
const ZIP64_END_SIGNATURE: u64 = 0x0606_4b50;
const CENTRAL_SIGNATURE: u64 = 0x0201_4b50;
const LOCAL_SIGNATURE: u64 = 0x0403_4b50;

/// The value a 32-bit field holds when its real value is in a ZIP64 block.
const SATURATED_32: u64 = 0xFFFF_FFFF;
const SATURATED_16: u64 = 0xFFFF;

/// A `u64` as an index. On a 32-bit target a value past `usize::MAX` becomes
/// `usize::MAX`, which no slice reaches, so the access that uses it refuses.
fn index(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// The little-endian unsigned integer of `width` bytes at `at`, or `what`.
fn le(bytes: &[u8], at: usize, width: usize, what: &'static str) -> Result<u64, CmRefusal> {
    let field = bytes
        .get(at..)
        .and_then(|rest| rest.get(..width))
        .ok_or(CmRefusal::ArchiveMalformed { what })?;
    let mut wide = [0_u8; 8];
    wide.iter_mut()
        .zip(field)
        .for_each(|(slot, byte)| *slot = *byte);
    Ok(u64::from_le_bytes(wide))
}

/// Whether the four bytes at `at` are `signature`.
fn signed(bytes: &[u8], at: usize, signature: u64) -> bool {
    le(bytes, at, 4, CUT_SHORT).is_ok_and(|found| found == signature)
}

/// `len` bytes of a source starting at `base`: the whole outer archive, or
/// one stored day zip inside it.
struct Span<'a, B> {
    src: &'a B,
    base: u64,
    len: u64,
}

impl<'a, B: ReadAt> Span<'a, B> {
    /// The first `len` bytes of `src`.
    const fn of(src: &'a B, len: u64) -> Self {
        Self { src, base: 0, len }
    }

    /// Whether `n` bytes at `at` lie inside the span.
    fn holds(&self, at: u64, n: u64) -> Result<(), CmRefusal> {
        if at.checked_add(n).is_none_or(|end| end > self.len) {
            return Err(CmRefusal::ArchiveMalformed { what: PAST_THE_END });
        }
        Ok(())
    }

    /// `n` bytes at `at`, refused past the span's end.
    fn read(&self, at: u64, n: u64) -> Result<Vec<u8>, CmRefusal> {
        self.holds(at, n)?;
        let mut buf = vec![0; index(n)];
        self.src
            .read_at(self.base.saturating_add(at), &mut buf)
            .map_err(|why| CmRefusal::ArchiveUnavailable { kind: why.kind() })?;
        Ok(buf)
    }
}

/// One central-directory record, its ZIP64 values applied.
struct Entry {
    name: Vec<u8>,
    method: u64,
    /// The general-purpose flag (APPNOTE 4.4.4): bit 0 is encryption, bit
    /// 3 a data descriptor after the data.
    flags: u16,
    crc32: u32,
    /// Compressed length: the stored bytes, for a stored member.
    packed: u64,
    /// Uncompressed length.
    len: u64,
    /// Where its local header starts.
    offset: u64,
    /// `(date << 16) | time`, the record's DOS date and time (APPNOTE
    /// 4.4.6), which the tick store keeps per entry (`FORMAT.md` §3).
    dos_time: u32,
}

/// The body of the ZIP64 extra block (id 1) in `extra`, or nothing.
fn zip64_block(extra: &[u8]) -> &[u8] {
    let mut rest = extra;
    while let [id_lo, id_hi, size_lo, size_hi, tail @ ..] = rest {
        let size = usize::from(u16::from_le_bytes([*size_lo, *size_hi]));
        let (body, after) = tail.split_at(size.min(tail.len()));
        if u16::from_le_bytes([*id_lo, *id_hi]) == 1 {
            return body;
        }
        rest = after;
    }
    &[]
}

/// The length of a ZIP64 end record with no extensible data: its 12-byte
/// head plus the 44 bytes its size field then states (APPNOTE 4.3.14), as
/// every real container measured writes it (design §8 B7: 56 + 20 = 76
/// bytes before the end record).
const ZIP64_RECORD_LEN: u64 = 56;

/// What a ZIP64 end record of [`ZIP64_RECORD_LEN`] states its own size as.
const ZIP64_RECORD_SIZE_FIELD: u64 = 44;

/// Where the ZIP64 end record starts when it sits, as APPNOTE lays it out,
/// right before the 20-byte locator that precedes the end record at
/// `end_at`, and states the size of a record with no extensible data;
/// `None` otherwise, when the locator's stated offset is the only word on
/// it. Two reads, O(1).
fn zip64_record_at<B: ReadAt>(span: &Span<'_, B>, end_at: u64) -> Result<Option<u64>, CmRefusal> {
    let Some(at) = end_at.checked_sub(20 + ZIP64_RECORD_LEN) else {
        return Ok(None);
    };
    let head = span.read(at, 12)?;
    let found = signed(&head, 0, ZIP64_END_SIGNATURE)
        && le(&head, 4, 8, CUT_SHORT).is_ok_and(|size| size == ZIP64_RECORD_SIZE_FIELD);
    Ok(found.then_some(at))
}

/// Refuses a zip whose central directory, as its end record states it
/// (`offset`, `size`, `count` records), ends before `actual`, where the next
/// record actually starts, AND whose first central record is found that
/// many bytes later (design §8 B7): the bytes between are a prefix its
/// offsets were not adjusted for, [`CmRefusal::ArchivePrefixed`], named
/// before the directory is read. A directory stated short with no record
/// where the prefix would put it is not a prefix: the walk that follows
/// refuses it by what it finds (a record cut short, a size that does not
/// match), as does one stated to run past `actual`. One 4-byte read, O(1).
fn unprefixed<B: ReadAt>(
    span: &Span<'_, B>,
    actual: u64,
    (offset, size, count): (u64, u64, u64),
) -> Result<(), CmRefusal> {
    let Some(bytes) = offset
        .checked_add(size)
        .and_then(|stated| actual.checked_sub(stated))
        .filter(|bytes| *bytes > 0)
    else {
        return Ok(());
    };
    let shifted = offset
        .checked_add(bytes)
        .and_then(|at| span.read(at, 4).ok())
        .is_some_and(|head| signed(&head, 0, CENTRAL_SIGNATURE));
    if count == 0 || shifted {
        return Err(CmRefusal::ArchivePrefixed { bytes });
    }
    Ok(())
}

/// The central directory's `(count, size, offset)` as the end record states
/// them, from the ZIP64 end record when a locator precedes it, with a prefix
/// named before any of them is used (design §8 B7).
///
/// The end record is the last 22 bytes plus a comment; when a ZIP64 locator
/// sits right before it, the ZIP64 end record gives the count, size and
/// offset instead. That is the operator's outer archive (128,918,864,518
/// bytes): its end record saturates the offset; its first 256 central records
/// carry a plain 32-bit local-header offset and an `UT` block only, and the
/// other 3,953 saturate the offset alone and carry it in a ZIP64 block before
/// the `UT` block, which is the layout this reader was written against (its
/// whole central directory walked with `od`, charter GDFL section, D-0812).
fn directory_bounds<B: ReadAt>(span: &Span<'_, B>) -> Result<(u64, u64, u64), CmRefusal> {
    let tail_len = span.len.min(TAIL_MAX);
    let tail_at = span.len - tail_len;
    let tail = span.read(tail_at, tail_len)?;
    let end = (0..tail.len().saturating_sub(END_LEN - 1))
        .rev()
        .find(|&at| {
            signed(&tail, at, END_SIGNATURE)
                && le(&tail, at + 20, 2, CUT_SHORT)
                    .is_ok_and(|comment| at + END_LEN + index(comment) == tail.len())
        })
        .ok_or(CmRefusal::ArchiveMalformed {
            what: NO_END_RECORD,
        })?;
    let mut count = le(&tail, end + 10, 2, CUT_SHORT)?;
    let mut size = le(&tail, end + 12, 4, CUT_SHORT)?;
    let mut offset = le(&tail, end + 16, 4, CUT_SHORT)?;
    let end_at = tail_at + end as u64;
    let locator = match end_at.checked_sub(20) {
        Some(at) => Some(span.read(at, 20)?).filter(|loc| signed(loc, 0, LOCATOR_SIGNATURE)),
        None => None,
    };
    if let Some(locator) = locator {
        // The prefix is measured first, from the ZIP64 end record found where
        // APPNOTE puts it (design §8 B7), so it is named before the
        // locator's stated offset, short by the prefix in a prefixed zip, is
        // used to read the record as before.
        if let Some(found) = zip64_record_at(span, end_at)? {
            let record = span.read(found, ZIP64_RECORD_LEN)?;
            let field = |at| le(&record, at, 8, CUT_SHORT);
            unprefixed(span, found, (field(48)?, field(40)?, field(32)?))?;
        }
        let record = span.read(le(&locator, 8, 8, CUT_SHORT)?, 56)?;
        if !signed(&record, 0, ZIP64_END_SIGNATURE) {
            return Err(CmRefusal::ArchiveMalformed {
                what: NO_ZIP64_RECORD,
            });
        }
        count = le(&record, 32, 8, CUT_SHORT)?;
        size = le(&record, 40, 8, CUT_SHORT)?;
        offset = le(&record, 48, 8, CUT_SHORT)?;
    } else if count == SATURATED_16 || size == SATURATED_32 || offset == SATURATED_32 {
        return Err(CmRefusal::ArchiveMalformed {
            what: SATURATED_WITHOUT_LOCATOR,
        });
    } else {
        unprefixed(span, end_at, (offset, size, count))?;
    }
    Ok((count, size, offset))
}

/// How many entries the end record of the zip of `len` bytes in `source`
/// states its central directory holds (the ZIP64 count when a locator
/// precedes it), without reading the directory: the tail read and at most
/// two ZIP64 reads of [`directory_bounds`], O(1). IA-3b's year-zip check
/// (`year_zip_empty`, RA-322) asks it.
///
/// # Errors
///
/// [`directory_bounds`]' refusals: no end record, a saturated end record with
/// no locator, a prefix, a failed read.
pub fn stated_entries<B: ReadAt>(source: &B, len: u64) -> Result<u64, CmRefusal> {
    directory_bounds(&Span::of(source, len)).map(|(count, _, _)| count)
}

/// Every central-directory record of the zip in `span`, in order, from the
/// bounds [`directory_bounds`] reads. The walked records must end exactly
/// at the declared directory size.
fn central_directory<B: ReadAt>(span: &Span<'_, B>) -> Result<Vec<Entry>, CmRefusal> {
    let (count, size, offset) = directory_bounds(span)?;
    if size > CENTRAL_CAP {
        return Err(CmRefusal::ArchiveMalformed {
            what: CENTRAL_OVER_CAP,
        });
    }
    let directory = span.read(offset, size)?;
    let mut entries = Vec::new();
    let mut at = 0_usize;
    for _ in 0..count {
        let record = directory.get(at..).unwrap_or_default();
        if !signed(record, 0, CENTRAL_SIGNATURE) {
            return Err(CmRefusal::ArchiveMalformed {
                what: NO_CENTRAL_SIGNATURE,
            });
        }
        let field = |at, width| le(record, at, width, CUT_SHORT);
        let name_len = index(field(28, 2)?);
        let extra_len = index(field(30, 2)?);
        let comment_len = index(field(32, 2)?);
        let method = field(10, 2)?;
        let flags = u16::try_from(field(8, 2)?).unwrap_or(u16::MAX);
        let crc32 = u32::try_from(field(16, 4)?).unwrap_or_default();
        let (packed, len, local) = (field(20, 4)?, field(24, 4)?, field(42, 4)?);
        let cut = CmRefusal::ArchiveMalformed { what: CUT_SHORT };
        let name = record.get(46..46 + name_len).ok_or_else(|| cut.clone())?;
        let extra = record
            .get(46 + name_len..46 + name_len + extra_len)
            .ok_or(cut)?;
        // A saturated field's value is the next u64 of the ZIP64 block, in
        // the order uncompressed, compressed, offset (APPNOTE 4.5.3).
        let block = zip64_block(extra);
        let mut taken = 0;
        let mut wide = |value: u64| -> Result<u64, CmRefusal> {
            if value != SATURATED_32 {
                return Ok(value);
            }
            let wide = le(block, taken, 8, NO_ZIP64_VALUE)?;
            taken += 8;
            Ok(wide)
        };
        let len = wide(len)?;
        let packed = wide(packed)?;
        let offset = wide(local)?;
        let dos_time = u32::try_from(field(14, 2)? << 16 | field(12, 2)?).unwrap_or_default();
        entries.push(Entry {
            name: name.to_vec(),
            method,
            flags,
            crc32,
            packed,
            len,
            offset,
            dos_time,
        });
        at += 46 + name_len + extra_len + comment_len;
    }
    // The count and the size are two statements of one directory: records
    // left after the count would be read as absent, and a day zip or member
    // among them would be listed as a day or file the vendor never shipped,
    // for a corrupt archive (round-3 review). The operator's archive walks its
    // 4,209 records to exactly its 493,028 bytes (charter, GDFL section).
    if at != directory.len() {
        return Err(CmRefusal::ArchiveMalformed {
            what: SIZE_MISMATCH,
        });
    }
    Ok(entries)
}

/// The three-letter month the vendor's folders use: `APR_2024`.
pub(crate) const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// `INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_01042024.zip`: where the outer
/// archive files a tree's day zip.
#[must_use]
pub fn day_zip_name(kind: CmKind, day: Day) -> String {
    let tree = match kind {
        CmKind::Indices => "INDICES",
        CmKind::Stocks => "STOCKS",
    };
    let month = MONTHS
        .get(usize::from(day.month()).saturating_sub(1))
        .copied()
        .unwrap_or_default();
    let year = day.year();
    format!(
        "{tree}/{year:04}/{month}_{year:04}/{}.zip",
        crate::gdfl_cm::day_folder_name(kind, day)
    )
}

/// What one central-directory name is to the day table (design §8 C2, C3).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Named {
    /// Exactly a day zip's name: filed where its name says it belongs.
    DayZip(CmKind, Day),
    /// A day zip's file name under another folder than the one its name
    /// files it in, the root included (C3, F9): `day_zip_misfiled`.
    Misfiled(CmKind, Day),
    /// A tree's day-zip prefix and `.zip` around something that is not a
    /// real `ddmmyyyy` (C2): `day_zip_name_invalid`, named in the census.
    Invalid,
    /// Anything else: a folder entry, a stray file.
    Other,
}

/// What `name` is to the day table. A tree's prefix and a real `ddmmyyyy`
/// name a copy of that day wherever it lies and whatever follows the date
/// (`... (1).zip`), as the loose zips' rule does (D-4834); only the exact
/// name at its own place is the day zip. O(the name's length).
fn named(name: &str) -> Named {
    let file = name.rsplit_once('/').map_or(name, |(_, file)| file);
    let Some(stem) = file.strip_suffix(".zip") else {
        return Named::Other;
    };
    for kind in [CmKind::Indices, CmKind::Stocks] {
        let Some(rest) = stem.strip_prefix(kind.descriptor().folder_prefix) else {
            continue;
        };
        let Some(day) = rest.get(..8).and_then(crate::gdfl_cm::ddmmyyyy) else {
            return Named::Invalid;
        };
        return if day_zip_name(kind, day) == name {
            Named::DayZip(kind, day)
        } else {
            Named::Misfiled(kind, day)
        };
    }
    if stem.starts_with("GFDLCM_") {
        Named::Invalid
    } else {
        Named::Other
    }
}

/// Where one stored day zip lies in the outer archive.
#[derive(Debug, Clone, Copy)]
struct Inner {
    /// Its local header's offset.
    header: u64,
    /// Its stored length.
    len: u64,
}

/// How many names of one kind the archive census names before it only
/// counts (D-4817: ≤ 16 named per reason).
pub const CENSUS_NAMED: usize = 16;

/// What the outer archive's names were, beside the day zips it files
/// (design §8 A10, C2, C3; D-4817): counted, the first [`CENSUS_NAMED`] of
/// each named. Report only: the days it concerns are refused by name on
/// their own.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchiveCensus {
    /// Names that are not day zips at all: folder entries, stray files.
    pub ignored: usize,
    /// Day-zip file names under another folder than their own.
    pub misfiled: usize,
    /// The first misfiled names, as found.
    pub misfiled_named: Vec<String>,
    /// Names with a tree's prefix whose date is not a real `ddmmyyyy`.
    pub invalid: usize,
    /// The first invalid names, as found.
    pub invalid_named: Vec<String>,
}

impl ArchiveCensus {
    /// Counts `name` under `count`, naming it while fewer than
    /// [`CENSUS_NAMED`] are named.
    fn note(count: &mut usize, named: &mut Vec<String>, name: &str) {
        *count += 1;
        if named.len() < CENSUS_NAMED {
            named.push(name.to_owned());
        }
    }
}

/// What the day table holds for one tree and day (D-4818): the stored day
/// zip, or the fault of its container that refuses that day only. 24 bytes
/// as `Option<Slot>`, the same as the plain table it replaced, so the
/// table's worst case (`docs/06-limits.md`) is unchanged.
#[derive(Debug, Clone)]
enum Slot {
    /// One stored, readable day zip at its own place.
    Held(Inner),
    /// Two copies of the day, a lone copy under another folder, or a lone
    /// copy the reader cannot take as it is stored.
    Refused(Box<CmRefusal>),
}

/// The one rule every zip source files a copy of a day by (D-4818, design
/// §8 A16, A17, B19, C3): the first copy fills the day's slot, at its own
/// place as the day zip (or the fault that keeps it from being read) and
/// anywhere else as `day_zip_misfiled`, naming where it was found; a second
/// copy, wherever it lies, makes the day `archive_duplicate_day`. Each for
/// that day only.
fn file_copy(cell: &mut Option<Slot>, day: Day, copy: Result<Inner, CmRefusal>) {
    *cell = Some(match cell {
        Some(_) => Slot::Refused(Box::new(CmRefusal::ArchiveDuplicateDay { day })),
        None => copy.map_or_else(|why| Slot::Refused(Box::new(why)), Slot::Held),
    });
}

/// The outer archive, its central directory read once into a dense table of
/// day zips per tree, indexed by days since the archive's first day.
#[derive(Debug)]
pub struct Archive<B> {
    source: B,
    len: u64,
    /// The first day any day zip names, in days since the epoch.
    first: u32,
    /// Day zips of the index tree, then of the stock tree, each the stored
    /// day zip or the fault of its container that refuses that day only
    /// (D-4818).
    days: [Vec<Option<Slot>>; 2],
    /// What the names beside the day zips were.
    census: ArchiveCensus,
    /// What every day listing files (D-3333): the swept surface unless
    /// [`Archive::with_scope`] names another.
    scope: Arc<CmScope>,
    /// Each tree's first and last day zip day, days since the epoch
    /// ([`Archive::span`], IA-3b item 8).
    spans: [Option<(u32, u32)>; 2],
    /// The outer file watched from its directory's read to every later read
    /// (IA-3b item 4, RA-323): `None` for bytes that are not a file.
    watch: Option<Watch>,
}

impl Archive<std::fs::File> {
    /// Opens and indexes the archive at `path`, through one handle every
    /// later read uses, its identity taken as the directory is read and
    /// checked after each day's listing and each member read (IA-3b item 4).
    ///
    /// # Errors
    ///
    /// [`CmRefusal::ArchiveUnavailable`] when the file cannot be opened or
    /// sized, and every refusal of [`Archive::from_source`].
    pub fn open(path: &Path) -> Result<Self, CmRefusal> {
        let unavailable = |why: std::io::Error| CmRefusal::ArchiveUnavailable { kind: why.kind() };
        let file = std::fs::File::open(path).map_err(unavailable)?;
        let watch = Watch::new(path, Arc::new(file.try_clone().map_err(unavailable)?))
            .map_err(unavailable)?;
        let len = watch.listed().len;
        let archive = Self::from_source(file, len)?;
        Ok(Self {
            watch: Some(watch),
            ..archive
        })
    }
}

/// [`Watch::check`] as an archive's refusal: a changed file is
/// [`CmRefusal::SourceChanging`], an unmeasurable one
/// [`CmRefusal::ArchiveUnavailable`], which the run's loss probe asks about.
/// Nothing when there is no file to watch.
pub(crate) fn unchanged(watch: Option<&Watch>) -> Result<(), CmRefusal> {
    match watch.map(Watch::check) {
        None | Some(Ok(())) => Ok(()),
        Some(Err(WatchFault::Changed(why))) => Err(why),
        Some(Err(WatchFault::Unreachable(kind))) => Err(CmRefusal::ArchiveUnavailable { kind }),
    }
}

/// The table slot of `kind` (0 for indices, 1 for stocks).
const fn tree(kind: CmKind) -> usize {
    match kind {
        CmKind::Indices => 0,
        CmKind::Stocks => 1,
    }
}

/// The general-purpose flag's encryption bit (APPNOTE 4.4.4, bit 0).
const ENCRYPTED: u16 = 1;

/// The fault, if any, that keeps a member from being read as it is stored:
/// encryption first, as [`member_bytes`] checks it, then a method other
/// than stored for a day zip read in place.
fn stored_fault(name: &str, method: u64, flags: u16) -> Option<CmRefusal> {
    if flags & ENCRYPTED != 0 {
        return Some(CmRefusal::ArchiveMemberEncrypted {
            member: name.to_owned(),
        });
    }
    (method != 0).then(|| CmRefusal::ArchiveMemberCompressed {
        member: name.to_owned(),
    })
}

impl<B: ReadAt> Archive<B> {
    /// Reads the central directory of `len` bytes of `source` once, O(entries),
    /// and files every day zip by tree and day. A fault of one day zip
    /// refuses that day only, never the archive (D-4818): a compressed or
    /// encrypted day zip, two for one day, or one under another folder than
    /// its name's. Each is answered by [`CmSource::day`] for its day.
    ///
    /// # Errors
    ///
    /// [`CmRefusal::ArchiveUnavailable`] on a failed read,
    /// [`CmRefusal::ArchiveMalformed`] when the bytes are not a zip, and
    /// [`CmRefusal::ArchivePrefixed`] when bytes precede its first record.
    pub fn from_source(source: B, len: u64) -> Result<Self, CmRefusal> {
        let entries = central_directory(&Span::of(&source, len))?;
        // Two passes over the directory already in memory, so the table is
        // sized once and filled in place: no map, no sort (CM-13).
        let days_named =
            entries
                .iter()
                .filter_map(|entry| match named(&String::from_utf8_lossy(&entry.name)) {
                    Named::DayZip(_, day) | Named::Misfiled(_, day) => Some(day.days_from_epoch()),
                    Named::Invalid | Named::Other => None,
                });
        let span = days_named.fold(None, |span: Option<(u32, u32)>, day| {
            Some(span.map_or((day, day), |(lo, hi)| (lo.min(day), hi.max(day))))
        });
        // One cell per day from the first day zip named to the last, none
        // when the archive names none.
        let first = span.map_or(0, |(lo, _)| lo);
        let width = span.map_or(0, |(lo, hi)| index(u64::from(hi - lo)) + 1);
        let mut days = [vec![None; width], vec![None; width]];
        let mut spans: [Option<(u32, u32)>; 2] = [None, None];
        let mut census = ArchiveCensus::default();
        for entry in entries {
            let name = String::from_utf8_lossy(&entry.name);
            let (kind, day, copy) = match named(&name) {
                Named::DayZip(kind, day) => {
                    let copy = stored_fault(&name, entry.method, entry.flags).map_or(
                        Ok(Inner {
                            header: entry.offset,
                            len: entry.packed,
                        }),
                        Err,
                    );
                    (kind, day, copy)
                }
                Named::Misfiled(kind, day) => {
                    let ArchiveCensus {
                        misfiled,
                        misfiled_named,
                        ..
                    } = &mut census;
                    ArchiveCensus::note(misfiled, misfiled_named, &name);
                    let why = CmRefusal::DayZipMisfiled {
                        found: name.clone().into_owned(),
                        expected: day_zip_name(kind, day),
                    };
                    (kind, day, Err(why))
                }
                Named::Invalid => {
                    let ArchiveCensus {
                        invalid,
                        invalid_named,
                        ..
                    } = &mut census;
                    ArchiveCensus::note(invalid, invalid_named, &name);
                    continue;
                }
                Named::Other => {
                    census.ignored += 1;
                    continue;
                }
            };
            let slot = index(u64::from(day.days_from_epoch() - first));
            if let Some(cell) = days
                .get_mut(tree(kind))
                .and_then(|table| table.get_mut(slot))
            {
                file_copy(cell, day, copy);
            }
            // Each tree's span, two comparisons a day zip (IA-3b item 8).
            if let Some(span) = spans.get_mut(tree(kind)) {
                let at = day.days_from_epoch();
                *span = Some(span.map_or((at, at), |(lo, hi)| (lo.min(at), hi.max(at))));
            }
        }
        Ok(Self {
            source,
            len,
            first,
            days,
            census,
            scope: CmScope::swept_shared(),
            spans,
            watch: None,
        })
    }

    /// The first and last day the tree of `kind` holds a day zip for, at
    /// its own place or refused for that day (IA-3b item 8, FM-133): the
    /// source's span, which a plan states so the days outside it read as
    /// `outside_source`, never as holes. `None` when the tree holds none.
    /// O(days in the archive's span), once per run, over the table in
    /// memory.
    #[must_use]
    pub fn span(&self, kind: CmKind) -> Option<(Day, Day)> {
        let (first, last) = (*self.spans.get(tree(kind))?)?;
        Some((Day::from_days(first).ok()?, Day::from_days(last).ok()?))
    }

    /// This archive, its day listings filing what `scope` files (D-3333).
    #[must_use]
    pub fn with_scope(self, scope: Arc<CmScope>) -> Self {
        Self { scope, ..self }
    }

    /// How many names in the archive are not day zips filed where their own
    /// name says: folder entries, stray files, invalid and misfiled day-zip
    /// names. Report only: [`Archive::census`] names them, and a misfiled
    /// one refuses its own day.
    #[must_use]
    pub const fn ignored(&self) -> usize {
        self.census.ignored + self.census.invalid + self.census.misfiled
    }

    /// What the names beside the day zips were (D-4817).
    #[must_use]
    pub const fn census(&self) -> &ArchiveCensus {
        &self.census
    }

    /// Where the day zip of `kind` on `day` lies: one index into the day
    /// table, which holds the day zip or its refusal, then its local header
    /// read and checked.
    fn day_zip_span(&self, kind: CmKind, day: Day) -> Result<Option<Inner>, CmRefusal> {
        let Some(slot) = day
            .days_from_epoch()
            .checked_sub(self.first)
            .map(|slot| index(u64::from(slot)))
        else {
            return Ok(None);
        };
        let inner = match self.days.get(tree(kind)).and_then(|table| table.get(slot)) {
            Some(Some(Slot::Held(inner))) => *inner,
            Some(Some(Slot::Refused(why))) => return Err(CmRefusal::clone(why)),
            Some(None) | None => return Ok(None),
        };
        let outer = Span::of(&self.source, self.len);
        let local = outer.read(inner.header, 30)?;
        if !signed(&local, 0, LOCAL_SIGNATURE) {
            return Err(CmRefusal::ArchiveMalformed {
                what: NO_LOCAL_SIGNATURE,
            });
        }
        let data =
            inner.header + 30 + le(&local, 26, 2, CUT_SHORT)? + le(&local, 28, 2, CUT_SHORT)?;
        outer.holds(data, inner.len)?;
        Ok(Some(Inner {
            header: data,
            len: inner.len,
        }))
    }
}

/// Where one member of a day zip lies, for [`CmSource::fetch`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZipLocator {
    /// Where the day zip's bytes start in the outer archive.
    base: u64,
    /// The day zip's stored length.
    span: u64,
    /// The member's local header, from the day zip's start.
    header: u64,
    /// The member's method, as its central record states.
    method: u16,
    /// The member's compressed length, as its central record states.
    packed: u64,
    /// The member's general-purpose flag, as its central record states:
    /// bit 0 refuses it as encrypted before any byte is inflated.
    flags: u16,
}

/// "a member's data runs past its day zip": the local header's name and
/// extra lengths, or the compressed length, put it there.
const MEMBER_PAST_DAY_ZIP: &str = "a member's data runs past its day zip";

/// The vendor's zips are a [`CmSource`] (D-2800): the outer central directory
/// is read once by [`Archive::from_source`], a day is one index into the day
/// table and one read of that day zip's central directory (O(its entries)),
/// and a file is one local header, one contiguous read of its compressed
/// bytes and one inflate. Nothing is extracted to disk.
impl<B: ReadAt> CmSource for Archive<B> {
    type Locator = ZipLocator;

    /// The scope this source's listings are built under (WP-12f, D-4878).
    fn scope(&self) -> Arc<CmScope> {
        Arc::clone(&self.scope)
    }

    /// The day zip's central directory, every entry in its order with the
    /// size and CRC-32 it states. `None` when the archive holds no day zip
    /// for that tree and day.
    ///
    /// # Errors
    ///
    /// [`CmRefusal::ArchiveMalformed`] when the day zip is not where its
    /// record says or is not a zip, and [`CmRefusal::ArchiveUnavailable`] on
    /// a failed read.
    fn day(&self, kind: CmKind, day: Day) -> Result<Option<DayListing<ZipLocator>>, CmRefusal> {
        let Some(inner) = self.day_zip_span(kind, day)? else {
            return Ok(None);
        };
        let span = Span {
            src: &self.source,
            base: inner.header,
            len: inner.len,
        };
        let mut listing =
            DayListing::scoped(kind, day, Arc::clone(&self.scope)).with_member_bytes(inner.len);
        let entries = central_directory(&span)?;
        unchanged(self.watch.as_ref())?;
        // AN EMPTY DAY ZIP IS NAMED, NEVER A DAY OF NO FILE (IA-3b, RA-322,
        // design §8 B28a): the day stays open.
        if entries.is_empty() {
            return Err(CmRefusal::DayZipEmpty {
                name: format!("{}.zip", crate::gdfl_cm::day_folder_name(kind, day)),
            });
        }
        for entry in entries {
            let locator = ZipLocator {
                base: inner.header,
                span: inner.len,
                header: entry.offset,
                method: u16::try_from(entry.method).unwrap_or(u16::MAX),
                packed: entry.packed,
                flags: entry.flags,
            };
            listing.push(
                &String::from_utf8_lossy(&entry.name),
                entry.len,
                entry.crc32,
                locator,
            );
        }
        Ok(Some(listing))
    }

    /// The member's bytes: stored ones as they are, deflated ones inflated,
    /// at most one byte past the length the central directory states, so a
    /// member that inflates longer is refused by [`crate::gdfl_cm::verify`]
    /// without being inflated whole.
    ///
    /// # Errors
    ///
    /// [`CmRefusal::ArchiveMalformed`] when the member's local header is not
    /// where its record says or its data runs past the day zip,
    /// [`CmRefusal::ArchiveMethodUnknown`] for a method other than stored or
    /// deflated, [`CmRefusal::ArchiveMemberCorrupt`] for a deflate stream that
    /// does not inflate, and [`CmRefusal::ArchiveUnavailable`] on a failed
    /// read.
    fn fetch(
        &self,
        _listing: &DayListing<ZipLocator>,
        file: &ListedFile<ZipLocator>,
    ) -> Result<Vec<u8>, CmRefusal> {
        let bytes = member_bytes(&self.source, file.locator, &file.entry, file.len)?;
        // AFTER EACH READ, THE FILE IS STILL THE ONE LISTED (IA-3b item 4):
        // so the last read of a day is always followed by a check, and a day
        // read from a file that changed under it is never closed.
        unchanged(self.watch.as_ref())?;
        Ok(bytes)
    }
}

/// A member's bytes, read through `at` from `source`: stored ones as they
/// are, deflated ones inflated, at most one byte past `len`, the length the
/// central directory states, so a member that inflates longer is refused by
/// [`crate::gdfl_cm::verify`] without being inflated whole. The one member
/// read every zip source shares: the capital-market day zips here and the
/// options day zips of [`crate::gdfl_nfo`] (D-2806).
///
/// # Errors
///
/// [`CmRefusal::ArchiveMemberEncrypted`] for a member whose flag sets bit 0,
/// before any byte of it is read (D-4818),
/// [`CmRefusal::ArchiveMalformed`] when the member's local header is not
/// where its record says or its data runs past its zip,
/// [`CmRefusal::ArchiveMethodUnknown`] for a method other than stored or
/// deflated, [`CmRefusal::ArchiveMemberCorrupt`] for a deflate stream that
/// does not inflate, and [`CmRefusal::ArchiveUnavailable`] on a failed read.
pub fn member_bytes<B: ReadAt>(
    source: &B,
    at: ZipLocator,
    entry: &str,
    len: u64,
) -> Result<Vec<u8>, CmRefusal> {
    if at.flags & ENCRYPTED != 0 {
        return Err(CmRefusal::ArchiveMemberEncrypted {
            member: entry.to_owned(),
        });
    }
    // THE ONE-MEMBER CAP BEFORE ANY READ (IA-3b, RA-238, design §8 B28): a
    // worker holds one member's packed bytes and their inflate in memory, so a
    // record stating either past `MEMBER_CAP` refuses the member, and its day
    // stays open, before a byte of it is read.
    let size = len.max(at.packed);
    if size > crate::tickstore_write::MEMBER_CAP {
        return Err(CmRefusal::MemberTooLarge {
            name: entry.to_owned(),
            size,
        });
    }
    let span = Span {
        src: source,
        base: at.base,
        len: at.span,
    };
    let local = span.read(at.header, 30)?;
    if !signed(&local, 0, LOCAL_SIGNATURE) {
        return Err(CmRefusal::ArchiveMalformed {
            what: NO_MEMBER_SIGNATURE,
        });
    }
    let data = at.header + 30 + le(&local, 26, 2, CUT_SHORT)? + le(&local, 28, 2, CUT_SHORT)?;
    if span.holds(data, at.packed).is_err() {
        return Err(CmRefusal::ArchiveMalformed {
            what: MEMBER_PAST_DAY_ZIP,
        });
    }
    let packed = span.read(data, at.packed)?;
    let limit = len.saturating_add(1);
    match at.method {
        0 => Ok(packed.into_iter().take(index(limit)).collect()),
        8 => {
            let mut out = Vec::new();
            std::io::Read::read_to_end(
                &mut std::io::Read::take(flate2::read::DeflateDecoder::new(&*packed), limit),
                &mut out,
            )
            .map_err(|_| CmRefusal::ArchiveMemberCorrupt {
                member: entry.to_owned(),
            })?;
            Ok(out)
        }
        _ => Err(CmRefusal::ArchiveMethodUnknown {
            member: entry.to_owned(),
            method: at.method,
        }),
    }
}

/// One central-directory record of a zip, its ZIP64 values applied, for a
/// reader whose zips are not the capital-market day table (D-2806).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipEntry {
    /// The entry's name, byte for byte as the record states it (lossy UTF-8).
    pub name: String,
    /// The entry's uncompressed length.
    pub len: u64,
    /// The entry's CRC-32.
    pub crc32: u32,
    /// Where [`member_bytes`] reads it.
    pub locator: ZipLocator,
    /// The name's bytes exactly as the record states them, for a writer that
    /// must keep them byte for byte (`crate::tickstore_write`, E2E-02).
    pub raw_name: Vec<u8>,
    /// `(date << 16) | time` from the record (APPNOTE 4.4.6).
    pub dos_time: u32,
}

/// Every central record of the zip that occupies `len` bytes at `base` of
/// `source`, in its order: O(entries), the same walk the capital-market day
/// table is built from, with every refusal it names.
///
/// # Errors
///
/// [`CmRefusal::ArchiveMalformed`] when the bytes are not a zip, and
/// [`CmRefusal::ArchiveUnavailable`] on a failed read.
pub fn zip_entries<B: ReadAt>(source: &B, base: u64, len: u64) -> Result<Vec<ZipEntry>, CmRefusal> {
    let span = Span {
        src: source,
        base,
        len,
    };
    Ok(central_directory(&span)?
        .into_iter()
        .map(|entry| ZipEntry {
            name: String::from_utf8_lossy(&entry.name).into_owned(),
            dos_time: entry.dos_time,
            len: entry.len,
            crc32: entry.crc32,
            locator: ZipLocator {
                base,
                span: len,
                header: entry.offset,
                method: u16::try_from(entry.method).unwrap_or(u16::MAX),
                packed: entry.packed,
                flags: entry.flags,
            },
            raw_name: entry.name,
        })
        .collect())
}

/// Where a STORED member's bytes lie in `source`, as `(offset, length)`: its
/// local header read and checked, so a zip filed inside another zip can be
/// listed with [`zip_entries`] in place (D-2806). A compressed member has no
/// such span and is refused by name.
///
/// # Errors
///
/// [`CmRefusal::ArchiveMemberEncrypted`] for an encrypted member (D-4818),
/// [`CmRefusal::ArchiveMemberCompressed`] for a member that is not stored,
/// [`CmRefusal::ArchiveMalformed`] when its local header is not where its
/// record says or its data runs past its zip, and
/// [`CmRefusal::ArchiveUnavailable`] on a failed read.
pub fn stored_span<B: ReadAt>(source: &B, entry: &ZipEntry) -> Result<(u64, u64), CmRefusal> {
    let at = entry.locator;
    if let Some(fault) = stored_fault(&entry.name, u64::from(at.method), at.flags) {
        return Err(fault);
    }
    let span = Span {
        src: source,
        base: at.base,
        len: at.span,
    };
    let local = span.read(at.header, 30)?;
    if !signed(&local, 0, LOCAL_SIGNATURE) {
        return Err(CmRefusal::ArchiveMalformed {
            what: NO_LOCAL_SIGNATURE,
        });
    }
    let data = at.header + 30 + le(&local, 26, 2, CUT_SHORT)? + le(&local, 28, 2, CUT_SHORT)?;
    span.holds(data, at.packed)?;
    Ok((at.base.saturating_add(data), at.packed))
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;
    use crate::gdfl_cm::{Expect, ExtVariant, HEADER_OPEN_INTEREST, decode, read_day, read_listed};
    use brutex_core::instrument::{Exchange, InstrumentKey};

    /// The zip polynomial's check value (IEEE 802.3, reflected `0xEDB88320`):
    /// CRC-32 of the nine ASCII digits is `CBF43926`, and of nothing is zero.
    #[test]
    fn crc32_matches_the_zip_polynomial() {
        // Formatted at run time: gate 1d reads a quoted digit string as a
        // possible path segment.
        let digits = 123_456_789.to_string();
        let one_off = 123_456_788.to_string();
        assert_eq!(crc32(digits.as_bytes()), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
        assert_ne!(crc32(digits.as_bytes()), crc32(one_off.as_bytes()));
    }

    // ── a zip writer, for synthetic archives only ─────────────────────────

    /// Which central-directory fields a written entry saturates to
    /// `0xFFFF_FFFF` and carries in a ZIP64 extra block instead.
    #[derive(Clone, Copy, PartialEq)]
    enum Wide {
        /// None: a plain 32-bit entry.
        No,
        /// The local-header offset only, as 3,953 of the operator's 4,209
        /// outer records do (the rest are plain; charter, GDFL section).
        Offset,
        /// Uncompressed size, compressed size and offset, behind an unrelated
        /// extra block that the reader must step over.
        All,
    }

    struct Put<'a> {
        name: &'a str,
        method: u16,
        data: &'a [u8],
        /// What the central directory declares: the bytes' own CRC and length
        /// for a stored member, anything for a declared-compressed one.
        crc: u32,
        len: u64,
        /// The central record's comment, which the reader must step over.
        comment: &'a str,
        /// The general-purpose flag both headers state (bit 11, UTF-8, is
        /// added to the central one as the vendor's writer sets it).
        flags: u16,
        /// A data descriptor after the data (bit 3), the local header's CRC
        /// and sizes then deliberately wrong, as every outer day zip of the
        /// operator's archives writes them zero (design §8 B13).
        descriptor: bool,
    }

    fn put<'a>(name: &'a str, data: &'a [u8]) -> Put<'a> {
        Put {
            name,
            method: 0,
            data,
            crc: crc32(data),
            len: data.len() as u64,
            comment: "",
            flags: 0,
            descriptor: false,
        }
    }

    fn u16le(v: usize) -> [u8; 2] {
        u16::try_from(v).unwrap().to_le_bytes()
    }

    fn u32le(v: u64) -> [u8; 4] {
        u32::try_from(v).unwrap().to_le_bytes()
    }

    /// The extended-timestamp block a wide entry's local header carries.
    const WIDE_LOCAL_EXTRA: [u8; 9] = [0x55, 0x54, 5, 0, 1, 0, 0, 0, 0];

    /// A zip of `entries`, with a ZIP64 end record and locator when `zip64`.
    fn zip(entries: &[Put<'_>], wide: Wide, zip64: bool) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for entry in entries {
            let offset = out.len() as u64;
            let local_flags = entry.flags | if entry.descriptor { 8 } else { 0 };
            out.extend(0x0403_4b50_u32.to_le_bytes());
            out.extend([20, 0]);
            out.extend(local_flags.to_le_bytes());
            out.extend(entry.method.to_le_bytes());
            out.extend([0; 4]);
            if entry.descriptor {
                // Deliberately wrong: a reader that used these would read
                // one byte, or none, of the member.
                out.extend(0xDEAD_BEEF_u32.to_le_bytes());
                out.extend(1_u32.to_le_bytes());
                out.extend(0_u32.to_le_bytes());
            } else {
                out.extend(entry.crc.to_le_bytes());
                out.extend(u32le(entry.data.len() as u64));
                out.extend(u32le(entry.len));
            }
            out.extend(u16le(entry.name.len()));
            // A wide entry's local header carries an extended-timestamp block
            // as Info-ZIP writes one, so the data starts after the name AND
            // the local extra field, which the reader must step over.
            let local_extra: &[u8] = if wide == Wide::No {
                &[]
            } else {
                &WIDE_LOCAL_EXTRA
            };
            out.extend(u16le(local_extra.len()));
            out.extend(entry.name.as_bytes());
            out.extend(local_extra);
            out.extend(entry.data);
            if entry.descriptor {
                out.extend(0x0807_4b50_u32.to_le_bytes());
                out.extend(entry.crc.to_le_bytes());
                out.extend(u32le(entry.data.len() as u64));
                out.extend(u32le(entry.len));
            }
            let saturated = 0xFFFF_FFFF_u64;
            let (size, comp, at, extra): (u64, u64, u64, Vec<u8>) = match wide {
                Wide::No => (entry.len, entry.data.len() as u64, offset, Vec::new()),
                Wide::Offset => {
                    let mut extra = vec![1, 0, 8, 0];
                    extra.extend(offset.to_le_bytes());
                    extra.extend([0x55, 0x54, 1, 0, 0]);
                    (entry.len, entry.data.len() as u64, saturated, extra)
                }
                Wide::All => {
                    let mut extra = vec![0x55, 0x54, 1, 0, 0, 1, 0, 24, 0];
                    extra.extend(entry.len.to_le_bytes());
                    extra.extend((entry.data.len() as u64).to_le_bytes());
                    extra.extend(offset.to_le_bytes());
                    (saturated, saturated, saturated, extra)
                }
            };
            central.extend(0x0201_4b50_u32.to_le_bytes());
            central.extend([45, 3, 45, 0]);
            central.extend((local_flags | 0x0800).to_le_bytes());
            central.extend(entry.method.to_le_bytes());
            central.extend([0; 4]);
            central.extend(entry.crc.to_le_bytes());
            central.extend(u32le(comp));
            central.extend(u32le(size));
            central.extend(u16le(entry.name.len()));
            central.extend(u16le(extra.len()));
            central.extend(u16le(entry.comment.len()));
            central.extend([0; 4]);
            central.extend([0, 0, 0xA4, 0x81]);
            central.extend(u32le(at));
            central.extend(entry.name.as_bytes());
            central.extend(extra);
            central.extend(entry.comment.as_bytes());
        }
        let cd_at = out.len() as u64;
        let cd_len = central.len() as u64;
        let count = entries.len();
        out.extend(central);
        if zip64 {
            let record = out.len() as u64;
            out.extend(0x0606_4b50_u32.to_le_bytes());
            out.extend(44_u64.to_le_bytes());
            out.extend([45, 0, 45, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend((count as u64).to_le_bytes());
            out.extend((count as u64).to_le_bytes());
            out.extend(cd_len.to_le_bytes());
            out.extend(cd_at.to_le_bytes());
            out.extend(0x0706_4b50_u32.to_le_bytes());
            out.extend([0; 4]);
            out.extend(record.to_le_bytes());
            out.extend(1_u32.to_le_bytes());
        }
        out.extend(0x0605_4b50_u32.to_le_bytes());
        out.extend([0; 4]);
        out.extend(u16le(count));
        out.extend(u16le(count));
        out.extend(u32le(cd_len));
        out.extend(if zip64 { [0xFF; 4] } else { u32le(cd_at) });
        out.extend([0, 0]);
        out
    }

    // ── synthetic day files (invented values; see gdfl_cm's tests) ────────

    fn row(time: &str, ltp: &str) -> String {
        [
            "NIFTY 50.NSE_IDX",
            "01/04/2024",
            time,
            ltp,
            "0",
            "0",
            "0",
            "0",
            "0",
            "0",
        ]
        .join(",")
    }

    /// A NIFTY 50 day: a session row and two post-close rows, and no final
    /// newline after the last.
    fn csv() -> Vec<u8> {
        [
            HEADER_OPEN_INTEREST.to_owned(),
            row("09:15:00", "10000"),
            row("16:00:00", "10001.5"),
            row("16:00:01", "10002.5"),
        ]
        .join("\r\n")
        .into_bytes()
    }

    fn at() -> Day {
        Day::new(2024, 4, 1).unwrap()
    }

    fn nifty_expect() -> Expect<'static> {
        Expect {
            kind: CmKind::Indices,
            stem: "NIFTY 50.NSE_IDX",
            day: at(),
        }
    }

    fn nifty_key() -> InstrumentKey {
        InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap()
    }

    const MEMBER: &str = "GFDLCM_INDICES_TICK_01042024/NIFTY 50.NSE_IDX.CSV";
    const DAY_ZIP: &str = "INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_01042024.zip";

    /// `raw` deflated, as the vendor's day zips hold their members.
    fn deflate(raw: &[u8]) -> Vec<u8> {
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, raw).unwrap();
        encoder.finish().unwrap()
    }

    /// A deflated member named `name` whose bytes are `packed` (`deflate` of
    /// `raw`), with the CRC-32 and length of `raw` stated.
    fn deflated<'a>(name: &'a str, packed: &'a [u8], raw: &[u8]) -> Put<'a> {
        Put {
            name,
            method: 8,
            data: packed,
            crc: crc32(raw),
            len: raw.len() as u64,
            comment: "",
            flags: 0,
            descriptor: false,
        }
    }

    /// The inner day zip: a folder entry and the file DEFLATED, as the
    /// vendor's are.
    fn inner(raw: &[u8]) -> Vec<u8> {
        let packed = deflate(raw);
        zip(
            &[
                put("GFDLCM_INDICES_TICK_01042024/", b""),
                deflated(MEMBER, &packed, raw),
            ],
            Wide::No,
            false,
        )
    }

    /// The outer archive as the operator's is laid out: ZIP64, every member
    /// stored, folder entries beside the day zips.
    fn outer(day_zip: &[u8]) -> Vec<u8> {
        zip(
            &[put("INDICES/", b""), put(DAY_ZIP, day_zip)],
            Wide::Offset,
            true,
        )
    }

    fn archive(bytes: Vec<u8>) -> Result<Archive<Vec<u8>>, CmRefusal> {
        let len = bytes.len() as u64;
        Archive::from_source(bytes, len)
    }

    /// The listing of a day the archive holds.
    fn listed<B: ReadAt>(archive: &Archive<B>, kind: CmKind, day: Day) -> DayListing<ZipLocator> {
        archive.day(kind, day).unwrap().unwrap()
    }

    /// The length and CRC-32 the listing states for `file` inside the day
    /// folder, found by its exact name; `None` when no entry of that exact
    /// name is filed.
    fn stated(day: &DayListing<ZipLocator>, file: &str) -> Result<Option<(u64, u32)>, CmRefusal> {
        let stem = file.rsplit_once('.').map_or("", |(stem, _)| stem);
        Ok(day
            .locate(stem)?
            .filter(|(listed, _)| listed.name() == file)
            .map(|(listed, _)| (listed.len, listed.crc32)))
    }

    /// The bytes the archive fetches for NIFTY 50 on `at()`, unverified.
    fn fetched(archive: &Archive<Vec<u8>>) -> Result<Vec<u8>, CmRefusal> {
        let day = listed(archive, CmKind::Indices, at());
        let (file, _) = day.locate("NIFTY 50.NSE_IDX").unwrap().unwrap();
        archive.fetch(&day, file)
    }

    // ── reading a member ───────────────────────────────────────────────────

    /// A deflated member is inflated in memory to exactly the vendor's bytes,
    /// a complete file without a final newline included, and `read_day`
    /// over the archive decodes it as `decode` does; nothing touches the
    /// disk.
    #[test]
    fn a_deflated_member_is_inflated_and_read_end_to_end() {
        let bytes = csv();
        assert!(!bytes.ends_with(b"\n"));
        let archive = archive(outer(&inner(&bytes))).unwrap();
        assert_eq!(archive.ignored(), 1, "the INDICES/ folder entry");
        let day = listed(&archive, CmKind::Indices, at());
        assert_eq!(
            stated(&day, "NIFTY 50.NSE_IDX.CSV"),
            Ok(Some((bytes.len() as u64, crc32(&bytes))))
        );
        assert_eq!(fetched(&archive), Ok(bytes.clone()));
        let read = read_day(&archive, &nifty_key(), at()).unwrap().unwrap();
        assert_eq!(read.name, "NIFTY 50.NSE_IDX.CSV");
        assert_eq!(read.ext, ExtVariant::Upper);
        assert_eq!(read.file, decode(&bytes, &nifty_expect()).unwrap());
        assert_eq!(
            read_listed(&archive, &day, &nifty_key()),
            read_day(&archive, &nifty_key(), at())
        );
    }

    /// A stored (method 0) member is handed back as it is, and a member's
    /// data starts after its local header's name AND extra field, whose
    /// lengths the local header states (a wide entry carries a 9-byte
    /// extended-timestamp block there), with ZIP64 sizes read from the
    /// central record when it saturates them.
    #[test]
    fn a_stored_member_is_read_as_it_is() {
        let bytes = csv();
        for wide in [Wide::No, Wide::Offset, Wide::All] {
            let day_zip = zip(&[put(MEMBER, &bytes)], wide, wide == Wide::All);
            let archive = archive(outer(&day_zip)).unwrap();
            assert_eq!(fetched(&archive), Ok(bytes.clone()));
            assert!(read_day(&archive, &nifty_key(), at()).unwrap().is_some());
        }
    }

    /// The central directory's length and CRC-32 are what the inflated bytes
    /// are checked against: a member stated one byte longer or shorter than
    /// it inflates, or with another CRC-32, is refused before anything
    /// decodes it, and one that inflates longer is stopped one byte past its
    /// stated length.
    #[test]
    fn a_member_that_is_not_what_its_record_states_is_refused() {
        let whole = csv();
        let packed = deflate(&whole);
        let refused = |len: u64, crc: u32| {
            let mut member = deflated(MEMBER, &packed, &whole);
            member.len = len;
            member.crc = crc;
            let archive = archive(outer(&zip(&[member], Wide::No, false))).unwrap();
            (fetched(&archive), read_day(&archive, &nifty_key(), at()))
        };
        let n = whole.len() as u64;
        let (bytes, read) = refused(n - 1, crc32(&whole));
        assert_eq!(
            bytes.unwrap().len() as u64,
            n,
            "stopped one byte past n - 1"
        );
        assert_eq!(
            read,
            Err(CmRefusal::SourceLengthMismatch {
                file: n,
                member: n - 1
            })
        );
        let (bytes, read) = refused(n + 1, crc32(&whole));
        assert_eq!(bytes, Ok(whole.clone()));
        assert_eq!(
            read,
            Err(CmRefusal::SourceLengthMismatch {
                file: n,
                member: n + 1
            })
        );
        let (_, read) = refused(n, crc32(&whole) ^ 1);
        assert_eq!(
            read,
            Err(CmRefusal::SourceCrcMismatch {
                file: crc32(&whole),
                member: crc32(&whole) ^ 1
            })
        );
        let stored = {
            let mut member = put(MEMBER, &whole);
            member.len = 3;
            archive(outer(&zip(&[member], Wide::No, false))).unwrap()
        };
        assert_eq!(
            fetched(&stored).unwrap().len(),
            4,
            "stored: stopped at 3 + 1"
        );
    }

    /// A member whose method is neither stored nor deflated, or whose deflate
    /// stream is broken, is refused by name; nothing is guessed.
    #[test]
    fn a_member_that_does_not_inflate_is_refused() {
        let whole = csv();
        let mut odd = put(MEMBER, &whole);
        odd.method = 12;
        let archive_odd = archive(outer(&zip(&[odd], Wide::No, false))).unwrap();
        assert_eq!(
            fetched(&archive_odd),
            Err(CmRefusal::ArchiveMethodUnknown {
                member: MEMBER.to_owned(),
                method: 12
            })
        );
        let mut packed = deflate(&whole);
        packed[0] = 0xFF;
        let broken = deflated(MEMBER, &packed, &whole);
        let archive_broken = archive(outer(&zip(&[broken], Wide::No, false))).unwrap();
        assert_eq!(
            fetched(&archive_broken),
            Err(CmRefusal::ArchiveMemberCorrupt {
                member: MEMBER.to_owned()
            })
        );
        assert_eq!(
            read_day(&archive_broken, &nifty_key(), at()),
            Err(CmRefusal::ArchiveMemberCorrupt {
                member: MEMBER.to_owned()
            })
        );
    }

    /// A member's local header must be where its central record says, and
    /// its data must lie inside its day zip.
    #[test]
    fn a_member_that_is_not_where_its_record_says_is_refused() {
        let whole = csv();
        let day_zip = zip(&[put(MEMBER, &whole)], Wide::No, false);
        let mut moved = day_zip.clone();
        patch(&mut moved, 0, b"PK\x03\x05");
        assert_eq!(
            fetched(&archive(outer(&moved)).unwrap()),
            Err(bad(NO_MEMBER_SIGNATURE))
        );
        // The local header's name length, pointed past the day zip's end.
        let mut long = day_zip;
        patch(&mut long, 26, &u16le(60_000));
        assert_eq!(
            fetched(&archive(outer(&long)).unwrap()),
            Err(bad(MEMBER_PAST_DAY_ZIP))
        );
    }

    /// The next day's day zip: its own name, holding the same bytes, so a
    /// test can show a fault of one day leaves the next one readable.
    const NEXT_ZIP: &str = "INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_02042024.zip";

    fn next_day() -> Day {
        Day::new(2024, 4, 2).unwrap()
    }

    /// B8 (D-4818, F13): a day zip the outer archive DEFLATES is refused for
    /// its own day, by name, never for the whole archive; the next day
    /// reads, and the refusal never closes the day it names.
    #[test]
    fn outer_member_not_stored_refuses() {
        let day_zip = inner(&csv());
        let mut entry = put(DAY_ZIP, &day_zip);
        entry.method = 8;
        let archive = archive(zip(&[entry, put(NEXT_ZIP, &day_zip)], Wide::Offset, true)).unwrap();
        let why = archive.day(CmKind::Indices, at()).unwrap_err();
        assert_eq!(
            why,
            CmRefusal::ArchiveMemberCompressed {
                member: DAY_ZIP.to_owned()
            }
        );
        assert!(!crate::gdfl_import::cm_cause(&why).closes);
        assert!(archive.day(CmKind::Indices, next_day()).unwrap().is_some());
    }

    /// A day the archive holds no day zip for is `None`, a day the vendor
    /// never shipped, whichever side of the archive's days it falls and
    /// whichever tree; a file the day zip does not hold is `None` too, and a
    /// file is found by its exact bytes.
    #[test]
    fn a_day_or_file_absent_from_the_archive_is_none() {
        let archive = archive(outer(&inner(&csv()))).unwrap();
        for (kind, day) in [
            (CmKind::Indices, Day::new(2024, 4, 2).unwrap()),
            (CmKind::Indices, Day::new(2024, 3, 28).unwrap()),
            (CmKind::Stocks, at()),
        ] {
            assert_eq!(archive.day(kind, day), Ok(None), "{kind:?} {day}");
        }
        let day = listed(&archive, CmKind::Indices, at());
        assert_eq!(stated(&day, "NIFTY BANK.NSE_IDX.csv"), Ok(None));
        assert_eq!(stated(&day, "nifty 50.NSE_IDX.CSV"), Ok(None));
        let bank = InstrumentKey::index(Exchange::Nse, "BANKNIFTY").unwrap();
        assert_eq!(read_day(&archive, &bank, at()), Ok(None));
        let next = Day::new(2024, 4, 2).unwrap();
        assert_eq!(read_day(&archive, &nifty_key(), next), Ok(None));
    }

    /// B19 (D-4818, F38, FM-044): two day zips for one day refuse that day
    /// only, as a duplicate that never closes it; the archive opens and the
    /// next day reads.
    #[test]
    fn two_members_for_one_day_refuse() {
        let day_zip = inner(&csv());
        let archive = archive(zip(
            &[
                put(DAY_ZIP, &day_zip),
                put(DAY_ZIP, &day_zip),
                put(NEXT_ZIP, &day_zip),
            ],
            Wide::Offset,
            true,
        ))
        .unwrap();
        let why = archive.day(CmKind::Indices, at()).unwrap_err();
        assert_eq!(why, CmRefusal::ArchiveDuplicateDay { day: at() });
        let cause = crate::gdfl_import::cm_cause(&why);
        assert_eq!((cause.code, cause.closes), ("archive_duplicate_day", false));
        assert!(archive.day(CmKind::Indices, next_day()).unwrap().is_some());
        assert_eq!(read_day(&archive, &nifty_key(), at()).err(), Some(why));
    }

    /// The dense day table's worst case, the figure `docs/06-limits.md`
    /// states: a corrupt archive naming one day zip at 1970-01-01 and one at
    /// 9999-12-31 (the span `Day` admits) sizes each tree's table at
    /// 2,932,897 cells of 24 bytes, about 70 MB a tree and 141 MB for both,
    /// before any other check. Bounded, so never a crash (round-2 review).
    #[test]
    fn the_day_table_worst_case_is_the_whole_day_range() {
        let first = Day::new(1_970, 1, 1).unwrap().days_from_epoch();
        let last = Day::new(9_999, 12, 31).unwrap().days_from_epoch();
        assert_eq!((first, last - first + 1), (0, 2_932_897));
        assert_eq!(std::mem::size_of::<Option<Slot>>(), 24);
    }

    /// Only `<TREE>/<yyyy>/<MON>_<yyyy>/GFDLCM_<TREE>_TICK_<ddmmyyyy>.zip`
    /// whose parts agree is a day zip; everything else is counted and ignored,
    /// a compressed one included. Days far apart share one dense table.
    #[test]
    fn names_that_are_not_day_zips_are_ignored_and_counted() {
        let day_zip = inner(&csv());
        let stock_zip = zip(
            &[put("GFDLCM_STOCK_TICK_15052026/SBIN.NSE.csv", b"x")],
            Wide::No,
            false,
        );
        let mut txt = put(
            "INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_01042024.txt",
            b"",
        );
        txt.comment = "a comment before the day zips";
        let mut compressed = put("INDICES/2024/APR_2024/readme.zip", b"");
        compressed.method = 8;
        let entries = [
            put("INDICES/", b""),
            txt,
            put("STOCKS/2024/APR_2024/GFDLCM_INDICES_TICK_01042024.zip", b""),
            put(
                "INDICES/2025/APR_2024/GFDLCM_INDICES_TICK_01042024.zip",
                b"",
            ),
            put(
                "INDICES/2024/MAY_2024/GFDLCM_INDICES_TICK_01042024.zip",
                b"",
            ),
            put(
                "INDICES/2024/APR_2025/GFDLCM_INDICES_TICK_01042024.zip",
                b"",
            ),
            put("GFDLCM_INDICES_TICK_01042024.zip", b""),
            put(
                "INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_31042024.zip",
                b"",
            ),
            compressed,
            put(DAY_ZIP, &day_zip),
            put(
                "STOCKS/2026/MAY_2026/GFDLCM_STOCK_TICK_15052026.zip",
                &stock_zip,
            ),
        ];
        let archive = archive(zip(&entries, Wide::Offset, true)).unwrap();
        assert_eq!(archive.ignored(), 9);
        // Five copies of the day under another folder (C3, F9): counted and
        // named; one invalid date (C2); three that are no day zip at all.
        let census = archive.census();
        assert_eq!((census.misfiled, census.invalid, census.ignored), (5, 1, 3));
        assert_eq!(
            census.misfiled_named.first().map(String::as_str),
            Some("STOCKS/2024/APR_2024/GFDLCM_INDICES_TICK_01042024.zip")
        );
        assert_eq!(
            census.invalid_named,
            ["INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_31042024.zip"]
        );
        let may = Day::new(2026, 5, 15).unwrap();
        let span = may.days_from_epoch() - at().days_from_epoch() + 1;
        assert_eq!(archive.first, at().days_from_epoch());
        assert_eq!(
            archive.days.each_ref().map(Vec::len),
            [usize::try_from(span).unwrap(); 2],
            "one cell per day from the first day zip to the last, no wider"
        );
        // Six copies of one day refuse that day as a duplicate (D-4818),
        // and only that day: the stock tree's day still lists.
        assert_eq!(
            archive.day(CmKind::Indices, at()).err(),
            Some(CmRefusal::ArchiveDuplicateDay { day: at() })
        );
        assert_eq!(
            stated(&listed(&archive, CmKind::Stocks, may), "SBIN.NSE.csv"),
            Ok(Some((1, crc32(b"x"))))
        );
        assert_eq!(archive.day(CmKind::Indices, may), Ok(None));
    }

    /// A member is found by its exact name only. A name whose stem is no
    /// swept ticker (INDIA VIX, a folder entry, a member outside the day
    /// folder) is counted and never filed, and asking for the other spelling
    /// of a filed member's extension, or for a name with no extension, is
    /// `ArchiveMemberMissing`.
    #[test]
    fn a_member_is_found_by_its_exact_name_and_every_other_name_is_counted() {
        let day_zip = zip(
            &[
                put("GFDLCM_INDICES_TICK_01042024/", b""),
                put(MEMBER, b"a"),
                put("GFDLCM_INDICES_TICK_01042024/INDIA VIX.NSE_IDX.csv", b"b"),
                put("ELSEWHERE/NIFTY BANK.NSE_IDX.csv", b"Z"),
            ],
            Wide::No,
            false,
        );
        let day = listed(&archive(outer(&day_zip)).unwrap(), CmKind::Indices, at());
        assert_eq!(day.unresolved(), 3, "folder entry, VIX, outside the folder");
        assert_eq!(stated(&day, "NIFTY 50.NSE_IDX.CSV").unwrap().unwrap().0, 1);
        for file in [
            "NIFTY 50.NSE_IDX.csv",
            "NIFTY BANK.NSE_IDX.csv",
            "INDIA VIX.NSE_IDX.csv",
            "NIFTY 50",
        ] {
            assert_eq!(stated(&day, file), Ok(None), "{file}");
        }
    }

    /// CM-13: a member at the root of a day zip, outside its day folder, is
    /// counted and never filed, even when its bare name is a swept ticker's
    /// file; only `<folder>/<file>` is that ticker's member.
    #[test]
    fn a_member_outside_its_day_folder_is_counted_never_filed() {
        let root = zip(&[put("NIFTY 50.NSE_IDX.CSV", b"a")], Wide::No, false);
        let day = listed(&archive(outer(&root)).unwrap(), CmKind::Indices, at());
        assert_eq!(day.unresolved(), 1, "the root-level member");
        assert_eq!(stated(&day, "NIFTY 50.NSE_IDX.CSV"), Ok(None));
        // Beside the real member, the root-level twin neither shadows it nor
        // makes the ticker ambiguous.
        let both = zip(
            &[put("NIFTY 50.NSE_IDX.CSV", b"ROOT"), put(MEMBER, b"a")],
            Wide::No,
            false,
        );
        let day = listed(&archive(outer(&both)).unwrap(), CmKind::Indices, at());
        assert_eq!(day.unresolved(), 1);
        assert_eq!(stated(&day, "NIFTY 50.NSE_IDX.CSV").unwrap().unwrap().0, 1);
    }

    /// CM-10: every one of the twelve month folders is spelled as the
    /// vendor's tree spells it (`ls INDICES/2019` lists `JAN_2019` to
    /// `DEC_2019`), and each day zip name parses back to its own tree and day.
    #[test]
    fn every_month_folder_is_spelled_as_the_vendor_spells_it() {
        let months = [
            "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
        ];
        for (month, mon) in (1_u8..=12).zip(months) {
            let day = Day::new(2019, month, 1).unwrap();
            for (kind, tree, prefix) in [
                (CmKind::Indices, "INDICES", "GFDLCM_INDICES_TICK_"),
                (CmKind::Stocks, "STOCKS", "GFDLCM_STOCK_TICK_"),
            ] {
                let name = day_zip_name(kind, day);
                assert_eq!(
                    name,
                    format!("{tree}/2019/{mon}_2019/{prefix}01{month:02}2019.zip")
                );
                assert_eq!(named(&name), Named::DayZip(kind, day), "{name}");
            }
        }
    }

    /// Design §3.3, revision 9: a member under an index day zip is probed
    /// only against index keys, so a share's file there is counted and never
    /// filed, whatever its stem; and a member under a stock day zip only
    /// against share keys.
    #[test]
    fn members_resolve_only_in_day_zips_of_their_kind() {
        let day_zip = zip(
            &[
                put(MEMBER, b"a"),
                put("GFDLCM_INDICES_TICK_01042024/RELIANCE.NSE.csv", b"b"),
            ],
            Wide::No,
            false,
        );
        let day = listed(&archive(outer(&day_zip)).unwrap(), CmKind::Indices, at());
        assert_eq!(day.unresolved(), 1, "the share's file in an index day zip");
        assert_eq!(stated(&day, "RELIANCE.NSE.csv"), Ok(None));
        assert_eq!(stated(&day, "NIFTY 50.NSE_IDX.CSV").unwrap().unwrap().0, 1);

        // The other direction (round-3 review): a stock day zip keeps its
        // kind, so a share's file is filed and an index's file is counted.
        let stock_zip = zip(
            &[
                put("GFDLCM_STOCK_TICK_01042024/RELIANCE.NSE.csv", b"abc"),
                put("GFDLCM_STOCK_TICK_01042024/NIFTY 50.NSE_IDX.csv", b"D"),
            ],
            Wide::No,
            false,
        );
        let stocks = listed(
            &archive(zip(
                &[put(
                    "STOCKS/2024/APR_2024/GFDLCM_STOCK_TICK_01042024.zip",
                    &stock_zip,
                )],
                Wide::Offset,
                true,
            ))
            .unwrap(),
            CmKind::Stocks,
            at(),
        );
        assert_eq!(
            stocks.unresolved(),
            1,
            "the index's file in a stock day zip"
        );
        assert_eq!(stated(&stocks, "RELIANCE.NSE.csv").unwrap().unwrap().0, 3);
        assert_eq!(stated(&stocks, "NIFTY 50.NSE_IDX.csv"), Ok(None));
    }

    #[test]
    fn an_empty_archive_holds_no_day() {
        let archive = archive(zip(&[], Wide::No, false)).unwrap();
        assert_eq!(archive.ignored(), 0);
        assert_eq!(archive.day(CmKind::Indices, at()), Ok(None));
    }

    // ── availability ───────────────────────────────────────────────────────

    struct Broken;

    impl ReadAt for Broken {
        fn read_at(&self, _offset: u64, _buf: &mut [u8]) -> std::io::Result<()> {
            Err(std::io::ErrorKind::Interrupted.into())
        }
    }

    #[test]
    fn archive_unavailable_refuses() {
        let absent =
            std::env::temp_dir().join(format!("gdfl_archive_absent_{}.zip", std::process::id()));
        assert_eq!(
            Archive::open(&absent).err(),
            Some(CmRefusal::ArchiveUnavailable {
                kind: std::io::ErrorKind::NotFound
            })
        );
        assert_eq!(
            Archive::from_source(Broken, 100).err(),
            Some(CmRefusal::ArchiveUnavailable {
                kind: std::io::ErrorKind::Interrupted
            })
        );
    }

    #[test]
    fn an_archive_on_disk_is_read_through_the_file() {
        let path =
            std::env::temp_dir().join(format!("gdfl_archive_on_disk_{}.zip", std::process::id()));
        std::fs::write(&path, outer(&inner(&csv()))).unwrap();
        let archive = Archive::open(&path).unwrap();
        assert!(read_day(&archive, &nifty_key(), at()).unwrap().is_some());
        std::fs::remove_file(&path).unwrap();
    }

    /// Design §3.3 asks for `read_at`: a positional read, so one `Archive`
    /// shared by many threads never has one thread's read land at another's
    /// offset. A day zip whose own directory sits behind a large stored member
    /// makes every `day` call several reads far apart.
    #[test]
    fn concurrent_day_reads_of_one_file_never_interfere() {
        let pad = vec![b'P'; 300_000];
        let mut entries = vec![put("GFDLCM_INDICES_TICK_01042024/PAD", &pad)];
        entries.push(put(MEMBER, b"a"));
        let day_zip = zip(&entries, Wide::No, false);
        let path =
            std::env::temp_dir().join(format!("gdfl_archive_threads_{}.zip", std::process::id()));
        std::fs::write(&path, outer(&day_zip)).unwrap();
        let archive = Archive::open(&path).unwrap();
        let failures = std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    for _ in 0..100 {
                        let found = archive.day(CmKind::Indices, at()).map(|day| {
                            day.and_then(|day| stated(&day, "NIFTY 50.NSE_IDX.CSV").ok().flatten())
                        });
                        let wrong = found != Ok(Some((1, crc32(b"a"))));
                        // Counted without a branch, so the test has no arm
                        // that only a failing run reaches.
                        failures
                            .fetch_add(usize::from(wrong), std::sync::atomic::Ordering::Relaxed);
                    }
                });
            }
        });
        std::fs::remove_file(&path).unwrap();
        assert_eq!(failures.into_inner(), 0, "of 800 concurrent reads");
    }

    #[test]
    fn a_read_past_the_end_of_bytes_is_an_error() {
        let bytes = vec![1_u8, 2, 3];
        let mut two = [0; 2];
        assert!(bytes.read_at(1, &mut two).is_ok());
        assert_eq!(two, [2, 3]);
        assert!(bytes.read_at(2, &mut two).is_err());
        assert!(bytes.read_at(u64::MAX, &mut two).is_err());
    }

    // ── malformed archives ─────────────────────────────────────────────────

    fn malformed(bytes: Vec<u8>) -> CmRefusal {
        archive(bytes).err().unwrap()
    }

    /// The refusal for a malformed archive saying `what`.
    const fn bad(what: &'static str) -> CmRefusal {
        CmRefusal::ArchiveMalformed { what }
    }

    /// Where the end record of `bytes` starts: the last 22 bytes, no comment.
    fn eocd(bytes: &[u8]) -> usize {
        bytes.len() - 22
    }

    fn patch(bytes: &mut [u8], at: usize, with: &[u8]) {
        bytes[at..at + with.len()].copy_from_slice(with);
    }

    #[test]
    fn no_end_record_refuses() {
        for bytes in [Vec::new(), vec![0; 21], vec![7; 400]] {
            assert_eq!(malformed(bytes), bad(NO_END_RECORD));
        }
        let mut commented = zip(&[], Wide::No, false);
        patch(&mut commented, 20, &[1, 0]);
        assert_eq!(
            malformed(commented.clone()),
            bad(NO_END_RECORD),
            "a comment length that does not reach the end"
        );
        commented.push(b'c');
        assert!(archive(commented).is_ok(), "a one-byte comment that does");
        let mut longest = zip(&[], Wide::No, false);
        patch(&mut longest, 20, &[0xFF, 0xFF]);
        longest.extend(vec![b'c'; 0xFFFF]);
        assert!(
            archive(longest).is_ok(),
            "the longest comment a 16-bit length declares"
        );
    }

    /// The end record is the LAST one that reaches the end of the bytes,
    /// searched back from the end as every zip reader does: a self-consistent
    /// end record inside a member's data, whose comment length runs to the
    /// end, is never taken for the archive's own.
    #[test]
    fn the_last_end_record_is_read_not_one_inside_a_member() {
        let mut fake = 0x0605_4b50_u32.to_le_bytes().to_vec();
        fake.extend([0; 18]);
        let mut bytes = zip(&[put("INDICES/", b""), put("FAKE", &fake)], Wide::No, false);
        let inside = bytes
            .windows(4)
            .position(|w| w == 0x0605_4b50_u32.to_le_bytes())
            .unwrap();
        assert!(inside < eocd(&bytes), "the fake comes before the real one");
        let comment = u16le(bytes.len() - inside - 22);
        patch(&mut bytes, inside + 20, &comment);
        assert_eq!(
            archive(bytes).unwrap().ignored(),
            2,
            "both entries of the real directory, not the fake's empty one"
        );
    }

    #[test]
    fn a_saturated_end_record_without_a_locator_refuses() {
        let plain = zip(&[put("a", b"")], Wide::No, false);
        let end = eocd(&plain);
        for (at, with) in [
            (end + 10, [0xFF, 0xFF].as_slice()),
            (end + 12, &[0xFF; 4]),
            (end + 16, &[0xFF; 4]),
        ] {
            let mut bytes = plain.clone();
            patch(&mut bytes, at, with);
            assert_eq!(malformed(bytes), bad(SATURATED_WITHOUT_LOCATOR), "{at}");
        }
    }

    #[test]
    fn a_locator_that_points_at_no_zip64_record_refuses() {
        let mut bytes = outer(&inner(&csv()));
        let locator = eocd(&bytes) - 20;
        patch(&mut bytes, locator + 8, &0_u64.to_le_bytes());
        assert_eq!(malformed(bytes), bad(NO_ZIP64_RECORD));
    }

    #[test]
    fn a_central_directory_past_the_end_or_over_the_cap_refuses() {
        let mut bytes = zip(&[put("a", b"")], Wide::No, false);
        let end = eocd(&bytes);
        patch(&mut bytes, end + 12, &u32le(1_000));
        assert_eq!(malformed(bytes), bad(PAST_THE_END));
        let zip64 = outer(&inner(&csv()));
        let record = eocd(&zip64) - 20 - 56;
        for (len, said) in [
            (CENTRAL_CAP, PAST_THE_END),
            (CENTRAL_CAP + 1, CENTRAL_OVER_CAP),
        ] {
            let mut bytes = zip64.clone();
            patch(&mut bytes, record + 40, &len.to_le_bytes());
            assert_eq!(malformed(bytes), bad(said), "{len}");
        }
    }

    #[test]
    fn a_central_record_without_its_signature_or_cut_short_refuses() {
        let plain = zip(&[put("a", b"")], Wide::No, false);
        let central = plain.len() - 22 - 47;
        let mut bytes = plain.clone();
        patch(&mut bytes, central, b"PK\x01\x03");
        assert_eq!(malformed(bytes), bad(NO_CENTRAL_SIGNATURE));
        let mut bytes = plain.clone();
        patch(&mut bytes, eocd(&plain) + 10, &[2, 0]);
        assert_eq!(
            malformed(bytes),
            bad(NO_CENTRAL_SIGNATURE),
            "more entries declared than recorded"
        );
        for at in [28, 30] {
            let mut bytes = plain.clone();
            patch(&mut bytes, central + at, &[9, 0]);
            assert_eq!(malformed(bytes), bad(CUT_SHORT), "{at}");
        }
        let mut bytes = plain.clone();
        let end = eocd(&plain);
        patch(&mut bytes, end + 12, &u32le(40));
        assert_eq!(malformed(bytes), bad(CUT_SHORT), "a fixed part cut short");
    }

    /// An end record whose count understates the records in its directory
    /// would hide the records after the count, and each day zip or member
    /// among them would then refuse `ArchiveMemberMissing`, which names the
    /// operator's inputs and writes no record (design §3.3), for what is a
    /// corrupt archive. The walked records must use up the declared size
    /// exactly, in the outer ZIP64 directory and in a plain day zip's, and a
    /// last record's comment may not run past them (round-3 review).
    #[test]
    fn the_records_must_end_at_the_declared_directory_size() {
        let second = "INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_02042024.zip";
        let day_zip = inner(&csv());
        let both = zip(
            &[put(DAY_ZIP, &day_zip), put(second, &day_zip)],
            Wide::Offset,
            true,
        );
        assert!(archive(both.clone()).is_ok(), "the counts as written");
        let record = eocd(&both) - 20 - 56;
        let mut wide = both.clone();
        patch(&mut wide, record + 24, &1_u64.to_le_bytes());
        patch(&mut wide, record + 32, &1_u64.to_le_bytes());
        assert_eq!(malformed(wide), bad(SIZE_MISMATCH), "the ZIP64 count");
        let other = "GFDLCM_INDICES_TICK_01042024/NIFTY BANK.NSE_IDX.csv";
        let two = zip(&[put(MEMBER, b"a"), put(other, b"b")], Wide::No, false);
        let end = eocd(&two);
        let mut short = two.clone();
        patch(&mut short, end + 8, &[1, 0]);
        patch(&mut short, end + 10, &[1, 0]);
        assert_eq!(
            archive(outer(&short)).unwrap().day(CmKind::Indices, at()),
            Err(bad(SIZE_MISMATCH)),
            "a day zip's plain count"
        );
        assert!(
            archive(outer(&two))
                .unwrap()
                .day(CmKind::Indices, at())
                .is_ok()
        );
        let plain = zip(&[put("a", b"")], Wide::No, false);
        let mut over = plain.clone();
        patch(&mut over, plain.len() - 22 - 47 + 32, &[5, 0]);
        assert_eq!(
            malformed(over),
            bad(SIZE_MISMATCH),
            "a comment past the end"
        );
    }

    #[test]
    fn a_saturated_field_without_its_zip64_value_refuses() {
        let day_zip = inner(&csv());
        let mut bytes = outer(&day_zip);
        // The central directory starts after both local records.
        let cd = 30 + "INDICES/".len() + 30 + DAY_ZIP.len() + day_zip.len();
        let block = cd
            + bytes[cd..]
                .windows(4)
                .position(|w| w == [1, 0, 8, 0])
                .unwrap();
        patch(&mut bytes, block, &[9, 0]);
        assert_eq!(malformed(bytes), bad(NO_ZIP64_VALUE));
        // A declared-compressed member whose uncompressed length (7) differs
        // from its stored length (3), so reading the block's first two values
        // in the wrong order is seen: APPNOTE 4.5.3 puts the uncompressed size
        // first, then the compressed size, then the offset.
        let declared = Put {
            name: "a",
            method: 8,
            data: b"abc",
            crc: 0,
            len: 7,
            comment: "",
            flags: 0,
            descriptor: false,
        };
        let wide = zip(&[declared], Wide::All, false);
        let entries = central_directory(&Span::of(&wide, wide.len() as u64)).unwrap();
        assert_eq!(
            entries[0].len, 7,
            "the first wide value is the uncompressed length"
        );
        assert_eq!(entries[0].packed, 3, "the second is the compressed length");
        assert_eq!(entries[0].offset, 0, "the third is the local-header offset");
        let mut short = wide.clone();
        let block = short.windows(4).position(|w| w == [1, 0, 24, 0]).unwrap();
        patch(&mut short, block + 2, &[16, 0]);
        assert_eq!(malformed(short), bad(NO_ZIP64_VALUE), "two of three values");
    }

    #[test]
    fn a_day_zip_that_is_not_where_it_says_refuses() {
        let good = outer(&inner(&csv()));
        // The folder entry's local header, name and extended-timestamp block.
        let local = 30 + "INDICES/".len() + WIDE_LOCAL_EXTRA.len();
        let mut bytes = good.clone();
        patch(&mut bytes, local, b"PK\x03\x05");
        assert_eq!(
            archive(bytes)
                .unwrap()
                .day(CmKind::Indices, at())
                .unwrap_err(),
            bad(NO_LOCAL_SIGNATURE)
        );
        let mut bytes = good.clone();
        let size = good.len() - 22 - 20 - 56 - (46 + DAY_ZIP.len() + 17) + 20;
        patch(&mut bytes, size, &u32le(good.len() as u64));
        assert_eq!(
            archive(bytes)
                .unwrap()
                .day(CmKind::Indices, at())
                .unwrap_err(),
            bad(PAST_THE_END)
        );
        let broken = archive(outer(b"not a zip")).unwrap();
        assert_eq!(
            broken.day(CmKind::Indices, at()).unwrap_err(),
            bad(NO_END_RECORD)
        );
    }

    #[test]
    fn a_day_zip_naming_one_ticker_twice_refuses_that_ticker() {
        // Design §3.3, revision 8: two members for one ticker slot refuse
        // `AmbiguousCaseTwins` when that ticker is looked up, whether the two
        // names are equal or differ only in the extension's case; the other
        // tickers of the day are still found.
        let other = "GFDLCM_INDICES_TICK_01042024/NIFTY BANK.NSE_IDX.csv";
        let lower = "GFDLCM_INDICES_TICK_01042024/NIFTY 50.NSE_IDX.csv";
        for second in [MEMBER, lower] {
            let twice = zip(
                &[put(MEMBER, b"a"), put(second, b"b"), put(other, b"Z")],
                Wide::No,
                false,
            );
            let archive = archive(outer(&twice)).unwrap();
            let day = listed(&archive, CmKind::Indices, at());
            assert_eq!(
                stated(&day, "NIFTY 50.NSE_IDX.CSV"),
                Err(CmRefusal::AmbiguousCaseTwins),
                "{second}"
            );
            assert_eq!(
                stated(&day, "NIFTY BANK.NSE_IDX.csv").unwrap().unwrap().0,
                1
            );
            assert_eq!(day.unresolved(), 0);
            assert_eq!(
                read_day(&archive, &nifty_key(), at()),
                Err(CmRefusal::AmbiguousCaseTwins)
            );
        }
    }

    #[test]
    fn every_archive_refusal_says_what_it_refused() {
        for (refusal, words) in [
            (
                CmRefusal::ArchiveUnavailable {
                    kind: std::io::ErrorKind::NotFound,
                },
                "archive unavailable: entity not found",
            ),
            (
                CmRefusal::ArchiveMalformed { what: CUT_SHORT },
                "archive malformed: a zip record is cut short",
            ),
            (
                CmRefusal::ArchiveDuplicateDay { day: at() },
                "two day zips for 2024-04-01",
            ),
            (
                CmRefusal::ArchiveMemberCompressed {
                    member: "M".to_owned(),
                },
                "M is compressed",
            ),
            (
                CmRefusal::ArchiveMethodUnknown {
                    member: "M".to_owned(),
                    method: 12,
                },
                "M is stored with method 12",
            ),
            (
                CmRefusal::ArchiveMemberCorrupt {
                    member: "M".to_owned(),
                },
                "M does not inflate",
            ),
        ] {
            let text = refusal.to_string();
            assert!(text.contains(words), "{text}");
        }
    }

    // ── IA-3: the container faults per day, and the shapes the real
    //    containers have (design §8 B7, B12, B13, C2, C3; D-4818) ──────────

    /// `bytes` with `n` bytes put before its first record and no offset
    /// adjusted, as a self-extracting stub or a careless concatenation
    /// leaves a zip.
    fn prefixed(bytes: &[u8], n: usize) -> Vec<u8> {
        let mut out = vec![0x5A; n];
        out.extend_from_slice(bytes);
        out
    }

    /// B7: a zip with bytes before its first record is refused in its own
    /// name, the prefix measured from the end record, plain and ZIP64, and
    /// never read through offsets short by the prefix. With no prefix the
    /// same bytes open.
    #[test]
    fn a_zip_with_bytes_before_its_first_record_is_refused_by_name() {
        let day_zip = inner(&csv());
        for zip64 in [false, true] {
            let whole = zip(&[put(DAY_ZIP, &day_zip)], Wide::No, zip64);
            assert!(archive(whole.clone()).is_ok(), "zip64={zip64}");
            for n in [1, 37, 4_096] {
                assert_eq!(
                    malformed(prefixed(&whole, n)),
                    CmRefusal::ArchivePrefixed { bytes: n as u64 },
                    "zip64={zip64} n={n}"
                );
            }
        }
        // The inner day zip is held to the same rule inside its span.
        let outer = zip(&[put(DAY_ZIP, &prefixed(&day_zip, 9))], Wide::Offset, true);
        assert_eq!(
            archive(outer).unwrap().day(CmKind::Indices, at()).err(),
            Some(CmRefusal::ArchivePrefixed { bytes: 9 })
        );
        let cause = crate::gdfl_import::cm_cause(&CmRefusal::ArchivePrefixed { bytes: 9 });
        assert_eq!((cause.code, cause.closes), ("archive_prefixed", false));
    }

    /// B13 (F27): every outer day zip of the operator's archives carries a
    /// data descriptor, its local CRC and sizes zero. Built that way here,
    /// with the local sizes deliberately wrong and a descriptor after the
    /// data, the day is read by its central sizes, end to end, and so is a
    /// member of the day zip written the same way.
    #[test]
    fn a_member_with_a_data_descriptor_is_read_by_its_central_sizes() {
        let raw = csv();
        let packed = deflate(&raw);
        let mut member = deflated(MEMBER, &packed, &raw);
        member.descriptor = true;
        let day_zip = zip(
            &[put("GFDLCM_INDICES_TICK_01042024/", b""), member],
            Wide::No,
            false,
        );
        let mut entry = put(DAY_ZIP, &day_zip);
        entry.descriptor = true;
        let archive = archive(zip(&[put("INDICES/", b""), entry], Wide::Offset, true)).unwrap();
        assert_eq!(fetched(&archive), Ok(raw.clone()));
        let read = read_day(&archive, &nifty_key(), at()).unwrap().unwrap();
        assert_eq!(&*read.bytes, raw.as_slice());
        assert_eq!(read.file, decode(&raw, &nifty_expect()).unwrap());
    }

    /// B12 (FM-026): an encrypted member is refused by its own name before
    /// any byte is inflated, never as a corrupt member or a CRC mismatch:
    /// an encrypted day zip for its day only, an encrypted file of a day
    /// when it is fetched. Neither closes its day.
    #[test]
    fn an_encrypted_member_is_refused_by_its_own_name() {
        let day_zip = inner(&csv());
        let mut entry = put(DAY_ZIP, &day_zip);
        entry.flags = 1;
        let outer = zip(&[entry, put(NEXT_ZIP, &day_zip)], Wide::Offset, true);
        let archive_a = archive(outer).unwrap();
        let why = archive_a.day(CmKind::Indices, at()).unwrap_err();
        assert_eq!(
            why,
            CmRefusal::ArchiveMemberEncrypted {
                member: DAY_ZIP.to_owned()
            }
        );
        assert!(
            archive_a
                .day(CmKind::Indices, next_day())
                .unwrap()
                .is_some()
        );
        let raw = csv();
        let packed = deflate(&raw);
        let mut member = deflated(MEMBER, &packed, &raw);
        member.flags = 1;
        let day_zip = zip(
            &[put("GFDLCM_INDICES_TICK_01042024/", b""), member],
            Wide::No,
            false,
        );
        let archive_b = archive(outer_of(&day_zip)).unwrap();
        let why = fetched(&archive_b).unwrap_err();
        assert_eq!(
            why,
            CmRefusal::ArchiveMemberEncrypted {
                member: MEMBER.to_owned()
            }
        );
        let cause = crate::gdfl_import::cm_cause(&why);
        assert_eq!(
            (cause.code, cause.closes),
            ("archive_member_encrypted", false)
        );
    }

    /// The outer archive holding `day_zip` as the one day zip of `at()`.
    fn outer_of(day_zip: &[u8]) -> Vec<u8> {
        outer(day_zip)
    }

    /// C3 (F9, FM-023): a lone day zip under another folder than its own is
    /// named where it was found and where it belongs, for its day only, and
    /// never read as a day the vendor did not ship.
    #[test]
    fn a_misfiled_day_zip_is_named_where_it_was_found() {
        let day_zip = inner(&csv());
        for found in [
            "INDICES/2024/MAY_2024/GFDLCM_INDICES_TICK_01042024.zip",
            "STOCKS/2024/APR_2024/GFDLCM_INDICES_TICK_01042024.zip",
            "GFDLCM_INDICES_TICK_01042024.zip",
        ] {
            let archive = archive(zip(
                &[put(found, &day_zip), put(NEXT_ZIP, &day_zip)],
                Wide::Offset,
                true,
            ))
            .unwrap();
            let why = archive.day(CmKind::Indices, at()).unwrap_err();
            assert_eq!(
                why,
                CmRefusal::DayZipMisfiled {
                    found: found.to_owned(),
                    expected: DAY_ZIP.to_owned()
                },
                "{found}"
            );
            assert!(!crate::gdfl_import::cm_cause(&why).closes);
            assert!(archive.day(CmKind::Indices, next_day()).unwrap().is_some());
            assert_eq!(archive.census().misfiled_named, [found]);
        }
    }

    /// C2: a day zip name whose date is not real is named in the census,
    /// at most [`CENSUS_NAMED`] by name, every one counted, and files
    /// nothing.
    #[test]
    fn an_impossible_day_zip_name_is_named() {
        let names: Vec<String> = (0..20)
            .map(|n| format!("INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_32{n:02}2024.zip"))
            .collect();
        let entries: Vec<Put<'_>> = names.iter().map(|name| put(name, b"")).collect();
        let archive = archive(zip(&entries, Wide::No, false)).unwrap();
        let census = archive.census();
        assert_eq!(
            (census.invalid, census.invalid_named.len()),
            (20, CENSUS_NAMED)
        );
        assert_eq!(census.invalid_named.first(), names.first());
        assert_eq!(archive.days.each_ref().map(Vec::len), [0, 0]);
    }

    /// IA-GR-27 (D-4818): no container fault closes a day, so removing a
    /// copy, re-zipping it or a build that reads it fills the day on a
    /// re-run; a name the vendor gave a file the same way every time does.
    #[test]
    fn no_container_fault_closes_a_day() {
        for fault in [
            CmRefusal::ArchiveDuplicateDay { day: at() },
            CmRefusal::ArchiveMemberCompressed {
                member: "M".to_owned(),
            },
            CmRefusal::ArchiveMethodUnknown {
                member: "M".to_owned(),
                method: 12,
            },
            CmRefusal::ArchiveMemberEncrypted {
                member: "M".to_owned(),
            },
            CmRefusal::ArchivePrefixed { bytes: 1 },
            CmRefusal::DayZipMisfiled {
                found: "a".to_owned(),
                expected: "b".to_owned(),
            },
            CmRefusal::DayFileMisfiled {
                found: "a".to_owned(),
                expected: "b".to_owned(),
            },
            CmRefusal::ArchiveMalformed { what: CUT_SHORT },
        ] {
            let cause = crate::gdfl_import::cm_cause(&fault);
            assert!(!cause.closes, "{fault}");
            assert_eq!(cause.stage.as_str(), "vendor", "{fault}");
        }
        let missing = CmRefusal::ExtensionMissing {
            stem: "NIFTY 50.NSE_IDX".to_owned(),
        };
        let cause = crate::gdfl_import::cm_cause(&missing);
        assert_eq!((cause.code, cause.closes), ("extension_missing", true));
    }

    /// Every refusal IA-3 adds names what it refused.
    #[test]
    fn every_ia3_refusal_says_what_it_refused() {
        for (refusal, words) in [
            (
                CmRefusal::ArchivePrefixed { bytes: 37 },
                "archive prefixed: 37 bytes",
            ),
            (
                CmRefusal::ArchiveMemberEncrypted {
                    member: "M".to_owned(),
                },
                "M is encrypted",
            ),
            (
                CmRefusal::DayZipMisfiled {
                    found: "F".to_owned(),
                    expected: "E".to_owned(),
                },
                "found at F, its name files it at E",
            ),
            (
                CmRefusal::DayFileMisfiled {
                    found: "F".to_owned(),
                    expected: "E".to_owned(),
                },
                "tick-store day file misfiled: found at F",
            ),
            (
                CmRefusal::ExtensionMissing {
                    stem: "S".to_owned(),
                },
                "S is a wanted stem with no extension",
            ),
        ] {
            let text = refusal.to_string();
            assert!(text.contains(words), "{text}");
        }
    }

    // ── the real containers (ignored: they read the operator's archives) ──

    /// The real capital-market outer zip and every yearly options zip, read
    /// where `BRUTEX_GDFL_CM_ARCHIVE` and `BRUTEX_GDFL_NFO_ARCHIVES` name
    /// them, read only: each opens with no prefix (B7), and the day spans,
    /// the census, and how many day zips carry a data descriptor, an
    /// encryption bit or a method other than stored are printed for the
    /// charter (IA-0b, SF-17). Run with `--ignored --nocapture`.
    #[test]
    #[ignore = "reads the operator's real archives, named by two variables"]
    fn the_real_containers_open_with_no_prefix_and_print_their_facts() {
        if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
            let archive = Archive::open(Path::new(&path)).unwrap();
            for (tree, name) in [(0, "indices"), (1, "stocks")] {
                let held: Vec<usize> = archive.days[tree]
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, cell)| matches!(cell, Some(Slot::Held(_))).then_some(slot))
                    .collect();
                let day = |slot: usize| {
                    u32::try_from(slot)
                        .ok()
                        .and_then(|slot| Day::from_days(archive.first + slot).ok())
                };
                println!(
                    "cm_zip {name}: days={} first={:?} last={:?}",
                    held.len(),
                    held.first().map(|slot| day(*slot)),
                    held.last().map(|slot| day(*slot)),
                );
            }
            println!(
                "cm_zip prefix=0 refused_days={} census={:?}",
                archive
                    .days
                    .iter()
                    .flatten()
                    .filter(|cell| matches!(cell, Some(Slot::Refused(_))))
                    .count(),
                archive.census()
            );
            let span = Span::of(&archive.source, archive.len);
            let entries = central_directory(&span).unwrap();
            let count = |test: &dyn Fn(&Entry) -> bool| entries.iter().filter(|e| test(e)).count();
            println!(
                "cm_zip entries={} descriptor={} encrypted={} not_stored={}",
                entries.len(),
                count(&|e| e.flags & 8 != 0),
                count(&|e| e.flags & ENCRYPTED != 0),
                count(&|e| e.method != 0),
            );
        }
        if let Some(dir) = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES") {
            let mut years: Vec<_> = std::fs::read_dir(&dir)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "zip"))
                .collect();
            years.sort();
            for year in years {
                let file = std::fs::File::open(&year).unwrap();
                let len = file.metadata().unwrap().len();
                let entries = zip_entries(&file, 0, len).unwrap();
                let zips: Vec<&ZipEntry> = entries
                    .iter()
                    .filter(|e| {
                        Path::new(&e.name)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
                    })
                    .collect();
                println!(
                    "nfo_zips {}: prefix=0 entries={} day_zips={} first={:?} last={:?} \
                     descriptor={} encrypted={} not_stored={}",
                    year.display(),
                    entries.len(),
                    zips.len(),
                    zips.first().map(|e| &e.name),
                    zips.last().map(|e| &e.name),
                    zips.iter().filter(|e| e.locator.flags & 8 != 0).count(),
                    zips.iter()
                        .filter(|e| e.locator.flags & ENCRYPTED != 0)
                        .count(),
                    zips.iter().filter(|e| e.locator.method != 0).count(),
                );
            }
        }
    }
}
