//! Bollinger Bands over closes, exact, for the PSRB strategy family (D-8590).
//!
//! # What it reads
//!
//! The published definition, as report A of the PSRB research read it on
//! 3 Nov 2026 (its D4; D-8590), UNVERIFIED against `TradingView`'s own output:
//! `basis = SMA(close, n)`, `upper = basis + z * sigma`,
//! `lower = basis - z * sigma`, where `sigma` is the POPULATION standard
//! deviation of the same `n` closes and the current candle's close is one of
//! them (offset 0). The population divisor is the reading (D-8592); the sample
//! divisor `n - 1` is a second, labelled reading
//! ([`Window::above_upper_sample`], [`Window::below_lower_sample`]), which has
//! no value at `n = 1`.
//!
//! The module holds no vocabulary position and emits no condition bit. It is a
//! reading for the F&O strategy path, not a sweep condition. It reads OHLC
//! closes only: a candle's timestamp is never read, so ordering and holes are
//! the caller's, and nothing here reads a candle the caller has not folded yet.
//!
//! # Exact, integers only (D-8591)
//!
//! A window of `n` closes keeps `S`, their sum, and `Q`, the sum of their
//! squares, in `i128`. With `V = n * Q - S^2` (which is `n^2 * sigma^2`, so
//! never negative) and, for a close `c`, `D = n * c - S`:
//!
//! * `c > basis + z * sigma` exactly when `D > 0` and
//!   `(1000 * D)^2 > z_milli^2 * V`, `z_milli` being `z` in thousandths;
//! * `c < basis - z * sigma` is the mirror, with `-D`;
//! * the sample reading multiplies the left side by `n - 1` and the right side
//!   by `n`.
//!
//! No square root and no rounding enters a comparison, and a close exactly on a
//! band is not beyond it. [`Window::upper_floor`], [`Window::lower_ceil`] and
//! [`Window::z_milli`] give the band values and the z-score for records and
//! seeds through [`crate::vwap::isqrt_i128`]; they are exact floors and
//! ceilings of the real values and agree with the comparisons on every integer
//! close, but no decision needs them (D-8593).
//!
//! # Cost
//!
//! [`Bollinger::fold`] and every comparison are a fixed handful of `i128`
//! operations, whatever the period and however many candles came before: O(1)
//! per candle, measured flat by `C-PSRB-BOLL-01` (history) and
//! `C-PSRB-BOLL-02` (period) in `crates/indicators/benches/ratio.rs`
//! (D-8595). The band values and the z-score each add one `isqrt_i128`, which
//! is bounded by its `ITERATION_CEILING` of 16 iterations and not uniform
//! (`C-I-03`). The ring of closes is the only memory that grows with the
//! period: 8 bytes a close, 32 KiB at [`MAX_PERIOD`], allocated once in
//! [`Bollinger::new`] and never again.
//!
//! # Domain (D-8591)
//!
//! A period of `1..=MAX_PERIOD`, closes of `1..=MAX_CLOSE` paisa and a width of
//! `0..=MAX_WIDTH_MILLI` thousandths keep every product of the arithmetic
//! inside `i128`; each bound is a compile-time assertion beside it. A larger
//! input is refused by name, never wrapped, and every refusal leaves the state
//! exactly as it was (D-8594).

use std::hint::black_box;
use std::time::Instant;

use crate::vwap::isqrt_i128;
use crate::{Candle, Corrupt, OI_NULL};

/// The longest window, in closes (D-8590).
pub const MAX_PERIOD: u16 = 4_096;

/// The largest close a window holds, in paisa: `2^32` paisa, Rs 4.29 crore a
/// share (D-8591).
pub const MAX_CLOSE: i64 = 1 << 32;

/// The widest band, in thousandths of a sigma: `z` at most 10.000 (D-8591).
pub const MAX_WIDTH_MILLI: u32 = 10_000;

/// The denominator of a [`Width`]: `z` is held in thousandths.
pub const WIDTH_DEN: i128 = 1_000;

// THE DOMAIN BOUNDS (D-8591). Each product below is the largest the arithmetic
// of this module can form inside the domain, computed with `checked_mul` at
// compile time, so a wider domain that left `i128` fails the build here rather
// than wrapping at run time. The three constants are the domain's edges in
// `i128`, tied to the public constants by the first assertion: this module
// writes no `as`, and a widening `From` cannot run in a constant.
const PERIOD_EDGE: i128 = 4_096;
const CLOSE_EDGE: i128 = 4_294_967_296;
const WIDTH_EDGE: i128 = 10_000;
const _: () =
    assert!(MAX_PERIOD == 4_096 && MAX_CLOSE == 4_294_967_296 && MAX_WIDTH_MILLI == 10_000);

// `|D| = |n * c - S|` is below `n * MAX_CLOSE` = 2^44: `S` is a sum of `n`
// closes in `1..=MAX_CLOSE`, and so is `n * c`. `S * 1000` stays below 2^54.
const _: () = assert!(if let Some(reach) = PERIOD_EDGE.checked_mul(CLOSE_EDGE)
    && let Some(scaled) = reach.checked_mul(WIDTH_DEN)
{
    reach <= 1 << 44 && scaled < 1 << 54
} else {
    false
});

// `V = n * Q - S^2` is at most `(n * MAX_CLOSE)^2 / 4` <= 2^86: a population
// variance is at most a quarter of the squared range. `n * Q` and `S^2` are
// each at most `(n * MAX_CLOSE)^2` = 2^88.
const _: () = assert!(if let Some(reach) = PERIOD_EDGE.checked_mul(CLOSE_EDGE)
    && let Some(square) = reach.checked_mul(reach)
{
    square <= 1 << 88 && square / 4 <= 1 << 86
} else {
    false
});

// The squares test's left side with the sample's factor, `(1000 * D)^2 * n`,
// is at most 2^120.
const _: () = assert!(if let Some(reach) = PERIOD_EDGE.checked_mul(CLOSE_EDGE)
    && let Some(scaled) = reach.checked_mul(WIDTH_DEN)
    && let Some(square) = scaled.checked_mul(scaled)
    && let Some(product) = square.checked_mul(PERIOD_EDGE)
{
    product <= 1 << 120
} else {
    false
});

// Its right side, `z_milli^2 * n * V`, is at most 2^125 (with `V` <= 2^86).
const _: () = assert!(
    if let Some(width_square) = WIDTH_EDGE.checked_mul(WIDTH_EDGE)
        && let Some(widened) = width_square.checked_mul(PERIOD_EDGE)
        && let Some(product) = widened.checked_mul(1 << 86)
    {
        product <= 1 << 125
    } else {
        false
    }
);

// The z-score's numerator, `10^6 * D^2`, is at most 2^108; the root it is
// compared with squares back to at most that.
const _: () = assert!(if let Some(reach) = PERIOD_EDGE.checked_mul(CLOSE_EDGE)
    && let Some(square) = reach.checked_mul(reach)
    && let Some(product) = square.checked_mul(WIDTH_DEN * WIDTH_DEN)
{
    product <= 1 << 108
} else {
    false
});

/// A bound on every band value [`Window::upper_floor`] and
/// [`Window::lower_ceil`] can return, in paisa: `6 * MAX_CLOSE`.
///
/// A population sigma is at most half the window's range, which is below
/// `MAX_CLOSE`, so `basis + 10 * sigma` stays below `6 * MAX_CLOSE` and
/// `basis - 10 * sigma` above `-5 * MAX_CLOSE`. It is the fallback of the
/// narrowing to `i64`, which a value inside the domain never takes.
const BAND_EDGE: i64 = 6 * MAX_CLOSE;

/// Why a constructor or a fold refused. A refusal changes nothing (D-8594).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A window of no closes: [`Bollinger::new`] was given 0.
    PeriodZero,
    /// A window longer than [`MAX_PERIOD`] closes.
    PeriodAboveMax {
        /// The period asked for.
        period: u16,
    },
    /// A close outside `1..=MAX_CLOSE` paisa, given to [`Close::new`] or folded.
    CloseOutOfRange {
        /// The close, in paisa.
        close: i64,
    },
    /// A width above [`MAX_WIDTH_MILLI`] thousandths.
    WidthAboveMax {
        /// The width asked for, in thousandths.
        milli: u32,
    },
    /// The ring's write position lies outside the ring: a state no call of this
    /// API makes, refused by name rather than assumed away (D-8594).
    RingOutOfStep,
    /// The candle is not one: [`Candle::check_evaluable`]'s own refusal.
    Corrupt(Corrupt),
}

/// A close a window can hold: `1..=MAX_CLOSE` paisa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Close(i64);

impl Close {
    /// The close of `paisa`.
    ///
    /// # Errors
    ///
    /// [`Refusal::CloseOutOfRange`], naming `paisa`, when it is below 1 or
    /// above [`MAX_CLOSE`].
    pub const fn new(paisa: i64) -> Result<Self, Refusal> {
        if paisa < 1 {
            return Err(Refusal::CloseOutOfRange { close: paisa });
        }
        if paisa > MAX_CLOSE {
            return Err(Refusal::CloseOutOfRange { close: paisa });
        }
        Ok(Self(paisa))
    }

    /// The close, in paisa.
    #[must_use]
    pub const fn paisa(self) -> i64 {
        self.0
    }
}

/// A band multiplier `z`, in thousandths of a sigma: `2_000` is 2.000, and
/// `0..=MAX_WIDTH_MILLI` is the domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Width(u32);

impl Width {
    /// The width of `milli` thousandths.
    ///
    /// # Errors
    ///
    /// [`Refusal::WidthAboveMax`], naming `milli`, above [`MAX_WIDTH_MILLI`].
    pub const fn from_milli(milli: u32) -> Result<Self, Refusal> {
        if milli > MAX_WIDTH_MILLI {
            return Err(Refusal::WidthAboveMax { milli });
        }
        Ok(Self(milli))
    }

    /// The width, in thousandths.
    #[must_use]
    pub const fn milli(self) -> u32 {
        self.0
    }
}

/// The running window of the last `period` closes (D-8590).
///
/// The ring is allocated once, in [`Bollinger::new`], at its full length and
/// filled with zeros, so the filling phase needs no branch: a zero leaving the
/// ring takes nothing from either sum. Every fold writes one slot and moves two
/// sums; the window is copied out of the sums and the last close and never
/// reads the ring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bollinger {
    /// The last `period` closes, the oldest overwritten next; zeros until filled.
    ring: Vec<i64>,
    /// The slot the next close is written to, `0..period`.
    next: usize,
    /// Closes held, `0..=period`.
    filled: u16,
    /// The window's length, `1..=MAX_PERIOD`.
    period: u16,
    /// `S`, the sum of the ring.
    sum: i128,
    /// `Q`, the sum of the ring's squares.
    sum_sq: i128,
    /// The close folded last. `Close(1)` before the first fold, which no
    /// reading shows: [`Bollinger::window`] is `None` until the window is full.
    last: Close,
}

impl Bollinger {
    /// An empty window of `period` closes, its ring allocated here, once.
    ///
    /// # Errors
    ///
    /// [`Refusal::PeriodZero`] for 0, and [`Refusal::PeriodAboveMax`], naming
    /// the period, above [`MAX_PERIOD`].
    pub fn new(period: u16) -> Result<Self, Refusal> {
        if period == 0 {
            return Err(Refusal::PeriodZero);
        }
        if period > MAX_PERIOD {
            return Err(Refusal::PeriodAboveMax { period });
        }
        Ok(Self {
            ring: vec![0; usize::from(period)],
            next: 0,
            filled: 0,
            period,
            sum: 0,
            sum_sq: 0,
            last: Close(1),
        })
    }

    /// The window's length, in closes.
    #[must_use]
    pub const fn period(&self) -> u16 {
        self.period
    }

    /// Fold one candle's close into the window.
    ///
    /// Everything is checked before anything moves: the candle
    /// ([`Candle::check_evaluable`]), its close ([`Close::new`]) and the ring's
    /// write position, which `get_mut` reads once. Then the close replaces the
    /// oldest slot and both sums move by the difference.
    ///
    /// # Errors
    ///
    /// [`Refusal::Corrupt`] for a record that is not a candle (a close of 0 or
    /// below is the candle's own refusal), [`Refusal::CloseOutOfRange`] for a
    /// close above [`MAX_CLOSE`], and [`Refusal::RingOutOfStep`] for a write
    /// position outside the ring. Each leaves the state unchanged.
    pub fn fold(&mut self, candle: &Candle) -> Result<(), Refusal> {
        candle.check_evaluable().map_err(Refusal::Corrupt)?;
        let close = Close::new(candle.close)?;
        let Some(slot) = self.ring.get_mut(self.next) else {
            return Err(Refusal::RingOutOfStep);
        };
        let old = i128::from(std::mem::replace(slot, close.paisa()));
        let new = i128::from(close.paisa());
        self.sum += new - old;
        self.sum_sq += new * new - old * old;
        self.filled = self.filled.saturating_add(1).min(self.period);
        self.next += 1;
        if self.next == self.ring.len() {
            self.next = 0;
        }
        self.last = close;
        Ok(())
    }

    /// Whether `period` closes are held: true from the `period`-th fold on.
    #[must_use]
    pub const fn warm(&self) -> bool {
        self.filled == self.period
    }

    /// The window of the last `period` closes, the current candle's included,
    /// or `None` until `period` closes are held.
    #[must_use]
    pub fn window(&self) -> Option<Window> {
        if !self.warm() {
            return None;
        }
        Some(Window {
            period: self.period,
            sum: self.sum,
            sum_sq: self.sum_sq,
            last: self.last,
        })
    }
}

/// One full window's sums, copied out of a [`Bollinger`].
///
/// `Copy`, so a caller that wants the convention "the anchor excludes the
/// candle" keeps the window of the candle before and judges the next close
/// against it (D-8590).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    period: u16,
    sum: i128,
    sum_sq: i128,
    last: Close,
}

impl Window {
    /// `n`, the window's length in closes.
    #[must_use]
    pub const fn period(&self) -> u16 {
        self.period
    }

    /// The close folded last, the window's newest.
    #[must_use]
    pub const fn last(&self) -> Close {
        self.last
    }

    /// `S`, the sum of the window's closes, in paisa.
    #[must_use]
    pub const fn sum(&self) -> i128 {
        self.sum
    }

    /// `Q`, the sum of the squares of the window's closes.
    #[must_use]
    pub const fn sum_sq(&self) -> i128 {
        self.sum_sq
    }

    /// `V = n * Q - S^2`, which is `n^2` times the population variance, so the
    /// population sigma is `sqrt(V) / n`. Never negative.
    #[must_use]
    pub fn variance_numerator(&self) -> i128 {
        self.n() * self.sum_sq - self.sum * self.sum
    }

    /// `floor(S / n)`: the basis, rounded down to the paisa.
    #[must_use]
    pub fn basis_floor(&self) -> i64 {
        // The mean of closes in `1..=MAX_CLOSE` lies in that range, so the
        // narrowing never takes its fallback.
        i64::try_from(self.sum.div_euclid(self.n())).unwrap_or(MAX_CLOSE)
    }

    /// `close > basis + z * sigma`, exactly: `D > 0` and
    /// `(1000 * D)^2 > z_milli^2 * V`. A close on the band is not above it.
    #[must_use]
    pub fn above_upper(&self, close: Close, width: Width) -> bool {
        let reach = self.reach(close);
        reach > 0 && square(reach * WIDTH_DEN) > self.spread(width)
    }

    /// `close < basis - z * sigma`, exactly: `D < 0` and
    /// `(1000 * D)^2 > z_milli^2 * V`. A close on the band is not below it.
    #[must_use]
    pub fn below_lower(&self, close: Close, width: Width) -> bool {
        let reach = self.reach(close);
        reach < 0 && square(reach * WIDTH_DEN) > self.spread(width)
    }

    /// The floor of the real upper band, in paisa:
    /// `(1000 * S + isqrt(z_milli^2 * V)).div_euclid(1000 * n)`.
    ///
    /// Exact, because `floor((a + y) / m) = floor((a + floor(y)) / m)` for
    /// integers `a` and `m > 0` and a real `y >= 0`, and
    /// `floor(z_milli * sqrt(V)) = isqrt(z_milli^2 * V)`. For every integer
    /// close, `close > upper_floor` exactly when [`Window::above_upper`] holds.
    #[must_use]
    pub fn upper_floor(&self, width: Width) -> i64 {
        let root = isqrt_i128(self.spread(width));
        let upper = (self.sum * WIDTH_DEN + root).div_euclid(self.n() * WIDTH_DEN);
        i64::try_from(upper).unwrap_or(BAND_EDGE)
    }

    /// The ceiling of the real lower band, in paisa:
    /// `-((isqrt(z_milli^2 * V) - 1000 * S).div_euclid(1000 * n))`, exact by
    /// the same argument as [`Window::upper_floor`]. For every integer close,
    /// `close < lower_ceil` exactly when [`Window::below_lower`] holds.
    #[must_use]
    pub fn lower_ceil(&self, width: Width) -> i64 {
        let root = isqrt_i128(self.spread(width));
        let below = (root - self.sum * WIDTH_DEN).div_euclid(self.n() * WIDTH_DEN);
        i64::try_from(-below).unwrap_or(-BAND_EDGE)
    }

    /// `floor(1000 * (close - basis) / sigma)`, the z-score in thousandths,
    /// rounded down; `None` when sigma is 0 (`V = 0`).
    ///
    /// It is `floor(1000 * D / sqrt(V))`: for `D >= 0`,
    /// `isqrt((10^6 * D^2).div_euclid(V))`; for `D < 0` the negated ceiling of
    /// that root, which is the root itself when `root^2 * V = 10^6 * D^2` and
    /// one more otherwise.
    #[must_use]
    pub fn z_milli(&self, close: Close) -> Option<i64> {
        let variance = self.variance_numerator();
        if variance == 0 {
            return None;
        }
        let reach = self.reach(close);
        let scaled = square(reach * WIDTH_DEN);
        let root = isqrt_i128(scaled.div_euclid(variance));
        let z = if reach >= 0 {
            root
        } else if square(root) * variance == scaled {
            -root
        } else {
            -root - 1
        };
        i64::try_from(z).ok()
    }

    /// The labelled sample reading (D-8592): `close > basis + z * s`, with `s`
    /// the sample sigma `sqrt(V / (n * (n - 1)))`, exactly: `D > 0` and
    /// `(1000 * D)^2 * (n - 1) > z_milli^2 * n * V`. `None` when `n = 1`,
    /// where the sample sigma has no value.
    #[must_use]
    pub fn above_upper_sample(&self, close: Close, width: Width) -> Option<bool> {
        if self.period == 1 {
            return None;
        }
        let reach = self.reach(close);
        Some(reach > 0 && self.beyond_sample(reach, width))
    }

    /// The mirror of [`Window::above_upper_sample`]: `close < basis - z * s`,
    /// exactly; `None` when `n = 1`.
    #[must_use]
    pub fn below_lower_sample(&self, close: Close, width: Width) -> Option<bool> {
        if self.period == 1 {
            return None;
        }
        let reach = self.reach(close);
        Some(reach < 0 && self.beyond_sample(reach, width))
    }

    /// `n`, widened for the arithmetic.
    fn n(&self) -> i128 {
        i128::from(self.period)
    }

    /// `D = n * close - S`: `n` times the close's distance from the basis.
    fn reach(&self, close: Close) -> i128 {
        self.n() * i128::from(close.paisa()) - self.sum
    }

    /// `z_milli^2 * V`: the square of `1000 * n` times the band's half-width.
    fn spread(&self, width: Width) -> i128 {
        square(i128::from(width.milli())) * self.variance_numerator()
    }

    /// `(1000 * D)^2 * (n - 1) > z_milli^2 * n * V`.
    fn beyond_sample(&self, reach: i128, width: Width) -> bool {
        square(reach * WIDTH_DEN) * (self.n() - 1) > self.spread(width) * self.n()
    }
}

/// `x^2`.
const fn square(x: i128) -> i128 {
    x * x
}

/// One cost reading of a round of single-step timings: three ranks and the
/// count. The bench wrapper sorts each round and reads it with [`summarize`].
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
/// The bench wrapper sorts: this crate holds no sort (gate 11). The `p`-th
/// percentile of `n` samples is the `ceil(p * n / 100)`-th smallest. An empty
/// round reads zeros, which [`flat_within`] refuses.
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

/// The judge of `C-PSRB-BOLL-01` and `C-PSRB-BOLL-02`, whose wrapper is in
/// `crates/indicators/benches/ratio.rs`: every cost's p99 is within
/// `ceiling_permille` thousandths of the FIRST cost's p99 (flat across the
/// sizes) and at most `budget_ns`.
///
/// No costs, or a first p99 of zero, is a failure: a step timed at nothing has
/// been shown absent, not constant, which is the harness's own rule.
#[must_use]
pub fn flat_within(costs: &[StepCost], ceiling_permille: u64, budget_ns: u64) -> bool {
    let Some(first) = costs.first() else {
        return false;
    };
    let base = u128::from(first.p99_ns);
    base > 0
        && costs.iter().all(|cost| {
            u128::from(cost.p99_ns) * 1_000 <= base * u128::from(ceiling_permille)
                && cost.p99_ns <= budget_ns
        })
}

/// The period of `C-PSRB-BOLL-01`'s step: 20 closes.
const STEP_PERIOD: u16 = 20;

/// The candles folded before `C-PSRB-BOLL-02` times its step: 20,000.
const PERIOD_HISTORY: u64 = 20_000;

/// The width both bench steps judge at: z 2.000.
const STEP_WIDTH: Width = Width(2_000);
const _: () = assert!(STEP_WIDTH.0 <= MAX_WIDTH_MILLI);

/// Where the bench's synthetic walk starts: Rs 25,000.00, in paisa.
const WALK_BASE: i64 = 2_500_000;

/// One minute, in microseconds: the walk's spacing.
const MINUTE_MICROS: i64 = 60_000_000;

/// `C-PSRB-BOLL-01`'s measurement (D-8595): a window of 20 closes is fed
/// `history` candles of the synthetic walk, then `rounds` rounds of `samples`
/// steps are each timed alone, in nanoseconds, unsorted.
///
/// A step is one [`Bollinger::fold`], one [`Bollinger::window`] and the two
/// population comparisons [`Window::above_upper`] and [`Window::below_lower`]
/// at z 2.000, of the close just folded. Nothing is printed here; the wrapper
/// in `crates/indicators/benches/ratio.rs` reads and judges.
///
/// # Errors
///
/// A fold's refusal, which only a walk long enough to drift out of
/// `1..=MAX_CLOSE` paisa can make: the walk moves at most 4 paisa a candle from
/// Rs 25,000.00.
pub fn measure_step(history: u64, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal> {
    measure(STEP_PERIOD, history, samples, rounds, WALK_BASE)
}

/// `C-PSRB-BOLL-02`'s measurement (D-8595): the same step as
/// [`measure_step`], at a window of `period` closes after 20,000 candles.
///
/// # Errors
///
/// [`Refusal::PeriodZero`] and [`Refusal::PeriodAboveMax`] for a period
/// [`Bollinger::new`] refuses, and a fold's refusal as for [`measure_step`].
pub fn measure_period(period: u16, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal> {
    measure(period, PERIOD_HISTORY, samples, rounds, WALK_BASE)
}

/// Both measurements, with the walk's start a parameter so a test can make
/// the walk's candles refuse.
fn measure(
    period: u16,
    history: u64,
    samples: u64,
    rounds: u64,
    base: i64,
) -> Result<Vec<Vec<u64>>, Refusal> {
    let mut bands = Bollinger::new(period)?;
    let mut walk = Walk::new(base);
    for _ in 0..history {
        bands.fold(&walk.next_candle())?;
    }
    let mut timings = Vec::with_capacity(usize::try_from(rounds).unwrap_or(0));
    for _ in 0..rounds {
        let mut round = Vec::with_capacity(usize::try_from(samples).unwrap_or(0));
        for _ in 0..samples {
            let candle = walk.next_candle();
            let began = Instant::now();
            let folded = bands.fold(black_box(&candle));
            black_box(bands.window().map(judge));
            let elapsed = began.elapsed().as_nanos();
            folded?;
            round.push(u64::try_from(elapsed).unwrap_or(u64::MAX));
        }
        timings.push(round);
    }
    Ok(timings)
}

/// The two population comparisons of a bench step, of the window's own last
/// close.
fn judge(window: Window) -> (bool, bool) {
    let close = window.last();
    (
        window.above_upper(close, STEP_WIDTH),
        window.below_lower(close, STEP_WIDTH),
    )
}

/// The bench's synthetic candles: a deterministic xorshift walk of the close,
/// moving -4..=4 paisa a minute, every price of a candle its close. Invented;
/// no vendor row.
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
            self.stamp, self.close, self.close, self.close, self.close, 1, OI_NULL,
        )
    }
}

#[cfg(test)]
#[path = "bollinger_tests.rs"]
mod tests;
