//! The data rows the host reads (plan 3.1: "every level is a data row"; SNIPER s11 (3)). Every reader fails closed: an unknown
//! key, a missing row or a value outside its set is a named refusal, never a default.

use std::collections::BTreeMap;

use sniper_kernel::ladder::Ladder;

/// A month key (`common_wf`'s): year * 12 + (month - 1).
pub type Mk = i32;

/// Parses `YYYY-MM`.
#[must_use]
pub fn parse_mk(s: &str) -> Option<Mk> {
    let (y, m) = s.trim().split_once('-')?;
    let (y, m): (i32, i32) = (y.parse().ok()?, m.parse().ok()?);
    if (1..=12).contains(&m) { Some(y * 12 + m - 1) } else { None }
}

/// `YYYY-MM` of a month key.
#[must_use]
pub fn mk_text(mk: Mk) -> String {
    format!("{:04}-{:02}", mk.div_euclid(12), mk.rem_euclid(12) + 1)
}

/// TSV rows with a header: comment lines (`#`) skipped; returns (header, rows).
fn tsv(text: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let mut header: Vec<String> = Vec::new();
    let mut rows = Vec::new();
    for l in text.lines() {
        if l.starts_with('#') || l.trim().is_empty() {
            continue;
        }
        let f: Vec<String> = l.split('\t').map(str::to_string).collect();
        if header.is_empty() {
            header = f;
        } else {
            rows.push(f);
        }
    }
    (header, rows)
}

fn col(header: &[String], name: &str) -> Result<usize, String> {
    header.iter().position(|h| h == name).ok_or_else(|| format!("column {name} missing"))
}

/// The RUN rows (`data/SNIPER-RUN-v1.tsv`).
#[derive(Clone, Debug, Default)]
pub struct RunRows {
    map: BTreeMap<String, String>,
}

impl RunRows {
    /// Parses the RUN rows (key, value, source); a repeated key or a row without a source is refused.
    ///
    /// # Errors
    /// The refused row.
    pub fn parse(text: &str) -> Result<Self, String> {
        let (h, rows) = tsv(text);
        let (k, v, s) = (col(&h, "key")?, col(&h, "value")?, col(&h, "source")?);
        let mut map = BTreeMap::new();
        for r in rows {
            let (Some(key), Some(val), Some(src)) = (r.get(k), r.get(v), r.get(s)) else { return Err(format!("RUN row short: {r:?}")) };
            if src.trim().is_empty() {
                return Err(format!("RUN row {key} has no source"));
            }
            if map.insert(key.clone(), val.clone()).is_some() {
                return Err(format!("RUN row {key} repeated"));
            }
        }
        Ok(Self { map })
    }

    /// A value.
    ///
    /// # Errors
    /// A missing key.
    pub fn get(&self, key: &str) -> Result<&str, String> {
        self.map.get(key).map(String::as_str).ok_or_else(|| format!("RUN row {key} missing"))
    }

    /// A number.
    ///
    /// # Errors
    /// A missing key or a value that does not parse.
    pub fn num<T: std::str::FromStr>(&self, key: &str) -> Result<T, String> {
        self.get(key)?.trim().parse().map_err(|_| format!("RUN row {key} is not a number"))
    }

    /// A comma list of numbers.
    ///
    /// # Errors
    /// A missing key or an item that does not parse.
    pub fn list<T: std::str::FromStr>(&self, key: &str) -> Result<Vec<T>, String> {
        self.get(key)?.split(',').map(|x| x.trim().parse().map_err(|_| format!("RUN row {key}: item {x} is not a number"))).collect()
    }

    /// `HH:MM:SS` as seconds after midnight.
    ///
    /// # Errors
    /// A malformed time.
    pub fn hms(&self, key: &str) -> Result<i64, String> {
        let v = self.get(key)?;
        let p: Vec<i64> = v.split(':').map(|x| x.parse().unwrap_or(-1)).collect();
        match p.as_slice() {
            [h, m, s] if (0..24).contains(h) && (0..60).contains(m) && (0..60).contains(s) => Ok(h * 3600 + m * 60 + s),
            _ => Err(format!("RUN row {key} is not HH:MM:SS")),
        }
    }
}

/// The learning windows the sweep reads, from common_wf's own WINDOWS-STD-v1.tsv (rows WF and TT).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Windows {
    /// (id, learn from, learn to, test from, test to).
    pub rows: Vec<(String, Mk, Mk, Mk, Mk)>,
}

impl Windows {
    /// Reads the rows `ids` (in that order).
    ///
    /// # Errors
    /// A missing row or month.
    pub fn parse(text: &str, ids: &[&str]) -> Result<Self, String> {
        let (h, rows) = tsv(text);
        let (ci, lf, lt, tf, tt) = (col(&h, "id")?, col(&h, "learn_from")?, col(&h, "learn_to")?, col(&h, "test_from")?, col(&h, "test_to")?);
        let mut out = Vec::new();
        for id in ids {
            let r = rows.iter().find(|r| r.get(ci).is_some_and(|x| x == id)).ok_or_else(|| format!("WINDOWS row {id} missing"))?;
            let m = |i: usize| r.get(i).and_then(|x| parse_mk(x)).ok_or_else(|| format!("WINDOWS row {id}: bad month"));
            out.push(((*id).to_string(), m(lf)?, m(lt)?, m(tf)?, m(tt)?));
        }
        Ok(Self { rows: out })
    }

    /// The learning hull (the union of the learn ranges).
    #[must_use]
    pub fn hull(&self) -> (Mk, Mk) {
        let from = self.rows.iter().map(|r| r.1).min().unwrap_or(0);
        let to = self.rows.iter().map(|r| r.2).max().unwrap_or(-1);
        (from, to)
    }
}

/// The three core ladders of one side (M-RSI, M-CCI, M-WR), from the dials file's `values` and `mirror` cells.
#[derive(Clone, Debug)]
pub struct CoreLadders {
    /// RSI, CCI, %R.
    pub l: [Ladder; 3],
}

/// The core ladders of both sides.
///
/// # Errors
/// A missing core row or a cell that does not parse.
pub fn core_ladders(dials_text: &str) -> Result<(CoreLadders, CoreLadders), String> {
    let (h, rows) = tsv(dials_text);
    let (ci, cv, cm) = (col(&h, "id")?, col(&h, "values")?, col(&h, "mirror")?);
    let mut long = Vec::new();
    let mut short = Vec::new();
    for id in ["M-RSI", "M-CCI", "M-WR"] {
        let r = rows.iter().find(|r| r.get(ci).is_some_and(|x| x == id)).ok_or_else(|| format!("dials row {id} missing"))?;
        let lg = Ladder::long_of(id, r.get(cv).map_or("", String::as_str)).map_err(|e| e.to_string())?;
        let sh = Ladder::short_of(&lg, r.get(cm).map_or("", String::as_str)).map_err(|e| e.to_string())?;
        long.push(lg);
        short.push(sh);
    }
    let arr = |v: Vec<Ladder>| -> Result<[Ladder; 3], String> { v.try_into().map_err(|_| "core ladders".to_string()) };
    Ok((CoreLadders { l: arr(long)? }, CoreLadders { l: arr(short)? }))
}

/// An outer dial's row (id, values grammar, mult, status) for the dials the host wires.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialRow {
    /// The id.
    pub id: String,
    /// The `values` cell.
    pub values: String,
    /// The `mirror` cell.
    pub mirror: String,
    /// The `status` cell (LIVE / HELD:<part> / LATE).
    pub status: String,
    /// The `mult` cell (a number, `htf` or `htf*k`).
    pub mult: String,
}

/// Every dial row of the file, in the sealed order (the code's digit order: the file's row order, M-CCI first).
///
/// # Errors
/// A malformed file.
pub fn dial_rows(dials_text: &str) -> Result<Vec<DialRow>, String> {
    let (h, rows) = tsv(dials_text);
    let (ci, cv, cm, cs, cx) = (col(&h, "id")?, col(&h, "values")?, col(&h, "mirror")?, col(&h, "status")?, col(&h, "mult")?);
    rows.iter()
        .map(|r| {
            let g = |i: usize| r.get(i).cloned().unwrap_or_default();
            Ok(DialRow { id: g(ci), values: g(cv), mirror: g(cm), status: g(cs), mult: g(cx) })
        })
        .collect()
}

/// The kind of one exit set (plan 4 item 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExitKind {
    /// XS: the first candle closing on the other side of Supertrend (period, factor milli).
    Supertrend(u16, u32),
    /// XE: the first candle closing across EMA period (with S7's context filter on entry).
    Ema(u16),
    /// XQ-A: REQ 47's small stop at x / 100 ATR 14.
    SmallStop(u32),
    /// XQ-G: the go-positive timer, seconds.
    GoPositive(u32),
    /// X-1510: hold to the square-off.
    Clock,
    /// XC (SNIPER s20, the ONE combined exit of every NEW search): the first of the Supertrend flip against the trade (period,
    /// factor milli; the same flip rule as the XS row of that setting) and a plain clock exit `secs` seconds after the entry fill
    /// (E0, the crate's `path::Rule::TimeExit`, the timer exits' path), plus the standing 15:10 square-off.
    SupertrendOrClock(u16, u32, u32),
}

/// One base exit set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitSet {
    /// Its id.
    pub id: String,
    /// Its kind.
    pub kind: ExitKind,
}

/// SNIPER s20: an XC id `XC-ST<period>-<factor x 10>-T<seconds>` (e.g. `XC-ST10-30-T300`: Supertrend 10 x 3.0 or 300 s), its
/// row's `kind` cell `COMBINED`, its Supertrend setting one of the readings' rows (`cands::ST_ROWS`), its clock 1..=3600 s.
/// Fail closed: any other form is a named refusal, never a default.
fn xc_kind(id: &str, rest: &str, kind_cell: Option<&String>) -> Result<ExitKind, String> {
    let bad = || format!("exit id {id}: not of the form XC-ST<period>-<factor x 10>-T<seconds> (SNIPER s20)");
    let mut it = rest.split('-');
    let (Some(p), Some(m), Some(t), None) = (it.next(), it.next(), it.next(), it.next()) else { return Err(bad()) };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let t = t.strip_prefix('T').ok_or_else(bad)?;
    if !(digits(p) && digits(m) && digits(t)) {
        return Err(bad());
    }
    let p: u16 = p.parse().map_err(|_| bad())?;
    let f: u32 = m.parse::<u32>().map_err(|_| bad())?.checked_mul(100).ok_or_else(bad)?;
    let secs: u32 = t.parse().map_err(|_| bad())?;
    if !crate::cands::ST_ROWS.contains(&(p, f)) {
        return Err(format!("exit id {id}: Supertrend ({p}, {f}/1000) is not a readings row (cands::ST_ROWS); refused (SNIPER s20)"));
    }
    if !(1..=3600).contains(&secs) {
        return Err(format!("exit id {id}: clock {secs} s outside 1..=3600 (SNIPER s20)"));
    }
    if kind_cell.map(String::as_str) != Some("COMBINED") {
        return Err(format!("exit id {id}: its kind cell must be COMBINED (SNIPER s20), found {kind_cell:?}"));
    }
    Ok(ExitKind::SupertrendOrClock(p, f, secs))
}

/// The 21 BASE exit sets of the exits file (composites are SEARCH-QUEUED under S11 or BESIDE, never base sets), then SNIPER
/// s20's combined XC rows (zero in SNIPER-EXITS-v2; one, `XC-ST10-30-T300`, in SNIPER-EXITS-v3), in the file's order.
///
/// # Errors
/// An id of no known form, a malformed XC row, a repeated id, or a base-set count other than 21.
pub fn exit_sets(exits_text: &str) -> Result<Vec<ExitSet>, String> {
    let (h, rows) = tsv(exits_text);
    let ci = col(&h, "id")?;
    let ck = h.iter().position(|x| x == "kind");
    let mut out = Vec::new();
    let mut base = 0_usize;
    for r in &rows {
        let Some(id) = r.get(ci) else { continue };
        if id.ends_with("-*") {
            continue;
        }
        let num = |s: &str| s.parse::<u32>().map_err(|_| format!("exit id {id}"));
        let kind = if let Some(rest) = id.strip_prefix("XC-ST") {
            xc_kind(id, rest, ck.and_then(|k| r.get(k)))?
        } else if let Some(rest) = id.strip_prefix("XS-") {
            let (p, m) = rest.split_once('-').ok_or_else(|| format!("exit id {id}"))?;
            ExitKind::Supertrend(u16::try_from(num(p)?).map_err(|_| format!("exit id {id}"))?, num(m)? * 100)
        } else if let Some(p) = id.strip_prefix("XE-") {
            ExitKind::Ema(u16::try_from(num(p)?).map_err(|_| format!("exit id {id}"))?)
        } else if let Some(x) = id.strip_prefix("XQ-A") {
            ExitKind::SmallStop(num(x)?)
        } else if let Some(t) = id.strip_prefix("XQ-G") {
            ExitKind::GoPositive(num(t)?)
        } else if id == "X-1510" {
            ExitKind::Clock
        } else {
            return Err(format!("exit id {id} of no known form"));
        };
        if !matches!(kind, ExitKind::SupertrendOrClock(..)) {
            base += 1;
        }
        if out.iter().any(|e: &ExitSet| &e.id == id) {
            return Err(format!("exit id {id} repeated"));
        }
        out.push(ExitSet { id: id.clone(), kind });
    }
    if base != 21 {
        return Err(format!("exits file holds {base} base sets, not 21"));
    }
    Ok(out)
}

/// A file's text and its sha256 (hex), both recorded in SN1.
///
/// # Errors
/// An unreadable file.
pub fn read_with_sha(path: &str) -> Result<(String, String), String> {
    let b = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let sha = crate::sha::hex(&crate::sha::sha256(&b));
    let t = String::from_utf8(b).map_err(|_| format!("{path}: not UTF-8"))?;
    Ok((t, sha))
}
