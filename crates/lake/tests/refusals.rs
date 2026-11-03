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

use lake::error::{ColumnType, LakeError};
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
                // A chunk short of its bytes must be refused by name, not
                // padded. (This arm used to collect into a list asserted empty
                // at the end; it fails here, on the first accepted cut.)
                Ok(batch) => {
                    let got: Vec<Option<i64>> = (0..batch.len())
                        .map(|i| batch.row(i).expect("row in range").open_interest())
                        .collect();
                    let real = got.iter().filter(|v| v.is_some()).count();
                    let first = got.iter().position(Option::is_none);
                    panic!(
                        "{layout}, cut to 1/{divisor}: ACCEPTED, rows={}, real={real}, \
                         FABRICATED nulls={}, first at row {first:?} whose true value is {:?}",
                        batch.len(),
                        got.len() - real,
                        first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))
                    );
                }
            }
        }
    }

    // And the case that lands exactly on a page boundary, where the remainder
    // is empty rather than short. `page.rs` answers that with `Ok(None)` — the
    // same answer it gives a chunk that is genuinely finished.
    let sound = multi_page_cash_file(ROWS, 200);
    let full = footer_chunk_len(&sound, OI);
    // COUNTED (P1-14-07): every iteration could `continue` -- a helper whose
    // page-header read stopped matching the writer's layout answers `None` for
    // every cut -- and the half below would be green having run nothing.
    // BASE-COV-LIBS-LS3: a cut the helper cannot find, or one that is the
    // whole chunk, now fails here by name rather than being skipped and
    // caught only by the count below.
    let mut boundary_cases = 0_usize;
    let cuts = [1_i64, 2, 4, 8, 12];
    for pages in cuts {
        let len = exact_page_prefix(&sound, OI, pages)
            .unwrap_or_else(|| panic!("no {pages}-page prefix: the helper lost the layout"));
        assert!(
            len < full,
            "{pages} page(s) are the whole {full}-byte chunk; the fixture must hold more"
        );
        boundary_cases += 1;
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
                panic!(
                    "cut to exactly {pages} page(s) ({len} of {full} bytes): ACCEPTED, \
                     rows={}, real={real}, FABRICATED nulls={}, first at row {first:?} \
                     whose true value is {:?}",
                    batch.len(),
                    got.len() - real,
                    first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))
                );
            }
        }
    }
    assert_eq!(
        boundary_cases,
        cuts.len(),
        "every exact page-boundary cut must reach `read_row_group`; the fixture \
         holds more pages than the largest cut"
    );
    // The helper itself: asked for more pages than the chunk holds, it finds
    // none rather than answering the whole chunk.
    assert_eq!(exact_page_prefix(&sound, OI, 1_000), None);
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
        // Opening it at all is the defect, whatever a read would then do: the
        // shape is refused at the SCHEMA gate, before a page is decompressed.
        // A read used to decode every row to None while the file holds 7, 8,
        // 9 — each of them i64::MIN downstream, the vendor-reported-none
        // sentinel, invented. (Until BASE-COV-LIBS-LS3 this arm also accepted
        // a read that decoded 7, 8, 9; the reader refuses the shape, so that
        // was an outcome no reader of this crate can produce.)
        Ok(file) => panic!(
            "ACCEPTED a definition-level-2 open_interest: the file opened as {file:?}; the \
             shape must be refused at the SCHEMA gate, before a page is decompressed"
        ),
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

// ---------------------------------------------------------------------------
// BASE-COV-LIBS-LS3 (attempt 5): THE SCAN ITSELF, ON FILES MADE TO CARRY EACH
// DEFECT.
//
// The real-lake test below proves something only if the scan can see what it
// asserts is absent: a scan that never noticed a short chunk would pass the
// real lake vacuously. So every detector is shown firing on a file built to
// trip it, and quiet on a sound one, on every machine; the real lake adds its
// own files on a machine that has it. Extreme cases: a REQUIRED leaf, a
// REPEATED leaf, 3, 7 and 17 leaves, version-2 data pages, a declared count
// of zero, a chunk cut to one page, a negative offset, a negative length, a
// chunk past the end of the file, a page header of garbage, a page whose body
// length is negative, a directory and a file of junk where a lake file
// should be; a stride sample of nothing, of zero, of more than there is.
// Combinations: both defect-1 lies in one chunk at once, and a declared short
// count beside a nested leaf in one file. Out of the box: the tree the scan
// walks holds a year that is a file and a contract with no `1minute`.
// ---------------------------------------------------------------------------

/// A file of `leaves` INT64 leaves `c0..`, three rows each, every leaf
/// OPTIONAL but the last, which is `last`; `v2` writes version-2 data pages.
fn int64_leaves_file(leaves: usize, last: Repetition, v2: bool) -> Vec<u8> {
    let mut kinds = vec![Repetition::OPTIONAL; leaves];
    kinds[leaves - 1] = last;
    let fields: Vec<Arc<Type>> = kinds
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            Arc::new(
                Type::primitive_type_builder(&format!("c{i}"), PhysicalType::INT64)
                    .with_repetition(*kind)
                    .build()
                    .expect("leaf"),
            )
        })
        .collect();
    let schema = Arc::new(
        Type::group_type_builder("schema")
            .with_fields(fields)
            .build()
            .expect("schema"),
    );
    let version = [
        parquet::file::properties::WriterVersion::PARQUET_1_0,
        parquet::file::properties::WriterVersion::PARQUET_2_0,
    ][usize::from(v2)];
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(Compression::UNCOMPRESSED)
            .set_writer_version(version)
            .build(),
    );
    let mut out: Vec<u8> = Vec::new();
    {
        let mut writer = SerializedFileWriter::new(&mut out, schema, props).expect("writer");
        let mut group = writer.next_row_group().expect("row group");
        for kind in &kinds {
            let mut column = group.next_column().expect("column").expect("one per leaf");
            let ColumnWriter::Int64ColumnWriter(typed) = column.untyped() else {
                panic!("every leaf is INT64");
            };
            let (defs, reps): (Option<&[i16]>, Option<&[i16]>) = match kind {
                Repetition::REQUIRED => (None, None),
                Repetition::REPEATED => (Some(&[1, 1, 1]), Some(&[0, 0, 0])),
                Repetition::OPTIONAL => (Some(&[1, 1, 1]), None),
            };
            typed.write_batch(&[1_i64, 2, 3], defs, reps).expect("i64");
            column.close().expect("close column");
        }
        group.close().expect("close row group");
        writer.close().expect("close writer");
    }
    out
}

/// An edit of one column chunk's footer metadata.
type ChunkEdit<'a> = &'a dyn Fn(&mut parquet_format_safe::ColumnMetaData);

/// The four defect lists of a scan, by length:
/// `[declared_short, pages_short, nested, unreadable]`.
fn defects(s: &Scan) -> [usize; 4] {
    [
        s.declared_short.len(),
        s.pages_short.len(),
        s.nested.len(),
        s.unreadable.len(),
    ]
}

#[test]
fn the_real_lake_scan_finds_each_defect_in_a_file_made_to_carry_it() {
    let dir = std::env::temp_dir().join(format!("brutex-lake-scan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let scan = |name: &str, bytes: &[u8]| {
        let path = dir.join(name);
        std::fs::write(&path, bytes).expect("write the fixture");
        let mut s = Scan::default();
        scan_file(&path, "2020", &mut s);
        s
    };

    // Sound files: counted by width, every defect list empty.
    let sound = multi_page_cash_file(40, 10);
    let s = scan("cash.parquet", &sound);
    assert_eq!(
        (s.files, s.cash, s.fno, s.row_groups, s.rows),
        (1, 1, 0, 1, 40)
    );
    assert_eq!(defects(&s), [0; 4]);
    assert_eq!(s.years.iter().collect::<Vec<_>>(), ["2020"]);
    let s = scan(
        "wide.parquet",
        &int64_leaves_file(17, Repetition::OPTIONAL, false),
    );
    assert_eq!((s.files, s.cash, s.fno, s.rows), (1, 0, 1, 3));
    assert_eq!(defects(&s), [0; 4]);
    let s = scan(
        "three.parquet",
        &int64_leaves_file(3, Repetition::OPTIONAL, false),
    );
    assert_eq!((s.files, s.cash, s.fno), (1, 0, 0), "neither width");
    let s = scan(
        "v2.parquet",
        &int64_leaves_file(7, Repetition::OPTIONAL, true),
    );
    assert_eq!(
        (s.cash, defects(&s)),
        (1, [0; 4]),
        "version-2 pages are counted"
    );

    // Defect 3: a leaf that is not flat, by either level.
    let s = scan(
        "required.parquet",
        &int64_leaves_file(7, Repetition::REQUIRED, false),
    );
    assert_eq!(defects(&s), [0, 0, 1, 0]);
    assert!(s.nested[0].ends_with("leaf `c6` max_def_level=0 max_rep_level=0"));
    let s = scan(
        "repeated.parquet",
        &int64_leaves_file(7, Repetition::REPEATED, false),
    );
    assert_eq!(defects(&s), [0, 0, 1, 0]);
    assert!(s.nested[0].ends_with("leaf `c6` max_def_level=1 max_rep_level=1"));

    // Defect 1, declared and delivered, each alone and both in one chunk.
    let oi = |bytes: &[u8], edit: &dyn Fn(&mut parquet_format_safe::ColumnMetaData)| {
        patch_footer(bytes, |meta| {
            edit(
                meta.row_groups[0].columns[OI]
                    .meta_data
                    .as_mut()
                    .expect("metadata"),
            );
        })
    };
    let s = scan("declared.parquet", &oi(&sound, &|m| m.num_values = 0));
    assert_eq!(defects(&s), [1, 0, 0, 0]);
    assert!(
        s.declared_short[0].ends_with("`open_interest`: declared num_values=0 against num_rows=40")
    );
    let one_page = exact_page_prefix(&sound, OI, 1).expect("a first page");
    let s = scan(
        "pages.parquet",
        &oi(&sound, &|m| m.total_compressed_size = one_page),
    );
    assert_eq!(defects(&s), [0, 1, 0, 0]);
    assert!(s.pages_short[0].ends_with("page headers deliver 10 values against num_rows=40"));
    let both = oi(&sound, &|m| {
        m.num_values = 10;
        m.total_compressed_size = one_page;
    });
    assert_eq!(defects(&scan("both.parquet", &both)), [1, 1, 0, 0]);

    // A footer whose chunk cannot be walked: every way is named.
    let walks: [(&str, ChunkEdit, &str); 4] = [
        (
            "offset",
            &|m| m.data_page_offset = -1,
            "offset -1 is not a position",
        ),
        (
            "length",
            &|m| m.total_compressed_size = -1,
            "length -1 is not a length",
        ),
        (
            "past",
            &|m| m.total_compressed_size = 1 << 40,
            "runs past the file end",
        ),
        ("garbage", &|m| m.data_page_offset = 0, "page header at 0:"),
    ];
    for (name, edit, said) in walks {
        let s = scan(&format!("{name}.parquet"), &oi(&sound, edit));
        assert_eq!(defects(&s)[3], 1, "{name}");
        assert!(
            s.unreadable[0].contains(said),
            "{name}: {}",
            s.unreadable[0]
        );
    }
    // A page whose header parses and whose body length is negative.
    let mut torn = sound.clone();
    let mut start = 0_usize;
    let _ = patch_footer(&sound, |meta| {
        let chunk = meta.row_groups[0].columns[OI]
            .meta_data
            .as_ref()
            .expect("metadata");
        start = usize::try_from(chunk.data_page_offset).expect("a position");
    });
    let mut header: Vec<u8> = Vec::new();
    parquet_format_safe::PageHeader {
        type_: parquet_format_safe::PageType::DATA_PAGE,
        uncompressed_page_size: 0,
        compressed_page_size: -1,
        crc: None,
        data_page_header: Some(parquet_format_safe::DataPageHeader {
            num_values: 10,
            encoding: parquet_format_safe::Encoding(0),
            definition_level_encoding: parquet_format_safe::Encoding(3),
            repetition_level_encoding: parquet_format_safe::Encoding(3),
            statistics: None,
        }),
        index_page_header: None,
        dictionary_page_header: None,
        data_page_header_v2: None,
    }
    .write_to_out_protocol(&mut TCompactOutputProtocol::new(&mut header))
    .expect("a header serialises");
    torn[start..start + header.len()].copy_from_slice(&header);
    let s = scan("torn.parquet", &torn);
    assert!(
        s.unreadable[0].ends_with("page size -1 is not a length"),
        "{}",
        s.unreadable[0]
    );

    // Where a lake file should be: a directory, and bytes that are not one.
    let mut s = Scan::default();
    scan_file(&dir, "2020", &mut s);
    assert_eq!((s.files, defects(&s)), (0, [0, 0, 0, 1]));
    let s = scan("junk.parquet", b"PAR1 not a parquet footer PAR1");
    assert_eq!((s.files, defects(&s)), (0, [0, 0, 0, 1]));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_real_lake_walk_finds_month_files_and_samples_by_stride() {
    let root = std::env::temp_dir().join(format!("brutex-lake-walk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let fno = root.join("FNO");
    for contract in ["a", "b", "c", "d"] {
        for year in ["2019", "2020"] {
            let at = fno.join(contract).join("1minute").join(year);
            std::fs::create_dir_all(&at).expect("a year");
            for month in ["01.parquet", "02.parquet"] {
                std::fs::write(at.join(month), b"").expect("a month");
            }
        }
    }
    std::fs::create_dir_all(fno.join("e")).expect("a contract with no 1minute");
    std::fs::create_dir_all(fno.join("f").join("1minute")).expect("1minute");
    std::fs::write(fno.join("f").join("1minute").join("2021"), b"").expect("a year that is a file");

    let mut found = Vec::new();
    month_files(&fno.join("a"), &mut found);
    let mut years: Vec<&str> = found.iter().map(|(_, y)| y.as_str()).collect();
    years.sort_unstable();
    assert_eq!(years, ["2019", "2019", "2020", "2020"]);
    assert!(
        found
            .iter()
            .all(|(p, _)| p.extension().is_some_and(|e| e == "parquet"))
    );
    for empty in ["e", "f", "missing"] {
        let mut none = Vec::new();
        month_files(&fno.join(empty), &mut none);
        assert!(none.is_empty(), "{empty}");
    }

    assert_eq!(
        sample_dirs(&fno, 2).len(),
        2,
        "six entries at a stride of three"
    );
    assert_eq!(
        sample_dirs(&fno, 100).len(),
        6,
        "more wanted than there are: all of them"
    );
    assert_eq!(sample_dirs(&fno, 6).len(), 6);
    assert!(sample_dirs(&fno, 0).is_empty(), "none wanted");
    assert!(
        sample_dirs(&fno.join("e"), 3).is_empty(),
        "an empty directory"
    );
    assert!(
        sample_dirs(&root.join("missing"), 3).is_empty(),
        "no directory"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[ignore = "needs ~/.brutex/lake, which cannot be tracked: CI gate 1 forbids a .parquet in this repository. Run with `cargo test -p lake -- --ignored` on a machine that has the lake."]
fn no_real_lake_file_triggers_either_defect() {
    let Some(home) = std::env::var_os("HOME") else {
        panic!(
            "MISSING FIXTURE: ~/.brutex/lake. This test is #[ignore]d precisely because the fixture cannot be tracked, so running it explicitly means you believe you have the lake. Reporting `ok` here would be a test that asserted nothing. (the real-lake scan: no HOME.)"
        );
    };
    let root = std::path::Path::new(&home).join(".brutex/lake/bars/NSE");
    assert!(
        root.exists(),
        "MISSING FIXTURE: ~/.brutex/lake. This test is #[ignore]d precisely because the fixture cannot be tracked, so running it explicitly means you believe you have the lake. Reporting `ok` here would be a test that asserted nothing. (the real-lake scan: ~/.brutex/lake is not on this machine.)"
    );

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
    // month token, so no REAL name is ever changed by this tolerance. No HOME
    // is no lake, and is skipped exactly as an absent lake is.
    let real = std::env::var_os("HOME")
        .map(|home| std::path::Path::new(&home).join(".brutex/lake/bars/NSE/FNO"));
    println!("{}", month_spelling_census(real.as_deref()));
}

/// The month-spelling census over the contract directories under `root`
/// (at most 20,000): every name must parse and render back byte for byte.
/// Answers the line the test prints.
///
/// A SKIP, not a refusal, and the distinction is the whole point of the
/// caller not being `#[ignore]`d. Everything the caller asserts before the
/// census runs without the lake: that case folding stays injective across all
/// twelve months, and that four spellings of one month round-trip to the
/// canonical form. This census is an EXTRA -- it measures that no real
/// directory name is affected -- so its absence must not fail a test that has
/// already proved something.
///
/// A bulk edit turned this into a hard refusal along with the six genuinely
/// vacuous tests in this crate, and CI caught it: the commit message said this
/// test was deliberately excluded and the edit had included it anyway.
///
/// BASE-COV-LIBS-LS3: a function of its root, so the census and its skip are
/// both driven on every machine by
/// `the_month_spelling_census_round_trips_what_it_finds_and_skips_what_is_absent`.
fn month_spelling_census(root: Option<&std::path::Path>) -> String {
    use lake::contract::ContractName;

    let Some(root) = root.filter(|r| r.exists()) else {
        return "SKIPPING the month-spelling census: ~/.brutex/lake is absent.\n  \
                Everything asserted above this point ran."
            .to_owned();
    };
    let mut checked = 0_usize;
    for entry in std::fs::read_dir(root)
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
    format!("{checked} contract directories round-tripped byte for byte")
}

#[test]
fn the_month_spelling_census_round_trips_what_it_finds_and_skips_what_is_absent() {
    let root = std::env::temp_dir().join(format!("brutex-lake-census-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for name in [
        "NSE-NIFTY-01Apr20-10000-CE",
        "NSE-BANKNIFTY-24Apr24-FUT",
        "NSE-NIFTY-02Jan19-1-PE",
    ] {
        std::fs::create_dir_all(root.join(name)).expect("a contract directory");
    }
    assert_eq!(
        month_spelling_census(Some(&root)),
        "3 contract directories round-tripped byte for byte"
    );
    let empty = root.join("NSE-NIFTY-01Apr20-10000-CE");
    assert_eq!(
        month_spelling_census(Some(&empty)),
        "0 contract directories round-tripped byte for byte",
        "an empty root holds nothing to refuse"
    );
    let missing = root.join("absent");
    for absent in [None, Some(missing.as_path())] {
        assert!(month_spelling_census(absent).starts_with("SKIPPING"));
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The census is not vacuous: a name that parses but does not render back as
/// itself fails it.
#[test]
#[should_panic(expected = "a real lake name must round-trip byte for byte")]
fn the_month_spelling_census_fails_on_a_name_that_does_not_round_trip() {
    let root = std::env::temp_dir().join(format!("brutex-lake-census-odd-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("NSE-NIFTY-01APR20-10000-CE")).expect("a directory");
    let _ = month_spelling_census(Some(&root));
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

// ---------------------------------------------------------------------------
// AUDIT probestore-7 — a STORED open-interest sentinel is refused by name, and
// the values this reader does not judge are stated, not implied.
// ---------------------------------------------------------------------------

/// One cash row as the seven leaves hold it; `None` open interest is a null.
#[derive(Clone, Copy)]
struct Row {
    timestamp: i64,
    ohlc: [f64; 4],
    volume: i64,
    open_interest: Option<i64>,
}

/// A one-group, seven-column cash file holding exactly `rows`.
fn cash_file_of(rows: &[Row]) -> Vec<u8> {
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
    let all = vec![1_i16; rows.len()];
    let mut out: Vec<u8> = Vec::new();
    {
        let mut writer = SerializedFileWriter::new(&mut out, schema, props).expect("writer");
        let mut group = writer.next_row_group().expect("row group");
        let mut at = 0_usize;
        while let Some(mut column) = group.next_column().expect("column") {
            match column.untyped() {
                ColumnWriter::Int64ColumnWriter(typed) => match at {
                    0 => {
                        let v: Vec<i64> = rows.iter().map(|r| r.timestamp).collect();
                        typed.write_batch(&v, Some(&all), None).expect("timestamp");
                    }
                    5 => {
                        let v: Vec<i64> = rows.iter().map(|r| r.volume).collect();
                        typed.write_batch(&v, Some(&all), None).expect("volume");
                    }
                    _ => {
                        let v: Vec<i64> = rows.iter().filter_map(|r| r.open_interest).collect();
                        let defs: Vec<i16> = rows
                            .iter()
                            .map(|r| i16::from(r.open_interest.is_some()))
                            .collect();
                        typed
                            .write_batch(&v, Some(&defs), None)
                            .expect("open interest");
                    }
                },
                ColumnWriter::DoubleColumnWriter(typed) => {
                    let v: Vec<f64> = rows.iter().map(|r| r.ohlc[at - 1]).collect();
                    typed.write_batch(&v, Some(&all), None).expect("price");
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

/// A plain, sane row carrying `open_interest`.
const fn sane(open_interest: Option<i64>) -> Row {
    Row {
        timestamp: 1_584_685_680_000_000,
        ohlc: [100.0, 101.0, 99.0, 100.5],
        volume: 5,
        open_interest,
    }
}

/// Opens `bytes` from a private file and reads group 0.
fn read_back(name: &str, bytes: &[u8]) -> Result<lake::batch::Batch, LakeError> {
    let path = std::env::temp_dir().join(format!(
        "brutex-lake-sentinel-{name}-{}.parquet",
        std::process::id()
    ));
    std::fs::write(&path, bytes).expect("the scratch file is writable");
    let file = LakeFile::open(&path).expect("a well-formed cash file opens");
    let read = file.read_row_group(0);
    let _ = std::fs::remove_file(&path);
    read
}

/// A STORED `i64::MIN` OPEN INTEREST IS REFUSED, NAMING ITS ROW.
///
/// The reader maps a null to `i64::MIN`, so a file storing `i64::MIN` read
/// back as "the vendor reported none": a null the file never held. It is
/// refused at the first, a middle and the last row; one above it, zero, a
/// negative and a real null still read, the null as `None` and zero as zero.
#[test]
fn a_stored_open_interest_sentinel_is_refused_by_row() {
    for (name, at) in [("first", 0), ("middle", 2), ("last", 4)] {
        let mut rows = [sane(Some(7)); 5];
        rows[at].open_interest = Some(i64::MIN);
        let read = read_back(name, &cash_file_of(&rows)).map(|b| b.len());
        assert!(
            matches!(
                read,
                Err(LakeError::OpenInterestIsNullSentinel { row }) if row == at
            ),
            "a stored sentinel at row {at}: {read:?}"
        );
    }
    let rows = [
        sane(Some(i64::MIN + 1)),
        sane(Some(0)),
        sane(None),
        sane(Some(-1)),
        sane(Some(i64::MAX)),
    ];
    let batch = read_back("edges", &cash_file_of(&rows)).expect("no stored sentinel");
    let got: Vec<Option<i64>> = (0..5)
        .map(|i| batch.row(i).expect("five rows").open_interest())
        .collect();
    assert_eq!(
        got,
        [Some(i64::MIN + 1), Some(0), None, Some(-1), Some(i64::MAX)]
    );
}

/// WHAT THIS READER DOES NOT JUDGE DECODES AS WRITTEN, AS ITS DOC NOW SAYS.
///
/// The crate header said every bad thing is refused, and an extreme
/// timestamp, a `low` above the `high`, an `open` outside the range, a
/// negative price, volume and open interest all decoded silently. They still
/// decode, because judging a bar is the consumer's job, and the header and
/// `read_row_group` now say so; this pins that statement to the behaviour.
#[test]
fn values_the_reader_does_not_judge_decode_as_written() {
    let row = Row {
        timestamp: i64::MIN,
        ohlc: [100.0, 1.0, 200.0, -3.0],
        volume: -7,
        open_interest: Some(-9),
    };
    let batch = read_back("unjudged", &cash_file_of(&[row])).expect("not judged, so read");
    let bar = batch.row(0).expect("one row");
    assert_eq!(bar.timestamp_micros, i64::MIN);
    assert_eq!(
        [
            bar.open.raw(),
            bar.high.raw(),
            bar.low.raw(),
            bar.close.raw()
        ],
        [10_000, 100, 20_000, -300]
    );
    assert_eq!((bar.volume, bar.open_interest()), (-7, Some(-9)));
    let header = include_str!("../src/lib.rs");
    assert!(header.contains("not every implausible\n//! value in it."));
    let reader = include_str!("../src/reader.rs");
    assert!(reader.contains("/// # What is NOT checked"));
}

/// One cash file whose `timestamp` leaf declares `logical`, three rows.
fn cash_file_with_timestamp(logical: Option<&parquet::basic::LogicalType>) -> Vec<u8> {
    let mut fields: Vec<Arc<Type>> = Vec::new();
    for (name, ty) in CASH {
        let mut leaf = Type::primitive_type_builder(name, ty).with_repetition(Repetition::OPTIONAL);
        if name == "timestamp" {
            leaf = leaf.with_logical_type(logical.cloned());
        }
        fields.push(Arc::new(leaf.build().expect("leaf")));
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
        while let Some(mut column) = group.next_column().expect("column") {
            match column.untyped() {
                ColumnWriter::Int64ColumnWriter(typed) => {
                    typed
                        .write_batch(&[1_i64, 2, 3], Some(&[1, 1, 1]), None)
                        .expect("i64");
                }
                ColumnWriter::DoubleColumnWriter(typed) => {
                    typed
                        .write_batch(&[1.0_f64, 2.0, 3.0], Some(&[1, 1, 1]), None)
                        .expect("f64");
                }
                _ => panic!("the cash layout has no other physical type"),
            }
            column.close().expect("close column");
        }
        group.close().expect("close row group");
        writer.close().expect("close writer");
    }
    out
}

/// audit-20261003 hunt-store-5 (D-1528). The `timestamp` leaf's LOGICAL type
/// was never checked, so a NANOS or MILLIS timestamp, or a MICROS one that is
/// not adjusted to UTC (a wall clock), decoded silently as "microseconds since
/// the epoch, UTC": off by ×1000, ÷1000 or +5h30. Each is now refused by name
/// at the schema gate. An undeclared INT64 and a UTC MICROS timestamp still
/// open.
#[test]
fn a_timestamp_in_another_unit_or_not_in_utc_is_refused_by_name() {
    use parquet::basic::{LogicalType, TimeUnit};
    let stamp = |unit, utc| Some(LogicalType::timestamp(utc, unit));
    for (logical, said) in [
        (stamp(TimeUnit::NANOS, true), "NANOS"),
        (stamp(TimeUnit::MILLIS, true), "MILLIS"),
        (stamp(TimeUnit::MICROS, false), "not adjusted to UTC"),
    ] {
        match LakeFile::from_bytes(cash_file_with_timestamp(logical.as_ref())) {
            Err(LakeError::UnsupportedTimestamp { declared }) => {
                assert!(declared.contains(said), "{said}: {declared}");
            }
            Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),
            Ok(_) => panic!("{said}: a timestamp this reader would misread was opened"),
        }
    }
    for clean in [None, stamp(TimeUnit::MICROS, true)] {
        assert!(
            LakeFile::from_bytes(cash_file_with_timestamp(clean.as_ref())).is_ok(),
            "{clean:?} is what this reader decodes"
        );
    }
}

/// One cash file whose leaf `leaf` is declared with `physical`, `converted`
/// and `logical`, three rows. Every other leaf is plain.
///
/// The parquet writer records exactly what it is handed: a leaf given only a
/// converted type is written with no logical type, which is what a legacy
/// writer (parquet-mr, older pyarrow) emits.
fn cash_file_annotated(
    leaf: &str,
    physical: PhysicalType,
    converted: parquet::basic::ConvertedType,
    mut logical: Option<parquet::basic::LogicalType>,
) -> Vec<u8> {
    let mut fields: Vec<Arc<Type>> = Vec::new();
    for (name, ty) in CASH {
        let mut built =
            Type::primitive_type_builder(name, ty).with_repetition(Repetition::OPTIONAL);
        if name == leaf {
            built = Type::primitive_type_builder(name, physical)
                .with_repetition(Repetition::OPTIONAL)
                .with_converted_type(converted)
                .with_logical_type(logical.take());
        }
        fields.push(Arc::new(built.build().expect("leaf")));
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
        while let Some(mut column) = group.next_column().expect("column") {
            match column.untyped() {
                ColumnWriter::Int64ColumnWriter(typed) => {
                    // 2024-01-01 09:15 IST in MILLISECONDS, then two more.
                    typed
                        .write_batch(
                            &[1_704_080_700_000_i64, 1_704_080_760_000, 1_704_080_820_000],
                            Some(&[1, 1, 1]),
                            None,
                        )
                        .expect("i64");
                }
                ColumnWriter::DoubleColumnWriter(typed) => {
                    typed
                        .write_batch(&[1.0_f64, 2.0, 3.0], Some(&[1, 1, 1]), None)
                        .expect("f64");
                }
                ColumnWriter::Int96ColumnWriter(typed) => {
                    let one = parquet::data_type::Int96::from(vec![1_u32, 2, 3]);
                    typed
                        .write_batch(&[one, one, one], Some(&[1, 1, 1]), None)
                        .expect("i96");
                }
                ColumnWriter::Int32ColumnWriter(typed) => {
                    typed
                        .write_batch(&[1_i32, 2, 3], Some(&[1, 1, 1]), None)
                        .expect("i32");
                }
                _ => panic!("this fixture writes no other physical type"),
            }
            column.close().expect("close column");
        }
        group.close().expect("close row group");
        writer.close().expect("close writer");
    }
    out
}

/// audit-20261004 h-pull-1 (D-2270). D-1528 read only the LOGICAL type, so a
/// `timestamp` leaf carrying only the legacy CONVERTED type `TIMESTAMP_MILLIS`
/// opened, and its milliseconds decoded as microseconds: 1000x too small,
/// January 1970, no refusal. Every converted type but `NONE` and
/// `TIMESTAMP_MICROS` is now refused by name, a logical type that disagrees
/// with its converted type is refused, and an INT96 timestamp still fails the
/// physical-type check.
#[test]
fn a_timestamp_declared_only_by_a_legacy_converted_type_is_refused_unless_it_is_micros() {
    use parquet::basic::{ConvertedType, LogicalType, TimeUnit};

    let millis = cash_file_annotated(
        "timestamp",
        PhysicalType::INT64,
        ConvertedType::TIMESTAMP_MILLIS,
        None,
    );
    match LakeFile::from_bytes(millis) {
        Err(LakeError::UnsupportedTimestamp { declared }) => {
            assert!(declared.contains("TIMESTAMP_MILLIS"), "{declared}");
        }
        Err(other) => panic!("refused as the wrong thing: {other:?}"),
        Ok(_) => panic!("a MILLIS timestamp opened and would decode as micros"),
    }
    // Every other converted type parquet lets annotate an INT64 means
    // something this reader does not decode as a UTC microsecond.
    for other in [
        ConvertedType::INT_64,
        ConvertedType::UINT_64,
        ConvertedType::TIME_MICROS,
    ] {
        match LakeFile::from_bytes(cash_file_annotated(
            "timestamp",
            PhysicalType::INT64,
            other,
            None,
        )) {
            Err(LakeError::UnsupportedTimestamp { declared }) => {
                assert!(declared.contains(&other.to_string()), "{declared}");
            }
            Err(wrong) => panic!("{other}: refused as the wrong thing: {wrong:?}"),
            Ok(_) => panic!("{other}: a timestamp this reader would misread was opened"),
        }
    }
    // The legacy spelling of what the reader decodes still opens, and so do
    // the two spellings D-1528 already accepted.
    for (converted, logical) in [
        (ConvertedType::TIMESTAMP_MICROS, None),
        (ConvertedType::NONE, None),
        (
            ConvertedType::TIMESTAMP_MICROS,
            Some(LogicalType::timestamp(true, TimeUnit::MICROS)),
        ),
    ] {
        assert!(
            LakeFile::from_bytes(cash_file_annotated(
                "timestamp",
                PhysicalType::INT64,
                converted,
                logical.clone(),
            ))
            .is_ok(),
            "{converted} {logical:?} is what this reader decodes"
        );
    }
    // A logical MICROS (UTC) leaf whose converted type was rewritten to
    // MILLIS: the two disagree, and the file never opens.
    let disagreeing = patch_footer(
        &cash_file_annotated(
            "timestamp",
            PhysicalType::INT64,
            ConvertedType::NONE,
            Some(LogicalType::timestamp(true, TimeUnit::MICROS)),
        ),
        |meta| {
            meta.schema[1].converted_type =
                Some(parquet_format_safe::ConvertedType::TIMESTAMP_MILLIS);
        },
    );
    assert!(
        LakeFile::from_bytes(disagreeing).is_err(),
        "a logical MICROS with a converted MILLIS is refused"
    );
    // INT96, the other physical encoding of a timestamp, is not INT64.
    match LakeFile::from_bytes(cash_file_annotated(
        "timestamp",
        PhysicalType::INT96,
        ConvertedType::NONE,
        None,
    )) {
        Err(LakeError::ColumnTypeMismatch { name, .. }) => assert_eq!(name, "timestamp"),
        Err(other) => panic!("INT96 refused as the wrong thing: {other:?}"),
        Ok(_) => panic!("an INT96 timestamp opened"),
    }
}

/// audit-20261004 h-pull-1 (D-2270). The integer count columns had no
/// annotation check at all: an unsigned 64-bit `volume` reads values past
/// `i64::MAX` as negative, and a DECIMAL or TIME annotation is a different
/// quantity under the right name. Only a signed integer annotation, or none,
/// decodes as the plain integer the reader returns.
#[test]
fn an_integer_column_annotated_as_anything_but_a_signed_integer_is_refused_by_name() {
    use parquet::basic::{ConvertedType, LogicalType};

    for (leaf, converted, logical, said) in [
        ("volume", ConvertedType::UINT_64, None, "UINT_64"),
        (
            "open_interest",
            ConvertedType::TIME_MICROS,
            None,
            "TIME_MICROS",
        ),
        (
            "volume",
            ConvertedType::TIMESTAMP_MICROS,
            None,
            "TIMESTAMP_MICROS",
        ),
        (
            "open_interest",
            ConvertedType::NONE,
            Some(LogicalType::integer(64, false)),
            "is_signed: false",
        ),
        (
            "volume",
            ConvertedType::NONE,
            Some(LogicalType::Unknown),
            "Unknown",
        ),
    ] {
        match LakeFile::from_bytes(cash_file_annotated(
            leaf,
            PhysicalType::INT64,
            converted,
            logical.clone(),
        )) {
            Err(LakeError::UnsupportedIntegerAnnotation { name, declared }) => {
                assert_eq!(name, leaf, "{said}");
                assert!(declared.contains(said), "{said}: {declared}");
            }
            Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),
            Ok(_) => panic!("{said}: an integer column this reader would misread was opened"),
        }
    }
    for (converted, logical) in [
        (ConvertedType::NONE, None),
        (ConvertedType::INT_64, None),
        (ConvertedType::NONE, Some(LogicalType::integer(64, true))),
    ] {
        for leaf in ["volume", "open_interest"] {
            assert!(
                LakeFile::from_bytes(cash_file_annotated(
                    leaf,
                    PhysicalType::INT64,
                    converted,
                    logical.clone(),
                ))
                .is_ok(),
                "{leaf} {converted} {logical:?} decodes as a signed integer"
            );
        }
    }
    // BASE-COV-LIBS-LS3: a count written 32 bits wide under a SIGNED
    // annotation, which the annotation check alone would pass, is refused by
    // its physical type first, naming both widths.
    for leaf in ["volume", "open_interest"] {
        match LakeFile::from_bytes(cash_file_annotated(
            leaf,
            PhysicalType::INT32,
            ConvertedType::INT_32,
            Some(LogicalType::integer(32, true)),
        )) {
            Err(LakeError::ColumnTypeMismatch { name, want, got }) => assert_eq!(
                (name, want, got),
                (leaf, ColumnType::Int64, ColumnType::Int32)
            ),
            other => panic!("an INT32 {leaf}: {:?}", other.map(|f| f.num_rows())),
        }
    }
}
