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
    pub(crate) by: HashMap<String, Vec<Lim>>,
    /// Source descriptions (file, sha256, rows), printed in every run log.
    pub sources: Vec<String>,
    /// integrate:D F3 (switch `int34::F3D`): data:F3's dated stock table, read exactly (None without the switch or the file).
    pub stock: Option<StockDated>,
}

/// integrate:D F3 (data:F3 `F3-STOCK-FREEZE.md` 6.1): the EXACT 16-column header of `STOCK-FREEZE-DATED.tsv`.
pub const F3_HEADER: &str = "index\teffective_from\tfreeze_qty_units\tkind\tsource_file\tsource_sha256\trule_circular\tmwpl_min\tmwpl_min_session\thighest_strike_max\thighest_strike_session\tnotional_bound_rs\twindow_from\tsplit_bonus_cut\tbound_at_rule_notional_units\tnse_symbol_of_session";
/// F3 row kinds (data:F3 section 0): NSE's own published limit, the proven conservative bound, the gap bound.
pub const F3_KINDS: [&str; 3] = ["SOURCE", "RULE-BOUND", "GAP-BOUND"];

/// One (stock, session) row of data:F3's table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StockRow {
    /// The limit in shares: NSE rejects an order of MORE than this.
    pub units: u64,
    /// 0 SOURCE, 1 RULE-BOUND, 2 GAP-BOUND.
    pub kind: u8,
    /// The row repeats another NSE symbol's row of the same company (col 16 != col 1).
    pub alias: bool,
    /// The source file's index in [`StockDated::files`].
    pub file: u32,
}

/// integrate:D F3: data:F3's dated stock table: (cube underlying name, session) -> its row; O(1) (two hash reads).
#[derive(Clone, Debug, Default)]
pub struct StockDated {
    pub rows: HashMap<String, HashMap<i64, StockRow>>,
    /// The distinct source file names (SOURCE: NSE's contract file; bounds: the binding bhavcopy / MWPL file).
    pub files: Vec<String>,
    /// The first and last session the table covers (a stock session outside reads the earlier sources, as before).
    pub first: i64,
    pub last: i64,
    pub path: String,
    pub sha: String,
    /// Rows per kind, alias rows, rows.
    pub kinds: [u64; 3],
    pub aliases: u64,
    /// The NSE symbol of the session of every ALIAS row ((cube name, session) -> NSE symbol: col 16): data:F7's band files and NSE's other
    /// files name a renamed company by its symbol OF THAT SESSION.
    pub nse_of: HashMap<(String, i64), String>,
    pub n: u64,
}

impl StockDated {
    /// Parses the table STRICTLY (data:F3 section 7 item 2): the exact header; 16 fields a line; col 2 a session; col 3 a positive integer;
    /// col 4 a kind; col 6 64 lowercase hex; col 16 non-empty; (col 1, col 2) unique. Any other line refuses the run with its line number.
    ///
    /// # Errors
    /// The header or a line that is not the format.
    pub fn parse(text: &str, path: &str, sha: &str) -> Result<Self, String> {
        let mut lines = text.lines().enumerate();
        let head = lines.next().map_or("", |x| x.1);
        if head != F3_HEADER {
            return Err(format!("--freeze {path}: not data:F3's 16-column header"));
        }
        let mut s = Self { path: path.to_owned(), sha: sha.to_owned(), first: i64::MAX, last: i64::MIN, ..Self::default() };
        let mut fidx: HashMap<String, u32> = HashMap::new();
        for (i, l) in lines {
            let f: Vec<&str> = l.split('\t').collect();
            let bad = |why: &str| format!("--freeze {path}: line {}: {why}: refused", i + 1);
            // MUTANT m_v34i_f3_fields_unchecked: a short or malformed line read as far as it goes
            let strict = !cfg!(feature = "m_v34i_f3_fields_unchecked");
            if f.len() != 16 && strict {
                return Err(bad(&format!("{} fields, not 16", f.len())));
            }
            let g = |k: usize| f.get(k).copied().unwrap_or("");
            let sym = g(0);
            let day = parse_day(g(1)).ok_or_else(|| bad("col 2 is not a session YYYY-MM-DD"))?;
            let units = g(2).parse::<u64>().ok().filter(|&u| u > 0 || !strict).ok_or_else(|| bad("col 3 is not a positive integer"))?;
            let kind = F3_KINDS.iter().position(|&k| k == g(3)).ok_or_else(|| bad("col 4 is not SOURCE / RULE-BOUND / GAP-BOUND"))?;
            let h = g(5);
            if strict && (h.len() != 64 || !h.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))) {
                return Err(bad("col 6 is not 64 lowercase hex"));
            }
            let nse = g(15);
            if strict && (sym.is_empty() || nse.is_empty()) {
                return Err(bad("col 1 or col 16 empty"));
            }
            let nf = u32::try_from(fidx.len()).map_err(|_| "files")?;
            let file = *fidx.entry(g(4).to_owned()).or_insert(nf);
            // MUTANT m_v34i_f3_units_plus_one: NSE's MaxTradQty (the limit + 1) read as the limit
            let units = if cfg!(feature = "m_v34i_f3_units_plus_one") { units + 1 } else { units };
            let row = StockRow { units, kind: u8::try_from(kind).unwrap_or(0), alias: nse != sym, file };
            if s.rows.entry(sym.to_owned()).or_default().insert(day, row).is_some() && strict {
                return Err(bad(&format!("({sym}, {}) repeated", g(1))));
            }
            if let Some(x) = s.kinds.get_mut(kind) {
                *x += 1;
            }
            s.aliases += u64::from(row.alias);
            if row.alias {
                s.nse_of.insert((sym.to_owned(), day), nse.to_owned());
            }
            s.n += 1;
            s.first = s.first.min(day);
            s.last = s.last.max(day);
        }
        let mut files: Vec<(u32, String)> = fidx.into_iter().map(|(k, v)| (v, k)).collect();
        files.sort_unstable();
        s.files = files.into_iter().map(|x| x.1).collect();
        Ok(s)
    }

    /// The row of `name` on session `day`: Some(Some(row)) = in range and held; Some(None) = in range, NO row (refused by the caller); None =
    /// outside the table's range (the earlier sources answer, as before). EXACT session: never the latest earlier row (data:F3 7.3).
    #[must_use]
    pub fn lookup(&self, name: &str, day: i64) -> Option<Option<StockRow>> {
        if day < self.first || day > self.last {
            return None;
        }
        let m = self.rows.get(name);
        // MUTANT m_v34i_f3_latest_row: the old "latest row on or before the session" reading (an earlier session's limit fills a hole)
        if cfg!(feature = "m_v34i_f3_latest_row") {
            return Some(m.and_then(|r| r.iter().filter(|(d, _)| **d <= day).max_by_key(|(d, _)| **d).map(|x| *x.1)));
        }
        // MUTANT m_v34i_f3_alias_ignored: col 16 ignored (an alias row read as missing: the ETERNAL / LTM days fall to NONE_IN_RANGE)
        Some(m.and_then(|r| r.get(&day)).copied().filter(|r| !(cfg!(feature = "m_v34i_f3_alias_ignored") && r.alias)))
    }
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
        // integrate:D F3 (data:F3 7.1): data:F3's exact 16-column header is tested BEFORE the old prefix branch, under the switch only (off:
        // the old branch reads its columns 1-3, as before)
        if crate::int34::on(crate::int34::F3D) && head == F3_HEADER {
            if self.stock.is_some() {
                return Err(format!("--freeze {path}: a second data:F3 stock table: refused"));
            }
            let st = StockDated::parse(&text, path, &sha)?;
            let [k0, k1, k2] = st.kinds;
            self.sources.push(format!("data:F3 dated stock freeze table {path} (sha256 {sha}; {} rows: SOURCE {k0} / RULE-BOUND {k1} / GAP-BOUND {k2}, ALIAS {}; sessions {} .. {}; read EXACTLY: a stock contract-day in range reads its own session's row, none = refused)", st.n, st.aliases, crate::cal::fmt_day(st.first), crate::cal::fmt_day(st.last)));
            self.stock = Some(st);
            return Ok(());
        }
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
    let Some(t) = FREEZE.get() else { return 0 };
    slice_units_in(t, c, ci)
}

/// [`slice_units`] on a given table (the tests' own).
#[must_use]
pub fn slice_units_in(t: &Freeze, c: &Cube, ci: usize) -> u64 {
    let Some(k) = c.cons.get(ci) else { return 0 };
    let name = c.unds.get(usize::from(k.und)).map_or("", |u| u.name.as_str());
    // integrate:D F3 (data:F3 7.3-7.6): a STOCK contract-day in the dated table's range reads its OWN session's row (O(1), MEASURED), counted
    // per distinct contract-day by kind and alias; a missing row was refused before pricing (`f3_refused`): 0 here, counted once
    if crate::int34::on(crate::int34::F3D)
        && k.kind == 1
        && let Some(st) = &t.stock
    {
        let t0 = std::time::Instant::now();
        let got = st.lookup(name, c.day);
        f3_lat(t0);
        if let Some(r) = got {
            let lot = u64::from(k.lot.max(1));
            return match r {
                Some(row) => {
                    if f3_note(c.day, k.sid, Some(row)) { slice_of(row.units, lot) } else { (row.units / lot * lot).max(lot) }
                }
                None => {
                    f3_note(c.day, k.sid, None);
                    0
                }
            };
        }
    }
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
    // integrate:D F3 (data:F3 7.2): an optional pinned sha256 of the dated stock table refuses any other file
    if let Some(p) = a.iter().position(|x| x == "--freeze-stock-sha").and_then(|i| a.get(i + 1)) {
        let got = f.stock.as_ref().map_or("(no data:F3 table)", |s| s.sha.as_str());
        if got != p.as_str() {
            return Err(format!("--freeze-stock-sha {p}: the data:F3 table read is {got}: refused"));
        }
        let _ = STOCK_SHA_PIN.set(p.clone());
    }
    let words = f.sources.join("; ");
    let _ = FREEZE.set(f);
    Ok(Some(words))
}

// ------------------------------------------------------------------------------------------------ integrate:D F3: refusal, counters, words

/// F3: whether a stock event is refused because its contract-day has NO row in data:F3's table although its session is in range (counted once
/// per distinct contract-day; never "no limit").
#[must_use]
pub fn f3_refused(c: &Cube, ci: usize) -> bool {
    if !crate::int34::on(crate::int34::F3D) {
        return false;
    }
    FREEZE.get().is_some_and(|t| f3_refused_in(t, c, ci))
}

/// [`f3_refused`] on a given table.
#[must_use]
pub fn f3_refused_in(t: &Freeze, c: &Cube, ci: usize) -> bool {
    let (Some(k), Some(st)) = (c.cons.get(ci), t.stock.as_ref()) else { return false };
    if k.kind != 1 || !crate::int34::on(crate::int34::F3D) {
        return false;
    }
    let name = c.unds.get(usize::from(k.und)).map_or("", |u| u.name.as_str());
    match st.lookup(name, c.day) {
        Some(None) => {
            f3_note(c.day, k.sid, None);
            // MUTANT m_v34i_f3_none_unsliced: a missing row in range priced as "no limit" (one order a block) instead of refused
            !cfg!(feature = "m_v34i_f3_none_unsliced")
        }
        _ => false,
    }
}

/// F3 counter codes: 0 SOURCE, 1 RULE-BOUND, 2 GAP-BOUND, 3 NONE_IN_RANGE; +8 = ALIAS.
const F3_NONE: u8 = 3;

#[cfg(not(test))]
static F3_SEEN: std::sync::Mutex<std::collections::BTreeMap<(i64, u32), u8>> = std::sync::Mutex::new(std::collections::BTreeMap::new());
#[cfg(test)]
thread_local! {
    static F3_SEEN: std::cell::RefCell<std::collections::BTreeMap<(i64, u32), u8>> = const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

/// F3: notes one contract-day's reading (the first note of a contract-day decides; returns whether it was the first).
pub fn f3_note(day: i64, sid: u32, r: Option<StockRow>) -> bool {
    let code = r.map_or(F3_NONE, |x| x.kind + if x.alias { 8 } else { 0 });
    #[cfg(not(test))]
    {
        F3_SEEN.lock().is_ok_and(|mut m| match m.entry((day, sid)) {
            std::collections::btree_map::Entry::Vacant(v) => {
                v.insert(code);
                true
            }
            std::collections::btree_map::Entry::Occupied(_) => false,
        })
    }
    #[cfg(test)]
    {
        F3_SEEN.with(|m| match m.borrow_mut().entry((day, sid)) {
            std::collections::btree_map::Entry::Vacant(v) => {
                v.insert(code);
                true
            }
            std::collections::btree_map::Entry::Occupied(_) => false,
        })
    }
}

/// F3: the distinct contract-days per reading: [SOURCE, RULE-BOUND, GAP-BOUND, NONE_IN_RANGE, ALIAS].
#[must_use]
pub fn f3_counts() -> [u64; 5] {
    let mut out = [0_u64; 5];
    let mut add = |code: u8| {
        if let Some(x) = out.get_mut(usize::from(code & 7)) {
            *x += 1;
        }
        if code & 8 != 0
            && let Some(x) = out.get_mut(4)
        {
            *x += 1;
        }
    };
    #[cfg(not(test))]
    {
        if let Ok(m) = F3_SEEN.lock() {
            m.values().for_each(|&c| add(c));
        }
    }
    #[cfg(test)]
    {
        F3_SEEN.with(|m| m.borrow().values().for_each(|&c| add(c)));
    }
    out
}

/// F3: sampled lookup latencies (ns; every 16th lookup; MEASURED, printed with the counts).
static F3_LAT: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());
static F3_LAT_N: AtomicU64 = AtomicU64::new(0);

fn f3_lat(t0: std::time::Instant) {
    if F3_LAT_N.fetch_add(1, Ordering::Relaxed) % 16 == 0
        && let Ok(mut v) = F3_LAT.lock()
        && v.len() < 1 << 20
    {
        v.push(u32::try_from(t0.elapsed().as_nanos()).unwrap_or(u32::MAX));
    }
}

/// F3: whether data:F3's table is loaded (the switch's check).
#[must_use]
pub fn stock_loaded() -> bool {
    FREEZE.get().is_some_and(|t| t.stock.is_some())
}

/// F3: the table's words (path, sha256) or "(none)".
#[must_use]
pub fn stock_src() -> String {
    FREEZE.get().and_then(|t| t.stock.as_ref()).map_or_else(|| "(none)".to_owned(), |s| format!("{} (sha256 {})", s.path, s.sha))
}

/// F3: `--freeze-stock-sha HEX` (optional): the pinned table sha256 (checklist row 12 needs it to hold when given).
pub static STOCK_SHA_PIN: OnceLock<String> = OnceLock::new();

/// F3: the per-trade words (data:F3 7.8): the freeze value, its kind and its source file (SOURCE) or "conservative bound".
#[must_use]
pub fn trade_words(c: &Cube, ci: usize) -> String {
    let (Some(k), Some(st)) = (c.cons.get(ci), FREEZE.get().and_then(|t| t.stock.as_ref())) else { return "-".to_owned() };
    if k.kind != 1 {
        return "- (index: the dated circular table)".to_owned();
    }
    let name = c.unds.get(usize::from(k.und)).map_or("", |u| u.name.as_str());
    match st.lookup(name, c.day) {
        Some(Some(r)) => format!(
            "{} units ({}{}: {})",
            r.units,
            F3_KINDS.get(usize::from(r.kind)).copied().unwrap_or("?"),
            if r.alias { ", ALIAS" } else { "" },
            if r.kind == 0 { st.files.get(r.file as usize).cloned().unwrap_or_default() } else { "conservative bound, proven never kinder (F3-STOCK-FREEZE.md section 6)".to_owned() }
        ),
        Some(None) => "NONE_IN_RANGE (refused)".to_owned(),
        None => "outside data:F3's range (the earlier sources)".to_owned(),
    }
}

/// F3: checklist row 12 (data:F3 7.7): COVERED only when no stock contract-day in range lacked its row, no stock lookup read the undated
/// file, nothing was unsliced and the pinned sha256 (when given) is the table's; the evidence names the counts and the proof.
#[must_use]
pub fn item12_f3(state: &'static str) -> (&'static str, String) {
    let [src, rb, gb, none, alias] = f3_counts();
    let sha = FREEZE.get().and_then(|t| t.stock.as_ref()).map_or(String::new(), |s| s.sha.clone());
    let pin_ok = STOCK_SHA_PIN.get().is_none_or(|p| *p == sha);
    let (u, n) = (UNDATED.load(Ordering::Relaxed), NONE.load(Ordering::Relaxed));
    let st = if state != "COVERED" { state } else if none == 0 && u == 0 && n == 0 && pin_ok { "COVERED" } else { "NOT COVERED" };
    (st, format!("; integrate:D F3: data:F3's table (sha256 {sha}{}) read EXACTLY on distinct stock contract-days: SOURCE {src} / RULE-BOUND {rb} / GAP-BOUND {gb} (ALIAS {alias}) / NONE_IN_RANGE (refused) {none}; RULE-BOUND / GAP-BOUND = conservative bound proven never kinder on 138,264 NSE-published stock-sessions (F3-STOCK-FREEZE.md section 6)", if STOCK_SHA_PIN.get().is_some() { if pin_ok { ", pinned: holds" } else { ", pinned: DIFFERS" } } else { "" }))
}

/// F3: the counts line (RESULTS, beside the old counters).
#[must_use]
pub fn f3_line() -> String {
    let [src, rb, gb, none, alias] = f3_counts();
    let mut v = F3_LAT.lock().map(|g| g.clone()).unwrap_or_default();
    v.sort_unstable();
    let q = |p: usize| v.get(v.len().saturating_sub(1) * p / 100).copied().unwrap_or(0);
    format!("integrate:D F3 dated stock freeze (distinct stock contract-days): SOURCE {src} / RULE-BOUND {rb} / GAP-BOUND {gb} / of them ALIAS {alias} / NONE_IN_RANGE (refused, never 'no limit') {none}; lookup O(1) (two hash reads) MEASURED p50 {} / p99 {} / max {} ns over {} sampled lookups (every 16th)", q(50), q(99), v.last().copied().unwrap_or(0), v.len())
}

/// The lookup counters as printed.
#[must_use]
pub fn counts_line() -> String {
    if crate::int34::on(crate::int34::F3D) {
        return format!("{}; {}", f3_line(), counts_line_old());
    }
    counts_line_old()
}

fn counts_line_old() -> String {
    format!("freeze lookups{}: dated {} / UNDATED (stock session before the first dated stock file: the earliest file held, labelled) {} / no limit known (one order a block) {}; a lot alone above the freeze quantity {}", if crate::sw::any_ra() { " (distinct contract-days)" } else { "" }, DATED.load(Ordering::Relaxed), UNDATED.load(Ordering::Relaxed), NONE.load(Ordering::Relaxed), LOT_ABOVE.load(Ordering::Relaxed))
}

/// integrate:D F7: the NSE symbol of a cube underlying name on a session (data:F3's alias rows: ETERNAL's sessions before the rename are
/// ZOMATO's in NSE's files); the name itself otherwise.
#[must_use]
pub fn nse_symbol(name: &str, day: i64) -> String {
    FREEZE.get().and_then(|t| t.stock.as_ref()).and_then(|s| s.nse_of.get(&(name.to_owned(), day)).cloned()).unwrap_or_else(|| name.to_owned())
}
