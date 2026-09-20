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

pub mod admission;
pub mod align;
pub mod audit;
pub mod bootstrap;
pub mod bootstrap_zero_v2;
pub mod bound;
pub mod closed;
pub mod excursion;
pub mod exit_grid_policy;
pub mod expression;
pub mod family_allocation_v1;
pub mod grid;
pub mod identity;
pub mod outcome;
pub mod pbo;
pub mod portfolio;
pub mod rank;
pub mod replay_mask;
pub mod report;
pub mod resample;
pub mod research_family;
pub mod search_allocation_v1;
pub mod signal_candle_stop;
pub mod significance;
pub mod split;
pub mod synthetic;
pub mod topn;
pub mod trade;
pub mod validate;

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

/// The completion identity shared by retained and streamed outcomes.
fn complete(census: &Census, first_swept: Option<usize>, bars: u64, extinct: bool) -> bool {
    bars > 0 && first_swept.is_some() && bars == census.swept && extinct && census.reconciles()
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
        complete(
            &self.census,
            self.first_swept,
            self.sweep.bars,
            self.sweep.completed(),
        )
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
        let (column, sweep) = self.fold_and_walk(bars, evaluator);
        Self::outcome_of(&column, sweep)
    }

    /// Walk a caller-built column without rebuilding its evaluator state.
    ///
    /// This is the retained-result counterpart of
    /// [`Self::run_prepared_ranked_by_reporting`].  It exists for a column whose
    /// masks require a borrowed external dataset, such as the stored one-day
    /// causal reference stream; rebuilding it through [`Evaluator`] would
    /// silently discard that dataset.
    #[must_use]
    pub fn run_prepared(&self, column: &Column) -> Outcome {
        let live = live_positions();
        let sweep = self.ladder.walk(column.bits(), &live);
        Self::outcome_of(column, sweep)
    }

    /// One fold over the bars, then one walk of the ladder over the result.
    ///
    /// # Why the retained path stays separate
    ///
    /// [`Self::run`] promises a full [`Sweep`] and therefore retains every
    /// survivor. [`Self::run_ranked`] deliberately cannot call this helper: it
    /// feeds each frontier to a bounded edge ranker and returns exact tallies
    /// instead, so calling the retained body would restore the OOM this split
    /// exists to remove. The agreement test below compares the two walks level
    /// by level rather than making retention share an implementation.
    fn fold_and_walk(&self, bars: &[Candle], evaluator: &mut Evaluator) -> (Column, Sweep) {
        self.fold_and_walk_reporting(bars, evaluator, &|_, _, _| {})
    }

    /// [`Self::fold_and_walk`], forwarding a per-level reporter to the ladder.
    ///
    /// # Why the forwarding is a separate method rather than a field
    ///
    /// [`Sweeper`] is constructed by a `const fn` and stored by value in several
    /// callers; a reporter is a borrow with a lifetime, and putting one on the
    /// struct would put that lifetime on every type holding a `Sweeper`. The
    /// reporter is also per-CALL rather than per-sweeper -- `cli` walks eight
    /// rungs concurrently and each wants its own rung's identity captured.
    ///
    /// This crate is on CI gate 17's silence list and stays compliant: the gate
    /// greps for `telemetry::`, `log::`, `println!` and `eprintln!`, and a
    /// closure parameter is none of those. Nothing is emitted here. The reporter
    /// is called at the ladder's level boundary and whatever the CALLER does with
    /// it happens in the caller's crate, which is where gate 17 stops.
    fn fold_and_walk_reporting(
        &self,
        bars: &[Candle],
        evaluator: &mut Evaluator,
        on_level: &dyn Fn(&engine::Frontier, usize, u64),
    ) -> (Column, Sweep) {
        let column = Column::build(bars, evaluator);
        // The live position list is `Evaluator::positions` — every bit this
        // vocabulary can actually compute — rather than every live bit in the
        // table, so the ladder is never handed a position that would measure as
        // permanently false because nothing computes it.
        let live = live_positions();
        let sweep = self.ladder.walk_reporting(column.bits(), &live, on_level);
        (column, sweep)
    }

    /// The census and the sweep, taken from the very column that was walked.
    ///
    /// Split out beside [`Self::fold_and_walk`] so the pairing
    /// [`Outcome::is_complete`] depends on cannot be got wrong by a caller: the
    /// census and the sweep always come from one fold.
    fn outcome_of(column: &Column, sweep: Sweep) -> Outcome {
        Outcome {
            census: column.census(),
            first_swept: column.first_swept(),
            sweep,
        }
    }

    /// [`Self::run_prepared`], keeping the streamed tallies instead of the
    /// levels.
    ///
    /// # Why a second door and not a flag
    ///
    /// The two walks return different types, and that is the whole difference:
    /// [`Sweep`] retains every survivor of every level, [`keep::Streamed`]
    /// reduces each level to its five exits and a COUNT and drops the level.
    /// `engine`'s own doc is unambiguous that nothing else differs — *"it is
    /// the same code: both call `walk_into` and differ only in what they do
    /// with a level they have finished with"* — and
    /// `engine::tests::a_streamed_walk_and_a_retaining_walk_are_the_same_walk`
    /// requires every level, counter, exclusion and survivor to agree.
    ///
    /// # What it unblocks
    ///
    /// `Sweep` carries no count of combinations CONSIDERED; only `Streamed`
    /// does, as `streamed`. That is the ledger's `combinations` field, so a
    /// caller holding a `Sweep` cannot file an honest row — the nearest figure
    /// it has is `all_frequent().count()`, which is the KEPT total and a
    /// different quantity. `cli`'s whole-store sweep was in exactly that
    /// position: it built a run identity per instrument-month and could not
    /// record it.
    ///
    /// It is also the cheaper walk in memory, which matters more here than
    /// anywhere: the batch runs months in parallel, and retention is what binds.
    #[must_use]
    pub fn run_prepared_streamed(&self, column: &Column) -> StreamedOutcome {
        self.run_prepared_streamed_reporting(column, &mut |_, _, _| {})
    }

    /// The same streamed walk, exposing its measured level counters.
    pub fn run_prepared_streamed_reporting(
        &self,
        column: &Column,
        report: &mut dyn FnMut(&engine::Frontier, usize, u64),
    ) -> StreamedOutcome {
        let live = live_positions();
        let sweep = self.ladder.walk_streamed(column.bits(), &live, report);
        StreamedOutcome {
            census: column.census(),
            first_swept: column.first_swept(),
            sweep,
        }
    }

    /// [`Self::run`], and then scores what it found.
    ///
    /// # Why this exists beside `run` rather than replacing it
    ///
    /// `run` builds a [`Column`], walks the ladder, and **drops the column**. A
    /// caller that then wants to know which combinations were strongest has no
    /// choice but to rebuild it — a second fold over every bar with a second
    /// freshly-warmed evaluator, which is both the largest cost in the run and
    /// an invitation to the one error [`crate::outcome::Edge::mismatched`]
    /// exists to catch: a column and a [`Forward`] built from different slices
    /// produce a mean and a `t` belonging to other bars.
    ///
    /// This method builds both from `bars`, once, so the mispairing is
    /// unreachable by construction rather than merely detected after the fact.
    ///
    /// `run` stays because three of the five commands genuinely do not rank —
    /// the threshold search calls it per probe, and paying for a `Forward` and
    /// a heap on a probe that exists only to discover `min_hits` would be work
    /// nobody reads.
    ///
    /// # Cost
    ///
    /// One `Column::build` (O(bars), unchanged from `run`), one
    /// [`crate::outcome::forward`] pass (O(bars)), then one edge pass per
    /// surviving combination and O(log keep) to admit. Ranking memory is
    /// O(keep × worker chunks), while engine result retention is O(depth): every
    /// frontier becomes a fixed-size tally after its callback returns. Neither
    /// term grows with the total number of combinations produced.
    pub fn run_ranked(
        &self,
        bars: &[Candle],
        evaluator: &mut Evaluator,
        horizon: crate::outcome::Horizon,
        keep: usize,
    ) -> RankedRun {
        self.run_ranked_by(
            bars,
            evaluator,
            horizon,
            keep,
            crate::rank::Lens::Detectability,
        )
    }

    /// [`Self::run_ranked`], under a chosen [`crate::rank::Lens`].
    ///
    /// # Why the lens has to be chosen HERE and not downstream
    ///
    /// `keep` is a hard cut and this is where it falls. Everything the heap does
    /// not admit is gone before the caller sees a single combination, so a
    /// caller that wanted a different question answered cannot ask it
    /// afterwards — it can only re-order what detectability already chose.
    ///
    /// That is the defect `crates/cli`'s cap comment describes and calls
    /// unrecoverable: *"No tier ladder, no rule and no report can recover that.
    /// They all filter cells, and the cells were never computed."* Passing the
    /// lens down to the cut is what makes it recoverable.
    ///
    /// The cost is unchanged. Both lenses read the same [`crate::outcome::Edge`]
    /// from the same single pass; [`crate::outcome::Edge::payoff_bp`] is
    /// arithmetic on fields that pass already accumulated.
    pub fn run_ranked_by(
        &self,
        bars: &[Candle],
        evaluator: &mut Evaluator,
        horizon: crate::outcome::Horizon,
        keep: usize,
        lens: crate::rank::Lens,
    ) -> RankedRun {
        self.run_ranked_by_reporting(bars, evaluator, horizon, keep, lens, &|_, _, _| {})
    }

    /// [`Self::run_ranked_by`], reporting each ladder level as it completes.
    ///
    /// # The window this opens
    ///
    /// This is the call `cli` makes on the sweep path, and everything between
    /// entering it and returning was previously silent -- a measured 71 minutes
    /// on a real eight-rung run, during which the only evidence any rung existed
    /// was the span-load line it printed before starting.
    ///
    /// The reporter is handed the completed level, the cumulative distinct
    /// candidates admitted and the cumulative pairs walked: the two quantities
    /// the ladder's budgets are measured against, so a caller can see how close a
    /// walk is to refusing while there is still time to act on it.
    pub fn run_ranked_by_reporting(
        &self,
        bars: &[Candle],
        evaluator: &mut Evaluator,
        horizon: crate::outcome::Horizon,
        keep: usize,
        lens: crate::rank::Lens,
        on_level: &dyn Fn(&engine::Frontier, usize, u64),
    ) -> RankedRun {
        let column = Column::build(bars, evaluator);
        // THE SAME SLICE THE COLUMN WAS BUILT FROM, and that is the whole point
        // of computing it here rather than leaving it to the caller. It is built
        // before the walk because each completed level is scored while the
        // engine is lending it to the callback, then dropped.
        let forward = crate::outcome::forward(bars, &column, horizon);
        self.rank_prepared(column, None, &forward, keep, lens, on_level)
    }

    /// Walk an already-built signal column while scoring every retired
    /// frontier on a caller-supplied execution column and forward series.
    ///
    /// Frequency still comes from `column`; only the edge score that makes the
    /// hard top-N cut comes from `scoring_column` and `forward`. This is the
    /// prepared form needed when signals are found on a coarse rung but entries
    /// and exits fill on one-minute bars. Scoring after the stream returned
    /// would be too late: every candidate below the signal-series top-N would
    /// already be gone.
    pub fn run_prepared_ranked_by_reporting(
        &self,
        column: Column,
        scoring_column: &Column,
        forward: &crate::outcome::Forward,
        keep: usize,
        lens: crate::rank::Lens,
        on_level: &dyn Fn(&engine::Frontier, usize, u64),
    ) -> RankedRun {
        self.rank_prepared(column, Some(scoring_column), forward, keep, lens, on_level)
    }

    /// Streams every frequent itemset with its exact closure verdict before its
    /// frontier is retired.
    ///
    /// This is the uncapped population door. Unlike [`Self::run_ranked`], it
    /// never orders by evidence and never discards a row below `keep`; every
    /// itemset is offered once in ladder level then canonical-mask order. The
    /// caller may expand each closed mask into both directions and every
    /// versioned exit cell without a hidden evidence prefix.
    ///
    /// A sink refusal is remembered and no further row is offered. The engine
    /// still completes its current deterministic walk because its retirement
    /// callback is infallible; the partial sink is never returned as an answer,
    /// and this method returns the original refusal after the walk. A future
    /// fallible engine callback may make that failure faster without changing
    /// the correctness contract.
    ///
    /// # Errors
    ///
    /// Returns the first error produced by `on_member`. No [`PopulationRun`] is
    /// exposed in that case.
    pub fn run_prepared_population_by_reporting<E, F>(
        &self,
        column: Column,
        on_level: &dyn Fn(&engine::Frontier, usize, u64),
        mut on_member: F,
    ) -> Result<PopulationRun, E>
    where
        F: FnMut(PopulationMember) -> Result<(), E>,
    {
        let live = live_positions();
        let mut considered: u64 = 0;
        let mut redundant: u64 = 0;
        let mut closed: u64 = 0;
        let mut closure_complete = true;
        let mut sink_error: Option<E> = None;
        let mut progress = |level: &engine::Frontier, admitted: usize, pairs: u64| {
            on_level(level, admitted, pairs);
        };
        let mut retire = |level: &engine::Frontier, next: Option<&engine::Frontier>| {
            considered =
                considered.saturating_add(u64::try_from(level.frequent.len()).unwrap_or(u64::MAX));
            if level.frequent.is_empty() {
                return;
            }
            let (redundant_here, known) = if let Some(next) = next {
                (crate::closed::redundant_between(level, next), true)
            } else {
                closure_complete = false;
                (std::collections::HashSet::with_capacity(0), false)
            };
            redundant =
                redundant.saturating_add(u64::try_from(redundant_here.len()).unwrap_or(u64::MAX));
            for item in &level.frequent {
                let closure = if !known {
                    ClosureVerdict::Unknown
                } else if redundant_here.contains(&item.mask) {
                    ClosureVerdict::Redundant
                } else {
                    closed = closed.saturating_add(1);
                    ClosureVerdict::Closed
                };
                if sink_error.is_none()
                    && let Err(why) = on_member(PopulationMember {
                        item: *item,
                        closure,
                    })
                {
                    sink_error = Some(why);
                }
            }
        };
        let sweep = self.ladder.walk_streamed_with_retirement(
            column.bits(),
            &live,
            &mut progress,
            &mut retire,
        );
        if let Some(why) = sink_error {
            return Err(why);
        }
        debug_assert_eq!(
            considered, sweep.streamed,
            "the population sink must see every survivor the engine streams"
        );
        let trials = sweep.levels.iter().fold(0_u64, |total, level| {
            total
                .saturating_add(level.survivors)
                .saturating_add(level.infrequent)
        });
        Ok(PopulationRun {
            outcome: RankedOutcome {
                census: column.census(),
                first_swept: column.first_swept(),
                effective_trials: trials.saturating_sub(redundant),
                trials,
                closure_complete,
                sweep,
            },
            column,
            considered,
            redundant,
            closed,
        })
    }

    /// The one streamed ranked walk shared by same-series and projected runs.
    fn rank_prepared(
        &self,
        column: Column,
        scoring_column: Option<&Column>,
        forward: &crate::outcome::Forward,
        keep: usize,
        lens: crate::rank::Lens,
        on_level: &dyn Fn(&engine::Frontier, usize, u64),
    ) -> RankedRun {
        let scored_on = scoring_column.unwrap_or(&column);
        let live = live_positions();
        let mut accumulator = crate::rank::Accumulator::new(keep, lens);
        let mut progress = |level: &engine::Frontier, admitted: usize, pairs: u64| {
            on_level(level, admitted, pairs);
        };
        let mut retire = |level: &engine::Frontier, next: Option<&engine::Frontier>| {
            accumulator.offer_retired(level, next, scored_on, forward);
        };
        let sweep = self.ladder.walk_streamed_with_retirement(
            column.bits(),
            &live,
            &mut progress,
            &mut retire,
        );
        let ranked = accumulator.finish();
        debug_assert_eq!(
            ranked.considered, sweep.streamed,
            "the ranker must see every survivor the engine streams"
        );
        ranked_run(column, sweep, ranked)
    }
}

/// Rank a restored retained sweep using the same accumulator as streamed runs.
/// No indicator fold or combination search is repeated. Every preceding level
/// is scored once, including the complete history restored from a checkpoint.
///
/// # Errors
/// Refuses a signal-column count mismatch, malformed level accounting, or a
/// failed tally allocation. The caller must bind the full signal/scoring/forward
/// inputs to the checkpoint's validated identity before invoking this adapter.
pub fn rank_checkpointed_sweep(
    column: Column,
    scoring_column: Option<&Column>,
    forward: &crate::outcome::Forward,
    sweep: Sweep,
    keep: usize,
    lens: crate::rank::Lens,
) -> Result<RankedRun, String> {
    if sweep.bars != u64::try_from(column.bits().len()).unwrap_or(u64::MAX)
        || sweep.bars != column.census().swept
        || !sweep.levels.iter().all(engine::Frontier::reconciles)
        || !restored_terminal(&sweep)
    {
        return Err("restored sweep and signal column/accounting disagree".into());
    }
    let mut levels = Vec::new();
    levels
        .try_reserve_exact(sweep.levels.len())
        .map_err(|error| error.to_string())?;
    let mut accumulator = crate::rank::Accumulator::new(keep, lens);
    let mut streamed = 0_u64;
    for (index, level) in sweep.levels.iter().enumerate() {
        // A partial successor cannot certify closure for its predecessor.
        // This is exactly the shared engine retirement callback's rule.
        let next = sweep
            .levels
            .get(index + 1)
            .filter(|next| sweep.halted.is_none_or(|halt| halt.k != next.k));
        accumulator.offer_retired(level, next, scoring_column.unwrap_or(&column), forward);
        let tally = engine::keep::Tally::of(level);
        streamed = streamed.saturating_add(tally.survivors);
        levels.push(tally);
    }
    let ranked = accumulator.finish();
    let summary = engine::keep::Streamed {
        levels,
        streamed,
        excluded: sweep.excluded,
        bars: sweep.bars,
        min_hits: sweep.min_hits,
        halted: sweep.halted,
    };
    Ok(ranked_run(column, summary, ranked))
}

fn restored_terminal(sweep: &Sweep) -> bool {
    let Some(last) = sweep.levels.last() else {
        return sweep.halted.is_some_and(|halt| halt.k == 0);
    };
    let sequence = sweep.levels.iter().enumerate().all(|(offset, level)| {
        u64::from(level.k) == u64::try_from(offset).unwrap_or(u64::MAX).saturating_add(1)
            && (offset + 1 == sweep.levels.len() || !level.frequent.is_empty())
    });
    sequence
        && sweep
            .halted
            .map_or(last.frequent.is_empty(), |halt| halt.k == last.k)
}

fn ranked_run(
    column: Column,
    sweep: engine::keep::Streamed,
    ranked: crate::rank::Ranked,
) -> RankedRun {
    let trials = sweep.levels.iter().fold(0_u64, |total, level| {
        total
            .saturating_add(level.survivors)
            .saturating_add(level.infrequent)
    });
    RankedRun {
        outcome: RankedOutcome {
            census: column.census(),
            first_swept: column.first_swept(),
            effective_trials: trials.saturating_sub(ranked.redundant),
            trials,
            closure_complete: ranked.closure_complete,
            sweep,
        },
        ranked,
        column,
    }
}

/// Whether one frequent itemset is an exact closed representative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClosureVerdict {
    /// No immediate superset has equal support; this mask belongs to the
    /// lossless closed population.
    Closed,
    /// An immediate superset has the same support; this mask is recoverable
    /// from that closed superset and is not a separate strategy population row.
    Redundant,
    /// The ladder halted before a successor frontier could decide closure.
    Unknown,
}

/// One canonical frequent itemset at the instant its frontier retires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationMember {
    /// Exact mask and support measured by the engine.
    pub item: engine::Itemset,
    /// Closure verdict decided from the immediate successor frontier.
    pub closure: ClosureVerdict,
}

/// One uncapped streamed population walk and its exact reconciliation.
#[derive(Clone, Debug)]
pub struct PopulationRun {
    /// Census, extinction/halt state, level tallies and closure completeness.
    pub outcome: RankedOutcome,
    /// The same signal column that was walked, retained for exact downstream
    /// pricing and evidence derivation.
    pub column: Column,
    /// Every frequent itemset offered to the sink.
    pub considered: u64,
    /// Itemsets recoverable from an equal-support closed superset.
    pub redundant: u64,
    /// Closed itemsets available for direction/exit-cell expansion.
    pub closed: u64,
}

impl PopulationRun {
    /// Whether enumeration, census and every closure decision are complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.outcome.is_complete() && self.considered == self.redundant.saturating_add(self.closed)
    }
}

/// What an UNRANKED streamed sweep produced: the census, with level tallies.
///
/// [`Outcome`]'s sibling, and distinct from [`RankedOutcome`] below: this one
/// carries no ranking because its caller does none. The two walks return
/// different sweep types and a caller wants the census and `first_swept` either
/// way; see [`Sweeper::run_prepared_streamed`] for which to reach for and why.
#[derive(Clone, Debug)]
pub struct StreamedOutcome {
    /// Where every offered bar went -- swept, still warming, or refused by name.
    pub census: Census,
    /// The caller-slice index of the first swept bar, or `None` if none warmed.
    pub first_swept: Option<usize>,
    /// The walk, with each level reduced to its tally.
    pub sweep: engine::keep::Streamed,
}

impl StreamedOutcome {
    /// Whether a warmed, reconciled input produced a complete streamed walk.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        complete(
            &self.census,
            self.first_swept,
            self.sweep.bars,
            self.sweep.completed(),
        )
    }
}

/// A ranked sweep's census and exact level tallies, without retained survivors.
///
/// This is the streamed counterpart of [`Outcome`]. A ranked caller never needs
/// every frequent itemset after the bounded edge ranker has made its cut, so
/// representing this result as [`Sweep`] would force the engine to retain the
/// very vectors the ranked path exists to discard. [`engine::keep::Streamed`]
/// preserves the extinction depth, halt, exclusions and every level counter;
/// [`RankedRun::ranked`] preserves the best rows and how many were considered.
#[derive(Clone, Debug)]
pub struct RankedOutcome {
    /// Where every offered bar went — swept, still warming, or refused by name.
    pub census: Census,
    /// The caller-slice index of the first swept bar, or `None` if none warmed.
    pub first_swept: Option<usize>,
    /// Every candidate whose support was measured: infrequent plus survivors.
    pub trials: u64,
    /// [`Self::trials`] after exact equal-support duplicates are removed.
    pub effective_trials: u64,
    /// Whether closure was decided for every non-empty level.
    pub closure_complete: bool,
    /// The ladder result with each retired survivor vector reduced to its exact
    /// count and all other counters retained.
    pub sweep: engine::keep::Streamed,
}

impl RankedOutcome {
    /// Did this streamed run produce a complete, trustworthy answer?
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.closure_complete
            && complete(
                &self.census,
                self.first_swept,
                self.sweep.bars,
                self.sweep.completed(),
            )
    }
}

/// Everything one ranked sweep produced, including the column it was measured on.
///
/// # Why the column is returned rather than dropped
///
/// [`Sweeper::run`] builds a [`Column`] and throws it away, so a caller that
/// then wants to TRADE what the sweep found has to build a second one — a second
/// fold over every bar with a second freshly-warmed evaluator. `crates/cli`'s
/// audit did exactly that, and the two folds were not merely wasteful: the
/// column came from the caller's evaluator while the sweep came from a private
/// one, so a caller supplying custom widths would have had masks discovered
/// under one vocabulary indexing a column built under another. That is the
/// mispairing [`crate::outcome::Edge::mismatched`] exists to catch, reachable by
/// construction rather than by accident.
///
/// Returning the column makes one fold serve the sweep, the ranking and the
/// trade walk, so the three cannot disagree about what they measured.
#[derive(Debug)]
pub struct RankedRun {
    /// The census, warm-up boundary, and streamed ladder tallies.
    pub outcome: RankedOutcome,
    /// The strongest combinations by |t|, best first, bounded by `keep`.
    pub ranked: crate::rank::Ranked,
    /// The bar-bit column every figure above was measured on.
    pub column: Column,
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

/// Structural probe boundaries from the shared threshold search.
#[derive(Debug)]
pub enum AutoProbeEvent<'a> {
    /// Must succeed before this ladder computes any candidate.
    Started(Ladder),
    /// Actual counters reported by this probe's engine walk.
    Level {
        /// The completed frontier accounting.
        frontier: &'a engine::Frontier,
        /// Cumulative admitted candidates.
        admitted: usize,
        /// Cumulative join pairs.
        pairs: u64,
    },
    /// The probe's exact outcome, before another probe can start.
    Finished(&'a Sweep),
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
        self.auto_prepared(&column)
    }

    /// Search the threshold over one already-built condition column.
    ///
    /// The column is consumed because the returned [`Auto`] retains only its
    /// census and sweep tallies.  External-reference evaluators can therefore
    /// build once, validate their own causal receipts, and use the same search
    /// as an ordinary [`Evaluator`] without reconstructing masks under a
    /// different input policy.
    #[must_use]
    pub fn auto_prepared(&self, column: &Column) -> Auto {
        self.auto_prepared_reporting(column, &|_| Ok(()))
            .unwrap_or_else(|_| self.auto_memory_refusal(column))
    }

    fn auto_memory_refusal(&self, column: &Column) -> Auto {
        Auto {
            affordable: false,
            outcome: Self::outcome_of(column, self.ladder.refused_on_memory(column.census().swept)),
            min_hits: None,
            attempts: 0,
            refused_below: None,
        }
    }

    /// Search with durable admission hooks around every actual probe.
    ///
    /// # Errors
    /// Returns the reporter's refusal before another probe can run. A failed
    /// start prevents that probe; a failed level or finish prevents publication.
    pub fn auto_prepared_reporting(
        &self,
        column: &Column,
        report: &dyn Fn(AutoProbeEvent<'_>) -> Result<(), String>,
    ) -> Result<Auto, String> {
        let live = live_positions();
        // THE OWNED SUPPORT COLUMN IS BUILT ONCE TOO, AND IT WAS NOT.
        //
        // The doc above says "the column is folded once and every probe walks
        // the same one". That was true of `Column::build` and FALSE of the
        // owned column the ladder actually counts against: `Ladder::walk`
        // copies its input itself, so each of the ~17 halvings and ~17
        // bisections below would rebuild identical rows.
        //
        // One row-major copy is 58.7 MB at 1,222,791 bars. So an auto-tuned run
        // rebuilt that identical allocation up to thirty-four times, having
        // already paid for it once. Found by an O(1) audit; no per-walk cost was
        // wrong, which is why review missed it.
        let support_column = engine::column::Column::try_from_rows(column.bits())
            .map_err(|why| format!("the auto support column could not be allocated: {why}"))?;
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
            let sweep = self.auto_probe(threshold, &support_column, &live, report)?;
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
                let sweep = self.auto_probe(mid, &support_column, &live, report)?;
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
        Ok(Auto {
            affordable: min_hits.is_some(),
            outcome: Outcome {
                census,
                first_swept,
                sweep,
            },
            min_hits,
            attempts,
            refused_below,
        })
    }

    fn auto_probe(
        &self,
        threshold: u64,
        column: &engine::column::Column,
        live: &[u32],
        report: &dyn Fn(AutoProbeEvent<'_>) -> Result<(), String>,
    ) -> Result<Sweep, String> {
        let ladder = Ladder::with_min_hits(threshold)
            .with_ceiling(self.ladder.ceiling())
            .with_pair_budget(PROBE_PAIRS)
            .with_support_lanes(self.ladder.support_lanes());
        report(AutoProbeEvent::Started(ladder))?;
        let failure = std::cell::RefCell::new(None);
        let sweep = ladder.walk_column(column, live, &|frontier, admitted, pairs| {
            if failure.borrow().is_none() {
                *failure.borrow_mut() = report(AutoProbeEvent::Level {
                    frontier,
                    admitted,
                    pairs,
                })
                .err();
            }
        });
        if let Some(why) = failure.into_inner() {
            return Err(why);
        }
        report(AutoProbeEvent::Finished(&sweep))?;
        Ok(sweep)
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
    use super::{
        ClosureVerdict, Outcome, PopulationMember, Sweeper, candle, column_of, live_positions,
    };
    use crate::synthetic;
    use engine::Ladder;
    use indicators::column::Column;
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
        Ladder::with_min_hits(600)
            .with_ceiling(50_000)
            .with_support_lanes(1)
    }

    #[test]
    fn the_population_door_streams_every_closed_mask_without_an_evidence_cap() {
        let bars = synthetic::sessions(8);
        let column = Column::build(&bars, &mut evaluator());
        let mut members: Vec<PopulationMember> = Vec::new();
        let population = Sweeper::new(bounded())
            .run_prepared_population_by_reporting(
                column,
                &|_, _, _| {},
                |member| -> Result<(), &'static str> {
                    members.push(member);
                    Ok(())
                },
            )
            .expect("the in-memory sink cannot refuse");

        let retaining = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let expected = crate::closed::closed(&retaining.sweep);
        let actual_closed: Vec<_> = members
            .iter()
            .filter(|member| member.closure == ClosureVerdict::Closed)
            .map(|member| member.item)
            .collect();

        assert!(population.is_complete());
        assert_eq!(
            population.considered,
            u64::try_from(members.len()).unwrap_or(u64::MAX)
        );
        assert_eq!(population.considered, expected.considered);
        assert_eq!(population.redundant, expected.redundant());
        assert_eq!(
            population.closed,
            u64::try_from(actual_closed.len()).unwrap_or(u64::MAX)
        );
        assert_eq!(
            actual_closed, expected.kept,
            "streaming closure must equal the uncapped retaining reference"
        );
        assert!(
            members
                .iter()
                .all(|member| member.closure != ClosureVerdict::Unknown),
            "an extinct ladder decides every closure verdict"
        );
    }

    #[test]
    fn one_population_sink_refusal_exposes_no_partial_run() {
        let bars = synthetic::sessions(8);
        let column = Column::build(&bars, &mut evaluator());
        let mut offered: u64 = 0;
        let result = Sweeper::new(bounded()).run_prepared_population_by_reporting(
            column,
            &|_, _, _| {},
            |_| -> Result<(), &'static str> {
                offered = offered.saturating_add(1);
                Err("population writer refused")
            },
        );
        assert_eq!(
            result.expect_err("a refusing population writer must withhold the partial run"),
            "population writer refused"
        );
        assert_eq!(
            offered, 1,
            "after the first refusal no later row may reach a partial sink"
        );
    }

    /// `run_ranked` AGREES WITH `run`, AND PAIRS ITS FORWARD WITH ITS OWN COLUMN.
    ///
    /// # Why this test had to be written before anything else here was trusted
    ///
    /// `run_ranked` shipped with **zero** coverage: its one production call site
    /// is inside `cli::sweep_stored_inner`, which refuses at the commit gate on
    /// any build without `BRUTEX_COMMIT` -- and `cargo test` WAS such a build.
    /// `crates/cli/build.rs` has stamped one from `.git/HEAD` since `086149d5`
    /// and stamps the test harness too, so that hole would close itself today.
    /// It is recorded because the argument for the split does not rest on it. So
    /// the whole body, `Column::build` through `rank::rank`, never executed in
    /// the suite. `CLAUDE.md` §9 asks for 100% on a touched crate and this was a
    /// hole straight through the middle of the change.
    ///
    /// # The two properties, and why the second is the one that matters
    ///
    /// The first is agreement: the streamed sweep half of `run_ranked` must
    /// produce every counter and the same extinction depth as `run`, while its
    /// bounded ranking must equal ranking the retained result afterwards.
    ///
    /// The second is the claim the method's own doc makes and nothing proved:
    /// that building the column and the `Forward` from **one** slice makes
    /// `Edge::mismatched` unreachable *by construction* rather than merely
    /// detected afterwards. A non-zero count means the mean and the `t` of that
    /// row were computed from returns belonging to other bars — the row would
    /// render as `MISPAIRED`, and a report that can print that is a report whose
    /// numbers cannot be believed.
    #[test]
    fn a_ranked_run_agrees_with_a_plain_one_and_never_mispairs() {
        let bars = synthetic::sessions(8);

        let plain = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let run = Sweeper::new(bounded()).run_ranked(
            &bars,
            &mut evaluator(),
            crate::outcome::Horizon::DEFAULT,
            10,
        );

        assert_eq!(
            run.outcome.census, plain.census,
            "the same bars folded the same way must yield the same census"
        );
        assert_eq!(
            run.outcome.first_swept, plain.first_swept,
            "and the same warm-up boundary"
        );
        assert_eq!(
            run.outcome.sweep.depth(),
            plain.sweep.depth(),
            "and the same ladder depth — a ranked run is a plain run that also scores"
        );
        assert_eq!(run.outcome.sweep.bars, plain.sweep.bars);
        assert_eq!(run.outcome.sweep.min_hits, plain.sweep.min_hits);
        assert_eq!(run.outcome.sweep.halted, plain.sweep.halted);
        assert_eq!(run.outcome.sweep.excluded, plain.sweep.excluded);
        assert_eq!(
            run.outcome.sweep.levels.len(),
            plain.sweep.levels.len(),
            "the empty extinction level and a partial halted level are retained as tallies"
        );
        for (streamed, retained) in run.outcome.sweep.levels.iter().zip(&plain.sweep.levels) {
            assert_eq!(
                *streamed,
                engine::keep::Tally::of(retained),
                "every level counter must survive dropping its itemsets"
            );
        }
        assert_eq!(
            run.ranked.considered, run.outcome.sweep.streamed,
            "every streamed survivor must reach the edge ranker exactly once"
        );
        assert!(
            run.outcome.is_complete(),
            "dropping retired survivor vectors must not make a complete walk read partial"
        );
        assert_eq!(
            run.outcome.trials,
            crate::significance::trials(&plain.sweep),
            "streaming must retain every support evaluation in the raw trial count"
        );
        assert_eq!(
            run.outcome.effective_trials,
            crate::significance::effective_trials(&plain.sweep),
            "adjacent retirement must deflate exactly the duplicates a retained sweep finds"
        );

        let legacy = crate::rank::rank_by(
            &plain.sweep,
            &run.column,
            &crate::outcome::forward(&bars, &run.column, crate::outcome::Horizon::DEFAULT),
            10,
            crate::rank::Lens::Detectability,
        );
        assert_eq!(run.ranked.considered, legacy.considered);
        assert_eq!(
            run.ranked.top, legacy.top,
            "streaming changes retention, not which edge-ranked rows survive the cut"
        );

        // THE FIXTURE HAS TO PRODUCE SOMETHING, or every assertion below is
        // vacuous and this test would pass on a `rank` that returned nothing.
        assert!(
            !run.ranked.top.is_empty(),
            "the fixture must produce ranked rows, or the mispairing assertion \
             below proves nothing"
        );
        assert!(
            run.ranked.top.len() <= 10,
            "the heap is bounded by `keep`, which is the whole reason it is a \
             heap and not a sort"
        );

        // THE PROPERTY THE DOC CLAIMS. One slice in, so no row can be scored
        // against returns belonging to other bars.
        for scored in &run.ranked.top {
            assert_eq!(
                scored.edge.mismatched, 0,
                "the column and the forward came from one slice, so no row may \
                 be mispaired; a non-zero count makes that row's mean and t \
                 meaningless"
            );
        }

        // AND THE COLUMN COMES BACK, which is what lets a caller trade what was
        // found without folding the bars a second time under a second evaluator.
        assert_eq!(
            run.column.bits().len(),
            usize::try_from(run.outcome.census.swept).unwrap_or(usize::MAX),
            "the returned column is the one the census counted, not another"
        );
    }

    /// The projected execution series must decide the hard cut while every
    /// signal frontier is still live, not after signal-ranked rows were dropped.
    #[test]
    fn prepared_projected_ranking_matches_a_retained_execution_ranking() {
        let bars = synthetic::sessions(8);
        let signal = Column::build(&bars, &mut evaluator());
        let onto: Vec<Option<usize>> = (0..signal.bits().len())
            .map(|index| Some(index.saturating_mul(2)))
            .collect();
        let (execution, dropped) = signal
            .reproject_checked(&onto, &bars)
            .expect("the alignment is parallel to the signal column");
        assert_eq!(dropped, 0, "every fixture signal has an execution bar");
        let forward = crate::outcome::forward(&bars, &execution, crate::outcome::Horizon::DEFAULT);
        let retained = Sweeper::new(bounded()).run(&bars, &mut evaluator());

        for lens in [crate::rank::Lens::Detectability, crate::rank::Lens::Payoff] {
            let expected = crate::rank::rank_by(&retained.sweep, &execution, &forward, 10, lens);
            let got = Sweeper::new(bounded()).run_prepared_ranked_by_reporting(
                signal.clone(),
                &execution,
                &forward,
                10,
                lens,
                &|_, _, _| {},
            );

            assert_eq!(got.ranked.top, expected.top, "projected {lens:?} cut");
            assert_eq!(
                got.ranked.closed_top, expected.closed_top,
                "projected {lens:?} closure filter"
            );
            assert_eq!(got.ranked.considered, expected.considered);
            assert!(got.outcome.is_complete());
        }
    }

    #[test]
    fn prepared_retained_and_auto_paths_equal_the_ordinary_folded_paths() {
        let bars = synthetic::sessions(8);
        let column = Column::build(&bars, &mut evaluator());
        let retained = Sweeper::new(bounded()).run(&bars, &mut evaluator());
        let prepared = Sweeper::new(bounded()).run_prepared(&column);
        assert_eq!(prepared.census, retained.census);
        assert_eq!(prepared.first_swept, retained.first_swept);
        assert_eq!(prepared.sweep, retained.sweep);

        let search = Ladder::with_min_hits(1).with_ceiling(50_000);
        let ordinary_auto = Sweeper::new(search).auto(&bars, &mut evaluator());
        let prepared_auto = Sweeper::new(search).auto_prepared(&column);
        assert_eq!(prepared_auto.affordable, ordinary_auto.affordable);
        assert_eq!(prepared_auto.min_hits, ordinary_auto.min_hits);
        assert_eq!(prepared_auto.attempts, ordinary_auto.attempts);
        assert_eq!(prepared_auto.refused_below, ordinary_auto.refused_below);
        assert_eq!(prepared_auto.outcome.census, ordinary_auto.outcome.census);
        assert_eq!(
            prepared_auto.outcome.first_swept,
            ordinary_auto.outcome.first_swept
        );
        assert_eq!(prepared_auto.outcome.sweep, ordinary_auto.outcome.sweep);
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

    #[test]
    fn a_halted_ranked_sweep_cannot_certify_closure() {
        let bars = synthetic::sessions(8);
        let tight = Ladder::with_min_hits(1).with_ceiling(1);
        let run = Sweeper::new(tight).run_ranked(
            &bars,
            &mut evaluator(),
            crate::outcome::Horizon::DEFAULT,
            10,
        );

        assert!(
            run.outcome.sweep.halted.is_some(),
            "the fixture must breach"
        );
        assert!(
            !run.outcome.closure_complete,
            "a partial successor cannot prove the level below it closed"
        );
        assert!(!run.outcome.is_complete());
        assert_eq!(run.ranked.considered, run.outcome.sweep.streamed);
    }

    #[test]
    fn checkpoint_ranking_preserves_streamed_scores_trials_and_partial_closure() {
        let bars = synthetic::sessions(8);
        let signal = Column::build(&bars, &mut evaluator());
        let forward = crate::outcome::forward(&bars, &signal, crate::outcome::Horizon::DEFAULT);
        let bit_column = engine::column::Column::try_from_rows(signal.bits()).expect("column");
        for ladder in [bounded(), Ladder::with_min_hits(1).with_ceiling(1)] {
            for lens in [
                crate::rank::Lens::Detectability,
                crate::rank::Lens::Payoff,
                crate::rank::Lens::Path,
            ] {
                for keep in [0, 10] {
                    let expected = Sweeper::new(ladder).run_prepared_ranked_by_reporting(
                        signal.clone(),
                        &signal,
                        &forward,
                        keep,
                        lens,
                        &|_, _, _| {},
                    );
                    let mut saved = Vec::new();
                    let completed = ladder
                        .walk_checkpointed(&bit_column, &live_positions(), [7; 32], &mut |view| {
                            if view.terminal() {
                                view.write_to(&mut saved)
                                    .map_err(|error| error.to_string())?;
                            }
                            Ok(())
                        })
                        .expect("retained checkpoint walk");
                    let checkpoint = engine::resume::Checkpoint::read_from(
                        &mut saved.as_slice(),
                        saved.len() as u64,
                        [7; 32],
                    )
                    .expect("decode");
                    let resumed = ladder
                        .resume_checkpointed(
                            &bit_column,
                            &live_positions(),
                            [7; 32],
                            checkpoint,
                            &mut |_| Ok(()),
                        )
                        .expect("resume");
                    assert_eq!(resumed, completed);
                    let got = super::rank_checkpointed_sweep(
                        signal.clone(),
                        Some(&signal),
                        &forward,
                        resumed,
                        keep,
                        lens,
                    )
                    .expect("rank restored");
                    assert_eq!(got.ranked.top, expected.ranked.top);
                    assert_eq!(got.ranked.closed_top, expected.ranked.closed_top);
                    assert_eq!(got.ranked.considered, expected.ranked.considered);
                    assert_eq!(got.ranked.redundant, expected.ranked.redundant);
                    assert_eq!(got.outcome.trials, expected.outcome.trials);
                    assert_eq!(
                        got.outcome.effective_trials,
                        expected.outcome.effective_trials
                    );
                    assert_eq!(
                        got.outcome.closure_complete,
                        expected.outcome.closure_complete
                    );
                    assert_eq!(got.outcome.is_complete(), expected.outcome.is_complete());
                    assert_eq!(got.outcome.sweep.levels, expected.outcome.sweep.levels);
                    assert_eq!(got.outcome.sweep.excluded, expected.outcome.sweep.excluded);
                    assert_eq!(got.outcome.sweep.halted, expected.outcome.sweep.halted);
                }
            }
        }
    }

    #[test]
    fn checkpoint_rank_adapter_refuses_a_missing_terminal_or_mismatched_column() {
        let bars = synthetic::sessions(8);
        let signal = Column::build(&bars, &mut evaluator());
        let forward = crate::outcome::forward(&bars, &signal, crate::outcome::Horizon::DEFAULT);
        let original = Sweeper::new(bounded()).run_prepared(&signal).sweep;
        assert!(
            original
                .levels
                .last()
                .expect("terminal")
                .frequent
                .is_empty()
        );
        for mutation in 0..4 {
            let mut invalid = original.clone();
            match mutation {
                0 => {
                    invalid.levels.pop();
                }
                1 => invalid.bars += 1,
                2 => invalid.levels.clear(),
                _ => invalid.levels.first_mut().expect("first").generated += 1,
            }
            assert!(
                super::rank_checkpointed_sweep(
                    signal.clone(),
                    None,
                    &forward,
                    invalid,
                    10,
                    crate::rank::Lens::Detectability
                )
                .is_err()
            );
        }
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

    /// This crate cannot open the store, and that is checked rather than promised.
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
    /// The architectural boundary is absolute: `runner` consumes slices a caller
    /// already holds and must not acquire a filesystem or store dependency of its
    /// own. `cli sweep-stored` may load real bars and pass those slices in; this
    /// crate may not pull, ingest or open them itself. So "I inspected it" is not
    /// a good enough guarantee for the one unguarded link. This runs under
    /// `cargo test`, needs no workflow change, and fails the moment a filesystem
    /// call appears here.
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
    ///
    /// **It has now happened a fifth time, in this very paragraph.** Widening
    /// the scan to every module put `lib.rs`'s own doc under it, and the
    /// sentence below naming the modules that were unguarded spelled one of the
    /// needles while doing so. The rule is not a footnote about the array
    /// literal; it covers the prose too, because the prose is in the file.
    ///
    /// # IT SCANNED TWO MODULES OF TWENTY-FOUR
    ///
    /// The list was `lib.rs` and `synthetic.rs`. `crates/runner/src/` holds
    /// twenty-four modules, so an open-a-file call in `admission.rs`,
    /// `validate.rs`, `exit_grid_policy.rs`, `grid.rs`, `outcome.rs`, `trade.rs`,
    /// `rank.rs`, `replay_mask.rs`, `audit.rs`, `report.rs`, `bootstrap.rs`,
    /// `excursion.rs`, `align.rs`, `closed.rs`, `pbo.rs`, `portfolio.rs`,
    /// `resample.rs`, `significance.rs`, `split.rs`, `topn.rs`, `bound.rs` or
    /// `identity.rs` passed unseen — **twenty-two of the twenty-four, including
    /// every module that actually holds the sweep.** A guard whose own doc says
    /// it "fails the moment a filesystem call appears here" has to look at
    /// "here".
    ///
    /// # THE ENVIRONMENT NEEDLE IS SEPARATED, AND NOT DELETED
    ///
    /// Extending the list makes one needle match: `validate.rs:441` reads
    /// `BRUTEX_GRID_RUNGS` to size the walk-forward exit ladder. Deleting the
    /// needle to get green would trade a real guard for a green tick, so the
    /// needle stays and moves to its own tier, because it is guarding a
    /// different thing from the other fifteen.
    ///
    /// The fifteen name a FILE or a PROCESS: any of them is the operator's rule
    /// broken outright, in any module, with no exception. An environment read is
    /// not one of those — it reads a `usize` and cannot open anything — and it
    /// is on this list only because a PATH is how a file would arrive. So the
    /// second tier asserts what that threat actually needs:
    ///
    /// * no module may call `env::var`, in any module — an environment read
    ///   whose result is a `String` is the one that could be a path;
    /// * the crate may make exactly ONE environment read of any kind, and its
    ///   variable must be the audited one.
    ///
    /// A second env read, or a first one under any other name, fails here. And
    /// the fifteen still stand behind it: even a path that arrived could not be
    /// opened.
    #[test]
    fn this_crate_cannot_open_a_file_and_cannot_name_the_store() {
        let banned: [&str; 15] = [
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
        ];
        // EVERY MODULE UNDER `crates/runner/src/`. A module missing from this
        // list is a module the guard does not cover, which is what it was.
        let sources: [(&str, &str); 25] = [
            ("admission.rs", include_str!("admission.rs")),
            ("align.rs", include_str!("align.rs")),
            ("audit.rs", include_str!("audit.rs")),
            ("bootstrap.rs", include_str!("bootstrap.rs")),
            ("bound.rs", include_str!("bound.rs")),
            ("closed.rs", include_str!("closed.rs")),
            ("excursion.rs", include_str!("excursion.rs")),
            ("exit_grid_policy.rs", include_str!("exit_grid_policy.rs")),
            ("expression.rs", include_str!("expression.rs")),
            ("grid.rs", include_str!("grid.rs")),
            ("identity.rs", include_str!("identity.rs")),
            ("lib.rs", include_str!("lib.rs")),
            ("outcome.rs", include_str!("outcome.rs")),
            ("pbo.rs", include_str!("pbo.rs")),
            ("portfolio.rs", include_str!("portfolio.rs")),
            ("rank.rs", include_str!("rank.rs")),
            ("replay_mask.rs", include_str!("replay_mask.rs")),
            ("report.rs", include_str!("report.rs")),
            ("resample.rs", include_str!("resample.rs")),
            ("significance.rs", include_str!("significance.rs")),
            ("split.rs", include_str!("split.rs")),
            ("synthetic.rs", include_str!("synthetic.rs")),
            ("topn.rs", include_str!("topn.rs")),
            ("trade.rs", include_str!("trade.rs")),
            ("validate.rs", include_str!("validate.rs")),
        ];
        for (name, src) in sources {
            for needle in banned {
                assert!(
                    !src.contains(needle),
                    "crates/runner/src/{name} contains `{needle}`. This crate is the \
                     one link in the sweep chain gate 22 does not guard. It may \
                     consume caller-supplied generated or stored candle slices, \
                     but it may not open a file, pull, ingest or acquire a store \
                     dependency itself."
                );
            }
        }

        // THE SECOND TIER. A `String`-valued environment read is the one that
        // could carry a path, and no module makes one.
        for (name, src) in sources {
            assert!(
                !src.contains(concat!("en", "v::var(")),
                "crates/runner/src/{name} reads a String-valued environment \
                 variable. That is how a path reaches a program, and this crate \
                 may not name a file at all."
            );
        }

        // AND EXACTLY ONE ENVIRONMENT READ IN THE CRATE, of the audited
        // variable. `validate.rs` sizes its exit ladder from it -- a `usize`,
        // which opens nothing -- and a second reader, or a different name,
        // fails here rather than being noticed later.
        let reads: Vec<(&str, &str)> = sources
            .iter()
            .flat_map(|&(name, src)| {
                src.match_indices(concat!("va", "r_os("))
                    .map(move |(at, _)| {
                        (name, src.get(at..at.saturating_add(28)).unwrap_or_default())
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(
            reads.len(),
            1,
            "this crate makes {} environment reads and may make exactly one: \
             {reads:?}",
            reads.len()
        );
        assert!(
            reads.first().is_some_and(
                |&(name, text)| name == "validate.rs" && text.contains("BRUTEX_GRID_RUNGS")
            ),
            "the one environment read must be `validate.rs` sizing its exit \
             ladder, and it is {reads:?}"
        );

        // And the dependency set, exactly — the same argument gate 22 clause A
        // makes: a crate that cannot NAME a bar reader cannot call one.
        let manifest = include_str!("../Cargo.toml");
        for forbidden in ["store", "pull", "reqwest", "telemetry"] {
            assert!(
                !manifest.contains(&format!("\n{forbidden} ")),
                "crates/runner must not depend on `{forbidden}` -- that is how a \
                 caller-owned input would become a runner-owned read"
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
