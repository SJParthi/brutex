//! The Top Volume screener on the cube (O(1) per contract window, O(list) per candle end) and the spike EVENTS of every filter
//! combination. ONE segment and ONE expiry per list, always (operator 14 Nov 19:09 IST): a list is (segment, expiry), its members are
//! the contracts of the underlyings whose CURRENT (or, for the next-expiry run, NEXT) expiry that is.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use crate::cal::{CLOSE_S, CUT_S, NM, OPEN_S};
use crate::cube::Cube;

/// Timeframes in minutes; 0 = the day so far (window from 09:15 to every 1-minute end).
pub const TFS: [u32; 10] = [1, 2, 3, 4, 5, 10, 15, 30, 60, 0];
/// Top N by window volume (lots).
pub const TOPN: [usize; 3] = [10, 20, 50];
/// v3.2 addendum 14 (a): the top-K-by-volume dial (the contract must rank in the top K by VOLUME of its list at the decision second, then
/// by gain): 5 / 10 / 20 (the old top-50 lists let thin contracts through).
pub const TOPK_A14: [usize; 3] = [5, 10, 20];

/// The top-by-volume values of the run: v3.1's [10, 20, 50], or addendum 14's [5, 10, 20] under the LIQ14 switch.
#[must_use]
pub fn topn() -> [usize; 3] {
    // MUTANT m_v32_topk_v31: the old top 10 / 20 / 50 kept under the switch
    if crate::sw::on(crate::sw::LIQ14) && !cfg!(feature = "m_v32_topk_v31") { TOPK_A14 } else { TOPN }
}
/// The largest N (the prefix kept per candle end).
pub const NMAX: usize = 50;
/// Minimum premium at T (paisa): Rs 1, 5, 10, 20.
pub const MINPREM: [i64; 4] = [100, 500, 1_000, 2_000];
/// Gain thresholds (%): an event is every list member whose window gain is at or above it.
pub const THR: [i64; 9] = [100, 150, 200, 300, 500, 1_000, 2_000, 5_000, 10_000];
/// Rank-only events: the first K members of the gain-sorted list with a gain above 0.
pub const RANK: [usize; 3] = [1, 3, 5];
/// Threshold dial levels (9 gains + 3 ranks).
pub const NTHR: usize = 12;
/// Configs per segment: tf x minvol x topN x minprem x minval x thr.
pub const NCFG: usize = 10 * 4 * 3 * 4 * 4 * NTHR;

/// Config id of (tf, mv, tn, mp, mval, thr).
#[must_use]
pub const fn cfg_id(tf: usize, mv: usize, tn: usize, mp: usize, mval: usize, thr: usize) -> usize {
    ((((tf * 4 + mv) * 3 + tn) * 4 + mp) * 4 + mval) * NTHR + thr
}

/// (tf, mv, tn, mp, mval, thr) of a config id.
#[must_use]
pub const fn cfg_parts(id: usize) -> (usize, usize, usize, usize, usize, usize) {
    let thr = id % NTHR;
    let r = id / NTHR;
    let mval = r % 4;
    let r = r / 4;
    let mp = r % 4;
    let r = r / 4;
    let tn = r % 3;
    let r = r / 3;
    let mv = r % 4;
    (r / 4, mv, tn, mp, mval, thr)
}

/// The threshold label.
#[must_use]
pub fn thr_name(t: usize) -> String {
    THR.get(t).map_or_else(|| format!("rank top{}", RANK.get(t.saturating_sub(9)).copied().unwrap_or(0)), |g| format!("gain>={g}%"))
}

/// The timeframe label.
#[must_use]
pub fn tf_name(i: usize) -> String {
    match TFS.get(i) {
        Some(0) => "day-so-far".to_owned(),
        Some(m) => format!("{m}m"),
        None => "?".to_owned(),
    }
}

/// The measured levels of one segment: per timeframe, 4 minimum-volume levels (lots) and 4 minimum traded-value levels (paisa, level 0 =
/// off), from the distribution of the top-50-by-volume members over every candle end of every staged day.
#[derive(Clone, Debug, Default)]
pub struct Levels {
    pub minvol: [[u64; 4]; 10],
    pub minval: [[u64; 4]; 10],
    /// Members sampled per timeframe.
    pub samples: [u64; 10],
}

/// One list member at one candle end.
#[derive(Clone, Copy, Debug, Default)]
pub struct Member {
    pub ci: u32,
    pub lots: u64,
    pub val: u64,
    pub ct: i32,
    pub c0: i32,
    pub gain_bp: i64,
    /// The window start reference is the contract's first open of the day (day-so-far, or listed / first traded inside the window).
    pub first_open_ref: bool,
}

/// The lists of one segment on one day: (expiry, member contract indices). `next` = the next-expiry run. `v2` = rule R3: a STOCK
/// underlying on its own expiry day lists its NEXT expiry (the day's own snapshot) when Tickvault recorded one that day; with none, its
/// expiring contracts stay listed and every event of theirs is refused at pricing (`stock_expiry_no_next`, counted).
/// v3.2-B (addendum 17): the lists of one segment and day with every INDEX list kept to the run's index underlyings (`--idx-und`: each
/// index its own account; unset = every index underlying together, byte for byte the v3.2 lists); stock lists untouched.
#[must_use]
pub fn lists(c: &Cube, seg: u8, next: bool, v2: bool) -> Vec<(u32, Vec<u32>)> {
    crate::side17::keep_unds(c, seg, lists_all(c, seg, next, v2))
}

#[must_use]
fn lists_all(c: &Cube, seg: u8, next: bool, v2: bool) -> Vec<(u32, Vec<u32>)> {
    // v3.1 item 6: the labelled VARIANT (walk-forward choice only): one list per (underlying, expiry); v3: one per (segment, expiry date)
    if crate::sw::on(crate::sw::LISTS_PER_UND) {
        let mut by: HashMap<(u32, u16), Vec<u32>> = HashMap::new();
        for (ci, k) in c.cons.iter().enumerate() {
            if k.kind != seg {
                continue;
            }
            let Some(u) = c.unds.get(usize::from(k.und)) else { continue };
            let want = if next { u.next } else if v2 && seg == 1 && u.nearest == c.ymd && u.next != 0 { u.next } else { u.nearest };
            if want == 0 || k.expiry != want {
                continue;
            }
            by.entry((want, k.und)).or_default().push(u32::try_from(ci).unwrap_or(0));
        }
        let mut v: Vec<((u32, u16), Vec<u32>)> = by.into_iter().collect();
        v.sort_by_key(|x| x.0);
        return v.into_iter().map(|((e, _), l)| (e, l)).collect();
    }
    // v3.2 addendum 16: INDEX lists inside the expiry the which-expiry dial picks (nearest = v3.1; busiest: every candidate expiry listed,
    // the decision-second filter in `generate` keeps each underlying's busiest one)
    if seg == 0 && !next && crate::sw::on(crate::sw::EXPIRY16) && crate::sw::on(crate::sw::EXP_NOSUB) {
        // REPAIR A (check-engine-2 M2): the target expiry from the previous session's bhavcopy list; a target not staged is REFUSED and
        // counted per day and underlying (no list for that underlying that day), never replaced by the nearest
        let st = crate::expiry::staged(c);
        let pick = crate::expiry::PICK.load(std::sync::atomic::Ordering::Relaxed);
        let mut allow: HashMap<u16, Vec<u32>> = HashMap::new();
        for (ui, u) in c.unds.iter().enumerate() {
            if u.kind != 0 {
                continue;
            }
            let und = u16::try_from(ui).unwrap_or(u16::MAX);
            let listed = crate::expiry::listed_for(c.day, &u.name);
            let staged = st.get(&und).map_or(&[][..], Vec::as_slice);
            match crate::expiry::dial_allowed(pick, u.nearest, listed.as_deref(), staged, c.ymd) {
                Ok((v, miss)) => {
                    if !miss.is_empty()
                        && let Ok(mut g) = crate::expiry::PARTIAL.lock()
                    {
                        g.insert((c.day, u.name.clone()), miss.join("; "));
                    }
                    allow.insert(und, v);
                }
                Err(w) => {
                    if let Ok(mut g) = crate::expiry::NOT_STAGED.lock() {
                        g.insert((c.day, u.name.clone()), w);
                    }
                }
            }
        }
        let mut by: HashMap<u32, Vec<u32>> = HashMap::new();
        for (ci, k) in c.cons.iter().enumerate() {
            if k.kind != 0 || !allow.get(&k.und).is_some_and(|v| v.contains(&k.expiry)) {
                continue;
            }
            by.entry(k.expiry).or_default().push(u32::try_from(ci).unwrap_or(0));
        }
        let mut v: Vec<(u32, Vec<u32>)> = by.into_iter().collect();
        v.sort_by_key(|x| x.0);
        return v;
    }
    if seg == 0 && !next && crate::sw::on(crate::sw::EXPIRY16) {
        let st = crate::expiry::staged(c);
        let mut by: HashMap<u32, Vec<u32>> = HashMap::new();
        for (ci, k) in c.cons.iter().enumerate() {
            if k.kind != 0 {
                continue;
            }
            let Some(u) = c.unds.get(usize::from(k.und)) else { continue };
            let exps = st.get(&k.und).map_or(&[][..], Vec::as_slice);
            let ok = match crate::expiry::pick_of(u.nearest, u.next, exps) {
                Some(w) => w != 0 && k.expiry == w,
                None => crate::expiry::candidates(u.nearest, u.next, exps).contains(&k.expiry),
            };
            if ok {
                by.entry(k.expiry).or_default().push(u32::try_from(ci).unwrap_or(0));
            }
        }
        let mut v: Vec<(u32, Vec<u32>)> = by.into_iter().collect();
        v.sort_by_key(|x| x.0);
        return v;
    }
    let mut by: HashMap<u32, Vec<u32>> = HashMap::new();
    for (ci, k) in c.cons.iter().enumerate() {
        if k.kind != seg {
            continue;
        }
        let Some(u) = c.unds.get(usize::from(k.und)) else { continue };
        let want = if next {
            u.next
        } else if v2 && seg == 1 && u.nearest == c.ymd && u.next != 0 {
            u.next
        } else {
            u.nearest
        };
        if want == 0 || k.expiry != want {
            continue;
        }
        by.entry(want).or_default().push(u32::try_from(ci).unwrap_or(0));
    }
    let mut v: Vec<(u32, Vec<u32>)> = by.into_iter().collect();
    v.sort_by_key(|x| x.0);
    v
}

/// Rule R3 facts of one day's STOCK underlyings: (underlyings on their own expiry day with a next expiry staged (rolled), those with
/// none staged (their events refused)).
#[must_use]
pub fn stock_roll_facts(c: &Cube) -> (u32, u32) {
    let (mut rolled, mut refused) = (0_u32, 0_u32);
    for u in &c.unds {
        if u.kind == 1 && u.nearest == c.ymd {
            if u.next != 0 {
                rolled += 1;
            } else {
                refused += 1;
            }
        }
    }
    (rolled, refused)
}

/// The rank-list key of a contract inside ONE list (one expiry): (underlying index, side). Rule R2.
#[inline]
#[must_use]
pub fn rank_key(c: &Cube, ci: u32) -> u32 {
    c.cons.get(ci as usize).map_or(u32::MAX, |k| (u32::from(k.und) << 1) | u32::from(k.right & 1))
}

/// The window of candle k of timeframe tf: (start slot, end slot exclusive, decision second T). None past 15:30.
#[must_use]
pub fn window(tf: u32, k: usize) -> Option<(usize, usize, i64)> {
    let (a, b) = if tf == 0 { (0, k + 1) } else { (k * tf as usize, (k + 1) * tf as usize) };
    let t = OPEN_S + i64::try_from(b).ok()? * 60;
    if b > NM || t > CLOSE_S { None } else { Some((a, b, t)) }
}

/// One contract's member record over [a, b) (None: no volume, no price or no start reference).
#[must_use]
pub fn member(c: &Cube, ci: usize, a: usize, b: usize, day_window: bool) -> Option<Member> {
    let k = c.cons.get(ci)?;
    if k.lot == 0 {
        return None;
    }
    let lots = c.wvol(ci, a, b) / u64::from(k.lot);
    if lots == 0 {
        return None;
    }
    let ct = c.close_at(ci, b.checked_sub(1)?);
    if ct <= 0 {
        return None;
    }
    let fm = usize::try_from(k.first_min).ok();
    let (c0, fo) = if day_window {
        if fm.is_some_and(|f| f < b) { (k.first_open, true) } else { return None }
    } else if a > 0 && c.close_at(ci, a - 1) > 0 {
        (c.close_at(ci, a - 1), false)
    } else if fm.is_some_and(|f| f >= a && f < b) {
        (k.first_open, true)
    } else {
        return None;
    };
    if c0 <= 0 {
        return None;
    }
    let gain_bp = (i64::from(ct) - i64::from(c0)) * 10_000 / i64::from(c0);
    Some(Member { ci: u32::try_from(ci).ok()?, lots, val: c.wval(ci, a, b), ct, c0, gain_bp, first_open_ref: fo })
}

/// v3.2 (addenda 13 / 15): the gain measure of the screener's trigger: 0 = inside the signal window (v3.1), 1 = vs the previous session's
/// close (the number Dhan shows: the previous session's bhavcopy LastPric, else its ClsPric; known before the open). `--gain-measure`.
pub static GAIN_MEASURE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
static PREVC: std::sync::OnceLock<std::sync::Mutex<HashMap<i64, std::sync::Arc<Vec<i32>>>>> = std::sync::OnceLock::new();

/// The previous-session close of every contract of a cube (paisa, 0 = none: such a contract is no member under the previous-close measure),
/// cached per day.
#[must_use]
pub fn prev_closes(c: &Cube) -> std::sync::Arc<Vec<i32>> {
    let m = PREVC.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    if let Ok(g) = m.lock()
        && let Some(v) = g.get(&c.day)
    {
        return std::sync::Arc::clone(v);
    }
    let v: Vec<i32> = c.cons.iter().map(|k| {
        let name = c.unds.get(usize::from(k.und)).map_or("", |u| u.name.as_str());
        crate::bhav::prev_close(c.day, name, k.expiry, k.strike, k.right).map_or(0, |(_, cls, last, _)| i32::try_from(if last > 0 { last } else { cls }).unwrap_or(0))
    }).collect();
    let a = std::sync::Arc::new(v);
    if let Ok(mut g) = m.lock() {
        g.insert(c.day, std::sync::Arc::clone(&a));
    }
    a
}

/// v3.2: one contract's member record with the gain measured from `prev` (the previous session's close) instead of the window start.
#[must_use]
pub fn member_prev(c: &Cube, ci: usize, a: usize, b: usize, day_window: bool, prev: i32) -> Option<Member> {
    // MUTANT m_v32_gain_measure_ignored: the window start kept under the previous-close measure
    if cfg!(feature = "m_v32_gain_measure_ignored") {
        return member(c, ci, a, b, day_window);
    }
    let k = c.cons.get(ci)?;
    if k.lot == 0 || prev <= 0 {
        return None;
    }
    let lots = c.wvol(ci, a, b) / u64::from(k.lot);
    if lots == 0 {
        return None;
    }
    let ct = c.close_at(ci, b.checked_sub(1)?);
    if ct <= 0 {
        return None;
    }
    let gain_bp = (i64::from(ct) - i64::from(prev)) * 10_000 / i64::from(prev);
    Some(Member { ci: u32::try_from(ci).ok()?, lots, val: c.wval(ci, a, b), ct, c0: prev, gain_bp, first_open_ref: false })
}

/// The list's members at one window, sorted by lots descending (ties: value descending, then contract index).
pub fn members(c: &Cube, list: &[u32], a: usize, b: usize, day_window: bool, out: &mut Vec<Member>) {
    out.clear();
    if GAIN_MEASURE.load(std::sync::atomic::Ordering::Relaxed) == 1 {
        let pv = prev_closes(c);
        for &ci in list {
            if let Some(m) = member_prev(c, ci as usize, a, b, day_window, pv.get(ci as usize).copied().unwrap_or(0)) {
                out.push(m);
            }
        }
        out.sort_unstable_by(|x, y| y.lots.cmp(&x.lots).then(y.val.cmp(&x.val)).then(x.ci.cmp(&y.ci)));
        return;
    }
    for &ci in list {
        if let Some(m) = member(c, ci as usize, a, b, day_window) {
            out.push(m);
        }
    }
    out.sort_unstable_by(|x, y| y.lots.cmp(&x.lots).then(y.val.cmp(&x.val)).then(x.ci.cmp(&y.ci)));
}

fn round2(x: u64) -> u64 {
    if x < 100 {
        return x.max(1);
    }
    let mut p = 1_u64;
    while x / p >= 100 {
        p *= 10;
    }
    (x / p) * p
}

fn quant(v: &mut [u64], q: u64) -> u64 {
    if v.is_empty() {
        return 1;
    }
    v.sort_unstable();
    let i = (u64::try_from(v.len() - 1).unwrap_or(0) * q / 100) as usize;
    v.get(i).copied().unwrap_or(1)
}

/// Measures the levels of segment `seg` over every staged day (current-expiry lists).
#[must_use]
pub fn measure(cubes: &[Cube], seg: u8) -> Levels {
    let mut lv = Levels::default();
    let mut buf = Vec::new();
    for (ti, &tf) in TFS.iter().enumerate() {
        let mut lots: Vec<u64> = Vec::new();
        let mut vals: Vec<u64> = Vec::new();
        for c in cubes {
            let ls = lists(c, seg, false, false);
            for k in 0..NM {
                let Some((a, b, _)) = window(tf, k) else { break };
                for (_, l) in &ls {
                    members(c, l, a, b, tf == 0, &mut buf);
                    for m in buf.iter().take(NMAX) {
                        lots.push(m.lots);
                        vals.push(m.val);
                    }
                }
            }
        }
        if let Some(s) = lv.samples.get_mut(ti) {
            *s = u64::try_from(lots.len()).unwrap_or(0);
        }
        let mv = [quant(&mut lots, 25), quant(&mut lots, 50), quant(&mut lots, 75), quant(&mut lots, 90)];
        let mvl = [0, quant(&mut vals, 25), quant(&mut vals, 50), quant(&mut vals, 75)];
        if let (Some(a), Some(b)) = (lv.minvol.get_mut(ti), lv.minval.get_mut(ti)) {
            *a = [round2(mv[0]), round2(mv[1]), round2(mv[2]), round2(mv[3])];
            *b = [0, round2(mvl[1]), round2(mvl[2]), round2(mvl[3])];
        }
    }
    lv
}

/// One unique event candidate (day, contract, decision second).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct UEv {
    pub di: u16,
    pub seg: u8,
    pub ci: u32,
    pub t: u32,
}

/// One event entry of a config list: the unique event and its reporting facts.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ent {
    pub ue: u32,
    pub gain_bp: i64,
    pub lots: u64,
}

/// The base filter config of the reporting tables: lowest minimum volume, top 50, minimum premium Rs 1, traded value off.
pub const BASE: (usize, usize, usize, usize) = (0, 2, 0, 0);

/// After-event facts at the base filter config (premium path of each event) and the random-time baseline.
#[derive(Clone, Debug, Default)]
pub struct After {
    /// [seg][tf][thr]: events, lower after 5 / 15 / 30 min (of those with a price then), with a price then, worst run (bp) within
    /// 5 / 15 / 30 min (max over events of the highest 1m HIGH after T against the premium at T), sum of runs (bp) for the mean.
    pub n: Vec<u64>,
    pub lower: Vec<[u64; 3]>,
    pub priced: Vec<[u64; 3]>,
    pub worst: Vec<[i64; 3]>,
    pub sum_run: Vec<[i64; 3]>,
    /// The same for the random baseline (one random top-50 member at one random candle end of the same timeframe and day per event).
    pub r_lower: Vec<[u64; 3]>,
    pub r_priced: Vec<[u64; 3]>,
    pub r_worst: Vec<[i64; 3]>,
    pub r_sum_run: Vec<[i64; 3]>,
}

/// Every config's events of one segment on one day.
#[derive(Clone, Debug, Default)]
pub struct DayEvents {
    pub uev: Vec<UEv>,
    /// Per config: entries in time order (ties: gain descending, lots descending, contract).
    pub lists: Vec<Vec<Ent>>,
    /// [tf][thr] at the base config: events with the window-start reference at or below 10 paisa (0.05 / 0.10 spikes), before the
    /// premium filter, and how many the premium filter removed (raw = minimum volume + top 50 only).
    pub raw_n: Vec<u64>,
    pub raw_tiny: Vec<u64>,
    pub first_open_ref: Vec<u64>,
    /// Events at the base config at or after the 15:08 cut (counted, never traded).
    pub after_cut: Vec<u64>,
    pub after: After,
    /// Screener evaluations (candle end x list) and their wall time (ns), for the p50 / p99.
    pub eval_ns: Vec<u32>,
    pub lists_n: usize,
    /// [tf]: list members evaluated at every candle end, and those skipped (no volume in the window, no price yet, no start reference).
    pub evaluated: Vec<u64>,
    pub skipped: Vec<u64>,
    /// [tf][thr] at the base config: events on a contract expiring that day (index expiry days; stock ones are refused at pricing).
    pub expiry_day: Vec<u64>,
    pub list_sizes: Vec<(u32, usize)>,
    /// Rule R2 (v2): [tf][thr] at the base config, rank-list members dropped as another strike of a key (underlying, side) already
    /// taken at that candle end; and the same over every config.
    pub rank_dup: Vec<u64>,
    pub rank_dup_all: u64,
    /// Rule R3 (v2): stock underlyings on their own expiry day, (rolled to the next expiry, refused for no next-expiry data).
    pub roll: (u32, u32),
}

fn splitmix(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The premium path after slot b (decision at the end of slot b-1): (close at T+m (forward-filled), highest HIGH in slots [b, b+m)).
fn path_after(c: &Cube, ci: usize, b: usize, m: usize) -> (i32, i32) {
    // v3.4 E8: a direct array read, counted outside a loaded segment slice
    c.guard(ci);
    let e = (b + m).min(NM);
    if e <= b {
        return (0, 0);
    }
    let cl = c.close_at(ci, e - 1);
    let mut hi = 0;
    for s in b..e {
        if c.flags.get(Cube::ix(ci, s)).is_some_and(|f| f & 1 == 1) {
            hi = hi.max(c.high.get(Cube::ix(ci, s)).copied().unwrap_or(0));
        }
    }
    (cl, hi)
}

/// Generates every config's events of segment `seg` on cube `c` (day index `di`), current (`next` false) or next expiry, under rule set
/// `v2` (R2 rank-list dedup, R3 stock expiry-day roll) or v1. `keep` (one flag per config): only those configs' lists are stored (the
/// cross-rule evaluations); None = every config.
#[must_use]
pub fn generate(c: &Cube, di: u16, seg: u8, lv: &Levels, next: bool, v2: bool, keep: Option<&[bool]>) -> DayEvents {
    let ls = lists(c, seg, next, v2);
    let mut de = DayEvents { lists: vec![Vec::new(); NCFG], raw_n: vec![0; 10 * NTHR], raw_tiny: vec![0; 10 * NTHR], first_open_ref: vec![0; 10 * NTHR], after_cut: vec![0; 10 * NTHR], ..DayEvents::default() };
    de.lists_n = ls.len();
    de.evaluated = vec![0; 10];
    de.skipped = vec![0; 10];
    de.expiry_day = vec![0; 10 * NTHR];
    de.rank_dup = vec![0; 10 * NTHR];
    de.roll = if v2 && seg == 1 { stock_roll_facts(c) } else { (0, 0) };
    de.list_sizes = ls.iter().map(|(e, l)| (*e, l.len())).collect();
    let nat = 10 * NTHR;
    de.after = After { n: vec![0; nat], lower: vec![[0; 3]; nat], priced: vec![[0; 3]; nat], worst: vec![[0; 3]; nat], sum_run: vec![[0; 3]; nat], r_lower: vec![[0; 3]; nat], r_priced: vec![[0; 3]; nat], r_worst: vec![[0; 3]; nat], r_sum_run: vec![[0; 3]; nat] };
    let mut uix: HashMap<(u32, u32), u32> = HashMap::new();
    let mut buf: Vec<Member> = Vec::new();
    // v3.2 addendum 16 (index lists): the days-to-expiry dial per contract and, for the busiest pick, each underlying's busiest candidate
    // expiry at every slot end (day-so-far volume up to the decision second)
    let exp16 = seg == 0 && !next && crate::sw::on(crate::sw::EXPIRY16);
    let dte_ok: Vec<bool> = if exp16 { c.cons.iter().map(|k| crate::expiry::dte_pass(crate::expiry::bucket(crate::expiry::sessions_to(c.day, crate::cal::day_of_ymd(k.expiry))))).collect() } else { Vec::new() };
    let busy: HashMap<u16, Vec<u32>> = if exp16 && crate::expiry::PICK.load(std::sync::atomic::Ordering::Relaxed) == 3 {
        let st = crate::expiry::staged(c);
        c.unds.iter().enumerate().filter(|(_, u)| u.kind == 0).map(|(ui, u)| {
            let und = u16::try_from(ui).unwrap_or(u16::MAX);
            let cands = crate::expiry::busy_cands(c, und, &st);
            (und, (0..=NM).map(|b| crate::expiry::busiest(c, und, &cands, b).unwrap_or(0)).collect())
        }).collect()
    } else {
        HashMap::new()
    };
    let mut filt: Vec<(usize, Member)> = Vec::with_capacity(NMAX);
    let mut gs: Vec<(usize, Member)> = Vec::with_capacity(NMAX);
    // per (tf, list): the members at every candle end, for the random baseline (top-50 member sets).
    let mut rng = 0x5F1D_EFAD_E000_0000_u64 ^ (u64::from(c.ymd) << 8) ^ u64::from(seg);
    for (ti, &tf) in TFS.iter().enumerate() {
        let mut ends: Vec<(usize, usize, i64)> = Vec::new();
        for k in 0..NM {
            match window(tf, k) {
                Some(w) => ends.push(w),
                None => break,
            }
        }
        // the per-T events of this tf, merged across lists, before ordering
        for &(a, b, t) in &ends {
            let mut at_t: Vec<(usize, Ent)> = Vec::new();
            for (_, l) in &ls {
                let t0 = std::time::Instant::now();
                members(c, l, a, b, tf == 0, &mut buf);
                if exp16 {
                    buf.retain(|m| {
                        let k = c.cons.get(m.ci as usize);
                        dte_ok.get(m.ci as usize).copied().unwrap_or(false) && (busy.is_empty() || k.is_some_and(|k| busy.get(&k.und).and_then(|v| v.get(b)).copied() == Some(k.expiry)))
                    });
                }
                de.eval_ns.push(u32::try_from(t0.elapsed().as_nanos()).unwrap_or(u32::MAX));
                if let (Some(ev), Some(sk)) = (de.evaluated.get_mut(ti), de.skipped.get_mut(ti)) {
                    *ev += u64::try_from(l.len()).unwrap_or(0);
                    *sk += u64::try_from(l.len().saturating_sub(buf.len())).unwrap_or(0);
                }
                // raw base view (minimum volume level 0, top 50, no premium / value filter): tiny-start counts
                {
                    let mvl = lv.minvol.get(ti).map_or(1, |x| x[0]);
                    let raw: Vec<&Member> = buf.iter().take_while(|m| m.lots >= mvl).take(NMAX).collect();
                    let mut rs: Vec<&Member> = raw.clone();
                    rs.sort_unstable_by(|x, y| y.gain_bp.cmp(&x.gain_bp).then(y.lots.cmp(&x.lots)).then(x.ci.cmp(&y.ci)));
                    for th in 0..NTHR {
                        let ev: Vec<&&Member> = if let Some(g) = THR.get(th) {
                            rs.iter().take_while(|m| m.gain_bp >= g * 100).collect()
                        } else if v2 {
                            // R2: the best strike per (underlying, side) only; the next distinct key fills the K
                            let kk = RANK.get(th - 9).copied().unwrap_or(0);
                            let mut seen: Vec<u32> = Vec::with_capacity(kk);
                            let mut v: Vec<&&Member> = Vec::with_capacity(kk);
                            for m in rs.iter().filter(|m| m.gain_bp > 0) {
                                if v.len() >= kk {
                                    break;
                                }
                                let key = rank_key(c, m.ci);
                                if seen.contains(&key) {
                                    continue;
                                }
                                seen.push(key);
                                v.push(m);
                            }
                            v
                        } else {
                            rs.iter().filter(|m| m.gain_bp > 0).take(RANK.get(th - 9).copied().unwrap_or(0)).collect()
                        };
                        let i = ti * NTHR + th;
                        if let (Some(n), Some(tn)) = (de.raw_n.get_mut(i), de.raw_tiny.get_mut(i)) {
                            *n += u64::try_from(ev.len()).unwrap_or(0);
                            *tn += u64::try_from(ev.iter().filter(|m| m.c0 <= 10).count()).unwrap_or(0);
                        }
                    }
                }
                for mp in 0..4 {
                    for mval in 0..4 {
                        let pmin = MINPREM.get(mp).copied().unwrap_or(0);
                        let vmin = lv.minval.get(ti).map_or(0, |x| x[mval]);
                        filt.clear();
                        for m in &buf {
                            if i64::from(m.ct) >= pmin && m.val >= vmin {
                                filt.push((filt.len(), *m));
                                if filt.len() >= NMAX {
                                    break;
                                }
                            }
                        }
                        gs.clear();
                        gs.extend_from_slice(&filt);
                        gs.sort_unstable_by(|x, y| y.1.gain_bp.cmp(&x.1.gain_bp).then(y.1.lots.cmp(&x.1.lots)).then(x.1.ci.cmp(&y.1.ci)));
                        for mv in 0..4 {
                            let vl = lv.minvol.get(ti).map_or(1, |x| x[mv]);
                            let cut = filt.iter().take_while(|x| x.1.lots >= vl).count();
                            for (tn, &nn) in topn().iter().enumerate() {
                                let mcut = nn.min(cut);
                                for th in 0..NTHR {
                                    let id = cfg_id(ti, mv, tn, mp, mval, th);
                                    let kept = keep.is_none_or(|kp| kp.get(id).copied().unwrap_or(false));
                                    let mut taken = 0_usize;
                                    let mut tk: [u32; 8] = [u32::MAX; 8];
                                    for &(r, m) in &gs {
                                        if r >= mcut {
                                            continue;
                                        }
                                        let ok = if let Some(g) = THR.get(th) {
                                            if m.gain_bp < g * 100 {
                                                break;
                                            }
                                            true
                                        } else {
                                            let k = RANK.get(th - 9).copied().unwrap_or(0);
                                            if taken >= k || m.gain_bp <= 0 {
                                                break;
                                            }
                                            if v2 {
                                                // R2: another strike of a key already taken at this candle end is dropped; the next key fills K
                                                let key = rank_key(c, m.ci);
                                                if tk.get(..taken).is_some_and(|x| x.contains(&key)) {
                                                    de.rank_dup_all += 1;
                                                    if (mv, tn, mp, mval) == BASE
                                                        && let Some(x) = de.rank_dup.get_mut(ti * NTHR + th)
                                                    {
                                                        *x += 1;
                                                    }
                                                    continue;
                                                }
                                                if let Some(s) = tk.get_mut(taken) {
                                                    *s = key;
                                                }
                                            }
                                            true
                                        };
                                        if !ok {
                                            continue;
                                        }
                                        taken += 1;
                                        let key = (m.ci, u32::try_from(t).unwrap_or(0));
                                        let ue = if let Some(&x) = uix.get(&key) {
                                            x
                                        } else {
                                            let x = u32::try_from(de.uev.len()).unwrap_or(0);
                                            de.uev.push(UEv { di, seg, ci: m.ci, t: key.1 });
                                            uix.insert(key, x);
                                            x
                                        };
                                        if !kept {
                                            // a cross-rule evaluation stores only its configs; the unique-event numbering (the tie order
                                            // inside a candle end) stays that of the full run, so its lists equal the full run's
                                            continue;
                                        }
                                        at_t.push((id, Ent { ue, gain_bp: m.gain_bp, lots: m.lots }));
                                        if (mv, tn, mp, mval) == BASE {
                                            let i = ti * NTHR + th;
                                            if t >= CUT_S {
                                                if let Some(x) = de.after_cut.get_mut(i) {
                                                    *x += 1;
                                                }
                                            }
                                            if c.cons.get(m.ci as usize).is_some_and(|k| k.expiry == c.ymd) {
                                                if let Some(x) = de.expiry_day.get_mut(i) {
                                                    *x += 1;
                                                }
                                            }
                                            if m.first_open_ref {
                                                if let Some(x) = de.first_open_ref.get_mut(i) {
                                                    *x += 1;
                                                }
                                            }
                                            // after-event path of this event and one random baseline sale
                                            if let Some(n) = de.after.n.get_mut(i) {
                                                *n += 1;
                                            }
                                            for (q, &mm) in [5_usize, 15, 30].iter().enumerate() {
                                                let (cl, hi) = path_after(c, m.ci as usize, b, mm);
                                                if b + mm <= NM && cl > 0 {
                                                    if let (Some(p), Some(lw)) = (de.after.priced.get_mut(i), de.after.lower.get_mut(i)) {
                                                        p[q] += 1;
                                                        if cl < m.ct {
                                                            lw[q] += 1;
                                                        }
                                                    }
                                                    let run = (i64::from(hi.max(m.ct)) - i64::from(m.ct)) * 10_000 / i64::from(m.ct);
                                                    if let (Some(w), Some(s)) = (de.after.worst.get_mut(i), de.after.sum_run.get_mut(i)) {
                                                        w[q] = w[q].max(run);
                                                        s[q] += run;
                                                    }
                                                }
                                            }
                                            // random baseline: a random candle end of this tf, a random top-50 member of the same list there
                                            if !ends.is_empty() {
                                                let r1 = usize::try_from(splitmix(&mut rng) % u64::try_from(ends.len()).unwrap_or(1)).unwrap_or(0);
                                                if let Some(&(ra, rb, _)) = ends.get(r1) {
                                                    let mut rb2 = Vec::new();
                                                    members(c, l, ra, rb, tf == 0, &mut rb2);
                                                    rb2.truncate(NMAX);
                                                    if !rb2.is_empty() {
                                                        let r2 = usize::try_from(splitmix(&mut rng) % u64::try_from(rb2.len()).unwrap_or(1)).unwrap_or(0);
                                                        if let Some(rm) = rb2.get(r2) {
                                                            for (q, &mm) in [5_usize, 15, 30].iter().enumerate() {
                                                                let (cl, hi) = path_after(c, rm.ci as usize, rb, mm);
                                                                if rb + mm <= NM && cl > 0 {
                                                                    if let (Some(p), Some(lw)) = (de.after.r_priced.get_mut(i), de.after.r_lower.get_mut(i)) {
                                                                        p[q] += 1;
                                                                        if cl < rm.ct {
                                                                            lw[q] += 1;
                                                                        }
                                                                    }
                                                                    let run = (i64::from(hi.max(rm.ct)) - i64::from(rm.ct)) * 10_000 / i64::from(rm.ct);
                                                                    if let (Some(w), Some(s)) = (de.after.r_worst.get_mut(i), de.after.r_sum_run.get_mut(i)) {
                                                                        w[q] = w[q].max(run);
                                                                        s[q] += run;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // order this T's entries per config: gain desc, lots desc, unique event (stable merge across lists)
            at_t.sort_by(|x, y| x.0.cmp(&y.0).then(y.1.gain_bp.cmp(&x.1.gain_bp)).then(y.1.lots.cmp(&x.1.lots)).then(x.1.ue.cmp(&y.1.ue)));
            let mut last: Option<(usize, u32)> = None;
            for (id, e) in at_t {
                if last == Some((id, e.ue)) {
                    continue;
                }
                last = Some((id, e.ue));
                if let Some(v) = de.lists.get_mut(id) {
                    v.push(e);
                }
            }
        }
    }
    de
}

/// v3.2 (addenda 12b / 13b): one event's place in its list at its decision second under config `id`, by the screener's own steps for that
/// one config (the list of its segment and expiry, the members of the window ending at T, the premium / value filters, the minimum volume,
/// the top N by volume, the gain order; rule R2's one strike per underlying + side for a rank-only config; the addendum 16 filters):
/// (rank by gain (1 = first), rank by volume, members in the top N, the event's member record). None: the event is not a member there.
#[must_use]
pub fn rank_at(c: &Cube, seg: u8, ci: u32, id: usize, lv: &Levels, t: u32) -> Option<(usize, usize, usize, Member)> {
    let (ti, mv, tn, mp, mval, thr) = cfg_parts(id);
    let tf = *TFS.get(ti)?;
    let b = usize::try_from((i64::from(t) - OPEN_S) / 60).ok()?;
    let a = if tf == 0 { 0 } else { b.checked_sub(tf as usize)? };
    let ls = lists(c, seg, false, true);
    let (_, l) = ls.iter().find(|(_, l)| l.contains(&ci))?;
    let mut buf = Vec::new();
    members(c, l, a, b, tf == 0, &mut buf);
    if seg == 0 && crate::sw::on(crate::sw::EXPIRY16) {
        let st = crate::expiry::staged(c);
        let pick3 = crate::expiry::PICK.load(std::sync::atomic::Ordering::Relaxed) == 3;
        buf.retain(|m| {
            let Some(k) = c.cons.get(m.ci as usize) else { return false };
            let dte = crate::expiry::dte_pass(crate::expiry::bucket(crate::expiry::sessions_to(c.day, crate::cal::day_of_ymd(k.expiry))));
            let busy = !pick3 || c.unds.get(usize::from(k.und)).is_some_and(|u| { let _ = u; crate::expiry::busiest(c, k.und, &crate::expiry::busy_cands(c, k.und, &st), b) == Some(k.expiry) });
            dte && busy
        });
    }
    let pmin = MINPREM.get(mp).copied().unwrap_or(0);
    let vmin = lv.minval.get(ti).map_or(0, |x| x[mval]);
    let filt: Vec<Member> = buf.iter().filter(|m| i64::from(m.ct) >= pmin && m.val >= vmin).take(NMAX).copied().collect();
    let vl = lv.minvol.get(ti).map_or(1, |x| x[mv]);
    let cut = filt.iter().take_while(|m| m.lots >= vl).count();
    let mcut = topn().get(tn).copied().unwrap_or(0).min(cut);
    let mut gs: Vec<(usize, Member)> = filt.iter().copied().enumerate().collect();
    gs.sort_unstable_by(|x, y| y.1.gain_bp.cmp(&x.1.gain_bp).then(y.1.lots.cmp(&x.1.lots)).then(x.1.ci.cmp(&y.1.ci)));
    let mut rank = 0_usize;
    let mut seen: Vec<u32> = Vec::new();
    for (r, m) in &gs {
        if *r >= mcut {
            continue;
        }
        if thr >= 9 {
            let key = rank_key(c, m.ci);
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
        }
        rank += 1;
        if m.ci == ci {
            return Some((rank, r + 1, mcut, *m));
        }
    }
    None
}

/// `aggcheck <stage> <cube dir> <day>`: our 1m fold of 3, 5, 10, 15, 30, 60 minutes against Tickvault's own tick-built files, on every
/// option contract of the day: bucket sets, close equal, volume (abs) equal.
///
/// # Errors
/// A missing input.
pub fn cmd_aggcheck(a: &[String]) -> Result<String, String> {
    let (Some(stage), Some(cdir), Some(d)) = (a.get(2), a.get(3), a.get(4)) else { return Err("aggcheck <stage> <cube dir> <day>".to_owned()) };
    let c = crate::cube::load(&PathBuf::from(cdir), d)?;
    let sid_ci: HashMap<u32, usize> = c.cons.iter().enumerate().map(|(i, k)| (k.sid, i)).collect();
    let mut out = String::new();
    for w in [3_usize, 5, 10, 15, 30, 60] {
        let p = PathBuf::from(stage).join(format!("candles_{w}m/{d}/candles_{w}m-{d}.csv.gz"));
        let mut g = crate::csvgz::GzLines::open(&p)?;
        let mut f = Vec::new();
        let (mut n, mut ours_missing, mut close_eq, mut vol_eq, mut both_eq, mut complete_n, mut complete_eq) = (0_u64, 0_u64, 0_u64, 0_u64, 0_u64, 0_u64, 0_u64);
        let mut first = true;
        while let Some(l) = g.next_line()? {
            crate::csvgz::split(l, &mut f);
            if first {
                first = false;
                continue;
            }
            if crate::csvgz::field(l, &f, 5) != b"NSE_FNO" {
                continue;
            }
            let Some(sid) = crate::csvgz::int(crate::csvgz::field(l, &f, 6)).and_then(|x| u32::try_from(x).ok()) else { continue };
            let Some(&ci) = sid_ci.get(&sid) else { continue };
            let Some((_, sod, _)) = crate::csvgz::ts(crate::csvgz::field(l, &f, 0)) else { continue };
            let Ok(s0) = usize::try_from((sod - OPEN_S) / 60) else { continue };
            let s1 = (s0 + w).min(NM);
            if s0 >= NM {
                continue;
            }
            n += 1;
            let tv_close = crate::csvgz::paisa(crate::csvgz::field(l, &f, 11)).map_or(0, |x| x.0);
            let tv_vol = crate::csvgz::int(crate::csvgz::field(l, &f, 12)).unwrap_or(0).unsigned_abs();
            let present: Vec<usize> = (s0..s1).filter(|&s| c.flags.get(Cube::ix(ci, s)).is_some_and(|x| x & 1 == 1)).collect();
            let Some(&lastp) = present.last() else {
                ours_missing += 1;
                continue;
            };
            let our_close = i64::from(c.close.get(Cube::ix(ci, lastp)).copied().unwrap_or(0));
            let our_vol = c.wvol(ci, s0, s1);
            let ce = our_close == tv_close;
            let ve = our_vol == tv_vol;
            close_eq += u64::from(ce);
            vol_eq += u64::from(ve);
            both_eq += u64::from(ce && ve);
            if present.len() == s1 - s0 {
                complete_n += 1;
                complete_eq += u64::from(ce && ve);
            }
        }
        g.finish()?;
        let pct = |x: u64, y: u64| if y == 0 { 0.0 } else { 100.0 * x as f64 / y as f64 };
        let _ = writeln!(out, "{d} {w}m: Tickvault option buckets {n}; ours with no 1m bar inside {ours_missing}; close equal {close_eq} ({:.2}%); volume equal {vol_eq} ({:.2}%); both {both_eq} ({:.2}%); buckets with every 1m bar present {complete_n}, both equal {complete_eq} ({:.2}%)", pct(close_eq, n), pct(vol_eq, n), pct(both_eq, n), pct(complete_eq, complete_n));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    //! Screener extreme cases: a window with no volume is skipped; a contract listed inside the window takes its first open as the start.
    use super::*;
    use crate::cube::{Contract, Cube, Und};

    fn cube() -> Cube {
        let mut c = Cube { ymd: 2026_11_04, day: crate::cal::days_from_civil(2026, 11, 4), ..Cube::default() };
        c.unds.push(Und { name: "NIFTY".to_owned(), kind: 0, spot_seg: 0, spot_sid: 13, nearest: 2026_11_09, next: 0 });
        // contract 0 trades from slot 0; contract 1 is listed at slot 10 (first open Rs 2.00), nothing before
        for (i, first) in [(0_u32, 0_i32), (1, 10)] {
            c.cons.push(Contract { sid: 100 + i, und: 0, kind: 0, right: 0, expiry: 2026_11_09, lot: 65, strike: 2_500_000, first_min: first, first_open: 200, symbol: format!("C{i}"), n_sec: 0 });
        }
        let nc = 2;
        c.close = vec![0; nc * NM];
        c.flags = vec![0; nc * NM];
        c.high = vec![0; nc * NM];
        c.low = vec![0; nc * NM];
        c.vol = vec![0; nc * NM];
        c.oi = vec![0; nc * NM];
        c.cumvol = vec![0; nc * (NM + 1)];
        c.cumval = vec![0; nc * (NM + 1)];
        for ci in 0..nc {
            let mut cv = 0_u64;
            for m in 0..NM {
                let first = usize::try_from(c.cons[ci].first_min).unwrap_or(0);
                // contract 0: volume only in slots 0..5 and 20..; contract 1: from slot 10
                let traded = if ci == 0 { m < 5 || m >= 20 } else { m >= first };
                let px = if ci == 0 { 1_000 } else { 200 + i32::try_from(m).unwrap_or(0) * 10 };
                if m >= first {
                    c.close[Cube::ix(ci, m)] = px;
                    c.flags[Cube::ix(ci, m)] = if traded { 5 } else { 3 };
                }
                if traded && m >= first {
                    c.vol[Cube::ix(ci, m)] = 650;
                    cv += 650;
                }
                c.cumvol[Cube::px(ci, m + 1)] = cv;
            }
        }
        c
    }

    #[test]
    fn zero_volume_window_skipped_and_listed_midday_uses_first_open() {
        let c = cube();
        // contract 0, 5-minute window [5, 10): no volume -> not a member
        assert!(member(&c, 0, 5, 10, false).is_none());
        // contract 0, window [0, 5): volume, start reference = its first open (no earlier close), labelled
        let m = member(&c, 0, 0, 5, false).unwrap_or_else(|| panic!("member"));
        assert!(m.first_open_ref);
        // contract 1 listed at slot 10: window [10, 15) starts at its first open Rs 2.00; close at slot 14 = Rs 3.40 -> +70%
        let m = member(&c, 1, 10, 15, false).unwrap_or_else(|| panic!("member"));
        assert!(m.first_open_ref);
        assert_eq!((m.c0, m.ct, m.gain_bp), (200, 340, 7_000));
        // contract 1, window [15, 20): the start is the close at slot 14 (Rs 3.40), not the first open
        let m = member(&c, 1, 15, 20, false).unwrap_or_else(|| panic!("member"));
        assert!(!m.first_open_ref);
        assert_eq!(m.c0, 340);
        // contract 1 before its listing: no member
        assert!(member(&c, 1, 0, 5, false).is_none());
    }

    #[test]
    fn windows_never_pass_1530_and_follow_the_0915_grid() {
        assert_eq!(window(5, 0), Some((0, 5, OPEN_S + 300)));
        assert_eq!(window(60, 5), Some((300, 360, OPEN_S + 360 * 60)));
        assert_eq!(window(60, 6), None);
        assert_eq!(window(4, 92), Some((368, 372, OPEN_S + 372 * 60)));
        assert_eq!(window(4, 93), None);
        assert_eq!(window(0, 374), Some((0, 375, CLOSE_S)));
        assert_eq!(window(0, 375), None);
    }

    /// One underlying, 4 contracts of one expiry trading every minute (10 lots a minute); at 09:16 the closes jump from Rs 1.00 to: CE A
    /// Rs 4.00 (+300%), CE B Rs 3.00 (+200%), CE C Rs 2.00 (+100%), PE D Rs 1.50 (+50%). `kind` 0 index, 1 stock.
    fn cube4(kind: u8, ymd: u32, nearest: u32, next: u32, exps: [u32; 4]) -> Cube {
        let d = crate::cal::day_of_ymd(ymd);
        let mut c = Cube { ymd, day: d, ..Cube::default() };
        c.unds.push(Und { name: if kind == 0 { "NIFTY".to_owned() } else { "SBIN".to_owned() }, kind, spot_seg: kind, spot_sid: 13, nearest, next });
        let jump = [400, 300, 200, 150];
        for i in 0..4_usize {
            c.cons.push(Contract { sid: 200 + u32::try_from(i).unwrap_or(0), und: 0, kind, right: u8::from(i == 3), expiry: exps[i], lot: 65, strike: 2_500_000 + 10_000 * i64::try_from(i).unwrap_or(0), first_min: 0, first_open: 100, symbol: format!("K{i}"), n_sec: 0 });
        }
        let nc = 4;
        c.close = vec![0; nc * NM];
        c.flags = vec![5; nc * NM];
        c.high = vec![0; nc * NM];
        c.low = vec![0; nc * NM];
        c.vol = vec![650; nc * NM];
        c.oi = vec![0; nc * NM];
        c.cumvol = vec![0; nc * (NM + 1)];
        c.cumval = vec![0; nc * (NM + 1)];
        for ci in 0..nc {
            let (mut cv, mut cl) = (0_u64, 0_u64);
            for m in 0..NM {
                let px = if m == 0 { 100 } else { jump[ci] };
                c.close[Cube::ix(ci, m)] = px;
                c.high[Cube::ix(ci, m)] = px;
                c.low[Cube::ix(ci, m)] = px;
                cv += 650;
                cl += 650 * u64::try_from(px).unwrap_or(0);
                c.cumvol[Cube::px(ci, m + 1)] = cv;
                c.cumval[Cube::px(ci, m + 1)] = cl;
            }
        }
        c
    }

    fn lv1() -> Levels {
        Levels { minvol: [[1; 4]; 10], minval: [[0; 4]; 10], samples: [0; 10] }
    }

    /// The contracts of one config's list (contract index, decision second).
    fn evs_of(de: &DayEvents, id: usize) -> Vec<(u32, u32)> {
        de.lists.get(id).map(|v| v.iter().filter_map(|e| de.uev.get(e.ue as usize).map(|u| (u.ci, u.t))).collect()).unwrap_or_default()
    }

    #[test]
    fn r2_rank_lists_keep_the_best_strike_per_underlying_side() {
        let c = cube4(0, 2026_11_04, 2026_11_09, 0, [2026_11_09; 4]);
        let t = u32::try_from(OPEN_S + 120).unwrap_or(0); // the 1-minute candle 09:16 ends at 09:17
        let (top3, top5, g100) = (cfg_id(0, 0, 0, 0, 0, 10), cfg_id(0, 0, 0, 0, 0, 11), cfg_id(0, 0, 0, 0, 0, 0));
        let v1 = generate(&c, 0, 0, &lv1(), false, false, None);
        assert_eq!(evs_of(&v1, top3), vec![(0, t), (1, t), (2, t)], "v1 stacks CE strikes A, B, C");
        assert_eq!(evs_of(&v1, top5), vec![(0, t), (1, t), (2, t), (3, t)]);
        let v2 = generate(&c, 0, 0, &lv1(), false, true, None);
        assert_eq!(evs_of(&v2, top3), vec![(0, t), (3, t)], "v2: the best CE and the best PE of the underlying, nothing else");
        assert_eq!(evs_of(&v2, top5), vec![(0, t), (3, t)]);
        // gain-threshold lists keep every member (the account refuses the held key: rule R1)
        assert_eq!(evs_of(&v2, g100), evs_of(&v1, g100));
        assert_eq!(evs_of(&v2, g100), vec![(0, t), (1, t), (2, t)]);
        // the base config (top 50) counted the two dropped CE strikes of rank top3
        assert_eq!(v2.rank_dup.get(10).copied(), Some(2));
        // keep: only the kept config's list is stored
        let mut keep = vec![false; NCFG];
        keep[top3] = true;
        let k = generate(&c, 0, 0, &lv1(), false, true, Some(&keep));
        assert_eq!(evs_of(&k, top3), evs_of(&v2, top3));
        assert!(evs_of(&k, top5).is_empty());
    }

    #[test]
    fn r3_stock_expiry_day_lists_the_next_expiry_or_refuses() {
        let (day, next) = (2026_11_23, 2026_12_29);
        // a stock on its own expiry day, next expiry staged: contracts 0, 1 expire today, 2, 3 at the next expiry
        let c = cube4(1, day, day, next, [day, day, next, next]);
        assert_eq!(lists(&c, 1, false, true), vec![(next, vec![2, 3])], "v2 lists the next expiry");
        assert_eq!(lists(&c, 1, false, false), vec![(day, vec![0, 1])], "v1 listed the expiring contracts");
        assert_eq!(stock_roll_facts(&c), (1, 0));
        // no next expiry staged: the expiring list stays and every event of it is refused at pricing
        let c0 = cube4(1, day, day, 0, [day; 4]);
        assert_eq!(lists(&c0, 1, false, true), vec![(day, vec![0, 1, 2, 3])]);
        assert_eq!(stock_roll_facts(&c0), (0, 1));
        let var = std::collections::BTreeMap::new();
        assert_eq!(crate::trade::size_event(&c0, 0, 36_000, &var, true).refuse, crate::trade::Refuse::StockExpiryNoNext);
        // an INDEX underlying on its expiry day keeps its expiring contracts (expiry-day trading allowed, E1-E4)
        let ci = cube4(0, day, day, next, [day, day, next, next]);
        assert_eq!(lists(&ci, 0, false, true), vec![(day, vec![0, 1])]);
        // a stock before its expiry day is unchanged
        let cb = cube4(1, 2026_11_20, day, 0, [day; 4]);
        assert_eq!(lists(&cb, 1, false, true), lists(&cb, 1, false, false));
    }
}
