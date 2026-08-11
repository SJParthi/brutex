//! The opening range, at four windows.
//!
//! **20 vocabulary positions**, 86–105: five relations for each of the 5-, 15-,
//! 30- and 60-minute windows measured from the session open.
//!
//! # A window emits nothing until it closes
//!
//! `docs/09-design-sources.md` §3 records the design source, and the detail that
//! matters is cosmetic in Pine and load-bearing here: the indicator draws the
//! level **transparent while it is still moving** and solid once the window has
//! shut. That encodes a rule. A high that can still rise is not a level, and a
//! bit read against it would answer a different question on the next bar — a
//! repaint, in the only sense that matters to a backtest.
//!
//! So every position stays false for every bar inside its own window, and the
//! first bar that can set one is the first bar **after** the window closes. On a
//! one-minute rung that costs 5 bars of the 375 for `orb5` and 60 for `orb60`.
//!
//! # The window is a clock span, not a bar count
//!
//! `docs/06-limits.md` records ten sessions wholly or partly outside 09:15–15:29
//! and says outright that any code hardcoding 375 bars is wrong on those days. So
//! membership is decided by the bar's IST minute-of-day against the open, never by
//! how many bars have arrived. A session with a hole in it, a late open, or a
//! half day all work without a special case, because none of them changes what
//! "within 15 minutes of 09:15" means.
//!
//! # What this deliberately does not do
//!
//! It does not know the sweep's timeframe. A 5-minute window cannot be resolved
//! on a rung whose bars are longer than the window — the design source refuses to
//! draw in exactly that case (`timeframe.multiplier <= inputMax`). Deciding that
//! is the caller's, because only the caller knows the rung; this module reports
//! [`Orb::window_closed`] so the caller can abstain per run rather than per bar.

use crate::Candle;
use vocab::{ConditionMask, Tolerance};

/// Minutes from IST midnight to the NSE open, 09:15.
const OPEN_MINUTE: i64 = 9 * 60 + 15;
const IST_OFFSET_MICROS: i64 = 19_800 * 1_000_000;
const MICROS_PER_MINUTE: i64 = 60 * 1_000_000;
const MINUTES_PER_DAY: i64 = 24 * 60;

/// The four window lengths, in minutes, in the order positions 86–105 carry them.
pub const WINDOWS: [i64; 4] = [5, 15, 30, 60];

/// The first vocabulary position of the opening-range group.
pub const ORB_FIRST: u16 = 86;

/// Relations per window, in position order. Five of them, so window `w` owns
/// `ORB_FIRST + 5*w ..= ORB_FIRST + 5*w + 4`.
const RELATIONS_PER_WINDOW: u16 = 5;

/// The number of windows, counted in the `u16` the position arithmetic needs.
///
/// [`Orb::bits`] used to walk `0..WINDOWS.len()` and derive this per window with
/// `u16::try_from(w)`, skipping the window on a refusal. The refusal cannot happen —
/// the loop bound is four — and an arm no input can take is worse than dead weight:
/// it reads as a case that was thought about, and it is a region no passing test can
/// execute. Counting in `u16` from the start removes the arm rather than hiding it.
const WINDOW_COUNT: u16 = 4;

/// `WINDOW_COUNT` and [`WINDOWS`] describe the same four windows. A fifth length added
/// to one and not the other is a build failure here, not a window that is silently
/// never swept.
const _: () = assert!(WINDOWS.len() == 4 && WINDOW_COUNT == 4);

/// Minutes since the session open, or `None` before it.
///
/// `div_euclid` and `rem_euclid`, never `/` and `%`: both truncate toward zero,
/// which puts a pre-epoch stamp on the wrong day and a negative remainder in the
/// wrong minute.
#[must_use]
pub fn minutes_since_open(ts_micros: i64) -> Option<i64> {
    let ist = ts_micros.wrapping_add(IST_OFFSET_MICROS);
    let minute_of_day = ist
        .div_euclid(MICROS_PER_MINUTE)
        .rem_euclid(MINUTES_PER_DAY);
    minute_of_day.checked_sub(OPEN_MINUTE).filter(|m| *m >= 0)
}

/// One window's running extremes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Window {
    high: i64,
    low: i64,
    /// True once a bar has landed inside the window, so the extremes mean something.
    seeded: bool,
    /// True once a bar has arrived at or past the window's end.
    closed: bool,
}

impl Window {
    const fn new() -> Self {
        Self {
            high: 0,
            low: 0,
            seeded: false,
            closed: false,
        }
    }
}

/// The four opening-range windows for one session.
///
/// Fixed size, no allocation, and it does not grow with the number of bars — the
/// same constant-space argument the Fibonacci family rests on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Orb {
    session_day: i64,
    live: bool,
    windows: [Window; 4],
}

const _: () = assert!(core::mem::size_of::<Orb>() <= 128);

impl Default for Orb {
    fn default() -> Self {
        Self::new()
    }
}

impl Orb {
    /// An empty state, before any session.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            session_day: i64::MIN,
            live: false,
            windows: [Window::new(); 4],
        }
    }

    /// Has window `w` (0-indexed into [`WINDOWS`]) closed, so its level is final?
    ///
    /// A caller that reads this per run rather than per bar is the one place the
    /// timeframe question belongs — see the module documentation.
    #[must_use]
    pub fn window_closed(&self, w: usize) -> bool {
        self.windows.get(w).is_some_and(|x| x.closed && x.seeded)
    }

    /// Window `w`'s extremes, once it has closed. `None` while it is still
    /// forming, or if no bar ever landed inside it.
    #[must_use]
    pub fn extremes(&self, w: usize) -> Option<(i64, i64)> {
        self.windows
            .get(w)
            .filter(|x| x.closed && x.seeded)
            .map(|x| (x.high, x.low))
    }

    /// The full per-bar step: reset on a new IST day, emit, then fold.
    ///
    /// The emit-before-fold order is the same discipline the Fibonacci family
    /// uses, and here it is what stops the bar that CLOSES a window from being
    /// measured against a level its own high just set.
    ///
    /// # Errors
    ///
    /// [`crate::Corrupt`] for a record that is not a bar.
    pub fn step(
        &mut self,
        bar: &Candle,
        tolerance: Tolerance,
    ) -> Result<ConditionMask, crate::Corrupt> {
        bar.check()?;
        let day = crate::ist_day(bar.ts_micros);
        if day != self.session_day || !self.live {
            *self = Self {
                session_day: day,
                live: true,
                windows: [Window::new(); 4],
            };
        }
        // Two different things happen around the emit, in two different orders,
        // and conflating them cost this family one bar.
        //
        // A window's HIGH and LOW are an ANCHOR: a reference the current bar is
        // measured against, so they must EXCLUDE it — hence `fold` after the emit.
        // But WHETHER THE WINDOW HAS CLOSED is a DESCRIPTION of the current bar, so
        // it must INCLUDE it. `closed` used to be set inside `fold`, i.e. after the
        // emit, so on the bar whose minute-since-open first equals the window length
        // — the very first bar whose close should be compared against a final
        // opening range — `bits()` still saw the window as forming and emitted
        // nothing. The family was false for one bar longer than it should be, once
        // per window per day.
        self.mark_closed(bar);
        let bits = self.bits(bar.close, tolerance);
        self.fold(bar);
        Ok(bits)
    }

    /// Close any window this bar has reached the end of.
    ///
    /// Separate from [`Self::fold`] because it runs on the other side of the emit:
    /// see [`Self::step`]. `closed` has exactly one owner, and this is it.
    fn mark_closed(&mut self, bar: &Candle) {
        let Some(since) = minutes_since_open(bar.ts_micros) else {
            return;
        };
        // Zipped, not indexed. `self.windows.get_mut(w)` for `w` drawn from
        // `WINDOWS.iter().enumerate()` cannot be `None` — both are four wide, and
        // `WINDOW_COUNT` asserts that at compile time — so the `Some` arm this used to
        // spell was a condition with no false case. See `WINDOW_COUNT`.
        for (slot, length) in self.windows.iter_mut().zip(WINDOWS) {
            if since >= length {
                slot.closed = true;
            }
        }
    }

    /// Fold one bar into whichever windows are still open.
    fn fold(&mut self, bar: &Candle) {
        let Some(since) = minutes_since_open(bar.ts_micros) else {
            // Before the open. A pre-open print is not part of any opening range,
            // and ingest is supposed to have dropped it — but this module does not
            // rely on that, because a filter it cannot see is a filter it cannot
            // trust.
            return;
        };
        // Zipped for the reason `Self::mark_closed` is: the `continue` that used to
        // handle a missing slot handled nothing, because `[Window; 4]` and a four-long
        // `WINDOWS` cannot disagree about an index.
        for (slot, length) in self.windows.iter_mut().zip(WINDOWS) {
            if since < length {
                if slot.seeded {
                    if bar.high > slot.high {
                        slot.high = bar.high;
                    }
                    if bar.low < slot.low {
                        slot.low = bar.low;
                    }
                } else {
                    slot.high = bar.high;
                    slot.low = bar.low;
                    slot.seeded = true;
                }
            }
            // `closed` is NOT set here. `Self::mark_closed` owns it, because it must
            // run before the emit and this must run after — see `Self::step`.
        }
    }

    /// The twenty positions, for one closing price.
    ///
    /// # Cost
    ///
    /// Four windows times five relations — a compile-time constant, which is what
    /// makes this O(1) rather than "O(windows)". No allocation, no division.
    /// Measured by `C-I-02`, in `crates/indicators/benches/ratio.rs`.
    #[must_use]
    pub fn bits(&self, close: i64, tolerance: Tolerance) -> ConditionMask {
        let mut mask = ConditionMask::ZERO;
        for offset in 0..WINDOW_COUNT {
            let Some((high, low)) = self.extremes(usize::from(offset)) else {
                // Still forming, or never seeded. Every position for this window
                // stays false: a level that can still move is not a level, and
                // `docs/03-vocabulary.md` §4 wants false rather than "probably".
                continue;
            };
            let base = ORB_FIRST + offset * RELATIONS_PER_WINDOW;
            let range = high.saturating_sub(low);

            if close > high {
                mask = set(mask, base);
            }
            if close < low {
                mask = set(mask, base + 1);
            }
            if low <= close && close <= high {
                mask = set(mask, base + 2);
            }
            mask = near(mask, base + 3, tolerance, close, high, range);
            mask = near(mask, base + 4, tolerance, close, low, range);
        }
        mask
    }
}

/// Set a plain position, ignoring a refusal.
///
/// A refusal means this module and the table disagree about a position number,
/// which `orb::the_positions_agree_with_the_vocabulary` makes a test failure. At
/// run time, setting nothing is the safe answer.
fn set(mask: ConditionMask, index: u16) -> ConditionMask {
    vocab::table::set_exact(mask, index).unwrap_or(mask)
}

/// Set a banded position through `vocab`, so the band check happens once and in
/// the crate that owns it.
fn near(
    mask: ConditionMask,
    index: u16,
    tolerance: Tolerance,
    close: i64,
    level: i64,
    range: i64,
) -> ConditionMask {
    vocab::table::set_near(mask, index, tolerance, close, level, range).unwrap_or(mask)
}

/// Every position this module can set.
#[must_use]
pub fn positions() -> [u16; 20] {
    let mut out = [0u16; 20];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = ORB_FIRST + u16::try_from(i).unwrap_or(u16::MAX);
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes — see \
              crates/costs/src/money.rs and crates/indicators/tests/shared_core_doc.rs: \
              a test that cannot panic cannot fail. `expect` and not the \
              `let ... else { unreachable!() }` this module used to spell, because \
              `unreachable!` expands to a panic inside THIS crate and is therefore a \
              coverage region no green run can ever execute, while `expect` panics \
              inside the standard library and leaves no such region behind."
)]
mod tests {
    use super::*;

    fn tol() -> Tolerance {
        vocab::tolerance::pinned_fib().expect("the fib width is pinned")
    }

    /// A bar `m` minutes after the open on IST day 20,000.
    fn at(m: i64, h: i64, l: i64, c: i64) -> Candle {
        let ts =
            (20_000 * MINUTES_PER_DAY + OPEN_MINUTE + m) * MICROS_PER_MINUTE - IST_OFFSET_MICROS;
        Candle {
            ts_micros: ts,
            open: c,
            high: h,
            low: l,
            close: c,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    fn ok(o: &mut Orb, b: &Candle) -> ConditionMask {
        o.step(b, tol()).expect("this fixture bar is sane")
    }

    /// The minute arithmetic, at the boundaries that matter.
    #[test]
    fn the_open_is_minute_zero_and_before_it_is_none() {
        let open = at(0, 1, 1, 1);
        assert_eq!(minutes_since_open(open.ts_micros), Some(0));
        assert_eq!(minutes_since_open(at(5, 1, 1, 1).ts_micros), Some(5));
        assert_eq!(minutes_since_open(at(374, 1, 1, 1).ts_micros), Some(374));
        // One minute before the open is not in any window.
        assert_eq!(minutes_since_open(open.ts_micros - MICROS_PER_MINUTE), None);
    }

    /// **Nothing fires while a window is still open.** The whole point.
    #[test]
    fn a_forming_window_emits_nothing() {
        let mut o = Orb::new();
        for m in 0..5 {
            let mask = ok(&mut o, &at(m, 100 + m, 90 - m, 95));
            for index in positions() {
                assert!(
                    !mask.get(u32::from(index)),
                    "position {index} fired at minute {m}, inside every window",
                );
            }
        }
        assert!(!o.window_closed(0), "orb5 closed while still forming");
    }

    /// The 5-minute window closes at minute 5 and its level is then final.
    #[test]
    fn the_five_minute_window_closes_at_minute_five() {
        let mut o = Orb::new();
        for m in 0..5 {
            let _ = ok(&mut o, &at(m, 110, 90, 100));
        }
        assert!(!o.window_closed(0));
        let _ = ok(&mut o, &at(5, 120, 80, 100));
        assert!(o.window_closed(0), "orb5 did not close at minute 5");
        assert_eq!(
            o.extremes(0),
            Some((110, 90)),
            "the level moved after close"
        );
        // The wider windows are still forming.
        assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
    }

    /// Nesting: a narrower window's extremes are contained by a wider one's. A
    /// violation is a proof of a look-ahead bug, and it is free to check.
    #[test]
    fn narrower_window_extremes_are_contained_by_wider() {
        let mut o = Orb::new();
        for m in 0..70 {
            let h = 100 + (m * 7) % 23;
            let l = 90 - (m * 5) % 19;
            let _ = ok(&mut o, &at(m, h, l, 95));
        }
        let mut previous: Option<(i64, i64)> = None;
        for w in 0..WINDOWS.len() {
            let (high, low) = o.extremes(w).expect("every window has closed by minute 70");
            if let Some((ph, pl)) = previous {
                assert!(high >= ph, "window {w}'s high is below a narrower one's");
                assert!(low <= pl, "window {w}'s low is above a narrower one's");
            }
            previous = Some((high, low));
        }
    }

    /// Exactly one of above / below / inside holds once a window has closed.
    #[test]
    fn exactly_one_of_the_three_relations_holds_after_close() {
        let mut o = Orb::new();
        for m in 0..61 {
            let _ = ok(&mut o, &at(m, 110, 90, 100));
        }
        for close in [50_i64, 89, 90, 100, 110, 111, 500] {
            let mask = o.bits(close, tol());
            for offset in 0..WINDOW_COUNT {
                let base = u32::from(ORB_FIRST + offset * RELATIONS_PER_WINDOW);
                let n = u32::from(mask.get(base))
                    + u32::from(mask.get(base + 1))
                    + u32::from(mask.get(base + 2));
                assert_eq!(
                    n, 1,
                    "window {offset} at close {close} had {n} of the three relations",
                );
            }
        }
    }

    /// A new IST day discards the previous session's windows.
    #[test]
    fn a_new_session_starts_the_windows_again() {
        let mut o = Orb::new();
        for m in 0..61 {
            let _ = ok(&mut o, &at(m, 110, 90, 100));
        }
        assert!(o.window_closed(3));
        let mut next = at(0, 105, 95, 100);
        next.ts_micros += MINUTES_PER_DAY * MICROS_PER_MINUTE;
        let mask = ok(&mut o, &next);
        assert!(mask.is_empty(), "the new session emitted on its first bar");
        for w in 0..WINDOWS.len() {
            assert!(
                !o.window_closed(w),
                "window {w} stayed closed across the day"
            );
        }
    }

    /// A pre-open print belongs to no window.
    #[test]
    fn a_bar_before_the_open_is_folded_into_nothing() {
        let mut o = Orb::new();
        let mut early = at(0, 9_999, 1, 100);
        early.ts_micros -= 30 * MICROS_PER_MINUTE;
        let _ = ok(&mut o, &early);
        for m in 0..6 {
            let _ = ok(&mut o, &at(m, 110, 90, 100));
        }
        assert_eq!(
            o.extremes(0),
            Some((110, 90)),
            "a pre-open print reached the opening range",
        );
    }

    /// Only the twenty positions this module owns are ever set.
    #[test]
    fn nothing_outside_the_twenty_positions_is_set() {
        let owned = positions();
        let mut o = Orb::new();
        let mut union = ConditionMask::ZERO;
        for m in 0..120 {
            let h = 2_500_000 + (m * 31) % 400;
            let l = 2_500_000 - (m * 17) % 400;
            // The close is derived from the span so it is always INSIDE it. The old
            // fixture used an independent `2_500_000 + (m*13)%300`, which lands above
            // the high on 39 of these 120 bars — an invalid candle that the old
            // two-check preamble accepted and `Candle::check` now refuses.
            let c = l + (m * 13) % (h - l + 1);
            union = union.union(&ok(&mut o, &at(m, h, l, c)));
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

    /// Every position is live, and its kind matches how this module sets it.
    #[test]
    fn the_positions_agree_with_the_vocabulary() {
        for (i, index) in positions().iter().enumerate() {
            let def = vocab::table::definition(*index).expect("86..=105 are allocated");
            assert!(
                vocab::table::is_live(*index),
                "position {index} is not live"
            );
            let wants_near = i % 5 >= 3;
            let is_near = def.kind == vocab::Kind::Near;
            // Chosen BEFORE the assertion rather than inside its message. An `if` in a
            // failure message is a branch that runs only when the test fails, so both
            // of its arms are regions a green run cannot reach; here both run on every
            // pass — twelve bare positions and eight banded ones.
            let sets_it_as = if wants_near { "banded" } else { "bare" };
            assert_eq!(
                is_near, wants_near,
                "position {index} (`{}`) is {:?} and this module sets it as {sets_it_as}",
                def.name, def.kind,
            );
            let expected_window = WINDOWS.get(i / 5).copied().unwrap_or(0);
            assert!(
                def.name.starts_with(&format!("orb{expected_window}_")),
                "position {index} is `{}` and belongs to the {expected_window}-minute window",
                def.name,
            );
        }
    }

    /// Same bars, same bits, byte for byte.
    #[test]
    fn the_same_session_twice_gives_the_same_bits() {
        let run = || {
            let mut o = Orb::new();
            (0..120)
                .map(|m| {
                    let h = 2_500_000 + (m * 31) % 400;
                    let l = 2_500_000 - (m * 17) % 400;
                    let c = l + (m * 13) % (h - l + 1);
                    ok(&mut o, &at(m, h, l, c)).words()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    /// A corrupt record is refused and folded into nothing.
    #[test]
    fn a_corrupt_bar_is_refused() {
        let mut o = Orb::new();
        assert_eq!(
            o.step(&at(0, 90, 110, 100), tol()),
            Err(crate::Corrupt::HighBelowLow),
        );
        assert_eq!(
            o.step(&at(0, i64::MAX, i64::MIN, 0), tol()),
            Err(crate::Corrupt::RangeOverflows),
        );
    }

    /// `Orb::default()` is [`Orb::new`], and in particular it is not a zeroed struct.
    ///
    /// Nothing called `default()`, which is exactly how a hand-written `Default` gets
    /// replaced by `#[derive(Default)]` in a later cleanup without anything going red.
    /// A derived one would carry `session_day: 0` and `live: false`; day 0 is a real
    /// IST day, 1970-01-01, so the first bar of that session would find
    /// `day == self.session_day` and — because `live` is false — still reset, which
    /// looks harmless until the second field is the one that changes. `Orb::new` picks
    /// `i64::MIN` deliberately: no bar this crate accepts can carry that IST day, so a
    /// fresh state cannot be mistaken for a session in progress on the strength of the
    /// day alone. This asserts the sentinel, not just the equality, because two derived
    /// zeroes would satisfy an equality between two derived values.
    #[test]
    fn default_is_the_empty_state_and_not_a_zeroed_one() {
        let fresh = Orb::default();
        assert_eq!(fresh, Orb::new(), "Orb::default() drifted from Orb::new()");
        assert_eq!(
            fresh.session_day,
            i64::MIN,
            "a fresh Orb claims a real IST day, so a bar on that day would not reset it"
        );
        assert!(
            !fresh.live,
            "a fresh Orb claims a session is already running"
        );
        for w in 0..WINDOWS.len() {
            assert!(
                !fresh.window_closed(w),
                "window {w} is closed before any bar has arrived"
            );
            assert_eq!(
                fresh.extremes(w),
                None,
                "window {w} has a level before any bar has arrived"
            );
        }
        // And it folds a session exactly as `new` does, which equality of the empty
        // state alone does not prove.
        let mut from_default = Orb::default();
        let mut from_new = Orb::new();
        for m in 0..6 {
            let bar = at(m, 110, 90, 100);
            assert_eq!(
                ok(&mut from_default, &bar).words(),
                ok(&mut from_new, &bar).words(),
                "minute {m} emitted differently from a defaulted Orb"
            );
        }
        assert_eq!(
            from_default, from_new,
            "the two states diverged over a session"
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "as `mod tests` above, and for the same coverage reason: `unreachable!` \
              expands to a panic inside this crate, so the arm a passing test never \
              takes is a region a passing test can never cover."
)]
mod boundary_bar {
    use super::*;

    fn at(minute: i64, o: i64, h: i64, l: i64, c: i64) -> Candle {
        // Minute 0 is 09:15 IST = 555 minutes past IST midnight, in UTC micros.
        const OPEN_UTC_MICROS: i64 = (555 - 330) * 60 * 1_000_000;
        Candle {
            ts_micros: OPEN_UTC_MICROS + minute * 60 * 1_000_000,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    /// The bar at minute == window length must already be measured against the
    /// finished opening range.
    ///
    /// Minutes 0..4 form the 5-minute range. Minute 5 is the first bar OUTSIDE it,
    /// so its close is the first close that can be compared against a final level —
    /// and it was being skipped, because `closed` was set inside `fold`, which runs
    /// after the emit. One lost bar per window per day, silently.
    #[test]
    fn the_bar_at_the_window_boundary_is_already_measured_against_it() {
        let tolerance = vocab::tolerance::pinned_fib().expect("the pinned fib tolerance is valid");
        let mut o = Orb::new();
        // Minutes 0..4: range 2_500_000 .. 2_501_000.
        for m in 0..5 {
            let _ = o
                .step(
                    &at(m, 2_500_500, 2_501_000, 2_500_000, 2_500_500),
                    tolerance,
                )
                .expect("a sane bar");
        }
        // Minute 5: outside the 5-minute window, closing well above its high.
        let bits = o
            .step(
                &at(5, 2_502_000, 2_502_000, 2_502_000, 2_502_000),
                tolerance,
            )
            .expect("a sane bar");
        assert!(
            o.window_closed(0),
            "the 5-minute window must be closed at minute 5"
        );
        let any = (0..vocab::ConditionMask::BITS).any(|b| bits.get(b));
        assert!(
            any,
            "minute 5 closed above a finished 5-minute range and emitted NOTHING; \
             the boundary bar was being skipped"
        );
    }

    /// The window is not closed one bar early either.
    ///
    /// Without this half, `mark_closed` could use `>` instead of `>=`, or run a bar
    /// too eagerly, and the test above would still pass.
    #[test]
    fn the_window_is_not_closed_before_its_length_is_reached() {
        let tolerance = vocab::tolerance::pinned_fib().expect("the pinned fib tolerance is valid");
        let mut o = Orb::new();
        for m in 0..5 {
            let _ = o
                .step(
                    &at(m, 2_500_500, 2_501_000, 2_500_000, 2_500_500),
                    tolerance,
                )
                .expect("a sane bar");
            assert!(
                !o.window_closed(0),
                "minute {m} is inside the 5-minute window; it must still be forming"
            );
        }
    }
}
