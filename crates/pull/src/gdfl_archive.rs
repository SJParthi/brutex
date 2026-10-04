//! The vendor's capital-market zips read in place, the one [`CmSource`]
//! today: the outer archive's central directory indexed once, each stored
//! day zip read by offset, and each member inflated in memory and checked
//! against the size and CRC-32 its central record states. Nothing is
//! extracted to disk. D-0812, D-2800.

use std::path::Path;

use crate::gdfl_cm::{CmKind, CmRefusal, CmSource, DayListing, ListedFile};
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
    crc32: u32,
    /// Compressed length: the stored bytes, for a stored member.
    packed: u64,
    /// Uncompressed length.
    len: u64,
    /// Where its local header starts.
    offset: u64,
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

/// Every central-directory record of the zip in `span`, in order.
///
/// The end record is the last 22 bytes plus a comment; when a ZIP64 locator
/// sits right before it, the ZIP64 end record gives the count, size and
/// offset instead. That is the operator's outer archive (128,918,864,518
/// bytes): its end record saturates the offset; its first 256 central records
/// carry a plain 32-bit local-header offset and an `UT` block only, and the
/// other 3,953 saturate the offset alone and carry it in a ZIP64 block before
/// the `UT` block, which is the layout this reader was written against (its
/// whole central directory walked with `od`, charter GDFL section, D-0812).
/// The walked records must end exactly at the declared directory size.
fn central_directory<B: ReadAt>(span: &Span<'_, B>) -> Result<Vec<Entry>, CmRefusal> {
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
    }
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
        entries.push(Entry {
            name: name.to_vec(),
            method,
            crc32,
            packed,
            len,
            offset,
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
const MONTHS: [&str; 12] = [
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

/// The tree and day a member name files, when it is exactly a day zip's name.
fn day_zip(name: &str) -> Option<(CmKind, Day)> {
    let file = name.rsplit_once('/')?.1;
    let (kind, day) = crate::gdfl_cm::folder_day(file.strip_suffix(".zip")?).ok()?;
    (day_zip_name(kind, day) == name).then_some((kind, day))
}

/// Where one stored day zip lies in the outer archive.
#[derive(Debug, Clone, Copy)]
struct Inner {
    /// Its local header's offset.
    header: u64,
    /// Its stored length.
    len: u64,
}

/// The outer archive, its central directory read once into a dense table of
/// day zips per tree, indexed by days since the archive's first day.
#[derive(Debug)]
pub struct Archive<B> {
    source: B,
    len: u64,
    /// The first day any day zip names, in days since the epoch.
    first: u32,
    /// Day zips of the index tree, then of the stock tree.
    days: [Vec<Option<Inner>>; 2],
    /// Names that are not day zips: folder entries and anything else.
    ignored: usize,
}

impl Archive<std::fs::File> {
    /// Opens and indexes the archive at `path`.
    ///
    /// # Errors
    ///
    /// [`CmRefusal::ArchiveUnavailable`] when the file cannot be opened or
    /// sized, and every refusal of [`Archive::from_source`].
    pub fn open(path: &Path) -> Result<Self, CmRefusal> {
        let unavailable = |why: std::io::Error| CmRefusal::ArchiveUnavailable { kind: why.kind() };
        let file = std::fs::File::open(path).map_err(unavailable)?;
        let len = file.metadata().map_err(unavailable)?.len();
        Self::from_source(file, len)
    }
}

/// The table slot of `kind` (0 for indices, 1 for stocks).
const fn tree(kind: CmKind) -> usize {
    match kind {
        CmKind::Indices => 0,
        CmKind::Stocks => 1,
    }
}

impl<B: ReadAt> Archive<B> {
    /// Reads the central directory of `len` bytes of `source` once, O(entries),
    /// and files every day zip by tree and day.
    ///
    /// # Errors
    ///
    /// [`CmRefusal::ArchiveUnavailable`] on a failed read,
    /// [`CmRefusal::ArchiveMalformed`] when the bytes are not a zip,
    /// [`CmRefusal::ArchiveMemberCompressed`] for a day zip that is not
    /// stored, and [`CmRefusal::ArchiveDuplicateDay`] for two day zips of one
    /// tree on one day.
    pub fn from_source(source: B, len: u64) -> Result<Self, CmRefusal> {
        let entries = central_directory(&Span::of(&source, len))?;
        let mut found = Vec::with_capacity(entries.len());
        let mut ignored = 0_usize;
        for entry in entries {
            let name = String::from_utf8_lossy(&entry.name);
            let Some((kind, day)) = day_zip(&name) else {
                ignored += 1;
                continue;
            };
            if entry.method != 0 {
                return Err(CmRefusal::ArchiveMemberCompressed {
                    member: name.into_owned(),
                });
            }
            let inner = Inner {
                header: entry.offset,
                len: entry.packed,
            };
            found.push((kind, day, inner));
        }
        let first = found
            .iter()
            .map(|(_, day, _)| day.days_from_epoch())
            .min()
            .unwrap_or(0);
        let width = found
            .iter()
            .map(|(_, day, _)| index(u64::from(day.days_from_epoch() - first)) + 1)
            .max()
            .unwrap_or(0);
        let mut days = [vec![None; width], vec![None; width]];
        for (kind, day, inner) in found {
            let slot = index(u64::from(day.days_from_epoch() - first));
            let held = days
                .get_mut(tree(kind))
                .and_then(|table| table.get_mut(slot))
                .map(|cell| cell.replace(inner));
            if matches!(held, Some(Some(_))) {
                return Err(CmRefusal::ArchiveDuplicateDay { day });
            }
        }
        Ok(Self {
            source,
            len,
            first,
            days,
            ignored,
        })
    }

    /// How many names in the archive are not day zips. Report only: the
    /// census lists them, and nothing here refuses one.
    #[must_use]
    pub const fn ignored(&self) -> usize {
        self.ignored
    }

    /// Where the day zip of `kind` on `day` lies: one index into the day
    /// table, then its local header read and checked.
    fn day_zip_span(&self, kind: CmKind, day: Day) -> Result<Option<Inner>, CmRefusal> {
        let Some(inner) = day
            .days_from_epoch()
            .checked_sub(self.first)
            .and_then(|slot| self.days.get(tree(kind))?.get(index(u64::from(slot))))
            .copied()
            .flatten()
        else {
            return Ok(None);
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
        let mut listing = DayListing::new(kind, day);
        for entry in central_directory(&span)? {
            let locator = ZipLocator {
                base: inner.header,
                span: inner.len,
                header: entry.offset,
                method: u16::try_from(entry.method).unwrap_or(u16::MAX),
                packed: entry.packed,
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
        let at = file.locator;
        let span = Span {
            src: &self.source,
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
        let limit = file.len.saturating_add(1);
        match at.method {
            0 => Ok(packed.into_iter().take(index(limit)).collect()),
            8 => {
                let mut out = Vec::new();
                std::io::Read::read_to_end(
                    &mut std::io::Read::take(flate2::read::DeflateDecoder::new(&*packed), limit),
                    &mut out,
                )
                .map_err(|_| CmRefusal::ArchiveMemberCorrupt {
                    member: file.entry.to_string(),
                })?;
                Ok(out)
            }
            _ => Err(CmRefusal::ArchiveMethodUnknown {
                member: file.entry.to_string(),
                method: at.method,
            }),
        }
    }
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
    }

    fn put<'a>(name: &'a str, data: &'a [u8]) -> Put<'a> {
        Put {
            name,
            method: 0,
            data,
            crc: crc32(data),
            len: data.len() as u64,
            comment: "",
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
            out.extend(0x0403_4b50_u32.to_le_bytes());
            out.extend([20, 0, 0, 0]);
            out.extend(entry.method.to_le_bytes());
            out.extend([0; 4]);
            out.extend(entry.crc.to_le_bytes());
            out.extend(u32le(entry.data.len() as u64));
            out.extend(u32le(entry.len));
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
            central.extend([45, 3, 45, 0, 0, 8]);
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

    #[test]
    fn outer_member_not_stored_refuses() {
        let day_zip = inner(&csv());
        let mut entry = put(DAY_ZIP, &day_zip);
        entry.method = 8;
        assert_eq!(
            archive(zip(&[entry], Wide::Offset, true)).err(),
            Some(CmRefusal::ArchiveMemberCompressed {
                member: DAY_ZIP.to_owned()
            })
        );
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

    #[test]
    fn two_members_for_one_day_refuse() {
        let day_zip = inner(&csv());
        assert_eq!(
            archive(zip(
                &[put(DAY_ZIP, &day_zip), put(DAY_ZIP, &day_zip)],
                Wide::Offset,
                true
            ))
            .err(),
            Some(CmRefusal::ArchiveDuplicateDay { day: at() })
        );
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
        assert_eq!(std::mem::size_of::<Option<Inner>>(), 24);
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
        let may = Day::new(2026, 5, 15).unwrap();
        let span = may.days_from_epoch() - at().days_from_epoch() + 1;
        assert_eq!(archive.first, at().days_from_epoch());
        assert_eq!(
            archive.days.each_ref().map(Vec::len),
            [usize::try_from(span).unwrap(); 2],
            "one cell per day from the first day zip to the last, no wider"
        );
        assert!(archive.day(CmKind::Indices, at()).is_ok());
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
                assert_eq!(day_zip(&name), Some((kind, day)), "{name}");
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
}
