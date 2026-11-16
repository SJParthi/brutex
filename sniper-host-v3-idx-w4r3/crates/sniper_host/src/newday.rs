//! The NEW-DAY path (S9, binding; SNIPER s11 (3): "A new trading day is added incrementally: its rows and sums, never a full
//! re-run"): the day is folded from the carried CONT states (`common_feat::series::Series` kept by the caller across days), its
//! outcome partition is priced (`price`) and APPENDED as a new partition (written once, never over an earlier one), and only the
//! test / paper sums of the shown picks move, in O(that day's trades). Picks never change: their learning months are closed (every
//! new day falls in a test or paper month). No sweep, shortlist or twin is re-run.

use std::collections::BTreeMap;

use crate::db::Db;

/// The test / paper sums of one shown pick (additive: O(1) a trade). On the `common_wf` link these are `Acc::add` calls.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TestSums {
    /// Net per calendar year, paisa.
    pub year_net: BTreeMap<i32, i64>,
    /// Trades per calendar year.
    pub year_trades: BTreeMap<i32, u32>,
    /// Net per month key.
    pub month_net: BTreeMap<i32, i64>,
}

impl TestSums {
    /// Adds one trade.
    ///
    /// # Errors
    /// A sum that leaves i64.
    pub fn add(&mut self, mk: i32, net: i64) -> Result<(), String> {
        let y = mk.div_euclid(12);
        let e = self.year_net.entry(y).or_insert(0);
        *e = e.checked_add(net).ok_or("overflow")?;
        *self.year_trades.entry(y).or_insert(0) += 1;
        let m = self.month_net.entry(mk).or_insert(0);
        *m = m.checked_add(net).ok_or("overflow")?;
        Ok(())
    }
}

/// Appends one day's priced rows (fixed-width records) as a new partition `name`, written once with its audit line.
///
/// # Errors
/// A partition that exists, a record of another width, or an I/O error.
pub fn append_day(db: &Db, name: &str, width: usize, records: &[Vec<u8>]) -> Result<String, String> {
    let mut b = Vec::with_capacity(records.len() * width);
    for r in records {
        if r.len() != width {
            return Err(format!("{name}: a record of {} bytes, the partition's width is {width}", r.len()));
        }
        b.extend_from_slice(r);
    }
    db.write_bin(name, &b)
}
