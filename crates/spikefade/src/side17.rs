//! v3.2-B (MAIN-RULINGS-ENGINE-v22 Addendum 17; job v32-par, agent build:v32-B-par, Addendum 21.1): EVERY INDEX AS ITS OWN ACCOUNT and the
//! SIDE-BY-SIDE account.
//!
//! 1. `--idx-und U[,U...]`: the INDEX lists of the run hold only the named index underlyings' contracts (every list of the segment and
//!    expiry, all strikes, CE and PE, ranked inside the kept contracts only), so `--idx-und FINNIFTY` is FINNIFTY's own account,
//!    `--idx-und NIFTY` NIFTY alone, and no flag = every index the cubes hold together (the combined index account). Stock lists are never
//!    touched; index and stock are never mixed in one search (the segments stay separate processes or separate sweeps). Every index
//!    underlying the run's cubes hold is printed with its sessions (first .. last, count) and whether this run kept it: a month the cubes
//!    do not hold is NAMED, never filled.
//! 2. The SIDE-BY-SIDE account (SIDE17): the verdict view's best combos of the INDEX sweep and of the STOCK sweep (distinct trade lists, at
//!    most 4 each) offered together to ONE Rs 20 L account: each combo's own decisions as walked alone, then the one account's rules
//!    across all of them, in the kernel's order: the same contract or another strike of the same underlying + side + expiry open
//!    (refused, ruled), the margin in use + this margin > Rs 20 L (refused, cap), the free money (Rs 20 L + min(0, realized) - margin in
//!    use) less the RUNNING MTM LOSS of every open position at the sale's decision second (minute-close marks of each position's own
//!    segment, losses only) below this margin (refused: `margin_mtm_short` when the free money alone would cover it, else the cash rule).
//!    Greedy by the verdict order (months >= Rs 2 L, the worst month, the smaller max DD, the bigger net) up to 5 members; its margin
//!    utilization (peak, median, minutes above 80% of Rs 20 L, idle) and the peak of margin in use + the running MTM loss at its sale
//!    instants are printed. HINDSIGHT (chosen on the same period); printed BESIDE the separate index and stock accounts, never instead.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::acct::Sum;
use crate::cal::fmt_rs;
use crate::cube::Cube;

#[cfg(not(test))]
static IDX_UND: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
#[cfg(test)]
thread_local! {
    static IDX_UND: std::cell::RefCell<Option<Vec<String>>> = const { std::cell::RefCell::new(None) };
}

/// The run's index-underlying filter (None = every index underlying).
#[must_use]
pub fn idx_und() -> Option<Vec<String>> {
    #[cfg(not(test))]
    {
        IDX_UND.get().cloned()
    }
    #[cfg(test)]
    {
        IDX_UND.with(|x| x.borrow().clone())
    }
}

/// Sets the filter (once per run; a test thread sets its own).
pub fn set_idx_und(v: Vec<String>) {
    #[cfg(not(test))]
    {
        let _ = IDX_UND.set(v);
    }
    #[cfg(test)]
    {
        IDX_UND.with(|x| *x.borrow_mut() = Some(v));
    }
}

/// Reads `--idx-und U[,U...]` (upper-case NSE underlying names, e.g. NIFTY, BANKNIFTY, FINNIFTY, MIDCPNIFTY, NIFTYNXT50).
///
/// # Errors
/// An empty or malformed list.
pub fn from_args(a: &[String]) -> Result<Option<String>, String> {
    let Some(v) = a.iter().position(|x| x == "--idx-und").and_then(|i| a.get(i + 1)) else { return Ok(None) };
    let names: Vec<String> = v.split(',').map(|x| x.trim().to_owned()).filter(|x| !x.is_empty()).collect();
    if names.is_empty() || names.iter().any(|x| !x.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '&' || c == '-')) {
        return Err(format!("--idx-und U[,U...] (NSE index underlying names), not {v}"));
    }
    set_idx_und(names.clone());
    Ok(Some(format!("INDEX lists kept to the index underlyings {} (addendum 17: each index its own account; stock lists untouched)", names.join(", "))))
}

/// Keeps every INDEX list of one day to the run's index underlyings (lists left empty are dropped); stock lists and an unset filter
/// return the lists unchanged.
#[must_use]
pub fn keep_unds(c: &Cube, seg: u8, v: Vec<(u32, Vec<u32>)>) -> Vec<(u32, Vec<u32>)> {
    if seg != 0 || cfg!(feature = "m_v32b_idx_und_ignored") {
        // MUTANT m_v32b_idx_und_ignored: the filter never applied
        return v;
    }
    let Some(keep) = idx_und() else { return v };
    let ok = |ci: u32| -> bool {
        c.cons.get(ci as usize).and_then(|k| c.unds.get(usize::from(k.und))).is_some_and(|u| u.kind == 0 && keep.iter().any(|n| *n == u.name))
    };
    v.into_iter()
        .filter_map(|(e, l)| {
            let l2: Vec<u32> = l.into_iter().filter(|&ci| ok(ci)).collect();
            (!l2.is_empty()).then_some((e, l2))
        })
        .collect()
}

/// Per index underlying of the run's cubes: the sessions (day numbers) holding its option contracts.
pub static IDX_DAYS: std::sync::Mutex<BTreeMap<String, Vec<i64>>> = std::sync::Mutex::new(BTreeMap::new());

/// v3.2 round B (addendum 17: "never fill a gap silently"): per index underlying the sessions whose spot came from ANOTHER source than GDFL
/// 1-second rows (the extra-index cube tool writes `spot_seg` 2 = Zerodha 1-minute candles when GDFL holds no spot month; 255 = no spot at
/// all: every contract refused NoUnderlying), counted and printed per underlying.
pub static IDX_SPOT_SRC: std::sync::Mutex<BTreeMap<(String, u8), Vec<i64>>> = std::sync::Mutex::new(BTreeMap::new());

/// Notes one cube's index underlyings (called once per day and segment 0 by the day preparation).
pub fn note_day(c: &Cube, seg: u8) {
    if seg != 0 {
        return;
    }
    let mut names: Vec<&str> = c.cons.iter().filter(|k| k.kind == 0).filter_map(|k| c.unds.get(usize::from(k.und))).filter(|u| u.kind == 0).map(|u| u.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    if let Ok(mut g) = IDX_DAYS.lock() {
        for n in names {
            let v = g.entry(n.to_owned()).or_default();
            if !v.contains(&c.day) {
                v.push(c.day);
            }
        }
    }
    if let Ok(mut g) = IDX_SPOT_SRC.lock() {
        for u in c.unds.iter().filter(|u| u.kind == 0 && u.spot_seg != 0) {
            let v = g.entry((u.name.clone(), u.spot_seg)).or_default();
            if !v.contains(&c.day) {
                v.push(c.day);
            }
        }
    }
}

/// The words of the index underlyings the run's cubes hold (each with its sessions) and of the filter.
#[must_use]
pub fn idx_words() -> String {
    let g = IDX_DAYS.lock().map(|g| g.clone()).unwrap_or_default();
    let keep = idx_und();
    let mut v: Vec<String> = g
        .iter()
        .map(|(n, d)| {
            let mut d = d.clone();
            d.sort_unstable();
            let kept = keep.as_ref().is_none_or(|k| k.iter().any(|x| x == n));
            format!("{n}: {} sessions ({} .. {}){}", d.len(), d.first().map_or("-".to_owned(), |&x| crate::cal::fmt_day(x)), d.last().map_or("-".to_owned(), |&x| crate::cal::fmt_day(x)), if kept { "" } else { ", NOT in this run's lists (--idx-und)" })
        })
        .collect();
    if let Some(k) = &keep {
        for n in k.iter().filter(|n| !g.contains_key(*n)) {
            v.push(format!("{n}: 0 sessions in the run's cubes (NOT HELD: no account; never filled)"));
        }
    }
    let src = IDX_SPOT_SRC.lock().map(|g| g.clone()).unwrap_or_default();
    let sw: Vec<String> = src.iter().map(|((n, s), d)| format!("{n}: {} sessions with the spot from {}", d.len(), match s { 2 => "Zerodha 1-minute candles (GDFL holds no 1-s spot that month; labelled, the same per-minute series the cube keeps)", 255 => "NO source (every contract refused NoUnderlying, counted)", _ => "another source" })).collect();
    let spot = if sw.is_empty() { "every index session's spot from GDFL 1-second rows".to_owned() } else { format!("spot sources: {}", sw.join("; ")) };
    format!("index underlyings in the run's cubes: {}; index lists {}", if v.is_empty() { "none".to_owned() } else { v.join("; ") }, keep.map_or_else(|| "hold every index underlying together (the combined index account)".to_owned(), |k| format!("kept to {} (its own account)", k.join(", ")))) + &format!("; {spot}")
}

/// One side-by-side member trade: the s26 trade (entry us, known us, margin, net, day, contract key, R1 key), its segment (0 index,
/// 1 stock), the decision second of the day, and the MTM reading (units sold, value sold, 1 + the offset of its minute closes in its
/// segment's marks; 0 = none).
#[derive(Clone, Copy, Debug, Default)]
pub struct STrade {
    pub p: crate::v32::PTrade,
    pub seg: u8,
    pub sod: u32,
    pub units: u32,
    pub sold: i64,
    pub mark1: u32,
}

impl STrade {
    /// v3.2 round B: the trade as the ONE common walk reads it.
    #[must_use]
    pub fn w(&self) -> crate::v32::WTrade {
        crate::v32::WTrade { p: self.p, seg: self.seg, pm: crate::v32::PMark { units: u64::from(self.units), sold: self.sold, mark1: self.mark1 } }
    }
}

/// The side-by-side walk's outcome.
#[derive(Clone, Debug, Default)]
pub struct SideRes {
    pub net: i64,
    pub trades: u32,
    pub by_seg: [u32; 2],
    /// Refused: the Rs 20 L cap, the cash rule, the running MTM loss alone, the one-account rules.
    pub cap: u32,
    pub cash: u32,
    pub mtm: u32,
    pub ruled: u32,
    pub dd: i64,
    pub mo: BTreeMap<i64, i64>,
    /// (entry us, known us, margin) of the trades taken (the utilization reads them).
    pub taken: Vec<(i64, i64, i64)>,
    /// The largest margin in use + running MTM loss seen at a sale instant (after the sale).
    pub peak_mtm: i64,
    /// v3.2 round B: every member trade's outcome in the walk's order (member, trade index, outcome word).
    pub log: Vec<(usize, usize, &'static str)>,
}

/// The running MTM loss of one short position at decision second `sod` (the kernel's `unrealised` reading: the option's last 1-minute
/// close at or before the decision, losses only).
#[must_use]
pub fn loss_at(marks: &[i32], mark1: u32, units: u32, sold: i64, sod: u32) -> i64 {
    if mark1 == 0 {
        return 0;
    }
    let b = (i64::from(sod) - crate::cal::OPEN_S) / 60;
    if b < 1 {
        return 0;
    }
    let b = usize::try_from(b - 1).unwrap_or(0).min(crate::cal::NM - 1);
    let mark = marks.get(mark1 as usize - 1 + b).copied().unwrap_or(0);
    if mark > 0 { (i64::from(mark) * i64::from(units) - sold).max(0) } else { 0 }
}

/// The side-by-side walk of the members' trades under ONE Rs 20 L account (the kernel's rules in its order; MTM from each position's own
/// segment marks). Same entry instant: the trade known first, then the index segment, then the member order.
#[must_use]
pub fn side_walk(members: &[&[STrade]], marks: [&[i32]; 2], months_all: &[i64]) -> SideRes {
    // v3.2 round B (ONE common runtime): the very walk of the s26 portfolio (`v32::port_walk`), each position marked from its own segment
    let w: Vec<Vec<crate::v32::WTrade>> = members.iter().map(|m| m.iter().map(|t| t.w()).collect()).collect();
    let refs: Vec<&[crate::v32::WTrade]> = w.iter().map(Vec::as_slice).collect();
    let r = crate::v32::port_walk(&refs, &marks, months_all);
    SideRes { net: r.net, trades: r.trades, by_seg: r.by_seg, cap: r.cap, cash: r.cash, mtm: r.mtm, ruled: r.ruled, dd: r.dd, mo: r.mo, taken: r.taken.iter().map(|x| (x.0, x.1, x.2)).collect(), peak_mtm: r.peak_mtm, log: r.log }
}

/// One account's booked trades with their MTM reading (the very same `detail3` steps the s26 layer reads; each trade's net and exit from
/// the sweep's outcome table).
#[must_use]
pub fn side_trades(sr: &crate::report3::Seg3, sm: &Sum, csets: &[crate::kern::CSet], dd: &[i64], seg: u8) -> Vec<STrade> {
    let nl = crate::trade::NL;
    let (si, l, ni) = (usize::from(sm.x) / nl, usize::from(sm.x) % nl, usize::from(sm.n));
    let (Some(list), Some(cs)) = (sr.ul(sm.list as usize), csets.get(si)) else { return Vec::new() };
    let tf = sr.uls.get(sm.list as usize).and_then(|x| if x.2 == u8::MAX { None } else { Some(x.2) });
    let (dec, _, _) = crate::kern::detail3(&sr.evs, &sr.ex, &sr.tab, &list, cs, l, ni, tf, true, u16::try_from(sr.days.len()).unwrap_or(0));
    dec.iter()
        .filter(|d| d.code == "booked")
        .filter_map(|d| {
            let e = sr.evs.get(d.eid as usize)?;
            let x = sr.ex.get(d.eid as usize)?;
            let o = crate::kern::out_of_decision(e, x, &sr.tab, cs, l, ni, tf)?;
            let day = *dd.get(usize::from(e.di))?;
            let dk = u64::try_from(day).unwrap_or(0) << 32;
            let p = (crate::cal::at_sod(day, i64::from(e.t)), crate::cal::at_sod(day, i64::from(o.exit) + crate::trade::BASE_S), i64::from(d.lots) * e.tab.margin_lot + e.tab.addon, i64::from(o.net), day, dk | u64::from(e.ci), dk | u64::from(e.skey));
            Some(STrade { p, seg, sod: e.t, units: o.units, sold: o.sold, mark1: x.mark1 })
        })
        .collect()
}

/// v3.2 round B: one side-by-side candidate: a segment's combo with its trades as walked alone. `kind` V = the verdict view's best (chosen
/// on the whole period: HINDSIGHT), B = a split's way B pick (chosen on that split's learning months only).
#[derive(Clone, Debug, Default)]
pub struct Cand {
    pub kind: char,
    pub words: String,
    pub net: i64,
    pub trades: Vec<STrade>,
}

/// v3.2 round B: one segment's side-by-side candidates, self-contained (the minute closes its trades read are copied into its own arena,
/// `trades[].mark1` = 1 + the offset there), so the SAME combine runs inside one run and across two separate runs (index and stock are
/// separate processes in the all-years plan): `side17-cands-<seg>.tsv` per run, `spikefade side17-combine`.
#[derive(Clone, Debug, Default)]
pub struct CandSet {
    pub seg: u8,
    /// The run's sessions (day numbers).
    pub days: Vec<i64>,
    pub cands: Vec<Cand>,
    pub marks: Vec<i32>,
    /// Per split with a way B pick: (split id, its first test month id, the candidate index).
    pub wf: Vec<(String, i64, usize)>,
}

impl CandSet {
    /// Adds a candidate (an identical trade list already held is reused: its index), copying the minute closes of its trades.
    fn add(&mut self, kind: char, words: String, net: i64, mut trades: Vec<STrade>, tab_marks: &[i32], moved: &mut BTreeMap<u32, u32>) -> usize {
        let sig = |v: &[STrade]| v.iter().map(|y| (y.p.0, y.p.1, y.p.3, y.p.5)).collect::<Vec<_>>();
        if let Some(i) = self.cands.iter().position(|c| sig(&c.trades) == sig(&trades)) {
            return i;
        }
        for t in &mut trades {
            if t.mark1 == 0 {
                continue;
            }
            let m1 = *moved.entry(t.mark1).or_insert_with(|| {
                let a = t.mark1 as usize - 1;
                let at = u32::try_from(self.marks.len() + 1).unwrap_or(0);
                self.marks.extend_from_slice(tab_marks.get(a..a + crate::cal::NM).unwrap_or(&[0; crate::cal::NM]));
                at
            });
            t.mark1 = m1;
        }
        self.cands.push(Cand { kind, words, net, trades });
        self.cands.len() - 1
    }

    /// The candidates of one segment of a run: the verdict view's best (at most 4 distinct trade lists) and every split's way B pick.
    #[must_use]
    pub fn of_run(sr: &crate::report3::Seg3, seg: u8, csets: &[crate::kern::CSet], dd: &[i64], words: &dyn Fn(usize, &Sum) -> String) -> Self {
        // the RUN's sessions (a segment may hold no contract on a session, e.g. the stock segment on a Muhurat day: the account still
        // counts that session, net Rs 0), so both segments of one run, and two runs of the same days, carry the same sessions
        let mut cs = Self { seg, days: dd.to_vec(), ..Self::default() };
        let mut moved: BTreeMap<u32, u32> = BTreeMap::new();
        if let Some(tops) = sr.tops.get(6) {
            for s in tops.best(crate::acct::VIEW_V).iter().filter(|s| s.trades > 0) {
                if cs.cands.len() >= 4 {
                    break;
                }
                let tr = side_trades(sr, s, csets, dd, seg);
                let n0 = cs.cands.len();
                let _ = cs.add('V', words(usize::from(seg), s), s.net, tr, &sr.tab.marks, &mut moved);
                let _ = n0;
            }
        }
        if let Some(c) = crate::split18::cal() {
            for (si, sp) in sr.sp18.splits.iter().enumerate() {
                let Some(b) = sr.sp18.best.get(si).and_then(|v| v.first()).copied().flatten() else { continue };
                let Some(&((_, _), (ta, _))) = sp.folds.first() else { continue };
                let tr = side_trades(sr, &b.sm, csets, dd, seg);
                let i = cs.add('B', words(usize::from(seg), &b.sm), b.sm.net, tr, &sr.tab.marks, &mut moved);
                cs.wf.push((sp.id.clone(), c.month_id.get(ta).copied().unwrap_or(0), i));
            }
        }
        cs
    }

    /// The candidates file (one segment of a run).
    #[must_use]
    pub fn to_tsv(&self) -> String {
        let mut s = format!("SIDE17-CANDS\tv1\tseg\t{}\nD\t{}\n", self.seg, self.days.iter().map(i64::to_string).collect::<Vec<_>>().join(","));
        for (i, c) in self.cands.iter().enumerate() {
            let _ = writeln!(s, "C\t{i}\t{}\t{}\t{}", c.kind, c.net, c.words.replace(['\t', '\n'], " "));
            for t in &c.trades {
                let _ = writeln!(s, "T\t{i}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", t.p.0, t.p.1, t.p.2, t.p.3, t.p.4, t.p.5, t.p.6, t.sod, t.units, t.sold, t.mark1);
            }
        }
        for (id, m0, i) in &self.wf {
            let _ = writeln!(s, "S\t{id}\t{m0}\t{i}");
        }
        for (k, ch) in self.marks.chunks(crate::cal::NM).enumerate() {
            let _ = writeln!(s, "M\t{}\t{}", k * crate::cal::NM + 1, ch.iter().map(i32::to_string).collect::<Vec<_>>().join(","));
        }
        s
    }

    /// The inverse of [`CandSet::to_tsv`].
    ///
    /// # Errors
    /// A malformed file.
    pub fn of_tsv(text: &str) -> Result<Self, String> {
        let mut cs = Self::default();
        let bad = |w: &str| format!("side17 candidates file: bad {w} line");
        for l in text.lines() {
            let f: Vec<&str> = l.split('\t').collect();
            let n = |i: usize| -> Result<i64, String> { f.get(i).and_then(|x| x.parse::<i64>().ok()).ok_or_else(|| bad(f.first().copied().unwrap_or("?"))) };
            match f.first().copied() {
                Some("SIDE17-CANDS") => cs.seg = u8::try_from(n(3)?).map_err(|_| bad("header"))?,
                Some("D") => cs.days = f.get(1).map_or(Ok(Vec::new()), |x| x.split(',').filter(|y| !y.is_empty()).map(|y| y.parse::<i64>().map_err(|_| bad("D"))).collect::<Result<Vec<_>, _>>())?,
                Some("C") => cs.cands.push(Cand { kind: f.get(2).and_then(|x| x.chars().next()).unwrap_or('?'), net: n(3)?, words: f.get(4..).map(|x| x.join("\t")).unwrap_or_default(), trades: Vec::new() }),
                Some("T") => {
                    let i = usize::try_from(n(1)?).map_err(|_| bad("T"))?;
                    let t = STrade { p: (n(2)?, n(3)?, n(4)?, n(5)?, n(6)?, u64::try_from(n(7)?).map_err(|_| bad("T"))?, u64::try_from(n(8)?).map_err(|_| bad("T"))?), seg: cs.seg, sod: u32::try_from(n(9)?).map_err(|_| bad("T"))?, units: u32::try_from(n(10)?).map_err(|_| bad("T"))?, sold: n(11)?, mark1: u32::try_from(n(12)?).map_err(|_| bad("T"))? };
                    cs.cands.get_mut(i).ok_or_else(|| bad("T (candidate)"))?.trades.push(t);
                }
                Some("S") => cs.wf.push((f.get(1).copied().unwrap_or("?").to_owned(), n(2)?, usize::try_from(n(3)?).map_err(|_| bad("S"))?)),
                Some("M") => {
                    let at = usize::try_from(n(1)?).map_err(|_| bad("M"))?;
                    if at != cs.marks.len() + 1 {
                        return Err(bad("M (offset)"));
                    }
                    for x in f.get(2).copied().unwrap_or("").split(',') {
                        cs.marks.push(x.parse::<i32>().map_err(|_| bad("M"))?);
                    }
                }
                _ => {}
            }
        }
        Ok(cs)
    }
}

/// The A19 figures of a walked account over the sessions `days` (a session, week or month without a closed trade is net Rs 0: failing):
/// (months >= Rs 2 L, months, positive weeks, weeks, positive sessions, sessions, worst month net).
fn a19_of(r: &crate::v32::WalkRes, days: &[i64]) -> (usize, usize, usize, usize, usize, usize, i64) {
    let mut mo: BTreeMap<i64, i64> = BTreeMap::new();
    let mut wk: BTreeMap<i64, i64> = BTreeMap::new();
    let mut dpos = 0;
    for &d in days {
        let v = r.day_net.get(&d).copied().unwrap_or(0);
        *mo.entry(crate::v32::month_id(d)).or_insert(0) += v;
        *wk.entry(crate::v32::week_of(d)).or_insert(0) += v;
        dpos += usize::from(v > 0);
    }
    (mo.values().filter(|&&v| crate::kern::month_ok(v)).count(), mo.len(), wk.values().filter(|&&v| v > 0).count(), wk.len(), dpos, days.len(), mo.values().copied().min().unwrap_or(0))
}

/// v3.2 round B: THE side-by-side combine (inside one run, or across two runs from their candidates files): ONE Rs 20 L account holding
/// index AND stock option sales, the ONE common walk (`v32::port_walk`, each position marked from its own segment). (1) HINDSIGHT: the
/// verdict views' best, greedy by the verdict order, up to 5 members; (2) WALK-FORWARD: per split, that split's way B pick of each segment
/// (each chosen on the split's learning months only) walked together over the split's test months, with the addendum 19 verdict. Returns
/// the RESULTS section and the TSV of every member trade's outcome (taken or the refusal rule) for an independent re-derivation.
#[must_use]
pub fn combined(idx: &CandSet, stk: &CandSet) -> (String, String) {
    let mut md = String::new();
    let mut tsv = String::from("account\tmember\tsegment\tcandidate\tentry_us\tknown_us\tmargin_paisa\tnet_paisa\tday\toutcome\n");
    let _ = writeln!(md, "## Addendum 17: the SIDE-BY-SIDE account (index AND stock options under ONE Rs 20 L account, the running MTM loss included; beside the separate accounts)");
    let _ = writeln!(md);
    if idx.days != stk.days {
        let _ = writeln!(md, "NOT RUN: the index and the stock candidates come from runs over different sessions ({} vs {}); the side-by-side account needs the same sessions.", idx.days.len(), stk.days.len());
        let _ = writeln!(md);
        return (md, tsv);
    }
    let days = &idx.days;
    let mut months: Vec<i64> = days.iter().map(|&d| crate::v32::month_id(d)).collect();
    months.dedup();
    // MUTANT m_v32rb_combine_marks_swapped: each segment's positions marked from the other segment's arena
    let marks: [&[i32]; 2] = if cfg!(feature = "m_v32rb_combine_marks_swapped") { [&stk.marks, &idx.marks] } else { [&idx.marks, &stk.marks] };
    // member = (segment, candidate index)
    let trades_of = |m: (u8, usize), from_day: i64| -> Vec<STrade> {
        let set = if m.0 == 0 { idx } else { stk };
        set.cands.get(m.1).map_or_else(Vec::new, |c| c.trades.iter().filter(|t| t.p.4 >= from_day).map(|t| STrade { seg: m.0, ..*t }).collect())
    };
    let walk = |mem: &[(u8, usize)], from_day: i64| -> (crate::v32::WalkRes, Vec<Vec<STrade>>) {
        let tr: Vec<Vec<STrade>> = mem.iter().map(|&m| trades_of(m, from_day)).collect();
        let w: Vec<Vec<crate::v32::WTrade>> = tr.iter().map(|v| v.iter().map(STrade::w).collect()).collect();
        let refs: Vec<&[crate::v32::WTrade]> = w.iter().map(Vec::as_slice).collect();
        (crate::v32::port_walk(&refs, &marks, &months), tr)
    };
    let mut log_rows = |acct: &str, mem: &[(u8, usize)], r: &crate::v32::WalkRes, tr: &[Vec<STrade>], tsv: &mut String| {
        for &(mi, ti, out) in &r.log {
            let (Some(&(sg, ci)), Some(t)) = (mem.get(mi), tr.get(mi).and_then(|v| v.get(ti))) else { continue };
            let _ = writeln!(tsv, "{acct}\t{}\t{}\t{ci}\t{}\t{}\t{}\t{}\t{}\t{out}", mi + 1, if sg == 0 { "index" } else { "stock" }, t.p.0, t.p.1, t.p.2, t.p.3, crate::cal::fmt_day(t.p.4));
        }
    };
    let row = |r: &crate::v32::WalkRes, ds: &[i64]| -> String {
        let (mok, mn, wp, wn, dp, dn, worst) = a19_of(r, ds);
        let taken: Vec<(i64, i64, i64)> = r.taken.iter().map(|x| (x.0, x.1, x.2)).collect();
        let (upk, umed, uhi, uidle) = crate::v32::utilization(&taken, ds);
        format!("{} ({} / {}) | {} / {} / {} / {} | {mok} / {mn} | {wp} / {wn} | {dp} / {dn} | {} | {} | {} | {} / {} | {} / {} | {}", r.trades, r.by_seg[0], r.by_seg[1], r.cap, r.cash, r.mtm, r.ruled, fmt_rs(worst), fmt_rs(r.dd), fmt_rs(r.net), fmt_rs(upk), fmt_rs(umed), crate::v32::share(uhi, 10_000), crate::v32::share(uidle, 10_000), fmt_rs(r.peak_mtm))
    };
    let head = "trades taken (index / stock) | refused: Rs 20 L cap / cash rule / running MTM loss alone / one-account rules | months >= Rs 2 L / months | positive weeks / weeks | positive sessions / sessions | worst month Rs | max DD Rs | net Rs | margin peak / median Rs | minutes > 80% of Rs 20 L / idle | peak margin + running MTM loss at a sale Rs";
    // (1) HINDSIGHT: the verdict views' best
    let vc: Vec<(u8, usize)> = [(0_u8, idx), (1, stk)].iter().flat_map(|(sg, s)| s.cands.iter().enumerate().filter(|(_, c)| c.kind == 'V').map(move |(i, _)| (*sg, i))).collect();
    let _ = writeln!(md, "### (1) Members chosen on this whole period (HINDSIGHT: NOT tradeable live)");
    let _ = writeln!(md);
    if !vc.iter().any(|m| m.0 == 0) || !vc.iter().any(|m| m.0 == 1) {
        let _ = writeln!(md, "NOT RUN: needs a verdict-view combo of BOTH segments (index {}, stock {}).", vc.iter().filter(|m| m.0 == 0).count(), vc.iter().filter(|m| m.0 == 1).count());
        let _ = writeln!(md);
    } else {
        let name = |m: &(u8, usize)| -> String {
            let s = if m.0 == 0 { idx } else { stk };
            format!("{} {}: {}", if m.0 == 0 { "INDEX" } else { "STOCK" }, m.1 + 1, s.cands.get(m.1).map_or("?", |c| c.words.as_str()))
        };
        let _ = writeln!(md, "Candidates: {}.", vc.iter().map(name).collect::<Vec<_>>().join("; "));
        let _ = writeln!(md);
        let _ = writeln!(md, "| members | {head} |");
        let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|");
        let score = |r: &crate::v32::WalkRes| (r.mo.values().filter(|&&v| crate::kern::month_ok(v)).count(), r.mo.values().copied().min().unwrap_or(0), -r.dd, r.net);
        let first = (0..vc.len()).max_by(|&a, &b| score(&walk(&vc[a..=a], i64::MIN).0).cmp(&score(&walk(&vc[b..=b], i64::MIN).0)).then(b.cmp(&a))).unwrap_or(0);
        let mut chosen: Vec<usize> = vec![first];
        loop {
            let mem: Vec<(u8, usize)> = chosen.iter().filter_map(|&i| vc.get(i).copied()).collect();
            let (r, tr) = walk(&mem, i64::MIN);
            let lab = chosen.iter().filter_map(|&i| vc.get(i)).map(|m| format!("{}{}", if m.0 == 0 { "I" } else { "S" }, m.1 + 1)).collect::<Vec<_>>().join(" + ");
            let _ = writeln!(md, "| {lab} | {} |", row(&r, days).replace(" | ", " | "));
            log_rows(&format!("hindsight {lab}"), &mem, &r, &tr, &mut tsv);
            if chosen.len() >= 5 {
                break;
            }
            let cur = score(&r);
            let best = (0..vc.len()).filter(|i| !chosen.contains(i)).map(|i| {
                let mut c2: Vec<(u8, usize)> = mem.clone();
                c2.extend(vc.get(i).copied());
                (score(&walk(&c2, i64::MIN).0), i)
            }).max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
            match best {
                Some((sc, i)) if sc > cur => chosen.push(i),
                _ => break,
            }
        }
        let _ = writeln!(md);
    }
    // (2) WALK-FORWARD: each split's way B picks of the two segments together over the split's test months
    let _ = writeln!(md, "### (2) Members chosen on learning months only (WALK-FORWARD: per split, each segment's way B pick of that split, walked together over the split's test months)");
    let _ = writeln!(md);
    let both: Vec<(&String, i64, usize, usize)> = idx.wf.iter().filter_map(|(id, m0, i)| stk.wf.iter().find(|x| &x.0 == id).map(|x| (id, *m0, *i, x.2))).collect();
    if both.is_empty() {
        let _ = writeln!(md, "NOT RUN: no split has a way B pick in both segments (needs the split permutations, --splits18 on, and a learning month and a test month in the run).");
        let _ = writeln!(md);
    } else {
        let _ = writeln!(md, "| split | test months | index pick / stock pick | {head} | walk-forward verdict (addendum 19 on the test months) |");
        let _ = writeln!(md, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
        let (mut pass, mut ran) = (0, 0);
        for (id, m0, ii, si) in both {
            let from = days.iter().copied().find(|&d| crate::v32::month_id(d) >= m0).unwrap_or(i64::MAX);
            let tdays: Vec<i64> = days.iter().copied().filter(|&d| d >= from).collect();
            if tdays.is_empty() {
                continue;
            }
            let mem = [(0_u8, ii), (1_u8, si)];
            let (r, tr) = walk(&mem, from);
            let (mok, mn, wp, wn, dp, dn, _) = a19_of(&r, &tdays);
            let ok = mn > 0 && mok == mn && wp == wn && dp == dn;
            ran += 1;
            pass += usize::from(ok);
            let _ = writeln!(md, "| {id} | {}..{} | I{} / S{} | {} | {} |", crate::v32::month_name(m0), crate::v32::month_name(crate::v32::month_id(tdays.last().copied().unwrap_or(from))), ii + 1, si + 1, row(&r, &tdays), if ok { "PASSED" } else { "NOT PASSED" });
            log_rows(&format!("walk-forward {id}"), &mem, &r, &tr, &mut tsv);
        }
        let _ = writeln!(md);
        let _ = writeln!(md, "Walk-forward side-by-side accounts: {pass} of {ran} PASSED (every test month >= Rs 2,00,000 net, every week and every session positive, against the run's sessions of the test months). Each member was chosen on its split's learning months only (way B); the account combines them under one Rs 20 L margin including the running MTM loss. Proof step 1 (own engine); the TSV `side17-walk.tsv` lists every member trade's outcome for an independent re-derivation.");
        let _ = writeln!(md);
    }
    let _ = writeln!(md, "Read: every row is ONE account of Rs 20,00,000 holding index and stock option sales together (never more than Rs 20 L of margin in use; a sale refused when the free money less the running MTM loss of the open positions cannot cover its margin); positions are marked from their own segment's minute closes. Candidates: {} index, {} stock (the trade lists of identical combos are walked once).", idx.cands.len(), stk.cands.len());
    let _ = writeln!(md);
    (md, tsv)
}

/// The SIDE-BY-SIDE section of a run from its segments' candidate sets (the very combine `spikefade side17-combine` runs across two runs).
#[must_use]
pub fn section_of(sets: &[CandSet]) -> (String, String) {
    if !crate::sw::on(crate::sw::SEARCH15) {
        return ("## Addendum 17: the SIDE-BY-SIDE account\n\nNOT RUN: the side-by-side account reads the verdict view of each segment (needs --search15 on or --v32).\n\n".to_owned(), String::new());
    }
    match (sets.first(), sets.get(1)) {
        (Some(a), Some(b)) if !a.cands.is_empty() && !b.cands.is_empty() => combined(a, b),
        _ => ("## Addendum 17: the SIDE-BY-SIDE account\n\nNOT RUN in this run: it holds one segment only; each run writes `side17-cands-<seg>.tsv` and `spikefade side17-combine <index file> <stock file> <out dir>` combines two runs.\n\n".to_owned(), String::new()),
    }
}

/// `spikefade side17-combine <index side17-cands-idx.tsv> <stock side17-cands-stk.tsv> <out dir>`: the side-by-side account across two
/// separate runs (index and stock options are separate processes in the all-years plan); writes SIDE17-COMBINED.md and side17-walk.tsv.
///
/// # Errors
/// Unreadable inputs, a wrong segment, an unwritable output.
pub fn cmd_combine(a: &[String]) -> Result<(), String> {
    let (Some(fi), Some(fs), Some(out)) = (a.first(), a.get(1), a.get(2)) else { return Err("usage: spikefade side17-combine <side17-cands-idx.tsv> <side17-cands-stk.tsv> <out dir>".to_owned()) };
    let rd = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"));
    let (idx, stk) = (CandSet::of_tsv(&rd(fi)?)?, CandSet::of_tsv(&rd(fs)?)?);
    if idx.seg != 0 || stk.seg != 1 {
        return Err(format!("side17-combine: the first file must be the index segment's (seg 0) and the second the stock segment's (seg 1); got {} and {}", idx.seg, stk.seg));
    }
    let (md, tsv) = combined(&idx, &stk);
    std::fs::create_dir_all(out).map_err(|e| format!("{out}: {e}"))?;
    let head = format!("# SIDE17-COMBINED: the side-by-side account across two runs\n\nInputs: {fi} (sha256 {}), {fs} (sha256 {}).\n\n", crate::cube::sha256(std::path::Path::new(fi)), crate::cube::sha256(std::path::Path::new(fs)));
    std::fs::write(std::path::Path::new(out).join("SIDE17-COMBINED.md"), head + &md).map_err(|e| e.to_string())?;
    std::fs::write(std::path::Path::new(out).join("side17-walk.tsv"), tsv).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tr(entry_s: i64, known_s: i64, margin: i64, net: i64, ck: u64, rk: u64, seg: u8, units: u32, sold: i64, mark1: u32) -> STrade {
        let day = 20_000_i64;
        STrade { p: (crate::cal::at_sod(day, entry_s), crate::cal::at_sod(day, known_s), margin, net, day, ck, rk), seg, sod: u32::try_from(entry_s).unwrap_or(0), units, sold, mark1 }
    }

    /// The side-by-side account refuses a sale when the free money less the RUNNING MTM LOSS of the open positions (each from its own
    /// segment's minute closes) cannot cover its margin, while the cap and the cash rule alone would let it through.
    #[test]
    fn v32b_side_mtm_from_each_segment() {
        let open = crate::cal::OPEN_S;
        // index position: 100 units sold for Rs 10,000 (paisa 1,000,000) at 09:30; its minute closes jump to Rs 1,000 a unit by 10:00
        let mut im = vec![0_i32; crate::cal::NM];
        for (i, x) in im.iter_mut().enumerate() {
            *x = if i >= 40 { 100_000 } else { 10_000 };
        }
        let sm = vec![10_000_i32; crate::cal::NM];
        let a = tr(open + 15 * 60, open + 5 * 3_600, 150_000_000, -5_000_000, 1, 1, 0, 100, 1_000_000, 1);
        // a stock sale at 10:30 needing Rs 6 L: cap ok (Rs 15 L + 6 L > 20 L? no: 21 L > 20 L) -> use Rs 4 L: cap ok (19 L), free = 5 L
        // before MTM; the index position's loss at 10:30 = 100 x Rs 1,000 - Rs 10,000 = Rs 90,000 -> free 5 L - 0.9 L = 4.1 L >= 4 L: taken
        let b = tr(open + 75 * 60, open + 2 * 3_600, 40_000_000, 1_000_000, 2, 2, 1, 50, 500_000, 1);
        let r = side_walk(&[&[a], &[b]], [&im, &sm], &[]);
        assert_eq!((r.trades, r.mtm), (2, 0));
        // the same with Rs 4.5 L: 5 L - 0.9 L = 4.1 L < 4.5 L while 5 L >= 4.5 L: refused by the running MTM loss alone
        let c = tr(open + 75 * 60, open + 2 * 3_600, 45_000_000, 1_000_000, 2, 2, 1, 50, 500_000, 1);
        let r2 = side_walk(&[&[a], &[c]], [&im, &sm], &[]);
        assert_eq!((r2.trades, r2.mtm, r2.cap, r2.cash), (1, 1, 0, 0));
        // the stock marks never stand in for the index position's: with flat index marks the sale is taken
        let flat = vec![10_000_i32; crate::cal::NM];
        let r3 = side_walk(&[&[a], &[c]], [&flat, &im], &[]);
        assert_eq!((r3.trades, r3.mtm), (2, 0));
    }

    /// One account's rules across the two segments: the same contract key in ANOTHER segment is another contract (no false refusal), the
    /// Rs 20 L cap binds across segments.
    #[test]
    fn v32b_side_one_account_rules_and_cap() {
        let open = crate::cal::OPEN_S;
        let m = vec![0_i32; crate::cal::NM];
        let a = tr(open + 600, open + 3_600, 120_000_000, 100, 7, 7, 0, 1, 1, 0);
        let b = tr(open + 900, open + 3_600, 70_000_000, 100, 7, 7, 1, 1, 1, 0);
        let r = side_walk(&[&[a], &[b]], [&m, &m], &[]);
        assert_eq!((r.trades, r.ruled, r.cap), (2, 0, 0), "keys 7 of the index and of the stock segment are different contracts");
        let c = tr(open + 900, open + 3_600, 90_000_000, 100, 8, 8, 1, 1, 1, 0);
        let r2 = side_walk(&[&[a], &[c]], [&m, &m], &[]);
        assert_eq!((r2.trades, r2.cap), (1, 1), "Rs 12 L + Rs 9 L > Rs 20 L across the segments");
        let d = tr(open + 900, open + 3_600, 10_000_000, 100, 9, 7, 0, 1, 1, 0);
        let r3 = side_walk(&[&[a], &[d]], [&m, &m], &[]);
        assert_eq!((r3.trades, r3.ruled), (1, 1), "the same underlying + side + expiry in one segment");
    }

    /// `--idx-und`: the index lists keep only the named underlyings' contracts (lists left empty dropped); stock lists untouched; unset =
    /// every index underlying.
    #[test]
    fn v32b_idx_und_own_account() {
        let mut c = Cube::default();
        for (n, k) in [("NIFTY", 0_u8), ("BANKNIFTY", 0), ("FINNIFTY", 0), ("TCS", 1)] {
            c.unds.push(crate::cube::Und { name: n.to_owned(), kind: k, ..crate::cube::Und::default() });
        }
        for (u, k) in [(0_u16, 0_u8), (1, 0), (2, 0), (0, 0), (3, 1)] {
            c.cons.push(crate::cube::Contract { und: u, kind: k, ..crate::cube::Contract::default() });
        }
        let lists = vec![(20_261_124_u32, vec![0_u32, 1, 3]), (20_261_125, vec![2])];
        assert_eq!(keep_unds(&c, 0, lists.clone()), lists, "no filter: unchanged");
        set_idx_und(vec!["FINNIFTY".to_owned()]);
        assert_eq!(keep_unds(&c, 0, lists.clone()), vec![(20_261_125, vec![2])]);
        set_idx_und(vec!["NIFTY".to_owned()]);
        assert_eq!(keep_unds(&c, 0, lists.clone()), vec![(20_261_124, vec![0, 3])]);
        let stk = vec![(20_261_125_u32, vec![4_u32])];
        assert_eq!(keep_unds(&c, 1, stk.clone()), stk, "stock lists untouched");
        assert!(from_args(&["--idx-und".to_owned(), "nifty".to_owned()]).is_err());
    }

    /// Two candidate sets over 3 sessions: an index position whose minute closes jump (its own arena) and a stock sale the running MTM loss
    /// refuses; each set also carries a way B pick for a split whose test starts at the third session's month.
    fn sets3() -> (CandSet, CandSet) {
        let open = crate::cal::OPEN_S;
        let days = vec![20_000_i64, 20_001, 20_031];
        let mut jump = vec![10_000_i32; crate::cal::NM];
        for (i, x) in jump.iter_mut().enumerate() {
            if i >= 40 {
                *x = 100_000;
            }
        }
        let flat = vec![10_000_i32; crate::cal::NM];
        let ti = |d: i64, s: i64, k: i64, m: i64, n: i64, key: u64, mark1: u32| STrade { p: (crate::cal::at_sod(d, s), crate::cal::at_sod(d, k), m, n, d, key, key), seg: 0, sod: u32::try_from(s).unwrap_or(0), units: 100, sold: 1_000_000, mark1 };
        let idx = CandSet {
            seg: 0,
            days: days.clone(),
            marks: [jump.clone(), flat.clone()].concat(),
            cands: vec![
                Cand { kind: 'V', words: "idx combo".to_owned(), net: 0, trades: vec![ti(20_000, open + 900, open + 18_000, 150_000_000, -5_000_000, 1, 1)] },
                Cand { kind: 'B', words: "idx way B".to_owned(), net: 0, trades: vec![ti(20_000, open + 600, open + 1_200, 10_000_000, 1_000, 3, 0), ti(20_031, open + 900, open + 18_000, 150_000_000, 30_000_000, 4, 1)] },
            ],
            wf: vec![("TT-X".to_owned(), crate::v32::month_id(20_031), 1)],
        };
        let ts = |d: i64, s: i64, k: i64, m: i64, n: i64, key: u64| STrade { seg: 1, ..ti(d, s, k, m, n, key, 1) };
        let stk = CandSet {
            seg: 1,
            days,
            marks: flat,
            cands: vec![
                Cand { kind: 'V', words: "stk combo".to_owned(), net: 0, trades: vec![ts(20_000, open + 4_500, open + 7_200, 45_000_000, 1_000_000, 2)] },
                Cand { kind: 'B', words: "stk way B".to_owned(), net: 0, trades: vec![ts(20_000, open + 600, open + 1_200, 10_000_000, 9, 5), ts(20_031, open + 4_500, open + 7_200, 45_000_000, 25_000_000, 6)] },
            ],
            wf: vec![("TT-X".to_owned(), crate::v32::month_id(20_031), 1)],
        };
        (idx, stk)
    }

    /// v3.2 round B: the cross-run combine reads exactly what one run holds: the candidates file round trip is lossless and the combine of
    /// the read-back sets equals the combine of the sets themselves (RESULTS section and the per-trade TSV), so `spikefade side17-combine`
    /// on two separate runs = the side-by-side section of one run holding both segments.
    #[test]
    fn v32rb_side_combine_file_roundtrip_equals_in_run() {
        crate::sw::set(crate::sw::SEARCH15, true);
        let (a, b) = sets3();
        let (a2, b2) = (CandSet::of_tsv(&a.to_tsv()).unwrap_or_default(), CandSet::of_tsv(&b.to_tsv()).unwrap_or_default());
        assert_eq!(a2.to_tsv(), a.to_tsv());
        assert_eq!(b2.to_tsv(), b.to_tsv());
        assert_eq!(combined(&a, &b), combined(&a2, &b2));
        assert_eq!(section_of(&[a.clone(), b.clone()]), combined(&a, &b));
        let other = CandSet { days: vec![1, 2], ..b };
        assert!(combined(&a, &other).0.contains("NOT RUN"), "different sessions refused");
    }

    /// v3.2 round B: in the combine each position is marked from ITS OWN segment's arena (the index position's jump refuses the stock sale
    /// by the running MTM loss alone), and the walk-forward account reads only the split's test months.
    #[test]
    fn v32rb_side_combine_marks_each_segment_and_test_months() {
        crate::sw::set(crate::sw::SEARCH15, true);
        let (a, b) = sets3();
        let (md, tsv) = combined(&a, &b);
        let wf: Vec<&str> = tsv.lines().filter(|l| l.starts_with("walk-forward TT-X")).collect();
        assert_eq!(wf.len(), 2, "only the test month's trades (one per member):\n{tsv}");
        assert!(wf.iter().any(|l| l.contains("\tindex\t") && l.ends_with("\ttaken")), "{tsv}");
        assert!(wf.iter().any(|l| l.contains("\tstock\t") && l.ends_with("\tmtm_loss")), "the stock sale is refused by the index position's running loss (its own arena):\n{tsv}");
        assert!(md.contains("| TT-X |"));
    }
}
