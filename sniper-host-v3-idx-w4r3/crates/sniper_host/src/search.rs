//! The RUN driver of one timeframe family (plan gate 7; MAIN's FIRST SLICE): for each side and exit set (a CONTEXT), every
//! candidate row of the period is priced once through crate v2 at every N of the REQ 66b grid and every share level (the
//! 352-B outcome rows, D-S27), the lattice is swept (S0 + S1 over the dials the SLICE file names, LINE engine, learning months
//! only) at EVERY N, the per-N lists are merged with N as a trade-model digit above the dials' digits (D-S10: N searched inside
//! each search), and each list's entries are re-walked over the WHOLE period and offered to `common_wf`'s own `Lists`. The pick,
//! the verdict and every word are `common_wf`'s (SNIPER s11 (1)); the host adds only the plain combo words, the trade-count
//! words, the s12 label (S10) and the random-entry twin (SNIPER s3 (5)).
//!
//! **Exactness.** The sweep runs at every N, so a list's top k at each N is exact (SWEEP-EQ), and the best (combo, N) keys are
//! the merge of those lists: the plan's proxy-N shortlist limit (8.5) does not arise. Every re-walked entry's learning key is
//! compared with the sweep's key (`rewalk_equal`, counted; a mismatch refuses the search, never silent).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use common_fill::{Dial, US_S};
use common_wf::views::{Role, Ticket, Tickets, crit_val};
use common_wf::{Acc, Code as WfCode, Partition, PickWin, ReadSet, Trade, Verdict, ViewId};
use sniper_kernel::bits::Bits;
use sniper_kernel::code::Code;
use sniper_kernel::line::{Core, Row};
use wf_sweep::acc::{Key, Layout, Outcome};
use wf_sweep::lists::Lists;

use crate::cands::{self, Family, OuterValue};
use crate::db::Db;
use crate::esn1::{Inputs, day_tape, plan_of};
use crate::price::{self, Cfg, Priced, Refusal, flag};
use crate::rows::{ExitKind, ExitSet};
use crate::wfio;

/// One run's request.
#[derive(Clone, Debug)]
pub struct RunSpec {
    /// The timeframe (`3min` ...).
    pub tf: String,
    /// Sides (true = long).
    pub sides: Vec<bool>,
    /// Exit set ids.
    pub exits: Vec<String>,
    /// The SLICE file (dial value labels walked); `None`: every wired value.
    pub slice: Option<PathBuf>,
    /// The run folder.
    pub out: PathBuf,
    /// The run id.
    pub run_id: String,
    /// Worker threads.
    pub threads: usize,
    /// The lane's swap base, MB.
    pub swap_base_mb: u64,
    /// The data folder (the host's rows).
    pub data: PathBuf,
    /// The N of the grid swept (`None`: every N of `n_grid`, the exact search of N inside each search, D-S10). A subset is printed
    /// on every search line; every N stays priced and stored.
    pub n_sweep: Option<Vec<i64>>,
    /// SECONDS BUILDER (2026-11-14): the first and last IST day folded (`--from` / `--to`; a smoke over part of the period, labelled
    /// on every line). `None`: the whole period. The seconds families only (a minute run refuses it, named).
    pub period: Option<(i32, i32)>,
    /// DIALS BUILD: wire the seven context / calendar / range dials (`crate::ctxdials`; default true). `false`
    /// (`--context-dials off`) is binary k's run byte for byte: nothing is read and the seven stay `live_not_wired`.
    /// PORTER (secfd): on this tree `false` is sniper-host-v1-secf-a's run byte for byte (minutes and seconds drivers).
    pub context_dials: bool,
}

/// The SLICE rows: `dial  label` (a value label as `cands::outer_values` prints it, or `ALL`).
#[derive(Clone, Debug, Default)]
pub struct Slice {
    rows: Vec<(String, String)>,
    /// The file's sha256.
    pub sha: String,
    /// The file name.
    pub name: String,
}

impl Slice {
    /// Reads a slice file.
    ///
    /// # Errors
    /// An unreadable file or a short row.
    pub fn load(p: &Path) -> Result<Self, String> {
        let (t, sha) = crate::rows::read_with_sha(&p.to_string_lossy())?;
        let mut rows = Vec::new();
        for l in t.lines() {
            if l.starts_with('#') || l.starts_with("dial\t") || l.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = l.split('\t').collect();
            let (Some(d), Some(lab)) = (f.first(), f.get(1)) else { return Err(format!("slice row short: {l}")) };
            rows.push(((*d).to_string(), (*lab).to_string()));
        }
        Ok(Self { rows, sha, name: p.file_name().map_or_else(String::new, |x| x.to_string_lossy().to_string()) })
    }

    /// Does the slice walk any value of dial `dial`?
    #[must_use]
    pub fn has_dial(&self, dial: &str) -> bool {
        self.rows.iter().any(|(d, _)| d == dial)
    }

    /// Is value `label` of dial `dial` walked?
    #[must_use]
    pub fn keeps(&self, dial: &str, label: &str) -> bool {
        self.rows.iter().any(|(d, l)| d == dial && (l == "ALL" || l == label || l == label.trim_end_matches(" (mirror)")))
    }
}

/// The decoder of a merged code: digits LEAST significant first in the canonical order (M-CCI, M-RSI, M-WR, then every other
/// dial in the file order), then the N digit.
pub(crate) struct Decoder {
    radices: Vec<u64>,
    order: Vec<usize>,
    core_pos: [usize; 3],
    vmap: BTreeMap<(usize, u64), usize>,
}

/// One decoded combo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Combo {
    pub(crate) ni: usize,
    pub(crate) rsi: u16,
    pub(crate) cci: u16,
    pub(crate) wr: u16,
    pub(crate) outer: Option<usize>,
}

impl Decoder {
    pub(crate) fn new(radix: &[u64], core_pos: [usize; 3], values: &[OuterValue], n_grid: usize) -> Self {
        let mut order = vec![core_pos[1], core_pos[0], core_pos[2]];
        order.extend((0..radix.len()).filter(|i| !core_pos.contains(i)));
        let mut radices: Vec<u64> = order.iter().map(|&i| radix.get(i).copied().unwrap_or(1)).collect();
        radices.push(n_grid as u64);
        let vmap = values.iter().enumerate().map(|(i, v)| ((v.dial, v.value), i)).collect();
        Self { radices, order, core_pos, vmap }
    }

    /// The product of the dials' radices (the N digit's weight).
    pub(crate) fn n_weight(&self) -> Option<Code> {
        let mut acc = Code::default().plus(1)?;
        for &r in self.radices.iter().take(self.radices.len().saturating_sub(1)) {
            acc = acc.mul_add(r, 0)?;
        }
        Some(acc)
    }

    pub(crate) fn decode(&self, code: [u64; 4]) -> Result<Combo, String> {
        let d = Code(code).digits(&self.radices).ok_or("a code outside the radices")?;
        let g = |i: usize| d.get(i).copied().unwrap_or(0);
        let mut outer = None;
        for (pos, &dial) in self.order.iter().enumerate().skip(3) {
            let v = g(pos);
            if v != 0 {
                if outer.is_some() {
                    return Err("two outer values in an S1 code".to_string());
                }
                outer = Some(*self.vmap.get(&(dial, v)).ok_or_else(|| format!("dial {dial} value {v} not in this run's values"))?);
            }
        }
        let _ = self.core_pos;
        let u = |x: u64| u16::try_from(x).unwrap_or(u16::MAX);
        Ok(Combo { ni: usize::try_from(g(self.radices.len().saturating_sub(1))).unwrap_or(usize::MAX), cci: u(g(0)), rsi: u(g(1)), wr: u(g(2)), outer })
    }
}

/// The FIRST-mode trades of `cb` over every row (learning and test): per session the first row (in candle order) passing the
/// core cell and the outer value. The same rule as the kernel's LINE engine, computed directly (the re-walk).
pub(crate) fn combo_rows(krows: &[Row], kept: Option<&Bits>, core: Core, cb: &Combo) -> Vec<usize> {
    let pass = |r: &Row| {
        r.rb.min(core.nr.saturating_sub(1)) >= cb.rsi && r.wb.min(core.nw.saturating_sub(1)) >= cb.wr && r.cb.min(core.nc.saturating_sub(1)) >= cb.cci
    };
    let mut out = Vec::new();
    let mut last = u32::MAX;
    let mut one = |j: usize| {
        if let Some(r) = krows.get(j)
            && r.sess != last
            && pass(r)
        {
            out.push(j);
            last = r.sess;
        } else if cfg!(feature = "m_rw")
            && let Some(r) = krows.get(j)
            && r.sess == last
            && pass(r)
            && let Some(x) = out.last_mut()
        {
            // MUTANT m_rw: the re-walk keeps the LATER qualifying candle of a session (REWALK-EQ must catch it).
            *x = j;
        }
    };
    match kept {
        Some(b) => b.ones().for_each(&mut one),
        None => (0..krows.len()).for_each(&mut one),
    }
    out
}

/// The priced outcome table of one context: per N, per row; and per (N, row, level) the settlement second and the flags.
/// S7 BUILDER wave 4: crate-visible so the index seam (`crate::idx`) returns the same shape (the cash path builds it unchanged).
pub(crate) struct Table {
    pub(crate) kidx: Vec<usize>,
    pub(crate) outs: Vec<Vec<Outcome>>,
    pub(crate) close: Vec<Vec<u32>>,
    pub(crate) fl: Vec<Vec<u16>>,
    pub(crate) entry_first: Vec<i64>,
    /// SNIPER s16 trade rows: per row the size reference (paisa).
    pub(crate) refpx: Vec<i64>,
    /// SNIPER s16 trade rows: per N, per row, the sized count (taken, the rule's own, cut by the account base).
    pub(crate) sized: Vec<Vec<crate::size::Sized>>,
    /// SNIPER s16 trade rows: per N, per (row, level), (units entered, entry value, closing value) in paisa.
    pub(crate) qv: Vec<Vec<(u64, i64, i64)>>,
    pub(crate) nl: usize,
    pub(crate) records: Vec<u8>,
    pub(crate) refusals: BTreeMap<String, u64>,
    pub(crate) rewalks: u64,
    pub(crate) price_s: f64,
}

impl Table {
    /// S7 BUILDER wave 4: an empty table of `nn` N (the index seam fills it with [`Table::push_row`]).
    pub(crate) fn empty(nn: usize, price_s: f64) -> Self {
        Self {
            kidx: Vec::new(),
            outs: vec![Vec::new(); nn],
            close: vec![Vec::new(); nn],
            fl: vec![Vec::new(); nn],
            entry_first: Vec::new(),
            refpx: Vec::new(),
            sized: vec![Vec::new(); nn],
            qv: vec![Vec::new(); nn],
            nl: 0,
            records: Vec::new(),
            refusals: BTreeMap::new(),
            rewalks: 0,
            price_s,
        }
    }

    /// S7 BUILDER wave 4: one priced row (candidate `i` of session / group `sess`, booking month `mk`), exactly as `price_all` adds a
    /// cash row.
    pub(crate) fn push_row(&mut self, sess: u32, mk: i32, i: usize, p: &RowOut, nn: usize) {
        for ni in 0..nn {
            let mut o = Outcome { sess, mk, id: i as u64, ..Outcome::default() };
            for l in 0..5 {
                let (net, rd, close, f) = p.cells.get(ni * 5 + l).copied().unwrap_or_default();
                if let (Some(n), Some(x)) = (o.net.get_mut(l), o.rd_bp.get_mut(l)) {
                    *n = net;
                    *x = rd;
                }
                if let (Some(cl), Some(fv)) = (self.close.get_mut(ni), self.fl.get_mut(ni)) {
                    cl.push(close);
                    fv.push(f);
                }
            }
            if let Some(v) = self.outs.get_mut(ni) {
                v.push(o);
            }
            if let (Some(sv), Some(s)) = (self.sized.get_mut(ni), p.sized.get(ni)) {
                sv.push(*s);
            }
            if let Some(qv) = self.qv.get_mut(ni) {
                qv.extend(p.qv.get(ni * 5..ni * 5 + 5).unwrap_or(&[]));
            }
        }
        self.refpx.push(p.refpx);
        self.rewalks += u64::from(p.rewalks);
        self.entry_first.push(p.entry_first);
        self.kidx.push(i);
        self.records.extend_from_slice(&p.rec);
    }
}

/// S7 BUILDER wave 4: who prices a context's candidates. `Cash` is host v3's `price_all` (byte for byte, proof P2); `Idx` is the
/// index seam (`crate::idx::price_all_idx`: the S8b pricer on the S5 pack).
pub(crate) enum Pricer<'a> {
    /// ADANIENT cash (REQ 70, 2-s blocks).
    Cash,
    /// NIFTY / BANKNIFTY options, intraday.
    Idx(&'a crate::idx::IdxEnv),
}

/// One priced row in its compact form (made inside the pricing worker, so a full `Priced` never outlives its row).
pub(crate) struct RowOut {
    pub(crate) rec: Vec<u8>,
    /// Per (N, level): net, run-down bp, settlement second of the IST day, flags.
    pub(crate) cells: Vec<(i64, u16, u32, u16)>,
    pub(crate) rewalks: u32,
    pub(crate) entry_first: i64,
    /// SNIPER s16: the size reference, the sized count per N, and per (N, level) (units, entry value, closing value).
    pub(crate) refpx: i64,
    pub(crate) sized: Vec<crate::size::Sized>,
    pub(crate) qv: Vec<(u64, i64, i64)>,
}

pub(crate) fn compact(p: &Priced, day: i32, nn: usize, origin: i64) -> Result<RowOut, String> {
    let rec = price::encode(p, 5, origin).map_err(|e| format!("{e:?}"))?;
    let day0 = i64::from(day) * 86_400 - price::IST_S;
    let mut cells = Vec::with_capacity(nn * 5);
    let mut qv = Vec::with_capacity(nn * 5);
    for ni in 0..nn {
        let lvs = p.lv.get(ni).ok_or("a priced row without every N")?;
        for l in 0..5 {
            let x = lvs.get(l).copied().unwrap_or_default();
            let sod = if x.exit_known == 0 { 0 } else { (x.exit_known / US_S - day0).clamp(0, 86_399) };
            cells.push((x.net, x.rd_bp, u32::try_from(sod).unwrap_or(0), x.flags));
            qv.push((x.units, x.entry_value, x.exit_value));
        }
    }
    if p.sized.len() != nn {
        return Err("a priced row without a sized count at every N".to_string());
    }
    Ok(RowOut { rec, cells, rewalks: p.rewalks, entry_first: p.entry_first, refpx: p.refpx, sized: p.sized.clone(), qv })
}

#[allow(clippy::too_many_arguments, reason = "one context's pricing inputs")]
fn price_all(fam: &Family, ctx: &feat_io::host::Ctx, cfg: &Cfg, exit: &ExitSet, long: bool, slots: &[i64], hull_to: i32, threads: usize, sym: &str) -> Result<Table, String> {
    let xe = match exit.kind {
        ExitKind::Ema(p) => Some(p),
        _ => None,
    };
    let mut by_sess: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (i, c) in fam.cands.iter().enumerate() {
        by_sess.entry(c.sess).or_default().push(i);
    }
    let sess_list: Vec<(u32, Vec<usize>)> = by_sess.into_iter().collect();
    let next = AtomicUsize::new(0);
    let nn = slots.len();
    let origin = price::at_sod(common_feat::cal::parse_ymd("2018-01-01").ok_or("origin")?, 0);
    let priced: Mutex<Vec<(usize, Result<RowOut, String>)>> = Mutex::new(Vec::new());
    let ticks = Path::new(feat_io::defaults::TICKS);
    let tp = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                let mut local = Vec::new();
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some((sess, rows)) = sess_list.get(k) else { break };
                    let Some(se) = ctx.sessions.get(*sess as usize) else { continue };
                    let day = match day_tape(ticks, sym, se) {
                        Ok(Some(d)) => d,
                        Ok(None) => {
                            local.extend(rows.iter().map(|&i| (i, Err("no_tick_member".to_string()))));
                            continue;
                        }
                        Err(e) => {
                            local.extend(rows.iter().map(|&i| (i, Err(format!("tick_day: {e}")))));
                            continue;
                        }
                    };
                    let sq = price::at_sod(se.day, cfg.square_off_sod);
                    for &i in rows {
                        let Some(c) = fam.cands.get(i) else { continue };
                        if xe.is_some_and(|p| !cands::xe_with(c, long, p)) {
                            local.push((i, Err("xe_filter".to_string())));
                            continue;
                        }
                        let r = plan_of(fam, i, long, exit, sq).map_err(str::to_string).and_then(|plan| {
                            price::price(&day, cfg, c.end * US_S, long, plan, slots).map_err(|e| match e {
                                Refusal::Fill(x) => format!("fill: {x}"),
                                Refusal::Charges(x) => format!("charges: {x}"),
                                other => format!("{other:?}"),
                            })
                        });
                        local.push((i, r.and_then(|p| compact(&p, c.day, nn, origin))));
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
    let price_s = tp.elapsed().as_secs_f64();
    let mut t = Table {
        kidx: Vec::new(),
        outs: vec![Vec::new(); nn],
        close: vec![Vec::new(); nn],
        fl: vec![Vec::new(); nn],
        entry_first: Vec::new(),
        refpx: Vec::new(),
        sized: vec![Vec::new(); nn],
        qv: vec![Vec::new(); nn],
        nl: 0,
        records: Vec::new(),
        refusals: BTreeMap::new(),
        rewalks: 0,
        price_s,
    };
    let mut seen_test = false;
    for (i, r) in priced {
        let p = match r {
            Err(e) => {
                *t.refusals.entry(e.split(':').next().unwrap_or("?").to_string()).or_insert(0) += 1;
                continue;
            }
            Ok(p) => p,
        };
        let Some(c) = fam.cands.get(i) else { continue };
        if c.mk > hull_to {
            seen_test = true;
        } else if seen_test {
            return Err("a learning row after a test row: the learning rows are not a prefix".to_string());
        } else {
            t.nl += 1;
        }
        for ni in 0..nn {
            let mut o = Outcome { sess: c.sess, mk: c.mk, id: i as u64, ..Outcome::default() };
            for l in 0..5 {
                let (net, rd, close, f) = p.cells.get(ni * 5 + l).copied().unwrap_or_default();
                if let (Some(n), Some(x)) = (o.net.get_mut(l), o.rd_bp.get_mut(l)) {
                    *n = net;
                    *x = rd;
                }
                if let (Some(cl), Some(fv)) = (t.close.get_mut(ni), t.fl.get_mut(ni)) {
                    cl.push(close);
                    fv.push(f);
                }
            }
            if let Some(v) = t.outs.get_mut(ni) {
                v.push(o);
            }
            if let (Some(sv), Some(s)) = (t.sized.get_mut(ni), p.sized.get(ni)) {
                sv.push(*s);
            }
            if let Some(qv) = t.qv.get_mut(ni) {
                qv.extend(p.qv.get(ni * 5..ni * 5 + 5).unwrap_or(&[]));
            }
        }
        t.refpx.push(p.refpx);
        t.rewalks += u64::from(p.rewalks);
        t.entry_first.push(p.entry_first);
        t.kidx.push(i);
        t.records.extend_from_slice(&p.rec);
    }
    Ok(t)
}

/// A pick's own figures on its window's TEST months: per calendar year net, every test year positive, the test net.
pub(crate) fn test_figures(trades: &[(i32, i64)], test: (i32, i32)) -> (bool, i64, u32) {
    let mut years: BTreeMap<i32, (i64, u32)> = BTreeMap::new();
    for y in test.0.div_euclid(12)..=test.1.div_euclid(12) {
        years.insert(y, (0, 0));
    }
    let mut net = 0_i64;
    let mut n = 0_u32;
    for &(mk, x) in trades {
        if mk >= test.0 && mk <= test.1 {
            if let Some(e) = years.get_mut(&mk.div_euclid(12)) {
                e.0 = e.0.saturating_add(x);
                e.1 += 1;
            }
            net = net.saturating_add(x);
            n += 1;
        }
    }
    (years.values().all(|(v, k)| *k > 0 && *v > 0), net, n)
}

pub(crate) fn pct_of(v: &[i64], permille: usize) -> i64 {
    if v.is_empty() {
        return 0;
    }
    let i = (v.len() - 1) * permille / 1000;
    v.get(i).copied().unwrap_or(0)
}

/// What one context reported (folded into the run's lines).
pub(crate) struct CtxOut {
    pub(crate) lines: Vec<String>,
    pub(crate) searches: u64,
    pub(crate) wf_passed: u64,
    pub(crate) tt_passed: u64,
    pub(crate) both: u64,
    pub(crate) mismatches: u64,
}

/// Runs the family.
///
/// # Errors
/// A held gate, a refused input, or an I/O error.
#[allow(clippy::too_many_lines, reason = "the gate-7 pipeline of one family in one place")]
pub fn run_family(inp: &Inputs, spec: &RunSpec) -> Result<Vec<String>, String> {
    crate::refuse_if_red()?;
    if crate::sec::tf_secs(&spec.tf).is_some() {
        // SECONDS BUILDER (2026-11-14): the seconds families run their own driver (compact rows, design change (d)).
        return crate::secrun::run_family_sec(inp, spec);
    }
    if spec.period.is_some() {
        return Err("--from / --to restrict the seconds families only (a minute run reads the whole period; named)".to_string());
    }
    // S7 BUILDER wave 4: an INDEX OPTIONS row set runs the index family (crate::idx); CASH continues below, byte for byte.
    if inp.instr.kind == crate::instr::Kind::IdxOpt {
        return crate::idx::run_family_idx(inp, spec);
    }
    let run = &inp.run;
    let mut lines = Vec::new();
    let gate = crate::monitor::launch_gate(run, spec.swap_base_mb, &spec.out, 8)?;
    lines.push(format!("{} gate ok={} hold={:?} pressure={:?} swap_used={:?} swap_free={:?} disk_free={:?}", crate::ist_now(), gate.ok, gate.hold, gate.pressure, gate.swap_used_mb, gate.swap_free_mb, gate.disk_free_gib));
    if !gate.ok {
        return Err(format!("launch gate HOLDS: {:?}", gate.hold));
    }
    let slice = spec.slice.as_deref().map(Slice::load).transpose()?;
    let db = Db::open(&spec.out.join("db"), crate::VERSION)?;
    let hb = spec.out.join("HEARTBEAT.tsv");
    let beat = |stage: &str, extra: String| {
        let _ = crate::monitor::heartbeat(&hb, &[("stage", stage.to_string()), ("rss_mb", format!("{:?}", crate::monitor::rss_mb())), ("detail", extra)]);
    };
    let t0 = Instant::now();
    // --- readings (once for the family) ---
    let cont = common_feat::cal::parse_ymd("2018-09-07").ok_or("cont start")?;
    let ctx = feat_io::load_ctx(Path::new(feat_io::defaults::DATES), Path::new(feat_io::defaults::BHAV), cont)?;
    let store = Path::new(feat_io::defaults::STORE);
    let (stream, st) = feat_io::minute_stream(store, inp.instr.underlying(), &spec.tf, &ctx);
    if stream.tf_s < 60 {
        return Err(format!("{}: the seconds families are not wired in host v1 (named)", spec.tf));
    }
    let (s1, _) = feat_io::minute_stream(store, inp.instr.underlying(), "1min", &ctx);
    let facts = feat_io::host::day_facts(&s1, &ctx);
    let lv = feat_io::host::levels(&ctx, &facts);
    let cut = run.hms("last_decision_before_ist")?;
    let mut fam = cands::build(&ctx, &stream, &lv, (&inp.core.0, &inp.core.1), cut)?;
    lines.push(format!("readings: {} candles ({}), {} decision candidates, {} not entry {:?} (Session::entry_at at the candle END), K13 resets {}, K_tf {}, bars read {}, straddling candles kept {}", stream.candles.len(), stream.source, fam.cands.len(), fam.not_entry, fam.refused, fam.resets.len(), fam.kmax, st.bars, st.straddle_kept));
    // REQ 76e (2): the H-* dials read every OTHER timeframe of the minutes family (its last completed candle at S), folded
    // through common_feat once per run, only when the walked values hold an H-* dial.
    let wants_htf = inp.dials.iter().any(|d| d.id.starts_with("H-") && d.status.starts_with("LIVE") && slice.as_ref().is_none_or(|s| s.has_dial(&d.id)));
    if wants_htf {
        let th = Instant::now();
        let lad = cands::htf_ladders(&inp.dials)?;
        let fam_tfs: Vec<String> = run.list::<String>("minutes_family")?;
        let names: Vec<String> = fam_tfs.into_iter().filter(|x| *x != spec.tf).collect();
        let mut reads = Vec::with_capacity(names.len());
        for n in &names {
            let (s, _) = feat_io::minute_stream(store, inp.instr.underlying(), n, &ctx);
            if s.candles.is_empty() {
                return Err(format!("H-* dials: the {n} stream is empty"));
            }
            reads.push(cands::htf_reads(&ctx, &s, &lad)?);
        }
        cands::attach_htf(&mut fam, names.clone(), &reads);
        lines.push(format!(
            "higher timeframes for the H-* dials: {} ({:.1} s); reads refused across the K13 reset (MtfSlot::reset_at) per timeframe {:?}",
            names.join(", "),
            th.elapsed().as_secs_f64(),
            fam.htf_reset_refused
        ));
    }
    // DIALS BUILD: the seven context / calendar / range dials (K11 readings at every candidate), only in the ON mode and only when
    // the walked values hold one of them (as the H-* dials above). The OFF mode adds no line, no row and no read.
    let wants_ctx = spec.context_dials
        && inp.dials.iter().any(|d| crate::ctxdials::IDS.contains(&d.id.as_str()) && d.status.starts_with("LIVE") && slice.as_ref().is_none_or(|s| s.has_dial(&d.id)));
    let mut ctx_sn1: Vec<(String, String)> = Vec::new();
    if wants_ctx {
        let tx = Instant::now();
        let (l, rows) = crate::ctxdials::attach(&mut fam, &ctx, &stream, store)?;
        lines.extend(l);
        lines.push(format!("context dial readings attached in {:.1} s", tx.elapsed().as_secs_f64()));
        ctx_sn1 = rows;
    }
    let fam = fam;
    beat("readings", format!("{} candidates", fam.cands.len()));
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
    let sess_mk: Vec<i32> = ctx.sessions.iter().map(|s| {
        let (y, m, _) = common_feat::cal::civil(s.day);
        y * 12 + i32::try_from(m).unwrap_or(1) - 1
    }).collect();
    let wins: Vec<(String, i32, i32)> = inp.windows.rows.iter().map(|r| (r.0.clone(), r.1, r.2)).collect();
    let lay = Layout::new(&sess_mk, (hull_from, hull_to), &wins).map_err(|e| e.to_string())?;
    let twin_draws: u32 = run.num("twin_draws")?;
    let twin_seed: u64 = run.num("twin_seed")?;
    let wf_data = PathBuf::from(run.get("windows_file")?).parent().map(Path::to_path_buf).ok_or("windows_file has no folder")?;
    let part_tpl = crate::rows::read_with_sha(&spec.data.join("PART-SNIPER-v1.tsv").to_string_lossy())?;
    let labels = inp.words.standing(1, "printed by the tool").join(" | ");
    let mut total = CtxOut { lines: Vec::new(), searches: 0, wf_passed: 0, tt_passed: 0, both: 0, mismatches: 0 };
    let n_ctx = spec.sides.len() * spec.exits.len();
    let mut done_ctx = 0_usize;
    let tc = Instant::now();
    if let Some(l) = &spec.n_sweep {
        lines.push(format!("N swept: {l:?} of the grid {n_grid:?} (every N priced and stored; a pick's N is searched over the swept N only; named)"));
    }
    for &long in &spec.sides {
        for exit_id in &spec.exits {
            let exit = inp.exits.iter().find(|e| &e.id == exit_id).ok_or_else(|| format!("exit set {exit_id} not in the exits file"))?;
            let g = crate::monitor::launch_gate(run, spec.swap_base_mb, &spec.out, 1)?;
            if !g.ok {
                total.lines.push(format!("{} HOLD before {} {}: {:?} (named; the rest of the run waits for MAIN)", crate::ist_now(), if long { "LONG" } else { "SHORT" }, exit.id, g.hold));
                break;
            }
            let c = context(inp, spec, &db, &fam, &ctx, &cfg, exit, long, &slots, &n_grid, proxy_n, &lay, &cal, &wf_data, &part_tpl.0, slice.as_ref(), (twin_seed, twin_draws), &labels, (hull_from, hull_to), &Pricer::Cash)?;
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
            println!("{} context {done_ctx} of {n_ctx} done; {per} s a context so far; about {} min left at this pace (ESTIMATE)", crate::ist_now(), left / 60);
            beat("context", format!("{} {} done ({done_ctx} of {n_ctx}; about {} min left, ESTIMATE)", if long { "LONG" } else { "SHORT" }, exit.id, left / 60));
        }
    }
    lines.extend(total.lines);
    lines.push(format!(
        "SUMMARY {}: {} searches; walk-forward picks PASSED {} of {}; train/test picks PASSED {} of {}; both in the same search (REQ 73d) {} of {}; re-walk key mismatches {}",
        spec.tf, total.searches, total.wf_passed, total.searches, total.tt_passed, total.searches, total.both, total.searches, total.mismatches
    ));
    let mut sn1 = vec![
        vec![spec.run_id.clone(), "version".to_string(), crate::VERSION.to_string(), labels.clone()],
        vec![spec.run_id.clone(), "laws".to_string(), crate::law_line(), labels.clone()],
        vec![spec.run_id.clone(), "threads".to_string(), spec.threads.to_string(), labels.clone()],
        vec![spec.run_id.clone(), "gate".to_string(), format!("{gate:?}").replace('\n', " "), labels.clone()],
        vec![spec.run_id.clone(), "bar_source".to_string(), inp.words.line("BARSRC", &[]), labels.clone()],
        vec![spec.run_id.clone(), "stream_source".to_string(), stream.source.clone(), labels.clone()],
        vec![spec.run_id.clone(), "slice".to_string(), slice.as_ref().map_or_else(|| "ALL wired values".to_string(), |s| format!("{} sha256 {}", s.name, s.sha)), labels.clone()],
        vec![spec.run_id.clone(), "headline_partition_provenance".to_string(), "none (plan 6 item 7, D-S7)".to_string(), labels.clone()],
        vec![spec.run_id.clone(), "wall_s".to_string(), format!("{:.1}", t0.elapsed().as_secs_f64()), labels.clone()],
    ];
    for (f, s) in &inp.shas {
        sn1.push(vec![spec.run_id.clone(), format!("sha256 {f}"), s.clone(), labels.clone()]);
    }
    sn1.push(vec![spec.run_id.clone(), "sha256 PART-SNIPER-v1.tsv".to_string(), part_tpl.1.clone(), labels.clone()]);
    sn1.push(vec![spec.run_id.clone(), "qty_rule".to_string(), format!("{} ({})", inp.qty_rule.id(), inp.qty_rule.label()), labels.clone()]);
    // DIALS BUILD (PORTER: after xc's qty_rule row, so no row moves): the ON mode's 2 rows; none in the OFF mode.
    for (f, v) in &ctx_sn1 {
        sn1.push(vec![spec.run_id.clone(), f.clone(), v.clone(), labels.clone()]);
    }
    db.write(&format!("SN1-RUN-{}.tsv", spec.tf), &["run", "field", "value", "labels"], &sn1)?;
    let n = db.verify()?;
    lines.push(format!("DB: {} tables audited, chain verified; run folder {}", n, spec.out.display()));
    for l in inp.words.standing(1, "printed by the tool") {
        lines.push(l);
    }
    lines.push(format!("{} done in {:.1} s", crate::ist_now(), t0.elapsed().as_secs_f64()));
    beat("done", String::new());
    Ok(lines)
}

/// One context: price, sweep at every N, merge, re-walk through common_wf, report, twin, DB rows.
#[allow(clippy::too_many_arguments, clippy::too_many_lines, reason = "one context's pipeline in one place")]
pub(crate) fn context(
    inp: &Inputs,
    spec: &RunSpec,
    db: &Db,
    fam: &Family,
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
    pricer: &Pricer<'_>,
) -> Result<CtxOut, String> {
    let side = if long { "LONG" } else { "SHORT" };
    let cname = format!("{}|{side}|{}", spec.tf, exit.id);
    let mut lines = vec![format!("=== {} context {cname} ===", crate::ist_now())];
    let run = &inp.run;
    let ncol = 5_usize;
    let levels: Vec<u16> = run.list("levels_bp")?;
    // --- pricing: every candidate row of the period, every N, every level ---
    let (t, idx_facts) = match pricer {
        Pricer::Cash => (price_all(fam, ctx, cfg, exit, long, slots, hull.1, spec.threads, inp.instr.underlying())?, None),
        Pricer::Idx(env) => {
            let (t, f) = crate::idx::price_all_idx(env, fam, ctx, exit, long, slots, &levels, cfg.account, hull.1, spec.threads)?;
            (t, Some(f))
        }
    };
    let osha = db.write_bin(&format!("OUT-{}-{side}-{}.bin", spec.tf, exit.id), &t.records)?;
    let keys: Vec<(u32, u16)> = t.kidx.iter().filter_map(|&i| fam.cands.get(i).map(|c| (c.sess, c.k))).collect();
    db.write_bin(&format!("IX-{}-{side}-{}.bin", spec.tf, exit.id), &crate::query::build_index(&keys, ctx.sessions.len(), fam.kmax))?;
    // S7 BUILDER wave 4 (index runs only): the contract of every priced decision and the one-order gate F8 at its A2 quantity.
    if let Some(f) = &idx_facts {
        let stored: Vec<i64> = inp.instr.rows.get("stored_n").map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect()).ok_or("IDXOPT row stored_n missing")?;
        let (rows, ok, fail) = crate::idx::facts_rows(&spec.run_id, &cname, fam, f, n_grid, &stored, labels);
        db.write(&format!("IDX-ROWS-{}-{side}-{}.tsv", spec.tf, exit.id), &crate::idx::FACTS_HEADER, &rows)?;
        let mut us: Vec<u64> = f.iter().map(|x| x.price_us).collect();
        us.sort_unstable();
        let q = |p: usize| us.get((us.len().saturating_sub(1) * p) / 1000).copied().unwrap_or(0);
        lines.push(format!(
            "index contracts: {} priced decisions; one-order gate F8 at the A2 (Rs 5 L) quantity: {ok} fit one order, {fail} fail (IDX-ROWS-{}-{side}-{}.tsv); pricing per decision (the pick + {} N x 5 levels x 2 paths) p50 / p99 / max {} / {} / {} ms",
            f.len(),
            spec.tf,
            exit.id,
            n_grid.len(),
            crate::idxpos::ms3(q(500)),
            crate::idxpos::ms3(q(990)),
            crate::idxpos::ms3(us.last().copied().unwrap_or(0))
        ));
        if fail > 0 {
            return Err(format!("{cname}: the one-order gate F8 failed on {fail} decisions (never silent)"));
        }
    }
    // B2 (S3): the protection hold and, beside, the stop clip, counted over the context's priced rows (a row counts when any of its
    // (N, level) cells carries the flag; per pick the count is at the pick's own N and level, below).
    let any_flag = |f: u16| (0..t.kidx.len()).filter(|&j| t.fl.iter().any(|v| v.get(j * ncol..(j + 1) * ncol).is_some_and(|c| c.iter().any(|x| x & f != 0)))).count();
    lines.push(format!(
        "pricing: {} rows priced ({} learning, {} test), {} N x {} levels each; refused {:?}; {:.1} s wall on {} threads; S2 re-walks {}; Dhan market protection (REQ 70a) held units on {} rows at some (N, level); a block clipped by an order's stop on {} rows (beside); outcome rows {} B, sha {osha}; {}",
        t.kidx.len(),
        t.nl,
        t.kidx.len() - t.nl,
        n_grid.len(),
        ncol,
        t.refusals,
        t.price_s,
        spec.threads,
        t.rewalks,
        any_flag(flag::PROTECTED),
        any_flag(flag::CLIPPED),
        price::record_width(n_grid.len(), ncol),
        inp.qty_rule.label()
    ));
    // --- the lattice values of this context (over EVERY row; the sweep reads the learning prefix only) ---
    let krows = cands::kernel_rows(fam, long, &t.kidx);
    let xe = match exit.kind {
        ExitKind::Ema(p) => Some(p),
        _ => None,
    };
    let (mut values, wiring) = cands::outer_values(fam, &t.kidx, long, &inp.dials, xe);
    if let Some(s) = slice {
        values.retain(|v| inp.dials.get(v.dial).is_some_and(|d| s.keeps(&d.id, &v.label)));
    }
    let walked_values = values.iter().filter(|v| !v.empty).count() as u64;
    let core = cands::core_of(if long { &inp.core.0 } else { &inp.core.1 });
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
    // --- the sweep at every N (learning rows only), lists merged with the N digit ---
    let lrows = krows.get(..t.nl).unwrap_or(&[]);
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
    // Speed build: every swept N in ONE walk of the lattice (crate::sweep::run_all: each outer value walked once, its events
    // replayed into one sink per N); element j equals crate::sweep::run at the j-th swept N, read in the grid order below.
    let swept: Vec<usize> = n_grid.iter().enumerate().filter(|(_, n)| !spec.n_sweep.as_ref().is_some_and(|l| !l.contains(n))).map(|(ni, _)| ni).collect();
    let swept_outs: Vec<&[Outcome]> = swept.iter().map(|&ni| t.outs.get(ni).map(Vec::as_slice).ok_or("outcome table")).collect::<Result<_, _>>()?;
    let sctx0 = crate::sweep::Ctx { rows: lrows, out: swept_outs.first().copied().unwrap_or(&[]), lay, views: &inp.views, cols: ncol, core, sessions: ctx.sessions.len(), top, radix: &wiring.radix, core_pos, filter: None };
    let mut all = if swept_outs.is_empty() { Vec::new() } else { crate::sweep::run_all(&sctx0, &swept_outs, &values, spec.threads)? }.into_iter();
    for (ni, &n) in n_grid.iter().enumerate() {
        if spec.n_sweep.as_ref().is_some_and(|l| !l.contains(&n)) {
            continue;
        }
        let res = all.next().ok_or("outcome table")?;
        if !res.refused.is_empty() {
            return Err(format!("{cname} N={n}: the sweep refused {:?}", res.refused.iter().take(3).collect::<Vec<_>>()));
        }
        let shift = nw_code.mul_add(ni as u64, 0).ok_or("N digit overflow")?;
        for (m, l) in merged.iter_mut().zip(&res.lists) {
            m.merge(&l.shifted(shift.0).ok_or("shifted code overflow")?);
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
    let sweep_s = ts.elapsed().as_secs_f64();
    lines.push(format!("sweep at the swept N ({} of {}): {:.1} s wall on {} threads; accumulator event p50/p99/max {:?}; key read {:?}", spec.n_sweep.as_ref().map_or(n_grid.len(), Vec::len), n_grid.len(), sweep_s, spec.threads, acc_hist.summary(), key_hist.summary()));
    // --- the HARD gate (plan 8.4): per-event step counts identical at 1x / 10x / 100x (S0 on the WF-learning rows, the proxy N) ---
    let pn = n_grid.iter().position(|&n| n == proxy_n).unwrap_or(0);
    let wf_to = inp.windows.rows.first().map_or(hull.1, |r| r.2);
    let pout = t.outs.get(pn).ok_or("proxy outcomes")?;
    let base_rows: Vec<Row> = lrows.iter().zip(pout).filter(|(_, o)| o.mk <= wf_to).map(|(r, _)| *r).collect();
    let base_out: Vec<Outcome> = pout.iter().take(t.nl).filter(|o| o.mk <= wf_to).copied().collect();
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
    lines.push(format!("HARD gate (per-event step counts identical at 1x / 10x / 100x; S0 on the WF-learning rows at N = {proxy_n}): {} {sig:?}", if hard_ok { "PASS" } else { "FAIL" }));
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
                    ("family", "minutes".to_string()),
                ],
            )
        })
        .collect();
    let wf = wfio::load(&spec.data, wf_data, cal, &parts, members)?;
    let set = wf.ws.sets.get(wf.set).ok_or("window set")?;
    // --- re-walk every listed entry over the whole period; offer to common_wf's own Lists ---
    let mut cl = common_wf::Lists::new(&wf.views, set, &wf.rules, 0);
    let mut walked: BTreeMap<(usize, [u64; 4]), (Combo, Vec<usize>)> = BTreeMap::new();
    let mut mismatches = 0_u64;
    let mut mismatch_lines = Vec::new();
    let trades_of = |cb: &Combo, rows: &[usize], col: usize| -> Vec<Trade> {
        let out = t.outs.get(cb.ni);
        let close = t.close.get(cb.ni);
        let fl = t.fl.get(cb.ni);
        rows.iter()
            .filter_map(|&j| {
                let o = out?.get(j)?;
                let c = fam.cands.get(*t.kidx.get(j)?)?;
                let x = j * ncol + col;
                Some(Trade {
                    day: c.day,
                    exit_day: c.day,
                    close_s: close?.get(x).copied().unwrap_or(0),
                    net: o.net.get(col).copied().unwrap_or(0),
                    net_alt: 0,
                    rd_bp: i32::from(o.rd_bp.get(col).copied().unwrap_or(0)),
                    hole: fl?.get(x).copied().unwrap_or(0) & flag::HOLE != 0,
                    id: o.id,
                    mark_prev: 0,
                })
            })
            .collect()
    };
    let walk = |trades: &[Trade]| -> Result<Acc, String> {
        let mut acc = Acc::new(set, &wf.views, &wf.rules);
        acc.reset();
        for tr in trades {
            acc.add(tr).map_err(|e| format!("acc add: {e}"))?;
        }
        acc.finish().map_err(|e| format!("acc finish: {e}"))?;
        Ok(acc)
    };
    for (vi, v) in inp.views.iter().enumerate() {
        let cv = wf.views.views.iter().position(|x| x.id == v.id).ok_or_else(|| format!("view {} is not a common_wf VIEWS-STD row", v.id))?;
        let cview = wf.views.views.get(cv).ok_or("view")?;
        for col in 0..ncol {
            let Some(l) = merged.get(vi * ncol + col) else { continue };
            for (key, code, n) in l.entries() {
                let cb = dec.decode(code)?;
                let kept = cb.outer.and_then(|i| values.get(i)).map(|x| &x.kept);
                let rows = combo_rows(&krows, kept, core, &cb);
                let trades = trades_of(&cb, &rows, col);
                let acc = walk(&trades)?;
                // rewalk_equal: the sweep's key parts against common_wf's crit values of this view on the re-walk.
                let lr = acc.learn(cview.win).map_err(|e| format!("learn: {e}"))?;
                let vals: Vec<i64> = cview.key.iter().map(|c| crit_val(c, &lr, &acc, wf.rules.base_capital, false, &[]).unwrap_or(i64::MIN)).collect();
                if key.0.get(..vals.len()) != Some(vals.as_slice()) {
                    mismatches += 1;
                    if mismatch_lines.len() < 5 {
                        mismatch_lines.push(format!("rewalk_equal FAILED {} level {} code {}: sweep {:?} common_wf {:?}", v.id, col, Code(code).hex(), key.0.get(..vals.len()), vals));
                    }
                }
                if walked.contains_key(&(col, code)) {
                    continue;
                }
                let mut tk = Tickets::new();
                tk.push(Ticket { grid: 0, stage: u8::from(cb.outer.is_some()), set: ReadSet::EMPTY }, &wf.rules).map_err(|e| format!("ticket: {e}"))?;
                let kmax = wf.views.views.iter().map(|x| x.k as u64).max().unwrap_or(1);
                let pid = Partition(u32::try_from(col + 1).unwrap_or(1));
                for i in 0..n.min(kmax) {
                    let c2 = Code(code).plus(i).ok_or("code overflow")?;
                    cl.offer(&wf.views, pid, &tk, WfCode(c2.0), &acc).map_err(|e| format!("offer: {e}"))?;
                }
                walked.insert((col, code), (cb, rows));
            }
        }
    }
    lines.push(format!("re-walk: {} distinct (level, combo) entries walked over the whole period and offered to common_wf; rewalk_equal mismatches {}", walked.len(), mismatches));
    lines.extend(mismatch_lines);
    // --- the report: common_wf's words, per search (= per share level) ---
    let combo_words = |cb: &Combo| -> String {
        let lad = if long { &inp.core.0 } else { &inp.core.1 };
        let lv = |li: usize, j: u16| if j == 0 { "OFF".to_string() } else { lad.l.get(li).map_or_else(String::new, |x| x.level_text(usize::from(j))) };
        let outer = cb.outer.and_then(|i| values.get(i)).map_or_else(|| "every other dial OFF".to_string(), |x| format!("{} (every other dial OFF)", x.label));
        format!("RSI 14 {}; CCI 20 {}; %R 14 {}; {}; N = {}", lv(0, cb.rsi), lv(1, cb.cci), lv(2, cb.wr), outer, n_grid.get(cb.ni).copied().unwrap_or(0))
    };
    let mut pick_rows = Vec::new();
    let mut report_rows = Vec::new();
    let mut trade_rows = Vec::new();
    let mut twin_rows = Vec::new();
    let mut co = CtxOut { lines: Vec::new(), searches: 0, wf_passed: 0, tt_passed: 0, both: 0, mismatches };
    let honest = wf.ws.honest();
    let sniper_ids: Vec<&str> = inp.views.iter().map(|v| v.id.as_str()).collect();
    // twin slots: every session with a priced row in this context.
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
    for (col, bp) in levels.iter().enumerate() {
        let pid = u32::try_from(col + 1).unwrap_or(1);
        let p = wf.rules.partition(pid).map_err(|e| e.to_string())?.clone();
        let search = format!("{cname}|{}%", bp / 100);
        co.searches += 1;
        let mut out: Vec<String> = vec![match pricer {
            Pricer::Cash => format!("--- search {search} ({}; its own {} candles; cash intraday 5x; N slots of Rs 20 L / N; {}; 2018-09..2026-10; run {}) ---", inp.instr.underlying(), spec.tf, inp.qty_rule.label(), spec.run_id),
            Pricer::Idx(_) => format!("--- search {search} ({}; its own {} index candles; {}; N slots of Rs 20 L / N, headline N {}; {}; 2018-09..2026-10; run {}) ---", inp.instr.underlying(), spec.tf, inp.words.line("INSTR", &[]), inp.instr.rows.get("headline_n").map_or("?", String::as_str), inp.qty_rule.label(), spec.run_id),
        }];
        // SNIPER s16 + base Q7: one line per pick, printed after the search's METHOD lines (so no earlier line moves).
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
            let rows = walked.get(&(col, top.key.code.0)).map(|x| x.1.clone()).or_else(|| {
                // a later code of an offered run: the run's first code holds the same trades
                let kept_bits = cb.outer.and_then(|i| values.get(i)).map(|x| &x.kept);
                Some(combo_rows(&krows, kept_bits, core, &cb))
            }).unwrap_or_default();
            let trades = trades_of(&cb, &rows, col);
            let acc = walk(&trades)?;
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
            let n_open = common_wf::report::open_at_learn_end(&trades, learn.1);
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
            // HOST REPAIR B2 (S3: "Dhan's market protection ... counted per pick"): at the pick's own N and share level.
            let (k_prot, k_clip) = price::held_counts(rows.iter().map(|&j| t.fl.get(cb.ni).and_then(|f| f.get(j * ncol + col)).copied().unwrap_or(0)));
            out.push(format!(
                "{} protection: Dhan market protection (REQ 70a, {}% limit) held units on {k_prot} of {} trades (a block of the entry or the exit order printed past the protection limit while units were open; the crate's past_limit); a block clipped by an order's stop on {k_clip} of {} trades (the crate's clipped; beside)",
                v.id,
                cfg.protection_bp / 100,
                trades.len(),
                trades.len()
            ));
            // SNIPER s16 + base Q7: the pick's own account walk at its N (never more money than the account has free at that instant).
            let slot = slots.get(cb.ni).copied().unwrap_or(0);
            let acct_in: Vec<(u64, i64, i64)> = trades.iter().zip(&rows).map(|(x, &j)| (t.sized.get(cb.ni).and_then(|v| v.get(j)).map_or(0, |s| s.qty), t.refpx.get(j).copied().unwrap_or(0), x.net)).collect();
            let acct = crate::size::account_walk(cfg.account, &acct_in, cfg.lev);
            let k_cut = acct.iter().filter(|a| a.2.is_some()).count();
            let k_static = rows.iter().filter(|&&j| t.sized.get(cb.ni).and_then(|v| v.get(j)).is_some_and(|s| s.capped)).count();
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
            // P23's luck counts are the SWEEP's (every code offered to this view and level across every N), never the re-walked shortlist's.
            let sv = inp.views.iter().position(|x| x.id == v.id).unwrap_or(0);
            let lc0 = common_wf::report::LuckCtx { eligible: merged.get(sv * ncol + col).map_or(0, |l| l.offered), offered: cells_offered, views: inp.views.len() as u64, partitions: wf.rules.partitions.len() as u64, twin: None, mirror: None };
            let luck = |tw: Option<(u64, u64)>| -> Result<String, String> {
                let lc = common_wf::report::LuckCtx { twin: tw, ..lc0.clone() };
                common_wf::report::luck_line(&wf.words, set, v.win, &trades, &acc.facts(Partition(pid)), &lc).map_err(|e| e.to_string())
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
                // --- the random-entry twin (SNIPER s3 (5); plan 9 F10): the same entries per calendar year, test months only ---
                let test = (row.test_from.unwrap_or(0), row.test_to.unwrap_or(-1));
                let own: Vec<(i32, i64)> = trades.iter().map(|x| (common_wf::date::mk_of_day(x.exit_day), x.net)).collect();
                let (own_all, own_net, own_n) = test_figures(&own, test);
                let mut per_year: BTreeMap<i32, u32> = BTreeMap::new();
                for x in &trades {
                    *per_year.entry(common_feat::cal::civil(x.day).0).or_insert(0) += 1;
                }
                let out_n = t.outs.get(cb.ni).ok_or("outcomes")?;
                let mut nets = Vec::with_capacity(twin.1 as usize);
                let (mut k_net, mut k_all) = (0_u32, 0_u32);
                for world in 0..twin.1 {
                    let d = crate::twin::draws(twin.0, world, &tslots, &per_year);
                    let tw: Vec<(i32, i64)> = d
                        .iter()
                        .filter_map(|(s, c)| {
                            let j = *sess_rows.get(s)?.get(usize::from(c.saturating_sub(1)))?;
                            let o = out_n.get(j)?;
                            Some((o.mk, o.net.get(col).copied().unwrap_or(0)))
                        })
                        .collect();
                    let (all, net, _) = test_figures(&tw, test);
                    k_net += u32::from(net >= own_net);
                    k_all += u32::from(all);
                    nets.push(net);
                }
                nets.sort_unstable();
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
                for ((x, &j), a) in trades.iter().zip(&rows).zip(&acct) {
                    let c = t.kidx.get(j).and_then(|&i| fam.cands.get(i));
                    let sz = t.sized.get(cb.ni).and_then(|v| v.get(j)).copied().unwrap_or_default();
                    let (units, ev, xv) = t.qv.get(cb.ni).and_then(|v| v.get(j * ncol + col)).copied().unwrap_or_default();
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
                        c.map_or(0, |c| c.k).to_string(),
                        t.entry_first.get(j).map_or(0, |e| e / US_S).to_string(),
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
        // common_wf repair 2 (B1-R, RF7): the method line reads the run's OWN honest rows from its window sets and views; the
        // host passes only its items (one per honest row in use, each its row's HEADLINE view, of this search's partition).
        if let Some(ml) = common_wf::report::method_line(&wf.words, &items, &wf.ws, &wf.views, &p, &wf.rules).map_err(|e| e.to_string())? {
            for m in ml {
                out.push(format!("METHOD: {m} ({})", inp.qty_rule.label()));
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

/// The key of a list entry as printed (the parts in order).
#[must_use]
pub fn key_text(k: &Key) -> String {
    k.0.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
}

#[cfg(test)]
mod tests {
    //! Red first: REWALK-EQ (the pick re-walk equals the kernel's LINE engine in FIRST mode on every offered cell; mutant `m_rw`
    //! fails it), the merged-code decoder round trip (N above the dials' digits), and the slice rows.
    use std::collections::BTreeSet;

    use sniper_kernel::bits::Bits;
    use sniper_kernel::code::Code;
    use sniper_kernel::line::{Cell, Core, LineEngine, Row, Sink, WalkStats};
    use sniper_kernel::scale::splitmix;

    use super::{Combo, Decoder, Slice, combo_rows};
    use crate::cands::OuterValue;

    #[derive(Default)]
    struct Rec {
        cur: BTreeSet<u32>,
        offers: Vec<(Cell, u32, Vec<usize>)>,
    }
    impl Sink for Rec {
        type Err = ();
        fn reset(&mut self) {
            self.cur.clear();
        }
        fn insert(&mut self, row: u32) -> Result<(), ()> {
            if self.cur.insert(row) { Ok(()) } else { Err(()) }
        }
        fn remove(&mut self, row: u32) -> Result<(), ()> {
            if self.cur.remove(&row) { Ok(()) } else { Err(()) }
        }
        fn offer(&mut self, cell: Cell, run: u32) -> Result<(), ()> {
            self.offers.push((cell, run, self.cur.iter().map(|&x| x as usize).collect()));
            Ok(())
        }
    }

    #[test]
    fn rewalk_equals_the_line_engine_first_mode() {
        let core = Core { nr: 6, nc: 9, nw: 5 };
        let mut checked = 0;
        for seed in 1..=12_u64 {
            let mut st = seed * 977;
            let mut rows = Vec::new();
            for s in 0..40_u32 {
                for k in 1..=7_u16 {
                    st = splitmix(st);
                    let p = |x: u64, n: u16| u16::try_from(if x.is_multiple_of(3) { x % u64::from(n) } else { (x % 3).min(u64::from(n) - 1) }).unwrap();
                    rows.push(Row { sess: s, k: u32::from(k), rb: p(st, core.nr), cb: p(st >> 9, core.nc), wb: p(st >> 21, core.nw) });
                }
            }
            let mut kept = Bits::new(rows.len());
            for i in 0..rows.len() {
                if !splitmix(seed ^ ((i as u64) << 3)).is_multiple_of(4) {
                    kept.set(i);
                }
            }
            let mut eng = LineEngine::new(40, core);
            let mut sink = Rec::default();
            eng.walk(&rows, kept.ones(), &mut sink, &mut WalkStats::default()).unwrap();
            for (cell, run, set) in &sink.offers {
                for c in cell.c..cell.c + u16::try_from(*run).unwrap() {
                    let cb = Combo { ni: 0, rsi: cell.r, cci: c, wr: cell.w, outer: None };
                    assert_eq!(&combo_rows(&rows, Some(&kept), core, &cb), set, "seed {seed} cell r{} c{c} w{}", cell.r, cell.w);
                    checked += 1;
                }
            }
        }
        assert!(checked > 1000, "only {checked} cells checked");
    }

    #[test]
    fn merged_codes_decode_to_their_combo_and_n() {
        // dials in file order: M-RSI (6), X (4), M-CCI (9), Y (3), M-WR (5); core positions RSI 0, CCI 2, WR 4.
        let radix = [6_u64, 4, 9, 3, 5];
        let core_pos = [0, 2, 4];
        let values = vec![
            OuterValue { dial: 1, value: 2, label: "X=2".into(), kept: Bits::new(1), empty: false },
            OuterValue { dial: 3, value: 1, label: "Y=1".into(), kept: Bits::new(1), empty: false },
        ];
        let dec = Decoder::new(&radix, core_pos, &values, 11);
        let w = dec.n_weight().unwrap();
        for (ni, outer, (r, c, wr)) in [(0_usize, None, (0_u64, 0_u64, 0_u64)), (3, Some(0_usize), (5, 8, 4)), (10, Some(1), (2, 1, 3))] {
            // the sweep's code: digits CCI, RSI, WR, X, Y (least significant first), then N on top
            let (xd, yd) = match outer {
                Some(0) => (2, 0),
                Some(1) => (0, 1),
                _ => (0, 0),
            };
            let base = Code::mixed(&[(c, 9), (r, 6), (wr, 5), (xd, 4), (yd, 3)]).unwrap();
            let code = base.0;
            let shifted = w.mul_add(ni as u64, 0).unwrap();
            let mut sum = [0_u64; 4];
            let mut carry = 0_u128;
            for i in 0..4 {
                let s = u128::from(code[i]) + u128::from(shifted.0[i]) + carry;
                sum[i] = u64::try_from(s & u128::from(u64::MAX)).unwrap();
                carry = s >> 64;
            }
            let cb = dec.decode(sum).unwrap();
            assert_eq!(cb, Combo { ni, rsi: u16::try_from(r).unwrap(), cci: u16::try_from(c).unwrap(), wr: u16::try_from(wr).unwrap(), outer });
        }
        assert!(dec.decode(Code::mixed(&[(0, 9), (0, 6), (0, 5), (1, 4), (1, 3)]).unwrap().0).is_err(), "two outer values refused");
    }

    #[test]
    fn slice_rows_keep_their_labels_and_mirrors() {
        let dir = std::env::temp_dir().join(format!("sniper-slice-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("S.tsv");
        std::fs::write(&p, "# c\ndial\tlabel\tsource\nC-IDX\tALL\tx\nL-PIV\tL-PIV=ABOVE_R1\tx\n").unwrap();
        let s = Slice::load(&p).unwrap();
        assert!(s.keeps("C-IDX", "C-IDX=7"));
        assert!(s.keeps("L-PIV", "L-PIV=ABOVE_R1"));
        assert!(s.keeps("L-PIV", "L-PIV=ABOVE_R1 (mirror)"));
        assert!(!s.keeps("L-PIV", "L-PIV=ABOVE_P"));
        assert!(!s.keeps("C-GAP", "C-GAP GE 0"));
    }
}
