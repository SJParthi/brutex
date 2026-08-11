//! Bar shape, prior-bar sequence, position within the day, time of day, day type
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

use store::format::Bar;
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
    prior: [Option<bool>; PRIOR_N],
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
        bar: &Bar,
        previous: Option<PreviousSession>,
        tolerance: Tolerance,
    ) -> Result<ConditionMask, crate::Corrupt> {
        if bar.high < bar.low {
            return Err(crate::Corrupt::HighBelowLow);
        }
        if bar.high.checked_sub(bar.low).is_none() {
            return Err(crate::Corrupt::RangeOverflows);
        }
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
        if let Some(newest) = self.prior.first_mut() {
            *newest = Some(bar.close > bar.open);
        }
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
        bar: &Bar,
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
        // Zipped rather than indexed: `clippy::needless_range_loop` and
        // `clippy::indexing_slicing` are both denied, and iterating the array is
        // also the form that cannot go out of step with PRIOR_N.
        let all_known = self.prior.iter().all(Option::is_some);
        if all_known {
            let dirs: [bool; PRIOR_N] = {
                let mut out = [false; PRIOR_N];
                for (slot, dir) in out.iter_mut().zip(self.prior.iter()) {
                    *slot = dir.unwrap_or(false);
                }
                out
            };
            if dirs.iter().all(|d| *d) {
                mask = set(mask, 37);
            }
            if dirs.iter().all(|d| !*d) {
                mask = set(mask, 38);
            }
            // Strictly alternating: every adjacent pair differs.
            let alternating = dirs.windows(2).all(|pair| pair.first() != pair.last());
            if alternating {
                mask = set(mask, 39);
            }
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
        let Some(day_open) = self.day_open() else {
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
        if let Some((day_high, day_low)) = self.day_extremes() {
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
        }

        // The gap's midpoint, between yesterday's close and today's open. Only a
        // real gap has a midpoint; a flat open has nothing to be above or below.
        if today_open != prev_close {
            let mid = prev_close.midpoint(today_open);
            let Ok(mid) = i64::try_from(mid) else {
                return mask;
            };
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
            if let Ok(next) = vocab::table::set_near(mask, 68, tolerance, bar.close, mid, gap) {
                mask = next;
            }
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
mod tests {
    use super::*;

    fn tol() -> Tolerance {
        let Ok(t) = vocab::tolerance::pinned_fib() else {
            unreachable!("the fib width is pinned")
        };
        t
    }

    fn at(minute: i64, open: i64, high: i64, low: i64, close: i64) -> Bar {
        let ts = (40_000 * 1_440 + 555 + minute) * 60_000_000 - 19_800 * 1_000_000;
        Bar {
            ts_micros: ts,
            open,
            high,
            low,
            close,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    fn ok(state: &mut SessionState, bar: &Bar, prev: Option<PreviousSession>) -> ConditionMask {
        let Ok(m) = state.step(bar, prev, tol()) else {
            unreachable!("this fixture bar is sane")
        };
        m
    }

    /// Every position is live and its kind matches how this module sets it.
    #[test]
    fn the_positions_agree_with_the_vocabulary() {
        let mut seen = std::collections::BTreeSet::new();
        for index in positions() {
            assert!(seen.insert(index), "position {index} appears twice");
            let Some(def) = vocab::table::definition(index) else {
                unreachable!("position {index} is not in the table")
            };
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

    /// Gap up and gap down are exclusive, and a flat open is neither. The gap
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
}
