//! `Acc`: what one subject (a combo x lane x partition) holds per window set while its trades are walked (6.1-6.2). Every `add`
//! is O(1): at most one session, month, part and piece close (each a fixed amount of work bounded by the files) plus the cell
//! updates; a piece close stores its PREFIX snapshot, so an anchored window's learning figures are one cell (B5 (b)). The year
//! UNITS (DV1) carry the booked / second / touched / E4-floor bits and the MISS money gap per window (F9), so eligibility and the
//! verdict are mask operations.

use std::sync::Arc;

use crate::date::{first_day, last_day, mk_of_day};
use crate::dd::Dd;
use crate::rules::{HoleSource, MtmMarks, RunRules, TradeOrder};
use crate::views::Views;
use crate::windows::{SetTab, Windows};
use crate::{Day, MAX_BARS, MAX_FLOORS, MAX_PARTS, MAX_PIECES, MAX_SET_WINDOWS, Mk, Paisa, Partition, RED, SetId, WfErr, WinId, add, sub};

/// One booked trade (P6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trade {
    /// The ENTRY session (carried and straddle counts).
    pub day: Day,
    /// The EXIT session: the booking session (D36).
    pub exit_day: Day,
    /// The settlement second inside the exit session.
    pub close_s: u32,
    /// Net paisa (the partition's reading).
    pub net: Paisa,
    /// Legacy only (`alt_reading = dated`); never read in v1.
    pub net_alt: Paisa,
    /// The worst run-down, bp.
    pub rd_bp: i32,
    /// The host's feed-hole flag (refused with `hole_source = none`).
    pub hole: bool,
    /// The trade id.
    pub id: u64,
    /// A carried trade's last session-end mark before its exit; 0 for every intraday trade (R3).
    pub mark_prev: Paisa,
}

/// A month cell (F9): the additive and class fields only; no drawdown, run-down or worst trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonthCell {
    /// The month.
    pub mk: Mk,
    /// Its net.
    pub net: Paisa,
    /// Legacy only.
    pub net_alt: Paisa,
    /// Its trades.
    pub trades: u32,
    /// Its winning trades.
    pub wins: u32,
    /// The winners' sum.
    pub win_net: Paisa,
    /// The losers' sum.
    pub loss_net: Paisa,
    /// The winning hole trades' sum.
    pub hole_win_net: Paisa,
    /// The winning hole trades.
    pub holes: u32,
}

/// The deterministic step counters of one call (section 12): identical at 1x / 10x / 100x for the same event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Steps {
    /// Day-table reads.
    pub reads: u32,
    /// Order / exit checks.
    pub checks: u32,
    /// Session, month, part and piece closes.
    pub closes: u32,
    /// Prefix combines.
    pub combines: u32,
    /// Cell updates.
    pub updates: u32,
    /// Unit updates (bounded by the windows file).
    pub unit_updates: u32,
    /// Read-set compares, eligibility masks, key criteria and inserts (offer).
    pub offer_ops: u32,
}
impl Steps {
    /// The total.
    #[must_use]
    pub const fn total(&self) -> u32 {
        self.reads + self.checks + self.closes + self.combines + self.updates + self.unit_updates + self.offer_ops
    }
    /// Adds another call's steps.
    pub const fn plus(&mut self, o: &Self) {
        self.reads += o.reads;
        self.checks += o.checks;
        self.closes += o.closes;
        self.combines += o.combines;
        self.updates += o.updates;
        self.unit_updates += o.unit_updates;
        self.offer_ops += o.offer_ops;
    }
}

/// The cells of one piece (and, combined, of a learning part): every field combines left to right exactly (6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    /// Net.
    pub net: i64,
    /// Trades.
    pub trades: u64,
    /// Sessions with a trade.
    pub sessions: u64,
    /// Winning trades.
    pub wins: u64,
    /// The winners' sum.
    pub win_net: i64,
    /// The losers' sum (<= 0).
    pub loss_net: i64,
    /// The worst trade (i64::MAX = none).
    pub worst_trade: i64,
    /// The largest worst run-down, bp (0 = none).
    pub worst_rd: i64,
    /// The winning hole trades' sum.
    pub hole_win: i64,
    /// The winning hole trades.
    pub hole_wins: u64,
    /// Carried trades.
    pub carried: u64,
    /// Straddling trades.
    pub straddling: u64,
    /// Traded months.
    pub m_traded: u64,
    /// Positive months.
    pub m_pos: u64,
    /// Zero months with trades.
    pub m_zero: u64,
    /// Negative months.
    pub m_neg: u64,
    /// The worst traded month (i64::MAX = none).
    pub m_worst: i64,
    /// The sum of -net over negative months.
    pub losing: i64,
    /// Traded months below each bar of the views file.
    pub below: [u64; MAX_BARS],
    /// The drawdown over trades.
    pub dd_trade: Dd,
    /// The drawdown over session nets.
    pub dd_session: Dd,
    /// The drawdown over the mark-to-market equity (R3).
    pub dd_mtm: Dd,
    /// The last session-end mark (R3, U13).
    pub mark_end: i64,
    /// A mark was fed in this cell.
    pub marked: bool,
    /// The order-free learning fingerprint (P18, S5).
    pub fp: u64,
}
impl Cell {
    /// The identity.
    pub const EMPTY: Self = Self {
        net: 0,
        trades: 0,
        sessions: 0,
        wins: 0,
        win_net: 0,
        loss_net: 0,
        worst_trade: i64::MAX,
        worst_rd: 0,
        hole_win: 0,
        hole_wins: 0,
        carried: 0,
        straddling: 0,
        m_traded: 0,
        m_pos: 0,
        m_zero: 0,
        m_neg: 0,
        m_worst: i64::MAX,
        losing: 0,
        below: [0; MAX_BARS],
        dd_trade: Dd::ZERO,
        dd_session: Dd::ZERO,
        dd_mtm: Dd::ZERO,
        mark_end: 0,
        marked: false,
        fp: 0,
    };
    /// `self` followed by `b`.
    ///
    /// # Errors
    /// `Overflow`.
    pub fn combine(&self, b: &Self) -> Result<Self, WfErr> {
        let mut below = [0u64; MAX_BARS];
        for (i, x) in below.iter_mut().enumerate() {
            *x = self.below.get(i).copied().unwrap_or(0) + b.below.get(i).copied().unwrap_or(0);
        }
        Ok(Self {
            net: add(self.net, b.net)?,
            trades: self.trades + b.trades,
            sessions: self.sessions + b.sessions,
            wins: self.wins + b.wins,
            win_net: add(self.win_net, b.win_net)?,
            loss_net: add(self.loss_net, b.loss_net)?,
            worst_trade: self.worst_trade.min(b.worst_trade),
            worst_rd: self.worst_rd.max(b.worst_rd),
            hole_win: add(self.hole_win, b.hole_win)?,
            hole_wins: self.hole_wins + b.hole_wins,
            carried: self.carried + b.carried,
            straddling: self.straddling + b.straddling,
            m_traded: self.m_traded + b.m_traded,
            m_pos: self.m_pos + b.m_pos,
            m_zero: self.m_zero + b.m_zero,
            m_neg: self.m_neg + b.m_neg,
            m_worst: self.m_worst.min(b.m_worst),
            losing: add(self.losing, b.losing)?,
            below,
            dd_trade: self.dd_trade.combine(&b.dd_trade)?,
            dd_session: self.dd_session.combine(&b.dd_session)?,
            dd_mtm: self.dd_mtm.combine(&b.dd_mtm)?,
            mark_end: if b.marked { b.mark_end } else { self.mark_end },
            marked: self.marked || b.marked,
            fp: self.fp.wrapping_add(b.fp),
        })
    }
}

/// One year unit's booked figures (W02 / W10 items, W4 rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitCell {
    /// Net.
    pub net: i64,
    /// Trades.
    pub trades: u64,
    /// The winning hole trades' sum.
    pub hole_win: i64,
    /// The winning hole trades.
    pub hole_wins: u64,
    /// The best trade (i64::MIN = none).
    pub best: i64,
    /// Carried trades.
    pub carried: u64,
    /// Straddling trades.
    pub straddling: u64,
}
impl UnitCell {
    /// The identity.
    pub const EMPTY: Self = Self { net: 0, trades: 0, hole_win: 0, hole_wins: 0, best: i64::MIN, carried: 0, straddling: 0 };
}

/// The compact facts of a subject in one window set (6.1, F12): year-unit cells, the masks and the identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    /// The window set.
    pub set: SetId,
    /// The partition.
    pub partition: Partition,
    /// The identity hash (windows, views, RULES, crate).
    pub ident: u64,
    /// The year units' cells.
    pub units: Vec<UnitCell>,
    /// Units positive as booked (P8).
    pub booked: u64,
    /// Units positive in the second reading (P7).
    pub second: u64,
    /// Units with a trade.
    pub touched: u64,
    /// Units wholly outside the data span (F11).
    pub nodata: u64,
    /// Units with no walked session (D7).
    pub missing: u64,
}

/// The learning part of one window (P20): learning cells and learning masks only; no test field exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Learn {
    /// The combined learning cell.
    pub cell: Cell,
    /// Counted learning months (P9).
    pub counted: u64,
    /// Walked learning sessions inside the data span (E5).
    pub walked_sessions: u64,
    /// The learning units.
    pub units: u64,
    /// Booked-positive units.
    pub booked: u64,
    /// Touched units.
    pub touched: u64,
    /// Per E4 floor of the views file: units with at least that many trades.
    pub atleast: [u64; MAX_FLOORS],
    /// The MISS money gap of the learning units with trades (F9).
    pub gap: i64,
}

/// The accumulator of one subject in one window set.
#[derive(Clone, Debug)]
pub struct Acc {
    tab: Arc<SetTab>,
    order: TradeOrder,
    holes: HoleSource,
    marks: MtmMarks,
    /// RULES declare carried trades (`carried=yes`) or session-end marks: a carried trade may be booked (F2).
    carried_ok: bool,
    pub(crate) bars: [i64; MAX_BARS],
    pub(crate) nbars: usize,
    pub(crate) floors: [u64; MAX_FLOORS],
    pub(crate) nfloors: usize,
    ident: u64,
    span: (Day, Day),
    counted_piece: [u64; MAX_PIECES],
    sessions_piece: [u64; MAX_PIECES],
    counted_prefix: [u64; MAX_PIECES + 1],
    sessions_prefix: [u64; MAX_PIECES + 1],
    nodata: u64,
    pieces: [Cell; MAX_PIECES],
    snaps: [Cell; MAX_PIECES],
    run: Cell,
    closed: u32,
    units: [UnitCell; MAX_PARTS],
    booked: u64,
    second: u64,
    touched: u64,
    atleast: [u64; MAX_FLOORS],
    contrib: [i64; MAX_PARTS],
    gap: [i64; MAX_SET_WINDOWS],
    open_day: Option<Day>,
    sess_net: i64,
    open_month: Option<Mk>,
    month_net: i64,
    month_trades: u64,
    open_part: Option<u16>,
    part: UnitCell,
    open_piece: Option<u8>,
    cur: Cell,
    last: Option<(Day, u32, i64)>,
    last_mark: Option<Day>,
    last_month_fed: Option<Mk>,
    m_in_path: i64,
    pub(crate) month_fed: bool,
    traded: bool,
    finished: bool,
}

fn bits(mut m: u64) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        if m == 0 {
            None
        } else {
            let i = m.trailing_zeros();
            m &= m - 1;
            usize::try_from(i).ok()
        }
    })
}
/// The order-free fingerprint hash of one trade (P18, S5; the node reference's `tradeHash`).
#[must_use]
pub fn trade_hash(t: &Trade) -> u64 {
    let mut h: u64 = 0x243f_6a88_85a3_08d3;
    for v in [i64::from(t.day).cast_unsigned(), i64::from(t.exit_day).cast_unsigned(), u64::from(t.close_s), t.net.cast_unsigned(), t.id] {
        h = crate::mix(h ^ v);
    }
    h
}

impl Acc {
    /// A fresh accumulator of window set `w`: the bars and E4 floors from the views file; the order, holes and marks from RULES.
    #[must_use]
    pub fn new(w: &Windows, v: &Views, r: &RunRules) -> Self {
        let mut bars = [0i64; MAX_BARS];
        for (i, b) in v.bars.iter().take(MAX_BARS).enumerate() {
            if let Some(x) = bars.get_mut(i) {
                *x = *b;
            }
        }
        let mut floors = [0u64; MAX_FLOORS];
        for (i, f) in v.floors.iter().take(MAX_FLOORS).enumerate() {
            if let Some(x) = floors.get_mut(i) {
                *x = *f;
            }
        }
        let tab = Arc::clone(&w.tab);
        let span = (first_day(tab.period.0), last_day(tab.period.1));
        let mut a = Self {
            ident: crate::mix(tab.ident ^ v.ident.rotate_left(21) ^ r.ident.rotate_left(42)),
            tab,
            order: r.trade_order,
            holes: r.hole_source,
            marks: r.mtm_marks,
            carried_ok: r.carried || r.mtm_marks == MtmMarks::SessionEnd,
            bars,
            nbars: v.bars.len().min(MAX_BARS),
            floors,
            nfloors: v.floors.len().min(MAX_FLOORS),
            span,
            counted_piece: [0; MAX_PIECES],
            sessions_piece: [0; MAX_PIECES],
            counted_prefix: [0; MAX_PIECES + 1],
            sessions_prefix: [0; MAX_PIECES + 1],
            nodata: 0,
            pieces: [Cell::EMPTY; MAX_PIECES],
            snaps: [Cell::EMPTY; MAX_PIECES],
            run: Cell::EMPTY,
            closed: 0,
            units: [UnitCell::EMPTY; MAX_PARTS],
            booked: 0,
            second: 0,
            touched: 0,
            atleast: [0; MAX_FLOORS],
            contrib: [0; MAX_PARTS],
            gap: [0; MAX_SET_WINDOWS],
            open_day: None,
            sess_net: 0,
            open_month: None,
            month_net: 0,
            month_trades: 0,
            open_part: None,
            part: UnitCell::EMPTY,
            open_piece: None,
            cur: Cell::EMPTY,
            last: None,
            last_mark: None,
            last_month_fed: None,
            m_in_path: 0,
            month_fed: false,
            traded: false,
            finished: false,
        };
        a.statics();
        a
    }

    /// The set this accumulator belongs to.
    #[must_use]
    pub fn set_id(&self) -> SetId {
        self.tab.set
    }
    /// The identity hash (F12).
    #[must_use]
    pub const fn ident(&self) -> u64 {
        self.ident
    }

    /// The counted months and walked sessions per piece and the nodata units under the current data span (P9, F11): a pass over
    /// the period's months, bounded by the files, once per subject (never inside `add`).
    fn statics(&mut self) {
        let cal = Arc::clone(&self.tab.cal);
        let (s0, s1) = self.span;
        for (j, &(a, b)) in self.tab.pieces.iter().enumerate() {
            let mut counted = 0u64;
            for mk in a..=b {
                let (d0, d1) = (first_day(mk).max(s0), last_day(mk).min(s1));
                if d0 <= d1 && cal.walked_in(d0, d1) > 0 {
                    counted += 1;
                }
            }
            let sessions = u64::from(cal.walked_in(first_day(a).max(s0), last_day(b).min(s1)));
            if let Some(x) = self.counted_piece.get_mut(j) {
                *x = counted;
            }
            if let Some(x) = self.sessions_piece.get_mut(j) {
                *x = sessions;
            }
        }
        for j in 0..MAX_PIECES {
            let c = self.counted_prefix.get(j).copied().unwrap_or(0) + self.counted_piece.get(j).copied().unwrap_or(0);
            let s = self.sessions_prefix.get(j).copied().unwrap_or(0) + self.sessions_piece.get(j).copied().unwrap_or(0);
            if let Some(x) = self.counted_prefix.get_mut(j + 1) {
                *x = c;
            }
            if let Some(x) = self.sessions_prefix.get_mut(j + 1) {
                *x = s;
            }
        }
        self.nodata = self.tab.units.iter().enumerate().filter(|(_, u)| last_day(u.to).min(s1) < first_day(u.from).max(s0)).fold(0u64, |m, (i, _)| m | (1u64 << i));
    }

    /// Clears the subject state (only what was touched).
    pub fn reset(&mut self) {
        for j in bits(u64::from(self.closed)) {
            if let Some(x) = self.pieces.get_mut(j) {
                *x = Cell::EMPTY;
            }
            if let Some(x) = self.snaps.get_mut(j) {
                *x = Cell::EMPTY;
            }
        }
        for u in bits(self.touched) {
            if let Some(x) = self.units.get_mut(u) {
                *x = UnitCell::EMPTY;
            }
            if let Some(x) = self.contrib.get_mut(u) {
                *x = 0;
            }
        }
        self.gap = [0; MAX_SET_WINDOWS];
        self.run = Cell::EMPTY;
        self.closed = 0;
        self.booked = 0;
        self.second = 0;
        self.touched = 0;
        self.atleast = [0; MAX_FLOORS];
        self.open_day = None;
        self.sess_net = 0;
        self.open_month = None;
        self.month_net = 0;
        self.month_trades = 0;
        self.open_part = None;
        self.part = UnitCell::EMPTY;
        self.open_piece = None;
        self.cur = Cell::EMPTY;
        self.last = None;
        self.last_mark = None;
        self.last_month_fed = None;
        self.m_in_path = 0;
        self.month_fed = false;
        self.traded = false;
        self.finished = false;
        let full = (first_day(self.tab.period.0), last_day(self.tab.period.1));
        if self.span != full {
            self.span = full;
            self.statics();
        }
    }

    /// Sets the subject's data span (F11): parts wholly outside it read "no data for this instrument"; months count inside it.
    ///
    /// # Errors
    /// `BadDay` when `from > to`.
    pub fn set_data_span(&mut self, from: Day, to: Day) -> Result<(), WfErr> {
        if from > to {
            return Err(WfErr::BadDay);
        }
        self.span = (from, to);
        self.statics();
        Ok(())
    }

    fn month_index(&self, mk: Mk) -> Option<usize> {
        usize::try_from(mk - self.tab.period.0).ok().filter(|&i| i < self.tab.month_piece.len())
    }

    fn close_session(&mut self, st: &mut Steps) -> Result<(), WfErr> {
        if self.open_day.take().is_some() {
            self.cur.dd_session.push(self.sess_net)?;
            self.cur.sessions += 1;
            self.sess_net = 0;
            st.closes += 1;
        }
        Ok(())
    }
    fn close_month(&mut self, st: &mut Steps) -> Result<(), WfErr> {
        if self.open_month.take().is_some() && self.month_trades > 0 {
            let n = self.month_net;
            let c = &mut self.cur;
            c.m_traded += 1;
            if n > 0 {
                c.m_pos += 1;
            } else if n == 0 {
                c.m_zero += 1;
            } else {
                c.m_neg += 1;
                c.losing = sub(c.losing, n)?;
            }
            c.m_worst = c.m_worst.min(n);
            for i in 0..self.nbars {
                if let (Some(b), Some(x)) = (self.bars.get(i), c.below.get_mut(i))
                    && n < *b
                {
                    *x += 1;
                }
            }
            st.closes += 1;
        }
        self.month_net = 0;
        self.month_trades = 0;
        Ok(())
    }
    fn close_part(&mut self, st: &mut Steps) -> Result<(), WfErr> {
        let Some(pi) = self.open_part.take() else { return Ok(()) };
        if self.part.trades > 0 {
            let p = self.part;
            let mask = self.tab.part_units.get(usize::from(pi)).copied().unwrap_or(0);
            for u in bits(mask) {
                self.unit_merge(u, &p)?;
                st.unit_updates += 1;
            }
            st.closes += 1;
        }
        self.part = UnitCell::EMPTY;
        Ok(())
    }
    fn unit_merge(&mut self, u: usize, p: &UnitCell) -> Result<(), WfErr> {
        let holes = self.holes;
        let Some(c) = self.units.get_mut(u) else { return Err(WfErr::TooManyParts) };
        c.net = add(c.net, p.net)?;
        c.trades += p.trades;
        c.hole_win = add(c.hole_win, p.hole_win)?;
        c.hole_wins += p.hole_wins;
        c.best = c.best.max(p.best);
        c.carried += p.carried;
        c.straddling += p.straddling;
        let c = *c;
        let bit = 1u64 << u;
        self.touched |= bit;
        let booked = c.trades > 0 && if cfg!(feature = "m7") { c.net >= 0 } else { c.net > 0 };
        let second = booked && (holes == HoleSource::None || sub(c.net, c.hole_win)? > 0);
        self.booked = if booked { self.booked | bit } else { self.booked & !bit };
        self.second = if second { self.second | bit } else { self.second & !bit };
        for i in 0..self.nfloors {
            if let (Some(f), Some(m)) = (self.floors.get(i), self.atleast.get_mut(i)) {
                *m = if c.trades >= *f { *m | bit } else { *m & !bit };
            }
        }
        let new = if c.trades > 0 && c.net <= 0 { sub(1, c.net)? } else { 0 };
        let old = self.contrib.get(u).copied().unwrap_or(0);
        let delta = sub(new, old)?;
        if let Some(x) = self.contrib.get_mut(u) {
            *x = new;
        }
        for w in bits(self.tab.unit_learn_windows.get(u).copied().unwrap_or(0)) {
            if let Some(g) = self.gap.get_mut(w) {
                *g = add(*g, delta)?;
            }
        }
        Ok(())
    }
    fn close_piece(&mut self, st: &mut Steps) -> Result<(), WfErr> {
        let Some(j) = self.open_piece.take() else { return Ok(()) };
        if self.cur.trades > 0 || self.cur.marked || self.cur.m_traded > 0 {
            let j = usize::from(j);
            self.run = self.run.combine(&self.cur)?;
            if let Some(x) = self.pieces.get_mut(j) {
                *x = self.cur;
            }
            if let Some(x) = self.snaps.get_mut(j) {
                *x = self.run;
            }
            self.closed |= 1u32 << j;
            st.closes += 1;
            st.combines += 1;
        }
        self.cur = Cell::EMPTY;
        Ok(())
    }
    /// Closes the open cells before `day` / month `mk` as `add` steps 2-5 do (6.2), then opens `mk`'s month, part and piece.
    fn step_to(&mut self, day: Day, mk: Mk, trade: bool, st: &mut Steps) -> Result<(), WfErr> {
        if self.open_day != Some(day) {
            self.close_session(st)?;
        }
        let mi = self.month_index(mk).ok_or(WfErr::TradeOutsidePeriod)?;
        let piece = self.tab.month_piece.get(mi).copied().ok_or(WfErr::TradeOutsidePeriod)?;
        let part = self.tab.month_part.get(mi).copied().ok_or(WfErr::TradeOutsidePeriod)?;
        if self.open_month != Some(mk) {
            self.close_month(st)?;
        }
        if self.open_part != Some(part) {
            self.close_part(st)?;
        }
        if self.open_piece != Some(piece) {
            self.close_piece(st)?;
        }
        self.open_month = Some(mk);
        self.open_part = Some(part);
        self.open_piece = Some(piece);
        if trade {
            self.open_day = Some(day);
        }
        Ok(())
    }
    fn piece_of_month(&self, mk: Mk) -> Option<u8> {
        self.month_index(mk).and_then(|i| self.tab.month_piece.get(i).copied())
    }

    /// Books one trade at its EXIT session (P6, D36): O(1), at most one close of each kind plus the cell updates (S-ADD).
    ///
    /// # Errors
    /// `TradeOutsidePeriod`, `TradeOnUnwalkedSession`, `HoleWithoutSource`, `OutOfOrder`, `CarriedUndeclared` (a carried
    /// trade in a run whose RULES declare neither `carried=yes` nor session-end marks: P6 / R3, repair F2), `NeedsTrades`
    /// (after month cells), `Overflow`.
    pub fn add(&mut self, t: &Trade) -> Result<Steps, WfErr> {
        let mut st = Steps { reads: 1, checks: 1, ..Steps::default() };
        if t.net == i64::MIN || t.mark_prev == i64::MIN {
            return Err(WfErr::Overflow);
        }
        let book_day = if cfg!(feature = "m22") { t.day } else { t.exit_day };
        let em = mk_of_day(book_day);
        if em < self.tab.period.0 || em > self.tab.period.1 {
            if RED {
                return Ok(st);
            }
            return Err(WfErr::TradeOutsidePeriod);
        }
        if !self.tab.cal.walked(t.exit_day) {
            return Err(WfErr::TradeOnUnwalkedSession);
        }
        if t.hole && self.holes == HoleSource::None {
            return Err(WfErr::HoleWithoutSource);
        }
        if t.exit_day < self.span.0 || t.exit_day > self.span.1 {
            return Err(WfErr::TradeOutsidePeriod);
        }
        if t.day > t.exit_day || self.finished {
            return Err(WfErr::OutOfOrder);
        }
        if t.exit_day > t.day && !self.carried_ok && !cfg!(feature = "m49") {
            return Err(WfErr::CarriedUndeclared);
        }
        if self.month_fed {
            return Err(WfErr::NeedsTrades);
        }
        if let Some((d, s, n)) = self.last
            && (t.exit_day < d || (self.order == TradeOrder::SeqLossesFirst && t.exit_day == d && (t.close_s, t.net) < (s, n)))
        {
            return Err(WfErr::OutOfOrder);
        }
        if self.last_mark.is_some_and(|m| t.exit_day <= m) {
            return Err(WfErr::OutOfOrder);
        }
        let delta = sub(t.net, t.mark_prev)?;
        let in_path = sub(self.m_in_path, t.mark_prev)?;
        self.step_to(book_day, em, true, &mut st)?;
        let carried = t.exit_day > t.day;
        let straddle = carried && self.piece_of_month(mk_of_day(t.day)).is_none_or(|p| Some(p) < self.open_piece);
        let hole_win = t.hole && t.net > 0;
        let c = &mut self.cur;
        c.net = add(c.net, t.net)?;
        c.trades += 1;
        if t.net > 0 {
            c.wins += 1;
            c.win_net = add(c.win_net, t.net)?;
        } else if t.net < 0 {
            c.loss_net = add(c.loss_net, t.net)?;
        }
        c.worst_trade = c.worst_trade.min(t.net);
        c.worst_rd = c.worst_rd.max(i64::from(t.rd_bp));
        if hole_win {
            c.hole_win = add(c.hole_win, t.net)?;
            c.hole_wins += 1;
        }
        c.carried += u64::from(carried);
        c.straddling += u64::from(straddle);
        c.fp = if cfg!(feature = "m38") { c.fp.rotate_left(5) ^ trade_hash(t) } else { c.fp.wrapping_add(trade_hash(t)) };
        c.dd_trade.push(t.net)?;
        if self.marks == MtmMarks::SessionEnd {
            c.dd_mtm.push(delta)?;
        }
        self.m_in_path = in_path;
        self.sess_net = add(self.sess_net, t.net)?;
        self.month_net = add(self.month_net, t.net)?;
        self.month_trades += 1;
        let p = &mut self.part;
        p.net = add(p.net, t.net)?;
        p.trades += 1;
        if hole_win {
            p.hole_win = add(p.hole_win, t.net)?;
            p.hole_wins += 1;
        }
        p.best = p.best.max(t.net);
        p.carried += u64::from(carried);
        p.straddling += u64::from(straddle);
        self.last = Some((t.exit_day, t.close_s, t.net));
        self.traded = true;
        st.updates += 1;
        Ok(st)
    }

    /// One session-end mark of the open positions (R3; X1): first closes every cell before `day` exactly as `add` does, so a
    /// test-session mark never enters a learning prefix, then one step into `day`'s piece `dd_mtm`. Books nothing.
    ///
    /// # Errors
    /// `MarksWithoutRule`, `TradeOutsidePeriod`, `MarkOnUnwalkedSession`, `OutOfOrder`, `Overflow`.
    pub fn add_session_mtm(&mut self, day: Day, open_mtm: Paisa) -> Result<Steps, WfErr> {
        let mut st = Steps { reads: 1, checks: 1, ..Steps::default() };
        if self.marks == MtmMarks::None {
            return Err(WfErr::MarksWithoutRule);
        }
        if open_mtm == i64::MIN {
            return Err(WfErr::Overflow);
        }
        let mk = mk_of_day(day);
        if mk < self.tab.period.0 || mk > self.tab.period.1 {
            return Err(WfErr::TradeOutsidePeriod);
        }
        if !self.tab.cal.walked(day) {
            return Err(WfErr::MarkOnUnwalkedSession);
        }
        if self.finished || self.last.is_some_and(|(d, _, _)| day < d) || self.last_mark.is_some_and(|m| day < m) {
            return Err(WfErr::OutOfOrder);
        }
        let delta = sub(open_mtm, self.m_in_path)?;
        if !cfg!(feature = "m34") || self.open_piece.is_none() {
            self.step_to(day, mk, false, &mut st)?;
        }
        self.cur.dd_mtm.push(delta)?;
        self.cur.mark_end = open_mtm;
        self.cur.marked = true;
        self.m_in_path = open_mtm;
        self.last_mark = Some(day);
        st.updates += 1;
        Ok(st)
    }

    /// One month cell (F9): the additive and class fields only. A view reading K5, K6 or K7 refuses a month-fed `Acc`.
    ///
    /// # Errors
    /// `TradeOutsidePeriod`, `OutOfOrder`, `NeedsTrades` (after trades), `HoleWithoutSource`, `Overflow`.
    pub fn add_month(&mut self, c: &MonthCell) -> Result<Steps, WfErr> {
        let mut st = Steps { reads: 1, checks: 1, ..Steps::default() };
        if self.traded || self.last_mark.is_some() {
            return Err(WfErr::NeedsTrades);
        }
        if c.mk < self.tab.period.0 || c.mk > self.tab.period.1 {
            return Err(WfErr::TradeOutsidePeriod);
        }
        if self.finished || self.last_month_fed.is_some_and(|m| c.mk <= m) {
            return Err(WfErr::OutOfOrder);
        }
        if c.holes > 0 && self.holes == HoleSource::None {
            return Err(WfErr::HoleWithoutSource);
        }
        self.month_fed = true;
        self.last_month_fed = Some(c.mk);
        self.step_to(first_day(c.mk), c.mk, false, &mut st)?;
        let x = &mut self.cur;
        x.net = add(x.net, c.net)?;
        x.trades += u64::from(c.trades);
        x.wins += u64::from(c.wins);
        x.win_net = add(x.win_net, c.win_net)?;
        x.loss_net = add(x.loss_net, c.loss_net)?;
        x.hole_win = add(x.hole_win, c.hole_win_net)?;
        x.hole_wins += u64::from(c.holes);
        self.month_net = c.net;
        self.month_trades = u64::from(c.trades);
        let p = &mut self.part;
        p.net = add(p.net, c.net)?;
        p.trades += u64::from(c.trades);
        p.hole_win = add(p.hole_win, c.hole_win_net)?;
        p.hole_wins += u64::from(c.holes);
        self.close_month(&mut st)?;
        self.open_month = None;
        st.updates += 1;
        Ok(st)
    }

    /// Closes the open cells of every month before `mk` (a learn-first host, 5.1).
    ///
    /// # Errors
    /// `Overflow`.
    pub fn close_until(&mut self, mk: Mk) -> Result<Steps, WfErr> {
        let mut st = Steps::default();
        if self.open_month.is_some_and(|m| m < mk) {
            self.close_session(&mut st)?;
            self.close_month(&mut st)?;
            let target = self.month_index(mk).and_then(|i| self.tab.month_part.get(i).copied());
            if target != self.open_part {
                self.close_part(&mut st)?;
            }
            if self.piece_of_month(mk) != self.open_piece {
                self.close_piece(&mut st)?;
            }
        }
        Ok(st)
    }

    /// Closes the open session, month, part and piece (S-FINISH).
    ///
    /// # Errors
    /// `Overflow`.
    pub fn finish(&mut self) -> Result<Steps, WfErr> {
        let mut st = Steps::default();
        self.close_session(&mut st)?;
        self.close_month(&mut st)?;
        self.close_part(&mut st)?;
        self.close_piece(&mut st)?;
        self.finished = true;
        Ok(st)
    }

    /// Has the walk passed `w`'s learning end?
    #[must_use]
    pub fn learn_ready(&self, w: WinId) -> bool {
        let Some(s) = self.tab.spec.get(usize::from(w.0)).and_then(Option::as_ref) else { return false };
        self.finished || s.learn_end_piece.is_some_and(|e| self.open_piece.is_none_or(|p| p > e))
    }

    /// The learning part of `w` (P20): an anchored row reads ONE prefix cell; another row a bounded fold over its pieces.
    ///
    /// # Errors
    /// `SetMismatch` (a window of another set), `OutOfOrder` (the walk has not passed the learning end), `Overflow`.
    pub fn learn(&self, w: WinId) -> Result<Learn, WfErr> {
        let s = self.tab.spec.get(usize::from(w.0)).and_then(Option::as_ref).ok_or(WfErr::SetMismatch)?;
        if !self.learn_ready(w) {
            return Err(WfErr::OutOfOrder);
        }
        let e = s.learn_end_piece.ok_or(WfErr::KeyReadsTest)?;
        let e = if cfg!(feature = "m17") { (e + 1).min(31) } else { e };
        let upto = u64::from(self.closed) & ((2u64 << e) - 1);
        let (cell, counted, sessions) = if s.anchored {
            let cell = if upto == 0 { Cell::EMPTY } else { self.snaps.get(usize::try_from(63 - upto.leading_zeros()).unwrap_or(0)).copied().unwrap_or(Cell::EMPTY) };
            let k = usize::from(e) + 1;
            (cell, self.counted_prefix.get(k).copied().unwrap_or(0), self.sessions_prefix.get(k).copied().unwrap_or(0))
        } else {
            let mut cell = Cell::EMPTY;
            let (mut counted, mut sessions) = (0u64, 0u64);
            for j in bits(u64::from(s.learn_pieces)) {
                if self.closed & (1u32 << j) != 0 {
                    cell = cell.combine(self.pieces.get(j).unwrap_or(&Cell::EMPTY))?;
                }
                counted += self.counted_piece.get(j).copied().unwrap_or(0);
                sessions += self.sessions_piece.get(j).copied().unwrap_or(0);
            }
            (cell, counted, sessions)
        };
        let mut atleast = [0u64; MAX_FLOORS];
        for (i, x) in atleast.iter_mut().enumerate().take(self.nfloors) {
            *x = self.atleast.get(i).copied().unwrap_or(0) & s.learn_units;
        }
        let units = if cfg!(feature = "m4") { s.learn_units | s.test_list.first().map_or(0, |u| 1u64 << u) } else { s.learn_units };
        Ok(Learn {
            cell,
            counted,
            walked_sessions: sessions,
            units,
            booked: self.booked & units,
            touched: self.touched & units,
            atleast,
            gap: self.gap.get(usize::from(s.local)).copied().unwrap_or(0),
        })
    }

    /// The test months of `w` for W24: (positive, counted, the worst month or None when no month counts).
    ///
    /// # Errors
    /// `SetMismatch`, `Overflow`.
    pub fn test_months(&self, w: WinId) -> Result<(u64, u64, Option<i64>), WfErr> {
        let s = self.tab.spec.get(usize::from(w.0)).and_then(Option::as_ref).ok_or(WfErr::SetMismatch)?;
        let mut cell = Cell::EMPTY;
        let mut counted = 0u64;
        for j in bits(u64::from(s.test_pieces)) {
            if self.closed & (1u32 << j) != 0 {
                cell = cell.combine(self.pieces.get(j).unwrap_or(&Cell::EMPTY))?;
            }
            counted += self.counted_piece.get(j).copied().unwrap_or(0);
        }
        let worst = if counted == 0 {
            None
        } else if cell.m_traded < counted {
            Some(cell.m_worst.min(0))
        } else {
            Some(cell.m_worst)
        };
        Ok((cell.m_pos, counted, worst))
    }

    /// The compact facts for partition `p` (F12).
    #[must_use]
    pub fn facts(&self, p: Partition) -> Facts {
        let n = self.tab.units.len().min(MAX_PARTS);
        Facts {
            set: self.tab.set,
            partition: p,
            ident: self.ident,
            units: self.units.get(..n).map_or_else(Vec::new, <[UnitCell]>::to_vec),
            booked: self.booked,
            second: self.second,
            touched: self.touched,
            nodata: self.nodata,
            missing: self.tab.unit_missing,
        }
    }
    /// The bounded fold of the closed pieces in `mask` (PREFIX-EQ's other side; the rolling-row path).
    ///
    /// # Errors
    /// `Overflow`.
    pub fn fold_pieces(&self, mask: u32) -> Result<Cell, WfErr> {
        let mut cell = Cell::EMPTY;
        for j in bits(u64::from(mask & self.closed)) {
            cell = cell.combine(self.pieces.get(j).unwrap_or(&Cell::EMPTY))?;
        }
        Ok(cell)
    }
    /// The set tables.
    #[must_use]
    pub fn tab(&self) -> &SetTab {
        &self.tab
    }
    /// The bars and their counters' order (K13).
    pub(crate) fn bar_index(&self, bar: i64) -> Option<usize> {
        self.bars.get(..self.nbars).and_then(|b| b.iter().position(|x| *x == bar))
    }
    /// The floor index of an E4 level.
    pub(crate) fn floor_index(&self, n: u64) -> Option<usize> {
        self.floors.get(..self.nfloors).and_then(|f| f.iter().position(|x| *x == n))
    }
}
