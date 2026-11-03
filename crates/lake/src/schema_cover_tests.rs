#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "tests state their expectations"
)]
//! BASE-COV-LIBS-LS3 (attempt 5): the two parts of `schema.rs` no test above
//! ran. `spec` builds the column tables in a `const` context only, so it never
//! ran at all; and a `timestamp` leaf whose logical type is not a timestamp at
//! all (an integer, a time of day, a decimal, a date) was refused by no test.
//!
//! Extreme cases: the first and last row of each table; a signed and an
//! unsigned integer, a time in micros and in nanos, a decimal at its widest
//! INT64 precision, a date and a string; through `detect` and directly; and,
//! as combinations, a non-timestamp logical type beside a legacy converted
//! type that is itself a misread unit (the logical type is what is named),
//! and a refused timestamp in a file whose every other column is sound.
//! Out of the box: the refusal's rendering is what the operator reads, so it
//! is checked to name the annotation and to differ between annotations.

use std::sync::Arc;

use parquet::basic::{ConvertedType, LogicalType, Repetition, TimeUnit, Type as PhysicalType};
use parquet::schema::types::{SchemaDescriptor, Type};

use super::{
    CASH_SPEC, ColumnSpec, FNO_EXTRA_SPEC, Layout, detect, misread_timestamp,
    misread_timestamp_logical, spec, specs,
};
use crate::error::{ColumnType, LakeError};

/// **`spec` AT RUN TIME BUILDS EVERY ROW BOTH TABLES HOLD.** The tables are
/// built by `spec` in a `const` context, so this is the only place it runs,
/// and every row must come back as written: the name and the physical type.
#[test]
fn spec_at_run_time_rebuilds_every_row_of_both_tables() {
    let rows: Vec<ColumnSpec> = CASH_SPEC.iter().chain(&FNO_EXTRA_SPEC).copied().collect();
    assert_eq!(rows.len(), Layout::Fno.column_count());
    for row in &rows {
        let again = spec(row.name, row.ty);
        assert_eq!((again.name, again.ty), (row.name, row.ty));
    }
    let first = spec("timestamp", ColumnType::Int64);
    assert_eq!((first.name, first.ty), ("timestamp", ColumnType::Int64));
    let last = spec("greeks_provenance_id", ColumnType::Int32);
    let in_table = specs(Layout::Fno).last().expect("seventeen columns");
    assert_eq!((last.name, last.ty), (in_table.name, in_table.ty));
    let odd = spec("", ColumnType::Other);
    assert_eq!((odd.name, odd.ty), ("", ColumnType::Other));
}

/// A cash schema whose `timestamp` leaf carries `logical`.
fn cash_schema_with_timestamp(logical: &LogicalType) -> SchemaDescriptor {
    let fields: Vec<Arc<Type>> = specs(Layout::Cash)
        .map(|s| {
            let physical = match s.ty {
                ColumnType::Int64 => PhysicalType::INT64,
                ColumnType::Double => PhysicalType::DOUBLE,
                ColumnType::Int32 | ColumnType::Other => panic!("the cash layout has neither"),
            };
            let mut field = Type::primitive_type_builder(s.name, physical)
                .with_repetition(Repetition::OPTIONAL);
            if s.name == "timestamp" {
                field = field.with_logical_type(Some(logical.clone()));
            }
            Arc::new(field.build().expect("a primitive field builds"))
        })
        .collect();
    let root = Type::group_type_builder("schema")
        .with_fields(fields)
        .build()
        .expect("the group builds");
    SchemaDescriptor::new(Arc::new(root))
}

/// **A TIMESTAMP WHOSE LOGICAL TYPE IS NOT A TIMESTAMP IS REFUSED BY NAME**,
/// through `detect` exactly as a file's schema reaches it. An INT64 that says
/// it is an integer or a time of day is a column this reader would
/// misread as microseconds since the epoch.
#[test]
fn a_timestamp_annotated_as_something_else_is_refused_naming_the_annotation() {
    let refusals: Vec<String> = [
        LogicalType::integer(64, true),
        LogicalType::time(true, TimeUnit::MICROS),
        LogicalType::time(false, TimeUnit::NANOS),
    ]
    .into_iter()
    .map(|logical| {
        let shown = format!("{logical:?}");
        match detect(&cash_schema_with_timestamp(&logical)) {
            Err(LakeError::UnsupportedTimestamp { declared }) => {
                assert_eq!(
                    declared, shown,
                    "the refusal names the annotation as declared"
                );
                declared
            }
            other => panic!("{shown}: expected UnsupportedTimestamp, got {other:?}"),
        }
    })
    .collect();
    let mut distinct = refusals.clone();
    distinct.dedup();
    assert_eq!(distinct, refusals, "each annotation is named as itself");

    // The sound half: the same schema with the timestamp the reader decodes
    // opens as cash, so it is the annotation that refused above.
    let sound = cash_schema_with_timestamp(&LogicalType::timestamp(true, TimeUnit::MICROS));
    assert_eq!(
        detect(&sound).expect("UTC micros is the lake's own"),
        Layout::Cash
    );
}

/// **EVERY NON-TIMESTAMP LOGICAL TYPE IS A MISREAD, AND IT IS NAMED BEFORE
/// THE LEGACY CONVERTED TYPE.** Driven directly, so annotations no INT64 leaf
/// can carry (a date, a string) are covered too, and so are pairs `parquet`'s
/// schema builder would refuse to build.
#[test]
fn every_non_timestamp_logical_type_is_a_misread_and_is_named_first() {
    for logical in [
        LogicalType::Date,
        LogicalType::String,
        LogicalType::integer(8, false),
        LogicalType::decimal(2, 18),
    ] {
        let shown = format!("{logical:?}");
        assert_eq!(
            misread_timestamp_logical(Some(&logical)).as_deref(),
            Some(shown.as_str())
        );
        // Beside a converted type that is a misread of its own, the logical
        // type is still the one named.
        assert_eq!(
            misread_timestamp(Some(&logical), ConvertedType::TIMESTAMP_MILLIS).as_deref(),
            Some(shown.as_str())
        );
        assert_eq!(
            misread_timestamp(Some(&logical), ConvertedType::NONE).as_deref(),
            Some(shown.as_str())
        );
    }
    assert_eq!(
        misread_timestamp_logical(None),
        None,
        "no annotation is the lake's plain micros"
    );
}
