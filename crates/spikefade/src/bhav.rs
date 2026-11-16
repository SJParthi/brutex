//! v3.2 (MAIN-RULINGS-ENGINE-v22 addenda 13 / 13b): the OFFICIAL previous close of every traded option from NSE's F&O bhavcopy of the
//! PREVIOUS session (published the evening before: known live at the decision second, no look-ahead), and the dated expiry list of each
//! underlying (addendum 16: weekly / monthly, the regular weekday) from the same file.
//!
//! Files (`run3 --bhav-dir <dir>`, repeatable): NSE's UDiFF `BhavCopy_NSE_FO_0_0_0_YYYYMMDD_F_0000.csv.zip` (2024-07 on) and the old
//! `foDDMONYYYYbhav.csv.zip`; each zip is read through `/usr/bin/unzip -p` (the host does the I/O; nothing is extracted to disk). Option
//! rows only (UDiFF `IDO` / `STO`, old `OPTIDX` / `OPTSTK`); futures rows are dropped before any other column is read.
//!
//! Two previous-close numbers are kept per contract, both from the previous session's file: `ClsPric` (NSE's official CLOSE, the number the
//! ruling names) and `LastPric` (the last traded price). Measured on PIIND 23 NOV 2026 1900 PE: the 4 Nov file says ClsPric 7.35 and
//! LastPric 7.80, and the operator's Dhan screen shows 7.80 (+1,079.49% at Rs 92.00): Dhan's "previous close" is the LAST PRICE, not NSE's
//! CLOSE. Every trade prints both, named. The old format has no last price (close only, named).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::cal::days_from_civil;

/// A contract key: (symbol, expiry yyyymmdd, strike paisa, right 0 CE / 1 PE).
pub type Key = (String, u32, i64, u8);

/// One session's bhavcopy, option rows only.
#[derive(Clone, Debug, Default)]
pub struct BhavDay {
    pub day: i64,
    pub file: String,
    pub sha: String,
    /// (ClsPric, LastPric (0 = not in the file)) per contract, paisa.
    pub px: HashMap<Key, (i64, i64)>,
    /// Every option expiry listed per underlying (sorted).
    pub expiries: HashMap<String, Vec<u32>>,
    /// Option rows read.
    pub rows: u64,
}

/// The bhavcopy files held, by session.
#[derive(Debug, Default)]
pub struct Bhav {
    files: BTreeMap<i64, PathBuf>,
    cache: Mutex<HashMap<i64, Arc<BhavDay>>>,
    pub sources: Vec<String>,
}

/// The run's files.
pub static BHAV: OnceLock<Bhav> = OnceLock::new();

const MON: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];

/// The session of a bhavcopy file name.
#[must_use]
pub fn day_of_name(n: &str) -> Option<i64> {
    if let Some(r) = n.strip_prefix("BhavCopy_NSE_FO_0_0_0_") {
        let d = r.get(0..8)?;
        if !r.ends_with(".csv.zip") {
            return None;
        }
        return Some(days_from_civil(d.get(0..4)?.parse().ok()?, d.get(4..6)?.parse().ok()?, d.get(6..8)?.parse().ok()?));
    }
    // foDDMONYYYYbhav.csv.zip
    let r = n.strip_prefix("fo")?.strip_suffix("bhav.csv.zip")?;
    if r.len() != 9 {
        return None;
    }
    let m = MON.iter().position(|x| Some(*x) == r.get(2..5))?;
    Some(days_from_civil(r.get(5..9)?.parse().ok()?, i64::try_from(m + 1).ok()?, r.get(0..2)?.parse().ok()?))
}

/// "1900.00" / "1900" -> paisa (at most 2 decimals; more refused).
#[must_use]
pub fn paisa(s: &str) -> Option<i64> {
    common_fill::margin::parse_paisa_round(s.trim().as_bytes())
}

/// "2026-11-23" or "23-Nov-2026" -> yyyymmdd.
#[must_use]
pub fn ymd_of(s: &str) -> Option<u32> {
    let s = s.trim();
    if s.len() == 10 && s.as_bytes().get(4) == Some(&b'-') {
        return format!("{}{}{}", s.get(0..4)?, s.get(5..7)?, s.get(8..10)?).parse().ok();
    }
    let p: Vec<&str> = s.split('-').collect();
    let (d, m, y) = (p.first()?, p.get(1)?, p.get(2)?);
    let mi = MON.iter().position(|x| x.eq_ignore_ascii_case(m))?;
    format!("{y}{:02}{:02}", mi + 1, d.parse::<u32>().ok()?).parse().ok()
}

/// Parses one bhavcopy CSV text (UDiFF or old), option rows only.
///
/// # Errors
/// A header that is neither format.
pub fn parse(text: &str, day: i64, file: &str, sha: &str) -> Result<BhavDay, String> {
    let mut lines = text.lines();
    let head: Vec<String> = lines.next().unwrap_or("").split(',').map(|x| x.trim().to_owned()).collect();
    let col = |n: &str| head.iter().position(|h| h == n);
    let mut out = BhavDay { day, file: file.to_owned(), sha: sha.to_owned(), ..BhavDay::default() };
    let udiff = col("FinInstrmTp").is_some();
    let (ci, cs, ce, ck, cr, cc, cl) = if udiff {
        (col("FinInstrmTp"), col("TckrSymb"), col("XpryDt"), col("StrkPric"), col("OptnTp"), col("ClsPric"), col("LastPric"))
    } else {
        (col("INSTRUMENT"), col("SYMBOL"), col("EXPIRY_DT"), col("STRIKE_PR"), col("OPTION_TYP"), col("CLOSE"), None)
    };
    let (Some(ci), Some(cs), Some(ce), Some(ck), Some(cr), Some(cc)) = (ci, cs, ce, ck, cr, cc) else { return Err(format!("{file}: not an NSE F&O bhavcopy header")) };
    for l in lines {
        let f: Vec<&str> = l.split(',').collect();
        let inst = f.get(ci).map_or("", |x| x.trim());
        // futures (and anything else) dropped first
        if !matches!(inst, "IDO" | "STO" | "OPTIDX" | "OPTSTK") {
            continue;
        }
        let (Some(sym), Some(exp), Some(k), Some(rt), Some(close)) = (f.get(cs).map(|x| x.trim()), f.get(ce).and_then(|x| ymd_of(x)), f.get(ck).and_then(|x| paisa(x)), f.get(cr).map(|x| x.trim()), f.get(cc).and_then(|x| paisa(x))) else { continue };
        let right = match rt {
            "CE" => 0,
            "PE" => 1,
            _ => continue,
        };
        let last = cl.and_then(|i| f.get(i)).and_then(|x| paisa(x)).unwrap_or(0);
        out.px.insert((sym.to_owned(), exp, k, right), (close, last));
        let v = out.expiries.entry(sym.to_owned()).or_default();
        if !v.contains(&exp) {
            v.push(exp);
        }
        out.rows += 1;
    }
    for v in out.expiries.values_mut() {
        v.sort_unstable();
    }
    Ok(out)
}

impl Bhav {
    /// Indexes every bhavcopy zip of a directory.
    ///
    /// # Errors
    /// An unreadable directory.
    pub fn add_dir(&mut self, dir: &Path) -> Result<(), String> {
        // v3.4 E5: under PORTABLE the entries in name order (APFS and ext4 list a folder differently; "first one kept" must not depend on it)
        let ents: Vec<(String, std::path::PathBuf)> = if crate::kit::on(crate::kit::PORTABLE) {
            crate::kit::sorted_dir(dir).map_err(|e| format!("--bhav-dir {e}"))?
        } else {
            std::fs::read_dir(dir).map_err(|e| format!("--bhav-dir {}: {e}", dir.display()))?.flatten().map(|e| (e.file_name().to_string_lossy().into_owned(), e.path())).collect()
        };
        let mut n = 0_usize;
        for (name, path) in ents {
            if let Some(d) = day_of_name(&name) {
                // a later directory's copy of the same session wins only when the first had none (first one kept)
                self.files.entry(d).or_insert_with(|| path);
                n += 1;
            }
        }
        self.sources.push(format!("{} ({n} files)", dir.display()));
        Ok(())
    }

    /// Every session with a file held (sorted).
    #[must_use]
    pub fn sessions(&self) -> Vec<i64> {
        self.files.keys().copied().collect()
    }

    /// The previous session held (the latest file strictly before `day`).
    #[must_use]
    pub fn prev_of(&self, day: i64) -> Option<i64> {
        // MUTANT m_v32_prev_close_today: the session's OWN file read (published after its close: look-ahead)
        if cfg!(feature = "m_v32_prev_close_today") {
            return self.files.range(..=day).next_back().map(|(&d, _)| d);
        }
        self.files.range(..day).next_back().map(|(&d, _)| d)
    }

    /// One session's parsed file (cached).
    ///
    /// # Errors
    /// No file for the session, an unzip failure or an unknown header.
    pub fn day(&self, day: i64) -> Result<Arc<BhavDay>, String> {
        if let Ok(g) = self.cache.lock()
            && let Some(x) = g.get(&day)
        {
            return Ok(Arc::clone(x));
        }
        let p = self.files.get(&day).ok_or_else(|| format!("no bhavcopy for {}", crate::cal::fmt_day(day)))?;
        let o = std::process::Command::new(crate::kit::tool("unzip")).arg("-p").arg(p).output().map_err(|e| format!("unzip {}: {e}", p.display()))?;
        if !o.status.success() {
            return Err(format!("unzip -p {} failed", p.display()));
        }
        let text = String::from_utf8_lossy(&o.stdout);
        let name = p.file_name().map_or_else(String::new, |x| x.to_string_lossy().into_owned());
        let bd = Arc::new(parse(&text, day, &name, &crate::cube::sha256(p))?);
        if let Ok(mut g) = self.cache.lock() {
            g.insert(day, Arc::clone(&bd));
        }
        Ok(bd)
    }
}

/// Loads every `--bhav-dir` of the command line.
///
/// # Errors
/// A refused directory.
pub fn from_args(a: &[String]) -> Result<Option<String>, String> {
    let dirs: Vec<&String> = a.iter().enumerate().filter(|(_, x)| *x == "--bhav-dir").filter_map(|(i, _)| a.get(i + 1)).collect();
    if dirs.is_empty() {
        return Ok(None);
    }
    let mut b = Bhav::default();
    for d in dirs {
        b.add_dir(Path::new(d))?;
    }
    let words = format!("{}; sessions {} ({} .. {})", b.sources.join(" + "), b.files.len(), b.files.keys().next().map_or("-".to_owned(), |&d| crate::cal::fmt_day(d)), b.files.keys().next_back().map_or("-".to_owned(), |&d| crate::cal::fmt_day(d)));
    let _ = BHAV.set(b);
    Ok(Some(words))
}

/// The previous official close of a contract traded on `day`: (previous session, ClsPric, LastPric, file) or None (named by the caller).
#[must_use]
pub fn prev_close(day: i64, sym: &str, expiry: u32, strike: i64, right: u8) -> Option<(i64, i64, i64, String)> {
    let b = BHAV.get()?;
    let p = b.prev_of(day)?;
    // v3.4 F4: under STRICT a file the index holds that cannot be read (unzip missing or failing, an unknown header) is FATAL, not "no close"
    let bd = strict_day(b, p)?;
    let (c, l) = bd.px.get(&(sym.to_owned(), expiry, strike, right)).copied()?;
    Some((p, c, l, bd.file.clone()))
}

/// The expiries NSE listed for `sym` in the previous session's file (addendum 16's dated expiry list), with that session.
#[must_use]
pub fn expiries(day: i64, sym: &str) -> Option<(i64, Vec<u32>)> {
    let b = BHAV.get()?;
    let p = b.prev_of(day)?;
    let bd = strict_day(b, p)?;
    bd.expiries.get(sym).map(|v| (p, v.clone()))
}

/// v3.4 F4: one session's file for a lookup; under STRICT a read failure of a file the index holds is recorded as fatal (the run refuses
/// before its next batch file), so a box without `unzip` never runs on with previous close 0 and the staged expiries (critic 2.1).
pub(crate) fn strict_day(b: &Bhav, p: i64) -> Option<Arc<BhavDay>> {
    match b.day(p) {
        Ok(x) => Some(x),
        Err(e) => {
            // MUTANT m_kit_strict_bhav_swallowed: the failure swallowed under STRICT (v3.3's lenient None)
            if !cfg!(feature = "m_kit_strict_bhav_swallowed") {
                crate::kit::strict_fail("bhavcopy", &e);
            }
            None
        }
    }
}
