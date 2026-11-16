//! v3.2-B (MAIN-RULINGS-ENGINE-v22 Addendum 18; job v32-par, agent build:v32-B-par, Addendum 21.1): SPLIT PERMUTATIONS from the ONE walk.
//!
//! The sweep keeps, per account and run MONTH, one [`MPiece`]: the net, the realized levels and the max drawdown inside the month (under
//! the account's own drawdown reading: grouped same-second closes when DD_GROUPED is on), the trades, the positive sessions and positive
//! week pieces with the worst of each, and the cash-rule slack bounds. It is filled as the walk realizes each close (O(1) per close; a
//! month is closed once). A WINDOW of months a..=b composes EXACTLY (net adds; the drawdown of A then B = max(dd_A, dd_B, up_A - net_A -
//! low_B); counts add; the worst = the minimum), read in O(1) from a disjoint sparse table built once per account (O(M log M), M months),
//! so every split costs O(1) per window per account and NO account is walked a second time.
//!
//! Weeks: a calendar week that straddles a month end is cut there (two WEEK PIECES, one per month), so a window holds whole pieces of its
//! own months only: a learning window never reads a day outside its learning months (the no-look-ahead rule).
//!
//! Splits (from the run's own months; a split needs a learning month AND a test month in the run, else NOT RUN):
//! - `TT-<Y>`: train/test cut at the end of year Y = 2019..2024: learn = the run's first month .. Dec Y, test = Jan Y+1 .. the last month.
//! - `WF-AN-L<L>-S<S>` (anchored) and `WF-RO-L<L>-S<S>` (rolling), L = 6 / 12 / 24 learning months, S = 1 / 3 / 6-month steps: fold k
//!   tests months L + kS .. L + kS + S - 1 after learning months 0 .. L + kS - 1 (anchored) or kS .. L + kS - 1 (rolling).
//!
//! Each split CHOOSES by the addendum 15 / 19 order on its learning months only: the most months >= Rs 2,00,000 net, then the most
//! positive week pieces, then the most positive sessions, then the smallest max drawdown, then the most net (SEARCH settings, R1; ties by
//! the static tie key). Way B (the headline method, operator 9 Nov): the account chosen on the FIRST learning window, unchanged on every
//! later month. Way A (re-chosen at every step on that step's learning months): a LABELLED COMPARISON only.
//!
//! A test window PASSES (addendum 19 minimum) when every month >= Rs 2,00,000 net, every week piece and every session positive (a
//! session or week piece without a trade counts as Rs 0: failing); the addendum 15 money bar alone (every month >= Rs 2,00,000) beside.
//!
//! Per kept combo (every account any view keeps): its test months under every split that ran (TT: after the cut; walk-forward: from
//! month L on; for ONE combo the step and anchored / rolling change only the chooser, never its test months), the pass count, the
//! walk-forward pass count, the worst and the median split; PASSED only if it passes every walk-forward split that ran and at least 80%
//! of all splits that ran.
//!
//! Window figures read the ONE continuous walk (the account from the run's first day). A FRESH Rs 20 L account started at a later month
//! could decide differently only through the cash rule (past data only, never a look-ahead): the slack test of `wf.rs` is applied to every
//! chosen account-window and the windows it cannot prove equal are COUNTED and printed (never re-walked: no second walk).

use std::cmp::Ordering;
use std::fmt::Write as _;

use crate::acct::Sum;

/// "None" for the u16 day / week-piece indices.
pub const NONE16: u16 = u16::MAX;
/// The train/test cut years.
pub const TT_YEARS: [i64; 6] = [2019, 2020, 2021, 2022, 2023, 2024];
/// The walk-forward learning lengths (months) and steps (months).
pub const WF_L: [u8; 3] = [6, 12, 24];
pub const WF_S: [u8; 3] = [1, 3, 6];

/// The run calendar of the splits (set once per run under SPLITS18).
#[derive(Clone, Debug, Default)]
pub struct SCal {
    /// Run day index -> day number.
    pub days: Vec<i64>,
    /// Run day index -> month index (dense over the months that hold a run session).
    pub mo_of_day: Vec<u8>,
    /// Run day index -> week piece index (a calendar week cut at month ends).
    pub wp_of_day: Vec<u16>,
    /// Month index -> month id (year x 12 + month - 1).
    pub month_id: Vec<i64>,
    /// Month index -> sessions and week pieces.
    pub n_sess: Vec<u16>,
    pub n_wp: Vec<u16>,
    /// Week piece -> its month, first and last run day index.
    pub mo_of_wp: Vec<u8>,
    pub wp_first: Vec<u16>,
    pub wp_last: Vec<u16>,
    /// v3.2 round B (the day-major memory budget): the SEGMENTS of the split figures = runs of whole months between consecutive split
    /// boundaries (every split's first test month: the train/test cuts and the walk-forward learning lengths). Month index -> segment, and
    /// each segment's first and last month. Every window the way B choice and the per-combo statistics read (a learning window from the
    /// run's first month, a test window to its last month) is a run of WHOLE segments, so an account keeps one record per segment (at most
    /// [`MAXSEG`]) instead of one per month.
    pub seg_of_mo: Vec<u8>,
    pub seg_lo: Vec<u8>,
    pub seg_hi: Vec<u8>,
}

/// The most segments a run can have: 6 train/test cuts + 3 walk-forward learning lengths = 9 boundaries.
pub const MAXSEG: usize = 10;

impl SCal {
    /// The calendar of a run's days (ascending, distinct; at most 255 months).
    ///
    /// # Errors
    /// Days not ascending, more than 255 months or 65,534 sessions.
    pub fn from_days(days: &[i64]) -> Result<Self, String> {
        if days.windows(2).any(|w| w.first() >= w.get(1)) {
            return Err("split calendar: the run days are not strictly ascending".to_owned());
        }
        if days.len() >= usize::from(NONE16) {
            return Err("split calendar: more than 65,534 sessions".to_owned());
        }
        let mut c = Self { days: days.to_vec(), ..Self::default() };
        let (mut last_m, mut last_w) = (i64::MIN, i64::MIN);
        for (i, &d) in days.iter().enumerate() {
            let m = crate::v32::month_id(d);
            let w = crate::v32::week_of(d);
            if m != last_m {
                if c.month_id.len() >= 255 {
                    return Err("split calendar: more than 255 months".to_owned());
                }
                c.month_id.push(m);
                c.n_sess.push(0);
                c.n_wp.push(0);
            }
            let mi = c.month_id.len() - 1;
            if m != last_m || w != last_w {
                c.mo_of_wp.push(u8::try_from(mi).unwrap_or(u8::MAX));
                c.wp_first.push(u16::try_from(i).unwrap_or(NONE16));
                c.wp_last.push(u16::try_from(i).unwrap_or(NONE16));
                if let Some(x) = c.n_wp.get_mut(mi) {
                    *x += 1;
                }
            }
            if let Some(x) = c.wp_last.last_mut() {
                *x = u16::try_from(i).unwrap_or(NONE16);
            }
            if let Some(x) = c.n_sess.get_mut(mi) {
                *x += 1;
            }
            c.mo_of_day.push(u8::try_from(mi).unwrap_or(u8::MAX));
            c.wp_of_day.push(u16::try_from(c.wp_first.len() - 1).unwrap_or(NONE16));
            last_m = m;
            last_w = w;
        }
        c.set_segments();
        Ok(c)
    }

    /// v3.2 round B: cuts the run's months at every split boundary (each split's first test month) into at most [`MAXSEG`] segments.
    fn set_segments(&mut self) {
        let m = self.m();
        let mut b: Vec<usize> = splits(self).iter().filter_map(|s| s.folds.first().map(|f| f.1.0)).filter(|&x| x > 0 && x < m).collect();
        // MUTANT m_v32rb_seg_boundary_missing: the walk-forward 24-month boundary left out of the segments
        if cfg!(feature = "m_v32rb_seg_boundary_missing") {
            b.retain(|&x| x != 24);
        }
        b.sort_unstable();
        b.dedup();
        b.truncate(MAXSEG - 1);
        self.seg_of_mo = vec![0; m];
        self.seg_lo.clear();
        self.seg_hi.clear();
        let mut lo = 0_usize;
        for &x in b.iter().chain(std::iter::once(&m)) {
            if x > lo {
                let g = u8::try_from(self.seg_lo.len()).unwrap_or(u8::MAX);
                self.seg_lo.push(u8::try_from(lo).unwrap_or(u8::MAX));
                self.seg_hi.push(u8::try_from(x - 1).unwrap_or(u8::MAX));
                for s in self.seg_of_mo.get_mut(lo..x).unwrap_or(&mut []) {
                    *s = g;
                }
                lo = x;
            }
        }
    }

    /// The number of segments.
    #[must_use]
    pub fn nseg(&self) -> usize {
        self.seg_lo.len()
    }

    /// The sessions and week pieces of a run of months a..=b.
    fn counts(&self, a: usize, b: usize) -> (u16, u16) {
        let s = self.n_sess.get(a..=b).map_or(0, |x| x.iter().map(|&v| u32::from(v)).sum::<u32>());
        let w = self.n_wp.get(a..=b).map_or(0, |x| x.iter().map(|&v| u32::from(v)).sum::<u32>());
        (u16::try_from(s).unwrap_or(u16::MAX), u16::try_from(w).unwrap_or(u16::MAX))
    }

    /// The number of months.
    #[must_use]
    pub fn m(&self) -> usize {
        self.month_id.len()
    }

    /// "YYYY-MM" of a month index.
    #[must_use]
    pub fn month_name(&self, i: usize) -> String {
        self.month_id.get(i).map_or_else(|| "?".to_owned(), |&m| crate::v32::month_name(m))
    }

    /// The date of a run day index.
    #[must_use]
    pub fn day_name(&self, i: u16) -> String {
        self.days.get(usize::from(i)).map_or_else(|| "-".to_owned(), |&d| crate::cal::fmt_day(d))
    }

    /// The first .. last session of a week piece.
    #[must_use]
    pub fn wp_name(&self, w: u16) -> String {
        match (self.wp_first.get(usize::from(w)), self.wp_last.get(usize::from(w))) {
            (Some(&a), Some(&b)) => format!("{}..{}", self.day_name(a), self.day_name(b)),
            _ => "-".to_owned(),
        }
    }
}

#[cfg(not(test))]
static CAL: std::sync::OnceLock<&'static SCal> = std::sync::OnceLock::new();
#[cfg(test)]
thread_local! {
    static CAL: std::cell::Cell<Option<&'static SCal>> = const { std::cell::Cell::new(None) };
}

/// The run's split calendar (None until set).
#[must_use]
pub fn cal() -> Option<&'static SCal> {
    #[cfg(not(test))]
    {
        CAL.get().copied()
    }
    #[cfg(test)]
    {
        CAL.with(std::cell::Cell::get)
    }
}

/// Sets the run's split calendar (once; a test thread sets its own).
pub fn set_cal(c: SCal) {
    let r: &'static SCal = Box::leak(Box::new(c));
    #[cfg(not(test))]
    {
        let _ = CAL.set(r);
    }
    #[cfg(test)]
    {
        CAL.with(|x| x.set(Some(r)));
    }
}

/// One account's figures of one run month (levels relative to the month's start).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MPiece {
    pub net: i64,
    pub up: i64,
    pub low: i64,
    pub mdd: i64,
    /// The cash-rule slack bounds (wf.rs): the smallest slack of a passed check, the largest of a refused one.
    pub min_pass: i64,
    pub max_ref: i64,
    /// The worst session and week piece with a trade (i64::MAX: none) and where.
    pub d_min: i64,
    pub wk_min: i64,
    pub trades: u32,
    /// Sessions / week pieces with a trade, and of them positive.
    pub d_seen: u8,
    pub d_pos: u8,
    pub wk_seen: u8,
    pub wk_pos: u8,
    pub d_min_at: u16,
    pub wk_min_at: u16,
}

impl Default for MPiece {
    fn default() -> Self {
        Self { net: 0, up: 0, low: 0, mdd: 0, min_pass: i64::MAX, max_ref: i64::MIN, d_min: i64::MAX, wk_min: i64::MAX, trades: 0, d_seen: 0, d_pos: 0, wk_seen: 0, wk_pos: 0, d_min_at: NONE16, wk_min_at: NONE16 }
    }
}

/// The month series of one account (empty = splits off: every hook returns at once).
#[derive(Clone, Debug, Default)]
pub struct MSeries {
    pub m: Vec<MPiece>,
    cal: Option<&'static SCal>,
    /// The realized level at the current month's start.
    base: i64,
    cur_m: u8,
    cur_d: u16,
    d_net: i64,
    cur_wp: u16,
    wp_net: i64,
}

impl MSeries {
    /// A series for the run's months when SPLITS18 is on and the calendar is set; else empty (inert).
    #[must_use]
    pub fn new() -> Self {
        match cal() {
            Some(c) if crate::sw::on(crate::sw::SPLITS18) && c.m() > 0 => Self { m: vec![MPiece::default(); c.m()], cal: Some(c), base: 0, cur_m: u8::MAX, cur_d: NONE16, d_net: 0, cur_wp: NONE16, wp_net: 0 },
            _ => Self::default(),
        }
    }

    /// The series as the kernel keeps it: None (nothing allocated, every hook skipped) unless SPLITS18 is on with a calendar.
    #[must_use]
    pub fn boxed() -> Option<Box<Self>> {
        let s = Self::new();
        s.on().then(|| Box::new(s))
    }

    /// Whether the series is kept.
    #[inline]
    #[must_use]
    pub fn on(&self) -> bool {
        !self.m.is_empty()
    }

    fn seal_day(&mut self) {
        let Some(c) = self.cal else { return };
        if self.cur_d == NONE16 {
            return;
        }
        let d = self.cur_d;
        let mi = usize::from(c.mo_of_day.get(usize::from(d)).copied().unwrap_or(u8::MAX));
        if let Some(x) = self.m.get_mut(mi) {
            x.d_seen = x.d_seen.saturating_add(1);
            if self.d_net > 0 {
                x.d_pos = x.d_pos.saturating_add(1);
            }
            if self.d_net < x.d_min {
                x.d_min = self.d_net;
                x.d_min_at = d;
            }
        }
        self.wp_net += self.d_net;
        self.d_net = 0;
        self.cur_d = NONE16;
    }

    fn seal_wp(&mut self) {
        let Some(c) = self.cal else { return };
        if self.cur_wp == NONE16 {
            return;
        }
        let w = self.cur_wp;
        let mi = usize::from(c.mo_of_wp.get(usize::from(w)).copied().unwrap_or(u8::MAX));
        if let Some(x) = self.m.get_mut(mi) {
            x.wk_seen = x.wk_seen.saturating_add(1);
            if self.wp_net > 0 {
                x.wk_pos = x.wk_pos.saturating_add(1);
            }
            if self.wp_net < x.wk_min {
                x.wk_min = self.wp_net;
                x.wk_min_at = w;
            }
        }
        self.wp_net = 0;
        self.cur_wp = NONE16;
    }

    /// One realized close (`key` = run day x 86,400 + second; `dd`: update the levels now, as the year pieces do).
    #[inline]
    pub fn close(&mut self, key: u32, realized_after: i64, net: i64, dd: bool) {
        if self.m.is_empty() {
            return;
        }
        let Some(c) = self.cal else { return };
        let d = u16::try_from(key / 86_400).unwrap_or(NONE16);
        if d != self.cur_d {
            self.seal_day();
            let w = c.wp_of_day.get(usize::from(d)).copied().unwrap_or(NONE16);
            if w != self.cur_wp {
                self.seal_wp();
                self.cur_wp = w;
            }
            self.cur_d = d;
        }
        let mo = c.mo_of_day.get(usize::from(d)).copied().unwrap_or(u8::MAX);
        if mo != self.cur_m {
            self.base = realized_after - net;
            self.cur_m = mo;
        }
        let base = self.base;
        if let Some(x) = self.m.get_mut(usize::from(mo)) {
            x.net += net;
            x.trades += 1;
            if dd {
                let lvl = realized_after - base;
                x.up = x.up.max(lvl);
                x.low = x.low.min(lvl);
                x.mdd = x.mdd.max(x.up - lvl);
            }
        }
        self.d_net += net;
    }

    /// The last close of a second under the grouped reading (v3.1 item 5): the month's levels at the realized level after the group.
    #[inline]
    pub fn group_end(&mut self, key: u32, realized_after: i64) {
        // MUTANT m_v32b_split_group_end_skipped: the grouped reading's level update at the end of a second left out of the month series
        if self.m.is_empty() || cfg!(feature = "m_v32b_split_group_end_skipped") {
            return;
        }
        let Some(c) = self.cal else { return };
        let mo = c.mo_of_day.get(usize::try_from(key / 86_400).unwrap_or(usize::MAX)).copied().unwrap_or(u8::MAX);
        if mo != self.cur_m {
            return;
        }
        let base = self.base;
        if let Some(x) = self.m.get_mut(usize::from(mo)) {
            let lvl = realized_after - base;
            x.up = x.up.max(lvl);
            x.low = x.low.min(lvl);
            x.mdd = x.mdd.max(x.up - lvl);
        }
    }

    /// One cash check of run day `di` (its slack; passed or refused): the fresh-start test's bounds.
    #[inline]
    pub fn cash(&mut self, di: u16, slack: i64, passed: bool) {
        if self.m.is_empty() {
            return;
        }
        let Some(c) = self.cal else { return };
        let mo = usize::from(c.mo_of_day.get(usize::from(di)).copied().unwrap_or(u8::MAX));
        if let Some(x) = self.m.get_mut(mo) {
            if passed {
                x.min_pass = x.min_pass.min(slack);
            } else {
                x.max_ref = x.max_ref.max(slack);
            }
        }
    }

    /// The months with the open session and week piece closed (on a copy: the walk's state is not changed).
    #[must_use]
    pub fn sealed(&self) -> Vec<MPiece> {
        let mut s = self.clone();
        s.seal_day();
        s.seal_wp();
        s.m
    }
}

/// v3.2 round B: one account's figures of one SEGMENT (a run of whole months between split boundaries; levels relative to the segment's
/// start). The fields compose to exactly what [`Fig18::then`] gives over the segment's months (named test
/// `v32rb_seg_equals_month_windows`): the worst month among the months with a close (the earliest on ties) plus the first month of the
/// segment WITHOUT a close (net Rs 0), the worst session and week piece with a close, the counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SPiece {
    pub net: i64,
    pub up: i64,
    pub low: i64,
    pub mdd: i64,
    /// The worst month / week piece / session WITH a close (i64::MAX: none).
    pub mo_min: i64,
    pub wk_min: i64,
    pub d_min: i64,
    pub trades: u32,
    pub wk_seen: u16,
    pub wk_pos: u16,
    pub d_seen: u16,
    pub d_pos: u16,
    pub wk_min_at: u16,
    pub d_min_at: u16,
    /// Months with a close, of them >= Rs 2,00,000; where the worst one is; the first month of the segment without a close (u8::MAX: none
    /// found yet); the last month sealed.
    pub mo_seen: u8,
    pub mo_ok: u8,
    pub mo_min_at: u8,
    pub mo_gap: u8,
    pub mo_last: u8,
}

impl Default for SPiece {
    fn default() -> Self {
        Self { net: 0, up: 0, low: 0, mdd: 0, mo_min: i64::MAX, wk_min: i64::MAX, d_min: i64::MAX, trades: 0, wk_seen: 0, wk_pos: 0, d_seen: 0, d_pos: 0, wk_min_at: NONE16, d_min_at: NONE16, mo_seen: 0, mo_ok: 0, mo_min_at: u8::MAX, mo_gap: u8::MAX, mo_last: u8::MAX }
    }
}

/// v3.2 round B: the SEGMENT series of one account (one [`SPiece`] per segment, inline: ONE allocation of about 850 bytes per account
/// instead of 80 bytes per month). O(1) per close; a month, session and week piece is sealed once into its segment (O(1) per closed month
/// per window: the windows are composed from at most [`MAXSEG`] segments at the account's finish).
#[derive(Clone, Debug)]
pub struct SSeries {
    pub s: [SPiece; MAXSEG],
    cal: &'static SCal,
    /// The realized level at the current segment's start.
    base: i64,
    m_net: i64,
    d_net: i64,
    wp_net: i64,
    cur_d: u16,
    cur_wp: u16,
    cur_g: u8,
    cur_m: u8,
}

impl SSeries {
    /// A fresh series for the run's calendar.
    #[must_use]
    pub fn new(c: &'static SCal) -> Self {
        Self { s: [SPiece::default(); MAXSEG], cal: c, base: 0, m_net: 0, d_net: 0, wp_net: 0, cur_d: NONE16, cur_wp: NONE16, cur_g: u8::MAX, cur_m: u8::MAX }
    }

    fn seg_of_mo(&self, mo: u8) -> usize {
        usize::from(self.cal.seg_of_mo.get(usize::from(mo)).copied().unwrap_or(0)).min(MAXSEG - 1)
    }

    fn seal_day(&mut self) {
        if self.cur_d == NONE16 {
            return;
        }
        let d = self.cur_d;
        let g = self.seg_of_mo(self.cal.mo_of_day.get(usize::from(d)).copied().unwrap_or(0));
        if let Some(x) = self.s.get_mut(g) {
            x.d_seen = x.d_seen.saturating_add(1);
            if self.d_net > 0 {
                x.d_pos = x.d_pos.saturating_add(1);
            }
            if self.d_net < x.d_min {
                x.d_min = self.d_net;
                x.d_min_at = d;
            }
        }
        self.wp_net += self.d_net;
        self.d_net = 0;
        self.cur_d = NONE16;
    }

    fn seal_wp(&mut self) {
        if self.cur_wp == NONE16 {
            return;
        }
        let w = self.cur_wp;
        let g = self.seg_of_mo(self.cal.mo_of_wp.get(usize::from(w)).copied().unwrap_or(0));
        if let Some(x) = self.s.get_mut(g) {
            x.wk_seen = x.wk_seen.saturating_add(1);
            if self.wp_net > 0 {
                x.wk_pos = x.wk_pos.saturating_add(1);
            }
            if self.wp_net < x.wk_min {
                x.wk_min = self.wp_net;
                x.wk_min_at = w;
            }
        }
        self.wp_net = 0;
        self.cur_wp = NONE16;
    }

    fn seal_month(&mut self) {
        if self.cur_m == u8::MAX {
            return;
        }
        let mo = self.cur_m;
        let g = self.seg_of_mo(mo);
        let lo = self.cal.seg_lo.get(g).copied().unwrap_or(0);
        if let Some(x) = self.s.get_mut(g) {
            // the first month of the segment without a close: the gap before this month, if any
            // MUTANT m_v32rb_seg_gap_ignored: months without a close never become the worst month
            if x.mo_gap == u8::MAX && !cfg!(feature = "m_v32rb_seg_gap_ignored") {
                let expect = if x.mo_seen == 0 { lo } else { x.mo_last.saturating_add(1) };
                if mo > expect {
                    x.mo_gap = expect;
                }
            }
            x.mo_seen = x.mo_seen.saturating_add(1);
            x.mo_last = mo;
            if crate::kern::month_ok(self.m_net) {
                x.mo_ok = x.mo_ok.saturating_add(1);
            }
            if self.m_net < x.mo_min {
                x.mo_min = self.m_net;
                x.mo_min_at = mo;
            }
        }
        self.m_net = 0;
        self.cur_m = u8::MAX;
    }

    /// One realized close (`key` = run day x 86,400 + second; `dd`: update the levels now, as the year pieces do).
    #[inline]
    pub fn close(&mut self, key: u32, realized_after: i64, net: i64, dd: bool) {
        let c = self.cal;
        let d = u16::try_from(key / 86_400).unwrap_or(NONE16);
        if d != self.cur_d {
            self.seal_day();
            let w = c.wp_of_day.get(usize::from(d)).copied().unwrap_or(NONE16);
            if w != self.cur_wp {
                self.seal_wp();
                self.cur_wp = w;
            }
            self.cur_d = d;
        }
        let mo = c.mo_of_day.get(usize::from(d)).copied().unwrap_or(u8::MAX);
        if mo != self.cur_m {
            self.seal_month();
            self.cur_m = mo;
        }
        let g = u8::try_from(self.seg_of_mo(mo)).unwrap_or(0);
        // MUTANT m_v32rb_seg_base_per_month: the level base reset at every month instead of at the segment's start
        if g != self.cur_g || (cfg!(feature = "m_v32rb_seg_base_per_month") && self.m_net == 0) {
            self.base = realized_after - net;
            self.cur_g = g;
        }
        let base = self.base;
        if let Some(x) = self.s.get_mut(usize::from(g)) {
            x.net += net;
            x.trades += 1;
            if dd {
                let lvl = realized_after - base;
                x.up = x.up.max(lvl);
                x.low = x.low.min(lvl);
                x.mdd = x.mdd.max(x.up - lvl);
            }
        }
        self.m_net += net;
        self.d_net += net;
    }

    /// The last close of a second under the grouped reading (v3.1 item 5): the segment's levels at the realized level after the group.
    #[inline]
    pub fn group_end(&mut self, key: u32, realized_after: i64) {
        // MUTANT m_v32b_split_group_end_skipped: the grouped reading's level update at the end of a second left out of the split figures
        if cfg!(feature = "m_v32b_split_group_end_skipped") {
            return;
        }
        let mo = self.cal.mo_of_day.get(usize::try_from(key / 86_400).unwrap_or(usize::MAX)).copied().unwrap_or(u8::MAX);
        let g = u8::try_from(self.seg_of_mo(mo)).unwrap_or(0);
        if g != self.cur_g {
            return;
        }
        let base = self.base;
        if let Some(x) = self.s.get_mut(usize::from(g)) {
            let lvl = realized_after - base;
            x.up = x.up.max(lvl);
            x.low = x.low.min(lvl);
            x.mdd = x.mdd.max(x.up - lvl);
        }
    }

    /// Every segment's figures (the open session, week piece and month sealed on a copy; the walk's state is not changed).
    #[must_use]
    pub fn figs(&self) -> Vec<Fig18> {
        let mut s = self.clone();
        s.seal_day();
        s.seal_wp();
        s.seal_month();
        let c = self.cal;
        (0..c.nseg().min(MAXSEG))
            .map(|g| {
                let (lo, hi) = (usize::from(c.seg_lo.get(g).copied().unwrap_or(0)), usize::from(c.seg_hi.get(g).copied().unwrap_or(0)));
                let x = s.s.get(g).copied().unwrap_or_default();
                // the segment's first month without a close (the trailing gap after its last sealed month)
                let mut gap = x.mo_gap;
                if gap == u8::MAX && !cfg!(feature = "m_v32rb_seg_gap_ignored") {
                    let expect = if x.mo_seen == 0 { lo } else { usize::from(x.mo_last) + 1 };
                    if expect <= hi {
                        gap = u8::try_from(expect).unwrap_or(u8::MAX);
                    }
                }
                // the worst month: the smallest net, the earliest on ties (a month without a close = Rs 0)
                let (mut mo_min, mut mo_at) = (x.mo_min, if x.mo_seen == 0 { NONE16 } else { u16::from(x.mo_min_at) });
                if gap != u8::MAX && (0 < mo_min || (0 == mo_min && u16::from(gap) < mo_at)) {
                    mo_min = 0;
                    mo_at = u16::from(gap);
                }
                let (d_n, wk_n) = c.counts(lo, hi);
                Fig18 {
                    net: x.net,
                    mdd: x.mdd,
                    up: x.up,
                    low: x.low,
                    mo_min,
                    wk_min: x.wk_min,
                    d_min: x.d_min,
                    min_pass: i64::MAX,
                    max_ref: i64::MIN,
                    trades: x.trades,
                    mo_ok: u16::from(x.mo_ok),
                    mo_n: u16::try_from(hi + 1 - lo).unwrap_or(0),
                    wk_seen: x.wk_seen,
                    wk_pos: x.wk_pos,
                    wk_n,
                    d_seen: x.d_seen,
                    d_pos: x.d_pos,
                    d_n,
                    mo_min_at: mo_at,
                    wk_min_at: x.wk_min_at,
                    d_min_at: x.d_min_at,
                }
            })
            .collect()
    }
}

/// v3.2 round B: the split state an account keeps under SPLITS18: the segment series (always) and the month series (only under WAYA18,
/// whole mode: the way A comparison needs every month).
#[derive(Clone, Debug)]
pub struct Ser18 {
    pub s: SSeries,
    pub m: Option<MSeries>,
}

impl Ser18 {
    /// The state as the kernel keeps it: None (nothing allocated, every hook skipped) unless SPLITS18 is on with a calendar of at least one
    /// month.
    #[must_use]
    pub fn boxed() -> Option<Box<Self>> {
        let c = cal()?;
        if !crate::sw::on(crate::sw::SPLITS18) || c.m() == 0 {
            return None;
        }
        let m = if crate::sw::on(crate::sw::WAYA18) { Some(MSeries::new()) } else { None };
        Some(Box::new(Self { s: SSeries::new(c), m }))
    }

    /// One realized close (both series).
    #[inline]
    pub fn close(&mut self, key: u32, realized_after: i64, net: i64, dd: bool) {
        self.s.close(key, realized_after, net, dd);
        if let Some(m) = self.m.as_mut() {
            m.close(key, realized_after, net, dd);
        }
    }

    /// The last close of a second under the grouped reading (both series).
    #[inline]
    pub fn group_end(&mut self, key: u32, realized_after: i64) {
        self.s.group_end(key, realized_after);
        if let Some(m) = self.m.as_mut() {
            m.group_end(key, realized_after);
        }
    }

    /// One cash check (the month series' fresh-start bounds; the segment windows all start at the run's first month or are read on the
    /// continuous walk, so they need none).
    #[inline]
    pub fn cash(&mut self, di: u16, slack: i64, passed: bool) {
        if let Some(m) = self.m.as_mut() {
            m.cash(di, slack, passed);
        }
    }
}

/// The composed figures of a window of months.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fig18 {
    pub net: i64,
    pub mdd: i64,
    pub up: i64,
    pub low: i64,
    pub mo_min: i64,
    pub wk_min: i64,
    pub d_min: i64,
    pub min_pass: i64,
    pub max_ref: i64,
    pub trades: u32,
    pub mo_ok: u16,
    pub mo_n: u16,
    pub wk_seen: u16,
    pub wk_pos: u16,
    pub wk_n: u16,
    pub d_seen: u16,
    pub d_pos: u16,
    pub d_n: u16,
    pub mo_min_at: u16,
    pub wk_min_at: u16,
    pub d_min_at: u16,
}

impl Default for Fig18 {
    /// The identity of [`Fig18::then`] (an empty window).
    fn default() -> Self {
        Self { net: 0, mdd: 0, up: 0, low: 0, mo_min: i64::MAX, wk_min: i64::MAX, d_min: i64::MAX, min_pass: i64::MAX, max_ref: i64::MIN, trades: 0, mo_ok: 0, mo_n: 0, wk_seen: 0, wk_pos: 0, wk_n: 0, d_seen: 0, d_pos: 0, d_n: 0, mo_min_at: NONE16, wk_min_at: NONE16, d_min_at: NONE16 }
    }
}

fn min_at(a: (i64, u16), b: (i64, u16)) -> (i64, u16) {
    if b.0 < a.0 { b } else { a }
}

impl Fig18 {
    /// One month's figures.
    #[must_use]
    pub fn of(x: &MPiece, i: usize, c: &SCal) -> Self {
        Self {
            net: x.net,
            mdd: x.mdd,
            up: x.up,
            low: x.low,
            mo_min: x.net,
            wk_min: x.wk_min,
            d_min: x.d_min,
            min_pass: x.min_pass,
            max_ref: x.max_ref,
            trades: x.trades,
            mo_ok: u16::from(crate::kern::month_ok(x.net)),
            mo_n: 1,
            wk_seen: u16::from(x.wk_seen),
            wk_pos: u16::from(x.wk_pos),
            wk_n: c.n_wp.get(i).copied().unwrap_or(0),
            d_seen: u16::from(x.d_seen),
            d_pos: u16::from(x.d_pos),
            d_n: c.n_sess.get(i).copied().unwrap_or(0),
            mo_min_at: u16::try_from(i).unwrap_or(NONE16),
            wk_min_at: x.wk_min_at,
            d_min_at: x.d_min_at,
        }
    }

    /// The window A then B (exact; associative, identity = the default).
    #[must_use]
    pub fn then(&self, b: &Self) -> Self {
        // MUTANT m_v32b_split_dd_cross: the drawdown across the boundary (A's end below its peak, then B's low) left out
        let cross = if cfg!(feature = "m_v32b_split_dd_cross") { 0 } else { self.up - self.net - b.low };
        let (mo_min, mo_min_at) = min_at((self.mo_min, self.mo_min_at), (b.mo_min, b.mo_min_at));
        let (wk_min, wk_min_at) = min_at((self.wk_min, self.wk_min_at), (b.wk_min, b.wk_min_at));
        let (d_min, d_min_at) = min_at((self.d_min, self.d_min_at), (b.d_min, b.d_min_at));
        Self {
            net: self.net + b.net,
            mdd: self.mdd.max(b.mdd).max(cross),
            up: self.up.max(self.net + b.up),
            low: self.low.min(self.net + b.low),
            mo_min,
            wk_min,
            d_min,
            min_pass: self.min_pass.min(b.min_pass),
            max_ref: self.max_ref.max(b.max_ref),
            trades: self.trades + b.trades,
            mo_ok: self.mo_ok + b.mo_ok,
            mo_n: self.mo_n + b.mo_n,
            wk_seen: self.wk_seen + b.wk_seen,
            wk_pos: self.wk_pos + b.wk_pos,
            wk_n: self.wk_n + b.wk_n,
            d_seen: self.d_seen + b.d_seen,
            d_pos: self.d_pos + b.d_pos,
            d_n: self.d_n + b.d_n,
            mo_min_at,
            wk_min_at,
            d_min_at,
        }
    }

    /// The addendum 19 minimum: every month >= Rs 2,00,000 net, every week piece and every session positive.
    #[must_use]
    pub const fn pass19(&self) -> bool {
        self.mo_n > 0 && self.mo_ok == self.mo_n && self.wk_pos == self.wk_n && self.d_pos == self.d_n
    }

    /// The addendum 15 money bar alone: every month >= Rs 2,00,000 net.
    #[must_use]
    pub const fn pass15(&self) -> bool {
        self.mo_n > 0 && self.mo_ok == self.mo_n
    }

    /// The addendum 15 / 19 order (bigger is better; the denominators are the window's, equal for every account).
    #[must_use]
    pub fn key(&self) -> (u16, u16, u16, i64, i64) {
        // MUTANT m_v32b_split_key_no_days: the positive sessions left out of the order
        let d = if cfg!(feature = "m_v32b_split_key_no_days") { 0 } else { self.d_pos };
        (self.mo_ok, self.wk_pos, d, -self.mdd, self.net)
    }

    /// The worst week piece (a week piece without a trade = Rs 0) and the worst session likewise.
    #[must_use]
    pub fn worst_week(&self) -> i64 {
        if self.wk_seen < self.wk_n { self.wk_min.min(0) } else if self.wk_min == i64::MAX { 0 } else { self.wk_min }
    }

    #[must_use]
    pub fn worst_day(&self) -> i64 {
        if self.d_seen < self.d_n { self.d_min.min(0) } else if self.d_min == i64::MAX { 0 } else { self.d_min }
    }
}

/// Badness of a split's test figures (Ordering::Greater = worse): failing the addendum 19 minimum, then the larger share of failing
/// months, week pieces, sessions (exact fractions), then the larger max drawdown, then the smaller net.
#[must_use]
pub fn cmp_bad(a: &Fig18, b: &Fig18) -> Ordering {
    let frac = |f: u16, n: u16, g: u16, m: u16| -> Ordering { (u64::from(f) * u64::from(m.max(1))).cmp(&(u64::from(g) * u64::from(n.max(1)))) };
    (!a.pass19())
        .cmp(&!b.pass19())
        .then(frac(a.mo_n - a.mo_ok, a.mo_n, b.mo_n - b.mo_ok, b.mo_n))
        .then(frac(a.wk_n.saturating_sub(a.wk_pos), a.wk_n, b.wk_n.saturating_sub(b.wk_pos), b.wk_n))
        .then(frac(a.d_n.saturating_sub(a.d_pos), a.d_n, b.d_n.saturating_sub(b.d_pos), b.d_n))
        .then(a.mdd.cmp(&b.mdd))
        .then(b.net.cmp(&a.net))
}

/// A disjoint sparse table over one account's months: any window in O(1) after O(M log M).
#[derive(Clone, Debug, Default)]
pub struct Dst {
    n: usize,
    base: Vec<Fig18>,
    lv: Vec<Fig18>,
}

impl Dst {
    /// Builds the table of `items` (one per month), reusing the buffers.
    pub fn build(&mut self, items: &[Fig18]) {
        let n = items.len();
        self.n = n;
        self.base.clear();
        self.base.extend_from_slice(items);
        let mut k = 0_usize;
        while (1_usize << k) < n {
            k += 1;
        }
        self.lv.clear();
        self.lv.resize(k * n, Fig18::default());
        for h in 0..k {
            let half = 1_usize << h;
            let block = half << 1;
            let row = h * n;
            let mut s = 0_usize;
            while s < n {
                let mid = s + half;
                if mid < n {
                    if let (Some(slot), Some(x)) = (self.lv.get_mut(row + mid - 1), items.get(mid - 1)) {
                        *slot = *x;
                    }
                    for i in (s..mid - 1).rev() {
                        let nxt = self.lv.get(row + i + 1).copied().unwrap_or_default();
                        if let (Some(x), Some(slot)) = (items.get(i), self.lv.get_mut(row + i)) {
                            *slot = x.then(&nxt);
                        }
                    }
                    if let (Some(slot), Some(x)) = (self.lv.get_mut(row + mid), items.get(mid)) {
                        *slot = *x;
                    }
                    for i in mid + 1..(s + block).min(n) {
                        let prv = self.lv.get(row + i - 1).copied().unwrap_or_default();
                        if let (Some(x), Some(slot)) = (items.get(i), self.lv.get_mut(row + i)) {
                            *slot = prv.then(x);
                        }
                    }
                }
                s += block;
            }
        }
    }

    /// The window a..=b (the identity when empty or out of range).
    #[must_use]
    pub fn query(&self, a: usize, b: usize) -> Fig18 {
        if a > b || b >= self.n {
            return Fig18::default();
        }
        if a == b {
            return self.base.get(a).copied().unwrap_or_default();
        }
        let h = (usize::BITS - 1 - (a ^ b).leading_zeros()) as usize;
        let row = h * self.n;
        match (self.lv.get(row + a), self.lv.get(row + b)) {
            (Some(x), Some(y)) => x.then(y),
            _ => Fig18::default(),
        }
    }
}

/// One split: its folds as ((learn a, learn b), (test a, test b)) in month indices.
#[derive(Clone, Debug)]
pub struct Split {
    pub id: String,
    /// 0 train/test, 1 walk-forward anchored, 2 walk-forward rolling.
    pub kind: u8,
    pub l: u8,
    pub s: u8,
    pub year: i64,
    pub folds: Vec<((usize, usize), (usize, usize))>,
}

impl Split {
    /// Whether the split ran (a learning month and a test month in the run).
    #[must_use]
    pub fn run(&self) -> bool {
        !self.folds.is_empty()
    }

    /// The test months of ONE combo under this split (its first test month .. the run's last month).
    #[must_use]
    pub fn combo_test(&self, m: usize) -> Option<(usize, usize)> {
        self.folds.first().map(|f| (f.1.0, m.saturating_sub(1)))
    }

    /// Whether a walk-forward split.
    #[must_use]
    pub const fn wf(&self) -> bool {
        self.kind != 0
    }
}

/// Every split of the run's months (TT 6, WF anchored 9, WF rolling 9), in print order.
#[must_use]
pub fn splits(c: &SCal) -> Vec<Split> {
    let m = c.m();
    let mut v = Vec::new();
    for y in TT_YEARS {
        let last = c.month_id.iter().rposition(|&id| id <= y * 12 + 11);
        let folds = match last {
            Some(cut) if cut + 1 < m => vec![((0, cut), (cut + 1, m - 1))],
            _ => Vec::new(),
        };
        v.push(Split { id: format!("TT-{y}"), kind: 0, l: 0, s: 0, year: y, folds });
    }
    for (kind, kn) in [(1_u8, "AN"), (2, "RO")] {
        for l in WF_L {
            for s in WF_S {
                let (lu, su) = (usize::from(l), usize::from(s));
                let mut folds = Vec::new();
                let mut k = 0_usize;
                loop {
                    let ts = lu + k * su;
                    if ts >= m {
                        break;
                    }
                    let learn = if kind == 1 { (0, ts - 1) } else { (k * su, ts - 1) };
                    folds.push((learn, (ts, (ts + su - 1).min(m - 1))));
                    k += 1;
                }
                v.push(Split { id: format!("WF-{kn}-L{l}-S{s}"), kind, l, s, year: 0, folds });
            }
        }
    }
    v
}

/// One combo's figures of a split, compact (kept in every account summary).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fig18c {
    pub mo_ok: u16,
    pub mo_n: u16,
    pub wk_pos: u16,
    pub wk_n: u16,
    pub d_pos: u16,
    pub d_n: u16,
    pub mdd: i64,
    pub net: i64,
    pub mo_min: i64,
}

impl Fig18c {
    fn of(f: &Fig18) -> Self {
        Self { mo_ok: f.mo_ok, mo_n: f.mo_n, wk_pos: f.wk_pos, wk_n: f.wk_n, d_pos: f.d_pos, d_n: f.d_n, mdd: f.mdd, net: f.net, mo_min: if f.mo_min == i64::MAX { 0 } else { f.mo_min } }
    }

    fn words(&self) -> String {
        format!("months {} / {}, week pieces {} / {}, sessions {} / {}, worst month Rs {}, max DD Rs {}, net Rs {}", self.mo_ok, self.mo_n, self.wk_pos, self.wk_n, self.d_pos, self.d_n, crate::cal::fmt_rs(self.mo_min), crate::cal::fmt_rs(self.mdd), crate::cal::fmt_rs(self.net))
    }
}

/// One combo's split statistics (every account; 0 splits run = none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpStat {
    pub run: u8,
    pub pass19: u8,
    pub pass15: u8,
    pub wf_run: u8,
    pub wf_pass19: u8,
    pub wf_pass15: u8,
    /// The worst and the median split (split index, its test figures).
    pub worst: (u8, Fig18c),
    pub median: (u8, Fig18c),
}

/// The addendum 18 verdict of one combo: every walk-forward split that ran passed AND at least 80% of all splits that ran.
#[must_use]
pub fn passed(p: &SpStat) -> bool {
    // MUTANT m_v32b_split_passed_any_wf: the walk-forward condition dropped (80% of all splits alone)
    let wf_ok = cfg!(feature = "m_v32b_split_passed_any_wf") || (p.wf_run > 0 && p.wf_pass19 == p.wf_run);
    p.run > 0 && wf_ok && u32::from(p.pass19) * 5 >= u32::from(p.run) * 4
}

/// The static tie key of an account (report3's `wtie`).
pub type Tie = (u32, u16, u32, u16, u8);

/// One fold's pick so far.
#[derive(Clone, Copy, Debug)]
pub struct Best {
    pub key: (u16, u16, u16, i64, i64),
    pub tie: Tie,
    pub sm: Sum,
    pub learn: Fig18,
    /// The fold's test months, and (fold 0) every month after the first learning window (way B).
    pub test: Fig18,
    pub after: Fig18,
    /// The slack test proves a fresh account at the learning window's first month decides as the continuous walk.
    pub fresh_ok: bool,
}

/// The split accumulators of one segment (per thread, merged).
#[derive(Clone, Debug, Default)]
pub struct Sel18 {
    pub splits: Vec<Split>,
    pub m: usize,
    pub best: Vec<Vec<Option<Best>>>,
    /// Accounts whose series were read / offered to the folds.
    pub read: u64,
    pub offered: u64,
    /// Nanoseconds spent on the split figures (MEASURED): every account in `ns_sum`; every 64th account (deterministic sample) in `ns`
    /// (p50 / p99 printed; bounded memory on a billion-account run).
    pub ns: Vec<u32>,
    pub ns_sum: u64,
    /// v3.2 round B: whether way A (re-chosen at every step) is computed (WAYA18: the month series, whole mode only); without it every
    /// split keeps its fold-0 pick only (way B, the method).
    pub way_a: bool,
    dst: Dst,
    items: Vec<Fig18>,
    pre: Vec<Fig18>,
    suf: Vec<Fig18>,
}

impl Sel18 {
    /// The accumulators of the run's splits (empty when SPLITS18 is off or no calendar).
    #[must_use]
    pub fn new() -> Self {
        match cal() {
            Some(c) if crate::sw::on(crate::sw::SPLITS18) => {
                let splits = splits(c);
                let way_a = crate::sw::on(crate::sw::WAYA18);
                let best = splits.iter().map(|s| vec![None; if way_a { s.folds.len() } else { s.folds.len().min(1) }]).collect();
                Self { splits, m: c.m(), best, way_a, ..Self::default() }
            }
            _ => Self::default(),
        }
    }

    /// Merges another thread's accumulators (the better pick per fold; deterministic: key, then the smaller tie key).
    pub fn merge(&mut self, o: Self) {
        if self.splits.is_empty() {
            *self = Self { dst: Dst::default(), items: Vec::new(), pre: Vec::new(), suf: Vec::new(), ..o };
            return;
        }
        self.read += o.read;
        self.offered += o.offered;
        self.ns.extend(o.ns);
        self.ns_sum += o.ns_sum;
        for (a, b) in self.best.iter_mut().zip(o.best) {
            for (x, y) in a.iter_mut().zip(b) {
                if let Some(y) = y
                    && x.as_ref().is_none_or(|x| better(&y.key, &y.tie, x))
                {
                    *x = Some(y);
                }
            }
        }
    }
}

fn pnum<T: std::str::FromStr>(f: &mut std::str::Split<'_, char>) -> Result<T, String> {
    f.next().ok_or("short split checkpoint row")?.parse::<T>().map_err(|_| "bad split checkpoint number".to_owned())
}

fn fig_line(f: &Fig18) -> String {
    format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", f.net, f.mdd, f.up, f.low, f.mo_min, f.wk_min, f.d_min, f.min_pass, f.max_ref, f.trades, f.mo_ok, f.mo_n, f.wk_seen, f.wk_pos, f.wk_n, f.d_seen, f.d_pos, f.d_n, f.mo_min_at, f.wk_min_at, f.d_min_at)
}

fn fig_of(f: &mut std::str::Split<'_, char>) -> Result<Fig18, String> {
    Ok(Fig18 { net: pnum(f)?, mdd: pnum(f)?, up: pnum(f)?, low: pnum(f)?, mo_min: pnum(f)?, wk_min: pnum(f)?, d_min: pnum(f)?, min_pass: pnum(f)?, max_ref: pnum(f)?, trades: pnum(f)?, mo_ok: pnum(f)?, mo_n: pnum(f)?, wk_seen: pnum(f)?, wk_pos: pnum(f)?, wk_n: pnum(f)?, d_seen: pnum(f)?, d_pos: pnum(f)?, d_n: pnum(f)?, mo_min_at: pnum(f)?, wk_min_at: pnum(f)?, d_min_at: pnum(f)? })
}

fn c_line(c: &Fig18c) -> String {
    format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", c.mo_ok, c.mo_n, c.wk_pos, c.wk_n, c.d_pos, c.d_n, c.mdd, c.net, c.mo_min)
}

fn c_of(f: &mut std::str::Split<'_, char>) -> Result<Fig18c, String> {
    Ok(Fig18c { mo_ok: pnum(f)?, mo_n: pnum(f)?, wk_pos: pnum(f)?, wk_n: pnum(f)?, d_pos: pnum(f)?, d_n: pnum(f)?, mdd: pnum(f)?, net: pnum(f)?, mo_min: pnum(f)? })
}

/// v3.2 round B (checkpoint, addendum 20.3): one combo's split statistics as tab-separated fields (26).
#[must_use]
pub fn sp_line(p: &SpStat) -> String {
    format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", p.run, p.pass19, p.pass15, p.wf_run, p.wf_pass19, p.wf_pass15, p.worst.0, c_line(&p.worst.1), p.median.0, c_line(&p.median.1))
}

/// The inverse of [`sp_line`] on the rest of a checkpoint row (no fields left = no statistics: a row written with SPLITS18 off).
///
/// # Errors
/// A malformed row.
pub fn sp_of(f: &mut std::str::Split<'_, char>) -> Result<SpStat, String> {
    let mut g = f.clone().peekable();
    if g.peek().is_none_or(|x| x.is_empty()) {
        return Ok(SpStat::default());
    }
    Ok(SpStat { run: pnum(f)?, pass19: pnum(f)?, pass15: pnum(f)?, wf_run: pnum(f)?, wf_pass19: pnum(f)?, wf_pass15: pnum(f)?, worst: (pnum(f)?, c_of(f)?), median: (pnum(f)?, c_of(f)?) })
}

/// One fold's pick as its checkpoint line (tag S18B; v3.4: shared by the checkpoint, the batch files and `ckpt-merge`: one format).
#[must_use]
pub fn s18b_line(si: usize, fi: usize, b: &Best) -> String {
    format!("S18B\t{si}\t{fi}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", b.key.0, b.key.1, b.key.2, b.key.3, b.key.4, b.tie.0, b.tie.1, b.tie.2, b.tie.3, b.tie.4, u8::from(b.fresh_ok), fig_line(&b.learn), fig_line(&b.test), fig_line(&b.after), crate::ckpt::sum_line(&b.sm))
}

/// The inverse of [`s18b_line`] on the text after the tag: (split, fold, pick).
///
/// # Errors
/// A malformed row.
pub fn s18b_parse(rest: &str) -> Result<(usize, usize, Best), String> {
    let mut f = rest.split('\t');
    let (si, fi): (usize, usize) = (pnum(&mut f)?, pnum(&mut f)?);
    let key = (pnum(&mut f)?, pnum(&mut f)?, pnum(&mut f)?, pnum(&mut f)?, pnum(&mut f)?);
    let tie = (pnum(&mut f)?, pnum(&mut f)?, pnum(&mut f)?, pnum(&mut f)?, pnum(&mut f)?);
    let fresh_ok = pnum::<u8>(&mut f)? == 1;
    let (learn, test, after) = (fig_of(&mut f)?, fig_of(&mut f)?, fig_of(&mut f)?);
    let sm = crate::ckpt::sum_of_line(&f.collect::<Vec<_>>().join("\t"))?;
    Ok((si, fi, Best { key, tie, sm, learn, test, after, fresh_ok }))
}

/// The checkpoint lines of split accumulators given by their parts (v3.4: `Sel18::ckpt_lines` and `ckpt-merge` write through this one
/// function, so a merged checkpoint has the engine's own lines).
#[must_use]
pub fn ckpt_lines_of(read: u64, offered: u64, ns_sum: u64, ns: &[u32], best: &[Vec<Option<Best>>]) -> Vec<String> {
    let mut v = vec![format!("S18\t{read}\t{offered}\t{ns_sum}"), format!("S18N\t{}", ns.iter().map(u32::to_string).collect::<Vec<_>>().join(","))];
    for (si, bv) in best.iter().enumerate() {
        for (fi, b) in bv.iter().enumerate() {
            let Some(b) = b else { continue };
            v.push(s18b_line(si, fi, b));
        }
    }
    v
}

/// v3.4 (the shard kit): whether pick `(k, t)` replaces `b` (the strict order: key, then the smaller tie): the ONE rule of the sweep,
/// the thread merge and `ckpt-merge`.
#[must_use]
pub fn better_pick(k: &(u16, u16, u16, i64, i64), t: &Tie, b: &Best) -> bool {
    better(k, t, b)
}

impl Sel18 {
    /// The checkpoint lines of the accumulated picks (tags S18 = counters, S18N = the timing samples, S18B = one fold's pick).
    #[must_use]
    pub fn ckpt_lines(&self) -> Vec<String> {
        ckpt_lines_of(self.read, self.offered, self.ns_sum, &self.ns, &self.best)
    }

    /// v3.4 (E1): a fresh accumulator for ONE batch that continues this one's sampling position (`read`), so the per-batch accumulator
    /// samples the same accounts the cumulative one would.
    #[must_use]
    pub fn batch_start(&self) -> Self {
        let mut b = Self::new();
        b.read = self.read;
        b
    }

    /// v3.4 (E1): absorbs a batch accumulator made by [`Sel18::batch_start`] (exact continuation: counters, samples, the better pick per
    /// fold), so the cumulative accumulator equals the one-pass sweep.
    pub fn absorb(&mut self, b: Self) {
        if self.splits.is_empty() {
            return;
        }
        self.read = b.read;
        self.offered += b.offered;
        self.ns.extend(b.ns.iter().copied());
        self.ns_sum += b.ns_sum;
        for (a, bb) in self.best.iter_mut().zip(b.best) {
            for (x, y) in a.iter_mut().zip(bb) {
                if let Some(y) = y
                    && x.as_ref().is_none_or(|x| better(&y.key, &y.tie, x))
                {
                    *x = Some(y);
                }
            }
        }
    }

    /// v3.4 (E1 + E3): the batch's own parts, with an EXHAUSTIVE destructuring (a field added later without a merge law fails to compile):
    /// (accounts read since `read0`, accounts offered, split ns sum, timing samples, folds kept per split, every pick).
    #[must_use]
    pub fn batch_parts(&self, read0: u64) -> (u64, u64, u64, Vec<u32>, Vec<usize>, Vec<(usize, usize, Best)>) {
        let Self { splits: _, m: _, best, read, offered, ns, ns_sum, way_a: _, dst: _, items: _, pre: _, suf: _ } = self;
        let shape: Vec<usize> = best.iter().map(Vec::len).collect();
        let picks: Vec<(usize, usize, Best)> = best.iter().enumerate().flat_map(|(si, bv)| bv.iter().enumerate().filter_map(move |(fi, b)| b.map(|b| (si, fi, b)))).collect();
        (read.saturating_sub(read0), *offered, *ns_sum, ns.clone(), shape, picks)
    }

    /// Restores the picks from [`Sel18::ckpt_lines`] into a fresh accumulator of the same run.
    ///
    /// # Errors
    /// A malformed line or a fold the run does not have.
    pub fn ckpt_restore(&mut self, lines: &[String]) -> Result<(), String> {
        for l in lines {
            let (tag, rest) = l.split_once('\t').unwrap_or((l, ""));
            let mut f = rest.split('\t');
            match tag {
                "S18" => {
                    self.read = pnum(&mut f)?;
                    self.offered = pnum(&mut f)?;
                    self.ns_sum = pnum(&mut f)?;
                }
                "S18N" => self.ns = rest.split(',').filter(|x| !x.is_empty()).map(|x| x.parse::<u32>().map_err(|_| "S18N".to_owned())).collect::<Result<Vec<_>, _>>()?,
                "S18B" => {
                    let _ = &mut f;
                    let (si, fi, best) = s18b_parse(rest)?;
                    let slot = self.best.get_mut(si).and_then(|v| v.get_mut(fi)).ok_or_else(|| format!("split checkpoint: fold {si}/{fi} not in this run"))?;
                    // MUTANT m_v32rb_ckpt_picks_dropped: a resumed run loses the finished batches' split picks
                    if cfg!(feature = "m_v32rb_ckpt_picks_dropped") {
                        continue;
                    }
                    *slot = Some(best);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn better(k: &(u16, u16, u16, i64, i64), t: &Tie, b: &Best) -> bool {
    *k > b.key || (*k == b.key && *t < b.tie)
}

/// The fresh-start slack test of one window starting at month `a` and read to `b` (wf.rs `fresh_equal` on month pieces).
fn fresh_ok(d: &Dst, a: usize, b: usize) -> bool {
    // MUTANT m_v32b_split_fresh_trusted: every later-starting window trusted as a fresh account's
    if a == 0 || cfg!(feature = "m_v32b_split_fresh_trusted") {
        return true;
    }
    let r0 = d.query(0, a - 1).net;
    let f = d.query(a, b);
    !((r0 > 0 && f.min_pass < r0) || (r0 < 0 && f.max_ref >= r0))
}

/// One account at its finish: its split statistics into `sm.sp`, and (SEARCH settings with a trade: `offer`) its learning figures offered
/// to every fold of every split. O(M log M) once, then O(1) per window.
pub fn account(ser: Option<&Ser18>, sm: &mut Sum, sel: &mut Sel18, offer: bool, tie: Tie) {
    let Some(ser) = ser else { return };
    if sel.splits.is_empty() {
        return;
    }
    let Some(c) = cal() else { return };
    let t0 = std::time::Instant::now();
    // the segments' figures, then every prefix (a learning window from the run's first month) and every suffix (a test window to the run's
    // last month) by exact composition: O(segments) once per account, then O(1) per window
    let segs = ser.s.figs();
    let ns = segs.len();
    if ns == 0 {
        return;
    }
    sel.pre.clear();
    sel.pre.push(Fig18::default());
    for (j, f) in segs.iter().enumerate() {
        let p = sel.pre.get(j).copied().unwrap_or_default().then(f);
        sel.pre.push(p);
    }
    sel.suf.clear();
    sel.suf.resize(ns + 1, Fig18::default());
    for j in (0..ns).rev() {
        let s = segs.get(j).copied().unwrap_or_default().then(&sel.suf.get(j + 1).copied().unwrap_or_default());
        if let Some(x) = sel.suf.get_mut(j) {
            *x = s;
        }
    }
    let m = c.m();
    sel.read += 1;
    let seg_at = |a: usize| usize::from(c.seg_of_mo.get(a).copied().unwrap_or(0));
    // the combo's own test months under every split that ran (its first test month .. the run's last month)
    let mut res: Vec<(usize, Fig18)> = Vec::new();
    let mut st = SpStat::default();
    for (si, sp) in sel.splits.iter().enumerate() {
        let Some((a, _)) = sp.combo_test(m) else { continue };
        let f = sel.suf.get(seg_at(a)).copied().unwrap_or_default();
        st.run += 1;
        st.pass19 += u8::from(f.pass19());
        st.pass15 += u8::from(f.pass15());
        if sp.wf() {
            st.wf_run += 1;
            st.wf_pass19 += u8::from(f.pass19());
            st.wf_pass15 += u8::from(f.pass15());
        }
        res.push((si, f));
    }
    if !res.is_empty() {
        res.sort_by(|x, y| cmp_bad(&x.1, &y.1).then(x.0.cmp(&y.0)));
        if let Some(&(wi, wf)) = res.last() {
            st.worst = (u8::try_from(wi).unwrap_or(u8::MAX), Fig18c::of(&wf));
        }
        if let Some(&(mi, mf)) = res.get((res.len() - 1) / 2) {
            st.median = (u8::try_from(mi).unwrap_or(u8::MAX), Fig18c::of(&mf));
        }
    }
    sm.sp = st;
    if offer && sm.trades > 0 {
        sel.offered += 1;
        // way A (WAYA18, whole mode): every fold's own windows from the month series
        if sel.way_a
            && let Some(ms) = ser.m.as_ref()
        {
            let months = ms.sealed();
            sel.items.clear();
            sel.items.extend(months.iter().enumerate().map(|(i, x)| Fig18::of(x, i, c)));
            let items = std::mem::take(&mut sel.items);
            sel.dst.build(&items);
            sel.items = items;
        }
        for (si, sp) in sel.splits.iter().enumerate() {
            let Some(bv) = sel.best.get_mut(si) else { continue };
            for (fi, &((la, lb), (ta, tb))) in sp.folds.iter().enumerate() {
                if fi > 0 && !sel.way_a {
                    break;
                }
                let (lf, after, test, fresh) = if fi == 0 {
                    // way B: the first learning window (from the run's first month: a fresh account is the continuous walk) and every month
                    // after it, both runs of whole segments
                    let j = seg_at(ta);
                    // MUTANT m_v32b_split_lookahead: the learning window reads one segment beyond its end
                    let jl = if cfg!(feature = "m_v32b_split_lookahead") { (j + 1).min(ns) } else { j };
                    let test = if sel.way_a { sel.dst.query(ta, tb) } else { Fig18::default() };
                    (sel.pre.get(jl).copied().unwrap_or_default(), sel.suf.get(j).copied().unwrap_or_default(), test, true)
                } else {
                    let lb2 = if cfg!(feature = "m_v32b_split_lookahead") { (lb + 1).min(m - 1) } else { lb };
                    (sel.dst.query(la, lb2), Fig18::default(), sel.dst.query(ta, tb), fresh_ok(&sel.dst, la, tb))
                };
                if lf.trades == 0 {
                    continue;
                }
                let key = lf.key();
                let Some(slot) = bv.get_mut(fi) else { continue };
                if slot.as_ref().is_none_or(|b| better(&key, &tie, b)) {
                    *slot = Some(Best { key, tie, sm: *sm, learn: lf, test, after, fresh_ok: fresh });
                }
            }
        }
    }
    let el = u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX);
    sel.ns_sum = sel.ns_sum.saturating_add(el);
    if sel.read % 64 == 1 {
        sel.ns.push(u32::try_from(el).unwrap_or(u32::MAX));
    }
}

/// v3.2-B's algorithm (job v32-par), kept as the REFERENCE the segment figures are tested against: everything from the month series and
/// its disjoint sparse table (O(M log M) per account). Not called by the engine.
#[cfg(test)]
pub fn account_months(ms: Option<&MSeries>, sm: &mut Sum, sel: &mut Sel18, offer: bool, tie: Tie) {
    let Some(ms) = ms else { return };
    if !ms.on() || sel.splits.is_empty() {
        return;
    }
    let Some(c) = ms.cal else { return };
    let t0 = std::time::Instant::now();
    let months = ms.sealed();
    sel.items.clear();
    sel.items.extend(months.iter().enumerate().map(|(i, x)| Fig18::of(x, i, c)));
    let items = std::mem::take(&mut sel.items);
    sel.dst.build(&items);
    sel.items = items;
    let m = months.len();
    sel.read += 1;
    // the combo's own test months under every split that ran
    let mut res: Vec<(usize, Fig18)> = Vec::new();
    let mut st = SpStat::default();
    for (si, sp) in sel.splits.iter().enumerate() {
        let Some((a, b)) = sp.combo_test(m) else { continue };
        let f = sel.dst.query(a, b);
        st.run += 1;
        st.pass19 += u8::from(f.pass19());
        st.pass15 += u8::from(f.pass15());
        if sp.wf() {
            st.wf_run += 1;
            st.wf_pass19 += u8::from(f.pass19());
            st.wf_pass15 += u8::from(f.pass15());
        }
        res.push((si, f));
    }
    if !res.is_empty() {
        res.sort_by(|x, y| cmp_bad(&x.1, &y.1).then(x.0.cmp(&y.0)));
        if let Some(&(wi, wf)) = res.last() {
            st.worst = (u8::try_from(wi).unwrap_or(u8::MAX), Fig18c::of(&wf));
        }
        if let Some(&(mi, mf)) = res.get((res.len() - 1) / 2) {
            st.median = (u8::try_from(mi).unwrap_or(u8::MAX), Fig18c::of(&mf));
        }
    }
    sm.sp = st;
    if offer && sm.trades > 0 {
        sel.offered += 1;
        for (si, sp) in sel.splits.iter().enumerate() {
            let Some(bv) = sel.best.get_mut(si) else { continue };
            for (fi, &((la, lb), (ta, tb))) in sp.folds.iter().enumerate() {
                // MUTANT m_v32b_split_lookahead: the learning window reads one month beyond its end
                let lb = if cfg!(feature = "m_v32b_split_lookahead") { (lb + 1).min(m - 1) } else { lb };
                let lf = sel.dst.query(la, lb);
                if lf.trades == 0 {
                    continue;
                }
                let key = lf.key();
                let Some(slot) = bv.get_mut(fi) else { continue };
                if slot.as_ref().is_none_or(|b| better(&key, &tie, b)) {
                    let after = if fi == 0 { sel.dst.query(ta, m - 1) } else { Fig18::default() };
                    *slot = Some(Best { key, tie, sm: *sm, learn: lf, test: sel.dst.query(ta, tb), after, fresh_ok: fresh_ok(&sel.dst, la, if fi == 0 { m - 1 } else { tb }) });
                }
            }
        }
    }
    let el = u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX);
    sel.ns_sum = sel.ns_sum.saturating_add(el);
    if sel.read % 64 == 1 {
        sel.ns.push(u32::try_from(el).unwrap_or(u32::MAX));
    }
}

/// One window's figures in words (addendum 19: positive months / weeks / sessions of all, the worst month, week piece and session with
/// their dates, the max drawdown, the net); one table cell.
fn fig_words(f: &Fig18, c: &SCal) -> String {
    let wk = f.worst_week();
    let wk_at = if f.wk_min == i64::MAX || wk < f.wk_min { "a week piece without a trade".to_owned() } else { c.wp_name(f.wk_min_at) };
    let d = f.worst_day();
    let d_at = if f.d_min == i64::MAX || d < f.d_min { "a session without a trade".to_owned() } else { c.day_name(f.d_min_at) };
    format!(
        "months >= Rs 2 L {} / {}; positive week pieces {} / {}; positive sessions {} / {}; worst month Rs {} ({}); worst week piece Rs {} ({wk_at}); worst session Rs {} ({d_at}); max DD Rs {}; net Rs {}",
        f.mo_ok,
        f.mo_n,
        f.wk_pos,
        f.wk_n,
        f.d_pos,
        f.d_n,
        crate::cal::fmt_rs(if f.mo_min == i64::MAX { 0 } else { f.mo_min }),
        c.month_name(usize::from(f.mo_min_at)),
        crate::cal::fmt_rs(wk),
        crate::cal::fmt_rs(d),
        crate::cal::fmt_rs(f.mdd),
        crate::cal::fmt_rs(f.net)
    )
}

fn verdict(f: &Fig18) -> &'static str {
    if f.pass19() { "PASSED" } else { "NOT PASSED" }
}

fn quant(v: &[u32], p: usize) -> u32 {
    let mut x = v.to_vec();
    x.sort_unstable();
    x.get(x.len().saturating_sub(1) * p / 100).copied().unwrap_or(0)
}

/// The RESULTS section of one segment: every split's way B pick and verdict, the way A comparison, the slack-test counts.
#[must_use]
pub fn section(seg_name: &str, sel: &Sel18, words: &dyn Fn(&Sum) -> String) -> String {
    let mut s = String::new();
    let Some(c) = cal() else { return s };
    let _ = writeln!(s, "### {seg_name}: Addendum 18 split permutations (from the ONE walk; months {} .. {}, {} months, {} sessions)", c.month_name(0), c.month_name(sel.m.saturating_sub(1)), sel.m, c.days.len());
    let _ = writeln!(s);
    let ran: Vec<&Split> = sel.splits.iter().filter(|x| x.run()).collect();
    let _ = writeln!(s, "Splits run: {} of {} (TT {} of 6, walk-forward {} of 18); a split needs a learning month and a test month in this run. Accounts read {}, offered to the folds (SEARCH settings with a trade) {}; split figures per account MEASURED p50 {} ns / p99 {} ns (every 64th account sampled, {} samples), {:.1} s of thread time over every account (one O(M log M) table, then O(1) per window; no account walked twice).", ran.len(), sel.splits.len(), ran.iter().filter(|x| !x.wf()).count(), ran.iter().filter(|x| x.wf()).count(), sel.read, sel.offered, quant(&sel.ns, 50), quant(&sel.ns, 99), sel.ns.len(), sel.ns_sum as f64 / 1e9);
    let _ = writeln!(s);
    let _ = writeln!(s, "Way B (THE method: the combo chosen on the first learning window by the addendum 15 / 19 order on its learning months only, then unchanged on every later month); PASSED = every test month >= Rs 2,00,000 net, every week piece and every session positive (addendum 19; the addendum 15 money bar alone beside). Columns of figures: months >= Rs 2 L / months | positive week pieces / week pieces | positive sessions / sessions | worst month Rs | max DD Rs | net Rs.");
    let _ = writeln!(s);
    let _ = writeln!(s, "| split | learn (first fold) | test (way B) | folds | way B pick (chosen on learning months only) | learning figures | test figures | way B verdict (addendum 19) | money bar (addendum 15) | way A: re-chosen each step (LABELLED COMPARISON): folds passed / folds, distinct picks, test net Rs | fresh-start slack test: pick-windows not proven equal |");
    let _ = writeln!(s, "|---|---|---|---|---|---|---|---|---|---|---|");
    let (mut b_pass, mut b_run, mut wf_b_pass, mut wf_b_run) = (0, 0, 0, 0);
    for (si, sp) in sel.splits.iter().enumerate() {
        if !sp.run() {
            let _ = writeln!(s, "| {} | - | - | 0 | NOT RUN (no learning month or no test month in this run) | - | - | NOT RUN | - | - | - |", sp.id);
            continue;
        }
        let bv = sel.best.get(si).map_or(&[][..], Vec::as_slice);
        let ((la, lb), (ta, _)) = sp.folds.first().copied().unwrap_or(((0, 0), (0, 0)));
        let learn = format!("{}..{}", c.month_name(la), c.month_name(lb));
        let test = format!("{}..{}", c.month_name(ta), c.month_name(c.m().saturating_sub(1)));
        let (pick, lw, tw, v19, v15) = match bv.first().copied().flatten() {
            Some(b) => {
                b_run += 1;
                if sp.wf() {
                    wf_b_run += 1;
                }
                if b.after.pass19() {
                    b_pass += 1;
                    if sp.wf() {
                        wf_b_pass += 1;
                    }
                }
                (words(&b.sm), fig_words(&b.learn, c), fig_words(&b.after, c), verdict(&b.after), if b.after.pass15() { "passes" } else { "fails" })
            }
            None => ("no account with a learning trade".to_owned(), "-".to_owned(), "-".to_owned(), "NOT RUN", "-"),
        };
        let mut way_a = String::from("-");
        let mut unproven = 0;
        if sp.wf() && !sel.way_a {
            way_a = "not computed (way A re-chooses at every fold: it needs every month of every account, `--splits18-way-a on`, whole mode only)".to_owned();
        } else if sp.wf() {
            let mut fp = 0;
            let mut cat = Fig18::default();
            let mut ids: Vec<(u32, u16, u8)> = Vec::new();
            for b in bv.iter().flatten() {
                fp += usize::from(b.test.pass19());
                cat = cat.then(&b.test);
                let id = (b.sm.list, b.sm.x, b.sm.n);
                if !ids.contains(&id) {
                    ids.push(id);
                }
                unproven += usize::from(!b.fresh_ok);
            }
            way_a = format!("{fp} / {}, {} distinct, Rs {}", sp.folds.len(), ids.len(), crate::cal::fmt_rs(cat.net));
        } else {
            unproven = bv.iter().flatten().filter(|b| !b.fresh_ok).count();
        }
        let _ = writeln!(s, "| {} | {learn} | {test} | {} | {pick} | {lw} | {tw} | {v19} | {v15} | {way_a} | {unproven} |", sp.id, sp.folds.len());
    }
    let _ = writeln!(s);
    let _ = writeln!(s, "Way B over the splits that ran: {b_pass} of {b_run} PASSED (walk-forward {wf_b_pass} of {wf_b_run}). For ONE combo anchored and rolling coincide at the first fold and the step never changes its test months: the walk-forward way B rows differ by the learning length (6 / 12 / 24 months) only. Way A changes the combo at each step (never the headline){}. Split figures: {} segments of whole months between the split boundaries ({}), one record per segment per account (v3.2 round B; v3.2-B kept one per month); every way B window and every per-combo test window is a run of whole segments. HINDSIGHT: none (every pick reads its learning months only); proof step 1 (own engine).", if sel.way_a { "" } else { "; way A is not computed in this run" }, c.nseg(), (0..c.nseg()).map(|g| format!("{}..{}", c.month_name(usize::from(c.seg_lo.get(g).copied().unwrap_or(0))), c.month_name(usize::from(c.seg_hi.get(g).copied().unwrap_or(0))))).collect::<Vec<_>>().join(", "));
    let _ = writeln!(s);
    s
}

/// The per-combo table: every kept combo given (view label, account), with its split statistics and the addendum 18 verdict.
#[must_use]
pub fn combo_table(seg_name: &str, rows: &[(String, Sum)], words: &dyn Fn(&Sum) -> String) -> String {
    let mut s = String::new();
    let Some(c) = cal() else { return s };
    let splits = splits(c);
    let name = |i: u8| splits.get(usize::from(i)).map_or_else(|| "-".to_owned(), |x| x.id.clone());
    let _ = writeln!(s, "#### {seg_name}: Addendum 18 per kept combo (its own test months under every split that ran; the full-period net is HINDSIGHT)");
    let _ = writeln!(s);
    let _ = writeln!(s, "| view | combo | full-period net Rs (HINDSIGHT) | splits passed / run (addendum 19) | money bar passed / run (addendum 15) | walk-forward passed / run | worst split | median split | addendum 18 verdict |");
    let _ = writeln!(s, "|---|---|---|---|---|---|---|---|---|");
    for (lab, sm) in rows {
        let p = &sm.sp;
        if p.run == 0 {
            let _ = writeln!(s, "| {lab} | {} | {} | 0 / 0 | 0 / 0 | 0 / 0 | - | - | NOT RUN (no split ran) |", words(sm), crate::cal::fmt_rs(sm.net));
            continue;
        }
        let _ = writeln!(s, "| {lab} | {} | {} | {} / {} | {} / {} | {} / {} | {}: {} | {}: {} | {} |", words(sm), crate::cal::fmt_rs(sm.net), p.pass19, p.run, p.pass15, p.run, p.wf_pass19, p.wf_run, name(p.worst.0), p.worst.1.words(), name(p.median.0), p.median.1.words(), if passed(p) { "PASSED" } else { "NOT PASSED" });
    }
    let _ = writeln!(s);
    s
}

/// The TSV of every kept combo's split statistics (header + rows).
#[must_use]
pub fn tsv(rows: &[(String, String, Sum)], words: &dyn Fn(&str, &Sum) -> String) -> String {
    let mut t = String::from("segment\tview\tcombo\tfull_period_net_rs_hindsight\tsplits_run\tpassed_a19\tpassed_a15\twf_run\twf_passed_a19\twf_passed_a15\tworst_split\tworst_figures\tmedian_split\tmedian_figures\taddendum18_verdict\n");
    let Some(c) = cal() else { return t };
    let splits = splits(c);
    let name = |i: u8| splits.get(usize::from(i)).map_or_else(|| "-".to_owned(), |x| x.id.clone());
    for (seg, lab, sm) in rows {
        let p = &sm.sp;
        let _ = writeln!(t, "{seg}\t{lab}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", words(seg, sm), crate::cal::fmt_rs(sm.net), p.run, p.pass19, p.pass15, p.wf_run, p.wf_pass19, p.wf_pass15, if p.run == 0 { "-".to_owned() } else { name(p.worst.0) }, if p.run == 0 { "-".to_owned() } else { p.worst.1.words() }, if p.run == 0 { "-".to_owned() } else { name(p.median.0) }, if p.run == 0 { "-".to_owned() } else { p.median.1.words() }, if p.run == 0 { "NOT RUN" } else if passed(p) { "PASSED" } else { "NOT PASSED" });
    }
    t
}

/// The TSV of every split's pick: one WAY B row per (segment, split) (the pick of the first learning window, its test = every later month
/// to the run's last), then (WAYA18 only) one WAY A row per fold (re-chosen on that fold's learning months, its own test months).
#[must_use]
pub fn picks_tsv(segs: &[(&str, &Sel18)], words: &dyn Fn(&str, &Sum) -> String) -> String {
    let mut t = String::from("segment\tsplit\tkind\tlearn_months_L\tstep_S\tcut_year\tway\tfold\tlearn_from\tlearn_to\ttest_from\ttest_to\tpick\tlearn_months_ok\tlearn_months\tlearn_weekpieces_pos\tlearn_weekpieces\tlearn_sessions_pos\tlearn_sessions\tlearn_mdd_rs\tlearn_net_rs\ttest_months_ok\ttest_months\ttest_weekpieces_pos\ttest_weekpieces\ttest_sessions_pos\ttest_sessions\ttest_mdd_rs\ttest_net_rs\ttest_verdict_a19\tfresh_start_proven\n");
    let Some(c) = cal() else { return t };
    for (seg, sel) in segs {
        for (si, sp) in sel.splits.iter().enumerate() {
            for (fi, &((la, lb), (ta, tb))) in sp.folds.iter().enumerate() {
                let b = sel.best.get(si).and_then(|v| v.get(fi)).copied().flatten();
                let mut rows: Vec<(&str, usize, Fig18)> = Vec::new();
                if fi == 0 {
                    rows.push(("B", c.m().saturating_sub(1), b.map_or_else(Fig18::default, |b| b.after)));
                }
                if sel.way_a && sp.wf() {
                    rows.push(("A", tb, b.map_or_else(Fig18::default, |b| b.test)));
                }
                for (way, tend, te) in rows {
                    let (pick, l) = match b {
                        Some(b) => (words(seg, &b.sm), b.learn),
                        None => ("-".to_owned(), Fig18::default()),
                    };
                    let _ = writeln!(t, "{seg}\t{}\t{}\t{}\t{}\t{}\t{way}\t{fi}\t{}\t{}\t{}\t{}\t{pick}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", sp.id, ["TT", "WF-AN", "WF-RO"].get(usize::from(sp.kind)).copied().unwrap_or("?"), sp.l, sp.s, sp.year, c.month_name(la), c.month_name(lb), c.month_name(ta), c.month_name(tend), l.mo_ok, l.mo_n, l.wk_pos, l.wk_n, l.d_pos, l.d_n, crate::cal::fmt_rs(l.mdd), crate::cal::fmt_rs(l.net), te.mo_ok, te.mo_n, te.wk_pos, te.wk_n, te.d_pos, te.d_n, crate::cal::fmt_rs(te.mdd), crate::cal::fmt_rs(te.net), if b.is_some() { verdict(&te) } else { "NOT RUN" }, b.is_some_and(|b| b.fresh_ok));
                }
            }
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny deterministic generator (tests only).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn range(&mut self, lo: i64, hi: i64) -> i64 {
            lo + i64::try_from(self.next() % u64::try_from(hi - lo + 1).unwrap_or(1)).unwrap_or(0)
        }
    }

    /// The sessions of `months` months from 2018-10 (weekdays only; holidays ignored: a synthetic calendar).
    fn days_of(months: usize) -> Vec<i64> {
        let d0 = crate::cal::parse_day("2018-10-01").unwrap_or(0);
        let m0 = crate::v32::month_id(d0);
        (d0..d0 + 31 * i64::try_from(months + 1).unwrap_or(1)).filter(|&d| crate::rules::weekday(d) < 5 && usize::try_from(crate::v32::month_id(d) - m0).unwrap_or(usize::MAX) < months).collect()
    }

    fn on(months: usize) -> &'static SCal {
        crate::sw::set(crate::sw::SPLITS18, true);
        // round B: both series kept (the segments = the engine's figures, the months = v3.2-B's reference and way A)
        crate::sw::set(crate::sw::WAYA18, true);
        set_cal(SCal::from_days(&days_of(months)).unwrap_or_default());
        cal().unwrap_or_else(|| panic!("calendar"))
    }

    /// A random close sequence over the run days: (day index, second, net) in time order; some days without closes.
    fn closes(r: &mut Rng, ndays: usize) -> Vec<(u16, u32, i64)> {
        let mut v = Vec::new();
        for d in 0..ndays {
            if r.range(0, 4) == 0 {
                continue;
            }
            let n = r.range(1, 5);
            let mut t = 36_000_u32;
            for _ in 0..n {
                t += u32::try_from(r.range(0, 900)).unwrap_or(0);
                v.push((u16::try_from(d).unwrap_or(0), t, r.range(-9_000_000, 12_000_000)));
            }
        }
        v
    }

    /// The production accumulation of a close sequence (booking-order levels: dd per close, as the ungrouped reading): the segment series and
    /// the month series fed by the same hook calls.
    fn series(cl: &[(u16, u32, i64)]) -> Ser18 {
        let c = cal().unwrap_or_else(|| panic!("cal"));
        let mut ms = Ser18 { s: SSeries::new(c), m: Some(MSeries::new()) };
        let mut real = 0_i64;
        for &(d, t, n) in cl {
            real += n;
            ms.close(u32::from(d) * 86_400 + t, real, n, true);
        }
        ms
    }

    /// The same under the GROUPED reading (v3.1 item 5): closes of one second booked without the level update, then one group end.
    fn series_grouped(cl: &[(u16, u32, i64)]) -> Ser18 {
        let c = cal().unwrap_or_else(|| panic!("cal"));
        let mut ms = Ser18 { s: SSeries::new(c), m: Some(MSeries::new()) };
        let mut real = 0_i64;
        for (i, &(d, t, n)) in cl.iter().enumerate() {
            real += n;
            let key = u32::from(d) * 86_400 + t;
            ms.close(key, real, n, false);
            let next = cl.get(i + 1).map(|x| u32::from(x.0) * 86_400 + x.1);
            if next != Some(key) {
                ms.group_end(key, real);
            }
        }
        ms
    }

    /// Closes with WHOLE MONTHS skipped (about one in three) and same-second groups: the segment gaps and the grouped levels are exercised.
    fn closes_sparse(r: &mut Rng, c: &SCal) -> Vec<(u16, u32, i64)> {
        let skip: Vec<bool> = (0..c.m()).map(|_| r.range(0, 2) == 0).collect();
        let mut v = Vec::new();
        for d in 0..c.days.len() {
            if skip[usize::from(c.mo_of_day[d])] || r.range(0, 3) == 0 {
                continue;
            }
            let n = r.range(1, 4);
            let mut t = 36_000_u32;
            for _ in 0..n {
                // a third of the closes share the previous close's second
                if r.range(0, 2) != 0 {
                    t += u32::try_from(r.range(1, 900)).unwrap_or(1);
                }
                v.push((u16::try_from(d).unwrap_or(0), t, r.range(-9_000_000, 12_000_000)));
            }
        }
        v
    }

    /// The segments a..=b composed.
    fn seg_window(f: &[Fig18], a: usize, b: usize) -> Fig18 {
        f.get(a..=b).map_or_else(Fig18::default, |x| x.iter().fold(Fig18::default(), |acc, y| acc.then(y)))
    }

    /// A Fig18 without the fresh-start bounds (the segments keep none: their windows start at the run's first month or read the
    /// continuous walk).
    fn no_slack(f: Fig18) -> Fig18 {
        Fig18 { min_pass: 0, max_ref: 0, ..f }
    }

    /// Brute force from the raw closes: the window a..=b of months (net, max DD from the window's start, trades, months >= 2 L,
    /// positive sessions, positive week pieces).
    fn brute(c: &SCal, cl: &[(u16, u32, i64)], a: usize, b: usize) -> (i64, i64, u32, u16, u16, u16) {
        let inm = |d: u16| { let m = usize::from(c.mo_of_day[usize::from(d)]); m >= a && m <= b };
        let (mut lvl, mut peak, mut dd, mut tr) = (0_i64, 0_i64, 0_i64, 0_u32);
        let mut mo = vec![0_i64; c.m()];
        let mut day = std::collections::BTreeMap::<u16, i64>::new();
        let mut wp = std::collections::BTreeMap::<u16, i64>::new();
        for &(d, _, n) in cl.iter().filter(|x| inm(x.0)) {
            lvl += n;
            peak = peak.max(lvl);
            dd = dd.max(peak - lvl);
            tr += 1;
            mo[usize::from(c.mo_of_day[usize::from(d)])] += n;
            *day.entry(d).or_insert(0) += n;
            *wp.entry(c.wp_of_day[usize::from(d)]).or_insert(0) += n;
        }
        let mok = (a..=b).filter(|&m| crate::kern::month_ok(mo[m])).count();
        (lvl, dd, tr, u16::try_from(mok).unwrap_or(0), u16::try_from(day.values().filter(|&&x| x > 0).count()).unwrap_or(0), u16::try_from(wp.values().filter(|&&x| x > 0).count()).unwrap_or(0))
    }

    fn table(ser: &Ser18) -> Dst {
        let c = cal().unwrap_or_else(|| panic!("cal"));
        let ms = ser.m.as_ref().unwrap_or_else(|| panic!("the month series is kept in the tests"));
        let items: Vec<Fig18> = ms.sealed().iter().enumerate().map(|(i, x)| Fig18::of(x, i, c)).collect();
        let mut d = Dst::default();
        d.build(&items);
        d
    }

    /// v3.2 ROUND B, THE NAMED EXACTNESS TEST OF THE SEGMENTS: on random accounts (whole months without a close, same-second groups,
    /// booking order AND the grouped reading) every run of whole segments composed from the segment series equals v3.2-B's month series
    /// window over the same months (every field: net, max DD, levels, the worst month / week piece / session with where, the counts; the
    /// fresh-start bounds excepted, which the segments do not keep).
    #[test]
    fn v32rb_seg_equals_month_windows() {
        let mut n = 0;
        let mut gaps = 0;
        for months in [26_usize, 97] {
            let c = on(months);
            assert!(c.nseg() >= 4 && c.nseg() <= MAXSEG, "segments {}", c.nseg());
            let mut r = Rng(0x5e9_0001 + u64::try_from(months).unwrap_or(0));
            for k in 0..30 {
                let cl = closes_sparse(&mut r, c);
                let ms = if k % 2 == 0 { series(&cl) } else { series_grouped(&cl) };
                let d = table(&ms);
                let f = ms.s.figs();
                assert_eq!(f.len(), c.nseg());
                for a in 0..f.len() {
                    for b in a..f.len() {
                        let (lo, hi) = (usize::from(c.seg_lo[a]), usize::from(c.seg_hi[b]));
                        let want = no_slack(d.query(lo, hi));
                        let got = no_slack(seg_window(&f, a, b));
                        assert_eq!(got, want, "{months} months, account {k}, segments {a}..{b} = months {lo}..{hi}");
                        if k % 2 == 0 {
                            // and independently of the month code: a brute force from the raw closes (booking order)
                            let (net, dd, tr, mok, dpos, wpos) = brute(c, &cl, lo, hi);
                            assert_eq!((got.net, got.mdd, got.trades, got.mo_ok, got.d_pos, got.wk_pos), (net, dd, tr, mok, dpos, wpos), "brute force, segments {a}..{b}");
                        }
                        gaps += usize::from(want.mo_min == 0 && want.mo_min_at != NONE16 && got.mo_ok < got.mo_n);
                        n += 1;
                    }
                }
            }
        }
        assert!(n > 1_000 && gaps > 50, "windows checked {n}, with a month without a close {gaps}");
    }

    /// v3.2 ROUND B: the engine's per-combo statistics and every split's way B pick (segments) equal v3.2-B's month-series algorithm, and
    /// way A (every fold, from the months) is unchanged, on random accounts of 97 months (every boundary present).
    #[test]
    fn v32rb_stats_and_picks_equal_month_path() {
        let c = on(97);
        let mut r = Rng(0x0b0b_5eed);
        let accts: Vec<(Ser18, Sum)> = (0..40).map(|i| (if i % 3 == 0 { series_grouped(&closes_sparse(&mut r, c)) } else { series(&closes(&mut r, c.days.len())) }, Sum { trades: 1, ..Sum::default() })).collect();
        let mut s_new = Sel18::new();
        let mut s_old = Sel18::new();
        assert!(s_new.way_a);
        for (i, (ms, sm)) in accts.iter().enumerate() {
            let tie = (0, 0, u32::try_from(i).unwrap_or(0), 0, 0);
            let (mut a, mut b) = (*sm, *sm);
            account(Some(ms), &mut a, &mut s_new, true, tie);
            account_months(ms.m.as_ref(), &mut b, &mut s_old, true, tie);
            assert_eq!(a.sp, b.sp, "account {i}: split statistics");
            assert!(a.sp.run == 24);
        }
        for (si, sp) in s_new.splits.iter().enumerate() {
            for fi in 0..sp.folds.len() {
                let x = s_new.best[si][fi].map(|b| (b.key, b.tie, no_slack(b.learn), no_slack(b.test), no_slack(b.after), b.fresh_ok));
                let y = s_old.best[si][fi].map(|b| (b.key, b.tie, no_slack(b.learn), no_slack(b.test), no_slack(b.after), b.fresh_ok));
                assert_eq!(x, y, "split {} fold {fi}", sp.id);
            }
        }
    }

    /// v3.2 ROUND B: the memory an account carries under SPLITS18 (the day-major batch gate): one allocation of at most 1,000 bytes and no
    /// month series unless WAYA18.
    #[test]
    fn v32rb_seg_memory_bound() {
        let _ = on(97);
        crate::sw::set(crate::sw::WAYA18, false);
        let b = Ser18::boxed().unwrap_or_else(|| panic!("kept under SPLITS18"));
        assert!(b.m.is_none(), "no month series without WAYA18");
        assert!(std::mem::size_of::<Ser18>() <= 1_000, "Ser18 {} bytes", std::mem::size_of::<Ser18>());
        assert!(!Sel18::new().way_a);
    }

    /// EXACTNESS: every window of every random account, read in O(1) from the table, equals a brute-force recomputation from the raw
    /// closes (net, max drawdown from the window's start, trades, months >= Rs 2 L, positive sessions, positive week pieces).
    #[test]
    fn v32b_split_windows_exact() {
        let c = on(26);
        let mut r = Rng(0x9e37_79b9_7f4a_7c15);
        let mut n = 0;
        for _ in 0..40 {
            let cl = closes(&mut r, c.days.len());
            let ms = series(&cl);
            let d = table(&ms);
            for a in 0..c.m() {
                for b in a..c.m() {
                    let f = d.query(a, b);
                    let (net, dd, tr, mok, dpos, wpos) = brute(c, &cl, a, b);
                    assert_eq!((f.net, f.mdd, f.trades, f.mo_ok, f.d_pos, f.wk_pos), (net, dd, tr, mok, dpos, wpos), "window {a}..{b}");
                    assert_eq!(f.mo_n, u16::try_from(b - a + 1).unwrap_or(0));
                    n += 1;
                }
            }
        }
        assert!(n > 10_000, "windows checked {n}");
    }

    /// The splits of a 2018-10 .. 2026-10 calendar: 6 TT cuts and 18 walk-forward splits, their folds and boundaries.
    #[test]
    fn v32b_split_definitions() {
        let c = on(97);
        let v = splits(c);
        assert_eq!(v.len(), 24);
        let tt = v.iter().find(|x| x.id == "TT-2019").cloned().unwrap_or_else(|| panic!("TT-2019"));
        // 2018-10 .. 2019-12 = 15 months learn, test from 2020-01 (index 15) to the last month (96)
        assert_eq!(tt.folds, vec![((0, 14), (15, 96))]);
        let an = v.iter().find(|x| x.id == "WF-AN-L12-S3").cloned().unwrap_or_else(|| panic!("AN"));
        let ro = v.iter().find(|x| x.id == "WF-RO-L12-S3").cloned().unwrap_or_else(|| panic!("RO"));
        assert_eq!(an.folds.first().copied(), Some(((0, 11), (12, 14))));
        assert_eq!(an.folds.get(1).copied(), Some(((0, 14), (15, 17))));
        assert_eq!(ro.folds.get(1).copied(), Some(((3, 14), (15, 17))));
        assert_eq!(an.folds.len(), 29);
        assert_eq!(an.folds.last().copied(), Some(((0, 95), (96, 96))));
        let s1 = v.iter().find(|x| x.id == "WF-RO-L24-S1").cloned().unwrap_or_else(|| panic!("S1"));
        assert_eq!(s1.folds.len(), 73);
        assert!(v.iter().all(|x| x.run()));
        // every fold's learning months end before its test months start; tests of one split never overlap
        for x in &v {
            for f in &x.folds {
                assert!(f.0.1 < f.1.0 && f.0.0 <= f.0.1 && f.1.0 <= f.1.1);
            }
            for w in x.folds.windows(2) {
                assert!(w[0].1.1 < w[1].1.0);
            }
        }
        // a 5-month run: no walk-forward split (L >= 6) and no year cut with a test month after 2019-02
        let c5 = on(5);
        assert!(splits(c5).iter().all(|x| !x.run()));
    }

    /// Random accounts offered to every fold.
    fn picks(accts: &[(Ser18, Sum)]) -> Sel18 {
        let mut sel = Sel18::new();
        for (i, (ms, sm)) in accts.iter().enumerate() {
            let mut s = *sm;
            account(Some(ms), &mut s, &mut sel, true, (0, 0, u32::try_from(i).unwrap_or(0), 0, 0));
        }
        sel
    }

    /// THE NAMED NO-LOOK-AHEAD TEST: every fold's pick reads its learning months only: replacing every account's months AFTER a fold's
    /// learning window with other random months never changes that fold's pick or its learning figures.
    #[test]
    fn v32b_split_choice_reads_learning_months_only() {
        let c = on(40);
        let mut r = Rng(0x1234_5678_9abc_def1);
        let mk = |r: &mut Rng| -> Vec<(Vec<(u16, u32, i64)>, Sum)> { (0..30).map(|_| (closes(r, c.days.len()), Sum { trades: 1, ..Sum::default() })).collect() };
        let base = mk(&mut r);
        let accts: Vec<(Ser18, Sum)> = base.iter().map(|(cl, s)| (series(cl), *s)).collect();
        let sel = picks(&accts);
        let mut checked = 0;
        for cut in [7_usize, 12, 20, 30] {
            // every account: the same closes up to the cut month, other random closes after it
            let other = mk(&mut r);
            let mixed: Vec<(Ser18, Sum)> = base.iter().zip(&other).map(|((a, s), (b, _))| {
                let keep: Vec<(u16, u32, i64)> = a.iter().copied().filter(|x| usize::from(c.mo_of_day[usize::from(x.0)]) <= cut).chain(b.iter().copied().filter(|x| usize::from(c.mo_of_day[usize::from(x.0)]) > cut)).collect();
                (series(&keep), *s)
            }).collect();
            let sel2 = picks(&mixed);
            for (si, sp) in sel.splits.iter().enumerate() {
                for (fi, f) in sp.folds.iter().enumerate() {
                    if f.0.1 > cut {
                        continue;
                    }
                    let a = sel.best[si][fi].map(|b| (b.tie, b.learn));
                    let b = sel2.best[si][fi].map(|b| (b.tie, b.learn));
                    assert_eq!(a, b, "split {} fold {fi} (learn to month {}) changed when months after {cut} changed", sp.id, f.0.1);
                    checked += 1;
                }
            }
        }
        assert!(checked > 100, "folds checked {checked}");
    }

    /// The addendum 15 / 19 order: months >= Rs 2 L first, then positive week pieces, then positive sessions, then the smaller max
    /// drawdown, then the bigger net.
    #[test]
    fn v32b_split_order_months_weeks_days_dd_net() {
        let f = |mo_ok: u16, wk_pos: u16, d_pos: u16, mdd: i64, net: i64| Fig18 { mo_ok, wk_pos, d_pos, mdd, net, ..Fig18::default() };
        assert!(f(3, 0, 0, 900, -5).key() > f(2, 9, 99, 0, 1_000_000).key());
        assert!(f(3, 5, 0, 900, -5).key() > f(3, 4, 99, 0, 1_000_000).key());
        assert!(f(3, 5, 7, 900, -5).key() > f(3, 5, 6, 0, 1_000_000).key(), "more positive sessions beat a smaller drawdown");
        assert!(f(3, 5, 7, 100, -5).key() > f(3, 5, 7, 200, 1_000_000).key());
        assert!(f(3, 5, 7, 100, 6).key() > f(3, 5, 7, 100, 5).key());
    }

    /// PASSED needs EVERY walk-forward split that ran and at least 80% of all splits that ran (addendum 18's default).
    #[test]
    fn v32b_split_passed_rule() {
        let p = |run, pass19, wf_run, wf_pass19| SpStat { run, pass19, wf_run, wf_pass19, ..SpStat::default() };
        assert!(passed(&p(24, 24, 18, 18)));
        assert!(passed(&p(24, 20, 18, 18)), "20 of 24 = 83%");
        assert!(!passed(&p(24, 19, 18, 18)), "19 of 24 = 79%");
        assert!(!passed(&p(24, 23, 18, 17)), "one walk-forward split failed");
        assert!(!passed(&p(6, 6, 0, 0)), "no walk-forward split ran");
        assert!(!passed(&p(0, 0, 0, 0)));
    }

    /// The whole run as one window equals the account's own summary (net, trades, positive sessions, max drawdown) on a real walk
    /// through the kernel (events booked by step3, closes realized by flush: the hooks under test), grouped and booking order alike.
    #[test]
    fn v32b_split_whole_window_equals_account() {
        use crate::acct::SEv;
        use crate::kern::{CSet, walk_account};
        use crate::stage::{EvX, NR, Tab3};
        use crate::trade::{EvT, NL, Out};
        for grouped in [false, true] {
            let c = on(3);
            crate::sw::set(crate::sw::DD_GROUPED, grouped);
            let nd = c.days.len();
            let mut evs: Vec<SEv> = Vec::new();
            let mut ex: Vec<EvX> = Vec::new();
            let mut tab = Tab3::default();
            let mut r = Rng(0xfeed_beef_u64 + u64::from(grouped));
            for d in 0..nd {
                for k in 0..3_u32 {
                    let t = 36_000 + k * 600;
                    let et = EvT { lots: [1; 4], q_of: [0; 4], qs: vec![1], margin_lot: 10_000_000, filled: [true; NL], ..EvT::default() };
                    let x = EvX { r0: u32::try_from(tab.anc.len()).unwrap_or(0), t0: 0, nq: 1, tf: Vec::new(), fill: Vec::new(), mark1: 0 };
                    // two events of a day can close in the same second (grouped reading)
                    let exit_s = if k == 0 { 300 } else { 2_000 };
                    for _l in 0..NL {
                        let mut a = [crate::stage::ABSENT; NR];
                        a[0] = crate::stage::NEVER3;
                        tab.anc.push(a);
                        tab.os.push([0; NR]);
                        tab.obase.push(u32::try_from(tab.outs.len()).unwrap_or(0));
                        let net = i32::try_from(r.range(-3_000_000, 4_000_000)).unwrap_or(0);
                        tab.outs.push(Out { net, exit: u16::try_from(i64::from(36_000 + exit_s) - crate::trade::BASE_S).unwrap_or(0), flag: 1, ..Out::default() });
                    }
                    let di = u16::try_from(d).unwrap_or(0);
                    evs.push(SEv { di, ci: k + 10 * u32::from(di), t, skey: k + 10 * u32::from(di), tab: et });
                    ex.push(x);
                }
            }
            let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap_or(0)).collect();
            let keys = crate::kern::list_keys(&evs, &list);
            let cs = CSet { rules: vec![(false, 0)], sl: 0 };
            let (sm, pc, _) = walk_account(&evs, &ex, &tab, &list, &keys, &cs, 0, 1, None, true, true, &[], 0, u16::try_from(nd).unwrap_or(0));
            let ms = pc.ms.as_deref().unwrap_or_else(|| panic!("the series is kept under SPLITS18"));
            assert!(ms.m.is_some());
            let d = table(ms);
            let f = d.query(0, c.m() - 1);
            assert!(sm.trades > 10);
            assert_eq!((f.net, f.trades, f.d_pos, f.mdd), (sm.net, sm.trades, sm.days_pos, sm.mdd), "grouped {grouped}");
            // round B: the segments composed over the whole run give the same
            let g = seg_window(&ms.s.figs(), 0, c.nseg() - 1);
            assert_eq!((g.net, g.trades, g.d_pos, g.mdd), (sm.net, sm.trades, sm.days_pos, sm.mdd), "segments, grouped {grouped}");
        }
    }

    /// The fresh-start slack test on month pieces equals wf.rs `fresh_equal` on the same pieces (a window starting after a loss or a gain is
    /// proven equal only when no cash check inside it sits between the two readings).
    #[test]
    fn v32b_split_fresh_start_test_matches_wf() {
        let c = on(6);
        let mut r = Rng(0x0dd_ba11);
        let mut n_false = 0;
        for _ in 0..400 {
            let ms: Vec<MPiece> = (0..c.m()).map(|_| MPiece { net: r.range(-3, 3) * 1_000_000, min_pass: if r.range(0, 2) == 0 { i64::MAX } else { r.range(-4, 4) * 1_000_000 }, max_ref: if r.range(0, 2) == 0 { i64::MIN } else { r.range(-4, 4) * 1_000_000 }, ..MPiece::default() }).collect();
            let items: Vec<Fig18> = ms.iter().enumerate().map(|(i, x)| Fig18::of(x, i, c)).collect();
            let mut d = Dst::default();
            d.build(&items);
            let pcs: Vec<crate::kern::Piece> = ms.iter().map(|x| crate::kern::Piece { net: x.net, min_pass: x.min_pass, max_ref: x.max_ref, ..crate::kern::Piece::default() }).collect();
            for a in 0..c.m() {
                for b in a..c.m() {
                    let f = fresh_ok(&d, a, b);
                    assert_eq!(f, crate::wf::fresh_equal(&pcs, a, b), "window {a}..{b}");
                    n_false += usize::from(!f);
                }
            }
        }
        assert!(n_false > 100, "the test is not vacuous: {n_false} windows not proven equal");
    }

    /// COST at full history scale (MEASURED when run with --ignored --nocapture; never a gate: the machine is shared): one account of 97
    /// months (2018-10 .. 2026-10) offered to all 24 splits (every fold), p50 / p99 ns per account over 2,000 accounts.
    #[test]
    #[ignore = "a measurement, run explicitly"]
    fn v32b_split_cost_97_months() {
        let c = on(97);
        // the engine's default: segments only (WAYA18 off); the month series of the reference is dropped before the timing
        crate::sw::set(crate::sw::WAYA18, false);
        let mut r = Rng(0x5eed_0097);
        let accts: Vec<Ser18> = (0..2_000).map(|_| { let mut s = series(&closes(&mut r, c.days.len())); s.m = None; s }).collect();
        let mut sel = Sel18::new();
        let folds: usize = sel.splits.iter().map(|s| s.folds.len()).sum();
        let mut ns: Vec<u128> = Vec::new();
        for (i, ms) in accts.iter().enumerate() {
            let mut sm = Sum { trades: 1, ..Sum::default() };
            let t0 = std::time::Instant::now();
            account(Some(ms), &mut sm, &mut sel, true, (0, 0, u32::try_from(i).unwrap_or(0), 0, 0));
            ns.push(t0.elapsed().as_nanos());
        }
        ns.sort_unstable();
        println!("SPLIT COST (97 months, {} splits, {folds} folds, {} accounts): p50 {} ns, p99 {} ns per account; Ser18 {} bytes ({} segments of {} bytes; v3.2-B's MSeries {} bytes + {} x MPiece {} bytes)", sel.splits.len(), accts.len(), ns[ns.len() / 2], ns[ns.len() * 99 / 100], std::mem::size_of::<Ser18>(), c.nseg(), std::mem::size_of::<SPiece>(), std::mem::size_of::<MSeries>(), c.m(), std::mem::size_of::<MPiece>());
    }

    /// The switch rule: with SPLITS18 off the series is empty (every hook inert) and the account summary carries no split statistics.
    #[test]
    fn v32b_split_off_is_inert() {
        let _ = on(3);
        crate::sw::set(crate::sw::SPLITS18, false);
        let ms = MSeries::new();
        assert!(!ms.on());
        assert!(MSeries::boxed().is_none(), "nothing allocated per account when off");
        assert!(Ser18::boxed().is_none(), "round B: nothing allocated per account when off");
        let sel = Sel18::new();
        assert!(sel.splits.is_empty());
        let mut sm = Sum::default();
        let mut sel = sel;
        let c = cal().unwrap_or_else(|| panic!("cal"));
        let ser = Ser18 { s: SSeries::new(c), m: None };
        account(Some(&ser), &mut sm, &mut sel, true, (0, 0, 0, 0, 0));
        assert_eq!(sm.sp, SpStat::default());
    }

    /// v3.2 ROUND B, O(1) GATES (matrix X65-style, MEASURED and GATED): one close through the split hooks is O(1) (p99 under 20 us on this
    /// shared machine); one account's split figures at its finish cost O(segments + splits), independent of the history length (97 months
    /// at most 3x of 25 months at p50) and under 2 ms at p99.
    #[test]
    fn v32rb_split_o1_gates() {
        let mut p50 = Vec::new();
        for months in [25_usize, 97] {
            let c = on(months);
            crate::sw::set(crate::sw::WAYA18, false);
            let mut r = Rng(0x0001_5eed + u64::try_from(months).unwrap_or(0));
            let mut close_ns: Vec<u128> = Vec::new();
            let mut acc_ns: Vec<u128> = Vec::new();
            let mut sel = Sel18::new();
            for i in 0..300 {
                let cl = closes(&mut r, c.days.len());
                let mut s = Ser18 { s: SSeries::new(c), m: None };
                let mut real = 0_i64;
                for (j, &(d, t, n)) in cl.iter().enumerate() {
                    real += n;
                    let t0 = std::time::Instant::now();
                    s.close(u32::from(d) * 86_400 + t, real, n, true);
                    if j % 8 == 0 {
                        close_ns.push(t0.elapsed().as_nanos());
                    }
                }
                let mut sm = Sum { trades: 1, ..Sum::default() };
                let t0 = std::time::Instant::now();
                account(Some(&s), &mut sm, &mut sel, true, (0, 0, u32::try_from(i).unwrap_or(0), 0, 0));
                acc_ns.push(t0.elapsed().as_nanos());
            }
            close_ns.sort_unstable();
            acc_ns.sort_unstable();
            let q = |v: &[u128], p: usize| v.get((v.len() - 1) * p / 100).copied().unwrap_or(0);
            assert!(q(&close_ns, 99) < 20_000, "{months} months: close p99 {} ns", q(&close_ns, 99));
            assert!(q(&acc_ns, 99) < 2_000_000, "{months} months: account p99 {} ns", q(&acc_ns, 99));
            p50.push(q(&acc_ns, 50).max(1));
        }
        assert!(p50[1] <= 3 * p50[0] + 2_000, "the account's split cost grows with the months: p50 {} ns at 25 vs {} ns at 97", p50[0], p50[1]);
    }
}
