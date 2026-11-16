//! The RUN driver of the SECONDS families (3 / 5 / 10 / 15 / 30 s; SECONDS BUILDER, main session 5b507203, 2026-11-14). The same
//! pipeline as [`crate::search::run_family`], context by context (price every candidate row at every N and level, sweep S0 + S1 at
//! every N, merge, re-walk every listed entry over the whole period into `common_wf`'s own Lists, picks, verdicts and words from
//! `common_wf`, the random-entry twin, the DB rows), with design change (d) (MAIN 2026-11-13 20:56 IST, SIGNED) so that 3 s fits
//! under the RUN row `rss_cap_mb`:
//!
//! - the family is the COMPACT one of [`crate::sec`] (about 40 + `stride` bytes a row, never the 3 KB `Readings`);
//! - pricing runs session by session in CHUNKS (RUN row `sec_price_chunk_sessions`), each chunk's rows written at once, in order, to
//!   the OUT partition (streamed, hashed while written, [`Db::adopt_bin`]) and to one CELLS working file per N (60 bytes a row:
//!   net, run-down, settlement second and flags per level), so no context table of every (row, N, level) is ever held;
//! - the sweep at N reads that N's learning outcomes from its CELLS file (one N at a time) and makes each value's kept set when it
//!   is walked ([`crate::sweep::run_with`]);
//! - the re-walk, the picks and the twin read the CELLS file of ONE N at a time, the entries grouped by N; the report then prints
//!   in the order and the words of the minutes driver.
//!
//! The minutes families never enter this file (byte-identical to binary k: proved on two full contexts, CHANGES-sec.md).

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use common_fill::{Dial, US_S};
use common_wf::views::{Role, Ticket, Tickets, crit_val};
use common_wf::{Acc, Code as WfCode, Partition, PickWin, ReadSet, Trade, Verdict, ViewId};
use sniper_kernel::code::Code;
use sniper_kernel::line::Row;
use wf_sweep::acc::{Layout, Outcome};
use wf_sweep::lists::Lists;

use crate::cands::{EMA_ROWS, OuterValue, ST_ROWS};
use crate::db::Db;
use crate::esn1::{Inputs, day_tape};
use crate::price::{self, Cfg, Plan, Refusal, flag};
use crate::rows::{ExitKind, ExitSet};
use crate::search::{Combo, CtxOut, Decoder, RunSpec, Slice, combo_rows, compact, pct_of, test_figures};
use crate::sec::{self, SecFamily, SecSpec};
use crate::sha::{Sha256, hex};
use crate::wfio;

/// Bytes of one (row, N) cell record: net (i32) x 5, run-down (u16) x 5, settlement second (u32) x 5, flags (u16) x 5.
pub const CELL_B: usize = 60;
const NCOL: usize = 5;

/// The CELLS working file of grid N index `ni` (rewritten by every context: a working buffer, never a result; RUN row
/// `sec_cells_buffer`).
#[must_use]
pub fn cells_path(dir: &Path, ni: usize) -> PathBuf {
    dir.join(format!("CELLS-N{ni:02}.bin"))
}

/// One (row, N) cell, decoded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    /// Net per level, paisa.
    pub net: [i64; NCOL],
    /// Run-down per level, bp.
    pub rd: [u16; NCOL],
    /// Settlement second of the IST day per level.
    pub close: [u32; NCOL],
    /// Flags per level.
    pub fl: [u16; NCOL],
}

fn cell_of(b: &[u8]) -> Cell {
    let g = |i: usize| b.get(i).copied().unwrap_or(0);
    let mut c = Cell::default();
    for (l, x) in c.net.iter_mut().enumerate() {
        *x = i64::from(i32::from_le_bytes([g(4 * l), g(4 * l + 1), g(4 * l + 2), g(4 * l + 3)]));
    }
    for (l, x) in c.rd.iter_mut().enumerate() {
        *x = u16::from_le_bytes([g(20 + 2 * l), g(21 + 2 * l)]);
    }
    for (l, x) in c.close.iter_mut().enumerate() {
        *x = u32::from_le_bytes([g(30 + 4 * l), g(31 + 4 * l), g(32 + 4 * l), g(33 + 4 * l)]);
    }
    for (l, x) in c.fl.iter_mut().enumerate() {
        *x = u16::from_le_bytes([g(50 + 2 * l), g(51 + 2 * l)]);
    }
    c
}

fn cell_bytes(cells: &[(i64, u16, u32, u16)], ni: usize) -> Result<[u8; CELL_B], String> {
    let mut b = [0_u8; CELL_B];
    for l in 0..NCOL {
        let (net, rd, close, fl) = cells.get(ni * NCOL + l).copied().unwrap_or_default();
        let n32 = i32::try_from(net).map_err(|_| "a net outside i32 (refused; the OUT store holds i32)")?;
        let mut put = |o: usize, v: &[u8]| -> Result<(), String> {
            b.get_mut(o..o + v.len()).ok_or("cell layout")?.copy_from_slice(v);
            Ok(())
        };
        put(4 * l, &n32.to_le_bytes())?;
        put(20 + 2 * l, &rd.to_le_bytes())?;
        put(30 + 4 * l, &close.to_le_bytes())?;
        put(50 + 2 * l, &fl.to_le_bytes())?;
    }
    Ok(b)
}

/// Reads the first `m` cells of one N's file, in order.
fn read_cells(path: &Path, m: usize, mut f: impl FnMut(usize, Cell) -> Result<(), String>) -> Result<(), String> {
    let mut r = BufReader::with_capacity(4 << 20, File::open(path).map_err(|e| format!("{}: {e}", path.display()))?);
    let mut b = [0_u8; CELL_B];
    for j in 0..m {
        r.read_exact(&mut b).map_err(|e| format!("{} record {j}: {e}", path.display()))?;
        f(j, cell_of(&b))?;
    }
    Ok(())
}

/// One N's cells of every priced row (`CELL_B` bytes a row, decoded on access).
struct NCells {
    bytes: Vec<u8>,
}

impl NCells {
    fn load(path: &Path, m: usize) -> Result<Self, String> {
        let mut bytes = vec![0_u8; m * CELL_B];
        File::open(path).and_then(|mut f| f.read_exact(&mut bytes)).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self { bytes })
    }

    fn get(&self, j: usize) -> Option<Cell> {
        self.bytes.get(j * CELL_B..(j + 1) * CELL_B).map(cell_of)
    }
}

/// The signal exit anchor on the compact rows (`cands::signal_exit`'s rule, from the Supertrend flag byte and the EMA sign byte).
fn signal_exit(fam: &SecFamily, i: usize, long: bool, exit: &ExitSet) -> Option<(i64, bool)> {
    let c = fam.fix.get(i)?;
    let against = |x: usize| -> Option<bool> {
        match exit.kind {
            // SNIPER s20 (SECONDS PORTER): XC's Supertrend part is EXACTLY the XS rule of the same setting (xc's cands::signal_exit).
            ExitKind::Supertrend(p, f) | ExitKind::SupertrendOrClock(p, f, _) => {
                let j = ST_ROWS.iter().position(|&r| r == (p, f))?;
                let b = fam.st(x, j);
                (b & 1 != 0).then_some((b & 2 != 0) != long)
            }
            ExitKind::Ema(p) => {
                let j = EMA_ROWS.iter().position(|&q| q == p)?;
                match fam.ema_sign(x, j) {
                    0 => None,
                    1 => Some(!long),
                    2 => Some(long),
                    _ => Some(false),
                }
            }
            _ => None,
        }
    };
    let immediate = matches!(exit.kind, ExitKind::Supertrend(..) | ExitKind::SupertrendOrClock(..)) && against(i) == Some(true);
    let mut j = i + 1;
    while let Some(x) = fam.fix.get(j) {
        if x.sess != c.sess {
            return None;
        }
        if immediate || against(j) == Some(true) {
            return Some((x.end, immediate));
        }
        j += 1;
    }
    None
}

/// S7's XE context filter on the compact rows (`cands::xe_with`).
fn xe_with(fam: &SecFamily, i: usize, long: bool, p: u16) -> bool {
    let Some(j) = EMA_ROWS.iter().position(|&q| q == p) else { return false };
    match fam.ema_sign(i, j) {
        1 => long,
        2 => !long,
        _ => false,
    }
}

/// The plan of row `i` under `exit` (`esn1::plan_of`'s rule).
fn plan_of(fam: &SecFamily, i: usize, long: bool, exit: &ExitSet, sq: i64) -> Result<Plan, &'static str> {
    let clamp = |p: Plan| match p {
        Plan::Signal { anchor, .. } if anchor >= sq => Plan::Clock,
        p => p,
    };
    match exit.kind {
        ExitKind::Supertrend(..) | ExitKind::Ema(_) => Ok(clamp(signal_exit(fam, i, long, exit).map_or(Plan::Clock, |(end, immediate)| Plan::Signal { anchor: end * US_S, immediate }))),
        ExitKind::Clock => Ok(Plan::Clock),
        ExitKind::GoPositive(t) => Ok(Plan::Timer { secs: t }),
        // SNIPER s20 (SECONDS PORTER; xc's esn1::plan_of, word for word): the XS flip anchor of the same setting (no flip in the
        // session, or a flip at or after the square-off: the square-off is the signal part) and the plain clock `secs` after E0.
        ExitKind::SupertrendOrClock(_, _, secs) => Ok(match signal_exit(fam, i, long, exit) {
            Some((end, immediate)) if end * US_S < sq => Plan::ClockOrSignal { secs, anchor: end * US_S, immediate },
            _ => Plan::ClockOrSignal { secs, anchor: sq, immediate: false },
        }),
        ExitKind::SmallStop(x) => {
            let atr = fam.fix.get(i).map(|f| f.atr).filter(|&a| a != i64::MIN).ok_or("atr_unready")?;
            let d = (i128::from(atr) * i128::from(x) / 100 / common_feat::S).max(1);
            Ok(Plan::Stop { x: i64::try_from(d).map_err(|_| "atr_overflow")? })
        }
    }
}

/// SNIPER s16 trade rows: per level (units entered, entry value, closing value), paisa.
type LevelQv = [(u64, i64, i64); NCOL];
/// One re-walked entry: its combo, its rows and its trades.
type Walked = (Combo, Vec<usize>, Vec<Trade>);
/// One pick: its code, its combo, the test window of its twin (a headline view of an honest row).
type PickAt = ([u64; 4], Combo, Option<(i32, i32)>);

/// One context's priced table, its big parts on disk.
struct Priced2 {
    /// Priced row -> family row.
    kidx: Vec<u32>,
    /// The entry block's first second (epoch, `entry_first / US_S`) per priced row.
    entry_s: Vec<u32>,
    /// SECONDS PORTER (SNIPER s16): the size reference (paisa) per priced row; the sized count at any N is `size::size` of it
    /// (the pricing's own call, exact), so no per-(row, N) count is held.
    refpx: Vec<i64>,
    nl: usize,
    refusals: BTreeMap<String, u64>,
    rewalks: u64,
    price_s: f64,
    prot_rows: usize,
    clip_rows: usize,
    osha: String,
}

/// One family row priced at every N and level on its session's tape, in its compact form (the pricing worker's own body; the
/// SECONDS PORTER's SN6 re-pricing of the headline trades calls the same function, so a re-priced row is the priced row).
#[allow(clippy::too_many_arguments, reason = "one row's pricing inputs")]
fn price_row(fam: &SecFamily, cfg: &Cfg, exit: &ExitSet, long: bool, slots: &[i64], day: &price::Day, sday: i32, i: usize, origin: i64, xe: Option<u16>) -> Result<crate::search::RowOut, String> {
    let c = fam.fix.get(i).ok_or("row")?;
    if xe.is_some_and(|p| !xe_with(fam, i, long, p)) {
        return Err("xe_filter".to_string());
    }
    let sq = price::at_sod(sday, cfg.square_off_sod);
    plan_of(fam, i, long, exit, sq)
        .map_err(str::to_string)
        .and_then(|plan| {
            price::price(day, cfg, c.end * US_S, long, plan, slots).map_err(|e| match e {
                Refusal::Fill(x) => format!("fill: {x}"),
                Refusal::Charges(x) => format!("charges: {x}"),
                other => format!("{other:?}"),
            })
        })
        .and_then(|p| compact(&p, sday, slots.len(), origin))
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines, reason = "one context's pricing inputs, the chunk loop in one place")]
fn price_sec(fam: &SecFamily, ctx: &feat_io::host::Ctx, cfg: &Cfg, exit: &ExitSet, long: bool, slots: &[i64], hull_to: i32, threads: usize, chunk: usize, db: &Db, out_name: &str, cells_dir: &Path, sess_mk: &[i32], sym: &str) -> Result<Priced2, String> {
    let xe = match exit.kind {
        ExitKind::Ema(p) => Some(p),
        _ => None,
    };
    let mut ranges: Vec<(u32, usize, usize)> = Vec::new();
    for (i, f) in fam.fix.iter().enumerate() {
        match ranges.last_mut() {
            Some(r) if r.0 == f.sess => r.2 = i + 1,
            _ => ranges.push((f.sess, i, i + 1)),
        }
    }
    let nn = slots.len();
    let origin = price::at_sod(common_feat::cal::parse_ymd("2018-01-01").ok_or("origin")?, 0);
    let ticks = Path::new(feat_io::defaults::TICKS);
    let tmp = db.tmp_path(out_name);
    let mut ow = BufWriter::with_capacity(8 << 20, File::create(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?);
    let mut osh = Sha256::new();
    std::fs::create_dir_all(cells_dir).map_err(|e| format!("{}: {e}", cells_dir.display()))?;
    let mut cw: Vec<BufWriter<File>> = (0..nn).map(|ni| File::create(cells_path(cells_dir, ni)).map(|f| BufWriter::with_capacity(1 << 20, f)).map_err(|e| e.to_string())).collect::<Result<_, _>>()?;
    let mut t = Priced2 { kidx: Vec::new(), entry_s: Vec::new(), refpx: Vec::new(), nl: 0, refusals: BTreeMap::new(), rewalks: 0, price_s: 0.0, prot_rows: 0, clip_rows: 0, osha: String::new() };
    let tp = Instant::now();
    let mut seen_test = false;
    for part in ranges.chunks(chunk.max(1)) {
        let next = AtomicUsize::new(0);
        let priced: Mutex<Vec<(usize, Result<crate::search::RowOut, String>)>> = Mutex::new(Vec::new());
        std::thread::scope(|s| {
            for _ in 0..threads.max(1).min(part.len()) {
                s.spawn(|| {
                    let mut local = Vec::new();
                    loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&(sess, a, b)) = part.get(k) else { break };
                        let Some(se) = ctx.sessions.get(sess as usize) else { continue };
                        let day = match day_tape(ticks, sym, se) {
                            Ok(Some(d)) => d,
                            Ok(None) => {
                                local.extend((a..b).map(|i| (i, Err("no_tick_member".to_string()))));
                                continue;
                            }
                            Err(e) => {
                                local.extend((a..b).map(|i| (i, Err(format!("tick_day: {e}")))));
                                continue;
                            }
                        };
                        for i in a..b {
                            if fam.fix.get(i).is_none() {
                                continue;
                            }
                            local.push((i, price_row(fam, cfg, exit, long, slots, &day, se.day, i, origin, xe)));
                        }
                    }
                    if let Ok(mut g) = priced.lock() {
                        g.extend(local);
                    }
                });
            }
        });
        let mut priced = priced.into_inner().map_err(|_| "a pricing worker panicked")?;
        priced.sort_by_key(|x| x.0);
        for (i, r) in priced {
            let p = match r {
                Err(e) => {
                    *t.refusals.entry(e.split(':').next().unwrap_or("?").to_string()).or_insert(0) += 1;
                    continue;
                }
                Ok(p) => p,
            };
            let f = fam.fix.get(i).ok_or("row")?;
            let mk = sess_mk.get(f.sess as usize).copied().ok_or("session month")?;
            if mk > hull_to {
                seen_test = true;
            } else if seen_test {
                return Err("a learning row after a test row: the learning rows are not a prefix".to_string());
            } else {
                t.nl += 1;
            }
            ow.write_all(&p.rec).map_err(|e| e.to_string())?;
            osh.update(&p.rec);
            let (mut pr, mut cl) = (false, false);
            for (ni, w) in cw.iter_mut().enumerate() {
                w.write_all(&cell_bytes(&p.cells, ni)?).map_err(|e| e.to_string())?;
            }
            for x in &p.cells {
                pr |= x.3 & flag::PROTECTED != 0;
                cl |= x.3 & flag::CLIPPED != 0;
            }
            t.prot_rows += usize::from(pr);
            t.clip_rows += usize::from(cl);
            t.rewalks += u64::from(p.rewalks);
            t.refpx.push(p.refpx);
            t.entry_s.push(u32::try_from(p.entry_first / US_S).map_err(|_| "an entry second outside u32")?);
            t.kidx.push(u32::try_from(i).map_err(|_| "row index")?);
        }
    }
    for w in cw {
        let f = w.into_inner().map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    let f = ow.into_inner().map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())?;
    t.osha = db.adopt_bin(out_name, &hex(&osh.finish()))?;
    t.price_s = tp.elapsed().as_secs_f64();
    Ok(t)
}

/// SECONDS PORTER (SNIPER s16 trade rows): re-prices the priced rows `js` (indices into the context's priced rows) with
/// [`price_row`], one tape read per session, over `threads` workers, and returns per row the (units entered, entry value, closing
/// value) of every level at grid index `ni`. Fail closed: a row that does not price again, or whose cells at `ni` differ from its
/// CELLS record in any level (net, run-down, settlement second, flags), refuses the context, named.
#[allow(clippy::too_many_arguments, reason = "the pricing inputs of one context")]
fn reprice(fam: &SecFamily, ctx: &feat_io::host::Ctx, cfg: &Cfg, exit: &ExitSet, long: bool, slots: &[i64], kidx: &[u32], js: &BTreeSet<usize>, threads: usize, nc: &NCells, ni: usize, sym: &str) -> Result<BTreeMap<usize, LevelQv>, String> {
    let xe = match exit.kind {
        ExitKind::Ema(p) => Some(p),
        _ => None,
    };
    let origin = price::at_sod(common_feat::cal::parse_ymd("2018-01-01").ok_or("origin")?, 0);
    let ticks = Path::new(feat_io::defaults::TICKS);
    let mut by_sess: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
    for &j in js {
        let i = *kidx.get(j).ok_or("kidx")? as usize;
        by_sess.entry(fam.fix.get(i).ok_or("row")?.sess).or_default().push((j, i));
    }
    let work: Vec<(u32, Vec<(usize, usize)>)> = by_sess.into_iter().collect();
    let next = AtomicUsize::new(0);
    let got: Mutex<Vec<(usize, Result<crate::search::RowOut, String>)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads.max(1).min(work.len()) {
            s.spawn(|| {
                let mut local = Vec::new();
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some((sess, list)) = work.get(k) else { break };
                    let day = ctx.sessions.get(*sess as usize).ok_or_else(|| "session".to_string()).and_then(|se| day_tape(ticks, sym, se).and_then(|d| d.ok_or_else(|| "no_tick_member".to_string())).map(|d| (d, se.day)));
                    for &(j, i) in list {
                        local.push((j, day.as_ref().map_err(Clone::clone).and_then(|(d, sday)| price_row(fam, cfg, exit, long, slots, d, *sday, i, origin, xe))));
                    }
                }
                if let Ok(mut g) = got.lock() {
                    g.extend(local);
                }
            });
        }
    });
    let mut out = BTreeMap::new();
    for (j, r) in got.into_inner().map_err(|_| "a re-pricing worker panicked")? {
        let p = r.map_err(|e| format!("SN6 re-pricing: priced row {j} refused on re-pricing ({e}); named"))?;
        let c = nc.get(j).ok_or("SN6 re-pricing: a row outside the CELLS file")?;
        let mut q = [(0_u64, 0_i64, 0_i64); NCOL];
        for (l, qq) in q.iter_mut().enumerate() {
            let (net, rd, close, fl) = p.cells.get(ni * NCOL + l).copied().unwrap_or_default();
            if c.net.get(l) != Some(&net) || c.rd.get(l) != Some(&rd) || c.close.get(l) != Some(&close) || c.fl.get(l) != Some(&fl) {
                return Err(format!("SN6 re-pricing: priced row {j} level {l} at grid index {ni} differs from its CELLS record (net {net} rd {rd} close {close} flags {fl}); named"));
            }
            *qq = p.qv.get(ni * NCOL + l).copied().unwrap_or_default();
        }
        out.insert(j, q);
    }
    Ok(out)
}

/// The run's seconds family and what it needs, once per run.
struct Fam {
    fam: SecFamily,
    sess_mk: Vec<i32>,
}

/// Runs one seconds family (the `run` subcommand with a seconds `--tf`).
///
/// # Errors
/// A held gate, a refused input, a projected memory above the cap, or an I/O error.
#[allow(clippy::too_many_lines, reason = "the gate-7 pipeline of one family in one place (as search::run_family)")]
pub fn run_family_sec(inp: &Inputs, spec: &RunSpec) -> Result<Vec<String>, String> {
    let run = &inp.run;
    let mut lines = Vec::new();
    let gate = crate::monitor::launch_gate(run, spec.swap_base_mb, &spec.out, 8)?;
    lines.push(format!("{} gate ok={} hold={:?} pressure={:?} swap_used={:?} swap_free={:?} disk_free={:?}", crate::ist_now(), gate.ok, gate.hold, gate.pressure, gate.swap_used_mb, gate.swap_free_mb, gate.disk_free_gib));
    if !gate.ok {
        return Err(format!("launch gate HOLDS: {:?}", gate.hold));
    }
    lines.push(format!("{}; {}", crate::SEC_VERSION, crate::law_line()));
    let cap_mb: u64 = run.num("rss_cap_mb")?;
    let slice = spec.slice.as_deref().map(Slice::load).transpose()?;
    let db = Db::open(&spec.out.join("db"), crate::VERSION)?;
    let hb = spec.out.join("HEARTBEAT.tsv");
    let beat = |stage: &str, extra: String| {
        let _ = crate::monitor::heartbeat(&hb, &[("stage", stage.to_string()), ("rss_mb", format!("{:?}", crate::monitor::rss_mb())), ("detail", extra)]);
    };
    let t0 = Instant::now();
    let cont = common_feat::cal::parse_ymd("2018-09-07").ok_or("cont start")?;
    let ctx = feat_io::load_ctx(Path::new(feat_io::defaults::DATES), Path::new(feat_io::defaults::BHAV), cont)?;
    let family: Vec<String> = run.list::<String>("seconds_family")?;
    if !family.contains(&spec.tf) {
        return Err(format!("{}: not in the RUN row seconds_family {family:?}", spec.tf));
    }
    // (b) REQ 62: the H-* dials read the OTHER timeframes of the seconds family only (never a minute).
    let wants_htf = inp.dials.iter().any(|d| d.id.starts_with("H-") && d.status.starts_with("LIVE") && slice.as_ref().is_none_or(|s| s.has_dial(&d.id)));
    let lad = if wants_htf { Some(crate::cands::htf_ladders(&inp.dials)?) } else { None };
    let others: Vec<String> = family.iter().filter(|x| **x != spec.tf).cloned().collect();
    let (from, to) = spec.period.unwrap_or((cont, i32::MAX));
    let cut = run.hms("last_decision_before_ist")?;
    let sspec = SecSpec { tf: spec.tf.clone(), others: if wants_htf { others.clone() } else { Vec::new() }, from, to, threads: spec.threads, chunk: run.num("sec_read_chunk_sessions")? };
    // PORTER (secfd): the seven context dials, only in the ON mode and only when the walked values hold one of them (the minutes
    // driver's rule); their sources are read once here and each candidate's readings are made in the day loop. OFF: secf-a exactly.
    let wants_ctx = spec.context_dials
        && inp.dials.iter().any(|d| crate::ctxdials::IDS.contains(&d.id.as_str()) && d.status.starts_with("LIVE") && slice.as_ref().is_none_or(|s| s.has_dial(&d.id)));
    let tx = Instant::now();
    let prep = if wants_ctx { Some(crate::ctxdials::prepare(&ctx, Path::new(feat_io::defaults::STORE))?) } else { None };
    let prep_s = tx.elapsed().as_secs_f64();
    let fam = sec::build_family_ctx(&ctx, &sspec, Path::new(feat_io::defaults::TICKS), inp.instr.underlying(), &inp.dials, (&inp.core.0, &inp.core.1), cut, lad.as_ref(), prep.as_ref())?;
    let period_words = spec.period.map_or_else(|| "the whole period".to_string(), |(a, b)| format!("SMOKE PERIOD {}..{} only (labelled: a part of the period, never a result)", common_feat::cal::ymd(a), common_feat::cal::ymd(b)));
    lines.push(format!(
        "readings: {} sessions read ({} without a member {:?}), {} corrected 1-s bars ({} price-only), halt-window volume-0 seconds dropped {} {:?} (common_feat repair 2), {} tick rows ({} refused, {} out of order); {} {} candles ({}), {} decision candidates, {} not entry {:?} (Session::entry_at at the candle END), K13 resets {}, K_tf {}; other timeframes {:?} candles {:?}; {:.1} s wall ({:.1} s compressing); {period_words}",
        fam.stats.days,
        fam.stats.days_no_member.len(),
        fam.stats.days_no_member.iter().take(5).collect::<Vec<_>>(),
        fam.stats.bars_1s,
        fam.stats.price_only,
        fam.stats.halt_dropped,
        fam.stats.halt_dropped_days,
        fam.stats.tick_rows,
        fam.stats.bad_rows,
        fam.stats.out_of_order,
        fam.stats.candles,
        spec.tf,
        fam.source,
        fam.len(),
        fam.not_entry,
        fam.refused,
        fam.resets.len(),
        fam.kmax,
        fam.htf_names,
        fam.stats.other_candles,
        fam.stats.wall_s,
        fam.stats.compress_s
    ));
    if prep.is_some() {
        lines.extend(fam.ctx_lines.iter().cloned());
        lines.push(format!("context dial readings made in the day loop (sources read in {prep_s:.1} s)"));
    }
    let rss_fam = crate::monitor::rss_mb();
    // macOS compresses idle pages, so `ps` RSS can read below the rows' own bytes: the HOLD reads the larger of the two.
    let fam_mb = rss_fam.unwrap_or(0).max(u64::try_from(fam.resident_bytes() >> 20).unwrap_or(u64::MAX));
    lines.push(format!(
        "compact rows (design change (d)): {} rows x ({} B fixed + {} B bytes) + {} sessions x {} B = {} MB resident; process RSS {:?} MB (cap {cap_mb} MB, RUN row rss_cap_mb); H-* reads refused across the K13 reset per other timeframe {:?}",
        fam.len(),
        std::mem::size_of::<sec::RowFix>(),
        fam.lay.stride,
        ctx.sessions.len(),
        fam.lay.sess_stride,
        fam.resident_bytes() >> 20,
        rss_fam,
        fam.htf_reset_refused
    ));
    if fam_mb > cap_mb {
        return Err(format!("{fam_mb} MB (process RSS {rss_fam:?} MB, rows {} MB) after the family build is above the cap {cap_mb} MB (RUN row rss_cap_mb): no context started", fam.resident_bytes() >> 20));
    }
    beat("readings", format!("{} candidates", fam.len()));
    let sess_mk: Vec<i32> = ctx.sessions.iter().map(|s| {
        let (y, m, _) = common_feat::cal::civil(s.day);
        y * 12 + i32::try_from(m).unwrap_or(1) - 1
    }).collect();
    let fm = Fam { fam, sess_mk };
    let cal = wfio::calendar(&ctx.sessions)?;
    let (hull_from, hull_to) = inp.windows.hull();
    let levels: Vec<u16> = run.list("levels_bp")?;
    let cfg = Cfg {
        dial: Dial::new(&levels).map_err(|e| e.to_string())?,
        lev: (run.num("lev_num")?, run.num("lev_den")?),
        protection_bp: run.num("protection_bp")?,
        limit_tick: run.num("limit_tick_paisa")?,
        square_off_sod: run.hms("square_off_ist")?,
        cut_sod: cut,
        qty_rule: inp.qty_rule,
        account: run.num("account_paisa")?,
    };
    let account: i64 = run.num("account_paisa")?;
    let n_grid: Vec<i64> = run.list("n_grid")?;
    let proxy_n: i64 = run.num("proxy_n")?;
    let slots: Vec<i64> = n_grid.iter().map(|n| account / n.max(&1)).collect();
    let wins: Vec<(String, i32, i32)> = inp.windows.rows.iter().map(|r| (r.0.clone(), r.1, r.2)).collect();
    let lay = Layout::new(&fm.sess_mk, (hull_from, hull_to), &wins).map_err(|e| e.to_string())?;
    let twin_draws: u32 = run.num("twin_draws")?;
    let twin_seed: u64 = run.num("twin_seed")?;
    let wf_data = PathBuf::from(run.get("windows_file")?).parent().map(Path::to_path_buf).ok_or("windows_file has no folder")?;
    // the seconds partition template: signal_source / data_labels name GDFL corrected seconds (SNIPER s13 (2))
    let part_tpl = crate::rows::read_with_sha(&spec.data.join("PART-SNIPER-SEC-v1.tsv").to_string_lossy())?;
    let labels = inp.words.standing(1, "printed by the tool").join(" | ");
    let mut total = CtxOut { lines: Vec::new(), searches: 0, wf_passed: 0, tt_passed: 0, both: 0, mismatches: 0 };
    let n_ctx = spec.sides.len() * spec.exits.len();
    let mut done_ctx = 0_usize;
    let tc = Instant::now();
    if let Some(l) = &spec.n_sweep {
        lines.push(format!("N swept: {l:?} of the grid {n_grid:?} (every N priced and stored; a pick's N is searched over the swept N only; named)"));
    }
    for l in &lines {
        println!("{l}");
    }
    let cells_dir = spec.out.join("cells");
    for &long in &spec.sides {
        for exit_id in &spec.exits {
            let exit = inp.exits.iter().find(|e| &e.id == exit_id).ok_or_else(|| format!("exit set {exit_id} not in the exits file"))?;
            // the context's writes: the OUT store (record width a row) + the CELLS buffer (rewritten) + the index, in GiB
            let n = fm.fam.len() as u64;
            let writes_gib = (n * (price::record_width(n_grid.len(), NCOL) as u64 + (CELL_B * n_grid.len()) as u64)) >> 30;
            let g = crate::monitor::launch_gate(run, spec.swap_base_mb, &spec.out, writes_gib + 1)?;
            let proj = projected_mb(&fm.fam, spec.threads, n_grid.len(), run.num("sec_price_chunk_sessions")?);
            if !g.ok || fam_mb + proj > cap_mb {
                total.lines.push(format!(
                    "{} HOLD before {} {}: {:?}; projected peak {} + {proj} MB against the cap {cap_mb} MB (named; the rest of the run waits for MAIN)",
                    crate::ist_now(),
                    if long { "LONG" } else { "SHORT" },
                    exit.id,
                    g.hold,
                    fam_mb
                ));
                break;
            }
            let c = context(inp, spec, &db, &fm, &ctx, &cfg, exit, long, &slots, &n_grid, proxy_n, &lay, &cal, &wf_data, &part_tpl.0, slice.as_ref(), (twin_seed, twin_draws), &labels, (hull_from, hull_to), &cells_dir, &period_words)?;
            total.searches += c.searches;
            total.wf_passed += c.wf_passed;
            total.tt_passed += c.tt_passed;
            total.both += c.both;
            total.mismatches += c.mismatches;
            for l in &c.lines {
                println!("{l}");
            }
            done_ctx += 1;
            let per = tc.elapsed().as_secs() / (done_ctx as u64).max(1);
            let left = per * (n_ctx.saturating_sub(done_ctx) as u64);
            println!("{} context {done_ctx} of {n_ctx} done; {per} s a context so far; about {} min left at this pace (ESTIMATE); process RSS {:?} MB (cap {cap_mb})", crate::ist_now(), left / 60, crate::monitor::rss_mb());
            beat("context", format!("{} {} done ({done_ctx} of {n_ctx}; about {} min left, ESTIMATE)", if long { "LONG" } else { "SHORT" }, exit.id, left / 60));
        }
    }
    let mut out_lines = Vec::new();
    out_lines.extend(total.lines);
    out_lines.push(format!(
        "SUMMARY {}: {} searches; walk-forward picks PASSED {} of {}; train/test picks PASSED {} of {}; both in the same search (REQ 73d) {} of {}; re-walk key mismatches {}; {period_words}",
        spec.tf, total.searches, total.wf_passed, total.searches, total.tt_passed, total.searches, total.both, total.searches, total.mismatches
    ));
    let mut sn1 = vec![
        vec![spec.run_id.clone(), "version".to_string(), crate::SEC_VERSION.to_string(), labels.clone()],
        vec![spec.run_id.clone(), "laws".to_string(), crate::law_line(), labels.clone()],
        vec![spec.run_id.clone(), "threads".to_string(), spec.threads.to_string(), labels.clone()],
        vec![spec.run_id.clone(), "gate".to_string(), format!("{gate:?}").replace('\n', " "), labels.clone()],
        vec![spec.run_id.clone(), "bar_source".to_string(), inp.words.line("BARSRC", &[]), labels.clone()],
        vec![spec.run_id.clone(), "stream_source".to_string(), fm.fam.source.clone(), labels.clone()],
        vec![spec.run_id.clone(), "period".to_string(), period_words.clone(), labels.clone()],
        vec![spec.run_id.clone(), "seconds_family_htf".to_string(), fm.fam.htf_names.join(","), labels.clone()],
        vec![spec.run_id.clone(), "rows_1s_candles_candidates".to_string(), format!("{} {} {}", fm.fam.stats.bars_1s, fm.fam.stats.candles, fm.fam.len()), labels.clone()],
        vec![spec.run_id.clone(), "halt_window_volume0_seconds_dropped".to_string(), format!("{} {:?} (common_feat repair 2: plan 2.1 HALTS, K11)", fm.fam.stats.halt_dropped, fm.fam.stats.halt_dropped_days), labels.clone()],
        vec![spec.run_id.clone(), "slice".to_string(), slice.as_ref().map_or_else(|| "ALL wired values".to_string(), |s| format!("{} sha256 {}", s.name, s.sha)), labels.clone()],
        vec![spec.run_id.clone(), "headline_partition_provenance".to_string(), "none (plan 6 item 7, D-S7)".to_string(), labels.clone()],
        vec![spec.run_id.clone(), "wall_s".to_string(), format!("{:.1}", t0.elapsed().as_secs_f64()), labels.clone()],
    ];
    for (f, s) in &inp.shas {
        sn1.push(vec![spec.run_id.clone(), format!("sha256 {f}"), s.clone(), labels.clone()]);
    }
    sn1.push(vec![spec.run_id.clone(), "sha256 PART-SNIPER-SEC-v1.tsv".to_string(), part_tpl.1.clone(), labels.clone()]);
    sn1.push(vec![spec.run_id.clone(), "qty_rule".to_string(), format!("{} ({})", inp.qty_rule.id(), inp.qty_rule.label()), labels.clone()]);
    // PORTER (secfd): the ON mode's 2 rows (after qty_rule, so no row moves); none in the OFF mode.
    for (f, v) in &fm.fam.ctx_sn1 {
        sn1.push(vec![spec.run_id.clone(), f.clone(), v.clone(), labels.clone()]);
    }
    db.write(&format!("SN1-RUN-{}.tsv", spec.tf), &["run", "field", "value", "labels"], &sn1)?;
    let n = db.verify_stream()?;
    out_lines.push(format!("DB: {} tables audited, chain verified (streamed); run folder {}", n, spec.out.display()));
    for l in inp.words.standing(1, "printed by the tool") {
        out_lines.push(l);
    }
    out_lines.push(format!("{} done in {:.1} s; process RSS {:?} MB (cap {cap_mb})", crate::ist_now(), t0.elapsed().as_secs_f64(), crate::monitor::rss_mb()));
    beat("done", String::new());
    Ok(out_lines)
}

/// The projected extra resident MB of one context over the family (the largest phase): pricing chunk rows, the kernel rows,
/// one N's learning outcomes, per-thread kept sets and engine classes, one N's cells for the re-walk, the index build.
#[must_use]
pub fn projected_mb(fam: &SecFamily, threads: usize, nn: usize, chunk_sessions: usize) -> u64 {
    let n = fam.len() as u64;
    // kidx + entry second + the s16 size reference (8 B, SECONDS PORTER) + the kernel row, per priced row
    let base = n * (4 + 4 + 8 + std::mem::size_of::<Row>() as u64);
    let sweep = n * std::mem::size_of::<Outcome>() as u64 + threads as u64 * (n / 8 + 4 * n);
    // the re-walk holds one N's cells plus every listed entry's rows and trades (about 700 entries x at most 2,021 trades, measured on
    // the whole-period 30 s run) and one kept set
    let rewalk = n * CELL_B as u64 + (200 << 20) + n / 8;
    let index = 2 * 4 * 2_100 * (u64::from(fam.kmax) + 1);
    let chunk = chunk_sessions as u64 * (u64::from(fam.kmax) + 1) * (price::record_width(nn, NCOL) as u64 + 16 * (nn * NCOL) as u64 + 64);
    (base + sweep.max(rewalk).max(index).max(chunk)) >> 20
}

/// SECONDS PORTER: how many swept N one shared walk holds (fast-a's `run_all`, item 8): the N whose learning outcomes fit under
/// `cap_mb` beside the larger of the process RSS and the family's and the context's own row bytes, the per-thread kept sets and
/// engine classes (the seconds builder's model) and 256 MB of margin; at least 1, at most `nswept`. The group size changes no
/// output (element j of `run_all_kept` equals `run_with` at that N), only memory and speed.
fn sweep_group(fam: &SecFamily, nl: usize, m: usize, threads: usize, cap_mb: u64, nswept: usize) -> usize {
    let (nl, m, th) = (nl as u64, m as u64, threads.max(1) as u64);
    let rows_b = m * (4 + 4 + 8 + std::mem::size_of::<Row>() as u64) + u64::try_from(fam.resident_bytes()).unwrap_or(u64::MAX);
    let held_mb = (rows_b >> 20).max(crate::monitor::rss_mb().unwrap_or(0));
    let per_thread_mb = (th * (nl / 8 + 4 * nl)) >> 20;
    let per_n_b = nl * std::mem::size_of::<Outcome>() as u64 + th * (1 << 20);
    let free_b = cap_mb.saturating_sub(held_mb + per_thread_mb + 256) << 20;
    usize::try_from(free_b / per_n_b.max(1)).unwrap_or(usize::MAX).clamp(1, nswept.max(1))
}

/// One context: price (chunked, to disk), sweep at every N (one N's outcomes at a time, kept sets made when walked), merge, re-walk
/// grouped by N into common_wf, report, twin, DB rows. The lines and the DB tables are the minutes driver's (search::context).
#[allow(clippy::too_many_arguments, clippy::too_many_lines, reason = "one context's pipeline in one place")]
fn context(
    inp: &Inputs,
    spec: &RunSpec,
    db: &Db,
    fm: &Fam,
    ctx: &feat_io::host::Ctx,
    cfg: &Cfg,
    exit: &ExitSet,
    long: bool,
    slots: &[i64],
    n_grid: &[i64],
    proxy_n: i64,
    lay: &Layout,
    cal: &common_wf::Calendar,
    wf_data: &Path,
    part_tpl: &str,
    slice: Option<&Slice>,
    twin: (u64, u32),
    labels: &str,
    hull: (i32, i32),
    cells_dir: &Path,
    period_words: &str,
) -> Result<CtxOut, String> {
    let fam = &fm.fam;
    let side = if long { "LONG" } else { "SHORT" };
    let cname = format!("{}|{side}|{}", spec.tf, exit.id);
    let mut lines = vec![format!("=== {} context {cname} ===", crate::ist_now())];
    let run = &inp.run;
    let ncol = NCOL;
    let levels: Vec<u16> = run.list("levels_bp")?;
    let chunk: usize = run.num("sec_price_chunk_sessions")?;
    // --- pricing: every candidate row, every N, every level; OUT streamed, CELLS per N ---
    let t = price_sec(fam, ctx, cfg, exit, long, slots, hull.1, spec.threads, chunk, db, &format!("OUT-{}-{side}-{}.bin", spec.tf, exit.id), cells_dir, &fm.sess_mk, inp.instr.underlying())?;
    let m = t.kidx.len();
    let keys: Vec<(u32, u16)> = t.kidx.iter().filter_map(|&i| fam.fix.get(i as usize).map(|c| (c.sess, c.k))).collect();
    db.write_bin(&format!("IX-{}-{side}-{}.bin", spec.tf, exit.id), &crate::query::build_index(&keys, ctx.sessions.len(), fam.kmax))?;
    drop(keys);
    lines.push(format!(
        "pricing: {} rows priced ({} learning, {} test), {} N x {} levels each; refused {:?}; {:.1} s wall on {} threads (chunks of {chunk} sessions); S2 re-walks {}; Dhan market protection (REQ 70a) held units on {} rows at some (N, level); a block clipped by an order's stop on {} rows (beside); outcome rows {} B, sha {}; CELLS buffer {} MB; process RSS {:?} MB; {}",
        m,
        t.nl,
        m - t.nl,
        n_grid.len(),
        ncol,
        t.refusals,
        t.price_s,
        spec.threads,
        t.rewalks,
        t.prot_rows,
        t.clip_rows,
        price::record_width(n_grid.len(), ncol),
        t.osha,
        (m * CELL_B * n_grid.len()) >> 20,
        crate::monitor::rss_mb(),
        inp.qty_rule.label()
    ));
    // --- the lattice values (specs; kept sets made when walked) ---
    let krows = fam.kernel_rows(long, &t.kidx);
    let xe = match exit.kind {
        ExitKind::Ema(p) => Some(p),
        _ => None,
    };
    let (mut values, mut specs, wiring) = sec::values(fam, long, &inp.dials, xe);
    if let Some(s) = slice {
        let keep: Vec<bool> = values.iter().map(|v| inp.dials.get(v.dial).is_some_and(|d| s.keeps(&d.id, &v.label))).collect();
        let mut it = keep.iter();
        values.retain(|_| it.next().copied().unwrap_or(false));
        let mut it = keep.iter();
        specs.retain(|_| it.next().copied().unwrap_or(false));
    }
    let walked_values = values.iter().filter(|v| !v.empty).count() as u64;
    let core = crate::cands::core_of(if long { &inp.core.0 } else { &inp.core.1 });
    let pos = |id: &str| inp.dials.iter().position(|d| d.id == id).unwrap_or(0);
    let core_pos = [pos("M-RSI"), pos("M-CCI"), pos("M-WR")];
    for (p, n) in [(core_pos[0], core.nr), (core_pos[1], core.nc), (core_pos[2], core.nw)] {
        if wiring.radix.get(p).copied() != Some(u64::from(n)) {
            return Err(format!("core radix mismatch at dial {p}: {:?} vs {n}", wiring.radix.get(p)));
        }
    }
    let dec = Decoder::new(&wiring.radix, core_pos, &values, n_grid.len());
    let nw_code = dec.n_weight().ok_or("the N digit leaves 256 bits")?;
    let members = (1 + walked_values).saturating_mul(core.cells()).saturating_mul(n_grid.len() as u64);
    lines.push(format!(
        "lattice: {} outer values walked ({} EMPTY by S7) + S0, core {} cells, {} N: {} combos declared (grid LATTICE); slice {}",
        walked_values,
        values.len() as u64 - walked_values,
        core.cells(),
        n_grid.len(),
        members,
        slice.map_or_else(|| "ALL wired values".to_string(), |s| format!("{} ({})", s.name, s.sha.get(..16).unwrap_or(""))),
    ));
    let per_n = (1 + walked_values).saturating_mul(core.cells());
    lines.push(format!(
        "floor (D-S8: at least 10^9 structurally distinct combos per timeframe, side and exit set, at one N): {}",
        if per_n >= 1_000_000_000 { format!("met ({per_n} at each N)") } else { format!("NOT met by this walk ({per_n} at each N; a SLICE walks part of the lattice by MAIN's order, the full lattice is the same code without --slice)") }
    ));
    // --- the sweep at every N: that N's learning outcomes from its CELLS file, kept sets made when walked ---
    let nl = t.nl;
    let lrows = krows.get(..nl).unwrap_or(&[]);
    let kidx = &t.kidx;
    let outcomes = |ni: usize, upto: usize| -> Result<Vec<Outcome>, String> {
        let mut v = Vec::with_capacity(upto);
        read_cells(&cells_path(cells_dir, ni), upto, |j, c| {
            let i = *kidx.get(j).ok_or("kidx")? as usize;
            let f = fam.fix.get(i).ok_or("row")?;
            let mut o = Outcome { sess: f.sess, mk: fm.sess_mk.get(f.sess as usize).copied().unwrap_or(0), id: i as u64, ..Outcome::default() };
            for l in 0..ncol {
                if let (Some(n), Some(x)) = (o.net.get_mut(l), o.rd_bp.get_mut(l)) {
                    *n = c.net.get(l).copied().unwrap_or(0);
                    *x = c.rd.get(l).copied().unwrap_or(0);
                }
            }
            v.push(o);
            Ok(())
        })?;
        Ok(v)
    };
    let top: u64 = run.num("stage2_top")?;
    let mut merged: Vec<Lists> = Vec::new();
    for v in &inp.views {
        for _ in 0..ncol {
            merged.push(Lists::new(v.k.unwrap_or(top)));
        }
    }
    let mut sn4 = Vec::new();
    let mut acc_hist = sniper_kernel::steps::Hist::new();
    let mut cells_offered = 0_u64;
    let mut key_hist = sniper_kernel::steps::Hist::new();
    let ts = Instant::now();
    let kept_of = |vi: usize| specs.get(vi).map_or_else(|| sniper_kernel::bits::Bits::new(nl), |s| fam.kept(s, kidx, nl));
    let mut sweep_rss = None;
    // SECONDS PORTER (fast-a's shared walk, CHANGES-fast item 8): the swept N in GROUPS, each group in ONE walk of the lattice
    // (crate::sweep::run_all_kept: each outer value's kept set made and walked once, its events replayed into one sink per N of the
    // group); element j equals crate::sweep::run_with at the group's j-th N, so the group size changes no output. The group holds
    // its N's learning outcomes at once: as many N as fit under rss_cap_mb beside the family and the context's rows (at least 1).
    let swept: Vec<(usize, i64)> = n_grid.iter().copied().enumerate().filter(|(_, n)| !spec.n_sweep.as_ref().is_some_and(|l| !l.contains(n))).collect();
    let group = sweep_group(fam, nl, m, spec.threads, run.num("rss_cap_mb")?, swept.len());
    for grp in swept.chunks(group.max(1)) {
        let all = {
            let outs: Vec<Vec<Outcome>> = grp.iter().map(|&(ni, _)| outcomes(ni, nl)).collect::<Result<_, _>>()?;
            let refs: Vec<&[Outcome]> = outs.iter().map(Vec::as_slice).collect();
            let sctx = crate::sweep::Ctx { rows: lrows, out: refs.first().copied().unwrap_or(&[]), lay, views: &inp.views, cols: ncol, core, sessions: ctx.sessions.len(), top, radix: &wiring.radix, core_pos, filter: None };
            let all = crate::sweep::run_all_kept(&sctx, &refs, &values, Some(&kept_of), spec.threads, crate::sweep::CHUNK)?;
            sweep_rss = sweep_rss.max(crate::monitor::rss_mb());
            all
        };
        if all.len() != grp.len() {
            return Err(format!("{cname}: the shared walk returned {} tables for {} N", all.len(), grp.len()));
        }
        for (&(ni, n), res) in grp.iter().zip(all) {
        if !res.refused.is_empty() {
            return Err(format!("{cname} N={n}: the sweep refused {:?}", res.refused.iter().take(3).collect::<Vec<_>>()));
        }
        let shift = nw_code.mul_add(ni as u64, 0).ok_or("N digit overflow")?;
        for (mg, l) in merged.iter_mut().zip(&res.lists) {
            mg.merge(&l.shifted(shift.0).ok_or("shifted code overflow")?);
        }
        acc_hist.merge(&res.hist_acc);
        cells_offered += res.walk.cells_offered;
        key_hist.merge(&res.hist_key);
        sn4.push(vec![
            spec.run_id.clone(),
            cname.clone(),
            "S0+S1".to_string(),
            n.to_string(),
            res.values_walked.to_string(),
            res.values_empty.to_string(),
            res.walk.cells.to_string(),
            res.walk.runs_offered.to_string(),
            res.walk.empty_cells.to_string(),
            format!("{:?}", res.not_eligible),
            format!("{:?}|{:?}|{:?}|{:?}", res.walk.hist_enter.summary(), res.walk.hist_line.summary(), res.hist_acc.summary(), res.hist_key.summary()),
            res.walk.inserts.to_string(),
            res.walk.removes.to_string(),
            format!("{:?}", wiring.not_wired),
            inp.qty_rule.id().to_string(),
            labels.to_string(),
        ]);
        if n == proxy_n {
            lines.push(format!(
                "sweep N={n}: {} values + S0, {} core cells, {} offered runs, {} empty cells, inserts {} removes {}; not eligible (no trade, part not positive, below floor, passes E2) {:?}; steps p50/p99/max enter {:?} line {:?} acc {:?} key {:?}",
                res.values_walked.saturating_sub(1),
                res.walk.cells,
                res.walk.runs_offered,
                res.walk.empty_cells,
                res.walk.inserts,
                res.walk.removes,
                res.not_eligible,
                res.walk.hist_enter.summary(),
                res.walk.hist_line.summary(),
                res.hist_acc.summary(),
                res.hist_key.summary()
            ));
        }
        }
    }
    let sweep_s = ts.elapsed().as_secs_f64();
    lines.push(format!("sweep at the swept N ({} of {}): {:.1} s wall on {} threads; accumulator event p50/p99/max {:?}; key read {:?}; process RSS at most {:?} MB while sweeping; shared walk (fast-a item 8) {group} N a walk", spec.n_sweep.as_ref().map_or(n_grid.len(), Vec::len), n_grid.len(), sweep_s, spec.threads, acc_hist.summary(), key_hist.summary(), sweep_rss));
    // --- the HARD gate (plan 8.4) on the WF-learning rows at the proxy N, capped (RUN row sec_hard_gate_rows_max: 100x of every
    // seconds row would not fit the cap; named) ---
    let pn = n_grid.iter().position(|&n| n == proxy_n).unwrap_or(0);
    let wf_to = inp.windows.rows.first().map_or(hull.1, |r| r.2);
    let gate_max: usize = run.num("sec_hard_gate_rows_max")?;
    let pout = outcomes(pn, nl.min(gate_max.saturating_mul(4)))?;
    let base_pairs: Vec<(Row, Outcome)> = lrows.iter().zip(&pout).filter(|(_, o)| o.mk <= wf_to).take(gate_max).map(|(r, o)| (*r, *o)).collect();
    let base_rows: Vec<Row> = base_pairs.iter().map(|x| x.0).collect();
    let base_out: Vec<Outcome> = base_pairs.iter().map(|x| x.1).collect();
    drop(pout);
    let mut sig = Vec::new();
    let mut steps_rows = Vec::new();
    for times in [1_u16, 10, 100] {
        let (rt, src) = sniper_kernel::scale::replicate(&base_rows, times, 20_261_113);
        let ot: Vec<Outcome> = src.iter().enumerate().filter_map(|(j, &k)| base_out.get(k as usize).map(|o| Outcome { id: j as u64, ..*o })).collect();
        let gctx = crate::sweep::Ctx { rows: &rt, out: &ot, lay, views: &inp.views, cols: ncol, core, sessions: ctx.sessions.len(), top: 1000, radix: &wiring.radix, core_pos, filter: None };
        let g = crate::sweep::run(&gctx, &[], 1)?;
        let s = (g.walk.hist_enter.max(), g.walk.hist_line.max(), g.hist_acc.max(), g.hist_key.max());
        sig.push(s);
        steps_rows.push(vec![spec.run_id.clone(), cname.clone(), times.to_string(), rt.len().to_string(), format!("{s:?}"), format!("{:?}|{:?}|{:?}", g.walk.hist_enter.summary(), g.hist_acc.summary(), g.hist_key.summary()), labels.to_string()]);
    }
    let hard_ok = sig.windows(2).all(|w| w.first() == w.get(1));
    lines.push(format!("HARD gate (per-event step counts identical at 1x / 10x / 100x; S0 on the first {} WF-learning rows at N = {proxy_n}, capped at {gate_max} by the RUN row sec_hard_gate_rows_max, named): {} {sig:?}", base_rows.len(), if hard_ok { "PASS" } else { "FAIL" }));
    db.write(&format!("SN4-STEPS-{}-{side}-{}.tsv", spec.tf, exit.id), &["run", "context", "scale", "rows", "max_enter_line_acc_key", "p50_p99_max_enter_acc_key", "labels"], &steps_rows)?;
    db.write(
        &format!("SN4-{}-{side}-{}.tsv", spec.tf, exit.id),
        &["run", "context", "stage", "n", "outer_values_walked", "values_empty_s7", "core_cells", "runs_offered", "empty_cells", "not_eligible_e1_e2_e4_passese2", "steps_p50_p99_max_enter_line_acc_key", "inserts", "removes", "live_not_wired", "qty_rule", "labels"],
        &sn4,
    )?;
    // --- common_wf: RULES, the five search partitions (one per share level), its own views and words ---
    let parts: Vec<Vec<String>> = levels
        .iter()
        .enumerate()
        .map(|(c, bp)| {
            wfio::partition_row(
                part_tpl,
                u32::try_from(c + 1).unwrap_or(1),
                &[
                    ("tf", spec.tf.clone()),
                    ("side", if long { "long only".to_string() } else { "short only".to_string() }),
                    ("exit", exit.id.clone()),
                    ("level", format!("{}", bp / 100)),
                    ("family", "seconds".to_string()),
                ],
            )
        })
        .collect();
    let wf = wfio::load(&spec.data, wf_data, cal, &parts, members)?;
    let set = wf.ws.sets.get(wf.set).ok_or("window set")?;
    let walk = |trades: &[Trade]| -> Result<Acc, String> {
        let mut acc = Acc::new(set, &wf.views, &wf.rules);
        acc.reset();
        for tr in trades {
            acc.add(tr).map_err(|e| format!("acc add: {e}"))?;
        }
        acc.finish().map_err(|e| format!("acc finish: {e}"))?;
        Ok(acc)
    };
    let trades_from = |nc: &NCells, rows: &[usize], col: usize| -> Vec<Trade> {
        rows.iter()
            .filter_map(|&j| {
                let c = nc.get(j)?;
                let i = *kidx.get(j)? as usize;
                let f = fam.fix.get(i)?;
                let day = ctx.sessions.get(f.sess as usize)?.day;
                Some(Trade {
                    day,
                    exit_day: day,
                    close_s: c.close.get(col).copied().unwrap_or(0),
                    net: c.net.get(col).copied().unwrap_or(0),
                    net_alt: 0,
                    rd_bp: i32::from(c.rd.get(col).copied().unwrap_or(0)),
                    hole: c.fl.get(col).copied().unwrap_or(0) & flag::HOLE != 0,
                    id: i as u64,
                    mark_prev: 0,
                })
            })
            .collect()
    };
    // --- re-walk: every listed entry over the whole period, grouped by N (one N's cells at a time), then offered in the minutes
    // driver's order ---
    let mut entries: Vec<(usize, usize, wf_sweep::acc::Key, [u64; 4], u64)> = Vec::new();
    for (vi, _) in inp.views.iter().enumerate() {
        for col in 0..ncol {
            let Some(l) = merged.get(vi * ncol + col) else { continue };
            for (key, code, n) in l.entries() {
                entries.push((vi, col, key, code, n));
            }
        }
    }
    let mut want: BTreeMap<usize, BTreeSet<(usize, [u64; 4])>> = BTreeMap::new();
    let mut combos: BTreeMap<[u64; 4], Combo> = BTreeMap::new();
    for e in &entries {
        let cb = dec.decode(e.3)?;
        combos.insert(e.3, cb);
        want.entry(cb.ni).or_default().insert((e.1, e.3));
    }
    let mut walked_tr: BTreeMap<(usize, [u64; 4]), Walked> = BTreeMap::new();
    let mut rewalk_rss = None;
    // ONE kept set held at a time (a full-period 3 s kept set is 1.3 MB; the entries of a group are taken in outer-value order).
    let rows_of = |cb: &Combo, cache: &mut Option<(usize, sniper_kernel::bits::Bits)>| -> Vec<usize> {
        let kept = cb.outer.map(|oi| {
            if cache.as_ref().is_none_or(|c| c.0 != oi) {
                *cache = Some((oi, specs.get(oi).map_or_else(|| sniper_kernel::bits::Bits::new(m), |s| fam.kept(s, kidx, m))));
            }
            cache.as_ref().map_or_else(|| sniper_kernel::bits::Bits::new(m), |c| c.1.clone())
        });
        combo_rows(&krows, kept.as_ref(), core, cb)
    };
    for (ni, set_n) in &want {
        let nc = NCells::load(&cells_path(cells_dir, *ni), m)?;
        rewalk_rss = rewalk_rss.max(crate::monitor::rss_mb());
        let mut cache: Option<(usize, sniper_kernel::bits::Bits)> = None;
        let mut order: Vec<(Option<usize>, usize, [u64; 4])> = set_n.iter().map(|&(col, code)| (combos.get(&code).and_then(|c| c.outer), col, code)).collect();
        order.sort_unstable();
        for (_, col, code) in order {
            let cb = *combos.get(&code).ok_or("combo")?;
            let rows = rows_of(&cb, &mut cache);
            let trades = trades_from(&nc, &rows, col);
            walked_tr.insert((col, code), (cb, rows, trades));
        }
    }
    let mut cl = common_wf::Lists::new(&wf.views, set, &wf.rules, 0);
    let mut walked: BTreeSet<(usize, [u64; 4])> = BTreeSet::new();
    let mut mismatches = 0_u64;
    let mut mismatch_lines = Vec::new();
    for (vi, col, key, code, n) in &entries {
        let Some(v) = inp.views.get(*vi) else { continue };
        let cv = wf.views.views.iter().position(|x| x.id == v.id).ok_or_else(|| format!("view {} is not a common_wf VIEWS-STD row", v.id))?;
        let cview = wf.views.views.get(cv).ok_or("view")?;
        let (cb, _, trades) = walked_tr.get(&(*col, *code)).ok_or("re-walk entry missing")?;
        let acc = walk(trades)?;
        let lr = acc.learn(cview.win).map_err(|e| format!("learn: {e}"))?;
        let vals: Vec<i64> = cview.key.iter().map(|c| crit_val(c, &lr, &acc, wf.rules.base_capital, false, &[]).unwrap_or(i64::MIN)).collect();
        if key.0.get(..vals.len()) != Some(vals.as_slice()) {
            mismatches += 1;
            if mismatch_lines.len() < 5 {
                mismatch_lines.push(format!("rewalk_equal FAILED {} level {} code {}: sweep {:?} common_wf {:?}", v.id, col, Code(*code).hex(), key.0.get(..vals.len()), vals));
            }
        }
        if walked.contains(&(*col, *code)) {
            continue;
        }
        let mut tk = Tickets::new();
        tk.push(Ticket { grid: 0, stage: u8::from(cb.outer.is_some()), set: ReadSet::EMPTY }, &wf.rules).map_err(|e| format!("ticket: {e}"))?;
        let kmax = wf.views.views.iter().map(|x| x.k as u64).max().unwrap_or(1);
        let pid = Partition(u32::try_from(col + 1).unwrap_or(1));
        for i in 0..(*n).min(kmax) {
            let c2 = Code(*code).plus(i).ok_or("code overflow")?;
            cl.offer(&wf.views, pid, &tk, WfCode(c2.0), &acc).map_err(|e| format!("offer: {e}"))?;
        }
        walked.insert((*col, *code));
    }
    lines.push(format!("re-walk: {} distinct (level, combo) entries walked over the whole period and offered to common_wf; rewalk_equal mismatches {}; grouped by N ({} N files read; process RSS at most {:?} MB)", walked.len(), mismatches, want.len(), rewalk_rss));
    lines.extend(mismatch_lines);
    // --- the picks: (level, view) -> code; the ones not walked and every pick's twin / protection counts, grouped by N ---
    let honest = wf.ws.honest();
    let sniper_ids: Vec<&str> = inp.views.iter().map(|v| v.id.as_str()).collect();
    let mut sess_rows: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (j, r) in krows.iter().enumerate() {
        sess_rows.entry(r.sess).or_default().push(j);
    }
    let tslots: Vec<crate::twin::Slot> = sess_rows
        .iter()
        .filter_map(|(s, v)| {
            let d = ctx.sessions.get(*s as usize)?.day;
            Some(crate::twin::Slot { sess: *s, year: common_feat::cal::civil(d).0, candles: u16::try_from(v.len()).unwrap_or(u16::MAX) })
        })
        .collect();
    // per (col, view index): (code, combo, headline-honest twin wanted, test window)
    let mut picks: BTreeMap<(usize, usize), PickAt> = BTreeMap::new();
    for col in 0..levels.len() {
        let pid = u32::try_from(col + 1).unwrap_or(1);
        for (vi, v) in wf.views.views.iter().enumerate() {
            let vid = ViewId(u16::try_from(vi).unwrap_or(0));
            if !set.has(v.win) || !sniper_ids.contains(&v.id.as_str()) || v.role == Role::Miss {
                continue;
            }
            let kept = cl.kept(Partition(pid), vid);
            let Some(top) = kept.first() else { continue };
            let cb = dec.decode(top.key.code.0)?;
            let row = wf.ws.row(v.win);
            let tw = (v.role == Role::Headline && row.honest).then(|| (row.test_from.unwrap_or(0), row.test_to.unwrap_or(-1)));
            picks.insert((col, vi), (top.key.code.0, cb, tw));
        }
    }
    struct PickData {
        rows: Vec<usize>,
        trades: Vec<Trade>,
        prot: (u32, u32),
        twin: Option<(u32, u32, Vec<i64>)>,
        /// SECONDS PORTER (SNIPER s16): per trade the (units entered, entry value, closing value) at the pick's (N, level) for the SN6
        /// rows, re-priced (headline honest picks only, the ones with SN6 rows; empty for the others).
        qv: Vec<(u64, i64, i64)>,
    }
    let mut pdata: BTreeMap<(usize, usize), PickData> = BTreeMap::new();
    let mut by_n: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
    for (k, (_, cb, _)) in &picks {
        by_n.entry(cb.ni).or_default().push(*k);
    }
    for (ni, ks) in &by_n {
        let nc = NCells::load(&cells_path(cells_dir, *ni), m)?;
        let mut cache: Option<(usize, sniper_kernel::bits::Bits)> = None;
        for k in ks {
            let (code, cb, tw) = picks.get(k).cloned().ok_or("pick")?;
            let col = k.0;
            let (rows, trades) = match walked_tr.get(&(col, code)) {
                Some((_, r, tr)) => (r.clone(), tr.clone()),
                None => {
                    // a later code of an offered run: the run's first code holds the same trades
                    let r = rows_of(&cb, &mut cache);
                    let tr = trades_from(&nc, &r, col);
                    (r, tr)
                }
            };
            let prot = price::held_counts(rows.iter().map(|&j| nc.get(j).map_or(0, |c| c.fl.get(col).copied().unwrap_or(0))));
            let twin_out = tw.map(|test| {
                let own: Vec<(i32, i64)> = trades.iter().map(|x| (common_wf::date::mk_of_day(x.exit_day), x.net)).collect();
                let (_, own_net, _) = test_figures(&own, test);
                let mut per_year: BTreeMap<i32, u32> = BTreeMap::new();
                for x in &trades {
                    *per_year.entry(common_feat::cal::civil(x.day).0).or_insert(0) += 1;
                }
                let mut nets = Vec::with_capacity(twin.1 as usize);
                let (mut k_net, mut k_all) = (0_u32, 0_u32);
                for world in 0..twin.1 {
                    let d = crate::twin::draws(twin.0, world, &tslots, &per_year);
                    let tws: Vec<(i32, i64)> = d
                        .iter()
                        .filter_map(|(s, c)| {
                            let j = *sess_rows.get(s)?.get(usize::from(c.saturating_sub(1)))?;
                            let cc = nc.get(j)?;
                            let f = fam.fix.get(*kidx.get(j)? as usize)?;
                            Some((fm.sess_mk.get(f.sess as usize).copied().unwrap_or(0), cc.net.get(col).copied().unwrap_or(0)))
                        })
                        .collect();
                    let (all, net, _) = test_figures(&tws, test);
                    k_net += u32::from(net >= own_net);
                    k_all += u32::from(all);
                    nets.push(net);
                }
                nets.sort_unstable();
                (k_net, k_all, nets)
            });
            pdata.insert(*k, PickData { rows, trades, prot, twin: twin_out, qv: Vec::new() });
        }
        // SNIPER s16 trade rows (SN6, headline honest picks): re-price their rows at this N with the pricing's own function, each
        // re-priced row checked equal to its CELLS record (fail closed), then keep the pick's level's (units, entry, closing value).
        let heads: Vec<(usize, usize)> = ks.iter().copied().filter(|k| picks.get(k).is_some_and(|p| p.2.is_some())).collect();
        let js: BTreeSet<usize> = heads.iter().filter_map(|k| pdata.get(k)).flat_map(|pd| pd.rows.iter().copied()).collect();
        if !js.is_empty() {
            let rq = reprice(fam, ctx, cfg, exit, long, slots, kidx, &js, spec.threads, &nc, *ni, inp.instr.underlying())?;
            for k in &heads {
                if let Some(pd) = pdata.get_mut(k) {
                    pd.qv = pd.rows.iter().map(|j| rq.get(j).and_then(|q| q.get(k.0)).copied().unwrap_or_default()).collect();
                }
            }
        }
    }
    // --- the report: common_wf's words, per search (= per share level), as the minutes driver prints it ---
    let combo_words = |cb: &Combo| -> String {
        let lad = if long { &inp.core.0 } else { &inp.core.1 };
        let lv = |li: usize, j: u16| if j == 0 { "OFF".to_string() } else { lad.l.get(li).map_or_else(String::new, |x| x.level_text(usize::from(j))) };
        let outer = cb.outer.and_then(|i| values.get(i)).map_or_else(|| "every other dial OFF".to_string(), |x: &OuterValue| format!("{} (every other dial OFF)", x.label));
        format!("RSI 14 {}; CCI 20 {}; %R 14 {}; {}; N = {}", lv(0, cb.rsi), lv(1, cb.cci), lv(2, cb.wr), outer, n_grid.get(cb.ni).copied().unwrap_or(0))
    };
    let mut pick_rows = Vec::new();
    let mut report_rows = Vec::new();
    let mut trade_rows = Vec::new();
    let mut twin_rows = Vec::new();
    let mut co = CtxOut { lines: Vec::new(), searches: 0, wf_passed: 0, tt_passed: 0, both: 0, mismatches };
    for (col, bp) in levels.iter().enumerate() {
        let pid = u32::try_from(col + 1).unwrap_or(1);
        let p = wf.rules.partition(pid).map_err(|e| e.to_string())?.clone();
        let search = format!("{cname}|{}%", bp / 100);
        co.searches += 1;
        let mut out: Vec<String> = vec![format!("--- search {search} ({}; its own {} candles (GDFL corrected seconds); cash intraday 5x; N slots of Rs 20 L / N; {}; 2018-09..2026-10; run {}; {period_words}) ---", inp.instr.underlying(), spec.tf, inp.qty_rule.label(), spec.run_id)];
        // SNIPER s16 + base Q7: one line per pick, printed after the search's METHOD lines (so no earlier line moves; the minutes driver's rule).
        let mut acct_lines: Vec<String> = Vec::new();
        let mut m_items: BTreeMap<u8, common_wf::report::MethodItem> = BTreeMap::new();
        let mut verdicts: BTreeMap<String, bool> = BTreeMap::new();
        for (vi, v) in wf.views.views.iter().enumerate() {
            let vid = ViewId(u16::try_from(vi).unwrap_or(0));
            if !set.has(v.win) {
                continue;
            }
            if !sniper_ids.contains(&v.id.as_str()) {
                if matches!(v.role, Role::Headline | Role::Beside | Role::Miss) {
                    out.push(format!("{}: not walked in host v1 (its key needs a part the sweep does not hold yet; named)", v.id));
                }
                continue;
            }
            let cn = cl.counts(Partition(pid), vid);
            let kept = cl.kept(Partition(pid), vid);
            let row = wf.ws.row(v.win);
            if v.role == Role::Miss {
                out.push(format!("{} closest misses ({} kept of {} offered):", v.id, kept.len(), cn.offered));
                for k in kept.iter().take(10) {
                    let cb = dec.decode(k.key.code.0)?;
                    out.push(format!("  MISS {} [{}]: {}", Code(k.key.code.0).hex(), combo_words(&cb), common_wf::report::miss_checks(set, v.win, &k.facts).join("; ")));
                }
                continue;
            }
            let Some(top) = kept.first() else {
                out.push(format!("{} W07: {}", v.id, common_wf::report::no_pick_line(&wf.words, v, row, &cn, &p, &wf.rules).map_err(|e| e.to_string())?));
                if v.role == Role::Headline && row.honest {
                    m_items.insert(v.win.0, common_wf::report::MethodItem { win: v.win, partition: Partition(pid), view: v.id.clone(), label: row.pick_label.clone(), code: None, word: common_wf::report::ItemWord::NoPick });
                }
                continue;
            };
            let learn = set.learn_range(v.win).unwrap_or((0, -1));
            let pick = PickWin { set: set.set_id(), win: v.win, learn, kind: top.kind, code: top.key.code };
            let cb = dec.decode(top.key.code.0)?;
            let pd = pdata.get(&(col, vi)).ok_or("pick data missing")?;
            let rows = &pd.rows;
            let trades = &pd.trades;
            let acc = walk(trades)?;
            if acc.facts(Partition(pid)) != top.facts {
                return Err(format!("{search} {}: the report re-walk's facts differ from the kept facts (rewalk_equal)", v.id));
            }
            let ties = merged.get(inp.views.iter().position(|x| x.id == v.id).unwrap_or(0) * ncol + col).and_then(|l| l.top).map_or(0, |x| x.1);
            out.push(format!("{} PICK {} [{}] key {:?}", v.id, Code(top.key.code.0).hex(), combo_words(&cb), top.key.parts.get(..usize::from(top.key.n)).unwrap_or(&[])));
            for h in &honest {
                let facts = acc.facts(Partition(pid));
                let rv = common_wf::verdict(set, *h, pick, &facts, &wf.rules).map_err(|e| e.to_string())?;
                for ln in common_wf::report::reading_lines(&wf.words, set, *h, &rv, &pick, row, &facts, &acc, &wf.rules, &p, "", None).map_err(|e| e.to_string())? {
                    out.push(format!("{} {}: {ln}", v.id, wf.ws.row(*h).id));
                }
            }
            let n_open = common_wf::report::open_at_learn_end(trades, learn.1);
            out.push(format!("{} label: {}", v.id, common_wf::report::pick_label(&wf.words, row, learn, top, n_open, &wf.rules).map_err(|e| e.to_string())?));
            out.push(format!("{} ties: {} (every tied code of the sweep counted at #1, P18)", v.id, common_wf::report::ties_line(&wf.words, ties).map_err(|e| e.to_string())?));
            out.push(format!("{} margin: {}", v.id, common_wf::report::margin_line(&wf.words, kept, v).map_err(|e| e.to_string())?));
            out.push(format!("{} holdout: {}", v.id, common_wf::report::holdout_line(&wf.words, row, &wf.rules).map_err(|e| e.to_string())?));
            if row.fixed_after != "-" {
                out.push(format!("{} fixed_after: {}", v.id, row.fixed_after));
            }
            for line in common_wf::report::knowingly_lines(&wf.words, &p, &honest.iter().map(|h| wf.ws.row(*h)).collect::<Vec<_>>()).map_err(|e| e.to_string())? {
                out.push(format!("{} knowingly: {line}", v.id));
            }
            out.push(format!("{} s12: {}", v.id, inp.words.line("S12", &[])));
            let (lf, lt) = learn;
            let n_learn = u32::try_from(trades.iter().filter(|x| (lf..=lt).contains(&common_wf::date::mk_of_day(x.exit_day))).count()).unwrap_or(u32::MAX);
            let unseen = u32::try_from(trades.len()).unwrap_or(u32::MAX).saturating_sub(n_learn);
            let mut sess_set: Vec<i32> = trades.iter().filter(|x| (lf..=lt).contains(&common_wf::date::mk_of_day(x.exit_day))).map(|x| x.day).collect();
            sess_set.dedup();
            for w in inp.words.trade_count(n_learn, u32::try_from(sess_set.len()).unwrap_or(u32::MAX), unseen) {
                out.push(format!("{} counts: {w}", v.id));
            }
            let (k_prot, k_clip) = pd.prot;
            out.push(format!(
                "{} protection: Dhan market protection (REQ 70a, {}% limit) held units on {k_prot} of {} trades (a block of the entry or the exit order printed past the protection limit while units were open; the crate's past_limit); a block clipped by an order's stop on {k_clip} of {} trades (the crate's clipped; beside)",
                v.id,
                cfg.protection_bp / 100,
                trades.len(),
                trades.len()
            ));
            // SNIPER s16 + base Q7 (the minutes driver's line): the pick's own account walk at its N (never more money than the account
            // has free at that instant). The sized count is the pricing's own `size::size` of the row's reference (exact).
            let slot = slots.get(cb.ni).copied().unwrap_or(0);
            let sz_of = |j: usize| crate::size::size(cfg.qty_rule, slot, cfg.account, cfg.lev, t.refpx.get(j).copied().unwrap_or(0), 1);
            let acct_in: Vec<(u64, i64, i64)> = trades.iter().zip(rows).map(|(x, &j)| (sz_of(j).qty, t.refpx.get(j).copied().unwrap_or(0), x.net)).collect();
            let acct = crate::size::account_walk(cfg.account, &acct_in, cfg.lev);
            let k_cut = acct.iter().filter(|a| a.2.is_some()).count();
            let k_static = rows.iter().filter(|&&j| sz_of(j).capped).count();
            acct_lines.push(format!(
                "{} account: {} at N = {} (slot Rs {}); base Q7 (never more money than the account has free at that instant; one position at a time, so the free money is the account less the pick's losses so far, profits never added): the count fits on {} of {} trades; cut by the whole account at pricing on {k_static}; cut by the losses so far on {k_cut}{}",
                v.id,
                inp.qty_rule.label(),
                n_grid.get(cb.ni).copied().unwrap_or(0),
                common_wf::words::fmt_rs(slot).trim_start_matches("Rs ").trim_start_matches("Rs"),
                trades.len() - k_cut,
                trades.len(),
                if k_cut > 0 { " (each cut trade names the largest count that fits on its SN6 row for the headline picks; its net here stays at the rule's count: re-pricing a cut trade, and the sweep's own check, is plan FU-9 SC-ACCT, MAIN's: named)" } else { "" }
            ));
            let sv = inp.views.iter().position(|x| x.id == v.id).unwrap_or(0);
            let lc0 = common_wf::report::LuckCtx { eligible: merged.get(sv * ncol + col).map_or(0, |l| l.offered), offered: cells_offered, views: inp.views.len() as u64, partitions: wf.rules.partitions.len() as u64, twin: None, mirror: None };
            let luck = |tw: Option<(u64, u64)>| -> Result<String, String> {
                let lc = common_wf::report::LuckCtx { twin: tw, ..lc0.clone() };
                common_wf::report::luck_line(&wf.words, set, v.win, trades, &acc.facts(Partition(pid)), &lc).map_err(|e| e.to_string())
            };
            if !(v.role == Role::Headline && row.honest) {
                out.push(format!("{} luck: {}", v.id, luck(None)?));
            }
            let (_, vd) = common_wf::verdict(set, v.win, pick, &top.facts, &wf.rules).map_err(|e| e.to_string())?;
            let passed = matches!(vd, Verdict::Passed { .. });
            if v.role == Role::Headline && row.honest {
                verdicts.insert(row.id.clone(), passed);
                let word = match vd {
                    Verdict::Passed { .. } => common_wf::report::ItemWord::Passed,
                    Verdict::NotRun(common_wf::verdict::NotRun::ChosenRule { run }) => common_wf::report::ItemWord::Chosen(run),
                    _ => common_wf::report::ItemWord::NotPassed,
                };
                m_items.insert(v.win.0, common_wf::report::MethodItem { win: v.win, partition: Partition(pid), view: v.id.clone(), label: row.pick_label.clone(), code: Some(top.key.code), word });
                let test = (row.test_from.unwrap_or(0), row.test_to.unwrap_or(-1));
                let own: Vec<(i32, i64)> = trades.iter().map(|x| (common_wf::date::mk_of_day(x.exit_day), x.net)).collect();
                let (own_all, own_net, own_n) = test_figures(&own, test);
                let (k_net, k_all, nets) = pd.twin.clone().ok_or("twin missing")?;
                let tl = format!(
                    "random-entry twin ({} draws, seed {}; the same entries per calendar year at random decision candles of this context, at the pick's N and level; TEST months {}..{} only): random entries did as well (test net at least the pick's) in {k_net} of {}; every test year positive in {k_all} of {}; twin test net p5 / p50 / p95 {} / {} / {}; the pick: {own_n} test trades, test net {}, every test year positive: {}",
                    twin.1,
                    twin.0,
                    crate::rows::mk_text(test.0),
                    crate::rows::mk_text(test.1),
                    twin.1,
                    twin.1,
                    common_wf::words::fmt_rs(pct_of(&nets, 50)),
                    common_wf::words::fmt_rs(pct_of(&nets, 500)),
                    common_wf::words::fmt_rs(pct_of(&nets, 950)),
                    common_wf::words::fmt_rs(own_net),
                    if own_all { "yes" } else { "no" }
                );
                out.push(format!("{} twin: {tl}", v.id));
                out.push(format!("{} luck: {}", v.id, luck(Some((u64::from(twin.1), u64::from(k_all))))?));
                twin_rows.push(vec![spec.run_id.clone(), search.clone(), v.id.clone(), twin.1.to_string(), twin.0.to_string(), k_net.to_string(), k_all.to_string(), pct_of(&nets, 50).to_string(), pct_of(&nets, 500).to_string(), pct_of(&nets, 950).to_string(), own_net.to_string(), own_n.to_string(), inp.qty_rule.id().to_string(), labels.to_string()]);
                if pd.qv.len() != trades.len() {
                    return Err(format!("{search} {}: SN6 re-pricing holds {} of {} trades", v.id, pd.qv.len(), trades.len()));
                }
                for (((x, &j), a), &(units, ev, xv)) in trades.iter().zip(rows).zip(&acct).zip(&pd.qv) {
                    let k = kidx.get(j).and_then(|&i| fam.fix.get(i as usize)).map_or(0, |c| c.k);
                    let sz = sz_of(j);
                    let cap = match a.2 {
                        Some(fit) => format!("CUT by the losses so far: free Rs {} < money in Rs {}: the largest count that fits is {fit}", common_wf::words::fmt_rs(a.0).trim_start_matches("Rs "), common_wf::words::fmt_rs(a.1).trim_start_matches("Rs ")),
                        None if sz.capped => format!("CUT by the whole account at pricing: {} of the rule's {}", sz.qty, sz.rule_qty),
                        None => "fits".to_string(),
                    };
                    trade_rows.push(vec![
                        spec.run_id.clone(),
                        search.clone(),
                        v.id.clone(),
                        crate::rows::mk_text(common_wf::date::mk_of_day(x.day)),
                        x.day.to_string(),
                        k.to_string(),
                        t.entry_s.get(j).map_or(0, |&e| i64::from(e)).to_string(),
                        x.close_s.to_string(),
                        x.net.to_string(),
                        x.rd_bp.to_string(),
                        u8::from(x.hole).to_string(),
                        inp.qty_rule.id().to_string(),
                        t.refpx.get(j).copied().unwrap_or(0).to_string(),
                        slot.to_string(),
                        n_grid.get(cb.ni).copied().unwrap_or(0).to_string(),
                        format!("{}/{}", cfg.lev.0, cfg.lev.1),
                        "1".to_string(),
                        sz.rule_qty.to_string(),
                        sz.qty.to_string(),
                        units.to_string(),
                        ev.to_string(),
                        xv.to_string(),
                        a.1.to_string(),
                        a.0.to_string(),
                        cap,
                        labels.to_string(),
                    ]);
                }
            }
            pick_rows.push(vec![
                spec.run_id.clone(),
                search.clone(),
                v.id.clone(),
                format!("{:?}", v.role),
                Code(top.key.code.0).hex(),
                n_grid.get(cb.ni).copied().unwrap_or(0).to_string(),
                combo_words(&cb),
                format!("{:?}", top.key.parts.get(..usize::from(top.key.n)).unwrap_or(&[])),
                n_learn.to_string(),
                unseen.to_string(),
                common_wf::report::verdict_code(&vd),
                ties.to_string(),
                k_prot.to_string(),
                k_clip.to_string(),
                inp.qty_rule.id().to_string(),
                labels.to_string(),
            ]);
        }
        let items: Vec<common_wf::report::MethodItem> = honest.iter().filter_map(|h| m_items.get(&h.0).cloned()).collect();
        if let Some(ml) = common_wf::report::method_line(&wf.words, &items, &wf.ws, &wf.views, &p, &wf.rules).map_err(|e| e.to_string())? {
            for mline in ml {
                out.push(format!("METHOD: {mline} ({})", inp.qty_rule.label()));
            }
        }
        out.extend(acct_lines);
        let (w_ok, t_ok) = (verdicts.get("WF").copied().unwrap_or(false), verdicts.get("TT").copied().unwrap_or(false));
        co.wf_passed += u64::from(w_ok);
        co.tt_passed += u64::from(t_ok);
        co.both += u64::from(w_ok && t_ok);
        for (i, l) in out.iter().enumerate() {
            report_rows.push(vec![spec.run_id.clone(), search.clone(), i.to_string(), l.clone(), inp.qty_rule.id().to_string(), labels.to_string()]);
        }
        co.lines.extend(out);
    }
    db.write(&format!("SN-PICKS-{}-{side}-{}.tsv", spec.tf, exit.id), &["run", "search", "view", "role", "code", "n", "combo", "key", "learn_trades", "unseen_trades", "verdict_code", "ties_at_1", "protection_held_trades", "stop_clipped_trades", "qty_rule", "labels"], &pick_rows)?;
    db.write(&format!("SN-REPORT-{}-{side}-{}.tsv", spec.tf, exit.id), &["run", "search", "line_no", "line", "qty_rule", "labels"], &report_rows)?;
    db.write(&format!("SN6-{}-{side}-{}.tsv", spec.tf, exit.id), &["run", "search", "view", "month", "day", "candle", "entry_block_first_s", "close_sod", "net_paisa", "rd_bp", "hole", "qty_rule", "ref_px_paisa", "slot_paisa", "n", "lev", "unit", "rule_qty", "qty", "units_entered", "entry_value_paisa", "exit_value_paisa", "money_in_paisa", "free_before_paisa", "account_cap", "labels"], &trade_rows)?;
    db.write(&format!("SN9-{}-{side}-{}.tsv", spec.tf, exit.id), &["run", "search", "view", "draws", "seed", "k_net_at_least", "k_every_test_year_pos", "p5_net", "p50_net", "p95_net", "pick_test_net", "pick_test_trades", "qty_rule", "labels"], &twin_rows)?;
    let mut all = lines;
    all.extend(co.lines);
    co.lines = all;
    Ok(co)
}

/// Builds a seconds family over a period (the `sec-build` subcommand: the readings and the memory line, no context).
///
/// # Errors
/// A refused input.
pub fn sec_build(inp: &Inputs, tf: &str, period: Option<(i32, i32)>, threads: usize, with_htf: bool) -> Result<Vec<String>, String> {
    let run = &inp.run;
    let cont = common_feat::cal::parse_ymd("2018-09-07").ok_or("cont start")?;
    let ctx = feat_io::load_ctx(Path::new(feat_io::defaults::DATES), Path::new(feat_io::defaults::BHAV), cont)?;
    let family: Vec<String> = run.list::<String>("seconds_family")?;
    let lad = if with_htf { Some(crate::cands::htf_ladders(&inp.dials)?) } else { None };
    let (from, to) = period.unwrap_or((cont, i32::MAX));
    let sspec = SecSpec { tf: tf.to_string(), others: if with_htf { family.iter().filter(|x| *x != tf).cloned().collect() } else { Vec::new() }, from, to, threads, chunk: run.num("sec_read_chunk_sessions")? };
    let t0 = Instant::now();
    let fam = sec::build_family(&ctx, &sspec, Path::new(feat_io::defaults::TICKS), inp.instr.underlying(), &inp.dials, (&inp.core.0, &inp.core.1), run.hms("last_decision_before_ist")?, lad.as_ref())?;
    let n_grid: Vec<i64> = run.list("n_grid")?;
    let proj = projected_mb(&fam, threads, n_grid.len(), run.num("sec_price_chunk_sessions")?);
    let rss = crate::monitor::rss_mb();
    Ok(vec![format!(
        "sec-build {tf} {}: {} sessions, {} corrected 1-s bars (halt-window volume-0 seconds dropped {} {:?}), {} candles, {} candidates, K_tf {}, not entry {:?}, other timeframes {:?} candles {:?}; stride {} B + {} B fixed: {} MB resident; process RSS {:?} MB; projected context extra {proj} MB; {:.1} s wall ({:.1} s compressing)",
        period.map_or_else(|| "whole period".to_string(), |(a, b)| format!("{}..{}", common_feat::cal::ymd(a), common_feat::cal::ymd(b))),
        fam.stats.days,
        fam.stats.bars_1s,
        fam.stats.halt_dropped,
        fam.stats.halt_dropped_days,
        fam.stats.candles,
        fam.len(),
        fam.kmax,
        fam.refused,
        fam.htf_names,
        fam.stats.other_candles,
        fam.lay.stride,
        std::mem::size_of::<sec::RowFix>(),
        fam.resident_bytes() >> 20,
        rss,
        t0.elapsed().as_secs_f64(),
        fam.stats.compress_s
    )])
}

/// The fold check (`sec-dump`): the host's DAY-BY-DAY seconds stream over `[from, to]` written through `feat_io::run::run` with
/// default levels, exactly the file `feat-run seconds --tf <s> --from <from> --to <to>` writes (same name, same columns), so the two
/// files are compared byte for byte (`cmp`).
///
/// # Errors
/// A refused input or an existing output file.
pub fn sec_dump(tf: &str, from: i32, to: i32, outdir: &Path, sym: &str) -> Result<Vec<String>, String> {
    let tf_s = sec::tf_secs(tf).ok_or_else(|| format!("{tf}: not a seconds timeframe"))?;
    let cont = common_feat::cal::parse_ymd("2018-09-07").ok_or("cont start")?;
    let ctx = feat_io::load_ctx(Path::new(feat_io::defaults::DATES), Path::new(feat_io::defaults::BHAV), cont)?;
    let ticks = Path::new(feat_io::defaults::TICKS);
    let mut candles = Vec::new();
    let mut bars = 0_usize;
    let (mut dropped, mut dropped_days) = (0_u64, Vec::new());
    for (i, s) in ctx.sessions.iter().enumerate().filter(|(_, s)| s.day >= from && s.day <= to) {
        let Some(d) = sec::day_seconds(ticks, sym, i, s)? else { continue };
        bars += d.one.len();
        if d.halt_dropped > 0 {
            dropped += d.halt_dropped;
            dropped_days.push(format!("{} {}", common_feat::cal::ymd(s.day), d.halt_dropped));
        }
        let s1 = feat_io::host::Stream { tf_s: 1, source: sec::SEC_SOURCE.to_string(), candles: d.one };
        candles.extend(feat_io::host::fold_stream(&s1, tf_s, &ctx, &format!("{tf_s}s")).candles);
    }
    let s = feat_io::host::Stream { tf_s, source: format!("{tf_s}s day by day"), candles };
    let lv = vec![feat_io::host::Levels::default(); ctx.sessions.len()];
    let mut buf: Vec<u8> = Vec::new();
    let res = feat_io::run::run(&ctx, &s, &lv, &mut [], Some(&mut buf), None)?;
    std::fs::create_dir_all(outdir).map_err(|e| e.to_string())?;
    let path = outdir.join(format!("READINGS-{sym}-{tf_s}s-{}-{}.tsv", common_feat::cal::ymd(from), common_feat::cal::ymd(to)));
    if path.exists() {
        return Err(format!("{} exists: refused (never overwrite)", path.display()));
    }
    std::fs::write(&path, &buf).map_err(|e| e.to_string())?;
    Ok(vec![format!("sec-dump {tf_s}s {}..{}: {} candles from {} corrected 1-s bars (halt-window volume-0 seconds dropped {dropped} {dropped_days:?}), folded ONE DAY AT A TIME; wrote {}", common_feat::cal::ymd(from), common_feat::cal::ymd(to), res.rows, bars, path.display())])
}

/// SEC-EQ end to end on a seconds family (`sec-eq --tf 30s --from D --to D`): the FULL family built the minutes driver's way from a
/// WHOLE-RANGE fold of the same corrected seconds (one `fold_stream` over the range, `cands::build`, `htf_reads` + `attach_htf` over
/// the other seconds timeframes) against [`sec::build_family`] (day by day, streaming cursor, compact rows): every value of
/// `cands::outer_values`, both sides, every XE context. On a minutes timeframe (`sec-eq --tf 60min`): the full minute family
/// against [`sec::compress_family`] of it.
///
/// # Errors
/// A refused input.
pub fn sec_eq_run(inp: &Inputs, tf: &str, period: Option<(i32, i32)>, threads: usize) -> Result<Vec<String>, String> {
    sec_eq_run_ctx(inp, tf, period, threads, false)
}

/// PORTER (secfd): [`sec_eq_run`] with the seven context dials ON when `ctx_on` (`sec-eq --context-dials on`): the full family gets
/// `ctxdials::attach` (on the whole-range decision stream), the compact family `ctxdials::prepare` + the day loop; every value of
/// `cands::outer_values` (the DIALS BUILD arms included) must equal the compact kept set bit for bit. `false` is [`sec_eq_run`].
///
/// # Errors
/// A refused input.
#[allow(clippy::too_many_lines, reason = "the two SEC-EQ routes (seconds, minutes) in one place")]
pub fn sec_eq_run_ctx(inp: &Inputs, tf: &str, period: Option<(i32, i32)>, threads: usize, ctx_on: bool) -> Result<Vec<String>, String> {
    let run = &inp.run;
    let cont = common_feat::cal::parse_ymd("2018-09-07").ok_or("cont start")?;
    let ctx = feat_io::load_ctx(Path::new(feat_io::defaults::DATES), Path::new(feat_io::defaults::BHAV), cont)?;
    let cut = run.hms("last_decision_before_ist")?;
    let lad = crate::cands::htf_ladders(&inp.dials)?;
    let t0 = Instant::now();
    let (full, comp, what) = if let Some(tf_s) = sec::tf_secs(tf) {
        let (from, to) = period.ok_or("sec-eq on a seconds family needs --from and --to")?;
        let family: Vec<String> = run.list::<String>("seconds_family")?;
        let others: Vec<String> = family.iter().filter(|x| *x != tf).cloned().collect();
        let ticks = Path::new(feat_io::defaults::TICKS);
        let mut one = Vec::new();
        let mut facts = vec![feat_io::host::DayFact::default(); ctx.sessions.len()];
        for (i, s) in ctx.sessions.iter().enumerate().filter(|(_, s)| s.day >= from && s.day <= to && s.day >= cont) {
            let Some(d) = sec::day_seconds(ticks, inp.instr.underlying(), i, s)? else { continue };
            if let Some(f) = facts.get_mut(i) {
                f.open = d.one.first().map(|x| x.1.open);
                f.last = d.one.last().map(|x| x.1.close);
            }
            one.extend(d.one);
        }
        let s1 = feat_io::host::Stream { tf_s: 1, source: sec::SEC_SOURCE.to_string(), candles: one };
        let lv = feat_io::host::levels(&ctx, &facts);
        let dec = feat_io::host::fold_stream(&s1, tf_s, &ctx, tf);
        let mut full = crate::cands::build(&ctx, &dec, &lv, (&inp.core.0, &inp.core.1), cut)?;
        let mut reads = Vec::new();
        for o in &others {
            let st = feat_io::host::fold_stream(&s1, sec::tf_secs(o).ok_or("tf")?, &ctx, o);
            reads.push(crate::cands::htf_reads(&ctx, &st, &lad)?);
        }
        crate::cands::attach_htf(&mut full, others.clone(), &reads);
        let prep = if ctx_on {
            let store = Path::new(feat_io::defaults::STORE);
            crate::ctxdials::attach(&mut full, &ctx, &dec, store)?;
            Some(crate::ctxdials::prepare(&ctx, store)?)
        } else {
            None
        };
        let sspec = SecSpec { tf: tf.to_string(), others, from, to, threads, chunk: run.num("sec_read_chunk_sessions")? };
        let comp = sec::build_family_ctx(&ctx, &sspec, ticks, inp.instr.underlying(), &inp.dials, (&inp.core.0, &inp.core.1), cut, Some(&lad), prep.as_ref())?;
        (full, comp, format!("seconds {tf} {}..{}: whole-range fold + cands::build + attach_htf against build_family (day by day, cursor, compact)", common_feat::cal::ymd(from), common_feat::cal::ymd(to)))
    } else {
        let store = Path::new(feat_io::defaults::STORE);
        let (stream, _) = feat_io::minute_stream(store, inp.instr.underlying(), tf, &ctx);
        if stream.tf_s < 60 {
            return Err(format!("{tf}: unknown timeframe"));
        }
        let (s1, _) = feat_io::minute_stream(store, inp.instr.underlying(), "1min", &ctx);
        let facts = feat_io::host::day_facts(&s1, &ctx);
        let lv = feat_io::host::levels(&ctx, &facts);
        let mut full = crate::cands::build(&ctx, &stream, &lv, (&inp.core.0, &inp.core.1), cut)?;
        let names: Vec<String> = run.list::<String>("minutes_family")?.into_iter().filter(|x| x != tf).collect();
        let mut reads = Vec::new();
        for n in &names {
            let (s, _) = feat_io::minute_stream(store, inp.instr.underlying(), n, &ctx);
            reads.push(crate::cands::htf_reads(&ctx, &s, &lad)?);
        }
        crate::cands::attach_htf(&mut full, names, &reads);
        if ctx_on {
            crate::ctxdials::attach(&mut full, &ctx, &stream, store)?;
        }
        let comp = sec::compress_family(&full, &lv, &inp.dials)?;
        (full, comp, format!("minutes {tf} whole period: cands::build + attach_htf against compress_family"))
    };
    let (n, rows, bad) = sec::sec_eq(&full, &comp, &inp.dials);
    let what = if ctx_on { format!("{what} + the seven context dials ON (ctxdials::attach against the compact context section)") } else { what };
    let mut v = vec![format!(
        "SEC-EQ {what}: {rows} rows, {n} values compared (both sides x 6 XE contexts), mismatches {}: {}; {:.1} s",
        bad.len(),
        if bad.is_empty() { "PASS (every kept set bit for bit)" } else { "FAIL" },
        t0.elapsed().as_secs_f64()
    )];
    v.extend(bad);
    Ok(v)
}
