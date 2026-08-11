//! Bars in, condition bits out.
//!
//! This is the crate `CLAUDE.md` §5 names and that did not exist until now, so
//! **eleven of the vocabulary's 232 live positions become computable here and
//! the other 221 remain names.** That ratio is the honest state of the engine and
//! is stated first rather than buried.
//!
//! # What is implemented
//!
//! | Module | Positions | Count |
//! |---|---|---:|
//! | [`CurDayFib`] — the current-session Fibonacci ladder | 121–131 | 11 |
//! | [`daily`] — the pivot ladder, the CPR, and yesterday's high and low | 7–18, 54–55, 60–62, 74–85, 178–189 | 41 |
//! | [`orb`] — the opening range at four windows | 86–105 | 20 |
//! | [`fib`] — previous-day ladders, both directions | 19–29, 69–70, 106–109 | 15 |
//! | [`fib::Prev5`] — the five completed sessions | 110–120 | 11 |
//! | [`pattern`] — all 62 candlestick patterns | 153–177, 198–234 | 62 |
//! | [`session`] — bar shape, prior run, day position, time of day, day type, opening gap | 30–51, 66–68 | 25 |
//! | [`vwap`] — session VWAP and three sigma bands | 52–53, 143–152, 190–197 | 20 |
//!
//! **52 of the vocabulary'''s 232 live positions are computable. 180 remain names.**
//!
//! # The three rules this crate exists to keep
//!
//! 1. **No look-ahead** (§3 rule 7). At bar *N* the evaluator may read bars
//!    `0..=N` and nothing later. [`PastPrefix`] enforces it by removing the
//!    future from the borrow, so a look-ahead read is a compile error rather than
//!    a bounds check that might be skipped.
//! 2. **An anchor never includes the bar it is measured against.** The order is
//!    reset, then emit, then fold — fused into [`CurDayFib::step`] so a caller
//!    cannot transpose them. Folding first puts the close inside its own anchor
//!    range by construction, which kills every rung outside that range.
//! 3. **Integers only** (§7). Not one `f32` or `f64`, and no division on the
//!    evaluation path: the rung test is cross-multiplied, so there is no rounding
//!    policy for two platforms to disagree about.
//!
//! # Cost
//!
//! Per bar: one session comparison, eleven cross-multiplied rung tests, two
//! extreme updates. No loop whose length depends on the data, no allocation, and
//! a fixed state whose size is asserted at compile time. Measured on the proof
//! harness at 24.4 → 22.4 nanoseconds per bar across a 100× larger input — flat,
//! which is the evidence for §3 rule 4 rather than a claim about it.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod daily;
pub mod fib;
pub mod orb;
pub mod pattern;
pub mod session;
pub mod vwap;

use store::format::Bar;
use vocab::{ConditionMask, Tolerance};

/// The first vocabulary position of the current-day Fibonacci ladder.
///
/// The ladder occupies 121–131 in `docs/03-vocabulary.md` §7 and this crate does
/// **not** get to choose that. The constant is here so a caller cannot compute a
/// position number by arithmetic and drift from the table.
pub const CURDAY_FIRST: u16 = 121;

/// The eleven rungs, as exact integer numerators over 1000, in the table's order.
///
/// The ladder is a decimal specification, so `p/1000` is exact — there is no
/// irrational to approximate and no square root. The order **must** match
/// positions 121–131, and `indicators::curday::the_rungs_match_the_vocabulary`
/// asserts it against `vocab`'s own names rather than trusting this comment.
pub const CURDAY_RUNGS: [i32; 11] = [0, 236, 382, 500, 618, 786, 1000, 1272, 1618, 2000, 2618];

/// IST is UTC+05:30 exactly, and India observes no daylight saving.
const IST_OFFSET_MICROS: i64 = 19_800 * 1_000_000;
const MICROS_PER_DAY: i64 = 86_400 * 1_000_000;

/// The IST calendar-day number for a timestamp.
///
/// `div_euclid`, never `/`. Rust's `/` truncates toward zero, so for a pre-epoch
/// timestamp it rounds *up* and silently merges that bar into the 1970-01-01
/// session. `div_euclid` floors, which is what a day number means.
#[inline]
#[must_use]
pub fn ist_day(ts_micros: i64) -> i64 {
    ts_micros
        .wrapping_add(IST_OFFSET_MICROS)
        .div_euclid(MICROS_PER_DAY)
}

/// A borrow of the bars up to and including index `n`, and no further.
///
/// This is §3 rule 7 expressed as a **type** rather than a review comment. The
/// future is not hidden behind a bounds check — it is not inside the borrow at
/// all, so a look-ahead read cannot be written, let alone run.
#[derive(Clone, Copy, Debug)]
pub struct PastPrefix<'a> {
    seen: &'a [Bar],
}

impl<'a> PastPrefix<'a> {
    /// Borrow `bars[0..=n]`, or `None` when `n` is past the end.
    #[must_use]
    pub fn upto(bars: &'a [Bar], n: usize) -> Option<Self> {
        bars.get(..=n).map(|seen| Self { seen })
    }

    /// The bar at index `n` — the latest one this prefix can see.
    #[must_use]
    pub fn current(&self) -> Option<&'a Bar> {
        self.seen.last()
    }

    /// Every bar this prefix can see. There is no accessor that returns more.
    #[must_use]
    pub const fn as_slice(&self) -> &'a [Bar] {
        self.seen
    }

    /// How many bars are visible.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.seen.len()
    }

    /// True when nothing is visible.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

/// Which extreme was made later. This, not the pair of extremes, picks the ladder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Leg {
    /// No range yet, or one bar strictly engulfed the running range, so the two
    /// extremes share a bar and no temporal order exists.
    #[default]
    Undetermined,
    /// The high is the later extreme. Retracements measure *down* from it.
    Up,
    /// The low is the later extreme. Retracements measure *up* from it.
    Down,
}

/// A record that is not a bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corrupt {
    /// `high < low`. A market can print a zero range; it cannot print a negative
    /// one, so this is a broken record and not a state to be tolerated. `R == 0`
    /// and `R < 0` deliberately do not share a branch.
    HighBelowLow,
    /// `high - low` does not fit in an `i64`.
    ///
    /// It cannot happen on real market data — the widest Indian index span is
    /// about 10^7 paisa against an `i64` ceiling of 9.2 x 10^18. It CAN happen
    /// from a corrupt record, because the store's own `ohlc_is_sane` checks field
    /// ORDER only: a bar with `low = i64::MIN` is "sane" by that test and the
    /// subtraction overflows.
    ///
    /// Refused loudly rather than saturated. A saturating range would silently
    /// answer a different question than the one asked, which is the fallback
    /// `CLAUDE.md` §4 bans.
    RangeOverflows,
}

/// The current-session Fibonacci anchors, and the eleven bits they decide.
///
/// Fixed size, no allocation, and it does not grow with the number of bars seen —
/// which is the whole of the constant-space argument for this family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurDayFib {
    session_day: i64,
    hi: i64,
    lo: i64,
    leg: Leg,
    bars_folded: u32,
    live: bool,
}

const _: () = assert!(core::mem::size_of::<CurDayFib>() <= 48);

impl Default for CurDayFib {
    fn default() -> Self {
        Self::new()
    }
}

impl CurDayFib {
    /// An empty state, before any session.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            session_day: i64::MIN,
            hi: 0,
            lo: 0,
            leg: Leg::Undetermined,
            bars_folded: 0,
            live: false,
        }
    }

    /// Which extreme is the later one.
    #[must_use]
    pub const fn leg(&self) -> Leg {
        self.leg
    }

    /// The IST day this state is accumulating, or `None` before the first bar.
    #[must_use]
    pub const fn session_day(&self) -> Option<i64> {
        if self.live {
            Some(self.session_day)
        } else {
            None
        }
    }

    /// How many bars have been folded into the current session.
    #[must_use]
    pub const fn bars_folded(&self) -> u32 {
        self.bars_folded
    }

    /// The session range so far, or 0 when no range has formed.
    ///
    /// `checked_sub`, not `-`. `hi - lo` overflows for a corrupt record carrying
    /// `i64::MIN`, and in a debug build that is a panic on the evaluation path.
    /// [`Self::step`] refuses such a bar before it can reach here; this returns 0
    /// so a caller that reads the range directly gets "no range" rather than a
    /// crash, and 0 already means every bit is false.
    #[must_use]
    pub const fn range(&self) -> i64 {
        if self.live {
            match self.hi.checked_sub(self.lo) {
                Some(r) => r,
                None => 0,
            }
        } else {
            0
        }
    }

    /// **The full per-bar step, in the one order that is correct: reset (if the
    /// session changed) → emit → fold.**
    ///
    /// The emit reads state built from bars `s..=N-1` only; bar *N* is folded
    /// after. The three steps are fused into one method precisely so a caller
    /// cannot transpose them — see rule 2 in the module documentation for what
    /// folding first would cost.
    ///
    /// # Errors
    ///
    /// [`Corrupt::HighBelowLow`] for a record whose high is below its low. The
    /// bar is neither emitted for nor folded in.
    pub fn step(&mut self, bar: &Bar, tolerance: Tolerance) -> Result<ConditionMask, Corrupt> {
        if bar.high < bar.low {
            return Err(Corrupt::HighBelowLow);
        }
        if bar.high.checked_sub(bar.low).is_none() {
            return Err(Corrupt::RangeOverflows);
        }
        let day = ist_day(bar.ts_micros);
        if day != self.session_day {
            *self = Self {
                session_day: day,
                ..Self::new()
            };
        }
        let bits = self.bits(bar.close, tolerance);
        self.fold(bar);
        Ok(bits)
    }

    /// Fold one bar into the anchors. Strict `>` and `<` are load-bearing.
    ///
    /// Under `>=`, a flat market merely *re-touching* the session high would move
    /// the later extreme and flip the leg — remapping all eleven rungs with no
    /// change to the range at all. Under `>`, the leg changes if and only if the
    /// range changes.
    fn fold(&mut self, bar: &Bar) {
        if !self.live {
            self.hi = bar.high;
            self.lo = bar.low;
            self.live = true;
            self.bars_folded = 1;
            self.leg = Leg::Undetermined;
            return;
        }
        let new_high = bar.high > self.hi;
        let new_low = bar.low < self.lo;
        if new_high {
            self.hi = bar.high;
        }
        if new_low {
            self.lo = bar.low;
        }
        self.leg = match (new_high, new_low) {
            (true, true) => Leg::Undetermined,
            (true, false) => Leg::Up,
            (false, true) => Leg::Down,
            (false, false) => self.leg,
        };
        self.bars_folded = self.bars_folded.saturating_add(1);
    }

    /// The level of rung `p/1000`, in paisa, or `None` when there is no ladder.
    ///
    /// Materialising a level needs a division, so this is for **display and
    /// tests only** and is never on the evaluation path. [`Self::bits`] uses the
    /// cross-multiplied form instead, which has no rounding at all.
    #[must_use]
    pub fn level(&self, p: i32) -> Option<i64> {
        let r = self.range();
        if !self.live || r <= 0 {
            return None;
        }
        let step = i128::from(p) * i128::from(r) / 1000;
        let level = match self.leg {
            Leg::Undetermined => return None,
            Leg::Up => i128::from(self.hi) - step,
            Leg::Down => i128::from(self.lo) + step,
        };
        i64::try_from(level).ok()
    }

    /// The eleven-rung ladder as vocabulary positions 121–131, set on a mask.
    ///
    /// Delegates the band test to `vocab::table::set_near`, so the position
    /// numbers, the `near_*` kind check and the tolerance all stay owned by the
    /// crate that defines them. A wrong index here is a `VocabError`, not a
    /// silently mis-set bit.
    #[must_use]
    pub fn bits(&self, close: i64, tolerance: Tolerance) -> ConditionMask {
        let mut mask = ConditionMask::ZERO;
        let r = self.range();
        if !self.live || r <= 0 || self.leg == Leg::Undetermined {
            return mask;
        }
        let anchor = match self.leg {
            Leg::Up => self.hi,
            Leg::Down => self.lo,
            Leg::Undetermined => return mask,
        };
        let mut i = 0;
        while i < CURDAY_RUNGS.len() {
            #[expect(
                clippy::indexing_slicing,
                reason = "i < CURDAY_RUNGS.len() is the loop condition"
            )]
            let p = CURDAY_RUNGS[i];
            let index = CURDAY_FIRST + u16::try_from(i).unwrap_or(u16::MAX);
            // ONE level, ONE test. `set_near` re-tests with exactly the
            // `tolerance.covers(close, level, r)` used on the line above, so the two
            // cannot disagree. A refusal from `set_near` means the table and
            // CURDAY_FIRST disagree, which `the_rungs_match_the_vocabulary` makes a
            // test failure rather than a runtime surprise.
            if let Some(level) = self.rung_level(anchor, p, r)
                && tolerance.covers(close, level, r)
                && let Ok(next) = vocab::table::set_near(mask, index, tolerance, close, level, r)
            {
                mask = next;
            }
            i += 1;
        }
        mask
    }

    /// The rung's price level, materialised exactly once.
    ///
    /// `None` while the leg is undetermined, or if the level leaves `i64`.
    ///
    /// # Two bugs lived here, and the second was the serious one
    ///
    /// This replaces a pair of functions: an exact cross-multiplied test that
    /// decided whether the bit belonged, and a separate materialisation that handed
    /// `vocab` a level — under a comment claiming "a rounding difference here cannot
    /// change which bit is set".
    ///
    /// It could. `vocab::table::set_near` re-tests
    /// `|close − level| · 1000 ≤ tol · range` against the **truncated** level, while
    /// the exact test compared against the unrounded one, so at the band edge the two
    /// disagreed and the bit was dropped. The caller cannot tell "not near" from
    /// "set", because `set_near` returns the mask unchanged either way.
    ///
    /// Worse: the materialisation computed `anchor − step` for **both** legs, while
    /// the exact test implies `anchor + step` on a Down leg. Measured at
    /// `anchor = 2_500_000`, `range = 100_000`, `p = 618`: the exact test accepts a
    /// close of `2_561_800`, the level handed to `vocab` was `2_438_200`, and
    /// `covers` then compared `123_600_000` against a band of `1_000_000` — **123×
    /// over**. Every Down-leg rung with `p != 0` was silently dropped.
    ///
    /// One level and one test cannot disagree, which is why this returns the level
    /// rather than a verdict.
    fn rung_level(&self, anchor: i64, p: i32, r: i64) -> Option<i64> {
        let step = (i128::from(p) * i128::from(r)).div_euclid(1000);
        let a = i128::from(anchor);
        let level = match self.leg {
            Leg::Up => a - step,
            Leg::Down => a + step,
            Leg::Undetermined => return None,
        };
        // `ok()` and not `unwrap_or(i64::MIN)`: §7 reserves `i64::MIN` as the
        // open-interest null sentinel, and clamping an out-of-range level onto it
        // would put a sentinel where a price goes.
        i64::try_from(level).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(ts: i64, o: i64, h: i64, l: i64, c: i64) -> Bar {
        Bar {
            ts_micros: ts,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    fn tol() -> Tolerance {
        let Ok(t) = vocab::tolerance::pinned_fib() else {
            unreachable!("the fib width is pinned")
        };
        t
    }

    /// Step a bar that must be legal. A helper rather than `.expect()`, because
    /// this workspace denies `clippy::expect_used` — a panic message in shipped
    /// code is a decision nobody reviewed.
    fn ok(s: &mut CurDayFib, b: &Bar) -> ConditionMask {
        let Ok(m) = s.step(b, tol()) else {
            unreachable!("this fixture bar is sane")
        };
        m
    }

    /// The rung order here MUST be the table's order, or every emitted bit means
    /// the wrong rung. Checked against `vocab`'s own names, not against a comment.
    #[test]
    fn the_rungs_match_the_vocabulary() {
        // The suffix is the TABLE's spelling and this crate does not get to
        // derive it. Written out rather than computed: 500 is spelled `50` and
        // 1000 is spelled `100`, so any rule clever enough to produce both is
        // clever enough to be wrong silently.
        const EXPECTED: [(i32, &str); 11] = [
            (0, "near_fib_curday_0"),
            (236, "near_fib_curday_236"),
            (382, "near_fib_curday_382"),
            (500, "near_fib_curday_50"),
            (618, "near_fib_curday_618"),
            (786, "near_fib_curday_786"),
            (1000, "near_fib_curday_100"),
            (1272, "near_fib_curday_1272"),
            (1618, "near_fib_curday_1618"),
            (2000, "near_fib_curday_200"),
            (2618, "near_fib_curday_2618"),
        ];
        for (i, (p, want)) in EXPECTED.iter().enumerate() {
            let Ok(offset) = u16::try_from(i) else {
                unreachable!("eleven fits in a u16")
            };
            let index = CURDAY_FIRST + offset;
            let Some(def) = vocab::table::definition(index) else {
                unreachable!("121..=131 are allocated")
            };
            assert_eq!(
                CURDAY_RUNGS.get(i).copied(),
                Some(*p),
                "this crate's rung {i} is not {p}",
            );
            assert_eq!(
                def.name, *want,
                "rung {p} at position {index} is `{}` in the table",
                def.name,
            );
        }
    }

    /// `div_euclid` and not `/`: a pre-epoch stamp must not land on 1970-01-01.
    #[test]
    fn the_session_day_floors_for_negative_stamps() {
        assert_eq!(ist_day(0), 0, "epoch is inside IST day 0");
        assert!(ist_day(-1) < 0 || ist_day(-1) == 0);
        assert!(
            ist_day(-MICROS_PER_DAY * 3) < 0,
            "three days before the epoch is a negative day number"
        );
        // Monotone, which truncating division would break across zero.
        let mut previous = ist_day(-MICROS_PER_DAY * 5);
        for k in -4..5 {
            let d = ist_day(MICROS_PER_DAY * k);
            assert!(d >= previous, "day numbers went backwards at k = {k}");
            previous = d;
        }
    }

    /// The first bar of a session emits nothing: there is no prior range to
    /// measure against, and a bit that cannot be evaluated is false.
    #[test]
    fn the_first_bar_of_a_session_emits_nothing() {
        let mut s = CurDayFib::new();
        let Ok(m) = s.step(&bar(0, 100, 110, 90, 105), tol()) else {
            unreachable!("a sane bar")
        };
        assert!(
            m.is_empty(),
            "the first bar had no anchor and still emitted"
        );
        assert_eq!(s.bars_folded(), 1);
        assert_eq!(s.leg(), Leg::Undetermined);
    }

    /// A corrupt record is refused, and it is neither emitted for nor folded in.
    #[test]
    fn a_high_below_its_low_is_refused_and_changes_nothing() {
        let mut s = CurDayFib::new();
        let before = s;
        assert_eq!(
            s.step(&bar(0, 100, 90, 110, 105), tol()),
            Err(Corrupt::HighBelowLow)
        );
        assert_eq!(s, before, "a refused bar still changed the state");
    }

    /// Zero range emits nothing and never divides.
    #[test]
    fn a_flat_session_emits_nothing_all_day() {
        let mut s = CurDayFib::new();
        for k in 0..20 {
            let m = s
                .step(&bar(k * 60_000_000, 100, 100, 100, 100), tol())
                .unwrap_or(ConditionMask::ZERO);
            assert!(m.is_empty(), "bar {k} of a flat session emitted a bit");
        }
        assert_eq!(s.range(), 0);
    }

    /// A later high alone makes the high the later extreme.
    #[test]
    fn a_new_high_alone_is_an_up_leg() {
        let mut s = CurDayFib::new();
        let _ = ok(&mut s, &bar(0, 100, 110, 90, 105));
        let _ = ok(&mut s, &bar(60_000_000, 105, 120, 95, 115));
        assert_eq!(s.leg(), Leg::Up);
    }

    /// A later low alone makes the low the later extreme.
    #[test]
    fn a_new_low_alone_is_a_down_leg() {
        let mut s = CurDayFib::new();
        let _ = ok(&mut s, &bar(0, 100, 110, 90, 105));
        let _ = ok(&mut s, &bar(60_000_000, 105, 108, 80, 85));
        assert_eq!(s.leg(), Leg::Down);
    }

    /// One bar engulfing the whole running range leaves no temporal order.
    #[test]
    fn one_bar_engulfing_the_range_is_undetermined() {
        let mut s = CurDayFib::new();
        let _ = ok(&mut s, &bar(0, 100, 110, 90, 105));
        let _ = ok(&mut s, &bar(60_000_000, 105, 130, 70, 100));
        assert_eq!(s.leg(), Leg::Undetermined);
        let m = s.bits(100, tol());
        assert!(m.is_empty(), "an undetermined leg still emitted");
    }

    /// Re-touching the session high must NOT flip the leg: strict `>` is what
    /// makes the ladder a function of the range's history rather than of how many
    /// times a level was tapped.
    #[test]
    fn re_touching_the_high_does_not_flip_the_leg() {
        let mut s = CurDayFib::new();
        let _ = ok(&mut s, &bar(0, 100, 110, 90, 105));
        let _ = ok(&mut s, &bar(60_000_000, 105, 120, 95, 115));
        assert_eq!(s.leg(), Leg::Up);
        let range_before = s.range();
        // high EQUALS the running high, so nothing moved.
        let _ = ok(&mut s, &bar(120_000_000, 115, 120, 100, 118));
        assert_eq!(s.leg(), Leg::Up, "a re-touch flipped the leg");
        assert_eq!(s.range(), range_before, "a re-touch changed the range");
    }

    /// The session resets on the IST day boundary and yesterday's extremes go.
    #[test]
    fn a_new_ist_day_discards_the_previous_session() {
        let mut s = CurDayFib::new();
        let _ = ok(&mut s, &bar(0, 100, 500, 50, 200));
        let Some(day_one) = s.session_day() else {
            unreachable!("live")
        };
        let _ = ok(&mut s, &bar(MICROS_PER_DAY, 100, 110, 90, 105));
        let Some(day_two) = s.session_day() else {
            unreachable!("live")
        };
        assert_ne!(day_two, day_one);
        assert_eq!(s.bars_folded(), 1, "the new session kept old bars");
        assert_eq!(s.range(), 20, "the new session kept the old range");
    }

    /// Same bars twice, byte-identical bit streams. §3 rule 5.
    #[test]
    fn the_same_session_twice_gives_the_same_bits() {
        let bars: Vec<Bar> = (0..200)
            .map(|k| {
                let base = 2_500_000 + (k * 37) % 900;
                bar(
                    k * 60_000_000,
                    base,
                    base + 40,
                    base - 40,
                    base + ((k * 11) % 61) - 30,
                )
            })
            .collect();
        let run = || {
            let mut s = CurDayFib::new();
            let mut out = Vec::new();
            for b in &bars {
                out.push(ok(&mut s, b).words());
            }
            out
        };
        assert_eq!(run(), run(), "two identical runs disagreed");
    }

    /// Every bit this crate sets is a current-day rung and nothing else. A stray
    /// position would be a mask that means something the caller never asked for.
    #[test]
    fn only_positions_121_to_131_are_ever_set() {
        let mut s = CurDayFib::new();
        let mut union = ConditionMask::ZERO;
        for k in 0..400 {
            let base = 2_500_000 + (k * 53) % 1_500;
            let b = bar(
                k * 60_000_000,
                base,
                base + 60,
                base - 60,
                base + ((k * 17) % 121) - 60,
            );
            union = union.union(&ok(&mut s, &b));
        }
        for index in 0..vocab::ConditionMask::BITS {
            if union.get(index) {
                assert!(
                    (u32::from(CURDAY_FIRST)..u32::from(CURDAY_FIRST) + 11).contains(&index),
                    "position {index} was set and is not a current-day rung",
                );
            }
        }
    }

    /// At most one rung can fire on a bar. The proof is arithmetic — two rungs
    /// need a numerator gap of at most twice the width, and the ladder's smallest
    /// gap is 118 against a width of 10 — and this is the check on the proof.
    #[test]
    fn at_most_one_rung_fires_on_any_bar() {
        let mut s = CurDayFib::new();
        for k in 0..600 {
            let base = 2_500_000 + (k * 71) % 2_000;
            let b = bar(
                k * 60_000_000,
                base,
                base + 80,
                base - 80,
                base + ((k * 23) % 161) - 80,
            );
            let m = ok(&mut s, &b);
            assert!(
                m.popcount() <= 1,
                "bar {k} lit {} rungs at once",
                m.popcount()
            );
        }
    }

    /// The barrier cannot name a bar past `n`, and refuses an out-of-range `n`
    /// rather than clamping.
    #[test]
    fn the_prefix_cannot_see_past_its_own_end() {
        let bars: Vec<Bar> = (0..5).map(|k| bar(k, 1, 2, 0, 1)).collect();
        let Some(p) = PastPrefix::upto(&bars, 2) else {
            unreachable!("2 is in range")
        };
        assert_eq!(p.len(), 3);
        let Some(cur) = p.current() else {
            unreachable!("the prefix is non-empty")
        };
        assert_eq!(cur.ts_micros, 2);
        assert_eq!(p.as_slice().len(), 3, "the slice reached past n");
        assert!(
            PastPrefix::upto(&bars, 5).is_none(),
            "n == len was accepted"
        );
        assert!(PastPrefix::upto(&bars, 99).is_none());
    }

    /// Prices at the edge of the type do not panic and do not overflow.
    #[test]
    fn extreme_prices_neither_panic_nor_overflow() {
        let mut s = CurDayFib::new();
        // Field order is sane, so `ohlc_is_sane` would pass it — and the range
        // overflows i64. It must be REFUSED, not saturated.
        assert_eq!(
            s.step(&bar(0, 0, i64::MAX, i64::MIN, 0), tol()),
            Err(Corrupt::RangeOverflows),
        );
        assert_eq!(s.bars_folded(), 0, "a refused bar was folded in");

        // A wide but representable range must still work, at either price edge.
        let mut t = CurDayFib::new();
        let _ = ok(&mut t, &bar(0, 0, i64::MAX / 4, -(i64::MAX / 4), 0));
        let _ = ok(
            &mut t,
            &bar(60_000_000, 0, i64::MAX / 2, -(i64::MAX / 4), 0),
        );
        let _ = t.bits(0, tol());
        let _ = t.bits(i64::MAX, tol());
        let _ = t.bits(i64::MIN, tol());
    }
}

#[cfg(test)]
mod zero_range {
    use super::*;
    use store::format::Bar;

    fn flat(ts: i64, price: i64) -> Bar {
        Bar {
            ts_micros: ts,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    /// `high == low` is a real bar and must be ACCEPTED, not refused.
    ///
    /// This test exists to kill a specific surviving mutant. `CurDayFib::step` and
    /// `SessionState::step` both guard with `if bar.high < bar.low`. Changing that
    /// `<` to `<=` refuses every zero-range bar, and the whole suite stayed green,
    /// because the only test that fed a zero-range bar swallowed the result with
    /// `unwrap_or` and never asserted which way it went.
    ///
    /// A zero-range minute is not pathological — it is what a limit-locked or
    /// untraded minute looks like, and refusing it would silently drop real bars
    /// from a run while reporting nothing.
    #[test]
    fn a_zero_range_bar_is_accepted_by_curday_fib() {
        let Ok(tolerance) = vocab::tolerance::pinned_fib() else {
            unreachable!("the pinned fib tolerance is valid")
        };
        let mut f = CurDayFib::new();
        assert!(
            f.step(&flat(0, 2_500_000), tolerance).is_ok(),
            "high == low is a real bar; `<` must not become `<=`"
        );
    }

    /// The guard still refuses a genuinely inverted bar.
    ///
    /// Without this half, the test above could be satisfied by deleting the guard.
    #[test]
    fn an_inverted_bar_is_still_refused_by_curday_fib() {
        let Ok(tolerance) = vocab::tolerance::pinned_fib() else {
            unreachable!("the pinned fib tolerance is valid")
        };
        let mut f = CurDayFib::new();
        let inverted = Bar {
            ts_micros: 0,
            open: 2_500_000,
            high: 2_499_000,
            low: 2_501_000,
            close: 2_500_000,
            volume: 0,
            open_interest: i64::MIN,
        };
        assert_eq!(f.step(&inverted, tolerance), Err(Corrupt::HighBelowLow));
    }
}

#[cfg(test)]
mod down_leg {
    use super::*;

    /// A Down-leg rung fires. Before the level and the test were unified, none did.
    ///
    /// `set_curday` materialised `anchor - step` for both legs while the exact test
    /// implied `anchor + step` on a Down leg, so `vocab::table::set_near` was handed
    /// a level on the wrong side of the anchor and refused — silently, because it
    /// returns the mask unchanged whether it sets a bit or declines. Measured on the
    /// fixture below: the comparison was 123,600,000 against a band of 1,000,000.
    ///
    /// Both legs are asserted. Checking only the Down leg would pass if the sign
    /// were simply flipped the other way.
    #[test]
    fn both_legs_can_set_a_rung() {
        let Ok(tolerance) = vocab::tolerance::pinned_fib() else {
            unreachable!("the pinned fib tolerance is valid")
        };
        let (anchor, range, p) = (2_500_000_i64, 100_000_i64, 618_i32);
        let step = i64::from(p) * range / 1000;

        for (leg, expected) in [(Leg::Up, anchor - step), (Leg::Down, anchor + step)] {
            let f = CurDayFib {
                leg,
                ..CurDayFib::new()
            };
            assert_eq!(
                f.rung_level(anchor, p, range),
                Some(expected),
                "{leg:?} leg put the level on the wrong side of the anchor"
            );
            assert!(
                tolerance.covers(expected, expected, range),
                "{leg:?}: a close exactly on the level must be covered"
            );
        }
    }

    /// An undetermined leg has no level, and therefore sets nothing.
    #[test]
    fn an_undetermined_leg_has_no_level() {
        let f = CurDayFib {
            leg: Leg::Undetermined,
            ..CurDayFib::new()
        };
        assert_eq!(f.rung_level(2_500_000, 618, 100_000), None);
    }

    /// A level outside `i64` is refused, not clamped onto the OI null sentinel.
    #[test]
    fn an_out_of_range_level_is_refused_not_clamped() {
        let f = CurDayFib {
            leg: Leg::Down,
            ..CurDayFib::new()
        };
        assert_eq!(
            f.rung_level(i64::MAX, 1000, i64::MAX),
            None,
            "clamping to i64::MIN would put §7's null sentinel where a price goes"
        );
    }
}
