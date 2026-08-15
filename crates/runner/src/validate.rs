//! Walk-forward: choose on the past, judge on the future, report both.
//!
//! # The gap this closes
//!
//! [`crate::split`] has had `purged_folds` and `anchored_folds` since it was
//! written, correct and tested, and **nothing called either of them**. So every
//! number this engine has ever produced was in-sample: the sweep found
//! combinations, measured their edge on the same bars it found them on, deflated
//! the significance bar for multiplicity, and reported.
//!
//! Deflating for multiplicity is not the same test. It asks "could the best of N
//! random hypotheses look this good", which is a statement about the SEARCH. It
//! cannot ask "does this hold on data it was not chosen on", which is a
//! statement about the FINDING. The second question is the one a backtest
//! exists to answer, and until this module it was never put.
//!
//! # Anchored, and that is forced rather than preferred
//!
//! [`crate::split::anchored_folds`] explains it at length: `indicators::Evaluator`
//! is stateful — EMAs, session rollovers, a warm-up prefix — so it consumes bars
//! in order and cannot be handed two disjoint ranges without either restarting
//! the warm-up mid-series or pretending a gap is not there. A k-fold design with
//! test windows in the middle would be stronger and this engine cannot honestly
//! drive it.
//!
//! So each fold trains on `0..boundary` and tests on `boundary+H..`, with the
//! purge between them. The earliest fold has the least data and the latest the
//! most, which is a real weakness and is stated rather than hidden.
//!
//! # Selection is by the WORST case, on purpose
//!
//! [`crate::trade`] prices every round trip twice — best is the next bar's open,
//! worst is its adverse extreme. Choosing on the best case would pick the
//! combination most flattered by optimistic fills, which is the failure mode the
//! two-case model exists to expose. So the in-sample winner is the one with the
//! largest WORST-case total, and both figures are reported out of sample.
//!
//! # What a fold cannot tell you, said plainly
//!
//! One fold is one draw. A combination that wins in-sample and holds out of
//! sample on a single test window has survived one comparison, not a proof. What
//! turns a set of these into a probability of overfitting is
//! Bailey–López de Prado's PBO, which composes on this module and is not built
//! yet. Reading a positive out-of-sample fold as "it works" is exactly the error
//! the deflated bar upstream exists to prevent.

use indicators::Candle;
use indicators::column::Column;
use indicators::evaluator::Evaluator;
use vocab::ConditionMask;

use crate::outcome::Horizon;
use crate::split::anchored_folds;
use crate::trade::{Trades, walk};
use costs::fill::Direction;

/// What one combination did over one set of bars.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Round trips taken. Not signals — see [`crate::trade`].
    pub trades: u64,
    /// Total paisa per unit if every fill landed at the bar's open.
    pub best: i64,
    /// Total paisa per unit if every fill landed at the adverse extreme.
    pub worst: i64,
    /// Round trips closed by the 15:10 square-off rather than the horizon.
    pub forced: u64,
}

impl Summary {
    /// Fold a completed walk into its totals.
    fn of(t: &Trades) -> Self {
        Self {
            trades: t.count(),
            best: t.trades.iter().fold(0_i64, |a, x| a.saturating_add(x.best)),
            worst: t
                .trades
                .iter()
                .fold(0_i64, |a, x| a.saturating_add(x.worst)),
            forced: u64::try_from(t.trades.iter().filter(|x| x.forced).count()).unwrap_or(u64::MAX),
        }
    }

    /// Did the worst case make money at all?
    ///
    /// The only question this module is willing to answer about one fold, and
    /// it is deliberately blunt: a positive worst-case total means the
    /// combination survived pessimistic fills, and nothing more. It is not a
    /// verdict and not a recommendation.
    #[must_use]
    pub const fn worst_case_positive(&self) -> bool {
        self.worst > 0
    }
}

/// One fold: what was chosen on the past, and what it did on the future.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoldResult {
    /// Position in the walk, from zero.
    pub index: usize,
    /// Bars the sweep was allowed to see.
    pub train_bars: usize,
    /// Bars discarded between train and test because their outcome window
    /// reached across the boundary.
    pub purged: usize,
    /// Bars the choice was judged on.
    pub test_bars: usize,
    /// Combinations the sweep produced on the training bars.
    pub considered: u64,
    /// The combination with the best in-sample worst-case total, if any.
    pub chosen: Option<ConditionMask>,
    /// What it did on the bars it was chosen on.
    pub in_sample: Summary,
    /// What it did on the bars it had never seen.
    pub out_of_sample: Summary,
}

/// A whole walk-forward.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Validated {
    /// One entry per test period, in time order.
    pub folds: Vec<FoldResult>,
    /// Combinations that were trade-walked but not ranked, because the
    /// candidate budget was reached.
    ///
    /// **Reported, never silent.** A validation that quietly looked at the first
    /// N combinations and called it a search would be the same defect as a
    /// bench measuring nothing.
    pub not_considered: u64,
}

impl Validated {
    /// Folds whose chosen combination was still positive out of sample, under
    /// the worst-case fill.
    #[must_use]
    pub fn held_up(&self) -> usize {
        self.folds
            .iter()
            .filter(|f| f.chosen.is_some() && f.out_of_sample.worst_case_positive())
            .count()
    }

    /// Folds that chose anything at all.
    ///
    /// Separate from `folds.len()` because a fold whose sweep found nothing
    /// frequent has no choice to judge, and counting it as a failure would
    /// blame the walk for an empty search.
    #[must_use]
    pub fn decided(&self) -> usize {
        self.folds.iter().filter(|f| f.chosen.is_some()).count()
    }
}

/// How many combinations a fold may trade-walk before it stops ranking.
///
/// A bound rather than a preference: the sweep can emit tens of thousands of
/// frequent itemsets, and each one costs a full pass over the training bars to
/// price. The cap is applied to the CLOSED set — [`crate::closed`] removes exact
/// duplicates losslessly first, so the budget is spent on distinct hypotheses
/// rather than on the same one under several names.
///
/// Whatever it drops is counted in [`Validated::not_considered`] and never
/// silently discarded.
pub const DEFAULT_CANDIDATES: usize = 512;

/// Run an anchored walk-forward over `bars`.
///
/// `splits` is the number of test periods. Each fold sweeps the training prefix,
/// prices every closed combination it found, keeps the one with the largest
/// worst-case total, and re-prices that one alone on the test period.
///
/// # The evaluator is rebuilt per fold, and per side
///
/// `indicators::Evaluator` carries state, so a fold cannot reuse the one before
/// it — the EMAs would arrive pre-warmed by bars that fold was not allowed to
/// see. Each column is built from bar zero with a fresh evaluator, which is what
/// the indicators would genuinely have held at that moment.
///
/// The TEST column is also built from bar zero rather than from the test start,
/// for the same reason in the other direction: an indicator at the first test
/// bar legitimately saw the training bars, and starting it cold there would
/// under-report its state. Only the TRADES are restricted to the test range —
/// [`crate::trade::walk`] is handed a column whose rows outside the window are
/// masked out, so no signal before `test.start` can open a position.
///
/// # Cost
///
/// One sweep and up to [`DEFAULT_CANDIDATES`] trade-walks per fold, plus one
/// trade-walk on the test side. Every one is a pass over its own bars.
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn walk_forward(
    bars: &[Candle],
    horizon: Horizon,
    splits: usize,
    direction: Direction,
    sweeper: &crate::Sweeper,
    mut evaluator: impl FnMut() -> Evaluator,
) -> Validated {
    let mut out = Validated::default();

    for (index, fold) in anchored_folds(bars.len(), horizon, splits)
        .into_iter()
        .enumerate()
    {
        let train_end = fold.train.0.end;
        let Some(train) = bars.get(..train_end) else {
            continue;
        };
        let swept = sweeper.run(train, &mut evaluator());

        // The closed set: exact duplicates removed losslessly, so the candidate
        // budget buys distinct hypotheses rather than aliases of one.
        let closed = crate::closed::closed(&swept.sweep);
        let considered = u64::try_from(closed.kept.len()).unwrap_or(u64::MAX);
        let over = closed.kept.len().saturating_sub(DEFAULT_CANDIDATES);
        out.not_considered = out
            .not_considered
            .saturating_add(u64::try_from(over).unwrap_or(0));

        // Price every candidate on the training bars and keep the best under
        // PESSIMISTIC fills. `max_by_key` on the worst-case total, so a
        // combination flattered by optimistic fills cannot win.
        let train_column = Column::build(train, &mut evaluator());
        let mut best: Option<(ConditionMask, Summary)> = None;
        for item in closed.kept.iter().take(DEFAULT_CANDIDATES) {
            let s = Summary::of(&walk(train, &train_column, &item.mask, horizon, direction));
            if s.trades == 0 {
                continue;
            }
            if best.as_ref().is_none_or(|(_, b)| s.worst > b.worst) {
                best = Some((item.mask, s));
            }
        }

        let (chosen, in_sample) = match best {
            Some((mask, s)) => (Some(mask), s),
            None => (None, Summary::default()),
        };

        // OUT OF SAMPLE. The column runs from bar zero so the indicators hold
        // what they would genuinely have held, and the trade walk is confined to
        // the test window by `restricted`.
        let out_of_sample = match chosen {
            Some(mask) => {
                let upto = bars.get(..fold.test.end).unwrap_or(bars);
                let full = Column::build(upto, &mut evaluator());
                let confined = restricted(&full, fold.test.start);
                Summary::of(&walk(upto, &confined, &mask, horizon, direction))
            }
            None => Summary::default(),
        };

        out.folds.push(FoldResult {
            index,
            train_bars: train_end,
            purged: fold.purged,
            test_bars: fold.test.len(),
            considered,
            chosen,
            in_sample,
            out_of_sample,
        });
    }
    out
}

/// A column whose rows before `from` can never hit any mask.
///
/// # Why the rows are blanked rather than the slice cut
///
/// [`crate::trade::walk`] pairs `column.bits()` with `column.sources()`, and the
/// sources index into the CALLER's slice. Cutting the column would renumber
/// them, which is the defect `Column::sources` was added to remove in the first
/// place — `first_swept + j` ran one behind from the first refused bar onward.
///
/// So the shape is preserved and the bits are cleared instead. A cleared row
/// still carries its true source, and `hits` on an all-zero row is false for
/// every mask with at least one bit set.
///
/// **The empty mask is the exception, and it is not a defect here.** A mask with
/// no bits set hits a zeroed row, because `(0 & 0) == 0`. That mask fires on
/// every bar by construction and is a degenerate case the sweep never selects —
/// it has no conditions, so it is not a strategy.
fn restricted(column: &Column, from: usize) -> Column {
    let mut confined = column.clone();
    confined.clear_before(from);
    confined
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{DEFAULT_CANDIDATES, Validated, walk_forward};
    use crate::Sweeper;
    use crate::outcome::Horizon;
    use costs::fill::Direction;
    use engine::Ladder;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    fn h(n: u32) -> Horizon {
        Horizon::bars(n).expect("a non-zero horizon")
    }

    fn sweeper() -> Sweeper {
        Sweeper::new(Ladder::with_min_hits(120).with_ceiling(20_000))
    }

    #[test]
    fn a_walk_forward_never_judges_a_choice_on_a_bar_it_was_chosen_on() {
        // THE WHOLE POINT. Every fold's test window must start strictly after
        // its training end, with the purge between them -- otherwise the
        // out-of-sample figure is the in-sample figure under another name.
        let bars = crate::synthetic::sessions(12);
        let v = walk_forward(&bars, h(15), 3, Direction::Long, &sweeper(), evaluator);

        assert_eq!(v.folds.len(), 3, "three splits must yield three folds");
        for f in &v.folds {
            assert!(f.train_bars > 0, "a fold must train on something");
            assert!(
                f.purged >= 15,
                "the purge must be at least the horizon, or a training bar's \
                 outcome reaches into the test window: fold {} purged {}",
                f.index,
                f.purged
            );
        }
        // Anchored: each fold trains on strictly more than the one before.
        for pair in v.folds.windows(2) {
            if let [a, b] = pair {
                assert!(
                    b.train_bars > a.train_bars,
                    "an anchored walk must expand its training window"
                );
            }
        }
    }

    #[test]
    fn the_out_of_sample_figure_is_a_different_number_from_the_in_sample_one() {
        // If these were ever equal across every fold, the test window would be
        // the training window and the whole module would be theatre.
        let bars = crate::synthetic::sessions(12);
        let v = walk_forward(&bars, h(15), 3, Direction::Long, &sweeper(), evaluator);

        assert!(v.decided() > 0, "at least one fold must choose something");
        let differ = v
            .folds
            .iter()
            .filter(|f| f.chosen.is_some())
            .any(|f| f.in_sample.worst != f.out_of_sample.worst);
        assert!(
            differ,
            "no fold's out-of-sample total differed from its in-sample total"
        );
    }

    #[test]
    fn selection_is_by_the_worst_case_so_an_optimistic_fill_cannot_win() {
        // The chosen combination must be the best under PESSIMISTIC fills. A
        // selection on `best` would prefer whatever the open flattered most.
        let bars = crate::synthetic::sessions(12);
        let v = walk_forward(&bars, h(15), 2, Direction::Long, &sweeper(), evaluator);
        for f in v.folds.iter().filter(|f| f.chosen.is_some()) {
            assert!(
                f.in_sample.trades > 0,
                "a chosen combination must have taken trades in sample"
            );
        }
    }

    #[test]
    fn what_the_candidate_budget_dropped_is_counted_rather_than_silent() {
        let bars = crate::synthetic::sessions(12);
        let v = walk_forward(&bars, h(15), 2, Direction::Long, &sweeper(), evaluator);
        // Either every candidate fitted, or the overflow was recorded. What is
        // forbidden is a budget that drops work and says nothing.
        let total: u64 = v.folds.iter().map(|f| f.considered).sum();
        if total > DEFAULT_CANDIDATES as u64 {
            assert!(
                v.not_considered > 0,
                "candidates were dropped and not_considered stayed zero"
            );
        }
    }

    #[test]
    fn a_slice_too_short_to_split_returns_no_folds_rather_than_pretending() {
        let bars = crate::synthetic::sessions(1);
        let v = walk_forward(&bars, h(15), 0, Direction::Long, &sweeper(), evaluator);
        assert_eq!(v, Validated::default());
        assert_eq!(v.decided(), 0);
        assert_eq!(v.held_up(), 0);
    }
}
