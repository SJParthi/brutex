//! v3 `run3` (EXITS-DYNAMIC-DESIGN as signed with the critic's R1-R6, plus the LIQUIDITY DIAL of MAIN 15 Nov 02:5x): the screener and its
//! events exactly as v2.1, the liquidity dial on every list (7 values, one screener dimension), the readings at T, STAGE A (every menu
//! rule on every event and entry path, once), STAGE B (every account of every list for the chosen setting sets, account-major with exact
//! event skipping), whole or DAY-MAJOR (per day: stage A of the day, then every account of the batch advanced through the day; batches
//! sized by a memory gate), the per-piece window accumulators, and the reports (top accounts per set, view and role; THE pick from SEARCH
//! settings only (R1) with the best COMPARISON setting and the v2 grid's best (S0, s27 baseline) beside; the liquidity table; the stage A
//! per-rule view; the picks' trades recomputed from scratch by THE LAW (proof P5); the P6 scans).
//!
//! Label of every result here: LABELLED DIAGNOSTIC + HINDSIGHT when run on the 8 Tickvault days (every dial chosen on the days it is scored
//! on); Walk-forward NOT RUN (the 21 windows need learning months; the 8 days are all inside the 2026 piece). Nothing goes live.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use common_fill::xrule::TrailIndex;

use crate::acct::{NVIEW, SEv, Sum, Tops, VIEW_LETTERS, VIEW_NAMES};
use crate::mtable::{Lat, Manifest, MarginAt};
use crate::sw;
use crate::cal::{fmt_day, fmt_rs, fmt_sod, fmt_us};
use crate::cube::{Cube, SecTape};
use crate::kern::{CSet, Pieces, detail3, list_keys, walk_account};
use crate::margin::VarRow;
use crate::menu::{FAMILIES, Kind, Menu, Rd, Sets, rule_at};
use crate::read::{Cal3, LIQ_NAMES, LIQ_SHORT, NLIQ, Prep, base_reading, liq_pass, window_reading};
use crate::screen::{self, DayEvents, Ent, Levels, NCFG, TFS, cfg_parts};
use crate::stage::{ABSENT, Counts, EvX, MenuA, NEVER3, Tab3, XKey, absent_word, price_event3, trade3_once, trade3_once_s};
use crate::trade::{self, DayTape, NL, Refuse, SHARE_BP, block_of};

macro_rules! lg {
    ($log:expr, $($a:tt)*) => {{
        let s = format!($($a)*);
        eprintln!("{s}");
        let _ = writeln!($log, "{s}");
    }};
}

const SEG_SHORT: [&str; 2] = ["idx", "stk"];
const SEG_NAME: [&str; 2] = ["INDEX options", "STOCK options"];

fn now_ist() -> String {
    // v3.4 E5: under PORTABLE the system clock + 5 h 30 min (no tzdata; the same label)
    if let Some(s) = crate::kit::now_ist() {
        return s;
    }
    std::process::Command::new("/bin/date").args(["+%Y-%m-%d %H:%M:%S IST"]).env("TZ", "Asia/Kolkata").output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).map_or_else(|| "?".to_owned(), |s| s.trim().to_owned())
}

fn arg<'a>(a: &'a [String], k: &str) -> Option<&'a str> {
    a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str)
}

fn pct(v: &mut [u64], q: u64) -> u64 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    let i = (u64::try_from(v.len() - 1).unwrap_or(0) * q / 100) as usize;
    v.get(i).copied().unwrap_or(0)
}

/// One segment's v3 state.
#[derive(Default)]
pub struct Seg3 {
    pub lv: Levels,
    pub evs: Vec<SEv>,
    pub ex: Vec<EvX>,
    pub tab: Tab3,
    pub lists: Vec<Vec<u32>>,
    pub facts: Vec<Vec<(i64, u64)>>,
    pub tfmask: Vec<u16>,
    pub rd: Vec<Rd>,
    pub tfw: Vec<Vec<(u8, i64, i64)>>,
    pub liq: Vec<(u32, u32)>,
    /// Unique lists: (representative config, liquidity value, tf key) and the event ids.
    pub uls: Vec<(u32, u8, u8)>,
    pub ul_evs: Vec<Vec<u32>>,
    /// Every (config, liquidity value) of a unique list.
    pub ul_cfgs: Vec<Vec<(u32, u8)>>,
    pub cnt: Counts,
    pub days: Vec<usize>,
    /// Per liquidity value: (list entries removed, unique events removed, unique events kept).
    pub removed: [(u64, u64, u64); NLIQ],
    pub tops: Vec<Tops>,
    pub steps: u64,
    pub accounts: u64,
    pub t_stage_a: f64,
    pub t_stage_b: f64,
    pub t_read: f64,
    pub rewalks: u64,
    pub stage_a_bytes: usize,
    /// The 21 window rows (WINDOWS-MWF-v1) from this sweep.
    pub win: Vec<WinRes>,
    /// The year piece (year - 2018, 0..8) of every cube day index of the run.
    pub piece_of_day: Vec<u8>,
    /// v3.1 item 1: per event, its exact margin at T (empty under the proxy).
    pub mx: Vec<MarginAt>,
    /// v3.1: per run day index, the margin snapshot labels and the fallback session (a day without a table).
    pub mlabels: Vec<Vec<String>>,
    pub mfallback: Vec<Option<i64>>,
    /// v3.1: the margin lookup latencies (p50 / p99 printed).
    pub mlat: Lat,
    /// D1 (check-build-v3): the events whose stage A counts were merged (the day-major mode counts each event once).
    pub counted: Vec<bool>,
    /// D1: the events in at least one unique list.
    pub in_any: Vec<bool>,
    /// D3: the account-rows the day-major slack check flagged, re-walked exactly by a second streamed pass.
    pub flagged: Vec<Flag>,
    /// The account groups kept (8; 9 with the v3.1 HEADLINE group).
    pub ngroup: usize,
    /// check-build-v3 S1: per selected setting its design 5.2 tie key (families on, SL kind rank, SL level); 0s under the v3 ties.
    pub ties: Vec<u32>,
    /// v3.1: per selected setting, SEARCH (the headline group reads it).
    pub search: Vec<bool>,
    /// v3.2 addendum 14 (b): per event, the volume of the 60 s strictly before T (the coverage gate's reading; empty with LIQ14 off).
    pub vol60: Vec<u64>,
    /// v3.2 addendum 15: per stage B work item, ns per account-event step x 1,000 (the O(1) evidence: p50 / p99 printed; SEARCH15 only).
    pub step_ns: Vec<u64>,
    /// v3.2 addendum 17: per session the universe of the segment: (day index, underlyings, option contracts, contracts with a print,
    /// expiries listed).
    pub universe: Vec<(usize, u32, u32, u32, u32)>,
    /// v3.2-B (addendum 18): the split permutations of this segment (every split's pick; SPLITS18 only).
    pub sp18: crate::split18::Sel18,
    /// REPAIR A (check-engine-2 S5): sampled step blocks of the stage B kernel: (wall ns, steps) per block.
    pub step_blocks: Vec<(u64, u64)>,
    /// REPAIR A (check-engine-2 S3): per slot index, (accounts walked, accounts that booked at least one trade).
    pub n_booked: [(u64, u64); 4],
    /// REPAIR A (speed-up 4): (accounts covered, accounts walked) by the exact setting dedup in stage B.
    pub dedup: (u64, u64),
    /// v3.3 C3 (DERIVED): the unique lists are not stored (`ul_evs` empty); each is derived from its representative pair's per-config list
    /// filtered by that pair's liquidity value (`build_lists`' own definition), and `lists` is kept for the run.
    pub ul_derived: bool,
    /// v3.3 C4a (TABLEL): Table L summed online: per liquidity value, per share level (units filled, units wanted) over every event passing
    /// that value, added at each event's FIRST counted pricing; the events' fill lists are dropped.
    pub tl_online: bool,
    pub tl_sum: Vec<[(u128, u128); NL]>,
    /// v3.3 fix LV_CAUSAL (check-engine-2 M1 / check-engine-1 M2r): per run day index, the screener levels that day used (empty with the
    /// switch off: `lv` for every day).
    pub lvd: Vec<Levels>,
    /// v3.3 fix CAP_MIN (check-engine-1 M1r): (accounts checked minute by minute, accounts excluded) and one line per excluded account.
    pub cap33: (u64, u64),
    pub cap33_lines: Vec<String>,
    /// v3.3 fix STEP_DM (check-engine-2 M3): the day-major walk's sampled account-day blocks (a bounded uniform subsample).
    pub step33: crate::fix33::StepAcc,
}

impl Seg3 {
    /// v3.3 fix LV_CAUSAL: the screener levels of run day `di` (the run-wide `lv` with the switch off).
    #[must_use]
    pub fn lv_at(&self, di: usize) -> &Levels {
        self.lvd.get(di).unwrap_or(&self.lv)
    }
}

/// v3.3 C4a: one event's fill list added to the online Table L sums of every liquidity value it passes (readings `(c, g)`).
pub fn tl_add(sum: &mut [[(u128, u128); NL]], fill: &[(u8, u8, u64, u64)], c: u32, g: u32) {
    for (v, row) in sum.iter_mut().enumerate() {
        if !liq_pass(v, c, g) {
            continue;
        }
        for &(l, _, u, w) in fill {
            if let Some(x) = row.get_mut(usize::from(l)) {
                x.0 += u128::from(u);
                // MUTANT m_v33_tablel_want_as_got: the wanted units summed as the filled ones
                x.1 += if cfg!(feature = "m_v33_tablel_want_as_got") { u128::from(u) } else { u128::from(w) };
            }
        }
    }
}

impl Seg3 {
    /// The event ids of unique list `u`: borrowed from `ul_evs` (round B), or DERIVED (v3.3 C3) = the representative (config, liquidity)
    /// pair's per-config list filtered by that liquidity value, exactly as `build_lists` built `ul_evs[u]`.
    #[must_use]
    pub fn ul(&self, u: usize) -> Option<std::borrow::Cow<'_, [u32]>> {
        if self.ul_derived {
            let &(id, v, _) = self.uls.get(u)?;
            let l = self.lists.get(id as usize)?;
            // MUTANT m_v33_c3_derived_wrong_rep: the list derived from the next config's list
            let l = if cfg!(feature = "m_v33_c3_derived_wrong_rep") { self.lists.get(id as usize + 1).unwrap_or(l) } else { l };
            let v = usize::from(v);
            Some(std::borrow::Cow::Owned(l.iter().copied().filter(|&e| self.liq.get(e as usize).is_some_and(|&(c, g)| crate::read::liq_pass(v, c, g))).collect()))
        } else {
            self.ul_evs.get(u).map(|l| std::borrow::Cow::Borrowed(l.as_slice()))
        }
    }

    /// v3.3 C3: the day-`di` part of unique list `u` (event ids and their keys), derived or sliced; `out` / `outk` are cleared first.
    pub fn ul_day(&self, u: usize, di: usize, whole_keys: Option<&[u32]>, out: &mut Vec<u32>, outk: &mut Vec<u32>) {
        out.clear();
        outk.clear();
        let lo = u32::try_from(di).unwrap_or(0) * 86_400;
        let hi = lo + 86_400;
        if self.ul_derived {
            let Some(&(id, v, _)) = self.uls.get(u) else { return };
            let Some(l) = self.lists.get(id as usize) else { return };
            let l = if cfg!(feature = "m_v33_c3_derived_wrong_rep") { self.lists.get(id as usize + 1).unwrap_or(l) } else { l };
            let key = |e: u32| self.evs.get(e as usize).map_or(0, |x| u32::from(x.di) * 86_400 + x.t);
            let (a, b) = (l.partition_point(|&e| key(e) < lo), l.partition_point(|&e| key(e) < hi));
            let v = usize::from(v);
            for &e in l.get(a..b).unwrap_or(&[]) {
                if self.liq.get(e as usize).is_some_and(|&(c, g)| crate::read::liq_pass(v, c, g)) {
                    out.push(e);
                    outk.push(key(e));
                }
            }
        } else if let (Some(list), Some(ks)) = (self.ul_evs.get(u), whole_keys) {
            let (a, b) = (ks.partition_point(|&x| x < lo), ks.partition_point(|&x| x < hi));
            out.extend_from_slice(list.get(a..b).unwrap_or(&[]));
            outk.extend_from_slice(ks.get(a..b).unwrap_or(&[]));
        }
    }

    /// v3.3 C3: the event ids of the flagged account-rows' unique lists for the D3 exact re-walk, DERIVED once per list (C3 never stores
    /// `ul_evs`, which round B's re-walk read); an empty map when the lists are stored (the re-walk then reads `ul_evs` as round B does).
    #[must_use]
    pub fn flag_lists33(&self, lists: &[u32]) -> HashMap<u32, Vec<u32>> {
        // MUTANT m_v33_c3_flag_lists_stored: the flagged lists not derived under C3 (the re-walk then finds no list: every flagged account
        // would walk no event; the plain v3.3 binary of 23:37 had this defect, fixed at 23:39 before any PGO build)
        if !self.ul_derived || cfg!(feature = "m_v33_c3_flag_lists_stored") {
            return HashMap::new();
        }
        lists.iter().filter_map(|&u| self.ul(u as usize).map(|c| (u, c.into_owned()))).collect()
    }
}

/// check-build-v3 S1 (design 5.2): the tie key of a setting: families on (fewer first), the SL kind rank (P < A1 < A5 < H < S < V) and
/// the smaller SL level.
#[must_use]
pub fn tie_key(menu: &Menu, s: &crate::menu::Setting) -> u32 {
    let r = menu.rules.get(usize::from(s.sl));
    let rank = r.map_or(9, |m| match m.kind {
        Kind::SlP => 0,
        Kind::SlA1 => 1,
        Kind::SlA5 => 2,
        Kind::SlH => 3,
        Kind::SlS => 4,
        Kind::SlV => 5,
        _ => 9,
    });
    let lvl = r.map_or(0xFFFF, |m| u32::try_from(m.p1.clamp(0, 0xFFFF)).unwrap_or(0xFFFF));
    (u32::from(s.fams_on) << 24) | (rank << 16) | lvl
}

/// The tie order of a window pick (v3: (list, setting slot x 5 + share, N); S1: (tie key, setting slot, list, share, N)).
#[inline]
fn wtie(sm: &Sum) -> (u32, u16, u32, u16, u8) {
    if sw::on(sw::TIE52) {
        let nl = u16::try_from(NL).unwrap_or(5);
        (sm.tie, sm.x / nl, sm.list, sm.x % nl, sm.n)
    } else {
        (0, 0, sm.list, sm.x, sm.n)
    }
}

/// v3.1 (named tests): the headline gate.
#[must_use]
pub fn headline_gate(search: bool, l: usize, sm: &Sum) -> bool {
    headline_ok(search, l, 1, sm)
}

/// REPAIR A (named tests): the headline gate with the account's liquidity value (0 = the gate OFF).
#[must_use]
pub fn headline_gate_liq(search: bool, l: usize, liq: u8, sm: &Sum) -> bool {
    headline_ok(search, l, liq, sm)
}

/// v3.1 (named tests): the stress table drawdown.
pub fn dd_grouped_pub(v: &mut [(i64, i64)]) -> i64 {
    dd_grouped(v)
}

/// v3.1 decisions (a)-(c): the HEADLINE group (8): a SEARCH (small-stop) setting, share at most 25% (levels 0-1), and the account positive
/// at the gate stress (+2% adverse on both sides).
#[inline]
fn headline_ok(search: bool, l: usize, liq: u8, sm: &Sum) -> bool {
    // REPAIR A (check-engine-1 M2): addendum 14 (b) makes liquidity a HARD gate: no headline with the liquidity gate OFF (value 0)
    // MUTANT m_v32r_headline_liq_off: the liquidity value not read
    if sw::on(sw::HL_STRICT) && liq == 0 && !cfg!(feature = "m_v32r_headline_liq_off") {
        return false;
    }
    // MUTANT m_v31_headline_any_share: the 50-100% share levels admitted to the headline
    let share_ok = cfg!(feature = "m_v31_headline_any_share") || l <= 1;
    // MUTANT m_v31_headline_wide_stop (matrix X04): a wide (COMPARISON) stop admitted to the headline
    let stop_ok = cfg!(feature = "m_v31_headline_wide_stop") || search;
    // v3.2 addendum 14 (d): a headline's count of unfilled exits (units valued, not filled) must be 0
    // MUTANT m_v32_headline_unfilled_ok: unfilled exits admitted to the headline
    let liq_ok = !sw::on(sw::LIQ14) || cfg!(feature = "m_v32_headline_unfilled_ok") || sm.nc_trades == 0;
    stop_ok && share_ok && sm.net_s2 > 0 && liq_ok
}

/// D3: one flagged account-row (window row, list, setting slot, share level, slot count) and the account's summary.
#[derive(Clone, Copy, Debug)]
pub struct Flag {
    pub row: u8,
    pub list: u32,
    pub si: u32,
    pub l: u8,
    pub ni: u8,
    pub sm: Sum,
}

/// v3.4 E8: a segment slice is a SHARD's input only (an ordinary run, the reference and the Mac's resume read whole cubes: their closing
/// passes and RESULTS read whole-day facts of both segments).
fn slice_ok(c: &Cube, d: usize) -> Result<(), String> {
    if c.lim.is_some() && !crate::kit::on(crate::kit::SHARD) {
        return Err(format!("day index {d}: the cube is a segment slice (E8), a shard's input only: refused outside --shard on"));
    }
    Ok(())
}

/// D2 (check-build-v3): the run's days. Whole mode: every cube and seconds tape in memory (as v3). The streamed day-major mode: the slots
/// stay empty and each pass loads the days it works on, a few at a time, and drops them (history no longer needs every cube at once).
pub struct Store {
    cdir: PathBuf,
    days: Vec<String>,
    pub cubes: Vec<Cube>,
    pub tapes: Vec<SecTape>,
    loaded: Vec<bool>,
    pub streamed: bool,
    /// Day loads (a streamed run reloads a day once per pass).
    pub loads: u64,
    /// The process RSS sampled after each pass-day (KB; `ps`), its peak.
    pub rss_peak_kb: u64,
}

impl Store {
    /// The store; whole mode loads every day now (in parallel, as v3).
    ///
    /// # Errors
    /// A cube or tape that cannot be loaded.
    pub fn new(cdir: &Path, days: &[String], streamed: bool) -> Result<Self, String> {
        let n = days.len();
        let mut s = Self { cdir: cdir.to_path_buf(), days: days.to_vec(), cubes: vec![Cube::default(); n], tapes: vec![SecTape::default(); n], loaded: vec![false; n], streamed, loads: 0, rss_peak_kb: 0 };
        if !streamed {
            let all: Vec<usize> = (0..n).collect();
            s.ensure(&all)?;
        }
        Ok(s)
    }

    /// Loads the given day indexes that are not in memory (in parallel).
    ///
    /// # Errors
    /// A cube or tape that cannot be loaded.
    pub fn ensure(&mut self, dis: &[usize]) -> Result<(), String> {
        let want: Vec<usize> = dis.iter().copied().filter(|&d| self.loaded.get(d) == Some(&false)).collect();
        if want.is_empty() {
            return Ok(());
        }
        let cdir = self.cdir.clone();
        let names: Vec<(usize, String)> = want.iter().filter_map(|&d| self.days.get(d).map(|x| (d, x.clone()))).collect();
        let got: Vec<(usize, Result<(Cube, SecTape), String>)> = std::thread::scope(|sc| {
            let hs: Vec<_> = names.iter().map(|(d, name)| {
                let cdir = &cdir;
                (*d, sc.spawn(move || -> Result<(Cube, SecTape), String> { Ok((crate::cube::load_m(cdir, name)?, crate::cube::load_sec_m(cdir, name)?)) }))
            }).collect();
            hs.into_iter().map(|(d, h)| (d, h.join().unwrap_or_else(|_| Err("load thread".to_owned())))).collect()
        });
        for (d, r) in got {
            let (mut c, t) = r?;
            slice_ok(&c, d)?;
            // fix:v33 LOT (standing F1): the day's lots checked against its traded quantum (no-op with the switch off)
            crate::fixg::lot_mark(&mut c, &t);
            if let (Some(a), Some(b), Some(l)) = (self.cubes.get_mut(d), self.tapes.get_mut(d), self.loaded.get_mut(d)) {
                *a = c;
                *b = t;
                *l = true;
            }
            self.loads += 1;
        }
        Ok(())
    }

    /// Streamed mode: drops every loaded day not in `keep` (whole mode keeps all).
    pub fn release(&mut self, keep: &[usize]) {
        if !self.streamed {
            return;
        }
        // MUTANT m_v33f_dm_rss_unprinted (integrate, matrix X61): the streamed pass-days never sample the RSS (no peak to print)
        if !cfg!(feature = "m_v33f_dm_rss_unprinted") {
            self.sample_rss();
        }
        for d in 0..self.loaded.len() {
            if self.loaded.get(d) == Some(&true) && !keep.contains(&d) {
                if let (Some(a), Some(b), Some(l)) = (self.cubes.get_mut(d), self.tapes.get_mut(d), self.loaded.get_mut(d)) {
                    *a = Cube::default();
                    *b = SecTape::default();
                    *l = false;
                }
            }
        }
    }

    /// Samples the process RSS (`ps -o rss=`), keeping its peak.
    pub fn sample_rss(&mut self) {
        // v3.4 E5: under PORTABLE /proc when it exists, else ps by the tool path
        if crate::kit::on(crate::kit::PORTABLE) {
            if let Some(v) = crate::kit::rss_kb() {
                self.rss_peak_kb = self.rss_peak_kb.max(v);
            }
            return;
        }
        let pid = std::process::id().to_string();
        if let Ok(o) = std::process::Command::new("/bin/ps").args(["-o", "rss=", "-p", &pid]).output()
            && let Ok(v) = String::from_utf8_lossy(&o.stdout).trim().parse::<u64>()
        {
            self.rss_peak_kb = self.rss_peak_kb.max(v);
        }
    }

    /// One day's cube alone (the streamed pass 0: specials, presence, screener levels), never kept.
    ///
    /// # Errors
    /// A cube that cannot be loaded.
    pub fn cube_only(&self, d: usize) -> Result<Cube, String> {
        if self.loaded.get(d) == Some(&true) {
            return self.cubes.get(d).cloned().ok_or_else(|| "day".to_owned());
        }
        let c = crate::cube::load_m(&self.cdir, self.days.get(d).ok_or("day")?)?;
        slice_ok(&c, d)?;
        Ok(c)
    }

    /// The number of run days.
    #[must_use]
    pub fn n(&self) -> usize {
        self.days.len()
    }

    /// v3.3 C7a (PREFETCH): the cube dir and day name of day index `d` (a loader thread reads it while the current day is walked).
    #[must_use]
    pub fn src_of(&self, d: usize) -> Option<(PathBuf, String)> {
        Some((self.cdir.clone(), self.days.get(d)?.clone()))
    }

    /// v3.3 C7a: a day loaded by the prefetch thread put in its slot (the same bytes `ensure` would load; counted as one load).
    pub fn put(&mut self, d: usize, mut c: Cube, t: SecTape) {
        // v3.4 E8: a slice outside shard mode is counted as a slice violation (the shard check refuses; an ordinary run never reads one: the
        // loads before it refused by name)
        if slice_ok(&c, d).is_err() {
            crate::kit::SLICE_VIOLATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // fix:v33 LOT (standing F1): as `ensure`
        crate::fixg::lot_mark(&mut c, &t);
        if self.loaded.get(d) == Some(&false)
            && let (Some(a), Some(b), Some(l)) = (self.cubes.get_mut(d), self.tapes.get_mut(d), self.loaded.get_mut(d))
        {
            *a = c;
            *b = t;
            *l = true;
            self.loads += 1;
        }
    }
}

/// One window row's outcome in this sweep.
#[derive(Clone, Debug, Default)]
pub struct WinRes {
    pub row: Option<crate::wf::WinRow>,
    /// The data hold a session in the row's learning pieces (else NOT RUN).
    pub run: bool,
    /// THE pick of the row (SEARCH settings only, R1): the best learning key; its learning and test figures.
    pub best: Option<((i64, i64, i64), Sum, crate::wf::Fig, crate::wf::Fig)>,
    pub checked: u64,
    pub rewalked: u64,
    /// Day-major mode: account-rows the slack check flagged (a fresh start could decide differently) whose re-walk needs a second pass over
    /// the streamed days: left out of the row's pick and counted (the whole mode re-walks them exactly).
    pub pending: u64,
}

/// Offers one SEARCH account's pieces to the window rows: a row reads its composed pieces when the slack check proves a fresh start
/// equal; otherwise `rewalk` (the whole mode: an exact walk from the row's first learning piece) or, without it, the account-row is counted
/// PENDING and left out.
pub(crate) fn offer_windows(win: &mut [WinRes], rows: &[crate::wf::WinRow], p: &[crate::kern::Piece], sm: Sum, rewalk: Option<&dyn Fn(usize) -> Vec<crate::kern::Piece>>) {
    for (w, r) in win.iter_mut().zip(rows) {
        // fix:v33 WIN (Recheck-v33 F1): a sliding row is ranked after the sweep over the candidate set (`cand_whole`)
        if !w.run || crate::fixg::skip_sliding(r.growing) {
            continue;
        }
        let pp: Vec<crate::kern::Piece> = if crate::wf::fresh_equal(p, r.learn.0, r.test.1) {
            w.checked += 1;
            p.to_vec()
        } else if let Some(f) = rewalk {
            w.rewalked += 1;
            f(r.learn.0)
        } else {
            w.pending += 1;
            continue;
        };
        let lf = crate::wf::compose(&pp, r.learn.0, r.learn.1);
        if lf.trades == 0 {
            continue;
        }
        let tf2 = crate::wf::compose(&pp, r.test.0, r.test.1);
        let key = crate::wf::learn_key(&lf);
        if w.best.as_ref().is_none_or(|b| key > b.0 || (key == b.0 && wtie(&sm) < wtie(&b.1))) {
            w.best = Some((key, sm, lf, tf2));
        }
    }
}

/// The account groups whose tops are kept: 0 ALL, 1-4 S1-S4, 5 S0, 6 SEARCH (every set), 7 COMPARISON (every set).
pub const NGROUP: usize = 8;
/// Group names.
pub const GROUP_NAMES: [&str; NGROUP] = ["ALL sets", "S1 CORE", "S2 SL-DYN", "S3 ADD-ON", "S4 PAIRS", "S0 v2 grid (baseline)", "SEARCH settings (THE pick, R1)", "COMPARISON settings (wider stops, never the headline)"];

/// Compiles a setting's rules for the kernel.
#[must_use]
pub fn compile(menu: &MenuA, s: &crate::menu::Setting) -> CSet {
    let mut rules = Vec::with_capacity(s.rules.len());
    let mut sl = 0;
    for (k, &r) in s.rules.iter().enumerate() {
        let r = usize::from(r);
        let e = match menu.tfdep.get(r).copied().flatten() {
            Some(j) => (true, j),
            None => (false, u8::try_from(r).unwrap_or(u8::MAX)),
        };
        if s.labels.get(k) == Some(&0) {
            sl = k;
        }
        rules.push(e);
    }
    CSet { rules, sl }
}

/// `run3 <cube dir> <out dir> <days...> --var <C_VAR1> --holidays <tsv> --holidays2 <tsv> --menu <tsv> --sets <tsv> [--set S1|...]
/// [--liq all|off|0,2,5] [--cut v3|v2] [--threads N] [--segs idx|stk] [--day-major] [--batch-accounts N] [--skip on|off]
/// [--compare-day-major] [--compare-skip] [--top K] [--picks N]`
///
/// # Errors
/// A refused input.
pub fn cmd_run3(a: &[String]) -> Result<String, String> {
    common_fill::refuse_if_red().map_err(|e| e.to_string())?;
    sw::from_args(a)?;
    // v3.3 (Addendum 28.2): the exact speed switches (each off by default: round B FINAL byte for byte)
    crate::sw33::from_args(a)?;
    // v3.3 integrate (Addendum 28.2): the BLOCKING fixes of the re-checks (each off by default: round B FINAL byte for byte; --fix33 = all)
    crate::fix33::from_args(a)?;
    // v3.3 fix:v33 (Addenda 24-28): the BLOCKING items of the v3.3 gates (each off by default: v3.3 0577f612 byte for byte; --fix33g = all)
    crate::fixg::from_args(a)?;
    // v3.4 (build:shard, Addendum 30.3): the shard kit (each switch off by default: v3.3 final 8d9a5b67 byte for byte)
    let w_kit = crate::kit::from_args(a)?;
    if crate::fix33::on(crate::fix33::CAP_MIN) && !sw::on(sw::PORT_MTM) {
        return Err("--cap-minutes strict (or --fix33) needs the per-minute marks of REPAIR A (--portfolio-mtm on or --v32)".to_owned());
    }
    // v3.2: the dated NSE tables and dials (each inert unless its switch is on)
    let w_freeze = crate::freeze::from_args(a)?;
    let w_ban = crate::ban::from_args(a)?;
    let w_bhav = crate::bhav::from_args(a)?;
    let w_exp = crate::expiry::from_args(a)?;
    // REPAIR A (addendum 20.3): checkpoint and resume of a day-major run
    let w_ck = crate::ckpt::from_args(a)?;
    // v3.2 addenda 13 / 15: the screener's gain measure (the previous-close measure needs the bhavcopies and the FIELDS32 switch)
    match arg(a, "--gain-measure") {
        None | Some("window") => {}
        Some("prevclose") => {
            if !sw::on(sw::FIELDS32) || crate::bhav::BHAV.get().is_none() {
                return Err("--gain-measure prevclose needs --fields32 on (or --v32) and --bhav-dir".to_owned());
            }
            screen::GAIN_MEASURE.store(1, Ordering::Relaxed);
        }
        Some(o) => return Err(format!("--gain-measure window|prevclose, not {o}")),
    }
    if let Some(p) = arg(a, "--tbill") {
        let t = crate::gk::TBill::load(Path::new(p))?;
        let _ = crate::gk::TBILL.set(t);
    }
    if let Some(v) = arg(a, "--shares") {
        let mut m = 0_u8;
        for x in v.split(',') {
            let bp = x.trim().parse::<u16>().map_err(|_| format!("--shares {v}"))? * 100;
            let l = SHARE_BP.iter().position(|&b| b == bp).ok_or_else(|| format!("--shares: {x}% is not a share level (10, 25, 50, 75, 100)"))?;
            m |= 1 << l;
        }
        trade::SHARE_MASK.store(m, Ordering::Relaxed);
    }
    for (k, st) in [("--liq-cover-s", &trade::LIQ_COVER_S), ("--liq-exit-s", &trade::LIQ_EXIT_S)] {
        if let Some(v) = arg(a, k) {
            st.store(v.parse::<i64>().map_err(|_| format!("{k} {v}"))?, Ordering::Relaxed);
        }
    }
    let (Some(cdir), Some(odir)) = (a.get(2), a.get(3)) else { return Err("run3 <cube dir> <out dir> <days...>".to_owned()) };
    let threads: usize = arg(a, "--threads").and_then(|x| x.parse().ok()).unwrap_or(12);
    let days: Vec<String> = a.iter().skip(4).filter(|x| x.len() == 10 && x.starts_with("20")).cloned().collect();
    // v3.1 (matrix X63): the day index is 16 bits (v2.1 had 8: session 256 wrapped to day 0); the event key day x 86,400 + T is a u32, so a span
    // is refused by name beyond 40,000 sessions (never wrapped)
    if days.len() > 40_000 {
        return Err(format!("{} days: more than 40,000 sessions overflow the u32 event key (day x 86,400 + T): refused", days.len()));
    }
    let var_path = arg(a, "--var").ok_or("--var <C_VAR1 file> is required")?.to_owned();
    let hol1 = arg(a, "--holidays").ok_or("--holidays <NSE holiday tsv> is required")?.to_owned();
    let hol2 = arg(a, "--holidays2").ok_or("--holidays2 <NSE holiday tsv 2020-2025> is required (critic R5)")?.to_owned();
    let menu_p = arg(a, "--menu").ok_or("--menu <EXITS-DYN-MENU-v1.tsv> is required")?.to_owned();
    let sets_p = arg(a, "--sets").ok_or("--sets <EXITS-DYN-SETS-v1.tsv> is required")?.to_owned();
    let set_sel = arg(a, "--set").unwrap_or("S1").to_owned();
    let compat = match arg(a, "--cut") {
        Some("v2") => true,
        Some("v3") | None => false,
        Some(o) => return Err(format!("--cut v3|v2, not {o}")),
    };
    let liqs: Vec<usize> = match arg(a, "--liq") {
        Some("all") | None => (0..crate::read::nliq()).collect(),
        Some("off") => vec![0],
        Some(s) => s.split(',').map(|x| x.parse::<usize>().map_err(|_| format!("--liq {s}"))).collect::<Result<Vec<_>, _>>()?,
    };
    if liqs.iter().any(|&v| v >= NLIQ) || liqs.is_empty() {
        return Err("--liq values 0..6".to_owned());
    }
    let segsel: Vec<u8> = match arg(a, "--segs") {
        Some("idx") => vec![0],
        Some("stk") => vec![1],
        _ => vec![0, 1],
    };
    let day_major = a.iter().any(|x| x == "--day-major");
    // D2: the day-major mode streams the days (each pass loads the days it works on and drops them); --day-major-inmem = v3's in-memory
    // day-major (with D1 and D3 applied), kept for the D2 equality proof only
    let stream = day_major && !a.iter().any(|x| x == "--day-major-inmem");
    let cmp_dm = a.iter().any(|x| x == "--compare-day-major");
    let cmp_skip = a.iter().any(|x| x == "--compare-skip");
    let skip = !matches!(arg(a, "--skip"), Some("off"));
    let topk: usize = arg(a, "--top").and_then(|x| x.parse().ok()).unwrap_or(200);
    let batch_acc: u64 = arg(a, "--batch-accounts").and_then(|x| x.parse().ok()).unwrap_or(40_000_000);
    let mman: Option<Manifest> = match arg(a, "--margin-manifest") {
        Some(p) if sw::on(sw::MARGIN_EXACT) => Some(Manifest::load(Path::new(p))?),
        _ => None,
    };
    // margin rule v2 (addendum 9): the futures table for the index exposure base
    if sw::on(sw::MARGIN_V2) && sw::on(sw::MARGIN_EXACT) {
        match arg(a, "--margin-futures") {
            Some(p) => {
                let f = crate::mtable::Futures::load(Path::new(p))?;
                let _ = crate::mtable::FUTURES.set(f);
            }
            None => return Err("--margin-rule v2 (or --v31) needs --margin-futures <futures-in-files.tsv> (index exposure base = the near-month future in the file)".to_owned()),
        }
    }
    let started = now_ist();
    let run_id = format!("sf3-{}{}{}", started.replace(['-', ':'], "").replace(' ', "-").trim_end_matches("-IST"), if compat { "-cutv2" } else { "" }, if sw::any32() { "-v32" } else if sw::any() { "-v31" } else { "" });
    let rdir = PathBuf::from(odir).join("runs-v3").join(&run_id);
    std::fs::create_dir_all(&rdir).map_err(|e| e.to_string())?;
    let mut log = String::new();
    let exe = std::env::current_exe().map(|p| crate::cube::sha256(&p)).unwrap_or_else(|_| "?".to_owned());
    let menu = Menu::load(Path::new(&menu_p))?;
    let sets = Sets::load(Path::new(&sets_p), &menu)?;
    let ma = MenuA::new(&menu.rules)?;
    let sel = sets.select(&set_sel)?;
    let csets: Vec<CSet> = sel.iter().filter_map(|&i| sets.settings.get(i)).map(|s| compile(&ma, s)).collect();
    let tf_dep_sel = sel.iter().filter_map(|&i| sets.settings.get(i)).any(|s| crate::menu::tf_dependent(menu.rules.get(usize::from(s.sl)).map_or(Kind::SlP, |r| r.kind)));
    lg!(log, "run {run_id} started {started}; engine spikefade v3 (EXITS-DYNAMIC as signed with critic R1-R6 + the liquidity dial); fill crate {} {} + {} + {} {} (law {}); binary sha256 {exe}", common_fill::VERSION, common_fill::RELEASE, common_fill::FA_CHANGE, common_fill::RELEASE_V22, common_fill::XRULE_CHANGE, common_fill::law_compiled());
    if sw::any() {
        lg!(log, "ENGINE v3.1 = v3 + the v2.2 delta (MAIN-RULINGS-ENGINE-v22) + the operator's decisions (a)-(d) of 15 Nov; crate {} {} (margin {}); switches: {}", common_fill::RELEASE_V23, common_fill::V23_CHANGES, common_fill::margin::MARGIN_CHANGE, sw::words());
    }
    if sw::any32() {
        lg!(log, "ENGINE v3.2 = v3.1 + MAIN-RULINGS-ENGINE-v22 addenda 8b, 10-16 + the v3.1 check's M1-M4 / S1-S8; crate {} {}; v3.2 switches: {}", common_fill::RELEASE_V24, common_fill::V24_CHANGES, sw::words32());
        lg!(log, "v3.2 tables: freeze {}; ban {}; bhavcopy {}; T-bill {}; {w_exp}; liquidity cover F {} s / exit Fx {} s at the 25% share", w_freeze.as_deref().unwrap_or("-"), w_ban.as_deref().unwrap_or("-"), w_bhav.as_deref().unwrap_or("-"), crate::gk::TBILL.get().map_or("-", |t| t.source.as_str()), trade::LIQ_COVER_S.load(Ordering::Relaxed), trade::LIQ_EXIT_S.load(Ordering::Relaxed));
    }
    if let Some(w) = &w_ck {
        lg!(log, "REPAIR A: {w}");
    }
    for w in &w_kit {
        lg!(log, "{w}");
    }
    if crate::fix33::any() {
        lg!(log, "ENGINE v3.3 FIXES (integrate:v33, Addendum 28.2: the BLOCKING items of check-engine-1r / check-engine-2r; these change results and are printed in RESULTS): {}", crate::fix33::words());
        if crate::fix33::on(crate::fix33::WORDS33) {
            let vw = crate::fix33::vendor_words(Path::new(cdir), &days);
            lg!(log, "v3.3 FIX WORDS33: the run's data by day manifest: {vw}");
            let _ = crate::fix33::VENDOR.set(vw);
        }
        if crate::fix33::on(crate::fix33::CHG33) {
            let pre = days.iter().filter_map(|d| crate::cal::parse_day(d)).filter(|&d| d < crate::fix33::stamp_sourced_from()).count();
            crate::fix33::PRE_STAMP.store(pre, Ordering::Relaxed);
            lg!(log, "v3.3 FIX CHG33: {pre} session(s) before 2020-07-01 (stamp duty UNVERIFIED: conservative 0.02% on both sides); SEBI fee at 50% on the sessions 2020-06-01..2021-03-31");
        }
    }
    // fix:v33: the run's sessions from 2022-07-18 (GST on the SEBI fee) and the dated index exposure rows covering every session
    crate::fixg::GST_DAYS.store(u64::try_from(days.iter().filter_map(|d| crate::cal::parse_day(d)).filter(|&d| d >= crate::cal::days_from_civil(2022, 7, 18)).count()).unwrap_or(0), Ordering::Relaxed);
    if crate::fixg::any() {
        lg!(log, "ENGINE v3.3 GATE FIXES (fix:v33, Addenda 24-28: the BLOCKING items of the v3.3 gates and the re-check; these change results and are printed in RESULTS): {}", crate::fixg::words());
        let iw = crate::fixg::idx_cover(&days.iter().filter_map(|d| crate::cal::parse_day(d)).collect::<Vec<_>>())?;
        if !iw.is_empty() {
            lg!(log, "fix:v33 IDXELM: {iw}");
            let _ = crate::fixg::IDX_WORDS.set(iw);
        }
    }
    if crate::sw33::any() {
        lg!(log, "ENGINE v3.3 speed switches (Addendum 28.2; exact: every result file byte for byte as with them off; never printed in RESULTS): {}", crate::sw33::words());
    }
    if let Some(m) = &mman {
        lg!(log, "margin manifest {} (sha256 {}): {} snapshot tables", m.path, m.sha, m.rows.len());
    }
    lg!(log, "menu {} rows sha256 {}; sets {} settings sha256 {}; selected {set_sel}: {} settings ({} SEARCH, {} COMPARISON, {} S0); tf-dependent SL in the selection: {tf_dep_sel}", menu.len(), menu.sha, sets.settings.len(), sets.sha, sel.len(), sel.iter().filter(|&&i| sets.settings.get(i).is_some_and(|s| s.search)).count(), sel.iter().filter(|&&i| sets.settings.get(i).is_some_and(|s| !s.search)).count(), sel.iter().filter(|&&i| sets.settings.get(i).is_some_and(|s| s.in_s0)).count());
    lg!(log, "cut rule {}; liquidity values {:?}; threads {threads}; day-major {day_major}; event skipping {skip}; days {days:?}", if compat { "v2 COMPATIBILITY (watch to 15:08, no B2: proof P0 reading)" } else { "v3 (watch to 15:10, B2 cancel at a held anchor, no own order from 15:08)" }, liqs.iter().map(|&v| LIQ_SHORT.get(v).copied().unwrap_or("?")).collect::<Vec<_>>());
    // v3.2 round B (addendum 17): the other NSE indices: their cubes from a second root, their SPAN-file margin rows from a second manifest
    // (both appended per day / per snapshot; unset = the run reads exactly what repair A read)
    if let Some(p) = arg(a, "--cube-extra") {
        let _ = crate::cube::EXTRA.set(PathBuf::from(p));
        lg!(log, "ROUND B: extra cube root {p} (the other NSE indices' cubes, merged into each day's cube where held)");
    }
    if let Some(p) = arg(a, "--margin-extra") {
        if mman.is_none() {
            return Err("--margin-extra needs the exact margin (--margin exact + --margin-manifest)".to_owned());
        }
        let m = Manifest::load(Path::new(p))?;
        lg!(log, "ROUND B: extra margin manifest {} (sha256 {}): {} snapshot tables (the other NSE indices' option rows of the same SPAN files, appended to the same session + file's table)", m.path, m.sha, m.rows.len());
        let _ = crate::mtable::EXTRA_MAN.set(m);
    }
    // v3.4 E4 + F4: the inputs manifest (per day: both cube roots' presence and absence and each root's segment-slice sha256; every table
    // input by content) checked against --expect-inputs and named in every batch file's provenance; the C1 tool probes under STRICT
    if crate::kit::on(crate::kit::SHARD) || crate::kit::on(crate::kit::BFILES) || crate::kit::EXPECT_INPUTS.get().is_some() {
        let ti = Instant::now();
        let iseg = segsel.first().copied().unwrap_or(0);
        let text = crate::kit::inputs_manifest(a, Path::new(cdir), &days, iseg, threads)?;
        if let Some(w) = crate::kit::EXPECT_INPUTS.get() {
            crate::kit::check_inputs(&text, w)?;
        }
        let sha = crate::kit::sha256_bytes(text.as_bytes());
        if let Some(d) = crate::ckpt::CKPT_DIR.get() {
            let p = d.join(format!("{}.inputs.tsv", SEG_SHORT.get(usize::from(iseg)).copied().unwrap_or("x")));
            match std::fs::read_to_string(&p) {
                Ok(old) if old == text => {}
                Ok(_) => return Err(format!("{}: the inputs changed since this folder's first run (E4 / F4): refused; give the run a new folder", p.display())),
                Err(_) => std::fs::write(&p, &text).map_err(|e| format!("{}: {e}", p.display()))?,
            }
        }
        lg!(log, "v3.4 E4 inputs manifest: {} day(s), sha256 {sha} ({:.1} s){}", days.len(), ti.elapsed().as_secs_f64(), if crate::kit::EXPECT_INPUTS.get().is_some() { "; equal to --expect-inputs (F4: day sets and contents)" } else { "" });
        let _ = crate::kit::INPUTS_SHA.set(sha);
    }
    if crate::kit::on(crate::kit::STRICT) {
        lg!(log, "v3.4 F4 probes: {}", crate::kit::probe_tools(mman.is_some(), crate::bhav::BHAV.get().is_some())?);
    }
    let t_all = Instant::now();
    let mut store = Store::new(Path::new(cdir), &days, stream)?;
    let var = crate::margin::load_var(Path::new(&var_path), "30102026")?;
    let mut cal3 = Cal3::load(&[Path::new(&hol1), Path::new(&hol2)])?;
    // REPAIR A (check-engine-2 M3): the dated NSE special-session calendar (days to expiry and the theta target never read the loaded days)
    if sw::on(sw::SESS_DATED) {
        let p = arg(a, "--special-sessions").ok_or("--sess-cal dated needs --special-sessions <tsv>")?;
        let sc = crate::expiry::SpecialCal::load(Path::new(p))?;
        cal3.apply_dated(&sc);
        let _ = crate::expiry::SPECIAL.set(sc);
    }
    // pass 0: the special sessions, the segments present each day and (streamed) the screener levels from one cube at a time
    let mut presence: Vec<[bool; 2]> = vec![[false; 2]; days.len()];
    let mut pre_levels: [Option<Levels>; 2] = [None, None];
    // v3.3 fix LV_CAUSAL (check-engine-2 M1 / check-engine-1 M2r): each day's sorted level samples are kept per day (the same samples round B
    // concatenates), and each day's levels come from the previous W sessions only
    let causal = crate::fix33::on(crate::fix33::LV_CAUSAL);
    let mut day_smp: [Vec<Option<crate::fix33::DaySmp>>; 2] = if causal { [vec![None; days.len()], vec![None; days.len()]] } else { [Vec::new(), Vec::new()] };
    if stream {
        let t0 = Instant::now();
        let mut samples: [Vec<(Vec<u64>, Vec<u64>)>; 2] = [vec![(Vec::new(), Vec::new()); TFS.len()], vec![(Vec::new(), Vec::new()); TFS.len()]];
        let all: Vec<usize> = (0..days.len()).collect();
        // v3.4 E8: a non-regular day's span is read from EVERY contract's bars unless the dated calendar holds it, so a segment slice runs
        // only under the dated calendar (the span is then never used)
        let dated = cal3.dated;
        for chunk in all.chunks(threads.clamp(1, 8)) {
            type P0 = (usize, i64, Option<(usize, usize)>, [bool; 2], [Vec<(Vec<u64>, Vec<u64>)>; 2]);
            let res: Vec<Result<P0, String>> = std::thread::scope(|s| {
                let st = &store;
                let segsel = &segsel;
                let hs: Vec<_> = chunk.iter().map(|&d| s.spawn(move || -> Result<P0, String> {
                    let c = st.cube_only(d)?;
                    if c.lim.is_some() && !dated {
                        return Err(format!("day {d}: a segment slice (E8) needs the dated special-session calendar (--sess-cal dated + --special-sessions): a non-regular day's span reads every contract's bars: refused"));
                    }
                    let (mut lo, mut hi) = (usize::MAX, 0_usize);
                    for (i, f) in c.flags.iter().enumerate() {
                        if f & 1 == 1 {
                            lo = lo.min(i % crate::cal::NM);
                            hi = hi.max(i % crate::cal::NM);
                        }
                    }
                    let pres = [c.cons.iter().any(|k| k.kind == 0), c.cons.iter().any(|k| k.kind == 1)];
                    let smp = [if segsel.contains(&0) && pres[0] { crate::report::level_samples(&c, 0) } else { Vec::new() }, if segsel.contains(&1) && pres[1] { crate::report::level_samples(&c, 1) } else { Vec::new() }];
                    let smp = if causal { smp.map(crate::fix33::sort_samples) } else { smp };
                    Ok((d, c.day, (lo <= hi).then_some((lo, hi)), pres, smp))
                })).collect();
                hs.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("pass 0 thread".to_owned()))).collect()
            });
            for r in res {
                let (d, day, span, pres, smp) = r?;
                if cal3.dated {
                    // REPAIR A: the loaded day is verified against the dated table, never written into it
                    cal3.dated_check(day)?;
                    if cfg!(feature = "m_v32r_sess_loaded")
                        && let (Some(sp), false) = (span, cal3.regular(day))
                    {
                        cal3.special.insert(day, sp);
                    }
                } else if let (Some(sp), false) = (span, cal3.regular(day)) {
                    cal3.special.insert(day, sp);
                }
                if let Some(p) = presence.get_mut(d) {
                    *p = pres;
                }
                for (seg, v) in smp.into_iter().enumerate() {
                    if causal {
                        if !v.is_empty()
                            && let Some(x) = day_smp.get_mut(seg).and_then(|ds| ds.get_mut(d))
                        {
                            *x = Some(v);
                        }
                        continue;
                    }
                    if let Some(acc) = samples.get_mut(seg) {
                        for (x, (l, vv)) in acc.iter_mut().zip(v) {
                            x.0.extend(l);
                            x.1.extend(vv);
                        }
                    }
                }
            }
            store.loads += u64::try_from(chunk.len()).unwrap_or(0);
            store.sample_rss();
        }
        if !causal {
            let [s0, s1] = samples;
            pre_levels = [Some(crate::report::levels_from(s0)), Some(crate::report::levels_from(s1))];
        }
        lg!(log, "STREAMED day-major (D2): pass 0 read {} cubes one at a time (specials, segments, screener levels) in {:.1} s; RSS peak so far {} MB", days.len(), t0.elapsed().as_secs_f64(), store.rss_peak_kb / 1_024);
    } else {
        for c in &store.cubes {
            cal3.dated_check(c.day)?;
        }
        cal3.add_specials(&store.cubes);
        for (p, c) in presence.iter_mut().zip(&store.cubes) {
            *p = [c.cons.iter().any(|k| k.kind == 0), c.cons.iter().any(|k| k.kind == 1)];
        }
        if causal {
            for (d, c) in store.cubes.iter().enumerate() {
                for seg in 0..2_u8 {
                    if segsel.contains(&seg)
                        && presence.get(d).is_some_and(|p| p.get(usize::from(seg)).copied().unwrap_or(false))
                        && let Some(x) = day_smp.get_mut(usize::from(seg)).and_then(|ds| ds.get_mut(d))
                    {
                        *x = Some(crate::fix33::sort_samples(crate::report::level_samples(c, seg)));
                    }
                }
            }
        }
    }
    // v3.3 fix LV_CAUSAL: each segment's per-day levels (the previous W sessions only; the first session = warm-up) and their audit file
    let mut day_levels: [Option<Vec<Levels>>; 2] = [None, None];
    if causal {
        let tl = Instant::now();
        let w = crate::fix33::LV_WINDOW.load(Ordering::Relaxed);
        let mut tsv = String::from("day\tsegment\ttimeframe\tprevious_sessions_used\tsamples\tminvol_L1_lots\tL2\tL3\tL4\tminval_V1_paisa\tV2\tV3\n");
        for seg in 0..2_usize {
            if !segsel.contains(&u8::try_from(seg).unwrap_or(0)) {
                continue;
            }
            let ds = std::mem::take(&mut day_smp[seg]);
            let cl = crate::fix33::causal_levels(&ds, w, threads);
            drop(ds);
            let pres: Vec<bool> = presence.iter().map(|p| p.get(seg).copied().unwrap_or(false)).collect();
            let (dl, (np, nw, short)) = crate::fix33::day_levels(&cl, &pres, w);
            for (d, x) in cl.iter().enumerate() {
                if !pres.get(d).copied().unwrap_or(false) {
                    continue;
                }
                for ti in 0..TFS.len() {
                    match x {
                        Some((lv, n)) => {
                            let (a, b) = (lv.minvol.get(ti).copied().unwrap_or([0; 4]), lv.minval.get(ti).copied().unwrap_or([0; 4]));
                            let _ = writeln!(tsv, "{}\t{}\t{}\t{n}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", days.get(d).map_or("?", String::as_str), SEG_SHORT.get(seg).copied().unwrap_or("?"), crate::screen::tf_name(ti), lv.samples.get(ti).copied().unwrap_or(0), a[0], a[1], a[2], a[3], b[1], b[2], b[3]);
                        }
                        None => {
                            let _ = writeln!(tsv, "{}\t{}\t{}\t0\t0\tWARM-UP\tWARM-UP\tWARM-UP\tWARM-UP\tWARM-UP\tWARM-UP\tWARM-UP", days.get(d).map_or("?", String::as_str), SEG_SHORT.get(seg).copied().unwrap_or("?"), crate::screen::tf_name(ti));
                        }
                    }
                }
            }
            lg!(log, "v3.3 FIX LV_CAUSAL {}: screener levels per day from the previous {w} sessions only (check-engine-2 M1 / check-engine-1 M2r): {np} sessions, {nw} warm-up session(s) (no earlier session: no level known, no event), {short} with fewer than {w} earlier sessions; computed in {:.1} s; levels-causal.tsv", SEG_SHORT.get(seg).copied().unwrap_or("?"), tl.elapsed().as_secs_f64());
            if let Some(x) = day_levels.get_mut(seg) {
                *x = Some(dl);
            }
        }
        std::fs::write(rdir.join("levels-causal.tsv"), tsv).map_err(|e| e.to_string())?;
    }
    let cal = crate::rules::Calendar::load(Path::new(&hol1))?;
    // v3.2 addendum 16: the session calendar of the days-to-expiry count (NSE holidays + the run's special sessions)
    let _ = crate::expiry::SESS.set(crate::expiry::Sessions { hol: cal3.hol.clone(), special: cal3.special.keys().copied().collect() });
    // v3.2 addendum 15: the day -> month / week ids of the verdicts (SEARCH15 only; ids from the run's first month / week)
    if sw::on(sw::SEARCH15) {
        let dd: Vec<i64> = days.iter().filter_map(|d| crate::cal::parse_day(d)).collect();
        let (m0, w0) = (dd.first().map_or(0, |&d| crate::v32::month_id(d)), dd.first().map_or(0, |&d| crate::v32::week_of(d)));
        let _ = crate::kern::MONTH_OF.set(dd.iter().map(|&d| u16::try_from(crate::v32::month_id(d) - m0).unwrap_or(u16::MAX)).collect());
        let _ = crate::kern::WEEK_OF.set(dd.iter().map(|&d| u16::try_from(crate::v32::week_of(d) - w0).unwrap_or(u16::MAX)).collect());
    }
    // v3.2-B (addendum 18): the split calendar (run day -> month, week piece cut at month ends) of the split permutations
    if sw::on(sw::SPLITS18) {
        let dd: Vec<i64> = days.iter().filter_map(|d| crate::cal::parse_day(d)).collect();
        crate::split18::set_cal(crate::split18::SCal::from_days(&dd)?);
    }
    // v3.2-B (addendum 17): each index its own account (--idx-und)
    if let Some(w) = crate::side17::from_args(a)? {
        lg!(log, "{w}");
    }
    lg!(log, "loaded {} cubes; C_VAR1 rows {} (sha256 {}); theta calendar {} (years {:?}), special sessions {:?}", if stream { 0 } else { store.cubes.len() }, var.len(), crate::cube::sha256(Path::new(&var_path)), cal3.sources.join(" + "), cal3.years, cal3.special.keys().map(|&d| fmt_day(d)).collect::<Vec<_>>());
    let dial = trade::dial()?;
    let hand = a.iter().any(|x| x == "--hand" || x == "--hand-only");
    // REPAIR A (speed): --speed-probe K
    let probe_k: Option<usize> = arg(a, "--speed-probe").and_then(|x| x.parse::<usize>().ok()).filter(|&k| k > 0);
    let mut probe_lines: Vec<String> = Vec::new();
    let mut segs: Vec<Seg3> = Vec::new();
    for seg in 0..2_u8 {
        if !segsel.contains(&seg) {
            segs.push(Seg3::default());
            continue;
        }
        let sn = SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?");
        let pl = pre_levels.get(usize::from(seg)).cloned().flatten();
        let dl = day_levels.get_mut(usize::from(seg)).and_then(Option::take);
        let mut sr = prepare_segment(&mut store, seg, &cal3, pl, dl, &presence, mman.as_ref(), &mut log, threads)?;
        // v3.3 C3 (DERIVED): the streamed day-major mode keeps the per-config lists and derives each unique list from them
        let derive = stream && crate::sw33::on(crate::sw33::DERIVED);
        // v3.3 C4a (TABLEL): the streamed day-major mode sums Table L online
        if stream && crate::sw33::on(crate::sw33::TABLEL) {
            sr.tl_online = true;
            sr.tl_sum = vec![[(0, 0); NL]; NLIQ];
        }
        if derive {
            build_lists_derived(&mut sr, &liqs, tf_dep_sel, &mut log, sn);
        } else {
            build_lists(&mut sr, &liqs, tf_dep_sel, &mut log, sn);
        }
        sr.ngroup = if sw::on(sw::HEADLINE) { NGROUP + 1 } else { NGROUP };
        sr.ties = sel.iter().map(|&i| sets.settings.get(i).map_or(0, |s| if sw::on(sw::TIE52) { tie_key(&menu, s) } else { 0 })).collect();
        sr.search = sel.iter().map(|&i| sets.settings.get(i).is_some_and(|s| s.search)).collect();
        if stream && !derive {
            // D2: the per-config lists are folded into the unique lists; their memory goes
            sr.lists = Vec::new();
            sr.facts = Vec::new();
        } else if derive {
            sr.facts = Vec::new();
        }
        // v3.4 E2 / E6: the set-up's list digest; the plan checked against it (or written: the run stops here, nothing walked)
        if let Some(w) = kit_setup(&sr, seg, &days, &csets, &sel, &sets.sha, &menu.sha, batch_acc, threads, &mut log)? {
            std::fs::write(rdir.join("RUN.log"), &log).map_err(|e| e.to_string())?;
            return Ok(w);
        }
        if a.iter().any(|x| x == "--setup-only") {
            // v3.3 integrate (check-engine-2 M4: "then re-measure"; Addendum 28.1: >= 1,000,000,000 combos per segment "printed exactly"): the
            // run's OWN set-up (pass 0, prepare_segment, build_lists) and its counts and memory, then no stage A / B (a measurement, no result)
            store.sample_rss();
            let pairs: u128 = sr.ul_cfgs.iter().map(|v| v.len() as u128).sum();
            let nsh = u128::from(trade::SHARE_MASK.load(Ordering::Relaxed).count_ones());
            let combos = pairs * csets.len() as u128 * nsh * 4;
            let cfg_entries: u128 = sr.lists.iter().map(|l| l.len() as u128).sum();
            let ul_events: u128 = if sr.ul_derived { 0 } else { sr.ul_evs.iter().map(|l| l.len() as u128).sum() };
            // the events of every unique list (derived under C3: each list once, in parallel) and the account-event pairs of the walk (each list's
            // events x its accounts: settings x walked shares x 4 N), the exact basis of the stage B projection
            let nul = sr.uls.len();
            let srr = &sr;
            let per = nul.div_ceil(threads.max(1)).max(1);
            let ev_sum: u128 = std::thread::scope(|s| {
                let hs: Vec<_> = (0..nul).step_by(per).map(|a| s.spawn(move || (a..(a + per).min(nul)).map(|u| srr.ul(u).map_or(0, |l| l.len() as u128)).sum::<u128>())).collect();
                hs.into_iter().map(|h| h.join().unwrap_or(0)).sum()
            });
            let acc_per_list = csets.len() as u128 * nsh * 4;
            let line = format!("SETUP-ONLY {sn}: sessions {} ; events {} ; (config, liquidity) pairs with a list {pairs} ; unique lists {nul} ; events in the unique lists {ev_sum} ; settings {} x walked shares {nsh} x N 4 = {acc_per_list} accounts a list ; account-event pairs {} ; COMBOS covered {combos} ({}) ; per-config entries held {cfg_entries} ; unique-list events stored {ul_events}{} ; sampled RSS peak so far {} MB ; set-up wall {:.1} s since run start", sr.days.len(), sr.evs.len(), csets.len(), ev_sum * acc_per_list, if combos >= 1_000_000_000 { "AT LEAST 1,000,000,000: Addendum 28.1 holds" } else { "BELOW 1,000,000,000: Addendum 28.1 FAILS for this set" }, if sr.ul_derived { " (C3: derived, none stored)" } else { "" }, store.rss_peak_kb / 1_024, t_all.elapsed().as_secs_f64());
            lg!(log, "{line}");
            probe_lines.push(line);
            segs.push(Seg3::default());
            drop(sr);
            continue;
        }
        if a.iter().any(|x| x == "--hand-only") {
            let ta = Instant::now();
            store.ensure(&sr.days.clone())?;
            stage_a_whole(&mut sr, &store.cubes, &store.tapes, &var, &dial, &ma, seg, compat, threads)?;
            sr.t_stage_a = ta.elapsed().as_secs_f64();
            lg!(log, "{sn}: STAGE A {} events, {} entry paths, {:.1} s (hand-only: no stage B)", sr.evs.len(), sr.cnt.paths, sr.t_stage_a);
        } else if day_major && !cmp_dm {
            // v3.3: the day-major stage B with the exact speed switches (any of them on); round B's function otherwise (proof P0)
            if crate::sw33::any() {
                stage_b_day_major_v33(&mut sr, &mut store, &var, &dial, &ma, &sets, &sel, &csets, seg, compat, skip, threads, topk, batch_acc, &mut log)?;
                // v3.4 E1: a shard stops here, BEFORE the closing passes (the Mac's ONE resume of the merged checkpoint runs them)
                if crate::kit::on(crate::kit::SHARD) {
                    crate::kit::strict_check("shard end")?;
                    lg!(log, "v3.4 E8 slice guard: {} read(s) outside the loaded segment slice (0 = every read inside); {}", crate::kit::SLICE_VIOLATIONS.load(Ordering::Relaxed), crate::slice::decode_line());
                    let w = format!("SHARD DONE: segment {sn}, batch set {:?} of {} batches: every assigned batch's file written or verified in {}; {} batch(es) walked by this process; no closing pass run (merge with `spikefade ckpt-merge`, then ONE resume)", crate::kit::BATCH_SET.get().map(|s| s.iter().copied().collect::<Vec<_>>()).unwrap_or_default(), crate::kit::PLAN.get().map_or(0, |p| p.b.len()), crate::ckpt::CKPT_DIR.get().map_or_else(String::new, |d| d.display().to_string()), sr.accounts);
                    lg!(log, "{w}");
                    lg!(log, "run {run_id} finished {} after {:.1} s (shard)", now_ist(), t_all.elapsed().as_secs_f64());
                    std::fs::write(rdir.join("RUN.log"), &log).map_err(|e| e.to_string())?;
                    return Ok(w);
                }
            } else {
                stage_b_day_major(&mut sr, &mut store, &var, &dial, &ma, &sets, &sel, &csets, seg, compat, skip, threads, topk, batch_acc, &mut log)?;
            }
            // fix:v33 WIN (Recheck-v33 F1): the sliding rows' bounded candidate set joins the D3 exact re-walk (no-op with the switch off)
            cand_flags(&mut sr);
            d3_rewalk(&mut sr, &mut store, &var, &dial, &ma, &sets, &sel, &csets, seg, compat, threads, &mut log)?;
            d1_rebuild(&mut sr, &mut store, &var, &dial, &ma, seg, compat, threads, hand, &mut log)?;
        } else {
            let ta = Instant::now();
            stage_a_whole(&mut sr, &store.cubes, &store.tapes, &var, &dial, &ma, seg, compat, threads)?;
            sr.t_stage_a = ta.elapsed().as_secs_f64();
            sr.stage_a_bytes = sr.tab.bytes();
            lg!(log, "{sn}: STAGE A {} events, {} entry paths, {} distinct exit keys priced, {:.1} s, tables {} MB", sr.evs.len(), sr.cnt.paths, sr.tab.outs.len(), sr.t_stage_a, sr.stage_a_bytes / 1_000_000);
            if let Some(k) = probe_k {
                // REPAIR A (speed): the speed probe instead of stage B (exactness of SPEED-UP 4 + per-list cost on this run's data)
                let tp = Instant::now();
                let (np, accts, s1, ns1, walked, s2, ns2, bad, evn) = speed_probe(&sr, &csets, k, threads, skip);
                let sn = SEG_SHORT.get(usize::from(seg)).copied().unwrap_or("?");
                let pairs: u128 = sr.ul_cfgs.iter().map(|v| v.len() as u128).sum();
                let line = format!("SPEED-PROBE {sn}: every {k}th of {} unique lists ({} (config, liquidity) pairs) = {np} lists probed ({evn} list events); {accts} accounts (settings {} x shares {} x 4 N); PLAIN stage B kernel: {s1} steps, {:.3} CPU s (process, every thread; {:.2} ns CPU per step); SPEED-UP 4 exact dedup: {walked} accounts walked ({:.1}% of {accts}), {s2} steps, {:.3} CPU s incl. the decision codes ({:.2}x the plain CPU); lists whose digest of every account summary + pieces differs between the two: {bad} (exactness on this slice); stage A {:.1} s; probe wall {:.1} s at {threads} threads", sr.uls.len(), pairs, csets.len(), trade::SHARE_MASK.load(Ordering::Relaxed).count_ones(), ns1 as f64 / 1e9, if s1 > 0 { ns1 as f64 / s1 as f64 } else { 0.0 }, if accts > 0 { walked as f64 * 100.0 / accts as f64 } else { 0.0 }, ns2 as f64 / 1e9, if ns1 > 0 { ns2 as f64 / ns1 as f64 } else { 0.0 }, sr.t_stage_a, tp.elapsed().as_secs_f64());
                lg!(log, "{line}");
                probe_lines.push(line);
            } else {
            stage_b_whole(&mut sr, &sets, &sel, &csets, seg, skip, threads, topk, &mut log)?;
            // fix:v33 WIN (Recheck-v33 F1): the sliding rows ranked over the bounded candidate set (no-op with the switch off)
            cand_whole(&mut sr, &csets);
            }
            if a.iter().any(|x| x == "--dedup-ratio") {
                dedup_ratio(&sr, &csets, threads, &mut log);
            }
            if cmp_skip {
                compare_skip(&sr, &csets, threads, &mut log);
            }
            if cmp_dm {
                compare_day_major(&mut sr, &mut store, &var, &dial, &ma, &sets, &sel, &csets, seg, compat, skip, threads, topk, batch_acc, &mut log)?;
            }
        }
        if !sr.mx.is_empty() {
            lg!(log, "{sn}: {}", sr.mlat.line());
        }
        // v3.3 fix STEP_DM (check-engine-2 M3): the day-major walk's sampled blocks are the segment's step evidence
        if crate::fix33::on(crate::fix33::STEP_DM) && sr.step33.dm {
            sr.step_blocks = sr.step33.blocks.clone();
        }
        // v3.3 fix CAP_MIN (check-engine-1 M1r): the views a pick reads keep only accounts whose margin + running MTM loss never passed the base
        if crate::fix33::on(crate::fix33::CAP_MIN) && probe_k.is_none() && !a.iter().any(|x| x == "--hand-only") && !sr.evs.is_empty() {
            let dd: Vec<i64> = days.iter().filter_map(|d| crate::cal::parse_day(d)).collect();
            cap_filter33(&mut sr, &csets, &dd, &mut log, sn);
        }
        segs.push(sr);
    }
    if a.iter().any(|x| x == "--setup-only") {
        store.sample_rss();
        let mut md = format!("# SETUP-ONLY run {run_id} (v3.3 integrate: the run's own set-up, its exact counts and its memory; no stage A / B, no result row)\n\n");
        for l in &probe_lines {
            md.push_str(&format!("- {l}\n"));
        }
        md.push_str(&format!("- sampled RSS peak {} MB (ps after each pass-day; /usr/bin/time -l gives the true peak)\n", store.rss_peak_kb / 1_024));
        lg!(log, "run {run_id} finished {} after {:.1} s (setup only)", now_ist(), t_all.elapsed().as_secs_f64());
        std::fs::write(rdir.join("RUN.log"), &log).map_err(|e| e.to_string())?;
        std::fs::write(rdir.join("SETUP-ONLY.md"), md).map_err(|e| e.to_string())?;
        return Ok(log);
    }
    if hand {
        set_tfdep(&ma);
        hand_sample(&segs, &mut store, &dial, &menu, compat, &rdir, &mut log)?;
        if a.iter().any(|x| x == "--hand-only") {
            std::fs::write(rdir.join("RUN.log"), &log).map_err(|e| e.to_string())?;
            return Ok(log);
        }
    }
    if probe_k.is_some() {
        let mut md = format!("# SPEED PROBE run {run_id} (REPAIR A; counters and exactness only, no result row)\n\n");
        for l in &probe_lines {
            md.push_str(&format!("- {l}\n"));
        }
        lg!(log, "run {run_id} finished {} after {:.1} s (speed probe)", now_ist(), t_all.elapsed().as_secs_f64());
        std::fs::write(rdir.join("RUN.log"), &log).map_err(|e| e.to_string())?;
        std::fs::write(rdir.join("SPEED-PROBE.md"), md).map_err(|e| e.to_string())?;
        return Ok(log);
    }
    if a.iter().any(|x| x == "--cost-only") {
        // P7-H (critic F11): a TEST month is measured for its cost only: open counters, never a result row
        let mut md = String::new();
        let _ = writeln!(md, "# COST-ONLY run {run_id} (P7: counters only, no result row)\n");
        for (seg, sr) in segs.iter().enumerate() {
            if sr.evs.is_empty() {
                continue;
            }
            let mut d: Vec<u64> = sr.cnt.distinct.iter().map(|&x| u64::from(x)).collect();
            let _ = writeln!(md, "- {}: {} events, readings {:.1} s, stage A {:.1} s ({} paths, distinct keys p50 {} p99 {}), tables or peak tables + states {} MB, {} unique lists, {} accounts, {} steps, stage B {:.1} s ({:.2} s per setting), {:.2} ns per step", SEG_SHORT.get(seg).copied().unwrap_or("?"), sr.evs.len(), sr.t_read, sr.t_stage_a, sr.cnt.paths, pct(&mut d, 50), pct(&mut d, 99), sr.stage_a_bytes / 1_000_000, sr.uls.len(), sr.accounts, sr.steps, sr.t_stage_b, sr.t_stage_b / csets.len().max(1) as f64, if sr.steps > 0 { sr.t_stage_b * 1e9 / sr.steps as f64 } else { 0.0 });
        }
        if stream {
            let _ = writeln!(md, "- STREAMED (D2): {} day loads, sampled RSS peak {} MB", store.loads, store.rss_peak_kb / 1_024);
        }
        lg!(log, "run {run_id} finished {} after {:.1} s (cost only)", now_ist(), t_all.elapsed().as_secs_f64());
        std::fs::write(rdir.join("RUN.log"), &log).map_err(|e| e.to_string())?;
        std::fs::write(rdir.join("COST-ONLY.md"), md).map_err(|e| e.to_string())?;
        return Ok(log);
    }
    let md = write3(&mut log, &rdir, &run_id, &mut store, &var, &dial, &menu, &ma, &sets, &sel, &csets, &segs, &liqs, &cal, compat, &days, threads, mman.as_ref())?;
    if stream {
        store.sample_rss();
        lg!(log, "STREAMED (D2): {} day loads over every pass; sampled RSS peak {} MB (each pass-day; /usr/bin/time -l gives the true peak)", store.loads, store.rss_peak_kb / 1_024);
    }
    lg!(log, "run {run_id} finished {} after {:.1} s", now_ist(), t_all.elapsed().as_secs_f64());
    std::fs::write(rdir.join("RUN.log"), &log).map_err(|e| e.to_string())?;
    std::fs::write(rdir.join("RESULTS-SPIKEFADE-v3.md"), md).map_err(|e| e.to_string())?;
    Ok(log)
}

/// One day's share of a segment's preparation (v2.1's screener and events, the readings at T, the liquidity readings, the window readings,
/// and (v3.1 item 1) every event's exact margin at T).
#[derive(Default)]
struct DayOut {
    evs: Vec<SEv>,
    lists: Vec<Vec<(u32, i64, u64)>>,
    rd: Vec<Rd>,
    liq: Vec<(u32, u32)>,
    tfw: Vec<Vec<(u8, i64, i64)>>,
    tfmask: Vec<u16>,
    mx: Vec<MarginAt>,
    mlabels: Vec<String>,
    mfallback: Option<i64>,
    lat: Lat,
    prepared: usize,
    vol60: Vec<u64>,
    universe: (u32, u32, u32, u32),
}

#[allow(clippy::too_many_arguments)]
fn prepare_day(c: &Cube, tp: &SecTape, di: usize, seg: u8, lv: &Levels, cal3: &Cal3, mman: Option<&Manifest>) -> Result<DayOut, String> {
    let de = screen::generate(c, u16::try_from(di).unwrap_or(0), seg, lv, false, true, None);
    let mut o = DayOut::default();
    // v3.2-B (addendum 17): the index underlyings this day's cube holds (printed with their sessions)
    crate::side17::note_day(c, seg);
    // v3.2 addendum 17: the day's universe of this segment (every contract staged is a candidate of its list)
    {
        let cs: Vec<(usize, &crate::cube::Contract)> = c.cons.iter().enumerate().filter(|(_, k)| k.kind == seg).collect();
        let mut unds: Vec<u16> = cs.iter().map(|(_, k)| k.und).collect();
        unds.sort_unstable();
        unds.dedup();
        let mut exps: Vec<(u16, u32)> = cs.iter().map(|(_, k)| (k.und, k.expiry)).collect();
        exps.sort_unstable();
        exps.dedup();
        // v3.4 E8: direct array reads, counted outside a loaded segment slice
        for (ci, _) in &cs {
            c.guard(*ci);
        }
        let traded = cs.iter().filter(|(ci, _)| (0..crate::cal::NM).any(|m| c.flags.get(Cube::ix(*ci, m)).is_some_and(|f| f & 1 == 1))).count();
        o.universe = (u32::try_from(unds.len()).unwrap_or(0), u32::try_from(cs.len()).unwrap_or(0), u32::try_from(traded).unwrap_or(0), u32::try_from(exps.len()).unwrap_or(0));
    }
    for u in &de.uev {
        let sk = trade::side_key(c, u.ci as usize);
        o.evs.push(SEv { di: u.di, ci: u.ci, t: u.t, skey: sk, tab: trade::EvT::default() });
    }
    o.lists = de.lists.iter().map(|l| l.iter().map(|e| (e.ue, e.gain_bp, e.lots)).collect()).collect();
    o.tfmask = vec![0_u16; o.evs.len()];
    for (id, l) in de.lists.iter().enumerate() {
        let tf = cfg_parts(id).0;
        for e in l {
            if let Some(m) = o.tfmask.get_mut(e.ue as usize) {
                *m |= 1 << tf;
            }
        }
    }
    // readings: per contract-day (one preparation each), the events in (ci, t) order
    let mut idx: Vec<usize> = (0..o.evs.len()).collect();
    idx.sort_by_key(|&i| o.evs.get(i).map_or((0, 0), |e| (e.ci, e.t)));
    o.rd = vec![Rd::default(); o.evs.len()];
    o.liq = vec![(0, 0); o.evs.len()];
    o.tfw = vec![Vec::new(); o.evs.len()];
    let liq14 = sw::on(sw::LIQ14);
    if liq14 {
        o.vol60 = vec![0; o.evs.len()];
    }
    let mut last: Option<(u32, Prep)> = None;
    for i in idx {
        let Some(e) = o.evs.get(i) else { continue };
        if last.as_ref().is_none_or(|x| x.0 != e.ci) {
            last = Some((e.ci, Prep::new(c, e.ci as usize, tp.of(e.ci as usize))));
            o.prepared += 1;
        }
        let Some((_, pr)) = last.as_ref() else { continue };
        let t = i64::from(e.t);
        let m = o.tfmask.get(i).copied().unwrap_or(0);
        if let (Some(a), Some(b), Some(w)) = (o.rd.get_mut(i), o.liq.get_mut(i), o.tfw.get_mut(i)) {
            *a = base_reading(c, e.ci as usize, t, pr, cal3);
            *b = pr.liq_at(t);
            *w = (0..TFS.len()).filter(|&tf| m & (1 << tf) != 0).map(|tf| { let (h, r) = window_reading(c, e.ci as usize, t, TFS.get(tf).copied().unwrap_or(1)); (u8::try_from(tf).unwrap_or(0), h, r) }).collect();
        }
        if liq14 && let Some(v) = o.vol60.get_mut(i) {
            *v = pr.vol60_at(t);
        }
    }
    // v3.1 item 1: every event's exact margin at T (the per-contract-day table through THE margin module; O(1) per decision)
    if let Some(man) = mman {
        let dm = crate::mtable::load_day(man, c.day, true)?;
        let cm = crate::mtable::resolve(&dm, c, &mut o.lat);
        o.mlabels = dm.snaps.iter().map(|x| x.1.clone()).collect();
        o.mfallback = dm.fallback_from;
        for e in &o.evs {
            let Some(k) = c.cons.get(e.ci as usize) else {
                o.mx.push(MarginAt { refuse: Some(trade::Refuse::BadTape), ..MarginAt::default() });
                continue;
            };
            let b = usize::try_from((i64::from(e.t) - crate::cal::OPEN_S) / 60).unwrap_or(0);
            let s = i64::from(c.spot_at(usize::from(k.und), b.saturating_sub(1)));
            let m = if s <= 0 { MarginAt { refuse: Some(trade::Refuse::NoUnderlying), ..MarginAt::default() } } else { crate::mtable::exact_at(&dm, &cm, e.ci as usize, crate::cal::at_sod(c.day, i64::from(e.t)), s, u64::from(k.lot), c.ymd, Some(&mut o.lat)) };
            o.mx.push(m);
        }
    }
    Ok(o)
}

/// Levels, events, lists (v2.1's code), the timeframes of every event, the readings at T, the liquidity readings and (v3.1) the exact
/// margins: per day (whole mode: every day in memory, in parallel; the streamed mode: a few days loaded at a time and dropped, D2).
#[allow(clippy::too_many_arguments)]
fn prepare_segment(store: &mut Store, seg: u8, cal3: &Cal3, pre_lv: Option<Levels>, day_lv: Option<Vec<Levels>>, presence: &[[bool; 2]], mman: Option<&Manifest>, log: &mut String, threads: usize) -> Result<Seg3, String> {
    let sn = SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?");
    let seg_days: Vec<usize> = (0..store.n()).filter(|&d| presence.get(d).is_some_and(|p| p.get(usize::from(seg)).copied().unwrap_or(false))).collect();
    let lv = match (pre_lv, day_lv.as_ref()) {
        (Some(l), _) => l,
        // v3.3 fix LV_CAUSAL: the run-wide reading is never computed (no decision reads it); the last segment day's causal levels stand in
        (None, Some(dl)) => seg_days.last().and_then(|&d| dl.get(d)).cloned().unwrap_or_default(),
        (None, None) => {
            let sub: Vec<&Cube> = seg_days.iter().filter_map(|&i| store.cubes.get(i)).collect();
            crate::report::measure_refs(&sub, seg)
        }
    };
    let tr = Instant::now();
    let window = if store.streamed { threads.clamp(1, 6) } else { seg_days.len().max(1) };
    // v3.3 C2 (NOFACTS): each chunk's days are merged as soon as they are prepared (in day order, exactly the order of the one merge below),
    // and the per-config facts are never built (write-only in run3)
    if crate::sw33::on(crate::sw33::NOFACTS) {
        return prepare_segment_c2(store, seg, lv, day_lv, &seg_days, window, cal3, mman, log, tr);
    }
    let mut outs: Vec<DayOut> = Vec::with_capacity(seg_days.len());
    for chunk in seg_days.chunks(window) {
        store.ensure(chunk)?;
        let res: Vec<Result<DayOut, String>> = std::thread::scope(|s| {
            let st = &*store;
            let lv = &lv;
            let dl = day_lv.as_deref();
            let hs: Vec<_> = chunk.iter().map(|&di| s.spawn(move || match (st.cubes.get(di), st.tapes.get(di)) {
                (Some(c), Some(tp)) => prepare_day(c, tp, di, seg, dl.and_then(|v| v.get(di)).unwrap_or(lv), cal3, mman),
                _ => Err("day".to_owned()),
            })).collect();
            hs.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("prepare thread".to_owned()))).collect()
        });
        for r in res {
            outs.push(r?);
        }
        store.release(&[]);
    }
    let mut evs: Vec<SEv> = Vec::new();
    let mut lists: Vec<Vec<u32>> = vec![Vec::new(); NCFG];
    let mut facts: Vec<Vec<(i64, u64)>> = vec![Vec::new(); NCFG];
    let (mut rd, mut liq, mut tfw, mut tfmask, mut mx) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut vol60: Vec<u64> = Vec::new();
    let mut universe: Vec<(usize, u32, u32, u32, u32)> = Vec::new();
    let mut mlabels: Vec<Vec<String>> = vec![Vec::new(); store.n()];
    let mut mfallback: Vec<Option<i64>> = vec![None; store.n()];
    let mut mlat = Lat::default();
    let mut prepared = 0_usize;
    for (k, o) in outs.into_iter().enumerate() {
        let off = u32::try_from(evs.len()).unwrap_or(0);
        if let Some(&di) = seg_days.get(k) {
            if let Some(x) = mlabels.get_mut(di) {
                *x = o.mlabels;
            }
            if let Some(x) = mfallback.get_mut(di) {
                *x = o.mfallback;
            }
        }
        if let Some(&di) = seg_days.get(k) {
            universe.push((di, o.universe.0, o.universe.1, o.universe.2, o.universe.3));
        }
        evs.extend(o.evs);
        for (id, l) in o.lists.into_iter().enumerate() {
            if let (Some(dst), Some(fd)) = (lists.get_mut(id), facts.get_mut(id)) {
                for (ue, g, lots) in l {
                    dst.push(off + ue);
                    fd.push((g, lots));
                }
            }
        }
        rd.extend(o.rd);
        liq.extend(o.liq);
        tfw.extend(o.tfw);
        tfmask.extend(o.tfmask);
        mx.extend(o.mx);
        vol60.extend(o.vol60);
        mlat.merge(o.lat);
        prepared += o.prepared;
    }
    lg!(log, "{sn}: {} unique events; {} config entries (screener = v2.1 code, rule set v2)", evs.len(), lists.iter().map(Vec::len).sum::<usize>());
    let t_read = tr.elapsed().as_secs_f64();
    let (n1, n5, nv, nt) = (rd.iter().filter(|r: &&Rd| r.atr1.is_none()).count(), rd.iter().filter(|r: &&Rd| r.atr5.is_none()).count(), rd.iter().filter(|r: &&Rd| r.v300.is_none()).count(), rd.iter().filter(|r: &&Rd| r.theta.is_err()).count());
    lg!(log, "{sn}: readings at T for {} events ({prepared} contract-days prepared) in {t_read:.1} s; unready: ATR1 {n1}, ATR5 {n5}, V300 {nv}, theta {nt}", evs.len());
    let ex = vec![EvX::default(); evs.len()];
    let piece_of_day: Vec<u8> = store.days.iter().map(|d| u8::try_from((crate::cal::parse_day(d).map_or(crate::wf::Y0, |x| crate::cal::civil(x).0) - crate::wf::Y0).clamp(0, i64::try_from(crate::wf::NPIECE - 1).unwrap_or(8))).unwrap_or(0)).collect();
    Ok(Seg3 { lv, evs, ex, lists, facts, tfmask, rd, tfw, liq, days: seg_days, cnt: Counts::default(), t_read, tops: (0..NGROUP).map(|_| Tops::new(200)).collect(), piece_of_day, mx, mlabels, mfallback, mlat, ngroup: NGROUP, vol60, universe, lvd: day_lv.unwrap_or_default(), ..Seg3::default() })
}

/// v3.3 C2: one day's per-config screener output appended to the run's per-config lists (event ids shifted by the day's offset `off` in
/// the run's events; the facts (gain, lots) are not kept: write-only in run3).
pub fn append_cfg_lists(lists: &mut [Vec<u32>], day: Vec<Vec<(u32, i64, u64)>>, off: u32) {
    for (id, l) in day.into_iter().enumerate() {
        if let Some(dst) = lists.get_mut(id) {
            // MUTANT m_v33_c2_merge_offset: the day's event ids appended without the running offset
            let add = if cfg!(feature = "m_v33_c2_merge_offset") { 0 } else { off };
            dst.extend(l.into_iter().map(|(ue, _, _)| add + ue));
        }
    }
}

/// v3.3 C1 (COMPACT): the day-major state layout of one list: (setting, walked share level, slot index) -> index into the list's states.
/// Full = round B's (setting x 5 + level) x 4 + N over all 5 levels; compact = only the levels of `--shares`, in level order.
#[derive(Clone, Copy, Debug)]
pub struct Layout33 {
    pub nlv: usize,
    stride: usize,
    lv_pos: [usize; NL],
}

impl Layout33 {
    /// The layout under the run's share mask (`compact` = C1 on).
    #[must_use]
    pub fn new(compact: bool) -> Self {
        Self::with_mask(compact, trade::SHARE_MASK.load(Ordering::Relaxed))
    }

    /// The layout under share mask `mask` (bit l = share level l walked).
    #[must_use]
    pub fn with_mask(compact: bool, mask: u8) -> Self {
        let walked: Vec<usize> = (0..NL).filter(|&l| (mask >> l) & 1 == 1).collect();
        let nlv = if compact { walked.len().max(1) } else { NL };
        let mut lv_pos = [0_usize; NL];
        for (l, x) in lv_pos.iter_mut().enumerate() {
            *x = if compact { walked.iter().position(|&w| w == l).unwrap_or(0) } else { l };
        }
        // MUTANT m_v33_c1_stride_full: the compact allocation indexed with round B's 5-level stride
        let stride = if cfg!(feature = "m_v33_c1_stride_full") { NL } else { nlv };
        Self { nlv, stride, lv_pos }
    }

    /// The states one list holds.
    #[must_use]
    pub const fn per_list(&self, nsettings: usize) -> usize {
        nsettings * self.nlv * 4
    }

    /// The index of (setting `si`, share level `l`, slot index `ni`) inside one list's states.
    #[inline]
    #[must_use]
    pub fn idx(&self, si: usize, l: usize, ni: usize) -> usize {
        (si * self.stride + self.lv_pos.get(l).copied().unwrap_or(0)) * 4 + ni
    }
}

/// v3.3 (LARGEST): the order a day's work items are claimed in: the items with events, largest first (descending sub-list length, ties by
/// batch position), or in batch position order (round B).
#[must_use]
pub fn day_items33(lens: &[usize], largest: bool) -> Vec<usize> {
    let mut items: Vec<usize> = (0..lens.len()).filter(|&k| lens.get(k).copied().unwrap_or(0) > 0).collect();
    if largest {
        items.sort_by_key(|&k| (std::cmp::Reverse(lens.get(k).copied().unwrap_or(0)), k));
        // MUTANT m_v33_largest_drops_ties: equal-length items after the first dropped from the day's queue
        if cfg!(feature = "m_v33_largest_drops_ties") {
            items.dedup_by_key(|k| lens.get(*k).copied().unwrap_or(0));
        }
    }
    items
}

/// v3.3 (checkpoint fingerprint): the v3.3 part (the switch words and the batch order), so a resume never mixes two layouts, orders or list
/// stores.
#[must_use]
pub fn fp_v33(ordered: bool) -> String {
    // MUTANT m_v33_order_not_in_fingerprint: the batch order left out of the fingerprint
    let ord_word = if cfg!(feature = "m_v33_order_not_in_fingerprint") { "" } else if ordered { "; batch order config-sorted" } else { "; batch order creation" };
    format!("; v33 {}{ord_word}", crate::sw33::words())
}

/// v3.3 D2-1: the settings of one class work item (a class's accounts are split into setting chunks: disjoint states, one thread each).
pub const SC33: usize = 32;

/// v3.3 D2-1 (DESIGN-D2 section 3): the day's lists of a batch grouped into CLASSES of equal (timeframe key, day sub-list): FNV hash of the
/// event ids + length + tf key, then a FULL compare against the class's first member (a collision never merges two different sub-lists).
/// Lists with no event that day are in no class. Members in batch order; the first is the class's representative.
#[must_use]
pub fn day_classes33(subs: &[(Vec<u32>, Vec<u32>)], tfs: &[Option<u8>]) -> Vec<Vec<usize>> {
    let mut map: HashMap<(u64, usize, Option<u8>), Vec<usize>> = HashMap::new();
    let mut classes: Vec<Vec<usize>> = Vec::new();
    for (k, (s, _)) in subs.iter().enumerate() {
        if s.is_empty() {
            continue;
        }
        // MUTANT m_v33_d2_class_key_without_tf: lists of different timeframes (tf-dependent stops read other rows) share a class
        let tf = if cfg!(feature = "m_v33_d2_class_key_without_tf") { None } else { tfs.get(k).copied().flatten() };
        let h = crate::kern::fnv(s);
        let cands = map.entry((h, s.len(), tf)).or_default();
        let found = cands.iter().copied().find(|&ci| classes.get(ci).and_then(|m| m.first()).and_then(|&k0| subs.get(k0)).is_some_and(|x| x.0 == *s));
        match found {
            Some(ci) => {
                if let Some(m) = classes.get_mut(ci) {
                    m.push(k);
                }
            }
            None => {
                cands.push(classes.len());
                classes.push(vec![k]);
            }
        }
    }
    classes
}

/// v3.3 C7a (PREFETCH): the day a prefetch loads for day index `d`'s slot: (slot, (cube dir, day name)).
#[must_use]
pub fn pf_src33(store: &Store, d: usize) -> Option<(usize, (PathBuf, String))> {
    // MUTANT m_v33_prefetch_wrong_day: the file of the day after loaded into day d's slot
    let src_day = if cfg!(feature = "m_v33_prefetch_wrong_day") { d + 1 } else { d };
    store.src_of(src_day).map(|s| (d, s))
}

/// v3.3 C2 (NOFACTS): [`prepare_segment`]'s preparation with each chunk of days merged as soon as it is prepared (the chunks come in day
/// order and each chunk's days in order, so every vector receives exactly the sequence of the one merge after all days) and without the
/// per-config facts (built and never read in run3). Every other field is the same.
#[allow(clippy::too_many_arguments)]
fn prepare_segment_c2(store: &mut Store, seg: u8, lv: Levels, day_lv: Option<Vec<Levels>>, seg_days: &[usize], window: usize, cal3: &Cal3, mman: Option<&Manifest>, log: &mut String, tr: Instant) -> Result<Seg3, String> {
    let sn = SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?");
    let mut evs: Vec<SEv> = Vec::new();
    let mut lists: Vec<Vec<u32>> = vec![Vec::new(); NCFG];
    let (mut rd, mut liq, mut tfw, mut tfmask, mut mx) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut vol60: Vec<u64> = Vec::new();
    let mut universe: Vec<(usize, u32, u32, u32, u32)> = Vec::new();
    let mut mlabels: Vec<Vec<String>> = vec![Vec::new(); store.n()];
    let mut mfallback: Vec<Option<i64>> = vec![None; store.n()];
    let mut mlat = Lat::default();
    let mut prepared = 0_usize;
    let mut k = 0_usize;
    for chunk in seg_days.chunks(window) {
        store.ensure(chunk)?;
        let res: Vec<Result<DayOut, String>> = std::thread::scope(|s| {
            let st = &*store;
            let lv = &lv;
            let dl = day_lv.as_deref();
            let hs: Vec<_> = chunk.iter().map(|&di| s.spawn(move || match (st.cubes.get(di), st.tapes.get(di)) {
                (Some(c), Some(tp)) => prepare_day(c, tp, di, seg, dl.and_then(|v| v.get(di)).unwrap_or(lv), cal3, mman),
                _ => Err("day".to_owned()),
            })).collect();
            hs.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("prepare thread".to_owned()))).collect()
        });
        store.release(&[]);
        for r in res {
            let o = r?;
            let off = u32::try_from(evs.len()).unwrap_or(0);
            if let Some(&di) = seg_days.get(k) {
                if let Some(x) = mlabels.get_mut(di) {
                    *x = o.mlabels;
                }
                if let Some(x) = mfallback.get_mut(di) {
                    *x = o.mfallback;
                }
                universe.push((di, o.universe.0, o.universe.1, o.universe.2, o.universe.3));
            }
            evs.extend(o.evs);
            append_cfg_lists(&mut lists, o.lists, off);
            rd.extend(o.rd);
            liq.extend(o.liq);
            tfw.extend(o.tfw);
            tfmask.extend(o.tfmask);
            mx.extend(o.mx);
            vol60.extend(o.vol60);
            mlat.merge(o.lat);
            prepared += o.prepared;
            k += 1;
        }
    }
    lg!(log, "{sn}: {} unique events; {} config entries (screener = v2.1 code, rule set v2)", evs.len(), lists.iter().map(Vec::len).sum::<usize>());
    let t_read = tr.elapsed().as_secs_f64();
    let (n1, n5, nv, nt) = (rd.iter().filter(|r: &&Rd| r.atr1.is_none()).count(), rd.iter().filter(|r: &&Rd| r.atr5.is_none()).count(), rd.iter().filter(|r: &&Rd| r.v300.is_none()).count(), rd.iter().filter(|r: &&Rd| r.theta.is_err()).count());
    lg!(log, "{sn}: readings at T for {} events ({prepared} contract-days prepared) in {t_read:.1} s; unready: ATR1 {n1}, ATR5 {n5}, V300 {nv}, theta {nt}", evs.len());
    let ex = vec![EvX::default(); evs.len()];
    let piece_of_day: Vec<u8> = store.days.iter().map(|d| u8::try_from((crate::cal::parse_day(d).map_or(crate::wf::Y0, |x| crate::cal::civil(x).0) - crate::wf::Y0).clamp(0, i64::try_from(crate::wf::NPIECE - 1).unwrap_or(8))).unwrap_or(0)).collect();
    Ok(Seg3 { lv, evs, ex, lists, facts: Vec::new(), tfmask, rd, tfw, liq, days: seg_days.to_vec(), cnt: Counts::default(), t_read, tops: (0..NGROUP).map(|_| Tops::new(200)).collect(), piece_of_day, mx, mlabels, mfallback, mlat, ngroup: NGROUP, vol60, universe, lvd: day_lv.unwrap_or_default(), ..Seg3::default() })
}

/// v3.3 C3 (DERIVED): [`build_lists`] without storing the unique lists' event ids: the same FNV hash and length of each pair's filtered
/// list (computed in one pass, nothing materialized), the same candidates map, a FULL compare against each candidate's DERIVED sequence
/// (its representative pair's config list filtered by that pair's liquidity value), the same first-seen numbering, the same `uls` /
/// `ul_cfgs` / `removed`. `sr.lists` is kept (the lists are derived from it for the whole run).
pub(crate) fn build_lists_derived(sr: &mut Seg3, liqs: &[usize], tf_dep: bool, log: &mut String, sn: &str) {
    let mut seen: HashMap<(u64, usize, u8), Vec<u32>> = HashMap::new();
    let mut removed = [(0_u64, 0_u64, 0_u64); NLIQ];
    let nev = sr.evs.len();
    // every liquidity value's pass flags, once (a candidate's own value may differ from the pair's being built)
    let passes: Vec<Vec<bool>> = (0..NLIQ).map(|v| if liqs.contains(&v) { sr.liq.iter().map(|&(c, g)| liq_pass(v, c, g)).collect() } else { Vec::new() }).collect();
    let used: Vec<bool> = {
        let mut u = vec![false; nev];
        for l in &sr.lists {
            for &e in l {
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
        if let Some(r) = removed.get_mut(v) {
            r.1 = u64::try_from(used.iter().zip(pass).filter(|(u, p)| **u && !**p).count()).unwrap_or(0);
            r.2 = u64::try_from(used.iter().zip(pass).filter(|(u, p)| **u && **p).count()).unwrap_or(0);
        }
        for (id, l) in sr.lists.iter().enumerate() {
            if l.is_empty() {
                continue;
            }
            let mut h = 0xcbf2_9ce4_8422_2325_u64;
            let mut n = 0_usize;
            for &x in l {
                if pass.get(x as usize).copied().unwrap_or(false) {
                    h ^= u64::from(x);
                    h = h.wrapping_mul(0x0100_0000_01b3);
                    n += 1;
                }
            }
            if let Some(r) = removed.get_mut(v) {
                r.0 += u64::try_from(l.len() - n).unwrap_or(0);
            }
            if n == 0 {
                continue;
            }
            let tfk = if tf_dep { u8::try_from(cfg_parts(id).0).unwrap_or(0) } else { u8::MAX };
            let cands = seen.entry((h, n, tfk)).or_default();
            let mine = l.iter().copied().filter(|&e| pass.get(e as usize).copied().unwrap_or(false));
            let found = cands.iter().copied().find(|&u| {
                let Some(&(rid, rv, _)) = sr.uls.get(u as usize) else { return false };
                let (Some(rl), Some(rp)) = (sr.lists.get(rid as usize), passes.get(usize::from(rv))) else { return false };
                rl.iter().copied().filter(|&e| rp.get(e as usize).copied().unwrap_or(false)).eq(mine.clone())
            });
            let u = if let Some(u) = found {
                u
            } else {
                let u = u32::try_from(sr.uls.len()).unwrap_or(0);
                sr.uls.push((u32::try_from(id).unwrap_or(0), u8::try_from(v).unwrap_or(0), tfk));
                sr.ul_cfgs.push(Vec::new());
                cands.push(u);
                u
            };
            if let Some(c) = sr.ul_cfgs.get_mut(u as usize) {
                c.push((u32::try_from(id).unwrap_or(0), u8::try_from(v).unwrap_or(0)));
            }
        }
    }
    sr.removed = removed;
    sr.ul_derived = true;
    lg!(log, "{sn}: {} unique lists over {} (config, liquidity) pairs; removed per liquidity value (entries / unique events): {}", sr.uls.len(), sr.ul_cfgs.iter().map(Vec::len).sum::<usize>(), liqs.iter().map(|&v| format!("{} {} / {}", LIQ_SHORT.get(v).copied().unwrap_or("?"), removed.get(v).map_or(0, |x| x.0), removed.get(v).map_or(0, |x| x.1))).collect::<Vec<_>>().join("; "));
    lg!(log, "{sn}: v3.3 C3: unique lists DERIVED from {} per-config list entries (no ul_evs stored)", sr.lists.iter().map(Vec::len).sum::<usize>());
}

/// The lists of every (config, liquidity value), deduplicated into unique lists (by the event sequence; by (timeframe, sequence) when a
/// selected setting's SL depends on the window); the candidates each liquidity value removes.
pub(crate) fn build_lists(sr: &mut Seg3, liqs: &[usize], tf_dep: bool, log: &mut String, sn: &str) {
    let mut seen: HashMap<(u64, usize, u8), Vec<u32>> = HashMap::new();
    let mut removed = [(0_u64, 0_u64, 0_u64); NLIQ];
    for &v in liqs {
        let pass: Vec<bool> = sr.liq.iter().map(|&(c, g)| liq_pass(v, c, g)).collect();
        let used: Vec<bool> = {
            let mut u = vec![false; sr.evs.len()];
            for l in &sr.lists {
                for &e in l {
                    if let Some(x) = u.get_mut(e as usize) {
                        *x = true;
                    }
                }
            }
            u
        };
        if let Some(r) = removed.get_mut(v) {
            r.1 = u64::try_from(used.iter().zip(&pass).filter(|(u, p)| **u && !**p).count()).unwrap_or(0);
            r.2 = u64::try_from(used.iter().zip(&pass).filter(|(u, p)| **u && **p).count()).unwrap_or(0);
        }
        for (id, l) in sr.lists.iter().enumerate() {
            if l.is_empty() {
                continue;
            }
            let fl: Vec<u32> = l.iter().copied().filter(|&e| pass.get(e as usize).copied().unwrap_or(false)).collect();
            if let Some(r) = removed.get_mut(v) {
                r.0 += u64::try_from(l.len() - fl.len()).unwrap_or(0);
            }
            if fl.is_empty() {
                continue;
            }
            let tfk = if tf_dep { u8::try_from(cfg_parts(id).0).unwrap_or(0) } else { u8::MAX };
            let mut h = 0xcbf2_9ce4_8422_2325_u64;
            for &x in &fl {
                h ^= u64::from(x);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
            let cands = seen.entry((h, fl.len(), tfk)).or_default();
            let found = cands.iter().copied().find(|&u| sr.ul_evs.get(u as usize).is_some_and(|r| *r == fl));
            let u = if let Some(u) = found {
                u
            } else {
                let u = u32::try_from(sr.uls.len()).unwrap_or(0);
                sr.uls.push((u32::try_from(id).unwrap_or(0), u8::try_from(v).unwrap_or(0), tfk));
                sr.ul_evs.push(fl);
                sr.ul_cfgs.push(Vec::new());
                cands.push(u);
                u
            };
            if let Some(c) = sr.ul_cfgs.get_mut(u as usize) {
                c.push((u32::try_from(id).unwrap_or(0), u8::try_from(v).unwrap_or(0)));
            }
        }
    }
    sr.removed = removed;
    lg!(log, "{sn}: {} unique lists over {} (config, liquidity) pairs; removed per liquidity value (entries / unique events): {}", sr.uls.len(), sr.ul_cfgs.iter().map(Vec::len).sum::<usize>(), liqs.iter().map(|&v| format!("{} {} / {}", LIQ_SHORT.get(v).copied().unwrap_or("?"), removed.get(v).map_or(0, |x| x.0), removed.get(v).map_or(0, |x| x.1))).collect::<Vec<_>>().join("; "));
}

/// Stage A over a set of events (`only`: None = all), in parallel chunks (day, up to 64 contracts), merged into `sr.tab` / `sr.ex` / the
/// events' tables. D1 (check-build-v3): `count_once` merges the stage A counters of an event only the first time it is priced (the
/// day-major mode prices an event once per batch holding it); the events priced are then marked counted.
#[allow(clippy::too_many_arguments)]
fn stage_a_events(sr: &mut Seg3, only: Option<&[usize]>, cubes: &[Cube], tapes: &[SecTape], var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, ma: &MenuA, seg: u8, compat: bool, threads: usize) -> Result<(), String> {
    stage_a_events_c(sr, only, cubes, tapes, var, dial, ma, seg, compat, threads, false)
}

/// The stage A work items of a set of events (round B's cut, moved here unchanged apart from `maxc`): per day in ascending order, the day's
/// events sorted by (contract, time) (stable) and cut at contract boundaries into items of at most `maxc` contracts (round B: 64; v3.3
/// ACHUNK: [`crate::sw33::ACHUNK_N`]). Every cut keeps a contract-day's events in ONE item and the items, read in order, give the same event
/// sequence for any `maxc`; each item's per-contract state (tape, trail index, marks) starts at its contract and its tables are appended in
/// item order with the running offsets, so the tables, marks, counters and records are the same for any `maxc` (exact): only the number of
/// items the threads share changes.
#[must_use]
pub fn stage_a_chunks33(evs: &[SEv], ids: Vec<usize>, maxc: usize) -> Vec<(usize, Vec<usize>)> {
    let mut by_day: BTreeMap<u16, Vec<usize>> = BTreeMap::new();
    for i in ids {
        if let Some(e) = evs.get(i) {
            by_day.entry(e.di).or_default().push(i);
        }
    }
    let mut chunks: Vec<(usize, Vec<usize>)> = Vec::new();
    for (di, mut v) in by_day {
        v.sort_by_key(|&i| evs.get(i).map_or((0, 0), |e| (e.ci, e.t)));
        let mut cur: Vec<usize> = Vec::new();
        let (mut ncon, mut last) = (0_usize, u32::MAX);
        for i in v {
            let ci = evs.get(i).map_or(0, |e| e.ci);
            if ci != last {
                ncon += 1;
                last = ci;
                if ncon > maxc {
                    chunks.push((usize::from(di), std::mem::take(&mut cur)));
                    ncon = 1;
                }
            } else if cfg!(feature = "m_v33_achunk_splits_contract") && cur.len() >= maxc {
                // MUTANT m_v33_achunk_splits_contract: an item also cut INSIDE a contract after `maxc` events
                chunks.push((usize::from(di), std::mem::take(&mut cur)));
            }
            cur.push(i);
        }
        if !cur.is_empty() {
            chunks.push((usize::from(di), cur));
        }
    }
    chunks
}

#[allow(clippy::too_many_arguments)]
fn stage_a_events_c(sr: &mut Seg3, only: Option<&[usize]>, cubes: &[Cube], tapes: &[SecTape], var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, ma: &MenuA, seg: u8, compat: bool, threads: usize, count_once: bool) -> Result<(), String> {
    if count_once && sr.counted.len() != sr.evs.len() {
        sr.counted = vec![false; sr.evs.len()];
    }
    let mtm = sw::on(sw::MTM);
    let blk = block_of(seg);
    let ids: Vec<usize> = only.map_or_else(|| (0..sr.evs.len()).collect(), <[usize]>::to_vec);
    // v3.3 ACHUNK: work items of ACHUNK_N contract(s) instead of round B's 64, so a batch-day's pricing spreads over the threads (an index
    // day has about 65 contracts with events: round B's items gave it 1 or 2 threads)
    let maxc = if crate::sw33::on(crate::sw33::ACHUNK) { crate::sw33::ACHUNK_N } else { 64 };
    let chunks: Vec<(usize, Vec<usize>)> = stage_a_chunks33(&sr.evs, ids, maxc);
    type Done = (usize, Result<(Vec<(usize, trade::EvT, EvX)>, Tab3, Counts), String>);
    let next = AtomicUsize::new(0);
    let all: Mutex<Vec<Done>> = Mutex::new(Vec::new());
    let (evs, rd, tfw, mxs, counted, vol60s) = (&sr.evs, &sr.rd, &sr.tfw, &sr.mx, &sr.counted, &sr.vol60);
    let liq14 = sw::on(sw::LIQ14);
    let nrules = ma.rows.len();
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some((di, idxs)) = chunks.get(k) else { break };
                    let r = (|| {
                        let (Some(c), Some(tp)) = (cubes.get(*di), tapes.get(*di)) else { return Err("day".to_owned()) };
                        let mut tab = Tab3::default();
                        let mut cnt = Counts::new(nrules);
                        let mut scratch = Counts::new(nrules);
                        let mut out = Vec::with_capacity(idxs.len());
                        let mut mk: Option<(u32, u32)> = None;
                        let mut cache: Option<(u32, Option<DayTape>)> = None;
                        let mut tic: Option<(u32, Option<TrailIndex>)> = None;
                        for &i in idxs {
                            let Some(e) = evs.get(i) else { continue };
                            if cache.as_ref().is_none_or(|x| x.0 != e.ci) {
                                cache = Some((e.ci, DayTape::new(c.day, tp.of(e.ci as usize), blk).ok()));
                                tic = None;
                            }
                            let tape = cache.as_ref().and_then(|x| x.1.as_ref()).and_then(|d| d.tape().ok());
                            if tic.as_ref().is_none_or(|x| x.0 != e.ci) {
                                tic = Some((e.ci, tape.as_ref().map(|t| TrailIndex::new(t, false))));
                            }
                            let ti = tic.as_ref().and_then(|x| x.1.as_ref());
                            let base = rd.get(i).copied().unwrap_or_default();
                            let w = tfw.get(i).cloned().unwrap_or_default();
                            // v3.1 item 4: the contract-day's minute closes, once per contract in this chunk (the MTM check's marks)
                            let mark1 = if mtm {
                                if mk.is_none_or(|x| x.0 != e.ci) {
                                    let m0 = u32::try_from(tab.marks.len()).map_err(|_| "marks")?;
                                    // matrix X22: the mark of a minute is its HIGH when the bar is present (the worst price for a short), else the
                                    // forward-filled close; only bars ended by T are read (minute b - 1 at decision T)
                                    // fix:v33 (standing F6, matrix X22): the same rule, named and tested in one place (test xm_mtm_mark_is_minute_high)
                                    tab.marks.extend(crate::fixg::minute_marks(c, e.ci as usize));
                                    mk = Some((e.ci, m0 + 1));
                                }
                                mk.map_or(0, |x| x.1)
                            } else {
                                0
                            };
                            let cn = if count_once && counted.get(i).copied().unwrap_or(false) { &mut scratch } else { &mut cnt };
                            let (mut et, x) = price_event3(c, e.ci as usize, e.t, tape.as_ref(), ti, var, dial, ma, &base, &w, compat, true, &mut tab, cn, mxs.get(i), mark1)?;
                            // v3.2 addendum 14 (b)+(c): the coverage bits per slot count (refuse, never half-enter)
                            if liq14 {
                                let v = vol60s.get(i).copied().unwrap_or(0);
                                et.vol60 = v;
                                et.cover = trade::cover_bits(&et.lots, u64::from(c.cons.get(e.ci as usize).map_or(1, |k| k.lot)), v);
                            }
                            out.push((i, et, x));
                        }
                        Ok((out, tab, cnt))
                    })();
                    if let Ok(mut g) = all.lock() {
                        g.push((k, r));
                    }
                }
            });
        }
    });
    let mut done = all.into_inner().unwrap_or_default();
    done.sort_by_key(|x| x.0);
    if sr.cnt.absent.is_empty() {
        sr.cnt = Counts::new(nrules);
    }
    for (_, r) in done {
        let (v, tab, cnt) = r?;
        let (dr, dt) = sr.tab.append(tab)?;
        let dmk = sr.tab.marks_shift;
        sr.cnt.merge(&cnt);
        for (i, et, mut x) in v {
            if x.nq > 0 {
                x.r0 += dr;
                x.t0 += dt;
            }
            if x.mark1 > 0 {
                x.mark1 += dmk;
            }
            // v3.3 C4a: the event's fill list into the online Table L sums at its FIRST counted pricing, then dropped (every pricing)
            if sr.tl_online {
                if count_once && !sr.counted.get(i).copied().unwrap_or(true) {
                    let (c, g) = sr.liq.get(i).copied().unwrap_or((0, 0));
                    tl_add(&mut sr.tl_sum, &x.fill, c, g);
                }
                x.fill = Vec::new();
            }
            if count_once && let Some(cflag) = sr.counted.get_mut(i) {
                *cflag = true;
            }
            if let (Some(e), Some(xx)) = (sr.evs.get_mut(i), sr.ex.get_mut(i)) {
                e.tab = et;
                *xx = x;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn stage_a_whole(sr: &mut Seg3, cubes: &[Cube], tapes: &[SecTape], var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, ma: &MenuA, seg: u8, compat: bool, threads: usize) -> Result<(), String> {
    sr.tab = Tab3::default();
    sr.cnt = Counts::new(ma.rows.len());
    stage_a_events(sr, None, cubes, tapes, var, dial, ma, seg, compat, threads)
}

/// Whether a unique list holds a gain-threshold (spike) config.
fn spike_of(sr: &Seg3, u: usize) -> bool {
    sr.ul_cfgs.get(u).is_some_and(|v| v.iter().any(|&(id, _)| cfg_parts(id as usize).5 < 9))
}

/// The groups an account of setting slot `si` is offered to.
fn groups_of(sets: &Sets, sel: &[usize], si: usize) -> Vec<usize> {
    let mut g = vec![0];
    if let Some(s) = sel.get(si).and_then(|&i| sets.settings.get(i)) {
        g.push(usize::from(s.set));
        if s.in_s0 {
            g.push(5);
        }
        g.push(if s.search { 6 } else { 7 });
    }
    g
}

/// Stage B over every unique list in memory (the whole mode): work items (list, a chunk of settings), every account walked once.
#[allow(clippy::too_many_arguments)]
fn stage_b_whole(sr: &mut Seg3, sets: &Sets, sel: &[usize], csets: &[CSet], seg: u8, skip: bool, threads: usize, topk: usize, log: &mut String) -> Result<(), String> {
    let tb = Instant::now();
    // REPAIR A: SPEED-UP 4 (exact setting dedup across every setting of a list) walks one work item per list
    let dedup = sw::on(sw::DEDUP);
    let chunk = if dedup { csets.len().max(1) } else { 64_usize };
    let mut items: Vec<(usize, usize)> = Vec::new();
    let mut order: Vec<usize> = (0..sr.uls.len()).collect();
    order.sort_by_key(|&u| std::cmp::Reverse(sr.ul_evs.get(u).map_or(0, Vec::len)));
    for u in order {
        for c0 in (0..csets.len()).step_by(chunk) {
            items.push((u, c0));
        }
    }
    let stept = sw::on(sw::STEPT);
    let blocks_all: Mutex<Vec<(u64, u64)>> = Mutex::new(Vec::new());
    let nb_all: Mutex<([(u64, u64); 4], u64)> = Mutex::new(([(0, 0); 4], 0));
    let ndays = u16::try_from(sr.days.len()).unwrap_or(0);
    let gpre: Vec<Vec<usize>> = (0..csets.len()).map(|si| groups_of(sets, sel, si)).collect();
    // the year piece of every day index of the run, and the window rows whose learning pieces hold a session of this segment
    let of_day: Vec<u8> = sr.piece_of_day.clone();
    let rows = crate::wf::rows21();
    let present: Vec<bool> = (0..crate::wf::NPIECE).map(|p| sr.days.iter().any(|&d| of_day.get(d).copied() == Some(u8::try_from(p).unwrap_or(0)))).collect();
    let run_rows: Vec<bool> = rows.iter().map(|r| (r.learn.0..=r.learn.1).any(|p| present.get(p).copied().unwrap_or(false))).collect();
    let any_row = run_rows.iter().any(|&x| x);
    let search: Vec<bool> = (0..csets.len()).map(|si| sel.get(si).and_then(|&i| sets.settings.get(i)).is_some_and(|s| s.search)).collect();
    let win_all: Mutex<Vec<WinRes>> = Mutex::new(rows.iter().zip(&run_rows).map(|(r, &run)| WinRes { row: Some(r.clone()), run, ..WinRes::default() }).collect());
    // v3.2-B (addendum 18): every thread's split accumulators, merged (deterministic: key, then the tie key)
    let sp18_all: Mutex<crate::split18::Sel18> = Mutex::new(crate::split18::Sel18::default());
    let next = AtomicUsize::new(0);
    let ng = sr.ngroup.max(NGROUP);
    let headline = sw::on(sw::HEADLINE);
    let merged: Mutex<(Vec<Tops>, u64, u64)> = Mutex::new(((0..ng).map(|_| Tops::new(topk)).collect(), 0, 0));
    let step_all: Mutex<Vec<u64>> = Mutex::new(Vec::new());
    let search15 = sw::on(sw::SEARCH15);
    let srr = &*sr;
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut tops: Vec<Tops> = (0..ng).map(|_| Tops::new(topk)).collect();
                let (mut st, mut na) = (0_u64, 0_u64);
                let mut sns: Vec<u64> = Vec::new();
                let mut blocks: Vec<(u64, u64)> = Vec::new();
                let mut nb = [(0_u64, 0_u64); 4];
                let mut walked_dd = 0_u64;
                let mut kcache: Option<(usize, Vec<u32>)> = None;
                let mut win: Vec<WinRes> = rows.iter().zip(&run_rows).map(|(r, &run)| WinRes { row: Some(r.clone()), run, ..WinRes::default() }).collect();
                let mut sel18 = crate::split18::Sel18::new();
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(u, c0)) = items.get(k) else { break };
                    let Some(list) = srr.ul_evs.get(u) else { continue };
                    if kcache.as_ref().is_none_or(|x| x.0 != u) {
                        kcache = Some((u, list_keys(&srr.evs, list)));
                    }
                    let keys = kcache.as_ref().map_or(&[][..], |x| x.1.as_slice());
                    let tf = srr.uls.get(u).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) });
                    let spike = spike_of(srr, u);
                    let c1 = (c0 + chunk).min(csets.len());
                    let t_item = Instant::now();
                    let (od, np) = (if any_row { &of_day[..] } else { &[][..] }, if any_row { crate::wf::NPIECE } else { 0 });
                    let smp = if stept { Some(&mut blocks) } else { None };
                    let (accs, steps) = if dedup {
                        let (a, s, w) = crate::kern::walk_dedup(&srr.evs, &srr.ex, &srr.tab, list, keys, csets.get(c0..c1).unwrap_or(&[]), tf, true, skip, od, np, ndays, smp);
                        walked_dd += w;
                        (a, s)
                    } else if stept {
                        crate::kern::walk_chunk_s(&srr.evs, &srr.ex, &srr.tab, list, keys, csets.get(c0..c1).unwrap_or(&[]), tf, true, skip, od, np, ndays, smp)
                    } else {
                        crate::kern::walk_chunk(&srr.evs, &srr.ex, &srr.tab, list, keys, csets.get(c0..c1).unwrap_or(&[]), tf, true, skip, od, np, ndays)
                    };
                    st += steps;
                    if search15 && steps > 0 {
                        sns.push(u64::try_from(t_item.elapsed().as_nanos() * 1_000 / u128::from(steps)).unwrap_or(u64::MAX));
                    }
                    for (a, (sm0, pc)) in accs.iter().enumerate() {
                        let si = c0 + a / (NL * 4);
                        let l = (a % (NL * 4)) / 4;
                        if !trade::share_on(l) {
                            continue;
                        }
                        let mut sm = *sm0;
                        na += 1;
                        if let Some(x) = nb.get_mut(a % 4) {
                            x.0 += 1;
                            x.1 += u64::from(sm.trades > 0);
                        }
                        sm.seg = seg;
                        sm.list = u32::try_from(u).unwrap_or(0);
                        sm.x = u16::try_from(si * NL + l).unwrap_or(u16::MAX);
                        sm.spike = spike;
                        sm.tie = srr.ties.get(si).copied().unwrap_or(0);
                        // v3.2-B (addendum 18): the split statistics (kept in the summary) and the folds' picks (SEARCH settings), from the one walk
                        let tie18 = wtie(&sm);
                        crate::split18::account(pc.ms.as_deref(), &mut sm, &mut sel18, search.get(si).copied().unwrap_or(false), tie18);
                        for &gi in gpre.get(si).map_or(&[][..], Vec::as_slice) {
                            if let Some(t) = tops.get_mut(gi) {
                                t.offer(sm);
                            }
                        }
                        if headline && headline_ok(search.get(si).copied().unwrap_or(false), l, srr.uls.get(u).map_or(0, |x| x.1), &sm)
                            && let Some(t) = tops.get_mut(NGROUP)
                        {
                            t.offer(sm);
                        }
                        // the 21 windows (SEARCH settings only pick; a row with no learning session is NOT RUN)
                        if any_row && search.get(si).copied().unwrap_or(false) && sm.trades > 0
                            && let Some(cs) = csets.get(si)
                        {
                            let ni = a % 4;
                            // the exact re-walk from a row's start (a fresh account at its first learning piece)
                            let rw = |start: usize| -> Vec<crate::kern::Piece> {
                                let sub: Vec<u32> = list.iter().copied().filter(|&e| srr.evs.get(e as usize).is_some_and(|ev| usize::from(of_day.get(usize::from(ev.di)).copied().unwrap_or(0)) >= start)).collect();
                                let sk = list_keys(&srr.evs, &sub);
                                walk_account(&srr.evs, &srr.ex, &srr.tab, &sub, &sk, cs, l, ni, tf, true, true, &of_day, crate::wf::NPIECE, ndays).1.p
                            };
                            offer_windows(&mut win, &rows, &pc.p, sm, Some(&rw));
                        }
                    }
                }
                if let Ok(mut m) = merged.lock() {
                    for (a, b) in m.0.iter_mut().zip(tops) {
                        a.merge(b);
                    }
                    m.1 += st;
                    m.2 += na;
                }
                if let Ok(mut g) = step_all.lock() {
                    g.extend(sns);
                }
                if let Ok(mut g) = blocks_all.lock() {
                    g.extend(blocks);
                }
                if let Ok(mut g) = nb_all.lock() {
                    for (x, y) in g.0.iter_mut().zip(nb) {
                        x.0 += y.0;
                        x.1 += y.1;
                    }
                    g.1 += walked_dd;
                }
                if let Ok(mut g) = win_all.lock() {
                    for (a, b) in g.iter_mut().zip(win) {
                        a.checked += b.checked;
                        a.rewalked += b.rewalked;
                        a.pending += b.pending;
                        if let Some(bb) = b.best
                            && a.best.as_ref().is_none_or(|x| bb.0 > x.0 || (bb.0 == x.0 && wtie(&bb.1) < wtie(&x.1)))
                        {
                            a.best = Some(bb);
                        }
                    }
                }
                if let Ok(mut g) = sp18_all.lock() {
                    g.merge(sel18);
                }
            });
        }
    });
    let (tops, st, na) = merged.into_inner().unwrap_or_default();
    sr.step_ns = step_all.into_inner().unwrap_or_default();
    sr.step_blocks = blocks_all.into_inner().unwrap_or_default();
    let (nbk, wdd) = nb_all.into_inner().unwrap_or_default();
    sr.n_booked = nbk;
    sr.dedup = (na, wdd);
    sr.win = win_all.into_inner().unwrap_or_default();
    sr.sp18 = sp18_all.into_inner().unwrap_or_default();
    sr.rewalks = sr.win.iter().map(|w| w.rewalked).sum();
    sr.tops = tops;
    sr.steps = st;
    sr.accounts = na;
    sr.t_stage_b = tb.elapsed().as_secs_f64();
    if sw::any32() && (sw::on(sw::STEPT) || dedup) {
        // REPAIR A (check-engine-2 S8): the walked share levels counted (not NL); SPEED-UP 4's walked accounts named
        let nsh = trade::SHARE_MASK.load(Ordering::Relaxed).count_ones();
        lg!(log, "{}: STAGE B {} unique lists x {} settings x {nsh} share levels walked x 4 slot counts = {} accounts{}, {} account-event steps (after exact skipping: {skip}) in {:.1} s ({:.1} ns per step, wall)", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), sr.uls.len(), csets.len(), na, if dedup { format!(" covered, {wdd} walked by the exact setting dedup (SPEED-UP 4)") } else { " walked".to_owned() }, st, sr.t_stage_b, if st > 0 { sr.t_stage_b * 1e9 / st as f64 } else { 0.0 });
        return Ok(());
    }
    lg!(log, "{}: STAGE B {} unique lists x {} settings x {NL} share levels x 4 slot counts = {} accounts walked, {} account-event steps (after exact skipping: {skip}) in {:.1} s ({:.1} ns per step)", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), sr.uls.len(), csets.len(), na, st, sr.t_stage_b, if st > 0 { sr.t_stage_b * 1e9 / st as f64 } else { 0.0 });
    Ok(())
}

/// Speed-up 2 proof on the run's own data: every account of every list walked with and without skipping: every summary equal.
fn compare_skip(sr: &Seg3, csets: &[CSet], threads: usize, log: &mut String) {
    let t0 = Instant::now();
    let ndays = u16::try_from(sr.days.len()).unwrap_or(0);
    let next = AtomicUsize::new(0);
    let tot: Mutex<(u64, u64, u64, u64)> = Mutex::new((0, 0, 0, 0));
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let (mut n, mut bad, mut s1, mut s2) = (0_u64, 0_u64, 0_u64, 0_u64);
                loop {
                    let u = next.fetch_add(1, Ordering::Relaxed);
                    let Some(list) = sr.ul_evs.get(u) else { break };
                    let keys = list_keys(&sr.evs, list);
                    let tf = sr.uls.get(u).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) });
                    let (rows, sa) = crate::kern::walk_chunk(&sr.evs, &sr.ex, &sr.tab, list, &keys, csets, tf, true, true, &[], 0, ndays);
                    s1 += sa;
                    for (k, (a, _)) in rows.iter().enumerate() {
                        let (si, l, ni) = (k / (NL * 4), (k % (NL * 4)) / 4, k % 4);
                        let Some(cs) = csets.get(si) else { continue };
                        let (b, _, sb) = walk_account(&sr.evs, &sr.ex, &sr.tab, list, &keys, cs, l, ni, tf, true, false, &[], 0, ndays);
                        n += 1;
                        s2 += sb;
                        if (a.net, a.trades, a.wins, a.mdd, a.worst, a.worst_day, a.days_pos, a.days_neg, a.days_traded, a.pk_margin) != (b.net, b.trades, b.wins, b.mdd, b.worst, b.worst_day, b.days_pos, b.days_neg, b.days_traded, b.pk_margin) {
                            bad += 1;
                        }
                    }
                }
                if let Ok(mut t) = tot.lock() {
                    t.0 += n;
                    t.1 += bad;
                    t.2 += s1;
                    t.3 += s2;
                }
            });
        }
    });
    let (n, bad, s1, s2) = tot.into_inner().unwrap_or_default();
    lg!(log, "SKIP-EQ (speed-up 2 + the event-major chunk kernel): {n} accounts: chunk kernel WITH skipping vs walk_account WITHOUT skipping: {bad} summaries differ; steps {s1} with skipping vs {s2} without (x{:.2}); {:.1} s", if s1 > 0 { s2 as f64 / s1 as f64 } else { 0.0 }, t0.elapsed().as_secs_f64());
}

/// The DAY-MAJOR mode (Q10): batches of unique lists sized by the memory gate; per batch, per day: stage A of that day's events of the
/// batch's lists, then every account of the batch advanced through the day (its state carried; positions never cross a day), tables
/// dropped after the day. Stage A is recomputed per batch (R4 (d)). v3.1: D2 the day is loaded for its pass-day and dropped right after its
/// stage A (the walk reads only the tables); D1 each event's stage A counters are merged ONCE (the first batch also prices the events of no
/// unique list, so the counters equal the whole mode's); D3 an account-row the slack check flags is kept (\`sr.flagged\`) for the exact
/// re-walk pass ([\`d3_rewalk\`]), never dropped.
#[allow(clippy::too_many_arguments)]
fn stage_b_day_major(sr: &mut Seg3, store: &mut Store, var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, ma: &MenuA, sets: &Sets, sel: &[usize], csets: &[CSet], seg: u8, compat: bool, skip: bool, threads: usize, topk: usize, batch_acc: u64, log: &mut String) -> Result<(), String> {
    let tb = Instant::now();
    let per_list = u64::try_from(csets.len() * NL * 4).unwrap_or(u64::MAX);
    let lists_per_batch = usize::try_from((batch_acc / per_list.max(1)).max(1)).unwrap_or(1);
    // v3.2-B (addendum 18): every account's split state counted in the batch gate (a batch holds the same bytes as without it)
    let lists_per_batch = if sw::on(sw::SPLITS18) {
        let base = std::mem::size_of::<(crate::kern::Acc, Pieces)>() + crate::wf::NPIECE * std::mem::size_of::<crate::kern::Piece>();
        // round B: one segment series per account (one allocation; the month series is whole mode only)
        let extra = crate::split18::cal().map_or(0, |_| std::mem::size_of::<crate::split18::Ser18>() + 16);
        (lists_per_batch * base / (base + extra).max(1)).max(1)
    } else {
        lists_per_batch
    };
    let nl = sr.uls.len();
    let ndays = u16::try_from(sr.days.len()).unwrap_or(0);
    let ng = sr.ngroup.max(NGROUP);
    let headline = sw::on(sw::HEADLINE);
    let mut tops: Vec<Tops> = (0..ng).map(|_| Tops::new(topk)).collect();
    let (mut steps, mut accts, mut ta) = (0_u64, 0_u64, 0.0_f64);
    let of_day: Vec<u8> = sr.piece_of_day.clone();
    let rows = crate::wf::rows21();
    let present: Vec<bool> = (0..crate::wf::NPIECE).map(|p| sr.days.iter().any(|&d| of_day.get(d).copied() == Some(u8::try_from(p).unwrap_or(0)))).collect();
    let run_rows: Vec<bool> = rows.iter().map(|r| (r.learn.0..=r.learn.1).any(|p| present.get(p).copied().unwrap_or(false))).collect();
    let any_row = run_rows.iter().any(|&x| x);
    let mut win: Vec<WinRes> = rows.iter().zip(&run_rows).map(|(r, &run)| WinRes { row: Some(r.clone()), run, ..WinRes::default() }).collect();
    // D1: counters once per event; the events of no unique list are priced (and counted) in the first batch
    sr.counted = vec![false; sr.evs.len()];
    sr.cnt = Counts::new(ma.rows.len());
    let mut in_any = vec![false; sr.evs.len()];
    for l in &sr.ul_evs {
        for &e in l {
            if let Some(x) = in_any.get_mut(e as usize) {
                *x = true;
            }
        }
    }
    let mut orphans: BTreeMap<u16, Vec<usize>> = BTreeMap::new();
    for (i, e) in sr.evs.iter().enumerate() {
        if !in_any.get(i).copied().unwrap_or(true) {
            orphans.entry(e.di).or_default().push(i);
        }
    }
    sr.in_any = in_any;
    let mut flagged: Vec<Flag> = Vec::new();
    let mut nbk = [(0_u64, 0_u64); 4];
    let mut batches = 0;
    let mut sel18 = crate::split18::Sel18::new();
    let mut b0 = 0;
    let mut peak_bytes = 0_usize;
    // REPAIR A (addendum 20.3): checkpoint after every finished batch; a resumed run skips the finished batches' walks
    let ck_path = crate::ckpt::CKPT_DIR.get().map(|d| d.join(format!("{}.ckpt", SEG_SHORT.get(usize::from(seg)).copied().unwrap_or("x"))));
    let fp = format!("seg {seg}; days {} ({}..{}); settings {}; unique lists {nl}; lists per batch {lists_per_batch}; share mask {}; groups {ng}; skip {skip}; switches {}{}{}{}", sr.days.len(), sr.days.first().copied().unwrap_or(0), sr.days.last().copied().unwrap_or(0), csets.len(), trade::SHARE_MASK.load(Ordering::Relaxed), sw::words(), sw::words32(), crate::fix33::fp_part(), crate::fixg::fp_part());
    if let Some(p) = &ck_path
        && let Some(c) = crate::ckpt::load(p, &fp, ng, rows.len())?
    {
        b0 = c.next_b0;
        batches = c.batches;
        steps = c.steps;
        accts = c.accts;
        nbk = c.nbk;
        tops = c.tops;
        flagged = c.flagged;
        // round B (addendum 18): the finished batches' split picks
        sel18.ckpt_restore(&c.sp18)?;
        for (w, (ck, rw, pe, best)) in win.iter_mut().zip(c.win) {
            w.checked = ck;
            w.rewalked = rw;
            w.pending = pe;
            w.best = best;
        }
        // the finished batches' events (and the orphans of batch 1) priced ONCE, in one union stage A pass: their per-event facts and the
        // stage A counters, exactly as the uninterrupted run counted them (once per event); no account walks
        let t0 = Instant::now();
        for &di in &sr.days.clone() {
            let lo = u32::try_from(di).unwrap_or(0) * 86_400;
            let hi = lo + 86_400;
            let mut want: Vec<usize> = Vec::new();
            for u in 0..b0.min(nl) {
                let Some(list) = sr.ul_evs.get(u) else { continue };
                let ks = list_keys(&sr.evs, list);
                let (a, b) = (ks.partition_point(|&x| x < lo), ks.partition_point(|&x| x < hi));
                want.extend(list.get(a..b).unwrap_or(&[]).iter().map(|&e| e as usize));
            }
            if b0 > 0
                && let Some(v) = orphans.get(&u16::try_from(di).unwrap_or(0))
            {
                want.extend(v.iter().copied());
            }
            want.sort_unstable();
            want.dedup();
            if want.is_empty() {
                continue;
            }
            store.ensure(&[di])?;
            sr.tab = Tab3::default();
            stage_a_events_c(sr, Some(&want), &store.cubes, &store.tapes, var, dial, ma, seg, compat, threads, true)?;
            store.release(&[]);
        }
        ta += t0.elapsed().as_secs_f64();
        lg!(log, "{}: RESUMED from checkpoint {} (REPAIR A): {batches} finished batch(es) = lists 0..{b0} not walked again; their events priced once in a union stage A pass ({:.1} s)", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), p.display(), t0.elapsed().as_secs_f64());
    }
    while b0 < nl {
        let b1 = (b0 + lists_per_batch).min(nl);
        batches += 1;
        // account states of the batch: (list, setting, l, ni) -> (Acc, Pieces)
        let n_acc = (b1 - b0) * csets.len() * NL * 4;
        let mut states: Vec<(crate::kern::Acc, Pieces)> = vec![(crate::kern::Acc::new(), if any_row { Pieces::new(&of_day, crate::wf::NPIECE) } else { Pieces::new(&[], 0) }); n_acc];
        let keys: Vec<Vec<u32>> = (b0..b1).map(|u| sr.ul_evs.get(u).map_or_else(Vec::new, |l| list_keys(&sr.evs, l))).collect();
        for &di in &sr.days.clone() {
            // the day's events of the batch's lists
            let lo = u32::try_from(di).unwrap_or(0) * 86_400;
            let hi = lo + 86_400;
            let mut want: Vec<usize> = Vec::new();
            for (k, u) in (b0..b1).enumerate() {
                let Some(list) = sr.ul_evs.get(u) else { continue };
                let ks = keys.get(k).map_or(&[][..], Vec::as_slice);
                let (a, b) = (ks.partition_point(|&x| x < lo), ks.partition_point(|&x| x < hi));
                want.extend(list.get(a..b).unwrap_or(&[]).iter().map(|&e| e as usize));
            }
            if batches == 1
                && let Some(v) = orphans.get(&u16::try_from(di).unwrap_or(0))
            {
                want.extend(v.iter().copied());
            }
            want.sort_unstable();
            want.dedup();
            if want.is_empty() {
                continue;
            }
            let t0 = Instant::now();
            store.ensure(&[di])?;
            sr.tab = Tab3::default();
            if cfg!(feature = "m_v31_dm_stale_tab") {
                // MUTANT m_v31_dm_stale_tab: the day's table appended to the earlier days' (row offsets of earlier days kept)
            }
            stage_a_events_c(sr, Some(&want), &store.cubes, &store.tapes, var, dial, ma, seg, compat, threads, true)?;
            // D2: the walk reads only the stage A tables: the day's cube and tape go now
            store.release(&[]);
            ta += t0.elapsed().as_secs_f64();
            peak_bytes = peak_bytes.max(sr.tab.bytes() + n_acc * std::mem::size_of::<(crate::kern::Acc, Pieces)>());
            // advance every account of the batch through the day, in parallel over lists
            let next = AtomicUsize::new(0);
            let st_cell: Vec<Mutex<&mut [(crate::kern::Acc, Pieces)]>> = states.chunks_mut(csets.len() * NL * 4).map(Mutex::new).collect();
            let tot = Mutex::new(0_u64);
            // v3.3 fix STEP_DM (check-engine-2 M3): every stride-th account-day walk of a worker thread is timed (inert with the switch off)
            let stride33 = sr.step33.stride;
            let smp_all: Mutex<Vec<(u64, u64)>> = Mutex::new(Vec::new());
            let srr = &*sr;
            let of_day = &of_day;
            std::thread::scope(|s| {
                for _ in 0..threads {
                    s.spawn(|| {
                        let mut my = 0_u64;
                        let mut smp = crate::fix33::StepSampler::new(stride33);
                        loop {
                            let k = next.fetch_add(1, Ordering::Relaxed);
                            let u = b0 + k;
                            if u >= b1 {
                                break;
                            }
                            let (Some(list), Some(ks), Some(cell)) = (srr.ul_evs.get(u), keys.get(k), st_cell.get(k)) else { continue };
                            let (a, b) = (ks.partition_point(|&x| x < lo), ks.partition_point(|&x| x < hi));
                            if a >= b {
                                continue;
                            }
                            let sub = list.get(a..b).unwrap_or(&[]);
                            let subk = ks.get(a..b).unwrap_or(&[]);
                            let tf = srr.uls.get(u).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) });
                            let Ok(mut g) = cell.lock() else { continue };
                            for (si, cs) in csets.iter().enumerate() {
                                for l in (0..NL).filter(|&l| trade::share_on(l)) {
                                    for ni in 0..4 {
                                        let idx = (si * NL + l) * 4 + ni;
                                        if let Some(stt) = g.get_mut(idx) {
                                            let t0 = smp.want().then(Instant::now);
                                            let st1 = crate::kern::walk_day(&srr.evs, &srr.ex, &srr.tab, sub, subk, cs, l, ni, tf, true, skip, &mut stt.0, &mut stt.1, of_day);
                                            if let Some(t0) = t0 {
                                                smp.push(u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX), st1);
                                            }
                                            my += st1;
                                        }
                                    }
                                }
                            }
                        }
                        if let Ok(mut t) = tot.lock() {
                            *t += my;
                        }
                        if !smp.blocks.is_empty()
                            && let Ok(mut v) = smp_all.lock()
                        {
                            v.append(&mut smp.blocks);
                        }
                    });
                }
            });
            steps += tot.into_inner().unwrap_or(0);
            let b33 = smp_all.into_inner().unwrap_or_default();
            if !b33.is_empty() {
                sr.step33.merge(b33);
                sr.step33.dm = true;
            }
        }
        // finalize the batch's accounts
        for (k, u) in (b0..b1).enumerate() {
            let spike = spike_of(sr, u);
            for si in 0..csets.len() {
                let g = groups_of(sets, sel, si);
                let search = sr.search.get(si).copied().unwrap_or(false);
                for l in (0..NL).filter(|&l| trade::share_on(l)) {
                    for ni in 0..4 {
                        let idx = k * csets.len() * NL * 4 + (si * NL + l) * 4 + ni;
                        let Some((ac, pc)) = states.get_mut(idx) else { continue };
                        let mut sm = crate::kern::finish_p(ac, pc, ni, ndays);
                        accts += 1;
                        if let Some(x) = nbk.get_mut(ni) {
                            x.0 += 1;
                            x.1 += u64::from(sm.trades > 0);
                        }
                        sm.seg = seg;
                        sm.list = u32::try_from(u).unwrap_or(0);
                        sm.x = u16::try_from(si * NL + l).unwrap_or(u16::MAX);
                        sm.spike = spike;
                        sm.tie = sr.ties.get(si).copied().unwrap_or(0);
                        // v3.2-B (addendum 18): the split statistics and the folds' picks, from the one walk
                        let tie18 = wtie(&sm);
                        crate::split18::account(pc.ms.as_deref(), &mut sm, &mut sel18, search, tie18);
                        if any_row && search && sm.trades > 0 {
                            for (ri, (w, r)) in win.iter_mut().zip(&rows).enumerate() {
                                if !w.run || crate::fixg::skip_sliding(r.growing) {
                                    continue;
                                }
                                if crate::wf::fresh_equal(&pc.p, r.learn.0, r.test.1) {
                                    w.checked += 1;
                                    offer_row(w, r, &pc.p, sm);
                                } else {
                                    // D3: the fresh start may decide differently: re-walked exactly by the second streamed pass
                                    w.pending += 1;
                                    flagged.push(Flag { row: u8::try_from(ri).unwrap_or(u8::MAX), list: sm.list, si: u32::try_from(si).unwrap_or(0), l: u8::try_from(l).unwrap_or(0), ni: u8::try_from(ni).unwrap_or(0), sm });
                                }
                            }
                        }
                        for &gi in &g {
                            if let Some(t) = tops.get_mut(gi) {
                                t.offer(sm);
                            }
                        }
                        if headline && headline_ok(search, l, sr.uls.get(u).map_or(0, |x| x.1), &sm)
                            && let Some(t) = tops.get_mut(NGROUP)
                        {
                            t.offer(sm);
                        }
                    }
                }
            }
        }
        b0 = b1;
        // REPAIR A (addendum 20.3): the finished batch is never lost
        if let Some(p) = &ck_path {
            let c = crate::ckpt::Ckpt { fingerprint: fp.clone(), next_b0: b0, batches, steps, accts, nbk, tops: tops.clone(), win: win.iter().map(|w| (w.checked, w.rewalked, w.pending, w.best)).collect(), flagged: flagged.clone(), sp18: sel18.ckpt_lines() };
            crate::ckpt::save(p, &c)?;
            lg!(log, "{}: CHECKPOINT {} after batch {batches} (lists 0..{b0} of {nl} finished)", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), p.display());
            let stop = crate::ckpt::STOP_AFTER.load(Ordering::Relaxed);
            if stop > 0 && batches >= stop && b0 < nl {
                return Err(format!("stopped after {batches} finished batch(es) by --stop-after-batches (checkpoint test hook): resume with the same arguments"));
            }
        }
    }
    sr.tops = tops;
    sr.win = win;
    sr.flagged = flagged;
    sr.n_booked = nbk;
    sr.sp18 = sel18;
    sr.steps = steps;
    sr.accounts = accts;
    sr.t_stage_a = ta;
    sr.t_stage_b = tb.elapsed().as_secs_f64() - ta;
    sr.stage_a_bytes = peak_bytes;
    lg!(log, "{}: DAY-MAJOR {batches} batch(es) of up to {lists_per_batch} lists ({} accounts per list; gate {batch_acc} accounts); stage A per day per batch {ta:.1} s; stage B {:.1} s; {accts} accounts, {steps} steps; peak tables + states {} MB; stage A counted once per event: {} of {} events; window account-rows flagged for the exact re-walk (D3): {}", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), per_list, sr.t_stage_b, peak_bytes / 1_000_000, sr.counted.iter().filter(|&&c| c).count(), sr.evs.len(), sr.flagged.len());
    Ok(())
}

/// v3.3 C6 / D2-1a (ORDER): the batch order of the unique lists: INDEX options config-sorted (config id = tf, minimum volume, top N,
/// minimum premium, minimum value, threshold; then the liquidity value; ties by the list id), STOCK options creation order (DESIGN-D1 2.3:
/// stock lists overlap by timeframe and expiry, the liquidity value is the strongest filter). Every list keeps its id: the order only
/// decides which lists share a batch (heaps, window picks and split picks use strict total orders; the counters are sums).
#[must_use]
/// v3.4 E2: the batch starting at position `b0`: (its index k, its end): the plan's batch, or v3.3's uniform cut of `lpb` lists.
///
/// # Errors
/// A position that does not start a plan batch.
pub(crate) fn batch_next(plan: Option<&crate::kit::Plan>, b0: usize, lpb: usize, nl: usize) -> Result<(usize, usize), String> {
    // MUTANT m_shard_plan_from_size_of: the plan ignored and the batches cut by this target's own size_of gate
    match plan.filter(|_| !cfg!(feature = "m_shard_plan_from_size_of")) {
        Some(p) => p.batch_at(b0),
        None => Ok((b0 / lpb.max(1), (b0 + lpb).min(nl))),
    }
}

/// v3.4 E2: v3.3's lists per batch for `nset` settings at the account gate `batch_acc` (the size_of rule under SPLITS18), exactly as
/// `stage_b_day_major_v33` computes it: what `--write-plan` cuts with on THIS target (a plan then fixes it for every target).
#[must_use]
pub fn lpb33(nset: usize, batch_acc: u64) -> usize {
    let per_list = u64::try_from(Layout33::new(crate::sw33::on(crate::sw33::COMPACT)).per_list(nset)).unwrap_or(u64::MAX);
    let lpb = usize::try_from((batch_acc / per_list.max(1)).max(1)).unwrap_or(1);
    if sw::on(sw::SPLITS18) {
        let base = std::mem::size_of::<(crate::kern::Acc, Pieces)>() + crate::wf::NPIECE * std::mem::size_of::<crate::kern::Piece>();
        let extra = crate::split18::cal().map_or(0, |_| std::mem::size_of::<crate::split18::Ser18>() + 16);
        (lpb * base / (base + extra).max(1)).max(1)
    } else {
        lpb
    }
}

/// v3.4 E2 / E6 / E11: the set-up's digest, the plan check (`--batch-plan`) or the plan written (`--write-plan`, then the run stops).
/// Returns Some(words) when a plan was written (the caller stops).
fn kit_setup(sr: &Seg3, seg: u8, days: &[String], csets: &[CSet], sel: &[usize], sets_sha: &str, menu_sha: &str, batch_acc: u64, threads: usize, log: &mut String) -> Result<Option<String>, String> {
    let need = crate::kit::PLAN.get().is_some() || crate::kit::WRITE_PLAN.get().is_some() || crate::kit::any();
    if !need {
        return Ok(None);
    }
    let t0 = Instant::now();
    let dg = crate::kit::list_digest(sr, days, csets, sel, sets_sha, menu_sha, threads);
    let _ = crate::kit::DIGEST.set(dg.clone());
    let ord = batch_order33(&sr.uls, seg);
    let ordered = ord.iter().enumerate().any(|(p, &u)| p != u);
    let per_list = u64::try_from(Layout33::new(crate::sw33::on(crate::sw33::COMPACT)).per_list(csets.len())).unwrap_or(0);
    lg!(log, "{}: v3.4 E6 list digest {dg} ({} events, {} config lists, {} unique lists; {:.1} s)", SEG_SHORT.get(usize::from(seg)).copied().unwrap_or("?"), sr.evs.len(), sr.lists.len(), sr.uls.len(), t0.elapsed().as_secs_f64());
    crate::kit::strict_check("set-up")?;
    if let Some(p) = crate::kit::PLAN.get() {
        p.check(seg, days, csets.len(), &dg, crate::kit::ARGS_DIGEST.get().map_or("", String::as_str), &ord, ordered, per_list)?;
        lg!(log, "v3.4 E2: batch plan {} matches this run's set-up ({} batches; segment, days, settings, order, list ids and list digest equal)", p.path, p.b.len());
    }
    let Some(wp) = crate::kit::WRITE_PLAN.get() else { return Ok(None) };
    if wp.exists() {
        return Err(format!("--write-plan {}: the file exists (never overwritten): refused", wp.display()));
    }
    let lpbo = usize::try_from(crate::kit::LPB.load(Ordering::Relaxed)).unwrap_or(0);
    let (lpb, cut) = if lpbo > 0 { (lpbo, format!("--lists-per-batch {lpbo}")) } else { let l = lpb33(csets.len(), batch_acc); (l, format!("v3.3 gate --batch-accounts {batch_acc} on {}-{} (size_of rule) = {l} lists per batch", std::env::consts::ARCH, std::env::consts::OS)) };
    // E11: each batch's cost = its lists' events x accounts per list (account-event pairs), the lists derived in parallel
    let nl = ord.len();
    let next = AtomicUsize::new(0);
    let lens: Mutex<Vec<(usize, u128)>> = Mutex::new(Vec::with_capacity(nl));
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                let mut mine = Vec::new();
                loop {
                    let u = next.fetch_add(1, Ordering::Relaxed);
                    if u >= nl {
                        break;
                    }
                    mine.push((u, sr.ul(u).map_or(0, |l| l.len() as u128)));
                }
                if let Ok(mut v) = lens.lock() {
                    v.extend(mine);
                }
            });
        }
    });
    let mut ev_of = vec![0_u128; nl];
    for (u, n) in lens.into_inner().unwrap_or_default() {
        if let Some(x) = ev_of.get_mut(u) {
            *x = n;
        }
    }
    let mut b = Vec::new();
    let mut p0 = 0;
    while p0 < nl {
        let p1 = (p0 + lpb).min(nl);
        let ids: Vec<u32> = ord.get(p0..p1).unwrap_or(&[]).iter().map(|&u| u32::try_from(u).unwrap_or(u32::MAX)).collect();
        let cost: u128 = ids.iter().map(|&u| ev_of.get(u as usize).copied().unwrap_or(0)).sum::<u128>() * u128::from(per_list);
        b.push((p0, p1, cost, ids));
        p0 = p1;
    }
    let pl = crate::kit::Plan { path: wp.display().to_string(), sha: String::new(), seg, days: days.to_vec(), nset: csets.len(), nl, digest: dg, args: crate::kit::ARGS_DIGEST.get().cloned().unwrap_or_default(), order: if ordered { "config-sorted".to_owned() } else { "creation".to_owned() }, per_list, cut, b };
    let text = pl.text();
    let part = wp.with_extension("part");
    std::fs::write(&part, &text).map_err(|e| format!("{}: {e}", part.display()))?;
    std::fs::rename(&part, wp).map_err(|e| format!("{}: {e}", wp.display()))?;
    let w = format!("PLAN WRITTEN {}: segment {seg}, {} unique lists in {} batches ({}), list digest {}, plan sha256 {}; total cost {} account-event pairs", wp.display(), nl, pl.b.len(), pl.cut, pl.digest, crate::kit::sha256_bytes(text.as_bytes()), pl.b.iter().map(|x| x.2).sum::<u128>());
    lg!(log, "{w}");
    Ok(Some(w))
}

pub fn batch_order33(uls: &[(u32, u8, u8)], seg: u8) -> Vec<usize> {
    let mut ord: Vec<usize> = (0..uls.len()).collect();
    if crate::sw33::on(crate::sw33::ORDER) && seg == 0 {
        ord.sort_by_key(|&u| uls.get(u).map_or((u32::MAX, u8::MAX, u), |x| (x.0, x.1, u)));
    }
    ord
}

/// v3.3: the day-major stage B with the exact speed switches of SPEED-PLAN S1 (each alone or together; every result byte for byte as round
/// B's [`stage_b_day_major`]): C1 compact states (COMPACT), C3 derived lists (`sr.ul_derived`), C6 batch order (ORDER), largest-first
/// per-day queue (LARGEST), the hoisted position buffer (HOIST), the next day's cube prefetched (PREFETCH). The account rules, the kernel,
/// the offers, the checkpoint and the counters are round B's.
#[allow(clippy::too_many_arguments)]
fn stage_b_day_major_v33(sr: &mut Seg3, store: &mut Store, var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, ma: &MenuA, sets: &Sets, sel: &[usize], csets: &[CSet], seg: u8, compat: bool, skip: bool, threads: usize, topk: usize, batch_acc: u64, log: &mut String) -> Result<(), String> {
    let tb = Instant::now();
    let compact = crate::sw33::on(crate::sw33::COMPACT);
    let largest = crate::sw33::on(crate::sw33::LARGEST);
    let hoist = crate::sw33::on(crate::sw33::HOIST);
    let prefetch = crate::sw33::on(crate::sw33::PREFETCH);
    let dayclass = crate::sw33::on(crate::sw33::DAYCLASS);
    let measure = crate::sw33::MEASURE.load(Ordering::Relaxed);
    // D2-1 counters: replayed account-days, fallen back (R < R*), class accounts ghost-walked, exits at or before booking (must be 0),
    // classes, lists in classes of >= 2, lists walked (class members)
    let mut d2c = [0_u64; 7];
    let derived = sr.ul_derived;
    // C1: the state layout (list, setting, walked level, N); full = round B's (list, setting, level 0..5, N)
    let walked: Vec<usize> = (0..NL).filter(|&l| trade::share_on(l)).collect();
    let lay = Layout33::new(compact);
    let per_list_n = lay.per_list(csets.len());
    let idx_of = |si: usize, l: usize, ni: usize| -> usize { lay.idx(si, l, ni) };
    let per_list = u64::try_from(per_list_n).unwrap_or(u64::MAX);
    let lists_per_batch = usize::try_from((batch_acc / per_list.max(1)).max(1)).unwrap_or(1);
    // v3.2-B (addendum 18): every account's split state counted in the batch gate (round B's rule)
    let lists_per_batch = if sw::on(sw::SPLITS18) {
        let base = std::mem::size_of::<(crate::kern::Acc, Pieces)>() + crate::wf::NPIECE * std::mem::size_of::<crate::kern::Piece>();
        let extra = crate::split18::cal().map_or(0, |_| std::mem::size_of::<crate::split18::Ser18>() + 16);
        (lists_per_batch * base / (base + extra).max(1)).max(1)
    } else {
        lists_per_batch
    };
    // v3.4 E2: with an explicit plan the size_of gate is never consulted: the batches are the plan's (checked against this run in cmd_run3)
    let plan = crate::kit::PLAN.get();
    let lists_per_batch = plan.filter(|_| !cfg!(feature = "m_shard_plan_from_size_of")).map_or(lists_per_batch, |p| p.b.iter().map(|x| x.1 - x.0).max().unwrap_or(1));
    let shard = crate::kit::on(crate::kit::SHARD);
    let per_batch = shard || crate::kit::on(crate::kit::BFILES);
    let sshort = SEG_SHORT.get(usize::from(seg)).copied().unwrap_or("x");
    let nl = sr.uls.len();
    let ord = batch_order33(&sr.uls, seg);
    let ordered = ord.iter().enumerate().any(|(p, &u)| p != u);
    let ndays = u16::try_from(sr.days.len()).unwrap_or(0);
    let ng = sr.ngroup.max(NGROUP);
    let headline = sw::on(sw::HEADLINE);
    let mut tops: Vec<Tops> = (0..ng).map(|_| Tops::new(topk)).collect();
    let (mut steps, mut accts, mut ta) = (0_u64, 0_u64, 0.0_f64);
    let of_day: Vec<u8> = sr.piece_of_day.clone();
    let rows = crate::wf::rows21();
    let present: Vec<bool> = (0..crate::wf::NPIECE).map(|p| sr.days.iter().any(|&d| of_day.get(d).copied() == Some(u8::try_from(p).unwrap_or(0)))).collect();
    let run_rows: Vec<bool> = rows.iter().map(|r| (r.learn.0..=r.learn.1).any(|p| present.get(p).copied().unwrap_or(false))).collect();
    let any_row = run_rows.iter().any(|&x| x);
    let mut win: Vec<WinRes> = rows.iter().zip(&run_rows).map(|(r, &run)| WinRes { row: Some(r.clone()), run, ..WinRes::default() }).collect();
    sr.counted = vec![false; sr.evs.len()];
    sr.cnt = Counts::new(ma.rows.len());
    // the events of no unique list (priced, and counted, in the first batch); derived: an event is in some unique list iff some config list
    // holds it and some run liquidity value passes it (every non-empty (config, liquidity) pair's filtered list IS a unique list's events)
    let mut in_any = vec![false; sr.evs.len()];
    if derived {
        let lv: Vec<usize> = sr.uls.iter().map(|x| usize::from(x.1)).collect::<std::collections::BTreeSet<usize>>().into_iter().collect();
        let mut used = vec![false; sr.evs.len()];
        for l in &sr.lists {
            for &e in l {
                if let Some(x) = used.get_mut(e as usize) {
                    *x = true;
                }
            }
        }
        // the run's liquidity values = the values of every unique list's pairs (a value whose pairs all came out empty adds no event)
        let liqv: Vec<usize> = {
            let mut s: std::collections::BTreeSet<usize> = lv.into_iter().collect();
            for c in &sr.ul_cfgs {
                for &(_, v) in c {
                    s.insert(usize::from(v));
                }
            }
            s.into_iter().collect()
        };
        for (i, x) in in_any.iter_mut().enumerate() {
            *x = used.get(i).copied().unwrap_or(false) && sr.liq.get(i).is_some_and(|&(c, g)| liqv.iter().any(|&v| liq_pass(v, c, g)));
        }
    } else {
        for l in &sr.ul_evs {
            for &e in l {
                if let Some(x) = in_any.get_mut(e as usize) {
                    *x = true;
                }
            }
        }
    }
    let mut orphans: BTreeMap<u16, Vec<usize>> = BTreeMap::new();
    for (i, e) in sr.evs.iter().enumerate() {
        if !in_any.get(i).copied().unwrap_or(true) {
            orphans.entry(e.di).or_default().push(i);
        }
    }
    sr.in_any = in_any;
    let mut flagged: Vec<Flag> = Vec::new();
    let mut nbk = [(0_u64, 0_u64); 4];
    let mut batches = 0;
    let mut sel18 = crate::split18::Sel18::new();
    let mut b0 = 0;
    let mut peak_bytes = 0_usize;
    let ck_path = crate::ckpt::CKPT_DIR.get().map(|d| d.join(format!("{}.ckpt", SEG_SHORT.get(usize::from(seg)).copied().unwrap_or("x"))));
    // the fingerprint: round B's, plus the v3.3 words and the batch order (a resume never mixes two layouts, orders or list stores)
    // v3.4 E2 + E6: a planned run's fingerprint also names the plan's sha256 and the set-up's list digest (empty without a plan: P0)
    let fp = format!("seg {seg}; days {} ({}..{}); settings {}; unique lists {nl}; lists per batch {lists_per_batch}; share mask {}; groups {ng}; skip {skip}; switches {}{}{}{}{}{}", sr.days.len(), sr.days.first().copied().unwrap_or(0), sr.days.last().copied().unwrap_or(0), csets.len(), trade::SHARE_MASK.load(Ordering::Relaxed), sw::words(), sw::words32(), fp_v33(ordered), crate::fix33::fp_part(), crate::fixg::fp_part(), crate::kit::fp_part());
    let mut sbuf: Vec<u32> = Vec::new();
    let mut kbuf: Vec<u32> = Vec::new();
    // v3.4 E1: a shard never reads or writes the cumulative checkpoint (its unit is the batch file); one in its folder is refused
    if shard
        && let Some(p) = &ck_path
        && p.exists()
    {
        return Err(format!("{}: a cumulative checkpoint in a shard's folder (E1: a shard writes batch files only): refused; give the shard its own folder", p.display()));
    }
    if let Some(p) = ck_path.as_ref().filter(|_| !shard)
        && let Some(c) = crate::ckpt::load(p, &fp, ng, rows.len())?
    {
        b0 = c.next_b0;
        batches = c.batches;
        steps = c.steps;
        accts = c.accts;
        nbk = c.nbk;
        tops = c.tops;
        flagged = c.flagged;
        sel18.ckpt_restore(&c.sp18)?;
        for (w, (ck, rw, pe, best)) in win.iter_mut().zip(c.win) {
            w.checked = ck;
            w.rewalked = rw;
            w.pending = pe;
            w.best = best;
        }
        let t0 = Instant::now();
        let done: Vec<usize> = ord.iter().take(b0.min(nl)).copied().collect();
        let wkeys: Vec<Vec<u32>> = if derived { Vec::new() } else { done.iter().map(|&u| sr.ul_evs.get(u).map_or_else(Vec::new, |l| list_keys(&sr.evs, l))).collect() };
        for &di in &sr.days.clone() {
            let mut want: Vec<usize> = Vec::new();
            for (k, &u) in done.iter().enumerate() {
                sr.ul_day(u, di, wkeys.get(k).map(Vec::as_slice), &mut sbuf, &mut kbuf);
                want.extend(sbuf.iter().map(|&e| e as usize));
            }
            if b0 > 0
                && let Some(v) = orphans.get(&u16::try_from(di).unwrap_or(0))
            {
                want.extend(v.iter().copied());
            }
            want.sort_unstable();
            want.dedup();
            if want.is_empty() {
                continue;
            }
            store.ensure(&[di])?;
            sr.tab = Tab3::default();
            stage_a_events_c(sr, Some(&want), &store.cubes, &store.tapes, var, dial, ma, seg, compat, threads, true)?;
            store.release(&[]);
        }
        ta += t0.elapsed().as_secs_f64();
        lg!(log, "{}: RESUMED from checkpoint {} (REPAIR A / v3.3): {batches} finished batch(es) = batch positions 0..{b0} not walked again; their events priced once in a union stage A pass ({:.1} s)", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), p.display(), t0.elapsed().as_secs_f64());
    }
    let days_all: Vec<usize> = sr.days.clone();
    let ckdir = crate::ckpt::CKPT_DIR.get().cloned();
    let bset = crate::kit::BATCH_SET.get();
    while b0 < nl {
        // v3.4 E2: the batch starting at b0 (the plan's, or v3.3's uniform cut; k = its index)
        let (bk, b1) = batch_next(plan, b0, lists_per_batch, nl)?;
        // v3.4 E1: a shard walks only its assigned batches whose file is not already there and verified
        if shard {
            let dir = ckdir.as_ref().ok_or("--shard needs --checkpoint DIR")?;
            let assigned = bset.is_some_and(|s| s.contains(&bk));
            if !crate::kit::shard_walks(bk, bset, assigned && crate::kit::bck_done(dir, sshort, bk, &fp)?) {
                b0 = b1;
                continue;
            }
        }
        batches += 1;
        let blist: Vec<usize> = ord.get(b0..b1).unwrap_or(&[]).to_vec();
        let nb = blist.len();
        let n_acc = nb * per_list_n;
        let mut states: Vec<(crate::kern::Acc, Pieces)> = vec![(crate::kern::Acc::new(), if any_row { Pieces::new(&of_day, crate::wf::NPIECE) } else { Pieces::new(&[], 0) }); n_acc];
        let wkeys: Vec<Vec<u32>> = if derived { Vec::new() } else { blist.iter().map(|&u| sr.ul_evs.get(u).map_or_else(Vec::new, |l| list_keys(&sr.evs, l))).collect() };
        let tfs: Vec<Option<u8>> = blist.iter().map(|&u| sr.uls.get(u).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) })).collect();
        // v3.4 E1: a shard never prices the orphan events (counters only; the Mac's union pass rebuilds them)
        let first_batch = batches == 1 && !shard;
        // v3.4 E1 / E7: the batch's OWN contributions, kept beside the cumulative state (which stays v3.3's, byte for byte)
        let (bst0, bac0, bnbk0, bd2c0, bfl0) = (steps, accts, nbk, d2c, flagged.len());
        let (bt_utc, bt_wall, bt_cpu) = (crate::kit::utc_now(), Instant::now(), if per_batch { crate::kit::proc_cpu_s().unwrap_or_else(proc_cpu_s) } else { 0.0 });
        let mut btops: Vec<Tops> = if per_batch { (0..ng).map(|_| Tops::new(topk)).collect() } else { Vec::new() };
        let mut bwin: Vec<(u64, u64, u64, crate::kit::WinBest)> = if per_batch { vec![(0, 0, 0, None); rows.len()] } else { Vec::new() };
        let mut bsel = per_batch.then(|| sel18.batch_start());
        let mut acct_h = crate::kit::Sha256::new();
        let mut acct_n = 0_u64;
        // the day's parts of the batch: (want, per list (events, keys))
        let parts_of = |srr: &Seg3, di: usize| -> (Vec<usize>, Vec<(Vec<u32>, Vec<u32>)>) {
            let mut want: Vec<usize> = Vec::new();
            let mut subs: Vec<(Vec<u32>, Vec<u32>)> = Vec::with_capacity(nb);
            for (k, &u) in blist.iter().enumerate() {
                let (mut s, mut kk) = (Vec::new(), Vec::new());
                srr.ul_day(u, di, wkeys.get(k).map(Vec::as_slice), &mut s, &mut kk);
                want.extend(s.iter().map(|&e| e as usize));
                subs.push((s, kk));
            }
            if first_batch
                && let Some(v) = orphans.get(&u16::try_from(di).unwrap_or(0))
            {
                want.extend(v.iter().copied());
            }
            want.sort_unstable();
            want.dedup();
            (want, subs)
        };
        // the batch's days with something to price, in order; the next one's parts are made (and its cube prefetched) during the walk
        let mut pos = 0_usize;
        let next_parts = |srr: &Seg3, pos: &mut usize| -> Option<(usize, Vec<usize>, Vec<(Vec<u32>, Vec<u32>)>)> {
            while let Some(&di) = days_all.get(*pos) {
                *pos += 1;
                let (want, subs) = parts_of(srr, di);
                if !want.is_empty() {
                    return Some((di, want, subs));
                }
            }
            None
        };
        let mut cur = next_parts(sr, &mut pos);
        // --measure33 (a measurement, never a result): process CPU around each day's stage A (+ the next day's parts) and around its walk
        let (mut ma_cpu, mut mw_cpu, mut ma_wall, mut mw_wall, s_at0, mut mdays, mut mpairs) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, steps, 0_u32, 0_u64);
        let acc_per_list = u64::try_from(csets.len() * walked.len() * 4).unwrap_or(0);
        let mut mcpu = if measure { proc_cpu_s() } else { 0.0 };
        while let Some((di, want, subs)) = cur.take() {
            let t0 = Instant::now();
            store.ensure(&[di])?;
            sr.tab = Tab3::default();
            stage_a_events_c(sr, Some(&want), &store.cubes, &store.tapes, var, dial, ma, seg, compat, threads, true)?;
            store.release(&[]);
            ta += t0.elapsed().as_secs_f64();
            peak_bytes = peak_bytes.max(sr.tab.bytes() + n_acc * std::mem::size_of::<(crate::kern::Acc, Pieces)>());
            // the next day's parts (cheap) and, under PREFETCH, its cube + tape loaded on a loader thread while this day is walked
            let nxt = next_parts(sr, &mut pos);
            let pf_src = if prefetch { nxt.as_ref().and_then(|x| pf_src33(store, x.0)) } else { None };
            if measure {
                let c = proc_cpu_s();
                ma_cpu += c - mcpu;
                mcpu = c;
                ma_wall += t0.elapsed().as_secs_f64();
                mdays += 1;
                mpairs += subs.iter().map(|s| u64::try_from(s.0.len()).unwrap_or(0)).sum::<u64>() * acc_per_list;
            }
            let tw = Instant::now();
            if dayclass && seg == 0 {
                // D2-1 (index): the day's lists grouped into classes by (timeframe key, day sub-list); one ghost walk per class account,
                // replayed on each member account at or above its R* (else walked)
                let classes = day_classes33(&subs, &tfs);
                let nst = lay.per_list(1);
                let nch = csets.len().div_ceil(SC33).max(1);
                let mut cells: Vec<Mutex<&mut [(crate::kern::Acc, Pieces)]>> = Vec::with_capacity(nb * nch);
                for lst in states.chunks_mut(per_list_n.max(1)) {
                    let mut c = 0;
                    for ch in lst.chunks_mut((SC33 * nst).max(1)) {
                        cells.push(Mutex::new(ch));
                        c += 1;
                    }
                    for _ in c..nch {
                        cells.push(Mutex::new(&mut []));
                    }
                }
                let mut citems: Vec<(usize, usize)> = (0..classes.len()).flat_map(|ci| (0..nch).map(move |c| (ci, c))).collect();
                if largest {
                    let cost = |ci: usize| -> usize { classes.get(ci).map_or(0, |m| m.first().and_then(|&k| subs.get(k)).map_or(0, |s| s.0.len()) * (5 + m.len())) };
                    citems.sort_by_key(|&(ci, c)| (std::cmp::Reverse(cost(ci)), ci, c));
                }
                let next = AtomicUsize::new(0);
                let tot = Mutex::new((0_u64, [0_u64; 4]));
                // v3.3 fix STEP_DM (check-engine-2 M3): sampled account-day walks (inert with the switch off)
                let stride33 = sr.step33.stride;
                let smp_all: Mutex<Vec<(u64, u64)>> = Mutex::new(Vec::new());
                let srr = &*sr;
                let of_day = &of_day;
                let walked = &walked;
                let (citems, classes, subs, tfs, cells) = (&citems, &classes, &subs, &tfs, &cells);
                let idx_of = &idx_of;
                let mut fetched: Option<(usize, Result<(Cube, SecTape), String>)> = None;
                std::thread::scope(|s| {
                    let pf = pf_src.map(|(d, (cdir, name))| (d, s.spawn(move || -> Result<(Cube, SecTape), String> { Ok((crate::cube::load_m(&cdir, &name)?, crate::cube::load_sec_m(&cdir, &name)?)) })));
                    for _ in 0..threads {
                        s.spawn(|| {
                            let mut my = 0_u64;
                            // counters: replayed account-days, fallen back (R < R*), class accounts ghost-walked, exits at or before booking
                            let mut cnt = [0_u64; 4];
                            let mut buf = vec![crate::kern::Pos::default(); crate::kern::MAXCAP];
                            let mut tr = crate::kern::Trace::default();
                            let mut smp = crate::fix33::StepSampler::new(stride33);
                            let piece = of_day.get(di).copied().unwrap_or(0);
                            let dday = u16::try_from(di).unwrap_or(u16::MAX);
                            loop {
                                let j = next.fetch_add(1, Ordering::Relaxed);
                                let Some(&(ci, c)) = citems.get(j) else { break };
                                let Some(mem) = classes.get(ci) else { continue };
                                let Some(&k0) = mem.first() else { continue };
                                let (Some((sub, subk)), Some(&tf)) = (subs.get(k0), tfs.get(k0)) else { continue };
                                let mut guards: Vec<std::sync::MutexGuard<'_, &mut [(crate::kern::Acc, Pieces)]>> = Vec::with_capacity(mem.len());
                                for &k in mem {
                                    if let Some(cell) = cells.get(k * nch + c)
                                        && let Ok(gd) = cell.lock()
                                    {
                                        guards.push(gd);
                                    }
                                }
                                if guards.len() != mem.len() {
                                    continue;
                                }
                                let s0 = c * SC33;
                                let s1 = (s0 + SC33).min(csets.len());
                                for si in s0..s1 {
                                    let Some(cs) = csets.get(si) else { continue };
                                    for &l in walked {
                                        for ni in 0..4 {
                                            let Some(ix) = idx_of(si, l, ni).checked_sub(s0 * nst) else { continue };
                                            if mem.len() == 1 {
                                                if let Some(g0) = guards.first_mut()
                                                    && let Some(stt) = g0.get_mut(ix)
                                                {
                                                    let t0 = smp.want().then(Instant::now);
                                                    let st1 = crate::kern::walk_day_h(&srr.evs, &srr.ex, &srr.tab, sub, subk, cs, l, ni, tf, true, skip, &mut stt.0, &mut stt.1, of_day, &mut buf);
                                                    if let Some(t0) = t0 {
                                                        smp.push(u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX), st1);
                                                    }
                                                    my += st1;
                                                }
                                                continue;
                                            }
                                            let t0 = smp.want().then(Instant::now);
                                            crate::kern::ghost_day(&srr.evs, &srr.ex, &srr.tab, sub, subk, cs, l, ni, tf, skip, of_day, &mut buf, &mut tr);
                                            if let Some(t0) = t0 {
                                                smp.push(u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX), tr.steps);
                                            }
                                            my += tr.steps;
                                            cnt[2] += 1;
                                            // DESIGN-D2 Lemma D's premise: every exit strictly after its booking second (else the class walks)
                                            let bad = tr.booked.iter().filter(|(k, p)| p.exit <= *k).count() as u64;
                                            cnt[3] += bad;
                                            let ncap = crate::kern::cap_at(crate::kern::seg_of(&srr.evs, sub), ni);
                                            for gd in &mut guards {
                                                let Some(stt) = gd.get_mut(ix) else { continue };
                                                if bad == 0 && crate::kern::replay_day(&mut stt.0, &mut stt.1, &tr, dday, piece, &mut buf, ncap) {
                                                    cnt[0] += 1;
                                                } else {
                                                    cnt[1] += 1;
                                                    let t0 = smp.want().then(Instant::now);
                                                    let st1 = crate::kern::walk_day_h(&srr.evs, &srr.ex, &srr.tab, sub, subk, cs, l, ni, tf, true, skip, &mut stt.0, &mut stt.1, of_day, &mut buf);
                                                    if let Some(t0) = t0 {
                                                        smp.push(u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX), st1);
                                                    }
                                                    my += st1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if let Ok(mut t) = tot.lock() {
                                t.0 += my;
                                for (a, b) in t.1.iter_mut().zip(cnt) {
                                    *a += b;
                                }
                            }
                            if !smp.blocks.is_empty()
                                && let Ok(mut v) = smp_all.lock()
                            {
                                v.append(&mut smp.blocks);
                            }
                        });
                    }
                    if let Some((d, h)) = pf {
                        fetched = Some((d, h.join().unwrap_or_else(|_| Err("prefetch thread".to_owned()))));
                    }
                });
                let (st, cn) = tot.into_inner().unwrap_or((0, [0; 4]));
                steps += st;
                let b33 = smp_all.into_inner().unwrap_or_default();
                if !b33.is_empty() {
                    sr.step33.merge(b33);
                    sr.step33.dm = true;
                }
                for (a, b) in d2c.iter_mut().zip(cn) {
                    *a += b;
                }
                d2c[4] += u64::try_from(classes.len()).unwrap_or(0);
                d2c[5] += u64::try_from(classes.iter().filter(|m| m.len() >= 2).map(Vec::len).sum::<usize>()).unwrap_or(0);
                d2c[6] += u64::try_from(classes.iter().map(Vec::len).sum::<usize>()).unwrap_or(0);
                if let Some((d, r)) = fetched {
                    let (c, t) = r?;
                    store.put(d, c, t);
                }
                if measure {
                    let c = proc_cpu_s();
                    mw_cpu += c - mcpu;
                    mcpu = c;
                    mw_wall += tw.elapsed().as_secs_f64();
                }
                cur = nxt;
                continue;
            }
            // largest first: the day's lists by sub-list length (descending), ties by batch position
            let lens: Vec<usize> = subs.iter().map(|s| s.0.len()).collect();
            let items = day_items33(&lens, largest);
            let next = AtomicUsize::new(0);
            let st_cell: Vec<Mutex<&mut [(crate::kern::Acc, Pieces)]>> = states.chunks_mut(per_list_n.max(1)).map(Mutex::new).collect();
            let tot = Mutex::new(0_u64);
            // v3.3 fix STEP_DM (check-engine-2 M3): sampled account-day walks (inert with the switch off)
            let stride33 = sr.step33.stride;
            let smp_all: Mutex<Vec<(u64, u64)>> = Mutex::new(Vec::new());
            let srr = &*sr;
            let of_day = &of_day;
            let walked = &walked;
            let (items, subs, tfs) = (&items, &subs, &tfs);
            let idx_of = &idx_of;
            let mut fetched: Option<(usize, Result<(Cube, SecTape), String>)> = None;
            std::thread::scope(|s| {
                let pf = pf_src.map(|(d, (cdir, name))| (d, s.spawn(move || -> Result<(Cube, SecTape), String> { Ok((crate::cube::load_m(&cdir, &name)?, crate::cube::load_sec_m(&cdir, &name)?)) })));
                for _ in 0..threads {
                    s.spawn(|| {
                        let mut my = 0_u64;
                        let mut buf = if hoist { vec![crate::kern::Pos::default(); crate::kern::MAXCAP] } else { Vec::new() };
                        let mut smp = crate::fix33::StepSampler::new(stride33);
                        loop {
                            let j = next.fetch_add(1, Ordering::Relaxed);
                            let Some(&k) = items.get(j) else { break };
                            let (Some((sub, subk)), Some(cell), Some(&tf)) = (subs.get(k), st_cell.get(k), tfs.get(k)) else { continue };
                            let Ok(mut g) = cell.lock() else { continue };
                            for (si, cs) in csets.iter().enumerate() {
                                for &l in walked {
                                    for ni in 0..4 {
                                        if let Some(stt) = g.get_mut(idx_of(si, l, ni)) {
                                            let t0 = smp.want().then(Instant::now);
                                            let st1 = if hoist {
                                                crate::kern::walk_day_h(&srr.evs, &srr.ex, &srr.tab, sub, subk, cs, l, ni, tf, true, skip, &mut stt.0, &mut stt.1, of_day, &mut buf)
                                            } else {
                                                crate::kern::walk_day(&srr.evs, &srr.ex, &srr.tab, sub, subk, cs, l, ni, tf, true, skip, &mut stt.0, &mut stt.1, of_day)
                                            };
                                            if let Some(t0) = t0 {
                                                smp.push(u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX), st1);
                                            }
                                            my += st1;
                                        }
                                    }
                                }
                            }
                        }
                        if let Ok(mut t) = tot.lock() {
                            *t += my;
                        }
                        if !smp.blocks.is_empty()
                            && let Ok(mut v) = smp_all.lock()
                        {
                            v.append(&mut smp.blocks);
                        }
                    });
                }
                if let Some((d, h)) = pf {
                    fetched = Some((d, h.join().unwrap_or_else(|_| Err("prefetch thread".to_owned()))));
                }
            });
            steps += tot.into_inner().unwrap_or(0);
            let b33 = smp_all.into_inner().unwrap_or_default();
            if !b33.is_empty() {
                sr.step33.merge(b33);
                sr.step33.dm = true;
            }
            if let Some((d, r)) = fetched {
                let (c, t) = r?;
                store.put(d, c, t);
            }
            if measure {
                let c = proc_cpu_s();
                mw_cpu += c - mcpu;
                mcpu = c;
                mw_wall += tw.elapsed().as_secs_f64();
            }
            cur = nxt;
        }
        // finalize the batch's accounts (round B's loop; positions mapped to their lists)
        for (k, &u) in blist.iter().enumerate() {
            let spike = spike_of(sr, u);
            for si in 0..csets.len() {
                let g = groups_of(sets, sel, si);
                let search = sr.search.get(si).copied().unwrap_or(false);
                for &l in &walked {
                    for ni in 0..4 {
                        let idx = k * per_list_n + idx_of(si, l, ni);
                        let Some((ac, pc)) = states.get_mut(idx) else { continue };
                        let mut sm = crate::kern::finish_p(ac, pc, ni, ndays);
                        accts += 1;
                        if let Some(x) = nbk.get_mut(ni) {
                            x.0 += 1;
                            x.1 += u64::from(sm.trades > 0);
                        }
                        sm.seg = seg;
                        sm.list = u32::try_from(u).unwrap_or(0);
                        sm.x = u16::try_from(si * NL + l).unwrap_or(u16::MAX);
                        sm.spike = spike;
                        sm.tie = sr.ties.get(si).copied().unwrap_or(0);
                        let tie18 = wtie(&sm);
                        // v3.4: under E1 / E7 the batch's own split accumulator (it continues the cumulative one's sampling; absorbed below)
                        crate::split18::account(pc.ms.as_deref(), &mut sm, bsel.as_mut().unwrap_or(&mut sel18), search, tie18);
                        if any_row && search && sm.trades > 0 {
                            for (ri, (w, r)) in win.iter_mut().zip(&rows).enumerate() {
                                if !w.run || crate::fixg::skip_sliding(r.growing) {
                                    continue;
                                }
                                if crate::wf::fresh_equal(&pc.p, r.learn.0, r.test.1) {
                                    w.checked += 1;
                                    if let Some(bw) = bwin.get_mut(ri) {
                                        // v3.4: one candidate, offered to the cumulative row and to the batch's row by the same rule
                                        bw.0 += 1;
                                        if let Some((key, lf, tf2)) = row_candidate(r, &pc.p) {
                                            row_take(&mut w.best, key, sm, lf, tf2);
                                            row_take(&mut bw.3, key, sm, lf, tf2);
                                        }
                                    } else {
                                        offer_row(w, r, &pc.p, sm);
                                    }
                                } else {
                                    w.pending += 1;
                                    if let Some(bw) = bwin.get_mut(ri) {
                                        bw.2 += 1;
                                    }
                                    flagged.push(Flag { row: u8::try_from(ri).unwrap_or(u8::MAX), list: sm.list, si: u32::try_from(si).unwrap_or(0), l: u8::try_from(l).unwrap_or(0), ni: u8::try_from(ni).unwrap_or(0), sm });
                                }
                            }
                        }
                        for &gi in &g {
                            if let Some(t) = tops.get_mut(gi) {
                                t.offer(sm);
                            }
                            if let Some(t) = btops.get_mut(gi) {
                                t.offer(sm);
                            }
                        }
                        if headline && headline_ok(search, l, sr.uls.get(u).map_or(0, |x| x.1), &sm) {
                            if let Some(t) = tops.get_mut(NGROUP) {
                                t.offer(sm);
                            }
                            if let Some(t) = btops.get_mut(NGROUP) {
                                t.offer(sm);
                            }
                        }
                        if per_batch {
                            // critic F7a: every account's summary (finalize order) into the batch's all-accounts digest
                            acct_h.update(crate::ckpt::sum_line(&sm).as_bytes());
                            acct_h.update(b"\n");
                            acct_n += 1;
                        }
                    }
                }
            }
        }
        // v3.4 E1 / E7: the batch file (the batch's own contributions; canonical body + provenance)
        if let Some(bs) = bsel.take() {
            let (r_read, r_off, r_ns_sum, r_ns, shape, picks) = bs.batch_parts(sel18.read);
            sel18.absorb(bs);
            crate::kit::strict_check(&format!("{sshort} batch {bk}"))?;
            let dir = ckdir.as_ref().ok_or("batch files need --checkpoint DIR")?;
            let nbk_d: [(u64, u64); 4] = std::array::from_fn(|i| (nbk[i].0 - bnbk0[i].0, nbk[i].1 - bnbk0[i].1));
            let safe: [u64; 7] = std::array::from_fn(|i| d2c[i] - bd2c0[i]);
            let cpu = crate::kit::proc_cpu_s().unwrap_or_else(proc_cpu_s) - bt_cpu;
            let bf = crate::kit::Bck {
                fp: fp.clone(),
                k: bk,
                nb: plan.map_or(0, |p| p.b.len()),
                p0: b0,
                p1: b1,
                nl,
                tie52: sw::on(sw::TIE52),
                splits18: sw::on(sw::SPLITS18),
                steps: steps - bst0,
                accts: accts - bac0,
                nbk: nbk_d,
                safe,
                tops: btops.iter().map(Tops::canonical).collect(),
                win: std::mem::take(&mut bwin),
                flagged: flagged.get(bfl0..).unwrap_or(&[]).to_vec(),
                s18: (r_read, r_off),
                s18shape: shape,
                s18b: picks,
                acct: (acct_n, std::mem::take(&mut acct_h).hex()),
                body_sha: String::new(),
                ns_sum: r_ns_sum,
                s18n: r_ns,
                prov: crate::kit::provenance(threads, bt_utc, bt_wall.elapsed().as_secs_f64(), cpu),
            };
            let bp = crate::kit::write_bck(dir, sshort, &bf)?;
            lg!(log, "{}: BATCH FILE {} (batch {bk} of {}, positions {b0}..{b1}; {} accounts; body sha256 {})", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), bp.display(), bf.nb, bf.accts, crate::kit::sha256_bytes(bf.body().as_bytes()));
        }
        b0 = b1;
        if shard {
            let stop = crate::ckpt::STOP_AFTER.load(Ordering::Relaxed);
            if stop > 0 && batches >= stop {
                return Err(format!("stopped after {batches} batch file(s) by --stop-after-batches (shard test hook): re-run with the same arguments"));
            }
        }
        if let Some(p) = ck_path.as_ref().filter(|_| !shard) {
            let c = crate::ckpt::Ckpt { fingerprint: fp.clone(), next_b0: b0, batches, steps, accts, nbk, tops: tops.clone(), win: win.iter().map(|w| (w.checked, w.rewalked, w.pending, w.best)).collect(), flagged: flagged.clone(), sp18: sel18.ckpt_lines() };
            crate::ckpt::save(p, &c)?;
            lg!(log, "{}: CHECKPOINT {} after batch {batches} (batch positions 0..{b0} of {nl} finished)", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), p.display());
            let stop = crate::ckpt::STOP_AFTER.load(Ordering::Relaxed);
            if stop > 0 && batches >= stop && b0 < nl {
                return Err(format!("stopped after {batches} finished batch(es) by --stop-after-batches (checkpoint test hook): resume with the same arguments"));
            }
        }
        lg!(log, "{}: v3.3 batch {batches}: {nb} lists, {} accounts; cumulative stage A {ta:.1} s, stage B {:.1} s, steps {steps}; process CPU {:.1} s", SEG_SHORT.get(usize::from(seg)).copied().unwrap_or("?"), n_acc, tb.elapsed().as_secs_f64() - ta, proc_cpu_s());
        if measure {
            let bs = steps - s_at0;
            lg!(log, "MEASURE33 {} batch {batches}: lists {nb}; accounts {}; days priced {mdays}; account-event PAIRS {mpairs} (list events x walked accounts per list); account-event steps {bs}; stage A (+ next day.s parts) process CPU {ma_cpu:.1} s, wall {ma_wall:.1} s; walk process CPU {mw_cpu:.1} s, wall {mw_wall:.1} s; walk CPU per pair {:.3} ns, per step {:.2} ns; threads {threads}", SEG_SHORT.get(usize::from(seg)).copied().unwrap_or("?"), n_acc, if mpairs > 0 { mw_cpu * 1e9 / mpairs as f64 } else { 0.0 }, if bs > 0 { mw_cpu * 1e9 / bs as f64 } else { 0.0 });
        }
    }
    // C6: the flagged account-rows back in creation order (list, setting, share level, N, row): the D3 pass and every later reader see
    // exactly round B's sequence
    if ordered {
        flagged.sort_by_key(|f| (f.list, f.si, f.l, f.ni, f.row));
    }
    sr.tops = tops;
    sr.win = win;
    sr.flagged = flagged;
    sr.n_booked = nbk;
    sr.sp18 = sel18;
    sr.steps = steps;
    sr.accounts = accts;
    sr.t_stage_a = ta;
    sr.t_stage_b = tb.elapsed().as_secs_f64() - ta;
    sr.stage_a_bytes = peak_bytes;
    lg!(log, "{}: DAY-MAJOR {batches} batch(es) of up to {lists_per_batch} lists ({} accounts per list; gate {batch_acc} accounts); stage A per day per batch {ta:.1} s; stage B {:.1} s; {accts} accounts, {steps} steps; peak tables + states {} MB; stage A counted once per event: {} of {} events; window account-rows flagged for the exact re-walk (D3): {}; v3.3: {}", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), per_list, sr.t_stage_b, peak_bytes / 1_000_000, sr.counted.iter().filter(|&&c| c).count(), sr.evs.len(), sr.flagged.len(), crate::sw33::words());
    // the memory a walked account really holds (C1's line: the state, its pieces heap block and the segment series), MEASURED by size_of
    let heap = if any_row { crate::wf::NPIECE * std::mem::size_of::<crate::kern::Piece>() } else { 0 } + crate::split18::cal().filter(|_| sw::on(sw::SPLITS18)).map_or(0, |_| std::mem::size_of::<crate::split18::Ser18>() + 16);
    lg!(log, "{}: v3.3 bytes per held account state: {} inline + {heap} heap (pieces block + segment series) = {} B (allocator overhead not counted); {per_list_n} states per list (layout {})", SEG_SHORT.get(usize::from(seg)).copied().unwrap_or("?"), std::mem::size_of::<(crate::kern::Acc, Pieces)>(), std::mem::size_of::<(crate::kern::Acc, Pieces)>() + heap, if compact { "compact: walked share levels only" } else { "full: all 5 share levels" });
    if dayclass && seg == 0 {
        lg!(log, "{}: v3.3 D2-1 day classes: {} class-days over {} list-days ({} list-days in classes of >= 2); class accounts ghost-walked {}; member account-days REPLAYED {} / walked (R < R*) {}; exits at or before their booking second {} (must be 0: DESIGN-D2 Lemma D's premise; a class with one walks instead)", SEG_SHORT.get(usize::from(seg)).copied().unwrap_or("?"), d2c[4], d2c[6], d2c[5], d2c[2], d2c[0], d2c[1], d2c[3]);
    }
    Ok(())
}

/// Composes one account's pieces for one window row and offers the row's pick (the learning key, then the tie order).
/// fix:v33 WIN (Recheck-v33 F1), day-major: the sliding rows' candidate set as D3 flags (every SEARCH account with a trade that a group and
/// view keeps; bounded by groups x views x K), so [`d3_rewalk`] walks each FRESH from its row's first learning piece and offers it to the row
/// (exact). The sweep itself held no flagged row.
fn cand_flags(sr: &mut Seg3) {
    if !crate::fixg::on(crate::fixg::WIN) {
        return;
    }
    let rows = crate::wf::rows21();
    let cands = crate::fixg::candidates(&sr.tops, &sr.search, NL);
    crate::fixg::WIN_CANDS.fetch_add(u64::try_from(cands.len()).unwrap_or(0), Ordering::Relaxed);
    let mut add: Vec<Flag> = Vec::new();
    for (ri, (w, r)) in sr.win.iter().zip(&rows).enumerate() {
        if !w.run || r.growing {
            continue;
        }
        for &(list, si, l, ni, sm) in &cands {
            add.push(Flag { row: u8::try_from(ri).unwrap_or(u8::MAX), list, si, l, ni, sm });
        }
    }
    sr.flagged.extend(add);
}

/// fix:v33 WIN (Recheck-v33 F1), whole mode: each candidate walked FRESH from every sliding row's first learning piece (the whole mode's own
/// exact re-walk) and offered to the row.
fn cand_whole(sr: &mut Seg3, csets: &[CSet]) {
    if !crate::fixg::on(crate::fixg::WIN) {
        return;
    }
    let rows = crate::wf::rows21();
    let cands = crate::fixg::candidates(&sr.tops, &sr.search, NL);
    crate::fixg::WIN_CANDS.fetch_add(u64::try_from(cands.len()).unwrap_or(0), Ordering::Relaxed);
    let of_day = sr.piece_of_day.clone();
    let ndays = u16::try_from(sr.days.len()).unwrap_or(0);
    for (u, si, l, ni, sm) in cands {
        let Some(cs) = csets.get(si as usize) else { continue };
        let Some(list) = sr.ul(u as usize).map(std::borrow::Cow::into_owned) else { continue };
        let tf = sr.uls.get(u as usize).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) });
        for (ri, r) in rows.iter().enumerate() {
            if r.growing || !sr.win.get(ri).is_some_and(|w| w.run) {
                continue;
            }
            let start = r.learn.0;
            let sub: Vec<u32> = list.iter().copied().filter(|&e| sr.evs.get(e as usize).is_some_and(|ev| usize::from(of_day.get(usize::from(ev.di)).copied().unwrap_or(0)) >= start)).collect();
            let sk = list_keys(&sr.evs, &sub);
            let pp = walk_account(&sr.evs, &sr.ex, &sr.tab, &sub, &sk, cs, usize::from(l), usize::from(ni), tf, true, true, &of_day, crate::wf::NPIECE, ndays).1.p;
            if let Some(w) = sr.win.get_mut(ri) {
                offer_row(w, r, &pp, sm);
                w.rewalked += 1;
            }
        }
    }
    sr.rewalks = sr.win.iter().map(|w| w.rewalked).sum();
}

fn offer_row(w: &mut WinRes, r: &crate::wf::WinRow, p: &[crate::kern::Piece], sm: Sum) {
    if let Some((key, lf, tf2)) = row_candidate(r, p) {
        row_take(&mut w.best, key, sm, lf, tf2);
    }
}

/// v3.4 (split out of `offer_row`, unchanged): one account's candidate for a window row: (learning key, learning figure, test figure), or
/// None when it has no learning trade.
fn row_candidate(r: &crate::wf::WinRow, p: &[crate::kern::Piece]) -> Option<((i64, i64, i64), crate::wf::Fig, crate::wf::Fig)> {
    let lf = crate::wf::compose(p, r.learn.0, r.learn.1);
    if lf.trades == 0 {
        return None;
    }
    let tf2 = crate::wf::compose(p, r.test.0, r.test.1);
    Some((crate::wf::learn_key(&lf), lf, tf2))
}

/// v3.4: the window row's pick rule (the larger learning key, then the smaller `wtie`: strict, unique per account): the ONE rule of the sweep,
/// the batch files and `ckpt-merge`.
#[must_use]
pub(crate) fn row_better(key: (i64, i64, i64), sm: &Sum, b: &((i64, i64, i64), Sum, crate::wf::Fig, crate::wf::Fig)) -> bool {
    key > b.0 || (key == b.0 && wtie(sm) < wtie(&b.1))
}

/// v3.4: offers a candidate to a row's pick slot.
pub(crate) fn row_take(best: &mut Option<((i64, i64, i64), Sum, crate::wf::Fig, crate::wf::Fig)>, key: (i64, i64, i64), sm: Sum, lf: crate::wf::Fig, tf2: crate::wf::Fig) {
    if best.as_ref().is_none_or(|b| row_better(key, &sm, b)) {
        *best = Some((key, sm, lf, tf2));
    }
}

/// D3 (check-build-v3): the exact re-walk of every account-row the day-major slack check flagged: per distinct row start piece, a second
/// streamed pass over the days from that piece walks each flagged account as a FRESH account (positions never cross a day, so this is the
/// whole mode's walk from the row's first learning piece), then its pieces are composed and offered to the row. Nothing is left PENDING.
#[allow(clippy::too_many_arguments)]
pub(crate) fn d3_rewalk(sr: &mut Seg3, store: &mut Store, var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, ma: &MenuA, sets: &Sets, sel: &[usize], csets: &[CSet], seg: u8, compat: bool, threads: usize, log: &mut String) -> Result<(), String> {
    let _ = (sets, sel);
    if sr.flagged.is_empty() || cfg!(feature = "m_v31_window_pending") {
        return Ok(());
    }
    let t0 = Instant::now();
    let rows = crate::wf::rows21();
    let of_day: Vec<u8> = sr.piece_of_day.clone();
    let mut by_start: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (k, f) in sr.flagged.iter().enumerate() {
        if let Some(r) = rows.get(usize::from(f.row)) {
            by_start.entry(r.learn.0).or_default().push(k);
        }
    }
    let mut rewalked = 0_u64;
    for (start, fl) in by_start {
        // distinct accounts of this start
        let mut ix: HashMap<(u32, u32, u8, u8), usize> = HashMap::new();
        let mut accs: Vec<(u32, u32, u8, u8)> = Vec::new();
        for &k in &fl {
            let Some(f) = sr.flagged.get(k) else { continue };
            let key = (f.list, f.si, f.l, f.ni);
            if let std::collections::hash_map::Entry::Vacant(v) = ix.entry(key) {
                v.insert(accs.len());
                accs.push(key);
            }
        }
        let mut states: Vec<(crate::kern::Acc, Pieces)> = vec![(crate::kern::Acc::new(), Pieces::new(&of_day, crate::wf::NPIECE)); accs.len()];
        let lists: Vec<u32> = {
            let mut v: Vec<u32> = accs.iter().map(|a| a.0).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        // v3.3 C3: the flagged lists derived once (round B: read from ul_evs); named test v33_c3_flagged_lists_derived
        let own: HashMap<u32, Vec<u32>> = sr.flag_lists33(&lists);
        let keys: HashMap<u32, Vec<u32>> = lists.iter().map(|&u| (u, if sr.ul_derived { own.get(&u) } else { sr.ul_evs.get(u as usize) }.map_or_else(Vec::new, |l| list_keys(&sr.evs, l)))).collect();
        let days: Vec<usize> = sr.days.iter().copied().filter(|&d| usize::from(of_day.get(d).copied().unwrap_or(0)) >= start).collect();
        for di in days {
            let lo = u32::try_from(di).unwrap_or(0) * 86_400;
            let hi = lo + 86_400;
            let mut want: Vec<usize> = Vec::new();
            for &u in &lists {
                let (Some(list), Some(ks)) = (if sr.ul_derived { own.get(&u) } else { sr.ul_evs.get(u as usize) }, keys.get(&u)) else { continue };
                let (a, b) = (ks.partition_point(|&x| x < lo), ks.partition_point(|&x| x < hi));
                want.extend(list.get(a..b).unwrap_or(&[]).iter().map(|&e| e as usize));
            }
            want.sort_unstable();
            want.dedup();
            if want.is_empty() {
                continue;
            }
            store.ensure(&[di])?;
            sr.tab = Tab3::default();
            stage_a_events_c(sr, Some(&want), &store.cubes, &store.tapes, var, dial, ma, seg, compat, threads, true)?;
            store.release(&[]);
            let srr = &*sr;
            let next = AtomicUsize::new(0);
            let cells: Vec<Mutex<&mut (crate::kern::Acc, Pieces)>> = states.iter_mut().map(Mutex::new).collect();
            let of_day = &of_day;
            std::thread::scope(|s| {
                for _ in 0..threads {
                    s.spawn(|| loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        let (Some(&(u, si, l, ni)), Some(cell)) = (accs.get(k), cells.get(k)) else { break };
                        let (Some(list), Some(ks), Some(cs)) = (if srr.ul_derived { own.get(&u) } else { srr.ul_evs.get(u as usize) }, keys.get(&u), csets.get(si as usize)) else { continue };
                        let (a, b) = (ks.partition_point(|&x| x < lo), ks.partition_point(|&x| x < hi));
                        if a >= b {
                            continue;
                        }
                        let tf = srr.uls.get(u as usize).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) });
                        if let Ok(mut g) = cell.lock() {
                            let st = &mut **g;
                            let _ = crate::kern::walk_day(&srr.evs, &srr.ex, &srr.tab, list.get(a..b).unwrap_or(&[]), ks.get(a..b).unwrap_or(&[]), cs, usize::from(l), usize::from(ni), tf, true, true, &mut st.0, &mut st.1, of_day);
                        }
                    });
                }
            });
        }
        for &k in &fl {
            let Some(f) = sr.flagged.get(k).copied() else { continue };
            let Some(&ai) = ix.get(&(f.list, f.si, f.l, f.ni)) else { continue };
            let (Some(st), Some(r)) = (states.get(ai), rows.get(usize::from(f.row))) else { continue };
            if let Some(w) = sr.win.get_mut(usize::from(f.row)) {
                // REPAIR A (WF19): the last session, week and month of the fresh walk closed (on a copy) before its pieces are composed
                let (mut a2, mut p2) = (st.0.clone(), st.1.clone());
                crate::kern::close_day_p(&mut a2, &mut p2);
                crate::kern::finish_pieces(&a2, &mut p2);
                offer_row(w, r, &p2.p, f.sm);
                w.rewalked += 1;
                w.pending = w.pending.saturating_sub(1);
                rewalked += 1;
            }
        }
    }
    sr.rewalks = rewalked;
    lg!(log, "{}: D3 exact re-walk pass: {rewalked} flagged account-rows re-walked as fresh accounts from their row's first learning piece in {:.1} s; PENDING left: {}", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), t0.elapsed().as_secs_f64(), sr.win.iter().map(|w| w.pending).sum::<u64>());
    Ok(())
}

/// The picks every detail reads: (name, group, view). THE pick from SEARCH settings only (R1); with the v3.1 headline switch, THE pick is
/// the HEADLINE group's (decisions (a)-(c)) and the R1 SEARCH pick is printed beside.
#[must_use]
pub fn wanted_picks(ngroup: usize) -> Vec<(&'static str, usize, usize)> {
    let mut v: Vec<(&'static str, usize, usize)> = Vec::new();
    if ngroup > NGROUP {
        v.push(("HEADLINE-S", NGROUP, 8));
        v.push(("HEADLINE-A", NGROUP, 0));
    }
    v.extend([("THE-PICK-S-SEARCH", 6, 8), ("A-SEARCH", 6, 0), ("S-COMPARISON", 7, 8), ("S0-BASELINE-S", 5, 8), ("S0-BASELINE-A", 5, 0)]);
    // v3.2 (check M2 / S3): the X view detailed trade by trade (headline and SEARCH); addendum 15: the verdict view
    if sw::on(sw::PICKS32) {
        if ngroup > NGROUP {
            v.push(("HEADLINE-X", NGROUP, 1));
        }
        v.push(("X-SEARCH", 6, 1));
    }
    if sw::on(sw::SEARCH15) {
        if ngroup > NGROUP {
            v.push(("HEADLINE-V", NGROUP, crate::acct::VIEW_V));
        }
        v.push(("V-SEARCH", 6, crate::acct::VIEW_V));
    }
    v
}

/// v3.2 (check M2): the 10% share twin of a pick's account: the same list, setting and N at share level 0 (None when the pick already
/// uses the 10% share).
#[must_use]
pub fn twin10(sm: &Sum) -> Option<Sum> {
    let (si, l) = (usize::from(sm.x) / NL, usize::from(sm.x) % NL);
    if l == 0 {
        return None;
    }
    // MUTANT m_v32_twin_not_10: the twin keeps the pick's own share
    let l2 = if cfg!(feature = "m_v32_twin_not_10") { l } else { 0 };
    Some(Sum { x: u16::try_from(si * NL + l2).unwrap_or(0), ..*sm })
}

/// D1 (check-build-v3): after the day-major sweep, the stage A table of every event a detail reads (the picks' lists) is REBUILT in one
/// streamed pass (the sweep kept only the last day's table); the counters are not touched; every other event's row offsets are dropped so
/// no detail can read a dropped table (their fill counts, the liquidity table's input, stay).
#[allow(clippy::too_many_arguments)]
pub(crate) fn d1_rebuild(sr: &mut Seg3, store: &mut Store, var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, ma: &MenuA, seg: u8, compat: bool, threads: usize, hand: bool, log: &mut String) -> Result<(), String> {
    let t0 = Instant::now();
    let mut lists: Vec<u32> = Vec::new();
    for (_, g, v) in wanted_picks(sr.ngroup) {
        if let Some(sm) = sr.tops.get(g).and_then(|t| t.best(v).first().copied()) {
            lists.push(sm.list);
        }
    }
    // v3.2 addendum 15: the verdict view's best 20 and the Table W picks (the closest misses, the portfolio layer and the test-month
    // verdicts read their tables)
    if sw::on(sw::SEARCH15) {
        for g in [6, NGROUP] {
            if let Some(t) = sr.tops.get(g) {
                lists.extend(t.best(crate::acct::VIEW_V).iter().take(20).map(|s| s.list));
            }
        }
        lists.extend(sr.win.iter().filter_map(|w| w.best.as_ref().map(|b| b.1.list)));
        // v3.2 round B (addenda 17 + 18): every split's way B pick (the walk-forward side-by-side accounts read its trades)
        if sw::on(sw::SPLITS18) && sw::on(sw::SIDE17) {
            lists.extend(sr.sp18.best.iter().filter_map(|v| v.first().copied().flatten()).map(|b| b.sm.list));
        }
    }
    // v3.3 fix CAP_MIN (check-engine-1 M1r): the first K accounts of every view a pick or the verdict view reads (each is checked minute by
    // minute after this rebuild; the next checked account replaces a failing one), and every split's way B pick (its cap line)
    if crate::fix33::on(crate::fix33::CAP_MIN) {
        let k = crate::fix33::CAP_K.load(Ordering::Relaxed);
        for (g, v) in cap_views33(sr.ngroup) {
            if let Some(t) = sr.tops.get(g) {
                lists.extend(t.best(v).iter().take(k).map(|s| s.list));
            }
        }
        if sw::on(sw::SPLITS18) {
            lists.extend(sr.sp18.best.iter().filter_map(|v| v.first().copied().flatten()).map(|b| b.sm.list));
        }
    }
    lists.sort_unstable();
    lists.dedup();
    let mut want: Vec<usize> = lists.iter().flat_map(|&u| sr.ul(u as usize).map_or_else(Vec::new, |l| l.iter().map(|&e| e as usize).collect::<Vec<_>>())).collect();
    want.sort_unstable();
    want.dedup();
    let cnt = std::mem::take(&mut sr.cnt);
    sr.tab = Tab3::default();
    if !cfg!(feature = "m_v31_dm_stale_tab") {
        for &di in &sr.days.clone() {
            let w: Vec<usize> = want.iter().copied().filter(|&i| sr.evs.get(i).is_some_and(|e| usize::from(e.di) == di)).collect();
            if w.is_empty() {
                continue;
            }
            store.ensure(&[di])?;
            stage_a_events_c(sr, Some(&w), &store.cubes, &store.tapes, var, dial, ma, seg, compat, threads, false)?;
            store.release(&[]);
        }
        let mut inw = vec![false; sr.evs.len()];
        for &i in &want {
            if let Some(x) = inw.get_mut(i) {
                *x = true;
            }
        }
        for (x, &k) in sr.ex.iter_mut().zip(&inw) {
            if !k {
                x.nq = 0;
                x.r0 = 0;
                x.t0 = 0;
                x.mark1 = 0;
            }
        }
    }
    sr.cnt = cnt;
    lg!(log, "{}: D1 stage A rebuilt for the {} picks' lists ({} events{}) in {:.1} s; every other event's rows dropped", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), lists.len(), want.len(), if hand { "; the hand sample reads these events only" } else { "" }, t0.elapsed().as_secs_f64());
    Ok(())
}

/// The day-major proof on the run's own data: the whole mode's tops (already in `sr.tops`) against a day-major run's, view by view, account
/// by account (every listed account's figures equal).
#[allow(clippy::too_many_arguments)]
fn compare_day_major(sr: &mut Seg3, store: &mut Store, var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, ma: &MenuA, sets: &Sets, sel: &[usize], csets: &[CSet], seg: u8, compat: bool, skip: bool, threads: usize, topk: usize, batch_acc: u64, log: &mut String) -> Result<(), String> {
    let whole = std::mem::take(&mut sr.tops);
    let (ws, wa) = (sr.steps, sr.accounts);
    let (tab, ex) = (std::mem::take(&mut sr.tab), sr.ex.clone());
    let (cnt, evs_tab) = (sr.cnt.clone(), sr.evs.iter().map(|e| e.tab.clone()).collect::<Vec<_>>());
    stage_b_day_major(sr, store, var, dial, ma, sets, sel, csets, seg, compat, skip, threads, topk, batch_acc.min(5_000_000), log)?;
    let mut diff = 0_usize;
    let mut n = 0_usize;
    for (a, b) in whole.iter().zip(&sr.tops) {
        for v in 0..NVIEW {
            let (x, y) = (a.best(v), b.best(v));
            n += x.len();
            if x.len() != y.len() {
                diff += 1;
                continue;
            }
            for (p, q) in x.iter().zip(&y) {
                if (p.list, p.x, p.n, p.net, p.trades, p.wins, p.mdd, p.worst, p.worst_day, p.pk_margin) != (q.list, q.x, q.n, q.net, q.trades, q.wins, q.mdd, q.worst, q.worst_day, q.pk_margin) {
                    diff += 1;
                }
            }
        }
    }
    lg!(log, "DAY-MAJOR-EQ: whole mode vs day-major (batches of at most 5,000,000 accounts, stage A per day per batch): {n} listed accounts compared over {} groups x {NVIEW} views: {diff} differ; steps whole {ws} / day-major {}; accounts {wa} / {}", whole.len(), sr.steps, sr.accounts);
    sr.tops = whole;
    sr.tab = tab;
    sr.ex = ex;
    sr.cnt = cnt;
    for (e, t) in sr.evs.iter_mut().zip(evs_tab) {
        e.tab = t;
    }
    sr.steps = ws;
    sr.accounts = wa;
    Ok(())
}

/// The words of a setting slot index (its id, role, set).
fn set_words(sets: &Sets, sel: &[usize], x: u16) -> (String, bool, u8, usize) {
    let si = usize::from(x) / NL;
    let l = usize::from(x) % NL;
    sel.get(si).and_then(|&i| sets.settings.get(i)).map_or_else(|| ("?".to_owned(), false, 0, l), |s| (s.id.clone(), s.search, s.set, l))
}

/// The exit setting in words (design 5.6: "SL 0.25 x 1-min option ATR; trailing 10% after a 20% decay; time to 15:10").
fn setting_words(menu: &Menu, sets: &Sets, sel: &[usize], x: u16) -> String {
    let si = usize::from(x) / NL;
    let Some(s) = sel.get(si).and_then(|&i| sets.settings.get(i)) else { return "?".to_owned() };
    let mut v: Vec<String> = s.rules.iter().zip(&s.labels).filter_map(|(&r, &lab)| menu.rules.get(usize::from(r)).map(|m| if lab == 4 && FAMILIES.get(usize::from(m.fam)) == Some(&"TSL") { format!("{} (as TTP)", m.words) } else { m.words.clone() })).collect();
    if !s.rules.iter().any(|&r| menu.rules.get(usize::from(r)).is_some_and(|m| m.kind == Kind::Time)) {
        v.push("time to the 15:10 square-off".to_owned());
    }
    v.join("; ")
}

/// One pick's trades recomputed from scratch (proof P5): the very same account steps (`detail3`), each booked trade re-priced by
/// `trade3_once` (THE LAW's triggers, its own sale record at its own quantity), the sweep's net beside.
struct PickRow {
    line: String,
    net: i64,
    sweep_minus_detail: i64,
    label: String,
    own_in_cut_window: bool,
    search_sl_present: bool,
    /// v3.1: the trade's money and measurements (the jump stress, the adverse moves, the never-closed units).
    x: RowX,
    /// v3.2: the trade's v3.2 facts (None with every v3.2 switch off).
    r32: Option<crate::v32::Row32>,
}

/// v3.1: one trade's money as the jump stress re-prices it, its known instant (the drawdown order), its never-closed units, the adverse moves
/// in the first 2 / 5 / 10 s after the sale decision and after the exit decision (bp), and its net at the gate stress.
#[derive(Clone, Copy, Debug, Default)]
struct RowX {
    known: i64,
    money: crate::stress::Money,
    nc_units: u64,
    adv_sale: [i64; 3],
    adv_exit: [i64; 3],
    net_s2: i64,
    /// Matrix X03: an SL exit's first buy-back block price beyond its trigger level, bp of the sale's first price (None: not an SL exit
    /// with a buy-back fill).
    overshoot_bp: Option<i64>,
    /// Matrix X05 / X06: units an own exit's order did not buy back (the square-off or the valuation took them).
    exit_unfilled: u64,
    own_exit: bool,
    sl_exit: bool,
    /// Matrix X07 / X17: the trade's loss as bp of its slot (0 for a win).
    loss_slot_bp: i64,
    /// Matrix X11: the option traded on its own expiry day.
    expiry_day: bool,
    /// Matrix X25 / addendum 5: the highest price from the first sale fill to the last fill known, bp above the sale's average price.
    respike_bp: i64,
    /// Addendum 7: the charges per side and the buy side's own orders.
    charges_sell: i64,
    charges_buy: i64,
}

/// One booked decision's trade row (proof P5): the setting's rules at the event from the readings, the trade recomputed by THE LAW
/// (`trade3_once`), the sweep's net beside; v3.1 columns appended when a v3.1 switch is on.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn trade_row(pick: &str, pos: usize, seg: usize, sr: &Seg3, sm: &Sum, d: &crate::kern::Dec3, c: &Cube, tp: &SecTape, dial: &common_fill::Dial, menu: &Menu, sets: &Sets, sel: &[usize], csets: &[CSet], compat: bool, cal: &crate::rules::Calendar) -> Result<Option<PickRow>, String> {
    let u = sm.list as usize;
    let si = usize::from(sm.x) / NL;
    let l = usize::from(sm.x) % NL;
    let ni = usize::from(sm.n);
    let cs = csets.get(si).ok_or("setting")?;
    let set = sel.get(si).and_then(|&i| sets.settings.get(i)).ok_or("setting")?;
    let tf = sr.uls.get(u).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) });
    let blk = block_of(u8::try_from(seg).unwrap_or(0));
    let Some(e) = sr.evs.get(d.eid as usize) else { return Ok(None) };
    let Some(x) = sr.ex.get(d.eid as usize) else { return Ok(None) };
    let Some(kc) = c.cons.get(e.ci as usize) else { return Ok(None) };
    let dt = DayTape::new(c.day, tp.of(e.ci as usize), blk)?;
    let tape = dt.tape()?;
    let lot = u64::from(kc.lot);
    let qty = u64::from(d.lots) * lot;
    let p0 = e.tab.p0.get(l).copied().unwrap_or(0);
    // the setting's rules at this event from the readings (the window slot of the list's timeframe), in tie order
    let base = sr.rd.get(d.eid as usize).copied().unwrap_or_default();
    let w = tf.and_then(|t| sr.tfw.get(d.eid as usize).and_then(|v| v.iter().find(|z| z.0 == t)).copied());
    let rd = w.map_or(base, |(_, h, r)| Rd { hwin: h, rise: r, ..base });
    let rules: Vec<Result<(common_fill::xrule::XRule, bool), u8>> = set.rules.iter().map(|&r| menu.rules.get(usize::from(r)).map_or(Err(u8::MAX), |m| rule_at(m, &rd, p0))).collect();
    // v3.2 addendum 10: the recomputed trade counts the same executed orders as stage A (the contract-day's freeze slice)
    let slice = if sw::on(sw::ORDERS_A10) { crate::freeze::slice_units(c, e.ci as usize) } else { 0 };
    let Some((r, wpos, anc, key, tsec, level)) = (if slice > 0 || sw::on(sw::ORDERS_A10) { trade3_once_s(&tape, c.day, e.t, lot, qty, l, &rules, compat, dial, slice)? } else { trade3_once(&tape, c.day, e.t, lot, qty, l, &rules, compat, dial)? }) else { return Ok(None) };
    let o = crate::kern::out_of_decision(e, x, &sr.tab, cs, l, ni, tf).unwrap_or_default();
    let label = match (wpos, key) {
        (_, XKey::None) => if anc < NEVER3 { "squareoff (anchor at or after 15:10)".to_owned() } else { "squareoff 15:10 (no rule fired)".to_owned() },
        (Some(p), XKey::Held(_)) => format!("HELD {} -> squareoff (B2 cancel)", set.labels.get(p).and_then(|&lb| FAMILIES.get(usize::from(lb))).copied().unwrap_or("?")),
        (Some(p), XKey::Own(..)) => FAMILIES.get(usize::from(set.labels.get(p).copied().unwrap_or(0))).copied().unwrap_or("?").to_owned(),
        (None, _) => "?".to_owned(),
    };
    let own_in_cut = matches!(key, XKey::Own(s, _) if s >= crate::cal::CUT_S);
    let sl_present = rules.get(cs.sl).is_some_and(Result::is_ok);
    let dist: Vec<String> = rules.iter().zip(&set.rules).map(|(rr, &mr)| { let id = menu.rules.get(usize::from(mr)).map_or("?", |m| m.id.as_str()); match rr { Ok((xr, fl)) => format!("{id}={xr:?}{}", if *fl { " (1-paisa floor)" } else { "" }), Err(c2) => format!("{id}=ABSENT({})", absent_word(*c2)) } }).collect();
    let (lq_c, lq_g) = sr.liq.get(d.eid as usize).copied().unwrap_or((0, 0));
    let flags = format!("{}{}{}", if o.flag & crate::stage::FLAG_CUT_HELD != 0 { "CUT_HELD+REWALKED " } else { "" }, if r.unclosed == 0 { "" } else if sw::on(sw::WHY12) { "NEVER_CLOSED(valued, not filled: exit decision to 15:30) " } else { "UNCLOSABLE(valued S15) " }, if r.limit_out > 0 && r.bb_past_limit > 0 { "PROTECTED " } else { "" });
    let mut cols = vec![
        pick.to_owned(),
        (pos + 1).to_string(),
        SEG_SHORT.get(seg).copied().unwrap_or("?").to_owned(),
        c.unds.get(usize::from(kc.und)).map_or("?".to_owned(), |x| x.name.clone()),
        kc.symbol.clone(),
        kc.expiry.to_string(),
        fmt_day(c.day),
        fmt_sod(i64::from(e.t)),
        set.id.clone(),
        (SHARE_BP.get(l).copied().unwrap_or(0) / 100).to_string(),
        crate::acct::n_of_s(seg, ni).to_string(),
        d.lots.to_string(),
        fmt_rs(p0),
        r.units.to_string(),
        fmt_rs(r.sold_value),
        label.clone(),
        if tsec > 0 { fmt_sod(tsec) } else { "-".to_owned() },
        if anc < NEVER3 { fmt_sod(i64::from(anc) + trade::BASE_S) } else { "-".to_owned() },
        fmt_rs(level),
        dist.join(" | "),
        fmt_us(r.bb_first),
        fmt_rs(r.bb_first_px),
        r.bb_units.to_string(),
        fmt_rs(r.bb_value),
        r.sq_units.to_string(),
        fmt_rs(r.sq_value),
        r.unclosed.to_string(),
        fmt_rs(r.charges),
        fmt_rs(r.net),
        fmt_us(r.known),
        flags.trim().to_owned(),
        format!("{lq_c}/{lq_g}"),
        (i64::from(o.net) - r.net).to_string(),
        cal.expiry_words(crate::cal::day_of_ymd(kc.expiry)),
    ];
    // v3.1: the money the stress re-prices, the adverse moves (the sale from T; the exit from its decision: an own exit's anchor, else 15:10)
    let (own_orders, broker) = trade::buy_orders(r.bb_units, r.sq_units, r.unclosed);
    let money = crate::stress::Money { day: c.day, units: r.units, sold: r.sold_value, bought: r.buy_value, own_orders, broker, orders: r.orders_a10() };
    let t_us = crate::cal::at_sod(c.day, i64::from(e.t));
    let x_us = match key {
        XKey::Own(s, _) => crate::cal::at_sod(c.day, s),
        _ => crate::cal::at_sod(c.day, crate::cal::SQ_S),
    };
    let mut rx = RowX { known: r.known, money, nc_units: r.unclosed, charges_sell: r.charges_sell, charges_buy: r.charges_buy, ..RowX::default() };
    rx.own_exit = matches!(key, XKey::Own(..));
    rx.sl_exit = rx.own_exit && label == "SL";
    if rx.own_exit {
        rx.exit_unfilled = r.units.saturating_sub(r.bb_units);
    }
    if rx.sl_exit && r.bb_units > 0 && p0 > 0 {
        rx.overshoot_bp = Some((r.bb_first_px - level) * 10_000 / p0);
    }
    let slot = trade::ACCOUNT / crate::acct::n_of_s(seg, ni).max(1);
    rx.loss_slot_bp = if r.net < 0 { (-r.net) * 10_000 / slot.max(1) } else { 0 };
    rx.expiry_day = kc.expiry == c.ymd;
    if r.units > 0 && r.e0 > 0 {
        let avg = r.sold_value / i64::try_from(r.units).unwrap_or(1).max(1);
        if let Some((hi, _)) = tape.extreme(r.e0, r.known.max(r.e0 + 1), true)
            && avg > 0
        {
            rx.respike_bp = ((hi - avg) * 10_000 / avg).max(0);
        }
    }
    if sw::any() {
        for (k, &s) in crate::stress::ADV_S.iter().enumerate() {
            if let (Some(a), Some(b)) = (rx.adv_sale.get_mut(k), rx.adv_exit.get_mut(k)) {
                *a = crate::stress::adverse_bp(&tape, t_us, s, common_fill::Side::Sell);
                *b = crate::stress::adverse_bp(&tape, x_us, s, common_fill::Side::Buy);
            }
        }
        rx.net_s2 = crate::stress::stressed(&money, sw::GATE_BP.load(Ordering::Relaxed), true)?;
        let mxw = e.tab.mx.map_or_else(|| "proxy".to_owned(), |m| {
            let lab = sr.mlabels.get(usize::from(e.di)).and_then(|v| v.get(usize::from(m.snap))).cloned().unwrap_or_else(|| "?".to_owned());
            format!("{}{}{}", lab, if m.fallback { " (FALLBACK: no table that day)" } else { "" }, if m.stale { " STALE(spot moved > 1/3 of the price scan)" } else { "" })
        });
        cols.extend([
            fmt_rs(e.tab.margin_lot),
            fmt_rs(e.tab.margin_lot_proxy),
            format!("{}", if e.tab.addon > 0 { "Rs 20 per order (rule v2)" } else { "-" }),
            match money.orders {
                Some((so, bo, ko)) => format!("sale {} / buy {} (executed orders, every filling block x every freeze slice: {so} sale, {bo} own buy, {ko} Dhan square-off)", fmt_rs(r.charges_sell), fmt_rs(r.charges_buy)),
                None => format!("sale {} / buy {} (orders: 1 sale, {} own buy{})", fmt_rs(r.charges_sell), fmt_rs(r.charges_buy), money.own_orders, if money.broker { ", Dhan square-off fee" } else { "" }),
            },
            rx.overshoot_bp.map_or_else(|| "-".to_owned(), |b| format!("{}.{:02}%", b / 100, (b % 100).abs())),
            format!("{}.{:02}%", rx.respike_bp / 100, rx.respike_bp % 100),
            mxw,
            if r.unclosed > 0 { format!("{} units valued, not filled{}", r.unclosed, if r.val_fallback { " (no print after the exit decision: entry window)" } else { "" }) } else { "-".to_owned() },
            if r.fill_after_1530_real { "yes".to_owned() } else { "no".to_owned() },
            fmt_rs(rx.net_s2),
            rx.adv_sale.iter().map(i64::to_string).collect::<Vec<_>>().join("/"),
            rx.adv_exit.iter().map(i64::to_string).collect::<Vec<_>>().join("/"),
        ]);
    }
    // v3.2: every trade's v3.2 facts and columns (addenda 8 / 8b, 10, 12 / 12b / 13 / 13b, 14 (d), 16)
    let r32 = if sw::any32() {
        let (row32, extra) = row32_of(seg, sr, sm, d, e, c, kc, &r, wpos, anc, key, tsec, level, &label, &dist, set, menu, x_us, t_us, blk, slice, qty, &money, wpos.and_then(|p| rules.get(p)).and_then(|z| z.as_ref().ok()).map(|z| z.0))?;
        cols.extend(extra);
        Some(row32)
    } else {
        None
    };
    Ok(Some(PickRow { line: cols.join("\t"), net: r.net, sweep_minus_detail: i64::from(o.net) - r.net, label, own_in_cut_window: own_in_cut, search_sl_present: sl_present, x: rx, r32 }))
}

/// v3.2: the columns appended to every trade row when a v3.2 switch is on.
const TRADES_HEAD_V32: &str = "\torders_sale/own_buy/dhan_sq (every filling block x every freeze slice)\tfreeze_slice_units\tENTRY_WHY\tEXIT_WHY\tgain_inside_signal_window (what the robot used)\tgain_vs_previous_close_DHAN (bhavcopy LastPric of the previous session)\tgain_vs_previous_close_NSE_official (bhavcopy ClsPric)\tgain_vs_today_open\tentry_time_to_fill_s\texit_time_to_fill_s\tpct_of_wanted_units_sold\tprints_last_60s/longest_silence_s\texit_resends\tunfilled_exit_units\texpiry\tweekday\tweekly/monthly (source)\tsessions_to_expiry (bucket)\tmoved_by_holiday\texposure_rate_used\tgreeks_entry (bar, spot, premium, tenor s; IV, delta, gamma per Rs, position theta Rs/day, position vega Rs/vol pt, position rho)\tgreeks_exit\tgreeks_rate (T-bill release, age, stale)";
/// REPAIR A (check-engine-1 S8 / S4): the v3.2 columns with the time-to-fill definitions named and the rounding stated.
const TRADES_HEAD_V32_RA: &str = "\torders_sale/own_buy/dhan_sq (every filling block x every freeze slice)\tfreeze_slice_units\tENTRY_WHY\tEXIT_WHY\tgain_inside_signal_window (what the robot used; % rounded half up)\tgain_vs_previous_close_DHAN (bhavcopy LastPric of the previous session; % rounded half up)\tgain_vs_previous_close_NSE_official (bhavcopy ClsPric; % rounded half up)\tgain_vs_today_open (% rounded half up)\tentry_time_to_fill_s (from the decision T to the last sale fill known)\texit_time_to_fill_s (from the exit ORDER instant: the anchor when an own order went out, even one that filled nothing, else 15:10:00; to the last buy-back / square-off fill known)\tpct_of_wanted_units_sold (rounded half up)\tprints_last_60s/longest_silence_s\texit_resends\tunfilled_exit_units\texpiry\tweekday\tweekly/monthly (source)\tsessions_to_expiry (bucket)\tmoved_by_holiday\texposure_rate_used\tgreeks_entry (the app's own Greeks code on the 1-minute inputs, proven equal to the live app: bar, spot, premium, tenor s; IV, delta, gamma per Rs, position theta Rs/day, position vega Rs/vol pt, position rho)\tgreeks_exit\tgreeks_rate (T-bill release, age, stale)";

/// v3.2: one trade's v3.2 facts and its appended columns.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn row32_of(seg: usize, sr: &Seg3, sm: &Sum, d: &crate::kern::Dec3, e: &SEv, c: &Cube, kc: &crate::cube::Contract, r: &trade::Trip, wpos: Option<usize>, anc: u16, key: XKey, tsec: i64, level: i64, label: &str, dist: &[String], set: &crate::menu::Setting, menu: &Menu, x_us: i64, t_us: i64, blk: i64, slice: u64, qty: u64, money: &crate::stress::Money, xr: Option<common_fill::xrule::XRule>) -> Result<(crate::v32::Row32, Vec<String>), String> {
    let mut x = crate::v32::Row32 { day: c.day, t_us, known: r.known, net: r.net, margin: i64::from(d.lots) * e.tab.margin_lot + e.tab.addon, slice, ..crate::v32::Row32::default() };
    // REPAIR A (check-engine-2 M1 / S7): the trade's marks for the per-minute margin + MTM reading (the kernel's own sale units and value)
    x.seg = u8::try_from(seg).unwrap_or(0);
    x.units = r.units;
    x.sold = r.sold_value;
    x.mark1 = sr.ex.get(d.eid as usize).map_or(0, |z| z.mark1);
    x.expiry_day = kc.expiry == c.ymd;
    x.charges = r.charges;
    x.liq = sr.uls.get(sm.list as usize).map_or(0, |z| z.1);
    if sw::on(sw::CHG_DATED) {
        let o = money.orders.unwrap_or((1, money.own_orders, u32::from(money.broker)));
        x.charges_dearer = trade::charges_dearer_of(c.day, r.units, r.buy_value, r.sold_value, o);
    }
    if let Some(o) = money.orders {
        x.orders = o;
        let n = i64::from(o.0) + i64::from(o.1) + i64::from(o.2);
        x.brokerage_gst = n * 2_000 * 118 / 100;
    }
    let und = c.unds.get(usize::from(kc.und)).map_or("?", |u| u.name.as_str());
    // addendum 14 (d)
    let blk_us = blk * crate::cal::US_S;
    x.entry_ttf = if r.units > 0 { (r.sale_known - t_us).max(0) / crate::cal::US_S } else { 0 };
    let last_buy = r.bb_known.max(r.sq_known);
    x.exit_ttf = if r.units > 0 && last_buy > 0 { Some((last_buy - x_us).max(0) / crate::cal::US_S) } else { None };
    x.filled_bp = if qty == 0 { 0 } else if sw::on(sw::WHY12) { crate::v32::bp_round(i64::try_from(r.units).unwrap_or(0), i64::try_from(qty).unwrap_or(1)) } else { i64::try_from(r.units * 10_000 / qty).unwrap_or(0) };
    let (lq_c, lq_g) = sr.liq.get(d.eid as usize).copied().unwrap_or((0, 0));
    x.prints60 = lq_c;
    x.gap60 = lq_g;
    x.resends = if matches!(key, XKey::Own(..)) && r.bb_known > 0 && blk_us > 0 { u32::try_from(((r.bb_known - x_us).max(0) + blk_us - 1) / blk_us).unwrap_or(0).saturating_sub(1) } else { 0 };
    x.nc_units = r.unclosed;
    x.sl_exit = label == "SL";
    // addendum 16
    let (ed, monthly, msrc, wd, dte, moved) = crate::expiry::facts(c, e.ci as usize).unwrap_or((crate::cal::day_of_ymd(kc.expiry), false, "-", "?", 0, false));
    x.dte = dte;
    x.bucket = crate::expiry::bucket(dte);
    x.monthly = monthly;
    x.moved = moved;
    // the gains (addenda 13 / 13b): the signal window, the previous session's official close (both bhavcopy numbers), today's open
    let u = sm.list as usize;
    let rep = sr.uls.get(u).map_or(0, |z| z.0) as usize;
    let rk = crate::screen::rank_at(c, u8::try_from(seg).unwrap_or(0), e.ci, rep, sr.lv_at(usize::from(e.di)), e.t);
    let (tf_i, _, tn, _, _, thr) = screen::cfg_parts(rep);
    let ct = rk.as_ref().map_or_else(|| i64::from(c.close_at(e.ci as usize, usize::try_from((i64::from(e.t) - crate::cal::OPEN_S) / 60 - 1).unwrap_or(0))), |z| i64::from(z.3.ct));
    // REPAIR A (check-engine-1 S4): percentages rounded half up under WHY12 (truncated in h6)
    let gw = |a: i64, b: i64, s: &str| if sw::on(sw::WHY12) { crate::v32::gain_words_r(a, b, s) } else { crate::v32::gain_words(a, b, s) };
    let g_win = rk.as_ref().map_or_else(|| "- (not a member of its list at T under the representative config)".to_owned(), |z| gw(i64::from(z.3.c0), i64::from(z.3.ct), &format!("{} window ending {}{}", screen::tf_name(tf_i), fmt_sod(i64::from(e.t)), if z.3.first_open_ref { "; start = the contract's first open of the day" } else { "" })));
    let pc = crate::bhav::prev_close(c.day, und, kc.expiry, kc.strike, kc.right);
    let (g_dhan, g_nse) = match &pc {
        Some((pd, cls, last, file)) => (if *last > 0 { gw(*last, ct, &format!("LastPric of {} in {file}", fmt_day(*pd))) } else { format!("- (no LastPric in {file}: old format)") }, gw(*cls, ct, &format!("ClsPric of {} in {file}", fmt_day(*pd)))),
        None => ("- (no bhavcopy row for the contract in the previous session held)".to_owned(), "- (no bhavcopy row)".to_owned()),
    };
    let g_open = gw(i64::from(kc.first_open), ct, &format!("first traded price of the day, the 1-minute bar at {}", fmt_sod(crate::cal::OPEN_S + 60 * i64::from(kc.first_min.max(0)))));
    // the entry why (addendum 12b): signal, list, rank, the gain that fired against the threshold, liquidity, sizing, first sale block
    let trig = if thr < 9 { format!("trigger 'gain inside the signal window >= {}%'", screen::THR.get(thr).copied().unwrap_or(0)) } else { format!("trigger 'rank #1..{} by gain' (NO minimum % applies)", screen::RANK.get(thr - 9).copied().unwrap_or(0)) };
    let slot = trade::ACCOUNT / crate::acct::n_of_s(usize::from(sm.seg), usize::from(sm.n)).max(1);
    let entry_why = format!("signal at {} (end of the {} candle; decision only at candle ends, order from the next block, REQ 56) on the {} list of expiry {} ({}); {}; rank #{} by gain and #{} by volume among the top {} by volume ({} members); {} vs this pick's {}; liquidity: {} traded seconds of the last 60, longest silence {} s, last-60-s volume {}{}; sized: slot Rs {} (N {}), margin Rs {} a lot -> {} lots ({} units); first sale block {} at Rs {}", fmt_sod(i64::from(e.t)), screen::tf_name(tf_i), SEG_SHORT.get(seg).copied().unwrap_or("?"), fmt_day(ed), crate::report::cfg_words(seg, rep, &sr.lv), trig, rk.as_ref().map_or("?".to_owned(), |z| z.0.to_string()), rk.as_ref().map_or("?".to_owned(), |z| z.1.to_string()), screen::topn().get(tn).copied().unwrap_or(0), rk.as_ref().map_or("?".to_owned(), |z| z.2.to_string()), g_win, trig, lq_c, lq_g, e.tab.vol60, if sw::on(sw::LIQ14) { format!(" (covers the whole order within {} s and its buy-back within {} s at 25%: passed)", trade::LIQ_COVER_S.load(Ordering::Relaxed), trade::LIQ_EXIT_S.load(Ordering::Relaxed)) } else { String::new() }, fmt_rs(slot), crate::acct::n_of_s(usize::from(sm.seg), usize::from(sm.n)), fmt_rs(e.tab.margin_lot), d.lots, qty, fmt_us(r.e0), fmt_rs(r.p0));
    // the exit why (addendum 12): the rule, its level and how it was set, the second, the buy-back, re-sends, unfilled units
    let rule_words = wpos.and_then(|p| set.rules.get(p)).and_then(|&mr| menu.rules.get(usize::from(mr))).map_or_else(|| "-".to_owned(), |m| m.words.clone());
    let rule_dist = wpos.and_then(|p| dist.get(p)).cloned().unwrap_or_default();
    let fam = match label {
        "SL" => "stop-loss",
        "TT" => "trailing target trigger",
        "BE" => "breakeven stop",
        "TSL" => "trailing stop",
        "TTP" => "trailing target",
        "TP" => "target",
        "GO" => "go-positive time stop",
        "TIME" => "time limit",
        _ => "",
    };
    let bought = if r.bb_units > 0 { format!("the buy-back order went out at {} and bought {} units from Rs {} (first block {}) in {} blocks, {} re-sends, the last fill known {}", fmt_sod(crate::cal::sod_of(x_us)), r.bb_units, fmt_rs(r.bb_first_px), fmt_us(r.bb_first), r.bb_blocks, x.resends, fmt_us(r.bb_known)) } else if matches!(key, XKey::Own(..)) { "the buy-back order (re-sent every block) bought NOTHING before 15:10".to_owned() } else { String::new() };
    let sq = if r.sq_units > 0 { format!("our own 15:10 square-off bought {} units (Rs {}), the last fill known {}", r.sq_units, fmt_rs(r.sq_value), fmt_us(r.sq_known)) } else { String::new() };
    let nc = if r.unclosed > 0 { format!("{} units NOT bought back by 15:30: valued at the highest HIGH Rs {} between the exit decision and 15:30 (valued, not filled)", r.unclosed, fmt_rs(r.worst)) } else { "0 units unfilled".to_owned() };
    // REPAIR A (check-engine-1 M3): the rule word from the winning rule's family (a HELD row too), the trigger second (never the anchor), the
    // level in rupees AND how it was set
    let fam12 = wpos.and_then(|p| set.labels.get(p)).and_then(|&lb| FAMILIES.get(usize::from(lb))).map_or("", |f| match *f {
        "SL" => "stop-loss",
        "TT" => "trailing target trigger",
        "BE" => "breakeven stop",
        "TSL" => "trailing stop",
        "TTP" => "trailing target",
        "TP" => "target",
        "GO" => "go-positive time stop",
        "TIME" => "time limit",
        _ => "exit rule",
    });
    let how = crate::v32::level_words(xr.as_ref(), level, r.p0, &rule_words);
    let trig_s = if tsec > 0 { fmt_sod(tsec) } else { "-".to_owned() };
    let exit_why = match key {
        XKey::Own(s, _) if sw::on(sw::WHY12) => format!("{fam12} ({rule_words}; {rule_dist}) fired at {trig_s}: {how}; the price crossed Rs {} (own buy-back order from the anchor {}); {bought}{}{}; {nc}", fmt_rs(level), fmt_sod(s), if sq.is_empty() { String::new() } else { "; ".to_owned() }, sq),
        XKey::Held(s) if sw::on(sw::WHY12) => crate::v32::held_why(fam12, &rule_words, &rule_dist, &trig_s, &how, level, &fmt_sod(s), &if sq.is_empty() { "nothing to square off".to_owned() } else { sq.clone() }, &nc),
        XKey::Own(s, _) => format!("{fam} ({rule_words}; {rule_dist}) fired at {}: the level Rs {} was crossed (anchor {}); {bought}{}{}; {nc}", if tsec > 0 { fmt_sod(tsec) } else { "-".to_owned() }, fmt_rs(level), fmt_sod(s), if sq.is_empty() { String::new() } else { "; ".to_owned() }, sq),
        XKey::Held(s) => format!("{fam} ({rule_words}) fired at {} inside 15:08-15:10 where no own order is sent: the sale's rest was cancelled there; {sq}; {nc}", fmt_sod(s), sq = if sq.is_empty() { "nothing to square off".to_owned() } else { sq.clone() }),
        XKey::None => format!("no exit rule fired before 15:08 ({}): {}; {nc}", if anc < NEVER3 { "a rule fired only at or after 15:10" } else { "the rules never fired" }, if sq.is_empty() { "nothing to square off".to_owned() } else { sq.clone() }),
    };
    // addenda 8 / 8b: the Greeks at the entry decision and at the exit decision (the app's own crates/greeks)
    let gk_words = |inst: i64| -> (String, Option<(f64, f64)>, u64) {
        if !sw::on(sw::GREEKS) {
            return ("-".to_owned(), None, 0);
        }
        let rate = crate::gk::TBILL.get().and_then(|t| t.rate_for(c.day));
        let Some((m, _stamp, prem, spot, tenor)) = crate::gk::inputs_at(c, e.ci as usize, crate::cal::sod_of(inst)) else { return ("no bar before the decision".to_owned(), None, 0) };
        let g = crate::gk::bar_greeks(spot, kc.strike, prem, tenor, kc.right == 0, rate.map_or(0, |x| x.0), -i64::try_from(r.units).unwrap_or(0));
        if g.ok {
            (format!("bar {} spot Rs {} prem Rs {} tenor {} s; IV {:.4}% delta {:.5} gamma/Rs {:.6} theta Rs {:.2}/day vega Rs {:.2}/vol pt rho Rs {:.4}", fmt_sod(crate::cal::OPEN_S + 60 * i64::try_from(m).unwrap_or(0)), fmt_rs(spot), fmt_rs(prem), tenor, g.iv * 100.0, g.delta, g.gamma_r, g.pos_theta, g.pos_vega, g.pos_rho), Some((g.iv, g.delta)), g.ns)
        } else {
            (format!("REFUSED ({}); bar {} spot Rs {} prem Rs {} tenor {} s", g.refusal, fmt_sod(crate::cal::OPEN_S + 60 * i64::try_from(m).unwrap_or(0)), fmt_rs(spot), fmt_rs(prem), tenor), None, g.ns)
        }
    };
    let (ge, gev, gns) = gk_words(t_us);
    let (gx, _, _) = gk_words(x_us);
    if let Some((iv, de)) = gev {
        x.iv = Some(iv);
        x.delta = Some(de);
    }
    x.greeks_ns = gns;
    let rate_words = crate::gk::TBILL.get().and_then(|t| t.rate_for(c.day)).map_or_else(|| "-".to_owned(), |(y, rd, age, stale)| format!("91-day T-bill {}.{:04}% released {} ({age} days old{})", y / 10_000, y % 10_000, fmt_day(rd), if stale { ", STALE (> 14 days, the app's flag)" } else { "" }));
    let rate_used = e.tab.mx.map_or_else(|| "proxy".to_owned(), |m| {
        // fix:v33 IDXELM (standing F5): under the dated table the expiry-day words name the dated base (never "2% +"), then the row's class and
        // the ELM of one lot at the rate printed beside; off (idxc 0): v3.3's words byte for byte
        let xday = if m.idxc == 0 { " (index option on its own expiry day: 2% + the 2% expiry-day add-on)" } else if c.ymd >= common_fill::margin::IDX_EXPIRY_ADD_FROM { " (index option on its own expiry day: the dated base rate + the 2% expiry-day add-on, from 2024-11-20)" } else { " (index option on its own expiry day: the dated base rate, no add-on before 2024-11-20)" };
        let v = format!("{}.{:02}%{}", m.parts.rate_h / 100, m.parts.rate_h % 100, if kc.kind == 0 && kc.expiry == c.ymd { xday } else if kc.kind == 1 && sw::on(sw::MARGIN_V3) { " (stock: AEL OTH row, rule v3)" } else { "" });
        let base = m.parts.rate_h - if kc.kind == 0 && kc.expiry == c.ymd && c.ymd >= common_fill::margin::IDX_EXPIRY_ADD_FROM { common_fill::margin::IDX_EXPIRY_ADD_H } else { 0 };
        match m.idxc {
            1 => format!("{v} (index: CONSERVATIVE dated row, base {}.{:02}%: no dated evidence of 2% for this session (Addendum 4); the 2% reading beside: ELM Rs {} a lot)", base / 100, base % 100, fmt_rs(i64::from(m.elm2))),
            2 => format!("{v} (index: VERIFIED dated row, base {}.{:02}%)", base / 100, base % 100),
            _ => v,
        }
    });
    let extra = vec![
        money.orders.map_or_else(|| "-".to_owned(), |(a, b2, k)| format!("{a}/{b2}/{k}")),
        if slice > 0 { slice.to_string() } else { "no freeze limit known".to_owned() },
        entry_why,
        exit_why,
        g_win,
        g_dhan,
        g_nse,
        g_open,
        x.entry_ttf.to_string(),
        x.exit_ttf.map_or_else(|| "-".to_owned(), |v| v.to_string()),
        format!("{}.{:02}%", x.filled_bp / 100, x.filled_bp % 100),
        format!("{lq_c}/{lq_g}"),
        x.resends.to_string(),
        r.unclosed.to_string(),
        fmt_day(ed),
        wd.to_owned(),
        format!("{} ({msrc})", if monthly { "monthly" } else { "weekly" }),
        format!("{dte} ({})", crate::expiry::BUCKETS.get(x.bucket).copied().unwrap_or("?")),
        if moved { "yes".to_owned() } else { "no".to_owned() },
        rate_used,
        ge,
        gx,
        rate_words,
    ];
    Ok((x, extra))
}

const TRADES_HEAD: &str = "pick\ttrade_no\tsegment\tunderlying\tcontract\texpiry\tday\tdecision_T\tsetting\tshare_pct\tN\tlots\tp0_rs\tunits_sold\tsold_value_rs\texit_reason\ttrigger_second\texit_anchor\ttrigger_level_rs\trules_at_event (distances; floor; absent)\tbuyback_first_block\tbuyback_first_px_rs\tbuyback_units\tbuyback_value_rs\tsquareoff_units\tsquareoff_value_rs\tunclosed_units\tcharges_rs\tnet_rs\tlast_fill_known\tflags\tliq_traded_s/longest_gap_60s\tsweep_minus_detail_paisa\texpiry_note";
/// v3.1: the columns appended to every trade row when a v3.1 switch is on.
const TRADES_HEAD_V31: &str = "\tmargin_lot_used_rs\tmargin_lot_proxy_rs\tmargin_order_addon\tcharges_per_side\tstop_overshoot_pct_of_sale\tmax_above_sale_pct\tmargin_file\tnever_closed\treal_fill_after_1530\tnet_at_gate_stress_rs\tadverse_after_sale_2/5/10s_bp\tadverse_after_exit_decision_2/5/10s_bp";

/// The max drawdown of trades realized at their known instants (closes of one second grouped), in time order.
fn dd_grouped(v: &mut [(i64, i64)]) -> i64 {
    v.sort_by_key(|x| x.0);
    let (mut eq, mut peak, mut dd) = (0_i64, 0_i64, 0_i64);
    let mut i = 0;
    while let Some(&(t, _)) = v.get(i) {
        while let Some(&(t2, n)) = v.get(i) {
            if t2 != t {
                break;
            }
            eq += n;
            i += 1;
        }
        peak = peak.max(eq);
        dd = dd.max(peak - eq);
    }
    dd
}

/// All outputs; returns the RESULTS markdown.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn write3(log: &mut String, rdir: &Path, run_id: &str, store: &mut Store, var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, menu: &Menu, ma: &MenuA, sets: &Sets, sel: &[usize], csets: &[CSet], segs: &[Seg3], liqs: &[usize], cal: &crate::rules::Calendar, compat: bool, days: &[String], threads: usize, mman: Option<&Manifest>) -> Result<String, String> {
    let _ = (var, threads, ma);
    let v31 = sw::any();
    let mut md = String::new();
    let _ = writeln!(md, "# RESULTS-SPIKEFADE-v3: SNIPER s25b spike-fade option SELLING, every SL / TSL / TP / TTP kind (fixed and dynamic) + the liquidity dial, run `{run_id}`");
    let _ = writeln!(md);
    if v31 {
        let _ = writeln!(md, "**ENGINE v3.1** = engine v3 + the v2.2 delta (MAIN-RULINGS-ENGINE-v22: exact NSE margin, carried fill entitlement, orders end 15:30, never-closed units valued from the exit decision, the running MTM loss in the margin check, max DD with same-second closes grouped, list and wording fixes) + the operator's decisions of 15 Nov (a) headline picks only from SMALL stops (REQ 47), (b) the headline uses at most 25% of each block's usable volume (10% printed beside as the extreme worst case; 50-100% only as labelled comparisons), (c) the jump-stress table and the measured adverse moves beside every pick, a headline only if positive at +{}.{:02}% adverse on both sides, (d) NAKED SELLING ONLY (no hedged pair or other selling strategy is built; hedging stays a later option). Switches: {}.", sw::GATE_BP.load(Ordering::Relaxed) / 100, sw::GATE_BP.load(Ordering::Relaxed) % 100, sw::words());
        let _ = writeln!(md);
    }
    if crate::fix33::any() {
        let _ = writeln!(md, "**ENGINE v3.3 FIXES** (integrate:v33, MAIN-RULINGS-ENGINE-v22 Addendum 28.2: the BLOCKING items of the re-checks check-engine-1r / check-engine-2r; they change results): {}.", crate::fix33::words());
        let _ = writeln!(md);
    }
    // fix:v33: the gate fixes' words and their counts (none of this is printed with every fix:v33 switch off: v3.3 byte for byte)
    if crate::fixg::any() {
        let _ = writeln!(md, "**ENGINE v3.3 GATE FIXES** (fix:v33, MAIN-RULINGS-ENGINE-v22 Addenda 24-28: the BLOCKING items of the v3.3 gates and the re-check that code alone can close; they change results): {}.", crate::fixg::words());
        if crate::fixg::on(crate::fixg::LOT) {
            let _ = writeln!(md, "- {}", crate::fixg::lot_line());
        }
        if let Some(w) = crate::fixg::IDX_WORDS.get() {
            let _ = writeln!(md, "- {w}");
        }
        if crate::fixg::on(crate::fixg::WIN) {
            let _ = writeln!(md, "- {}", crate::fixg::win_line());
        }
        let _ = writeln!(md, "- Still OPEN (NOT GATED, printed on every result): price bands, band lock and NSE limit-price protection on re-sends, Dhan's option market protection % unsourced (standing F7 = exits F-B1: data or the operator's named acceptance); stock freeze limits undated before 2026-11-02 (standing F3: data or a MAIN ruling); the positional mode not built (exits F-B3).");
        let _ = writeln!(md);
    }
    let margin_words = match mman {
        Some(m) => if sw::on(sw::MARGIN_V2) { format!("Margin: the EXACT NSE margin per contract-day by MARGIN RULE v2 (addendum 9: SPAN + the option's value at NSE's file price + exposure on the near-month index FUTURE's price in the same file (index) or the live stock price (stock), + Rs 20 per order; it rebuilds 15 of 15 Dhan screens to the paisa, closed market only), from the margin-history table (manifest sha256 {}, {} tables; the latest table published strictly before T; begin-of-day files for history, intraday i2-i4 only where downloaded) and the futures table (sha256 {}); the v2.1 proxy (C_VAR1 of 2026-10-30 / assumed index formula) printed beside as a reading.", m.sha, m.rows.len(), crate::mtable::FUTURES.get().map_or("-", |f| f.sha.as_str())) } else { format!("Margin: the EXACT NSE margin per contract-day (rule v1: SPAN + the option's value at NSE's file price + exposure on the live spot), from the margin-history table (manifest sha256 {}, {} tables; the latest table published strictly before T); the v2.1 proxy printed beside as a reading.", m.sha, m.rows.len()) },
        None => "Index margin ASSUMED (no SPAN file); stock margin C_VAR1 dated 2026-10-30".to_owned(),
    };
    if crate::fix33::on(crate::fix33::WORDS33) {
        // v3.3 fix WORDS33 (check-engine-2 M2): the label block from the run's own data (each day's cube manifest names its vendor), never
        // the 8-day Tickvault words on a GDFL history run
        let (wf_ran, wf_run_rows) = (segs.iter().any(|s| s.win.iter().any(|w| w.run)), segs.iter().map(|s| s.win.iter().filter(|w| w.run).count()).max().unwrap_or(0));
        let _ = writeln!(md, "**LABELLED DIAGNOSTIC + HINDSIGHT** (every dial chosen on the same days it is scored on). Underlying: NIFTY and BANKNIFTY (index options; other NSE indices only where `--idx-und` / `--cube-extra` say so) and the F&O stocks the day cubes hold (stock options), segments never mixed. Signal: the Top Volume screener on 1-minute candles of the run's data: {} (one segment and one expiry per list) with the liquidity dial. Traded: the event option SOLD naked, intraday (no own order from 15:08, square-off from 15:10). Size: Rs 20 L account, N slots of Rs 20 L / N. Period: {} .. {} ({} days). Proof step 1 (own engine). {} Nothing goes live. {}; GIFT Nifty: not used (no data).", crate::fix33::VENDOR.get().map_or("vendor not read", String::as_str), days.first().map_or("?", String::as_str), days.last().map_or("?", String::as_str), days.len(), if wf_ran { format!("**Walk-forward / train-test: {wf_run_rows} of 21 many-window rows run on this period** (each row and every split prints its own PASSED / NOT PASSED below; only a PASSED walk-forward can go live).") } else { "**Walk-forward test NOT RUN; Train/test NOT RUN; many-window rows: 0 of 21 run** (no learning month and later test month inside this period).".to_owned() }, if v31 { margin_words.clone() } else { "Index margin ASSUMED (no SPAN file); stock margin C_VAR1 dated 2026-10-30".to_owned() });
    } else {
    let _ = writeln!(md, "**LABELLED DIAGNOSTIC + HINDSIGHT** (every dial chosen on the same days it is scored on). Underlying: NIFTY and BANKNIFTY (index options) and the F&O stocks Tickvault records (stock options), segments never mixed. Signal: the Top Volume screener on Tickvault 1-minute candles (one segment and one expiry per list) with the liquidity dial. Traded: the event option SOLD naked, intraday (no own order from 15:08, square-off from 15:10). Size: Rs 20 L account, N slots of Rs 20 L / N. Period: {} .. {} ({} days). Proof step 1 (own engine). **Walk-forward test NOT RUN; Train/test NOT RUN; many-window rows: 0 of 21 run** (the 8 Tickvault days lie inside the 2026 piece: no learning month). Nothing goes live. {}; GIFT Nifty: not used (no data).", days.first().map_or("?", String::as_str), days.last().map_or("?", String::as_str), days.len(), if v31 { margin_words.clone() } else { "Index margin ASSUMED (no SPAN file); stock margin C_VAR1 dated 2026-10-30".to_owned() });
    }
    let _ = writeln!(md);
    if v31 {
        let _ = writeln!(md, "- Top Volume lists are one per (segment, expiry date){}: NIFTY and BANKNIFTY share one list in the weeks their nearest expiries coincide, otherwise one list each (2026 check F2; the one-list-per-underlying variant is a labelled run for the walk-forward choice only: `--lists underlying`).", if sw::on(sw::LISTS_PER_UND) { " -- THIS RUN IS THE VARIANT: one list per underlying and expiry" } else { "" });
        let _ = writeln!(md, "- Fills: REQ 70 worst-case blocks (index 1 s, stock 2 s), {}; orders {}; units the fill rule never closes are VALUED, NOT FILLED ({}) and shown beside every row (trades, units, net; 2026 F1, v2.1 S-2).", if sw::on(sw::CARRY) { "the share's entitlement CARRIED across blocks (whole lots once the carry covers a lot; never more than share x cumulative usable volume)" } else { "the per-block whole-lot floor" }, if sw::on(sw::END_1530) { "end at 15:30:00 (no fill after 15:30)" } else { "walk to 15:40 (v2.1)" }, if sw::on(sw::VALUE_FROM_EXIT) { "at the highest HIGH between the exit decision and 15:30" } else { "at the highest HIGH from the entry to the session end (v2.1)" });
        let _ = writeln!(md, "- Margin check: available = Rs 20 L base (net losses taken off, profits never added) - margin in use{}; the Rs 20 L cap on the margin in use at every instant. Max drawdown {}. Net only (after charges), never gross.", if sw::on(sw::MTM) { " - the unrealised loss of the open positions (losses only; MTM refusals counted per row)" } else { "" }, if sw::on(sw::DD_GROUPED) { "on the equity at each close instant, same-second closes grouped (the booking-order figure beside)" } else { "in booking order" });
        let _ = writeln!(md, "- Worst day / worst trade print \"none\" when no day / trade lost (2026 F6). Every never-closed unit is counted, never hidden.");
        let _ = writeln!(md);
    }
    let _ = writeln!(md, "- Engine: spikefade v3 = `cp -p -R -n` of the spikefade ENGINE v2.1 (`scratch/spikefade-v21`, run `sf21-20261115-021451`), fill crate v2.2 (`pinned/common-fill-v22` = the FA1 crate + the additive xrule module). Cut rule: {}.", if compat { "v2 COMPATIBILITY (watch to 15:08, no B2)" } else { "v3 (watch to 15:10, B2 cancel at a held anchor)" });
    if v31 {
        let _ = writeln!(md, "- Engine v3.1: `scratch/spikefade-v31` = `cp -p -R -n` of `scratch/spikefade-v3` + the v2.2 delta + the decisions; fill crate v2.3 (`pinned/common-fill-v23` = crate v2.2 + the ONE margin module M1 + the opt-in carried entitlement K1). Day-major mode: D1 (pick tables rebuilt), D2 (days streamed), D3 (flagged window rows re-walked exactly) applied.");
    }
    // v3.3 (search:v33, Addendum 28.1): the loaded sets file's own name (round B printed the v1 name for any file) and, when the file holds
    // the role SEARCH1D, its count; a v1 run prints the same words as before
    let s1d_n = sets.settings.iter().filter(|s| s.s1d).count();
    let s1d_words = if s1d_n > 0 { format!("; v3.3 role SEARCH1D (Addendum 28.1): {s1d_n} SEARCH settings = the every-year set SEARCH-1D") } else { String::new() };
    let _ = writeln!(md, "- Menu `EXITS-DYN-MENU-v1.tsv` sha256 {} ({} canonical rules: 98 minus the 3 TTP rows identical to TSL rows, R2); sets `{}` sha256 {} ({} canonical settings: S1 392, S2 1,232, S3 1,537, S4 5,020; S0 = the v2 grid's 125 flagged{s1d_words}). This run: {} settings.", menu.sha, menu.len(), if sets.name.is_empty() { "EXITS-DYN-SETS-v1.tsv" } else { sets.name.as_str() }, sets.sha, sets.settings.len(), sel.len());
    let _ = writeln!(md, "- THE pick comes from SEARCH settings only (R1 / Q16: SL-P 2 / 3 / 5 % and SL-A1 0.10 / 0.25 / 0.50); every wider stop is a labelled COMPARISON, never the headline (REQ 47a). The v2 grid's best (S0) is printed beside as the s27 baseline.");
    let _ = writeln!(md, "- Spike-selling stops sit on the option's own chart, because that chart gives the signal (R6: MAIN's reading of REQ 47a for s25b, not a change of it; the operator may correct it). An underlying-spot stop (SL-U) is deferred: the cube holds only spot 1-minute closes (no spot seconds, high or low).");
    let _ = writeln!(md, "- Combinations (R2 / F13): {} distinct exit settings x 23,040 screener settings x 7 liquidity values x 5 share levels x 4 slot counts = {} accounts per segment for the full sets; this run walked the counts in Table 0 (\"best of X accounts walked\").", sets.settings.len(), u128::try_from(sets.settings.len()).unwrap_or(0) * 23_040 * 7 * 20);
    let _ = writeln!(md);
    // Table 0: counts and costs
    let _ = writeln!(md, "## Table 0. What was walked (MEASURED on this machine)");
    let _ = writeln!(md);
    let _ = writeln!(md, "| segment | unique events | readings s | stage A s | entry paths | distinct exit keys p50 / p99 / max | steps per path p50 / p99 / max | R pieces per path p50 / p99 / max | stage A MB | unique lists (config x liquidity) | accounts walked | account-event steps | stage B s | ns per step |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for (seg, sr) in segs.iter().enumerate() {
        if sr.evs.is_empty() {
            continue;
        }
        let mut d: Vec<u64> = sr.cnt.distinct.iter().map(|&x| u64::from(x)).collect();
        let mut s: Vec<u64> = sr.cnt.steps.iter().map(|&x| u64::from(x)).collect();
        let mut pcs: Vec<u64> = sr.cnt.pieces.iter().map(|&x| u64::from(x)).collect();
        let mx = |v: &[u64]| v.iter().copied().max().unwrap_or(0);
        let _ = writeln!(md, "| {} | {} | {:.1} | {:.1} | {} | {} / {} / {} | {} / {} / {} | {} / {} / {} | {} | {} | {} | {} | {:.1} | {:.1} |", SEG_SHORT.get(seg).copied().unwrap_or("?"), sr.evs.len(), sr.t_read, sr.t_stage_a, sr.cnt.paths, pct(&mut d, 50), pct(&mut d, 99), mx(&d), pct(&mut s, 50), pct(&mut s, 99), mx(&s), pct(&mut pcs, 50), pct(&mut pcs, 99), mx(&pcs), sr.stage_a_bytes / 1_000_000, sr.uls.len(), sr.accounts, sr.steps, sr.t_stage_b, if sr.steps > 0 { sr.t_stage_b * 1e9 / sr.steps as f64 } else { 0.0 });
    }
    let _ = writeln!(md);
    if v31 {
        // the margin table (item 1) and the O(1) lookup measurements (addendum 2)
        let _ = writeln!(md, "## Table M. The margin each event was sized with (v3.1 item 1) and the lookup costs (MEASURED, p50 / p99 per lookup kind)");
        let _ = writeln!(md);
        let _ = writeln!(md, "| segment | events | exact margin known | refused: no_span_row / no_elm_rate / no_margin_file | on a stale file (flagged) | on a fallback table (no table that day) | rule v2: index options without a future in the file (spot base used) | exact / proxy per lot p10 / median / p90 | lookups |");
        let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|");
        for (seg, sr) in segs.iter().enumerate() {
            if sr.evs.is_empty() {
                continue;
            }
            if sr.mx.is_empty() {
                let _ = writeln!(md, "| {} | {} | proxy run | - | - | - | - | - | - |", SEG_SHORT.get(seg).copied().unwrap_or("?"), sr.evs.len());
                continue;
            }
            let known = sr.mx.iter().filter(|m| m.refuse.is_none()).count();
            let by = |r: Refuse| sr.mx.iter().filter(|m| m.refuse == Some(r)).count();
            let stale = sr.mx.iter().filter(|m| m.refuse.is_none() && m.stale).count();
            let fb = sr.mx.iter().filter(|m| m.fallback).count();
            let mut ratio: Vec<i64> = sr.evs.iter().zip(&sr.mx).filter(|(e, m)| m.refuse.is_none() && e.tab.margin_lot_proxy > 0).map(|(e, m)| m.lot_exact * 1_000 / e.tab.margin_lot_proxy).collect();
            ratio.sort_unstable();
            let q = |p: usize| ratio.get(ratio.len().saturating_sub(1) * p / 100).map_or("-".to_owned(), |&v| format!("{}.{:03}", v / 1_000, v % 1_000));
            let ffb = sr.mx.iter().filter(|m| m.fut_fallback).count();
            let _ = writeln!(md, "| {} | {} | {known} | {} / {} / {} | {stale} | {fb} | {ffb} | {} / {} / {} | {} |", SEG_SHORT.get(seg).copied().unwrap_or("?"), sr.evs.len(), by(Refuse::NoSpanRow), by(Refuse::NoElmRate), by(Refuse::NoMarginFile), q(10), q(50), q(90), sr.mlat.line());
        }
        let _ = writeln!(md);
    }
    // Table W: the 21 windows
    if sw::on(sw::WF19) {
        table_w_a19(&mut md, segs, sets, sel, csets, days);
    } else {
    let _ = writeln!(md, "## Table W. Many walk-forward tests (WINDOWS-MWF-v1, 21 rows from this one sweep; THE pick per row from SEARCH settings only on its own learning pieces)");
    let _ = writeln!(md);
    if crate::fixg::on(crate::fixg::WIN) {
        let _ = writeln!(md, "{}", crate::fixg::win_line());
        let _ = writeln!(md);
    }
    let _ = writeln!(md, "| segment | row | learning pieces | test pieces | ran | THE pick (learning key: every learning year positive, smallest learning max DD, most learning net) | learning net Rs | learning max DD Rs | test years all positive | test net Rs | verdict | account-rows checked equal / re-walked |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|");
    let yr = |p: usize| crate::wf::Y0 + i64::try_from(p).unwrap_or(0);
    let mut passed = [0_usize; 2];
    for (seg, sr) in segs.iter().enumerate() {
        for w in &sr.win {
            let Some(r) = &w.row else { continue };
            let (pick, ln, ld, tp, tn, verdict) = match (&w.best, w.run) {
                (_, false) => ("-".to_owned(), "-".to_owned(), "-".to_owned(), "-".to_owned(), "-".to_owned(), "NOT RUN (no session in its learning years: the run's days lie outside them)".to_owned()),
                (None, true) => ("(no SEARCH account traded in the learning years)".to_owned(), "-".to_owned(), "-".to_owned(), "-".to_owned(), "-".to_owned(), "NOT PASSED (W07)".to_owned()),
                (Some((_, sm, lf, tf2)), true) => {
                    let ok = lf.all_pos && tf2.all_pos && tf2.trades > 0;
                    if ok {
                        if let Some(x) = passed.get_mut(seg) {
                            *x += 1;
                        }
                    }
                    (format!("{} / {}", set_words(sets, sel, sm.x).0, crate::report::cfg_words(seg, sr.uls.get(sm.list as usize).map_or(0, |u| u.0) as usize, &sr.lv)), fmt_rs(lf.net), fmt_rs(lf.mdd), if tf2.all_pos { "yes".to_owned() } else { "no".to_owned() }, fmt_rs(tf2.net), if ok { "PASSED".to_owned() } else { "NOT PASSED".to_owned() })
                }
            };
            let verdict = if w.pending > 0 { format!("{verdict}; {} account-rows PENDING a re-walk pass (day-major)", w.pending) } else { verdict };
            // v3.2 addendum 15: the row's pick re-walked on its TEST years: months at or above Rs 2,00,000 net (every month counted)
            let verdict = match (&w.best, sw::on(sw::SEARCH15)) {
                (Some((_, sm, _, _)), true) => {
                    let dd: Vec<i64> = days.iter().filter_map(|d| crate::cal::parse_day(d)).collect();
                    let in_test = |d: i64| { let y = crate::cal::civil(d).0 - crate::wf::Y0; y >= i64::try_from(r.test.0).unwrap_or(0) && y <= i64::try_from(r.test.1).unwrap_or(0) };
                    let tm = crate::v32::months_of(&dd.iter().copied().filter(|&d| in_test(d)).collect::<Vec<_>>());
                    let mut mo: BTreeMap<i64, i64> = tm.iter().map(|&m| (m, 0)).collect();
                    for t in decided_trades(sr, sm, csets, &dd).iter().filter(|t| in_test(t.4)) {
                        *mo.entry(crate::v32::month_id(t.4)).or_insert(0) += t.3;
                    }
                    let ok = mo.values().filter(|&&v| crate::kern::month_ok(v)).count();
                    format!("{verdict}; addendum 15 test months >= Rs 2,00,000: {ok} of {} (worst Rs {}){}", mo.len(), fmt_rs(mo.values().copied().min().unwrap_or(0)), if ok == mo.len() && !mo.is_empty() { "" } else { " -> NOT PASSED under addendum 15" })
                }
                _ => verdict,
            };
            let _ = writeln!(md, "| {} | {} | {}..{} | {}..{} | {} | {pick} | {ln} | {ld} | {tp} | {tn} | {verdict} | {} / {} |", SEG_SHORT.get(seg).copied().unwrap_or("?"), r.id, yr(r.learn.0), yr(r.learn.1), yr(r.test.0), yr(r.test.1), if w.run { "yes" } else { "no" }, w.checked, w.rewalked);
        }
    }
    let _ = writeln!(md);
    let _ = writeln!(md, "Many walk-forward tests: {} of 21 PASSED (index), {} of 21 PASSED (stock). The 21 rows share years: they are not 21 separate proofs.", passed[0], passed[1]);
    let _ = writeln!(md);
    }
    // Liquidity table
    let _ = writeln!(md, "## Table L. The liquidity dial (60 s strictly before T; per value: the candidates it removes and the share of wanted lots the sale filled)");
    let _ = writeln!(md);
    let _ = writeln!(md, "| segment | value | list entries removed | unique events removed | unique events kept | filled share of wanted units at share 10 / 25 / 50 / 75 / 100 % (kept, priced events, every slot count) |");
    let _ = writeln!(md, "|---|---|---|---|---|---|");
    let mut ltsv = String::from("segment\tliquidity\twords\tlist_entries_removed\tunique_events_removed\tunique_events_kept\tfilled_share_l10\tfilled_share_l25\tfilled_share_l50\tfilled_share_l75\tfilled_share_l100\n");
    for (seg, sr) in segs.iter().enumerate() {
        if sr.evs.is_empty() {
            continue;
        }
        for &v in liqs {
            let mut got = [0_u128; NL];
            let mut want = [0_u128; NL];
            if sr.tl_online {
                // v3.3 C4a: the online sums (the same integer sums over the same events)
                if let Some(row) = sr.tl_sum.get(v) {
                    for (l, &(a, b)) in row.iter().enumerate() {
                        if let (Some(x), Some(y)) = (got.get_mut(l), want.get_mut(l)) {
                            *x = a;
                            *y = b;
                        }
                    }
                }
            } else {
                for (i, x) in sr.ex.iter().enumerate() {
                    let (c, g) = sr.liq.get(i).copied().unwrap_or((0, 0));
                    if !liq_pass(v, c, g) {
                        continue;
                    }
                    for &(l, _, u, w) in &x.fill {
                        if let (Some(a), Some(b)) = (got.get_mut(usize::from(l)), want.get_mut(usize::from(l))) {
                            *a += u128::from(u);
                            *b += u128::from(w);
                        }
                    }
                }
            }
            let sh: Vec<String> = (0..NL).map(|l| { let (a, b) = (got.get(l).copied().unwrap_or(0), want.get(l).copied().unwrap_or(0)); if b == 0 { "-".to_owned() } else { format!("{}.{}%", a * 100 / b, (a * 1000 / b) % 10) } }).collect();
            let r = sr.removed.get(v).copied().unwrap_or_default();
            let _ = writeln!(md, "| {} | {} {} | {} | {} | {} | {} |", SEG_SHORT.get(seg).copied().unwrap_or("?"), LIQ_SHORT.get(v).copied().unwrap_or("?"), LIQ_NAMES.get(v).copied().unwrap_or("?"), r.0, r.1, r.2, sh.join(" / "));
            let _ = writeln!(ltsv, "{}\t{}\t{}\t{}\t{}\t{}\t{}", SEG_SHORT.get(seg).copied().unwrap_or("?"), LIQ_SHORT.get(v).copied().unwrap_or("?"), LIQ_NAMES.get(v).copied().unwrap_or("?"), r.0, r.1, r.2, sh.join("\t"));
        }
    }
    std::fs::write(rdir.join("liquidity.tsv"), ltsv).map_err(|e| e.to_string())?;
    let _ = writeln!(md);
    // Stage A per-rule table
    let mut rt = String::from("segment\trule\tfamily\trole\tpaths_present\tabsent_atr1\tabsent_atr5\tabsent_vol\tabsent_theta_none\tabsent_theta_no_calendar\tabsent_tpf_not_below_entry\tabsent_theta_level_le_0\tdistance_floor_1_paisa\tfired_own_before_1508\tfired_held_1508_1510\tnever\n");
    for (seg, sr) in segs.iter().enumerate() {
        for (i, r) in menu.rules.iter().enumerate() {
            let ab = sr.cnt.absent.get(i).copied().unwrap_or_default();
            let (own, held, never) = (sr.cnt.own.get(i).copied().unwrap_or(0), sr.cnt.held.get(i).copied().unwrap_or(0), sr.cnt.never.get(i).copied().unwrap_or(0));
            let _ = writeln!(rt, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{own}\t{held}\t{never}", SEG_SHORT.get(seg).copied().unwrap_or("?"), r.id, FAMILIES.get(usize::from(r.fam)).copied().unwrap_or("?"), if r.kind.sl() { if r.search { "SEARCH" } else { "COMPARISON" } } else { "-" }, own + held + never, ab[0], ab[1], ab[2], ab[3], ab[4], ab[5], ab[6], sr.cnt.floor.get(i).copied().unwrap_or(0));
        }
    }
    std::fs::write(rdir.join("stageA-rules.tsv"), rt).map_err(|e| e.to_string())?;
    let _ = writeln!(md, "Stage A per rule (present paths, absences by reason, 1-paisa floors, fired before 15:08 / held / never): `stageA-rules.tsv`. The absences are per (event, share level); an absent SL refuses the event for its settings (REQ 47, `sl_unready`).");
    let _ = writeln!(md);
    // Tops
    let none_or = |v: i64| if v31 && v == 0 { "none".to_owned() } else { fmt_rs(v) };
    let mut top = String::from("segment\tgroup\tview\trank\tlist_config\tliquidity\tequal_configs\tsetting\trole\tset\tshare_pct\tN\ttrades\twins\tnet_rs\tmax_dd_rs\tworst_trade_rs\tworst_day_rs\tdays_pos\tdays_neg\tpeak_margin_rs\texit_words");
    if v31 {
        top.push_str("\tnet_at_gate_stress_rs\tmax_dd_other_reading_rs\tnever_closed_trades\tnever_closed_units\tnever_closed_net_rs\tmtm_refusals");
    }
    top.push('\n');
    let mut picks: Vec<(String, usize, Sum)> = Vec::new();
    for (seg, sr) in segs.iter().enumerate() {
        if sr.evs.is_empty() {
            continue;
        }
        let ss = SEG_SHORT.get(seg).copied().unwrap_or("?");
        let _ = writeln!(md, "## {}: the best accounts (best of {} accounts walked)", SEG_NAME.get(seg).copied().unwrap_or("?"), sr.accounts);
        let _ = writeln!(md);
        for g in 0..sr.ngroup.max(NGROUP) {
            let Some(tops) = sr.tops.get(g) else { continue };
            for v in 0..NVIEW {
                for (rank, sm) in tops.best(v).iter().take(50).enumerate() {
                    let (id, search, setn, l) = set_words(sets, sel, sm.x);
                    let (rep, lq, _) = sr.uls.get(sm.list as usize).copied().unwrap_or((0, 0, 0));
                    let _ = write!(top, "{ss}\t{}\t{}\t{}\t{}\t{}\t{}\t{id}\t{}\tS{setn}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", group_name(g), crate::acct::view_letter(v), rank + 1, crate::report::cfg_words(seg, rep as usize, &sr.lv), LIQ_SHORT.get(usize::from(lq)).copied().unwrap_or("?"), sr.ul_cfgs.get(sm.list as usize).map_or(0, Vec::len), if search { "SEARCH" } else { "COMPARISON" }, SHARE_BP.get(l).copied().unwrap_or(0) / 100, crate::acct::n_of_s(usize::from(sm.seg), usize::from(sm.n)), sm.trades, sm.wins, fmt_rs(sm.net), fmt_rs(sm.mdd), none_or(sm.worst), none_or(sm.worst_day), sm.days_pos, sm.days_neg, fmt_rs(sm.pk_margin), setting_words(menu, sets, sel, sm.x));
                    if v31 {
                        let _ = write!(top, "\t{}\t{}\t{}\t{}\t{}\t{}", fmt_rs(sm.net_s2), fmt_rs(sm.mdd_alt), sm.nc_trades, sm.nc_units, fmt_rs(sm.nc_net), sm.mtm_ref);
                    }
                    top.push('\n');
                }
            }
        }
        // THE pick (view S, SEARCH only), REQ 55 view A (SEARCH), the best COMPARISON (S), the S0 baseline (S and A); v3.1: the HEADLINE picks first
        let wanted = wanted_picks(sr.ngroup);
        if v31 {
            let _ = writeln!(md, "| pick | group | view | list config | liquidity | setting (words) | role | share | N | trades | wins | net Rs | max DD Rs ({}) | max DD Rs (other reading) | worst trade Rs | worst day Rs | days + / - | net at +{}.{:02}% adverse both sides Rs | positive at the gate | never-closed trades / units / net Rs (valued, not filled) | MTM refusals |", if sw::on(sw::DD_GROUPED) { "same-second closes grouped" } else { "booking order" }, sw::GATE_BP.load(Ordering::Relaxed) / 100, sw::GATE_BP.load(Ordering::Relaxed) % 100);
            let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
        } else {
            let _ = writeln!(md, "| pick | group | view | list config | liquidity | setting (words) | role | share | N | trades | wins | net Rs | max DD Rs | worst trade Rs | worst day Rs | days + / - |");
            let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
        }
        for (name, g, v) in wanted {
            let Some(sm) = sr.tops.get(g).and_then(|t| t.best(v).first().copied()) else {
                if v31 {
                    let _ = writeln!(md, "| {ss}-{name} | {} | {} | (none: no account{}) | | | | | | | | | | | | | | | | | |", group_name(g), crate::acct::view_letter(v), if g == NGROUP { " passes the headline gate (SEARCH small stop, share <= 25%, positive at the gate stress)" } else if v == 8 { " with a trade and net > 0" } else { " with a trade" });
                } else {
                    let _ = writeln!(md, "| {ss}-{name} | {} | {} | (none: no account with a trade{}) | | | | | | | | | | | | |", GROUP_NAMES.get(g).copied().unwrap_or("?"), crate::acct::view_letter(v), if v == 8 { " and net > 0" } else { "" });
                }
                continue;
            };
            let (_, search, _, l) = set_words(sets, sel, sm.x);
            let (rep, lq, _) = sr.uls.get(sm.list as usize).copied().unwrap_or((0, 0, 0));
            if v31 {
                let _ = writeln!(md, "| {ss}-{name} | {} | {} | {} | {} | {} | {} | {}% | {} | {} | {} | {} | {} | {} | {} | {} | {} / {} | {} | {} | {} / {} / {} | {} |", group_name(g), crate::acct::view_name(v), crate::report::cfg_words(seg, rep as usize, &sr.lv), LIQ_NAMES.get(usize::from(lq)).copied().unwrap_or("?"), setting_words(menu, sets, sel, sm.x), if search { "SEARCH" } else { "COMPARISON" }, SHARE_BP.get(l).copied().unwrap_or(0) / 100, crate::acct::n_of_s(usize::from(sm.seg), usize::from(sm.n)), sm.trades, sm.wins, fmt_rs(sm.net), fmt_rs(sm.mdd), fmt_rs(sm.mdd_alt), none_or(sm.worst), none_or(sm.worst_day), sm.days_pos, sm.days_neg, fmt_rs(sm.net_s2), if sm.net_s2 > 0 { "yes" } else { "NO" }, sm.nc_trades, sm.nc_units, fmt_rs(sm.nc_net), sm.mtm_ref);
            } else {
                let _ = writeln!(md, "| {ss}-{name} | {} | {} | {} | {} | {} | {} | {}% | {} | {} | {} | {} | {} | {} | {} | {} / {} |", GROUP_NAMES.get(g).copied().unwrap_or("?"), crate::acct::view_name(v), crate::report::cfg_words(seg, rep as usize, &sr.lv), LIQ_NAMES.get(usize::from(lq)).copied().unwrap_or("?"), setting_words(menu, sets, sel, sm.x), if search { "SEARCH" } else { "COMPARISON" }, SHARE_BP.get(l).copied().unwrap_or(0) / 100, crate::acct::n_of_s(usize::from(sm.seg), usize::from(sm.n)), sm.trades, sm.wins, fmt_rs(sm.net), fmt_rs(sm.mdd), fmt_rs(sm.worst), fmt_rs(sm.worst_day), sm.days_pos, sm.days_neg);
            }
            picks.push((format!("{ss}-{name}"), seg, sm));
            // v3.2 (check M2): every pick's 10% share twin, detailed trade by trade beside it (the extreme worst case)
            if sw::on(sw::PICKS32)
                && let Some(tw) = twin10(&sm)
            {
                picks.push((format!("{ss}-{name}-AT-10PCT"), seg, tw));
            }
        }
        let _ = writeln!(md);
        if crate::fix33::on(crate::fix33::CAP_MIN) {
            // v3.3 fix CAP_MIN (check-engine-1 M1r): which accounts the per-minute cap check removed from the views these picks read
            let _ = writeln!(md, "v3.3 fix CAP_MIN (check-engine-1r M1r): the first {} accounts of every view a pick or the verdict view reads were checked MINUTE BY MINUTE (margin in use + the running MTM loss against Rs 20 L + min(0, realized); nothing acts between sales: no RMS square-off is modelled, matrix X20): {} checked, {} EXCLUDED; each such view keeps its checked accounts only, so every pick above passed the check. NOT cap-checked (never read them as picks): the summary rows of top-v3.tsv (best 50 per group and view) and the per-combo statistics of splits18.tsv.{}", crate::fix33::CAP_K.load(Ordering::Relaxed), sr.cap33.0, sr.cap33.1, if sr.cap33_lines.is_empty() { String::new() } else { format!(" Excluded: {}.", sr.cap33_lines.join("; ")) });
            let _ = writeln!(md);
        }
        // decision (b): the headline pick's same account at the 10% share, printed beside as the extreme worst case
        if v31 && sr.ngroup > NGROUP
            && let Some(sm) = sr.tops.get(NGROUP).and_then(|t| t.best(8).first().copied())
        {
            let (si, l) = (usize::from(sm.x) / NL, usize::from(sm.x) % NL);
            if l == 0 {
                let _ = writeln!(md, "Decision (b): {ss}-HEADLINE-S already uses the 10% share (the extreme worst case).");
            } else if let (Some(list), Some(cs)) = (sr.ul(sm.list as usize), csets.get(si)) {
                let tf = sr.uls.get(sm.list as usize).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) });
                let (_, s10, _) = detail3(&sr.evs, &sr.ex, &sr.tab, &list, cs, 0, usize::from(sm.n), tf, true, u16::try_from(sr.days.len()).unwrap_or(0));
                let _ = writeln!(md, "Decision (b): {ss}-HEADLINE-S's same account at the 10% share (the extreme worst case, beside): {} trades, net Rs {}, max DD Rs {}, net at the gate stress Rs {}, never-closed {} trades / {} units / Rs {}.", s10.trades, fmt_rs(s10.net), fmt_rs(s10.mdd), fmt_rs(s10.net_s2), s10.nc_trades, s10.nc_units, fmt_rs(s10.nc_net));
                if !sw::on(sw::PICKS32) {
                    picks.push((format!("{ss}-HEADLINE-S-AT-10PCT"), seg, Sum { x: u16::try_from(si * NL).unwrap_or(0), ..sm }));
                }
            }
            let _ = writeln!(md);
        }
    }
    std::fs::write(rdir.join("top-v3.tsv"), top).map_err(|e| e.to_string())?;
    // v3.2-B (addendum 18): every kept combo's split statistics and every fold's pick (SPLITS18)
    if sw::on(sw::SPLITS18) {
        let mut rows: Vec<(String, String, Sum)> = Vec::new();
        for (seg, sr) in segs.iter().enumerate() {
            for (g, t) in sr.tops.iter().enumerate() {
                for vw in 0..=crate::acct::VIEW_V {
                    for (i, s) in t.best(vw).iter().enumerate() {
                        rows.push((SEG_SHORT.get(seg).copied().unwrap_or("?").to_owned(), format!("group {g} view {} #{}", crate::acct::view_letter(vw), i + 1), *s));
                    }
                }
            }
        }
        let words = |ss: &str, s: &Sum| -> String {
            let seg = usize::from(ss == "stk");
            let Some(sr) = segs.get(seg) else { return "?".to_owned() };
            format!("{} / {} / {}% / N {}", crate::report::cfg_words(seg, sr.uls.get(s.list as usize).map_or(0, |u| u.0) as usize, &sr.lv), setting_words(menu, sets, sel, s.x), SHARE_BP.get(usize::from(s.x) % NL).copied().unwrap_or(0) / 100, crate::fix33::n_words(seg, usize::from(s.n)))
        };
        std::fs::write(rdir.join("splits18.tsv"), crate::split18::tsv(&rows, &words)).map_err(|e| e.to_string())?;
        let sels: Vec<(&str, &crate::split18::Sel18)> = segs.iter().enumerate().map(|(i, s)| (SEG_SHORT.get(i).copied().unwrap_or("?"), &s.sp18)).collect();
        std::fs::write(rdir.join("splits18-picks.tsv"), crate::split18::picks_tsv(&sels, &words)).map_err(|e| e.to_string())?;
    }
    // picks' trades (P5) and the P6 scans: decisions per pick (no data), then the trade rows day by day (D2: one day in memory at a time)
    let mut plans: Vec<(Vec<crate::kern::Dec3>, BTreeMap<&'static str, u64>, Sum)> = Vec::with_capacity(picks.len());
    for (_, seg, sm) in &picks {
        let sr = segs.get(*seg).ok_or("segment")?;
        let si = usize::from(sm.x) / NL;
        let l = usize::from(sm.x) % NL;
        let list = sr.ul(sm.list as usize).ok_or("list")?;
        let cs = csets.get(si).ok_or("setting")?;
        let tf = sr.uls.get(sm.list as usize).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) });
        let (dec, s2, _) = detail3(&sr.evs, &sr.ex, &sr.tab, &list, cs, l, usize::from(sm.n), tf, true, u16::try_from(sr.days.len()).unwrap_or(0));
        let mut why: BTreeMap<&'static str, u64> = BTreeMap::new();
        for d in &dec {
            *why.entry(d.code).or_insert(0) += 1;
        }
        plans.push((dec, why, s2));
    }
    let mut by_day: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
    for (k, (dec, _, _)) in plans.iter().enumerate() {
        let Some((_, seg, _)) = picks.get(k) else { continue };
        let Some(sr) = segs.get(*seg) else { continue };
        for (pos, d) in dec.iter().enumerate() {
            if d.code == "booked"
                && let Some(e) = sr.evs.get(d.eid as usize)
            {
                by_day.entry(usize::from(e.di)).or_default().push((k, pos));
            }
        }
    }
    let mut rows_of: Vec<Vec<(usize, PickRow)>> = (0..picks.len()).map(|_| Vec::new()).collect();
    for (di, list) in by_day {
        store.ensure(&[di])?;
        let (Some(c), Some(tp)) = (store.cubes.get(di), store.tapes.get(di)) else { return Err("day".to_owned()) };
        for (k, pos) in list {
            let (Some((name, seg, sm)), Some((dec, _, _))) = (picks.get(k), plans.get(k)) else { continue };
            let (Some(sr), Some(d)) = (segs.get(*seg), dec.get(pos)) else { continue };
            if let Some(r) = trade_row(name, pos, *seg, sr, sm, d, c, tp, dial, menu, sets, sel, csets, compat, cal)?
                && let Some(v) = rows_of.get_mut(k)
            {
                v.push((pos, r));
            }
        }
        store.release(&[]);
    }
    let mut tr = String::from(TRADES_HEAD);
    if v31 {
        tr.push_str(TRADES_HEAD_V31);
    }
    if sw::any32() {
        tr.push_str(if sw::on(sw::WHY12) { TRADES_HEAD_V32_RA } else { TRADES_HEAD_V32 });
    }
    tr.push('\n');
    let mut v32rows: Vec<(String, Vec<(crate::v32::Row32, String)>)> = Vec::new();
    let mut p5 = (0_u64, 0_u64, 0_u64);
    let (mut own_cut, mut sl_missing, mut lab_sum, mut booked) = (0_u64, 0_u64, 0_u64, 0_u64);
    let mut labels: BTreeMap<String, u64> = BTreeMap::new();
    let _ = writeln!(md, "## The picks' trades recomputed from scratch (proof P5) and the P6 scans");
    let _ = writeln!(md);
    let _ = writeln!(md, "| pick | booked trades | detail net Rs | sweep net Rs | trades whose sweep net differs from the detail | refusals |");
    let _ = writeln!(md, "|---|---|---|---|---|---|");
    let mut stress_md = String::new();
    for (k, (name, _seg, sm)) in picks.iter().enumerate() {
        let (Some((_, why, s2)), Some(rows)) = (plans.get(k), rows_of.get_mut(k)) else { continue };
        rows.sort_by_key(|x| x.0);
        let rows: Vec<&PickRow> = rows.iter().map(|x| &x.1).collect();
        let dn: i64 = rows.iter().map(|r| r.net).sum();
        let bad = rows.iter().filter(|r| r.sweep_minus_detail != 0).count();
        p5.0 += u64::try_from(rows.len()).unwrap_or(0);
        p5.1 += u64::try_from(bad).unwrap_or(0);
        // the 10%-share twin is a re-walk of its own (its sweep row is the headline account's): only its trades are checked
        let twin = name.ends_with("-AT-10PCT");
        if !twin && (s2.net != sm.net || s2.trades != sm.trades) {
            p5.2 += 1;
        }
        for r in &rows {
            booked += 1;
            lab_sum += 1;
            *labels.entry(r.label.clone()).or_insert(0) += 1;
            if r.own_in_cut_window {
                own_cut += 1;
            }
            if !r.search_sl_present {
                sl_missing += 1;
            }
            tr.push_str(&r.line);
            tr.push('\n');
        }
        if sw::any32() {
            v32rows.push((name.clone(), rows.iter().filter_map(|r| r.r32.clone().map(|x| (x, r.label.clone()))).collect()));
        }
        let refusals: Vec<String> = why.iter().filter(|(k2, _)| **k2 != "booked").map(|(k2, n)| format!("{k2} {n}")).collect();
        let _ = writeln!(md, "| {name} | {} | {} | {} | {bad} | {} |", rows.len(), fmt_rs(dn), fmt_rs(if twin { s2.net } else { sm.net }), refusals.join(", "));
        if v31 {
            // decision (c): the jump-stress table (a re-price of the booked trades, not a fill) and the adverse moves, for every pick
            let _ = writeln!(stress_md, "### {name} ({} trades; net Rs {}; never-closed {} trades / {} units / Rs {} valued, not filled)", rows.len(), fmt_rs(dn), rows.iter().filter(|r| r.x.nc_units > 0).count(), rows.iter().map(|r| r.x.nc_units).sum::<u64>(), fmt_rs(rows.iter().filter(|r| r.x.nc_units > 0).map(|r| r.net).sum()));
            let _ = writeln!(stress_md);
            let _ = writeln!(stress_md, "| k (every buy k% dearer; with both, every sale k% cheaper) | exits only: net Rs | worst trade Rs | max DD Rs | both sides: net Rs | worst trade Rs | max DD Rs |");
            let _ = writeln!(stress_md, "|---|---|---|---|---|---|---|");
            for &k_bp in std::iter::once(&0_i64).chain(crate::stress::KS_BP.iter()) {
                let mut cells: Vec<String> = Vec::new();
                for both in [false, true] {
                    let mut ev: Vec<(i64, i64)> = Vec::with_capacity(rows.len());
                    let mut worst = i64::MAX;
                    let mut net = 0_i64;
                    for r in &rows {
                        let n = crate::stress::stressed(&r.x.money, k_bp, both)?;
                        net += n;
                        worst = worst.min(n);
                        ev.push((r.x.known, n));
                    }
                    let dd = dd_grouped(&mut ev);
                    cells.extend([fmt_rs(net), if worst == i64::MAX || worst >= 0 { "none".to_owned() } else { fmt_rs(worst) }, fmt_rs(dd)]);
                }
                let _ = writeln!(stress_md, "| {}{} | {} |", if k_bp == 0 { "0 (as booked)".to_owned() } else { format!("{}%", k_bp / 100) }, "", cells.join(" | "));
            }
            let _ = writeln!(stress_md);
            let col = |f: &dyn Fn(&RowX) -> i64| -> String { let v: Vec<i64> = rows.iter().map(|r| f(&r.x)).collect(); let (a, b, c, d) = crate::stress::dist(&v); format!("{a} / {b} / {c} / {d}") };
            let _ = writeln!(stress_md, "Measured adverse move (bp of the last print before the decision; p50 / p90 / p99 / max over the {} trades): after the SALE decision 2 s {}, 5 s {}, 10 s {}; after the EXIT decision (own exit's anchor, else 15:10) 2 s {}, 5 s {}, 10 s {}. Positive at +{}.{:02}% adverse on both sides: {} (net Rs {}).", rows.len(), col(&|x| x.adv_sale[0]), col(&|x| x.adv_sale[1]), col(&|x| x.adv_sale[2]), col(&|x| x.adv_exit[0]), col(&|x| x.adv_exit[1]), col(&|x| x.adv_exit[2]), sw::GATE_BP.load(Ordering::Relaxed) / 100, sw::GATE_BP.load(Ordering::Relaxed) % 100, if rows.iter().map(|r| r.x.net_s2).sum::<i64>() > 0 { "YES" } else { "NO" }, fmt_rs(rows.iter().map(|r| r.x.net_s2).sum()));
            // matrix X03 / X05-X06 / X07-X17 / X11 / X25 and addendum 7, from the same rows
            let sl_n = rows.iter().filter(|r| r.x.sl_exit).count();
            let ov: Vec<i64> = rows.iter().filter_map(|r| r.x.overshoot_bp).collect();
            let (o50, _, o99, omax) = crate::stress::dist(&ov);
            let unf: Vec<&&PickRow> = rows.iter().filter(|r| r.x.own_exit && r.x.exit_unfilled > 0).collect();
            let worst = rows.iter().map(|r| r.x.loss_slot_bp).max().unwrap_or(0);
            let ls: Vec<i64> = rows.iter().map(|r| r.x.loss_slot_bp).collect();
            let (_, _, l99, _) = crate::stress::dist(&ls);
            let big10 = rows.iter().filter(|r| r.x.loss_slot_bp > 1_000).count();
            let big_acct = rows.iter().filter(|r| -r.net > trade::ACCOUNT / 200).count();
            let (ed, od): (Vec<&&PickRow>, Vec<&&PickRow>) = rows.iter().partition(|r| r.x.expiry_day);
            let sl_rate = |v: &[&&PickRow]| if v.is_empty() { "-".to_owned() } else { format!("{}%", v.iter().filter(|r| r.x.sl_exit).count() * 100 / v.len()) };
            let ladder: Vec<String> = [2_500_i64, 5_000, 10_000, 20_000, 50_000, 100_000, 200_000].iter().map(|&t| format!(">= {}%: {}", t / 100, rows.iter().filter(|r| r.x.respike_bp >= t).count())).collect();
            let rmax = rows.iter().map(|r| r.x.respike_bp).max().unwrap_or(0);
            let (cs_, cb_): (i64, i64) = (rows.iter().map(|r| r.x.charges_sell).sum(), rows.iter().map(|r| r.x.charges_buy).sum());
            let prem: i64 = rows.iter().map(|r| r.x.money.sold).sum();
            if sw::on(sw::WHY12) {
                // REPAIR A (check-engine-1 S4): signed percentages printed right (h6 printed 0.-39% for a fill 0.39% BELOW the level)
                let pc = |b: i64| format!("{}{}.{:02}%", if b < 0 { "-" } else { "" }, b.abs() / 100, b.abs() % 100);
                let _ = writeln!(stress_md, "- Gap past the stop (X03): {sl_n} SL exits; {} with a buy-back fill above the level; the first buy-back fill vs the level, p50 {} / p99 {} / max {} of the sale price (negative = below the level).", ov.iter().filter(|&&b| b > 0).count(), pc(o50), pc(o99), pc(omax));
            } else {
            let _ = writeln!(stress_md, "- Gap past the stop (X03): {sl_n} SL exits; {} with a buy-back fill above the level by p50 {}.{:02}% / p99 {}.{:02}% / max {}.{:02}% of the sale price.", ov.iter().filter(|&&b| b > 0).count(), o50 / 100, o50 % 100, o99 / 100, o99 % 100, omax / 100, omax % 100);
            }
            let _ = writeln!(stress_md, "- Own exit not bought back by its own order (X05 / X06): {} trades, {} units, net Rs {} (the re-send switch: {}).", unf.len(), unf.iter().map(|r| r.x.exit_unfilled).sum::<u64>(), fmt_rs(unf.iter().map(|r| r.net).sum()), if sw::on(sw::RESEND) { "ON, each exit re-sent every block until filled" } else { "off" });
            let _ = writeln!(stress_md, "- Tail (X07 / X17, a labelled reading until MAIN rules a gate): worst trade {}.{:02}% of its slot; p99 trade loss {}.{:02}% of the slot; trades losing more than 10% of their slot: {big10}; more than 0.5% of the account (Rs 10,000): {big_acct}.", worst / 100, worst % 100, l99 / 100, l99 % 100);
            let _ = writeln!(stress_md, "- Expiry day (X11): on the option's own expiry day {} trades, net Rs {}, SL exits {}; other days {} trades, net Rs {}, SL exits {} (no expiry-day dial in v3.1).", ed.len(), fmt_rs(ed.iter().map(|r| r.net).sum()), sl_rate(&ed), od.len(), fmt_rs(od.iter().map(|r| r.net).sum()), sl_rate(&od));
            let _ = writeln!(stress_md, "- Re-spikes (X25 / addendum 5): trades whose price rose above the sale before the last fill: {}; the worst +{}.{:02}%.", ladder.join(", "), rmax / 100, rmax % 100);
            let _ = writeln!(stress_md, "- Charges (addendum 7): sale side Rs {}, buy side Rs {}, total Rs {} = {}.{:02}% of the premium collected; Rs {} a trade (per-component split: open item).", fmt_rs(cs_), fmt_rs(cb_), fmt_rs(cs_ + cb_), if prem > 0 { (cs_ + cb_) * 100 / prem } else { 0 }, if prem > 0 { ((cs_ + cb_) * 10_000 / prem) % 100 } else { 0 }, fmt_rs(if rows.is_empty() { 0 } else { (cs_ + cb_) / i64::try_from(rows.len()).unwrap_or(1) }));
            let _ = writeln!(stress_md);
        }
    }
    std::fs::write(rdir.join("trades-v3.tsv"), tr).map_err(|e| e.to_string())?;
    let _ = writeln!(md);
    let _ = writeln!(md, "- **P5 (sweep vs detail)**: {} pick trades recomputed from scratch (THE LAW's triggers, each trade's own sale record): {} differ from the sweep; account summaries differing from the detail run: {}.", p5.0, p5.1, p5.2);
    let _ = writeln!(md, "- **P6 scans**: own orders anchored in [15:08, 15:10): {own_cut}; booked trades {booked}, exit reasons summed {lab_sum} ({}); trades without their SL present: {sl_missing}; 1-paisa floors and absences per rule: `stageA-rules.tsv`.", labels.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", "));
    lg!(log, "P5: {} pick trades, {} differ, {} summaries differ; P6: own orders in [15:08, 15:10) {own_cut}, trades {booked} = reasons {lab_sum}, SL missing {sl_missing}", p5.0, p5.1, p5.2);
    let _ = writeln!(md);
    if v31 {
        let _ = writeln!(md, "## Jump stress (decision (c), addendum 3: a STRESS of the booked trades, never a fill) and the measured adverse moves, every pick");
        let _ = writeln!(md);
        md.push_str(&stress_md);
    }
    if sw::any32() {
        md.push_str(&v32_sections(segs, sets, sel, csets, menu, days, &v32rows, log, rdir)?);
    }
    if v31 && !sw::any32() {
        let _ = writeln!(md, "## The Dhan-live checklist (addendum 9): COVERED with its evidence, or NOT COVERED with the reason (any NOT COVERED item blocks the word headline)");
        let _ = writeln!(md);
        let _ = writeln!(md, "| # | item | state | evidence / reason |");
        let _ = writeln!(md, "|---|---|---|---|");
        let cov = |b: bool| if b { "COVERED" } else { "NOT COVERED" };
        for (i, (item, ok, ev)) in [
            ("Signal at candle close, order after it (REQ 56); live timing W/L (REQ 69)", false, "REQ 56 COVERED (fills from the block after the candle end; v3 proof P4/P5); REQ 69 W/L latency dial NOT built in this engine"),
            ("Fills: REQ 70 blocks (index 1 s, stock 2 s), worst print, share 25% headline / 10% beside, carried entitlement, rest to the next block", sw::on(sw::CARRY) && sw::on(sw::HEADLINE), "crate v2.3 K1 (tests_v23 carry tests); headline gate share <= 25%; the 10% twin printed beside"),
            ("Slippage: worst prints + jump stress 1-100% + the +2% gate + the gap overshoot of every stop", sw::on(sw::HEADLINE), "the jump-stress tables and the X03 overshoot line per pick"),
            ("Bid-ask spread (no quotes in GDFL): covered by the +2% gate", sw::on(sw::HEADLINE), "headline gate; Tickvault quotes beside: NOT read by this engine"),
            ("Market protection and price bands: exits re-sent until filled; a band lock gives no fill", false, "re-send COVERED (crate v2.3 K2, tests_v23 v31_resend_buyback_chases_a_runaway_price); band lock NOT COVERED: no per-contract-day band data (matrix X10)"),
            ("Stops: REQ 47 small stops only; checked every second; unfilled exits keep chasing (re-send); breakeven / trail", sw::on(sw::HEADLINE) && sw::on(sw::RESEND), "SEARCH role (R1) + headline gate; per-second xrule (crate v2.2); K2 re-send for every own exit's rest"),
            ("Margin rule v2 (SPAN + option value + exposure on the near-month future / stock price + Rs 20 per order); the Rs 20 L cap with the running MTM loss", sw::on(sw::MARGIN_EXACT) && sw::on(sw::MARGIN_V2) && sw::on(sw::MTM), "margincheck2: 782/782 + 15/15 Dhan screens; margincheck rule v1 782/782; MTM refusals per row"),
            ("Charges: the Dhan estimator per component per side at dated rates, per trade", false, "per side per trade COVERED (trades-v3.tsv charges_per_side, the totals line per pick); per-COMPONENT split NOT printed yet"),
            ("Square-off: our own 15:10 order; orders end 15:30; never-closed units valued from the exit decision; Dhan's fee only for units Dhan squares off", sw::on(sw::OWN_SQUAREOFF) && sw::on(sw::END_1530) && sw::on(sw::VALUE_FROM_EXIT), "switches own square-off, close 1530, never-closed from-exit (tests31)"),
            ("Expiry and ban rules: stock options never on their own expiry day; index expiry-day trading allowed; F&O ban days", false, "expiry rules COVERED (v2 rule R3); F&O BAN days NOT applied by run3"),
            ("Account: slot Rs 20 L / N; lost money never reused; no compounding; one position per underlying + side + expiry", true, "kern.rs step3 (v2.1 rules, unchanged)"),
            ("Freeze-quantity slicing; whole lots", false, "whole lots COVERED; freeze slicing NOT COVERED (NSE's dated freeze files not on disk)"),
            ("Greeks at entry and exit", false, "NOT COVERED: the app's Greeks code (crates/greeks, /greeks.json) is not linked into this engine"),
            ("Data precision: the store / cube / NSE-file proofs", true, "cube proofs (CUBE-FORMAT, v2.1 checks); NSE tables: margin-history proofs (a)-(c)"),
            ("Labels: underlying, signal, traded, size, period, run, proof step, hindsight, walk-forward", true, "the label block at the top of this file"),
            ("Run-up / run-down and re-spikes per trade (REQ 48, addendum 5)", false, "re-spike ladder and adverse 2/5/10 s COVERED per pick; per-trade run-up / closest approach / stop hunts NOT printed"),
            ("Every BLOCKING / HIGH case of EXTREME-MATRIX-SELLING.md with its named test", false, "partly: see the builder's matrix coverage list (v31/progress.md); several HIGH rows need data not on disk (bands, halts, events, bhavcopy HIGH)"),
        ].iter().enumerate() {
            let _ = writeln!(md, "| {} | {item} | {} | {ev} |", i + 1, cov(*ok));
        }
        let _ = writeln!(md);
    }
    if sw::any32() {
        md.push_str(&v32_checklist());
    }
    let _ = writeln!(md, "Files: `top-v3.tsv` (best 50 per group x view), `trades-v3.tsv` (every pick trade: exit reason, trigger second, anchor, level, every rule's distance, flags CUT_HELD / REWALKED / PROTECTED / UNCLOSABLE, the liquidity readings), `liquidity.tsv`, `stageA-rules.tsv`, `RUN.log`.");
    Ok(md)
}

/// A group's printed name (the v3.1 HEADLINE group beyond v3's eight).
fn group_name(g: usize) -> &'static str {
    GROUP_NAMES.get(g).copied().unwrap_or(if g == NGROUP { "HEADLINE (SEARCH small stop, share <= 25%, positive at the gate stress)" } else { "?" })
}



/// Proof P0's replay context: the v3 machinery (readings, stage A with EVERY menu rule under the v2 compatibility cut, the v3 kernel) for
/// the S0 settings in the v2.1 engine's account order (x = ((k x 5 + j) x 5 + m) x 5 + l, account = x x 4 + N).
pub struct Replay {
    pub sr: Seg3,
    /// The S0 settings compiled, indexed by (k x 5 + j) x 5 + m.
    pub cs: Vec<CSet>,
    pub log: String,
}

impl Replay {
    /// Builds the replay of one segment from the v2.1 run's unique events and config lists.
    ///
    /// # Errors
    /// A refused input or a crate refusal.
    #[allow(clippy::too_many_arguments)]
    pub fn new(cubes: &[Cube], tapes: &[SecTape], var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, menu_p: &Path, sets_p: &Path, hol: &[&Path], seg: u8, evs: &[SEv], lists: &[Vec<u32>], days: &[usize], threads: usize) -> Result<Self, String> {
        let mut log = String::new();
        let menu = Menu::load(menu_p)?;
        let sets = Sets::load(sets_p, &menu)?;
        let ma = MenuA::new(&menu.rules)?;
        let mut cs: Vec<CSet> = vec![CSet::default(); 125];
        let mut n = 0;
        for s in &sets.settings {
            if let Some((k, j, m)) = s.v2
                && let Some(slot) = cs.get_mut((k * 5 + j) * 5 + m)
            {
                *slot = compile(&ma, s);
                n += 1;
            }
        }
        if n != 125 {
            return Err(format!("S0 settings {n}, want 125"));
        }
        let mut cal3 = Cal3::load(hol)?;
        cal3.add_specials(cubes);
        let mut tfmask = vec![0_u16; evs.len()];
        for (id, l) in lists.iter().enumerate() {
            let tf = cfg_parts(id).0;
            for &e in l {
                if let Some(m) = tfmask.get_mut(e as usize) {
                    *m |= 1 << tf;
                }
            }
        }
        let mut rd = vec![Rd::default(); evs.len()];
        let mut tfw: Vec<Vec<(u8, i64, i64)>> = vec![Vec::new(); evs.len()];
        let mut liq = vec![(0_u32, 0_u32); evs.len()];
        let mut by: BTreeMap<(u16, u32), Vec<usize>> = BTreeMap::new();
        for (i, e) in evs.iter().enumerate() {
            by.entry((e.di, e.ci)).or_default().push(i);
        }
        for ((di, ci), ids) in &by {
            let (Some(c), Some(tp)) = (cubes.get(usize::from(*di)), tapes.get(usize::from(*di))) else { continue };
            let pr = Prep::new(c, *ci as usize, tp.of(*ci as usize));
            for &i in ids {
                let Some(e) = evs.get(i) else { continue };
                let t = i64::from(e.t);
                if let (Some(a), Some(b), Some(w)) = (rd.get_mut(i), liq.get_mut(i), tfw.get_mut(i)) {
                    *a = base_reading(c, *ci as usize, t, &pr, &cal3);
                    *b = pr.liq_at(t);
                    let m = tfmask.get(i).copied().unwrap_or(0);
                    *w = (0..TFS.len()).filter(|&tf| m & (1 << tf) != 0).map(|tf| { let (h, r) = window_reading(c, *ci as usize, t, TFS.get(tf).copied().unwrap_or(1)); (u8::try_from(tf).unwrap_or(0), h, r) }).collect();
                }
            }
        }
        let mut sr = Seg3 { evs: evs.iter().map(|e| SEv { tab: trade::EvT::default(), ..e.clone() }).collect(), ex: vec![EvX::default(); evs.len()], rd, tfw, liq, tfmask, days: days.to_vec(), ..Seg3::default() };
        let t0 = Instant::now();
        stage_a_whole(&mut sr, cubes, tapes, var, dial, &ma, seg, true, threads)?;
        lg!(log, "P0 replay ({}): menu {} sha256 {} / sets sha256 {}; stage A with all {} rules under the v2 compatibility cut: {} events, {} paths, {} distinct exit keys, {:.1} s", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), menu.len(), menu.sha, sets.sha, ma.rows.len(), sr.evs.len(), sr.cnt.paths, sr.tab.outs.len(), t0.elapsed().as_secs_f64());
        Ok(Self { sr, cs, log })
    }

    /// Every account of one list in the v2.1 engine's order (2,500), by the PRODUCTION kernel of `run3` (the event-major chunk kernel with
    /// exact event skipping): the chunk index (setting x 5 + l) x 4 + N equals the v2.1 account index x x 4 + N with x = setting x 5 + l.
    #[must_use]
    pub fn sweep(&self, list: &[u32], ndays: u16, seg: u8, lid: u32) -> Vec<Sum> {
        let keys = list_keys(&self.sr.evs, list);
        let (rows, _) = crate::kern::walk_chunk(&self.sr.evs, &self.sr.ex, &self.sr.tab, list, &keys, &self.cs, None, true, true, &[], 0, ndays);
        rows.into_iter()
            .enumerate()
            .map(|(a, (mut sm, _))| {
                sm.seg = seg;
                sm.list = lid;
                sm.x = u16::try_from(a / 4).unwrap_or(0);
                sm
            })
            .collect()
    }
}

/// Field-by-field equality of two account summaries (P0).
#[must_use]
pub fn sum_eq(a: &Sum, b: &Sum) -> bool {
    (a.seg, a.list, a.x, a.n, a.net, a.trades, a.wins, a.mdd) == (b.seg, b.list, b.x, b.n, b.net, b.trades, b.wins, b.mdd) && (a.worst, a.worst_day, a.days_pos, a.days_neg, a.days_traded, a.ndays, a.spike, a.pk_margin) == (b.worst, b.worst_day, b.days_pos, b.days_neg, b.days_traded, b.ndays, b.spike, b.pk_margin)
}

/// The IST clock (from `date`).
#[must_use]
pub fn now() -> String {
    now_ist()
}

/// Proof P4's sample (EXITS-DYNAMIC-DESIGN 7 B3 P4: 30 real trades, one per exit kind and extreme case): per target (a setting of one or
/// two menu rules and the case it must show), the first event path (share 100% or 10%, the lot count of N = 10) where the stage A anchors
/// (or the trade itself) show the case; the trade recomputed by `trade3_once` (THE LAW) with every input the node hand check needs.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn hand_sample(segs: &[Seg3], store: &mut Store, dial: &common_fill::Dial, menu: &Menu, compat: bool, rdir: &Path, log: &mut String) -> Result<(), String> {
    let targets: [(&str, &[&str], &str); 32] = [
        ("SL-P", &["SL.P.5"], "w0"),
        ("SL-A1", &["SL.A1.50"], "w0"),
        ("SL-A5", &["SL.A5.200"], "w0"),
        ("SL-H", &["SL.H.2"], "w0"),
        ("SL-S", &["SL.S.25"], "w0"),
        ("SL-V", &["SL.V.100"], "w0"),
        ("GO", &["SL.P.100", "GO.180"], "w1"),
        ("BE", &["SL.P.100", "BE.20"], "w1"),
        ("BE-lock", &["SL.P.100", "BE.30L"], "w1"),
        ("TT", &["SL.P.100", "TT.15"], "w1"),
        ("TSL-fixed", &["SL.P.100", "TS.P.10.10"], "w1"),
        ("TSL-relative", &["SL.P.100", "TS.R.10.25"], "w1"),
        ("TSL-ATR", &["SL.P.100", "TS.A.50.50"], "w1"),
        ("TP-P", &["SL.P.100", "TP.P.30"], "w1"),
        ("TP-A", &["SL.P.100", "TP.A.100"], "w1"),
        ("TP-F", &["SL.P.100", "TP.F.100"], "w1"),
        ("TP-theta", &["SL.P.100", "TP.T.60"], "w1"),
        ("TTP-fixed", &["SL.P.100", "TTP.P.50.5"], "w1"),
        ("TTP-relative", &["SL.P.100", "TTP.R.50.25"], "w1"),
        ("TIME", &["SL.P.100", "TM.5"], "w1"),
        ("HELD-B2", &["SL.P.5", "TM.1"], "held"),
        ("SQUAREOFF", &["SL.P.100"], "none"),
        ("GAPPED-STOP", &["SL.P.5"], "gap"),
        ("MULTI-PIECE-R", &["SL.P.10"], "multi"),
        ("UNCLOSABLE", &["SL.P.100"], "unclosed"),
        ("TP-FLOOR-1-PAISA", &["SL.P.100", "TP.P.10"], "floor"),
        ("TSL-LATER-PIECE", &["SL.P.100", "TS.P.0.5"], "trail_later"),
        ("BE-INDEX-1S", &["SL.P.100", "BE.10"], "w1"),
        ("SL-P-2PCT", &["SL.P.2"], "w0"),
        ("TP-P-90", &["SL.P.100", "TP.P.90"], "w1"),
        ("HELD-B2-INDEX-1S", &["SL.P.5", "TM.1"], "held"),
        ("TP-THETA-ON-EXPIRY", &["SL.P.100", "TP.T.15"], "w1_expiry"),
    ];
    let idx_of = |id: &str| menu.rules.iter().position(|r| r.id == id || r.aliases.iter().any(|a| a == id));
    let mut out = String::from("target\tseg\tday\tsid\tci\tsymbol\tspot_seg\tspot_sid\tstrike\tright\texpiry\tlot\tT\ttf\tshare_bp\tqty\tblock\trules\tatr1_x1000\tatr5_x1000\tv300_s\tv300_m\thwin\trise\ttheta_tv\ttheta_mt\txrules\tp0\te0\te1\tfills\twinner\ttrigger_s\tanchor_s\tkey\tlevel\tunits\tsold\tbb_units\tbb_value\tbb_first_px\tsq_units\tsq_value\tunclosed\tworst\tcharges\tnet\tlimit_in\tlimit_out\n");
    let mut found = 0;
    for (ti, (name, ids, cond)) in targets.iter().enumerate() {
        let rows: Vec<usize> = ids.iter().filter_map(|i| idx_of(i)).collect();
        if rows.len() != ids.len() {
            return Err(format!("hand target {name}: unknown rule"));
        }
        let order: [usize; 2] = if name.contains("INDEX") || name.contains("EXPIRY") || ti % 2 == 1 { [0, 1] } else { [1, 0] };
        let mut done = false;
        'seg: for &seg in &order {
            let Some(sr) = segs.get(seg) else { continue };
            let blk = block_of(u8::try_from(seg).unwrap_or(0));
            for (eid, e) in sr.evs.iter().enumerate() {
                if e.tab.refuse != Refuse::None || e.tab.qs.is_empty() {
                    continue;
                }
                let Some(x) = sr.ex.get(eid) else { continue };
                let q = usize::from(e.tab.q_of.get(1).copied().unwrap_or(255));
                if q == 255 || x.nq == 0 {
                    continue;
                }
                // D2: the streamed mode loads the event's day here (one day at a time)
                if store.streamed {
                    store.ensure(&[usize::from(e.di)])?;
                    store.release(&[usize::from(e.di)]);
                }
                let (cubes, tapes) = (&store.cubes, &store.tapes);
                for l in [4_usize, 0] {
                    if !e.tab.filled.get(l).copied().unwrap_or(false) {
                        continue;
                    }
                    let ri = x.row(l, q);
                    let slot0 = if x.tf.is_empty() { None } else { Some(0) };
                    let anc: Vec<u16> = rows.iter().map(|&r| match sr.ex.get(eid).and_then(|_| ma_tf(segs, seg, r)) {
                        Some(j) => slot0.and_then(|s| sr.tab.tanc.get(x.trow(s, l, q))).and_then(|a| a.get(j)).copied().unwrap_or(ABSENT),
                        None => sr.tab.anc.get(ri).and_then(|a| a.get(r)).copied().unwrap_or(ABSENT),
                    }).collect();
                    if anc.first().is_none_or(|&a| a == ABSENT) {
                        continue;
                    }
                    let hit = crate::stage::first_hit(&anc);
                    let quick = match *cond {
                        "w0" => hit.is_some_and(|(w, a)| w == 0 && i64::from(a) + trade::BASE_S < crate::cal::CUT_S),
                        "w1" => hit.is_some_and(|(w, a)| w == 1 && i64::from(a) + trade::BASE_S < crate::cal::CUT_S),
                        "held" => hit.is_some_and(|(_, a)| { let s = i64::from(a) + trade::BASE_S; (crate::cal::CUT_S..crate::cal::SQ_S).contains(&s) }),
                        "none" => hit.is_none(),
                        "gap" | "multi" => hit.is_some_and(|(_, a)| i64::from(a) + trade::BASE_S < crate::cal::CUT_S),
                        "unclosed" => true,
                        "floor" | "trail_later" => hit.is_some_and(|(w, a)| w == 1 && i64::from(a) + trade::BASE_S < crate::cal::CUT_S),
                        "w1_expiry" => hit.is_some_and(|(w, a)| w == 1 && i64::from(a) + trade::BASE_S < crate::cal::CUT_S) && cubes.get(usize::from(e.di)).and_then(|c| c.cons.get(e.ci as usize).map(|k| k.expiry == c.ymd)).unwrap_or(false),
                        _ => false,
                    };
                    if !quick {
                        continue;
                    }
                    let Some(c) = cubes.get(usize::from(e.di)) else { continue };
                    let Some(kc) = c.cons.get(e.ci as usize) else { continue };
                    let tp = tapes.get(usize::from(e.di)).ok_or("tape")?;
                    let dt = DayTape::new(c.day, tp.of(e.ci as usize), blk)?;
                    let tape = dt.tape()?;
                    let lot = u64::from(kc.lot);
                    let lots = e.tab.qs.get(q).copied().unwrap_or(0);
                    let qty = u64::from(lots) * lot;
                    let p0 = e.tab.p0.get(l).copied().unwrap_or(0);
                    let base = sr.rd.get(eid).copied().unwrap_or_default();
                    let w = sr.tfw.get(eid).and_then(|v| v.first()).copied();
                    let rd = w.map_or(base, |(_, h, r)| Rd { hwin: h, rise: r, ..base });
                    let rules: Vec<Result<(common_fill::xrule::XRule, bool), u8>> = rows.iter().map(|&r| menu.rules.get(r).map_or(Err(u8::MAX), |m| rule_at(m, &rd, p0))).collect();
                    let Some((r, wpos, a, key, tsec, level)) = trade3_once(&tape, c.day, e.t, lot, qty, l, &rules, compat, dial)? else { continue };
                    let paths = trade::record_base(&tape, c.day, i64::from(e.t), lot, qty, dial).ok();
                    let fq: Vec<common_fill::BlockFill> = paths.as_ref().map(|p| trade::truncate(p.fills.get(l).map_or(&[][..], Vec::as_slice), qty)).unwrap_or_default();
                    let ok = match *cond {
                        "gap" => matches!(key, XKey::Own(..)) && r.bb_first_px > level && wpos == Some(0),
                        "multi" => fq.len() >= 2 && fq.get(1).is_some_and(|b| tsec > crate::cal::sod_of(b.block.at)),
                        "unclosed" => r.unclosed > 0,
                        "floor" => rules.get(1).is_some_and(|x| matches!(x, Ok((_, true)))),
                        "trail_later" => fq.len() >= 2,
                        _ => true,
                    };
                    if !ok {
                        continue;
                    }
                    let und = c.unds.get(usize::from(kc.und));
                    let th = rd.theta.map_or((0, 0), |x| x);
                    let fills = fq.iter().map(|b| format!("{}:{}:{}", crate::cal::sod_of(b.block.at), b.units, b.px)).collect::<Vec<_>>().join(";");
                    let xr = rules.iter().map(|x| match x { Ok((z, f)) => format!("{z:?}{}", if *f { "[floor]" } else { "" }), Err(cc) => format!("ABSENT({})", absent_word(*cc)) }).collect::<Vec<_>>().join(" | ");
                    let keyw = match key {
                        XKey::None => "none".to_owned(),
                        XKey::Own(s, _) => format!("own@{s}"),
                        XKey::Held(s) => format!("held@{s}"),
                    };
                    let _ = writeln!(out, "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{lot}\t{}\t{}\t{}\t{qty}\t{blk}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{xr}\t{p0}\t{}\t{}\t{fills}\t{}\t{tsec}\t{}\t{keyw}\t{level}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        SEG_SHORT.get(seg).copied().unwrap_or("?"), fmt_day(c.day), kc.sid, e.ci, kc.symbol, und.map_or(255, |u| u.spot_seg), und.map_or(0, |u| u.spot_sid), kc.strike, if kc.right == 0 { "CE" } else { "PE" }, kc.expiry, e.t, w.map_or(255, |z| z.0), SHARE_BP.get(l).copied().unwrap_or(0), ids.join("+"),
                        rd.atr1.map_or(-1, |v| v), rd.atr5.map_or(-1, |v| v), rd.v300.map_or(0, |v| v.0), rd.v300.map_or(0, |v| v.1), rd.hwin, rd.rise, th.0, th.1,
                        crate::cal::sod_of(r.e0), crate::cal::sod_of(r.e1), wpos.map_or(-1, |z| i64::try_from(z).unwrap_or(-1)), if a < NEVER3 { i64::from(a) + trade::BASE_S } else { -1 },
                        r.units, r.sold_value, r.bb_units, r.bb_value, r.bb_first_px, r.sq_units, r.sq_value, r.unclosed, r.worst, r.charges, r.net, r.limit_in, r.limit_out);
                    found += 1;
                    done = true;
                    break 'seg;
                }
            }
        }
        if !done {
            lg!(log, "P4 sample: target {name} ({}) has no case in these days", ids.join("+"));
        }
    }
    std::fs::write(rdir.join("hand-sample.tsv"), out).map_err(|e| e.to_string())?;
    lg!(log, "P4 sample: {found} of {} targets found -> hand-sample.tsv", targets.len());
    Ok(())
}

/// The tf-dependent index of menu row `r` (None: tf-independent) (the hand sample reads the segment's stage A menu).
fn ma_tf(_segs: &[Seg3], _seg: usize, r: usize) -> Option<usize> {
    TFDEP.with(|t| t.borrow().get(r).copied().flatten().map(usize::from))
}

thread_local! {
    static TFDEP: std::cell::RefCell<Vec<Option<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Sets the tf-dependent map the hand sample reads.
pub fn set_tfdep(ma: &MenuA) {
    TFDEP.with(|t| *t.borrow_mut() = ma.tfdep.clone());
}

/// Speed-up 4 measured (P7): on every 10th unique list, every (share level, slot count): how many DISTINCT decision sequences the selected
/// settings have (exact dedup groups, full compare): the share of accounts a dedup kernel would walk.
fn dedup_ratio(sr: &Seg3, csets: &[CSet], threads: usize, log: &mut String) {
    let t0 = Instant::now();
    let next = AtomicUsize::new(0);
    let tot: Mutex<(u64, u64)> = Mutex::new((0, 0));
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let (mut all, mut distinct) = (0_u64, 0_u64);
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let u = k * 10;
                    let Some(list) = sr.ul_evs.get(u) else { break };
                    let tf = sr.uls.get(u).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) });
                    for l in 0..NL {
                        for ni in 0..4 {
                            let rep = crate::kern::dedup_groups(&sr.evs, &sr.ex, &sr.tab, list, csets, l, ni, tf, crate::kern::fnv);
                            all += u64::try_from(rep.len()).unwrap_or(0);
                            distinct += u64::try_from(rep.iter().enumerate().filter(|(i, r)| i == *r).count()).unwrap_or(0);
                        }
                    }
                }
                if let Ok(mut t) = tot.lock() {
                    t.0 += all;
                    t.1 += distinct;
                }
            });
        }
    });
    let (all, distinct) = tot.into_inner().unwrap_or_default();
    lg!(log, "DEDUP-RATIO (speed-up 4, exact, sampled: every 10th list x 5 share levels x 4 slot counts): {all} accounts, {distinct} distinct decision sequences ({:.1}% would be walked); {:.1} s", if all > 0 { distinct as f64 * 100.0 / all as f64 } else { 0.0 }, t0.elapsed().as_secs_f64());
}

/// REPAIR A (speed, check-engine-2 S6): the SPEED PROBE on the run's own data (`--speed-probe K`): every K-th unique list (natural order) is
/// walked twice: the plain stage B kernel (every setting, 64-setting chunks as stage B) and SPEED-UP 4 (exact setting dedup over every
/// setting of the list) in two phases (all sampled lists plain, then all with SPEED-UP 4), each phase timed by the PROCESS CPU (every thread);
/// every probed list's digest of all its accounts' summaries and pieces must agree between the phases (exactness on this slice). Returns (lists probed, accounts, plain steps, plain core-ns, dedup walked accounts, dedup steps, dedup core-ns,
/// summaries differing, events in the probed lists).
fn speed_probe(sr: &Seg3, csets: &[CSet], k: usize, threads: usize, skip: bool) -> (u64, u64, u64, u128, u64, u64, u128, u64, u64) {
    let ndays = u16::try_from(sr.days.len()).unwrap_or(0);
    let sample: Vec<usize> = (0..sr.uls.len()).filter(|u| u % k.max(1) == 0).collect();
    // one walk of every sampled list (the plain chunk kernel, or SPEED-UP 4): per list (digest of every walked account's summary and pieces,
    // accounts, steps, walked accounts), the phase's PROCESS CPU (ps cputime before and after: every thread's CPU, not wall)
    let phase = |dedup: bool| -> (Vec<(u64, u64, u64, u64)>, u128) {
        let next = AtomicUsize::new(0);
        let res: Mutex<Vec<(usize, (u64, u64, u64, u64))>> = Mutex::new(Vec::new());
        let c0 = proc_cpu_s();
        std::thread::scope(|s| {
            for _ in 0..threads.max(1) {
                s.spawn(|| {
                    let mut mine: Vec<(usize, (u64, u64, u64, u64))> = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&u) = sample.get(i) else { break };
                        let Some(list) = sr.ul_evs.get(u) else { continue };
                        let keys = list_keys(&sr.evs, list);
                        let tf = sr.uls.get(u).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) });
                        let (rows, steps, walked) = if dedup {
                            crate::kern::walk_dedup(&sr.evs, &sr.ex, &sr.tab, list, &keys, csets, tf, true, skip, &[], 0, ndays, None)
                        } else {
                            let mut v: Vec<(Sum, Pieces)> = Vec::with_capacity(csets.len() * NL * 4);
                            let mut st = 0_u64;
                            for c0 in (0..csets.len()).step_by(64) {
                                let c1 = (c0 + 64).min(csets.len());
                                let (a, s1) = crate::kern::walk_chunk(&sr.evs, &sr.ex, &sr.tab, list, &keys, csets.get(c0..c1).unwrap_or(&[]), tf, true, skip, &[], 0, ndays);
                                v.extend(a);
                                st += s1;
                            }
                            let w = u64::try_from(v.len()).unwrap_or(0);
                            (v, st, w)
                        };
                        let mut h = 0xcbf2_9ce4_8422_2325_u64;
                        let mut accts = 0_u64;
                        for (a, (sm, pc)) in rows.iter().enumerate() {
                            if !trade::share_on((a % (NL * 4)) / 4) {
                                continue;
                            }
                            accts += 1;
                            for b in format!("{sm:?}{:?}", pc.p).bytes() {
                                h ^= u64::from(b);
                                h = h.wrapping_mul(0x0100_0000_01b3);
                            }
                        }
                        mine.push((i, (h, accts, steps, walked)));
                    }
                    if let Ok(mut g) = res.lock() {
                        g.extend(mine);
                    }
                });
            }
        });
        let ns = ((proc_cpu_s() - c0).max(0.0) * 1e9) as u128;
        let mut v = res.into_inner().unwrap_or_default();
        v.sort_by_key(|x| x.0);
        (v.into_iter().map(|x| x.1).collect(), ns)
    };
    let (p, ns1) = phase(false);
    let (d, ns2) = phase(true);
    let bad = u64::try_from(p.iter().zip(&d).filter(|(a, b)| a.0 != b.0 || a.1 != b.1).count()).unwrap_or(0) + u64::from(p.len() != d.len());
    let accts: u64 = p.iter().map(|x| x.1).sum();
    let s1: u64 = p.iter().map(|x| x.2).sum();
    let s2: u64 = d.iter().map(|x| x.2).sum();
    let walked: u64 = d.iter().map(|x| x.3).sum();
    let evn: u64 = sample.iter().filter_map(|&u| sr.ul_evs.get(u)).map(|l| u64::try_from(l.len()).unwrap_or(0)).sum();
    (u64::try_from(p.len()).unwrap_or(0), accts, s1, ns1, walked, s2, ns2, bad, evn)
}

/// The process's CPU seconds so far (every thread; `ps -o time=`, 10 ms resolution): the speed probe's per-phase CPU.
fn proc_cpu_s() -> f64 {
    // v3.4 E5: under PORTABLE /proc when it exists (Linux), else ps by the tool path (labels and MEASURED timing only)
    if let Some(c) = crate::kit::proc_cpu_s() {
        return c;
    }
    let pid = std::process::id().to_string();
    let Ok(o) = std::process::Command::new(crate::kit::tool("ps")).args(["-o", "time=", "-p", &pid]).output() else { return 0.0 };
    let t = String::from_utf8_lossy(&o.stdout).trim().to_owned();
    let parts: Vec<f64> = t.split(':').filter_map(|x| x.trim().parse::<f64>().ok()).collect();
    match parts.as_slice() {
        [m, s] => m * 60.0 + s,
        [h, m, s] => h * 3_600.0 + m * 60.0 + s,
        [s] => *s,
        _ => 0.0,
    }
}

/// Every field of two account summaries equal (the speed-up proofs: nothing is left out of the comparison).
#[must_use]
pub fn sum_eq_all(a: &Sum, b: &Sum) -> bool {
    format!("{a:?}") == format!("{b:?}")
}

/// v3.2: the booked trades of one account as the portfolio layer and the month verdicts read them (the very same `detail3` steps; each
/// trade's net and exit from the sweep's outcome table, proven equal to the recomputed trade by P5): (entry us, known us, margin, net, day).
fn decided_trades(sr: &Seg3, sm: &Sum, csets: &[CSet], dd: &[i64]) -> Vec<crate::v32::PTrade> {
    decided_trades_m(sr, sm, csets, dd, None).into_iter().map(|x| x.0).collect()
}

/// REPAIR A (check-engine-2 M1 / M4): the booked trades with their marks (units, value sold, the contract-day's marks offset); `from_piece`
/// = a FRESH account from the first event of that year piece (Table W's walk from the row's own start), None = the run's first day.
fn decided_trades_m(sr: &Seg3, sm: &Sum, csets: &[CSet], dd: &[i64], from_piece: Option<usize>) -> Vec<(crate::v32::PTrade, crate::v32::PMark)> {
    let (si, l, ni) = (usize::from(sm.x) / NL, usize::from(sm.x) % NL, usize::from(sm.n));
    let (Some(list0), Some(cs)) = (sr.ul(sm.list as usize), csets.get(si)) else { return Vec::new() };
    let sub: Vec<u32>;
    let list: &[u32] = match from_piece {
        Some(p0) => {
            sub = list0.iter().copied().filter(|&e| sr.evs.get(e as usize).is_some_and(|ev| usize::from(sr.piece_of_day.get(usize::from(ev.di)).copied().unwrap_or(0)) >= p0)).collect();
            &sub
        }
        None => &list0,
    };
    let tf = sr.uls.get(sm.list as usize).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) });
    let (dec, _, _) = detail3(&sr.evs, &sr.ex, &sr.tab, list, cs, l, ni, tf, true, u16::try_from(sr.days.len()).unwrap_or(0));
    dec.iter()
        .filter(|d| d.code == "booked")
        .filter_map(|d| {
            let e = sr.evs.get(d.eid as usize)?;
            let x = sr.ex.get(d.eid as usize)?;
            let o = crate::kern::out_of_decision(e, x, &sr.tab, cs, l, ni, tf)?;
            let day = *dd.get(usize::from(e.di))?;
            let dk = u64::try_from(day).unwrap_or(0) << 32;
            Some(((crate::cal::at_sod(day, i64::from(e.t)), crate::cal::at_sod(day, i64::from(o.exit) + trade::BASE_S), i64::from(d.lots) * e.tab.margin_lot + e.tab.addon, i64::from(o.net), day, dk | u64::from(e.ci), dk | u64::from(e.skey)), crate::v32::PMark { units: u64::from(o.units), sold: o.sold, mark1: x.mark1 }))
        })
        .collect()
}

/// v3.3 fix CAP_MIN (check-engine-1 M1r): the (group, view) pairs a pick or the verdict view reads (the wanted picks, and the verdict view of
/// the SEARCH and HEADLINE groups under SEARCH15).
#[must_use]
pub fn cap_views33(ngroup: usize) -> Vec<(usize, usize)> {
    let mut v: Vec<(usize, usize)> = wanted_picks(ngroup).into_iter().map(|(_, g, w)| (g, w)).collect();
    if sw::on(sw::SEARCH15) {
        v.push((6, crate::acct::VIEW_V));
        if ngroup > NGROUP {
            v.push((NGROUP, crate::acct::VIEW_V));
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// v3.3 fix CAP_MIN: the account key of a top-heap entry (list, setting x share, slot index).
#[must_use]
pub fn acct_key33(s: &Sum) -> (u32, u16, u8) {
    (s.list, s.x, s.n)
}

/// v3.3 fix CAP_MIN: every view of `views` keeps only its first `k` accounts that are not in `bad` (a failing account leaves every one of those
/// views; an unchecked account can never become a pick).
pub fn cap_retain33(tops: &mut [Tops], views: &[(usize, usize)], k: usize, bad: &std::collections::HashSet<(u32, u16, u8)>) {
    for &(g, v) in views {
        if let Some(t) = tops.get_mut(g) {
            // MUTANT m_v33f_cap_filter_off: the failing accounts kept in the views
            if cfg!(feature = "m_v33f_cap_filter_off") {
                t.retain_view(v, k, &|_| true);
            } else {
                t.retain_view(v, k, &|s| !bad.contains(&acct_key33(s)));
            }
        }
    }
}

/// v3.3 fix CAP_MIN (check-engine-1 M1r): the first K accounts of every view a pick or the verdict view reads, checked MINUTE BY MINUTE on their
/// decided trades and the kernel's marks ([`crate::fix33::cap_minutes`]: margin in use + the running MTM loss against Rs 20 L + min(0,
/// realized)); an account above the base in any minute is EXCLUDED from those views, and each view keeps only its checked accounts. The stage
/// A table of these accounts' lists is in memory (whole mode, or the D1 rebuild of the day-major mode, which takes them under CAP_MIN).
fn cap_filter33(sr: &mut Seg3, csets: &[CSet], dd: &[i64], log: &mut String, sn: &str) {
    let t0 = Instant::now();
    let k = crate::fix33::CAP_K.load(Ordering::Relaxed);
    let views = cap_views33(sr.ngroup);
    let run_days: Vec<i64> = sr.days.iter().filter_map(|&di| dd.get(di).copied()).collect();
    let mut cands: Vec<Sum> = Vec::new();
    for &(g, v) in &views {
        if let Some(t) = sr.tops.get(g) {
            cands.extend(t.best(v).into_iter().take(k));
        }
    }
    cands.sort_by_key(acct_key33);
    cands.dedup_by_key(|s| acct_key33(s));
    let marks = sr.tab.marks.as_slice();
    let mut bad: std::collections::HashSet<(u32, u16, u8)> = std::collections::HashSet::new();
    let mut lines: Vec<String> = Vec::new();
    for s in &cands {
        let tr = decided_trades_m(sr, s, csets, dd, None);
        let t5: Vec<(i64, i64, i64, i64, i64, crate::v32::PMark)> = tr.iter().map(|(p, pm)| (p.0, p.1, p.2, p.3, p.4, *pm)).collect();
        let c = crate::fix33::cap_minutes(&t5, marks, &run_days);
        if !c.ok() {
            bad.insert(acct_key33(s));
            lines.push(format!("list {} / setting x {} / N index {} ({} trades, net Rs {}): {}", s.list, s.x, s.n, s.trades, fmt_rs(s.net), crate::fix33::cap_words(&c)));
        }
    }
    cap_retain33(&mut sr.tops, &views, k, &bad);
    let (nc, nb) = (u64::try_from(cands.len()).unwrap_or(0), u64::try_from(bad.len()).unwrap_or(0));
    sr.cap33 = (nc, nb);
    sr.cap33_lines = lines;
    crate::fix33::CAP_CHECKED.fetch_add(nc, Ordering::Relaxed);
    crate::fix33::CAP_EXCLUDED.fetch_add(nb, Ordering::Relaxed);
    lg!(log, "{sn}: v3.3 FIX CAP_MIN (check-engine-1 M1r): {nc} accounts of {} views (first {k} each) checked minute by minute against Rs 20 L + min(0, realized): {nb} EXCLUDED; each view keeps its checked accounts only; {:.1} s", views.len(), t0.elapsed().as_secs_f64());
}

/// v3.2-B (addendum 18): the combos a RESULTS table shows per segment: the SEARCH and HEADLINE groups' views V (under SEARCH15), A, S, X and
/// the ALL group's view A, the first `k` of each (every kept combo of every group and view goes to splits18.tsv).
fn kept18(sr: &Seg3, k: usize) -> Vec<(String, Sum)> {
    let mut v = Vec::new();
    for (g, gname) in [(6_usize, "SEARCH"), (NGROUP, "HEADLINE"), (0, "ALL")] {
        let Some(t) = sr.tops.get(g) else { continue };
        for vw in [crate::acct::VIEW_V, 0, 8, 1] {
            if (vw == crate::acct::VIEW_V && !sw::on(sw::SEARCH15)) || (g == 0 && vw != 0) {
                continue;
            }
            for (i, s) in t.best(vw).iter().take(if g == 0 { 5 } else { k }).enumerate() {
                v.push((format!("{gname} {}{}", crate::acct::view_letter(vw), i + 1), *s));
            }
        }
    }
    v
}

/// v3.2: the RESULTS sections of every v3.2 switch (addenda 8b, 10, 12-16).
#[allow(clippy::too_many_lines)]
fn v32_sections(segs: &[Seg3], sets: &Sets, sel: &[usize], csets: &[CSet], menu: &Menu, days: &[String], v32rows: &[(String, Vec<(crate::v32::Row32, String)>)], log32: &mut String, rdir: &Path) -> Result<String, String> {
    let mut md = String::new();
    let dd: Vec<i64> = days.iter().filter_map(|d| crate::cal::parse_day(d)).collect();
    let months = crate::v32::months_of(&dd);
    let _ = crate::v32::UTIL_DAYS.set(dd.clone());
    let _ = writeln!(md, "## ENGINE v3.2: what changed from v3.1 and the readings of this run");
    let _ = writeln!(md);
    let _ = writeln!(md, "**ENGINE v3.2** = engine v3.1 (`scratch/spikefade-v31`, binary h5 sha256 470cea09f5624415...) + MAIN-RULINGS-ENGINE-v22 addenda 8b, 10, 11, 12 / 12b / 12c, 13 / 13b, 14, 15, 16 and the v3.1 check's M1-M4 / S1-S8; fill crate v2.4 (`pinned/common-fill-v24` = crate v2.3 + margin rule v3 M3 + the executed-order count O1). v3.2 switches: {}.", sw::words32());
    let _ = writeln!(md);
    let _ = writeln!(md, "- {}", crate::freeze::counts_line());
    if let Some(f) = crate::freeze::FREEZE.get() {
        let _ = writeln!(md, "- Freeze sources (NSE): {}", f.sources.join("; "));
    }
    let _ = writeln!(md, "- {}", crate::ban::counts_line());
    if let Some(b) = crate::ban::BANS.get() {
        let _ = writeln!(md, "- Ban lists (NSE fo_secban, session S reads S's own file): {}", b.sources.join(" + "));
    }
    if let Some(t) = crate::gk::TBILL.get() {
        let _ = writeln!(md, "- Greeks (addenda 8 / 8b): the app's OWN crates/greeks (path dependency, unmodified), the app's bar_greeks conversions; risk-free = the RBI 91-day T-bill {} (the latest release strictly before the trade day). Dividends not modelled.", t.source);
    }
    let _ = writeln!(md, "- The screener's gain measure (addenda 13 / 15): {}.", if screen::GAIN_MEASURE.load(Ordering::Relaxed) == 1 { "vs the PREVIOUS session's close (the bhavcopy LastPric of the previous session = Dhan's number, else its ClsPric; a contract without a previous close is no member): every timeframe's volume window and decision grid stay, only the gain's start price changes" } else { "inside the signal window (v3.1's; the day-so-far timeframe is the gain vs today's open)" });
    let _ = writeln!(md, "- Expiry (addendum 16): which expiry = {}; days to expiry = {} (index lists); expiry-day exposure = 2% + the 2% add-on on the option's own expiry day from 2024-11-20 (THE margin module).", crate::expiry::PICK_NAMES.get(usize::from(crate::expiry::PICK.load(Ordering::Relaxed))).copied().unwrap_or("?"), crate::expiry::DTE_NAMES.get(usize::from(crate::expiry::DTE.load(Ordering::Relaxed))).copied().unwrap_or("?"));
    if sw::on(sw::EXP_NOSUB) || sw::on(sw::SESS_DATED) {
        // REPAIR A (check-engine-2 M2 / M3): the strict dial's refusals and the dated session calendar
        let _ = writeln!(md, "- REPAIR A (check-engine-2 M2 / M3): {}; days to expiry and the theta target read {}.", crate::expiry::not_staged_words(), crate::expiry::sess_words());
    }
    if sw::on(sw::WHY12) {
        // REPAIR A (check-engine-1 S5 (b) / S6 / S7): the previous-close reading worded to its evidence; the Greeks and protection labels
        let _ = writeln!(md, "- Previous close (addenda 13 / 13b): the NSE F&O bhavcopy of the PREVIOUS session (published the evening before: no look-ahead). Both of its numbers are printed per trade: ClsPric (NSE's official close; NSE's own next-day PrvsClsgPric equals it) and LastPric (the last traded price). One Dhan screen (PIIND 23 NOV 2026 1900 PE: 4 Nov ClsPric 7.35, LastPric 7.80, Dhan 7.80) is consistent with LastPric, but the 3 Nov close was also 7.80: which number Dhan shows is NOT settled until a second Dhan screen where they differ.");
        let _ = writeln!(md, "- Greeks (addendum 8b): the app's own Greeks code on the 1-minute inputs (proven equal to the live app's /greeks.json, which serves option Greeks on 1-second rows only: a per-minute row-by-row comparison with /db is not possible). Market protection (S7): band = max(ceil 10% of the last traded price, 1 tick), buy limit rounded down, sell limit up: the s25 sell-diag's rule, UNVERIFIED (Dhan's and NSE's option protection slabs are not sourced).");
    } else {
    let _ = writeln!(md, "- Previous close (addenda 13 / 13b): the NSE F&O bhavcopy of the PREVIOUS session (published the evening before: no look-ahead). Both of its numbers are printed per trade: ClsPric (NSE's official close) and LastPric (the last traded price). MEASURED: PIIND 23 NOV 2026 1900 PE, 4 Nov file: ClsPric 7.35, LastPric 7.80; the operator's Dhan screen shows 7.80 (+1,079.49%): Dhan's 'previous close' is the LAST PRICE, not NSE's official close.");
    }
    let _ = writeln!(md);
    // addendum 17: the universe per session (every staged option contract of every underlying is a candidate of its list)
    let _ = writeln!(md, "Addendum 17, the universe per session (every option contract of every underlying the store holds for the day is a candidate of its segment + expiry list; Tickvault's 8 days hold NIFTY and BANKNIFTY options only among the indices, so no other index account exists here; the GDFL store holds FINNIFTY from 2021-01, MIDCPNIFTY from 2022-01 and NIFTYNXT50 from 2024-05, not in the cubes yet):");
    let _ = writeln!(md);
    let _ = writeln!(md, "| segment | session | underlyings | option contracts | contracts with a print | (underlying, expiry) pairs listed |");
    let _ = writeln!(md, "|---|---|---|---|---|---|");
    for (seg, sr) in segs.iter().enumerate() {
        for &(di, u, n, t, e) in &sr.universe {
            let _ = writeln!(md, "| {} | {} | {u} | {n} | {t} | {e} |", SEG_SHORT.get(seg).copied().unwrap_or("?"), days.get(di).map_or("?", String::as_str));
        }
    }
    let _ = writeln!(md);
    let marks2: [&[i32]; 2] = [segs.first().map_or(&[][..], |s| s.tab.marks.as_slice()), segs.get(1).map_or(&[][..], |s| s.tab.marks.as_slice())];
    for (name, rows) in v32rows {
        if sw::on(sw::PORT_MTM) || sw::on(sw::A19) || sw::on(sw::WK_RUN) {
            md.push_str(&crate::v32::pick_tables_m(name, rows, &months, marks2));
        } else {
            md.push_str(&crate::v32::pick_tables(name, rows, &months));
        }
    }
    if sw::on(sw::SEARCH15) {
        let _ = writeln!(md, "## Addendum 15: the combos walked, the verdict view, the closest misses and the s26 portfolio layer");
        let _ = writeln!(md);
        let _ = writeln!(md, "| segment | (config, liquidity) pairs | settings | share levels | slot counts | combos covered (pairs x settings x shares x N) | distinct accounts walked (unique lists x settings x shares x N) | account-event steps | ns per step p50 / p99 {} |", if sw::on(sw::STEPT) { "(MEASURED: wall clock over sampled step blocks, step-weighted)" } else { "(MEASURED per work item)" });
        let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|");
        for (seg, sr) in segs.iter().enumerate() {
            if sr.evs.is_empty() {
                continue;
            }
            let pairs: u128 = sr.ul_cfgs.iter().map(|v| v.len() as u128).sum();
            let nsh = u128::from(trade::SHARE_MASK.load(Ordering::Relaxed).count_ones());
            let combos = pairs * csets.len() as u128 * nsh * 4;
            let mut v = sr.step_ns.clone();
            v.sort_unstable();
            let q = |p: usize| v.get(v.len().saturating_sub(1) * p / 100).copied().unwrap_or(0);
            if sw::on(sw::STEPT) || sw::on(sw::NGRID) {
                // REPAIR A (check-engine-2 S3 / S5): the N grid of the segment, the step-weighted p50 / p99 of SAMPLED step blocks (wall clock)
                let (p50, p99, ns) = crate::kern::step_weighted(&sr.step_blocks);
                let grid: Vec<String> = (0..4).map(|ni| sw::n_slots(u8::try_from(seg).unwrap_or(0), ni).to_string()).collect();
                if crate::fix33::on(crate::fix33::STEP_DM) {
                    // v3.3 fix STEP_DM (check-engine-2 M3): the day-major walk's own sampled account-day walks; zero samples = NOT MEASURED
                    let what = crate::fix33::step_words(p50, p99, ns, &sr.step33);
                    let _ = writeln!(md, "| {} | {pairs} | {} | {nsh} | 4 ({}) | {combos} | {} | {} | {what} |", SEG_SHORT.get(seg).copied().unwrap_or("?"), csets.len(), grid.join(" / "), sr.accounts, sr.steps);
                    lg!(log32, "{}: COMBOS covered {combos} (pairs {pairs} x settings {} x shares {nsh} x N 4 = {}); distinct accounts walked {}; account-event steps {}; ns per step {what}", SEG_SHORT.get(seg).copied().unwrap_or("?"), csets.len(), grid.join("/"), sr.accounts, sr.steps);
                } else {
                let _ = writeln!(md, "| {} | {pairs} | {} | {nsh} | 4 ({}) | {combos} | {} | {} | {}.{:03} / {}.{:03} (step-weighted over {ns} sampled step blocks of the stage B kernel, wall clock, set-up excluded) |", SEG_SHORT.get(seg).copied().unwrap_or("?"), csets.len(), grid.join(" / "), sr.accounts, sr.steps, p50 / 1_000, p50 % 1_000, p99 / 1_000, p99 % 1_000);
                lg!(log32, "{}: COMBOS covered {combos} (pairs {pairs} x settings {} x shares {nsh} x N 4 = {}); distinct accounts walked {}; account-event steps {}; ns per step p50 {}.{:03} / p99 {}.{:03} (step-weighted, {ns} sampled step blocks, wall clock)", SEG_SHORT.get(seg).copied().unwrap_or("?"), csets.len(), grid.join("/"), sr.accounts, sr.steps, p50 / 1_000, p50 % 1_000, p99 / 1_000, p99 % 1_000);
                }
                let nb: Vec<String> = (0..4).map(|ni| format!("N {}: {} of {} accounts booked at least one trade", grid.get(ni).map_or("?", String::as_str), sr.n_booked.get(ni).map_or(0, |x| x.1), sr.n_booked.get(ni).map_or(0, |x| x.0))).collect();
                let _ = writeln!(md);
                let _ = writeln!(md, "{} accounts that booked at least one trade, per slot count (check-engine-2 S3): {}.", SEG_SHORT.get(seg).copied().unwrap_or("?"), nb.join("; "));
                let _ = writeln!(md);
                lg!(log32, "{}: accounts with >= 1 trade per N: {}", SEG_SHORT.get(seg).copied().unwrap_or("?"), nb.join("; "));
            } else {
                let _ = writeln!(md, "| {} | {pairs} | {} | {nsh} | 4 | {combos} | {} | {} | {}.{:03} / {}.{:03} |", SEG_SHORT.get(seg).copied().unwrap_or("?"), csets.len(), sr.accounts, sr.steps, q(50) / 1_000, q(50) % 1_000, q(99) / 1_000, q(99) % 1_000);
                lg!(log32, "{}: COMBOS covered {combos} (pairs {pairs} x settings {} x shares {nsh} x N 4); distinct accounts walked {}; account-event steps {}; ns per step p50 {}.{:03} / p99 {}.{:03}", SEG_SHORT.get(seg).copied().unwrap_or("?"), csets.len(), sr.accounts, sr.steps, q(50) / 1_000, q(50) % 1_000, q(99) / 1_000, q(99) % 1_000);
            }
        }
        let _ = writeln!(md);
        let _ = writeln!(md, "The months of this run: {} ({}); a month without a trade counts as Rs 0 (failing). Verdict order: (1) every month >= Rs 2,00,000 net, (2) every week positive if possible, (3) the smallest max drawdown, (4) the biggest net. HINDSIGHT on any same-period view; Walk-forward test NOT RUN unless Table W says PASSED.", months.len(), months.iter().map(|&m| crate::v32::month_name(m)).collect::<Vec<_>>().join(", "));
        let _ = writeln!(md);
        for (seg, sr) in segs.iter().enumerate() {
            if sr.evs.is_empty() {
                continue;
            }
            let ss = SEG_SHORT.get(seg).copied().unwrap_or("?");
            let Some(tops) = sr.tops.get(6) else { continue };
            let best = tops.best(crate::acct::VIEW_V);
            let _ = writeln!(md, "### {}: the verdict view V over SEARCH settings (best 20 of {} accounts walked): {}", SEG_NAME.get(seg).copied().unwrap_or("?"), sr.accounts, crate::acct::view_name(crate::acct::VIEW_V));
            let _ = writeln!(md);
            if sw::on(sw::A19) || sw::on(sw::WK_RUN) {
                // REPAIR A (check-engine-2 S1 / S9, addendum 19): days, weeks and months against the RUN's; the worst of each with its date;
                // the closest misses with the rupee gap of every failing month and the net of every non-positive week
                let run_days: Vec<i64> = sr.days.iter().filter_map(|&di| dd.get(di).copied()).collect();
                let _ = writeln!(md, "Verdict order (addendum 19): (1) every month >= Rs 2,00,000 net, (2) every week positive, (3) every session positive, (4) the smallest max drawdown, (5) the biggest net; counted against the run's {} sessions (a session, week or month without a trade is net 0: not positive).", run_days.len());
                let _ = writeln!(md);
                let _ = writeln!(md, "| rank | list config | setting (words) | share | N | trades | months >= Rs 2 L / months | positive weeks / weeks | positive days / days | worst day Rs (date) | worst week Rs (week of) | worst month Rs | max DD Rs | net Rs | closest miss: failing months' rupee gaps; non-positive weeks |");
                let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
                for (rank, sm) in best.iter().take(20).enumerate() {
                    let (rep, _, _) = sr.uls.get(sm.list as usize).copied().unwrap_or((0, 0, 0));
                    let l = usize::from(sm.x) % NL;
                    let tr = decided_trades(sr, sm, csets, &dd);
                    let v = crate::v32::v19_of(&tr.iter().map(|t| (t.4, t.3)).collect::<Vec<_>>(), &run_days);
                    let _ = writeln!(md, "| {} | {} | {} | {}% | {} | {} | {} / {} | {} / {} | {} / {} | {} ({}) | {} ({}) | {} ({}) | {} | {} | {} |", rank + 1, crate::report::cfg_words(seg, rep as usize, &sr.lv), setting_words(menu, sets, sel, sm.x), SHARE_BP.get(l).copied().unwrap_or(0) / 100, sw::n_slots(sm.seg, usize::from(sm.n)), sm.trades, v.months_ok.0, v.months_ok.1, v.weeks.0, v.weeks.1, v.days.0, v.days.1, fmt_rs(v.worst_day.0), fmt_day(v.worst_day.1), fmt_rs(v.worst_week.0), fmt_day(v.worst_week.1), fmt_rs(v.worst_month.0), crate::v32::month_name(v.worst_month.1), fmt_rs(sm.mdd), fmt_rs(sm.net), crate::v32::v19_miss_words(&v));
                }
                let _ = writeln!(md);
            } else {
            let _ = writeln!(md, "| rank | list config | setting (words) | share | N | trades | months >= Rs 2 L / months | worst seen month Rs | weeks positive / weeks seen | worst week Rs | max DD Rs | net Rs | closest miss: the rupee gap of every failing month |");
            let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|---|");
            for (rank, sm) in best.iter().take(20).enumerate() {
                let (rep, _, _) = sr.uls.get(sm.list as usize).copied().unwrap_or((0, 0, 0));
                let l = usize::from(sm.x) % NL;
                let miss = if rank < 10 {
                    let tr = decided_trades(sr, sm, csets, &dd);
                    let mut mo: BTreeMap<i64, i64> = months.iter().map(|&m| (m, 0)).collect();
                    for t in &tr {
                        *mo.entry(crate::v32::month_id(t.4)).or_insert(0) += t.3;
                    }
                    crate::v32::miss_words(&mo)
                } else {
                    "-".to_owned()
                };
                let _ = writeln!(md, "| {} | {} | {} | {}% | {} | {} | {} / {} | {} | {} / {} | {} | {} | {} | {miss} |", rank + 1, crate::report::cfg_words(seg, rep as usize, &sr.lv), setting_words(menu, sets, sel, sm.x), SHARE_BP.get(l).copied().unwrap_or(0) / 100, crate::acct::n_of_s(usize::from(sm.seg), usize::from(sm.n)), sm.trades, sm.mo_ok, months.len(), fmt_rs(if usize::from(sm.mo_seen) < months.len() { sm.mo_min.min(0) } else { sm.mo_min }), sm.wk_pos, sm.wk_seen, fmt_rs(sm.wk_min), fmt_rs(sm.mdd), fmt_rs(sm.net));
            }
            let _ = writeln!(md);
            }
            // the s26 portfolio layer over the verdict view's best accounts
            if sw::on(sw::PORT_MTM) {
                v32_portfolio_mtm(&mut md, seg, sr, &best, sets, sel, csets, menu, &dd, &months);
                continue;
            }
            // the candidates: the view's best accounts with DISTINCT trade lists (an account repeating another's trades, e.g. the same
            // decisions at another share level, adds nothing to one account and is skipped), at most 8
            let mut cands: Vec<Sum> = Vec::new();
            let mut trs: Vec<Vec<crate::v32::PTrade>> = Vec::new();
            for s in best.iter().filter(|s| s.trades > 0) {
                if cands.len() >= 8 {
                    break;
                }
                let tr = decided_trades(sr, s, csets, &dd);
                if trs.iter().any(|x| x.iter().map(|y| (y.0, y.1, y.3, y.5)).eq(tr.iter().map(|y| (y.0, y.1, y.3, y.5)))) {
                    continue;
                }
                cands.push(*s);
                trs.push(tr);
            }
            if cands.len() >= 2 {
                let score = |mo: &BTreeMap<i64, i64>, dd2: i64, net: i64| (mo.values().filter(|&&v| crate::kern::month_ok(v)).count(), mo.values().copied().min().unwrap_or(0), -dd2, net);
                let mut chosen: Vec<usize> = vec![0];
                let _ = writeln!(md, "s26 portfolio layer ({ss}): the verdict view's best accounts with distinct trade lists ({} of them: {}) combined under ONE Rs 20 L account (each combo's own decisions, then the one account's rules across all: one position per contract and per underlying + side + expiry, the shared margin never above Rs 20 L, the free money covering every sale), greedy by the verdict order:", cands.len(), cands.iter().map(|s| format!("{} / {} / {}% / N {}", crate::report::cfg_words(seg, sr.uls.get(s.list as usize).map_or(0, |u| u.0) as usize, &sr.lv), setting_words(menu, sets, sel, s.x), SHARE_BP.get(usize::from(s.x) % NL).copied().unwrap_or(0) / 100, crate::acct::n_of_s(usize::from(s.seg), usize::from(s.n)))).collect::<Vec<_>>().join("; "));
                let _ = writeln!(md);
                let _ = writeln!(md, "| combos | members (candidate numbers) | trades taken | refused: shared Rs 20 L cap / one-account rules | months >= Rs 2 L / months | worst month Rs | max DD Rs | net Rs | margin peak / median Rs | minutes > 80% of Rs 20 L / idle (addendum 17) |");
                let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|");
                loop {
                    let members: Vec<&[crate::v32::PTrade]> = chosen.iter().filter_map(|&i| trs.get(i).map(Vec::as_slice)).collect();
                    let (net, n, refused, ruled, dd2, mo) = crate::v32::portfolio(&members, &months);
                    let taken = crate::v32::LAST_TAKEN.lock().map(|g| g.clone()).unwrap_or_default();
                    let (upk, umed, uhi, uidle) = crate::v32::utilization(&taken, &dd);
                    let _ = writeln!(md, "| {} | {} | {n} | {refused} / {ruled} | {} / {} | {} | {} | {} | {} / {} | {} / {} |", chosen.len(), chosen.iter().map(|i| (i + 1).to_string()).collect::<Vec<_>>().join(" + "), mo.values().filter(|&&v| crate::kern::month_ok(v)).count(), mo.len(), fmt_rs(mo.values().copied().min().unwrap_or(0)), fmt_rs(dd2), fmt_rs(net), fmt_rs(upk), fmt_rs(umed), crate::v32::share(uhi, 10_000), crate::v32::share(uidle, 10_000));
                    if chosen.len() >= 5 {
                        break;
                    }
                    let cur = score(&mo, dd2, net);
                    let mut best_add: Option<(usize, (usize, i64, i64, i64))> = None;
                    for i in 0..trs.len() {
                        if chosen.contains(&i) {
                            continue;
                        }
                        let mut m2: Vec<&[crate::v32::PTrade]> = members.clone();
                        m2.push(trs.get(i).map_or(&[][..], Vec::as_slice));
                        let (n2, _, _, _, d2, mo2) = crate::v32::portfolio(&m2, &months);
                        let sc = score(&mo2, d2, n2);
                        if best_add.is_none_or(|b| sc > b.1) {
                            best_add = Some((i, sc));
                        }
                    }
                    match best_add {
                        Some((i, sc)) if sc > cur => chosen.push(i),
                        _ => break,
                    }
                }
                let _ = writeln!(md);
            }
        }
    }
    // v3.2-B (addendum 17): the index underlyings of the run's cubes and the --idx-und filter (each index its own account)
    if crate::side17::idx_und().is_some() || sw::on(sw::SIDE17) || crate::cube::EXTRA.get().is_some() {
        let _ = writeln!(md, "Addendum 17 (each index its own account): {}.", crate::side17::idx_words());
        if let Some(x) = crate::cube::EXTRA.get() {
            let _ = writeln!(md, "Other NSE indices (round B): extra cube root {} merged into {} day loads; extra margin tables appended {} (`--margin-extra` {}).", x.display(), crate::cube::EXTRA_DAYS.load(Ordering::Relaxed), crate::mtable::EXTRA_TABLES.load(Ordering::Relaxed), crate::mtable::EXTRA_MAN.get().map_or_else(|| "not set: their contracts find no margin row and are refused, counted".to_owned(), |m| format!("{} sha256 {}", m.path, m.sha)));
        }
        let _ = writeln!(md);
    }
    let words_of = |seg: usize, s: &Sum| -> String {
        let Some(sr) = segs.get(seg) else { return "?".to_owned() };
        format!("{} / {} / {}% / N {}", crate::report::cfg_words(seg, sr.uls.get(s.list as usize).map_or(0, |u| u.0) as usize, &sr.lv), setting_words(menu, sets, sel, s.x), SHARE_BP.get(usize::from(s.x) % NL).copied().unwrap_or(0) / 100, crate::fix33::n_words(seg, usize::from(s.n)))
    };
    // v3.2-B (addendum 18): the split permutations from the one walk
    if sw::on(sw::SPLITS18) {
        let _ = writeln!(md, "## Addendum 18: split permutations (train/test cuts and walk-forward, anchored and rolling) from the one walk");
        let _ = writeln!(md);
        for (seg, sr) in segs.iter().enumerate() {
            if sr.evs.is_empty() {
                continue;
            }
            let w = |s: &Sum| words_of(seg, s).replace(" | ", "; ");
            md.push_str(&crate::split18::section(SEG_NAME.get(seg).copied().unwrap_or("?"), &sr.sp18, &w));
            md.push_str(&crate::split18::combo_table(SEG_NAME.get(seg).copied().unwrap_or("?"), &kept18(sr, 10), &w));
        }
    }
    // v3.3 fix CAP_MIN (check-engine-1 M1r): the walk-forward picks (Table W rows, every split's way B pick) checked minute by minute too
    if crate::fix33::on(crate::fix33::CAP_MIN) {
        md.push_str(&cap_picks33(segs, csets, &dd));
    }
    // v3.2-B (addendum 17): the side-by-side account (index AND stock options under one Rs 20 L account incl. the running MTM loss)
    if sw::on(sw::SIDE17) {
        let w2 = |seg: usize, s: &Sum| words_of(seg, s).replace(" | ", "; ");
        // v3.2 round B: every segment's candidates written for the cross-run combine (`spikefade side17-combine`), then the SAME combine here
        let mut sets: Vec<crate::side17::CandSet> = Vec::new();
        for (i, sr) in segs.iter().enumerate().take(2) {
            let seg8 = u8::try_from(i).unwrap_or(0);
            if sr.evs.is_empty() || !sw::on(sw::SEARCH15) {
                sets.push(crate::side17::CandSet { seg: seg8, ..crate::side17::CandSet::default() });
                continue;
            }
            let cs = crate::side17::CandSet::of_run(sr, seg8, csets, &dd, &w2);
            std::fs::write(rdir.join(format!("side17-cands-{}.tsv", SEG_SHORT.get(i).copied().unwrap_or("x"))), cs.to_tsv()).map_err(|e| e.to_string())?;
            sets.push(cs);
        }
        let (smd, stsv) = crate::side17::section_of(&sets);
        md.push_str(&smd);
        if !stsv.is_empty() {
            std::fs::write(rdir.join("side17-walk.tsv"), stsv).map_err(|e| e.to_string())?;
        }
    }
    let _ = &months;
    Ok(md)
}

/// v3.3 fix CAP_MIN (check-engine-1 M1r): every Table W row's pick (walked from the row's own first learning piece) and every split's way B
/// pick (walked from the run's first day) checked minute by minute against Rs 20 L + min(0, realized); a pick above it in any minute is named
/// and reads as NOT PASSED (never a walk-forward pick, never tradeable). Their stage A tables are in memory (whole mode; the D1 rebuild).
fn cap_picks33(segs: &[Seg3], csets: &[CSet], dd: &[i64]) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "v3.3 fix CAP_MIN (check-engine-1r M1r): the walk-forward picks checked MINUTE BY MINUTE (margin in use + the running MTM loss against Rs 20 L + min(0, realized)):");
    let _ = writeln!(md);
    for (seg, sr) in segs.iter().enumerate() {
        if sr.evs.is_empty() {
            continue;
        }
        let marks = sr.tab.marks.as_slice();
        let run_days: Vec<i64> = sr.days.iter().filter_map(|&di| dd.get(di).copied()).collect();
        let check = |s: &Sum, from: Option<usize>| -> crate::fix33::CapRes {
            let tr = decided_trades_m(sr, s, csets, dd, from);
            let t5: Vec<(i64, i64, i64, i64, i64, crate::v32::PMark)> = tr.iter().map(|(p, pm)| (p.0, p.1, p.2, p.3, p.4, *pm)).collect();
            crate::fix33::cap_minutes(&t5, marks, &run_days)
        };
        let (mut nw, mut bw, mut ns, mut bs) = (0_usize, Vec::new(), 0_usize, Vec::new());
        for w in &sr.win {
            if let (Some(row), Some(b)) = (&w.row, &w.best) {
                nw += 1;
                let c = check(&b.1, Some(row.learn.0));
                if !c.ok() {
                    bw.push(format!("Table W row {}: {}", row.id, crate::fix33::cap_words(&c)));
                }
            }
        }
        for b in sr.sp18.best.iter().filter_map(|v| v.first().copied().flatten()) {
            ns += 1;
            let c = check(&b.sm, None);
            if !c.ok() {
                bs.push(format!("split pick list {} / setting x {} / N index {}: {}", b.sm.list, b.sm.x, b.sm.n, crate::fix33::cap_words(&c)));
            }
        }
        crate::fix33::CAP_CHECKED.fetch_add(u64::try_from(nw + ns).unwrap_or(0), Ordering::Relaxed);
        crate::fix33::CAP_EXCLUDED.fetch_add(u64::try_from(bw.len() + bs.len()).unwrap_or(0), Ordering::Relaxed);
        let _ = writeln!(md, "- {}: Table W picks {nw} checked, {} above the base; split way B picks {ns} checked, {} above the base{}{}", SEG_NAME.get(seg).copied().unwrap_or("?"), bw.len(), bs.len(), if bw.is_empty() && bs.is_empty() { " (every one held the cap in every minute)" } else { ": each such pick reads as NOT PASSED (EXCLUDED, never a walk-forward pick): " }, bw.iter().chain(&bs).cloned().collect::<Vec<_>>().join("; "));
    }
    let _ = writeln!(md);
    md
}

/// REPAIR A (check-engine-2 M4 / S2): Table W under addenda 15 / 19: each row's pick chosen on its LEARNING months by the addendum 19 order
/// (most months >= Rs 2 L, most positive weeks, most positive sessions, smallest max DD, most net; `wf::learn_key` under WF19), then the
/// SAME account walked from the row's own first learning piece (a fresh account: `decided_trades_m`) and its TEST period judged: PASSED only
/// when every test month >= Rs 2,00,000, every test week and every test session are positive. The count follows that verdict.
fn table_w_a19(md: &mut String, segs: &[Seg3], sets: &Sets, sel: &[usize], csets: &[CSet], days: &[String]) {
    let _ = writeln!(md, "## Table W. Many walk-forward tests under addenda 15 / 19 (WINDOWS-MWF-v1, 21 rows from this one sweep; THE pick per row from SEARCH settings only, chosen on its own LEARNING months by the addendum 19 order; the test period walked from the row's own start)");
    let _ = writeln!(md);
    if crate::fixg::on(crate::fixg::WIN) {
        let _ = writeln!(md, "{}", crate::fixg::win_line());
        let _ = writeln!(md);
    }
    let _ = writeln!(md, "| segment | row | learning pieces | test pieces | ran | THE pick (learning order: months >= Rs 2 L, positive weeks, positive sessions, smallest max DD, most net) | learning months >= Rs 2 L / weeks positive / sessions positive | learning net Rs | learning max DD Rs | test (walked from the row's start): months >= Rs 2 L / weeks positive / sessions positive | test worst day / week / month | test net Rs | verdict | account-rows checked equal / re-walked |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    let yr = |p: usize| crate::wf::Y0 + i64::try_from(p).unwrap_or(0);
    let dd: Vec<i64> = days.iter().filter_map(|d| crate::cal::parse_day(d)).collect();
    let mut passed = [0_usize; 2];
    for (seg, sr) in segs.iter().enumerate() {
        for w in &sr.win {
            let Some(r) = &w.row else { continue };
            let in_p = |d: i64, a: usize, b: usize| { let y = crate::cal::civil(d).0 - crate::wf::Y0; y >= i64::try_from(a).unwrap_or(0) && y <= i64::try_from(b).unwrap_or(0) };
            let seg_days: Vec<i64> = sr.days.iter().filter_map(|&di| dd.get(di).copied()).collect();
            let learn_days: Vec<i64> = seg_days.iter().copied().filter(|&d| in_p(d, r.learn.0, r.learn.1)).collect();
            let test_days: Vec<i64> = seg_days.iter().copied().filter(|&d| in_p(d, r.test.0, r.test.1)).collect();
            let (pick, lcnt, ln, ld, tcnt, tworst, tn, verdict) = match (&w.best, w.run) {
                (_, false) => ("-".to_owned(), "-".to_owned(), "-".to_owned(), "-".to_owned(), "-".to_owned(), "-".to_owned(), "-".to_owned(), "NOT RUN (no session in its learning years: the run's days lie outside them)".to_owned()),
                (None, true) => ("(no SEARCH account traded in the learning years)".to_owned(), "-".to_owned(), "-".to_owned(), "-".to_owned(), "-".to_owned(), "-".to_owned(), "-".to_owned(), "NOT PASSED (W07)".to_owned()),
                (Some((_, sm, lf, _)), true) => {
                    let lv = crate::v32::v19_of(&[], &learn_days);
                    let tr = decided_trades_m(sr, sm, csets, &dd, Some(r.learn.0));
                    let tt: Vec<(i64, i64)> = tr.iter().filter(|t| in_p(t.0.4, r.test.0, r.test.1)).map(|t| (t.0.4, t.0.3)).collect();
                    let v = crate::v32::v19_of(&tt, &test_days);
                    let ok = crate::v32::a19_passed(&v, tt.len());
                    if ok && let Some(x) = passed.get_mut(seg) {
                        *x += 1;
                    }
                    (
                        format!("{} / {}", set_words(sets, sel, sm.x).0, crate::report::cfg_words(seg, sr.uls.get(sm.list as usize).map_or(0, |u| u.0) as usize, &sr.lv)),
                        format!("{} / {} ; {} / {} ; {} / {}", lf.mo_ok, lv.months_ok.1, lf.wk_pos, lv.weeks.1, lf.d_pos, lv.days.1),
                        fmt_rs(lf.net),
                        fmt_rs(lf.mdd),
                        format!("{} / {} ; {} / {} ; {} / {}", v.months_ok.0, v.months_ok.1, v.weeks.0, v.weeks.1, v.days.0, v.days.1),
                        format!("Rs {} ({}) / Rs {} (week of {}) / Rs {} ({})", fmt_rs(v.worst_day.0), if v.days.1 > 0 { fmt_day(v.worst_day.1) } else { "-".to_owned() }, fmt_rs(v.worst_week.0), if v.weeks.1 > 0 { fmt_day(v.worst_week.1) } else { "-".to_owned() }, fmt_rs(v.worst_month.0), if v.months_ok.1 > 0 { crate::v32::month_name(v.worst_month.1) } else { "-".to_owned() }),
                        fmt_rs(tt.iter().map(|t| t.1).sum()),
                        if ok { "Walk-forward test PASSED (addenda 15 / 19: every test month >= Rs 2 L, every test week and session positive)".to_owned() } else if test_days.is_empty() { "NOT PASSED (no test session in the run)".to_owned() } else { format!("NOT PASSED (addenda 15 / 19): {}", crate::v32::v19_miss_words(&v)) },
                    )
                }
            };
            let verdict = if w.pending > 0 { format!("{verdict}; {} account-rows PENDING a re-walk pass (day-major)", w.pending) } else { verdict };
            let _ = writeln!(md, "| {} | {} | {}..{} | {}..{} | {} | {pick} | {lcnt} | {ln} | {ld} | {tcnt} | {tworst} | {tn} | {verdict} | {} / {} |", SEG_SHORT.get(seg).copied().unwrap_or("?"), r.id, yr(r.learn.0), yr(r.learn.1), yr(r.test.0), yr(r.test.1), if w.run { "yes" } else { "no" }, w.checked, w.rewalked);
        }
    }
    let _ = writeln!(md);
    let _ = writeln!(md, "Many walk-forward tests (addenda 15 / 19 verdict): {} of 21 PASSED (index), {} of 21 PASSED (stock). The 21 rows share years: they are not 21 separate proofs. A week is counted in the year piece of its first session for the learning order.", passed[0], passed[1]);
    let _ = writeln!(md);
}

/// REPAIR A (check-engine-2 M1 / S7 / S10): the s26 portfolio layer with the kernel's MTM rule at every portfolio sale, the refusals split
/// (shared Rs 20 L cap or cash rule / running MTM loss / one-account rules), the per-minute margin + MTM peak, and every row labelled.
#[allow(clippy::too_many_arguments)]
fn v32_portfolio_mtm(md: &mut String, seg: usize, sr: &Seg3, best: &[Sum], sets: &Sets, sel: &[usize], csets: &[CSet], menu: &Menu, dd: &[i64], months: &[i64]) {
    let ss = SEG_SHORT.get(seg).copied().unwrap_or("?");
    let mut cands: Vec<Sum> = Vec::new();
    let mut trs: Vec<Vec<(crate::v32::PTrade, crate::v32::PMark)>> = Vec::new();
    for s in best.iter().filter(|s| s.trades > 0) {
        if cands.len() >= 8 {
            break;
        }
        let tr = decided_trades_m(sr, s, csets, dd, None);
        if trs.iter().any(|x| x.iter().map(|y| (y.0.0, y.0.1, y.0.3, y.0.5)).eq(tr.iter().map(|y| (y.0.0, y.0.1, y.0.3, y.0.5)))) {
            continue;
        }
        cands.push(*s);
        trs.push(tr);
    }
    if cands.len() < 2 {
        return;
    }
    let marks = sr.tab.marks.as_slice();
    let run_days: Vec<i64> = sr.days.iter().filter_map(|&di| dd.get(di).copied()).collect();
    let score = |mo: &BTreeMap<i64, i64>, dd2: i64, net: i64| (mo.values().filter(|&&v| crate::kern::month_ok(v)).count(), mo.values().copied().min().unwrap_or(0), -dd2, net);
    // v3.3 fix CAP_MIN (check-engine-1 M1r): the minute-by-minute cap of the trades the last portfolio walk took
    let cap33 = crate::fix33::on(crate::fix33::CAP_MIN);
    let cap_last = |rd: &[i64]| -> crate::fix33::CapRes {
        let taken = crate::v32::LAST_TAKEN_MTM.lock().map(|g| g.clone()).unwrap_or_default();
        let nets = crate::v32::LAST_TAKEN_NET.lock().map(|g| g.clone()).unwrap_or_default();
        let t5: Vec<(i64, i64, i64, i64, i64, crate::v32::PMark)> = taken.iter().zip(&nets).map(|(x, n)| (x.0, x.1, x.2, n.0, n.1, x.3)).collect();
        crate::fix33::cap_minutes(&t5, marks, rd)
    };
    let mut chosen: Vec<usize> = vec![0];
    let _ = writeln!(md, "s26 portfolio layer ({ss}; LABEL: members walked alone, then combined; chosen on the same days they are scored on: HINDSIGHT, not tradeable live): the verdict view's best accounts with distinct trade lists ({} of them: {}) combined under ONE Rs 20 L account (each combo's own decisions, then the one account's rules across all: one position per contract and per underlying + side + expiry; a sale refused when margin in use + its margin passes Rs 20 L, or when the free money less the running MTM loss of the portfolio's open positions cannot cover it: the kernel's rule), greedy by the verdict order. A member trade the portfolio refuses still held its slot and R1 key in that member's own later decisions (members walked alone):", cands.len(), cands.iter().map(|s| format!("{} / {} / {}% / N {}", crate::report::cfg_words(seg, sr.uls.get(s.list as usize).map_or(0, |u| u.0) as usize, &sr.lv), setting_words(menu, sets, sel, s.x), SHARE_BP.get(usize::from(s.x) % NL).copied().unwrap_or(0) / 100, sw::n_slots(s.seg, usize::from(s.n)))).collect::<Vec<_>>().join("; "));
    let _ = writeln!(md);
    let _ = writeln!(md, "| combos | members (candidate numbers) | trades taken | refused: Rs 20 L cap or cash / running MTM loss / one-account rules | months >= Rs 2 L / months | worst month Rs | max DD Rs | net Rs | margin peak / median Rs | minutes > 80% of Rs 20 L / idle | margin + MTM per minute (addendum 17) | label |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|");
    loop {
        let members: Vec<&[(crate::v32::PTrade, crate::v32::PMark)]> = chosen.iter().filter_map(|&i| trs.get(i).map(Vec::as_slice)).collect();
        let ((net, n, refused, ruled, dd2, mo), mtm) = crate::v32::portfolio_mtm(&members, marks, months);
        let taken = crate::v32::LAST_TAKEN_MTM.lock().map(|g| g.clone()).unwrap_or_default();
        let (upk, umed, uhi, uidle) = crate::v32::utilization(&taken.iter().map(|x| (x.0, x.1, x.2)).collect::<Vec<_>>(), &run_days);
        let pk = crate::v32::margin_mtm_peak(&taken, marks, &run_days);
        // v3.3 fix CAP_MIN (check-engine-1 M1r): the portfolio's minute-by-minute cap against the account base (only admissible ones are rows)
        let capw = if cap33 { format!("; {}", crate::fix33::cap_words(&cap_last(&run_days))) } else { String::new() };
        let _ = writeln!(md, "| {} | {} | {n} | {refused} / {mtm} / {ruled} | {} / {} | {} | {} | {} | {} / {} | {} / {} | {}{capw} | members walked alone; chosen on the same days (hindsight) |", chosen.len(), chosen.iter().map(|i| (i + 1).to_string()).collect::<Vec<_>>().join(" + "), mo.values().filter(|&&v| crate::kern::month_ok(v)).count(), mo.len(), fmt_rs(mo.values().copied().min().unwrap_or(0)), fmt_rs(dd2), fmt_rs(net), fmt_rs(upk), fmt_rs(umed), crate::v32::share(uhi, 10_000), crate::v32::share(uidle, 10_000), crate::v32::mtm_peak_words(pk));
        if chosen.len() >= 5 {
            break;
        }
        let cur = score(&mo, dd2, net);
        let mut best_add: Option<(usize, (usize, i64, i64, i64))> = None;
        for i in 0..trs.len() {
            if chosen.contains(&i) {
                continue;
            }
            let mut m2: Vec<&[(crate::v32::PTrade, crate::v32::PMark)]> = members.clone();
            m2.push(trs.get(i).map_or(&[][..], Vec::as_slice));
            let ((n2, _, _, _, d2, mo2), _) = crate::v32::portfolio_mtm(&m2, marks, months);
            if cap33 && !cap_last(&run_days).ok() {
                // v3.3 fix CAP_MIN: a portfolio above the account base in any minute is never admitted
                crate::fix33::CAP_PORT_REFUSED.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            let sc = score(&mo2, d2, n2);
            if best_add.is_none_or(|b| sc > b.1) {
                best_add = Some((i, sc));
            }
        }
        match best_add {
            Some((i, sc)) if sc > cur => chosen.push(i),
            _ => break,
        }
    }
    let _ = writeln!(md);
}

/// v3.2: the 17-item Dhan-live checklist (addendum 9) as THIS build covers it (any NOT COVERED item blocks the word headline).
fn v32_checklist() -> String {
    let mut md = String::new();
    let _ = writeln!(md, "## The Dhan-live checklist (addendum 9) under engine v3.2: COVERED with its evidence, PARTIAL or NOT COVERED with the reason (any item not COVERED blocks the word headline)");
    let _ = writeln!(md);
    let _ = writeln!(md, "| # | item | state | evidence / reason |");
    let _ = writeln!(md, "|---|---|---|---|");
    let st = |b: bool| if b { "COVERED" } else { "NOT COVERED" };
    let rows: [(&str, &str, String); 17] = [
        ("Signal at candle close, order after it (REQ 56); live timing W/L (REQ 69)", "NOT COVERED", "REQ 56 COVERED (fills from the block after the candle end); REQ 69's W x L latency grid is NOT built in v3.2".to_owned()),
        ("Fills: REQ 70 blocks, worst print, share 25% headline / 10% beside, carried entitlement, rest to the next block", st(sw::on(sw::CARRY) && sw::on(sw::HEADLINE) && sw::on(sw::PICKS32)), "crate v2.3 K1; every A / X / S / V pick's 10% twin detailed trade by trade (PICKS32)".to_owned()),
        ("Slippage: worst prints + jump stress 1-100% + the +2% gate + the stop overshoot", st(sw::on(sw::HEADLINE) && sw::on(sw::ORDERS_A10)), "jump-stress tables per pick; the +2% gate re-prices with the SAME executed orders (addendum 10)".to_owned()),
        ("Bid-ask spread: covered by the +2% gate", st(sw::on(sw::HEADLINE)), "headline gate; Tickvault quotes not read".to_owned()),
        ("Market protection and price bands", "NOT COVERED", "re-send COVERED (crate K2); a band lock is NOT modelled (no per-contract-day band data)".to_owned()),
        ("Stops: REQ 47 small stops only; checked every second; unfilled exits keep chasing; breakeven / trail", st(sw::on(sw::HEADLINE) && sw::on(sw::RESEND)), "SEARCH role + headline gate; per-second xrule; K2 re-send".to_owned()),
        ("Margin rule v2 / v3 (stock OTH row) + the Rs 20 L cap with the running MTM loss", if sw::on(sw::MARGIN_V3) && sw::on(sw::MTM) && (sw::on(sw::PORT_MTM) || !sw::any_ra()) { "COVERED" } else if sw::on(sw::MARGIN_V3) && sw::on(sw::MTM) { "PARTIAL" } else { "NOT COVERED" }, if sw::on(sw::PORT_MTM) || !sw::any_ra() { if !sw::any_ra() { "crate v2.4 M3: 46 / 46 Dhan screens to Rs 0.00 (tests_v24 v32_margin_rule_v3_46_dhan_screens); history: the table has no futures, so index exposure falls back to the spot (labelled, counted in Table M)".to_owned() } else { "crate v2.4 M3: 46 / 46 Dhan screens to Rs 0.00 (tests_v24 v32_margin_rule_v3_46_dhan_screens); the kernel AND the s26 portfolio refuse a sale when margin in use + the running MTM loss would pass the free money (REPAIR A check-engine-2 M1, test v32r_portfolio_cap_counts_mtm); per-minute margin + MTM peak printed per pick and portfolio; history index exposure base: the near-month future of every BOD SPAN file when the run reads REPAIR A's futures-all table (else the spot fallback, counted in Table M)".to_owned() } } else { "crate v2.4 M3: 46 / 46 Dhan screens to Rs 0.00 (tests_v24 v32_margin_rule_v3_46_dhan_screens); history: the table has no futures, so index exposure falls back to the spot (labelled, counted in Table M); the s26 portfolio cap has no MTM (h6: PARTIAL)".to_owned() }),
        ("Charges: the Dhan estimator per side at dated rates, EVERY executed order counted (addendum 10)", if sw::on(sw::ORDERS_A10) && (sw::on(sw::CHG_DATED) || !sw::any_ra()) { "COVERED" } else if sw::on(sw::ORDERS_A10) { "PARTIAL" } else { "NOT COVERED" }, if sw::on(sw::CHG_DATED) || !sw::any_ra() { if !sw::any_ra() { "every filling block x every freeze slice = one order (crate v2.4 O1, tests_v24 + host tests32); per-component split: the post-pass (node == engine to the paisa on 8,273 trades of v3.1)".to_owned() } else { "every filling block x every freeze slice = one order (crate v2.4 O1, tests_v24 + host tests32); the DATED rate set of each session only (REPAIR A check-engine-1 M1; NSE circulars FA46730 / FA56129 / FA61137 held; test v32r_charges_dated_only); the dearer-of reading printed beside".to_owned() } } else { "every filling block x every freeze slice = one order (crate v2.4 O1, tests_v24 + host tests32); charges at the dearer of the dated and the 2026-10-30 sets (h6: history not at the dated rates, check-engine-1 M1)".to_owned() }),
        ("Square-off: our own 15:10 order; orders end 15:30; never-closed units valued from the exit decision", st(sw::on(sw::OWN_SQUAREOFF) && sw::on(sw::END_1530) && sw::on(sw::VALUE_FROM_EXIT)), "switches; tests31".to_owned()),
        ("Expiry and ban rules: stock options never on their expiry day; index expiry captured; F&O ban days", st(sw::on(sw::BAN) && sw::on(sw::EXPIRY16)), format!("rule R3; addendum 16 dials and facts; {}", crate::ban::counts_line())),
        ("Account: slot Rs 20 L / N; lost money never reused; no compounding; one position per underlying + side + expiry", "COVERED", "kern.rs step3 (v2.1 rules unchanged)".to_owned()),
        ("Freeze-quantity slicing; whole lots", st(sw::on(sw::ORDERS_A10) && crate::freeze::FREEZE.get().is_some()), crate::freeze::counts_line()),
        ("Greeks at entry and exit (the app's own code)", st(sw::on(sw::GREEKS)), "crates/greeks linked unmodified; 8b proof: v32/PROOF-GREEKS-8B-v32.txt".to_owned()),
        ("Data precision: the store / cube / NSE-file proofs", "COVERED", "cube proofs; every NSE file named with its sha256 in RUN.log".to_owned()),
        ("Labels: underlying, signal, traded, size, period, run, proof step, hindsight, walk-forward", "COVERED", "label block at the top; HINDSIGHT on every same-period view".to_owned()),
        ("Run-up / run-down and re-spikes per trade (REQ 48, addendum 5)", "PARTIAL", "re-spike ladder and adverse 2/5/10 s per pick; per-trade run-up / run-down not printed by the engine".to_owned()),
        ("Every BLOCKING / HIGH case of EXTREME-MATRIX-SELLING.md with its named test", "PARTIAL", "see BUILD.md (v3.2 adds freeze, ban, liquidity, expiry cases; band lock, halts, events still need data)".to_owned()),
    ];
    // v3.3 integrate: the fixes' own readings of items 7, 8 and 15 (check-engine-1r M1r / M3r, check-engine-2r M2)
    let mut rows = rows;
    if crate::fix33::on(crate::fix33::CAP_MIN)
        && let Some(r) = rows.get_mut(6)
    {
        r.1 = "PARTIAL";
        r.2 = format!("{}; v3.3 fix CAP_MIN (check-engine-1r M1r): nothing acts BETWEEN sales (no RMS square-off modelled, matrix X20), so every account a pick or the verdict view reads is checked minute by minute against Rs 20 L + min(0, realized) and EXCLUDED when above in any minute (test v33f_cap_minutes_base_and_exclusion); {}", r.2, crate::fix33::cap_line());
    }
    if crate::fix33::on(crate::fix33::CHG33)
        && let Some(r) = rows.get_mut(7)
    {
        let pre = crate::fix33::PRE_STAMP.load(Ordering::Relaxed);
        r.2 = format!("{}; v3.3 fix CHG33 (check-engine-1r M3r): SEBI fee at 50% 2020-06-01..2021-03-31 (SEBI/LAD-NRO/GN/2020/011, NSE FA44514; test v33f_sebi_half_dated); stamp duty before 2020-07-01 UNVERIFIED (conservative 0.02% on both sides, no source on disk): {pre} session(s) of this run", r.2);
        if pre > 0 && r.1 == "COVERED" {
            r.1 = "PARTIAL";
        }
    }
    if crate::fix33::on(crate::fix33::WORDS33)
        && let Some(r) = rows.get_mut(14)
    {
        r.2 = format!("{}; v3.3 fix WORDS33 (check-engine-2r M2): the data vendor and period of every run day from its cube manifest; split / side words with the segment's N grid (tests v33f_split_words_segment_grid, v33f_vendor_from_manifest)", r.2);
    }
    // fix:v33 ITEMS (standing F2): every row follows its own evidence (nothing changes with the switch off)
    if crate::fixg::on(crate::fixg::ITEMS) {
        if let Some(r) = rows.get_mut(4) {
            r.2 = format!("{}{}", r.2, crate::fixg::ITEM5);
        }
        if let Some(r) = rows.get_mut(6)
            && let Some(w) = crate::fixg::IDX_WORDS.get()
        {
            r.2 = format!("{}; {w}", r.2);
        }
        if let Some(r) = rows.get_mut(7) {
            let (s, w) = crate::fixg::item8(r.1, crate::fixg::on(crate::fixg::GST), crate::fixg::GST_DAYS.load(Ordering::Relaxed));
            r.1 = s;
            r.2 = format!("{}{w}", r.2);
        }
        if let Some(r) = rows.get_mut(11) {
            let (s, w) = crate::fixg::item12(r.1, crate::freeze::UNDATED.load(Ordering::Relaxed), crate::freeze::NONE.load(Ordering::Relaxed));
            r.1 = s;
            r.2 = format!("{}{w}{}", r.2, if crate::fixg::on(crate::fixg::LOT) { format!("; {}", crate::fixg::lot_line()) } else { String::new() });
        }
        if let Some(r) = rows.get_mut(12) {
            r.2 = crate::fixg::ITEM13.to_owned();
        }
        if let Some(r) = rows.get_mut(13) {
            r.1 = crate::fixg::ITEM14.0;
            r.2 = crate::fixg::ITEM14.1.to_owned();
        }
        if let Some(r) = rows.get_mut(16) {
            r.2 = crate::fixg::ITEM17.to_owned();
        }
    }
    for (i, (item, s, ev)) in rows.iter().enumerate() {
        let _ = writeln!(md, "| {} | {item} | {s} | {ev} |", i + 1);
    }
    let _ = writeln!(md);
    md
}
