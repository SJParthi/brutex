//! Phase A of the preview search: one stock's candles, readings, the five cores on every
//! exit lane, the filter library, and the would-be trades of both traded sides, written
//! as spool records (SEARCH-SPEC 2, 3, 4, 6.1).
//!
//! Copied from the preview's phase A (T1 `crates/psrb_preview/src/phase_a.rs`) and
//! extended. The store reads, the candle fold, holes and taint, the day mask, the
//! previous session, the fills, the trade walk (`simulate`) and the costs are T1's code
//! unchanged, so the bare cores re-create the preview's rows (SEARCH-SPEC 2.3; the run
//! checks it against the preview's rows.tsv before writing any result).

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
use indicators::pattern::{Patterns, Thresholds};
use indicators::rsi::{self, Rsi};
use indicators::trend::{Atr, Ema, SCALE, Trend};
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
use vocab::ConditionMask;

use crate::common::{
    CLOSE_CAS, CLOSE_FULL, CUT_MIN, EXPOSURE_PAISA, ExitKind, FAST_OF, HALF_EXPOSURE_PAISA, LANE_FAST,
    LANE_ST, N_CORE, N_LANE, N_TF, OPEN_MIN, SQ_MIN, ST_PERIOD, Status, TFS, US_MIN, US_S, WIN_FIRST_MI,
    WIN_LAST_MI, at_min, civil, day_number, ist_day_of, ist_min_of, lanes_of, month_of_index, session_day,
};
use crate::filters::{FW, N_BASE, body_bucket, candle_code, set};
use crate::nifty::Nifty;
use crate::spool::{Record, Trade};

/// Every level and width the cores and filters read, built once.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Levels {
    rsi_hi: rsi::Level,
    rsi_lo: rsi::Level,
    rsi60: rsi::Level,
    rsi40: rsi::Level,
    rsi80: rsi::Level,
    rsi20: rsi::Level,
    wr_m85: williams_r::Level,
    wr_m15: williams_r::Level,
    wr_m50: williams_r::Level,
    cci_hi: cci::Level,
    cci_lo: cci::Level,
    cci_100: cci::Level,
    cci_m100: cci::Level,
    w2: bollinger::Width,
    w25: bollinger::Width,
    w3: bollinger::Width,
}

impl Levels {
    pub(crate) fn new() -> Result<Self, String> {
        let r = |p: u32| rsi::Level::from_ppm(p).map_err(|e| format!("RSI level {p} refused: {e:?}"));
        let w = |p: i32| williams_r::Level::from_ppm(p).map_err(|e| format!("W%R level {p} refused: {e:?}"));
        let c = |m: i32| cci::Level::from_milli(m).map_err(|e| format!("CCI level {m} refused: {e:?}"));
        let b = |m: u32| bollinger::Width::from_milli(m).map_err(|e| format!("BB width {m} refused: {e:?}"));
        Ok(Self {
            rsi_hi: r(700_000)?,
            rsi_lo: r(300_000)?,
            rsi60: r(600_000)?,
            rsi40: r(400_000)?,
            rsi80: r(800_000)?,
            rsi20: r(200_000)?,
            wr_m85: w(-850_000)?,
            wr_m15: w(-150_000)?,
            wr_m50: w(-500_000)?,
            cci_hi: c(200_000)?,
            cci_lo: c(-200_000)?,
            cci_100: c(100_000)?,
            cci_m100: c(-100_000)?,
            w2: b(2_000)?,
            w25: b(2_500)?,
            w3: b(3_000)?,
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
    pub(crate) nifty: &'a Nifty,
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
    /// Decision candles within 10 / 200 clean candles of a hole (the Supertrend's and
    /// EMA 200's taint windows).
    pub(crate) taint10: [u64; N_TF],
    pub(crate) taint200: [u64; N_TF],
    /// Decision candles (allowed day, not a hole, before 15:08).
    pub(crate) decisions: [u64; N_TF],
    /// `[tf][lane][core][side]`: core signal candles before 15:08 (the records' cores).
    pub(crate) signals: Vec<u64>,
    /// The same at or after 15:08:00 (never traded).
    pub(crate) cut: Vec<u64>,
    /// `[tf][lane]`: records written.
    pub(crate) records: [[u64; N_LANE]; N_TF],
    /// `[tf][lane][with, fade]`: filled trades stopped at once (the band already crossed).
    pub(crate) at_once: [[[u64; 2]; N_LANE]; N_TF],
    /// `[tf][lane][with, fade]`: trades filled.
    pub(crate) filled: [[[u64; 2]; N_LANE]; N_TF],
}

/// Index into `Counts::signals` and `Counts::cut`.
pub(crate) const fn sig_ix(ti: usize, lane: usize, core: usize, side: usize) -> usize {
    ((ti * N_LANE + lane) * N_CORE + core) * 2 + side
}

/// Length of `Counts::signals`.
pub(crate) const N_SIG: usize = N_TF * N_LANE * N_CORE * 2;

fn add_n<const N: usize>(a: &mut [u64; N], b: &[u64; N]) {
    for (x, y) in a.iter_mut().zip(b.iter()) {
        *x += *y;
    }
}

impl Counts {
    pub(crate) fn new() -> Self {
        Self { signals: vec![0; N_SIG], cut: vec![0; N_SIG], ..Self::default() }
    }

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
        add_n(&mut self.candles, &o.candles);
        add_n(&mut self.holes, &o.holes);
        add_n(&mut self.holes_missing_minute, &o.holes_missing_minute);
        add_n(&mut self.holes_withheld, &o.holes_withheld);
        add_n(&mut self.holes_refused, &o.holes_refused);
        add_n(&mut self.padded_only, &o.padded_only);
        add_n(&mut self.unscheduled_bars, &o.unscheduled_bars);
        add_n(&mut self.taint10, &o.taint10);
        add_n(&mut self.taint200, &o.taint200);
        add_n(&mut self.decisions, &o.decisions);
        if self.signals.len() < N_SIG {
            self.signals.resize(N_SIG, 0);
        }
        if self.cut.len() < N_SIG {
            self.cut.resize(N_SIG, 0);
        }
        for (x, y) in self.signals.iter_mut().zip(o.signals.iter()) {
            *x += *y;
        }
        for (x, y) in self.cut.iter_mut().zip(o.cut.iter()) {
            *x += *y;
        }
        for ti in 0..N_TF {
            for lane in 0..N_LANE {
                if let (Some(a), Some(b)) = (
                    self.records.get_mut(ti).and_then(|x| x.get_mut(lane)),
                    o.records.get(ti).and_then(|x| x.get(lane)),
                ) {
                    *a += *b;
                }
                for k in 0..2 {
                    if let (Some(a), Some(b)) = (
                        self.at_once.get_mut(ti).and_then(|x| x.get_mut(lane)).and_then(|x| x.get_mut(k)),
                        o.at_once.get(ti).and_then(|x| x.get(lane)).and_then(|x| x.get(k)),
                    ) {
                        *a += *b;
                    }
                    if let (Some(a), Some(b)) = (
                        self.filled.get_mut(ti).and_then(|x| x.get_mut(lane)).and_then(|x| x.get_mut(k)),
                        o.filled.get(ti).and_then(|x| x.get(lane)).and_then(|x| x.get(k)),
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
    /// Records per `ti * N_LANE + lane`, in decision order.
    pub(crate) recs: Vec<Vec<Record>>,
    pub(crate) counts: Counts,
    pub(crate) notes: Vec<String>,
    /// A refusal that stops the run by name (a `price_trade` refusal, a torn reading).
    pub(crate) fatal: Option<String>,
    /// The stock's trading span and warm-up start, for the notes.
    pub(crate) span: String,
}

// ---------------------------------------------------------------------------
// Readings this tool writes itself (SEARCH-SPEC 3.7), integer, deterministic.
// ---------------------------------------------------------------------------

/// MACD (12, 26, 9): line from trend.rs's EMAs, signal an i128 EMA of the line.
#[derive(Clone, Copy, Debug, Default)]
struct Macd {
    m_count: u32,
    seed: i128,
    g: i128,
    m: i128,
    h: Option<i128>,
    h_prev: Option<i128>,
}

impl Macd {
    fn fold(&mut self, e12: &Ema, e26: &Ema) {
        self.h_prev = self.h;
        if !e26.warm() {
            self.h = None;
            return;
        }
        let m = e12.scaled() - e26.scaled();
        self.m = m;
        self.m_count = self.m_count.saturating_add(1);
        if self.m_count <= 9 {
            self.seed += m;
            if self.m_count == 9 {
                self.g = self.seed.div_euclid(9);
                self.h = Some(m - self.g);
            } else {
                self.h = None;
            }
        } else {
            self.g += (2 * (m - self.g)).div_euclid(10);
            self.h = Some(m - self.g);
        }
    }
}

/// Wilder's DMI / ADX 14, smoothed values scaled by 1,000,000.
#[derive(Clone, Copy, Debug, Default)]
struct Dmi {
    prev: Option<(i64, i64)>,
    n: u32,
    sum_p: i128,
    sum_m: i128,
    p: i128,
    m: i128,
    adx_n: u32,
    adx_sum: i128,
    adx: i128,
    ready: bool,
}

impl Dmi {
    fn fold(&mut self, h: i64, l: i64) {
        if let Some((ph, pl)) = self.prev {
            let up = i128::from(h) - i128::from(ph);
            let dn = i128::from(pl) - i128::from(l);
            let pdm = if up > dn && up > 0 { up } else { 0 };
            let mdm = if dn > up && dn > 0 { dn } else { 0 };
            self.n = self.n.saturating_add(1);
            if self.n <= 14 {
                self.sum_p += pdm * 1_000_000;
                self.sum_m += mdm * 1_000_000;
                if self.n == 14 {
                    self.p = self.sum_p.div_euclid(14);
                    self.m = self.sum_m.div_euclid(14);
                }
            } else {
                self.p += (pdm * 1_000_000 - self.p).div_euclid(14);
                self.m += (mdm * 1_000_000 - self.m).div_euclid(14);
            }
            if self.n >= 14 {
                let s = self.p + self.m;
                let dx = if s == 0 { 0 } else { (1_000_000 * (self.p - self.m).abs()).div_euclid(s) };
                self.adx_n = self.adx_n.saturating_add(1);
                if self.adx_n <= 14 {
                    self.adx_sum += dx;
                    if self.adx_n == 14 {
                        self.adx = self.adx_sum.div_euclid(14);
                        self.ready = true;
                    }
                } else {
                    self.adx += (dx - self.adx).div_euclid(14);
                }
            }
        }
        self.prev = Some((h, l));
    }
}

/// One closed candle of the stock's own candle order (or a hole), for "the last n
/// candles" windows (SEARCH-SPEC 3.1).
#[derive(Clone, Copy, Debug, Default)]
struct Slot {
    hole: bool,
    seq: u64,
    o: i64,
    h: i64,
    l: i64,
    c: i64,
    v: i64,
    /// Bollinger 20's variance numerator, warm and untainted.
    bbv: Option<i128>,
    /// EMA 50 / EMA 200 scaled, warm and untainted.
    e50: Option<i128>,
    e200: Option<i128>,
    obv: Option<i128>,
}

const RING: usize = 128;

/// The last `RING` slots, newest last.
#[derive(Clone, Debug)]
struct Ring {
    buf: Vec<Slot>,
    head: usize,
    len: usize,
}

impl Ring {
    fn new() -> Self {
        Self { buf: vec![Slot::default(); RING], head: 0, len: 0 }
    }

    fn push(&mut self, s: Slot) {
        if let Some(x) = self.buf.get_mut(self.head) {
            *x = s;
        }
        self.head = (self.head + 1) % RING;
        if self.len < RING {
            self.len += 1;
        }
    }

    /// The slot `j` back from the newest (0 = the newest).
    fn back(&self, j: usize) -> Option<&Slot> {
        if j >= self.len {
            return None;
        }
        self.buf.get((self.head + RING - 1 - j) % RING)
    }

    /// Slots `from..=to` back, all present and not holes.
    fn clean(&self, from: usize, to: usize) -> Option<Vec<&Slot>> {
        let mut v = Vec::with_capacity(to + 1 - from);
        for j in from..=to {
            let s = self.back(j)?;
            if s.hole {
                return None;
            }
            v.push(s);
        }
        Some(v)
    }
}

/// A candle of today's session as the bucket schedule saw it.
#[derive(Clone, Copy, Debug)]
struct SessBar {
    o: i64,
    h: i64,
    l: i64,
    c: i64,
    hole: bool,
    atr: Option<i64>,
}

#[derive(Clone, Copy, Debug)]
struct SwingPt {
    v: i64,
    seq: u64,
}

/// Fractal swings of 2 and 3 (index 0 and 1): the last two highs and lows, older first.
#[derive(Clone, Copy, Debug, Default)]
struct Swings {
    hi: [[Option<SwingPt>; 2]; 2],
    lo: [[Option<SwingPt>; 2]; 2],
}

/// The Fibonacci retracement state of one side on the last completed fractal-3 swing.
#[derive(Clone, Copy, Debug, Default)]
struct FibSwing {
    /// (origin, extreme): long = (swing low, swing high), short = (swing high, swing low).
    swing: Option<(i64, i64)>,
    from_seq: u64,
    cnt: [u8; 4],
}

/// The four retracement levels in thousandths.
const FIB_R: [i128; 4] = [382, 500, 618, 786];

/// One timeframe's readings, folded continuously from the stock's first candle.
struct TfState {
    st2: DualBandSuperTrend,
    st3: DualBandSuperTrend,
    st7: DualBandSuperTrend,
    bb: Bollinger,
    rsi: Rsi,
    wr: WilliamsR,
    cci: Cci,
    ema: Ema,
    ema50: Ema,
    ema12: Ema,
    ema26: Ema,
    macd: Macd,
    dmi: Dmi,
    atr: Atr,
    pats: Patterns,
    pmask: ConditionMask,
    /// Clean candles folded since the last hole (`u32::MAX` before any hole): T1's
    /// six equal counters, kept as one.
    since: u32,
    ring: Ring,
    seq: u64,
    obv: i128,
    obv_last: Option<i64>,
    /// Stochastic %K of the last two candles (None after a hole or not ready).
    k_hist: [Option<i128>; 2],
    k_now: Option<i128>,
    sw: Swings,
    fib: [FibSwing; 2],
    /// This candle's Fibonacci bits: `[side]`, bit `2 x level + touch`.
    fib_bits: [u8; 2],
    pd_cnt: [[u8; 4]; 2],
    pd_bits: [u8; 2],
    sess: Vec<Option<SessBar>>,
    above_ph: bool,
    below_pl: bool,
}

impl TfState {
    fn new() -> Result<Self, String> {
        Ok(Self {
            st2: DualBandSuperTrend::new(10, 2_000).map_err(|e| format!("ST 10/2: {e:?}"))?,
            st3: DualBandSuperTrend::new(10, 3_000).map_err(|e| format!("ST 10/3: {e:?}"))?,
            st7: DualBandSuperTrend::new(7, 1_500).map_err(|e| format!("ST 7/1.5: {e:?}"))?,
            bb: Bollinger::new(20).map_err(|e| format!("BB 20: {e:?}"))?,
            rsi: Rsi::new(14).map_err(|e| format!("RSI 14: {e:?}"))?,
            wr: WilliamsR::new(14).map_err(|e| format!("W%R 14: {e:?}"))?,
            cci: Cci::new(20).map_err(|e| format!("CCI 20: {e:?}"))?,
            ema: Ema::new(200),
            ema50: Ema::new(50),
            ema12: Ema::new(12),
            ema26: Ema::new(26),
            macd: Macd::default(),
            dmi: Dmi::default(),
            atr: Atr::new(14),
            pats: Patterns::new(Thresholds::CLASSICAL),
            pmask: ConditionMask::ZERO,
            since: u32::MAX,
            ring: Ring::new(),
            seq: 0,
            obv: 0,
            obv_last: None,
            k_hist: [None, None],
            k_now: None,
            sw: Swings::default(),
            fib: [FibSwing::default(); 2],
            fib_bits: [0, 0],
            pd_cnt: [[0; 4]; 2],
            pd_bits: [0, 0],
            sess: Vec::new(),
            above_ph: false,
            below_pl: false,
        })
    }

    /// Fold a candle into every reading or into none (AD 3): `Ok(false)` when a
    /// reading's check refuses it (folded into none, a hole). T1's fold plus the new
    /// readings, in the same branch.
    fn fold(&mut self, c: &Candle) -> Result<bool, String> {
        if c.check_evaluable().is_err() || bollinger::Close::new(c.close).is_err() {
            return Ok(false);
        }
        self.st2.fold(c).map_err(|e| format!("ST 10/2 refused a checked candle: {e:?}"))?;
        self.st3.fold(c).map_err(|e| format!("ST 10/3 refused a checked candle: {e:?}"))?;
        self.st7.fold(c).map_err(|e| format!("ST 7/1.5 refused a checked candle: {e:?}"))?;
        self.bb.fold(c).map_err(|e| format!("BB refused a checked candle: {e:?}"))?;
        self.rsi.fold(c).map_err(|e| format!("RSI refused a checked candle: {e:?}"))?;
        self.wr.fold(c).map_err(|e| format!("W%R refused a checked candle: {e:?}"))?;
        self.cci.fold(c).map_err(|e| format!("CCI refused a checked candle: {e:?}"))?;
        self.ema.fold(c.close);
        self.ema50.fold(c.close);
        self.ema12.fold(c.close);
        self.ema26.fold(c.close);
        self.macd.fold(&self.ema12, &self.ema26);
        self.dmi.fold(c.high, c.low);
        self.atr.fold(c);
        self.pmask = self.pats.step(c).map_err(|e| format!("patterns refused a checked candle: {e:?}"))?;
        Ok(true)
    }

    fn hole(&mut self) {
        self.since = 0;
    }

    fn clean_step(&mut self) {
        self.since = self.since.saturating_add(1);
    }

    /// A new session starts: the session-scoped state resets.
    fn new_session(&mut self) {
        self.sess.clear();
        self.above_ph = false;
        self.below_pl = false;
        self.pd_cnt = [[0; 4]; 2];
        self.pd_bits = [0, 0];
    }

    /// (lower, upper) of ST (10, 2), (10, 3), (7, 1.5) after the last fold, settled only.
    fn bands(&self) -> [Option<(i128, i128)>; 3] {
        let pick = |r: Option<indicators::dual_band_supertrend::Reading>| {
            r.filter(|x| x.settled).map(|x| (x.lower, x.upper))
        };
        [pick(self.st2.reading()), pick(self.st3.reading()), pick(self.st7.reading())]
    }

    /// The side each settled Supertrend reads: 0 long (Up), 1 short (Down).
    fn sides(&self) -> [Option<u8>; 3] {
        let pick = |r: Option<indicators::dual_band_supertrend::Reading>| {
            r.filter(|x| x.settled).map(|x| if x.trend == Trend::Up { 0 } else { 1 })
        };
        [pick(self.st2.reading()), pick(self.st3.reading()), pick(self.st7.reading())]
    }

    /// After a hole: a hole slot, no session bar, the Stochastic history broken.
    fn after_hole(&mut self, folded: Option<&Candle>) {
        self.seq += 1;
        self.ring.push(Slot { hole: true, seq: self.seq, ..Slot::default() });
        self.k_hist = [None, None];
        self.k_now = None;
        self.fib_bits = [0, 0];
        self.pd_bits = [0, 0];
        if let Some(c) = folded {
            self.sess.push(Some(SessBar { o: c.open, h: c.high, l: c.low, c: c.close, hole: true, atr: None }));
        } else {
            self.sess.push(None);
        }
    }

    /// After a clean candle: its slot, OBV, Stochastic, swings, Fibonacci touches.
    fn after_clean(&mut self, c: &Candle, prev: Option<(i128, i128, i128)>) {
        self.seq += 1;
        if let Some(pc) = self.obv_last {
            if c.close > pc {
                self.obv += i128::from(c.volume);
            } else if c.close < pc {
                self.obv -= i128::from(c.volume);
            }
        }
        self.obv_last = Some(c.close);
        let bbv = if self.since >= 20 { self.bb.window().map(|w| w.variance_numerator()) } else { None };
        let e50 = (self.ema50.warm() && self.since >= 50).then(|| self.ema50.scaled());
        let e200 = (self.ema.warm() && self.since >= 200).then(|| self.ema.scaled());
        self.ring.push(Slot {
            hole: false,
            seq: self.seq,
            o: c.open,
            h: c.high,
            l: c.low,
            c: c.close,
            v: c.volume,
            bbv,
            e50,
            e200,
            obv: Some(self.obv),
        });
        let atr = (self.atr.warm() && self.since >= 14).then(|| self.atr.value()).flatten();
        self.sess.push(Some(SessBar { o: c.open, h: c.high, l: c.low, c: c.close, hole: false, atr }));
        // Stochastic (14, 3) on t-13..=t, none a hole.
        let k = self.ring.clean(0, 13).and_then(|w| {
            let hh = w.iter().map(|s| s.h).max()?;
            let ll = w.iter().map(|s| s.l).min()?;
            (hh > ll).then(|| (1_000_000 * (i128::from(c.close) - i128::from(ll))).div_euclid(i128::from(hh) - i128::from(ll)))
        });
        self.k_hist = [self.k_now, self.k_hist[0]];
        self.k_now = k;
        self.swings_update();
        self.fib_update(c);
        self.pd_update(c, prev);
    }

    fn swings_update(&mut self) {
        for (fi, n) in [2_usize, 3].into_iter().enumerate() {
            let Some(w) = self.ring.clean(0, 2 * n) else { continue };
            let Some(mid) = w.get(n) else { continue };
            let is_hi = w.iter().enumerate().all(|(j, s)| j == n || s.h < mid.h);
            let is_lo = w.iter().enumerate().all(|(j, s)| j == n || s.l > mid.l);
            let pt_h = SwingPt { v: mid.h, seq: mid.seq };
            let pt_l = SwingPt { v: mid.l, seq: mid.seq };
            if is_hi && let Some(h) = self.sw.hi.get_mut(fi) {
                *h = [h[1], Some(pt_h)];
                if fi == 1 {
                    // A new up-swing: the latest confirmed swing low before it.
                    let low = self.sw.lo.get(1).and_then(|x| x[1]);
                    if let (Some(fl), Some(low)) = (self.fib.get_mut(0), low)
                        && low.seq < pt_h.seq
                    {
                        *fl = FibSwing { swing: Some((low.v, pt_h.v)), from_seq: self.seq, cnt: [0; 4] };
                    }
                }
            }
            if is_lo && let Some(l) = self.sw.lo.get_mut(fi) {
                *l = [l[1], Some(pt_l)];
                if fi == 1 {
                    let high = self.sw.hi.get(1).and_then(|x| x[1]);
                    if let (Some(fs), Some(high)) = (self.fib.get_mut(1), high)
                        && high.seq < pt_l.seq
                    {
                        *fs = FibSwing { swing: Some((high.v, pt_l.v)), from_seq: self.seq, cnt: [0; 4] };
                    }
                }
            }
        }
    }

    fn fib_update(&mut self, c: &Candle) {
        let (h, l, cl) = (i128::from(c.high), i128::from(c.low), i128::from(c.close));
        let seq = self.seq;
        for side in 0..2 {
            let mut bits = 0_u8;
            if let Some(f) = self.fib.get_mut(side)
                && let Some((a, b)) = f.swing
                && seq > f.from_seq
            {
                let (a, b) = (i128::from(a), i128::from(b));
                for (li, r) in FIB_R.iter().enumerate() {
                    let touched = if side == 0 {
                        // up-swing a (low) -> b (high): level b - r (b - a)
                        let lvl = 1000 * b - r * (b - a);
                        1000 * l <= lvl && 1000 * cl > lvl
                    } else {
                        // down-swing a (high) -> b (low): level b + r (a - b)
                        let lvl = 1000 * b + r * (a - b);
                        1000 * h >= lvl && 1000 * cl < lvl
                    };
                    if touched && let Some(n) = f.cnt.get_mut(li) {
                        if *n == 0 {
                            bits |= 1 << (2 * li);
                        } else if *n == 1 {
                            bits |= 1 << (2 * li + 1);
                        }
                        *n = n.saturating_add(1);
                    }
                }
            }
            if let Some(x) = self.fib_bits.get_mut(side) {
                *x = bits;
            }
        }
    }

    fn pd_update(&mut self, c: &Candle, prev: Option<(i128, i128, i128)>) {
        self.pd_bits = [0, 0];
        let Some((ph, pl, _)) = prev else { return };
        if ph <= pl {
            return;
        }
        let (h, l, cl) = (i128::from(c.high), i128::from(c.low), i128::from(c.close));
        for side in 0..2_usize {
            let mut bits = 0_u8;
            for (li, r) in FIB_R.iter().enumerate() {
                let touched = if side == 0 {
                    let lvl = 1000 * ph - r * (ph - pl);
                    1000 * l <= lvl && 1000 * cl > lvl
                } else {
                    let lvl = 1000 * pl + r * (ph - pl);
                    1000 * h >= lvl && 1000 * cl < lvl
                };
                if touched && let Some(n) = self.pd_cnt.get_mut(side).and_then(|x| x.get_mut(li)) {
                    if *n == 0 {
                        bits |= 1 << (2 * li);
                    } else if *n == 1 {
                        bits |= 1 << (2 * li + 1);
                    }
                    *n = n.saturating_add(1);
                }
            }
            if let Some(x) = self.pd_bits.get_mut(side) {
                *x = bits;
            }
        }
    }
}

/// A candle of one timeframe on one day, as the trade walk reads it (T1's, with the
/// third Supertrend, the sides and the clean count).
#[derive(Clone, Copy, Debug)]
struct Rec {
    /// Close instant T.
    t: i64,
    hole: bool,
    o: i64,
    h: i64,
    l: i64,
    c: i64,
    /// (lower, upper) of ST (10, 2) [0], (10, 3) [1], (7, 1.5) [2], settled.
    st: [Option<(i128, i128)>; 3],
    side: [Option<u8>; 3],
    clean: u32,
}

/// C1 at `bp` basis points: `10_000 x |2C - H - L| <= bp x (H + L + C)`, ties narrow.
pub(crate) fn cpr_narrow(h: i128, l: i128, c: i128, bp: i128) -> bool {
    10_000 * (2 * c - h - l).abs() <= bp * (h + l + c)
}

fn candle_of(b: &Bar) -> Candle {
    Candle::new(b.ts_micros, b.open, b.high, b.low, b.close, b.volume, b.open_interest)
}

pub(crate) fn sym_id(series: &str) -> Result<u32, String> {
    u32::try_from(brutex_core::universe::fnv1a(series) & 0xFFFF_FFFF).map_err(|_| format!("{series}: symbol id"))
}

pub(crate) enum MonthFile {
    Missing,
    Refused(String),
    Open(BarFile),
}

pub(crate) fn open_month(
    root: &Path,
    vendor: Vendor,
    segment: &str,
    series: &str,
    id: u32,
    tf: Timeframe,
    mi: usize,
) -> MonthFile {
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
        segment,
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
pub(crate) fn read_minutes(f: &BarFile, withheld: &mut u64) -> Result<Vec<Bar>, String> {
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
pub(crate) fn by_day(bars: &[Bar]) -> Vec<(i64, usize, usize)> {
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
        let w = if long { rest.iter().map(|x| x.low).min() } else { rest.iter().map(|x| x.high).max() };
        sim.kind = ExitKind::Unclosable;
        sim.exit_px = w.unwrap_or(sim.entry_px);
        sim.exit_fill = end_fill;
    }
    sim
}

/// 3.5: the entry, the trailed resting stop, the flip, the time exit and the square-off.
/// T1's walk unchanged; `recs` is the exit lane's timeframe, `i` the lane candle that
/// closes at the signal instant, `st_idx` the lane's Supertrend, `long` the TRADED side.
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

/// Prices one trade by `costs::equity_clock::price_trade` (PS 3.7): T1's `price` with
/// the quantity named (SIZE AFTER A FALL re-prices the halved quantity at the same fills).
#[allow(clippy::too_many_arguments, reason = "every fill input named")]
pub(crate) fn price_fills(
    long: bool,
    entry_us: i64,
    entry_px: i64,
    exit_us: i64,
    exit_px: i64,
    qty: i64,
    kind: ExitKind,
    t_signal: i64,
) -> Result<TwoClock, String> {
    let (es, xs) = if long { (OrderSide::Buy, OrderSide::Sell) } else { (OrderSide::Sell, OrderSide::Buy) };
    let by_broker = matches!(kind, ExitKind::SquareOff | ExitKind::Unclosable);
    let fills = TradeFills {
        orders: vec![
            Order {
                side: es,
                by_broker: false,
                fills: vec![Fill { second: entry_us.div_euclid(US_S), quantity: qty, price: Paisa::from_raw(entry_px) }],
            },
            Order {
                side: xs,
                by_broker,
                fills: vec![Fill { second: exit_us.div_euclid(US_S), quantity: qty, price: Paisa::from_raw(exit_px) }],
            },
        ],
    };
    let (y, m, dd) = civil(ist_day_of(t_signal))?;
    let day = TradeDay::new(y, m, dd).map_err(|e| format!("trade day {y}-{m:02}-{dd:02}: {e:?}"))?;
    price_trade(&fills, day).map_err(|e| format!("price_trade refused {y}-{m:02}-{dd:02} entry {qty} x {entry_px} exit {exit_px}: {e}"))
}

/// A walked trade, priced at full and at half exposure.
fn make_trade(sim: &Sim, long: bool, fallback: bool, t_signal: i64) -> Result<Trade, String> {
    let mut tr = Trade {
        status: sim.status,
        kind: sim.kind,
        fallback,
        late: sim.late,
        entry_us: sim.entry_fill,
        entry_px: sim.entry_px,
        qty: sim.qty,
        exit_us: sim.exit_fill,
        exit_px: sim.exit_px,
        stop0: sim.stop0,
        ..Trade::default()
    };
    if sim.status != Status::Filled {
        return Ok(tr);
    }
    tr.at_once = sim.kind == ExitKind::Stop
        && ((long && sim.entry_px <= sim.stop0) || (!long && sim.entry_px >= sim.stop0));
    let p = price_fills(long, sim.entry_fill, sim.entry_px, sim.exit_fill, sim.exit_px, sim.qty, sim.kind, t_signal)?;
    tr.gross = p.gross().raw();
    tr.charges = p.headline().raw();
    tr.net = p.net().raw();
    let qh = if sim.entry_px > 0 { HALF_EXPOSURE_PAISA.div_euclid(sim.entry_px) } else { 0 };
    if qh > 0 {
        let p = price_fills(long, sim.entry_fill, sim.entry_px, sim.exit_fill, sim.exit_px, qh, sim.kind, t_signal)?;
        tr.half_ok = true;
        tr.gross_h = p.gross().raw();
        tr.charges_h = p.headline().raw();
        tr.net_h = p.net().raw();
    }
    Ok(tr)
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
    /// ATR 14 on the accepted previous-session day bars (A18, A19).
    datr: Atr,
    /// The last 10 M2 sessions with stored minutes: cumulative volume by minute (A29).
    rvol: Vec<Vec<i128>>,
}

/// Phase A for one stock.
pub(crate) fn run_stock(ctx: &Ctx<'_>, sym: &Sym) -> StockOut {
    let mut out = StockOut { recs: (0..N_TF * N_LANE).map(|_| Vec::new()).collect(), counts: Counts::new(), ..StockOut::default() };
    if let Err(e) = stock_inner(ctx, sym, &mut out) {
        out.fatal = Some(format!("{}: {e}", sym.name));
    }
    out
}

#[allow(clippy::too_many_lines, reason = "one pass over one stock months")]
fn stock_inner(ctx: &Ctx<'_>, sym: &Sym, out: &mut StockOut) -> Result<(), String> {
    let id = sym_id(&sym.series)?;
    let n_mi = WIN_LAST_MI + 1;
    let mut zp = vec![0_u8; n_mi];
    for (mi, slot) in zp.iter_mut().enumerate() {
        *slot = match open_month(ctx.store, Vendor::Zerodha, "CASH", &sym.series, id, Timeframe::MINUTE_1, mi) {
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
        *slot = match open_month(ctx.store, Vendor::Gdfl, "CASH", &sym.series, id, Timeframe::SECOND_1, mi) {
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
        datr: Atr::new(14),
        rvol: Vec::new(),
    };
    for mi in warm..n_mi {
        let (y, m) = month_of_index(mi)?;
        let first_d = day_number(y, m, 1)?;
        let next_first = if m == 12 { day_number(y + 1, 1, 1)? } else { day_number(y, m + 1, 1)? };
        let mut minutes: Vec<Bar> = Vec::new();
        match zp.get(mi) {
            Some(1) => match open_month(ctx.store, Vendor::Zerodha, "CASH", &sym.series, id, Timeframe::MINUTE_1, mi) {
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
                schedule.insert(session_day(d)?, eligible).map_err(|e| format!("{} schedule {d}: {e}", sym.name))?;
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

/// The day's minute arrays (prefixes over the minutes that START before a minute index).
struct DayCx {
    prev: Option<(i128, i128, i128)>,
    /// The open of today's first stored minute.
    day_open: Option<i64>,
    /// `pv[k]`, `sv[k]`: VWAP sums over the stored minutes with volume > 0 among the first k.
    pv: Vec<i128>,
    sv: Vec<i128>,
    /// Highest high and lowest low of the stored minutes among the first k.
    dh: Vec<Option<i64>>,
    dl: Vec<Option<i64>>,
    /// Cumulative volume of the stored minutes among the first k.
    cv: Vec<i128>,
    or15: Option<(i64, i64)>,
    or30: Option<(i64, i64)>,
    datr: Option<i64>,
    weekday: i64,
    /// NIFTY: the latest stored minute close among the first k minutes, and the previous M2 close.
    n_last: Vec<Option<i64>>,
    n_prev: Option<i64>,
    /// A29: R(k), the sum of the cumulative volume over the last 10 earlier M2 sessions
    /// at minute k (empty with fewer than 10 sessions).
    rv: Vec<i128>,
}

fn day_cx(state: &StockState, prev: Option<(i128, i128, i128)>, zmin: &[Option<Bar>], nifty: &Nifty, d: i64) -> DayCx {
    let n = zmin.len();
    let mut cx = DayCx {
        prev,
        day_open: zmin.iter().flatten().next().map(|b| b.open),
        pv: Vec::with_capacity(n + 1),
        sv: Vec::with_capacity(n + 1),
        dh: Vec::with_capacity(n + 1),
        dl: Vec::with_capacity(n + 1),
        cv: Vec::with_capacity(n + 1),
        or15: None,
        or30: None,
        datr: state.datr.warm().then(|| state.datr.value()).flatten(),
        weekday: (d + 3).rem_euclid(7),
        n_last: Vec::new(),
        n_prev: nifty.prev_close(d),
        rv: Vec::new(),
    };
    if state.rvol.len() >= 10 {
        cx.rv = (0..=n)
            .map(|k| {
                state
                    .rvol
                    .iter()
                    .rev()
                    .take(10)
                    .map(|h| h.get(k).copied().unwrap_or_else(|| h.last().copied().unwrap_or(0)))
                    .sum()
            })
            .collect();
    }
    let (mut pv, mut sv, mut cv) = (0_i128, 0_i128, 0_i128);
    let (mut dh, mut dl): (Option<i64>, Option<i64>) = (None, None);
    cx.pv.push(0);
    cx.sv.push(0);
    cx.cv.push(0);
    cx.dh.push(None);
    cx.dl.push(None);
    for (j, z) in zmin.iter().enumerate() {
        if let Some(b) = z {
            if b.volume > 0 {
                pv += (i128::from(b.high) + i128::from(b.low) + i128::from(b.close)) * i128::from(b.volume);
                sv += i128::from(b.volume);
            }
            cv += i128::from(b.volume);
            dh = Some(dh.map_or(b.high, |x| x.max(b.high)));
            dl = Some(dl.map_or(b.low, |x| x.min(b.low)));
        }
        cx.pv.push(pv);
        cx.sv.push(sv);
        cx.cv.push(cv);
        cx.dh.push(dh);
        cx.dl.push(dl);
        if j == 14 {
            cx.or15 = dh.zip(dl);
        }
        if j == 29 {
            cx.or30 = dh.zip(dl);
        }
    }
    cx.n_last = nifty.last_closes(d);
    cx
}

/// One record waiting for its trades.
struct Pending {
    ti: usize,
    lane: usize,
    lane_tf: usize,
    lane_idx: usize,
    side: u8,
    cores: u8,
    fbits: [u64; FW],
    codes: [u8; 3],
    strength: i64,
    t: i64,
    sig: [i64; 4],
}

/// The cores' conditions on the closed candle without the Supertrend side (that is the
/// lane's): bit k = core K(k+1), for [long, short]. T1's `decide` split by core.
fn core_base(s: &TfState, close: i64, prev: Option<(i128, i128, i128)>, narrow: bool, lv: &Levels, fresh: [bool; 2]) -> [u8; 2] {
    let mut out = [0_u8; 2];
    let Ok(cb) = bollinger::Close::new(close) else { return out };
    let since = s.since;
    let w = if since >= 20 { s.bb.window() } else { None };
    let r = if since >= 14 { s.rsi.reading().filter(rsi::Reading::faithful) } else { None };
    let wr = if since >= 14 { s.wr.reading().filter(|x| !x.flat()) } else { None };
    let cc = if since >= 20 { s.cci.reading().filter(|x| !x.flat()) } else { None };
    let ema_ok = s.ema.warm() && since >= 200;
    let c = i128::from(close);
    let c2 = prev.map(|(h, l, pc)| {
        let t3 = h + l + pc;
        [3 * c > 2 * t3 - 3 * l, 3 * c < 2 * t3 - 3 * h]
    });
    for side in 0..2_usize {
        let long = side == 0;
        let mut bits = 0_u8;
        let bb2 = w.is_some_and(|w| if long { w.above_upper(cb, lv.w2) } else { w.below_lower(cb, lv.w2) });
        let bb3 = w.is_some_and(|w| if long { w.above_upper(cb, lv.w3) } else { w.below_lower(cb, lv.w3) });
        let rs = r.is_some_and(|r| if long { r.above(lv.rsi_hi) } else { r.below(lv.rsi_lo) });
        let c2s = c2.is_some_and(|x| x.get(side).copied().unwrap_or(false));
        if bb2 {
            bits |= 1;
        }
        if fresh.get(side).copied().unwrap_or(false) {
            bits |= 2;
        }
        if bb2 && rs && c2s {
            bits |= 4;
        }
        if narrow && c2s && bb3 && rs && ema_ok {
            let es = s.ema.scaled();
            let cs = c * SCALE;
            let ema_side = if long { cs > es } else { cs < es };
            let cci_ok = cc.is_some_and(|x| if long { x.above(lv.cci_hi) } else { x.below(lv.cci_lo) });
            if ema_side && cci_ok {
                let k4 = wr.is_some_and(|x| if long { x.above(lv.wr_m85) } else { x.below(lv.wr_m15) });
                let k5 = wr.is_some_and(|x| if long { x.above(lv.wr_m15) } else { x.below(lv.wr_m85) });
                if k4 {
                    bits |= 8;
                }
                if k5 {
                    bits |= 16;
                }
            }
        }
        if let Some(o) = out.get_mut(side) {
            *o = bits;
        }
    }
    out
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
            s.new_session();
            for b in *bars {
                let c = candle_of(b);
                if s.fold(&c)? {
                    s.clean_step();
                    s.after_clean(&c, None);
                } else {
                    s.hole();
                    s.after_hole(None);
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
    let cx = if allowed { Some(day_cx(state, prev, &zmin, ctx.nifty, d)) } else { None };
    let mut recs: Vec<Vec<Rec>> = Vec::with_capacity(N_TF);
    let mut bmaps: Vec<Vec<usize>> = Vec::with_capacity(N_TF);
    let mut pending: Vec<Pending> = Vec::new();
    for (ti, &k) in TFS.iter().enumerate() {
        let Some(s) = state.tf.get_mut(ti) else { continue };
        s.new_session();
        let bars = dbars.get(ti).copied().unwrap_or(&[]);
        let mut cur = 0_usize;
        let span = close_min - OPEN_MIN;
        let nb = (span + k - 1).div_euclid(k);
        let mut v: Vec<Rec> = Vec::with_capacity(usize::try_from(nb).unwrap_or(0));
        let mut bmap: Vec<usize> = Vec::with_capacity(usize::try_from(nb).unwrap_or(0));
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
            let mut rec = Rec { t: t_close, hole: true, o: 0, h: 0, l: 0, c: 0, st: [None; 3], side: [None; 3], clean: 0 };
            let bidx = usize::try_from(b).unwrap_or(usize::MAX);
            // The previous scheduled candle of the session, before this one is pushed.
            let prev_bucket: Option<Option<SessBar>> = bidx.checked_sub(1).map(|p| s.sess.get(p).copied().flatten());
            match bar {
                None => {
                    if !missing && cells.iter().all(|x| x.is_some_and(|b| b.volume == 0)) {
                        // A bucket of only padded minutes has no bar, neither written
                        // nor withheld (fold.rs:1056-1059): no candle, no hole.
                        if let Some(p) = out.counts.padded_only.get_mut(ti) {
                            *p += 1;
                        }
                        s.sess.push(None);
                        bmap.push(usize::MAX);
                        continue;
                    }
                    s.hole();
                    s.after_hole(None);
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
                        s.after_hole(None);
                        if let Some(h) = out.counts.holes.get_mut(ti) {
                            *h += 1;
                        }
                        if let Some(h) = out.counts.holes_refused.get_mut(ti) {
                            *h += 1;
                        }
                    } else if missing {
                        // A hole candle that holds bars is still folded (its minutes are real trades).
                        s.hole();
                        s.after_hole(Some(&c));
                        if let Some(h) = out.counts.holes.get_mut(ti) {
                            *h += 1;
                        }
                        if let Some(h) = out.counts.holes_missing_minute.get_mut(ti) {
                            *h += 1;
                        }
                    } else {
                        s.clean_step();
                        s.after_clean(&c, prev);
                        rec = Rec {
                            t: t_close,
                            hole: false,
                            o: bar.open,
                            h: bar.high,
                            l: bar.low,
                            c: bar.close,
                            st: s.bands(),
                            side: s.sides(),
                            clean: s.since,
                        };
                    }
                }
            }
            let my_idx = v.len();
            if allowed
                && !rec.hole
                && let Some(cx) = cx.as_ref()
            {
                if t_close < t_cut {
                    if let Some(x) = out.counts.decisions.get_mut(ti) {
                        *x += 1;
                    }
                    if s.since < 10
                        && let Some(x) = out.counts.taint10.get_mut(ti)
                    {
                        *x += 1;
                    }
                    if s.since < 200
                        && let Some(x) = out.counts.taint200.get_mut(ti)
                    {
                        *x += 1;
                    }
                }
                // K2's fresh R1 / S1 (SEARCH-SPEC 2.2).
                let fresh = prev.map_or([false, false], |(h, l, pc)| {
                    let t3 = h + l + pc;
                    let c = i128::from(rec.c);
                    let beyond = |x: i128| [3 * x > 2 * t3 - 3 * l, 3 * x < 2 * t3 - 3 * h];
                    let now = beyond(c);
                    match prev_bucket {
                        None => now,
                        Some(None) => [false, false],
                        Some(Some(pb)) => {
                            let was = beyond(i128::from(pb.c));
                            [now[0] && !was[0], now[1] && !was[1]]
                        }
                    }
                });
                let base = core_base(s, rec.c, prev, narrow, &ctx.levels, fresh);
                let mut lane_hits: [Option<(u8, u8, usize, usize)>; N_LANE] = [None; N_LANE];
                for lane in 0..lanes_of(ti) {
                    let st_i = LANE_ST.get(lane).copied().unwrap_or(0);
                    let fast = LANE_FAST.get(lane).copied().unwrap_or(false);
                    let (src, lane_tf, lane_idx): (Option<Rec>, usize, usize) = if fast {
                        let Some(f) = FAST_OF.get(ti).copied().flatten() else { continue };
                        let fk = TFS.get(f).copied().unwrap_or(1);
                        // The fast candle that closes at T: bucket (T - 09:15) / fk - 1.
                        let fb = (t_close - at_min(d, OPEN_MIN)).div_euclid(fk * US_MIN) - 1;
                        let fidx = usize::try_from(fb).ok().and_then(|fb| bmaps.get(f).and_then(|m| m.get(fb)).copied());
                        match fidx {
                            Some(ix) if ix != usize::MAX => (recs.get(f).and_then(|r| r.get(ix)).copied(), f, ix),
                            _ => (None, f, 0),
                        }
                    } else {
                        (Some(rec), ti, my_idx)
                    };
                    let Some(src) = src else { continue };
                    if src.hole || src.t != t_close {
                        continue;
                    }
                    let Some(side) = src.side.get(st_i).copied().flatten() else { continue };
                    if src.st.get(st_i).copied().flatten().is_none() {
                        continue;
                    }
                    if src.clean < ST_PERIOD.get(st_i).copied().unwrap_or(u32::MAX) {
                        continue;
                    }
                    let cores = base.get(usize::from(side)).copied().unwrap_or(0);
                    if cores == 0 {
                        continue;
                    }
                    for core in 0..N_CORE {
                        if cores & (1 << core) != 0 {
                            let ix = sig_ix(ti, lane, core, usize::from(side));
                            let table = if t_close < t_cut { &mut out.counts.signals } else { &mut out.counts.cut };
                            if let Some(x) = table.get_mut(ix) {
                                *x += 1;
                            }
                        }
                    }
                    if t_close < t_cut
                        && let Some(slot) = lane_hits.get_mut(lane)
                    {
                        *slot = Some((side, cores, lane_tf, lane_idx));
                    }
                }
                if lane_hits.iter().any(Option::is_some) {
                    let fx = filters_at(s, cx, &rec, &ctx.levels, ctx.nifty, ti, bidx, d);
                    for (lane, hit) in lane_hits.iter().enumerate() {
                        let Some((side, cores, lane_tf, lane_idx)) = *hit else { continue };
                        let si = usize::from(side);
                        pending.push(Pending {
                            ti,
                            lane,
                            lane_tf,
                            lane_idx,
                            side,
                            cores,
                            fbits: fx.bits.get(si).copied().unwrap_or([0; FW]),
                            codes: fx.codes.get(si).copied().unwrap_or([u8::MAX; 3]),
                            strength: fx.strength,
                            t: t_close,
                            sig: [rec.o, rec.h, rec.l, rec.c],
                        });
                    }
                }
            }
            // B02's memory: an earlier candle of this session (any with a bar) closed beyond PH / PL.
            if let Some(Some(sb)) = s.sess.last().copied()
                && let Some((ph, pl, _)) = prev
            {
                if i128::from(sb.c) > ph {
                    s.above_ph = true;
                }
                if i128::from(sb.c) < pl {
                    s.below_pl = true;
                }
            }
            bmap.push(my_idx);
            v.push(rec);
        }
        if let Some(u) = out.counts.unscheduled_bars.get_mut(ti) {
            *u += u64::try_from(bars.len().saturating_sub(cur)).unwrap_or(0);
        }
        recs.push(v);
        bmaps.push(bmap);
    }
    // The day's own bar becomes the next day's previous session (3.4); the daily ATR
    // folds the accepted day bar; the day's cumulative volume joins the RVOL history.
    if let Some(b) = day_bar {
        let r = DailyReference::new(candle_of(&b), DailyEligibility::Eligible);
        if r.is_ok() {
            state.datr.fold(&candle_of(&b));
        }
        state.prev = Some(
            r.map(|r| {
                let bar = r.bar();
                (i128::from(bar.high), i128::from(bar.low), i128::from(bar.close))
            })
            .map_err(|_| ()),
        );
    }
    if has_min {
        let mut cv = Vec::with_capacity(n_min + 1);
        let mut acc = 0_i128;
        cv.push(0);
        for z in &zmin {
            if let Some(b) = z {
                acc += i128::from(b.volume);
            }
            cv.push(acc);
        }
        state.rvol.push(cv);
        if state.rvol.len() > 10 {
            state.rvol.remove(0);
        }
    }
    if pending.is_empty() {
        return Ok(());
    }
    out.counts.candidate_days += 1;
    // The day's fills: GDFL seconds, or Zerodha minutes on a fallback day (T1's code).
    let mut fallback_why: Option<u8> = None;
    let mut secs: Vec<Tick> = Vec::new();
    let mut g_close: Vec<Option<i64>> = vec![None; n_min];
    if d == ctx.gdfl_gap_day {
        fallback_why = Some(0);
    } else {
        if matches!(gm, GMonth::NotOpened) {
            *gm = match open_month(ctx.store, Vendor::Gdfl, "CASH", &sym.series, id, Timeframe::SECOND_1, mi) {
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
    for p in &pending {
        let lv = recs.get(p.lane_tf).map_or(&[][..], Vec::as_slice);
        let st_i = LANE_ST.get(p.lane).copied().unwrap_or(0);
        let with_long = p.side == 0;
        let sw = simulate(&ticks, best.as_deref(), lv, p.lane_idx, st_i, with_long, d, close_min, time_from);
        let sf = simulate(&ticks, best.as_deref(), lv, p.lane_idx, st_i, !with_long, d, close_min, time_from);
        let tw = make_trade(&sw, with_long, fallback, p.t).map_err(|e| format!("{} {e}", sym.name))?;
        let tf = make_trade(&sf, !with_long, fallback, p.t).map_err(|e| format!("{} {e}", sym.name))?;
        for (k, tr) in [tw, tf].iter().enumerate() {
            if tr.status == Status::Filled {
                if let Some(x) = out.counts.filled.get_mut(p.ti).and_then(|x| x.get_mut(p.lane)).and_then(|x| x.get_mut(k)) {
                    *x += 1;
                }
                if tr.at_once
                    && let Some(x) = out.counts.at_once.get_mut(p.ti).and_then(|x| x.get_mut(p.lane)).and_then(|x| x.get_mut(k))
                {
                    *x += 1;
                }
            }
        }
        if let Some(x) = out.counts.records.get_mut(p.ti).and_then(|x| x.get_mut(p.lane)) {
            *x += 1;
        }
        let rec = Record {
            t: p.t,
            stock: sym.rank,
            side: p.side,
            cores: p.cores,
            codes: p.codes,
            fbits: p.fbits,
            strength: p.strength,
            sig: p.sig,
            tr: [tw, tf],
        };
        if let Some(v) = out.recs.get_mut(p.ti * N_LANE + p.lane) {
            v.push(rec);
        }
    }
    Ok(())
}

/// The filters' answers on one record candle.
struct FxOut {
    /// `[long, short]` filter bits (signal-side meaning).
    bits: [[u64; FW]; 2],
    /// `[long, short]` candle codes of t-2, t-1, t (`u8::MAX` when not available).
    codes: [[u8; 3]; 2],
    /// Relative volume of the signal candle in ppm of its 20-candle average (the
    /// monthly trade cap's strength score).
    strength: i64,
}

/// A candle of the session `j` back from bucket `b` (0 = this one), with a bar and not a hole.
fn sess_back(s: &TfState, b: usize, j: usize) -> Option<SessBar> {
    let i = b.checked_sub(j)?;
    s.sess.get(i).copied().flatten().filter(|x| !x.hole)
}

/// The price-mirrored candle (a short's meaning of a long test).
fn mirror(o: i128, h: i128, l: i128, c: i128) -> (i128, i128, i128, i128) {
    (-o, -l, -h, -c)
}

/// Every filter of the library on the candle that closes at T (SEARCH-SPEC 3; ADDENDUM 3
/// and 4). Reads only the candle t and earlier, today's minutes that start before T, the
/// previous sessions and NIFTY minutes that end at or before T.
#[allow(clippy::too_many_arguments, clippy::too_many_lines, clippy::similar_names, reason = "the library, one filter at a time")]
fn filters_at(s: &TfState, cx: &DayCx, rec: &Rec, lv: &Levels, nifty: &Nifty, ti: usize, b: usize, d: i64) -> FxOut {
    let mut bl = [0_u64; FW];
    let mut bs = [0_u64; FW];
    let mut put = |i: usize, l: bool, sh: bool| {
        set(&mut bl, i, l);
        set(&mut bs, i, sh);
    };
    let (o, h, l, c) = (i128::from(rec.o), i128::from(rec.h), i128::from(rec.l), i128::from(rec.c));
    let since = s.since;
    let t = rec.t;
    let tmin = ist_min_of(t);
    let k_min = usize::try_from(tmin - OPEN_MIN).unwrap_or(0);
    let cb = bollinger::Close::new(rec.c).ok();
    let v_t = s.ring.back(0).map_or(0, |x| i128::from(x.v));
    // (a) indicators
    let r = if since >= 14 { s.rsi.reading().filter(rsi::Reading::faithful) } else { None };
    put(0, r.is_some_and(|r| r.above(lv.rsi60)), r.is_some_and(|r| r.below(lv.rsi40)));
    put(1, r.is_some_and(|r| r.above(lv.rsi_hi)), r.is_some_and(|r| r.below(lv.rsi_lo)));
    put(2, r.is_some_and(|r| !r.above(lv.rsi80)), r.is_some_and(|r| !r.below(lv.rsi20)));
    let wr = if since >= 14 { s.wr.reading().filter(|x| !x.flat()) } else { None };
    put(3, wr.is_some_and(|x| x.above(lv.wr_m85)), wr.is_some_and(|x| x.below(lv.wr_m15)));
    put(4, wr.is_some_and(|x| x.above(lv.wr_m50)), wr.is_some_and(|x| x.below(lv.wr_m50)));
    put(5, wr.is_some_and(|x| x.above(lv.wr_m15)), wr.is_some_and(|x| x.below(lv.wr_m85)));
    let cc = if since >= 20 { s.cci.reading().filter(|x| !x.flat()) } else { None };
    put(6, cc.is_some_and(|x| x.above(lv.cci_100)), cc.is_some_and(|x| x.below(lv.cci_m100)));
    put(7, cc.is_some_and(|x| x.above(lv.cci_hi)), cc.is_some_and(|x| x.below(lv.cci_lo)));
    let w = if since >= 20 { s.bb.window() } else { None };
    let bbt = |wd: bollinger::Width| match (w, cb) {
        (Some(w), Some(cb)) => (w.above_upper(cb, wd), w.below_lower(cb, wd)),
        _ => (false, false),
    };
    let (a, z) = bbt(lv.w25);
    put(8, a, z);
    let (a, z) = bbt(lv.w3);
    put(9, a, z);
    let cs = c * SCALE;
    let e50 = (s.ema50.warm() && since >= 50).then(|| s.ema50.scaled());
    put(10, e50.is_some_and(|e| cs > e), e50.is_some_and(|e| cs < e));
    let slope = |n: usize, f: fn(&Slot) -> Option<i128>| -> Option<(i128, i128)> {
        let win = s.ring.clean(0, n)?;
        let now = f(win.first()?)?;
        let then = f(win.last()?)?;
        if win.iter().any(|x| f(x).is_none()) {
            return None;
        }
        Some((now, then))
    };
    let sl50 = slope(5, |x| x.e50);
    put(11, sl50.is_some_and(|(a, b)| a > b), sl50.is_some_and(|(a, b)| a < b));
    let e200 = (s.ema.warm() && since >= 200).then(|| s.ema.scaled());
    put(12, e200.is_some_and(|e| cs > e), e200.is_some_and(|e| cs < e));
    let sl200 = slope(10, |x| x.e200);
    put(13, sl200.is_some_and(|(a, b)| a > b), sl200.is_some_and(|(a, b)| a < b));
    let stk = e50.zip(e200);
    put(14, stk.is_some_and(|(a, b)| a > b), stk.is_some_and(|(a, b)| a < b));
    let dmi = (s.dmi.ready && since >= 28).then_some((s.dmi.adx, s.dmi.p, s.dmi.m));
    put(15, dmi.is_some_and(|(a, p, m)| a >= 200_000 && p > m), dmi.is_some_and(|(a, p, m)| a >= 200_000 && m > p));
    put(16, dmi.is_some_and(|(a, p, m)| a >= 300_000 && p > m), dmi.is_some_and(|(a, p, m)| a >= 300_000 && m > p));
    let datr = cx.datr.zip(cx.prev.map(|x| x.2));
    put(17, datr.is_some_and(|(a, pc)| 10_000 * i128::from(a) >= 250 * pc), datr.is_some_and(|(a, pc)| 10_000 * i128::from(a) >= 250 * pc));
    put(18, datr.is_some_and(|(a, pc)| 10_000 * i128::from(a) <= 150 * pc), datr.is_some_and(|(a, pc)| 10_000 * i128::from(a) <= 150 * pc));
    let mh = if since >= 35 { s.macd.h } else { None };
    put(19, mh.is_some_and(|x| x > 0), mh.is_some_and(|x| x < 0));
    let rise = mh.zip(s.macd.h_prev);
    put(20, rise.is_some_and(|(a, b)| s.macd.m > 0 && a > b), rise.is_some_and(|(a, b)| s.macd.m < 0 && a < b));
    let kd = if since >= 16 {
        match (s.k_now, s.k_hist[0], s.k_hist[1]) {
            (Some(k0), Some(k1), Some(k2)) => Some((k0, (k0 + k1 + k2).div_euclid(3))),
            _ => None,
        }
    } else {
        None
    };
    put(21, kd.is_some_and(|(k, dd)| k >= 800_000 && k > dd), kd.is_some_and(|(k, dd)| k <= 200_000 && k < dd));
    put(22, kd.is_some_and(|(k, dd)| k >= 500_000 && k > dd), kd.is_some_and(|(k, dd)| k <= 500_000 && k < dd));
    let pv = cx.pv.get(k_min).copied().unwrap_or(0);
    let sv = cx.sv.get(k_min).copied().unwrap_or(0);
    if sv > 0 && pv > 0 {
        let dv = 3 * c * sv - pv;
        put(23, dv > 0, dv < 0);
        put(24, dv > 0 && 10_000 * dv <= 50 * pv, -dv > 0 && 10_000 * (-dv) <= 50 * pv);
        put(25, 10_000 * dv >= 100 * pv, 10_000 * (-dv) >= 100 * pv);
    }
    let s20 = s.ring.clean(1, 20).map(|w| w.iter().map(|x| i128::from(x.v)).sum::<i128>());
    if let Some(s20) = s20.filter(|x| *x > 0) {
        put(26, 40 * v_t >= 3 * s20, 40 * v_t >= 3 * s20);
        put(27, 10 * v_t >= s20, 10 * v_t >= s20);
    }
    let cv_now = cx.cv.get(k_min).copied().unwrap_or(0);
    if let Some(&rr) = cx.rv.get(k_min) {
        let ok = rr > 0 && 20 * cv_now >= 3 * rr;
        put(28, ok, ok);
    }
    let obv = s.ring.clean(0, 10).and_then(|w| Some((w.first()?.obv?, w.last()?.obv?)));
    put(29, obv.is_some_and(|(a, b)| a > b), obv.is_some_and(|(a, b)| a < b));
    // (b) price context
    if let Some((ph, pl, pc)) = cx.prev {
        let t3 = ph + pl + pc;
        put(30, c > ph, c < pl);
        put(31, c > ph && !s.above_ph, c < pl && !s.below_pl);
        if let Some(dopen) = cx.day_open {
            let dop = i128::from(dopen);
            put(34, 10_000 * (dop - pc) >= 50 * pc, 10_000 * (pc - dop) >= 50 * pc);
            put(35, 10_000 * (pc - dop) >= 50 * pc, 10_000 * (dop - pc) >= 50 * pc);
            let flat = 10_000 * (dop - pc).abs() <= 25 * pc;
            put(36, flat, flat);
        }
        let wd = 10_000 * (2 * pc - ph - pl).abs();
        put(39, wd <= 10 * t3, wd <= 10 * t3);
        put(40, wd <= 20 * t3, wd <= 20 * t3);
        put(41, wd >= 50 * t3, wd >= 50 * t3);
        let r1 = 3 * c > 2 * t3 - 3 * pl;
        let s1 = 3 * c < 2 * t3 - 3 * ph;
        let r2 = 3 * c > t3 + 3 * (ph - pl);
        let s2 = 3 * c < t3 - 3 * (ph - pl);
        put(42, r1, s1);
        put(43, r2, s2);
        put(44, r1 && !r2, s1 && !s2);
        put(45, 3 * c > t3, 3 * c < t3);
    }
    if tmin >= 570
        && let Some((oh, ol)) = cx.or15
    {
        put(32, c > i128::from(oh), c < i128::from(ol));
    }
    if tmin >= 585
        && let Some((oh, ol)) = cx.or30
    {
        put(33, c > i128::from(oh), c < i128::from(ol));
    }
    let dh = cx.dh.get(k_min).copied().flatten().map(i128::from);
    let dl = cx.dl.get(k_min).copied().flatten().map(i128::from);
    if let (Some(dh), Some(dl)) = (dh, dl) {
        put(37, 10_000 * (dh - c) <= 25 * c, 10_000 * (c - dl) <= 25 * c);
        put(38, 10_000 * (c - dl) <= 100 * dl, 10_000 * (dh - c) <= 100 * dh);
    }
    // (c) bars and candles
    let pm = &s.pmask;
    put(46, pm.get(157), pm.get(158));
    put(47, pm.get(153), pm.get(156));
    put(48, pm.get(172), pm.get(173));
    put(49, pm.get(165), pm.get(166));
    let p1 = sess_back(s, b, 1);
    let p2 = sess_back(s, b, 2);
    if let Some(p) = p1 {
        let (ph1, pl1) = (i128::from(p.h), i128::from(p.l));
        let doji = ph1 > pl1 && 10 * (i128::from(p.c) - i128::from(p.o)).abs() <= ph1 - pl1;
        put(50, doji, doji);
        put(53, h > ph1 && l < pl1 && c > o, h > ph1 && l < pl1 && c < o);
        if let Some(q) = p2 {
            let (qh, ql) = (i128::from(q.h), i128::from(q.l));
            let inside = ph1 <= qh && pl1 >= ql;
            put(51, inside && c > ph1, inside && c < pl1);
            let (c1, c2) = (i128::from(p.c), i128::from(q.c));
            let ups = c > o && c1 > i128::from(p.o) && c2 > i128::from(q.o);
            let dns = c < o && c1 < i128::from(p.o) && c2 < i128::from(q.o);
            put(56, c > c1 && c1 > c2 && ups, c < c1 && c1 < c2 && dns);
            put(60, h > ph1 && ph1 > qh && l > pl1 && pl1 > ql, h < ph1 && ph1 < qh && l < pl1 && pl1 < ql);
        }
        let mut nr = true;
        let r1 = ph1 - pl1;
        for j in 2..=4 {
            match sess_back(s, b, j) {
                Some(x) => {
                    if r1 >= i128::from(x.h) - i128::from(x.l) {
                        nr = false;
                    }
                }
                None => nr = false,
            }
        }
        put(52, nr, nr);
    }
    if h > l {
        put(54, 5 * (c - l) >= 4 * (h - l), 5 * (h - c) >= 4 * (h - l));
    }
    put(55, c > o && 10 * (c - o) >= 6 * (h - l), c < o && 10 * (o - c) >= 6 * (h - l));
    if let Some(wn) = s.ring.clean(1, 10) {
        let sr: i128 = wn.iter().map(|x| i128::from(x.h) - i128::from(x.l)).sum();
        let wrb = 10 * (h - l) >= 2 * sr;
        put(57, wrb, wrb);
    }
    // (d) chart structure
    for (fi, n) in [(58_usize, 20_usize), (59, 50)] {
        if let Some(wn) = s.ring.clean(1, n) {
            let mx = wn.iter().map(|x| x.h).max().map(i128::from);
            let mn = wn.iter().map(|x| x.l).min().map(i128::from);
            put(fi, mx.is_some_and(|m| c > m), mn.is_some_and(|m| c < m));
        }
    }
    if let Some(wn) = s.ring.clean(0, 19) {
        let a = wn.get(0..10).unwrap_or(&[]);
        let z = wn.get(10..20).unwrap_or(&[]);
        let mxa = a.iter().map(|x| x.h).max();
        let mxz = z.iter().map(|x| x.h).max();
        let mna = a.iter().map(|x| x.l).min();
        let mnz = z.iter().map(|x| x.l).min();
        if let (Some(mxa), Some(mxz), Some(mna), Some(mnz)) = (mxa, mxz, mna, mnz) {
            put(61, mna > mnz && mxa > mxz, mxa < mxz && mna < mnz);
        }
    }
    if let (Some(w), Some(cb)) = (w, cb)
        && let Some(wn) = s.ring.clean(1, 120)
    {
        let vs: Option<Vec<i128>> = wn.iter().map(|x| x.bbv).collect();
        if let Some(vs) = vs
            && let Some(&v1) = vs.first()
        {
            let below = vs.iter().filter(|&&x| x < v1).count();
            let sq = below < 24;
            put(62, sq && w.above_upper(cb, lv.w2), sq && w.below_lower(cb, lv.w2));
        }
    }
    // (e) time and market
    let both = |x: bool| (x, x);
    let (a, z) = both(tmin <= 660);
    put(63, a, z);
    let (a, z) = both(tmin > 660 && tmin <= 810);
    put(64, a, z);
    let (a, z) = both(tmin > 810);
    put(65, a, z);
    let (a, z) = both(tmin >= 570);
    put(66, a, z);
    let wdy = cx.weekday;
    let (a, z) = both(wdy != 0);
    put(67, a, z);
    let (a, z) = both(wdy != 4);
    put(68, a, z);
    let (a, z) = both((1..=3).contains(&wdy));
    put(69, a, z);
    let n_t = cx.n_last.get(k_min).copied().flatten();
    if let (Some(nt), Some(pn)) = (n_t, cx.n_prev) {
        let (nt, pn) = (i128::from(nt), i128::from(pn));
        put(70, nt > pn, nt < pn);
        if let Some((_, _, pc)) = cx.prev {
            put(
                72,
                10_000 * (c - pc) * pn - 10_000 * (nt - pn) * pc >= 50 * pc * pn,
                10_000 * (pc - c) * pn - 10_000 * (pn - nt) * pc >= 50 * pc * pn,
            );
        }
    }
    if let Some(sd) = nifty.ema_side(ti, t) {
        put(71, sd > 0, sd < 0);
    }
    if let (Some(dopen), Some((_, _, pc))) = (cx.day_open, cx.prev) {
        let dop = i128::from(dopen);
        put(73, c > dop && c > pc, c < dop && c < pc);
    }
    // ADDENDUM 4 (1): candle anatomy of t, t-1, t-2 in the session; the short's meaning
    // is the long test on the price-mirrored candle.
    let cands: [Option<SessBar>; 3] = [sess_back(s, b, 0), p1, p2];
    for (j, cd) in cands.iter().enumerate() {
        let Some(x) = cd else { continue };
        let (xo, xh, xl, xc) = (i128::from(x.o), i128::from(x.h), i128::from(x.l), i128::from(x.c));
        let rg = xh - xl;
        if rg <= 0 {
            continue;
        }
        for side in 0..2 {
            let (mo, mh, ml, mc) = if side == 0 { (xo, xh, xl, xc) } else { mirror(xo, xh, xl, xc) };
            let body = (mc - mo).abs();
            let uw = mh - mc.max(mo);
            let lw = mc.min(mo) - ml;
            let bb = usize::from(body_bucket(body, rg));
            let ub = usize::from(body_bucket(uw, rg));
            let lb = usize::from(body_bucket(lw, rg));
            let cp = if 3 * (mc - ml) >= 2 * rg {
                0
            } else if 3 * (mc - ml) >= rg {
                1
            } else {
                2
            };
            let tgt = if side == 0 { &mut bl } else { &mut bs };
            set(tgt, N_BASE + j * 4 + bb, true);
            set(tgt, N_BASE + 12 + j * 4 + ub, true);
            set(tgt, N_BASE + 24 + j * 4 + lb, true);
            set(tgt, N_BASE + 36 + j * 3 + cp, true);
            if let Some(atr) = x.atr.filter(|a| *a > 0) {
                let atr = i128::from(atr);
                let ra = if 10 * rg < 7 * atr {
                    0
                } else if 10 * rg <= 13 * atr {
                    1
                } else {
                    2
                };
                set(tgt, N_BASE + 45 + j * 3 + ra, true);
            }
        }
    }
    let mut put = |i: usize, l: bool, sh: bool| {
        set(&mut bl, i, l);
        set(&mut bs, i, sh);
    };
    // ADDENDUM 4 (3): named patterns (N01..N17 = 128..144)
    let pairs: [(u32, u32); 16] = [
        (154, 155),
        (159, 160),
        (161, 162),
        (163, 164),
        (167, 168),
        (170, 169),
        (171, 171),
        (174, 175),
        (176, 177),
        (198, 199),
        (200, 201),
        (202, 203),
        (204, 205),
        (206, 207),
        (208, 209),
        (224, 224),
    ];
    for (j, (pl_, ps_)) in pairs.iter().enumerate() {
        put(N_BASE + 54 + j, pm.get(*pl_), pm.get(*ps_));
    }
    let doji = h > l && 10 * (c - o).abs() <= h - l;
    put(N_BASE + 70, doji, doji);
    // ADDENDUM 4 (4): chart structures (G01..G07 = 145..151)
    let prev_c = s.ring.back(1).filter(|x| !x.hole).map(|x| i128::from(x.c));
    for (fi, j) in [(N_BASE + 71, 0_usize), (N_BASE + 72, 1)] {
        let sh = s.sw.hi.get(j).and_then(|x| x[1]).map(|p| i128::from(p.v));
        let sl = s.sw.lo.get(j).and_then(|x| x[1]).map(|p| i128::from(p.v));
        let lg = sh.zip(prev_c).is_some_and(|(v, pc)| c > v && pc <= v);
        let st = sl.zip(prev_c).is_some_and(|(v, pc)| c < v && pc >= v);
        put(fi, lg, st);
    }
    {
        let hi = s.sw.hi.first().copied().unwrap_or([None, None]);
        let lo = s.sw.lo.first().copied().unwrap_or([None, None]);
        if let ([Some(h1), Some(h2)], [Some(l1), Some(l2)]) = (hi, lo) {
            put(N_BASE + 73, h2.v > h1.v && l2.v > l1.v, h2.v < h1.v && l2.v < l1.v);
        }
    }
    {
        let lo = s.sw.lo.get(1).copied().unwrap_or([None, None]);
        let hi = s.sw.hi.get(1).copied().unwrap_or([None, None]);
        let between = |a: u64, z: u64| -> Option<Vec<Slot>> {
            let mut out = Vec::new();
            let mut j = 0;
            while let Some(x) = s.ring.back(j) {
                if x.seq <= a {
                    break;
                }
                if x.seq < z {
                    if x.hole {
                        return None;
                    }
                    out.push(*x);
                }
                j += 1;
            }
            let oldest = s.ring.back(s.ring.len.saturating_sub(1)).map_or(u64::MAX, |x| x.seq);
            (oldest <= a && !out.is_empty()).then_some(out)
        };
        let mut lg = false;
        let mut st = false;
        if let [Some(l1), Some(l2)] = lo {
            let (a1, a2) = (i128::from(l1.v), i128::from(l2.v));
            if a1 > 0
                && 10_000 * (a2 - a1).abs() <= 30 * a1
                && let Some(btw) = between(l1.seq, l2.seq)
                && let Some(nk) = btw.iter().map(|x| x.h).max()
            {
                lg = c > i128::from(nk);
            }
        }
        if let [Some(h1), Some(h2)] = hi {
            let (a1, a2) = (i128::from(h1.v), i128::from(h2.v));
            if a1 > 0
                && 10_000 * (a2 - a1).abs() <= 30 * a1
                && let Some(btw) = between(h1.seq, h2.seq)
                && let Some(nk) = btw.iter().map(|x| x.l).min()
            {
                st = c < i128::from(nk);
            }
        }
        put(N_BASE + 74, lg, st);
    }
    if let Some(wn) = s.ring.clean(0, 16) {
        let c6 = wn.get(6).map(|x| i128::from(x.c));
        let c16 = wn.get(16).map(|x| i128::from(x.c));
        let f5 = wn.get(1..=5).unwrap_or(&[]);
        let mx = f5.iter().map(|x| x.h).max().map(i128::from);
        let mn = f5.iter().map(|x| x.l).min().map(i128::from);
        if let (Some(c6), Some(c16), Some(mx), Some(mn)) = (c6, c16, mx, mn) {
            let up = c6 - c16;
            let dn = c16 - c6;
            let lg = 10_000 * up >= 100 * c16 && 2 * (mx - mn) <= up && c > mx;
            let st = 10_000 * dn >= 100 * c16 && 2 * (mx - mn) <= dn && c < mn;
            put(N_BASE + 75, lg, st);
        }
    }
    if let Some(wn) = s.ring.clean(0, 10) {
        let a5 = wn.get(1..=5).unwrap_or(&[]);
        let b5 = wn.get(6..=10).unwrap_or(&[]);
        let (mxa, mxb) = (a5.iter().map(|x| x.h).max(), b5.iter().map(|x| x.h).max());
        let (mna, mnb) = (a5.iter().map(|x| x.l).min(), b5.iter().map(|x| x.l).min());
        if let (Some(mxa), Some(mxb), Some(mna), Some(mnb)) = (mxa, mxb, mna, mnb) {
            let tri = mxa < mxb && mna > mnb;
            put(N_BASE + 76, tri && c > i128::from(mxa), tri && c < i128::from(mna));
        }
    }
    if let Some(wn) = s.ring.clean(0, 30) {
        let mut lg = false;
        let mut st = false;
        let mut found_l = false;
        let mut found_s = false;
        for j in 1..=10_usize {
            let (Some(cj), Some(prior)) = (wn.get(j), wn.get(j + 1..=j + 20)) else { continue };
            let mx = prior.iter().map(|x| x.h).max().map(i128::from);
            let mn = prior.iter().map(|x| x.l).min().map(i128::from);
            if !found_l && let Some(mx) = mx && i128::from(cj.c) > mx {
                found_l = true;
                lg = l <= mx && c > mx;
            }
            if !found_s && let Some(mn) = mn && i128::from(cj.c) < mn {
                found_s = true;
                st = h >= mn && c < mn;
            }
        }
        put(N_BASE + 77, lg, st);
    }
    // ADDENDUM 4 (5): Fibonacci (H01..H08 swing, H09..H16 previous day)
    for li in 0..4_usize {
        for touch in 0..2_usize {
            let bit = 1_u8 << (2 * li + touch);
            let fl = s.fib_bits.first().is_some_and(|x| x & bit != 0);
            let fs = s.fib_bits.get(1).is_some_and(|x| x & bit != 0);
            put(N_BASE + 78 + li * 2 + touch, fl, fs);
            let pl_ = s.pd_bits.first().is_some_and(|x| x & bit != 0);
            let ps_ = s.pd_bits.get(1).is_some_and(|x| x & bit != 0);
            put(N_BASE + 86 + li * 2 + touch, pl_, ps_);
        }
    }
    // ADDENDUM 3 (3): a minimum expected move
    let atr = (s.atr.warm() && since >= 14).then(|| s.atr.value()).flatten().map(i128::from);
    for (j, lvl) in [25_i128, 50, 100].iter().enumerate() {
        let ok = atr.is_some_and(|a| 10_000 * a >= lvl * c);
        put(N_BASE + 94 + j, ok, ok);
    }
    let _ = d;
    // Candle codes (ADDENDUM 4 (2)).
    let mut codes = [[u8::MAX; 3]; 2];
    for (j, cd) in [p2, p1, sess_back(s, b, 0)].iter().enumerate() {
        let Some(x) = cd else { continue };
        let (xo, xh, xl, xc) = (i128::from(x.o), i128::from(x.h), i128::from(x.l), i128::from(x.c));
        if let Some(code) = candle_code(xo, xh, xl, xc) {
            codes[0][j] = code;
        }
        let (mo, mh, ml, mc) = mirror(xo, xh, xl, xc);
        if let Some(code) = candle_code(mo, mh, ml, mc) {
            codes[1][j] = code;
        }
    }
    let strength = s20
        .filter(|x| *x > 0)
        .map_or(0, |s20| i64::try_from((20 * 1_000_000 * v_t).div_euclid(s20)).unwrap_or(i64::MAX));
    FxOut { bits: [bl, bs], codes, strength }
}

/// Self-check (spec 5): one month of a stock's minutes folded by `derive_minutes`
/// gives 125 three-minute and 7 sixty-minute buckets per full session, each stamped
/// on the 09:15 grid. T1's check.
pub(crate) fn grid_check(root: &Path, series: &str, y: u16, m: u8) -> Result<String, String> {
    let id = sym_id(series)?;
    let mi = crate::common::month_index(y, m).ok_or_else(|| format!("{y}-{m:02}"))?;
    let f = match open_month(root, Vendor::Zerodha, "CASH", series, id, Timeframe::MINUTE_1, mi) {
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
            let bars: &[Bar] = dg.iter().find(|g| g.0 == d).and_then(|g| dv.bars.get(g.1..g.2)).unwrap_or(&[]);
            if bars.len() != want {
                return Err(format!("{series} {y}-{m:02} day {d}: {} {k}-minute bars from derive_minutes, expected {want}", bars.len()));
            }
            for (i, bar) in bars.iter().enumerate() {
                let step = i64::try_from(i).map_err(|_| "index".to_owned())?;
                let expect = at_min(d, OPEN_MIN + step * k);
                if bar.ts_micros != expect {
                    return Err(format!("{series} {y}-{m:02} day {d}: {k}-minute bar {i} stamped {} not {expect}", bar.ts_micros));
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
