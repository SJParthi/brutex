//! Williams %R over the last `period` candles, held in exact integers (PSRB-WILLR; D-8640
//! to D-8644).
//!
//! PSRB is the strategy family narrow CPR and Pivot R1, Supertrend, Bollinger Bands, RSI,
//! Williams %R, CCI and EMA 200, tested on NSE F&O stocks, cash, intraday. This module is its
//! Williams %R reading and nothing else. It emits no condition bit and holds no vocabulary
//! position: [`WilliamsR`] is a reading for the F&O strategy path, whose first caller, the
//! strategy part, is not built yet.
//!
//! # The definition: the published one, UNVERIFIED against `TradingView`'s own output
//!
//! With `HH` the highest high and `LL` the lowest low of the last `n` candles, the current
//! one included, and `C` the current close:
//!
//! ```text
//! %R = 100 * (C - HH) / (HH - LL) = -100 * (HH - C) / (HH - LL)
//! ```
//!
//! so `%R` lies in `-100..=0`: 0 with the close on the window's high, -100 on its low. That
//! is the form of `TradingView`'s built-in Williams %R script as the PSRB design states it;
//! neither their page nor their output was read or compared here, so parity is parity with
//! the published definition. A window whose highest high equals its lowest low, a window of
//! one price, has no value: where the script divides by zero and gives `na` (UNVERIFIED),
//! [`Reading::flat`] is true, [`Reading::ppm`] is `None` and every comparison and crossing
//! is false (D-8641).
//!
//! # Units and comparisons
//!
//! A [`Level`] is in parts per million of the 0..-100 scale, so -85 is -850,000, from
//! [`MIN_LEVEL_PPM`] to [`MAX_LEVEL_PPM`]. With `U = HH - C` and `R = HH - LL`, both
//! `0 <= U <= R`, the reading is above level `l` exactly when `10^6 U < -l R` and below it
//! exactly when `10^6 U > -l R`, compared in `i128` on the integers themselves: no
//! fixed-point scale, no rounding, and a tie is neither. [`Reading::ppm`] is the floor of
//! the exact value, for records and seeds only. Every product is bounded below `2^83` by the
//! `const` assertions beside the arithmetic (D-8641). The crossings are `TradingView`'s
//! `ta.crossover` and `ta.crossunder`: the previous reading at or on the other side of the
//! level and the current one strictly beyond it (D-8642).
//!
//! # The window: two monotonic deques, allocated once
//!
//! `highs` keeps the candles whose high no later candle in the window has met or beaten,
//! oldest first, so its front is `HH`; `lows` mirrors it for `LL`. Each fold first expires
//! at most one entry per deque, the one exactly `period` candles old (numbers rise by one
//! per fold and the front is the oldest); then it pops from the back every entry the new
//! candle's high meets or beats (its low, for `lows`), and pushes the new candle once. So a
//! deque never holds more than `period` entries, the capacity `period` reserved in
//! [`WilliamsR::new`] is never exceeded, and a state built by `new` never reallocates. A
//! clone reserves only its length; the measured path never clones (D-8643).
//!
//! # Warm-up and the counters
//!
//! The first reading is after exactly `period` candles, at candle `period - 1` counting
//! from 0, where `ta.highest(n)` and `ta.lowest(n)` first have a value. Each candle is
//! numbered by a `u64` that wraps, and an age is the wrapping difference, exact while no
//! entry is `2^64` candles old, which none can be at a `u16` period. The warm count is a
//! `u32` that saturates at `u32::MAX`, far above every period, so a warm state never turns
//! cold (D-8643). A daily reset is a new [`WilliamsR`]; one carried across sessions keeps its
//! window across the boundary (D-8640).
//!
//! # Streaming, and what is not read
//!
//! [`WilliamsR::fold`] takes one candle, refuses it with [`Candle::check_evaluable`]'s own
//! reason before anything moves, and then reads its high, low and close and nothing else. A
//! reading after candle `t` depends on candles `0..=t` only, and the current candle is in
//! its own reading; a caller that wants the reading before the current candle keeps the
//! previous one, which is `Copy`. Timestamps are not read: ordering and holes are the
//! caller's (D-8643).
//!
//! # Cost
//!
//! Per candle and per deque: one expiry check, at most one expiry, the back-pops, one push.
//! Each entry is pushed once and removed at most once, so over any run the removals are at
//! most the candles folded: amortized O(1). At the 99th percentile a deque removes at most
//! 100 entries whatever the period, by counting (if more than 1% of candles each removed
//! more than 100, the removals would exceed the pushes); the worst single candle removes
//! `period`, a staircase of falling highs and then a jump (`docs/06-limits.md`
//! L-PSRB-WILLR-01). `C-PSRB-WILLR-01` times one step after 2,000, 20,000 and 200,000
//! candles and `C-PSRB-WILLR-02` at periods 14, 140 and 1,400, both in
//! `crates/indicators/benches/ratio.rs` (D-8644).

use std::collections::VecDeque;
use std::hint::black_box;
use std::time::Instant;

use crate::{Candle, Corrupt, OI_NULL};

/// %R -100, the close on the window's low, in parts per million of the 0..-100 scale: the
/// smallest [`Level`] and the smallest [`Reading::ppm`].
pub const MIN_LEVEL_PPM: i32 = -1_000_000;

/// %R 0, the close on the window's high: the largest [`Level`] and the largest
/// [`Reading::ppm`]. -85 is -850,000.
pub const MAX_LEVEL_PPM: i32 = 0;

/// The 100 points of the scale in parts per million, `10^6`: the factor every comparison
/// multiplies `HH - C` by.
const PPM: i128 = 1_000_000;

// THE BOUNDS THAT RULE OUT AN `i128` OVERFLOW (D-8641). Each limit is a power of two and
// each product is taken with `checked_mul` inside the assertion, so a bound that did not fit
// `i128` fails the build instead of wrapping. Every operand in `Reading` is inside one of
// these, so no checked or saturating arithmetic is needed there, and none hides a wrong value.
//
// An evaluable candle's prices are positive `i64`s, so each lies in `1..2^63`. The window's
// highest high is at least the current high and its lowest low at most the current low, and
// the close lies between the two, so `U = HH - C` and `R = HH - LL` satisfy `0 <= U <= R < 2^63`.
const SPAN_LIMIT: i128 = 1 << (i64::BITS - 1);
// `10^6` and every `|level|` are at most `10^6`: below `2^20`.
const UNIT_LIMIT: i128 = 1 << 20;
// So `10^6 U`, `-l R` and the floor's dividend `-10^6 U` are each below `2^83` in size, far
// inside `i128` (`2^127`).
const PRODUCT_LIMIT: i128 = 1 << 83;
const _: () = assert!(PPM < UNIT_LIMIT && MIN_LEVEL_PPM == -1_000_000 && MAX_LEVEL_PPM == 0);
const _: () = assert!(matches!(
    SPAN_LIMIT.checked_mul(UNIT_LIMIT),
    Some(PRODUCT_LIMIT)
));
// The difference of two such products, the widest value any comparison could form, still
// fits: below `2^84`.
const _: () = assert!(PRODUCT_LIMIT.checked_mul(2).is_some());

/// Why a call was refused. Every refusal leaves the state exactly as it was (D-8643).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// [`WilliamsR::new`] was given period 0: a window of no candles has no high.
    PeriodZero,
    /// [`Level::from_ppm`] was given a value above [`MAX_LEVEL_PPM`] or below
    /// [`MIN_LEVEL_PPM`]: a %R above 0 or below -100.
    LevelOutOfRange {
        /// The value refused, in parts per million.
        ppm: i32,
    },
    /// The candle is not one: its own [`Candle::check_evaluable`] refusal (inverted, a
    /// price outside the range, an overflowing range, a negative volume, a nonpositive
    /// price).
    Corrupt(Corrupt),
}

/// A Williams %R level in parts per million of the 0..-100 scale: -850,000 is %R -85.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level(i32);

impl Level {
    /// A level of `ppm` parts per million, `MIN_LEVEL_PPM..=MAX_LEVEL_PPM`.
    ///
    /// # Errors
    ///
    /// [`Refusal::LevelOutOfRange`], naming the value, below [`MIN_LEVEL_PPM`] or above
    /// [`MAX_LEVEL_PPM`].
    pub const fn from_ppm(ppm: i32) -> Result<Self, Refusal> {
        if ppm < MIN_LEVEL_PPM || ppm > MAX_LEVEL_PPM {
            return Err(Refusal::LevelOutOfRange { ppm });
        }
        Ok(Self(ppm))
    }

    /// The level in parts per million, as given.
    #[must_use]
    pub const fn ppm(self) -> i32 {
        self.0
    }
}

/// Williams %R over the last `period` candles, folded one candle at a time.
///
/// Not `Copy`: it owns its two deques, each allocated once in [`WilliamsR::new`] with
/// capacity `period`. A clone reserves only its length, so a cloned state reallocates as its
/// window fills; the measured path never clones (D-8643).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WilliamsR {
    /// The window, in candles, `1..=u16::MAX`.
    period: u16,
    /// `(candle number, high)`, oldest first, the highs falling strictly front to back:
    /// the front is the window's highest high.
    highs: VecDeque<(u64, i64)>,
    /// `(candle number, low)`, oldest first, the lows rising strictly front to back: the
    /// front is the window's lowest low.
    lows: VecDeque<(u64, i64)>,
    /// The next candle's number, wrapping at `u64::MAX` (D-8643).
    next: u64,
    /// Candles folded, saturating at `u32::MAX` (D-8643).
    seen: u32,
    /// The last candle's close; 0 before the first.
    close: i64,
}

const _: () = assert!(core::mem::size_of::<WilliamsR>() <= 128);

impl WilliamsR {
    /// A fresh state over the last `period` candles; every period `1..=u16::MAX` is
    /// accepted. Both deques are allocated here, each with capacity `period`, and never
    /// again by folding.
    ///
    /// # Errors
    ///
    /// [`Refusal::PeriodZero`] for 0.
    pub fn new(period: u16) -> Result<Self, Refusal> {
        if period == 0 {
            return Err(Refusal::PeriodZero);
        }
        let capacity = usize::from(period);
        Ok(Self {
            period,
            highs: VecDeque::with_capacity(capacity),
            lows: VecDeque::with_capacity(capacity),
            next: 0,
            seen: 0,
            close: 0,
        })
    }

    /// The period, as given to [`WilliamsR::new`].
    #[must_use]
    pub const fn period(&self) -> u16 {
        self.period
    }

    /// Fold one candle in: its high, low and close, and nothing else.
    ///
    /// With `s` this candle's number and `n` the period, each deque first drops its front
    /// if that entry is `n` or more candles old (`s - t >= n`, wrapping), which only the
    /// entry numbered `s - n` can be. Then `highs` pops from the back every entry whose high
    /// is at most this candle's high and pushes `(s, high)`, and `lows` pops every entry
    /// whose low is at least this candle's low and pushes `(s, low)`. Then the number
    /// advances, wrapping, the warm count advances, saturating, and the close is kept.
    /// Expiry runs first, so before the push a deque holds at most `n - 1` entries and after
    /// it at most `n`.
    ///
    /// # Errors
    ///
    /// [`Refusal::Corrupt`] with the candle's own [`Candle::check_evaluable`] refusal,
    /// checked before anything moves, so the state is unchanged.
    pub fn fold(&mut self, candle: &Candle) -> Result<(), Refusal> {
        candle.check_evaluable().map_err(Refusal::Corrupt)?;
        let number = self.next;
        let period = u64::from(self.period);
        if self
            .highs
            .front()
            .is_some_and(|&(t, _)| number.wrapping_sub(t) >= period)
        {
            self.highs.pop_front();
        }
        if self
            .lows
            .front()
            .is_some_and(|&(t, _)| number.wrapping_sub(t) >= period)
        {
            self.lows.pop_front();
        }
        while self
            .highs
            .back()
            .is_some_and(|&(_, high)| high <= candle.high)
        {
            self.highs.pop_back();
        }
        self.highs.push_back((number, candle.high));
        while self.lows.back().is_some_and(|&(_, low)| low >= candle.low) {
            self.lows.pop_back();
        }
        self.lows.push_back((number, candle.low));
        self.next = number.wrapping_add(1);
        self.seen = self.seen.saturating_add(1);
        self.close = candle.close;
        Ok(())
    }

    /// True once `period` candles have been folded: from candle `period - 1` on, counting
    /// from 0. Not `const`: it widens the period with `u32::from`.
    #[must_use]
    pub fn warm(&self) -> bool {
        self.seen >= u32::from(self.period)
    }

    /// The reading after the last candle folded, or `None` until [`WilliamsR::warm`].
    #[must_use]
    pub fn reading(&self) -> Option<Reading> {
        if !self.warm() {
            return None;
        }
        // Neither front is `None` here: the fold that made the state warm pushed onto both
        // deques, and a fold that pops from a deque pushes onto it before it returns, so a
        // deque that has folded a candle is never empty. The default, the close, never
        // applies; `map_or` reads the front without a branch no input could take.
        let highest = self.highs.front().map_or(self.close, |&(_, high)| high);
        let lowest = self.lows.front().map_or(self.close, |&(_, low)| low);
        Some(Reading {
            period: self.period,
            highest,
            lowest,
            close: self.close,
        })
    }
}

/// One Williams %R reading: the period it was taken at, the window's highest high and
/// lowest low, and the current close, all in paisa.
///
/// `Copy`, so a caller keeps the previous candle's reading for the crossings, or any
/// earlier one it wants. It carries its period, so a kept copy judges itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    period: u16,
    highest: i64,
    lowest: i64,
    close: i64,
}

impl Reading {
    /// The period of the [`WilliamsR`] this reading came from.
    #[must_use]
    pub const fn period(&self) -> u16 {
        self.period
    }

    /// The window's highest high, `HH`, in paisa.
    #[must_use]
    pub const fn highest(&self) -> i64 {
        self.highest
    }

    /// The window's lowest low, `LL`, in paisa.
    #[must_use]
    pub const fn lowest(&self) -> i64 {
        self.lowest
    }

    /// The current candle's close, `C`, in paisa.
    #[must_use]
    pub const fn close(&self) -> i64 {
        self.close
    }

    /// A window of one price: `HH == LL`, so every candle in it has its high equal to its
    /// low, at that one price. There is no %R value (`TradingView`'s division by zero gives
    /// `na`, UNVERIFIED): [`Reading::ppm`] is `None` and every comparison and crossing is
    /// false.
    #[must_use]
    pub const fn flat(&self) -> bool {
        self.highest == self.lowest
    }

    /// `U = HH - C`, widened to `i128`: in `0..=R`.
    fn under(&self) -> i128 {
        i128::from(self.highest) - i128::from(self.close)
    }

    /// `R = HH - LL`, widened to `i128`: in `0..2^63`, and 0 exactly on a window of one
    /// price.
    fn range(&self) -> i128 {
        i128::from(self.highest) - i128::from(self.lowest)
    }

    /// The %R in parts per million, floored: `floor(-10^6 (HH - C) / (HH - LL))`, in
    /// `MIN_LEVEL_PPM..=MAX_LEVEL_PPM`; `None` on a window of one price
    /// ([`Reading::flat`]).
    ///
    /// For records and seeds. A decision compares with [`Reading::above`] and
    /// [`Reading::below`], which are exact: the floor can equal a level the exact %R is
    /// above.
    #[must_use]
    pub fn ppm(&self) -> Option<i32> {
        if self.flat() {
            return None;
        }
        // `0 <= U <= R` with `R > 0`, so the quotient lies in `-1_000_000..=0` and always
        // fits an `i32`; `try_from` narrows it without a cast and without a branch.
        i32::try_from((-PPM * self.under()).div_euclid(self.range())).ok()
    }

    /// %R strictly above `level`, exact: `10^6 (HH - C) < -level (HH - LL)` in `i128`. At
    /// -850,000 that is `100 (HH - C) < 85 (HH - LL)`. A tie is neither above nor below.
    ///
    /// No separate `R > 0` test: on a window of one price `U = R = 0`, both sides are 0
    /// and the strict comparison is false ([`Reading::flat`]).
    #[must_use]
    pub fn above(&self, level: Level) -> bool {
        PPM * self.under() < -i128::from(level.ppm()) * self.range()
    }

    /// %R strictly below `level`, exact: `10^6 (HH - C) > -level (HH - LL)` in `i128`. At
    /// -150,000 that is `100 (HH - C) > 15 (HH - LL)`. A tie is neither above nor below.
    ///
    /// False on a window of one price for the reason [`Reading::above`] gives.
    #[must_use]
    pub fn below(&self, level: Level) -> bool {
        PPM * self.under() > -i128::from(level.ppm()) * self.range()
    }

    /// This reading crossed above `level` from `previous`, in `TradingView`'s
    /// `ta.crossover` form: `previous` not above the level, at or below it, and this reading
    /// strictly above it.
    ///
    /// False when either reading is a window of one price ([`Reading::flat`]), as `na` on
    /// either side gives false, and false when the two were taken at different periods.
    /// The period check catches a reading of another period only; two streams of one period
    /// are the caller's to keep apart. The caller keeps `previous`, the reading of the
    /// candle before; this module stores none (D-8642).
    #[must_use]
    pub fn crossed_above(&self, previous: &Reading, level: Level) -> bool {
        previous.period == self.period
            && !previous.flat()
            && !previous.above(level)
            && self.above(level)
    }

    /// This reading crossed below `level` from `previous`, in `TradingView`'s
    /// `ta.crossunder` form: `previous` not below the level and this reading strictly below
    /// it; false next to a window of one price or across periods, as
    /// [`Reading::crossed_above`] (D-8642).
    #[must_use]
    pub fn crossed_below(&self, previous: &Reading, level: Level) -> bool {
        previous.period == self.period
            && !previous.flat()
            && !previous.below(level)
            && self.below(level)
    }
}

// ---- C-PSRB-WILLR-01 and C-PSRB-WILLR-02: the measuring half of the bench (D-8644).
//
// The timings are taken here and judged in `benches/ratio/psrb_willr.rs`, which sorts: gate
// 11 keeps every sort out of this crate. The helpers are this part's own copy of the F&O
// form (PARTS.md 2.4: a shared helper would make one part wait on another).

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

/// Reads p50, p99 and max from samples sorted ascending, by nearest rank: the `p` quantile
/// of `n` samples is the `ceil(p n)`-th smallest. No sample reads every figure 0, which
/// [`flat_within`] refuses.
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
/// No cost, or a p99 of 0 at any size, fails: an operation timed at zero has not been shown
/// cheap, it has been shown absent (the rule of `ratio` in the crate's bench).
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

/// The period [`measure_step`] times at: 14 (PARTS-WILLR-CCI.md 5.6).
const STEP_PERIOD: u16 = 14;

/// The candles [`measure_period`] folds before it times: 20,000.
const PERIOD_HISTORY: u64 = 20_000;

/// The level the timed comparisons and crossing read: -850,000, %R -85.
const BENCH_LEVEL: Level = Level(-850_000);

/// `rounds` rounds of `samples` timings of one step at period 14 after `history` candles,
/// in nanoseconds and unsorted: one [`WilliamsR::fold`], [`WilliamsR::reading`], both
/// comparisons of the reading against a level and [`Reading::crossed_above`] against the
/// previous candle's reading, each sample bracketed by two clock reads. `C-PSRB-WILLR-01`,
/// judged in `crates/indicators/benches/ratio.rs`.
///
/// # Errors
///
/// A refusal of [`WilliamsR::fold`], passed on. The candles are made valid, so none is
/// expected; one would fail the row.
pub fn measure_step(history: u64, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal> {
    measure(STEP_PERIOD, history, samples, rounds)
}

/// The same step at `period` after 20,000 candles: `C-PSRB-WILLR-02`, judged in
/// `crates/indicators/benches/ratio.rs`.
///
/// # Errors
///
/// [`Refusal::PeriodZero`] for period 0; a refusal of [`WilliamsR::fold`], passed on,
/// which the valid candles make unexpected.
pub fn measure_period(period: u16, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal> {
    measure(period, PERIOD_HISTORY, samples, rounds)
}

/// Folds `history` candles into a fresh [`WilliamsR`] at `period`, then times `rounds`
/// rounds of `samples` steps, the state carried on from one step to the next and never
/// cloned.
fn measure(period: u16, history: u64, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal> {
    let mut state = WilliamsR::new(period)?;
    let mut minute = 0_i64;
    for _ in 0..history {
        state.fold(&bench_candle(minute))?;
        minute += 1;
    }
    let mut previous = state.reading();
    let mut out = Vec::with_capacity(usize::try_from(rounds).unwrap_or(0));
    for _ in 0..rounds {
        let mut round = Vec::with_capacity(usize::try_from(samples).unwrap_or(0));
        for _ in 0..samples {
            let candle = bench_candle(minute);
            minute += 1;
            let start = Instant::now();
            state.fold(black_box(&candle))?;
            let reading = state.reading();
            if let (Some(now), Some(before)) = (reading, previous) {
                black_box((
                    now.above(BENCH_LEVEL),
                    now.below(BENCH_LEVEL),
                    now.crossed_above(&before, BENCH_LEVEL),
                ));
            }
            previous = reading;
            round.push(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX));
        }
        out.push(round);
    }
    Ok(out)
}

/// A valid candle at minute `m`, the deterministic series the bench times: the close one of
/// 4,001 prices 13 paisa apart around 25,000.00, the high 0 to 400 paisa above it and the
/// low 0 to 396 below, each picked by a fixed integer hash of the minute and the field. A
/// stride through the prices would make a sawtooth whose jumps empty a whole deque at once;
/// hashed prices keep both deques short and popping a few entries a candle, the typical
/// step. No clock and no random source, so two runs time the same work.
fn bench_candle(m: i64) -> Candle {
    let key = m.unsigned_abs().wrapping_mul(4);
    let close = 2_500_000 + (hashed(key, 4_001) - 2_000) * 13;
    Candle {
        ts_micros: m.saturating_mul(60_000_000),
        open: close,
        high: close + hashed(key + 1, 401),
        low: close - hashed(key + 2, 397),
        close,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// The finalizer of `SplitMix64` applied to `input`, reduced to `0..bound`: a fixed hash,
/// so the bench's prices are the same on every run.
fn hashed(input: u64, bound: u64) -> i64 {
    let mut z = input.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    // Below `bound`, which is at most 4,001, so it always fits; the fallback is never taken.
    i64::try_from((z ^ (z >> 31)) % bound).unwrap_or(0)
}

#[cfg(test)]
#[path = "williams_r_tests.rs"]
mod tests;
