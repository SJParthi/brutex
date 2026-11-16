//! v3 proof P3 (EXITS-DYNAMIC-DESIGN 7 B3): on 100% of the real days' (event, entry path, rule) queries,
//! - the fast scan (crate v2.2 `scan_fast`) equals crate v2.1's own `path::scan_one` on every v2.1 rule kind (Stop, Target, TimeExit,
//!   GoPositive, Trail), and THE LAW (`scan_law`, a plain walk per merged traded second) on EVERY rule;
//! - a binary dump of every path's NEW-rule queries (StopAt, TargetAt, TrailPct, ArmedStop, TimedStop: their parameters and the fast
//!   path's trigger second) for the independent NODE reference (`p3-node.mjs`), which walks the cube's own seconds tape.
//!
//! Dump `p3-<seg>-<yyyymmdd>.bin` (little endian): magic `SFP3DMP1`, then records: u32 contract index, u32 T (second of the day), u8 l,
//! u8 q, i32 E0, i32 E1, i32 watch end (seconds of the day), u16 fills, fills x (i32 block start, u32 units, i32 price), u16 rules, rules x
//! (u8 kind 1 StopAt 2 TargetAt 3 TrailPct 4 ArmedStop 5 TimedStop, i32 p1, i32 p2, i32 trigger second or -1).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use common_fill::path::{EntryView, Rule, scan_one};
use common_fill::xrule::{TrailIndex, XRule, scan_fast, scan_law};

use crate::cal::{CUT_S, SQ_S, at_sod, sod_of};
use crate::menu::{Menu, Sets};
use crate::read::Cal3;
use crate::stage::{MenuA, level_rules};
use crate::trade::{BaseErr, NL, Refuse, block_of, record_base, size_event, truncate};

fn to_v21(r: XRule) -> Option<Rule> {
    match r {
        XRule::Stop { x } => Some(Rule::Stop { x }),
        XRule::QuickCut { x } => Some(Rule::QuickCut { x }),
        XRule::Target { x } => Some(Rule::Target { x }),
        XRule::TimeExit { secs } => Some(Rule::TimeExit { secs }),
        XRule::GoPositive { secs } => Some(Rule::GoPositive { secs }),
        XRule::Trail { act, dist } => Some(Rule::Trail { act, dist }),
        _ => None,
    }
}

fn new_kind(r: &XRule) -> Option<(u8, i64, i64)> {
    match *r {
        XRule::StopAt { level } => Some((1, level, 0)),
        XRule::TargetAt { level } => Some((2, level, 0)),
        XRule::TrailPct { act, bp } => Some((3, act, i64::from(bp))),
        XRule::ArmedStop { act, off } => Some((4, act, off)),
        XRule::TimedStop { after, off } => Some((5, i64::from(after), off)),
        _ => None,
    }
}

/// `p3 <cube dir> <out dir> <days...> --var <C_VAR1> --holidays <tsv> --holidays2 <tsv> --menu <tsv> --sets <tsv> [--threads N]
/// [--cut v3|v2] [--no-dump]`
///
/// # Errors
/// A refused input.
#[allow(clippy::too_many_lines)]
pub fn cmd_p3(a: &[String]) -> Result<String, String> {
    let arg = |k: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).cloned();
    let (Some(cdir), Some(odir)) = (a.get(2), a.get(3)) else { return Err("p3 <cube dir> <out dir> <days...>".to_owned()) };
    let threads: usize = arg("--threads").and_then(|x| x.parse().ok()).unwrap_or(12);
    let days: Vec<String> = a.iter().skip(4).filter(|x| x.len() == 10 && x.starts_with("20")).cloned().collect();
    let var = crate::margin::load_var(Path::new(&arg("--var").ok_or("--var")?), "30102026")?;
    let menu = Menu::load(Path::new(&arg("--menu").ok_or("--menu")?))?;
    let _sets = Sets::load(Path::new(&arg("--sets").ok_or("--sets")?), &menu)?;
    let ma = MenuA::new(&menu.rules)?;
    let compat = arg("--cut").as_deref() == Some("v2");
    let dump = !a.iter().any(|x| x == "--no-dump");
    let h1 = arg("--holidays").ok_or("--holidays")?;
    let h2 = arg("--holidays2").ok_or("--holidays2")?;
    let odir = PathBuf::from(odir);
    std::fs::create_dir_all(&odir).map_err(|e| e.to_string())?;
    let mut cubes = Vec::new();
    let mut tapes = Vec::new();
    for d in &days {
        cubes.push(crate::cube::load(Path::new(cdir), d)?);
        tapes.push(crate::cube::load_sec(Path::new(cdir), d)?);
    }
    let mut cal3 = Cal3::load(&[Path::new(&h1), Path::new(&h2)])?;
    cal3.add_specials(&cubes);
    let dial = crate::trade::dial()?;
    let mut log = String::new();
    let _ = writeln!(log, "P3 started {}; menu sha256 {}; {} days; cut {}; dump {dump}", crate::report3::now(), menu.sha, days.len(), if compat { "v2" } else { "v3" });
    // the events of every config of both segments (the run's own screener), every unique event once
    for seg in 0..2_u8 {
        let seg_days: Vec<usize> = cubes.iter().enumerate().filter(|(_, c)| c.cons.iter().any(|k| k.kind == seg)).map(|(i, _)| i).collect();
        let sub: Vec<&crate::cube::Cube> = seg_days.iter().filter_map(|&i| cubes.get(i)).collect();
        let lv = crate::report::measure_refs(&sub, seg);
        let mut work: Vec<(usize, u32, Vec<(u32, u16)>)> = Vec::new();
        for &di in &seg_days {
            let Some(c) = cubes.get(di) else { continue };
            let de = crate::screen::generate(c, u16::try_from(di).unwrap_or(0), seg, &lv, false, true, None);
            // per unique event: its timeframe mask
            let mut mask: Vec<u16> = vec![0; de.uev.len()];
            for (id, l) in de.lists.iter().enumerate() {
                let tf = crate::screen::cfg_parts(id).0;
                for e in l {
                    if let Some(m) = mask.get_mut(e.ue as usize) {
                        *m |= 1 << tf;
                    }
                }
            }
            let mut by: BTreeMap<u32, Vec<(u32, u16)>> = BTreeMap::new();
            for (k, u) in de.uev.iter().enumerate() {
                let m = mask.get(k).copied().unwrap_or(0);
                if m != 0 {
                    by.entry(u.ci).or_default().push((u.t, m));
                }
            }
            for (ci, v) in by {
                work.push((di, ci, v));
            }
        }
        let next = AtomicUsize::new(0);
        let tot: Mutex<[u64; 8]> = Mutex::new([0; 8]);
        let files: Mutex<BTreeMap<usize, Vec<Vec<u8>>>> = Mutex::new(BTreeMap::new());
        let first_bad: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let blk = block_of(seg);
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| {
                    let mut t = [0_u64; 8];
                    let mut mine: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
                    loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        let Some((di, ci, evs)) = work.get(k) else { break };
                        let (Some(c), Some(tp)) = (cubes.get(*di), tapes.get(*di)) else { continue };
                        let Ok(dt) = crate::trade::DayTape::new(c.day, tp.of(*ci as usize), blk) else { continue };
                        let Ok(tape) = dt.tape() else { continue };
                        let ti = TrailIndex::new(&tape, false);
                        let pr = crate::read::Prep::new(c, *ci as usize, tp.of(*ci as usize));
                        let buf = mine.entry(*di).or_default();
                        for &(tt, m) in evs {
                            let ev = size_event(c, *ci as usize, tt, &var, true);
                            if ev.refuse != Refuse::None || ev.qs.is_empty() {
                                continue;
                            }
                            let Some(kc) = c.cons.get(*ci as usize) else { continue };
                            let lot = u64::from(kc.lot);
                            let qmax = u64::from(ev.qs.last().copied().unwrap_or(0)) * lot;
                            let paths = match record_base(&tape, c.day, i64::from(tt), lot, qmax, &dial) {
                                Ok(p) => p,
                                Err(BaseErr::NoReference | BaseErr::Crate(_)) => continue,
                            };
                            let base = crate::read::base_reading(c, *ci as usize, i64::from(tt), &pr, &cal3);
                            let tfw: Vec<(u8, i64, i64)> = (0..crate::screen::TFS.len()).filter(|&tf| m & (1 << tf) != 0).map(|tf| { let (h, r) = crate::read::window_reading(c, *ci as usize, i64::from(tt), crate::screen::TFS.get(tf).copied().unwrap_or(1)); (u8::try_from(tf).unwrap_or(0), h, r) }).collect();
                            let to = if compat { at_sod(c.day, CUT_S) } else { at_sod(c.day, SQ_S) };
                            for l in 0..NL {
                                let fl = paths.fills.get(l).cloned().unwrap_or_default();
                                let Some(b0) = fl.first().copied() else { continue };
                                let (main, per) = level_rules(&ma, &base, &tfw, b0.px);
                                let mut xs: Vec<XRule> = main.iter().filter_map(|r| r.as_ref().ok().map(|x| x.0)).collect();
                                for sr in &per {
                                    xs.extend(sr.iter().filter_map(|r| r.as_ref().ok().map(|x| x.0)));
                                }
                                for (q, &lots) in ev.qs.iter().enumerate() {
                                    let fq = truncate(&fl, u64::from(lots) * lot);
                                    let Some(f0) = fq.first() else { continue };
                                    let view = EntryView { e0: f0.block.at, e1: f0.block.known, fills: &fq };
                                    let mut st = 0_u32;
                                    let fast = scan_fast(&tape, &ti, &view, to, false, &xs, &mut st);
                                    let law = scan_law(&tape, &view, to, false, &xs);
                                    t[0] += u64::try_from(xs.len()).unwrap_or(0);
                                    for (i, (f, lw)) in fast.iter().zip(law.iter()).enumerate() {
                                        if f != lw {
                                            t[1] += 1;
                                            if let Ok(mut fb) = first_bad.lock()
                                                && fb.len() < 20
                                            {
                                                fb.push(format!("fast vs law: {} {} T {} l {l} q {q} rule {:?}: fast {f:?} law {lw:?}", c.ymd, kc.symbol, tt, xs.get(i)));
                                            }
                                        }
                                    }
                                    let old: Vec<(usize, Rule)> = xs.iter().enumerate().filter_map(|(i, r)| to_v21(*r).map(|x| (i, x))).collect();
                                    let rs: Vec<Rule> = old.iter().map(|x| x.1).collect();
                                    let mut s2 = 0_u32;
                                    let (one, _) = scan_one(&tape, &view, to, false, &rs, &[], &mut s2);
                                    t[2] += u64::try_from(rs.len()).unwrap_or(0);
                                    for (j, (i, _)) in old.iter().enumerate() {
                                        if fast.get(*i) != one.get(j) {
                                            t[3] += 1;
                                            if let Ok(mut fb) = first_bad.lock()
                                                && fb.len() < 20
                                            {
                                                fb.push(format!("fast vs scan_one: {} {} T {} l {l} q {q} rule {:?}", c.ymd, kc.symbol, tt, xs.get(*i)));
                                            }
                                        }
                                    }
                                    t[4] += 1;
                                    t[5] = t[5].max(u64::from(st));
                                    if dump {
                                        let news: Vec<(u8, i64, i64, i64)> = xs.iter().zip(fast.iter()).filter_map(|(r, f)| new_kind(r).map(|(k2, p1, p2)| (k2, p1, p2, f.map_or(-1, |tr| sod_of(tr.sec))))).collect();
                                        t[6] += u64::try_from(news.len()).unwrap_or(0);
                                        let i32of = |v: i64| i32::try_from(v).unwrap_or(if v < 0 { i32::MIN } else { i32::MAX });
                                        buf.extend_from_slice(&ci.to_le_bytes());
                                        buf.extend_from_slice(&tt.to_le_bytes());
                                        buf.push(u8::try_from(l).unwrap_or(0));
                                        buf.push(u8::try_from(q).unwrap_or(0));
                                        buf.extend_from_slice(&i32of(sod_of(view.e0)).to_le_bytes());
                                        buf.extend_from_slice(&i32of(sod_of(view.e1)).to_le_bytes());
                                        buf.extend_from_slice(&i32of(sod_of(to)).to_le_bytes());
                                        buf.extend_from_slice(&u16::try_from(fq.len()).unwrap_or(0).to_le_bytes());
                                        for b in &fq {
                                            buf.extend_from_slice(&i32of(sod_of(b.block.at)).to_le_bytes());
                                            buf.extend_from_slice(&u32::try_from(b.units).unwrap_or(u32::MAX).to_le_bytes());
                                            buf.extend_from_slice(&i32of(b.px).to_le_bytes());
                                        }
                                        buf.extend_from_slice(&u16::try_from(news.len()).unwrap_or(0).to_le_bytes());
                                        for (k2, p1, p2, f) in news {
                                            buf.push(k2);
                                            buf.extend_from_slice(&i32of(p1).to_le_bytes());
                                            buf.extend_from_slice(&i32of(p2).to_le_bytes());
                                            buf.extend_from_slice(&i32of(f).to_le_bytes());
                                        }
                                    }
                                }
                            }
                            t[7] += 1;
                        }
                    }
                    if let Ok(mut g) = tot.lock() {
                        for (x, y) in g.iter_mut().zip(t) {
                            *x += y;
                        }
                    }
                    if let Ok(mut f) = files.lock() {
                        for (di, b) in mine {
                            f.entry(di).or_default().push(b);
                        }
                    }
                });
            }
        });
        let t = tot.into_inner().unwrap_or_default();
        let _ = writeln!(log, "P3 {}: events {}, entry paths {}, rule queries {} (fast vs LAW differences {}), crate v2.1 rule queries {} (fast vs scan_one differences {}), new-rule queries dumped for node {}, max steps per path {}", if seg == 0 { "INDEX" } else { "STOCK" }, t[7], t[4], t[0], t[1], t[2], t[3], t[6], t[5]);
        for b in first_bad.into_inner().unwrap_or_default() {
            let _ = writeln!(log, "  {b}");
        }
        if dump {
            for (di, parts) in files.into_inner().unwrap_or_default() {
                let Some(c) = cubes.get(di) else { continue };
                let p = odir.join(format!("p3-{}-{}.bin", if seg == 0 { "idx" } else { "stk" }, c.ymd));
                let mut f = std::fs::File::create(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                f.write_all(b"SFP3DMP1").map_err(|e| e.to_string())?;
                for b in parts {
                    f.write_all(&b).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    let _ = writeln!(log, "P3 finished {}", crate::report3::now());
    std::fs::write(odir.join("P3.log"), &log).map_err(|e| e.to_string())?;
    Ok(log)
}
