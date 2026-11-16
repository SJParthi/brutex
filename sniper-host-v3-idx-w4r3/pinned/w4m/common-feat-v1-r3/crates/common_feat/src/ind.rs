//! The streaming indicator states (TradingView-equivalent definitions, exact integers).
//!
//! LIFT (REQ72-PLAN A.1 "v1 = a lift, not a rewrite"; `LIFT.md` names every source file and its sha256): the
//! arithmetic of the round 3 tree's `crates/indicators` (`rsi.rs`, `cci.rs`, `williams_r.rs`,
//! `dual_band_supertrend.rs`, `trend.rs` `Ema` / `Atr`), carried as written, with these named v1 changes:
//! - every reading is RAW (a [`Ratio`] or the state at `S`), never a level bit;
//! - the Supertrend reads the ATR at `S` (REQ72-DESIGN A.4) instead of the paisa-rounded ATR (the OLD reading is
//!   `--features red`);
//! - %R with a zero range and CCI with a zero mean deviation are `None` (`wr_zero_range`, `cci_zero_dev`, K6) instead
//!   of a value (the OLD reading under `red` is 0);
//! - Bollinger (REQ72-DESIGN VB-BB: population variance, SMA basis) keeps `n`, `sum x`, `sum x^2` exactly; band values
//!   for display use [`isqrt`], and every bit squares both sides;
//! - the EMA seed is a data row: `Sma` (the lift: the running mean of the first n closes, `indicators::Ema`) or
//!   `First` (Pine's `ta.ema`: the first close); both converge to the same value.
//!
//! Definitions (TradingView): RSI = Wilder (rma, SMA-seeded) of gains and losses; CCI on hlc3 with the mean absolute
//! deviation; %R over the last n highs and lows, this candle included; ATR = rma of the true range (the first candle's
//! TR = H - L); Supertrend = Pine `ta.supertrend` (hl2 +/- m ATR, latched bands, flip on a close beyond the band).

use std::collections::VecDeque;

use crate::S;
use crate::fold::Candle;
use crate::num::{Ratio, isqrt};

/// RSI n (Wilder), lifted from `indicators::rsi::Rsi`.
#[derive(Clone, Debug)]
pub struct Rsi {
    period: i128,
    previous: Option<i64>,
    changes: u32,
    gain: i128,
    loss: i128,
}

impl Rsi {
    /// A new RSI of `period` (>= 1).
    #[must_use]
    pub fn new(period: u16) -> Self {
        Self { period: i128::from(period.max(1)), previous: None, changes: 0, gain: 0, loss: 0 }
    }

    /// Folds one close. Returns its step count (6).
    pub fn fold(&mut self, close: i64) -> u32 {
        let Some(previous) = self.previous.replace(close) else {
            return 1;
        };
        let change = i128::from(close) - i128::from(previous);
        let gain = change.max(0) * S;
        let loss = (-change).max(0) * S;
        self.changes = self.changes.saturating_add(1);
        let n = self.period;
        match i128::from(self.changes).cmp(&n) {
            std::cmp::Ordering::Less => {
                self.gain += gain;
                self.loss += loss;
            }
            std::cmp::Ordering::Equal => {
                self.gain = (self.gain + gain).div_euclid(n);
                self.loss = (self.loss + loss).div_euclid(n);
            }
            std::cmp::Ordering::Greater => {
                self.gain += (gain - self.gain).div_euclid(n);
                self.loss += (loss - self.loss).div_euclid(n);
            }
        }
        6
    }

    /// Clears the state (a CONT reset at a factor ex-date, K13 X3).
    pub fn reset(&mut self) {
        *self = Self::new(u16::try_from(self.period).unwrap_or(14));
    }

    /// The Wilder averages (gain, loss) at `S`, once `period` changes are folded.
    #[must_use]
    pub fn sums(&self) -> Option<(i128, i128)> {
        (i128::from(self.changes) >= self.period).then_some((self.gain, self.loss))
    }

    /// RSI as the exact ratio `100 G / (G + L)`; `None` before warm-up and on a flat window (G = L = 0: not ready,
    /// K6; OBF34: a flat window sets neither the RSI 100 nor the RSI 0 bit).
    #[must_use]
    pub fn reading(&self) -> Option<Ratio> {
        let (g, l) = self.sums()?;
        if g + l == 0 {
            if cfg!(feature = "m_rsi_flat_ready") {
                return Ratio::new(100, 1);
            }
            return None;
        }
        Ratio::new(100 * g, g + l)
    }
}

/// The seed of an EMA (a data row).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmaSeed {
    /// The running mean of the first n values (the lift, `indicators::Ema`).
    Sma,
    /// The first value (Pine's `ta.ema`).
    First,
}

/// EMA n at `S`, lifted from `indicators::trend::Ema` (`Sma` seed) with Pine's first-value seed as a data row.
#[derive(Clone, Debug)]
pub struct Ema {
    period: i128,
    seed: EmaSeed,
    scaled: i128,
    seed_sum: i128,
    folded: u64,
}

impl Ema {
    /// A new EMA.
    #[must_use]
    pub fn new(period: u16, seed: EmaSeed) -> Self {
        Self { period: i128::from(period.max(1)), seed, scaled: 0, seed_sum: 0, folded: 0 }
    }

    /// Folds one value (paisa). Returns its step count (3).
    pub fn fold(&mut self, price: i64) -> u32 {
        self.folded = self.folded.saturating_add(1);
        let target = i128::from(price) * S;
        let count = i128::from(self.folded);
        match self.seed {
            EmaSeed::Sma if count <= self.period => {
                self.seed_sum += target;
                self.scaled = self.seed_sum.div_euclid(count.max(1));
                return 3;
            }
            EmaSeed::First if count == 1 => {
                self.scaled = target;
                return 3;
            }
            _ => {}
        }
        let delta = target - self.scaled;
        self.scaled += (delta * 2).div_euclid(self.period + 1);
        3
    }

    /// Values folded so far.
    #[must_use]
    pub const fn folded(&self) -> u64 {
        self.folded
    }

    /// The EMA at `S` (paisa x 10^6), or `None` before the first value.
    #[must_use]
    pub const fn value(&self) -> Option<i128> {
        if self.folded == 0 { None } else { Some(self.scaled) }
    }

    /// Clears the state.
    pub fn reset(&mut self) {
        *self = Self::new(u16::try_from(self.period).unwrap_or(1), self.seed);
    }
}

/// ATR n (rma of the true range, SMA-seeded) at `S`, lifted from `indicators::trend::Atr`.
#[derive(Clone, Debug)]
pub struct Atr {
    period: i128,
    scaled: i128,
    seed_sum: i128,
    previous_close: Option<i64>,
    folded: u64,
}

impl Atr {
    /// A new ATR.
    #[must_use]
    pub fn new(period: u16) -> Self {
        Self { period: i128::from(period.max(1)), scaled: 0, seed_sum: 0, previous_close: None, folded: 0 }
    }

    fn true_range(&self, c: &Candle) -> i128 {
        let hl = i128::from(c.high) - i128::from(c.low);
        let Some(p) = self.previous_close else {
            return hl;
        };
        let p = i128::from(p);
        let hc = (i128::from(c.high) - p).abs();
        let lc = (p - i128::from(c.low)).abs();
        hl.max(hc).max(lc)
    }

    /// Folds one candle. Returns its step count (4).
    pub fn fold(&mut self, c: &Candle) -> u32 {
        self.folded = self.folded.saturating_add(1);
        let tr = self.true_range(c) * S;
        let count = i128::from(self.folded);
        if count > self.period {
            self.scaled += (tr - self.scaled).div_euclid(self.period);
        } else {
            self.seed_sum += tr;
            self.scaled = self.seed_sum.div_euclid(count.max(1));
        }
        self.previous_close = Some(c.close);
        4
    }

    /// The ATR at `S` once `period` candles are folded.
    #[must_use]
    pub fn value(&self) -> Option<i128> {
        (i128::from(self.folded) >= self.period).then_some(self.scaled)
    }

    /// Clears the state.
    pub fn reset(&mut self) {
        *self = Self::new(u16::try_from(self.period).unwrap_or(1));
    }
}

/// A Supertrend reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StReading {
    /// True = green (up trend: the line is the lower band).
    pub up: bool,
    /// The line at `S`.
    pub line: i128,
    /// The final upper band at `S`.
    pub upper: i128,
    /// The final lower band at `S`.
    pub lower: i128,
    /// The side changed on this candle.
    pub flip: bool,
}

/// Supertrend (ATR period, factor in milli), the dual-band `TradingView` form, lifted from
/// `indicators::dual_band_supertrend::DualBandSuperTrend` (latch and flip rules as written; ATR at `S`).
#[derive(Clone, Debug)]
pub struct SuperTrend {
    atr: Atr,
    factor_milli: i128,
    previous_close: Option<i64>,
    upper: i128,
    lower: i128,
    up: bool,
    seen_atr: bool,
    reading: Option<StReading>,
}

impl SuperTrend {
    /// A new Supertrend.
    #[must_use]
    pub fn new(atr_period: u16, factor_milli: u32) -> Self {
        Self {
            atr: Atr::new(atr_period),
            factor_milli: i128::from(factor_milli),
            previous_close: None,
            upper: 0,
            lower: 0,
            up: false,
            seen_atr: false,
            reading: None,
        }
    }

    /// Folds one candle. Returns its step count (10).
    pub fn fold(&mut self, c: &Candle) -> u32 {
        let mut steps = self.atr.fold(c);
        let Some(average) = self.atr.value() else {
            self.previous_close = Some(c.close);
            return steps + 1;
        };
        // OLD (red): the paisa-rounded ATR of the lifted crate (`Atr::value` rounds to paisa there).
        let average = if cfg!(feature = "red") { (average.div_euclid(S)) * S } else { average };
        let mid = (i128::from(c.high) + i128::from(c.low)) * S / 2;
        let band = (average * self.factor_milli).div_euclid(1000);
        let basic_upper = mid + band;
        let basic_lower = mid - band;
        let pc = self.previous_close.map(|p| i128::from(p) * S);
        let upper = if basic_upper < self.upper || pc.is_some_and(|p| p > self.upper) { basic_upper } else { self.upper };
        let lower = if basic_lower > self.lower || pc.is_some_and(|p| p < self.lower) { basic_lower } else { self.lower };
        let close = i128::from(c.close) * S;
        let was = self.up;
        let up = if self.seen_atr {
            if self.up { close >= lower } else { close > upper }
        } else {
            false
        };
        let line = if up { lower } else { upper };
        self.reading = Some(StReading { up, line, upper, lower, flip: self.seen_atr && up != was });
        self.previous_close = Some(c.close);
        self.upper = upper;
        self.lower = lower;
        self.up = up;
        self.seen_atr = true;
        steps += 6;
        steps
    }

    /// The reading of the last candle, once the ATR is warm.
    #[must_use]
    pub const fn reading(&self) -> Option<StReading> {
        self.reading
    }

    /// Clears the state.
    pub fn reset(&mut self) {
        let f = u32::try_from(self.factor_milli).unwrap_or(3000);
        let p = u16::try_from(self.atr.period).unwrap_or(10);
        *self = Self::new(p, f);
    }
}

/// CCI n on hlc3 with the mean absolute deviation, lifted from `indicators::cci::Cci`.
#[derive(Clone, Debug)]
pub struct Cci {
    ring: VecDeque<i128>,
    sum: i128,
    last: i128,
    period: u16,
}

impl Cci {
    /// A new CCI.
    #[must_use]
    pub fn new(period: u16) -> Self {
        let period = period.clamp(1, 64);
        Self { ring: VecDeque::with_capacity(usize::from(period)), sum: 0, last: 0, period }
    }

    /// Folds one candle. Returns its step count (2).
    pub fn fold(&mut self, c: &Candle) -> u32 {
        let typical = i128::from(c.high) + i128::from(c.low) + i128::from(c.close);
        if self.ring.len() == usize::from(self.period) {
            self.sum -= self.ring.pop_front().unwrap_or(0);
        }
        self.ring.push_back(typical);
        self.sum += typical;
        self.last = typical;
        2
    }

    /// CCI as the exact ratio `1000 n (n TP - sum TP) / (15 sum |n TP_i - sum TP|)` (TP = H + L + C), with the step
    /// count of the O(n) deviation pass (the named bounded exception, n + 2). `None` before n candles, and on a zero
    /// mean deviation (`cci_zero_dev`).
    #[must_use]
    pub fn reading(&self) -> (Option<Ratio>, u32) {
        if self.ring.len() != usize::from(self.period) {
            return (None, 1);
        }
        let n = i128::from(self.period);
        let total: i128 = self.ring.iter().map(|&t| (n * t - self.sum).abs()).sum();
        let steps = u32::from(self.period) + 2;
        let dev = n * self.last - self.sum;
        if total == 0 {
            if cfg!(feature = "red") {
                return (Ratio::new(0, 1), steps);
            }
            return (None, steps);
        }
        (Ratio::new(1000 * n * dev, 15 * total), steps)
    }

    /// Clears the state.
    pub fn reset(&mut self) {
        *self = Self::new(self.period);
    }
}

/// Williams %R n, lifted from `indicators::williams_r::WilliamsR` (monotone deques).
#[derive(Clone, Debug)]
pub struct WilliamsR {
    period: u64,
    highs: VecDeque<(u64, i64)>,
    lows: VecDeque<(u64, i64)>,
    next: u64,
    seen: u64,
    close: i64,
}

impl WilliamsR {
    /// A new %R.
    #[must_use]
    pub fn new(period: u16) -> Self {
        let cap = usize::from(period.max(1));
        Self {
            period: u64::from(period.max(1)),
            highs: VecDeque::with_capacity(cap),
            lows: VecDeque::with_capacity(cap),
            next: 0,
            seen: 0,
            close: 0,
        }
    }

    /// Folds one candle. Returns its step count (amortised O(1): 2 + the pops of this call, at most 2n).
    pub fn fold(&mut self, c: &Candle) -> u32 {
        let number = self.next;
        let mut steps = 2u32;
        if self.highs.front().is_some_and(|&(t, _)| number - t >= self.period) {
            self.highs.pop_front();
            steps += 1;
        }
        if self.lows.front().is_some_and(|&(t, _)| number - t >= self.period) {
            self.lows.pop_front();
            steps += 1;
        }
        while self.highs.back().is_some_and(|&(_, h)| h <= c.high) {
            self.highs.pop_back();
            steps += 1;
        }
        self.highs.push_back((number, c.high));
        while self.lows.back().is_some_and(|&(_, l)| l >= c.low) {
            self.lows.pop_back();
            steps += 1;
        }
        self.lows.push_back((number, c.low));
        self.next = number + 1;
        self.seen += 1;
        self.close = c.close;
        steps
    }

    /// (highest high, lowest low) of the last n candles, once warm.
    #[must_use]
    pub fn hh_ll(&self) -> Option<(i64, i64)> {
        if self.seen < self.period {
            return None;
        }
        Some((self.highs.front().map_or(self.close, |x| x.1), self.lows.front().map_or(self.close, |x| x.1)))
    }

    /// %R as the exact ratio `-100 (HH - C) / (HH - LL)`; `None` before warm-up and on a zero range (`wr_zero_range`).
    #[must_use]
    pub fn reading(&self) -> Option<Ratio> {
        let (hh, ll) = self.hh_ll()?;
        if hh == ll {
            if cfg!(feature = "red") {
                return Ratio::new(0, 1);
            }
            return None;
        }
        Ratio::new(-100 * (i128::from(hh) - i128::from(self.close)), i128::from(hh) - i128::from(ll))
    }

    /// Clears the state.
    pub fn reset(&mut self) {
        *self = Self::new(u16::try_from(self.period).unwrap_or(14));
    }
}

/// A Bollinger reading: the exact sums and the display values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbReading {
    /// n.
    pub n: i128,
    /// sum of the closes (paisa).
    pub sum: i128,
    /// n sum x^2 - (sum x)^2 (paisa^2): n^2 times the population variance.
    pub var_n2: i128,
    /// The basis (SMA) at `S`.
    pub basis: i128,
    /// The upper band at `S` (display: through [`isqrt`]).
    pub upper: i128,
    /// The lower band at `S` (display).
    pub lower: i128,
    /// The last close (paisa).
    pub close: i64,
    /// The multiplier, milli.
    pub k_milli: i128,
}

impl BbReading {
    /// The close is beyond the upper band by at least `p_bp` basis points of the band (`p_bp` may be 0: beyond at all,
    /// strictly). Exact: both sides squared.
    #[must_use]
    pub fn above_upper_by(&self, p_bp: i128) -> bool {
        // 10^4 n x - (10^4 + p) sum >= (10^4 + p) (k_milli / 1000) sqrt(var_n2)
        let a = 10_000 + p_bp;
        let lhs = 10_000 * self.n * i128::from(self.close) - a * self.sum;
        if lhs < 0 || (p_bp == 0 && lhs == 0) {
            return false;
        }
        let r = a * self.k_milli;
        let l2 = lhs * lhs * 1_000_000;
        if p_bp == 0 { l2 > r * r * self.var_n2 } else { l2 >= r * r * self.var_n2 }
    }

    /// The close is beyond the lower band by at least `p_bp` (the mirror).
    #[must_use]
    pub fn below_lower_by(&self, p_bp: i128) -> bool {
        // (10^4 - p) sum - 10^4 n x >= (10^4 - p) k sqrt(var)
        let a = 10_000 - p_bp;
        let lhs = a * self.sum - 10_000 * self.n * i128::from(self.close);
        if lhs < 0 || (p_bp == 0 && lhs == 0) {
            return false;
        }
        let r = a * self.k_milli;
        let l2 = lhs * lhs * 1_000_000;
        if p_bp == 0 { l2 > r * r * self.var_n2 } else { l2 >= r * r * self.var_n2 }
    }

    /// Band width (upper - lower) / basis, as bp, at least `w_bp`. Exact: 2 k sqrt(var) 10^4 >= w sum (squared).
    #[must_use]
    pub fn width_ge(&self, w_bp: i128) -> bool {
        let l = 2 * self.k_milli * 10_000;
        let r = w_bp * 1000 * self.sum;
        l * l * self.var_n2 >= r * r
    }
}

/// Bollinger n x k (SMA basis, population standard deviation).
#[derive(Clone, Debug)]
pub struct Boll {
    ring: VecDeque<i64>,
    n: u16,
    k_milli: i128,
    sum: i128,
    sum2: i128,
}

impl Boll {
    /// A new Bollinger.
    #[must_use]
    pub fn new(n: u16, k_milli: u32) -> Self {
        let n = n.max(1);
        Self { ring: VecDeque::with_capacity(usize::from(n)), n, k_milli: i128::from(k_milli), sum: 0, sum2: 0 }
    }

    /// Folds one close. Returns its step count (4).
    pub fn fold(&mut self, close: i64) -> u32 {
        if self.ring.len() == usize::from(self.n)
            && let Some(old) = self.ring.pop_front()
        {
            self.sum -= i128::from(old);
            self.sum2 -= i128::from(old) * i128::from(old);
        }
        self.ring.push_back(close);
        self.sum += i128::from(close);
        self.sum2 += i128::from(close) * i128::from(close);
        4
    }

    /// The reading once n closes are held, with its step count (4: the square root is display only).
    #[must_use]
    pub fn reading(&self) -> (Option<BbReading>, u32) {
        if self.ring.len() != usize::from(self.n) {
            return (None, 1);
        }
        let n = i128::from(self.n);
        let var_n2 = n * self.sum2 - self.sum * self.sum;
        let basis = (self.sum * S).div_euclid(n);
        // sd at S = sqrt(var_n2) / n x S = isqrt(var_n2 S^2) / n
        let sd_s = isqrt(var_n2 * S * S);
        let band = (self.k_milli * sd_s).div_euclid(1000 * n);
        let close = self.ring.back().copied().unwrap_or(0);
        (
            Some(BbReading {
                n,
                sum: self.sum,
                var_n2,
                basis,
                upper: basis + band,
                lower: basis - band,
                close,
                k_milli: self.k_milli,
            }),
            4,
        )
    }

    /// Clears the state.
    pub fn reset(&mut self) {
        *self = Self::new(self.n, u32::try_from(self.k_milli).unwrap_or(2000));
    }
}
