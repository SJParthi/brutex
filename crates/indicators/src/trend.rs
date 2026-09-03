//! Moving averages, swing levels, `SuperTrend` and market structure.
//!
//! **14 vocabulary positions**: 0–5, 56–59, 64–65 and 72–73 — the last of the 232
//! live positions that had no computation behind them.
//!
//! # Every threshold here is UNVERIFIED, and that is stated rather than hidden
//!
//! None of these four families can be written without numbers, and **no tracked
//! document in this repository defines any of them.** §3 rule 1 forbids inventing a
//! number and passing it off as measured, so every one is gathered into
//! [`TrendThresholds`], named, given its classical value, and labelled. They are the
//! widely-used conventions — a 20 and 200 period EMA, a 10-period ATR with a 3×
//! multiplier, a 2-bar fractal — and they are not traceable to NSE or to anything
//! `docs/00-charter.md` records.
//!
//! This is the same position `crate::pattern` takes for its six pattern thresholds,
//! and taking it consistently is the point: a repository that declares six
//! conventions and refuses a seventh is not applying a rule, it is applying an
//! accident of which module was written first.
//!
//! # No floating point, anywhere (§7)
//!
//! An EMA is `ema += α(price − ema)` with `α = 2/(n+1)` — a fraction, which is where
//! a float usually enters. It does not here. The average is held **scaled by
//! [`SCALE`]** in `i128` and the update is written as one integer expression:
//!
//! ```text
//! ema += (price·SCALE − ema) · 2 / (n + 1)
//! ```
//!
//! The division truncates, and that is a real approximation rather than a hidden one:
//! at `SCALE = 1_000_000` a paisa is held to six further digits, so the truncation is
//! below one ten-thousandth of a paisa per step. Deterministic, which §3 rule 5
//! requires — the same input gives the same average on every machine, which a float
//! sum does not guarantee.
//!
//! ATR uses Wilder's smoothing, `atr += (tr − atr)/n`, in the same scaled form.
//!
//! # A period is FOLDED before it is named
//!
//! Every average here is seeded with its first value, which means it holds a number from
//! the first candle onward — and for the next `period − 1` candles that number is a fact
//! about where the run started, not a `period`-candle measurement. Emitting off it was a
//! real defect: `TrendState::new(CLASSICAL)` fed a close of 10,000 paisa and then 20,000
//! set positions 0, 2 and 64 on the **second** candle, so `close_above_ema200` meant
//! "close above the previous close" and `close_above_supertrend` meant "close above a
//! stop seeded from one candle's high-low span". Three positions the vocabulary names as
//! a 20-period, a 200-period and a 10-period-ATR measurement, and nothing downstream
//! could tell them from converged ones.
//!
//! So [`Ema`] and [`Atr`] count the candles folded into them and answer `warm`, and
//! [`TrendState::bits`] emits 0–5 and 64–65 only once the accumulator behind each has
//! folded its own period. `docs/03-vocabulary.md` §4: a bit that cannot be evaluated
//! evaluates **false**, and never "probably".
//!
//! The gate is on the **emission**, deliberately not on [`Ema::value`] or [`Atr::value`].
//! Those are the accumulator's honest state and other code needs it — [`SuperTrend`]
//! seeds its stop off the ATR's first value, and withholding it would leave the stop
//! unestablished for ten candles rather than merely unemitted, which is a different and
//! worse change than the one this defect asked for.
//!
//! # A swing is confirmed LATE, and that is not look-ahead
//!
//! A swing high is a bar whose high exceeds the `fractal` bars on **each** side. So
//! the bar at index *i* cannot be known to be a swing until bar *i + fractal* has
//! arrived. This module therefore publishes a swing level `fractal` bars after the
//! swing occurred.
//!
//! That delay is the whole reason it is sound. Confirming a swing at the bar itself
//! would require reading bars *i+1 … i+fractal*, which is exactly the look-ahead §3
//! rule 7 forbids. The level is a **delayed anchor**: it refers to the past and
//! becomes available later, which is different from a level that refers to the
//! future. Nothing here ever holds a bar it has not been given.
//!
//! # Cost
//!
//! Two scaled accumulators for the EMAs, one for ATR, a `SuperTrend` latch of two
//! prices and a boolean, and a fixed `2·fractal + 1` ring for the swing detector.
//! `size_of` is asserted below. Nothing grows with the number of candles fed, and
//! there is no allocation on the per-candle path.

use crate::Candle;
use vocab::{ConditionMask, Tolerance};

/// Fixed-point scale for the running averages: six digits below a paisa.
///
/// Large enough that the truncation in each update is negligible against a paisa,
/// small enough that `price · SCALE · 2` cannot approach `i128::MAX` for any `i64`
/// price — `9.2 × 10^18 × 10^6 × 2` is `1.8 × 10^25`, thirteen orders inside it.
pub const SCALE: i128 = 1_000_000;

/// Every threshold these four families need.
///
/// **UNVERIFIED.** These are the classical conventions. None is traceable to NSE, to
/// a vendor, or to any source `docs/00-charter.md` records, and none has been measured
/// against data. They are gathered here so the set is auditable in one place and a
/// change is one edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrendThresholds {
    /// Fast EMA period. 20.
    pub ema_fast: i128,
    /// Slow EMA period. 200.
    pub ema_slow: i128,
    /// ATR period for `SuperTrend`. 10.
    pub atr_period: i128,
    /// `SuperTrend` band multiplier, in thousandths. 3000 = 3×ATR.
    pub supertrend_mult: i128,
}

// Two fields used to sit here and NEITHER WAS EVER READ, which is worse than a magic
// number: a knob that looks adjustable and is not, in the public API of a crate another
// repository is meant to consume.
//
//   `fractal: usize` — stored on the detector and never consulted. The window is
//     [`RING`] at every use, and it has to be: the ring is a fixed-size array, so its
//     width is a compile-time property and a runtime struct cannot change it. Setting
//     `fractal: 3` on a `TrendThresholds` value changed nothing and reported nothing.
//     It is now [`FRACTAL`], derived FROM `RING` so the two cannot disagree.
//
//   `swing_band: i64` — never read anywhere. The band for positions 72 and 73 comes
//     from the `Tolerance` the caller passes, scaled by the confirming window's own
//     span. A second band knob here would have given two answers to one question, and
//     `vocab`'s is the one every other `Kind::Near` position in the vocabulary uses.
//
// Both are gone rather than wired up, because wiring them would mean either a
// heap-allocated ring (costing the constant-space property) or a second tolerance
// mechanism competing with `vocab`'s.

impl Default for TrendThresholds {
    fn default() -> Self {
        Self::CLASSICAL
    }
}

impl TrendThresholds {
    /// The classical set. **UNVERIFIED** — see the struct documentation.
    pub const CLASSICAL: Self = Self {
        ema_fast: 20,
        ema_slow: 200,
        atr_period: 10,
        supertrend_mult: 3_000,
    };
}

/// The swing window, in candles. Five: a peak tested against two neighbours each side.
///
/// A compile-time constant because the detector's ring is a fixed-size array, which is
/// what keeps the state O(1) in candles fed. Measured by `C-I-01`, in
/// `crates/indicators/benches/ratio.rs`: the per-candle cost is 1.009x at 200,000
/// candles folded against 1,000.
pub const RING: usize = 5;

/// Candles either side that a swing must exceed, **derived** from [`RING`].
///
/// Derived rather than declared, so the two cannot drift. It was a field on
/// [`TrendThresholds`] that nothing read: the window was `RING` at every use, so a
/// caller setting it got silence.
pub const FRACTAL: usize = RING / 2;

const _: () = assert!(
    RING == 2 * FRACTAL + 1,
    "the ring must hold a whole fractal window: an even RING has no middle candle, so \
     there is no bar to test against both sides"
);
const _: () = assert!(RING % 2 == 1, "a fractal window needs a middle");

/// An exponential moving average, held scaled and updated in integers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ema {
    period: i128,
    scaled: i128,
    /// Candles folded, saturating. Read only by [`Ema::warm`], which is what keeps a
    /// two-candle seed from being emitted as a 200-period measurement.
    folded: u64,
    seeded: bool,
}

impl Ema {
    /// A fresh average over `period` candles.
    #[must_use]
    pub const fn new(period: i128) -> Self {
        Self {
            period,
            scaled: 0,
            folded: 0,
            seeded: false,
        }
    }

    /// Fold one price in.
    ///
    /// Seeded with the first price rather than a simple average of the first `period`.
    /// That is a convention and it is stated: seeding with an SMA needs `period`
    /// candles of buffer, which would make the state grow with the period and cost the
    /// constant-space property for no gain in a long run. The two agree to within the
    /// smoothing constant after a few periods — which is why [`Ema::warm`] exists and
    /// why no vocabulary position is emitted before it is true.
    pub fn fold(&mut self, price: i64) {
        // Counted before either early return below, because both of them absorb the
        // candle: the seeding branch and the degenerate-period branch each `return`, and
        // a count taken at the end of the function would miss every candle folded into a
        // seed — which is exactly the run of candles this counter has to measure.
        self.folded = self.folded.saturating_add(1);
        let target = i128::from(price).saturating_mul(SCALE);
        if !self.seeded {
            self.scaled = target;
            self.seeded = true;
            return;
        }
        let denominator = self.period.saturating_add(1);
        if denominator <= 0 {
            return;
        }
        // ema += (target - ema) * 2 / (n + 1), all integer. `div_euclid` and not `/`:
        // the numerator is negative whenever price is below the average, and
        // truncation toward zero would bias the average upward on every down step.
        let delta = target.saturating_sub(self.scaled);
        let step = delta.saturating_mul(2).div_euclid(denominator);
        self.scaled = self.scaled.saturating_add(step);
    }

    /// The average in paisa, or `None` before the first candle.
    #[must_use]
    pub fn value(&self) -> Option<i64> {
        if !self.seeded {
            return None;
        }
        i64::try_from(self.scaled.div_euclid(SCALE)).ok()
    }

    /// True once `period` candles have been folded.
    ///
    /// The seed is the first price, so until then the average is that price plus a
    /// handful of steps — a fact about where the run started rather than about the
    /// market. Measured: a state fed 10,000 paisa and then 20,000 set
    /// `close_above_ema200`, where the "200-period average" was the previous close.
    ///
    /// **Separate from [`Ema::value`] on purpose, and this is the part a fix gets
    /// wrong.** The average is a real number from the first candle and every caller
    /// that wants it must keep getting it — [`SuperTrend::fold`] seeds its stop off the
    /// ATR's first value, and returning `None` there would leave the stop unestablished
    /// for ten candles instead of unemitted. What is gated is the **emission** of a
    /// vocabulary position, in [`TrendState::bits`], and nothing else.
    #[must_use]
    pub fn warm(&self) -> bool {
        i128::from(self.folded) >= self.period
    }
}

/// Average true range, Wilder-smoothed, held scaled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Atr {
    period: i128,
    scaled: i128,
    previous_close: Option<i64>,
    /// Candles folded, saturating. Read only by [`Atr::warm`].
    folded: u64,
    seeded: bool,
}

impl Atr {
    /// A fresh ATR over `period` candles.
    #[must_use]
    pub const fn new(period: i128) -> Self {
        Self {
            period,
            scaled: 0,
            previous_close: None,
            folded: 0,
            seeded: false,
        }
    }

    /// True range: the widest of the three classical spans.
    fn true_range(&self, candle: &Candle) -> i128 {
        let hl = i128::from(candle.high).saturating_sub(i128::from(candle.low));
        let Some(previous) = self.previous_close else {
            return hl;
        };
        let p = i128::from(previous);
        let hc = i128::from(candle.high).saturating_sub(p);
        let lc = p.saturating_sub(i128::from(candle.low));
        // Two-sided rather than `abs()`: `i64::MIN.abs()` panics in debug and wraps in
        // release, and the two profiles disagreeing about a bug is what §3 rule 5 dies
        // of. These are already `i128` so the risk is theoretical — the habit is not.
        let hc = if hc < 0 { -hc } else { hc };
        let lc = if lc < 0 { -lc } else { lc };
        let mut widest = hl;
        if hc > widest {
            widest = hc;
        }
        if lc > widest {
            widest = lc;
        }
        widest
    }

    /// Fold one candle in.
    ///
    /// The first candle seeds the range at its own high-low span — there is no previous
    /// close to gap from — so the seed is a one-candle range wearing a `period`-candle
    /// label until [`Atr::warm`] is true.
    pub fn fold(&mut self, candle: &Candle) {
        // Counted first, for the same reason as in `Ema::fold`: the seeding branch below
        // absorbs a candle without reaching the smoothing step.
        self.folded = self.folded.saturating_add(1);
        let tr = self.true_range(candle).saturating_mul(SCALE);
        if self.seeded {
            let denominator = self.period;
            if denominator > 0 {
                let delta = tr.saturating_sub(self.scaled);
                self.scaled = self.scaled.saturating_add(delta.div_euclid(denominator));
            }
        } else {
            self.scaled = tr;
            self.seeded = true;
        }
        self.previous_close = Some(candle.close);
    }

    /// The range in paisa, or `None` before the first candle.
    #[must_use]
    pub fn value(&self) -> Option<i64> {
        if !self.seeded {
            return None;
        }
        i64::try_from(self.scaled.div_euclid(SCALE)).ok()
    }

    /// True once `period` candles have been folded. See [`Ema::warm`] for why this is
    /// beside [`Atr::value`] rather than inside it.
    #[must_use]
    pub fn warm(&self) -> bool {
        i128::from(self.folded) >= self.period
    }
}

/// Which side of the `SuperTrend` the market is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trend {
    /// Price is above the trailing stop.
    Up,
    /// Price is below it.
    Down,
}

/// The `SuperTrend` trailing stop.
///
/// A latch, not a formula: the band only ever moves in the favourable direction while
/// the trend holds, and flips when price closes through it. That ratchet is the whole
/// indicator — a band recomputed from scratch each candle would whipsaw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuperTrend {
    atr: Atr,
    multiplier: i128,
    stop: Option<i64>,
    trend: Trend,
}

impl SuperTrend {
    /// A fresh trailing stop.
    #[must_use]
    pub const fn new(thresholds: TrendThresholds) -> Self {
        Self {
            atr: Atr::new(thresholds.atr_period),
            multiplier: thresholds.supertrend_mult,
            stop: None,
            trend: Trend::Up,
        }
    }

    /// The stop's price, or `None` before it is established.
    #[must_use]
    pub const fn stop(&self) -> Option<i64> {
        self.stop
    }

    /// Which side of it the market is on.
    #[must_use]
    pub const fn trend(&self) -> Trend {
        self.trend
    }

    /// True once the ATR under the band has folded its own period.
    ///
    /// The stop itself exists from the first candle and [`SuperTrend::stop`] keeps
    /// reporting it, because the ratchet needs a level to move from and the seed is the
    /// only level there is. This answers the different question positions 64 and 65
    /// actually ask: whether that level is yet a `period`-candle range rather than the
    /// first candle's own high-low span.
    #[must_use]
    pub fn warm(&self) -> bool {
        self.atr.warm()
    }

    /// Fold one candle in, moving or flipping the stop.
    pub fn fold(&mut self, candle: &Candle) {
        self.atr.fold(candle);
        let Some(atr) = self.atr.value() else { return };
        let mid = i128::from(candle.high).midpoint(i128::from(candle.low));
        let band = i128::from(atr)
            .saturating_mul(self.multiplier)
            .div_euclid(1000);
        let upper = mid.saturating_add(band);
        let lower = mid.saturating_sub(band);
        let close = i128::from(candle.close);

        let Some(previous) = self.stop else {
            // Seed on the side the first candle implies, so the first emitted bit is
            // not an artefact of an arbitrary starting trend.
            let (trend, stop) = if close >= mid {
                (Trend::Up, lower)
            } else {
                (Trend::Down, upper)
            };
            self.trend = trend;
            self.stop = i64::try_from(stop).ok();
            return;
        };
        let previous = i128::from(previous);

        let (trend, stop) = match self.trend {
            Trend::Up => {
                if close < previous {
                    (Trend::Down, upper)
                } else {
                    // Ratchet: the stop rises with the band and never falls.
                    (Trend::Up, if lower > previous { lower } else { previous })
                }
            }
            Trend::Down => {
                if close > previous {
                    (Trend::Up, lower)
                } else {
                    (Trend::Down, if upper < previous { upper } else { previous })
                }
            }
        };
        self.trend = trend;
        if let Ok(next) = i64::try_from(stop) {
            self.stop = Some(next);
        }
    }
}

/// A confirmed swing, and the window it was confirmed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swing {
    /// The swing's price.
    pub price: i64,
    /// The high-to-low span of the window that confirmed it, used as the band's
    /// range because a swing has no range of its own.
    pub window_span: i64,
    /// A monotonic sequence number for the candle that confirmed it.
    ///
    /// Needed because the swing high and the swing low are **independent latches
    /// confirmed at different times**, so the more recent one is the structure
    /// currently in force. Without it, [`Structure::observe`] cannot tell which of two
    /// simultaneous breaks matters — and its first version assumed simultaneity was
    /// impossible, which suppressed every real break in a trending market.
    pub confirmed_at: u64,
}

/// The fractal swing detector.
///
/// Holds `2 · fractal + 1` candles. The middle one is tested against both sides, so a
/// swing is published `fractal` candles after it happened — see the module
/// documentation for why that delay is what makes it sound rather than what makes it
/// late.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwingDetector {
    ring: [Option<(i64, i64)>; RING],
    next: usize,
    filled: usize,
    /// Candles folded, saturating. Only used to order the two latches.
    seen: u64,
    high: Option<Swing>,
    low: Option<Swing>,
}

impl Default for SwingDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SwingDetector {
    /// A fresh detector.
    ///
    /// It takes no thresholds because it reads none: the window is [`RING`], a const,
    /// and the tolerance a swing is judged against belongs to the caller's
    /// [`Tolerance`](crate::tolerance::Tolerance), not to a second knob here.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ring: [None; RING],
            next: 0,
            filled: 0,
            seen: 0,
            high: None,
            low: None,
        }
    }

    /// The most recently confirmed swing high.
    #[must_use]
    pub const fn swing_high(&self) -> Option<Swing> {
        self.high
    }

    /// The most recently confirmed swing low.
    #[must_use]
    pub const fn swing_low(&self) -> Option<Swing> {
        self.low
    }

    /// Fold one candle in and confirm a swing if the window now shows one.
    pub fn fold(&mut self, candle: &Candle) {
        self.seen = self.seen.saturating_add(1);
        // A scan across the fixed window, and NOT `ring.get_mut(self.next)`. `next` is
        // `(next + 1) % RING`, so that `get_mut` could never answer `None` — and a
        // branch no input can reach is a branch no test can hold anyone to, which is
        // the same objection this module raised against a field nobody reads. Five
        // comparisons over a five-slot array is the same constant cost, and both arms
        // of the one below run on every candle.
        let entry = Some((candle.high, candle.low));
        for (i, slot) in self.ring.iter_mut().enumerate() {
            if i == self.next {
                *slot = entry;
            }
        }
        self.next = self.next.saturating_add(1) % RING;
        if self.filled < RING {
            self.filled = self.filled.saturating_add(1);
            if self.filled < RING {
                return;
            }
        }
        self.confirm();
    }

    /// Test the middle of the window against both sides.
    ///
    /// Chronological order is recovered by rotating the ring so the oldest entry is
    /// first: `next` points at the slot the *next* candle will overwrite, which is the
    /// oldest one. The middle is [`FRACTAL`], the const derived from [`RING`], rather
    /// than `RING.checked_div(2)` — the divisor is a literal, so that `checked_div`
    /// could never answer `None` either.
    ///
    /// Both `else` arms below are defensive and **neither can run**: this is called
    /// only when `filled == RING`, so every slot holds a candle. They are left as a
    /// silent non-publication rather than turned into a panic, because publishing
    /// nothing is the safe answer to a state that cannot occur.
    fn confirm(&mut self) {
        let mut window = self.ring;
        window.rotate_left(self.next);
        let Some(Some((mid_high, mid_low))) = window.get(FRACTAL).copied() else {
            return;
        };

        let mut span_high = mid_high;
        let mut span_low = mid_low;
        let mut is_high = true;
        let mut is_low = true;
        for (i, entry) in window.iter().enumerate() {
            let Some((high, low)) = *entry else {
                return;
            };
            if high > span_high {
                span_high = high;
            }
            if low < span_low {
                span_low = low;
            }
            if i == FRACTAL {
                continue;
            }
            // Strict on both sides: a plateau is not a swing. Two adjacent equal highs
            // would otherwise both be "the" swing high, and a level that two bars can
            // claim is not a level.
            if high >= mid_high {
                is_high = false;
            }
            if low <= mid_low {
                is_low = false;
            }
        }
        // `checked_sub` and not `saturating_sub`, and the difference is a lost bit rather
        // than a rounding. The five-candle window's extremes come from FIVE DIFFERENT
        // BARS, so two of them at opposite ends of `i64` give a true span of up to
        // 1.84e19 while `saturating_sub` returns `i64::MAX` — a band **half** the real
        // width. A close inside the missing half then sets no `near_swing` bit and
        // NOTHING REFUSES, which is the §4 fallback that hides a failure: the sweep is
        // handed a silent false on a bar the band actually covers.
        //
        // On saturation the swing is not published at all. That is the same choice
        // `Candle::range` and `CurDayFib::range` already make, and it is why the three
        // sibling sites in `orb.rs`, `fib.rs` and `gap.rs` abstain rather than clamp.
        // Abstaining loses the swing; saturating loses a bit and says nothing.
        let Some(span) = span_high.checked_sub(span_low) else {
            return;
        };
        if is_high {
            self.high = Some(Swing {
                price: mid_high,
                window_span: span,
                confirmed_at: self.seen,
            });
        }
        if is_low {
            self.low = Some(Swing {
                price: mid_low,
                window_span: span,
                confirmed_at: self.seen,
            });
        }
    }
}

/// Market structure: which way the last decisive break went.
///
/// A **break of structure** is price closing beyond the last confirmed swing in the
/// direction the structure was already going. A **change of character** is the first
/// break in the *opposite* direction — the flip. Distinguishing them needs one bit of
/// memory, which is the whole of this type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Structure {
    /// The direction of the last break, or `None` before any.
    last: Option<Trend>,
}

/// What a candle did to the structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Break {
    /// Broke up, continuing an up structure.
    BosBullish,
    /// Broke down, continuing a down structure.
    BosBearish,
    /// Broke up against a down structure — the flip.
    ChochBullish,
    /// Broke down against an up structure.
    ChochBearish,
}

impl Structure {
    /// A fresh structure, with no break yet seen.
    #[must_use]
    pub const fn new() -> Self {
        Self { last: None }
    }

    /// Classify this candle's break, if it broke anything.
    ///
    /// The very first break is a **`BoS`**, not a `CHoCH`: with no prior direction there is
    /// no character to change. Calling it a `CHoCH` would make the first break of every
    /// run a flip, which is an artefact of starting rather than a fact about price.
    /// # Why this is `&self`, and returns the direction beside the break
    ///
    /// It was `observe(&mut self, ..)` and it wrote `self.last` from inside the EMIT.
    /// Calling it twice on one candle gave two different answers — a first down break
    /// after an up break returned `ChochBearish`, and the identical call then returned
    /// `BosBearish`, because the first call had already moved the latch. So
    /// `TrendState::bits` was not idempotent and any second look at the same bar
    /// permanently changed the mask the sweep later received.
    ///
    /// Classification is now pure and the latch advance is [`Self::advance`], called once
    /// per candle from `TrendState::step`. The direction travels with the break because
    /// the caller needs it in order to advance, and re-deriving it would be a second
    /// place to get the recency tie-break wrong.
    #[must_use]
    pub fn classify(
        &self,
        close: i64,
        swing_high: Option<Swing>,
        swing_low: Option<Swing>,
    ) -> Option<(Break, Trend)> {
        let broke_up = swing_high.is_some_and(|s| close > s.price);
        let broke_down = swing_low.is_some_and(|s| close < s.price);
        // Both at once IS reachable, and the first version of this function said it was
        // not. The swing high and the swing low are independent latches confirmed at
        // different candles, so in a rising market a low confirmed five candles ago
        // routinely sits ABOVE a high confirmed twenty candles ago. A close between them
        // then breaks both, and returning `None` there suppressed every real structure
        // break for as long as the stale latch survived — which in a trend is most of it.
        //
        // Resolved by recency rather than by refusing: "structure" means the structure
        // currently in force, and the more recently confirmed swing is that one. The
        // older latch has been superseded by price moving past it.
        let direction = match (broke_up, broke_down) {
            (false, false) => return None,
            (true, false) => Trend::Up,
            (false, true) => Trend::Down,
            (true, true) => {
                let up_at = swing_high.map_or(0, |s| s.confirmed_at);
                let down_at = swing_low.map_or(0, |s| s.confirmed_at);
                match up_at.cmp(&down_at) {
                    core::cmp::Ordering::Greater => Trend::Up,
                    core::cmp::Ordering::Less => Trend::Down,
                    // Confirmed on the same candle, which the detector can do when one
                    // window is both a peak and a trough — a single candle straddling
                    // its neighbours. Neither supersedes the other, so neither is the
                    // structure and there is nothing to report.
                    core::cmp::Ordering::Equal => return None,
                }
            }
        };
        let result = match (self.last, direction) {
            (Some(Trend::Down), Trend::Up) => Break::ChochBullish,
            (Some(Trend::Up), Trend::Down) => Break::ChochBearish,
            (_, Trend::Up) => Break::BosBullish,
            (_, Trend::Down) => Break::BosBearish,
        };
        Some((result, direction))
    }

    /// Advance the in-force direction. Called once per candle, from `TrendState::step`.
    pub const fn advance(&mut self, direction: Trend) {
        self.last = Some(direction);
    }

    /// The direction currently in force, or `None` before the first break.
    #[must_use]
    pub const fn in_force(&self) -> Option<Trend> {
        self.last
    }
}

impl Default for Structure {
    fn default() -> Self {
        Self::new()
    }
}

/// All fourteen positions, and the state they need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrendState {
    fast: Ema,
    slow: Ema,
    supertrend: SuperTrend,
    swings: SwingDetector,
    structure: Structure,
    thresholds: TrendThresholds,
}

const _: () = assert!(core::mem::size_of::<TrendState>() <= 512);

impl Default for TrendState {
    fn default() -> Self {
        Self::new(TrendThresholds::CLASSICAL)
    }
}

impl TrendState {
    /// A fresh state.
    #[must_use]
    pub const fn new(thresholds: TrendThresholds) -> Self {
        Self {
            fast: Ema::new(thresholds.ema_fast),
            slow: Ema::new(thresholds.ema_slow),
            supertrend: SuperTrend::new(thresholds),
            swings: SwingDetector::new(),
            structure: Structure::new(),
            thresholds,
        }
    }

    /// The full per-candle step: **emit, then fold.**
    ///
    /// Every quantity here is an **anchor** — a level the candle is measured against —
    /// so all of them must exclude it. An EMA that had already absorbed the current
    /// close would make `close_above_ema20` compare a price against an average
    /// containing it, and on the first candle of a run that comparison is always
    /// false by construction.
    ///
    /// # Errors
    ///
    /// [`crate::Corrupt`] for a record that is not a candle.
    pub fn step(
        &mut self,
        candle: &Candle,
        tolerance: Tolerance,
    ) -> Result<ConditionMask, crate::Corrupt> {
        // `check_evaluable`, not `check`: this module must refuse exactly what
        // `Evaluator::stepped` refuses or the mask is a mixture of two answers.
        candle.check_evaluable()?;
        // ONE classification, used for both the mask and the latch — not two calls that
        // happen to agree. A sweep proved that shape unguarded by folding the candle before
        // the advance, and by advancing on `candle.open` while the emit used `candle.close`.
        // Neither was caught by anything. Sharing the value makes both unconstructible.
        let (bits, broke) = self.emit(candle.close, tolerance);
        if let Some((_, direction)) = broke {
            self.structure.advance(direction);
        }
        // The fold comes LAST. The structure is read against the swings as they stood BEFORE
        // this candle — an anchor excludes the bar it is measured against — and moving this
        // line above the advance was the other uncaught mutation.
        self.fold(candle);
        Ok(bits)
    }

    /// Fold the candle into every accumulator.
    fn fold(&mut self, candle: &Candle) {
        self.fast.fold(candle.close);
        self.slow.fold(candle.close);
        self.supertrend.fold(candle);
        self.swings.fold(candle);
        // Structure is read from the swings as they stood BEFORE this candle, which is
        // why `observe` runs inside the emit and not here.
    }

    /// Can every one of the fourteen positions answer yet?
    ///
    /// Distinct from [`Self::warm`], which answers the narrower question positions 64 and 65
    /// ask — whether the ATR is a `period`-candle range. This is the conjunction: the two
    /// EMAs as well, so positions 0–5 are included. At `CLASSICAL` the binding constraint is
    /// the 200-candle average.
    ///
    /// Folded into `Evaluator::every_family_can_answer`, which a caller needs so that a
    /// sweep does not start on bars where whole families are structurally silent and depress
    /// every support they touch.
    #[must_use]
    pub fn every_position_can_answer(&self) -> bool {
        self.fast.warm() && self.slow.warm() && self.supertrend.warm()
    }

    /// The market-structure direction currently in force, for positions 278–279.
    ///
    /// `None` before the first break: there is no structure yet, and
    /// `docs/03-vocabulary.md` §4 forbids a bit evaluating to "probably". Bits 56–59 are the
    /// break EVENTS; this is the regime between them, and it was computed and unpublished.
    #[must_use]
    pub const fn structure_in_force(&self) -> Option<Trend> {
        self.structure.in_force()
    }

    /// The fourteen positions for one closing price.
    ///
    /// # Cost
    ///
    /// Two average comparisons, one stop comparison, two band tests and one structure
    /// classification. No loop over data, no allocation.
    #[must_use]
    pub fn bits(&self, close: i64, tolerance: Tolerance) -> ConditionMask {
        self.emit(close, tolerance).0
    }

    /// The mask, **and the classification it was built from**.
    ///
    /// # Why this exists rather than two `classify` calls
    ///
    /// `step` needs the break for the mask and the direction for the latch. It used to call
    /// `classify` twice — once inside `bits`, once for the advance — with arguments that
    /// merely happened to match. A verification sweep proved that unguarded two ways:
    ///
    /// * folding the candle before the advance, so the latch classified POST-fold swings
    ///   while the emit had used pre-fold ones;
    /// * advancing on `candle.open` while the emit classified `candle.close`.
    ///
    /// **Neither was caught by anything**, and both make the regime disagree with the bits
    /// that reported it — a bit that reads as a measurement and is an artefact of which of
    /// two readings won.
    ///
    /// One call, handed to both consumers, makes that disagreement UNCONSTRUCTIBLE rather
    /// than merely tested. Same reasoning as `bits(&self)`: prefer the shape in which the
    /// mistake cannot be written.
    fn emit(&self, close: i64, tolerance: Tolerance) -> (ConditionMask, Option<(Break, Trend)>) {
        let mut mask = ConditionMask::ZERO;

        // 0–3: close against each average. A position stays false while its average is
        // unseeded — and while that average has folded fewer candles than the period its
        // NAME claims — rather than guessing. docs/03-vocabulary.md §4.
        //
        // `warm` beside `value` and not inside it. Without the counter these bits fired
        // on the second candle of every run: fed 10,000 paisa and then 20,000, position 2
        // `close_above_ema200` compared the close against the PREVIOUS CLOSE and set,
        // and nothing downstream could tell that bit from one backed by two hundred
        // candles. Folding the gate into `value` instead would have starved
        // `SuperTrend::fold` of its seed — see `Ema::warm`.
        if self.fast.warm()
            && let Some(fast) = self.fast.value()
        {
            mask = side(mask, close, fast, 0, 1);
        }
        if self.slow.warm()
            && let Some(slow) = self.slow.value()
        {
            mask = side(mask, close, slow, 2, 3);
        }
        // 4–5: the averages against each other. BOTH must have folded their own period,
        // so the slow one governs and this pair is the last of the six to speak. Gating
        // on the fast average alone would compare a converged 20 against a seeded 200,
        // which is a statement about the seed and not about the market.
        if self.fast.warm()
            && self.slow.warm()
            && let (Some(fast), Some(slow)) = (self.fast.value(), self.slow.value())
        {
            mask = side(mask, fast, slow, 4, 5);
        }

        // 64–65: close against the trailing stop, once the range under the band is an
        // `atr_period`-candle measurement. The seeded stop comes off ONE candle's
        // high-low span, so before this the side price sits on is a fact about the first
        // candle of the run.
        if self.supertrend.warm()
            && let Some(stop) = self.supertrend.stop()
        {
            mask = side(mask, close, stop, 64, 65);
        }

        // 72–73: near a confirmed swing. `Kind::Near`, so `vocab` gates the band.
        for (swing, index) in [
            (self.swings.swing_high(), 72_u16),
            (self.swings.swing_low(), 73_u16),
        ] {
            if let Some(s) = swing
                && tolerance.covers(close, s.price, s.window_span)
                && let Ok(next) =
                    vocab::table::set_near(mask, index, tolerance, close, s.price, s.window_span)
            {
                mask = next;
            }
        }

        // 56–59: structure. Read the swings as they stand before this candle is folded.
        let broke =
            self.structure
                .classify(close, self.swings.swing_high(), self.swings.swing_low());
        if let Some((b, _)) = broke {
            let index = match b {
                Break::BosBullish => 56,
                Break::BosBearish => 57,
                Break::ChochBullish => 58,
                Break::ChochBearish => 59,
            };
            mask = set(mask, index);
        }
        (mask, broke)
    }

    /// The thresholds in force, for a caller that needs to record them.
    #[must_use]
    pub const fn thresholds(&self) -> TrendThresholds {
        self.thresholds
    }

    /// Every position this module can set.
    #[must_use]
    pub const fn positions() -> [u16; 14] {
        [0, 1, 2, 3, 4, 5, 56, 57, 58, 59, 64, 65, 72, 73]
    }
}

/// Set `index`, refusing anything the vocabulary rejects.
///
/// A refusal means the table and this module disagree about a position, which
/// `the_positions_match_the_vocabulary` makes a test failure rather than a runtime
/// surprise. Setting nothing is the safe response.
fn set(mask: ConditionMask, index: u16) -> ConditionMask {
    vocab::table::set_exact(mask, index).unwrap_or(mask)
}

/// Set `above` when `value` exceeds `level`, `below` when it is under, and **neither
/// when they are equal**.
///
/// Written as one function rather than three `if / else` pairs because the equality
/// case is where this family would otherwise go wrong, and an `else` collects it
/// silently. "Above" would become "not below", so a close sitting exactly on the
/// average would report itself as beneath it — the same defect that made position 227
/// fire on every zero-body bar and handed an undirected tri-star to the bearish bit.
/// Making it structural means it cannot be forgotten at the fourth call site.
fn side(mask: ConditionMask, value: i64, level: i64, above: u16, below: u16) -> ConditionMask {
    // A `match` on the ordering rather than an `if` chain: `Ordering` has exactly
    // three variants, so the compiler checks the equality arm exists instead of a
    // reader having to notice it does.
    match value.cmp(&level) {
        core::cmp::Ordering::Greater => set(mask, above),
        core::cmp::Ordering::Less => set(mask, below),
        core::cmp::Ordering::Equal => mask,
    }
}

#[cfg(test)]
/// Classify and advance in one call, which is what `observe` used to do.
///
/// Several tests below depend on the latch having moved: a `CHoCH` on the second call is
/// only a `CHoCH` because the first recorded a direction. Splitting the two apart in the
/// library must not silently change what those tests assert, so the sequence they relied
/// on is named once here rather than inlined at seven call sites.
fn step_structure(
    s: &mut Structure,
    close: i64,
    swing_high: Option<Swing>,
    swing_low: Option<Swing>,
) -> Option<Break> {
    let out = s.classify(close, swing_high, swing_low);
    if let Some((_, direction)) = out {
        s.advance(direction);
    }
    out.map(|(b, _)| b)
}

#[cfg(test)]
// `expect` and not `let .. else { unreachable!() }`. An `unreachable!()` expands to a
// panic written IN THIS CRATE, so `cargo llvm-cov` records a region no test can execute
// while the code is correct -- and a region that cannot run is one nobody can be held
// to. `Option::expect` panics inside the standard library, which is not instrumented,
// and refuses just as loudly. Nine fixtures below were the last uncovered lines in this
// file. The workspace denies `expect_used` for library code, where a panic IS a real
// defect; the same allow sits on the test module of `daily`, `core::price`,
// `greeks::bsm` and eleven others.
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// **True range picks the widest of the three classical spans, including the
    /// two that need a previous close.**
    ///
    /// Written against mutation survivors rather than from the formula. A
    /// crate-wide `cargo mutants` run had never reached this file; a file-scoped
    /// one found **14 survivors in `Atr::true_range` alone**, every one of them a
    /// comparison or a sign. The three cases below are the ones a bar can
    /// actually produce:
    ///
    /// * **inside bar** — `high - low` is widest, neither gap term competes;
    /// * **gap up** — the previous close sits BELOW the low, so `high - previous`
    ///   exceeds `high - low` and must win;
    /// * **gap down** — the previous close sits ABOVE the high, so
    ///   `previous - low` exceeds `high - low` and must win.
    ///
    /// The gap cases are what kill `if hc < 0` mutated to `if hc > 0`: under that
    /// mutation a positive `hc` is negated, goes negative, loses every comparison,
    /// and the answer silently falls back to `high - low`.
    #[test]
    fn true_range_is_the_widest_span_including_both_gaps() {
        // No previous close: the span is all there is.
        let mut atr = Atr::new(3);
        assert_eq!(
            atr.true_range(&candle(0, 110, 100, 105)),
            10,
            "with no previous close the range is high - low"
        );

        // Seed a previous close, then an INSIDE bar: neither gap term competes.
        atr.fold(&candle(0, 110, 100, 105));
        assert_eq!(
            atr.true_range(&candle(1, 108, 102, 106)),
            6,
            "inside bar: high - low wins, both gap spans are smaller"
        );

        // GAP UP. previous close 105, next bar entirely above it.
        // hl = 130 - 120 = 10; hc = 130 - 105 = 25; lc = 105 - 120 = -15 -> 15.
        assert_eq!(
            atr.true_range(&candle(2, 130, 120, 125)),
            25,
            "gap up: high - previous is widest and must not be discarded"
        );

        // GAP DOWN. previous close 105, next bar entirely below it.
        // hl = 90 - 80 = 10; hc = 90 - 105 = -15 -> 15; lc = 105 - 80 = 25.
        assert_eq!(
            atr.true_range(&candle(3, 90, 80, 85)),
            25,
            "gap down: previous - low is widest and must not be discarded"
        );
    }

    /// A tie between two spans still answers, and answers the same number.
    ///
    /// The boundary the `>` comparisons sit on: `if hc > widest` keeps `widest`
    /// when they are equal, and a mutation to `>=` swaps which of two identical
    /// numbers is stored. That cannot change the answer — which is why the
    /// equality case is asserted here and NOT claimed as a kill.
    #[test]
    fn a_tie_between_spans_answers_the_shared_value() {
        let mut atr = Atr::new(3);
        atr.fold(&candle(0, 110, 100, 110));
        // hl = 120 - 110 = 10; hc = 120 - 110 = 10; lc = 110 - 110 = 0.
        assert_eq!(
            atr.true_range(&candle(1, 120, 110, 115)),
            10,
            "hl and hc are equal; the answer is that value either way"
        );
    }

    fn candle(ts: i64, high: i64, low: i64, close: i64) -> Candle {
        Candle {
            ts_micros: ts,
            open: close,
            high,
            low,
            close,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    fn tol() -> Tolerance {
        vocab::tolerance::pinned_fib().expect("the pinned fib width is valid")
    }

    /// A series at one price leaves the average exactly on that price — no float drift.
    #[test]
    fn a_flat_series_gives_an_exact_average() {
        let mut e = Ema::new(20);
        for _ in 0..500 {
            e.fold(2_500_000);
        }
        assert_eq!(e.value(), Some(2_500_000), "a constant series has no drift");
    }

    /// The average moves toward a new level and never past it.
    #[test]
    fn the_average_converges_without_overshooting() {
        let mut e = Ema::new(20);
        e.fold(1_000_000);
        for _ in 0..2_000 {
            e.fold(2_000_000);
        }
        let v = e.value().expect("seeded");
        assert!(
            (1_999_990..=2_000_000).contains(&v),
            "converged to {v}, expected to reach 2_000_000 from below without passing it"
        );
    }

    /// Falling prices are not biased upward by truncation.
    ///
    /// `div_euclid` and not `/`: the numerator is negative on every down step, and
    /// truncation toward zero would round each one toward the old average, leaving the
    /// EMA permanently above a falling series.
    #[test]
    fn a_falling_series_is_not_biased_upward() {
        let mut e = Ema::new(20);
        e.fold(2_000_000);
        for _ in 0..2_000 {
            e.fold(1_000_000);
        }
        let v = e.value().expect("seeded");
        assert!(
            (1_000_000..=1_000_010).contains(&v),
            "converged to {v}, expected to reach 1_000_000 from above"
        );
    }

    /// An unseeded average sets nothing rather than guessing.
    #[test]
    fn nothing_is_emitted_before_the_averages_exist() {
        let t = TrendState::default();
        let mask = t.bits(2_500_000, tol());
        for index in [0_u16, 1, 2, 3, 4, 5, 64, 65, 72, 73] {
            assert!(
                !mask.get(u32::from(index)),
                "position {index} spoke too early"
            );
        }
    }

    /// A close exactly on the average sets neither side.
    ///
    /// The same degenerate-case discipline as position 227 and the tri-star: `else`
    /// must not collect equality, or "above" becomes "not below".
    #[test]
    fn a_close_exactly_on_the_average_sets_neither_side() {
        let mut t = TrendState::default();
        for i in 0..300_i64 {
            t.step(
                &candle(i * 60_000_000, 2_500_000, 2_500_000, 2_500_000),
                tol(),
            )
            .expect("sane candle");
        }
        let mask = t.bits(2_500_000, tol());
        assert!(
            !mask.get(0) && !mask.get(1),
            "close == ema20 sets neither 0 nor 1"
        );
        assert!(
            !mask.get(2) && !mask.get(3),
            "close == ema200 sets neither 2 nor 3"
        );
        assert!(
            !mask.get(4) && !mask.get(5),
            "ema20 == ema200 sets neither 4 nor 5"
        );
    }

    /// A rising series puts the fast average above the slow one.
    #[test]
    fn a_rising_series_puts_the_fast_average_above_the_slow() {
        let mut t = TrendState::default();
        for i in 0..400_i64 {
            let price = 2_000_000 + i * 1_000;
            t.step(
                &candle(i * 60_000_000, price + 500, price - 500, price),
                tol(),
            )
            .expect("sane candle");
        }
        let mask = t.bits(2_400_000, tol());
        assert!(mask.get(4), "ema20 must lead ema200 upward");
        assert!(!mask.get(5));
    }

    /// True range takes the widest of the three spans, including the overnight gap.
    #[test]
    fn true_range_includes_the_gap_from_the_previous_close() {
        let mut a = Atr::new(10);
        a.fold(&candle(0, 2_500_000, 2_499_000, 2_500_000));
        // Next candle opens far above: high-low is small but high-prev_close is large.
        a.fold(&candle(60_000_000, 2_520_000, 2_519_000, 2_520_000));
        let v = a.value().expect("seeded");
        assert!(
            v > 1_000,
            "ATR {v} ignored the 20,000-paisa gap and used only the 1,000 high-low span"
        );
    }

    /// The `SuperTrend` stop ratchets while the trend holds, and the reference is
    /// DROPPED when it does not.
    ///
    /// # Why the fixture now breaks and recovers
    ///
    /// It used to rise 2,000 paisa a candle for sixty candles and stop there, so
    /// `s.trend()` was `Up` on every one of them and the `else` arm below never ran once.
    /// That arm is not bookkeeping. A short stop sits ABOVE price and a long stop below
    /// it, so carrying `previous` across a flip compares two levels that mean opposite
    /// things -- and the first candle back on the long side would then be measured
    /// against a stop the market has already taken out, which is a comparison that can
    /// only pass by accident.
    ///
    /// So the fixture does the round trip: sixty candles up, twenty down at 20,000 a
    /// candle (far enough through the trailing stop to flip it), then back up at the same
    /// speed and through the short stop. Deleting `previous = None` makes the last
    /// assertion below fail, because the long stop resumes far under the level it held
    /// before the break and the ratchet would report that as a retreat.
    ///
    /// The DOWN-side ratchet is deliberately not re-asserted here --
    /// `the_stop_on_both_sides::a_short_stop_tightens_holds_and_then_flips` owns it, and
    /// two tests failing for one defect names the defect no better than one.
    #[test]
    fn the_stop_ratchets_and_does_not_retreat() {
        let mut s = SuperTrend::new(TrendThresholds::CLASSICAL);
        let mut previous: Option<i64> = None;
        // The last long stop before the market broke it, and the first long stop after
        // it came back. Both are long stops, and the second must be BELOW the first:
        // the ratchet is not allowed to survive the flip that invalidated it.
        let mut before_the_break: Option<i64> = None;
        let mut after_the_break: Option<i64> = None;
        let mut went_short = false;

        for i in 0..100_i64 {
            let price = match i {
                0..=59 => 2_000_000 + i * 2_000,
                60..=79 => 2_118_000 - (i - 59) * 20_000,
                _ => 1_718_000 + (i - 79) * 20_000,
            };
            s.fold(&candle(i * 60_000_000, price + 1_000, price - 1_000, price));
            if s.trend() == Trend::Up {
                if let (Some(now), Some(before)) = (s.stop(), previous) {
                    assert!(
                        now >= before,
                        "the stop fell from {before} to {now} in an up trend"
                    );
                }
                previous = s.stop();
                if went_short && after_the_break.is_none() {
                    after_the_break = previous;
                }
            } else {
                if !went_short {
                    before_the_break = previous;
                }
                went_short = true;
                previous = None;
            }
        }

        assert!(
            went_short,
            "the fixture never closed through the trailing stop, so the arm that drops \
             the reference never ran and the ratchet was tested in one direction only"
        );
        let broke = before_the_break.expect("a long stop was established before the break");
        let resumed = after_the_break.expect("the market recovered back through the short stop");
        assert!(
            resumed < broke,
            "the long stop resumed at {resumed}, at or above the {broke} it held before \
             the market took it out -- the ratchet carried across a flip, so a level the \
             market has already cleared is still being defended"
        );
    }

    /// A five-bar fractal peak is confirmed, and only two bars later.
    ///
    /// This is the look-ahead property made concrete: the swing at index 2 cannot be
    /// known until index 4 has arrived, and the detector does not claim it sooner.
    #[test]
    fn a_swing_high_is_confirmed_two_bars_late() {
        let mut d = SwingDetector::new();
        let highs = [2_500_000_i64, 2_501_000, 2_505_000, 2_502_000, 2_500_500];
        for (i, high) in (0_i64..).zip(highs.iter()) {
            d.fold(&candle(i * 60_000_000, *high, high - 2_000, high - 1_000));
            if i < 4 {
                assert_eq!(d.swing_high(), None, "bar {i} cannot know the peak yet");
            }
        }
        let s = d.swing_high().expect("the fifth bar completes the window");
        assert_eq!(s.price, 2_505_000, "the middle bar's high is the swing");
        assert!(s.window_span > 0, "the band needs a range to scale against");
    }

    /// A plateau is not a swing: two equal highs cannot both be the level.
    #[test]
    fn a_plateau_is_not_a_swing() {
        let mut d = SwingDetector::new();
        let highs = [2_500_000_i64, 2_505_000, 2_505_000, 2_502_000, 2_500_500];
        for (i, high) in (0_i64..).zip(highs.iter()) {
            d.fold(&candle(i * 60_000_000, *high, high - 2_000, high - 1_000));
        }
        assert_eq!(
            d.swing_high(),
            None,
            "an equal neighbour disqualifies the peak"
        );
    }

    /// The first break is a `BoS`, never a `CHoCH`.
    #[test]
    fn the_first_break_is_never_a_change_of_character() {
        let mut s = Structure::new();
        let high = Some(Swing {
            price: 2_500_000,
            window_span: 10_000,
            confirmed_at: 10,
        });
        assert_eq!(
            step_structure(&mut s, 2_501_000, high, None),
            Some(Break::BosBullish)
        );
    }

    /// A break the other way, after one, is a `CHoCH`.
    #[test]
    fn a_break_against_the_previous_direction_is_a_change_of_character() {
        let mut s = Structure::new();
        let high = Some(Swing {
            price: 2_500_000,
            window_span: 10_000,
            confirmed_at: 10,
        });
        let low = Some(Swing {
            price: 2_400_000,
            window_span: 10_000,
            confirmed_at: 20,
        });
        assert_eq!(
            step_structure(&mut s, 2_501_000, high, low),
            Some(Break::BosBullish)
        );
        assert_eq!(
            step_structure(&mut s, 2_399_000, high, low),
            Some(Break::ChochBearish)
        );
        // And back again.
        assert_eq!(
            step_structure(&mut s, 2_501_000, high, low),
            Some(Break::ChochBullish)
        );
        // Continuing in the same direction is a BoS, not another CHoCH.
        assert_eq!(
            step_structure(&mut s, 2_502_000, high, low),
            Some(Break::BosBullish)
        );
    }

    /// Breaking neither side reports nothing.
    #[test]
    fn a_candle_inside_the_structure_breaks_nothing() {
        let mut s = Structure::new();
        let high = Some(Swing {
            price: 2_500_000,
            window_span: 10_000,
            confirmed_at: 10,
        });
        let low = Some(Swing {
            price: 2_400_000,
            window_span: 10_000,
            confirmed_at: 20,
        });
        assert_eq!(step_structure(&mut s, 2_450_000, high, low), None);
    }

    /// Every position is live and of the kind the table declares.
    #[test]
    fn the_positions_match_the_vocabulary() {
        for index in TrendState::positions() {
            // `expect` rather than `unwrap_or_else(|| panic!(..))`: the closure is a
            // panic this crate owns, which is the same uncoverable region in a new
            // costume, and the workspace denies `panic` outright. The index is in the
            // assertion below, so the message loses nothing by dropping it.
            let def = vocab::table::definition(index).expect("the position is in the table");
            let expected = if index == 72 || index == 73 {
                vocab::Kind::Near
            } else {
                vocab::Kind::Plain
            };
            assert_eq!(def.kind, expected, "position {index} has the wrong kind");
            assert!(
                matches!(def.status, vocab::BitStatus::Live),
                "position {index} is not live"
            );
        }
    }

    /// A corrupt record is refused and changes no state.
    #[test]
    fn a_corrupt_record_is_refused() {
        let mut t = TrendState::default();
        let inverted = Candle {
            ts_micros: 0,
            open: 100,
            high: 90,
            low: 110,
            close: 100,
            volume: 0,
            open_interest: i64::MIN,
        };
        assert_eq!(t.step(&inverted, tol()), Err(crate::Corrupt::HighBelowLow));
        assert_eq!(t.fast.value(), None, "a refused candle folded nothing");
    }

    /// Two runs over one series agree exactly — §3 rule 5.
    #[test]
    fn two_runs_agree_byte_for_byte() {
        let series: Vec<Candle> = (0..300_i64)
            .map(|i| {
                let p = 2_500_000 + ((i * 137) % 811 - 405) * 7;
                candle(i * 60_000_000, p + 300, p - 300, p)
            })
            .collect();
        let run = || {
            let mut t = TrendState::default();
            series
                .iter()
                .filter_map(|c| t.step(c, tol()).ok())
                .collect::<Vec<_>>()
        };
        let first = run();
        for _ in 0..3 {
            assert_eq!(run(), first, "a rerun disagreed");
        }
    }

    /// `bits` is a function of the bar, so asking twice gives the same answer.
    ///
    /// It took `&mut self` and advanced the `Structure` latch from inside the emit.
    /// Measured on a rising-then-falling series: one bar gave `[1, 3, 4, 59, 65]` and the
    /// identical call on the same close then gave `[1, 3, 4, 57, 65]` — 59 is
    /// `choch_bearish` and 57 is `bos_bearish`, so the second read reclassified a change
    /// of character as a plain break because the first read had already moved the latch.
    ///
    /// Anything that looked twice — a debug print, a UI, a second consumer of the same
    /// evaluator — permanently changed what the sweep later got. `&self` makes that
    /// unconstructible rather than merely discouraged.
    #[test]
    fn bits_is_idempotent_across_a_break_and_a_change_of_character() {
        let tolerance = tol();
        let mut t = TrendState::new(TrendThresholds::CLASSICAL);
        let path: [i64; 24] = [
            2_400_000, 2_410_000, 2_430_000, 2_460_000, 2_500_000, 2_470_000, 2_440_000, 2_450_000,
            2_480_000, 2_520_000, 2_560_000, 2_530_000, 2_490_000, 2_500_000, 2_540_000, 2_580_000,
            2_550_000, 2_500_000, 2_450_000, 2_400_000, 2_380_000, 2_420_000, 2_460_000, 2_390_000,
        ];
        for (m, close) in path.iter().enumerate() {
            let minute = i64::try_from(m).expect("24 fits an i64");
            let bar = candle(minute * 60_000_000, close + 600, close - 600, *close);
            let first = t.bits(*close, tolerance);
            let second = t.bits(*close, tolerance);
            let third = t.bits(*close, tolerance);
            assert_eq!(
                first.words(),
                second.words(),
                "bar {m}: reading the bits twice gave two different masks"
            );
            assert_eq!(
                first.words(),
                third.words(),
                "bar {m}: and a third read differed again"
            );
            t.step(&bar, tolerance).expect("a sane candle");
        }
    }

    /// The latch advances once per candle, not once per read.
    ///
    /// The companion to the test above: `bits` being pure is only worth having if
    /// **A close exactly ON a swing level is a touch, not a break.**
    ///
    /// `close > s.price` and `close < s.price` are strict, and both mutate to
    /// their non-strict forms. Under either mutation, price merely REACHING the
    /// prior swing high reports a break of structure — and in a range that
    /// retests the same level repeatedly, every retest becomes a signal. The
    /// distinction between touching a level and breaking it is the whole content
    /// of the word "break".
    #[test]
    fn a_close_exactly_on_a_swing_level_is_not_a_break() {
        let s = Structure::new();
        let high = Swing {
            price: 2_500_000,
            confirmed_at: 10,
            window_span: 1_000,
        };
        let low = Swing {
            price: 2_400_000,
            confirmed_at: 5,
            window_span: 1_000,
        };

        assert_eq!(
            s.classify(2_500_000, Some(high), Some(low)),
            None,
            "a close exactly AT the swing high is a touch, not a break up"
        );
        assert_eq!(
            s.classify(2_400_000, Some(high), Some(low)),
            None,
            "a close exactly AT the swing low is a touch, not a break down"
        );

        // One paisa past either level IS a break, so the strictness is a
        // boundary and not a refusal to answer.
        assert!(
            s.classify(2_500_001, Some(high), Some(low)).is_some(),
            "one paisa above the swing high breaks it"
        );
        assert!(
            s.classify(2_399_999, Some(high), Some(low)).is_some(),
            "one paisa below the swing low breaks it"
        );
    }

    /// something still moves the structure on. Pinned through the public accessor rather
    /// than the private field.
    #[test]
    fn the_structure_latch_advances_exactly_once_per_candle() {
        let mut s = Structure::new();
        let high = Swing {
            price: 2_500_000,
            confirmed_at: 10,
            window_span: 1_000,
        };
        let low = Swing {
            price: 2_400_000,
            confirmed_at: 5,
            window_span: 1_000,
        };
        assert_eq!(
            s.in_force(),
            None,
            "nothing is in force before the first break"
        );

        // Classifying does not advance, however often it is asked.
        assert!(s.classify(2_501_000, Some(high), Some(low)).is_some());
        assert!(s.classify(2_501_000, Some(high), Some(low)).is_some());
        assert_eq!(
            s.in_force(),
            None,
            "classify moved the latch, so it is not the pure half it claims to be"
        );

        let (broke, direction) = s
            .classify(2_501_000, Some(high), Some(low))
            .expect("a close above the swing high is a break");
        assert_eq!(
            broke,
            Break::BosBullish,
            "the first break is a BoS, never a CHoCH"
        );
        s.advance(direction);
        assert_eq!(s.in_force(), Some(Trend::Up));

        // And the same shape the other way is a CHoCH every time it is asked.
        for _ in 0..3 {
            let (b, _) = s
                .classify(2_399_000, Some(high), Some(low))
                .expect("a close below the swing low is a break");
            assert_eq!(
                b,
                Break::ChochBearish,
                "a down break against an up structure is a CHoCH on every read"
            );
        }
    }

    /// A change of character reaches the mask, which is what proves the latch advances.
    ///
    /// # Why this test exists, and what it caught about its own siblings
    ///
    /// `bits_is_idempotent_..` and `the_structure_latch_advances_..` both pass if `step`
    /// stops advancing the latch altogether — measured, by deleting the advance and
    /// watching every test stay green. With `Structure::last` permanently `None`, the
    /// classification arm `(_, Trend::Up) => BosBullish` fires for ever and **positions
    /// 58 and 59 become unreachable**: every change of character is reported as a plain
    /// break, and a sweep looking for a reversal finds only continuations.
    ///
    /// So this drives the public `step` over a rise, a fall through the confirmed swing
    /// high, a rally and a second fall, and requires a `CHoCH` bit to appear. It is the only
    /// test in this file that fails when the advance is removed.
    #[test]
    fn a_change_of_character_reaches_the_mask() {
        let tolerance = tol();
        let mut t = TrendState::new(TrendThresholds::CLASSICAL);
        let mut seen_bos = false;
        let mut seen_choch = false;
        for m in 0..60_i64 {
            // Four legs: up, down through the swing high, up again, down again.
            let within = m % 15;
            let close = match m / 15 {
                0 => 2_400_000 + within * 8_000,
                1 => 2_520_000 - within * 9_000,
                2 => 2_385_000 + within * 10_000,
                _ => 2_535_000 - within * 11_000,
            };
            let bar = candle(m * 60_000_000, close + 700, close - 700, close);
            let mask = t.step(&bar, tolerance).expect("a sane candle");
            if mask.get(56) || mask.get(57) {
                seen_bos = true;
            }
            if mask.get(58) || mask.get(59) {
                seen_choch = true;
            }
        }
        assert!(
            seen_bos,
            "no break of structure at all, so the series proves nothing"
        );
        assert!(
            seen_choch,
            "a break in one direction followed by a break in the other never produced a \
             CHoCH, so `Structure::last` is never advancing — positions 58 and 59 are \
             unreachable and every reversal is being reported as a continuation"
        );
    }

    /// A swing window whose span leaves `i64` publishes no swing at all.
    ///
    /// The fourth of the four saturating band bases, and the one held back from the agent
    /// that fixed the other three because it lives in this file beside the warm-up gate.
    ///
    /// The window's extremes come from five different bars, so two of them at opposite ends
    /// of the type give a true span of up to 1.84e19. `saturating_sub` returned
    /// `i64::MAX` — a band HALF the real width — and a close inside the missing half then
    /// set no `near_swing` bit while nothing refused. A silent false on a bar the band
    /// actually covers is the §4 fallback that hides a failure, and it is worse than an
    /// absent swing because the sweep cannot tell the two apart.
    #[test]
    fn a_swing_window_whose_span_leaves_the_type_publishes_nothing() {
        let mut d = SwingDetector::new();
        // Five candles. The middle one is the peak, and two of the others sit at opposite
        // ends of `i64`, so the window's own span cannot be represented.
        let far_low = i64::MIN / 2 - 1;
        let far_high = i64::MAX / 2 + 1;
        let prices = [far_low, far_low + 1, far_high, far_low + 2, far_low + 3];
        for (m, p) in prices.iter().enumerate() {
            let minute = i64::try_from(m).expect("five fits an i64");
            d.fold(&candle(minute * 60_000_000, *p, *p, *p));
        }
        assert!(
            far_high.checked_sub(far_low).is_none(),
            "the fixture no longer overflows, so it does not test the span guard"
        );
        assert_eq!(
            d.swing_high(),
            None,
            "a swing was published from a window whose span cannot be represented, so its \
             `window_span` is a saturated number and every band scaled against it is up to \
             twice too narrow"
        );
        assert_eq!(d.swing_low(), None, "and neither latch may be published");
    }
}

#[cfg(test)]
mod double_break {
    use super::*;

    /// A recent swing low ABOVE an older swing high is reachable, and a close between
    /// them must still report a break.
    ///
    /// This is the case the first version of `observe` declared impossible. In a rising
    /// market a low confirmed five candles ago sits above a high confirmed twenty ago,
    /// and every close between the two broke both — so `broke_up == broke_down` and the
    /// function returned `None`, suppressing every real structure break for as long as
    /// the stale latch survived.
    #[test]
    fn a_recent_low_above_an_older_high_still_reports_a_break() {
        let older_high = Some(Swing {
            price: 2_500_000,
            window_span: 10_000,
            confirmed_at: 5,
        });
        let newer_low = Some(Swing {
            price: 2_510_000,
            window_span: 10_000,
            confirmed_at: 40,
        });
        // 2_505_000 is above the high AND below the low: both break.
        let mut s = Structure::new();
        assert_eq!(
            step_structure(&mut s, 2_505_000, older_high, newer_low),
            Some(Break::BosBearish),
            "the more recently confirmed swing is the structure in force"
        );
    }

    /// And with the ages reversed, the same close reports the other direction.
    ///
    /// Without this half the fix could be "always pick down" and stay green.
    #[test]
    fn reversing_which_swing_is_newer_reverses_the_break() {
        let newer_high = Some(Swing {
            price: 2_500_000,
            window_span: 10_000,
            confirmed_at: 40,
        });
        let older_low = Some(Swing {
            price: 2_510_000,
            window_span: 10_000,
            confirmed_at: 5,
        });
        let mut s = Structure::new();
        assert_eq!(
            step_structure(&mut s, 2_505_000, newer_high, older_low),
            Some(Break::BosBullish)
        );
    }

    /// Confirmed on the same candle: neither supersedes the other, so neither is the
    /// structure and there is nothing to report.
    #[test]
    fn two_swings_confirmed_together_report_nothing() {
        let high = Some(Swing {
            price: 2_500_000,
            window_span: 10_000,
            confirmed_at: 7,
        });
        let low = Some(Swing {
            price: 2_510_000,
            window_span: 10_000,
            confirmed_at: 7,
        });
        let mut s = Structure::new();
        assert_eq!(step_structure(&mut s, 2_505_000, high, low), None);
    }

    /// The detector really does stamp its confirmations, and the stamps increase.
    #[test]
    fn the_detector_stamps_each_confirmation_with_a_rising_sequence() {
        let mut d = SwingDetector::new();
        let mut first: Option<u64> = None;
        for (i, high) in (0_i64..).zip([
            2_500_000_i64,
            2_501_000,
            2_505_000,
            2_502_000,
            2_500_500,
            2_499_000,
            2_498_000,
            2_490_000,
            2_495_000,
            2_496_000,
        ]) {
            d.fold(&Candle {
                ts_micros: i * 60_000_000,
                open: high - 1_000,
                high,
                low: high - 2_000,
                close: high - 1_000,
                volume: 0,
                open_interest: i64::MIN,
            });
            if let Some(s) = d.swing_high() {
                if let Some(seen) = first {
                    assert!(
                        s.confirmed_at >= seen,
                        "a stamp went backwards: {} after {seen}",
                        s.confirmed_at
                    );
                } else {
                    first = Some(s.confirmed_at);
                    assert!(s.confirmed_at > 0, "a confirmation must carry a stamp");
                }
            }
        }
        assert!(first.is_some(), "a swing high was confirmed at some point");
    }
}

#[cfg(test)]
// A sibling of `mod tests` rather than a child, so it needs the allow in its own right;
// see the note there for why `expect` and not `unreachable!`.
#[allow(clippy::expect_used)]
mod thresholds_are_read {
    use super::*;

    /// Every field of [`TrendThresholds`] is actually consulted by a computation.
    ///
    /// This test exists because two of the six were not. `fractal` was stored on the
    /// detector and never read — the window was the const `RING` at every use — and
    /// `swing_band` was never read at all. Both were declared, documented, and inert: a
    /// caller setting either got silence, in the public API of a crate another
    /// repository consumes.
    ///
    /// The check is behavioural, not a grep: change each field and require the emitted
    /// bits to change. A field whose value cannot alter any output is not a threshold.
    #[test]
    fn changing_any_threshold_changes_what_is_emitted() {
        fn run(t: TrendThresholds) -> Vec<ConditionMask> {
            let mut s = TrendState::new(t);
            let tolerance = vocab::tolerance::pinned_fib().expect("pinned");
            (0..400_i64)
                .map(|i| {
                    // A series with a real trend and real reversals, so every family has
                    // something to say.
                    let wave = ((i * 37) % 211) - 105;
                    let p = 2_500_000 + i * 40 + wave * 9;
                    let candle = Candle {
                        ts_micros: i * 60_000_000,
                        open: p,
                        high: p + 600,
                        low: p - 600,
                        close: p + wave.clamp(-500, 500),
                        volume: 0,
                        open_interest: i64::MIN,
                    };
                    s.step(&candle, tolerance).unwrap_or(ConditionMask::ZERO)
                })
                .collect()
        }

        let base = TrendThresholds::CLASSICAL;
        let reference = run(base);

        // Three fields are observable in the emitted bits.
        let variants: [(&str, TrendThresholds); 3] = [
            (
                "ema_fast",
                TrendThresholds {
                    ema_fast: 5,
                    ..base
                },
            ),
            (
                "ema_slow",
                TrendThresholds {
                    ema_slow: 50,
                    ..base
                },
            ),
            (
                "supertrend_mult",
                TrendThresholds {
                    supertrend_mult: 500,
                    ..base
                },
            ),
        ];
        for (name, variant) in variants {
            assert_ne!(
                run(variant),
                reference,
                "changing `{name}` changed nothing, so it is not a threshold — it is a \
                 field nobody reads, which is what `fractal` and `swing_band` were"
            );
        }

        // `atr_period` is the fourth, and it needs its own observable rather than a
        // weaker version of this one. It sets the trailing stop's LEVEL, and positions
        // 64/65 encode only which SIDE of that level price is on — which on a trending
        // series is the same side for a fast and a slow ATR. Comparing masks therefore
        // proves nothing about it, and a test that cannot fail is worse than none.
        let mut fast = Atr::new(2);
        let mut slow = Atr::new(50);
        for i in 0..200_i64 {
            let p = 2_500_000 + ((i * 71) % 401) * 13;
            let candle = Candle {
                ts_micros: i * 60_000_000,
                open: p,
                high: p + 900,
                low: p - 900,
                close: p,
                volume: 0,
                open_interest: i64::MIN,
            };
            fast.fold(&candle);
            slow.fold(&candle);
        }
        assert_ne!(
            fast.value(),
            slow.value(),
            "`atr_period` changed nothing, so it is not a threshold either"
        );
    }

    /// The window is odd and its middle is derived, not declared twice.
    #[test]
    fn the_fractal_is_derived_from_the_ring() {
        assert_eq!(
            FRACTAL, 2,
            "a five-candle window has two neighbours each side"
        );
        assert_eq!(RING, 2 * FRACTAL + 1);
        assert_eq!(RING % 2, 1, "an even window has no middle candle to test");
    }

    /// A swing is still published exactly `FRACTAL` candles after it happened.
    ///
    /// The delay is what makes the level sound rather than late: confirming at the
    /// candle itself would require reading the ones after it.
    #[test]
    fn a_swing_is_still_confirmed_exactly_fractal_candles_late() {
        let mut d = SwingDetector::new();
        let highs = [2_500_000_i64, 2_501_000, 2_505_000, 2_502_000, 2_500_500];
        for (i, high) in (0_i64..).zip(highs.iter()) {
            d.fold(&Candle {
                ts_micros: i * 60_000_000,
                open: high - 1_000,
                high: *high,
                low: high - 2_000,
                close: high - 1_000,
                volume: 0,
                open_interest: i64::MIN,
            });
            let expected_by = i64::try_from(RING).unwrap_or(i64::MAX) - 1;
            if i < expected_by {
                assert_eq!(d.swing_high(), None, "candle {i} cannot know the peak yet");
            }
        }
        assert!(
            d.swing_high().is_some(),
            "the window completes at candle RING-1 and the swing is published then"
        );
    }
}

#[cfg(test)]
mod defaults_and_degenerate_periods {
    use super::*;

    fn bar(ts: i64, high: i64, low: i64, close: i64) -> Candle {
        Candle {
            ts_micros: ts,
            open: close,
            high,
            low,
            close,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    /// `Default` is the classical set, and a state reports the set it computes with.
    ///
    /// Two `Default` impls reach these numbers — [`TrendThresholds::default`] and
    /// [`TrendState::default`] — and each names the constant separately. If one drifted,
    /// a caller recording [`TrendState::thresholds`] would file a run under numbers the
    /// sweep did not use, which is §3 rule 3's identity broken quietly rather than
    /// loudly.
    #[test]
    fn the_default_thresholds_are_the_classical_set() {
        assert_eq!(
            TrendThresholds::default(),
            TrendThresholds::CLASSICAL,
            "`Default` must BE `CLASSICAL`, not a second opinion about it"
        );
        assert_eq!(
            TrendState::default().thresholds(),
            TrendThresholds::CLASSICAL,
            "a defaulted state must report the thresholds it actually computes with"
        );
    }

    /// A defaulted detector is an empty one, not a half-filled window.
    #[test]
    fn a_defaulted_detector_has_confirmed_nothing() {
        let d = SwingDetector::default();
        assert_eq!(
            d,
            SwingDetector::new(),
            "`Default` and `new` must be the same detector, or `fresh` means two things"
        );
        assert_eq!(
            d.swing_high(),
            None,
            "an empty window has confirmed no high"
        );
        assert_eq!(d.swing_low(), None, "an empty window has confirmed no low");
    }

    /// A defaulted structure carries no direction, so its first break is a `BoS`.
    ///
    /// A `Default` that started at `Some(_)` would make the first break of every run a
    /// `CHoCH` — an artefact of where the run started rather than a fact about price,
    /// which is exactly what [`Structure::observe`]'s first-break rule exists to refuse.
    #[test]
    fn a_defaulted_structure_has_no_prior_direction() {
        let mut s = Structure::default();
        assert_eq!(
            s,
            Structure::new(),
            "`Default` and `new` must be the same structure"
        );
        let high = Some(Swing {
            price: 2_500_000,
            window_span: 10_000,
            confirmed_at: 3,
        });
        assert_eq!(
            step_structure(&mut s, 2_501_000, high, None),
            Some(Break::BosBullish),
            "the first break out of a defaulted structure is a BoS, never a CHoCH"
        );
    }

    /// A period that cannot divide holds the seed instead of dividing by zero.
    ///
    /// `ema_fast` and `ema_slow` are public `i128` fields with no floor, so
    /// [`TrendState::new`] hands [`Ema::new`] whatever a caller writes. At `period == -1`
    /// the denominator `n + 1` is exactly zero and the update divides by it: without the
    /// guard this panics instead of reporting, and a panic in a sweep is 200,000 candles
    /// of work thrown away for one bad threshold.
    #[test]
    fn an_average_whose_period_cannot_divide_holds_its_seed() {
        let mut e = Ema::new(-1);
        e.fold(2_500_000);
        e.fold(9_000_000);
        e.fold(1_000_000);
        assert_eq!(
            e.value(),
            Some(2_500_000),
            "a period that cannot divide must hold the seed: not panic, and not guess a step"
        );
    }

    /// The same for the ATR, whose divisor is the period itself rather than `n + 1`.
    #[test]
    fn an_atr_whose_period_cannot_divide_holds_its_seed() {
        let mut a = Atr::new(0);
        // Seeds at the first candle's high-low span, 1,000 paisa.
        a.fold(&bar(0, 2_501_000, 2_500_000, 2_500_500));
        // A 200,000-wide candle would move a working ATR a very long way.
        a.fold(&bar(60_000_000, 2_600_000, 2_400_000, 2_500_000));
        assert_eq!(
            a.value(),
            Some(1_000),
            "a zero period must divide nothing, and must smooth nothing either"
        );
    }

    /// Nothing reports a range or a stop before the first candle.
    ///
    /// A zero would be worse than an absence here: a zero ATR is a zero `SuperTrend`
    /// band, which puts the stop exactly on the midpoint and flips the trend on the
    /// first candle that closes on the other side of it.
    #[test]
    fn an_unseeded_range_and_stop_report_nothing_rather_than_zero() {
        assert_eq!(
            Atr::new(10).value(),
            None,
            "an unfed ATR has no range to report, and zero is not `no range`"
        );
        let fresh = SuperTrend::new(TrendThresholds::CLASSICAL);
        assert_eq!(
            fresh.stop(),
            None,
            "there is no trailing stop before the first candle"
        );
        assert_eq!(
            fresh.trend(),
            Trend::Up,
            "the latch starts on one side by construction; `stop() == None` is what keeps \
             that arbitrary side out of positions 64 and 65"
        );
    }
}

#[cfg(test)]
mod the_stop_on_both_sides {
    use super::*;

    fn bar(ts: i64, high: i64, low: i64, close: i64) -> Candle {
        Candle {
            ts_micros: ts,
            open: close,
            high,
            low,
            close,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    /// The seed takes the side the first candle implies — and the other side proves it.
    ///
    /// Half of this had never run. Seeding always long would make the first 64/65 bit an
    /// artefact of the starting trend rather than a fact about the candle, and a test
    /// that only ever seeds long cannot tell the two apart.
    #[test]
    fn the_first_candle_decides_which_side_the_stop_seeds_on() {
        // mid is 2_500_000 and the first ATR is the 20,000 high-low span, so the band is
        // three times that: 60,000.
        let mut short = SuperTrend::new(TrendThresholds::CLASSICAL);
        short.fold(&bar(0, 2_510_000, 2_490_000, 2_491_000));
        assert_eq!(
            short.trend(),
            Trend::Down,
            "a close below the midpoint seeds short"
        );
        assert_eq!(
            short.stop(),
            Some(2_560_000),
            "a short stop sits at mid + band, above price"
        );

        let mut long = SuperTrend::new(TrendThresholds::CLASSICAL);
        long.fold(&bar(0, 2_510_000, 2_490_000, 2_509_000));
        assert_eq!(
            long.trend(),
            Trend::Up,
            "a close above the midpoint seeds long"
        );
        assert_eq!(
            long.stop(),
            Some(2_440_000),
            "a long stop sits at mid - band, below price"
        );
    }

    /// A short stop tightens downward, holds when the band widens, and flips on a close
    /// through it.
    ///
    /// The mirror of `tests::the_stop_ratchets_and_does_not_retreat`, and not a copy of
    /// it: that test seeds long and stays long, so the whole `Trend::Down` arm — the
    /// tightening, the hold, and the flip back up — had never run. A stop that loosened
    /// in a down trend would follow price back up and never be taken out, which is the
    /// ratchet's whole purpose defeated.
    #[test]
    fn a_short_stop_tightens_holds_and_then_flips() {
        let mut s = SuperTrend::new(TrendThresholds::CLASSICAL);
        s.fold(&bar(0, 2_510_000, 2_490_000, 2_491_000));
        let mut previous = s.stop();
        let mut tightened = 0_u32;
        for i in 1..20_i64 {
            let p = 2_490_000 - i * 2_000;
            s.fold(&bar(i * 60_000_000, p + 1_000, p - 1_000, p));
            assert_eq!(
                s.trend(),
                Trend::Down,
                "candle {i} closed far below the stop and must still be short"
            );
            let now = s.stop();
            assert!(
                matches!((previous, now), (Some(before), Some(after)) if after <= before),
                "the short stop loosened or vanished at candle {i}: {previous:?} then {now:?}"
            );
            if matches!((previous, now), (Some(before), Some(after)) if after < before) {
                tightened += 1;
            }
            previous = now;
        }
        assert!(
            tightened > 0,
            "a series falling 2,000 paisa a candle must tighten the short stop at least \
             once, or the ratchet is not moving at all"
        );

        // A candle whose range explodes puts mid + band far ABOVE the stop. The ratchet
        // must hold it, not follow the band out: the market has not taken it out.
        let held = s.stop();
        let p = 2_490_000 - 20 * 2_000;
        s.fold(&bar(20 * 60_000_000, p + 200_000, p - 1_000, p - 500));
        assert_eq!(
            s.trend(),
            Trend::Down,
            "a wide candle that still closed below the stop is not a flip"
        );
        assert_eq!(
            s.stop(),
            held,
            "a widening band must never loosen a stop price has not reached"
        );

        // And a close through it flips to long, with the stop on the other side of price.
        let close = p + 300_000;
        s.fold(&bar(21 * 60_000_000, p + 400_000, p, close));
        assert_eq!(
            s.trend(),
            Trend::Up,
            "a close above the short stop is the flip"
        );
        let flipped = s.stop();
        assert!(
            flipped.is_some_and(|stop| stop < close),
            "after the flip the stop must sit below price at mid - band, not above it: {flipped:?}"
        );
    }

    /// A stop that will not fit `i64` is refused, and the last good one stands.
    ///
    /// `supertrend_mult` is a public `i128` with no ceiling, so the band is only as
    /// bounded as its caller. The multiplier below is not one any operator would set: it
    /// is the coarse thing that puts `mid + band` past the top of `i64` while every price
    /// stays ordinary paisa, which is the only way to reach the conversion's failing arm.
    /// A wrapped stop would be a plausible wrong price, which §4 bans outright.
    #[test]
    fn a_stop_that_will_not_fit_i64_is_refused_and_the_last_good_one_stands() {
        let mut s = SuperTrend::new(TrendThresholds {
            supertrend_mult: 1_000_000_000_000_000_000_000,
            ..TrendThresholds::CLASSICAL
        });
        // A limit-locked minute: zero range, so the first ATR is zero and the band with
        // it. It is the only candle here whose stop can be represented at all.
        s.fold(&bar(0, 2_500_000, 2_500_000, 2_500_000));
        assert_eq!(
            s.trend(),
            Trend::Up,
            "a close on the midpoint seeds long — `side` gives equality to neither bit, \
             but the seed has to pick a side"
        );
        assert_eq!(
            s.stop(),
            Some(2_500_000),
            "a zero band puts the seeded stop exactly on the midpoint"
        );
        // A 2,000-wide candle takes the ATR to 200, so the band is 2 x 10^20 — past the
        // top of `i64` — and the close is under the stop, which is a flip to short.
        s.fold(&bar(60_000_000, 2_501_000, 2_499_000, 2_499_500));
        assert_eq!(s.trend(), Trend::Down, "the flip itself is still recorded");
        assert_eq!(
            s.stop(),
            Some(2_500_000),
            "an unrepresentable band must leave the last good stop in place, never wrap it"
        );
    }

    /// A true range that will not fit `i64` establishes no stop at all.
    ///
    /// `high - low` is 2^63 here, one past the top of the type. [`Candle::check`] catches
    /// it — this test asserts that it does — so [`TrendState::step`] never reaches the
    /// guard. The guard exists because [`SuperTrend::fold`] is public and takes an
    /// unchecked candle, and this crate is consumed by another repository.
    #[test]
    fn a_range_that_will_not_fit_i64_establishes_no_stop() {
        let mut s = SuperTrend::new(TrendThresholds::CLASSICAL);
        let corrupt = bar(0, i64::MAX, -1, 0);
        assert_eq!(
            corrupt.check(),
            Err(crate::Corrupt::RangeOverflows),
            "the record is corrupt and the type says so; `fold` sees it only from a \
             caller that skipped the check"
        );
        s.fold(&corrupt);
        assert_eq!(
            s.stop(),
            None,
            "an ATR that does not fit `i64` must establish no stop rather than a wrapped one"
        );
        // Wilder's smoothing sheds a tenth of that corrupt seed per candle, so the band
        // is still out of scale on the next candle and the stop is still absent — absent
        // and loud beats present and wrong.
        s.fold(&bar(60_000_000, 2_501_000, 2_499_000, 2_500_000));
        assert_eq!(
            s.stop(),
            None,
            "a stop that cannot be represented stays absent, and is not wrapped into range"
        );
    }
}

#[cfg(test)]
mod the_window_and_the_wrap {
    use super::*;

    fn bar(ts: i64, high: i64, low: i64, close: i64) -> Candle {
        Candle {
            ts_micros: ts,
            open: close,
            high,
            low,
            close,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    /// A window that is not full confirms nothing, at every count on the way up.
    ///
    /// The five candles are a real peak — the same ones
    /// `tests::a_swing_high_is_confirmed_two_bars_late` confirms — so a detector that
    /// published from a partly-filled ring would publish this one early, and the
    /// assertion names the count that did it.
    #[test]
    fn a_window_that_is_not_full_confirms_nothing() {
        let highs = [2_500_000_i64, 2_501_000, 2_505_000, 2_502_000, 2_500_500];
        for count in 1..RING {
            let mut d = SwingDetector::new();
            for (ts, high) in (0_i64..).zip(highs.iter().take(count)) {
                d.fold(&bar(ts * 60_000_000, *high, high - 2_000, high - 1_000));
            }
            assert_eq!(
                d.swing_high(),
                None,
                "{count} candles cannot fill a {RING}-candle window"
            );
            assert_eq!(
                d.swing_low(),
                None,
                "{count} candles cannot fill a {RING}-candle window"
            );
        }
    }

    /// A peak whose window straddles the ring's wrap is confirmed at the right candle.
    ///
    /// Ten candles through a five-slot ring, with the only peak at index 6 — so the
    /// window that confirms it spans the wrap. A detector that recovered chronological
    /// order wrongly there would name a neighbour's high, or stamp it against the wrong
    /// candle, and both are asserted exactly rather than as a property.
    #[test]
    fn a_peak_that_straddles_the_wrap_is_confirmed_at_the_right_candle() {
        let highs = [
            2_500_000_i64,
            2_501_000,
            2_502_000,
            2_503_000,
            2_504_000,
            2_505_000,
            2_520_000,
            2_506_000,
            2_505_500,
            2_505_200,
        ];
        let mut d = SwingDetector::new();
        for (i, high) in (0_i64..).zip(highs) {
            d.fold(&bar(i * 60_000_000, high, high - 2_000, high - 1_000));
            if i < 8 {
                assert_eq!(
                    d.swing_high(),
                    None,
                    "candle {i} cannot know the peak at index 6 yet"
                );
            }
        }
        assert_eq!(
            d.swing_high(),
            Some(Swing {
                price: 2_520_000,
                window_span: 18_000,
                confirmed_at: 9,
            }),
            "the peak is index 6's high, in a window spanning 2_502_000 to 2_520_000, \
             confirmed on the ninth candle folded"
        );
    }

    /// One candle can be both the high and the low of its window, and both latches then
    /// carry the same stamp.
    ///
    /// `confirm` publishes twice from one window here. A version that published the high
    /// and returned would leave the swing low stale, and 73 would then measure price
    /// against a level ten candles older than the one 72 uses.
    #[test]
    fn an_outside_candle_confirms_both_latches_on_the_same_candle() {
        let window = [
            (2_501_000_i64, 2_499_000_i64),
            (2_502_000, 2_498_000),
            (2_520_000, 2_480_000),
            (2_503_000, 2_497_000),
            (2_504_000, 2_496_000),
        ];
        let mut d = SwingDetector::new();
        for (i, (high, low)) in (0_i64..).zip(window) {
            d.fold(&bar(i * 60_000_000, high, low, high - 1_000));
        }
        assert_eq!(
            d.swing_high(),
            Some(Swing {
                price: 2_520_000,
                window_span: 40_000,
                confirmed_at: 5,
            }),
            "the outside candle's high is the swing high"
        );
        assert_eq!(
            d.swing_low(),
            Some(Swing {
                price: 2_480_000,
                window_span: 40_000,
                confirmed_at: 5,
            }),
            "and its low is the swing low, stamped on the same candle"
        );
        // Both prices come from ONE candle, so the low is below the high and no close can
        // break both at once. `observe`'s `Equal` arm is therefore unreachable from a
        // detector, and reachable only through a direct call with hand-built swings —
        // `double_break::two_swings_confirmed_together_report_nothing` is that call.
    }
}

#[cfg(test)]
mod every_break_is_classified {
    use super::*;

    /// Every prior direction against every direction a close can break, once each.
    ///
    /// The four `Break` variants come out of a two-way match, and a scenario test walks
    /// one path through it at a time. This walks all six: no prior direction, a prior
    /// long, and a prior short, each against an up break and a down break. `(None,
    /// Down)` is the row that matters most — reporting a `CHoCH` there, because there is
    /// no prior direction to compare against, is the defect the first-break rule exists
    /// to prevent.
    #[test]
    fn every_prior_direction_and_break_direction_pair_is_classified() {
        let high = Swing {
            price: 2_500_000,
            window_span: 10_000,
            confirmed_at: 10,
        };
        let low = Swing {
            price: 2_400_000,
            window_span: 10_000,
            confirmed_at: 20,
        };
        let cases = [
            (None, 2_501_000_i64, Trend::Up, Break::BosBullish),
            (None, 2_399_000, Trend::Down, Break::BosBearish),
            (Some(Trend::Up), 2_501_000, Trend::Up, Break::BosBullish),
            (Some(Trend::Up), 2_399_000, Trend::Down, Break::ChochBearish),
            (Some(Trend::Down), 2_501_000, Trend::Up, Break::ChochBullish),
            (Some(Trend::Down), 2_399_000, Trend::Down, Break::BosBearish),
        ];
        for (last, close, direction, expected) in cases {
            let mut s = Structure { last };
            assert_eq!(
                step_structure(&mut s, close, Some(high), Some(low)),
                Some(expected),
                "a close at {close} with {last:?} behind it must be {expected:?}"
            );
            assert_eq!(
                s.last,
                Some(direction),
                "the break must be remembered as {direction:?}, or the next one is \
                 classified against a stale direction"
            );
        }

        // A close that reaches neither level breaks nothing and remembers nothing.
        let mut s = Structure::new();
        assert_eq!(
            step_structure(&mut s, 2_450_000, Some(high), Some(low)),
            None,
            "a close between the two levels is not a break"
        );
        assert_eq!(
            s.last, None,
            "a candle that broke nothing must not install a direction"
        );
    }
}

#[cfg(test)]
// A sibling module, so it carries the allow in its own right; see `mod tests` for why
// `expect` and not `unreachable!`.
#[allow(clippy::expect_used)]
mod a_period_is_folded_before_it_is_named {
    use super::*;

    fn bar(ts: i64, high: i64, low: i64, close: i64) -> Candle {
        Candle {
            ts_micros: ts,
            open: close,
            high,
            low,
            close,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    fn tol() -> Tolerance {
        vocab::tolerance::pinned_fib().expect("the pinned fib width is valid")
    }

    /// The measured defect: three positions spoke on the SECOND candle of a run.
    ///
    /// All eight of the two families are asserted, not just the three that fired: the
    /// other five are the opposite sides of the same comparisons, and a fix that moved
    /// the defect from `above` to `below` would otherwise pass.
    ///
    /// `TrendState::new(CLASSICAL)`, close 10,000 paisa then 20,000, emitted `[]` and
    /// then `[0, 2, 64]` — `close_above_ema20`, `close_above_ema200` and
    /// `close_above_supertrend`. The "ema200" was the previous close, one candle old;
    /// the trailing stop was seeded from a one-candle true range. Three positions the
    /// vocabulary names as a 20-period, a 200-period and a 10-period-ATR measurement
    /// were artefacts of the fold's first candle, and nothing downstream could tell
    /// them from converged ones.
    ///
    /// `docs/03-vocabulary.md` §4: a bit that cannot be evaluated evaluates **false**,
    /// and never "probably".
    #[test]
    fn the_second_candle_of_a_run_names_no_period_at_all() {
        let tolerance = tol();
        let mut t = TrendState::new(TrendThresholds::CLASSICAL);
        let first = t
            .step(&bar(0, 10_000, 10_000, 10_000), tolerance)
            .expect("a sane candle");
        let second = t
            .step(&bar(60_000_000, 20_000, 20_000, 20_000), tolerance)
            .expect("a sane candle");
        for index in [0_u32, 1, 2, 3, 4, 5, 64, 65] {
            assert!(
                !first.get(index),
                "position {index} spoke on the first candle, before anything was folded"
            );
            assert!(
                !second.get(index),
                "position {index} spoke on the second candle of the run, where the average \
                 behind it is the previous close and the stop is a one-candle range"
            );
        }
    }

    /// And each position speaks on exactly the candle its own period completes.
    ///
    /// The other half of the guard, and it is the half that stops the fix from being a
    /// silent mute. Gating the emission on a counter that never satisfies — or on the
    /// wrong period — would leave positions 0–5 and 64–65 permanently false, which
    /// §4 bans as loudly as a wrong value: a position that can never fire is a
    /// condition the sweep can never rank.
    ///
    /// The emit precedes the fold, so at candle *n* the accumulators hold *n − 1*
    /// candles. A 20-period average is therefore first named at candle 21, a
    /// 200-period at 201, and the 10-period ATR under the stop at 11. The series rises
    /// 1,000 paisa a candle, so once each position is entitled to speak it has
    /// something to say: close is above both averages and above the ratcheting stop,
    /// and the fast average is above the slow one.
    #[test]
    fn each_position_speaks_on_the_candle_its_own_period_completes() {
        let tolerance = tol();
        let mut t = TrendState::new(TrendThresholds::CLASSICAL);
        let watched = [(0_u32, 21_i64), (2, 201), (4, 201), (64, 11)];
        let mut first: [Option<i64>; 4] = [None; 4];
        for n in 1..=260_i64 {
            let price = 2_000_000 + n * 1_000;
            let mask = t
                .step(
                    &bar(n * 60_000_000, price + 500, price - 500, price),
                    tolerance,
                )
                .expect("a sane candle");
            for (seen, (index, _)) in first.iter_mut().zip(watched) {
                if seen.is_none() && mask.get(index) {
                    *seen = Some(n);
                }
            }
            // The falling side of every pair must stay silent on a series that only
            // rises: a gate that let the counter select the wrong arm would show up
            // here and nowhere else.
            for index in [1_u32, 3, 5, 65] {
                assert!(
                    !mask.get(index),
                    "position {index} claimed a fall at candle {n} of a rising series"
                );
            }
        }
        for (seen, (index, expected)) in first.iter().zip(watched) {
            assert_eq!(
                *seen,
                Some(expected),
                "position {index} first spoke at {seen:?}; its period completes at candle \
                 {expected}, so anything earlier names a measurement it has not taken and \
                 anything later — or never — is a live condition the sweep cannot reach"
            );
        }
    }
}
