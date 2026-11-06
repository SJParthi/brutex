//! The accounts of the search (SEARCH-SPEC 6.4-6.6), the ranking (9.1), the streaming
//! top-K heaps (7.2) and the sign-flip luck stream (8.2). Integers only.
//!
//! `Fast` runs the six plain accounts of one combination in one pass over its kept
//! records (LONG, SHORT, BOTH and their FADE twins); `Book` is the one general account
//! with the account controls and trade output, used by the control stage, the picks and
//! the chains. A unit test holds `Fast` equal to `Book` with every control off.

use std::cmp::Ordering;

use crate::common::{N_MONTHS, SLOTS, UNSEEN_FIRST, draw};
use crate::spool::{Hot, HotTrade};

/// Rs 20,00,000 in paisa: C0 (the window's starting capital).
pub(crate) const C0: i128 = 200_000_000;

/// One month of an account: counts, money and the closed-equity path (end, low, high,
/// fall) over its exits in time order, the exits of one second applied together.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MonthAcc {
    pub(crate) trades: u32,
    pub(crate) wins: u32,
    pub(crate) net: i64,
    pub(crate) gross: i64,
    pub(crate) charges: i64,
    pub(crate) hold: i64,
    pub(crate) end: i64,
    pub(crate) low: i64,
    pub(crate) high: i64,
    pub(crate) fall: i64,
}

/// A 98-month row (2018-09 ..= 2026-10).
pub(crate) type Row = [MonthAcc; N_MONTHS];

/// An empty row.
pub(crate) const ROW0: Row = [MonthAcc { trades: 0, wins: 0, net: 0, gross: 0, charges: 0, hold: 0, end: 0, low: 0, high: 0, fall: 0 }; N_MONTHS];

/// The path join (PSE 2.4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Path {
    pub(crate) end: i128,
    pub(crate) low: i128,
    pub(crate) high: i128,
    pub(crate) fall: i128,
}

impl Path {
    pub(crate) fn join(self, m: &MonthAcc) -> Self {
        let (be, bl, bh, bf) = (i128::from(m.end), i128::from(m.low), i128::from(m.high), i128::from(m.fall));
        Self {
            end: self.end + be,
            high: self.high.max(self.end + bh),
            low: self.low.min(self.end + bl),
            fall: self.fall.max(bf).max(self.high - (self.end + bl)),
        }
    }
}

/// The ranking key of one recipe on a span of months (SEARCH-SPEC 9.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Key {
    /// 0 passes every month, 1 misses some, 2 no trade in any month.
    pub(crate) standing: u8,
    /// Months below +2.5% (a no-trade month is below).
    pub(crate) below: u16,
    /// The worst month's net over its start capital; `worst_d == 0`: a month started at
    /// capital <= 0 (worst possible).
    pub(crate) worst_n: i128,
    pub(crate) worst_d: i128,
    /// Max drawdown in tenths of a percent of C0, half up.
    pub(crate) dd: i64,
    pub(crate) fall: i64,
    pub(crate) net: i64,
    pub(crate) wins: u32,
    pub(crate) trades: u32,
    pub(crate) hold: i64,
    pub(crate) gross: i64,
    pub(crate) charges: i64,
    pub(crate) months: u16,
    /// The stable id (SEARCH-SPEC 9.1 key 8).
    pub(crate) id: u64,
}

/// The key of `row` on months `a..b`, a fresh Rs 20 lakh account at month `a`.
pub(crate) fn key_range(row: &Row, a: usize, b: usize, id: u64) -> Key {
    let mut k = Key { id, worst_n: 1, worst_d: 0, ..Key::default() };
    let mut cap = C0;
    let mut path = Path::default();
    let mut first = true;
    for m in row.get(a..b).unwrap_or(&[]) {
        let net = i128::from(m.net);
        let pass = m.trades > 0 && cap > 0 && 40 * net >= cap;
        if !pass {
            k.below += 1;
        }
        // worst month: net / cap, capital <= 0 is the worst possible
        if cap > 0 {
            if first || (k.worst_d > 0 && net * k.worst_d < k.worst_n * cap) {
                k.worst_n = net;
                k.worst_d = cap;
            }
        } else {
            k.worst_n = -1;
            k.worst_d = 0;
        }
        if cap <= 0 {
            k.worst_d = 0;
        }
        first = false;
        cap += net;
        k.trades += m.trades;
        k.wins += m.wins;
        k.net += m.net;
        k.gross += m.gross;
        k.charges += m.charges;
        k.hold += m.hold;
        path = path.join(m);
        k.months += 1;
    }
    k.fall = i64::try_from(path.fall).unwrap_or(i64::MAX);
    k.dd = i64::try_from((2_000 * path.fall + C0).div_euclid(2 * C0)).unwrap_or(i64::MAX);
    k.standing = if k.trades == 0 {
        2
    } else if k.below == 0 {
        0
    } else {
        1
    };
    k
}

/// The window prefixes (learning months from 2018-09): Y2020 .. Y2026 and HINDSIGHT.
/// FIXED's learning window is Y2023's (2018-09 ..= 2022-12, 52 months).
pub(crate) const PREFIX: [usize; 8] = [16, 28, 40, 52, 64, 76, 88, 98];
pub(crate) const N_WIN: usize = 8;
pub(crate) const WIN_NAMES: [&str; 8] = ["Y2020", "Y2021", "Y2022", "Y2023", "Y2024", "Y2025", "Y2026", "HINDSIGHT"];

/// The keys of every window prefix in one pass.
pub(crate) fn prefix_keys(row: &Row, id: u64) -> [Key; N_WIN] {
    let mut out = [Key::default(); N_WIN];
    let mut k = Key { id, worst_n: 1, worst_d: 0, ..Key::default() };
    let mut cap = C0;
    let mut path = Path::default();
    let mut w = 0_usize;
    for (mi, m) in row.iter().enumerate() {
        let net = i128::from(m.net);
        let pass = m.trades > 0 && cap > 0 && 40 * net >= cap;
        if !pass {
            k.below += 1;
        }
        if cap > 0 {
            if mi == 0 || (k.worst_d > 0 && net * k.worst_d < k.worst_n * cap) {
                k.worst_n = net;
                k.worst_d = cap;
            }
        } else {
            k.worst_n = -1;
            k.worst_d = 0;
        }
        cap += net;
        k.trades += m.trades;
        k.wins += m.wins;
        k.net += m.net;
        k.gross += m.gross;
        k.charges += m.charges;
        k.hold += m.hold;
        path = path.join(m);
        k.months += 1;
        if PREFIX.get(w).is_some_and(|p| *p == mi + 1) {
            let mut x = k;
            x.fall = i64::try_from(path.fall).unwrap_or(i64::MAX);
            x.dd = i64::try_from((2_000 * path.fall + C0).div_euclid(2 * C0)).unwrap_or(i64::MAX);
            x.standing = if x.trades == 0 {
                2
            } else if x.below == 0 {
                0
            } else {
                1
            };
            if let Some(o) = out.get_mut(w) {
                *o = x;
            }
            w += 1;
        }
    }
    out
}

/// THE RANKING: `Less` when `a` ranks above `b`.
pub(crate) fn cmp_key(a: &Key, b: &Key) -> Ordering {
    let o = a.standing.cmp(&b.standing);
    if o != Ordering::Equal {
        return o;
    }
    if a.standing == 2 {
        return a.id.cmp(&b.id);
    }
    if a.standing == 1 {
        let o = a.below.cmp(&b.below);
        if o != Ordering::Equal {
            return o;
        }
        // the highest worst month first
        let o = match (a.worst_d > 0, b.worst_d > 0) {
            (true, true) => (b.worst_n * a.worst_d).cmp(&(a.worst_n * b.worst_d)),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => Ordering::Equal,
        };
        if o != Ordering::Equal {
            return o;
        }
    }
    let o = a.dd.cmp(&b.dd);
    if o != Ordering::Equal {
        return o;
    }
    let o = b.net.cmp(&a.net);
    if o != Ordering::Equal {
        return o;
    }
    let (wa, ta, wb, tb) = (i128::from(a.wins), i128::from(a.trades), i128::from(b.wins), i128::from(b.trades));
    let o = (wb * ta).cmp(&(wa * tb));
    if o != Ordering::Equal {
        return o;
    }
    let o = a.trades.cmp(&b.trades);
    if o != Ordering::Equal {
        return o;
    }
    let o = (i128::from(a.hold) * tb).cmp(&(i128::from(b.hold) * ta));
    if o != Ordering::Equal {
        return o;
    }
    let o = (i128::from(b.net) * ta).cmp(&(i128::from(a.net) * tb));
    if o != Ordering::Equal {
        return o;
    }
    a.id.cmp(&b.id)
}

/// Every figure of two keys but the id is equal (the same result).
pub(crate) fn same_result(a: &Key, b: &Key) -> bool {
    a.standing == b.standing
        && a.below == b.below
        && a.worst_n == b.worst_n
        && a.worst_d == b.worst_d
        && a.dd == b.dd
        && a.fall == b.fall
        && a.net == b.net
        && a.wins == b.wins
        && a.trades == b.trades
        && a.hold == b.hold
        && a.gross == b.gross
        && a.charges == b.charges
        && a.months == b.months
}

/// One heap entry: the key on the heap's months and (once materialized) the recipe's
/// 98-month row. Phase B keeps keys only and materializes the survivors' rows at the
/// end of each timeframe run (a re-evaluation of each surviving recipe).
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) key: Key,
    pub(crate) row: Option<Box<Row>>,
}

/// A fixed-capacity top-K, best first, a key never held twice.
#[derive(Clone, Debug)]
pub(crate) struct TopK {
    pub(crate) cap: usize,
    pub(crate) v: Vec<Entry>,
}

impl TopK {
    pub(crate) fn new(cap: usize) -> Self {
        Self { cap, v: Vec::with_capacity(cap + 1) }
    }

    /// Would `k` enter?
    pub(crate) fn wants(&self, k: &Key) -> bool {
        self.v.len() < self.cap || self.v.last().is_some_and(|w| cmp_key(k, &w.key) == Ordering::Less)
    }

    pub(crate) fn offer(&mut self, k: &Key, row: Option<&Row>) {
        // Distinct results: an entry whose every ranked figure equals k's is the same
        // result (a filter the core implies, a control that never binds); the smaller
        // stable id is kept, so the heap is still exact whatever the order or split.
        if let Some(e) = self.v.iter_mut().find(|e| same_result(&e.key, k)) {
            if k.id < e.key.id {
                e.key = *k;
                e.row = row.map(|r| Box::new(*r));
                self.v.sort_by(|x, y| cmp_key(&x.key, &y.key));
            }
            return;
        }
        if !self.wants(k) || self.v.iter().any(|e| e.key.id == k.id) {
            return;
        }
        let at = self.v.partition_point(|e| cmp_key(&e.key, k) == Ordering::Less);
        self.v.insert(at, Entry { key: *k, row: row.map(|r| Box::new(*r)) });
        self.v.truncate(self.cap);
    }

    pub(crate) fn merge(&mut self, o: &Self) {
        for e in &o.v {
            self.offer(&e.key, e.row.as_deref());
        }
    }
}

/// The heaps of one timeframe run: `[window][scope]`, scope 0 = TF, 1..=6 = TF x SIDE.
#[derive(Clone, Debug)]
pub(crate) struct Heaps {
    pub(crate) h: Vec<TopK>,
}

pub(crate) const N_SCOPE_TF: usize = 7;

impl Heaps {
    pub(crate) fn new(cap: usize) -> Self {
        Self { h: (0..N_WIN * N_SCOPE_TF).map(|_| TopK::new(cap)).collect() }
    }

    pub(crate) fn get(&self, w: usize, s: usize) -> Option<&TopK> {
        self.h.get(w * N_SCOPE_TF + s)
    }

    /// Offer one row of side `side` to every window's TF and TF x SIDE heaps; `only`
    /// limits the offer to one window (the build-up).
    pub(crate) fn offer_row(&mut self, row: &Row, id: u64, side: usize, only: Option<usize>) {
        let keys = prefix_keys(row, id);
        for (w, k) in keys.iter().enumerate() {
            if only.is_some_and(|o| o != w) {
                continue;
            }
            for s in [0, 1 + side] {
                if let Some(h) = self.h.get_mut(w * N_SCOPE_TF + s) {
                    h.offer(k, None);
                }
            }
        }
    }

    pub(crate) fn merge(&mut self, o: &Self) {
        for (a, b) in self.h.iter_mut().zip(o.h.iter()) {
            a.merge(b);
        }
    }
}

/// The sign-flip test's flip masks (SEARCH-SPEC 8.2): 1,000 draws of 82 bits.
pub(crate) const N_DRAWS: usize = 1_000;
pub(crate) const LUCK_SEED: u64 = 20_261_106;

pub(crate) fn flips() -> Vec<u128> {
    (0..N_DRAWS)
        .map(|b| {
            let bb = u64::try_from(b).unwrap_or(0);
            let w0 = u128::from(draw(LUCK_SEED, bb, 0, 0x51));
            let w1 = u128::from(draw(LUCK_SEED, bb, 1, 0x51) & ((1_u64 << 18) - 1));
            w0 | (w1 << 64)
        })
        .collect()
}

/// The 82-month sign masks of a row: months with trades and `40 x net >= C0` (pos) or
/// `40 x net <= -C0` (neg), C0 = Rs 20 lakh for every month.
pub(crate) fn luck_masks(row: &Row) -> (u128, u128) {
    let (mut pos, mut neg) = (0_u128, 0_u128);
    for (i, m) in row.iter().skip(UNSEEN_FIRST).enumerate() {
        if m.trades == 0 {
            continue;
        }
        let n = 40 * i128::from(m.net);
        if n >= C0 {
            pos |= 1 << i;
        } else if n <= -C0 {
            neg |= 1 << i;
        }
    }
    (pos, neg)
}

/// The luck stream: `best[b]` = the largest months-passed of any series under draw b.
#[derive(Clone, Debug)]
pub(crate) struct Luck {
    pub(crate) best: Vec<u16>,
    min: u16,
    pub(crate) offered: u64,
    pub(crate) evaluated: u64,
}

impl Luck {
    pub(crate) fn new() -> Self {
        Self { best: vec![0; N_DRAWS], min: 0, offered: 0, evaluated: 0 }
    }

    pub(crate) fn offer(&mut self, flips: &[u128], pos: u128, neg: u128) {
        self.offered += 1;
        let active = u16::try_from((pos | neg).count_ones()).unwrap_or(u16::MAX);
        if active <= self.min {
            return;
        }
        self.evaluated += 1;
        let mut mn = u16::MAX;
        for (b, f) in self.best.iter_mut().zip(flips.iter()) {
            let s = u16::try_from(((pos & !f) | (neg & f)).count_ones()).unwrap_or(0);
            if s > *b {
                *b = s;
            }
            if *b < mn {
                mn = *b;
            }
        }
        self.min = mn;
    }

    pub(crate) fn offer_row(&mut self, flips: &[u128], row: &Row) {
        let (p, n) = luck_masks(row);
        self.offer(flips, p, n);
    }

    pub(crate) fn merge(&mut self, o: &Self) {
        for (a, b) in self.best.iter_mut().zip(o.best.iter()) {
            if *b > *a {
                *a = *b;
            }
        }
        self.min = self.best.iter().copied().min().unwrap_or(0);
        self.offered += o.offered;
        self.evaluated += o.evaluated;
    }
}

// ---------------------------------------------------------------------------
// The fast plain accounts.
// ---------------------------------------------------------------------------

/// One trade as the fast accounts read it (compact: one record is about one cache line).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MiniTrade {
    pub(crate) exit_us: i64,
    pub(crate) net: i64,
    pub(crate) gross: i32,
    pub(crate) charges: i32,
    pub(crate) hold: u32,
    pub(crate) status: u8,
    pub(crate) month: u8,
}

/// One record as the fast accounts read it, in the unit's position order.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Mini {
    pub(crate) t: i64,
    pub(crate) stock: u16,
    pub(crate) side: u8,
    pub(crate) tr: [MiniTrade; 2],
}

impl Mini {
    /// The compact view of a hot record; a gross or charge beyond i32 (never seen: a
    /// trade is Rs 5 lakh of exposure) is refused by name.
    pub(crate) fn of(h: &Hot) -> Result<Self, String> {
        let mut tr = [MiniTrade::default(); 2];
        for (d, s) in tr.iter_mut().zip(h.tr.iter()) {
            *d = MiniTrade {
                exit_us: s.exit_us,
                net: s.net,
                gross: i32::try_from(s.gross).map_err(|_| format!("gross {} beyond i32", s.gross))?,
                charges: i32::try_from(s.charges).map_err(|_| format!("charges {} beyond i32", s.charges))?,
                hold: s.hold,
                status: s.status,
                month: s.month,
            };
        }
        Ok(Self { t: h.t, stock: h.stock, side: h.side, tr })
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Pos {
    exit: i64,
    stock: u16,
    net: i64,
    gross: i64,
    charges: i64,
    hold: u32,
    month: u8,
}

/// One plain account (every control off): T1's `admit` order of checks, months and
/// the grouped closed-equity path.
#[derive(Clone, Debug)]
pub(crate) struct FastAcc {
    open: [Pos; SLOTS],
    n: usize,
    held: [u64; 4],
    next: i64,
    pend_sec: i64,
    pend_net: i64,
    pend_month: usize,
    has_pend: bool,
    pub(crate) row: Row,
}

impl FastAcc {
    pub(crate) fn new() -> Self {
        Self { open: [Pos::default(); SLOTS], n: 0, held: [0; 4], next: i64::MAX, pend_sec: 0, pend_net: 0, pend_month: 0, has_pend: false, row: ROW0 }
    }

    pub(crate) fn reset(&mut self) {
        self.n = 0;
        self.held = [0; 4];
        self.next = i64::MAX;
        self.has_pend = false;
        self.row = ROW0;
    }

    fn flush(&mut self) {
        if !self.has_pend {
            return;
        }
        self.has_pend = false;
        if let Some(m) = self.row.get_mut(self.pend_month) {
            let e = m.end + self.pend_net;
            m.end = e;
            if e > m.high {
                m.high = e;
            }
            if e < m.low {
                m.low = e;
            }
            if m.high - e > m.fall {
                m.fall = m.high - e;
            }
        }
    }

    /// The open positions are kept sorted by exit, latest first: the earliest exit is
    /// `open[n - 1]` and leaves without a scan.
    fn realize(&mut self, t: i64) {
        while self.n > 0 && self.next < t {
            self.n -= 1;
            let p = self.open[self.n];
            self.next = if self.n > 0 { self.open[self.n - 1].exit } else { i64::MAX };
            let w = usize::from(p.stock >> 6);
            if let Some(h) = self.held.get_mut(w) {
                *h &= !(1_u64 << (p.stock & 63));
            }
            let mi = usize::from(p.month);
            if let Some(m) = self.row.get_mut(mi) {
                m.trades += 1;
                if p.net > 0 {
                    m.wins += 1;
                }
                m.net += p.net;
                m.gross += p.gross;
                m.charges += p.charges;
                m.hold += i64::from(p.hold);
                if self.has_pend && self.pend_sec == p.exit && self.pend_month == mi {
                    self.pend_net += p.net;
                } else {
                    self.flush();
                    self.pend_sec = p.exit;
                    self.pend_net = p.net;
                    self.pend_month = mi;
                    self.has_pend = true;
                }
            }
        }
        self.flush();
    }

    #[inline]
    pub(crate) fn step(&mut self, t: i64, stock: u16, tr: &MiniTrade) {
        if self.next < t {
            self.realize(t);
        }
        let w = usize::from(stock >> 6);
        let bit = 1_u64 << (stock & 63);
        if self.held.get(w).is_some_and(|h| h & bit != 0) {
            return;
        }
        if self.n >= SLOTS {
            return;
        }
        if tr.status != 0 || tr.month == u8::MAX {
            return;
        }
        let pos = Pos { exit: tr.exit_us, stock, net: tr.net, gross: i64::from(tr.gross), charges: i64::from(tr.charges), hold: tr.hold, month: tr.month };
        let mut i = self.n;
        while i > 0 && self.open[i - 1].exit < pos.exit {
            self.open[i] = self.open[i - 1];
            i -= 1;
        }
        self.open[i] = pos;
        self.n += 1;
        self.next = self.open[self.n - 1].exit;
        if let Some(h) = self.held.get_mut(w) {
            *h |= bit;
        }
    }

    pub(crate) fn finish(&mut self) {
        self.realize(i64::MAX);
    }
}

/// The six plain accounts of one combination.
pub(crate) struct Fast {
    pub(crate) acc: Vec<FastAcc>,
}

impl Fast {
    pub(crate) fn new() -> Self {
        Self { acc: (0..6).map(|_| FastAcc::new()).collect() }
    }

    pub(crate) fn reset(&mut self) {
        for a in &mut self.acc {
            a.reset();
        }
    }

    /// One kept record: its side's WITH account, BOTH, its side's FADE account, FADE-BOTH.
    #[inline]
    pub(crate) fn step(&mut self, r: &Mini) {
        let s = usize::from(r.side);
        let [w, f] = &r.tr;
        if let Some(a) = self.acc.get_mut(s) {
            a.step(r.t, r.stock, w);
        }
        if let Some(a) = self.acc.get_mut(2) {
            a.step(r.t, r.stock, w);
        }
        if let Some(a) = self.acc.get_mut(3 + s) {
            a.step(r.t, r.stock, f);
        }
        if let Some(a) = self.acc.get_mut(5) {
            a.step(r.t, r.stock, f);
        }
    }

    pub(crate) fn finish(&mut self) {
        for a in &mut self.acc {
            a.finish();
        }
    }
}

// ---------------------------------------------------------------------------
// The account controls (SEARCH-SPEC 6.6; ADD-RISK; ADDENDUM 3 (1)).
// ---------------------------------------------------------------------------

/// One option set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Opt {
    /// DAY LOSS in bp of the month-start capital (0 off).
    pub(crate) day_loss: i128,
    pub(crate) month_loss: i128,
    pub(crate) target: i128,
    /// SIZE AFTER A FALL in bp of the peak (0 off).
    pub(crate) fall: i128,
    /// TRADE CAP: 0 off, 1 one per stock a day, 2 five a day, 3 ten a day, 4 ten a
    /// month, 5 twenty a month, 6 forty a month (the last three: ADDENDUM 3 (1)).
    pub(crate) cap: u8,
}

pub(crate) const DAY_LOSS: [i128; 4] = [0, 50, 100, 150];
pub(crate) const MONTH_LOSS: [i128; 4] = [0, 200, 400, 600];
pub(crate) const TARGET: [i128; 4] = [0, 250, 300, 400];
pub(crate) const FALL: [i128; 3] = [0, 500, 1_000];
pub(crate) const N_CAP: usize = 7;
/// 4 x 4 x 4 x 3 x 7 option sets; 0 = every control off.
pub(crate) const N_OPT: usize = 4 * 4 * 4 * 3 * N_CAP;

pub(crate) fn opt_of(o: usize) -> Opt {
    let cap = o % N_CAP;
    let r = o / N_CAP;
    let f = r % 3;
    let r = r / 3;
    let g = r % 4;
    let r = r / 4;
    let m = r % 4;
    let dl = r / 4;
    Opt {
        day_loss: DAY_LOSS.get(dl).copied().unwrap_or(0),
        month_loss: MONTH_LOSS.get(m).copied().unwrap_or(0),
        target: TARGET.get(g).copied().unwrap_or(0),
        fall: FALL.get(f).copied().unwrap_or(0),
        cap: u8::try_from(cap).unwrap_or(0),
    }
}

pub(crate) fn opt_words(o: usize) -> String {
    if o == 0 {
        return "no account control".to_owned();
    }
    let x = opt_of(o);
    let mut v: Vec<String> = Vec::new();
    if x.day_loss > 0 {
        v.push(format!("day loss limit -{}.{:02}%", x.day_loss / 100, x.day_loss % 100));
    }
    if x.month_loss > 0 {
        v.push(format!("month loss limit -{}%", x.month_loss / 100));
    }
    if x.target > 0 {
        v.push(format!("month target lock +{}.{:02}%", x.target / 100, x.target % 100));
    }
    if x.fall > 0 {
        v.push(format!("half size while more than {}% below the peak", x.fall / 100));
    }
    match x.cap {
        1 => v.push("one trade per stock a day".to_owned()),
        2 => v.push("at most 5 trades a day".to_owned()),
        3 => v.push("at most 10 trades a day".to_owned()),
        4 => v.push("at most 10 trades a month (strongest first at one instant)".to_owned()),
        5 => v.push("at most 20 trades a month (strongest first at one instant)".to_owned()),
        6 => v.push("at most 40 trades a month (strongest first at one instant)".to_owned()),
        _ => {}
    }
    v.join(", ")
}

/// Does the option set order one instant's records by strength (a monthly cap)?
pub(crate) fn by_strength(o: usize) -> bool {
    opt_of(o).cap >= 4
}

/// One trade a `Book` took.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Took {
    /// Index of the record in the kept list the book was fed.
    pub(crate) src: u32,
    pub(crate) half: bool,
    pub(crate) net: i64,
    pub(crate) gross: i64,
    pub(crate) charges: i64,
    pub(crate) exit: i64,
    pub(crate) month: u8,
    pub(crate) kind: u8,
}

#[derive(Clone, Copy, Debug, Default)]
struct BPos {
    exit: i64,
    stock: u16,
    net: i64,
    gross: i64,
    charges: i64,
    hold: u32,
    month: u8,
    day: u32,
    src: u32,
    half: bool,
    kind: u8,
}

/// What a book counted.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Counters {
    pub(crate) signals: u64,
    pub(crate) while_open: u64,
    pub(crate) slots_full: u64,
    pub(crate) not_filled: u64,
    pub(crate) zero_qty: u64,
    pub(crate) no_ratio: u64,
    /// Blocked by: day loss, month loss, target lock, trade cap.
    pub(crate) blocked: [u64; 4],
    pub(crate) half_trades: u64,
    pub(crate) half_zero: u64,
    /// Trades by exit kind: stop, flip, time, square-off, unclosable.
    pub(crate) exits: [u64; 5],
    pub(crate) gross_win: i128,
    pub(crate) gross_loss: i128,
    pub(crate) wins: u64,
    pub(crate) losses: u64,
}

/// One general account with the controls.
#[derive(Clone, Debug)]
pub(crate) struct Book {
    opt: Opt,
    pub(crate) eq: i128,
    pub(crate) peak: i128,
    cap_m: i128,
    month_key: u8,
    day_key: u32,
    day_net: i128,
    month_net: i128,
    day_trades: u32,
    month_trades: u32,
    day_stocks: [u64; 4],
    open: Vec<BPos>,
    held: [u64; 4],
    pend_sec: i64,
    pend_net: i64,
    pend_month: usize,
    has_pend: bool,
    pub(crate) row: Row,
    pub(crate) counters: Counters,
    pub(crate) took: Option<Vec<Took>>,
    /// Preview-style drawdown: equity marked after every exit, bp of the peak.
    pub(crate) peak_x: i128,
    pub(crate) dd_bp: i128,
    pub(crate) dd_paisa: i128,
    pub(crate) dd_peak_ts: i64,
    pub(crate) dd_low_ts: i64,
    peak_x_ts: i64,
}

impl Book {
    /// A fresh account at capital `c0` (Rs 20 lakh at the window's first month).
    pub(crate) fn new(opt: Opt, c0: i128, record: bool) -> Self {
        Self {
            opt,
            eq: c0,
            peak: c0,
            cap_m: c0,
            month_key: u8::MAX,
            day_key: u32::MAX,
            day_net: 0,
            month_net: 0,
            day_trades: 0,
            month_trades: 0,
            day_stocks: [0; 4],
            open: Vec::with_capacity(SLOTS),
            held: [0; 4],
            pend_sec: 0,
            pend_net: 0,
            pend_month: 0,
            has_pend: false,
            row: ROW0,
            counters: Counters::default(),
            took: record.then(Vec::new),
            peak_x: c0,
            dd_bp: 0,
            dd_paisa: 0,
            dd_peak_ts: 0,
            dd_low_ts: 0,
            peak_x_ts: 0,
        }
    }

    /// Carry the account into a new run (the chain): a new option set, the same money.
    pub(crate) fn set_opt(&mut self, opt: Opt) {
        self.opt = opt;
    }

    fn flush(&mut self) {
        if !self.has_pend {
            return;
        }
        self.has_pend = false;
        if let Some(m) = self.row.get_mut(self.pend_month) {
            let e = m.end + self.pend_net;
            m.end = e;
            if e > m.high {
                m.high = e;
            }
            if e < m.low {
                m.low = e;
            }
            if m.high - e > m.fall {
                m.fall = m.high - e;
            }
        }
        if self.eq > self.peak {
            self.peak = self.eq;
        }
    }

    /// Open positions are kept sorted by (exit, stock), latest first: the next to leave
    /// is the last element.
    fn realize(&mut self, t: i64) {
        loop {
            let Some(last) = self.open.last() else { break };
            if last.exit >= t {
                break;
            }
            let Some(p) = self.open.pop() else { break };
            let w = usize::from(p.stock >> 6);
            if let Some(h) = self.held.get_mut(w) {
                *h &= !(1_u64 << (p.stock & 63));
            }
            let net = i128::from(p.net);
            self.eq += net;
            if p.day == self.day_key {
                self.day_net += net;
            }
            if p.month == self.month_key {
                self.month_net += net;
            }
            // the preview's per-exit drawdown
            if self.eq > self.peak_x {
                self.peak_x = self.eq;
                self.peak_x_ts = p.exit;
            } else if self.peak_x > 0 {
                let bp = (10_000 * (self.peak_x - self.eq)).div_euclid(self.peak_x);
                if bp > self.dd_bp {
                    self.dd_bp = bp;
                    self.dd_paisa = self.peak_x - self.eq;
                    self.dd_peak_ts = self.peak_x_ts;
                    self.dd_low_ts = p.exit;
                }
            }
            let mi = usize::from(p.month);
            if let Some(m) = self.row.get_mut(mi) {
                m.trades += 1;
                if p.net > 0 {
                    m.wins += 1;
                }
                m.net += p.net;
                m.gross += p.gross;
                m.charges += p.charges;
                m.hold += i64::from(p.hold);
            }
            if self.has_pend && self.pend_sec == p.exit && self.pend_month == mi {
                self.pend_net += p.net;
            } else {
                self.flush();
                self.pend_sec = p.exit;
                self.pend_net = p.net;
                self.pend_month = mi;
                self.has_pend = true;
            }
            if let Some(x) = self.counters.exits.get_mut(usize::from(p.kind)) {
                *x += 1;
            }
            if p.net > 0 {
                self.counters.wins += 1;
                self.counters.gross_win += net;
            } else if p.net < 0 {
                self.counters.losses += 1;
                self.counters.gross_loss -= net;
            }
            if let Some(tk) = self.took.as_mut() {
                tk.push(Took { src: p.src, half: p.half, net: p.net, gross: p.gross, charges: p.charges, exit: p.exit, month: p.month, kind: p.kind });
            }
        }
        self.flush();
    }

    /// One kept record; `which` 0 WITH, 1 FADE; `src` its index in the kept list.
    pub(crate) fn step(&mut self, r: &Hot, which: usize, src: u32) {
        if r.day != self.day_key {
            self.day_key = r.day;
            self.day_net = 0;
            self.day_trades = 0;
            self.day_stocks = [0; 4];
        }
        self.realize(r.t);
        if r.month_t != self.month_key {
            self.month_key = r.month_t;
            self.cap_m = self.eq;
            self.month_net = 0;
            self.month_trades = 0;
        }
        self.counters.signals += 1;
        // The admit checks first, then the controls: a control only blocks entries (it
        // never changes a state a rejected record could change), so the set of trades is
        // the same in either order; "blocked by a control" counts the entries that would
        // otherwise have been taken.
        let w = usize::from(r.stock >> 6);
        let bit = 1_u64 << (r.stock & 63);
        if self.held.get(w).is_some_and(|h| h & bit != 0) {
            self.counters.while_open += 1;
            return;
        }
        if self.open.len() >= SLOTS {
            self.counters.slots_full += 1;
            return;
        }
        let Some(tr) = r.tr.get(which) else { return };
        match tr.status {
            0 => {}
            1 => {
                self.counters.not_filled += 1;
                return;
            }
            2 => {
                self.counters.zero_qty += 1;
                return;
            }
            _ => {
                self.counters.no_ratio += 1;
                return;
            }
        }
        if tr.month == u8::MAX {
            return;
        }
        let o = self.opt;
        let cm = self.cap_m;
        if o.day_loss > 0 && 10_000 * self.day_net <= -o.day_loss * cm {
            self.counters.blocked[0] += 1;
            return;
        }
        if o.month_loss > 0 && 10_000 * self.month_net <= -o.month_loss * cm {
            self.counters.blocked[1] += 1;
            return;
        }
        if o.target > 0 && 10_000 * self.month_net >= o.target * cm {
            self.counters.blocked[2] += 1;
            return;
        }
        let capped = match o.cap {
            1 => self.day_stocks.get(w).is_some_and(|x| x & bit != 0),
            2 => self.day_trades >= 5,
            3 => self.day_trades >= 10,
            4 => self.month_trades >= 10,
            5 => self.month_trades >= 20,
            6 => self.month_trades >= 40,
            _ => false,
        };
        if capped {
            self.counters.blocked[3] += 1;
            return;
        }
        let half = o.fall > 0 && 10_000 * (self.peak - self.eq) > o.fall * self.peak;
        if half && !tr.half_ok {
            self.counters.zero_qty += 1;
            self.counters.half_zero += 1;
            return;
        }
        let (net, gross, charges) = if half { (tr.net_h, tr.gross_h, tr.charges_h) } else { (tr.net, tr.gross, tr.charges) };
        if half {
            self.counters.half_trades += 1;
        }
        let pos = BPos {
            exit: tr.exit_us,
            stock: r.stock,
            net,
            gross,
            charges,
            hold: tr.hold,
            month: tr.month,
            day: r.day,
            src,
            half,
            kind: tr.kind,
        };
        let at = self.open.partition_point(|x| (x.exit, x.stock) > (pos.exit, pos.stock));
        self.open.insert(at, pos);
        if let Some(h) = self.held.get_mut(w) {
            *h |= bit;
        }
        if let Some(h) = self.day_stocks.get_mut(w) {
            *h |= bit;
        }
        self.day_trades += 1;
        self.month_trades += 1;
    }

    pub(crate) fn finish(&mut self) {
        self.realize(i64::MAX);
    }
}

/// Order one instant's records by strength (desc), then symbol, as a monthly cap needs.
pub(crate) fn strength_order(v: &mut [Hot]) {
    let mut i = 0;
    while i < v.len() {
        let t = v.get(i).map_or(0, |x| x.t);
        let mut j = i;
        while v.get(j).is_some_and(|x| x.t == t) {
            j += 1;
        }
        if let Some(g) = v.get_mut(i..j)
            && g.len() > 1
        {
            g.sort_by(|a, b| b.strength.cmp(&a.strength).then(a.stock.cmp(&b.stock)));
        }
        i = j;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "tests")]
mod tests {
    use super::*;
    use crate::spool::HotTrade;

    fn mk(t: i64, stock: u16, side: u8, exit: i64, net: i64, month: u8) -> Hot {
        let tr = HotTrade { exit_us: exit, net, gross: net + 10, charges: 10, hold: 60, status: 0, month, half_ok: true, net_h: net / 2, gross_h: net / 2 + 5, charges_h: 5, kind: 0 };
        let fade = HotTrade { net: -net - 20, gross: -net, ..tr };
        Hot { t, stock, side, day: u32::try_from(t / 1000).unwrap(), month_t: month, tr: [tr, fade], ..Hot::default() }
    }

    fn series(n: usize, seed: u64) -> Vec<Hot> {
        let mut v = Vec::new();
        for i in 0..n {
            let x = draw(seed, u64::try_from(i).unwrap(), 7, 9);
            let t = i64::try_from(i).unwrap() * 37 + 1000;
            let stock = u16::try_from(x % 30).unwrap();
            let side = u8::try_from((x >> 8) % 2).unwrap();
            let exit = t + 1 + i64::try_from((x >> 16) % 400).unwrap();
            let net = i64::try_from((x >> 24) % 20_000).unwrap() - 9_000;
            let month = u8::try_from((i * 98) / n).unwrap();
            v.push(mk(t, stock, side, exit, net, month));
        }
        v
    }

    #[test]
    fn fast_equals_book_with_every_control_off() {
        for seed in 1..6 {
            let v = series(3_000, seed);
            let mut f = Fast::new();
            for r in &v {
                f.step(&Mini::of(r).unwrap());
            }
            f.finish();
            for side in 0..6 {
                let mut b = Book::new(opt_of(0), C0, false);
                for (i, r) in v.iter().enumerate() {
                    let keep = match side {
                        0 | 3 => r.side == 0,
                        1 | 4 => r.side == 1,
                        _ => true,
                    };
                    if keep {
                        b.step(r, usize::from(side >= 3), u32::try_from(i).unwrap());
                    }
                }
                b.finish();
                assert_eq!(b.row, f.acc[side].row, "seed {seed} side {side}");
            }
        }
    }

    #[test]
    fn the_path_join_equals_the_month_by_month_fold() {
        let v = series(2_000, 9);
        let mut f = FastAcc::new();
        for r in &v {
            f.step(r.t, r.stock, &Mini::of(r).unwrap().tr[0]);
        }
        f.finish();
        let mut p = Path::default();
        for m in &f.row {
            p = p.join(m);
        }
        // the same fall from a flat walk over months' paths, recomputed from (end, low, high, fall)
        let mut eq = 0_i128;
        let mut hi = 0_i128;
        let mut fall = 0_i128;
        for m in &f.row {
            fall = fall.max(i128::from(m.fall)).max(hi - (eq + i128::from(m.low)));
            hi = hi.max(eq + i128::from(m.high));
            eq += i128::from(m.end);
        }
        assert_eq!(p.fall, fall);
        assert_eq!(p.end, eq);
    }

    #[test]
    fn the_heap_keeps_the_exact_top_k_whatever_the_order() {
        let mut keys: Vec<Key> = (0..500_u64)
            .map(|i| {
                let x = draw(3, i, 1, 2);
                Key { standing: u8::try_from(x % 3).unwrap(), below: u16::try_from((x >> 4) % 50).unwrap(), worst_n: i128::from(i64::try_from((x >> 8) % 1000).unwrap()) - 500, worst_d: C0, dd: i64::try_from((x >> 20) % 30).unwrap(), net: i64::try_from((x >> 30) % 100).unwrap(), wins: 3, trades: 10, id: i, ..Key::default() }
            })
            .collect();
        let row = ROW0;
        let mut a = TopK::new(20);
        for k in &keys {
            a.offer(k, Some(&row));
        }
        let mut b1 = TopK::new(20);
        let mut b2 = TopK::new(20);
        for (i, k) in keys.iter().rev().enumerate() {
            if i % 2 == 0 { b1.offer(k, None) } else { b2.offer(k, Some(&row)) }
        }
        b1.merge(&b2);
        keys.sort_by(cmp_key);
        let mut distinct: Vec<Key> = Vec::new();
        for k in &keys {
            if !distinct.iter().any(|d| same_result(d, k)) {
                distinct.push(*k);
            }
        }
        let want: Vec<u64> = distinct.iter().take(20).map(|k| k.id).collect();
        let got_a: Vec<u64> = a.v.iter().map(|e| e.key.id).collect();
        let got_b: Vec<u64> = b1.v.iter().map(|e| e.key.id).collect();
        assert_eq!(got_a, want);
        assert_eq!(got_b, want);
    }

    #[test]
    fn the_luck_merge_is_order_free() {
        let fl = flips();
        let masks: Vec<(u128, u128)> = (0..300_u64).map(|i| (u128::from(draw(5, i, 0, 0)) & ((1 << 82) - 1), u128::from(draw(6, i, 0, 0)) << 18 & ((1 << 82) - 1))).collect();
        let mut one = Luck::new();
        for &(p, n) in &masks {
            one.offer(&fl, p & !n, n & !p);
        }
        let mut a = Luck::new();
        let mut b = Luck::new();
        for (i, &(p, n)) in masks.iter().enumerate().rev() {
            if i % 3 == 0 { a.offer(&fl, p & !n, n & !p) } else { b.offer(&fl, p & !n, n & !p) }
        }
        b.merge(&a);
        assert_eq!(one.best, b.best);
    }

    #[test]
    fn each_control_blocks_exactly_at_its_threshold_and_never_changes_an_exit() {
        // two trades on one day: the first loses exactly 0.5% of Rs 20 lakh (Rs 10,000)
        let day = 1;
        let lose = mk(1000, 1, 0, 1500, -1_000_000, 5);
        let next = mk(1600, 2, 0, 1700, 500, 5);
        let mut lose = lose;
        let mut next = next;
        lose.day = day;
        next.day = day;
        for (dl, blocked) in [(50_i128, true), (100, false)] {
            let o = Opt { day_loss: dl, ..Opt::default() };
            let mut b = Book::new(o, C0, true);
            b.step(&lose, 0, 0);
            b.step(&next, 0, 1);
            b.finish();
            assert_eq!(b.counters.blocked[0], u64::from(blocked), "day loss {dl}");
            let took = b.took.unwrap();
            assert_eq!(took[0].exit, 1500);
            assert_eq!(took[0].net, -1_000_000);
        }
    }

    #[test]
    fn the_option_sets_are_768_times_the_monthly_caps() {
        assert_eq!(N_OPT, 1_344);
        assert_eq!(opt_of(0), Opt::default());
        let mut seen = std::collections::BTreeSet::new();
        for o in 0..N_OPT {
            let x = opt_of(o);
            seen.insert((x.day_loss, x.month_loss, x.target, x.fall, x.cap));
        }
        assert_eq!(seen.len(), N_OPT);
    }
}
