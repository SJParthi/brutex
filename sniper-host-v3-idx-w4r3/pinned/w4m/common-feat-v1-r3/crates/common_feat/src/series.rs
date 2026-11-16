//! `Series`: one (instrument, timeframe) stream (REQ72-DESIGN A.2). `fold(&Candle, &CandleCtx) -> &Readings` with a
//! fixed set of streaming states, so look-ahead is unreachable by the shape of the fold, and O(1) a candle (the named
//! bounded exceptions: CCI's deviation pass, %R's amortised deque, the NR-n scan of at most 6).
//!
//! Warm-up is CONT (one stream across sessions, `TradingView`). A reading is READY after `need` candles since the
//! stream's start or its last reset (F-C1). For a recursive reading (RSI, ATR and so Supertrend, every EMA) `need` is
//! the candle count at which the seed's remaining weight falls to `seed_weight_max_ppm` or below (one convergence
//! rule, SNIPER-ADANIENT-PLAN 2.1 / CRITIC F5); for a window reading it is the window. An unready reading is still
//! printed RAW but its `ready` flag is false: a host never reads it as true or false.

use crate::anatomy::{Anatomy, AnatomyState, PatCfg};
use crate::cal::{candle_index, ist_sod, weekday};
use crate::fold::Candle;
use crate::ind::{Atr, BbReading, Boll, Cci, Ema, EmaSeed, Rsi, StReading, SuperTrend, WilliamsR};
use crate::num::Ratio;
use crate::rvol::Rvol;
use crate::steps::Steps;

/// The most rows of one reading kind a series holds (a data-row list longer than this is refused at load).
pub const MAXK: usize = 8;

/// The series' rows (FEATURES rows: periods, multipliers, the EMA seed, the readiness rule).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeriesCfg {
    /// Candle length, seconds.
    pub tf_s: i64,
    /// RSI periods.
    pub rsi: Vec<u16>,
    /// CCI periods.
    pub cci: Vec<u16>,
    /// %R periods.
    pub wr: Vec<u16>,
    /// Bollinger (n, k milli).
    pub bb: Vec<(u16, u32)>,
    /// Supertrend (ATR period, factor milli).
    pub st: Vec<(u16, u32)>,
    /// EMA periods.
    pub ema: Vec<u16>,
    /// EMA seed.
    pub ema_seed: EmaSeed,
    /// The ATR of the bar-size reading (VB-RNG / AN-BSZ).
    pub atr: u16,
    /// Relative volume: sessions in the mean.
    pub rvol_m: usize,
    /// Relative volume: sessions present before it is read.
    pub rvol_need: usize,
    /// Candle indices a session can hold (the rvol rings).
    pub kmax: usize,
    /// Pattern thresholds.
    pub pat: PatCfg,
    /// The readiness rule of the recursive readings, ppm of the seed's weight.
    pub seed_weight_max_ppm: i128,
}

impl SeriesCfg {
    /// The sniper's rows (SNIPER-REQUIREMENTS section 13 (3)): RSI 14, CCI 20, %R 14, Bollinger 20 x 2 and 20 x 3,
    /// Supertrend 7 x 1.5, 10 x 2, 10 x 3, EMA 9 / 20 / 50 / 100 / 200, ATR 14, relative volume over 20 sessions,
    /// seed weight 1,000 ppm (a provisional readiness row: set red-first by MAIN, printed on every result).
    #[must_use]
    pub fn sniper(tf_s: i64) -> Self {
        let per_session = 22_500 / tf_s.max(1) + 2;
        Self {
            tf_s,
            rsi: vec![14],
            cci: vec![20],
            wr: vec![14],
            bb: vec![(20, 2000), (20, 3000)],
            st: vec![(7, 1500), (10, 2000), (10, 3000)],
            ema: vec![9, 20, 50, 100, 200],
            ema_seed: EmaSeed::Sma,
            atr: 14,
            rvol_m: 20,
            rvol_need: 20,
            kmax: usize::try_from(per_session).unwrap_or(400),
            pat: PatCfg::default(),
            seed_weight_max_ppm: 1000,
        }
    }

    /// Refuses a row list longer than [`MAXK`].
    ///
    /// # Errors
    ///
    /// Names the list that is too long.
    pub fn check(&self) -> Result<(), String> {
        for (name, n) in [
            ("rsi", self.rsi.len()),
            ("cci", self.cci.len()),
            ("wr", self.wr.len()),
            ("bb", self.bb.len()),
            ("st", self.st.len()),
            ("ema", self.ema.len()),
        ] {
            if n > MAXK {
                return Err(format!("{name}: {n} rows > {MAXK}"));
            }
        }
        Ok(())
    }
}

/// The readiness `need` of a recursive average with smoothing `a_num / a_den` (weight kept a step
/// `1 - a`), seeded over `seed` values: `seed + t`, t the first step count with `(1 - a)^t <= bound_ppm / 10^6`.
#[must_use]
pub fn need_recursive(seed: u64, a_num: i128, a_den: i128, bound_ppm: i128) -> u64 {
    let scale: i128 = 1_000_000_000_000;
    let bound = bound_ppm * 1_000_000;
    let mut w = scale;
    let mut t = 0u64;
    while w > bound && t < 1_000_000 {
        w = (w * (a_den - a_num)).div_euclid(a_den);
        t += 1;
    }
    seed + t
}

/// The context of one candle, from the host's dated session table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandleCtx {
    /// The session table's open of this candle's session (K14).
    pub session_open: i64,
    /// The session's IST day.
    pub day: i32,
    /// The first candle of a new session.
    pub new_session: bool,
    /// A regular full session (relative-volume rings push and read only these, K14).
    pub regular_full: bool,
    /// A factor ex-date session starts here: every CONT stream resets (K13 X3).
    pub cont_reset: bool,
}

/// The RAW readings of one candle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Readings {
    /// The candle.
    pub candle: Candle,
    /// Its session's IST day.
    pub day: i32,
    /// Its number in the session from the session table's open (TM-CANDLE).
    pub k: i64,
    /// Its start time, seconds since IST midnight (TM-TOD).
    pub tod: i64,
    /// Weekday, 0 = Sunday.
    pub weekday: u8,
    /// RSI per row.
    pub rsi: Vec<Option<Ratio>>,
    /// CCI per row.
    pub cci: Vec<Option<Ratio>>,
    /// %R per row.
    pub wr: Vec<Option<Ratio>>,
    /// Bollinger per row.
    pub bb: Vec<Option<BbReading>>,
    /// Supertrend per row.
    pub st: Vec<Option<StReading>>,
    /// EMA per row, at `S`.
    pub ema: Vec<Option<i128>>,
    /// The bar-size ATR at `S`.
    pub atr: Option<i128>,
    /// Range / ATR as an exact ratio.
    pub range_atr: Option<Ratio>,
    /// Anatomy and patterns.
    pub anat: Anatomy,
    /// Relative volume.
    pub rvol: Option<Ratio>,
    /// Ready flags per RSI row.
    pub rsi_ready: Vec<bool>,
    /// Ready flags per Supertrend row.
    pub st_ready: Vec<bool>,
    /// Ready flags per EMA row.
    pub ema_ready: Vec<bool>,
    /// The bar-size ATR is ready.
    pub atr_ready: bool,
    /// Candles since the stream's start or last reset (this one included).
    pub since_reset: u64,
}

/// One (instrument, timeframe) stream.
#[derive(Clone, Debug)]
pub struct Series {
    cfg: SeriesCfg,
    rsi: Vec<Rsi>,
    cci: Vec<Cci>,
    wr: Vec<WilliamsR>,
    bb: Vec<Boll>,
    st: Vec<SuperTrend>,
    ema: Vec<Ema>,
    atr: Atr,
    anat: AnatomyState,
    rvol: Rvol,
    rsi_need: Vec<u64>,
    st_need: Vec<u64>,
    ema_need: Vec<u64>,
    atr_need: u64,
    since_reset: u64,
    out: Readings,
}

impl Series {
    /// A new stream.
    ///
    /// # Errors
    ///
    /// Refuses a row list longer than [`MAXK`].
    pub fn new(cfg: SeriesCfg) -> Result<Self, String> {
        cfg.check()?;
        let b = cfg.seed_weight_max_ppm;
        let rsi_need = cfg.rsi.iter().map(|&n| need_recursive(u64::from(n) + 1, 1, i128::from(n.max(1)), b)).collect();
        let st_need = cfg.st.iter().map(|&(n, _)| need_recursive(u64::from(n), 1, i128::from(n.max(1)), b)).collect();
        let ema_need = cfg
            .ema
            .iter()
            .map(|&n| {
                let seed = if cfg.ema_seed == EmaSeed::Sma { u64::from(n) } else { 1 };
                need_recursive(seed, 2, i128::from(n) + 1, b)
            })
            .collect();
        let atr_need = need_recursive(u64::from(cfg.atr), 1, i128::from(cfg.atr.max(1)), b);
        let out = Readings {
            candle: Candle { start: 0, end: 0, open: 0, high: 0, low: 0, close: 0, volume: 0 },
            day: 0,
            k: 0,
            tod: 0,
            weekday: 0,
            rsi: vec![None; cfg.rsi.len()],
            cci: vec![None; cfg.cci.len()],
            wr: vec![None; cfg.wr.len()],
            bb: vec![None; cfg.bb.len()],
            st: vec![None; cfg.st.len()],
            ema: vec![None; cfg.ema.len()],
            atr: None,
            range_atr: None,
            anat: Anatomy {
                colour: 0,
                range: 0,
                body: 0,
                upper_wick: 0,
                lower_wick: 0,
                body_pct: None,
                upper_pct: None,
                lower_pct: None,
                pat_long: 0,
                pat_short: 0,
                pat_cold: 0,
            },
            rvol: None,
            rsi_ready: vec![false; cfg.rsi.len()],
            st_ready: vec![false; cfg.st.len()],
            ema_ready: vec![false; cfg.ema.len()],
            atr_ready: false,
            since_reset: 0,
        };
        Ok(Self {
            rsi: cfg.rsi.iter().map(|&n| Rsi::new(n)).collect(),
            cci: cfg.cci.iter().map(|&n| Cci::new(n)).collect(),
            wr: cfg.wr.iter().map(|&n| WilliamsR::new(n)).collect(),
            bb: cfg.bb.iter().map(|&(n, k)| Boll::new(n, k)).collect(),
            st: cfg.st.iter().map(|&(n, f)| SuperTrend::new(n, f)).collect(),
            ema: cfg.ema.iter().map(|&n| Ema::new(n, cfg.ema_seed)).collect(),
            atr: Atr::new(cfg.atr),
            anat: AnatomyState::new(cfg.pat),
            rvol: Rvol::new(cfg.rvol_m, cfg.rvol_need, cfg.kmax),
            rsi_need,
            st_need,
            ema_need,
            atr_need,
            since_reset: 0,
            out,
            cfg,
        })
    }

    /// The rows.
    #[must_use]
    pub const fn cfg(&self) -> &SeriesCfg {
        &self.cfg
    }

    /// The readiness `need` of the EMA rows (printed on every result).
    #[must_use]
    pub fn ema_need(&self) -> &[u64] {
        &self.ema_need
    }

    /// The readiness `need` of the RSI rows.
    #[must_use]
    pub fn rsi_need(&self) -> &[u64] {
        &self.rsi_need
    }

    /// The readiness `need` of the Supertrend rows.
    #[must_use]
    pub fn st_need(&self) -> &[u64] {
        &self.st_need
    }

    fn reset_cont(&mut self) {
        self.rsi.iter_mut().for_each(Rsi::reset);
        self.cci.iter_mut().for_each(Cci::reset);
        self.wr.iter_mut().for_each(WilliamsR::reset);
        self.bb.iter_mut().for_each(Boll::reset);
        self.st.iter_mut().for_each(SuperTrend::reset);
        self.ema.iter_mut().for_each(Ema::reset);
        self.atr.reset();
        self.since_reset = 0;
    }

    /// Folds one completed candle. Returns its readings and the step count of this event.
    pub fn fold(&mut self, c: &Candle, ctx: &CandleCtx) -> (&Readings, Steps) {
        let mut st = Steps::default();
        if ctx.cont_reset {
            self.reset_cont();
            st.other += 8;
        }
        if ctx.new_session {
            self.anat.session();
            st.other += 1;
        }
        self.since_reset += 1;
        let o = &mut self.out;
        o.candle = *c;
        o.day = ctx.day;
        o.k = candle_index(c.start, ctx.session_open, self.cfg.tf_s);
        o.tod = ist_sod(c.start);
        o.weekday = weekday(ctx.day);
        for (i, r) in self.rsi.iter_mut().enumerate() {
            st.momentum += r.fold(c.close);
            if let Some(slot) = o.rsi.get_mut(i) {
                *slot = r.reading();
            }
            if let (Some(f), Some(n)) = (o.rsi_ready.get_mut(i), self.rsi_need.get(i)) {
                *f = self.since_reset >= *n && r.reading().is_some();
            }
        }
        for (i, r) in self.cci.iter_mut().enumerate() {
            st.momentum += r.fold(c);
            let (v, s) = r.reading();
            st.momentum += s;
            if let Some(slot) = o.cci.get_mut(i) {
                *slot = v;
            }
        }
        for (i, r) in self.wr.iter_mut().enumerate() {
            st.momentum += r.fold(c);
            if let Some(slot) = o.wr.get_mut(i) {
                *slot = r.reading();
            }
        }
        for (i, r) in self.bb.iter_mut().enumerate() {
            st.vol += r.fold(c.close);
            let (v, s) = r.reading();
            st.vol += s;
            if let Some(slot) = o.bb.get_mut(i) {
                *slot = v;
            }
        }
        for (i, r) in self.st.iter_mut().enumerate() {
            st.trend += r.fold(c);
            if let Some(slot) = o.st.get_mut(i) {
                *slot = r.reading();
            }
            if let (Some(f), Some(n)) = (o.st_ready.get_mut(i), self.st_need.get(i)) {
                *f = self.since_reset >= *n && r.reading().is_some();
            }
        }
        for (i, r) in self.ema.iter_mut().enumerate() {
            st.trend += r.fold(c.close);
            if let Some(slot) = o.ema.get_mut(i) {
                *slot = r.value();
            }
            if let (Some(f), Some(n)) = (o.ema_ready.get_mut(i), self.ema_need.get(i)) {
                *f = self.since_reset >= *n;
            }
        }
        st.trend += self.atr.fold(c);
        o.atr = self.atr.value();
        o.atr_ready = self.since_reset >= self.atr_need && o.atr.is_some();
        o.range_atr = o.atr.and_then(|a| Ratio::new((i128::from(c.high) - i128::from(c.low)) * crate::S, a));
        let (anat, s) = self.anat.fold(c);
        o.anat = anat;
        st.other += s;
        let (rv, s) = self.rvol.fold(o.k, c.volume, ctx.regular_full);
        o.rvol = rv;
        st.other += s;
        o.since_reset = self.since_reset;
        (&self.out, st)
    }
}
