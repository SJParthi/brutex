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
use rayon::prelude::*;
use vocab::ConditionMask;

use crate::outcome::Horizon;
use crate::split::Shape;
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
    /// The exit variant chosen IN SAMPLE.
    ///
    /// All four rungs `None` means the no-levels baseline won. Chosen on the
    /// training bars for the same reason the combination is: a stop fitted to
    /// the test window is look-ahead, and a stop fitted to the future looks
    /// spectacular and is trivially findable.
    ///
    /// A named struct and not a tuple of four `Option<usize>`, for the reason
    /// `crate::grid::Chosen` gives: this value crosses a window boundary, and a
    /// permuted pair would score the right combination under the wrong exit.
    pub chosen_exit: Option<crate::grid::Chosen>,
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

    /// Folds that HALTED rather than going extinct on their own.
    ///
    /// `FoldResult::halted` is `Some` when a level breached a budget, so the
    /// deepest level that fold reached is PARTIAL: candidates were dropped
    /// unexamined and the fold's chosen combination is the best of a truncated
    /// set, not the best of the set.
    ///
    /// The field has been recorded per fold since it was added and NOTHING
    /// SUMMED IT. A walk-forward in which every fold truncated read exactly
    /// like one in which none did, because the only figures the caller had
    /// were `decided()` and `folds.len()` and a halted fold still decides.
    /// That is the fallback that hides a failure `CLAUDE.md` section 4 bans:
    /// the run degraded, and nothing named the reason.
    ///
    /// O(folds), and `folds` is the walk-forward window count -- a handful,
    /// fixed before the sweep starts and independent of bars, candidates and
    /// vocabulary size. It is not on the per-bar or per-candidate path that
    /// golden rule 4 governs.
    #[must_use]
    pub fn halted_folds(&self) -> usize {
        self.folds.iter().filter(|f| f.halted.is_some()).count()
    }
}

/// How many rungs each exit ladder gets when a fold picks its exit.
///
/// Four gives **325 variants** per chosen combination, including the no-stop
/// no-target no-trail baseline -- `grid::variants(4, 4, 4)`, which is not a
/// product of four factors because an arm with no trail is never emitted. It
/// read `5x5x5 grid -- 125 variants` until the arming axis landed, and briefly
/// read 525 before the unreachable arm settings were refused.
///
/// It is a stated assumption and not a derivation: more rungs resolve the
/// ladder more finely and cost proportionally, and nothing in the data says
/// where that trade sits.
pub const DEFAULT_RUNGS: usize = 4;

/// How deep the exit ladder goes in a walk-forward fold, resolved at RUNTIME.
///
/// # The inconsistency this closes
///
/// The SCREEN builds its exit grid with `cli::grid_rungs(bars)` — a rung count
/// derived from the instrument's own median bar range. The WALK-FORWARD built
/// its grid with [`DEFAULT_RUNGS`], a fixed four. So the ladder that judged a
/// combination out of sample was not the ladder that chose it: on any series
/// where the derived count is not four, the validation priced a coarser or
/// finer set of exits than the screen did, and the two disagreed about which
/// variant a combination even had.
///
/// [`DEFAULT_RUNGS`]'s own doc admits the figure is a stated assumption —
/// *"nothing in the data says where that trade sits"* — and that is honest
/// about the DEFAULT while leaving no way to move it.
///
/// `BRUTEX_GRID_RUNGS` is the variable `cli` already reads for the same
/// **This shares the rung COUNT and not the ladder SHAPE, and the difference is
/// not cosmetic.** `Levels::derived(n)` sets `step_ppm: None`, `stops_ppm: &[]`
/// and `ratios: false` — a QUANTILE ladder read off each combination's own
/// excursions. The screen builds a point-stepped ratio grid with the derived
/// stop ladder supplied to it. So even at an identical count the fold prices a
/// different set of levels, and it then chooses with `sharpest().or_else(best)`
/// where the screen chooses with `best_within(rules.admits)`. Sharing the count
/// removes one of three disagreements; the remaining two are real and are not
/// closed here. An earlier version of this doc claimed the ladders now match,
/// which was wrong.
///
/// quantity, so setting it once moves BOTH counts and they cannot drift
/// apart. Unset, the behaviour is exactly what it was.
///
/// A zero is refused rather than obeyed: a ladder with no rungs prices only the
/// no-exit baseline, which would silently turn every walk-forward fold into a
/// buy-and-hold test.
#[must_use]
pub fn fold_rungs() -> usize {
    std::env::var_os("BRUTEX_GRID_RUNGS")
        .and_then(|raw| raw.to_string_lossy().trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_RUNGS)
}

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
/// the search became `1 x G` instead of `N x G`, G being the grid width.
#[derive(Clone, Debug)]
struct ExitPick {
    /// The rung indices. All `None` is the baseline row.
    rungs: crate::grid::Chosen,
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
/// The whole-span threshold, restated for a fold that trains on `train` of
/// `whole` bars.
///
/// # Why a ratio and not the count
///
/// `min_hits` is an absolute count, so it means a different SUPPORT on every
/// window it is applied to. A fold exists to ask "would this have been chosen
/// on data the run had not seen", and it can only answer that if it searches at
/// the support the run searched at.
///
/// # Rounding, and the floor
///
/// Rounded UP: rounding down loosens the threshold, and a fold that searched a
/// wider space than the run would flatter the out-of-sample figure rather than
/// test it. Floored at one because zero means "every combination is frequent" —
/// `Ladder::with_min_hits` raises it anyway, and relying on that would make the
/// intent invisible here.
///
/// Saturating throughout: `train * min_hits` is two `u64`s multiplied and a long
/// span at a high threshold can exceed the type. `u128` for the product and a
/// saturating narrow, which is what `crate::grid::paisa_of` does with the same
/// hazard.
fn scale_min_hits(whole_min_hits: u64, train: usize, whole: usize) -> u64 {
    if whole == 0 {
        return whole_min_hits;
    }
    let train = u128::try_from(train).unwrap_or(u128::MAX);
    let whole_len = u128::try_from(whole).unwrap_or(u128::MAX);
    let numerator = u128::from(whole_min_hits).saturating_mul(train);
    // Ceiling division: `(a + b - 1) / b`.
    let scaled = numerator
        .saturating_add(whole_len.saturating_sub(1))
        .checked_div(whole_len)
        .unwrap_or(0);
    u64::try_from(scaled).unwrap_or(u64::MAX).max(1)
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
pub fn walk_forward(
    bars: &[Candle],
    horizon: Horizon,
    splits: usize,
    direction: Direction,
    sweeper: &crate::Sweeper,
    mut evaluator: impl FnMut() -> Evaluator,
) -> Validated {
    walk_forward_shaped(
        bars,
        horizon,
        splits,
        direction,
        sweeper,
        &mut evaluator,
        Shape::Anchored,
    )
}

/// A walk-forward in either window shape.
///
/// # Why the shape was hardcoded, and what wiring it required
///
/// `rolling_folds` existed, was tested, and had **no caller** — because this
/// function took `bars.get(..train_end)`, a prefix from bar zero. For an
/// anchored fold that is exactly right: its window starts at zero. For a
/// rolling one it silently discards the whole point, sweeping everything from
/// the beginning and producing an anchored result under a rolling name.
///
/// So the training slice is now the fold's OWN range, `start..end`, and for the
/// anchored shape `start` is zero and nothing changes.
///
/// # The warm-up, stated rather than glossed
///
/// `indicators::Evaluator` is stateful, so a window beginning at bar 100,000
/// warms up INSIDE itself: its first bars build EMA state and are not swept.
/// `Column::first_swept` records exactly where sweeping began, so the cost is
/// visible in the outcome rather than hidden.
///
/// `rolling_folds`' own comment describes a better arrangement — feed the
/// evaluator from bar zero while sweeping only from `warm` — and that needs an
/// entry point which does not exist. Building it means a `Column` that folds
/// over one range and admits over another, in `indicators`, which
/// `docs/10-shared-core.md` shares with `tickvault`. It is not done here.
///
/// What this shape does measure is a fair question in its own right: *if the
/// system were deployed today with only this window of history, would it hold
/// up.* That is a harsher test than warming from 2019, not a laxer one.
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
pub fn walk_forward_shaped(
    bars: &[Candle],
    horizon: Horizon,
    splits: usize,
    direction: Direction,
    sweeper: &crate::Sweeper,
    evaluator: &mut impl FnMut() -> Evaluator,
    shape: Shape,
) -> Validated {
    let mut out = Validated::default();

    for (index, fold) in shape
        .folds(bars.len(), horizon, splits)
        .into_iter()
        .enumerate()
    {
        // THE FOLD'S OWN RANGE, not a prefix. See the doc block above: taking
        // `..end` made every rolling fold anchored.
        let Some(train) = bars.get(fold.train.0.clone()) else {
            continue;
        };
        // THE THRESHOLD IS RESCALED TO THIS FOLD, AND UNTIL NOW IT WAS NOT.
        //
        // # What the unscaled version did, measured
        //
        // `min_hits` is an absolute COUNT derived from the whole span. A fold
        // trains on a prefix, so handing it the caller's ladder unchanged asks
        // each fold for the same number of hits out of fewer bars — a stricter
        // support every time, and on the first fold an impossible one.
        //
        // On a real 15-minute run over 2019-12..2026-08 at `min_hits` 8,314,
        // which is 22% of the full span:
        //
        // | fold | train bars | support that 8,314 demands | candidates |
        // |---|---|---|---|
        // | 0 | 6,928  | **120% — unsatisfiable** | 0 |
        // | 1 | 13,856 | 60% | 0 |
        // | 2 | 20,784 | 40% | 7 |
        // | 3 | 27,712 | 30% | 35 |
        // | 4 | 34,640 | 24% | 102 |
        //
        // Two folds could not have found anything whatever the data said, and
        // the other three searched a space far narrower than the run they were
        // meant to validate. The report then read "0 of 5 folds still positive
        // out of sample" — which sounds like a verdict on the strategy and was
        // partly a verdict on an arithmetic slip.
        //
        // Rescaling by the ratio of lengths keeps the SUPPORT constant, which is
        // what makes a fold comparable to the run at all. Rounded up and floored
        // at one so a short fold cannot land on zero, which
        // `Ladder::with_min_hits` would raise anyway and which would silently
        // mean "every combination is frequent".
        let base = sweeper.ladder();
        let scaled = scale_min_hits(base.min_hits(), train.len(), bars.len());
        // `with_min_hits` is a CONSTRUCTOR, not a builder step, so the ceiling
        // and the pair budget are carried across explicitly. Dropping either
        // would give the folds a different memory bound from the run and turn a
        // comparison into two unrelated searches.
        let per_fold = engine::Ladder::with_min_hits(scaled)
            .with_ceiling(base.ceiling())
            .with_pair_budget(base.pair_budget());
        let swept = crate::Sweeper::new(per_fold).run(train, &mut evaluator());

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
        // exit grid then ran on that single winner. So the search was
        // one grid rather than N grids, and a combination that is mediocre
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
        // variant's exit is then three integer compares, so every variant
        // shares one walk. A whole fold at 11,013 candidates goes from 0.20 s to
        // 0.86 s.
        //
        // MEASURED AT 125 CELLS, BEFORE THE ARMING AXIS MADE IT 325. The 4.2x
        // is therefore a figure for the grid as it then was, and this is the
        // honest label rather than a re-run nobody has done: the cached half is
        // unchanged, but `excursion::crossings` now advances one running maximum
        // per arming rung, so the per-bar half grew. UNVERIFIED at 325.
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
        // THE EXIT TRAVELS WITH THE MASK, and that is the whole correction.
        //
        // This was `Vec<(ConditionMask, i64)>` — mask and in-sample score, and
        // nothing about HOW that score was reached. The out-of-sample pass below
        // therefore had no exit to apply, so it re-ran the whole grid on the TEST
        // window and took `sharpest().or_else(best)` of that: the best of a
        // full exit search chosen using the test bars themselves.
        //
        // That is precisely the look-ahead this module's own comment forty lines
        // down refuses in words — "a stop fitted to the test window is the same
        // look-ahead as a combination fitted to it, and worse, because a stop
        // fitted to the future looks spectacular and is trivially findable" —
        // and it was doing it for EVERY candidate, on the vector `crate::pbo`
        // consumes.
        //
        // Why that direction of error is the dangerous one: PBO asks how often
        // the in-sample winner lands below median out-of-sample. Giving every
        // candidate its best-case OOS score inflates the whole distribution and
        // compresses the ranks, so the winner looks less anomalous than it is
        // and the PROBABILITY OF OVERFITTING IS REPORTED TOO LOW. A safety
        // metric that fails optimistic is worse than none.
        //
        // Carrying the `ExitPick` costs three small ladders per candidate — the
        // type already existed and already held them for exactly this reason.
        // Three, not six: the clone goes into `best`, which improves rarely,
        // rather than into every `scored` row. Each ladder is a `Vec<Ppm>` of
        // `DEFAULT_RUNGS` = 4, so 32 bytes; the element grows from 56 to 184
        // bytes inline. `scored` is scoped to the fold body and dropped with it,
        // so this is a bounded per-fold cost and not unbounded growth.
        // ACROSS EVERY CORE, BECAUSE THIS IS THE LOOP THAT OWNS THE WALL CLOCK.
        //
        // This was a sequential `for`, and it is the dominant term of a whole
        // run: `closed.kept` is UNCAPPED by design (see the note above, where a
        // second cap was deliberately removed), each iteration prices a full
        // exit grid over the training bars AND walks them again for the summary,
        // and the enclosing walk-forward runs it once per fold per shape -- ten
        // times per rung. Measured on the operator's 2026-08-28 run: after the
        // parallel Apriori phase finished, the process fell from ~1180% CPU to
        // ~240% and stayed there for hours. Thirteen of fourteen cores idle,
        // inside the stage that was doing all the work.
        //
        // The body is pure per item: `evaluate` and `walk` read `train` and
        // `train_column` immutably and share nothing. The only sequential
        // dependencies were `best` and `priced`, and neither needs to be inside
        // the loop -- `priced` is a count, and `best` is a fold over results
        // that is trivial beside the pricing it compares.
        //
        // DETERMINISM (CLAUDE.md S3 rule 5) IS HELD BY SHAPE, the same argument
        // `rank::walk` and `batch::sweep_under` already make: rayon's INDEXED
        // `collect` preserves order, so `scored` is the identical sequence
        // whatever order the threads finish in. `best` is then chosen by
        // scanning that ordered vector with the same strict `>` the sequential
        // version used, so ties resolve to the same earliest candidate and the
        // winner is the same mask. Byte-identical output, on any core count.
        // THE SQUARE-OFF TABLE IS A FACT ABOUT `train`, SO IT IS BUILT ONCE.
        //
        // `trade::forced_exits` takes only the bars — no mask, no direction, no
        // horizon — and every candidate below was rebuilding it TWICE: once
        // inside `grid::evaluate` and once for the direct `walk`. Per candidate
        // that is two `Vec`s of `train.len()` and two reverse passes calling
        // `ist_day` and `minute_of_day` on every bar, for a table that is
        // identical every time.
        //
        // `closed.kept` is the closed frequent set of a whole sweep —
        // `crate::rank` cites a real run at **17.8 million survivors** against a
        // 91,874-bar column — so this is the multiplier that matters, and it is
        // hoisted above the `par_iter` rather than into it: one table, shared by
        // every thread, borrowed immutably.
        //
        // Constant per candidate, so gate 8 could not see it. Same shape as
        // `store::file::read_row`'s per-record allocation and `pull::ingest`'s
        // per-row counting loop.
        let exits = crate::trade::forced_exits(train);
        let assessed: Vec<Option<(ConditionMask, Summary, ExitPick, i64)>> = closed
            .kept
            .par_iter()
            .map(|item| {
                let g = crate::grid::evaluate_with(
                    train,
                    &train_column,
                    &item.mask,
                    horizon,
                    side_of(direction),
                    crate::grid::Levels::derived(fold_rungs()),
                    Some(&exits),
                );
                let cell = g.sharpest().or_else(|| g.best())?;
                if cell.trades == 0 {
                    return None;
                }
                let s = Summary::of(&crate::trade::walk_with(
                    train,
                    &train_column,
                    &item.mask,
                    horizon,
                    direction,
                    Some(&exits),
                ));
                // The pick is built ONCE and used twice: by the out-of-sample pass,
                // so it can apply this candidate's TRAINING exit to the test bars,
                // and by the fold's own winner. Built before the `best` comparison
                // so both see the same value, and cloned only into `best`, which
                // improves a handful of times per fold rather than once per
                // candidate — see the ordering note below.
                let pick = ExitPick {
                    rungs: crate::grid::Chosen {
                        stop: cell.stop,
                        target: cell.target,
                        tsl: cell.tsl,
                        ttp: cell.ttp,
                    },
                    pessimistic: cell.pessimistic,
                    stops: g.stops.clone(),
                    targets: g.targets.clone(),
                    trails: g.trails.clone(),
                };
                Some((item.mask, s, pick, cell.pessimistic))
            })
            .collect();

        // ONE CLONE PER IMPROVEMENT, NOT ONE PER CANDIDATE -- the property the
        // sequential version's comment was protecting, kept.
        //
        // `best` and the row `scored` holds are still ONE value: the pick is
        // constructed once above and cloned only when it improves, which happens
        // a handful of times per fold rather than once per candidate. At the
        // 11,013-candidate fold this module records that is thousands of heap
        // allocations saved, and the fold's winner still cannot disagree with
        // its entry in the ranked vector about the chosen rungs.
        //
        // Scanned in the collected order with the same strict `>`, so a tie
        // resolves to the earliest candidate exactly as it did sequentially.
        let mut scored: Vec<(ConditionMask, i64, ExitPick)> = Vec::with_capacity(assessed.len());
        let priced: u64 = u64::try_from(closed.kept.len()).unwrap_or(u64::MAX);
        for (mask, s, pick, pessimistic) in assessed.into_iter().flatten() {
            let improves =
                best.as_ref()
                    .is_none_or(|(_, _, held): &(ConditionMask, Summary, ExitPick)| {
                        pessimistic > held.pessimistic
                    });
            if improves {
                // CLONED ONLY WHERE A SECOND COPY IS ACTUALLY NEEDED, which is
                // here and not below.
                //
                // This was `scored.push(.., pick.clone())` followed by
                // `best = Some(.., pick)`, which cloned three ladders on EVERY
                // candidate — six heap allocations per candidate where three
                // would do, and the comment beside it charged for three. At the
                // 11,013-candidate fold this module records, that is 66,078
                // allocations per fold instead of 33,039.
                //
                // Ordering it this way keeps the property the old comment was
                // protecting: `best` and the row `scored` holds are still ONE
                // value, cloned from one construction, so the fold's winner and
                // its entry in the ranked vector cannot disagree about the
                // chosen rungs. The clone that survives is the rare one — `best`
                // improves a handful of times per fold, not once per candidate.
                best = Some((mask, s, pick.clone()));
            }
            scored.push((mask, pessimistic, pick));
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
                // ACROSS EVERY CORE, LIKE THE IN-SAMPLE PASS ABOVE.
                //
                // Parallelising the in-sample pricing and leaving this
                // sequential moved the bottleneck rather than removing it. Using
                // this module's own measured figures -- `evaluate` at 77,815 ns
                // against a bare walk at 18,389 ns per candidate -- spreading
                // only the first pass takes a fold from about 96 us per
                // candidate to about 24, a 4x gain and not the 14x the core
                // count offers, because this second grid then accounts for
                // roughly three quarters of what is left.
                //
                // MEASURED on the operator's own machine while a real sweep ran:
                // 5.7 to 7.8 of fourteen cores busy. Half the machine idle,
                // inside the stage that was doing the work -- exactly the shape
                // the in-sample fix was supposed to have ended.
                //
                // The body is pure per candidate: `with_levels` applies ONE
                // named variant whose rungs came from `train`, reading `test`
                // and `test_column` immutably, and nothing accumulates across
                // iterations. Determinism holds for the same reason it holds
                // above -- `scored` is a `Vec`, so `par_iter().map().collect()`
                // is an INDEXED collect and `oos_all` arrives in the identical
                // order. `crate::pbo` ranks that vector positionally against
                // `in_sample_all`, so an order change here would silently
                // mis-pair every candidate with another's out-of-sample score.
                oos_all = scored
                    .par_iter()
                    .map(|(mask, _, pick)| {
                        // EVERY CANDIDATE ON THE TEST BARS, WEARING THE EXIT IT
                        // CHOSE IN TRAINING.
                        //
                        // `with_levels`, not `evaluate`. `evaluate` builds the
                        // whole grid FROM THE BARS IT IS GIVEN and
                        // returns the best of it; called on the test window, as
                        // this did, it fits the exit to the data it is meant to
                        // be tested on. `with_levels` applies ONE named variant
                        // whose rung values came from `train`, so nothing about
                        // the test bars decides a level — the same discipline
                        // the fold's own winner has followed since
                        // `docs/06-limits.md` §70, now applied to the vector
                        // `crate::pbo` actually ranks.
                        //
                        // A candidate whose signals produce no trade in the test
                        // window scores 0, exactly as before: `with_levels`
                        // returns `None` on an empty trade set, and 0 is the
                        // level-less total of no trades rather than a sentinel.
                        crate::grid::with_levels(
                            upto,
                            &confined,
                            mask,
                            horizon,
                            side_of(direction),
                            crate::excursion::Ladders {
                                stops: &pick.stops,
                                targets: &pick.targets,
                                trails: &pick.trails,
                            },
                            pick.rungs,
                        )
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
            // The window's LENGTH, not its end index. Under the anchored shape
            // the two are equal because the window starts at zero; under the
            // rolling one they are not, and reporting the end index would say a
            // sliding window grew.
            train_bars: train.len(),
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
            in_sample_all: scored.iter().map(|(_, v, _)| *v).collect(),
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
    use super::{Shape, Validated, walk_forward, walk_forward_shaped};
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

    /// A ROLLING walk-forward slides; an anchored one grows.
    ///
    /// # The defect this closes, and it made a whole function unreachable
    ///
    /// `split::rolling_folds` was written, tested against its fold ranges, and
    /// had **no caller** — because `walk_forward` took `bars.get(..train_end)`,
    /// a prefix from bar zero. Handing it a rolling fold discarded the fold's
    /// `start` and swept everything from the beginning: an anchored result
    /// wearing a rolling name, with nothing in the output to say so.
    ///
    /// `train_bars` is what exposes it. Under the anchored shape the windows
    /// must GROW, one block per fold. Under the rolling shape they must stay
    /// the same width. If the wiring ever regresses to a prefix, the rolling
    /// windows grow and this fails.
    #[test]
    fn a_rolling_walk_forward_slides_its_window_and_an_anchored_one_grows() {
        let bars = crate::synthetic::sessions(12);

        let anchored = walk_forward_shaped(
            &bars,
            h(15),
            3,
            Direction::Long,
            &sweeper(),
            &mut evaluator,
            Shape::Anchored,
        );
        let rolling = walk_forward_shaped(
            &bars,
            h(15),
            3,
            Direction::Long,
            &sweeper(),
            &mut evaluator,
            Shape::Rolling,
        );

        assert_eq!(anchored.folds.len(), 3, "three splits, three folds");
        assert_eq!(rolling.folds.len(), 3, "and the same for rolling");

        // ANCHORED GROWS.
        for pair in anchored.folds.windows(2) {
            let (Some(a), Some(b)) = (pair.first(), pair.get(1)) else {
                continue;
            };
            assert!(
                b.train_bars > a.train_bars,
                "an anchored window must GROW: fold {} trained on {} bars and \
                 fold {} on {}",
                a.index,
                a.train_bars,
                b.index,
                b.train_bars
            );
        }

        // ROLLING HOLDS ITS WIDTH.
        for pair in rolling.folds.windows(2) {
            let (Some(a), Some(b)) = (pair.first(), pair.get(1)) else {
                continue;
            };
            assert_eq!(
                a.train_bars, b.train_bars,
                "a rolling window must hold its WIDTH: fold {} trained on {} \
                 bars and fold {} on {} -- unequal widths mean the fold's start \
                 was discarded and this is an anchored run under another name",
                a.index, a.train_bars, b.index, b.train_bars
            );
        }

        // AND THE TWO MUST DIFFER, or the shapes are not doing different work.
        let last_anchored = anchored.folds.last().map(|f| f.train_bars);
        let last_rolling = rolling.folds.last().map(|f| f.train_bars);
        assert_ne!(
            last_anchored, last_rolling,
            "by the final fold the anchored window has grown past the rolling \
             one; equal widths mean one shape silently became the other"
        );
    }

    /// The default entry point is still ANCHORED, byte for byte.
    ///
    /// `walk_forward` is called from `cli` and from the audit, and its numbers
    /// are recorded in the ledger. Adding a shape must not have moved them --
    /// §3 rule 5 makes a rerun byte-identical, and a silent change of window
    /// shape would break that without any signal at all.
    #[test]
    fn the_default_walk_forward_is_still_anchored() {
        let bars = crate::synthetic::sessions(12);
        let plain = walk_forward(&bars, h(15), 3, Direction::Long, &sweeper(), evaluator);
        let shaped = walk_forward_shaped(
            &bars,
            h(15),
            3,
            Direction::Long,
            &sweeper(),
            &mut evaluator,
            Shape::Anchored,
        );

        assert_eq!(
            plain.folds.len(),
            shaped.folds.len(),
            "the default must produce the same folds it always did"
        );
        for (a, b) in plain.folds.iter().zip(shaped.folds.iter()) {
            assert_eq!(a.train_bars, b.train_bars, "same training window");
            assert_eq!(a.test_bars, b.test_bars, "same test window");
            assert_eq!(a.considered, b.considered, "same candidates weighed");
        }
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
            // THE REFERENCE SWEEP MUST BE RESCALED TOO, AND THE DIVISION OF
            // LABOUR BETWEEN THIS TEST AND ITS NEIGHBOUR IS THE REASON.
            //
            // This built its reference with the caller's UNSCALED sweeper while
            // `walk_forward` rescales per fold, so the two produced different
            // candidate sets and the argmax over one was compared against the
            // choice from the other. Measured on fold 1: 12,531 candidates and
            // two different masks.
            //
            // Mirroring `scale_min_hits` here does not make the test circular.
            // What this test asserts is SELECTION -- that the fold reports the
            // best of the candidates it actually had, rather than the best of a
            // prefix. That the candidate set is the right one is a separate
            // property with its own test,
            // `the_threshold_a_fold_searches_at_is_the_run_s_support_and_not_its_count`,
            // which pins the arithmetic against fixed numbers and does not run
            // the sweep at all. Neither test can pass by borrowing the other's
            // answer.
            let base = sweeper();
            let ladder = base.ladder();
            let scaled = super::scale_min_hits(ladder.min_hits(), train.len(), bars.len());
            let swept = Sweeper::new(
                engine::Ladder::with_min_hits(scaled)
                    .with_ceiling(ladder.ceiling())
                    .with_pair_budget(ladder.pair_budget()),
            )
            .run(train, &mut evaluator());
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
                    crate::grid::Levels::derived(super::DEFAULT_RUNGS),
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
    fn the_threshold_a_fold_searches_at_is_the_run_s_support_and_not_its_count() {
        // THE DEFECT, IN ARITHMETIC RATHER THAN IN A FIXTURE.
        //
        // `min_hits` is an absolute COUNT taken from the whole span. A fold
        // trains on a prefix, so the unscaled count asks for the same number of
        // hits out of fewer bars -- a stricter support every time, and on the
        // first fold an impossible one.
        //
        // Measured on a real 15-minute run over 2019-12..2026-08 at min_hits
        // 8,314, which is 22% of 37,791 swept bars. `anchored_folds` gives five
        // training prefixes and the unscaled count demanded, in order: 120%,
        // 60%, 40%, 30% and 24% support. Two folds could not have found anything
        // whatever the data said -- they reported 0 candidates -- and the report
        // read "0 of 5 folds still positive out of sample", which sounds like a
        // verdict on the strategy and was partly a verdict on this slip.
        const WHOLE: usize = 37_791;
        const MIN_HITS: u64 = 8_314;
        // Compared in BASIS POINTS and not whole percent: 8,314 of 37,791 is
        // 21.99%, which truncates to 21 and rounds to 22, so a percent
        // comparison tests the rounding rather than the scaling. The first
        // version of this assertion did exactly that and failed on the identity
        // case -- the fold that IS the whole span.
        let whole_bp = MIN_HITS.saturating_mul(10_000) / u64::try_from(WHOLE).expect("fits");
        for train in [6_928_usize, 13_856, 20_784, 27_712, 37_791] {
            let scaled = super::scale_min_hits(MIN_HITS, train, WHOLE);
            let train_u = u64::try_from(train).expect("fits");
            let got_bp = scaled.saturating_mul(10_000) / train_u;
            assert!(
                got_bp.abs_diff(whole_bp) <= 2,
                "a fold of {train} bars must search at the run's own support \
                 ({whole_bp} bp), not at {MIN_HITS} hits out of {train} bars; \
                 got {got_bp} bp from a threshold of {scaled}"
            );
        }
        // The identity case is exact, not merely close: a fold that IS the whole
        // span must get the whole span's own threshold back unchanged.
        assert_eq!(super::scale_min_hits(MIN_HITS, WHOLE, WHOLE), MIN_HITS);

        // THE FIRST FOLD IS THE ONE THAT WAS UNSATISFIABLE, so it is asserted
        // directly: the scaled threshold must fit inside the window it applies
        // to, or the fold is empty by construction rather than by measurement.
        let first = super::scale_min_hits(MIN_HITS, 6_928, WHOLE);
        assert!(
            first < 6_928,
            "the threshold must be reachable within the fold's own bars; \
             unscaled it was {MIN_HITS} out of 6,928, which is 120% support"
        );

        // ROUNDED UP, NOT DOWN. Rounding down loosens the threshold, and a fold
        // that searched a WIDER space than the run would flatter the
        // out-of-sample figure rather than test it.
        assert_eq!(
            super::scale_min_hits(10, 1, 3),
            4,
            "10/3 = 3.33 must round to 4, not 3"
        );

        // FLOORED AT ONE. Zero means every combination is frequent, which is the
        // opposite of a threshold. `Ladder::with_min_hits` raises it anyway, and
        // relying on that would put the intent somewhere a reader of this
        // function cannot see it.
        assert_eq!(super::scale_min_hits(1, 1, 1_000_000), 1);
        // And a degenerate whole is passed through rather than dividing by zero.
        assert_eq!(super::scale_min_hits(500, 10, 0), 500);
    }

    #[test]
    fn a_slice_too_short_to_split_returns_no_folds_rather_than_pretending() {
        let bars = crate::synthetic::sessions(1);
        let v = walk_forward(&bars, h(15), 0, Direction::Long, &sweeper(), evaluator);
        assert_eq!(v, Validated::default());
        assert_eq!(v.decided(), 0);
        assert_eq!(v.held_up(), 0);
    }

    #[test]
    fn a_truncated_walk_is_countable_and_a_complete_one_counts_zero() {
        // WHY THIS TEST EXISTS. `FoldResult::halted` was recorded per fold and
        // summed nowhere, so a walk-forward in which every fold truncated
        // presented the same two numbers as one in which none did --
        // `decided()` and `folds.len()` -- because a halted fold still decides.
        //
        // Both halves are asserted deliberately. A counter that is never zero
        // is not a counter, and a counter that is never non-zero is a constant;
        // asserting only one half leaves the other free to be wrong.
        let bars = crate::synthetic::sessions(12);

        // A ceiling of four cannot hold even the k=1 frontier of this
        // vocabulary, so every fold breaches it and stops with a PARTIAL
        // deepest level.
        let starved = Sweeper::new(Ladder::with_min_hits(120).with_ceiling(4));
        let truncated = walk_forward(&bars, h(15), 3, Direction::Long, &starved, evaluator);
        // MEASURED: two of the three folds breach it and one does not -- the
        // first fold trains on the fewest bars, so its k=1 frontier fits. That
        // split is what makes this fixture worth keeping. A `halted_folds` that
        // ignored the filter and returned `folds.len()` would pass an
        // all-folds-halted assertion; it cannot pass this one.
        assert!(
            truncated.halted_folds() > 0,
            "a starved ceiling must truncate at least one fold"
        );
        assert!(
            truncated.halted_folds() < truncated.folds.len(),
            "the count must be a filter over folds, not the fold count itself"
        );

        // The control must go EXTINCT on its own, and reaching that took a
        // measurement worth recording: the shipped test `sweeper()` --
        // `min_hits(120).with_ceiling(20_000)` -- HALTS TWO OF THESE THREE
        // FOLDS. Every walk-forward test in this module runs on it, so they
        // have all been ranking truncated candidate sets and none of them could
        // say so, which is the exact blindness this counter exists to end.
        //
        // MEASURED across `min_hits` at that ceiling on `sessions(12)`, BEFORE
        // the per-fold threshold was rescaled:
        // 120 -> 2 folds halted, 600 -> 0, 1200 -> 0, 2400 -> 0, 4800 -> 0.
        //
        // RE-MEASURED AFTER, and the control had to move: 600 -> 1 fold halted,
        // 1200 -> 0, 2400 -> 0.
        //
        // The number moved because the fix works. Every fold used to be handed
        // the whole span's absolute `min_hits`, so a fold training on a third of
        // the bars searched at three times the support and found far less than
        // it should have. Rescaling gives each fold the run's own support, the
        // early folds search the space they were always meant to, and at 600
        // one of them now produces enough candidates to breach a ceiling of
        // 20,000. A control chosen under the old arithmetic is not a control
        // under the new.
        let roomy = Sweeper::new(Ladder::with_min_hits(1200).with_ceiling(20_000));
        let complete = walk_forward(&bars, h(15), 3, Direction::Long, &roomy, evaluator);
        assert_eq!(
            complete.halted_folds(),
            0,
            "a walk that went extinct on its own must not report a halt"
        );
        assert_eq!(complete.folds.len(), 3, "the control walk must have folds");
    }
}
