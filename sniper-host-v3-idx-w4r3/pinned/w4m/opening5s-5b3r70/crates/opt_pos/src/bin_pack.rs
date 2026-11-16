//! `idx-pack` (S5, wave 4): the index option pack's data parts (`opt_pos::pack`), read only on every input.
//!
//! `idx-pack itm --store ROOT --ladder F --freeze F --cal DIR --index IX --from D --to D --out DIR`
//!   DP-IDXITM (the A2 pick of every second, runs), DP-IDXLEAD (busiest + runner-up, runs) and DP-IDXNEED (the contract-day list:
//!   intraday picks, their later sessions to expiry, the roll window's next-expiry picks, the leaders), the F8 one-order gate on
//!   every second, sizes and times.
//! `idx-pack newpx --ticks ROOT --store ROOT --need F --index IX --bhav-root D... --out DIR`
//!   DP-NEWPX-IDXOPT: each needed contract-day's corrected seconds from the RAW NFO ticks (LTQ = 0 new-price rows kept, the
//!   scout's newpx rule = `idx_cal::ticks::fold_seconds`), in [09:15:00, dated F&O end), with the decoder check: (2) every second
//!   whose trade rows are all LTQ > 0 equals the store's 1-s bar on high, low, close and volume, every store bar has its ticks;
//!   (1) the day high / low of the whole day's corrected fold against the F&O bhavcopy's per-contract high / low, every miss
//!   listed. A contract-day whose check fails is listed as REFUSED (a later refusal when touched, s4.3a). Sizes and times.
//!   RESUMED WAVE 4: the member is also searched under the OLDER YYMON name (2018-19 NIFTY monthly files); a contract-day with
//!   no member is EMPTY (no store row either: consistent) or REFUSED/missing (the store has rows); every failure is written with
//!   BOTH sources to `DP-NEWPX-FAILS-<IX>.tsv` (per second: both / ticks_only / store_only; per contract-day: missing / empty /
//!   unchecked / hl_miss), the sidecar of the production reader `opt_pos::newpx` (the DATA law).
//! `idx-pack law --pack DIR --index IX --out FILE` (RESUMED WAVE 4)
//!   The DATA law on REAL data: every contract-day of every month's `DP-NEWPX-FAILS` through `opt_pos::newpx::day_rows` (the
//!   production reader), each result checked against BOTH sources: every failing second is a hole, never kinder than either
//!   source (a `both` second: HIGH >= both HIGHs, LOW <= both LOWs, volume <= both volumes; a one-source second: no fill row), a
//!   whole-day failure is one whole-session hole (`missing`: no rows), every other second equals the corrected second exactly.

#![allow(clippy::too_many_lines, reason = "one driver per data part")]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use idx_cal::day::{self, Day};
use idx_cal::storeio::{self, at_sod};
use idx_cal::ticks;
use opt_pos::chain::Folders;
use opt_pos::freeze::FreezeTable;
use opt_pos::ladder::Ladder;
use opt_pos::pack::{self, Live, LotState, Pick, Runs};
use opt_pos::secs;

struct CalRow {
    file_read: Day,
    known_listed: Day,
    known_eff: Day,
    next_eff: Option<Day>,
    d_exp: bool,
}

struct Args {
    kv: BTreeMap<String, Vec<String>>,
}
impl Args {
    fn one(&self, k: &str) -> Result<String, String> {
        self.kv.get(k).and_then(|v| v.first()).cloned().ok_or_else(|| format!("--{k} is required"))
    }
    fn path(&self, k: &str) -> Result<PathBuf, String> {
        self.one(k).map(PathBuf::from)
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let cmd = a.get(1).cloned().unwrap_or_default();
    let mut kv: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut i = 2;
    while let (Some(k), Some(v)) = (a.get(i), a.get(i + 1)) {
        kv.entry(k.trim_start_matches("--").to_owned()).or_default().push(v.clone());
        i += 2;
    }
    let args = Args { kv };
    let r = match cmd.as_str() {
        "itm" => run_itm(&args),
        "newpx" => run_newpx(&args),
        "law" => run_law(&args),
        x => Err(format!("usage: idx-pack itm|newpx|law ... (got {x})")),
    };
    if let Err(e) = r {
        eprintln!("idx-pack: {e}");
        std::process::exit(2);
    }
}

fn cal_rows(cal: &Path, ix: &str) -> Result<BTreeMap<Day, CalRow>, String> {
    let mut m = BTreeMap::new();
    for l in fs::read_to_string(cal.join(format!("DP-IDXCAL-{ix}.tsv"))).map_err(|e| e.to_string())?.lines() {
        if l.starts_with('#') || l.starts_with("session") {
            continue;
        }
        let c: Vec<&str> = l.split('\t').collect();
        let p = |k: usize| c.get(k).and_then(|x| day::parse(x));
        if let (Some(s), Some(f), Some(kl), Some(ke)) = (p(0), p(2), p(3), p(4)) {
            m.insert(s, CalRow { file_read: f, known_listed: kl, known_eff: ke, next_eff: p(6), d_exp: c.get(9) == Some(&"1") });
        }
    }
    Ok(m)
}

/// alias date -> [(first file, last file, final date)] from IDXCAL-SERIES (S1).
type FinalOf = BTreeMap<Day, Vec<(Day, Day, Day)>>;

fn series(cal: &Path, ix: &str) -> Result<FinalOf, String> {
    let mut final_of: FinalOf = BTreeMap::new();
    for l in fs::read_to_string(cal.join(format!("IDXCAL-SERIES-{ix}.tsv"))).map_err(|e| e.to_string())?.lines() {
        if l.starts_with('#') || l.starts_with("series") {
            continue;
        }
        let c: Vec<&str> = l.split('\t').collect();
        let als: Vec<(Day, Day, Day)> = c.get(1).unwrap_or(&"").split(" -> ").filter_map(|a| Some((day::parse(a.get(0..10)?)?, day::parse(a.get(12..22)?)?, day::parse(a.get(24..34)?)?))).collect();
        let Some(fin) = als.last().map(|x| x.0) else { continue };
        for (dt, f0, f1) in als {
            final_of.entry(dt).or_default().push((f0, f1, fin));
        }
    }
    Ok(final_of)
}

fn folder_date(final_of: &FinalOf, alias: Day, file: Day) -> Day {
    final_of.get(&alias).and_then(|v| v.iter().find(|(f0, f1, _)| *f0 <= file && file <= *f1)).map_or(alias, |x| x.2)
}

fn ladder_alias(l: &Ladder, ix: &str, file: Day, eff: Day) -> Option<Day> {
    if l.get(ix, file, eff).is_some() {
        return Some(eff);
    }
    (1..=3).map(|k| eff + k).find(|x| l.get(ix, file, *x).is_some())
}

fn hms(sod: i64) -> String {
    format!("{:02}:{:02}:{:02}", sod / 3600, (sod / 60) % 60, sod % 60)
}

fn run_itm(a: &Args) -> Result<(), String> {
    let t0 = Instant::now();
    let root = a.path("store")?;
    let ix = a.one("index")?;
    let cal = a.path("cal")?;
    let out = a.path("out")?;
    let (from, to) = (day::parse(&a.one("from")?).ok_or("from")?, day::parse(&a.one("to")?).ok_or("to")?);
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let ladder = Ladder::parse(&fs::read_to_string(a.path("ladder")?).map_err(|e| e.to_string())?)?;
    let freeze = FreezeTable::parse(&fs::read_to_string(a.path("freeze")?).map_err(|e| e.to_string())?)?;
    let cr = cal_rows(&cal, &ix)?;
    let final_of = series(&cal, &ix)?;
    let fo = Folders::list(&root, &ix)?;
    let sessions: Vec<Day> = cr.keys().copied().collect();
    let mut itm_txt = String::from("day\tside\tjob\tfrom\tto\tpick\tstrike\tdepth\tskip_no_trade\tskip_big\tlot\tmax_order\n");
    let mut lead_txt = String::from("day\tside\tfrom\tto\tleader\trunner_up\n");
    // Q38 fit_strict (RESUMED WAVE 4; plan s8 S5 / s6.3 "the fit_strict contracts"): on a day where the smallest freeze KNOWN BEFORE
    // the session over [d, the held expiry] is below today's freeze, the strict pick is taken every second too; its runs go to
    // DP-IDXSTRICT and its contract-days (and their later sessions) join DP-IDXNEED as `strict` / `strict_pos`.
    let mut strict_txt = String::from("day\tside\tjob\tfrom\tto\tpick\tstrike\tdepth\tskip_no_trade\tskip_big\tlot\tmax_order\tstrict_freeze\n");
    let (mut n_strict_days, mut n_strict_runs) = (0u64, 0u64);
    let mut need: BTreeMap<(String, Day), BTreeSet<&'static str>> = BTreeMap::new();
    let (mut n_sec, mut n_runs, mut n_lead_runs, mut n_noitm, mut n_f8_bad, mut n_days, mut n_nospot) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut per_side_picks: Vec<usize> = Vec::new();
    let mut f8_named = String::new();
    for (&d, c) in cr.range(from..=to) {
        n_days += 1;
        let spot = secs::spot_day(&root, &ix, d)?;
        let fu = freeze.units(&ix, d).unwrap_or(0);
        let mut jobs: Vec<(u8, Day, i64, i64)> = vec![(0, c.known_listed, 9 * 3600 + 15 * 60 + 1, 15 * 3600 + 30 * 60)];
        if c.d_exp
            && c.known_eff == d
            && let Some(ne) = c.next_eff
        {
            jobs.push((1, ladder_alias(&ladder, &ix, c.file_read, ne).unwrap_or(ne), 15 * 3600, 15 * 3600 + 30 * 60));
        }
        for (job, alias, s_from, s_to) in jobs {
            let e_job = if job == 0 { c.known_eff } else { c.next_eff.unwrap_or(c.known_eff) };
            let fs_strict = freeze.strict_min(&ix, d, e_job).unwrap_or(fu);
            let strict = fs_strict > 0 && fs_strict < fu;
            if strict {
                n_strict_days += 1;
            }
            let Some(row) = ladder.get(&ix, c.file_read, alias) else {
                let _ = writeln!(itm_txt, "{}\t-\t{job}\t-\t-\tno_ladder_row\t0\t0\t0\t0\t0\t0", day::fmt(d));
                continue;
            };
            let fdate = folder_date(&final_of, alias, c.file_read);
            for ce in [true, false] {
                let side = if ce { "CE" } else { "PE" };
                let mut live: Vec<Live> = Vec::new();
                // F-e (RESUMED WAVE 4): a listed strike with no store folder or no rows that day is a GHOST: counted by the ITM rule
                let mut ghosts: Vec<i64> = Vec::new();
                for &k in if ce { &row.ce } else { &row.pe } {
                    if !fo.has(fdate, k, ce) {
                        ghosts.push(k);
                        continue;
                    }
                    let name = storeio::contract_name(fdate, k, ce);
                    if let Some(rows) = secs::option_day(&root, &ix, &name, d)? {
                        live.push(Live { strike: k, folder: name, rows, cursor: 0, units: 0, lot: LotState::new(row) });
                    } else {
                        ghosts.push(k);
                    }
                }
                let mut buf = pack::lead_buf(&live);
                let mut runs: Runs<Pick> = Runs::default();
                let mut lruns: Runs<(Option<String>, Option<String>)> = Runs::default();
                let mut sp_cur = 0usize;
                let mut picks: BTreeSet<String> = BTreeSet::new();
                let mut spicks: BTreeSet<String> = BTreeSet::new();
                let mut sruns: Runs<Pick> = Runs::default();
                for sod in s_from..s_to {
                    let t = at_sod(d, sod);
                    for c in &mut live {
                        c.advance(t);
                    }
                    while spot.get(sp_cur).is_some_and(|r| r.ts < t) {
                        sp_cur += 1;
                    }
                    let sp = sp_cur.checked_sub(1).and_then(|k| spot.get(k)).map(|r| r.close);
                    let (p, _q) = pack::pick_at(&live, &ghosts, ce, sp, t, fu)?;
                    n_sec += 1;
                    if p.what == "no_itm_fit" {
                        n_noitm += 1;
                    }
                    if p.what == "no_spot" {
                        n_nospot += 1;
                    }
                    if !pack::f8_ok(&p, &live) {
                        n_f8_bad += 1;
                        if f8_named.len() < 4000 {
                            let _ = writeln!(f8_named, "F8 FAIL {} {side} {} {}", day::fmt(d), hms(sod), p.what);
                        }
                    }
                    if p.strike != 0 {
                        picks.insert(p.what.clone());
                    }
                    runs.push(sod, p);
                    if strict {
                        let (ps, _) = pack::pick_at(&live, &ghosts, ce, sp, t, fs_strict)?;
                        if ps.strike != 0 {
                            spicks.insert(ps.what.clone());
                        }
                        sruns.push(sod, ps);
                    }
                    if job == 0
                        && let Some(s) = sp
                    {
                        let (l1, l2) = pack::lead_at(&mut buf, &live, s);
                        for x in [&l1, &l2].into_iter().flatten() {
                            need.entry((x.clone(), d)).or_default().insert("lead");
                        }
                        lruns.push(sod, (l1, l2));
                    }
                }
                per_side_picks.push(picks.len());
                for f in &spicks {
                    need.entry((f.clone(), d)).or_default().insert("strict");
                    if let Some(e) = f.get(0..10).and_then(day::parse) {
                        for &dd in sessions.iter().filter(|&&x| x > d && x <= e) {
                            need.entry((f.clone(), dd)).or_default().insert("strict_pos");
                        }
                    }
                }
                n_strict_runs += u64::try_from(sruns.v.len()).unwrap_or(0);
                for (f, z, p) in &sruns.v {
                    let _ = writeln!(strict_txt, "{}\t{side}\t{job}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{fs_strict}", day::fmt(d), hms(*f), hms(*z), p.what, p.strike, p.depth, p.skip_nt, p.skip_big, p.lot, p.max_order);
                }
                for f in &picks {
                    need.entry((f.clone(), d)).or_default().insert(if job == 0 { "itm" } else { "roll" });
                    // positional: the picked contract's later sessions to its expiry (the folder's date is its true expiry)
                    if let Some(e) = f.get(0..10).and_then(day::parse) {
                        for &dd in sessions.iter().filter(|&&x| x > d && x <= e) {
                            need.entry((f.clone(), dd)).or_default().insert("pos");
                        }
                    }
                }
                n_runs += u64::try_from(runs.v.len()).unwrap_or(0);
                n_lead_runs += u64::try_from(lruns.v.len()).unwrap_or(0);
                for (f, z, p) in &runs.v {
                    let _ = writeln!(itm_txt, "{}\t{side}\t{job}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", day::fmt(d), hms(*f), hms(*z), p.what, p.strike, p.depth, p.skip_nt, p.skip_big, p.lot, p.max_order);
                }
                for (f, z, (l1, l2)) in &lruns.v {
                    let _ = writeln!(lead_txt, "{}\t{side}\t{}\t{}\t{}\t{}", day::fmt(d), hms(*f), hms(*z), l1.as_deref().unwrap_or("-"), l2.as_deref().unwrap_or("-"));
                }
            }
        }
    }
    let tag = format!("{ix}-{}-{}", day::fmt(from), day::fmt(to));
    let (iz, lz) = (pack::zst(itm_txt.as_bytes()), pack::zst(lead_txt.as_bytes()));
    fs::write(out.join(format!("DP-IDXITM-{tag}.tsv.zst")), &iz).map_err(|e| e.to_string())?;
    fs::write(out.join(format!("DP-IDXLEAD-{tag}.tsv.zst")), &lz).map_err(|e| e.to_string())?;
    fs::write(out.join(format!("DP-IDXSTRICT-{tag}.tsv.zst")), pack::zst(strict_txt.as_bytes())).map_err(|e| e.to_string())?;
    let mut need_txt = String::from("contract\tday\twhy\n");
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for ((f, d), w) in &need {
        for k in w {
            *kinds.entry(k).or_default() += 1;
        }
        let _ = writeln!(need_txt, "{f}\t{}\t{}", day::fmt(*d), w.iter().copied().collect::<Vec<_>>().join(","));
    }
    fs::write(out.join(format!("DP-IDXNEED-{tag}.tsv")), &need_txt).map_err(|e| e.to_string())?;
    let mx = per_side_picks.iter().copied().max().unwrap_or(0);
    let mean = if per_side_picks.is_empty() { 0 } else { per_side_picks.iter().sum::<usize>() * 100 / per_side_picks.len() };
    let mut rep = String::new();
    let _ = writeln!(rep, "idx-pack itm {tag}: sessions {n_days}; seconds priced {n_sec} (both sides, current + roll window); no_itm_fit seconds {n_noitm}; no_spot seconds {n_nospot}");
    let _ = writeln!(rep, "DP-IDXITM runs {n_runs} (raw {} B, zstd {} B); DP-IDXLEAD runs {n_lead_runs} (raw {} B, zstd {} B)", itm_txt.len(), iz.len(), lead_txt.len(), lz.len());
    let _ = writeln!(rep, "distinct ITM-fit contracts a side-day: max {mx}, mean {}.{:02}", mean / 100, mean % 100);
    let _ = writeln!(rep, "DP-IDXNEED contract-days {} ({} B): by kind {kinds:?}", need.len(), need_txt.len());
    let _ = writeln!(rep, "F8 one-order gate: {n_f8_bad} failures on {n_sec} seconds");
    let _ = writeln!(rep, "fit_strict (Q38, beside): day-jobs (current or roll window) with a stricter known freeze {n_strict_days}; DP-IDXSTRICT runs {n_strict_runs}");
    let _ = writeln!(rep, "wall {:.1} s", t0.elapsed().as_secs_f64());
    rep.push_str(&f8_named);
    fs::write(out.join(format!("PACK-ITM-{tag}.txt")), &rep).map_err(|e| e.to_string())?;
    print!("{rep}");
    Ok(())
}

/// (expiry, strike, CE?) -> (high, low) of one bhavcopy day.
type BhavHl = BTreeMap<(Day, i64, bool), (i64, i64)>;

/// The F&O bhavcopy's per-contract (high, low) of `index` on file day `d` (either form), keyed (expiry, strike, CE?).
fn bhav_hl(roots: &[PathBuf], d: Day, index: &str) -> Result<Option<BhavHl>, String> {
    let p = idx_cal::bhav::bhav_path(roots, d, true).or_else(|| idx_cal::bhav::bhav_path(roots, d, false));
    let Some(p) = p else { return Ok(None) };
    let csv = idx_cal::bhav::read_single_member(&p)?;
    let text = String::from_utf8_lossy(&csv);
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().unwrap_or("").split(',').map(str::trim).collect();
    let col = |n: &str| head.iter().position(|h| *h == n);
    let mut m = BTreeMap::new();
    // UDiFF: TckrSymb, FinInstrmTp (IDO), XpryDt, StrkPric, OptnTp, HghPric, LwPric; old form: INSTRUMENT, SYMBOL, EXPIRY_DT, STRIKE_PR, OPTION_TYP, HIGH, LOW
    let (sy, tp, xp, sk, ot, hi, lo, udiff) = if let Some(s) = col("TckrSymb") {
        (s, col("FinInstrmTp"), col("XpryDt"), col("StrkPric"), col("OptnTp"), col("HghPric"), col("LwPric"), true)
    } else {
        (col("SYMBOL").ok_or("bhav: no SYMBOL")?, col("INSTRUMENT"), col("EXPIRY_DT"), col("STRIKE_PR"), col("OPTION_TYP"), col("HIGH"), col("LOW"), false)
    };
    let (Some(tp), Some(xp), Some(sk), Some(ot), Some(hi), Some(lo)) = (tp, xp, sk, ot, hi, lo) else { return Err("bhav: a column is missing".into()) };
    for l in lines {
        let c: Vec<&str> = l.split(',').map(str::trim).collect();
        let g = |k: usize| c.get(k).copied().unwrap_or("");
        if g(sy) != index || !(g(tp) == "OPTIDX" || g(tp) == "IDO") {
            continue;
        }
        let e = if udiff { day::parse(g(xp)) } else { day::parse_dmy(g(xp)) };
        let (Some(e), Some(k), Some(h), Some(w)) = (e, idx_cal::bhav::paisa(g(sk)), idx_cal::bhav::paisa(g(hi)), idx_cal::bhav::paisa(g(lo))) else { continue };
        m.insert((e, k, g(ot) == "CE"), (h, w));
    }
    Ok(Some(m))
}

fn run_newpx(a: &Args) -> Result<(), String> {
    let t0 = Instant::now();
    let tick_root = a.path("ticks")?;
    let root = a.path("store")?;
    let ix = a.one("index")?;
    let out = a.path("out")?;
    let roots: Vec<PathBuf> = a.kv.get("bhav-root").map(|v| v.iter().map(PathBuf::from).collect()).unwrap_or_default();
    let cal = a.path("cal")?;
    let final_of = series(&cal, &ix)?;
    // every alias of a series by its final date (the tick member carries the listed date of its day)
    let mut aliases_of: BTreeMap<Day, BTreeSet<Day>> = BTreeMap::new();
    for (al, v) in &final_of {
        for (_, _, fin) in v {
            aliases_of.entry(*fin).or_default().insert(*al);
        }
    }
    let mut by_day: BTreeMap<Day, Vec<String>> = BTreeMap::new();
    for l in fs::read_to_string(a.path("need")?).map_err(|e| e.to_string())?.lines().skip(1) {
        let c: Vec<&str> = l.split('\t').collect();
        if let (Some(f), Some(d)) = (c.first(), c.get(1).and_then(|x| day::parse(x))) {
            by_day.entry(d).or_default().push((*f).to_owned());
        }
    }
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    const MON: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
    let mut n_rt_bad = 0u64;
    let (mut n_cd, mut n_secs, mut n_price_only, mut n_kinder, mut n_check2_bad, mut n_refused, mut n_hl_miss, mut n_hl_checked, mut raw_b, mut z_b) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0usize, 0usize);
    let mut misses = String::new();
    let (mut n_empty, mut n_unchecked) = (0u64, 0u64);
    // DP-NEWPX-FAILS (RESUMED WAVE 4): the DATA law's sidecar (lane B v2 s5.4, signed POSITIONAL v2): every decoder-check failure
    // with BOTH sources, per second (`both`, `ticks_only`, `store_only`) or per contract-day (`missing`, `empty`, `unchecked`,
    // `hl_miss`), so the production reader applies the law without re-folding a tick.
    let mut fails = String::from("contract\tday\tsod\tkind\ts_high\ts_low\ts_close\ts_vol\tt_high\tt_low\tt_close\tt_vol\n");
    let hms = |s: i64| format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60);
    for (&d, cons) in &by_day {
        let (y, m, dd) = day::to_civil(d);
        let mon = MON.get(usize::try_from(m).unwrap_or(1).saturating_sub(1)).copied().unwrap_or("JAN");
        let path = tick_root.join("options").join(format!("{y:04}")).join(format!("{mon}_{y:04}")).join(format!("GFDLNFO_TICK_{dd:02}{m:02}{y:04}.bts"));
        let mut opened = ticks::read_index(&path);
        let hl = bhav_hl(&roots, d, &ix)?;
        let end_sod = (secs::fno_end(d) - at_sod(d, 0)) / 1_000_000;
        let mut day_bin: Vec<u8> = Vec::new();
        for f in cons {
            n_cd += 1;
            let (Some(e), Some(rest)) = (f.get(0..10).and_then(day::parse), f.get(11..)) else { continue };
            let (k, ce) = match rest.rsplit_once('-') {
                Some((k, s)) => (k.parse::<i64>().unwrap_or(0), s == "CE"),
                None => (0, true),
            };
            let Ok((ref mut file, ref entries)) = opened else {
                n_refused += 1;
                let _ = writeln!(misses, "REFUSED {f} {}: no NFO tick day file", day::fmt(d));
                continue;
            };
            // the member under any alias of the series (the folder's date first)
            let mut cands: Vec<Day> = vec![e];
            cands.extend(aliases_of.get(&e).map_or(&BTreeSet::new(), |s| s).iter().copied().filter(|x| *x != e));
            let mut got = None;
            let rupees = k / 100;
            let side_s = if ce { "CE" } else { "PE" };
            let mut names: Vec<String> = cands
                .iter()
                .map(|al| {
                    let (ay, am, ad) = day::to_civil(*al);
                    let amon = MON.get(usize::try_from(am).unwrap_or(1).saturating_sub(1)).copied().unwrap_or("JAN");
                    format!("{ix}{ad:02}{amon}{:02}{rupees}{side_s}.NFO.csv", ay % 100)
                })
                .collect();
            // RESUMED WAVE 4: the OLDER YYMON name (e.g. NIFTY18SEP10950CE.NFO.csv) that the 2018-19 MONTHLY files use (the first
            // run refused all 2,868 NIFTY contract-days of 2018-09..2019-01 for want of it; opt-beside s28 item 4 reads both forms)
            let (ey, em, _) = day::to_civil(e);
            let emon = MON.get(usize::try_from(em).unwrap_or(1).saturating_sub(1)).copied().unwrap_or("JAN");
            names.push(format!("{ix}{:02}{emon}{rupees}{side_s}.NFO.csv", ey % 100));
            for tk in names {
                if let Some(en) = entries.iter().find(|x| ticks::basename(&x.name) == tk || ticks::basename(&x.name) == tk.replace(".csv", ".CSV")) {
                    got = Some((en, tk));
                    break;
                }
            }
            let Some((en, tk)) = got else {
                // RESUMED WAVE 4: EMPTY (the store has no row either: no trade that day, consistent, not a failure) or MISSING
                // (the store has rows but the tick file has no member: a failing contract-day, a whole-session HOLE)
                let n_store = secs::option_day(&root, &ix, f, d)?.map_or(0, |v| v.len());
                if n_store == 0 {
                    n_empty += 1;
                    let _ = writeln!(fails, "{f}\t{}\t-\tempty\t-\t-\t-\t0\t-\t-\t-\t0", day::fmt(d));
                    let _ = writeln!(misses, "EMPTY {f} {}: no tick member and no store row (no trade that day: consistent)", day::fmt(d));
                } else {
                    n_refused += 1;
                    let _ = writeln!(fails, "{f}\t{}\t-\tmissing\t-\t-\t-\t{n_store}\t-\t-\t-\t0", day::fmt(d));
                    let _ = writeln!(misses, "REFUSED {f} {}: no tick member under any alias while the store has {n_store} rows", day::fmt(d));
                }
                continue;
            };
            let (rows, _) = ticks::read_entry(file, en, &tk)?;
            let (secs_c, _) = ticks::fold_seconds(&rows, 9 * 3600 + 15 * 60, i32::try_from(end_sod).unwrap_or(0));
            // (2) against the store's 1-s bars (the store keeps the LTQ > 0 rows: the `traded` reading)
            let store = secs::option_day(&root, &ix, f, d)?.unwrap_or_default();
            let st: BTreeMap<i64, (i64, i64, i64, u64)> = store.iter().map(|r| ((r.ts - at_sod(d, 0)) / 1_000_000, (r.high, r.low, r.close, r.vol))).collect();
            let mut bad = 0u64;
            let mut seen: BTreeSet<i64> = BTreeSet::new();
            let mut f_rows = String::new();
            for s in &secs_c {
                n_secs += 1;
                let sod = i64::from(s.sod);
                if s.price_only() {
                    n_price_only += 1;
                }
                if let Some((_, th, tl, tc)) = s.traded {
                    if secs::halt_der(d).is_some_and(|(x, z)| at_sod(d, sod) >= x && at_sod(d, sod) < z) {
                        continue;
                    }
                    seen.insert(sod);
                    if (th, tl) != (s.high, s.low) {
                        n_kinder += 1;
                    }
                    if st.get(&sod) != Some(&(th, tl, tc, s.volume)) {
                        bad += 1;
                        match st.get(&sod) {
                            Some(&(sh, sl, sc, sv)) => {
                                let _ = writeln!(f_rows, "{f}\t{}\t{}\tboth\t{sh}\t{sl}\t{sc}\t{sv}\t{th}\t{tl}\t{tc}\t{}", day::fmt(d), hms(sod), s.volume);
                            }
                            None => {
                                let _ = writeln!(f_rows, "{f}\t{}\t{}\tticks_only\t-\t-\t-\t0\t{th}\t{tl}\t{tc}\t{}", day::fmt(d), hms(sod), s.volume);
                            }
                        }
                    }
                }
            }
            for (sod, &(sh, sl, sc, sv)) in st.iter().filter(|(x, _)| !seen.contains(x)) {
                bad += 1;
                let _ = writeln!(f_rows, "{f}\t{}\t{}\tstore_only\t{sh}\t{sl}\t{sc}\t{sv}\t-\t-\t-\t0", day::fmt(d), hms(*sod));
            }
            let unchecked = bad > 0 && st.is_empty();
            if unchecked {
                // no store row at all that day (2026-10-30: the frozen store lacks the day): nothing to check against
                n_unchecked += 1;
                let _ = writeln!(fails, "{f}\t{}\t-\tunchecked\t-\t-\t-\t0\t-\t-\t-\t{bad}", day::fmt(d));
                let _ = writeln!(misses, "UNCHECKED {f} {}: no store row; {bad} traded tick seconds (a whole-session HOLE)", day::fmt(d));
            } else {
                fails.push_str(&f_rows);
            }
            // (1) the whole day's corrected high / low against the bhavcopy
            if let Some(m) = hl.as_ref().and_then(|m| m.get(&(e, k, ce)).copied().or_else(|| aliases_of.get(&e).and_then(|s| s.iter().find_map(|al| m.get(&(*al, k, ce)).copied())))) {
                n_hl_checked += 1;
                let (all, _) = ticks::fold_seconds(&rows, 0, 86_400);
                let (h, l) = all.iter().fold((i64::MIN, i64::MAX), |(h, l), s| (h.max(s.high), l.min(s.low)));
                if (h, l) != m && m != (0, 0) {
                    n_hl_miss += 1;
                    let _ = writeln!(misses, "HL {f} {}: corrected {h}/{l} bhav {}/{}", day::fmt(d), m.0, m.1);
                    let _ = writeln!(fails, "{f}\t{}\t-\thl_miss\t{}\t{}\t-\t0\t{h}\t{l}\t-\t0", day::fmt(d), m.0, m.1);
                }
            }
            if bad > 0 && !unchecked {
                n_check2_bad += 1;
                let _ = writeln!(misses, "CHECK2 {f} {}: {bad} seconds differ from the store's traded bars", day::fmt(d));
            }
            // the compact block, round-tripped before it is kept
            let ns: Vec<pack::NewSec> = secs_c.iter().map(|s| (s.sod, s.open, s.high, s.low, s.close, s.volume)).collect();
            let blk = pack::enc_block(f, &ns);
            let mut at = 0usize;
            let (n2, v2) = pack::dec_block(&blk, &mut at)?;
            if n2 != *f || v2 != ns || at != blk.len() {
                n_rt_bad += 1;
            }
            day_bin.extend_from_slice(&blk);
        }
        let z = pack::zst(&day_bin);
        raw_b += day_bin.len();
        z_b += z.len();
        let dir = out.join(format!("{y:04}"));
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        fs::write(dir.join(format!("{}.newpx.zst", day::fmt(d))), &z).map_err(|e| e.to_string())?;
    }
    let mut rep = String::new();
    let _ = writeln!(rep, "idx-pack newpx {ix}: contract-days {n_cd} on {} days; corrected seconds {n_secs} (price-only {n_price_only}; seconds whose new-price LTQ = 0 rows widen the store's high / low {n_kinder})", by_day.len());
    let _ = writeln!(rep, "decoder check (2) vs the store's 1-s bars: {n_check2_bad} contract-days differ; refused (no day file / no member while the store has rows) {n_refused}; empty (no member, no store row) {n_empty}; unchecked (no store row, ticks present) {n_unchecked}");
    let _ = writeln!(rep, "decoder check (1) day high / low vs the F&O bhavcopy: {n_hl_miss} misses of {n_hl_checked} checked");
    let _ = writeln!(rep, "compact blocks round-trip failures {n_rt_bad}; bytes: compact {raw_b}, zstd {z_b}; wall {:.1} s", t0.elapsed().as_secs_f64());
    rep.push_str(&misses);
    fs::write(out.join(format!("PACK-NEWPX-{ix}.txt")), &rep).map_err(|e| e.to_string())?;
    fs::write(out.join(format!("DP-NEWPX-FAILS-{ix}.tsv")), &fails).map_err(|e| e.to_string())?;
    print!("{}", rep.lines().take(4).collect::<Vec<_>>().join("\n"));
    println!();
    Ok(())
}

/// RESUMED WAVE 4: `idx-pack law` (see the crate text): the production reader's DATA law on every failing contract-day of the
/// pack, checked against both sources in the sidecar; the counts by kind and every violation (0 required) are written to `--out`.
fn run_law(a: &Args) -> Result<(), String> {
    use opt_pos::newpx::{self, FailKind, Fails, NewpxDay, Src};
    let t0 = Instant::now();
    let dir = a.path("pack")?;
    let ix = a.one("index")?;
    let out = a.path("out")?;
    let mut months: Vec<PathBuf> = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.join(format!("DP-NEWPX-FAILS-{ix}.tsv")).exists()).collect();
    months.sort();
    let mut kinds: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_src: BTreeMap<&'static str, u64> = BTreeMap::new();
    let (mut n_keys, mut n_fail_secs, mut n_hole_secs, mut n_rows, mut n_bad) = (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut bad = String::new();
    for m in &months {
        let mut fl = Fails::default();
        fl.add(&fs::read_to_string(m.join(format!("DP-NEWPX-FAILS-{ix}.tsv"))).map_err(|e| e.to_string())?)?;
        let mut days: BTreeMap<Day, Option<NewpxDay>> = BTreeMap::new();
        let keys: Vec<(String, Day)> = fl.keys().cloned().collect();
        for (f, d) in &keys {
            n_keys += 1;
            if !days.contains_key(d) {
                days.insert(*d, NewpxDay::open(m, *d)?);
            }
            let blk = days.get(d).and_then(Option::as_ref).and_then(|x| x.get(f));
            let rows_f = fl.get(f, *d);
            for r in rows_f {
                *kinds.entry(format!("{:?}", r.kind)).or_insert(0) += 1;
            }
            let (from, to, halt) = (at_sod(*d, 9 * 3600 + 15 * 60), secs::fno_end(*d), secs::halt_der(*d));
            let r = newpx::day_rows(*d, blk, rows_f, from, to, halt);
            *by_src
                .entry(match r.src {
                    Src::Agree => "agree",
                    Src::Seconds(_) => "seconds",
                    Src::WholeHole(FailKind::Missing, _) => "whole_hole_missing",
                    Src::WholeHole(FailKind::Unchecked, _) => "whole_hole_unchecked",
                    Src::WholeHole(..) => "whole_hole_hl_miss",
                    Src::Empty => "empty",
                    Src::NotInPack => "not_in_pack",
                })
                .or_insert(0) += 1;
            n_rows += u64::try_from(r.rows.len()).unwrap_or(0);
            n_hole_secs += r.holes.iter().map(|h| u64::try_from((h.1 - h.0) / 1_000_000).unwrap_or(0)).sum::<u64>();
            let mut viol = |why: String| {
                n_bad += 1;
                if n_bad <= 200 {
                    let _ = writeln!(bad, "VIOLATION {f} {}: {why}", day::fmt(*d));
                }
            };
            let in_hole = |t: i64| r.holes.iter().any(|h| t >= h.0 && t < h.1);
            let row_at = |t: i64| r.rows.iter().find(|x| x.ts == t);
            let corrected: BTreeMap<i64, (i64, i64, i64, u64)> = blk.unwrap_or(&[]).iter().map(|s| (at_sod(*d, i64::from(s.0)), (s.2, s.3, s.4, s.5))).collect();
            let whole = rows_f.iter().find(|x| matches!(x.kind, FailKind::Missing | FailKind::Unchecked | FailKind::HlMiss)).map(|x| x.kind);
            let empty = rows_f.iter().any(|x| x.kind == FailKind::Empty);
            if empty {
                if !r.rows.is_empty() || !r.holes.is_empty() {
                    viol("empty contract-day has rows or holes".into());
                }
                continue;
            }
            if let Some(k) = whole {
                if r.holes != vec![(from, to)] {
                    viol(format!("{k:?}: not one whole-session hole"));
                }
                if k == FailKind::Missing && !r.rows.is_empty() {
                    viol("missing: rows present".into());
                }
            }
            let mut fail_ts: BTreeSet<i64> = BTreeSet::new();
            for x in rows_f.iter().filter(|x| x.sod.is_some()) {
                let t = at_sod(*d, x.sod.unwrap_or(0));
                if t < from || t >= to || halt.is_some_and(|(p, q)| t >= p && t < q) {
                    continue;
                }
                n_fail_secs += 1;
                fail_ts.insert(t);
                if !in_hole(t) {
                    viol(format!("failing second {t} not in a hole"));
                }
                match (x.kind, row_at(t)) {
                    (FailKind::Both, Some(w)) => {
                        let (Some(st), Some(c)) = (x.store, corrected.get(&t)) else {
                            viol("both: a source is missing".into());
                            continue;
                        };
                        if w.high < st.0.max(c.0) || w.low > st.1.min(c.1) || w.vol > st.3.min(c.3) {
                            viol(format!("both at {t}: row {}/{}/{} kinder than store {:?} / corrected {:?}", w.high, w.low, w.vol, st, c));
                        }
                    }
                    (FailKind::TicksOnly | FailKind::StoreOnly, Some(_)) => viol(format!("one-source second {t} has a fill row")),
                    _ => {}
                }
            }
            if whole != Some(FailKind::Missing) {
                for w in &r.rows {
                    if fail_ts.contains(&w.ts) {
                        continue;
                    }
                    match corrected.get(&w.ts) {
                        Some(c) if *c == (w.high, w.low, w.close, w.vol) => {}
                        _ => viol(format!("row at {} differs from the corrected second", w.ts)),
                    }
                }
                let n_corr = corrected.iter().filter(|(t, c)| **t >= from && **t < to && !halt.is_some_and(|(p, q)| **t >= p && **t < q) && c.3 > 0 && !fail_ts.contains(t)).count();
                let n_kept = r.rows.iter().filter(|w| !fail_ts.contains(&w.ts)).count();
                if n_corr != n_kept {
                    viol(format!("clean traded seconds {n_corr} but rows {n_kept}"));
                }
            }
        }
    }
    let mut rep = String::new();
    let _ = writeln!(rep, "idx-pack law {ix}: the DATA law on every failing contract-day of {} months: contract-days {n_keys}; by result {by_src:?}; sidecar rows by kind {kinds:?}", months.len());
    let _ = writeln!(rep, "failing seconds inside the windows {n_fail_secs}; hole seconds {n_hole_secs}; fill rows kept {n_rows}; VIOLATIONS {n_bad} -> {}", if n_bad == 0 { "PASS" } else { "FAIL" });
    let _ = writeln!(rep, "wall {:.1} s", t0.elapsed().as_secs_f64());
    rep.push_str(&bad);
    fs::write(&out, &rep).map_err(|e| e.to_string())?;
    print!("{}", rep.lines().take(2).collect::<Vec<_>>().join("\n"));
    println!();
    Ok(())
}
