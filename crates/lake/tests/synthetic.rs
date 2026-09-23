//! End-to-end decode tests against Parquet files this test writes itself.
//!
//! # Why the fixtures are synthesised rather than committed
//!
//! CI gate 1 walks every tracked file and allows exactly `.rs .toml .md .lock
//! .html .css .yml` outside `web/`. A `.parquet` fixture **cannot be committed
//! to this repository** — it is the build failure that gate exists to be.
//!
//! So these tests build the file in memory with `parquet`'s own writer, which
//! is available with no cargo features enabled and therefore pulls no C. The
//! pages are UNCOMPRESSED, which exercises every layer of the reader except
//! the ZSTD path: the footer, the schema check, the page headers, the
//! definition levels, the null expansion and the paisa conversion.
//!
//! The ZSTD path and the real column values are covered by `real_lake.rs`,
//! which reads the actual 40 GB lake when it is present.

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

use lake::bar::OPEN_INTEREST_NULL;
use lake::error::{ColumnType, LakeError};
use lake::reader::LakeFile;
use lake::schema::Layout;

use parquet::basic::{Compression, Repetition, Type as PhysicalType};
use parquet::column::writer::ColumnWriter;
use parquet::file::properties::WriterProperties;
use parquet::file::writer::SerializedFileWriter;
use parquet::schema::types::Type;

use parquet_format_safe::FileMetaData;
use parquet_format_safe::thrift::protocol::{TCompactInputProtocol, TCompactOutputProtocol};

/// One column's worth of values to write, already split into present values
/// and the definition levels that place them.
enum Col {
    I64(Vec<i64>, Vec<i16>),
    I32(Vec<i32>, Vec<i16>),
    F64(Vec<f64>, Vec<i16>),
}

/// Builds an uncompressed Parquet file from `(name, physical type, column)`.
fn write(cols: &[(&str, PhysicalType, Col)]) -> Vec<u8> {
    let fields: Vec<Arc<Type>> = cols
        .iter()
        .map(|(name, ty, _)| {
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
    let mut out: Vec<u8> = Vec::new();
    {
        let mut writer = SerializedFileWriter::new(&mut out, schema, props).expect("writer");
        let mut group = writer.next_row_group().expect("row group");
        let mut at = 0;
        while let Some(mut column) = group.next_column().expect("column") {
            let (_, _, source) = cols.get(at).expect("column count matches schema");
            match (column.untyped(), source) {
                (ColumnWriter::Int64ColumnWriter(typed), Col::I64(vals, defs)) => {
                    typed
                        .write_batch(vals, Some(defs), None)
                        .expect("write i64");
                }
                (ColumnWriter::Int32ColumnWriter(typed), Col::I32(vals, defs)) => {
                    typed
                        .write_batch(vals, Some(defs), None)
                        .expect("write i32");
                }
                (ColumnWriter::DoubleColumnWriter(typed), Col::F64(vals, defs)) => {
                    typed
                        .write_batch(vals, Some(defs), None)
                        .expect("write f64");
                }
                _ => panic!("writer type does not match the column"),
            }
            column.close().expect("close column");
            at += 1;
        }
        group.close().expect("close row group");
        writer.close().expect("close writer");
    }
    out
}

/// The seven cash columns, with the values given and no nulls.
fn cash_file(ts: &[i64], px: &[f64], vol: &[i64], oi: &[Option<i64>]) -> Vec<u8> {
    let all = vec![1_i16; ts.len()];
    let (oi_vals, oi_defs) = split(oi);
    write(&[
        (
            "timestamp",
            PhysicalType::INT64,
            Col::I64(ts.to_vec(), all.clone()),
        ),
        (
            "open",
            PhysicalType::DOUBLE,
            Col::F64(px.to_vec(), all.clone()),
        ),
        (
            "high",
            PhysicalType::DOUBLE,
            Col::F64(px.to_vec(), all.clone()),
        ),
        (
            "low",
            PhysicalType::DOUBLE,
            Col::F64(px.to_vec(), all.clone()),
        ),
        (
            "close",
            PhysicalType::DOUBLE,
            Col::F64(px.to_vec(), all.clone()),
        ),
        ("volume", PhysicalType::INT64, Col::I64(vol.to_vec(), all)),
        (
            "open_interest",
            PhysicalType::INT64,
            Col::I64(oi_vals, oi_defs),
        ),
    ])
}

/// Splits optionals into the present values and their definition levels.
fn split<T: Copy>(v: &[Option<T>]) -> (Vec<T>, Vec<i16>) {
    let mut vals = Vec::new();
    let mut defs = Vec::new();
    for x in v {
        match x {
            Some(x) => {
                vals.push(*x);
                defs.push(1);
            }
            None => defs.push(0),
        }
    }
    (vals, defs)
}

#[test]
fn a_cash_file_decodes_to_paisa_and_the_layout_is_detected() {
    let bytes = cash_file(
        &[1_000, 2_000, 3_000],
        &[23_109.55, 49.5, 8_473.1],
        &[10, 20, 30],
        &[Some(5), None, Some(0)],
    );
    let f = LakeFile::from_bytes(bytes).expect("a file this test just wrote must open");
    assert_eq!(f.layout(), Layout::Cash);
    assert_eq!(f.num_rows(), 3);
    assert_eq!(f.row_groups(), 1);

    let b = f.read_row_group(0).expect("decode");
    assert_eq!(b.len(), 3);

    let r0 = b.row(0).expect("row 0");
    assert_eq!(r0.timestamp_micros, 1_000);
    // Prices are paisa integers, converted half-up at the one boundary.
    assert_eq!(r0.close.raw(), 2_310_955);
    assert_eq!(r0.volume, 10);
    assert_eq!(r0.open_interest(), Some(5));

    assert_eq!(b.row(1).expect("row 1").close.raw(), 4_950);
    assert_eq!(b.row(2).expect("row 2").close.raw(), 847_310);
}

#[test]
fn a_null_open_interest_stays_distinct_from_a_zero_one() {
    // Row 0 has a real zero, row 1 has a null, row 2 has a real value. The
    // whole point is that rows 0 and 1 must not come back the same.
    let bytes = cash_file(
        &[1, 2, 3],
        &[10.0, 10.0, 10.0],
        &[1, 1, 1],
        &[Some(0), None, Some(75)],
    );
    let f = LakeFile::from_bytes(bytes).expect("open");
    let b = f.read_row_group(0).expect("decode");

    let zero = b.row(0).expect("row 0");
    let null = b.row(1).expect("row 1");
    let real = b.row(2).expect("row 2");

    assert_eq!(zero.open_interest, 0, "a real zero stays zero");
    assert_eq!(zero.open_interest(), Some(0));

    assert_eq!(
        null.open_interest, OPEN_INTEREST_NULL,
        "a null becomes the i64::MIN sentinel"
    );
    assert_eq!(null.open_interest(), None);

    assert_ne!(
        zero.open_interest, null.open_interest,
        "zero and null must never collapse into each other"
    );
    assert_eq!(real.open_interest(), Some(75));
}

#[test]
fn a_null_shifts_no_later_value_onto_the_wrong_bar() {
    // The failure this guards: parquet stores only present values, so if the
    // reader ignored definition levels, 300 would land on row 1 instead of
    // row 2 and every bar after a null would carry its neighbour's data.
    let bytes = cash_file(
        &[1, 2, 3, 4],
        &[1.0, 2.0, 3.0, 4.0],
        &[100, 200, 300, 400],
        &[Some(100), None, Some(300), None],
    );
    let f = LakeFile::from_bytes(bytes).expect("open");
    let b = f.read_row_group(0).expect("decode");

    assert_eq!(b.row(0).expect("r0").open_interest(), Some(100));
    assert_eq!(b.row(1).expect("r1").open_interest(), None);
    assert_eq!(
        b.row(2).expect("r2").open_interest(),
        Some(300),
        "the value after a null must stay on its own row"
    );
    assert_eq!(b.row(3).expect("r3").open_interest(), None);

    // And the always-present columns are unaffected.
    let vols: Vec<i64> = b.iter().map(|r| r.volume).collect();
    assert_eq!(vols, vec![100, 200, 300, 400]);
}

#[test]
fn a_null_price_is_refused_by_name_rather_than_becoming_zero() {
    let all = vec![1_i16; 2];
    let bytes = write(&[
        (
            "timestamp",
            PhysicalType::INT64,
            Col::I64(vec![1, 2], all.clone()),
        ),
        // `high` is null on row 1.
        (
            "open",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0, 2.0], all.clone()),
        ),
        (
            "high",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0], vec![1, 0]),
        ),
        (
            "low",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0, 2.0], all.clone()),
        ),
        (
            "close",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0, 2.0], all.clone()),
        ),
        ("volume", PhysicalType::INT64, Col::I64(vec![1, 2], all)),
        (
            "open_interest",
            PhysicalType::INT64,
            Col::I64(vec![], vec![0, 0]),
        ),
    ]);
    let f = LakeFile::from_bytes(bytes).expect("open");
    match f.read_row_group(0) {
        Err(LakeError::UnexpectedNull { column, row }) => {
            assert_eq!(column, "high");
            assert_eq!(row, 1);
        }
        other => panic!(
            "expected UnexpectedNull, got {other:?}",
            other = other.err()
        ),
    }
}

#[test]
fn a_wrongly_typed_column_is_refused_by_name() {
    // `volume` written as a DOUBLE where the layout requires INT64.
    let all = vec![1_i16; 1];
    let bytes = write(&[
        (
            "timestamp",
            PhysicalType::INT64,
            Col::I64(vec![1], all.clone()),
        ),
        (
            "open",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0], all.clone()),
        ),
        (
            "high",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0], all.clone()),
        ),
        (
            "low",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0], all.clone()),
        ),
        (
            "close",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0], all.clone()),
        ),
        (
            "volume",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0], all.clone()),
        ),
        ("open_interest", PhysicalType::INT64, Col::I64(vec![1], all)),
    ]);
    match LakeFile::from_bytes(bytes) {
        Err(LakeError::ColumnTypeMismatch { name, want, got }) => {
            assert_eq!(name, "volume");
            assert_eq!(want, ColumnType::Int64);
            assert_eq!(got, ColumnType::Double);
        }
        other => panic!(
            "expected ColumnTypeMismatch, got {other:?}",
            other = other.err()
        ),
    }
}

#[test]
fn a_renamed_column_is_refused_rather_than_read_positionally() {
    // Right count, right types, one wrong name. Reading this positionally
    // would file `turnover` as the volume without a murmur.
    let all = vec![1_i16; 1];
    let bytes = write(&[
        (
            "timestamp",
            PhysicalType::INT64,
            Col::I64(vec![1], all.clone()),
        ),
        (
            "open",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0], all.clone()),
        ),
        (
            "high",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0], all.clone()),
        ),
        (
            "low",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0], all.clone()),
        ),
        (
            "close",
            PhysicalType::DOUBLE,
            Col::F64(vec![1.0], all.clone()),
        ),
        (
            "turnover",
            PhysicalType::INT64,
            Col::I64(vec![1], all.clone()),
        ),
        ("open_interest", PhysicalType::INT64, Col::I64(vec![1], all)),
    ]);
    match LakeFile::from_bytes(bytes) {
        Err(LakeError::UnexpectedSchema { columns, names }) => {
            assert_eq!(columns, 7);
            assert!(names.contains(&"turnover".to_owned()), "{names:?}");
        }
        other => panic!(
            "expected UnexpectedSchema, got {other:?}",
            other = other.err()
        ),
    }
}

#[test]
fn a_schema_of_an_unknown_width_is_refused() {
    let all = vec![1_i16; 1];
    let bytes = write(&[
        (
            "timestamp",
            PhysicalType::INT64,
            Col::I64(vec![1], all.clone()),
        ),
        ("open", PhysicalType::DOUBLE, Col::F64(vec![1.0], all)),
    ]);
    match LakeFile::from_bytes(bytes) {
        Err(LakeError::UnexpectedSchema { columns, .. }) => assert_eq!(columns, 2),
        other => panic!(
            "expected UnexpectedSchema, got {other:?}",
            other = other.err()
        ),
    }
}

#[test]
fn an_fno_file_carries_greeks_at_full_precision_and_spot_as_paisa() {
    let all = vec![1_i16; 2];
    let d = |v: [f64; 2]| Col::F64(v.to_vec(), all.clone());
    let bytes = write(&[
        (
            "timestamp",
            PhysicalType::INT64,
            Col::I64(vec![1, 2], all.clone()),
        ),
        ("open", PhysicalType::DOUBLE, d([49.5, 49.5])),
        ("high", PhysicalType::DOUBLE, d([49.5, 49.5])),
        ("low", PhysicalType::DOUBLE, d([49.5, 49.5])),
        ("close", PhysicalType::DOUBLE, d([49.5, 49.5])),
        (
            "volume",
            PhysicalType::INT64,
            Col::I64(vec![76, 76], all.clone()),
        ),
        (
            "open_interest",
            PhysicalType::INT64,
            Col::I64(vec![75, 75], all.clone()),
        ),
        // Row 0 carries the real sample values; row 1 has the whole greeks
        // block null, which is the pattern measured in the lake.
        (
            "iv",
            PhysicalType::DOUBLE,
            Col::F64(vec![0.678_618_464_061_773_3], vec![1, 0]),
        ),
        (
            "delta",
            PhysicalType::DOUBLE,
            Col::F64(vec![0.103_791_597_602_742_98], vec![1, 0]),
        ),
        (
            "gamma",
            PhysicalType::DOUBLE,
            Col::F64(vec![0.000_171_426_804_295_494_02], vec![1, 0]),
        ),
        (
            "theta",
            PhysicalType::DOUBLE,
            Col::F64(vec![-7.788_961_946_081_791], vec![1, 0]),
        ),
        (
            "vega",
            PhysicalType::DOUBLE,
            Col::F64(vec![2.791_593_074_801_929_3], vec![1, 0]),
        ),
        (
            "rho",
            PhysicalType::DOUBLE,
            Col::F64(vec![0.276_837_206_382_207_27], vec![1, 0]),
        ),
        // spot_at_bar is present on BOTH rows: it is independently nullable.
        ("spot_at_bar", PhysicalType::DOUBLE, d([8_473.1, 8_500.0])),
        (
            "t_years_used",
            PhysicalType::DOUBLE,
            Col::F64(vec![0.033_280_060_882_800_61], vec![1, 0]),
        ),
        (
            "rate_used",
            PhysicalType::DOUBLE,
            Col::F64(vec![0.065], vec![1, 0]),
        ),
        (
            "greeks_provenance_id",
            PhysicalType::INT32,
            Col::I32(vec![1_380_610, 1_380_611], all),
        ),
    ]);

    let f = LakeFile::from_bytes(bytes).expect("open");
    assert_eq!(f.layout(), Layout::Fno);
    let b = f.read_row_group(0).expect("decode");

    let r0 = b.row(0).expect("row 0");
    let g = r0.greeks.expect("row 0 has greeks");
    // Full precision, bit for bit. Nothing is snapped to the paisa grid.
    assert_eq!(g.iv.to_bits(), 0.678_618_464_061_773_3_f64.to_bits());
    assert_eq!(
        g.gamma.to_bits(),
        0.000_171_426_804_295_494_02_f64.to_bits()
    );
    assert_eq!(g.theta.to_bits(), (-7.788_961_946_081_791_f64).to_bits());
    assert_eq!(g.rate_used.to_bits(), 0.065_f64.to_bits());
    // ...while the spot beside them IS money, and is an integer.
    assert_eq!(r0.spot_at_bar.expect("spot").raw(), 847_310);
    assert_eq!(r0.greeks_provenance_id, Some(1_380_610));

    // Row 1: greeks null as a unit, spot and provenance still present.
    let r1 = b.row(1).expect("row 1");
    assert!(!r1.has_greeks(), "the block is null as a unit");
    assert_eq!(r1.spot_at_bar.expect("spot").raw(), 850_000);
    assert_eq!(r1.greeks_provenance_id, Some(1_380_611));
}

#[test]
fn a_partly_null_greeks_row_is_refused_rather_than_half_reported() {
    let all = vec![1_i16; 1];
    let d = |v: f64| Col::F64(vec![v], all.clone());
    let bytes = write(&[
        (
            "timestamp",
            PhysicalType::INT64,
            Col::I64(vec![1], all.clone()),
        ),
        ("open", PhysicalType::DOUBLE, d(1.0)),
        ("high", PhysicalType::DOUBLE, d(1.0)),
        ("low", PhysicalType::DOUBLE, d(1.0)),
        ("close", PhysicalType::DOUBLE, d(1.0)),
        (
            "volume",
            PhysicalType::INT64,
            Col::I64(vec![1], all.clone()),
        ),
        (
            "open_interest",
            PhysicalType::INT64,
            Col::I64(vec![1], all.clone()),
        ),
        ("iv", PhysicalType::DOUBLE, d(0.5)),
        ("delta", PhysicalType::DOUBLE, d(0.5)),
        ("gamma", PhysicalType::DOUBLE, d(0.5)),
        ("theta", PhysicalType::DOUBLE, d(0.5)),
        // vega null while the rest are present: a pattern the lake never shows.
        ("vega", PhysicalType::DOUBLE, Col::F64(vec![], vec![0])),
        ("rho", PhysicalType::DOUBLE, d(0.5)),
        ("spot_at_bar", PhysicalType::DOUBLE, d(100.0)),
        ("t_years_used", PhysicalType::DOUBLE, d(0.5)),
        ("rate_used", PhysicalType::DOUBLE, d(0.065)),
        (
            "greeks_provenance_id",
            PhysicalType::INT32,
            Col::I32(vec![1], all),
        ),
    ]);
    let f = LakeFile::from_bytes(bytes).expect("open");
    match f.read_row_group(0) {
        Err(LakeError::PartialGreeks { row, present }) => {
            assert_eq!(row, 0);
            assert_eq!(present, 7, "seven of the eight were there");
        }
        other => panic!("expected PartialGreeks, got {other:?}", other = other.err()),
    }
}

#[test]
fn a_row_group_past_the_end_is_refused() {
    let bytes = cash_file(&[1], &[1.0], &[1], &[Some(1)]);
    let f = LakeFile::from_bytes(bytes).expect("open");
    match f.read_row_group(1) {
        Err(LakeError::NoSuchRowGroup { asked, held }) => {
            assert_eq!(asked, 1);
            assert_eq!(held, 1);
        }
        other => panic!(
            "expected NoSuchRowGroup, got {other:?}",
            other = other.err()
        ),
    }
}

#[test]
fn a_file_truncated_after_it_was_valid_is_refused_not_read_short() {
    // Write a real file, then cut its tail off. This is the shape an
    // interrupted write leaves behind, and it must not decode partially.
    let good = cash_file(&[1, 2, 3], &[1.0, 2.0, 3.0], &[1, 2, 3], &[Some(1); 3]);
    assert!(
        LakeFile::from_bytes(good.clone()).is_ok(),
        "baseline must open"
    );

    let cut = good.len() / 2;
    let truncated = good.get(..cut).expect("shorter than the whole").to_vec();
    let err = LakeFile::from_bytes(truncated).expect_err("a half file must not open");
    // Either the tail magic is gone or the footer will not parse; both are
    // named refusals, and neither is a partial read.
    assert!(
        matches!(
            err,
            LakeError::NotParquet { .. } | LakeError::FooterUnreadable { .. }
        ),
        "got {err:?}"
    );
}

#[test]
fn read_all_returns_every_row_group() {
    let bytes = cash_file(&[1, 2], &[1.0, 2.0], &[1, 2], &[Some(1), Some(2)]);
    let f = LakeFile::from_bytes(bytes).expect("open");
    let all = f.read_all().expect("read all");
    assert_eq!(all.len(), 1);
    assert_eq!(all.first().expect("one group").len(), 2);
}

// ---------------------------------------------------------------------------
// A FOOTER THAT PARSES BUT LIES.
//
// Every refusal above is driven by a file that is malformed in its BYTES. The
// two below are malformed in their VALUES: the thrift footer decodes perfectly
// and then says the column chunk starts at a negative offset or is a negative
// number of bytes long. That is what a bad sector, a half-written file or a
// truncated object body leaves behind, and it is the one shape `parquet`'s own
// accessor answers with `assert!` rather than an error.
// ---------------------------------------------------------------------------

/// Re-serialises a file's footer after `edit` has changed it.
///
/// The only way to obtain a file whose footer parses and whose values are
/// impossible: `parquet`'s writer will never emit one, and a corrupt fixture
/// cannot be committed to this repository at all — CI gate 1 allows no
/// `.parquet` file, as the module header explains. So the corruption is applied
/// here, in memory, to a file this test wrote a moment earlier.
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

/// Three rows of cash bars, valid in every respect.
fn sound_cash_file() -> Vec<u8> {
    cash_file(
        &[1, 2, 3],
        &[10.0, 11.0, 12.0],
        &[100, 200, 300],
        &[Some(7), Some(8), Some(9)],
    )
}

/// **A negative chunk length is refused by name and never aborts the process.**
///
/// `parquet`'s `ColumnChunkMetaData::byte_range` ends in
///
/// ```text
/// assert!(col_start >= 0 && col_len >= 0,
///         "column start and length should not be negative");
/// ```
///
/// so while the reader called it, one corrupt footer took the whole process
/// down — and took with it the sweep that was part-way through the other
/// 115 files. There is nothing to catch: `[profile.release]` sets
/// `panic = "abort"`. `CLAUDE.md` §4 forbids exactly this — degrade loudly and
/// name the reason, or refuse.
///
/// Restoring the `chunk.byte_range()` call in `Columns::pages` makes this test
/// abort rather than fail.
#[test]
fn a_negative_chunk_length_in_the_footer_is_refused_by_name_and_never_aborts() {
    let broken = patch_footer(&sound_cash_file(), |meta| {
        meta.row_groups[0].columns[0]
            .meta_data
            .as_mut()
            .expect("the timestamp chunk has metadata")
            .total_compressed_size = -16;
    });

    // The file still OPENS: the footer is well-formed thrift. The impossible
    // value is only reached when the chunk is read, and that is where it must
    // be named.
    let file = LakeFile::from_bytes(broken).expect("a well-formed footer still parses");
    match file.read_row_group(0) {
        Err(LakeError::ImpossibleLength { what, value }) => {
            assert_eq!(what, "column chunk length");
            assert_eq!(value, -16, "the offending number is the one reported");
        }
        other => panic!(
            "expected ImpossibleLength, got {:?}",
            other.map(|b| b.len())
        ),
    }
}

/// **A negative chunk offset is refused by name, from either field it can
/// come from.**
///
/// `byte_range` takes the chunk's start from `dictionary_page_offset` when
/// there is one and from `data_page_offset` when there is not, so both arms are
/// corrupted here. Covering only the first would leave the fallback — the arm a
/// dictionary-less column takes — unproven.
#[test]
fn a_negative_chunk_offset_is_refused_by_name_from_either_field_it_can_come_from() {
    for (label, dictionary, data, expected) in [
        ("dictionary page", Some(-4_i64), None, -4_i64),
        ("data page", None, Some(-8_i64), -8_i64),
    ] {
        let broken = patch_footer(&sound_cash_file(), |meta| {
            let chunk = meta.row_groups[0].columns[0]
                .meta_data
                .as_mut()
                .expect("the timestamp chunk has metadata");
            // `None` is not "leave it alone" here: the data-page case must
            // clear the dictionary offset the writer set, or the first arm
            // shadows the second and the fallback is never taken.
            chunk.dictionary_page_offset = dictionary;
            if let Some(offset) = data {
                chunk.data_page_offset = offset;
            }
        });

        let file = LakeFile::from_bytes(broken).expect("a well-formed footer still parses");
        match file.read_row_group(0) {
            Err(LakeError::ImpossibleLength { what, value }) => {
                assert_eq!(what, "column chunk offset", "{label}");
                assert_eq!(value, expected, "{label}");
            }
            other => panic!(
                "{label}: expected ImpossibleLength, got {:?}",
                other.map(|b| b.len())
            ),
        }
    }
}
