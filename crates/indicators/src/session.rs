//! Candle shape, prior-bar sequence, position within the day, time of day, day type
//! and the opening gap.
//!
//! **25 vocabulary positions**: 30–51 and 66–68.
//!
//! # These are DESCRIPTIONS, not anchors, and the fold order is inverted
//!
//! Every other family in this crate emits before folding, because a reference
//! price must not include the bar measured against it. This one is the opposite:
//! `close_at_day_high` is a statement about *where this bar sits*, and excluding
//! the bar from the day's high would make it unable to fire on the bar that set
//! the high — which is the only bar it should ever fire on.
//!
//! That split is the rule D-0080's block comment records: an **anchor** never
//! includes the bar it measures; a **description** always does.
//!
//! # Two sets of thresholds nobody wrote down
//!
//! `docs/06-limits.md` records ten sessions wholly or partly outside 09:15–15:29,
//! and no tracked document says where "mid-morning" ends or how large a "large
//! body" is. Both sets are gathered into named structs, labelled **UNVERIFIED**,
//! for the same reason `pattern::Thresholds` is: one place to audit, one edit to
//! change.
//!
//! # Cost
//!
//! Twenty-five predicates over the current bar, two prior bars, the day's running
//! extremes and yesterday's close. Fixed state, no allocation, no division on the
//! deciding path.

use crate::Candle;
use core::cmp::Ordering;
use vocab::{ConditionMask, Tolerance};

/// Where the four time-of-day windows end, in minutes after the 09:15 open.
///
/// **UNVERIFIED.** Positions 44–47 are `early_morning`, `mid_morning`, `midday`
/// and `afternoon`, and no tracked document says where any boundary is. These
/// split a 375-minute session into four roughly equal parts on the hour, which is
/// a convention and not a measurement.
///
/// The windows are half-open and exhaustive, so **exactly one** of the four holds
/// on every in-session bar — asserted by
/// `session::exactly_one_time_of_day_window_holds`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayWindows {
    /// `early_morning` covers `[0, this)`. 60 = until 10:15.
    pub early_ends: i64,
    /// `mid_morning` covers `[early_ends, this)`. 165 = until 12:00.
    pub mid_ends: i64,
    /// `midday` covers `[mid_ends, this)`. 285 = until 14:00.
    pub midday_ends: i64,
    /// `afternoon` covers `[midday_ends, this)`. 375 = until 15:30.
    pub session_ends: i64,
}

impl Default for DayWindows {
    fn default() -> Self {
        Self::CLASSICAL
    }
}

impl DayWindows {
    /// Four windows on the hour. **UNVERIFIED** — see the struct documentation.
    pub const CLASSICAL: Self = Self {
        early_ends: 60,
        mid_ends: 165,
        midday_ends: 285,
        session_ends: 375,
    };
}

/// How large a body must be to be "large", and how long a wick must be to be
/// "long", in thousandths of the bar's range.
///
/// **UNVERIFIED**, for the same reason [`crate::pattern::Thresholds`] is. Deliberately
/// a separate struct: positions 33–36 are the shipped vocabulary's own
/// `bar_large_body` and `long_upper_wick`, and tying them to the candlestick
/// thresholds would make one decision change two families silently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeLimits {
    /// A body at or above this fraction of the range is large. 600 = 60%.
    pub large_body: i64,
    /// A body at or below this fraction of the range is small. 300 = 30%.
    pub small_body: i64,
    /// A body at or below this fraction of the range is a doji. 100 = 10%.
    pub doji_body: i64,
    /// A wick at or above this fraction of the range is long. 400 = 40%.
    pub long_wick: i64,
}

impl Default for ShapeLimits {
    fn default() -> Self {
        Self::CLASSICAL
    }
}

impl ShapeLimits {
    /// **UNVERIFIED** — see the struct documentation.
    pub const CLASSICAL: Self = Self {
        large_body: 600,
        small_body: 300,
        doji_body: 100,
        long_wick: 400,
    };
}

/// How many prior bars `prior_n_bullish` and friends look back.
///
/// **UNVERIFIED.** The shipped names say `prior_n_*` and no document says what `n`
/// is. Three is the conventional reading of a "run".
pub const PRIOR_N: usize = 3;

/// Everything the previous completed session fixes that this module needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviousSession {
    /// Yesterday's high.
    pub high: i64,
    /// Yesterday's low.
    pub low: i64,
    /// Yesterday's close — the reference the opening gap is measured from.
    pub close: i64,
}

/// The running state for one session.
#[derive(Clone, Copy, Debug)]
pub struct SessionState {
    session_day: i64,
    live: bool,
    /// The day's running extremes, **including** the newest bar.
    day_high: i64,
    day_low: i64,
    /// Today's opening price, fixed by the first bar.
    day_open: i64,
    /// The last [`PRIOR_N`] bar directions, newest first. `None` where absent.
    ///
    /// An [`Ordering`] and not a `bool`, because `close` against `open` has three
    /// answers and a `bool` has two. The ring stored `close > open`, which filed a
    /// flat bar under `false` alongside every genuine down bar: 38 then reported a
    /// bearish run over bars on which 31 had never fired, and the run 38 names was
    /// expressible by no live position. Positions 30 and 31 have always asked the
    /// three-valued question of the current bar; this asks it of the prior ones.
    prior: [Option<Ordering>; PRIOR_N],
    windows: DayWindows,
    limits: ShapeLimits,
}

const _: () = assert!(core::mem::size_of::<SessionState>() <= 128);

impl Default for SessionState {
    fn default() -> Self {
        Self::new(DayWindows::CLASSICAL, ShapeLimits::CLASSICAL)
    }
}

impl SessionState {
    /// A new state.
    #[must_use]
    pub const fn new(windows: DayWindows, limits: ShapeLimits) -> Self {
        Self {
            session_day: i64::MIN,
            live: false,
            day_high: 0,
            day_low: 0,
            day_open: 0,
            prior: [None; PRIOR_N],
            windows,
            limits,
        }
    }

    /// The day's running extremes, including the newest bar. `None` before the
    /// first bar of a session.
    #[must_use]
    pub const fn day_extremes(&self) -> Option<(i64, i64)> {
        if self.live {
            Some((self.day_high, self.day_low))
        } else {
            None
        }
    }

    /// Today's opening price. `None` before the first bar.
    #[must_use]
    pub const fn day_open(&self) -> Option<i64> {
        if self.live { Some(self.day_open) } else { None }
    }

    /// **Fold, then emit** — see the module documentation for why this family is
    /// the opposite of the anchor families.
    ///
    /// # Errors
    ///
    /// [`crate::Corrupt`] for a record that is not a bar. It is neither folded nor
    /// emitted for.
    pub fn step(
        &mut self,
        bar: &Candle,
        previous: Option<PreviousSession>,
        tolerance: Tolerance,
    ) -> Result<ConditionMask, crate::Corrupt> {
        bar.check()?;
        let day = crate::ist_day(bar.ts_micros);
        if day != self.session_day || !self.live {
            self.session_day = day;
            self.live = true;
            self.day_high = bar.high;
            self.day_low = bar.low;
            self.day_open = bar.open;
            self.prior = [None; PRIOR_N];
        } else {
            if bar.high > self.day_high {
                self.day_high = bar.high;
            }
            if bar.low < self.day_low {
                self.day_low = bar.low;
            }
        }
        let bits = self.bits(bar, previous, tolerance);
        // The direction ring is the one thing that must NOT include this bar: it
        // answers "were the bars BEFORE this one bullish", so it is folded after.
        // Rotate right by one and put this bar's direction at the front. `rotate_
        // right` rather than an index loop: `clippy::needless_range_loop` and
        // `clippy::indexing_slicing` are both denied, and both are right that an
        // index into a fixed array is a panic the compiler cannot rule out.
        self.prior.rotate_right(1);
        // `let [newest, ..]` and not `first_mut()`: `prior` is an ARRAY of known
        // length, so a fixed-length pattern is irrefutable and compiles to no branch
        // at all. `first_mut()` handed back an `Option` whose `None` arm a
        // three-element array makes impossible — an arm no input can reach, and
        // therefore one no test can cover. A `PRIOR_N` of zero would be a compile
        // error here rather than a silent no-op, which is the stronger guard.
        let [newest, ..] = &mut self.prior;
        // `cmp` and not `close > open`: a flat bar is a legal bar — `check` refuses
        // only `high < low` and a range that overflows — and a boolean direction has
        // nowhere to put it but the down side.
        *newest = Some(bar.close.cmp(&bar.open));
        Ok(bits)
    }

    /// The 25 positions for one bar.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "six named position groups in one table is the readable form; \
                  splitting them hides which group sets which position"
    )]
    pub fn bits(
        &self,
        bar: &Candle,
        previous: Option<PreviousSession>,
        tolerance: Tolerance,
    ) -> ConditionMask {
        let mut mask = ConditionMask::ZERO;
        let open = i128::from(bar.open);
        let close = i128::from(bar.close);
        let high = i128::from(bar.high);
        let low = i128::from(bar.low);
        let body = (close - open).abs();
        let range = high - low;
        let top = if open > close { open } else { close };
        let bottom = if open < close { open } else { close };

        // ---- 30–36: bar shape ------------------------------------------------
        if close > open {
            mask = set(mask, 30);
        }
        if close < open {
            mask = set(mask, 31);
        }
        if range > 0 {
            let at_most = |permille: i64| body * 1000 <= range * i128::from(permille);
            let at_least = |permille: i64| body * 1000 >= range * i128::from(permille);
            if at_most(self.limits.doji_body) {
                mask = set(mask, 32);
            }
            if at_least(self.limits.large_body) {
                mask = set(mask, 33);
            }
            if at_most(self.limits.small_body) {
                mask = set(mask, 34);
            }
            if (high - top) * 1000 >= range * i128::from(self.limits.long_wick) {
                mask = set(mask, 35);
            }
            if (bottom - low) * 1000 >= range * i128::from(self.limits.long_wick) {
                mask = set(mask, 36);
            }
        }

        // ---- 37–39: prior-bar sequence ---------------------------------------
        // Iterated rather than indexed: `clippy::needless_range_loop` and
        // `clippy::indexing_slicing` are both denied, and iterating the array is
        // also the form that cannot go out of step with PRIOR_N.
        //
        // Each slot is compared against `Some(direction)` rather than unwrapped
        // behind a "the ring is full" guard. That guard needed a placeholder for the
        // absent slots it had already ruled out, and the placeholder it chose —
        // `false` — was the same value a down bar carried, so the arm no input could
        // reach decided the answer if it ever were reached. An absent slot is unequal
        // to every direction, so an incomplete ring cannot read as a run at all.
        let up = Some(Ordering::Greater);
        let down = Some(Ordering::Less);
        if self.prior.iter().all(|d| *d == up) {
            mask = set(mask, 37);
        }
        if self.prior.iter().all(|d| *d == down) {
            mask = set(mask, 38);
        }
        // Strictly alternating: every adjacent pair is one up bar and one down bar.
        // A flat bar breaks the alternation instead of counting as "different from its
        // neighbour" — the same rule that keeps it out of 37 and 38, and the reason
        // the first half of this is not redundant. Without it `Some(Equal)` beside
        // `Some(Greater)` differs, so up-flat-up reported an alternation over a run
        // containing no down bar.
        let directional = self.prior.iter().all(|d| *d == up || *d == down);
        let differing = self
            .prior
            .windows(2)
            .all(|pair| pair.first() != pair.last());
        if directional && differing {
            mask = set(mask, 39);
        }

        // ---- 40–43: position within the day ----------------------------------
        if let Some((day_high, day_low)) = self.day_extremes() {
            let dh = i128::from(day_high);
            let dl = i128::from(day_low);
            let day_range = dh - dl;
            if close == dh {
                mask = set(mask, 40);
            }
            if close == dl {
                mask = set(mask, 41);
            }
            if day_range > 0 {
                // Upper third: close - low >= 2/3 of the day range, cross-multiplied.
                if (close - dl) * 3 >= day_range * 2 {
                    mask = set(mask, 42);
                }
                if (close - dl) * 3 <= day_range {
                    mask = set(mask, 43);
                }
            }
        }

        // ---- 44–47: time of day ----------------------------------------------
        if let Some(since) = crate::orb::minutes_since_open(bar.ts_micros) {
            let w = self.windows;
            let index = if since < w.early_ends {
                Some(44)
            } else if since < w.mid_ends {
                Some(45)
            } else if since < w.midday_ends {
                Some(46)
            } else if since < w.session_ends {
                Some(47)
            } else {
                // Past 15:30. `docs/06-limits.md` records the 2021-02-24 session
                // running to 16:59, so this is reachable on real data and must not
                // silently fall into `afternoon`.
                None
            };
            if let Some(index) = index {
                mask = set(mask, index);
            }
        }

        // ---- 48–51 and 66–68: yesterday's session ----------------------------
        let Some(prev) = previous else {
            return mask;
        };
        // Both halves of "has a session begun" in ONE test, because they are one
        // question: [`Self::day_open`] and [`Self::day_extremes`] each answer
        // `self.live` and nothing else. Asked separately — as they were, the extremes
        // in an `if let` of their own below — the second `None` arm could only be
        // taken on a state where the first had already returned, so it was an arm no
        // input can reach and no test can cover.
        let (Some(day_open), Some((day_high, day_low))) = (self.day_open(), self.day_extremes())
        else {
            return mask;
        };
        let prev_close = i128::from(prev.close);
        let today_open = i128::from(day_open);
        if today_open > prev_close {
            mask = set(mask, 48);
        }
        if today_open < prev_close {
            mask = set(mask, 49);
        }
        let ph = i128::from(prev.high);
        let pl = i128::from(prev.low);
        let dh = i128::from(day_high);
        let dl = i128::from(day_low);
        if dh <= ph && dl >= pl {
            mask = set(mask, 50);
        }
        if dh > ph && dl < pl {
            mask = set(mask, 51);
        }

        // The gap's midpoint, between yesterday's close and today's open. Only a
        // real gap has a midpoint; a flat open has nothing to be above or below.
        if today_open != prev_close {
            // The midpoint is taken in `i64`, not in `i128` and then narrowed. Both
            // widths are signed and both round towards zero, so the value is the same
            // one; but the midpoint of two `i64` always fits an `i64`, so the
            // narrowing could only refuse on a state that cannot exist — a region no
            // input can reach and no test can cover.
            let mid = prev.close.midpoint(day_open);
            if close > i128::from(mid) {
                mask = set(mask, 66);
            }
            if close < i128::from(mid) {
                mask = set(mask, 67);
            }
            let gap = (today_open - prev_close).abs();
            let Ok(gap) = i64::try_from(gap) else {
                return mask;
            };
            // `unwrap_or`, the same way [`set`] below ignores a refusal, and not an
            // `if let Ok(next)`: `set_near` refuses only when 68 is absent, retired or
            // not a `Near` position, and `the_positions_agree_with_the_vocabulary`
            // asserts it is all three. So the `if let`'s else was an arm no input can
            // reach and no test can cover, while `unwrap_or` decides the same thing
            // inside `core`. The two are the same expression: on a refusal the mask is
            // returned unchanged either way.
            mask = vocab::table::set_near(mask, 68, tolerance, bar.close, mid, gap).unwrap_or(mask);
        }
        mask
    }
}

/// Set a plain position, ignoring a refusal.
fn set(mask: ConditionMask, index: u16) -> ConditionMask {
    vocab::table::set_exact(mask, index).unwrap_or(mask)
}

/// Every position this module can set.
#[must_use]
pub const fn positions() -> [u16; 25] {
    [
        30, 31, 32, 33, 34, 35, 36, // bar shape
        37, 38, 39, // prior-bar sequence
        40, 41, 42, 43, // position within the day
        44, 45, 46, 47, // time of day
        48, 49, 50, 51, // day type
        66, 67, 68, // the opening gap
    ]
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes — a test that \
              cannot panic cannot fail — and here it buys a second thing. \
              `unreachable!` expands to a panic inside THIS crate, so llvm-cov counts \
              a region that can never run and the coverage gate can never reach 100 on \
              this file. `.expect` panics inside core, which is not instrumented: the \
              same failure, with the refused value printed beside the message, and no \
              dead region left behind."
)]
mod tests {
    use super::*;

    fn tol() -> Tolerance {
        vocab::tolerance::pinned_fib().expect("the fib width is pinned")
    }

    fn at(minute: i64, open: i64, high: i64, low: i64, close: i64) -> Candle {
        let ts = (40_000 * 1_440 + 555 + minute) * 60_000_000 - 19_800 * 1_000_000;
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

    fn ok(state: &mut SessionState, bar: &Candle, prev: Option<PreviousSession>) -> ConditionMask {
        state
            .step(bar, prev, tol())
            .expect("this fixture bar is sane")
    }

    /// Every position is live and its kind matches how this module sets it.
    #[test]
    fn the_positions_agree_with_the_vocabulary() {
        let mut seen = std::collections::BTreeSet::new();
        for index in positions() {
            assert!(seen.insert(index), "position {index} appears twice");
            // Built before the call, not interpolated on the failure path: `expect`
            // takes a `&str`, and the position is what identifies a failing row. A
            // message assembled only when the test fails is a region a green run
            // cannot reach — the same reason `tests/extremes.rs` builds its labels
            // up front.
            let missing = format!("position {index} is not in the table");
            let def = vocab::table::definition(index).expect(&missing);
            assert!(vocab::table::is_live(index), "position {index} is not live");
            let wants_near = index == 68;
            assert_eq!(
                def.kind == vocab::Kind::Near,
                wants_near,
                "position {index} (`{}`) is {:?}",
                def.name,
                def.kind,
            );
        }
        assert_eq!(seen.len(), 25);
    }

    /// **Exactly one** time-of-day window holds on every in-session bar, and none
    /// past the close — the 2021-02-24 session ran to 16:59 and must not fall into
    /// `afternoon` by default.
    #[test]
    fn exactly_one_time_of_day_window_holds() {
        let mut state = SessionState::default();
        for minute in 0..375 {
            let mask = ok(&mut state, &at(minute, 100, 110, 90, 105), None);
            let n: u32 = (44..=47).map(|i| u32::from(mask.get(i))).sum();
            assert_eq!(n, 1, "minute {minute} matched {n} windows");
        }
        // 16:00 is 405 minutes after the open: past 15:30, so no window.
        let mask = ok(&mut state, &at(405, 100, 110, 90, 105), None);
        let n: u32 = (44..=47).map(|i| u32::from(mask.get(i))).sum();
        assert_eq!(n, 0, "a bar past the close matched a window");
    }

    /// A description INCLUDES its own bar: the bar that sets the day's high must
    /// be able to report that its close is at the day high.
    #[test]
    fn the_bar_that_sets_the_day_high_can_report_it() {
        let mut state = SessionState::default();
        let mask = ok(&mut state, &at(0, 100, 120, 90, 120), None);
        assert!(
            mask.get(40),
            "the first bar's close was the day high and did not report it",
        );
        assert!(!mask.get(41));
    }

    /// Bullish and bearish are mutually exclusive, and a flat bar is neither.
    #[test]
    fn direction_is_exclusive_and_a_flat_bar_is_neither() {
        let mut state = SessionState::default();
        let up = ok(&mut state, &at(0, 100, 110, 95, 108), None);
        assert!(up.get(30) && !up.get(31));
        let down = ok(&mut state, &at(1, 108, 110, 95, 100), None);
        assert!(down.get(31) && !down.get(30));
        let flat = ok(&mut state, &at(2, 100, 110, 90, 100), None);
        assert!(
            !flat.get(30) && !flat.get(31),
            "a flat bar claimed a direction"
        );
    }

    /// The prior-bar sequence EXCLUDES the current bar — it answers a question
    /// about the bars before it.
    #[test]
    fn a_run_of_three_up_bars_sets_prior_n_bullish_on_the_fourth() {
        let mut state = SessionState::default();
        for minute in 0..3 {
            let mask = ok(&mut state, &at(minute, 100, 110, 95, 108), None);
            assert!(
                !mask.get(37),
                "prior_n_bullish fired at minute {minute}, before three priors existed",
            );
        }
        let mask = ok(&mut state, &at(3, 108, 112, 100, 110), None);
        assert!(
            mask.get(37),
            "three prior up bars did not set prior_n_bullish"
        );
        assert!(!mask.get(38));
        // 39 MUST stay clear, and nothing pinned that until an audit replaced the
        // alternation test's `differing` with `true` and watched all 22 session tests
        // stay green. `prior_alternating` on a monotone run is the same class of defect
        // as 38 firing on flat bars: a bit that reads as a measurement of one shape while
        // the run is the opposite shape.
        assert!(
            !mask.get(39),
            "three consecutive UP bars set prior_alternating, which is the opposite of \
             what the position names"
        );
    }

    /// Alternating means strictly alternating over the three priors.
    #[test]
    fn alternating_is_strict() {
        let mut state = SessionState::default();
        // up, down, up  -> the fourth bar sees an alternating run
        let _ = ok(&mut state, &at(0, 100, 110, 95, 108), None);
        let _ = ok(&mut state, &at(1, 108, 110, 95, 100), None);
        let _ = ok(&mut state, &at(2, 100, 110, 95, 108), None);
        let mask = ok(&mut state, &at(3, 108, 112, 100, 110), None);
        assert!(mask.get(39), "an alternating run was not detected");
        assert!(!mask.get(37) && !mask.get(38));
    }

    /// A run of flat bars is a run of neither direction.
    ///
    /// The ring stored `close > open` — a two-valued answer to the three-valued
    /// question positions 30 and 31 have always asked of the current bar — so every
    /// flat bar was filed as "not up" and therefore as down. Three flat bars then set
    /// 38 on the fourth while 31 had not fired on one of them: a bit that reads as a
    /// measurement and is an artefact of the encoding. Over-reporting is only half of
    /// it. With `false` meaning both "closed down" and "closed unchanged", the
    /// predicate "all three prior bars closed down" was expressible by no live
    /// position at all, so every mask that wanted the real run got this one instead.
    #[test]
    fn a_run_of_flat_bars_is_neither_a_bullish_nor_a_bearish_run() {
        let flat = |minute| at(minute, 2_500_000, 2_500_300, 2_499_700, 2_500_000);
        let mut state = SessionState::default();
        for minute in 0..3 {
            let mask = ok(&mut state, &flat(minute), None);
            assert!(
                !mask.get(30) && !mask.get(31),
                "minute {minute}: a flat bar claimed a direction, so the run below \
                 proves nothing"
            );
            assert!(mask.get(32), "minute {minute}: a flat bar is a doji");
        }
        let mask = ok(&mut state, &flat(3), None);
        assert!(
            !mask.get(38),
            "prior_n_bearish fired over three bars on which bar_bearish never did"
        );
        assert!(!mask.get(37), "prior_n_bullish fired over three flat bars");
        assert!(!mask.get(39), "three flat bars alternated");
    }

    /// The bearish run has a positive case, so "38 never fires" is not a way to pass
    /// `a_run_of_flat_bars_is_neither_a_bullish_nor_a_bearish_run`.
    #[test]
    fn a_run_of_three_down_bars_sets_prior_n_bearish_on_the_fourth() {
        let mut state = SessionState::default();
        let mut px = 2_500_000i64;
        for minute in 0..3 {
            let open = px;
            px -= 400;
            let mask = ok(
                &mut state,
                &at(minute, open, open + 100, px - 100, px),
                None,
            );
            assert!(
                mask.get(31),
                "minute {minute}: a down bar was not reported as one"
            );
            assert!(
                !mask.get(38),
                "prior_n_bearish fired at minute {minute}, before three priors existed"
            );
        }
        let mask = ok(&mut state, &at(3, px, px + 100, px - 500, px - 400), None);
        assert!(
            mask.get(38),
            "three prior down bars did not set prior_n_bearish"
        );
        assert!(!mask.get(37));
        assert!(
            !mask.get(39),
            "three consecutive DOWN bars set prior_alternating, which is the opposite of \
             what the position names"
        );
    }

    /// A flat bar inside a run breaks the alternation as well as the two runs.
    ///
    /// Up, then an unchanged bar, then up is not an alternation, and it is the case that
    /// catches a ring which encodes "unchanged" as "down": under that encoding the three
    /// directions
    /// read as true, false, true — every adjacent pair different — and 39 fired on a
    /// sequence containing no down bar at all.
    #[test]
    fn a_flat_bar_between_two_up_bars_is_not_an_alternation() {
        let mut state = SessionState::default();
        let _ = ok(&mut state, &at(0, 100, 110, 95, 108), None);
        let flat = ok(&mut state, &at(1, 108, 112, 104, 108), None);
        assert!(
            !flat.get(30) && !flat.get(31),
            "the middle bar was not flat, so this proves nothing"
        );
        let _ = ok(&mut state, &at(2, 108, 118, 104, 116), None);
        let mask = ok(&mut state, &at(3, 116, 120, 110, 118), None);
        assert!(
            !mask.get(39),
            "an unchanged bar counted as the opposite of its neighbours"
        );
        assert!(!mask.get(37) && !mask.get(38));
    }

    /// A new IST day resets the day's extremes, the open, and the direction ring.
    #[test]
    fn a_new_session_resets_everything() {
        let mut state = SessionState::default();
        let _ = ok(&mut state, &at(0, 100, 500, 50, 200), None);
        assert_eq!(state.day_extremes(), Some((500, 50)));
        let mut next = at(0, 100, 110, 90, 105);
        next.ts_micros += 1_440 * 60_000_000;
        let mask = ok(&mut state, &next, None);
        assert_eq!(
            state.day_extremes(),
            Some((110, 90)),
            "yesterday's extremes survived"
        );
        assert_eq!(state.day_open(), Some(100));
        assert!(
            !mask.get(37) && !mask.get(38) && !mask.get(39),
            "the ring survived"
        );
    }

    /// Gap up and gap down are exclusive, and an unchanged open is neither. The gap
    /// midpoint positions need a real gap.
    #[test]
    fn the_opening_gap_needs_a_real_gap() {
        let prev = PreviousSession {
            high: 120,
            low: 80,
            close: 100,
        };
        let mut up = SessionState::default();
        let mask = ok(&mut up, &at(0, 110, 115, 105, 112), Some(prev));
        assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");
        // The gap is 100..110, midpoint 105; the close 112 is above it.
        assert!(mask.get(66) && !mask.get(67));

        let mut flat = SessionState::default();
        let mask = ok(&mut flat, &at(0, 100, 115, 95, 112), Some(prev));
        assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");
        assert!(
            !mask.get(66) && !mask.get(67) && !mask.get(68),
            "a flat open produced a gap midpoint",
        );
    }

    /// Inside and outside days are exclusive by construction.
    #[test]
    fn inside_and_outside_days_are_exclusive() {
        let prev = PreviousSession {
            high: 120,
            low: 80,
            close: 100,
        };
        let mut inside = SessionState::default();
        let mask = ok(&mut inside, &at(0, 100, 115, 85, 110), Some(prev));
        assert!(mask.get(50) && !mask.get(51));

        let mut outside = SessionState::default();
        let mask = ok(&mut outside, &at(0, 100, 125, 75, 110), Some(prev));
        assert!(mask.get(51) && !mask.get(50));
    }

    /// Only the 25 positions this module owns are ever set.
    #[test]
    fn nothing_outside_the_twenty_five_positions_is_set() {
        let owned = positions();
        let prev = PreviousSession {
            high: 2_520_000,
            low: 2_480_000,
            close: 2_500_000,
        };
        let mut state = SessionState::default();
        let mut union = ConditionMask::ZERO;
        let mut px = 2_500_000i64;
        for minute in 0..375 {
            let drift = (minute * 41) % 301 - 150;
            let open = px;
            let close = px + drift;
            let high = open.max(close) + (minute * 17) % 80;
            let low = open.min(close) - (minute * 23) % 80;
            px = close;
            union = union.union(&ok(
                &mut state,
                &at(minute, open, high, low, close),
                Some(prev),
            ));
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

    /// Same bars, same bits, byte for byte.
    #[test]
    fn the_same_session_twice_gives_the_same_bits() {
        let prev = PreviousSession {
            high: 2_520_000,
            low: 2_480_000,
            close: 2_500_000,
        };
        let run = || {
            let mut state = SessionState::default();
            let mut px = 2_500_000i64;
            (0..300)
                .map(|minute| {
                    let drift = (minute * 37) % 201 - 100;
                    let open = px;
                    let close = px + drift;
                    let high = open.max(close) + (minute * 13) % 60;
                    let low = open.min(close) - (minute * 19) % 60;
                    px = close;
                    ok(&mut state, &at(minute, open, high, low, close), Some(prev)).words()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    /// A corrupt record is refused and folded into nothing.
    #[test]
    fn a_corrupt_bar_is_refused() {
        let mut state = SessionState::default();
        assert_eq!(
            state.step(&at(0, 100, 90, 110, 105), None, tol()),
            Err(crate::Corrupt::HighBelowLow),
        );
        assert_eq!(state.day_extremes(), None, "a refused bar was folded in");
    }

    /// The two threshold sets are separate structs, deliberately.
    #[test]
    fn the_two_threshold_sets_are_independent() {
        assert_eq!(DayWindows::default(), DayWindows::CLASSICAL);
        assert_eq!(ShapeLimits::default(), ShapeLimits::CLASSICAL);
        assert_eq!(DayWindows::CLASSICAL.session_ends, 375);
        assert_eq!(ShapeLimits::CLASSICAL.large_body, 600);
        // The windows are exhaustive and ordered, or "exactly one holds" is false.
        let w = DayWindows::CLASSICAL;
        assert!(w.early_ends < w.mid_ends);
        assert!(w.mid_ends < w.midday_ends);
        assert!(w.midday_ends < w.session_ends);
    }

    /// A state before its first bar answers nothing, and `bits` with yesterday in hand
    /// still emits no day-type verdict.
    ///
    /// [`SessionState::day_open`] and [`SessionState::day_extremes`] each answer
    /// `self.live` and nothing else, and the fields behind them hold **zero** until a
    /// bar arrives. Without the guard at the head of the yesterday block, a caller
    /// holding a completed previous session would get position 48 or 49 computed from
    /// an open of zero — a gap the width of the entire price scale, off a session that
    /// has not begun. `bits` is public and takes `&self`, so this is a state a consumer
    /// can reach: `step` is not the only door.
    ///
    /// The shape and time-of-day bits, which need only the bar, are still emitted. This
    /// is a guard on the families that need a session, not a blanket refusal.
    #[test]
    fn a_state_before_its_first_bar_emits_no_day_type() {
        let state = SessionState::default();
        assert_eq!(state.day_open(), None, "an unstarted session named an open");
        assert_eq!(
            state.day_extremes(),
            None,
            "an unstarted session named its extremes"
        );
        let prev = PreviousSession {
            high: 120,
            low: 80,
            close: 100,
        };
        let mask = state.bits(&at(0, 100, 110, 95, 108), Some(prev), tol());
        for index in [40, 41, 42, 43, 48, 49, 50, 51, 66, 67, 68] {
            assert!(
                !mask.get(index),
                "position {index} was emitted before the session began"
            );
        }
        assert!(
            mask.get(30),
            "a bullish bar is bullish whether or not a session is live"
        );
        assert!(mask.get(44), "the time of day needs no session state");
    }

    /// The shape positions are exactly the fractions [`ShapeLimits`] names.
    ///
    /// Hand-computed against a range of exactly 1000 paisa, so every bit is a statement
    /// about a number this test can name: a 600-paisa body is 60% of the range and is
    /// `bar_large_body`; a 50-paisa body is 5%, which is both a doji and a small body,
    /// and leaves wicks of 450 and 500 paisa, both past the 40% limit; a 450-paisa body
    /// with 275-paisa wicks is none of the five.
    ///
    /// `bar_large_body` was pinned by nothing at all. The only test that ever set it
    /// asserts which positions this module **owns**, not which are right, so reading
    /// the 600 limit as 6%, or `>=` as `>`, would have stayed green.
    #[test]
    fn the_shape_positions_are_the_fractions_the_limits_name() {
        // (name, bar, [doji 32, large 33, small 34, long upper wick 35, long lower 36])
        let cases: [(&str, Candle, [bool; 5]); 4] = [
            (
                "a 600-paisa body over a 1000-paisa range is large and not small",
                at(0, 2_499_200, 2_500_000, 2_499_000, 2_499_800),
                [false, true, false, false, false],
            ),
            (
                "a 50-paisa body is a doji, a small body, and has both wicks long",
                at(0, 2_499_500, 2_500_000, 2_499_000, 2_499_550),
                [true, false, true, true, true],
            ),
            (
                "a 450-paisa body with 275-paisa wicks is none of the five",
                at(0, 2_499_275, 2_500_000, 2_499_000, 2_499_725),
                [false, false, false, false, false],
            ),
            (
                "a zero-range bar has no shape: every fraction of zero is undefined",
                at(0, 2_500_000, 2_500_000, 2_500_000, 2_500_000),
                [false, false, false, false, false],
            ),
        ];
        for (name, bar, want) in cases {
            let mut state = SessionState::default();
            let mask = ok(&mut state, &bar, None);
            for (offset, expected) in want.iter().enumerate() {
                let index = 32 + u32::try_from(offset).expect("five fits in a u32");
                assert_eq!(mask.get(index), *expected, "{name}: position {index}");
            }
        }
    }

    /// The thirds are measured from the day's **low**, their boundary is inclusive, and
    /// a close in the middle third is in neither.
    ///
    /// Both positions are cross-multiplied against the day's range, so a transposed end
    /// would put every close in the opposite third and still look plausible. Computed
    /// against a 3000-paisa day, where the two boundaries sit exactly 1000 and 2000
    /// paisa above the low and are named here rather than approached.
    #[test]
    fn the_day_thirds_are_measured_from_the_low_and_include_their_boundary() {
        // (name, close, in the upper third, in the lower third)
        let cases: [(&str, i64, bool, bool); 5] = [
            ("the close at the day high", 2_502_000, true, false),
            ("exactly two thirds up", 2_501_000, true, false),
            ("the middle third", 2_500_500, false, false),
            ("exactly one third up", 2_500_000, false, true),
            ("the close at the day low", 2_499_000, false, true),
        ];
        for (name, close, upper, lower) in cases {
            let mut state = SessionState::default();
            let bar = at(0, 2_500_500, 2_502_000, 2_499_000, close);
            let mask = ok(&mut state, &bar, None);
            assert_eq!(mask.get(42), upper, "{name}: the upper third");
            assert_eq!(mask.get(43), lower, "{name}: the lower third");
        }
    }

    /// A pre-open print belongs to no time-of-day window.
    ///
    /// `crate::orb::minutes_since_open` is `None` before 09:15, and the `if let` that
    /// consumes it is the only thing standing between an 08:00 print and
    /// `early_morning` — without it the bar would be described as the first hour of a
    /// session that had not started. Ingest is supposed to drop such a print, but this
    /// module does not lean on a filter it cannot see; `crate::orb::fold` says the same
    /// thing in the same words. `exactly_one_time_of_day_window_holds` covers the other
    /// end, a bar past the close.
    #[test]
    fn a_pre_open_bar_belongs_to_no_time_of_day_window() {
        let mut state = SessionState::default();
        // Seventy-five minutes before the open is 08:00 IST.
        let mask = ok(&mut state, &at(-75, 100, 110, 95, 108), None);
        for index in 44..=47 {
            assert!(!mask.get(index), "a pre-open bar matched window {index}");
        }
        assert!(
            mask.get(30),
            "the pre-open bar was not evaluated at all, so this proves nothing"
        );
    }

    /// A close on the gap's midpoint sets the midpoint band; a close outside the band
    /// does not.
    ///
    /// `the_opening_gap_needs_a_real_gap` covers the half where there is no gap to have
    /// a midpoint. The other half had never been reached: 68 is the one `Near` position
    /// this module owns, and `vocab::table::set_near` returns the mask unchanged
    /// whether it sets a bit or declines — so a wrong index, a wrong kind or a wrong
    /// base would have been silent. The band is the pinned fib width, 10/1000 of the
    /// gap: 100 paisa across a gap of 10,000.
    #[test]
    fn a_close_on_the_gap_midpoint_sets_the_band_and_one_outside_it_does_not() {
        let prev = PreviousSession {
            high: 2_520_000,
            low: 2_480_000,
            close: 2_500_000,
        };
        // The gap runs 2_500_000..2_510_000, so its midpoint is 2_505_000 exactly.
        let mut on = SessionState::default();
        let mask = ok(
            &mut on,
            &at(0, 2_510_000, 2_512_000, 2_504_000, 2_505_000),
            Some(prev),
        );
        assert!(
            mask.get(68),
            "a close exactly on the gap midpoint did not set the band"
        );
        assert!(
            !mask.get(66) && !mask.get(67),
            "a close ON the midpoint is neither above nor below it"
        );
        assert!(mask.get(48), "an open above yesterday's close is a gap up");

        let mut off = SessionState::default();
        let mask = ok(
            &mut off,
            &at(0, 2_510_000, 2_512_000, 2_504_000, 2_512_000),
            Some(prev),
        );
        assert!(
            !mask.get(68),
            "a close 7,000 paisa from the midpoint, against a band of 100, set it anyway"
        );
        assert!(mask.get(66), "a close above the midpoint was not reported");
    }

    /// A gap wider than `i64` drops the band rather than wrapping.
    ///
    /// The gap is taken in `i128`, because `today_open - prev_close` for two `i64`
    /// prices need not fit an `i64`, and `set_near` needs it back as an `i64` base.
    /// Yesterday closing at the bottom of the type and today opening at the top is
    /// exactly that state. The positions decided before the width check still stand and
    /// the band is simply absent, which is the only honest answer: wrapping would hand
    /// `vocab` a band computed from a negative width.
    #[test]
    fn a_gap_too_wide_for_the_type_drops_the_band_rather_than_wrapping() {
        let prev = PreviousSession {
            high: 0,
            low: i64::MIN,
            close: i64::MIN,
        };
        let mut state = SessionState::default();
        let top = at(0, i64::MAX, i64::MAX, i64::MAX, i64::MAX);
        let mask = ok(&mut state, &top, Some(prev));
        assert!(
            mask.get(48),
            "an open above yesterday's close is a gap up at any width"
        );
        assert!(
            mask.get(66),
            "the close sits above the midpoint of that gap"
        );
        assert!(
            !mask.get(68),
            "a gap that does not fit i64 still produced a band"
        );
    }
}
