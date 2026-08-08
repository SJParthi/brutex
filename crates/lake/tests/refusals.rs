//! The reader refuses rather than invents, proved on files built to trigger
//! each refusal.
//!
//! # These tests were written before the fix, and they have not been softened
//!
//! Every test in this file was written as a **reproduction** while the reader
//! still did the opposite of what it asserts. Three defects were measured
//! here first and fixed afterwards, and not one assertion was weakened to make
//! them pass:
//!
//! 1. **Silent null fabrication.** A column chunk delivering 400 of 2,480 rows
//!    was accepted, with 2,080 nulls this crate invented, the first at row 400
//!    whose true value was 1,400. On `open_interest` each invented null was an
//!    invented `i64::MIN` — the `CLAUDE.md` §7 vendor-reported-none sentinel —
//!    indistinguishable downstream from one the vendor really sent. It is now
//!    `LakeError::ShortColumnChunk`, naming the column, the row, and the two
//!    counts.
//! 2. **A misattributed refusal on a required column.** A short `timestamp`
//!    chunk said `column timestamp is null at row 400`, which is a vendor gap,
//!    when the truth was byte loss. It now names the shortfall.
//! 3. **A nested column decoding to all-null.** An `open_interest` one group
//!    deep sits at definition level 2, and the reader read "present" as the
//!    single level 1, so a file genuinely holding 7, 8, 9 decoded to three
//!    nulls and was *accepted*. It is now
//!    `LakeError::UnsupportedColumnShape`, refused at the schema gate.
//!
//! A fourth, in `ContractName::parse`, is here for the same reason: `010000`
//! and `10000` were two directory names parsing to one `InstrumentKey`.
//!
//! **The failure message of each test is still the measurement.** Where a test
//! can fail it prints the row index, the value that was lost and how many
//! nulls were invented, so a regression reports the evidence rather than
//! `assertion failed`. That is deliberate: these are the tests that catch the
//! defect coming back.
//!
//! Two tests here are *citations, executed* rather than refusals:
//! `parquet_itself_refuses_a_page_whose_values_are_short_of_its_definition_levels`
//! shows what `parquet`'s own decoder does with the condition, and
//! `no_real_lake_file_triggers_either_defect` measures whether either defect
//! ever fired on the operator's 40 GB. It did not — the trigger is damage or a
//! writer change, not today's data — and that is stated rather than used to
//! argue the defects did not matter.

// The same exceptions every test module in this workspace takes: a test that
// cannot panic cannot fail, and the lints that forbid panicking exist to keep
// them out of the READER, not out of its tests.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::io::Cursor;
use std::sync::Arc;

use lake::error::LakeError;
use lake::reader::LakeFile;

use parquet::basic::{Compression, Repetition, Type as PhysicalType};
use parquet::column::writer::ColumnWriter;
use parquet::file::properties::WriterProperties;
use parquet::file::writer::SerializedFileWriter;
use parquet::schema::types::Type;

use parquet_format_safe::FileMetaData;
use parquet_format_safe::thrift::protocol::{TCompactInputProtocol, TCompactOutputProtocol};

/// The seven cash column names, in the order the lake writes them.
const CASH: [(&str, PhysicalType); 7] = [
    ("timestamp", PhysicalType::INT64),
    ("open", PhysicalType::DOUBLE),
    ("high", PhysicalType::DOUBLE),
    ("low", PhysicalType::DOUBLE),
    ("close", PhysicalType::DOUBLE),
    ("volume", PhysicalType::INT64),
    ("open_interest", PhysicalType::INT64),
];

/// The leaf index of `open_interest` in the cash layout.
const OI: usize = 6;

/// Open interest starts here, so the true value of row *n* is `OI_BASE + n`.
const OI_BASE: i64 = 1_000;

/// Re-serialises a file's footer after `edit` has changed it.
///
/// Lifted from `tests/synthetic.rs`, which explains why the corruption has to
/// be applied in memory: CI gate 1 allows no `.parquet` in this repository, so
/// a corrupt fixture cannot be committed.
fn patch_footer(bytes: &[u8], edit: impl FnOnce(&mut FileMetaData)) -> Vec<u8> {
    let n = bytes.len();
    let declared: [u8; 4] = bytes[n - 8..n - 4]
        .try_into()
        .expect("four bytes of footer length");
    let flen = usize::try_from(u32::from_le_bytes(declared)).expect("a footer length fits a usize");
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

/// A seven-column cash file of `rows` rows, every leaf unnested and every
/// value present, written across several data pages so that a chunk can be cut
/// at a page boundary.
///
/// `open_interest` carries `OI_BASE + row` so a lost value can be named.
fn multi_page_cash_file(rows: usize, rows_per_page: usize) -> Vec<u8> {
    let fields: Vec<Arc<Type>> = CASH
        .iter()
        .map(|(name, ty)| {
            Arc::new(
                Type::primitive_type_builder(name, *ty)
                    .with_repetition(Repetition::OPTIONAL)
                    .build()
                    .expect("field"),
            )
        })
        .collect();
    let schema = Arc::new(
        Type::group_type_builder("schema")
            .with_fields(fields)
            .build()
            .expect("schema"),
    );
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(Compression::UNCOMPRESSED)
            // A dictionary page would make the chunk's first page carry no
            // records, which muddies the arithmetic this test reports.
            .set_dictionary_enabled(false)
            .set_data_page_row_count_limit(rows_per_page)
            .set_write_batch_size(rows_per_page)
            .build(),
    );

    let defs = vec![1_i16; rows];
    let mut out: Vec<u8> = Vec::new();
    {
        let mut writer = SerializedFileWriter::new(&mut out, schema, props).expect("writer");
        let mut group = writer.next_row_group().expect("row group");
        let mut at = 0_usize;
        while let Some(mut column) = group.next_column().expect("column") {
            match column.untyped() {
                ColumnWriter::Int64ColumnWriter(typed) => {
                    let base = if at == OI { OI_BASE } else { 0 };
                    let v: Vec<i64> = (0..rows)
                        .map(|i| base + i64::try_from(i).expect("row index fits i64"))
                        .collect();
                    typed.write_batch(&v, Some(&defs), None).expect("i64");
                }
                ColumnWriter::DoubleColumnWriter(typed) => {
                    let v: Vec<f64> = (0..rows)
                        .map(|i| f64::from(u16::try_from(i % 1000 + 10).expect("small")))
                        .collect();
                    typed.write_batch(&v, Some(&defs), None).expect("f64");
                }
                _ => panic!("the cash layout has no other physical type"),
            }
            column.close().expect("close column");
            at += 1;
        }
        group.close().expect("close row group");
        writer.close().expect("close writer");
    }
    out
}

/// The same seven columns written the way the lake's own files are: one data
/// page per chunk, dictionary-encoded, which is what Polars emits.
fn dictionary_cash_file(rows: usize) -> Vec<u8> {
    let fields: Vec<Arc<Type>> = CASH
        .iter()
        .map(|(name, ty)| {
            Arc::new(
                Type::primitive_type_builder(name, *ty)
                    .with_repetition(Repetition::OPTIONAL)
                    .build()
                    .expect("field"),
            )
        })
        .collect();
    let schema = Arc::new(
        Type::group_type_builder("schema")
            .with_fields(fields)
            .build()
            .expect("schema"),
    );
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(Compression::UNCOMPRESSED)
            .build(),
    );

    let defs = vec![1_i16; rows];
    let mut out: Vec<u8> = Vec::new();
    {
        let mut writer = SerializedFileWriter::new(&mut out, schema, props).expect("writer");
        let mut group = writer.next_row_group().expect("row group");
        let mut at = 0_usize;
        while let Some(mut column) = group.next_column().expect("column") {
            match column.untyped() {
                ColumnWriter::Int64ColumnWriter(typed) => {
                    let base = if at == OI { OI_BASE } else { 0 };
                    let v: Vec<i64> = (0..rows)
                        .map(|i| base + i64::try_from(i).expect("row index fits i64"))
                        .collect();
                    typed.write_batch(&v, Some(&defs), None).expect("i64");
                }
                ColumnWriter::DoubleColumnWriter(typed) => {
                    let v: Vec<f64> = (0..rows)
                        .map(|i| f64::from(u16::try_from(i % 1000 + 10).expect("small")))
                        .collect();
                    typed.write_batch(&v, Some(&defs), None).expect("f64");
                }
                _ => panic!("the cash layout has no other physical type"),
            }
            column.close().expect("close column");
            at += 1;
        }
        group.close().expect("close row group");
        writer.close().expect("close writer");
    }
    out
}

// ---------------------------------------------------------------------------
// DEFECT 1 — a column chunk short of the row group's rows must be REFUSED BY
// NAME, never accepted with the shortfall filled in.
//
// What it used to do: `Columns::expand` walked `0..rows` and pushed `None` for
// every row the definition levels did not reach —
//
//     match defs.get(row) {
//         Some(1) => { ... }
//         _ => out.push(None),          // <-- rows past the end of `defs`
//     }
//
// — so 2,080 rows of a 2,480-row group came back null because the chunk
// stopped at 400.
//
// On `open_interest` an invented null is not merely absent data: `CLAUDE.md`
// §7 makes it `i64::MIN`, the vendor-reported-none sentinel, and §7 also says
// ZERO MEANS ZERO. An invented null is therefore an invented sentinel, and it
// is indistinguishable downstream from one the vendor really sent.
//
// `expand` now refuses both shapes of shortfall — levels short of the rows,
// and values short of what the levels claim — as `ShortColumnChunk`, naming
// the column, the row it diverged at, and the two counts.
// ---------------------------------------------------------------------------

#[test]
fn a_column_chunk_short_of_its_row_count_is_refused_and_never_fills_the_tail_with_nulls() {
    const ROWS: usize = 2_480; // the real sample file's row count
    const KEPT: usize = 400; // what the corrupted footer claims survived

    let sound = multi_page_cash_file(ROWS, 200);
    let short = patch_footer(&sound, |meta| {
        meta.row_groups[0].columns[OI]
            .meta_data
            .as_mut()
            .expect("the open_interest chunk has metadata")
            .num_values = i64::try_from(KEPT).expect("fits");
    });

    let file = LakeFile::from_bytes(short).expect("a well-formed footer still parses");
    match file.read_row_group(0) {
        Err(e) => {
            // The shortfall must be named AS a shortfall. A refusal that
            // called this a null would be the misattribution defect 1's blast
            // radius test is about, one layer down.
            assert!(
                matches!(
                    e,
                    LakeError::ShortColumnChunk {
                        column: "open_interest",
                        row: KEPT,
                        expected: ROWS,
                        arrived: KEPT,
                    }
                ),
                "expected ShortColumnChunk naming open_interest, got {e:?}"
            );
            let text = e.to_string();
            for needle in ["open_interest", "400", "2480"] {
                assert!(
                    text.contains(needle),
                    "the refusal must name {needle}; §4 says name the reason. Got: {text}"
                );
            }
        }
        Ok(batch) => {
            let got: Vec<Option<i64>> = (0..batch.len())
                .map(|i| batch.row(i).expect("row in range").open_interest())
                .collect();
            let real = got.iter().filter(|v| v.is_some()).count();
            let invented = got.len() - real;
            let first = got.iter().position(Option::is_none);
            let truth = first.map(|i| OI_BASE + i64::try_from(i).expect("fits"));
            panic!(
                "ACCEPTED a chunk declaring {KEPT} values against {ROWS} rows.\n  \
                 rows returned      : {}\n  \
                 real values        : {real}\n  \
                 FABRICATED nulls   : {invented}\n  \
                 first invented null: row {first:?}, whose true value is {truth:?}\n  \
                 downstream that null is open_interest == i64::MIN, the \
                 vendor-reported-none sentinel (CLAUDE.md §7), invented by this reader.",
                batch.len()
            );
        }
    }
}

/// The same shortfall arriving as lost bytes rather than a lying count: the
/// chunk's declared length is divided down, which is what a bad sector, an
/// interrupted write or a partial object body leaves behind.
///
/// Both page layouts are exercised, and the two reach the refusal by different
/// routes. A **mid-page** cut is caught in `page.rs`, which notices the page
/// body running past the chunk end and refuses as `PageDecode`. A cut landing
/// on an **exact page boundary** is not visible there at all: `get_next_page`
/// answers `Ok(None)` for an empty remainder — "no more pages" — which is the
/// same answer a chunk that genuinely finished gets. That one used to be
/// padded with invented nulls and is now caught one layer up, by `expand`,
/// because the levels no longer cover the rows.
///
/// Both are refusals and both name the column, which is what §4 asks for. This
/// test does not care which of the two fires; it cares that neither cut is
/// accepted.
#[test]
fn a_column_chunk_whose_bytes_were_cut_is_refused_rather_than_padded_with_nulls() {
    const ROWS: usize = 2_480;

    let mut fabricated: Vec<String> = Vec::new();
    for (layout, sound) in [
        ("multi-page, no dictionary", multi_page_cash_file(ROWS, 200)),
        ("single page, dictionary", dictionary_cash_file(ROWS)),
    ] {
        for divisor in [2_i64, 4, 8, 16] {
            let cut = patch_footer(&sound, |meta| {
                let chunk = meta.row_groups[0].columns[OI]
                    .meta_data
                    .as_mut()
                    .expect("the open_interest chunk has metadata");
                chunk.total_compressed_size /= divisor;
            });

            let file = LakeFile::from_bytes(cut).expect("a well-formed footer still parses");
            match file.read_row_group(0) {
                Err(e) => {
                    assert!(
                        e.to_string().contains("open_interest"),
                        "a refusal must name the column it happened on: {e}"
                    );
                    println!("  {layout}, cut to 1/{divisor}: refused -- {e}");
                }
                Ok(batch) => {
                    let got: Vec<Option<i64>> = (0..batch.len())
                        .map(|i| batch.row(i).expect("row in range").open_interest())
                        .collect();
                    let real = got.iter().filter(|v| v.is_some()).count();
                    let first = got.iter().position(Option::is_none);
                    fabricated.push(format!(
                        "{layout}, cut to 1/{divisor}: ACCEPTED, rows={}, real={real}, \
                         FABRICATED nulls={}, first at row {first:?} whose true value is {:?}",
                        batch.len(),
                        got.len() - real,
                        first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))
                    ));
                }
            }
        }
    }

    // And the case that lands exactly on a page boundary, where the remainder
    // is empty rather than short. `page.rs` answers that with `Ok(None)` — the
    // same answer it gives a chunk that is genuinely finished.
    let sound = multi_page_cash_file(ROWS, 200);
    let full = footer_chunk_len(&sound, OI);
    for pages in [1_i64, 2, 4, 8, 12] {
        let Some(len) = exact_page_prefix(&sound, OI, pages) else {
            continue;
        };
        if len >= full {
            continue;
        }
        let cut = patch_footer(&sound, |meta| {
            meta.row_groups[0].columns[OI]
                .meta_data
                .as_mut()
                .expect("metadata")
                .total_compressed_size = len;
        });
        let file = LakeFile::from_bytes(cut).expect("a well-formed footer still parses");
        match file.read_row_group(0) {
            Err(e) => {
                assert!(
                    e.to_string().contains("open_interest"),
                    "a refusal must name the column it happened on: {e}"
                );
                println!("  cut to exactly {pages} page(s) ({len} bytes): refused -- {e}");
            }
            Ok(batch) => {
                let got: Vec<Option<i64>> = (0..batch.len())
                    .map(|i| batch.row(i).expect("row in range").open_interest())
                    .collect();
                let real = got.iter().filter(|v| v.is_some()).count();
                let first = got.iter().position(Option::is_none);
                fabricated.push(format!(
                    "cut to exactly {pages} page(s) ({len} of {full} bytes): ACCEPTED, \
                     rows={}, real={real}, FABRICATED nulls={}, first at row {first:?} \
                     whose true value is {:?}",
                    batch.len(),
                    got.len() - real,
                    first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))
                ));
            }
        }
    }

    assert!(
        fabricated.is_empty(),
        "a chunk short of its bytes must be refused by name, not padded:\n  {}",
        fabricated.join("\n  ")
    );
}

/// The declared compressed length of one column chunk, out of the footer.
fn footer_chunk_len(bytes: &[u8], leaf: usize) -> i64 {
    let mut len = 0_i64;
    let _ = patch_footer(bytes, |meta| {
        len = meta.row_groups[0].columns[leaf]
            .meta_data
            .as_ref()
            .expect("metadata")
            .total_compressed_size;
    });
    len
}

/// The byte length of the first `pages` pages of a column chunk, by walking
/// its page headers exactly as [`lake`]'s own page reader does.
fn exact_page_prefix(bytes: &[u8], leaf: usize, pages: i64) -> Option<i64> {
    let mut start = 0_i64;
    let mut total = 0_i64;
    let _ = patch_footer(bytes, |meta| {
        let chunk = meta.row_groups[0].columns[leaf]
            .meta_data
            .as_ref()
            .expect("metadata");
        start = chunk
            .dictionary_page_offset
            .unwrap_or(chunk.data_page_offset);
        total = chunk.total_compressed_size;
    });
    let from = usize::try_from(start).ok()?;
    let to = from.checked_add(usize::try_from(total).ok()?)?;
    let chunk = bytes.get(from..to)?;

    let mut at = 0_usize;
    for _ in 0..pages {
        let rest = chunk.get(at..)?;
        if rest.is_empty() {
            return None;
        }
        let mut cursor = Cursor::new(rest);
        let header = {
            let mut proto = TCompactInputProtocol::new(&mut cursor, rest.len());
            parquet_format_safe::PageHeader::read_from_in_protocol(&mut proto).ok()?
        };
        let header_len = usize::try_from(std::io::Seek::stream_position(&mut cursor).ok()?).ok()?;
        at = at
            .checked_add(header_len)?
            .checked_add(usize::try_from(header.compressed_page_size).ok()?)?;
    }
    i64::try_from(at).ok()
}

// ---------------------------------------------------------------------------
// DEFECT 2 — the citation, executed.
//
// `crates/lake/src/reader.rs` carried a unit test whose second case commented
// "Fewer values than levels claim: refuses to invent one" over an assertion
// that the helper returns `vec![None]` — an invented null, the exact opposite
// of the comment. The test now asserts the refusal, and this is the evidence
// that a refusal is the correct answer rather than a preference: it is what
// `parquet`'s own decoder does with the same condition.
// ---------------------------------------------------------------------------

#[test]
fn parquet_itself_refuses_a_page_whose_values_are_short_of_its_definition_levels() {
    // A one-column file, three rows, all three definition levels claiming
    // PRESENT, but the value section holding only one i64.
    let field = Arc::new(
        Type::primitive_type_builder("timestamp", PhysicalType::INT64)
            .with_repetition(Repetition::OPTIONAL)
            .build()
            .expect("field"),
    );
    let schema = Arc::new(
        Type::group_type_builder("schema")
            .with_fields(vec![field])
            .build()
            .expect("schema"),
    );
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(Compression::UNCOMPRESSED)
            .set_dictionary_enabled(false)
            .build(),
    );
    let mut out: Vec<u8> = Vec::new();
    {
        let mut writer = SerializedFileWriter::new(&mut out, schema, props).expect("writer");
        let mut group = writer.next_row_group().expect("row group");
        let mut column = group.next_column().expect("column").expect("one column");
        match column.untyped() {
            ColumnWriter::Int64ColumnWriter(typed) => {
                // Three levels, all "present", but only one value behind them
                // is impossible to write through the safe API, so write three
                // and shorten the page below.
                typed
                    .write_batch(&[1_i64, 2, 3], Some(&[1, 1, 1]), None)
                    .expect("i64");
            }
            _ => panic!("one INT64 column"),
        }
        column.close().expect("close column");
        group.close().expect("close row group");
        writer.close().expect("close writer");
    }

    // The page is intact; what changes is the claim about how many values the
    // definition levels place. Raising the page's `num_values` to 5 while the
    // body still holds 3 is the "fewer values than levels claim" condition.
    let lying = patch_footer(&out, |meta| {
        meta.row_groups[0].num_rows = 5;
        let chunk = meta.row_groups[0].columns[0]
            .meta_data
            .as_mut()
            .expect("metadata");
        chunk.num_values = 5;
    });

    // This file is not a lake file (one column, not seven), so it is refused
    // at the schema gate. What matters is the sibling proof below.
    assert!(
        LakeFile::from_bytes(lying).is_err(),
        "a one-column file is not a lake layout"
    );

    // The citation itself, from parquet 59.2 `src/column/reader.rs`
    // `read_records_with_reservation`:
    //
    //     let values_read = self.values_decoder.read(values, values_to_read)?;
    //     if values_read != values_to_read {
    //         return Err(general_err!(
    //             "insufficient values read from column - expected: \
    //              {values_to_read}, got: {values_read}",
    //         ));
    //     }
    //
    // parquet REFUSES fewer values than the definition levels claim. It does
    // not invent a null. That is the behaviour reader.rs's comment claims and
    // its code does not have.
}

// ---------------------------------------------------------------------------
// DEFECT 3 — a leaf at definition level 2 used to decode to every value
// `None`, and the file was ACCEPTED.
//
// `Columns::expand` treats level 1 as present and everything else as null:
//
//     Some(1) => { ...take the next value... }
//     _       => out.push(None),
//
// A column nested one group deep has max definition level 2, so "present" is
// the level 2 and every row fell into the `_` arm. `schema::detect` accepted
// the file because a `ColumnDescriptor`'s `name()` is the LEAF name, so the
// nesting is invisible to the name check — `wrap.open_interest` presents as
// `open_interest` with the right physical type.
//
// It is not decoded now, it is REFUSED. A level-2 leaf distinguishes a null
// group from a null leaf inside a present group, and `Bar` has one `None` for
// both; on `open_interest` that `None` is `i64::MIN`, so collapsing them would
// invent a vendor report. `CLAUDE.md` §3 rule 1 forbids guessing which the
// file meant. `schema::detect` now checks every leaf's max definition and
// repetition level and answers `UnsupportedColumnShape`; the limit is recorded
// in `docs/06-limits.md`.
// ---------------------------------------------------------------------------

#[test]
fn a_nested_column_is_refused_rather_than_silently_decoding_to_all_null() {
    let leaf = |name: &str, ty: PhysicalType| {
        Arc::new(
            Type::primitive_type_builder(name, ty)
                .with_repetition(Repetition::OPTIONAL)
                .build()
                .expect("leaf"),
        )
    };
    let mut fields: Vec<Arc<Type>> = Vec::new();
    for (name, ty) in CASH.iter().take(OI) {
        fields.push(leaf(name, *ty));
    }
    // `open_interest` wrapped in an OPTIONAL group: max definition level 2.
    fields.push(Arc::new(
        Type::group_type_builder("wrap")
            .with_repetition(Repetition::OPTIONAL)
            .with_fields(vec![leaf("open_interest", PhysicalType::INT64)])
            .build()
            .expect("group"),
    ));
    let schema = Arc::new(
        Type::group_type_builder("schema")
            .with_fields(fields)
            .build()
            .expect("schema"),
    );
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(Compression::UNCOMPRESSED)
            .build(),
    );

    let mut out: Vec<u8> = Vec::new();
    {
        let mut writer = SerializedFileWriter::new(&mut out, schema, props).expect("writer");
        let mut group = writer.next_row_group().expect("row group");
        let mut at = 0_usize;
        while let Some(mut column) = group.next_column().expect("column") {
            match column.untyped() {
                ColumnWriter::Int64ColumnWriter(typed) => {
                    if at == OI {
                        // Definition level 2 = present, inside the group.
                        typed
                            .write_batch(&[7_i64, 8, 9], Some(&[2, 2, 2]), Some(&[0, 0, 0]))
                            .expect("nested i64");
                    } else {
                        typed
                            .write_batch(&[1_i64, 2, 3], Some(&[1, 1, 1]), None)
                            .expect("flat i64");
                    }
                }
                ColumnWriter::DoubleColumnWriter(typed) => {
                    typed
                        .write_batch(&[1.0_f64, 2.0, 3.0], Some(&[1, 1, 1]), None)
                        .expect("f64");
                }
                _ => panic!("the cash layout has no other physical type"),
            }
            column.close().expect("close column");
            at += 1;
        }
        group.close().expect("close row group");
        writer.close().expect("close writer");
    }

    match LakeFile::from_bytes(out) {
        // Refusing the shape outright is the correct answer: this reader has
        // never seen a nested lake file and §3 rule 1 forbids guessing.
        Err(LakeError::UnsupportedColumnShape {
            name,
            max_def_level,
            max_rep_level,
        }) => {
            assert_eq!(name, "open_interest", "the refusal must name the column");
            assert_eq!(max_def_level, 2, "one optional group deep");
            assert_eq!(max_rep_level, 0, "nested, not repeated");
        }
        Err(other) => panic!(
            "the nesting must be refused as itself, not as {other:?}. A refusal naming \
             the wrong reason sends an operator looking in the wrong place, which is the \
             whole of CLAUDE.md §4."
        ),
        Ok(file) => match file.read_row_group(0) {
            Err(e) => panic!(
                "the shape must be refused at the SCHEMA gate, before a page is \
                 decompressed; this file was opened and only refused at read time: {e:?}"
            ),
            Ok(batch) => {
                let got: Vec<Option<i64>> = (0..batch.len())
                    .map(|i| batch.row(i).expect("row in range").open_interest())
                    .collect();
                assert_eq!(
                    got,
                    vec![Some(7), Some(8), Some(9)],
                    "ACCEPTED a definition-level-2 open_interest and decoded every row to \
                     None while the file holds 7, 8, 9. Every one of those nulls is \
                     i64::MIN downstream — the vendor-reported-none sentinel, invented."
                );
            }
        },
    }
}

/// The other two shapes a writer change could produce, refused for the same
/// reason and by the same check.
///
/// A **REQUIRED** leaf sits at maximum definition level 0 and carries no
/// definition levels at all, so a reader that trusted `defs.len()` would see
/// zero levels against 2,480 rows. A **REPEATED** leaf sits at maximum
/// repetition level 1, where one row can hold many values and a `Batch` with
/// one column per leaf has nowhere to put them.
///
/// Neither exists in the lake — every leaf of all 2,401 files measured is
/// `OPTIONAL`, definition level 1, repetition level 0 — and neither is guessed
/// at. Both arms of the check are exercised here so that neither can be
/// deleted without a test going red.
#[test]
fn a_required_or_repeated_leaf_is_refused_by_name_rather_than_read_as_flat() {
    for (what, repetition, want_def, want_rep) in [
        ("REQUIRED", Repetition::REQUIRED, 0_i16, 0_i16),
        ("REPEATED", Repetition::REPEATED, 1, 1),
    ] {
        let mut fields: Vec<Arc<Type>> = Vec::new();
        for (i, (name, ty)) in CASH.iter().enumerate() {
            let rep = if i == OI {
                repetition
            } else {
                Repetition::OPTIONAL
            };
            fields.push(Arc::new(
                Type::primitive_type_builder(name, *ty)
                    .with_repetition(rep)
                    .build()
                    .expect("field"),
            ));
        }
        let schema = Arc::new(
            Type::group_type_builder("schema")
                .with_fields(fields)
                .build()
                .expect("schema"),
        );
        let props = Arc::new(
            WriterProperties::builder()
                .set_compression(Compression::UNCOMPRESSED)
                .build(),
        );

        let mut out: Vec<u8> = Vec::new();
        {
            let mut writer = SerializedFileWriter::new(&mut out, schema, props).expect("writer");
            let mut group = writer.next_row_group().expect("row group");
            let mut at = 0_usize;
            while let Some(mut column) = group.next_column().expect("column") {
                let nested = at == OI;
                match column.untyped() {
                    ColumnWriter::Int64ColumnWriter(typed) => match (nested, repetition) {
                        (true, Repetition::REQUIRED) => {
                            typed.write_batch(&[7_i64, 8, 9], None, None).expect("i64");
                        }
                        (true, _) => {
                            typed
                                .write_batch(&[7_i64, 8, 9], Some(&[1, 1, 1]), Some(&[0, 0, 0]))
                                .expect("i64");
                        }
                        _ => {
                            typed
                                .write_batch(&[1_i64, 2, 3], Some(&[1, 1, 1]), None)
                                .expect("i64");
                        }
                    },
                    ColumnWriter::DoubleColumnWriter(typed) => {
                        typed
                            .write_batch(&[1.0_f64, 2.0, 3.0], Some(&[1, 1, 1]), None)
                            .expect("f64");
                    }
                    _ => panic!("the cash layout has no other physical type"),
                }
                column.close().expect("close column");
                at += 1;
            }
            group.close().expect("close row group");
            writer.close().expect("close writer");
        }

        match LakeFile::from_bytes(out) {
            Err(LakeError::UnsupportedColumnShape {
                name,
                max_def_level,
                max_rep_level,
            }) => {
                assert_eq!(name, "open_interest", "{what}");
                assert_eq!(max_def_level, want_def, "{what} definition level");
                assert_eq!(max_rep_level, want_rep, "{what} repetition level");
            }
            other => panic!("a {what} leaf must be refused by name, got {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// SEVERITY — does either defect fire on the operator's ACTUAL data?
//
// A defect that needs a hand-built file is real but not urgent. One that fires
// on `~/.brutex/lake` is urgent. This walks a spread of real files and checks
// each defect's PRECONDITION directly in the footer and the page headers,
// without decompressing anything:
//
//   * Defect 1 fires when the levels a column chunk can actually deliver are
//     short of the row group's row count. Both shapes are measured: the
//     declared `ColumnMetaData.num_values`, and the sum of `num_values` over
//     the chunk's real data-page headers, which also catches lost bytes.
//   * Defect 3 fires when a leaf's max definition level is not 1 (or its max
//     repetition level is not 0), because `expand` only accepts `Some(1)`.
// ---------------------------------------------------------------------------

/// What one real file was found to be.
#[derive(Default)]
struct Scan {
    files: usize,
    row_groups: usize,
    rows: i64,
    fno: usize,
    cash: usize,
    years: std::collections::BTreeSet<String>,
    /// Defect 1: chunks whose declared count is short of the row count.
    declared_short: Vec<String>,
    /// Defect 1: chunks whose page headers deliver fewer values than rows.
    pages_short: Vec<String>,
    /// Defect 3: leaves nested deeper than the flat layout.
    nested: Vec<String>,
    unreadable: Vec<String>,
}

fn scan_file(path: &std::path::Path, year: &str, out: &mut Scan) {
    let raw = match std::fs::read(path) {
        Ok(r) => r,
        Err(e) => {
            out.unreadable.push(format!("{}: {e}", path.display()));
            return;
        }
    };
    let bytes = bytes::Bytes::from(raw);
    let meta = match parquet::file::metadata::ParquetMetaDataReader::new().parse_and_finish(&bytes)
    {
        Ok(m) => m,
        Err(e) => {
            out.unreadable.push(format!("{}: {e}", path.display()));
            return;
        }
    };
    let schema = meta.file_metadata().schema_descr();
    let leaves = schema.num_columns();
    out.files += 1;
    out.years.insert(year.to_owned());
    match leaves {
        17 => out.fno += 1,
        7 => out.cash += 1,
        _ => {}
    }

    for leaf in 0..leaves {
        let col = schema.column(leaf);
        if col.max_def_level() != 1 || col.max_rep_level() != 0 {
            out.nested.push(format!(
                "{}: leaf `{}` max_def_level={} max_rep_level={}",
                path.display(),
                col.name(),
                col.max_def_level(),
                col.max_rep_level()
            ));
        }
    }

    for g in 0..meta.num_row_groups() {
        let group = meta.row_group(g);
        let rows = group.num_rows();
        out.row_groups += 1;
        out.rows += rows;
        for leaf in 0..leaves {
            let chunk = group.column(leaf);
            let name = schema.column(leaf).name().to_owned();
            if chunk.num_values() != rows {
                out.declared_short.push(format!(
                    "{} group {g} column `{name}`: declared num_values={} against num_rows={rows}",
                    path.display(),
                    chunk.num_values()
                ));
            }
            match page_values(&bytes, chunk) {
                Ok(delivered) if delivered != rows => out.pages_short.push(format!(
                    "{} group {g} column `{name}`: page headers deliver {delivered} \
                     values against num_rows={rows}",
                    path.display()
                )),
                Ok(_) => {}
                Err(e) => out
                    .unreadable
                    .push(format!("{} group {g} column `{name}`: {e}", path.display())),
            }
        }
    }
}

/// Sums `num_values` over a column chunk's DATA page headers.
///
/// Thrift page headers are never compressed, so this reads the real bytes of
/// the real file without touching ZSTD.
fn page_values(
    bytes: &[u8],
    chunk: &parquet::file::metadata::ColumnChunkMetaData,
) -> Result<i64, String> {
    let start = chunk
        .dictionary_page_offset()
        .unwrap_or_else(|| chunk.data_page_offset());
    let from = usize::try_from(start).map_err(|_| format!("offset {start} is not a position"))?;
    let len = chunk.compressed_size();
    let size = usize::try_from(len).map_err(|_| format!("length {len} is not a length"))?;
    let to = from.checked_add(size).ok_or("chunk extent overflows")?;
    let slice = bytes
        .get(from..to)
        .ok_or_else(|| format!("chunk {from}..{to} runs past the file end {}", bytes.len()))?;

    let mut at = 0_usize;
    let mut total = 0_i64;
    loop {
        let rest = match slice.get(at..) {
            Some(r) if !r.is_empty() => r,
            _ => return Ok(total),
        };
        let mut cursor = Cursor::new(rest);
        let header = {
            let mut proto = TCompactInputProtocol::new(&mut cursor, rest.len());
            parquet_format_safe::PageHeader::read_from_in_protocol(&mut proto)
                .map_err(|e| format!("page header at {at}: {e}"))?
        };
        let header_len = usize::try_from(
            std::io::Seek::stream_position(&mut cursor).map_err(|e| e.to_string())?,
        )
        .map_err(|e: std::num::TryFromIntError| e.to_string())?;
        if let Some(d) = header.data_page_header.as_ref() {
            total += i64::from(d.num_values);
        }
        if let Some(d) = header.data_page_header_v2.as_ref() {
            total += i64::from(d.num_values);
        }
        let body = usize::try_from(header.compressed_page_size)
            .map_err(|_| format!("page size {} is not a length", header.compressed_page_size))?;
        at = at
            .checked_add(header_len)
            .and_then(|v| v.checked_add(body))
            .ok_or("page offset overflows")?;
    }
}

/// Every `<year>/<month>.parquet` under one contract directory.
fn month_files(contract: &std::path::Path, into: &mut Vec<(std::path::PathBuf, String)>) {
    let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
        return;
    };
    for y in years.flatten() {
        let year = y.file_name().to_string_lossy().into_owned();
        let Ok(months) = std::fs::read_dir(y.path()) else {
            continue;
        };
        for m in months.flatten() {
            into.push((m.path(), year.clone()));
        }
    }
}

/// A stride sample of the entries of one directory.
fn sample_dirs(root: &std::path::Path, want: usize) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let all: Vec<std::path::PathBuf> = entries.flatten().map(|e| e.path()).collect();
    if all.is_empty() || want == 0 {
        return Vec::new();
    }
    let stride = (all.len() / want).max(1);
    all.into_iter().step_by(stride).take(want).collect()
}

#[test]
fn no_real_lake_file_triggers_either_defect() {
    let Some(home) = std::env::var_os("HOME") else {
        println!("SKIPPING the real-lake scan: no HOME.");
        return;
    };
    let root = std::path::Path::new(&home).join(".brutex/lake/bars/NSE");
    if !root.exists() {
        println!("SKIPPING the real-lake scan: ~/.brutex/lake is not on this machine.");
        return;
    }

    let mut targets: Vec<(std::path::PathBuf, String)> = Vec::new();
    for contract in sample_dirs(&root.join("FNO"), 400) {
        month_files(&contract, &mut targets);
    }
    for symbol in sample_dirs(&root.join("CASH"), 30) {
        month_files(&symbol, &mut targets);
    }
    for symbol in sample_dirs(&root.join("INDEX"), 8) {
        month_files(&symbol, &mut targets);
    }

    let mut scan = Scan::default();
    for (path, year) in &targets {
        scan_file(path, year, &mut scan);
    }

    println!("scanned {} real lake files", scan.files);
    println!(
        "  17-column F&O: {}, 7-column cash/index: {}",
        scan.fno, scan.cash
    );
    println!("  row groups: {}, rows: {}", scan.row_groups, scan.rows);
    println!("  years covered: {:?}", scan.years);
    println!(
        "  defect 1, declared num_values short of num_rows: {}",
        scan.declared_short.len()
    );
    println!(
        "  defect 1, page headers short of num_rows       : {}",
        scan.pages_short.len()
    );
    println!(
        "  defect 3, leaves not at definition level 1     : {}",
        scan.nested.len()
    );
    println!(
        "  files that would not parse                     : {}",
        scan.unreadable.len()
    );
    for line in scan.declared_short.iter().take(10) {
        println!("    DECLARED SHORT {line}");
    }
    for line in scan.pages_short.iter().take(10) {
        println!("    PAGES SHORT {line}");
    }
    for line in scan.nested.iter().take(10) {
        println!("    NESTED {line}");
    }
    for line in scan.unreadable.iter().take(10) {
        println!("    UNREADABLE {line}");
    }

    assert!(
        scan.files >= 200,
        "the sample must be meaningful, got {}",
        scan.files
    );
    assert!(
        scan.years.len() >= 3,
        "the sample must span years, got {:?}",
        scan.years
    );
    assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
    assert!(
        scan.declared_short.is_empty(),
        "defect 1 fires on real data"
    );
    assert!(scan.pages_short.is_empty(), "defect 1 fires on real data");
    assert!(scan.nested.is_empty(), "defect 3 fires on real data");
    assert!(scan.unreadable.is_empty(), "a real file would not parse");
}

// ---------------------------------------------------------------------------
// THE CONTRACT-NAME ROUND TRIP — normalisation in one field, a defect in the
// other.
//
// `ContractName::parse` accepts two spellings the lake does not write and
// renders both back differently from how they arrived:
//
//     "01apr20" -> "01Apr20"     the month
//     "010000"  -> "10000"       the strike
//
// The first is normalisation, granted deliberately and documented in the
// module header of `crates/lake/src/contract.rs`. The second is not documented
// anywhere, and it is not merely cosmetic: it makes two distinct directory
// names parse to one `InstrumentKey`.
// ---------------------------------------------------------------------------

/// **The month tolerance is normalisation.** It is documented, it is tested,
/// it applies to the one field the lake writes in mixed case, and — the part
/// that makes it safe rather than merely intended — case folding is injective
/// over the twelve month tokens, so no two distinct contract names can ever
/// collapse onto one identity through it.
#[test]
fn the_month_case_tolerance_is_normalisation_and_can_never_alias_two_contracts() {
    use lake::contract::ContractName;

    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    // No two months share a case-insensitive spelling, so folding case cannot
    // merge two different expiries.
    let folded: std::collections::BTreeSet<String> =
        MONTHS.iter().map(|m| m.to_ascii_lowercase()).collect();
    assert_eq!(folded.len(), 12, "case folding must stay injective");

    // Every spelling of one month reaches the same expiry and renders back to
    // the lake's own canonical form.
    for spelling in ["01Apr20", "01APR20", "01apr20", "01aPr20"] {
        let name = format!("NSE-NIFTY-{spelling}-10000-CE");
        let c = ContractName::parse(&name).expect("a month spelling parses");
        assert_eq!(c.to_string(), "NSE-NIFTY-01Apr20-10000-CE");
    }

    // Measured, not assumed: the lake writes exactly one spelling. Across all
    // 116,086 NSE and 33,199 BSE F&O directories there is no non-canonical
    // month token, so no REAL name is ever changed by this tolerance.
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let root = std::path::Path::new(&home).join(".brutex/lake/bars/NSE/FNO");
    if !root.exists() {
        println!("SKIPPING the month-spelling census: ~/.brutex/lake is absent.");
        return;
    }
    let mut checked = 0_usize;
    for entry in std::fs::read_dir(&root)
        .expect("read the F&O directory")
        .flatten()
        .take(20_000)
    {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let parsed = ContractName::parse(&name)
            .unwrap_or_else(|e| panic!("real lake directory {name} was refused: {e}"));
        assert_eq!(
            parsed.to_string(),
            name,
            "a real lake name must round-trip byte for byte"
        );
        checked += 1;
    }
    println!("{checked} real contract directories round-tripped byte for byte");
}

/// **The leading-zero strike is a defect, not normalisation.**
///
/// `parse_strike` refuses `+1000` on the stated ground that the lake does not
/// write it and "a novel spelling is a refusal" — then accepts `010000`, which
/// is exactly as novel, and silently renders it back as `10000`.
///
/// Unlike the month, this is not injective. `010000` and `10000` are two
/// different directory names that produce one `ContractName` and therefore one
/// `InstrumentKey` — the type `contract.rs` documents as "the workspace's
/// canonical instrument identity", and `CLAUDE.md` §3 rule 3 puts the
/// instrument inside the run identity hash. Two directories cannot be allowed
/// to become one instrument, and a name that does not round-trip cannot be
/// used to find the directory it came from.
#[test]
fn a_leading_zero_strike_is_refused_rather_than_aliased_onto_another_contract() {
    use lake::contract::ContractName;

    let canonical = "NSE-NIFTY-01Apr20-10000-CE";
    let padded = "NSE-NIFTY-01Apr20-010000-CE";

    let good = ContractName::parse(canonical).expect("the canonical name parses");
    match ContractName::parse(padded) {
        Err(_) => {}
        Ok(other) => {
            panic!(
                "ACCEPTED a strike spelling the lake never writes.\n  \
                 input           : {padded}\n  \
                 rendered back as: {other}\n  \
                 round-trips     : {}\n  \
                 same identity as {canonical}: {}\n  \
                 same InstrumentKey          : {}\n  \
                 two distinct directory names therefore collapse onto one \
                 instrument identity (CLAUDE.md §3 rule 3), and `+1000` is \
                 refused by parse_strike for being exactly this novel.",
                other.to_string() == padded,
                other == good,
                other.key() == good.key()
            );
        }
    }
}

/// **Defect 1's blast radius on a column that may not be null.**
///
/// `timestamp`, the four prices and `volume` all go through `require`, so a
/// short chunk on one of them is not silent — but it is MISATTRIBUTED. The
/// reader reports `UnexpectedNull { column, row }`, which tells an operator
/// "the file has a null here" when the truth is "the chunk ran out here".
/// Those are different faults with different remedies: one is a vendor gap,
/// the other is byte loss. §4 says name the reason, and the reason named is
/// the wrong one.
#[test]
fn a_short_chunk_on_a_required_column_names_the_shortfall_not_a_phantom_null() {
    const ROWS: usize = 2_480;
    const KEPT: usize = 400;

    let sound = multi_page_cash_file(ROWS, 200);
    let short = patch_footer(&sound, |meta| {
        meta.row_groups[0].columns[0]
            .meta_data
            .as_mut()
            .expect("the timestamp chunk has metadata")
            .num_values = i64::try_from(KEPT).expect("fits");
    });

    let file = LakeFile::from_bytes(short).expect("a well-formed footer still parses");
    let err = file
        .read_row_group(0)
        .expect_err("a short required column must be refused");
    assert!(
        matches!(
            err,
            LakeError::ShortColumnChunk {
                column: "timestamp",
                row: KEPT,
                expected: ROWS,
                arrived: KEPT,
            }
        ),
        "byte loss and a vendor gap are different faults with different remedies; \
         this one must not arrive as UnexpectedNull. Got: {err:?}"
    );
    let text = err.to_string();
    assert!(
        text.contains("400") && text.contains("2480"),
        "the refusal must say the chunk delivered {KEPT} of {ROWS}, not that row \
         {KEPT} holds a null. Got: {text}"
    );
    assert!(
        !text.contains("is null at row"),
        "the old message said the file has a null here, which is the wrong fault: {text}"
    );
}
