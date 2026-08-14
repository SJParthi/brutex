//! One sweep, end to end: bars in, condition-bit column, Apriori ladder, result.
//!
//! # The join that gate 22 makes impossible anywhere else
//!
//! `crates/indicators` turns a [`Candle`] into a [`ConditionMask`].
//! `crates/engine` walks the combination ladder over a slice of them. **Neither
//! may name the other**: CI gate 22 clause A compares each sweep crate's
//! dependency set against a hardcoded `vocab`, and ships no allowlist. So the
//! two halves can only meet in a crate that is not on that list, and this is it.
//!
//! That is the gate working rather than the gate being avoided. Gate 22 exists
//! so a bar cannot reach the sweep; keeping the join out of the swept crates is
//! what preserves it. The three crates holding the mask, the vocabulary and the
//! ladder still cannot name a filesystem — and neither can this one.
//!
//! # It reads no bar from anywhere, and it never has
//!
//! [`Sweeper::run`] takes a slice the caller already holds. Nothing here opens a
//! file. The operator's standing rule is that a vendor pull is forbidden **and
//! so are the bars already on disk**, so the only input this crate has ever been
//! driven with is generated in-process — see [`synthetic`].
//!
//! # What a caller must decide, and what it must not
//!
//! [`Evaluator::new`] needs a `Widths`, an `Availability` and a `Thresholds`;
//! [`Ladder`] needs a `min_hits`. Every one is a decision about the run that
//! `CLAUDE.md` §3 rule 1 will not let this crate invent, so all four are the
//! caller's and none has a default here.
//!
//! **One of them is a trap and this crate refuses it.** `Availability` decides
//! whether the twenty VWAP positions mean anything for the whole run, and the
//! only function that computes it — `vwap::availability_of` — reads the WHOLE
//! slice, so the mask at bar 0 would depend on bar N. That is look-ahead, which
//! §3 rule 7 forbids outright. [`Sweeper::run`] therefore takes the verdict from
//! the caller and never derives it.
//!
//! # Cost
//!
//! Per bar the work is one [`indicators::column::Column`] step; per candidate it
//! is what `engine::Ladder::walk` costs. This crate adds one pass over the
//! column to hand it to the ladder and nothing else. UNVERIFIED as a measured
//! figure: this crate ships no bench yet, and saying so is cheaper than a number
//! nobody took.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod synthetic;

use engine::{Ladder, Sweep};
use indicators::column::{Census, Column};
use indicators::evaluator::Evaluator;
use indicators::{Candle, OI_NULL};
use vocab::ConditionMask;

/// Everything one sweep produced, and everything it refused on the way.
///
/// The two halves are kept together on purpose: a [`Sweep`] read without its
/// [`Census`] cannot tell "no combination was frequent" from "almost every bar
/// was refused as corrupt", and `CLAUDE.md` §4 bans a result that hides the
/// second behind the first.
#[derive(Clone, Debug)]
pub struct Outcome {
    /// Where every offered bar went — swept, still warming, or refused by name.
    pub census: Census,
    /// The index, in the caller's own slice, of the first bar that was swept.
    /// `None` when the run never warmed up, in which case the sweep is empty
    /// because there was nothing to sweep rather than nothing to find.
    pub first_swept: Option<usize>,
    /// The ladder's result. Check [`Sweep::completed`] before reading it as a
    /// complete answer.
    pub sweep: Sweep,
}

impl Outcome {
    /// Did this run produce a complete, trustworthy answer?
    ///
    /// Three ways it can be false, and they are different facts: the run never
    /// warmed up, so nothing was measured; every bar was refused; or the ladder
    /// breached its candidate ceiling and stopped short. [`Self::census`] and
    /// [`Sweep::halted`] say which.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.first_swept.is_some() && self.sweep.completed() && self.census.reconciles()
    }
}

/// Drives one sweep over a slice of candles.
pub struct Sweeper {
    ladder: Ladder,
}

impl Sweeper {
    /// A sweeper that walks `ladder` over whatever column it is given.
    #[must_use]
    pub const fn new(ladder: Ladder) -> Self {
        Self { ladder }
    }

    /// The ladder this sweeper will walk, echoed so a result is self-describing.
    #[must_use]
    pub const fn ladder(&self) -> Ladder {
        self.ladder
    }

    /// Folds `bars` into a condition-bit column and walks the ladder over it.
    ///
    /// `evaluator` is borrowed rather than built here because every one of its
    /// four construction parameters is a decision `CLAUDE.md` §3 rule 1 forbids
    /// this crate from making. It is left borrowed on return so a caller whose
    /// run produced nothing can read the warm-up diagnostics off it.
    ///
    /// The live position list is [`Evaluator::positions`] — every bit this
    /// vocabulary can actually compute — rather than every live bit in the
    /// table, so the ladder is never handed a position that would measure as
    /// permanently false because nothing computes it.
    pub fn run(&self, bars: &[Candle], evaluator: &mut Evaluator) -> Outcome {
        let column = Column::build(bars, evaluator);
        let live = live_positions();
        let sweep = self.ladder.walk(column.bits(), &live);
        Outcome {
            census: column.census(),
            first_swept: column.first_swept(),
            sweep,
        }
    }
}

/// Every position the indicator crate can compute, as the ladder wants them.
///
/// `Evaluator::positions()` returns `u16`; `Ladder::walk` takes `u32`. The
/// widening is infallible, and `u32::from` says so without a fallible arm that
/// no input could reach.
#[must_use]
pub fn live_positions() -> Vec<u32> {
    Evaluator::positions().into_iter().map(u32::from).collect()
}

/// A bar the evaluator will accept, for a caller assembling one by hand.
///
/// Not a market fact and not pretending to be one: it is a well-formed OHLC
/// record with an open interest of [`OI_NULL`], which is what §7 reserves for
/// "absent" rather than zero.
#[must_use]
pub const fn candle(ts_micros: i64, open: i64, high: i64, low: i64, close: i64) -> Candle {
    Candle::new(ts_micros, open, high, low, close, 0, OI_NULL)
}

/// The column a slice of bars produces, without walking a ladder over it.
///
/// Exposed because the column is worth inspecting on its own — it is the thing
/// `docs/03-vocabulary.md` describes — and because a caller that wants to walk
/// two different ladders over one column should not fold the bars twice.
#[must_use]
pub fn column_of(bars: &[Candle], evaluator: &mut Evaluator) -> Vec<ConditionMask> {
    Column::build(bars, evaluator).bits().to_vec()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail, and `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind."
)]
mod tests {
    use super::{Outcome, Sweeper, candle, column_of, live_positions};
    use crate::synthetic;
    use engine::Ladder;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;

    /// A ladder bounded on both axes, for every test that walks a real column.
    ///
    /// `min_hits` is 600 of 3,000 bars — 20% support, which is a frequency
    /// rather than a memorisation threshold — and the ceiling refuses a level
    /// long before it can cost a gigabyte. See
    /// `a_warm_run_produces_a_complete_sweep` for why both are needed and what
    /// happened when neither was there.
    fn bounded() -> Ladder {
        Ladder::with_min_hits(600).with_ceiling(50_000)
    }

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("both pinned widths are valid"),
            // Absent, never derived: `vwap::availability_of` reads the whole
            // slice, which would make bar 0's mask depend on bar N. §3 rule 7.
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    #[test]
    fn a_run_over_no_bars_measures_nothing_and_says_so() {
        let out = Sweeper::new(Ladder::with_min_hits(1)).run(&[], &mut evaluator());

        assert_eq!(out.census.offered, 0);
        assert_eq!(out.first_swept, None);
        assert!(out.sweep.bars == 0, "no bars means no denominator");
        assert!(
            !out.is_complete(),
            "an empty run is not a complete answer -- it measured nothing"
        );
        assert!(out.census.reconciles());
    }

    #[test]
    fn a_cold_run_sweeps_nothing_and_the_census_explains_why() {
        // One session cannot fill a five-session ladder.
        let bars = synthetic::sessions(1);
        let mut ev = evaluator();
        let out = Sweeper::new(Ladder::with_min_hits(1)).run(&bars, &mut ev);

        assert_eq!(out.census.offered, bars.len() as u64);
        assert_eq!(out.census.warming, bars.len() as u64);
        assert_eq!(out.census.swept, 0);
        assert_eq!(out.first_swept, None);
        assert!(!out.is_complete());
        assert!(out.census.reconciles());
        assert!(ev.sessions_until_every_family_can_answer() > 0);
    }

    /// A ladder bounded on BOTH axes, and the first draft was bounded on neither.
    ///
    /// # This test OOM-killed the process, and that is the finding
    ///
    /// It first read `Ladder::with_min_hits(2)` over eight sessions. Two hits out
    /// of 3,000 bars is **0.067% support**, so nearly all 238 computable
    /// positions are frequent, nearly every pair of them is, and
    /// `every_subset_is_frequent` never prunes. The frontier grows as `C(238, k)`
    /// and the run died by SIGKILL — the exact shape `engine::Halt` was written
    /// for, arriving on the very first real column anyone drove through it.
    ///
    /// The per-level ceiling did not save it, and the reason is worth writing
    /// down: `DEFAULT_CEILING` bounds ONE level at about a gigabyte, and
    /// `Sweep::levels` retains every level, so the real bound is depth times
    /// that. A per-level cap is not a total cap. `docs/06-limits.md` is where
    /// that belongs and it is not there yet.
    ///
    /// So this fixture bounds both: a threshold that means something as a
    /// frequency, and a ceiling small enough that a breach refuses in
    /// milliseconds instead of gigabytes.
    #[test]
    fn a_warm_run_produces_a_complete_sweep() {
        let bars = synthetic::sessions(8);
        let ladder = bounded();
        let out = Sweeper::new(ladder).run(&bars, &mut evaluator());

        assert!(
            out.first_swept.is_some(),
            "eight sessions must warm the run"
        );
        assert_eq!(out.census.refused(), 0, "generated bars are all valid bars");
        assert_eq!(
            out.sweep.bars, out.census.swept,
            "the denominator is the column"
        );
        assert!(out.sweep.completed(), "nothing should breach the ceiling");
        assert!(out.is_complete());
        assert!(
            !out.sweep.levels.is_empty(),
            "k=1 always runs, even if it finds nothing"
        );
    }

    #[test]
    fn the_same_bars_give_the_same_outcome_twice() {
        // §3 rule 5. Two runs over one input agree on every count and every
        // combination, or a result is not reproducible.
        let bars = synthetic::sessions(8);
        let a = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let b = Sweeper::new(bounded()).run(&bars, &mut evaluator());

        assert_eq!(a.census, b.census);
        assert_eq!(a.first_swept, b.first_swept);
        assert_eq!(a.sweep.depth(), b.sweep.depth());
        let left: Vec<_> = a.sweep.all_frequent().copied().collect();
        let right: Vec<_> = b.sweep.all_frequent().copied().collect();
        assert_eq!(left, right, "the same bars must give the same combinations");
    }

    #[test]
    fn a_refused_bar_is_named_and_changes_no_mask() {
        let mut bars = synthetic::sessions(8);
        let last = bars.last().copied().expect("the run is not empty");
        // high < low: a market can print a zero range, never a negative one.
        bars.push(candle(
            last.ts_micros + 60_000_000,
            last.close,
            last.close - 10,
            last.close + 10,
            last.close,
        ));
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());

        assert_eq!(out.census.high_below_low, 1);
        assert_eq!(out.census.refused(), 1);
        assert!(out.census.reconciles());
        // The column is exactly what the clean run produced.
        let clean = Sweeper::new(bounded()).run(&synthetic::sessions(8), &mut evaluator());
        assert_eq!(out.census.swept, clean.census.swept);
    }

    #[test]
    fn the_live_positions_are_the_ones_the_vocabulary_can_compute() {
        let live = live_positions();
        assert_eq!(
            live.len(),
            Evaluator::positions().len(),
            "the ladder is handed every computable position and no other"
        );
        assert!(
            live.iter()
                .all(|p| u16::try_from(*p).is_ok_and(vocab::table::is_live)),
            "a tombstoned position must never reach the ladder"
        );
        let mut sorted = live.clone();
        sorted.sort_unstable();
        assert_eq!(live, sorted, "positions arrive in table order");
    }

    #[test]
    fn a_column_can_be_taken_without_walking_a_ladder() {
        let bars = synthetic::sessions(8);
        let bits = column_of(&bars, &mut evaluator());
        let out = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        assert_eq!(bits.len() as u64, out.census.swept);
    }

    #[test]
    fn the_sweeper_echoes_the_ladder_it_was_given() {
        let s = Sweeper::new(Ladder::with_min_hits(7));
        assert_eq!(s.ladder().min_hits(), 7);
        assert_eq!(s.ladder().ceiling(), engine::DEFAULT_CEILING);
    }

    #[test]
    fn a_halted_sweep_is_not_a_complete_outcome() {
        // The ceiling is what stops a frontier that never goes extinct. An
        // Outcome must not read as complete when the ladder stopped short.
        let bars = synthetic::sessions(8);
        let tight = Ladder::with_min_hits(1).with_ceiling(1);
        let out = Sweeper::new(tight).run(&bars, &mut evaluator());

        // Asserted unconditionally, not behind `if halted.is_some()`. A ceiling of
        // one over a warm 3,000-bar column MUST breach -- and guarding the
        // assertion behind the thing it is testing makes the test pass when the
        // halt stops happening, which is the one outcome it exists to catch. It
        // also left an else-arm no run can enter, which llvm-cov counts forever.
        assert!(
            out.sweep.halted.is_some(),
            "a ceiling of one cannot survive a real column"
        );
        assert!(!out.is_complete(), "a halted sweep is not complete");
        assert!(out.census.reconciles());
    }

    /// The exact call that OOM-killed this process, now a loud refusal.
    ///
    /// `min_hits(2)` over eight sessions is 0.067% support: nearly all 238
    /// computable positions are frequent, nearly every pair is, and the subset
    /// prune never fires — the frontier grows as `C(238, k)`. This call used to
    /// end in SIGKILL, and the per-level ceiling did not stop it because
    /// `Sweep::levels` retains every level, so the peak was depth × ceiling.
    ///
    /// The budget is cumulative now, so the same call refuses instead of dying.
    /// That difference — refusing versus being killed — is the whole of what
    /// makes an unattended sweep safe to start.
    #[test]
    fn the_configuration_that_oom_killed_the_process_now_refuses() {
        let bars = synthetic::sessions(8);
        let ladder = Ladder::with_min_hits(2).with_ceiling(200_000);
        let out = Sweeper::new(ladder).run(&bars, &mut evaluator());

        let halt = out.sweep.halted;
        assert!(halt.is_some(), "it must refuse, not run away");
        assert!(!out.is_complete(), "and must not read as a complete answer");
        assert!(
            out.census.reconciles(),
            "every bar still lands in one bucket"
        );
        // The levels below the breach are complete and are kept: a caller paid
        // for them and §4 asks for a named degradation, not a discarded result.
        assert!(!out.sweep.levels.is_empty());
    }

    #[test]
    fn is_complete_needs_all_three_conditions() {
        // The negative case, so the conjunction cannot rot into a constant.
        let bars = synthetic::sessions(8);
        let good = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        assert!(good.is_complete());

        let never_warm = Outcome {
            first_swept: None,
            ..good.clone()
        };
        assert!(
            !never_warm.is_complete(),
            "a run that never warmed is not complete"
        );
    }
}
