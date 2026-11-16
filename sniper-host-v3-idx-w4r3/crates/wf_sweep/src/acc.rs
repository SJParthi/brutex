//! `SweepAcc` (decision D-S14, addition 1): the learning cells of `Acc` for C columns, insert / remove in ANY order, the exact
//! max drawdown from per-year-part session trees (SNIPER-ADANIENT-PLAN section 7), learning months only.
//!
//! **Layout.** The learning hull (TT's learn months, 2018-09..2022-12 on the standard windows) is cut into calendar-year PARTS
//! (common_wf DV1: per-window year units, cut only by the window's own boundaries; every standard learn window starts at the
//! hull's first month and ends on a December, so the parts are calendar years and WF's parts are a prefix of TT's). Each part
//! holds one segment tree per column whose leaves are the part's sessions (at most 366, depth at most 9: fixed by the calendar,
//! never by the data size). A leaf holds the session's trade (FIRST mode: at most one trade a session and side), and a node the
//! drawdown monoid of its span, the worst trade and the worst run-down. A window's K5 is the left-to-right composition of its
//! parts' roots (2 parts for WF, 5 for TT).
//!
//! **Cost per event** (plan 8.4): an insert or removal writes one leaf and re-merges its path (<= 9 merges, SC-PATH charged to
//! the insert, F19) per column, updates one part cell and one month cell per column and the window cells holding them: a fixed
//! number of steps. A key read composes at most 5 part roots (SC-K5). Mutant `m_s14` charges the path merges to the READ.
//!
//! **Lazy trees (speed build sniper-host-v1-fast, 2026-11-14).** An insert or removal writes its leaf and only MARKS its path
//! dirty (stopping at the first node already dirty: a dirty node's ancestors are dirty); a key read re-merges exactly the dirty
//! nodes under the part roots it composes, with the same node product, so every root equals the eager tree's root bit for bit.
//! The step counters are unchanged by design: an event is still charged its whole path (`depth` merges, SC-PATH charged to the
//! insert, F19) as a PREPAID charge, and each merge a read performs is on a node some charged event dirtied, so the merges
//! performed never exceed the merges charged (test `lazy_merges_never_exceed_the_charged_path`). Exactness of the refusals: a
//! tree merge can refuse only on a checked-sum overflow; [`SweepAcc::new`] turns the lazy mode on only when the sum of |net| over
//! the whole outcome table is at most `i64::MAX / 4` per column, which bounds every node value of every possible held set by
//! twice that sum, so no merge of the eager tree could ever have refused; otherwise the accumulator keeps the eager path. The
//! learning facts of a (window, column) are cached until the next event (an exact memo: same state, same facts); the window
//! cells' month-derived parts (K8's losing months, K9's positive months) are brought up to date at the next key read from the
//! months touched since the last one (the same telescoping integer updates as the per-event ones); and a view
//! whose list would reject the key on its cheap leading parts alone skips the tree ([`SweepAcc::view_key_gated`]).

use crate::dd::{self, Dd};
use crate::lists::{Code, Lists};
use crate::views::View;
use crate::{MAXC, Mk, Paisa, SweepErr, add, mix, overflow, sub, unknown_row};

/// No row / no leaf.
const NONE: u32 = u32::MAX;

/// One candidate row's priced outcome (the outcome store's fields the sweep reads).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Outcome {
    /// The session index.
    pub sess: u32,
    /// The trade's month key (its session's month).
    pub mk: Mk,
    /// The trade id (the order-free fingerprint's input).
    pub id: u64,
    /// Net per column (share level) at the proxy N, paisa.
    pub net: [Paisa; MAXC],
    /// The worst run-down per column, bp (S4: a u16 by a checked conversion).
    pub rd_bp: [u16; MAXC],
}

/// One learning window of the set (from WINDOWS-STD-v1.tsv: WF learn 2018-09..2019-12, TT learn 2018-09..2022-12).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Win {
    /// The window's id (`WF`, `TT`).
    pub name: String,
    /// Its first learning month.
    pub from: Mk,
    /// Its last learning month.
    pub to: Mk,
    /// Its parts: the first `parts` parts of the layout.
    pub parts: usize,
}

/// The learning hull's calendar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    /// The hull's first and last month.
    pub hull: (Mk, Mk),
    /// The parts' calendar years.
    pub years: Vec<i32>,
    /// Leaves per part.
    pub leaves: Vec<usize>,
    /// Per session index: (part, leaf), `NONE` outside the hull.
    sess: Vec<(u32, u32)>,
    /// The windows.
    pub wins: Vec<Win>,
    /// Per window: its COUNTED learning months (P9: months holding at least one session), K9's denominator.
    pub counted: Vec<u32>,
}

impl Layout {
    /// The layout from every session's month key (indexed like the host's session table), the hull and the windows.
    ///
    /// # Errors
    /// `BadWindow` for a window that does not start at the hull's first month, or ends neither on a December nor at the hull's
    /// last month, or leaves the hull.
    pub fn new(sess_mk: &[Mk], hull: (Mk, Mk), wins: &[(String, Mk, Mk)]) -> Result<Self, SweepErr> {
        let (y0, y1) = (hull.0.div_euclid(12), hull.1.div_euclid(12));
        let years: Vec<i32> = (y0..=y1).collect();
        let mut leaves = vec![0_usize; years.len()];
        let mut sess = Vec::with_capacity(sess_mk.len());
        for &mk in sess_mk {
            if mk < hull.0 || mk > hull.1 {
                sess.push((NONE, NONE));
                continue;
            }
            let p = usize::try_from(mk.div_euclid(12) - y0).map_err(|_| SweepErr::Overflow)?;
            let l = leaves.get_mut(p).ok_or_else(overflow)?;
            sess.push((u32::try_from(p).map_err(|_| SweepErr::Overflow)?, u32::try_from(*l).map_err(|_| SweepErr::Overflow)?));
            *l += 1;
        }
        let mut has = vec![false; usize::try_from(hull.1 - hull.0 + 1).unwrap_or(0)];
        for &mk in sess_mk {
            if mk >= hull.0
                && mk <= hull.1
                && let Some(x) = usize::try_from(mk - hull.0).ok().and_then(|i| has.get_mut(i))
            {
                *x = true;
            }
        }
        let mut counted = Vec::new();
        let mut ws = Vec::new();
        for (name, from, to) in wins {
            if *from != hull.0 || *to > hull.1 || (*to != hull.1 && to.rem_euclid(12) != 11) || to < from {
                return Err(SweepErr::BadWindow(name.clone()));
            }
            let parts = usize::try_from(to.div_euclid(12) - y0 + 1).map_err(|_| SweepErr::Overflow)?;
            ws.push(Win { name: name.clone(), from: *from, to: *to, parts });
            let n = has.iter().take(usize::try_from(to - hull.0 + 1).unwrap_or(0)).filter(|x| **x).count();
            counted.push(u32::try_from(n).map_err(|_| SweepErr::Overflow)?);
        }
        Ok(Self { hull, years, leaves, sess, wins: ws, counted })
    }

    /// The months of the hull.
    #[must_use]
    pub fn months(&self) -> usize {
        usize::try_from(self.hull.1 - self.hull.0 + 1).unwrap_or(0)
    }

    /// POSITIONAL v2 (B4, design 6.3): the leaves from the GROUPS' booking months (`leaf_mk`, one entry per group in time order: the
    /// group index is the `Outcome.sess` the accumulator reads), and the COUNTED months of K9 from the SESSIONS' months (`month_mk`, as
    /// common_wf `statics()` counts them: a learning month with no booking is still counted). The cash [`Layout::new`] is unchanged (P2).
    ///
    /// # Errors
    /// As [`Layout::new`].
    pub fn new_pos(leaf_mk: &[Mk], month_mk: &[Mk], hull: (Mk, Mk), wins: &[(String, Mk, Mk)]) -> Result<Self, SweepErr> {
        let mut l = Self::new(leaf_mk, hull, wins)?;
        l.counted = Self::new(month_mk, hull, wins)?.counted;
        Ok(l)
    }

    /// The tree depth of a part (the SC-PATH bound).
    #[must_use]
    pub fn depth(&self, p: usize) -> u32 {
        self.leaves.get(p).map_or(0, |&n| n.max(1).next_power_of_two().trailing_zeros())
    }
}

/// A tree node: the drawdown monoid of its span (TRADE base), the worst trade, the worst run-down, the trades.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Node {
    dd: Dd,
    worst: i64,
    rd: u16,
    n: u32,
}

impl Node {
    const EMPTY: Self = Self { dd: Dd::ZERO, worst: i64::MAX, rd: 0, n: 0 };

    fn combine(&self, b: &Self) -> Result<Self, SweepErr> {
        let d = if cfg!(feature = "m_s6") { dd::combine(&b.dd, &self.dd)? } else { dd::combine(&self.dd, &b.dd)? };
        Ok(Self { dd: d, worst: self.worst.min(b.worst), rd: self.rd.max(b.rd), n: self.n.saturating_add(b.n) })
    }

    /// `combine` with the identity shortcut of the lazy trees: a side holding no trade (`n == 0`, which is exactly `EMPTY`: a
    /// node is built from its leaves by `combine`, and `EMPTY` composes to `EMPTY`) returns the other side unchanged. Bit-identical
    /// to `combine`: the drawdown monoid's identity is exact for every node `push` / `combine` build (max prefix >= total >= min
    /// prefix, max drawdown >= max prefix - total and >= -min prefix), and `min(worst, i64::MAX)`, `max(rd, 0)`, `n + 0` change
    /// nothing. Used only by a lazy accumulator (inside the exactness bound no merge refuses, so no refusal is skipped).
    fn combine_fast(&self, b: &Self) -> Result<Self, SweepErr> {
        if b.n == 0 {
            return Ok(*self);
        }
        if self.n == 0 {
            return Ok(*b);
        }
        self.combine(b)
    }
}

/// The REQ 73 key of one (window, column) (P16: K5, K7, K6, K8, K9, K10, K4; CODE breaks the last tie in the lists). Every
/// part is "smaller is better", so the derived order is the key's order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(pub [i64; KEYN]);

/// Key parts held (the MISS key is the longest: K1 K16 K17 K5 K7 K6 K8 K9 K10 K4).
pub const KEYN: usize = 10;

impl Key {
    /// A key from its parts in order (the unused tail is 0, so keys of one kind compare as their parts).
    #[must_use]
    pub fn of(parts: &[i64]) -> Self {
        let mut a = [0_i64; KEYN];
        for (x, y) in a.iter_mut().zip(parts) {
            *x = *y;
        }
        Self(a)
    }
}

/// The learning facts behind a key (printed, never re-derived).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Learn {
    /// Net, paisa.
    pub net: i64,
    /// Trades.
    pub trades: u32,
    /// Winning trades.
    pub wins: u32,
    /// K5: max drawdown, paisa.
    pub max_dd: i64,
    /// K7: the worst trade's net, paisa.
    pub worst: i64,
    /// K6: the worst run-down, bp.
    pub rd_bp: u16,
    /// K8: the sum of the losing learning months, paisa (<= 0).
    pub losing: i64,
    /// Learning months with a trade, and how many of them were positive.
    pub months_traded: u32,
    /// See `months_traded`.
    pub months_pos: u32,
    /// The order-free fingerprint of the learning trade set (P18).
    pub fp: u64,
    /// The lowest cumulative net (the empty prefix 0 included): SC-ACCT reads it (base Q7: a free-cash cut is possible only when
    /// it falls below -(Rs 20 L - slot)).
    pub min_prefix: i64,
}

/// Why a cell is not eligible (counted per reason).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotEligible {
    /// E1: no learning trade.
    NoTrade,
    /// E2: a learning year part not positive (or with no trade).
    PartNotPositive,
    /// E4: a learning year part below the trade floor.
    BelowFloor,
    /// E7 (a MISS view): the cell passes E2, so it is no miss.
    PassesE2,
}

/// Which REQ 73 / P16 key a view row ranks by (VIEWS-SNIPER-v1.tsv `key` column; COMMON-WF VIEWS-STD-v1 rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyKind {
    /// `-R73` and its floors: E1 E2 (E4), key K5(TRADE) K7 K6 K8 K9 K10 K4.
    R73,
    /// `-N` (REQ 55 way B-N): E1 E2, key K4 K6 K5(TRADE).
    N,
    /// `-MISS` (P17): E1 E7 (not E2), key K1 K16 K17 K5(TRADE) K7 K6 K8 K9 K10 K4.
    Miss,
}

#[derive(Clone, Copy, Debug, Default)]
struct WinCell {
    net: i64,
    trades: u32,
    wins: u32,
    fp: u64,
    losing: i64,
    m_traded: u32,
    m_pos: u32,
}

/// The lazy trees are exact when the sum of |net| over the outcome table is at most this, per column (module doc): every node
/// value of every possible held set then lies within twice that sum, so no checked merge of the eager tree could refuse.
pub const LAZY_SUM_MAX: i64 = i64::MAX / 4;

/// Re-merges the dirty nodes under node `i` (children first), counting each merge; a clean node is left as it is.
fn fix(t: &mut [Node], d: &mut [bool], i: usize, merges: &mut u64) -> Result<(), SweepErr> {
    if !d.get(i).copied().unwrap_or(false) {
        return Ok(());
    }
    // A clean child is never entered (a leaf is never dirty).
    if d.get(2 * i).copied().unwrap_or(false) {
        fix(t, d, 2 * i, merges)?;
    }
    if d.get(2 * i + 1).copied().unwrap_or(false) {
        fix(t, d, 2 * i + 1, merges)?;
    }
    let (l, r) = (t.get(2 * i).copied().ok_or_else(overflow)?, t.get(2 * i + 1).copied().ok_or_else(overflow)?);
    *t.get_mut(i).ok_or_else(overflow)? = l.combine_fast(&r)?;
    if let Some(f) = d.get_mut(i) {
        *f = false;
    }
    *merges += 1;
    Ok(())
}

/// One month cell's move from `old` to `new` (net, trades) applied to a window cell: K8's losing-month sum and K9's traded /
/// positive month counts (binary k's per-event update, the same checked integers; consecutive moves telescope).
fn month_into(c: &mut WinCell, old: (i64, u32), new: (i64, u32)) -> Result<(), SweepErr> {
    let ((old_net, old_n), (new_net, new_n)) = (old, new);
    let lose = |net: i64, n: u32| if n > 0 { net.min(0) } else { 0 };
    c.losing = add(sub(c.losing, lose(old_net, old_n))?, lose(new_net, new_n))?;
    let (t0, t1) = (u32::from(old_n > 0), u32::from(new_n > 0));
    let (p0, p1) = (u32::from(old_n > 0 && old_net > 0), u32::from(new_n > 0 && new_net > 0));
    c.m_traded = (c.m_traded + t1).checked_sub(t0).ok_or_else(overflow)?;
    c.m_pos = (c.m_pos + p1).checked_sub(p0).ok_or_else(overflow)?;
    Ok(())
}

/// The value node `i` holds in the eager tree, read without writing (a dirty node is composed from its children).
fn value(t: &[Node], d: &[bool], i: usize) -> Result<Node, SweepErr> {
    if !d.get(i).copied().unwrap_or(false) {
        return Ok(t.get(i).copied().unwrap_or(Node::EMPTY));
    }
    value(t, d, 2 * i)?.combine_fast(&value(t, d, 2 * i + 1)?)
}

/// The sweep accumulator over one outcome table.
pub struct SweepAcc<'a> {
    lay: &'a Layout,
    out: &'a [Outcome],
    cols: usize,
    pow: Vec<usize>,
    depth: Vec<u64>,
    trees: Vec<Vec<Node>>,
    dirty: Vec<Vec<bool>>,
    lazy: bool,
    part_net: Vec<i64>,
    part_n: Vec<u32>,
    month_net: Vec<i64>,
    month_n: Vec<u32>,
    msync_net: Vec<i64>,
    msync_n: Vec<u32>,
    mdirty: Vec<bool>,
    mlist: Vec<Vec<u32>>,
    win: Vec<WinCell>,
    held: Vec<u32>,
    touched: Vec<u32>,
    pending: u64,
    epoch: u64,
    memo: Vec<(u64, Learn)>,
    /// Insert / removal / key-read counters.
    pub inserts: u64,
    /// See `inserts`.
    pub removes: u64,
    /// Key reads.
    pub key_reads: u64,
    /// Tree merges charged to the events (SC-PATH: each column event its whole path), resets included.
    pub merges_charged: u64,
    /// Tree merges performed (eager: at the event; lazy: at the reads); never above `merges_charged`.
    pub merges_done: u64,
}

impl<'a> SweepAcc<'a> {
    /// An empty accumulator of `cols` columns over `out` (indexed by row id). The trees are lazy when the outcome table's sum of
    /// |net| is at most [`LAZY_SUM_MAX`] in every column, eager otherwise ([`SweepAcc::is_lazy`]).
    ///
    /// # Errors
    /// `TooManyColumns`.
    pub fn new(lay: &'a Layout, out: &'a [Outcome], cols: usize, sessions: usize) -> Result<Self, SweepErr> {
        if cols == 0 || cols > MAXC {
            return Err(SweepErr::TooManyColumns);
        }
        let np = lay.leaves.len();
        let pow: Vec<usize> = lay.leaves.iter().map(|&n| n.max(1).next_power_of_two()).collect();
        let depth: Vec<u64> = pow.iter().map(|p| u64::from(p.trailing_zeros())).collect();
        let mut trees = Vec::with_capacity(cols * np);
        let mut dirty = Vec::with_capacity(cols * np);
        for _ in 0..cols {
            for &p in &pow {
                trees.push(vec![Node::EMPTY; 2 * p]);
                dirty.push(vec![false; 2 * p]);
            }
        }
        let mut lazy = true;
        for c in 0..cols {
            let s: i128 = out.iter().map(|o| i128::from(o.net.get(c).copied().unwrap_or(0)).abs()).sum();
            if s > i128::from(LAZY_SUM_MAX) {
                lazy = false;
            }
        }
        let nm = lay.months();
        let nw = lay.wins.len();
        Ok(Self {
            lay,
            out,
            cols,
            pow,
            depth,
            trees,
            dirty,
            lazy,
            part_net: vec![0; cols * np],
            part_n: vec![0; cols * np],
            month_net: vec![0; cols * nm],
            month_n: vec![0; cols * nm],
            msync_net: vec![0; cols * nm],
            msync_n: vec![0; cols * nm],
            mdirty: vec![false; cols * nm],
            mlist: vec![Vec::new(); cols],
            win: vec![WinCell::default(); cols * nw],
            held: vec![NONE; sessions],
            touched: Vec::new(),
            pending: 0,
            epoch: 1,
            memo: vec![(0, Learn::default()); cols * nw],
            inserts: 0,
            removes: 0,
            key_reads: 0,
            merges_charged: 0,
            merges_done: 0,
        })
    }

    /// The columns.
    #[must_use]
    pub const fn cols(&self) -> usize {
        self.cols
    }

    /// The layout.
    #[must_use]
    pub const fn layout(&self) -> &Layout {
        self.lay
    }

    /// The EAGER accumulator whatever the outcome table (binary k's tree path: every event re-merges its path at once); the
    /// reference of LAZY-EQ.
    ///
    /// # Errors
    /// `TooManyColumns`.
    pub fn new_eager(lay: &'a Layout, out: &'a [Outcome], cols: usize, sessions: usize) -> Result<Self, SweepErr> {
        let mut a = Self::new(lay, out, cols, sessions)?;
        a.lazy = false;
        Ok(a)
    }

    /// True when the trees are lazy (the outcome table is inside the exactness bound [`LAZY_SUM_MAX`]).
    #[must_use]
    pub const fn is_lazy(&self) -> bool {
        self.lazy
    }

    /// Writes one leaf. Eager: re-merges its path. Lazy: marks its path dirty up to the first node already dirty. Returns the
    /// merges CHARGED to the event: the whole path (`depth` of the part), as the eager tree performs them.
    fn set_leaf(&mut self, col: usize, part: usize, leaf: usize, node: Node) -> Result<u64, SweepErr> {
        let np = self.lay.leaves.len();
        let p = self.pow.get(part).copied().ok_or_else(overflow)?;
        let depth = self.depth.get(part).copied().ok_or_else(overflow)?;
        let t = self.trees.get_mut(col * np + part).ok_or_else(overflow)?;
        let mut i = p + leaf;
        *t.get_mut(i).ok_or_else(overflow)? = node;
        if self.lazy {
            let d = self.dirty.get_mut(col * np + part).ok_or_else(overflow)?;
            i /= 2;
            while i >= 1 {
                let f = d.get_mut(i).ok_or_else(overflow)?;
                if *f {
                    break;
                }
                *f = true;
                i /= 2;
            }
        } else {
            while i > 1 {
                i /= 2;
                let (l, r) = (t.get(2 * i).copied().ok_or_else(overflow)?, t.get(2 * i + 1).copied().ok_or_else(overflow)?);
                *t.get_mut(i).ok_or_else(overflow)? = l.combine(&r)?;
                self.merges_done += 1;
            }
        }
        self.merges_charged += depth;
        Ok(depth)
    }

    fn month_update(&mut self, col: usize, m: usize, dnet: i64, dn: i32) -> Result<(), SweepErr> {
        let nm = self.lay.months();
        let idx = col * nm + m;
        let old_net = self.month_net.get(idx).copied().ok_or_else(overflow)?;
        let old_n = self.month_n.get(idx).copied().ok_or_else(overflow)?;
        let new_net = add(old_net, dnet)?;
        let new_n = u32::try_from(i64::from(old_n) + i64::from(dn)).map_err(|_| SweepErr::Overflow)?;
        *self.month_net.get_mut(idx).ok_or_else(overflow)? = new_net;
        *self.month_n.get_mut(idx).ok_or_else(overflow)? = new_n;
        if self.lazy {
            // Lazy month cells: the window cells' K8 / K9 inputs are brought up to date at the next key read (sync_months), with
            // the same telescoping integer updates; the first touch since the last sync lists the month.
            let f = self.mdirty.get_mut(idx).ok_or_else(overflow)?;
            if !*f {
                *f = true;
                self.mlist.get_mut(col).ok_or_else(overflow)?.push(u32::try_from(m).map_err(|_| SweepErr::Overflow)?);
            }
            return Ok(());
        }
        let mk = self.lay.hull.0 + i32::try_from(m).map_err(|_| SweepErr::Overflow)?;
        let nw = self.lay.wins.len();
        for w in 0..nw {
            if self.lay.wins.get(w).is_some_and(|x| mk <= x.to) {
                let c = self.win.get_mut(col * nw + w).ok_or_else(overflow)?;
                month_into(c, (old_net, old_n), (new_net, new_n))?;
            }
        }
        Ok(())
    }

    /// Brings column `col`'s window cells up to date with every month touched since the last sync (lazy month cells).
    fn sync_months(&mut self, col: usize) -> Result<(), SweepErr> {
        let Some(list) = self.mlist.get_mut(col) else { return Ok(()) };
        if list.is_empty() {
            return Ok(());
        }
        let mut list = std::mem::take(list);
        let nm = self.lay.months();
        let nw = self.lay.wins.len();
        for &m in &list {
            let idx = col * nm + m as usize;
            let old = (self.msync_net.get(idx).copied().ok_or_else(overflow)?, self.msync_n.get(idx).copied().ok_or_else(overflow)?);
            let new = (self.month_net.get(idx).copied().ok_or_else(overflow)?, self.month_n.get(idx).copied().ok_or_else(overflow)?);
            let mk = self.lay.hull.0 + i32::try_from(m).map_err(|_| SweepErr::Overflow)?;
            for w in 0..nw {
                if self.lay.wins.get(w).is_some_and(|x| mk <= x.to) {
                    month_into(self.win.get_mut(col * nw + w).ok_or_else(overflow)?, old, new)?;
                }
            }
            *self.msync_net.get_mut(idx).ok_or_else(overflow)? = new.0;
            *self.msync_n.get_mut(idx).ok_or_else(overflow)? = new.1;
            *self.mdirty.get_mut(idx).ok_or_else(overflow)? = false;
        }
        list.clear();
        if let Some(l) = self.mlist.get_mut(col) {
            *l = list;
        }
        Ok(())
    }

    /// Window `w`'s cell of column `col` as the eager accumulator holds it now (read only: the months touched since the last
    /// sync are applied to a copy).
    fn cell_now(&self, w: usize, col: usize) -> Result<WinCell, SweepErr> {
        let nm = self.lay.months();
        let nw = self.lay.wins.len();
        let mut c = self.win.get(col * nw + w).copied().ok_or_else(overflow)?;
        let to = self.lay.wins.get(w).ok_or_else(overflow)?.to;
        for &m in self.mlist.get(col).map_or(&[][..], Vec::as_slice) {
            let idx = col * nm + m as usize;
            let mk = self.lay.hull.0 + i32::try_from(m).map_err(|_| SweepErr::Overflow)?;
            if mk <= to {
                let old = (self.msync_net.get(idx).copied().ok_or_else(overflow)?, self.msync_n.get(idx).copied().ok_or_else(overflow)?);
                let new = (self.month_net.get(idx).copied().ok_or_else(overflow)?, self.month_n.get(idx).copied().ok_or_else(overflow)?);
                month_into(&mut c, old, new)?;
            }
        }
        Ok(c)
    }

    fn apply(&mut self, row: u32, sign: i32) -> Result<u64, SweepErr> {
        // Any event (even a refused one) ends the memo of every learning fact.
        self.epoch += 1;
        let o = *self.out.get(row as usize).ok_or_else(unknown_row)?;
        let in_hull = o.mk >= self.lay.hull.0 && o.mk <= self.lay.hull.1;
        if !in_hull {
            if cfg!(feature = "m_s1") {
                // MUTANT m-S1: the test-month trade is taken into the window sums (a test month reaches the keys).
                for c in 0..self.cols {
                    if let Some(w) = self.win.get_mut(c * self.lay.wins.len()) {
                        w.net = add(w.net, i64::from(sign) * o.net.get(c).copied().unwrap_or(0))?;
                    }
                }
                return Ok(1);
            }
            return Err(SweepErr::TestMonthInSweep);
        }
        let (part, leaf) = self.lay.sess.get(o.sess as usize).copied().ok_or_else(unknown_row)?;
        if part == NONE {
            return Err(SweepErr::TestMonthInSweep);
        }
        let held = self.held.get(o.sess as usize).copied().ok_or_else(unknown_row)?;
        if sign > 0 && held != NONE {
            return Err(SweepErr::LeafOccupied);
        }
        if sign < 0 && held != row {
            return Err(SweepErr::NotHeld);
        }
        let (part, leaf) = (part as usize, leaf as usize);
        let np = self.lay.leaves.len();
        let nw = self.lay.wins.len();
        let m = usize::try_from(o.mk - self.lay.hull.0).map_err(|_| SweepErr::Overflow)?;
        let fpm = mix(o.id);
        let mut steps = 0_u64;
        for c in 0..self.cols {
            let net = o.net.get(c).copied().ok_or_else(overflow)?;
            let rd = o.rd_bp.get(c).copied().ok_or_else(overflow)?;
            let node = if sign > 0 { Node { dd: dd::one(net)?, worst: net, rd, n: 1 } } else { Node::EMPTY };
            let merges = self.set_leaf(c, part, leaf, node)?;
            if cfg!(feature = "m_s14") {
                self.pending += merges;
            } else {
                steps += 1 + merges;
            }
            let pi = c * np + part;
            let pn = self.part_net.get_mut(pi).ok_or_else(overflow)?;
            *pn = if sign > 0 { add(*pn, net)? } else { sub(*pn, net)? };
            let pc = self.part_n.get_mut(pi).ok_or_else(overflow)?;
            *pc = if sign > 0 { pc.checked_add(1) } else { pc.checked_sub(1) }.ok_or_else(overflow)?;
            self.month_update(c, m, if sign > 0 { net } else { net.checked_neg().ok_or_else(overflow)? }, sign)?;
            steps += 2;
            for w in 0..nw {
                if self.lay.wins.get(w).is_some_and(|x| part < x.parts) {
                    let cell = self.win.get_mut(c * nw + w).ok_or_else(overflow)?;
                    if sign > 0 {
                        cell.net = add(cell.net, net)?;
                        cell.trades += 1;
                        cell.wins += u32::from(net > 0);
                        cell.fp = cell.fp.wrapping_add(fpm);
                    } else {
                        cell.net = sub(cell.net, net)?;
                        cell.trades = cell.trades.checked_sub(1).ok_or_else(overflow)?;
                        cell.wins = cell.wins.checked_sub(u32::from(net > 0)).ok_or_else(overflow)?;
                        cell.fp = cell.fp.wrapping_sub(fpm);
                    }
                    steps += 1;
                }
            }
        }
        if let Some(h) = self.held.get_mut(o.sess as usize) {
            if sign > 0 {
                *h = row;
                self.touched.push(o.sess);
            } else {
                *h = NONE;
            }
        }
        Ok(steps)
    }

    /// Row `row` becomes a trade. Returns the event's step count.
    ///
    /// # Errors
    /// `TestMonthInSweep` outside the learning hull; `LeafOccupied` for a second trade in a session; `Overflow`.
    pub fn insert(&mut self, row: u32) -> Result<u64, SweepErr> {
        self.inserts += 1;
        self.apply(row, 1)
    }

    /// Row `row` stops being a trade. Returns the event's step count.
    ///
    /// # Errors
    /// `NotHeld` when the row is not the session's trade; `Overflow`.
    pub fn remove(&mut self, row: u32) -> Result<u64, SweepErr> {
        self.removes += 1;
        self.apply(row, -1)
    }

    /// Empties the accumulator: every held trade removed (O(touched)).
    ///
    /// # Errors
    /// `Overflow`.
    pub fn reset(&mut self) -> Result<(), SweepErr> {
        if self.lazy && !cfg!(feature = "m_s1") && !cfg!(feature = "m_s14") {
            return self.reset_lazy();
        }
        let t = std::mem::take(&mut self.touched);
        for s in &t {
            let h = self.held.get(*s as usize).copied().unwrap_or(NONE);
            if h != NONE {
                self.apply(h, -1)?;
            }
        }
        self.touched = t;
        self.touched.clear();
        Ok(())
    }

    /// [`SweepAcc::reset`] on lazy trees: every held trade's leaf is emptied (its path marked dirty, the path charged as a
    /// removal's), and the part, month and window cells are set to their empty values directly instead of being walked back
    /// one removal at a time. The result is the state the removals reach: every cell of the empty set is 0 (integer sums and the
    /// wrapping fingerprint return exactly to 0), every month cell is in sync, and only held trades contribute to any cell
    /// (a session removed earlier already gave its contribution back).
    fn reset_lazy(&mut self) -> Result<(), SweepErr> {
        self.epoch += 1;
        let t = std::mem::take(&mut self.touched);
        let np = self.lay.leaves.len();
        let nm = self.lay.months();
        for &s in &t {
            let h = self.held.get(s as usize).copied().unwrap_or(NONE);
            if h == NONE {
                continue;
            }
            let o = *self.out.get(h as usize).ok_or_else(unknown_row)?;
            let (part, leaf) = self.lay.sess.get(o.sess as usize).copied().ok_or_else(unknown_row)?;
            let (part, leaf) = (part as usize, leaf as usize);
            let m = usize::try_from(o.mk - self.lay.hull.0).map_err(|_| SweepErr::Overflow)?;
            for c in 0..self.cols {
                self.set_leaf(c, part, leaf, Node::EMPTY)?;
                *self.part_net.get_mut(c * np + part).ok_or_else(overflow)? = 0;
                *self.part_n.get_mut(c * np + part).ok_or_else(overflow)? = 0;
                let idx = c * nm + m;
                *self.month_net.get_mut(idx).ok_or_else(overflow)? = 0;
                *self.month_n.get_mut(idx).ok_or_else(overflow)? = 0;
                *self.msync_net.get_mut(idx).ok_or_else(overflow)? = 0;
                *self.msync_n.get_mut(idx).ok_or_else(overflow)? = 0;
                *self.mdirty.get_mut(idx).ok_or_else(overflow)? = false;
            }
            if let Some(x) = self.held.get_mut(s as usize) {
                *x = NONE;
            }
        }
        for c in 0..self.cols {
            let Some(list) = self.mlist.get_mut(c) else { continue };
            for &m in list.iter() {
                let idx = c * nm + m as usize;
                if let (Some(a), Some(b), Some(d)) = (self.msync_net.get_mut(idx), self.msync_n.get_mut(idx), self.mdirty.get_mut(idx)) {
                    (*a, *b, *d) = (0, 0, false);
                }
            }
            list.clear();
        }
        self.win.iter_mut().for_each(|x| *x = WinCell::default());
        self.touched = t;
        self.touched.clear();
        Ok(())
    }

    /// The learning facts of a window cell and the composed part roots.
    const fn facts(cell: WinCell, acc: Node) -> Learn {
        Learn {
            net: cell.net,
            trades: cell.trades,
            wins: cell.wins,
            max_dd: acc.dd.max_dd,
            worst: if acc.n == 0 { 0 } else { acc.worst },
            rd_bp: acc.rd,
            losing: cell.losing,
            months_traded: cell.m_traded,
            months_pos: cell.m_pos,
            fp: cell.fp,
            min_prefix: acc.dd.min_prefix,
        }
    }

    /// The learning facts of window `w`, column `col` (read only: a dirty node is composed from its children, never written).
    ///
    /// # Errors
    /// `Overflow`.
    pub fn learn(&self, w: usize, col: usize) -> Result<(Learn, u64), SweepErr> {
        let np = self.lay.leaves.len();
        let nw = self.lay.wins.len();
        let win = self.lay.wins.get(w).ok_or_else(overflow)?;
        let cell = if self.lazy { self.cell_now(w, col)? } else { self.win.get(col * nw + w).copied().ok_or_else(overflow)? };
        let mut acc = Node::EMPTY;
        let mut steps = 0_u64;
        for p in 0..win.parts {
            let root = match (self.trees.get(col * np + p), self.dirty.get(col * np + p)) {
                (Some(t), Some(d)) => value(t, d, 1)?,
                (Some(t), None) => t.get(1).copied().unwrap_or(Node::EMPTY),
                _ => Node::EMPTY,
            };
            acc = if self.lazy { acc.combine_fast(&root)? } else { acc.combine(&root)? };
            steps += 1;
        }
        Ok((Self::facts(cell, acc), steps))
    }

    /// [`SweepAcc::learn`] for a key read: re-merges the dirty nodes under the window's part roots (lazy trees), then memoises
    /// the facts until the next event. Same facts and same step count as `learn`.
    fn learn_fast(&mut self, w: usize, col: usize) -> Result<(Learn, u64), SweepErr> {
        let nw = self.lay.wins.len();
        let parts = self.lay.wins.get(w).ok_or_else(overflow)?.parts;
        let mi = col * nw + w;
        if let Some(&(e, l)) = self.memo.get(mi)
            && e == self.epoch
        {
            return Ok((l, u64::try_from(parts).map_err(|_| SweepErr::Overflow)?));
        }
        if self.lazy {
            let np = self.lay.leaves.len();
            for p in 0..parts {
                if let (Some(t), Some(d)) = (self.trees.get_mut(col * np + p), self.dirty.get_mut(col * np + p)) {
                    fix(t, d, 1, &mut self.merges_done)?;
                }
            }
            self.sync_months(col)?;
        }
        let r = self.learn(w, col)?;
        if let Some(m) = self.memo.get_mut(mi) {
            *m = (self.epoch, r.0);
        }
        Ok(r)
    }

    /// E1 / E2 / E4 for window `w`, column `col`, trade floor `floor` (at least `floor` trades in EVERY learning year part, and
    /// every part net positive).
    ///
    /// # Errors
    /// The first eligibility rule the cell fails.
    pub fn eligible(&self, w: usize, col: usize, floor: u32) -> Result<(), NotEligible> {
        let np = self.lay.leaves.len();
        let nw = self.lay.wins.len();
        let Some(win) = self.lay.wins.get(w) else { return Err(NotEligible::NoTrade) };
        let trades = self.win.get(col * nw + w).map_or(0, |c| c.trades);
        if trades == 0 {
            return Err(NotEligible::NoTrade);
        }
        for p in 0..win.parts {
            let n = self.part_n.get(col * np + p).copied().unwrap_or(0);
            let net = self.part_net.get(col * np + p).copied().unwrap_or(0);
            if cfg!(feature = "red") && n == 0 {
                // RED (lenient): a learning year part with no trade is dropped from E2.
                continue;
            }
            if n < floor.max(1) {
                return Err(if n == 0 { NotEligible::PartNotPositive } else { NotEligible::BelowFloor });
            }
            if net <= 0 {
                return Err(NotEligible::PartNotPositive);
            }
        }
        Ok(())
    }

    /// The REQ 73 key of window `w` from its learning facts (P16 order: K5 K7 K6 K8 K9 K10 K4).
    fn r73_key(&self, w: usize, l: &Learn) -> Result<Key, SweepErr> {
        let neg = |x: i64| x.checked_neg().ok_or_else(overflow);
        // The module's own crit values (common_wf::views::crit_val, P15): K7 = the worst trade's LOSS (0 when no trade lost);
        // K8 = the losing-month sum as a positive magnitude; K9 = 1 unless EVERY counted learning month is positive (P9).
        let k7 = if l.worst >= 0 { 0 } else { neg(l.worst)? };
        let counted = self.lay.counted.get(w).copied().unwrap_or(0);
        let k9 = i64::from(l.months_pos != counted);
        Ok(Key::of(&[l.max_dd, k7, i64::from(l.rd_bp), neg(l.losing)?, k9, i64::from(l.trades), neg(l.net)?]))
    }

    /// The key of window `w`, column `col` (`None` when not eligible under `floor`), its facts, and the read's step count.
    ///
    /// # Errors
    /// `Overflow`.
    pub fn key(&mut self, w: usize, col: usize, floor: u32) -> Result<(Result<Key, NotEligible>, Learn, u64), SweepErr> {
        self.key_reads += 1;
        if let Err(e) = self.eligible(w, col, floor) {
            return Ok((Err(e), Learn::default(), 1));
        }
        let (l, mut steps) = self.learn_fast(w, col)?;
        if cfg!(feature = "m_s14") {
            steps += std::mem::take(&mut self.pending);
        }
        let k = self.r73_key(w, &l)?;
        Ok((Ok(k), l, steps + 1))
    }
}

impl SweepAcc<'_> {
    /// The key of window `w`, column `col` under the view kind `kind` (floor `floor` for R73's E4), and the read's step count.
    /// Every part is `common_wf::views::crit_val`'s (SWEEP-EQ, `tests_wf.rs`): K1 = learning units not booked-positive, K16 =
    /// learning units with no trade, K17 = the money gap (1 - net over every failing unit with trades, +1 paisa per empty unit).
    ///
    /// # Errors
    /// `Overflow`.
    pub fn view_key(&mut self, w: usize, col: usize, floor: u32, kind: KeyKind) -> Result<(Result<Key, NotEligible>, u64), SweepErr> {
        self.view_key_gated(w, col, floor, kind, |_| false).map(|(k, s)| (k.map(|x| x.unwrap_or_default()), s))
    }

    /// [`SweepAcc::view_key`] with a GATE on the key's cheap leading parts: `rejects(prefix)` is asked with the parts known
    /// before the tree is read (`-N`: K4; `-MISS`: K1 K16 K17; `-R73` has none, K5 is the tree's); when it answers true (the
    /// caller's list would reject every key with that prefix: `Lists::rejects_prefix`), the tree is not read and `Ok(None)` is
    /// returned with the SAME step count and counters as the full read. On an eager accumulator (outside the exactness bound) and
    /// in the m_s14 mutant the gate is never asked. `Ok(Some(key))` is exactly `view_key`'s key.
    ///
    /// # Errors
    /// `Overflow`.
    pub fn view_key_gated(&mut self, w: usize, col: usize, floor: u32, kind: KeyKind, rejects: impl FnOnce(&[i64]) -> bool) -> Result<(Result<Option<Key>, NotEligible>, u64), SweepErr> {
        let gate = self.lazy && !cfg!(feature = "m_s14");
        match kind {
            KeyKind::R73 => self.key(w, col, floor).map(|(k, _, s)| (k.map(Some), s)),
            KeyKind::N => {
                self.key_reads += 1;
                if let Err(e) = self.eligible(w, col, 1) {
                    return Ok((Err(e), 1));
                }
                if gate {
                    let nw = self.lay.wins.len();
                    let parts = self.lay.wins.get(w).ok_or_else(overflow)?.parts;
                    let net = self.win.get(col * nw + w).ok_or_else(overflow)?.net;
                    if rejects(&[net.checked_neg().ok_or_else(overflow)?]) {
                        return Ok((Ok(None), u64::try_from(parts).map_err(|_| SweepErr::Overflow)? + 1));
                    }
                }
                let (l, mut steps) = self.learn_fast(w, col)?;
                if cfg!(feature = "m_s14") {
                    steps += std::mem::take(&mut self.pending);
                }
                let k = self.r73_key(w, &l)?;
                // K4 K6 K5 from the same parts (P16: -N's order on the standard drawdown base).
                let p = |i: usize| k.0.get(i).copied().unwrap_or(0);
                Ok((Ok(Some(Key::of(&[p(6), p(2), p(0)]))), steps + 1))
            }
            KeyKind::Miss => {
                self.key_reads += 1;
                let np = self.lay.leaves.len();
                let nw = self.lay.wins.len();
                let Some(win) = self.lay.wins.get(w) else { return Ok((Err(NotEligible::NoTrade), 1)) };
                let parts = win.parts;
                if self.win.get(col * nw + w).map_or(0, |c| c.trades) == 0 {
                    return Ok((Err(NotEligible::NoTrade), 1));
                }
                if self.eligible_std(w, col).is_ok() {
                    return Ok((Err(NotEligible::PassesE2), 1));
                }
                let (mut k1, mut k16, mut k17) = (0_i64, 0_i64, 0_i64);
                for p in 0..parts {
                    let n = self.part_n.get(col * np + p).copied().unwrap_or(0);
                    let net = self.part_net.get(col * np + p).copied().unwrap_or(0);
                    if n == 0 {
                        k1 += 1;
                        k16 += 1;
                        // MUTANT m_k17: an empty learning unit adds nothing to the money gap (common_wf adds 1 paisa).
                        k17 = add(k17, i64::from(!cfg!(feature = "m_k17")))?;
                    } else if net <= 0 {
                        k1 += 1;
                        k17 = add(k17, sub(1, net)?)?;
                    }
                }
                let extra = 1 + u64::try_from(parts).unwrap_or(0);
                if gate && rejects(&[k1, k16, k17]) {
                    return Ok((Ok(None), u64::try_from(parts).map_err(|_| SweepErr::Overflow)? + extra));
                }
                let (l, steps) = self.learn_fast(w, col)?;
                let neg = |x: i64| x.checked_neg().ok_or_else(overflow);
                let k7 = if l.worst >= 0 { 0 } else { neg(l.worst)? };
                let counted = self.lay.counted.get(w).copied().unwrap_or(0);
                let k9 = i64::from(l.months_pos != counted);
                let k = Key::of(&[k1, k16, k17, l.max_dd, k7, i64::from(l.rd_bp), neg(l.losing)?, k9, i64::from(l.trades), neg(l.net)?]);
                Ok((Ok(Some(k)), steps + extra))
            }
        }
    }

    /// E1 + E2 under the STANDARD reading (never the red build's lenient one): a MISS view's E7 is "not E2" as `common_wf` reads it.
    fn eligible_std(&self, w: usize, col: usize) -> Result<(), NotEligible> {
        let np = self.lay.leaves.len();
        let Some(win) = self.lay.wins.get(w) else { return Err(NotEligible::NoTrade) };
        for p in 0..win.parts {
            let n = self.part_n.get(col * np + p).copied().unwrap_or(0);
            let net = self.part_net.get(col * np + p).copied().unwrap_or(0);
            if n == 0 || net <= 0 {
                return Err(NotEligible::PartNotPositive);
            }
        }
        Ok(())
    }
}

/// The most learning year parts the grouped offer path holds on the stack (more parts take the per-view path).
const PMAX: usize = 16;

/// The most windows the grouped offer path counts on the stack (more windows take the per-view path).
const WMAX: usize = 8;

/// Where one grouped offer's results go: the per-view lists, the not-eligible counts and the key-read step tally.
pub struct OfferSink<'s> {
    /// One list per (view, column): index view * cols + column.
    pub lists: &'s mut [Lists],
    /// Not eligible counts: no trade, part not positive, below floor, passes E2 (a MISS view).
    pub ne: &'s mut [u64; 4],
    /// `tally[s]` = key reads of `s` steps for `s` below [`TALLY_MAX`] (the caller adds them to its histogram).
    pub tally: &'s mut Vec<u64>,
    /// Key reads of [`TALLY_MAX`] steps or more, one entry each (a mutant's pending charge; the caller adds each one).
    pub big: &'s mut Vec<u64>,
}

/// The step counts the tally holds by index (larger ones go to `OfferSink::big`, one entry each).
pub const TALLY_MAX: usize = 4096;

impl OfferSink<'_> {
    fn count(&mut self, steps: u64, n: u64) {
        let i = usize::try_from(steps).unwrap_or(usize::MAX);
        if i >= TALLY_MAX {
            for _ in 0..n {
                self.big.push(steps);
            }
            return;
        }
        if i >= self.tally.len() {
            self.tally.resize(i + 1, 0);
        }
        if let Some(x) = self.tally.get_mut(i) {
            *x += n;
        }
    }

    fn not_eligible(&mut self, e: NotEligible, n: u64) {
        let i = match e {
            NotEligible::NoTrade => 0,
            NotEligible::PartNotPositive => 1,
            NotEligible::BelowFloor => 2,
            NotEligible::PassesE2 => 3,
        };
        if let Some(x) = self.ne.get_mut(i) {
            *x += n;
        }
    }
}

/// E1 / E2 / E4 from a window's trades and its parts' (trades, net), exactly [`SweepAcc::eligible`]'s rule and order.
fn eligible_from(trades: u32, parts: &[(u32, i64)], floor: u32) -> Result<(), NotEligible> {
    if trades == 0 {
        return Err(NotEligible::NoTrade);
    }
    for &(n, net) in parts {
        if cfg!(feature = "red") && n == 0 {
            // RED (lenient): a learning year part with no trade is dropped from E2.
            continue;
        }
        if n < floor.max(1) {
            return Err(if n == 0 { NotEligible::PartNotPositive } else { NotEligible::BelowFloor });
        }
        if net <= 0 {
            return Err(NotEligible::PartNotPositive);
        }
    }
    Ok(())
}

impl SweepAcc<'_> {
    /// Offers the code run (`code`, `run` codes) to every (view, column) list at once: the host's offer loop (one
    /// `view_key_gated` per view and column, then `offer_learn` / `count_rejected`), grouped by (column, window) so that each
    /// window's trades and part cells are read once and every view of that window is decided from them. Every list receives
    /// exactly the offer the per-view loop gives it (lists are independent, so the order across lists is free), every key-read
    /// step count and every not-eligible reason is counted as there. In an eager accumulator, in the `m_s14` mutant (whose
    /// pending charge is taken by the first read in view order) and for a window of more than `PMAX` parts, the per-view loop
    /// itself runs.
    ///
    /// # Errors
    /// `Overflow`.
    pub fn offer_all(&mut self, views: &[View], code: Code, run: u64, sink: &mut OfferSink<'_>) -> Result<(), SweepErr> {
        let cols = self.cols;
        let nw = self.lay.wins.len();
        let grouped = self.lazy && !cfg!(feature = "m_s14") && nw <= WMAX && self.lay.wins.iter().all(|w| w.parts <= PMAX);
        if !grouped {
            for (vi, v) in views.iter().enumerate() {
                for c in 0..cols {
                    let mut list = sink.lists.get_mut(vi * cols + c);
                    let (k, st) = self.view_key_gated(v.win, c, v.floor, v.kind, |p| list.as_ref().is_some_and(|l| l.rejects_prefix(p)))?;
                    match k {
                        Ok(Some(key)) => {
                            if let Some(l) = list.as_mut() {
                                l.offer_learn(key, code, run);
                            }
                        }
                        Ok(None) => {
                            if let Some(l) = list.as_mut() {
                                l.count_rejected(run);
                            }
                        }
                        Err(e) => sink.not_eligible(e, 1),
                    }
                    sink.count(st, 1);
                }
            }
            return Ok(());
        }
        let np = self.lay.leaves.len();
        let mut buf = [(0_u32, 0_i64); PMAX];
        // The views of each window, counted once per offer.
        let mut nvw = [0_u64; WMAX];
        for v in views {
            if let Some(x) = nvw.get_mut(v.win) {
                *x += 1;
            }
        }
        for c in 0..cols {
            for w in 0..nw {
                let Some(win) = self.lay.wins.get(w) else { continue };
                let parts = win.parts;
                let nv = nvw.get(w).copied().unwrap_or(0);
                if nv == 0 {
                    continue;
                }
                self.key_reads += nv;
                let cell = self.win.get(c * nw + w).copied().ok_or_else(overflow)?;
                if cell.trades == 0 {
                    // E1 fails for every view of this window: one step each (view_key_gated's first refusal).
                    sink.not_eligible(NotEligible::NoTrade, nv);
                    sink.count(1, nv);
                    continue;
                }
                for (p, b) in buf.iter_mut().enumerate().take(parts) {
                    *b = (self.part_n.get(c * np + p).copied().unwrap_or(0), self.part_net.get(c * np + p).copied().unwrap_or(0));
                }
                let pc = buf.get(..parts).ok_or_else(overflow)?;
                let p64 = u64::try_from(parts).map_err(|_| SweepErr::Overflow)?;
                // E2 under the standard reading (MISS's E7) and the MISS key's leading parts K1 K16 K17, from the same cells.
                let std_e2 = pc.iter().all(|&(n, net)| n != 0 && net > 0);
                // The REQ 73 key of this (window, column) does not depend on the view's floor: composed once, at the first view
                // that needs it, with its read's step count (learn's steps + 1, the same for every view: learn_fast is a memo).
                let mut r73: Option<(Key, u64)> = None;
                for (vi, v) in views.iter().enumerate() {
                    if v.win != w {
                        continue;
                    }
                    let mut list = sink.lists.get_mut(vi * cols + c);
                    let (k, st): (Result<Option<Key>, NotEligible>, u64) = match v.kind {
                        KeyKind::R73 => match eligible_from(cell.trades, pc, v.floor) {
                            Err(e) => (Err(e), 1),
                            Ok(()) => {
                                let (k, s) = match r73 {
                                    Some(x) => x,
                                    None => {
                                        let (l, s) = self.learn_fast(w, c)?;
                                        let x = (self.r73_key(w, &l)?, s + 1);
                                        r73 = Some(x);
                                        x
                                    }
                                };
                                (Ok(Some(k)), s)
                            }
                        },
                        KeyKind::N => match eligible_from(cell.trades, pc, 1) {
                            Err(e) => (Err(e), 1),
                            Ok(()) => {
                                let k4 = cell.net.checked_neg().ok_or_else(overflow)?;
                                if list.as_ref().is_some_and(|l| l.rejects_prefix(&[k4])) {
                                    (Ok(None), p64 + 1)
                                } else {
                                    let (k, s) = match r73 {
                                        Some(x) => x,
                                        None => {
                                            let (l, s) = self.learn_fast(w, c)?;
                                            let x = (self.r73_key(w, &l)?, s + 1);
                                            r73 = Some(x);
                                            x
                                        }
                                    };
                                    // K4 K6 K5 from the same parts (P16: -N's order on the standard drawdown base).
                                    let p = |i: usize| k.0.get(i).copied().unwrap_or(0);
                                    (Ok(Some(Key::of(&[p(6), p(2), p(0)]))), s)
                                }
                            }
                        },
                        KeyKind::Miss => {
                            if std_e2 {
                                (Err(NotEligible::PassesE2), 1)
                            } else {
                                let (mut k1, mut k16, mut k17) = (0_i64, 0_i64, 0_i64);
                                for &(n, net) in pc {
                                    if n == 0 {
                                        k1 += 1;
                                        k16 += 1;
                                        // MUTANT m_k17: an empty learning unit adds nothing to the money gap (common_wf adds 1 paisa).
                                        k17 = add(k17, i64::from(!cfg!(feature = "m_k17")))?;
                                    } else if net <= 0 {
                                        k1 += 1;
                                        k17 = add(k17, sub(1, net)?)?;
                                    }
                                }
                                let extra = 1 + p64;
                                if list.as_ref().is_some_and(|l| l.rejects_prefix(&[k1, k16, k17])) {
                                    (Ok(None), p64 + extra)
                                } else {
                                    let (l, s) = self.learn_fast(w, c)?;
                                    let neg = |x: i64| x.checked_neg().ok_or_else(overflow);
                                    let k7 = if l.worst >= 0 { 0 } else { neg(l.worst)? };
                                    let counted = self.lay.counted.get(w).copied().unwrap_or(0);
                                    let k9 = i64::from(l.months_pos != counted);
                                    (Ok(Some(Key::of(&[k1, k16, k17, l.max_dd, k7, i64::from(l.rd_bp), neg(l.losing)?, k9, i64::from(l.trades), neg(l.net)?]))), s + extra)
                                }
                            }
                        }
                    };
                    match k {
                        Ok(Some(key)) => {
                            if let Some(l) = list.as_mut() {
                                l.offer_learn(key, code, run);
                            }
                        }
                        Ok(None) => {
                            if let Some(l) = list.as_mut() {
                                l.count_rejected(run);
                            }
                        }
                        Err(e) => sink.not_eligible(e, 1),
                    }
                    sink.count(st, 1);
                }
            }
        }
        Ok(())
    }
}
