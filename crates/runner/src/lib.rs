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
//! column to hand it to the ladder and nothing else.
//!
//! Measured by `C-R-01` in `crates/runner/benches/ratio.rs`: the column build
//! costs the same per offered bar at 3,000 and 12,000 bars. The LADDER half is
//! not per-bar and is not claimed to be -- it is O(candidates), `engine` bounds
//! it, and a first version of that bench row which divided a whole run by a bar
//! count read 0.002x and was measuring the vocabulary.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod closed;
pub mod identity;
pub mod outcome;
pub mod rank;
pub mod report;
pub mod resample;
pub mod significance;
pub mod split;
pub mod synthetic;
pub mod trade;

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
    /// Four ways it can be false, and they are different facts: the run never
    /// warmed up, so nothing was measured; every bar was refused; the ladder
    /// breached a budget and stopped short; or no ladder was walked at all.
    /// [`Self::census`] and [`Sweep::halted`] say which.
    ///
    /// # The fourth clause, and why a `Sweep` alone cannot answer this
    ///
    /// [`Sweeper::auto`] returns `Sweep::default()` when its search kept no
    /// rung. That value has `bars: 0`, no levels, and — because nothing ran to
    /// breach anything — `halted: None`, which [`Sweep::completed`] reads as a
    /// clean finish. Without `bars > 0` this method answered **true** for a
    /// search that measured nothing, and `report::render` printed "trustworthy
    /// as a whole answer: yes" underneath it.
    ///
    /// That is the fallback `CLAUDE.md` §4 bans outright — a failure wearing a
    /// success's clothes — and it was found by an adversarial audit rather than
    /// by any test here, because every test asked whether a real sweep reported
    /// itself correctly and none asked what an absent one reported.
    ///
    /// The clause is `bars == census.swept`, not `bars > 0`. Both exclude the
    /// fabricated value, and the equality also catches a sweep walked over a
    /// column that is not the one this census describes — a mismatch nothing
    /// else here would notice. `Sweeper::run` satisfies it by construction,
    /// because it hands `Ladder::walk` the very column it took the census from.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.first_swept.is_some()
            && self.sweep.bars == self.census.swept
            && self.sweep.completed()
            && self.census.reconciles()
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

/// Pairs one probe may walk while the search is looking for a threshold.
///
/// A search parameter, not a result parameter. It is small on purpose: a probe
/// only has to answer "is this threshold cheap", and a threshold that **finishes**
/// inside `PROBE_PAIRS` iterations has finished — running it again under a
/// larger budget cannot change its answer, because it never reached the smaller
/// one. So the kept sweep is returned as measured and is never re-walked.
///
/// The consequence is that the search is **conservative**: a threshold needing
/// more than this but less than the ladder's real budget is rejected, so the
/// chosen threshold may be higher than strictly necessary. That is the safe
/// direction, and it is stated rather than hidden.
///
/// # The value is measured, not chosen
///
/// An audit found the first value — `1 << 22` — sitting **4096× below** the
/// budget it tunes for, which rejects thresholds that would comfortably finish
/// and pins the answer near 50% support. But probing at the real budget is not
/// affordable either. Both ends were measured, same machine, same fixture, whole
/// `auto` search end to end:
///
/// | probe budget | search time |
/// |---|---|
/// | `1 << 22` | 0.39 s |
/// | `1 << 24` | 0.71 s |
/// | `1 << 26` | 1.76 s |
/// | **`1 << 28`** | **5.60 s** |
/// | `1 << 34` (the real budget) | **319.84 s** |
///
/// `1 << 28` searches 64× deeper than the original for 5.6 seconds, and leaves
/// the gap at 64× rather than 4096×. Past it the curve turns sharply: the last
/// step costs 57× the time for 64× the reach.
const PROBE_PAIRS: u64 = 1 << 28;

/// A sweep the engine tuned for itself, and the search that got there.
#[derive(Clone, Debug)]
pub struct Auto {
    /// Did the search find a threshold that actually measured something?
    ///
    /// False when no rung produced a non-empty result: either the column never
    /// warmed, or every affordable threshold was vacuous. `min_hits` is `None`
    /// in exactly those cases, and this field says so without a caller having to
    /// infer it from an `Option`.
    pub affordable: bool,
    /// The deepest sweep that proved affordable, with its census.
    pub outcome: Outcome,
    /// The threshold it settled on. `None` when the column was empty or when even
    /// the cheapest threshold refused.
    pub min_hits: Option<u64>,
    /// Ladders walked, one per halving.
    pub attempts: u32,
    /// The first threshold that refused. Below this the column is unaffordable.
    pub refused_below: Option<u64>,
}

impl Sweeper {
    /// Sweeps without a caller choosing a threshold.
    ///
    /// # The manual step this removes
    ///
    /// `min_hits` is the only knob a sweep has, and it cannot be guessed. An
    /// audit measured this exact column: 53% support finishes in 6 ms, 11%
    /// support takes 495 seconds, and 9% support ran for over 1,500 seconds
    /// before it was killed. The safe value is a property of the data, and no
    /// operator can know it before running.
    ///
    /// # Why this could not be written before the pair budget existed
    ///
    /// A first attempt predicted affordability from `C(n, 2)` at k=2. It hung,
    /// because k=2 is never what explodes — the frontier blows up at k≥10, and no
    /// arithmetic over one level bounds the ladder.
    ///
    /// A search only became possible once **every walk terminates**. With the
    /// pair budget in place a probe that is too low refuses in bounded time
    /// instead of running for ever, so the search can simply *try*.
    ///
    /// # The search
    ///
    /// Cost is monotone in the threshold: a higher `min_hits` admits fewer
    /// positions, so fewer candidates and fewer pairs — always. So the search
    /// starts at the whole column, where nothing can be frequent, and halves. The
    /// last probe that **completed** is the answer; the first that refused is the
    /// edge, reported so a caller can see what this column cannot afford.
    /// Monotonicity is what makes stopping at the first refusal correct rather
    /// than merely convenient.
    ///
    /// The column is folded **once** and every probe walks the same one, so the
    /// search costs `log2(bars)` ladder walks over a column built one time.
    pub fn auto(&self, bars: &[Candle], evaluator: &mut Evaluator) -> Auto {
        let column = Column::build(bars, evaluator);
        let live = live_positions();
        let census = column.census();
        let first_swept = column.first_swept();

        let mut attempts = 0_u32;
        let mut best: Option<(u64, Sweep)> = None;
        // The highest rung known to FINISH, vacuous or not -- the bisection's
        // upper bracket. Keyed on `best` instead, the bisection was dead code.
        let mut last_completing: Option<u64> = None;
        let mut refused_below = None;
        // ONE BELOW THE COLUMN, NOT AT IT, AND THE DIFFERENCE IS A WRONG ANSWER.
        //
        // At `min_hits == swept` a position must hit every bar to be frequent —
        // and D-0080 excludes a position at `support == bars` as `AlwaysTrue`
        // before k=1. So that rung is empty BY CONSTRUCTION, not by measurement,
        // and it always "completes". An audit found the consequence: if the next
        // rung down refuses, the search keeps the empty one and `is_complete()`
        // reports true on a sweep that measured nothing.
        //
        // `swept - 1` is the highest threshold that can yield anything at all.
        let mut threshold = census.swept.saturating_sub(1);

        // `swept == 0` is a run that never warmed up: no column, nothing to tune.
        while threshold >= 1 {
            attempts = attempts.saturating_add(1);
            // The CALLER's ceiling, not a fresh default: `auto` used to be an
            // associated function and silently discarded whatever budget the
            // `Sweeper` was built with.
            let sweep = Ladder::with_min_hits(threshold)
                .with_ceiling(self.ladder.ceiling())
                .with_pair_budget(PROBE_PAIRS)
                .walk(column.bits(), &live);
            if sweep.completed() {
                // A PROBE THAT FOUND NOTHING IS NOT AN ANSWER, and this is the
                // whole of the bug an audit found on a 98,124-bar column.
                //
                // Near the top of the range every position is excluded before
                // k=1 -- D-0080 refuses `support == bars` as `AlwaysTrue`, and
                // nothing else clears so high a threshold -- so the walk
                // "completes" with depth 0. Keeping that as the result reports
                // `is_complete() == true` on a sweep that measured nothing,
                // which reads exactly like a genuine finding of "no combination
                // was frequent". The audit's column had 145,735 frequent sets
                // waiting 3.9 seconds away at the very threshold the search had
                // just refused.
                //
                // Starting one rung lower did NOT fix it: at 98,124 bars,
                // `swept - 1` still demands 98,123 of 98,124 and is just as
                // empty. Emptiness has to be tested for, not arithmetic'd around.
                // TWO facts are recorded, and conflating them was a defect.
                //
                // `last_completing` is "this rung finished inside the probe
                // budget", vacuous or not. `best` is the stronger "and it found
                // something". The bisection below needs the FIRST to have a
                // bracket at all -- an audit found that keying the bracket on
                // `best` made the bisection dead code on precisely the columns it
                // was written to repair.
                last_completing = Some(threshold);
                if sweep.depth() >= 1 {
                    best = Some((threshold, sweep));
                }
            } else {
                refused_below = Some(threshold);
                break;
            }
            if threshold == 1 {
                break;
            }
            threshold = threshold.checked_div(2).unwrap_or(1).max(1);
        }

        // BISECT THE BRACKET THE HALVING LEFT BEHIND.
        //
        // # The factor of two the grid was throwing away
        //
        // Halving probes only powers of two below the column, so the answer was
        // pinned to that grid while the true frontier sits anywhere between the
        // last rung that completed and the first that refused. An adversarial
        // audit measured the cost on one real column, at this search's own probe
        // budget:
        //
        // | | chosen by the grid | deepest affordable | |
        // |---|---|---|---|
        // | `min_hits` | 48,374 (50.0%) | 30,961 (32.0%) | |
        // | depth | 12 | 14 | +2 levels |
        // | frequent sets | 6,631 | 51,778 | **7.8x** |
        //
        // The grid jumped 48,374 -> 24,187 and never tried 30,961, so 45,147
        // combinations that this machine could afford in 0.55 s were reported as
        // not existing. On a second column the grid jumped from a vacuous rung
        // straight past 55,471 and returned "nothing affordable" one probe away
        // from 60,383 combinations.
        //
        // # Why bisection is correct here and not merely closer
        //
        // The same monotonicity that makes stopping at the first refusal correct
        // makes this correct: cost falls as the threshold rises, so the range
        // holds exactly one crossover — every threshold above it completes and
        // every one below refuses. Bisection finds that crossover. It cannot
        // land on a threshold the halving would have accepted and this rejects,
        // because both ask the same question of the same column.
        //
        // The cost is `log2(bracket)` more walks on a column already folded, and
        // the bracket is at most the last rung's own width.
        // # THE PREDICATE IS NOT THE ONE THIS CODE FIRST BISECTED ON
        //
        // The first version bisected on `completed() && depth() >= 1` and argued
        // monotonicity from cost alone. Two adversarial audits killed both halves
        // of that:
        //
        // * **It was dead code.** The bracket's top was `best`, which is only set
        //   by a NON-VACUOUS rung. `swept - 1` is vacuous by construction --
        //   D-0080 excludes `support == bars` as AlwaysTrue -- so on any column
        //   where the grid goes vacuous-then-refused, `best` stays `None`, `hi`
        //   collapses onto `lo`, and the loop never runs. Measured: at ceiling
        //   5,000 `auto` returned "nothing affordable, 0 combinations" while
        //   4,722 combinations were affordable at t=592. The comment above this
        //   block described that exact column as the thing being repaired.
        //
        // * **The conflated predicate is not monotone.** Two separate facts move
        //   in OPPOSITE directions with the threshold:
        //
        //   | | low threshold | high threshold |
        //   |---|---|---|
        //   | completes inside the budget | no -- too many candidates | yes |
        //   | finds anything (`depth >= 1`) | yes | no -- all excluded |
        //
        //   So `completed && depth>=1` is false, then true, then false again. A
        //   bisection whose else-arm moves `lo` upward can step over the band
        //   entirely and still answer `None`.
        //
        // Bisecting on `completed()` ALONE fixes both. That predicate is monotone
        // in the threshold on its own -- cost falls as the threshold rises, full
        // stop -- so the bracket `(refused, last_completing]` holds exactly one
        // crossover and bisection converges on it. Vacuousness is then a question
        // asked ABOUT the answer rather than a term inside the search: `best` is
        // updated only when a completing probe also found something, so it ends
        // holding the lowest threshold that both finished and measured anything.
        // `zip` rather than a tuple pattern: the grid always completes the top
        // rung before it can refuse anything, so `(Some, None)` cannot occur and
        // matching on it would leave an arm no run reaches.
        if let Some((refused, top)) = refused_below.zip(last_completing) {
            let mut lo = refused;
            let mut hi = top;
            while hi.saturating_sub(lo) > 1 {
                let mid = lo.saturating_add(hi.saturating_sub(lo) / 2);
                attempts = attempts.saturating_add(1);
                let sweep = Ladder::with_min_hits(mid)
                    .with_ceiling(self.ladder.ceiling())
                    .with_pair_budget(PROBE_PAIRS)
                    .walk(column.bits(), &live);
                if sweep.completed() {
                    let found = sweep.depth() >= 1;
                    hi = mid;
                    // Lower thresholds admit more, so each accepted probe is a
                    // better answer than the last -- and a vacuous one is not an
                    // answer at all, so it narrows the bracket without being kept.
                    if found {
                        best = Some((mid, sweep));
                    }
                } else {
                    lo = mid;
                }
            }
            // The edge is now the tightest one measured, not the grid's.
            refused_below = Some(lo);
        }

        let (min_hits, sweep) = best.map_or((None, Sweep::default()), |(t, s)| (Some(t), s));
        Auto {
            affordable: min_hits.is_some(),
            outcome: Outcome {
                census,
                first_swept,
                sweep,
            },
            min_hits,
            attempts,
            refused_below,
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
    /// `min_hits` is 600 of the 1,124 SWEPT bars — 53.4% support, which is a
    /// frequency rather than a memorisation threshold. Not 600 of the 3,000
    /// offered: `Ladder::walk` is handed `column.bits()`, so `Sweep::bars` is
    /// the swept count and `report.rs` renders the ratio as "of swept bars".
    /// This comment read "20%" against the offered count until an audit caught
    /// the two denominators disagreeing across three files — and the ceiling refuses a level
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

    /// This crate cannot read a bar, and that is checked rather than promised.
    ///
    /// # Why this test exists and why it is structural
    ///
    /// CI gate 22 pins `vocab`, `indicators` and `engine` to a `vocab`-only
    /// dependency set and forbids any filesystem call site in their `src/` or
    /// `benches/`. **`crates/runner` is not on that list**, and it cannot be:
    /// gate 22 is what stops `indicators` and `engine` naming each other, so the
    /// crate that joins them has to sit outside it. That leaves exactly one crate
    /// in the sweep chain whose purity is not welded shut by CI.
    ///
    /// The operator's rule is absolute — no vendor pull, no ingest, and not the
    /// bars already on disk either — so "I inspected it" is not a good enough
    /// guarantee for the one unguarded link. This runs under `cargo test`, needs
    /// no workflow change, and fails the moment a filesystem call appears here.
    ///
    /// The probe list is gate 22 clause B's own, plus the embedding macro and the
    /// path and environment types a reader would need before it could name a file
    /// at all.
    ///
    /// **Every needle is assembled from fragments, and the first draft was not.**
    /// It spelled three of them literally and the test refused its own source —
    /// which is the right failure and the fourth time this workspace has met it:
    /// gate 17 refused a comment describing gate 17, gate 22 clause A read a
    /// comment as a dependency table, and `core`'s float scanner read a hex digest
    /// as a type name. A guard that reads text must never spell what it hunts.
    #[test]
    fn this_crate_cannot_open_a_file_and_cannot_name_the_store() {
        let banned: [&str; 16] = [
            concat!("Fi", "le::open"),
            concat!("Fi", "le::create"),
            concat!("Open", "Options"),
            concat!("f", "s::read"),
            concat!("f", "s::write"),
            concat!("st", "d::fs"),
            concat!("read", "_dir"),
            concat!("Mmap", "Options"),
            concat!("mem", "map"),
            concat!("include_", "bytes!"),
            concat!("Comm", "and::new"),
            concat!("proce", "ss::Command"),
            concat!("st", "d::os::"),
            concat!("lib", "c::"),
            concat!("Path", "Buf"),
            concat!("en", "v::var"),
        ];
        let sources = [
            ("lib.rs", include_str!("lib.rs")),
            ("synthetic.rs", include_str!("synthetic.rs")),
        ];
        for (name, src) in sources {
            for needle in banned {
                assert!(
                    !src.contains(needle),
                    "crates/runner/src/{name} contains `{needle}`. This crate is the \
                     one link in the sweep chain gate 22 does not guard, and the \
                     operator's rule is that neither a pull nor a stored bar may \
                     ever reach it. Every candle here comes from `synthetic::bar`, \
                     which is arithmetic on two integers."
                );
            }
        }

        // And the dependency set, exactly — the same argument gate 22 clause A
        // makes: a crate that cannot NAME a bar reader cannot call one.
        let manifest = include_str!("../Cargo.toml");
        for forbidden in ["store", "pull", "reqwest", "telemetry"] {
            assert!(
                !manifest.contains(&format!("\n{forbidden} ")),
                "crates/runner must not depend on `{forbidden}` -- that is how a \
                 stored bar would reach the sweep"
            );
        }
    }

    /// The configuration an audit measured at over 1,500 seconds, now bounded.
    ///
    /// # What the audit actually found, and why the first fix was not enough
    ///
    /// A counting allocator and a watchdog were run across the whole threshold
    /// range on this exact column (8 sessions, 1,124 swept bars):
    ///
    /// | `min_hits` | support | peak heap | wall | outcome |
    /// |---|---|---|---|---|
    /// | 600 | 53.4% | 0.9 MB | 0.006 s | extinct |
    /// | 300 | 26.7% | 18.6 MB | 1.49 s | extinct |
    /// | 125 | 11.1% | 314.7 MB | 495 s | last safe |
    /// | 100 | 8.9% | 640 MB | **>1500 s** | killed, still in the k=10 join |
    /// | 50 | 4.4% | 1,594 MB | **>1500 s** | killed |
    ///
    /// **Peak heap never exceeded 3.3% of that machine.** Memory was never the
    /// binding constraint; time was, and the candidate ceiling could not see it —
    /// it counts what a level HOLDS, and `popcount != k` rejects almost every pair
    /// before it is ever counted.
    ///
    /// The pair budget counts what the join DOES. Under one, the same call that
    /// ran for twenty-five minutes and was killed refuses in milliseconds and says
    /// which budget it spent.
    #[test]
    fn a_threshold_that_used_to_run_for_ever_now_refuses_by_time() {
        let bars = synthetic::sessions(8);
        // The exact threshold the audit could not complete, under a pair budget
        // small enough for a test. The DEFAULT budget bounds the same call at
        // 2^34 pairs; this one only has to prove the mechanism fires.
        let ladder = Ladder::with_min_hits(2).with_pair_budget(50_000);
        let out = Sweeper::new(ladder).run(&bars, &mut evaluator());

        let halt = out.sweep.halted.expect("it must refuse, not run for ever");
        assert_eq!(
            halt.breach,
            engine::Breach::Pairs,
            "TIME is what bound here -- the candidate ceiling was nowhere near spent"
        );
        assert!(
            halt.candidates < engine::DEFAULT_CEILING,
            "if the ceiling had bound, the pair budget would be the wrong fix"
        );
        assert!(!out.is_complete(), "and it must not read as a whole answer");
        assert!(out.census.reconciles());
    }

    /// A sweep with no human number in it, and it terminates.
    ///
    /// The first version of this hung. It predicted affordability from `C(n,2)`
    /// at k=2, and k=2 is never what explodes. This one probes, which is only
    /// possible because the pair budget makes every probe terminate.
    #[test]
    fn auto_tunes_itself_and_finishes() {
        let bars = synthetic::sessions(8);
        let auto = Sweeper::new(bounded()).auto(&bars, &mut evaluator());

        let chosen = auto.min_hits.expect("a warm column must yield a threshold");
        assert!(auto.outcome.is_complete(), "what it keeps must be whole");
        assert_eq!(
            auto.outcome.sweep.min_hits, chosen,
            "the sweep must echo the threshold the search settled on"
        );
        assert!(chosen <= auto.outcome.census.swept);
        // Halving, so the search is logarithmic rather than linear.
        assert!(
            auto.attempts <= 64,
            "{} attempts is not a halving search",
            auto.attempts
        );
    }

    /// It never hands back a partial answer, whatever it had to reject.
    #[test]
    fn auto_never_returns_a_partial_answer() {
        let bars = synthetic::sessions(8);
        let auto = Sweeper::new(bounded()).auto(&bars, &mut evaluator());
        assert!(
            auto.outcome.sweep.halted.is_none(),
            "a refused probe must be discarded, never returned"
        );
        // Monotone: the edge it found is strictly below what it kept. Checked over
        // a warm column AND a cold one so both arms of the match are taken —
        // an arm no run enters is a region llvm-cov counts forever.
        for column in [synthetic::sessions(8), synthetic::sessions(1)] {
            let a = Sweeper::new(bounded()).auto(&column, &mut evaluator());
            let monotone = match (a.min_hits, a.refused_below) {
                (Some(kept), Some(edge)) => edge < kept,
                _ => true,
            };
            assert!(monotone, "the refused threshold must be below the kept one");
        }
    }

    /// A column of exactly one swept bar, which is where the search starts at 1.
    ///
    /// The `threshold == 1` exit is otherwise unreachable: on any real column the
    /// search refuses long before halving that far, so the arm would be a region
    /// no run enters.
    #[test]
    fn auto_over_a_single_swept_bar_starts_and_stops_at_one() {
        let all = synthetic::sessions(8);
        let warm = Sweeper::new(bounded())
            .auto(&all, &mut evaluator())
            .outcome
            .first_swept
            .expect("eight sessions warm up");
        // TWO bars past the warm-up, so the search starts at `swept - 1` = 1 and
        // the `threshold == 1` exit is the one it takes.
        let head = all.get(..warm.saturating_add(2)).unwrap_or(&[]);
        let auto = Sweeper::new(bounded()).auto(head, &mut evaluator());

        assert_eq!(auto.outcome.census.swept, 2, "two bars past warm-up");
        assert_eq!(auto.attempts, 1, "the search starts at 1 and stops there");
        assert_eq!(auto.min_hits, Some(1));
        assert!(auto.outcome.census.reconciles());

        // And ONE swept bar has no threshold at all: the only position that could
        // be frequent would hit every bar, which D-0080 excludes before k=1. The
        // search must report that rather than invent a rung.
        let single = all.get(..warm.saturating_add(1)).unwrap_or(&[]);
        let none = Sweeper::new(bounded()).auto(single, &mut evaluator());
        assert_eq!(none.outcome.census.swept, 1);
        assert_eq!(
            none.attempts, 0,
            "no threshold below a one-bar column exists"
        );
        assert_eq!(none.min_hits, None);
    }

    /// The tuner must never hand back an empty answer marked complete.
    ///
    /// # The bug this pins, which survived two earlier fixes
    ///
    /// An audit drove a 98,124-bar column and got back
    /// `chose=98124 depth=0 frequent=0 is_complete=true` — while the very
    /// threshold the search had just refused completed in **3.89 s with 145,735
    /// frequent sets**. An empty result reported as whole reads exactly like a
    /// genuine finding of "no combination was frequent", which is the worst
    /// failure this crate can produce.
    ///
    /// Two fixes did not close it. Starting at `swept` was obviously wrong;
    /// starting at `swept - 1` is *just as* empty on a large column, because
    /// demanding 98,123 of 98,124 bars excludes everything the same way.
    /// Emptiness has to be **tested for**, not arithmetic'd around: a probe is
    /// only an answer if it found something.
    #[test]
    fn auto_never_keeps_a_vacuous_rung() {
        // A cold column is in the list on purpose: it takes the `None` arm, and
        // an arm no fixture enters is a region llvm-cov counts forever.
        for sessions in [8_i64, 16, 1] {
            let bars = synthetic::sessions(sessions);
            let auto = Sweeper::new(bounded()).auto(&bars, &mut evaluator());

            match auto.min_hits {
                Some(chosen) => {
                    assert!(
                        auto.affordable,
                        "a chosen threshold must be reported as affordable"
                    );
                    assert!(
                        auto.outcome.sweep.depth() >= 1,
                        "{sessions} sessions: kept threshold {chosen} measured \
                         nothing -- depth 0 is emptiness by construction, not a \
                         finding"
                    );
                    assert!(
                        auto.outcome.sweep.all_frequent().count() >= 1,
                        "a kept rung must carry at least one combination"
                    );
                    assert!(
                        chosen < auto.outcome.census.swept,
                        "a threshold at the column size excludes every position"
                    );
                }
                None => assert!(
                    !auto.affordable,
                    "no threshold means not affordable, and both must agree"
                ),
            }
        }
    }

    #[test]
    fn auto_over_a_cold_or_empty_column_attempts_nothing() {
        for bars in [synthetic::sessions(1), Vec::new()] {
            let auto = Sweeper::new(bounded()).auto(&bars, &mut evaluator());
            assert_eq!(auto.min_hits, None, "there was no column to tune");
            assert_eq!(auto.attempts, 0, "and so nothing was walked");
            assert!(!auto.outcome.is_complete());
            assert!(auto.outcome.census.reconciles());
        }
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
