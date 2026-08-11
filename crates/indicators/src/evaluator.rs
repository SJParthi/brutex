//! One bar in, its whole mask out.
//!
//! # Why this exists
//!
//! Before this module, `crates/indicators` had **seven modules and no evaluator**.
//! Each returned its own [`ConditionMask`] and no production code unioned them —
//! the only `union` calls in the crate were inside its own tests. So "give me every
//! condition bit for this bar" was a function that did not exist, and every caller
//! would have had to reinvent the session bookkeeping below. An audit named this the
//! missing integration layer and it was right.
//!
//! # What the bookkeeping actually is
//!
//! Three families need a *completed* session, not the running one:
//!
//! - [`crate::daily`] needs the previous session's high, low and close to build the
//!   CPR and the S1–S5 / R1–R5 ladder.
//! - [`crate::fib::prev_day_bits`] needs the same levels.
//! - [`crate::fib::Prev5`] needs the last five completed sessions' extremes.
//!
//! None of them can be handed a bar. Something has to notice the IST day change,
//! close the books on the session that just ended, and roll the result forward. That
//! is the whole of what this module adds, and it is why the seven modules could not
//! simply be called in a row.
//!
//! # The order, and why it is not one order
//!
//! Per bar: **roll over, then emit, then fold.**
//!
//! The fold is last because the day's running high and low are an **anchor** — a
//! reference the current bar is measured *against* — so they must exclude it. Were
//! they folded first, the first bar of a session would be compared to a level its own
//! high had just set, and `close_at_day_high` would be true on every opening bar.
//!
//! The rollover is first because *which session we are in* is a **description** of
//! the current bar and must include it. `crate::orb` had this exact bug: it set its
//! `closed` flag inside its fold, after the emit, and the family was false for one
//! bar longer than it should have been, once per window per day.
//!
//! A module holding both kinds of quantity needs both orders. That is not a
//! contradiction; it is the distinction, and it is the one thing to hold onto when
//! reading any of these modules.
//!
//! # Cost
//!
//! Seven module states, each fixed-size, plus four `i64` of session bookkeeping.
//! `size_of` is asserted below. Nothing grows with the number of bars fed, and there
//! is no allocation on the per-bar path — the mask is `Copy` and the unions are
//! bitwise ORs over six `u64`.

use crate::Candle;
use vocab::{ConditionMask, Tolerance};

use crate::daily::DailyLevels;
use crate::fib::Prev5;
use crate::gap::GapFib;
use crate::orb::Orb;
use crate::pattern::{Patterns, Thresholds};
use crate::session::{PreviousSession, SessionState};
use crate::trend::TrendState;
use crate::vwap::{Availability, Vwap};
use crate::{Corrupt, CurDayFib};

/// The two tolerance widths a full evaluation needs.
///
/// Two, not one, and this is not a convenience: the Fibonacci families measure their
/// band as thousandths of the **session range**, while the pivot families measure it
/// as thousandths of the **CPR width**. Those are different quantities and a single
/// number cannot serve both — at the width correct for one, the other is off by a
/// factor of fifty. Recorded as D-0079.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Widths {
    /// Band width for the Fibonacci ladders, in thousandths of the session range.
    pub fib: Tolerance,
    /// Band width for the pivot and CPR families, in thousandths of the CPR width.
    pub pivot: Tolerance,
}

impl Widths {
    /// The widths this repository ships.
    ///
    /// # Errors
    ///
    /// [`vocab::VocabError`] if either pinned width is unusable, which a const
    /// assertion in `vocab::tolerance` already makes impossible — this returns a
    /// `Result` rather than panicking because §9 denies `unwrap` in this workspace.
    pub fn pinned() -> Result<Self, vocab::VocabError> {
        Ok(Self {
            fib: vocab::tolerance::pinned_fib()?,
            pivot: vocab::tolerance::pinned_pivot()?,
        })
    }
}

/// Every module, and the session bookkeeping that feeds the ones needing yesterday.
#[derive(Clone, Copy, Debug)]
pub struct Evaluator {
    widths: Widths,
    /// The IST day of the bar last folded. `i64::MIN` before the first bar.
    day: i64,
    /// True once at least one bar has been folded into the running session.
    seeded: bool,
    /// The timestamp of the last accepted bar, so a receding one can be refused.
    last_ts: Option<i64>,
    /// The running session's extremes and last close — **including** every bar
    /// folded so far, which is why the fold happens after the emit.
    running_high: i64,
    running_low: i64,
    running_close: i64,
    running_open: i64,
    /// Levels from the last **completed** session. `None` on the first day, where
    /// every position depending on yesterday is correctly false rather than guessed.
    yesterday: Option<DailyLevels>,
    /// Yesterday's shape, for the gap family.
    previous: Option<PreviousSession>,
    prev5: Prev5,
    gap: GapFib,
    trend: TrendState,
    orb: Orb,
    curday: CurDayFib,
    patterns: Patterns,
    session: SessionState,
    vwap: Vwap,
}

// Fixed size. If this assertion ever fails, something in a module started growing
// with the number of bars and the constant-space claim is no longer true.
const _: () = assert!(core::mem::size_of::<Evaluator>() <= 1792);

impl Evaluator {
    /// A fresh evaluator.
    ///
    /// `availability` is the VWAP verdict for the **whole run**, decided by the
    /// caller before the first bar and never revisited. A run must not change its
    /// mind halfway about whether bit 52 means "below VWAP" or "no opinion" — see
    /// [`crate::vwap`]. On index spot it is [`Availability::Absent`], which makes all
    /// twenty VWAP positions false for the run rather than emitting a plain average
    /// wearing a VWAP label.
    #[must_use]
    pub const fn new(widths: Widths, availability: Availability, thresholds: Thresholds) -> Self {
        Self {
            widths,
            day: i64::MIN,
            seeded: false,
            last_ts: None,
            running_high: 0,
            running_low: 0,
            running_close: 0,
            running_open: 0,
            yesterday: None,
            previous: None,
            prev5: Prev5::new(),
            gap: GapFib::new(),
            trend: TrendState::new(crate::trend::TrendThresholds::CLASSICAL),
            orb: Orb::new(),
            curday: CurDayFib::new(),
            patterns: Patterns::new(thresholds),
            session: SessionState::new(
                crate::session::DayWindows::CLASSICAL,
                crate::session::ShapeLimits::CLASSICAL,
            ),
            vwap: Vwap::for_slice(availability),
        }
    }

    /// Every condition bit this crate can compute for `bar`.
    ///
    /// # Errors
    ///
    /// [`Corrupt::HighBelowLow`] or [`Corrupt::RangeOverflows`] for a record that is
    /// not a bar. Checked **once, here**, before any module sees it: seven modules
    /// each re-deriving the same refusal is seven chances to disagree about what a
    /// bar is, and a partially-evaluated bar is worse than a refused one.
    pub fn step(&mut self, bar: &Candle) -> Result<ConditionMask, Corrupt> {
        if bar.high < bar.low {
            return Err(Corrupt::HighBelowLow);
        }
        if bar.high.checked_sub(bar.low).is_none() {
            return Err(Corrupt::RangeOverflows);
        }
        // Containment, and it was missing. `open` and `close` are ticks, so both must
        // lie inside the extremes; when they do not, every wick length goes negative
        // and every `_at_most` shape predicate is satisfied by a `<=` against a
        // positive bound. The result is a bar labelled "no wicks" because its wicks
        // are impossible. `store::format::Bar::ohlc_is_sane` already refuses exactly
        // this — the check was absent here because a doc comment in this crate said
        // that function checked field order only, and it does not.
        if bar.open > bar.high || bar.close > bar.high || bar.open < bar.low || bar.close < bar.low
        {
            return Err(Corrupt::PriceOutsideRange);
        }
        // Ordering, and it was also missing. The rollover below triggers on
        // `today != self.day` — an inequality — so a receding timestamp closes the
        // books on a LATER session and installs it as `yesterday`. Every level the
        // daily, prev-day and prev-5 families emit would then come from a session
        // after the bar being evaluated: look-ahead, which §3 rule 7 requires be
        // prevented by a mechanism and not by review. A monotonic feed is the store's
        // guarantee, but this crate no longer depends on the store, and a live
        // consumer does not control its feed.
        if let Some(last) = self.last_ts
            && bar.ts_micros <= last
        {
            return Err(Corrupt::TimestampNotIncreasing);
        }

        // ── roll over first: which session we are in describes THIS bar ─────────
        let today = crate::ist_day(bar.ts_micros);
        if today != self.day {
            if self.seeded {
                self.close_the_books();
            }
            self.day = today;
            self.seeded = false;
        }

        // ── emit: every module, unioned ─────────────────────────────────────────
        let mut mask = ConditionMask::ZERO;
        mask = mask.union(&self.curday.step(bar, self.widths.fib)?);
        mask = mask.union(&self.patterns.step(bar)?);
        mask = mask.union(&self.orb.step(bar, self.widths.fib)?);
        mask = mask.union(&self.session.step(bar, self.previous, self.widths.fib)?);
        mask = mask.union(&self.prev5.bits(bar.close, self.widths.fib));
        mask = mask.union(&self.gap.step(bar, self.widths.fib)?);
        mask = mask.union(&self.trend.step(bar, self.widths.fib)?);
        if let Some(levels) = self.yesterday.as_ref() {
            mask = mask.union(&crate::daily::bits(levels, bar.close, self.widths.pivot));
            mask = mask.union(&crate::fib::prev_day_bits(
                levels,
                bar.close,
                self.widths.fib,
            ));
        }
        // VWAP has one refusal the shared check cannot make — a negative volume — and
        // one the shared check makes differently, the accumulator ceiling. Both used to
        // be discarded by `if let Ok(v) = ...`, which is the §4 fallback that hides a
        // failure in its purest form: a bar VWAP called corruption came back as a
        // SUCCESSFUL evaluation with the whole twenty-position family silently absent,
        // indistinguishable from a run that legitimately has no volume. Three separate
        // review dimensions found it independently.
        //
        // A refused bar is now a refused bar. `Refused::Corrupt` is already what
        // `Candle::check` returns and cannot fire here; the other two are mapped so the
        // caller learns which, rather than learning nothing.
        match self.vwap.step(bar, self.widths.fib) {
            Ok(v) => mask = mask.union(&v),
            Err(crate::vwap::Refused::Corrupt(why)) => return Err(why),
            Err(crate::vwap::Refused::AccumulatorTooLarge) => {
                return Err(Corrupt::AccumulatorTooLarge);
            }
        }

        // ── fold last: the day's extremes are an anchor and must exclude the bar ─
        if self.seeded {
            if bar.high > self.running_high {
                self.running_high = bar.high;
            }
            if bar.low < self.running_low {
                self.running_low = bar.low;
            }
        } else {
            self.running_high = bar.high;
            self.running_low = bar.low;
            self.running_open = bar.open;
            self.seeded = true;
        }
        self.running_close = bar.close;
        self.last_ts = Some(bar.ts_micros);

        // Nothing outside the live vocabulary may reach a caller: a retired or void
        // position that escaped would be swept as a real condition.
        Ok(vocab::table::only_live(mask))
    }

    /// Hand the finished session to the families that need yesterday.
    fn close_the_books(&mut self) {
        self.prev5
            .push_completed_session(self.running_high, self.running_low);
        // A session whose levels are unusable leaves `yesterday` as it was rather
        // than half-updating it: stale-and-consistent beats fresh-and-partial, and
        // `DailyLevels` already refuses rather than inventing a ladder.
        if let Ok(levels) = DailyLevels::from_previous_session(
            self.running_high,
            self.running_low,
            self.running_close,
        ) {
            self.yesterday = Some(levels);
        }
        self.previous = Some(PreviousSession {
            high: self.running_high,
            low: self.running_low,
            close: self.running_close,
        });
    }

    /// Completed sessions folded so far, saturating at the `Prev5` window.
    #[must_use]
    pub const fn sessions_completed(&self) -> usize {
        self.prev5.filled()
    }

    /// Whether the families that need yesterday can answer yet.
    #[must_use]
    pub const fn has_yesterday(&self) -> bool {
        self.yesterday.is_some()
    }

    /// Every position this evaluator can ever set.
    ///
    /// The union of nine sources' own `positions()` — eight modules and the current-day
    /// Fibonacci rung range — so it cannot drift from them: adding a position to a
    /// module adds it here. 234 positions today, which is every live bit in the table.
    #[must_use]
    pub fn positions() -> Vec<u16> {
        let mut all: Vec<u16> = Vec::new();
        all.extend(crate::daily::positions());
        all.extend(crate::pattern::positions());
        all.extend(crate::session::positions());
        all.extend(crate::orb::positions());
        all.extend(crate::fib::positions());
        all.extend(crate::vwap::positions());
        all.extend(crate::gap::GapFib::positions());
        all.extend(crate::trend::TrendState::positions());
        let last = crate::CURDAY_FIRST
            .saturating_add(u16::try_from(crate::CURDAY_RUNGS.len()).unwrap_or(0))
            .saturating_sub(1);
        all.extend(crate::CURDAY_FIRST..=last);
        all.sort_unstable();
        all.dedup();
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IST_OPEN_UTC_MICROS: i64 = (555 - 330) * 60 * 1_000_000;
    const DAY_MICROS: i64 = 24 * 60 * 60 * 1_000_000;

    fn widths() -> Widths {
        let Ok(w) = Widths::pinned() else {
            unreachable!("both pinned widths are valid")
        };
        w
    }

    fn fresh() -> Evaluator {
        Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL)
    }

    /// A synthetic session. Prices wander so bars differ from one another.
    fn session(day: i64, n: i64) -> Vec<Candle> {
        (0..n)
            .map(|m| {
                let mid = 2_500_000 + ((m * 137) % 811 - 405) * 7;
                Candle {
                    ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * 60 * 1_000_000,
                    open: mid,
                    high: mid + 300 + (m % 11) * 20,
                    low: mid - 300 - (m % 7) * 20,
                    close: mid + ((m % 5) - 2) * 40,
                    volume: 0,
                    open_interest: i64::MIN,
                }
            })
            .collect()
    }

    /// The evaluator's reach is exactly the union of its modules'.
    ///
    /// Derived rather than listed, so adding a position to any module adds it here
    /// and a hand-maintained list cannot drift.
    #[test]
    fn the_position_set_is_the_union_of_the_modules() {
        let all = Evaluator::positions();
        assert_eq!(
            all.len(),
            234,
            "234 positions are computable — every live one"
        );
        let mut sorted = all.clone();
        sorted.sort_unstable();
        assert_eq!(all, sorted, "positions() must be sorted");
        let mut deduped = all.clone();
        deduped.dedup();
        assert_eq!(
            all.len(),
            deduped.len(),
            "no module may claim another's bit"
        );
        for p in &all {
            assert!(
                vocab::table::definition(*p).is_some(),
                "position {p} is not in the vocabulary"
            );
        }
    }

    /// Nothing outside the live vocabulary ever reaches a caller.
    ///
    /// A retired (6, 19, 25) or void (39 of them) position escaping would be swept
    /// as a real condition, which §3.8 forbids.
    #[test]
    fn only_live_positions_are_ever_emitted() {
        let mut e = fresh();
        for day in 0..3_i64 {
            for bar in &session(20_400 + day, 40) {
                let Ok(mask) = e.step(bar) else {
                    unreachable!("a sane bar")
                };
                assert_eq!(
                    vocab::table::only_live(mask),
                    mask,
                    "a non-live position escaped"
                );
                let claimed = Evaluator::positions();
                let mut bit: u32 = 0;
                while bit < vocab::ConditionMask::BITS {
                    if mask.get(bit) {
                        let Ok(index) = u16::try_from(bit) else {
                            unreachable!("bit < 384")
                        };
                        assert!(
                            claimed.contains(&index),
                            "bit {bit} was set but no module claims it"
                        );
                    }
                    bit = bit.saturating_add(1);
                }
            }
        }
    }

    /// The families needing yesterday are silent on day one and speak on day two.
    ///
    /// This is the bookkeeping this module exists for. Before it, nothing closed the
    /// books on a session, so `DailyLevels` and `Prev5` could never be fed at all.
    #[test]
    fn yesterday_arrives_only_after_a_session_completes() {
        let mut e = fresh();
        assert!(!e.has_yesterday(), "no session has completed yet");
        assert_eq!(e.sessions_completed(), 0);

        for bar in &session(20_500, 30) {
            let Ok(_) = e.step(bar) else {
                unreachable!("a sane bar")
            };
        }
        assert!(!e.has_yesterday(), "the first session has not ended yet");

        // The first bar of the next IST day closes the books on the first.
        let Some(next) = session(20_501, 1).first().copied() else {
            unreachable!("one bar")
        };
        let Ok(_) = e.step(&next) else {
            unreachable!("a sane bar")
        };
        assert!(e.has_yesterday(), "day two must see day one's levels");
        assert_eq!(e.sessions_completed(), 1);
    }

    /// Prev5 fills over five sessions and then holds.
    #[test]
    fn five_sessions_fill_the_prev5_window() {
        let mut e = fresh();
        for day in 0..8_i64 {
            for bar in &session(20_600 + day, 5) {
                let Ok(_) = e.step(bar) else {
                    unreachable!("a sane bar")
                };
            }
        }
        assert_eq!(
            e.sessions_completed(),
            5,
            "the window saturates at five and does not grow"
        );
    }

    /// A corrupt record is refused once, centrally, before any module sees it.
    #[test]
    fn a_corrupt_record_is_refused_before_any_module_runs() {
        let mut e = fresh();
        let inverted = Candle {
            ts_micros: 0,
            open: 2_500_000,
            high: 2_499_000,
            low: 2_501_000,
            close: 2_500_000,
            volume: 0,
            open_interest: i64::MIN,
        };
        assert_eq!(e.step(&inverted), Err(Corrupt::HighBelowLow));
        assert!(!e.has_yesterday(), "a refused bar changed no state");
        assert_eq!(e.sessions_completed(), 0);

        let straddling = Candle {
            ts_micros: 0,
            open: 0,
            high: i64::MAX,
            low: i64::MIN,
            close: 0,
            volume: 0,
            open_interest: i64::MIN,
        };
        assert_eq!(e.step(&straddling), Err(Corrupt::RangeOverflows));
    }

    /// Two runs over one slice agree exactly — §3.5, measured rather than asserted.
    #[test]
    fn two_runs_agree_byte_for_byte() {
        let bars: Vec<Candle> = (0..4_i64).flat_map(|d| session(20_700 + d, 25)).collect();
        let once: Vec<ConditionMask> = {
            let mut e = fresh();
            bars.iter().filter_map(|b| e.step(b).ok()).collect()
        };
        for _ in 0..3 {
            let again: Vec<ConditionMask> = {
                let mut e = fresh();
                bars.iter().filter_map(|b| e.step(b).ok()).collect()
            };
            assert_eq!(again, once, "a rerun disagreed with the first run");
        }
    }

    /// With `Availability::Absent`, not one of the twenty VWAP positions is set.
    #[test]
    fn an_absent_volume_verdict_silences_the_whole_vwap_family() {
        let mut e = fresh();
        for bar in &session(20_800, 60) {
            let Ok(mask) = e.step(bar) else {
                unreachable!("a sane bar")
            };
            for p in crate::vwap::positions() {
                assert!(!mask.get(u32::from(p)), "VWAP position {p} was set");
            }
        }
    }
}
