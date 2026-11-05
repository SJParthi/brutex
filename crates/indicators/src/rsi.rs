//! Wilder's RSI over closes, held in exact integers (PSRB-RSI; D-8580 to D-8584).
//!
//! PSRB is the strategy family Pivot R1, Supertrend, RSI and Bollinger Bands, tested
//! on NSE F&O stocks, cash, intraday. This module is its RSI reading and nothing
//! else. It emits no condition bit and holds no vocabulary position: [`Rsi`] is a
//! reading for the F&O strategy path, whose first caller, the strategy part, is not
//! built yet.
//!
//! # The definition: the published one, UNVERIFIED against `TradingView`'s own output
//!
//! With `change = C_t - C_(t-1)` on consecutive closes:
//!
//! ```text
//! avgGain = RMA(max(change, 0), n)        avgLoss = RMA(max(-change, 0), n)
//! RSI     = 100 - 100 / (1 + avgGain / avgLoss), and 100 when avgLoss = 0
//! ```
//!
//! `RMA` is Wilder's average: seeded by the simple mean of its first `n` values, then
//! `avg += (x - avg) / n`. That is `TradingView`'s published form (`ta.rma` seeded by
//! `ta.sma`, as research report A read it on 3 Nov 2026). No output of theirs was
//! compared with this module, so parity here is parity with the published formulas.
//! The 100 when the average loss is 0 is the guard of their built-in RSI script
//! (`down == 0 ? 100 : ...`), read from secondary sources and not from their help
//! page, which leaves the case open (D-8582). [`Reading::flat`] says when both
//! averages are zero, so a caller can treat that reading as undefined instead.
//!
//! # Warm-up
//!
//! The first candle only records its close; each later one is one change. The first
//! reading is after exactly `period` changes, at candle `period` counting from 0,
//! which is where `ta.rma` seeded by `ta.sma` gives its first value. The change
//! counter is a `u32` that saturates at `u32::MAX`, far above every `u16` period, so
//! once past the seed it never meets the period again and a Wilder step never
//! re-seeds (D-8580). A daily reset is a new [`Rsi`]; one carried across sessions
//! keeps smoothing.
//!
//! # Integers only (`CLAUDE.md` section 7)
//!
//! Both averages are held scaled by [`SCALE`], six digits below a paisa, in `i128`.
//! The seed and every Wilder step are floored with `div_euclid`, the form
//! [`Atr`](crate::trend::Atr) uses, so each average stays within `period` units of
//! `10^-6` paisa of the exact Wilder average and never above it. The RSI is a ratio of
//! the two, so its own bound is relative: within `10^6 n / (G + L)` ppm of exact
//! Wilder, which grows without limit as `G + L` shrinks in a long run of unchanged
//! closes. [`Reading::faithful`] is the guard: true only where that bound is within
//! 0.1 RSI point. Comparisons against a [`Level`] are exact cross-multiplications on
//! the integer state, and every product is bounded below `2^127` by the
//! `const` assertions beside the arithmetic (D-8581; `docs/06-limits.md`
//! L-PSRB-RSI-01).
//!
//! # Streaming, and what is not read
//!
//! [`Rsi::fold`] takes one candle, refuses it with [`Candle::check_evaluable`]'s own
//! reason before anything moves, and then reads its close and nothing else. A reading
//! after candle `t` depends on candles `0..=t` and on nothing later, and the current
//! candle is in its own reading; a caller that wants the reading before the current
//! candle keeps the previous one, which is `Copy`. Timestamps are not read: ordering
//! and holes are the caller's (D-8583).
//!
//! # Cost
//!
//! The state is a fixed set of fixed-size fields, `size_of::<Rsi>()` is asserted below,
//! and nothing allocates. One fold is the same few `i128` operations whatever came
//! before it and whatever the period: `C-PSRB-RSI-01` times it after 2,000, 20,000 and
//! 200,000 candles and `C-PSRB-RSI-02` at periods 14, 140 and 1,400, both in
//! `crates/indicators/benches/ratio.rs` (D-8584).

use std::cmp::Ordering;
use std::hint::black_box;
use std::time::Instant;

use crate::trend::SCALE;
use crate::{Candle, Corrupt, OI_NULL};

/// RSI 100 in parts per million of the 0..100 scale: the largest [`Level`] and the
/// largest [`Reading::ppm`].
pub const MAX_LEVEL_PPM: u32 = 1_000_000;

/// The guard of [`Reading::faithful`]: `G + L >= FAITHFUL_FACTOR * period`, both
/// averages in [`SCALE`] units.
///
/// By the ratio bound a reading that size is within 1,000 ppm, 0.1 RSI point, of exact
/// Wilder (D-8581).
pub const FAITHFUL_FACTOR: i128 = 1_000;

/// [`MAX_LEVEL_PPM`] in `i128`: the factor every comparison multiplies the gain by.
const PPM: i128 = 1_000_000;

// THE BOUNDS THAT RULE OUT AN `i128` OVERFLOW (D-8581). Each limit is a power of two
// and each product is taken with `checked_mul` inside the assertion, so a bound that
// did not fit `i128` fails the build instead of wrapping. Every operand below is one
// of these, so no checked or saturating arithmetic is needed in `fold` or `Reading`,
// and none hides a wrong value.
//
// An evaluable close is a positive `i64`, so it lies in `1..2^63`, and a move between
// two of them lies strictly between `-2^63` and `2^63`.
const MOVE_LIMIT: i128 = 1 << (i64::BITS - 1);
// `SCALE` is 10^6 and every level is at most 10^6: both below `2^20`.
const UNIT_LIMIT: i128 = 1 << 20;
// A period is a `u16`: below `2^16`.
const PERIOD_LIMIT: i128 = 1 << u16::BITS;
// A scaled gain or loss is below `2^63 * 2^20 = 2^83`, and so is either average: the
// seed is a mean of such values, and a Wilder step moves an average toward one and
// never past it.
const GAIN_LIMIT: i128 = 1 << 83;
const _: () = assert!(SCALE < UNIT_LIMIT && PPM < UNIT_LIMIT && MAX_LEVEL_PPM == 1_000_000);
const _: () = assert!(matches!(
    MOVE_LIMIT.checked_mul(UNIT_LIMIT),
    Some(GAIN_LIMIT)
));
// A seed sum adds at most `period` of them: below `2^99`.
const _: () = assert!(matches!(GAIN_LIMIT.checked_mul(PERIOD_LIMIT), Some(s) if s == 1 << 99));
// `1_000_000 * G`: below `2^103`.
const _: () = assert!(matches!(GAIN_LIMIT.checked_mul(UNIT_LIMIT), Some(p) if p == 1 << 103));
// `level * (G + L)`: `G + L` is below `2^84` and a level below `2^20`, so the product is
// below `2^104`, inside the `2^105` PARTS.md 3.3 names.
const _: () = assert!(matches!(
    GAIN_LIMIT.checked_mul(2),
    Some(sum) if matches!(sum.checked_mul(UNIT_LIMIT), Some(p) if p <= 1 << 105)
));
// `FAITHFUL_FACTOR * period`: below `2^26`.
const _: () = assert!(matches!(FAITHFUL_FACTOR.checked_mul(PERIOD_LIMIT), Some(f) if f < 1 << 26));

/// Why a call was refused. Every refusal leaves the state exactly as it was (D-8583).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// [`Rsi::new`] was given period 0: an average over no changes is no average.
    PeriodZero,
    /// [`Level::from_ppm`] was given more than [`MAX_LEVEL_PPM`]: an RSI above 100.
    LevelAboveMax {
        /// The value refused, in parts per million.
        ppm: u32,
    },
    /// The candle is not one: its own [`Candle::check_evaluable`] refusal (inverted,
    /// out of range, an overflowing range, a negative volume, a nonpositive price).
    Corrupt(Corrupt),
}

/// An RSI level in parts per million of the 0..100 scale: 700,000 is RSI 70.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level(u32);

impl Level {
    /// A level of `ppm` parts per million, `0..=MAX_LEVEL_PPM`.
    ///
    /// # Errors
    ///
    /// [`Refusal::LevelAboveMax`], naming the value, above [`MAX_LEVEL_PPM`].
    pub const fn from_ppm(ppm: u32) -> Result<Self, Refusal> {
        if ppm > MAX_LEVEL_PPM {
            return Err(Refusal::LevelAboveMax { ppm });
        }
        Ok(Self(ppm))
    }

    /// The level in parts per million, as given.
    #[must_use]
    pub const fn ppm(self) -> u32 {
        self.0
    }
}

/// Wilder's RSI over closes, folded one candle at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rsi {
    /// The smoothing period `n`, `1..=u16::MAX`.
    period: u16,
    /// The last close folded; `None` before the first candle.
    previous: Option<i64>,
    /// Changes folded, saturating at `u32::MAX` (D-8580).
    changes: u32,
    /// The scaled sum of the gains while `changes < period`; the scaled average gain
    /// from the seed on.
    gain: i128,
    /// The scaled sum of the losses while `changes < period`; the scaled average loss
    /// from the seed on.
    loss: i128,
}

const _: () = assert!(core::mem::size_of::<Rsi>() <= 96);

impl Rsi {
    /// A fresh RSI over `period` changes; every period `1..=u16::MAX` is accepted.
    ///
    /// # Errors
    ///
    /// [`Refusal::PeriodZero`] for 0.
    pub fn new(period: u16) -> Result<Self, Refusal> {
        if period == 0 {
            return Err(Refusal::PeriodZero);
        }
        Ok(Self {
            period,
            previous: None,
            changes: 0,
            gain: 0,
            loss: 0,
        })
    }

    /// The period, as given to [`Rsi::new`].
    #[must_use]
    pub const fn period(&self) -> u16 {
        self.period
    }

    /// Fold one candle in: its close, and only its close.
    ///
    /// The first candle only records its close. Each later one is one change: its gain
    /// `max(change, 0)` and loss `max(-change, 0)`, scaled. With the change already
    /// counted, fewer than `period` changes add both to their sums; the `period`-th adds
    /// them and seeds each average with its floored mean; every later one takes the
    /// floored Wilder step `avg += (x - avg).div_euclid(period)`.
    ///
    /// # Errors
    ///
    /// [`Refusal::Corrupt`] with the candle's own [`Candle::check_evaluable`] refusal,
    /// checked before anything moves, so the state is unchanged.
    pub fn fold(&mut self, candle: &Candle) -> Result<(), Refusal> {
        candle.check_evaluable().map_err(Refusal::Corrupt)?;
        let Some(previous) = self.previous.replace(candle.close) else {
            return Ok(());
        };
        // Both closes are positive `i64`s, so the move is inside `MOVE_LIMIT`, a scaled
        // gain or loss inside `GAIN_LIMIT`, and a seed sum inside `2^99` (the bounds above).
        let change = i128::from(candle.close) - i128::from(previous);
        let gain = change.max(0) * SCALE;
        let loss = (-change).max(0) * SCALE;
        self.changes = self.changes.saturating_add(1);
        let n = i128::from(self.period);
        match self.changes.cmp(&u32::from(self.period)) {
            Ordering::Less => {
                self.gain += gain;
                self.loss += loss;
            }
            Ordering::Equal => {
                self.gain = (self.gain + gain).div_euclid(n);
                self.loss = (self.loss + loss).div_euclid(n);
            }
            Ordering::Greater => {
                self.gain += (gain - self.gain).div_euclid(n);
                self.loss += (loss - self.loss).div_euclid(n);
            }
        }
        Ok(())
    }

    /// True once `period` changes have been folded: from candle `period` on, counting
    /// from 0.
    #[must_use]
    pub fn warm(&self) -> bool {
        self.changes >= u32::from(self.period)
    }

    /// The reading after the last candle folded, or `None` until [`Rsi::warm`].
    #[must_use]
    pub fn reading(&self) -> Option<Reading> {
        if !self.warm() {
            return None;
        }
        Some(Reading {
            period: self.period,
            avg_gain: self.gain,
            avg_loss: self.loss,
        })
    }
}

/// One RSI reading: the period it was taken at and both averages, scaled by [`SCALE`].
///
/// `Copy`, so a caller keeps any earlier reading it wants. It carries its period, so a
/// kept copy judges [`Reading::faithful`] by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    period: u16,
    avg_gain: i128,
    avg_loss: i128,
}

impl Reading {
    /// The period of the [`Rsi`] this reading came from.
    #[must_use]
    pub const fn period(&self) -> u16 {
        self.period
    }

    /// The average gain, scaled by [`SCALE`]: paisa times `10^6`.
    #[must_use]
    pub const fn avg_gain_scaled(&self) -> i128 {
        self.avg_gain
    }

    /// The average loss, scaled by [`SCALE`]: paisa times `10^6`.
    #[must_use]
    pub const fn avg_loss_scaled(&self) -> i128 {
        self.avg_loss
    }

    /// Both averages exactly 0.
    ///
    /// The reading is then 100 by the built-in script's guard (D-8582); this says so,
    /// so a caller can treat the reading as undefined, a labelled variant. A long run of
    /// unchanged closes ends here: in exact arithmetic the RSI does not move, but each
    /// floored average loses `ceil(X / n)` per close and reaches 0 after a finite run.
    #[must_use]
    pub const fn flat(&self) -> bool {
        self.avg_gain == 0 && self.avg_loss == 0
    }

    /// `G + L >= FAITHFUL_FACTOR * period`: the reading is within 0.1 RSI point, 1,000
    /// ppm, of exact Wilder.
    ///
    /// The ratio bound puts the floored state within `10^6 n / (G + L)` ppm of exact
    /// Wilder, so at `G + L >= 1_000 n` it is within 1,000 ppm. Both averages zero reads
    /// false, and so does every reading that may be more than 0.1 point off: in a long
    /// run of unchanged closes one average reaches 0 before the other and the reading
    /// is exactly 0 or 100 where the exact RSI has not moved, and that reading is
    /// unfaithful whenever the exact RSI is at least 0.1 and at most 99.9 points. A
    /// departure from an exact RSI within 0.1 point of 0 or 100 can read faithful, and
    /// it is then within 0.1 point of exact (D-8582). The strategy decides only on
    /// faithful readings; [`Reading::ppm`], [`Reading::above`] and [`Reading::below`]
    /// stay exact on the state whatever this says. Not `const`: it widens the period
    /// with `i128::from`.
    #[must_use]
    pub fn faithful(&self) -> bool {
        self.avg_gain + self.avg_loss >= FAITHFUL_FACTOR * i128::from(self.period)
    }

    /// The RSI in parts per million, floored: `floor(10^6 G / (G + L))`, which is
    /// `100 - 100 / (1 + G / L)` in ppm; 1,000,000 when `L == 0`, both averages zero
    /// included (the built-in script's `down == 0 ? 100`).
    ///
    /// For records and seeds. A decision compares the state with [`Reading::above`] and
    /// [`Reading::below`], because the floor can equal a level the state is above.
    #[must_use]
    pub fn ppm(&self) -> u32 {
        if self.avg_loss == 0 {
            return MAX_LEVEL_PPM;
        }
        // With `L > 0` and `G >= 0` the quotient lies in `0..1_000_000`, so it always
        // fits a `u32`; the fallback is the bound itself and is never taken.
        let ratio = (PPM * self.avg_gain).div_euclid(self.avg_gain + self.avg_loss);
        u32::try_from(ratio).unwrap_or(MAX_LEVEL_PPM)
    }

    /// RSI strictly above `level`, exact: `10^6 G > level (G + L)` on the state. Both
    /// averages zero reads as RSI 100, so it is above every level below 1,000,000. A tie
    /// is neither above nor below.
    #[must_use]
    pub fn above(&self, level: Level) -> bool {
        let total = self.avg_gain + self.avg_loss;
        if total == 0 {
            return level.ppm() < MAX_LEVEL_PPM;
        }
        PPM * self.avg_gain > i128::from(level.ppm()) * total
    }

    /// RSI strictly below `level`, exact: `10^6 G < level (G + L)` on the state. Both
    /// averages zero reads as RSI 100, below nothing. A tie is neither above nor below.
    #[must_use]
    pub fn below(&self, level: Level) -> bool {
        PPM * self.avg_gain < i128::from(level.ppm()) * (self.avg_gain + self.avg_loss)
    }
}

// ---- C-PSRB-RSI-01 and C-PSRB-RSI-02: the measuring half of the bench (D-8584).
//
// The timings are taken here and judged in `benches/ratio/psrb_rsi.rs`, which sorts:
// gate 11 keeps every sort out of this crate. The helpers are this part's own copy of
// the F&O form (PARTS.md 2.4: a shared helper would make one part wait on another).

/// One step's cost over one round of samples, read by rank.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepCost {
    /// The median sample, in nanoseconds.
    pub p50_ns: u64,
    /// The 99th-percentile sample, in nanoseconds.
    pub p99_ns: u64,
    /// The largest sample, in nanoseconds.
    pub max_ns: u64,
    /// How many samples the figures were read from.
    pub samples: usize,
}

/// Reads p50, p99 and max from samples sorted ascending, by nearest rank: the `p`
/// quantile of `n` samples is the `ceil(p n)`-th smallest. No sample reads every
/// figure 0, which [`flat_within`] refuses.
#[must_use]
pub fn summarize(sorted: &[u64]) -> StepCost {
    StepCost {
        p50_ns: nearest_rank(sorted, 500),
        p99_ns: nearest_rank(sorted, 990),
        max_ns: sorted.last().copied().unwrap_or(0),
        samples: sorted.len(),
    }
}

/// The sample of rank `ceil(permille * n / 1000)` in `sorted`, or 0 with no sample.
fn nearest_rank(sorted: &[u64], permille: usize) -> u64 {
    let rank = sorted.len().saturating_mul(permille).div_ceil(1_000);
    sorted.get(rank.saturating_sub(1)).copied().unwrap_or(0)
}

/// The verdict on one row's sizes, each read from its best round: every p99 within
/// `ceiling_permille` thousandths of the first size's and inside `budget_ns`.
///
/// No cost, or a p99 of 0 at any size, fails: an operation timed at zero has not been
/// shown cheap, it has been shown absent (the rule of `ratio` in the crate's bench).
#[must_use]
pub fn flat_within(costs: &[StepCost], ceiling_permille: u64, budget_ns: u64) -> bool {
    let Some(first) = costs.first() else {
        return false;
    };
    let limit = u128::from(first.p99_ns) * u128::from(ceiling_permille);
    costs.iter().all(|cost| {
        cost.p99_ns > 0 && u128::from(cost.p99_ns) * 1_000 <= limit && cost.p99_ns <= budget_ns
    })
}

/// The period [`measure_step`] times at: 14, the video's.
const STEP_PERIOD: u16 = 14;

/// The candles [`measure_period`] folds before it times: 20,000.
const PERIOD_HISTORY: u64 = 20_000;

/// `rounds` rounds of `samples` timings of one step at period 14 after `history`
/// candles, in nanoseconds and unsorted: one [`Rsi::fold`], [`Rsi::reading`] and both
/// comparisons of the reading against a level, each sample bracketed by two clock
/// reads. `C-PSRB-RSI-01`, judged in `crates/indicators/benches/ratio.rs`.
///
/// # Errors
///
/// A refusal of [`Rsi::fold`], passed on. The candles are made valid, so none is
/// expected; one would fail the row.
pub fn measure_step(history: u64, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal> {
    measure(STEP_PERIOD, history, samples, rounds)
}

/// The same step at `period` after 20,000 candles: `C-PSRB-RSI-02`, judged in
/// `crates/indicators/benches/ratio.rs`.
///
/// # Errors
///
/// [`Refusal::PeriodZero`] for period 0; a refusal of [`Rsi::fold`], passed on, which
/// the valid candles make unexpected.
pub fn measure_period(period: u16, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal> {
    measure(period, PERIOD_HISTORY, samples, rounds)
}

/// Folds `history` candles into a fresh [`Rsi`] at `period`, then times `rounds` rounds
/// of `samples` steps, the state carried on from one step to the next.
fn measure(period: u16, history: u64, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal> {
    let mut rsi = Rsi::new(period)?;
    let level = Level(MAX_LEVEL_PPM / 2);
    let mut minute = 0_i64;
    for _ in 0..history {
        rsi.fold(&bench_candle(minute))?;
        minute += 1;
    }
    let mut out = Vec::with_capacity(usize::try_from(rounds).unwrap_or(0));
    for _ in 0..rounds {
        let mut round = Vec::with_capacity(usize::try_from(samples).unwrap_or(0));
        for _ in 0..samples {
            let candle = bench_candle(minute);
            minute += 1;
            let start = Instant::now();
            rsi.fold(black_box(&candle))?;
            if let Some(reading) = rsi.reading() {
                black_box((reading.above(level), reading.below(level)));
            }
            round.push(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX));
        }
        out.push(round);
    }
    Ok(out)
}

/// A valid candle at minute `m` whose close wanders: a stride of 7,919 through 4,001
/// prices 13 paisa apart around 25,000.00, so consecutive closes always differ and both
/// averages keep moving. Deterministic: no clock and no randomness, so two runs time the
/// same work.
fn bench_candle(m: i64) -> Candle {
    let close = 2_500_000 + (m.wrapping_mul(7_919).rem_euclid(4_001) - 2_000) * 13;
    Candle {
        ts_micros: m.saturating_mul(60_000_000),
        open: close,
        high: close + 400,
        low: close - 400,
        close,
        volume: 0,
        open_interest: OI_NULL,
    }
}

#[cfg(test)]
#[path = "rsi_tests.rs"]
mod tests;
