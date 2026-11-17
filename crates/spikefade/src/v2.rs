//! v2 report pieces: v1 vs v2 evaluations of the headline configs (each config under both rule sets), the v1 picks' settings parser,
//! the 2-second index comparison (checker F2), the per-trade fill-block EVIDENCE (raw 1-minute rows + the 1-second rows of every block),
//! the expiry facts and the re-estimated (spot-marked) margin beside.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::acct::{Dec, SEv, Sum, detail_list, x_parts};
use crate::cal::{OPEN_S, day_of_ymd, fmt_day, fmt_rs, fmt_sod, fmt_us, sod_of};
use crate::cube::{Cube, SecTape};
use crate::margin::{VarRow, per_unit_index, per_unit_stock};
use crate::rules::{Calendar, wd_name};
use crate::screen::{Levels, MINPREM, NTHR, TFS, TOPN, cfg_id, thr_name, tf_name};
use crate::trade::{self, DayTape, NRULE, NSLOT, Out, SHARE_BP, STOP_PCT, TGT_PCT, TIME_MIN};

/// A booked trade: (day index, contract, T, last fill known (sod), margin, R1 key, net), paisa.
pub type Bk = (u16, u32, u32, i64, i64, u32, i64);

/// One evaluation of a config (one account) under one rule set.
#[derive(Clone, Debug, Default)]
pub struct Eval {
    pub label: String,
    pub rules: &'static str,
    pub blk: i64,
    pub events: usize,
    pub s: Sum,
    /// Peak margin in use recomputed from the booked trades' intervals [T, last fill known).
    pub pk_rows: i64,
    pub why: BTreeMap<&'static str, u64>,
    /// Booked trades as (day index, contract, T, last fill known (sod), margin, R1 key, net).
    pub booked: Vec<Bk>,
}

/// The settings words of a pick parsed back to (segment, config id, exit id, slot index). The config part must print back exactly
/// (`cfg_words`), so the levels are the same as the run that printed it.
#[must_use]
pub fn parse_settings(s: &str, lv: &[Levels], cfg_words: &dyn Fn(usize, usize, &Levels) -> String) -> Option<(usize, usize, usize, usize)> {
    let parts: Vec<&str> = s.split(" || ").collect();
    let (cfgp, exitp, np) = (parts.first()?, parts.get(1)?, parts.get(2)?);
    let f: Vec<&str> = cfgp.split(" | ").collect();
    let seg = match *f.first()? {
        "idx" => 0,
        "stk" => 1,
        _ => return None,
    };
    let tfw = f.get(1)?.strip_prefix("TF ")?;
    let tf = (0..TFS.len()).find(|&i| tf_name(i) == tfw)?;
    let mv = f.get(2)?.split("(L").nth(1)?.trim_end_matches(')').parse::<usize>().ok()?.checked_sub(1)?;
    let topn: usize = f.get(3)?.strip_prefix("top ")?.parse().ok()?;
    let tn = TOPN.iter().position(|&x| x == topn)?;
    let prem: i64 = f.get(4)?.strip_prefix("min prem Rs ")?.parse().ok()?;
    let mp = MINPREM.iter().position(|&x| x == prem * 100)?;
    let mval: usize = f.get(5)?.split("(V").nth(1)?.trim_end_matches(')').parse().ok()?;
    let thw = f.get(6)?;
    let thr = (0..NTHR).find(|&i| thr_name(i) == *thw)?;
    let cfg = cfg_id(tf, mv, tn, mp, mval, thr);
    let lvs = lv.get(seg)?;
    if cfg_words(seg, cfg, lvs) != *cfgp {
        return None;
    }
    let e: Vec<&str> = exitp.split(" | ").collect();
    let e0 = *e.first()?;
    let k = STOP_PCT.iter().position(|&x| format!("stop +{x}%") == e0)?;
    let tw = e.get(1)?;
    let j = if *tw == "target none" { 4 } else { TGT_PCT.iter().position(|&x| format!("target -{x}%") == *tw)? };
    let mw = e.get(2)?;
    let m = if *mw == "time to 15:10" { 4 } else { TIME_MIN.iter().position(|&x| format!("time {x} min") == *mw)? };
    let e3 = *e.get(3)?;
    let l = SHARE_BP.iter().position(|&x| format!("share {}%", x / 100) == e3)?;
    let n: i64 = np.strip_prefix("N ")?.split(' ').next()?.parse().ok()?;
    let ni = NSLOT.iter().position(|&x| x == n)?;
    Some((seg, cfg, ((k * 5 + j) * 5 + m) * 5 + l, ni))
}

/// One row of a v1 `picks.tsv`.
#[derive(Clone, Debug, Default)]
pub struct V1Pick {
    pub pick: String,
    pub settings: String,
    pub cols: HashMap<String, String>,
}

/// Reads a run's `picks.tsv` (v1 or v2).
///
/// # Errors
/// An unreadable file.
pub fn read_picks(dir: &Path) -> Result<Vec<V1Pick>, String> {
    let p = dir.join("picks.tsv");
    let t = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut lines = t.lines();
    let head: Vec<&str> = lines.next().unwrap_or("").split('\t').collect();
    let mut out = Vec::new();
    for l in lines {
        let f: Vec<&str> = l.split('\t').collect();
        let cols: HashMap<String, String> = head.iter().zip(f.iter()).map(|(h, v)| ((*h).to_owned(), (*v).to_owned())).collect();
        out.push(V1Pick { pick: cols.get("pick").cloned().unwrap_or_default(), settings: cols.get("settings").cloned().unwrap_or_default(), cols });
    }
    Ok(out)
}

/// Prices the given events (day index, contract, T) with blocks of `blk` seconds and runs one account (exit `x`, slot index `ni`) under
/// rule set `v2` (or v1).
///
/// # Errors
/// A crate refusal.
pub fn eval_entries(entries: &[(u16, u32, u32)], x: usize, ni: usize, ndays: u16, v2: bool, blk: i64, cubes: &[Cube], tapes: &[SecTape], var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, label: &str) -> Result<Eval, String> {
    eval_entries_full(entries, x, ni, ndays, v2, blk, trade::run_reading(), cubes, tapes, var, dial, label).map(|f| f.ev)
}

/// v2.1: one evaluation with its priced events kept (the fill-gap statistics re-walk its booked trades).
pub struct Full {
    pub ev: Eval,
    pub evs: Vec<SEv>,
    pub store: Vec<Out>,
    pub astore: Vec<[u16; NRULE]>,
    pub dec: Vec<Dec>,
}

/// v2.1: [`eval_entries`] under an explicit fill reading (the side-by-side: v2.1 prorate, anchor, v2 row), its tables kept.
///
/// # Errors
/// A crate refusal.
pub fn eval_entries_full(entries: &[(u16, u32, u32)], x: usize, ni: usize, ndays: u16, v2: bool, blk: i64, reading: common_fill::CapMode, cubes: &[Cube], tapes: &[SecTape], var: &BTreeMap<String, VarRow>, dial: &common_fill::Dial, label: &str) -> Result<Full, String> {
    let mut store: Vec<Out> = Vec::new();
    let mut astore: Vec<[u16; NRULE]> = Vec::new();
    // v2.1: priced contract by contract (one contract-day tape held at a time: an index tape with its range table is a few MB), the events
    // kept in their list order (each event's outcome offsets are its own, so the pricing order changes nothing)
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by_key(|&i| entries.get(i).map_or((0, 0, 0), |e| (e.0, e.1, e.2)));
    let mut slots: Vec<Option<SEv>> = (0..entries.len()).map(|_| None).collect();
    let mut cur: Option<((u16, u32), Option<DayTape>)> = None;
    for i in order {
        let Some(&(di, ci, t)) = entries.get(i) else { continue };
        let (Some(c), Some(tp)) = (cubes.get(usize::from(di)), tapes.get(usize::from(di))) else { continue };
        if cur.as_ref().is_none_or(|x| x.0 != (di, ci)) {
            cur = Some(((di, ci), DayTape::with_reading(c.day, tp.of(ci as usize), blk, reading).ok()));
        }
        let tape = cur.as_ref().and_then(|x| x.1.as_ref()).and_then(|d| d.tape().ok());
        let tab = trade::price_event(c, ci as usize, t, tape.as_ref(), var, dial, &mut store, &mut astore, v2)?;
        if let Some(s) = slots.get_mut(i) {
            *s = Some(SEv { di, ci, t, skey: trade::side_key(c, ci as usize), tab });
        }
    }
    let evs: Vec<SEv> = slots.into_iter().flatten().collect();
    let idx: Vec<u32> = (0..u32::try_from(evs.len()).unwrap_or(0)).collect();
    let (dec, s) = detail_list(&evs, &idx, &store, &astore, x, ni, ndays, v2);
    let ev = summarize(&evs, &store, &dec, s, x, ni, if v2 { "v2" } else { "v1" }, blk, label);
    Ok(Full { ev, evs, store, astore, dec })
}

/// The evaluation summary of a detailed run over `evs` (refusals by cause, booked trades, the interval peak of the margin in use).
#[must_use]
pub fn summarize(evs: &[SEv], store: &[Out], dec: &[Dec], s: Sum, x: usize, ni: usize, rules: &'static str, blk: i64, label: &str) -> Eval {
    let (_, _, _, l) = x_parts(x);
    let mut why: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut booked = Vec::new();
    for d in dec {
        *why.entry(d.code).or_insert(0) += 1;
        if d.code != "booked" {
            continue;
        }
        let Some(e) = evs.get(d.eid as usize) else { continue };
        let q = usize::from(e.tab.q_of.get(ni).copied().unwrap_or(255));
        let o = store.get(e.tab.idx(l, q, d.cand)).copied().unwrap_or_default();
        let known = i64::from(o.exit) + trade::BASE_S;
        booked.push((e.di, e.ci, e.t, known, i64::from(d.lots) * e.tab.margin_lot, e.skey, i64::from(o.net)));
    }
    let pk_rows = interval_peak(booked.iter().map(|b| (b.0, i64::from(b.2), b.3, b.4)));
    Eval { label: label.to_owned(), rules, blk, events: dec.len(), s, pk_rows, why, booked }
}

/// The peak of the margin in use over intervals (day, from, to, margin): a position holds its margin from T to its last fill known.
pub fn interval_peak(it: impl Iterator<Item = (u16, i64, i64, i64)>) -> i64 {
    let mut ev: Vec<(i64, i64)> = Vec::new();
    for (di, a, b, m) in it {
        let d = i64::from(di) * 86_400;
        ev.push((d + a, m));
        ev.push((d + b.max(a), -m));
    }
    ev.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.cmp(&y.1)));
    let (mut cur, mut pk) = (0_i64, 0_i64);
    for (_, m) in ev {
        cur += m;
        pk = pk.max(cur);
    }
    pk
}

/// R1 audit of booked trades: pairs of positions with the same (underlying, side, expiry) open at the same instant (must be 0 under v2).
#[must_use]
pub fn r1_overlaps(booked: &[Bk]) -> usize {
    let mut n = 0;
    for (i, a) in booked.iter().enumerate() {
        for b in booked.iter().skip(i + 1) {
            if a.0 == b.0 && a.5 == b.5 && i64::from(b.2) < a.3 && i64::from(a.2) < b.3 {
                n += 1;
            }
        }
    }
    n
}

/// The refusal causes in a fixed order (the side-by-side columns).
pub const CAUSES: [(&str, &str); 12] = [
    ("same_side_dup", "same-side duplicate (another strike of a held underlying+side+expiry)"),
    ("contract_open", "the same contract already open"),
    ("slots_full", "all N slots in use"),
    ("margin_cap_20L", "margin in use would pass Rs 20 L"),
    ("margin_cash_short", "free cash after realized losses short"),
    ("margin_over_slot", "one lot's margin above the slot"),
    ("not_filled", "the sale never filled"),
    ("after_cut_1508", "event at or after 15:08"),
    ("stock_expiry_no_next", "stock expiry day, no next-expiry data"),
    ("stock_expiry_day", "stock expiry day (v1 code)"),
    ("no_reference", "nothing traded before T"),
    ("no_margin_row", "no C_VAR1 row"),
];

/// "same_side_dup 12, slots_full 3, ..." (every cause with a count, plus any other code).
#[must_use]
pub fn why_words(w: &BTreeMap<&'static str, u64>) -> String {
    let mut v: Vec<String> = Vec::new();
    for (k, _) in CAUSES {
        if let Some(n) = w.get(k)
            && *n > 0
        {
            v.push(format!("{k} {n}"));
        }
    }
    for (k, n) in w {
        if *k != "booked" && !CAUSES.iter().any(|c| c.0 == *k) && *n > 0 {
            v.push(format!("{k} {n}"));
        }
    }
    if v.is_empty() { "none".to_owned() } else { v.join(", ") }
}

/// The expiry class of one trade (rule R3 / E1-E4 words).
#[must_use]
pub fn expiry_class(cal: &Calendar, c: &Cube, ci: usize) -> (String, i64) {
    let Some(k) = c.cons.get(ci) else { return ("?".to_owned(), -1) };
    let e = day_of_ymd(k.expiry);
    let s = cal.sessions(c.day, e);
    let nearest = c.unds.get(usize::from(k.und)).map_or(0, |u| u.nearest);
    let w = if k.kind == 0 {
        match s {
            0 => "index: ON its expiry day (allowed, E1-E4)".to_owned(),
            1 => "index: tomorrow's expiry (next regular session)".to_owned(),
            n => format!("index: {n} sessions to expiry"),
        }
    } else if k.expiry == c.ymd {
        "STOCK ON ITS OWN EXPIRY DAY (refused by R3; must never be booked)".to_owned()
    } else if k.expiry != nearest {
        format!("stock: rolled to the next expiry (own expiry day today), {s} sessions to it")
    } else {
        match s {
            1 => "stock: tomorrow's expiry (allowed: the day before)".to_owned(),
            n => format!("stock: {n} sessions to expiry"),
        }
    };
    (w, s)
}

/// Raw 1-minute rows (open, high, low, close, |volume|) of the given security ids on one staged day, by slot (read only).
///
/// # Errors
/// An unreadable staged file.
pub fn raw_1m(stage: &Path, d: &str, sids: &HashSet<u32>) -> Result<HashMap<u32, BTreeMap<usize, [i64; 5]>>, String> {
    let p = stage.join(format!("candles_1m/{d}/candles_1m-{d}.csv.gz"));
    let mut g = crate::csvgz::GzLines::open(&p)?;
    let mut f = Vec::new();
    let mut out: HashMap<u32, BTreeMap<usize, [i64; 5]>> = HashMap::new();
    let mut first = true;
    while let Some(l) = g.next_line()? {
        if first {
            first = false;
            continue;
        }
        crate::csvgz::split(l, &mut f);
        if crate::csvgz::field(l, &f, 5) != b"NSE_FNO" {
            continue;
        }
        let Some(sid) = crate::csvgz::int(crate::csvgz::field(l, &f, 6)).and_then(|x| u32::try_from(x).ok()) else { continue };
        if !sids.contains(&sid) {
            continue;
        }
        let Some((_, sod, _)) = crate::csvgz::ts(crate::csvgz::field(l, &f, 0)) else { continue };
        let Ok(slot) = usize::try_from((sod - OPEN_S) / 60) else { continue };
        let px = |k: usize| crate::csvgz::paisa(crate::csvgz::field(l, &f, k)).map_or(0, |x| x.0);
        let v = crate::csvgz::int(crate::csvgz::field(l, &f, 12)).unwrap_or(0).abs();
        out.entry(sid).or_default().entry(slot).or_insert([px(8), px(9), px(10), px(11), v]);
    }
    g.finish()?;
    Ok(out)
}

/// One evidence trade's inputs.
pub struct EvIn<'a> {
    pub pick: &'a str,
    pub no: usize,
    pub di: u16,
    pub ci: u32,
    pub t: u32,
    pub tf: usize,
    pub thr: usize,
    pub gain_bp: i64,
    pub wlots: u64,
    pub lots: u16,
    pub l: usize,
    pub k: usize,
    pub j: usize,
    pub m: usize,
    pub x: Option<i64>,
    pub kind: &'a str,
    pub margin_unit: i64,
    pub margin_lot: i64,
    pub s: i64,
    pub margin_in_use: i64,
    pub nopen: usize,
    pub trip: &'a trade::Trip,
    pub cls: String,
}

fn rows_in(sec: &[crate::cube::Sec], a: i64, b: i64) -> Vec<&crate::cube::Sec> {
    let i0 = sec.partition_point(|s| i64::from(s.sod) < a);
    sec.get(i0..).unwrap_or(&[]).iter().take_while(|s| i64::from(s.sod) < b).collect()
}

/// v2.1 (checker F-A): a block's usable volume in words: the whole number, or the exact fraction and its value to the paisa.
#[must_use]
pub fn usable_words(b: &common_fill::Block) -> String {
    if b.cap_den == 0 {
        format!("{}", b.vol)
    } else if b.cap_num % b.cap_den == 0 {
        format!("{}", b.cap_num / b.cap_den)
    } else {
        format!("{}/{} = {}.{:02}", b.cap_num, b.cap_den, b.cap_num / b.cap_den, (b.cap_num % b.cap_den) * 100 / b.cap_den)
    }
}

fn blocks_md(out: &mut String, title: &str, bl: &[common_fill::BlockFill], sec: &[crate::cube::Sec], buy: bool, share_pct: u16, lot: u64, fa: Option<(&[i64], i64)>) {
    let _ = writeln!(out, "{title}");
    let _ = writeln!(out);
    if bl.is_empty() {
        let _ = writeln!(out, "(no block read)");
        let _ = writeln!(out);
        return;
    }
    if let Some((prev, anchor_sod)) = fa {
        // v2.1 (checker F-A): every row with its previous print and its silence g; its usable part n / g (n = the silence's seconds inside
        // the block and at or after the order's send); the block's usable volume (an exact fraction, floored once at the share)
        let _ = writeln!(out, "| block | its 1-second rows (time H / L / V units; previous print; silence g s; usable n/g of V) | block {} | recorded volume | USABLE volume (v2.1) | {share_pct}% of usable in whole lots of {lot} | units taken | cumulative | why |", if buy { "highest HIGH (paid)" } else { "lowest LOW (received)" });
        let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|");
        let show = 40_usize;
        for b in bl.iter().take(show) {
            let a = sod_of(b.block.at);
            let z = sod_of(b.block.to);
            let i0 = sec.partition_point(|s| i64::from(s.sod) < a);
            let rs: Vec<String> = sec
                .get(i0..)
                .unwrap_or(&[])
                .iter()
                .enumerate()
                .take_while(|(_, s)| i64::from(s.sod) < z)
                .map(|(k, s)| {
                    let ts = i64::from(s.sod);
                    let p = prev.get(i0 + k).copied().map_or(ts - 1, sod_of);
                    let g = ts - p;
                    let from = a.max(anchor_sod);
                    let n = (ts - p.max(from - 1)).clamp(0, g);
                    format!("{} {}/{}/{} (prev {}; g {g}; usable {n}/{g})", fmt_sod(ts), fmt_rs(i64::from(s.high)), fmt_rs(i64::from(s.low)), s.vol, fmt_sod(p))
                })
                .collect();
            let cap = if b.block.cap_den == 0 { (b.block.vol * u64::from(share_pct) / 100) / lot.max(1) * lot.max(1) } else { u64::try_from(b.block.cap_num * u128::from(share_pct) / (b.block.cap_den * 100)).unwrap_or(0) / lot.max(1) * lot.max(1) };
            let px = if buy { b.block.high } else { b.block.low };
            let _ = writeln!(out, "| {}–{}{} | {} | {} | {} | {} | {cap} | {} | {} | {} |", fmt_sod(a), fmt_sod(z), if b.block.clipped { " (clipped by the stop)" } else { "" }, if rs.is_empty() { "-".to_owned() } else { rs.join("; ") }, fmt_rs(px), b.block.vol, usable_words(&b.block), b.units, b.cum, b.why.code());
        }
        if bl.len() > show {
            let rest: u64 = bl.iter().skip(show).map(|b| b.units).sum();
            let _ = writeln!(out, "| … {} more blocks | | | | | | {rest} more units | {} | |", bl.len() - show, bl.last().map_or(0, |b| b.cum));
        }
        let _ = writeln!(out);
        return;
    }
    let _ = writeln!(out, "| block | its 1-second rows (time H / L / V units) | block {} | block volume | {share_pct}% of it in whole lots of {lot} | units taken | cumulative | why |", if buy { "highest HIGH (paid)" } else { "lowest LOW (received)" });
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|");
    let show = 40_usize;
    for b in bl.iter().take(show) {
        let a = sod_of(b.block.at);
        let z = sod_of(b.block.to);
        let rs: Vec<String> = rows_in(sec, a, z).iter().map(|s| format!("{} {}/{}/{}", fmt_sod(i64::from(s.sod)), fmt_rs(i64::from(s.high)), fmt_rs(i64::from(s.low)), s.vol)).collect();
        let cap = (b.block.vol * u64::from(share_pct) / 100) / lot.max(1) * lot.max(1);
        let px = if buy { b.block.high } else { b.block.low };
        let _ = writeln!(out, "| {}–{}{} | {} | {} | {} | {} | {} | {} | {} |", fmt_sod(a), fmt_sod(z), if b.block.clipped { " (clipped by the stop)" } else { "" }, if rs.is_empty() { "-".to_owned() } else { rs.join("; ") }, fmt_rs(px), b.block.vol, cap, b.units, b.cum, b.why.code());
    }
    if bl.len() > show {
        let rest: u64 = bl.iter().skip(show).map(|b| b.units).sum();
        let _ = writeln!(out, "| … {} more blocks | | | | | {rest} more units | {} | |", bl.len() - show, bl.last().map_or(0, |b| b.cum));
    }
    let _ = writeln!(out);
}

/// One trade's evidence (markdown) and whether the evidence walk's totals equal the engine's trade.
///
/// # Errors
/// A crate refusal.
pub fn evidence_md(e: &EvIn<'_>, c: &Cube, sec: &[crate::cube::Sec], raw: Option<&BTreeMap<usize, [i64; 5]>>, blk: i64, dial: &common_fill::Dial) -> Result<(String, bool), String> {
    let mut out = String::new();
    let Some(kc) = c.cons.get(e.ci as usize) else { return Ok((out, false)) };
    let und = c.unds.get(usize::from(kc.und)).map_or("?", |u| u.name.as_str());
    let lot = u64::from(kc.lot);
    let qty = u64::from(e.lots) * lot;
    let dt = DayTape::new(c.day, sec, blk)?;
    let tape = dt.tape()?;
    let ev = trade::evidence(&tape, c.day, e.t, lot, qty, e.l, e.x, dial)?;
    let tr = e.trip;
    let _ = writeln!(out, "#### {} trade {}: {} ({} {} strike Rs {}, lot {}), {} T {}", e.pick, e.no, kc.symbol, und, if kc.right == 0 { "CE" } else { "PE" }, fmt_rs(kc.strike), lot, fmt_day(c.day), fmt_sod(i64::from(e.t)));
    let _ = writeln!(out);
    let ex = day_of_ymd(kc.expiry);
    let _ = writeln!(out, "- **Expiry** {} ({}) from the day's instrument snapshot; {}.", fmt_day(ex), wd_name(ex), e.cls);
    // signal window
    let tfm = TFS.get(e.tf).copied().unwrap_or(1) as usize;
    let b = usize::try_from((i64::from(e.t) - OPEN_S) / 60).unwrap_or(0);
    let a = if tfm == 0 { 0 } else { b.saturating_sub(tfm) };
    let ct = c.close_at(e.ci as usize, b.saturating_sub(1));
    let start = if tfm != 0 && a > 0 && c.close_at(e.ci as usize, a - 1) > 0 { (c.close_at(e.ci as usize, a - 1), format!("the 1-minute close at {}", fmt_sod(OPEN_S + i64::try_from(a).unwrap_or(1) * 60 - 60))) } else { (kc.first_open, "the contract's first open of the day".to_owned()) };
    let _ = writeln!(out, "- **Signal** TF {} ({}), window {}–{} (decision T = the candle END, {}); start reference Rs {} ({}); close at T Rs {}; window gain {:.2}% (engine); window volume {} lots ({} units).", tf_name(e.tf), thr_name(e.thr), fmt_sod(OPEN_S + i64::try_from(a).unwrap_or(0) * 60), fmt_sod(i64::from(e.t)), fmt_sod(i64::from(e.t)), fmt_rs(i64::from(start.0)), start.1, fmt_rs(i64::from(ct)), e.gain_bp as f64 / 100.0, e.wlots, e.wlots * lot);
    let _ = writeln!(out);
    let _ = writeln!(out, "1-minute OHLCV of the signal window (raw staged `candles_1m` rows; volume = |volume| units):");
    let _ = writeln!(out);
    let _ = writeln!(out, "| minute | open | high | low | close | volume |");
    let _ = writeln!(out, "|---|---|---|---|---|---|");
    let slots: Vec<usize> = (a..b).collect();
    let pr = |s: usize, o: &mut String| {
        let r = raw.and_then(|m| m.get(&s));
        let mn = fmt_sod(OPEN_S + i64::try_from(s).unwrap_or(0) * 60);
        match r {
            Some(x) => {
                let _ = writeln!(o, "| {mn} | {} | {} | {} | {} | {} |", fmt_rs(x[0]), fmt_rs(x[1]), fmt_rs(x[2]), fmt_rs(x[3]), x[4]);
            }
            None => {
                let _ = writeln!(o, "| {mn} | no bar | | | (close carried {}) | 0 |", fmt_rs(i64::from(c.close_at(e.ci as usize, s))));
            }
        }
    };
    if slots.len() <= 40 {
        for &s in &slots {
            pr(s, &mut out);
        }
    } else {
        for &s in slots.iter().take(5) {
            pr(s, &mut out);
        }
        let _ = writeln!(out, "| … {} minutes | | | | | |", slots.len() - 25);
        for &s in slots.iter().skip(slots.len() - 20) {
            pr(s, &mut out);
        }
    }
    let _ = writeln!(out);
    let share = SHARE_BP.get(e.l).copied().unwrap_or(0) / 100;
    let Some(ev) = ev else {
        let _ = writeln!(out, "No traded second before T: no sale (not booked).");
        return Ok((out, false));
    };
    let _ = writeln!(out, "- **Sale** SELL {} lots x {} = {} units, from T in {}-second blocks; Dhan protection limit Rs {} (10% below the last traded second's LOW before T, 5-paisa tick); each block pays its LOWEST LOW; {}% of each block's volume in whole lots, the rest to the next block{}.", e.lots, lot, qty, blk, fmt_rs(ev.lim_in), share, e.x.map_or(String::new(), |x| format!("; the sale stops at the exit anchor {}", fmt_sod(x))));
    let _ = writeln!(out);
    // v2.1: under a capacity reading every block shows each row's previous print, silence and usable part (the order's send in seconds)
    let fa = |anchor_sod: i64| if dt.mode() == common_fill::CapMode::Row { None } else { Some((dt.prev(), anchor_sod)) };
    if dt.mode() != common_fill::CapMode::Row {
        let _ = writeln!(out, "- **Fill reading** {} (checker F-A): a 1-second row printed after a silence of g seconds carries everything traded since the previous print; only its share n/g counts, n = the silence's seconds inside the block and at or after the order's send; the usable volume is an exact fraction, floored once at the share. The block's price is its recorded print(s), not every trade inside it.", trade::reading_name(dt.mode()));
        let _ = writeln!(out);
    }
    blocks_md(&mut out, "Sale blocks:", &ev.sale, sec, false, share, lot, fa(i64::from(e.t)));
    let stop_lvl = (tr.p0 * (100 + STOP_PCT.get(e.k).copied().unwrap_or(0)) + 99) / 100;
    let _ = writeln!(
        out,
        "- **Exit** {} at {}: stop +{}% (level Rs {} vs the first fill Rs {}, above R(s)); target {}; time exit {}; no own order at or after 15:08, square-off at 15:10 (intraday only).",
        e.kind,
        e.x.map_or("15:10:00".to_owned(), fmt_sod),
        STOP_PCT.get(e.k).copied().unwrap_or(0),
        fmt_rs(stop_lvl),
        fmt_rs(tr.p0),
        TGT_PCT.get(e.j).map_or("none".to_owned(), |g| format!("-{g}% (level Rs {} vs the first fill)", fmt_rs(tr.p0 - tr.p0 * g / 100))),
        TIME_MIN.get(e.m).map_or("none".to_owned(), |t| format!("{t} min after E0 {}", fmt_us(tr.e0)))
    );
    let _ = writeln!(out);
    if e.x.is_some() {
        blocks_md(&mut out, &format!("Buy-back blocks (BUY from the exit anchor, protection limit Rs {}; each block pays its HIGHEST HIGH):", ev.lim_out.map_or("none".to_owned(), fmt_rs)), &ev.bb, sec, true, share, lot, fa(e.x.unwrap_or(crate::cal::SQ_S)));
    }
    if !ev.sq.is_empty() || ev.units > ev.bb_units {
        blocks_md(&mut out, "Square-off blocks (BUY from 15:10:00 to the session end, no limit; each block pays its HIGHEST HIGH):", &ev.sq, sec, true, share, lot, fa(crate::cal::SQ_S));
    }
    let ok = ev.units == tr.units && ev.sold == tr.sold_value && ev.bb_units == tr.bb_units && ev.bb_value == tr.bb_value && ev.sq_units == tr.sq_units && ev.sq_value == tr.sq_value;
    let _ = writeln!(out, "- **Money** units sold {} for Rs {} (average Rs {}); bought back {} units for Rs {} + square-off {} units for Rs {}{}; charges Rs {} (sale side Rs {}, buy side Rs {}); **net Rs {}**.", tr.units, fmt_rs(tr.sold_value), fmt_rs(if tr.units > 0 { tr.sold_value / i64::try_from(tr.units).unwrap_or(1) } else { 0 }), tr.bb_units, fmt_rs(tr.bb_value), tr.sq_units, fmt_rs(tr.sq_value), if tr.unclosed > 0 { format!(" + {} unclosed units valued at Rs {}", tr.unclosed, fmt_rs(tr.worst)) } else { String::new() }, fmt_rs(tr.charges), fmt_rs(tr.charges_sell), fmt_rs(tr.charges_buy), fmt_rs(tr.net));
    let _ = writeln!(out, "- **Margin** S Rs {} (the underlying's last 1-minute close at or before T); per unit Rs {}, per lot Rs {}; {} lots -> Rs {} blocked; margin in use right after this booking Rs {} over {} open positions (cap Rs 20,00,000).", fmt_rs(e.s), fmt_rs(e.margin_unit), fmt_rs(e.margin_lot), e.lots, fmt_rs(i64::from(e.lots) * e.margin_lot), fmt_rs(e.margin_in_use), e.nopen);
    let _ = writeln!(out, "- **Check** the evidence walk's units and values equal the engine's trade: {}.", if ok { "yes" } else { "NO (see the trade row)" });
    let _ = writeln!(out);
    Ok((out, ok))
}

/// The re-estimated margin beside the cap: every booked position's margin re-estimated at every minute END with the live underlying
/// (that minute's close), the position counted while T <= the instant < its last fill known (units = the units sold). At its own T a
/// position's estimate equals its entry estimate. Returns (peak, minute ends above Rs 20 L, the day and instant of the peak).
#[must_use]
pub fn remark_peak(rows: &[(u16, u32, u32, i64, u64)], cubes: &[Cube], var: &BTreeMap<String, VarRow>) -> (i64, u64, String) {
    let mut acc: BTreeMap<(u16, usize), i64> = BTreeMap::new();
    for &(di, ci, t, known, units) in rows {
        let Some(c) = cubes.get(usize::from(di)) else { continue };
        let Some(k) = c.cons.get(ci as usize) else { continue };
        let b0 = usize::try_from((i64::from(t) - OPEN_S) / 60).unwrap_or(0);
        let name = c.unds.get(usize::from(k.und)).map_or("", |u| u.name.as_str());
        for s in b0.saturating_sub(1)..crate::cal::NM {
            // the instant = the END of minute slot s (s = b0 - 1 is the decision instant T itself)
            let tau = OPEN_S + (i64::try_from(s).unwrap_or(0) + 1) * 60;
            if tau >= known {
                break;
            }
            let sp = i64::from(c.spot_at(usize::from(k.und), s));
            if sp <= 0 {
                continue;
            }
            let mu = if k.kind == 0 { per_unit_index(sp, k.strike, k.right == 0, k.expiry == c.ymd).total } else { var.get(name).map_or(0, |v| per_unit_stock(sp, k.strike, k.right == 0, v).total) };
            *acc.entry((di, s)).or_insert(0) += mu * i64::try_from(units).unwrap_or(0);
        }
    }
    let mut pk = 0_i64;
    let mut at = String::new();
    let mut over = 0_u64;
    for (&(di, s), &v) in &acc {
        if v > trade::ACCOUNT {
            over += 1;
        }
        if v > pk {
            pk = v;
            at = format!("{} {}", cubes.get(usize::from(di)).map_or("?".to_owned(), |c| fmt_day(c.day)), fmt_sod(OPEN_S + (i64::try_from(s).unwrap_or(0) + 1) * 60));
        }
    }
    (pk, over, at)
}

/// Moves an existing file aside (`.old-<epoch>`), never deleting, then writes the new content.
///
/// # Errors
/// A file system refusal.
pub fn write_new(p: &Path, content: &str) -> Result<(), String> {
    if p.exists() {
        let ep = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let mut q = PathBuf::from(p);
        q.set_extension(format!("md.old-{ep}"));
        std::fs::rename(p, &q).map_err(|e| format!("move aside {}: {e}", p.display()))?;
    }
    std::fs::write(p, content).map_err(|e| format!("{}: {e}", p.display()))
}
