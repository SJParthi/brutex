//! The candlestick patterns — all 62.
//!
//! Vocabulary positions 153–177 and 198–234. Every one is `Kind::Plain`: a
//! pattern is true or false on the bars themselves and needs no band.
//!
//! # The thresholds are conventions, and that is stated rather than hidden
//!
//! A pattern predicate cannot be written without numbers. "A small body", "a long
//! lower wick", "a gap" — each needs a threshold, and **no tracked document in
//! this repository defines any of them.** `CLAUDE.md` §3 rule 1 forbids inventing
//! a number and passing it off as measured, so every threshold this module needs is
//! gathered into [`Thresholds`] in one place, named, given a value, and labelled
//! **UNVERIFIED**. They are the widely-used classical conventions and they are not
//! traceable to NSE or to any source `docs/00-charter.md` records.
//!
//! That is the honest position and it is different from silence in two ways a
//! reader can act on: the numbers are in one struct rather than scattered through
//! 62 predicates, and changing one is a single edit whose blast radius is the
//! struct's own documentation.
//!
//! Contrast position 63 `narrow_cpr_day`, which this crate refuses to compute at
//! all: there, "narrow" has no conventional value either, and unlike a hammer the
//! pattern literature offers none. Refusing is right when no convention exists;
//! declaring is right when one does.
//!
//! # Integers only, and no division
//!
//! Every ratio is cross-multiplied. `body ≤ range/10` is written
//! `body * 10 ≤ range`, and products go through `i128` so a price at the edge of
//! `i64` cannot overflow the comparison. Not one `f32` or `f64`.
//!
//! # Cost
//!
//! A fixed five-bar ring, 62 predicates each a bounded number of integer
//! comparisons, no allocation, and no loop whose length depends on the data. Five
//! bars is the deepest any classical pattern reaches (rising three methods, mat
//! hold, breakaway, ladder bottom), so the state is `5 × 56` bytes and never grows.

use crate::Candle;
use vocab::ConditionMask;

/// How deep the deepest pattern reaches. Five bars: rising three methods, mat
/// hold, breakaway and ladder bottom all need five.
pub const LOOKBACK: usize = 5;

/// The first pattern position, and the first of the appended block.
pub const PATTERN_FIRST: u16 = 153;
/// Where the appended 37 begin.
pub const PATTERN_APPENDED_FIRST: u16 = 198;

/// Every threshold the 62 predicates need, in thousandths.
///
/// **UNVERIFIED.** These are the classical conventions. None is traceable to NSE,
/// to a vendor, or to any source `docs/00-charter.md` records, and none has been
/// measured against this repository's own data. They are gathered here so the set
/// is auditable in one place and a change is one edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Thresholds {
    /// A body this small relative to its range is a doji. 100 = 10%.
    pub doji_body: i64,
    /// A body this large relative to its range is a long body. 700 = 70%.
    pub long_body: i64,
    /// A wick this small relative to its range is "no wick". 50 = 5%.
    pub tiny_wick: i64,
    /// A body this small relative to its range is a "small body" — looser than a
    /// doji, and what harami and star patterns mean. 300 = 30%.
    pub small_body: i64,
    /// A wick must be at least this many thousandths of the body to count as
    /// "long" for a hammer or shooting star. 2000 = twice the body.
    pub long_wick_vs_body: i64,
    /// For a high wave candle, each shadow must be at least this many thousandths
    /// of the **range**. 300 = 30% each side.
    ///
    /// Measured against the range and not the body on purpose. Position 227's
    /// original predicate used two `_vs_body` ratios, which are **vacuous when the
    /// body is zero**: `upper * 1000 >= 0 * 3000` is `upper >= 0`, true for every
    /// bar. So every `open == close` doji set 227 whatever its shadows looked like,
    /// including an upper-91%/lower-9% shooting-star shape — the opposite of "long
    /// shadows on both sides". A ratio against the body cannot express this
    /// pattern; only one against the range can.
    pub high_wave_shadow: i64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self::CLASSICAL
    }
}

impl Thresholds {
    /// The classical set. **UNVERIFIED** — see the struct documentation.
    pub const CLASSICAL: Self = Self {
        doji_body: 100,
        long_body: 700,
        tiny_wick: 50,
        small_body: 300,
        long_wick_vs_body: 2000,
        high_wave_shadow: 300,
    };
}

/// One bar reduced to the quantities every predicate is written in.
///
/// All `i128`, because a product of two `i64` prices does not fit an `i64` and the
/// alternative is a comparison that is correct only for small prices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shape {
    open: i128,
    high: i128,
    low: i128,
    close: i128,
    /// `|close - open|`.
    body: i128,
    /// `high - low`. Never negative: a bar with `high < low` is refused upstream.
    range: i128,
    /// `high - max(open, close)`.
    upper: i128,
    /// `min(open, close) - low`.
    lower: i128,
}

impl Shape {
    fn of(bar: &Candle) -> Self {
        let open = i128::from(bar.open);
        let high = i128::from(bar.high);
        let low = i128::from(bar.low);
        let close = i128::from(bar.close);
        let top = if open > close { open } else { close };
        let bottom = if open < close { open } else { close };
        Self {
            open,
            high,
            low,
            close,
            body: (close - open).abs(),
            range: high - low,
            upper: high - top,
            lower: bottom - low,
        }
    }

    const fn bullish(&self) -> bool {
        self.close > self.open
    }
    const fn bearish(&self) -> bool {
        self.close < self.open
    }
    const fn top(&self) -> i128 {
        if self.open > self.close {
            self.open
        } else {
            self.close
        }
    }
    const fn bottom(&self) -> i128 {
        if self.open < self.close {
            self.open
        } else {
            self.close
        }
    }
    /// The body's midpoint. `midpoint` and not `(a + b) / 2`: the sum of two
    /// prices near the edge of the type overflows, and clippy is right that the
    /// hand-written form is a latent bug even where these values cannot reach it.
    const fn mid(&self) -> i128 {
        self.open.midpoint(self.close)
    }

    /// `body * 1000 <= range * permille`, the cross-multiplied form of
    /// `body <= range * permille/1000`.
    fn body_at_most(&self, permille: i64) -> bool {
        self.body * 1000 <= self.range * i128::from(permille)
    }
    fn body_at_least(&self, permille: i64) -> bool {
        self.body * 1000 >= self.range * i128::from(permille)
    }
    fn upper_at_most(&self, permille: i64) -> bool {
        self.upper * 1000 <= self.range * i128::from(permille)
    }
    fn lower_at_most(&self, permille: i64) -> bool {
        self.lower * 1000 <= self.range * i128::from(permille)
    }
    /// A shadow at least `permille/1000` of the RANGE. Unlike the `_vs_body`
    /// ratios this stays meaningful on a zero-body bar.
    fn upper_vs_range(&self, permille: i64) -> bool {
        self.upper * 1000 >= self.range * i128::from(permille)
    }
    fn lower_vs_range(&self, permille: i64) -> bool {
        self.lower * 1000 >= self.range * i128::from(permille)
    }
    /// A wick at least `permille/1000` of the body.
    fn lower_vs_body(&self, permille: i64) -> bool {
        self.lower * 1000 >= self.body * i128::from(permille)
    }
    fn upper_vs_body(&self, permille: i64) -> bool {
        self.upper * 1000 >= self.body * i128::from(permille)
    }
    fn is_doji(&self, thr: Thresholds) -> bool {
        self.range > 0 && self.body_at_most(thr.doji_body)
    }
    fn is_long(&self, thr: Thresholds) -> bool {
        self.range > 0 && self.body_at_least(thr.long_body)
    }
    fn is_small(&self, thr: Thresholds) -> bool {
        self.range > 0 && self.body_at_most(thr.small_body)
    }
}

/// The last five bars, and the 62 positions they decide.
///
/// Fixed size. It does not grow with the number of bars seen, which is the whole
/// of the constant-space argument for this family.
#[derive(Clone, Copy, Debug)]
pub struct Patterns {
    /// The last [`LOOKBACK`] bars, **oldest first and newest last**. Shifted by
    /// [`Patterns::step`], which is why there is no cursor field: see the comment
    /// on the shift itself.
    ring: [Candle; LOOKBACK],
    /// How many bars have been folded, saturating at [`LOOKBACK`].
    depth: usize,
    /// The IST day of the most recent bar, so a session boundary can break a
    /// pattern rather than let it straddle an overnight gap.
    session_day: i64,
    thresholds: Thresholds,
}

const _: () = assert!(core::mem::size_of::<Patterns>() <= 400);

impl Default for Patterns {
    fn default() -> Self {
        Self::new(Thresholds::CLASSICAL)
    }
}

const EMPTY_BAR: Candle = Candle {
    ts_micros: 0,
    open: 0,
    high: 0,
    low: 0,
    close: 0,
    volume: 0,
    open_interest: i64::MIN,
};

impl Patterns {
    /// A new detector with the given thresholds.
    #[must_use]
    pub const fn new(thresholds: Thresholds) -> Self {
        Self {
            ring: [EMPTY_BAR; LOOKBACK],
            depth: 0,
            session_day: i64::MIN,
            thresholds,
        }
    }

    /// The thresholds in force.
    #[must_use]
    pub const fn thresholds(&self) -> Thresholds {
        self.thresholds
    }

    /// How many bars of history are available, up to [`LOOKBACK`].
    #[must_use]
    pub const fn depth(&self) -> usize {
        self.depth
    }

    /// Candle `n` back from the newest. `0` is the newest.
    ///
    /// The newest bar is always the LAST slot — [`Self::step`] shifts the ring
    /// rather than writing through a cursor — so lag `n` is slot
    /// `LOOKBACK - 1 - n` and there is no modular arithmetic to get wrong. The
    /// subtraction cannot wrap: `n < self.depth` by the line above, and `depth`
    /// never exceeds [`LOOKBACK`].
    fn back(&self, n: usize) -> Option<&Candle> {
        if n >= self.depth {
            return None;
        }
        self.ring.get(LOOKBACK - 1 - n)
    }

    /// The full per-bar step: **fold, then emit.**
    ///
    /// The opposite order from the anchor families, and deliberately: a pattern is
    /// a statement about a run of bars *including the newest one*. An anchor is a
    /// reference price a bar is measured against and must exclude it; a pattern is
    /// a description of the bars themselves and must include it. That distinction
    /// is the one D-0080's block comment records.
    ///
    /// # Errors
    ///
    /// [`crate::Corrupt`] for a record that is not a bar. It is neither folded nor
    /// emitted for.
    pub fn step(&mut self, bar: &Candle) -> Result<ConditionMask, crate::Corrupt> {
        bar.check()?;
        let day = crate::ist_day(bar.ts_micros);
        if day != self.session_day {
            // A pattern must not straddle an overnight gap: 15:29 Friday and 09:15
            // Monday are not adjacent bars, and every multi-bar predicate here
            // assumes adjacency. Dropping the depth to zero is what makes that true:
            // `back` refuses every lag at or beyond it, so yesterday's bars are
            // unreachable from the first bar of the new session onward.
            self.depth = 0;
            self.session_day = day;
        }
        // THE RING IS SHIFTED, NOT INDEXED. This was a cursor and an
        // `if let Some(slot) = self.ring.get_mut(self.next) { .. }`, whose absent arm
        // cannot run while the cursor is held below [`LOOKBACK`]: `cargo llvm-cov`
        // records it as a region no passing test can execute, and a region that cannot
        // run is one nobody can be held to. Destructuring a fixed array is TOTAL, so
        // the oldest bar is dropped by name and there is no arm left to hide. The same
        // objection, and the same answer, as `GapFib::fold`.
        //
        // The newest bar lands in the last slot, which is what `back` now reads. Five
        // slots moved rather than one written — a different constant, the same O(1),
        // and one fewer field to keep in step with the depth.
        //
        // The five names are the five slots, so a change to `LOOKBACK` stops the build
        // here rather than folding a bar into a ring of the wrong length. That is the
        // loud refusal §4 asks for in place of a silent adjustment.
        let [_dropped, second, third, fourth, fifth] = self.ring;
        self.ring = [second, third, fourth, fifth, *bar];
        if self.depth < LOOKBACK {
            self.depth += 1;
        }
        Ok(self.bits())
    }

    /// The 62 positions for the current ring.
    ///
    /// # Cost
    ///
    /// 62 predicates over at most five bars. Both counts are compile-time
    /// constants, no allocation, no division.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "62 named predicates in one table is the readable form; splitting \
                  them across helpers hides which position each one sets"
    )]
    pub fn bits(&self) -> ConditionMask {
        let thr = self.thresholds;
        let mut mask = ConditionMask::ZERO;
        let Some(c0) = self.back(0) else {
            return mask;
        };
        let bar0 = Shape::of(c0);
        if bar0.range <= 0 {
            // A single-price bar, and the four-price doji is not one of the shapes it
            // can have — it is the ONLY one. Every bar that reaches here came through
            // [`Self::step`], which ran `Candle::check`, which refuses a record unless
            // `low <= open, close <= high`. A zero range therefore forces
            // `open == close == high == low` and there is no second case to handle.
            //
            // This used to re-test that equality and fall through to `return mask` when
            // it failed. The fall-through cannot execute while `check` is correct, so
            // `cargo llvm-cov` reported it uncovered forever and a reader could not tell
            // a dead branch from a case somebody forgot. The premise is pinned by
            // `a_zero_range_bar_cannot_carry_an_open_or_close_off_the_price` instead,
            // which is where it belongs: in a test, not in a branch nothing takes.
            return set(mask, 225);
        }

        // ---- one-bar patterns -------------------------------------------------
        let hammer_shape = bar0.is_small(thr)
            && bar0.lower_vs_body(thr.long_wick_vs_body)
            && bar0.upper_at_most(thr.tiny_wick);
        let star_shape = bar0.is_small(thr)
            && bar0.upper_vs_body(thr.long_wick_vs_body)
            && bar0.lower_at_most(thr.tiny_wick);
        if hammer_shape {
            mask = set(mask, 153);
        }
        if star_shape {
            mask = set(mask, 154);
        }
        if bar0.is_doji(thr)
            && bar0.lower_vs_body(thr.long_wick_vs_body)
            && bar0.upper_at_most(thr.tiny_wick)
        {
            mask = set(mask, 174);
        }
        if bar0.is_doji(thr)
            && bar0.upper_vs_body(thr.long_wick_vs_body)
            && bar0.lower_at_most(thr.tiny_wick)
        {
            mask = set(mask, 175);
        }
        if bar0.is_doji(thr)
            && !bar0.upper_at_most(thr.tiny_wick)
            && !bar0.lower_at_most(thr.tiny_wick)
        {
            mask = set(mask, 224);
            // A rickshaw man is a long-legged doji whose body sits mid-range.
            let centred =
                (bar0.mid() - bar0.high.midpoint(bar0.low)).abs() * 1000 <= bar0.range * 100;
            if centred {
                mask = set(mask, 226);
            }
        }
        // A high wave candle is a small body with VERY LONG shadows on both sides.
        //
        // The two `_vs_body` tests alone do not say that. They are ratios against
        // the body, so on a zero-body bar `upper * 1000 >= body * 3000` reduces to
        // `0 >= 0`, which is true — and a four-price doji (open == high == low ==
        // close, both shadows of length zero) satisfied every clause and set 227.
        // The bit then meant "long shadows" on a bar with no shadows at all, which
        // is the opposite of the pattern. Requiring neither shadow to be tiny is
        // the same guard 224 already uses one branch above, and it is what makes
        // the ratio tests mean what their names say.
        if bar0.is_doji(thr)
            && bar0.upper_vs_range(thr.high_wave_shadow)
            && bar0.lower_vs_range(thr.high_wave_shadow)
        {
            mask = set(mask, 227);
        }
        if bar0.is_small(thr)
            && !bar0.is_doji(thr)
            && bar0.upper * 1000 >= bar0.range * 200
            && bar0.lower * 1000 >= bar0.range * 200
        {
            mask = set(mask, 171);
        }
        if bar0.is_long(thr)
            && bar0.upper_at_most(thr.tiny_wick)
            && bar0.lower_at_most(thr.tiny_wick)
        {
            mask = set(mask, if bar0.bullish() { 172 } else { 173 });
        }
        if bar0.is_long(thr) && bar0.bullish() && bar0.lower_at_most(thr.tiny_wick) {
            mask = set(mask, 204);
        }
        if bar0.is_long(thr) && bar0.bearish() && bar0.upper_at_most(thr.tiny_wick) {
            mask = set(mask, 205);
        }

        // ---- two-bar patterns -------------------------------------------------
        let Some(c1) = self.back(1) else {
            return mask;
        };
        let bar1 = Shape::of(c1);

        // A hanging man is a hammer after an advance; an inverted hammer that
        // follows one is a shooting star. Prior direction is the only difference,
        // which is why they are separate positions and not separate shapes.
        if hammer_shape && bar1.bullish() {
            mask = set(mask, 155);
        }
        if star_shape && bar1.bullish() {
            mask = set(mask, 156);
        }
        if bar1.bearish() && bar0.bullish() && bar0.open <= bar1.close && bar0.close >= bar1.open {
            mask = set(mask, 157);
        }
        if bar1.bullish() && bar0.bearish() && bar0.open >= bar1.close && bar0.close <= bar1.open {
            mask = set(mask, 158);
        }
        if bar1.bearish()
            && bar0.bullish()
            && bar0.top() <= bar1.open
            && bar0.bottom() >= bar1.close
        {
            mask = set(mask, 159);
        }
        if bar1.bullish()
            && bar0.bearish()
            && bar0.top() <= bar1.close
            && bar0.bottom() >= bar1.open
        {
            mask = set(mask, 160);
        }
        if bar1.bearish()
            && bar0.bullish()
            && bar0.open < bar1.low
            && bar0.close > bar1.mid()
            && bar0.close < bar1.open
        {
            mask = set(mask, 161);
        }
        if bar1.bullish()
            && bar0.bearish()
            && bar0.open > bar1.high
            && bar0.close < bar1.mid()
            && bar0.close > bar1.open
        {
            mask = set(mask, 162);
        }
        if bar0.high == bar1.high && (bar0.bullish() != bar1.bullish()) {
            mask = set(mask, 169);
        }
        if bar0.low == bar1.low && (bar0.bullish() != bar1.bullish()) {
            mask = set(mask, 170);
        }
        if bar1.bearish() && bar0.bullish() && bar0.open > bar1.open {
            mask = set(mask, 202);
        }
        if bar1.bullish() && bar0.bearish() && bar0.open < bar1.open {
            mask = set(mask, 203);
        }
        if (bar1.bearish() && bar0.bullish() || bar1.bullish() && bar0.bearish())
            && bar0.close == bar1.close
        {
            mask = set(mask, if bar0.bullish() { 206 } else { 207 });
        }
        if bar1.bullish() && bar0.bullish() && bar0.open == bar1.open {
            mask = set(mask, 208);
        }
        if bar1.bearish() && bar0.bearish() && bar0.open == bar1.open {
            mask = set(mask, 209);
        }
        if bar1.bearish() && bar0.bullish() && bar0.open < bar1.low && bar0.close == bar1.low {
            mask = set(mask, 210);
        }
        if bar1.bearish()
            && bar0.bullish()
            && bar0.open < bar1.low
            && bar0.close > bar1.low
            && bar0.close <= bar1.close
        {
            mask = set(mask, 211);
        }
        if bar1.bearish()
            && bar0.bullish()
            && bar0.open < bar1.low
            && bar0.close > bar1.close
            && bar0.close < bar1.mid()
        {
            mask = set(mask, 212);
        }
        if bar1.bearish() && bar0.bearish() && bar0.close == bar1.close {
            mask = set(mask, 229);
        }
        if bar1.bullish()
            && bar0.bullish()
            && bar0.is_small(thr)
            && bar1.is_small(thr)
            && bar0.body == bar1.body
        {
            mask = set(mask, 228);
        }

        // ---- three-bar patterns ----------------------------------------------
        let Some(c2) = self.back(2) else {
            return mask;
        };
        let bar2 = Shape::of(c2);

        if bar2.bearish()
            && bar2.is_long(thr)
            && bar1.is_small(thr)
            && bar0.bullish()
            && bar0.close > bar2.mid()
        {
            mask = set(mask, 163);
        }
        if bar2.bullish()
            && bar2.is_long(thr)
            && bar1.is_small(thr)
            && bar0.bearish()
            && bar0.close < bar2.mid()
        {
            mask = set(mask, 164);
        }
        if bar2.bullish()
            && bar1.bullish()
            && bar0.bullish()
            && bar1.close > bar2.close
            && bar0.close > bar1.close
        {
            mask = set(mask, 165);
            if bar1.body < bar2.body && bar0.body < bar1.body {
                mask = set(mask, 231);
            }
            if bar0.is_small(thr) && bar1.is_long(thr) {
                mask = set(mask, 232);
            }
        }
        if bar2.bearish()
            && bar1.bearish()
            && bar0.bearish()
            && bar1.close < bar2.close
            && bar0.close < bar1.close
        {
            mask = set(mask, 166);
            if bar0.open == bar1.open && bar1.open == bar2.open {
                mask = set(mask, 230);
            }
        }
        if bar2.bearish()
            && bar1.bullish()
            && bar1.top() <= bar2.open
            && bar1.bottom() >= bar2.close
            && bar0.bullish()
            && bar0.close > bar2.open
        {
            mask = set(mask, 167);
        }
        if bar2.bullish()
            && bar1.bearish()
            && bar1.top() <= bar2.close
            && bar1.bottom() >= bar2.open
            && bar0.bearish()
            && bar0.close < bar2.open
        {
            mask = set(mask, 168);
        }
        if bar2.bearish()
            && bar1.is_doji(thr)
            && bar1.high < bar2.low
            && bar0.bullish()
            && bar0.low > bar1.high
        {
            mask = set(mask, 198);
        }
        if bar2.bullish()
            && bar1.is_doji(thr)
            && bar1.low > bar2.high
            && bar0.bearish()
            && bar0.high < bar1.low
        {
            mask = set(mask, 199);
        }
        if bar2.bullish()
            && bar1.bullish()
            && bar0.bearish()
            && bar1.low > bar2.high
            && bar0.close < bar1.low
            && bar0.close > bar2.bottom()
        {
            mask = set(mask, 213);
        }
        if bar2.bearish()
            && bar1.bearish()
            && bar0.bullish()
            && bar1.high < bar2.low
            && bar0.close > bar1.high
            && bar0.close < bar2.top()
        {
            mask = set(mask, 214);
        }
        if bar2.bullish()
            && bar1.bullish()
            && bar0.bullish()
            && bar1.low > bar2.high
            && bar0.open == bar1.open
            && bar0.body == bar1.body
        {
            mask = set(mask, 215);
        }
        if bar2.bearish() && bar1.bullish() && bar0.bearish() && bar0.close == bar2.close {
            mask = set(mask, 217);
        }
        if bar2.bearish()
            && bar2.is_long(thr)
            && bar1.bearish()
            && bar1.open < bar2.close
            && bar0.bullish()
            && bar0.open < bar1.close
            && bar0.close > bar1.open
        {
            mask = set(mask, 221);
        }
        // A tri-star's direction comes from where the third doji closes relative to
        // the first. When they close at the SAME price the pattern has no direction,
        // and `else` handed that case to the bearish bit — the same degenerate-falls-
        // into-one-branch defect that made position 227 fire on every zero-body bar.
        // Three dojis at one price is not a bearish tri-star; it is not a tri-star.
        if bar2.is_doji(thr) && bar1.is_doji(thr) && bar0.is_doji(thr) {
            if bar0.close > bar2.close {
                mask = set(mask, 233);
            } else if bar0.close < bar2.close {
                mask = set(mask, 234);
            }
        }

        // ---- four- and five-bar patterns -------------------------------------
        let Some(c3) = self.back(3) else {
            return mask;
        };
        let bar3 = Shape::of(c3);
        if bar3.bearish()
            && bar2.bearish()
            && bar1.bearish()
            && bar0.bullish()
            && bar0.close > bar3.open
        {
            mask = set(mask, 200);
        }
        if bar3.bullish()
            && bar2.bullish()
            && bar1.bullish()
            && bar0.bearish()
            && bar0.close < bar3.open
        {
            mask = set(mask, 201);
        }
        if bar3.bearish()
            && bar2.bearish()
            && bar1.bearish()
            && bar1.upper > 0
            && bar0.bearish()
            && bar0.high > bar1.high
        {
            mask = set(mask, 220);
        }

        let Some(c4) = self.back(4) else {
            return mask;
        };
        let bar4 = Shape::of(c4);
        if bar4.bullish()
            && bar4.is_long(thr)
            && bar2.bearish()
            && bar1.bearish()
            && bar3.bearish()
            && bar0.bullish()
            && bar0.close > bar4.close
        {
            mask = set(mask, 176);
        }
        if bar4.bearish()
            && bar4.is_long(thr)
            && bar2.bullish()
            && bar1.bullish()
            && bar3.bullish()
            && bar0.bearish()
            && bar0.close < bar4.close
        {
            mask = set(mask, 177);
        }
        if bar4.bullish()
            && bar4.is_long(thr)
            && bar3.bullish()
            && bar2.is_small(thr)
            && bar1.is_small(thr)
            && bar0.bullish()
            && bar0.close > bar3.high
        {
            mask = set(mask, 216);
        }
        if bar4.bearish()
            && bar3.bearish()
            && bar2.bearish()
            && bar1.bearish()
            && bar0.bullish()
            && bar0.close > bar2.high
        {
            mask = set(mask, 218);
        }
        if bar4.bullish()
            && bar3.bullish()
            && bar2.bullish()
            && bar1.bullish()
            && bar0.bearish()
            && bar0.close < bar2.low
        {
            mask = set(mask, 219);
        }
        if bar4.bearish()
            && bar4.is_long(thr)
            && bar3.bearish()
            && bar2.bearish()
            && bar1.bearish()
            && bar0.bullish()
            && bar0.close > bar3.high
        {
            mask = set(mask, 222);
        }
        if bar4.bullish()
            && bar4.is_long(thr)
            && bar3.bullish()
            && bar2.bullish()
            && bar1.bullish()
            && bar0.bearish()
            && bar0.close < bar3.low
        {
            mask = set(mask, 223);
        }
        mask
    }
}

/// Set a plain position, ignoring a refusal.
///
/// A refusal means this module and the table disagree about a position number,
/// which `pattern::every_position_is_a_live_plain_pattern` makes a test failure.
fn set(mask: ConditionMask, index: u16) -> ConditionMask {
    vocab::table::set_exact(mask, index).unwrap_or(mask)
}

/// Every position this module can set: 153–177 and 198–234.
///
/// **ZIPPED, NOT INDEXED.** This was two `while` loops over a cursor and an
/// `if let Some(slot) = out.get_mut(i) { .. }` in each, whose absent arm cannot run
/// while the cursor stays below 62 — the same region `cargo llvm-cov` can never
/// execute, and the same answer as the shift in [`Patterns::step`]. `zip` stops at
/// the shorter side, so no arm is left dead, and a plan that ran short would leave
/// a zero behind that `pattern::every_position_is_a_live_plain_pattern` refuses.
#[must_use]
pub fn positions() -> [u16; 62] {
    let mut out = [0u16; 62];
    let plan = (PATTERN_FIRST..=177).chain(PATTERN_APPENDED_FIRST..=234);
    for (slot, index) in out.iter_mut().zip(plan) {
        *slot = index;
    }
    out
}

// `.expect` and not `let .. else { unreachable!() }`. The `unreachable!` form expands
// to a panic written IN THIS CRATE, so `cargo llvm-cov` records a region no passing
// test can ever execute — and a region that cannot run is one nobody can be held to.
// `Result::expect` panics inside the standard library, which is not instrumented, and
// refuses just as loudly. The workspace denies `expect_used` for library code, where a
// panic is a real defect; the same allow sits on the test modules of
// `indicators::daily`, `indicators::orb` and eleven others.
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn mk(ts: i64, open: i64, high: i64, low: i64, close: i64) -> Candle {
        Candle {
            ts_micros: ts,
            open,
            high,
            low,
            close,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    /// Minute `m` of IST day 30,000.
    fn at(minute: i64, open: i64, high: i64, low: i64, close: i64) -> Candle {
        let ts = (30_000 * 1_440 + 555 + minute) * 60_000_000 - 19_800 * 1_000_000;
        mk(ts, open, high, low, close)
    }

    fn ok(p: &mut Patterns, bar: &Candle) -> ConditionMask {
        p.step(bar).expect("this fixture bar is sane")
    }

    /// EVERY SHADOW RATIO MEETS ITS THRESHOLD EXACTLY, AND ZERO RANGE IS NOT A BAR.
    ///
    /// The `Shape` ratios are cross-multiplied — `upper * 1000 >= range *
    /// permille` — and three mutations live on each: the `*` turned into `+`,
    /// into `/`, and the whole predicate replaced by `true`. All three need a
    /// case whose answer is FALSE and a threshold hit exactly, which no test in
    /// this file had.
    ///
    /// `is_doji`, `is_long` and `is_small` add `range > 0`. A zero-range bar —
    /// open, high, low and close all equal — is the only input where `>` and
    /// `>=` disagree there, and it is a real bar: a halted or untraded minute
    /// prints exactly that.
    #[test]
    fn the_shadow_ratios_are_exact_and_a_zero_range_bar_is_none_of_them() {
        // range 100, upper 20, lower 30, body 50.
        // open 130 close 180 -> body 50, top 180, bottom 130.
        // high 200 -> upper 20. low 100 -> lower 30. range 100.
        let sh = Shape::of(&mk(0, 130, 200, 100, 180));
        assert_eq!(sh.range, 100, "the fixture's range is what the maths uses");
        assert_eq!(sh.upper, 20, "and its upper shadow");
        assert_eq!(sh.lower, 30, "and its lower");
        assert_eq!(sh.body, 50, "and its body");

        // 20/100 = 200 permille, exactly.
        assert!(sh.upper_at_most(200), "20 of 100 IS at most 200 permille");
        assert!(!sh.upper_at_most(199), "and is not at most 199");
        assert!(sh.upper_vs_range(200), "it is also at least 200");
        assert!(
            !sh.upper_vs_range(201),
            "but not at least 201 -- the case a `-> true` replacement cannot \
             survive, and the one that forces the cross-multiply to multiply"
        );

        // 30/100 = 300 permille, exactly.
        assert!(sh.lower_at_most(300), "30 of 100 IS at most 300 permille");
        assert!(!sh.lower_at_most(299), "and is not at most 299");
        assert!(sh.lower_vs_range(300), "and at least 300");

        // Against the BODY rather than the range: 30/50 = 600, 20/50 = 400.
        assert!(sh.lower_vs_body(600), "30 of a 50 body IS 600 permille");
        assert!(!sh.lower_vs_body(601), "and not 601");
        assert!(sh.upper_vs_body(400), "20 of a 50 body IS 400 permille");
        assert!(
            !sh.upper_vs_body(401),
            "and not 401 -- these two ratios divide by the BODY, so a bar with \
             a large body and the same shadows must answer differently"
        );

        // ── range > 0, and the bar that makes it matter ──────────────────────
        //
        // A halted minute prints open == high == low == close. Its range is
        // zero, its body is zero, and `body_at_most` is `0 <= 0` — true for
        // every threshold. Without the `range > 0` guard EVERY such bar would be
        // a doji, a long body and a small body at once, which is three
        // contradictory bits on a bar that never traded.
        let flat = Shape::of(&mk(0, 100, 100, 100, 100));
        assert_eq!(flat.range, 0, "a halted minute has no range");
        let thr = Thresholds::CLASSICAL;
        assert!(!flat.is_doji(thr), "a bar that never traded is not a doji");
        assert!(!flat.is_long(thr), "nor a long body");
        assert!(!flat.is_small(thr), "nor a small one");

        // And a real bar still classifies, so the guard refuses only the flat one.
        let tiny = Shape::of(&mk(0, 100, 200, 100, 101));
        assert!(
            tiny.is_doji(thr),
            "a one-paisa body over a 100 range is a doji"
        );
        let big = Shape::of(&mk(0, 100, 200, 100, 190));
        assert!(
            big.is_long(thr),
            "and a 90-of-100 body is a long one -- the guard refuses only the \
             flat bar, not every bar"
        );
    }

    /// THE PARTIAL-RECOVERY PATTERN: THREE PRICE CLAUSES, EACH AT ITS EDGE.
    ///
    /// Bit 212 needs a bar that gapped below the prior low, closed back above
    /// the prior close, and yet failed to reach the prior body's midpoint. Those
    /// three prices bracket a narrow window, and each edge of it is a separate
    /// clause — an `&&` turned into `||` makes any one of them sufficient, which
    /// would fire this bit on bars that never recovered at all.
    ///
    /// Every clause gets two cases: one past its threshold, which kills the
    /// `&&`, and one exactly at it, which kills the comparison. They are not
    /// interchangeable — `>` and `<` both refuse a value on the wrong side, so
    /// only the threshold itself separates a strict comparison from a loose one.
    #[test]
    fn every_edge_of_the_partial_recovery_window_is_required() {
        // bar1 bearish: open 200, close 100, low 90, mid 150.
        let prior = at(10, 200, 210, 90, 100);
        let fires = |bar0: &Candle| -> bool {
            let mut p = Patterns::default();
            let _prev = ok(&mut p, &prior);
            ok(&mut p, bar0).get(212)
        };

        assert!(
            fires(&at(11, 80, 125, 75, 120)),
            "opening at 80 below the 90 low and closing at 120 -- above the 100 \
             close, below the 150 midpoint -- satisfies all five clauses"
        );

        // `bar0.open < bar1.low`
        assert!(
            !fires(&at(11, 95, 125, 90, 120)),
            "an open ABOVE the prior low never gapped down, so there is nothing \
             to recover from"
        );
        assert!(
            !fires(&at(11, 90, 125, 85, 120)),
            "and an open EXACTLY at the prior low has not gapped below it; `<=` \
             would accept this and `<` must not"
        );

        // `bar0.close > bar1.close`
        assert!(
            !fires(&at(11, 80, 125, 75, 95)),
            "a close BELOW the prior close recovered nothing"
        );
        assert!(
            !fires(&at(11, 80, 125, 75, 100)),
            "and a close EXACTLY at the prior close recovered nothing either -- \
             `>=` would call this a recovery and `>` must not"
        );

        // `bar0.close < bar1.mid()`
        assert!(
            !fires(&at(11, 80, 165, 75, 160)),
            "a close ABOVE the midpoint is a full piercing, which is bit 161 -- \
             this clause is what keeps the two apart"
        );
        assert!(
            !fires(&at(11, 80, 155, 75, 150)),
            "and a close EXACTLY at the midpoint has reached it, so the recovery \
             is not partial; `<=` would accept this and `<` must not"
        );
    }

    /// THE HARAMI PAIR: CONTAINMENT IS TWO CLAUSES, NOT ONE.
    ///
    /// Bits 159 and 160 fire when the current body sits wholly inside the prior
    /// one. That containment is TWO separate clauses — a top under the prior
    /// body's upper edge and a bottom over its lower edge — and an `&&` turned
    /// into `||` makes either one alone sufficient, which is a bar poking out of
    /// the prior body in one direction being called an inside bar.
    ///
    /// Three of the four clauses on each bit are isolable. The prior bar's own
    /// direction is not: making `bar1` a doji collapses its open and close onto
    /// one price, so the two containment bounds become the same number and the
    /// second clause fails alongside the first. Recorded rather than faked.
    #[test]
    fn the_harami_body_must_be_contained_on_both_sides() {
        // bar1 bearish, open 200 close 100 — the containing body.
        let down = at(10, 200, 210, 90, 100);
        let inside_up = |bar0: &Candle| -> bool {
            let mut p = Patterns::default();
            let _prev = ok(&mut p, &down);
            ok(&mut p, bar0).get(159)
        };

        assert!(
            inside_up(&at(11, 120, 185, 115, 180)),
            "a bullish body from 120 to 180 sits wholly inside 100..200"
        );
        assert!(
            !inside_up(&at(11, 120, 215, 115, 210)),
            "a top ABOVE the prior open is not contained; `||` would accept this \
             on the strength of the bottom alone"
        );
        assert!(
            !inside_up(&at(11, 90, 185, 85, 180)),
            "and a bottom BELOW the prior close is not contained either -- both \
             bounds are required, which is what the `&&` between them means"
        );
        assert!(
            !inside_up(&at(11, 150, 185, 115, 150)),
            "a bar that closed where it opened has not risen, so it is no \
             bullish harami however well contained it is"
        );

        // 160 is the mirror: a bearish body inside a bullish one.
        let up = at(10, 100, 210, 90, 200);
        let inside_down = |bar0: &Candle| -> bool {
            let mut p = Patterns::default();
            let _prev = ok(&mut p, &up);
            ok(&mut p, bar0).get(160)
        };

        assert!(
            inside_down(&at(11, 180, 185, 115, 120)),
            "a bearish body from 180 down to 120 sits wholly inside 100..200"
        );
        assert!(
            !inside_down(&at(11, 210, 215, 115, 120)),
            "a top above the prior close is not contained"
        );
        assert!(
            !inside_down(&at(11, 180, 185, 85, 90)),
            "nor is a bottom below the prior open"
        );
        assert!(
            !inside_down(&at(11, 150, 185, 115, 150)),
            "and a doji has not fallen, so it is no bearish harami"
        );
    }

    /// THE DARK-CLOUD MIRROR: EVERY CLAUSE AND EVERY THRESHOLD.
    ///
    /// Bit 162 is bit 161 reflected — prior bar up, current bar down, gapping
    /// ABOVE the prior high instead of below the prior low. Seven mutants live
    /// on it: four `&&` turned into `||` and three comparisons flipped one step.
    ///
    /// Both techniques are applied to each price clause, because they are not
    /// interchangeable and a earlier version of the 161 test proved it. A clause
    /// negated well past its threshold kills the `&&` and leaves the comparison
    /// alive — `>` and `>=` both refuse an open of 205 against a high of 210.
    /// Only the threshold value itself separates them.
    #[test]
    fn every_clause_and_threshold_of_the_dark_cloud_pattern_is_required() {
        // bar1: bullish, open 100 close 200, high 210, mid 150.
        let prior = at(10, 100, 210, 90, 200);
        // bar0: bearish, open 220 (> high), close 130 (< mid, > prior open).
        let fires = |bar0: &Candle| -> bool {
            let mut p = Patterns::default();
            let _prev = ok(&mut p, &prior);
            ok(&mut p, bar0).get(162)
        };

        assert!(
            fires(&at(11, 220, 225, 125, 130)),
            "the fixture must satisfy all five clauses first -- negating one \
             clause of a pattern that never fired proves nothing about it"
        );

        // `bar0.open > bar1.high`: below it, then exactly at it.
        assert!(
            !fires(&at(11, 205, 210, 125, 130)),
            "an open BELOW the prior high has not gapped above it"
        );
        assert!(
            !fires(&at(11, 210, 215, 125, 130)),
            "and an open EXACTLY at the prior high has not gapped either -- the \
             comparison is strict, and `>=` would accept this"
        );

        // `bar0.close < bar1.mid()`: above it, then exactly at it.
        assert!(
            !fires(&at(11, 220, 225, 125, 160)),
            "a close ABOVE the prior body's midpoint has not clouded it"
        );
        assert!(
            !fires(&at(11, 220, 225, 125, 150)),
            "and a close EXACTLY at the midpoint has not passed it; `<=` would \
             accept this and `<` must not"
        );

        // `bar0.close > bar1.open`: below it, then exactly at it.
        assert!(
            !fires(&at(11, 220, 225, 85, 90)),
            "a close BELOW the prior open is a full engulfing, not a dark cloud"
        );
        assert!(
            !fires(&at(11, 220, 225, 95, 100)),
            "and a close EXACTLY at the prior open is a full retracement; `>=` \
             would accept this and `>` must not"
        );
    }

    /// THE GAP PATTERNS: EVERY CLAUSE ISOLABLE, INCLUDING THE COMPARISON.
    ///
    /// Bits 202 and 203 are three clauses each, and unlike the piercing pattern
    /// **all three can be negated one at a time** — nothing here constrains the
    /// price relative to the prior body, so a doji breaks a direction clause
    /// without disturbing the others.
    ///
    /// Six mutants live on these two lines: four `&&` turned into `||`, and the
    /// two comparisons flipped one step. The comparison needs the boundary value
    /// itself — `bar0.open` exactly equal to `bar1.open`, which `>` refuses and
    /// `>=` accepts.
    #[test]
    fn every_clause_of_the_gap_patterns_is_required_and_the_gap_is_strict() {
        // 202: prior bearish, current bullish, current opens ABOVE prior open.
        let prior_down = at(10, 200, 210, 90, 100);
        let up = |bar0: &Candle| -> bool {
            let mut p = Patterns::default();
            let _prev = ok(&mut p, &prior_down);
            ok(&mut p, bar0).get(202)
        };

        assert!(
            up(&at(11, 210, 260, 205, 250)),
            "the fixture must fire first"
        );
        assert!(
            !up(&at(11, 210, 260, 205, 210)),
            "a current bar that closed where it opened has not risen; the \
             `bar0.bullish()` clause carries this alone"
        );
        assert!(
            !up(&at(11, 190, 260, 185, 250)),
            "opening BELOW the prior open is not a gap up, whatever the close did"
        );
        assert!(
            !up(&at(11, 200, 260, 195, 250)),
            "opening EXACTLY at the prior open is not a gap either -- the \
             comparison is strict, and `>=` would call this a gap"
        );

        // The prior bar's own direction, negated with a doji so nothing else moves.
        let mut p = Patterns::default();
        let _prev = ok(&mut p, &at(10, 200, 210, 90, 200));
        assert!(
            !ok(&mut p, &at(11, 210, 260, 205, 250)).get(202),
            "a prior bar that closed where it opened was not a decline, so this \
             is no reversal; the `bar1.bearish()` clause carries this alone"
        );

        // 203 is the mirror: prior bullish, current bearish, opening BELOW.
        let prior_up = at(10, 100, 210, 90, 200);
        let down = |bar0: &Candle| -> bool {
            let mut p = Patterns::default();
            let _prev = ok(&mut p, &prior_up);
            ok(&mut p, bar0).get(203)
        };

        assert!(
            down(&at(11, 90, 95, 40, 50)),
            "the mirror fixture fires too"
        );
        assert!(
            !down(&at(11, 90, 95, 40, 90)),
            "a doji has not fallen; `bar0.bearish()` carries this"
        );
        assert!(
            !down(&at(11, 110, 115, 40, 50)),
            "opening ABOVE the prior open is not a gap down"
        );
        assert!(
            !down(&at(11, 100, 115, 40, 50)),
            "and opening EXACTLY at it is not a gap down either -- strict, so \
             `<=` would wrongly accept this"
        );
    }

    /// EACH CLAUSE OF A TWO-BAR PATTERN IS LOAD-BEARING ON ITS OWN.
    ///
    /// Bit 161 fires on five conditions joined by `&&`. `cargo-mutants` turns
    /// each `&&` into `||`, and **the only input that separates them is one
    /// where exactly ONE clause is false and every other holds** — with `||`
    /// that bar still fires the bit, with `&&` it must not. A test that simply
    /// fails every clause at once proves nothing: both versions refuse it.
    ///
    /// Forty-six of this file's survivors are `&&` in `Patterns::bits`, and this
    /// is the shape that kills them.
    ///
    /// # Two of the five clauses cannot be isolated, and that is arithmetic
    ///
    /// `bar1.bearish()` and `bar0.bullish()` are entangled with the price
    /// clauses beneath them. The pattern needs `bar0.close` above `bar1.mid()`
    /// and below `bar1.open`, and `bar0.open` below `bar1.low` — so making
    /// `bar1` bullish moves `bar1.open` below `bar0.close` and breaks a second
    /// clause at the same time, and making `bar0` bearish needs its close below
    /// an open that is already below `bar1.low`, which puts the close under
    /// `bar1.mid()` too. No single-clause negation exists for either, so they
    /// are left to the three that do and recorded here rather than faked.
    #[test]
    fn every_price_clause_of_the_piercing_pattern_is_required_on_its_own() {
        // bar1: bearish, open 200 close 100, low 90, mid 150.
        let prior = at(10, 200, 210, 90, 100);
        // bar0: bullish, open 80 (< low), close 170 (> mid, < open of bar1).
        let firing = at(11, 80, 180, 75, 170);

        let fires = |bar0: &Candle| -> bool {
            let mut p = Patterns::default();
            let _prev = ok(&mut p, &prior);
            ok(&mut p, bar0).get(161)
        };

        assert!(
            fires(&firing),
            "the fixture must satisfy all five clauses, or negating one proves \
             nothing about that clause"
        );

        // Clause: `bar0.open < bar1.low`. 95 is above bar1's low of 90 and
        // everything else still holds.
        assert!(
            !fires(&at(11, 95, 180, 75, 170)),
            "an open ABOVE the prior low is not a piercing gap-down; `||` would \
             still fire this bit"
        );

        // Clause: `bar0.close > bar1.mid()`. 140 is under the 150 midpoint.
        assert!(
            !fires(&at(11, 80, 180, 75, 140)),
            "a close that failed to reclaim the prior body's midpoint has not \
             pierced it"
        );

        // Clause: `bar0.close < bar1.open`. 205 is above bar1's open of 200.
        assert!(
            !fires(&at(11, 80, 210, 75, 205)),
            "a close ABOVE the prior open is an engulfing, not a piercing -- the \
             two are different bits and this clause is what separates them"
        );

        // ── THE BOUNDARY VALUES, WHICH THE THREE ABOVE DO NOT REACH ──────────
        //
        // Negating a clause with a value well past its threshold kills the `&&`
        // and leaves the COMPARISON untouched: 95 against a low of 90 is refused
        // by `<` and by `<=` alike. Only the threshold value itself separates
        // them, and a first version of this test did not have it -- measured, by
        // re-running mutation and finding 475, 476 and 477 still alive.
        assert!(
            !fires(&at(11, 90, 180, 85, 170)),
            "an open EXACTLY at the prior low has not gapped below it; `<=` \
             would accept this and `<` must not"
        );
        assert!(
            !fires(&at(11, 80, 180, 75, 150)),
            "a close EXACTLY at the prior body's midpoint has not passed it; \
             `>=` would accept this and `>` must not"
        );
        assert!(
            !fires(&at(11, 80, 205, 75, 200)),
            "a close EXACTLY at the prior open is a full retracement, not a \
             partial pierce; `<=` would accept this and `<` must not"
        );
    }

    /// All 62 positions are live, plain, and named `pat_*`.
    #[test]
    fn every_position_is_a_live_plain_pattern() {
        let mut seen = std::collections::BTreeSet::new();
        for index in positions() {
            assert!(seen.insert(index), "position {index} appears twice");
            let found = vocab::table::definition(index);
            assert!(found.is_some(), "position {index} is not in the table");
            let def = found.expect("asserted to be Some on the line above");
            assert!(vocab::table::is_live(index), "position {index} is not live");
            assert_eq!(
                def.kind,
                vocab::Kind::Plain,
                "position {index} (`{}`) needs a tolerance and a pattern has none",
                def.name,
            );
            assert!(
                def.name.starts_with("pat_"),
                "position {index} is `{}`, not a pattern",
                def.name,
            );
        }
        assert_eq!(seen.len(), 62);
    }

    /// The ring covers exactly 153..=177 and 198..=234, with no stray index.
    #[test]
    fn the_position_set_is_the_two_pattern_blocks() {
        let owned = positions();
        for index in 153..=177u16 {
            assert!(owned.contains(&index), "{index} missing from the plan");
        }
        for index in 198..=234u16 {
            assert!(owned.contains(&index), "{index} missing from the plan");
        }
        for index in [152u16, 178, 197, 235] {
            assert!(!owned.contains(&index), "{index} is not a pattern position");
        }
    }

    /// The primitives, on a bar chosen so every quantity is a round number.
    #[test]
    fn the_shape_primitives_are_what_they_say() {
        // open 100, high 130, low 90, close 110
        let shape = Shape::of(&mk(0, 100, 130, 90, 110));
        assert_eq!(shape.body, 10);
        assert_eq!(shape.range, 40);
        assert_eq!(shape.upper, 20, "high - max(open, close)");
        assert_eq!(shape.lower, 10, "min(open, close) - low");
        assert!(shape.bullish() && !shape.bearish());
        assert_eq!(shape.top(), 110);
        assert_eq!(shape.bottom(), 100);
        // body 10 of range 40 is 250 permille: a small body, not a doji at 100.
        assert!(shape.body_at_most(250) && !shape.body_at_most(249));
    }

    /// Nothing fires with no history, and nothing with a single bar beyond the
    /// one-bar patterns.
    #[test]
    fn an_empty_detector_emits_nothing() {
        let p = Patterns::default();
        assert!(p.bits().is_empty());
        assert_eq!(p.depth(), 0);
    }

    /// A four-price doji is the only pattern defined on a zero-range bar.
    #[test]
    fn a_single_price_bar_is_a_four_price_doji_and_nothing_else() {
        let mut p = Patterns::default();
        let m = ok(&mut p, &at(0, 100, 100, 100, 100));
        assert!(m.get(225), "the four-price doji did not fire");
        assert_eq!(m.popcount(), 1, "a single-price bar lit something else too");
    }

    /// A hammer: small body at the top, long lower wick, no upper wick.
    #[test]
    fn a_hammer_fires_and_a_hanging_man_needs_a_prior_advance() {
        let hammer = at(1, 118, 120, 100, 120);
        let mut p = Patterns::default();
        let m = ok(&mut p, &hammer);
        assert!(m.get(153), "the hammer did not fire");
        assert!(!m.get(155), "a hanging man fired with no prior bar");

        // After an up bar, the same shape is also a hanging man.
        let mut q = Patterns::default();
        let _ = ok(&mut q, &at(0, 100, 112, 99, 110));
        let m2 = ok(&mut q, &hammer);
        assert!(
            m2.get(153) && m2.get(155),
            "the hanging man did not fire after an advance"
        );
    }

    /// Bullish engulfing: a down bar then an up bar whose body covers it.
    #[test]
    fn bullish_and_bearish_engulfing_are_not_transposed() {
        let mut p = Patterns::default();
        let _ = ok(&mut p, &at(0, 110, 112, 98, 100));
        let m = ok(&mut p, &at(1, 99, 116, 98, 114));
        assert!(m.get(157), "bullish engulfing did not fire");
        assert!(!m.get(158), "bearish engulfing fired on a bullish setup");

        let mut q = Patterns::default();
        let _ = ok(&mut q, &at(0, 100, 112, 98, 110));
        let m2 = ok(&mut q, &at(1, 112, 114, 96, 98));
        assert!(m2.get(158), "bearish engulfing did not fire");
        assert!(!m2.get(157));
    }

    /// **A pattern must not straddle an overnight gap.** Friday 15:29 and Monday
    /// 09:15 are not adjacent bars, and every multi-bar predicate assumes they are.
    #[test]
    fn a_new_session_clears_the_ring() {
        let mut p = Patterns::default();
        let _ = ok(&mut p, &at(0, 110, 112, 98, 100));
        assert_eq!(p.depth(), 1);
        let mut next_day = at(0, 99, 116, 98, 114);
        next_day.ts_micros += 1_440 * 60_000_000;
        let m = ok(&mut p, &next_day);
        assert_eq!(p.depth(), 1, "the ring survived a session boundary");
        assert!(
            !m.get(157),
            "an engulfing pattern was found across an overnight gap",
        );
    }

    /// Only the 62 positions this module owns are ever set, over a long walk.
    #[test]
    fn nothing_outside_the_sixty_two_positions_is_set() {
        let owned = positions();
        let mut p = Patterns::default();
        let mut union = ConditionMask::ZERO;
        let mut px = 2_500_000i64;
        for m in 0..600 {
            let drift = (m * 37) % 401 - 200;
            let open = px;
            let close = px + drift;
            let high = open.max(close) + (m * 13) % 90;
            let low = open.min(close) - (m * 17) % 90;
            px = close;
            union = union.union(&ok(&mut p, &at(m, open, high, low, close)));
        }
        for index in 0..ConditionMask::BITS {
            if union.get(index) {
                let as_u16 = u16::try_from(index).unwrap_or(u16::MAX);
                assert!(
                    owned.contains(&as_u16),
                    "position {index} is not this module's"
                );
            }
        }
    }

    /// A bullish and a bearish version of the same pattern can never both fire.
    #[test]
    fn opposite_polarity_patterns_never_co_occur() {
        let pairs = [
            (157u32, 158u32),
            (159, 160),
            (161, 162),
            (163, 164),
            (165, 166),
            (167, 168),
            (172, 173),
            (198, 199),
            (200, 201),
            (202, 203),
            (204, 205),
            (206, 207),
            (208, 209),
            (213, 214),
            (222, 223),
            (233, 234),
        ];
        let mut p = Patterns::default();
        let mut px = 2_500_000i64;
        for m in 0..900 {
            let drift = (m * 53) % 601 - 300;
            let open = px;
            let close = px + drift;
            let high = open.max(close) + (m * 29) % 120;
            let low = open.min(close) - (m * 31) % 120;
            px = close;
            let mask = ok(&mut p, &at(m % 370, open, high, low, close));
            for (bull, bear) in pairs {
                assert!(
                    !(mask.get(bull) && mask.get(bear)),
                    "positions {bull} and {bear} fired on the same bar at m={m}",
                );
            }
        }
    }

    /// A corrupt record is refused and folded into nothing.
    #[test]
    fn a_corrupt_bar_is_refused_and_not_folded() {
        let mut p = Patterns::default();
        assert_eq!(
            p.step(&at(0, 100, 90, 110, 105)),
            Err(crate::Corrupt::HighBelowLow)
        );
        assert_eq!(p.depth(), 0, "a refused bar was folded in");
        assert_eq!(
            p.step(&at(0, 0, i64::MAX, i64::MIN, 0)),
            Err(crate::Corrupt::RangeOverflows),
        );
        assert_eq!(p.depth(), 0);
    }

    /// Prices at the edge of the type do not overflow the cross-multiplied ratios.
    #[test]
    fn extreme_but_representable_prices_do_not_overflow() {
        let mut p = Patterns::default();
        let wide = i64::MAX / 4;
        let first = ok(&mut p, &at(0, 0, wide, -wide, wide / 2));
        let second = ok(&mut p, &at(1, wide / 2, wide, -wide, -wide / 2));
        let after = p.bits();

        // The three results used to be `let _ =`, which asserted nothing. An audit was
        // right that `bits() -> ConditionMask::ZERO` survives that — but it is worth being
        // precise about what the old form DID enforce, because it was not nothing: this
        // workspace sets `overflow-checks = true` in the release profile as well as debug
        // (verified by reading `cargo build --release -v`), so an overflowing multiply here
        // panics and fails the test. "No panic" is a real assertion about overflow.
        //
        // What was missing is any statement about the ANSWER. These bars are accepted, so
        // the family produces a mask, and the mask is what a sweep would see. Pinned:
        // MEASURED, not assumed. I first asserted this was empty -- "the first bar has no
        // predecessor, so no pattern can be decided on it" -- and the test refused it:
        // `[0, 0, 8796093022208, 0, 0, 0]`, which is position 171, `pat_spinning_top`. A
        // spinning top is a SINGLE-BAR pattern and needs no predecessor, so the family is
        // right and my expectation was wrong. That is the whole reason this assertion is
        // here rather than a `let _ =`: it makes the family state which patterns it can
        // decide on a bar with no history.
        let spinning_top = u32::from(
            (0..vocab::table::NEXT_FREE)
                .find(|i| vocab::table::name(*i) == Some("pat_spinning_top"))
                .unwrap_or(vocab::table::NEXT_FREE),
        );
        assert!(
            first.get(spinning_top),
            "at these extremes the first bar is a spinning top -- a single-bar pattern -- \
             and the family must say so even with no predecessor: {first:?}"
        );
        assert_eq!(
            second, after,
            "`bits()` must return what `step` just returned; a second call is not a second \
             answer"
        );
        // Every bit set must belong to this family. A free function, not a method --
        // `positions()` is module-level here.
        let owned: std::collections::BTreeSet<u16> = positions().into_iter().collect();
        for bit in 0..vocab::ConditionMask::BITS {
            if after.get(bit) {
                let index = u16::try_from(bit).unwrap_or(u16::MAX);
                assert!(
                    owned.contains(&index),
                    "position {index} is set and is not one of this family's 62"
                );
            }
        }
    }

    /// Same bars, same bits, byte for byte.
    #[test]
    fn the_same_bars_give_the_same_bits() {
        let run = || {
            let mut p = Patterns::default();
            let mut px = 2_500_000i64;
            (0..300)
                .map(|m| {
                    let drift = (m * 41) % 301 - 150;
                    let open = px;
                    let close = px + drift;
                    let high = open.max(close) + (m * 19) % 70;
                    let low = open.min(close) - (m * 23) % 70;
                    px = close;
                    ok(&mut p, &at(m, open, high, low, close)).words()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run(), "the same bars must give the same bits");
        // The equality above holds for ANY deterministic body -- including
        // `bits() -> ConditionMask::ZERO` and any constant -- so it cannot see the one
        // mutation that matters. An audit applied exactly that mutation and it survived
        // ALL SIX of these idempotence tests, which between them are §3 rule 5's only
        // direct guard in this crate.
        //
        // This closes it without needing to know what the bits SHOULD be, which is what
        // the sibling tests in this module are for. A body that ignores its input emits
        // the same value on every bar, and these fixtures deliberately vary the bars. So:
        // the run must not be constant.
        let observed = run();
        assert!(
            observed
                .iter()
                .skip(1)
                .zip(observed.iter())
                .any(|(later, earlier)| later != earlier),
            "every bar produced an identical result, so this test would pass on a body \
             that ignores its input entirely"
        );
    }

    /// The thresholds are one struct, and the default is the classical set.
    #[test]
    fn the_thresholds_are_gathered_and_defaulted() {
        let thr = Thresholds::default();
        assert_eq!(thr, Thresholds::CLASSICAL);
        assert_eq!(thr.doji_body, 100);
        assert_eq!(thr.long_body, 700);
        assert_eq!(thr.tiny_wick, 50);
        assert_eq!(thr.small_body, 300);
        assert_eq!(thr.long_wick_vs_body, 2000);
        // And a detector carries whatever it was given, so a future decision entry
        // can change the numbers without touching 62 predicates.
        let custom = Thresholds {
            doji_body: 50,
            ..Thresholds::CLASSICAL
        };
        assert_eq!(Patterns::new(custom).thresholds().doji_body, 50);
    }
}

// `.expect` for the reason spelled out above `mod tests`: an `unreachable!` here is a
// region `cargo llvm-cov` can never execute.
#[cfg(test)]
#[allow(clippy::expect_used)]
mod degenerate {
    use super::*;
    use Candle;

    fn bar(ts: i64, o: i64, h: i64, l: i64, c: i64) -> Candle {
        Candle {
            ts_micros: ts,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    /// THE BOUNDARY VALUE ITSELF, WHICH NO TEST HERE REACHED.
    ///
    /// `cargo-mutants` over this crate — its first run ever — left **49
    /// survivors, nine of them in this file**, and they share one shape: a
    /// comparison flipped by a single step, `>` to `>=` and `<` to `<=`. That is
    /// not nine defects, it is one gap. Every test exercised these predicates on
    /// values comfortably inside the range and never on the value that decides
    /// them.
    ///
    /// It matters here more than in most crates: this file turns a bar into
    /// condition bits. A comparison off by one step does not crash and does not
    /// look wrong — it silently reclassifies bars at the edge, changing which
    /// combinations the sweep finds and changing nothing a reader can see.
    ///
    /// # Four of the nine are equivalent and are not chased
    ///
    /// `Shape::top` and `Shape::bottom` are `if open > close { open } else
    /// { close }`. At `open == close` both arms return the same number, so `>`
    /// and `>=` compute the same function and no assertion can separate them.
    /// The two inside `Shape::of` have the same shape. Recorded at
    /// `docs/06-limits.md` §82 rather than hunted.
    #[test]
    fn a_doji_is_neither_bullish_nor_bearish() {
        // `open == close` is exactly where `>` and `>=` disagree, and the only
        // place they can: every other bar is decided identically by both.
        let doji = Shape::of(&bar(0, 100, 110, 90, 100));
        assert!(
            !doji.bullish(),
            "a bar that closed where it opened has not risen; `close >= open` \
             would call this bullish and it is not"
        );
        assert!(
            !doji.bearish(),
            "and it has not fallen either; `close <= open` would call it bearish"
        );

        // One paisa either side, so the predicates are shown to work at all
        // rather than merely to refuse.
        assert!(Shape::of(&bar(0, 100, 110, 90, 101)).bullish());
        assert!(Shape::of(&bar(0, 100, 110, 90, 99)).bearish());
    }

    /// `body_at_least` and `body_at_most` meet exactly at the threshold.
    ///
    /// Three mutants lived here: the function replaced by `true`, and the
    /// cross-multiply's `*` turned into `+` and into `/`. All three need a case
    /// whose answer is FALSE, which no test in this file had.
    #[test]
    fn the_body_thresholds_are_exact_at_the_boundary() {
        // range 100, body 30 — exactly 300 permille.
        let at = Shape::of(&bar(0, 100, 200, 100, 130));
        assert_eq!(at.range, 100, "the fixture's range is what the maths uses");
        assert_eq!(at.body, 30, "and its body");

        assert!(
            at.body_at_least(300),
            "a body of exactly 300 permille IS at least 300"
        );
        assert!(
            at.body_at_most(300),
            "and it is at most 300 — the two must meet, not overlap or gap"
        );
        assert!(
            !at.body_at_least(301),
            "301 permille is more than this body, so `>=` must refuse it — the \
             case a `-> true` replacement cannot survive"
        );
        assert!(!at.body_at_most(299), "and 299 is less than it");
    }

    /// A one-sided doji is not a high wave candle.
    ///
    /// This is the input that exposed the defect, and finding it took two attempts.
    /// The first version of this test used a four-price doji — which `is_doji`
    /// already excludes via `range > 0`, so the test passed without exercising the
    /// fix at all. A test that cannot fail is the §4 sin, so here is the real case:
    ///
    /// `open == close`, range 110, upper shadow 100 (91% of range), lower shadow 10
    /// (9%). Body is zero, so the original `upper * 1000 >= body * 3000` reduced to
    /// `100 >= 0` and `lower * 1000 >= body * 3000` to `10 >= 0` — both vacuously
    /// true. Position 227, "very long shadows on BOTH sides", fired on a
    /// shooting-star shape. Every `open == close` bar set it, whatever its shape.
    #[test]
    fn a_one_sided_doji_is_not_a_high_wave_candle() {
        let mut p = Patterns::new(Thresholds::default());
        let lopsided = bar(0, 2_500_000, 2_500_100, 2_499_990, 2_500_000);
        let mask = p.step(&lopsided).expect("a sane bar is not corrupt");
        assert!(
            !mask.get(227),
            "the lower shadow is 9% of range; 227 claims long shadows on BOTH sides"
        );
    }

    /// A four-price doji sets nothing in this family either.
    #[test]
    fn a_four_price_doji_is_not_a_high_wave_candle() {
        let mut p = Patterns::new(Thresholds::default());
        let flat = bar(0, 2_500_000, 2_500_000, 2_500_000, 2_500_000);
        let mask = p.step(&flat).expect("a zero-range bar is not corrupt");
        assert!(!mask.get(227), "no range means no shadows");
    }

    /// A symmetric zero-body doji with long shadows both sides IS a high wave.
    #[test]
    fn a_symmetric_zero_body_doji_with_long_shadows_is_a_high_wave() {
        let mut p = Patterns::new(Thresholds::default());
        let wave = bar(0, 2_500_000, 2_500_050, 2_499_950, 2_500_000);
        let mask = p.step(&wave).expect("a sane bar is not corrupt");
        assert!(
            mask.get(227),
            "50% shadow each side, zero body: this is the pattern"
        );
    }

    /// The bit still fires on a bar that genuinely has the shape.
    ///
    /// Without this half, the fix above could be "never set 227" and stay green.
    #[test]
    fn a_real_high_wave_candle_still_sets_the_bit() {
        let mut p = Patterns::new(Thresholds::default());
        // Tiny body at mid-range, long shadows either side.
        let wave = bar(0, 2_500_000, 2_530_000, 2_470_000, 2_500_100);
        let mask = p.step(&wave).expect("a sane bar is not corrupt");
        assert!(
            mask.get(227),
            "a small body with long shadows both sides IS a high wave"
        );
    }
}

// `.expect` for the reason spelled out above `mod tests`.
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tri_star {
    use super::*;

    fn doji(ts: i64, price: i64, wick: i64) -> Candle {
        Candle {
            ts_micros: ts,
            open: price,
            high: price + wick,
            low: price - wick,
            close: price,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    /// Three dojis closing at the same price is not a directional tri-star.
    ///
    /// The branch was `if bar0.close > bar2.close { 233 } else { 234 }`, so equal
    /// closes fell into the bearish bit — the identical defect shape as position 227
    /// firing on every zero-body bar: a degenerate case handed to one arm because
    /// `else` catches everything the condition does not.
    #[test]
    fn three_dojis_at_one_price_set_neither_direction() {
        let mut p = Patterns::new(Thresholds::CLASSICAL);
        for i in 0..3_i64 {
            let mask = p
                .step(&doji(i * 60_000_000, 2_500_000, 5_000))
                .expect("a doji is a sane bar");
            if i == 2 {
                assert!(!mask.get(233), "no bullish tri-star without a higher close");
                assert!(!mask.get(234), "no bearish tri-star without a LOWER close");
            }
        }
    }

    /// A genuinely bearish tri-star still sets 234.
    ///
    /// Without this half, the fix above could be "never set 234" and stay green.
    #[test]
    fn a_descending_tri_star_is_bearish() {
        let mut p = Patterns::new(Thresholds::CLASSICAL);
        let prices = [2_500_000_i64, 2_499_000, 2_498_000];
        let mut last = ConditionMask::ZERO;
        for (i, price) in (0_i64..).zip(prices.iter()) {
            let mask = p
                .step(&doji(i * 60_000_000, *price, 5_000))
                .expect("a doji is a sane bar");
            last = mask;
        }
        assert!(
            last.get(234),
            "the third doji closed lower: bearish tri-star"
        );
        assert!(!last.get(233));
    }

    /// And an ascending one still sets 233.
    #[test]
    fn an_ascending_tri_star_is_bullish() {
        let mut p = Patterns::new(Thresholds::CLASSICAL);
        let prices = [2_498_000_i64, 2_499_000, 2_500_000];
        let mut last = ConditionMask::ZERO;
        for (i, price) in (0_i64..).zip(prices.iter()) {
            let mask = p
                .step(&doji(i * 60_000_000, *price, 5_000))
                .expect("a doji is a sane bar");
            last = mask;
        }
        assert!(
            last.get(233),
            "the third doji closed higher: bullish tri-star"
        );
        assert!(!last.get(234));
    }
}

/// One exemplar per position, and the position that means the opposite.
///
/// # What was missing
///
/// Most of the 62 positions were never asked to fire. What the random walks in
/// `super::tests` assert is that nothing OUTSIDE the family fires and that opposite
/// polarities never fire together — and **both of those hold trivially for a
/// predicate that can never be true at all.** A clause with a reversed comparison,
/// a threshold measured against the wrong denominator, or a pair of `set` calls
/// written the wrong way round would have shipped green.
///
/// Every case below is read off one predicate, and asserts both halves: the bit
/// fires on bars built for it, and the bit that means the opposite stays dark. The
/// second half is what catches a transposition — 176 and 177, or 222 and 223, are
/// otherwise indistinguishable to a test that only ever asks "did something fire".
///
/// The case list is also checked for completeness against [`positions`], so a new
/// position cannot be added to the module without an exemplar.
///
/// `.expect` and not `let .. else { unreachable!() }`: the `unreachable!` form
/// expands to a panic written in THIS crate, so `cargo llvm-cov` records a region
/// no passing test can ever execute, and a region that cannot run is one nobody
/// can be held to. `Result::expect` panics inside the standard library, which is
/// not instrumented, and refuses just as loudly. The workspace denies
/// `expect_used` for library code, where a panic is a real defect; the same allow
/// sits on the test modules of `indicators::daily` and eleven others.
#[cfg(test)]
#[allow(clippy::expect_used)]
mod exemplars {
    use super::{Candle, ConditionMask, Patterns, positions};

    /// `(open, high, low, close)` in paisa. §7: integers, never a float.
    type Ohlc = (i64, i64, i64, i64);

    /// Minute `minute` of IST day 30,000 — the same clock `super::tests` uses, so
    /// every bar of a case lands in one session and none of them is dropped by the
    /// overnight-gap reset.
    fn at(minute: i64, (open, high, low, close): Ohlc) -> Candle {
        Candle {
            ts_micros: (30_000 * 1_440 + 555 + minute) * 60_000_000 - 19_800 * 1_000_000,
            open,
            high,
            low,
            close,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    /// Fold the bars oldest first and hand back the mask the newest one emits.
    ///
    /// Oldest first because that is the only order the ring accepts: a pattern is a
    /// statement about a run of bars, and `back(1)` must be the bar before
    /// `back(0)`.
    fn fold(bars: &[Ohlc]) -> ConditionMask {
        let mut detector = Patterns::default();
        let mut last = ConditionMask::ZERO;
        for (minute, ohlc) in (0_i64..).zip(bars) {
            last = detector
                .step(&at(minute, *ohlc))
                .expect("every exemplar bar is a sane record");
        }
        last
    }

    /// The bars, the position they must light, and one that must stay dark.
    struct Case {
        /// The position the bars must set.
        want: u16,
        /// The vocabulary's own name for `want`, asserted against the bit table so
        /// a renumbering cannot silently re-point an exemplar at another pattern.
        name: &'static str,
        /// A position these same bars must NOT set: the mirror-image pattern where
        /// there is one, and otherwise the neighbour whose predicate differs from
        /// `want`'s by the one clause this shape fails.
        dark: u16,
        /// Oldest bar first. The asserted mask is the newest bar's.
        bars: &'static [Ohlc],
    }

    /// One case per position, in position order.
    ///
    /// The prices are small round numbers so a reader can check a ratio by hand:
    /// a body of 20 in a range of 200 is 10%, which is the doji threshold, and a
    /// body of 100 in a range of 140 is 71%, which clears the long-body one.
    const CASES: &[Case] = &[
        Case {
            want: 153,
            name: "pat_hammer",
            dark: 154,
            // Body 20 at the top of a 200 range, lower wick 180, no upper wick.
            bars: &[(1180, 1200, 1000, 1200)],
        },
        Case {
            want: 154,
            name: "pat_inverted_hammer",
            dark: 153,
            bars: &[(1020, 1200, 1000, 1000)],
        },
        Case {
            want: 155,
            name: "pat_hanging_man",
            dark: 156,
            // The same hammer, after an up bar. Prior direction is the whole
            // difference between 153 and 155.
            bars: &[(1000, 1120, 990, 1100), (1180, 1200, 1000, 1200)],
        },
        Case {
            want: 156,
            name: "pat_shooting_star",
            dark: 155,
            bars: &[(1000, 1120, 990, 1100), (1020, 1200, 1000, 1000)],
        },
        Case {
            want: 157,
            name: "pat_bullish_engulfing",
            dark: 158,
            bars: &[(1100, 1110, 1000, 1010), (1000, 1120, 995, 1110)],
        },
        Case {
            want: 158,
            name: "pat_bearish_engulfing",
            dark: 157,
            bars: &[(1010, 1110, 1000, 1100), (1110, 1120, 1000, 1000)],
        },
        Case {
            want: 159,
            name: "pat_bullish_harami",
            dark: 160,
            bars: &[(1100, 1110, 990, 1000), (1020, 1090, 1010, 1080)],
        },
        Case {
            want: 160,
            name: "pat_bearish_harami",
            dark: 159,
            bars: &[(1000, 1110, 990, 1100), (1080, 1090, 1010, 1020)],
        },
        Case {
            want: 161,
            name: "pat_piercing_line",
            dark: 162,
            // Opens below the prior low, closes above the prior body's midpoint of
            // 1050 but below its open.
            bars: &[(1100, 1110, 990, 1000), (980, 1070, 975, 1060)],
        },
        Case {
            want: 162,
            name: "pat_dark_cloud_cover",
            dark: 161,
            bars: &[(1000, 1110, 990, 1100), (1120, 1130, 1030, 1040)],
        },
        Case {
            want: 163,
            name: "pat_morning_star",
            dark: 164,
            // Long black, small star, then a close above the first body's midpoint.
            bars: &[
                (1100, 1100, 1000, 1000),
                (990, 1030, 960, 995),
                (1000, 1070, 995, 1060),
            ],
        },
        Case {
            want: 164,
            name: "pat_evening_star",
            dark: 163,
            bars: &[
                (1000, 1100, 1000, 1100),
                (1110, 1140, 1070, 1105),
                (1100, 1105, 1030, 1040),
            ],
        },
        Case {
            want: 165,
            name: "pat_three_white_soldiers",
            dark: 166,
            // Bodies grow, so neither 231 nor 232 rides along.
            bars: &[
                (1000, 1060, 990, 1050),
                (1040, 1110, 1030, 1100),
                (1090, 1160, 1080, 1150),
            ],
        },
        Case {
            want: 166,
            name: "pat_three_black_crows",
            dark: 165,
            bars: &[
                (1100, 1110, 1040, 1050),
                (1060, 1070, 990, 1000),
                (1010, 1020, 940, 950),
            ],
        },
        Case {
            want: 167,
            name: "pat_three_inside_up",
            dark: 168,
            bars: &[
                (1100, 1110, 990, 1000),
                (1020, 1090, 1010, 1080),
                (1085, 1130, 1080, 1120),
            ],
        },
        Case {
            want: 168,
            name: "pat_three_inside_down",
            dark: 167,
            bars: &[
                (1000, 1110, 990, 1100),
                (1080, 1090, 1010, 1020),
                (1015, 1020, 970, 980),
            ],
        },
        Case {
            want: 169,
            name: "pat_tweezer_top",
            dark: 170,
            // Two highs at 1080, opposite bodies, and lows that differ.
            bars: &[(1000, 1080, 990, 1050), (1070, 1080, 1000, 1010)],
        },
        Case {
            want: 170,
            name: "pat_tweezer_bottom",
            dark: 169,
            bars: &[(1000, 1080, 990, 1050), (1070, 1075, 990, 1010)],
        },
        Case {
            want: 171,
            name: "pat_spinning_top",
            dark: 224,
            // Body 20 of a 100 range: small at 30%, and NOT a doji at 10%.
            bars: &[(1050, 1100, 1000, 1070)],
        },
        Case {
            want: 172,
            name: "pat_marubozu_bullish",
            dark: 173,
            bars: &[(1000, 1100, 1000, 1100)],
        },
        Case {
            want: 173,
            name: "pat_marubozu_bearish",
            dark: 172,
            bars: &[(1100, 1100, 1000, 1000)],
        },
        Case {
            want: 174,
            name: "pat_dragonfly_doji",
            dark: 175,
            bars: &[(1000, 1000, 900, 1000)],
        },
        Case {
            want: 175,
            name: "pat_gravestone_doji",
            dark: 174,
            bars: &[(1000, 1100, 1000, 1000)],
        },
        Case {
            want: 176,
            name: "pat_rising_three_methods",
            dark: 177,
            // Long white, three black inside it, then a close above the first.
            bars: &[
                (1000, 1100, 1000, 1100),
                (1090, 1095, 1050, 1060),
                (1060, 1065, 1030, 1040),
                (1040, 1045, 1020, 1025),
                (1030, 1130, 1025, 1120),
            ],
        },
        Case {
            want: 177,
            name: "pat_falling_three_methods",
            dark: 176,
            bars: &[
                (1100, 1100, 1000, 1000),
                (1010, 1050, 1005, 1040),
                (1040, 1070, 1035, 1060),
                (1060, 1080, 1055, 1075),
                (1070, 1075, 990, 995),
            ],
        },
        Case {
            want: 198,
            name: "pat_abandoned_baby_bull",
            dark: 199,
            // The doji gaps clear of both neighbours: high 970 below the first low
            // of 1000, and low 990 above the doji's high.
            bars: &[
                (1100, 1110, 1000, 1010),
                (960, 970, 950, 960),
                (1000, 1060, 990, 1050),
            ],
        },
        Case {
            want: 199,
            name: "pat_abandoned_baby_bear",
            dark: 198,
            bars: &[
                (1010, 1110, 1000, 1100),
                (1150, 1160, 1140, 1150),
                (1100, 1130, 1050, 1060),
            ],
        },
        Case {
            want: 200,
            name: "pat_three_line_strike_bull",
            dark: 201,
            bars: &[
                (1100, 1110, 1040, 1050),
                (1050, 1060, 990, 1000),
                (1000, 1010, 940, 950),
                (950, 1120, 945, 1110),
            ],
        },
        Case {
            want: 201,
            name: "pat_three_line_strike_bear",
            dark: 200,
            bars: &[
                (1050, 1110, 1040, 1100),
                (1100, 1160, 1090, 1150),
                (1150, 1210, 1140, 1200),
                (1200, 1210, 1040, 1045),
            ],
        },
        Case {
            want: 202,
            name: "pat_kicker_bull",
            dark: 203,
            bars: &[(1100, 1110, 1000, 1010), (1150, 1210, 1140, 1200)],
        },
        Case {
            want: 203,
            name: "pat_kicker_bear",
            dark: 202,
            bars: &[(1010, 1110, 1000, 1100), (950, 960, 890, 900)],
        },
        Case {
            want: 204,
            name: "pat_belt_hold_bull",
            dark: 205,
            // Body 100 of a 140 range is 71%, so long; no lower wick, and an upper
            // wick too big for the marubozu at 172.
            bars: &[(1000, 1140, 1000, 1100)],
        },
        Case {
            want: 205,
            name: "pat_belt_hold_bear",
            dark: 204,
            bars: &[(1100, 1100, 960, 1000)],
        },
        Case {
            want: 206,
            name: "pat_counterattack_bull",
            dark: 207,
            bars: &[(1100, 1110, 990, 1000), (950, 1010, 940, 1000)],
        },
        Case {
            want: 207,
            name: "pat_counterattack_bear",
            dark: 206,
            bars: &[(1000, 1110, 990, 1100), (1150, 1160, 1090, 1100)],
        },
        Case {
            want: 208,
            name: "pat_separating_lines_bull",
            dark: 209,
            bars: &[(1000, 1060, 990, 1050), (1000, 1080, 995, 1070)],
        },
        Case {
            want: 209,
            name: "pat_separating_lines_bear",
            dark: 208,
            bars: &[(1050, 1060, 990, 1000), (1050, 1055, 940, 950)],
        },
        Case {
            want: 210,
            name: "pat_on_neck",
            dark: 211,
            // Closes exactly AT the prior low, which is what separates 210 from the
            // in-neck at 211.
            bars: &[(1100, 1110, 1000, 1010), (990, 1005, 985, 1000)],
        },
        Case {
            want: 211,
            name: "pat_in_neck",
            dark: 210,
            // Closes just above that low, and no higher than the prior close.
            bars: &[(1100, 1110, 1000, 1010), (990, 1010, 985, 1005)],
        },
        Case {
            want: 212,
            name: "pat_thrusting",
            dark: 211,
            // Past the prior close, but short of the prior body's midpoint of 1055.
            bars: &[(1100, 1110, 1000, 1010), (990, 1030, 985, 1020)],
        },
        Case {
            want: 213,
            name: "pat_tasuki_gap_up",
            dark: 214,
            bars: &[
                (1000, 1060, 990, 1050),
                (1100, 1160, 1090, 1150),
                (1080, 1085, 1030, 1040),
            ],
        },
        Case {
            want: 214,
            name: "pat_tasuki_gap_down",
            dark: 213,
            bars: &[
                (1050, 1060, 990, 1000),
                (960, 970, 900, 910),
                (980, 1020, 975, 1010),
            ],
        },
        Case {
            want: 215,
            name: "pat_side_by_side_white",
            dark: 213,
            // Two identical white bodies after a gap up.
            bars: &[
                (1000, 1060, 990, 1050),
                (1100, 1160, 1090, 1150),
                (1100, 1160, 1090, 1150),
            ],
        },
        Case {
            want: 216,
            name: "pat_mat_hold",
            dark: 176,
            // The second bar is WHITE, which is what keeps 176 dark on these bars.
            bars: &[
                (1000, 1100, 1000, 1100),
                (1105, 1160, 1100, 1150),
                (1145, 1155, 1120, 1140),
                (1140, 1150, 1115, 1135),
                (1130, 1180, 1125, 1170),
            ],
        },
        Case {
            want: 217,
            name: "pat_stick_sandwich",
            dark: 229,
            // Black, white, black with the two blacks closing at 1010.
            bars: &[
                (1100, 1110, 1000, 1010),
                (1000, 1080, 995, 1070),
                (1060, 1065, 1005, 1010),
            ],
        },
        Case {
            want: 218,
            name: "pat_ladder_bottom",
            dark: 219,
            bars: &[
                (1200, 1210, 1140, 1150),
                (1150, 1160, 1090, 1100),
                (1100, 1110, 1040, 1050),
                (1050, 1060, 990, 1000),
                (1000, 1130, 995, 1120),
            ],
        },
        Case {
            want: 219,
            name: "pat_ladder_top",
            dark: 218,
            bars: &[
                (1150, 1210, 1140, 1200),
                (1200, 1260, 1190, 1250),
                (1250, 1310, 1240, 1300),
                (1300, 1360, 1290, 1350),
                (1350, 1355, 1230, 1235),
            ],
        },
        Case {
            want: 220,
            name: "pat_concealing_baby_swallow",
            dark: 200,
            // Four black bars, the third carrying an upper shadow of 30 and the
            // fourth reaching above its high. The fourth is black, which is what
            // keeps the three-line strike at 200 dark.
            bars: &[
                (1100, 1110, 1040, 1050),
                (1050, 1060, 990, 1000),
                (1000, 1030, 940, 950),
                (1020, 1040, 960, 970),
            ],
        },
        Case {
            want: 221,
            name: "pat_unique_three_river",
            dark: 163,
            // The middle bar's body is 80 of a 95 range — far too big for the small
            // star a morning star needs, which is the clause 163 fails here.
            bars: &[
                (1100, 1100, 1000, 1000),
                (990, 995, 900, 910),
                (900, 1010, 895, 1000),
            ],
        },
        Case {
            want: 222,
            name: "pat_breakaway_bull",
            dark: 223,
            bars: &[
                (1200, 1200, 1100, 1100),
                (1100, 1110, 1040, 1050),
                (1050, 1060, 990, 1000),
                (1000, 1010, 940, 950),
                (950, 1130, 945, 1120),
            ],
        },
        Case {
            want: 223,
            name: "pat_breakaway_bear",
            dark: 222,
            bars: &[
                (1100, 1200, 1100, 1200),
                (1200, 1260, 1190, 1250),
                (1250, 1310, 1240, 1300),
                (1300, 1360, 1290, 1350),
                (1350, 1355, 1180, 1185),
            ],
        },
        Case {
            want: 224,
            name: "pat_long_legged_doji",
            dark: 153,
            // A shadow of 100 either side of a zero body: neither is the tiny upper
            // wick a hammer needs.
            bars: &[(1000, 1100, 900, 1000)],
        },
        Case {
            want: 225,
            name: "pat_four_price_doji",
            dark: 224,
            bars: &[(1000, 1000, 1000, 1000)],
        },
        Case {
            want: 226,
            name: "pat_rickshaw_man",
            dark: 171,
            bars: &[(1000, 1100, 900, 1000)],
        },
        Case {
            want: 227,
            name: "pat_high_wave",
            dark: 171,
            // 50 of a 100 range on each side: half the range in shadow either way.
            bars: &[(1000, 1050, 950, 1000)],
        },
        Case {
            want: 228,
            name: "pat_homing_pigeon",
            dark: 208,
            // Two small bodies of 10 in a 90 range, and opens that differ.
            bars: &[(1000, 1050, 960, 1010), (1005, 1055, 965, 1015)],
        },
        Case {
            want: 229,
            name: "pat_matching_low",
            dark: 207,
            // Two black bars closing at 1010. Same polarity, which is what the
            // counterattack at 207 refuses.
            bars: &[(1100, 1110, 1000, 1010), (1050, 1060, 1005, 1010)],
        },
        Case {
            want: 230,
            name: "pat_identical_three_crows",
            dark: 165,
            bars: &[
                (1100, 1110, 1040, 1050),
                (1100, 1105, 990, 1000),
                (1100, 1102, 940, 950),
            ],
        },
        Case {
            want: 231,
            name: "pat_advance_block",
            dark: 232,
            // Bodies 60, 50, 45: shrinking. The last body is 45 of a 55 range, so
            // it is not the small body 232 needs.
            bars: &[
                (1000, 1070, 990, 1060),
                (1050, 1105, 1040, 1100),
                (1095, 1145, 1090, 1140),
            ],
        },
        Case {
            want: 232,
            name: "pat_deliberation",
            dark: 231,
            // Bodies 50, 110, 5: the second GROWS, so the advance block is dark.
            bars: &[
                (1000, 1060, 990, 1050),
                (1040, 1160, 1035, 1150),
                (1155, 1200, 1150, 1160),
            ],
        },
        Case {
            want: 233,
            name: "pat_tri_star_bull",
            dark: 234,
            bars: &[
                (1000, 1010, 990, 1000),
                (1010, 1020, 1000, 1010),
                (1020, 1030, 1010, 1020),
            ],
        },
        Case {
            want: 234,
            name: "pat_tri_star_bear",
            dark: 233,
            bars: &[
                (1020, 1030, 1010, 1020),
                (1010, 1020, 1000, 1010),
                (1000, 1010, 990, 1000),
            ],
        },
    ];

    /// Every position fires on bars built for it, and the position that means the
    /// opposite stays dark.
    ///
    /// Absent this, a predicate that can never be true — a reversed comparison, a
    /// ratio against the wrong denominator — passes every other test in the file,
    /// because none of them asks any position to fire.
    #[test]
    fn every_position_fires_on_its_own_exemplar_and_leaves_its_opposite_dark() {
        let mut seen = std::collections::BTreeSet::new();
        for case in CASES {
            let found = vocab::table::definition(case.want);
            assert!(
                found.is_some(),
                "position {} is not in the table",
                case.want
            );
            let def = found.expect("asserted to be Some on the line above");
            assert_eq!(
                def.name, case.name,
                "position {} is `{}` in the table, not `{}`",
                case.want, def.name, case.name,
            );
            let mask = fold(case.bars);
            // `count` is hoisted rather than written `case.bars.len()` inside the
            // message: a computed argument in an assert message is only evaluated when
            // the assertion FAILS, so `cargo llvm-cov` counts it as a region a passing
            // run can never close. Two regions leaked here and this is the one-line fix.
            let count = case.bars.len();
            assert!(
                mask.get(u32::from(case.want)),
                "position {} (`{}`) did not fire on the {count} bars built for it",
                case.want,
                case.name,
            );
            assert!(
                !mask.get(u32::from(case.dark)),
                "position {} (`{}`) also lit {}, which means the opposite",
                case.want,
                case.name,
                case.dark,
            );
            assert!(
                seen.insert(case.want),
                "position {} has two cases",
                case.want
            );
        }
        for index in positions() {
            assert!(
                seen.contains(&index),
                "position {index} has no exemplar, so nothing proves it can fire",
            );
        }
    }

    /// A long-legged doji whose body sits away from the middle is not a rickshaw
    /// man.
    ///
    /// 226 is the one position that lives inside another's branch, and the centring
    /// test is all that separates them. Without this the guard could be deleted and
    /// every long-legged doji would become a rickshaw man with 224's own case still
    /// green — the same shape of defect as 227 firing on every zero-body bar.
    #[test]
    fn a_long_legged_doji_with_an_off_centre_body_is_not_a_rickshaw_man() {
        // Zero body at 1080 in a 900..1100 range. The range midpoint is 1000, so the
        // body sits 80 above it: 40% of the range, against the 10% the guard allows.
        let mask = fold(&[(1080, 1100, 900, 1080)]);
        assert!(
            mask.get(224),
            "a zero body with a shadow either side is a long-legged doji"
        );
        assert!(
            !mask.get(226),
            "the body sits 80 off a 200 range: a rickshaw man's body is mid-range"
        );
    }
}
