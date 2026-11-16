//! One CONTEXT end to end (plan 8.4 E-SN1; gate 6): (timeframe, side, exit set) on ADANIENT: readings, candidates, pricing of every
//! TT-learning row through crate v2, the outcome partition, S0 + S1 over the worker threads, the DB rows with their audit, the
//! heartbeat, and the measured unit costs that replace the plan's estimates. The minute families only in host v1 (the seconds
//! families need common_feat's GDFL-seconds folds wired: named on every result).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use common_fill::{Dial, SecIn, US_S};
use wf_sweep::acc::{Layout, Outcome};
use wf_sweep::views::View;

use crate::cands::{self, Family};
use crate::db::Db;
use crate::price::{self, Cfg, Day, Plan, Priced, Refusal};
use crate::rows::{self, CoreLadders, DialRow, ExitKind, ExitSet, RunRows, Windows};
use crate::size::QtyRule;
use crate::words::Words;

/// Every data row a run reads, with the sha of every file.
pub struct Inputs {
    /// RUN rows (SNIPER-RUN-v5.tsv: v3 + `qty_rule`, SNIPER s16).
    pub run: RunRows,
    /// SNIPER s16: the sizing rule (RUN row `qty_rule`; read fail closed at load).
    pub qty_rule: QtyRule,
    /// WORDS rows.
    pub words: Words,
    /// VIEWS rows.
    pub views: Vec<View>,
    /// WF and TT windows (common_wf's file).
    pub windows: Windows,
    /// The dial rows.
    pub dials: Vec<DialRow>,
    /// The core ladders (long, short).
    pub core: (CoreLadders, CoreLadders),
    /// The 21 base exit sets.
    pub exits: Vec<ExitSet>,
    /// (file, sha256) of every input.
    pub shas: Vec<(String, String)>,
    /// S7 (INDEX S7 builder): the instrument (SNIPER-INSTRUMENT-v1.tsv, law-checked at load; CASH only priced by this binary).
    pub instr: crate::instr::Instrument,
    /// S7: (file, sha256) of the instrument rows (printed by `identity`; a CASH run prints nothing new, so its results stay
    /// byte-identical to host v3, proof P2).
    pub instr_sha: (String, String),
}

/// Loads every data row (fail closed).
///
/// # Errors
/// The first refused file or row.
pub fn load_inputs(data_dir: &Path) -> Result<Inputs, String> {
    let p = |n: &str| data_dir.join(n).to_string_lossy().to_string();
    // SNIPER s16: the run rows are SNIPER-RUN-v5.tsv (v3 + the qty_rule row); a folder without it is refused (never a default).
    // SECONDS PORTER (secf, 2026-11-14): SNIPER-RUN-v6.tsv (= v5 + the seconds builder's rows seconds_family,
    // sec_read_chunk_sessions, sec_price_chunk_sessions, sec_cells_buffer, sec_hard_gate_rows_max) when the data folder holds it,
    // else v5 (printed in the identity lines and SN1 by its file name; the minute families read no new row). The seconds lane's v4
    // (v3 + the seconds rows, no qty_rule row) is never read.
    let run_file = if data_dir.join("SNIPER-RUN-v6.tsv").exists() { "SNIPER-RUN-v6.tsv" } else { "SNIPER-RUN-v5.tsv" };
    let (rt, rs) = rows::read_with_sha(&p(run_file)).map_err(|e| format!("{e} (SNIPER s16: the RUN rows with the qty_rule row are SNIPER-RUN-v5.tsv)"))?;
    let run = RunRows::parse(&rt)?;
    // S7 (INDEX S7 builder, plan s3.4): the instrument is a DATA row set, SNIPER-INSTRUMENT-v1.tsv beside the base RUN file (one
    // source per row: a key in both files is refused); the law table L1-L8 runs here, before any reading; an option row set is then
    // refused by name (S4 / S5 not built); a CASH row set must name the files the pinned readers read.
    let (itext, isha) = rows::read_with_sha(&p("SNIPER-INSTRUMENT-v1.tsv")).map_err(|e| format!("{e} (S7: the instrument is a data row set, SNIPER-INSTRUMENT-v1.tsv; never a code default)"))?;
    let irows = RunRows::parse(&itext)?;
    let instr = crate::instr::Instrument::from_rows(&irows)?;
    if let Some(k) = instr.rows.keys().find(|k| run.get(k).is_ok()) {
        return Err(format!("S7: row {k} is in both {run_file} and SNIPER-INSTRUMENT-v1.tsv (one source per row)"));
    }
    instr.require_built()?;
    // S7 BUILDER wave 4: the cash instrument's supporting files are checked for CASH only; an IDXOPT row set reads its own files
    // (crate::idx::IdxEnv, every one sha-printed) and its own words (WORDS-SNIPER-IDX-v1.tsv: the index label blocks, plan s4.6 / s5.1).
    if instr.kind == crate::instr::Kind::Cash {
        instr.check_cash_files(feat_io::defaults::BHAV, crate::ctxdials::EXPIRIES)?;
    }
    let qty_rule = QtyRule::parse(run.get("qty_rule")?)?;
    let words_file = if instr.kind == crate::instr::Kind::IdxOpt { "WORDS-SNIPER-IDX-v1.tsv" } else { "WORDS-SNIPER-v1.tsv" };
    let (wt, ws) = rows::read_with_sha(&p(words_file))?;
    let words = Words::parse(&wt)?;
    let (wint, wins) = rows::read_with_sha(run.get("windows_file")?)?;
    let windows = Windows::parse(&wint, &["WF", "TT"])?;
    let (vt, vs) = rows::read_with_sha(&p("VIEWS-SNIPER-v2.tsv"))?;
    let names: Vec<String> = windows.rows.iter().map(|r| r.0.clone()).collect();
    let views = wf_sweep::views::parse(&vt, &names).map_err(|e| e.to_string())?;
    let (dt, ds) = rows::read_with_sha(run.get("dials_file")?)?;
    let dials = rows::dial_rows(&dt)?;
    let core = rows::core_ladders(&dt)?;
    let (et, es) = rows::read_with_sha(run.get("exits_file")?)?;
    let exits = rows::exit_sets(&et)?;
    crate::law::load(run.get("fill_law")?)?;
    let pin = |p: &str| rows::read_with_sha(p).map(|x| x.1).unwrap_or_else(|e| format!("UNREADABLE {e}"));
    let fill_pin = pin("/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/common-fill-v2/SHA256SUMS");
    // S7 BUILDER wave 4: the feat pin is REPAIR 3 (= S4; every ADANIENT reading byte-identical to repair 2): its SHA256SUMS-r3.
    let feat_pin = pin(&format!("{}/../../pinned/w4m/common-feat-v1-r3/SHA256SUMS-r3", env!("CARGO_MANIFEST_DIR")));
    let wf_pin = pin("/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/sniper-host-v1/pinned/common-wf-v1-r2/SHA256SUMS");
    let shas = vec![
        ("common-fill-v2/SHA256SUMS (crate v2 pin e3b911350f56a5f9)".to_string(), fill_pin),
        ("pinned common-feat-v1-r3/SHA256SUMS-r3 (repair 3 = S4 of SNIPER-IDXOPT-PLAN-v3.1, b05fafb38dab24d0)".to_string(), feat_pin),
        ("pinned common-wf-v1-r2/SHA256SUMS (repair 2, 5e01f0cf8fc749c7)".to_string(), wf_pin),
        (run_file.to_string(), rs),
        (words_file.to_string(), ws),
        ("VIEWS-SNIPER-v2.tsv".to_string(), vs),
        (run.get("windows_file")?.to_string(), wins),
        (run.get("dials_file")?.to_string(), ds),
        (run.get("exits_file")?.to_string(), es),
    ];
    Ok(Inputs { run, qty_rule, words, views, windows, dials, core, exits, shas, instr, instr_sha: ("SNIPER-INSTRUMENT-v1.tsv".to_string(), isha) })
}

/// One context's request.
#[derive(Clone, Debug)]
pub struct Spec {
    /// The timeframe name (`3min` ...).
    pub tf: String,
    /// Long or short.
    pub long: bool,
    /// The exit set id.
    pub exit: String,
    /// The run folder.
    pub out: PathBuf,
    /// The run id.
    pub run_id: String,
    /// Worker threads (the RUN row unless overridden).
    pub threads: usize,
    /// The swap base (MB used when the lane started), for the gate.
    pub swap_base_mb: u64,
}

pub(crate) fn civil(day: i32) -> (i32, u32, u32) {
    common_feat::cal::civil(day)
}

/// The corrected GDFL seconds of one session day (DP-NEWPX's rule through `feat_io::ticks`: LTQ = 0 new-price rows kept; a
/// price-only second has volume 0, crate S2 reads it as no print).
pub(crate) fn day_tape(ticks: &Path, sym: &str, sess: &feat_io::data::Session) -> Result<Option<Day>, String> {
    let (y, m, d) = civil(sess.day);
    let member = format!("{sym}.NSE.csv");
    let Some((rows, _)) = feat_io::ticks::read_member(&feat_io::ticks::day_path(ticks, y, m, d), &member)? else { return Ok(None) };
    let (secs, _) = feat_io::ticks::fold_seconds(&rows, 0, 86_400);
    let mut out = Vec::with_capacity(secs.len());
    let mut po = Vec::new();
    for s in secs {
        let ts = price::at_sod(sess.day, i64::from(s.sod));
        if s.price_only() {
            po.push(ts);
        }
        out.push(SecIn { ts, high: s.high, low: s.low, vol: s.volume });
    }
    Ok(Some(Day::new(sess.day, out, &sess.segs, sess.kind == "REGULAR", po)?))
}

/// The plan of candle `i` under `exit`.
pub(crate) fn plan_of(fam: &Family, i: usize, long: bool, exit: &ExitSet, sq: i64) -> Result<Plan, &'static str> {
    match exit.kind {
        ExitKind::Supertrend(..) | ExitKind::Ema(_) => Ok(cands::signal_exit(fam, i, long, exit).map_or(Plan::Clock, |(end, immediate)| Plan::Signal { anchor: end * US_S, immediate }).clamp_sq(sq)),
        ExitKind::Clock => Ok(Plan::Clock),
        ExitKind::GoPositive(t) => Ok(Plan::Timer { secs: t }),
        // SNIPER s20: the XS flip anchor of the same setting (no flip in the session, or a flip at or after the square-off: the
        // square-off is the signal part, as an XS row's Plan::Clock) and the plain clock `secs` after E0, the earlier wins in price.
        ExitKind::SupertrendOrClock(_, _, secs) => Ok(match cands::signal_exit(fam, i, long, exit) {
            Some((end, immediate)) if end * US_S < sq => Plan::ClockOrSignal { secs, anchor: end * US_S, immediate },
            _ => Plan::ClockOrSignal { secs, anchor: sq, immediate: false },
        }),
        ExitKind::SmallStop(x) => {
            let atr = fam.cands.get(i).and_then(|c| c.atr).ok_or("atr_unready")?;
            // x / 100 ATR 14 (S = 10^6 paisa), floored to whole paisa, at least 1 paisa.
            let d = (atr * i128::from(x) / 100 / common_feat::S).max(1);
            Ok(Plan::Stop { x: i64::try_from(d).map_err(|_| "atr_overflow")? })
        }
    }
}

trait ClampSq {
    fn clamp_sq(self, sq: i64) -> Plan;
}

impl ClampSq for Plan {
    fn clamp_sq(self, sq: i64) -> Plan {
        match self {
            Plan::Signal { anchor, .. } if anchor >= sq => Plan::Clock,
            p => p,
        }
    }
}

/// The result of one context.
pub struct Report {
    /// The lines printed.
    pub lines: Vec<String>,
}

/// Runs one context end to end.
///
/// # Errors
/// A refused input, a held gate, or an I/O error.
#[allow(clippy::too_many_lines, reason = "the gate-6 pipeline in one place: gate, readings, pricing, store, sweep, DB rows")]
pub fn run_context(inp: &Inputs, spec: &Spec) -> Result<Report, String> {
    crate::refuse_if_red()?;
    let run = &inp.run;
    let mut lines = Vec::new();
    let gate = crate::monitor::launch_gate(run, spec.swap_base_mb, &spec.out, 4)?;
    lines.push(format!("gate ok={} hold={:?} pressure={:?} swap_used={:?} swap_free={:?} disk_free={:?}", gate.ok, gate.hold, gate.pressure, gate.swap_used_mb, gate.swap_free_mb, gate.disk_free_gib));
    if !gate.ok {
        return Err(format!("launch gate HOLDS: {:?}", gate.hold));
    }
    let db = Db::open(&spec.out.join("db"), crate::VERSION)?;
    let hb = spec.out.join("HEARTBEAT.tsv");
    let beat = |stage: &str, extra: String| {
        let _ = crate::monitor::heartbeat(&hb, &[("stage", stage.to_string()), ("rss_mb", format!("{:?}", crate::monitor::rss_mb())), ("detail", extra)]);
    };
    let exit = inp.exits.iter().find(|e| e.id == spec.exit).ok_or_else(|| format!("exit set {} not in the exits file", spec.exit))?;
    let t0 = Instant::now();
    let cont = common_feat::cal::parse_ymd("2018-09-07").ok_or("cont start")?;
    let ctx = feat_io::load_ctx(Path::new(feat_io::defaults::DATES), Path::new(feat_io::defaults::BHAV), cont)?;
    let store = Path::new(feat_io::defaults::STORE);
    let (stream, st) = feat_io::minute_stream(store, inp.instr.underlying(), &spec.tf, &ctx);
    if stream.tf_s < 60 {
        return Err(format!("{}: the seconds families are not wired in host v1", spec.tf));
    }
    let (s1, _) = feat_io::minute_stream(store, inp.instr.underlying(), "1min", &ctx);
    let facts = feat_io::host::day_facts(&s1, &ctx);
    let lv = feat_io::host::levels(&ctx, &facts);
    let cut = run.hms("last_decision_before_ist")?;
    let fam = cands::build(&ctx, &stream, &lv, (&inp.core.0, &inp.core.1), cut)?;
    lines.push(format!("readings: {} candles ({}), {} decision candidates, {} not entry {:?} (Session::entry_at at the candle END), K13 resets {}, K_tf {}, bars read {}, straddling candles kept {}", stream.candles.len(), stream.source, fam.cands.len(), fam.not_entry, fam.refused, fam.resets.len(), fam.kmax, st.bars, st.straddle_kept));
    beat("readings", format!("{} candidates", fam.cands.len()));
    let (hull_from, hull_to) = inp.windows.hull();
    let xe = match exit.kind {
        ExitKind::Ema(p) => Some(p),
        _ => None,
    };
    let learn: Vec<usize> = (0..fam.cands.len()).filter(|&i| fam.cands.get(i).is_some_and(|c| c.mk >= hull_from && c.mk <= hull_to)).collect();
    // --- pricing (parallel over sessions) ---
    let levels: Vec<u16> = run.list("levels_bp")?;
    let dial = Dial::new(&levels).map_err(|e| e.to_string())?;
    let cfg = Cfg {
        dial,
        lev: (run.num("lev_num")?, run.num("lev_den")?),
        protection_bp: run.num("protection_bp")?,
        limit_tick: run.num("limit_tick_paisa")?,
        square_off_sod: run.hms("square_off_ist")?,
        cut_sod: cut,
        qty_rule: inp.qty_rule,
        account: run.num("account_paisa")?,
    };
    let account: i64 = run.num("account_paisa")?;
    let stored: Vec<i64> = run.list("stored_n")?;
    let slots: Vec<i64> = stored.iter().map(|n| account / n.max(&1)).collect();
    let origin = price::at_sod(common_feat::cal::parse_ymd("2018-01-01").ok_or("origin")?, 0);
    let mut by_sess: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for &i in &learn {
        if let Some(c) = fam.cands.get(i) {
            by_sess.entry(c.sess).or_default().push(i);
        }
    }
    let sess_list: Vec<(u32, Vec<usize>)> = by_sess.into_iter().collect();
    let next = AtomicUsize::new(0);
    let priced: Mutex<Vec<(usize, Result<Priced, String>)>> = Mutex::new(Vec::new());
    let ticks = Path::new(feat_io::defaults::TICKS);
    let tp = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..spec.threads.max(1) {
            s.spawn(|| {
                let mut local = Vec::new();
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some((sess, rows)) = sess_list.get(k) else { break };
                    let Some(se) = ctx.sessions.get(*sess as usize) else { continue };
                    let day = match day_tape(ticks, inp.instr.underlying(), se) {
                        Ok(Some(d)) => d,
                        Ok(None) => {
                            for &i in rows {
                                local.push((i, Err("no_tick_member".to_string())));
                            }
                            continue;
                        }
                        Err(e) => {
                            for &i in rows {
                                local.push((i, Err(format!("tick_day: {e}"))));
                            }
                            continue;
                        }
                    };
                    let sq = price::at_sod(se.day, cfg.square_off_sod);
                    for &i in rows {
                        let Some(c) = fam.cands.get(i) else { continue };
                        if xe.is_some_and(|p| !cands::xe_with(c, spec.long, p)) {
                            local.push((i, Err("xe_filter".to_string())));
                            continue;
                        }
                        let r = plan_of(&fam, i, spec.long, exit, sq).map_err(str::to_string).and_then(|plan| {
                            price::price(&day, &cfg, c.end * US_S, spec.long, plan, &slots).map_err(|e| match e {
                                Refusal::Fill(x) => format!("fill: {x}"),
                                Refusal::Charges(x) => format!("charges: {x}"),
                                other => format!("{other:?}"),
                            })
                        });
                        local.push((i, r));
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
    let price_secs = tp.elapsed();
    let mut refusals: BTreeMap<String, u64> = BTreeMap::new();
    let mut kidx = Vec::new();
    let mut outcomes = Vec::new();
    let mut records = Vec::new();
    let mut rewalks = 0_u64;
    let mut steps = sniper_kernel::steps::Hist::new();
    for (i, r) in &priced {
        match r {
            Err(e) => *refusals.entry(e.split(':').next().unwrap_or("?").to_string()).or_insert(0) += 1,
            Ok(p) => {
                let Some(c) = fam.cands.get(*i) else { continue };
                let rec = match price::encode(p, levels.len(), origin) {
                    Ok(b) => b,
                    Err(e) => {
                        *refusals.entry(format!("{e:?}")).or_insert(0) += 1;
                        continue;
                    }
                };
                let mut o = Outcome { sess: c.sess, mk: c.mk, id: *i as u64, ..Outcome::default() };
                if let Some(first) = p.lv.first() {
                    for (l, x) in first.iter().take(levels.len()).enumerate() {
                        if let (Some(n), Some(rd)) = (o.net.get_mut(l), o.rd_bp.get_mut(l)) {
                            *n = x.net;
                            *rd = x.rd_bp;
                        }
                    }
                }
                rewalks += u64::from(p.rewalks);
                steps.add(p.steps);
                kidx.push(*i);
                outcomes.push(o);
                records.extend_from_slice(&rec);
            }
        }
    }
    let side = if spec.long { "LONG" } else { "SHORT" };
    let part = format!("OUT-{}-{}-{}.bin", spec.tf, side, exit.id);
    let sha = db.write_bin(&part, &records)?;
    let keys: Vec<(u32, u16)> = kidx.iter().filter_map(|&i| fam.cands.get(i).map(|c| (c.sess, c.k))).collect();
    db.write_bin(&format!("IX-{}-{}-{}.bin", spec.tf, side, exit.id), &crate::query::build_index(&keys, ctx.sessions.len(), fam.kmax))?;
    lines.push(format!(
        "pricing: {} rows priced, {} refused {:?}; {:.1} s wall on {} threads = {} us a row (wall x threads / rows); S2 re-walks {rewalks}; crate blocks per row p50/p99/max {:?}; partition {part} sha {sha}; {}",
        kidx.len(),
        priced.len() - kidx.len(),
        refusals,
        price_secs.as_secs_f64(),
        spec.threads,
        (price_secs.as_micros() * spec.threads as u128) / (kidx.len().max(1) as u128),
        steps.summary(),
        inp.qty_rule.label()
    ));
    beat("priced", format!("{} rows", kidx.len()));
    // --- the sweep ---
    let krows = cands::kernel_rows(&fam, spec.long, &kidx);
    let sess_mk: Vec<i32> = ctx.sessions.iter().map(|s| {
        let (y, m, _) = civil(s.day);
        y * 12 + i32::try_from(m).unwrap_or(1) - 1
    }).collect();
    let wins: Vec<(String, i32, i32)> = inp.windows.rows.iter().map(|r| (r.0.clone(), r.1, r.2)).collect();
    let lay = Layout::new(&sess_mk, (hull_from, hull_to), &wins).map_err(|e| e.to_string())?;
    let (values, wiring) = cands::outer_values(&fam, &kidx, spec.long, &inp.dials, xe);
    let core = cands::core_of(if spec.long { &inp.core.0 } else { &inp.core.1 });
    let pos = |id: &str| inp.dials.iter().position(|d| d.id == id).unwrap_or(0);
    let filter = None;
    let sctx = crate::sweep::Ctx {
        rows: &krows,
        out: &outcomes,
        lay: &lay,
        views: &inp.views,
        cols: levels.len(),
        core,
        sessions: ctx.sessions.len(),
        top: run.num("stage2_top")?,
        radix: &wiring.radix,
        core_pos: [pos("M-RSI"), pos("M-CCI"), pos("M-WR")],
        filter,
    };
    let ts = Instant::now();
    let res = crate::sweep::run(&sctx, &values, spec.threads)?;
    let sweep_secs = ts.elapsed();
    let combos = res.walk.cells;
    lines.push(format!(
        "sweep S0+S1: {} outer values walked (+S0), {} EMPTY (S7), {} core cells = {:.3e} combos; {} offered runs ({} cells), {} empty cells; inserts {} removes {}; {:.1} s wall on {} threads; refused {:?}",
        res.values_walked.saturating_sub(1),
        res.values_empty,
        combos,
        combos as f64,
        res.walk.runs_offered,
        res.walk.cells_offered,
        res.walk.empty_cells,
        res.walk.inserts,
        res.walk.removes,
        sweep_secs.as_secs_f64(),
        spec.threads,
        res.refused.iter().take(3).collect::<Vec<_>>()
    ));
    lines.push(format!(
        "step counters p50/p99/max: kernel enter {:?}, kernel line {:?}, accumulator event {:?}, key read {:?}",
        res.walk.hist_enter.summary(),
        res.walk.hist_line.summary(),
        res.hist_acc.summary(),
        res.hist_key.summary()
    ));
    lines.push(format!("wired dials {:?}; LIVE not wired in host v1 {:?}; HELD/LATE {:?}", wiring.wired, wiring.not_wired, wiring.held));
    beat("swept", format!("{combos} cells"));
    // --- the HARD gate (plan 8.4) on the real rows: WF-learning rows replicated 1x / 10x / 100x inside the same calendar ---
    let wf_to = inp.windows.rows.first().map_or(hull_to, |r| r.2);
    let wf_rows: Vec<usize> = (0..krows.len()).filter(|&j| outcomes.get(j).is_some_and(|o| o.mk <= wf_to)).collect();
    let base_rows: Vec<sniper_kernel::line::Row> = wf_rows.iter().filter_map(|&j| krows.get(j).copied()).collect();
    let base_out: Vec<Outcome> = wf_rows.iter().filter_map(|&j| outcomes.get(j).copied()).collect();
    let mut gate_lines = Vec::new();
    let mut gate_sig = Vec::new();
    for times in [1_u16, 10, 100] {
        let (rt, src) = sniper_kernel::scale::replicate(&base_rows, times, 20_261_113);
        let ot: Vec<Outcome> = src.iter().enumerate().filter_map(|(j, &k)| base_out.get(k as usize).map(|o| Outcome { id: j as u64, ..*o })).collect();
        let gctx = crate::sweep::Ctx { rows: &rt, out: &ot, lay: &lay, views: &inp.views, cols: levels.len(), core, sessions: ctx.sessions.len(), top: 1000, radix: &wiring.radix, core_pos: [pos("M-RSI"), pos("M-CCI"), pos("M-WR")], filter: None };
        let tg = Instant::now();
        let g = crate::sweep::run(&gctx, &[], 1)?;
        let sig = (g.walk.hist_enter.max(), g.walk.hist_line.max(), g.hist_acc.max(), g.hist_key.max());
        gate_sig.push(sig);
        gate_lines.push(format!("{times}x: {} rows; per-event max (enter, line, acc, key) {sig:?}; p50/p99 enter {:?} acc {:?} key {:?}; inserts {}; wall {:.2} s", rt.len(), g.walk.hist_enter.summary(), g.hist_acc.summary(), g.hist_key.summary(), g.walk.inserts, tg.elapsed().as_secs_f64()));
    }
    let hard_ok = gate_sig.windows(2).all(|w| w.first() == w.get(1));
    lines.push(format!("HARD gate (per-event step counts identical at 1x / 10x / 100x, S0 on the WF-learning rows): {}", if hard_ok { "PASS" } else { "FAIL" }));
    lines.extend(gate_lines.iter().cloned());
    // --- DB rows ---
    let labels = inp.words.standing(1, "printed by the tool").join(" | ");
    let mut list_rows = Vec::new();
    for (vi, v) in inp.views.iter().enumerate() {
        for (c, bp) in levels.iter().enumerate() {
            let Some(l) = res.lists.get(vi * levels.len() + c) else { continue };
            for (rank, (k, code, n)) in l.entries().take(100).enumerate() {
                list_rows.push(vec![
                    spec.run_id.clone(),
                    spec.tf.clone(),
                    side.to_string(),
                    exit.id.clone(),
                    v.id.clone(),
                    bp.to_string(),
                    (rank + 1).to_string(),
                    sniper_kernel::code::Code(code).hex(),
                    n.to_string(),
                    k.0.iter().map(i64::to_string).collect::<Vec<_>>().join(","),
                    labels.clone(),
                ]);
            }
        }
    }
    db.write(&format!("SN-LISTS-{}-{}-{}.tsv", spec.tf, side, exit.id), &["run", "tf", "side", "exit", "view", "level_bp", "rank", "code", "run_len", "key_K5_K7_K6_K8_K9_K10_K4", "labels"], &list_rows)?;
    let sn4 = vec![vec![
        spec.run_id.clone(),
        format!("{}|{side}|{}", spec.tf, exit.id),
        "S0+S1".to_string(),
        res.values_walked.to_string(),
        res.values_empty.to_string(),
        combos.to_string(),
        res.walk.runs_offered.to_string(),
        res.walk.empty_cells.to_string(),
        format!("{:?}", res.not_eligible),
        format!("{:?}|{:?}|{:?}|{:?}", res.walk.hist_enter.summary(), res.walk.hist_line.summary(), res.hist_acc.summary(), res.hist_key.summary()),
        rewalks.to_string(),
        format!("{refusals:?}"),
        format!("{:?}", wiring.not_wired),
        format!("{:.1}|{:.1}", price_secs.as_secs_f64(), sweep_secs.as_secs_f64()),
        spec.threads.to_string(),
        labels.clone(),
    ]];
    db.write(
        &format!("SN4-{}-{}-{}.tsv", spec.tf, side, exit.id),
        &["run", "context", "stage", "outer_values_walked", "values_empty_s7", "core_cells", "runs_offered", "empty_cells", "not_eligible_e1_e2_e4", "steps_p50_p99_max_enter_line_acc_key", "s2_rewalks", "price_refusals", "live_not_wired", "wall_s_price_sweep", "threads", "labels"],
        &sn4,
    )?;
    let steps_rows: Vec<Vec<String>> = gate_lines.iter().map(|l| vec![spec.run_id.clone(), format!("{}|{side}|{}", spec.tf, exit.id), if hard_ok { "PASS".to_string() } else { "FAIL".to_string() }, l.clone(), labels.clone()]).collect();
    db.write(&format!("SN4-STEPS-{}-{}-{}.tsv", spec.tf, side, exit.id), &["run", "context", "hard_gate", "scale_line", "labels"], &steps_rows)?;
    let mut sn1 = vec![
        vec![spec.run_id.clone(), "version".to_string(), crate::VERSION.to_string(), labels.clone()],
        vec![spec.run_id.clone(), "laws".to_string(), crate::law_line(), labels.clone()],
        vec![spec.run_id.clone(), "threads".to_string(), spec.threads.to_string(), labels.clone()],
        vec![spec.run_id.clone(), "gate".to_string(), format!("{gate:?}").replace('\n', " "), labels.clone()],
        vec![spec.run_id.clone(), "bar_source".to_string(), inp.words.line("BARSRC", &[]), labels.clone()],
        vec![spec.run_id.clone(), "stream_source".to_string(), stream.source.clone(), labels.clone()],
        vec![spec.run_id.clone(), "start_end".to_string(), format!("{:.1} s", t0.elapsed().as_secs_f64()), labels.clone()],
        vec![spec.run_id.clone(), "qty_rule".to_string(), format!("{} ({})", inp.qty_rule.id(), inp.qty_rule.label()), labels.clone()],
    ];
    for (f, s) in &inp.shas {
        sn1.push(vec![spec.run_id.clone(), format!("sha256 {f}"), s.clone(), labels.clone()]);
    }
    db.write(&format!("SN1-{}-{}-{}.tsv", spec.tf, side, exit.id), &["run", "field", "value", "labels"], &sn1)?;
    let n = db.verify()?;
    lines.push(format!("DB: {} tables audited, chain verified; run folder {}", n, spec.out.display()));
    lines.push(format!("NOT DONE in host v1 (named, never silent): the picks, verdict words, method lines and luck lines need common_wf (no SHA256SUMS yet); the shortlist re-walk; the twins' worlds; the seconds families; {} LIVE dials not wired", wiring.not_wired.len()));
    for l in inp.words.standing(1, "printed by the tool") {
        lines.push(l);
    }
    beat("done", String::new());
    Ok(Report { lines })
}
