//! `feat-run`: the run driver of `common_feat` v1 (ADANIENT, 3-MINUTE FIRST).
//!
//! ```text
//! feat-run ca       --outdir D                      the Zerodha re-basing check (Kite's step per event, CA-BASIS, undo test)
//! feat-run calib    --outdir D                      the calibration table (Zerodha 3-minute, 2026-04-01 09:15) + EMA sweep
//! feat-run readings --tf 3min --htf 15min,60min --outdir D     every candle's RAW readings (TSV) + run counts
//! feat-run steps    --tf 3min --times 1,10,100 --outdir D      the step-count gate (identical p50 / p99 / max)
//! feat-run recon    --threads 14 --outdir D         Zerodha candles against the corrected GDFL fold, all minute TFs
//! feat-run seconds  --tf 5 --from 2026-03-16 --to 2026-04-01 --outdir D   a seconds family from the corrected ticks
//! common: --store P --ticks P --dates P --bhav P --symbol ADANIENT
//! ```
//! Every output file is new (refused if it exists), written to a temporary name, synced and renamed.

#![allow(
    clippy::many_single_char_names,
    reason = "the driver's short names (a = args, c = context, d = out folder, h = feeds, l = levels, r = readings, s = stream) as in the library"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use common_feat::cal::{parse_ymd, ymd};
use common_feat::ind::EmaSeed;
use common_feat::num::rupees_s;
use feat_io::host::{self, Stream};
use feat_io::run::{self, Htf};
use feat_io::{calib_check, defaults, load_ctx, minute_stream, recon};

struct Args {
    cmd: String,
    kv: BTreeMap<String, String>,
}

impl Args {
    fn get(&self, k: &str, d: &str) -> String {
        self.kv.get(k).cloned().unwrap_or_else(|| d.to_owned())
    }
    fn path(&self, k: &str, d: &str) -> PathBuf {
        PathBuf::from(self.get(k, d))
    }
}

fn args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().ok_or("usage: feat-run <ca|calib|readings|steps|recon|seconds> [--key value]...")?;
    let mut kv = BTreeMap::new();
    while let Some(k) = it.next() {
        let k = k.strip_prefix("--").ok_or_else(|| format!("expected --key, got {k}"))?.to_owned();
        let v = it.next().ok_or_else(|| format!("--{k} needs a value"))?;
        kv.insert(k, v);
    }
    Ok(Args { cmd, kv })
}

fn now_ist() -> String {
    // epoch seconds -> IST wall clock (labels in files; the progress file's labels come from `date`)
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let t = i64::try_from(t).unwrap_or(0);
    let d = common_feat::cal::ist_day(t);
    let s = common_feat::cal::ist_sod(t);
    format!("{} {:02}:{:02}:{:02} IST", ymd(d), s / 3600, (s / 60) % 60, s % 60)
}

/// Writes a NEW file: refused if it exists; temp, sync, rename.
fn write_new(path: &Path, body: &[u8]) -> Result<(), String> {
    if path.exists() {
        return Err(format!("{} exists: refused (never overwrite)", path.display()));
    }
    let tmp = path.with_extension("tmp-write");
    let mut f = std::fs::File::create_new(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    f.write_all(body).map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn outdir(a: &Args) -> Result<PathBuf, String> {
    let d = a.path("outdir", "");
    if d.as_os_str().is_empty() {
        return Err("--outdir is required".into());
    }
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

fn header_lines(a: &Args, what: &str) -> String {
    format!(
        "# feat-run {what}: common_feat law {} (build {}), written {}\n# store {} ; ticks {} ; dates {} ; bhav {} ; symbol {}\n",
        common_feat::LAW,
        if common_feat::RED { "RED: no headline or search run" } else { "standard" },
        now_ist(),
        a.get("store", defaults::STORE),
        a.get("ticks", defaults::TICKS),
        a.get("dates", defaults::DATES),
        a.get("bhav", defaults::BHAV),
        a.get("symbol", "ADANIENT")
    )
}

fn ctx(a: &Args, from_first: bool) -> Result<host::Ctx, String> {
    let start = if from_first { parse_ymd("2018-09-03").unwrap_or(0) } else { parse_ymd(&a.get("cont-start", "2018-09-07")).ok_or("cont-start")? };
    load_ctx(&a.path("dates", defaults::DATES), &a.path("bhav", defaults::BHAV), start)
}

fn htfs(a: &Args, store: &Path, sym: &str, c: &host::Ctx, def: &str) -> Result<Vec<Htf>, String> {
    let mut v = Vec::new();
    for t in a.get("htf", def).split(',').filter(|s| !s.is_empty()) {
        let (s, _) = minute_stream(store, sym, t, c);
        v.push(Htf::new(t, s)?);
    }
    Ok(v)
}

fn cmd_ca(a: &Args) -> Result<(), String> {
    let d = outdir(a)?;
    let c = ctx(a, true)?;
    let store = a.path("store", defaults::STORE);
    let sym = a.get("symbol", "ADANIENT");
    let (zday, st) = host::zerodha_stream(&store, &sym, "1day", &c, false);
    let tol: i64 = a.get("tol-ticks", "2").parse().map_err(|_| "tol-ticks")?;
    let mut body = header_lines(a, "ca");
    let _ = writeln!(body, "# Zerodha 1day bars read {}, not on a session {}; notes: {}", st.bars, st.not_session, st.notes.len());
    for n in &st.notes {
        let _ = writeln!(body, "# note: {n}");
    }
    let _ = writeln!(body, "# tol_ticks (PROVISIONAL row, not DP-CA-RULES-v1's sealed ratio_tol_ticks) = {tol}");
    body.push_str(&feat_io::camea::check(&c, &zday, tol));
    write_new(&d.join("CA-CHECK.tsv"), body.as_bytes())?;
    println!("wrote {}", d.join("CA-CHECK.tsv").display());
    Ok(())
}

fn cmd_calib(a: &Args) -> Result<(), String> {
    let d = outdir(a)?;
    let c = ctx(a, false)?;
    let store = a.path("store", defaults::STORE);
    let sym = a.get("symbol", "ADANIENT");
    let (s3, st) = minute_stream(&store, &sym, "3min", &c);
    let (s1, _) = minute_stream(&store, &sym, "1min", &c);
    let facts = host::day_facts(&s1, &c);
    let lv = host::levels(&c, &facts);
    let day = parse_ymd("2026-04-01").ok_or("day")?;
    let mut h = htfs(a, &store, &sym, &c, "15min,60min")?;
    let res = run::run(&c, &s3, &lv, &mut h, None, Some((day, 9 * 3600 + 15 * 60)))?;
    let (r, l) = res.calib.ok_or("the calibration candle 2026-04-01 09:15 is not in the stream")?;
    let ours = run::calib_values(&r);
    let mut body = header_lines(a, "calib");
    let _ = writeln!(body, "# source: {}", s3.source);
    let _ = writeln!(body, "# bars read {} undone {} insane {} not-a-session {} outside-segment {} straddle-kept {} before-start {}", st.bars, st.undone, st.insane, st.not_session, st.outside_segment, st.straddle_kept, st.before_start);
    let _ = writeln!(body, "# CONT from {} ; readiness rule seed_weight_max_ppm 1000 (PROVISIONAL) ; candles folded {}", ymd(c.cont_start), res.rows);
    let _ = writeln!(body, "# THE CALIBRATION VECTOR: ADANIENT 3-minute candle 2026-04-01 09:15 (the operator's Dhan chart is a TEST of the formulas, never data)");
    let _ = writeln!(body, "value\tchart\tmodule (Zerodha 3min)\tdiff vs chart\tindependent node ref (same Zerodha candles)\tCAL-2 re-derived (= ref at 2 dp)\tcomputed task's range (GDFL-tick figures: NOT Zerodha, NOT a gate)\tin that range (NOT a gate)");
    let mut all = true;
    let mut task_all = true;
    for row in calib_check(&ours) {
        let diff = match (feat_io::ticks::parse_paisa(&row.ours), feat_io::ticks::parse_paisa(&row.chart)) {
            (Some(x), Some(y)) => common_feat::num::rupees(x - y),
            _ => "NA".into(),
        };
        all &= row.ref_pass;
        task_all &= row.task_pass;
        let pf = |b: bool| if b { "PASS" } else { "FAIL" };
        let _ = writeln!(body, "{}\t{}\t{}\t{diff}\t{}\t{}\t{}\t{}", row.name, row.chart, row.ours, row.zref, pf(row.ref_pass), row.task_range, if row.task_pass { "yes" } else { "NO (named difference)" });
    }
    let _ = writeln!(body, "# NOT ZERODHA, NOT A GATE (SNIPER s13 (5); check-build-feat-v1 F7): every row inside the computed task's GDFL-tick ranges: {}", if task_all { "YES" } else { "NO: each NO row is a named data difference (Zerodha closes vs GDFL ticks), never tuned away" });
    // the EMA start sweep (Pine ta.ema: the first close as the seed)
    let idx = s3.candles.iter().position(|x| x.1 == r.candle).ok_or("calib index")?;
    let _ = writeln!(body, "\n# EMA 200 started `back` candles before the calibration candle (back = candles folded, the calibration candle included)");
    let _ = writeln!(body, "back\tfirst candle\tEMA200 seed=first close (Pine ta.ema)\tEMA200 seed=SMA (lift)");
    let mut hits = Vec::new();
    for back in (300..=420).step_by(1) {
        let f = run::ema_from(&s3, idx, back - 1, EmaSeed::First).map_or("NA".into(), rupees_s);
        let sm = run::ema_from(&s3, idx, back - 1, EmaSeed::Sma).map_or("NA".into(), rupees_s);
        let fc = run::candle_at(&s3, idx + 1 - back).map_or("NA".into(), |x| {
            let s = common_feat::cal::ist_sod(x.start);
            format!("{} {:02}:{:02}", ymd(common_feat::cal::ist_day(x.start)), s / 3600, (s / 60) % 60)
        });
        if f == "1802.98" {
            hits.push(format!("{back} ({fc})"));
        }
        if back % 10 == 0 || f == "1802.98" {
            let _ = writeln!(body, "{back}\t{fc}\t{f}\t{sm}");
        }
    }
    let _ = writeln!(body, "# the chart's 1802.98 (first-close seed) is reproduced when started back = {}", if hits.is_empty() { "none of 300..420".into() } else { hits.join(", ") });
    // levels of the day
    let pv = |p: Option<common_feat::levels::Pivots>| {
        p.map_or("NA".into(), |p| {
            format!(
                "P {} BC {} TC {} R1 {} R2 {} S1 {} S2 {} width_bp {}",
                rupees_s(common_feat::levels::Pivots::at_s(p.p6)),
                rupees_s(common_feat::levels::Pivots::at_s(p.bc6)),
                rupees_s(common_feat::levels::Pivots::at_s(p.tc6)),
                rupees_s(common_feat::levels::Pivots::at_s(p.r1_6)),
                rupees_s(common_feat::levels::Pivots::at_s(p.r2_6)),
                rupees_s(common_feat::levels::Pivots::at_s(p.s1_6)),
                rupees_s(common_feat::levels::Pivots::at_s(p.s2_6)),
                p.cpr_width_bp().map_or("NA".into(), |w| common_feat::num::units_s(w.floor_s(), 2))
            )
        })
    };
    let _ = writeln!(body, "\n# CAL-4 pivots / CPR of 2026-04-01 (previous session 2026-03-30)\nofficial\t{}\nlast trade\t{}", pv(l.piv_d), pv(l.piv_d_last));
    let _ = writeln!(body, "weekly\t{}\nmonthly\t{}", pv(l.piv_w), pv(l.piv_m));
    let g = |x: Option<common_feat::num::Ratio>| x.map_or("NA".into(), |r| common_feat::num::units_s(r.floor_s(), 2));
    let _ = writeln!(
        body,
        "# CAL-5 gap and change (bp): open gap vs official {} / vs last {} ; close change vs official {} / vs last {} (pc official {}, pc last {})",
        g(l.open.zip(l.pc_off).and_then(|(x, b)| host::bp(x, b))),
        g(l.open.zip(l.pc_last).and_then(|(x, b)| host::bp(x, b))),
        g(l.pc_off.and_then(|b| host::bp(r.candle.close, b))),
        g(l.pc_last.and_then(|b| host::bp(r.candle.close, b))),
        l.pc_off.map_or("NA".into(), common_feat::num::rupees),
        l.pc_last.map_or("NA".into(), common_feat::num::rupees)
    );
    let _ = writeln!(body, "# all calibration rows pass: {}", if all { "YES" } else { "NO" });
    write_new(&d.join("CALIBRATION.tsv"), body.as_bytes())?;
    println!("{body}");
    if all { Ok(()) } else { Err("calibration: at least one row FAILS (named above; never tuned away)".into()) }
}

fn cmd_readings(a: &Args) -> Result<(), String> {
    let d = outdir(a)?;
    let c = ctx(a, false)?;
    let store = a.path("store", defaults::STORE);
    let sym = a.get("symbol", "ADANIENT");
    let tf = a.get("tf", "3min");
    let (s, st) = minute_stream(&store, &sym, &tf, &c);
    let (s1, _) = minute_stream(&store, &sym, "1min", &c);
    let facts = host::day_facts(&s1, &c);
    let lv = host::levels(&c, &facts);
    let mut h = htfs(a, &store, &sym, &c, "15min,60min")?;
    let path = d.join(format!("READINGS-{sym}-{tf}.tsv"));
    if path.exists() {
        return Err(format!("{} exists", path.display()));
    }
    let tmp = path.with_extension("tmp-write");
    let f = std::fs::File::create_new(&tmp).map_err(|e| e.to_string())?;
    let mut w = std::io::BufWriter::new(f);
    let t0 = std::time::Instant::now();
    let res = run::run(&c, &s, &lv, &mut h, Some(&mut w), None)?;
    w.flush().map_err(|e| e.to_string())?;
    w.get_ref().sync_all().map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    let (p50, p99, mx) = res.hist.summary();
    let mut body = header_lines(a, "readings");
    let _ = writeln!(body, "tf\t{tf}\nsource\t{}\nrows\t{}\nbars_read\t{}\nundone\t{}\ninsane_refused\t{}\nnot_a_session\t{}\noutside_segment\t{}\nstraddle_kept\t{}\nbefore_cont_start\t{}", s.source, res.rows, st.bars, st.undone, st.insane, st.not_session, st.outside_segment, st.straddle_kept, st.before_start);
    let _ = writeln!(body, "decision_in_halt_rows\t{}\nsession_no_entry_rows\t{}\nhalt_windows\t{}", res.halt_rows, res.no_entry_session_rows, c.sessions.iter().map(|x| x.halts.len()).sum::<usize>());
    let _ = writeln!(body, "pc_last_source\tZerodha's last 1-minute close of the previous session (after the Kite undo), NOT the bhavcopy LAST; beside only (gap_last, chg_last, piv_d_last; check-build-feat-v1 F2)");
    let _ = writeln!(body, "steps_per_candle_p50_p99_max\t{p50}\t{p99}\t{mx}\nunready_rsi14\t{}\nunready_ema200\t{}\nunready_st10x3\t{}\nrows_on_k13_spans\t{}\nwall_s\t{}", res.unready[0], res.unready[1], res.unready[2], res.k13_rows, t0.elapsed().as_secs());
    for x in &h {
        let _ = writeln!(body, "htf\t{}\tforming_reads_refused\t{}\treset_reads_refused\t{}", x.name, x.forming_refused(), x.reset_refused());
    }
    for n in &st.notes {
        let _ = writeln!(body, "note\t{n}");
    }
    write_new(&d.join(format!("READINGS-{sym}-{tf}.summary.tsv")), body.as_bytes())?;
    println!("{body}");
    Ok(())
}

fn cmd_steps(a: &Args) -> Result<(), String> {
    let d = outdir(a)?;
    let c = ctx(a, false)?;
    let store = a.path("store", defaults::STORE);
    let sym = a.get("symbol", "ADANIENT");
    let tf = a.get("tf", "3min");
    let (s, _) = minute_stream(&store, &sym, &tf, &c);
    let mut body = header_lines(a, "steps");
    let _ = writeln!(body, "times\tcandles\tsteps_p50\tsteps_p99\tsteps_max\twall_ns_p50\twall_ns_p99\twall_ns_max");
    let mut first: Option<(u32, u32, u32)> = None;
    let mut same = true;
    for t in a.get("times", "1,10,100").split(',') {
        let times: usize = t.parse().map_err(|_| "times")?;
        let (hist, ns) = run::step_gate(&c, &s, times)?;
        let sm = hist.summary();
        if let Some(f) = first {
            same &= f == sm;
        } else {
            first = Some(sm);
        }
        let _ = writeln!(body, "{times}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", hist.count(), sm.0, sm.1, sm.2, run::pct(&ns, 500), run::pct(&ns, 990), ns.last().copied().unwrap_or(0));
    }
    let _ = writeln!(body, "# HARD gate (per-candle step counts identical p50 / p99 / max at every size): {}", if same { "PASS" } else { "FAIL" });
    write_new(&d.join(format!("STEPS-{sym}-{tf}.tsv")), body.as_bytes())?;
    println!("{body}");
    if same { Ok(()) } else { Err("step gate FAILS".into()) }
}

fn cmd_recon(a: &Args) -> Result<(), String> {
    let d = outdir(a)?;
    let c = ctx(a, true)?;
    let store = a.path("store", defaults::STORE);
    let ticks = a.path("ticks", defaults::TICKS);
    let sym = a.get("symbol", "ADANIENT");
    let threads: usize = a.get("threads", "14").parse().map_err(|_| "threads")?;
    let mut tfs: Vec<(String, Stream)> = Vec::new();
    let mut body = header_lines(a, "recon");
    for t in a.get("tfs", "1min,2min,3min,5min,10min,15min,30min,60min").split(',') {
        let (s, st) = host::zerodha_stream(&store, &sym, t, &c, true);
        let _ = writeln!(body, "# {t}: zerodha bars {} undone {} insane {} not-a-session {} outside-segment {} straddle-kept {} before-start {}", st.bars, st.undone, st.insane, st.not_session, st.outside_segment, st.straddle_kept, st.before_start);
        for n in &st.notes {
            let _ = writeln!(body, "# {t} note: {n}");
        }
        tfs.push((t.to_owned(), s));
    }
    let t0 = std::time::Instant::now();
    let calib_day = parse_ymd("2026-04-01").unwrap_or(0);
    let r = recon::reconcile(&store, &ticks, &sym, &c, &tfs, threads, calib_day);
    let _ = writeln!(body, "# wall {} s, threads {threads}", t0.elapsed().as_secs());
    let _ = writeln!(body, "tf\tmonth\tcompared\texact\tany_beyond_1_tick\topen_beyond\thigh_beyond\tlow_beyond\tclose_beyond\tmax_abs_o\tmax_abs_h\tmax_abs_l\tmax_abs_c\tzerodha_missing\tgdfl_missing\tvolume_equal");
    let mut tot: BTreeMap<usize, recon::Counts> = BTreeMap::new();
    for ((tfi, mk), x) in &r.by_month {
        let name = tfs.get(*tfi).map_or("?", |t| t.0.as_str());
        let _ = writeln!(
            body,
            "{name}\t{:04}-{:02}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            mk / 12,
            mk % 12 + 1,
            x.compared,
            x.exact,
            x.any,
            x.beyond[0],
            x.beyond[1],
            x.beyond[2],
            x.beyond[3],
            x.max_abs[0],
            x.max_abs[1],
            x.max_abs[2],
            x.max_abs[3],
            x.z_missing,
            x.g_missing,
            x.vol_equal
        );
        let e = tot.entry(*tfi).or_default();
        e.compared += x.compared;
        e.exact += x.exact;
        e.any += x.any;
        e.z_missing += x.z_missing;
        e.g_missing += x.g_missing;
        e.vol_equal += x.vol_equal;
        for k in 0..4 {
            if let (Some(a1), Some(b1)) = (e.beyond.get_mut(k), x.beyond.get(k)) {
                *a1 += *b1;
            }
            if let (Some(a1), Some(b1)) = (e.max_abs.get_mut(k), x.max_abs.get(k)) {
                *a1 = (*a1).max(*b1);
            }
        }
    }
    let _ = writeln!(body, "\n# TOTALS\ntf\tcompared\texact\tany_beyond_1_tick\topen\thigh\tlow\tclose\tzerodha_missing\tgdfl_missing\tvolume_equal");
    for (tfi, x) in &tot {
        let name = tfs.get(*tfi).map_or("?", |t| t.0.as_str());
        let _ = writeln!(body, "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", x.compared, x.exact, x.any, x.beyond[0], x.beyond[1], x.beyond[2], x.beyond[3], x.z_missing, x.g_missing, x.vol_equal);
    }
    let _ = writeln!(body, "\n# the tick decoder against the store's 1-s bars (store rule: LTQ > 0 rows only), per month\nmonth\ttick_traded_secs\tstore_bars\tequal\tdiffer\tstore_only\ttick_only\tprice_only_secs\tnewpx_changed_secs");
    for (mk, s) in &r.store {
        let _ = writeln!(body, "{:04}-{:02}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", mk / 12, mk % 12 + 1, s.tick_traded, s.store_bars, s.equal, s.differ, s.store_only, s.tick_only, s.price_only, s.newpx_changed);
    }
    write_new(&d.join("RECON-SUMMARY.tsv"), body.as_bytes())?;
    write_new(&d.join("RECON-MISMATCHES.tsv"), format!("tf\tdate\tstart\tz_open\tz_high\tz_low\tz_close\tg_open\tg_high\tg_low\tg_close\tz_volume\tg_volume\ttick\n{}", r.mism).as_bytes())?;
    write_new(&d.join("RECON-ZERODHA-MISSING.tsv"), format!("tf\tdate\tstart\tg_open\tg_high\tg_low\tg_close\tg_volume\n{}", r.zmiss).as_bytes())?;
    write_new(&d.join("RECON-GDFL-MISSING.tsv"), format!("tf\tdate\tstart\tz_open\tz_high\tz_low\tz_close\tz_volume\n{}", r.gmiss).as_bytes())?;
    write_new(&d.join("RECON-DAY-NOTES.tsv"), r.notes.as_bytes())?;
    let mut cs = String::from("sod\topen\thigh\tlow\tclose\tvolume\ttraded_ohlc\n");
    for x in &r.calib_secs {
        let _ = writeln!(cs, "{}\t{}\t{}\t{}\t{}\t{}\t{:?}", x.sod, x.open, x.high, x.low, x.close, x.volume, x.traded);
    }
    write_new(&d.join("NEWPX-1S-2026-04-01.tsv"), cs.as_bytes())?;
    println!("{body}");
    Ok(())
}

fn cmd_seconds(a: &Args) -> Result<(), String> {
    let d = outdir(a)?;
    let c = ctx(a, false)?;
    let ticks = a.path("ticks", defaults::TICKS);
    let sym = a.get("symbol", "ADANIENT");
    let tf: i64 = a.get("tf", "5").parse().map_err(|_| "tf")?;
    let from = parse_ymd(&a.get("from", "2026-03-16")).ok_or("from")?;
    let to = parse_ymd(&a.get("to", "2026-04-01")).ok_or("to")?;
    // REPAIR 2: the ONE corrected session-seconds read (host::session_seconds; halt-window volume-0 seconds dropped, counted).
    let mut one = Vec::new();
    let mut dropped = 0_u64;
    let mut dropped_days = Vec::new();
    for (i, s) in c.sessions.iter().enumerate().filter(|(_, s)| s.day >= from && s.day <= to) {
        let Some(day) = host::session_seconds(&ticks, &sym, s)? else { continue };
        if day.halt_dropped > 0 {
            dropped += day.halt_dropped;
            dropped_days.push(format!("{} {}", ymd(s.day), day.halt_dropped));
        }
        one.extend(day.candles.into_iter().map(|x| (i, x)));
    }
    let s1 = Stream { tf_s: 1, source: "gdfl ticks, corrected 1-s (LTQ = 0 new-price rows kept; halt-window volume-0 seconds dropped)".into(), candles: one };
    let s = host::fold_stream(&s1, tf, &c, &format!("{tf}s"));
    let lv = vec![host::Levels::default(); c.sessions.len()];
    let path = d.join(format!("READINGS-{sym}-{tf}s-{}-{}.tsv", ymd(from), ymd(to)));
    let mut buf: Vec<u8> = Vec::new();
    let res = run::run(&c, &s, &lv, &mut [], Some(&mut buf), None)?;
    write_new(&path, &buf)?;
    let (p50, p99, mx) = res.hist.summary();
    println!("seconds family {tf}s {}..{}: {} candles from {} corrected 1-s bars; halt-window volume-0 seconds dropped {dropped} {dropped_days:?}; steps p50 {p50} p99 {p99} max {mx}; wrote {}", ymd(from), ymd(to), res.rows, s1.candles.len(), path.display());
    Ok(())
}

/// S4 (r3, wave 4): `feat-run idx-official --index NIFTY --outdir D [--store P]`: the index official daily rows
/// (`feat_io::idxofficial`), from the store and S1's dated close (`data/idx/INDEX-CLOSE-<IX>.tsv`).
fn cmd_idx_official(a: &Args) -> Result<(), String> {
    let d = outdir(a)?;
    let ix = a.get("index", "");
    let rows = feat_io::instr::load_default()?;
    let row = rows.get(&ix).ok_or_else(|| format!("--index {ix}: no INDEX row in FEAT-INSTR-ROWS-v1.tsv"))?;
    if row.segment != "INDEX" {
        return Err(format!("--index {ix}: not an INDEX row"));
    }
    let store = a.path("store", defaults::STORE);
    // the context without official rows (they are what this builds)
    let c = host::Ctx { sessions: feat_io::data::sessions(&row.dates, &row.special)?, ca: feat_io::data::ca(&row.ca)?, ticks: feat_io::data::ticks(&row.tick)?, bhav: BTreeMap::new(), cont_start: row.cont_start };
    let cp = feat_io::data_dir().join("idx").join(format!("INDEX-CLOSE-{ix}.tsv"));
    let close = feat_io::idxofficial::parse_close(&std::fs::read_to_string(&cp).map_err(|e| format!("{}: {e}", cp.display()))?)?;
    let (out, notes) = feat_io::idxofficial::build(&store, &row.zsymbol, &c, &close)?;
    let body = feat_io::idxofficial::to_tsv(&ix, &out, &notes, &now_ist());
    let p = d.join(format!("INDEX-OFFICIAL-{ix}-v1.tsv"));
    write_new(&p, body.as_bytes())?;
    println!("{}", body.lines().nth(1).unwrap_or(""));
    println!("wrote {} ({} rows, {} notes)", p.display(), out.len(), notes.len());
    Ok(())
}

/// S4 (r3, wave 4): `feat-run idx-dials --index NIFTY --tf 60min --days 2023-06-28,... --outdir D [--store P]`: every candle of the
/// index decision stream on the named sessions read at its END by `feat_io::idxdials` (X-PEER, X-VIX, D-EXP, D-MEND, G-52H, G-52L,
/// G-DRNG, D-DTE), as exact integers / rationals, for the independent node recompute; plus the stream's Muhurat count.
fn cmd_idx_dials(a: &Args) -> Result<(), String> {
    let d = outdir(a)?;
    let ix = a.get("index", "");
    let tf = a.get("tf", "60min");
    let rows = feat_io::instr::load_default()?;
    let row = rows.get(&ix).ok_or_else(|| format!("--index {ix}: no row"))?;
    let peer = rows.get(&row.peer).ok_or_else(|| format!("{ix}: no X-PEER row {}", row.peer))?;
    let store = a.path("store", defaults::STORE);
    let c = row.load_ctx()?;
    let days: Vec<i32> = a.get("days", "").split(',').filter(|s| !s.is_empty()).map(|s| parse_ymd(s).ok_or_else(|| format!("day {s}"))).collect::<Result<_, _>>()?;
    let (s, st) = feat_io::minute_stream_read(&store, &row.zread(), &tf, &c);
    let p = feat_io::idxdials::IdxPrep::prepare(row, peer, &c, &store)?;
    let all = p.read_stream(&c, &s);
    let mut body = header_lines(a, "idx-dials");
    let _ = writeln!(body, "# index {ix} tf {tf}: stream {} candles ({}); muhurat candles excluded {}; store notes {}; peer {} ; ATR holes {}", s.candles.len(), s.source, st.muhurat_excluded, p.notes.len(), peer.underlying, p.atr_holes.len());
    let _ = writeln!(body, "day\tend_ist\tsess\tclose\thi\tlo\tpeer_num\tpeer_den\tvix\td_exp\td_mend\th52\tl52\tdrng_num\tdrng_den\tdte\tdte_known\tdte_sessions\tdte_weekdays\tdte_unverified\tdte_stale\tdte_data_end");
    let opt = |v: Option<i64>| v.map_or_else(|| "-".to_owned(), |x| x.to_string());
    let ob = |v: Option<bool>| v.map_or_else(|| "-".to_owned(), |x| u8::from(x).to_string());
    let mut n = 0_usize;
    for (k, (sess, end, r)) in all.iter().enumerate() {
        let Some(cd) = s.candles.get(k).map(|x| x.1) else { break };
        let Some(day) = c.sessions.get(*sess).map(|x| x.day) else { continue };
        if !days.contains(&day) {
            continue;
        }
        let rng = s.candles.iter().filter(|(i, x)| i == sess && x.end <= *end).fold((i64::MIN, i64::MAX), |(h, l), (_, x)| (h.max(x.high), l.min(x.low)));
        let sod = end - common_feat::cal::at(day, 0, 0, 0);
        let (pn, pd) = r.ctx.nifty_bp.map_or(("-".to_owned(), "-".to_owned()), |(a, b)| (a.to_string(), b.to_string()));
        let (gn, gd) = r.ctx.drng.map_or(("-".to_owned(), "-".to_owned()), |(a, b)| (a.to_string(), b.to_string()));
        let _ = writeln!(
            body,
            "{}\t{:02}:{:02}:{:02}\t{sess}\t{}\t{}\t{}\t{pn}\t{pd}\t{}\t{}\t{}\t{}\t{}\t{gn}\t{gd}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            ymd(day),
            sod / 3600,
            (sod / 60) % 60,
            sod % 60,
            cd.close,
            rng.0,
            rng.1,
            opt(r.ctx.vix),
            ob(r.ctx.exp),
            ob(r.ctx.mend),
            opt(r.ctx.h52),
            opt(r.ctx.l52),
            feat_io::idxdials::dte_word(&r.dte),
            ymd(r.dte.known),
            r.dte.sessions,
            r.dte.weekdays,
            u8::from(r.dte.unverified),
            u8::from(r.dte.stale),
            u8::from(r.dte.data_end)
        );
        n += 1;
    }
    // the calendar dials of every named session, with or without a candle (2024-11-13 lies in the 35-session Zerodha gap, O1)
    let _ = writeln!(body, "#cal\tday\td_exp\tdte\tdte_known\tdte_sessions\tdte_weekdays\tdte_unverified\tdte_stale\tdte_data_end\tcandles");
    for day in &days {
        let t = p.cal.dte(*day);
        let nc = c.index_of(*day).map_or(0, |i| s.candles.iter().filter(|(k, _)| *k == i).count());
        let _ = writeln!(body, "cal\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{nc}", ymd(*day), ob(p.cal.d_exp(*day)), feat_io::idxdials::dte_word(&t), ymd(t.known), t.sessions, t.weekdays, u8::from(t.unverified), u8::from(t.stale), u8::from(t.data_end));
    }
    let path = d.join(format!("IDX-DIALS-{ix}-{tf}.tsv"));
    write_new(&path, body.as_bytes())?;
    println!("idx-dials {ix} {tf}: {n} candles on {} sessions; stream {} candles; muhurat excluded {}; wrote {}", days.len(), s.candles.len(), st.muhurat_excluded, path.display());
    Ok(())
}

fn main() {
    let r = args().and_then(|a| match a.cmd.as_str() {
        "ca" => cmd_ca(&a),
        "calib" => cmd_calib(&a),
        "readings" => cmd_readings(&a),
        "steps" => cmd_steps(&a),
        "recon" => cmd_recon(&a),
        "seconds" => cmd_seconds(&a),
        "idx-official" => cmd_idx_official(&a),
        "idx-dials" => cmd_idx_dials(&a),
        x => Err(format!("unknown command {x}")),
    });
    if let Err(e) = r {
        eprintln!("feat-run: {e}");
        std::process::exit(1);
    }
}
