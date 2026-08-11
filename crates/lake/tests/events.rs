//! The events this crate emits, proven to reach a file on disk.
//!
//! # What was missing
//!
//! `crates/lake` has three `telemetry::emit` call sites — one in
//! [`lake::page`], one in `lake::schema`, one in [`lake::reader`] — and until
//! this file existed exactly one of them was proven to write anything.
//! `lake::page::a_refused_page_writes_its_reason_to_the_log` covers that one
//! and states the case for why it had to exist: CI gate 18 deleted an `emit`
//! outright and the whole suite stayed green. The other two carried the same
//! defect. `note_shape` could have been mutated to `()` and no gate would have
//! noticed; the `lake.file` line could have been deleted and every test in
//! `synthetic.rs` and `refusals.rs` would still pass, because every one of them
//! asserts on the `Result` and none of them asserts that anything was
//! *recorded*. An observation nothing observes is worth what an untested branch
//! is worth, and `CLAUDE.md` §4's ban on a test that asserts nothing is the
//! same rule wearing the other mask.
//!
//! # Why this is its own test binary, and why there is one test in it
//!
//! `telemetry::install` writes a per-process `OnceLock` and **refuses a second
//! call**, naming the path already installed. So there is at most one install
//! per process, and the unit of work is therefore one table-driven test per
//! *binary*. `crates/lake`'s lib target already spends its single install on
//! the page test; cargo compiles every file under `tests/` into its own binary
//! and runs it as its own process, so the install below is a different
//! `OnceLock` and cannot collide with that one.
//!
//! The floor is `Trace`. `lake.file` announces a successful open at `Debug`,
//! which the default `Info` floor drops — a test that left the floor alone
//! would watch the successful-open line get filtered and conclude nothing.
//! What the floor does is `telemetry::sink`'s to prove, not this file's.
//!
//! # Why the fixtures are written rather than committed
//!
//! CI gate 1 walks every tracked file and allows exactly `.rs .toml .md .lock
//! .html .css .yml` outside `web/`. A `.parquet` fixture cannot be committed to
//! this repository — it is the build failure that gate exists to be. So the
//! six files below are built in memory by `parquet`'s own writer, which is
//! available with no cargo features enabled and therefore pulls no C, and
//! written to a temporary directory. They must reach a real path: the
//! `lake.file` line is emitted by [`lake::reader::LakeFile::open`], which takes
//! a `&Path`, and `from_bytes` — the entry point every other test in this
//! crate uses — emits nothing at all.

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

use std::sync::Arc;

use lake::reader::LakeFile;

use parquet::basic::{Compression, Repetition, Type as PhysicalType};
use parquet::column::writer::ColumnWriter;
use parquet::file::properties::WriterProperties;
use parquet::file::writer::SerializedFileWriter;
use parquet::schema::types::Type;

use telemetry::{Config, Level, OwnedValue, Query};

/// The seven cash columns, in the order and the types the lake writes them.
///
/// Kept flat and complete here so that each fixture below is one *named*
/// departure from a file that opens, and the event it produces can be read as
/// the consequence of that departure and nothing else.
const CASH: [(&str, PhysicalType); 7] = [
    ("timestamp", PhysicalType::INT64),
    ("open", PhysicalType::DOUBLE),
    ("high", PhysicalType::DOUBLE),
    ("low", PhysicalType::DOUBLE),
    ("close", PhysicalType::DOUBLE),
    ("volume", PhysicalType::INT64),
    ("open_interest", PhysicalType::INT64),
];

/// How many rows every fixture carries.
///
/// Three, and not zero: the `rows` field of a refused open is `0`, so a sound
/// file that also held zero rows would make that field prove nothing.
const ROWS: i64 = 3;

/// Builds a three-row uncompressed Parquet file from the columns given.
///
/// `nested` names one column to wrap in a single `OPTIONAL` group, which is
/// the only way to produce a leaf at maximum definition level 2 — the fourth
/// shape refusal, and the one that is invisible to a name check because a
/// `ColumnDescriptor`'s `name()` is the *leaf* name.
fn parquet_bytes(cols: &[(&str, PhysicalType)], nested: Option<&str>) -> Vec<u8> {
    let leaf = |name: &str, ty: PhysicalType| -> Arc<Type> {
        Arc::new(
            Type::primitive_type_builder(name, ty)
                .with_repetition(Repetition::OPTIONAL)
                .build()
                .expect("a flat optional leaf"),
        )
    };
    let fields: Vec<Arc<Type>> = cols
        .iter()
        .map(|(name, ty)| {
            if nested == Some(*name) {
                Arc::new(
                    Type::group_type_builder("wrap")
                        .with_repetition(Repetition::OPTIONAL)
                        .with_fields(vec![leaf(name, *ty)])
                        .build()
                        .expect("one optional group deep"),
                )
            } else {
                leaf(name, *ty)
            }
        })
        .collect();
    let schema = Arc::new(
        Type::group_type_builder("schema")
            .with_fields(fields)
            .build()
            .expect("a schema"),
    );
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(Compression::UNCOMPRESSED)
            .build(),
    );

    let mut out: Vec<u8> = Vec::new();
    {
        let mut writer = SerializedFileWriter::new(&mut out, schema, props).expect("a writer");
        let mut group = writer.next_row_group().expect("a row group");
        let mut at = 0_usize;
        while let Some(mut column) = group.next_column().expect("a column") {
            let (name, _) = cols[at];
            // A leaf inside one optional group is PRESENT at definition level
            // 2, and carries repetition levels because the group is a level of
            // nesting. A flat leaf is present at 1 and carries none.
            let (defs, reps): (&[i16], Option<&[i16]>) = if nested == Some(name) {
                (&[2, 2, 2], Some(&[0, 0, 0]))
            } else {
                (&[1, 1, 1], None)
            };
            match column.untyped() {
                ColumnWriter::Int64ColumnWriter(typed) => {
                    typed
                        .write_batch(&[1_i64, 2, 3], Some(defs), reps)
                        .expect("three i64");
                }
                ColumnWriter::DoubleColumnWriter(typed) => {
                    typed
                        .write_batch(&[1.0_f64, 2.0, 3.0], Some(defs), reps)
                        .expect("three f64");
                }
                _ => panic!("these fixtures hold only INT64 and DOUBLE columns"),
            }
            column.close().expect("the column closes");
            at += 1;
        }
        group.close().expect("the row group closes");
        writer.close().expect("the file closes");
    }
    out
}

/// The four fields one `lake.schema` line must carry.
///
/// They are asserted in full rather than by substring because the whole reason
/// `note_shape` takes four arguments is that a shape refusal an operator cannot
/// act on is not worth writing: `column` says where to look, and `want` against
/// `got` says what to change.
struct Shape {
    /// What the line says happened.
    said: &'static str,
    /// The column it names.
    column: &'static str,
    /// What this build required there.
    want: &'static str,
    /// What the file held instead.
    got: &'static str,
}

/// One fixture file, the outcome it must produce, and the schema line — if any
/// — that must precede that outcome on disk.
struct Case {
    /// The file's name under the fixture directory. It must appear in the
    /// `path` field of the `lake.file` line, so an operator reading the log
    /// knows which of 116,086 contract files this was.
    name: &'static str,
    /// The bytes written there.
    bytes: Vec<u8>,
    /// Whether `LakeFile::open` must accept it. Exactly one case is `true`.
    opens: bool,
    /// The `lake.schema` line this file must produce first, or `None` for the
    /// two whose fault is not a schema fault at all.
    schema: Option<Shape>,
}

/// EVERY EMIT SITE IN THIS CRATE OUTSIDE `page.rs` REACHES A FILE, driven
/// through the production path and read back through the public reader.
///
/// Six files are written to disk and opened. Between them they drive both
/// remaining sites and all four arms of the schema one:
///
/// | fixture | `lake.schema` | `lake.file` |
/// |---|---|---|
/// | the seven cash columns | — | `opened`, `Debug`, three rows |
/// | a sentence, not Parquet | — | `refused`, `Error` |
/// | three columns | count | `refused`, `Error` |
/// | `close` renamed `shut` | position | `refused`, `Error` |
/// | `volume` as DOUBLE | type | `refused`, `Error` |
/// | `open_interest` nested | level | `refused`, `Error` |
///
/// # What each assertion is holding down
///
/// **The lines are walked in order, interleaved.** A schema line must arrive
/// *before* the file line for the same fixture, because the two are not
/// interchangeable and the ordering is what proves it: `lake.file` says a path
/// was refused and structurally cannot say why — `detect` runs inside
/// `from_bytes`, before `open` knows the outcome — so a schema line arriving
/// after would be a different event about a different thing.
///
/// **The walk ends by asserting there is nothing left.** That is the bound
/// D-0075 states in prose: one line per file, never one per row group and never
/// one per row. A `note_shape` that had crept into the loop body rather than
/// the refusing arms, or a second `lake.file` line, fails here and nowhere
/// else.
///
/// **Absence is asserted where presence is.** The sound file must produce *no*
/// `lake.schema` line — `note_shape`'s own contract is that only the refusing
/// arms call it, and a version that announced every file would drown the one
/// line an operator needs. Likewise `rows` is `3` on the open and `0` on every
/// refusal, which is a claim about the `match` in `LakeFile::open` and not
/// merely about a constant: a level and a row count that did not move with the
/// outcome would leave "the lake is empty" and "the lake would not open"
/// reaching a page as the same line, which is the silence that emit site was
/// added to end.
#[test]
fn every_file_this_reader_opens_or_refuses_writes_its_line_to_the_log() {
    let root = std::env::temp_dir().join(format!("brutex-lake-events-{}", std::process::id()));
    let _ignored = std::fs::remove_dir_all(&root);
    let logs = root.join("log");
    let files = root.join("files");
    std::fs::create_dir_all(&files).expect("a directory to write the fixtures into");

    let sink = telemetry::install(&Config::new(&logs).with_min_level(Level::Trace))
        .expect("this binary holds the only install in its own process");

    let renamed: Vec<(&str, PhysicalType)> = CASH
        .iter()
        .map(|(name, ty)| {
            if *name == "close" {
                ("shut", *ty)
            } else {
                (*name, *ty)
            }
        })
        .collect();
    let retyped: Vec<(&str, PhysicalType)> = CASH
        .iter()
        .map(|(name, ty)| {
            if *name == "volume" {
                (*name, PhysicalType::DOUBLE)
            } else {
                (*name, *ty)
            }
        })
        .collect();

    let cases = vec![
        Case {
            name: "sound.parquet",
            bytes: parquet_bytes(&CASH, None),
            opens: true,
            schema: None,
        },
        Case {
            // Not Parquet at all, and long enough that the magic check is what
            // refuses it rather than the length check.
            name: "sentence.parquet",
            bytes: b"this is a sentence and it is not a parquet file".to_vec(),
            opens: false,
            schema: None,
        },
        Case {
            name: "three-columns.parquet",
            bytes: parquet_bytes(&CASH[..3], None),
            opens: false,
            schema: Some(Shape {
                said: "the column count is neither shape the lake holds",
                column: "(the whole file)",
                want: "7 columns (cash or index) or 17 (F&O)",
                got: "3 columns",
            }),
        },
        Case {
            // The dangerous one: the right number of columns under a different
            // name, which read positionally would put a rho where a volume
            // belongs.
            name: "renamed.parquet",
            bytes: parquet_bytes(&renamed, None),
            opens: false,
            schema: Some(Shape {
                said: "a column is not the one this build expects at that position",
                column: "close",
                want: "close at leaf 4",
                got: "shut at leaf 4",
            }),
        },
        Case {
            name: "retyped.parquet",
            bytes: parquet_bytes(&retyped, None),
            opens: false,
            schema: Some(Shape {
                said: "a column holds a physical type this build does not decode",
                column: "volume",
                want: "INT64",
                got: "DOUBLE",
            }),
        },
        Case {
            name: "nested.parquet",
            bytes: parquet_bytes(&CASH, Some("open_interest")),
            opens: false,
            schema: Some(Shape {
                said: "a column is nested or repeated, and every row of it would read as null",
                column: "open_interest",
                want: "definition level 1, repetition level 0",
                got: "definition level 2, repetition level 0",
            }),
        },
    ];

    for case in &cases {
        let path = files.join(case.name);
        std::fs::write(&path, &case.bytes).expect("the fixture reaches the disk");
        assert_eq!(
            LakeFile::open(&path).is_ok(),
            case.opens,
            "the premise for {} is wrong: this fixture no longer produces the \
             outcome whose event the rest of this test is about",
            case.name
        );
    }

    let found = telemetry::tail(&logs, sink.keep_files(), &Query::last(64));
    assert_eq!(found.malformed, 0, "every line this crate wrote decodes");
    assert!(
        found.errors.is_empty(),
        "the log itself was readable: {:?}",
        found.errors
    );

    // The tail hands back the newest first; the fixtures ran oldest first.
    let mut lines = found.records;
    lines.reverse();

    let mut at = 0_usize;
    for case in &cases {
        if let Some(shape) = &case.schema {
            let line = lines.get(at).unwrap_or_else(|| {
                panic!(
                    "{} produced no line where its lake.schema refusal belongs. \
                     Without this the emit in schema::note_shape is deletable and \
                     every gate stays green. The log held:\n{lines:#?}",
                    case.name
                )
            });
            assert_eq!(line.target, "lake.schema", "for {}", case.name);
            assert_eq!(line.level, Level::Error, "a shape refusal is never quiet");
            assert_eq!(line.message, shape.said, "for {}", case.name);
            assert_eq!(
                line.field("column").and_then(OwnedValue::as_str),
                Some(shape.column),
                "the line must name WHERE to look, for {}: {:?}",
                case.name,
                line.fields
            );
            assert_eq!(
                line.field("want").and_then(OwnedValue::as_str),
                Some(shape.want),
                "and what this build required, for {}: {:?}",
                case.name,
                line.fields
            );
            assert_eq!(
                line.field("got").and_then(OwnedValue::as_str),
                Some(shape.got),
                "and what the file held, for {}: {:?}",
                case.name,
                line.fields
            );
            at += 1;
        }

        let line = lines.get(at).unwrap_or_else(|| {
            panic!(
                "{} produced no lake.file line. Without this the emit in \
                 LakeFile::open is deletable and every gate stays green. The \
                 log held:\n{lines:#?}",
                case.name
            )
        });
        assert_eq!(line.target, "lake.file", "for {}", case.name);
        assert_eq!(
            line.message,
            if case.opens { "opened" } else { "refused" },
            "for {}",
            case.name
        );
        assert_eq!(
            line.level,
            if case.opens {
                Level::Debug
            } else {
                Level::Error
            },
            "a file this build cannot read is louder than one it can, and {} \
             disagreed",
            case.name
        );
        assert!(
            line.field("path")
                .and_then(OwnedValue::as_str)
                .is_some_and(|path| path.ends_with(case.name)),
            "the line must name the file, for {}: {:?}",
            case.name,
            line.fields
        );
        assert_eq!(
            line.field("bytes").and_then(OwnedValue::as_u64),
            Some(u64::try_from(case.bytes.len()).expect("a fixture smaller than u64::MAX")),
            "and how much of it was read, for {}: {:?}",
            case.name,
            line.fields
        );
        assert_eq!(
            line.field("rows").and_then(OwnedValue::as_i64),
            Some(if case.opens { ROWS } else { 0 }),
            "a refusal counts no rows and an open counts all of them, and {} \
             disagreed: {:?}",
            case.name,
            line.fields
        );
        at += 1;
    }

    assert_eq!(
        at,
        lines.len(),
        "SIX FILES, TEN LINES, AND NOTHING ELSE. A seventh line is either a \
         second event for one file or a schema line off the refusing arms, and \
         both break the bound D-0075 states: one line per file, never one per \
         row group and never one per row. The log held:\n{lines:#?}"
    );

    let _ignored = std::fs::remove_dir_all(&root);
}
