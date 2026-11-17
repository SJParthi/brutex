//! The account sweep: every (event list, exit config, slot count) account of one segment, simulated event by event with every account of
//! the list updated per event (the event's outcome table stays hot). Rules: N slots of Rs 20 L / N, each position the largest whole lot
//! count whose margin estimate fits its slot; a slot refills when its position's last fill is known; one position per contract at a time;
//! the free margin (Rs 20 L + min(0, realized net) - open margins) must cover the position's margin (lost money never reused, no
//! compounding, base Q7; a position whose margin it cannot cover is refused, margin_cash_short); intraday only. The drawdown is on the
//! realized net in the order positions close (their last fill known).
//!
//! Rule set v2 (operator 14 Nov 23:1x IST, `rules.rs`), checked in this order after the event-level refusals and the slot's lot count:
//! `contract_open` (the same contract held), `same_side_dup` (R1: another strike of a held (underlying, side, expiry)), `slots_full`,
//! `margin_cap_20L` (R4: margin in use + this position's margin > Rs 20 L), `margin_cash_short` (v1's lost-money rule), `not_filled`.
//! v1 order (the regression replay): `slots_full`, `contract_open`, `margin_cash_short`, `not_filled`. The margin in use and its peak are
//! tracked per account (entry estimates, blocked from T to the last fill).

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::trade::{ACCOUNT, BASE_S, EvT, NRULE, NSLOT, Out, Refuse, cand_of};

/// Exit configs: (stop k, target j, time m, share l) = ((k*5 + j)*5 + m)*5 + l.
pub const NX: usize = 625;
/// Accounts per list.
pub const NACC: usize = NX * 4;

/// (k, j, m, l) of an exit config.
#[must_use]
pub const fn x_parts(x: usize) -> (usize, usize, usize, usize) {
    (x / 125, (x / 25) % 5, (x / 5) % 5, x % 5)
}

/// One sweep event (a unique event with its table). `skey` = the R1 key (underlying, side, expiry) of the day (`trade::side_key`).
#[derive(Clone, Debug, Default)]
pub struct SEv {
    pub di: u16,
    pub ci: u32,
    pub t: u32,
    pub skey: u32,
    pub tab: EvT,
}

/// An account's summary.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sum {
    pub seg: u8,
    pub list: u32,
    pub x: u16,
    pub n: u8,
    pub net: i64,
    pub trades: u32,
    pub wins: u32,
    pub mdd: i64,
    pub worst: i64,
    pub worst_day: i64,
    pub days_pos: u16,
    pub days_neg: u16,
    pub days_traded: u16,
    pub ndays: u16,
    /// The account's event list comes from at least one GAIN-threshold config (+100% or more: a spike list, not rank-only).
    pub spike: bool,
    /// The peak margin in use (entry estimates of the open positions, paisa).
    pub pk_margin: i64,
    /// v3.1 item 5: the OTHER max drawdown figure, printed beside once (booking order when `mdd` is the grouped one, and the reverse).
    pub mdd_alt: i64,
    /// v3.1 decision (c): the net at the headline gate's stress (+2% adverse on both sides).
    pub net_s2: i64,
    /// v3.1 item 3: trades with units the fill rule never closed, those units, and those trades' net (valued, not filled).
    pub nc_trades: u32,
    pub nc_units: u64,
    pub nc_net: i64,
    /// v3.1 item 4: events refused by the MTM-inclusive margin check alone (margin_mtm_short).
    pub mtm_ref: u32,
    /// check-build-v3 S1 (design 5.2): the tie key of the account's setting (families on, SL kind rank, level), 0 under the v3 ties.
    pub tie: u32,
    /// v3.2 addendum 15 (SEARCH15): months the account saw, of them at or above Rs 2,00,000 net, the worst seen month's net; weeks seen,
    /// positive, the worst week's net (paisa). A month or week without an event for the account is not seen (net 0, failing).
    pub mo_seen: u16,
    pub mo_ok: u16,
    pub mo_min: i64,
    pub wk_seen: u16,
    pub wk_pos: u16,
    pub wk_min: i64,
    /// v3.2-B (addendum 18): this combo's split statistics (SPLITS18; zeros otherwise).
    pub sp: crate::split18::SpStat,
}

impl Sum {
    /// Every staged day of the segment net positive.
    #[must_use]
    pub const fn every_day(&self) -> bool {
        self.days_pos == self.ndays
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Pos {
    ci: u32,
    skey: u32,
    exit: u32,
    margin: i64,
    net: i64,
}

#[derive(Clone, Debug, Default)]
struct Acc {
    nopen: usize,
    next_exit: u32,
    realized: i64,
    peak: i64,
    mdd: i64,
    trades: u32,
    wins: u32,
    worst: i64,
    worst_day: i64,
    day_net: i64,
    day_trades: u32,
    cur_day: u16,
    days_pos: u16,
    days_neg: u16,
    days_traded: u16,
    margin_open: i64,
    pk_margin: i64,
}

/// The views kept per segment (REQ 55 biggest-profit views first, then the standing ranking).
pub const NVIEW: usize = 9;
/// View names.
pub const VIEW_NAMES: [&str; NVIEW] = [
    "A biggest net (overall)",
    "X biggest net, SPIKE events only (gain threshold +100% or more; rank-only lists excluded)",
    "B biggest net, every staged day positive",
    "C biggest net, no losing day (every traded day positive)",
    "D biggest net, at most 1 losing day",
    "E biggest net, at least 1 trade a day on average",
    "F biggest net, at least 5 trades a day on average",
    "G biggest net, at least 10 trades a day on average",
    "S standing: every staged day positive, then smallest max drawdown, then most net (net > 0)",
];
/// View letters.
pub const VIEW_LETTERS: [&str; NVIEW] = ["A", "X", "B", "C", "D", "E", "F", "G", "S"];
/// v3.2 addendum 15: the VERDICT view (heap index 9, kept only under the SEARCH15 switch; printed in its own section, never in v3.1's
/// tables): (1) the most months at or above Rs 2,00,000 net, (2) the most positive weeks, (3) the smallest max drawdown, (4) the biggest net.
pub const VIEW_V: usize = 9;
/// The verdict view's name.
pub const VIEW_V_NAME: &str = "V verdict order (addendum 15): most months >= Rs 2,00,000 net, then most positive weeks, then smallest max drawdown, then most net";
/// REPAIR A: the verdict view's name under addendum 19.
pub const VIEW_V_NAME19: &str = "V verdict order (addendum 19): most months >= Rs 2,00,000 net, then most positive weeks, then most positive sessions, then smallest max drawdown, then most net";

/// A view's printed name (the verdict view beyond v3.1's nine).
#[must_use]
pub fn view_name(v: usize) -> &'static str {
    VIEW_NAMES.get(v).copied().unwrap_or(if v == VIEW_V { if crate::sw::on(crate::sw::A19) { VIEW_V_NAME19 } else { VIEW_V_NAME } } else { "?" })
}

/// A view's letter.
#[must_use]
pub fn view_letter(v: usize) -> &'static str {
    VIEW_LETTERS.get(v).copied().unwrap_or(if v == VIEW_V { "V" } else { "?" })
}

/// The verdict key of an account (addendum 15's order).
#[must_use]
pub fn verdict_key(s: &Sum) -> (i64, i64, i64) {
    // REPAIR A (check-engine-2 S9, addendum 19): months >= Rs 2 L, then positive weeks, then positive SESSIONS, then the smallest max DD,
    // then the most net
    if crate::sw::on(crate::sw::A19) {
        // MUTANT m_v32r_a19_key_no_days: the positive sessions left out of the order
        let d = if cfg!(feature = "m_v32r_a19_key_no_days") { 0 } else { i64::from(s.days_pos) };
        return (i64::from(s.mo_ok) * 10_000_000_000 + i64::from(s.wk_pos) * 100_000 + d, -s.mdd, s.net);
    }
    (i64::from(s.mo_ok) * 100_000 + i64::from(s.wk_pos), -s.mdd, s.net)
}

#[derive(Clone, Copy, Debug)]
struct Keyed {
    k: (i64, i64, i64),
    s: Sum,
}
impl PartialEq for Keyed {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Keyed {}
impl PartialOrd for Keyed {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Keyed {
    fn cmp(&self, o: &Self) -> Ordering {
        if crate::sw::on(crate::sw::TIE52) {
            // check-build-v3 S1 (design 5.2): after the key, fewer families on, then the SL kind rank and the smaller level (the tie key),
            // then the setting's file order, then list, share level and N (smaller is better throughout)
            let nl = crate::trade::NL as u16;
            return self.k.cmp(&o.k).then(o.s.seg.cmp(&self.s.seg)).then(o.s.tie.cmp(&self.s.tie)).then((o.s.x / nl).cmp(&(self.s.x / nl))).then(o.s.list.cmp(&self.s.list)).then((o.s.x % nl).cmp(&(self.s.x % nl))).then(o.s.n.cmp(&self.s.n));
        }
        self.k.cmp(&o.k).then(o.s.seg.cmp(&self.s.seg)).then(o.s.list.cmp(&self.s.list)).then(o.s.x.cmp(&self.s.x)).then(o.s.n.cmp(&self.s.n))
    }
}

/// Top-K heaps of every view.
#[derive(Clone, Debug, Default)]
pub struct Tops {
    k: usize,
    h: Vec<BinaryHeap<std::cmp::Reverse<Keyed>>>,
}

impl Tops {
    /// Empty heaps keeping `k` per view.
    #[must_use]
    pub fn new(k: usize) -> Self {
        Self { k, h: vec![BinaryHeap::new(); NVIEW + 1] }
    }

    fn push_view(&mut self, v: usize, key: (i64, i64, i64), s: Sum) {
        let k = self.k;
        let Some(h) = self.h.get_mut(v) else { return };
        let e = Keyed { k: key, s };
        if h.len() < k {
            h.push(std::cmp::Reverse(e));
        } else if let Some(top) = h.peek()
            && e > top.0
        {
            h.pop();
            h.push(std::cmp::Reverse(e));
        }
    }

    /// Offers one account to every view.
    pub fn offer(&mut self, s: Sum) {
        if s.trades == 0 {
            return;
        }
        let byn = (s.net, -s.mdd, i64::from(s.trades));
        self.push_view(0, byn, s);
        if s.spike {
            self.push_view(1, byn, s);
        }
        if s.every_day() {
            self.push_view(2, byn, s);
        }
        if s.days_neg == 0 {
            self.push_view(3, byn, s);
        }
        if s.days_neg <= 1 {
            self.push_view(4, byn, s);
        }
        let nd = u32::from(s.ndays.max(1));
        if s.trades >= nd {
            self.push_view(5, byn, s);
        }
        if s.trades >= 5 * nd {
            self.push_view(6, byn, s);
        }
        if s.trades >= 10 * nd {
            self.push_view(7, byn, s);
        }
        if s.net > 0 {
            self.push_view(8, (-i64::from(s.ndays.saturating_sub(s.days_pos)), -s.mdd, s.net), s);
        }
        if crate::sw::on(crate::sw::SEARCH15) {
            self.push_view(VIEW_V, verdict_key(&s), s);
        }
    }

    /// Merges another set of heaps.
    pub fn merge(&mut self, o: Self) {
        for (v, h) in o.h.into_iter().enumerate() {
            for e in h {
                self.push_view(v, e.0.k, e.0.s);
            }
        }
    }

    /// The view's accounts, best first.
    #[must_use]
    pub fn best(&self, v: usize) -> Vec<Sum> {
        let mut x: Vec<Keyed> = self.h.get(v).map(|h| h.iter().map(|e| e.0).collect()).unwrap_or_default();
        x.sort_by(|a, b| b.cmp(a));
        x.into_iter().map(|e| e.s).collect()
    }

    /// v3.3 fix CAP_MIN (check-engine-1 M1r): view `v` keeps only its first `k` accounts (best first) that `keep` admits; every other entry of
    /// that view is dropped (an account that was not checked can never become a pick). The other views are untouched.
    pub fn retain_view(&mut self, v: usize, k: usize, keep: &dyn Fn(&Sum) -> bool) {
        let Some(h) = self.h.get_mut(v) else { return };
        let mut x: Vec<Keyed> = h.iter().map(|e| e.0).collect();
        x.sort_by(|a, b| b.cmp(a));
        *h = x.into_iter().take(k).filter(|e| keep(&e.s)).map(std::cmp::Reverse).collect();
    }
}

/// What one account does with one event (shared by the sweep and the detailed run, so both read the same rules). Returns (verdict code,
/// the slot's lot count, the exit candidate).
#[inline]
fn step(ac: &mut Acc, p: &mut [Pos], e: &SEv, store: &[Out], astore: &[[u16; NRULE]], (k, j, m, l): (usize, usize, usize, usize), ni: usize, v2: bool) -> (&'static str, u16, usize) {
    let key = u32::from(e.di) * 86_400 + e.t;
    let tab = &e.tab;
    if ac.cur_day != e.di {
        flush(ac, p, u32::MAX);
        close_day(ac);
        ac.cur_day = e.di;
    }
    if tab.refuse != Refuse::None {
        return (tab.refuse.code(), 0, 0);
    }
    let lots = tab.lots.get(ni).copied().unwrap_or(0);
    if lots == 0 {
        return ("margin_over_slot", 0, 0);
    }
    if ac.next_exit <= key {
        flush(ac, p, key);
    }
    if v2 {
        let open = p.get(..ac.nopen).unwrap_or(&[]);
        if open.iter().any(|q| q.ci == e.ci) {
            return ("contract_open", lots, 0);
        }
        // R1: at most one open position per (underlying, side, expiry)
        if open.iter().any(|q| q.skey == e.skey) {
            return ("same_side_dup", lots, 0);
        }
        if ac.nopen >= p.len() {
            return ("slots_full", lots, 0);
        }
    } else {
        if ac.nopen >= p.len() {
            return ("slots_full", lots, 0);
        }
        if p.get(..ac.nopen).is_some_and(|o| o.iter().any(|q| q.ci == e.ci)) {
            return ("contract_open", lots, 0);
        }
    }
    let margin = i64::from(lots) * tab.margin_lot;
    // R4: the margin in use at this instant (every open position's entry estimate) plus this one never passes Rs 20 L
    if v2 && ac.margin_open + margin > ACCOUNT {
        return ("margin_cap_20L", lots, 0);
    }
    let free = ACCOUNT + ac.realized.min(0) - ac.margin_open;
    if free < margin {
        return ("margin_cash_short", lots, 0);
    }
    if !tab.filled.get(l).copied().unwrap_or(false) {
        return ("not_filled", lots, 13);
    }
    let q = usize::from(tab.q_of.get(ni).copied().unwrap_or(255));
    let Some(anc) = astore.get(tab.aidx(l, q)) else { return ("not_filled", lots, 13) };
    let cand = cand_of(anc, k, j, m);
    let Some(o) = store.get(tab.idx(l, q, cand)) else { return ("not_filled", lots, cand) };
    if o.flag & 1 == 0 {
        return ("not_filled", lots, cand);
    }
    let ex = u32::from(e.di) * 86_400 + u32::try_from(i64::from(o.exit) + BASE_S).unwrap_or(0);
    if let Some(slot) = p.get_mut(ac.nopen) {
        *slot = Pos { ci: e.ci, skey: e.skey, exit: ex, margin, net: i64::from(o.net) };
    }
    ac.nopen += 1;
    ac.margin_open += margin;
    ac.pk_margin = ac.pk_margin.max(ac.margin_open);
    ac.next_exit = ac.next_exit.min(ex);
    ac.trades += 1;
    ac.day_trades += 1;
    ("booked", lots, cand)
}

/// Runs every account of one event list under rule set `v2` (or v1). Returns one summary per account (index = x * 4 + n).
#[must_use]
pub fn sweep_list(evs: &[SEv], list: &[u32], store: &[Out], astore: &[[u16; NRULE]], ndays: u16, seg: u8, lid: u32, v2: bool) -> Vec<Sum> {
    let mut acc: Vec<Acc> = vec![Acc { next_exit: u32::MAX, cur_day: u16::MAX, ..Acc::default() }; NACC];
    let cap: [usize; 4] = [5, 10, 20, 40];
    let parts: Vec<(usize, usize, usize, usize)> = (0..NX).map(x_parts).collect();
    let mut base = vec![0_usize; NACC];
    let mut tot = 0_usize;
    for (a, b) in base.iter_mut().enumerate() {
        *b = tot;
        tot += cap.get(a % 4).copied().unwrap_or(0);
    }
    let mut pos: Vec<Pos> = vec![Pos::default(); tot];
    for &eid in list {
        let Some(e) = evs.get(eid as usize) else { continue };
        let all_refused = e.tab.refuse != Refuse::None;
        for a in 0..NACC {
            let ni = a % 4;
            let Some(ac) = acc.get_mut(a) else { continue };
            if ac.cur_day == e.di && (all_refused || e.tab.lots.get(ni).copied().unwrap_or(0) == 0) {
                continue;
            }
            let b0 = base.get(a).copied().unwrap_or(0);
            let p = pos.get_mut(b0..b0 + cap.get(ni).copied().unwrap_or(0)).unwrap_or(&mut []);
            let xp = parts.get(a / 4).copied().unwrap_or((0, 0, 0, 0));
            let _ = step(ac, p, e, store, astore, xp, ni, v2);
        }
    }
    let mut out = Vec::with_capacity(NACC);
    for (a, ac) in acc.iter_mut().enumerate() {
        let b0 = base.get(a).copied().unwrap_or(0);
        let p = pos.get_mut(b0..b0 + cap.get(a % 4).copied().unwrap_or(0)).unwrap_or(&mut []);
        flush(ac, p, u32::MAX);
        close_day(ac);
        out.push(Sum { seg, list: lid, x: u16::try_from(a / 4).unwrap_or(0), n: u8::try_from(a % 4).unwrap_or(0), net: ac.realized, trades: ac.trades, wins: ac.wins, mdd: ac.mdd, worst: ac.worst, worst_day: ac.worst_day, days_pos: ac.days_pos, days_neg: ac.days_neg, days_traded: ac.days_traded, ndays, spike: false, pk_margin: ac.pk_margin, ..Sum::default() });
    }
    out
}

/// Realizes every open position whose exit is known at or before `upto`, in exit order.
fn flush(ac: &mut Acc, p: &mut [Pos], upto: u32) {
    loop {
        let mut bi = usize::MAX;
        let mut be = u32::MAX;
        for (i, q) in p.iter().take(ac.nopen).enumerate() {
            if q.exit < be {
                be = q.exit;
                bi = i;
            }
        }
        if bi == usize::MAX || be > upto {
            ac.next_exit = be;
            return;
        }
        let q = p.get(bi).copied().unwrap_or_default();
        ac.realized += q.net;
        ac.day_net += q.net;
        if q.net > 0 {
            ac.wins += 1;
        }
        ac.worst = ac.worst.min(q.net);
        ac.peak = ac.peak.max(ac.realized);
        ac.mdd = ac.mdd.max(ac.peak - ac.realized);
        ac.margin_open -= q.margin;
        let last = ac.nopen - 1;
        p.swap(bi, last);
        ac.nopen -= 1;
    }
}

fn close_day(ac: &mut Acc) {
    if ac.cur_day == u16::MAX {
        return;
    }
    if ac.day_trades > 0 {
        ac.days_traded += 1;
    }
    ac.worst_day = ac.worst_day.min(ac.day_net);
    match ac.day_net.cmp(&0) {
        Ordering::Greater => ac.days_pos += 1,
        Ordering::Less => ac.days_neg += 1,
        Ordering::Equal => {}
    }
    ac.day_net = 0;
    ac.day_trades = 0;
}

/// One decision of a detailed run: the event, the verdict code, the slot's lot count, the exit candidate, the margin in use right after
/// the decision (every open position's entry estimate, this one included when booked) and the open positions then.
#[derive(Clone, Copy, Debug)]
pub struct Dec {
    pub eid: u32,
    pub code: &'static str,
    pub lots: u16,
    pub cand: usize,
    pub margin_in_use: i64,
    pub nopen: usize,
}

/// One account's detailed run (the very same `step` as the sweep) under rule set `v2` (or v1): every decision, and the summary.
#[must_use]
pub fn detail_list(evs: &[SEv], list: &[u32], store: &[Out], astore: &[[u16; NRULE]], x: usize, ni: usize, ndays: u16, v2: bool) -> (Vec<Dec>, Sum) {
    let cap = [5_usize, 10, 20, 40].get(ni).copied().unwrap_or(5);
    let mut ac = Acc { next_exit: u32::MAX, cur_day: u16::MAX, ..Acc::default() };
    let mut p = vec![Pos::default(); cap];
    let mut out = Vec::new();
    let xp = x_parts(x);
    for &eid in list {
        let Some(e) = evs.get(eid as usize) else { continue };
        let (code, lots, cand) = step(&mut ac, &mut p, e, store, astore, xp, ni, v2);
        out.push(Dec { eid, code, lots, cand, margin_in_use: ac.margin_open, nopen: ac.nopen });
    }
    flush(&mut ac, &mut p, u32::MAX);
    close_day(&mut ac);
    let s = Sum { seg: 0, list: 0, x: u16::try_from(x).unwrap_or(0), n: u8::try_from(ni).unwrap_or(0), net: ac.realized, trades: ac.trades, wins: ac.wins, mdd: ac.mdd, worst: ac.worst, worst_day: ac.worst_day, days_pos: ac.days_pos, days_neg: ac.days_neg, days_traded: ac.days_traded, ndays, spike: false, pk_margin: ac.pk_margin, ..Sum::default() };
    (out, s)
}

/// The slot count of index `ni`.
#[must_use]
pub fn n_of(ni: usize) -> i64 {
    NSLOT.get(ni).copied().unwrap_or(0)
}

/// REPAIR A (check-engine-2 S3): the slot count of slot index `ni` in segment `seg` (the per-segment grid under the NGRID switch; v3.1's
/// 5 / 10 / 20 / 40 otherwise, so every v3.1 / h6 print is unchanged).
#[must_use]
pub fn n_of_s(seg: usize, ni: usize) -> i64 {
    crate::sw::n_slots(u8::try_from(seg).unwrap_or(0), ni)
}

#[cfg(test)]
mod tests {
    //! Rule R1 (one open position per (underlying, side, expiry)) and rule R4 (the Rs 20 L margin cap), each against v1.
    use super::*;
    use crate::trade::{NCAND, NL};

    /// One event at `t` (second of the day) on contract `ci` with R1 key `skey`: `lots` lots of margin `margin_lot` per lot at every slot
    /// count, the sale filled at every level, every exit candidate the same outcome (net `net`, last fill known `dur` seconds after T).
    fn ev(store: &mut Vec<Out>, astore: &mut Vec<[u16; NRULE]>, ci: u32, skey: u32, t: u32, lots: u16, margin_lot: i64, net: i32, dur: u32) -> SEv {
        let mut tab = EvT { lots: [lots; 4], q_of: [0; 4], qs: vec![lots], margin_lot, filled: [true; NL], ..EvT::default() };
        tab.off = u32::try_from(store.len()).unwrap_or(0);
        tab.aoff = u32::try_from(astore.len()).unwrap_or(0);
        let exit = u16::try_from(i64::from(t + dur) - BASE_S).unwrap_or(0);
        store.extend(std::iter::repeat_n(Out { net, exit, flag: 1, ..Out::default() }, NL * NCAND));
        astore.extend(std::iter::repeat_n([crate::trade::NEVER; NRULE], NL));
        SEv { di: 0, ci, t, skey, tab }
    }

    const X_SQ: usize = 120; // stop +5%, no target, no time exit, share 10%: every anchor NEVER -> the square-off candidate

    fn codes(evs: &[SEv], store: &[Out], astore: &[[u16; NRULE]], v2: bool) -> (Vec<&'static str>, Sum, Vec<i64>) {
        let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap_or(0)).collect();
        let (d, s) = detail_list(evs, &list, store, astore, X_SQ, 0, 1, v2);
        (d.iter().map(|x| x.code).collect(), s, d.iter().map(|x| x.margin_in_use).collect())
    }

    #[test]
    fn r1_one_open_position_per_underlying_side_expiry() {
        let (mut st, mut ast) = (Vec::new(), Vec::new());
        let (ce, pe) = (7 << 16, (7 << 16) | (1 << 15)); // the same underlying and expiry, CE and PE
        let h = 3_600;
        let evs = vec![
            ev(&mut st, &mut ast, 0, ce, 36_000, 1, 1_000_000, 500, h),          // 10:00 CE strike A, open to 11:00
            ev(&mut st, &mut ast, 0, ce, 36_900, 1, 1_000_000, 500, h),          // 10:15 the same contract again
            ev(&mut st, &mut ast, 1, ce, 37_800, 1, 1_000_000, 500, h),          // 10:30 CE strike B while A is open
            ev(&mut st, &mut ast, 2, pe, 37_800, 1, 1_000_000, 500, h),          // 10:30 the PE of the same underlying
            ev(&mut st, &mut ast, 1, ce, 39_600, 1, 1_000_000, 500, h),          // 11:00 CE strike B after A's last fill (11:00)
        ];
        let (c2, s2, _) = codes(&evs, &st, &ast, true);
        assert_eq!(c2, vec!["booked", "contract_open", "same_side_dup", "booked", "booked"]);
        assert_eq!(s2.trades, 3);
        // v1: strike B is booked beside A (the stacking the operator ruled out)
        let (c1, s1, _) = codes(&evs, &st, &ast, false);
        assert_eq!(c1, vec!["booked", "contract_open", "booked", "booked", "contract_open"]);
        assert_eq!(s1.trades, 3);
    }

    #[test]
    fn r4_margin_in_use_never_passes_20_lakh() {
        let (mut st, mut ast) = (Vec::new(), Vec::new());
        let slot = ACCOUNT / 5; // N 5: Rs 4 L
        let mut evs = Vec::new();
        // four positions of exactly one slot each (different underlyings), all open to 15:00
        for i in 0..4_u32 {
            evs.push(ev(&mut st, &mut ast, i, i << 16, 36_000 + i * 60, 2, slot / 2, 100, 18_000));
        }
        // a fifth that would need Rs 4 L + 2 paisa (lots forced past the slot): refused by the cap
        evs.push(ev(&mut st, &mut ast, 9, 9 << 16, 36_600, 2, slot / 2 + 1, 100, 600));
        // a sixth of exactly one slot: the margin in use reaches Rs 20 L exactly, allowed
        evs.push(ev(&mut st, &mut ast, 10, 10 << 16, 36_660, 2, slot / 2, 100, 600));
        let (c2, s2, m2) = codes(&evs, &st, &ast, true);
        assert_eq!(c2, vec!["booked", "booked", "booked", "booked", "margin_cap_20L", "booked"]);
        assert_eq!(s2.pk_margin, ACCOUNT);
        assert_eq!(m2, vec![slot, 2 * slot, 3 * slot, 4 * slot, 4 * slot, ACCOUNT]);
        // v1 refused the same trade through its cash rule (free = Rs 20 L + min(0, realized) - margin in use)
        let (c1, _, _) = codes(&evs, &st, &ast, false);
        assert_eq!(c1, vec!["booked", "booked", "booked", "booked", "margin_cash_short", "booked"]);
    }
}

impl Tops {
    /// REPAIR A (checkpoint): every view's kept accounts with their keys (heap order does not matter: `import` re-offers them).
    #[must_use]
    pub fn export(&self) -> (usize, Vec<Vec<((i64, i64, i64), Sum)>>) {
        (self.k, self.h.iter().map(|h| h.iter().map(|e| (e.0.k, e.0.s)).collect()).collect())
    }

    /// v3.4 (the shard kit, critic F2): every view's kept accounts with their keys, BEST FIRST (the `Keyed` total order): the canonical
    /// form of a batch file's kept entries (independent of the heaps' insertion history).
    #[must_use]
    pub fn canonical(&self) -> (usize, Vec<Vec<((i64, i64, i64), Sum)>>) {
        (self.k, self.h.iter().map(|h| {
            let mut x: Vec<Keyed> = h.iter().map(|e| e.0).collect();
            x.sort_by(|a, b| b.cmp(a));
            x.into_iter().map(|e| (e.k, e.s)).collect()
        }).collect())
    }

    /// v3.4 (`ckpt-merge`, E3): offers one kept entry to view `v` with its stored key (the same `push_view` as every offer: top-K under a
    /// strict total order, so the merge of per-batch top-Ks is the top-K of the union).
    pub fn offer_keyed(&mut self, v: usize, key: (i64, i64, i64), s: Sum) {
        // MUTANT m_kit_merge_tops_keeps_first_tie: an entry equal on the key alone never replaces the kept one (the total order ignored)
        if cfg!(feature = "m_kit_merge_tops_keeps_first_tie") {
            let k = self.k;
            if let Some(h) = self.h.get_mut(v) {
                if h.len() < k {
                    h.push(std::cmp::Reverse(Keyed { k: key, s }));
                } else if h.peek().is_some_and(|t| key > t.0.k) {
                    h.pop();
                    h.push(std::cmp::Reverse(Keyed { k: key, s }));
                }
            }
            return;
        }
        self.push_view(v, key, s);
    }

    /// v3.4: the number of views (the batch files' shape).
    #[must_use]
    pub fn nviews(&self) -> usize {
        self.h.len()
    }

    /// REPAIR A (checkpoint): heaps rebuilt from an export (the same keys and accounts: the same `best` order, the order is total).
    #[must_use]
    pub fn import(k: usize, views: &[Vec<((i64, i64, i64), Sum)>]) -> Self {
        let mut t = Self::new(k);
        for (v, list) in views.iter().enumerate() {
            for &(key, s) in list {
                t.push_view(v, key, s);
            }
        }
        t
    }
}
