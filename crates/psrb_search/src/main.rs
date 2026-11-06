//! SCRATCH PREVIEW SEARCH, never committed to any branch: the PSRB preview search over
//! the F&O stocks (`work-20260925/state/psrb-preview-20261106/search/SEARCH-SPEC.md`
//! with `OPERATOR-ADDENDUM-RISK-20261106.md`, addenda 1-4). NOT the PSRB result.
//!
//! It opens every store file read-only through `store::file::BarFile::open_existing`,
//! installs no telemetry sink, writes only under `--out`, and never deletes a file.

#![forbid(unsafe_code)]
#![allow(clippy::indexing_slicing, reason = "the search's hot loops index bounded arrays; Rust bounds-checks every index")]

mod acct;
mod check;
mod common;
mod filters;
mod merge;
mod nifty;
mod phase_a;
mod prev;
mod report;
mod search;
mod spool;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use brutex_core::price::Paisa;
use costs::day::TradeDay;
use costs::equity_clock::{Fill, Order, TradeFills, price_trade};
use costs::rate::OrderSide;
use indicators::trend::{Ema, SCALE};
use pull::fno_membership::Membership;
use rayon::prelude::*;

use crate::common::{N_LANE, N_TF, OPEN_MIN, TFS, at_min, day_number, lanes_of, session_day};
use crate::phase_a::{Counts, Ctx, Levels, Sym, cpr_narrow};

struct Args {
    store: PathBuf,
    membership: PathBuf,
    universe: PathBuf,
    out: PathBuf,
    preview_rows: Option<PathBuf>,
    threads: usize,
    only: Vec<String>,
    limit: Option<usize>,
    tree_sha: String,
    command: String,
    stage_a: bool,
    stage_b: bool,
    twins: Vec<u64>,
    tfs: Vec<usize>,
    light: Vec<usize>,
    no_real: bool,
    report_dir: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let raw: Vec<String> = std::env::args().collect();
    let command = raw.join(" ");
    let mut a = Args {
        store: PathBuf::new(),
        membership: PathBuf::new(),
        universe: PathBuf::new(),
        out: PathBuf::new(),
        preview_rows: None,
        threads: std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
        only: Vec::new(),
        limit: None,
        tree_sha: "unknown".to_owned(),
        command,
        stage_a: true,
        stage_b: true,
        twins: Vec::new(),
        tfs: vec![5, 4, 3, 2, 1, 0],
        light: Vec::new(),
        no_real: false,
        report_dir: None,
    };
    let mut have = [false; 4];
    let mut it = raw.iter().skip(1);
    while let Some(x) = it.next() {
        let mut val = || it.next().cloned().ok_or_else(|| format!("{x} needs a value"));
        match x.as_str() {
            "--store" => {
                a.store = PathBuf::from(val()?);
                have[0] = true;
            }
            "--membership" => {
                a.membership = PathBuf::from(val()?);
                have[1] = true;
            }
            "--universe" => {
                a.universe = PathBuf::from(val()?);
                have[2] = true;
            }
            "--out" => {
                a.out = PathBuf::from(val()?);
                have[3] = true;
            }
            "--preview-rows" => a.preview_rows = Some(PathBuf::from(val()?)),
            "--threads" => a.threads = val()?.parse().map_err(|e| format!("--threads: {e}"))?,
            "--only" => a.only = val()?.split(',').map(str::to_owned).collect(),
            "--limit" => a.limit = Some(val()?.parse().map_err(|e| format!("--limit: {e}"))?),
            "--tree-sha" => a.tree_sha = val()?,
            "--skip-a" => a.stage_a = false,
            "--only-a" => a.stage_b = false,
            "--twins" => {
                a.twins = val()?.split(',').filter(|x| !x.is_empty()).map(|x| x.parse().map_err(|e| format!("--twins: {e}"))).collect::<Result<_, _>>()?;
            }
            "--light-tfs" => {
                let v: Vec<i64> = val()?.split(',').map(|x| x.parse().map_err(|e| format!("--light-tfs: {e}"))).collect::<Result<_, _>>()?;
                a.light = v.iter().filter_map(|k| TFS.iter().position(|t| t == k)).collect();
            }
            "--twins-only" => a.no_real = true,
            "--report-dir" => a.report_dir = Some(PathBuf::from(val()?)),
            "--tfs" => {
                let v: Vec<i64> = val()?.split(',').map(|x| x.parse().map_err(|e| format!("--tfs: {e}"))).collect::<Result<_, _>>()?;
                a.tfs = v.iter().filter_map(|k| TFS.iter().position(|t| t == k)).collect();
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if have.iter().any(|h| !h) {
        return Err("--store, --membership, --universe and --out are required".to_owned());
    }
    a.threads = a.threads.max(1);
    Ok(a)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("psrb_search: STOPPED: {e}");
            ExitCode::FAILURE
        }
    }
}

fn load_universe(path: &Path) -> Result<Vec<Sym>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut names: Vec<String> = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(str::to_owned).collect();
    names.sort_unstable();
    names.dedup();
    names
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let series = if name == "ICICILIFE" { "ICICIPRULI".to_owned() } else { name.clone() };
            let rank = u16::try_from(i).map_err(|_| "universe too large".to_owned())?;
            Ok(Sym { name, series, rank })
        })
        .collect()
}

fn self_checks(store: &Path) -> Result<(Vec<String>, String), String> {
    let mut lines = Vec::new();
    let leg = |side: OrderSide, price: i64| Order { side, by_broker: false, fills: vec![Fill { second: 0, quantity: 400, price: Paisa::from_raw(price) }] };
    let trade = TradeFills { orders: vec![leg(OrderSide::Buy, 1_250_00), leg(OrderSide::Sell, 1_262_50)] };
    let day = TradeDay::new(2018, 9, 3).map_err(|e| format!("CN01 day: {e:?}"))?;
    let p = price_trade(&trade, day).map_err(|e| format!("CN01 price_trade refused: {e}"))?;
    let (dated, today) = (p.dated_official().total.raw(), p.today_official().total.raw());
    if dated != 313_25 || today != 225_79 {
        return Err(format!("self-check CN01 failed: dated official {dated}, today official {today} paisa (expected 31325, 22579)"));
    }
    lines.push("CN01 (400 at Rs 1,250.00, sold Rs 1,262.50, 2018-09-03): dated official Rs 313.25, today official Rs 225.79: OK".to_owned());
    let version = p.version().to_owned();
    let k: i128 = 9_214_157_878_975_800;
    let (h, l, c) = (1001 * k, 998 * k, 1001 * k);
    if !(cpr_narrow(h, l, c, 10) && !cpr_narrow(h, l, c, 9) && !cpr_narrow(h, l - 1, c, 10)) {
        return Err("self-check CPR tie failed".to_owned());
    }
    lines.push("CPR-1 tie (H = C = 1001k, L = 998k): narrow at 10 bp, not at 9 bp; with L one paisa lower not narrow at 10 bp: OK".to_owned());
    let mut e = Ema::new(200);
    for _ in 0..199 {
        e.fold(101);
    }
    let warm199 = e.warm();
    e.fold(100);
    let st = e.scaled();
    if warm199 || !e.warm() || st != 100_995_000 || 100 * SCALE >= st {
        return Err(format!("self-check EMA fixture failed: scaled {st}"));
    }
    lines.push("EMA 200 fixture (199 closes of 101, then 100): state 100995000, close 100 short true, long false; not warm after 199: OK".to_owned());
    lines.push(phase_a::grid_check(store, "RELIANCE", 2024, 3)?);
    lines.push(nifty::nifty_grid_check(store)?);
    Ok((lines, version))
}

fn probe_membership(dir: &Path) -> Result<(Option<Membership>, String, String, String), String> {
    let first = session_day(day_number(2018, 9, 3)?)?;
    let last = session_day(day_number(2026, 9, 30)?)?;
    match Membership::open(dir) {
        Err(e) => Ok((None, format!("survivorship: today's list on every day (the membership folder was refused: {e})"), "today's list".to_owned(), format!("membership probe: open refused: {e}"))),
        Ok(ms) => {
            let a = ms.is_member("RELIANCE", first);
            let b = ms.is_member("RELIANCE", last);
            let c = ms.is_member("SYRMA", last);
            let line = format!("membership probe: RELIANCE 2018-09-03 {a:?}, RELIANCE 2026-09-30 {b:?}, SYRMA 2026-09-30 {c:?}");
            if matches!(a, Ok(true)) && matches!(b, Ok(true)) && !matches!(c, Ok(true)) {
                let digest = ms.file_digest_hex();
                Ok((Some(ms), format!("dated membership from {} (inferred from GDFL stock-option listings, 2018-09..2026-09; 2026-09-30 carried into 2026-10)", dir.display()), digest, format!("{line}: OK")))
            } else {
                Ok((None, "survivorship: today's list on every day (the membership probe answered otherwise)".to_owned(), "today's list".to_owned(), format!("{line}: FAILED, today's list used")))
            }
        }
    }
}

fn hms(secs: u64) -> String {
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60)
}

fn now_ist() -> Result<String, String> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?;
    let us = i64::try_from(now.as_micros()).map_err(|_| "clock".to_owned())?;
    Ok(common::fmt_ist(us))
}

/// Phase A's counters as numbers, for the stage-B-only re-run.
fn counts_vec(c: &Counts) -> Vec<u64> {
    let mut v = vec![
        c.z_months_read,
        c.z_months_missing,
        c.z_months_refused,
        c.g_months_missing,
        c.g_months_refused,
        c.fold_errors,
        c.charter_minutes_withheld,
        c.m2_days,
        c.m2_days_no_minutes,
        c.trade_days,
        c.member_false_days,
        c.member_unknown_days,
        c.prev_refused_days,
        c.no_prev_days,
        c.candidate_days,
        c.fallback_days,
        c.fallback_gap_days,
        c.fallback_no_month_days,
        c.fallback_empty_days,
        c.fallback_read_error_days,
    ];
    for a in [&c.candles, &c.holes, &c.holes_missing_minute, &c.holes_withheld, &c.holes_refused, &c.padded_only, &c.unscheduled_bars, &c.taint10, &c.taint200, &c.decisions] {
        v.extend_from_slice(a);
    }
    v.extend_from_slice(&c.signals);
    v.extend_from_slice(&c.cut);
    for ti in 0..N_TF {
        for lane in 0..N_LANE {
            v.push(c.records[ti][lane]);
            v.extend_from_slice(&c.at_once[ti][lane]);
            v.extend_from_slice(&c.filled[ti][lane]);
        }
    }
    v
}

fn counts_from(v: &[u64]) -> Counts {
    let mut c = Counts::new();
    let mut it = v.iter().copied();
    let mut n = || it.next().unwrap_or(0);
    c.z_months_read = n();
    c.z_months_missing = n();
    c.z_months_refused = n();
    c.g_months_missing = n();
    c.g_months_refused = n();
    c.fold_errors = n();
    c.charter_minutes_withheld = n();
    c.m2_days = n();
    c.m2_days_no_minutes = n();
    c.trade_days = n();
    c.member_false_days = n();
    c.member_unknown_days = n();
    c.prev_refused_days = n();
    c.no_prev_days = n();
    c.candidate_days = n();
    c.fallback_days = n();
    c.fallback_gap_days = n();
    c.fallback_no_month_days = n();
    c.fallback_empty_days = n();
    c.fallback_read_error_days = n();
    for a in [
        &mut c.candles,
        &mut c.holes,
        &mut c.holes_missing_minute,
        &mut c.holes_withheld,
        &mut c.holes_refused,
        &mut c.padded_only,
        &mut c.unscheduled_bars,
        &mut c.taint10,
        &mut c.taint200,
        &mut c.decisions,
    ] {
        for x in a.iter_mut() {
            *x = n();
        }
    }
    for x in &mut c.signals {
        *x = n();
    }
    for x in &mut c.cut {
        *x = n();
    }
    for ti in 0..N_TF {
        for lane in 0..N_LANE {
            c.records[ti][lane] = n();
            for k in 0..2 {
                c.at_once[ti][lane][k] = n();
            }
            for k in 0..2 {
                c.filled[ti][lane][k] = n();
            }
        }
    }
    c
}

#[allow(clippy::too_many_lines, reason = "the run's one sequence")]
fn run() -> Result<(), String> {
    let t0 = Instant::now();
    let args = parse_args()?;
    std::fs::create_dir_all(&args.out).map_err(|e| format!("{}: {e}", args.out.display()))?;
    let started = now_ist()?;
    eprintln!("psrb_search: started {started} IST, threads {}", args.threads);
    let mut syms = load_universe(&args.universe)?;
    let universe_n = syms.len();
    if !args.only.is_empty() {
        syms.retain(|s| args.only.iter().any(|o| *o == s.name));
    }
    if let Some(n) = args.limit {
        syms.truncate(n);
    }
    eprintln!("psrb_search: universe {universe_n} names, running {}", syms.len());
    let levels = Levels::new()?;
    let (mut checks, cost_version) = self_checks(&args.store)?;
    let (membership, membership_label, membership_digest, probe_line) = probe_membership(&args.membership)?;
    checks.push(probe_line);
    for c in &checks {
        eprintln!("self-check: {c}");
    }
    let pool = rayon::ThreadPoolBuilder::new().num_threads(args.threads).build().map_err(|e| format!("thread pool: {e}"))?;
    // Progress every minute.
    let stop = Arc::new(AtomicBool::new(false));
    let stocks_done = Arc::new(AtomicUsize::new(0));
    let recs_done = Arc::new(AtomicU64::new(0));
    let progress = {
        let (stop, sd, rd) = (Arc::clone(&stop), Arc::clone(&stocks_done), Arc::clone(&recs_done));
        let total = syms.len();
        std::thread::spawn(move || {
            let start = Instant::now();
            let mut last = 0_u64;
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_secs(1));
                let secs = start.elapsed().as_secs();
                if secs >= last + 60 {
                    last = secs;
                    let stage = search::STAGE.lock().map(|g| g.clone()).unwrap_or_default();
                    let rows = search::ROWS.load(Ordering::Relaxed);
                    let rate = rows.checked_div(secs).unwrap_or(0);
                    eprintln!(
                        "[progress {}] {stage}; phase A stocks {}/{total}, records {}; phase B rows tried {rows} ({rate}/s overall), tasks {}",
                        hms(secs),
                        sd.load(Ordering::Relaxed),
                        rd.load(Ordering::Relaxed),
                        search::TASKS.load(Ordering::Relaxed)
                    );
                }
            }
        })
    };
    let mut counts = Counts::new();
    let mut notes: Vec<String> = Vec::new();
    let mut spool_bytes = 0_u64;
    let mut phase_a_secs = 0_u64;
    let mut nifty_line = String::new();
    let counts_path = args.out.join("spool").join("phase-a-counts.txt");
    if args.stage_a {
        search::stage("phase A: NIFTY".to_owned());
        let nifty = nifty::Nifty::load(&args.store)?;
        nifty_line = format!(
            "{} months read, {} missing, {} minutes; ready EMA 20 candles by timeframe {:?}",
            nifty.months_read,
            nifty.months_missing,
            nifty.minutes,
            nifty.ema_counts()
        );
        eprintln!("psrb_search: NIFTY {nifty_line}");
        let ctx = Ctx {
            store: &args.store,
            membership: membership.as_ref(),
            levels,
            nifty: &nifty,
            win_first: day_number(2018, 9, 3)?,
            win_last: day_number(2026, 10, 30)?,
            cas_day: day_number(2026, 8, 3)?,
            outage_day: day_number(2021, 2, 24)?,
            gdfl_gap_day: day_number(2019, 5, 30)?,
            oct_first: day_number(2026, 10, 1)?,
            carry_day: session_day(day_number(2026, 9, 30)?)?,
        };
        search::stage("phase A: stocks".to_owned());
        let ta = Instant::now();
        let out_dir = args.out.clone();
        let results: Vec<(Counts, Vec<String>, String, u64, Option<String>)> = pool.install(|| {
            syms.par_iter()
                .with_max_len(1)
                .map(|s| {
                    let st = Instant::now();
                    let r = phase_a::run_stock(&ctx, s);
                    let mut bytes = 0_u64;
                    let mut nrec = 0_u64;
                    let mut fatal = r.fatal.clone();
                    if fatal.is_none() {
                        for ti in 0..N_TF {
                            for lane in 0..lanes_of(ti) {
                                let v = r.recs.get(ti * N_LANE + lane).map_or(&[][..], Vec::as_slice);
                                nrec += u64::try_from(v.len()).unwrap_or(0);
                                match spool::write_stock(&out_dir, ti, lane, s, v) {
                                    Ok(b) => bytes += b,
                                    Err(e) => fatal = Some(format!("{}: spool write: {e}", s.name)),
                                }
                            }
                        }
                    }
                    stocks_done.fetch_add(1, Ordering::Relaxed);
                    recs_done.fetch_add(nrec, Ordering::Relaxed);
                    eprintln!("[stock] {} records {} in {} s{}", s.name, nrec, st.elapsed().as_secs(), fatal.as_ref().map_or_else(String::new, |f| format!(" FATAL {f}")));
                    (r.counts, r.notes, r.span, bytes, fatal)
                })
                .collect()
        });
        phase_a_secs = ta.elapsed().as_secs();
        let fatal: Vec<String> = results.iter().filter_map(|r| r.4.clone()).collect();
        if !fatal.is_empty() {
            return Err(format!("{} stock(s) stopped the run: {}", fatal.len(), fatal.join(" | ")));
        }
        for (c, n, sp, b, _) in &results {
            counts.add(c);
            notes.extend(n.iter().cloned());
            if !sp.is_empty() {
                notes.push(format!("span: {sp}"));
            }
            spool_bytes += *b;
        }
        let text: Vec<String> = counts_vec(&counts).iter().map(u64::to_string).collect();
        let meta = format!("{}\n{}\n{}\n{}\n", text.join(" "), spool_bytes, phase_a_secs, nifty_line);
        std::fs::write(&counts_path, meta).map_err(|e| format!("{}: {e}", counts_path.display()))?;
        eprintln!("psrb_search: phase A done in {}, spool {} bytes", hms(phase_a_secs), spool_bytes);
    } else {
        let text = std::fs::read_to_string(&counts_path).map_err(|e| format!("{}: {e} (run phase A first)", counts_path.display()))?;
        let mut lines = text.lines();
        let nums: Vec<u64> = lines.next().unwrap_or("").split(' ').filter_map(|x| x.parse().ok()).collect();
        counts = counts_from(&nums);
        spool_bytes = lines.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        phase_a_secs = lines.next().and_then(|x| x.parse().ok()).unwrap_or(0);
        nifty_line = lines.next().unwrap_or("").to_owned();
        notes.push("phase A's spool and counters come from an earlier run of this tool (--skip-a)".to_owned());
    }
    if !args.stage_b {
        stop.store(true, Ordering::Relaxed);
        let _ = progress.join();
        return Ok(());
    }
    let lib = search::Lib::new();
    let log = |s: String| eprintln!("psrb_search: {s}");
    // The 2.3 self-check first: the bare cores against the preview's rows.tsv.
    let start_ts = at_min(day_number(2018, 9, 3)?, OPEN_MIN);
    let mut check23: Vec<String> = Vec::new();
    if let Some(rt_path) = &args.preview_rows {
        let rt = check::read_rt(rt_path)?;
        let mut bad = 0_usize;
        for &ti in &args.tfs {
            search::stage(format!("self-check 2.3 tf {} min", TFS.get(ti).copied().unwrap_or(0)));
            let (lines, b) = pool.install(|| check::check_tf(&args.out, &syms, ti, &rt, start_ts))?;
            for l in &lines {
                eprintln!("check 2.3: {l}");
            }
            bad += b;
            check23.extend(lines);
        }
        if bad > 0 {
            stop.store(true, Ordering::Relaxed);
            let _ = progress.join();
            let p = args.out.join("CHECK-2.3-FAILED.txt");
            let _ = std::fs::write(&p, check23.join("\n"));
            return Err(format!("self-check 2.3 failed: {bad} bare-core rows differ from the preview's rows.tsv (see {})", p.display()));
        }
    } else {
        check23.push("not run (no --preview-rows given)".to_owned());
    }
    // The real search, then the twins.
    let mut seeds: Vec<Option<u64>> = if args.no_real { Vec::new() } else { vec![None] };
    seeds.extend(args.twins.iter().map(|s| Some(*s)));
    let mut real: Option<(merge::MergeOut, report::Info, merge::LuckVerdict)> = None;
    let mut twins: Vec<report::Twin> = Vec::new();
    for twin in seeds {
        let tb = Instant::now();
        let mut tfs_out: Vec<search::TfOut> = Vec::new();
        for &ti in &args.tfs {
            search::stage(format!("phase B tf {} min{}", TFS.get(ti).copied().unwrap_or(0), twin.map_or_else(String::new, |s| format!(" (twin seed {s})"))));
            let light = args.light.contains(&ti);
            let label = twin.map_or_else(|| "real".to_owned(), |s| format!("twin-{s}"));
            let state = args.out.join("tf-state").join(&label).join(format!("tf{}{}.txt", TFS.get(ti).copied().unwrap_or(0), if light { "-light" } else { "" }));
            let r = if state.exists() {
                let r = search::load_tf(&state)?;
                log(format!("phase B tf {} min loaded from {} ({} rows + {} build-up, {} s when run)", TFS.get(ti).copied().unwrap_or(0), state.display(), r.tried, r.tried_buildup, r.secs));
                r
            } else {
                let r = pool.install(|| search::run_tf(&args.out, &syms, ti, &lib, twin, light, &log))?;
                search::save_tf(&state, &r)?;
                r
            };
            log(format!("phase B tf {} min done in {} s: {} rows + {} build-up{}", TFS.get(ti).copied().unwrap_or(0), r.secs, r.tried, r.tried_buildup, if light { " (light)" } else { "" }));
            tfs_out.push(r);
        }
        let tm = Instant::now();
        search::stage(format!("merge{}", twin.map_or_else(String::new, |s| format!(" (twin seed {s})"))));
        let m = pool.install(|| merge::merge(&args.out, &syms, &tfs_out, twin, &log))?;
        let merge_secs = tm.elapsed().as_secs();
        let chain_row = m.chains.first().and_then(|v| v.first()).map(|j| *j.row).unwrap_or(acct::ROW0);
        let lv = merge::luck_verdict(&m.luck, &chain_row);
        match twin {
            None => {
                let mut info = report::Info {
                    command: args.command.clone(),
                    tree_sha: args.tree_sha.clone(),
                    started: started.clone(),
                    threads: args.threads,
                    stocks: syms.len(),
                    checks: checks.clone(),
                    check23: check23.clone(),
                    membership_label: membership_label.clone(),
                    membership_digest: membership_digest.clone(),
                    cost_version: cost_version.clone(),
                    counts: counts.clone(),
                    notes: notes.clone(),
                    nifty: nifty_line.clone(),
                    phase_a_secs,
                    tf_secs: tfs_out.iter().map(|t| (t.ti, t.secs, t.tried, t.tried_buildup)).collect(),
                    merge_secs,
                    spool_bytes,
                    units: tfs_out.iter().flat_map(|t| t.units.iter().map(move |(c, l, n)| (t.ti, *c, *l, *n))).collect(),
                    twins: Vec::new(),
                    twins_pending: !args.twins.is_empty(),
                    bare: tfs_out.iter().flat_map(|t| t.bare.iter().cloned()).collect(),
                    singles: tfs_out.iter().flat_map(|t| t.singles.iter().cloned()).collect(),
                    filters: tfs_out.iter().flat_map(|t| t.filters.iter().cloned()).collect(),
                    light: args.light.clone(),
                };
                if args.twins.is_empty() {
                    info.twins_pending = true;
                }
                let rdir = args.report_dir.clone().unwrap_or_else(|| args.out.clone());
                report::write_all(&rdir, &m, &info, &lib, &lv, false)?;
                log(format!("real search done: phase B {} s, merge {merge_secs} s; reports written to {}", tb.elapsed().as_secs() - merge_secs, args.out.display()));
                info.bare.clear();
                info.singles.clear();
                info.filters.clear();
                real = Some((m, info, lv));
            }
            Some(seed) => {
                let t = report::twin_of(seed, &m, tb.elapsed().as_secs());
                let dir = args.report_dir.clone().unwrap_or_else(|| args.out.clone()).join(format!("twin-{seed}"));
                std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
                std::fs::write(dir.join("summary.tsv"), report::twin_tsv(&t)).map_err(|e| format!("twin summary: {e}"))?;
                log(format!("twin seed {seed} done in {} s: {} of 82 months, net {}", t.secs, t.pass, t.key.net));
                twins.push(t);
                if let Some((m0, info0, lv0)) = real.as_mut() {
                    info0.twins = twins.clone();
                    info0.twins_pending = twins.len() < args.twins.len();
                    let rdir = args.report_dir.clone().unwrap_or_else(|| args.out.clone());
                    report::write_all(&rdir, m0, info0, &lib, lv0, true)?;
                }
            }
        }
    }
    stop.store(true, Ordering::Relaxed);
    let _ = progress.join();
    let refused = search::MINI_REFUSED.load(Ordering::Relaxed);
    if refused > 0 {
        eprintln!("psrb_search: WARNING {refused} records had a gross or charge beyond i32 in the compact view and were zeroed there");
    }
    eprintln!("psrb_search: done in {}", hms(t0.elapsed().as_secs()));
    Ok(())
}
