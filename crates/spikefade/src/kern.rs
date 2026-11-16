//! v3 STAGE B (EXITS-DYNAMIC-DESIGN 3.5): the accounts of one event list for a set of exit settings: (setting, share level l, slot count N),
//! with the v2.1 account rules UNCHANGED (R1 one position per underlying + side + expiry, `contract_open`, `slots_full`, R4 the Rs 20 L cap,
//! the cash rule, intraday only; the drawdown on the realized net in close order). The only change is the exit: per event the account
//! reads its setting's rule anchors for its (l, q) row and takes the FIRST HIT (the minimum anchor, the tie order SL, TT, BE, TSL, TTP, TP,
//! GO, TIME); a setting whose SL is ABSENT at the event refuses it (REQ 47: no trade without its stop; `sl_unready`, counted).
//!
//! Account-major with EXACT EVENT SKIPPING (speed-up 2): an account with all N slots full and its next exit after the event refuses every
//! event up to that exit with its state unchanged, so it jumps ahead to the first list event at or after that exit (binary search over the
//! list's keys). `skip = false` walks every event (the plain kernel; both proven equal: `sf3_event_skip_exact`).
//!
//! Per account and year piece (the 21 windows from one sweep, `wf.rs`): net, trades, wins, max drawdown, the running peak and low, and the
//! cash-rule slack (the smallest slack of a passed cash check, the largest of a refused one) so a sliding window's fresh start can be proven
//! equal or re-walked exactly.

use std::cmp::Ordering;

use crate::acct::{SEv, Sum};
use crate::stage::{ABSENT, EvX, NEVER3, Tab3};
use crate::trade::{ACCOUNT, BASE_S, Out, Refuse};

/// One compiled setting: its rules in tie order as (tf-dependent?, index), the SL's position in that list.
#[derive(Clone, Debug, Default)]
pub struct CSet {
    /// (true = a tf-dependent rule: index into the tf row; false: a menu row index into the main row).
    pub rules: Vec<(bool, u8)>,
    /// The SL's position in `rules`.
    pub sl: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Pos {
    pub ci: u32,
    pub skey: u32,
    pub exit: u32,
    /// The position's margin (at most one slot <= Rs 20 L: i32 paisa) and its net (the outcome table's i32).
    pub margin: i32,
    pub net: i32,
    /// The year piece of the position's day (its realized net counts there).
    pub piece: u8,
    /// v3.1 item 4: units sold, value sold and 1 + the offset of the contract-day's minute closes in the table (0 = none).
    pub units: u32,
    pub sold: i64,
    pub mark1: u32,
    /// v3.1 item 3: units never closed (valued, not filled).
    pub nc_units: u32,
    /// v3.1 decision (c): the net at the gate stress.
    pub net_s2: i32,
    /// v3.4 CAP20 (matrix X20): the position's first trading minute (09:15 = 0: its decision's minute); its last minute and its loss bound
    /// are derived from `exit` and the marks (0 with the switch off; this field sits in the struct's padding).
    pub i0: u16,
}

/// Per year piece accumulators of one account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub net: i64,
    pub trades: u32,
    pub wins: u32,
    /// The max drawdown inside the piece (realized, from the piece's start).
    pub mdd: i64,
    /// The highest realized level inside the piece relative to its start (>= 0).
    pub up: i64,
    /// The lowest realized level inside the piece relative to its start (<= 0).
    pub low: i64,
    /// The smallest slack (realized - demand) of a PASSED cash check (i64::MAX: none).
    pub min_pass: i64,
    /// The largest slack of a REFUSED cash check (i64::MIN: none).
    pub max_ref: i64,
    /// REPAIR A (check-engine-2 S2 / M4, WF19): months of the piece at or above Rs 2,00,000 net, positive weeks counted in the piece of their
    /// first session, positive sessions.
    pub mo_ok: u16,
    pub wk_pos: u16,
    pub d_pos: u16,
    /// v3.4 CAP20 (matrix X20): the piece's trading minutes above Rs 20 L (margin in use + running MTM loss, minute-HIGH marks; saturating).
    /// It sits in the struct's padding (the piece keeps its v3.3 size); 0 with the switch off.
    pub cap: u16,
}

impl Default for Piece {
    fn default() -> Self {
        Self { net: 0, trades: 0, wins: 0, mdd: 0, up: 0, low: 0, min_pass: i64::MAX, max_ref: i64::MIN, mo_ok: 0, wk_pos: 0, d_pos: 0, cap: 0 }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Acc {
    pub nopen: usize,
    pub next_exit: u32,
    pub realized: i64,
    pub peak: i64,
    pub mdd: i64,
    pub trades: u32,
    pub wins: u32,
    pub worst: i64,
    pub worst_day: i64,
    pub day_net: i64,
    pub day_trades: u32,
    pub cur_day: u16,
    pub days_pos: u16,
    pub days_neg: u16,
    pub days_traded: u16,
    pub margin_open: i64,
    pub pk_margin: i64,
    /// Exit labels of the booked trades (counts by the winner's family rank; 8 = no own exit).
    pub labels: [u32; 9],
    pub sl_unready: u32,
    /// v3.1 item 5: the drawdown with same-second closes grouped (its peak beside).
    pub gpeak: i64,
    pub gmdd: i64,
    /// v3.1: the realized net at the gate stress; never-closed trades / units / net; MTM-only refusals.
    pub realized_s2: i64,
    pub nc_trades: u32,
    pub nc_units: u64,
    pub nc_net: i64,
    pub mtm_ref: u32,
    /// v3.2 addendum 15 (SEARCH15): the month and week verdict accumulators (the days the account saw, in time order): the current month /
    /// week id and its net; months seen / at or above Rs 2,00,000 / the worst; weeks seen / positive / the worst. Weeks and months with no
    /// event for the account are never seen (net 0: a failing month, counted by the report against the run's months).
    pub mo_cur: u16,
    pub mo_net: i64,
    pub mo_seen: u16,
    pub mo_ok: u16,
    pub mo_min: i64,
    pub wk_cur: u16,
    pub wk_net: i64,
    pub wk_seen: u16,
    pub wk_pos: u16,
    pub wk_min: i64,
    /// v3.4 CAP20 (matrix X20): 1 + the first and 1 + the last run day holding a trading minute whose margin in use + running MTM loss
    /// (minute-HIGH marks) passed Rs 20,00,000 (0 = none). Both sit in the struct's padding: the account state keeps its v3.3 size (the
    /// batch gate unchanged; `v34_cap20_state_sizes_unchanged`). The minutes themselves are counted per year piece (`Piece::cap`).
    pub cap_first1: u16,
    pub cap_last1: u16,
}

/// v3.4 CAP20: the per-minute cap's walk state of one account inside one day (positions never cross a day): the next minute to evaluate,
/// the minute and partial sum of the positions closed in it, the open positions' summed loss bounds, and the day's minutes above the cap
/// (the ghost trace carries it to every replayed class member).
#[derive(Clone, Copy, Debug)]
pub struct CapDay {
    pub next: u16,
    pub pmin: u16,
    pub part: i64,
    pub bound: i64,
    pub day: u16,
}

impl CapDay {
    /// A fresh day state.
    #[must_use]
    pub const fn new() -> Self {
        Self { next: 0, pmin: u16::MAX, part: 0, bound: 0, day: 0 }
    }
}

impl Default for CapDay {
    fn default() -> Self {
        Self::new()
    }
}

impl Acc {
    /// A fresh account.
    #[must_use]
    pub fn new() -> Self {
        Self { next_exit: u32::MAX, cur_day: u16::MAX, mo_cur: u16::MAX, wk_cur: u16::MAX, mo_min: i64::MAX, wk_min: i64::MAX, ..Self::default() }
    }
}

/// v3.2 addendum 15: the month minimum (Rs 2,00,000 net, paisa) and the run's day -> month / week ids (set once under SEARCH15).
pub const MONTH_MIN: i64 = 20_000_000;
pub static MONTH_OF: std::sync::OnceLock<Vec<u16>> = std::sync::OnceLock::new();
pub static WEEK_OF: std::sync::OnceLock<Vec<u16>> = std::sync::OnceLock::new();

#[cfg(test)]
thread_local! {
    /// REPAIR A (named tests): a test thread's own day -> month / week maps (the process-wide OnceLocks are never set by a test).
    pub static TEST_MAPS: std::cell::Cell<Option<(&'static [u16], &'static [u16])>> = const { std::cell::Cell::new(None) };
}

/// The run's day -> month / week maps (SEARCH15; a test thread's own maps first under cfg(test)).
#[inline]
fn maps() -> Option<(&'static [u16], &'static [u16])> {
    #[cfg(test)]
    if let Some(m) = TEST_MAPS.with(std::cell::Cell::get) {
        return Some(m);
    }
    Some((MONTH_OF.get()?.as_slice(), WEEK_OF.get()?.as_slice()))
}

/// v3.2: whether a month's net passes the minimum (at or above Rs 2,00,000).
#[inline]
#[must_use]
pub const fn month_ok(net: i64) -> bool {
    // MUTANT m_v32_month_verdict_strict: exactly Rs 2,00,000 failed (the minimum read as strictly above)
    if cfg!(feature = "m_v32_month_verdict_strict") { net > MONTH_MIN } else { net >= MONTH_MIN }
}

pub fn close_month(ac: &mut Acc) {
    if ac.mo_cur != u16::MAX {
        ac.mo_seen += 1;
        if month_ok(ac.mo_net) {
            ac.mo_ok += 1;
        }
        ac.mo_min = ac.mo_min.min(ac.mo_net);
    }
    if ac.wk_cur != u16::MAX {
        ac.wk_seen += 1;
        if ac.wk_net > 0 {
            ac.wk_pos += 1;
        }
        ac.wk_min = ac.wk_min.min(ac.wk_net);
    }
}

/// v3.2: adds a closed day's net to its month and week (closing the previous ones on a change).
fn verdict_day(ac: &mut Acc, di: u16, net: i64) {
    let Some((mo, wk)) = maps() else { return };
    let m = mo.get(usize::from(di)).copied().unwrap_or(u16::MAX);
    let w = wk.get(usize::from(di)).copied().unwrap_or(u16::MAX);
    verdict_ids(ac, m, w, net);
}

/// v3.2: the verdict step of one closed day in month `m` and week `w` (the named test reads it without the run's day maps).
pub fn verdict_ids(ac: &mut Acc, m: u16, w: u16, net: i64) {
    if m != ac.mo_cur {
        if ac.mo_cur != u16::MAX {
            ac.mo_seen += 1;
            if month_ok(ac.mo_net) {
                ac.mo_ok += 1;
            }
            ac.mo_min = ac.mo_min.min(ac.mo_net);
        }
        ac.mo_cur = m;
        ac.mo_net = 0;
    }
    if w != ac.wk_cur {
        if ac.wk_cur != u16::MAX {
            ac.wk_seen += 1;
            if ac.wk_net > 0 {
                ac.wk_pos += 1;
            }
            ac.wk_min = ac.wk_min.min(ac.wk_net);
        }
        ac.wk_cur = w;
        ac.wk_net = 0;
    }
    ac.mo_net += net;
    ac.wk_net += net;
}

/// The per-account piece state (empty = no windows). v3.1: the day map is no longer copied into every account (check-build-v3 D2: history
/// day-major held one copy of it per account); the walks read it from their caller.
#[derive(Clone, Debug, Default)]
pub struct Pieces {
    pub p: Vec<Piece>,
    /// v3.2-B (addendum 18; round B: the segment series, one allocation of about 850 bytes): the account's split state (None unless
    /// SPLITS18: 8 bytes and every hook skipped when off).
    pub ms: Option<Box<crate::split18::Ser18>>,
    /// The realized level at the start of the current piece.
    base: i64,
    cur: u8,
    on: bool,
    /// REPAIR A (WF19): the piece of the account's current day, of its current week's first session and of its current month.
    day_piece: u8,
    wk_piece: u8,
    mo_piece: u8,
}

impl Pieces {
    /// Piece accumulators for `n` pieces (windows on when the day map is not empty).
    #[must_use]
    pub fn new(of_day: &[u8], n: usize) -> Self {
        Self { p: vec![Piece::default(); n], ms: crate::split18::Ser18::boxed(), base: 0, cur: u8::MAX, on: !of_day.is_empty(), day_piece: 0, wk_piece: 0, mo_piece: 0 }
    }

    fn on(&self) -> bool {
        self.on
    }

    /// v3.1 item 5: the last close of a second under the grouped reading: the piece's levels and drawdown at the realized level after the group.
    fn group_end(&mut self, piece: u8, realized_after: i64) {
        if !self.on() || self.cur != piece {
            return;
        }
        if let Some(x) = self.p.get_mut(usize::from(piece)) {
            let lvl = realized_after - self.base;
            x.up = x.up.max(lvl);
            x.low = x.low.min(lvl);
            x.mdd = x.mdd.max(x.up - lvl);
        }
    }

    /// One close. `dd`: update the piece's levels and drawdown now (always under the booking order; under the grouped reading only at the
    /// last close of its second, v3.1 item 5).
    fn realize(&mut self, piece: u8, realized_after: i64, net: i64, dd: bool) {
        if !self.on() {
            return;
        }
        if self.cur != piece {
            self.base = realized_after - net;
            self.cur = piece;
        }
        if let Some(x) = self.p.get_mut(usize::from(piece)) {
            x.net += net;
            x.trades += 1;
            if net > 0 {
                x.wins += 1;
            }
            if dd {
                let lvl = realized_after - self.base;
                x.up = x.up.max(lvl);
                x.low = x.low.min(lvl);
                x.mdd = x.mdd.max(x.up - lvl);
            }
        }
    }

    fn cash(&mut self, piece: u8, slack: i64, passed: bool) {
        if !self.on() {
            return;
        }
        if let Some(x) = self.p.get_mut(usize::from(piece)) {
            if passed {
                x.min_pass = x.min_pass.min(slack);
            } else {
                x.max_ref = x.max_ref.max(slack);
            }
        }
    }
}

/// Whether the grouped drawdown reading is in force (v3.1 item 5).
#[inline]
fn grouped() -> bool {
    crate::sw::on(crate::sw::DD_GROUPED) && !cfg!(feature = "m_v31_dd_ungrouped")
}

/// Realizes every open position whose exit is known at or before `upto`, in exit order (v2.1's `flush`, the pieces beside). v3.1 item 5:
/// beside the booking-order drawdown (v2.1, per close) the drawdown on the equity at each close INSTANT (the closes of one second summed
/// first) is kept; the account's `mdd` reads the grouped one under the switch.
fn flush(ac: &mut Acc, p: &mut [Pos], upto: u32, pc: &mut Pieces) {
    let gr = grouped();
    // the second of the closes being realized (a group ends when the next close is in another second or none is due)
    let mut group: Option<(u32, u8)> = None;
    loop {
        let mut bi = usize::MAX;
        let mut be = u32::MAX;
        for (i, q) in p.iter().take(ac.nopen).enumerate() {
            if q.exit < be {
                be = q.exit;
                bi = i;
            }
        }
        let due = bi != usize::MAX && be <= upto;
        if let Some((g, piece)) = group
            && (!due || be != g)
        {
            ac.gpeak = ac.gpeak.max(ac.realized);
            ac.gmdd = ac.gmdd.max(ac.gpeak - ac.realized);
            if gr {
                pc.group_end(piece, ac.realized);
                if let Some(m) = pc.ms.as_deref_mut() {
                    m.group_end(g, ac.realized);
                }
            }
            group = None;
        }
        if !due {
            ac.next_exit = be;
            return;
        }
        let q = p.get(bi).copied().unwrap_or_default();
        let net = i64::from(q.net);
        ac.realized += net;
        ac.day_net += net;
        if net > 0 {
            ac.wins += 1;
        }
        ac.worst = ac.worst.min(net);
        ac.peak = ac.peak.max(ac.realized);
        ac.mdd = ac.mdd.max(ac.peak - ac.realized);
        ac.margin_open -= i64::from(q.margin);
        ac.realized_s2 += i64::from(q.net_s2);
        if q.nc_units > 0 {
            ac.nc_trades += 1;
            ac.nc_units += u64::from(q.nc_units);
            ac.nc_net += net;
        }
        let last = ac.nopen - 1;
        p.swap(bi, last);
        ac.nopen -= 1;
        pc.realize(q.piece, ac.realized, net, !gr);
        if let Some(m) = pc.ms.as_deref_mut() {
            m.close(q.exit, ac.realized, net, !gr);
        }
        group = Some((be, q.piece));
    }
}

/// v3.1 item 4: the unrealised LOSS of the open positions at decision second `t` (losses only): per position max(0, mark x units - value
/// sold), the mark = the option's last 1-minute close at or before T (the decision candle's close; forward filled). Uses the sale's final
/// units and value (conservative while the sale or a buy-back is still filling). O(open positions).
#[must_use]
pub fn unrealised(p: &[Pos], tab: &Tab3, t: u32) -> i64 {
    let b = (i64::from(t) - crate::cal::OPEN_S) / 60;
    if b < 1 {
        return 0;
    }
    let b = usize::try_from(b - 1).unwrap_or(0).min(crate::cal::NM - 1);
    let mut u = 0_i64;
    for q in p {
        if q.mark1 == 0 {
            continue;
        }
        let mark = tab.marks.get(q.mark1 as usize - 1 + b).copied().unwrap_or(0);
        if mark > 0 {
            u += (i64::from(mark) * i64::from(q.units) - q.sold).max(0);
        }
    }
    u
}

// ---- v3.4 CAP20 (matrix X20, standing F6 remainder; round C's kernel rule, ported): the Rs 20 L cap at EVERY trading minute -------------
//
// The rule (addendum 17, "hard at every instant"): per trading minute b (09:15 + b, b < 375) of a session, margin in use + running MTM loss
// = the sum over the positions open in minute b (from the decision's minute to the minute holding the last fill known) of (margin + max(0,
// mark(b) x units - value sold)), mark(b) = minute b's HIGH, else the forward-filled close (the X22 marks table). Counted in the kernel for
// EVERY account. Cost: O(1) per step when no breach is possible: a minute is final once an event of a later minute arrives (no later
// booking starts before it); final minutes are evaluated lazily right before a flush and at the day's end; a position closed in the current
// minute leaves its share of that minute in a one-minute partial sum. Before any evaluation the bound margin in use + the open positions'
// loss bounds (each the loss at the highest mark from its first minute to the day's end) + the partial sum is compared with Rs 20 L: at or
// below it no minute of the range can pass the cap and the range is skipped; otherwise the minutes are evaluated exactly, O(minutes x open
// positions), counted (`fix34::CAP_EXACT`, `fix34::CAP_MINUTES`). The count reads only the day's positions (never the carried realized net),
// so a D2-1 class member at or above R* has the ghost's count.

/// The trading minutes of a session (09:15-15:30).
pub const CAP_NM: u16 = 375;

/// Whether the per-minute cap is counted (v3.4 CAP20).
#[inline]
#[must_use]
pub fn cap_on() -> bool {
    crate::fix34::on(crate::fix34::CAP20)
}

/// The minute index (09:15 = 0) of a second of the day, clamped to [0, 375].
#[inline]
#[must_use]
pub fn cap_minute(sod: u32) -> u16 {
    u16::try_from(((i64::from(sod) - crate::cal::OPEN_S).max(0) / 60).min(i64::from(CAP_NM))).unwrap_or(CAP_NM)
}

/// The exclusive end minute of a position known at second `sod` (the minute holding it counts; 15:30 caps it).
#[inline]
#[must_use]
pub fn cap_end(sod: u32) -> u16 {
    let close = crate::cal::OPEN_S + 60 * i64::from(CAP_NM);
    u16::try_from(((i64::from(sod).min(close) - crate::cal::OPEN_S + 59).max(0) / 60).min(i64::from(CAP_NM))).unwrap_or(CAP_NM)
}

/// A position's exclusive end minute (from its exit key: day x 86,400 + second).
#[inline]
#[must_use]
pub fn cap_i1(q: &Pos) -> u16 {
    cap_end(q.exit % 86_400)
}

/// One position's running MTM loss in minute `b` (the marks table: minute HIGH, else the forward-filled close).
#[inline]
fn cap_loss(q: &Pos, tab: &Tab3, b: u16) -> i64 {
    if q.mark1 == 0 {
        return 0;
    }
    let mark = tab.marks.get(q.mark1 as usize - 1 + usize::from(b).min(crate::cal::NM - 1)).copied().unwrap_or(0);
    if mark <= 0 {
        return 0;
    }
    (i64::from(mark) * i64::from(q.units) - q.sold).max(0)
}

/// A booked position's loss bound: the loss at the highest minute mark from its first minute to the day's end (the suffix maxima).
#[inline]
#[must_use]
pub fn cap_peak(tab: &Tab3, q: &Pos) -> i64 {
    if q.mark1 == 0 {
        return 0;
    }
    // MUTANT m_v34_cap_bound_low: the bound read at the position's FIRST minute only (not the highest later mark): it can rule out a breach
    // that a later re-spike makes, so a minute above the cap goes uncounted
    let ix = q.mark1 as usize - 1 + usize::from(q.i0).min(crate::cal::NM - 1);
    let mark = if cfg!(feature = "m_v34_cap_bound_low") { tab.marks.get(ix).copied().unwrap_or(0) } else { tab.smax.get(ix).copied().unwrap_or(i32::MAX) };
    if mark <= 0 {
        return 0;
    }
    (i64::from(mark) * i64::from(q.units) - q.sold).max(0)
}

/// Evaluates the final minutes [next, upto) of the account's current day: skipped when the O(1) bound rules a breach out, else every minute
/// exactly; each minute above Rs 20 L is counted in the day's count `cd.day` (saturating; [`cap_day_end`] marks the account and its piece).
pub fn cap_eval(ac: &Acc, cd: &mut CapDay, p: &[Pos], tab: &Tab3, upto: u16) {
    let upto = upto.min(CAP_NM);
    if cd.next >= upto {
        return;
    }
    let part_in = cd.pmin != u16::MAX && cd.pmin >= cd.next && cd.pmin < upto;
    let bound = ac.margin_open + cd.bound + if part_in { cd.part } else { 0 };
    // MUTANT m_v34_cap_skip_always: the O(1) bound trusted as if it always ruled a breach out (no minute ever evaluated)
    if bound > ACCOUNT && !cfg!(feature = "m_v34_cap_skip_always") {
        crate::fix34::CAP_EXACT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        crate::fix34::CAP_MINUTES.fetch_add(u64::from(upto - cd.next), std::sync::atomic::Ordering::Relaxed);
        let open = p.get(..ac.nopen).unwrap_or(&[]);
        for b in cd.next..upto {
            let mut v = if part_in && b == cd.pmin { cd.part } else { 0 };
            for q in open {
                if q.i0 <= b && b < cap_i1(q) {
                    v += i64::from(q.margin) + cap_loss(q, tab, b);
                }
            }
            if v > ACCOUNT {
                cd.day = cd.day.saturating_add(1);
            }
        }
    }
    cd.next = upto;
    if cd.pmin != u16::MAX && cd.pmin < upto {
        cd.pmin = u16::MAX;
        cd.part = 0;
    }
}

/// CAP20: marks `n` (> 0) minutes above the cap on run day `di` in the account (its first / last such day) and in the piece of its day.
pub fn cap_mark(ac: &mut Acc, pc: &mut Pieces, di: u16, n: u16) {
    if n == 0 {
        return;
    }
    let d1 = di.saturating_add(1);
    if ac.cap_first1 == 0 {
        ac.cap_first1 = d1;
    }
    ac.cap_last1 = ac.cap_last1.max(d1);
    if pc.on
        && let Some(x) = pc.p.get_mut(usize::from(pc.day_piece))
    {
        x.cap = x.cap.saturating_add(n);
    }
}

/// Before a flush up to key `upto` in minute `m`: every position the flush will close that still holds minute `m` leaves its share of
/// minute `m` (margin + its loss in `m`) in the partial sum; its bound leaves the open sum.
pub fn cap_pre_close(ac: &Acc, cd: &mut CapDay, p: &[Pos], tab: &Tab3, upto: u32, m: u16) {
    for q in p.get(..ac.nopen).unwrap_or(&[]) {
        if q.exit > upto {
            continue;
        }
        cd.bound -= cap_peak(tab, q);
        // MUTANT m_v34_cap_drop_closed: a position closed inside a minute forgotten for that minute (its margin and loss left out)
        if q.i0 <= m && m < cap_i1(q) && !cfg!(feature = "m_v34_cap_drop_closed") {
            if cd.pmin != m {
                cd.pmin = m;
                cd.part = 0;
            }
            cd.part += i64::from(q.margin) + cap_loss(q, tab, m);
        }
    }
}

/// The day's end: every remaining minute of the account's current day evaluated (the open positions are then flushed by the caller), the
/// day's minutes above the cap marked in the account and the piece of its day ([`cap_mark`]), the day state reset. Returns the day's count
/// (the ghost trace carries it to every replayed class member).
pub fn cap_day_end(ac: &mut Acc, cd: &mut CapDay, p: &[Pos], tab: &Tab3, pc: &mut Pieces) -> u16 {
    if ac.cur_day == u16::MAX {
        *cd = CapDay::new();
        return 0;
    }
    cap_eval(ac, cd, p, tab, CAP_NM);
    let n = cd.day;
    let di = ac.cur_day;
    cap_mark(ac, pc, di, n);
    *cd = CapDay::new();
    n
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
    // v3.2 addendum 15: the month / week verdicts (read only under the SEARCH15 switch, which sets the day maps)
    if maps().is_some() {
        let (di, net) = (ac.cur_day, ac.day_net);
        verdict_day(ac, di, net);
    }
    ac.day_net = 0;
    ac.day_trades = 0;
}

/// REPAIR A (check-engine-2 S2 / M4): whether the per-piece month / week / session counters run (WF19, the run's month maps, windows on).
#[inline]
fn wf19_on(pc: &Pieces) -> bool {
    pc.on && crate::sw::on(crate::sw::WF19) && maps().is_some()
}

/// [`close_day`] with REPAIR A's piece counters: the closed session's positive count, and the month / week its verdict step closes counted
/// in the piece of that month / of that week's first session (WF19 only; otherwise exactly `close_day`).
pub fn close_day_p(ac: &mut Acc, pc: &mut Pieces) {
    if ac.cur_day == u16::MAX || !wf19_on(pc) {
        close_day(ac);
        return;
    }
    let (di, net) = (ac.cur_day, ac.day_net);
    if net > 0
        && let Some(x) = pc.p.get_mut(usize::from(pc.day_piece))
    {
        x.d_pos += 1;
    }
    let m = maps().and_then(|v| v.0.get(usize::from(di)).copied()).unwrap_or(u16::MAX);
    let w = maps().and_then(|v| v.1.get(usize::from(di)).copied()).unwrap_or(u16::MAX);
    if m != ac.mo_cur {
        if ac.mo_cur != u16::MAX
            && month_ok(ac.mo_net)
            && let Some(x) = pc.p.get_mut(usize::from(pc.mo_piece))
        {
            x.mo_ok += 1;
        }
        pc.mo_piece = pc.day_piece;
    }
    if w != ac.wk_cur {
        if ac.wk_cur != u16::MAX
            && ac.wk_net > 0
            && let Some(x) = pc.p.get_mut(usize::from(pc.wk_piece))
        {
            x.wk_pos += 1;
        }
        pc.wk_piece = pc.day_piece;
    }
    close_day(ac);
}

/// REPAIR A (WF19): at an account's end, its open month and week counted in their pieces (the account's own summary closes them on a copy).
pub fn finish_pieces(ac: &Acc, pc: &mut Pieces) {
    if !wf19_on(pc) {
        return;
    }
    if ac.mo_cur != u16::MAX
        && month_ok(ac.mo_net)
        && let Some(x) = pc.p.get_mut(usize::from(pc.mo_piece))
    {
        x.mo_ok += 1;
    }
    if ac.wk_cur != u16::MAX
        && ac.wk_net > 0
        && let Some(x) = pc.p.get_mut(usize::from(pc.wk_piece))
    {
        x.wk_pos += 1;
    }
}

/// The (row) data one account reads at one event: its main row and tf row indices.
#[inline]
fn rows_of(x: &EvX, l: usize, q: usize, slot: Option<usize>) -> (usize, Option<usize>) {
    (x.row(l, q), slot.map(|s| x.trow(s, l, q)))
}

/// The first hit of a setting at a path: (winner position, anchor, out index), or the SL ABSENT (Err).
#[inline]
fn decide(cs: &CSet, tab: &Tab3, ri: usize, tri: Option<usize>) -> Result<(Option<usize>, usize), ()> {
    let (Some(anc), Some(os), Some(&ob)) = (tab.anc.get(ri), tab.os.get(ri), tab.obase.get(ri)) else { return Err(()) };
    let tanc = tri.and_then(|t| tab.tanc.get(t));
    let tos = tri.and_then(|t| tab.tos.get(t));
    let read = |&(tfd, i): &(bool, u8)| -> (u16, u8) {
        if tfd {
            (tanc.and_then(|a| a.get(usize::from(i))).copied().unwrap_or(ABSENT), tos.and_then(|o| o.get(usize::from(i))).copied().unwrap_or(0))
        } else {
            (anc.get(usize::from(i)).copied().unwrap_or(ABSENT), os.get(usize::from(i)).copied().unwrap_or(0))
        }
    };
    if cs.rules.get(cs.sl).map(read).is_none_or(|x| x.0 == ABSENT) {
        return Err(());
    }
    let mut best = NEVER3;
    let mut bi = usize::MAX;
    let mut bslot = 0_u8;
    for (i, r) in cs.rules.iter().enumerate() {
        let (a, s) = read(r);
        let better = if cfg!(feature = "m_sf3_tie_tp_first") { a <= best && a < NEVER3 } else { a < best };
        if better {
            best = a;
            bi = i;
            bslot = s;
        }
    }
    Ok(((bi != usize::MAX).then_some(bi), ob as usize + usize::from(bslot)))
}

/// v3.3 D2-1 (DESIGN-D2 section 2, SPEED-PLAN S2): the GHOST trace of one (day sub-list, setting, share level, N): the day walked by the
/// very same rule cascade (`step3g::<true>`) with the lost-money term min(0, realized) read as 0, from a fresh day state (positions never
/// cross a day: every account starts each day with no position, no margin in use and no pending exit; Lemma A). Recorded: every booked
/// position with its booking key, the refusal counters the cascade bumps, the cash-check offsets (rel - d) and the threshold R*: an account
/// whose start-of-day realized net is at or above R* makes exactly the ghost's decisions (Lemma C), so its day is the ghost's trace replayed
/// through the real flush / pieces / segment-series hooks ([`replay_day`]).
#[derive(Clone, Debug)]
pub struct Trace {
    pub booked: Vec<(u32, Pos)>,
    pub sl_unready: u32,
    pub mtm_ref: u32,
    /// min over the passed cash checks of (rel - d); i64::MAX = none.
    pub cash_min_pass: i64,
    /// max over the recorded refused cash checks of (rel - d); i64::MIN = none.
    pub cash_max_ref: i64,
    /// The smallest start-of-day realized net whose real walk equals the trace (i64::MIN = every level).
    pub r_star: i64,
    /// The ghost walk's account-event steps.
    pub steps: u64,
    /// v3.4 CAP20: the ghost day's trading minutes above Rs 20 L (the count reads only the day's positions: every replayed class member has
    /// it; 0 with the switch off).
    pub cap_day: u16,
}

impl Default for Trace {
    fn default() -> Self {
        Self { booked: Vec::new(), sl_unready: 0, mtm_ref: 0, cash_min_pass: i64::MAX, cash_max_ref: i64::MIN, r_star: i64::MIN, steps: 0, cap_day: 0 }
    }
}

impl Trace {
    /// Empties the trace (the buffer kept).
    pub fn reset(&mut self) {
        self.booked.clear();
        self.sl_unready = 0;
        self.mtm_ref = 0;
        self.cash_min_pass = i64::MAX;
        self.cash_max_ref = i64::MIN;
        self.r_star = i64::MIN;
        self.steps = 0;
        self.cap_day = 0;
    }
}

/// What one account does with one event (v2.1's `step` with the v3 exit): (verdict code, lots, winner position).
#[allow(clippy::too_many_arguments)]
#[inline]
pub fn step3(ac: &mut Acc, p: &mut [Pos], e: &SEv, x: &EvX, tab: &Tab3, cs: &CSet, l: usize, ni: usize, tf: Option<u8>, v2: bool, pc: &mut Pieces, piece: u8) -> (&'static str, u16, Option<usize>) {
    let mut cd = CapDay::new();
    step3g::<false>(ac, p, e, x, tab, cs, l, ni, tf, v2, pc, piece, None, &mut cd)
}

/// v3.4: [`step3`] with the account's per-minute cap walk state (CAP20; untouched with the switch off): the walks keep one per account.
#[allow(clippy::too_many_arguments)]
#[inline]
pub fn step3c(ac: &mut Acc, p: &mut [Pos], e: &SEv, x: &EvX, tab: &Tab3, cs: &CSet, l: usize, ni: usize, tf: Option<u8>, v2: bool, pc: &mut Pieces, piece: u8, cd: &mut CapDay) -> (&'static str, u16, Option<usize>) {
    step3g::<false>(ac, p, e, x, tab, cs, l, ni, tf, v2, pc, piece, None, cd)
}

/// THE rule cascade of one account and one event (`step3`). v3.3: generic over GHOST (D2-1): with GHOST the lost-money term min(0, realized)
/// reads 0 and the trace records what the cascade decided (`g`); without it (every engine walk) the code is round B's `step3`, line for line.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn step3g<const GHOST: bool>(ac: &mut Acc, p: &mut [Pos], e: &SEv, x: &EvX, tab: &Tab3, cs: &CSet, l: usize, ni: usize, tf: Option<u8>, v2: bool, pc: &mut Pieces, piece: u8, mut g: Option<&mut Trace>, cd: &mut CapDay) -> (&'static str, u16, Option<usize>) {
    // the lost-money term of the cash / MTM rule: the ONLY read of carried state in the cascade (DESIGN-D2 Lemma B)
    // MUTANT m_v33_d2_ghost_reads_loss: the ghost reads the real lost-money term (its threshold R* then no longer bounds the replays)
    let lost = |r: i64| -> i64 { if GHOST && !cfg!(feature = "m_v33_d2_ghost_reads_loss") { 0 } else { r.min(0) } };
    let key = u32::from(e.di) * 86_400 + e.t;
    let t = &e.tab;
    if ac.cur_day != e.di {
        // v3.4 CAP20: the previous day's remaining minutes evaluated (and marked) before its positions are realized
        if cap_on() {
            let _ = cap_day_end(ac, cd, p, tab, pc);
        }
        flush(ac, p, u32::MAX, pc);
        close_day_p(ac, pc);
        ac.cur_day = e.di;
        pc.day_piece = piece;
    }
    if t.refuse != Refuse::None {
        return (t.refuse.code(), 0, None);
    }
    let lots = t.lots.get(ni).copied().unwrap_or(0);
    if lots == 0 {
        return ("margin_over_slot", 0, None);
    }
    // v3.2 addendum 14 (b)+(c): the last-60-second volume at the headline share must cover the WHOLE sale and the WHOLE buy-back of this
    // slot count's order: a contract that fails is refused, never half-entered (state unchanged, like a slot without a lot)
    if !t.cover_ok(ni) {
        return ("liq_cover", lots, None);
    }
    if ac.next_exit <= key {
        // v3.4 CAP20: the final minutes before this event's minute evaluated, then the closing positions' share of this minute kept
        if cap_on() {
            let m = cap_minute(e.t);
            cap_eval(ac, cd, p, tab, m);
            cap_pre_close(ac, cd, p, tab, key, m);
        }
        flush(ac, p, key, pc);
    }
    if v2 {
        let open = p.get(..ac.nopen).unwrap_or(&[]);
        if open.iter().any(|q| q.ci == e.ci) {
            return ("contract_open", lots, None);
        }
        if open.iter().any(|q| q.skey == e.skey) {
            return ("same_side_dup", lots, None);
        }
        if ac.nopen >= p.len() {
            return ("slots_full", lots, None);
        }
        // v3.4 SIDECAP (matrix X08, a dial; off unless MAIN's K is given): at most K open shorts on one side (CE or PE) per account. The
        // side is bit 15 of the R1 key; O(open positions) like the two scans above
        if crate::fix34::on(crate::fix34::SIDECAP) {
            let side = e.skey & 0x8000;
            let k = usize::try_from(crate::fix34::SIDE_K.load(std::sync::atomic::Ordering::Relaxed)).unwrap_or(usize::MAX);
            // MUTANT m_v34_sidecap_off: the dial read but never applied
            if open.iter().filter(|q| q.skey & 0x8000 == side).count() >= k && !cfg!(feature = "m_v34_sidecap_off") {
                return ("same_side_cap", lots, None);
            }
        }
    } else {
        if ac.nopen >= p.len() {
            return ("slots_full", lots, None);
        }
        if p.get(..ac.nopen).is_some_and(|o| o.iter().any(|q| q.ci == e.ci)) {
            return ("contract_open", lots, None);
        }
    }
    // v3.1 margin rule v2: the order's Rs 20 is blocked with the lots' margin (0 under rule v1 and the proxy)
    let margin = i64::from(lots) * t.margin_lot + t.addon;
    if v2 && ac.margin_open + margin > ACCOUNT {
        return ("margin_cap_20L", lots, None);
    }
    if crate::sw::on(crate::sw::MTM) && !cfg!(feature = "m_v31_mtm_off") {
        // v3.1 item 4 (F-C): available = Rs 20 L + min(0, realized) - margin in use - the open positions' unrealised loss. A check refused
        // for any realized level when D = demand + U > 0; otherwise its slack realized - D decides as v2.1's (the windows' fresh-start test).
        let u = unrealised(p.get(..ac.nopen).unwrap_or(&[]), tab, e.t);
        let d = ac.margin_open + margin - ACCOUNT + u;
        let free0 = ACCOUNT + lost(ac.realized) - ac.margin_open;
        if free0 - u < margin {
            if d <= 0 {
                pc.cash(piece, ac.realized - d, false);
                if let Some(m) = pc.ms.as_deref_mut() {
                    m.cash(e.di, ac.realized - d, false);
                }
                if GHOST && let Some(g) = g.as_deref_mut() {
                    g.cash_max_ref = g.cash_max_ref.max(ac.realized - d);
                }
            }
            if free0 >= margin {
                ac.mtm_ref += 1;
                if GHOST && let Some(g) = g.as_deref_mut() {
                    // the real account counts margin_mtm_short iff min(0, R + rel) >= margin + margin in use - Rs 20 L
                    g.mtm_ref += 1;
                    // MUTANT m_v33_d2_rstar_mtm_term_dropped: the MTM-refusal term of R* left out
                    if !cfg!(feature = "m_v33_d2_rstar_mtm_term_dropped") {
                        g.r_star = g.r_star.max(margin + ac.margin_open - ACCOUNT - ac.realized);
                    }
                }
                return ("margin_mtm_short", lots, None);
            }
            return ("margin_cash_short", lots, None);
        }
        if d <= 0 {
            pc.cash(piece, ac.realized - d, true);
            if let Some(m) = pc.ms.as_deref_mut() {
                m.cash(e.di, ac.realized - d, true);
            }
        }
        if GHOST && let Some(g) = g.as_deref_mut() {
            // passed (the ghost passes only with d <= 0): the real account passes iff min(0, R + rel) >= d, i.e. R >= d - rel
            g.cash_min_pass = g.cash_min_pass.min(ac.realized - d);
            g.r_star = g.r_star.max(d - ac.realized);
        }
    } else {
        let free = ACCOUNT + lost(ac.realized) - ac.margin_open;
        // the cash rule's slack: realized - (open + margin - ACCOUNT); refused iff slack < 0 (with R4 before it, demand <= 0)
        let slack = ac.realized - (ac.margin_open + margin - ACCOUNT);
        if free < margin {
            pc.cash(piece, slack, false);
            if let Some(m) = pc.ms.as_deref_mut() {
                m.cash(e.di, slack, false);
            }
            if GHOST && let Some(g) = g.as_deref_mut() {
                g.cash_max_ref = g.cash_max_ref.max(slack);
                // a ghost refusal here is refused for every R (min(0, .) <= 0): no R* term
            }
            return ("margin_cash_short", lots, None);
        }
        pc.cash(piece, slack, true);
        if let Some(m) = pc.ms.as_deref_mut() {
            m.cash(e.di, slack, true);
        }
        if GHOST && let Some(g) = g.as_deref_mut() {
            // the real account passes iff min(0, R + rel) >= open + margin - Rs 20 L = rel - slack
            g.cash_min_pass = g.cash_min_pass.min(slack);
            g.r_star = g.r_star.max(-slack);
        }
    }
    if !t.filled.get(l).copied().unwrap_or(false) {
        return ("not_filled", lots, None);
    }
    let q = usize::from(t.q_of.get(ni).copied().unwrap_or(255));
    let slot = match tf {
        Some(tfid) => match x.slot_of(tfid) {
            Some(s) => Some(s),
            None => return ("no_tf_slot", lots, None),
        },
        None => None,
    };
    let (ri, tri) = rows_of(x, l, q, slot);
    let Ok((win, oi)) = decide(cs, tab, ri, tri) else {
        ac.sl_unready += 1;
        if GHOST && let Some(g) = g.as_deref_mut() {
            g.sl_unready += 1;
        }
        return ("sl_unready", lots, None);
    };
    let Some(o) = tab.outs.get(oi).copied() else { return ("not_filled", lots, win) };
    if o.flag & 1 == 0 {
        return ("not_filled", lots, win);
    }
    let ex = u32::from(e.di) * 86_400 + u32::try_from(i64::from(o.exit) + BASE_S).unwrap_or(0);
    // v3.4 CAP20: the position's first trading minute (its decision's minute; 0 with the switch off) and its loss bound into the open sum
    let cap = cap_on();
    let pos = Pos { ci: e.ci, skey: e.skey, exit: ex, margin: i32::try_from(margin).unwrap_or(i32::MAX), net: o.net, piece, units: o.units, sold: o.sold, mark1: x.mark1, nc_units: o.nc_units, net_s2: o.net_s2, i0: if cap { cap_minute(e.t) } else { 0 } };
    if cap {
        cd.bound += cap_peak(tab, &pos);
    }
    if let Some(sl) = p.get_mut(ac.nopen) {
        *sl = pos;
    }
    if GHOST && let Some(g) = g.as_deref_mut() {
        g.booked.push((key, pos));
    }
    ac.nopen += 1;
    ac.margin_open += margin;
    ac.pk_margin = ac.pk_margin.max(ac.margin_open);
    ac.next_exit = ac.next_exit.min(ex);
    ac.trades += 1;
    ac.day_trades += 1;
    ("booked", lots, win)
}

/// The out of a booked decision (detail lists): the same `decide` as the sweep.
#[must_use]
pub fn out_of_decision(e: &SEv, x: &EvX, tab: &Tab3, cs: &CSet, l: usize, ni: usize, tf: Option<u8>) -> Option<Out> {
    let q = usize::from(e.tab.q_of.get(ni).copied().unwrap_or(255));
    let slot = match tf {
        Some(tfid) => Some(x.slot_of(tfid)?),
        None => None,
    };
    let (ri, tri) = rows_of(x, l, q, slot);
    decide(cs, tab, ri, tri).ok().and_then(|(_, oi)| tab.outs.get(oi).copied())
}

/// The slot capacity of slot index `ni` (v3.1's grid).
pub const CAP: [usize; 4] = [5, 10, 20, 40];

/// REPAIR A (check-engine-2 S3): the largest slot count of any grid (5..100).
pub const MAXCAP: usize = 100;

/// REPAIR A: the capacity (slot count N) of slot index `ni` for the segment of a list (`seg` from the list's events; v3.1's CAP unless the
/// NGRID switch is on).
#[inline]
#[must_use]
pub fn cap_at(seg: u8, ni: usize) -> usize {
    usize::try_from(crate::sw::n_slots(seg, ni)).unwrap_or(5).clamp(1, MAXCAP)
}

/// The segment of a list (its first event's; 0 for an empty list).
#[inline]
#[must_use]
pub fn seg_of(evs: &[SEv], list: &[u32]) -> u8 {
    list.first().and_then(|&i| evs.get(i as usize)).map_or(0, |e| e.tab.seg)
}

/// One account's walk over a list (account-major; `skip` = exact event skipping). Returns its summary (v2.1 `Sum`; `x` = setting slot x 5
/// + l) and its pieces.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn walk_account(evs: &[SEv], ex: &[EvX], tab: &Tab3, list: &[u32], keys: &[u32], cs: &CSet, l: usize, ni: usize, tf: Option<u8>, v2: bool, skip: bool, of_day: &[u8], npieces: usize, ndays: u16) -> (Sum, Pieces, u64) {
    let cap = cap_at(seg_of(evs, list), ni);
    let mut ac = Acc::new();
    let mut p = vec![Pos::default(); cap];
    let mut pc = Pieces::new(of_day, npieces);
    let mut i = 0_usize;
    let mut steps = 0_u64;
    // v3.4 CAP20: the account's per-minute cap walk state (untouched with the switch off)
    let mut cd = CapDay::new();
    while let Some(&eid) = list.get(i) {
        let Some(e) = evs.get(eid as usize) else {
            i += 1;
            continue;
        };
        let Some(x) = ex.get(eid as usize) else {
            i += 1;
            continue;
        };
        steps += 1;
        let piece = of_day.get(usize::from(e.di)).copied().unwrap_or(0);
        let _ = step3c(&mut ac, &mut p, e, x, tab, cs, l, ni, tf, v2, &mut pc, piece, &mut cd);
        // exact skip: all slots full and the next exit after the NEXT event: jump to the first event at or after the next exit
        if skip && ac.nopen >= cap && ac.next_exit != u32::MAX {
            let target = if cfg!(feature = "m_sf3_skip_wrong") { ac.next_exit.saturating_add(1) } else { ac.next_exit };
            let rest = keys.get(i + 1..).unwrap_or(&[]);
            let j = rest.partition_point(|&k| k < target);
            // only events of the SAME day can be skipped (a day change flushes); next_exit is inside the day, so they all are
            i += 1 + j;
        } else {
            i += 1;
        }
    }
    if cap_on() {
        let _ = cap_day_end(&mut ac, &mut cd, &p, tab, &mut pc);
    }
    flush(&mut ac, &mut p, u32::MAX, &mut pc);
    close_day_p(&mut ac, &mut pc);
    finish_pieces(&ac, &mut pc);
    let s = sum_of(&ac, ni, ndays);
    (s, pc, steps)
}

/// An account's summary (v2.1 `Sum` + the v3.1 fields; `mdd` = the grouped drawdown under item 5's switch, the other beside).
#[must_use]
pub fn sum_of(ac: &Acc, ni: usize, ndays: u16) -> Sum {
    let (mdd, alt) = if grouped() { (ac.gmdd, ac.mdd) } else { (ac.mdd, ac.gmdd) };
    // v3.2 addendum 15: the open month / week closed on a copy (the account itself is not changed by a summary)
    let mut v = ac.clone();
    close_month(&mut v);
    Sum { seg: 0, list: 0, x: 0, n: u8::try_from(ni).unwrap_or(0), net: ac.realized, trades: ac.trades, wins: ac.wins, mdd, worst: ac.worst, worst_day: ac.worst_day, days_pos: ac.days_pos, days_neg: ac.days_neg, days_traded: ac.days_traded, ndays, spike: false, pk_margin: ac.pk_margin, mdd_alt: alt, net_s2: ac.realized_s2, nc_trades: ac.nc_trades, nc_units: ac.nc_units, nc_net: ac.nc_net, mtm_ref: ac.mtm_ref, tie: 0, mo_seen: v.mo_seen, mo_ok: v.mo_ok, mo_min: if v.mo_seen == 0 { 0 } else { v.mo_min }, wk_seen: v.wk_seen, wk_pos: v.wk_pos, wk_min: if v.wk_seen == 0 { 0 } else { v.wk_min }, sp: crate::split18::SpStat::default(), cap_first1: ac.cap_first1, cap_last1: ac.cap_last1 }
}

/// One decision of a detailed v3 run.
#[derive(Clone, Copy, Debug)]
pub struct Dec3 {
    pub eid: u32,
    pub code: &'static str,
    pub lots: u16,
    pub win: Option<usize>,
    pub margin_in_use: i64,
    pub nopen: usize,
}

/// One account's detailed run (the very same `step3`, no skipping): every decision and the summary.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn detail3(evs: &[SEv], ex: &[EvX], tab: &Tab3, list: &[u32], cs: &CSet, l: usize, ni: usize, tf: Option<u8>, v2: bool, ndays: u16) -> (Vec<Dec3>, Sum, Acc) {
    let cap = cap_at(seg_of(evs, list), ni);
    let mut ac = Acc::new();
    let mut p = vec![Pos::default(); cap];
    let mut pc = Pieces::default();
    let mut out = Vec::new();
    // v3.4 CAP20: one walk state for the account (the detail never reads the marks; untouched with the switch off)
    let mut cd = CapDay::new();
    for &eid in list {
        let (Some(e), Some(x)) = (evs.get(eid as usize), ex.get(eid as usize)) else { continue };
        let (code, lots, win) = step3c(&mut ac, &mut p, e, x, tab, cs, l, ni, tf, v2, &mut pc, 0, &mut cd);
        out.push(Dec3 { eid, code, lots, win, margin_in_use: ac.margin_open, nopen: ac.nopen });
    }
    flush(&mut ac, &mut p, u32::MAX, &mut pc);
    close_day(&mut ac);
    let s = sum_of(&ac, ni, ndays);
    (out, s, ac)
}

/// The keys (day x 86,400 + T) of a list's events (sorted: lists are in time order).
#[must_use]
pub fn list_keys(evs: &[SEv], list: &[u32]) -> Vec<u32> {
    list.iter().map(|&i| evs.get(i as usize).map_or(0, |e| u32::from(e.di) * 86_400 + e.t)).collect()
}

/// DAY-MAJOR: one account advanced through one day's events of its list (`sub`, with their keys), its state carried in `ac` / `pc`. At the
/// day's end every open position is realized (exact: positions never cross a day, and the next event's day change would realize them the
/// same way, in exit order). Returns the account-event steps.
#[allow(clippy::too_many_arguments)]
pub fn walk_day(evs: &[SEv], ex: &[EvX], tab: &Tab3, sub: &[u32], subk: &[u32], cs: &CSet, l: usize, ni: usize, tf: Option<u8>, v2: bool, skip: bool, ac: &mut Acc, pc: &mut Pieces, of_day: &[u8]) -> u64 {
    let cap = cap_at(seg_of(evs, sub), ni);
    // positions never cross a day: a fixed stack buffer (no allocation per account and day)
    let mut buf = [Pos::default(); MAXCAP];
    let p = buf.get_mut(..cap).unwrap_or(&mut []);
    let mut i = 0_usize;
    let mut steps = 0_u64;
    // v3.4 CAP20: the account-day's per-minute cap walk state (positions never cross a day; untouched with the switch off)
    let mut cd = CapDay::new();
    while let Some(&eid) = sub.get(i) {
        let (Some(e), Some(x)) = (evs.get(eid as usize), ex.get(eid as usize)) else {
            i += 1;
            continue;
        };
        // an event refused at event level, or with no lot at this slot count, on the account's current day: passed with the state unchanged
        // (v2.1's sweep_list reading, as the chunk kernel)
        if ac.cur_day == e.di && (e.tab.refuse != Refuse::None || e.tab.lots.get(ni).copied().unwrap_or(0) == 0) {
            i += 1;
            continue;
        }
        steps += 1;
        let piece = of_day.get(usize::from(e.di)).copied().unwrap_or(0);
        let _ = step3c(ac, p, e, x, tab, cs, l, ni, tf, v2, pc, piece, &mut cd);
        if skip && ac.nopen >= cap && ac.next_exit != u32::MAX {
            let rest = subk.get(i + 1..).unwrap_or(&[]);
            i += 1 + rest.partition_point(|&k| k < ac.next_exit);
        } else {
            i += 1;
        }
    }
    if cap_on() {
        let _ = cap_day_end(ac, &mut cd, p, tab, pc);
    }
    flush(ac, p, u32::MAX, pc);
    steps
}

/// v3.3 (DESIGN-D3 3.5, switch HOIST): [`walk_day`] with the position buffer given by the caller (one per worker thread instead of a
/// zero-filled stack array per call). Exact: positions never cross a day (`walk_day` ends each day with a full flush, so `nopen` is 0 at
/// every call) and a slot at or beyond `nopen` is never read before it is written. Everything else is `walk_day`'s code.
#[allow(clippy::too_many_arguments)]
pub fn walk_day_h(evs: &[SEv], ex: &[EvX], tab: &Tab3, sub: &[u32], subk: &[u32], cs: &CSet, l: usize, ni: usize, tf: Option<u8>, v2: bool, skip: bool, ac: &mut Acc, pc: &mut Pieces, of_day: &[u8], buf: &mut [Pos]) -> u64 {
    let cap = cap_at(seg_of(evs, sub), ni);
    // MUTANT m_v33_hoist_whole_buffer: the thread's whole buffer (MAXCAP slots) used instead of this slot count's `cap` slots
    let p = if cfg!(feature = "m_v33_hoist_whole_buffer") { buf } else { buf.get_mut(..cap).unwrap_or(&mut []) };
    walk_day_core::<false>(evs, ex, tab, sub, subk, cs, l, ni, tf, v2, skip, ac, pc, of_day, p, cap, None)
}

/// v3.3: `walk_day`'s loop over one day's sub-list (round B's lines: the refused / no-lot pass on the account's day, the exact skip, the
/// end-of-day flush), generic over GHOST (D2-1: the cascade records its trace). `p` = the account's `cap` position slots.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn walk_day_core<const GHOST: bool>(evs: &[SEv], ex: &[EvX], tab: &Tab3, sub: &[u32], subk: &[u32], cs: &CSet, l: usize, ni: usize, tf: Option<u8>, v2: bool, skip: bool, ac: &mut Acc, pc: &mut Pieces, of_day: &[u8], p: &mut [Pos], cap: usize, mut g: Option<&mut Trace>) -> u64 {
    let mut i = 0_usize;
    let mut steps = 0_u64;
    // v3.4 CAP20: the account-day's per-minute cap walk state (untouched with the switch off)
    let mut cd = CapDay::new();
    while let Some(&eid) = sub.get(i) {
        let (Some(e), Some(x)) = (evs.get(eid as usize), ex.get(eid as usize)) else {
            i += 1;
            continue;
        };
        if ac.cur_day == e.di && (e.tab.refuse != Refuse::None || e.tab.lots.get(ni).copied().unwrap_or(0) == 0) {
            i += 1;
            continue;
        }
        steps += 1;
        let piece = of_day.get(usize::from(e.di)).copied().unwrap_or(0);
        let _ = step3g::<GHOST>(ac, p, e, x, tab, cs, l, ni, tf, v2, pc, piece, g.as_deref_mut(), &mut cd);
        if skip && ac.nopen >= cap && ac.next_exit != u32::MAX {
            let rest = subk.get(i + 1..).unwrap_or(&[]);
            i += 1 + rest.partition_point(|&k| k < ac.next_exit);
        } else {
            i += 1;
        }
    }
    if cap_on() {
        let n = cap_day_end(ac, &mut cd, p, tab, pc);
        // D2-1: the ghost day's count rides on the trace (it reads only the day's positions: every class member at or above R* has it)
        if GHOST && let Some(t) = g.as_deref_mut() {
            t.cap_day = n;
        }
    }
    flush(ac, p, u32::MAX, pc);
    steps
}

/// v3.3 D2-1: the GHOST walk of one day sub-list for one account slot (setting, share level, N): `walk_day`'s loop and THE cascade
/// (`step3g::<true>`) on a fresh day state, recording the trace into `g` (reset first). Its throwaway account and pieces are never offered.
#[allow(clippy::too_many_arguments)]
pub fn ghost_day(evs: &[SEv], ex: &[EvX], tab: &Tab3, sub: &[u32], subk: &[u32], cs: &CSet, l: usize, ni: usize, tf: Option<u8>, skip: bool, of_day: &[u8], buf: &mut [Pos], g: &mut Trace) {
    g.reset();
    let cap = cap_at(seg_of(evs, sub), ni);
    let p = buf.get_mut(..cap).unwrap_or(&mut []);
    let mut ac = Acc::new();
    let mut pc = Pieces::default();
    let s = walk_day_core::<true>(evs, ex, tab, sub, subk, cs, l, ni, tf, true, skip, &mut ac, &mut pc, of_day, p, cap, Some(&mut *g));
    g.steps = s;
}

/// v3.3 D2-1: replays ghost trace `g` of day `di` on a REAL account whose start-of-day realized net is at or above the trace's R*: `step3`'s
/// day change, then every booked position in order, each placed after the flush `step3` makes at its booking key, the refusal counters,
/// the cash-check slacks (R + the recorded offsets), the day's end flush. Every realization goes through the real `flush`, so the pieces,
/// the grouped drawdown and the segment series (SPLITS18) receive exactly the walk's sequence (DESIGN-D2 Lemma D). Returns false, with the
/// account untouched, when R < R* (the caller walks the day). `buf` = a reusable slot buffer (slots at or beyond `nopen` never read before
/// written).
pub fn replay_day(ac: &mut Acc, pc: &mut Pieces, g: &Trace, di: u16, piece: u8, buf: &mut [Pos], cap: usize) -> bool {
    if ac.cur_day == di {
        return false;
    }
    // MUTANT m_v33_d2_no_rstar: every account replayed, whatever its start-of-day realized net
    if ac.realized < g.r_star && !cfg!(feature = "m_v33_d2_no_rstar") {
        return false;
    }
    let p = buf.get_mut(..cap).unwrap_or(&mut []);
    // step3's day change (no position is open at a day's start: the flush realizes nothing)
    flush(ac, p, u32::MAX, pc);
    close_day_p(ac, pc);
    ac.cur_day = di;
    pc.day_piece = piece;
    let r0 = ac.realized;
    for &(key, pos) in &g.booked {
        if ac.next_exit <= key {
            flush(ac, p, key, pc);
        }
        if let Some(sl) = p.get_mut(ac.nopen) {
            *sl = pos;
        }
        ac.nopen += 1;
        ac.margin_open += i64::from(pos.margin);
        ac.pk_margin = ac.pk_margin.max(ac.margin_open);
        ac.next_exit = ac.next_exit.min(pos.exit);
        ac.trades += 1;
        ac.day_trades += 1;
    }
    ac.sl_unready += g.sl_unready;
    // MUTANT m_v33_d2_mtm_ref_dropped: the trace's MTM refusals not counted on the replayed account
    if !cfg!(feature = "m_v33_d2_mtm_ref_dropped") {
        ac.mtm_ref += g.mtm_ref;
    }
    // MUTANT m_v33_d2_cash_offsets_dropped: the cash-check slacks of the day not recorded in the pieces
    if !cfg!(feature = "m_v33_d2_cash_offsets_dropped") {
        if g.cash_min_pass != i64::MAX {
            pc.cash(piece, r0 + g.cash_min_pass, true);
            if let Some(m) = pc.ms.as_deref_mut() {
                m.cash(di, r0 + g.cash_min_pass, true);
            }
        }
        if g.cash_max_ref != i64::MIN {
            pc.cash(piece, r0 + g.cash_max_ref, false);
            if let Some(m) = pc.ms.as_deref_mut() {
                m.cash(di, r0 + g.cash_max_ref, false);
            }
        }
    }
    // v3.4 CAP20: the ghost day's minutes above Rs 20 L (they read only the day's positions, which the replay books unchanged)
    // MUTANT m_v34_cap_ghost_dropped: the trace's count not carried to the replayed account
    if cap_on() && !cfg!(feature = "m_v34_cap_ghost_dropped") {
        cap_mark(ac, pc, di, g.cap_day);
    }
    flush(ac, p, u32::MAX, pc);
    true
}

/// The summary of a day-major account after its last day.
#[must_use]
pub fn finish(ac: &mut Acc, ni: usize, ndays: u16) -> Sum {
    close_day(ac);
    sum_of(ac, ni, ndays)
}

/// REPAIR A: [`finish`] with the account's pieces (the WF19 piece counters of its last session, week and month).
#[must_use]
pub fn finish_p(ac: &mut Acc, pc: &mut Pieces, ni: usize, ndays: u16) -> Sum {
    close_day_p(ac, pc);
    finish_pieces(ac, pc);
    sum_of(ac, ni, ndays)
}

/// The decision code of one setting at one event of a list (speed-up 4): the out index chosen, `u32::MAX` = the SL absent, `u32::MAX - 1` = an
/// event every setting treats alike (refused at event level, no lot, the level not filled, no tf slot).
#[must_use]
pub fn decision_code(e: &SEv, x: &EvX, tab: &Tab3, cs: &CSet, l: usize, ni: usize, tf: Option<u8>) -> u32 {
    let t = &e.tab;
    if t.refuse != Refuse::None || t.lots.get(ni).copied().unwrap_or(0) == 0 || !t.filled.get(l).copied().unwrap_or(false) {
        return u32::MAX - 1;
    }
    let q = usize::from(t.q_of.get(ni).copied().unwrap_or(255));
    let slot = match tf {
        Some(tfid) => match x.slot_of(tfid) {
            Some(s) => Some(s),
            None => return u32::MAX - 1,
        },
        None => None,
    };
    let (ri, tri) = rows_of(x, l, q, slot);
    match decide(cs, tab, ri, tri) {
        Ok((_, oi)) => u32::try_from(oi).unwrap_or(u32::MAX - 2),
        Err(()) => u32::MAX,
    }
}

/// SPEED-UP 4 (exact setting dedup): settings whose decision codes agree on EVERY event of the list at (l, N) walk ONE account (their
/// accounts are identical: the account rules read only the chosen outcome). Grouped by `hash` of the code sequence, with a FULL compare
/// of the sequences on every hash match (a collision never merges two different settings). Returns each setting's representative.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn dedup_groups(evs: &[SEv], ex: &[EvX], tab: &Tab3, list: &[u32], csets: &[CSet], l: usize, ni: usize, tf: Option<u8>, hash: fn(&[u32]) -> u64) -> Vec<usize> {
    let seqs: Vec<Vec<u32>> = csets.iter().map(|cs| list.iter().map(|&i| match (evs.get(i as usize), ex.get(i as usize)) { (Some(e), Some(x)) => decision_code(e, x, tab, cs, l, ni, tf), _ => u32::MAX - 1 }).collect()).collect();
    let mut seen: std::collections::HashMap<u64, Vec<usize>> = std::collections::HashMap::new();
    let mut rep = vec![0_usize; csets.len()];
    for (k, s) in seqs.iter().enumerate() {
        let h = hash(s);
        let bucket = seen.entry(h).or_default();
        // MUTANT m_sf3_dedup_hash_only: a hash match is trusted without comparing the sequences
        let found = if cfg!(feature = "m_sf3_dedup_hash_only") { bucket.first().copied() } else { bucket.iter().copied().find(|&j| seqs.get(j) == Some(s)) };
        match found {
            Some(j) => {
                if let Some(r) = rep.get_mut(k) {
                    *r = j;
                }
            }
            None => {
                bucket.push(k);
                if let Some(r) = rep.get_mut(k) {
                    *r = k;
                }
            }
        }
    }
    rep
}

/// The default sequence hash (FNV-1a over the codes).
#[must_use]
pub fn fnv(s: &[u32]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for &x in s {
        h ^= u64::from(x);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// EVENT-MAJOR walk of a chunk of settings over one list (the stage B kernel of `run3`): per event, every account of the chunk (setting,
/// share level, slot count) in turn, so the event's rows stay hot; per account an exact SKIP key (all N slots full: every event before its
/// next exit is refused with its state unchanged, so it is passed by one compare: the same skipping as `walk_account`). A refused event or
/// a slot count with no lot on the account's current day is passed the same way (v2.1's `sweep_list` reading). Returns per account (index =
/// (setting x 5 + l) x 4 + N) its summary and pieces, and the account-event steps taken.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn walk_chunk(evs: &[SEv], ex: &[EvX], tab: &Tab3, list: &[u32], keys: &[u32], csets: &[CSet], tf: Option<u8>, v2: bool, skip: bool, of_day: &[u8], npieces: usize, ndays: u16) -> (Vec<(Sum, Pieces)>, u64) {
    walk_chunk_s(evs, ex, tab, list, keys, csets, tf, v2, skip, of_day, npieces, ndays, None)
}

/// [`walk_chunk`] with REPAIR A's sampled step timing (check-engine-2 S5): every 64th event, the wall time of that event's account steps and
/// their count go to `samples` (the stage B kernel's own steps only: no set-up, no summary).
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn walk_chunk_s(evs: &[SEv], ex: &[EvX], tab: &Tab3, list: &[u32], keys: &[u32], csets: &[CSet], tf: Option<u8>, v2: bool, skip: bool, of_day: &[u8], npieces: usize, ndays: u16, samples: Option<&mut Vec<(u64, u64)>>) -> (Vec<(Sum, Pieces)>, u64) {
    let per = NL_K * 4;
    let accs: Vec<AccId> = (0..csets.len() * per).map(|a| (u32::try_from(a / per).unwrap_or(0), u8::try_from((a % per) / 4).unwrap_or(0), u8::try_from(a % 4).unwrap_or(0))).collect();
    walk_accs(evs, ex, tab, list, keys, csets, &accs, tf, v2, skip, of_day, npieces, ndays, samples)
}

/// One account of a chunk walk: (setting index into the walk's settings, share level l, slot index ni).
pub type AccId = (u32, u8, u8);

/// EVENT-MAJOR walk of an explicit set of accounts over one list (the stage B kernel; [`walk_chunk`] = every (setting, l, N) of a chunk;
/// REPAIR A's exact setting dedup walks only the representatives): per event every account in turn, the exact skip key, refused events and
/// slot counts without a lot passed on the account's current day; share levels outside `--shares` are not walked. Returns per account (in
/// `accs` order) its summary and pieces, and the account-event steps.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn walk_accs(evs: &[SEv], ex: &[EvX], tab: &Tab3, list: &[u32], keys: &[u32], csets: &[CSet], accs: &[AccId], tf: Option<u8>, v2: bool, skip: bool, of_day: &[u8], npieces: usize, ndays: u16, mut samples: Option<&mut Vec<(u64, u64)>>) -> (Vec<(Sum, Pieces)>, u64) {
    let n = accs.len();
    let lseg = seg_of(evs, list);
    let mut acc: Vec<Acc> = vec![Acc::new(); n];
    let mut pcs: Vec<Pieces> = (0..n).map(|_| Pieces::new(of_day, npieces)).collect();
    let mut base = vec![0_usize; n];
    let mut tot = 0_usize;
    for (b, &(_, _, ni)) in base.iter_mut().zip(accs) {
        *b = tot;
        tot += cap_at(lseg, usize::from(ni));
    }
    let mut pos = vec![Pos::default(); tot];
    let mut until = vec![0_u32; n];
    // v3.4 CAP20: one per-minute cap walk state per account (untouched with the switch off)
    let mut cds = vec![CapDay::new(); n];
    let mut steps = 0_u64;
    for (i, &eid) in list.iter().enumerate() {
        let (Some(e), Some(x)) = (evs.get(eid as usize), ex.get(eid as usize)) else { continue };
        let key = keys.get(i).copied().unwrap_or(0);
        let piece = of_day.get(usize::from(e.di)).copied().unwrap_or(0);
        let refused = e.tab.refuse != Refuse::None;
        // MUTANT m_v32r_step_unsampled: no step block is ever sampled
        let timed = samples.is_some() && i % 64 == 0 && !cfg!(feature = "m_v32r_step_unsampled");
        let (t0, s0) = (if timed { Some(std::time::Instant::now()) } else { None }, steps);
        for (a, &(si, l, ni)) in accs.iter().enumerate() {
            let (si, l, ni) = (si as usize, usize::from(l), usize::from(ni));
            // v3.2: share levels outside `--shares` are not walked (their accounts are never offered)
            if !crate::trade::share_on(l) {
                continue;
            }
            let Some(ac) = acc.get_mut(a) else { continue };
            if ac.cur_day == e.di && (refused || e.tab.lots.get(ni).copied().unwrap_or(0) == 0) {
                continue;
            }
            if skip && key < until.get(a).copied().unwrap_or(0) {
                continue;
            }
            steps += 1;
            let cap = cap_at(lseg, ni);
            let b0 = base.get(a).copied().unwrap_or(0);
            let (Some(cs), Some(p), Some(pc), Some(cd)) = (csets.get(si), pos.get_mut(b0..b0 + cap), pcs.get_mut(a), cds.get_mut(a)) else { continue };
            let _ = step3c(ac, p, e, x, tab, cs, l, ni, tf, v2, pc, piece, cd);
            if skip && ac.nopen >= cap && ac.next_exit != u32::MAX {
                let target = if cfg!(feature = "m_sf3_skip_wrong") { ac.next_exit.saturating_add(1) } else { ac.next_exit };
                if let Some(u) = until.get_mut(a) {
                    *u = target;
                }
            }
        }
        if let (Some(t0), Some(sv)) = (t0, samples.as_deref_mut())
            && steps > s0
        {
            sv.push((u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX), steps - s0));
        }
    }
    let mut out = Vec::with_capacity(n);
    for (a, (ac, pc)) in acc.iter_mut().zip(pcs).enumerate() {
        let ni = accs.get(a).map_or(0, |x| usize::from(x.2));
        let cap = cap_at(lseg, ni);
        let b0 = base.get(a).copied().unwrap_or(0);
        let mut pc = pc;
        if let Some(p) = pos.get_mut(b0..b0 + cap) {
            if cap_on()
                && let Some(cd) = cds.get_mut(a)
            {
                let _ = cap_day_end(ac, cd, p, tab, &mut pc);
            }
            flush(ac, p, u32::MAX, &mut pc);
        }
        close_day_p(ac, &mut pc);
        finish_pieces(ac, &mut pc);
        let s = sum_of(ac, ni, ndays);
        out.push((s, pc));
    }
    (out, steps)
}

/// REPAIR A (check-engine-2 S5): the step-weighted p50 / p99 of sampled step blocks (ns per step x 1,000), and the number of blocks: each
/// block's per-step cost counts once per step it timed.
#[must_use]
pub fn step_weighted(blocks: &[(u64, u64)]) -> (u64, u64, usize) {
    let mut v: Vec<(u64, u64)> = blocks.iter().filter(|b| b.1 > 0).map(|&(ns, st)| (ns.saturating_mul(1_000) / st, st)).collect();
    if v.is_empty() {
        return (0, 0, 0);
    }
    v.sort_unstable();
    let total: u64 = v.iter().map(|x| x.1).sum();
    let at = |q: u64| -> u64 {
        let want = (total * q).div_ceil(100).max(1);
        let mut c = 0_u64;
        for &(x, w) in &v {
            c += w;
            if c >= want {
                return x;
            }
        }
        v.last().map_or(0, |x| x.0)
    };
    (at(50), at(99), v.len())
}

/// The share levels (for the chunk index arithmetic).
const NL_K: usize = crate::trade::NL;

/// REPAIR A: SPEED-UP 4 WIRED (exact setting dedup, `--dedup on`): per walked share level and slot count, the settings whose decision codes
/// agree on EVERY event of the list ([`dedup_groups`]: hash, then a FULL compare) walk ONE account; every other setting of the group gets
/// that account's summary and pieces (identical by construction: the account rules read only the chosen outcome). Returns the
/// [`walk_chunk`] layout over every setting ((setting x 5 + l) x 4 + N), the steps walked and the accounts walked.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn walk_dedup(evs: &[SEv], ex: &[EvX], tab: &Tab3, list: &[u32], keys: &[u32], csets: &[CSet], tf: Option<u8>, v2: bool, skip: bool, of_day: &[u8], npieces: usize, ndays: u16, samples: Option<&mut Vec<(u64, u64)>>) -> (Vec<(Sum, Pieces)>, u64, u64) {
    let per = NL_K * 4;
    let n = csets.len() * per;
    let mut rep_of = vec![usize::MAX; n];
    let mut accs: Vec<AccId> = Vec::new();
    for l in (0..NL_K).filter(|&l| crate::trade::share_on(l)) {
        for ni in 0..4 {
            let reps = dedup_groups(evs, ex, tab, list, csets, l, ni, tf, fnv);
            let mut pos_of_rep: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
            for (si, &r0) in reps.iter().enumerate() {
                // MUTANT m_v32r_dedup_wrong_rep: every setting mapped to the first setting's account
                let r = if cfg!(feature = "m_v32r_dedup_wrong_rep") { 0 } else { r0 };
                let k = *pos_of_rep.entry(r).or_insert_with(|| {
                    accs.push((u32::try_from(r).unwrap_or(0), u8::try_from(l).unwrap_or(0), u8::try_from(ni).unwrap_or(0)));
                    accs.len() - 1
                });
                if let Some(x) = rep_of.get_mut((si * NL_K + l) * 4 + ni) {
                    *x = k;
                }
            }
        }
    }
    let (res, steps) = walk_accs(evs, ex, tab, list, keys, csets, &accs, tf, v2, skip, of_day, npieces, ndays, samples);
    let walked = u64::try_from(accs.len()).unwrap_or(0);
    let mut out = Vec::with_capacity(n);
    for (a, &k) in rep_of.iter().enumerate() {
        match res.get(k) {
            Some(x) => out.push(x.clone()),
            None => out.push((sum_of(&Acc::new(), a % 4, ndays), Pieces::new(of_day, npieces))),
        }
    }
    (out, steps, walked)
}
