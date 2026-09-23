//! Opening a lake file and decoding a row group out of it.
//!
//! # Cost, stated plainly
//!
//! [`LakeFile::open`] reads the whole file and parses its thrift footer. That
//! is O(file bytes) and it is not pretended otherwise anywhere in this crate.
//! [`LakeFile::read_row_group`] decompresses every page of every column in the
//! group, which is O(bytes in the group). Both are real work paid once.
//! [`crate::batch::Batch::row`] is the operation that carries a bound, and its
//! proof is named there.
//!
//! # Why the whole file is read into memory
//!
//! Lake files are per contract, per timeframe, per month, and the largest one
//! measured is a few megabytes. `CLAUDE.md` §4 bans a **writable** memory
//! mapping — it raises a signal on a full disk and a signal cannot be caught —
//! and a read-only mapping would need a dependency outside the set this crate
//! is allowed. A plain read is honest about its cost and cannot fault.

use std::fs;
use std::path::Path;

use brutex_core::price::Paisa;
use bytes::Bytes;
use parquet::basic::Compression;
use parquet::column::reader::ColumnReader;
use parquet::file::metadata::{ParquetMetaData, ParquetMetaDataReader};

use crate::bar::{Greeks, OPEN_INTEREST_NULL, paisa_from_lake};
use crate::batch::Batch;
use crate::error::LakeError;
use crate::page::{Codec, LakePageReader};
use crate::schema::{Layout, detect};

/// The Parquet magic, at both ends of every file.
const MAGIC: [u8; 4] = *b"PAR1";

/// `PAR1` + a four-byte footer length + `PAR1`.
const MIN_FILE: usize = 12;

/// An opened lake Parquet file.
pub struct LakeFile {
    bytes: Bytes,
    meta: ParquetMetaData,
    layout: Layout,
}

impl core::fmt::Debug for LakeFile {
    /// Written by hand because the derived form would dump the whole file —
    /// megabytes of compressed pages — into a test failure message.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LakeFile")
            .field("layout", &self.layout)
            .field("bytes", &self.bytes.len())
            .field("row_groups", &self.meta.num_row_groups())
            .field("num_rows", &self.meta.file_metadata().num_rows())
            .finish()
    }
}

impl LakeFile {
    /// Opens a lake file, refusing anything that is not one.
    ///
    /// The magic bytes are checked before the footer is handed to the thrift
    /// parser. That ordering is deliberate: a JPEG given to the parser
    /// produces an opaque thrift error, and an operator needs to be told the
    /// file is simply not Parquet.
    ///
    /// # Errors
    ///
    /// * [`LakeError::Io`] if the file cannot be read.
    /// * [`LakeError::Truncated`] if it is shorter than a Parquet file can be.
    /// * [`LakeError::NotParquet`] if either magic is missing.
    /// * [`LakeError::FooterUnreadable`] if the footer will not parse.
    /// * [`LakeError::UnexpectedSchema`], [`LakeError::MissingColumn`] or
    ///   [`LakeError::ColumnTypeMismatch`] if the columns are not one of the
    ///   two shapes the lake contains.
    pub fn open(path: &Path) -> Result<Self, LakeError> {
        let raw = fs::read(path).map_err(|e| LakeError::Io {
            reason: e.to_string(),
        })?;
        let bytes = raw.len();
        let opened = Self::from_bytes(raw);
        // ONE LINE PER FILE. Never per row, and never per row group here —
        // a parquet row group holds tens of thousands of rows and this lake
        // holds millions, so a finer event would roll a run out of the 64 MiB
        // window before it finished. D-0075.
        //
        // THE TWO OUTCOMES ARE NOT INTERCHANGEABLE, which is why the level
        // moves. A lake file that opens and holds nothing is an ordinary state.
        // One that will NOT open is a file on disk this build cannot read —
        // and before this line, both reached a page as the same silence.
        let (level, said, rows) = match &opened {
            Ok(file) => (telemetry::Level::Debug, "opened", file.num_rows()),
            Err(_) => (telemetry::Level::Error, "refused", 0),
        };
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::new(level, "lake.file", said)
                .with("path", telemetry::Value::Str(&path.display().to_string()))
                .with("bytes", telemetry::Value::Uint(bytes as u64))
                .with("rows", telemetry::Value::Int(rows)),
        );
        opened
    }

    /// Same as [`LakeFile::open`], for bytes already in hand.
    ///
    /// # Errors
    ///
    /// As [`LakeFile::open`], less the I/O failure.
    pub fn from_bytes(raw: Vec<u8>) -> Result<Self, LakeError> {
        if raw.len() < MIN_FILE {
            return Err(LakeError::Truncated {
                len: raw.len(),
                minimum: MIN_FILE,
            });
        }
        let head = first_four(&raw).ok_or(LakeError::Truncated {
            len: raw.len(),
            minimum: MIN_FILE,
        })?;
        let tail = last_four(&raw).ok_or(LakeError::Truncated {
            len: raw.len(),
            minimum: MIN_FILE,
        })?;
        if head != MAGIC || tail != MAGIC {
            return Err(LakeError::NotParquet { head, tail });
        }

        let bytes = Bytes::from(raw);
        let meta = ParquetMetaDataReader::new()
            .parse_and_finish(&bytes)
            .map_err(|e| LakeError::FooterUnreadable {
                reason: e.to_string(),
            })?;
        let layout = detect(meta.file_metadata().schema_descr())?;
        Ok(Self {
            bytes,
            meta,
            layout,
        })
    }

    /// Which of the two column shapes this file has.
    #[must_use]
    pub const fn layout(&self) -> Layout {
        self.layout
    }

    /// How many row groups the file holds.
    #[must_use]
    pub fn row_groups(&self) -> usize {
        self.meta.num_row_groups()
    }

    /// How many rows the file holds in total.
    #[must_use]
    pub fn num_rows(&self) -> i64 {
        self.meta.file_metadata().num_rows()
    }

    /// What wrote the file, when it says.
    #[must_use]
    pub fn created_by(&self) -> Option<&str> {
        self.meta.file_metadata().created_by()
    }

    /// Decodes one row group.
    ///
    /// # Errors
    ///
    /// [`LakeError::NoSuchRowGroup`] past the end;
    /// [`LakeError::ImpossibleLength`] if the group's declared row count is
    /// negative or wider than the file that holds it;
    /// [`LakeError::UnknownCodec`]
    /// for a compression this reader does not implement;
    /// [`LakeError::PageDecode`] if a page will not decode;
    /// [`LakeError::UnexpectedNull`] if a column that must not be null is;
    /// [`LakeError::NotRepresentable`] if a price will not become paisa.
    pub fn read_row_group(&self, index: usize) -> Result<Batch, LakeError> {
        let held = self.row_groups();
        if index >= held {
            return Err(LakeError::NoSuchRowGroup { asked: index, held });
        }
        let group = self.meta.row_group(index);
        let declared_rows = group.num_rows();
        let rows = usize::try_from(declared_rows).map_err(|_| LakeError::ImpossibleLength {
            what: "row group row count",
            value: declared_rows,
        })?;

        // THE ROW COUNT IS A NUMBER THE FILE CHOSE, AND SEVEN RESERVATION
        // SITES BELOW SPEND IT UNREAD. `Columns::int64`, `int32` and `double`
        // each open with `Vec::with_capacity(self.rows)` twice — once for
        // values, once for definition levels — and `Columns::greeks` does it
        // once more for its output. Until this bound existed the only thing
        // between a corrupt footer and those calls was the `usize::try_from`
        // above, which rejects a negative and nothing else. `num_rows = 1 << 40`
        // is a perfectly positive `i64`, and it asks for an 8 TiB `Vec<i64>`.
        //
        // AN OVERSIZED RESERVATION IS NOT A REFUSAL. `Vec::with_capacity`
        // reports nothing: when the allocator says no it calls
        // `std::alloc::handle_alloc_error`, which aborts. That is not a panic,
        // so `catch_unwind` and a test harness cannot see it either, and the
        // root `Cargo.toml`'s `panic = "abort"` in `[profile.release]` removes
        // even the theoretical unwind. Whether a request that large is refused
        // outright or handed back as address space the kernel cannot back is
        // the machine's overcommit policy and not this reader's choice — that
        // second outcome is UNMEASURED here, and neither of them is a refusal
        // an operator can read. It is the same death parquet's own
        // `ColumnChunkMetaData::byte_range` assert delivers, which the comment
        // in `Self::pages` below describes and refuses; this is the second door
        // into it. `CLAUDE.md` §4 — degrade loudly and name the reason, or
        // refuse — and an abort does neither.
        //
        // `Vec::try_reserve` would return an error instead, but it would return
        // it at the reservation, several frames from the footer field that
        // lied, with only a byte count to report. The refusal belongs here,
        // where the impossible number can still be named against the file it
        // came from.
        //
        // THE CEILING IS THE FILE'S OWN LENGTH, AND IT IS DELIBERATELY CRUDE.
        // `LakeFile` holds every byte of the file in memory — the module header
        // says why — so `self.bytes.len()` is exact and free to read. Past this
        // check every reservation in this function is at most a small constant
        // multiple of bytes already resident, which is the property worth
        // having: peak memory becomes proportional to the file, not to a number
        // the file asked for.
        //
        // WHAT THIS IS NOT. It is not a proof of Parquet legality. A column of
        // pure nulls RLE-encodes an arbitrarily long run of levels in a handful
        // of bytes, so a *legal* Parquet file can declare more rows than it has
        // bytes, and this refuses one. No such file is in the lake: every lake
        // leaf is an unnested OPTIONAL primitive, `timestamp`, `open`, `high`,
        // `low`, `close` and `volume` are null in none of the 170,547 F&O rows
        // and 78,448 cash/index rows measured — the measurement
        // `LakeError::UnexpectedNull` cites — and the largest file measured is
        // a few megabytes against row counts in the thousands. The margin on
        // every real file is orders of magnitude, and if a file ever does sit
        // the other side of it, this names the file instead of dying on it.
        //
        // Nor does it make allocation failure impossible. A multi-megabyte
        // group still reserves tens of megabytes here and a machine already at
        // its limit can still lose that. What it removes is the case where one
        // corrupt footer field demands terabytes.
        if rows > self.bytes.len() {
            return Err(LakeError::ImpossibleLength {
                what: "row group row count wider than the file that holds it",
                value: declared_rows,
            });
        }

        let mut cols = Columns::new(self, index, rows);

        let timestamp = cols.int64("timestamp")?;
        let timestamp = require(timestamp, "timestamp")?;
        let open = cols.price("open")?;
        let high = cols.price("high")?;
        let low = cols.price("low")?;
        let close = cols.price("close")?;
        let volume = require(cols.int64("volume")?, "volume")?;

        // Open interest is the one column whose null is meaningful, and
        // CLAUDE.md section 7 fixes its representation: i64::MIN, distinct
        // from a real zero.
        let open_interest = cols
            .int64("open_interest")?
            .into_iter()
            .map(|v| v.unwrap_or(OPEN_INTEREST_NULL))
            .collect();

        let (spot_at_bar, greeks, greeks_provenance_id) = if self.layout.has_greeks() {
            let spot = cols.optional_price("spot_at_bar")?;
            let g = cols.greeks()?;
            let prov = cols.int32("greeks_provenance_id")?;
            (spot, g, prov)
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        Ok(Batch::new(
            self.layout,
            timestamp,
            open,
            high,
            low,
            close,
            volume,
            open_interest,
            spot_at_bar,
            greeks,
            greeks_provenance_id,
        ))
    }

    /// Every row group in order, decoded one at a time.
    ///
    /// # Errors
    ///
    /// As [`LakeFile::read_row_group`], on the first group that fails.
    pub fn read_all(&self) -> Result<Vec<Batch>, LakeError> {
        (0..self.row_groups())
            .map(|i| self.read_row_group(i))
            .collect()
    }
}

/// Turns a column of optionals into a column of values, refusing a null.
fn require<T>(values: Vec<Option<T>>, column: &'static str) -> Result<Vec<T>, LakeError> {
    let mut out = Vec::with_capacity(values.len());
    for (row, v) in values.into_iter().enumerate() {
        match v {
            Some(v) => out.push(v),
            None => return Err(LakeError::UnexpectedNull { column, row }),
        }
    }
    Ok(out)
}

fn first_four(raw: &[u8]) -> Option<[u8; 4]> {
    raw.get(0..4)?.try_into().ok()
}

fn last_four(raw: &[u8]) -> Option<[u8; 4]> {
    raw.get(raw.len().checked_sub(4)?..)?.try_into().ok()
}

/// Decodes named columns out of one row group.
struct Columns<'a> {
    file: &'a LakeFile,
    group: usize,
    rows: usize,
}

impl<'a> Columns<'a> {
    const fn new(file: &'a LakeFile, group: usize, rows: usize) -> Self {
        Self { file, group, rows }
    }

    /// The leaf index of a column by name.
    fn index_of(&self, name: &'static str) -> Result<usize, LakeError> {
        let schema = self.file.meta.file_metadata().schema_descr();
        (0..schema.num_columns())
            .find(|i| schema.column(*i).name() == name)
            .ok_or(LakeError::MissingColumn { name })
    }

    /// Builds a page reader over one column chunk, refusing an unknown codec.
    fn pages(&self, name: &'static str) -> Result<(LakePageReader, usize), LakeError> {
        let i = self.index_of(name)?;
        let group = self.file.meta.row_group(self.group);
        let chunk = group.column(i);

        let codec = match chunk.compression() {
            Compression::UNCOMPRESSED => Codec::Uncompressed,
            Compression::ZSTD(_) => Codec::Zstd,
            other => {
                // Named, never skipped. Enabling the parquet features for the
                // other codecs would pull a C library; see CLAUDE.md section 2.
                return Err(LakeError::UnknownCodec {
                    column: name.to_owned(),
                    codec: format!("{other:?}"),
                });
            }
        };

        // PARQUET'S OWN ACCESSOR ABORTS HERE, so it is not the one called.
        // `ColumnChunkMetaData::byte_range` (parquet 59.2,
        // `src/file/metadata/mod.rs`) ends in
        //
        //     assert!(col_start >= 0 && col_len >= 0,
        //             "column start and length should not be negative");
        //
        // and a lake file whose footer parsed but whose values are corrupt is
        // exactly the input that reaches it: a negative `dictionary_page_offset`
        // or `total_compressed_size` killed the process instead of naming the
        // file. `CLAUDE.md` §4 — degrade loudly and name the reason, or refuse,
        // never abort — so the two fields `byte_range` would have read are read
        // here and refused by name. The arithmetic below is what it does, minus
        // the abort; `usize::try_from` rejects a negative on every target, and
        // on a 32-bit one it also rejects an offset past the address space.
        //
        // The offending number is also what gets reported. It used to be
        // `chunk.num_values()`, which named a different field entirely and sent
        // the operator looking in the wrong place.
        let declared_start = chunk
            .dictionary_page_offset()
            .unwrap_or_else(|| chunk.data_page_offset());
        let declared_len = chunk.compressed_size();
        let start = usize::try_from(declared_start).map_err(|_| LakeError::ImpossibleLength {
            what: "column chunk offset",
            value: declared_start,
        })?;
        let len = usize::try_from(declared_len).map_err(|_| LakeError::ImpossibleLength {
            what: "column chunk length",
            value: declared_len,
        })?;
        let end = start.checked_add(len).ok_or(LakeError::ImpossibleLength {
            what: "column chunk extent",
            value: declared_len,
        })?;
        let slice = self
            .file
            .bytes
            .get(start..end)
            .ok_or_else(|| LakeError::PageDecode {
                column: name.to_owned(),
                reason: format!("column chunk {start}..{end} runs past the end of the file"),
            })?;

        Ok((
            LakePageReader::new(Bytes::copy_from_slice(slice), chunk.num_values(), codec),
            i,
        ))
    }

    /// Expands contiguous non-null values against definition levels.
    ///
    /// Parquet stores only the values that are present, so a column of 2,480
    /// rows with 300 nulls holds 2,180 values. The definition levels say which
    /// rows they belong to, and this is where the two are put back together.
    /// Getting it wrong would shift every value after the first null onto the
    /// wrong bar.
    ///
    /// # There is no padding arm, and its absence is the point
    ///
    /// This function used to answer a short chunk with nulls: it walked
    /// `0..rows` and pushed `None` for every row the levels did not reach, and
    /// for every row whose level said PRESENT once the values had run out. A
    /// chunk delivering 400 of 2,480 rows was therefore *accepted*, with 2,080
    /// nulls this crate invented and nothing downstream able to tell them from
    /// real ones. On `open_interest` each was an invented `i64::MIN`, the
    /// `CLAUDE.md` §7 sentinel. `CLAUDE.md` §4 bans exactly that: a fallback
    /// that hides a failure. Both conditions are now
    /// [`LakeError::ShortColumnChunk`], named with the column, the row, and
    /// the two counts.
    ///
    /// **Neither condition is a Parquet-legal file**, which is why refusing
    /// costs nothing real:
    ///
    /// * *Fewer levels than rows.* `parquet.thrift` (shipped verbatim as
    ///   `parquet-format-safe-0.2.4/parquet.thrift`) defines `RowGroup` field
    ///   3 `num_rows` as "Number of rows in this row group",
    ///   `ColumnMetaData` field 5 `num_values` as "Number of values in this
    ///   column", and `DataPageHeader` field 1 `num_values` as "Number of
    ///   values, including NULLs, in this data page". For an unnested,
    ///   unrepeated leaf — maximum definition level 1, maximum repetition
    ///   level 0, the only shape `crate::schema::detect` accepts — one value
    ///   including nulls is one row, so the levels must cover every row of the
    ///   group. Short levels mean the chunk stopped early, which is byte loss,
    ///   not a tail of nulls.
    /// * *Fewer values than the levels claim.* `DataPageHeaderV2` in the same
    ///   file states "Number of non-null = num\_values - num\_nulls which is
    ///   also the number of values in the data section": the data section's
    ///   length is *derived* from the levels, so a page holding fewer values
    ///   is malformed rather than a page with bonus nulls. `parquet` 59.2's
    ///   own `read_records_with_reservation`
    ///   (`src/column/reader.rs:290`) agrees and refuses first —
    ///
    ///   ```text
    ///   let values_read = self.values_decoder.read(values, values_to_read)?;
    ///   if values_read != values_to_read {
    ///       return Err(general_err!(
    ///           "insufficient values read from column - expected: \
    ///            {values_to_read}, got: {values_read}",
    ///       ));
    ///   }
    ///   ```
    ///
    ///   so this arm is defence on a helper rather than a path a file reaches.
    ///   It is written as a refusal and not an `unwrap` because the comment
    ///   that used to sit here claimed a refusal the code did not perform.
    ///
    /// Only two levels can arrive. `crate::schema::detect` refuses any leaf
    /// whose maximum definition level is not 1, and the RLE/bit-packed hybrid
    /// encodes levels at `ceil(log2(max_def_level + 1))` bits — one bit — so
    /// the decoder cannot produce a 2 to be silently read as null.
    fn expand<T: Copy>(
        column: &'static str,
        values: &[T],
        defs: &[i16],
        rows: usize,
    ) -> Result<Vec<Option<T>>, LakeError> {
        if defs.len() != rows {
            return Err(LakeError::ShortColumnChunk {
                column,
                row: defs.len().min(rows),
                expected: rows,
                arrived: defs.len(),
            });
        }
        let mut out = Vec::with_capacity(rows);
        let mut next = 0usize;
        for (row, level) in defs.iter().enumerate() {
            // A definition level of 1 means present; 0 means null.
            if *level == 1 {
                let Some(v) = values.get(next) else {
                    return Err(LakeError::ShortColumnChunk {
                        column,
                        row,
                        // The scan is on the refusal path only, and the read
                        // it belongs to ends here.
                        expected: defs.iter().filter(|d| **d == 1).count(),
                        arrived: values.len(),
                    });
                };
                next += 1;
                out.push(Some(*v));
            } else {
                out.push(None);
            }
        }
        Ok(out)
    }

    fn int64(&mut self, name: &'static str) -> Result<Vec<Option<i64>>, LakeError> {
        let (pages, i) = self.pages(name)?;
        let desc = self.file.meta.file_metadata().schema_descr().column(i);
        let mut values: Vec<i64> = Vec::with_capacity(self.rows);
        let mut defs: Vec<i16> = Vec::with_capacity(self.rows);
        match parquet::column::reader::get_column_reader(desc, Box::new(pages)) {
            ColumnReader::Int64ColumnReader(mut r) => {
                // `read_records` answers `(records, values, levels)`, and a
                // short read shows up as `records < self.rows`. The triple is
                // not the check here for one reason: `levels` IS `defs.len()`
                // on an unnested leaf at definition level 1, which is the shape
                // `schema::detect` has already insisted on, and `defs.len()`
                // against `self.rows` is checked inside `Self::expand` — the
                // one function that used to pad the shortfall with nulls.
                // Checking the same number twice would leave a branch no file
                // can reach and a mutant no test can kill.
                r.read_records(self.rows, Some(&mut defs), None, &mut values)
                    .map_err(|e| LakeError::PageDecode {
                        column: name.to_owned(),
                        reason: e.to_string(),
                    })?;
            }
            _ => return Err(self.mismatch(name)),
        }
        Self::expand(name, &values, &defs, self.rows)
    }

    fn int32(&mut self, name: &'static str) -> Result<Vec<Option<i32>>, LakeError> {
        let (pages, i) = self.pages(name)?;
        let desc = self.file.meta.file_metadata().schema_descr().column(i);
        let mut values: Vec<i32> = Vec::with_capacity(self.rows);
        let mut defs: Vec<i16> = Vec::with_capacity(self.rows);
        match parquet::column::reader::get_column_reader(desc, Box::new(pages)) {
            ColumnReader::Int32ColumnReader(mut r) => {
                // The triple is discarded for the reason `Self::int64` states.
                r.read_records(self.rows, Some(&mut defs), None, &mut values)
                    .map_err(|e| LakeError::PageDecode {
                        column: name.to_owned(),
                        reason: e.to_string(),
                    })?;
            }
            _ => return Err(self.mismatch(name)),
        }
        Self::expand(name, &values, &defs, self.rows)
    }

    fn double(&mut self, name: &'static str) -> Result<Vec<Option<f64>>, LakeError> {
        let (pages, i) = self.pages(name)?;
        let desc = self.file.meta.file_metadata().schema_descr().column(i);
        let mut values: Vec<f64> = Vec::with_capacity(self.rows);
        let mut defs: Vec<i16> = Vec::with_capacity(self.rows);
        match parquet::column::reader::get_column_reader(desc, Box::new(pages)) {
            ColumnReader::DoubleColumnReader(mut r) => {
                // The triple is discarded for the reason `Self::int64` states.
                r.read_records(self.rows, Some(&mut defs), None, &mut values)
                    .map_err(|e| LakeError::PageDecode {
                        column: name.to_owned(),
                        reason: e.to_string(),
                    })?;
            }
            _ => return Err(self.mismatch(name)),
        }
        Self::expand(name, &values, &defs, self.rows)
    }

    /// The type this reader wanted against the type the file holds.
    fn mismatch(&self, name: &'static str) -> LakeError {
        let want = crate::schema::specs(self.file.layout)
            .find(|s| s.name == name)
            .map_or(crate::error::ColumnType::Other, |s| s.ty);
        let got = self
            .index_of(name)
            .map_or(crate::error::ColumnType::Other, |i| {
                crate::schema::physical(
                    self.file
                        .meta
                        .file_metadata()
                        .schema_descr()
                        .column(i)
                        .physical_type(),
                )
            });
        LakeError::ColumnTypeMismatch { name, want, got }
    }

    /// A price column that must be present on every row.
    fn price(&mut self, name: &'static str) -> Result<Vec<Paisa>, LakeError> {
        let raw = self.double(name)?;
        let mut out = Vec::with_capacity(raw.len());
        for (row, v) in raw.into_iter().enumerate() {
            let Some(rupees) = v else {
                return Err(LakeError::UnexpectedNull { column: name, row });
            };
            out.push(paisa_from_lake(name, row, rupees)?);
        }
        Ok(out)
    }

    /// A price column that may be null on any row.
    fn optional_price(&mut self, name: &'static str) -> Result<Vec<Option<Paisa>>, LakeError> {
        let raw = self.double(name)?;
        let mut out = Vec::with_capacity(raw.len());
        for (row, v) in raw.into_iter().enumerate() {
            match v {
                Some(rupees) => out.push(Some(paisa_from_lake(name, row, rupees)?)),
                None => out.push(None),
            }
        }
        Ok(out)
    }

    /// The eight-value greeks block, assembled per row.
    ///
    /// Measured across 120 real F&O files and 170,547 rows: these eight are
    /// null as one unit. A row where some are present and some are not is
    /// therefore a fact this reader has never seen, and it is refused rather
    /// than papered over — emitting `None` would silently discard the greeks
    /// that were there, and filling the gaps with zero would invent them.
    fn greeks(&mut self) -> Result<Vec<Option<Greeks>>, LakeError> {
        let iv = self.double("iv")?;
        let delta = self.double("delta")?;
        let gamma = self.double("gamma")?;
        let theta = self.double("theta")?;
        let vega = self.double("vega")?;
        let rho = self.double("rho")?;
        let t = self.double("t_years_used")?;
        let rate = self.double("rate_used")?;

        let mut out = Vec::with_capacity(self.rows);
        for row in 0..self.rows {
            let cells = [
                iv.get(row).copied().flatten(),
                delta.get(row).copied().flatten(),
                gamma.get(row).copied().flatten(),
                theta.get(row).copied().flatten(),
                vega.get(row).copied().flatten(),
                rho.get(row).copied().flatten(),
                t.get(row).copied().flatten(),
                rate.get(row).copied().flatten(),
            ];
            let present = cells.iter().filter(|c| c.is_some()).count();
            match present {
                0 => out.push(None),
                8 => {
                    let [iv, delta, gamma, theta, vega, rho, t_years_used, rate_used] = cells;
                    out.push(Some(Greeks {
                        iv: unwrap_present(iv, "iv", row)?,
                        delta: unwrap_present(delta, "delta", row)?,
                        gamma: unwrap_present(gamma, "gamma", row)?,
                        theta: unwrap_present(theta, "theta", row)?,
                        vega: unwrap_present(vega, "vega", row)?,
                        rho: unwrap_present(rho, "rho", row)?,
                        t_years_used: unwrap_present(t_years_used, "t_years_used", row)?,
                        rate_used: unwrap_present(rate_used, "rate_used", row)?,
                    }));
                }
                n => {
                    return Err(LakeError::PartialGreeks {
                        row,
                        present: u8::try_from(n).unwrap_or(u8::MAX),
                    });
                }
            }
        }
        Ok(out)
    }
}

/// Unwraps a cell the caller has already counted as present.
///
/// Written as a fallible helper rather than an `unwrap` because
/// `clippy::unwrap_used` is denied workspace-wide and a "this cannot happen"
/// comment is not a proof.
fn unwrap_present(v: Option<f64>, column: &'static str, row: usize) -> Result<f64, LakeError> {
    v.ok_or(LakeError::UnexpectedNull { column, row })
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes"
)]
mod tests {
    use super::*;

    use std::io::Cursor;
    use std::sync::Arc;

    // The writer half of `parquet`, and the thrift codec from
    // `parquet-format-safe`. Both are ordinary dependencies of this crate
    // rather than dev-dependencies, so a unit test inside the library can
    // reach them; neither pulls a feature `Cargo.toml` refused. The writer is
    // available with no cargo features enabled, which is why it does not
    // reintroduce the C zstd binding `CLAUDE.md` §2 bans.
    use parquet::basic::{Repetition, Type as PhysicalType};
    use parquet::column::writer::ColumnWriter;
    use parquet::file::properties::WriterProperties;
    use parquet::file::writer::SerializedFileWriter;
    use parquet::schema::types::Type;
    use parquet_format_safe::FileMetaData;
    use parquet_format_safe::thrift::protocol::{TCompactInputProtocol, TCompactOutputProtocol};

    #[test]
    fn a_file_that_is_not_parquet_is_refused_by_name() {
        // A JPEG header, which is emphatically not Parquet.
        let jpeg = vec![0xff, 0xd8, 0xff, 0xe0, 0, 0, 0, 0, 0, 0, 0, 0];
        match LakeFile::from_bytes(jpeg) {
            Err(LakeError::NotParquet { head, .. }) => {
                assert_eq!(head, [0xff, 0xd8, 0xff, 0xe0]);
            }
            other => panic!("expected NotParquet, got {other:?}", other = other.err()),
        }
    }

    #[test]
    fn a_file_with_the_head_magic_but_not_the_tail_is_refused() {
        let mut v = b"PAR1".to_vec();
        v.extend_from_slice(&[0; 8]);
        match LakeFile::from_bytes(v) {
            Err(LakeError::NotParquet { head, tail }) => {
                assert_eq!(head, *b"PAR1");
                assert_eq!(tail, [0, 0, 0, 0]);
            }
            other => panic!("expected NotParquet, got {other:?}", other = other.err()),
        }
    }

    #[test]
    fn a_truncated_file_is_refused_by_name_not_skipped() {
        for len in 0..MIN_FILE {
            let v = vec![b'P'; len];
            match LakeFile::from_bytes(v) {
                Err(LakeError::Truncated { len: got, minimum }) => {
                    assert_eq!(got, len);
                    assert_eq!(minimum, MIN_FILE);
                }
                other => panic!("len {len}: expected Truncated, got {:?}", other.err()),
            }
        }
    }

    #[test]
    fn a_file_with_both_magics_but_a_junk_footer_is_refused() {
        let mut v = b"PAR1".to_vec();
        v.extend_from_slice(&[0xaa; 32]);
        v.extend_from_slice(&[0xff, 0xff, 0xff, 0x7f]); // absurd footer length
        v.extend_from_slice(b"PAR1");
        assert!(matches!(
            LakeFile::from_bytes(v),
            Err(LakeError::FooterUnreadable { .. })
        ));
    }

    #[test]
    fn expand_puts_values_back_on_the_rows_they_belong_to() {
        // Three rows, middle one null: the value 20 must land on row 2, not
        // row 1. This is the bug that would shift a whole series by one bar.
        let got = Columns::expand("open_interest", &[10_i64, 20], &[1, 0, 1], 3).unwrap();
        assert_eq!(got, vec![Some(10), None, Some(20)]);

        // All null.
        assert_eq!(
            Columns::expand::<i64>("open_interest", &[], &[0, 0], 2).unwrap(),
            vec![None, None]
        );
        // All present.
        assert_eq!(
            Columns::expand("open_interest", &[1_i64, 2], &[1, 1], 2).unwrap(),
            vec![Some(1), Some(2)]
        );
        // A group of no rows is empty, not an error.
        assert_eq!(
            Columns::expand::<i64>("open_interest", &[], &[], 0).unwrap(),
            Vec::new()
        );
    }

    /// **Fewer levels than rows is a SHORT CHUNK, and it is not legitimate
    /// Parquet.**
    ///
    /// This case used to read `// Fewer levels than rows: the tail is null
    /// rather than a panic` over an assertion that `expand(&[1], &[1], 3)`
    /// answers `[Some(1), None, None]`. "Rather than a panic" is a false
    /// dichotomy — `CLAUDE.md` §4 offers a third answer, a named refusal — and
    /// the two invented nulls were the whole of defect 1 in miniature.
    ///
    /// What the format says, from `parquet.thrift` as shipped verbatim in
    /// `parquet-format-safe-0.2.4`: `RowGroup` field 3 `num_rows` is "Number
    /// of rows in this row group"; `ColumnMetaData` field 5 `num_values` is
    /// "Number of values in this column"; `DataPageHeader` field 1
    /// `num_values` is "Number of values, including NULLs, in this data page".
    /// Every lake leaf is an unnested optional primitive — maximum definition
    /// level 1, maximum repetition level 0, which `schema::detect` now insists
    /// on — so one value including nulls is exactly one row and the levels
    /// must cover the whole group. Fewer levels than rows therefore means the
    /// chunk stopped early: byte loss, not a tail of nulls.
    #[test]
    fn fewer_levels_than_the_row_count_is_refused_as_a_short_chunk() {
        match Columns::expand("open_interest", &[1_i64], &[1], 3) {
            Err(LakeError::ShortColumnChunk {
                column,
                row,
                expected,
                arrived,
            }) => {
                assert_eq!(column, "open_interest");
                assert_eq!(row, 1, "row 1 is the first the chunk cannot deliver");
                assert_eq!(expected, 3);
                assert_eq!(arrived, 1);
            }
            other => panic!("expected ShortColumnChunk, got {other:?}"),
        }

        // More levels than rows is the same fault seen from the other side,
        // and is refused too rather than truncated to fit.
        assert!(matches!(
            Columns::expand("volume", &[1_i64, 2], &[1, 1], 1),
            Err(LakeError::ShortColumnChunk {
                expected: 1,
                arrived: 2,
                ..
            })
        ));
    }

    /// **Fewer values than the levels claim REFUSES — which is what the
    /// comment always said and what the code did not do.**
    ///
    /// The line this replaces read `// Fewer values than levels claim: refuses
    /// to invent one` over `assert_eq!(Columns::expand::<i64>(&[], &[1], 1),
    /// vec![None])`. That return value *is* an invented one: a null on a row
    /// whose definition level says PRESENT.
    ///
    /// The refusal is what the format and the decoder both do.
    /// `DataPageHeaderV2` in `parquet.thrift`: "Number of non-null =
    /// `num_values` - `num_nulls` which is also the number of values in the
    /// data section" — the data section's length is derived from the levels,
    /// so a page holding fewer is malformed. And `parquet` 59.2's own
    /// `read_records_with_reservation` (`src/column/reader.rs:290`) returns
    /// `insufficient values read from column - expected: {n}, got: {m}` before
    /// this helper is ever reached, which is why this arm is defence on a
    /// helper rather than a path a file can take.
    #[test]
    fn fewer_values_than_the_levels_claim_is_refused_and_never_invents_a_null() {
        match Columns::expand::<i64>("open_interest", &[], &[1], 1) {
            Err(LakeError::ShortColumnChunk {
                column,
                row,
                expected,
                arrived,
            }) => {
                assert_eq!(column, "open_interest");
                assert_eq!(row, 0);
                assert_eq!(expected, 1, "one level claims PRESENT");
                assert_eq!(arrived, 0, "and no value arrived behind it");
            }
            other => panic!("expected ShortColumnChunk, got {other:?}"),
        }

        // The shortfall is named at the row it happens on, not at row 0: two
        // values behind three PRESENT levels runs out at row 3, and the nulls
        // in between are real ones that must not be miscounted.
        match Columns::expand("iv", &[1.0_f64, 2.0], &[1, 0, 1, 0, 1], 5) {
            Err(LakeError::ShortColumnChunk {
                row,
                expected,
                arrived,
                ..
            }) => {
                assert_eq!(row, 4);
                assert_eq!(expected, 3);
                assert_eq!(arrived, 2);
            }
            other => panic!("expected ShortColumnChunk, got {other:?}"),
        }
    }

    #[test]
    fn a_short_chunk_says_which_column_ran_out_where_and_by_how_much() {
        let e = LakeError::ShortColumnChunk {
            column: "open_interest",
            row: 400,
            expected: 2_480,
            arrived: 400,
        };
        let text = e.to_string();
        for needle in ["open_interest", "400", "2480"] {
            assert!(text.contains(needle), "must name {needle}, got: {text}");
        }
        // And it must not be mistakable for the other refusal.
        assert!(!text.contains("is null at row"), "got: {text}");
    }

    #[test]
    fn require_names_the_column_and_row_of_an_unexpected_null() {
        let ok = require(vec![Some(1_i64), Some(2)], "volume").unwrap();
        assert_eq!(ok, vec![1, 2]);

        match require(vec![Some(1_i64), None], "volume") {
            Err(LakeError::UnexpectedNull { column, row }) => {
                assert_eq!(column, "volume");
                assert_eq!(row, 1);
            }
            other => panic!("expected UnexpectedNull, got {other:?}"),
        }
    }

    #[test]
    fn unwrap_present_refuses_rather_than_panicking() {
        assert!(unwrap_present(None, "iv", 4).is_err());
        assert_eq!(
            unwrap_present(Some(1.5), "iv", 0).unwrap().to_bits(),
            1.5_f64.to_bits()
        );
    }

    // -----------------------------------------------------------------------
    // A FOOTER THAT LIES ABOUT `num_rows` USED TO KILL THE PROCESS.
    //
    // `read_row_group` hands the row group's declared `num_rows` to seven
    // `Vec::with_capacity` sites, and before the bound above the only guard was
    // a sign check. `Vec::with_capacity` cannot refuse: the allocator's "no"
    // becomes `std::alloc::handle_alloc_error`, which aborts rather than
    // panics, so no harness catches it. One edited footer field therefore
    // replaced a named refusal with a process that stops.
    //
    // The fixture is built and corrupted IN MEMORY. CI gate 1 walks every
    // tracked file and allows no `.parquet` in this repository, so neither the
    // sound file nor the lying one can be committed. This is the same
    // write-then-patch-the-thrift-footer pattern `tests/refusals.rs` uses; it
    // is repeated here rather than shared because an integration test cannot
    // reach a private helper and this refusal is one line of `reader.rs`.
    // -----------------------------------------------------------------------

    /// The seven cash columns, in the order `crate::schema::detect` insists on.
    const CASH: [(&str, PhysicalType); 7] = [
        ("timestamp", PhysicalType::INT64),
        ("open", PhysicalType::DOUBLE),
        ("high", PhysicalType::DOUBLE),
        ("low", PhysicalType::DOUBLE),
        ("close", PhysicalType::DOUBLE),
        ("volume", PhysicalType::INT64),
        ("open_interest", PhysicalType::INT64),
    ];

    /// Writes an uncompressed seven-column cash file of `rows` rows, every
    /// value present.
    ///
    /// Uncompressed because the codec is not what is under test here. The ZSTD
    /// path is driven on every `cargo test` run, CI's included, by
    /// `a_zstd_copy_decodes_to_exactly_what_the_uncompressed_original_does`
    /// and `a_zstd_copy_carries_paisa_nulls_and_zeros_to_the_right_rows` in
    /// `tests/synthetic.rs`, which recompress a file like this one with
    /// `ruzstd`'s encoder, and by
    /// `a_ruzstd_encoded_compressed_block_decodes_through_the_page_path` in
    /// `page.rs`. Frames as Polars wrote them are reached only by the
    /// `#[ignore]`d `tests/real_lake.rs`, on a machine that has the lake.
    fn cash_file(rows: usize) -> Vec<u8> {
        let fields: Vec<Arc<Type>> = CASH
            .iter()
            .map(|(name, ty)| {
                Arc::new(
                    Type::primitive_type_builder(name, *ty)
                        .with_repetition(Repetition::OPTIONAL)
                        .build()
                        .expect("a primitive field builds"),
                )
            })
            .collect();
        let schema = Arc::new(
            Type::group_type_builder("schema")
                .with_fields(fields)
                .build()
                .expect("the group type builds"),
        );
        let props = Arc::new(
            WriterProperties::builder()
                .set_compression(Compression::UNCOMPRESSED)
                .build(),
        );

        // Every level is 1 — PRESENT — so this file exercises no null path and
        // cannot fail for a reason other than the one under test.
        let defs = vec![1_i16; rows];
        let ints: Vec<i64> =
            (0..i64::try_from(rows).expect("a test row count fits an i64")).collect();
        let doubles: Vec<f64> = vec![100.5_f64; rows];

        let mut out: Vec<u8> = Vec::new();
        {
            let mut writer = SerializedFileWriter::new(&mut out, schema, props).expect("writer");
            let mut group = writer.next_row_group().expect("row group");
            while let Some(mut column) = group.next_column().expect("column") {
                match column.untyped() {
                    ColumnWriter::Int64ColumnWriter(typed) => {
                        typed.write_batch(&ints, Some(&defs), None).expect("i64");
                    }
                    ColumnWriter::DoubleColumnWriter(typed) => {
                        typed.write_batch(&doubles, Some(&defs), None).expect("f64");
                    }
                    _ => panic!("the cash layout is INT64 and DOUBLE only"),
                }
                column.close().expect("close column");
            }
            group.close().expect("close row group");
            writer.close().expect("close writer");
        }
        out
    }

    /// Re-serialises a file's footer after `edit` has changed it.
    ///
    /// The footer length is the four bytes before the trailing `PAR1`, and it
    /// is rewritten because an edited field almost never re-encodes to the same
    /// number of varint bytes.
    fn patch_footer(bytes: &[u8], edit: impl FnOnce(&mut FileMetaData)) -> Vec<u8> {
        let n = bytes.len();
        let declared: [u8; 4] = bytes[n - 8..n - 4]
            .try_into()
            .expect("four bytes of footer length");
        let flen =
            usize::try_from(u32::from_le_bytes(declared)).expect("a footer length fits a usize");
        let start = n - 8 - flen;
        let mut meta = {
            let mut cursor = Cursor::new(&bytes[start..n - 8]);
            let mut proto = TCompactInputProtocol::new(&mut cursor, flen * 64 + 1_000_000);
            FileMetaData::read_from_in_protocol(&mut proto)
                .expect("the footer this test just wrote parses")
        };
        edit(&mut meta);

        let mut out = bytes[..start].to_vec();
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut proto = TCompactOutputProtocol::new(&mut buf);
            meta.write_to_out_protocol(&mut proto)
                .expect("the edited footer re-serialises");
        }
        let rewritten = u32::try_from(buf.len()).expect("a footer is far under 4 GiB");
        out.extend_from_slice(&buf);
        out.extend_from_slice(&rewritten.to_le_bytes());
        out.extend_from_slice(b"PAR1");
        out
    }

    /// **A DECLARED ROW COUNT WIDER THAN THE WHOLE FILE IS REFUSED BY NAME.**
    ///
    /// `1 << 40` is chosen because it is the shape of the fault rather than an
    /// arbitrary large number: it is positive, so the sign check that used to
    /// be the only guard passes it, it fits a `usize` on every 64-bit target,
    /// so `usize::try_from` passes it too, and `Vec::<i64>::with_capacity` of
    /// it asks for 8 TiB.
    ///
    /// **What the unbounded reader does with this input was not run**, and the
    /// honest statement is that it is not a failed assertion either way: the
    /// allocator either refuses and `handle_alloc_error` aborts the test
    /// binary, or the kernel hands back address space it cannot back and the
    /// process is at the mercy of the first page it touches. Neither is a
    /// refusal, which is the whole point of bounding the count instead.
    ///
    /// The sound half of the assertion is not decoration. If the fixture stopped
    /// decoding — a writer change, a schema drift — the refusal below would
    /// still fire and would prove nothing, because a file that cannot be read
    /// at all is refused for other reasons first.
    #[test]
    fn a_row_count_wider_than_the_file_is_refused_rather_than_reserved() {
        const ROWS: usize = 8;
        /// Positive, `usize`-representable, and 8 TiB of `i64` wide.
        const ABSURD: i64 = 1 << 40;

        let sound = cash_file(ROWS);
        let opened = LakeFile::from_bytes(sound.clone()).expect("a seven-column cash file opens");
        assert_eq!(opened.layout(), Layout::Cash);
        assert_eq!(
            opened
                .read_row_group(0)
                .expect("the sound group decodes")
                .len(),
            ROWS,
            "the fixture must decode, or the refusal below proves nothing"
        );

        // ONLY `num_rows` CHANGES. Every page byte is the one the writer
        // produced, so this is a footer that lies rather than a file that is
        // short — the two are different faults and only the first is under test.
        let lying = patch_footer(&sound, |meta| {
            meta.row_groups[0].num_rows = ABSURD;
        });
        assert!(
            ABSURD > i64::try_from(lying.len()).expect("a test fixture fits an i64"),
            "the fixture must be smaller than the lie, or the bound is not what refuses"
        );

        let file = LakeFile::from_bytes(lying)
            .expect("the footer still parses; it is the VALUE that is impossible");
        match file.read_row_group(0) {
            Err(LakeError::ImpossibleLength { what, value }) => {
                assert_eq!(
                    value, ABSURD,
                    "the refusal must report the number the file gave"
                );
                assert!(
                    what.contains("row count"),
                    "the refusal must name the field, got {what:?}"
                );
            }
            other => panic!("expected ImpossibleLength, got {:?}", other.err()),
        }
    }

    #[test]
    fn the_magic_and_minimum_are_what_the_format_says() {
        assert_eq!(MAGIC, *b"PAR1");
        assert_eq!(MIN_FILE, 12);
    }
}
