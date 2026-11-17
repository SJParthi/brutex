//! v3.1 (MAIN-RULINGS-ENGINE-v22 item 1 and addendum 2): the margin TABLES the engine reads, through THE ONE margin module of crate v2.3
//! (`common_fill::margin`: SPAN + the option's value at NSE's file price + exposure, integer paisa, the rule that rebuilds the operator's
//! Dhan screen). This module only does the I/O around it: the snapshot MANIFEST (`v31/margin/build-snapshots.mjs`: per session its
//! begin-of-day table and, for the 8 Tickvault days, the intraday i2-i4 tables with their HTTP Last-Modified), `zstd -dc` as a child
//! process (no new crate), the per-day contract resolution (each cube contract looked up ONCE per snapshot in the crate's flat O(1) table),
//! and the decision lookup (the latest snapshot published strictly before T, then one row read and a few integer operations: O(1)).
//!
//! No look-ahead: a begin-of-day table is in force from its session's start (NSE publishes i1 the evening before; margin-history proves the
//! XML created stamp is before 09:15); an intraday table from its Last-Modified instant (strictly before T). A session without any table
//! falls back to the latest EARLIER session's last table, labelled and counted (extreme case "NSE file missing for a day").

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use common_fill::margin::{MarginErr, Parts, Snapshot, near_future, short_margin, short_margin_base, stale};

use crate::cube::Cube;
use crate::trade::Refuse;

/// One snapshot table of the manifest.
#[derive(Clone, Debug)]
pub struct SnapMeta {
    /// The session (IST day).
    pub session: i64,
    /// The SPAN file label (e.g. `nsccl.20261102.i2`).
    pub label: String,
    /// A begin-of-day table (in force from the session start).
    pub bod: bool,
    /// An intraday table's effective instant (UTC us; i64::MIN for a begin-of-day table).
    pub eff_us: i64,
    pub path: PathBuf,
    pub sha: String,
}

/// The manifest.
#[derive(Clone, Debug, Default)]
pub struct Manifest {
    pub rows: Vec<SnapMeta>,
    pub path: String,
    pub sha: String,
}

impl Manifest {
    /// Reads the manifest (header first).
    ///
    /// # Errors
    /// An unreadable file or a malformed row.
    pub fn load(p: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let mut rows = Vec::new();
        for (n, l) in text.lines().enumerate().skip(1) {
            if l.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = l.split('\t').collect();
            let (Some(s), Some(lab), Some(kind), Some(eff), Some(path)) = (f.first(), f.get(1), f.get(2), f.get(3), f.get(4)) else { return Err(format!("{} line {}: short row", p.display(), n + 1)) };
            let session = crate::cal::parse_day(s).ok_or_else(|| format!("{} line {}: session {s}", p.display(), n + 1))?;
            let bod = *kind == "bod";
            let eff_us = if bod { i64::MIN } else { eff.parse::<i64>().map_err(|_| format!("{} line {}: effective instant {eff}", p.display(), n + 1))? };
            rows.push(SnapMeta { session, label: (*lab).to_owned(), bod, eff_us, path: PathBuf::from(path), sha: f.get(5).map_or_else(String::new, |x| (*x).to_owned()) });
        }
        rows.sort_by_key(|r| (r.session, r.eff_us));
        Ok(Self { rows, path: p.display().to_string(), sha: crate::cube::sha256(p) })
    }

    /// The tables of one session (begin-of-day first), or the latest earlier session's LAST table as a fallback (with its session).
    #[must_use]
    pub fn of_day(&self, day: i64) -> (Vec<&SnapMeta>, Option<i64>) {
        let v: Vec<&SnapMeta> = self.rows.iter().filter(|r| r.session == day).collect();
        if !v.is_empty() {
            return (v, None);
        }
        match self.rows.iter().rev().find(|r| r.session < day) {
            Some(r) => (vec![r], Some(r.session)),
            None => (Vec::new(), None),
        }
    }
}

/// v3.2 round B (addendum 17): the extra margin manifest (`--margin-extra`): the other NSE indices' option rows of the same SPAN files,
/// one table per (session, label) of the main manifest; set once per run.
pub static EXTRA_MAN: std::sync::OnceLock<Manifest> = std::sync::OnceLock::new();
/// The extra tables appended (counted, printed).
pub static EXTRA_TABLES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// v3.2 round B: appends another table's ROWS (its header line dropped) to a table's bytes (the same columns, the same session).
pub fn append_rows(main: &mut Vec<u8>, extra: &[u8]) {
    // MUTANT m_v32rb_margin_extra_ignored: the other indices' rows never appended
    if cfg!(feature = "m_v32rb_margin_extra_ignored") {
        return;
    }
    let body = extra.iter().position(|&c| c == b'\n').map_or(&[][..], |p| extra.get(p + 1..).unwrap_or(&[]));
    if !main.is_empty() && !main.ends_with(b"\n") {
        main.push(b'\n');
    }
    main.extend_from_slice(body);
}

/// `zstd -dc <file>` (a child process; nothing in the file is executed).
///
/// # Errors
/// The child could not run or failed (a damaged file).
pub fn read_table(p: &Path) -> Result<Vec<u8>, String> {
    let out = Command::new(crate::kit::tool("zstd")).arg("-dc").arg(p).stdin(Stdio::null()).stderr(Stdio::piped()).output().map_err(|e| format!("zstd -dc {}: {e}", p.display()))?;
    if !out.status.success() {
        return Err(format!("zstd -dc {}: exit {} {}", p.display(), out.status, String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(out.stdout)
}

/// Margin rule v2 (addendum 9): the futures' prices per SPAN file (margin-calibration `futures-in-files.tsv`: file, und, fut_expiry,
/// fut_price): (file label, underlying) -> [(expiry yyyymmdd, price paisa)].
#[derive(Clone, Debug, Default)]
pub struct Futures {
    pub by: std::collections::HashMap<(String, String), Vec<(u32, i64)>>,
    pub path: String,
    pub sha: String,
}

impl Futures {
    /// Reads the futures table (header first).
    ///
    /// # Errors
    /// An unreadable file or a malformed row.
    pub fn load(p: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let mut by: std::collections::HashMap<(String, String), Vec<(u32, i64)>> = std::collections::HashMap::new();
        for l in text.lines().skip(1) {
            let f: Vec<&str> = l.split('\t').collect();
            let (Some(file), Some(und), Some(e), Some(px)) = (f.first(), f.get(1), f.get(2), f.get(3)) else { continue };
            let (Ok(e), Some(px)) = (e.parse::<u32>(), common_fill::margin::parse_paisa_round(px.as_bytes())) else { return Err(format!("{}: bad row {l}", p.display())) };
            by.entry(((*file).to_owned(), (*und).to_owned())).or_default().push((e, px));
        }
        Ok(Self { by, path: p.display().to_string(), sha: crate::cube::sha256(p) })
    }

    /// The near-month future's price for an option of `und` expiring `expiry` in the file `label` (the first on or after it).
    #[must_use]
    pub fn base(&self, label: &str, und: &str, expiry: u32) -> Option<i64> {
        self.by.get(&(label.to_owned(), und.to_owned())).and_then(|v| near_future(v, expiry))
    }
}

/// The process-wide futures table (margin rule v2), set once from `--margin-futures`.
pub static FUTURES: std::sync::OnceLock<Futures> = std::sync::OnceLock::new();

/// One session's snapshots, parsed (sorted by effective instant; a begin-of-day table first).
#[derive(Clone, Debug, Default)]
pub struct DayM {
    pub day: i64,
    /// Set when the session had no table: the session whose last table stands in (labelled, counted).
    pub fallback_from: Option<i64>,
    pub snaps: Vec<(i64, String, Snapshot)>,
}

/// Loads one session's snapshots (each table read through `zstd -dc` and parsed by the crate). `verify` checks each file's sha256 against
/// the manifest (a corrupt or partial table is refused, never read).
///
/// # Errors
/// A table that cannot be read, fails its sha256 check, or is not the documented format.
pub fn load_day(m: &Manifest, day: i64, verify: bool) -> Result<DayM, String> {
    let (v, fb) = m.of_day(day);
    let mut snaps = Vec::with_capacity(v.len());
    for r in v {
        if verify && !r.sha.is_empty() {
            let s = crate::cube::sha256(&r.path);
            if s != r.sha {
                return Err(format!("{}: sha256 {s} is not the manifest's {} (corrupt or partial table: refused)", r.path.display(), r.sha));
            }
        }
        let mut bytes = read_table(&r.path)?;
        // v3.2 round B (addendum 17): the other NSE indices' rows of the SAME SPAN file (same session and label), appended (header dropped)
        if let Some(x) = EXTRA_MAN.get().and_then(|em| em.rows.iter().find(|e| e.session == r.session && e.label == r.label)) {
            if verify && !x.sha.is_empty() {
                let s = crate::cube::sha256(&x.path);
                if s != x.sha {
                    return Err(format!("{}: sha256 {s} is not the extra manifest's {} (corrupt or partial table: refused)", x.path.display(), x.sha));
                }
            }
            append_rows(&mut bytes, &read_table(&x.path)?);
            EXTRA_TABLES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        let s = Snapshot::parse(&bytes).map_err(|e| format!("{}: {e}", r.path.display()))?;
        // a fallback table is in force from the session start (it was published before the session)
        snaps.push((if fb.is_some() { i64::MIN } else { r.eff_us }, r.label.clone(), s));
    }
    snaps.sort_by_key(|x| x.0);
    Ok(DayM { day, fallback_from: fb, snaps })
}

/// Lookup latencies (nanoseconds per lookup, timed one by one) and probe counts, per lookup kind.
#[derive(Clone, Debug, Default)]
pub struct Lat {
    /// Contract-day resolution: one underlying hash lookup + one flat-table find per (contract, snapshot).
    pub resolve_ns: Vec<u32>,
    /// The decision lookup: snapshot pick + row read + the margin arithmetic.
    pub decide_ns: Vec<u32>,
    pub probes: Vec<u32>,
}

impl Lat {
    /// Adds another set.
    pub fn merge(&mut self, o: Self) {
        self.resolve_ns.extend(o.resolve_ns);
        self.decide_ns.extend(o.decide_ns);
        self.probes.extend(o.probes);
    }

    /// "p50 / p99 / max" of a sample (sorted copy).
    #[must_use]
    pub fn q(v: &[u32]) -> (u32, u32, u32, usize) {
        let mut s = v.to_vec();
        s.sort_unstable();
        let at = |p: usize| s.get((s.len().saturating_sub(1)) * p / 100).copied().unwrap_or(0);
        (at(50), at(99), s.last().copied().unwrap_or(0), s.len())
    }

    /// The printed line.
    #[must_use]
    pub fn line(&self) -> String {
        let (a50, a99, amax, an) = Self::q(&self.resolve_ns);
        let (b50, b99, bmax, bn) = Self::q(&self.decide_ns);
        let (p50, p99, pmax, _) = Self::q(&self.probes);
        format!("margin lookups (MEASURED, ns per lookup incl. the timer): contract-day resolution p50 {a50} / p99 {a99} / max {amax} ({an} lookups; flat-table probes p50 {p50} / p99 {p99} / max {pmax}); decision lookup p50 {b50} / p99 {b99} / max {bmax} ({bn} lookups)")
    }
}

/// The per-day flat contract index: row id of contract ci in snapshot j at `rows[ci * nsnap + j]` (u32::MAX = not in that table).
#[derive(Clone, Debug, Default)]
pub struct ConM {
    pub nsnap: usize,
    pub rows: Vec<u32>,
}

/// Resolves every contract of the cube in every snapshot of the day ONCE (O(1) each).
#[must_use]
pub fn resolve(dm: &DayM, c: &Cube, lat: &mut Lat) -> ConM {
    let nsnap = dm.snaps.len();
    let mut rows = vec![u32::MAX; c.cons.len() * nsnap];
    for (ci, k) in c.cons.iter().enumerate() {
        let name = c.unds.get(usize::from(k.und)).map_or("", |u| u.name.as_str());
        for (j, (_, _, s)) in dm.snaps.iter().enumerate() {
            let t0 = Instant::now();
            let mut probes = 0_u32;
            let id = s.und_id(name).and_then(|u| s.find_id(u, k.expiry, k.right, k.strike, &mut probes));
            let ns = u32::try_from(t0.elapsed().as_nanos()).unwrap_or(u32::MAX);
            lat.resolve_ns.push(ns);
            lat.probes.push(probes);
            if let (Some(id), Some(slot)) = (id, rows.get_mut(ci * nsnap + j)) {
                *slot = id;
            }
        }
    }
    ConM { nsnap, rows }
}

/// The exact margin of one decision (per lot), with its parts, the snapshot used and the flags.
#[derive(Clone, Copy, Debug, Default)]
pub struct MarginAt {
    /// None = the margin is known; else the refusal (no_margin_file / no_span_row / no_elm_rate).
    pub refuse: Option<Refuse>,
    pub lot_exact: i64,
    pub parts: Parts,
    /// The snapshot index in the day's list (labels: [`DayM::snaps`]).
    pub snap: u8,
    /// The live spot moved more than 1/3 of the price scan since the file (addendum 2: flagged; the stricter reading is beside).
    pub stale: bool,
    /// The day had no table: the latest earlier session's stands in.
    pub fallback: bool,
    /// Margin rule v2: the order's Rs 20 (0 under rule v1).
    pub addon: i64,
    /// Margin rule v2: no near-month future in the file for this index option: the live spot is the exposure base (labelled, counted).
    pub fut_fallback: bool,
    /// fix:v33 IDXELM (standing F5): an index row priced at the dated table's rate: 1 = a CONSERVATIVE row (no dated evidence of 2%), 2 = a
    /// VERIFIED row; 0 = the crate's flat rate (switch off, or a stock row). `elm2` = the ELM of one lot at the rate printed beside (paisa).
    pub idxc: u8,
    pub elm2: i32,
}

/// fix:v33 IDXELM (standing F5): an INDEX row's exposure re-priced at the dated table's rate on trade date `ymd` (the crate's expiry-day
/// add-on kept on top): ELM = rate x base x q rounded once (the crate's own `elm_paisa`), exact = SPAN + premium + ELM, NSE-only = SPAN + ELM;
/// returns (parts, class 1 conservative / 2 verified / 0 untouched, the ELM at the rate printed beside). Off, or a stock row: untouched.
/// Err = the table does not cover the date (refused; `fixg::idx_cover` checks every run session first).
pub fn idx_dated(idx: bool, p: Parts, base: i64, lot: u64, ymd: u32) -> Result<(Parts, u8, i32), ()> {
    if !idx {
        return Ok((p, 0, 0));
    }
    let add = p.rate_h - common_fill::margin::IDX_ELM_H;
    match crate::fixg::idx_rate(ymd, add) {
        Ok(None) => Ok((p, 0, 0)),
        Ok(Some((rate, beside, cons))) => {
            let elm = common_fill::margin::elm_paisa(rate, base, lot).map_err(|_| ())?;
            let elm2 = common_fill::margin::elm_paisa(beside, base, lot).map_err(|_| ())?;
            let nse_only = p.span.checked_add(elm).ok_or(())?;
            let exact = nse_only.checked_add(p.premium).ok_or(())?;
            Ok((Parts { elm, nse_only, exact, rate_h: rate, ..p }, if cons { 1 } else { 2 }, i32::try_from(elm2).unwrap_or(i32::MAX)))
        }
        Err(_) => Err(()),
    }
}

/// The decision lookup: the latest snapshot in force strictly before `t_us`, the contract's row, THE margin for one lot at spot `s`.
#[must_use]
pub fn exact_at(dm: &DayM, cm: &ConM, ci: usize, t_us: i64, s: i64, lot: u64, day_ymd: u32, lat: Option<&mut Lat>) -> MarginAt {
    let t0 = Instant::now();
    let mut out = MarginAt { fallback: dm.fallback_from.is_some(), ..MarginAt::default() };
    // at most a handful of snapshots a day: the scan is O(1)
    // MUTANT m_v31_snap_lookahead: a table published AT the decision instant used (look-ahead by its own second)
    let j = dm.snaps.iter().rposition(|x| if cfg!(feature = "m_v31_snap_lookahead") { x.0 <= t_us } else { x.0 < t_us });
    match j {
        None => out.refuse = Some(Refuse::NoMarginFile),
        Some(j) => {
            out.snap = u8::try_from(j).unwrap_or(u8::MAX);
            let id = cm.rows.get(ci * cm.nsnap + j).copied().unwrap_or(u32::MAX);
            match dm.snaps.get(j).and_then(|x| x.2.row(id)) {
                None => out.refuse = Some(Refuse::NoSpanRow),
                Some(r) => {
                    // fix:v33 IDXELM: the exposure base the crate priced with (the dated index rate re-prices the ELM on it)
                    let mut base_used = s;
                    match {
                        if crate::sw::on(crate::sw::MARGIN_V2) {
                            // addendum 9: an index option's exposure base is the near-month future's price in the same file; a stock's its live price
                            out.addon = common_fill::margin::ORDER_ADDON_PAISA;
                            let base = if r.idx {
                                let und = dm.snaps.get(j).and_then(|x| x.2.unds.get(usize::from(r.und))).map_or("", String::as_str);
                                let label = dm.snaps.get(j).map_or("", |x| x.1.as_str());
                                match FUTURES.get().and_then(|f| f.base(label, und, r.expiry)) {
                                    Some(b) => b,
                                    None => {
                                        out.fut_fallback = true;
                                        s
                                    }
                                }
                            } else {
                                s
                            };
                            base_used = base;
                            // v3.2 addendum 11: margin rule v3 for STOCK options (the AEL OTH row whatever the moneyness; crate v2.4 M3)
                            // MUTANT m_v32_margin_v3_host_v2: rule v2's stock rate kept under the v3 switch
                            if crate::sw::on(crate::sw::MARGIN_V3) && !cfg!(feature = "m_v32_margin_v3_host_v2") { common_fill::margin::short_margin_base_v3(r, lot, s, base, day_ymd) } else { short_margin_base(r, lot, s, base, day_ymd) }
                        } else {
                            short_margin(r, lot, s, day_ymd)
                        }
                    } {
                        Ok(p) => match idx_dated(r.idx, p, base_used, lot, day_ymd) {
                            Ok((p, idxc, elm2)) => {
                                out.parts = p;
                                out.lot_exact = p.exact;
                                out.stale = stale(r, s);
                                out.idxc = idxc;
                                out.elm2 = elm2;
                            }
                            Err(()) => out.refuse = Some(Refuse::NoMarginFile),
                        },
                        Err(MarginErr::NoElmRate) => out.refuse = Some(Refuse::NoElmRate),
                        Err(_) => out.refuse = Some(Refuse::NoUnderlying),
                    }
                }
            }
        }
    }
    if let Some(l) = lat {
        l.decide_ns.push(u32::try_from(t0.elapsed().as_nanos()).unwrap_or(u32::MAX));
    }
    out
}

fn num_paisa(s: &str) -> Option<i64> {
    common_fill::margin::parse_paisa(s.as_bytes())
}

/// `margincheck <manifest> <margin-per-trade.tsv>`: THE proof that the engine's margin module rebuilds margin-truth's per-trade margins:
/// every row's day, decision time, contract, lot and live spot through the snapshot manifest and the crate's rule; margin per lot, SPAN,
/// premium term, ELM, NSE-only reading and the file used compared field by field.
///
/// # Errors
/// An unreadable input.
pub fn cmd_margincheck(a: &[String]) -> Result<String, String> {
    let (Some(mp), Some(tp)) = (a.get(2), a.get(3)) else { return Err("margincheck <manifest> <margin-per-trade.tsv>".to_owned()) };
    let m = Manifest::load(Path::new(mp))?;
    let text = std::fs::read_to_string(tp).map_err(|e| format!("{tp}: {e}"))?;
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().unwrap_or("").split('\t').collect();
    let col = |n: &str| head.iter().position(|h| *h == n).ok_or(format!("column {n}"));
    let (cd, ct, cu, ce, ck, cr, cl, cs, cx, csp, cpr, cel, cno, cf) = (col("day")?, col("decision_T")?, col("underlying")?, col("expiry")?, col("strike")?, col("right")?, col("lot")?, col("spot_S_at_T")?, col("margin_lot_exact")?, col("span_lot")?, col("premium_term_lot")?, col("elm_lot")?, col("nse_only_lot_span_plus_elm")?, col("file_used")?);
    let mut days: std::collections::BTreeMap<i64, (DayM, std::collections::HashMap<(String, u32, u8, i64), Option<u32>>)> = std::collections::BTreeMap::new();
    let (mut n, mut eq, mut bad) = (0_u64, 0_u64, Vec::new());
    let mut lat = Lat::default();
    let mut by_file: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    for l in lines {
        let f: Vec<&str> = l.split('\t').collect();
        let g = |i: usize| f.get(i).copied().unwrap_or("");
        n += 1;
        let day = crate::cal::parse_day(g(cd)).ok_or("day")?;
        let hms: Vec<i64> = g(ct).split(':').filter_map(|x| x.parse().ok()).collect();
        let sod = hms.first().copied().unwrap_or(0) * 3_600 + hms.get(1).copied().unwrap_or(0) * 60 + hms.get(2).copied().unwrap_or(0);
        let t_us = crate::cal::at_sod(day, sod);
        let und = g(cu).to_owned();
        let expiry: u32 = g(ce).parse().map_err(|_| "expiry")?;
        let strike = num_paisa(g(ck)).ok_or("strike")?;
        let right: u8 = if g(cr) == "PE" || g(cr) == "P" { 1 } else { 0 };
        let lot: u64 = g(cl).parse().map_err(|_| "lot")?;
        let s = num_paisa(g(cs)).ok_or("spot")?;
        if let std::collections::btree_map::Entry::Vacant(v) = days.entry(day) {
            v.insert((load_day(&m, day, true)?, std::collections::HashMap::new()));
        }
        let Some((dm, cache)) = days.get_mut(&day) else { continue };
        // the decision lookup exactly as the engine does it: snapshot by time, then the flat-table row
        let j = dm.snaps.iter().rposition(|x| x.0 < t_us);
        let got = j.and_then(|j| {
            let (_, label, snap) = dm.snaps.get(j)?;
            let key = (und.clone(), expiry, right, strike);
            let id = *cache.entry(key).or_insert_with(|| {
                let t0 = Instant::now();
                let mut pr = 0;
                let id = snap.find(&und, expiry, right, strike, &mut pr);
                lat.resolve_ns.push(u32::try_from(t0.elapsed().as_nanos()).unwrap_or(u32::MAX));
                lat.probes.push(pr);
                id
            });
            let t0 = Instant::now();
            let r = snap.row(id?)?;
            let p = short_margin(r, lot, s, crate::cal::ymd_of(day)).ok()?;
            lat.decide_ns.push(u32::try_from(t0.elapsed().as_nanos()).unwrap_or(u32::MAX));
            Some((label.clone(), p))
        });
        let want = (num_paisa(g(cx)), num_paisa(g(csp)), num_paisa(g(cpr)), num_paisa(g(cel)), num_paisa(g(cno)), g(cf).to_owned());
        match got {
            Some((label, p)) if (Some(p.exact), Some(p.span), Some(p.premium), Some(p.elm), Some(p.nse_only), label.clone()) == want => {
                eq += 1;
                *by_file.entry(if label.ends_with(".i1") { "begin-of-day i1".to_owned() } else { "intraday i2-i4".to_owned() }).or_insert(0) += 1;
            }
            other => bad.push(format!("{} {} {} {}: engine {:?} vs margin-truth {:?}", g(cd), g(ct), g(cu), g(ck), other.map(|(lb, p)| (lb, p.exact, p.span, p.premium, p.elm, p.nse_only)), want)),
        }
    }
    let mut out = format!("MARGINCHECK: {n} margin-truth trades (margin-per-trade.tsv sha256 {}), manifest {} (sha256 {}): {eq} EQUAL field by field (margin per lot, SPAN, premium term, ELM, NSE-only, file used), {} different; by file kind: {}\n", crate::cube::sha256(Path::new(tp)), m.path, m.sha, bad.len(), by_file.iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>().join(", "));
    out.push_str(&lat.line());
    out.push('\n');
    for b in bad.iter().take(20) {
        out.push_str(b);
        out.push('\n');
    }
    Ok(out)
}

/// `margincheck2 <manifest> <futures.tsv> <margin-per-trade.tsv> <margin-per-trade-v2.tsv> <calibration.tsv>`: MARGIN RULE v2's proofs: the
/// 782 margin-truth trades re-priced (per-lot margin before the Rs 20 and the lots that fit (slot - Rs 20)) against the calibrator's
/// `margin_lot_v2` / `lots_fit_v2`, and the 15 Dhan screens (per-lot margin + Rs 20 = Dhan) against `calibration.tsv`.
///
/// # Errors
/// An unreadable input.
pub fn cmd_margincheck2(a: &[String]) -> Result<String, String> {
    let (Some(mp), Some(fp), Some(t1), Some(t2), Some(cp)) = (a.get(2), a.get(3), a.get(4), a.get(5), a.get(6)) else { return Err("margincheck2 <manifest> <futures.tsv> <margin-per-trade.tsv> <margin-per-trade-v2.tsv> <calibration.tsv>".to_owned()) };
    let m = Manifest::load(Path::new(mp))?;
    let fut = Futures::load(Path::new(fp))?;
    let read = |p: &str| -> Result<(Vec<String>, Vec<Vec<String>>), String> {
        let t = std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?;
        let mut it = t.lines();
        let h: Vec<String> = it.next().unwrap_or("").split('\t').map(str::to_owned).collect();
        Ok((h, it.map(|l| l.split('\t').map(str::to_owned).collect()).collect()))
    };
    let (h1, r1) = read(t1)?;
    let (h2, r2) = read(t2)?;
    let c = |h: &[String], n: &str| h.iter().position(|x| x == n).ok_or(format!("column {n}"));
    let (a_pick, a_no, a_day, a_t, a_und, a_exp, a_k, a_r, a_lot, a_s, a_file) = (c(&h1, "pick")?, c(&h1, "trade_no")?, c(&h1, "day")?, c(&h1, "decision_T")?, c(&h1, "underlying")?, c(&h1, "expiry")?, c(&h1, "strike")?, c(&h1, "right")?, c(&h1, "lot")?, c(&h1, "spot_S_at_T")?, c(&h1, "file_used")?);
    let (b_pick, b_no, b_m, b_fit, b_slot) = (c(&h2, "pick")?, c(&h2, "trade_no")?, c(&h2, "margin_lot_v2")?, c(&h2, "lots_fit_v2")?, c(&h2, "slot_rs")?);
    let want: std::collections::HashMap<(String, String), (Option<i64>, i64, Option<i64>)> = r2.iter().map(|f| {
        let g = |i: usize| f.get(i).cloned().unwrap_or_default();
        ((g(b_pick), g(b_no)), (num_paisa(&g(b_m)), g(b_fit).parse::<i64>().unwrap_or(-1), num_paisa(&g(b_slot))))
    }).collect();
    let mut days: std::collections::BTreeMap<i64, DayM> = std::collections::BTreeMap::new();
    let (mut n, mut eq) = (0_u64, 0_u64);
    let mut bad: Vec<String> = Vec::new();
    for f in &r1 {
        let g = |i: usize| f.get(i).cloned().unwrap_or_default();
        n += 1;
        let day = crate::cal::parse_day(&g(a_day)).ok_or("day")?;
        if let std::collections::btree_map::Entry::Vacant(v) = days.entry(day) {
            v.insert(load_day(&m, day, true)?);
        }
        let Some(dm) = days.get(&day) else { continue };
        let hms: Vec<i64> = g(a_t).split(':').filter_map(|x| x.parse().ok()).collect();
        let t_us = crate::cal::at_sod(day, hms.first().copied().unwrap_or(0) * 3_600 + hms.get(1).copied().unwrap_or(0) * 60 + hms.get(2).copied().unwrap_or(0));
        let j = dm.snaps.iter().rposition(|x| x.0 < t_us);
        let und = g(a_und);
        let expiry: u32 = g(a_exp).parse().unwrap_or(0);
        let strike = num_paisa(&g(a_k)).unwrap_or(0);
        let right: u8 = if g(a_r) == "PE" { 1 } else { 0 };
        let lot: u64 = g(a_lot).parse().unwrap_or(0);
        let s = num_paisa(&g(a_s)).unwrap_or(0);
        let got = j.and_then(|j| {
            let (_, label, snap) = dm.snaps.get(j)?;
            let mut pr = 0;
            let r = snap.row(snap.find(&und, expiry, right, strike, &mut pr)?)?;
            let base = if r.idx { fut.base(label, &und, r.expiry)? } else { s };
            let p = short_margin_base(r, lot, s, base, crate::cal::ymd_of(day)).ok()?;
            Some((label.clone(), p.exact))
        });
        let key = (g(a_pick), g(a_no));
        let w = want.get(&key).copied();
        let ok = match (&got, w) {
            (Some((label, mlot)), Some((Some(wm), wfit, Some(slot)))) => *label == g(a_file) && *mlot == wm && common_fill::margin::lots_fit_v2(slot, *mlot) == wfit,
            _ => false,
        };
        if ok {
            eq += 1;
        } else if bad.len() < 20 {
            bad.push(format!("{} #{}: engine {:?} vs calibrator {:?}", key.0, key.1, got, w));
        }
    }
    // the 15 Dhan screens
    let (hc, rc) = read(cp)?;
    let (c_con, c_q, c_base, c_dhan) = (c(&hc, "contract")?, c(&hc, "qty")?, c(&hc, "elm_base_value")?, c(&hc, "dhan_margin")?);
    let d16 = crate::cal::parse_day("2026-11-16").ok_or("day")?;
    let dm16 = load_day(&m, d16, true)?;
    let (mut sn, mut se) = (0_u64, 0_u64);
    let mut sbad: Vec<String> = Vec::new();
    for f in &rc {
        let g = |i: usize| f.get(i).cloned().unwrap_or_default();
        sn += 1;
        let parts: Vec<String> = g(c_con).split_whitespace().map(str::to_owned).collect();
        let (und, exp, k, rt) = (parts.first().cloned().unwrap_or_default(), parts.get(1).and_then(|x| x.parse::<u32>().ok()).unwrap_or(0), num_paisa(parts.get(2).map_or("", String::as_str)).unwrap_or(0), parts.get(3).cloned().unwrap_or_default());
        let q: u64 = g(c_q).parse().unwrap_or(0);
        let got = dm16.snaps.first().and_then(|(_, label, snap)| {
            let mut pr = 0;
            let r = snap.row(snap.find(&und, exp, u8::from(rt == "PE"), k, &mut pr)?)?;
            // the index base from the futures table (checked against the screen sheet's base); the stock base = NSE's close (the sheet's)
            let sheet = num_paisa(&g(c_base))?;
            let base = if r.idx { fut.base(label, &und, r.expiry)? } else { sheet };
            if base != sheet {
                return None;
            }
            let p = short_margin_base(r, q, base, base, crate::cal::ymd_of(d16)).ok()?;
            Some(p.exact + common_fill::margin::ORDER_ADDON_PAISA)
        });
        if got.is_some() && got == num_paisa(&g(c_dhan)) {
            se += 1;
        } else {
            sbad.push(format!("screen {}: engine {:?} vs Dhan {}", g(0), got, g(c_dhan)));
        }
    }
    let mut out = format!("MARGINCHECK2 (margin rule v2, addendum 9): {n} margin-truth trades re-priced: {eq} EQUAL to the calibrator's margin_lot_v2 AND lots_fit_v2 (file used equal too), {} different; Dhan screens: {se} of {sn} EQUAL to the paisa (SPAN + option value + exposure + Rs 20); futures table {} sha256 {}\n", n - eq, fut.path, fut.sha);
    for b in bad.iter().chain(sbad.iter()).take(25) {
        out.push_str(b);
        out.push('\n');
    }
    Ok(out)
}

/// `margin-one --manifest M --day YYYY-MM-DD --und U --expiry YYYYMMDD --strike K --right CE|PE --lot L [--spot S] [--time HH:MM:SS]`:
/// one short position's margin by THE rule (the GRASIM calibration uses it; spot defaults to the table's underlying price).
///
/// # Errors
/// A missing argument or table.
pub fn cmd_margin_one(a: &[String]) -> Result<String, String> {
    let arg = |k: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str);
    let m = Manifest::load(Path::new(arg("--manifest").ok_or("--manifest")?))?;
    let day = crate::cal::parse_day(arg("--day").ok_or("--day")?).ok_or("--day")?;
    let und = arg("--und").ok_or("--und")?;
    let expiry: u32 = arg("--expiry").ok_or("--expiry")?.parse().map_err(|_| "--expiry")?;
    let strike = num_paisa(arg("--strike").ok_or("--strike")?).ok_or("--strike")?;
    let right: u8 = if matches!(arg("--right"), Some("PE" | "P")) { 1 } else { 0 };
    let lot: u64 = arg("--lot").ok_or("--lot")?.parse().map_err(|_| "--lot")?;
    let sod = arg("--time").map_or(9 * 3_600 + 15 * 60, |t| t.split(':').filter_map(|x| x.parse::<i64>().ok()).zip([3_600_i64, 60, 1]).map(|(v, k)| v * k).sum());
    let dm = load_day(&m, day, true)?;
    let t_us = crate::cal::at_sod(day, sod);
    let j = dm.snaps.iter().rposition(|x| x.0 < t_us).ok_or("no snapshot in force")?;
    let (_, label, snap) = dm.snaps.get(j).ok_or("snapshot")?;
    let mut pr = 0;
    let id = snap.find(und, expiry, right, strike, &mut pr).ok_or("contract not in the table")?;
    let r = snap.row(id).ok_or("row")?;
    let s = match arg("--spot") {
        Some(x) => num_paisa(x).ok_or("--spot")?,
        None => r.und_price,
    };
    let p = short_margin(r, lot, s, crate::cal::ymd_of(day)).map_err(|e| e.to_string())?;
    let rs = crate::cal::fmt_rs;
    Ok(format!("SHORT {und} {expiry} {} {} x {lot} | table {label} (created {}, AEL {}) | spot {} (table underlying price {})\n  SPAN = {} x {lot} = Rs {}\n  premium term = file price {} x {lot} = Rs {}\n  ELM = {}.{:02}% x {} x {lot} = Rs {} (rounded once, half away from zero)\n  EXACT (SPAN + premium + ELM) = Rs {}; NSE-only (SPAN + ELM) = Rs {}", rs(strike), if right == 0 { "CE" } else { "PE" }, snap.created, snap.ael, rs(s), rs(r.und_price), rs(r.span_unit), rs(p.span), rs(r.price), rs(p.premium), p.rate_h / 100, p.rate_h % 100, rs(s), rs(p.elm), rs(p.exact), rs(p.nse_only)))
}
