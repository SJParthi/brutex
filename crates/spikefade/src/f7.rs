//! v3.4 integrate:D F7 (MAIN-RULINGS-ENGINE-v22 Addendum 30.2, standing F7 = exits F-B1, F29; data:F7 `v34/F7-BANDS-PROTECTION.md` section 5):
//! NSE's option price bands (the dated begin-of-day band of every contract from NSE's contract files, the proven no-file bound before
//! 2024-02-06), the band lock at the KNOWN level, NSE's limit price protection on re-sent orders, and Dhan's option market protection, as
//! dated READINGS of every exit; the dearest admissible exit is booked, the sale follows the one strict rule R5.
//!
//! Switches: `int34::PROT`, `int34::LPP`, `int34::CUT` (this module) and build:D-high's BANDS (`fix34::BANDS`, the band source). Every one off:
//! no code here runs (v3.4 build:D-high byte for byte).

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

/// One dated segment of data:F7's engine-facing switch table `F7-PROTECTION-READINGS.tsv` (data:F7 5.1): from / to inclusive sessions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seg7 {
    pub from: i64,
    pub to: i64,
    /// LPP live for index weekly + monthly / index other expiries / stock options.
    pub lpp_idx_wm: bool,
    pub lpp_idx_other: bool,
    pub lpp_stk: bool,
    /// NSE's cap on option market orders (20% / Rs 10) live; and clipped to the LPP band.
    pub mpp_nse: bool,
    pub mpp_lpp: bool,
    /// The band source: true = NSE's exact file (from 2024-02-06), false = the proven no-file bound.
    pub band_file: bool,
    /// The dated minimum range (paise).
    pub min_range: i64,
    /// The exit readings: R0, R5, R10, RM.
    pub r0: bool,
    pub r5: bool,
    pub r10: bool,
    pub rm: bool,
    /// Dhan's API order rate (orders a second).
    pub rate: u32,
    /// Stock-option LPP ceiling / floor in the underlying's cooling-off (from 2024-11-18).
    pub cool_stk: bool,
}

/// data:F7's dated switch table.
#[derive(Clone, Debug, Default)]
pub struct Readings {
    pub segs: Vec<Seg7>,
    pub src: String,
}

/// The run's table.
pub static READINGS_T: OnceLock<Readings> = OnceLock::new();

impl Readings {
    /// Parses `F7-PROTECTION-READINGS.tsv` (comment lines `#`; the header `from to lpp_idx_wm ...`): the segments must be contiguous sessions
    /// in date order (a gap or an overlap is refused: data:F7's test `f7_rules_tables_load_dated`).
    ///
    /// # Errors
    /// A header, row or segment order that is not the format.
    pub fn parse(text: &str, src: &str) -> Result<Self, String> {
        let mut lines = text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty());
        let head = lines.next().unwrap_or("");
        let want = ["from", "to", "lpp_idx_wm", "lpp_idx_other", "lpp_stk", "ter", "mpp_nse", "mpp_lpp", "band_src", "min_range", "tick", "exit_readings", "entry_reading", "rate", "cool_stk"];
        let h: Vec<&str> = head.split('\t').collect();
        if h.len() < want.len() || h.iter().zip(want).any(|(a, b)| *a != b) {
            return Err(format!("--f7-readings {src}: header must start {}", want.join("<TAB>")));
        }
        let mut segs: Vec<Seg7> = Vec::new();
        for l in lines {
            let f: Vec<&str> = l.split('\t').collect();
            let g = |k: usize| f.get(k).copied().unwrap_or("");
            let bad = || format!("--f7-readings {src}: bad row {l}");
            let day = |k: usize| crate::cal::parse_day(g(k)).ok_or_else(bad);
            let b01 = |k: usize| match g(k) {
                "0" => Ok(false),
                "1" => Ok(true),
                _ => Err(bad()),
            };
            let ex = g(11);
            let s = Seg7 {
                from: day(0)?,
                to: day(1)?,
                lpp_idx_wm: b01(2)?,
                lpp_idx_other: b01(3)?,
                lpp_stk: b01(4)?,
                mpp_nse: match g(6) {
                    "none" => false,
                    "20pc10" => true,
                    _ => return Err(bad()),
                },
                mpp_lpp: b01(7)?,
                band_file: match g(8) {
                    "file" => true,
                    "bound" => false,
                    _ => return Err(bad()),
                },
                min_range: g(9).parse::<i64>().map_err(|_| bad())?,
                r0: ex.split(',').any(|x| x == "R0"),
                r5: ex.split(',').any(|x| x == "R5"),
                r10: ex.split(',').any(|x| x == "R10"),
                rm: ex.split(',').any(|x| x == "RM"),
                rate: g(13).parse::<u32>().map_err(|_| bad())?,
                cool_stk: b01(14)?,
            };
            if g(12) != "R5" || s.to < s.from {
                return Err(bad());
            }
            // contiguous: the next segment starts the calendar day after the previous one ends, or after a weekend between (Saturday /
            // Sunday only: 1970-01-01 was a Thursday). MUTANT m_v34i_f7_segment_gap: the check skipped (a missing day goes unnoticed)
            if let Some(p) = segs.last()
                && !cfg!(feature = "m_v34i_f7_segment_gap")
            {
                let weekend = |d: i64| matches!((d + 4).rem_euclid(7), 0 | 6);
                if s.from <= p.to || !(p.to + 1..s.from).all(weekend) {
                    return Err(format!("--f7-readings {src}: segment {} .. {} does not follow the previous one (ends {}) without a gap or an overlap", g(0), g(1), crate::cal::fmt_day(p.to)));
                }
            }
            segs.push(s);
        }
        if segs.is_empty() {
            return Err(format!("--f7-readings {src}: no segment"));
        }
        Ok(Self { segs, src: src.to_owned() })
    }

    /// The segment of session `day` (None = outside the table: the strict readings refuse to price it, counted by the caller).
    #[must_use]
    pub fn seg(&self, day: i64) -> Option<&Seg7> {
        let i = self.segs.partition_point(|s| s.to < day);
        self.segs.get(i).filter(|s| s.from <= day)
    }
}

/// Loads `--f7-readings FILE` into [`READINGS_T`].
///
/// # Errors
/// An unreadable or refused table.
pub fn load_readings(p: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(p).map_err(|e| format!("--f7-readings {p}: {e}"))?;
    let sha = note_sha("readings", p);
    let r = Readings::parse(&text, &format!("{p} (sha256 {sha})"))?;
    let _ = READINGS_T.set(r);
    Ok(())
}

/// The table's words.
#[must_use]
pub fn readings_src() -> String {
    READINGS_T.get().map_or_else(|| "(no table)".to_owned(), |r| format!("{} ({} dated segments)", r.src, r.segs.len()))
}

// ------------------------------------------------------------------------------------------------ the band sources (data:F7 5.1)

/// data:F7's session -> NSE contract file manifest (`F7-OPR-SESSIONS.tsv`): the file of session D-1 holds session D's begin-of-day band.
#[derive(Clone, Debug, Default)]
pub struct Opr {
    pub by: BTreeMap<i64, (String, String)>,
    pub src: String,
}

/// The run's manifest (`--bands <F7-OPR-SESSIONS.tsv>` under BANDS).
pub static OPR_T: OnceLock<Opr> = OnceLock::new();
/// The header start of data:F7's manifest.
pub const OPR_HEADER: &str = "session\tsession_kind\tnse_file\tnse_file_day\tfile_sha256";

/// Whether a `--bands` file is data:F7's manifest (else build:D-high's normalised BandTable).
#[must_use]
pub fn is_manifest(p: &str) -> bool {
    std::fs::read_to_string(p).ok().is_some_and(|t| t.lines().find(|l| !l.starts_with('#')).is_some_and(|h| h.starts_with(OPR_HEADER)))
}

/// Loads data:F7's manifest: one row per session (YYYYMMDD), its NSE file and that file's sha256 (checked when the day is loaded).
///
/// # Errors
/// An unreadable manifest, a bad row, a repeated session.
pub fn load_manifest(p: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(p).map_err(|e| format!("--bands {p}: {e}"))?;
    let mut by = BTreeMap::new();
    for l in text.lines().filter(|l| !l.starts_with('#')).skip(1) {
        let f: Vec<&str> = l.split('\t').collect();
        let (Some(s), Some(path), Some(sha)) = (f.first(), f.get(2), f.get(4)) else { return Err(format!("--bands {p}: bad row {l}")) };
        let day = s.get(0..4).zip(s.get(4..6)).zip(s.get(6..8)).and_then(|((y, m), d)| Some(crate::cal::days_from_civil(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?))).ok_or_else(|| format!("--bands {p}: bad session {s}"))?;
        if sha.len() != 64 || by.insert(day, ((*path).to_owned(), (*sha).to_owned())).is_some() {
            return Err(format!("--bands {p}: bad sha256 or a repeated session in {l}"));
        }
    }
    let src = format!("{p} (sha256 {}; {} sessions {} .. {} with NSE's exact begin-of-day band file)", note_sha("manifest", p), by.len(), by.keys().next().map_or("-".to_owned(), |&d| crate::cal::fmt_day(d)), by.keys().next_back().map_or("-".to_owned(), |&d| crate::cal::fmt_day(d)));
    let _ = OPR_T.set(Opr { by, src });
    Ok(())
}

/// data:F3's strike adjustments (`ca-events.tsv`: session, symbol): on such a (symbol, session) the no-file bound is not used (the base was
/// adjusted: band unknown), data:F7 4.4.
pub static CA_T: OnceLock<std::collections::HashSet<(String, i64)>> = OnceLock::new();

/// Loads `--f7-ca FILE`.
///
/// # Errors
/// An unreadable file or a bad row.
pub fn load_ca(p: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(p).map_err(|e| format!("--f7-ca {p}: {e}"))?;
    note_sha("ca", p);
    let mut s = std::collections::HashSet::new();
    for l in text.lines().skip(1) {
        let f: Vec<&str> = l.split('\t').collect();
        let (Some(d), Some(sym)) = (f.first(), f.get(1)) else { continue };
        let day = d.get(0..4).zip(d.get(4..6)).zip(d.get(6..8)).and_then(|((y, m), dd)| Some(crate::cal::days_from_civil(y.parse().ok()?, m.parse().ok()?, dd.parse().ok()?))).ok_or_else(|| format!("--f7-ca {p}: bad row {l}"))?;
        s.insert(((*sym).to_owned(), day));
    }
    let _ = CA_T.set(s);
    Ok(())
}

/// One session's band rows: NSE symbol -> (expiry yyyymmdd, strike paisa, right 0 CE / 1 PE) -> (lower, upper, tick) paisa; `exact` = NSE's
/// file (else the proven no-file bound: lower = max(5, settle - min range) is an UPPER bound of the true lower band, upper = settle + min
/// range a LOWER bound of the true upper band).
#[derive(Clone, Debug, Default)]
pub struct DayBand {
    pub by: HashMap<String, HashMap<(u32, i64, u8), (i64, i64, i64)>>,
    pub exact: bool,
    pub src: String,
}

/// The per-session cache (a few sessions: day-major walks the days in order).
static DAYS: Mutex<VecDeque<(i64, Arc<DayBand>)>> = Mutex::new(VecDeque::new());
const DAYS_KEPT: usize = 8;

/// The band rows of session `day` (cached): NSE's file when the manifest holds the session, else the no-file bound when the dated table says
/// "bound", else None (counted by the caller as band unknown).
///
/// # Errors
/// A file whose sha256 is not the manifest's, an unreadable file.
pub fn day_band(day: i64) -> Result<Option<Arc<DayBand>>, String> {
    if let Ok(g) = DAYS.lock()
        && let Some((_, b)) = g.iter().find(|x| x.0 == day)
    {
        return Ok(Some(Arc::clone(b)));
    }
    let seg = READINGS_T.get().and_then(|r| r.seg(day));
    // a MEASUREMENT mode only (`--f7-force-bound on`, data:F7 test f7_bound_cost_on_rs3_era): the no-file bound even where NSE's file exists
    let force = FORCE_BOUND.load(std::sync::atomic::Ordering::Relaxed);
    let b = if let Some((path, sha)) = OPR_T.get().and_then(|o| o.by.get(&day)).filter(|_| !force) {
        load_file_day(path, sha)?
    } else if force || seg.is_some_and(|s| !s.band_file) {
        match load_bound_day(day, seg.map_or(300, |s| s.min_range))? {
            Some(b) => b,
            None => return Ok(None),
        }
    } else {
        return Ok(None);
    };
    let b = Arc::new(b);
    if let Ok(mut g) = DAYS.lock() {
        if g.len() >= DAYS_KEPT {
            g.pop_front();
        }
        g.push_back((day, Arc::clone(&b)));
    }
    Ok(Some(b))
}

/// NSE's 1980 epoch (seconds) -> yyyymmdd (the contract file's XpryDt).
#[must_use]
pub fn ymd_of_nse(secs: i64) -> u32 {
    crate::cal::ymd_of(3_652 + secs.div_euclid(86_400))
}

/// Parses one NSE contract file (gz): the option rows' MinPric / MaxPric / BidIntrvl.
///
/// # Errors
/// The sha256 is not the manifest's; the file cannot be read; a needed column is missing.
pub fn load_file_day(path: &str, sha: &str) -> Result<DayBand, String> {
    let p = std::path::Path::new(path);
    // MUTANT m_v34i_f7_sha_unchecked: the file read whatever its sha256
    let got = crate::kit::sha256_file(p).unwrap_or_default();
    if got != sha && !cfg!(feature = "m_v34i_f7_sha_unchecked") {
        return Err(format!("{path}: sha256 {got} is not data:F7's manifest's {sha}: refused"));
    }
    let o = std::process::Command::new(crate::kit::tool("gzip")).arg("-dc").arg(p).output().map_err(|e| format!("gzip -dc {path}: {e}"))?;
    if !o.status.success() {
        return Err(format!("gzip -dc {path} failed"));
    }
    parse_contract_file(&String::from_utf8_lossy(&o.stdout), &format!("{path} (sha256 {sha})"))
}

/// The option rows of an NSE contract file's text.
///
/// # Errors
/// A needed column missing.
pub fn parse_contract_file(text: &str, src: &str) -> Result<DayBand, String> {
    let mut lines = text.lines();
    let h: Vec<&str> = lines.next().unwrap_or("").split(',').collect();
    let ix = |n: &str| h.iter().position(|x| x.trim() == n);
    let (Some(is), Some(ix_), Some(ik), Some(io), Some(imin), Some(imax), Some(it)) = (ix("TckrSymb"), ix("XpryDt"), ix("StrkPric"), ix("OptnTp"), ix("MinPric"), ix("MaxPric"), ix("BidIntrvl")) else { return Err(format!("{src}: not NSE's contract file header")) };
    let mut by: HashMap<String, HashMap<(u32, i64, u8), (i64, i64, i64)>> = HashMap::new();
    for l in lines {
        let f: Vec<&str> = l.split(',').collect();
        let g = |k: usize| f.get(k).map_or("", |x| x.trim());
        let right = match g(io) {
            "CE" => 0,
            "PE" => 1,
            _ => continue,
        };
        let (Ok(xs), Ok(k), Ok(lo), Ok(hi), Ok(tk)) = (g(ix_).parse::<i64>(), g(ik).parse::<i64>(), g(imin).parse::<i64>(), g(imax).parse::<i64>(), g(it).parse::<i64>()) else { continue };
        if hi <= lo {
            // a contract expiring on the file's own day (zero width): never looked up for the next session (data:F7 5.1)
            continue;
        }
        by.entry(g(is).to_owned()).or_default().insert((ymd_of_nse(xs), k, right), (lo, hi, tk.max(1)));
    }
    Ok(DayBand { by, exact: true, src: src.to_owned() })
}

/// The no-file bound of session `day` (data:F7 5.1 / 4.4): from the PREVIOUS session's F&O bhavcopy SETTLE (SETTLE_PR / SttlmPric):
/// upper >= settle + min range (`hi` = that lower bound), lower <= max(5, settle - min range); tick 5. None when no bhavcopy is held.
///
/// # Errors
/// A bhavcopy that cannot be read.
pub fn load_bound_day(day: i64, min_range: i64) -> Result<Option<DayBand>, String> {
    let Some(b) = crate::bhav::BHAV.get() else { return Ok(None) };
    let Some(p) = b.prev_of(day) else { return Ok(None) };
    let Some(path) = b.file_of(p) else { return Ok(None) };
    let o = std::process::Command::new(crate::kit::tool("unzip")).arg("-p").arg(path).output().map_err(|e| format!("unzip -p {}: {e}", path.display()))?;
    if !o.status.success() {
        return Err(format!("unzip -p {} failed", path.display()));
    }
    let text = String::from_utf8_lossy(&o.stdout);
    parse_bound(&text, day, min_range, CA_T.get(), &path.display().to_string()).map(Some)
}

/// The no-file bound rows of session `day` from the previous session's bhavcopy text (the tests' entry; see [`load_bound_day`]); `ca` = the
/// strike-adjusted (symbol, session) pairs left without a bound.
///
/// # Errors
/// A header without the settle column.
pub fn parse_bound(text: &str, day: i64, min_range: i64, ca: Option<&std::collections::HashSet<(String, i64)>>, src: &str) -> Result<DayBand, String> {
    let mut lines = text.lines();
    let h: Vec<String> = lines.next().unwrap_or("").split(',').map(|x| x.trim().to_owned()).collect();
    let col = |n: &str| h.iter().position(|x| x == n);
    let udiff = col("FinInstrmTp").is_some();
    // MUTANT m_v34i_bound_uses_close (data:F7 m_f7_bound_uses_close): the CLOSE read instead of the SETTLE
    let close = cfg!(feature = "m_v34i_bound_uses_close");
    let (ci, cs, ce, ck, cr, cst) = if udiff { (col("FinInstrmTp"), col("TckrSymb"), col("XpryDt"), col("StrkPric"), col("OptnTp"), col(if close { "ClsPric" } else { "SttlmPric" })) } else { (col("INSTRUMENT"), col("SYMBOL"), col("EXPIRY_DT"), col("STRIKE_PR"), col("OPTION_TYP"), col(if close { "CLOSE" } else { "SETTLE_PR" })) };
    let (Some(ci), Some(cs), Some(ce), Some(ck), Some(cr), Some(cst)) = (ci, cs, ce, ck, cr, cst) else { return Err(format!("{src}: no SETTLE column")) };
    let mut by: HashMap<String, HashMap<(u32, i64, u8), (i64, i64, i64)>> = HashMap::new();
    for l in lines {
        let f: Vec<&str> = l.split(',').collect();
        let g = |k: usize| f.get(k).map_or("", |x| x.trim());
        if !matches!(g(ci), "IDO" | "STO" | "OPTIDX" | "OPTSTK") {
            continue;
        }
        let right = match g(cr) {
            "CE" => 0,
            "PE" => 1,
            _ => continue,
        };
        let sym = g(cs);
        // a strike adjustment on the session: the base is adjusted (band unknown, data:F7 4.4)
        if ca.is_some_and(|c| c.contains(&(sym.to_owned(), day))) {
            continue;
        }
        let (Some(exp), Some(k), Some(st)) = (crate::bhav::ymd_of(g(ce)), crate::bhav::paisa(g(ck)), crate::bhav::paisa(g(cst))) else { continue };
        if st <= 0 {
            continue;
        }
        by.entry(sym.to_owned()).or_default().insert((exp, k, right), ((st - min_range).max(5), st + min_range, 5));
    }
    Ok(DayBand { by, exact: false, src: format!("no-file bound from {src} (settle +/- Rs {})", min_range / 100) })
}

/// The band of contract `ci` on its day: (lower, upper, tick, exact) or None (unknown: no source row; counted by the caller).
#[must_use]
pub fn band_of(c: &crate::cube::Cube, ci: usize) -> Option<(i64, i64, i64, bool)> {
    let k = c.cons.get(ci)?;
    let name = c.unds.get(usize::from(k.und))?.name.as_str();
    let db = match day_band(c.day) {
        Ok(x) => x?,
        Err(e) => {
            note_err(&e);
            return None;
        }
    };
    let sym = crate::freeze::nse_symbol(name, c.day);
    let r = db.by.get(sym.as_str()).and_then(|m| m.get(&(k.expiry, k.strike, k.right & 1))).copied()?;
    Some((r.0, r.1, r.2, db.exact))
}

/// The exact band (NSE's file) of contract `ci` for build:D-high's B1 clip on the R0 path (never the bound: a limit is never clipped to a
/// bound, data:F7 5.4).
#[must_use]
pub fn exact_band(c: &crate::cube::Cube, ci: usize) -> Option<(i64, i64, i64)> {
    OPR_T.get()?;
    band_of(c, ci).filter(|b| b.3).map(|b| (b.0, b.1, b.2))
}

/// A band source error (a sha256 or read failure): the first is kept and the run refuses at its end (never priced silently without).
pub static BAND_ERR: Mutex<Option<String>> = Mutex::new(None);

fn note_err(e: &str) {
    // under the kit's STRICT (a shard) every lookup failure is fatal before the next batch file
    crate::kit::strict_fail("F7 band source", e);
    if let Ok(mut g) = BAND_ERR.lock()
        && g.is_none()
    {
        *g = Some(e.to_owned());
    }
}

// ------------------------------------------------------------------------------------------------ the contract-day context (data:F7 5.2)

/// The readings, in the booking's tie order (data:F7 5.9: ties go to the reading earlier in R0, R10, R5, RM).
pub const RD_NAMES: [&str; 4] = ["R0", "R10", "R5", "RM"];

/// One contract-day's F7 context: its band (or bound, or none), whether NSE's LPP is live for its class, the dated tick, the readings the
/// session lists, the market-order cap's LPP clip, the session's first second.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ctx {
    pub band: Option<(i64, i64, i64, bool)>,
    pub lpp: bool,
    pub mpp_lpp: bool,
    /// NSE's cap on option market orders is live (from 2022-05-02); before it RM is an uncapped market order.
    pub mpp_nse: bool,
    pub tick: i64,
    /// Readings listed for the session, in [`RD_NAMES`] order (R0 always).
    pub rd: [bool; 4],
    pub open: i64,
}

/// The context of contract `ci` on its day (None with PROT off; an error when the dated table does not cover the session: never priced
/// without its rules).
///
/// # Errors
/// A session outside data:F7's dated table.
pub fn ctx_of(c: &crate::cube::Cube, ci: usize) -> Result<Option<Ctx>, String> {
    if !crate::int34::on(crate::int34::PROT) {
        return Ok(None);
    }
    let Some(k) = c.cons.get(ci) else { return Ok(None) };
    let seg = READINGS_T.get().and_then(|r| r.seg(c.day)).ok_or_else(|| format!("{}: outside data:F7's dated readings table: refused (never priced without its dated rules)", crate::cal::fmt_day(c.day)))?;
    // LPP by class (data:F7 2.3): stock options; index options expiring within three serial months (weekly + monthly) and the longer-dated rest
    let lpp = crate::int34::on(crate::int34::LPP)
        && if k.kind == 1 {
            seg.lpp_stk
        } else if crate::cal::day_of_ymd(k.expiry) - c.day <= 92 {
            seg.lpp_idx_wm
        } else {
            seg.lpp_idx_other
        };
    // a band SOURCE error (a file whose sha256 is not the manifest's, an unreadable file) refuses the run: never priced without its band
    let band = if crate::fix34::on(crate::fix34::BANDS) {
        match day_band(c.day) {
            Err(e) => {
                note_err(&e);
                return Err(e);
            }
            Ok(_) => band_of(c, ci),
        }
    } else {
        None
    };
    // the dated tick: the file's BidIntrvl (1 paisa for a stock option under Rs 250 from 2025-11-03), else 5 paise
    let tick = band.filter(|b| b.3).map_or(crate::trade::TICK, |b| b.2);
    Ok(Some(Ctx { band, lpp, mpp_lpp: seg.mpp_lpp && seg.mpp_nse, mpp_nse: seg.mpp_nse, tick, rd: [seg.r0, seg.r10, seg.r5, seg.rm], open: crate::cal::at_sod(c.day, crate::cal::OPEN_S) }))
}

impl Ctx {
    /// The crate's strict view of reading `rd` (1 = R10, 2 = R5, 3 = RM; R0 has none).
    #[must_use]
    pub fn strict(&self, rd: usize) -> Option<common_fill::Strict> {
        let law = match rd {
            1 => common_fill::ResendLaw::Protection { bp: 1_000, tick: self.tick },
            2 => common_fill::ResendLaw::Protection { bp: 500, tick: self.tick },
            3 if self.mpp_nse => common_fill::ResendLaw::NseMarket { lpp_clip: self.mpp_lpp },
            // RM before 2022-05-02: an exchange market order with no price cap (data:F7 5.2)
            3 => common_fill::ResendLaw::Uncapped,
            _ => return None,
        };
        let (up, dn, exact) = match self.band {
            Some((lo, hi, _, ex)) => (Some(hi), Some(lo), ex),
            None => (None, None, false),
        };
        Some(common_fill::Strict { open: self.open, up, dn, exact, lpp: self.lpp, law })
    }

    /// The strict sale (data:F7 5.8): R5's view (5% limit on the dated tick, the lower-band lock).
    #[must_use]
    pub fn sale_strict(&self) -> Option<common_fill::Strict> {
        self.strict(2)
    }
}

// ------------------------------------------------------------------------------------------------ counters, words, checklist row 5

/// The sales the strict rule refused, per distinct event (session, contract, decision second): (refused at T by NSE's LPP, some share level
/// R0 fills and R5 does not).
static SALE_SEEN: Mutex<BTreeMap<(i64, u32, u32), bool>> = Mutex::new(BTreeMap::new());

/// Notes one event's strict-sale refusal (`lpp` = refused at T by NSE's protection; else a share level R5 does not fill).
pub fn note_sale_refused(day: i64, sid: u32, t: u32, lpp: bool) {
    if let Ok(mut g) = SALE_SEEN.lock() {
        g.entry((day, sid, t)).or_insert(lpp);
    }
}

/// (events refused at T by NSE's LPP, events with a share level R0 fills but R5 refuses).
#[must_use]
pub fn sale_counts() -> (u64, u64) {
    SALE_SEEN.lock().map_or((0, 0), |g| (g.values().filter(|&&x| x).count() as u64, g.values().filter(|&&x| !x).count() as u64))
}

/// The per-trade column: the booked reading, R0's net beside and the cost, the booked paths' counters.
#[must_use]
pub fn trade_words(f: Option<&crate::trade::F7Trip>, net: i64) -> String {
    let Some(f) = f else { return "-".to_owned() };
    let rs = |v: i64| format!("{}{}.{:02}", if v < 0 { "-" } else { "" }, v.abs() / 100, v.abs() % 100);
    format!(
        "{} booked (R0 net Rs {}; cost of the F7 rules Rs {}); LPP rejected blocks {}, LPP reference unknown {}, band-locked seconds {}, band clips {}, units left by the 15:23 cut {}",
        RD_NAMES.get(usize::from(f.rd)).copied().unwrap_or("?"),
        rs(f.r0_net),
        rs(net - f.r0_net),
        f.lpp_rejected,
        f.lpp_unknown,
        f.locked_secs,
        f.band_clipped,
        f.cut_units
    )
}

/// The RESULTS lines (the dated table, the band source, the model, the counts).
#[must_use]
pub fn result_lines() -> Vec<String> {
    if !crate::int34::on(crate::int34::PROT) {
        return Vec::new();
    }
    let (lpp_t, lvl) = sale_counts();
    let mut v = vec![
        format!("the dated readings table {} (data:F7 5.1): per session the exit readings (R0 = the v3.4 path; R10 / R5 = Dhan's option protection at 10% / 5% on the dated tick, re-sent every block; RM = NSE's market-order cap LTP + max(20%, Rs 10), before 2026-04-01), the LPP classes, the minimum range, Dhan's API rate", readings_src()),
        format!("the band source: {} (NSE's exact begin-of-day band from 2024-02-06; before it the proven no-file bound settle(previous session) +/- the minimum range, never used to clip a limit, data:F7 4.4; strike-adjusted contracts {})", OPR_T.get().map_or_else(|| "(no manifest: no band, the strict readings lock at the session's running extremes only)".to_owned(), |o| o.src.clone()), CA_T.get().map_or_else(|| "not excluded (no --f7-ca)".to_owned(), |c| format!("excluded ({} (symbol, session) pairs, data:F3 ca-events)", c.len()))),
        "the model (data:F7 5.2-5.9, crate v2.6 CHANGE S1): every own buy-back and 15:10 square-off walked under each reading of its session, the DEAREST booked (the lowest net after every charge, the never-closed value and Dhan's fee), ties to the earlier of R0, R10, R5, RM; R0's net printed beside every trade; in the strict readings a re-send above NSE's High LPP (the lowest possible 30-second reference + Rs 20 / x 1.4) gets 0 capacity, a second whose every print sits at one price at or above the known band level (NSE's band, its bound, or the session's highest print so far) gives 0 volume, a limit is clipped to an exact band before any print passes it, and nothing is sent at or after 15:23:00 (the rest valued per ruling item 3 + Dhan's fee); the sale by the ONE strict rule R5 (5% limit, NSE's LPP at T, the clip to an exact lower band, the lower-band lock)".to_owned(),
        format!("strict sales refused (distinct events): by NSE's LPP at T {lpp_t}; with a share level R0 fills and R5 refuses {lvl} (sale_refused_by_f7; those trades are not taken)"),
    ];
    if FORCE_BOUND.load(std::sync::atomic::Ordering::Relaxed) {
        v.push("MEASUREMENT MODE --f7-force-bound on: the no-file bound used even where NSE's exact band file exists (data:F7 4.4: the bound's extra strictness); NOT a result".to_owned());
    }
    if let Ok(g) = BAND_ERR.lock()
        && let Some(e) = g.as_ref()
    {
        v.push(format!("BAND SOURCE ERROR (contract-days priced without a band): {e}"));
    }
    v.push("NOT MODELLED in this build (open, result-changing, named): the stock-option cooling-off cap of NSE/FAOP/64995 (data:F7 5.5: needs the underlying's cash prints per second) and Dhan's API order-rate budget (data:F7 5.7: FIFO delays in the account sweep)".to_owned());
    v
}

/// Checklist row 5 (market protection and price bands) under PROT: COVERED with the readings' evidence, NOT COVERED while a band source error
/// stands; the two parts not modelled are named.
#[must_use]
pub fn item5(state: &'static str) -> (&'static str, String) {
    let _ = state;
    let err = BAND_ERR.lock().map_or(true, |g| g.is_some());
    // the row's state follows THIS evidence (the earlier words described the build before data:F7's model): NOT COVERED on a band source error
    // or without a band source; PARTIAL while two parts of data:F7's model are not built (said by name)
    let src = crate::fix34::on(crate::fix34::BANDS) && (OPR_T.get().is_some() || crate::fix34::BAND_T.get().is_some());
    let st = if err || !src { "NOT COVERED" } else { "PARTIAL" };
    (st, format!("; integrate:D F7 (data:F7 F7-BANDS-PROTECTION.md 5): the dated readings R0 / R10 / R5 / RM booked at the dearest per trade (column f7_booked_reading), NSE's LPP, the band lock at the known level (NSE's band, its proven no-file bound, or the session's running extreme) and the 15:23 cut in the strict readings, the sale by R5 (tests tests34i v34i_f7_*, v34i_xm_*); band source: {}; PARTIAL: the stock-option cooling-off cap (5.5) and Dhan's order-rate budget (5.7) are NOT modelled (open, named)", OPR_T.get().map_or_else(|| "(none)".to_owned(), |o| o.src.clone())))
}

/// A MEASUREMENT mode (`--f7-force-bound on`): the no-file bound read even where NSE's exact band file exists (data:F7 4.4: how much stricter
/// the bound is than the truth, on the Rs 3-era sessions that have files). Never in a launch profile; named in the fingerprint and RESULTS.
pub static FORCE_BOUND: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The sha256 of every F7 table read (readings, manifest, ca): the fingerprint names them by content (`int34::fp_part`).
pub static SHAS: Mutex<BTreeMap<&'static str, String>> = Mutex::new(BTreeMap::new());

fn note_sha(k: &'static str, p: &str) -> String {
    let s = crate::cube::sha256(std::path::Path::new(p));
    if let Ok(mut g) = SHAS.lock() {
        g.insert(k, s.clone());
    }
    s
}
