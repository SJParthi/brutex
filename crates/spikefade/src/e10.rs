//! v3.4-e10 (build:shard-memory, Mon 16 Nov 2026; v34/shard-mem/DECISION.md = design-ship E10 + grafts, with EVERY MUST M1-M8 of
//! v34/shard-mem/CRITIC.md): SHIP THE SET-UP. The Mac computes a segment's set-up ONCE (a streamed writer that is also the plan run) and
//! stores it as a digest-bound STORE; every cloud shard reads the store and walks its batches on a ONE-DAY WINDOW of events, so no shard
//! ever holds (or computes) the every-year set-up (MEASURED 27,999 MiB on the Mac for the index search; a box allows 13,674 MiB).
//!
//! Switches (each OFF by default: with none on, this binary is the shard kit 74b675db byte for byte, proof P-E0):
//! - R1 BHAV_LRU (`--bhav-cache N`): the bhavcopy memo keeps at most N parsed sessions (least recently used out); a miss re-parses the same
//!   sha-checked file (a pure function of it).
//! - R2 LV_STREAM (`--levels-stream on`, needs LV_CAUSAL): pass 0 keeps the sorted samples of the last W present days only and computes each
//!   day's causal levels when it arrives (the same window `fix33::causal_levels` reads).
//! - SHIP_W (`--write-setup DIR` with `--write-plan PLAN`): the streamed writer. Per segment day ONE section = the day's event records AND
//!   every config's day list (local event ids); each section is decoded back and compared before it is kept; the list stage, the unchanged
//!   F5 list digest (v1), the cut and the costs are computed over the stored form; the plan gets a SETUP line = v2.
//! - SHIP_R (`--setup-from DIR`, a shard only): no pass-0 samples, no readings, no list build; the store is verified (v2, every section's
//!   sha256 E_d and decoded-record digest L_d, the members M_u of every list the shard walks), then each batch-day's window is decoded,
//!   guarded (M2 + G1) and swapped into the segment state for stage A and the walk; the parts the walk received are hashed (M3).
//! - SHIP_AUDIT (`--setup-audit on`, Mac proofs only): the in-memory set-up is ALSO built and every decoded window compared with it.
//!
//! No event id reaches a batch body (CRITIC 1): inside a day the window renames ids by a constant (global = local + e0), which keeps every
//! index, in-day sort key and D2-1 class key of the batch path.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::fmt::Write as _;
use std::io::{Read as _, Seek as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::acct::SEv;
use crate::kit::{Sha256, hex, sha256_bytes};
use crate::menu::Rd;
use crate::mtable::MarginAt;
use crate::read::liq_pass;
use crate::report3::Seg3;
use crate::screen::Levels;
use crate::stage::EvX;
use crate::trade::Refuse;

// ------------------------------------------------------------------------------------------------ switches and counters

/// SHIP_W: `--write-setup DIR`.
pub static WRITE: OnceLock<PathBuf> = OnceLock::new();
/// SHIP_R: `--setup-from DIR`.
pub static FROM: OnceLock<PathBuf> = OnceLock::new();
/// SHIP_R: the verified store of this run (set once the store is opened).
pub static SHIP: OnceLock<Ship> = OnceLock::new();
/// SHIP_AUDIT (`--setup-audit on`).
static AUDIT: AtomicBool = AtomicBool::new(false);
/// `--setup-recompute on`: a Mac-proof shard that recomputes the set-up although its plan carries a SETUP line (CRITIC S2).
static RECOMPUTE: AtomicBool = AtomicBool::new(false);
/// R2 LV_STREAM (`--levels-stream on`).
pub static LV_STREAM: AtomicBool = AtomicBool::new(false);
/// SHIP_R: counted violations (M2 window shape / identity, G1 out-of-window ids, M3 walked parts, G2 audit differences); any voids the
/// next batch file (and the shard's end).
pub static VIOL: AtomicU64 = AtomicU64::new(0);
static VIOL_WHY: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// SHIP_R: windows decoded, their summed decode nanoseconds (MEASURED, RUN.log only).
pub static DECODES: AtomicU64 = AtomicU64::new(0);
pub static DECODE_NS: AtomicU64 = AtomicU64::new(0);
/// G2: windows audited and the differences found.
pub static AUDITED: AtomicU64 = AtomicU64::new(0);

/// The new runtime-only flags (left out of the argument digest: they never change a result, proofs P-E0 / P-E1 / P-E2).
pub const RUNTIME: [&str; 6] = ["--bhav-cache", "--levels-stream", "--write-setup", "--setup-from", "--setup-audit", "--setup-recompute"];

/// The SHIP_W folder, when the writer runs.
#[must_use]
pub fn write_dir() -> Option<&'static PathBuf> {
    WRITE.get()
}

/// The SHIP_R folder, when this run is a store-reading shard.
#[must_use]
pub fn from_dir() -> Option<&'static PathBuf> {
    FROM.get()
}

/// R2 on.
#[must_use]
pub fn lv_stream() -> bool {
    LV_STREAM.load(Ordering::Relaxed)
}

/// G2 on.
#[must_use]
pub fn audit() -> bool {
    AUDIT.load(Ordering::Relaxed)
}

/// Counts one violation (at most 20 reasons kept).
pub fn violation(why: &str) {
    VIOL.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut v) = VIOL_WHY.lock()
        && v.len() < 20
    {
        v.push(why.to_owned());
    }
}

/// Err when a violation was counted (the batch file is not written; the shard is void).
///
/// # Errors
/// The violations, by name.
pub fn strict_check(at: &str) -> Result<(), String> {
    let n = VIOL.load(Ordering::Relaxed);
    if n > 0 {
        let why = VIOL_WHY.lock().map(|v| v.join(" | ")).unwrap_or_default();
        return Err(format!("E10 WINDOW GUARD at {at}: {n} violation(s) of the shipped set-up (M2 / G1 / M3 / G2): the batch is void: {why}"));
    }
    Ok(())
}

fn flag_on(a: &[String], k: &str) -> Result<bool, String> {
    match a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str) {
        None | Some("off") => Ok(false),
        Some("on") => Ok(true),
        Some(v) => Err(format!("{k} on|off, not {v}")),
    }
}

/// Reads the E10 flags of `run3` (after `kit::from_args`, `fix33::from_args` and `sw33::from_args`).
///
/// # Errors
/// A malformed value or a combination E10 refuses (by name).
pub fn from_args(a: &[String]) -> Result<Vec<String>, String> {
    let get = |k: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str);
    let has = |k: &str| a.iter().any(|x| x == k);
    let mut notes = Vec::new();
    if let Some(v) = get("--bhav-cache") {
        let n = v.parse::<usize>().map_err(|_| format!("--bhav-cache {v}"))?;
        if n < 2 {
            return Err(format!("--bhav-cache {v}: at least 2 sessions (R1)"));
        }
        crate::bhav::CAP.store(n, Ordering::Relaxed);
        notes.push(format!("E10 R1 BHAV_LRU: at most {n} parsed bhavcopy sessions held (least recently used out; exact: a re-parse of the same sha-checked file)"));
    }
    if flag_on(a, "--levels-stream")? {
        if !crate::fix33::on(crate::fix33::LV_CAUSAL) {
            return Err("--levels-stream on needs LV_CAUSAL (--levels causal or --fix33): it streams the causal window".to_owned());
        }
        LV_STREAM.store(true, Ordering::Relaxed);
        notes.push("E10 R2 LV_STREAM: pass 0 keeps the last W present days' samples only (exact: the same causal window)".to_owned());
    }
    let audit = flag_on(a, "--setup-audit")?;
    let recompute = flag_on(a, "--setup-recompute")?;
    let w = get("--write-setup");
    let r = get("--setup-from");
    if w.is_some() && r.is_some() {
        return Err("--write-setup and --setup-from: a run writes a store or reads one, never both".to_owned());
    }
    if (w.is_some() || r.is_some()) && a.iter().any(|x| x.to_ascii_lowercase().contains("positional")) {
        // CRITIC M8 / DECISION R-15: the one-day window cannot carry a position across days
        return Err("E10 refuses a POSITIONAL run by name (Addendum 36 / R-15: the one-day window cannot carry positions across days; a carry design is needed first)".to_owned());
    }
    let one_seg = matches!(get("--segs"), Some("idx" | "stk"));
    let streamed = has("--day-major") && !has("--day-major-inmem");
    let derived = crate::sw33::on(crate::sw33::DERIVED);
    if let Some(d) = w {
        if crate::kit::WRITE_PLAN.get().is_none() || crate::kit::PLAN.get().is_some() || crate::kit::on(crate::kit::SHARD) {
            return Err("--write-setup DIR needs --write-plan PLAN (it IS the plan run) and refuses --batch-plan / --shard".to_owned());
        }
        if !crate::kit::on(crate::kit::STRICT) || !one_seg || !streamed || !derived {
            return Err("--write-setup needs --strict on, ONE segment (--segs idx|stk), the streamed --day-major and the v3.3 DERIVED lists (--v33)".to_owned());
        }
        let dir = PathBuf::from(d);
        if dir.exists() || part_of(&dir).exists() {
            return Err(format!("--write-setup {d}: the folder or its .part exists (a store is never overwritten; move it aside): refused"));
        }
        let _ = WRITE.set(dir);
        notes.push(format!("E10 SHIP_W: the set-up is written to the store {d} (sections decoded back before they are kept)"));
    }
    if let Some(d) = r {
        let planned_setup = crate::kit::PLAN.get().is_some_and(|p| !p.setup.is_empty());
        if !crate::kit::on(crate::kit::SHARD) || !planned_setup || crate::kit::WRITE_PLAN.get().is_some() {
            return Err("--setup-from DIR needs --shard on and a --batch-plan whose SETUP line names the store (and refuses --write-plan)".to_owned());
        }
        if !one_seg || !streamed || !derived || !crate::sw33::any() {
            return Err("--setup-from needs ONE segment, the streamed --day-major and the v3.3 DERIVED lists (--v33)".to_owned());
        }
        for x in ["--setup-only", "--hand", "--hand-only", "--speed-probe", "--cost-only", "--compare-day-major", "--compare-skip", "--dedup-ratio"] {
            if has(x) {
                return Err(format!("--setup-from: {x} reads the whole in-memory set-up a store-reading shard never holds: refused"));
            }
        }
        let _ = FROM.set(PathBuf::from(d));
        notes.push(format!("E10 SHIP_R: the set-up is READ from the store {d} (no pass-0 samples, no readings, no list build); one-day windows"));
    }
    if audit && r.is_none() {
        return Err("--setup-audit on needs --setup-from (it compares the store's windows with the in-memory set-up)".to_owned());
    }
    // CRITIC S2: a shard never recomputes a set-up whose plan names a store (unless a Mac proof says so)
    let setup_plan = crate::kit::PLAN.get().is_some_and(|p| !p.setup.is_empty());
    s2_check(crate::kit::on(crate::kit::SHARD), setup_plan, r.is_some(), recompute)?;
    if recompute {
        notes.push("E10: --setup-recompute on (a Mac-proof shard recomputing the set-up of a SETUP plan)".to_owned());
    }
    AUDIT.store(audit, Ordering::Relaxed);
    RECOMPUTE.store(recompute, Ordering::Relaxed);
    Ok(notes)
}

/// CRITIC S2: a shard whose plan names a store must READ it (`--setup-from`); only a Mac proof may ask to recompute it.
///
/// # Errors
/// A shard that would rebuild the every-year set-up under a SHIP_R memory gate.
pub fn s2_check(shard: bool, setup_plan: bool, setup_from: bool, recompute: bool) -> Result<(), String> {
    // MUTANT m_e10_recompute_allowed: a shard with a SETUP plan and no --setup-from recomputes the whole set-up
    if shard && setup_plan && !setup_from && !recompute && !cfg!(feature = "m_e10_recompute_allowed") {
        return Err("--shard on with a plan that carries a SETUP line needs --setup-from STORE (CRITIC S2: a box never rebuilds the every-year set-up under a SHIP_R memory gate)".to_owned());
    }
    Ok(())
}

/// `DIR.part` of a store folder.
#[must_use]
pub fn part_of(dir: &Path) -> PathBuf {
    PathBuf::from(format!("{}.part", dir.display()))
}

// ------------------------------------------------------------------------------------------------ the record codec (v1)

/// The decode-controlling flags of a store (M1: bound into v2 and derived by every shard from its own switches).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    /// Every event carries its 60-s volume (LIQ14).
    pub vol60: bool,
    /// Every event carries its exact margin at T (MARGIN_EXACT with a margin manifest).
    pub mx: bool,
}

impl Flags {
    /// The SETUP.tsv line.
    #[must_use]
    pub fn line(self) -> String {
        format!("FLAGS\tvol60={}\tmx={}", u8::from(self.vol60), u8::from(self.mx))
    }

    const fn bits(self) -> u8 {
        (self.vol60 as u8) | ((self.mx as u8) << 1)
    }
}

/// M1: the flags THIS run's switches imply (`mx_manifest` = the exact margin with its manifest, as `cmd_run3` builds `mman`).
#[must_use]
pub fn own_flags(mx_manifest: bool) -> Flags {
    Flags { vol60: crate::sw::on(crate::sw::LIQ14), mx: mx_manifest }
}

/// CRITIC S9: the theta refusal codes the readings produce (read.rs 407 / 412 / 450 / 455 / 459; menu.rs 611's default).
pub const THETA_CODES: [&str; 2] = ["theta_none", "theta_no_calendar"];

const fn refuse_code(r: Refuse) -> u8 {
    // CRITIC S9: exhaustive, no `_` arm: a variant added later fails to compile here
    match r {
        Refuse::None => 0,
        Refuse::NoUnderlying => 1,
        Refuse::NoMarginRow => 2,
        Refuse::StockExpiryDay => 3,
        Refuse::StockExpiryNoNext => 4,
        Refuse::AfterCut => 5,
        Refuse::BadTape => 6,
        Refuse::NoReference => 7,
        Refuse::NoMarginFile => 8,
        Refuse::NoSpanRow => 9,
        Refuse::NoElmRate => 10,
        Refuse::FnoBan => 11,
        Refuse::ChargesRate => 12,
        Refuse::BadLot => 13,
    }
}

fn refuse_of(c: u8) -> Result<Refuse, String> {
    Ok(match c {
        0 => Refuse::None,
        1 => Refuse::NoUnderlying,
        2 => Refuse::NoMarginRow,
        3 => Refuse::StockExpiryDay,
        4 => Refuse::StockExpiryNoNext,
        5 => Refuse::AfterCut,
        6 => Refuse::BadTape,
        7 => Refuse::NoReference,
        8 => Refuse::NoMarginFile,
        9 => Refuse::NoSpanRow,
        10 => Refuse::NoElmRate,
        11 => Refuse::FnoBan,
        12 => Refuse::ChargesRate,
        13 => Refuse::BadLot,
        o => return Err(format!("record: unknown refusal code {o}")),
    })
}

fn theta_code(s: &str) -> Result<u8, String> {
    match s {
        "theta_none" => Ok(1),
        "theta_no_calendar" => Ok(2),
        o => Err(format!("record: theta code {o:?} has no store code (CRITIC S9: the codec refuses, never truncates)")),
    }
}

fn theta_of(c: u8) -> Result<&'static str, String> {
    match c {
        1 => Ok("theta_none"),
        2 => Ok("theta_no_calendar"),
        o => Err(format!("record: unknown theta code {o}")),
    }
}

fn put_var(v: &mut Vec<u8>, mut x: u64) {
    while x >= 0x80 {
        v.push((x as u8) | 0x80);
        x >>= 7;
    }
    v.push(x as u8);
}

const fn zz(x: i64) -> u64 {
    ((x << 1) ^ (x >> 63)) as u64
}

const fn unzz(x: u64) -> i64 {
    ((x >> 1) as i64) ^ -((x & 1) as i64)
}

/// A bounds-checked reader of a section.
struct Cur<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cur<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self.b.get(self.p..self.p + n).ok_or_else(|| format!("section: short at byte {} (wanted {n})", self.p))?;
        self.p += n;
        Ok(s)
    }
    fn arr<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let s = self.take(N)?;
        let mut a = [0_u8; N];
        a.copy_from_slice(s);
        Ok(a)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.arr::<1>()?[0])
    }
    fn bool(&mut self) -> Result<bool, String> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            o => Err(format!("section: bool byte {o}")),
        }
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.arr()?))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.arr()?))
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.arr()?))
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.arr()?))
    }
    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.arr()?))
    }
    fn u128(&mut self) -> Result<u128, String> {
        Ok(u128::from_le_bytes(self.arr()?))
    }
    fn var(&mut self) -> Result<u64, String> {
        let mut x = 0_u64;
        let mut s = 0_u32;
        loop {
            let c = self.u8()?;
            if s >= 64 {
                return Err("section: varint overflow".to_owned());
            }
            x |= u64::from(c & 0x7f) << s;
            if c & 0x80 == 0 {
                return Ok(x);
            }
            s += 7;
        }
    }
}

/// Encodes one event record (every field of every struct NAMED: a field added later fails to compile here, CRITIC S9 / DECISION 4.3(e)).
///
/// # Errors
/// A value the codec cannot carry (an unknown theta code, more than 255 timeframe windows, a flagged field missing or present unflagged).
#[allow(clippy::too_many_arguments)]
pub fn enc_rec(out: &mut Vec<u8>, ev: &SEv, liq: (u32, u32), tfmask: u16, rd: &Rd, tfw: &[(u8, i64, i64)], vol60: Option<u64>, mx: Option<&MarginAt>, fl: Flags) -> Result<(), String> {
    // the stage A table `tab` is the set-up's default here (the decode-back compares the v1 line, which prints it)
    let SEv { di, ci, t, skey, tab: _ } = ev;
    out.extend_from_slice(&di.to_le_bytes());
    out.extend_from_slice(&ci.to_le_bytes());
    out.extend_from_slice(&t.to_le_bytes());
    out.extend_from_slice(&skey.to_le_bytes());
    out.extend_from_slice(&liq.0.to_le_bytes());
    out.extend_from_slice(&liq.1.to_le_bytes());
    out.extend_from_slice(&tfmask.to_le_bytes());
    let Rd { atr1, atr5, hwin, rise, v300, theta } = *rd;
    for o in [atr1, atr5] {
        match o {
            None => out.push(0),
            Some(x) => {
                out.push(1);
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
    }
    out.extend_from_slice(&hwin.to_le_bytes());
    out.extend_from_slice(&rise.to_le_bytes());
    // MUTANT m_e10_rec_drops_v300: the 300-s reading never stored (decoded as unready)
    match v300.filter(|_| !cfg!(feature = "m_e10_rec_drops_v300")) {
        None => out.push(0),
        Some((a, b)) => {
            out.push(1);
            out.extend_from_slice(&a.to_le_bytes());
            out.extend_from_slice(&b.to_le_bytes());
        }
    }
    match theta {
        Ok((a, b)) => {
            out.push(0);
            out.extend_from_slice(&a.to_le_bytes());
            out.extend_from_slice(&b.to_le_bytes());
        }
        Err(s) => out.push(theta_code(s)?),
    }
    let k = u8::try_from(tfw.len()).map_err(|_| format!("record: {} timeframe windows (the codec carries at most 255)", tfw.len()))?;
    out.push(k);
    for &(a, b, c) in tfw {
        out.push(a);
        out.extend_from_slice(&b.to_le_bytes());
        out.extend_from_slice(&c.to_le_bytes());
    }
    match (fl.vol60, vol60) {
        (true, Some(v)) => out.extend_from_slice(&v.to_le_bytes()),
        (false, None) => {}
        _ => return Err("record: the 60-s volume does not match FLAGS vol60".to_owned()),
    }
    match (fl.mx, mx) {
        (true, Some(m)) => enc_mx(out, m),
        (false, None) => {}
        _ => return Err("record: the exact margin does not match FLAGS mx".to_owned()),
    }
    Ok(())
}

fn enc_mx(out: &mut Vec<u8>, m: &MarginAt) {
    let MarginAt { refuse, lot_exact, parts, snap, stale, fallback, addon, fut_fallback, idxc, elm2 } = *m;
    out.push(refuse.map_or(255, refuse_code));
    out.extend_from_slice(&lot_exact.to_le_bytes());
    let common_fill::margin::Parts { span, premium, elm, exact, nse_only, rate_h, otm } = parts;
    for x in [span, premium, elm, exact, nse_only, rate_h] {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out.push(u8::from(otm));
    out.push(snap);
    out.push(u8::from(stale));
    out.push(u8::from(fallback));
    out.extend_from_slice(&addon.to_le_bytes());
    out.push(u8::from(fut_fallback));
    out.push(idxc);
    out.extend_from_slice(&elm2.to_le_bytes());
}

fn dec_mx(c: &mut Cur<'_>) -> Result<MarginAt, String> {
    let rc = c.u8()?;
    let refuse = if rc == 255 { None } else { Some(refuse_of(rc)?) };
    let lot_exact = c.i64()?;
    let (span, premium, elm, exact, nse_only, rate_h) = (c.i64()?, c.i64()?, c.i64()?, c.i64()?, c.i64()?, c.i64()?);
    let otm = c.bool()?;
    let parts = common_fill::margin::Parts { span, premium, elm, exact, nse_only, rate_h, otm };
    let snap = c.u8()?;
    let stale = c.bool()?;
    let fallback = c.bool()?;
    let addon = c.i64()?;
    let fut_fallback = c.bool()?;
    let idxc = c.u8()?;
    let elm2 = c.i32()?;
    Ok(MarginAt { refuse, lot_exact, parts, snap, stale, fallback, addon, fut_fallback, idxc, elm2 })
}

/// One decoded record.
pub type Rec = (SEv, (u32, u32), u16, Rd, Vec<(u8, i64, i64)>, Option<u64>, Option<MarginAt>);

fn dec_rec(c: &mut Cur<'_>, fl: Flags) -> Result<Rec, String> {
    let (di, ci, t, skey) = (c.u16()?, c.u32()?, c.u32()?, c.u32()?);
    let ev = SEv { di, ci, t, skey, tab: crate::trade::EvT::default() };
    let liq = (c.u32()?, c.u32()?);
    let tfmask = c.u16()?;
    let opt = |c: &mut Cur<'_>| -> Result<Option<i64>, String> {
        match c.u8()? {
            0 => Ok(None),
            1 => Ok(Some(c.i64()?)),
            o => Err(format!("record: option tag {o}")),
        }
    };
    let atr1 = opt(c)?;
    let atr5 = opt(c)?;
    let hwin = c.i64()?;
    let rise = c.i64()?;
    let v300 = match c.u8()? {
        0 => None,
        1 => Some((c.u128()?, c.u64()?)),
        o => return Err(format!("record: v300 tag {o}")),
    };
    let theta = match c.u8()? {
        0 => Ok((c.i64()?, c.i64()?)),
        k => Err(theta_of(k)?),
    };
    let rd = Rd { atr1, atr5, hwin, rise, v300, theta };
    let k = usize::from(c.u8()?);
    let mut tfw = Vec::with_capacity(k);
    for _ in 0..k {
        tfw.push((c.u8()?, c.i64()?, c.i64()?));
    }
    let vol60 = if fl.vol60 { Some(c.u64()?) } else { None };
    let mx = if fl.mx { Some(dec_mx(c)?) } else { None };
    Ok((ev, liq, tfmask, rd, tfw, vol60, mx))
}

/// The v1 record line of one event: EXACTLY the line `kit::list_digest` hashes for it (its Debug forms, the getters' Options included).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn v1_line(ev: Option<&SEv>, liq: Option<&(u32, u32)>, tfmask: Option<&u16>, rd: Option<&Rd>, tfw: Option<&Vec<(u8, i64, i64)>>, vol60: Option<&u64>, mx: Option<&MarginAt>) -> String {
    format!("{ev:?}|{liq:?}|{tfmask:?}|{rd:?}|{tfw:?}|{vol60:?}|{mx:?}\n")
}

// ------------------------------------------------------------------------------------------------ one day's section

const MAGIC: &[u8; 4] = b"SFE2";

/// One segment day's set-up as `prepare_day` produced it (event ids local to the day: global = e0 + local).
#[derive(Clone, Debug, Default)]
pub struct DayIn {
    pub di: usize,
    pub evs: Vec<SEv>,
    pub liq: Vec<(u32, u32)>,
    pub tfmask: Vec<u16>,
    pub rd: Vec<Rd>,
    pub tfw: Vec<Vec<(u8, i64, i64)>>,
    pub vol60: Vec<u64>,
    pub mx: Vec<MarginAt>,
    /// Per config id: the day's list as local event ids, in list order.
    pub lists: Vec<Vec<u32>>,
    pub prepared: usize,
}

impl DayIn {
    fn line(&self, i: usize) -> String {
        v1_line(self.evs.get(i), self.liq.get(i), self.tfmask.get(i), self.rd.get(i), self.tfw.get(i), self.vol60.get(i), self.mx.get(i))
    }
}

/// Encodes one day's section (header, records, the per-config day lists) after checking its shape and key order.
///
/// # Errors
/// A shape the store cannot carry or a list out of key order (CRITIC S10).
pub fn enc_section(d: &DayIn, fl: Flags, ncfg: usize) -> Result<Vec<u8>, String> {
    let n = d.evs.len();
    let n32 = u32::try_from(n).map_err(|_| format!("day {}: {n} events", d.di))?;
    let di = u16::try_from(d.di).map_err(|_| format!("day index {}", d.di))?;
    let want = |len: usize, on: bool| if on { len == n } else { len == 0 };
    if d.liq.len() != n || d.tfmask.len() != n || d.rd.len() != n || d.tfw.len() != n || !want(d.vol60.len(), fl.vol60) || !want(d.mx.len(), fl.mx) || d.lists.len() > ncfg {
        return Err(format!("day {}: per-event arrays of another length than its {n} events (or more than {ncfg} config lists): refused", d.di));
    }
    let mut out = Vec::with_capacity(16 + n * 260);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&di.to_le_bytes());
    out.extend_from_slice(&n32.to_le_bytes());
    out.push(fl.bits());
    for (i, e) in d.evs.iter().enumerate() {
        if e.di != di {
            return Err(format!("day {}: event {i} carries day {}", d.di, e.di));
        }
        let (Some(&liq), Some(&tm), Some(rd), Some(tfw)) = (d.liq.get(i), d.tfmask.get(i), d.rd.get(i), d.tfw.get(i)) else { return Err("record arrays".to_owned()) };
        enc_rec(&mut out, e, liq, tm, rd, tfw, d.vol60.get(i).copied(), d.mx.get(i), fl)?;
    }
    let used: Vec<(usize, &Vec<u32>)> = d.lists.iter().enumerate().filter(|(_, l)| !l.is_empty()).collect();
    out.extend_from_slice(&u32::try_from(used.len()).map_err(|_| "configs")?.to_le_bytes());
    let mut payload: Vec<u8> = Vec::new();
    let mut index: Vec<u8> = Vec::with_capacity(used.len() * 10);
    for (cfg, l) in &used {
        let start = payload.len();
        let mut prev = 0_i64;
        let mut last_t = 0_u32;
        for (k, &x) in l.iter().enumerate() {
            let t = d.evs.get(x as usize).map(|e| e.t).ok_or_else(|| format!("day {}: config {cfg} lists event {x} of {n}", d.di))?;
            // CRITIC S10: the stored day blocks equal `ul_day`'s partition-point slices only on key-sorted lists
            if k > 0 && t < last_t {
                return Err(format!("day {}: config {cfg}'s list is not in key order (second {t} after {last_t}): refused (CRITIC S10)", d.di));
            }
            last_t = t;
            // MUTANT m_e10_cfg_unsigned_delta: the delta stored without its sign (a list whose ids go down decodes wrongly)
            let delta = i64::from(x) - prev;
            if cfg!(feature = "m_e10_cfg_unsigned_delta") {
                put_var(&mut payload, delta.unsigned_abs());
            } else {
                put_var(&mut payload, zz(delta));
            }
            prev = i64::from(x);
        }
        index.extend_from_slice(&u16::try_from(*cfg).map_err(|_| "config id")?.to_le_bytes());
        index.extend_from_slice(&u32::try_from(l.len()).map_err(|_| "list")?.to_le_bytes());
        index.extend_from_slice(&u32::try_from(payload.len() - start).map_err(|_| "list bytes")?.to_le_bytes());
    }
    out.extend_from_slice(&index);
    out.extend_from_slice(&payload);
    Ok(out)
}

/// One decoded day: the window a shard swaps into the segment state for stage A and the walk (event ids local to the day).
#[derive(Clone, Debug, Default)]
pub struct Window {
    pub di: usize,
    pub e0: u64,
    pub n: u32,
    pub evs: Vec<SEv>,
    pub ex: Vec<EvX>,
    pub rd: Vec<Rd>,
    pub tfw: Vec<Vec<(u8, i64, i64)>>,
    pub mx: Vec<MarginAt>,
    pub vol60: Vec<u64>,
    pub liq: Vec<(u32, u32)>,
    pub tfmask: Vec<u16>,
    pub counted: Vec<bool>,
    /// (config, count, payload start, payload end) per config with entries, ascending config.
    idx: Vec<(u32, u32, usize, usize)>,
    raw: Vec<u8>,
}

impl Window {
    /// The v1 line of local event `i`.
    #[must_use]
    pub fn line(&self, i: usize) -> String {
        v1_line(self.evs.get(i), self.liq.get(i), self.tfmask.get(i), self.rd.get(i), self.tfw.get(i), self.vol60.get(i), self.mx.get(i))
    }

    /// The configs with entries this day.
    pub fn cfgs(&self) -> impl Iterator<Item = u32> + '_ {
        self.idx.iter().map(|x| x.0)
    }

    /// Config `cfg`'s day list (local ids, list order) into `out` (cleared first; empty when the config has no entry this day).
    ///
    /// # Errors
    /// A malformed list (bad varint, a count or byte length that does not match, an id outside the day).
    pub fn cfg_day(&self, cfg: u32, out: &mut Vec<u32>) -> Result<(), String> {
        out.clear();
        let Ok(k) = self.idx.binary_search_by_key(&cfg, |x| x.0) else { return Ok(()) };
        let &(_, cnt, a, b) = self.idx.get(k).ok_or("index")?;
        let mut c = Cur { b: self.raw.get(a..b).ok_or("list payload")?, p: 0 };
        let mut prev = 0_i64;
        for _ in 0..cnt {
            let x = prev + unzz(c.var()?);
            if x < 0 || x >= i64::from(self.n) {
                return Err(format!("day {}: config {cfg} lists local id {x} of {}", self.di, self.n));
            }
            out.push(x as u32);
            prev = x;
        }
        if c.p != c.b.len() {
            return Err(format!("day {}: config {cfg}'s list has {} trailing byte(s)", self.di, c.b.len() - c.p));
        }
        Ok(())
    }

    /// Swaps the window's per-event arrays with the segment state's (in for stage A and the walk; again to take them back).
    pub fn swap(&mut self, sr: &mut Seg3) {
        std::mem::swap(&mut self.evs, &mut sr.evs);
        std::mem::swap(&mut self.ex, &mut sr.ex);
        std::mem::swap(&mut self.rd, &mut sr.rd);
        std::mem::swap(&mut self.tfw, &mut sr.tfw);
        std::mem::swap(&mut self.mx, &mut sr.mx);
        std::mem::swap(&mut self.vol60, &mut sr.vol60);
        std::mem::swap(&mut self.liq, &mut sr.liq);
        std::mem::swap(&mut self.tfmask, &mut sr.tfmask);
        std::mem::swap(&mut self.counted, &mut sr.counted);
    }
}

/// Decodes one raw section (exhaustive: a bad magic, a short section, an unknown tag or a trailing byte is refused).
///
/// # Errors
/// As above.
pub fn dec_section(raw: Vec<u8>, fl: Flags, e0: u64) -> Result<Window, String> {
    let mut c = Cur { b: &raw, p: 0 };
    if c.take(4)? != MAGIC {
        return Err("section: bad magic (not SFE2)".to_owned());
    }
    let di = c.u16()?;
    let n = c.u32()?;
    if c.u8()? != fl.bits() {
        return Err(format!("day {di}: the section's flags differ from FLAGS"));
    }
    let nn = n as usize;
    let mut w = Window { di: usize::from(di), e0, n, ..Window::default() };
    w.evs.reserve(nn);
    for i in 0..nn {
        let (ev, liq, tm, rd, tfw, v, m) = dec_rec(&mut c, fl)?;
        if ev.di != di {
            return Err(format!("day {di}: record {i} carries day {}", ev.di));
        }
        w.evs.push(ev);
        w.liq.push(liq);
        w.tfmask.push(tm);
        w.rd.push(rd);
        w.tfw.push(tfw);
        if let Some(v) = v {
            w.vol60.push(v);
        }
        if let Some(m) = m {
            w.mx.push(m);
        }
    }
    w.ex = vec![EvX::default(); nn];
    w.counted = vec![false; nn];
    let nc = c.u32()? as usize;
    let mut idx = Vec::with_capacity(nc);
    let mut tot = 0_usize;
    let mut last: Option<u32> = None;
    for _ in 0..nc {
        let (cfg, cnt, nb) = (u32::from(c.u16()?), c.u32()?, c.u32()? as usize);
        if last.is_some_and(|l| cfg <= l) || cnt == 0 {
            return Err(format!("day {di}: config index not ascending or an empty entry at config {cfg}"));
        }
        last = Some(cfg);
        idx.push((cfg, cnt, tot, tot + nb));
        tot += nb;
    }
    let base = c.p;
    if base + tot != raw.len() {
        return Err(format!("day {di}: the list payload is {} bytes, the index says {tot} (short or trailing bytes)", raw.len().saturating_sub(base)));
    }
    w.idx = idx.into_iter().map(|(a, b, s, e)| (a, b, base + s, base + e)).collect();
    w.raw = raw;
    Ok(w)
}

/// One day encoded, decoded back, compared, compressed and hashed (a worker thread's output; nothing of it depends on another day).
#[derive(Debug, Default)]
pub struct DaySec {
    pub di: usize,
    pub n: u32,
    pub raw_len: u64,
    pub frame: Vec<u8>,
    pub e_d: String,
    pub l_d: String,
    /// The day's v1 event lines (from the ORIGINAL structs), concatenated, and their ends.
    pub lines: Vec<u8>,
    pub ends: Vec<usize>,
    pub liq: Vec<(u32, u32)>,
    pub lists: Vec<Vec<u32>>,
    pub nent: u64,
    pub unready: [u64; 4],
    pub prepared: usize,
}

/// Encodes a day, DECODES IT BACK and compares every event's v1 line and every config's day list with the originals, then compresses it
/// (the frame is decompressed and compared too). `corrupt` (tests only) flips one byte of the raw section before the decode-back.
///
/// # Errors
/// A shape the codec refuses, any difference after the decode-back (the writer stops; its store is never renamed), a zstd failure.
pub fn seal_day(d: DayIn, fl: Flags, ncfg: usize, corrupt: Option<usize>) -> Result<DaySec, String> {
    let mut raw = enc_section(&d, fl, ncfg)?;
    if let Some(k) = corrupt
        && let Some(b) = raw.get_mut(k)
    {
        *b ^= 0x5a;
    }
    let n = u32::try_from(d.evs.len()).map_err(|_| "events")?;
    let mut lines: Vec<u8> = Vec::with_capacity(d.evs.len() * 900);
    let mut ends = Vec::with_capacity(d.evs.len());
    for i in 0..d.evs.len() {
        lines.extend_from_slice(d.line(i).as_bytes());
        ends.push(lines.len());
    }
    // MUTANT m_e10_writer_no_decode_back: the section kept without the decode-back compare
    if !cfg!(feature = "m_e10_writer_no_decode_back") {
        let w = dec_section(raw.clone(), fl, 0).map_err(|e| format!("SHIP_W decode-back of day {}: {e}: refused", d.di))?;
        let mut a = 0_usize;
        for (i, &b) in ends.iter().enumerate() {
            if lines.get(a..b) != Some(w.line(i).as_bytes()) {
                return Err(format!("SHIP_W decode-back of day {}: event {i}'s record differs after the round trip: refused", d.di));
            }
            a = b;
        }
        let mut got = Vec::new();
        for (cfg, l) in d.lists.iter().enumerate() {
            w.cfg_day(u32::try_from(cfg).map_err(|_| "config")?, &mut got)?;
            if &got != l {
                return Err(format!("SHIP_W decode-back of day {}: config {cfg}'s day list differs after the round trip: refused", d.di));
            }
        }
        if w.cfgs().any(|c| d.lists.get(c as usize).is_none_or(Vec::is_empty)) {
            return Err(format!("SHIP_W decode-back of day {}: a config list appeared: refused", d.di));
        }
    }
    let frame = crate::slice::zstd_c(&raw)?;
    if crate::slice::zstd_d(&frame)? != raw {
        return Err(format!("SHIP_W: day {}'s zstd frame does not decompress to its section: refused", d.di));
    }
    let unready = [
        d.rd.iter().filter(|r| r.atr1.is_none()).count() as u64,
        d.rd.iter().filter(|r| r.atr5.is_none()).count() as u64,
        d.rd.iter().filter(|r| r.v300.is_none()).count() as u64,
        d.rd.iter().filter(|r| r.theta.is_err()).count() as u64,
    ];
    Ok(DaySec {
        di: d.di,
        n,
        raw_len: raw.len() as u64,
        e_d: sha256_bytes(&raw),
        l_d: sha256_bytes(&lines),
        frame,
        lines,
        ends,
        nent: d.lists.iter().map(|l| l.len() as u64).sum(),
        liq: d.liq,
        lists: d.lists,
        unready,
        prepared: d.prepared,
    })
}

// ------------------------------------------------------------------------------------------------ the per-config lists (writer, in memory)

/// A source of whole per-config lists (global event ids, list order): the in-memory compact store (writer) or plain vectors (tests).
pub trait LSrc: Sync {
    /// The number of configs.
    fn ncfg(&self) -> usize;
    /// Config `id`'s whole list into `out` (cleared first).
    fn list_into(&self, id: usize, out: &mut Vec<u32>);
}

/// Plain vectors (the in-memory set-up's `lists`).
pub struct VecSrc<'a>(pub &'a [Vec<u32>]);

impl LSrc for VecSrc<'_> {
    fn ncfg(&self) -> usize {
        self.0.len()
    }
    fn list_into(&self, id: usize, out: &mut Vec<u32>) {
        out.clear();
        if let Some(l) = self.0.get(id) {
            out.extend_from_slice(l);
        }
    }
}

/// The writer's per-config lists as zigzag-delta varints of the global ids (about 1.4 B an entry instead of 4).
#[derive(Clone, Debug, Default)]
pub struct Compact {
    pub b: Vec<Vec<u8>>,
    last: Vec<u32>,
    pub cnt: Vec<u64>,
}

impl Compact {
    /// Empty lists for `ncfg` configs.
    #[must_use]
    pub fn new(ncfg: usize) -> Self {
        Self { b: vec![Vec::new(); ncfg], last: vec![0; ncfg], cnt: vec![0; ncfg] }
    }

    /// Appends one day's lists (local ids) at the day's first global id `e0`.
    ///
    /// # Errors
    /// A global id beyond u32.
    pub fn push_day(&mut self, e0: u64, lists: &[Vec<u32>]) -> Result<(), String> {
        for (id, l) in lists.iter().enumerate() {
            let (Some(b), Some(last), Some(cnt)) = (self.b.get_mut(id), self.last.get_mut(id), self.cnt.get_mut(id)) else { return Err(format!("config {id} beyond the store")) };
            for &x in l {
                let g = u32::try_from(e0 + u64::from(x)).map_err(|_| "a global event id beyond u32")?;
                put_var(b, zz(i64::from(g) - i64::from(*last)));
                *last = g;
                *cnt += 1;
            }
        }
        Ok(())
    }

    /// The bytes held (MEASURED in RUN.log).
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.b.iter().map(Vec::len).sum()
    }
}

impl LSrc for Compact {
    fn ncfg(&self) -> usize {
        self.b.len()
    }
    fn list_into(&self, id: usize, out: &mut Vec<u32>) {
        out.clear();
        let (Some(b), Some(&n)) = (self.b.get(id), self.cnt.get(id)) else { return };
        let mut c = Cur { b: b.as_slice(), p: 0 };
        let mut prev = 0_i64;
        for _ in 0..n {
            let Ok(v) = c.var() else { return };
            prev += unzz(v);
            out.push(prev as u32);
        }
        // MUTANT m_e10_store_drops_last_entry: the stored form loses each list's last entry
        if cfg!(feature = "m_e10_store_drops_last_entry") {
            out.pop();
        }
    }
}

/// The unique lists of the set-up: (representative config, liquidity value, tf key), every (config, value) pair, the per-value removals and
/// each list's member count.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListStage {
    pub uls: Vec<(u32, u8, u8)>,
    pub ul_cfgs: Vec<Vec<(u32, u8)>>,
    pub removed: [(u64, u64, u64); crate::read::NLIQ],
    pub ul_n: Vec<u64>,
}

/// `report3::build_lists_derived`'s rule over any list source: the same FNV hash and length of each pair's filtered list, the same
/// candidates map, the same FULL compare against each candidate's derived sequence, the same first-seen numbering (tests: equal to it on
/// random set-ups; P-E1: the writer's plan = the in-memory plan; G7 at full scale before anything is shown).
#[must_use]
pub fn lists_stage(src: &dyn LSrc, liq: &[(u32, u32)], liqs: &[usize], tf_dep: bool) -> ListStage {
    let nl = crate::read::NLIQ;
    let mut seen: HashMap<(u64, usize, u8), Vec<u32>> = HashMap::new();
    let mut st = ListStage::default();
    let nev = liq.len();
    let passes: Vec<Vec<bool>> = (0..nl).map(|v| if liqs.contains(&v) { liq.iter().map(|&(c, g)| liq_pass(v, c, g)).collect() } else { Vec::new() }).collect();
    let mut buf: Vec<u32> = Vec::new();
    let mut rbuf: Vec<u32> = Vec::new();
    let used: Vec<bool> = {
        let mut u = vec![false; nev];
        for id in 0..src.ncfg() {
            src.list_into(id, &mut buf);
            for &e in &buf {
                if let Some(x) = u.get_mut(e as usize) {
                    *x = true;
                }
            }
        }
        u
    };
    let empty: Vec<bool> = Vec::new();
    for &v in liqs {
        let pass = passes.get(v).unwrap_or(&empty);
        if let Some(r) = st.removed.get_mut(v) {
            r.1 = used.iter().zip(pass).filter(|(u, p)| **u && !**p).count() as u64;
            r.2 = used.iter().zip(pass).filter(|(u, p)| **u && **p).count() as u64;
        }
        for id in 0..src.ncfg() {
            src.list_into(id, &mut buf);
            if buf.is_empty() {
                continue;
            }
            let mut h = 0xcbf2_9ce4_8422_2325_u64;
            let mut n = 0_usize;
            for &x in &buf {
                if pass.get(x as usize).copied().unwrap_or(false) {
                    h ^= u64::from(x);
                    h = h.wrapping_mul(0x0100_0000_01b3);
                    n += 1;
                }
            }
            if let Some(r) = st.removed.get_mut(v) {
                r.0 += (buf.len() - n) as u64;
            }
            if n == 0 {
                continue;
            }
            let tfk = if tf_dep { u8::try_from(crate::screen::cfg_parts(id).0).unwrap_or(0) } else { u8::MAX };
            let cands = seen.entry((h, n, tfk)).or_default();
            let mut found = None;
            for &u in cands.iter() {
                let Some(&(rid, rv, _)) = st.uls.get(u as usize) else { continue };
                let Some(rp) = passes.get(usize::from(rv)) else { continue };
                src.list_into(rid as usize, &mut rbuf);
                let mine = buf.iter().copied().filter(|&e| pass.get(e as usize).copied().unwrap_or(false));
                if rbuf.iter().copied().filter(|&e| rp.get(e as usize).copied().unwrap_or(false)).eq(mine) {
                    found = Some(u);
                    break;
                }
            }
            let u = if let Some(u) = found {
                u
            } else {
                let u = u32::try_from(st.uls.len()).unwrap_or(0);
                st.uls.push((u32::try_from(id).unwrap_or(0), u8::try_from(v).unwrap_or(0), tfk));
                st.ul_cfgs.push(Vec::new());
                st.ul_n.push(n as u64);
                cands.push(u);
                u
            };
            if let Some(c) = st.ul_cfgs.get_mut(u as usize) {
                c.push((u32::try_from(id).unwrap_or(0), u8::try_from(v).unwrap_or(0)));
            }
        }
    }
    st
}

/// The members of unique list `u` (global ids): its representative config's whole list filtered by its liquidity value (`Seg3::ul`).
pub fn members(src: &dyn LSrc, liq: &[(u32, u32)], ul: (u32, u8, u8), out: &mut Vec<u32>) {
    src.list_into(ul.0 as usize, out);
    let v = usize::from(ul.1);
    out.retain(|&e| liq.get(e as usize).is_some_and(|&(c, g)| liq_pass(v, c, g)));
}

// ------------------------------------------------------------------------------------------------ the v1 digest (F5), unchanged bytes

/// `kit::par_digest`'s composition, streamed: items in order, one sha256 per fixed chunk, then sha256(n u64 LE | chunk digests).
#[derive(Clone, Debug)]
pub struct ParStream {
    n: u64,
    chunk: usize,
    cur: Sha256,
    k: usize,
    digests: Vec<[u8; 32]>,
}

impl ParStream {
    /// A stream of chunk size `chunk`.
    #[must_use]
    pub fn new(chunk: usize) -> Self {
        Self { n: 0, chunk: chunk.max(1), cur: Sha256::new(), k: 0, digests: Vec::new() }
    }

    /// One item's bytes.
    pub fn push(&mut self, item: &[u8]) {
        self.cur.update(item);
        self.k += 1;
        self.n += 1;
        // MUTANT m_e10_digest_event_shift: the chunk cut one item late
        let at = if cfg!(feature = "m_e10_digest_event_shift") { self.chunk + 1 } else { self.chunk };
        if self.k == at {
            self.digests.push(std::mem::take(&mut self.cur).finish());
            self.k = 0;
        }
    }

    /// Items pushed so far.
    #[must_use]
    pub const fn count(&self) -> u64 {
        self.n
    }

    /// The digest.
    #[must_use]
    pub fn finish(mut self) -> [u8; 32] {
        if self.k > 0 {
            self.digests.push(std::mem::take(&mut self.cur).finish());
        }
        let mut h = Sha256::new();
        h.update(&self.n.to_le_bytes());
        for d in &self.digests {
            h.update(d);
        }
        h.finish()
    }
}

/// The v1 header: EXACTLY the bytes `kit::list_digest` hashes before its events (HDR.bin).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn v1_header(days: &[String], segdays: &[usize], pieces: &[u8], sets_sha: &str, menu_sha: &str, sel: &[usize], ties: &[u32], search: &[bool], csets: &[crate::kern::CSet], lv: &Levels, lvd: &[Levels]) -> Vec<u8> {
    let mut h = v1_prefix(days, segdays, pieces, sets_sha, menu_sha, sel, ties, search, csets);
    h.extend_from_slice(format!("levels {lv:?}\nlevels per day {lvd:?}\n").as_bytes());
    h
}

/// The header without its two levels lines (what a shard rebuilds from its OWN run: the header bound, test e10_header_bound).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn v1_prefix(days: &[String], segdays: &[usize], pieces: &[u8], sets_sha: &str, menu_sha: &str, sel: &[usize], ties: &[u32], search: &[bool], csets: &[crate::kern::CSet]) -> Vec<u8> {
    let mut h: Vec<u8> = Vec::new();
    h.extend_from_slice(b"SF-LIST-DIGEST v1\n");
    for d in days {
        h.extend_from_slice(d.as_bytes());
        h.extend_from_slice(b",");
    }
    h.extend_from_slice(format!("\nsegdays {segdays:?}\npieces {pieces:?}\n").as_bytes());
    h.extend_from_slice(format!("settings sets {sets_sha} menu {menu_sha} sel {sel:?}\nties {ties:?}\nsearch {search:?}\n").as_bytes());
    for cs in csets {
        h.extend_from_slice(format!("{cs:?}\n").as_bytes());
    }
    h
}

/// v1 from its parts (= `kit::list_digest` of the in-memory set-up).
#[must_use]
pub fn v1_assemble(hdr: &[u8], nev: u64, ev: &[u8; 32], ncfg: usize, cfg: &[u8; 32], nul: usize, ul: &[u8; 32]) -> String {
    let mut h = Sha256::new();
    h.update(hdr);
    h.update(format!("events {nev}\n").as_bytes());
    h.update(ev);
    h.update(format!("config lists {ncfg}\n").as_bytes());
    h.update(cfg);
    h.update(format!("unique lists {nul}\n").as_bytes());
    h.update(ul);
    h.hex()
}

/// The bytes v1 (and M_u) hash for one list: its length (u64 LE) then its ids (u32 LE).
#[must_use]
pub fn list_bytes(l: &[u32]) -> Vec<u8> {
    let mut b = Vec::with_capacity(8 + 4 * l.len());
    b.extend_from_slice(&(l.len() as u64).to_le_bytes());
    for e in l {
        b.extend_from_slice(&e.to_le_bytes());
    }
    b
}

/// v1's config-list part over a list source (`kit::par_digest` itself, chunks of 64).
#[must_use]
pub fn cfg_digest(src: &dyn LSrc, threads: usize) -> [u8; 32] {
    crate::kit::par_digest(src.ncfg(), 64, threads, &|i, hh| {
        let mut l = Vec::new();
        src.list_into(i, &mut l);
        hh.update(&list_bytes(&l));
    })
}

/// v1's unique-list part (chunks of 16) and every list's member digest M_u = sha256(len u64 LE | members u32 LE).
#[must_use]
pub fn ul_digest(src: &dyn LSrc, liq: &[(u32, u32)], st: &ListStage, threads: usize) -> ([u8; 32], Vec<String>, Vec<u64>) {
    let nul = st.uls.len();
    let mu: Vec<OnceLock<(String, u64)>> = (0..nul).map(|_| OnceLock::new()).collect();
    let d = crate::kit::par_digest(nul, 16, threads, &|u, hh| {
        hh.update(format!("{:?}|{:?}\n", st.uls.get(u), st.ul_cfgs.get(u)).as_bytes());
        let Some(&ul) = st.uls.get(u) else { return };
        let mut m = Vec::new();
        members(src, liq, ul, &mut m);
        let b = list_bytes(&m);
        hh.update(&b);
        if let Some(x) = mu.get(u) {
            let _ = x.set((sha256_bytes(&b), m.len() as u64));
        }
    });
    let (mut out, mut ns) = (Vec::with_capacity(nul), Vec::with_capacity(nul));
    for x in mu {
        let (a, b) = x.into_inner().unwrap_or_default();
        out.push(a);
        ns.push(b);
    }
    (d, out, ns)
}

// ------------------------------------------------------------------------------------------------ the store: SETUP.tsv, v2

/// One segment day's line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DLine {
    pub di: usize,
    pub date: String,
    pub e0: u64,
    pub n: u32,
    pub raw: u64,
    pub file: String,
    pub off: u64,
    pub len: u64,
    pub nent: u64,
    pub l_d: String,
    pub e_d: String,
}

impl DLine {
    fn text_wo(&self) -> String {
        format!("D\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", self.di, self.date, self.e0, self.n, self.raw, self.file, self.off, self.len, self.nent, self.l_d)
    }
    fn text(&self) -> String {
        format!("{}\t{}", self.text_wo(), self.e_d)
    }
}

/// One unique list's line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ULine {
    pub u: usize,
    pub rep: u32,
    pub v: u8,
    pub tfk: u8,
    pub n: u64,
    pub cfgs: Vec<(u32, u8)>,
    pub m_u: String,
}

impl ULine {
    fn text_wo(&self) -> String {
        format!("U\t{}\t{}\t{}\t{}\t{}\t{}", self.u, self.rep, self.v, self.tfk, self.n, self.cfgs.iter().map(|(c, v)| format!("{c}:{v}")).collect::<Vec<_>>().join(","))
    }
    fn text(&self) -> String {
        format!("{}\t{}", self.text_wo(), self.m_u)
    }
}

/// The store's facts besides its D and U lines.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Meta {
    pub seg: u8,
    pub days: Vec<String>,
    pub segdays: Vec<usize>,
    pub nev: u64,
    pub ncfg: usize,
    pub liqs: Vec<usize>,
    pub tfdep: bool,
    pub flags: Flags,
    pub args: String,
    pub inputs: String,
    pub v1: String,
    pub hdr_len: u64,
    pub hdr_sha: String,
}

/// The core text of SETUP.tsv: every line but V2, OBJ, PROV and END (M1: all of it is bound into v2).
#[must_use]
pub fn core_text(m: &Meta, d: &[DLine], u: &[ULine]) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "SPIKEFADE-SETUP\t2");
    let _ = writeln!(s, "SEG\t{}", m.seg);
    let _ = writeln!(s, "DAYS\t{}\t{}", m.days.len(), m.days.join(","));
    let _ = writeln!(s, "SEGDAYS\t{}\t{}", m.segdays.len(), m.segdays.iter().map(usize::to_string).collect::<Vec<_>>().join(","));
    let _ = writeln!(s, "NEV\t{}", m.nev);
    let _ = writeln!(s, "NCFG\t{}", m.ncfg);
    let _ = writeln!(s, "LIQS\t{}", m.liqs.iter().map(usize::to_string).collect::<Vec<_>>().join(","));
    let _ = writeln!(s, "TFDEP\t{}", u8::from(m.tfdep));
    let _ = writeln!(s, "{}", m.flags.line());
    let _ = writeln!(s, "ARGS\t{}", m.args);
    let _ = writeln!(s, "INPUTS\t{}", m.inputs);
    let _ = writeln!(s, "V1\t{}", m.v1);
    let _ = writeln!(s, "HDR\tHDR.bin\t{}\t{}", m.hdr_len, m.hdr_sha);
    for x in d {
        let _ = writeln!(s, "{}", x.text());
    }
    for x in u {
        let _ = writeln!(s, "{}", x.text());
    }
    s
}

fn ev_root(d: &[DLine]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(&(d.len() as u32).to_le_bytes());
    for x in d {
        h.update(x.text_wo().as_bytes());
        h.update(b"\n");
        h.update(x.e_d.as_bytes());
    }
    h.finish()
}

fn ul_root(u: &[ULine]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(&(u.len() as u32).to_le_bytes());
    for ch in u.chunks(16) {
        let mut k = Sha256::new();
        for x in ch {
            k.update(x.text_wo().as_bytes());
            k.update(b"\n");
            k.update(x.m_u.as_bytes());
        }
        h.update(&k.finish());
    }
    h.finish()
}

/// M1: v2 = sha256("SF-SETUP-DIGEST v2\n" | sha256(the core text) | ev_root | ul_root). The core text holds FLAGS, SEG, NEV, NCFG, LIQS,
/// TFDEP, DAYS, SEGDAYS, ARGS, INPUTS, V1, the HDR line, every D line (E_d, L_d, n, e0) and every U line (M_u). The per-config day lists live
/// inside each day's section, so E_d (in ev_root) binds them: there is no config-major chunk store and no cfg_root.
#[must_use]
pub fn v2_of(core: &str, d: &[DLine], u: &[ULine]) -> String {
    // MUTANT m_e10_flags_unbound: the FLAGS line (it decides how every record is decoded) left out of v2
    let core: String = if cfg!(feature = "m_e10_flags_unbound") { core.lines().filter(|l| !l.starts_with("FLAGS\t")).map(|l| format!("{l}\n")).collect() } else { core.to_owned() };
    let mut c = Sha256::new();
    c.update(core.as_bytes());
    let mut h = Sha256::new();
    h.update(b"SF-SETUP-DIGEST v2\n");
    h.update(&c.finish());
    h.update(&ev_root(d));
    h.update(&ul_root(u));
    h.hex()
}

// ------------------------------------------------------------------------------------------------ the writer's sequential part

/// The store being written (`DIR.part`): the event files, the D lines, the liquidity readings, the compact per-config lists, the streamed
/// v1 event digest and the counts the in-memory run prints.
pub struct Builder {
    part: PathBuf,
    pub fl: Flags,
    pub ncfg: usize,
    pub d: Vec<DLine>,
    pub e0: u64,
    file: Option<(String, std::fs::File, u64)>,
    pub liq: Vec<(u32, u32)>,
    pub compact: Compact,
    pub ev: ParStream,
    pub nent: u128,
    pub unready: [u64; 4],
    pub prepared: usize,
    pub raw_bytes: u64,
    pub frame_bytes: u64,
}

impl Builder {
    /// A new store in `part` (created; refused when it exists).
    ///
    /// # Errors
    /// An existing or uncreatable folder.
    pub fn new(part: &Path, fl: Flags, ncfg: usize) -> Result<Self, String> {
        if part.exists() {
            return Err(format!("{}: exists (never overwritten): refused", part.display()));
        }
        std::fs::create_dir_all(part.join("ev")).map_err(|e| format!("{}: {e}", part.display()))?;
        Ok(Self { part: part.to_path_buf(), fl, ncfg, d: Vec::new(), e0: 0, file: None, liq: Vec::new(), compact: Compact::new(ncfg), ev: ParStream::new(4_096), nent: 0, unready: [0; 4], prepared: 0, raw_bytes: 0, frame_bytes: 0 })
    }

    /// Appends one sealed day (in day order) under its date.
    ///
    /// # Errors
    /// An unwritable file.
    pub fn push(&mut self, s: DaySec, date: &str) -> Result<(), String> {
        let month = date.get(0..7).ok_or("date")?.to_owned();
        let name = format!("ev/{month}.sfe.zst");
        if self.file.as_ref().is_none_or(|f| f.0 != name) {
            if let Some((_, f, _)) = self.file.take() {
                f.sync_all().map_err(|e| e.to_string())?;
            }
            let p = self.part.join(&name);
            if p.exists() {
                return Err(format!("{}: a month file reopened (days out of order): refused", p.display()));
            }
            let f = std::fs::File::create(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            self.file = Some((name.clone(), f, 0));
        }
        let (_, f, off) = self.file.as_mut().ok_or("file")?;
        f.write_all(&s.frame).map_err(|e| e.to_string())?;
        let at = *off;
        *off += s.frame.len() as u64;
        self.d.push(DLine { di: s.di, date: date.to_owned(), e0: self.e0, n: s.n, raw: s.raw_len, file: name, off: at, len: s.frame.len() as u64, nent: s.nent, l_d: s.l_d, e_d: s.e_d });
        self.compact.push_day(self.e0, &s.lists)?;
        self.liq.extend_from_slice(&s.liq);
        let mut a = 0;
        for &b in &s.ends {
            self.ev.push(s.lines.get(a..b).unwrap_or(&[]));
            a = b;
        }
        self.e0 += u64::from(s.n);
        self.nent += u128::from(s.nent);
        for (x, y) in self.unready.iter_mut().zip(s.unready) {
            *x += y;
        }
        self.prepared += s.prepared;
        self.raw_bytes += s.raw_len;
        self.frame_bytes += s.frame.len() as u64;
        Ok(())
    }

    /// Closes the event files.
    ///
    /// # Errors
    /// A failing fsync.
    pub fn close(&mut self) -> Result<(), String> {
        if let Some((_, f, _)) = self.file.take() {
            f.sync_all().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

/// Reads one stored frame and returns its raw section after checking E_d (`check` false only under a test of the check itself).
///
/// # Errors
/// An unreadable file, a zstd failure, a sha256 that differs.
pub fn read_raw(dir: &Path, d: &DLine, check: bool) -> Result<Vec<u8>, String> {
    let p = dir.join(&d.file);
    let mut f = std::fs::File::open(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    f.seek(std::io::SeekFrom::Start(d.off)).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut z = vec![0_u8; usize::try_from(d.len).map_err(|_| "frame")?];
    f.read_exact(&mut z).map_err(|e| format!("{}: day {} frame: {e}", p.display(), d.date))?;
    let raw = crate::slice::zstd_d(&z)?;
    // MUTANT m_e10_trusts_vectors: the section used without its sha256 check
    if check && !cfg!(feature = "m_e10_trusts_vectors") && (raw.len() as u64 != d.raw || sha256_bytes(&raw) != d.e_d) {
        return Err(format!("store: day {} ({}): the section's sha256 is not its D line's E_d: refused", d.di, d.date));
    }
    Ok(raw)
}

/// Writes SETUP.tsv: the core, the OBJ lines (sha256 of every object as stored), V2, PROV, END.
///
/// # Errors
/// An unwritable or unreadable file.
pub fn write_setup_tsv(part: &Path, core: &str, v2: &str, prov: &[(String, String)]) -> Result<(), String> {
    let mut s = core.to_owned();
    let mut objs: Vec<(String, PathBuf)> = vec![("HDR.bin".to_owned(), part.join("HDR.bin"))];
    for (name, p) in crate::kit::sorted_dir(&part.join("ev"))? {
        objs.push((format!("ev/{name}"), p));
    }
    for (name, p) in objs {
        let len = std::fs::metadata(&p).map_err(|e| format!("{}: {e}", p.display()))?.len();
        let sha = crate::kit::sha256_file(&p).ok_or_else(|| format!("{}: unreadable", p.display()))?;
        let _ = writeln!(s, "OBJ\t{name}\t{len}\t{sha}");
    }
    let _ = writeln!(s, "V2\t{v2}");
    let _ = writeln!(s, "PROV\t{}", prov.iter().map(|(k, v)| format!("{k}={}", v.replace(['\t', '\n'], " "))).collect::<Vec<_>>().join("\t"));
    let _ = writeln!(s, "END");
    let p = part.join("SETUP.tsv");
    std::fs::write(&p, s).map_err(|e| format!("{}: {e}", p.display()))
}

// ------------------------------------------------------------------------------------------------ the reader (SHIP_R)

/// The facts a shard derives from its OWN run (M1: never taken from the store).
#[derive(Clone, Debug, Default)]
pub struct Own {
    pub seg: u8,
    pub days: Vec<String>,
    pub segdays: Vec<usize>,
    pub liqs: Vec<usize>,
    pub tfdep: bool,
    pub flags: Flags,
    pub args: String,
    pub inputs: String,
    pub ncfg: usize,
}

/// G2: the in-memory set-up a SHIP_AUDIT run compares every window with.
#[derive(Default)]
pub struct Audit {
    pub evs: Vec<SEv>,
    pub liq: Vec<(u32, u32)>,
    pub tfmask: Vec<u16>,
    pub rd: Vec<Rd>,
    pub tfw: Vec<Vec<(u8, i64, i64)>>,
    pub vol60: Vec<u64>,
    pub mx: Vec<MarginAt>,
    pub lists: Vec<Vec<u32>>,
}

/// A verified store (SHIP_R).
pub struct Ship {
    pub dir: PathBuf,
    pub meta: Meta,
    pub v2: String,
    pub d: Vec<DLine>,
    d_of: HashMap<usize, usize>,
    pub u: Vec<ULine>,
    pub hdr: Vec<u8>,
    pub audit: Mutex<Option<Audit>>,
}

fn parse_hex64(s: &str, what: &str) -> Result<String, String> {
    if s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) { Ok(s.to_owned()) } else { Err(format!("SETUP.tsv: {what} is not a sha256: {s:?}")) }
}

fn num<T: std::str::FromStr>(x: Option<&str>, what: &str) -> Result<T, String> {
    x.ok_or_else(|| format!("SETUP.tsv: short {what}"))?.parse::<T>().map_err(|_| format!("SETUP.tsv: bad {what}"))
}

impl Ship {
    /// Opens and VERIFIES a store against the run's plan and its own facts (refuses by name before the first batch): the exhaustive parse,
    /// v2 = the plan's SETUP line (M1: every decode-controlling line bound), V1 = the plan's DIGEST, FLAGS / SEG / DAYS / SEGDAYS / NCFG / NEV /
    /// LIQS / TFDEP / ARGS / INPUTS = this run's own, the D lines' e0 chain, HDR.bin's sha256.
    ///
    /// # Errors
    /// Any of the above.
    pub fn open(dir: &Path, plan_setup: &str, plan_digest: &str, own: &Own) -> Result<Self, String> {
        let text = std::fs::read_to_string(dir.join("SETUP.tsv")).map_err(|e| format!("--setup-from {}: SETUP.tsv: {e}", dir.display()))?;
        if !text.ends_with("END\n") {
            return Err("SETUP.tsv: truncated (no END line): refused".to_owned());
        }
        let mut core = String::new();
        let mut m = Meta::default();
        let (mut d, mut u) = (Vec::new(), Vec::new());
        let mut v2 = String::new();
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let (mut ndays, mut nseg) = (0_usize, 0_usize);
        for (i, l) in text.lines().enumerate() {
            let mut f = l.split('\t');
            let tag = f.next().unwrap_or("");
            if !matches!(tag, "V2" | "OBJ" | "PROV" | "END") {
                core.push_str(l);
                core.push('\n');
            }
            if matches!(tag, "SEG" | "DAYS" | "SEGDAYS" | "NEV" | "NCFG" | "LIQS" | "TFDEP" | "FLAGS" | "ARGS" | "INPUTS" | "V1" | "HDR" | "V2") && !seen.insert(tag) {
                return Err(format!("SETUP.tsv: {tag} repeated (line {})", i + 1));
            }
            match tag {
                "SPIKEFADE-SETUP" if i == 0 => {
                    if f.next() != Some("2") {
                        return Err("SETUP.tsv: version is not 2".to_owned());
                    }
                }
                "SEG" => m.seg = num(f.next(), "SEG")?,
                "DAYS" => {
                    ndays = num(f.next(), "DAYS")?;
                    m.days = f.next().unwrap_or("").split(',').filter(|x| !x.is_empty()).map(str::to_owned).collect();
                }
                "SEGDAYS" => {
                    nseg = num(f.next(), "SEGDAYS")?;
                    m.segdays = f.next().unwrap_or("").split(',').filter(|x| !x.is_empty()).map(|x| x.parse::<usize>().map_err(|_| "SETUP.tsv: bad SEGDAYS".to_owned())).collect::<Result<_, _>>()?;
                }
                "NEV" => m.nev = num(f.next(), "NEV")?,
                "NCFG" => m.ncfg = num(f.next(), "NCFG")?,
                "LIQS" => m.liqs = f.next().unwrap_or("").split(',').filter(|x| !x.is_empty()).map(|x| x.parse::<usize>().map_err(|_| "SETUP.tsv: bad LIQS".to_owned())).collect::<Result<_, _>>()?,
                "TFDEP" => {
                    m.tfdep = match f.next() {
                        Some("1") => true,
                        Some("0") => false,
                        o => return Err(format!("SETUP.tsv: TFDEP {o:?}")),
                    }
                }
                "FLAGS" => {
                    let (a, b) = (f.next(), f.next());
                    m.flags = match (a, b) {
                        (Some(x), Some(y)) => Flags {
                            vol60: match x {
                                "vol60=1" => true,
                                "vol60=0" => false,
                                o => return Err(format!("SETUP.tsv: FLAGS {o:?}")),
                            },
                            mx: match y {
                                "mx=1" => true,
                                "mx=0" => false,
                                o => return Err(format!("SETUP.tsv: FLAGS {o:?}")),
                            },
                        },
                        _ => return Err("SETUP.tsv: short FLAGS".to_owned()),
                    };
                    if f.next().is_some() {
                        return Err("SETUP.tsv: FLAGS has an unknown field".to_owned());
                    }
                }
                "ARGS" => m.args = parse_hex64(f.next().unwrap_or(""), "ARGS")?,
                "INPUTS" => m.inputs = parse_hex64(f.next().unwrap_or(""), "INPUTS")?,
                "V1" => m.v1 = parse_hex64(f.next().unwrap_or(""), "V1")?,
                "HDR" => {
                    if f.next() != Some("HDR.bin") {
                        return Err("SETUP.tsv: HDR names another object".to_owned());
                    }
                    m.hdr_len = num(f.next(), "HDR bytes")?;
                    m.hdr_sha = parse_hex64(f.next().unwrap_or(""), "HDR")?;
                }
                "D" => {
                    let x = DLine {
                        di: num(f.next(), "D di")?,
                        date: f.next().ok_or("SETUP.tsv: short D")?.to_owned(),
                        e0: num(f.next(), "D e0")?,
                        n: num(f.next(), "D n")?,
                        raw: num(f.next(), "D raw")?,
                        file: f.next().ok_or("SETUP.tsv: short D")?.to_owned(),
                        off: num(f.next(), "D offset")?,
                        len: num(f.next(), "D frame")?,
                        nent: num(f.next(), "D entries")?,
                        l_d: parse_hex64(f.next().unwrap_or(""), "D L_d")?,
                        e_d: parse_hex64(f.next().unwrap_or(""), "D E_d")?,
                    };
                    if f.next().is_some() || !x.file.starts_with("ev/") || x.file.contains("..") {
                        return Err(format!("SETUP.tsv: D line {} malformed", i + 1));
                    }
                    d.push(x);
                }
                "U" => {
                    let uu: usize = num(f.next(), "U u")?;
                    if uu != u.len() {
                        return Err(format!("SETUP.tsv: U {uu} out of order (expected {})", u.len()));
                    }
                    let x = ULine {
                        u: uu,
                        rep: num(f.next(), "U rep")?,
                        v: num(f.next(), "U v")?,
                        tfk: num(f.next(), "U tfk")?,
                        n: num(f.next(), "U n")?,
                        cfgs: f.next().unwrap_or("").split(',').filter(|x| !x.is_empty()).map(|p| p.split_once(':').and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u8>().ok()?))).ok_or_else(|| "SETUP.tsv: bad U pair".to_owned())).collect::<Result<_, _>>()?,
                        m_u: parse_hex64(f.next().unwrap_or(""), "U M_u")?,
                    };
                    if f.next().is_some() {
                        return Err(format!("SETUP.tsv: U line {} malformed", i + 1));
                    }
                    u.push(x);
                }
                "OBJ" => {}
                "V2" => v2 = parse_hex64(f.next().unwrap_or(""), "V2")?,
                "PROV" => {}
                "END" => {}
                t => return Err(format!("SETUP.tsv: unknown tag {t:?} at line {} (refused, never skipped)", i + 1)),
            }
        }
        if m.days.len() != ndays || m.segdays.len() != nseg || v2.is_empty() || m.v1.is_empty() || m.hdr_sha.is_empty() {
            return Err("SETUP.tsv: a count that does not match its list, or no V1 / V2 / HDR line: refused".to_owned());
        }
        // M1: v2 recomputed from every decode-controlling line
        let got = v2_of(&core, &d, &u);
        if got != v2 {
            return Err(format!("store {}: v2 recomputed {got} but SETUP.tsv says {v2}: an edited or damaged SETUP.tsv: refused", dir.display()));
        }
        if plan_setup != v2 {
            return Err(format!("store {}: v2 {v2} but the plan's SETUP line is {plan_setup}: another store: refused", dir.display()));
        }
        // MUTANT m_e10_plan_v1_unchecked: the store's V1 not compared with the plan's list digest
        if m.v1 != plan_digest && !cfg!(feature = "m_e10_plan_v1_unchecked") {
            return Err(format!("store {}: V1 {} is not the plan's DIGEST {plan_digest}: refused", dir.display(), m.v1));
        }
        // M1: the decode-controlling facts derived from THIS run's own switches and inputs
        let nev: u64 = d.iter().map(|x| u64::from(x.n)).sum();
        let mut why = Vec::new();
        if m.flags != own.flags {
            why.push(format!("FLAGS {:?} vs this run's switches {:?}", m.flags, own.flags));
        }
        if m.seg != own.seg {
            why.push(format!("SEG {} vs {}", m.seg, own.seg));
        }
        if m.days != own.days {
            why.push("DAYS".to_owned());
        }
        if m.segdays != own.segdays {
            why.push(format!("SEGDAYS ({} vs this run's {} segment days)", m.segdays.len(), own.segdays.len()));
        }
        if m.ncfg != own.ncfg {
            why.push(format!("NCFG {} vs {}", m.ncfg, own.ncfg));
        }
        if m.nev != nev {
            why.push(format!("NEV {} vs the D lines' sum {nev}", m.nev));
        }
        if m.liqs != own.liqs {
            why.push(format!("LIQS {:?} vs {:?}", m.liqs, own.liqs));
        }
        if m.tfdep != own.tfdep {
            why.push("TFDEP".to_owned());
        }
        if m.args != own.args {
            why.push(format!("ARGS {} vs this run's {}", m.args, own.args));
        }
        if m.inputs != own.inputs {
            why.push(format!("INPUTS {} vs this run's inputs manifest {}", m.inputs, own.inputs));
        }
        let mut e0 = 0_u64;
        for (k, x) in d.iter().enumerate() {
            if Some(&x.di) != m.segdays.get(k) || x.e0 != e0 || m.days.get(x.di) != Some(&x.date) {
                why.push(format!("D line {k} (day {}): not the k-th segment day, or e0 {} vs the chain's {e0}", x.di, x.e0));
                break;
            }
            e0 += u64::from(x.n);
        }
        if d.len() != m.segdays.len() {
            why.push(format!("{} D lines for {} segment days", d.len(), m.segdays.len()));
        }
        if !why.is_empty() {
            return Err(format!("store {} vs this run (M1): {}: refused before the first batch", dir.display(), why.join("; ")));
        }
        let hdr = std::fs::read(dir.join("HDR.bin")).map_err(|e| format!("store HDR.bin: {e}"))?;
        if hdr.len() as u64 != m.hdr_len || sha256_bytes(&hdr) != m.hdr_sha {
            return Err("store HDR.bin: its sha256 or size is not the HDR line's: refused".to_owned());
        }
        let d_of = d.iter().enumerate().map(|(k, x)| (x.di, k)).collect();
        Ok(Self { dir: dir.to_path_buf(), meta: m, v2, d, d_of, u, hdr, audit: Mutex::new(None) })
    }

    /// The D line of run day `di`.
    #[must_use]
    pub fn dline(&self, di: usize) -> Option<&DLine> {
        self.d_of.get(&di).and_then(|&k| self.d.get(k))
    }

    /// Header bound (DECISION test 16): the v1 header prefix THIS run rebuilds (days, segment days, pieces, settings, ties, search, every
    /// compiled setting) must equal HDR.bin up to its two levels lines.
    ///
    /// # Errors
    /// A different header.
    pub fn header_check(&self, own_prefix: &[u8]) -> Result<(), String> {
        // MUTANT m_e10_hdr_unchecked: the store's header trusted
        if cfg!(feature = "m_e10_hdr_unchecked") {
            return Ok(());
        }
        let ok = self.hdr.get(..own_prefix.len()) == Some(own_prefix) && self.hdr.get(own_prefix.len()..).is_some_and(|r| r.starts_with(b"levels "));
        if !ok {
            return Err("store HDR.bin: the v1 header this run rebuilds (days, segment days, pieces, settings, ties, search, compiled settings) differs from the store's: refused".to_owned());
        }
        Ok(())
    }

    /// The window of run day `di` (E_d checked; M2's identity is checked by [`guard`]).
    ///
    /// # Errors
    /// No D line, an unreadable or damaged section.
    pub fn window(&self, di: usize) -> Result<Window, String> {
        let t0 = std::time::Instant::now();
        let mut k = *self.d_of.get(&di).ok_or_else(|| format!("store: no D line for day {di}"))?;
        // MUTANT m_e10_window_prev_day: the previous segment day's window handed out for this day
        if cfg!(feature = "m_e10_window_prev_day") && k > 0 {
            k -= 1;
        }
        let d = self.d.get(k).ok_or("D")?;
        let mut w = dec_section(read_raw(&self.dir, d, true)?, self.meta.flags, d.e0)?;
        // MUTANT m_e10_window_short_mx: the reader drops the exact margins (stage A would keep the PROXY margin silently)
        if cfg!(feature = "m_e10_window_short_mx") {
            w.mx.clear();
        }
        DECODES.fetch_add(1, Ordering::Relaxed);
        DECODE_NS.fetch_add(u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX), Ordering::Relaxed);
        Ok(w)
    }

    /// The start check (S1 + M3 + M5): ONE streamed pass over every segment day (nothing kept): each section's E_d, its decoded records'
    /// v1-line digest L_d (decode equality at full scale on THIS platform), and for every list the shard walks, its members derived by
    /// [`day_sub`] (the same function the walk uses) day by day into a running sha256 that must equal M_u. Days are decoded in parallel
    /// chunks; the hashers are fed in day order. Returns its words (MEASURED wall).
    ///
    /// # Errors
    /// The first difference, by name.
    pub fn start_check(&self, lists: &[usize], threads: usize) -> Result<String, String> {
        let t0 = std::time::Instant::now();
        let mut hs: Vec<Sha256> = lists.iter().map(|&u| self.m3_start(u)).collect::<Result<_, _>>()?;
        let mut nraw = 0_u64;
        for ch in self.d.chunks(threads.clamp(1, 16)) {
            type Out = (usize, Vec<Vec<u32>>, u64);
            let res: Vec<Result<Out, String>> = std::thread::scope(|s| {
                let hh: Vec<_> = ch
                    .iter()
                    .map(|d| {
                        s.spawn(move || -> Result<Out, String> {
                            let raw = read_raw(&self.dir, d, true)?;
                            let nr = raw.len() as u64;
                            let w = dec_section(raw, self.meta.flags, d.e0)?;
                            if w.di != d.di || w.n != d.n {
                                return Err(format!("store: day {} section is day {} with {} events (D line: {}): refused", d.di, w.di, w.n, d.n));
                            }
                            let mut lh = Sha256::new();
                            for i in 0..w.n as usize {
                                lh.update(w.line(i).as_bytes());
                            }
                            if lh.hex() != d.l_d {
                                return Err(format!("store: day {} ({}): the decoded records' digest is not L_d (S1: decode differs on this platform): refused", d.di, d.date));
                            }
                            let mut subs = Vec::with_capacity(lists.len());
                            for &u in lists {
                                let ul = self.u.get(u).ok_or("U")?;
                                let mut s1 = Vec::new();
                                day_sub(&w, ul.rep, ul.v, &mut s1)?;
                                subs.push(s1);
                            }
                            Ok((d.di, subs, nr))
                        })
                    })
                    .collect();
                hh.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("start check thread".to_owned()))).collect()
            });
            for (r, d) in res.into_iter().zip(ch) {
                let (_, subs, nr) = r?;
                nraw += nr;
                for (h, s1) in hs.iter_mut().zip(&subs) {
                    for &l in s1 {
                        h.update(&u32::try_from(d.e0 + u64::from(l)).map_err(|_| "id")?.to_le_bytes());
                    }
                }
            }
        }
        for (h, &u) in hs.into_iter().zip(lists) {
            let ul = self.u.get(u).ok_or("U")?;
            // MUTANT m_e10_skips_members: the members never compared with M_u at start
            if h.hex() != ul.m_u && !cfg!(feature = "m_e10_skips_members") {
                return Err(format!("store: list {u}'s members derived from the day sections are not its M_u: refused before the first batch"));
            }
        }
        Ok(format!("E10 start check: {} segment day(s), {nraw} raw section bytes, every E_d and L_d verified; {} list(s) of this shard: every M_u verified; {:.1} s (MEASURED)", self.d.len(), lists.len(), t0.elapsed().as_secs_f64()))
    }

    /// M3: a list's running hasher at its batch start (fed n(u) first, as M_u is).
    ///
    /// # Errors
    /// An unknown list.
    pub fn m3_start(&self, u: usize) -> Result<Sha256, String> {
        let ul = self.u.get(u).ok_or_else(|| format!("store: no U line for list {u}"))?;
        let mut h = Sha256::new();
        h.update(&ul.n.to_le_bytes());
        Ok(h)
    }

    /// G2: compares a decoded window with the in-memory set-up (its records' v1 lines and every config's day list); returns the
    /// differences counted.
    pub fn audit_window(&self, w: &Window) -> u64 {
        let Ok(g) = self.audit.lock() else { return 1 };
        let Some(a) = g.as_ref() else { return 0 };
        AUDITED.fetch_add(1, Ordering::Relaxed);
        let mut bad = 0_u64;
        let e0 = w.e0 as usize;
        for i in 0..w.n as usize {
            let j = e0 + i;
            let want = v1_line(a.evs.get(j), a.liq.get(j), a.tfmask.get(j), a.rd.get(j), a.tfw.get(j), a.vol60.get(j), a.mx.get(j));
            if w.line(i) != want {
                bad += 1;
            }
        }
        let lo = e0 as u32;
        let hi = lo + w.n;
        let mut got = Vec::new();
        for (cfg, l) in a.lists.iter().enumerate() {
            let a0 = l.partition_point(|&e| e < lo);
            let a1 = l.partition_point(|&e| e < hi);
            if w.cfg_day(cfg as u32, &mut got).is_err() {
                bad += 1;
                continue;
            }
            let want: Vec<u32> = l.get(a0..a1).unwrap_or(&[]).iter().map(|&e| e - lo).collect();
            // MUTANT m_e10_audit_blind: the config lists never compared
            if got != want && !cfg!(feature = "m_e10_audit_blind") {
                bad += 1;
            }
        }
        bad
    }
}

/// THE derivation of a unique list's day sub-list (M3: the start check and the walk both call it): the representative config's day list
/// filtered by the list's liquidity value (`Seg3::ul_day` under C3, on the day's window).
///
/// # Errors
/// A malformed day list.
pub fn day_sub(w: &Window, rep: u32, v: u8, out: &mut Vec<u32>) -> Result<(), String> {
    w.cfg_day(rep, out)?;
    let v = usize::from(v);
    out.retain(|&l| w.liq.get(l as usize).is_some_and(|&(c, g)| liq_pass(v, c, g)));
    Ok(())
}

/// The walk's parts of one batch-day under SHIP_R: per list of the batch (in batch order) its day sub-list (local ids) and keys, and the
/// sorted union `want`; every sub-list the walk receives is fed (as GLOBAL ids) into the list's running M3 hasher.
///
/// # Errors
/// A malformed day list or an unknown list.
pub fn ship_parts(ship: &Ship, w: &Window, blist: &[usize], m3: &mut [Sha256]) -> Result<(Vec<usize>, Vec<(Vec<u32>, Vec<u32>)>), String> {
    let mut want: Vec<usize> = Vec::new();
    let mut subs: Vec<(Vec<u32>, Vec<u32>)> = Vec::with_capacity(blist.len());
    for (k, &u) in blist.iter().enumerate() {
        let ul = ship.u.get(u).ok_or_else(|| format!("store: no U line for list {u}"))?;
        // MUTANT m_e10_parts_wrong_liq: the walk filters with the pair's value + 1 (the start check, which calls day_sub directly, passes)
        let v = if cfg!(feature = "m_e10_parts_wrong_liq") { ul.v.saturating_add(1) } else { ul.v };
        let mut s = Vec::new();
        day_sub(w, ul.rep, v, &mut s)?;
        let kk: Vec<u32> = s.iter().map(|&l| w.evs.get(l as usize).map_or(0, |e| u32::from(e.di) * 86_400 + e.t)).collect();
        if let Some(h) = m3.get_mut(k) {
            let mut b = Vec::with_capacity(4 * s.len());
            for &l in &s {
                b.extend_from_slice(&u32::try_from(w.e0 + u64::from(l)).map_err(|_| "id")?.to_le_bytes());
            }
            h.update(&b);
        }
        want.extend(s.iter().map(|&e| e as usize));
        subs.push((s, kk));
    }
    want.sort_unstable();
    want.dedup();
    // CHECK F-1 proof: the batch's largest batch-day union (= setup-stats' U_k*), first day on ties; printed by `mem_line` (RUN.log only)
    if let Ok(mut g) = BATCH_UMAX.lock()
        && want.len() > g.0
    {
        *g = (want.len(), w.di);
    }
    Ok((want, subs))
}

/// The identity of the window whose arrays are swapped into the segment state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WinId {
    pub di: usize,
    pub e0: u64,
    pub n: u32,
}

/// M2 + G1: the window in the segment state against the day's D line, the run's FLAGS, the loaded cube and the batch-day's parts. Called
/// before stage A and again before the walk; every miss is a counted violation (it voids the batch file). Returns the misses.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn guard(sr: &Seg3, id: WinId, d: &DLine, fl: Flags, di: usize, loaded: bool, want: &[usize], subs: &[(Vec<u32>, Vec<u32>)]) -> Vec<String> {
    let mut why = Vec::new();
    let n = id.n as usize;
    if id.di != di || d.di != di || id.e0 != d.e0 || id.n != d.n {
        why.push(format!("window of day {} (e0 {}, n {}) used for day {di} (D line: day {}, e0 {}, n {})", id.di, id.e0, id.n, d.di, d.e0, d.n));
    }
    if !loaded {
        why.push(format!("day {di}'s cube is not loaded (an empty default cube would make every event BadTape)"));
    }
    let lens = [("evs", sr.evs.len()), ("ex", sr.ex.len()), ("rd", sr.rd.len()), ("tfw", sr.tfw.len()), ("liq", sr.liq.len()), ("tfmask", sr.tfmask.len()), ("counted", sr.counted.len())];
    for (name, l) in lens {
        if l != n {
            why.push(format!("day {di}: {name} holds {l} entries for {n} events (stage A would fill a default)"));
        }
    }
    for (name, l, on) in [("mx", sr.mx.len(), fl.mx), ("vol60", sr.vol60.len(), fl.vol60)] {
        if l != if on { n } else { 0 } {
            why.push(format!("day {di}: {name} holds {l} entries (FLAGS {name}={})", u8::from(on)));
        }
    }
    // MUTANT m_e10_window_unguarded: the ids handed to stage A and the kernel not checked against the window (the kernel skips them silently)
    if !cfg!(feature = "m_e10_window_unguarded") {
        if let Some(&x) = want.iter().find(|&&x| x >= n) {
            why.push(format!("day {di}: stage A asked for local id {x} of {n} (G1)"));
        }
        if let Some((k, _)) = subs.iter().enumerate().find(|(_, s)| s.0.iter().any(|&x| x as usize >= n)) {
            why.push(format!("day {di}: list position {k}'s sub-list holds an id outside the window (G1)"));
        }
    }
    if let Some(&x) = want.iter().find(|&&x| sr.evs.get(x).is_some_and(|e| usize::from(e.di) != di)) {
        why.push(format!("day {di}: event {x} of the window carries day {}", sr.evs.get(x).map_or(0, |e| e.di)));
    }
    why
}

/// Runs [`guard`] and counts its misses.
#[allow(clippy::too_many_arguments)]
pub fn guard_count(sr: &Seg3, id: WinId, d: &DLine, fl: Flags, di: usize, loaded: bool, want: &[usize], subs: &[(Vec<u32>, Vec<u32>)], at: &str) {
    for w in guard(sr, id, d, fl, di, loaded, want, subs) {
        violation(&format!("{at}: {w}"));
    }
}

/// M3 at a batch's end: every list's running hasher = its M_u; a difference is a counted violation (no batch file is written).
pub fn m3_close(ship: &Ship, blist: &[usize], m3: Vec<Sha256>, bk: usize) {
    for (h, &u) in m3.into_iter().zip(blist) {
        let got = h.hex();
        if ship.u.get(u).is_none_or(|x| x.m_u != got) {
            violation(&format!("batch {bk}: list {u}'s walked parts hash {got}, not its M_u (M3)"));
        }
    }
}

/// The provenance keys of an E10 run (outside the body; none when E10 is off: P0).
#[must_use]
pub fn prov_extra() -> Vec<(String, String)> {
    if FROM.get().is_none() {
        return Vec::new();
    }
    let hwm = std::fs::read_to_string("/proc/self/status").ok().and_then(|t| t.lines().find_map(|l| l.strip_prefix("VmHWM:")).and_then(|r| r.split_whitespace().next().map(str::to_owned))).unwrap_or_else(|| "-".to_owned());
    vec![
        ("setup".to_owned(), SHIP.get().map_or_else(|| "-".to_owned(), |s| s.v2.clone())),
        ("hwm_kb".to_owned(), hwm),
        ("arena".to_owned(), std::env::var("GLIBC_TUNABLES").ok().filter(|x| !x.is_empty()).unwrap_or_else(|| "-".to_owned())),
    ]
}

/// The E10 words of a run's end (RUN.log; never in RESULTS).
#[must_use]
pub fn words() -> String {
    let nd = DECODES.load(Ordering::Relaxed);
    let ns = DECODE_NS.load(Ordering::Relaxed);
    format!("E10 SHIP_R: {nd} day window(s) decoded (mean {:.1} ms, MEASURED); violations {} (M2 / G1 / M3 / G2: 0 = every window and every walked part bound to the store); windows audited (G2) {}", if nd > 0 { ns as f64 / nd as f64 / 1e6 } else { 0.0 }, VIOL.load(Ordering::Relaxed), AUDITED.load(Ordering::Relaxed))
}

// ------------------------------------------------------------------------------------------------ R2 LV_STREAM: the causal levels ring

/// R2: the causal levels of one segment computed as the days arrive: the last W present days' samples only (`fix33::causal_levels`'
/// window `present[k - min(k, W) .. k]`, never the day itself).
pub struct LvRing {
    w: usize,
    k: usize,
    win: VecDeque<Arc<crate::fix33::DaySmp>>,
    pub cl: Vec<Option<(Levels, usize)>>,
}

impl LvRing {
    /// A ring for `ndays` run days and window `w`.
    #[must_use]
    pub fn new(ndays: usize, w: usize) -> Self {
        Self { w: w.max(1), k: 0, win: VecDeque::new(), cl: vec![None; ndays] }
    }

    /// Run day `d`'s pass-0 samples (in run-day order). CRITIC S11: a day counts as PRESENT exactly when its samples are non-empty (the
    /// predicate of report3.rs 803-809), whatever their timeframes hold.
    pub fn feed(&mut self, d: usize, smp: crate::fix33::DaySmp) {
        // MUTANT m_e10_lvstream_presence_pred: present = "some sample in some timeframe" instead of the pass-0 predicate
        let present = if cfg!(feature = "m_e10_lvstream_presence_pred") { smp.iter().any(|(a, _)| !a.is_empty()) } else { !smp.is_empty() };
        if !present {
            return;
        }
        // MUTANT m_e10_lvstream_includes_today: the day's own samples in its window (look-ahead)
        if cfg!(feature = "m_e10_lvstream_includes_today") {
            self.win.push_back(Arc::new(smp.clone()));
            while self.win.len() > self.w {
                self.win.pop_front();
            }
        }
        if self.k > 0 {
            let take = self.k.min(self.w);
            let refs: Vec<&crate::fix33::DaySmp> = self.win.iter().skip(self.win.len().saturating_sub(take)).map(|a| &**a).collect();
            if let Some(x) = self.cl.get_mut(d) {
                *x = Some((crate::fix33::levels_of(&refs), refs.len()));
            }
        }
        if !cfg!(feature = "m_e10_lvstream_includes_today") {
            self.win.push_back(Arc::new(smp));
            while self.win.len() > self.w {
                self.win.pop_front();
            }
        }
        self.k += 1;
    }
}

// ------------------------------------------------------------------------------------------------ R1 BHAV_LRU: the bounded memo

/// R1: one bounded memo lookup: `cap` 0 = unbounded (v3.3's never-evicted cache); otherwise at most `cap` entries, the least recently used
/// out. `load` is a pure function of the key (a sha-checked file), so a miss after an eviction returns the same value.
///
/// # Errors
/// The loader's.
pub fn lru_get<T>(cache: &Mutex<HashMap<i64, Arc<T>>>, ticks: &Mutex<(u64, HashMap<i64, u64>)>, cap: usize, key: i64, load: &dyn Fn(i64) -> Result<Arc<T>, String>) -> Result<Arc<T>, String> {
    if let Ok(g) = cache.lock()
        && let Some(x) = g.get(&key)
    {
        if cap > 0
            && let Ok(mut t) = ticks.lock()
        {
            t.0 += 1;
            let now = t.0;
            t.1.insert(key, now);
        }
        return Ok(Arc::clone(x));
    }
    let v = load(key)?;
    if let Ok(mut g) = cache.lock() {
        if cap > 0
            && let Ok(mut t) = ticks.lock()
        {
            while g.len() >= cap && !g.contains_key(&key) {
                let Some((&old, _)) = t.1.iter().filter(|e| g.contains_key(e.0)).min_by_key(|e| *e.1) else { break };
                // MUTANT m_e10_bhav_lru_stale: the evicted session's slot keeps its key and takes the new value
                if cfg!(feature = "m_e10_bhav_lru_stale") {
                    g.insert(old, Arc::clone(&v));
                    t.1.remove(&old);
                    t.0 += 1;
                    let now = t.0;
                    t.1.insert(key, now);
                    return Ok(v);
                }
                g.remove(&old);
                t.1.remove(&old);
            }
            t.0 += 1;
            let now = t.0;
            t.1.insert(key, now);
        }
        g.insert(key, Arc::clone(&v));
    }
    Ok(v)
}

/// The days a shard's batch set walks: the union of the plan's lists of its batches.
#[must_use]
pub fn shard_lists() -> Vec<usize> {
    let (Some(p), Some(set)) = (crate::kit::PLAN.get(), crate::kit::BATCH_SET.get()) else { return Vec::new() };
    let mut v: Vec<usize> = set.iter().filter_map(|&k| p.b.get(k)).flat_map(|b| b.3.iter().map(|&u| u as usize)).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// `hex` re-export for the writer's words.
#[must_use]
pub fn hex32(b: &[u8; 32]) -> String {
    hex(b)
}

// ------------------------------------------------------------------------------------------------ run glue (cmd_run3)

fn lg(log: &mut String, s: &str) {
    eprintln!("{s}");
    let _ = writeln!(log, "{s}");
}

/// The year piece of every run day (`prepare_segment`'s rule).
#[must_use]
pub fn pieces_of(days: &[String]) -> Vec<u8> {
    days.iter().map(|d| u8::try_from((crate::cal::parse_day(d).map_or(crate::wf::Y0, |x| crate::cal::civil(x).0) - crate::wf::Y0).clamp(0, i64::try_from(crate::wf::NPIECE - 1).unwrap_or(8))).unwrap_or(0)).collect()
}

/// One prepared day as the store takes it (the screener's per-config entries reduced to their local event ids).
#[must_use]
pub fn day_in(di: usize, o: crate::report3::DayOut) -> DayIn {
    DayIn { di, lists: o.lists.into_iter().map(|l| l.into_iter().map(|(ue, _, _)| ue).collect()).collect(), evs: o.evs, liq: o.liq, tfmask: o.tfmask, rd: o.rd, tfw: o.tfw, vol60: o.vol60, mx: o.mx, prepared: o.prepared }
}

/// SHIP_W: the streamed writer = the plan run. Per chunk of segment days: `prepare_day` (the engine's own per-day set-up), the section
/// encoded, decoded back, compared, compressed; then in day order appended to the store. Then the list stage, v1, the cut and the costs
/// over the stored form, v2, SETUP.tsv, the store renamed from `.part`, and LAST the plan with its SETUP line. Returns the words.
///
/// # Errors
/// Any refusal of the set-up, the codec, the decode-back, the store or the plan.
#[allow(clippy::too_many_arguments)]
pub fn write_setup(store: &mut crate::report3::Store, seg: u8, cal3: &crate::read::Cal3, pre_lv: Option<Levels>, day_lv: Option<Vec<Levels>>, presence: &[[bool; 2]], mman: Option<&crate::mtable::Manifest>, log: &mut String, threads: usize, liqs: &[usize], tf_dep: bool, days: &[String], csets: &[crate::kern::CSet], sel: &[usize], sets: &crate::menu::Sets, menu: &crate::menu::Menu, batch_acc: u64) -> Result<String, String> {
    let t_all = std::time::Instant::now();
    let (bt_utc, cpu0) = (crate::kit::utc_now(), crate::kit::proc_cpu_s());
    let dir = WRITE.get().ok_or("SHIP_W is off")?.clone();
    let part = part_of(&dir);
    let wp = crate::kit::WRITE_PLAN.get().ok_or("SHIP_W needs --write-plan")?.clone();
    if wp.exists() {
        return Err(format!("--write-plan {}: the file exists (never overwritten): refused", wp.display()));
    }
    let inputs = crate::kit::INPUTS_SHA.get().cloned().ok_or("SHIP_W: no inputs manifest (E4) computed")?;
    let sn = ["INDEX options", "STOCK options"].get(usize::from(seg)).copied().unwrap_or("?");
    let seg_days: Vec<usize> = (0..days.len()).filter(|&d| presence.get(d).is_some_and(|p| p.get(usize::from(seg)).copied().unwrap_or(false))).collect();
    let lv = match (pre_lv, day_lv.as_ref()) {
        (Some(l), _) => l,
        (None, Some(dl)) => seg_days.last().and_then(|&d| dl.get(d)).cloned().unwrap_or_default(),
        (None, None) => return Err("SHIP_W needs the streamed pass-0 levels (the streamed --day-major)".to_owned()),
    };
    let fl = own_flags(mman.is_some());
    let ncfg = crate::screen::NCFG;
    let window = threads.clamp(1, 6);
    let mut b = Builder::new(&part, fl, ncfg)?;
    lg(log, &format!("E10 SHIP_W: writing the {sn} set-up of {} segment day(s) to {} (then renamed to {}); FLAGS vol60={} mx={}; {window} day(s) in flight", seg_days.len(), part.display(), dir.display(), u8::from(fl.vol60), u8::from(fl.mx)));
    let tr = std::time::Instant::now();
    for chunk in seg_days.chunks(window) {
        store.ensure(chunk)?;
        let res: Vec<Result<DaySec, String>> = std::thread::scope(|s| {
            let st = &*store;
            let lv = &lv;
            let dl = day_lv.as_deref();
            let hs: Vec<_> = chunk
                .iter()
                .map(|&di| {
                    s.spawn(move || -> Result<DaySec, String> {
                        let (Some(c), Some(tp)) = (st.cubes.get(di), st.tapes.get(di)) else { return Err("day".to_owned()) };
                        let o = crate::report3::prepare_day(c, tp, di, seg, dl.and_then(|v| v.get(di)).unwrap_or(lv), cal3, mman)?;
                        seal_day(day_in(di, o), fl, ncfg, None)
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("writer thread".to_owned()))).collect()
        });
        store.release(&[]);
        for (r, &di) in res.into_iter().zip(chunk) {
            b.push(r?, days.get(di).ok_or("day")?)?;
        }
        crate::kit::strict_check("SHIP_W readings")?;
    }
    b.close()?;
    let t_read = tr.elapsed().as_secs_f64();
    lg(log, &format!("{sn}: {} unique events; {} config entries (screener = v2.1 code, rule set v2)", b.e0, b.nent));
    lg(log, &format!("{sn}: readings at T for {} events ({} contract-days prepared) in {t_read:.1} s; unready: ATR1 {}, ATR5 {}, V300 {}, theta {}", b.e0, b.prepared, b.unready[0], b.unready[1], b.unready[2], b.unready[3]));
    store.sample_rss();
    lg(log, &format!("E10 SHIP_W: {} section(s) sealed (each decoded back and compared); raw {} B, stored (zstd) {} B; per-config lists held as varints {} B (capacity {} B) for {} entries; liquidity readings {} B (capacity {} B); sampled RSS peak so far {} MB", b.d.len(), b.raw_bytes, b.frame_bytes, b.compact.bytes(), b.compact.b.iter().map(Vec::capacity).sum::<usize>(), b.nent, b.liq.len() * 8, b.liq.capacity() * 8, store.rss_peak_kb / 1_024));
    // the list stage over the stored form (the in-memory rule, report3::build_lists_derived)
    let tl = std::time::Instant::now();
    let st = lists_stage(&b.compact, &b.liq, liqs, tf_dep);
    lg(log, &format!("{sn}: {} unique lists over {} (config, liquidity) pairs; removed per liquidity value (entries / unique events): {}", st.uls.len(), st.ul_cfgs.iter().map(Vec::len).sum::<usize>(), liqs.iter().map(|&v| format!("{} {} / {}", crate::read::LIQ_SHORT.get(v).copied().unwrap_or("?"), st.removed.get(v).map_or(0, |x| x.0), st.removed.get(v).map_or(0, |x| x.1))).collect::<Vec<_>>().join("; ")));
    lg(log, &format!("{sn}: v3.3 C3: unique lists DERIVED from {} per-config list entries (no ul_evs stored); E10 list stage over the stored lists {:.1} s", b.compact.cnt.iter().sum::<u64>(), tl.elapsed().as_secs_f64()));
    // v1 = kit::list_digest of the in-memory set-up, from its parts
    let t0 = std::time::Instant::now();
    let pieces = pieces_of(days);
    let ties: Vec<u32> = sel.iter().map(|&i| sets.settings.get(i).map_or(0, |s| if crate::sw::on(crate::sw::TIE52) { crate::report3::tie_key(menu, s) } else { 0 })).collect();
    let search: Vec<bool> = sel.iter().map(|&i| sets.settings.get(i).is_some_and(|s| s.search)).collect();
    let lvd = day_lv.unwrap_or_default();
    let hdr = v1_header(days, &seg_days, &pieces, &sets.sha, &menu.sha, sel, &ties, &search, csets, &lv, &lvd);
    let nev = b.e0;
    let ev_d = std::mem::replace(&mut b.ev, ParStream::new(1)).finish();
    let cfg_d = cfg_digest(&b.compact, threads);
    let (ul_d, mu, mn) = ul_digest(&b.compact, &b.liq, &st, threads);
    if mn != st.ul_n {
        return Err("SHIP_W: a unique list's member count differs between the list stage and the digest pass: refused".to_owned());
    }
    let v1 = v1_assemble(&hdr, nev, &ev_d, ncfg, &cfg_d, st.uls.len(), &ul_d);
    let sshort = ["idx", "stk"].get(usize::from(seg)).copied().unwrap_or("?");
    lg(log, &format!("{sshort}: v3.4 E6 list digest {v1} ({nev} events, {ncfg} config lists, {} unique lists; {:.1} s)", st.uls.len(), t0.elapsed().as_secs_f64()));
    crate::kit::strict_check("set-up")?;
    // the plan: kit_setup's cut and costs (E2 / E11) over the stored form
    let ord = crate::report3::batch_order33(&st.uls, seg);
    let ordered = ord.iter().enumerate().any(|(p, &u)| p != u);
    let per_list = u64::try_from(crate::report3::Layout33::new(crate::sw33::on(crate::sw33::COMPACT)).per_list(csets.len())).unwrap_or(0);
    let lpbo = usize::try_from(crate::kit::LPB.load(Ordering::Relaxed)).unwrap_or(0);
    let (lpb, cut) = if lpbo > 0 {
        (lpbo, format!("--lists-per-batch {lpbo}"))
    } else {
        let l = crate::report3::lpb33(csets.len(), batch_acc);
        (l, format!("v3.3 gate --batch-accounts {batch_acc} on {}-{} (size_of rule) = {l} lists per batch", std::env::consts::ARCH, std::env::consts::OS))
    };
    let nl = ord.len();
    let mut bb = Vec::new();
    let mut p0 = 0;
    while p0 < nl {
        let p1 = (p0 + lpb).min(nl);
        let ids: Vec<u32> = ord.get(p0..p1).unwrap_or(&[]).iter().map(|&u| u32::try_from(u).unwrap_or(u32::MAX)).collect();
        let cost: u128 = ids.iter().map(|&u| u128::from(st.ul_n.get(u as usize).copied().unwrap_or(0))).sum::<u128>() * u128::from(per_list);
        bb.push((p0, p1, cost, ids));
        p0 = p1;
    }
    let mut pl = crate::kit::Plan { path: wp.display().to_string(), sha: String::new(), seg, days: days.to_vec(), nset: csets.len(), nl, digest: v1.clone(), args: crate::kit::ARGS_DIGEST.get().cloned().unwrap_or_default(), order: if ordered { "config-sorted".to_owned() } else { "creation".to_owned() }, per_list, cut, b: bb, setup: String::new() };
    // the store's index and v2
    let ul_lines: Vec<ULine> = st.uls.iter().enumerate().map(|(u, &(rep, v, tfk))| ULine { u, rep, v, tfk, n: st.ul_n.get(u).copied().unwrap_or(0), cfgs: st.ul_cfgs.get(u).cloned().unwrap_or_default(), m_u: mu.get(u).cloned().unwrap_or_default() }).collect();
    std::fs::write(part.join("HDR.bin"), &hdr).map_err(|e| format!("HDR.bin: {e}"))?;
    let meta = Meta { seg, days: days.to_vec(), segdays: seg_days.clone(), nev, ncfg, liqs: liqs.to_vec(), tfdep: tf_dep, flags: fl, args: pl.args.clone(), inputs, v1: v1.clone(), hdr_len: hdr.len() as u64, hdr_sha: sha256_bytes(&hdr) };
    let core = core_text(&meta, &b.d, &ul_lines);
    let v2 = v2_of(&core, &b.d, &ul_lines);
    // every stored frame read back from disk: E_d (the bytes a box will download)
    let tv = std::time::Instant::now();
    for ch in b.d.chunks(threads.clamp(1, 16)) {
        let res: Vec<Result<Vec<u8>, String>> = std::thread::scope(|s| {
            let part = &part;
            let hs: Vec<_> = ch.iter().map(|d| s.spawn(move || read_raw(part, d, true))).collect();
            hs.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("verify thread".to_owned()))).collect()
        });
        for r in res {
            r?;
        }
    }
    lg(log, &format!("E10 SHIP_W: every stored section read back from disk and its E_d verified ({:.1} s)", tv.elapsed().as_secs_f64()));
    store.sample_rss();
    let cpu = crate::kit::proc_cpu_s().or(cpu0).map_or_else(|| "?".to_owned(), |c| format!("{c:.1}"));
    let prov: Vec<(String, String)> = vec![
        ("source".to_owned(), crate::kit::source_sha().to_owned()),
        ("rustc".to_owned(), option_env!("SF_RUSTC").unwrap_or("unset").to_owned()),
        ("target".to_owned(), format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)),
        ("utc_start".to_owned(), crate::kit::utc_label(bt_utc)),
        ("utc_end".to_owned(), crate::kit::utc_label(crate::kit::utc_now())),
        ("wall_s".to_owned(), format!("{:.1}", t_all.elapsed().as_secs_f64())),
        ("cpu_s".to_owned(), cpu),
        ("rss_peak_sampled_kb".to_owned(), store.rss_peak_kb.to_string()),
        ("raw_bytes".to_owned(), b.raw_bytes.to_string()),
        ("stored_bytes".to_owned(), b.frame_bytes.to_string()),
        ("lists_varint_bytes".to_owned(), b.compact.bytes().to_string()),
    ];
    write_setup_tsv(&part, &core, &v2, &prov)?;
    std::fs::rename(&part, &dir).map_err(|e| format!("{} -> {}: {e}", part.display(), dir.display()))?;
    // LAST the plan (a complete store always precedes its plan)
    pl.setup = v2.clone();
    let text = pl.text();
    let pp = wp.with_extension("part");
    std::fs::write(&pp, &text).map_err(|e| format!("{}: {e}", pp.display()))?;
    std::fs::rename(&pp, &wp).map_err(|e| format!("{}: {e}", wp.display()))?;
    let w = format!("PLAN WRITTEN {}: segment {seg}, {} unique lists in {} batches ({}), list digest {}, plan sha256 {}; total cost {} account-event pairs; SETUP WRITTEN {}: v2 {v2} ({} segment days, {nev} events, raw {} B, stored {} B; wall {:.1} s)", wp.display(), nl, pl.b.len(), pl.cut, pl.digest, sha256_bytes(text.as_bytes()), pl.b.iter().map(|x| x.2).sum::<u128>(), dir.display(), seg_days.len(), b.raw_bytes, b.frame_bytes, t_all.elapsed().as_secs_f64());
    lg(log, &w);
    Ok(w)
}

/// SHIP_R: opens and verifies the store (M1) and builds the segment state the walk needs (segment days, pieces, unique lists); every
/// per-event array stays EMPTY (each batch-day's window is swapped in and out).
///
/// # Errors
/// A store that does not verify against the plan and this run.
#[allow(clippy::too_many_arguments)]
pub fn ship_seg(seg: u8, days: &[String], presence: &[[bool; 2]], liqs: &[usize], tf_dep: bool, mx_manifest: bool, log: &mut String) -> Result<Seg3, String> {
    let dir = FROM.get().ok_or("SHIP_R is off")?;
    let plan = crate::kit::PLAN.get().ok_or("SHIP_R needs --batch-plan")?;
    let segdays: Vec<usize> = (0..days.len()).filter(|&d| presence.get(d).is_some_and(|p| p.get(usize::from(seg)).copied().unwrap_or(false))).collect();
    let own = Own { seg, days: days.to_vec(), segdays: segdays.clone(), liqs: liqs.to_vec(), tfdep: tf_dep, flags: own_flags(mx_manifest), args: crate::kit::ARGS_DIGEST.get().cloned().unwrap_or_default(), inputs: crate::kit::INPUTS_SHA.get().cloned().ok_or("SHIP_R: no inputs manifest (E4) computed")?, ncfg: crate::screen::NCFG };
    let ship = Ship::open(dir, &plan.setup, &plan.digest, &own)?;
    lg(log, &format!("E10 SHIP_R: store {} v2 {}: SETUP.tsv parsed (exhaustive); v2 = the plan's SETUP line; V1 = the plan's DIGEST; FLAGS / SEG / DAYS / SEGDAYS / NCFG / NEV / LIQS / TFDEP / ARGS / INPUTS = this run's own (M1); {} segment day(s), {} event(s), {} unique list(s)", dir.display(), ship.v2, ship.d.len(), ship.meta.nev, ship.u.len()));
    let sr = Seg3 {
        days: segdays,
        piece_of_day: pieces_of(days),
        uls: ship.u.iter().map(|x| (x.rep, x.v, x.tfk)).collect(),
        ul_cfgs: ship.u.iter().map(|x| x.cfgs.clone()).collect(),
        ul_derived: true,
        tops: (0..crate::report3::NGROUP).map(|_| crate::acct::Tops::new(200)).collect(),
        ngroup: crate::report3::NGROUP,
        ..Seg3::default()
    };
    if SHIP.set(ship).is_err() {
        return Err("SHIP_R: one store per run".to_owned());
    }
    Ok(sr)
}

/// G2: the in-memory set-up (built only under `--setup-audit on`) attached to the store for the window compare.
pub fn attach_audit(a: Seg3) {
    if let Some(s) = SHIP.get()
        && let Ok(mut g) = s.audit.lock()
    {
        *g = Some(Audit { evs: a.evs, liq: a.liq, tfmask: a.tfmask, rd: a.rd, tfw: a.tfw, vol60: a.vol60, mx: a.mx, lists: a.lists });
    }
}

/// SHIP_R in place of `kit_setup`: the header bound, DIGEST := V1, the plan check against this run (segment, days, settings, order, every
/// list id, V1, the argument digest), then the start check over every segment day for the lists this shard walks.
///
/// # Errors
/// Any refusal.
#[allow(clippy::too_many_arguments)]
pub fn ship_check(sr: &Seg3, seg: u8, days: &[String], csets: &[crate::kern::CSet], sel: &[usize], sets_sha: &str, menu_sha: &str, threads: usize, log: &mut String) -> Result<(), String> {
    let ship = SHIP.get().ok_or("SHIP_R: no store")?;
    let pre = v1_prefix(days, &sr.days, &sr.piece_of_day, sets_sha, menu_sha, sel, &sr.ties, &sr.search, csets);
    ship.header_check(&pre)?;
    let _ = crate::kit::DIGEST.set(ship.meta.v1.clone());
    crate::kit::strict_check("set-up")?;
    let plan = crate::kit::PLAN.get().ok_or("SHIP_R needs --batch-plan")?;
    let ord = crate::report3::batch_order33(&sr.uls, seg);
    let ordered = ord.iter().enumerate().any(|(p, &u)| p != u);
    let per_list = u64::try_from(crate::report3::Layout33::new(crate::sw33::on(crate::sw33::COMPACT)).per_list(csets.len())).unwrap_or(0);
    plan.check(seg, days, csets.len(), &ship.meta.v1, crate::kit::ARGS_DIGEST.get().map_or("", String::as_str), &ord, ordered, per_list)?;
    lg(log, &format!("v3.4 E2: batch plan {} matches this run's set-up ({} batches; segment, days, settings, order, list ids and list digest equal); E10 header bound: this run's v1 header = HDR.bin", plan.path, plan.b.len()));
    let lists = shard_lists();
    let w = ship.start_check(&lists, threads)?;
    lg(log, &w);
    Ok(())
}

/// CRITIC M6(b): the process's high-water mark and RSS (Linux /proc: VmHWM / VmRSS), else the RSS by ps (the Mac: the true peak is
/// `/usr/bin/time -l`'s).
/// CHECK F-1 (the proof of `setup-stats`): it also names the batch's largest batch-day union the walk received (U*, = setup-stats' U_k*)
/// and its run day, then resets it for the next batch.
#[must_use]
pub fn mem_line() -> String {
    let (u, di) = BATCH_UMAX.lock().map(|mut g| std::mem::replace(&mut *g, (0, usize::MAX))).unwrap_or((0, usize::MAX));
    let day = SHIP.get().and_then(|s| s.dline(di)).map_or_else(|| "-".to_owned(), |d| d.date.clone());
    let dis = if di == usize::MAX { "-".to_owned() } else { di.to_string() };
    let uw = format!("batch-day union U* {u} on run day {dis} ({day})");
    if let Ok(t) = std::fs::read_to_string("/proc/self/status") {
        let g = |k: &str| t.lines().find_map(|l| l.strip_prefix(k)).map_or("-", str::trim).to_owned();
        return format!("VmHWM {} ; VmRSS {} ; {uw}", g("VmHWM:"), g("VmRSS:"));
    }
    format!("RSS {} kB (ps; no /proc here: the true peak is /usr/bin/time -l's) ; {uw}", crate::kit::rss_kb().map_or_else(|| "?".to_owned(), |x| x.to_string()))
}

/// ship:shard-memory (CHECK-r2 fix): the memory part of [`mem_line`] alone, for the SHIP_R start-up line (after the E4 inputs manifest, pass 0
/// and the start check; before batch 1). No batch facts, and the batch-day union record stays for batch 1. RUN.log only, never in a body.
#[must_use]
pub fn mem_now() -> String {
    // MUTANT m_e10_mem_now_batch_facts: the start-up line is mem_line's (batch facts; it consumes the first batch's U* record)
    if cfg!(feature = "m_e10_mem_now_batch_facts") {
        return mem_line();
    }
    if let Ok(t) = std::fs::read_to_string("/proc/self/status") {
        let g = |k: &str| t.lines().find_map(|l| l.strip_prefix(k)).map_or("-", str::trim).to_owned();
        return format!("VmHWM {} ; VmRSS {}", g("VmHWM:"), g("VmRSS:"));
    }
    format!("RSS {} kB (ps; no /proc here: the true peak is /usr/bin/time -l's)", crate::kit::rss_kb().map_or_else(|| "?".to_owned(), |x| x.to_string()))
}

/// Opens and verifies a store against a plan with the store's OWN facts standing in for a run's (M1's run-side derivation happens in every
/// shard): `setup-check` and `setup-stats`.
///
/// # Errors
/// An unreadable SETUP.tsv or any refusal of [`Ship::open`].
fn open_as_check(store: &str, pl: &crate::kit::Plan) -> Result<Ship, String> {
    let text = std::fs::read_to_string(Path::new(store).join("SETUP.tsv")).map_err(|e| format!("{store}: {e}"))?;
    let get = |tag: &str| text.lines().find_map(|l| l.strip_prefix(tag).map(str::to_owned));
    let mut own = Own { seg: pl.seg, days: pl.days.clone(), args: pl.args.clone(), ncfg: crate::screen::NCFG, ..Own::default() };
    own.segdays = get("SEGDAYS\t").and_then(|x| x.split_once('\t').map(|y| y.1.split(',').filter(|z| !z.is_empty()).filter_map(|z| z.parse().ok()).collect())).unwrap_or_default();
    own.liqs = get("LIQS\t").map(|x| x.split(',').filter(|z| !z.is_empty()).filter_map(|z| z.parse().ok()).collect()).unwrap_or_default();
    own.tfdep = get("TFDEP\t").as_deref() == Some("1");
    own.flags = Flags { vol60: get("FLAGS\t").is_some_and(|x| x.contains("vol60=1")), mx: get("FLAGS\t").is_some_and(|x| x.contains("mx=1")) };
    own.inputs = get("INPUTS\t").unwrap_or_default();
    Ship::open(Path::new(store), &pl.setup, &pl.digest, &own)
}

/// `spikefade setup-check STORE PLAN [--threads N]`: the whole verification a box does, on EVERY list (the writer's closing check and the
/// Mac's re-check of a downloaded copy): the exhaustive parse, v2, the plan's SETUP and DIGEST, HDR.bin, every section's E_d and L_d, every
/// list's M_u. The store's own facts stand in for a run's (M1's run-side derivation happens in every shard).
///
/// # Errors
/// The first refusal.
pub fn cmd_setup_check(a: &[String]) -> Result<String, String> {
    let (Some(store), Some(plan)) = (a.get(2), a.get(3)) else { return Err("setup-check STORE PLAN [--threads N]".to_owned()) };
    let threads: usize = a.iter().position(|x| x == "--threads").and_then(|i| a.get(i + 1)).and_then(|x| x.parse().ok()).unwrap_or(4);
    let pl = crate::kit::Plan::load(Path::new(plan))?;
    let ship = open_as_check(store, &pl)?;
    let all: Vec<usize> = (0..ship.u.len()).collect();
    let w = ship.start_check(&all, threads)?;
    Ok(format!("SETUP CHECK PASSED: store {store} v2 {} = plan {}'s SETUP line; V1 = its DIGEST {}; {w}", ship.v2, pl.path, pl.digest))
}

// ------------------------------------------------------------------------------------------------ setup-stats (CHECK F-1; DECISION 4.4 / 4.8; CRITIC M6(a))

/// The walk's largest batch-day union of the current batch and its run day (fed by [`ship_parts`], read and reset by [`mem_line`] after
/// each batch file; RUN.log only, never a body): the run-side side of setup-stats' proof.
static BATCH_UMAX: Mutex<(usize, usize)> = Mutex::new((0, usize::MAX));

/// One plan batch's row of `setup-stats`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatRow {
    /// L_k: the batch's lists.
    pub lists: usize,
    /// U_k*: the largest batch-day union over the segment days, |union over the batch's lists u of `day_sub(window(d), rep_u, v_u)`| (EXACT).
    pub umax: u64,
    /// The run day of U_k* (the first such segment day; None when every union is empty).
    pub day: Option<usize>,
    /// That day's events (n_d) and raw section bytes (raw_d).
    pub n_d: u32,
    pub raw_d: u64,
    /// The segment days with a non-empty union (the batch-days the walk visits).
    pub days: u64,
}

/// pred_k in bytes (DECISION 4.8 as CHECK F-1 writes it; an ESTIMATE): L_k x PERLIST x 1,800 B + 1,800 MiB + U_k* x 36.2 KB (36,200 B) +
/// 2 x (n_d x 715 B + raw_d).
#[must_use]
pub fn pred_bytes(lists: usize, per_list: u64, umax: u64, n_d: u32, raw_d: u64) -> u128 {
    (lists as u128) * u128::from(per_list) * 1_800 + 1_800 * 1_048_576 + u128::from(umax) * 36_200 + 2 * (u128::from(n_d) * 715 + u128::from(raw_d))
}

/// U_k* of every plan batch over every segment day of the store: each day decoded once (E_d checked as it is read), the unions by THE walk's
/// derivation [`day_sub`] (the start check's and [`ship_parts`]'); `threads` days at a time; the first day wins a tie (the walk's day order).
///
/// # Errors
/// A list id the store does not hold, an unreadable or damaged section.
pub fn batch_unions(ship: &Ship, plan: &crate::kit::Plan, threads: usize) -> Result<Vec<StatRow>, String> {
    for (k, b) in plan.b.iter().enumerate() {
        if let Some(&u) = b.3.iter().find(|&&u| u as usize >= ship.u.len()) {
            return Err(format!("setup-stats: batch {k} names list {u}; the store holds {} unique lists: refused", ship.u.len()));
        }
    }
    let mut rows: Vec<StatRow> = plan.b.iter().map(|b| StatRow { lists: b.3.len(), ..StatRow::default() }).collect();
    // MUTANT m_e10_stats_first_day: only the first segment day scanned
    let days: &[DLine] = if cfg!(feature = "m_e10_stats_first_day") { ship.d.get(..1).unwrap_or(&[]) } else { &ship.d };
    for ch in days.chunks(threads.clamp(1, 16)) {
        let res: Vec<Result<Vec<u64>, String>> = std::thread::scope(|s| {
            let hs: Vec<_> = ch
                .iter()
                .map(|d| {
                    s.spawn(move || -> Result<Vec<u64>, String> {
                        let w = dec_section(read_raw(&ship.dir, d, true)?, ship.meta.flags, d.e0)?;
                        if w.di != d.di || w.n != d.n {
                            return Err(format!("store: day {} section is day {} with {} events (D line: {}): refused", d.di, w.di, w.n, d.n));
                        }
                        let mut mark = vec![0_u32; w.n as usize];
                        let mut sub = Vec::new();
                        let mut out = Vec::with_capacity(plan.b.len());
                        for (k, b) in plan.b.iter().enumerate() {
                            let stamp = u32::try_from(k + 1).map_err(|_| "setup-stats: batches beyond u32")?;
                            let mut c = 0_u64;
                            for &u in &b.3 {
                                let ul = ship.u.get(u as usize).ok_or("U")?;
                                day_sub(&w, ul.rep, ul.v, &mut sub)?;
                                for &l in &sub {
                                    if let Some(m) = mark.get_mut(l as usize)
                                        && *m != stamp
                                    {
                                        *m = stamp;
                                        c += 1;
                                    }
                                }
                            }
                            out.push(c);
                        }
                        Ok(out)
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("setup-stats thread".to_owned()))).collect()
        });
        for (r, d) in res.into_iter().zip(ch) {
            for (row, c) in rows.iter_mut().zip(r?) {
                if c > 0 {
                    row.days += 1;
                }
                if c > row.umax {
                    row.umax = c;
                    row.day = Some(d.di);
                    row.n_d = d.n;
                    row.raw_d = d.raw;
                }
            }
        }
    }
    Ok(rows)
}

/// The cube bytes of run day `d` in one root: (`.bin`, `.sec.bin`, `.sfz`), 0 for a file the root does not hold.
fn cube_bytes(root: &Path, d: &str) -> [u64; 3] {
    let len = |x: String| std::fs::metadata(root.join(x)).map_or(0, |m| m.len());
    [len(format!("{d}.bin")), len(format!("{d}.sec.bin")), len(format!("{d}.sfz"))]
}

/// `spikefade setup-stats STORE PLAN [--threads N] [--cube DIR] [--cube-extra DIR]` (CHECK F-1; DECISION 4.4 / 4.8; CRITIC M6(a)): the store
/// opened and verified as `setup-check` opens it (v2 = the plan's SETUP line, V1 = its DIGEST, HDR.bin; every section's E_d as it is read).
/// One TSV row per plan batch k: k, L_k, accounts = L_k x PERLIST, U_k* with that day's date, run day, n_d and raw section bytes, the segment
/// days with a non-empty union, and pred_k (MiB, ESTIMATE); then KSTAR (the batch with the largest pred_k, the canary's batch) and LAST
/// CUBEPEAK: the largest-cube day (the `.bin` + `.sec.bin` + `.sfz` bytes of every run day, both roots).
///
/// # Errors
/// The first refusal.
pub fn cmd_setup_stats(a: &[String]) -> Result<String, String> {
    let (Some(store), Some(plan)) = (a.get(2), a.get(3)) else { return Err("setup-stats STORE PLAN [--threads N] [--cube DIR] [--cube-extra DIR]".to_owned()) };
    let opt = |k: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).cloned();
    let threads: usize = opt("--threads").and_then(|x| x.parse().ok()).unwrap_or(4);
    let t0 = std::time::Instant::now();
    let pl = crate::kit::Plan::load(Path::new(plan))?;
    let ship = open_as_check(store, &pl)?;
    let rows = batch_unions(&ship, &pl, threads)?;
    let mib = |b: u128| b.div_ceil(1_048_576);
    let date = |di: Option<usize>| di.and_then(|d| ship.meta.days.get(d)).map_or_else(|| "-".to_owned(), Clone::clone);
    let mut s = String::new();
    let _ = writeln!(s, "# setup-stats (CHECK F-1): store {store} v2 {}; plan {} sha256 {}; {} batch(es), PERLIST {}; {} segment day(s) decoded, every E_d checked; {:.1} s (MEASURED)", ship.v2, pl.path, pl.sha, pl.b.len(), pl.per_list, ship.d.len(), t0.elapsed().as_secs_f64());
    let _ = writeln!(s, "# U_k* = max over the segment days d of |union over the batch's lists u of day_sub(window(d), rep_u, v_u)| (EXACT; the first day on a tie); pred_k (ESTIMATE, MiB) = L_k x PERLIST x 1,800 B + 1,800 MiB + U_k* x 36.2 KB (36,200 B) + 2 x (n_d x 715 B + raw_d)");
    let _ = writeln!(s, "k\tlists\taccounts\tustar\tustar_date\tustar_di\tn_d\traw_d\tdays\tpred_mib");
    let mut kstar: Option<(usize, u128)> = None;
    for (k, r) in rows.iter().enumerate() {
        let p = pred_bytes(r.lists, pl.per_list, r.umax, r.n_d, r.raw_d);
        if kstar.is_none_or(|(_, q)| p > q) {
            kstar = Some((k, p));
        }
        let _ = writeln!(s, "{k}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", r.lists, (r.lists as u128) * u128::from(pl.per_list), r.umax, date(r.day), r.day.map_or_else(|| "-".to_owned(), |d| d.to_string()), r.n_d, r.raw_d, r.days, mib(p));
    }
    match kstar.and_then(|(k, p)| rows.get(k).map(|r| (k, p, r))) {
        Some((k, p, r)) => {
            let _ = writeln!(s, "KSTAR\t{k}\t{}\tpred_k {} MiB (the largest); U_k* {} on {} (n_d {}, raw_d {} B); {} lists", mib(p), mib(p), r.umax, date(r.day), r.n_d, r.raw_d, r.lists);
        }
        None => {
            let _ = writeln!(s, "KSTAR\t-\t-\tno batch in the plan");
        }
    }
    let roots: Vec<(&str, PathBuf)> = [("main", opt("--cube")), ("extra", opt("--cube-extra"))].into_iter().filter_map(|(n, p)| p.map(|x| (n, PathBuf::from(x)))).collect();
    if roots.is_empty() {
        let _ = writeln!(s, "CUBEPEAK\t-\t-\tno cube root given (--cube DIR [--cube-extra DIR])");
        return Ok(s);
    }
    let mut peak: Option<(usize, u64, Vec<[u64; 3]>)> = None;
    for (di, d) in pl.days.iter().enumerate() {
        let parts: Vec<[u64; 3]> = roots.iter().map(|(_, r)| cube_bytes(r, d)).collect();
        let tot: u64 = parts.iter().flatten().sum();
        if peak.as_ref().is_none_or(|p| tot > p.1) {
            peak = Some((di, tot, parts));
        }
    }
    match peak {
        Some((di, tot, parts)) => {
            let what: Vec<String> = roots.iter().zip(&parts).map(|((n, r), [b, c, z])| format!("{n} {}: .bin {b} B, .sec.bin {c} B, .sfz {z} B", r.display())).collect();
            let seg = if ship.dline(di).is_some() { "a segment day" } else { "not a segment day" };
            let _ = writeln!(s, "CUBEPEAK\t{}\t{di}\t{tot}\tthe largest-cube run day of {} ({seg}): {} B in all; {}", date(Some(di)), pl.days.len(), tot, what.join("; "));
        }
        None => {
            let _ = writeln!(s, "CUBEPEAK\t-\t-\tthe plan has no run day");
        }
    }
    Ok(s)
}
