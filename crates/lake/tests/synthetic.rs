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
//! pages it writes are UNCOMPRESSED, which exercises the footer, the schema
//! check, the page headers, the definition levels, the null expansion and the
//! paisa conversion.
//!
//! The ZSTD path is driven too, by the section at the end of this file: it
//! recompresses every page of such a file with `ruzstd`'s own encoder and
//! requires the copy to decode to exactly what the original does. That section
//! says what it does not prove — its frames are `ruzstd`'s, not the ones Polars
//! wrote — and the real frames and the real column values are still covered
//! only by `real_lake.rs`. Every test there is `#[ignore]`d on every machine,
//! so `cargo test` and CI never run one; they read the lake — the 40 GB at
//! `~/.brutex/lake` that D-0056 describes — only when started explicitly with
//! `--ignored` on a machine that has it.

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

use lake::bar::{Bar, OPEN_INTEREST_NULL};
use lake::error::{ColumnType, LakeError};
use lake::reader::LakeFile;
use lake::schema::Layout;

use parquet::basic::{Compression, Repetition, Type as PhysicalType};
use parquet::column::writer::ColumnWriter;
use parquet::file::properties::WriterProperties;
use parquet::file::writer::SerializedFileWriter;
use parquet::schema::types::Type;

use parquet_format_safe::thrift::protocol::{TCompactInputProtocol, TCompactOutputProtocol};
use parquet_format_safe::{CompressionCodec, FileMetaData, PageHeader, PageType};

use ruzstd::encoding::{CompressionLevel, compress_to_vec};

/// One column's worth of values to write, already split into present values
/// and the definition levels that place them.
enum Col {
    I64(Vec<i64>, Vec<i16>),
    I32(Vec<i32>, Vec<i16>),
    F64(Vec<f64>, Vec<i16>),
}

/// Builds an uncompressed Parquet file from `(name, physical type, column)`.
fn write(cols: &[(&str, PhysicalType, Col)]) -> Vec<u8> {
    write_with(
        cols,
        WriterProperties::builder()
            .set_compression(Compression::UNCOMPRESSED)
            .build(),
    )
}

/// As [`write`], under writer properties the caller chooses.
///
/// Every fixture above the ZSTD section takes the writer's defaults, and those
/// defaults dictionary-encode every column and put each chunk's rows in ONE
/// data page. The ZSTD round trip needs the other shape as well — PLAIN values
/// and several pages to a chunk — so the properties are a parameter rather than
/// a second copy of the column loop below.
fn write_with(cols: &[(&str, PhysicalType, Col)], props: WriterProperties) -> Vec<u8> {
    write_groups(&[cols], props)
}

/// As [`write_with`], one row group per element of `groups`, in order.
///
/// `SerializedFileWriter` opens a row group when `next_row_group` is called
/// and at no other time. `WriterProperties`' row-group row limit does not
/// split one: in `parquet` 59.2 the only code that splits a group on it is
/// the Arrow writer, which this crate's `default-features = false` leaves
/// out. So several row groups are several calls, and every group must name
/// the same columns in the same order — the schema is taken from the first.
fn write_groups(groups: &[&[(&str, PhysicalType, Col)]], props: WriterProperties) -> Vec<u8> {
    let cols = groups.first().expect("a file has at least one row group");
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
    let props = Arc::new(props);
    let mut out: Vec<u8> = Vec::new();
    {
        let mut writer = SerializedFileWriter::new(&mut out, schema, props).expect("writer");
        for cols in groups {
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
        }
        writer.close().expect("close writer");
    }
    out
}

/// The seven cash columns, with the values given and no nulls.
fn cash_file(ts: &[i64], px: &[f64], vol: &[i64], oi: &[Option<i64>]) -> Vec<u8> {
    write(&cash_columns(ts, px, vol, oi))
}

/// The columns [`cash_file`] writes, for a caller that needs its own writer
/// properties.
fn cash_columns(
    ts: &[i64],
    px: &[f64],
    vol: &[i64],
    oi: &[Option<i64>],
) -> Vec<(&'static str, PhysicalType, Col)> {
    let all = vec![1_i16; ts.len()];
    let (oi_vals, oi_defs) = split(oi);
    vec![
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
    ]
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
    let (mut meta, start) = read_footer(bytes);
    edit(&mut meta);
    close_with_footer(bytes[..start].to_vec(), &meta)
}

/// A file's thrift footer, and the offset it starts at.
fn read_footer(bytes: &[u8]) -> (FileMetaData, usize) {
    let n = bytes.len();
    let declared: [u8; 4] = bytes[n - 8..n - 4]
        .try_into()
        .expect("four bytes of footer length");
    let flen = usize::try_from(u32::from_le_bytes(declared)).expect("a footer length fits a usize");
    let start = n - 8 - flen;
    let mut cursor = Cursor::new(&bytes[start..n - 8]);
    let mut proto = TCompactInputProtocol::new(&mut cursor, flen * 64 + 1_000_000);
    let meta = FileMetaData::read_from_in_protocol(&mut proto)
        .expect("the footer this test just wrote parses");
    (meta, start)
}

/// Appends `meta` to `out` as a footer: the thrift bytes, their length, and
/// the closing magic.
fn close_with_footer(mut out: Vec<u8>, meta: &FileMetaData) -> Vec<u8> {
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

// ---------------------------------------------------------------------------
// THE ZSTD PATH THROUGH A WHOLE FILE, WHICH EVERY REAL LAKE FILE TAKES AND NO
// CI RUN USED TO.
//
// Every lake file is Polars-written Parquet with ZSTD-compressed pages, so the
// one decode path the operator's data always takes is the `Compression::ZSTD`
// arm of `Columns::pages` and the `Codec::Zstd` arm of `LakePageReader::decode`.
// Until this section the only tests that reached that path END TO END were the
// `#[ignore]`d ones in `real_lake.rs`, which no CI runner runs, and every
// fixture above is UNCOMPRESSED. So `Columns::pages` could map
// `Compression::ZSTD(_)` to a refusal, or to `Codec::Uncompressed`, and the
// whole lake suite stayed green: both mutations were run against the reader
// and its tests as they stood before this section, and neither failed a test.
//
// THE DECODE ARM WAS NOT THE GAP. The `Codec::Zstd` arm of `decode` was already
// unit-tested in `src/page.rs`, on hand-built Raw and RLE frames, and returning
// the body undecoded there fails four of those tests. What they did not do is
// reach that arm through `LakeFile`, walk a chunk of more than one compressed
// page, or hand it a Compressed block; this section and `page.rs`'s
// `a_ruzstd_encoded_compressed_block_decodes_through_the_page_path` add those.
//
// `parquet`'s own writer cannot produce the fixture — its `zstd` feature is the
// C binding `crates/lake/Cargo.toml` exists to keep out. `ruzstd` can: the
// crate this reader already decodes with also carries a public encoder,
// `ruzstd::encoding`, gated on no cargo feature. So the file is written
// UNCOMPRESSED exactly as above and every page body is then recompressed in
// memory, with the page headers and the footer rewritten to match. No
// dependency is added and `Cargo.lock` does not move.
//
// WHAT THIS DOES NOT PROVE. The frames are `ruzstd`'s, at its `Fastest` level,
// not the ones Polars' zstd wrote. The two encoders are free to choose
// different block types, literal encodings and sequence tables, so a frame
// feature only the real encoder emits is still reached only by `real_lake.rs`.
// What is proven here is the reader's plumbing — the codec mapping, the page
// walk over compressed bodies, the header-length check and the value decoders
// behind it — on frames a conforming encoder produced.
// ---------------------------------------------------------------------------

/// What [`zstd_copy`] rewrote, so a test can prove the rewrite reached every
/// kind of page it claims to.
struct Recompressed {
    /// The whole ZSTD file.
    bytes: Vec<u8>,
    /// `DICTIONARY_PAGE`s recompressed, across every chunk.
    dictionary_pages: usize,
    /// `DATA_PAGE`s recompressed, across every chunk.
    data_pages: usize,
}

/// The four bytes every ZSTD frame opens with, RFC 8878 §3.1.1.
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// A footer offset or length as a `usize`, for a file this test wrote.
fn unsigned(v: i64) -> usize {
    usize::try_from(v).expect("a footer this test wrote holds no negative offset")
}

/// A `usize` position as the `i64` a footer carries.
fn signed(v: usize) -> i64 {
    i64::try_from(v).expect("a test file is far under 8 EiB")
}

/// Rewrites an UNCOMPRESSED file as a ZSTD one, page by page.
///
/// Each column chunk's pages are walked with the same thrift `PageHeader` the
/// reader parses; each body is compressed with `ruzstd`'s encoder; the header
/// is re-serialised with the new `compressed_page_size`; and the chunk's
/// `dictionary_page_offset`, `data_page_offset` and `total_compressed_size`,
/// and its row group's `file_offset` and `total_compressed_size`, are moved to
/// where the smaller pages now sit. `codec` becomes ZSTD. Nothing else changes —
/// not a value, not a definition level, not `uncompressed_page_size`, and not
/// one byte of any page once it is decompressed again.
///
/// **The page index is dropped rather than moved.** Its offsets would point at
/// bytes that no longer exist, and a stale pointer left in a fixture is a trap
/// for the next test that reads it. `LakeFile` does not: `parquet`'s
/// `ParquetMetaDataReader::new()` defaults to `PageIndexPolicy::Skip`.
/// `ColumnChunk::file_offset` is left alone because `parquet` 59.2 writes `0`
/// there, so there is nothing to move.
fn zstd_copy(original: &[u8]) -> Recompressed {
    let (mut meta, _) = read_footer(original);
    let mut out: Vec<u8> = b"PAR1".to_vec();
    let (mut dictionary_pages, mut data_pages) = (0, 0);
    for group in &mut meta.row_groups {
        let group_start = out.len();
        for column in &mut group.columns {
            let chunk = column
                .meta_data
                .as_mut()
                .expect("every chunk this test wrote has metadata");
            assert_eq!(
                chunk.codec,
                CompressionCodec::UNCOMPRESSED,
                "the source must be uncompressed, or its bodies are not the page bytes"
            );
            let first = unsigned(
                chunk
                    .dictionary_page_offset
                    .unwrap_or(chunk.data_page_offset),
            );
            let end = first + unsigned(chunk.total_compressed_size);
            let data_at = unsigned(chunk.data_page_offset);
            let chunk_start = out.len();
            let mut moved_data_at = None;

            let mut pos = first;
            while pos < end {
                let mut cursor = Cursor::new(&original[pos..end]);
                let mut header = {
                    let mut proto = TCompactInputProtocol::new(&mut cursor, end - pos);
                    PageHeader::read_from_in_protocol(&mut proto)
                        .expect("a page header this test just wrote parses")
                };
                let body_at =
                    pos + usize::try_from(cursor.position()).expect("a header length fits a usize");
                let body =
                    &original[body_at..body_at + unsigned(header.compressed_page_size.into())];
                assert_eq!(
                    header.uncompressed_page_size, header.compressed_page_size,
                    "an uncompressed page is its own size"
                );
                match header.type_ {
                    PageType::DICTIONARY_PAGE => dictionary_pages += 1,
                    PageType::DATA_PAGE => data_pages += 1,
                    other => panic!("page type {} is not one this writer emits", other.0),
                }

                let squeezed = compress_to_vec(body, CompressionLevel::Fastest);
                assert_eq!(
                    squeezed.get(..4),
                    Some(&ZSTD_MAGIC[..]),
                    "ruzstd wrote a ZSTD frame and not the body back"
                );
                header.compressed_page_size =
                    i32::try_from(squeezed.len()).expect("a test page fits an i32");
                // A page CRC covers the COMPRESSED bytes, so the old one is now
                // wrong. `None` is what a writer that computes none emits.
                header.crc = None;
                if pos == data_at {
                    moved_data_at = Some(out.len());
                }
                {
                    let mut proto = TCompactOutputProtocol::new(&mut out);
                    header
                        .write_to_out_protocol(&mut proto)
                        .expect("a rewritten page header serialises");
                }
                out.extend_from_slice(&squeezed);
                pos = body_at + body.len();
            }
            assert_eq!(pos, end, "the walk ends exactly on the chunk's own end");

            chunk.codec = CompressionCodec::ZSTD;
            chunk.data_page_offset = signed(
                moved_data_at.expect("`data_page_offset` names a page boundary the walk crossed"),
            );
            if chunk.dictionary_page_offset.is_some() {
                chunk.dictionary_page_offset = Some(signed(chunk_start));
            }
            chunk.total_compressed_size = signed(out.len() - chunk_start);
            column.offset_index_offset = None;
            column.offset_index_length = None;
            column.column_index_offset = None;
            column.column_index_length = None;
        }
        if group.file_offset.is_some() {
            group.file_offset = Some(signed(group_start));
        }
        if group.total_compressed_size.is_some() {
            group.total_compressed_size = Some(signed(out.len() - group_start));
        }
    }
    Recompressed {
        bytes: close_with_footer(out, &meta),
        dictionary_pages,
        data_pages,
    }
}

/// Every group's rows, group by group — the whole of what a file decodes to,
/// with the row-group boundaries it was decoded across still in place.
fn groups(file: &LakeFile) -> Vec<Vec<Bar>> {
    file.read_all()
        .expect("every group decodes")
        .iter()
        .map(|group| group.iter().collect())
        .collect()
}

/// Every row of every group, in order, with the boundaries flattened away.
fn rows(file: &LakeFile) -> Vec<Bar> {
    groups(file).into_iter().flatten().collect()
}

/// **A ZSTD copy of a file decodes to exactly what the UNCOMPRESSED original
/// does, through the codec arm every real lake file takes.**
///
/// Three fixtures, because the three shapes take different paths through the
/// reader. `sound_cash_file` is what the writer's defaults produce: every
/// column dictionary-encoded, so each chunk opens with a `DICTIONARY_PAGE` and
/// the chunk start comes from `dictionary_page_offset`. The second turns the
/// dictionary off and caps a page at one row: PLAIN values, no dictionary page,
/// so the start falls back to `data_page_offset`, and four compressed pages per
/// chunk, so the walk has to land each header exactly where the previous
/// compressed body ended. It carries a null open interest beside a real zero,
/// so definition levels cross the codec as well.
///
/// The third is THREE row groups, of 2, 3 and 1 rows, under the writer's
/// defaults. The first two are one row group each, and one row group never
/// asks the rewrite to move a second group's chunk offsets, or the reader to
/// find a compressed chunk that starts where another group's compressed bytes
/// end. Its null sits in the middle group, so a definition level crosses the
/// codec past the first group too. The two reads are compared group by group,
/// not flattened, so a row that came back in the wrong group fails.
///
/// **The equality is the claim, and the rest of the test is what stops it being
/// vacuous.** Two reads that both fail the same way, or a copy that was never
/// actually compressed, would compare equal too. So the rewrite must have
/// reached every page it should; every chunk of the copy is asserted to be
/// labelled ZSTD; the group count and each group's row count are pinned; and
/// the same copy relabelled UNCOMPRESSED must be REFUSED, which proves the
/// bodies really are frames and that the label is what routes them through
/// `ruzstd`. The values both reads agree on are pinned by the test after this
/// one.
///
/// Mapping `Compression::ZSTD(_)` in `Columns::pages` to a refusal, or to
/// `Codec::Uncompressed`, makes this test fail, and restoring it makes it pass.
#[test]
fn a_zstd_copy_decodes_to_exactly_what_the_uncompressed_original_does() {
    let plain_and_paged = write_with(
        &cash_columns(
            &[1, 2, 3, 4],
            &[10.0, 11.5, 12.25, 13.0],
            &[100, 200, 300, 400],
            &[Some(7), None, Some(0), Some(9)],
        ),
        WriterProperties::builder()
            .set_compression(Compression::UNCOMPRESSED)
            .set_dictionary_enabled(false)
            .set_write_batch_size(1)
            .set_data_page_row_count_limit(1)
            .build(),
    );
    let (first, second, third) = (
        cash_columns(&[1, 2], &[10.0, 11.0], &[100, 200], &[Some(7), Some(8)]),
        cash_columns(
            &[3, 4, 5],
            &[12.0, 13.5, 14.25],
            &[300, 400, 500],
            &[None, Some(0), Some(9)],
        ),
        cash_columns(&[6], &[15.0], &[600], &[Some(10)]),
    );
    let three_groups = write_groups(
        &[first.as_slice(), second.as_slice(), third.as_slice()],
        WriterProperties::builder()
            .set_compression(Compression::UNCOMPRESSED)
            .build(),
    );

    for (label, original, want_group_rows, want_dictionary_pages, want_data_pages) in [
        ("dictionary-encoded", sound_cash_file(), &[3][..], 7, 7),
        ("PLAIN, one row per page", plain_and_paged, &[4][..], 0, 28),
        ("three row groups", three_groups, &[2, 3, 1][..], 21, 21),
    ] {
        let copy = zstd_copy(&original);
        assert_eq!(
            (copy.dictionary_pages, copy.data_pages),
            (want_dictionary_pages, want_data_pages),
            "{label}: the rewrite reached every page of every chunk"
        );
        let (meta, _) = read_footer(&copy.bytes);
        assert!(
            meta.row_groups
                .iter()
                .flat_map(|group| &group.columns)
                .all(|column| column
                    .meta_data
                    .as_ref()
                    .is_some_and(|chunk| chunk.codec == CompressionCodec::ZSTD)),
            "{label}: every chunk of the copy is labelled ZSTD"
        );

        let want = groups(&LakeFile::from_bytes(original).expect("the original opens"));
        let zstd = LakeFile::from_bytes(copy.bytes.clone()).expect("the ZSTD copy opens");
        assert_eq!(zstd.layout(), Layout::Cash, "{label}");
        assert_eq!(
            zstd.row_groups(),
            want_group_rows.len(),
            "{label}: the copy holds every row group the original does"
        );
        let got = groups(&zstd);
        let got_group_rows: Vec<usize> = got.iter().map(Vec::len).collect();
        assert_eq!(
            got_group_rows, want_group_rows,
            "{label}: every row came back, in the row group it was written to"
        );
        assert_eq!(
            got, want,
            "{label}: the ZSTD copy decodes to the uncompressed original, \
             group for group and bar for bar"
        );

        // THE LABEL IS WHAT ROUTES THE BODIES THROUGH `ruzstd`. The same bytes
        // called UNCOMPRESSED reach the arm that compares a body's length with
        // `uncompressed_page_size`, and a compressed body is not that length.
        let mislabelled = patch_footer(&copy.bytes, |meta| {
            for group in &mut meta.row_groups {
                for column in &mut group.columns {
                    column
                        .meta_data
                        .as_mut()
                        .expect("every chunk has metadata")
                        .codec = CompressionCodec::UNCOMPRESSED;
                }
            }
        });
        match LakeFile::from_bytes(mislabelled)
            .expect("the relabelled footer still parses")
            .read_all()
        {
            Err(LakeError::PageDecode { column, reason }) => {
                assert_eq!(column, "timestamp", "{label}: the first chunk read refuses");
                assert!(
                    reason.contains("uncompressed page is"),
                    "{label}: refused as a body of the wrong size, got: {reason}"
                );
            }
            other => panic!(
                "{label}: frames read as UNCOMPRESSED must be refused, got {:?}",
                other.map(|groups| groups.len())
            ),
        }
    }
}

/// **The values themselves, read out of the ZSTD copy.** The comparison above
/// proves the copy agrees with the original; this pins what both say, so a
/// defect shared by the two reads cannot hide behind their agreement.
#[test]
fn a_zstd_copy_carries_paisa_nulls_and_zeros_to_the_right_rows() {
    let copy = zstd_copy(&cash_file(
        &[1_000, 2_000, 3_000],
        &[23_109.55, 49.5, 8_473.1],
        &[10, 20, 30],
        &[Some(5), None, Some(0)],
    ));
    let got = rows(&LakeFile::from_bytes(copy.bytes).expect("the ZSTD copy opens"));

    let stamps: Vec<i64> = got.iter().map(|bar| bar.timestamp_micros).collect();
    assert_eq!(stamps, [1_000, 2_000, 3_000]);
    let closes: Vec<i64> = got.iter().map(|bar| bar.close.raw()).collect();
    assert_eq!(closes, [2_310_955, 4_950, 847_310], "paisa, half-up");
    let volumes: Vec<i64> = got.iter().map(|bar| bar.volume).collect();
    assert_eq!(volumes, [10, 20, 30]);
    let interest: Vec<Option<i64>> = got.iter().map(Bar::open_interest).collect();
    assert_eq!(
        interest,
        [Some(5), None, Some(0)],
        "a null stays a null and a zero stays a zero after the codec"
    );
}
