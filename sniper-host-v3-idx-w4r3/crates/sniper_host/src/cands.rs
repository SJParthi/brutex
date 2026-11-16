//! The candidate rows of one decision timeframe: every candle's RAW readings from `common_feat` (the minute families on Zerodha's
//! own candles after the Kite undo, SNIPER s13 / S1, through `feat_io::minute_stream`), their exact core buckets for both sides,
//! the outer dials' value predicates (every level from the dials file's rows; each comparison is the module's own exact reading),
//! and the exit anchors of the signal exits (XS, XE).
//!
//! No reading is computed here: a value is a `common_feat` reading compared with a data-row level, by `Ratio` cross-multiplication
//! or by the module's own predicate (`BbReading::above_upper_by`, `Pivots::zone`, ...).

use common_feat::levels::{CprZone, PivZone, Pivots};
use common_feat::num::Ratio;
use common_feat::series::{CandleCtx, Readings, Series, SeriesCfg};
use feat_io::host::{Ctx, Levels, Stream};
use sniper_kernel::bits::Bits;
use sniper_kernel::ladder::Ladder;
use sniper_kernel::line::Row;

use crate::rows::{CoreLadders, DialRow, ExitKind, ExitSet};

/// The Supertrend rows the exits and T-ST read (period, factor milli), in the dials file's order (7 / 10 / 14 x 1.5 / 2 / 3).
pub const ST_ROWS: [(u16, u32); 9] = [(7, 1500), (7, 2000), (7, 3000), (10, 1500), (10, 2000), (10, 3000), (14, 1500), (14, 2000), (14, 3000)];
/// The EMA periods (T-EMA, XE).
pub const EMA_ROWS: [u16; 5] = [9, 20, 50, 100, 200];

/// One decision candle.
#[derive(Clone, Debug)]
pub struct Cand {
    /// Its index in the family (the H-* dials read the family's higher-timeframe table by it).
    pub idx: u32,
    /// The session index (the dated session table).
    pub sess: u32,
    /// The IST day.
    pub day: i32,
    /// Its month key.
    pub mk: i32,
    /// Its candle number in the session.
    pub k: u16,
    /// Start and END, epoch seconds.
    pub start: i64,
    /// See `start`.
    pub end: i64,
    /// OHLC, paisa.
    pub ohlc: [i64; 4],
    /// An entry session and an END strictly before the broker's cut.
    pub entry: bool,
    /// Core buckets, long then short: RSI, CCI, %R.
    pub core: [[u16; 3]; 2],
    /// Supertrend per [`ST_ROWS`]: (up, line at S, flip), `None` when unready.
    pub st: [Option<(bool, i128, bool)>; 9],
    /// EMA per [`EMA_ROWS`] at S, `None` when unready.
    pub ema: [Option<i128>; 5],
    /// ATR 14 at S (REQ 47's stop distance), `None` when unready.
    pub atr: Option<i128>,
    /// The readings the outer dials compare (kept whole: the module's own exact values).
    pub r: Readings,
    /// The day's levels (pivots, previous close, ...).
    pub lv: Levels,
}

/// One timeframe's candidates.
#[derive(Clone, Debug, Default)]
pub struct Family {
    /// Candle length, seconds.
    pub tf_s: i64,
    /// The source label (printed on every result).
    pub source: String,
    /// The candidates, oldest first (rows of one session contiguous).
    pub cands: Vec<Cand>,
    /// Candles refused as decisions (not an entry session, END inside a market-wide halt, at or after the cut), counted.
    pub not_entry: u64,
    /// The refused decision candles by reason (HOST REPAIR H1 (b)): `Session::refusal`'s own words (`session_no_entry`,
    /// `decision_in_halt`, both joined by `+`) and `at_or_after_cut`, each counted, never silent.
    pub refused: std::collections::BTreeMap<String, u64>,
    /// The K13 CONT reset instants of the decision stream (the ex-date session's open, at the stream's first candle of that
    /// session with `cont_reset`), ascending: every higher-timeframe slot resets with them (HOST REPAIR H1 (a)).
    pub resets: Vec<i64>,
    /// Per other timeframe (slot order): candidate reads refused because the last completed candle ended at or before the
    /// last K13 reset (`MtfSlot::reset_refused`), counted.
    pub htf_reset_refused: Vec<u64>,
    /// K_tf: the largest candle number before the cut.
    pub kmax: u16,
    /// The higher-timeframe reads (H-* dials): `n_htf` slots per candidate, flat (`cand * n_htf + slot`); empty until attached.
    pub htf: Vec<Option<HtfRead>>,
    /// Slots per candidate (the family's other timeframes: 8 for minutes, 4 for seconds).
    pub n_htf: usize,
    /// The other timeframes in slot order (printed in the value labels).
    pub htf_names: Vec<String>,
    /// DIALS BUILD: the seven context / calendar / range readings per candidate (`crate::ctxdials`), indexed like `cands`; EMPTY
    /// in the OFF mode (`--context-dials off`), where those dials stay `live_not_wired` exactly as in host v1 (binary k).
    pub ctxr: Vec<crate::ctxdials::CtxRead>,
}

pub(crate) fn ratio(r: Option<Ratio>) -> Option<(i128, i128)> {
    r.map(|x| (x.num, x.den))
}

fn ctx_of(ctx: &Ctx, i: usize, prev: Option<usize>) -> CandleCtx {
    let s = ctx.sessions.get(i);
    let day = s.map_or(0, |s| s.day);
    let new_session = prev != Some(i);
    CandleCtx {
        session_open: s.map_or(0, feat_io::data::Session::open),
        day,
        new_session,
        regular_full: s.is_some_and(|s| s.regular_full),
        cont_reset: new_session && ctx.ca.cont_reset(day),
    }
}

/// The series configuration: the module's sniper rows with the nine Supertrend rows split over two series (`common_feat`
/// holds at most `MAXK` = 8 rows of a kind in one series).
fn cfgs(tf_s: i64) -> (SeriesCfg, SeriesCfg) {
    let mut a = SeriesCfg::sniper(tf_s);
    a.st = ST_ROWS.iter().take(8).copied().collect();
    a.ema = EMA_ROWS.to_vec();
    let mut b = SeriesCfg::sniper(tf_s);
    b.st = ST_ROWS.iter().skip(8).copied().collect();
    (a, b)
}

/// Builds the candidates of one stream. `cut_sod`: the broker's own-order cut (a decision candle's END strictly before it).
///
/// # Errors
/// A series configuration the module refuses.
pub fn build(ctx: &Ctx, stream: &Stream, lv: &[Levels], core: (&CoreLadders, &CoreLadders), cut_sod: i64) -> Result<Family, String> {
    let mut b = CandBuilder::new(stream.tf_s, &stream.source)?;
    for &(i, c) in &stream.candles {
        let idx = u32::try_from(b.fam.cands.len()).unwrap_or(u32::MAX);
        if let Some(cand) = b.push(ctx, i, &c, lv, core, cut_sod, idx) {
            b.fam.cands.push(cand);
        }
    }
    Ok(b.fam)
}

/// The incremental form of [`build`] (SECONDS BUILDER, 2026-11-14): one decision candle at a time, the series state carried, so the
/// seconds families fold their candles one session day at a time (the 1-s source is never held whole). [`build`] is this builder
/// over a whole stream; every count lands in `fam` exactly as before (the candidates themselves are the caller's to keep).
pub struct CandBuilder {
    sa: Series,
    sb: Series,
    prev: Option<usize>,
    /// The family's counts (`not_entry`, `refused`, `resets`, `kmax`); `cands` holds what the caller pushes.
    pub fam: Family,
}

impl CandBuilder {
    /// A builder of `tf_s`-second candles labelled `source`.
    ///
    /// # Errors
    /// A series configuration the module refuses.
    pub fn new(tf_s: i64, source: &str) -> Result<Self, String> {
        let (ca, cb) = cfgs(tf_s);
        Ok(Self { sa: Series::new(ca)?, sb: Series::new(cb)?, prev: None, fam: Family { tf_s, source: source.to_string(), ..Family::default() } })
    }

    /// Folds candle `c` of session `i` (every candle of the stream, in order) and returns its candidate, numbered `idx`, when it is
    /// a decision candle that may enter (counted by reason when it may not).
    #[allow(clippy::too_many_arguments, reason = "the build loop's own inputs, one candle at a time")]
    pub fn push(&mut self, ctx: &Ctx, i: usize, c: &common_feat::fold::Candle, lv: &[Levels], core: (&CoreLadders, &CoreLadders), cut_sod: i64, idx: u32) -> Option<Cand> {
        let cc = ctx_of(ctx, i, self.prev);
        self.prev = Some(i);
        if cc.cont_reset && self.fam.resets.last().is_none_or(|&x| x < cc.session_open) {
            // K13 (HOST REPAIR H1 (a), as feat_io::run::run): the higher timeframes reset with the decision stream.
            self.fam.resets.push(cc.session_open);
        }
        let (rb, _) = self.sb.fold(c, &cc);
        let st9 = rb.st.first().copied().flatten().filter(|_| rb.st_ready.first().copied().unwrap_or(false));
        let (r, _) = self.sa.fold(c, &cc);
        let fam = &mut self.fam;
        let sess = ctx.sessions.get(i)?;
        let sod_end = c.end - (i64::from(r.day) * 86_400 - crate::price::IST_S);
        // HOST REPAIR H1 (b): the decision is the session's own law at the candle END (`Session::entry_at`: an entry session
        // and its END outside every market-wide halt), never the bare session flag. Mutant m_h1_entry reads the bare flag.
        let may = if cfg!(feature = "m_h1_entry") { sess.entry } else { sess.entry_at(c.end) };
        let entry = may && sod_end < cut_sod;
        if !entry {
            fam.not_entry += 1;
            let why = if may { "at_or_after_cut" } else { sess.refusal(c.end).unwrap_or("session_no_entry") };
            *fam.refused.entry(why.to_string()).or_insert(0) += 1;
            return None;
        }
        let k = u16::try_from(r.k.max(0)).unwrap_or(u16::MAX);
        fam.kmax = fam.kmax.max(k);
        let rsi = if r.rsi_ready.first().copied().unwrap_or(false) { ratio(r.rsi.first().copied().flatten()) } else { None };
        let cci = ratio(r.cci.first().copied().flatten());
        let wr = ratio(r.wr.first().copied().flatten());
        let b = |l: &CoreLadders| [l.l[0].bucket(rsi), l.l[1].bucket(cci), l.l[2].bucket(wr)];
        let mut st: [Option<(bool, i128, bool)>; 9] = [None; 9];
        for (j, x) in st.iter_mut().enumerate().take(8) {
            if r.st_ready.get(j).copied().unwrap_or(false) {
                *x = r.st.get(j).copied().flatten().map(|s| (s.up, s.line, s.flip));
            }
        }
        if let Some(x) = st.get_mut(8) {
            *x = st9.map(|s| (s.up, s.line, s.flip));
        }
        let mut ema: [Option<i128>; 5] = [None; 5];
        for (j, x) in ema.iter_mut().enumerate() {
            if r.ema_ready.get(j).copied().unwrap_or(false) {
                *x = r.ema.get(j).copied().flatten();
            }
        }
        let (y, m, _) = common_feat::cal::civil(r.day);
        Some(Cand {
            idx,
            sess: u32::try_from(i).unwrap_or(u32::MAX),
            day: r.day,
            mk: y * 12 + i32::try_from(m).unwrap_or(1) - 1,
            k,
            start: c.start,
            end: c.end,
            ohlc: [c.open, c.high, c.low, c.close],
            entry,
            core: [b(core.0), b(core.1)],
            st,
            ema,
            atr: if r.atr_ready { r.atr } else { None },
            r: r.clone(),
            lv: lv.get(i).copied().unwrap_or_default(),
        })
    }
}

/// The kernel rows of one side over the given candidate indices.
#[must_use]
pub fn kernel_rows(fam: &Family, long: bool, idx: &[usize]) -> Vec<Row> {
    let s = usize::from(!long);
    idx.iter()
        .filter_map(|&i| fam.cands.get(i))
        .map(|c| {
            let b = c.core.get(s).copied().unwrap_or_default();
            Row { sess: c.sess, k: u32::from(c.k), rb: b[0], cb: b[1], wb: b[2] }
        })
        .collect()
}

/// The signal exit anchor of candle `i` under `exit` (plan 4 item 5): XS: the END of the first later candle of the session that
/// CLOSES on the other side of that Supertrend line; when the entry candle already closes on the exit side, the next candle's
/// END (F17, `immediate`). XE: the first later candle closing across the EMA. `None`: no such candle (hold to the square-off).
/// XC (SNIPER s20): its Supertrend part is EXACTLY the XS rule of the same setting (the same arm, the same F17 immediate exit);
/// its clock part is priced in `price` (crate `Rule::TimeExit` from E0).
#[must_use]
pub fn signal_exit(fam: &Family, i: usize, long: bool, exit: &ExitSet) -> Option<(i64, bool)> {
    let c = fam.cands.get(i)?;
    let st_j = |p: u16, f: u32| ST_ROWS.iter().position(|&r| r == (p, f));
    let against = |x: &Cand| -> Option<bool> {
        match exit.kind {
            ExitKind::Supertrend(p, f) | ExitKind::SupertrendOrClock(p, f, _) => x.st.get(st_j(p, f)?).copied().flatten().map(|(up, _, _)| up != long),
            ExitKind::Ema(p) => {
                let e = x.ema.get(EMA_ROWS.iter().position(|&q| q == p)?).copied().flatten()?;
                let close_s = i128::from(x.ohlc[3]) * common_feat::S;
                Some(if long { close_s < e } else { close_s > e })
            }
            _ => None,
        }
    };
    let immediate = matches!(exit.kind, ExitKind::Supertrend(..) | ExitKind::SupertrendOrClock(..)) && against(c) == Some(true);
    let mut j = i + 1;
    while let Some(x) = fam.cands.get(j) {
        if x.sess != c.sess {
            return None;
        }
        if immediate || against(x) == Some(true) {
            return Some((x.end, immediate));
        }
        j += 1;
    }
    None
}

/// S7: the XE context filter: the entry candle's close on the WITH side of EMA P (a long above it, a short below it).
#[must_use]
pub fn xe_with(c: &Cand, long: bool, p: u16) -> bool {
    let Some(e) = EMA_ROWS.iter().position(|&q| q == p).and_then(|j| c.ema.get(j).copied().flatten()) else { return false };
    let close_s = i128::from(c.ohlc[3]) * common_feat::S;
    if long { close_s > e } else { close_s < e }
}

/// One outer dial value: its dial, its value index (1-based; 0 is OFF), its label, and its kept rows.
#[derive(Clone, Debug)]
pub struct OuterValue {
    /// The dial's position in the dials file.
    pub dial: usize,
    /// The value index (1..radix-1).
    pub value: u64,
    /// A printable label.
    pub label: String,
    /// The kept rows (over the context's row list).
    pub kept: Bits,
    /// EMPTY by construction (S7's (P, AGAINST) under its own XE context), counted, never walked.
    pub empty: bool,
}

/// What the host wired, what it did not (printed on SN4: never silently dropped).
#[derive(Clone, Debug, Default)]
pub struct Wiring {
    /// Dials wired with their value counts.
    pub wired: Vec<(String, u64)>,
    /// LIVE dials not wired in host v1 (named on every result; walked when wired).
    pub not_wired: Vec<String>,
    /// HELD / LATE dials (never walked in stage 1).
    pub held: Vec<String>,
    /// The radix of every dial in the file order (OFF included), for the canonical code.
    pub radix: Vec<u64>,
}

pub(crate) fn terms(values: &str) -> Vec<(String, Vec<i64>)> {
    values
        .split(';')
        .filter_map(|t| {
            let (cmp, spec) = t.split_once('=')?;
            let lv: Vec<i64> = if let Some(r) = spec.strip_prefix("range:") {
                let p: Vec<i64> = r.split(':').filter_map(|x| x.parse().ok()).collect();
                match p.as_slice() {
                    [lo, hi, st] if *st > 0 && hi >= lo => (0..=(hi - lo) / st).map(|i| lo + i * st).collect(),
                    _ => Vec::new(),
                }
            } else if let Some(l) = spec.strip_prefix("list:") {
                l.split('|').filter_map(|x| x.parse().ok()).collect()
            } else {
                Vec::new()
            };
            Some((cmp.to_string(), lv))
        })
        .collect()
}

pub(crate) fn list_labels(values: &str) -> Vec<String> {
    values.split_once("list:").map(|(_, l)| l.split('|').map(str::to_string).collect()).unwrap_or_default()
}

/// The value count of a dial row (OFF included), from its grammar: the canonical code's radix (3.4).
#[must_use]
pub fn radix_of(d: &DialRow, kmax: u16, minutes: bool) -> u64 {
    let htf: u64 = if minutes { 8 } else { 4 };
    let m = d.mult.trim();
    let mult: u64 = if let Some(k) = m.strip_prefix("htf*") { htf * k.parse::<u64>().unwrap_or(1) } else if m == "htf" { htf } else { m.parse().unwrap_or(1) };
    let v = &d.values;
    let n: u64 = if v.contains("=cand") {
        u64::from(kmax)
    } else if v.contains("=dowsets") {
        30
    } else if v.starts_with("EQ=list:") {
        list_labels(v).len() as u64
    } else {
        terms(v).iter().map(|(_, l)| l.len() as u64).sum()
    };
    1 + mult * n
}

pub(crate) fn cmp_ok(cmp: &str, r: Option<(i128, i128)>, level: i64) -> bool {
    let Some((num, den)) = r else { return false };
    let l = i128::from(level) * den;
    match cmp {
        "GE" => num >= l,
        "LE" => num <= l,
        "LT" => num < l,
        _ => false,
    }
}

pub(crate) fn pct100(r: Option<Ratio>) -> Option<(i128, i128)> {
    r.map(|x| (x.num * 100, x.den))
}

pub(crate) fn piv_label(z: PivZone, label: &str, long: bool) -> bool {
    use PivZone::{AboveR2, BelowS2, PR1, R1R2, S1P, S2S1};
    let l = if long {
        label.to_string()
    } else {
        // the short side reads the mirror column's zone for the same value position
        match label {
            "ABOVE_P" => "BELOW_P",
            "ABOVE_R1" => "BELOW_S1",
            "ABOVE_R2" => "BELOW_S2",
            "P_R1" => "S1_P",
            "R1_R2" => "S2_S1",
            "S1_P" => "P_R1",
            x => x,
        }
        .to_string()
    };
    match l.as_str() {
        "ABOVE_P" => matches!(z, PR1 | R1R2 | AboveR2),
        "ABOVE_R1" => matches!(z, R1R2 | AboveR2),
        "ABOVE_R2" => z == AboveR2,
        "P_R1" => z == PR1,
        "R1_R2" => z == R1R2,
        "S1_P" => z == S1P,
        "BELOW_P" => matches!(z, S1P | S2S1 | BelowS2),
        "BELOW_S1" => matches!(z, S2S1 | BelowS2),
        "BELOW_S2" => z == BelowS2,
        "S2_S1" => z == S2S1,
        _ => false,
    }
}

pub(crate) fn cpr_label(z: CprZone, label: &str, long: bool) -> bool {
    match (label, long) {
        ("ABOVE_TC", true) => z == CprZone::Above,
        ("ABOVE_TC", false) => z == CprZone::Below,
        ("INSIDE", _) => z == CprZone::Inside,
        _ => false,
    }
}

/// The outer values of the dials the host wires, over the context rows `idx` (candidate indices), for one side. LIVE dials not
/// wired yet are named in `Wiring::not_wired`; HELD and LATE dials are never walked in stage 1.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one arm per wired dial, each a data-row ladder against the module's reading")]
pub fn outer_values(fam: &Family, idx: &[usize], long: bool, dials: &[DialRow], xe: Option<u16>) -> (Vec<OuterValue>, Wiring) {
    let n = idx.len();
    let mut out = Vec::new();
    let mut wiring = Wiring::default();
    let cand = |r: usize| idx.get(r).and_then(|&i| fam.cands.get(i));
    for (di, d) in dials.iter().enumerate() {
        wiring.radix.push(radix_of(d, fam.kmax, fam.tf_s >= 60));
        if d.id.starts_with("M-") {
            continue;
        }
        if !d.status.starts_with("LIVE") {
            wiring.held.push(format!("{} ({})", d.id, d.status));
            continue;
        }
        let ts = terms(&d.values);
        let mut vi = 0_u64;
        let mut push = |label: String, pred: &dyn Fn(&Cand) -> bool, empty: bool, out: &mut Vec<OuterValue>| {
            vi += 1;
            let mut kept = Bits::new(n);
            if !empty {
                for r in 0..n {
                    if cand(r).is_some_and(pred) {
                        kept.set(r);
                    }
                }
            }
            out.push(OuterValue { dial: di, value: vi, label, kept, empty });
        };
        let before = out.len();
        match d.id.as_str() {
            "C-IDX" => {
                for k in 1..=fam.kmax {
                    push(format!("C-IDX={k}"), &|c: &Cand| c.k == k, false, &mut out);
                }
            }
            "C-COL" => {
                for lab in list_labels(&d.values) {
                    let green = lab == "GREEN";
                    push(format!("C-COL={lab}"), &|c: &Cand| if green { c.ohlc[3] > c.ohlc[0] } else { c.ohlc[3] < c.ohlc[0] }, false, &mut out);
                }
            }
            "K-DOW" => {
                for set in 1_u8..31 {
                    // bit b of the set = weekday b + 1 (Monday = 1 ... Friday = 5); all five = OFF (31 is not a value).
                    push(format!("K-DOW={set:05b}"), &|c: &Cand| (1..=5).contains(&c.r.weekday) && (set >> (c.r.weekday - 1)) & 1 == 1, false, &mut out);
                }
            }
            "T-ST" => {
                for (j, _) in ST_ROWS.iter().enumerate() {
                    for side in 0..2_u8 {
                        for flip in 0..3_u8 {
                            for (cmp, lv) in &ts {
                                for &x in lv {
                                    let pred = |c: &Cand| {
                                        let Some((up, line, f)) = c.st.get(j).copied().flatten() else { return false };
                                        let with = up == long;
                                        let side_ok = if side == 0 { with } else { !with };
                                        let flip_ok = match flip {
                                            1 => f,
                                            2 => !f,
                                            _ => true,
                                        };
                                        let close_s = i128::from(c.ohlc[3]) * common_feat::S;
                                        let dist = if (side == 0) == long { close_s - line } else { line - close_s };
                                        side_ok && flip_ok && cmp_ok(cmp, Some((dist * 10_000, close_s)), x)
                                    };
                                    push(format!("T-ST[{j},{side},{flip}] {cmp} {x}"), &pred, false, &mut out);
                                }
                            }
                        }
                    }
                }
            }
            "T-EMA" => {
                for (j, &p) in EMA_ROWS.iter().enumerate() {
                    for side in 0..2_u8 {
                        // S7: under XE-P, (P, AGAINST) is EMPTY by construction (counted, never walked).
                        let empty = xe == Some(p) && side == 1;
                        for (cmp, lv) in &ts {
                            for &x in lv {
                                let pred = |c: &Cand| {
                                    let Some(e) = c.ema.get(j).copied().flatten() else { return false };
                                    let close_s = i128::from(c.ohlc[3]) * common_feat::S;
                                    let above_side = (side == 0) == long;
                                    let dist = if above_side { close_s - e } else { e - close_s };
                                    dist >= 0 && cmp_ok(cmp, Some((dist * 10_000, close_s)), x)
                                };
                                push(format!("T-EMA[{p},{side}] {cmp} {x}"), &pred, empty, &mut out);
                            }
                        }
                    }
                }
            }
            "V-BB" => {
                for j in 0..2_usize {
                    for (_, lv) in &ts {
                        for &x in lv {
                            let pred = |c: &Cand| {
                                c.r.bb.get(j).copied().flatten().is_some_and(|b| if long { b.above_upper_by(i128::from(x)) } else { b.below_lower_by(i128::from(x)) })
                            };
                            push(format!("V-BB[{j}] beyond by {x} bp (of the band, the module's reading)"), &pred, false, &mut out);
                        }
                    }
                }
            }
            "C-BODY" | "C-UW" | "C-LW" | "C-RNG" | "V-RVOL" | "C-GAP" | "P-CHG" | "G-PDH" | "G-PDL" | "L-CPRW" => {
                let id = d.id.clone();
                for (cmp, lv) in &ts {
                    for &x in lv {
                        let id2 = id.clone();
                        let pred = move |c: &Cand| {
                            let a = &c.r.anat;
                            let reading: Option<(i128, i128)> = match id2.as_str() {
                                "C-BODY" => ratio(a.body_pct),
                                "C-UW" => ratio(if long { a.upper_pct } else { a.lower_pct }),
                                "C-LW" => ratio(if long { a.lower_pct } else { a.upper_pct }),
                                "C-RNG" => pct100(c.r.range_atr),
                                "V-RVOL" => pct100(c.r.rvol),
                                "C-GAP" => c.lv.open.zip(c.lv.pc_off).and_then(|(o, p)| ratio(feat_io::host::bp(o, p))),
                                "P-CHG" => c.lv.pc_off.and_then(|p| ratio(feat_io::host::bp(c.ohlc[3], p))),
                                "G-PDH" => {
                                    if long {
                                        c.lv.pdh.and_then(|p| ratio(feat_io::host::bp(c.ohlc[3], p)))
                                    } else {
                                        c.lv.pdl.and_then(|p| feat_io::host::bp(c.ohlc[3], p)).map(|r| (-r.num, r.den))
                                    }
                                }
                                "G-PDL" => {
                                    if long {
                                        c.lv.pdl.and_then(|p| ratio(feat_io::host::bp(c.ohlc[3], p)))
                                    } else {
                                        c.lv.pdh.and_then(|p| feat_io::host::bp(c.ohlc[3], p)).map(|r| (-r.num, r.den))
                                    }
                                }
                                "L-CPRW" => c.lv.piv_d.and_then(|p| ratio(p.cpr_width_bp())),
                                _ => None,
                            };
                            cmp_ok(cmp, reading, x)
                        };
                        push(format!("{id} {cmp} {x}"), &pred, false, &mut out);
                    }
                }
            }
            "L-PIV" | "L-CPR" => {
                let piv = d.id == "L-PIV";
                for lab in list_labels(&d.values) {
                    let pred = |c: &Cand| {
                        let Some(p): Option<Pivots> = c.lv.piv_d else { return false };
                        if piv { piv_label(p.zone(c.ohlc[3]), &lab, long) } else { cpr_label(p.cpr_zone(c.ohlc[3]), &lab, long) }
                    };
                    push(format!("{}={lab}{}", d.id, if long { "" } else { " (mirror)" }), &pred, false, &mut out);
                }
            }
            "L-PIVW" | "L-PIVM" | "L-CPRWK" | "L-CPRMO" => {
                // B5 spans: a weekly / monthly level whose source window is unknown (K13) is `unknown`: never true.
                let span = if matches!(d.id.as_str(), "L-PIVW" | "L-CPRWK") { 1 } else { 2 };
                let piv = d.id.starts_with("L-PIV");
                for lab in list_labels(&d.values) {
                    let pred = |c: &Cand| {
                        if c.lv.span_unknown.get(span).copied().unwrap_or(true) {
                            return false;
                        }
                        let Some(p): Option<Pivots> = (if span == 1 { c.lv.piv_w } else { c.lv.piv_m }) else { return false };
                        if piv { piv_label(p.zone(c.ohlc[3]), &lab, long) } else { cpr_label(p.cpr_zone(c.ohlc[3]), &lab, long) }
                    };
                    push(format!("{}={lab}{}", d.id, if long { "" } else { " (mirror)" }), &pred, false, &mut out);
                }
            }
            "L-CPRWKW" | "L-CPRMOW" => {
                let span = if d.id == "L-CPRWKW" { 1 } else { 2 };
                for (cmp, lv) in &ts {
                    for &x in lv {
                        let pred = |c: &Cand| {
                            if c.lv.span_unknown.get(span).copied().unwrap_or(true) {
                                return false;
                            }
                            let p = if span == 1 { c.lv.piv_w } else { c.lv.piv_m };
                            cmp_ok(cmp, p.and_then(|p| ratio(p.cpr_width_bp())), x)
                        };
                        push(format!("{} {cmp} {x}", d.id), &pred, false, &mut out);
                    }
                }
            }
            "V-BBW" => {
                // The module's exact width test (`BbReading::width_ge`, bp of the basis); LE reads "not at least x" (an equality
                // exactly at x, a measure-zero case of an irrational width, reads as not LE; named).
                for j in 0..2_usize {
                    for (cmp, lv) in &ts {
                        for &x in lv {
                            let ge = cmp == "GE";
                            let pred = |c: &Cand| c.r.bb.get(j).copied().flatten().is_some_and(|b| b.width_ge(i128::from(x)) == ge);
                            push(format!("V-BBW[{j}] {cmp} {x}"), &pred, false, &mut out);
                        }
                    }
                }
            }
            "C-PAT" => {
                use common_feat::anatomy::pat;
                for lab in list_labels(&d.values) {
                    let bit = match lab.as_str() {
                        "MARUBOZU" => pat::MARUBOZU,
                        "ENGULF" | "ENGULFING" => pat::ENGULFING,
                        "HAMMER" => pat::HAMMER,
                        "INSIDE" => pat::INSIDE,
                        "OUTSIDE" => pat::OUTSIDE,
                        "DOJI" => pat::DOJI,
                        "NR4" => pat::NR4,
                        "NR7" => pat::NR7,
                        _ => 0,
                    };
                    // a cold pattern (needs earlier candles of the same session, K15) is unknown: never true.
                    let pred = |c: &Cand| bit != 0 && c.r.anat.pat_cold & bit == 0 && (if long { c.r.anat.pat_long } else { c.r.anat.pat_short }) & bit != 0;
                    push(format!("C-PAT={lab}{}", if long { "" } else { " (mirror)" }), &pred, bit == 0, &mut out);
                }
            }
            "H-RSI" | "H-CCI" | "H-WR" | "H-BB" | "H-ST" | "H-EMA" if fam.n_htf > 0 => {
                // REQ 76e (2): the reading of ANOTHER timeframe of the same family, its last COMPLETED candle at S (common_feat's
                // MtfSlot, attached by `attach_htf`); every level a bucket of the dial row's own ladder (long or mirror).
                let side = usize::from(!long);
                let get = |c: &Cand, h: usize| fam.htf.get(c.idx as usize * fam.n_htf + h).copied().flatten();
                for h in 0..fam.n_htf {
                    let hn = fam.htf_names.get(h).cloned().unwrap_or_default();
                    match d.id.as_str() {
                        "H-RSI" | "H-CCI" | "H-WR" => {
                            let n = ts.iter().map(|t| t.1.len()).sum::<usize>();
                            for j in 1..=n {
                                let jj = u16::try_from(j).unwrap_or(u16::MAX);
                                let id = d.id.clone();
                                let pred = move |c: &Cand| {
                                    get(c, h).is_some_and(|r| {
                                        let b = match id.as_str() {
                                            "H-RSI" => r.rsi,
                                            "H-CCI" => r.cci,
                                            _ => r.wr,
                                        };
                                        b.get(side).copied().unwrap_or(0) >= jj
                                    })
                                };
                                push(format!("{}[{hn}] rung {j}", d.id), &pred, false, &mut out);
                            }
                        }
                        "H-BB" => {
                            let n = ts.iter().map(|t| t.1.len()).sum::<usize>();
                            for b in 0..2_usize {
                                for j in 1..=n {
                                    let jj = u16::try_from(j).unwrap_or(u16::MAX);
                                    let pred = move |c: &Cand| get(c, h).is_some_and(|r| r.bb.get(side).and_then(|x| x.get(b)).copied().unwrap_or(0) >= jj);
                                    push(format!("H-BB[{hn},{b}] rung {j}"), &pred, false, &mut out);
                                }
                            }
                        }
                        "H-ST" => {
                            for j in 0..9_usize {
                                for (li, lab) in list_labels(&d.values).iter().enumerate() {
                                    let agree = li == 0;
                                    let _ = lab;
                                    let pred = move |c: &Cand| get(c, h).is_some_and(|r| match r.st.get(j).copied().unwrap_or(0) {
                                        1 => long == agree,
                                        2 => long != agree,
                                        _ => false,
                                    });
                                    push(format!("H-ST[{hn},{j}] {}", if agree { "AGREE" } else { "AGAINST" }), &pred, false, &mut out);
                                }
                            }
                        }
                        _ => {
                            for j in 0..5_usize {
                                for li in 0..list_labels(&d.values).len() {
                                    let agree = li == 0;
                                    let pred = move |c: &Cand| get(c, h).is_some_and(|r| match r.ema.get(j).copied().unwrap_or(0) {
                                        1 => long == agree,
                                        2 => long != agree,
                                        _ => false,
                                    });
                                    push(format!("H-EMA[{hn},{}] {}", EMA_ROWS.get(j).copied().unwrap_or(0), if agree { "AGREE" } else { "AGAINST" }), &pred, false, &mut out);
                                }
                            }
                        }
                    }
                }
            }
            "X-NIFTY" | "X-VIX" | "G-52H" | "G-52L" | "G-DRNG" if !fam.ctxr.is_empty() => {
                // DIALS BUILD: every level of the row's own terms (GE / LE / LT, in the file's order), against the K11 reading of
                // `ctxdials::reading` (the row's own mirror on the short side).
                let id = d.id.clone();
                for (cmp, lv) in &ts {
                    for &x in lv {
                        let id2 = id.clone();
                        let pred = move |c: &Cand| fam.ctxr.get(c.idx as usize).is_some_and(|r| cmp_ok(cmp, crate::ctxdials::reading(&id2, r, c.ohlc[3], long), x));
                        push(format!("{id} {cmp} {x}"), &pred, false, &mut out);
                    }
                }
            }
            "D-EXP" | "D-MEND" if !fam.ctxr.is_empty() => {
                // DIALS BUILD: ON / NOT from the row's list; an unknown bit is never true; a label other than ON / NOT is EMPTY.
                let exp = d.id == "D-EXP";
                for lab in list_labels(&d.values) {
                    let want = match lab.as_str() {
                        "ON" => Some(true),
                        "NOT" => Some(false),
                        _ => None,
                    };
                    let pred = move |c: &Cand| want.is_some() && fam.ctxr.get(c.idx as usize).and_then(|r| if exp { r.exp } else { r.mend }) == want;
                    push(format!("{}={lab}", d.id), &pred, want.is_none(), &mut out);
                }
            }
            _ => {
                wiring.not_wired.push(d.id.clone());
                continue;
            }
        }
        wiring.wired.push((d.id.clone(), (out.len() - before) as u64));
    }
    (out, wiring)
}

/// The full-core ladders of one side as a `sniper_kernel::line::Core`.
#[must_use]
pub fn core_of(l: &CoreLadders) -> sniper_kernel::line::Core {
    let v = |x: &Ladder| u16::try_from(x.values()).unwrap_or(u16::MAX);
    sniper_kernel::line::Core { nr: v(&l.l[0]), nc: v(&l.l[1]), nw: v(&l.l[2]) }
}

/// One other timeframe's last COMPLETED candle at a decision (REQ 76e (2)), compact: per side (long, short) the rung count of
/// the H-RSI / H-CCI / H-WR ladders (the dial rows' own ladders and mirrors) and of H-BB's excess ladder per band row; the
/// Supertrend side per row and the close against each EMA (0 unready, 1 up / above, 2 down / below, 3 equal).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HtfRead {
    /// H-RSI rungs passed (long, short).
    pub rsi: [u16; 2],
    /// H-CCI rungs passed.
    pub cci: [u16; 2],
    /// H-WR rungs passed.
    pub wr: [u16; 2],
    /// H-BB rungs passed per side and band row.
    pub bb: [[u16; 2]; 2],
    /// Supertrend side per [`ST_ROWS`].
    pub st: [u8; 9],
    /// The close against each EMA of [`EMA_ROWS`].
    pub ema: [u8; 5],
}

/// The H-* dials' ladders from their rows (long and the mirror), so every rung is a data row (S8's OPEN last rung included).
pub struct HtfLadders {
    rsi: [Ladder; 2],
    cci: [Ladder; 2],
    wr: [Ladder; 2],
    bb: Vec<i64>,
}

/// Builds the H-* ladders.
///
/// # Errors
/// A missing or malformed H-* row.
pub fn htf_ladders(dials: &[DialRow]) -> Result<HtfLadders, String> {
    let row = |id: &str| dials.iter().find(|d| d.id == id).ok_or_else(|| format!("dials row {id} missing"));
    let pair = |id: &str| -> Result<[Ladder; 2], String> {
        let r = row(id)?;
        let l = Ladder::long_of(id, &r.values).map_err(|e| format!("{id}: {e}"))?;
        let s = Ladder::short_of(&l, &r.mirror).map_err(|e| format!("{id}: {e}"))?;
        Ok([l, s])
    };
    let bb: Vec<i64> = terms(&row("H-BB")?.values).into_iter().flat_map(|t| t.1).collect();
    Ok(HtfLadders { rsi: pair("H-RSI")?, cci: pair("H-CCI")?, wr: pair("H-WR")?, bb })
}

/// Folds one other timeframe's stream through `common_feat` (the same series rows as [`build`]) and returns every candle's END
/// with its compact H-* reads. Every candle of the stream is folded and kept (entry or not: a higher timeframe's last completed
/// candle may belong to a special session).
///
/// # Errors
/// A series configuration the module refuses.
pub fn htf_reads(ctx: &Ctx, stream: &Stream, lad: &HtfLadders) -> Result<Vec<(i64, HtfRead)>, String> {
    let mut b = HtfBuilder::new(stream.tf_s)?;
    Ok(stream.candles.iter().map(|&(i, c)| b.push(ctx, i, &c, lad)).collect())
}

/// The incremental form of [`htf_reads`] (SECONDS BUILDER, 2026-11-14): one candle of the other timeframe at a time, its series
/// carried, so a seconds family folds its other timeframes one session day at a time. [`htf_reads`] is this builder over a stream.
pub struct HtfBuilder {
    sa: Series,
    sb: Series,
    prev: Option<usize>,
}

impl HtfBuilder {
    /// A builder of `tf_s`-second candles (the same series rows as [`build`]).
    ///
    /// # Errors
    /// A series configuration the module refuses.
    pub fn new(tf_s: i64) -> Result<Self, String> {
        let (ca, cb) = cfgs(tf_s);
        Ok(Self { sa: Series::new(ca)?, sb: Series::new(cb)?, prev: None })
    }

    /// Folds candle `c` of session `i` (every candle of the other stream, in order); returns its END and its compact H-* read.
    pub fn push(&mut self, ctx: &Ctx, i: usize, c: &common_feat::fold::Candle, lad: &HtfLadders) -> (i64, HtfRead) {
        let cc = ctx_of(ctx, i, self.prev);
        self.prev = Some(i);
        let (rb, _) = self.sb.fold(c, &cc);
        let rb_st = rb.st_ready.first().copied().unwrap_or(false).then(|| rb.st.first().copied().flatten()).flatten();
        let (r, _) = self.sa.fold(c, &cc);
        let rsi = if r.rsi_ready.first().copied().unwrap_or(false) { ratio(r.rsi.first().copied().flatten()) } else { None };
        let cci = ratio(r.cci.first().copied().flatten());
        let wr = ratio(r.wr.first().copied().flatten());
        let mut h = HtfRead::default();
        for s in 0..2_usize {
            if let (Some(a), Some(b), Some(w)) = (h.rsi.get_mut(s), h.cci.get_mut(s), h.wr.get_mut(s)) {
                *a = lad.rsi.get(s).map_or(0, |l| l.bucket(rsi));
                *b = lad.cci.get(s).map_or(0, |l| l.bucket(cci));
                *w = lad.wr.get(s).map_or(0, |l| l.bucket(wr));
            }
            for bi in 0..2_usize {
                let n = r.bb.get(bi).copied().flatten().map_or(0, |b| {
                    lad.bb.iter().take_while(|&&x| if s == 0 { b.above_upper_by(i128::from(x)) } else { b.below_lower_by(i128::from(x)) }).count()
                });
                if let Some(x) = h.bb.get_mut(s).and_then(|v| v.get_mut(bi)) {
                    *x = u16::try_from(n).unwrap_or(u16::MAX);
                }
            }
        }
        for (j, x) in h.st.iter_mut().enumerate() {
            let rd = if j < 8 { r.st_ready.get(j).copied().unwrap_or(false).then(|| r.st.get(j).copied().flatten()).flatten() } else { rb_st };
            *x = rd.map_or(0, |s| if s.up { 1 } else { 2 });
        }
        let close_s = i128::from(c.close) * common_feat::S;
        for (j, x) in h.ema.iter_mut().enumerate() {
            let e = if r.ema_ready.get(j).copied().unwrap_or(false) { r.ema.get(j).copied().flatten() } else { None };
            *x = e.map_or(0, |e| match close_s.cmp(&e) {
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Less => 2,
                std::cmp::Ordering::Equal => 3,
            });
        }
        (c.end, h)
    }
}

/// Attaches the other timeframes' reads to every candidate: slot `h` of candidate `c` is the read of timeframe `h`'s last candle
/// with END at or before `c`'s END that ended after the decision stream's last K13 reset, through `common_feat::mtf::MtfSlot`
/// with its `reset_at` (a candle still forming is never read; a pre-reset candle is never read, counted per timeframe).
pub fn attach_htf(fam: &mut Family, names: Vec<String>, others: &[Vec<(i64, HtfRead)>]) {
    let n = others.len();
    let ends: Vec<i64> = fam.cands.iter().map(|c| c.end).collect();
    let mut out = vec![None; fam.cands.len() * n];
    let mut refused = Vec::with_capacity(n);
    for (h, reads) in others.iter().enumerate() {
        let (vals, rr) = htf_slots(&ends, &fam.resets, reads);
        refused.push(rr);
        for (ci, v) in vals.into_iter().enumerate() {
            if let Some(x) = out.get_mut(ci * n + h) {
                *x = v;
            }
        }
    }
    fam.htf = out;
    fam.n_htf = n;
    fam.htf_names = names;
    fam.htf_reset_refused = refused;
}

/// For decision ENDs `ends` (ascending), the decision stream's K13 reset instants `resets` (ascending; each the ex-date
/// session's open) and one other timeframe's `(end, value)` candles (ascending), the value of the last candle with
/// `end <= S` at each decision S that ended after the last reset before S, through `common_feat::mtf::MtfSlot` and its
/// `reset_at` (HOST REPAIR H1 (a), check-build-feat-v1-r2 section 7: never a bare slot). Every reset whose open lies before S
/// is given to the slot before it is read, exactly as `feat_io::run::run` resets its feeds at the stream's first candle of the
/// ex-date session (every candle of that session ends after its open; every earlier one at or before it). Returns the values
/// and the slot's `reset_refused` count. Mutant `m_s4` (plan m-S4) puts the next candle before it has ended; mutant
/// `m_h1_reset` never calls `reset_at` (the v1 host).
#[must_use]
pub fn htf_slots<T: Copy>(ends: &[i64], resets: &[i64], reads: &[(i64, T)]) -> (Vec<Option<T>>, u64) {
    let mut slot: common_feat::mtf::MtfSlot<T> = common_feat::mtf::MtfSlot::new();
    let mut p = 0_usize;
    let mut q = 0_usize;
    let mut out = Vec::with_capacity(ends.len());
    for &s in ends {
        while let Some(&r) = resets.get(q) {
            if r >= s {
                break;
            }
            if !cfg!(feature = "m_h1_reset") {
                slot.reset_at(r);
            }
            q += 1;
        }
        while let Some((end, v)) = reads.get(p) {
            if *end > s && !cfg!(feature = "m_s4") {
                break;
            }
            slot.put(*end, *v);
            p += 1;
            if cfg!(feature = "m_s4") {
                break;
            }
        }
        out.push(if cfg!(feature = "m_s4") { reads.get(p.saturating_sub(1)).map(|x| x.1) } else { slot.at(s) });
    }
    let refused = slot.reset_refused();
    (out, refused)
}

/// The streaming form of [`htf_slots`] (SECONDS BUILDER, 2026-11-14): the same `common_feat::mtf::MtfSlot` fed in the same order,
/// for a family folded one session day at a time. Before every read at decision END `s` it gives the slot every registered K13 reset
/// `r < s` (ascending), then every queued candle with `end <= s` (in order), then reads `MtfSlot::at(s)`: exactly [`htf_slots`]'s
/// three steps. The caller registers every reset and queues every candle of the other timeframe as it folds them (a candle is put
/// only once a later decision reaches its END). Test `htf_cursor_equals_htf_slots` holds it to [`htf_slots`] on random sequences;
/// mutant `m_sec_htf` puts the next queued candle before it has ended (a forming candle read): caught there.
#[derive(Clone, Debug)]
pub struct HtfCursor<T: Copy> {
    slot: common_feat::mtf::MtfSlot<T>,
    queue: std::collections::VecDeque<(i64, T)>,
    resets: std::collections::VecDeque<i64>,
}

impl<T: Copy> Default for HtfCursor<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy> HtfCursor<T> {
    /// An empty cursor.
    #[must_use]
    pub fn new() -> Self {
        Self { slot: common_feat::mtf::MtfSlot::new(), queue: std::collections::VecDeque::new(), resets: std::collections::VecDeque::new() }
    }

    /// Registers a K13 reset instant of the decision stream (ascending).
    pub fn reset(&mut self, r: i64) {
        self.resets.push_back(r);
    }

    /// Queues one candle of the other timeframe (ascending ENDs).
    pub fn put(&mut self, end: i64, v: T) {
        self.queue.push_back((end, v));
    }

    /// The value at decision END `s` (ascending `s`).
    pub fn at(&mut self, s: i64) -> Option<T> {
        while let Some(&r) = self.resets.front() {
            if r >= s {
                break;
            }
            self.slot.reset_at(r);
            self.resets.pop_front();
        }
        while let Some(&(end, v)) = self.queue.front() {
            if end > s && !cfg!(feature = "m_sec_htf") {
                break;
            }
            self.slot.put(end, v);
            self.queue.pop_front();
            if cfg!(feature = "m_sec_htf") {
                return Some(v);
            }
        }
        self.slot.at(s)
    }

    /// Reads refused because the last completed candle ended at or before the last reset (`MtfSlot::reset_refused`).
    #[must_use]
    pub fn reset_refused(&self) -> u64 {
        self.slot.reset_refused()
    }
}

#[cfg(test)]
mod tests {
    use super::{HtfCursor, htf_slots};

    #[test]
    fn htf_cursor_equals_htf_slots() {
        // random decision ENDs, other-timeframe ENDs and K13 resets; the cursor fed as a day-at-a-time builder feeds it (every
        // candle queued before the decisions that reach it, every reset registered before the first decision after it).
        let mut st = 0x2026_1114_u64;
        let mut next = || {
            st ^= st << 13;
            st ^= st >> 7;
            st ^= st << 17;
            st
        };
        for round in 0..200 {
            let mut t = 0_i64;
            let mut ends = Vec::new();
            let mut reads = Vec::new();
            let mut resets = Vec::new();
            let mut t2 = 0_i64;
            for _ in 0..60 {
                t += 1 + i64::try_from(next() % 7).unwrap();
                ends.push(t);
                if next() % 9 == 0 {
                    resets.push(t - 1 - i64::try_from(next() % 2).unwrap());
                }
            }
            resets.sort_unstable();
            resets.dedup();
            for k in 0..80 {
                t2 += 1 + i64::try_from(next() % 6).unwrap();
                reads.push((t2, k));
            }
            let (want, want_ref) = htf_slots(&ends, &resets, &reads);
            let mut c = HtfCursor::new();
            let (mut p, mut q) = (0, 0);
            let mut got = Vec::new();
            for &s in &ends {
                while let Some(&r) = resets.get(q) {
                    if r >= s {
                        break;
                    }
                    c.reset(r);
                    q += 1;
                }
                // a builder queues every candle as soon as it is folded: here, everything ending before the next decision
                while let Some(&(e, v)) = reads.get(p) {
                    if e > s + i64::try_from(next() % 5).unwrap() {
                        break;
                    }
                    c.put(e, v);
                    p += 1;
                }
                got.push(c.at(s));
            }
            assert_eq!(got, want, "round {round}");
            assert_eq!(c.reset_refused(), want_ref, "round {round}");
        }
    }

    #[test]
    fn sn_x15_a_higher_timeframe_reads_its_last_completed_candle_only() {
        // decisions end at 100, 200, 300, 450; the 15-minute-like candles end at 90, 200, 250, 400, 600.
        let reads = [(90_i64, 'a'), (200, 'b'), (250, 'c'), (400, 'd'), (600, 'e')];
        assert_eq!(htf_slots(&[100, 200, 300, 450], &[], &reads).0, vec![Some('a'), Some('b'), Some('c'), Some('d')]);
        // before the first completed candle: nothing (never the forming one).
        assert_eq!(htf_slots(&[50], &[], &reads).0, vec![None]);
    }

    #[test]
    fn h1a_a_higher_timeframe_never_reads_across_the_k13_reset() {
        // HOST REPAIR H1 (a) (check-build-feat-v1-r2 section 7 (a)): the decision stream resets at the ex-date session's open
        // 300. Decisions end at 290, 310, 340, 420; the other timeframe's candles end at 200, 280, 330, 400, 600. At 310 the
        // last completed candle (280) ended before the reset: unknown, counted; at 340 the post-reset candle 330 is read; a
        // decision before the reset still reads 280. Mutant m_h1_reset (no reset_at, the v1 host) reads 280 at 310: caught here.
        let reads = [(200_i64, 'a'), (280, 'b'), (330, 'c'), (400, 'd'), (600, 'e')];
        let (v, refused) = htf_slots(&[290, 310, 340, 420], &[300], &reads);
        assert_eq!(v, vec![Some('b'), None, Some('c'), Some('d')], "a pre-reset candle is never read after the K13 reset");
        assert_eq!(refused, 1, "the refused read is counted");
        // a candle ending exactly AT the reset instant is pre-reset (floor: end > reset only), as MtfSlot reads it.
        let (v, refused) = htf_slots(&[305], &[300], &[(300_i64, 'x')]);
        assert_eq!((v, refused), (vec![None], 1));
        // a decision ending exactly at the reset instant belongs to the earlier session: no reset applied yet.
        let (v, refused) = htf_slots(&[300], &[300], &[(280_i64, 'y')]);
        assert_eq!((v, refused), (vec![Some('y')], 0));
    }
}
