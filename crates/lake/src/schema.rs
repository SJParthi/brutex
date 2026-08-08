//! The two column layouts the lake actually contains, and the refusal for
//! anything else.
//!
//! Verified by reading the thrift footers, not by trusting a brief:
//!
//! * **Cash / index**, 7 columns — `timestamp, open, high, low, close, volume,
//!   open_interest`. Found under `bars/<EX>/CASH/**` and `bars/<EX>/INDEX/**`.
//! * **F&O**, 17 columns — the seven above, in the same order, followed by
//!   `iv, delta, gamma, theta, vega, rho, spot_at_bar, t_years_used,
//!   rate_used, greeks_provenance_id`.
//!
//! The cash layout is a strict prefix of the F&O one, which is why a single
//! ordered specification covers both.
//!
//! Two facts here are easy to get wrong and are pinned by tests:
//! `greeks_provenance_id` is **`INT32`**, not `INT64`; and **every** column is
//! `OPTIONAL`, so every column carries definition levels and any of them could
//! in principle be null.

use parquet::basic::Type as PhysicalType;
use parquet::schema::types::SchemaDescriptor;

use crate::error::{ColumnType, LakeError};

/// A column this reader requires, and the physical type it must have.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ColumnSpec {
    /// The column name as the lake writes it.
    pub(crate) name: &'static str,
    /// The physical type it must hold.
    pub(crate) ty: ColumnType,
}

const fn spec(name: &'static str, ty: ColumnType) -> ColumnSpec {
    ColumnSpec { name, ty }
}

/// The seven columns every lake bar file carries, in file order.
pub(crate) const CASH_SPEC: [ColumnSpec; 7] = [
    spec("timestamp", ColumnType::Int64),
    spec("open", ColumnType::Double),
    spec("high", ColumnType::Double),
    spec("low", ColumnType::Double),
    spec("close", ColumnType::Double),
    spec("volume", ColumnType::Int64),
    spec("open_interest", ColumnType::Int64),
];

/// The ten further columns an F&O bar file carries, in file order.
pub(crate) const FNO_EXTRA_SPEC: [ColumnSpec; 10] = [
    spec("iv", ColumnType::Double),
    spec("delta", ColumnType::Double),
    spec("gamma", ColumnType::Double),
    spec("theta", ColumnType::Double),
    spec("vega", ColumnType::Double),
    spec("rho", ColumnType::Double),
    spec("spot_at_bar", ColumnType::Double),
    spec("t_years_used", ColumnType::Double),
    spec("rate_used", ColumnType::Double),
    // INT32, not INT64. Reading this as an i64 decodes garbage.
    spec("greeks_provenance_id", ColumnType::Int32),
];

/// Which of the two shapes a file has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layout {
    /// A 7-column cash or index bar file. No greeks.
    Cash,
    /// A 17-column F&O bar file, carrying the greeks block.
    Fno,
}

impl Layout {
    /// How many leaf columns this layout has.
    #[must_use]
    pub const fn column_count(self) -> usize {
        match self {
            Self::Cash => CASH_SPEC.len(),
            Self::Fno => CASH_SPEC.len() + FNO_EXTRA_SPEC.len(),
        }
    }

    /// Whether files of this layout carry the greeks columns.
    #[must_use]
    pub const fn has_greeks(self) -> bool {
        matches!(self, Self::Fno)
    }
}

/// Maps a parquet physical type onto the small reporting enum.
pub(crate) const fn physical(t: PhysicalType) -> ColumnType {
    match t {
        PhysicalType::INT32 => ColumnType::Int32,
        PhysicalType::INT64 => ColumnType::Int64,
        PhysicalType::DOUBLE => ColumnType::Double,
        _ => ColumnType::Other,
    }
}

/// Decides which layout a file has, refusing anything that is neither.
///
/// The column *count* selects the candidate layout and every column is then
/// checked by name and by type, so a file with the right number of columns
/// under different names is refused rather than read positionally.
pub(crate) fn detect(schema: &SchemaDescriptor) -> Result<Layout, LakeError> {
    let n = schema.num_columns();
    let layout = if n == Layout::Cash.column_count() {
        Layout::Cash
    } else if n == Layout::Fno.column_count() {
        Layout::Fno
    } else {
        return Err(LakeError::UnexpectedSchema {
            columns: n,
            names: names(schema),
        });
    };

    for (i, s) in specs(layout).enumerate() {
        if i >= n {
            return Err(LakeError::MissingColumn { name: s.name });
        }
        // `column(i)` hands back an owning pointer, so the name is read out of
        // it before it is dropped rather than borrowed past its life.
        let col = schema.column(i);
        let got = physical(col.physical_type());
        if col.name() != s.name {
            // The count matched but the names do not: this is a different
            // schema wearing the right size, and reading it positionally would
            // put a rho where a volume belongs.
            return Err(LakeError::UnexpectedSchema {
                columns: n,
                names: names(schema),
            });
        }
        if got != s.ty {
            return Err(LakeError::ColumnTypeMismatch {
                name: s.name,
                want: s.ty,
                got,
            });
        }
    }
    Ok(layout)
}

/// The ordered specification for a layout.
pub(crate) fn specs(layout: Layout) -> impl Iterator<Item = ColumnSpec> {
    let extra = match layout {
        Layout::Cash => &FNO_EXTRA_SPEC[0..0],
        Layout::Fno => &FNO_EXTRA_SPEC[..],
    };
    CASH_SPEC.into_iter().chain(extra.iter().copied())
}

/// Every leaf column name, for an error message.
fn names(schema: &SchemaDescriptor) -> Vec<String> {
    (0..schema.num_columns())
        .map(|i| schema.column(i).name().to_owned())
        .collect()
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

    #[test]
    fn the_two_layouts_have_the_sizes_the_lake_has() {
        assert_eq!(Layout::Cash.column_count(), 7);
        assert_eq!(Layout::Fno.column_count(), 17);
        assert!(!Layout::Cash.has_greeks());
        assert!(Layout::Fno.has_greeks());
    }

    #[test]
    fn cash_is_a_strict_prefix_of_fno() {
        let cash: Vec<&str> = specs(Layout::Cash).map(|s| s.name).collect();
        let fno: Vec<&str> = specs(Layout::Fno).map(|s| s.name).collect();
        assert_eq!(cash.len(), 7);
        assert_eq!(fno.len(), 17);
        assert_eq!(
            &fno[..7],
            &cash[..],
            "the cash columns must lead the F&O ones"
        );
    }

    #[test]
    fn provenance_id_is_int32_not_int64() {
        // Getting this wrong decodes garbage rather than failing, so it is
        // pinned here as well as being checked at read time.
        let s = FNO_EXTRA_SPEC
            .iter()
            .find(|s| s.name == "greeks_provenance_id")
            .expect("present");
        assert_eq!(s.ty, ColumnType::Int32);
    }

    #[test]
    fn the_greeks_block_is_all_double() {
        for name in [
            "iv",
            "delta",
            "gamma",
            "theta",
            "vega",
            "rho",
            "spot_at_bar",
            "t_years_used",
            "rate_used",
        ] {
            let s = FNO_EXTRA_SPEC.iter().find(|s| s.name == name).unwrap();
            assert_eq!(s.ty, ColumnType::Double, "{name}");
        }
    }

    #[test]
    fn physical_types_map_and_anything_else_is_other() {
        assert_eq!(physical(PhysicalType::INT32), ColumnType::Int32);
        assert_eq!(physical(PhysicalType::INT64), ColumnType::Int64);
        assert_eq!(physical(PhysicalType::DOUBLE), ColumnType::Double);
        assert_eq!(physical(PhysicalType::BOOLEAN), ColumnType::Other);
        assert_eq!(physical(PhysicalType::FLOAT), ColumnType::Other);
        assert_eq!(physical(PhysicalType::BYTE_ARRAY), ColumnType::Other);
        assert_eq!(physical(PhysicalType::INT96), ColumnType::Other);
        assert_eq!(
            physical(PhysicalType::FIXED_LEN_BYTE_ARRAY),
            ColumnType::Other
        );
    }

    #[test]
    fn column_type_renders_its_name() {
        assert_eq!(ColumnType::Int32.to_string(), "INT32");
        assert_eq!(ColumnType::Int64.to_string(), "INT64");
        assert_eq!(ColumnType::Double.to_string(), "DOUBLE");
        assert_eq!(
            ColumnType::Other.to_string(),
            "an unsupported physical type"
        );
    }
}
