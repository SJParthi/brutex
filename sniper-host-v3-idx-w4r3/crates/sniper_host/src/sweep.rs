//! S0 + S1 of one context (plan 8.1-8.2): the core alone, then every wired outer value alone x the full core, sharded over the
//! worker threads (the count a RUN data row, SNIPER s14). Each worker holds its own `LineEngine`, `SweepAcc` and `Lists` per
//! (view, level column); `Lists::merge` joins them (associative, P18). The sink below is wiring only: every key part is
//! `wf_sweep`'s (the D-S14 additions), every engine step the kernel's.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use sniper_kernel::bits::Bits;
use sniper_kernel::code::Code;
use sniper_kernel::line::{Cell, Core, LineEngine, Row, Sink, WalkStats};
use sniper_kernel::steps::Hist;
use wf_sweep::acc::{Layout, OfferSink, Outcome, SweepAcc};
use wf_sweep::lists::Lists;
use wf_sweep::views::View;
use wf_sweep::{MAXC, SweepErr};

use crate::cands::OuterValue;

/// Everything one context's sweep reads.
pub struct Ctx<'a> {
    /// The kernel rows (TT-learning rows with a priced outcome), in time order.
    pub rows: &'a [Row],
    /// The outcome of each row (same index).
    pub out: &'a [Outcome],
    /// The learning layout.
    pub lay: &'a Layout,
    /// The views.
    pub views: &'a [View],
    /// Level columns (5).
    pub cols: usize,
    /// The core.
    pub core: Core,
    /// Sessions in the table.
    pub sessions: usize,
    /// The list bound (`stage2_top`).
    pub top: u64,
    /// Every dial's radix in the file order (OFF included).
    pub radix: &'a [u64],
    /// The positions of M-RSI, M-CCI, M-WR in the dial list.
    pub core_pos: [usize; 3],
    /// S7's XE context filter (rows on the WITH side of EMA P), AND-ed onto every combo.
    pub filter: Option<&'a Bits>,
}

/// One context's sweep result.
pub struct Out {
    /// Lists per (view, column): index view * cols + column.
    pub lists: Vec<Lists>,
    /// The kernel's walk counts.
    pub walk: WalkStats,
    /// Steps per accumulator insert / removal event.
    pub hist_acc: Hist,
    /// Steps per key read.
    pub hist_key: Hist,
    /// Not eligible counts: no trade, part not positive, below floor, passes E2 (a MISS view).
    pub not_eligible: [u64; 4],
    /// Outer values walked (S0 counted as one).
    pub values_walked: u64,
    /// Outer values EMPTY by construction (S7), never walked.
    pub values_empty: u64,
    /// Sink refusals (the first one names the run's refusal).
    pub refused: Vec<String>,
}

/// The canonical code's base of one outer value: every digit 0 but the outer dial's (plan 3.4: M-CCI least significant, then M-RSI,
/// M-WR, then every other dial in the file order).
fn base_code(ctx: &Ctx<'_>, v: Option<&OuterValue>) -> Option<Code> {
    let mut digits: Vec<(u64, u64)> = Vec::with_capacity(ctx.radix.len());
    for &p in &[ctx.core_pos[1], ctx.core_pos[0], ctx.core_pos[2]] {
        digits.push((0, *ctx.radix.get(p)?));
    }
    for (i, &r) in ctx.radix.iter().enumerate() {
        if ctx.core_pos.contains(&i) {
            continue;
        }
        let d = v.filter(|x| x.dial == i).map_or(0, |x| x.value);
        digits.push((d, r));
    }
    Code::mixed(&digits)
}

struct HostSink<'a> {
    acc: SweepAcc<'a>,
    lists: Vec<Lists>,
    views: &'a [View],
    core: Core,
    base: Code,
    hist_acc: Hist,
    hist_key: Hist,
    ne: [u64; 4],
    /// Key reads per step count (flushed into `hist_key` when the worker ends: the same histogram as one add per read).
    tally: Vec<u64>,
    /// Key reads of TALLY_MAX steps or more (flushed one by one).
    big: Vec<u64>,
}

impl Sink for HostSink<'_> {
    type Err = SweepErr;
    fn reset(&mut self) {
        // A reset that fails leaves the accumulator inconsistent: the next insert refuses (LeafOccupied), so it is never silent.
        let _ = self.acc.reset();
    }
    fn insert(&mut self, row: u32) -> Result<(), SweepErr> {
        let s = self.acc.insert(row)?;
        self.hist_acc.add(s);
        Ok(())
    }
    fn remove(&mut self, row: u32) -> Result<(), SweepErr> {
        let s = self.acc.remove(row)?;
        self.hist_acc.add(s);
        Ok(())
    }
    fn offer(&mut self, cell: Cell, run: u32) -> Result<(), SweepErr> {
        let core_code = u64::from(cell.c) + u64::from(self.core.nc) * (u64::from(cell.r) + u64::from(self.core.nr) * u64::from(cell.w));
        let Some(code) = self.base.plus(core_code) else { return Err(SweepErr::Overflow) };
        // Speed build: every (view, level column) list is offered at once, grouped by (column, window) inside wf_sweep
        // (SweepAcc::offer_all: the same offer, gate, step count and not-eligible reason per list as one view_key per list).
        let mut sink = OfferSink { lists: &mut self.lists, ne: &mut self.ne, tally: &mut self.tally, big: &mut self.big };
        self.acc.offer_all(self.views, code.0, u64::from(run), &mut sink)
    }
}

struct Shard {
    lists: Vec<Lists>,
    walk: WalkStats,
    hist_acc: Hist,
    hist_key: Hist,
    ne: [u64; 4],
    walked: u64,
    empty: u64,
    refused: Vec<String>,
}

/// Runs S0 (the core alone) and S1 (each value of `values` alone x the full core) over `threads` workers.
///
/// # Errors
/// A context the accumulator refuses (more columns than `MAXC`).
pub fn run(ctx: &Ctx<'_>, values: &[OuterValue], threads: usize) -> Result<Out, String> {
    run_with(ctx, values, None, threads)
}

/// A value's kept set made when it is walked (the seconds families, design change (d): never every value's set at once).
pub type KeptOf<'a> = &'a (dyn Fn(usize) -> Bits + Sync);

/// [`run`] with each value's kept set made by `kept_of(value index)` when it is walked (SECONDS BUILDER, 2026-11-14) instead of read
/// from `OuterValue::kept`; `None` reads `OuterValue::kept` (the minutes families, unchanged).
///
/// # Errors
/// A context the accumulator refuses (more columns than `MAXC`).
pub fn run_with(ctx: &Ctx<'_>, values: &[OuterValue], kept_of: Option<KeptOf<'_>>, threads: usize) -> Result<Out, String> {
    if ctx.cols == 0 || ctx.cols > MAXC {
        return Err(format!("{} columns", ctx.cols));
    }
    let next = AtomicUsize::new(0);
    let total = values.len() + 1;
    let shards: Mutex<Vec<Shard>> = Mutex::new(Vec::new());
    let n = ctx.rows.len();
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                let Ok(acc) = SweepAcc::new(ctx.lay, ctx.out, ctx.cols, ctx.sessions) else { return };
                let mut sink = HostSink {
                    acc,
                    lists: bounded_lists(ctx),
                    views: ctx.views,
                    core: ctx.core,
                    base: Code::default(),
                    hist_acc: Hist::new(),
                    hist_key: Hist::new(),
                    ne: [0; 4],
                    tally: Vec::new(),
                    big: Vec::new(),
                };
                let mut eng = LineEngine::new(ctx.sessions, ctx.core);
                let mut sh = Shard { lists: Vec::new(), walk: WalkStats::default(), hist_acc: Hist::new(), hist_key: Hist::new(), ne: [0; 4], walked: 0, empty: 0, refused: Vec::new() };
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= total {
                        break;
                    }
                    let v = if i == 0 { None } else { values.get(i - 1) };
                    if v.is_some_and(|x| x.empty) {
                        sh.empty += 1;
                        continue;
                    }
                    let Some(base) = base_code(ctx, v) else {
                        sh.refused.push(format!("code overflow at value {i}"));
                        continue;
                    };
                    sink.base = base;
                    let mut kept = match (v, kept_of) {
                        (None, _) => Bits::full(n),
                        (Some(_), Some(f)) => f(i - 1),
                        (Some(x), None) => x.kept.clone(),
                    };
                    if let Some(f) = ctx.filter {
                        kept.and_with(f);
                    }
                    if let Err(e) = eng.walk(ctx.rows, kept.ones(), &mut sink, &mut sh.walk) {
                        sh.refused.push(format!("value {i}: {e}"));
                    }
                    sh.walked += 1;
                }
                for (s, &k) in sink.tally.iter().enumerate() {
                    sink.hist_key.add_n(s as u64, k);
                }
                    for &s in &sink.big {
                        sink.hist_key.add(s);
                    }
                sh.lists = sink.lists;
                sh.hist_acc = sink.hist_acc;
                sh.hist_key = sink.hist_key;
                sh.ne = sink.ne;
                if let Ok(mut g) = shards.lock() {
                    g.push(sh);
                }
            });
        }
    });
    let shards = shards.into_inner().map_err(|_| "a worker panicked".to_string())?;
    let mut out = Out {
        lists: bounded_lists(ctx),
        walk: WalkStats::default(),
        hist_acc: Hist::new(),
        hist_key: Hist::new(),
        not_eligible: [0; 4],
        values_walked: 0,
        values_empty: 0,
        refused: Vec::new(),
    };
    for sh in &shards {
        for (a, b) in out.lists.iter_mut().zip(&sh.lists) {
            a.merge(b);
        }
        out.walk.merge(&sh.walk);
        out.hist_acc.merge(&sh.hist_acc);
        out.hist_key.merge(&sh.hist_key);
        for (a, b) in out.not_eligible.iter_mut().zip(sh.ne) {
            *a += b;
        }
        out.values_walked += sh.walked;
        out.values_empty += sh.empty;
        out.refused.extend(sh.refused.iter().cloned());
    }
    Ok(out)
}

/// One list per (view, column), each bounded by its view row's `k` (the run's `stage2_top` when the row names none).
fn bounded_lists(ctx: &Ctx<'_>) -> Vec<Lists> {
    let mut v = Vec::with_capacity(ctx.views.len() * ctx.cols);
    for view in ctx.views {
        for _ in 0..ctx.cols {
            v.push(Lists::new(view.k.unwrap_or(ctx.top)));
        }
    }
    v
}

/// One event the kernel hands its sink (the speed build's shared walk).
#[derive(Clone, Copy, Debug)]
enum Ev {
    Reset,
    Insert(u32),
    Remove(u32),
    Offer(Cell, u32),
}

/// Events buffered before they are handed on (bounded memory: 1 MB a worker whatever the value's walk length).
pub const CHUNK: usize = 1 << 16;

/// The fan-out sink of the shared walk: it buffers the kernel's events and hands each full chunk (and the rest at the value's
/// end) to every N's `HostSink` in order; a sink's first refusal stops ITS deliveries for the rest of the value (exactly where
/// a walk with that sink would have stopped), and the fan-out itself never refuses, so the walk always runs to its end for the
/// others.
struct Fan<'s, 'a> {
    ev: Vec<Ev>,
    chunk: usize,
    sinks: &'s mut [HostSink<'a>],
    err: Vec<Option<SweepErr>>,
}

impl Fan<'_, '_> {
    fn push(&mut self, e: Ev) {
        self.ev.push(e);
        if self.ev.len() >= self.chunk {
            self.flush();
        }
    }

    fn flush(&mut self) {
        for (sink, err) in self.sinks.iter_mut().zip(self.err.iter_mut()) {
            if err.is_none()
                && let Err(e) = replay(sink, &self.ev)
            {
                *err = Some(e);
            }
        }
        self.ev.clear();
    }
}

impl Sink for Fan<'_, '_> {
    type Err = SweepErr;
    fn reset(&mut self) {
        self.push(Ev::Reset);
    }
    fn insert(&mut self, row: u32) -> Result<(), SweepErr> {
        self.push(Ev::Insert(row));
        Ok(())
    }
    fn remove(&mut self, row: u32) -> Result<(), SweepErr> {
        self.push(Ev::Remove(row));
        Ok(())
    }
    fn offer(&mut self, cell: Cell, run: u32) -> Result<(), SweepErr> {
        self.push(Ev::Offer(cell, run));
        Ok(())
    }
}

/// Hands a recorded event sequence to one sink in order, stopping at the sink's first refusal: exactly what that sink receives
/// when the kernel walks with it (the walk's control flow reads only the engine's own state, never the sink's, and a walk stops
/// at the sink's first refusal).
fn replay(sink: &mut HostSink<'_>, ev: &[Ev]) -> Result<(), SweepErr> {
    for e in ev {
        match *e {
            Ev::Reset => sink.reset(),
            Ev::Insert(r) => sink.insert(r)?,
            Ev::Remove(r) => sink.remove(r)?,
            Ev::Offer(c, n) => sink.offer(c, n)?,
        }
    }
    Ok(())
}

struct ShardN {
    lists: Vec<Vec<Lists>>,
    walk: WalkStats,
    hist_acc: Vec<Hist>,
    hist_key: Vec<Hist>,
    ne: Vec<[u64; 4]>,
    walked: u64,
    empty: u64,
    refused: Vec<Vec<String>>,
}

/// [`run`] for several outcome tables of ONE context at once (the N of the grid, speed build): the kernel walks each outer
/// value once and its events are handed, in chunks of [`CHUNK`], to one `HostSink` per table. Element `j` of the result equals
/// `run` on `ctx` with `out = outs[j]`: the same values walked by the same engine give the same events (the walk never reads its
/// sink), every list receives the same offers (the worker a value lands on is free in `run` too: lists merge associatively,
/// histograms and counts add), and a sink's refusal stops its deliveries where the walk with that sink would stop. The walk
/// counts are the walk's own, the same for every table; when any sink refused, every table is swept again with `run` (whose
/// walk stops at its sink's refusal, so its counts are partial there).
///
/// # Errors
/// A context the accumulator refuses (more columns than `MAXC`).
pub fn run_all(ctx: &Ctx<'_>, outs: &[&[Outcome]], values: &[OuterValue], threads: usize) -> Result<Vec<Out>, String> {
    run_all_with(ctx, outs, values, threads, CHUNK)
}

/// [`run_all`] with the event chunk `chunk` (tests use small chunks to cross every chunk boundary).
///
/// # Errors
/// A context the accumulator refuses (more columns than `MAXC`).
pub fn run_all_with(ctx: &Ctx<'_>, outs: &[&[Outcome]], values: &[OuterValue], threads: usize, chunk: usize) -> Result<Vec<Out>, String> {
    run_all_kept(ctx, outs, values, None, threads, chunk)
}

/// [`run_all_with`] with each value's kept set made by `kept_of(value index)` when it is walked, ONCE for every table (SECONDS
/// PORTER, 2026-11-14: the seconds families' compact rows, design change (d), get the speed build's shared walk); `None` reads
/// `OuterValue::kept` (the minutes families, unchanged). Element `j` equals [`run_with`] on `ctx` with `out = outs[j]` and the
/// same `kept_of`, by the same argument as [`run_all`] (the walk never reads its sink; the kept set is the walk's input).
///
/// # Errors
/// A context the accumulator refuses (more columns than `MAXC`).
pub fn run_all_kept(ctx: &Ctx<'_>, outs: &[&[Outcome]], values: &[OuterValue], kept_of: Option<KeptOf<'_>>, threads: usize, chunk: usize) -> Result<Vec<Out>, String> {
    if ctx.cols == 0 || ctx.cols > MAXC {
        return Err(format!("{} columns", ctx.cols));
    }
    let next = AtomicUsize::new(0);
    let total = values.len() + 1;
    let shards: Mutex<Vec<ShardN>> = Mutex::new(Vec::new());
    let n = ctx.rows.len();
    let nt = outs.len();
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                let mut sinks = Vec::with_capacity(nt);
                for o in outs {
                    let Ok(acc) = SweepAcc::new(ctx.lay, o, ctx.cols, ctx.sessions) else { return };
                    sinks.push(HostSink {
                        acc,
                        lists: bounded_lists(ctx),
                        views: ctx.views,
                        core: ctx.core,
                        base: Code::default(),
                        hist_acc: Hist::new(),
                        hist_key: Hist::new(),
                        ne: [0; 4],
                        tally: Vec::new(),
                    big: Vec::new(),
                    });
                }
                let mut eng = LineEngine::new(ctx.sessions, ctx.core);
                let mut sh = ShardN { lists: Vec::new(), walk: WalkStats::default(), hist_acc: Vec::new(), hist_key: Vec::new(), ne: Vec::new(), walked: 0, empty: 0, refused: vec![Vec::new(); nt] };
                let mut buf: Vec<Ev> = Vec::with_capacity(chunk.max(1));
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= total {
                        break;
                    }
                    let v = if i == 0 { None } else { values.get(i - 1) };
                    if v.is_some_and(|x| x.empty) {
                        sh.empty += 1;
                        continue;
                    }
                    let Some(base) = base_code(ctx, v) else {
                        for r in &mut sh.refused {
                            r.push(format!("code overflow at value {i}"));
                        }
                        continue;
                    };
                    let mut kept = match (v, kept_of) {
                        (None, _) => Bits::full(n),
                        (Some(_), Some(f)) => f(i - 1),
                        (Some(x), None) => x.kept.clone(),
                    };
                    if let Some(f) = ctx.filter {
                        kept.and_with(f);
                    }
                    for sink in &mut sinks {
                        sink.base = base;
                    }
                    buf.clear();
                    let mut fan = Fan { ev: std::mem::take(&mut buf), chunk: chunk.max(1), sinks: &mut sinks, err: vec![None; nt] };
                    // The fan-out never refuses, so the walk always runs to its end.
                    let _ = eng.walk(ctx.rows, kept.ones(), &mut fan, &mut sh.walk);
                    fan.flush();
                    for (e, r) in fan.err.iter().zip(sh.refused.iter_mut()) {
                        if let Some(e) = e {
                            r.push(format!("value {i}: {e}"));
                        }
                    }
                    buf = fan.ev;
                    sh.walked += 1;
                }
                for mut sink in sinks {
                    for (s, &k) in sink.tally.iter().enumerate() {
                        sink.hist_key.add_n(s as u64, k);
                    }
                    for &s in &sink.big {
                        sink.hist_key.add(s);
                    }
                    sh.lists.push(sink.lists);
                    sh.hist_acc.push(sink.hist_acc);
                    sh.hist_key.push(sink.hist_key);
                    sh.ne.push(sink.ne);
                }
                if let Ok(mut g) = shards.lock() {
                    g.push(sh);
                }
            });
        }
    });
    let shards = shards.into_inner().map_err(|_| "a worker panicked".to_string())?;
    let mut res = Vec::with_capacity(nt);
    for j in 0..nt {
        let mut out = Out {
            lists: bounded_lists(ctx),
            walk: WalkStats::default(),
            hist_acc: Hist::new(),
            hist_key: Hist::new(),
            not_eligible: [0; 4],
            values_walked: 0,
            values_empty: 0,
            refused: Vec::new(),
        };
        for sh in &shards {
            if let Some(ls) = sh.lists.get(j) {
                for (a, b) in out.lists.iter_mut().zip(ls) {
                    a.merge(b);
                }
            }
            out.walk.merge(&sh.walk);
            if let Some(h) = sh.hist_acc.get(j) {
                out.hist_acc.merge(h);
            }
            if let Some(h) = sh.hist_key.get(j) {
                out.hist_key.merge(h);
            }
            if let Some(ne) = sh.ne.get(j) {
                for (a, b) in out.not_eligible.iter_mut().zip(ne) {
                    *a += b;
                }
            }
            out.values_walked += sh.walked;
            out.values_empty += sh.empty;
            if let Some(r) = sh.refused.get(j) {
                out.refused.extend(r.iter().cloned());
            }
        }
        res.push(out);
    }
    // A refusal (never in a search run: any refusal aborts the context) stops `run`'s walk for that N where it happens, so its
    // walk counts are partial there; the shared walk always runs to its end. To stay equal to `run` in every case, the N are then
    // swept again one by one with `run` itself.
    if res.iter().any(|o| !o.refused.is_empty()) {
        return outs.iter().map(|o| run_with(&Ctx { out: o, ..*ctx }, values, kept_of, threads)).collect();
    }
    Ok(res)
}
