//! The Commodity Channel Index on the typical price, held in exact integers (PSRB-CCI; D-8650 to D-8655).
//!
//! PSRB is the strategy family narrow CPR and Pivot R1, Supertrend, Bollinger, RSI, Williams %R, CCI and EMA
//! 200, tested on NSE F&O stocks, cash, intraday. This module is its CCI reading and nothing else. It emits no
//! condition bit and holds no vocabulary position: [`Cci`] is a reading for the F&O strategy path, whose first
//! caller, the strategy part, is not built yet (D-8650).
//!
//! # The definition: the published one, UNVERIFIED against `TradingView`'s own output
//!
//! ```text
//! TP  = (high + low + close) / 3               the typical price, `hlc3`
//! SMA = the mean of the last n typical prices  `ta.sma(hlc3, n)`
//! MD  = the mean of |TP_i - SMA| over them     `ta.dev(hlc3, n)`
//! CCI = (TP - SMA) / (0.015 * MD)              `ta.cci(hlc3, n)`
//! ```
//!
//! That is the form of `TradingView`'s built-in CCI script as the PSRB design states it: the mean absolute
//! deviation is taken from the same SMA, over the same `n` candles, and the current candle's typical price is
//! one of them (offset 0). No output of theirs was compared with this module, so parity here is parity with the
//! published formulas. The first reading is after exactly `n` candles, at candle `n - 1` counting from 0, where
//! `ta.sma` and `ta.dev` first have a value. A [`Cci`] carried across sessions keeps its window; a daily reset
//! is a new [`Cci`] (D-8650).
//!
//! # Exact, in thirds and n-multiples (D-8651)
//!
//! With `T = high + low + close` (three typical prices), `S` the sum of the last `n` values of `T` (`3 n SMA`),
//! `dev = n T - S` for the current candle (`3 n (TP - SMA)`) and `D` the sum of `|n T_i - S|` over the window
//! (`3 n^2 MD`), every quantity is an integer, and with `0.015 = 3 / 200`:
//!
//! ```text
//! CCI = (dev / 3n) / ((3 / 200) * (D / 3n^2)) = 200 n dev / (3 D)
//! ```
//!
//! So `1000 CCI > m` exactly when `200_000 n dev > 3 m D` (both sides multiplied by `3 D > 0`), and
//! [`Reading::above`] and [`Reading::below`] compare those two `i128` products: no division, no rounding, and a
//! tie is neither. At the confirmed level, `CCI > +200` exactly when `n dev > 3 D`, which is
//! `TP - SMA > 3 MD`. [`Reading::milli`] is the floor of `1000 CCI`, for records and seeds; no decision needs
//! it. A [`Level`] is held in thousandths of a point (+200 is 200,000) within +/-10,000 points
//! ([`MAX_LEVEL_MILLI`]), a chosen domain well inside the `i128` bound; a level beyond it is refused, never
//! wrapped. Every product is bounded below `2^127` by the `const` assertions beside the arithmetic.
//!
//! # A window of one typical price (D-8652)
//!
//! When every typical price in the window is equal, `D` is 0 and so is `dev`, and the published ratio is
//! `0 / 0`, which `TradingView` reads as `na` (UNVERIFIED). [`Reading::flat`] names that state: it has no
//! [`Reading::milli`], and both comparisons are false at every level. Different candles can share a typical
//! price, (101, 99, 100) and (102, 98, 100) both have `T = 300`, so such a window need not be a still market.
//!
//! # The ceiling (D-8652)
//!
//! The deviations `n T_i - S` sum to 0, so `D` is twice the sum of the positive ones and `D >= 2 |dev|`. Hence
//! `|CCI| <= 100 n / 3`, reached exactly when `dev` is not 0 and the current candle is the only one strictly on
//! its side of the mean. A period of 6 or less never reads above +200 (at 6 the ceiling is exactly +200), a
//! period of 1 always has `D = 0`, and a period of 2 reads +66.666... or -66.666... whenever it has a value.
//!
//! # Cost (D-8653; `docs/06-limits.md` L-PSRB-CCI-02)
//!
//! [`Cci::fold`] is O(1): two `i128` additions, at most one pop and one push, whatever the period and however
//! many candles came before (`C-PSRB-CCI-01` and `C-PSRB-CCI-02`, whose wrapper is in
//! `crates/indicators/benches/ratio.rs`). [`Cci::reading`] is O(period), by design: the mean deviation is taken
//! around the CURRENT mean, which moves with every candle, so every term of `D` changes and one pass over the
//! window is needed. At the confirmed period 20 that is 20 steps. An order-statistic tree would make it
//! O(log period); it is not built, and that is decided (D-8653).
//!
//! # Memory (D-8654; L-PSRB-CCI-03)
//!
//! The window is a ring of `T` values, a `VecDeque<i128>` reserved for `period` values in [`Cci::new`] and
//! never asked for more: a fold pops the oldest value before it pushes once the ring is full, so the ring never
//! holds more than `period` values and a ring built by `new` never reallocates. 16 bytes a candle, about 1 MiB
//! at `u16::MAX`. A clone reserves only its length; the measured path never clones.
//!
//! # Streaming, and what is not read (D-8654)
//!
//! [`Cci::fold`] takes one candle, refuses it with [`Candle::check_evaluable`]'s own reason before anything
//! moves, and then reads its high, low and close and nothing else. A reading after candle `t` depends on
//! candles `0..=t` and on nothing later, and the current candle is in its own reading; a caller that wants the
//! reading before the current candle keeps the previous one, which is `Copy`. Timestamps are not read: ordering
//! and holes are the caller's.

use std::collections::VecDeque;
use std::hint::black_box;
use std::time::Instant;

use crate::{Candle, Corrupt, OI_NULL};

/// The widest [`Level`], in thousandths of a CCI point: +/-10,000 points (+200 is 200,000).
///
/// A chosen domain, 50 times the confirmed level, not one the arithmetic forces: with `D < 2^97` the bound
/// `3 |m| D < 2^127` would allow about 357,914 points. At 10,000 points every product stays below `2^123`
/// (D-8651; `docs/06-limits.md` L-PSRB-CCI-01).
pub const MAX_LEVEL_MILLI: i32 = 10_000_000;

/// `1000 * 200`: thousandths of a point times the 200 of `CCI = 200 n dev / (3 D)`.
const MILLI_SCALE: i128 = 200_000;

// THE BOUNDS THAT RULE OUT AN `i128` OVERFLOW (D-8651). Each limit is a power of two and each product is taken
// with `checked_mul` inside the assertion, so a bound that left `i128` fails the build instead of wrapping.
// Every operand of the arithmetic below is inside one of them, so no checked or saturating arithmetic is
// needed in `fold` or `Reading`, and none hides a wrong value. The level's edge is written out in `i128` and
// tied to the public constant by its own assertion: this module writes no `as`, and a widening `From` cannot
// run in a constant.
//
// A price of an evaluable candle is a positive `i64`: below `2^63`.
const PRICE_LIMIT: i128 = 1 << (i64::BITS - 1);
// `T = H + L + C` is at least 3 and below `3 * 2^63`, inside `2^65`.
const TYPICAL_LIMIT: i128 = 1 << 65;
// A period is a `u16`: below `2^16`.
const PERIOD_LIMIT: i128 = 1 << u16::BITS;
// `S`, a sum of at most `n` values of `T`, is positive and below `2^81`, and so is `n T`; so `|n T_i - S|`
// is below `2^81` for every `T_i` of the window, `dev` included.
const SUM_LIMIT: i128 = 1 << 81;
// `D`, a sum of `n` such terms, is below `2^97`, and the divisor `3 D` of `Reading::milli` below `2^99`.
const TOTAL_LIMIT: i128 = 1 << 97;
// `MILLI_SCALE` is below `2^18`.
const SCALE_LIMIT: i128 = 1 << 18;
// `|m|` is at most `MAX_LEVEL_MILLI`, `10^7`, below `2^24`.
const LEVEL_EDGE: i128 = 10_000_000;
const _: () = assert!(MAX_LEVEL_MILLI == 10_000_000 && LEVEL_EDGE < 1 << 24);
const _: () = assert!(matches!(PRICE_LIMIT.checked_mul(3), Some(t) if t < TYPICAL_LIMIT));
const _: () = assert!(matches!(
    TYPICAL_LIMIT.checked_mul(PERIOD_LIMIT),
    Some(SUM_LIMIT)
));
const _: () = assert!(matches!(
    SUM_LIMIT.checked_mul(PERIOD_LIMIT),
    Some(TOTAL_LIMIT)
));
// `200_000 n |dev|`, the left side of every comparison: below `2^18 * 2^16 * 2^81 = 2^115`.
const _: () = assert!(MILLI_SCALE < SCALE_LIMIT);
const _: () = assert!(matches!(
    SCALE_LIMIT.checked_mul(PERIOD_LIMIT),
    Some(f) if matches!(f.checked_mul(SUM_LIMIT), Some(p) if p == 1 << 115)
));
// `3 |m| D`, the right side: at most `3 * 10^7 * 2^97`, below `2^123`.
const _: () = assert!(matches!(
    LEVEL_EDGE.checked_mul(3),
    Some(f) if matches!(f.checked_mul(TOTAL_LIMIT), Some(p) if p < 1 << 123)
));

/// Why a constructor or a fold refused. A refusal changes nothing (D-8654).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A window of no candles: [`Cci::new`] was given 0.
    PeriodZero,
    /// A level beyond +/-[`MAX_LEVEL_MILLI`] thousandths of a point, given to [`Level::from_milli`].
    LevelOutOfRange {
        /// The level asked for, in thousandths of a point.
        milli: i32,
    },
    /// The candle is not one: [`Candle::check_evaluable`]'s own refusal.
    Corrupt(Corrupt),
}

/// A CCI level in thousandths of a point: +200 is `200_000` and -100 is `-100_000`. The domain is
/// `-MAX_LEVEL_MILLI..=MAX_LEVEL_MILLI` (D-8651).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level(i32);

impl Level {
    /// The level of `milli` thousandths of a point.
    ///
    /// # Errors
    ///
    /// [`Refusal::LevelOutOfRange`], naming `milli`, above [`MAX_LEVEL_MILLI`] or below its negation.
    pub const fn from_milli(milli: i32) -> Result<Self, Refusal> {
        if milli > MAX_LEVEL_MILLI {
            return Err(Refusal::LevelOutOfRange { milli });
        }
        if milli < -MAX_LEVEL_MILLI {
            return Err(Refusal::LevelOutOfRange { milli });
        }
        Ok(Self(milli))
    }

    /// The level, in thousandths of a point.
    #[must_use]
    pub const fn milli(self) -> i32 {
        self.0
    }
}

/// The running window of the last `period` typical prices, each held as `T = H + L + C` (D-8650).
///
/// The ring is reserved once, in [`Cci::new`], for `period` values. `S`, their sum, moves by two additions a
/// fold, and the current candle's `T` is kept beside it, so [`Cci::reading`] reads the ring once, and only to
/// sum `D`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cci {
    /// The last `period` values of `T`, the oldest at the front; fewer until the window is full.
    ring: VecDeque<i128>,
    /// `S`, the sum of the ring.
    sum: i128,
    /// The current candle's `T`, stored by [`Cci::fold`]. 0 before the first fold, which no reading shows:
    /// [`Cci::reading`] is `None` until the window is full.
    last: i128,
    /// The window's length, `1..=u16::MAX` candles.
    period: u16,
}

impl Cci {
    /// An empty window of `period` candles, its ring reserved here, once, for `period` values.
    ///
    /// # Errors
    ///
    /// [`Refusal::PeriodZero`] for 0. Every other `u16` is accepted.
    pub fn new(period: u16) -> Result<Self, Refusal> {
        if period == 0 {
            return Err(Refusal::PeriodZero);
        }
        Ok(Self {
            ring: VecDeque::with_capacity(usize::from(period)),
            sum: 0,
            last: 0,
            period,
        })
    }

    /// The window's length, in candles.
    #[must_use]
    pub const fn period(&self) -> u16 {
        self.period
    }

    /// Fold one candle into the window. O(1) whatever the period and the history: two `i128` additions, at
    /// most one pop and one push (`C-PSRB-CCI-01` and `C-PSRB-CCI-02`, whose wrapper is in
    /// `crates/indicators/benches/ratio.rs`).
    ///
    /// The candle is checked first ([`Candle::check_evaluable`]), so a refusal moves nothing. Then its `T`,
    /// widened to `i128` before the sum, is pushed; when the ring already holds `period` values the oldest is
    /// popped first and taken out of `S`, so the ring never holds more than `period` values. That pop reads its
    /// `Option` with `unwrap_or(0)`, whose default never applies: the arm runs only when the ring is full, and a
    /// full ring holds `period >= 1` values. `T` is kept as the current candle's.
    ///
    /// # Errors
    ///
    /// [`Refusal::Corrupt`] for a record that is not a candle. The state is unchanged.
    pub fn fold(&mut self, candle: &Candle) -> Result<(), Refusal> {
        candle.check_evaluable().map_err(Refusal::Corrupt)?;
        let typical = i128::from(candle.high) + i128::from(candle.low) + i128::from(candle.close);
        if self.ring.len() == usize::from(self.period) {
            self.sum -= self.ring.pop_front().unwrap_or(0);
        }
        self.ring.push_back(typical);
        self.sum += typical;
        self.last = typical;
        Ok(())
    }

    /// Whether `period` candles are held: true from the `period`-th fold on, and never false again.
    #[must_use]
    pub fn warm(&self) -> bool {
        self.ring.len() == usize::from(self.period)
    }

    /// The reading of the last `period` candles, the current one included, or `None` until `period` candles
    /// are held.
    ///
    /// O(period), by design (D-8653; `docs/06-limits.md` L-PSRB-CCI-02): `dev` comes from the kept `T` and
    /// `S`, and one pass over the ring sums `D`, because the mean every term is taken around moves with every
    /// candle. The [`Reading`] copies `dev`, `D` and the period, and never reads the ring again.
    #[must_use]
    pub fn reading(&self) -> Option<Reading> {
        if !self.warm() {
            return None;
        }
        let n = i128::from(self.period);
        let total_deviation = self
            .ring
            .iter()
            .map(|&typical| (n * typical - self.sum).abs())
            .sum();
        Some(Reading {
            period: self.period,
            deviation: n * self.last - self.sum,
            total_deviation,
        })
    }
}

/// One full window's reading, copied out of a [`Cci`]: `dev`, `D` and the period they were taken at.
///
/// `Copy`, so a caller that wants the convention "the anchor excludes the candle" keeps the reading of the
/// candle before and judges with it after the next fold (D-8650).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    /// The window's length, `n`.
    period: u16,
    /// `dev = n T - S` for the current candle.
    deviation: i128,
    /// `D`, the sum of `|n T_i - S|` over the window.
    total_deviation: i128,
}

impl Reading {
    /// `n`, the window's length in candles: a kept copy judges itself.
    #[must_use]
    pub const fn period(&self) -> u16 {
        self.period
    }

    /// `dev = n T - S` for the current candle: `3 n (TP - SMA)`.
    #[must_use]
    pub const fn deviation(&self) -> i128 {
        self.deviation
    }

    /// `D`, the sum of `|n T_i - S|` over the window: `3 n^2 MD`.
    #[must_use]
    pub const fn total_deviation(&self) -> i128 {
        self.total_deviation
    }

    /// `D == 0`: every typical price in the window is equal, and the reading has no value (D-8652).
    #[must_use]
    pub const fn flat(&self) -> bool {
        self.total_deviation == 0
    }

    /// `floor(1000 CCI) = floor(200_000 n dev / (3 D))`, or `None` on a window of one typical price
    /// ([`Reading::flat`]).
    ///
    /// For records and seeds; decisions read [`Reading::above`] and [`Reading::below`], which are exact. It
    /// always fits an `i64`: by the ceiling, `|1000 CCI| <= 100_000 n / 3 <= 2_184_500_000`, which passes
    /// `i32::MAX` at the largest periods and is why the value is an `i64`.
    #[must_use]
    pub fn milli(&self) -> Option<i64> {
        if self.flat() {
            return None;
        }
        let floor = self.scaled().div_euclid(3 * self.total_deviation);
        i64::try_from(floor).ok()
    }

    /// `CCI > level`, exactly: `D > 0` and `200_000 n dev > 3 m D`, with `m` the level in thousandths. A tie is
    /// not above, and the answer is false on a window of one typical price ([`Reading::flat`]).
    #[must_use]
    pub fn above(&self, level: Level) -> bool {
        self.total_deviation > 0 && self.scaled() > self.threshold(level)
    }

    /// `CCI < level`, exactly: `D > 0` and `200_000 n dev < 3 m D`. A tie is not below, and the answer is false
    /// on a window of one typical price ([`Reading::flat`]).
    #[must_use]
    pub fn below(&self, level: Level) -> bool {
        self.total_deviation > 0 && self.scaled() < self.threshold(level)
    }

    /// `200_000 n dev`: `3 D` times `1000 CCI`.
    fn scaled(&self) -> i128 {
        MILLI_SCALE * i128::from(self.period) * self.deviation
    }

    /// `3 m D`: `3 D` times the level in thousandths.
    fn threshold(&self, level: Level) -> i128 {
        3 * i128::from(level.milli()) * self.total_deviation
    }
}

/// One cost reading of a round of single-step timings: three ranks and the count. The bench wrapper sorts
/// each round and reads it with [`summarize`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepCost {
    /// The nearest-rank 50th percentile, in nanoseconds.
    pub p50_ns: u64,
    /// The nearest-rank 99th percentile, in nanoseconds.
    pub p99_ns: u64,
    /// The slowest sample, in nanoseconds.
    pub max_ns: u64,
    /// How many samples the round held.
    pub samples: usize,
}

/// A round's [`StepCost`], read by rank off timings SORTED ascending.
///
/// The bench wrapper sorts: this crate holds no sort (gate 11). The `p`-th percentile of `n` samples is the
/// `ceil(p * n / 100)`-th smallest. An empty round reads zeros, which [`flat_within`] refuses.
#[must_use]
pub fn summarize(sorted: &[u64]) -> StepCost {
    StepCost {
        p50_ns: nearest_rank(sorted, 50),
        p99_ns: nearest_rank(sorted, 99),
        max_ns: sorted.last().copied().unwrap_or(0),
        samples: sorted.len(),
    }
}

/// The `ceil(percent * n / 100)`-th smallest of `sorted`, or 0 when empty.
fn nearest_rank(sorted: &[u64], percent: usize) -> u64 {
    let rank = sorted.len().saturating_mul(percent).div_ceil(100);
    sorted.get(rank.saturating_sub(1)).copied().unwrap_or(0)
}

/// The judge of `C-PSRB-CCI-01` and `C-PSRB-CCI-02`, whose wrapper is in `crates/indicators/benches/ratio.rs`:
/// every cost's p99 is above zero, within `ceiling_permille` thousandths of the FIRST cost's p99, and at most
/// `budget_ns`.
///
/// No costs, or a p99 of zero at any size, is a failure: a step timed at nothing has been shown absent, not
/// cheap, which is the harness's own rule.
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

/// The period of `C-PSRB-CCI-01`'s step: 20, the confirmed one.
const STEP_PERIOD: u16 = 20;

/// The candles folded before `C-PSRB-CCI-02` and the unjudged reading line time their step: 20,000.
const PERIOD_HISTORY: u64 = 20_000;

/// The levels a bench step judges at: +200 and -200, the confirmed ones.
const STEP_ABOVE: Level = Level(200_000);
const STEP_BELOW: Level = Level(-200_000);
const _: () = assert!(STEP_ABOVE.0 <= MAX_LEVEL_MILLI && STEP_BELOW.0 >= -MAX_LEVEL_MILLI);

/// Where the bench's synthetic walk starts: Rs 25,000.00, in paisa.
const WALK_BASE: i64 = 2_500_000;

/// How far a walk candle's high and low sit from its close, in paisa.
const WALK_REACH: i64 = 4;

/// One minute, in microseconds: the walk's spacing.
const MINUTE_MICROS: i64 = 60_000_000;

/// `C-PSRB-CCI-01`'s measurement (D-8655): a window of 20 candles is fed `history` candles of the synthetic
/// walk, then `rounds` rounds of `samples` steps are each timed alone, in nanoseconds, unsorted.
///
/// A step is one [`Cci::fold`], one [`Cci::reading`] and the two comparisons [`Reading::above`] at +200 and
/// [`Reading::below`] at -200, of the reading just made. Nothing is printed here; the wrapper in
/// `crates/indicators/benches/ratio.rs` reads and judges.
///
/// # Errors
///
/// A fold's refusal, passed on, which only a walk that drifted to a nonpositive price could make: the walk
/// moves at most 4 paisa a candle from Rs 25,000.00.
pub fn measure_step(history: u64, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal> {
    measure(
        STEP_PERIOD,
        history,
        samples,
        rounds,
        WALK_BASE,
        read_and_judge,
    )
}

/// `C-PSRB-CCI-02`'s measurement (D-8655): [`Cci::fold`] ALONE, timed at a window of `period` candles after
/// 20,000 candles, so the ring's length is the only thing that changes between sizes. The reading, which is
/// O(period) by design, is not in this step; [`measure_reading_period`] times it, and the wrapper in
/// `crates/indicators/benches/ratio.rs` prints that line unjudged.
///
/// # Errors
///
/// [`Refusal::PeriodZero`] for period 0, and a fold's refusal as for [`measure_step`].
pub fn measure_period(period: u16, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal> {
    measure(
        period,
        PERIOD_HISTORY,
        samples,
        rounds,
        WALK_BASE,
        fold_only,
    )
}

/// The step of [`measure_step`], with [`Cci::reading`] in it, at a window of `period` candles after 20,000
/// candles: the measured slope of a reading across periods, O(period) by design (D-8653; `docs/06-limits.md`
/// L-PSRB-CCI-02). The wrapper in `crates/indicators/benches/ratio.rs` prints it on one line without an id and
/// judges nothing with it.
///
/// # Errors
///
/// [`Refusal::PeriodZero`] for period 0, and a fold's refusal as for [`measure_step`].
pub fn measure_reading_period(
    period: u16,
    samples: u64,
    rounds: u64,
) -> Result<Vec<Vec<u64>>, Refusal> {
    measure(
        period,
        PERIOD_HISTORY,
        samples,
        rounds,
        WALK_BASE,
        read_and_judge,
    )
}

/// The three measurements: a fresh [`Cci`] at `period` folds `history` candles of the walk from `base` paisa,
/// then `rounds` rounds of `samples` steps are timed, the state carried from one step to the next. A step is
/// one fold and then `after`, inside the two clock reads. The walk's start is a parameter so a test can make its
/// candles refuse.
fn measure(
    period: u16,
    history: u64,
    samples: u64,
    rounds: u64,
    base: i64,
    after: impl Fn(&Cci),
) -> Result<Vec<Vec<u64>>, Refusal> {
    let mut cci = Cci::new(period)?;
    let mut walk = Walk::new(base);
    for _ in 0..history {
        cci.fold(&walk.next_candle())?;
    }
    let mut timings = Vec::with_capacity(usize::try_from(rounds).unwrap_or(0));
    for _ in 0..rounds {
        let mut round = Vec::with_capacity(usize::try_from(samples).unwrap_or(0));
        for _ in 0..samples {
            let candle = walk.next_candle();
            let began = Instant::now();
            let folded = cci.fold(black_box(&candle));
            after(&cci);
            let elapsed = began.elapsed().as_nanos();
            folded?;
            round.push(u64::try_from(elapsed).unwrap_or(u64::MAX));
        }
        timings.push(round);
    }
    Ok(timings)
}

/// After a fold, `C-PSRB-CCI-02`'s step reads nothing; the state is handed to `black_box` so the fold is kept.
fn fold_only(cci: &Cci) {
    black_box(cci);
}

/// After a fold, the reading and its two comparisons at the confirmed levels.
fn read_and_judge(cci: &Cci) {
    black_box(
        cci.reading()
            .map(|reading| (reading.above(STEP_ABOVE), reading.below(STEP_BELOW))),
    );
}

/// The bench's synthetic candles: a deterministic xorshift walk of the close, moving -4..=4 paisa a minute,
/// the open at the close and the high and low [`WALK_REACH`] paisa either side of it. Invented; no vendor row.
struct Walk {
    state: u64,
    close: i64,
    stamp: i64,
}

impl Walk {
    /// A walk from `base` paisa.
    const fn new(base: i64) -> Self {
        Self {
            state: 0x9E37_79B9_7F4A_7C15,
            close: base,
            stamp: 0,
        }
    }

    /// The next minute's candle.
    fn next_candle(&mut self) -> Candle {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        let step = i64::try_from(self.state % 9).unwrap_or(4) - 4;
        self.close = self.close.saturating_add(step);
        self.stamp = self.stamp.saturating_add(MINUTE_MICROS);
        Candle::new(
            self.stamp,
            self.close,
            self.close.saturating_add(WALK_REACH),
            self.close.saturating_sub(WALK_REACH),
            self.close,
            1,
            OI_NULL,
        )
    }
}

#[cfg(test)]
#[path = "cci_tests.rs"]
mod tests;
