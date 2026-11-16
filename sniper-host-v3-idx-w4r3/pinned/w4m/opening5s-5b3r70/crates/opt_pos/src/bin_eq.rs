//! `idx-eq`: gates G-EQ1 (S5) and G-EQ2 (S6) of SNIPER-IDXOPT-PLAN-v3.1 on the earlier one-second index runs (every trade
//! file of the 10 finished idx1s runs: check-idx1s-early-A / -B's rows).
//! * G-EQ1: the busiest leader, its units, the runner-up and its units, the lot and the lot source, recomputed through
//!   `opt_pos::chain` (the idx1s reading: the current expiry = the first listed expiry at or after the session in the file
//!   dated before it; per strike the folder within 4 days with the most units) equal to the tool's on every trade.
//! * G-EQ2: each trade fed its own decision, contract, lot, quantity (`qty_rule = floor`, the row's), limits and exit anchor,
//!   priced by `opt_pos::price::price_intraday` (block 1, both protection paths, REQ 70a pay, square-off, charges by
//!   `costs::option_clock`): the same fill rows and the same booked / dated / kept / off nets, booked path and unpaid units.
//!
//! WAVE 4 (check-build-idx-S1-S6 D3, D4): `--freeze FILE` (the dated table) gives each trade its one-order maximum for the
//! per-leg reading (D3); the leftover row is labelled `unclosable_in_session`, as the idx1s files write it (D4), and a settled
//! leftover `settled_at_intrinsic`.
//!
//! `idx-eq --store ROOT --listing F --ladder F --freeze F --out DIR RUN_DIR...`

#![allow(clippy::too_many_lines, reason = "one driver")]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use idx_cal::day::{self, Day};
use idx_cal::listing;
use idx_cal::storeio::{self, at_sod};
use opt_pos::chain::{self, Cand, Folders};
use opt_pos::freeze::FreezeTable;
use opt_pos::ladder::Ladder;
use opt_pos::price::{self, Leftover, Plan};
use opt_pos::secs::{self, Sec};

fn hms(s: &str) -> Option<i64> {
    let p: Vec<i64> = s.split(':').filter_map(|x| x.parse().ok()).collect();
    Some(p.first()? * 3600 + p.get(1)? * 60 + p.get(2)?)
}
fn stamp(s: &str) -> Option<i64> {
    Some(at_sod(day::parse(s.get(0..10)?)?, hms(s.get(11..19)?)?))
}
fn ist(us: i64) -> String {
    let (d, s) = storeio::ist_of(us);
    format!("{} {}", day::fmt(d), storeio::hms(s))
}
fn rs(s: &str) -> Option<i64> {
    let neg = s.starts_with('-');
    let t: String = s.trim_start_matches('-').trim_start_matches("Rs ").chars().filter(|c| *c != ',').collect();
    let (a, b) = t.split_once('.').unwrap_or((&t, "0"));
    let p = a.parse::<i64>().ok()? * 100 + format!("{b:0<2}").get(0..2)?.parse::<i64>().ok()?;
    Some(if neg { -p } else { p })
}

/// (block start, units, block volume, price, known).
type FillRow = (i64, u64, u64, i64, i64);

fn main() {
    if let Err(e) = run() {
        eprintln!("idx-eq: {e}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    let (mut root, mut lst, mut lad, mut out, mut frz) = (PathBuf::new(), PathBuf::new(), PathBuf::new(), PathBuf::new(), PathBuf::new());
    let mut runs = Vec::new();
    let mut i = 1;
    while let Some(k) = a.get(i) {
        let v = || a.get(i + 1).map(PathBuf::from).ok_or_else(|| format!("{k} needs a value"));
        match k.as_str() {
            "--store" => root = v()?,
            "--listing" => lst = v()?,
            "--ladder" => lad = v()?,
            "--out" => out = v()?,
            "--freeze" => frz = v()?,
            _ => {
                runs.push(PathBuf::from(k));
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let listing = listing::parse_listing(&fs::read_to_string(&lst).map_err(|e| e.to_string())?)?;
    let ladder = Ladder::parse(&fs::read_to_string(&lad).map_err(|e| e.to_string())?)?;
    let freeze = FreezeTable::parse(&fs::read_to_string(&frz).map_err(|e| format!("--freeze {}: {e}", frz.display()))?)?;
    let mut folders: BTreeMap<String, Folders> = BTreeMap::new();
    for ix in ["NIFTY", "BANKNIFTY"] {
        folders.insert(ix.to_string(), Folders::list(&root, ix)?);
    }
    let mut files: Vec<PathBuf> = Vec::new();
    for r in &runs {
        for sh in fs::read_dir(r).map_err(|e| e.to_string())? {
            let sh = sh.map_err(|e| e.to_string())?.path();
            if sh.is_dir() {
                for f in fs::read_dir(&sh).map_err(|e| e.to_string())? {
                    let p = f.map_err(|e| e.to_string())?.path();
                    if p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("trades-") && n.ends_with(".tsv")) {
                        files.push(p);
                    }
                }
            }
        }
    }
    files.sort();
    let mut diffs = String::from("file\ttrade\tgate\twhat\n");
    let mut cnt: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut busy_cache: BTreeMap<(String, Day, bool), (Vec<Cand>, Day)> = BTreeMap::new();
    let mut spot_cache: BTreeMap<(String, Day), Vec<Sec>> = BTreeMap::new();
    let mut opt_cache: BTreeMap<(String, Day), Vec<Sec>> = BTreeMap::new();
    let (mut n_tr, mut n_fill) = (0usize, 0usize);
    for f in &files {
        let text = fs::read_to_string(f).map_err(|e| e.to_string())?;
        let mut th: Vec<String> = Vec::new();
        let mut fh: Vec<String> = Vec::new();
        let mut trades: Vec<Vec<String>> = Vec::new();
        let mut fills: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
        for l in text.lines() {
            let c: Vec<String> = l.split('\t').map(str::to_string).collect();
            match c.first().map(String::as_str) {
                Some("#trade") => th = c,
                Some("#fill") => fh = c,
                Some("trade") => trades.push(c),
                Some("fill") => fills.entry(c.get(1).cloned().unwrap_or_default()).or_default().push(c),
                _ => {}
            }
        }
        let tc = |n: &str| th.iter().position(|h| h == n).ok_or_else(|| format!("no trade column {n}"));
        let fc = |n: &str| fh.iter().position(|h| h == n).ok_or_else(|| format!("no fill column {n}"));
        let cols: Vec<usize> = ["no", "index", "date", "candle", "decision_t", "contract", "lot", "lot_src", "busiest_units_before_t", "runner_up", "runner_up_units", "r_second", "r_high", "qty", "buy_protection_limit", "reserve", "units_filled", "money_in", "exit_decision", "late_exit_rejected", "net_booked", "net_dated", "share_level", "path_booked", "net_protection_kept", "net_protection_off", "unpaid_units"].iter().map(|n| tc(n)).collect::<Result<_, _>>()?;
        let fcols: Vec<usize> = ["block", "units", "block_volume", "price", "known_at", "kind"].iter().map(|n| fc(n)).collect::<Result<_, _>>()?;
        let name = f.display().to_string();
        let name = name.rsplit("/run/").next().unwrap_or(&name).to_string();
        for t in &trades {
            let g = |k: usize| cols.get(k).and_then(|&c| t.get(c)).map_or("", String::as_str);
            let no = g(0).to_string();
            let ix = g(1).to_string();
            let Some(d) = day::parse(g(2)) else { continue };
            n_tr += 1;
            let k: usize = g(3).parse().unwrap_or(1);
            let tt = stamp(g(4)).unwrap_or(0);
            let contract = g(5).to_string();
            let ce = contract.ends_with("CE");
            let mut bad = |gate: &'static str, what: String, diffs: &mut String| {
                *cnt.entry(gate).or_default() += 1;
                let _ = writeln!(diffs, "{name}\t{no}\t{gate}\t{what}");
            };
            // ---- G-EQ1
            let files_ix = listing.get(&ix).ok_or("listing index")?;
            let pf = files_ix.iter().rfind(|r| r.file < d).ok_or("no file before")?;
            let e = pf.listed.iter().map(|l| l.expiry).find(|x| *x >= d).ok_or("no expiry")?;
            let row = ladder.get(&ix, pf.file, e).ok_or_else(|| format!("no ladder row {ix} {} {}", day::fmt(pf.file), day::fmt(e)))?.clone();
            let key = (ix.clone(), d, ce);
            if !busy_cache.contains_key(&key) {
                if busy_cache.len() > 200_000 {
                    busy_cache.clear();
                }
                let ts: Vec<i64> = (1..=3).map(|j| at_sod(d, 9 * 3600 + 15 * 60 + 5 * j)).collect();
                let fo = folders.get(&ix).ok_or("folders")?;
                let c = chain::busiest_cands(&root, &ix, fo, &row, ce, e, d, &ts)?;
                busy_cache.insert(key.clone(), (c, e));
            }
            if !spot_cache.contains_key(&(ix.clone(), d)) {
                if spot_cache.len() > 100 {
                    spot_cache.clear();
                }
                spot_cache.insert((ix.clone(), d), secs::spot_day(&root, &ix, d)?);
            }
            let spot_t = spot_cache.get(&(ix.clone(), d)).and_then(|s| secs::last_before(s, tt)).map_or(0, |s| s.close);
            let (cands, _) = busy_cache.get(&key).ok_or("cache")?;
            let (lead, run_up) = chain::leader(cands, k - 1, spot_t);
            let lead_name = lead.map_or("-".to_string(), |c| c.folder.clone());
            let lead_u = lead.map_or(0, |c| c.units.get(k - 1).copied().unwrap_or(0));
            let ru_name = run_up.map_or("-".to_string(), |c| c.folder.clone());
            let ru_u = run_up.map_or(0, |c| c.units.get(k - 1).copied().unwrap_or(0));
            if lead_name != contract || lead_u.to_string() != g(8) {
                bad("G-EQ1", format!("leader {lead_name} {lead_u} vs {contract} {}", g(8)), &mut diffs);
            }
            if ru_name != g(9) || (ru_name != "-" && ru_u.to_string() != g(10)) {
                bad("G-EQ1", format!("runner-up {ru_name} {ru_u} vs {} {}", g(9), g(10)), &mut diffs);
            }
            if !opt_cache.contains_key(&(contract.clone(), d)) {
                if opt_cache.len() > 300 {
                    opt_cache.clear();
                }
                let rows = secs::option_day(&root, &ix, &contract, d)?.unwrap_or_default();
                opt_cache.insert((contract.clone(), d), rows);
            }
            let rows = opt_cache.get(&(contract.clone(), d)).ok_or("opt cache")?;
            let prints: Vec<u64> = rows.iter().filter(|r| r.ts < tt).map(|r| r.vol).collect();
            let (lot, src) = chain::lot_at(&row, &prints);
            if lot.to_string() != g(6) || src != g(7) {
                bad("G-EQ1", format!("lot {lot} '{src}' vs {} '{}'", g(6), g(7)), &mut diffs);
            }
            let last = secs::last_before(rows, tt);
            if last.map(|r| (ist(r.ts), r.high)) != Some((g(11).to_string(), rs(g(12)).unwrap_or(-1))) {
                bad("G-EQ1", format!("size ref {:?} vs {} {}", last.map(|r| (ist(r.ts), r.high)), g(11), g(12)), &mut diffs);
            }
            // ---- G-EQ2
            let xd = g(18);
            let xt = if xd.len() >= 19 && g(19) != "true" { stamp(xd) } else { None };
            let plan = Plan {
                d,
                t: tt,
                lot: u64::try_from(lot.max(1)).unwrap_or(1),
                qty: g(13).parse().unwrap_or(0),
                buy_lim: rs(g(14)).unwrap_or(0),
                reserve: rs(g(15)).unwrap_or(0),
                exit: xt,
                bp: g(22).trim_end_matches('%').parse::<u16>().unwrap_or(0) * 100,
                block_s: 1,
                leftover: Leftover::WorstSinceE0,
                freeze: u64::try_from(freeze.units(&ix, d).unwrap_or(0)).unwrap_or(0),
                max_order: u64::try_from(freeze.max_one_order(&ix, d, lot).unwrap_or(0)).unwrap_or(0),
            };
            let pr = match price::price_intraday(rows, &plan) {
                Ok(p) => p,
                Err(e) => {
                    bad("G-EQ2", format!("refused {e}"), &mut diffs);
                    continue;
                }
            };
            let bk = pr.paths.get(pr.booked).cloned().unwrap_or_default();
            let path_word = if pr.booked == 0 { "protection kept" } else { "protection off" };
            let kept_net = pr.paths.first().map_or(0, |p| p.net);
            let off_net = pr.paths.get(1).map_or(0, |p| p.net);
            let checks = [
                ("units_filled", bk.units_in.to_string(), g(16).to_string()),
                ("money_in", bk.value_in.to_string(), rs(g(17)).unwrap_or(-1).to_string()),
                ("net_booked", bk.net.to_string(), rs(g(20)).unwrap_or(-1).to_string()),
                ("net_dated", bk.net_dated.to_string(), rs(g(21)).unwrap_or(-1).to_string()),
                ("path_booked", path_word.to_string(), g(23).to_string()),
                ("net_protection_kept", kept_net.to_string(), rs(g(24)).unwrap_or(-1).to_string()),
                ("net_protection_off", off_net.to_string(), rs(g(25)).unwrap_or(-1).to_string()),
                ("unpaid_units", bk.unpaid.to_string(), g(26).to_string()),
            ];
            for (w, x, y) in checks {
                if x != y {
                    bad("G-EQ2", format!("{w} {x} vs {y}"), &mut diffs);
                }
            }
            let want: Vec<(String, FillRow)> = fills
                .get(&no)
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .map(|fr| {
                    let fg = |k: usize| fcols.get(k).and_then(|&c| fr.get(c)).map_or("", String::as_str);
                    (fg(5).split(' ').next().unwrap_or("").to_string(), (stamp(fg(0).get(0..19).unwrap_or("")).unwrap_or(0), fg(1).parse().unwrap_or(0), fg(2).parse().unwrap_or(0), rs(fg(3)).unwrap_or(0), stamp(fg(4)).unwrap_or(0)))
                })
                .collect();
            n_fill += want.len();
            let got: Vec<(String, FillRow)> = bk
                .fills
                .iter()
                .map(|x| (match x.0 { price::Leg::Entry => "entry", price::Leg::Exit => "exit", price::Leg::SquareOff => "square-off", price::Leg::Unclosable => "unclosable_in_session", price::Leg::Settled => "settled_at_intrinsic" }.to_string(), (x.1, x.2, x.3, x.4, x.5)))
                .collect();
            if got != want {
                bad("G-EQ2", format!("fills: {} crate rows vs {} file rows; first crate {:?} file {:?}", got.len(), want.len(), got.first(), want.first()), &mut diffs);
            }
        }
    }
    let mut rep = String::new();
    let _ = writeln!(rep, "G-EQ1 / G-EQ2 on the idx1s runs: trade files {}; trades {n_tr}; fill rows {n_fill}", files.len());
    let _ = writeln!(rep, "G-EQ1 differences (leader, runner-up, lot + source, size reference): {}", cnt.get("G-EQ1").copied().unwrap_or(0));
    let _ = writeln!(rep, "G-EQ2 differences (fills, units, money in, booked / dated / kept / off nets, booked path, unpaid): {}", cnt.get("G-EQ2").copied().unwrap_or(0));
    let _ = writeln!(rep, "VERDICT: {}", if cnt.is_empty() { "PASS: 0 differences" } else { "FAIL (see EQ-DIFFS.tsv)" });
    fs::write(out.join("EQ-REPORT.txt"), &rep).map_err(|e| e.to_string())?;
    fs::write(out.join("EQ-DIFFS.tsv"), &diffs).map_err(|e| e.to_string())?;
    print!("{rep}");
    Ok(())
}
