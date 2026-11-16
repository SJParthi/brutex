//! The LINE engine, the only stage-1 engine (SNIPER-ADANIENT-PLAN 8.3, B1).
//!
//! For one outer combo the kept set S of candidate rows is known. The rows are grouped into classes by (RSI bucket, %R bucket).
//! For each of the `nr x nw` LINES (r, w) (OFF included): reset the accumulator (O(touched)); gather the classes with RSI bucket
//! at least r and %R bucket at least w into CCI buckets (a counting sort, O(rows + nc)); walk c = nc - 1 .. 0 (OFF last): insert the rows of
//! bucket c, then offer cell (r, c, w). Consecutive CCI levels whose kept TRADE set did not change form one tie run, offered once
//! with its length (`Lists::offer_learn(.., run)`, decision D-S14).
//!
//! **FIRST mode, C-IDX OFF** (8.3): when a row of session s enters, if s has no trade it is inserted; if s's trade is a LATER
//! candle, that trade is REMOVED and the new row inserted (the session's first qualifying candle moved earlier); otherwise
//! nothing changes. Each entering row causes at most one change. With C-IDX ON the kept set holds at most one row a session,
//! and the same rule is exact.
//!
//! **Red** (`--features red`): CUBE's suffix reading (B1): every qualifying candle is a trade (no FIRST mode). Mutants:
//! `m_s2` offers a run of n as n - 1; `m_s5` keeps the LATER candle.

use crate::steps::Hist;

/// No row.
pub const NONE: u32 = u32::MAX;

/// One candidate row of a context (a decision candle of one session): its session, its candle index in the session, and the
/// buckets of its three core readings ([`crate::ladder::Ladder::bucket`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Row {
    /// The session index (the host's dated session table).
    pub sess: u32,
    /// The candle index in its session (1 = the first candle; K14: counted from the session table's OPEN). POSITIONAL v2 (B5, 2026-11-15):
    /// u32 (was u16), so a positional GROUP's ordinal (up to about 8 x 10^5 candles on 1 minute) replicated 100x by the HARD gate never
    /// saturates; a session's candle number fits unchanged (P2: the OFF mode byte-identical).
    pub k: u32,
    /// RSI bucket (0 = passes only OFF).
    pub rb: u16,
    /// CCI bucket.
    pub cb: u16,
    /// %R bucket.
    pub wb: u16,
}

/// The core's value counts, OFF included (32 x 568 x 22 on the sealed ladders).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Core {
    /// RSI values.
    pub nr: u16,
    /// CCI values.
    pub nc: u16,
    /// %R values.
    pub nw: u16,
}

impl Core {
    /// Cells of the core.
    #[must_use]
    pub fn cells(&self) -> u64 {
        u64::from(self.nr) * u64::from(self.nc) * u64::from(self.nw)
    }
}

/// One core cell: value indices (0 = OFF).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Cell {
    /// RSI value index.
    pub r: u16,
    /// CCI value index (in an offered run: the LOWEST index of the run, i.e. the smallest code).
    pub c: u16,
    /// %R value index.
    pub w: u16,
}

/// Where the kernel hands every event: the accumulator and the lists (`wf_sweep::SweepAcc` + `wf_sweep::Lists` in the host).
pub trait Sink {
    /// A refusal of the sink (a test-month trade, an overflow ...).
    type Err;
    /// Empties the accumulator (O(touched)).
    fn reset(&mut self);
    /// Row `row` becomes a trade.
    ///
    /// # Errors
    /// The sink's refusal.
    fn insert(&mut self, row: u32) -> Result<(), Self::Err>;
    /// Row `row` stops being a trade.
    ///
    /// # Errors
    /// The sink's refusal.
    fn remove(&mut self, row: u32) -> Result<(), Self::Err>;
    /// A tie run: `run` consecutive CCI levels (from `cell.c` upward) with the same kept trade set.
    ///
    /// # Errors
    /// The sink's refusal.
    fn offer(&mut self, cell: Cell, run: u32) -> Result<(), Self::Err>;
}

/// What one walk counted (SN4's fields).
#[derive(Clone, Debug, Default)]
pub struct WalkStats {
    /// Outer combos walked.
    pub combos: u64,
    /// Lines walked.
    pub lines: u64,
    /// Lines skipped exactly because no row qualifies (every cell empty).
    pub lines_empty: u64,
    /// Core cells covered.
    pub cells: u64,
    /// Cells with no trade (never offered: E1 fails exactly).
    pub empty_cells: u64,
    /// Tie runs offered.
    pub runs_offered: u64,
    /// Cells inside offered runs.
    pub cells_offered: u64,
    /// Rows entering a line.
    pub entered: u64,
    /// Inserts handed to the sink.
    pub inserts: u64,
    /// Removals handed to the sink (FIRST mode moves).
    pub removes: u64,
    /// Kernel steps per entering row (the HARD gate's event).
    pub hist_enter: Hist,
    /// Kernel steps per line.
    pub hist_line: Hist,
}

impl WalkStats {
    /// Joins another shard's counts.
    pub fn merge(&mut self, o: &Self) {
        self.combos += o.combos;
        self.lines += o.lines;
        self.lines_empty += o.lines_empty;
        self.cells += o.cells;
        self.empty_cells += o.empty_cells;
        self.runs_offered += o.runs_offered;
        self.cells_offered += o.cells_offered;
        self.entered += o.entered;
        self.inserts += o.inserts;
        self.removes += o.removes;
        self.hist_enter.merge(&o.hist_enter);
        self.hist_line.merge(&o.hist_line);
    }
}

/// The engine's reusable buffers (one per worker thread).
#[derive(Clone, Debug)]
pub struct LineEngine {
    core: Core,
    cur: Vec<u32>,
    touched: Vec<u32>,
    classes: Vec<Vec<u32>>,
    nonempty: Vec<usize>,
    cnt: Vec<u32>,
    order: Vec<u32>,
}

impl LineEngine {
    /// Buffers for `sessions` sessions and the core `core`.
    #[must_use]
    pub fn new(sessions: usize, core: Core) -> Self {
        let ncls = usize::from(core.nr) * usize::from(core.nw);
        Self {
            core,
            cur: vec![NONE; sessions],
            touched: Vec::new(),
            classes: vec![Vec::new(); ncls],
            nonempty: Vec::new(),
            cnt: vec![0; usize::from(core.nc) + 1],
            order: Vec::new(),
        }
    }

    fn enter<S: Sink>(&mut self, rows: &[Row], j: u32, sink: &mut S, st: &mut WalkStats, trades: &mut u64) -> Result<bool, S::Err> {
        st.entered += 1;
        let Some(row) = rows.get(j as usize) else { return Ok(false) };
        let s = row.sess as usize;
        if cfg!(feature = "red") {
            // RED: CUBE's suffix reading: every qualifying candle is a trade.
            sink.insert(j)?;
            st.inserts += 1;
            *trades += 1;
            st.hist_enter.add(2);
            return Ok(true);
        }
        let cur = self.cur.get(s).copied().unwrap_or(NONE);
        if cur == NONE {
            sink.insert(j)?;
            st.inserts += 1;
            *trades += 1;
            if let Some(x) = self.cur.get_mut(s) {
                *x = j;
            }
            self.touched.push(row.sess);
            st.hist_enter.add(2);
            return Ok(true);
        }
        let ck = rows.get(cur as usize).map_or(u32::MAX, |r| r.k);
        let better = if cfg!(feature = "m_s5") { row.k > ck } else { row.k < ck };
        if better {
            sink.remove(cur)?;
            sink.insert(j)?;
            st.removes += 1;
            st.inserts += 1;
            if let Some(x) = self.cur.get_mut(s) {
                *x = j;
            }
            st.hist_enter.add(3);
            Ok(true)
        } else {
            st.hist_enter.add(1);
            Ok(false)
        }
    }

    fn flush<S: Sink>(sink: &mut S, st: &mut WalkStats, r: u16, c: u16, w: u16, run: u32, trades: u64) -> Result<(), S::Err> {
        if run == 0 {
            return Ok(());
        }
        if trades == 0 {
            st.empty_cells += u64::from(run);
            return Ok(());
        }
        let n = if cfg!(feature = "m_s2") && run > 1 { run - 1 } else { run };
        sink.offer(Cell { r, c, w }, n)?;
        st.runs_offered += 1;
        st.cells_offered += u64::from(run);
        Ok(())
    }

    /// Walks the core completely for one outer combo whose kept rows are `kept` (indices into `rows`).
    ///
    /// # Errors
    /// The sink's first refusal (the walk stops).
    pub fn walk<S: Sink>(&mut self, rows: &[Row], kept: impl Iterator<Item = usize>, sink: &mut S, st: &mut WalkStats) -> Result<(), S::Err> {
        let Core { nr, nc, nw } = self.core;
        st.combos += 1;
        for &c in &self.nonempty {
            if let Some(v) = self.classes.get_mut(c) {
                v.clear();
            }
        }
        self.nonempty.clear();
        for i in kept {
            let Some(row) = rows.get(i) else { continue };
            let cls = usize::from(row.rb.min(nr.saturating_sub(1))) * usize::from(nw) + usize::from(row.wb.min(nw.saturating_sub(1)));
            if let Some(v) = self.classes.get_mut(cls) {
                if v.is_empty() {
                    self.nonempty.push(cls);
                }
                v.push(u32::try_from(i).unwrap_or(NONE));
            }
        }
        let line_steps = 1 + u64::from(nc) + u64::from(nr) * u64::from(nw);
        for r in 0..nr {
            for w in 0..nw {
                st.lines += 1;
                st.cells += u64::from(nc);
                st.hist_line.add(line_steps);
                // Gather the qualifying rows into CCI buckets (counting sort).
                self.cnt.iter_mut().for_each(|x| *x = 0);
                let mut n_line = 0_usize;
                for &cls in &self.nonempty {
                    let (cr, cw) = (cls / usize::from(nw), cls % usize::from(nw));
                    if cr < usize::from(r) || cw < usize::from(w) {
                        continue;
                    }
                    for &j in self.classes.get(cls).map_or(&[][..], Vec::as_slice) {
                        let cb = rows.get(j as usize).map_or(0, |x| x.cb.min(nc.saturating_sub(1)));
                        if let Some(x) = self.cnt.get_mut(usize::from(cb)) {
                            *x += 1;
                        }
                        n_line += 1;
                    }
                }
                if n_line == 0 {
                    st.lines_empty += 1;
                    st.empty_cells += u64::from(nc);
                    continue;
                }
                // Prefix sums: bucket c occupies order[start[c] .. start[c + 1]).
                let mut acc = 0_u32;
                for x in &mut self.cnt {
                    let v = *x;
                    *x = acc;
                    acc += v;
                }
                self.order.clear();
                self.order.resize(n_line, NONE);
                let mut fill = self.cnt.clone();
                for &cls in &self.nonempty {
                    let (cr, cw) = (cls / usize::from(nw), cls % usize::from(nw));
                    if cr < usize::from(r) || cw < usize::from(w) {
                        continue;
                    }
                    for &j in self.classes.get(cls).map_or(&[][..], Vec::as_slice) {
                        let cb = usize::from(rows.get(j as usize).map_or(0, |x| x.cb.min(nc.saturating_sub(1))));
                        if let Some(p) = fill.get_mut(cb) {
                            if let Some(o) = self.order.get_mut(*p as usize) {
                                *o = j;
                            }
                            *p += 1;
                        }
                    }
                }
                // Reset the accumulator and the FIRST-mode map (O(touched)).
                sink.reset();
                for s in self.touched.drain(..) {
                    if let Some(x) = self.cur.get_mut(s as usize) {
                        *x = NONE;
                    }
                }
                let mut trades = 0_u64;
                let mut run = 0_u32;
                for c in (0..nc).rev() {
                    let (a, b) = (
                        self.cnt.get(usize::from(c)).copied().unwrap_or(0) as usize,
                        self.cnt.get(usize::from(c) + 1).copied().unwrap_or(0) as usize,
                    );
                    // Will this bucket change the kept trade set? (checked BEFORE any insert, so the open run is offered with
                    // its own set: the sink reads the accumulator as it stands at the offer).
                    if run > 0 && a < b && self.changes(rows, a, b) {
                        Self::flush(sink, st, r, c + 1, w, run, trades)?;
                        run = 0;
                    }
                    for idx in a..b {
                        let j = self.order.get(idx).copied().unwrap_or(NONE);
                        if j != NONE {
                            self.enter(rows, j, sink, st, &mut trades)?;
                        }
                    }
                    run += 1;
                }
                Self::flush(sink, st, r, 0, w, run, trades)?;
            }
        }
        Ok(())
    }

    /// Whether entering `order[a..b]` changes the kept trade set (a pure check: no sink call).
    fn changes(&self, rows: &[Row], a: usize, b: usize) -> bool {
        if cfg!(feature = "red") {
            return true;
        }
        (a..b).any(|idx| {
            let j = self.order.get(idx).copied().unwrap_or(NONE);
            let Some(row) = rows.get(j as usize) else { return false };
            let cur = self.cur.get(row.sess as usize).copied().unwrap_or(NONE);
            if cur == NONE {
                return true;
            }
            let ck = rows.get(cur as usize).map_or(u32::MAX, |r| r.k);
            if cfg!(feature = "m_s5") { row.k > ck } else { row.k < ck }
        })
    }
}
