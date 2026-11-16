//! `f1-replay` (S2): check-v21 F1. Every fill row of the 10 finished one-second index runs (state/opening5s-20261111/run/
//! full-idx1s-*/share-*/trades-*.tsv: the trade files check-idx1s-early-A and -B re-walked) is replayed on crate v2.1's
//! `std70::walk` with a 1-second tape (`Tape::with_block(1)`) built from the same raw 1-second option bars (the frozen
//! store, generation-aware door). Each order is rebuilt as the tool and checker A built it (check-idx1s-early-A-work/
//! a-core.mjs `recompute`): the entry BUY from the decision candle's end, stopped at the exit decision (or the 15:10
//! square-off start), its limit the trade row's buy protection limit on the protection-kept path and none on the off
//! path, its payable exposure the trade row's reserve (REQ 70a) on both paths; the exit SELL from the exit decision to the
//! square-off start, its limit Dhan's protection from the last print before the decision (kept path); the square-off SELL
//! from 15:10 to the dated F&O end. Only the booked path's fills are in the files. Pass: 0 differences in block, units,
//! volume, price and known_at, and the same number of fill rows per trade.
//!
//! WAVE 4 (check-build-idx-S1-S6 D4): units still open at the dated F&O end are the row `unclosable_in_session` (the idx1s
//! files' own label), valued exactly as `opt_pos::price` does: block = the last second before the F&O end, volume 0, price = the
//! lowest LOW since E0 (the start of the entry's first filling block; 5 paisa when none), known at the F&O end. It is compared
//! like every other row.
//!
//! `f1-replay --store ROOT --out DIR RUN_DIR...`

#![allow(clippy::too_many_lines, reason = "one replay driver")]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use common_fill::std70::{BlockFill, Kind, LevelFill, Order, Why, walk};
use common_fill::tape::{Row, SecIndex, Seg, Tape};
use common_fill::{Dial, LevelMask, MAXL, Side, US_S, limits};
use idx_cal::day::{self, Day};
use idx_cal::storeio::{self, at_sod};
use store::path::Timeframe;

#[derive(Clone, Copy, Debug)]
struct R {
    ts: i64,
    high: i64,
    low: i64,
    close: i64,
    vol: u64,
}
impl Row for R {
    fn ts(&self) -> i64 {
        self.ts
    }
    fn high(&self) -> i64 {
        self.high
    }
    fn low(&self) -> i64 {
        self.low
    }
    fn vol(&self) -> u64 {
        self.vol
    }
}

fn hms(s: &str) -> Option<i64> {
    let p: Vec<i64> = s.split(':').filter_map(|x| x.parse().ok()).collect();
    Some(p.first()? * 3600 + p.get(1)? * 60 + p.get(2)?)
}
/// `YYYY-MM-DD HH:MM:SS` (IST) to UTC micros.
fn stamp(s: &str) -> Option<i64> {
    let d = day::parse(s.get(0..10)?)?;
    Some(at_sod(d, hms(s.get(11..19)?)?))
}
fn ist(us: i64) -> String {
    let (d, s) = storeio::ist_of(us);
    format!("{} {}", day::fmt(d), storeio::hms(s))
}
/// `Rs 1,23,456.78` / `-Rs 5.00` to paisa.
fn rs(s: &str) -> Option<i64> {
    let neg = s.starts_with('-');
    let t: String = s.trim_start_matches('-').trim_start_matches("Rs ").chars().filter(|c| *c != ',').collect();
    let (a, b) = t.split_once('.').unwrap_or((&t, "0"));
    let p = a.parse::<i64>().ok()? * 100 + format!("{b:0<2}").get(0..2)?.parse::<i64>().ok()?;
    Some(if neg { -p } else { p })
}

/// The derivatives halts (check-idx1s-early-A a-core.mjs HALT_DER, from the tool's session facts).
fn halt(d: Day) -> Option<(i64, i64)> {
    let f = |a: &str, b: &str| Some((at_sod(d, hms(a)?), at_sod(d, hms(b)?)));
    match day::fmt(d).as_str() {
        "2020-03-13" => f("09:21:45", "10:21:00"),
        "2020-03-23" => f("09:58:01", "10:57:00"),
        "2021-02-24" => f("11:40:00", "15:45:00"),
        _ => None,
    }
}
fn fno_end(d: Day) -> i64 {
    match day::fmt(d).as_str() {
        "2021-02-24" => at_sod(d, 17 * 3600),
        _ if d >= day::from_civil(2026, 8, 3) => at_sod(d, 15 * 3600 + 40 * 60),
        _ => at_sod(d, 15 * 3600 + 30 * 60),
    }
}
fn sq0(d: Day) -> i64 {
    let s = at_sod(d, 15 * 3600 + 10 * 60);
    match halt(d) {
        Some((a, b)) if s >= a && s < b => b,
        _ => s,
    }
}

struct Day1 {
    rows: Vec<R>,
}

fn load(root: &Path, ix: &str, contract: &str, d: Day) -> Result<Day1, String> {
    let bars = storeio::day_bars(root, Vendor::Gdfl, "FNO", ix, Some(contract), Timeframe::SECOND_1, d, 9 * 3600 + 15 * 60, (fno_end(d) - at_sod(d, 0)) / US_S)?.ok_or_else(|| format!("{contract} {}: no month file", day::fmt(d)))?;
    let h = halt(d);
    let rows = bars
        .iter()
        .filter(|b| b.volume > 0 && b.high > 0 && b.low > 0 && b.high >= b.low)
        .filter(|b| !h.is_some_and(|(a, z)| b.ts_micros >= a && b.ts_micros < z))
        .map(|b| R { ts: b.ts_micros, high: b.high, low: b.low, close: b.close, vol: u64::try_from(b.volume).unwrap_or(0) })
        .collect();
    Ok(Day1 { rows })
}

/// One recorded fill: (block start, units, block volume, price, known).
type F = (i64, u64, u64, i64, i64);

#[allow(clippy::too_many_arguments, reason = "one order")]
fn walk1(tape: &Tape<'_, R>, side: Side, anchor: i64, stop: i64, lot: u64, want: u64, limit: Option<i64>, money: Option<i64>, bp: u16, kind: Kind) -> Result<(Vec<F>, LevelFill), String> {
    let dial = Dial::new(&[bp]).map_err(|e| format!("{e:?}"))?;
    let o = Order { side, anchor, stop, unit: lot, limit, kind };
    let mut w = [0u64; MAXL];
    if let Some(x) = w.first_mut() {
        *x = want;
    }
    let mut m: [Option<i64>; MAXL] = [None; MAXL];
    if let Some(x) = m.first_mut() {
        *x = money;
    }
    let mut out = [LevelFill::default(); MAXL];
    let mut v: Vec<F> = Vec::new();
    let mut rec = |_l: usize, bf: &BlockFill| {
        if bf.why == Why::Filled && bf.units > 0 {
            v.push((bf.block.at, bf.units, bf.block.vol, bf.px, bf.block.known));
        }
    };
    walk(tape, &o, &dial, LevelMask::one(0), &mut w, Some(&m), None, &mut out, Some(&mut rec)).map_err(|e| format!("walk {e:?}"))?;
    Ok((v, out.first().copied().unwrap_or_default()))
}

fn main() {
    if let Err(e) = run() {
        eprintln!("f1-replay: {e}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    let mut root = PathBuf::new();
    let mut out = PathBuf::new();
    let mut runs = Vec::new();
    let mut i = 1;
    while let Some(k) = a.get(i) {
        match k.as_str() {
            "--store" => {
                root = PathBuf::from(a.get(i + 1).ok_or("--store")?);
                i += 2;
            }
            "--out" => {
                out = PathBuf::from(a.get(i + 1).ok_or("--out")?);
                i += 2;
            }
            _ => {
                runs.push(PathBuf::from(k));
                i += 1;
            }
        }
    }
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let mut files: Vec<PathBuf> = Vec::new();
    for r in &runs {
        for sh in fs::read_dir(r).map_err(|e| format!("{}: {e}", r.display()))? {
            let sh = sh.map_err(|e| e.to_string())?.path();
            if !sh.is_dir() {
                continue;
            }
            for f in fs::read_dir(&sh).map_err(|e| e.to_string())? {
                let p = f.map_err(|e| e.to_string())?.path();
                if p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("trades-") && n.ends_with(".tsv")) {
                    files.push(p);
                }
            }
        }
    }
    files.sort();
    let mut rep = String::new();
    let mut diffs = String::from("file\ttrade\twhat\n");
    let (mut n_tr, mut n_fill, mut n_diff, mut n_tr_diff, mut n_lim_diff, mut n_refused) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut cache: BTreeMap<(String, Day), Result<Day1, String>> = BTreeMap::new();
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
        let tc = |n: &str| th.iter().position(|h| h == n);
        let fc = |n: &str| fh.iter().position(|h| h == n);
        let (Some(c_no), Some(c_ix), Some(c_date), Some(c_dec), Some(c_con), Some(c_lot), Some(c_qty), Some(c_lim), Some(c_res), Some(c_xd), Some(c_late), Some(c_share), Some(c_path), Some(_c_uf)) = (
            tc("no"), tc("index"), tc("date"), tc("decision_t"), tc("contract"), tc("lot"), tc("qty"), tc("buy_protection_limit"), tc("reserve"), tc("exit_decision"), tc("late_exit_rejected"), tc("share_level"), tc("path_booked"), tc("units_filled"),
        ) else {
            return Err(format!("{}: trade header", f.display()));
        };
        let (Some(f_block), Some(f_units), Some(f_vol), Some(f_px), Some(f_known), Some(f_kind)) = (fc("block"), fc("units"), fc("block_volume"), fc("price"), fc("known_at"), fc("kind")) else {
            return Err(format!("{}: fill header", f.display()));
        };
        let name = f.strip_prefix(runs.first().and_then(|r| r.parent()).unwrap_or(Path::new("/"))).unwrap_or(f).display().to_string();
        for t in &trades {
            let g = |c: usize| t.get(c).map_or("", String::as_str);
            let no = g(c_no).to_string();
            let ix = g(c_ix);
            let Some(d) = day::parse(g(c_date)) else { continue };
            n_tr += 1;
            let mut want_rows: Vec<(String, F)> = Vec::new();
            for fr in fills.get(&no).map_or(&[][..], Vec::as_slice) {
                let fg = |c: usize| fr.get(c).map_or("", String::as_str);
                let blk = fg(f_block);
                let st = stamp(blk.get(0..19).unwrap_or("")).unwrap_or(0);
                let kind = fg(f_kind).split(' ').next().unwrap_or("").to_string();
                want_rows.push((kind, (st, fg(f_units).parse().unwrap_or(0), fg(f_vol).parse().unwrap_or(0), rs(fg(f_px)).unwrap_or(0), stamp(fg(f_known)).unwrap_or(0))));
            }
            n_fill += want_rows.len();
            for (k, _) in &want_rows {
                *by_kind.entry(k.clone()).or_default() += 1;
            }
            let lot: u64 = g(c_lot).parse().unwrap_or(0);
            let qty: u64 = g(c_qty).parse().unwrap_or(0);
            let bp: u16 = g(c_share).trim_end_matches('%').parse::<u16>().unwrap_or(0) * 100;
            let kept = g(c_path) == "protection kept";
            let buy_lim = rs(g(c_lim));
            let reserve = rs(g(c_res));
            let t0 = stamp(g(c_dec)).unwrap_or(0);
            let xd = g(c_xd);
            let late = g(c_late) == "true";
            let xt = if xd.len() >= 19 { stamp(xd).unwrap_or(0) } else { 0 };
            let s0 = sq0(d);
            let decided = xt > 0 && !late && xt < s0;
            let cancel = if decided { xt } else { s0 };
            let contract = g(c_con).to_string();
            let key = (contract.clone(), d);
            if !cache.contains_key(&key) {
                if cache.len() > 600 {
                    cache.clear();
                }
                cache.insert(key.clone(), load(&root, ix, &contract, d));
            }
            let Some(Ok(day1)) = cache.get(&(contract.clone(), d)) else {
                n_refused += 1;
                let _ = writeln!(diffs, "{name}\t{no}\tstore refused: {:?}", cache.get(&(contract.clone(), d)).and_then(|x| x.as_ref().err()));
                continue;
            };
            let rows = &day1.rows;
            let open = at_sod(d, 9 * 3600 + 15 * 60);
            let end = fno_end(d);
            let segs: Vec<Seg> = match halt(d) {
                Some((a0, z)) => vec![Seg { open, last_sec: a0 - US_S, auction_open: false }, Seg { open: z, last_sec: end - US_S, auction_open: false }],
                None => vec![Seg { open, last_sec: end - US_S, auction_open: false }],
            };
            let res = (|| -> Result<Vec<(String, F)>, String> {
                let ix0 = SecIndex::new(rows, open, end).map_err(|e| format!("index {e:?}"))?;
                let tape = Tape::new(rows, &ix0, None).and_then(|t| t.with_segments(&segs)).and_then(|t| t.with_block(1)).map_err(|e| format!("tape {e:?}"))?;
                // the buy limit recomputed from the last print before t (S8) must equal the row's
                let ltp = rows.iter().rev().find(|r| r.ts < t0).map(|r| r.close);
                let lim_re = ltp.map(|p| limits::protection(p, Side::Buy, 1_000, 5));
                if lim_re != buy_lim {
                    n_lim_diff += 1;
                }
                let mut got: Vec<(String, F)> = Vec::new();
                let (en, ef) = walk1(&tape, Side::Buy, t0, cancel, lot, qty, if kept { buy_lim } else { None }, reserve, bp, Kind::Entry)?;
                got.extend(en.into_iter().map(|x| ("entry".to_string(), x)));
                let mut rest = ef.units;
                if decided && rest > 0 {
                    let sl = rows.iter().rev().find(|r| r.ts < xt).map(|r| limits::protection(r.close, Side::Sell, 1_000, 5));
                    let (xs, xf) = walk1(&tape, Side::Sell, xt, s0, lot, rest, if kept { sl } else { None }, None, bp, Kind::SignalExit)?;
                    got.extend(xs.into_iter().map(|x| ("exit".to_string(), x)));
                    rest -= xf.units;
                }
                if rest > 0 {
                    let (qs, qf) = walk1(&tape, Side::Sell, s0, end, lot, rest, None, None, bp, Kind::SquareOff)?;
                    got.extend(qs.into_iter().map(|x| ("square-off".to_string(), x)));
                    rest -= qf.units;
                }
                if rest > 0 {
                    let e0 = ef.e0;
                    let px = rows.iter().filter(|r| r.ts >= e0 && r.ts < end).map(|r| r.low).min().unwrap_or(5);
                    got.push(("unclosable_in_session".to_string(), (end - US_S, rest, 0, px, end)));
                }
                Ok(got)
            })();
            match res {
                Err(e) => {
                    n_refused += 1;
                    let _ = writeln!(diffs, "{name}\t{no}\treplay refused: {e}");
                }
                Ok(got) => {
                    let mut bad = 0usize;
                    if got.len() != want_rows.len() {
                        bad += 1;
                        let _ = writeln!(diffs, "{name}\t{no}\trow count crate {} vs file {}", got.len(), want_rows.len());
                    }
                    for (j, (gk, gf)) in got.iter().enumerate() {
                        match want_rows.get(j) {
                            Some((wk, wf)) if wk == gk && wf == gf => {}
                            Some((wk, wf)) => {
                                bad += 1;
                                let _ = writeln!(diffs, "{name}\t{no}\trow {j}: crate {gk} {} {} vol {} px {} known {} | file {wk} {} {} vol {} px {} known {}", ist(gf.0), gf.1, gf.2, gf.3, ist(gf.4), ist(wf.0), wf.1, wf.2, wf.3, ist(wf.4));
                            }
                            None => {}
                        }
                    }
                    if bad > 0 {
                        n_tr_diff += 1;
                        n_diff += bad;
                    }
                }
            }
        }
    }
    let _ = writeln!(rep, "F1 REPLAY on crate v2.1 (common_fill {} {}, law {}): trade files {}; trade rows {n_tr}; fill rows {n_fill} (by kind {by_kind:?})", common_fill::VERSION, common_fill::RELEASE, common_fill::law_of_block(1), files.len());
    let _ = writeln!(rep, "differences: {n_diff} rows on {n_tr_diff} trades; replays refused {n_refused}; buy-limit recompute differing from the row {n_lim_diff}");
    let _ = writeln!(rep, "VERDICT: {}", if n_diff == 0 && n_refused == 0 { "PASS (0 differences in block, units, volume, price, known_at)" } else { "FAIL" });
    fs::write(out.join("F1-REPLAY.txt"), &rep).map_err(|e| e.to_string())?;
    fs::write(out.join("F1-DIFFS.tsv"), &diffs).map_err(|e| e.to_string())?;
    print!("{rep}");
    Ok(())
}
