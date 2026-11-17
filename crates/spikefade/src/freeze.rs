//! v3.2 (MAIN-RULINGS-ENGINE-v22 addendum 10, checklist item 12): NSE's DATED quantity-freeze limits, the order slices they force, read
//! from NSE's own files (each date's limit = the latest NSE publication effective on or before that session; never a later file for an
//! earlier day, except the labelled stock reading below).
//!
//! Sources (`run3 --freeze <file>`, repeatable):
//! - INDEX (NIFTY, BANKNIFTY): the dated circular table of the freeze lane (`state/nse-freeze-20261114/FREEZE-TABLE-NIFTY-BANKNIFTY.tsv`:
//!   one row per change from NSE's monthly FAOP circulars 2018-08-01 .. 2026-11-02, each with its circular number, URL and sha256).
//! - STOCKS: NSE's quantity freeze file `qtyfreeze.csv` (nsearchives /content/fo/), given as `<file>@<effective session YYYY-MM-DD>`: the
//!   file held is Last-Modified 2026-10-30 13:45:03 GMT (19:15 IST), so it serves the sessions from 2026-11-02. NSE keeps no dated archive
//!   of the stock file (the old archives URL answers 404; FAOP61157 discontinued the report, FAOP65435 restored qtyfreeze.csv from
//!   2024-12-09), so a stock session BEFORE the first dated stock file reads the earliest stock file held, LABELLED "undated" and counted
//!   (a gap is never filled silently).
//!
//! Slicing: NSE rejects an order ABOVE the freeze quantity (the freeze lane's reading), so one order carries at most floor(freeze / lot)
//! whole lots: slice = floor(freeze / lot) x lot (one lot when a lot alone is above the freeze quantity, counted).

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::cal::parse_day;
use crate::cube::Cube;

/// One dated limit: (effective session day, freeze units, source index).
type Lim = (i64, u64, u16);

/// The dated freeze limits of every symbol.
#[derive(Clone, Debug, Default)]
pub struct Freeze {
    by: HashMap<String, Vec<Lim>>,
    /// Source descriptions (file, sha256, rows), printed in every run log.
    pub sources: Vec<String>,
}

/// The run's table (set once from the command line).
pub static FREEZE: OnceLock<Freeze> = OnceLock::new();
/// Lookups that found a dated limit; lookups on a stock session before the first dated stock file (undated reading); lookups with no
/// limit at all (one order a block); lots alone above the freeze quantity.
pub static DATED: AtomicU64 = AtomicU64::new(0);
pub static UNDATED: AtomicU64 = AtomicU64::new(0);
pub static NONE: AtomicU64 = AtomicU64::new(0);
pub static LOT_ABOVE: AtomicU64 = AtomicU64::new(0);

impl Freeze {
    /// Parses one source: the circular table (tab-separated, header `index\teffective_from\tfreeze_qty_units...`) or `qtyfreeze.csv@DATE`.
    ///
    /// # Errors
    /// An unreadable file or a row that is not the documented format.
    pub fn add(&mut self, spec: &str) -> Result<(), String> {
        let (path, eff) = match spec.rsplit_once('@') {
            Some((p, d)) if parse_day(d).is_some() => (p, parse_day(d)),
            _ => (spec, None),
        };
        let p = Path::new(path);
        let text = std::fs::read_to_string(p).map_err(|e| format!("--freeze {path}: {e}"))?;
        let sha = crate::cube::sha256(p);
        let src = u16::try_from(self.sources.len()).map_err(|_| "freeze sources")?;
        let mut n = 0_usize;
        let mut lines = text.lines();
        let head = lines.next().unwrap_or("");
        if head.starts_with("index\teffective_from\tfreeze_qty_units") {
            for l in lines {
                let f: Vec<&str> = l.split('\t').collect();
                let (Some(sym), Some(d), Some(q)) = (f.first(), f.get(1).and_then(|x| parse_day(x)), f.get(2).and_then(|x| x.trim().parse::<u64>().ok())) else {
                    return Err(format!("--freeze {path}: bad row {l}"));
                };
                self.by.entry((*sym).trim().to_owned()).or_default().push((d, q, src));
                n += 1;
            }
            self.sources.push(format!("index circular table {path} (sha256 {sha}, {n} dated rows)"));
        } else if head.contains("VOL_FRZ_QTY") {
            let Some(day) = eff else { return Err(format!("--freeze {path}: NSE's qtyfreeze.csv needs @<effective session YYYY-MM-DD> (its Last-Modified)")) };
            for l in lines {
                let f: Vec<&str> = l.split(',').map(str::trim).collect();
                let (Some(sym), Some(q)) = (f.get(1), f.get(2).and_then(|x| x.parse::<u64>().ok())) else {
                    if l.trim().is_empty() {
                        continue;
                    }
                    return Err(format!("--freeze {path}: bad row {l}"));
                };
                // the index rows of the stock file are left to the dated circular table (both say NIFTY 1800 / BANKNIFTY 900 on 2026-11-02)
                if matches!(*sym, "NIFTY" | "BANKNIFTY") && self.by.contains_key(*sym) {
                    continue;
                }
                self.by.entry((*sym).to_owned()).or_default().push((day, q, src));
                n += 1;
            }
            self.sources.push(format!("NSE qtyfreeze.csv {path} (sha256 {sha}, {n} symbols) effective from {}", crate::cal::fmt_day(day)));
        } else {
            return Err(format!("--freeze {path}: neither the circular table nor NSE's qtyfreeze.csv"));
        }
        for v in self.by.values_mut() {
            v.sort_unstable();
        }
        Ok(())
    }

    /// The freeze limit of `sym` on session `day`: (units, dated). Dated = the latest row effective on or before `day`; else the earliest row
    /// (undated: a session before the first file held, labelled); None = no row at all.
    #[must_use]
    pub fn units(&self, sym: &str, day: i64) -> Option<(u64, bool)> {
        let v = self.by.get(sym)?;
        // MUTANT m_v32_freeze_undated: the latest file read for every day (a later file for an earlier session)
        if cfg!(feature = "m_v32_freeze_undated") {
            return v.last().map(|x| (x.1, true));
        }
        let i = v.partition_point(|x| x.0 <= day);
        if i > 0 { v.get(i - 1).map(|x| (x.1, true)) } else { v.first().map(|x| (x.1, false)) }
    }
}

/// The units one order of a lot of `lot` units may carry under freeze `f`: floor(f / lot) x lot (a lot when the lot alone is above it).
#[must_use]
pub fn slice_of(f: u64, lot: u64) -> u64 {
    let lot = lot.max(1);
    let s = f / lot * lot;
    if s == 0 {
        LOT_ABOVE.fetch_add(1, Ordering::Relaxed);
        lot
    } else {
        s
    }
}

/// The order slice (units) of contract `ci` on its cube's day (0 = no limit known: one order a block, counted).
#[must_use]
pub fn slice_units(c: &Cube, ci: usize) -> u64 {
    let Some(k) = c.cons.get(ci) else { return 0 };
    let Some(t) = FREEZE.get() else { return 0 };
    let name = c.unds.get(usize::from(k.und)).map_or("", |u| u.name.as_str());
    // REPAIR A: the counters count DISTINCT contract-days (a fact of the run's data), not repeated lookups (a work count that differs when a
    // day-major run prices an event in several batches, or a resumed run once: proof XCKPT)
    let first = !crate::sw::any_ra() || SEEN.lock().map_or(true, |mut g| g.insert((c.day, k.sid)));
    match t.units(name, c.day) {
        Some((f, dated)) => {
            if !first {
            } else if dated {
                DATED.fetch_add(1, Ordering::Relaxed);
            } else {
                UNDATED.fetch_add(1, Ordering::Relaxed);
            }
            if first { slice_of(f, u64::from(k.lot)) } else { (f / u64::from(k.lot.max(1)) * u64::from(k.lot.max(1))).max(u64::from(k.lot.max(1))) }
        }
        None => {
            if first {
                NONE.fetch_add(1, Ordering::Relaxed);
            }
            0
        }
    }
}

/// REPAIR A: the contract-days already counted.
static SEEN: std::sync::Mutex<std::collections::BTreeSet<(i64, u32)>> = std::sync::Mutex::new(std::collections::BTreeSet::new());

/// Loads every `--freeze` source of the command line into [`FREEZE`].
///
/// # Errors
/// A refused source.
pub fn from_args(a: &[String]) -> Result<Option<String>, String> {
    let specs: Vec<&String> = a.iter().enumerate().filter(|(_, x)| *x == "--freeze").filter_map(|(i, _)| a.get(i + 1)).collect();
    if specs.is_empty() {
        return Ok(None);
    }
    let mut f = Freeze::default();
    // the circular table first (its index rows win over the stock file's)
    let mut ord: Vec<&String> = specs.clone();
    ord.sort_by_key(|s| u8::from(s.contains("qtyfreeze")));
    for s in ord {
        f.add(s)?;
    }
    let words = f.sources.join("; ");
    let _ = FREEZE.set(f);
    Ok(Some(words))
}

/// The lookup counters as printed.
#[must_use]
pub fn counts_line() -> String {
    format!("freeze lookups{}: dated {} / UNDATED (stock session before the first dated stock file: the earliest file held, labelled) {} / no limit known (one order a block) {}; a lot alone above the freeze quantity {}", if crate::sw::any_ra() { " (distinct contract-days)" } else { "" }, DATED.load(Ordering::Relaxed), UNDATED.load(Ordering::Relaxed), NONE.load(Ordering::Relaxed), LOT_ABOVE.load(Ordering::Relaxed))
}
