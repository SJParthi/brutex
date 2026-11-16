//! The dated NSE freeze table (`state/nse-freeze-20261114/FREEZE-TABLE-NIFTY-BANKNIFTY.tsv`, sha16 1536c39bc16f4e38; PASSED
//! its independent check, MAIN 12:06 note) and the one-order maximum.

use std::collections::BTreeMap;

use idx_cal::day::{self, Day};

/// One change row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreezeRow {
    /// Effective from (inclusive).
    pub from: Day,
    /// Freeze quantity, units.
    pub units: i64,
    /// The circular's date (1-4 days before `from`).
    pub circular_date: Day,
    /// The circular's number.
    pub circular: String,
}

/// The table per index, rows ascending by `from`.
#[derive(Clone, Debug, Default)]
pub struct FreezeTable {
    rows: BTreeMap<String, Vec<FreezeRow>>,
}

impl FreezeTable {
    /// Parses the TSV (header `index effective_from freeze_qty_units ... circular_number circular_date ...`).
    ///
    /// # Errors
    /// A missing column or a malformed row.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        let head: Vec<&str> = lines.next().ok_or("empty freeze table")?.split('\t').collect();
        let col = |n: &str| head.iter().position(|h| *h == n).ok_or_else(|| format!("freeze table: no column {n}"));
        let (ci, cf, cu, cn, cd) = (col("index")?, col("effective_from")?, col("freeze_qty_units")?, col("circular_number")?, col("circular_date")?);
        let mut rows: BTreeMap<String, Vec<FreezeRow>> = BTreeMap::new();
        for (i, l) in lines.enumerate() {
            if l.trim().is_empty() {
                continue;
            }
            let c: Vec<&str> = l.split('\t').collect();
            let g = |k: usize| c.get(k).copied().unwrap_or("");
            let from = day::parse(g(cf)).ok_or_else(|| format!("freeze row {}: from", i + 2))?;
            let units: i64 = g(cu).trim().parse().map_err(|_| format!("freeze row {}: units", i + 2))?;
            let circular_date = day::parse(g(cd)).ok_or_else(|| format!("freeze row {}: circular date", i + 2))?;
            rows.entry(g(ci).to_string()).or_default().push(FreezeRow { from, units, circular_date, circular: g(cn).to_string() });
        }
        for v in rows.values_mut() {
            v.sort_by_key(|r| r.from);
        }
        Ok(Self { rows })
    }

    /// The freeze quantity in force on day `d` (None before the first row: refused by name, F13).
    #[must_use]
    pub fn units(&self, index: &str, d: Day) -> Option<i64> {
        self.rows.get(index)?.iter().take_while(|r| r.from <= d).last().map(|r| r.units)
    }

    /// The one-order maximum on day `d` for a lot: floor(freeze / lot) x lot (12:06 note; e.g. 1,800 / 65 = 27 lots = 1,755).
    #[must_use]
    pub fn max_one_order(&self, index: &str, d: Day, lot: i64) -> Option<i64> {
        let f = self.units(index, d)?;
        (lot > 0).then(|| (f / lot) * lot)
    }

    /// `fit_strict` (Q38, beside): the smallest freeze in force on any day of `[d, until]`, using only the rows whose
    /// circular is DATED BEFORE `d` (no future circular is seen).
    #[must_use]
    pub fn strict_min(&self, index: &str, d: Day, until: Day) -> Option<i64> {
        let known: Vec<&FreezeRow> = self.rows.get(index)?.iter().filter(|r| r.circular_date < d || r.from <= d).collect();
        let at_d = known.iter().take_while(|r| r.from <= d).last()?.units;
        Some(known.iter().filter(|r| r.from > d && r.from <= until).map(|r| r.units).fold(at_d, i64::min))
    }

    /// The change rows of an index.
    #[must_use]
    pub fn rows(&self, index: &str) -> &[FreezeRow] {
        self.rows.get(index).map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> FreezeTable {
        let text = "index\teffective_from\tfreeze_qty_units\tcircular_number\tcircular_date\nNIFTY\t2022-08-01\t2800\tA\t2022-07-29\nNIFTY\t2022-09-01\t1800\tFAOP53491\t2022-08-30\nNIFTY\t2026-10-05\t3510\tB\t2026-10-01\nBANKNIFTY\t2025-05-02\t600\tC\t2025-04-30\n";
        FreezeTable::parse(text).unwrap_or_default()
    }

    /// The 12:06 note's example, the cut on an expiry day (NIFTY 2022-09-01) and R2's lot that does not divide the freeze
    /// (BANKNIFTY lot 35 at freeze 900 -> 875).
    #[test]
    fn one_order_maximum_and_dated_rows() {
        let f = t();
        let d = |s: &str| day::parse(s).unwrap_or(0);
        assert_eq!(f.units("NIFTY", d("2022-08-31")), Some(2_800));
        assert_eq!(f.units("NIFTY", d("2022-09-01")), Some(1_800));
        assert_eq!(f.max_one_order("NIFTY", d("2026-10-05"), 65), Some(3_510));
        assert_eq!(f.max_one_order("NIFTY", d("2022-09-01"), 65), Some(1_755));
        assert_eq!(f.units("NIFTY", d("2022-07-31")), None, "a day before the table is refused by name");
        let b = FreezeTable::parse("index\teffective_from\tfreeze_qty_units\tcircular_number\tcircular_date\nBANKNIFTY\t2024-01-01\t900\tX\t2023-12-29\n").unwrap_or_default();
        assert_eq!(b.max_one_order("BANKNIFTY", d("2024-06-01"), 35), Some(875));
        // fit_strict on 2022-08-25 sees no cut: the 09-01 circular is dated 08-30, after the decision (s4.2 (10)).
        assert_eq!(f.strict_min("NIFTY", d("2022-08-25"), d("2022-09-01")), Some(2_800));
        assert_eq!(f.strict_min("NIFTY", d("2022-08-31"), d("2022-09-08")), Some(1_800));
    }
}
