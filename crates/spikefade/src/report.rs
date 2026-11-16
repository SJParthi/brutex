//! `run`: load the cubes, measure the minimum-volume / traded-value levels per segment, generate every config's events (index and stock
//! separately, never mixed), price every unique event once (the outcome table), sweep every account, re-run the top picks in detail with
//! the random-time twin, and write every output file.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use crate::acct::{NACC, NVIEW, SEv, Sum, Tops, VIEW_LETTERS, VIEW_NAMES, detail_list, n_of, sweep_list, x_parts};
use crate::rules::{Calendar, Rules};
use crate::v2;
use crate::cal::{CUT_S, NM, fmt_day, fmt_rs, fmt_sod, fmt_us, sod_of};
use crate::cube::{Cube, SecTape};
use crate::margin::VarRow;
use crate::screen::{self, DayEvents, Ent, Levels, MINPREM, NCFG, NTHR, TFS, TOPN, cfg_id, cfg_parts, thr_name, tf_name};
use crate::trade::{self, DayTape, NRULE, Out, SHARE_BP, STOP_PCT, TGT_PCT, TIME_MIN, block_of};

macro_rules! lg {
    ($log:expr, $($a:tt)*) => {{
        let s = format!($($a)*);
        eprintln!("{s}");
        let _ = writeln!($log, "{s}");
    }};
}

const SEG_NAME: [&str; 2] = ["INDEX options", "STOCK options"];
const SEG_SHORT: [&str; 2] = ["idx", "stk"];

fn now_ist() -> String {
    // v3.4 E5: under PORTABLE the system clock + 5 h 30 min (no tzdata; the same label)
    if let Some(s) = crate::kit::now_ist() {
        return s;
    }
    std::process::Command::new("/bin/date").args(["+%Y-%m-%d %H:%M:%S IST"]).env("TZ", "Asia/Kolkata").output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).map_or_else(|| "?".to_owned(), |s| s.trim().to_owned())
}

fn pctile(v: &mut [u64], q: u64) -> u64 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    let i = (u64::try_from(v.len() - 1).unwrap_or(0) * q / 100) as usize;
    v.get(i).copied().unwrap_or(0)
}

fn exit_words(x: usize) -> String {
    let (k, j, m, l) = x_parts(x);
    format!("stop +{}% | target {} | time {} | share {}%", STOP_PCT.get(k).copied().unwrap_or(0), TGT_PCT.get(j).map_or("none".to_owned(), |g| format!("-{g}%")), TIME_MIN.get(m).map_or("to 15:10".to_owned(), |t| format!("{t} min")), SHARE_BP.get(l).copied().unwrap_or(0) / 100)
}

pub(crate) fn cfg_words(seg: usize, id: usize, lv: &Levels) -> String {
    let (tf, mv, tn, mp, mval, thr) = cfg_parts(id);
    // v3.3 fix LV_CAUSAL (check-engine-2 M1): the level dials are per-day values (levels-causal.tsv), named by their rule
    if crate::fix33::on(crate::fix33::LV_CAUSAL) {
        let (wv, wl) = crate::fix33::level_words(mv, mval);
        return format!("{} | TF {} | min vol {wv} | top {} | min prem Rs {} | min value {wl} | {}", SEG_SHORT.get(seg).copied().unwrap_or("?"), tf_name(tf), screen::topn().get(tn).copied().unwrap_or(0), MINPREM.get(mp).copied().unwrap_or(0) / 100, thr_name(thr));
    }
    let vmin = lv.minvol.get(tf).map_or(0, |x| x[mv]);
    let vval = lv.minval.get(tf).map_or(0, |x| x[mval]);
    format!("{} | TF {} | min vol {} lots (L{}) | top {} | min prem Rs {} | min value {} (V{}) | {}", SEG_SHORT.get(seg).copied().unwrap_or("?"), tf_name(tf), vmin, mv + 1, screen::topn().get(tn).copied().unwrap_or(0), MINPREM.get(mp).copied().unwrap_or(0) / 100, if vval == 0 { "off".to_owned() } else { format!("Rs {}", fmt_rs(i64::try_from(vval).unwrap_or(0))) }, mval, thr_name(thr))
}

#[derive(Default)]
struct SegRun {
    lv: Levels,
    evs: Vec<SEv>,
    store: Vec<Out>,
    astore: Vec<[u16; NRULE]>,
    /// per config: global event ids in time order
    lists: Vec<Vec<u32>>,
    /// per config: per-day event counts
    day_counts: Vec<Vec<u32>>,
    /// per config: entry facts (gain, lots) aligned with lists
    facts: Vec<Vec<(i64, u64)>>,
    ulist_of: Vec<u32>,
    ulists: Vec<u32>,
    configs_of: Vec<Vec<u32>>,
    tops: Tops,
    agg: Vec<(u64, u64, u64, Option<Sum>)>,
    days: Vec<usize>,
    dev: Vec<DayEvents>,
    next_lists: usize,
    next_contracts: usize,
    price_ns: Vec<u64>,
    sweep_ns: Vec<(u64, u64)>,
    refusals: BTreeMap<&'static str, u64>,
}

/// The run context of the v2 pieces.
pub struct Ctx<'a> {
    pub rules: Rules,
    pub cal: &'a Calendar,
    /// A v1 run directory whose picks are compared side by side (`--compare-v1`).
    pub cmp_v1: Option<&'a Path>,
    /// The staged Tickvault directory (raw 1-minute rows for the evidence; read only).
    pub stage: Option<&'a Path>,
    pub iblk: i64,
    /// v2.1 (checker F-A): the main grid's fill reading (Prorate = the v2.1 reading; Row = the v2 replay switch).
    pub reading: common_fill::CapMode,
    /// v2.1: the v2 run whose headline picks are compared side by side (`--compare-v2`).
    pub cmp_v2: Option<&'a Path>,
    /// v2.1: the independent checker's output folder (`--checker`): its own sensitivity figures are printed beside, read only.
    pub checker: Option<&'a Path>,
}

/// `run <cube dir> <out dir> <days...> --var <C_VAR1> --holidays <NSE holiday tsv> [--rules v1|v2] [--compare-v1 <v1 run dir>]
/// [--stage <stage dir>] [--threads N] [--seeds S] [--index-block 1|2] [--segs idx|stk]`
///
/// # Errors
/// A refused input.
pub fn cmd_run(a: &[String]) -> Result<String, String> {
    let (Some(cdir), Some(odir)) = (a.get(2), a.get(3)) else { return Err("run <cube dir> <out dir> <days...>".to_owned()) };
    let threads: usize = a.iter().position(|x| x == "--threads").and_then(|i| a.get(i + 1)).and_then(|x| x.parse().ok()).unwrap_or(8);
    let seeds: u64 = a.iter().position(|x| x == "--seeds").and_then(|i| a.get(i + 1)).and_then(|x| x.parse().ok()).unwrap_or(20);
    let iblk: i64 = a.iter().position(|x| x == "--index-block").and_then(|i| a.get(i + 1)).and_then(|x| x.parse().ok()).unwrap_or(1);
    if !(1..=2).contains(&iblk) {
        return Err("--index-block 1|2".to_owned());
    }
    trade::INDEX_BLOCK.store(iblk, std::sync::atomic::Ordering::Relaxed);
    // v2.1 (checker F-A): the fill reading of the main grid: prorate (the v2.1 default), anchor, or v2 (the replay switch to v2)
    let reading_code: u8 = match a.iter().position(|x| x == "--fill-reading").and_then(|i| a.get(i + 1)).map(String::as_str) {
        Some("prorate") | None => 2,
        Some("anchor") => 1,
        Some("v2") => 0,
        Some(o) => return Err(format!("--fill-reading prorate|anchor|v2, not {o}")),
    };
    trade::FILL_READING.store(reading_code, std::sync::atomic::Ordering::Relaxed);
    let reading = trade::cap_mode_of(reading_code);
    let segsel: Vec<u8> = match a.iter().position(|x| x == "--segs").and_then(|i| a.get(i + 1)).map(String::as_str) {
        Some("idx") => vec![0],
        Some("stk") => vec![1],
        _ => vec![0, 1],
    };
    let days: Vec<String> = a.iter().skip(4).filter(|x| x.len() == 10 && x.starts_with("20")).cloned().collect();
    let var_path = a.iter().position(|x| x == "--var").and_then(|i| a.get(i + 1)).cloned().ok_or("--var <C_VAR1 file> is required")?;
    let rules = match a.iter().position(|x| x == "--rules").and_then(|i| a.get(i + 1)).map(String::as_str) {
        Some("v1") => Rules { v2: false },
        Some("v2") | None => Rules { v2: true },
        Some(o) => return Err(format!("--rules v1|v2, not {o}")),
    };
    let hol_path = a.iter().position(|x| x == "--holidays").and_then(|i| a.get(i + 1)).cloned().ok_or("--holidays <NSE holiday tsv> is required")?;
    let cal = Calendar::load(Path::new(&hol_path))?;
    let cmp_v1: Option<PathBuf> = a.iter().position(|x| x == "--compare-v1").and_then(|i| a.get(i + 1)).map(PathBuf::from);
    let stage: Option<PathBuf> = a.iter().position(|x| x == "--stage").and_then(|i| a.get(i + 1)).map(PathBuf::from);
    let cmp_v2: Option<PathBuf> = a.iter().position(|x| x == "--compare-v2").and_then(|i| a.get(i + 1)).map(PathBuf::from);
    let checker: Option<PathBuf> = a.iter().position(|x| x == "--checker").and_then(|i| a.get(i + 1)).map(PathBuf::from);
    // v3 (proof P0): --exits-v3 <menu> <sets> --holidays2 <tsv>: the account sweep by the v3 machinery (readings, stage A with every menu
    // rule under the v2 compatibility cut, the v3 kernel) for the S0 settings in this engine's account order; --p0-check: v2.1's own sweep
    // beside, every account compared field by field
    let v3_menu: Option<(PathBuf, PathBuf, PathBuf)> = match (a.iter().position(|x| x == "--exits-v3").and_then(|i| Some((a.get(i + 1)?, a.get(i + 2)?))), a.iter().position(|x| x == "--holidays2").and_then(|i| a.get(i + 1))) {
        (Some((m, s)), Some(h2)) => Some((PathBuf::from(m), PathBuf::from(s), PathBuf::from(h2))),
        (Some(_), None) => return Err("--exits-v3 needs --holidays2 <nse-holidays-2020-2025.tsv>".to_owned()),
        _ => None,
    };
    let p0_check = a.iter().any(|x| x == "--p0-check");
    let cdir = PathBuf::from(cdir);
    let odir = PathBuf::from(odir);
    let started = now_ist();
    // v2.1: run ids sf21-* (the v2.1 prorate reading), sf21anchor-* (the anchor reading), sf21v2read-* (the replay switch to v2); v1 rules add v1
    let rprefix = match reading {
        common_fill::CapMode::Prorate => "sf21",
        common_fill::CapMode::Anchor => "sf21anchor",
        common_fill::CapMode::Row => "sf21v2read",
    };
    let rprefix = if v3_menu.is_some() { "sf3s0" } else { rprefix };
    let run_id = format!("{rprefix}{}-{}{}", if rules.v2 { "" } else { "v1" }, started.replace(['-', ':'], "").replace(' ', "-").trim_end_matches("-IST").to_owned(), if iblk == 2 { "-idx2s" } else { "" });
    // v3: every run of this binary writes under runs-v3 (never the v2 / v2.1 run folders or their RESULTS files)
    let rdir = odir.join("runs-v3").join(&run_id);
    std::fs::create_dir_all(&rdir).map_err(|e| e.to_string())?;
    let mut log = String::new();
    lg!(log, "run {run_id} started {started}; RULES {} (v2 = operator 14 Nov 23:1x: one open position per underlying+side+expiry, rank lists dedup, stock expiry-day roll, Rs 20 L cap); FILL READING {} (engine v2.1, checker F-A; fill crate {} {} step {} + {}); threads {threads}; seeds {seeds}; index block {iblk} s; segments {segsel:?}; days {days:?}; holidays {} ({} rows)", rules.name(), trade::reading_name(reading), common_fill::VERSION, common_fill::RELEASE, common_fill::STEP, common_fill::FA_CHANGE, cal.source, cal.hol.len());
    let t_all = Instant::now();
    // ---- load cubes and tapes in parallel
    let t0 = Instant::now();
    let loaded: Vec<Result<(Cube, SecTape), String>> = std::thread::scope(|s| {
        let hs: Vec<_> = days.iter().map(|d| {
            let cdir = cdir.clone();
            s.spawn(move || -> Result<(Cube, SecTape), String> { Ok((crate::cube::load(&cdir, d)?, crate::cube::load_sec(&cdir, d)?)) })
        }).collect();
        hs.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("load thread".to_owned()))).collect()
    });
    let mut cubes = Vec::new();
    let mut tapes = Vec::new();
    for r in loaded {
        let (c, t) = r?;
        cubes.push(c);
        tapes.push(t);
    }
    lg!(log, "loaded {} cubes in {:.1} s", cubes.len(), t0.elapsed().as_secs_f64());
    let var = crate::margin::load_var(Path::new(&var_path), "30102026")?;
    lg!(log, "C_VAR1 rows {} ({var_path})", var.len());
    let dial = trade::dial()?;
    // ---- window-query micro-benchmark (O(1) evidence): batches of 1,000 random (contract, window) queries
    let bench = bench_windows(&cubes);
    let mut segs: Vec<SegRun> = Vec::new();
    for seg in 0..2_u8 {
        let ts = Instant::now();
        if !segsel.contains(&seg) {
            segs.push(SegRun::default());
            continue;
        }
        let seg_days: Vec<usize> = cubes.iter().enumerate().filter(|(_, c)| c.cons.iter().any(|k| k.kind == seg)).map(|(i, _)| i).collect();
        let seg_cubes: Vec<Cube> = Vec::new();
        let _ = seg_cubes;
        let sub: Vec<&Cube> = seg_days.iter().filter_map(|&i| cubes.get(i)).collect();
        let lv = measure_refs(&sub, seg);
        lg!(log, "{}: levels measured in {:.1} s", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), ts.elapsed().as_secs_f64());
        // events per day, in parallel
        let te = Instant::now();
        let dev: Vec<DayEvents> = std::thread::scope(|s| {
            let hs: Vec<_> = seg_days.iter().map(|&di| {
                let c = cubes.get(di);
                let lv = &lv;
                s.spawn(move || c.map(|c| screen::generate(c, u16::try_from(di).unwrap_or(0), seg, lv, false, rules.v2, None)).unwrap_or_default())
            }).collect();
            hs.into_iter().map(|h| h.join().unwrap_or_default()).collect()
        });
        let mut next_lists = 0;
        let mut next_contracts = 0;
        for &di in &seg_days {
            if let Some(c) = cubes.get(di) {
                let nl = screen::lists(c, seg, true, rules.v2);
                next_lists += nl.len();
                next_contracts += nl.iter().map(|x| x.1.len()).sum::<usize>();
            }
        }
        lg!(log, "{}: events generated in {:.1} s", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), te.elapsed().as_secs_f64());
        // merge: global unique events and config lists
        let mut evs: Vec<SEv> = Vec::new();
        let mut lists: Vec<Vec<u32>> = vec![Vec::new(); NCFG];
        let mut facts: Vec<Vec<(i64, u64)>> = vec![Vec::new(); NCFG];
        let mut day_counts: Vec<Vec<u32>> = vec![vec![0; cubes.len()]; NCFG];
        for de in &dev {
            let off = u32::try_from(evs.len()).unwrap_or(0);
            for u in &de.uev {
                let sk = cubes.get(usize::from(u.di)).map_or(u32::MAX, |c| trade::side_key(c, u.ci as usize));
                evs.push(SEv { di: u.di, ci: u.ci, t: u.t, skey: sk, tab: trade::EvT::default() });
            }
            for (id, l) in de.lists.iter().enumerate() {
                if let (Some(dst), Some(fd), Some(dc)) = (lists.get_mut(id), facts.get_mut(id), day_counts.get_mut(id)) {
                    for e in l {
                        let Ent { ue, gain_bp, lots } = *e;
                        dst.push(off + ue);
                        fd.push((gain_bp, lots));
                        if let Some(x) = de.uev.get(ue as usize).and_then(|u| dc.get_mut(usize::from(u.di))) {
                            *x += 1;
                        }
                    }
                }
            }
        }
        lg!(log, "{}: {} unique events; {} config entries", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), evs.len(), lists.iter().map(Vec::len).sum::<usize>());
        // price unique events, per day in parallel
        let tp = Instant::now();
        let mut by_day: Vec<Vec<usize>> = vec![Vec::new(); cubes.len()];
        for (i, e) in evs.iter().enumerate() {
            if let Some(v) = by_day.get_mut(usize::from(e.di)) {
                v.push(i);
            }
        }
        // work chunks: (day, the events of up to 64 contracts of that day, contract-sorted so each contract-day tape is built once)
        let blk = block_of(seg);
        let mut chunks: Vec<(usize, Vec<usize>)> = Vec::new();
        for (di, idxs) in by_day.iter().enumerate() {
            let mut v = idxs.clone();
            v.sort_by_key(|&i| evs.get(i).map_or((0, 0), |e| (e.ci, e.t)));
            let mut cur: Vec<usize> = Vec::new();
            let mut ncon = 0_usize;
            let mut last_ci = u32::MAX;
            for i in v {
                let ci = evs.get(i).map_or(0, |e| e.ci);
                if ci != last_ci {
                    ncon += 1;
                    last_ci = ci;
                    if ncon > 64 {
                        chunks.push((di, std::mem::take(&mut cur)));
                        ncon = 1;
                    }
                }
                cur.push(i);
            }
            if !cur.is_empty() {
                chunks.push((di, cur));
            }
        }
        type Priced = (Vec<(usize, trade::EvT)>, Vec<Out>, Vec<[u16; NRULE]>, Vec<u64>);
        let next_chunk = AtomicUsize::new(0);
        let priced_all: Mutex<Vec<(usize, Result<Priced, String>)>> = Mutex::new(Vec::new());
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| {
                    loop {
                        let k = next_chunk.fetch_add(1, Ordering::Relaxed);
                        let Some((di, idxs)) = chunks.get(k) else { break };
                        let r = (|| -> Result<Priced, String> {
                            let (Some(c), Some(tp)) = (cubes.get(*di), tapes.get(*di)) else { return Err("day".to_owned()) };
                            let mut store: Vec<Out> = Vec::new();
                            let mut astore: Vec<[u16; NRULE]> = Vec::new();
                            let mut out = Vec::with_capacity(idxs.len());
                            let mut ns = Vec::with_capacity(idxs.len());
                            let mut cache: Option<(u32, Option<DayTape>)> = None;
                            for &i in idxs {
                                let Some(e) = evs.get(i) else { continue };
                                let t0 = Instant::now();
                                if cache.as_ref().is_none_or(|x| x.0 != e.ci) {
                                    cache = Some((e.ci, DayTape::new(c.day, tp.of(e.ci as usize), blk).ok()));
                                }
                                let tape = cache.as_ref().and_then(|x| x.1.as_ref()).and_then(|d| d.tape().ok());
                                let tab = trade::price_event(c, e.ci as usize, e.t, tape.as_ref(), &var, &dial, &mut store, &mut astore, rules.v2)?;
                                ns.push(u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX));
                                out.push((i, tab));
                            }
                            Ok((out, store, astore, ns))
                        })();
                        if let Ok(mut g) = priced_all.lock() {
                            g.push((k, r));
                        }
                    }
                });
            }
        });
        let mut priced = priced_all.into_inner().unwrap_or_default();
        priced.sort_by_key(|x| x.0);
        let mut store: Vec<Out> = Vec::new();
        let mut astore: Vec<[u16; NRULE]> = Vec::new();
        let mut price_ns = Vec::new();
        for (_, r) in priced {
            let (tabs, st, ast, ns) = r?;
            let off = u32::try_from(store.len()).map_err(|_| "store")?;
            let aoff = u32::try_from(astore.len()).map_err(|_| "anchor store")?;
            store.extend_from_slice(&st);
            astore.extend_from_slice(&ast);
            for (i, mut tab) in tabs {
                if !tab.qs.is_empty() && tab.refuse == trade::Refuse::None {
                    tab.off += off;
                    tab.aoff += aoff;
                }
                if let Some(e) = evs.get_mut(i) {
                    e.tab = tab;
                }
            }
            price_ns.extend(ns);
        }
        let mut refusals: BTreeMap<&'static str, u64> = BTreeMap::new();
        for e in &evs {
            *refusals.entry(e.tab.refuse.code()).or_insert(0) += 1;
        }
        lg!(log, "{}: priced {} events in {:.1} s ({} outcomes, {} MB)", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), evs.len(), tp.elapsed().as_secs_f64(), store.len(), store.len() * std::mem::size_of::<Out>() / 1_000_000);
        // unique lists
        let mut ulist_of = vec![u32::MAX; NCFG];
        let mut ulists: Vec<u32> = Vec::new();
        let mut configs_of: Vec<Vec<u32>> = Vec::new();
        let mut seen: HashMap<(u64, usize), Vec<u32>> = HashMap::new();
        for (id, l) in lists.iter().enumerate() {
            if l.is_empty() {
                continue;
            }
            let mut h = 0xcbf2_9ce4_8422_2325_u64;
            for &x in l {
                h ^= u64::from(x);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
            let cands = seen.entry((h, l.len())).or_default();
            let found = cands.iter().copied().find(|&u| ulists.get(u as usize).and_then(|&rep| lists.get(rep as usize)).is_some_and(|r| r == l));
            let u = if let Some(u) = found {
                u
            } else {
                let u = u32::try_from(ulists.len()).unwrap_or(0);
                ulists.push(u32::try_from(id).unwrap_or(0));
                configs_of.push(Vec::new());
                cands.push(u);
                u
            };
            if let Some(x) = ulist_of.get_mut(id) {
                *x = u;
            }
            if let Some(c) = configs_of.get_mut(u as usize) {
                c.push(u32::try_from(id).unwrap_or(0));
            }
        }
        let ndays = u16::try_from(seg_days.len()).unwrap_or(0);
        lg!(log, "{}: {} non-empty configs -> {} unique lists", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), lists.iter().filter(|l| !l.is_empty()).count(), ulists.len());
        let replay = match &v3_menu {
            Some((m, s, h2)) => {
                let r = crate::report3::Replay::new(&cubes, &tapes, &var, &dial, m, s, &[Path::new(&hol_path), h2.as_path()], seg, &evs, &lists, &seg_days, threads)?;
                log.push_str(&r.log);
                eprint!("{}", r.log);
                Some(r)
            }
            None => None,
        };
        let p0_bad = AtomicUsize::new(0);
        let p0_n = AtomicUsize::new(0);
        // sweep
        let tw = Instant::now();
        let mut order: Vec<usize> = (0..ulists.len()).collect();
        order.sort_by_key(|&u| std::cmp::Reverse(ulists.get(u).and_then(|&r| lists.get(r as usize)).map_or(0, Vec::len)));
        let next = AtomicUsize::new(0);
        let tops_all = Mutex::new(Tops::new(3000));
        let agg_all: Mutex<Vec<(u64, u64, u64, Option<Sum>)>> = Mutex::new(vec![(0, 0, 0, None); 10 * NTHR]);
        let sweep_ns: Mutex<Vec<(u64, u64)>> = Mutex::new(Vec::new());
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| {
                    let mut tops = Tops::new(3000);
                    let mut agg: Vec<(u64, u64, u64, Option<Sum>)> = vec![(0, 0, 0, None); 10 * NTHR];
                    let mut sn = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&u) = order.get(i) else { break };
                        let Some(l) = ulists.get(u).and_then(|&r| lists.get(r as usize)) else { continue };
                        let t0 = Instant::now();
                        let sums = match &replay {
                            Some(r) => {
                                let v3s = r.sweep(l, ndays, seg, u32::try_from(u).unwrap_or(0));
                                if p0_check {
                                    let v21 = sweep_list(&evs, l, &store, &astore, ndays, seg, u32::try_from(u).unwrap_or(0), rules.v2);
                                    let bad = v21.len().abs_diff(v3s.len()) + v21.iter().zip(&v3s).filter(|(x, y)| !crate::report3::sum_eq(x, y)).count();
                                    p0_bad.fetch_add(bad, Ordering::Relaxed);
                                    p0_n.fetch_add(v21.len(), Ordering::Relaxed);
                                }
                                v3s
                            }
                            None => sweep_list(&evs, l, &store, &astore, ndays, seg, u32::try_from(u).unwrap_or(0), rules.v2),
                        };
                        sn.push((u64::try_from(t0.elapsed().as_nanos()).unwrap_or(0), u64::try_from(l.len()).unwrap_or(0)));
                        // (tf, thr) aggregates over every config sharing this list
                        let mut tfthr: Vec<usize> = configs_of.get(u).map(|v| v.iter().map(|&id| { let (tf, _, _, _, _, thr) = cfg_parts(id as usize); tf * NTHR + thr }).collect()).unwrap_or_default();
                        tfthr.sort_unstable();
                        tfthr.dedup();
                        let mult = configs_of.get(u).map_or(1, Vec::len) as u64;
                        let spike = configs_of.get(u).is_some_and(|v| v.iter().any(|&id| cfg_parts(id as usize).5 < 9));
                        for sm0 in &sums {
                            let mut sm = *sm0;
                            sm.spike = spike;
                            let sm = &sm;
                            tops.offer(*sm);
                            for &k in &tfthr {
                                if let Some(g) = agg.get_mut(k) {
                                    let m = configs_of.get(u).map_or(0, |v| v.iter().filter(|&&id| { let (tf, _, _, _, _, thr) = cfg_parts(id as usize); tf * NTHR + thr == k }).count()) as u64;
                                    g.0 += m;
                                    if sm.net > 0 {
                                        g.1 += m;
                                    }
                                    if sm.every_day() && sm.trades > 0 {
                                        g.2 += m;
                                    }
                                    if sm.trades > 0 && g.3.is_none_or(|b| sm.net > b.net) {
                                        g.3 = Some(*sm);
                                    }
                                }
                            }
                        }
                        let _ = mult;
                    }
                    if let Ok(mut t) = tops_all.lock() {
                        t.merge(tops);
                    }
                    if let Ok(mut g) = agg_all.lock() {
                        for (a, b) in g.iter_mut().zip(agg) {
                            a.0 += b.0;
                            a.1 += b.1;
                            a.2 += b.2;
                            if let Some(bb) = b.3
                                && a.3.is_none_or(|x| bb.net > x.net)
                            {
                                a.3 = Some(bb);
                            }
                        }
                    }
                    if let Ok(mut v) = sweep_ns.lock() {
                        v.extend(sn);
                    }
                });
            }
        });
        let tops = tops_all.into_inner().unwrap_or_default();
        let agg = agg_all.into_inner().unwrap_or_default();
        let sweep_ns = sweep_ns.into_inner().unwrap_or_default();
        let steps: u64 = sweep_ns.iter().map(|x| x.1).sum::<u64>() * NACC as u64;
        lg!(log, "{}: swept {} unique lists x {} accounts ({} account-event steps) in {:.1} s{}", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), ulists.len(), NACC, steps, tw.elapsed().as_secs_f64(), if replay.is_some() { " (ACCOUNTS BY THE v3 KERNEL: S0 settings, v2 compatibility cut)" } else { "" });
        if replay.is_some() && p0_check {
            lg!(log, "P0-ACCOUNTS {}: {} accounts compared field by field (v3 kernel vs the v2.1 engine's sweep_list on the same lists): {} differ", SEG_NAME.get(usize::from(seg)).copied().unwrap_or("?"), p0_n.load(Ordering::Relaxed), p0_bad.load(Ordering::Relaxed));
        }
        segs.push(SegRun { lv, evs, store, astore, lists, day_counts, facts, ulist_of, ulists, configs_of, tops, agg, days: seg_days, dev, next_lists, next_contracts, price_ns, sweep_ns, refusals });
    }
    // ---- detailed picks, twin, files
    let mut md = String::new();
    let ctx = Ctx { rules, cal: &cal, cmp_v1: cmp_v1.as_deref(), stage: stage.as_deref(), iblk, reading, cmp_v2: cmp_v2.as_deref(), checker: checker.as_deref() };
    write_results(&mut md, &mut log, &rdir, &run_id, &cubes, &tapes, &var, &dial, &segs, &bench, &days, threads, seeds, &cdir, &ctx)?;
    lg!(log, "run {run_id} finished {} after {:.1} s", now_ist(), t_all.elapsed().as_secs_f64());
    std::fs::write(rdir.join("RUN.log"), &log).map_err(|e| e.to_string())?;
    if v3_menu.is_some() {
        std::fs::write(rdir.join("RESULTS-SPIKEFADE-v3-S0-REPLAY.md"), &md).map_err(|e| e.to_string())?;
    } else if rules.v2 && iblk == 1 && reading == common_fill::CapMode::Prorate {
        // v2.1: never touches v1's or v2's RESULTS files; an earlier v21 file is moved aside, never deleted
        // v3: the v2.1 RESULTS file in the state root is never touched by this binary; the run folder only
        std::fs::write(rdir.join("RESULTS-SPIKEFADE-DIAG-v21-by-v3-binary.md"), &md).map_err(|e| e.to_string())?;
    } else {
        // the anchor reading, the v2 replay switch, v1 rules or 2-second index blocks: inside the run folder only
        std::fs::write(rdir.join(format!("RESULTS-SPIKEFADE-DIAG-{}-{}{}.md", if rules.v2 { "v2rules" } else { "v1rules" }, trade::reading_short(reading), if iblk == 2 { "-idx2s" } else { "" })), &md).map_err(|e| e.to_string())?;
    }
    Ok(log)
}

pub(crate) fn measure_refs(cubes: &[&Cube], seg: u8) -> Levels {
    // v3.1 (check-build-v3 D2): the same loop split into per-day samples (one cube at a time: the streamed day-major mode) and the
    // percentile step; the percentiles sort their samples, so the levels are those of the old loop exactly.
    let mut all: Vec<(Vec<u64>, Vec<u64>)> = vec![(Vec::new(), Vec::new()); TFS.len()];
    for c in cubes {
        for (acc, (l, v)) in all.iter_mut().zip(level_samples(c, seg)) {
            acc.0.extend(l);
            acc.1.extend(v);
        }
    }
    levels_from(all)
}

/// v3.1 (D2): one day's screener samples per timeframe (the member lots and values of every list at every candle end).
pub(crate) fn level_samples(c: &Cube, seg: u8) -> Vec<(Vec<u64>, Vec<u64>)> {
    let mut out = Vec::with_capacity(TFS.len());
    let mut buf = Vec::new();
    let ls = screen::lists(c, seg, false, false);
    for &tf in &TFS {
        let mut lots: Vec<u64> = Vec::new();
        let mut vals: Vec<u64> = Vec::new();
        for k in 0..NM {
            let Some((a, b, _)) = screen::window(tf, k) else { break };
            for (_, l) in &ls {
                screen::members(c, l, a, b, tf == 0, &mut buf);
                for m in buf.iter().take(screen::NMAX) {
                    lots.push(m.lots);
                    vals.push(m.val);
                }
            }
        }
        out.push((lots, vals));
    }
    out
}

/// v3.1 (D2): the levels of the samples (every run day's, concatenated in any order).
pub(crate) fn levels_from(mut all: Vec<(Vec<u64>, Vec<u64>)>) -> Levels {
    let mut lv = Levels::default();
    for (ti, (lots, vals)) in all.iter_mut().enumerate() {
        if let Some(s) = lv.samples.get_mut(ti) {
            *s = u64::try_from(lots.len()).unwrap_or(0);
        }
        let q = |v: &mut Vec<u64>, p: u64| -> u64 { r2(pctile(v, p)) };
        let mv = [q(lots, 25), q(lots, 50), q(lots, 75), q(lots, 90)];
        let mvl = [0, q(vals, 25), q(vals, 50), q(vals, 75)];
        if let (Some(a), Some(b)) = (lv.minvol.get_mut(ti), lv.minval.get_mut(ti)) {
            *a = mv;
            *b = mvl;
        }
    }
    lv
}

pub(crate) fn r2(x: u64) -> u64 {
    if x < 100 {
        return x.max(1);
    }
    let mut p = 1_u64;
    while x / p >= 100 {
        p *= 10;
    }
    (x / p) * p
}

struct Bench {
    batch_ns: Vec<u64>,
    n_batches: usize,
}

fn bench_windows(cubes: &[Cube]) -> Bench {
    let mut rng = 0x1234_5678_9abc_def0_u64;
    let mut nx = || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    let mut batch_ns = Vec::new();
    let mut sink = 0_u64;
    for _ in 0..2_000 {
        let Some(c) = cubes.get(usize::try_from(nx() % cubes.len().max(1) as u64).unwrap_or(0)) else { continue };
        if c.cons.is_empty() {
            continue;
        }
        let t0 = Instant::now();
        for _ in 0..1_000 {
            let ci = usize::try_from(nx() % c.cons.len() as u64).unwrap_or(0);
            let a = usize::try_from(nx() % 375).unwrap_or(0);
            let b = (a + 1 + usize::try_from(nx() % 60).unwrap_or(0)).min(NM);
            sink = sink.wrapping_add(c.wvol(ci, a, b)).wrapping_add(c.wval(ci, a, b)).wrapping_add(u64::try_from(c.close_at(ci, b - 1).max(0)).unwrap_or(0));
        }
        batch_ns.push(u64::try_from(t0.elapsed().as_nanos()).unwrap_or(0));
    }
    if sink == 42 {
        batch_ns.push(0);
    }
    let n = batch_ns.len();
    Bench { batch_ns, n_batches: n }
}

#[derive(Clone, Debug, Default)]
struct TradeRow {
    line: String,
    /// v2.1: the trade's `trade_no` in trades-picks.tsv (its position in the pick's decision list), the key of fill-gaps.tsv.
    no: usize,
    net: i64,
    lots1_net: i64,
    di: u16,
    ci: u32,
    t: u32,
    known: i64,
    run_down: i64,
    margin: i64,
    sold: i64,
    charges: i64,
    units: u64,
    lots: u16,
    contract: String,
    und: String,
    exit_kind: &'static str,
    x: Option<i64>,
    buy_value: i64,
    gapped: bool,
    partial: bool,
    after_1530: bool,
    unclosed: u64,
    // v2: the R1 key, the margin in use right after booking, the expiry facts, and the evidence inputs
    skey: u32,
    margin_in_use: i64,
    nopen: usize,
    sessions: i64,
    cls: String,
    kind: u8,
    expiry: u32,
    gain_bp: i64,
    wlots: u64,
    margin_unit: i64,
    margin_lot: i64,
    s: i64,
    trip: trade::Trip,
}

/// Per staged day (cube index): security id -> (the last 1-minute close at or before 15:29:59 of the latest EARLIER staged session that
/// traded it, that session's day), for the day-change column. The first staged day has none (labelled "vs day open").
type Prev = Vec<HashMap<u32, (i32, i64)>>;

fn prev_closes(cubes: &[Cube]) -> Prev {
    let mut order: Vec<usize> = (0..cubes.len()).collect();
    order.sort_by_key(|&i| cubes.get(i).map_or(0, |c| c.day));
    let mut out: Prev = vec![HashMap::new(); cubes.len()];
    let mut last: HashMap<u32, (i32, i64)> = HashMap::new();
    for &i in &order {
        let Some(c) = cubes.get(i) else { continue };
        if let Some(o) = out.get_mut(i) {
            o.clone_from(&last);
        }
        for (ci, k) in c.cons.iter().enumerate() {
            let cl = c.close_at(ci, 374);
            if cl > 0 {
                last.insert(k.sid, (cl, c.day));
            }
        }
    }
    out
}

/// The config shown for an account: the first config of its list (a GAIN-threshold one when `spike`).
fn rep_of(sr: &SegRun, s: &Sum, spike: bool) -> usize {
    let first = sr.ulists.get(s.list as usize).copied().unwrap_or(0) as usize;
    if !spike {
        return first;
    }
    sr.configs_of.get(s.list as usize).and_then(|v| v.iter().copied().find(|&id| cfg_parts(id as usize).5 < 9)).map_or(first, |x| x as usize)
}

fn settings_words(seg: usize, sr: &SegRun, sm: &Sum) -> String {
    let rep = rep_of(sr, sm, sm.spike);
    let n = n_of(usize::from(sm.n));
    format!("{} || {} || N {n} (slot Rs {}) || block {} s", cfg_words(seg, rep, &sr.lv), exit_words(usize::from(sm.x)), fmt_rs(trade::ACCOUNT / n.max(1)), block_of(u8::try_from(seg).unwrap_or(0)))
}

#[allow(clippy::too_many_arguments)]
fn pick_detail(seg: usize, sr: &SegRun, sm: &Sum, cubes: &[Cube], tapes: &[SecTape], dial: &common_fill::Dial, pick: &str, prev: &Prev, ctx: &Ctx<'_>) -> Result<(Vec<TradeRow>, BTreeMap<&'static str, u64>, Sum, u64), String> {
    let u = sm.list as usize;
    let rep = sr.ulists.get(u).copied().ok_or("list")? as usize;
    let list = sr.lists.get(rep).ok_or("list")?;
    let facts = sr.facts.get(rep).ok_or("facts")?;
    let x = usize::from(sm.x);
    let ni = usize::from(sm.n);
    let (dec, s2) = detail_list(&sr.evs, list, &sr.store, &sr.astore, x, ni, sm.ndays, ctx.rules.v2);
    let (k, j, m, l) = x_parts(x);
    let mut rows = Vec::new();
    let mut why: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut mism = 0_u64;
    let (tf, _, _, _, _, thr) = cfg_parts(rep);
    let blk = block_of(u8::try_from(seg).unwrap_or(0));
    let settings = settings_words(seg, sr, sm);
    let n = n_of(ni);
    for (pos, dd) in dec.iter().enumerate() {
        let (eid, code, lots, cand) = (&dd.eid, &dd.code, &dd.lots, &dd.cand);
        *why.entry(code).or_insert(0) += 1;
        if *code != "booked" {
            continue;
        }
        let Some(e) = sr.evs.get(*eid as usize) else { continue };
        let Some(c) = cubes.get(usize::from(e.di)) else { continue };
        let Some(kc) = c.cons.get(e.ci as usize) else { continue };
        let tp = tapes.get(usize::from(e.di)).ok_or("tape")?;
        let dt = DayTape::new(c.day, tp.of(e.ci as usize), blk)?;
        let tape = dt.tape()?;
        let lot = u64::from(kc.lot);
        let qty = u64::from(*lots) * lot;
        // the trade recomputed from scratch at its own quantity (its own sale record, its own triggers, its own exit), and at 1 lot
        let Some((r, anc, cand2, _inb)) = trade::trade_once(&tape, c.day, e.t, lot, qty, l, k, j, m, dial)? else {
            mism += 1;
            continue;
        };
        let r1 = trade::trade_once(&tape, c.day, e.t, lot, lot, l, k, j, m, dial)?.map_or(0, |x| x.0.net);
        let q = usize::from(e.tab.q_of.get(ni).copied().unwrap_or(255));
        let o = sr.store.get(e.tab.idx(l, q, *cand)).copied().unwrap_or_default();
        if i64::from(o.net) != r.net || cand2 != *cand {
            mism += 1;
        }
        let xx = trade::cand_anchor(&anc, cand2);
        let ((up, _), (down, tdown)) = common_fill::runs(&tape, r.e0, if r.known > 0 { r.known } else { tape.session_end() }, false, r.sold_value, r.units);
        let p0 = r.p0;
        let stop_lvl = (p0 * (100 + STOP_PCT.get(k).copied().unwrap_or(0)) + 99) / 100;
        let tgt_lvl = TGT_PCT.get(j).map(|g| p0 - p0 * g / 100);
        let cut = trade::cut_off();
        let pick3 = [anc.get(k).copied().unwrap_or(trade::NEVER), if j < 4 { anc.get(5 + j).copied().unwrap_or(trade::NEVER) } else { trade::NEVER }, if m < 4 { anc.get(9 + m).copied().unwrap_or(trade::NEVER) } else { trade::NEVER }];
        let mn = pick3.iter().copied().min().unwrap_or(trade::NEVER);
        let ek = match cand2 {
            0..=4 => "stop",
            5..=8 => "target",
            9..=12 => "time",
            _ => {
                if mn != trade::NEVER && mn >= cut { "cut->squareoff" } else { "squareoff" }
            }
        };
        let gapped = cand2 < 5 && r.bb_first_px > stop_lvl;
        let (gain, wl) = facts.get(list.iter().position(|z| z == eid).unwrap_or(0)).copied().unwrap_or((0, 0));
        let b = usize::try_from((i64::from(e.t) - crate::cal::OPEN_S) / 60).unwrap_or(1);
        let prem_t = c.close_at(e.ci as usize, b.saturating_sub(1));
        let und = c.unds.get(usize::from(kc.und)).map_or("?".to_owned(), |x| x.name.clone());
        let avg_sale = if r.units > 0 { r.sold_value / i64::try_from(r.units).unwrap_or(1) } else { 0 };
        let margin_blocked = i64::from(*lots) * e.tab.margin_lot;
        // day change: vs the previous staged session's last close (named), else vs the contract's first open of the day (labelled)
        let (dref, dlab) = match prev.get(usize::from(e.di)).and_then(|p| p.get(&kc.sid)) {
            Some(&(pc, pd)) => (pc, format!("prev close {}", fmt_day(pd))),
            None => (kc.first_open, "day open (no earlier staged close)".to_owned()),
        };
        let dchg = if dref > 0 { format!("{:.2}", (f64::from(prem_t) - f64::from(dref)) * 100.0 / f64::from(dref)) } else { "-".to_owned() };
        let tgt_words = TGT_PCT.get(j).map_or("none".to_owned(), |g| format!("-{g}%"));
        let exd = crate::cal::day_of_ymd(kc.expiry);
        let (cls, sessions) = v2::expiry_class(ctx.cal, c, e.ci as usize);
        let time_words = TIME_MIN.get(m).map_or("to 15:10".to_owned(), |t| format!("{t} min"));
        let line = [
            pick.to_owned(),
            (pos + 1).to_string(),
            SEG_SHORT.get(seg).copied().unwrap_or("?").to_owned(),
            und.clone(),
            kc.symbol.clone(),
            kc.sid.to_string(),
            kc.expiry.to_string(),
            fmt_rs(kc.strike),
            if kc.right == 0 { "CE" } else { "PE" }.to_owned(),
            lot.to_string(),
            fmt_day(c.day),
            fmt_sod(i64::from(e.t)),
            tf_name(tf),
            thr_name(thr),
            format!("{:.2}", gain as f64 / 100.0),
            wl.to_string(),
            fmt_rs(i64::from(prem_t)),
            dchg,
            dlab,
            settings.clone(),
            n.to_string(),
            fmt_rs(trade::ACCOUNT / n.max(1)),
            (SHARE_BP.get(l).copied().unwrap_or(0) / 100).to_string(),
            format!("+{}%", STOP_PCT.get(k).copied().unwrap_or(0)),
            tgt_words,
            time_words,
            blk.to_string(),
            fmt_rs(e.tab.s),
            fmt_rs(e.tab.margin_unit),
            fmt_rs(e.tab.margin_lot),
            lots.to_string(),
            e.tab.closest.get(ni).copied().unwrap_or(0).to_string(),
            fmt_rs(margin_blocked),
            fmt_rs(r.limit_in),
            fmt_us(r.e0),
            r.units.to_string(),
            fmt_rs(avg_sale),
            fmt_rs(r.sold_value),
            fmt_rs(p0),
            fmt_rs(stop_lvl),
            tgt_lvl.map_or("-".to_owned(), fmt_rs),
            ek.to_owned(),
            xx.map_or("15:10:00".to_owned(), fmt_sod),
            fmt_rs(r.limit_out),
            fmt_us(r.bb_first),
            fmt_rs(r.bb_first_px),
            r.bb_units.to_string(),
            fmt_rs(r.bb_value),
            r.sq_units.to_string(),
            fmt_rs(r.sq_value),
            r.unclosed.to_string(),
            fmt_rs(r.worst),
            fmt_rs(r.buy_value),
            fmt_rs(r.charges_sell),
            fmt_rs(r.charges_buy),
            fmt_rs(r.charges),
            fmt_rs(r.net),
            fmt_rs(up),
            fmt_rs(down),
            fmt_us(tdown),
            fmt_us(r.known),
            u8::from(gapped).to_string(),
            u8::from(r.units < qty).to_string(),
            u8::from(r.after_1530).to_string(),
            fmt_rs(r1),
            (i64::from(o.net) - r.net).to_string(),
            ctx.rules.name().to_owned(),
            format!("{}|{}|{}", und, if kc.right == 0 { "CE" } else { "PE" }, kc.expiry),
            crate::rules::wd_name(exd).to_owned(),
            u8::from(ctx.cal.is_trading(exd)).to_string(),
            ctx.cal.expiry_words(exd),
            sessions.to_string(),
            cls.clone(),
            fmt_rs(dd.margin_in_use),
            dd.nopen.to_string(),
        ]
        .join("\t");
        rows.push(TradeRow { line, no: pos + 1, net: r.net, lots1_net: r1, di: e.di, ci: e.ci, t: e.t, known: r.known, run_down: down, margin: margin_blocked, sold: r.sold_value, charges: r.charges, units: r.units, lots: *lots, contract: kc.symbol.clone(), und, exit_kind: ek, x: xx, buy_value: r.buy_value, gapped, partial: r.units < qty, after_1530: r.after_1530, unclosed: r.unclosed, skey: e.skey, margin_in_use: dd.margin_in_use, nopen: dd.nopen, sessions, cls, kind: kc.kind, expiry: kc.expiry, gain_bp: gain, wlots: wl, margin_unit: e.tab.margin_unit, margin_lot: e.tab.margin_lot, s: e.tab.s, trip: r.clone() });
    }
    Ok((rows, why, s2, mism))
}

const TRADE_HEAD: &str = "pick\ttrade_no\tsegment\tunderlying\tcontract\tsecurity_id\texpiry\tstrike_rs\tright\tlot\tday\tdecision_T\ttimeframe\tthreshold\twindow_gain_pct\twindow_lots\tpremium_at_T_rs\tday_change_pct\tday_change_ref\tsettings\tN\tslot_rs\tshare_pct\tstop\ttarget\ttime_exit\tblock_s\tunderlying_S_rs\tmargin_unit_rs\tmargin_lot_rs\tlots\tclosest_lots_c11\tmargin_blocked_rs\tsale_limit_rs\tsale_first_block\tunits_sold\tavg_sale_rs\tsold_value_rs\tp0_first_block_rs\tstop_level_vs_p0_rs\ttarget_level_vs_p0_rs\texit_kind\texit_decision_X\tbuyback_limit_rs\tbuyback_first_block\tbuyback_first_px_rs\tbuyback_units\tbuyback_value_rs\tsquareoff_units\tsquareoff_value_rs\tunclosed_units\tunclosed_valued_at_rs\tbuy_value_total_rs\tcharges_sell_side_rs\tcharges_buy_side_rs\tcharges_rs\tnet_rs\trun_up_rs\trun_down_rs\trun_down_at\tlast_fill_known\tgapped_stop\tpartial_sale\tfill_after_1530\tnet_if_1_lot_rs\tsweep_minus_detail_paisa\trules\tside_key\texpiry_weekday\texpiry_is_trading_day\texpiry_note\tsessions_to_expiry\texpiry_class\tmargin_in_use_after_rs\topen_positions_after";

/// Peak margin of overlapping positions of one pick's booked trades (each blocked from T to its last fill).
fn peak_margin(rows: &[TradeRow]) -> i64 {
    let mut ev: Vec<(i64, i64)> = Vec::with_capacity(rows.len() * 2);
    for r in rows {
        let d = i64::from(r.di) * 86_400;
        let end = if r.known > 0 { i64::from(r.di) * 86_400 + sod_of(r.known) } else { d + 86_399 };
        ev.push((d + i64::from(r.t), r.margin));
        ev.push((end, -r.margin));
    }
    ev.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let (mut cur, mut pk) = (0_i64, 0_i64);
    for (_, m) in ev {
        cur += m;
        pk = pk.max(cur);
    }
    pk
}

struct PickOut {
    seg: usize,
    pid: String,
    s: Sum,
    rows: Vec<TradeRow>,
    why: BTreeMap<&'static str, u64>,
    s2: Sum,
    mism: u64,
    tw: Vec<(i64, u32)>,
    twl: String,
}

/// The worst day in words (0 with no losing day = none).
fn wd(s: &Sum) -> String {
    if s.days_neg == 0 { "none (no losing day)".to_owned() } else { fmt_rs(s.worst_day) }
}

/// One view's accounts, deduplicated: accounts with the same (list, N, net, trades, wins, max DD) are one row (their count beside).
fn distinct(best: &[Sum]) -> Vec<(Sum, usize)> {
    let mut out: Vec<(Sum, usize)> = Vec::new();
    for s in best {
        if let Some(x) = out.iter_mut().find(|(o, _)| o.list == s.list && o.n == s.n && o.net == s.net && o.trades == s.trades && o.wins == s.wins && o.mdd == s.mdd) {
            x.1 += 1;
        } else {
            out.push((*s, 1));
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn write_results(md: &mut String, log: &mut String, rdir: &Path, run_id: &str, cubes: &[Cube], tapes: &[SecTape], var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, segs: &[SegRun], bench: &Bench, days: &[String], threads: usize, seeds: u64, cdir: &Path, ctx: &Ctx<'_>) -> Result<(), String> {
    let started = now_ist();
    let prev = prev_closes(cubes);
    let mut body = String::new();
    // ---- views per segment: biggest-profit views first, standing ranking last
    let mut picks: Vec<(usize, String, Sum)> = Vec::new();
    for (seg, sr) in segs.iter().enumerate() {
        let sname = SEG_NAME.get(seg).copied().unwrap_or("?");
        let _ = writeln!(body, "## {sname}: the biggest-profit views (REQ 55) first, the standing ranking (S) last");
        let _ = writeln!(body);
        let _ = writeln!(body, "Label: underlying = each traded option's own underlying ({}); signal source = our Top Volume screener ({} only, one expiry per list) on Tickvault 1-minute candles; traded = {} SOLD naked; size = slot Rs 20 L / N, largest whole lots whose margin estimate fits; period = {} staged days ({}); run `{run_id}`; proof step 1 (in-sample replay, HINDSIGHT: dials chosen on the scored days); Walk-forward test NOT RUN.", if seg == 0 { "NIFTY, BANKNIFTY" } else { "the F&O stocks Tickvault records" }, sname, sname, sr.days.len(), sr.days.iter().map(|&d| cubes.get(d).map_or("?".to_owned(), |c| fmt_day(c.day))).collect::<Vec<_>>().join(", "));
        let _ = writeln!(body);
        for v in 0..NVIEW {
            let best = distinct(&sr.tops.best(v));
            let vl = VIEW_LETTERS.get(v).copied().unwrap_or("?");
            let _ = writeln!(body, "### {} — {sname}", VIEW_NAMES.get(v).copied().unwrap_or("?"));
            let _ = writeln!(body);
            if best.is_empty() {
                let _ = writeln!(body, "No account qualifies in this view.");
                let _ = writeln!(body);
                continue;
            }
            let _ = writeln!(body, "| # | pick | config (one of the configs giving this exact event list) | exits (one of the equal ones) | N | trades | wins | net Rs | max DD Rs | worst trade Rs | worst day Rs | days + / - / traded of {} |", sr.days.len());
            let _ = writeln!(body, "|---|---|---|---|---|---|---|---|---|---|---|---|");
            for (i, (s, eq)) in best.iter().take(10).enumerate() {
                let rep = rep_of(sr, s, v == 1);
                let pid = format!("{}-{vl}{}", SEG_SHORT.get(seg).copied().unwrap_or("?"), i + 1);
                let neq = sr.configs_of.get(s.list as usize).map_or(0, Vec::len);
                let _ = writeln!(body, "| {} | {pid} | {} (+{} equal configs) | {} (+{} equal exit configs) | {} | {} | {} | {} | {} | {} | {} | {} / {} / {} |", i + 1, cfg_words(seg, rep, &sr.lv), neq.saturating_sub(1), exit_words(usize::from(s.x)), eq - 1, n_of(usize::from(s.n)), s.trades, s.wins, fmt_rs(s.net), fmt_rs(s.mdd), fmt_rs(s.worst), wd(s), s.days_pos, s.days_neg, s.days_traded);
                let top = if v <= 1 || v == NVIEW - 1 { 5 } else { 2 };
                if i < top && !picks.iter().any(|p| p.0 == seg && p.2.list == s.list && p.2.x == s.x && p.2.n == s.n) {
                    picks.push((seg, pid, *s));
                }
            }
            let _ = writeln!(body);
        }
    }
    // ---- detailed picks + twin (parallel over picks)
    // hand-check coverage PROBES (not picks): each segment's A1 event list with a 5-minute and a 60-minute time exit and with N 40
    let a1: Vec<(usize, Sum)> = picks.iter().filter(|p| p.1.ends_with("-A1")).map(|p| (p.0, p.2)).collect();
    for (seg, s) in a1 {
        let (k, j, _, l) = x_parts(usize::from(s.x));
        for (pi, (m2, n2)) in [(0_usize, usize::from(s.n)), (3, usize::from(s.n)), (4, 3)].iter().enumerate() {
            let mut q = s;
            q.x = u16::try_from(((k * 5 + j) * 5 + m2) * 5 + l).unwrap_or(0);
            q.n = u8::try_from(*n2).unwrap_or(0);
            picks.push((seg, format!("{}-P{}", SEG_SHORT.get(seg).copied().unwrap_or("?"), pi + 1), q));
        }
    }
    lg!(log, "picks: {} (details + {seeds}-seed twins; P = hand-check coverage probes) from {}", picks.len(), now_ist());
    let next = AtomicUsize::new(0);
    let outs: Mutex<Vec<(usize, Result<PickOut, String>)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((seg, pid, sm)) = picks.get(i) else { break };
                    let r = (|| -> Result<PickOut, String> {
                        let sr = segs.get(*seg).ok_or("seg")?;
                        let (rows, why, s2, mism) = pick_detail(*seg, sr, sm, cubes, tapes, dial, pid, &prev, ctx)?;
                        let (tw, twl) = twin(*seg, sr, sm, cubes, tapes, var, dial, pid, seeds, ctx.rules.v2)?;
                        Ok(PickOut { seg: *seg, pid: pid.clone(), s: *sm, rows, why, s2, mism, tw, twl })
                    })();
                    if let Ok(mut g) = outs.lock() {
                        g.push((i, r));
                    }
                }
            });
        }
    });
    let mut outs = outs.into_inner().unwrap_or_default();
    outs.sort_by_key(|x| x.0);
    let mut po: Vec<PickOut> = Vec::new();
    for (_, r) in outs {
        let mut x = r?;
        if x.pid.contains("-P") {
            let (list, xx, nn, spike) = (x.s.list, x.s.x, x.s.n, x.s.spike);
            x.s = x.s2;
            x.s.list = list;
            x.s.x = xx;
            x.s.n = nn;
            x.s.spike = spike;
        }
        po.push(x);
    }
    lg!(log, "picks done {}", now_ist());
    let mut tsv = String::from(TRADE_HEAD);
    tsv.push('\n');
    let mut twin_tsv = String::from("pick\tseed\ttwin_events\ttwin_trades\ttwin_net_rs\ttwin_max_dd_rs\n");
    let mut pick_tsv = String::from("pick\tsegment\tsettings\tnet_sweep_rs\tnet_detail_rs\ttrades\twins\tmax_dd_rs\tworst_trade_rs\tworst_day_rs\tworst_run_down_rs\tpeak_margin_rs\tnet_if_1_lot_rs\ttrades_per_lot_net_rs\ttwin_median_rs\ttwin_mean_rs\ttwin_best_rs\ttwin_seeds_ge_pick\tsweep_vs_detail_differences\trefusals\trules\tsame_side_dup\tcontract_open\tmargin_cap_20L\tmargin_cash_short\tpeak_margin_account_rs\tsame_side_pairs_open_together\n");
    let mut sec = String::new();
    let _ = writeln!(sec, "## The top picks in detail, with the random-time selling twin beside (every trade in `runs/{run_id}/trades-picks.tsv`)");
    let _ = writeln!(sec);
    let _ = writeln!(sec, "Twin = the same pick's event count, each event replaced by a random candle end (before 15:08) of the same timeframe and day and a random member of the same expiry list's top-N volume set there (the pick's own minimum volume, premium and traded value), sold with the same exits, share level and slot count; {seeds} seeds (seed s = 1..{seeds}, printed in twin.tsv). Time decay alone makes many random sales win: a pick is only interesting where it beats most twin seeds. P1 / P2 / P3 = hand-check coverage PROBES, not picks: the segment's A1 event list with a 5-minute time exit, a 60-minute time exit (exits anchored at or after 15:08 wait for the square-off) and N 40 (its 'sweep' net is the detail re-run, the same step code).");
    let _ = writeln!(sec);
    let _ = writeln!(sec, "| pick | segment | net Rs (sweep) | net Rs (detail re-run) | trades | wins | max DD Rs | worst trade Rs | worst run-down Rs | peak margin Rs (trade intervals / account) | same-side duplicates refused | same-side pairs open together | net if every booked trade were 1 lot Rs | twin median / mean / best net Rs | twin seeds >= pick | sweep vs detail differences | refusals by cause |");
    let _ = writeln!(sec, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    let mut all_rows: Vec<(String, usize, TradeRow)> = Vec::new();
    let mut heads: Vec<String> = Vec::new();
    for p in &po {
        let Some(sr) = segs.get(p.seg) else { continue };
        let worst_rd = p.rows.iter().map(|r| r.run_down).max().unwrap_or(0);
        let lots1: i64 = p.rows.iter().map(|r| r.lots1_net).sum();
        let pk = peak_margin(&p.rows);
        for r in &p.rows {
            tsv.push_str(&r.line);
            tsv.push('\n');
            all_rows.push((p.pid.clone(), p.seg, r.clone()));
        }
        twin_tsv.push_str(&p.twl);
        let mut nets: Vec<i64> = p.tw.iter().map(|x| x.0).collect();
        nets.sort_unstable();
        let med = nets.get(nets.len() / 2).copied().unwrap_or(0);
        let mean = if nets.is_empty() { 0 } else { nets.iter().sum::<i64>() / i64::try_from(nets.len()).unwrap_or(1) };
        let best = nets.last().copied().unwrap_or(0);
        let ge = nets.iter().filter(|&&x| x >= p.s.net).count();
        let whys: Vec<String> = p.why.iter().map(|(k, v)| format!("{k} {v}")).collect();
        let bk: Vec<v2::Bk> = p.rows.iter().map(|r| (r.di, r.ci, r.t, if r.known > 0 { sod_of(r.known) } else { crate::cal::END_S }, r.margin, r.skey, r.net)).collect();
        let r1o = v2::r1_overlaps(&bk);
        let ssd = p.why.get("same_side_dup").copied().unwrap_or(0);
        let _ = writeln!(sec, "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} / {} | {ssd} | {r1o} | {} | {} / {} / {} | {ge} of {} | {} | {} |", p.pid, SEG_SHORT.get(p.seg).copied().unwrap_or("?"), fmt_rs(p.s.net), fmt_rs(p.s2.net), p.s2.trades, p.s2.wins, fmt_rs(p.s2.mdd), fmt_rs(p.s2.worst), fmt_rs(worst_rd), fmt_rs(pk), fmt_rs(p.s2.pk_margin), fmt_rs(lots1), fmt_rs(med), fmt_rs(mean), fmt_rs(best), nets.len(), p.mism, whys.join(", "));
        let _ = writeln!(pick_tsv, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{ge}\t{}\t{}", p.pid, SEG_SHORT.get(p.seg).copied().unwrap_or("?"), settings_words(p.seg, sr, &p.s), fmt_rs(p.s.net), fmt_rs(p.s2.net), p.s2.trades, p.s2.wins, fmt_rs(p.s2.mdd), fmt_rs(p.s2.worst), fmt_rs(p.s2.worst_day), fmt_rs(worst_rd), fmt_rs(pk), fmt_rs(lots1), fmt_rs(if p.rows.is_empty() { 0 } else { lots1 / i64::try_from(p.rows.len()).unwrap_or(1) }), fmt_rs(med), fmt_rs(mean), fmt_rs(best), p.mism, whys.join(", "));
        let _ = pick_tsv.pop();
        let _ = writeln!(pick_tsv, "\t{}\t{ssd}\t{}\t{}\t{}\t{}\t{r1o}", ctx.rules.name(), p.why.get("contract_open").copied().unwrap_or(0), p.why.get("margin_cap_20L").copied().unwrap_or(0), p.why.get("margin_cash_short").copied().unwrap_or(0), fmt_rs(p.s2.pk_margin));
        if p.pid.ends_with("-A1") || p.pid.ends_with("-X1") || p.pid.ends_with("-S1") {
            heads.push(format!("- **{}** ({}): {} → net **Rs {}** on {} trades ({} wins) over {} days, max DD Rs {}, worst trade Rs {}, worst day {}, peak margin Rs {}; random-time twin median Rs {} (best Rs {}, {ge} of {} seeds at or above the pick); per lot (every booked trade 1 lot) Rs {}; rules {}: same-side duplicates refused {ssd}, same-side pairs open together {r1o}; refusals: {}.", p.pid, SEG_NAME.get(p.seg).copied().unwrap_or("?"), settings_words(p.seg, sr, &p.s), fmt_rs(p.s2.net), p.s2.trades, p.s2.wins, sr.days.len(), fmt_rs(p.s2.mdd), fmt_rs(p.s2.worst), if p.s2.days_neg == 0 { "none (no losing day)".to_owned() } else { format!("Rs {}", fmt_rs(p.s2.worst_day)) }, fmt_rs(pk), fmt_rs(med), fmt_rs(best), nets.len(), fmt_rs(lots1), ctx.rules.name(), v2::why_words(&p.why)));
        }
    }
    let _ = writeln!(sec);
    std::fs::write(rdir.join("trades-picks.tsv"), &tsv).map_err(|e| e.to_string())?;
    std::fs::write(rdir.join("twin.tsv"), &twin_tsv).map_err(|e| e.to_string())?;
    std::fs::write(rdir.join("picks.tsv"), &pick_tsv).map_err(|e| e.to_string())?;
    // ---- TradingView-style: Properties + Overview + List of trades of each segment's #1 biggest-net pick and #1 standing pick
    let mut gaps_tsv = String::from("pick\ttrade_no\tday\tdecision_T\tcontract\tunits_sold\tfirst_sale_block\tfirst_print_prev_print\tfirst_print_silence_s\tfirst_sale_block_after_print_before_T\tfilled_blocks\tfilled_blocks_fed_by_silence_gt_block\tunits_filled\tunits_from_silence_fed_blocks\trecorded_volume_filled_blocks\tusable_volume_filled_blocks\tnet_rs\n");
    for p in po.iter().filter(|p| p.pid.ends_with("-A1") || p.pid.ends_with("-X1") || p.pid.ends_with("-S1")) {
        let Some(sr) = segs.get(p.seg) else { continue };
        let rep = sr.ulists.get(p.s.list as usize).copied().unwrap_or(0) as usize;
        let (tf, mv, tn, mp, mval, thr) = cfg_parts(rep);
        let (k, j, m, l) = x_parts(usize::from(p.s.x));
        let n = n_of(usize::from(p.s.n));
        let mut unds: Vec<&str> = p.rows.iter().map(|r| r.und.as_str()).collect();
        unds.sort_unstable();
        unds.dedup();
        let _ = writeln!(sec, "### TradingView-style: {} ({}) — LABELLED DIAGNOSTIC, HINDSIGHT", p.pid, SEG_NAME.get(p.seg).copied().unwrap_or("?"));
        let _ = writeln!(sec);
        let _ = writeln!(sec, "**Properties.** Segment {} (never mixed); underlyings traded: {}; signal source: Top Volume screener, current expiry of each underlying, one expiry per list; timeframe {}; minimum window volume {} lots (L{}); top {} by window volume; minimum premium Rs {}; minimum window traded value {}; event {}; side: CE or PE, whichever spiked; trade: SELL the event contract naked, decision at the candle end T, order from T (REQ 56); stop +{}% of the first fill price above R(s) (REQ 47, S16); target {}; time exit {}; volume share {}% of each {}-second block (REQ 70b{}); 15:08 own-order cut, 15:10 square-off; account Rs 20,00,000; N {} slots of Rs {}; size = largest whole lots whose margin estimate fits the slot (C11 closest count printed beside); margin estimate: {}; charges: Dhan option estimator (s25 sell-diag, dearer of the dated and the 2026-10-30 rate sets); period {}; run `{run_id}`; proof step 1.", SEG_NAME.get(p.seg).copied().unwrap_or("?"), unds.join(", "), tf_name(tf), sr.lv.minvol.get(tf).map_or(0, |x| x[mv]), mv + 1, TOPN.get(tn).copied().unwrap_or(0), MINPREM.get(mp).copied().unwrap_or(0) / 100, { let v = sr.lv.minval.get(tf).map_or(0, |x| x[mval]); if v == 0 { "off".to_owned() } else { format!("Rs {}", fmt_rs(i64::try_from(v).unwrap_or(0))) } }, thr_name(thr), STOP_PCT.get(k).copied().unwrap_or(0), TGT_PCT.get(j).map_or("none".to_owned(), |g| format!("-{g}% of the first fill price below R(s)")), TIME_MIN.get(m).map_or("none (to 15:10)".to_owned(), |t| format!("{t} min after E0")), SHARE_BP.get(l).copied().unwrap_or(0) / 100, block_of(u8::try_from(p.seg).unwrap_or(0)), if p.seg == 0 { "; index options 1-s blocks, REQ 73b/73c" } else { "" }, n, fmt_rs(trade::ACCOUNT / n.max(1)), if p.seg == 0 { "ASSUMED index formula (no SPAN source on disk): per unit max(ceil(S x 6%) - OTM, ceil(S x 3%)) + ceil(S x 2%), + 2% on the option's expiry day" } else { "the s25 sell-diag PROXY: per unit max(ceil(S x VaR%) - OTM, ceil(S x 7.5%)) + ceil(S x ELM%), NSE C_VAR1 of 2026-10-30 (latest on disk; 1-8 sessions stale)" }, sr.days.iter().map(|&d| cubes.get(d).map_or("?".to_owned(), |c| fmt_day(c.day))).collect::<Vec<_>>().join(", "));
        let _ = writeln!(sec);
        let wins = p.rows.iter().filter(|r| r.net > 0).count();
        let losses = p.rows.iter().filter(|r| r.net < 0).count();
        let best_t = p.rows.iter().map(|r| r.net).max().unwrap_or(0);
        let avg = if p.rows.is_empty() { 0 } else { p.rows.iter().map(|r| r.net).sum::<i64>() / i64::try_from(p.rows.len()).unwrap_or(1) };
        let sold: i64 = p.rows.iter().map(|r| r.sold).sum();
        let chg: i64 = p.rows.iter().map(|r| r.charges).sum();
        let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
        for r in &p.rows {
            *kinds.entry(r.exit_kind).or_insert(0) += 1;
        }
        let kw: Vec<String> = kinds.iter().map(|(k, v)| format!("{k} {v}")).collect();
        // v2.1 (checker F-A): every trade's fill blocks re-walked for the sparse-tape facts (FIX 3) and the per-trade evidence columns
        let v21tv = ctx.reading != common_fill::CapMode::Row;
        let mut gf: Vec<RowGap> = Vec::new();
        if v21tv {
            for r in &p.rows {
                gf.push(row_gap(r, l, block_of(u8::try_from(p.seg).unwrap_or(0)), cubes, tapes, dial)?);
            }
            let (nb, ng, ua, ug, rv, uv, fb): (u64, u64, u64, u64, u64, u64, usize) = gf.iter().fold((0, 0, 0, 0, 0, 0, 0), |a, g| (a.0 + u64::from(g.nb), a.1 + u64::from(g.ng), a.2 + g.ua, a.3 + g.ug, a.4 + g.rv, a.5 + g.uv, a.6 + usize::from(g.first_before_t)));
            let _ = writeln!(sec, "**Fill reading (v2.1, checker F-A).** {}: a 1-second row printed after a silence of g seconds carries everything traded since the previous print; only n/g of it counts (n = the silence's seconds inside the block and at or after the order's send); the share {}% applies to the block's exact usable volume, floored once in whole lots; the block price is the block's recorded print(s), not every trade inside it. This pick's filled blocks: {nb}, of them {ng} ({}) fed by a silence longer than the {}-second block, carrying {} of the filled units; recorded volume of the filled blocks {rv} units, usable {uv} units; trades whose first sale block followed a print inside the signal candle (before T) by more than one block: {fb} of {}; full per-block rows in `evidence/{}.md` and per trade in `fill-gaps.tsv`.", trade::reading_name(ctx.reading), SHARE_BP.get(l).copied().unwrap_or(0) / 100, pct1(ng, nb), block_of(u8::try_from(p.seg).unwrap_or(0)), pct1(ug, ua), p.rows.len(), p.pid);
            let _ = writeln!(sec);
        }
        let _ = writeln!(sec, "**Overview.** Net Rs {} (net only, after charges Rs {}); trades {} (wins {wins}, losses {losses}); average trade Rs {}; best trade Rs {}; worst trade Rs {}; worst day Rs {}; max drawdown Rs {}; days positive / negative / traded {} / {} / {} of {}; premium sold (money in) Rs {}; peak margin blocked Rs {}; exits: {}; gapped stops {}; partial sales {}; fills known after 15:30 {}; unclosed units {}.", fmt_rs(p.s2.net), fmt_rs(chg), p.rows.len(), fmt_rs(avg), fmt_rs(best_t), fmt_rs(p.s2.worst), fmt_rs(p.s2.worst_day), fmt_rs(p.s2.mdd), p.s2.days_pos, p.s2.days_neg, p.s2.days_traded, sr.days.len(), fmt_rs(sold), fmt_rs(peak_margin(&p.rows)), kw.join(", "), p.rows.iter().filter(|r| r.gapped).count(), p.rows.iter().filter(|r| r.partial).count(), p.rows.iter().filter(|r| r.after_1530).count(), p.rows.iter().map(|r| r.unclosed).sum::<u64>());
        let _ = writeln!(sec);
        let _ = writeln!(sec, "**List of trades** (first 40 of {}; all in trades-picks.tsv with every field):", p.rows.len());
        let _ = writeln!(sec);
        if v21tv {
            let _ = writeln!(sec, "| # (trade_no in trades-picks.tsv, fill-gaps.tsv, evidence) | day | T | contract | lots | units sold | sold Rs | first sale block | its first print: previous print / silence | filled blocks (fed by a silence > block) | recorded / usable volume of the filled blocks | margin Rs | exit | X | bought back Rs | charges Rs | net Rs | cum net Rs |");
            let _ = writeln!(sec, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
            let mut cum = 0_i64;
            for (i, (r, g)) in p.rows.iter().zip(gf.iter()).enumerate() {
                cum += r.net;
                if i < 40 {
                    let _ = writeln!(sec, "| {} | {} | {} | {} | {} | {} | {} | {} | {} / {} s | {} ({}) | {} / {} | {} | {} | {} | {} | {} | {} | {} |", format!("{} ({})", i + 1, r.no), cubes.get(usize::from(r.di)).map_or("?".to_owned(), |c| fmt_day(c.day)), fmt_sod(i64::from(r.t)), r.contract, r.lots, r.units, fmt_rs(r.sold), g.first_block.map_or("-".to_owned(), fmt_sod), g.first_prev.map_or("-".to_owned(), fmt_sod), g.first_gap, g.nb, g.ng, g.rv, g.uv, fmt_rs(r.margin), r.exit_kind, r.x.map_or("15:10:00".to_owned(), fmt_sod), fmt_rs(r.buy_value), fmt_rs(r.charges), fmt_rs(r.net), fmt_rs(cum));
                }
            }
            let _ = writeln!(sec);
            for (r, g) in p.rows.iter().zip(gf.iter()) {
                let _ = writeln!(gaps_tsv, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", p.pid, r.no, cubes.get(usize::from(r.di)).map_or("?".to_owned(), |c| fmt_day(c.day)), fmt_sod(i64::from(r.t)), r.contract, r.units, g.first_block.map_or("-".to_owned(), fmt_sod), g.first_prev.map_or("-".to_owned(), fmt_sod), g.first_gap, u8::from(g.first_before_t), g.nb, g.ng, g.ua, g.ug, g.rv, g.uv, fmt_rs(r.net));
            }
            continue;
        }
        let _ = writeln!(sec, "| # | day | T | contract | lots | units sold | sold Rs | margin Rs | exit | X | bought back Rs | charges Rs | net Rs | cum net Rs |");
        let _ = writeln!(sec, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
        let mut cum = 0_i64;
        for (i, r) in p.rows.iter().enumerate() {
            cum += r.net;
            if i < 40 {
                let _ = writeln!(sec, "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |", i + 1, cubes.get(usize::from(r.di)).map_or("?".to_owned(), |c| fmt_day(c.day)), fmt_sod(i64::from(r.t)), r.contract, r.lots, r.units, fmt_rs(r.sold), fmt_rs(r.margin), r.exit_kind, r.x.map_or("15:10:00".to_owned(), fmt_sod), fmt_rs(r.buy_value), fmt_rs(r.charges), fmt_rs(r.net), fmt_rs(cum));
            }
        }
        let _ = writeln!(sec);
    }
    if ctx.reading != common_fill::CapMode::Row {
        std::fs::write(rdir.join("fill-gaps.tsv"), &gaps_tsv).map_err(|e| e.to_string())?;
    }
    // ---- worst trades of the picks
    let mut worst: Vec<&(String, usize, TradeRow)> = all_rows.iter().collect();
    worst.sort_by_key(|x| x.2.net);
    let _ = writeln!(sec, "### The 15 worst trades among the picks (full rows in trades-picks.tsv)");
    let _ = writeln!(sec);
    let _ = writeln!(sec, "| pick | segment | day | T | contract | lots | sold Rs | exit | X | bought back Rs | net Rs | gapped stop |");
    let _ = writeln!(sec, "|---|---|---|---|---|---|---|---|---|---|---|---|");
    for (pid, seg, r) in worst.iter().take(15) {
        let _ = writeln!(sec, "| {pid} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |", SEG_SHORT.get(*seg).copied().unwrap_or("?"), cubes.get(usize::from(r.di)).map_or("?".to_owned(), |c| fmt_day(c.day)), fmt_sod(i64::from(r.t)), r.contract, r.lots, fmt_rs(r.sold), r.exit_kind, r.x.map_or("15:10:00".to_owned(), fmt_sod), fmt_rs(r.buy_value), fmt_rs(r.net), u8::from(r.gapped));
    }
    let _ = writeln!(sec);
    let full_v2 = ctx.rules.v2 && ctx.iblk == 1;
    // v2.1 (checker F-A): any capacity reading other than v2's row reading
    let v21 = ctx.reading != common_fill::CapMode::Row;
    if full_v2 {
        let (inl, n_ev, n_ok) = evidence_files(log, rdir, cubes, tapes, dial, segs, &po, ctx)?;
        lg!(log, "evidence: {n_ev} trades written to runs/{run_id}/evidence/, {n_ok} with the evidence walk equal to the engine's trade");
        let _ = writeln!(sec, "## Evidence (d): every trade of the six headline picks, TradingView style");
        let _ = writeln!(sec);
        if v21 {
            let _ = writeln!(sec, "Label: LABELLED DIAGNOSTIC + HINDSIGHT; underlying = each traded option's own underlying (index: NIFTY, BANKNIFTY; stock: the F&O stocks Tickvault records); signal source = Top Volume screener on Tickvault 1-minute candles; traded = the event option SOLD naked, intraday; size = slot Rs 20 L / N; period = the 8 staged days; run `{run_id}`; fill reading {}; proof step 1; Walk-forward test NOT RUN.", trade::reading_name(ctx.reading));
            let _ = writeln!(sec);
            let _ = writeln!(sec, "One file per pick: `runs/{run_id}/evidence/<pick>.md` ({n_ev} trades; the evidence walk re-reads every fill block and equals the engine's trade in {n_ok} of {n_ev}). Each trade shows the signal window's raw 1-minute OHLCV rows and window gain, then every sale / buy-back / square-off block with its 1-second rows (HIGH / LOW / volume), and NEW in v2.1 each row's previous print, its silence g (seconds since that print) and its usable part n/g (n = the silence's seconds inside the block and at or after the order's send), the block's recorded volume beside its USABLE volume (an exact fraction), the share in whole lots of the usable volume, the block price used (sale = lowest LOW, buy = highest HIGH: the block's recorded print(s), not every trade inside it), the exit reason, lots, margin (per lot, blocked, in use), charges and net. The first trade of each headline pick:");
        } else {
            let _ = writeln!(sec, "One file per pick: `runs/{run_id}/evidence/<pick>.md` ({n_ev} trades; the evidence walk re-reads every fill block and equals the engine's trade in {n_ok} of {n_ev}). Each trade shows the signal window's raw 1-minute OHLCV rows and window gain, every sale / buy-back / square-off block with its 1-second rows (HIGH / LOW / volume), the block price used (sale = lowest LOW, buy = highest HIGH), the block volume and the share in whole lots, the exit reason, lots, margin (per lot, blocked, in use), charges and net. The first trade of each headline pick:");
        }
        let _ = writeln!(sec);
        sec.push_str(&inl);
    }
    // ---- the file: header, headline, then the sections
    if v21 {
        let _ = writeln!(md, "# RESULTS-SPIKEFADE-DIAG-v21: SNIPER s25b spike-fade option SELLING from the Top Volume screener, rule set {}, engine v2.1 (checker F-A: the sparse-tape fill capacity)", ctx.rules.name());
        let _ = writeln!(md);
        let _ = writeln!(md, "Run `{run_id}` (rules {}; FILL READING {}), written {started} by the s25b v2.1 BUILDER (main 5b507203 workflow step). **LABELLED DIAGNOSTIC + HINDSIGHT: {} Tickvault days, every dial chosen on the same days it is scored on. Nothing here is a verdict, a walk-forward result or a live candidate (proof step 1 of the ladder: an in-sample replay on recorded Dhan-feed data; Walk-forward test NOT RUN; Train/test NOT RUN; nothing goes live).** Segments are never mixed (operator 14 Nov 19:09 IST): every list, event, account and table below is INDEX options or STOCK options, one expiry per list. Intraday only: every trade closes by the 15:10 square-off; positional holding is not tested here.", ctx.rules.name(), trade::reading_name(ctx.reading), days.len());
        let _ = writeln!(md);
        let _ = writeln!(md, "Days: {}. Signal source = our Top Volume screener on Tickvault 1-minute candles (unchanged from v2: minimum window volume, then top N by window volume, then gain % descending; one segment, one expiry per list); traded = the event option SOLD naked (index options or stock options, never both in one account); size = slot Rs 20 L / N (N 5 / 10 / 20 / 40), the largest whole lot count whose margin estimate fits the slot, lost money never reused, no compounding; fills = REQ 70 with the v2.1 capacity reading (fill crate {} {} + {}): the sale at each block's lowest LOW, the buy-back at its highest HIGH, volume share dial 10 / 25 / 50 / 75 / 100% of each block's USABLE volume, the rest to the next block; blocks of 1 s for INDEX options and 2 s for STOCK options; **the block price is the block's one (or two) recorded print(s), not every trade inside it**; Dhan option charges (the s25 sell-diag estimator); net only.", days.join(", "), common_fill::RELEASE, common_fill::STEP, common_fill::FA_CHANGE);
    } else {
    let _ = writeln!(md, "# RESULTS-SPIKEFADE-DIAG{}: SNIPER s25b spike-fade option SELLING from the Top Volume screener, rule set {}", if ctx.rules.v2 { "-v2" } else { "-v1replay" }, ctx.rules.name());
    let _ = writeln!(md);
    let _ = writeln!(md, "Run `{run_id}` (rules {}: {}), written {started} by the s25b v2 BUILDER (main 5b507203 workflow step). **LABELLED DIAGNOSTIC + HINDSIGHT: {} Tickvault days, every dial chosen on the same days it is scored on. Nothing here is a verdict, a walk-forward result or a live candidate (proof step 1 of the ladder: an in-sample replay on recorded Dhan-feed data; Walk-forward test NOT RUN; Train/test NOT RUN; nothing goes live).** Segments are never mixed (operator 14 Nov 19:09 IST): every list, event, account and table below is INDEX options or STOCK options, one expiry per list. Intraday only: every trade closes by the 15:10 square-off; positional holding is not tested here.", ctx.rules.name(), if ctx.rules.v2 { "one open position per underlying + side + expiry, rank lists keep the best strike per key, stock expiry-day roll to the next expiry, Rs 20 L margin cap" } else { "the v1 rules replayed for the regression check" }, days.len());
    let _ = writeln!(md);
    let _ = writeln!(md, "Days: {}. Signal source = our Top Volume screener on Tickvault 1-minute candles (minimum window volume, then top N by window volume, then gain % descending; one segment, one expiry per list); traded = the event option SOLD naked (index options or stock options, never both in one account); size = slot Rs 20 L / N (N 5 / 10 / 20 / 40), the largest whole lot count whose margin estimate fits the slot, lost money never reused, no compounding; fills = REQ 70 (crate v2.1: the sale at each block's lowest LOW, the buy-back at its highest HIGH, volume share dial 10 / 25 / 50 / 75 / 100%, the rest to the next block; blocks of 1 s for INDEX options (REQ 73b/73c, SNIPER s25 item 4) and 2 s for STOCK options); Dhan option charges (the s25 sell-diag estimator); net only.", days.join(", "));
    }
    let _ = writeln!(md);
    if v21 {
        let _ = writeln!(md, "Sparse-tape fact (checker F-A, binds every v2.1 number): Tickvault's 1-second option rows exist only for seconds with a print; a row after a silence carries every unit traded since the previous print. The median traded contract-day has about 16-18 thousand prints for index options and about 850-3,700 for stock options, of about 22,500 session seconds (see the last column of Tables A and B). v2.1 counts only the part of such a row that falls in the block's own seconds at or after the order's send.");
        let _ = writeln!(md);
    }
    let _ = writeln!(md, "Data facts that bind every number: 2026-11-10 was a holiday; 2026-11-08 is the Muhurat session (option rows 14:21-15:39, no stock options recorded); the day before 2026-11-02 is not staged, so 2026-11-02 has no previous close (its day change is vs the day open, labelled); Tickvault records the CURRENT expiry only (contract 5.2), so the NEXT-expiry run has no contracts (counted below, never filled); no F&O ban list for November 2026 is on disk (the latest is 2026-10-30), so no ban-day refusal could be applied (named, not downloaded); no NSE C_VAR1 file for November 2026 is on disk, so stock-option margins use the 2026-10-30 file (dated before every staged day: no look-ahead; 1-8 sessions stale); index-option margins use an ASSUMED formula (no SPAN source on disk), so every pick also shows its result at 1 lot per trade.");
    let _ = writeln!(md);
    if full_v2 {
        if v21 {
            md.push_str(&v21_top(log, run_id, rdir, cubes, tapes, var, dial, segs, &po, ctx)?);
        } else {
            md.push_str(&v2_top(log, run_id, rdir, cubes, tapes, var, dial, segs, &po, ctx)?);
        }
    }
    let _ = writeln!(md, "## Headline (biggest net first, REQ 55; standing pick beside), rules {}", ctx.rules.name());
    let _ = writeln!(md);
    let _ = writeln!(md, "Label: LABELLED DIAGNOSTIC + HINDSIGHT; underlying = each option's own underlying; signal source = Top Volume screener on Tickvault 1-minute candles; traded = the event option SOLD naked, intraday (15:10 square-off); size = slot Rs 20 L / N; period = the staged days; proof step 1; Walk-forward test NOT RUN.");
    let _ = writeln!(md);
    for h in &heads {
        let _ = writeln!(md, "{h}");
    }
    let _ = writeln!(md);
    // the best account per event threshold (over every timeframe and every other dial), per segment
    for (seg, sr) in segs.iter().enumerate() {
        if sr.days.is_empty() {
            continue;
        }
        let sname = SEG_NAME.get(seg).copied().unwrap_or("?");
        let _ = writeln!(md, "### {sname}: the best account per event threshold (any timeframe, any other dial; HINDSIGHT)");
        let _ = writeln!(md);
        let _ = writeln!(md, "Label: underlying = {}; signal source = Top Volume screener ({sname} only, one expiry per list) on Tickvault 1-minute candles; traded = {sname} SOLD naked, intraday; size = slot Rs 20 L / N; period = {} staged days; rules {}; proof step 1 (HINDSIGHT); Walk-forward test NOT RUN. 'Accounts with net > 0 of all' counts only configs that have events (configs with no event are not in the denominator).", if seg == 0 { "NIFTY, BANKNIFTY" } else { "the F&O stocks Tickvault records" }, sr.days.len(), ctx.rules.name());
        let _ = writeln!(md);
        let _ = writeln!(md, "| threshold | events (base filters, all timeframes) | best net Rs | timeframe | config | exits | N | trades | max DD Rs | accounts with net > 0 of all |");
        let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|");
        for th in 0..NTHR {
            let mut best: Option<(usize, Sum)> = None;
            let (mut acc, mut pos) = (0_u64, 0_u64);
            for ti in 0..10 {
                if let Some(g) = sr.agg.get(ti * NTHR + th) {
                    acc += g.0;
                    pos += g.1;
                    if let Some(b) = g.3
                        && best.is_none_or(|x| b.net > x.1.net)
                    {
                        best = Some((ti, b));
                    }
                }
            }
            let (bmv, btn, bmp, bmval) = screen::BASE;
            let nev: u64 = (0..10).map(|ti| sr.lists.get(cfg_id(ti, bmv, btn, bmp, bmval, th)).map_or(0, |l| l.len() as u64)).sum();
            match best {
                Some((ti, b)) => {
                    let rep = sr.configs_of.get(b.list as usize).and_then(|v| v.iter().copied().find(|&id| { let (tf, _, _, _, _, t2) = cfg_parts(id as usize); tf == ti && t2 == th })).unwrap_or(0) as usize;
                    let _ = writeln!(md, "| {} | {nev} | {} | {} | {} | {} | {} | {} | {} | {pos} of {acc} |", thr_name(th), fmt_rs(b.net), tf_name(ti), cfg_words(seg, rep, &sr.lv), exit_words(usize::from(b.x)), n_of(usize::from(b.n)), b.trades, fmt_rs(b.mdd));
                }
                None => {
                    let _ = writeln!(md, "| {} | {nev} | no account traded | - | - | - | - | - | - | {pos} of {acc} |", thr_name(th));
                }
            }
        }
        let _ = writeln!(md);
    }
    md.push_str(&body);
    md.push_str(&sec);
    // ---- per segment tables
    let mut lv_tsv = String::from("segment\ttimeframe\tsamples\tminvol_L1_lots\tL2\tL3\tL4\tminval_V0\tV1_paisa\tV2_paisa\tV3_paisa\n");
    let mut ev_tsv = String::from("segment\tconfig_id\tconfig\ttotal_events\tper_day\n");
    let mut base_tsv = String::from("segment\tday\ttimeframe\tthreshold\tevents_base\n");
    let mut after_tsv = String::from("segment\ttimeframe\tthreshold\tevents\tlower_5m\tlower_15m\tlower_30m\tpriced_5m\tpriced_15m\tpriced_30m\tworst_run_5m_pct\tworst_run_15m_pct\tworst_run_30m_pct\tmean_run_5m_pct\tmean_run_15m_pct\tmean_run_30m_pct\trandom_lower_5m\trandom_lower_15m\trandom_lower_30m\trandom_priced_5m\trandom_priced_15m\trandom_priced_30m\trandom_worst_run_30m_pct\n");
    let mut where_tsv = String::from("segment\ttimeframe\tthreshold\taccounts\tnet_positive\tevery_day_positive\tbest_net_rs\tbest_config\tbest_exits\tbest_N\tbest_trades\tbest_max_dd_rs\n");
    for (seg, sr) in segs.iter().enumerate() {
        let sname = SEG_NAME.get(seg).copied().unwrap_or("?");
        let ss = SEG_SHORT.get(seg).copied().unwrap_or("?");
        for ti in 0..10 {
            let a = sr.lv.minvol.get(ti).copied().unwrap_or([0; 4]);
            let b = sr.lv.minval.get(ti).copied().unwrap_or([0; 4]);
            let _ = writeln!(lv_tsv, "{ss}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", tf_name(ti), sr.lv.samples.get(ti).copied().unwrap_or(0), a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]);
        }
        for id in 0..NCFG {
            let tot = sr.lists.get(id).map_or(0, Vec::len);
            let pd: Vec<String> = sr.day_counts.get(id).map(|v| sr.days.iter().map(|&d| v.get(d).copied().unwrap_or(0).to_string()).collect()).unwrap_or_default();
            let _ = writeln!(ev_tsv, "{ss}\t{id}\t{}\t{tot}\t{}", cfg_words(seg, id, &sr.lv), pd.join(","));
        }
        let (bmv, btn, bmp, bmval) = screen::BASE;
        // event counts per day x tf x thr at the base config
        let _ = writeln!(md, "## {sname}: event counts per day x timeframe x threshold (base filters: min volume L1, top 50, min premium Rs 1, traded value off)");
        let _ = writeln!(md);
        let mut head = String::from("| timeframe | threshold |");
        let mut sep = String::from("|---|---|");
        for &d in &sr.days {
            let _ = write!(head, " {} |", cubes.get(d).map_or("?".to_owned(), |c| fmt_day(c.day).get(5..).unwrap_or("").to_owned()));
            sep.push_str("---|");
        }
        head.push_str(" total | at/after 15:08 (never traded) | start ref = first open | raw (no premium filter) | raw with start <= Rs 0.10 |");
        sep.push_str("---|---|---|---|---|");
        let _ = writeln!(md, "{head}");
        let _ = writeln!(md, "{sep}");
        for ti in 0..10 {
            for th in 0..NTHR {
                let id = cfg_id(ti, bmv, btn, bmp, bmval, th);
                let mut row = format!("| {} | {} |", tf_name(ti), thr_name(th));
                let mut tot = 0_u64;
                for &d in &sr.days {
                    let n = sr.day_counts.get(id).and_then(|v| v.get(d)).copied().unwrap_or(0);
                    tot += u64::from(n);
                    let _ = write!(row, " {n} |");
                    let _ = writeln!(base_tsv, "{ss}\t{}\t{}\t{}\t{n}", cubes.get(d).map_or("?".to_owned(), |c| fmt_day(c.day)), tf_name(ti), thr_name(th));
                }
                let i = ti * NTHR + th;
                let ac: u64 = sr.dev.iter().map(|de| de.after_cut.get(i).copied().unwrap_or(0)).sum();
                let fo: u64 = sr.dev.iter().map(|de| de.first_open_ref.get(i).copied().unwrap_or(0)).sum();
                let rn: u64 = sr.dev.iter().map(|de| de.raw_n.get(i).copied().unwrap_or(0)).sum();
                let rt: u64 = sr.dev.iter().map(|de| de.raw_tiny.get(i).copied().unwrap_or(0)).sum();
                let _ = writeln!(md, "{row} {tot} | {ac} | {fo} | {rn} | {rt} |");
            }
        }
        let _ = writeln!(md);
        // after-event
        let _ = writeln!(md, "## {sname}: what happened after each event (base filters), with the random-time baseline beside");
        let _ = writeln!(md);
        let _ = writeln!(md, "Each horizon counts only the events with that many minutes left before 15:39 (so a 5-minute worst run can exceed the 15-minute one). Lower = the premium (1-minute close) m minutes after T is below the premium at T; worst run = the highest 1-minute HIGH within m minutes after T against the premium at T (a seller's worst move). Random = one random top-50 member of the same list at one random candle end of the same timeframe and day per event (seeded).");
        let _ = writeln!(md);
        let _ = writeln!(md, "| timeframe | threshold | events | lower 5m | lower 15m | lower 30m | worst run 5m | worst run 15m | worst run 30m | mean run 30m | random lower 5m | random lower 15m | random lower 30m | random worst run 30m |");
        let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
        for ti in 0..10 {
            for th in 0..NTHR {
                let i = ti * NTHR + th;
                let sum3 = |f: &dyn Fn(&DayEvents) -> [u64; 3]| -> [u64; 3] { sr.dev.iter().fold([0; 3], |a, de| { let x = f(de); [a[0] + x[0], a[1] + x[1], a[2] + x[2]] }) };
                let mx3 = |f: &dyn Fn(&DayEvents) -> [i64; 3]| -> [i64; 3] { sr.dev.iter().fold([0; 3], |a, de| { let x = f(de); [a[0].max(x[0]), a[1].max(x[1]), a[2].max(x[2])] }) };
                let sm3 = |f: &dyn Fn(&DayEvents) -> [i64; 3]| -> [i64; 3] { sr.dev.iter().fold([0; 3], |a, de| { let x = f(de); [a[0] + x[0], a[1] + x[1], a[2] + x[2]] }) };
                let n: u64 = sr.dev.iter().map(|de| de.after.n.get(i).copied().unwrap_or(0)).sum();
                let lw = sum3(&|de| de.after.lower.get(i).copied().unwrap_or([0; 3]));
                let pr = sum3(&|de| de.after.priced.get(i).copied().unwrap_or([0; 3]));
                let wr = mx3(&|de| de.after.worst.get(i).copied().unwrap_or([0; 3]));
                let sr3 = sm3(&|de| de.after.sum_run.get(i).copied().unwrap_or([0; 3]));
                let rl = sum3(&|de| de.after.r_lower.get(i).copied().unwrap_or([0; 3]));
                let rp = sum3(&|de| de.after.r_priced.get(i).copied().unwrap_or([0; 3]));
                let rw = mx3(&|de| de.after.r_worst.get(i).copied().unwrap_or([0; 3]));
                let pc = |a: u64, b: u64| if b == 0 { "-".to_owned() } else { format!("{:.1}%", 100.0 * a as f64 / b as f64) };
                let bp = |x: i64| format!("{:.1}%", x as f64 / 100.0);
                let mean30 = if pr[2] == 0 { 0 } else { sr3[2] / i64::try_from(pr[2]).unwrap_or(1) };
                let _ = writeln!(after_tsv, "{ss}\t{}\t{}\t{n}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.2}", tf_name(ti), thr_name(th), lw[0], lw[1], lw[2], pr[0], pr[1], pr[2], wr[0] as f64 / 100.0, wr[1] as f64 / 100.0, wr[2] as f64 / 100.0, if pr[0] == 0 { 0.0 } else { sr3[0] as f64 / 100.0 / pr[0] as f64 }, if pr[1] == 0 { 0.0 } else { sr3[1] as f64 / 100.0 / pr[1] as f64 }, mean30 as f64 / 100.0, rl[0], rl[1], rl[2], rp[0], rp[1], rp[2], rw[2] as f64 / 100.0);
                if n == 0 {
                    continue;
                }
                let _ = writeln!(md, "| {} | {} | {n} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |", tf_name(ti), thr_name(th), pc(lw[0], pr[0]), pc(lw[1], pr[1]), pc(lw[2], pr[2]), bp(wr[0]), bp(wr[1]), bp(wr[2]), bp(mean30), pc(rl[0], rp[0]), pc(rl[1], rp[1]), pc(rl[2], rp[2]), bp(rw[2]));
            }
        }
        let _ = writeln!(md);
        // where the money is (best per tf x thr)
        let _ = writeln!(md, "## {sname}: best account per timeframe x threshold (over every other dial)");
        let _ = writeln!(md);
        let _ = writeln!(md, "| timeframe | threshold | accounts | net > 0 | every day + | best net Rs | its config / exits / N | trades | max DD Rs |");
        let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|");
        for ti in 0..10 {
            for th in 0..NTHR {
                let Some(g) = sr.agg.get(ti * NTHR + th) else { continue };
                let (acc, pos, ed, best) = *g;
                let (bn, bc, bx, bnn, bt, bd) = best.map_or(("-".to_owned(), "-".to_owned(), "-".to_owned(), 0, 0, "-".to_owned()), |s| {
                    let rep = sr.configs_of.get(s.list as usize).and_then(|v| v.iter().copied().find(|&id| { let (tf, _, _, _, _, thr) = cfg_parts(id as usize); tf == ti && thr == th })).unwrap_or(0) as usize;
                    (fmt_rs(s.net), cfg_words(seg, rep, &sr.lv), exit_words(usize::from(s.x)), n_of(usize::from(s.n)), s.trades, fmt_rs(s.mdd))
                });
                let _ = writeln!(where_tsv, "{ss}\t{}\t{}\t{acc}\t{pos}\t{ed}\t{bn}\t{bc}\t{bx}\t{bnn}\t{bt}\t{bd}", tf_name(ti), thr_name(th));
                if acc == 0 {
                    continue;
                }
                let _ = writeln!(md, "| {} | {} | {acc} | {pos} | {ed} | {bn} | {bc} / {bx} / N {bnn} | {bt} | {bd} |", tf_name(ti), thr_name(th));
            }
        }
        let _ = writeln!(md);
        // levels + refusals + engine facts
        let _ = writeln!(md, "## {sname}: measured dial levels, refusals, engine facts");
        let _ = writeln!(md);
        let _ = writeln!(md, "Minimum volume levels (lots in the window) = the 25th / 50th / 75th / 90th percentiles of the window volume of the top-50-by-volume members over every candle end of every staged day, per timeframe (2 significant digits); minimum traded value levels = off / 25th / 50th / 75th percentile of their window traded value (sum of 1-minute volume x 1-minute close).");
        let _ = writeln!(md);
        let _ = writeln!(md, "| timeframe | samples | min vol L1 / L2 / L3 / L4 (lots) | min value V1 / V2 / V3 (Rs) |");
        let _ = writeln!(md, "|---|---|---|---|");
        for ti in 0..10 {
            let a = sr.lv.minvol.get(ti).copied().unwrap_or([0; 4]);
            let b = sr.lv.minval.get(ti).copied().unwrap_or([0; 4]);
            let _ = writeln!(md, "| {} | {} | {} / {} / {} / {} | {} / {} / {} |", tf_name(ti), sr.lv.samples.get(ti).copied().unwrap_or(0), a[0], a[1], a[2], a[3], fmt_rs(i64::try_from(b[1]).unwrap_or(0)), fmt_rs(i64::try_from(b[2]).unwrap_or(0)), fmt_rs(i64::try_from(b[3]).unwrap_or(0)));
        }
        let _ = writeln!(md);
        let rf: Vec<String> = sr.refusals.iter().map(|(k, v)| format!("{k}: {v}")).collect();
        let mut pn = sr.price_ns.clone();
        let p50 = pctile(&mut pn, 50);
        let p99 = pctile(&mut pn, 99);
        let mut ev_ns: Vec<u64> = sr.dev.iter().flat_map(|de| de.eval_ns.iter().map(|&x| u64::from(x))).collect();
        let e50 = pctile(&mut ev_ns, 50);
        let e99 = pctile(&mut ev_ns, 99);
        let mut per_ev: Vec<u64> = sr.sweep_ns.iter().filter(|x| x.1 > 0).map(|x| x.0 / x.1).collect();
        let s50 = pctile(&mut per_ev, 50);
        let s99 = pctile(&mut per_ev, 99);
        let entries: usize = sr.lists.iter().map(Vec::len).sum();
        let ul_entries: u64 = sr.sweep_ns.iter().map(|x| x.1).sum();
        let lsz: Vec<String> = sr.dev.iter().zip(&sr.days).map(|(de, &d)| format!("{}: {} lists ({})", cubes.get(d).map_or("?".to_owned(), |c| fmt_day(c.day)), de.lists_n, de.list_sizes.iter().map(|(e, n)| format!("expiry {e}: {n} contracts")).collect::<Vec<_>>().join(", "))).collect();
        let _ = writeln!(md, "- Lists per day (current expiry of each underlying; one segment, one expiry per list): {}.", lsz.join("; "));
        let _ = writeln!(md, "- NEXT-expiry run: {} lists, {} contracts over the staged days (Tickvault records the current expiry only, contract 5.2): **not runnable on this data**, counted, never filled.", sr.next_lists, sr.next_contracts);
        let _ = writeln!(md, "- Unique events (day, contract, decision second) {}; config entries {entries}; non-empty configs {} of {NCFG}; unique event lists {} (accounts swept {} = lists x {NACC}; account-event steps {}).", sr.evs.len(), sr.lists.iter().filter(|l| !l.is_empty()).count(), sr.ulists.len(), sr.ulists.len() * NACC, ul_entries * NACC as u64);
        let _ = writeln!(md, "- Event-level refusals (same for every dial): {}.", rf.join(", "));
        let _ = writeln!(md, "- Timing: screener evaluation (one list at one candle end, O(list)) p50 {e50} ns, p99 {e99} ns; event pricing (outcome table) p50 {} us, p99 {} us; sweep per list-event (all {NACC} accounts) p50 {} us, p99 {} us.", p50 / 1_000, p99 / 1_000, s50 / 1_000, s99 / 1_000);
        let _ = writeln!(md);
    }
    let mut bn = bench.batch_ns.clone();
    let b50 = pctile(&mut bn, 50);
    let b99 = pctile(&mut bn, 99);
    let _ = writeln!(md, "## Engine: O(1) window query evidence");
    let _ = writeln!(md);
    let _ = writeln!(md, "Window query = window volume + window traded value + close at T for one (contract, window): three prefix / array reads. {} batches of 1,000 random queries over random days: per query p50 {:.1} ns, p99 {:.1} ns (batch time / 1,000).", bench.n_batches, b50 as f64 / 1_000.0, b99 as f64 / 1_000.0);
    let _ = writeln!(md);
    extreme_section(md, cubes, segs, cdir, days, &all_rows);
    std::fs::write(rdir.join("levels.tsv"), &lv_tsv).map_err(|e| e.to_string())?;
    std::fs::write(rdir.join("events-all-configs.tsv"), &ev_tsv).map_err(|e| e.to_string())?;
    std::fs::write(rdir.join("events-base.tsv"), &base_tsv).map_err(|e| e.to_string())?;
    std::fs::write(rdir.join("after-events.tsv"), &after_tsv).map_err(|e| e.to_string())?;
    std::fs::write(rdir.join("where-best.tsv"), &where_tsv).map_err(|e| e.to_string())?;
    // top lists as TSV
    let mut top_tsv = String::from("segment\tview\trank\tconfig\tequal_configs\texits\tN\ttrades\twins\tnet_rs\tmax_dd_rs\tworst_trade_rs\tdays_pos\tdays_neg\tdays_traded\n");
    for (seg, sr) in segs.iter().enumerate() {
        for v in 0..NVIEW {
            for (i, s) in sr.tops.best(v).iter().enumerate() {
                let rep = sr.ulists.get(s.list as usize).copied().unwrap_or(0) as usize;
                let _ = writeln!(top_tsv, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", SEG_SHORT.get(seg).copied().unwrap_or("?"), VIEW_LETTERS.get(v).copied().unwrap_or("?"), i + 1, cfg_words(seg, rep, &sr.lv), sr.configs_of.get(s.list as usize).map_or(0, Vec::len), exit_words(usize::from(s.x)), n_of(usize::from(s.n)), s.trades, s.wins, fmt_rs(s.net), fmt_rs(s.mdd), fmt_rs(s.worst), s.days_pos, s.days_neg, s.days_traded);
            }
        }
    }
    std::fs::write(rdir.join("top-accounts.tsv"), &top_tsv).map_err(|e| e.to_string())?;
    lg!(log, "results written {}", now_ist());
    Ok(())
}

/// The random-time selling twin of one pick: for every event of the pick's list, one random candle end of the same timeframe and day
/// (before 15:08) and one random member of the same expiry list's top-N volume set there (the pick's own minimum volume, minimum premium
/// and minimum traded value), sold with the same exits and slot count. Seeds 1..=n. Returns ((net, trades) per seed, tsv lines).
#[allow(clippy::too_many_arguments)]
fn twin(seg: usize, sr: &SegRun, s: &Sum, cubes: &[Cube], tapes: &[SecTape], var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, pid: &str, n: u64, v2r: bool) -> Result<(Vec<(i64, u32)>, String), String> {
    let rep = sr.ulists.get(s.list as usize).copied().ok_or("list")? as usize;
    let list = sr.lists.get(rep).ok_or("list")?;
    let (tf, mv, tn, mp, mval, _) = cfg_parts(rep);
    let tfm = TFS.get(tf).copied().unwrap_or(1);
    let vmin = sr.lv.minvol.get(tf).map_or(1, |x| x[mv]);
    let valmin = sr.lv.minval.get(tf).map_or(0, |x| x[mval]);
    let pmin = MINPREM.get(mp).copied().unwrap_or(0);
    let topn = TOPN.get(tn).copied().unwrap_or(10);
    let mut out = Vec::new();
    let mut lines = String::new();
    let mut tape_cache: HashMap<(u16, u32), Option<DayTape>> = HashMap::new();
    let mut buf = Vec::new();
    for seed in 1..=n {
        let mut rng = 0xA5A5_0000_0000_0000_u64 ^ (seed << 20) ^ u64::from(s.list) ^ (u64::from(s.x) << 40);
        let mut nx = || {
            rng = rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = rng;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        let mut evs: Vec<SEv> = Vec::new();
        let mut store: Vec<Out> = Vec::new();
        let mut astore: Vec<[u16; NRULE]> = Vec::new();
        for &eid in list {
            let Some(e) = sr.evs.get(eid as usize) else { continue };
            let Some(c) = cubes.get(usize::from(e.di)) else { continue };
            let Some(kc) = c.cons.get(e.ci as usize) else { continue };
            let ls = screen::lists(c, u8::try_from(seg).unwrap_or(0), false, v2r);
            let Some((_, l)) = ls.iter().find(|(ex, _)| *ex == kc.expiry) else { continue };
            let ends: Vec<(usize, usize, i64)> = (0..NM).map_while(|k| screen::window(tfm, k)).filter(|w| w.2 < CUT_S).collect();
            if ends.is_empty() {
                continue;
            }
            let mut picked: Option<(u32, u32)> = None;
            for _ in 0..20 {
                let Some(&(a, b, t)) = ends.get(usize::try_from(nx() % ends.len() as u64).unwrap_or(0)) else { continue };
                screen::members(c, l, a, b, tfm == 0, &mut buf);
                let set: Vec<u32> = buf.iter().filter(|m| i64::from(m.ct) >= pmin && m.val >= valmin).take(screen::NMAX).take_while(|m| m.lots >= vmin).take(topn).map(|m| m.ci).collect();
                if set.is_empty() {
                    continue;
                }
                let ci = set.get(usize::try_from(nx() % set.len() as u64).unwrap_or(0)).copied().unwrap_or(0);
                picked = Some((ci, u32::try_from(t).unwrap_or(0)));
                break;
            }
            let Some((ci, t)) = picked else { continue };
            let tp = tapes.get(usize::from(e.di)).ok_or("tape")?;
            let dt = tape_cache.entry((e.di, ci)).or_insert_with(|| DayTape::new(c.day, tp.of(ci as usize), block_of(u8::try_from(seg).unwrap_or(0))).ok());
            let tape = dt.as_ref().and_then(|d| d.tape().ok());
            let tab = trade::price_event(c, ci as usize, t, tape.as_ref(), var, dial, &mut store, &mut astore, v2r)?;
            evs.push(SEv { di: e.di, ci, t, skey: trade::side_key(c, ci as usize), tab });
        }
        let mut idx: Vec<u32> = (0..u32::try_from(evs.len()).unwrap_or(0)).collect();
        idx.sort_by_key(|&i| evs.get(i as usize).map_or((0, 0), |e| (e.di, e.t)));
        let (_, sm) = detail_list(&evs, &idx, &store, &astore, usize::from(s.x), usize::from(s.n), s.ndays, v2r);
        let _ = writeln!(lines, "{pid}\t{seed}\t{}\t{}\t{}\t{}", evs.len(), sm.trades, fmt_rs(sm.net), fmt_rs(sm.mdd));
        out.push((sm.net, sm.trades));
    }
    Ok((out, lines))
}

/// The cube manifests' build statistics of one day: (build seconds, stat name -> value).
fn manifest_stats(cdir: &Path, d: &str) -> (f64, BTreeMap<String, u64>) {
    let mut out = BTreeMap::new();
    let mut secs = 0.0;
    let Ok(t) = std::fs::read_to_string(cdir.join(format!("{d}.manifest.tsv"))) else { return (secs, out) };
    for l in t.lines() {
        if let Some(x) = l.strip_prefix("build_seconds\t") {
            secs = x.trim().parse().unwrap_or(0.0);
        }
        if let Some(x) = l.strip_prefix("stats\t") {
            let body = x.trim().trim_start_matches("Stats {").trim_end_matches('}');
            for kv in body.split(',') {
                if let Some((k, v)) = kv.split_once(':') {
                    out.insert(k.trim().to_owned(), v.trim().parse().unwrap_or(0));
                }
            }
        }
    }
    (secs, out)
}

/// EXTREME CASES: every case of the brief, named, with its count on this data and how the engine treats it.
#[allow(clippy::too_many_arguments)]
fn extreme_section(md: &mut String, cubes: &[Cube], segs: &[SegRun], cdir: &Path, days: &[String], rows: &[(String, usize, TradeRow)]) {
    let mut tot: BTreeMap<String, u64> = BTreeMap::new();
    let mut builds: Vec<(String, f64)> = Vec::new();
    for d in days {
        let (s, m) = manifest_stats(cdir, d);
        builds.push((d.clone(), s));
        for (k, v) in m {
            *tot.entry(k).or_insert(0) += v;
        }
    }
    let g = |k: &str| tot.get(k).copied().unwrap_or(0);
    // cross-day identity checks: the same security id with another lot, strike, expiry or underlying; strike lists per (underlying, expiry)
    let mut order: Vec<usize> = (0..cubes.len()).collect();
    order.sort_by_key(|&i| cubes.get(i).map_or(0, |c| c.day));
    let mut seen: HashMap<u32, (u32, i64, u32, String, i64)> = HashMap::new();
    let (mut lot_chg, mut strike_chg, mut exp_chg, mut und_chg) = (0_u64, 0_u64, 0_u64, 0_u64);
    let mut lot_examples: Vec<String> = Vec::new();
    let mut adj: BTreeMap<String, (u64, String)> = BTreeMap::new();
    let mut sets: HashMap<(String, u32), (i64, std::collections::BTreeSet<(i64, u8)>)> = HashMap::new();
    let (mut strikes_added, mut strikes_removed, mut list_pairs) = (0_u64, 0_u64, 0_u64);
    for &i in &order {
        let Some(c) = cubes.get(i) else { continue };
        let mut today: HashMap<(String, u32), std::collections::BTreeSet<(i64, u8)>> = HashMap::new();
        for k in &c.cons {
            let un = c.unds.get(usize::from(k.und)).map_or(String::new(), |u| u.name.clone());
            if let Some((lot, st, ex, u0, d0)) = seen.get(&k.sid) {
                if *lot != k.lot {
                    lot_chg += 1;
                    if lot_examples.len() < 5 {
                        lot_examples.push(format!("{} ({}: lot {} on {} -> {} on {})", k.symbol, k.sid, lot, fmt_day(*d0), k.lot, fmt_day(c.day)));
                    }
                }
                if *st != k.strike {
                    strike_chg += 1;
                    let e = adj.entry(format!("{un} {} -> {}", fmt_day(*d0), fmt_day(c.day))).or_insert((0, format!("{} -> {}", fmt_rs(*st), fmt_rs(k.strike))));
                    e.0 += 1;
                }
                exp_chg += u64::from(*ex != k.expiry);
                und_chg += u64::from(*u0 != un);
            }
            seen.insert(k.sid, (k.lot, k.strike, k.expiry, un.clone(), c.day));
            today.entry((un, k.expiry)).or_default().insert((k.strike, k.right));
        }
        for (key, set) in today {
            if let Some((_, prev)) = sets.get(&key) {
                list_pairs += 1;
                strikes_added += u64::try_from(set.difference(prev).count()).unwrap_or(0);
                strikes_removed += u64::try_from(prev.difference(&set).count()).unwrap_or(0);
            }
            sets.insert(key, (c.day, set));
        }
    }
    let ev_sum = |seg: usize, f: &dyn Fn(&DayEvents) -> u64| -> u64 { segs.get(seg).map_or(0, |sr| sr.dev.iter().map(f).sum()) };
    let refu = |seg: usize, k: &str| -> u64 { segs.get(seg).and_then(|sr| sr.refusals.get(k).copied()).unwrap_or(0) };
    let tiny = |seg: usize| ev_sum(seg, &|de| de.raw_tiny.iter().sum());
    let rawn = |seg: usize| ev_sum(seg, &|de| de.raw_n.iter().sum());
    let fo = |seg: usize| ev_sum(seg, &|de| de.first_open_ref.iter().sum());
    let sk = |seg: usize| ev_sum(seg, &|de| de.skipped.iter().sum());
    let evd = |seg: usize| ev_sum(seg, &|de| de.evaluated.iter().sum());
    let exd = |seg: usize| ev_sum(seg, &|de| de.expiry_day.iter().sum());
    let trades = rows.len();
    let gapped = rows.iter().filter(|r| r.2.gapped).count();
    let partial = rows.iter().filter(|r| r.2.partial).count();
    let after = rows.iter().filter(|r| r.2.after_1530).count();
    let unclosed = rows.iter().filter(|r| r.2.unclosed > 0).count();
    let exp_tr = rows.iter().filter(|r| cubes.get(usize::from(r.2.di)).is_some_and(|c| r.2.contract.contains(&{ let (y, mo, d) = crate::cal::civil(c.day); format!("{d:02}{}{y}", ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"].get(usize::try_from(mo - 1).unwrap_or(0)).copied().unwrap_or("?")) }))).count();
    let muh = cubes.iter().position(|c| c.ymd == 2026_11_08);
    let muh_first = muh.and_then(|i| cubes.get(i)).map_or("-".to_owned(), |c| c.cons.iter().filter_map(|k| usize::try_from(k.first_min).ok()).min().map_or("-".to_owned(), |m| fmt_sod(crate::cal::OPEN_S + i64::try_from(m).unwrap_or(0) * 60)));
    let muh_trades = muh.map_or(0, |i| rows.iter().filter(|r| usize::from(r.2.di) == i).count());
    let _ = writeln!(md, "## Extreme cases (each named, counted on this data, and how the engine treats it)");
    let _ = writeln!(md);
    let _ = writeln!(md, "Counts: cube = the per-day cube builds (manifests, all staged days); events = the screener at every candle end (index | stock); trades = the booked trades of the detailed picks ({trades}). Unit tests (`cargo test -p spikefade`) cover the cases the data does not show.");
    let _ = writeln!(md);
    let _ = writeln!(md, "| case | on this data | treatment |");
    let _ = writeln!(md, "|---|---|---|");
    let _ = writeln!(md, "| premium 0.05 spikes | events whose window-start premium was Rs 0.10 or less, before the minimum-premium filter (minimum volume L1 + top 50 only, every timeframe x threshold): index {} of {}, stock {} of {} | the minimum premium dial (Rs 1 / 5 / 10 / 20 at T) and the minimum traded value dial drop them before the top-N cut; every event table prints raw vs filtered |", tiny(0), rawn(0), tiny(1), rawn(1));
    let _ = writeln!(md, "| contracts listed mid-day | cube: {} contract-days first print after 09:15; events whose start reference is the contract's first open (listed or first traded inside the window, or day-so-far): index {}, stock {} | gain = vs the first open of the day (labelled `start ref = first open`), never a guessed previous close |", g("contracts_listed_midday"), fo(0), fo(1));
    let _ = writeln!(md, "| windows with no bar or zero volume | list members skipped at candle ends (no volume in the window, no price yet, or no start reference): index {} of {}, stock {} of {} member evaluations; cube: {} option minutes with a bar and volume 0, {} contract-days with no bar at all | a member needs window volume of at least one lot and a price; forward-filled closes are marked (flag bit 1), never counted as volume |", sk(0), evd(0), sk(1), evd(1), g("rows_1m_zero_vol_opt"), g("contracts_no_bar"));
    let _ = writeln!(md, "| missing 1-second bars inside a fill block | trades with a sale cut short of its quantity (blocks without enough printed volume, the rest moving on): {partial} of {trades}; 1-s option rows with volume 0: {} | a block's price and volume come only from its printed seconds; an empty block is jumped in O(1) (crate v2.1), never filled; HAND-CHECK-25 counts blocks with a missing second |", g("rows_1s_zero_vol"));
    let _ = writeln!(md, "| stop gapped through | trades whose first buy-back block price was beyond the stop level: {gapped} of {trades} | the buy-back pays each block's HIGHEST high from the trigger second's end (max(trigger end, E1)), never the stop level; unit test `gapped_stop_pays_block_worst` |");
    let _ = writeln!(md, "| price-band freezes (repeated price, no volume) | cube: {} option minutes with a bar, no volume and the same close as before | a second with volume 0 is not a print (crate `valid`): no fill, no trigger from it; such minutes add no window volume |", g("price_freeze_minutes"));
    let _ = writeln!(md, "| Muhurat session 2026-11-08 | first option bar {muh_first}; booked pick trades that day: {muh_trades}; no stock options recorded that day | normal rules (09:15 grid, 15:08 own-order cut, 15:10 square-off to 15:40); the grid simply has no bars before the session (candles before it do not exist, so no event) |");
    let _ = writeln!(md, "| first staged day without a previous close (2026-11-02) | every 2026-11-02 trade | its day change is vs the contract's first open of the day, labelled `day open (no earlier staged close)`; the window gain never needs a previous day |");
    let _ = writeln!(md, "| index expiry days (NIFTY 2026-11-03 and 2026-11-09 in this span) | events on a contract expiring that day (base filters, every timeframe x threshold): index {}; pick trades in an expiry-day contract: {exp_tr} | index options may trade on their expiry day; every position is squared off intraday (15:10, rest to 15:40), so settlement at intrinsic never arises (crate `settle_intrinsic` unused, named) |", exd(0));
    let _ = writeln!(md, "| stock-option expiry day refused | stock events on their own expiry day: {} refused (`stock_expiry_day`); none occurs in this span (the stock expiry is 2026-11-23/24, after the last staged day) | refused at pricing for every dial (crate `expiry_guard` rule); unit test `stock_expiry_day_refused` |", refu(1, "stock_expiry_day"));
    let _ = writeln!(md, "| OI 0 | cube: {} option minutes with a bar and OI 0 (flag bit 3) | OI is printed, never used in a decision |", g("rows_1m_oi0_opt"));
    let _ = writeln!(md, "| lot-size or strike-list changes inside the span | same security id with another lot {lot_chg}, strike {strike_chg}, expiry {exp_chg}, underlying {und_chg}{}; (underlying, expiry) lists seen on consecutive staged days {list_pairs}: strikes added {strikes_added}, dropped {strikes_removed} (Tickvault picks ATM +/- up to 25 strikes per day for stocks) | every day reads its OWN instrument map (lot, strike, expiry from that day's Dhan snapshot); lists are rebuilt per day, never carried |", if lot_examples.is_empty() && adj.is_empty() { String::new() } else { format!(" ({}{}; a corporate-action strike adjustment keeps the security id; the day-change column compares across it)", lot_examples.join("; "), adj.iter().take(5).map(|(k, (n, ex))| format!("{k}: {n} ids, e.g. strike {ex}")).collect::<Vec<_>>().join("; ")) });
    let _ = writeln!(md, "| duplicates | cube: 1-minute duplicate keys {}, 1-second duplicates {}, unsorted 1-second rows {} | the first row per (contract, minute / second) is kept, the rest counted |", g("rows_1m_dup"), g("rows_1s_dup"), g("rows_1s_unsorted"));
    let _ = writeln!(md, "| CRLF | the staged files are LF (the importer strips CR); the reader strips a CR if present (`csvgz::next_line`); HAND-CHECK-25 counts raw CR lines | quoted CSV, empty = NULL, signed candle volume read as abs |");
    let _ = writeln!(md, "| options printing to 15:39:59 after 15:30 | cube: {} option 1-minute rows and {} 1-second rows at or after 15:30; pick trades with a fill known after 15:30: {after}; trades with units never bought back (valued at the highest HIGH from E0 to 15:40): {unclosed} | no event is decided after 15:30 (and none traded at or after the 15:08 cut); the 15:10 square-off may fill in those seconds (flagged `fill_after_1530`) |", g("rows_1m_after_1530_opt"), g("rows_1s_after_1530"));
    let _ = writeln!(md, "| events at or after the 15:08 cut | refused `after_cut_1508`: index {}, stock {} unique events | counted, never traded (B1 cut) |", refu(0, "after_cut_1508"), refu(1, "after_cut_1508"));
    let _ = writeln!(md, "| F&O ban days | no ban list for November 2026 on disk (latest 2026-10-30) | no refusal could be applied; named here and in the header, nothing downloaded |");
    let _ = writeln!(md);
    // engine: per-day build p50 / p99
    let mut bs: Vec<u64> = builds.iter().map(|(_, s)| (s * 1_000.0) as u64).collect();
    let b50 = pctile(&mut bs.clone(), 50);
    let b99 = pctile(&mut bs, 99);
    let _ = writeln!(md, "## Engine: per-day cube builds (incremental, days in parallel)");
    let _ = writeln!(md);
    let _ = writeln!(md, "Per-day build wall time (all staged days built at once, one process per day, from the staged csv.gz through gzip -dc): p50 {:.1} s, p99 {:.1} s ({}). A cube is rebuilt only when a staged input's size/mtime changes AND its sha256 differs (manifest `cube/<day>.manifest.tsv`); otherwise `spikefade cube` answers `cube current`.", b50 as f64 / 1_000.0, b99 as f64 / 1_000.0, builds.iter().map(|(d, s)| format!("{d} {s:.1} s")).collect::<Vec<_>>().join(", "));
    let _ = writeln!(md);
    // our 1m fold vs Tickvault's own tick-built timeframes (aggcheck files written beside the cubes)
    let mut any = false;
    for d in days {
        if let Ok(t) = std::fs::read_to_string(cdir.join(format!("aggcheck-{d}.txt"))) {
            if !any {
                let _ = writeln!(md, "## Our 1-minute fold vs Tickvault's own 3 / 5 / 10 / 15 / 30 / 60-minute files (every option contract of the sampled day)");
                let _ = writeln!(md);
                let _ = writeln!(md, "Expected differences (contract 1.5, 4.4, 4.6): Tickvault folds its timeframes from ticks, so a late tick's quantity carried into the next bucket and the 09:15 bucket's vendor day-open/high/low differ from a 1-minute fold; a bucket with a missing 1-minute bar is incomplete.");
                let _ = writeln!(md);
                any = true;
            }
            for l in t.lines() {
                let _ = writeln!(md, "- {l}");
            }
        }
    }
    if any {
        let _ = writeln!(md);
    }
}

/// One headline config to compare: (segment, pick id, config id, exit id, slot index).
type CmpCfg = (usize, String, usize, usize, usize);

fn cmp_row(md: &mut String, pick: &str, who: &str, ev: &v2::Eval, r1: Option<usize>) {
    let w = &ev.why;
    let g = |k: &str| w.get(k).copied().unwrap_or(0);
    let other: u64 = w.iter().filter(|(k, _)| !["booked", "same_side_dup", "contract_open", "slots_full", "margin_cap_20L", "margin_cash_short"].contains(k)).map(|(_, v)| *v).sum();
    let _ = writeln!(md, "| {pick} | {who} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |", ev.events, fmt_rs(ev.s.net), ev.s.trades, ev.s.wins, g("same_side_dup"), g("contract_open"), g("slots_full"), g("margin_cap_20L"), g("margin_cash_short"), other, fmt_rs(ev.s.pk_margin), fmt_rs(ev.pk_rows), fmt_rs(ev.s.mdd), fmt_rs(ev.s.worst), wd(&ev.s), r1.map_or("-".to_owned(), |x| x.to_string()));
}

const CMP_HEAD: &str = "| pick | rules / source | events in list | net Rs | trades | wins | same-side duplicates refused | same contract open | slots full | margin cap 20 L | cash short | other refusals | peak margin in use Rs (account) | peak margin Rs (trade intervals) | max DD Rs | worst trade Rs | worst day Rs | same-side pairs open together |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|";

/// v1's printed numbers of a pick (picks.tsv) as an Eval-like row: (net, trades, wins, max DD, worst, worst day, peak margin, refusals).
fn v1_printed(md: &mut String, pick: &str, p: &v2::V1Pick) {
    let c = |k: &str| p.cols.get(k).cloned().unwrap_or_default();
    let refs = c("refusals");
    let get = |name: &str| -> String { refs.split(", ").find_map(|x| x.strip_prefix(&format!("{name} "))).unwrap_or("0").to_owned() };
    let other: u64 = refs.split(", ").filter_map(|x| x.rsplit_once(' ')).filter(|(k, _)| !["booked", "same_side_dup", "contract_open", "slots_full", "margin_cap_20L", "margin_cash_short"].contains(k)).filter_map(|(_, v)| v.parse::<u64>().ok()).sum();
    let _ = writeln!(md, "| {pick} | v1 run `sf-20261114-215542` (printed) | {} | {} | {} | {} | (not a v1 rule) | {} | {} | (not a v1 rule) | {} | {other} | - | {} | {} | {} | {} | - |", refs.split(", ").filter_map(|x| x.rsplit_once(' ')).filter_map(|(_, v)| v.parse::<u64>().ok()).sum::<u64>(), c("net_detail_rs"), c("trades"), c("wins"), get("contract_open"), get("slots_full"), get("margin_cash_short"), c("peak_margin_rs"), c("max_dd_rs"), c("worst_trade_rs"), c("worst_day_rs"));
}

/// The v1-vs-v2 sections (side by side, expiry facts, margin, the 2-second index comparison). Returns the markdown and the numbers the
/// pick table needs (R1 audit per v2 pick).
#[allow(clippy::too_many_arguments)]
fn v2_top(log: &mut String, run_id: &str, rdir: &Path, cubes: &[Cube], tapes: &[SecTape], var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, segs: &[SegRun], po: &[PickOut], ctx: &Ctx<'_>) -> Result<String, String> {
    let mut md = String::new();
    let heads = ["-A1", "-X1", "-S1"];
    let lvs: Vec<Levels> = segs.iter().map(|s| s.lv.clone()).collect();
    let v1p: Vec<v2::V1Pick> = match ctx.cmp_v1 {
        Some(d) => v2::read_picks(d)?,
        None => Vec::new(),
    };
    let mut v1cfg: Vec<(CmpCfg, v2::V1Pick)> = Vec::new();
    for p in &v1p {
        if !heads.iter().any(|h| p.pick.ends_with(h)) {
            continue;
        }
        match v2::parse_settings(&p.settings, &lvs, &|seg, id, lv| cfg_words(seg, id, lv)) {
            Some((seg, cfg, x, ni)) => v1cfg.push(((seg, p.pick.clone(), cfg, x, ni), p.clone())),
            None => lg!(log, "v1 pick {}: its settings do not print back with this run's levels: {}", p.pick, p.settings),
        }
    }
    let mut v2cfg: Vec<CmpCfg> = Vec::new();
    for p in po.iter().filter(|p| heads.iter().any(|h| p.pid.ends_with(h))) {
        let Some(sr) = segs.get(p.seg) else { continue };
        v2cfg.push((p.seg, p.pid.clone(), rep_of(sr, &p.s, p.s.spike), usize::from(p.s.x), usize::from(p.s.n)));
    }
    // v1-rule event lists of exactly these configs (the full run's numbering, so the lists equal a v1 run's)
    let mut v1lists: Vec<HashMap<usize, Vec<(u16, u32, u32)>>> = vec![HashMap::new(); 2];
    for seg in 0..2_usize {
        let Some(sr) = segs.get(seg) else { continue };
        if sr.days.is_empty() {
            continue;
        }
        let mut keep = vec![false; NCFG];
        for c in v1cfg.iter().map(|x| &x.0).chain(v2cfg.iter()).filter(|c| c.0 == seg) {
            if let Some(k) = keep.get_mut(c.2) {
                *k = true;
            }
        }
        if !keep.iter().any(|&k| k) {
            continue;
        }
        let t0 = Instant::now();
        let devs: Vec<DayEvents> = std::thread::scope(|s| {
            let hs: Vec<_> = sr.days.iter().map(|&di| {
                let c = cubes.get(di);
                let (lv, keep) = (&sr.lv, &keep);
                s.spawn(move || c.map(|c| screen::generate(c, u16::try_from(di).unwrap_or(0), u8::try_from(seg).unwrap_or(0), lv, false, false, Some(keep))).unwrap_or_default())
            }).collect();
            hs.into_iter().map(|h| h.join().unwrap_or_default()).collect()
        });
        for (id, k) in keep.iter().enumerate() {
            if !*k {
                continue;
            }
            let mut v = Vec::new();
            for de in &devs {
                for e in de.lists.get(id).map_or(&[][..], Vec::as_slice) {
                    if let Some(u) = de.uev.get(e.ue as usize) {
                        v.push((u.di, u.ci, u.t));
                    }
                }
            }
            if let Some(m) = v1lists.get_mut(seg) {
                m.insert(id, v);
            }
        }
        lg!(log, "{}: v1-rule lists of {} compared configs generated in {:.1} s", SEG_NAME.get(seg).copied().unwrap_or("?"), keep.iter().filter(|&&k| k).count(), t0.elapsed().as_secs_f64());
    }
    let eval_v2 = |c: &CmpCfg, label: &str| -> Option<v2::Eval> {
        let sr = segs.get(c.0)?;
        let list = sr.lists.get(c.2)?;
        let (dec, s) = detail_list(&sr.evs, list, &sr.store, &sr.astore, c.3, c.4, u16::try_from(sr.days.len()).unwrap_or(0), true);
        Some(v2::summarize(&sr.evs, &sr.store, &dec, s, c.3, c.4, "v2", block_of(u8::try_from(c.0).unwrap_or(0)), label))
    };
    let eval_v1 = |c: &CmpCfg, blk: i64, label: &str| -> Result<Option<v2::Eval>, String> {
        let Some(sr) = segs.get(c.0) else { return Ok(None) };
        let Some(entries) = v1lists.get(c.0).and_then(|m| m.get(&c.2)) else { return Ok(None) };
        v2::eval_entries(entries, c.3, c.4, u16::try_from(sr.days.len()).unwrap_or(0), false, blk, cubes, tapes, var, dial, label).map(Some)
    };
    let entries_v2 = |c: &CmpCfg| -> Vec<(u16, u32, u32)> { segs.get(c.0).and_then(|sr| sr.lists.get(c.2).map(|l| l.iter().filter_map(|&e| sr.evs.get(e as usize).map(|x| (x.di, x.ci, x.t))).collect())).unwrap_or_default() };
    // ---- section: what changed
    let _ = writeln!(md, "## What v2 changes (operator 14 Nov 2026 23:1x IST ruling, recorded as the s25b ruling in SNIPER-REQUIREMENTS.md)");
    let _ = writeln!(md);
    let _ = writeln!(md, "- **R1 one position per underlying + side + expiry.** At most ONE open position per (underlying, CE/PE, expiry) at any instant. A CE and a PE of the same underlying may be open together. Every other event of an already-held key is refused and counted: `same_side_dup` (another strike) or `contract_open` (the same contract again, v1's rule kept). A position is open from its decision T to its last fill.");
    let _ = writeln!(md, "- **R2 rank lists.** A rank-top-K list keeps only the best-ranked strike per (underlying, side, expiry) at each candle end; the next distinct key fills the K. An index list holds ONE underlying (NIFTY's weekly or BANKNIFTY's monthly expiry), so a rank-top-3 or top-5 index list now gives at most 2 events per candle end (its best CE and its best PE): rank top 3 and top 5 become the same list there. Gain-threshold lists keep every member; R1 refuses the extra strikes in the account.");
    let _ = writeln!(md, "- **R3 expiry.** The contract and its expiry come from the same day's instrument snapshot. Stock options never trade on their own expiry day: that day the stock list is the underlying's next expiry, and with no next-expiry data in Tickvault that day the day's stock events are refused and counted (`stock_expiry_no_next`). Index options keep expiry-day trading (signed E1-E4); every index trade below says whether it was on its expiry day or used tomorrow's expiry.");
    let _ = writeln!(md, "- **R4 margin cap.** The margin in use at any instant (every open position's entry estimate, blocked from T to its last fill) never passes Rs 20,00,000 (SNIPER s26 addendum 2). A trade that would pass it is refused and counted (`margin_cap_20L`). v1's cash rule stays after it (lost money never reused: `margin_cash_short`).");
    let _ = writeln!(md, "- **Intraday only.** Every trade closes by the 15:10 square-off (no own order from 15:08; the square-off fills to 15:40 under the block law). Positional holding is NOT tested in this diagnostic.");
    let _ = writeln!(md, "- **Report fixes from the checker:** F1 (the TradingView 'average trade' is now printed in rupees, not paise) and F2 (the 2-second index figure is now each pick's OWN config re-run at 2-second blocks; v1's 'Rs 2,08,837.44' was the 2-second run's own re-picked best account, a different config).");
    let _ = writeln!(md);
    // ---- section: side by side
    let _ = writeln!(md, "## v1 vs v2 side by side, every headline pick (LABELLED DIAGNOSTIC + HINDSIGHT, proof step 1)");
    let _ = writeln!(md);
    let _ = writeln!(md, "Label: underlying = each traded option's own underlying (index: NIFTY, BANKNIFTY; stock: the F&O stocks Tickvault records); signal source = our Top Volume screener on Tickvault 1-minute candles (one segment, one expiry per list); traded = the event option SOLD naked, intraday; size = slot Rs 20 L / N, largest whole lots whose margin estimate fits; period = the 8 staged Tickvault days 2026-11-02..2026-11-11 (stock options 7: none on Muhurat 11-08); run `{run_id}` (v2) beside the v1 run `{}`; proof step 1 (in-sample replay; dials chosen on the scored days); Walk-forward test NOT RUN; nothing goes live.", ctx.cmp_v1.and_then(|p| p.file_name()).map_or("?".to_owned(), |x| x.to_string_lossy().into_owned()));
    let _ = writeln!(md);
    let _ = writeln!(md, "**Table 1: v1's six headline configs (picked under v1 rules), each re-run under v2 rules.** 'v1 rules (replayed here)' re-runs the same config with this binary's v1 switch; it must equal the v1 run's printed row (checked below). 'same-side pairs open together' audits the booked trades: pairs of positions of one (underlying, side, expiry) open at the same instant.");
    let _ = writeln!(md);
    let _ = writeln!(md, "{CMP_HEAD}");
    let mut checks: Vec<String> = Vec::new();
    let mut evals: Vec<(String, String, v2::Eval)> = Vec::new();
    for (c, p) in &v1cfg {
        v1_printed(&mut md, &c.1, p);
        let r1 = eval_v1(c, block_of(u8::try_from(c.0).unwrap_or(0)), "v1 replay")?;
        if let Some(e) = &r1 {
            cmp_row(&mut md, &c.1, "v1 rules (replayed here)", e, Some(v2::r1_overlaps(&e.booked)));
            let pc = |k: &str| p.cols.get(k).cloned().unwrap_or_default();
            let same = pc("net_detail_rs") == fmt_rs(e.s.net) && pc("trades") == e.s.trades.to_string() && pc("wins") == e.s.wins.to_string() && pc("max_dd_rs") == fmt_rs(e.s.mdd) && pc("worst_trade_rs") == fmt_rs(e.s.worst) && pc("peak_margin_rs") == fmt_rs(e.pk_rows);
            let refs = pc("refusals");
            let mut rsame = true;
            for (k, v) in &e.why {
                if !refs.split(", ").any(|x| x == format!("{k} {v}")) {
                    rsame = false;
                }
            }
            checks.push(format!("{} {}: net, trades, wins, max DD, worst trade, peak margin {}; every verdict count {}", c.1, if same && rsame { "EQUAL" } else { "DIFFERENT" }, if same { "equal" } else { "DIFFERENT" }, if rsame { "equal" } else { "DIFFERENT" }));
            evals.push((format!("v1 {}", c.1), "v1 config, v1 rules (replayed)".to_owned(), e.clone()));
        }
        if let Some(e) = eval_v2(c, "v1 config, v2 rules") {
            cmp_row(&mut md, &c.1, "**v2 rules (same config)**", &e, Some(v2::r1_overlaps(&e.booked)));
            evals.push((format!("v1 {}", c.1), "v1 config, v2 rules".to_owned(), e));
        }
    }
    let _ = writeln!(md);
    let _ = writeln!(md, "Replay check (this binary's v1 switch vs the v1 run's printed picks): {}.", if checks.is_empty() { if ctx.cmp_v1.is_some() { "NOT DONE: the v1 picks' settings did not print back with this run's levels (another day set?)".to_owned() } else { "no v1 run given".to_owned() } } else { checks.join("; ") });
    let _ = writeln!(md);
    let _ = writeln!(md, "**Table 2: v2's own headline picks (the whole dial grid re-searched under v2 rules), each also run under v1 rules.**");
    let _ = writeln!(md);
    let _ = writeln!(md, "{CMP_HEAD}");
    for c in &v2cfg {
        if let Some(e) = eval_v2(c, "v2 pick") {
            cmp_row(&mut md, &c.1, "**v2 rules (this run's pick)**", &e, Some(v2::r1_overlaps(&e.booked)));
            evals.push((format!("v2 {}", c.1), "v2 pick, v2 rules".to_owned(), e));
        }
        if let Some(e) = eval_v1(c, block_of(u8::try_from(c.0).unwrap_or(0)), "v2 config, v1 rules")? {
            cmp_row(&mut md, &c.1, "v1 rules (same config)", &e, Some(v2::r1_overlaps(&e.booked)));
            evals.push((format!("v2 {}", c.1), "v2 config, v1 rules".to_owned(), e));
        }
    }
    let _ = writeln!(md);
    // every booked trade of every compared row, and the worst trade of each row
    let mut ct = String::from("config\tsource\trules\tday\tdecision_T\tcontract\tsecurity_id\tlast_fill_known\tmargin_blocked_rs\tnet_rs\n");
    let _ = writeln!(md, "The worst trade of each compared row (every booked trade of every row in `runs/{run_id}/compare-trades.tsv`):");
    let _ = writeln!(md);
    let _ = writeln!(md, "| config | source | worst trade: day | T | contract | margin blocked Rs | net Rs |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|");
    for (cfgn, src, e) in &evals {
        let mut worst: Option<&v2::Bk> = None;
        for b in &e.booked {
            let (day, sym, sid) = cubes.get(usize::from(b.0)).map_or(("?".to_owned(), "?".to_owned(), 0), |c| (fmt_day(c.day), c.cons.get(b.1 as usize).map_or("?".to_owned(), |k| k.symbol.clone()), c.cons.get(b.1 as usize).map_or(0, |k| k.sid)));
            let _ = writeln!(ct, "{cfgn}\t{src}\t{}\t{day}\t{}\t{sym}\t{sid}\t{}\t{}\t{}", e.rules, fmt_sod(i64::from(b.2)), fmt_sod(b.3), fmt_rs(b.4), fmt_rs(b.6));
            if worst.is_none_or(|w| b.6 < w.6) {
                worst = Some(b);
            }
        }
        if let Some(w) = worst {
            let (day, sym) = cubes.get(usize::from(w.0)).map_or(("?".to_owned(), "?".to_owned()), |c| (fmt_day(c.day), c.cons.get(w.1 as usize).map_or("?".to_owned(), |k| k.symbol.clone())));
            let _ = writeln!(md, "| {cfgn} | {src} | {day} | {} | {sym} | {} | {} |", fmt_sod(i64::from(w.2)), fmt_rs(w.4), fmt_rs(w.6));
        }
    }
    let _ = writeln!(md);
    std::fs::write(rdir.join("compare-trades.tsv"), &ct).map_err(|e| e.to_string())?;
    for c in &v2cfg {
        if let Some(sr) = segs.get(c.0) {
            let s = Sum { list: 0, x: u16::try_from(c.3).unwrap_or(0), n: u8::try_from(c.4).unwrap_or(0), ..Sum::default() };
            let _ = writeln!(md, "- {}: {} || {} || N {} || block {} s", c.1, cfg_words(c.0, c.2, &sr.lv), exit_words(c.3), n_of(c.4), block_of(u8::try_from(c.0).unwrap_or(0)));
            let _ = s;
        }
    }
    for (c, _) in &v1cfg {
        if let Some(sr) = segs.get(c.0) {
            let _ = writeln!(md, "- v1 {}: {} || {} || N {}", c.1, cfg_words(c.0, c.2, &sr.lv), exit_words(c.3), n_of(c.4));
        }
    }
    let _ = writeln!(md);
    // ---- the 2-second index comparison (F2)
    let _ = writeln!(md, "## Index options at 2-second blocks (checker F2, fixed): each pick's OWN config re-run with 2-second fill blocks");
    let _ = writeln!(md);
    let _ = writeln!(md, "The index headline uses 1-second blocks (REQ 73b/73c, SNIPER s25 item 4); 2-second blocks are the harsher fill and are printed beside. Same config, same exits, same N; only the block length changes.");
    let _ = writeln!(md);
    for c in v2cfg.iter().filter(|c| c.0 == 0) {
        let base = eval_v2(c, "1 s");
        let e2 = v2::eval_entries(&entries_v2(c), c.3, c.4, u16::try_from(segs.first().map_or(0, |s| s.days.len())).unwrap_or(0), true, 2, cubes, tapes, var, dial, "2 s")?;
        let _ = writeln!(md, "- v2 {}: at 1-second blocks Rs {} on {} trades; **at 2-second blocks Rs {} on {} trades** (max DD Rs {}, worst trade Rs {}).", c.1, base.as_ref().map_or("-".to_owned(), |b| fmt_rs(b.s.net)), base.as_ref().map_or(0, |b| b.s.trades), fmt_rs(e2.s.net), e2.s.trades, fmt_rs(e2.s.mdd), fmt_rs(e2.s.worst));
    }
    for (c, _) in v1cfg.iter().filter(|(c, _)| c.0 == 0 && c.1.ends_with("-A1")) {
        if let Some(e2) = eval_v1(c, 2, "v1 idx-A1 at 2 s")? {
            let _ = writeln!(md, "- v1 idx-A1's own config under v1 rules at 2-second blocks (replayed here): Rs {} on {} trades (the checker's own code: Rs 2,08,675.53 on 164 trades). The v1 report's 'Rs 2,08,837.44 on 155 trades' was the 2-second run's own re-picked best account (TF 10m, min vol L3, top 20, min prem Rs 20, V3), not idx-A1's config.", fmt_rs(e2.s.net), e2.s.trades);
        }
    }
    let _ = writeln!(md);
    // ---- expiry facts
    let _ = writeln!(md, "## Expiry (a): from each day's own instrument snapshot, checked against NSE's trading-holiday list");
    let _ = writeln!(md);
    let _ = writeln!(md, "Calendar: NSE's own holiday list (`{}`, {} rows, read only; 2026 rows include 2026-11-08 Muhurat (a Sunday special session), 2026-11-10 Diwali-Balipratipada and 2026-11-24 Guru Nanak Jayanti). NSE expiries are Tuesdays (from 2025-09-01); a Tuesday holiday moves the expiry to the trading day before. 'sessions' = regular trading days after the trade day up to the expiry (0 = the expiry day itself, 1 = tomorrow's expiry; the Muhurat Sunday is not a regular session).", ctx.cal.source, ctx.cal.hol.len());
    let _ = writeln!(md);
    let _ = writeln!(md, "| day | segment | expiry in the snapshot | underlyings in that list | the expiry checked | sessions from the day | rule |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|");
    for c in cubes {
        for seg in 0..2_u8 {
            for (ex, l) in screen::lists(c, seg, false, ctx.rules.v2) {
                let mut un: Vec<&str> = l.iter().filter_map(|&ci| c.cons.get(ci as usize).and_then(|k| c.unds.get(usize::from(k.und))).map(|u| u.name.as_str())).collect();
                un.sort_unstable();
                un.dedup();
                let e = crate::cal::day_of_ymd(ex);
                let s = ctx.cal.sessions(c.day, e);
                let rule = if seg == 0 { if s == 0 { "index EXPIRY DAY: allowed (E1-E4)".to_owned() } else if s == 1 { "index: tomorrow's expiry".to_owned() } else { format!("index: {s} sessions ahead") } } else if ex == c.ymd { "stock EXPIRY DAY: refused (no next-expiry data)".to_owned() } else { format!("stock: {s} sessions ahead (never on its expiry day)") };
                let names = if un.len() > 6 { format!("{} underlyings ({}, ...)", un.len(), un.iter().take(4).copied().collect::<Vec<_>>().join(", ")) } else { un.join(", ") };
                let _ = writeln!(md, "| {} {} | {} | {} | {names} | {} | {s} | {rule} |", fmt_day(c.day), crate::rules::wd_name(c.day), SEG_SHORT.get(usize::from(seg)).copied().unwrap_or("?"), ex, ctx.cal.expiry_words(e));
            }
        }
    }
    let _ = writeln!(md);
    let roll: (u32, u32) = segs.get(1).map_or((0, 0), |sr| sr.dev.iter().fold((0, 0), |a, de| (a.0 + de.roll.0, a.1 + de.roll.1)));
    let sx = segs.get(1).and_then(|sr| sr.refusals.get("stock_expiry_no_next").copied()).unwrap_or(0);
    let _ = writeln!(md, "- **Stock expiry days in the 8 days: none.** Every stock contract in the snapshots expires 2026-11-23 (Monday), the holiday shift of Tuesday 2026-11-24, after the last staged day (2026-11-11). Underlyings on their own expiry day over the staged days: {} rolled to the next expiry, {} with no next-expiry data; stock events refused `stock_expiry_no_next`: {sx}. The rule is coded and unit-tested (R3 tests and a mutant) but this data never reaches it.", roll.0, roll.1);
    let _ = writeln!(md, "- **Index expiries:** NIFTY weekly 2026-11-03 (Tuesday), 2026-11-09 (Monday: Tuesday 2026-11-10 was the Diwali-Balipratipada holiday, so the expiry moved to the trading day before) and 2026-11-17 (Tuesday); BANKNIFTY monthly 2026-11-23 (Monday: last Tuesday 2026-11-24 is a holiday). Every snapshot expiry above is a regular trading day and the trading day at or before its Tuesday.");
    let _ = writeln!(md);
    let _ = writeln!(md, "Index trades of the v2 headline picks by expiry class:");
    let _ = writeln!(md);
    let _ = writeln!(md, "| pick | trades | on the expiry day (E1-E4) | tomorrow's expiry | 2+ sessions ahead | expiry-day trades (day T contract) | tomorrow's-expiry trades (day T contract) |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|");
    for p in po.iter().filter(|p| p.seg == 0 && heads.iter().any(|h| p.pid.ends_with(h))) {
        let d0: Vec<String> = p.rows.iter().filter(|r| r.sessions == 0).map(|r| format!("{} {} {}", cubes.get(usize::from(r.di)).map_or("?".to_owned(), |c| fmt_day(c.day)), fmt_sod(i64::from(r.t)), r.contract)).collect();
        let d1: Vec<String> = p.rows.iter().filter(|r| r.sessions == 1).map(|r| format!("{} {} {}", cubes.get(usize::from(r.di)).map_or("?".to_owned(), |c| fmt_day(c.day)), fmt_sod(i64::from(r.t)), r.contract)).collect();
        let n2 = p.rows.iter().filter(|r| r.sessions >= 2).count();
        let cut = |v: &[String]| if v.len() > 12 { format!("{} (+{} more; all in trades-picks.tsv)", v.iter().take(12).cloned().collect::<Vec<_>>().join("; "), v.len() - 12) } else if v.is_empty() { "none".to_owned() } else { v.join("; ") };
        let _ = writeln!(md, "| {} | {} | {} | {} | {n2} | {} | {} |", p.pid, p.rows.len(), d0.len(), d1.len(), cut(&d0), cut(&d1));
    }
    let _ = writeln!(md);
    let _ = writeln!(md, "Stock trades of the v2 headline picks: {}. None is on its own expiry day (R3).", po.iter().filter(|p| p.seg == 1 && heads.iter().any(|h| p.pid.ends_with(h))).map(|p| { let mut m: BTreeMap<i64, usize> = BTreeMap::new(); for r in &p.rows { *m.entry(r.sessions).or_insert(0) += 1; } format!("{} {} trades ({})", p.pid, p.rows.len(), m.iter().map(|(s, n)| format!("{n} at {s} sessions to expiry")).collect::<Vec<_>>().join(", ")) }).collect::<Vec<_>>().join("; "));
    let _ = writeln!(md);
    // ---- margin
    let _ = writeln!(md, "## Margin (b): per trade and at every instant, hard cap Rs 20,00,000");
    let _ = writeln!(md);
    let _ = writeln!(md, "Per trade (every row of trades-picks.tsv): `underlying_S_rs`, `margin_unit_rs`, `margin_lot_rs`, `lots`, `margin_blocked_rs` and, new in v2, `margin_in_use_after_rs` (every open position's margin right after this booking, this one included) and `open_positions_after`. Index margins are an ASSUMED formula (no SPAN source on disk): per unit max(ceil(S x 6%) - OTM, ceil(S x 3%)) + ceil(S x 2%), + 2% on the option's expiry day. Stock margins are the s25 sell-diag proxy with NSE's 2026-10-30 C_VAR1 (the latest on disk).");
    let _ = writeln!(md);
    let _ = writeln!(md, "Why the cap never binds here: each position is sized to fit its slot (Rs 20 L / N), at most N are open, so the margin in use is at most Rs 20 L by construction; and v1's cash rule (free = Rs 20 L + min(0, realized) - margin in use) already refused anything past Rs 20 L. v1 never passed Rs 20 L either; v2 makes the check explicit and counts it first. The cap uses each position's ENTRY estimate. Beside it, the 'spot-marked' peak re-estimates every open position's margin each minute with the live underlying (units sold held from T to the last fill): a broker's intraday margin moves this way, so it shows how close a real account would come.");
    let _ = writeln!(md);
    let _ = writeln!(md, "| pick | rules | trades | peak margin in use Rs (account, entry estimates) | the same from trade intervals | refused margin_cap_20L | refused margin_cash_short | spot-marked peak Rs (beside) | minute ends above Rs 20 L (spot-marked) | spot-marked peak at |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|");
    for p in po.iter().filter(|p| heads.iter().any(|h| p.pid.ends_with(h))) {
        let rm: Vec<(u16, u32, u32, i64, u64)> = p.rows.iter().map(|r| (r.di, r.ci, r.t, if r.known > 0 { sod_of(r.known) } else { crate::cal::END_S }, r.units)).collect();
        let (pk, over, at) = v2::remark_peak(&rm, cubes, var);
        let _ = writeln!(md, "| {} | v2 | {} | {} | {} | {} | {} | {} | {over} | {at} |", p.pid, p.rows.len(), fmt_rs(p.s2.pk_margin), fmt_rs(peak_margin(&p.rows)), p.why.get("margin_cap_20L").copied().unwrap_or(0), p.why.get("margin_cash_short").copied().unwrap_or(0), fmt_rs(pk));
    }
    let _ = writeln!(md);
    Ok(md)
}

/// The per-trade fill-block EVIDENCE of the six headline picks: one file per pick under `runs/<run>/evidence/`, and the first trade of
/// each inline. Returns (inline markdown, trades, trades whose evidence walk equals the engine's trade).
#[allow(clippy::too_many_arguments)]
fn evidence_files(log: &mut String, rdir: &Path, cubes: &[Cube], tapes: &[SecTape], dial: &common_fill::Dial, segs: &[SegRun], po: &[PickOut], ctx: &Ctx<'_>) -> Result<(String, usize, usize), String> {
    let heads = ["-A1", "-X1", "-S1"];
    let picks: Vec<&PickOut> = po.iter().filter(|p| heads.iter().any(|h| p.pid.ends_with(h))).collect();
    // raw 1-minute rows of every evidence trade's contract, one pass per day, days in parallel (at most 8 at once)
    let mut need: BTreeMap<u16, std::collections::HashSet<u32>> = BTreeMap::new();
    for p in &picks {
        for r in &p.rows {
            if let Some(k) = cubes.get(usize::from(r.di)).and_then(|c| c.cons.get(r.ci as usize)) {
                need.entry(r.di).or_default().insert(k.sid);
            }
        }
    }
    let t0 = Instant::now();
    let raw: HashMap<u16, HashMap<u32, BTreeMap<usize, [i64; 5]>>> = match ctx.stage {
        Some(st) => {
            let jobs: Vec<(u16, std::collections::HashSet<u32>)> = need.into_iter().collect();
            let res: Vec<(u16, Result<HashMap<u32, BTreeMap<usize, [i64; 5]>>, String>)> = std::thread::scope(|s| {
                let hs: Vec<_> = jobs.iter().map(|(di, sids)| {
                    let d = cubes.get(usize::from(*di)).map_or(String::new(), |c| fmt_day(c.day));
                    s.spawn(move || (*di, v2::raw_1m(st, &d, sids)))
                }).collect();
                hs.into_iter().filter_map(|h| h.join().ok()).collect()
            });
            let mut m = HashMap::new();
            for (di, r) in res {
                m.insert(di, r?);
            }
            m
        }
        None => HashMap::new(),
    };
    lg!(log, "evidence: raw 1-minute rows read for {} days in {:.1} s", raw.len(), t0.elapsed().as_secs_f64());
    let edir = rdir.join("evidence");
    std::fs::create_dir_all(&edir).map_err(|e| e.to_string())?;
    let mut inline = String::new();
    let (mut n, mut ok) = (0_usize, 0_usize);
    for p in &picks {
        let Some(sr) = segs.get(p.seg) else { continue };
        let rep = sr.ulists.get(p.s.list as usize).copied().unwrap_or(0) as usize;
        let (tf, _, _, _, _, thr) = cfg_parts(rep);
        let (k, j, m, l) = x_parts(usize::from(p.s.x));
        let blk = block_of(u8::try_from(p.seg).unwrap_or(0));
        let mut f = String::new();
        let _ = writeln!(f, "# Evidence, {} ({}): every trade's signal candle and fill blocks — LABELLED DIAGNOSTIC + HINDSIGHT, rules {}", p.pid, SEG_NAME.get(p.seg).copied().unwrap_or("?"), ctx.rules.name());
        let _ = writeln!(f);
        let _ = writeln!(f, "Settings: {}. Intraday only. 1-second rows = the cube's seconds tape (the fill input: HIGH / LOW / |volume| of each traded second, read from the staged `candles_1s`); 1-minute rows = the raw staged `candles_1m`. A SALE gets each block's lowest LOW, a BUY pays each block's highest HIGH, never the decision price; the share is the dial's part of each block's volume in whole lots; the rest moves to the next block; blocks with no traded second are jumped (not listed). Margin in use = every open position's entry estimate right after the booking (cap Rs 20,00,000).", settings_words(p.seg, sr, &p.s));
        let _ = writeln!(f);
        for (i, r) in p.rows.iter().enumerate() {
            let Some(c) = cubes.get(usize::from(r.di)) else { continue };
            let Some(tp) = tapes.get(usize::from(r.di)) else { continue };
            let sid = c.cons.get(r.ci as usize).map_or(0, |k| k.sid);
            // v2.1: under a capacity reading the evidence is numbered by trades-picks.tsv's trade_no (fill-gaps.tsv's key); the v2 replay keeps v2's
            let e = v2::EvIn { pick: &p.pid, no: if ctx.reading == common_fill::CapMode::Row { i + 1 } else { r.no }, di: r.di, ci: r.ci, t: r.t, tf, thr, gain_bp: r.gain_bp, wlots: r.wlots, lots: r.lots, l, k, j, m, x: r.x, kind: r.exit_kind, margin_unit: r.margin_unit, margin_lot: r.margin_lot, s: r.s, margin_in_use: r.margin_in_use, nopen: r.nopen, trip: &r.trip, cls: r.cls.clone() };
            let (txt, good) = v2::evidence_md(&e, c, tp.of(r.ci as usize), raw.get(&r.di).and_then(|m| m.get(&sid)), blk, dial)?;
            n += 1;
            ok += usize::from(good);
            f.push_str(&txt);
            if i == 0 {
                inline.push_str(&txt);
            }
        }
        std::fs::write(edir.join(format!("{}.md", p.pid)), &f).map_err(|e| e.to_string())?;
    }
    Ok((inline, n, ok))
}

// ======================================================================================================================================
// v2.1 (checker F-A of check-spikefade-diag-v2.md, 15 Nov 2026): the sparse-tape fill capacity. The v2-vs-v21 side by side, the three
// readings beside every pick, the sparse-tape counts (FIX 3), the refusals by cause, the 2-second index comparison, margin and expiry.
// ======================================================================================================================================

/// The sparse-tape facts of one evaluation's booked trades (FIX 3), re-walked under the evaluation's own reading.
#[derive(Clone, Copy, Debug, Default)]
struct GapStat {
    trades: usize,
    blocks: u64,
    gap_blocks: u64,
    units: u64,
    gap_units: u64,
    rec_vol: u64,
    usable_vol: u64,
    first_before_t: u64,
    median_rows: u64,
    contract_days: usize,
}

/// One booked trade of an evaluation, re-walked: (day index, contract, T, lots, exit anchor or None).
type Lite = (u16, u32, u32, u16, Option<i64>);

/// The booked trades of a detailed account run with their exit anchors (from the anchor table the run priced with).
fn booked_lite(evs: &[SEv], astore: &[[u16; NRULE]], dec: &[crate::acct::Dec], x: usize, ni: usize) -> Vec<Lite> {
    let (_, _, _, l) = x_parts(x);
    let mut v = Vec::new();
    for d in dec.iter().filter(|d| d.code == "booked") {
        let Some(e) = evs.get(d.eid as usize) else { continue };
        let q = usize::from(e.tab.q_of.get(ni).copied().unwrap_or(255));
        let anc = astore.get(e.tab.aidx(l, q)).copied().unwrap_or([trade::NEVER; NRULE]);
        v.push((e.di, e.ci, e.t, d.lots, trade::cand_anchor(&anc, d.cand)));
    }
    v
}

/// FIX 3: the sparse-tape counts of a list of booked trades under `reading` (blocks of `blk` seconds, share level of exit id `x`).
#[allow(clippy::too_many_arguments)]
fn gap_stat(trades: &[Lite], x: usize, blk: i64, reading: common_fill::CapMode, cubes: &[Cube], tapes: &[SecTape], dial: &common_fill::Dial) -> Result<GapStat, String> {
    let (_, _, _, l) = x_parts(x);
    let mut g = GapStat { trades: trades.len(), ..GapStat::default() };
    let mut rows: BTreeMap<(u16, u32), u64> = BTreeMap::new();
    let mut per_trade: Vec<u64> = Vec::with_capacity(trades.len());
    for &(di, ci, t, lots, xx) in trades {
        let (Some(c), Some(tp)) = (cubes.get(usize::from(di)), tapes.get(usize::from(di))) else { continue };
        let Some(kc) = c.cons.get(ci as usize) else { continue };
        let lot = u64::from(kc.lot);
        let dt = DayTape::with_reading(c.day, tp.of(ci as usize), blk, reading)?;
        let tape = dt.tape()?;
        // the checker's definition: the traded contract-day's 1-second prints, one value PER TRADE (repeats kept)
        per_trade.push(u64::try_from(dt.rows().iter().filter(|r| r.vol > 0).count()).unwrap_or(0));
        rows.entry((di, ci)).or_insert(0);
        let Some(ev) = trade::evidence(&tape, c.day, t, lot, u64::from(lots) * lot, l, xx, dial)? else { continue };
        let (nb, ng, ua, ug, rv, uv, fb) = trade::gap_facts(&dt, &ev, i64::from(t), blk);
        g.blocks += u64::from(nb);
        g.gap_blocks += u64::from(ng);
        g.units += ua;
        g.gap_units += ug;
        g.rec_vol += rv;
        g.usable_vol += uv;
        g.first_before_t += u64::from(fb);
    }
    g.contract_days = rows.len();
    g.median_rows = if per_trade.is_empty() { 0 } else { per_trade.sort_unstable(); per_trade.get(per_trade.len() / 2).copied().unwrap_or(0) };
    Ok(g)
}

fn pct1(a: u64, b: u64) -> String {
    if b == 0 { "-".to_owned() } else { format!("{:.1}%", 100.0 * a as f64 / b as f64) }
}

/// "after_cut_1508 95, contract_open 110, ..." (every non-booked code with a count, alphabetical: the checker's own format).
fn why_alpha(w: &BTreeMap<&'static str, u64>) -> String {
    let v: Vec<String> = w.iter().filter(|(k, n)| **k != "booked" && **n > 0).map(|(k, n)| format!("{k} {n}")).collect();
    if v.is_empty() { "none".to_owned() } else { v.join(", ") }
}

/// The checker's account summaries (read only): (config label "v2 idx-A1", rules) -> columns.
fn checker_rows(dir: Option<&Path>, file: &str) -> HashMap<(String, String), Vec<String>> {
    let mut m = HashMap::new();
    let Some(d) = dir else { return m };
    let Ok(t) = std::fs::read_to_string(d.join(file)) else { return m };
    for l in t.lines().skip(1) {
        let f: Vec<String> = l.split('\t').map(str::to_owned).collect();
        if let (Some(a), Some(b)) = (f.first(), f.get(1)) {
            m.insert((a.clone(), b.clone()), f.clone());
        }
    }
    m
}

/// One evaluation row of the side-by-side.
struct EvRow {
    who: String,
    reading: common_fill::CapMode,
    ev: v2::Eval,
    gs: GapStat,
}

const V21_HEAD: &str = "| pick | fill reading / source | events | net Rs | trades | wins | max DD Rs | worst trade Rs | worst day Rs | peak margin Rs | same-side dup | contract open | slots full | not filled | other refusals | fill blocks | fed by a silence longer than the block | share of filled units from them | first sale block after a print before T | median 1-s prints per traded contract-day |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|";

fn v21_row(md: &mut String, pick: &str, r: &EvRow) {
    let w = &r.ev.why;
    let g = |k: &str| w.get(k).copied().unwrap_or(0);
    let other: u64 = w.iter().filter(|(k, _)| !["booked", "same_side_dup", "contract_open", "slots_full", "not_filled"].contains(k)).map(|(_, v)| *v).sum();
    let s = &r.ev.s;
    let _ = writeln!(md, "| {pick} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {other} | {} | {} ({}) | {} | {} of {} | {} |", r.who, r.ev.events, fmt_rs(s.net), s.trades, s.wins, fmt_rs(s.mdd), fmt_rs(s.worst), wd(s), fmt_rs(s.pk_margin), g("same_side_dup"), g("contract_open"), g("slots_full"), g("not_filled"), r.gs.blocks, r.gs.gap_blocks, pct1(r.gs.gap_blocks, r.gs.blocks), pct1(r.gs.gap_units, r.gs.units), r.gs.first_before_t, r.gs.trades, r.gs.median_rows);
}

/// The v2-vs-v21 sections. v2's six headline configs (read from the v2 run's picks.tsv) and v21's own six picks, each under the three fill
/// readings (v2.1 prorate = the main grid; anchor and v2 re-priced here), the checker's own figures beside, FIX 3, refusals by cause, the
/// 2-second index comparison, margin and expiry of the v21 picks.
#[allow(clippy::too_many_arguments)]
fn v21_top(log: &mut String, run_id: &str, rdir: &Path, cubes: &[Cube], tapes: &[SecTape], var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, segs: &[SegRun], po: &[PickOut], ctx: &Ctx<'_>) -> Result<String, String> {
    use common_fill::CapMode;
    let mut md = String::new();
    let heads = ["-A1", "-X1", "-S1"];
    let lvs: Vec<Levels> = segs.iter().map(|s| s.lv.clone()).collect();
    let v2name = ctx.cmp_v2.and_then(|p| p.file_name()).map_or("?".to_owned(), |x| x.to_string_lossy().into_owned());
    let v2p: Vec<v2::V1Pick> = match ctx.cmp_v2 {
        Some(d) => v2::read_picks(d)?,
        None => Vec::new(),
    };
    let mut v2cfg: Vec<(CmpCfg, v2::V1Pick)> = Vec::new();
    for p in &v2p {
        if !heads.iter().any(|h| p.pick.ends_with(h)) {
            continue;
        }
        match v2::parse_settings(&p.settings, &lvs, &|seg, id, lv| cfg_words(seg, id, lv)) {
            Some((seg, cfg, x, ni)) => v2cfg.push(((seg, p.pick.clone(), cfg, x, ni), p.clone())),
            None => lg!(log, "v2 pick {}: its settings do not print back with this run's levels: {}", p.pick, p.settings),
        }
    }
    let mut v21cfg: Vec<CmpCfg> = Vec::new();
    for p in po.iter().filter(|p| heads.iter().any(|h| p.pid.ends_with(h))) {
        let Some(sr) = segs.get(p.seg) else { continue };
        v21cfg.push((p.seg, p.pid.clone(), rep_of(sr, &p.s, p.s.spike), usize::from(p.s.x), usize::from(p.s.n)));
    }
    let blk_of = |seg: usize| block_of(u8::try_from(seg).unwrap_or(0));
    let nd = |seg: usize| u16::try_from(segs.get(seg).map_or(0, |s| s.days.len())).unwrap_or(0);
    let entries = |c: &CmpCfg| -> Vec<(u16, u32, u32)> { segs.get(c.0).and_then(|sr| sr.lists.get(c.2).map(|l| l.iter().filter_map(|&e| sr.evs.get(e as usize).map(|x| (x.di, x.ci, x.t))).collect())).unwrap_or_default() };
    // the main grid's own reading (v2.1 prorate): straight from the swept tables
    let eval_main = |c: &CmpCfg, label: &str| -> Result<Option<EvRow>, String> {
        let Some(sr) = segs.get(c.0) else { return Ok(None) };
        let Some(list) = sr.lists.get(c.2) else { return Ok(None) };
        let (dec, s) = detail_list(&sr.evs, list, &sr.store, &sr.astore, c.3, c.4, nd(c.0), true);
        let ev = v2::summarize(&sr.evs, &sr.store, &dec, s, c.3, c.4, "v2", blk_of(c.0), label);
        let gs = gap_stat(&booked_lite(&sr.evs, &sr.astore, &dec, c.3, c.4), c.3, blk_of(c.0), ctx.reading, cubes, tapes, dial)?;
        Ok(Some(EvRow { who: label.to_owned(), reading: ctx.reading, ev, gs }))
    };
    // another reading: every event of the config re-priced under it
    let eval_other = |c: &CmpCfg, reading: CapMode, blk: i64, label: &str| -> Result<EvRow, String> {
        let f = v2::eval_entries_full(&entries(c), c.3, c.4, nd(c.0), true, blk, reading, cubes, tapes, var, dial, label)?;
        let gs = gap_stat(&booked_lite(&f.evs, &f.astore, &f.dec, c.3, c.4), c.3, blk, reading, cubes, tapes, dial)?;
        Ok(EvRow { who: label.to_owned(), reading, ev: f.ev, gs })
    };
    let ck_pro = checker_rows(ctx.checker, "acct-summary-prorate.tsv");
    let ck_anc = checker_rows(ctx.checker, "acct-summary-prorate-anchor.tsv");
    let ck_row = checker_rows(ctx.checker, "acct-summary.tsv");
    let ck_cmp = |m: &HashMap<(String, String), Vec<String>>, key: &str, e: &v2::Eval| -> String {
        match m.get(&(key.to_owned(), "v2".to_owned())) {
            None => "-".to_owned(),
            Some(f) => {
                let c = |i: usize| f.get(i).cloned().unwrap_or_default();
                // the checker prints the worst day even when it is positive (no losing day); this tool keeps 0.00 then
                let ckwd = if c(8).starts_with('-') { c(8) } else { "0.00".to_owned() };
                let same = c(2) == fmt_rs(e.s.net) && c(3) == e.s.trades.to_string() && c(4) == e.s.wins.to_string() && c(5) == fmt_rs(e.s.mdd) && c(7) == fmt_rs(e.s.worst) && ckwd == fmt_rs(e.s.worst_day) && c(9) == fmt_rs(e.s.pk_margin) && c(10) == why_alpha(&e.why);
                format!("Rs {} on {} trades: {}", c(2), c(3), if same { "EQUAL (net, trades, wins, max DD, worst trade, worst day, peak margin, every refusal count)" } else { "DIFFERENT" })
            }
        }
    };
    let t0 = Instant::now();
    // ---- evaluate (v2's picks: printed + 3 readings; v21's picks: 3 readings), parallel over configs
    let jobs: Vec<(usize, CmpCfg)> = v2cfg.iter().map(|(c, _)| (0_usize, c.clone())).chain(v21cfg.iter().map(|c| (1_usize, c.clone()))).collect();
    // at most 4 configs at once (each re-pricing holds one contract-day tape at a time)
    let next = AtomicUsize::new(0);
    let res_all: Mutex<Vec<(usize, Result<(usize, CmpCfg, Vec<EvRow>), String>)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some((who, c)) = jobs.get(k) else { break };
                    let r = (|| -> Result<(usize, CmpCfg, Vec<EvRow>), String> {
                        let mut rows = Vec::new();
                        let blk = blk_of(c.0);
                        if *who == 0 {
                            rows.push(eval_other(c, CapMode::Row, blk, "v2 reading, replayed here (the replay switch)")?);
                            rows.push(eval_other(c, CapMode::Anchor, blk, "anchor reading (no volume from before the order is sent)")?);
                            if let Some(r) = eval_main(c, "**v2.1 prorate (THE v21 reading)**")? {
                                rows.push(r);
                            }
                        } else {
                            if let Some(r) = eval_main(c, "**v2.1 prorate (this run's pick)**")? {
                                rows.push(r);
                            }
                            rows.push(eval_other(c, CapMode::Anchor, blk, "anchor reading (same config)")?);
                            rows.push(eval_other(c, CapMode::Row, blk, "v2 reading (same config)")?);
                        }
                        Ok((*who, c.clone(), rows))
                    })();
                    if let Ok(mut g) = res_all.lock() {
                        g.push((k, r));
                    }
                }
            });
        }
    });
    let mut res_all = res_all.into_inner().unwrap_or_default();
    res_all.sort_by_key(|x| x.0);
    let mut done: Vec<(usize, CmpCfg, Vec<EvRow>)> = Vec::new();
    for (_, r) in res_all {
        done.push(r?);
    }
    lg!(log, "v21 side by side: {} configs x 3 readings evaluated in {:.1} s", done.len(), t0.elapsed().as_secs_f64());
    // ---- section: what v2.1 changes
    let _ = writeln!(md, "## What v2.1 changes (checker F-A, the one must-fix of `check-spikefade-diag-v2.md`, applied as the check writes it)");
    let _ = writeln!(md);
    let _ = writeln!(md, "- **The data fact.** Tickvault's 1-second option rows are SPARSE: a row is written only for a second with a print, and a row that follows a silence carries EVERY unit traded since the previous print (the median traded stock-option contract-day has well under a thousand rows for about 22,500 session seconds). v2 counted the whole of such a row as tradable inside its own second, sometimes volume that traded before the order was sent, inside the signal candle (REQ 56 forbids that).");
    let _ = writeln!(md, "- **The v2.1 rule (FA1, the checker's FIX 1).** Fill capacity counts only volume that can have traded at or after the order instant and inside the block: each row gives `usable = |volume| x n / g`, where g = the seconds of (previous print, this print] and n = those of them inside [max(block start, order send), the block's volume end). The usable volume of a block is the exact sum of its rows' fractions; the share level (10/25/50/75/100%) is applied to it and floored ONCE, in whole lots; the rest of the order waits for the next block. The day's first print is read as following 09:14:59 (its volume spread from the 09:15:00 open: the checker's stated reading). Prices are unchanged: a sale still gets the block's lowest LOW, a buy-back pays its highest HIGH (the block's recorded print(s), not every trade inside it: FIX 4).");
    let _ = writeln!(md, "- **The three readings (FIX 2), printed beside every pick:** `v2.1 prorate` (the rule above, THE v21 reading and headline); `anchor` (the checker's minimal REQ 56 reading: n = the silence's seconds at or after the order's send, the rest of the row counted in the block of its print); `v2` (each row's whole volume in its own second: the replay switch, `run --fill-reading v2`, which reproduces v2).");
    let _ = writeln!(md, "- **Where it lives.** `scratch/spikefade-v21` = `cp -p -R -n` of `scratch/spikefade-v2` (v2 untouched); the fill crate is `pinned/common-fill-v21-fa1` = crate v2.1 + change FA1 (`Tape::with_cap`, `CapMode`): OPT-IN, and without it every reading is crate v2.1's byte for byte (its 102 tests, the G0 vectors and the extreme-case matrix pass unchanged; 7 new FA1 tests incl. a randomized oracle on sparse days). The engine, screener, events, exits, R1-R4, charges and margin are v2's.");
    let _ = writeln!(md, "- **Not changed here (checker follow-ups, not part of F-A):** F-B (square-off prints after 15:30 still fill, to 15:40), F-C (no MTM-inclusive margin check), F-D (drawdown tie order), F-E..F-G wording. They are named again in the proof notes.");
    let _ = writeln!(md);
    // ---- Table A: v2's picks
    let _ = writeln!(md, "## v2 vs v21 side by side, every headline pick (LABELLED DIAGNOSTIC + HINDSIGHT, proof step 1)");
    let _ = writeln!(md);
    let _ = writeln!(md, "Label: underlying = each traded option's own underlying (index: NIFTY, BANKNIFTY; stock: the F&O stocks Tickvault records); signal source = our Top Volume screener on Tickvault 1-minute candles (one segment, one expiry per list; unchanged from v2); traded = the event option SOLD naked, intraday; size = slot Rs 20 L / N, largest whole lots whose margin estimate fits; period = the 8 staged Tickvault days 2026-11-02..2026-11-11 (stock options 7: none on Muhurat 11-08); run `{run_id}` (v21) beside the v2 run `{v2name}`; proof step 1 (in-sample replay; dials chosen on the scored days); Walk-forward test NOT RUN; nothing goes live. 'Fed by a silence longer than the block' = filled blocks (sale, buy-back, square-off) whose first print followed the previous print by more than the block length; 'first sale block after a print before T' = trades whose first filled sale block's first print followed a print inside the signal candle (before T) by more than one block.");
    let _ = writeln!(md);
    let _ = writeln!(md, "**Table A: v2's six headline picks (picked under the v2 reading), each re-run under the anchor and the v2.1 readings.** The v2 reading replayed here must equal the v2 run's printed row (checked below).");
    let _ = writeln!(md);
    let _ = writeln!(md, "{V21_HEAD}");
    let mut checks: Vec<String> = Vec::new();
    let mut ck_lines: Vec<String> = Vec::new();
    let mut summary: Vec<String> = Vec::new();
    for (who, c, rows) in done.iter().filter(|x| x.0 == 0) {
        let _ = who;
        let Some((_, p)) = v2cfg.iter().find(|(cc, _)| cc.1 == c.1) else { continue };
        let pc = |k: &str| p.cols.get(k).cloned().unwrap_or_default();
        let refs = pc("refusals");
        let rget = |name: &str| -> String { refs.split(", ").find_map(|x| x.strip_prefix(&format!("{name} "))).unwrap_or("0").to_owned() };
        let other: u64 = refs.split(", ").filter_map(|x| x.rsplit_once(' ')).filter(|(k, _)| !["booked", "same_side_dup", "contract_open", "slots_full", "not_filled"].contains(k)).filter_map(|(_, v)| v.parse::<u64>().ok()).sum();
        let evn: u64 = refs.split(", ").filter_map(|x| x.rsplit_once(' ')).filter_map(|(_, v)| v.parse::<u64>().ok()).sum();
        let _ = writeln!(md, "| {} | v2 run `{v2name}` (printed) | {evn} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {other} | (not computed by v2) | | | | |", c.1, pc("net_detail_rs"), pc("trades"), pc("wins"), pc("max_dd_rs"), pc("worst_trade_rs"), pc("worst_day_rs"), pc("peak_margin_account_rs"), rget("same_side_dup"), rget("contract_open"), rget("slots_full"), rget("not_filled"));
        for r in rows {
            v21_row(&mut md, &c.1, r);
        }
        if let Some(r) = rows.iter().find(|r| r.reading == CapMode::Row) {
            let e = &r.ev;
            let same = pc("net_detail_rs") == fmt_rs(e.s.net) && pc("trades") == e.s.trades.to_string() && pc("wins") == e.s.wins.to_string() && pc("max_dd_rs") == fmt_rs(e.s.mdd) && pc("worst_trade_rs") == fmt_rs(e.s.worst) && pc("peak_margin_account_rs") == fmt_rs(e.s.pk_margin);
            let refs = pc("refusals");
            let rsame = e.why.iter().all(|(k, v)| refs.split(", ").any(|x| x == format!("{k} {v}")));
            checks.push(format!("{} {}", c.1, if same && rsame { "EQUAL (net, trades, wins, max DD, worst trade, peak margin, every verdict count)" } else { "DIFFERENT" }));
        }
        let key = format!("v2 {}", c.1);
        for r in rows {
            let ck = match r.reading {
                CapMode::Prorate => ck_cmp(&ck_pro, &key, &r.ev),
                CapMode::Anchor => ck_cmp(&ck_anc, &key, &r.ev),
                CapMode::Row => ck_cmp(&ck_row, &key, &r.ev),
            };
            ck_lines.push(format!("{} {}: checker's own code {}", c.1, trade::reading_short(r.reading), ck));
        }
        let v21n = rows.iter().find(|r| r.reading == CapMode::Prorate).map_or("-".to_owned(), |r| fmt_rs(r.ev.s.net));
        let ancn = rows.iter().find(|r| r.reading == CapMode::Anchor).map_or("-".to_owned(), |r| fmt_rs(r.ev.s.net));
        let v2n = rows.iter().find(|r| r.reading == CapMode::Row).map_or("-".to_owned(), |r| fmt_rs(r.ev.s.net));
        summary.push(format!("| {} (v2's pick) | {v2n} (printed by v2: {}) | {ancn} | **{v21n}** |", c.1, pc("net_detail_rs")));
    }
    let _ = writeln!(md);
    let _ = writeln!(md, "Replay check (this binary's v2 reading vs the v2 run's printed picks): {}.", if checks.is_empty() { "NOT DONE (no v2 run given, or its settings did not print back)".to_owned() } else { checks.join("; ") });
    let _ = writeln!(md);
    let _ = writeln!(md, "The independent checker's own node code (`check-v2/out/acct-summary*.tsv`, its sensitivity runs of F-A; read only) beside this engine, config by config: {}.", if ck_lines.is_empty() { "not given".to_owned() } else { ck_lines.join("; ") });
    let _ = writeln!(md);
    // ---- Table B: v21's own picks
    let _ = writeln!(md, "**Table B: v21's own headline picks (the whole dial grid re-searched under the v2.1 prorate reading), each also run under the anchor and the v2 readings.**");
    let _ = writeln!(md);
    let _ = writeln!(md, "{V21_HEAD}");
    for (_, c, rows) in done.iter().filter(|x| x.0 == 1) {
        for r in rows {
            v21_row(&mut md, &c.1, r);
        }
        let get = |m: CapMode| rows.iter().find(|r| r.reading == m).map_or("-".to_owned(), |r| fmt_rs(r.ev.s.net));
        summary.push(format!("| {} (v21's own pick) | {} | {} | **{}** |", c.1, get(CapMode::Row), get(CapMode::Anchor), get(CapMode::Prorate)));
    }
    let _ = writeln!(md);
    let _ = writeln!(md, "**Summary: net Rs under each reading (same config, same events, same exits, same N; only the fill capacity changes).**");
    let _ = writeln!(md);
    let _ = writeln!(md, "| pick | v2 reading | anchor reading | v2.1 prorate (v21) |");
    let _ = writeln!(md, "|---|---|---|---|");
    for l in &summary {
        let _ = writeln!(md, "{l}");
    }
    let _ = writeln!(md);
    for (c, _) in &v2cfg {
        if let Some(sr) = segs.get(c.0) {
            let _ = writeln!(md, "- v2 {}: {} || {} || N {} || block {} s", c.1, cfg_words(c.0, c.2, &sr.lv), exit_words(c.3), n_of(c.4), blk_of(c.0));
        }
    }
    for c in &v21cfg {
        if let Some(sr) = segs.get(c.0) {
            let _ = writeln!(md, "- v21 {}: {} || {} || N {} || block {} s", c.1, cfg_words(c.0, c.2, &sr.lv), exit_words(c.3), n_of(c.4), blk_of(c.0));
        }
    }
    let _ = writeln!(md);
    // ---- refusals by cause
    let _ = writeln!(md, "## Refusals by cause, every compared row (LABELLED DIAGNOSTIC + HINDSIGHT)");
    let _ = writeln!(md);
    let _ = writeln!(md, "Label: LABELLED DIAGNOSTIC + HINDSIGHT; underlying = each traded option's own underlying (index: NIFTY, BANKNIFTY; stock: the F&O stocks Tickvault records); signal source = our Top Volume screener on Tickvault 1-minute candles; traded = the event option SOLD naked, intraday; size = slot Rs 20 L / N; period = the 8 staged days 2026-11-02..2026-11-11; run `{run_id}`; each row's fill reading named in its row; proof step 1; Walk-forward test NOT RUN. Each event of a pick's list is either booked or refused for exactly one cause; 'not_filled' = the sale never filled one whole lot before 15:10 (thin usable volume under v2.1).");
    let _ = writeln!(md);
    let mut head = String::from("| pick | fill reading | events | booked |");
    let mut sep = String::from("|---|---|---|---|");
    for (k, _) in v2::CAUSES {
        let _ = write!(head, " {k} |");
        sep.push_str("---|");
    }
    head.push_str(" other |");
    sep.push_str("---|");
    let _ = writeln!(md, "{head}");
    let _ = writeln!(md, "{sep}");
    let mut rf_tsv = String::from("pick\tsource\treading\tevents\tbooked");
    for (k, _) in v2::CAUSES {
        let _ = write!(rf_tsv, "\t{k}");
    }
    rf_tsv.push_str("\tother\n");
    for (who, c, rows) in &done {
        for r in rows {
            let w = &r.ev.why;
            let mut line = format!("| {} {} | {} | {} | {} |", if *who == 0 { "v2's" } else { "v21's" }, c.1, trade::reading_short(r.reading), r.ev.events, w.get("booked").copied().unwrap_or(0));
            let mut tl = format!("{}\t{}\t{}\t{}\t{}", c.1, if *who == 0 { "v2 pick" } else { "v21 pick" }, trade::reading_short(r.reading), r.ev.events, w.get("booked").copied().unwrap_or(0));
            for (k, _) in v2::CAUSES {
                let n = w.get(k).copied().unwrap_or(0);
                let _ = write!(line, " {n} |");
                let _ = write!(tl, "\t{n}");
            }
            let other: u64 = w.iter().filter(|(k, _)| **k != "booked" && !v2::CAUSES.iter().any(|x| x.0 == **k)).map(|(_, v)| *v).sum();
            let _ = writeln!(md, "{line} {other} |");
            let _ = writeln!(rf_tsv, "{tl}\t{other}");
        }
    }
    let _ = writeln!(md);
    for (seg, sr) in segs.iter().enumerate() {
        if sr.days.is_empty() {
            continue;
        }
        let rf: Vec<String> = sr.refusals.iter().map(|(k, v)| format!("{}: {v}", if *k == "-" { "priced (no event-level refusal)" } else { k })).collect();
        let _ = writeln!(md, "- {} event-level refusals over every unique event of the grid (the same for every dial and reading except `no_reference`, which reads the tape): {}.", SEG_NAME.get(seg).copied().unwrap_or("?"), rf.join(", "));
    }
    let _ = writeln!(md);
    std::fs::write(rdir.join("refusals-by-cause.tsv"), &rf_tsv).map_err(|e| e.to_string())?;
    // every booked trade of every compared row
    let mut ct = String::from("config\tsource\treading\tday\tdecision_T\tcontract\tsecurity_id\tlast_fill_known\tmargin_blocked_rs\tnet_rs\n");
    for (who, c, rows) in &done {
        for r in rows {
            for b in &r.ev.booked {
                let (day, sym, sid) = cubes.get(usize::from(b.0)).map_or(("?".to_owned(), "?".to_owned(), 0), |cc| (fmt_day(cc.day), cc.cons.get(b.1 as usize).map_or("?".to_owned(), |k| k.symbol.clone()), cc.cons.get(b.1 as usize).map_or(0, |k| k.sid)));
                let _ = writeln!(ct, "{} {}\t{}\t{}\t{day}\t{}\t{sym}\t{sid}\t{}\t{}\t{}", if *who == 0 { "v2" } else { "v21" }, c.1, if *who == 0 { "v2 pick" } else { "v21 pick" }, trade::reading_short(r.reading), fmt_sod(i64::from(b.2)), fmt_sod(b.3), fmt_rs(b.4), fmt_rs(b.6));
            }
        }
    }
    std::fs::write(rdir.join("compare-trades-v21.tsv"), &ct).map_err(|e| e.to_string())?;
    // ---- FIX 3 in words
    let _ = writeln!(md, "## The sparse tape in numbers (FIX 3)");
    let _ = writeln!(md);
    let _ = writeln!(md, "Label: LABELLED DIAGNOSTIC + HINDSIGHT; underlying = each traded option's own underlying (index: NIFTY, BANKNIFTY; stock: the F&O stocks Tickvault records); signal source = Top Volume screener on Tickvault 1-minute candles; traded = the event option SOLD naked, intraday; size = slot Rs 20 L / N; period = the 8 staged days; run `{run_id}`; proof step 1. Read from the last five columns of Tables A and B: under the v2 reading most stock-option fill blocks are fed by a row that follows a silence longer than the block (its volume accumulated since the previous print); under v2.1 the same blocks keep their price but only the silence's seconds inside the block count. The last column is the median, over each row's trades, of the traded contract-day's 1-second prints (out of about 22,500 session seconds): index options mostly 13-18 thousand, stock options mostly 800-3,500 (one print every 6-28 seconds).");
    let _ = writeln!(md);
    // ---- 2-second index blocks (v21 picks, v2.1 reading)
    let _ = writeln!(md, "## Index options at 2-second blocks (v21 picks, v2.1 prorate reading)");
    let _ = writeln!(md);
    let _ = writeln!(md, "Label: LABELLED DIAGNOSTIC + HINDSIGHT; underlying = NIFTY, BANKNIFTY (INDEX options only); signal source = Top Volume screener on Tickvault 1-minute candles; traded = the event option SOLD naked, intraday; size = slot Rs 20 L / N; period = the 8 staged days; fill reading v2.1 prorate; same config, exits and N as each v21 index pick; only the block length changes (1 s headline, REQ 73b/73c; 2 s the harsher fill beside); run `{run_id}`; HINDSIGHT; proof step 1.");
    let _ = writeln!(md);
    for c in v21cfg.iter().filter(|c| c.0 == 0) {
        let base = done.iter().find(|x| x.0 == 1 && x.1.1 == c.1).and_then(|x| x.2.iter().find(|r| r.reading == CapMode::Prorate)).map(|r| (r.ev.s.net, r.ev.s.trades));
        let e2 = v2::eval_entries_full(&entries(c), c.3, c.4, nd(0), true, 2, CapMode::Prorate, cubes, tapes, var, dial, "2 s")?;
        let _ = writeln!(md, "- v21 {}: at 1-second blocks Rs {} on {} trades; **at 2-second blocks Rs {} on {} trades** (max DD Rs {}, worst trade Rs {}).", c.1, base.map_or("-".to_owned(), |b| fmt_rs(b.0)), base.map_or(0, |b| b.1), fmt_rs(e2.ev.s.net), e2.ev.s.trades, fmt_rs(e2.ev.s.mdd), fmt_rs(e2.ev.s.worst));
    }
    let _ = writeln!(md);
    // ---- expiry and margin of the v21 picks (rules unchanged from v2)
    let _ = writeln!(md, "## Expiry and margin of the v21 headline picks (rules unchanged from v2: R3, R4)");
    let _ = writeln!(md);
    let _ = writeln!(md, "Label: LABELLED DIAGNOSTIC + HINDSIGHT; underlying = each traded option's own underlying (index: NIFTY, BANKNIFTY; stock: the F&O stocks Tickvault records); signal source = Top Volume screener on Tickvault 1-minute candles; traded = the event option SOLD naked, intraday; size = slot Rs 20 L / N; period = the 8 staged days; run `{run_id}`; fill reading v2.1 prorate; proof step 1; Walk-forward test NOT RUN. Expiry facts per trade are in `trades-picks.tsv` (expiry weekday, trading day, sessions to expiry, class); the snapshot expiries and the calendar are the v2 report's (NIFTY 03 / 09 / 17 Nov, BANKNIFTY and stocks 23 Nov; no stock expiry day in the 8 days, so R3's roll is never reached). Margin: entry estimates (index ASSUMED formula, stock C_VAR1 2026-10-30 proxy), cap Rs 20,00,000; the spot-marked peak re-estimates every open position each minute with the live underlying (beside).");
    let _ = writeln!(md);
    let _ = writeln!(md, "| pick | trades | index trades on the expiry day / tomorrow's expiry / 2+ sessions | stock trades by sessions to expiry | peak margin in use Rs (account) | the same from trade intervals | refused margin_cap_20L | spot-marked peak Rs | minute ends above Rs 20 L (spot-marked) | at |");
    let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|");
    for p in po.iter().filter(|p| heads.iter().any(|h| p.pid.ends_with(h))) {
        let rm: Vec<(u16, u32, u32, i64, u64)> = p.rows.iter().map(|r| (r.di, r.ci, r.t, if r.known > 0 { sod_of(r.known) } else { crate::cal::END_S }, r.units)).collect();
        let (pk, over, at) = v2::remark_peak(&rm, cubes, var);
        let ix = if p.seg == 0 { format!("{} / {} / {}", p.rows.iter().filter(|r| r.sessions == 0).count(), p.rows.iter().filter(|r| r.sessions == 1).count(), p.rows.iter().filter(|r| r.sessions >= 2).count()) } else { "-".to_owned() };
        let st = if p.seg == 1 { let mut m: BTreeMap<i64, usize> = BTreeMap::new(); for r in &p.rows { *m.entry(r.sessions).or_insert(0) += 1; } m.iter().map(|(s, n)| format!("{n} at {s}")).collect::<Vec<_>>().join(", ") } else { "-".to_owned() };
        let _ = writeln!(md, "| {} | {} | {ix} | {st} | {} | {} | {} | {} | {over} | {at} |", p.pid, p.rows.len(), fmt_rs(p.s2.pk_margin), fmt_rs(peak_margin(&p.rows)), p.why.get("margin_cap_20L").copied().unwrap_or(0), fmt_rs(pk));
    }
    let _ = writeln!(md);
    Ok(md)
}

/// v2.1: one booked trade's sparse-tape facts (its own reading = the run's), with its first sale block's first print and the silence before it.
#[derive(Clone, Copy, Debug, Default)]
struct RowGap {
    nb: u32,
    ng: u32,
    ua: u64,
    ug: u64,
    rv: u64,
    uv: u64,
    first_before_t: bool,
    first_block: Option<i64>,
    first_prev: Option<i64>,
    first_gap: i64,
}

/// v2.1: re-walks one detailed trade (the run's reading) for its fill blocks' sparse-tape facts.
fn row_gap(r: &TradeRow, l: usize, blk: i64, cubes: &[Cube], tapes: &[SecTape], dial: &common_fill::Dial) -> Result<RowGap, String> {
    let (Some(c), Some(tp)) = (cubes.get(usize::from(r.di)), tapes.get(usize::from(r.di))) else { return Ok(RowGap::default()) };
    let Some(kc) = c.cons.get(r.ci as usize) else { return Ok(RowGap::default()) };
    let lot = u64::from(kc.lot);
    let dt = DayTape::new(c.day, tp.of(r.ci as usize), blk)?;
    let tape = dt.tape()?;
    let Some(ev) = trade::evidence(&tape, c.day, r.t, lot, u64::from(r.lots) * lot, l, r.x, dial)? else { return Ok(RowGap::default()) };
    let (nb, ng, ua, ug, rv, uv, fb) = trade::gap_facts(&dt, &ev, i64::from(r.t), blk);
    let mut g = RowGap { nb, ng, ua, ug, rv, uv, first_before_t: fb, ..RowGap::default() };
    if let Some(b) = ev.sale.iter().find(|b| b.why == common_fill::Why::Filled) {
        let rows = dt.rows();
        let prev = dt.prev_any();
        let i0 = rows.partition_point(|x| x.ts < b.block.at);
        if let Some((j, x)) = rows.get(i0..).unwrap_or(&[]).iter().enumerate().find(|(_, x)| x.ts < b.block.to && x.vol > 0) {
            let p = prev.get(i0 + j).copied().unwrap_or(x.ts - common_fill::US_S);
            g.first_block = Some(sod_of(b.block.at));
            g.first_prev = Some(sod_of(p));
            g.first_gap = (x.ts - p) / common_fill::US_S;
        }
    }
    Ok(g)
}
