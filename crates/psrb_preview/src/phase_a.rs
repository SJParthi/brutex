//! Phase A: one stock's candles, readings, signals and would-be trades.
//!
//! A candidate's path depends only on its own stock's data, never on the portfolio
//! (PREVIEW-SPEC 5, Phase A). The stock's minutes are read through the store's own
//! read-only door (`BarFile::open_existing`), folded by the store's own fold
//! (`pull::fold::derive_minutes`, `PADDED_FOLD = Absent`), and every reading is the
//! part tip's own type.

use std::path::Path;

use brutex_core::price::Paisa;
use brutex_core::vendor::Vendor;
use costs::day::TradeDay;
use costs::equity_clock::{Fill, Order, TradeFills, TwoClock, price_trade};
use costs::rate::OrderSide;
use indicators::Candle;
use indicators::anchored::{DailyEligibility, DailyReference};
use indicators::bollinger::{self, Bollinger};
use indicators::cci::{self, Cci};
use indicators::dual_band_supertrend::DualBandSuperTrend;
use indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS;
use indicators::rsi::{self, Rsi};
use indicators::trend::{Ema, SCALE, Trend};
use indicators::williams_r::{self, WilliamsR};
use pull::calendar::{DayKind, Runtime, Session};
use pull::cash_auction::Schedule;
use pull::fno_membership::Membership;
use pull::fold::{Bucket, derive_minutes};
use pull::session::Day;
use pull::vendor::Venue;
use store::file::{BarFile, StoreError};
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::quarantine::Probe;

use crate::common::{
    CLOSE_CAS, CLOSE_FULL, CUT_MIN, Cand, EXPOSURE_PAISA, ExitKind, N_RS, N_TF, OPEN_MIN,
    READING_PERIODS, SQ_MIN, Status, TFS, US_MIN, US_S, WIN_FIRST_MI, WIN_LAST_MI, at_min,
    civil, day_number, ist_day_of, ist_min_of, month_of_index, session_day,
};

/// Every level and width of the three rule sets, built once (AD:96-98).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Levels {
    rsi_hi: rsi::Level,
    rsi_lo: rsi::Level,
    wr_m85: williams_r::Level,
    wr_m15: williams_r::Level,
    cci_hi: cci::Level,
    cci_lo: cci::Level,
    w2: bollinger::Width,
    w3: bollinger::Width,
}

impl Levels {
    pub(crate) fn new() -> Result<Self, String> {
        Ok(Self {
            rsi_hi: rsi::Level::from_ppm(700_000).map_err(|e| format!("RSI 70 refused: {e:?}"))?,
            rsi_lo: rsi::Level::from_ppm(300_000).map_err(|e| format!("RSI 30 refused: {e:?}"))?,
            wr_m85: williams_r::Level::from_ppm(-850_000)
                .map_err(|e| format!("W%R -85 refused: {e:?}"))?,
            wr_m15: williams_r::Level::from_ppm(-150_000)
                .map_err(|e| format!("W%R -15 refused: {e:?}"))?,
            cci_hi: cci::Level::from_milli(200_000).map_err(|e| format!("CCI +200 refused: {e:?}"))?,
            cci_lo: cci::Level::from_milli(-200_000)
                .map_err(|e| format!("CCI -200 refused: {e:?}"))?,
            w2: bollinger::Width::from_milli(2_000).map_err(|e| format!("BB 2 refused: {e:?}"))?,
            w3: bollinger::Width::from_milli(3_000).map_err(|e| format!("BB 3 refused: {e:?}"))?,
        })
    }
}

/// One stock of the 218.
#[derive(Clone, Debug)]
pub(crate) struct Sym {
    /// The 218 spelling (membership, reports).
    pub(crate) name: String,
    /// The stored series (ICICILIFE reads ICICIPRULI).
    pub(crate) series: String,
    /// Rank in the 218 spellings' byte order.
    pub(crate) rank: u16,
}

/// What every stock shares.
pub(crate) struct Ctx<'a> {
    pub(crate) store: &'a Path,
    /// `None`: today's list on every day (the probe failed).
    pub(crate) membership: Option<&'a Membership>,
    pub(crate) levels: Levels,
    pub(crate) win_first: i64,
    pub(crate) win_last: i64,
    pub(crate) cas_day: i64,
    pub(crate) outage_day: i64,
    pub(crate) gdfl_gap_day: i64,
    pub(crate) oct_first: i64,
    pub(crate) carry_day: Day,
}

/// Counters of one stock (summed over stocks by the caller).
#[derive(Clone, Debug, Default)]
pub(crate) struct Counts {
    pub(crate) z_months_read: u64,
    pub(crate) z_months_missing: u64,
    pub(crate) z_months_refused: u64,
    pub(crate) g_months_missing: u64,
    pub(crate) g_months_refused: u64,
    pub(crate) fold_errors: u64,
    pub(crate) charter_minutes_withheld: u64,
    pub(crate) m2_days: u64,
    pub(crate) m2_days_no_minutes: u64,
    pub(crate) trade_days: u64,
    pub(crate) member_false_days: u64,
    pub(crate) member_unknown_days: u64,
    pub(crate) prev_refused_days: u64,
    pub(crate) no_prev_days: u64,
    pub(crate) candidate_days: u64,
    pub(crate) fallback_days: u64,
    pub(crate) fallback_gap_days: u64,
    pub(crate) fallback_no_month_days: u64,
    pub(crate) fallback_empty_days: u64,
    pub(crate) fallback_read_error_days: u64,
    pub(crate) candles: [u64; N_TF],
    pub(crate) holes: [u64; N_TF],
    pub(crate) holes_missing_minute: [u64; N_TF],
    pub(crate) holes_withheld: [u64; N_TF],
    pub(crate) holes_refused: [u64; N_TF],
    pub(crate) padded_only: [u64; N_TF],
    pub(crate) unscheduled_bars: [u64; N_TF],
    /// [rule set][timeframe][reading]: decision candles blocked by that reading's taint.
    pub(crate) blocked: [[[u64; 6]; N_TF]; N_RS],
    /// Decision candles blocked by any taint.
    pub(crate) blocked_any: [[u64; N_TF]; N_RS],
    /// [rule set][timeframe][side]: signal candles before 15:08 (the candidates).
    pub(crate) signals: [[[u64; 2]; N_TF]; N_RS],
    /// Signal candles at or after 15:08:00 (counted per candle).
    pub(crate) cut: [[[u64; 2]; N_TF]; N_RS],
}

fn add6(a: &mut [u64; 6], b: &[u64; 6]) {
    for (x, y) in a.iter_mut().zip(b.iter()) {
        *x += *y;
    }
}

impl Counts {
    pub(crate) fn add(&mut self, o: &Self) {
        self.z_months_read += o.z_months_read;
        self.z_months_missing += o.z_months_missing;
        self.z_months_refused += o.z_months_refused;
        self.g_months_missing += o.g_months_missing;
        self.g_months_refused += o.g_months_refused;
        self.fold_errors += o.fold_errors;
        self.charter_minutes_withheld += o.charter_minutes_withheld;
        self.m2_days += o.m2_days;
        self.m2_days_no_minutes += o.m2_days_no_minutes;
        self.trade_days += o.trade_days;
        self.member_false_days += o.member_false_days;
        self.member_unknown_days += o.member_unknown_days;
        self.prev_refused_days += o.prev_refused_days;
        self.no_prev_days += o.no_prev_days;
        self.candidate_days += o.candidate_days;
        self.fallback_days += o.fallback_days;
        self.fallback_gap_days += o.fallback_gap_days;
        self.fallback_no_month_days += o.fallback_no_month_days;
        self.fallback_empty_days += o.fallback_empty_days;
        self.fallback_read_error_days += o.fallback_read_error_days;
        add6(&mut self.candles, &o.candles);
        add6(&mut self.holes, &o.holes);
        add6(&mut self.holes_missing_minute, &o.holes_missing_minute);
        add6(&mut self.holes_withheld, &o.holes_withheld);
        add6(&mut self.holes_refused, &o.holes_refused);
        add6(&mut self.padded_only, &o.padded_only);
        add6(&mut self.unscheduled_bars, &o.unscheduled_bars);
        for rs in 0..N_RS {
            for ti in 0..N_TF {
                if let (Some(a), Some(b)) = (
                    self.blocked.get_mut(rs).and_then(|x| x.get_mut(ti)),
                    o.blocked.get(rs).and_then(|x| x.get(ti)),
                ) {
                    add6(a, b);
                }
                if let (Some(a), Some(b)) = (
                    self.blocked_any.get_mut(rs).and_then(|x| x.get_mut(ti)),
                    o.blocked_any.get(rs).and_then(|x| x.get(ti)),
                ) {
                    *a += *b;
                }
                for side in 0..2 {
                    if let (Some(a), Some(b)) = (
                        self.signals.get_mut(rs).and_then(|x| x.get_mut(ti)).and_then(|x| x.get_mut(side)),
                        o.signals.get(rs).and_then(|x| x.get(ti)).and_then(|x| x.get(side)),
                    ) {
                        *a += *b;
                    }
                    if let (Some(a), Some(b)) = (
                        self.cut.get_mut(rs).and_then(|x| x.get_mut(ti)).and_then(|x| x.get_mut(side)),
                        o.cut.get(rs).and_then(|x| x.get(ti)).and_then(|x| x.get(side)),
                    ) {
                        *a += *b;
                    }
                }
            }
        }
    }
}

/// One stock's answer.
#[derive(Debug, Default)]
pub(crate) struct StockOut {
    pub(crate) cands: Vec<Cand>,
    pub(crate) counts: Counts,
    pub(crate) notes: Vec<String>,
    /// A refusal that stops the run by name (a `price_trade` refusal, a torn reading).
    pub(crate) fatal: Option<String>,
    /// The stock's trading span and warm-up start, for the notes.
    pub(crate) span: String,
}

/// One timeframe's readings, folded continuously from the stock's first candle.
struct TfState {
    st2: DualBandSuperTrend,
    st3: DualBandSuperTrend,
    bb: Bollinger,
    rsi: Rsi,
    wr: WilliamsR,
    cci: Cci,
    ema: Ema,
    /// Clean candles folded since the last hole, per reading (`u32::MAX` before any hole).
    clean: [u32; 6],
}

impl TfState {
    fn new() -> Result<Self, String> {
        Ok(Self {
            st2: DualBandSuperTrend::new(10, 2_000).map_err(|e| format!("ST 10/2: {e:?}"))?,
            st3: DualBandSuperTrend::new(10, 3_000).map_err(|e| format!("ST 10/3: {e:?}"))?,
            bb: Bollinger::new(20).map_err(|e| format!("BB 20: {e:?}"))?,
            rsi: Rsi::new(14).map_err(|e| format!("RSI 14: {e:?}"))?,
            wr: WilliamsR::new(14).map_err(|e| format!("W%R 14: {e:?}"))?,
            cci: Cci::new(20).map_err(|e| format!("CCI 20: {e:?}"))?,
            ema: Ema::new(200),
            clean: [u32::MAX; 6],
        })
    }

    /// Fold a candle into every reading or into none (AD 3): `Ok(false)` when a
    /// reading's check refuses it (folded into none, a hole).
    fn fold(&mut self, c: &Candle) -> Result<bool, String> {
        if c.check_evaluable().is_err() || bollinger::Close::new(c.close).is_err() {
            return Ok(false);
        }
        self.st2.fold(c).map_err(|e| format!("ST 10/2 refused a checked candle: {e:?}"))?;
        self.st3.fold(c).map_err(|e| format!("ST 10/3 refused a checked candle: {e:?}"))?;
        self.bb.fold(c).map_err(|e| format!("BB refused a checked candle: {e:?}"))?;
        self.rsi.fold(c).map_err(|e| format!("RSI refused a checked candle: {e:?}"))?;
        self.wr.fold(c).map_err(|e| format!("W%R refused a checked candle: {e:?}"))?;
        self.cci.fold(c).map_err(|e| format!("CCI refused a checked candle: {e:?}"))?;
        self.ema.fold(c.close);
        Ok(true)
    }

    fn hole(&mut self) {
        self.clean = [0; 6];
    }

    fn clean_step(&mut self) {
        for x in &mut self.clean {
            *x = x.saturating_add(1);
        }
    }

    /// (lower, upper) of ST 10/2 and ST 10/3 after the last fold, settled only.
    fn bands(&self) -> [Option<(i128, i128)>; 2] {
        let pick = |r: Option<indicators::dual_band_supertrend::Reading>| {
            r.filter(|x| x.settled).map(|x| (x.lower, x.upper))
        };
        [pick(self.st2.reading()), pick(self.st3.reading())]
    }
}

/// A candle of one timeframe on one day, as the trade walk reads it.
#[derive(Clone, Copy, Debug)]
struct Rec {
    /// Close instant T.
    t: i64,
    hole: bool,
    o: i64,
    h: i64,
    l: i64,
    c: i64,
    /// (lower, upper) of ST 10/2 [0] and ST 10/3 [1].
    st: [Option<(i128, i128)>; 2],
    /// Bit `rs * 2 + side`: a signal before 15:08.
    sig: u8,
}

enum Decision {
    NotReady,
    Blocked(u8),
    Ready(Option<usize>),
}

/// 3.1: every condition on the closed candle; `rs` 0 VIDEO-ORIGINAL, 1 AUTHOR-V2,
/// 2 VIDEO-SCREEN (AUTHOR-V2 with Williams %R long > -15, short < -85).
fn decide(
    rs: usize,
    s: &TfState,
    close: i64,
    prev: Option<(i128, i128, i128)>,
    narrow: bool,
    lv: &Levels,
) -> Decision {
    let vo = rs == 0;
    let st = if vo { s.st3.reading() } else { s.st2.reading() };
    let Some(st) = st else {
        return Decision::NotReady;
    };
    if !st.settled {
        return Decision::NotReady;
    }
    let Some(w) = s.bb.window() else {
        return Decision::NotReady;
    };
    let Some(r) = s.rsi.reading() else {
        return Decision::NotReady;
    };
    let wr = s.wr.reading();
    let cc = s.cci.reading();
    if !vo && (wr.is_none() || cc.is_none() || !s.ema.warm()) {
        return Decision::NotReady;
    }
    let used: &[usize] = if vo { &[0, 1, 2] } else { &[0, 1, 2, 3, 4, 5] };
    let mut mask = 0_u8;
    for &i in used {
        let clean = s.clean.get(i).copied().unwrap_or(0);
        let need = READING_PERIODS.get(i).copied().unwrap_or(u32::MAX);
        if clean < need {
            mask |= 1 << i;
        }
    }
    if mask != 0 {
        return Decision::Blocked(mask);
    }
    if !r.faithful() {
        return Decision::Ready(None);
    }
    let Some((h, l, pc)) = prev else {
        return Decision::Ready(None);
    };
    if !vo && !narrow {
        return Decision::Ready(None);
    }
    let Ok(cb) = bollinger::Close::new(close) else {
        return Decision::Ready(None);
    };
    let width = if vo { lv.w2 } else { lv.w3 };
    let c = i128::from(close);
    let t3 = h + l + pc;
    let mut long = 3 * c > 2 * t3 - 3 * l
        && st.trend == Trend::Up
        && w.above_upper(cb, width)
        && r.above(lv.rsi_hi);
    let mut short = 3 * c < 2 * t3 - 3 * h
        && st.trend == Trend::Down
        && w.below_lower(cb, width)
        && r.below(lv.rsi_lo);
    if !vo {
        let (Some(wr), Some(cc)) = (wr, cc) else {
            return Decision::Ready(None);
        };
        if wr.flat() || cc.flat() {
            return Decision::Ready(None);
        }
        let es = s.ema.scaled();
        let cs = c * SCALE;
        let (wl, ws) = if rs == 1 { (lv.wr_m85, lv.wr_m15) } else { (lv.wr_m15, lv.wr_m85) };
        long = long && wr.above(wl) && cc.above(lv.cci_hi) && cs > es;
        short = short && wr.below(ws) && cc.below(lv.cci_lo) && cs < es;
    }
    if long {
        Decision::Ready(Some(0))
    } else if short {
        Decision::Ready(Some(1))
    } else {
        Decision::Ready(None)
    }
}

/// C1 at `bp` basis points: `10_000 x |2C - H - L| <= bp x (H + L + C)`, ties narrow.
pub(crate) fn cpr_narrow(h: i128, l: i128, c: i128, bp: i128) -> bool {
    10_000 * (2 * c - h - l).abs() <= bp * (h + l + c)
}

fn candle_of(b: &Bar) -> Candle {
    Candle::new(b.ts_micros, b.open, b.high, b.low, b.close, b.volume, b.open_interest)
}

fn sym_id(series: &str) -> Result<u32, String> {
    u32::try_from(brutex_core::universe::fnv1a(series) & 0xFFFF_FFFF)
        .map_err(|_| format!("{series}: symbol id"))
}

enum MonthFile {
    Missing,
    Refused(String),
    Open(BarFile),
}

fn open_month(root: &Path, vendor: Vendor, series: &str, id: u32, tf: Timeframe, mi: usize) -> MonthFile {
    let (y, m) = match month_of_index(mi) {
        Ok(v) => v,
        Err(e) => return MonthFile::Refused(e),
    };
    let month = match YearMonth::new(y, m) {
        Ok(v) => v,
        Err(e) => return MonthFile::Refused(e.to_string()),
    };
    let path = match StorePath::new(PathParts {
        vendor,
        exchange: "NSE",
        segment: "CASH",
        symbol: series,
        contract: None,
        timeframe: tf,
        month,
        file: FileKind::Bars,
    }) {
        Ok(p) => p,
        Err(e) => return MonthFile::Refused(e.to_string()),
    };
    match store::quarantine::probe(root, path) {
        Ok(Probe::Clear) => {}
        Ok(Probe::Quarantined(_)) => {
            return MonthFile::Refused(format!("{series} {y}-{m:02} is quarantined"));
        }
        Err(e) => return MonthFile::Refused(format!("{series} {y}-{m:02} quarantine unknown: {e}")),
    }
    match BarFile::open_existing(root, path, id) {
        Ok(f) => MonthFile::Open(f),
        Err(StoreError::Missing { .. }) => MonthFile::Missing,
        Err(e) => MonthFile::Refused(format!("{series} {y}-{m:02}: {e}")),
    }
}

/// One month of stored minutes, oldest first, the charter's non-regular sessions
/// withheld (as `cli::stored::decode_loaded` withholds them for `load_span`).
fn read_minutes(f: &BarFile, withheld: &mut u64) -> Result<Vec<Bar>, String> {
    let n = f.records();
    let cap = usize::try_from(n).map_err(|_| format!("{n} records"))?;
    let mut v = Vec::with_capacity(cap);
    for i in 0..n {
        let b = f.read_record(i).map_err(|e| format!("record {i} of {n}: {e}"))?;
        if CHARTER_NON_REGULAR_IST_DAYS.contains(&ist_day_of(b.ts_micros)) {
            *withheld += 1;
            continue;
        }
        v.push(b);
    }
    Ok(v)
}

/// Index ranges of `bars` per IST day, in order.
fn by_day(bars: &[Bar]) -> Vec<(i64, usize, usize)> {
    let mut out: Vec<(i64, usize, usize)> = Vec::new();
    for (i, b) in bars.iter().enumerate() {
        let d = ist_day_of(b.ts_micros);
        match out.last_mut() {
            Some(last) if last.0 == d => last.2 = i + 1,
            _ => out.push((d, i, i + 1)),
        }
    }
    out
}

/// The slice of `bars` on day `d`, walking `cursor` forward through `groups`.
fn day_slice<'b>(bars: &'b [Bar], groups: &[(i64, usize, usize)], cursor: &mut usize, d: i64) -> &'b [Bar] {
    while let Some(g) = groups.get(*cursor) {
        if g.0 < d {
            *cursor += 1;
            continue;
        }
        if g.0 == d {
            return bars.get(g.1..g.2).unwrap_or(&[]);
        }
        break;
    }
    &[]
}

/// One traded second (or, on a fallback day, one traded minute).
#[derive(Clone, Copy, Debug)]
struct Tick {
    /// Start of the second (minute).
    ts: i64,
    /// The fill's second for slots and costs: the second itself, or minute start + 59 s.
    fill: i64,
    high: i64,
    low: i64,
}

#[derive(Clone, Copy, Debug, Default)]
struct Sim {
    status: Status,
    kind: ExitKind,
    entry_fill: i64,
    entry_px: i64,
    qty: i64,
    exit_fill: i64,
    exit_px: i64,
    stop0: i64,
    late: bool,
}

/// FSPEC 6.6 R1: `L_g = L_z x G / Z`, Z and G of the latest minute ending at or before
/// `t` that has both a traded Zerodha minute and a GDFL second; a long's level floored,
/// a short's ceiled. On a fallback day (`best` = None) the level is used as it is.
fn map_level(band: i128, best: Option<&[Option<(i64, i64)>]>, d: i64, t: i64, long: bool) -> Option<i64> {
    let Some(b) = best else {
        return i64::try_from(band).ok();
    };
    let j = (t - at_min(d, OPEN_MIN)).div_euclid(US_MIN) - 1;
    let (z, g) = b.get(usize::try_from(j).ok()?).copied().flatten()?;
    if z <= 0 {
        return None;
    }
    let num = band.checked_mul(i128::from(g))?;
    let den = i128::from(z);
    let q = if long { num.div_euclid(den) } else { -((-num).div_euclid(den)) };
    i64::try_from(q).ok()
}

fn exit_at(mut sim: Sim, ticks: &[Tick], k: usize, kind: ExitKind, long: bool, e: usize, end_fill: i64) -> Sim {
    if let Some(x) = ticks.get(k) {
        sim.kind = kind;
        sim.exit_px = if long { x.low } else { x.high };
        sim.exit_fill = x.fill;
    } else {
        // Nothing traded to the session end: the worst print since the entry, by the
        // broker, flagged (D-8127's remainder rule, AD:339-346).
        let rest = ticks.get(e..).unwrap_or(&[]);
        let w = if long {
            rest.iter().map(|x| x.low).min()
        } else {
            rest.iter().map(|x| x.high).max()
        };
        sim.kind = ExitKind::Unclosable;
        sim.exit_px = w.unwrap_or(sim.entry_px);
        sim.exit_fill = end_fill;
    }
    sim
}

/// 3.5: the entry, the trailed resting stop, the flip, the time exit and the square-off.
#[allow(clippy::too_many_arguments, reason = "one walk, every input named")]
fn simulate(
    ticks: &[Tick],
    best: Option<&[Option<(i64, i64)>]>,
    recs: &[Rec],
    i: usize,
    st_idx: usize,
    long: bool,
    d: i64,
    close_min: i64,
    time_from: i64,
) -> Sim {
    let mut sim = Sim::default();
    let Some(rec) = recs.get(i) else {
        return sim;
    };
    let t0 = rec.t;
    let e = ticks.partition_point(|x| x.ts < t0);
    let Some(et) = ticks.get(e) else {
        sim.status = Status::EntryNotFilled;
        return sim;
    };
    let px = if long { et.high } else { et.low };
    sim.entry_px = px;
    sim.entry_fill = et.fill;
    if px <= 0 {
        sim.status = Status::ZeroQty;
        return sim;
    }
    let qty = EXPOSURE_PAISA.div_euclid(px);
    if qty == 0 {
        sim.status = Status::ZeroQty;
        return sim;
    }
    sim.qty = qty;
    sim.late = et.fill - t0 > 60 * US_S;
    let Some((lo0, up0)) = rec.st.get(st_idx).copied().flatten() else {
        sim.status = Status::NoRatio;
        return sim;
    };
    let band0 = if long { lo0 } else { up0 };
    let Some(mut stop) = map_level(band0, best, d, t0, long) else {
        sim.status = Status::NoRatio;
        return sim;
    };
    sim.stop0 = stop;
    sim.status = Status::Filled;
    let t_cut = at_min(d, CUT_MIN);
    let t_time = t_cut - US_S;
    let t_sq = at_min(d, SQ_MIN);
    let end_fill = at_min(d, close_min) - US_S;
    let hit = |x: &Tick, stop: i64| if long { x.low <= stop } else { x.high >= stop };
    let mut cur = e + 1;
    if et.fill >= t_time {
        // An entry filled at or after 15:07:59 goes straight to the square-off rule.
        let k = cur + ticks.get(cur..).map_or(0, |rest| rest.partition_point(|x| x.ts < t_sq));
        return exit_at(sim, ticks, k, ExitKind::SquareOff, long, e, end_fill);
    }
    for r in recs.get(i + 1..).unwrap_or(&[]) {
        if r.t >= t_cut {
            break;
        }
        while let Some(x) = ticks.get(cur) {
            if x.ts >= r.t {
                break;
            }
            if hit(x, stop) {
                return exit_at(sim, ticks, cur, ExitKind::Stop, long, e, end_fill);
            }
            cur += 1;
        }
        if r.hole {
            continue;
        }
        let Some((lo, up)) = r.st.get(st_idx).copied().flatten() else {
            continue;
        };
        let c = i128::from(r.c);
        if (long && c < lo) || (!long && c > up) {
            return exit_at(sim, ticks, cur, ExitKind::Flip, long, e, end_fill);
        }
        let band = if long { lo } else { up };
        if let Some(v) = map_level(band, best, d, r.t, long)
            && ((long && v > stop) || (!long && v < stop))
        {
            stop = v;
        }
    }
    while let Some(x) = ticks.get(cur) {
        if x.ts >= time_from {
            break;
        }
        if hit(x, stop) {
            return exit_at(sim, ticks, cur, ExitKind::Stop, long, e, end_fill);
        }
        cur += 1;
    }
    match ticks.get(cur) {
        Some(x) if x.ts < t_sq => exit_at(sim, ticks, cur, ExitKind::Time, long, e, end_fill),
        Some(_) => exit_at(sim, ticks, cur, ExitKind::SquareOff, long, e, end_fill),
        None => exit_at(sim, ticks, ticks.len(), ExitKind::Unclosable, long, e, end_fill),
    }
}

/// Prices one candidate by `costs::equity_clock::price_trade` (3.7).
pub(crate) fn price(c: &Cand) -> Result<TwoClock, String> {
    let long = c.side == 0;
    let (es, xs) = if long { (OrderSide::Buy, OrderSide::Sell) } else { (OrderSide::Sell, OrderSide::Buy) };
    let by_broker = matches!(c.exit_kind, ExitKind::SquareOff | ExitKind::Unclosable);
    let fills = TradeFills {
        orders: vec![
            Order {
                side: es,
                by_broker: false,
                fills: vec![Fill {
                    second: c.entry_us.div_euclid(US_S),
                    quantity: c.qty,
                    price: Paisa::from_raw(c.entry_px),
                }],
            },
            Order {
                side: xs,
                by_broker,
                fills: vec![Fill {
                    second: c.exit_us.div_euclid(US_S),
                    quantity: c.qty,
                    price: Paisa::from_raw(c.exit_px),
                }],
            },
        ],
    };
    let (y, m, dd) = civil(ist_day_of(c.t_signal))?;
    let day = TradeDay::new(y, m, dd).map_err(|e| format!("trade day {y}-{m:02}-{dd:02}: {e:?}"))?;
    price_trade(&fills, day).map_err(|e| {
        format!(
            "price_trade refused stock rank {} {y}-{m:02}-{dd:02} entry {} x {} exit {}: {e}",
            c.stock, c.qty, c.entry_px, c.exit_px
        )
    })
}

/// The GDFL month a day's fills read, opened once, on first use.
enum GMonth {
    NotOpened,
    Missing,
    Refused,
    Open(BarFile),
}

struct StockState {
    tf: Vec<TfState>,
    /// The last M2 day's day bar: `Some(Ok(H, L, C))`, `Some(Err)` when refused.
    prev: Option<Result<(i128, i128, i128), ()>>,
}

/// Phase A for one stock.
pub(crate) fn run_stock(ctx: &Ctx<'_>, sym: &Sym) -> StockOut {
    let mut out = StockOut::default();
    if let Err(e) = stock_inner(ctx, sym, &mut out) {
        out.fatal = Some(format!("{}: {e}", sym.name));
    }
    out
}

#[allow(clippy::too_many_lines, reason = "one pass over one stock months")]
fn stock_inner(ctx: &Ctx<'_>, sym: &Sym, out: &mut StockOut) -> Result<(), String> {
    let id = sym_id(&sym.series)?;
    let n_mi = WIN_LAST_MI + 1;
    // Which months each feed holds (1 present, 0 absent, 2 refused).
    let mut zp = vec![0_u8; n_mi];
    for (mi, slot) in zp.iter_mut().enumerate() {
        *slot = match open_month(ctx.store, Vendor::Zerodha, &sym.series, id, Timeframe::MINUTE_1, mi) {
            MonthFile::Open(_) => 1,
            MonthFile::Missing => 0,
            MonthFile::Refused(why) => {
                out.notes.push(format!("{} zerodha month refused: {why}", sym.name));
                2
            }
        };
    }
    let mut gp = vec![0_u8; n_mi];
    for (mi, slot) in gp.iter_mut().enumerate().skip(WIN_FIRST_MI) {
        *slot = match open_month(ctx.store, Vendor::Gdfl, &sym.series, id, Timeframe::SECOND_1, mi) {
            MonthFile::Open(_) => 1,
            MonthFile::Missing => 0,
            MonthFile::Refused(why) => {
                out.notes.push(format!("{} gdfl month refused: {why}", sym.name));
                2
            }
        };
    }
    let first_z = zp.iter().position(|&x| x == 1);
    let first_g = gp.iter().position(|&x| x == 1);
    let (Some(first_z), Some(first_g)) = (first_z, first_g) else {
        out.notes.push(format!("{}: no Zerodha or no GDFL month in the store; not run", sym.name));
        return Ok(());
    };
    let start_mi = first_z.max(first_g).max(WIN_FIRST_MI);
    let Some(z_at) = (start_mi..n_mi).find(|&mi| zp.get(mi) == Some(&1)) else {
        out.notes.push(format!("{}: no Zerodha month from its trading start; not run", sym.name));
        return Ok(());
    };
    let mut warm = z_at;
    while warm > 0 && zp.get(warm - 1) == Some(&1) {
        warm -= 1;
    }
    let (sy, sm) = month_of_index(start_mi)?;
    let trade_start = day_number(sy, sm, 1)?.max(ctx.win_first);
    let (wy, wm) = month_of_index(warm)?;
    out.span = format!(
        "{}: candles from {wy}-{wm:02}, trading from {sy}-{sm:02} (first Zerodha month {}, first GDFL month {})",
        sym.name,
        month_of_index(first_z).map_or_else(|e| e, |(y, m)| format!("{y}-{m:02}")),
        month_of_index(first_g).map_or_else(|e| e, |(y, m)| format!("{y}-{m:02}")),
    );
    for mi in start_mi..n_mi {
        match gp.get(mi) {
            Some(0) => out.counts.g_months_missing += 1,
            Some(2) => out.counts.g_months_refused += 1,
            _ => {}
        }
    }
    let mut state = StockState {
        tf: (0..N_TF).map(|_| TfState::new()).collect::<Result<Vec<_>, _>>()?,
        prev: None,
    };
    for mi in warm..n_mi {
        let (y, m) = month_of_index(mi)?;
        let first_d = day_number(y, m, 1)?;
        let next_first = if m == 12 { day_number(y + 1, 1, 1)? } else { day_number(y, m + 1, 1)? };
        let mut minutes: Vec<Bar> = Vec::new();
        match zp.get(mi) {
            Some(1) => match open_month(ctx.store, Vendor::Zerodha, &sym.series, id, Timeframe::MINUTE_1, mi) {
                MonthFile::Open(f) => match read_minutes(&f, &mut out.counts.charter_minutes_withheld) {
                    Ok(v) => {
                        out.counts.z_months_read += 1;
                        minutes = v;
                    }
                    Err(why) => {
                        out.counts.z_months_refused += 1;
                        out.notes.push(format!("{} zerodha {y}-{m:02} read refused: {why}", sym.name));
                    }
                },
                MonthFile::Missing => out.counts.z_months_missing += 1,
                MonthFile::Refused(why) => {
                    out.counts.z_months_refused += 1;
                    out.notes.push(format!("{} zerodha {y}-{m:02} refused at load: {why}", sym.name));
                }
            },
            Some(2) => out.counts.z_months_refused += 1,
            _ => out.counts.z_months_missing += 1,
        }
        let groups = by_day(&minutes);
        let mut schedule = Schedule::default();
        for &(d, _, b) in &groups {
            if d >= ctx.cas_day
                && let Some(last) = b.checked_sub(1).and_then(|i| minutes.get(i))
            {
                let eligible = ist_min_of(last.ts_micros) <= CLOSE_CAS - 1;
                schedule
                    .insert(session_day(d)?, eligible)
                    .map_err(|e| format!("{} schedule {d}: {e}", sym.name))?;
            }
        }
        let mut derived: Vec<Vec<Bar>> = Vec::with_capacity(N_TF);
        for &k in &TFS {
            if minutes.is_empty() {
                derived.push(Vec::new());
                continue;
            }
            let secs = u32::try_from(60 * k).map_err(|_| "bucket".to_owned())?;
            let bucket = Bucket::of_secs(secs).ok_or_else(|| format!("bucket {secs} s refused"))?;
            match derive_minutes(&minutes, bucket, Venue::NseCash, Some(&schedule), Runtime::default()) {
                Ok(dv) => derived.push(dv.bars),
                Err(e) => {
                    out.counts.fold_errors += 1;
                    out.notes.push(format!("{} {y}-{m:02} {k}min fold refused: {e}", sym.name));
                    derived.push(Vec::new());
                }
            }
        }
        let day_bars = if minutes.is_empty() {
            Vec::new()
        } else {
            match derive_minutes(&minutes, Bucket::DAY, Venue::NseCash, Some(&schedule), Runtime::default()) {
                Ok(dv) => dv.bars,
                Err(e) => {
                    out.counts.fold_errors += 1;
                    out.notes.push(format!("{} {y}-{m:02} day fold refused: {e}", sym.name));
                    Vec::new()
                }
            }
        };
        let dgroups: Vec<Vec<(i64, usize, usize)>> = derived.iter().map(|v| by_day(v)).collect();
        let bgroups = by_day(&day_bars);
        let mut cur_min = 0_usize;
        let mut cur_tf = [0_usize; N_TF];
        let mut cur_day = 0_usize;
        let mut gm = GMonth::NotOpened;
        let mut d = first_d;
        while d < next_first && d <= ctx.win_last {
            let mins = day_slice(&minutes, &groups, &mut cur_min, d);
            let mut dbars: [&[Bar]; N_TF] = [&[]; N_TF];
            for ti in 0..N_TF {
                if let (Some(slot), Some(v), Some(g), Some(c)) =
                    (dbars.get_mut(ti), derived.get(ti), dgroups.get(ti), cur_tf.get_mut(ti))
                {
                    *slot = day_slice(v, g, c, d);
                }
            }
            let day_bar = day_slice(&day_bars, &bgroups, &mut cur_day, d).first().copied();
            process_day(ctx, sym, id, mi, &mut state, d, mins, &dbars, day_bar, trade_start, &mut gm, out)?;
            d += 1;
        }
    }
    Ok(())
}

/// One IST day of one stock: fold every timeframe, mark holes, decide, walk trades.
#[allow(clippy::too_many_arguments, clippy::too_many_lines, reason = "one day, every input named")]
fn process_day(
    ctx: &Ctx<'_>,
    sym: &Sym,
    id: u32,
    mi: usize,
    state: &mut StockState,
    d: i64,
    mins: &[Bar],
    dbars: &[&[Bar]; N_TF],
    day_bar: Option<Bar>,
    trade_start: i64,
    gm: &mut GMonth,
    out: &mut StockOut,
) -> Result<(), String> {
    let kind = Runtime::default().kind_of(d);
    let m2 = kind == DayKind::Open(Session::full());
    if !m2 {
        // Not a regular full session: its stored candles still fold; nothing is
        // decided; its record is never the previous session.
        for (ti, bars) in dbars.iter().enumerate() {
            let Some(s) = state.tf.get_mut(ti) else { continue };
            for b in *bars {
                if s.fold(&candle_of(b))? {
                    s.clean_step();
                } else {
                    s.hole();
                }
            }
        }
        return Ok(());
    }
    out.counts.m2_days += 1;
    let has_min = !mins.is_empty();
    if !has_min {
        out.counts.m2_days_no_minutes += 1;
    }
    let last_min = mins.last().map_or(CLOSE_FULL, |b| ist_min_of(b.ts_micros));
    let close_min = if d >= ctx.cas_day && has_min && last_min <= CLOSE_CAS - 1 { CLOSE_CAS } else { CLOSE_FULL };
    let n_min = usize::try_from(close_min - OPEN_MIN).map_err(|_| "minutes".to_owned())?;
    let mut zmin: Vec<Option<Bar>> = vec![None; n_min];
    for b in mins {
        let j = ist_min_of(b.ts_micros) - OPEN_MIN;
        if let Ok(j) = usize::try_from(j)
            && let Some(slot) = zmin.get_mut(j)
        {
            *slot = Some(*b);
        }
    }
    let in_span = d >= trade_start && d >= ctx.win_first && d <= ctx.win_last;
    let mut allowed = in_span && has_min && d != ctx.outage_day;
    if allowed && let Some(ms) = ctx.membership {
        let q = if d >= ctx.oct_first { ctx.carry_day } else { session_day(d)? };
        // Any of its spellings: the 218 name first, then the stored series (ICICILIFE
        // is stored as ICICIPRULI).
        let mut answer = ms.is_member(&sym.name, q);
        if sym.series != sym.name && !matches!(answer, Ok(true)) {
            let other = ms.is_member(&sym.series, q);
            if matches!(other, Ok(true)) || answer.is_err() {
                answer = other;
            }
        }
        match answer {
            Ok(true) => {}
            Ok(false) => {
                out.counts.member_false_days += 1;
                allowed = false;
            }
            Err(_) => {
                out.counts.member_unknown_days += 1;
                allowed = false;
            }
        }
    }
    let prev = match state.prev {
        Some(Ok(hlc)) => Some(hlc),
        Some(Err(())) => {
            if allowed {
                out.counts.prev_refused_days += 1;
            }
            None
        }
        None => {
            if allowed {
                out.counts.no_prev_days += 1;
            }
            None
        }
    };
    if allowed {
        out.counts.trade_days += 1;
    }
    let narrow = prev.is_some_and(|(h, l, c)| cpr_narrow(h, l, c, 20));
    let t_cut = at_min(d, CUT_MIN);
    let mut recs: Vec<Vec<Rec>> = Vec::with_capacity(N_TF);
    for (ti, &k) in TFS.iter().enumerate() {
        let Some(s) = state.tf.get_mut(ti) else { continue };
        let bars = dbars.get(ti).copied().unwrap_or(&[]);
        let mut cur = 0_usize;
        let span = close_min - OPEN_MIN;
        let nb = (span + k - 1).div_euclid(k);
        let mut v: Vec<Rec> = Vec::with_capacity(usize::try_from(nb).unwrap_or(0));
        for b in 0..nb {
            let sm = OPEN_MIN + b * k;
            let em = (sm + k).min(close_min);
            let t_open = at_min(d, sm);
            let t_close = at_min(d, em);
            while let Some(x) = bars.get(cur) {
                if x.ts_micros < t_open {
                    if let Some(u) = out.counts.unscheduled_bars.get_mut(ti) {
                        *u += 1;
                    }
                    cur += 1;
                } else {
                    break;
                }
            }
            let bar = match bars.get(cur) {
                Some(x) if x.ts_micros == t_open => {
                    cur += 1;
                    Some(*x)
                }
                _ => None,
            };
            let lo = usize::try_from(sm - OPEN_MIN).unwrap_or(0);
            let hi = usize::try_from(em - OPEN_MIN).unwrap_or(0);
            let cells = zmin.get(lo..hi).unwrap_or(&[]);
            let missing = cells.iter().any(Option::is_none);
            let mut rec = Rec { t: t_close, hole: true, o: 0, h: 0, l: 0, c: 0, st: [None, None], sig: 0 };
            match bar {
                None => {
                    if !missing && cells.iter().all(|x| x.is_some_and(|b| b.volume == 0)) {
                        // A bucket of only padded minutes has no bar, neither written
                        // nor withheld (fold.rs:1056-1059): no candle, no hole.
                        if let Some(p) = out.counts.padded_only.get_mut(ti) {
                            *p += 1;
                        }
                        continue;
                    }
                    s.hole();
                    if let Some(h) = out.counts.holes.get_mut(ti) {
                        *h += 1;
                    }
                    let which = if missing { &mut out.counts.holes_missing_minute } else { &mut out.counts.holes_withheld };
                    if let Some(h) = which.get_mut(ti) {
                        *h += 1;
                    }
                }
                Some(bar) => {
                    let c = candle_of(&bar);
                    if let Some(n) = out.counts.candles.get_mut(ti) {
                        *n += 1;
                    }
                    if !s.fold(&c)? {
                        s.hole();
                        if let Some(h) = out.counts.holes.get_mut(ti) {
                            *h += 1;
                        }
                        if let Some(h) = out.counts.holes_refused.get_mut(ti) {
                            *h += 1;
                        }
                    } else if missing {
                        // A hole candle that holds bars is still folded (its minutes are real trades).
                        s.hole();
                        if let Some(h) = out.counts.holes.get_mut(ti) {
                            *h += 1;
                        }
                        if let Some(h) = out.counts.holes_missing_minute.get_mut(ti) {
                            *h += 1;
                        }
                    } else {
                        s.clean_step();
                        rec = Rec {
                            t: t_close,
                            hole: false,
                            o: bar.open,
                            h: bar.high,
                            l: bar.low,
                            c: bar.close,
                            st: s.bands(),
                            sig: 0,
                        };
                    }
                }
            }
            if allowed && !rec.hole {
                let mut sig = 0_u8;
                for rs in 0..N_RS {
                    match decide(rs, s, rec.c, prev, narrow, &ctx.levels) {
                        Decision::Blocked(mask) => {
                            if t_close < t_cut {
                                if let Some(a) = out.counts.blocked_any.get_mut(rs).and_then(|x| x.get_mut(ti)) {
                                    *a += 1;
                                }
                                if let Some(row) = out.counts.blocked.get_mut(rs).and_then(|x| x.get_mut(ti)) {
                                    for (ri, cell) in row.iter_mut().enumerate() {
                                        if mask & (1 << ri) != 0 {
                                            *cell += 1;
                                        }
                                    }
                                }
                            }
                        }
                        Decision::Ready(Some(side)) => {
                            let table = if t_close < t_cut { &mut out.counts.signals } else { &mut out.counts.cut };
                            if let Some(cell) = table.get_mut(rs).and_then(|x| x.get_mut(ti)).and_then(|x| x.get_mut(side)) {
                                *cell += 1;
                            }
                            if t_close < t_cut {
                                sig |= 1 << (rs * 2 + side);
                            }
                        }
                        Decision::Ready(None) | Decision::NotReady => {}
                    }
                }
                rec.sig = sig;
            }
            v.push(rec);
        }
        if let Some(u) = out.counts.unscheduled_bars.get_mut(ti) {
            *u += u64::try_from(bars.len().saturating_sub(cur)).unwrap_or(0);
        }
        recs.push(v);
    }
    // The day's own bar becomes the next day's previous session (3.4).
    if let Some(b) = day_bar {
        state.prev = Some(
            DailyReference::new(candle_of(&b), DailyEligibility::Eligible)
                .map(|r| {
                    let bar = r.bar();
                    (i128::from(bar.high), i128::from(bar.low), i128::from(bar.close))
                })
                .map_err(|_| ()),
        );
    }
    if !recs.iter().any(|v| v.iter().any(|r| r.sig != 0)) {
        return Ok(());
    }
    out.counts.candidate_days += 1;
    // The day's fills: GDFL seconds, or Zerodha minutes on a fallback day.
    let mut fallback_why: Option<u8> = None;
    let mut secs: Vec<Tick> = Vec::new();
    let mut g_close: Vec<Option<i64>> = vec![None; n_min];
    if d == ctx.gdfl_gap_day {
        fallback_why = Some(0);
    } else {
        if matches!(gm, GMonth::NotOpened) {
            *gm = match open_month(ctx.store, Vendor::Gdfl, &sym.series, id, Timeframe::SECOND_1, mi) {
                MonthFile::Open(f) => GMonth::Open(f),
                MonthFile::Missing => GMonth::Missing,
                MonthFile::Refused(why) => {
                    out.notes.push(format!("{} gdfl month refused at fill time: {why}", sym.name));
                    GMonth::Refused
                }
            };
        }
        match gm {
            GMonth::Open(f) => {
                let read = (|| -> Result<(), String> {
                    let a = f.first_at_or_after(at_min(d, OPEN_MIN)).map_err(|e| e.to_string())?;
                    let b = f.first_at_or_after(at_min(d, close_min)).map_err(|e| e.to_string())?;
                    secs.reserve(usize::try_from(b.saturating_sub(a)).unwrap_or(0));
                    for i in a..b {
                        let r = f.read_record(i).map_err(|e| e.to_string())?;
                        secs.push(Tick { ts: r.ts_micros, fill: r.ts_micros, high: r.high, low: r.low });
                        let j = ist_min_of(r.ts_micros) - OPEN_MIN;
                        if let Ok(j) = usize::try_from(j)
                            && let Some(slot) = g_close.get_mut(j)
                        {
                            *slot = Some(r.close);
                        }
                    }
                    Ok(())
                })();
                if let Err(why) = read {
                    out.notes.push(format!("{} gdfl day {d} unreadable: {why}", sym.name));
                    secs.clear();
                    fallback_why = Some(3);
                } else if secs.is_empty() {
                    fallback_why = Some(2);
                }
            }
            GMonth::Missing | GMonth::Refused | GMonth::NotOpened => fallback_why = Some(1),
        }
    }
    let fallback = fallback_why.is_some();
    let (ticks, best): (Vec<Tick>, Option<Vec<Option<(i64, i64)>>>) = if fallback {
        out.counts.fallback_days += 1;
        match fallback_why {
            Some(0) => out.counts.fallback_gap_days += 1,
            Some(1) => out.counts.fallback_no_month_days += 1,
            Some(2) => out.counts.fallback_empty_days += 1,
            _ => out.counts.fallback_read_error_days += 1,
        }
        let mt: Vec<Tick> = zmin
            .iter()
            .flatten()
            .filter(|b| b.volume > 0)
            .map(|b| Tick { ts: b.ts_micros, fill: b.ts_micros + 59 * US_S, high: b.high, low: b.low })
            .collect();
        (mt, None)
    } else {
        let mut best: Vec<Option<(i64, i64)>> = Vec::with_capacity(n_min);
        let mut last: Option<(i64, i64)> = None;
        for (z, g) in zmin.iter().zip(g_close.iter()) {
            if let (Some(z), Some(g)) = (z, g)
                && z.volume > 0
            {
                last = Some((z.close, *g));
            }
            best.push(last);
        }
        (secs, Some(best))
    };
    let time_from = if fallback { at_min(d, CUT_MIN) } else { at_min(d, CUT_MIN) - US_S };
    for (ti, v) in recs.iter().enumerate() {
        for (i, r) in v.iter().enumerate() {
            if r.sig == 0 {
                continue;
            }
            for rs in 0..N_RS {
                for side in 0..2_usize {
                    if r.sig & (1 << (rs * 2 + side)) == 0 {
                        continue;
                    }
                    let st_idx = if rs == 0 { 1 } else { 0 };
                    let sim = simulate(&ticks, best.as_deref(), v, i, st_idx, side == 0, d, close_min, time_from);
                    let mut c = Cand {
                        t_signal: r.t,
                        stock: sym.rank,
                        rs: u8::try_from(rs).unwrap_or(u8::MAX),
                        tf: u8::try_from(ti).unwrap_or(u8::MAX),
                        side: u8::try_from(side).unwrap_or(u8::MAX),
                        status: sim.status,
                        exit_kind: sim.kind,
                        fallback,
                        late: sim.late,
                        entry_us: sim.entry_fill,
                        entry_px: sim.entry_px,
                        qty: sim.qty,
                        exit_us: sim.exit_fill,
                        exit_px: sim.exit_px,
                        gross: 0,
                        charges: 0,
                        net: 0,
                        stop0: sim.stop0,
                        sig: [r.o, r.h, r.l, r.c],
                    };
                    if c.status == Status::Filled {
                        let priced = price(&c)?;
                        c.gross = priced.gross().raw();
                        c.charges = priced.headline().raw();
                        c.net = priced.net().raw();
                    }
                    out.cands.push(c);
                }
            }
        }
    }
    Ok(())
}

/// Self-check (spec 5): one month of a stock's minutes folded by `derive_minutes`
/// gives 125 three-minute and 7 sixty-minute buckets per full session, each stamped
/// on the 09:15 grid.
pub(crate) fn grid_check(root: &Path, series: &str, y: u16, m: u8) -> Result<String, String> {
    let id = sym_id(series)?;
    let mi = crate::common::month_index(y, m).ok_or_else(|| format!("{y}-{m:02}"))?;
    let f = match open_month(root, Vendor::Zerodha, series, id, Timeframe::MINUTE_1, mi) {
        MonthFile::Open(f) => f,
        MonthFile::Missing => return Err(format!("{series} {y}-{m:02} is not in the store")),
        MonthFile::Refused(why) => return Err(why),
    };
    let mut withheld = 0_u64;
    let minutes = read_minutes(&f, &mut withheld)?;
    let groups = by_day(&minutes);
    let schedule = Schedule::default();
    let mut lines: Vec<String> = Vec::new();
    for (k, want) in [(3_i64, 125_usize), (60, 7)] {
        let secs = u32::try_from(60 * k).map_err(|_| "bucket".to_owned())?;
        let bucket = Bucket::of_secs(secs).ok_or_else(|| format!("bucket {secs} refused"))?;
        let dv = derive_minutes(&minutes, bucket, Venue::NseCash, Some(&schedule), Runtime::default())
            .map_err(|e| format!("{series} {y}-{m:02} {k}min fold refused: {e}"))?;
        let dg = by_day(&dv.bars);
        let mut full_days = 0_u32;
        for &(d, a, b) in &groups {
            if b - a != 375 || Runtime::default().kind_of(d) != DayKind::Open(Session::full()) {
                continue;
            }
            full_days += 1;
            let bars: &[Bar] = dg
                .iter()
                .find(|g| g.0 == d)
                .and_then(|g| dv.bars.get(g.1..g.2))
                .unwrap_or(&[]);
            if bars.len() != want {
                return Err(format!(
                    "{series} {y}-{m:02} day {d}: {} {k}-minute bars from derive_minutes, expected {want}",
                    bars.len()
                ));
            }
            for (i, bar) in bars.iter().enumerate() {
                let step = i64::try_from(i).map_err(|_| "index".to_owned())?;
                let expect = at_min(d, OPEN_MIN + step * k);
                if bar.ts_micros != expect {
                    return Err(format!(
                        "{series} {y}-{m:02} day {d}: {k}-minute bar {i} stamped {} not {expect}",
                        bar.ts_micros
                    ));
                }
            }
        }
        if full_days == 0 {
            return Err(format!("{series} {y}-{m:02}: no full session with all 375 minutes stored"));
        }
        lines.push(format!(
            "{series} {y}-{m:02}: {full_days} full sessions with all 375 minutes, each {want} {k}-minute buckets from derive_minutes on the 09:15 grid"
        ));
    }
    Ok(lines.join("; "))
}
