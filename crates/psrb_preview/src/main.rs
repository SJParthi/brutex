//! SCRATCH PREVIEW, never committed to any branch: one fixed PSRB backtest over the F&O
//! stocks, every timeframe, three rule sets, each side and both together.
//!
//! The spec is `work-20260925/state/psrb-preview-20261106/PREVIEW-SPEC.md` with
//! `OPERATOR-ADDENDUM-20261106.md` (the third rule set, VIDEO-SCREEN). This is NOT the
//! PSRB result: PSRB-STRAT is not built and data parts are still being fixed. It opens
//! every store file read-only through `store::file::BarFile::open_existing`, installs
//! no telemetry sink, writes only under `--out`, and never deletes a file.

#![forbid(unsafe_code)]

mod common;
mod phase_a;
mod phase_b;
mod report;

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

use crate::common::{Cand, N_RS, N_TF, OPEN_MIN, at_min, day_number, session_day};
use crate::phase_a::{Counts, Ctx, Levels, StockOut, Sym, cpr_narrow};
use crate::phase_b::{AccountOut, Stats};

/// One of the 54 rows: a rule set, a timeframe and an account.
pub(crate) struct Row {
    pub(crate) rs: usize,
    pub(crate) tf: usize,
    pub(crate) acct: usize,
    pub(crate) out: AccountOut,
    pub(crate) stats: Stats,
}

/// Everything the reports read.
pub(crate) struct Run {
    pub(crate) command: String,
    pub(crate) tree_sha: String,
    pub(crate) syms: Vec<Sym>,
    /// Candidates per (rule set x 6 + timeframe), sorted by (T, symbol rank, side).
    pub(crate) groups: Vec<Vec<Cand>>,
    pub(crate) rows: Vec<Row>,
    pub(crate) counts: Counts,
    pub(crate) notes: Vec<String>,
    pub(crate) spans: Vec<String>,
    pub(crate) checks: Vec<String>,
    pub(crate) membership_label: String,
    pub(crate) membership_digest: String,
    pub(crate) cost_version: String,
    pub(crate) threads: usize,
    pub(crate) phase_a: Duration,
    pub(crate) phase_b: Duration,
    pub(crate) started: String,
    pub(crate) stocks_run: usize,
    pub(crate) trades_tsv: bool,
}

struct Args {
    store: PathBuf,
    membership: PathBuf,
    universe: PathBuf,
    out: PathBuf,
    threads: usize,
    only: Vec<String>,
    limit: Option<usize>,
    trades_tsv: bool,
    tree_sha: String,
    command: String,
}

fn parse_args() -> Result<Args, String> {
    let raw: Vec<String> = std::env::args().collect();
    let command = raw.join(" ");
    let mut store = None;
    let mut membership = None;
    let mut universe = None;
    let mut out = None;
    let mut threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let mut only = Vec::new();
    let mut limit = None;
    let mut trades_tsv = true;
    let mut tree_sha = "unknown".to_owned();
    let mut it = raw.iter().skip(1);
    while let Some(a) = it.next() {
        let mut val = || it.next().cloned().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--store" => store = Some(PathBuf::from(val()?)),
            "--membership" => membership = Some(PathBuf::from(val()?)),
            "--universe" => universe = Some(PathBuf::from(val()?)),
            "--out" => out = Some(PathBuf::from(val()?)),
            "--threads" => threads = val()?.parse().map_err(|e| format!("--threads: {e}"))?,
            "--only" => only = val()?.split(',').map(str::to_owned).collect(),
            "--limit" => limit = Some(val()?.parse().map_err(|e| format!("--limit: {e}"))?),
            "--tree-sha" => tree_sha = val()?,
            "--no-trades-tsv" => trades_tsv = false,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(Args {
        store: store.ok_or("--store is required")?,
        membership: membership.ok_or("--membership is required")?,
        universe: universe.ok_or("--universe is required")?,
        out: out.ok_or("--out is required")?,
        threads: threads.max(1),
        only,
        limit,
        trades_tsv,
        tree_sha,
        command,
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("psrb_preview: STOPPED: {e}");
            ExitCode::FAILURE
        }
    }
}

fn load_universe(path: &Path) -> Result<Vec<Sym>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut names: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect();
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
    // COSTS CN01: 400 bought at Rs 1,250, sold at Rs 1,262.50, 2018-09-03.
    let leg = |side: OrderSide, price: i64| Order {
        side,
        by_broker: false,
        fills: vec![Fill { second: 0, quantity: 400, price: Paisa::from_raw(price) }],
    };
    let trade = TradeFills { orders: vec![leg(OrderSide::Buy, 1_250_00), leg(OrderSide::Sell, 1_262_50)] };
    let day = TradeDay::new(2018, 9, 3).map_err(|e| format!("CN01 day: {e:?}"))?;
    let p = price_trade(&trade, day).map_err(|e| format!("CN01 price_trade refused: {e}"))?;
    let (dated, today) = (p.dated_official().total.raw(), p.today_official().total.raw());
    if dated != 313_25 || today != 225_79 {
        return Err(format!("self-check CN01 failed: dated official {dated}, today official {today} paisa (expected 31325, 22579)"));
    }
    lines.push(format!(
        "CN01 (400 at Rs 1,250.00, sold Rs 1,262.50, 2018-09-03): dated official {} and today official {} (expected Rs 313.25 and Rs 225.79): OK",
        common::rupees(i128::from(dated)),
        common::rupees(i128::from(today))
    ));
    let version = p.version().to_owned();
    // The CPR tie at the i64 extreme (AD:536-543).
    let k: i128 = 9_214_157_878_975_800;
    let (h, l, c) = (1001 * k, 998 * k, 1001 * k);
    if !(cpr_narrow(h, l, c, 10) && !cpr_narrow(h, l, c, 9) && !cpr_narrow(h, l - 1, c, 10)) {
        return Err("self-check CPR tie failed".to_owned());
    }
    lines.push("CPR-1 tie (H = C = 1001k, L = 998k): narrow at 10 bp, not at 9 bp; with L one paisa lower not narrow at 10 bp: OK".to_owned());
    // The EMA fixture (AD:572-578).
    let mut e = Ema::new(200);
    for _ in 0..199 {
        e.fold(101);
    }
    let warm199 = e.warm();
    e.fold(100);
    let st = e.scaled();
    let short = 100 * SCALE < st;
    let long = 100 * SCALE > st;
    if warm199 || !e.warm() || st != 100_995_000 || !short || long {
        return Err(format!("self-check EMA fixture failed: scaled {st}, short {short}, long {long}"));
    }
    lines.push("EMA 200 fixture (199 closes of 101, then 100): state 100995000, close 100 short true, long false; not warm after 199: OK".to_owned());
    lines.push(phase_a::grid_check(store, "RELIANCE", 2024, 3)?);
    Ok((lines, version))
}

fn probe_membership(dir: &Path) -> Result<(Option<Membership>, String, String, String), String> {
    let first = session_day(day_number(2018, 9, 3)?)?;
    let last = session_day(day_number(2026, 9, 30)?)?;
    match Membership::open(dir) {
        Err(e) => Ok((
            None,
            format!("survivorship: today's list on every day (the membership folder was refused: {e})"),
            "today's list".to_owned(),
            format!("membership probe: open refused: {e}"),
        )),
        Ok(ms) => {
            let a = ms.is_member("RELIANCE", first);
            let b = ms.is_member("RELIANCE", last);
            let c = ms.is_member("SYRMA", last);
            let line = format!(
                "membership probe: RELIANCE 2018-09-03 {a:?}, RELIANCE 2026-09-30 {b:?}, SYRMA 2026-09-30 {c:?}"
            );
            if matches!(a, Ok(true)) && matches!(b, Ok(true)) && !matches!(c, Ok(true)) {
                let digest = ms.file_digest_hex();
                Ok((
                    Some(ms),
                    format!(
                        "dated membership from {} (inferred from GDFL stock-option listings, 2018-09..2026-09; 2026-09-30 carried into 2026-10)",
                        dir.display()
                    ),
                    digest,
                    format!("{line}: OK"),
                ))
            } else {
                Ok((
                    None,
                    "survivorship: today's list on every day (the membership probe answered otherwise)".to_owned(),
                    "today's list".to_owned(),
                    format!("{line}: FAILED, today's list used"),
                ))
            }
        }
    }
}

fn hms(secs: u64) -> String {
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60)
}

#[allow(clippy::too_many_lines, reason = "the run's one sequence")]
fn run() -> Result<(), String> {
    let t0 = Instant::now();
    let args = parse_args()?;
    std::fs::create_dir_all(&args.out).map_err(|e| format!("{}: {e}", args.out.display()))?;
    let started = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?;
        let us = i64::try_from(now.as_micros()).map_err(|_| "clock".to_owned())?;
        common::fmt_ist(us)
    };
    eprintln!("psrb_preview: started {started} IST, threads {}", args.threads);
    let mut syms = load_universe(&args.universe)?;
    let universe_n = syms.len();
    if !args.only.is_empty() {
        syms.retain(|s| args.only.iter().any(|o| *o == s.name));
    }
    if let Some(n) = args.limit {
        syms.truncate(n);
    }
    eprintln!("psrb_preview: universe {universe_n} names, running {}", syms.len());
    let levels = Levels::new()?;
    let (mut checks, cost_version) = self_checks(&args.store)?;
    let (membership, membership_label, membership_digest, probe_line) = probe_membership(&args.membership)?;
    checks.push(probe_line);
    for c in &checks {
        eprintln!("self-check: {c}");
    }
    let ctx = Ctx {
        store: &args.store,
        membership: membership.as_ref(),
        levels,
        win_first: day_number(2018, 9, 3)?,
        win_last: day_number(2026, 10, 30)?,
        cas_day: day_number(2026, 8, 3)?,
        outage_day: day_number(2021, 2, 24)?,
        gdfl_gap_day: day_number(2019, 5, 30)?,
        oct_first: day_number(2026, 10, 1)?,
        carry_day: session_day(day_number(2026, 9, 30)?)?,
    };
    let start_ts = at_min(ctx.win_first, OPEN_MIN);
    // Phase A, one stock per worker, progress every minute.
    let done = Arc::new(AtomicUsize::new(0));
    let ncand = Arc::new(AtomicU64::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let total = syms.len();
    let progress = {
        let (done, ncand, stop) = (Arc::clone(&done), Arc::clone(&ncand), Arc::clone(&stop));
        std::thread::spawn(move || {
            let start = Instant::now();
            let mut last = 0_u64;
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_secs(1));
                let secs = start.elapsed().as_secs();
                if secs >= last + 60 {
                    last = secs;
                    let n = u64::try_from(done.load(Ordering::Relaxed)).unwrap_or(0);
                    let hundredths = (n * 6_000).checked_div(secs).unwrap_or(0);
                    eprintln!(
                        "[progress {}] stocks done {n}/{total}, rate {}.{:02} stocks/min, candidates {}",
                        hms(secs),
                        hundredths / 100,
                        hundredths % 100,
                        ncand.load(Ordering::Relaxed)
                    );
                }
            }
        })
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build()
        .map_err(|e| format!("thread pool: {e}"))?;
    let ta = Instant::now();
    let mut results: Vec<StockOut> = pool.install(|| {
        syms.par_iter()
            .with_max_len(1)
            .map(|s| {
                let st = Instant::now();
                let r = phase_a::run_stock(&ctx, s);
                done.fetch_add(1, Ordering::Relaxed);
                ncand.fetch_add(u64::try_from(r.cands.len()).unwrap_or(0), Ordering::Relaxed);
                eprintln!(
                    "[stock] {} candidates {} in {} s{}",
                    s.name,
                    r.cands.len(),
                    st.elapsed().as_secs(),
                    r.fatal.as_ref().map_or_else(String::new, |f| format!(" FATAL {f}"))
                );
                r
            })
            .collect()
    });
    let phase_a_time = ta.elapsed();
    stop.store(true, Ordering::Relaxed);
    let _ = progress.join();
    let fatal: Vec<String> = results.iter().filter_map(|r| r.fatal.clone()).collect();
    if !fatal.is_empty() {
        return Err(format!("{} stock(s) stopped the run: {}", fatal.len(), fatal.join(" | ")));
    }
    let mut counts = Counts::default();
    let mut notes = Vec::new();
    let mut spans = Vec::new();
    let mut groups: Vec<Vec<Cand>> = (0..N_RS * N_TF).map(|_| Vec::new()).collect();
    for r in &mut results {
        counts.add(&r.counts);
        notes.append(&mut r.notes);
        spans.push(r.span.clone());
        for c in r.cands.drain(..) {
            let g = usize::from(c.rs) * N_TF + usize::from(c.tf);
            if let Some(v) = groups.get_mut(g) {
                v.push(c);
            }
        }
        r.cands.shrink_to_fit();
    }
    drop(results);
    let tb = Instant::now();
    groups.par_iter_mut().for_each(|v| v.sort_by_key(|c| (c.t_signal, c.stock, c.side)));
    let keys: Vec<(usize, usize, usize)> =
        (0..N_RS).flat_map(|rs| (0..N_TF).flat_map(move |tf| (0..3).map(move |a| (rs, tf, a)))).collect();
    let rows: Vec<Row> = pool.install(|| {
        keys.par_iter()
            .map(|&(rs, tf, acct)| {
                let g = groups.get(rs * N_TF + tf).map_or(&[][..], Vec::as_slice);
                let out = phase_b::admit(g, acct);
                let stats = phase_b::metrics(g, &out.trades, start_ts);
                Row { rs, tf, acct, out, stats }
            })
            .collect()
    });
    let phase_b_time = tb.elapsed();
    let run = Run {
        command: args.command.clone(),
        tree_sha: args.tree_sha.clone(),
        stocks_run: syms.len(),
        syms,
        groups,
        rows,
        counts,
        notes,
        spans,
        checks,
        membership_label,
        membership_digest,
        cost_version,
        threads: args.threads,
        phase_a: phase_a_time,
        phase_b: phase_b_time,
        started,
        trades_tsv: args.trades_tsv,
    };
    report::write_all(&run, &args.out, t0)?;
    eprintln!(
        "psrb_preview: done in {} (phase A {}, phase B {}); outputs in {}",
        hms(t0.elapsed().as_secs()),
        hms(phase_a_time.as_secs()),
        hms(phase_b_time.as_secs()),
        args.out.display()
    );
    Ok(())
}
