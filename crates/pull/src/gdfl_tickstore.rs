//! The verified tick store, the second [`CmSource`] (D-2801): one `.bts` day
//! file per vendor day zip, holding every entry of that zip losslessly, read
//! strictly as its format specification v1 (`BRTXTS01`) states.
//!
//! The specification is `/Volumes/WD_BLACK/brutex/tickstore-data/FORMAT.md`,
//! version 1, sha256
//! `6edc62a772b257a2aa1b45c0785a23936d9bff47b87648bbb1d9a84ab7063259`,
//! recorded in `docs/00-charter.md` (GDFL capital-market section). Nothing
//! here comes from the store's own tool: the file layout (§3), the block
//! encoding (§4) and the paths (§2) are read off the specification, and every
//! rebuilt file is still held by [`crate::gdfl_cm::read_listed`] to the zip
//! central directory's size and CRC-32 the index states.
//!
//! # Cost
//!
//! Listing a day reads the 40-byte footer, the index frame and decodes it once,
//! O(entries), then files the swept tickers into the listing's ticker-slot
//! array; a lookup after that is one index (CM-13). Fetching a file is one
//! read of its block and O(the file's bytes) to decode and rebuild it.
//! UNVERIFIED as measured times (`docs/06-limits.md`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::gdfl_archive::{ReadAt, crc32, day_zip_name};
use crate::gdfl_cm::{CmKind, CmRefusal, CmSource, DayListing, ListedFile};
use crate::session::Day;

/// The first eight bytes of a day file (§3).
const MAGIC: &[u8; 8] = b"BRTXTS01";
/// The last eight bytes of a day file, ending its footer (§3).
const FOOTER_MAGIC: &[u8; 8] = b"BRTXTSE1";
/// The footer's length (§3).
const FOOTER_LEN: u64 = 40;
/// One index entry's fixed fields after its name: kind, off, len, size, crc,
/// rows, `dos_time` (§3).
const ENTRY_FIXED: u64 = 41;

/// Not a day file: too short, or its first eight bytes are not `BRTXTS01`.
const NO_MAGIC: &str = "the file does not start with BRTXTS01";
/// The footer does not end with `BRTXTSE1`.
const NO_FOOTER: &str = "the footer does not end with BRTXTSE1";
/// `index_off + index_len + 40` is not the file's length.
const INDEX_NOT_AT_END: &str = "the index and footer do not end the file";
/// The index frame does not decode to `index_raw_len` bytes.
const INDEX_LENGTH: &str = "the index does not decode to its stated length";
/// The decoded index's CRC-32 is not `index_crc`.
const INDEX_CRC: &str = "the index's CRC-32 is not the footer's";
/// The index does not hold exactly `n` whole entries.
const INDEX_ENTRIES: &str = "the index does not hold exactly its stated entries";
/// An entry kind other than 0, 1 or 2.
const ENTRY_KIND: &str = "an index entry's kind is not 0, 1 or 2";
/// A block outside `[8, index_off)`, or a directory entry with a block.
const BLOCK_RANGE: &str = "a block does not lie inside the blocks area";
/// A zstd frame that does not decode, or not to its stated length.
const FRAME: &str = "a zstd frame does not decode to its stated length";
/// A block whose first byte is not its entry's kind.
const BLOCK_KIND: &str = "a block's first byte is not its entry's kind";
/// A columnar block cut short or carrying a field outside its range.
const COLUMNAR: &str = "a columnar block is malformed";
/// A columnar block whose column count is not the header's commas plus one.
const COLUMN_COUNT: &str = "a columnar block's column count is not its header's";
/// A columnar block whose row count is not its index entry's.
const ROW_COUNT: &str = "a columnar block's row count is not its index entry's";
/// A column whose text does not hold one field per row, or not its stated
/// length.
const COLUMN_TEXT: &str = "a column's text is not one field per row of its stated length";
/// A numeric column's payload is not `rows + 2 + w * rows` bytes with `w`
/// at most 8.
const NUMERIC: &str = "a numeric column's payload is not its stated shape";
/// A time field outside `0..=359_999` seconds, which no two-digit-hour `HH:MM:SS`
/// can show.
const TIME_RANGE: &str = "a numeric time field is out of range";

/// Where one entry's block lies in its day file, for [`CmSource::fetch`].
#[derive(Debug, Clone)]
pub struct TickLocator {
    /// The day file, opened once per listing and shared by its entries.
    file: Arc<std::fs::File>,
    /// The entry's kind (§3): 0 directory, 1 columnar, 2 raw.
    kind: u8,
    /// The block's absolute offset.
    off: u64,
    /// The block's length.
    len: u64,
    /// The data rows the index states.
    rows: u64,
}

/// One entry of a decoded index (§3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    /// The raw zip entry name.
    pub name: String,
    /// 0 directory, 1 columnar, 2 raw.
    pub kind: u8,
    /// The block's absolute offset.
    pub off: u64,
    /// The block's length.
    pub len: u64,
    /// The original file's length (the zip central directory's size).
    pub size: u64,
    /// The original file's CRC-32 (the zip central directory's).
    pub crc: u32,
    /// Data rows, 0 for a raw block.
    pub rows: u64,
    /// `(zip date << 16) | zip time`.
    pub dos_time: u32,
}

/// `len` bytes of `src` at `at`, or `TickStoreUnavailable`.
fn read<B: ReadAt>(src: &B, at: u64, len: u64) -> Result<Vec<u8>, CmRefusal> {
    let mut buf = vec![0; usize::try_from(len).unwrap_or(usize::MAX)];
    src.read_at(at, &mut buf)
        .map_err(|why| CmRefusal::TickStoreUnavailable { kind: why.kind() })?;
    Ok(buf)
}

/// The refusal for a day file saying `what`.
const fn bad(what: &'static str) -> CmRefusal {
    CmRefusal::TickStoreMalformed { what }
}

/// One zstd frame, decoded to exactly `want` bytes or refused: the decoder is
/// stopped one byte past `want`, so a frame with more to give fails rather
/// than filling the buffer.
fn unzstd(frame: &[u8], want: u64) -> Result<Vec<u8>, CmRefusal> {
    let decoder = ruzstd::decoding::StreamingDecoder::new(frame).map_err(|_| bad(FRAME))?;
    let mut out = Vec::new();
    std::io::Read::read_to_end(
        &mut std::io::Read::take(decoder, want.saturating_add(1)),
        &mut out,
    )
    .map_err(|_| bad(FRAME))?;
    if u64::try_from(out.len()).unwrap_or(u64::MAX) == want {
        Ok(out)
    } else {
        Err(bad(FRAME))
    }
}

/// A little-endian reader over a byte slice that refuses with `what` when it
/// runs past the end.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
    what: &'static str,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8], what: &'static str) -> Self {
        Self { bytes, at: 0, what }
    }

    fn take(&mut self, n: u64) -> Result<&'a [u8], CmRefusal> {
        let n = usize::try_from(n).unwrap_or(usize::MAX);
        let end = self.at.checked_add(n).ok_or(bad(self.what))?;
        let got = self.bytes.get(self.at..end).ok_or(bad(self.what))?;
        self.at = end;
        Ok(got)
    }

    fn uint(&mut self, width: u64) -> Result<u64, CmRefusal> {
        let mut wide = [0_u8; 8];
        wide.iter_mut()
            .zip(self.take(width)?)
            .for_each(|(slot, byte)| *slot = *byte);
        Ok(u64::from_le_bytes(wide))
    }

    fn u8(&mut self) -> Result<u8, CmRefusal> {
        Ok(self.take(1)?.first().copied().unwrap_or_default())
    }

    fn u32(&mut self) -> Result<u32, CmRefusal> {
        Ok(u32::try_from(self.uint(4)?).unwrap_or_default())
    }

    fn u64(&mut self) -> Result<u64, CmRefusal> {
        self.uint(8)
    }

    const fn done(&self) -> bool {
        self.at == self.bytes.len()
    }
}

/// The footer, the index frame and every entry of the day file of `len`
/// bytes in `src`, validated exactly as §3 states: the magic, the footer's
/// magic, `index_off + index_len + 40 == len`, the decoded length, the CRC-32,
/// exactly `n` entries with nothing after them, each kind 0, 1 or 2, a
/// directory entry with no block and every other block inside `[8,
/// index_off)`. O(entries).
///
/// # Errors
///
/// [`CmRefusal::TickStoreMalformed`] naming the first rule broken, and
/// [`CmRefusal::TickStoreUnavailable`] on a failed read.
pub fn read_index<B: ReadAt>(src: &B, len: u64) -> Result<Vec<IndexEntry>, CmRefusal> {
    if len < 8 + FOOTER_LEN {
        return Err(bad(NO_MAGIC));
    }
    if read(src, 0, 8)? != MAGIC {
        return Err(bad(NO_MAGIC));
    }
    let footer = read(src, len - FOOTER_LEN, FOOTER_LEN)?;
    let mut f = Cursor::new(&footer, NO_FOOTER);
    let (index_off, index_len, index_raw_len) = (f.u64()?, f.u64()?, f.u64()?);
    let (index_crc, n) = (f.u32()?, f.u32()?);
    if f.take(8)? != FOOTER_MAGIC {
        return Err(bad(NO_FOOTER));
    }
    let end = index_off
        .checked_add(index_len)
        .and_then(|sum| sum.checked_add(FOOTER_LEN));
    if end != Some(len) || index_off < 8 {
        return Err(bad(INDEX_NOT_AT_END));
    }
    let index =
        unzstd(&read(src, index_off, index_len)?, index_raw_len).map_err(|_| bad(INDEX_LENGTH))?;
    if crc32(&index) != index_crc {
        return Err(bad(INDEX_CRC));
    }
    let mut c = Cursor::new(&index, INDEX_ENTRIES);
    let mut entries = Vec::new();
    for _ in 0..n {
        let name_len = c.uint(2)?;
        let name = String::from_utf8_lossy(c.take(name_len)?).into_owned();
        let fixed = c.take(ENTRY_FIXED)?;
        let mut e = Cursor::new(fixed, INDEX_ENTRIES);
        let entry = IndexEntry {
            name,
            kind: e.u8()?,
            off: e.u64()?,
            len: e.u64()?,
            size: e.u64()?,
            crc: e.u32()?,
            rows: e.u64()?,
            dos_time: e.u32()?,
        };
        let inside = entry.off >= 8
            && entry
                .off
                .checked_add(entry.len)
                .is_some_and(|block_end| block_end <= index_off);
        match entry.kind {
            0 if entry.len != 0 => return Err(bad(BLOCK_RANGE)),
            1 | 2 if !inside => return Err(bad(BLOCK_RANGE)),
            0..=2 => {}
            _ => return Err(bad(ENTRY_KIND)),
        }
        entries.push(entry);
    }
    if !c.done() {
        return Err(bad(INDEX_ENTRIES));
    }
    Ok(entries)
}

/// The original file an entry's block holds, rebuilt as §4 states. A
/// directory entry is empty; a raw block is its zstd frame decoded to the
/// entry's `size`; a columnar block is rebuilt row by row from its columns.
/// The result is NOT checked here against `size` and `crc`:
/// [`crate::gdfl_cm::read_listed`] does that for every source.
///
/// # Errors
///
/// [`CmRefusal::TickStoreMalformed`] naming the first rule broken, and
/// [`CmRefusal::TickStoreUnavailable`] on a failed read.
pub fn rebuild<B: ReadAt>(src: &B, entry: &IndexEntry) -> Result<Vec<u8>, CmRefusal> {
    if entry.kind == 0 {
        return Ok(Vec::new());
    }
    let block = read(src, entry.off, entry.len)?;
    let (&kind, body) = block.split_first().ok_or(bad(BLOCK_KIND))?;
    if kind != entry.kind {
        return Err(bad(BLOCK_KIND));
    }
    if kind == 2 {
        return unzstd(body, entry.size);
    }
    columnar(body, entry.rows)
}

/// A columnar block's body after its kind byte (§4, kind 1), rebuilt.
fn columnar(body: &[u8], index_rows: u64) -> Result<Vec<u8>, CmRefusal> {
    let mut c = Cursor::new(body, COLUMNAR);
    let terminator: &[u8] = match c.u8()? {
        0 => b"\n",
        1 => b"\r\n",
        _ => return Err(bad(COLUMNAR)),
    };
    let trailing = match c.u8()? {
        0 => false,
        1 => true,
        _ => return Err(bad(COLUMNAR)),
    };
    let header_len = c.u32()?;
    let header = c.take(u64::from(header_len))?;
    let ncols = c.u32()?;
    let fields = header.split(|&b| b == b',').count();
    if usize::try_from(ncols).ok() != Some(fields) {
        return Err(bad(COLUMN_COUNT));
    }
    let rows = c.u64()?;
    if rows != index_rows {
        return Err(bad(ROW_COUNT));
    }
    let mut columns = Vec::new();
    for _ in 0..ncols {
        let tag = c.u8()?;
        let (payload_len, text_len, zlen) = (c.u64()?, c.u64()?, c.u64()?);
        let payload = unzstd(c.take(zlen)?, payload_len)?;
        let column = match tag {
            0 => Column::text(payload, rows)?,
            1 => Column::numeric(&payload, rows)?,
            _ => return Err(bad(COLUMNAR)),
        };
        if column.text_len() != text_len {
            return Err(bad(COLUMN_TEXT));
        }
        columns.push(column);
    }
    if !c.done() {
        return Err(bad(COLUMNAR));
    }
    let mut out = header.to_vec();
    for row in 0..usize::try_from(rows).unwrap_or(usize::MAX) {
        out.extend_from_slice(terminator);
        for (at, column) in columns.iter().enumerate() {
            if at > 0 {
                out.push(b',');
            }
            out.extend_from_slice(column.field(row));
        }
    }
    if trailing {
        out.extend_from_slice(terminator);
    }
    Ok(out)
}

/// One column's fields: its text, and where each field ends in it.
struct Column {
    text: Vec<u8>,
    /// `ends[i]` is the end of field `i`; field `i` starts one past the end
    /// of field `i - 1` (its `\n`), or at 0.
    ends: Vec<usize>,
}

impl Column {
    /// A text column (tag 0): the payload is the column text, one field per
    /// row joined by `\n`. With no rows it must be empty.
    fn text(payload: Vec<u8>, rows: u64) -> Result<Self, CmRefusal> {
        if rows == 0 {
            return if payload.is_empty() {
                Ok(Self {
                    text: payload,
                    ends: Vec::new(),
                })
            } else {
                Err(bad(COLUMN_TEXT))
            };
        }
        let mut ends: Vec<usize> = payload
            .iter()
            .enumerate()
            .filter(|&(_, &b)| b == b'\n')
            .map(|(at, _)| at)
            .collect();
        ends.push(payload.len());
        if u64::try_from(ends.len()).unwrap_or(u64::MAX) != rows {
            return Err(bad(COLUMN_TEXT));
        }
        Ok(Self {
            text: payload,
            ends,
        })
    }

    /// A numeric column (tag 1): `shape[rows]`, `delta`, `w`, then `w` byte
    /// planes of `rows` bytes, each value zigzag-decoded, summed when `delta`
    /// is 1, and rendered from its shape (§4).
    fn numeric(payload: &[u8], rows: u64) -> Result<Self, CmRefusal> {
        let count = usize::try_from(rows).map_err(|_| bad(NUMERIC))?;
        let mut cursor = Cursor::new(payload, NUMERIC);
        let shapes = cursor.take(rows)?;
        let delta = cursor.u8()?;
        let width = cursor.u8()?;
        if delta > 1 || width > 8 {
            return Err(bad(NUMERIC));
        }
        let planes = cursor.take(u64::from(width).saturating_mul(rows))?;
        if !cursor.done() {
            return Err(bad(NUMERIC));
        }
        let mut text = Vec::new();
        let mut ends = Vec::with_capacity(count);
        let mut previous: i64 = 0;
        for (i, &shape) in shapes.iter().enumerate() {
            // Plane k holds byte k of every value, little-endian (§4).
            let mut le = [0_u8; 8];
            for (k, slot) in le.iter_mut().take(usize::from(width)).enumerate() {
                *slot = planes.get(k * count + i).copied().unwrap_or_default();
            }
            let x = u64::from_le_bytes(le);
            let z = i64::from_ne_bytes(((x >> 1) ^ (x & 1).wrapping_neg()).to_ne_bytes());
            let v = if delta == 1 {
                previous.wrapping_add(z)
            } else {
                z
            };
            previous = v;
            if i > 0 {
                text.push(b'\n');
            }
            render(shape, v, &mut text)?;
            ends.push(text.len());
        }
        Ok(Self { text, ends })
    }

    /// The length of the column text: its fields joined by `\n`.
    fn text_len(&self) -> u64 {
        u64::try_from(self.text.len()).unwrap_or(u64::MAX)
    }

    /// Field `row`.
    fn field(&self, row: usize) -> &[u8] {
        let start = row
            .checked_sub(1)
            .and_then(|before| self.ends.get(before))
            .map_or(0, |end| end + 1);
        let end = self.ends.get(row).copied().unwrap_or(start);
        self.text.get(start..end).unwrap_or_default()
    }
}

/// One numeric field's text from its shape and value (§4): bit 5 a time
/// `HH:MM:SS`; otherwise an optional `-` (bit 4), then `|v|` left-padded with
/// zeros to at least `d + 1` digits and a `.` `d` digits from the right when
/// `d > 0`, `d` being the low four bits.
fn render(shape: u8, v: i64, out: &mut Vec<u8>) -> Result<(), CmRefusal> {
    if shape & 0x20 != 0 {
        let Some(secs) = u64::try_from(v).ok().filter(|&secs| secs < 360_000) else {
            return Err(bad(TIME_RANGE));
        };
        let text = format!("{:02}:{:02}:{:02}", secs / 3_600, secs / 60 % 60, secs % 60);
        out.extend_from_slice(text.as_bytes());
        return Ok(());
    }
    if shape & 0x10 != 0 {
        out.push(b'-');
    }
    let d = usize::from(shape & 0x0F);
    let digits = format!("{:0width$}", v.unsigned_abs(), width = d + 1);
    let split = digits.len() - d;
    out.extend_from_slice(digits.get(..split).unwrap_or_default().as_bytes());
    if d > 0 {
        out.push(b'.');
        out.extend_from_slice(digits.get(split..).unwrap_or_default().as_bytes());
    }
    Ok(())
}

/// The tick store under its root (§2): `cm/<TREE>/<yyyy>/<MON_yyyy>/<day
/// folder>.bts` for a capital-market day. Only that exact name is opened, so
/// a `*.bts.tmp` the ingest left behind is never read; a day with no `.bts`
/// was not ingested and is `None`. Files are opened read-only and read with
/// positional reads; nothing is mapped.
#[derive(Debug, Clone)]
pub struct TickStore {
    root: PathBuf,
}

impl TickStore {
    /// The store whose root is `root`.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// Where the day file of tree `kind` on `day` lies (§2).
    #[must_use]
    pub fn day_path(&self, kind: CmKind, day: Day) -> PathBuf {
        let zip = day_zip_name(kind, day);
        let stem = zip.strip_suffix(".zip").unwrap_or(&zip);
        self.root.join(format!("cm/{stem}.bts"))
    }
}

impl CmSource for TickStore {
    type Locator = TickLocator;

    /// The day file's index, validated (§3), every entry in its order (the
    /// zip's) with the size and CRC-32 the zip's central directory stated.
    /// `None` when the store holds no `.bts` for that tree and day.
    ///
    /// # Errors
    ///
    /// [`CmRefusal::TickStoreUnavailable`] when the file exists but cannot be
    /// opened or read, and every refusal of [`read_index`].
    fn day(&self, kind: CmKind, day: Day) -> Result<Option<DayListing<TickLocator>>, CmRefusal> {
        let unavailable =
            |why: std::io::Error| CmRefusal::TickStoreUnavailable { kind: why.kind() };
        let file = match std::fs::File::open(self.day_path(kind, day)) {
            Ok(file) => file,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(why) => return Err(unavailable(why)),
        };
        let file_len = file.metadata().map_err(unavailable)?.len();
        let entries = read_index(&file, file_len)?;
        let file = Arc::new(file);
        let mut listing = DayListing::new(kind, day);
        for entry in entries {
            let locator = TickLocator {
                file: Arc::clone(&file),
                kind: entry.kind,
                off: entry.off,
                len: entry.len,
                rows: entry.rows,
            };
            listing.push(&entry.name, entry.size, entry.crc, locator);
        }
        Ok(Some(listing))
    }

    /// The entry rebuilt by [`rebuild`].
    ///
    /// # Errors
    ///
    /// Every refusal of [`rebuild`].
    fn fetch(
        &self,
        _listing: &DayListing<TickLocator>,
        file: &ListedFile<TickLocator>,
    ) -> Result<Vec<u8>, CmRefusal> {
        let at = &file.locator;
        let entry = IndexEntry {
            name: file.entry.to_string(),
            kind: at.kind,
            off: at.off,
            len: at.len,
            size: file.len,
            crc: file.crc32,
            rows: at.rows,
            dos_time: 0,
        };
        // The block was checked inside the file when the day was listed; a
        // file that shrank since is a failed positional read, never a short
        // buffer.
        rebuild(&*at.file, &entry)
    }
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;
    use crate::gdfl_cm::{CmFile, Expect, HEADER_OPEN_INTEREST, decode, read_day, read_listed};
    use brutex_core::instrument::{Exchange, InstrumentKey};

    // ── a writer for synthetic day files (invented rows only) ──────────────

    fn zstd(bytes: &[u8]) -> Vec<u8> {
        ruzstd::encoding::compress_to_vec(bytes, ruzstd::encoding::CompressionLevel::Fastest)
    }

    /// One column of a columnar block, with the field texts it must render.
    enum Col<'a> {
        /// Tag 0: the fields as text.
        Text(Vec<&'a str>),
        /// Tag 1: one shape and one value per row, delta-coded or not, in
        /// `width` byte planes, and the texts they render to.
        Num {
            shapes: Vec<u8>,
            delta: bool,
            width: u8,
            values: Vec<i64>,
            texts: Vec<&'a str>,
        },
    }

    /// The column's tag, payload and column-text length.
    fn column(col: &Col<'_>) -> (u8, Vec<u8>, u64) {
        let text_len = |texts: &[&str]| {
            (texts.iter().map(|t| t.len()).sum::<usize>() + texts.len().saturating_sub(1)) as u64
        };
        match col {
            Col::Text(fields) => (0, fields.join("\n").into_bytes(), text_len(fields)),
            Col::Num {
                shapes,
                delta,
                width,
                values,
                texts,
            } => {
                let mut previous = 0_i64;
                let xs: Vec<u64> = values
                    .iter()
                    .map(|&v| {
                        let z = if *delta { v.wrapping_sub(previous) } else { v };
                        previous = v;
                        u64::from_ne_bytes(((z << 1) ^ (z >> 63)).to_ne_bytes())
                    })
                    .collect();
                let mut payload = shapes.clone();
                payload.push(u8::from(*delta));
                payload.push(*width);
                for k in 0..u32::from(*width) {
                    payload.extend(xs.iter().map(|x| ((x >> (8 * k)) & 0xFF) as u8));
                }
                (1, payload, text_len(texts))
            }
        }
    }

    /// A columnar (kind 1) block.
    fn columnar_block(
        crlf: bool,
        trailing: bool,
        header: &str,
        rows: u64,
        cols: &[Col<'_>],
    ) -> Vec<u8> {
        let mut out = vec![1, u8::from(crlf), u8::from(trailing)];
        out.extend((header.len() as u32).to_le_bytes());
        out.extend(header.as_bytes());
        out.extend((cols.len() as u32).to_le_bytes());
        out.extend(rows.to_le_bytes());
        for col in cols {
            let (tag, payload, text_len) = column(col);
            let frame = zstd(&payload);
            out.push(tag);
            out.extend((payload.len() as u64).to_le_bytes());
            out.extend(text_len.to_le_bytes());
            out.extend((frame.len() as u64).to_le_bytes());
            out.extend(frame);
        }
        out
    }

    /// A raw (kind 2) block.
    fn raw_block(bytes: &[u8]) -> Vec<u8> {
        let mut out = vec![2];
        out.extend(zstd(bytes));
        out
    }

    /// One entry to write: its name, kind, block, and the size, CRC-32 and
    /// rows its index states.
    struct Put {
        name: String,
        kind: u8,
        block: Vec<u8>,
        size: u64,
        crc: u32,
        rows: u64,
    }

    /// An entry whose stated size and CRC-32 are those of `original`.
    fn put(name: &str, kind: u8, block: Vec<u8>, original: &[u8], rows: u64) -> Put {
        Put {
            name: name.to_owned(),
            kind,
            block,
            size: original.len() as u64,
            crc: crc32(original),
            rows,
        }
    }

    /// The decoded index of `entries` laid out from offset 8, and the offset
    /// where the blocks end.
    fn index_of(entries: &[Put]) -> (Vec<u8>, u64) {
        let mut index = Vec::new();
        let mut at = 8_u64;
        for e in entries {
            index.extend((e.name.len() as u16).to_le_bytes());
            index.extend(e.name.as_bytes());
            index.push(e.kind);
            index.extend(at.to_le_bytes());
            index.extend((e.block.len() as u64).to_le_bytes());
            index.extend(e.size.to_le_bytes());
            index.extend(e.crc.to_le_bytes());
            index.extend(e.rows.to_le_bytes());
            index.extend(0x5821_6000_u32.to_le_bytes());
            at += e.block.len() as u64;
        }
        (index, at)
    }

    /// A day file of `entries` around a given decoded `index`, with its
    /// footer stating `n` entries.
    fn day_file_with(entries: &[Put], index: &[u8], n: u32) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        for e in entries {
            out.extend(&e.block);
        }
        let frame = zstd(index);
        let index_off = out.len() as u64;
        out.extend(&frame);
        out.extend(index_off.to_le_bytes());
        out.extend((frame.len() as u64).to_le_bytes());
        out.extend((index.len() as u64).to_le_bytes());
        out.extend(crc32(index).to_le_bytes());
        out.extend(n.to_le_bytes());
        out.extend(FOOTER_MAGIC);
        out
    }

    fn day_file(entries: &[Put]) -> Vec<u8> {
        let (index, _) = index_of(entries);
        day_file_with(entries, &index, entries.len() as u32)
    }

    fn len(bytes: &[u8]) -> u64 {
        bytes.len() as u64
    }

    // ── synthetic rows (invented values) ───────────────────────────────────

    fn n50_row(time: &str, ltp: &str) -> String {
        format!("NIFTY 50.NSE_IDX,01/04/2024,{time},{ltp},0,0,0,0,0,0")
    }

    const TIMES: [&str; 3] = ["09:15:00", "09:15:00", "16:00:01"];
    const LTPS: [&str; 3] = ["10000.05", "9999.95", "10001.50"];

    /// The ten columns of three invented NIFTY 50 rows: two text columns, a
    /// delta-coded time column, a delta-coded two-place LTP column whose
    /// second step is negative, and six one-plane zero columns.
    fn n50_columns() -> Vec<Col<'static>> {
        let zero = || Col::Num {
            shapes: vec![0; 3],
            delta: false,
            width: 1,
            values: vec![0; 3],
            texts: vec!["0"; 3],
        };
        let mut cols = vec![
            Col::Text(vec!["NIFTY 50.NSE_IDX"; 3]),
            Col::Text(vec!["01/04/2024"; 3]),
            Col::Num {
                shapes: vec![0x20; 3],
                delta: true,
                width: 4,
                values: vec![33_300, 33_300, 57_601],
                texts: TIMES.to_vec(),
            },
            Col::Num {
                shapes: vec![2; 3],
                delta: true,
                width: 3,
                values: vec![1_000_005, 999_995, 1_000_150],
                texts: LTPS.to_vec(),
            },
        ];
        cols.extend((0..6).map(|_| zero()));
        cols
    }

    /// The CSV those columns rebuild, CRLF or LF, with or without the final
    /// terminator.
    fn n50_csv(crlf: bool, trailing: bool) -> Vec<u8> {
        let t = if crlf { "\r\n" } else { "\n" };
        let mut text = HEADER_OPEN_INTEREST.to_owned();
        for (time, ltp) in TIMES.iter().zip(LTPS) {
            text.push_str(t);
            text.push_str(&n50_row(time, ltp));
        }
        if trailing {
            text.push_str(t);
        }
        text.into_bytes()
    }

    fn entry_of(bytes: &[u8], at: usize) -> IndexEntry {
        read_index(&bytes.to_vec(), len(bytes)).unwrap()[at].clone()
    }

    fn rebuilt(bytes: &[u8], at: usize) -> Result<Vec<u8>, CmRefusal> {
        rebuild(&bytes.to_vec(), &entry_of(bytes, at))
    }

    // ── the format ─────────────────────────────────────────────────────────

    /// A columnar block rebuilds the CSV byte for byte under every line
    /// ending and either final terminator: text columns, a delta-coded time
    /// column, a two-place price column with a negative step, and one-plane
    /// zeros (§4).
    #[test]
    fn a_columnar_block_rebuilds_its_csv_exactly() {
        for crlf in [true, false] {
            for trailing in [true, false] {
                let csv = n50_csv(crlf, trailing);
                let block = columnar_block(crlf, trailing, HEADER_OPEN_INTEREST, 3, &n50_columns());
                let file = day_file(&[put("F/NIFTY 50.NSE_IDX.csv", 1, block, &csv, 3)]);
                assert_eq!(
                    rebuilt(&file, 0),
                    Ok(csv),
                    "crlf {crlf} trailing {trailing}"
                );
            }
        }
    }

    /// With no rows every column text is empty and the file is the header,
    /// with its terminator when `trailing`.
    #[test]
    fn a_columnar_block_of_no_rows_is_its_header() {
        let cols = || {
            vec![
                Col::Text(Vec::new()),
                Col::Num {
                    shapes: Vec::new(),
                    delta: true,
                    width: 0,
                    values: Vec::new(),
                    texts: Vec::new(),
                },
            ]
        };
        for (trailing, want) in [(false, "A,B"), (true, "A,B\r\n")] {
            let block = columnar_block(true, trailing, "A,B", 0, &cols());
            let file = day_file(&[put("F/x.csv", 1, block, want.as_bytes(), 0)]);
            assert_eq!(rebuilt(&file, 0), Ok(want.as_bytes().to_vec()));
        }
    }

    /// Numeric fields render as §4 states: zero padding to `d + 1` digits and
    /// a point `d` from the right, a sign from bit 4 alone, a time from bit
    /// 5, zigzag both ways, and the full eight planes.
    #[test]
    fn numeric_fields_render_as_the_specification_states() {
        // Two texts are formatted at run time: gate 1d reads a quoted digit
        // string as a possible path segment.
        let (seven, max) = (7.to_string(), i64::MAX.to_string());
        let cases: [(u8, i64, &str); 10] = [
            (2, 5, "0.05"),
            (2, 0, "0.00"),
            (0, 7, &seven),
            (3, 12_345, "12.345"),
            (0x12, 150, "-1.50"),
            (0x10, -42, "-42"),
            (0, -42, "42"),
            (0x20, 0, "00:00:00"),
            (0x20, 359_999, "99:59:59"),
            (0, i64::MAX, &max),
        ];
        for delta in [false, true] {
            let col = Col::Num {
                shapes: cases.iter().map(|c| c.0).collect(),
                delta,
                width: 8,
                values: cases.iter().map(|c| c.1).collect(),
                texts: cases.iter().map(|c| c.2).collect(),
            };
            let want = cases.iter().map(|c| c.2).collect::<Vec<_>>().join("\n");
            let original = format!("V\n{want}");
            let block = columnar_block(false, false, "V", cases.len() as u64, &[col]);
            let file = day_file(&[put("F/x.csv", 1, block, original.as_bytes(), 10)]);
            assert_eq!(
                String::from_utf8(rebuilt(&file, 0).unwrap()).unwrap(),
                original,
                "delta {delta}"
            );
        }
    }

    /// A raw block is its zstd frame; a directory entry has no block and
    /// rebuilds to nothing; an index is read in its order with every field.
    #[test]
    fn raw_blocks_and_directory_entries_and_the_index() {
        let odd = b"a,b\r\nmixed\nends\r".to_vec();
        let file = day_file(&[
            put("F/", 0, Vec::new(), b"", 0),
            put("F/odd.csv", 2, raw_block(&odd), &odd, 0),
        ]);
        let entries = read_index(&file, len(&file)).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[1],
            IndexEntry {
                name: "F/odd.csv".to_owned(),
                kind: 2,
                off: 8,
                len: entries[1].len,
                size: len(&odd),
                crc: crc32(&odd),
                rows: 0,
                dos_time: 0x5821_6000,
            }
        );
        assert_eq!(entries[0].len, 0);
        assert_eq!(rebuilt(&file, 0), Ok(Vec::new()));
        assert_eq!(rebuilt(&file, 1), Ok(odd));
    }

    // ── every refusal ──────────────────────────────────────────────────────

    fn small() -> Vec<u8> {
        let odd = b"x".to_vec();
        day_file(&[put("F/odd.csv", 2, raw_block(&odd), &odd, 0)])
    }

    fn index_refusal(bytes: &[u8]) -> CmRefusal {
        read_index(&bytes.to_vec(), len(bytes)).unwrap_err()
    }

    fn patch(bytes: &mut [u8], at: usize, with: &[u8]) {
        bytes[at..at + with.len()].copy_from_slice(with);
    }

    /// The two edges of §3's layout: a file exactly as long as its magic and
    /// footer reaches the index checks (and fails there: an empty index is
    /// no zstd frame), and a file of directory entries only, whose index
    /// starts at offset 8 with no blocks before it, reads.
    #[test]
    fn the_shortest_file_and_an_index_at_offset_eight() {
        let mut shortest = MAGIC.to_vec();
        shortest.extend(8_u64.to_le_bytes());
        shortest.extend(0_u64.to_le_bytes());
        shortest.extend(0_u64.to_le_bytes());
        shortest.extend(0_u32.to_le_bytes());
        shortest.extend(0_u32.to_le_bytes());
        shortest.extend(FOOTER_MAGIC);
        assert_eq!(len(&shortest), 48);
        assert_eq!(index_refusal(&shortest), bad(INDEX_LENGTH));
        let dirs = day_file(&[put("F/", 0, Vec::new(), b"", 0)]);
        let entries = read_index(&dirs, len(&dirs)).unwrap();
        assert_eq!((entries.len(), entries[0].kind), (1, 0));
    }

    #[test]
    fn a_file_without_its_magic_or_footer_refuses() {
        assert_eq!(index_refusal(&small()[..47]), bad(NO_MAGIC));
        let mut wrong = small();
        wrong[7] = b'2';
        assert_eq!(index_refusal(&wrong), bad(NO_MAGIC));
        let mut footer = small();
        let end = footer.len();
        footer[end - 1] = b'2';
        assert_eq!(index_refusal(&footer), bad(NO_FOOTER));
    }

    #[test]
    fn an_index_that_does_not_end_the_file_refuses() {
        let good = small();
        let end = good.len();
        let off = u64::from_le_bytes(good[end - 40..end - 32].try_into().unwrap());
        for bad_off in [off + 1, off - 1, 7, u64::MAX] {
            let mut bytes = good.clone();
            patch(&mut bytes, end - 40, &bad_off.to_le_bytes());
            assert_eq!(index_refusal(&bytes), bad(INDEX_NOT_AT_END), "{bad_off}");
        }
        // An index_off below 8 with an index_len that still sums to the file.
        let mut low = good.clone();
        let index_len = u64::from_le_bytes(good[end - 32..end - 24].try_into().unwrap());
        patch(&mut low, end - 40, &4_u64.to_le_bytes());
        patch(&mut low, end - 32, &(index_len + off - 4).to_le_bytes());
        assert_eq!(index_refusal(&low), bad(INDEX_NOT_AT_END));
    }

    #[test]
    fn an_index_of_the_wrong_length_or_crc_refuses() {
        let good = small();
        let end = good.len();
        let raw = u64::from_le_bytes(good[end - 24..end - 16].try_into().unwrap());
        for wrong in [raw - 1, raw + 1] {
            let mut bytes = good.clone();
            patch(&mut bytes, end - 24, &wrong.to_le_bytes());
            assert_eq!(index_refusal(&bytes), bad(INDEX_LENGTH));
        }
        let mut crc = good.clone();
        crc[end - 16] ^= 1;
        assert_eq!(index_refusal(&crc), bad(INDEX_CRC));
        // A frame that is not zstd at all.
        let mut frame = good.clone();
        let off = u64::from_le_bytes(good[end - 40..end - 32].try_into().unwrap()) as usize;
        frame[off] ^= 0xFF;
        assert_eq!(index_refusal(&frame), bad(INDEX_LENGTH));
    }

    #[test]
    fn an_index_that_is_not_exactly_its_entries_refuses() {
        let odd = b"x".to_vec();
        let entries = [put("F/odd.csv", 2, raw_block(&odd), &odd, 0)];
        let (index, _) = index_of(&entries);
        assert_eq!(
            index_refusal(&day_file_with(&entries, &index, 2)),
            bad(INDEX_ENTRIES),
            "one entry short"
        );
        assert_eq!(
            index_refusal(&day_file_with(&entries, &index, 0)),
            bad(INDEX_ENTRIES),
            "bytes after the last entry"
        );
        assert_eq!(
            index_refusal(&day_file_with(&entries, &index[..index.len() - 1], 1)),
            bad(INDEX_ENTRIES),
            "an entry cut short"
        );
    }

    #[test]
    fn an_entry_of_another_kind_or_outside_the_blocks_refuses() {
        let odd = b"x".to_vec();
        let block = raw_block(&odd);
        let blocks_end = 8 + block.len() as u64;
        let refused = |kind: u8, off: u64, len: u64| {
            let mut entry = put("F/odd.csv", 2, block.clone(), &odd, 0);
            let (mut index, _) = index_of(std::slice::from_ref(&entry));
            let at = 2 + entry.name.len();
            index[at] = kind;
            index[at + 1..at + 9].copy_from_slice(&off.to_le_bytes());
            index[at + 9..at + 17].copy_from_slice(&len.to_le_bytes());
            entry.kind = kind;
            index_refusal(&day_file_with(&[entry], &index, 1))
        };
        assert_eq!(refused(3, 8, block.len() as u64), bad(ENTRY_KIND));
        assert_eq!(
            refused(0, 8, 1),
            bad(BLOCK_RANGE),
            "a directory with a block"
        );
        assert_eq!(
            refused(2, 7, block.len() as u64),
            bad(BLOCK_RANGE),
            "before the blocks"
        );
        assert_eq!(
            refused(1, 8, blocks_end),
            bad(BLOCK_RANGE),
            "past the index"
        );
        assert_eq!(
            refused(2, u64::MAX, 2),
            bad(BLOCK_RANGE),
            "an overflowing range"
        );
        // The edge itself is inside.
        let file = day_file(&[put("F/odd.csv", 2, block.clone(), &odd, 0)]);
        assert_eq!(
            read_index(&file, len(&file)).unwrap()[0].off + block.len() as u64,
            blocks_end
        );
    }

    #[test]
    fn a_block_that_is_not_its_entry_refuses() {
        let odd = b"x".to_vec();
        let mut file = day_file(&[put("F/odd.csv", 2, raw_block(&odd), &odd, 0)]);
        let entry = entry_of(&file, 0);
        file[8] = 1;
        assert_eq!(rebuild(&file, &entry), Err(bad(BLOCK_KIND)));
        let empty = IndexEntry {
            len: 0,
            ..entry.clone()
        };
        assert_eq!(rebuild(&file, &empty), Err(bad(BLOCK_KIND)));
        let longer = IndexEntry {
            size: 2,
            ..entry.clone()
        };
        file[8] = 2;
        assert_eq!(
            rebuild(&file, &longer),
            Err(bad(FRAME)),
            "a frame shorter than stated"
        );
        let shorter = IndexEntry { size: 0, ..entry };
        assert_eq!(
            rebuild(&file, &shorter),
            Err(bad(FRAME)),
            "a frame longer than stated"
        );
    }

    /// A columnar block with a field outside its range refuses by the rule.
    #[test]
    fn a_malformed_columnar_block_refuses() {
        let csv = n50_csv(true, true);
        let good = columnar_block(true, true, HEADER_OPEN_INTEREST, 3, &n50_columns());
        let refused = |block: Vec<u8>, rows: u64| {
            let file = day_file(&[put("F/n.csv", 1, block, &csv, rows)]);
            rebuilt(&file, 0).unwrap_err()
        };
        let header_end = 3 + 4 + HEADER_OPEN_INTEREST.len();
        for (at, byte) in [(1, 2), (2, 2)] {
            let mut block = good.clone();
            block[at] = byte;
            assert_eq!(refused(block, 3), bad(COLUMNAR), "byte {at}");
        }
        assert_eq!(
            refused(good[..header_end + 2].to_vec(), 3),
            bad(COLUMNAR),
            "cut short"
        );
        let mut ncols = good.clone();
        ncols[header_end] = 9;
        assert_eq!(refused(ncols, 3), bad(COLUMN_COUNT));
        assert_eq!(refused(good.clone(), 2), bad(ROW_COUNT));
        let mut tag = good.clone();
        tag[header_end + 12] = 2;
        assert_eq!(refused(tag, 3), bad(COLUMNAR), "tag 2");
        let mut after = good.clone();
        after.push(0);
        assert_eq!(refused(after, 3), bad(COLUMNAR), "bytes after the columns");
        let mut text_len = good.clone();
        text_len[header_end + 12 + 9] ^= 1;
        assert_eq!(
            refused(text_len, 3),
            bad(COLUMN_TEXT),
            "a text length off by one"
        );
    }

    /// A column whose fields are not one per row, or a numeric payload not
    /// of its stated shape, refuses (§4).
    #[test]
    fn a_column_that_is_not_its_rows_refuses() {
        let one = |col: Col<'_>, rows: u64| {
            let block = columnar_block(false, false, "V", rows, &[col]);
            let file = day_file(&[put("F/v.csv", 1, block, b"V", rows)]);
            rebuilt(&file, 0).unwrap_err()
        };
        assert_eq!(one(Col::Text(vec!["a", "b"]), 3), bad(COLUMN_TEXT));
        assert_eq!(
            one(Col::Text(vec!["a"]), 0),
            bad(COLUMN_TEXT),
            "text with no rows"
        );
        let num = |shapes: Vec<u8>, delta: bool, width: u8, values: Vec<i64>| Col::Num {
            shapes,
            delta,
            width,
            values,
            texts: vec!["0"],
        };
        assert_eq!(
            one(num(vec![0x20], false, 8, vec![360_000]), 1),
            bad(TIME_RANGE)
        );
        assert_eq!(one(num(vec![0x20], false, 8, vec![-1]), 1), bad(TIME_RANGE));
        let payload = |delta: u8, width: u8, planes: usize| {
            let mut p = vec![0, delta, width];
            p.extend(vec![0; planes]);
            p
        };
        for (p, why) in [
            (payload(2, 1, 1), "delta 2"),
            (payload(0, 9, 9), "width 9"),
            (payload(0, 1, 2), "a plane too long"),
            (payload(0, 2, 1), "a plane too short"),
        ] {
            assert_eq!(Column::numeric(&p, 1).err(), Some(bad(NUMERIC)), "{why}");
        }
        assert_eq!(
            Column::numeric(&[0, 0], u64::MAX).err(),
            Some(bad(NUMERIC)),
            "more rows than any payload"
        );
    }

    #[test]
    fn a_failed_read_is_unavailable() {
        struct Broken;
        impl ReadAt for Broken {
            fn read_at(&self, _: u64, _: &mut [u8]) -> std::io::Result<()> {
                Err(std::io::ErrorKind::Interrupted.into())
            }
        }
        assert_eq!(
            read_index(&Broken, 100),
            Err(CmRefusal::TickStoreUnavailable {
                kind: std::io::ErrorKind::Interrupted
            })
        );
    }

    #[test]
    fn every_tick_store_refusal_says_what_it_refused() {
        assert_eq!(
            bad(NO_MAGIC).to_string(),
            "tick store malformed: the file does not start with BRTXTS01"
        );
        assert_eq!(
            CmRefusal::TickStoreUnavailable {
                kind: std::io::ErrorKind::NotFound
            }
            .to_string(),
            "tick store unavailable: entity not found"
        );
    }

    // ── the store on disk, end to end ──────────────────────────────────────

    fn at() -> Day {
        Day::new(2024, 4, 1).unwrap()
    }

    fn nifty() -> InstrumentKey {
        InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap()
    }

    /// A store under a fresh temporary root holding the 2024-04-01 index day
    /// as `entries`.
    fn store(tag: &str, entries: &[Put]) -> (TickStore, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("gdfl_tickstore_{}_{tag}", std::process::id()));
        let store = TickStore::new(&root);
        let path = store.day_path(CmKind::Indices, at());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, day_file(entries)).unwrap();
        (store, root)
    }

    /// A day file is found at `cm/<TREE>/<yyyy>/<MON_yyyy>/<day folder>.bts`
    /// (§2), listed in its order, and every file is rebuilt and then held by
    /// `read_listed` to the stated size and CRC-32 before it decodes.
    #[test]
    fn a_day_file_is_read_end_to_end_through_the_source() {
        let csv = n50_csv(true, false);
        let block = columnar_block(true, false, HEADER_OPEN_INTEREST, 3, &n50_columns());
        let (store, root) = store(
            "Reads",
            &[
                put("GFDLCM_INDICES_TICK_01042024/", 0, Vec::new(), b"", 0),
                put(
                    "GFDLCM_INDICES_TICK_01042024/NIFTY 50.NSE_IDX.csv",
                    1,
                    block,
                    &csv,
                    3,
                ),
            ],
        );
        assert_eq!(
            store.day_path(CmKind::Stocks, Day::new(2023, 1, 19).unwrap()),
            root.join("cm/STOCKS/2023/JAN_2023/GFDLCM_STOCK_TICK_19012023.bts")
        );
        let listing = store.day(CmKind::Indices, at()).unwrap().unwrap();
        assert_eq!(listing.entries().len(), 2);
        assert_eq!(listing.unresolved(), 1, "the folder entry");
        let read = read_listed(&store, &listing, &nifty()).unwrap().unwrap();
        let expect = Expect {
            kind: CmKind::Indices,
            stem: "NIFTY 50.NSE_IDX",
            day: at(),
        };
        let want: CmFile = decode(&csv, &expect).unwrap();
        assert_eq!(read.file, want);
        assert_eq!(
            read_day(&store, &nifty(), at()).unwrap().unwrap().file,
            want
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Only the exact `.bts` name is opened: a day with only a `*.bts.tmp`
    /// left by an ingest, or with nothing, was not ingested and is `None`; a
    /// day file that cannot be read refuses `TickStoreUnavailable`; a rebuilt
    /// file that is not its stated size refuses before it decodes.
    #[test]
    fn a_temporary_or_absent_day_is_none_and_an_unreadable_one_refuses() {
        let csv = n50_csv(true, false);
        let block = columnar_block(true, false, HEADER_OPEN_INTEREST, 3, &n50_columns());
        let mut wrong = put(
            "GFDLCM_INDICES_TICK_01042024/NIFTY 50.NSE_IDX.csv",
            1,
            block,
            &csv,
            3,
        );
        wrong.size += 1;
        let (store, root) = store("Absent", &[wrong]);
        assert_eq!(
            read_day(&store, &nifty(), at()),
            Err(CmRefusal::SourceLengthMismatch {
                file: len(&csv),
                member: len(&csv) + 1
            })
        );
        let next = Day::new(2024, 4, 2).unwrap();
        let tmp = store
            .day_path(CmKind::Indices, next)
            .with_extension("bts.tmp");
        std::fs::write(&tmp, small()).unwrap();
        assert!(matches!(store.day(CmKind::Indices, next), Ok(None)));
        assert!(matches!(store.day(CmKind::Stocks, at()), Ok(None)));
        let blocked = store.day_path(CmKind::Indices, next);
        std::fs::create_dir_all(&blocked).unwrap();
        assert!(matches!(
            store.day(CmKind::Indices, next),
            Err(CmRefusal::TickStoreUnavailable { .. })
        ));
        let third = Day::new(2024, 4, 3).unwrap();
        let locked = store.day_path(CmKind::Indices, third);
        std::fs::write(&locked, small()).unwrap();
        let mut perms = std::fs::metadata(&locked).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o000);
        std::fs::set_permissions(&locked, perms).unwrap();
        assert!(matches!(
            store.day(CmKind::Indices, third),
            Err(CmRefusal::TickStoreUnavailable { .. })
        ));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
