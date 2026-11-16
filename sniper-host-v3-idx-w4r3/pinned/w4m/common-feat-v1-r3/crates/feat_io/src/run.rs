//! The readings run (3-MINUTE FIRST), the calibration table, the EMA start sweep and the step-count gate.

use std::fmt::Write as _;

use common_feat::cal::{ist_sod, ymd};
use common_feat::fold::Candle;
use common_feat::ind::EmaSeed;
use common_feat::levels::{Pivots, cpr_zone_name, piv_zone_name};
use common_feat::mtf::MtfSlot;
use common_feat::num::{Ratio, rupees, rupees_s, units_s};
use common_feat::series::{CandleCtx, Readings, Series, SeriesCfg};
use common_feat::steps::StepHist;

use crate::host::{Ctx, Levels, Stream, bp};

/// A higher-timeframe snapshot (the reading of one completed candle of another timeframe).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HtfSnap {
    /// RSI 14 (with its ready flag folded in: `None` when unready).
    pub rsi: Option<Ratio>,
    /// CCI 20.
    pub cci: Option<Ratio>,
    /// %R 14.
    pub wr: Option<Ratio>,
    /// Supertrend 10 x 3 side (true = green), when ready.
    pub st_up: Option<bool>,
    /// EMA 200 at `S`, when ready.
    pub ema200: Option<i128>,
}

fn snap(r: &Readings) -> HtfSnap {
    let ready_rsi = r.rsi_ready.first().copied().unwrap_or(false);
    HtfSnap {
        rsi: if ready_rsi { r.rsi.first().copied().flatten() } else { None },
        cci: r.cci.first().copied().flatten(),
        wr: r.wr.first().copied().flatten(),
        st_up: if r.st_ready.get(2).copied().unwrap_or(false) { r.st.get(2).copied().flatten().map(|s| s.up) } else { None },
        ema200: if r.ema_ready.get(4).copied().unwrap_or(false) { r.ema.get(4).copied().flatten() } else { None },
    }
}

fn opt_s(r: Option<Ratio>) -> String {
    r.map_or_else(|| "NA".into(), |x| x.floor_s().to_string())
}

fn opt_i(r: Option<i128>) -> String {
    r.map_or_else(|| "NA".into(), |x| x.to_string())
}

fn ctx_of(ctx: &Ctx, i: usize, prev: Option<usize>) -> CandleCtx {
    let s = ctx.sessions.get(i);
    let day = s.map_or(0, |s| s.day);
    let new_session = prev != Some(i);
    CandleCtx {
        session_open: s.map_or(0, crate::data::Session::open),
        day,
        new_session,
        regular_full: s.is_some_and(|s| s.regular_full),
        cont_reset: new_session && ctx.ca.cont_reset(day),
    }
}

/// The readings header.
#[must_use]
pub fn header(htf: &[String]) -> String {
    let mut h = String::from(
        "date\tstart\tk\ttod_s\twd\tentry\tkind\topen\thigh\tlow\tclose\tvolume\trsi14_u\trsi14_ready\tcci20_u\twr14_u\tbb20x2_basis_s\tbb20x2_upper_s\tbb20x2_lower_s\tbb20x3_basis_s\tbb20x3_upper_s\tbb20x3_lower_s\tst7x1.5_up\tst7x1.5_line_s\tst7x1.5_flip\tst10x2_up\tst10x2_line_s\tst10x2_flip\tst10x3_up\tst10x3_line_s\tst10x3_flip\tst_ready\tema9_s\tema20_s\tema50_s\tema100_s\tema200_s\tema_ready\tatr14_s\trange_atr_u\tcolour\tbody_pct_u\tupper_pct_u\tlower_pct_u\tpat_long\tpat_short\tpat_cold\trvol_u\tgap_off_bp_u\tgap_last_bp_u\tchg_off_bp_u\tchg_last_bp_u\tpiv_d\tcpr_d\tcpr_d_width_bp_u\tpiv_d_last\tpiv_w\tcpr_w\tpiv_m\tcpr_m\tpdh_bp_u\tpdl_bp_u\tspan_unknown_dwm\tk13_stock_refused\tsince_reset",
    );
    for t in htf {
        let _ = write!(h, "\t{t}_rsi14_u\t{t}_cci20_u\t{t}_wr14_u\t{t}_st10x3_up\t{t}_ema200_s");
    }
    // REPAIR 1 (B3): the counted reason a decision candle may not enter, LAST so every earlier column keeps its place
    h.push_str("\tentry_refusal");
    h
}

fn piv_cols(p: Option<Pivots>, close: i64) -> (String, String) {
    p.map_or_else(
        || ("NA".into(), "NA".into()),
        |p| (piv_zone_name(p.zone(close)).into(), cpr_zone_name(p.cpr_zone(close)).into()),
    )
}

fn bools(v: &[bool]) -> String {
    v.iter().map(|b| if *b { '1' } else { '0' }).collect()
}

/// One readings row.
#[allow(clippy::too_many_lines, reason = "one TSV row, column by column, in the header's order")]
#[must_use]
pub fn row(ctx: &Ctx, i: usize, r: &Readings, lv: &Levels, htf: &[Option<HtfSnap>]) -> String {
    let c = r.candle;
    let s = ctx.sessions.get(i);
    let mut o = String::with_capacity(900);
    let _ = write!(
        o,
        "{}\t{:02}:{:02}:{:02}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        ymd(r.day),
        r.tod / 3600,
        (r.tod / 60) % 60,
        r.tod % 60,
        r.k,
        r.tod,
        r.weekday,
        u8::from(s.is_some_and(|s| s.entry_at(c.end))),
        s.map_or("NA", |s| s.kind.as_str()),
        c.open,
        c.high,
        c.low,
        c.close,
        c.volume
    );
    let _ = write!(
        o,
        "\t{}\t{}\t{}\t{}",
        opt_s(r.rsi.first().copied().flatten()),
        u8::from(r.rsi_ready.first().copied().unwrap_or(false)),
        opt_s(r.cci.first().copied().flatten()),
        opt_s(r.wr.first().copied().flatten())
    );
    for k in 0..2 {
        match r.bb.get(k).copied().flatten() {
            Some(b) => {
                let _ = write!(o, "\t{}\t{}\t{}", b.basis, b.upper, b.lower);
            }
            None => o.push_str("\tNA\tNA\tNA"),
        }
    }
    for k in 0..3 {
        match r.st.get(k).copied().flatten() {
            Some(x) => {
                let _ = write!(o, "\t{}\t{}\t{}", u8::from(x.up), x.line, u8::from(x.flip));
            }
            None => o.push_str("\tNA\tNA\tNA"),
        }
    }
    let _ = write!(o, "\t{}", bools(&r.st_ready));
    for k in 0..5 {
        let _ = write!(o, "\t{}", opt_i(r.ema.get(k).copied().flatten()));
    }
    let _ = write!(o, "\t{}\t{}\t{}", bools(&r.ema_ready), opt_i(r.atr), opt_s(r.range_atr));
    let a = r.anat;
    let _ = write!(
        o,
        "\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        a.colour,
        opt_s(a.body_pct),
        opt_s(a.upper_pct),
        opt_s(a.lower_pct),
        a.pat_long,
        a.pat_short,
        a.pat_cold,
        opt_s(r.rvol)
    );
    let gap_off = lv.open.zip(lv.pc_off).and_then(|(x, b)| bp(x, b));
    let gap_last = lv.open.zip(lv.pc_last).and_then(|(x, b)| bp(x, b));
    let chg_off = lv.pc_off.and_then(|b| bp(c.close, b));
    let chg_last = lv.pc_last.and_then(|b| bp(c.close, b));
    let _ = write!(o, "\t{}\t{}\t{}\t{}", opt_s(gap_off), opt_s(gap_last), opt_s(chg_off), opt_s(chg_last));
    let (pd, cd) = piv_cols(lv.piv_d, c.close);
    let width = lv.piv_d.and_then(|p| p.cpr_width_bp());
    let (pdl, _) = piv_cols(lv.piv_d_last, c.close);
    let (pw, cw) = piv_cols(lv.piv_w, c.close);
    let (pm, cm) = piv_cols(lv.piv_m, c.close);
    let pdh = lv.pdh.and_then(|h| bp(c.close, h));
    let pdlo = lv.pdl.and_then(|l| bp(c.close, l));
    let refused = lv.span_unknown.iter().any(|x| *x);
    let _ = write!(
        o,
        "\t{pd}\t{cd}\t{}\t{pdl}\t{pw}\t{cw}\t{pm}\t{cm}\t{}\t{}\t{}\t{}\t{}",
        opt_s(width),
        opt_s(pdh),
        opt_s(pdlo),
        bools(&lv.span_unknown),
        u8::from(refused),
        r.since_reset
    );
    for h in htf {
        match h {
            Some(h) => {
                let _ = write!(
                    o,
                    "\t{}\t{}\t{}\t{}\t{}",
                    opt_s(h.rsi),
                    opt_s(h.cci),
                    opt_s(h.wr),
                    h.st_up.map_or("NA", |u| if u { "1" } else { "0" }),
                    opt_i(h.ema200)
                );
            }
            None => o.push_str("\tNA\tNA\tNA\tNA\tNA"),
        }
    }
    let _ = write!(o, "\t{}", s.map_or("NA", |s| s.refusal(c.end).unwrap_or("-")));
    o
}

/// The higher-timeframe streams fed in time order: each holds its own `Series` and its slot.
pub struct Htf {
    /// Its name.
    pub name: String,
    stream: Stream,
    series: Series,
    at: usize,
    prev: Option<usize>,
    slot: MtfSlot<HtfSnap>,
}

impl Htf {
    /// A higher-timeframe feed.
    ///
    /// # Errors
    ///
    /// A bad series configuration.
    pub fn new(name: &str, stream: Stream) -> Result<Self, String> {
        let series = Series::new(SeriesCfg::sniper(stream.tf_s))?;
        Ok(Self { name: name.into(), stream, series, at: 0, prev: None, slot: MtfSlot::new() })
    }

    /// Folds every candle that has ENDED by `s`, then reads the slot at `s` (K1, OBF35).
    pub fn at(&mut self, ctx: &Ctx, s: i64) -> Option<HtfSnap> {
        while let Some(&(i, c)) = self.stream.candles.get(self.at) {
            // A candle is put only once it has ended. Under the mutant m_mtf_forming the slot reads the last put
            // one whatever its end: the feed then puts candles at their START, the look-ahead the test catches.
            let gate = if cfg!(feature = "m_mtf_forming") { c.start } else { c.end };
            if gate > s {
                break;
            }
            let cc = ctx_of(ctx, i, self.prev);
            self.prev = Some(i);
            let (r, _) = self.series.fold(&c, &cc);
            let sn = snap(r);
            self.slot.put(c.end, sn);
            self.at += 1;
        }
        self.slot.at(s)
    }

    /// Reads that met a forming candle.
    #[must_use]
    pub const fn forming_refused(&self) -> u64 {
        self.slot.forming_refused()
    }

    /// K13 (REPAIR 1, B1): the decision stream's CONT readings reset at instant `at` (the ex-date session's open); this
    /// feed never again reads a candle that ended at or before it.
    pub fn cont_reset(&mut self, at: i64) {
        self.slot.reset_at(at);
    }

    /// Reads that met only a pre-reset candle (read as unknown).
    #[must_use]
    pub const fn reset_refused(&self) -> u64 {
        self.slot.reset_refused()
    }
}

/// The result of a readings run.
pub struct RunOut {
    /// Rows written.
    pub rows: u64,
    /// Per-candle step histogram.
    pub hist: StepHist,
    /// The calibration candle's readings, if the run met it.
    pub calib: Option<(Readings, Levels)>,
    /// Unready counts per reading over the rows (rsi, ema200, st10x3).
    pub unready: [u64; 3],
    /// Sessions refused by K13's stock span (rows).
    pub k13_rows: u64,
    /// Decision candles whose END lies inside a market-wide halt (`decision_in_halt`, entry 0; REPAIR 1 B3).
    pub halt_rows: u64,
    /// Decision candles of a session that is never an entry session (`session_no_entry`).
    pub no_entry_session_rows: u64,
}

/// Runs the readings of one decision timeframe, writing rows to `out` (when given), with the higher-timeframe feeds.
///
/// # Errors
///
/// A bad series configuration.
pub fn run(
    ctx: &Ctx,
    stream: &Stream,
    lvl: &[Levels],
    htf: &mut [Htf],
    mut out: Option<&mut dyn std::io::Write>,
    calib_at: Option<(i32, i64)>,
) -> Result<RunOut, String> {
    let mut series = Series::new(SeriesCfg::sniper(stream.tf_s))?;
    let mut prev = None;
    let mut res = RunOut { rows: 0, hist: StepHist::new(), calib: None, unready: [0; 3], k13_rows: 0, halt_rows: 0, no_entry_session_rows: 0 };
    if let Some(w) = out.as_deref_mut() {
        let names: Vec<String> = htf.iter().map(|h| h.name.clone()).collect();
        writeln!(w, "{}", header(&names)).map_err(|e| e.to_string())?;
    }
    let empty = Levels::default();
    for &(i, c) in &stream.candles {
        let cc = ctx_of(ctx, i, prev);
        prev = Some(i);
        if cc.cont_reset {
            // K13 (REPAIR 1, B1): the higher timeframes reset with the decision stream
            for h in htf.iter_mut() {
                h.cont_reset(cc.session_open);
            }
        }
        let snaps: Vec<Option<HtfSnap>> = htf.iter_mut().map(|h| h.at(ctx, c.end)).collect();
        if let Some(s) = ctx.sessions.get(i) {
            res.halt_rows += u64::from(s.in_halt(c.end));
            res.no_entry_session_rows += u64::from(!s.entry);
        }
        let (r, st) = series.fold(&c, &cc);
        res.hist.add(st.total());
        let lv = lvl.get(i).unwrap_or(&empty);
        if !r.rsi_ready.first().copied().unwrap_or(false) {
            res.unready[0] += 1;
        }
        if !r.ema_ready.get(4).copied().unwrap_or(false) {
            res.unready[1] += 1;
        }
        if !r.st_ready.get(2).copied().unwrap_or(false) {
            res.unready[2] += 1;
        }
        if lv.span_unknown.iter().any(|x| *x) {
            res.k13_rows += 1;
        }
        if let Some(w) = out.as_deref_mut() {
            writeln!(w, "{}", row(ctx, i, r, lv, &snaps)).map_err(|e| e.to_string())?;
        }
        if calib_at == Some((r.day, ist_sod(c.start))) {
            res.calib = Some((r.clone(), *lv));
        }
        res.rows += 1;
    }
    Ok(res)
}

/// The chart's values (SNIPER-REQUIREMENTS section 4; the operator's Dhan chart: a TEST of the formulas, never data).
pub const CHART: [(&str, &str); 12] = [
    ("open", "1825.00"),
    ("high", "1831.70"),
    ("low", "1803.80"),
    ("close", "1831.50"),
    ("RSI 14", "87.36"),
    ("CCI 20", "655.67"),
    ("%R 14", "-0.25"),
    ("Supertrend 10 x 3", "1786.81"),
    ("BB 20 x 3 basis", "1762.96"),
    ("BB 20 x 3 upper", "1810.52"),
    ("BB 20 x 3 lower", "1715.39"),
    ("EMA 200", "1802.98"),
];

/// The module's values for the calibration candle, in the order of [`CHART`] (two decimals, half away from zero).
#[must_use]
pub fn calib_values(r: &Readings) -> [String; 12] {
    let c = r.candle;
    let na = || "NA".to_string();
    let bb = r.bb.get(1).copied().flatten();
    [
        rupees(c.open),
        rupees(c.high),
        rupees(c.low),
        rupees(c.close),
        r.rsi.first().copied().flatten().map_or_else(na, |x| units_s(x.floor_s(), 2)),
        r.cci.first().copied().flatten().map_or_else(na, |x| units_s(x.floor_s(), 2)),
        r.wr.first().copied().flatten().map_or_else(na, |x| units_s(x.floor_s(), 2)),
        r.st.get(2).copied().flatten().map_or_else(na, |x| rupees_s(x.line)),
        bb.map_or_else(na, |b| rupees_s(b.basis)),
        bb.map_or_else(na, |b| rupees_s(b.upper)),
        bb.map_or_else(na, |b| rupees_s(b.lower)),
        r.ema.get(4).copied().flatten().map_or_else(na, rupees_s),
    ]
}

/// EMA 200 started `back` candles before the calibration candle (index `at` of the stream), with `seed`.
#[must_use]
pub fn ema_from(stream: &Stream, at: usize, back: usize, seed: EmaSeed) -> Option<i128> {
    let mut e = common_feat::ind::Ema::new(200, seed);
    let from = at.checked_sub(back)?;
    for &(_, c) in stream.candles.get(from..=at)? {
        e.fold(c.close);
    }
    e.value()
}

/// The 1x / 10x / 100x step-count gate (SNIPER-ADANIENT-PLAN 8.4, the HARD gate): the stream's candles folded
/// `times` times in a row through ONE series (the state carries across copies, as more history would), returning the
/// per-candle step histogram and the wall nanoseconds histogram per candle (p50 / p99 / max printed beside, never the
/// gate).
///
/// # Errors
///
/// A bad series configuration.
pub fn step_gate(ctx: &Ctx, stream: &Stream, times: usize) -> Result<(StepHist, Vec<u64>), String> {
    let mut series = Series::new(SeriesCfg::sniper(stream.tf_s))?;
    let mut hist = StepHist::new();
    let mut ns: Vec<u64> = Vec::with_capacity(stream.candles.len().saturating_mul(times).min(50_000_000));
    for _ in 0..times {
        let mut prev = None;
        for &(i, c) in &stream.candles {
            let cc = ctx_of(ctx, i, prev);
            prev = Some(i);
            let t0 = std::time::Instant::now();
            let (_, st) = series.fold(&c, &cc);
            let dt = u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX);
            hist.add(st.total());
            ns.push(dt);
        }
    }
    ns.sort_unstable();
    Ok((hist, ns))
}

/// Nearest-rank percentile of a sorted list.
#[must_use]
pub fn pct(sorted: &[u64], permille: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (sorted.len() * permille).div_ceil(1000).max(1);
    sorted.get(rank - 1).copied().unwrap_or(0)
}

/// A candle at an offset (the EMA sweep reports the start candle of each `back`).
#[must_use]
pub fn candle_at(stream: &Stream, idx: usize) -> Option<Candle> {
    stream.candles.get(idx).map(|x| x.1)
}
