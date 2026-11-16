//! The seven LIVE dials host v1 named `live_not_wired` on every SN4 row (X-NIFTY, X-VIX, D-EXP, D-MEND, G-52H, G-52L, G-DRNG),
//! wired exactly as their rows in the dials file say (plan/SNIPER-DIALS-v2.plan.tsv: reading, values, units, mirror, range_source).
//!
//! **K11.** Every reading below uses only data that had arrived by the decision instant S (the decision candle's END):
//!
//! | Dial | Reading at S | Source (our own store first) |
//! |---|---|---|
//! | X-NIFTY | NIFTY 50's last COMPLETED 1-minute candle of the decision session with END <= S: its close against NIFTY 50's close of the previous session of the table, in bp | Zerodha `NSE/INDEX/NIFTY` 1min and 1day (store `shadow/fib-frozen-2`) |
//! | X-VIX | INDIA VIX's last COMPLETED 1-minute candle of the decision session with END <= S: its close, x100 (the store's paisa = the level x 100) | Zerodha `NSE/INDEX/INDIAVIX` 1min |
//! | D-EXP | the decision session is a monthly ADANIENT stock F&O expiry that was KNOWN before the session (a contract of that expiry traded on an earlier day): ON / NOT | the store's GDFL FNO chain census (`scout/adanient-opt-expiries.tsv`) |
//! | D-MEND | the decision session is the last session of its calendar month as known on the prior evening (F4): ON / NOT | the dated session table (`feat_io::defaults::DATES`) and the late-closure rows [`LATE_CLOSURES`] |
//! | G-52H / G-52L | the decision candle's close against the highest official high / lowest official low of the [`WEEKS_52`] sessions of the dated table BEFORE the decision session (every kind, as the signed vector SN-X09 B5 counts them), each through the CA factors onto the decision session's basis (X2, as the period levels; F2) | official CM bhav (`feat_io::defaults::BHAV`), the CA book |
//! | G-DRNG | the decision session's range so far (every decision-stream candle of the session with END <= S) against the daily ATR 14 known at S (common_feat's `Atr` folded over the official rows of the table's sessions BEFORE the decision session from the CONT start, reset at every factor event, K13 X3; ready by the module's own convergence rule, F5) | the decision stream; official CM bhav |
//!
//! A reading that cannot be made from such data is `None` (unknown) and never true, counted on the run's lines. Nothing here is
//! read when the run passes `--context-dials off` (binary k's behaviour, byte for byte).

use std::collections::BTreeMap;
use std::path::Path;

use brutex_core::vendor::Vendor;
use common_feat::ca::through;
use common_feat::cal::{civil, days_from_civil, ist_day, weekday};
use common_feat::fold::Candle;
use common_feat::ind::Atr;
use common_feat::series::{SeriesCfg, need_recursive};
use feat_io::host::{Ctx, Stream};

use crate::cands::Family;

/// The seven dials this module wires, in the dials file's order.
pub const IDS: [&str; 7] = ["X-NIFTY", "X-VIX", "D-EXP", "D-MEND", "G-52H", "G-52L", "G-DRNG"];

/// The build that wires them (printed on SN1 and the run lines in the ON mode only). PORTER (secfd): this tree is
/// sniper-host-v1-secf-a + the dials diff of sniper-host-v1-dials-a, so the string names both. HOST v3 (2026-11-14): this tree is
/// sniper-host-v2-a + the deltas of sniper-host-v1-xcdb-a / sniper-host-v1-secfdb-a (CHANGES-v3), so no v3 binary calls itself
/// secfdb-a; identity text only (ON mode).
pub const BUILD: &str = "sniper-host-v3-a (sniper-host-v2-a + the deltas of sniper-host-v1-xcdb-a / sniper-host-v1-secfdb-a: the dials of sniper-host-v1-dials-a: X-NIFTY, X-VIX, D-EXP, D-MEND, G-52H, G-52L, G-DRNG; + check-build-xc B1: XC's clock never at or after the 15:08:00 cut)";

/// The point-in-time expiry calendar: the store's GDFL FNO chain census of ADANIENT (one row per monthly expiry with the first
/// instant any contract of it traded).
pub const EXPIRIES: &str = "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/sniper-req76-20261113/scout/adanient-opt-expiries.tsv";

/// The window of G-52H / G-52L: sessions of the dated table before the decision session (REQ72-PLAN LV-52W; F2; SN-X09 B5).
pub const WEEKS_52: usize = 252;

/// F4's late closures: (closed day, day the closure was announced) as `YYYY-MM-DD` pairs. A closure announced on or after a
/// session t was still a trading day on t's prior evening. No dated source checked for 2018-09..2026-10 names a closure announced
/// after its prior session that falls after a month's last realised session, so the list is empty (printed on the run lines);
/// test SN-X34 proves the rule on a synthetic row.
pub const LATE_CLOSURES: &[(&str, &str)] = &[];

/// What [`attach`] returns: the run lines and the SN1 rows (field, value).
pub type Attached = (Vec<String>, Vec<(String, String)>);

/// The seven readings of one decision candle (`None`: unknown, never true).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CtxRead {
    /// X-NIFTY: 10^4 (close at S - previous close) / previous close, as (num, den).
    pub nifty_bp: Option<(i128, i128)>,
    /// X-VIX: INDIA VIX at S, x100.
    pub vix: Option<i64>,
    /// D-EXP.
    pub exp: Option<bool>,
    /// D-MEND.
    pub mend: Option<bool>,
    /// The 52-week high (paisa, on the decision session's basis).
    pub h52: Option<i64>,
    /// The 52-week low.
    pub l52: Option<i64>,
    /// G-DRNG: (range so far x 100 x `common_feat::S`, daily ATR 14 scaled by `common_feat::S`), so `num / den` is the ratio x100.
    pub drng: Option<(i128, i128)>,
}

/// The close of the last bar of `bars` (sorted by END) with END <= `s` that belongs to IST day `day` (K11: the last COMPLETED
/// candle at S of the same session; a bar still forming at S is never read, and the previous session's bars never stand in).
#[must_use]
pub fn last_close_at(bars: &[Candle], day: i32, s: i64) -> Option<i64> {
    // MUTANT m_k11 reads the candle still forming at S (one minute ahead).
    let n = bars.partition_point(|b| b.end <= if cfg!(feature = "m_k11") { s + 60 } else { s });
    let b = bars.get(n.checked_sub(1)?)?;
    (ist_day(b.start) == day).then_some(b.close)
}

/// The daily bars' close by IST day.
#[must_use]
pub fn daily_closes(bars: &[Candle]) -> BTreeMap<i32, i64> {
    bars.iter().map(|b| (ist_day(b.start), b.close)).collect()
}

/// X-NIFTY: the % change in bp of `close` against `prev`.
#[must_use]
pub fn change_bp(close: Option<i64>, prev: Option<i64>) -> Option<(i128, i128)> {
    close.zip(prev).and_then(|(c, p)| feat_io::host::bp(c, p)).map(|r| (r.num, r.den))
}

/// Parses the chain census: (expiry day, IST day the first contract of that expiry traded).
///
/// # Errors
/// A row without both dates.
pub fn parse_expiries(text: &str) -> Result<Vec<(i32, i32)>, String> {
    let mut out = Vec::new();
    for l in text.lines() {
        if l.starts_with("expiry\t") || l.starts_with('#') || l.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        let e = f.first().and_then(|x| common_feat::cal::parse_ymd(x));
        let k = f.get(9).and_then(|x| x.get(..10)).and_then(common_feat::cal::parse_ymd);
        match (e, k) {
            (Some(e), Some(k)) => out.push((e, k)),
            _ => return Err(format!("expiry row without its dates: {l}")),
        }
    }
    out.sort_unstable();
    Ok(out)
}

/// D-EXP at session day `t`: ON when `t` is an expiry KNOWN before `t` (a contract of it traded on an earlier day, so the expiry
/// stood on the prior evening); unknown outside the calendar's span (before its first known day, after its last expiry).
#[must_use]
pub fn expiry_flag(t: i32, ex: &[(i32, i32)]) -> Option<bool> {
    let last = ex.iter().map(|e| e.0).max()?;
    let first = ex.iter().map(|e| e.1).min()?;
    if t > last || t <= first {
        return None;
    }
    // MUTANT m_k11 counts an expiry first listed ON the day as known on the prior evening.
    Some(ex.iter().any(|&(e, k)| e == t && if cfg!(feature = "m_k11") { k <= t } else { k < t }))
}

fn month_key(d: i32) -> i32 {
    let (y, m, _) = civil(d);
    y * 12 + i32::try_from(m).unwrap_or(1)
}

fn month_last_day(d: i32) -> i32 {
    let (y, m, _) = civil(d);
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    days_from_civil(ny, nm, 1) - 1
}

/// D-MEND for every session of `days` (ascending), F4: ON when no later session of the same calendar month is in the table and no
/// closure of a later day of that month was announced on or after the session (`late`: (closed day, announced day); such a day
/// was still a session on the prior evening). In the table's last month the bit is known only when every day left in the month
/// after the table's end is a Saturday or Sunday; otherwise unknown.
#[must_use]
pub fn month_end_flags(days: &[i32], late: &[(i32, i32)]) -> Vec<Option<bool>> {
    let mut out = Vec::with_capacity(days.len());
    for (i, &t) in days.iter().enumerate() {
        let next = days.get(i + 1).copied();
        if next.is_some_and(|n| month_key(n) == month_key(t)) {
            out.push(Some(false));
            continue;
        }
        if next.is_none() && ((t + 1)..=month_last_day(t)).any(|d| (1..=5).contains(&weekday(d))) {
            out.push(None);
            continue;
        }
        // MUTANT m_k11 reads a closure announced on the day itself as known on the prior evening.
        let late_hit = late.iter().any(|&(c, a)| month_key(c) == month_key(t) && c > t && if cfg!(feature = "m_k11") { a > t } else { a >= t });
        out.push(Some(!late_hit));
    }
    out
}

/// G-52H / G-52L of session `i`: the highest official high and lowest official low over the [`WEEKS_52`] sessions of the dated
/// session table before session `i` (every kind: the signed vector SN-X09 B5 counts them so, "G-52H on 2019-09-18 is unknown, on
/// 2019-09-19 ready", the window then 2018-09-06..2019-09-18), each through the CA factors onto session `i`'s basis (X2, as the
/// period levels). Unknown when the table holds fewer sessions before `i`, when a window session has no official row, or when a
/// factor is unproven (K13: the demerger keeps 2018-09-03..05 out of every window).
#[must_use]
pub fn hl52(ctx: &Ctx, i: usize) -> Option<(i64, i64)> {
    let t = ctx.sessions.get(i)?.day;
    let mut n = 0_usize;
    let (mut hi, mut lo) = (i64::MIN, i64::MAX);
    // MUTANT m_k11 lets the window hold the decision session itself (its official row arrives after the close).
    let top = if cfg!(feature = "m_k11") { i + 1 } else { i };
    for j in (0..top).rev() {
        if n == WEEKS_52 {
            break;
        }
        let s = ctx.sessions.get(j)?;
        let b = ctx.bhav.get(&s.day)?;
        let f = ctx.ca.factor(s.day, t)?;
        hi = hi.max(through(b.high, f));
        lo = lo.min(through(b.low, f));
        n += 1;
    }
    (n == WEEKS_52).then_some((hi, lo))
}

/// The daily ATR 14 known at every session's decision instants (indexed like the session table): common_feat's `Atr` folded over
/// the official rows of the sessions BEFORE the session (every kind of the dated table), from the CONT start, reset at every factor event (K13 X3, as
/// every CONT reading) and at a session without an official row (those days are the second value, named on the run lines); ready after the module's
/// own `need` (F5: `need_recursive` with `SeriesCfg::sniper`'s ATR period and seed weight).
#[must_use]
pub fn daily_atr(ctx: &Ctx) -> (Vec<Option<i128>>, Vec<i32>) {
    let cfg = SeriesCfg::sniper(86_400);
    let need = need_recursive(u64::from(cfg.atr), 1, i128::from(cfg.atr.max(1)), cfg.seed_weight_max_ppm);
    let mut atr = Atr::new(cfg.atr);
    let mut since = 0_u64;
    let mut holes = Vec::new();
    let mut out = Vec::with_capacity(ctx.sessions.len());
    for s in &ctx.sessions {
        let live = s.day >= ctx.cont_start;
        if live && ctx.ca.cont_reset(s.day) {
            atr.reset();
            since = 0;
        }
        if !cfg!(feature = "m_k11") {
            out.push(if since >= need { atr.value() } else { None });
        }
        if live {
            if let Some(b) = ctx.bhav.get(&s.day) {
                let _ = atr.fold(&Candle { start: s.open(), end: s.end(), open: b.open, high: b.high, low: b.low, close: b.close, volume: b.volume });
                since += 1;
            } else {
                atr.reset();
                since = 0;
                holes.push(s.day);
            }
        }
        if cfg!(feature = "m_k11") {
            // MUTANT m_k11: the reading at t after folding t's own official row.
            out.push(if since >= need { atr.value() } else { None });
        }
    }
    (out, holes)
}

/// The session's (high, low) so far at every candle of `stream`: over every candle of the same session that ended at or before
/// this one's END (the candle itself included), in stream order.
#[must_use]
pub fn day_range_so_far(stream: &Stream) -> Vec<(usize, i64, i64, i64)> {
    let mut out = Vec::with_capacity(stream.candles.len());
    let mut cur: Option<(usize, i64, i64)> = None;
    for &(i, c) in &stream.candles {
        let (hi, lo) = match cur {
            Some((s, h, l)) if s == i => (h.max(c.high), l.min(c.low)),
            _ => (c.high, c.low),
        };
        cur = Some((i, hi, lo));
        out.push((i, c.end, hi, lo));
        // MUTANT m_k11: the previous candle's range takes this (later) candle's extremes.
        if cfg!(feature = "m_k11") && out.len() > 1 && let Some(prev) = out.len().checked_sub(2).and_then(|k| out.get_mut(k)) && prev.0 == i {
            prev.2 = hi;
            prev.3 = lo;
        }
    }
    out
}

/// G-DRNG's ratio: (range x 100 x S, ATR) with `atr` scaled by `common_feat::S`.
#[must_use]
pub fn drng(hi: i64, lo: i64, atr: Option<i128>) -> Option<(i128, i128)> {
    let a = atr.filter(|&a| a > 0)?;
    Some(((i128::from(hi) - i128::from(lo)) * 100 * common_feat::S, a))
}

/// The reading of dial `id` (X-NIFTY, X-VIX, G-52H, G-52L, G-DRNG) for one side, as the (num, den) the dials file's levels are
/// compared with. The mirrors are the rows' own: X-NIFTY, X-VIX and G-DRNG "same"; G-52H's short side reads the 52-week LOW with
/// the LE mirror (`-(close - low) / low >= x`, i.e. within |x| above the low); G-52L's short side reads the 52-week HIGH
/// (`-(close - high) / high >= x`, i.e. at least x below the high).
#[must_use]
pub fn reading(id: &str, r: &CtxRead, close: i64, long: bool) -> Option<(i128, i128)> {
    let bp = |b: Option<i64>| b.and_then(|b| feat_io::host::bp(close, b)).map(|q| (q.num, q.den));
    let neg = |v: Option<(i128, i128)>| v.map(|(n, d)| (-n, d));
    match id {
        "X-NIFTY" => r.nifty_bp,
        "X-VIX" => r.vix.map(|v| (i128::from(v), 1)),
        "G-52H" => {
            if long {
                bp(r.h52)
            } else {
                neg(bp(r.l52))
            }
        }
        "G-52L" => {
            if long {
                bp(r.l52)
            } else {
                neg(bp(r.h52))
            }
        }
        "G-DRNG" => r.drng,
        _ => None,
    }
}

fn read_index(store: &Path, symbol: &str, tf: &str) -> Result<(Vec<Candle>, Vec<String>), String> {
    let t = feat_io::zstore::tf_of(tf).ok_or_else(|| format!("timeframe {tf}"))?;
    let (mut bars, notes) = feat_io::zstore::read_range(store, Vendor::Zerodha, "INDEX", symbol, t, (2018, 1), (2026, 11));
    bars.sort_by_key(|b| b.start);
    Ok((bars, notes))
}

fn known_line(name: &str, n: usize, known: usize, first: Option<i32>) -> String {
    format!("{name} known on {known} of {n} candidates{}", first.map_or_else(String::new, |d| format!(" (first {})", common_feat::cal::ymd(d))))
}

/// Reads the sources and attaches every candidate's readings to `fam.ctxr` (indexed like `fam.cands`). `stream` is the decision
/// stream `fam` was built from. Returns the run lines (sources, counts) and the SN1 rows (field, value).
///
/// # Errors
/// An unreadable expiry calendar or an empty index stream.
#[allow(clippy::too_many_lines, reason = "the seven readings over one family in one pass")]
pub fn attach(fam: &mut Family, ctx: &Ctx, stream: &Stream, store: &Path) -> Result<Attached, String> {
    let (n1, n1_notes) = read_index(store, "NIFTY", "1min")?;
    let (nd, nd_notes) = read_index(store, "NIFTY", "1day")?;
    let (v1, v1_notes) = read_index(store, "INDIAVIX", "1min")?;
    if n1.is_empty() || nd.is_empty() || v1.is_empty() {
        return Err(format!("context dials: an empty index stream (NIFTY 1min {}, NIFTY 1day {}, INDIAVIX 1min {})", n1.len(), nd.len(), v1.len()));
    }
    let n_close = daily_closes(&nd);
    let (ex_text, ex_sha) = crate::rows::read_with_sha(EXPIRIES)?;
    let ex = parse_expiries(&ex_text)?;
    let days: Vec<i32> = ctx.sessions.iter().map(|s| s.day).collect();
    let late: Vec<(i32, i32)> = LATE_CLOSURES
        .iter()
        .map(|(c, a)| common_feat::cal::parse_ymd(c).zip(common_feat::cal::parse_ymd(a)).ok_or_else(|| format!("late closure row {c} {a}")))
        .collect::<Result<_, _>>()?;
    let mend = month_end_flags(&days, &late);
    let hl: Vec<Option<(i64, i64)>> = (0..ctx.sessions.len()).map(|i| hl52(ctx, i)).collect();
    let (atr, atr_holes) = daily_atr(ctx);
    let rng = day_range_so_far(stream);
    let mut out = Vec::with_capacity(fam.cands.len());
    let mut p = 0_usize;
    for c in &fam.cands {
        let i = c.sess as usize;
        while rng.get(p).is_some_and(|&(s, e, _, _)| (s, e) != (i, c.end)) {
            p += 1;
        }
        let (hi, lo) = rng.get(p).map_or((None, None), |&(_, _, h, l)| (Some(h), Some(l)));
        let prev = i.checked_sub(1).and_then(|j| ctx.sessions.get(j)).and_then(|s| n_close.get(&s.day).copied());
        let hlv = hl.get(i).copied().flatten();
        out.push(CtxRead {
            nifty_bp: change_bp(last_close_at(&n1, c.day, c.end), prev),
            vix: last_close_at(&v1, c.day, c.end),
            exp: expiry_flag(c.day, &ex),
            mend: mend.get(i).copied().flatten(),
            h52: hlv.map(|x| x.0),
            l52: hlv.map(|x| x.1),
            drng: hi.zip(lo).and_then(|(h, l)| drng(h, l, atr.get(i).copied().flatten())),
        });
    }
    if out.len() != fam.cands.len() || (rng.get(p).is_none() && !fam.cands.is_empty()) {
        return Err("context dials: a candidate without its stream candle (the day range so far)".to_string());
    }
    let n = out.len();
    let cnt = |f: &dyn Fn(&CtxRead) -> bool| -> (usize, Option<i32>) {
        let known = out.iter().filter(|r| f(r)).count();
        let first = fam.cands.iter().zip(&out).find(|(_, r)| f(r)).map(|(c, _)| c.day);
        (known, first)
    };
    let (kn, fn_) = cnt(&|r| r.nifty_bp.is_some());
    let (kv, fv) = cnt(&|r| r.vix.is_some());
    let (ke, fe) = cnt(&|r| r.exp.is_some());
    let (km, fm) = cnt(&|r| r.mend.is_some());
    let (kh, fh) = cnt(&|r| r.h52.is_some());
    let (kd, fd) = cnt(&|r| r.drng.is_some());
    let on_e = out.iter().filter(|r| r.exp == Some(true)).count();
    let on_m = out.iter().filter(|r| r.mend == Some(true)).count();
    let vmin = out.iter().filter_map(|r| r.vix).min().unwrap_or(0);
    let vmax = out.iter().filter_map(|r| r.vix).max().unwrap_or(0);
    let notes: Vec<String> = n1_notes.iter().chain(&nd_notes).chain(&v1_notes).cloned().collect();
    let lines = vec![
        format!("context dials ON ({BUILD}): {} wired; --context-dials off reads none of this (sniper-host-v1-xcdb-a / sniper-host-v1-secfdb-a off, plus common_feat repair 2 of sniper-host-v2-a: CHANGES-v3)", IDS.join(", ")),
        format!(
            "context dial sources: zerodha NSE/INDEX/NIFTY 1min {} bars, 1day {} bars; zerodha NSE/INDEX/INDIAVIX 1min {} bars (VIX x100 at S on the candidates: {} .. {}); store notes {:?}; expiries {} ({} monthly expiries, sha256 {}); session table {} sessions; late-closure rows {} (F4); daily ATR 14 holes (a session without an official row: reset; the official CM file ends 2026-09-30, as for the pivots, C-GAP, P-CHG and G-PDH / G-PDL of host v1) {} {:?}",
            n1.len(),
            nd.len(),
            v1.len(),
            vmin,
            vmax,
            notes,
            EXPIRIES,
            ex.len(),
            ex_sha,
            days.len(),
            late.len(),
            atr_holes.len(),
            atr_holes.iter().map(|d| common_feat::cal::ymd(*d)).collect::<Vec<_>>()
        ),
        format!(
            "context dial readings (K11: only data arrived by S; unknown never true, counted): {}; {}; {} (ON on {on_e}); {} (ON on {on_m}); {} (G-52H and G-52L, window {WEEKS_52} table sessions); {}",
            known_line("X-NIFTY", n, kn, fn_),
            known_line("X-VIX", n, kv, fv),
            known_line("D-EXP", n, ke, fe),
            known_line("D-MEND", n, km, fm),
            known_line("G-52H/G-52L", n, kh, fh),
            known_line("G-DRNG", n, kd, fd)
        ),
    ];
    let sn1 = vec![
        ("context_dials".to_string(), format!("ON: {BUILD}")),
        ("context_dial_sources".to_string(), format!("zerodha NSE/INDEX/NIFTY 1min+1day, NSE/INDEX/INDIAVIX 1min (store {}); expiries sha256 {ex_sha}; late closures {}", store.display(), late.len())),
    ];
    fam.ctxr = out;
    Ok((lines, sn1))
}

/// PORTER (secfd): the SECONDS driver's form of [`attach`]. The seconds family is built one session day at a time and every candidate
/// is compressed at once (sec.rs, design change (d)), so the readings are made per candidate from these prepared sources: the same
/// sources, the same functions ([`last_close_at`], [`change_bp`], [`expiry_flag`], [`month_end_flags`], [`hl52`], [`daily_atr`],
/// [`drng`]) and the same K11 rules as [`attach`]; G-DRNG's range so far is the day's decision-stream candles with END <= S (the
/// decision candle included), kept by the day loop exactly as [`day_range_so_far`] keeps it. SEC-EQ with the dials ON holds the
/// compact rows to [`attach`] + `cands::outer_values` on a whole-range fold, bit for bit.
#[derive(Clone, Debug, Default)]
pub struct Prep {
    n1: Vec<Candle>,
    v1: Vec<Candle>,
    nd_len: usize,
    notes: Vec<String>,
    ex_n: usize,
    ex_sha: String,
    late_n: usize,
    /// Per session of the table: NIFTY 50's daily close of the PREVIOUS session.
    prev_close: Vec<Option<i64>>,
    /// Per session: (session day, D-EXP at that day).
    exp: Vec<(i32, Option<bool>)>,
    mend: Vec<Option<bool>>,
    hl: Vec<Option<(i64, i64)>>,
    atr: Vec<Option<i128>>,
    atr_holes: Vec<i32>,
    ex: Vec<(i32, i32)>,
    store: String,
}

/// The counts of the readings made (printed as [`attach`] prints them).
#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    n: usize,
    known: [usize; 6],
    first: [Option<i32>; 6],
    on_e: usize,
    on_m: usize,
    vmin: Option<i64>,
    vmax: Option<i64>,
}

impl Counts {
    /// Counts one candidate's readings (`day`: its IST day).
    pub fn add(&mut self, day: i32, r: &CtxRead) {
        self.n += 1;
        let k = [r.nifty_bp.is_some(), r.vix.is_some(), r.exp.is_some(), r.mend.is_some(), r.h52.is_some(), r.drng.is_some()];
        for (j, &b) in k.iter().enumerate() {
            if b {
                if let Some(x) = self.known.get_mut(j) {
                    *x += 1;
                }
                if let Some(f) = self.first.get_mut(j) {
                    f.get_or_insert(day);
                }
            }
        }
        self.on_e += usize::from(r.exp == Some(true));
        self.on_m += usize::from(r.mend == Some(true));
        if let Some(v) = r.vix {
            self.vmin = Some(self.vmin.map_or(v, |m| m.min(v)));
            self.vmax = Some(self.vmax.map_or(v, |m| m.max(v)));
        }
    }
}

/// Reads the sources once for a seconds family ([`attach`]'s sources).
///
/// # Errors
/// An unreadable expiry calendar or an empty index stream.
pub fn prepare(ctx: &Ctx, store: &Path) -> Result<Prep, String> {
    let (n1, n1_notes) = read_index(store, "NIFTY", "1min")?;
    let (nd, nd_notes) = read_index(store, "NIFTY", "1day")?;
    let (v1, v1_notes) = read_index(store, "INDIAVIX", "1min")?;
    if n1.is_empty() || nd.is_empty() || v1.is_empty() {
        return Err(format!("context dials: an empty index stream (NIFTY 1min {}, NIFTY 1day {}, INDIAVIX 1min {})", n1.len(), nd.len(), v1.len()));
    }
    let n_close = daily_closes(&nd);
    let (ex_text, ex_sha) = crate::rows::read_with_sha(EXPIRIES)?;
    let ex = parse_expiries(&ex_text)?;
    let days: Vec<i32> = ctx.sessions.iter().map(|s| s.day).collect();
    let late: Vec<(i32, i32)> = LATE_CLOSURES
        .iter()
        .map(|(c, a)| common_feat::cal::parse_ymd(c).zip(common_feat::cal::parse_ymd(a)).ok_or_else(|| format!("late closure row {c} {a}")))
        .collect::<Result<_, _>>()?;
    let mend = month_end_flags(&days, &late);
    let hl: Vec<Option<(i64, i64)>> = (0..ctx.sessions.len()).map(|i| hl52(ctx, i)).collect();
    let (atr, atr_holes) = daily_atr(ctx);
    let prev_close = (0..ctx.sessions.len()).map(|i| i.checked_sub(1).and_then(|j| ctx.sessions.get(j)).and_then(|s| n_close.get(&s.day).copied())).collect();
    let exp = days.iter().map(|&d| (d, expiry_flag(d, &ex))).collect();
    let notes: Vec<String> = n1_notes.iter().chain(&nd_notes).chain(&v1_notes).cloned().collect();
    Ok(Prep { nd_len: nd.len(), n1, v1, notes, ex_n: ex.len(), ex_sha, late_n: late.len(), prev_close, exp, mend, hl, atr, atr_holes, ex, store: store.display().to_string() })
}

/// The readings of one candidate of session `i` (IST day `day`, END `end`), with the day's range so far `(hi, lo)` at it.
#[must_use]
pub fn read_one(p: &Prep, i: usize, day: i32, end: i64, hi: i64, lo: i64) -> CtxRead {
    let exp = match p.exp.get(i) {
        Some(&(d, e)) if d == day => e,
        _ => expiry_flag(day, &p.ex),
    };
    let hlv = p.hl.get(i).copied().flatten();
    CtxRead {
        nifty_bp: change_bp(last_close_at(&p.n1, day, end), p.prev_close.get(i).copied().flatten()),
        vix: last_close_at(&p.v1, day, end),
        exp,
        mend: p.mend.get(i).copied().flatten(),
        h52: hlv.map(|x| x.0),
        l52: hlv.map(|x| x.1),
        drng: drng(hi, lo, p.atr.get(i).copied().flatten()),
    }
}

/// [`attach`]'s run lines and SN1 rows for the readings counted in `c` (`sessions`: the table's session count).
#[must_use]
pub fn lines(p: &Prep, c: &Counts, sessions: usize) -> Attached {
    let kl = |j: usize, name: &str| known_line(name, c.n, c.known.get(j).copied().unwrap_or(0), c.first.get(j).copied().flatten());
    let lines = vec![
        format!("context dials ON ({BUILD}): {} wired; --context-dials off reads none of this (sniper-host-v1-xcdb-a / sniper-host-v1-secfdb-a off, plus common_feat repair 2 of sniper-host-v2-a: CHANGES-v3)", IDS.join(", ")),
        format!(
            "context dial sources: zerodha NSE/INDEX/NIFTY 1min {} bars, 1day {} bars; zerodha NSE/INDEX/INDIAVIX 1min {} bars (VIX x100 at S on the candidates: {} .. {}); store notes {:?}; expiries {} ({} monthly expiries, sha256 {}); session table {} sessions; late-closure rows {} (F4); daily ATR 14 holes (a session without an official row: reset; the official CM file ends 2026-09-30, as for the pivots, C-GAP, P-CHG and G-PDH / G-PDL of host v1) {} {:?}",
            p.n1.len(),
            p.nd_len,
            p.v1.len(),
            c.vmin.unwrap_or(0),
            c.vmax.unwrap_or(0),
            p.notes,
            EXPIRIES,
            p.ex_n,
            p.ex_sha,
            sessions,
            p.late_n,
            p.atr_holes.len(),
            p.atr_holes.iter().map(|d| common_feat::cal::ymd(*d)).collect::<Vec<_>>()
        ),
        format!(
            "context dial readings (K11: only data arrived by S; unknown never true, counted): {}; {}; {} (ON on {}); {} (ON on {}); {} (G-52H and G-52L, window {WEEKS_52} table sessions); {}",
            kl(0, "X-NIFTY"),
            kl(1, "X-VIX"),
            kl(2, "D-EXP"),
            c.on_e,
            kl(3, "D-MEND"),
            c.on_m,
            kl(4, "G-52H/G-52L"),
            kl(5, "G-DRNG")
        ),
    ];
    let sn1 = vec![
        ("context_dials".to_string(), format!("ON: {BUILD}")),
        ("context_dial_sources".to_string(), format!("zerodha NSE/INDEX/NIFTY 1min+1day, NSE/INDEX/INDIAVIX 1min (store {}); expiries sha256 {}; late closures {}", p.store, p.ex_sha, p.late_n)),
    ];
    (lines, sn1)
}

#[cfg(test)]
mod tests {
    //! K11 per dial, red first: each reading is computed at S, then every datum that would arrive AFTER S (later candles of the
    //! session, later sessions, the decision session's own official row, a later expiry, a closure announced on the day) is changed
    //! and the reading must not move; a datum that arrived BY S is changed and the reading must move (the test sees the input).
    use super::*;
    use common_feat::cal::at;

    fn d(y: i32, m: u32, dd: u32) -> i32 {
        days_from_civil(y, m, dd)
    }

    fn bar(day: i32, hh: i64, mm: i64, close: i64) -> Candle {
        let start = at(day, hh, mm, 0);
        Candle { start, end: start + 60, open: close, high: close, low: close, close, volume: 1 }
    }

    #[test]
    fn k11_x_nifty_and_x_vix_read_the_last_completed_minute_of_the_session_only() {
        let t = d(2024, 3, 5);
        let y = d(2024, 3, 4);
        let s = at(t, 9, 18, 0);
        // yesterday's last bar, today's 09:15, 09:16, 09:17 bars (ends 09:16, 09:17, 09:18) and a forming 09:18 bar (end 09:19).
        let mut bars = vec![bar(y, 15, 29, 100), bar(t, 9, 15, 101), bar(t, 9, 16, 102), bar(t, 9, 17, 103), bar(t, 9, 18, 104)];
        assert_eq!(last_close_at(&bars, t, s), Some(103), "the candle ending exactly at S is completed and read");
        // every bar after S changes: the reading must not move.
        if let Some(b) = bars.get_mut(4) {
            b.close = 999;
        }
        bars.push(bar(t, 9, 19, 777));
        assert_eq!(last_close_at(&bars, t, s), Some(103), "a bar ending after S is never read");
        // a bar that ended by S changes: the reading moves.
        if let Some(b) = bars.get_mut(3) {
            b.close = 555;
        }
        assert_eq!(last_close_at(&bars, t, s), Some(555));
        // before the session's first completed candle: unknown (yesterday's bar never stands in).
        assert_eq!(last_close_at(&bars, t, at(t, 9, 15, 30)), None);
        // X-NIFTY's change against the previous close (24,000.00 -> 24,240.00 = +100 bp).
        assert_eq!(change_bp(Some(2_424_000), Some(2_400_000)).map(|(n, dn)| n / dn), Some(100));
        assert_eq!(change_bp(Some(2_424_000), None), None, "no previous close: unknown");
        // the previous close is the previous SESSION's daily close: a daily bar of the decision day (published after the close)
        // is never the previous close (read by the session index below).
        let daily = daily_closes(&[bar(y, 0, 0, 2_400_000), bar(t, 0, 0, 2_500_000)]);
        assert_eq!(daily.get(&y).copied(), Some(2_400_000));
    }

    #[test]
    fn k11_d_exp_is_on_only_for_an_expiry_known_before_the_session() {
        let e1 = d(2024, 3, 28);
        let e2 = d(2024, 4, 25);
        let ex = vec![(e1, d(2024, 1, 2)), (e2, d(2024, 2, 1))];
        assert_eq!(expiry_flag(e1, &ex), Some(true));
        assert_eq!(expiry_flag(e1 - 1, &ex), Some(false));
        // an expiry first listed ON the day itself was not known on the prior evening: NOT.
        let late = vec![(e1, e1), (e2, d(2024, 2, 1))];
        assert_eq!(expiry_flag(e1, &late), Some(false));
        // adding a LATER expiry (data after S) never moves the bit.
        let mut more = ex.clone();
        more.push((d(2024, 5, 30), d(2024, 3, 1)));
        assert_eq!(expiry_flag(e1, &more), Some(true));
        assert_eq!(expiry_flag(e1 - 1, &more), Some(false));
        // outside the calendar's span: unknown.
        assert_eq!(expiry_flag(e2 + 1, &ex), None);
        assert_eq!(expiry_flag(d(2024, 1, 2), &ex), None);
    }

    #[test]
    fn k11_sn_x34_d_mend_reads_the_calendar_as_known_on_the_prior_evening() {
        // March 2024: sessions ..., 26, 27, 28 (Thu); 29 Good Friday closed; April sessions follow.
        let days = vec![d(2024, 3, 26), d(2024, 3, 27), d(2024, 3, 28), d(2024, 4, 1), d(2024, 4, 2)];
        let f = month_end_flags(&days, &[]);
        assert_eq!(f, vec![Some(false), Some(false), Some(true), Some(false), None], "the last March session is ON; the table's last month is unknown (weekdays left)");
        // SN-X34 (F4): a closure of 2024-03-28 announced ON 2024-03-27 (after the 26th's session): on the 26th's evening the 28th
        // was still a session, so the 27th (the realised last session) is NOT; the same closure announced on the 26th is known.
        let days2 = vec![d(2024, 3, 26), d(2024, 3, 27), d(2024, 4, 1)];
        let late_after = [(d(2024, 3, 28), d(2024, 3, 27))];
        let late_before = [(d(2024, 3, 28), d(2024, 3, 26))];
        assert_eq!(month_end_flags(&days2, &late_after).get(1).copied().flatten(), Some(false));
        assert_eq!(month_end_flags(&days2, &late_before).get(1).copied().flatten(), Some(true));
        // sessions of LATER months never move an earlier month's bit.
        let mut days3 = days.clone();
        days3.push(d(2024, 5, 2));
        assert_eq!(month_end_flags(&days3, &[]).get(2).copied().flatten(), Some(true));
        // a table ending on a month's last weekday: the rest of the month is a weekend, so the bit is known.
        let fri = d(2026, 10, 30);
        assert_eq!(month_end_flags(&[d(2026, 10, 29), fri], &[]), vec![Some(false), Some(true)]);
    }

    #[test]
    fn k11_g_drng_reads_the_session_so_far_and_the_atr_of_earlier_sessions() {
        let t = d(2024, 3, 5);
        let mk = |hh: i64, mm: i64, h: i64, l: i64| {
            let start = at(t, hh, mm, 0);
            (7_usize, Candle { start, end: start + 180, open: l, high: h, low: l, close: h, volume: 1 })
        };
        let mut s = Stream { tf_s: 180, source: String::new(), candles: vec![mk(9, 15, 110, 100), mk(9, 18, 120, 105), mk(9, 21, 200, 50)] };
        let r = day_range_so_far(&s);
        // at the 09:18 candle (END 09:21) the range is 120 - 100; the 09:21 candle (after S) is not in it.
        assert_eq!(r.get(1).map(|x| (x.2, x.3)), Some((120, 100)));
        if let Some(c) = s.candles.get_mut(2) {
            c.1.high = 10_000;
        }
        assert_eq!(day_range_so_far(&s).get(1).map(|x| (x.2, x.3)), Some((120, 100)), "a later candle never moves the range so far");
        // the decision candle itself (END = S) is in it.
        if let Some(c) = s.candles.get_mut(1) {
            c.1.high = 130;
        }
        assert_eq!(day_range_so_far(&s).get(1).map(|x| (x.2, x.3)), Some((130, 100)));
        // a new session starts a new range.
        s.candles.push((8, Candle { start: at(t + 1, 9, 15, 0), end: at(t + 1, 9, 18, 0), open: 1, high: 3, low: 1, close: 2, volume: 1 }));
        assert_eq!(day_range_so_far(&s).get(3).map(|x| (x.2, x.3)), Some((3, 1)));
        // the ratio x100: range 30 against ATR 20 (scaled) is 150.
        let (n, dn) = drng(130, 100, Some(20 * common_feat::S)).unwrap_or((0, 1));
        assert_eq!(n / dn, 150);
        assert_eq!(drng(130, 100, None), None, "ATR unready: unknown");
    }

    #[test]
    fn mirrors_follow_the_dial_rows() {
        let r = CtxRead { h52: Some(10_000), l52: Some(5_000), nifty_bp: Some((50, 1)), vix: Some(1_425), drng: Some((300, 2)), ..CtxRead::default() };
        // close 9,800: 2% below the 52-week high, 96% above the low.
        let v = |id: &str, long: bool| reading(id, &r, 9_800, long).map(|(n, dn)| n.div_euclid(dn));
        assert_eq!(v("G-52H", true), Some(-200));
        assert_eq!(v("G-52H", false), Some(-9_600), "short: -(close - low) / low");
        assert_eq!(v("G-52L", true), Some(9_600));
        assert_eq!(v("G-52L", false), Some(200), "short: -(close - high) / high");
        assert_eq!(v("X-NIFTY", true), v("X-NIFTY", false));
        assert_eq!(v("X-VIX", false), Some(1_425));
        assert_eq!(v("G-DRNG", true), Some(150));
    }
}
