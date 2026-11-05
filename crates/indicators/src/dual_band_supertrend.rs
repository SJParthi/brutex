//! The standard two-band Supertrend, a reading for the PSRB strategy family
//! (PSRB-STV; D-8600 to D-8605).
//!
//! # Why a second Supertrend, and why the first one stays as it is
//!
//! [`crate::trend::SuperTrend`] is a latch. It holds one stop, ratchets it while
//! the trend holds, re-seeds it at the fresh basic band when the trend flips, and
//! decides a flip by comparing the close with the PREVIOUS candle's stop.
//! Vocabulary positions read it, `close_above_supertrend` (64) and
//! `close_below_supertrend` (65) among them, so it is not touched here: not a
//! line of `trend.rs` changes (D-8600).
//!
//! The published Supertrend carries TWO bands. Both ratchet on every candle,
//! whether the trend is using them or not, and the trend turns when the close
//! crosses the CURRENT candle's band. That is `TradingView`'s published script
//! and the standard form TA-Lib's page describes. The two readings differ in band
//! logic alone: the ATR is [`crate::trend::Atr`], reused unchanged, and the band
//! arithmetic is the latch's own (D-8601). Nine fixtures in
//! `dual_band_supertrend_tests.rs` show where they part: the seed, a tighter line
//! after a flip either way, an earlier flip down, an unmoving series, a wide
//! candle, period one, two gaps and two equal bands.
//!
//! This is the published definition, UNVERIFIED against `TradingView`'s own
//! output: no chart's numbers were compared with these readings, and parity here
//! means parity with the published rules.
//!
//! # The rules, per candle
//!
//! With `n` the ATR period, `f` the factor in thousandths, `pu` and `pl` the
//! previous candle's bands (both 0 before the first ATR candle, which is what the
//! script's `nz()` gives a missing value) and `pc` the previous close (`None` at
//! the very first candle, and `None` compares false):
//!
//! ```text
//! mid         = midpoint(high, low)                  floored, as the latch's
//! band        = (atr * f).div_euclid(1000)           atr in whole paisa
//! upper       = mid + band  if mid + band < pu  or  pc > pu,  else pu
//! lower       = mid - band  if mid - band > pl  or  pc < pl,  else pl
//! trend       = Down at the first ATR candle; then, from the PREVIOUS trend:
//!               Down -> Up when close > upper;  Up -> Down when close < lower
//! line        = lower when Up, upper when Down
//! settled     = false at the first ATR candle, true after it
//! ```
//!
//! Nothing is read before the ATR has folded its period: the script plots
//! nothing before its ATR exists. **The previous trend, not a comparison of
//! values, picks the band that is judged (D-8602).** The script asks whether the
//! previous line equalled the previous upper band, which in floating point is the
//! previous trend except where the two bands are exactly equal; whole paisa make
//! equal bands common, and the fixture with two equal bands shows an up candle the
//! literal comparison would turn down. Callers decide only on settled readings
//! (D-8604).
//!
//! # Integers only, and where they part from real arithmetic
//!
//! Prices are paisa `i64` (`CLAUDE.md` section 7). Every band is an `i128`, so no
//! band is ever absent the way the latch's `i64` stop can be. The ATR is floored
//! to whole paisa and so are `mid` and `band`, as the latch floors them; a close
//! within a few paisa of a band can therefore compare differently from the
//! script's floating point (`docs/06-limits.md`, L-PSRB-STV-01).
//!
//! # Cost
//!
//! A fold is one ATR step and a fixed handful of `i128` comparisons on state of a
//! fixed size, asserted below: no history is kept, nothing is scanned, nothing is
//! allocated. The step's p99 is re-measured by `C-PSRB-STV-01` (2,000 to 200,000
//! candles folded) and `C-PSRB-STV-02` (ATR periods 10 to 1,000), both run by
//! `crates/indicators/benches/ratio.rs`: O(1) in candles folded and in the period.
//! The measuring functions live here so the bench wrapper beside that file only
//! sorts, prints and judges.

use std::hint::black_box;
use std::time::Instant;

use crate::trend::{Atr, Trend};
use crate::{Candle, Corrupt, OI_NULL};

/// The widest factor accepted, in thousandths of the ATR: 100x.
pub const MAX_FACTOR_MILLI: u32 = 100_000;

// THE i128 BOUNDS, asserted where the arithmetic is. An ATR in whole paisa is
// below 2^63 (it is an `i64`), and the factor is at most 2^17, so the product is
// at most 2^80 and `band` below 2^71. `mid` lies between two positive `i64`
// prices, so `mid + band` and `mid - band` stay within 2^63 + 2^80 of zero, far
// inside `i128`. No checked or saturating arithmetic is needed below, and none
// hides a wrong value: every profile of this workspace keeps overflow checks on,
// so a bound that failed would stop loudly rather than wrap.
const _: () = assert!(MAX_FACTOR_MILLI <= 1 << 17);
const _: () =
    assert!(matches!((1_i128 << 63).checked_mul(1 << 17), Some(product) if product == 1 << 80));
const _: () = assert!(matches!((1_i128 << 80).checked_add(1 << 63), Some(sum) if sum < 1 << 81));

/// Why a [`DualBandSuperTrend`] was not built, or a candle was not folded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// An ATR period of zero: there is no average of no candles.
    PeriodZero,
    /// A factor of zero, or above [`MAX_FACTOR_MILLI`].
    FactorOutOfRange {
        /// The factor that was asked for, in thousandths.
        milli: u32,
    },
    /// The candle is not one an indicator may fold: the refusal
    /// [`Candle::check_evaluable`] gives, unchanged.
    Corrupt(Corrupt),
}

/// One candle's two-band reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    /// Which band the market is on: `Up` reads the lower band, `Down` the upper.
    pub trend: Trend,
    /// The Supertrend line: `lower` when `Up`, `upper` when `Down`, in paisa.
    pub line: i128,
    /// The upper band, ratcheted, in paisa.
    pub upper: i128,
    /// The lower band, ratcheted, in paisa.
    pub lower: i128,
    /// False at the first ATR candle, whose direction and prior bands are the
    /// script's defaults rather than measurements (D-8604); true after it.
    pub settled: bool,
}

/// The two-band Supertrend over one stream of candles.
///
/// Every field is a fixed-size value, so the state is `Copy` and owns nothing on
/// the heap; its size is asserted below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DualBandSuperTrend {
    /// The engine's own ATR, reused unchanged (D-8601).
    atr: Atr,
    /// The factor in thousandths, widened once at construction.
    factor_milli: i128,
    /// The last folded close, `None` before the first candle.
    previous_close: Option<i64>,
    /// The upper band after the last ATR candle; 0 before the first.
    upper: i128,
    /// The lower band after the last ATR candle; 0 before the first.
    lower: i128,
    /// The trend after the last ATR candle. Read only once an ATR candle has
    /// been folded, so its starting value is never judged.
    trend: Trend,
    /// True once an ATR candle has been folded.
    seen_atr: bool,
    /// The reading after the last candle, `None` until the ATR is warm.
    reading: Option<Reading>,
}

const _: () = assert!(core::mem::size_of::<DualBandSuperTrend>() <= 256);

impl DualBandSuperTrend {
    /// A fresh reading over an ATR of `atr_period` candles, with bands
    /// `factor_milli` thousandths of the ATR from the candle's midpoint.
    ///
    /// # Errors
    ///
    /// [`Refusal::PeriodZero`] for a period of 0, then
    /// [`Refusal::FactorOutOfRange`] naming a factor of 0 or above
    /// [`MAX_FACTOR_MILLI`]. Every period from 1 to `u16::MAX` is accepted.
    pub fn new(atr_period: u16, factor_milli: u32) -> Result<Self, Refusal> {
        if atr_period == 0 {
            return Err(Refusal::PeriodZero);
        }
        if factor_milli == 0 || factor_milli > MAX_FACTOR_MILLI {
            return Err(Refusal::FactorOutOfRange {
                milli: factor_milli,
            });
        }
        Ok(Self {
            atr: Atr::new(i128::from(atr_period)),
            factor_milli: i128::from(factor_milli),
            previous_close: None,
            upper: 0,
            lower: 0,
            trend: Trend::Down,
            seen_atr: false,
            reading: None,
        })
    }

    /// Fold one candle in, by the rules in this module's documentation.
    ///
    /// The candle is validated before anything moves. Its timestamp is not read:
    /// ordering and holes are the caller's.
    ///
    /// # Errors
    ///
    /// [`Refusal::Corrupt`] with the reason [`Candle::check_evaluable`] gives. A
    /// refused candle changes nothing.
    pub fn fold(&mut self, candle: &Candle) -> Result<(), Refusal> {
        candle.check_evaluable().map_err(Refusal::Corrupt)?;
        let mut atr = self.atr;
        atr.fold(candle);
        let Some(average) = atr.value().filter(|_| atr.warm()) else {
            self.atr = atr;
            self.previous_close = Some(candle.close);
            return Ok(());
        };
        let mid = i128::from(candle.high).midpoint(i128::from(candle.low));
        let band = (i128::from(average) * self.factor_milli).div_euclid(1000);
        let basic_upper = mid + band;
        let basic_lower = mid - band;
        let previous_close = self.previous_close.map(i128::from);
        let upper =
            if basic_upper < self.upper || previous_close.is_some_and(|close| close > self.upper) {
                basic_upper
            } else {
                self.upper
            };
        let lower =
            if basic_lower > self.lower || previous_close.is_some_and(|close| close < self.lower) {
                basic_lower
            } else {
                self.lower
            };
        let close = i128::from(candle.close);
        // The PREVIOUS trend picks the band judged, never a comparison of the
        // previous line with the previous band (D-8602).
        let trend = if self.seen_atr {
            match self.trend {
                Trend::Down if close > upper => Trend::Up,
                Trend::Up if close < lower => Trend::Down,
                kept => kept,
            }
        } else {
            Trend::Down
        };
        let line = match trend {
            Trend::Up => lower,
            Trend::Down => upper,
        };
        self.reading = Some(Reading {
            trend,
            line,
            upper,
            lower,
            settled: self.seen_atr,
        });
        self.atr = atr;
        self.previous_close = Some(candle.close);
        self.upper = upper;
        self.lower = lower;
        self.trend = trend;
        self.seen_atr = true;
        Ok(())
    }

    /// The reading after the last candle, or `None` until the ATR has folded
    /// its period.
    #[must_use]
    pub fn reading(&self) -> Option<Reading> {
        self.reading
    }

    /// The reused ATR in whole paisa, once it has folded its period.
    ///
    /// [`Atr::value`] reads a running mean from the first candle on; this reads
    /// only the warm value the bands are built from.
    #[must_use]
    pub fn atr(&self) -> Option<i64> {
        self.atr.value().filter(|_| self.atr.warm())
    }
}

/// One measured size's step cost, read by rank from one round of timings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepCost {
    /// The median step, in nanoseconds.
    pub p50_ns: u64,
    /// The 99th-percentile step, in nanoseconds.
    pub p99_ns: u64,
    /// The slowest step, in nanoseconds.
    pub max_ns: u64,
    /// How many timings were read.
    pub samples: usize,
}

/// Reads one round of timings, already sorted ascending, by nearest rank.
///
/// The sort is the caller's: the bench wrapper sorts, because gate 11 keeps a
/// sort out of this crate's production code. An empty round reads all zeros.
#[must_use]
pub fn summarize(sorted: &[u64]) -> StepCost {
    StepCost {
        p50_ns: at_rank(sorted, 500),
        p99_ns: at_rank(sorted, 990),
        max_ns: sorted.last().copied().unwrap_or(0),
        samples: sorted.len(),
    }
}

/// The nearest-rank value at `permille` of a sorted slice: the smallest value
/// with at least that share of the slice at or below it; 0 for an empty slice.
fn at_rank(sorted: &[u64], permille: usize) -> u64 {
    let rank = sorted.len().saturating_mul(permille).div_ceil(1000);
    sorted.get(rank.saturating_sub(1)).copied().unwrap_or(0)
}

/// Whether every size's p99 stays within `ceiling_permille` thousandths of the
/// first size's and at or under `budget_ns`.
///
/// No costs, or a first p99 of zero, judge false: a step that timed at nothing
/// has been shown absent, not bounded.
#[must_use]
pub fn flat_within(costs: &[StepCost], ceiling_permille: u64, budget_ns: u64) -> bool {
    let Some(first) = costs.first() else {
        return false;
    };
    let base = u128::from(first.p99_ns);
    base > 0
        && costs.iter().all(|cost| {
            u128::from(cost.p99_ns) * 1000 <= base * u128::from(ceiling_permille)
                && cost.p99_ns <= budget_ns
        })
}

/// The ATR period `C-PSRB-STV-01` measures at.
const STEP_PERIOD: u16 = 10;
/// The factor both bench rows measure at, in thousandths: 3x.
const STEP_FACTOR_MILLI: u32 = 3_000;
/// The candles folded before `C-PSRB-STV-02` times a step.
const PERIOD_HISTORY: u64 = 20_000;
/// The price the synthetic candles wander around, in paisa: Rs 2,500.
const SYNTHETIC_BASE: i64 = 250_000;

/// `C-PSRB-STV-01`'s timings: one [`DualBandSuperTrend::fold`] plus
/// [`DualBandSuperTrend::reading`] after `history` candles, at ATR period 10 and
/// factor 3,000. `rounds` vectors of `samples` unsorted nanosecond timings, each
/// one step; the wrapper in `crates/indicators/benches/ratio.rs` sorts, reads and
/// judges them.
///
/// # Errors
///
/// A [`Refusal`] from building or folding, which the constant period and factor
/// and the synthetic candles never give.
pub fn measure_step(history: u64, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal> {
    let mut state = DualBandSuperTrend::new(STEP_PERIOD, STEP_FACTOR_MILLI)?;
    for step in 0..history {
        state.fold(&synthetic(step))?;
    }
    timed(&mut state, history, samples, rounds)
}

/// `C-PSRB-STV-02`'s timings: the same step after 20,000 candles, at ATR period
/// `period` and factor 3,000, in the same form as [`measure_step`].
///
/// # Errors
///
/// [`Refusal::PeriodZero`] for a period of 0; otherwise none, for the reason
/// [`measure_step`] gives.
pub fn measure_period(period: u16, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal> {
    let mut state = DualBandSuperTrend::new(period, STEP_FACTOR_MILLI)?;
    for step in 0..PERIOD_HISTORY {
        state.fold(&synthetic(step))?;
    }
    timed(&mut state, PERIOD_HISTORY, samples, rounds)
}

/// Times `rounds` rounds of `samples` single steps, continuing the synthetic
/// stream at candle `from`. The candle is built before the clock starts.
fn timed(
    state: &mut DualBandSuperTrend,
    from: u64,
    samples: u64,
    rounds: u64,
) -> Result<Vec<Vec<u64>>, Refusal> {
    let mut out = Vec::with_capacity(usize::try_from(rounds).unwrap_or(0));
    let mut step = from;
    for _ in 0..rounds {
        let mut round = Vec::with_capacity(usize::try_from(samples).unwrap_or(0));
        for _ in 0..samples {
            let candle = synthetic(step);
            step = step.wrapping_add(1);
            let start = Instant::now();
            state.fold(black_box(&candle))?;
            black_box(state.reading());
            round.push(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX));
        }
        out.push(round);
    }
    Ok(out)
}

/// The `step`th candle of a deterministic synthetic stream above
/// [`SYNTHETIC_BASE`]. A triangle wave of 400 candles carries the price up 40
/// paisa a candle for 200 candles and down for 200, and a splitmix64 of the step
/// spreads the open and the close up to 256 paisa either side of it and grows
/// wicks up to 127. Every candle is evaluable, and the drift crosses a 3x band
/// both ways, so a measured step meets both trends and their turns. Noise
/// alone never would: without a drift a close does not reach `mid + 3 ATR`.
fn synthetic(step: u64) -> Candle {
    let mut bits = step.wrapping_add(0x9E37_79B9_7F4A_7C15);
    bits = (bits ^ (bits >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    bits = (bits ^ (bits >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    bits ^= bits >> 31;
    let phase = step % 400;
    let level = SYNTHETIC_BASE + 40 * i64::try_from(phase.min(400 - phase)).unwrap_or(0);
    let open = level - 256 + i64::try_from(bits & 0x1FF).unwrap_or(0);
    let close = level - 256 + i64::try_from((bits >> 16) & 0x1FF).unwrap_or(0);
    let wick = i64::try_from((bits >> 32) & 0x7F).unwrap_or(0);
    Candle {
        ts_micros: i64::try_from(step)
            .unwrap_or(i64::MAX)
            .saturating_mul(60_000_000),
        open,
        high: open.max(close) + wick,
        low: open.min(close) - wick,
        close,
        volume: 1,
        open_interest: OI_NULL,
    }
}

#[cfg(test)]
#[path = "dual_band_supertrend_tests.rs"]
mod tests;
