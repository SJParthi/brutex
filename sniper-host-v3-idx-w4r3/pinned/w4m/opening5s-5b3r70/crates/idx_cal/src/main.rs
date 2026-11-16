//! `idx-cal`: build step S1 (data audit, read only) of SNIPER-IDXOPT-PLAN-v3.1 for NIFTY and BANKNIFTY.
//!
//! ```text
//! idx-cal all --listing F --sessions F --specials F --bhav-root D [--bhav-root D ...] --store ROOT --facts F
//!             --tick-root D --out DIR [--no-store]
//! ```
//! Writes, under `--out`: `DP-IDXCAL-<IX>.tsv`, `IDXCAL-SERIES-<IX>.tsv`, `IDXCAL-VANISH-<IX>.tsv`,
//! `IDXCAL-TRUEDAYS-<IX>.tsv`, `DTE-CLOSURES-<IX>.tsv`, `INDEX-CLOSE-<IX>.tsv`, `FNO-END-ROWS.tsv`, `COVERAGE-<IX>.tsv`,
//! `FOLDERS-<IX>.tsv`, `MUHURAT-LIST.tsv`, `TICK-20261030-CHECK.txt` and the gate report `G-CAL-REPORT.txt`. Nothing else
//! is written anywhere.

#![allow(clippy::too_many_lines, reason = "one driver, one stage after another, in plan order")]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use idx_cal::bhav::{self, Row};
use idx_cal::cal::{self, Cal, RowSource};
use idx_cal::day::{self, Day};
use idx_cal::fno_end;
use idx_cal::listing::{self, FileRow, Special};
use idx_cal::storeio::{self, MonthCache, at_sod, hms, ist_of};
use idx_cal::ticks;
use store::path::Timeframe;

const INDICES: [&str; 2] = ["NIFTY", "BANKNIFTY"];

struct Args {
    listing: PathBuf,
    sessions: PathBuf,
    specials: PathBuf,
    bhav_roots: Vec<PathBuf>,
    store: PathBuf,
    facts: PathBuf,
    tick_root: PathBuf,
    out: PathBuf,
    no_store: bool,
}

fn args() -> Result<Args, String> {
    let v: Vec<String> = std::env::args().collect();
    if v.get(1).map(String::as_str) != Some("all") {
        return Err("usage: idx-cal all --listing F --sessions F --specials F --bhav-root D.. --store ROOT --facts F --tick-root D --out DIR [--no-store]".into());
    }
    let mut a = Args { listing: PathBuf::new(), sessions: PathBuf::new(), specials: PathBuf::new(), bhav_roots: Vec::new(), store: PathBuf::new(), facts: PathBuf::new(), tick_root: PathBuf::new(), out: PathBuf::new(), no_store: false };
    let mut i = 2;
    while let Some(k) = v.get(i) {
        let val = || v.get(i + 1).map(PathBuf::from).ok_or_else(|| format!("{k} needs a value"));
        match k.as_str() {
            "--listing" => a.listing = val()?,
            "--sessions" => a.sessions = val()?,
            "--specials" => a.specials = val()?,
            "--bhav-root" => a.bhav_roots.push(val()?),
            "--store" => a.store = val()?,
            "--facts" => a.facts = val()?,
            "--tick-root" => a.tick_root = val()?,
            "--out" => a.out = val()?,
            "--no-store" => {
                a.no_store = true;
                i += 1;
                continue;
            }
            _ => return Err(format!("unknown argument {k}")),
        }
        i += 2;
    }
    Ok(a)
}

/// The bhavcopy zips as a [`RowSource`], cached per file.
struct BhavSource<'a> {
    roots: &'a [PathBuf],
    index: String,
    cache: BTreeMap<(Day, bool), Vec<Row>>,
    reads: usize,
}

impl RowSource for BhavSource<'_> {
    fn rows(&mut self, file: Day, udiff: bool) -> Result<Vec<Row>, String> {
        if let Some(v) = self.cache.get(&(file, udiff)) {
            return Ok(v.clone());
        }
        let p = bhav::bhav_path(self.roots, file, udiff).ok_or_else(|| format!("no {} bhavcopy zip for {}", if udiff { "UDiFF" } else { "old-form" }, day::fmt(file)))?;
        let csv = bhav::read_single_member(&p)?;
        let rows = bhav::index_rows(&csv, &self.index)?;
        self.reads += 1;
        self.cache.insert((file, udiff), rows.clone());
        Ok(rows)
    }
}

fn write(out: &Path, name: &str, text: &str) -> Result<(), String> {
    fs::write(out.join(name), text).map_err(|e| format!("{name}: {e}"))
}

fn dates(v: &[Day]) -> String {
    if v.is_empty() {
        return "-".to_string();
    }
    v.iter().map(|d| day::fmt(*d)).collect::<Vec<_>>().join(",")
}

/// Groups consecutive session rows with the same (known, true) pair into windows.
fn windows(c: &Cal) -> Vec<(Day, Day, usize, Day, Option<Day>)> {
    let mut g: Vec<(Day, Day, usize, Day, Option<Day>)> = Vec::new();
    for r in &c.rows {
        if Some(r.known_eff) == r.true_current {
            continue;
        }
        match g.last_mut() {
            Some(last) if last.3 == r.known_eff && last.4 == r.true_current => {
                last.1 = r.session;
                last.2 += 1;
            }
            _ => g.push((r.session, r.session, 1, r.known_eff, r.true_current)),
        }
    }
    g
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("idx-cal: {e}");
            std::process::exit(2);
        }
    }
}

fn run() -> Result<(), String> {
    let a = args()?;
    fs::create_dir_all(&a.out).map_err(|e| e.to_string())?;
    let listing = listing::parse_listing(&fs::read_to_string(&a.listing).map_err(|e| format!("listing: {e}"))?)?;
    let sessions = listing::parse_days(&fs::read_to_string(&a.sessions).map_err(|e| format!("sessions: {e}"))?)?;
    let specials = listing::parse_specials(&fs::read_to_string(&a.specials).map_err(|e| format!("specials: {e}"))?)?;
    // Regular sessions: the table less every special row that is not a regular full session (HALT rows are windows
    // inside a regular session and are NOT removed; DEMERGER / DEFECT rows are ADANIENT or data rows and stay regular).
    let not_regular: BTreeSet<Day> = specials.iter().filter(|s| matches!(s.kind.as_str(), "MUHURAT" | "DR" | "WEEKEND")).map(|s| s.day).collect();
    let regular: BTreeSet<Day> = sessions.iter().copied().filter(|d| !not_regular.contains(d)).collect();
    let lo = day::from_civil(2018, 9, 4);
    let hi = day::from_civil(2026, 10, 30);
    let identity = !cfg!(feature = "m_idx_redate");
    let look_ahead = cfg!(feature = "m_idx_cal");
    let mut rep = String::new();
    let _ = writeln!(rep, "G-CAL REPORT (idx-cal, S1 of SNIPER-IDXOPT-PLAN-v3.1; build features: m_idx_redate={} m_idx_cal={})", !identity, look_ahead);
    let _ = writeln!(rep, "session table: {} sessions {}..{}; regular {}; range of rows {}..{}", sessions.len(), dates(&sessions.iter().next().copied().into_iter().collect::<Vec<_>>()), dates(&sessions.iter().next_back().copied().into_iter().collect::<Vec<_>>()), regular.len(), day::fmt(lo), day::fmt(hi));
    let mut cals: BTreeMap<String, Cal> = BTreeMap::new();
    let mut classes_of: BTreeMap<String, BTreeMap<Day, bool>> = BTreeMap::new();
    let mut gate_fail = 0usize;
    for ix in INDICES {
        let files: &Vec<FileRow> = listing.get(ix).ok_or_else(|| format!("listing has no {ix}"))?;
        let bh: BTreeSet<Day> = files.iter().map(|f| f.file).collect();
        let in_range_files = bh.range(lo..=hi).count();
        let table_only: Vec<Day> = sessions.range(lo..=hi).filter(|d| !bh.contains(d)).copied().collect();
        let files_only: Vec<Day> = bh.range(lo..=hi).filter(|d| !sessions.contains(d)).copied().collect();
        let mut src = BhavSource { roots: &a.bhav_roots, index: ix.to_string(), cache: BTreeMap::new(), reads: 0 };
        let c = cal::build(ix, files, &sessions, &regular, lo, hi, &mut src, identity, look_ahead)?;
        let cls = cal::classes(ix, &c.true_days, lo, hi);
        let n_true = cls.len();
        let n_month = cls.values().filter(|m| **m).count();
        let first_weekly = cls.iter().find(|(_, m)| !**m).map(|(d, _)| *d);
        let last_weekly = cls.iter().rev().find(|(_, m)| !**m).map(|(d, _)| *d);
        let _ = writeln!(rep, "\n== {ix}");
        let _ = writeln!(rep, "bhavcopy files {} ({}..{}); in range {in_range_files}; table sessions in range {}; table-only sessions (no bhavcopy) {}; bhavcopy-only dates {}", files.len(), day::fmt(c.files.first().copied().unwrap_or(0)), day::fmt(c.files.last().copied().unwrap_or(0)), sessions.range(lo..=hi).count(), dates(&table_only), dates(&files_only));
        let _ = writeln!(rep, "(2) TRUE EXPIRY DAYS in range: {n_true} = {} weekly + {n_month} monthly; first weekly {}; last weekly {}", n_true - n_month, first_weekly.map_or_else(|| "-".into(), day::fmt), last_weekly.map_or_else(|| "-".into(), day::fmt));
        let mut by: BTreeMap<String, usize> = BTreeMap::new();
        for (d, m) in &cls {
            *by.entry(format!("{}:{}", if *m { "M" } else { "W" }, day::weekday_name(*d))).or_default() += 1;
        }
        let _ = writeln!(rep, "weekday x class: {}", by.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", "));
        // the naive "current as known" reading, for comparison only (never a count)
        let mut naive: BTreeSet<Day> = BTreeSet::new();
        for w in files.windows(2) {
            if let (Some(p), Some(f)) = (w.first(), w.get(1)) {
                if f.file < lo || f.file > hi {
                    continue;
                }
                if let Some(x) = p.listed.iter().map(|l| l.expiry).find(|x| *x >= f.file) {
                    naive.insert(x);
                }
            }
        }
        let naive_extra: Vec<Day> = naive.iter().filter(|x| !c.true_days.contains(x)).copied().collect();
        let true_missing: Vec<Day> = c.true_days.range(lo..=hi).filter(|x| !naive.contains(x)).copied().collect();
        let _ = writeln!(rep, "naive reading (first listed >= d in the file before d; NOT a count): {} dates; extra {}; missing true days {}", naive.len(), dates(&naive_extra), dates(&true_missing));
        let _ = writeln!(rep, "vanish events {} (bhavcopy zips read {}): resolved by fin_id {}, by strike/side {}, unresolved {}", c.vanishes.len(), src.reads, c.vanishes.iter().filter(|v| v.how == "fin_id").count(), c.vanishes.iter().filter(|v| v.how == "strike_side").count(), c.vanishes.iter().filter(|v| v.succ.is_none()).count());
        let max_old = c.vanishes.iter().filter(|v| v.how == "strike_side").map(|v| v.shift).max().unwrap_or(0);
        let max_ud = c.vanishes.iter().filter(|v| v.how == "fin_id").map(|v| v.shift).max().unwrap_or(0);
        let _ = writeln!(rep, "largest re-date: old form {max_old} days (window 3), UDiFF {max_ud} days");
        let _ = writeln!(rep, "3-day window / identity violations: {}", c.window_violations.len());
        for w in &c.window_violations {
            let _ = writeln!(rep, "  VIOLATION {w}");
        }
        let ever = c.series.iter().filter(|s| s.ever_current).count();
        let _ = writeln!(rep, "(1) TRUTH ASSERTION: series ever current as known {ever}; differences {}; open at the data end {}", c.truth_diffs.len(), c.open_at_end.len());
        for d in &c.truth_diffs {
            let _ = writeln!(rep, "  DIFF {d}");
        }
        for d in &c.open_at_end {
            let _ = writeln!(rep, "  DATA-END {d}");
        }
        let _ = writeln!(rep, "stale windows (as-known current expiry != true current expiry):");
        for (f, t, n, k, tr) in windows(&c) {
            let _ = writeln!(rep, "  {}..{} ({n} sessions): known {} ({}), true {}", day::fmt(f), day::fmt(t), day::fmt(k), day::weekday_name(k), tr.map_or_else(|| "none (after the data end)".to_string(), day::fmt));
        }
        let k11 = c.rows.iter().filter(|r| r.file_read >= r.session).count();
        let _ = writeln!(rep, "K11: rows reading a file dated on or after their session: {k11} (must be 0)");
        let dte_diff = c.rows.iter().filter(|r| r.dte_sessions != r.dte_weekdays || !r.specials.is_empty()).count();
        let _ = writeln!(rep, "(5) D-DTE closure list: {dte_diff} sessions with a weekday closure or a special session inside (d, E_known] (dte_closure_unverified; NSE holiday circulars not on disk: O2 (iv))");
        if !c.truth_diffs.is_empty() || !c.window_violations.is_empty() || k11 > 0 {
            gate_fail += 1;
        }
        // files
        let mut t = String::from("# DP-IDXCAL rows (S1, idx-cal): one row per session of the dated session table in range; the file read is the latest bhavcopy dated before the session\nsession\tweekday\tfile_read\tknown_listed\tknown_eff\tseries\tnext_eff\ttrue_current\tstale\td_exp\tclass_known\tdte_sessions\tdte_weekdays\tdte_closure_unverified\tclosures\tspecials\n");
        for r in &c.rows {
            let class_known = cls.get(&r.known_eff).map_or("-", |m| if *m { "M" } else { "W" });
            let stale = r.true_current.is_some() && Some(r.known_eff) != r.true_current;
            let unv = r.dte_sessions != r.dte_weekdays || !r.specials.is_empty();
            let _ = writeln!(t, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{class_known}\t{}\t{}\t{}\t{}\t{}", day::fmt(r.session), day::weekday_name(r.session), day::fmt(r.file_read), day::fmt(r.known_listed), day::fmt(r.known_eff), r.series, r.next_eff.map_or_else(|| "-".into(), day::fmt), r.true_current.map_or_else(|| "-".into(), day::fmt), u8::from(stale), u8::from(r.d_exp), r.dte_sessions, r.dte_weekdays, u8::from(unv), dates(&r.closures), dates(&r.specials));
        }
        write(&a.out, &format!("DP-IDXCAL-{ix}.tsv"), &t)?;
        let mut t = String::from("# series: aliases (listed date [first file..last file]) joined by contract identity; true_last = the last file listing it under any alias; causal_roll = the first session it is current as known with its effective date that day\nseries\taliases\ttrue_last\tcausal_roll\tever_current\topen_at_end\tequal\n");
        for (si, s) in c.series.iter().enumerate() {
            let al: Vec<String> = s.aliases.iter().filter_map(|&i| c.aliases.get(i)).map(|x| format!("{} [{}..{}]", day::fmt(x.date), day::fmt(c.files.get(x.first).copied().unwrap_or(0)), day::fmt(c.files.get(x.last).copied().unwrap_or(0)))).collect();
            let _ = writeln!(t, "{si}\t{}\t{}\t{}\t{}\t{}\t{}", al.join(" -> "), day::fmt(s.true_last), s.causal_roll.map_or_else(|| "-".into(), day::fmt), u8::from(s.ever_current), u8::from(s.open_at_end), u8::from(s.causal_roll == Some(s.true_last)));
        }
        write(&a.out, &format!("IDXCAL-SERIES-{ix}.tsv"), &t)?;
        let mut t = String::from("# vanish events: a listed date absent from the next file before it expired, and its successor by contract identity\nlast_file\tnext_file\tdate\tsuccessor\tshift_days\thow\tdetail\n");
        for v in &c.vanishes {
            let _ = writeln!(t, "{}\t{}\t{}\t{}\t{}\t{}\t{}", day::fmt(v.last_file), day::fmt(v.next_file), day::fmt(v.date), v.succ.map_or_else(|| "-".into(), day::fmt), v.shift, v.how, v.detail);
        }
        write(&a.out, &format!("IDXCAL-VANISH-{ix}.tsv"), &t)?;
        let mut t = String::from("# true expiry days in range (a date whose own bhavcopy lists it); class W/M by the monthly-weekday rule; era weekday from the circulars (cal::monthly_weekday / weekly_weekday); holiday check: an off-weekday day must be the previous session of its era weekday's date, which is not a session\nday\tweekday\tclass\tera_weekday\ton_weekday\tnominal\tholiday_rule_ok\n");
        let mut hol_bad = Vec::new();
        for (d, m) in &cls {
            let (y, mo) = day::ym(*d);
            let w = if *m { cal::monthly_weekday(ix, y, mo) } else { cal::weekly_weekday(ix, *d) };
            let mut nominal = *d;
            while day::weekday(nominal) != w && nominal < *d + 7 {
                nominal += 1;
            }
            let on = nominal == *d;
            let ok = on || (!regular.contains(&nominal) && regular.range(..nominal).next_back() == Some(d));
            if !ok {
                hol_bad.push(format!("{} {} ({}) nominal {}", day::fmt(*d), day::weekday_name(*d), if *m { "M" } else { "W" }, day::fmt(nominal)));
            }
            let _ = writeln!(t, "{}\t{}\t{}\t{}\t{}\t{}\t{}", day::fmt(*d), day::weekday_name(*d), if *m { "M" } else { "W" }, ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].get(w as usize).copied().unwrap_or("?"), u8::from(on), day::fmt(nominal), u8::from(ok));
        }
        write(&a.out, &format!("IDXCAL-TRUEDAYS-{ix}.tsv"), &t)?;
        let _ = writeln!(rep, "(3c) circular eras: off-weekday true days {}; holiday-rule disagreements {} {}", cls.iter().filter(|(d, m)| { let (y, mo) = day::ym(**d); day::weekday(**d) != if **m { cal::monthly_weekday(ix, y, mo) } else { cal::weekly_weekday(ix, **d) } }).count(), hol_bad.len(), hol_bad.join("; "));
        if !hol_bad.is_empty() {
            gate_fail += 1;
        }
        let mut t = String::from("# D-DTE closure list (F2, R6): sessions with a weekday closure or a special session inside (d, E_known]; dte_sessions = regular sessions (ex-post, the session table), dte_weekdays = the causal count with no closure announcement on disk (NSE holiday circulars: O2 (iv)); the search reads dte_sessions with the label dte_closure_unverified until O2 (iv)\nsession\tknown_eff\tdte_sessions\tdte_weekdays\tclosures\tspecials\n");
        for r in c.rows.iter().filter(|r| r.dte_sessions != r.dte_weekdays || !r.specials.is_empty()) {
            let _ = writeln!(t, "{}\t{}\t{}\t{}\t{}\t{}", day::fmt(r.session), day::fmt(r.known_eff), r.dte_sessions, r.dte_weekdays, dates(&r.closures), dates(&r.specials));
        }
        write(&a.out, &format!("DTE-CLOSURES-{ix}.tsv"), &t)?;
        cals.insert(ix.to_string(), c);
        classes_of.insert(ix.to_string(), cls);
    }
    // Muhurat list (Q39)
    let mut t = String::from("# Muhurat sessions (Q39: their candles are excluded from every signal stream: no entry, no flip, no indicator update; positions are held through)\nday\tweekday\tsegments\tsource\n");
    let muh: Vec<&Special> = specials.iter().filter(|s| s.kind == "MUHURAT").collect();
    for s in &muh {
        let _ = writeln!(t, "{}\t{}\t{}\tSESSIONS-SPECIAL-v1.tsv", day::fmt(s.day), day::weekday_name(s.day), s.segments);
    }
    write(&a.out, "MUHURAT-LIST.tsv", &t)?;
    let _ = writeln!(rep, "\nMuhurat list: {} sessions: {}", muh.len(), muh.iter().map(|s| day::fmt(s.day)).collect::<Vec<_>>().join(", "));
    let _ = writeln!(rep, "Muhurat sessions on a true expiry day: {}", INDICES.iter().map(|ix| format!("{ix} {}", muh.iter().filter(|s| cals.get(*ix).is_some_and(|c| c.true_days.contains(&s.day))).count())).collect::<Vec<_>>().join(", "));
    if !a.no_store {
        store_parts(&a, &sessions, &specials, &cals, &classes_of, &mut rep)?;
    }
    let _ = writeln!(rep, "\nG-CAL (Rust side) VERDICT: {}", if gate_fail == 0 { "PASS (items 1, 2, 3b, 3c, 5; item 3a in FOLDERS-*; item 4 is the node recompute)" } else { "FAIL" });
    write(&a.out, "G-CAL-REPORT.txt", &rep)?;
    print!("{rep}");
    Ok(())
}

fn fno_end_of(d: Day, specials: &[Special]) -> i64 {
    for s in specials.iter().filter(|s| s.day == d && s.kind != "HALT" && !s.kind.starts_with("DEMERGER") && !s.kind.starts_with("DEFECT")) {
        if let Some(last) = s.segments.split(',').next_back().and_then(|seg| seg.split_once('-')).map(|(_, e)| e) {
            let p: Vec<i64> = last.split(':').filter_map(|x| x.parse().ok()).collect();
            if let (Some(h), Some(m), Some(sec)) = (p.first(), p.get(1), p.get(2)) {
                return h * 3600 + m * 60 + sec;
            }
        }
    }
    fno_end::regular_end(d)
}

fn store_parts(a: &Args, sessions: &BTreeSet<Day>, specials: &[Special], cals: &BTreeMap<String, Cal>, classes_of: &BTreeMap<String, BTreeMap<Day, bool>>, rep: &mut String) -> Result<(), String> {
    let root = &a.store;
    let lo = day::from_civil(2018, 9, 4);
    let hi = day::from_civil(2026, 10, 30);
    // ---- the dated official index close file
    let facts = fs::read_to_string(&a.facts).map_err(|e| format!("facts: {e}"))?;
    let mut dayend: BTreeMap<(Day, String), (String, i64)> = BTreeMap::new();
    for l in facts.lines().skip(1) {
        let c: Vec<&str> = l.split('\t').collect();
        if let (Some(d), Some(ix), Some(lt), Some(lp)) = (c.first().and_then(|x| day::parse(x)), c.get(1), c.get(8), c.get(9).and_then(|x| x.parse::<i64>().ok())) {
            dayend.insert((d, (*ix).to_string()), ((*lt).to_string(), lp));
        }
    }
    let mut closes: BTreeMap<String, BTreeMap<Day, i64>> = BTreeMap::new();
    for ix in INDICES {
        let mut m: BTreeMap<Day, i64> = BTreeMap::new();
        let mut notes = Vec::new();
        let (mut y, mut mo) = (2018u16, 9u8);
        while (y, mo) <= (2026, 10) {
            match storeio::open_month(root, Vendor::Zerodha, "INDEX", ix, None, Timeframe::DAY_1, y, mo) {
                storeio::MonthFile::Open(f) => {
                    for i in 0..f.records() {
                        let b = f.read_record(i).map_err(|e| e.to_string())?;
                        m.insert(ist_of(b.ts_micros).0, b.close);
                    }
                }
                storeio::MonthFile::Missing => notes.push(format!("{y}-{mo:02} missing")),
                storeio::MonthFile::Refused(e) => notes.push(format!("{y}-{mo:02} refused {e}")),
            }
            (y, mo) = if mo == 12 { (y + 1, 1) } else { (y, mo + 1) };
        }
        // UDiFF UndrlygPric on the true expiry days in the UDiFF era (the settlement days)
        let mut src = BhavSource { roots: &a.bhav_roots, index: ix.to_string(), cache: BTreeMap::new(), reads: 0 };
        let mut und: BTreeMap<Day, i64> = BTreeMap::new();
        if let Some(c) = cals.get(ix) {
            for &d in c.true_days.range(day::from_civil(2024, 7, 8)..=hi) {
                if let Ok(rows) = src.rows(d, true) {
                    let mut cnt: BTreeMap<i64, usize> = BTreeMap::new();
                    for r in &rows {
                        if let Some(u) = r.underlying {
                            *cnt.entry(u).or_default() += 1;
                        }
                    }
                    if let Some((u, _)) = cnt.iter().max_by_key(|(_, n)| **n) {
                        und.insert(d, *u);
                    }
                }
                src.cache.clear();
            }
        }
        let mut t = format!("# INDEX-CLOSE-{ix}: the dated official index close (paisa), one row per session in range: source Zerodha 1day close (store zerodha/NSE/INDEX/{ix}/1day, generation-aware reader); checked against the GDFL tick store's day-end row (INDEX-OPEN-FACTS-R2.tsv last_paisa, from 2018-12-03) and the UDiFF bhavcopy UndrlygPric on the true expiry days from 2024-07-08; the NSE index bhavcopy (ind_close_all) is not on disk. OFFICIAL = the tick-store day-end value where it exists and differs from the Zerodha close by at most 1 paisa (Zerodha 1 paisa low on 40 NIFTY days: float truncation in the vendor candle is the likely cause, not proven), else the Zerodha close (before 2018-12-03, where the day-end row is absent, and on the 2021-02-24 outage day, where the 16:15-cutoff day-end row predates the extended session to 17:00); src names which\nday\tclose_paisa\tdayend_time\tdayend_paisa\tdayend_equal\tudiff_underlying\tudiff_equal\texpiry_day\tofficial\tsrc\n");
        let mut miss_close = Vec::new();
        let mut neq_dayend = Vec::new();
        let mut neq_und = Vec::new();
        let mut n_dayend = 0usize;
        let mut n_und = 0usize;
        let td = cals.get(ix).map(|c| c.true_days.clone()).unwrap_or_default();
        for &d in sessions.range(lo..=hi) {
            let cl = m.get(&d).copied();
            let de = dayend.get(&(d, ix.to_string())).cloned();
            let u = und.get(&d).copied();
            if cl.is_none() {
                miss_close.push(day::fmt(d));
            }
            let de_eq = match (&de, cl) {
                (Some((_, p)), Some(c)) => {
                    n_dayend += 1;
                    if *p == c { "1" } else { neq_dayend.push(format!("{} z {c} t {p}", day::fmt(d))); "0" }
                }
                _ => "-",
            };
            let u_eq = match (u, cl) {
                (Some(p), Some(c)) => {
                    n_und += 1;
                    if p == c { "1" } else { neq_und.push(format!("{} z {c} u {p}", day::fmt(d))); "0" }
                }
                _ => "-",
            };
            let (official, osrc) = match (cl, &de) { (Some(c), Some((_, p))) if (p - c).abs() <= 1 => (Some(*p), if *p == c { "zerodha=dayend" } else { "dayend (zerodha 1 paisa off)" }), (Some(c), _) => (Some(c), "zerodha"), (None, Some((_, p))) => (Some(*p), "dayend"), (None, None) => (None, "-") };
            let _ = writeln!(t, "{}\t{}\t{}\t{}\t{de_eq}\t{}\t{u_eq}\t{}\t{}\t{osrc}", day::fmt(d), cl.map_or_else(|| "-".into(), |x| x.to_string()), de.as_ref().map_or("-", |x| x.0.as_str()), de.as_ref().map_or_else(|| "-".into(), |x| x.1.to_string()), u.map_or_else(|| "-".into(), |x| x.to_string()), u8::from(td.contains(&d)), official.map_or_else(|| "-".into(), |x| x.to_string()));
        }
        write(&a.out, &format!("INDEX-CLOSE-{ix}.tsv"), &t)?;
        let _ = writeln!(rep, "\nCLOSE FILE {ix}: sessions {} ; Zerodha 1day missing on {} [{}]; vs tick-store day-end: compared {n_dayend}, unequal {} [{}]; vs UDiFF UndrlygPric (true expiry days from 2024-07-08): compared {n_und}, unequal {} [{}]; store notes [{}]", sessions.range(lo..=hi).count(), miss_close.len(), miss_close.join(","), neq_dayend.len(), neq_dayend.iter().take(40).cloned().collect::<Vec<_>>().join("; "), neq_und.len(), neq_und.join("; "), notes.join("; "));
        closes.insert(ix.to_string(), m);
    }
    // ---- store folders (G-CAL 3a)
    let mut listings = BTreeMap::new();
    for ix in INDICES {
        let (lst, odd) = storeio::list_contracts(root, ix)?;
        let c = cals.get(ix).ok_or("cal")?;
        let cl = closes.get(ix).ok_or("closes")?;
        let mut t = format!("# FOLDERS-{ix}: G-CAL (3a) store contract folders per true expiry day; the strike nearest that day's official close, CE and PE: the last print's IST day must be the expiry day\nday\tce_folders\tpe_folders\tatm_strike\tce_last_print\tpe_last_print\tok\n");
        let mut bad = Vec::new();
        for &e in c.true_days.range(lo..=hi) {
            let (ce_n, pe_n) = lst.get(&e).map_or((0, 0), |v: &[Vec<i64>; 2]| (v.first().map_or(0, Vec::len), v.get(1).map_or(0, Vec::len)));
            let close = cl.get(&e).copied();
            let strikes: Vec<i64> = lst.get(&e).and_then(|v| v.first()).cloned().unwrap_or_default();
            let atm = close.and_then(|c0| strikes.iter().min_by_key(|k| ((**k - c0).abs(), **k)).copied());
            let mut lasts = Vec::new();
            for ce in [true, false] {
                let lp = atm.and_then(|k| {
                    let name = storeio::contract_name(e, k, ce);
                    let (y, m) = storeio::ym_of(e);
                    match storeio::open_month(root, Vendor::Gdfl, "FNO", ix, Some(&name), Timeframe::SECOND_1, y, m) {
                        storeio::MonthFile::Open(f) => f.records().checked_sub(1).and_then(|i| f.read_record(i).ok()).map(|b| ist_of(b.ts_micros)),
                        _ => None,
                    }
                });
                lasts.push(lp);
            }
            let ok = ce_n > 0 && pe_n > 0 && lasts.iter().all(|l| l.is_some_and(|(d, _)| d == e));
            if !ok {
                bad.push(day::fmt(e));
            }
            let f = |l: &Option<(Day, i64)>| l.map_or_else(|| "-".to_string(), |(d, s)| format!("{} {}", day::fmt(d), hms(s)));
            let _ = writeln!(t, "{}\t{ce_n}\t{pe_n}\t{}\t{}\t{}\t{}", day::fmt(e), atm.map_or_else(|| "-".into(), |k| k.to_string()), f(lasts.first().unwrap_or(&None)), f(lasts.get(1).unwrap_or(&None)), u8::from(ok));
        }
        let not_true: Vec<String> = lst.iter().filter(|(d, _)| !c.true_days.contains(d) && **d >= lo && **d <= hi).map(|(d, v)| format!("{} ({} CE / {} PE)", day::fmt(*d), v.first().map_or(0, Vec::len), v.get(1).map_or(0, Vec::len))).collect();
        let _ = writeln!(t, "# folders under dates that are NOT true expiry days (in range): {}", not_true.join("; "));
        let _ = writeln!(t, "# folder names that do not parse: {}", odd.len());
        write(&a.out, &format!("FOLDERS-{ix}.tsv"), &t)?;
        let _ = writeln!(rep, "\n(3a) STORE FOLDERS {ix}: true expiry days {}; failing (no folders, or the ATM CE/PE last print not on the expiry day) {} [{}]; folders under non-true dates in range: {}", c.true_days.range(lo..=hi).count(), bad.len(), bad.join(","), not_true.join("; "));
        listings.insert(ix.to_string(), lst);
    }
    // ---- coverage + F&O end measurement
    let mut fno_rows = String::from("# FNO-END-ROWS: the dated F&O end per session (seconds of day, exclusive): 15:30:00, 15:40:00 from 2026-08-03 (B5: measured in the store's option prints; NSE circular not on disk), a special session's own last segment end (SESSIONS-SPECIAL-v1.tsv); the index close for entries is 15:30:00 in every era\nday\tfno_end\tindex_close\tsource\n");
    for &d in sessions.range(lo..=hi) {
        let e = fno_end_of(d, specials);
        let src = if e != fno_end::regular_end(d) { "special session segment" } else if d >= fno_end::era_start() { "15:40 era (measured; circular not on disk)" } else { "regular" };
        let _ = writeln!(fno_rows, "{}\t{}\t15:30:00\t{src}", day::fmt(d), hms(e));
    }
    write(&a.out, "FNO-END-ROWS.tsv", &fno_rows)?;
    let mut meas = String::from("# F&O-end measurement 2026-07-27..2026-08-07: the last print of the 6 contracts nearest the 12:00 spot (current expiry as known) and of the GDFL index seconds\n");
    for ix in INDICES {
        let c = cals.get(ix).ok_or("cal")?;
        let lst = listings.get(ix).ok_or("listing")?;
        let rowmap: BTreeMap<Day, &cal::Row1> = c.rows.iter().map(|r| (r.session, r)).collect();
        let final_date: BTreeMap<usize, Day> = c.series.iter().enumerate().map(|(si, s)| (si, s.aliases.last().and_then(|&i| c.aliases.get(i)).map_or(0, |x| x.date))).collect();
        let mut mc = MonthCache::default();
        let mut cur_month = (0u16, 0u8);
        let mut t = format!("# COVERAGE-{ix} (s6.1 re-run in Rust): GDFL index 1-s rows in [09:15:00, 15:30:00); Zerodha 1min candles starting in [09:15:00, 15:30:00); the option 1-s rows of the 3 strikes nearest the 12:00 spot a side (current expiry as known, its store folder = the series' final date) in [09:15:00, F&O end)\nday\tidx1s_rows\tz1m_candles\tspot_paisa\tspot_src\texpiry_folder\tce_strikes\tpe_strikes\topt_rows\tfno_end\n");
        let (mut idx0, mut idxlow, mut z0, mut opt0, mut nos) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for &d in sessions.range(lo..=hi) {
            let ymk = storeio::ym_of(d);
            if ymk != cur_month {
                mc.clear();
                cur_month = ymk;
            }
            let (y, m) = ymk;
            let from = at_sod(d, 9 * 3600 + 15 * 60);
            let to = at_sod(d, fno_end::INDEX_CLOSE_S);
            let noon = at_sod(d, 12 * 3600);
            let (n_idx, spot_g) = match mc.month(root, Vendor::Gdfl, "INDEX", ix, None, Timeframe::SECOND_1, y, m) {
                Ok(v) => {
                    let s = storeio::slice(v, from, to);
                    (s.len(), storeio::slice(v, from, noon).last().map(|b| b.close))
                }
                Err(_) => (0, None),
            };
            let (n_z, spot_z) = match mc.month(root, Vendor::Zerodha, "INDEX", ix, None, Timeframe::MINUTE_1, y, m) {
                Ok(v) => {
                    let s = storeio::slice(v, from, to);
                    (s.len(), storeio::slice(v, from, noon - 60 * storeio::US).last().map(|b| b.close))
                }
                Err(_) => (0, None),
            };
            let (spot, src) = match (spot_g, spot_z) {
                (Some(s), _) => (Some(s), "gdfl_1s"),
                (None, Some(s)) => (Some(s), "zerodha_close"),
                _ => (None, "-"),
            };
            if n_idx == 0 {
                idx0.push(day::fmt(d));
            } else if n_idx < 20_000 {
                idxlow.push(format!("{} {n_idx}", day::fmt(d)));
            }
            if n_z == 0 {
                z0.push(day::fmt(d));
            }
            let end = fno_end_of(d, specials);
            let mut opt_rows = 0usize;
            let mut cks = Vec::new();
            let mut pks = Vec::new();
            let mut folder = 0;
            let mut last_opt: Option<i64> = None;
            if let (Some(sp), Some(r)) = (spot, rowmap.get(&d)) {
                folder = final_date.get(&r.series).copied().unwrap_or(r.known_eff);
                if let Some(v) = lst.get(&folder) {
                    for (side, ce) in [(0usize, true), (1usize, false)] {
                        let mut ks: Vec<i64> = v.get(side).cloned().unwrap_or_default();
                        ks.sort_by_key(|k| ((k - sp).abs(), *k));
                        ks.truncate(3);
                        for k in ks {
                            let name = storeio::contract_name(folder, k, ce);
                            if let Ok(Some(bars)) = storeio::day_bars(root, Vendor::Gdfl, "FNO", ix, Some(&name), Timeframe::SECOND_1, d, 9 * 3600 + 15 * 60, end.max(16 * 3600)) {
                                let within = bars.iter().filter(|b| b.ts_micros < at_sod(d, end)).count();
                                opt_rows += within;
                                if let Some(b) = bars.last() {
                                    last_opt = Some(last_opt.map_or(b.ts_micros, |x: i64| x.max(b.ts_micros)));
                                }
                            }
                            if ce { cks.push(k) } else { pks.push(k) }
                        }
                    }
                }
                if opt_rows == 0 {
                    opt0.push(day::fmt(d));
                }
            } else {
                nos.push(day::fmt(d));
            }
            if d >= day::from_civil(2026, 7, 27) && d <= day::from_civil(2026, 8, 7) {
                let li = match mc.month(root, Vendor::Gdfl, "INDEX", ix, None, Timeframe::SECOND_1, y, m) {
                    Ok(v) => storeio::slice(v, from, at_sod(d, 16 * 3600)).last().map(|b| ist_of(b.ts_micros).1),
                    Err(_) => None,
                };
                let _ = writeln!(meas, "{ix}\t{}\tlast option print {}\tlast index second {}\tdated F&O end {}", day::fmt(d), last_opt.map_or_else(|| "-".into(), |x| hms(ist_of(x).1)), li.map_or_else(|| "-".into(), hms), hms(end));
            }
            let ks = |v: &[i64]| if v.is_empty() { "-".to_string() } else { v.iter().map(i64::to_string).collect::<Vec<_>>().join(",") };
            let _ = writeln!(t, "{}\t{n_idx}\t{n_z}\t{}\t{src}\t{}\t{}\t{}\t{opt_rows}\t{}", day::fmt(d), spot.map_or_else(|| "-".into(), |x| x.to_string()), if folder == 0 { "-".into() } else { day::fmt(folder) }, ks(&cks), ks(&pks), hms(end));
        }
        write(&a.out, &format!("COVERAGE-{ix}.tsv"), &t)?;
        let _ = writeln!(rep, "\nCOVERAGE {ix}: GDFL index 1-s: 0 rows on {} sessions [{}]; below 20,000 rows on {} [{}]; Zerodha 1min: 0 candles on {} sessions [{}]; no spot at 12:00 on {} [{}]; option rows 0 where the spot exists on {} [{}]", idx0.len(), idx0.join(","), idxlow.len(), idxlow.join(", "), z0.len(), z0.join(","), nos.len(), nos.join(","), opt0.len(), opt0.join(","));
        let _ = classes_of;
    }
    for ix in INDICES {
        let gdir = root.join("bar-generations-v1").join("bars").join("gdfl").join("NSE").join("INDEX").join(ix).join("1s");
        let mut g: Vec<String> = fs::read_dir(&gdir).map(|rd| rd.filter_map(Result::ok).map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
        g.sort();
        let _ = writeln!(rep, "GDFL INDEX {ix} 1s generation pointers (months served from a promoted revision through the generation-aware door): {} [{}]", g.len(), g.join(" "));
    }
    write(&a.out, "FNO-END-MEASURED.txt", &meas)?;
    let _ = writeln!(rep, "\nF&O END measured (FNO-END-MEASURED.txt):\n{meas}");
    // ---- the 2026-10-30 tick file: footer, index, CRC of every NIFTY / BANKNIFTY option member
    let p = a.tick_root.join("options").join("2026").join("OCT_2026").join("GFDLNFO_TICK_30102026.bts");
    let mut tk = format!("# TICK FILE CHECK {}\n", p.display());
    match ticks::read_index(&p) {
        Ok((mut f, entries)) => {
            let _ = writeln!(tk, "footer + zstd index + index CRC-32: OK; {} index entries", entries.len());
            let (mut n, mut ok, mut bad) = (0usize, 0usize, Vec::new());
            for e in entries.iter().filter(|e| e.kind != 0) {
                let b = ticks::basename(&e.name).to_string();
                if !(b.starts_with("NIFTY") || b.starts_with("BANKNIFTY")) || b.starts_with("NIFTYNXT") || b.starts_with("NIFTYIT") {
                    continue;
                }
                n += 1;
                match ticks::read_entry(&mut f, e, &b) {
                    Ok((_, info)) if info.crc_ok => ok += 1,
                    Ok(_) => bad.push(format!("{b}: crc not ok")),
                    Err(er) => bad.push(er),
                }
            }
            let _ = writeln!(tk, "NIFTY* / BANKNIFTY* members rebuilt and held to the index size + CRC-32: {n}; OK {ok}; refused {} {}", bad.len(), bad.iter().take(20).cloned().collect::<Vec<_>>().join("; "));
        }
        Err(e) => {
            let _ = writeln!(tk, "REFUSED: {e}");
        }
    }
    write(&a.out, "TICK-20261030-CHECK.txt", &tk)?;
    let _ = writeln!(rep, "\n{tk}");
    Ok(())
}
