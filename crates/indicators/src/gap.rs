//! Fibonacci over the overnight gap leg.
//!
//! **11 vocabulary positions**, 132–142.
//!
//! # The source, and why it had to be read off the cell formulas
//!
//! `docs/09-design-sources.md` §4 records this one. It is not a Pine script — it
//! arrived as a spreadsheet with `GAPUP` and `GAPDOWN` sheets, and the rule was read
//! out of the **cell formulas rather than the row labels, because the two disagree**.
//!
//! | | Gap up | Gap down |
//! |---|---|---|
//! | `X1` | previous day's **last 3-minute candle HIGH** | previous day's **last 3-minute candle LOW** |
//! | `X2` | today's **first 3-minute candle HIGH** | today's **first 3-minute candle LOW** |
//! | `X3` | `X2 - X1` | `X1 - X2` |
//! | `X4` | `X3 / 2` | `X3 / 2` |
//! | `X5` | `X2 - X4` | `X2 + X4` |
//!
//! # The two sheets are one formula
//!
//! Written out, both branches reduce to the same expression, and this was checked
//! before any of it was implemented rather than assumed:
//!
//! ```text
//! up:      X2 - p·(X2 - X1)/1000
//! down:    X2 + p·(X1 - X2)/1000
//! both:    X2 + p·(X1 - X2)/1000      <- the up form is this with the sign folded in
//! ```
//!
//! So [`GapLeg::level`] has one body for both directions. Rung 0 sits on `X2`
//! (today's opening extreme) and rung 1000 on `X1` (yesterday's closing extreme);
//! the extension rungs run past `X1`. At `p = 500` the level is `(X1 + X2)/2`
//! exactly, which is the sheet's own `X5` — and §4 records that `X5 == (X1+X2)/2`
//! held exactly across all six worked examples in the file. The eleven-rung ladder
//! contains the sheet's single midpoint as a strict superset.
//!
//! # Three-minute candles, from one-minute bars
//!
//! The source specifies **3-minute** candles at both ends, and the engine's own rung
//! is one minute. Rather than require a folded feed, this module folds three
//! one-minute bars itself: a three-slot tail of the current session gives yesterday's
//! last 3-minute candle at the day boundary, and the first three bars of the new
//! session give today's first. Both are fixed-size, so the cost is O(1) per bar.
//! Measured by `C-I-01`, in `crates/indicators/benches/ratio.rs`.
//!
//! On a rung already 3 minutes or longer a single bar *is* the candle, and the fold
//! degenerates correctly — the first bar seeds it and the next two, if the session
//! has them, only widen the extremes.
//!
//! # What it refuses, and why refusing is the answer
//!
//! `X3` is a **length** and the sheet's arithmetic requires it positive. So a session
//! that did not gap has no leg: the eleven rungs would collapse onto one price and
//! setting eleven bits at once for a single price is not a ladder, it is noise. The
//! table's own comment at position 132 says exactly this. [`GapFib`] holds `None`
//! until a real gap is established and emits nothing until then.
//!
//! The direction test is the sheet's own requirement that `X3 > 0`, so it is derived
//! rather than invented: a gap **up** needs today's first-3 high above yesterday's
//! last-3 high, and a gap **down** needs yesterday's last-3 low above today's
//! first-3 low. A session satisfying neither is not a gap.

use crate::Candle;
use crate::evaluator::Calendar;
use vocab::{ConditionMask, Tolerance};

/// The first vocabulary position of the gap-Fibonacci group.
pub const GAP_FIRST: u16 = 132;

/// The rungs, in thousandths, in the order positions 132–142 carry them.
///
/// Identical to the current-day ladder: the same seven retracements and four
/// extensions. A different ladder here would mean two "0.618"s in one vocabulary
/// that were not the same fraction.
pub const GAP_RUNGS: [i32; 11] = [0, 236, 382, 500, 618, 786, 1000, 1272, 1618, 2000, 2618];

/// The last position, derived so it cannot disagree with the ladder's length.
///
/// `saturating_sub` on the count rather than an `as` cast: `clippy::cast_possible_
/// truncation` is denied in this workspace and the lint is right — a cast here
/// would be correct only because the ladder happens to be short, which is
/// proof-by-adjacency and stops holding after an edit.
pub const GAP_LAST: u16 = {
    let count = GAP_RUNGS.len();
    assert!(count <= u16::MAX as usize, "the ladder outgrew a u16 index");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the assertion above is a const assertion: this cannot truncate"
    )]
    let last = GAP_FIRST + (count as u16) - 1;
    last
};

const _: () = assert!(GAP_LAST == 142, "the gap group is positions 132..=142");

/// How many one-minute bars make the source's 3-minute candle.
const CANDLE_MINUTES: usize = 3;

/// Which way the market gapped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Today opened above yesterday's close region. The sheet's `GAPUP`.
    Up,
    /// Today opened below it. The sheet's `GAPDOWN`.
    Down,
}

/// The two ends of a real overnight gap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GapLeg {
    /// `X1` — yesterday's last 3-minute candle, high for an up gap, low for a down.
    pub x1: i64,
    /// `X2` — today's first 3-minute candle, same field as `x1`.
    pub x2: i64,
    /// Which sheet applies.
    pub direction: Direction,
}

impl GapLeg {
    /// The gap's length, `X3`, or `None` when it does not fit `i64`. Positive whenever
    /// it exists: [`GapFib::establish`] builds a leg only when it is.
    ///
    /// `checked_sub` and an `Option`, exactly as [`Self::level`] answers one. `X1` and
    /// `X2` come from two DIFFERENT sessions, and `Candle::check` reads one record — a
    /// flat bar at either end of the type has a zero range and is accepted — so the leg
    /// can span 1.84e19 where neither bar does. `saturating_sub` pinned that at
    /// `i64::MAX`, and every rung is a fraction of this length: rung 0 is `X2` at any
    /// length, so position 132 still fired on today's opening extreme while the ten rungs
    /// above it stood at prices of a gap half the real one. A saturated length is a
    /// plausible number for a gap the market never made, which §4 bans, and it is the
    /// same objection [`Self::level`] answers with `None` rather than a clamp.
    #[must_use]
    pub const fn length(&self) -> Option<i64> {
        match self.direction {
            Direction::Up => self.x2.checked_sub(self.x1),
            Direction::Down => self.x1.checked_sub(self.x2),
        }
    }

    /// Rung `p`'s price, in paisa. `None` if it leaves `i64`.
    ///
    /// One body for both directions — see the module documentation for the algebra.
    /// `div_euclid` and not `/`: truncation toward zero would round a level on the
    /// wrong side of the anchor for a negative product, and the two sheets differ
    /// exactly in that sign.
    ///
    /// `None` rather than a clamp: §7 reserves `i64::MIN` for the open-interest null,
    /// and pinning an out-of-range level onto it would put a sentinel where a price
    /// goes.
    #[must_use]
    pub fn level(&self, p: i32) -> Option<i64> {
        let x1 = i128::from(self.x1);
        let x2 = i128::from(self.x2);
        let step = (i128::from(p) * (x1 - x2)).div_euclid(1000);
        i64::try_from(x2 + step).ok()
    }

    /// The sheet's `X5`, the gap midpoint — rung 500 of this ladder.
    ///
    /// Present because it is the one value the source states directly and the only
    /// one its six worked examples pin. If the ladder is right, this equals
    /// `(X1 + X2)/2`, and a test asserts it.
    #[must_use]
    pub fn midpoint(&self) -> Option<i64> {
        self.level(500)
    }
}

/// The gap ladder for one session, and the bookkeeping that establishes it.
///
/// Fixed size: a three-slot tail, a three-bar accumulator, and the leg. Nothing
/// grows with the number of bars fed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GapFib {
    /// The IST day currently being folded. `i64::MIN` before the first bar.
    day: i64,
    /// The current session's rolling last-three extremes, newest folded last.
    tail: [Option<(i64, i64)>; CANDLE_MINUTES],
    /// Where the next tail entry goes.
    tail_next: usize,
    /// Yesterday's last 3-minute candle, `(high, low)`.
    yesterday: Option<(i64, i64)>,
    /// Today's first 3-minute candle while it is still forming.
    today_high: i64,
    today_low: i64,
    /// Bars folded into today's first candle, saturating at [`CANDLE_MINUTES`].
    today_bars: usize,
    /// The established leg. `None` until three bars of the new session have arrived
    /// and a real gap was found.
    leg: Option<GapLeg>,
}

const _: () = assert!(core::mem::size_of::<GapFib>() <= 160);

impl Default for GapFib {
    fn default() -> Self {
        Self::new()
    }
}

impl GapFib {
    /// An empty state, before any session.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            day: i64::MIN,
            tail: [None; CANDLE_MINUTES],
            tail_next: 0,
            yesterday: None,
            today_high: 0,
            today_low: 0,
            today_bars: 0,
            leg: None,
        }
    }

    /// The established leg, or `None` while the session has not gapped or has not
    /// yet produced three bars.
    #[must_use]
    pub const fn leg(&self) -> Option<GapLeg> {
        self.leg
    }

    /// The full per-bar step: roll over, emit, then fold.
    ///
    /// The leg is an **anchor** — a reference the current bar is measured against —
    /// so it is fixed once early in the session and never moves. Once established it
    /// cannot depend on the current bar, which is why the emit reads a leg built from
    /// bars strictly before it.
    ///
    /// # Errors
    ///
    /// [`crate::Corrupt`] for a record that is not a bar.
    pub fn step(
        &mut self,
        bar: &Candle,
        tolerance: Tolerance,
        calendar: &Calendar,
    ) -> Result<ConditionMask, crate::Corrupt> {
        // `check_evaluable`, not `check`: this module must refuse exactly what
        // `Evaluator::stepped` refuses or the mask is a mixture of two answers.
        bar.check_evaluable()?;

        let today = crate::ist_day(bar.ts_micros);
        if today != self.day {
            // The session that just ENDED is `self.day`, not `today`. That is the
            // same slip `Evaluator::stepped` names at its own rollover, and asking
            // about the wrong one inverts the fix here exactly as it does there: the
            // Muhurat edge would be handed forward and the regular day after it
            // discarded.
            //
            // Before the first bar `self.day` is `i64::MIN`, which appears in no
            // calendar, so the verdict is "regular" -- and the fold below then finds
            // an empty tail and promotes nothing, so the answer does not matter.
            let ending_was_regular = !calendar.is_non_regular(self.day);
            self.close_the_session(ending_was_regular);
            self.day = today;
        }

        let bits = self.bits(bar.close, tolerance);
        self.fold(bar);
        Ok(bits)
    }

    /// Hand the finished session's last 3-minute candle forward and reset.
    ///
    /// # Why this takes a verdict
    ///
    /// `docs/00-charter.md` states the Muhurat rule in three clauses, and this is
    /// the third: the Muhurat day's own OHLC never enters the previous-day anchor,
    /// the multi-day rolling history, **or the previous-session edge**. D-0110
    /// closed the first two in `Evaluator::close_the_books` and left this one open,
    /// because this module kept its own day cursor and consulted no calendar at
    /// all. A Muhurat session's last three minutes became the next day's gap
    /// anchor, which inverted the leg's direction on the sessions after it.
    ///
    /// `ending_was_regular` decides only whether the edge is HANDED FORWARD. Every
    /// reset below is unconditional, and deliberately: a Muhurat session is still a
    /// session, and leaving its tail, its bar count or its leg in place would carry
    /// it into the next day by a different route than the one this parameter shuts.
    fn close_the_session(&mut self, ending_was_regular: bool) {
        // Fold whatever the tail holds. Fewer than three entries is a short session,
        // and the source's "last 3-minute candle" is then whatever traded — refusing
        // a half-day outright would drop a real gap.
        let folded = self
            .tail
            .iter()
            .flatten()
            .copied()
            .reduce(|(ah, al), (bh, bl)| {
                (if bh > ah { bh } else { ah }, if bl < al { bl } else { al })
            });
        if ending_was_regular && folded.is_some() {
            self.yesterday = folded;
        }
        self.tail = [None; CANDLE_MINUTES];
        self.tail_next = 0;
        self.today_bars = 0;
        self.leg = None;
    }

    /// Fold one bar into the tail and, while it is forming, today's first candle.
    ///
    /// # The absent arm below cannot run, and removing it costs more than it saves
    ///
    /// `tail_next` is 0 in [`GapFib::new`] and in [`GapFib::close_the_session`], and
    /// every fold re-derives it `% CANDLE_MINUTES`, so it is always a valid index into
    /// the three-slot tail and `get_mut` is always `Some`. `cargo llvm-cov` therefore
    /// records the `if let`'s absent arm as a region no passing test can execute — the
    /// same objection the ladder in [`GapFib::bits`] answers by walking rather than
    /// indexing.
    ///
    /// It is deliberately **not** answered the same way here. Dropping the cursor for a
    /// total three-element shift — `let [_, b, c] = self.tail; self.tail = [b, c, new]`
    /// — does remove the arm, and it carries exactly the same values, because
    /// [`GapFib::close_the_session`] folds the slots with a max and a min and never
    /// reads their order. It also shrinks this struct by 16 bytes, which moves
    /// `size_of::<Evaluator>()` from 1664 to 1648 and makes the measurement recorded in
    /// `docs/10-shared-core.md` — and quoted again in a row of `docs/05-decisions.md` —
    /// a number nobody took. §3 rule 6 forbids reporting such a measurement and §3
    /// rule 8 makes the ledger append-only, so the honest trade is one uncovered region
    /// rather than one stale measurement. `only_the_last_three_bars_of_a_session_reach_
    /// tomorrows_leg` pins the behaviour either way, so the choice stays open.
    fn fold(&mut self, bar: &Candle) {
        if let Some(slot) = self.tail.get_mut(self.tail_next) {
            *slot = Some((bar.high, bar.low));
        }
        self.tail_next = self.tail_next.saturating_add(1) % CANDLE_MINUTES;

        if self.today_bars < CANDLE_MINUTES {
            if self.today_bars == 0 {
                self.today_high = bar.high;
                self.today_low = bar.low;
            } else {
                if bar.high > self.today_high {
                    self.today_high = bar.high;
                }
                if bar.low < self.today_low {
                    self.today_low = bar.low;
                }
            }
            self.today_bars = self.today_bars.saturating_add(1);
            if self.today_bars == CANDLE_MINUTES {
                self.leg = self.establish();
            }
        }
    }

    /// Build the leg, or refuse.
    ///
    /// The direction test is the sheet's own requirement that `X3` be positive, so it
    /// is derived from the source rather than chosen: an up gap needs today's first-3
    /// high above yesterday's last-3 high, a down gap needs yesterday's last-3 low
    /// above today's first-3 low. Neither holding means the session did not gap.
    /// # The two tests are not exclusive, and the doc above assumed they were
    ///
    /// "Neither holding means the session did not gap" describes three cases —
    /// up, down, and neither. There is a fourth: today's first-3 candle
    /// strictly ENGULFING yesterday's last-3, where `today_high > y_high` and
    /// `y_low > today_low` both hold.
    ///
    /// Written as two sequential `if`s, that fourth case silently became `Up`,
    /// because `Up` is tested first. Nothing disclosed the tie-break — the
    /// direction it produced was a property of statement order.
    ///
    /// An engulfing open is the OPPOSITE of a gap: price traded continuously
    /// across yesterday's whole range rather than jumping over part of it, so
    /// there is no discontinuity for a leg to measure. `x1`/`x2` would then span
    /// intraday expansion, and every one of the eleven Fibonacci positions
    /// derived from it would be measuring a move that is not the one the sheet
    /// names. Refusing it is what the doc above already promised.
    ///
    /// The bias this removes is directional: the tie always resolved `Up`, so
    /// the up ladder fired on sessions the down ladder never saw.
    fn establish(&self) -> Option<GapLeg> {
        let (y_high, y_low) = self.yesterday?;
        let above = self.today_high > y_high;
        let below = y_low > self.today_low;
        if above && below {
            return None;
        }
        if above {
            return Some(GapLeg {
                x1: y_high,
                x2: self.today_high,
                direction: Direction::Up,
            });
        }
        if below {
            return Some(GapLeg {
                x1: y_low,
                x2: self.today_low,
                direction: Direction::Down,
            });
        }
        None
    }

    /// The eleven positions for one closing price.
    ///
    /// # Cost
    ///
    /// Eleven rungs, each one cross-multiplied band test. A compile-time constant,
    /// which is what makes this O(1) rather than "O(rungs)".
    /// Measured by `C-I-02`, in `crates/indicators/benches/ratio.rs`.
    #[must_use]
    pub fn bits(&self, close: i64, tolerance: Tolerance) -> ConditionMask {
        let mut mask = ConditionMask::ZERO;
        let Some(leg) = self.leg else {
            return mask;
        };
        // A leg whose length leaves `i64` decides nothing — see [`GapLeg::length`]. The
        // whole ladder abstains rather than the individual rungs: the length is the base
        // of every one of them, so there is no rung it is right for.
        let Some(range) = leg.length() else {
            return mask;
        };
        // THE LADDER IS WALKED, NOT INDEXED. This was `while i < GAP_RUNGS.len()`
        // around a `let Some(p) = GAP_RUNGS.get(i) else { break }`, which is an arm
        // that cannot run while the loop condition holds — `cargo llvm-cov` records
        // it as a region no passing test can ever execute, and a region that cannot
        // run is one nobody can be held to. Walking the array removes the arm
        // instead of hiding it. The ladder is still a fixed eleven, so the cost
        // claim above is the same claim.
        for (i, p) in GAP_RUNGS.into_iter().enumerate() {
            let index = GAP_FIRST.saturating_add(u16::try_from(i).unwrap_or(u16::MAX));
            // ONE level, ONE test: `set_near` re-tests with exactly the `covers` used
            // on the line above, so the two cannot disagree. This is the shape the
            // current-day family was corrected to after its own level and its own
            // test were computed separately and drifted apart at the band edge.
            if let Some(level) = leg.level(p)
                && tolerance.covers(close, level, range)
                && let Ok(next) =
                    vocab::table::set_near(mask, index, tolerance, close, level, range)
            {
                mask = next;
            }
        }
        mask
    }

    /// Every position this module can set.
    #[must_use]
    pub const fn positions() -> [u16; 11] {
        [132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142]
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes — a test that \
              cannot panic cannot fail — and the same second reason `crate::tests` \
              gives: `unreachable!` expands to a panic inside THIS crate, so llvm-cov \
              counts a region that no passing run can execute and the coverage floor \
              can never be reached on this file. `.expect` panics inside core, which \
              is not instrumented: the same failure, with the refused value printed \
              beside the message, and no dead region left behind."
)]
mod tests {
    use super::*;

    const IST_OPEN_UTC_MICROS: i64 = (555 - 330) * 60 * 1_000_000;
    const DAY_MICROS: i64 = 24 * 60 * 60 * 1_000_000;

    fn at(day: i64, minute: i64, high: i64, low: i64, close: i64) -> Candle {
        Candle {
            ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + minute * 60 * 1_000_000,
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

    /// Step a bar this fixture calls sane, and hand back the bits it emitted.
    ///
    /// A helper so the call sites below read as one line each, and so a fixture that
    /// this file calls sane while `Candle::check` refuses it names itself once rather
    /// than at every site.
    fn ok(g: &mut GapFib, bar: &Candle) -> ConditionMask {
        g.step(bar, tol(), &Calendar::charter())
            .expect("this fixture bar is sane")
    }

    /// An engulfing open is not a gap in either direction, and used to be `Up`.
    ///
    /// The two direction tests are independent, so a session whose first three
    /// minutes trade both above yesterday's last-3 high AND below its low
    /// satisfies both. Written as sequential `if`s that resolved to whichever
    /// was written first, which was `Up` — a direction decided by statement
    /// order and disclosed nowhere.
    ///
    /// The four cases are asserted together rather than the new one alone: what
    /// is being pinned is that the tests partition, and a test that only
    /// asserted the engulfing case would pass just as well if `Up` and `Down`
    /// had swapped underneath it.
    #[test]
    fn an_engulfing_open_is_neither_direction_rather_than_up() {
        let leg_of = |y_high: i64, y_low: i64, today_high: i64, today_low: i64| {
            GapFib {
                yesterday: Some((y_high, y_low)),
                today_high,
                today_low,
                ..GapFib::default()
            }
            .establish()
            .map(|leg| leg.direction)
        };

        // ENGULFING: above yesterday's high AND below its low. Both tests hold.
        assert_eq!(
            leg_of(2_450_000, 2_400_000, 2_460_000, 2_390_000),
            None,
            "a session that traded across yesterday's whole range did not gap over any of it"
        );
        // Only above.
        assert_eq!(
            leg_of(2_450_000, 2_400_000, 2_460_000, 2_410_000),
            Some(Direction::Up)
        );
        // Only below.
        assert_eq!(
            leg_of(2_450_000, 2_400_000, 2_440_000, 2_390_000),
            Some(Direction::Down)
        );
        // Inside: neither test holds, which the doc already called not a gap.
        assert_eq!(
            leg_of(2_450_000, 2_400_000, 2_440_000, 2_410_000),
            None,
            "an inside session did not gap either"
        );
    }

    /// The two sheets are one formula. Checked against both branches written out
    /// separately, exactly as the spreadsheet has them.
    #[test]
    fn both_sheet_branches_reduce_to_one_expression() {
        for (x1, x2, dir) in [
            (2_400_000_i64, 2_450_000_i64, Direction::Up),
            (2_450_000, 2_400_000, Direction::Down),
        ] {
            let leg = GapLeg {
                x1,
                x2,
                direction: dir,
            };
            for p in GAP_RUNGS {
                // The sheet's own arithmetic, per direction, kept apart on purpose.
                let sheet = match dir {
                    Direction::Up => {
                        i128::from(x2) - (i128::from(p) * (i128::from(x2) - i128::from(x1))) / 1000
                    }
                    Direction::Down => {
                        i128::from(x2) + (i128::from(p) * (i128::from(x1) - i128::from(x2))) / 1000
                    }
                };
                let got = leg.level(p).expect("levels for real prices fit i64");
                assert_eq!(i128::from(got), sheet, "{dir:?} rung {p}");
            }
        }
    }

    /// `X5 == (X1 + X2) / 2` in both directions — the one value the source states
    /// directly, and the only one its six worked examples pin.
    #[test]
    fn the_midpoint_is_the_sheets_x5() {
        for (x1, x2, dir) in [
            (2_400_000_i64, 2_450_000_i64, Direction::Up),
            (2_450_000, 2_400_000, Direction::Down),
        ] {
            let leg = GapLeg {
                x1,
                x2,
                direction: dir,
            };
            assert_eq!(leg.midpoint(), Some(x1.midpoint(x2)), "{dir:?}");
        }
    }

    /// Rung 0 is today's extreme and rung 1000 is yesterday's, both directions.
    #[test]
    fn the_ladder_runs_from_today_to_yesterday() {
        for dir in [Direction::Up, Direction::Down] {
            let leg = GapLeg {
                x1: 2_400_000,
                x2: 2_450_000,
                direction: dir,
            };
            assert_eq!(leg.level(0), Some(2_450_000), "{dir:?} rung 0 is X2");
            assert_eq!(leg.level(1000), Some(2_400_000), "{dir:?} rung 1000 is X1");
        }
    }

    /// The length is positive in both directions — the sheet's `X3` is a length.
    #[test]
    fn the_leg_length_is_positive_in_both_directions() {
        assert_eq!(
            GapLeg {
                x1: 100,
                x2: 140,
                direction: Direction::Up
            }
            .length(),
            Some(40)
        );
        assert_eq!(
            GapLeg {
                x1: 140,
                x2: 100,
                direction: Direction::Down
            }
            .length(),
            Some(40)
        );
    }

    /// A session that did not gap establishes no leg and sets nothing.
    ///
    /// Eleven bits at one price is not a ladder. The table's own comment at position
    /// 132 says the evaluator's job is to abstain.
    #[test]
    fn a_session_that_did_not_gap_sets_nothing() {
        let mut g = GapFib::new();
        // Day one: a plain session.
        for m in 0..6 {
            let _ = ok(&mut g, &at(30_000, m, 2_501_000, 2_499_000, 2_500_000));
        }
        // Day two opens inside yesterday's last candle — no gap either way.
        for m in 0..6 {
            let mask = ok(&mut g, &at(30_001, m, 2_500_500, 2_499_500, 2_500_000));
            assert_eq!(mask, ConditionMask::ZERO, "no gap, no bits");
        }
        assert_eq!(g.leg(), None, "no leg was established");
    }

    /// TOUCHING YESTERDAY'S EXTREME IS NOT GAPPING PAST IT, IN EITHER DIRECTION.
    ///
    /// Both direction tests in [`GapFib::establish`] are strict, and the sheet's own
    /// requirement is that `X3` be POSITIVE. Relax either to `>=` and an open that
    /// exactly touches yesterday's edge establishes a leg of length zero — eleven rungs
    /// all sitting on one price, which is the "eleven bits at one price is not a ladder"
    /// that `a_session_that_did_not_gap_sets_nothing` refuses one case away from here.
    ///
    /// The fixtures above open comfortably clear of the anchor, and `>` and `>=` answer
    /// a clear open identically. The distinction is at one value per side, and each is
    /// asserted twice: ON yesterday's extreme, where there is no leg, and one paisa past
    /// it, where there is.
    #[test]
    fn an_open_that_only_touches_yesterdays_edge_has_not_gapped() {
        // Yesterday's last three bars span 2_499_000..=2_501_000 on both days below.
        let yesterday = |g: &mut GapFib| {
            for m in 0..6 {
                let _ = ok(g, &at(30_000, m, 2_501_000, 2_499_000, 2_500_000));
            }
        };
        // Today's opening candle, three bars of one shape.
        let opening = |g: &mut GapFib, high: i64, low: i64| {
            for m in 0..3 {
                let _ = ok(g, &at(30_001, m, high, low, high.midpoint(low)));
            }
        };

        // Exactly on both edges: neither test is satisfied by a touch.
        let mut flat = GapFib::new();
        yesterday(&mut flat);
        opening(&mut flat, 2_501_000, 2_499_000);
        assert_eq!(
            flat.leg(),
            None,
            "an open touching BOTH of yesterday's edges gapped past neither; a leg \
             here has length zero and puts eleven rungs on one price"
        );

        // One paisa above the high is an up gap, and it is the shortest one there is.
        let mut up = GapFib::new();
        yesterday(&mut up);
        opening(&mut up, 2_501_001, 2_499_000);
        assert_eq!(
            up.leg().map(|l| l.direction),
            Some(Direction::Up),
            "one paisa above yesterday's high is an up gap"
        );

        // One paisa below the low is a down gap. The high stays ON yesterday's high,
        // so the up test is refused by a touch and the down test decides.
        let mut down = GapFib::new();
        yesterday(&mut down);
        opening(&mut down, 2_501_000, 2_498_999);
        assert_eq!(
            down.leg().map(|l| l.direction),
            Some(Direction::Down),
            "one paisa below yesterday's low is a down gap, and the touched high \
             above it must not claim the session first"
        );
    }

    /// THE OPENING CANDLE'S HIGH IS THE MAX OVER ITS THREE BARS, NOT THE LAST BAR'S.
    ///
    /// `today_bars == 0` picks the branch that SEEDS the extremes; every later bar of the
    /// candle takes the branch that compares. Turn that `==` into `!=` and the two swap:
    /// the first bar compares against whatever the previous session left behind, and
    /// bars two and three each RE-SEED — so the candle ends up holding the last bar's
    /// extremes rather than the widest of the three.
    ///
    /// Every fixture above opens with three bars of the same shape, where the last bar's
    /// high IS the max and the two readings agree. This one spikes on the FIRST bar and
    /// settles back, which is what an opening minute does: the gap is 99,000 paisa and
    /// the mutation measures it as 64,000, a leg 35% short with every rung on it moved.
    ///
    /// `>` to `>=` and `<` to `<=` on the two comparisons below are NOT chased and are
    /// recorded in `docs/06-limits.md`: at equality both write the value already held.
    #[test]
    fn the_opening_candles_extreme_is_the_widest_bar_and_not_the_last() {
        let mut g = GapFib::new();
        // Yesterday's last three bars top out at 2_501_000.
        for m in 0..6 {
            let _ = ok(&mut g, &at(30_000, m, 2_501_000, 2_499_000, 2_500_000));
        }
        // Today gaps up and settles back INSIDE its own opening minute.
        for (m, high) in [(0_i64, 2_600_000_i64), (1, 2_570_000), (2, 2_565_000)] {
            let _ = ok(&mut g, &at(30_001, m, high, 2_550_000, 2_560_000));
        }
        assert_eq!(
            g.leg(),
            Some(GapLeg {
                x1: 2_501_000,
                x2: 2_600_000,
                direction: Direction::Up,
            }),
            "the leg was measured to a bar that is not the candle's high -- the \
             opening spike is what the gap is, and settling back does not shrink it"
        );
    }

    /// A real gap up establishes the leg the sheet describes.
    #[test]
    fn a_gap_up_establishes_the_sheets_leg() {
        let mut g = GapFib::new();
        // Yesterday's last three bars top out at 2_500_000.
        for (m, h) in [(0_i64, 2_498_000_i64), (1, 2_499_000), (2, 2_500_000)] {
            let _ = ok(&mut g, &at(30_100, m, h, h - 2_000, h - 500));
        }
        // Today's first three bars top out at 2_520_000 — clear of yesterday.
        for (m, h) in [(0_i64, 2_515_000_i64), (1, 2_518_000), (2, 2_520_000)] {
            let _ = ok(&mut g, &at(30_101, m, h, h - 1_000, h - 200));
        }
        let leg = g.leg().expect("a gap up was established");
        assert_eq!(leg.direction, Direction::Up);
        assert_eq!(leg.x1, 2_500_000, "X1 is yesterday's last-3 HIGH");
        assert_eq!(leg.x2, 2_520_000, "X2 is today's first-3 HIGH");
        assert_eq!(leg.length(), Some(20_000));
        assert_eq!(leg.midpoint(), Some(2_510_000));
    }

    /// A real gap down uses the LOWS, as the `GAPDOWN` sheet does.
    #[test]
    fn a_gap_down_uses_the_lows() {
        let mut g = GapFib::new();
        for (m, l) in [(0_i64, 2_502_000_i64), (1, 2_501_000), (2, 2_500_000)] {
            let _ = ok(&mut g, &at(30_200, m, l + 2_000, l, l + 500));
        }
        for (m, l) in [(0_i64, 2_485_000_i64), (1, 2_482_000), (2, 2_480_000)] {
            let _ = ok(&mut g, &at(30_201, m, l + 1_000, l, l + 200));
        }
        let leg = g.leg().expect("a gap down was established");
        assert_eq!(leg.direction, Direction::Down);
        assert_eq!(leg.x1, 2_500_000, "X1 is yesterday's last-3 LOW");
        assert_eq!(leg.x2, 2_480_000, "X2 is today's first-3 LOW");
        assert_eq!(leg.length(), Some(20_000));
        assert_eq!(leg.midpoint(), Some(2_490_000));
    }

    /// Nothing is emitted before three bars of the new session have arrived.
    ///
    /// The leg is an anchor: it cannot exist until the candle defining it is complete,
    /// and a partial candle would make the level move under the bits.
    #[test]
    fn no_bits_until_the_opening_candle_is_complete() {
        let mut g = GapFib::new();
        for (m, h) in [(0_i64, 2_498_000_i64), (1, 2_499_000), (2, 2_500_000)] {
            let _ = ok(&mut g, &at(30_300, m, h, h - 2_000, h - 500));
        }
        for m in 0..3_i64 {
            // The close was 2_510_000 with a low of 2_519_000 — BELOW its own low.
            // An invalid candle in my own fixture, which `Candle::check` now refuses.
            let bar = at(30_301, m, 2_520_000, 2_519_000, 2_519_500);
            let mask = ok(&mut g, &bar);
            assert_eq!(
                mask,
                ConditionMask::ZERO,
                "bar {m} is inside the opening candle"
            );
            if m < 2 {
                assert_eq!(g.leg(), None, "the leg cannot exist yet");
            }
        }
        assert!(g.leg().is_some(), "the third bar completes the candle");
    }

    /// A close sitting exactly on a rung sets that rung.
    #[test]
    fn a_close_on_the_midpoint_sets_the_midpoint_rung() {
        let mut g = GapFib::new();
        for (m, h) in [(0_i64, 2_498_000_i64), (1, 2_499_000), (2, 2_500_000)] {
            let _ = ok(&mut g, &at(30_400, m, h, h - 2_000, h - 500));
        }
        for (m, h) in [(0_i64, 2_515_000_i64), (1, 2_518_000), (2, 2_520_000)] {
            let _ = ok(&mut g, &at(30_401, m, h, h - 1_000, h - 200));
        }
        // Leg is 2_500_000 .. 2_520_000; the midpoint is 2_510_000, rung 500 = index 135.
        let mask = ok(&mut g, &at(30_401, 3, 2_510_100, 2_509_900, 2_510_000));
        assert!(
            mask.get(135),
            "a close on the midpoint sets rung 0.5 (position 135)"
        );
    }

    /// Every position is live, `Kind::Near`, and inside the declared block.
    #[test]
    fn the_positions_match_the_vocabulary() {
        let all = GapFib::positions();
        assert_eq!(all.len(), GAP_RUNGS.len(), "one position per rung");
        for (i, index) in all.iter().enumerate() {
            let expect = GAP_FIRST.saturating_add(u16::try_from(i).unwrap_or(u16::MAX));
            assert_eq!(*index, expect, "positions are contiguous from GAP_FIRST");
            let def = vocab::table::definition(*index).expect("132..=142 are allocated");
            assert_eq!(def.kind, vocab::Kind::Near, "position {index} needs a band");
            assert!(
                def.name.starts_with("near_fib_gap_"),
                "position {index} is named {} and should be a gap rung",
                def.name
            );
        }
        assert_eq!(*all.last().unwrap_or(&0), GAP_LAST);
    }

    /// A corrupt record is refused and changes no state.
    #[test]
    fn a_corrupt_record_is_refused() {
        let mut g = GapFib::new();
        let inverted = Candle {
            ts_micros: 0,
            open: 100,
            high: 90,
            low: 110,
            close: 100,
            volume: 0,
            open_interest: i64::MIN,
        };
        assert_eq!(
            g.step(&inverted, tol(), &Calendar::charter()),
            Err(crate::Corrupt::HighBelowLow)
        );
        assert_eq!(g.leg(), None);
    }

    /// The charter's THIRD Muhurat clause, which D-0110 left open.
    ///
    /// `docs/00-charter.md`: the Muhurat day's own OHLC never enters the previous-day
    /// anchor, the multi-day rolling history, "or the previous-session edge". This
    /// module IS the previous-session edge, and it consulted no calendar at all.
    ///
    /// Two `GapFib`s, the same bars, and the only difference between them is which
    /// calendar they were handed. That second half is what gives this test teeth: a
    /// "fix" that simply stopped promoting anything would satisfy the first
    /// assertion and fail the second.
    #[test]
    fn a_muhurat_sessions_edge_never_becomes_the_next_days_anchor() {
        /// 2025-10-21, the afternoon Muhurat session -- the one the pull's
        /// 09:15-15:30 window admits, and the fifth entry of
        /// `CHARTER_NON_REGULAR_IST_DAYS`.
        const MUHURAT: i64 = 20_382;

        let regular = [
            at(MUHURAT - 1, 0, 2_501_000, 2_499_000, 2_500_000),
            at(MUHURAT - 1, 1, 2_502_000, 2_498_000, 2_500_000),
            at(MUHURAT - 1, 2, 2_503_000, 2_497_000, 2_500_000),
        ];
        let muhurat = [
            at(MUHURAT, 0, 2_900_000, 2_800_000, 2_850_000),
            at(MUHURAT, 1, 2_901_000, 2_799_000, 2_850_000),
            at(MUHURAT, 2, 2_902_000, 2_798_000, 2_850_000),
        ];
        let after = at(MUHURAT + 1, 0, 2_600_000, 2_590_000, 2_595_000);

        let edge_after = |calendar: &Calendar| {
            let mut g = GapFib::new();
            for bar in regular.iter().chain(muhurat.iter()).chain([&after]) {
                g.step(bar, tol(), calendar)
                    .expect("every fixture bar here is sane");
            }
            g.yesterday
        };

        assert_eq!(
            edge_after(&Calendar::charter()),
            Some((2_503_000, 2_497_000)),
            "the anchor for the day after a Muhurat session must be the last REGULAR \
             session's edge"
        );
        assert_eq!(
            edge_after(&Calendar::all_regular()),
            Some((2_902_000, 2_798_000)),
            "told every day is regular, the Muhurat edge IS promoted -- so the calendar \
             is what prevents it, rather than this gate having stopped promoting at all"
        );
    }

    /// A one-bar session hands forward its own bar and nothing older.
    ///
    /// `close_the_session` clears the tail unconditionally and nothing pinned that: a
    /// mutation deleting `self.tail = [None; CANDLE_MINUTES];` survived the entire
    /// suite, because every other fixture gives each session at least
    /// `CANDLE_MINUTES` bars, which overwrites the whole ring and hides the staleness.
    /// A single-bar session is the shortest input that can see it.
    #[test]
    fn a_one_bar_session_hands_forward_only_its_own_bar() {
        let mut g = GapFib::new();
        for m in 0..3 {
            ok(
                &mut g,
                &at(
                    1,
                    m,
                    2_510_000 + m * 1_000,
                    2_490_000 - m * 1_000,
                    2_500_000,
                ),
            );
        }
        // Day 2 trades once. Day 3 opening is what closes it.
        ok(&mut g, &at(2, 0, 2_600_000, 2_599_000, 2_599_500));
        ok(&mut g, &at(3, 0, 2_700_000, 2_699_000, 2_699_500));

        assert_eq!(
            g.yesterday,
            Some((2_600_000, 2_599_000)),
            "the anchor is the one-bar session's own edge; day 1's tail must have been \
             cleared when day 1 closed"
        );
    }

    /// `Default` must be `new`, or a caller writing `GapFib::default()` starts a run in
    /// a state nothing in this file reasons about.
    ///
    /// `evaluator` builds its members by name, so nothing in the workspace had ever
    /// called this impl — and the field that decides everything, `day`, holds §7's
    /// `i64::MIN` null until the first bar. A `Default` that drifted to
    /// `Self { day: 0, .. }` would make the epoch's own session look already open, and
    /// the first real bar would then be folded into it instead of opening a session:
    /// no rollover, so no `yesterday`, so no leg all day and eleven positions silently
    /// absent.
    #[test]
    fn the_default_state_is_the_empty_one() {
        assert_eq!(
            GapFib::default(),
            GapFib::new(),
            "`Default` drifted from `new`"
        );
        assert_eq!(
            GapFib::default().leg(),
            None,
            "a defaulted state named a leg before its first bar"
        );
    }

    /// A leg whose length does not fit `i64` sets NOTHING, rather than laying eleven
    /// rungs over a saturated one.
    ///
    /// `X1` and `X2` come from different sessions, and each bar below is legal on its own
    /// — `Candle::check` reads one record, and a flat bar at either end of the type has a
    /// zero range — so the LEG spans 1.84e19 where no record does. Under `saturating_sub`
    /// that became `i64::MAX`, and every rung is a fraction of it: rung 0 is `X2` whatever
    /// the length, so position 132 still fired at today's opening extreme while the ten
    /// rungs above it sat at prices of a gap half the real one. Nothing refused.
    #[test]
    fn a_leg_whose_length_leaves_the_type_sets_nothing() {
        const HI: i64 = 9_200_000_000_000_000_000;
        const LO: i64 = -9_200_000_000_000_000_000;

        let mut g = GapFib::new();
        for m in 0..3 {
            let _ = ok(&mut g, &at(30_700, m, LO, LO, LO));
        }
        for m in 0..3 {
            let _ = ok(&mut g, &at(30_701, m, HI, HI, HI));
        }
        let leg = g
            .leg()
            .expect("today's first-3 high cleared yesterday's last-3 high");
        assert_eq!(leg.direction, Direction::Up);
        assert_eq!(leg.x1, LO, "X1 is yesterday's last-3 high");
        assert_eq!(leg.x2, HI, "X2 is today's first-3 high");
        assert_eq!(
            leg.length(),
            None,
            "a leg spanning 1.84e19 reported a length that fits i64, so ten rungs were \
             laid out over a gap the market never made"
        );
        // A close on `X2` is rung 0 of this ladder at any length, which is why it is the
        // close that shows a saturated length firing a bit.
        let mask = ok(&mut g, &at(30_701, 3, HI, HI, HI));
        assert_eq!(
            mask,
            ConditionMask::ZERO,
            "a leg whose length leaves i64 decided a rung on today's opening extreme"
        );

        // And a gap spanning half as much — 9.2e18, which DOES fit — fires rung 0 on the
        // same close from the same code, so the empty mask above is the length leaving
        // the type and not a ladder that never emits at this magnitude.
        let mut fits = GapFib::new();
        for m in 0..3 {
            let _ = ok(&mut fits, &at(30_800, m, LO / 2, LO / 2, LO / 2));
        }
        for m in 0..3 {
            let _ = ok(&mut fits, &at(30_801, m, HI / 2, HI / 2, HI / 2));
        }
        assert_eq!(
            fits.leg().map(|leg| leg.length()),
            Some(Some(HI / 2 - LO / 2)),
            "a representable 9.2e18 gap did not report its own length"
        );
        let mask = ok(&mut fits, &at(30_801, 3, HI / 2, HI / 2, HI / 2));
        assert!(
            mask.get(u32::from(GAP_FIRST)),
            "a representable gap did not fire rung 0 on a close sitting exactly on X2"
        );
    }

    /// `X1` is the last 3-minute candle, **not** the session's own extreme.
    ///
    /// The tail is three slots and a real session is hundreds of bars, so the source's
    /// "previous day's last 3-minute candle" is only what the tail still holds at the
    /// bell. Nothing pinned which three bars those are: **every other test in this file
    /// feeds a session exactly three bars**, and on a three-bar session "the last three"
    /// and "the whole session" are the same candle, so a tail that never dropped a slot,
    /// or kept only the newest bar, passed all of them. Either mistake anchors all
    /// eleven rungs on a candle the source does not name, and [`GapFib::fold`]'s cursor
    /// wraps `% CANDLE_MINUTES` to prevent exactly that.
    ///
    /// Both fixtures below are chosen so the three answers differ. Yesterday's last
    /// three bars are m3, m4 and m5: their high, 2,500,000, comes from the **oldest**
    /// of the three and their low, 2,450,000, from the **middle** one. The session's own
    /// extremes, 2,600,000 and 2,400,000, are m0's — outside the tail. So keeping the
    /// whole session, or only the newest bar, fails on the value, not merely on a sign.
    #[test]
    fn only_the_last_three_bars_of_a_session_reach_tomorrows_leg() {
        // (minute, high, low, close). m0 spikes, and the cursor must have overwritten
        // its slot before the bell.
        let yesterday = [
            (0_i64, 2_600_000_i64, 2_400_000_i64, 2_450_000_i64),
            (1, 2_450_000, 2_440_000, 2_445_000),
            (2, 2_455_000, 2_445_000, 2_450_000),
            (3, 2_500_000, 2_470_000, 2_490_000),
            (4, 2_460_000, 2_450_000, 2_455_000),
            (5, 2_465_000, 2_455_000, 2_460_000),
        ];

        // Today clears 2,500,000 — yesterday's last-3 HIGH — so the gap is up and X1
        // is that high. It does NOT clear the session high of 2,600,000.
        let mut up = GapFib::new();
        for (m, h, l, c) in yesterday {
            let _ = ok(&mut up, &at(30_600, m, h, l, c));
        }
        for (m, h, l, c) in [
            (0_i64, 2_515_000_i64, 2_512_000_i64, 2_514_000_i64),
            (1, 2_518_000, 2_514_000, 2_516_000),
            (2, 2_520_000, 2_517_000, 2_519_000),
        ] {
            let _ = ok(&mut up, &at(30_601, m, h, l, c));
        }
        let leg = up
            .leg()
            .expect("today's first-3 high cleared the last-3 high");
        assert_eq!(leg.direction, Direction::Up);
        assert_eq!(
            leg.x1, 2_500_000,
            "X1 must be the last-3 HIGH, which is m3's — not m5's 2,465,000 and not \
             the session high of 2,600,000"
        );
        assert_eq!(leg.x2, 2_520_000, "X2 is today's first-3 HIGH");
        assert_eq!(leg.length(), Some(20_000));

        // Same yesterday, and today undercuts 2,450,000 — the last-3 LOW, which is
        // m4's. It does not undercut the session low of 2,400,000, and its high stays
        // under the last-3 high, so the up test cannot fire first.
        let mut down = GapFib::new();
        for (m, h, l, c) in yesterday {
            let _ = ok(&mut down, &at(30_600, m, h, l, c));
        }
        for (m, h, l, c) in [
            (0_i64, 2_437_000_i64, 2_435_000_i64, 2_436_000_i64),
            (1, 2_434_000, 2_430_000, 2_432_000),
            (2, 2_430_000, 2_425_000, 2_427_000),
        ] {
            let _ = ok(&mut down, &at(30_601, m, h, l, c));
        }
        let leg = down
            .leg()
            .expect("today's first-3 low undercut the last-3 low");
        assert_eq!(leg.direction, Direction::Down);
        assert_eq!(
            leg.x1, 2_450_000,
            "X1 must be the last-3 LOW, which is m4's — not m5's 2,455,000 and not \
             the session low of 2,400,000"
        );
        assert_eq!(leg.x2, 2_425_000, "X2 is today's first-3 LOW");
        assert_eq!(leg.length(), Some(25_000));
    }
}
