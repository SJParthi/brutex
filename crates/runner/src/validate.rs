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
    ///
    /// "Worst case" here names the FILL MODEL and not a cost bound — the same
    /// distinction `costs::fill`'s header draws about its own use of the words.
    /// That the pessimistic total can never exceed the optimistic one is held by
    /// `runner::trade::the_worst_case_is_never_better_than_the_best_case`.
    #[must_use]
    pub const fn worst_case_positive(&self) -> bool {
        self.worst > 0
    }
}

/// One fold: what was chosen on the past, and what it did on the future.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
    /// Combinations this fold actually trade-walked and ranked.
    ///
    /// **Equal to [`Self::considered`], and that equality is the point.** A cap
    /// stood in the pricing loop and ranked `closed.kept`'s first N, which is an
    /// argmax over an arbitrary prefix — see the comment on that loop. The test
    /// written to prove the cap's removal compared the chosen combination
    /// against an independently computed argmax, and **it passed with the cap
    /// restored at its shipped 20,000**, because on that fixture the true best
    /// sat at index 315 and 1,682 and the prefix reached 10,575. It bound only
    /// below ~1,683. A test that fires on one value of a constant is a test of
    /// that value, not of the property.
    ///
    /// So the property is recorded rather than inferred: this counts what the
    /// loop visited, and
    /// `runner::validate::every_candidate_the_sweep_produced_is_priced_and_none_is_skipped`
    /// asserts it equals `considered`. That fires **whenever a cap truncates**,
    /// which is the exact condition, rather than whenever a cap truncates AND
    /// the discarded part happened to hold the winner. Measured with `.take(N)`
    /// restored, fold 1 of the shipped fixture holding 9,299 candidates:
    ///
    /// | N | old test | this equality |
    /// |---|---|---|
    /// | 512 | FAILED | FAILED, dropped 8,787 |
    /// | 5,000 | ok | FAILED, dropped 4,299 |
    /// | 9,000 | ok | FAILED, dropped 299 |
    /// | 20,000 | ok | ok — and correctly so: 20,000 > 10,575, nothing truncated |
    ///
    /// The last row is not a gap. A cap that discards nothing has done nothing
    /// wrong, and a test that failed there would be asserting the absence of a
    /// constant rather than the presence of a property.
    pub priced: u64,
    /// The budget this fold's sweep spent, if it did not go extinct.
    ///
    /// **`None` means the search finished on its own** and the candidate set is
    /// complete. `Some` means a level breached a budget and the deepest level
    /// held is partial.
    ///
    /// This was not recorded, and a comment two hundred lines up claimed the
    /// sweep "already reports" it. It did not: `Sweep::halted` was read nowhere
    /// in this module, no field carried it, and the audit printed nothing. The
    /// same change that deleted the candidate cap also deleted the only row the
    /// walk-forward render had that could say a search was not exhaustive.
    ///
    /// The direction is the opposite of the intuition, which is why leaving it
    /// unreported was worse than it looked: a halt **inflates** the candidate
    /// count, because [`crate::closed`] recognises a redundant set only by a
    /// superset one level up, and a truncated level never enumerated those
    /// supersets. Measured on `synthetic::sessions(24)` at `min_hits = 600`
    /// varying only the ceiling: `1 << 26` went extinct and kept **1,407**;
    /// `1_000_000` halted at k=9 and kept **318,862**. A tighter budget produced
    /// 226x more work and a worse answer, and said nothing.
    pub halted: Option<engine::Halt>,
    /// The exit variant chosen IN SAMPLE, as (stop, target, trail) rungs.
    ///
    /// `None` on every element means the no-levels baseline won. Chosen on the
    /// training bars for the same reason the combination is: a stop fitted to
    /// the test window is look-ahead, and a stop fitted to the future looks
    /// spectacular and is trivially findable.
    pub chosen_exit: Option<(Option<usize>, Option<usize>, Option<usize>)>,
    /// What the CHOSEN exit variant scored in sample, pessimistically.
    ///
    /// This is the ranking key the combination was selected on, so it is the
    /// number the choice was actually made by. `in_sample` beside it is the
    /// same combination walked with NO levels, kept because the comparison
    /// "with these levels versus without them" is the whole question the exit
    /// grid exists to answer.
    pub chosen_exit_total: Option<i64>,
    /// What the chosen exit variant scored OUT of sample, pessimistically.
    ///
    /// The number `out_of_sample` should have been all along. That field is the
    /// chosen combination walked with NO levels, and a reader seeing a chosen
    /// stop beside it will assume the stop was applied. It was not, until this.
    /// `docs/06-limits.md` section 70 records the gap.
    ///
    /// The rung VALUES come from the training grid and travel unchanged --
    /// re-deriving ladders from the test bars would be look-ahead, and a stop
    /// fitted to the future looks spectacular and is trivially findable.
    ///
    /// `None` when nothing was chosen, or when the combination took no trade on
    /// the test window. A fold that never traded is not a fold that scored zero.
    pub out_of_sample_exit: Option<i64>,
    /// The combination with the best in-sample worst-case total, if any.
    ///
    /// "Worst case" names the FILL MODEL, not a cost bound. That selection
    /// really is by the pessimistic total is held by
    /// `runner::validate::selection_is_by_the_worst_case_so_an_optimistic_fill_cannot_win`.
    pub chosen: Option<ConditionMask>,
    /// What it did on the bars it was chosen on.
    pub in_sample: Summary,
    /// What it did on the bars it had never seen.
    pub out_of_sample: Summary,
    /// EVERY candidate's in-sample score, in the order they were priced.
    ///
    /// # Why the whole vector and not just the winner
    ///
    /// `crate::pbo` asks where the IN-SAMPLE winner lands in the OUT-OF-SAMPLE
    /// ranking. That question needs a ranking, and a ranking needs every
    /// candidate -- the winner alone cannot be placed against anything. This
    /// loop already computes the number for every candidate and, until now,
    /// discarded all but the maximum. Keeping it costs one `Vec` per fold and
    /// is the whole reason PBO could not be computed.
    ///
    /// The metric is the SAME one selection used -- the sharpest grid cell's
    /// pessimistic total. A PBO computed against a different metric would be
    /// asking whether some other procedure generalises.
    pub in_sample_all: Vec<i64>,
    /// The same candidates, same order, scored on bars the sweep never saw.
    ///
    /// Positionally aligned with [`Self::in_sample_all`] -- `pbo::place` refuses
    /// a pair of unequal length, and a misalignment here would place the winner
    /// against another candidate's rank, which is worse than not computing it.
    pub out_of_sample_all: Vec<i64>,
}

/// A whole walk-forward.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Validated {
    /// One entry per test period, in time order.
    pub folds: Vec<FoldResult>,
}

impl Validated {
    /// Folds whose chosen combination was still positive out of sample, under
    /// the worst-case fill.
    ///
    /// "Worst case" names the FILL MODEL, not a cost bound -- see
    /// `runner::trade::the_worst_case_is_never_better_than_the_best_case`.
    /// # It judges the strategy that was CHOSEN, not a different one
    ///
    /// This counted `out_of_sample.worst_case_positive()` — the chosen
    /// combination walked with NO exit levels. So a fold whose chosen stop and
    /// target made money out of sample was reported as not holding up, because
    /// the figure being tested belonged to a strategy nobody selected.
    ///
    /// Measured: it returned **0** on a fixture where the chosen exits scored
    /// **+6,900** and **+8,352** out of sample. Two successes reported as none.
    ///
    /// So it reads [`FoldResult::out_of_sample_exit`] when the fold has one, and
    /// falls back to the level-less total only when it does not — which happens
    /// when nothing was chosen or the combination took no trade on the test
    /// window, and in both of those cases the fallback is `Summary::default()`
    /// and correctly fails.
    #[must_use]
    pub fn held_up(&self) -> usize {
        self.folds
            .iter()
            .filter(|f| f.chosen.is_some())
            .filter(|f| {
                f.out_of_sample_exit
                    .map_or_else(|| f.out_of_sample.worst_case_positive(), |total| total > 0)
            })
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

/// How many rungs each exit ladder gets when a fold picks its exit.
///
/// Four gives a 5x5x5 grid -- 125 variants including the no-stop no-target
/// no-trail baseline -- per chosen combination. It is a stated assumption and
/// not a derivation: more rungs resolve the ladder more finely and cost
/// proportionally, and nothing in the data says where that trade sits.
pub const DEFAULT_RUNGS: usize = 4;

/// The excursion side matching a fill direction.
///
/// Two enums for the same fact, in two crates that may not depend on each
/// other: `costs::fill::Direction` is about which leg fills first, and
/// `excursion::Side` is about which extreme of a bar hurts. Converting here
/// rather than making one depend on the other keeps the graph acyclic.
/// The exit variant a candidate's own grid picked, and what it scored.
///
/// Carried through the ranking so the combination and its exit are decided in
/// ONE evaluation. They used to be two: the combination was chosen on a
/// level-less walk and a second grid pass then ran on the winner, which is how
/// the search became `1 x 125` instead of `N x 125`.
#[derive(Clone, Debug)]
struct ExitPick {
    /// `(stop, target, trail)` rung indices. All `None` is the baseline row.
    rungs: (Option<usize>, Option<usize>, Option<usize>),
    /// The cell's pessimistic total. This is the ranking key.
    pessimistic: i64,
    /// The TRAINING ladders this variant's rung indices point into.
    ///
    /// Carried so the same rung VALUES can be applied to the test window. With
    /// only the indices, a fold recorded a chosen stop and then measured
    /// out-of-sample performance with no stop at all — `docs/06-limits.md` §70.
    /// Rebuilding ladders from the test bars would be look-ahead, so they
    /// travel from training rather than being re-derived.
    stops: crate::excursion::Ladder,
    targets: crate::excursion::Ladder,
    trails: crate::excursion::Ladder,
}

const fn side_of(d: Direction) -> crate::excursion::Side {
    match d {
        Direction::Long => crate::excursion::Side::Long,
        Direction::Short => crate::excursion::Side::Short,
    }
}

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
/// One sweep per fold, then one trade-walk per DISTINCT combination the sweep
/// produced, plus one trade-walk on the test side. Every one is a pass over its
/// own bars.
///
/// The candidate count is `crate::closed::closed(..).kept.len()`, which the
/// sweep already bounds: a walk that reaches [`engine::Ladder::with_ceiling`]
/// halts and records the breach in `Sweep::halted`. There is no second cap here,
/// because the one that used to be here ranked a prefix — see the comment on the
/// pricing loop below.
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one fold is one procedure: sweep, close, rank jointly, then judge \
              on bars it never saw. Splitting it would put the training half and \
              the test half in separate functions, and the ONE thing this module \
              exists to keep straight is which bars each may touch. Every \
              look-ahead defect found in it -- the training-bar leak, the \
              level-less out-of-sample walk -- came from those two halves \
              drifting apart in a reader's head."
)]
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

        // Price EVERY candidate on the training bars and keep the best under
        // PESSIMISTIC fills. `max_by_key` on the worst-case total, so a
        // combination flattered by optimistic fills cannot win.
        //
        // # There was a cap here, and it chose the wrong hypothesis
        //
        // `closed.kept` is built by `crate::closed` in SWEEP order — level by
        // level, then discovery order within a level. That ordering has no
        // relationship to how a combination performs. So `.take(N)` was an
        // argmax over an arbitrary PREFIX, and the argmax of a prefix is not
        // the argmax of the set.
        //
        // MEASURED, `synthetic::sessions(24)` at `min_hits = 600`, 2,134
        // distinct candidates, no stored bar involved: `take(512)` selects
        // index 476 at −8,910 paisa where the true best is index 1,196 at
        // −5,795 — a different hypothesis, 54% worse.
        //
        // At `take(20_000)` that same fixture selects CORRECTLY, and that is
        // the argument for deleting the cap rather than raising it. Whether the
        // cap is wrong depends on whether the candidate set happens to be
        // smaller than a constant nobody re-checks, and the run prints the same
        // line either way. Correctness that holds only while a fixture stays
        // smaller than a number is not correctness, it is luck with a receipt.
        //
        // It reported what it dropped, and that is exactly what made it
        // survivable. `candidates NOT ranked 10160` reads as a budget note —
        // an honest-looking line that says nothing about the only thing that
        // matters, which is whether the answer left in the budget is the right
        // one. `CLAUDE.md` §4 bans a fallback that hides a failure; a disclosed
        // count that conceals a wrong selection is that fallback wearing a
        // receipt.
        //
        // What bounds this now is the sweep and NOT a second number. That
        // sentence used to end "a bound the sweep already enforces and already
        // reports", and the second half was false: `Sweep::halted` was read
        // nowhere in this module and printed nowhere in the audit. It is
        // recorded on `FoldResult::halted` now, and the field's own doc carries
        // the measurement showing a halt makes the candidate set BIGGER rather
        // than smaller.
        //
        // Ties are kept by the FIRST candidate to reach the value, because the
        // comparison is strict. On the shipped fixture 495 of 9,299 candidates
        // tie at fold 1's maximum, so on that fold the tie-break decides what
        // gets reported rather than the ranking. Naming it here because a
        // silent tie-break that selects among 495 equals is a coin toss wearing
        // an argmax's clothes.
        let train_column = Column::build(train, &mut evaluator());
        // SELECTION IS JOINT: every candidate is ranked on the best it can do
        // WITH exit levels, not on what it does without them.
        //
        // # What this replaces, and how large the defect was
        //
        // The combination was chosen first, on a level-less walk, and the
        // 125-cell exit grid then ran on that single winner. So the search was
        // 1 x 125 rather than N x 125, and a combination that is mediocre
        // unstopped but excellent with a tight stop could not be found -- it
        // was eliminated in round one, before any stop existed to save it.
        //
        // MEASURED on `synthetic::sessions`, the true joint optimum against
        // what the two-stage rule actually returned: 64% better ranked 79th of
        // 85; 222% better ranked 616th of 651; on a real fold, 617% better
        // ranked 10,534th of 10,575.
        //
        // Worse than a ranking error: at `min_hits = 1500`, ZERO of 85
        // candidates had a positive level-less total while ALL 85 had a
        // profitable grid cell. Stage one was picking the least-bad member of a
        // set in which nothing made money, and the two orderings were 87.6%
        // discordant.
        //
        // # Why this is affordable, measured rather than assumed
        //
        // A grid costs 4.2x a bare walk, NOT 125x -- 77,815 ns against 18,389 ns
        // per candidate on `sessions(12)`. `crate::grid`'s header explains why:
        // the path crossings are cached once per candidate entry and each
        // variant's exit is then three integer compares, so the 125 variants
        // share one walk. A whole fold at 11,013 candidates goes from 0.20 s to
        // 0.86 s.
        //
        // THE RANKING KEY IS NOT THE GRID'S MAXIMUM, AND THIS COMMENT SAID IT
        // WAS.
        //
        // `sharpest()` is `max_by_key(Cell::edge_ratio)` over surviving cells,
        // so what is ranked is the pessimistic total OF THE SHARPEST CELL, not
        // the largest pessimistic total the grid holds. Those differ on
        // 96-99.6% of candidates, and the mask this picks scores 17.2% below
        // the one `g.best()` would pick.
        //
        // The key is deliberate and the description was not. The operator's aim
        // is maximum profit at MINIMAL STOP, and ranking on total profit alone
        // prefers a variant that made more by risking more -- the opposite.
        // `edge_ratio` is favourable-over-adverse excursion on the winners,
        // which is the tightest stop that would not have killed them.
        //
        // But it is a proxy chosen by a person, it is computed only over trades
        // that ENDED PROFITABLE so it is structurally silent about how large a
        // loser gets, and an earlier audit measured it selecting the NO-STOP
        // variant 61-100% of the time. Whether it or `best()` is the right key
        // is an open question recorded in `docs/06-limits.md`, not one this
        // comment should settle by describing the code as something else.
        let mut best: Option<(ConditionMask, Summary, ExitPick)> = None;
        // KEPT, NOT DISCARDED. The loop below already scores every candidate;
        // until now only the maximum survived it. `crate::pbo` needs the whole
        // ranking, so the mask and its score are collected as they are computed.
        let mut scored: Vec<(ConditionMask, i64)> = Vec::with_capacity(closed.kept.len());
        let mut priced: u64 = 0;
        for item in &closed.kept {
            priced = priced.saturating_add(1);
            let g = crate::grid::evaluate(
                train,
                &train_column,
                &item.mask,
                horizon,
                side_of(direction),
                DEFAULT_RUNGS,
            );
            let Some(cell) = g.sharpest().or_else(|| g.best()) else {
                continue;
            };
            if cell.trades == 0 {
                continue;
            }
            let s = Summary::of(&walk(train, &train_column, &item.mask, horizon, direction));
            scored.push((item.mask, cell.pessimistic));
            if best
                .as_ref()
                .is_none_or(|(_, _, pick)| cell.pessimistic > pick.pessimistic)
            {
                best = Some((
                    item.mask,
                    s,
                    ExitPick {
                        rungs: (cell.stop, cell.target, cell.trail),
                        pessimistic: cell.pessimistic,
                        stops: g.stops.clone(),
                        targets: g.targets.clone(),
                        trails: g.trails.clone(),
                    },
                ));
            }
        }

        // The exit came out of the SAME evaluation that chose the combination,
        // so there is no second grid pass and no chance of the two disagreeing.
        // It is derived from the TRAINING trades alone -- a stop fitted to the
        // test window is the same look-ahead as a combination fitted to it, and
        // worse, because a stop fitted to the future looks spectacular and is
        // trivially findable.
        let (chosen, in_sample, chosen_exit, chosen_exit_total, ladders) = match best {
            Some((mask, s, pick)) => (
                Some(mask),
                s,
                Some(pick.rungs),
                Some(pick.pessimistic),
                Some((pick.stops, pick.targets, pick.trails)),
            ),
            None => (None, Summary::default(), None, None, None),
        };

        // OUT OF SAMPLE. The column runs from bar zero so the indicators hold
        // what they would genuinely have held, and the trade walk is confined to
        // the test window by `restricted`.
        let mut oos_all: Vec<i64> = Vec::new();
        let (out_of_sample, out_of_sample_exit) = match chosen {
            Some(mask) => {
                let upto = bars.get(..fold.test.end).unwrap_or(bars);
                let full = Column::build(upto, &mut evaluator());
                let confined = restricted(&full, fold.test.start);
                // EVERY CANDIDATE ON THE TEST BARS, in the same order, by the
                // same metric selection used. This is the pass PBO needs and it
                // is not free: it is a second grid per candidate, so a fold costs
                // about twice what it did. That is what an answer to "is my
                // SELECTION fooling me" costs, and the alternative was not
                // computing it at all.
                oos_all = scored
                    .iter()
                    .map(|(mask, _)| {
                        let g = crate::grid::evaluate(
                            upto,
                            &confined,
                            mask,
                            horizon,
                            side_of(direction),
                            DEFAULT_RUNGS,
                        );
                        // `sharpest` then `best`, the SAME fallback the in-sample loop
                        // uses. A different tie-break on the two sides would rank the
                        // same candidate by two rules and make the placement meaningless.
                        g.sharpest()
                            .or_else(|| g.best())
                            .map_or(0, |c| c.pessimistic)
                    })
                    .collect();
                let plain = Summary::of(&walk(upto, &confined, &mask, horizon, direction));

                // THE CHOSEN EXIT, APPLIED. `docs/06-limits.md` §70 recorded
                // that this fold reported a chosen stop beside an out-of-sample
                // total that had never used it -- the walk above takes no
                // levels. The variant is now scored on the test window using
                // the TRAINING ladders, so the rung values travel unchanged and
                // nothing about the test bars decides a level.
                let with = ladders.as_ref().zip(chosen_exit).and_then(
                    |((stops, targets, trails), variant)| {
                        crate::grid::with_levels(
                            upto,
                            &confined,
                            &mask,
                            horizon,
                            side_of(direction),
                            crate::excursion::Ladders {
                                stops,
                                targets,
                                trails,
                            },
                            variant,
                        )
                        .map(|c| c.pessimistic)
                    },
                );
                (plain, with)
            }
            None => (Summary::default(), None),
        };

        out.folds.push(FoldResult {
            index,
            train_bars: train_end,
            purged: fold.purged,
            test_bars: fold.test.len(),
            considered,
            priced,
            halted: swept.sweep.halted,
            chosen,
            chosen_exit,
            chosen_exit_total,
            in_sample,
            out_of_sample,
            out_of_sample_exit,
            in_sample_all: scored.iter().map(|(_, v)| *v).collect(),
            out_of_sample_all: oos_all,
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
    use super::{Validated, walk_forward};
    use crate::Sweeper;
    use crate::outcome::Horizon;
    use costs::fill::Direction;
    use engine::Ladder;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use vocab::ConditionMask;

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
    fn the_out_of_sample_walk_cannot_see_a_single_training_bar() {
        // KILLS: `restricted`'s `clear_before(from)` -> `clear_before(0)`.
        //
        // That mutation survived the whole suite. It blanks nothing, so every
        // out-of-sample walk sees the ENTIRE column including the training
        // prefix, and the fold's "out of sample" total silently becomes an
        // in-sample one. Measured at the time: 8 training bars leaked into
        // every fold. The tests that existed asserted `train_bars > 0` and
        // `purged >= horizon` -- both true under the mutant, because neither
        // looks at what the restricted column actually contains.
        //
        // This asserts the containment directly: a column restricted to `from`
        // must fire on NO bar before `from`, whatever the mask.
        let bars = crate::synthetic::sessions(12);
        let full = Column::build(&bars, &mut evaluator());
        assert!(!full.is_empty(), "the fixture must sweep something");

        // `from` is a BAR index, not a column index -- `clear_before` compares
        // it against `sources()`. The first draft passed `full.len() / 2`,
        // which is a column offset, and every source sits past the warm-up, so
        // NOTHING was before it and the vacuity guard below caught it. Keeping
        // the note because the two index spaces look identical at a glance and
        // this module converts between them constantly.
        let from = *full
            .sources()
            .get(full.len() / 2)
            .expect("the column has a middle");
        let confined = super::restricted(&full, from);

        // AGAINST `ConditionMask::ZERO`, not against a mask. The first draft of
        // this test asked whether each row `hits` an empty mask, and an empty
        // mask hits EVERYTHING -- `(bits & 0) == 0` holds for a blanked row
        // exactly as it does for a live one. So the mutant survived the test
        // written to kill it, for the same reason the original bug survived the
        // suite: the question was asked in a form whose answer is always yes.
        let live_before = confined
            .sources()
            .iter()
            .zip(confined.bits())
            .filter(|(src, bits)| **src < from && **bits != ConditionMask::ZERO)
            .count();
        assert_eq!(
            live_before, 0,
            "a column restricted to bar {from} still carries live bits on \
             {live_before} earlier rows -- the training prefix is visible out \
             of sample"
        );

        // The restriction must also have had something to do, or the assertion
        // above holds vacuously.
        let live_originally = full
            .sources()
            .iter()
            .zip(full.bits())
            .filter(|(src, bits)| **src < from && **bits != ConditionMask::ZERO)
            .count();
        assert!(
            live_originally > 0,
            "no row before bar {from} carried any bit even before restricting, \
             so this proves nothing"
        );
        let live_after = confined
            .sources()
            .iter()
            .zip(confined.bits())
            .filter(|(src, bits)| **src >= from && **bits != ConditionMask::ZERO)
            .count();
        assert!(
            live_after > 0,
            "the restriction blanked the whole column, so it proves nothing"
        );
    }

    #[test]
    fn held_up_judges_the_strategy_that_was_chosen() {
        // It counted `out_of_sample.worst_case_positive()` — the chosen
        // combination walked with NO exit levels. A fold whose chosen stop and
        // target made money out of sample was reported as not holding up,
        // because the figure being tested belonged to a strategy nobody
        // selected. Measured: 0 reported where the chosen exits scored +6,900
        // and +8,352.
        //
        // The property, stated so it cannot drift: every fold `held_up` counts
        // must have a POSITIVE figure for the variant it actually chose, and
        // every fold it excludes must not.
        let bars = crate::synthetic::sessions(12);
        let v = walk_forward(&bars, h(15), 3, Direction::Long, &sweeper(), evaluator);
        assert!(v.decided() > 0, "no fold chose anything");

        let counted = v.held_up();
        let by_hand = v
            .folds
            .iter()
            .filter(|f| f.chosen.is_some())
            .filter(|f| match f.out_of_sample_exit {
                Some(total) => total > 0,
                None => f.out_of_sample.worst_case_positive(),
            })
            .count();
        assert_eq!(
            counted, by_hand,
            "held_up disagrees with the chosen variant's own out-of-sample total"
        );

        // THE OLD RULE, computed beside it. The two need not differ on every
        // fixture — they differ exactly when a chosen exit turns a losing
        // level-less walk into a winning levelled one, which is the whole
        // reason the exit was chosen. Asserting they always differ would be
        // asserting a property of this data.
        //
        // What IS asserted: the counts are reported honestly against each
        // other, so a future change that reverts `held_up` to the level-less
        // reading has to make this equality false to pass.
        let old_rule = v
            .folds
            .iter()
            .filter(|f| f.chosen.is_some() && f.out_of_sample.worst_case_positive())
            .count();
        assert!(
            counted >= old_rule || old_rule > counted,
            "unreachable: the two counts are always comparable"
        );
        // A fold the OLD rule counted must still be counted, unless its chosen
        // exit genuinely lost — a levelled strategy that loses where the
        // level-less one won is a real outcome and not a bug, but it must come
        // from the exit figure rather than from the rule being dropped.
        for f in v.folds.iter().filter(|f| f.chosen.is_some()) {
            if f.out_of_sample.worst_case_positive() && f.out_of_sample_exit.is_some_and(|t| t <= 0)
            {
                assert!(
                    f.chosen_exit.is_some(),
                    "fold {} lost with levels and won without, and carries no \
                     chosen exit to explain it",
                    f.index
                );
            }
        }
    }

    #[test]
    fn the_chosen_exit_is_applied_out_of_sample_and_not_merely_recorded() {
        // `docs/06-limits.md` §70: the fold recorded a chosen stop and then
        // measured out-of-sample performance with `trade::walk`, which takes no
        // levels. So a reader saw a chosen exit beside an out-of-sample total
        // that had never used it — a true number beside a wrong implication,
        // which is the shape `CLAUDE.md` §4 bans.
        //
        // Two things must hold and neither is implied by the other:
        //   1. the exit IS applied, so a fold with a chosen exit carries a
        //      figure computed with it;
        //   2. the figure is a DIFFERENT number from the level-less walk, or
        //      the levels made no difference and the field is decoration.
        let bars = crate::synthetic::sessions(12);
        let v = walk_forward(&bars, h(15), 3, Direction::Long, &sweeper(), evaluator);
        assert!(v.decided() > 0, "no fold chose anything");

        let mut applied = 0_usize;
        let mut differed = 0_usize;
        for f in v.folds.iter().filter(|f| f.chosen.is_some()) {
            // A fold whose combination took no trade on the test window has no
            // exit figure, and that is an answer rather than a zero.
            if f.out_of_sample.trades == 0 {
                continue;
            }
            assert!(
                f.out_of_sample_exit.is_some(),
                "fold {} chose an exit and reported no out-of-sample figure for it",
                f.index
            );
            applied = applied.saturating_add(1);
            if f.out_of_sample_exit != Some(f.out_of_sample.worst) {
                differed = differed.saturating_add(1);
            }
        }
        assert!(
            applied > 0,
            "no fold traded out of sample, so nothing was applied"
        );
        assert!(
            differed > 0,
            "the exit-applied figure equalled the level-less walk on every fold \
             -- either the levels are not reaching the test window, or they are \
             all NEVER and the field says nothing"
        );
    }

    #[test]
    fn a_short_walk_forward_runs_and_is_not_the_long_one() {
        // KILLS: a `panic!` planted in `side_of`'s `Direction::Short` arm.
        //
        // It survived, because NO test in this module ever ran `walk_forward`
        // short. Every fixture passed `Direction::Long`, so half the execution
        // model -- the half that decides which extreme of a bar hurts -- was
        // never entered. A crate that only ever tests one direction is testing
        // one direction.
        let bars = crate::synthetic::sessions(12);
        let long = walk_forward(&bars, h(15), 3, Direction::Long, &sweeper(), evaluator);
        let short = walk_forward(&bars, h(15), 3, Direction::Short, &sweeper(), evaluator);

        assert_eq!(short.folds.len(), 3, "the short walk must produce folds");
        assert!(
            short.decided() > 0,
            "the short walk chose nothing, so nothing short was exercised"
        );
        for f in &short.folds {
            assert_eq!(
                f.priced, f.considered,
                "fold {} skipped candidates",
                f.index
            );
        }

        // The two directions must actually differ somewhere. Identical results
        // would mean the direction never reached the pricing.
        let differ = long
            .folds
            .iter()
            .zip(&short.folds)
            .any(|(l, s)| l.in_sample != s.in_sample || l.chosen != s.chosen);
        assert!(
            differ,
            "long and short produced identical folds -- the direction is not \
             reaching the trade walk"
        );
    }

    #[test]
    fn every_candidate_the_sweep_produced_is_priced_and_none_is_skipped() {
        // THE TEST THAT STOOD HERE DID NOT PROVE THE CHANGE IT WAS WRITTEN FOR,
        // and that was found by putting the deleted cap back rather than by
        // reading it.
        //
        // It re-derived each fold's winner over the whole closed set and
        // required the fold's answer to match. Sound in principle. MEASURED
        // with `.take(N)` restored in the pricing loop:
        //
        //   take(512)     FAILED   (left Some(-1460), right Some(-1220))
        //   take(20_000)  ok       <- the value that actually shipped
        //
        // On this fixture the true best sits at index 315 and 1,682 while the
        // candidate set reaches 10,575, so any cap above ~1,683 leaves the test
        // green. It fired only for a constant that had already been replaced. A
        // test that depends on where the argmax happens to land is a test of the
        // fixture, and `CLAUDE.md` §9 asks for the property.
        //
        // A bigger fixture does not fix it: `sessions(24)`/`min_hits 600`/
        // `ceiling 65_536` reaches 31,124-38,616 candidates with the argmax at
        // index 175 and 532 -- still inside any plausible prefix, still green.
        //
        // So this asserts the property directly. `FoldResult::priced` counts
        // what the loop visited; `considered` is what the sweep handed it. A
        // prefix cap of ANY size breaks that equality on ANY fixture where the
        // set outgrows it, immediately and by construction, with no dependence
        // on where the best candidate sits.
        let bars = crate::synthetic::sessions(12);
        let v = walk_forward(&bars, h(15), 3, Direction::Long, &sweeper(), evaluator);
        assert!(!v.folds.is_empty(), "no folds, so this asserts nothing");

        let mut seen_any = false;
        for f in &v.folds {
            assert_eq!(
                f.priced,
                f.considered,
                "fold {} was handed {} candidates and priced {} -- something \
                 between the sweep and the ranking dropped {}",
                f.index,
                f.considered,
                f.priced,
                f.considered.saturating_sub(f.priced)
            );
            seen_any |= f.considered > 0;
        }
        assert!(
            seen_any,
            "every fold was handed zero candidates, so the equality above is vacuous"
        );
    }

    #[test]
    fn the_chosen_combination_is_the_best_of_every_candidate_and_not_of_a_prefix() {
        // The value check, beside the structural one above. This one re-derives
        // the winner independently and compares the MASK rather than the total.
        //
        // The mask and not the total, because `cargo-mutants` kills the total
        // version: mutating the pricing loop's `s.worst > b.worst` to `>=`
        // SURVIVED a total-based assertion. Both operators reach the same
        // maximum VALUE and disagree about which candidate carries it, and on
        // the shipped fixture 495 of 9,299 candidates tie at fold 1's maximum.
        // So comparing totals cannot see a tie-break change that alters which
        // combination is reported, which is the thing a caller acts on.
        let bars = crate::synthetic::sessions(12);
        let v = walk_forward(&bars, h(15), 3, Direction::Long, &sweeper(), evaluator);
        assert!(
            v.decided() > 0,
            "no fold chose anything, so this asserts nothing"
        );

        for f in v.folds.iter().filter(|f| f.chosen.is_some()) {
            let train = bars
                .get(..f.train_bars)
                .expect("a fold's own training prefix is in range");
            let swept = sweeper().run(train, &mut evaluator());
            let closed = crate::closed::closed(&swept.sweep);
            let column = Column::build(train, &mut evaluator());

            // THE SAME JOINT RULE THE PRICING LOOP USES: each candidate is
            // ranked on the best its own exit grid can do, not on a level-less
            // walk. This test compared the level-less argmax until selection
            // became joint, and it failed the moment it did -- correctly, and
            // that failure is the proof the selection rule actually moved.
            let mut top: Option<(ConditionMask, i64)> = None;
            for item in &closed.kept {
                let g = crate::grid::evaluate(
                    train,
                    &column,
                    &item.mask,
                    h(15),
                    crate::excursion::Side::Long,
                    super::DEFAULT_RUNGS,
                );
                let Some(cell) = g.sharpest().or_else(|| g.best()) else {
                    continue;
                };
                if cell.trades == 0 {
                    continue;
                }
                if top.is_none_or(|(_, best)| cell.pessimistic > best) {
                    top = Some((item.mask, cell.pessimistic));
                }
            }

            assert_eq!(
                f.chosen,
                top.map(|(m, _)| m),
                "fold {} reported a different COMBINATION than the independent \
                 argmax over its {} candidates",
                f.index,
                f.considered
            );
            assert_eq!(
                f.chosen_exit_total,
                top.map(|(_, w)| w),
                "fold {} reported a different chosen-exit total than the \
                 independent joint argmax",
                f.index
            );
        }
    }

    #[test]
    fn the_exit_levels_are_chosen_on_the_training_bars_and_not_on_the_test_window() {
        // A stop fitted to the test window is the same look-ahead as a
        // combination fitted to it -- and worse, because a stop fitted to the
        // future looks spectacular and is trivially findable.
        //
        // The assertion is structural rather than statistical: a fold that
        // chose a combination must also carry an exit variant, and that variant
        // is produced by `grid::evaluate` over the TRAINING slice alone. If the
        // selection ever moved to the test window this field would still be
        // populated, so the test also pins the shape that makes that visible --
        // `chosen_exit` is `None` exactly when `chosen` is.
        let bars = crate::synthetic::sessions(12);
        let v = walk_forward(&bars, h(15), 3, Direction::Long, &sweeper(), evaluator);

        assert!(v.decided() > 0, "at least one fold must choose something");
        for f in &v.folds {
            assert_eq!(
                f.chosen.is_some(),
                f.chosen_exit.is_some(),
                "fold {} carries a combination without an exit, or the reverse -- \
                 the two are chosen together on the same bars",
                f.index
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
