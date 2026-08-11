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
    seeded: bool,
}

impl Ema {
    /// A fresh average over `period` candles.
    #[must_use]
    pub const fn new(period: i128) -> Self {
        Self {
            period,
            scaled: 0,
            seeded: false,
        }
    }

    /// Fold one price in.
    ///
    /// Seeded with the first price rather than a simple average of the first `period`.
    /// That is a convention and it is stated: seeding with an SMA needs `period`
    /// candles of buffer, which would make the state grow with the period and cost the
    /// constant-space property for no gain in a long run. The two agree to within the
    /// smoothing constant after a few periods.
    pub fn fold(&mut self, price: i64) {
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
}

/// Average true range, Wilder-smoothed, held scaled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Atr {
    period: i128,
    scaled: i128,
    previous_close: Option<i64>,
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
    pub fn fold(&mut self, candle: &Candle) {
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
        if let Some(slot) = self.ring.get_mut(self.next) {
            *slot = Some((candle.high, candle.low));
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
    /// oldest one.
    fn confirm(&mut self) {
        let mut window = self.ring;
        window.rotate_left(self.next);
        let Some(middle_index) = RING.checked_div(2) else {
            return;
        };
        let Some(Some((mid_high, mid_low))) = window.get(middle_index).copied() else {
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
            if i == middle_index {
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
        let span = span_high.saturating_sub(span_low);
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
    pub fn observe(
        &mut self,
        close: i64,
        swing_high: Option<Swing>,
        swing_low: Option<Swing>,
    ) -> Option<Break> {
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
        self.last = Some(direction);
        Some(result)
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
        candle.check()?;
        let bits = self.bits(candle.close, tolerance);
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

    /// The fourteen positions for one closing price.
    ///
    /// # Cost
    ///
    /// Two average comparisons, one stop comparison, two band tests and one structure
    /// classification. No loop over data, no allocation.
    #[must_use]
    pub fn bits(&mut self, close: i64, tolerance: Tolerance) -> ConditionMask {
        let mut mask = ConditionMask::ZERO;

        // 0–3: close against each average. A position stays false while its average is
        // unseeded rather than guessing — docs/03-vocabulary.md §4.
        if let Some(fast) = self.fast.value() {
            mask = side(mask, close, fast, 0, 1);
        }
        if let Some(slow) = self.slow.value() {
            mask = side(mask, close, slow, 2, 3);
        }
        // 4–5: the averages against each other.
        if let (Some(fast), Some(slow)) = (self.fast.value(), self.slow.value()) {
            mask = side(mask, fast, slow, 4, 5);
        }

        // 64–65: close against the trailing stop.
        if let Some(stop) = self.supertrend.stop() {
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
                .observe(close, self.swings.swing_high(), self.swings.swing_low());
        if let Some(b) = broke {
            let index = match b {
                Break::BosBullish => 56,
                Break::BosBearish => 57,
                Break::ChochBullish => 58,
                Break::ChochBearish => 59,
            };
            mask = set(mask, index);
        }
        mask
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
mod tests {
    use super::*;

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
        let Ok(t) = vocab::tolerance::pinned_fib() else {
            unreachable!("the pinned fib width is valid")
        };
        t
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
        let Some(v) = e.value() else {
            unreachable!("seeded")
        };
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
        let Some(v) = e.value() else {
            unreachable!("seeded")
        };
        assert!(
            (1_000_000..=1_000_010).contains(&v),
            "converged to {v}, expected to reach 1_000_000 from above"
        );
    }

    /// An unseeded average sets nothing rather than guessing.
    #[test]
    fn nothing_is_emitted_before_the_averages_exist() {
        let mut t = TrendState::default();
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
            let Ok(_) = t.step(
                &candle(i * 60_000_000, 2_500_000, 2_500_000, 2_500_000),
                tol(),
            ) else {
                unreachable!("sane candle")
            };
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
            let Ok(_) = t.step(
                &candle(i * 60_000_000, price + 500, price - 500, price),
                tol(),
            ) else {
                unreachable!("sane candle")
            };
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
        let Some(v) = a.value() else {
            unreachable!("seeded")
        };
        assert!(
            v > 1_000,
            "ATR {v} ignored the 20,000-paisa gap and used only the 1,000 high-low span"
        );
    }

    /// The `SuperTrend` stop ratchets and never retreats while the trend holds.
    #[test]
    fn the_stop_ratchets_and_does_not_retreat() {
        let mut s = SuperTrend::new(TrendThresholds::CLASSICAL);
        let mut previous: Option<i64> = None;
        for i in 0..60_i64 {
            let price = 2_000_000 + i * 2_000;
            s.fold(&candle(i * 60_000_000, price + 1_000, price - 1_000, price));
            if s.trend() == Trend::Up {
                if let (Some(now), Some(before)) = (s.stop(), previous) {
                    assert!(
                        now >= before,
                        "the stop fell from {before} to {now} in an up trend"
                    );
                }
                previous = s.stop();
            } else {
                previous = None;
            }
        }
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
        let Some(s) = d.swing_high() else {
            unreachable!("the fifth bar completes the window")
        };
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
        assert_eq!(s.observe(2_501_000, high, None), Some(Break::BosBullish));
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
        assert_eq!(s.observe(2_501_000, high, low), Some(Break::BosBullish));
        assert_eq!(s.observe(2_399_000, high, low), Some(Break::ChochBearish));
        // And back again.
        assert_eq!(s.observe(2_501_000, high, low), Some(Break::ChochBullish));
        // Continuing in the same direction is a BoS, not another CHoCH.
        assert_eq!(s.observe(2_502_000, high, low), Some(Break::BosBullish));
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
        assert_eq!(s.observe(2_450_000, high, low), None);
    }

    /// Every position is live and of the kind the table declares.
    #[test]
    fn the_positions_match_the_vocabulary() {
        for index in TrendState::positions() {
            let Some(def) = vocab::table::definition(index) else {
                unreachable!("position {index} is in the table")
            };
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
            s.observe(2_505_000, older_high, newer_low),
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
            s.observe(2_505_000, newer_high, older_low),
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
        assert_eq!(s.observe(2_505_000, high, low), None);
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
            let Ok(tolerance) = vocab::tolerance::pinned_fib() else {
                unreachable!("pinned")
            };
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
