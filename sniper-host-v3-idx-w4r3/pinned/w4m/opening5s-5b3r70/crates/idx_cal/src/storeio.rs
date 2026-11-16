//! The store's read-only door, COPIED from `scratch/sniper-opt-beside/crates/opt_beside/src/chain.rs` (`sym_id`,
//! `MonthFile`, `open_month`, `read_range`; itself copied from the opening robot's `store_io.rs`): every month through
//! `store::file::BarFile::open_existing` (generation-aware: a promoted month is served from `bar-revisions-v1/<n>/`), with
//! the quarantine probe. Plus small helpers for IST days. Read only.

use std::collections::BTreeMap;
use std::path::Path;

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::file::{BarFile, StoreError};
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::quarantine::Probe;

use crate::day::{self, Day};

/// Microseconds in a second.
pub const US: i64 = 1_000_000;

/// The UTC microsecond of `sod` seconds after IST midnight of day `d`.
#[must_use]
pub fn at_sod(d: Day, sod: i64) -> i64 {
    (i64::from(d) * 86_400 - 19_800 + sod) * US
}

/// The IST day and second of day of a UTC microsecond stamp.
#[must_use]
pub fn ist_of(ts: i64) -> (Day, i64) {
    let s = ts.div_euclid(US) + 19_800;
    (Day::try_from(s.div_euclid(86_400)).unwrap_or(0), s.rem_euclid(86_400))
}

/// `HH:MM:SS` of a second of day.
#[must_use]
pub fn hms(sod: i64) -> String {
    format!("{:02}:{:02}:{:02}", sod / 3600, (sod / 60) % 60, sod % 60)
}

/// The symbol id the store's headers carry (FNV-1a, low 32 bits). COPIED.
#[must_use]
pub fn sym_id(series: &str) -> u32 {
    u32::try_from(brutex_core::universe::fnv1a(series) & 0xFFFF_FFFF).unwrap_or(0)
}

/// One month's door. COPIED.
#[allow(clippy::large_enum_variant, reason = "copied verbatim (one month file open at a time)")]
pub enum MonthFile {
    /// No such month.
    Missing,
    /// Refused, with the reason.
    Refused(String),
    /// Open.
    Open(BarFile),
}

/// Opens one stored month read-only (`contract` for an option contract directory). COPIED.
#[allow(clippy::too_many_arguments, reason = "one path")]
#[must_use]
pub fn open_month(root: &Path, vendor: Vendor, segment: &str, symbol: &str, contract: Option<&str>, tf: Timeframe, y: u16, m: u8) -> MonthFile {
    let month = match YearMonth::new(y, m) {
        Ok(v) => v,
        Err(e) => return MonthFile::Refused(e.to_string()),
    };
    let c = match contract {
        None => None,
        Some(text) => match Contract::parse(text) {
            Some(c) => Some(c),
            None => return MonthFile::Refused(format!("contract {text} does not parse")),
        },
    };
    let path = match StorePath::new(PathParts { vendor, exchange: "NSE", segment, symbol, contract: c, timeframe: tf, month, file: FileKind::Bars }) {
        Ok(p) => p,
        Err(e) => return MonthFile::Refused(e.to_string()),
    };
    match store::quarantine::probe(root, path) {
        Ok(Probe::Clear) => {}
        Ok(Probe::Quarantined(_)) => return MonthFile::Refused(format!("{symbol} {y}-{m:02} is quarantined")),
        Err(e) => return MonthFile::Refused(format!("{symbol} {y}-{m:02} quarantine unknown: {e}")),
    }
    match BarFile::open_existing(root, path, sym_id(symbol)) {
        Ok(f) => MonthFile::Open(f),
        Err(StoreError::Missing { .. }) => MonthFile::Missing,
        Err(e) => MonthFile::Refused(format!("{symbol} {y}-{m:02}: {e}")),
    }
}

/// The records stamped in `[from, to)`, read through `.tix`. COPIED.
///
/// # Errors
/// A store read error.
pub fn read_range(f: &BarFile, from: i64, to: i64) -> Result<Vec<Bar>, String> {
    let a = f.first_at_or_after(from).map_err(|e| e.to_string())?;
    let b = f.first_at_or_after(to).map_err(|e| e.to_string())?;
    let mut v = Vec::with_capacity(usize::try_from(b.saturating_sub(a)).unwrap_or(0));
    for i in a..b {
        v.push(f.read_record(i).map_err(|e| e.to_string())?);
    }
    Ok(v)
}

/// The `(year, month)` of a day as the store's month key.
#[must_use]
pub fn ym_of(d: Day) -> (u16, u8) {
    let (y, m) = day::ym(d);
    (u16::try_from(y).unwrap_or(0), u8::try_from(m).unwrap_or(0))
}

/// Reads the bars of one series on day `d` in `[from_sod, to_sod)`. `Ok(None)` when the month file is missing.
///
/// # Errors
/// A refused month or a read error.
#[allow(clippy::too_many_arguments, reason = "one path")]
pub fn day_bars(root: &Path, vendor: Vendor, segment: &str, symbol: &str, contract: Option<&str>, tf: Timeframe, d: Day, from_sod: i64, to_sod: i64) -> Result<Option<Vec<Bar>>, String> {
    let (y, m) = ym_of(d);
    match open_month(root, vendor, segment, symbol, contract, tf, y, m) {
        MonthFile::Missing => Ok(None),
        MonthFile::Refused(e) => Err(e),
        MonthFile::Open(f) => Ok(Some(read_range(&f, at_sod(d, from_sod), at_sod(d, to_sod))?)),
    }
}

/// (symbol, contract, tf seconds, year, month).
type MonthKey = (String, String, u32, u16, u8);

/// Expiry -> [CE strikes, PE strikes].
pub type ContractMap = BTreeMap<Day, [Vec<i64>; 2]>;

/// A whole month of one series, cached by `(symbol, contract, tf, y, m)`.
#[derive(Default)]
pub struct MonthCache {
    map: BTreeMap<MonthKey, Result<Vec<Bar>, String>>,
}

impl MonthCache {
    /// The month's bars (`Ok(empty)` when missing).
    #[allow(clippy::too_many_arguments, reason = "one path")]
    pub fn month(&mut self, root: &Path, vendor: Vendor, segment: &str, symbol: &str, contract: Option<&str>, tf: Timeframe, y: u16, m: u8) -> &Result<Vec<Bar>, String> {
        let key = (symbol.to_string(), contract.unwrap_or("").to_string(), tf.secs(), y, m);
        self.map.entry(key).or_insert_with(|| match open_month(root, vendor, segment, symbol, contract, tf, y, m) {
            MonthFile::Missing => Ok(Vec::new()),
            MonthFile::Refused(e) => Err(e),
            MonthFile::Open(f) => {
                let n = f.records();
                let mut v = Vec::with_capacity(usize::try_from(n).unwrap_or(0));
                for i in 0..n {
                    match f.read_record(i) {
                        Ok(b) => v.push(b),
                        Err(e) => return Err(format!("{symbol} {y}-{m:02} record {i}: {e}")),
                    }
                }
                Ok(v)
            }
        })
    }

    /// Drops every cached month (the caller bounds memory month by month).
    pub fn clear(&mut self) {
        self.map.clear();
    }
}

/// The bars of a cached month inside `[from, to)` (UTC micros), by binary search.
#[must_use]
pub fn slice(bars: &[Bar], from: i64, to: i64) -> &[Bar] {
    let a = bars.partition_point(|b| b.ts_micros < from);
    let b = bars.partition_point(|b| b.ts_micros < to);
    bars.get(a..b).unwrap_or(&[])
}

/// The option contract folder name of the store: `<expiry>-<strike paisa>-<CE|PE>`.
#[must_use]
pub fn contract_name(expiry: Day, strike: i64, ce: bool) -> String {
    format!("{}-{strike}-{}", day::fmt(expiry), if ce { "CE" } else { "PE" })
}

/// Lists `bars/gdfl/NSE/FNO/<index>` (directory names only): expiry -> (CE strikes, PE strikes), plus names that do not
/// parse.
///
/// # Errors
/// The directory cannot be read.
pub fn list_contracts(root: &Path, index: &str) -> Result<(ContractMap, Vec<String>), String> {
    let dir = root.join("bars").join("gdfl").join("NSE").join("FNO").join(index);
    let mut out: BTreeMap<Day, [Vec<i64>; 2]> = BTreeMap::new();
    let mut odd = Vec::new();
    for e in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let e = e.map_err(|e| e.to_string())?;
        let name = e.file_name().to_string_lossy().into_owned();
        let parsed = (|| {
            let d = day::parse(name.get(0..10)?)?;
            let rest = name.get(11..)?;
            let (k, side) = rest.rsplit_once('-')?;
            let k: i64 = k.parse().ok()?;
            let ce = match side {
                "CE" => true,
                "PE" => false,
                _ => return None,
            };
            Some((d, k, ce))
        })();
        match parsed {
            Some((d, k, ce)) => {
                let slot = out.entry(d).or_default();
                if let Some(v) = slot.get_mut(usize::from(!ce)) {
                    v.push(k);
                }
            }
            None => odd.push(name),
        }
    }
    for v in out.values_mut() {
        for s in v.iter_mut() {
            s.sort_unstable();
        }
    }
    odd.sort();
    Ok((out, odd))
}
