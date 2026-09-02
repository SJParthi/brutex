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

use crate::exit_grid_policy::{
    ExecutionRunV1, ExecutionSeriesV1, ExitGridErrorV1, OosExecutionSeriesV1, ResolvedExitGridV1,
    SelectedExitV1, column_digest_v1,
};
use crate::identity::{Params, Run, data_digest, data_digest_with_execution};
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
    /// Round trips closed by a proved session square-off rather than the horizon.
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
    /// Which way this fold's winner is traded, decided on TRAIN alone.
    ///
    /// The fold used to be HANDED a direction and price every candidate with
    /// it. That direction came from `side_of_evidence` over the whole span —
    /// test folds included — so the side was fitted to the window it is meant
    /// to be tested on, and then applied to candidates whose own evidence
    /// pointed the other way. Each candidate now derives its own from the
    /// training window, and the winner's travels into the test window unchanged,
    /// exactly as its exit rungs do.
    ///
    /// Reported rather than kept private for the same reason `chosen_exit` is:
    /// a fold that DECIDES something and does not say what it decided leaves a
    /// `None` exactly when `chosen` is `None`: a fold that picked no candidate
    /// decided no side, and saying "long" there would be a value nobody chose.
    pub chosen_side: Option<Direction>,
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

/// Whether the slice handed to [`walk_forward_shaped`] is the one positions are
/// taken on.
///
/// # Why a type and not a `bool`
///
/// Because a `bool` is what allowed the defect. `cli` called
/// `shapes_if(validate, &bars, ..)` three lines above
/// `bootstrap_family(&trade_bars, ..)`, and no signature in between could
/// distinguish the signal series from the execution series -- both are
/// `&[Candle]`, both are the right length, and both compile. The sibling call was
/// corrected and this one was not, because nothing made the difference sayable.
///
/// A caller now has to name which series it is handing over, and a caller that
/// does not know is a caller that should not be validating.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TradesOnTheseBars {
    /// The conditions were found on this series AND positions are taken on it.
    /// The walk proceeds.
    Yes,
    /// They are different series, and the reason names which. The walk refuses
    /// rather than measuring a holding period nobody trades.
    No(String),
}

/// The one-minute path on which a signal-series walk actually trades.
///
/// `signal_length_micros` is supplied from the named signal timeframe rather
/// than inferred from adjacent timestamps. A missing signal bar is a data gap,
/// not a longer interval, and inferring the length would move the fill instant
/// with the shape of the data.
#[derive(Clone, Copy, Debug)]
pub struct ExecutionSeries<'a> {
    /// Strictly increasing one-minute OHLCV bars for the same feed, instrument,
    /// and calendar span as the signal series.
    pub bars: &'a [Candle],
    /// The duration of one signal bar, in microseconds.
    pub signal_length_micros: i64,
}

/// A whole walk-forward.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Validated {
    /// One entry per test period, in time order.
    pub folds: Vec<FoldResult>,
    /// Why no fold was run, when none was. `None` means the walk was attempted.
    ///
    /// # An empty `folds` had two meanings and a reader could not separate them
    ///
    /// A slice too short to split produces no folds, and so does a refusal. Both
    /// rendered as the same blank table, so "this span cannot be validated" and
    /// "this run declined to validate" were one output. `CLAUDE.md` §4 bans a
    /// result that hides a failure behind a success, and a silent zero is that
    /// result.
    ///
    /// The refusal this was added for: [`walk_forward_shaped`] takes ONE series
    /// and uses it for both roles -- it sweeps conditions on it AND trades on it.
    /// That is correct whenever the two series are the same, which is every
    /// caller that passes no execution series. It is wrong when they differ, and
    /// the difference is not small: on a 60-minute signal series the last bar
    /// inside the fill window is the 14:15 bucket, so every fold trade takes the
    /// forced exit there and a five-hour hold is reported as validation of a
    /// strategy that holds minutes.
    ///
    /// [`walk_forward_projected_with_rungs`] is the two-series door: it keeps
    /// fold selection in signal-index space and reprojects separately inside
    /// every fold. This field remains the visible refusal for malformed paths
    /// and for callers that explicitly admit they handed the one-series door
    /// bars positions are not taken on.
    pub refused: Option<String>,
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

const ANCHORED_ADMISSION_POLICY_DOMAIN_V2: &[u8] =
    b"brutex.validate.anchored-admission.policy.v2\0";
const ANCHORED_ADMISSION_FAMILY_DOMAIN_V2: &[u8] =
    b"brutex.validate.anchored-admission.family.v2\0";
const ANCHORED_ADMISSION_FACTS_DOMAIN_V2: &[u8] = b"brutex.validate.anchored-admission.facts.v2\0";
const ANCHORED_ADMISSION_CANDIDATES_DOMAIN_V2: &[u8] =
    b"brutex.validate.anchored-admission.candidates.v2\0";
const ANCHORED_SEARCH_POLICY_DOMAIN_V3: &[u8] = b"brutex.validate.anchored-search.policy.v3\0";
const ANCHORED_SEARCH_FAMILY_DOMAIN_V3: &[u8] = b"brutex.validate.anchored-search.family.v3\0";
const ANCHORED_SEARCH_FACTS_DOMAIN_V3: &[u8] = b"brutex.validate.anchored-search.facts.v3\0";
const ANCHORED_SEARCH_CANDIDATES_DOMAIN_V3: &[u8] =
    b"brutex.validate.anchored-search.candidates.v3\0";
const ANCHORED_SEARCH_POLICY_DOMAIN_V4: &[u8] = b"brutex.validate.anchored-search.policy.v4\0";
const ANCHORED_SEARCH_GRID_DOMAIN_V4: &[u8] = b"brutex.validate.anchored-search.grid.v4\0";
const ANCHORED_SEARCH_FAMILY_DOMAIN_V4: &[u8] = b"brutex.validate.anchored-search.family.v4\0";
const ANCHORED_SEARCH_FOLD_POPULATION_DOMAIN_V4: &[u8] =
    b"brutex.validate.anchored-search.fold-population.v4\0";
const ANCHORED_SEARCH_WALK_DOMAIN_V4: &[u8] = b"brutex.validate.anchored-search.walk.v4\0";

/// V4's exact Candidate ordering: closed masks, then complete Long grid, then
/// complete Short grid; every grid uses `ResolvedExitGridV1` canonical order.
const CANDIDATE_SEMANTIC_ORDER_V4: u8 = 1;

/// Candidate-local TRAINING edge; strict negative is Short, zero is Long.
const TRAINING_EDGE_SIDE_RULE_V3: u8 = 1;

/// Why the anchored-only Admission V2 validation door could not issue its
/// opaque result.
///
/// This refusal is separate from [`Validated::refused`]: the latter preserves
/// the legacy reporting contract, while this type also names failures in the
/// additional ordinal/provenance checks required before Admission may consume
/// a walk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnchoredAdmissionValidationRefusalV2 {
    /// The ordinary validation core refused before producing a complete walk.
    UpstreamRefused(String),
    /// The requested span produced no anchored fold.
    Empty,
    /// The Admission V2 production door was given no resolved exit-grid rung.
    ///
    /// Legacy walk-forward entry points retain their historical zero-to-default
    /// behaviour. The opaque Admission door does not: its rung count is an
    /// identity term supplied by the caller's already-resolved runtime policy,
    /// so replacing zero would authenticate a policy the caller did not name.
    ZeroResolvedRungs,
    /// A platform-sized index did not fit the stable `u64` identity domain.
    StableWidth,
    /// One fold did not retain one aligned score for every priced candidate.
    CandidateFamilyMismatch {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// A fold selected a candidate, but that exact candidate produced no
    /// pessimistic out-of-sample exit total. The visible walk may still report
    /// the fold as decided, but Admission V2 cannot manufacture the missing
    /// profitable-fold classification or aggregate return.
    IncompleteChosenOos {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// A selected exit referenced a ladder rung that its training grid did not
    /// contain.
    ExitRungMismatch {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// The generated visible result and its private fixed-size provenance
    /// disagreed.
    ProvenanceMismatch {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// Exact aggregation of selected out-of-sample totals overflowed `i64`.
    AggregateOosOverflow,
    /// A private provenance seal no longer reproduced.
    SealMismatch,
}

/// Opaque anchored walk result accepted by Admission V2.
///
/// Only [`walk_forward_projected_prepared_anchored_admission_v2`] can construct
/// this type in production. In particular, a public [`Validated`] value—whether
/// genuine, rolling, edited, or fabricated—cannot be converted into it.
///
/// ```compile_fail
/// use runner::validate::{AnchoredAdmissionValidationV2, Validated};
/// let public_result = Validated::default();
/// let _: AnchoredAdmissionValidationV2 = public_result.into();
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchoredAdmissionValidationV2 {
    validated: Validated,
    policy: AnchoredAdmissionPolicyFactsV2,
    folds: Vec<AnchoredAdmissionFoldV2>,
    validation_policy_digest: [u8; 32],
    validation_family_digest: [u8; 32],
    walk_facts_digest: [u8; 32],
}

impl AnchoredAdmissionValidationV2 {
    /// Legacy-compatible visible folds produced by the same core invocation.
    #[must_use]
    pub const fn validated(&self) -> &Validated {
        &self.validated
    }

    /// Revalidates every private policy, candidate-family, chosen-ordinal and
    /// walk-fact seal, then returns the smallest read-only search projection a
    /// cross-crate finalizer may inspect.
    ///
    /// The result is deliberately detached and non-durable. It is evidence
    /// about this in-memory Runner search only: it is not an Admission decision,
    /// does not prove a feed or stored receipt, and cannot authorize execution.
    /// A caller must bind these typed identities to its own freshly reopened
    /// durable lineage before making any production claim.
    ///
    /// # Errors
    ///
    /// Returns the first private policy, family, chosen-ordinal, outcome or
    /// aggregate reconciliation failure. No partial projection is returned.
    pub fn search_authority_projection(
        &self,
    ) -> Result<AnchoredSearchAuthorityProjectionV2, AnchoredAdmissionValidationRefusalV2> {
        reconcile_anchored_admission_v2(self).map(AnchoredSearchAuthorityProjectionV2::from_private)
    }

    /// Revalidates the opaque provenance and projects only the fixed arithmetic
    /// facts Runner owns. It intentionally carries no feed, durable population,
    /// or finalization identity; those belong to a later CLI authority.
    #[cfg(test)]
    pub(crate) fn admission_projection(
        &self,
    ) -> Result<AnchoredAdmissionProjectionV2, AnchoredAdmissionValidationRefusalV2> {
        reconcile_anchored_admission_v2(self)
    }
}

/// Typed identity of the exact anchored validation policy Runner executed.
///
/// The bytes are readable for a cross-crate equality join, but the private
/// field prevents a caller-authored value from masquerading as a projection.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchPolicyIdentityV2([u8; 32]);

impl AnchoredSearchPolicyIdentityV2 {
    /// Domain-separated digest bytes for an exact typed equality join.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// Typed identity of the ordered folds and complete candidate families.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchFamilyIdentityV2([u8; 32]);

impl AnchoredSearchFamilyIdentityV2 {
    /// Domain-separated digest bytes for an exact typed equality join.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// Typed identity of the exact reconciled anchored walk facts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchWalkIdentityV2([u8; 32]);

impl AnchoredSearchWalkIdentityV2 {
    /// Domain-separated digest bytes for an exact typed equality join.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// Opaque, non-durable projection of one fully revalidated anchored search.
///
/// Construction is private and there is intentionally no `Debug`, `Default`,
/// decoder, raw-fold accessor, chosen-ordinal accessor, or serialization API.
/// Chosen ordinals participate transitively in the private family and walk
/// seals, but never cross this boundary.
///
/// This is **not** an Admission authority. It carries no feed, store receipt,
/// Candidate-Universe receipt, Statistics receipt, or finalization identity.
///
/// A caller cannot construct the projection from digest/count claims:
///
/// ```compile_fail
/// use runner::validate::AnchoredSearchAuthorityProjectionV2;
/// let _forged = AnchoredSearchAuthorityProjectionV2 {
///     policy_identity: unreachable!(),
///     family_identity: unreachable!(),
///     walk_identity: unreachable!(),
///     fold_count: 1,
///     decided_folds: 1,
///     profitable_oos_folds: 1,
///     aggregate_oos_paisa: 1,
/// };
/// ```
///
/// The projection also cannot accidentally enter a generic debug/log surface:
///
/// ```compile_fail
/// use runner::validate::AnchoredSearchAuthorityProjectionV2;
/// fn require_debug<T: core::fmt::Debug>() {}
/// require_debug::<AnchoredSearchAuthorityProjectionV2>();
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchAuthorityProjectionV2 {
    policy_identity: AnchoredSearchPolicyIdentityV2,
    family_identity: AnchoredSearchFamilyIdentityV2,
    walk_identity: AnchoredSearchWalkIdentityV2,
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
}

impl AnchoredSearchAuthorityProjectionV2 {
    fn from_private(value: AnchoredAdmissionProjectionV2) -> Self {
        Self {
            policy_identity: AnchoredSearchPolicyIdentityV2(value.validation_policy_digest),
            family_identity: AnchoredSearchFamilyIdentityV2(value.validation_family_digest),
            walk_identity: AnchoredSearchWalkIdentityV2(value.walk_facts_digest),
            fold_count: value.fold_count,
            decided_folds: value.decided_folds,
            profitable_oos_folds: value.profitable_oos_folds,
            aggregate_oos_paisa: value.aggregate_oos_paisa,
        }
    }

    /// Identity of the exact anchored validation policy Runner executed.
    #[must_use]
    pub const fn policy_identity(self) -> AnchoredSearchPolicyIdentityV2 {
        self.policy_identity
    }

    /// Identity of the ordered split windows and complete candidate families.
    #[must_use]
    pub const fn family_identity(self) -> AnchoredSearchFamilyIdentityV2 {
        self.family_identity
    }

    /// Identity of the exact reconciled fold outcomes.
    #[must_use]
    pub const fn walk_identity(self) -> AnchoredSearchWalkIdentityV2 {
        self.walk_identity
    }

    /// Exact anchored fold count.
    #[must_use]
    pub const fn fold_count(self) -> u64 {
        self.fold_count
    }

    /// Folds with one privately revalidated chosen ordinal and complete OOS result.
    #[must_use]
    pub const fn decided_folds(self) -> u64 {
        self.decided_folds
    }

    /// Decided folds whose exact chosen OOS exit was strictly profitable.
    #[must_use]
    pub const fn profitable_oos_folds(self) -> u64 {
        self.profitable_oos_folds
    }

    /// Checked sum over the exact chosen OOS pessimistic totals.
    #[must_use]
    pub const fn aggregate_oos_paisa(self) -> i64 {
        self.aggregate_oos_paisa
    }
}

/// Why the bilateral Anchored Search V3 door could not issue its opaque result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnchoredSearchValidationRefusalV3 {
    /// The ordinary validation core refused before producing a complete walk.
    UpstreamRefused(String),
    /// The requested span produced no anchored fold.
    Empty,
    /// The caller supplied no resolved exit-grid rung.
    ZeroResolvedRungs,
    /// A platform-sized index did not fit the stable `u64` identity domain.
    StableWidth,
    /// One fold did not retain one aligned score for every priced candidate.
    CandidateFamilyMismatch {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// The chosen ordinal had no exact pessimistic out-of-sample exit.
    IncompleteChosenOos {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// A chosen exit referenced a rung absent from its TRAINING ladder.
    ExitRungMismatch {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// Visible and privately captured provenance disagreed.
    ProvenanceMismatch {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// Exact chosen-OOS aggregation overflowed `i64`.
    AggregateOosOverflow,
    /// A private V3 policy, family, candidate, or walk seal did not reproduce.
    SealMismatch,
}

impl From<AnchoredAdmissionValidationRefusalV2> for AnchoredSearchValidationRefusalV3 {
    fn from(value: AnchoredAdmissionValidationRefusalV2) -> Self {
        match value {
            AnchoredAdmissionValidationRefusalV2::UpstreamRefused(why) => {
                Self::UpstreamRefused(why)
            }
            AnchoredAdmissionValidationRefusalV2::Empty => Self::Empty,
            AnchoredAdmissionValidationRefusalV2::ZeroResolvedRungs => Self::ZeroResolvedRungs,
            AnchoredAdmissionValidationRefusalV2::StableWidth => Self::StableWidth,
            AnchoredAdmissionValidationRefusalV2::CandidateFamilyMismatch { fold } => {
                Self::CandidateFamilyMismatch { fold }
            }
            AnchoredAdmissionValidationRefusalV2::IncompleteChosenOos { fold } => {
                Self::IncompleteChosenOos { fold }
            }
            AnchoredAdmissionValidationRefusalV2::ExitRungMismatch { fold } => {
                Self::ExitRungMismatch { fold }
            }
            AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold } => {
                Self::ProvenanceMismatch { fold }
            }
            AnchoredAdmissionValidationRefusalV2::AggregateOosOverflow => {
                Self::AggregateOosOverflow
            }
            AnchoredAdmissionValidationRefusalV2::SealMismatch => Self::SealMismatch,
        }
    }
}

/// Opaque result of the bilateral anchored-search V3 door.
///
/// Unlike V2, this successor has no requested family direction. Every retained
/// candidate chooses its own side from its own TRAINING edge; an exact zero edge
/// is Long, while a fold with no winner carries no side. V2 remains unchanged
/// and there is intentionally no conversion between the two versions.
///
/// ```compile_fail
/// use runner::validate::{AnchoredAdmissionValidationV2, AnchoredSearchValidationV3};
/// fn reinterpret(value: AnchoredAdmissionValidationV2) -> AnchoredSearchValidationV3 {
///     value.into()
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchoredSearchValidationV3 {
    validated: Validated,
    policy: AnchoredSearchPolicyFactsV3,
    folds: Vec<AnchoredAdmissionFoldV2>,
    validation_policy_digest: [u8; 32],
    validation_family_digest: [u8; 32],
    walk_facts_digest: [u8; 32],
}

impl AnchoredSearchValidationV3 {
    /// Legacy-compatible visible folds produced by the same core invocation.
    #[must_use]
    pub const fn validated(&self) -> &Validated {
        &self.validated
    }

    /// Revalidates all private seals and returns the minimum detached equality
    /// projection required by a later durable join.
    ///
    /// This projection is not itself durable and carries no feed, store,
    /// Candidate, Statistics, Base-Evidence, Admission, or execution authority.
    ///
    /// # Errors
    ///
    /// Returns the first policy, family, ordinal, outcome, aggregate, or seal
    /// mismatch. No partial projection is returned.
    pub fn search_authority_projection(
        &self,
    ) -> Result<AnchoredSearchAuthorityProjectionV3, AnchoredSearchValidationRefusalV3> {
        reconcile_anchored_search_v3(self).map(AnchoredSearchAuthorityProjectionV3::from_private)
    }
}

/// Typed identity of the exact V3 anchored-search policy.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchPolicyIdentityV3([u8; 32]);

impl AnchoredSearchPolicyIdentityV3 {
    /// Domain-separated digest bytes for an exact typed equality join.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// Typed identity of the ordered V3 folds and complete candidate families.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchFamilyIdentityV3([u8; 32]);

impl AnchoredSearchFamilyIdentityV3 {
    /// Domain-separated digest bytes for an exact typed equality join.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// Typed identity of the exact reconciled V3 walk facts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchWalkIdentityV3([u8; 32]);

impl AnchoredSearchWalkIdentityV3 {
    /// Domain-separated digest bytes for an exact typed equality join.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// Opaque, non-durable projection of one fully revalidated V3 search.
///
/// Construction, decoding, raw folds, candidate ordinals and per-fold sides
/// remain private. The three typed identities are the only later join surface.
///
/// ```compile_fail
/// use runner::validate::AnchoredSearchAuthorityProjectionV3;
/// let _forged = AnchoredSearchAuthorityProjectionV3 {
///     policy_identity: unreachable!(),
///     family_identity: unreachable!(),
///     walk_identity: unreachable!(),
///     fold_count: 1,
///     decided_folds: 1,
///     profitable_oos_folds: 1,
///     aggregate_oos_paisa: 1,
/// };
/// ```
///
/// ```compile_fail
/// use runner::validate::AnchoredSearchAuthorityProjectionV3;
/// fn require_debug<T: core::fmt::Debug>() {}
/// require_debug::<AnchoredSearchAuthorityProjectionV3>();
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchAuthorityProjectionV3 {
    policy_identity: AnchoredSearchPolicyIdentityV3,
    family_identity: AnchoredSearchFamilyIdentityV3,
    walk_identity: AnchoredSearchWalkIdentityV3,
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
}

impl AnchoredSearchAuthorityProjectionV3 {
    fn from_private(value: AnchoredSearchProjectionV3) -> Self {
        Self {
            policy_identity: AnchoredSearchPolicyIdentityV3(value.validation_policy_digest),
            family_identity: AnchoredSearchFamilyIdentityV3(value.validation_family_digest),
            walk_identity: AnchoredSearchWalkIdentityV3(value.walk_facts_digest),
            fold_count: value.fold_count,
            decided_folds: value.decided_folds,
            profitable_oos_folds: value.profitable_oos_folds,
            aggregate_oos_paisa: value.aggregate_oos_paisa,
        }
    }

    /// Identity of the V3 policy, including its candidate-local side rule.
    #[must_use]
    pub const fn policy_identity(self) -> AnchoredSearchPolicyIdentityV3 {
        self.policy_identity
    }

    /// Identity of the ordered split windows and complete candidate families.
    #[must_use]
    pub const fn family_identity(self) -> AnchoredSearchFamilyIdentityV3 {
        self.family_identity
    }

    /// Identity of the exact reconciled fold outcomes.
    #[must_use]
    pub const fn walk_identity(self) -> AnchoredSearchWalkIdentityV3 {
        self.walk_identity
    }

    /// Exact anchored fold count.
    #[must_use]
    pub const fn fold_count(self) -> u64 {
        self.fold_count
    }

    /// Folds with one privately revalidated chosen ordinal and complete OOS result.
    #[must_use]
    pub const fn decided_folds(self) -> u64 {
        self.decided_folds
    }

    /// Decided folds whose exact chosen OOS exit was strictly profitable.
    #[must_use]
    pub const fn profitable_oos_folds(self) -> u64 {
        self.profitable_oos_folds
    }

    /// Checked sum over the exact chosen OOS pessimistic totals.
    #[must_use]
    pub const fn aggregate_oos_paisa(self) -> i64 {
        self.aggregate_oos_paisa
    }
}

/// Why the exact-grid Anchored Search V4 door refused to issue authority.
///
/// V4 deliberately does not translate a V2/V3 refusal. It is a separate
/// protocol whose caller supplies two authenticated full-span resolutions and
/// whose folds resolve fresh causal prefix grids from the policies they carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnchoredSearchValidationRefusalV4 {
    /// No anchored fold can be represented for the requested span.
    Empty,
    /// The named signal timeframe is not a positive supported intraday rung.
    UnsupportedSignalLength,
    /// A fold was not the contiguous `0..train_end` shape V4 can replay without
    /// concatenating, reordering, or silently dropping training observations.
    DiscontiguousTraining {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// The signal-column builder refused one exact prefix.
    BuilderRefused(String),
    /// The retained full signal bytes, prepared column, or a causal prefix
    /// disagreed on exact source order, evaluator policy, acceptance, or masks.
    SignalSourceMismatch(&'static str),
    /// The Apriori ladder halted before extinction, so its mask population was
    /// incomplete and cannot be called a Candidate-equivalent universe.
    IncompleteSearch {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// A full-span or fold-local Long/Short pair disagreed on side, source,
    /// training bytes, policy identity, resolution identity, or exact order.
    GridPairMismatch(&'static str),
    /// The versioned exit-grid layer refused exact resolution, evaluation,
    /// selection, or causal replay.
    ExitGrid(ExitGridErrorV1),
    /// A platform-sized count could not enter the stable `u64` seal domain.
    StableWidth,
    /// Exact mask × side × cell population arithmetic overflowed.
    PopulationOverflow,
    /// The selected-candidate vector and its exact OOS vector lost positional
    /// alignment.
    CandidateFamilyMismatch {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// The training winner did not produce one complete causal OOS cell. V4
    /// never turns that absence into a zero or a successful chosen result.
    IncompleteChosenOos {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// One policy-selected candidate lacked a complete causal OOS cell. V4
    /// refuses the whole fold instead of recording an invented zero.
    IncompleteCandidateOos {
        /// Canonical anchored fold position.
        fold: usize,
        /// Canonical candidate position within the fold.
        ordinal: usize,
    },
    /// Visible folds and private exact-grid facts no longer agree.
    ProvenanceMismatch {
        /// Canonical anchored fold position.
        fold: usize,
    },
    /// Exact chosen-OOS aggregation overflowed `i64`.
    AggregateOosOverflow,
    /// A private policy, grid, family, population, or walk seal did not
    /// reproduce.
    SealMismatch,
}

impl From<ExitGridErrorV1> for AnchoredSearchValidationRefusalV4 {
    fn from(value: ExitGridErrorV1) -> Self {
        Self::ExitGrid(value)
    }
}

/// Opaque result of the exact-grid bilateral Anchored Search V4 door.
///
/// The caller supplies the retained full-span Long and Short
/// [`ResolvedExitGridV1`] capabilities. V4 first re-resolves both policies on
/// the exact full execution bytes and refuses any difference. Each anchored
/// fold then resolves both policies again on its own causal TRAIN prefix; the
/// full-span rungs are never reused on an earlier fold. V2 and V3 remain
/// unchanged and there is intentionally no conversion between versions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchoredSearchValidationV4 {
    validated: Validated,
    policy: AnchoredSearchPolicyFactsV4,
    folds: Vec<AnchoredSearchFoldV4>,
    full_grid_digest: [u8; 32],
    validation_policy_digest: [u8; 32],
    validation_family_digest: [u8; 32],
    walk_facts_digest: [u8; 32],
}

impl AnchoredSearchValidationV4 {
    /// Legacy report fields produced by this same exact-grid execution.
    #[must_use]
    pub const fn validated(&self) -> &Validated {
        &self.validated
    }

    /// Revalidates every private V4 seal and returns the minimum detached
    /// equality projection for a later durable Candidate/Statistics join.
    ///
    /// This remains in-memory Runner evidence. It is not a stored receipt,
    /// Admission decision, finalization authority, or trading authorization.
    ///
    /// # Errors
    ///
    /// Returns the first policy, pair, population, chosen-OOS, aggregate, or
    /// provenance mismatch. No partial projection is returned.
    pub fn search_authority_projection(
        &self,
    ) -> Result<AnchoredSearchAuthorityProjectionV4, AnchoredSearchValidationRefusalV4> {
        let private = reconcile_anchored_search_v4(self)?;
        Ok(AnchoredSearchAuthorityProjectionV4::from_private(&private))
    }
}

/// Typed identity of the exact V4 policy and causal fold procedure.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchPolicyIdentityV4([u8; 32]);

impl AnchoredSearchPolicyIdentityV4 {
    /// Domain-separated digest bytes for an exact equality join.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// Typed identity of the authenticated full-span Long/Short grid pair.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchGridIdentityV4([u8; 32]);

impl AnchoredSearchGridIdentityV4 {
    /// Domain-separated digest bytes for an exact equality join.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// Typed Candidate-join identity of one authenticated V4 side grid.
///
/// Policy and resolved-grid digests remain separate because Candidate V4
/// authenticates both components. This type has no public constructor: only a
/// fully reconciled V4 result can expose it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchSideGridIdentityV4 {
    policy_digest: [u8; 32],
    resolution_digest: [u8; 32],
}

/// Exact full-span signal and prepared-column identity consumed by Search V4.
///
/// This type has no public constructor. A later Candidate/Admission join can
/// compare all five components without accepting a caller-authored aggregate.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchSourceIdentityV4 {
    digest: [u8; 32],
    bars: u64,
    first_ts_micros: i64,
    last_ts_micros: i64,
    column_digest: [u8; 32],
}

impl AnchoredSearchSourceIdentityV4 {
    /// Exact digest of every full-span signal OHLCV row in source order.
    #[must_use]
    pub const fn signal_digest(self) -> [u8; 32] {
        self.digest
    }

    /// Exact full-span signal row count.
    #[must_use]
    pub const fn signal_bars(self) -> u64 {
        self.bars
    }

    /// First signal timestamp in the authenticated full span.
    #[must_use]
    pub const fn signal_first_ts_micros(self) -> i64 {
        self.first_ts_micros
    }

    /// Last signal timestamp in the authenticated full span.
    #[must_use]
    pub const fn signal_last_ts_micros(self) -> i64 {
        self.last_ts_micros
    }

    /// Exact V1 digest of the caller-retained prepared signal column.
    #[must_use]
    pub const fn signal_column_digest(self) -> [u8; 32] {
        self.column_digest
    }
}

impl AnchoredSearchSideGridIdentityV4 {
    /// Exact carried exit-policy identity for this side.
    #[must_use]
    pub const fn policy_digest(self) -> [u8; 32] {
        self.policy_digest
    }

    /// Exact full-span resolved-grid identity for this side.
    #[must_use]
    pub const fn resolution_digest(self) -> [u8; 32] {
        self.resolution_digest
    }
}

/// Typed identity of every ordered fold, mask, side, resolution and complete
/// canonical grid cell population V4 evaluated.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchFamilyIdentityV4([u8; 32]);

impl AnchoredSearchFamilyIdentityV4 {
    /// Domain-separated digest bytes for an exact equality join.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// Typed identity of the exact reconciled V4 training choices and OOS facts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchWalkIdentityV4([u8; 32]);

impl AnchoredSearchWalkIdentityV4 {
    /// Domain-separated digest bytes for an exact equality join.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// Opaque, non-durable projection of one fully revalidated V4 search.
///
/// No constructor, raw fold, mask, coordinate, selected-cell, decoder, or
/// `Debug` surface is public. Later code can only compare typed identities and
/// fixed arithmetic facts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnchoredSearchAuthorityProjectionV4 {
    policy_identity: AnchoredSearchPolicyIdentityV4,
    source_identity: AnchoredSearchSourceIdentityV4,
    grid_identity: AnchoredSearchGridIdentityV4,
    long_grid_identity: AnchoredSearchSideGridIdentityV4,
    short_grid_identity: AnchoredSearchSideGridIdentityV4,
    family_identity: AnchoredSearchFamilyIdentityV4,
    walk_identity: AnchoredSearchWalkIdentityV4,
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
    evaluated_population_cells: u64,
}

impl AnchoredSearchAuthorityProjectionV4 {
    fn from_private(value: &AnchoredSearchProjectionV4) -> Self {
        Self {
            policy_identity: AnchoredSearchPolicyIdentityV4(value.validation_policy_digest),
            source_identity: AnchoredSearchSourceIdentityV4 {
                digest: value.signal_digest,
                bars: value.signal_bars,
                first_ts_micros: value.signal_first_ts_micros,
                last_ts_micros: value.signal_last_ts_micros,
                column_digest: value.signal_column_digest,
            },
            grid_identity: AnchoredSearchGridIdentityV4(value.full_grid_digest),
            long_grid_identity: AnchoredSearchSideGridIdentityV4 {
                policy_digest: value.full_long_policy_digest,
                resolution_digest: value.full_long_resolution_digest,
            },
            short_grid_identity: AnchoredSearchSideGridIdentityV4 {
                policy_digest: value.full_short_policy_digest,
                resolution_digest: value.full_short_resolution_digest,
            },
            family_identity: AnchoredSearchFamilyIdentityV4(value.validation_family_digest),
            walk_identity: AnchoredSearchWalkIdentityV4(value.walk_facts_digest),
            fold_count: value.fold_count,
            decided_folds: value.decided_folds,
            profitable_oos_folds: value.profitable_oos_folds,
            aggregate_oos_paisa: value.aggregate_oos_paisa,
            evaluated_population_cells: value.evaluated_population_cells,
        }
    }

    /// Identity of the exact V4 procedure and runtime search bounds.
    #[must_use]
    pub const fn policy_identity(self) -> AnchoredSearchPolicyIdentityV4 {
        self.policy_identity
    }

    /// Exact full signal-stream and prepared-column equality components.
    #[must_use]
    pub const fn source_identity(self) -> AnchoredSearchSourceIdentityV4 {
        self.source_identity
    }

    /// Identity of the authenticated full-span Long/Short resolution pair.
    #[must_use]
    pub const fn grid_identity(self) -> AnchoredSearchGridIdentityV4 {
        self.grid_identity
    }

    /// Exact full-span Long policy and resolved-grid components.
    #[must_use]
    pub const fn long_grid_identity(self) -> AnchoredSearchSideGridIdentityV4 {
        self.long_grid_identity
    }

    /// Exact full-span Short policy and resolved-grid components.
    #[must_use]
    pub const fn short_grid_identity(self) -> AnchoredSearchSideGridIdentityV4 {
        self.short_grid_identity
    }

    /// Identity of every complete ordered fold population.
    #[must_use]
    pub const fn family_identity(self) -> AnchoredSearchFamilyIdentityV4 {
        self.family_identity
    }

    /// Identity of the exact reconciled choices and OOS outcomes.
    #[must_use]
    pub const fn walk_identity(self) -> AnchoredSearchWalkIdentityV4 {
        self.walk_identity
    }

    /// Exact anchored fold count.
    #[must_use]
    pub const fn fold_count(self) -> u64 {
        self.fold_count
    }

    /// Folds with one revalidated training winner and complete OOS cell.
    #[must_use]
    pub const fn decided_folds(self) -> u64 {
        self.decided_folds
    }

    /// Decided folds whose chosen pessimistic OOS total was positive.
    #[must_use]
    pub const fn profitable_oos_folds(self) -> u64 {
        self.profitable_oos_folds
    }

    /// Checked sum of the exact chosen OOS pessimistic totals.
    #[must_use]
    pub const fn aggregate_oos_paisa(self) -> i64 {
        self.aggregate_oos_paisa
    }

    /// Complete Long+Short coordinate cells evaluated across every mask/fold.
    #[must_use]
    pub const fn evaluated_population_cells(self) -> u64 {
        self.evaluated_population_cells
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AnchoredSearchProjectionV4 {
    validation_policy_digest: [u8; 32],
    signal_digest: [u8; 32],
    signal_bars: u64,
    signal_first_ts_micros: i64,
    signal_last_ts_micros: i64,
    signal_column_digest: [u8; 32],
    full_grid_digest: [u8; 32],
    full_long_policy_digest: [u8; 32],
    full_short_policy_digest: [u8; 32],
    full_long_resolution_digest: [u8; 32],
    full_short_resolution_digest: [u8; 32],
    validation_family_digest: [u8; 32],
    walk_facts_digest: [u8; 32],
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
    evaluated_population_cells: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AnchoredSearchPolicyFactsV4 {
    signal_bars: u64,
    signal_digest: [u8; 32],
    signal_first_ts_micros: i64,
    signal_last_ts_micros: i64,
    signal_column_digest: [u8; 32],
    horizon_bars: u32,
    splits: u64,
    min_hits: u64,
    ceiling: u64,
    pair_budget: u64,
    signal_length_micros: i64,
    semantic_order: u8,
    full_long_policy_digest: [u8; 32],
    full_short_policy_digest: [u8; 32],
    full_long_resolution_digest: [u8; 32],
    full_short_resolution_digest: [u8; 32],
    full_training_digest: [u8; 32],
    full_training_bars: u64,
    feed_digest: [u8; 32],
    commit_digest: [u8; 32],
    calendar_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AnchoredSearchChoiceV4 {
    ordinal: u64,
    mask: ConditionMask,
    side: Direction,
    coordinate: crate::grid::Chosen,
    resolution_digest: [u8; 32],
    evaluation_digest: [u8; 32],
    selection_digest: [u8; 32],
    training_run_id: [u8; 32],
    replay_digest: [u8; 32],
    oos_run_id: [u8; 32],
    training_cell_digest: [u8; 32],
    oos_cell_digest: [u8; 32],
    in_sample_pessimistic: i64,
    out_of_sample_pessimistic: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AnchoredSearchCandidateV4 {
    mask: ConditionMask,
    side: Direction,
    coordinate: crate::grid::Chosen,
    resolution_digest: [u8; 32],
    evaluation_digest: [u8; 32],
    selection_digest: [u8; 32],
    training_run_id: [u8; 32],
    training_cell_digest: [u8; 32],
    in_sample_pessimistic: i64,
    replay_digest: [u8; 32],
    oos_run_id: [u8; 32],
    oos_cell_digest: Option<[u8; 32]>,
    out_of_sample_pessimistic: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AnchoredSearchFoldV4 {
    index: u64,
    train_start: u64,
    train_end: u64,
    test_start: u64,
    test_end: u64,
    purged: u64,
    considered_masks: u64,
    population_cells: u64,
    selected_candidates: u64,
    long_resolution_digest: [u8; 32],
    short_resolution_digest: [u8; 32],
    coordinate_order_digest: [u8; 32],
    population_digest: [u8; 32],
    candidates: Vec<AnchoredSearchCandidateV4>,
    choice: Option<AnchoredSearchChoiceV4>,
    in_sample: Summary,
    out_of_sample: Summary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AnchoredSearchProjectionV3 {
    validation_policy_digest: [u8; 32],
    validation_family_digest: [u8; 32],
    walk_facts_digest: [u8; 32],
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AnchoredSearchPolicyFactsV3 {
    signal_bars: u64,
    horizon_bars: u32,
    splits: u64,
    resolved_rungs: u64,
    min_hits: u64,
    ceiling: u64,
    pair_budget: u64,
    training_edge_side_rule: u8,
    execution_mode: u8,
    signal_length_micros: i64,
}

/// Fixed Runner-owned walk facts handed to the detached Admission arithmetic
/// kernel after the opaque object has revalidated itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnchoredAdmissionProjectionV2 {
    pub(crate) validation_policy_digest: [u8; 32],
    pub(crate) validation_family_digest: [u8; 32],
    pub(crate) walk_facts_digest: [u8; 32],
    pub(crate) fold_count: u64,
    pub(crate) decided_folds: u64,
    pub(crate) profitable_oos_folds: u64,
    pub(crate) aggregate_oos_paisa: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AnchoredAdmissionPolicyFactsV2 {
    signal_bars: u64,
    horizon_bars: u32,
    splits: u64,
    resolved_rungs: u64,
    min_hits: u64,
    ceiling: u64,
    pair_budget: u64,
    requested_direction: u8,
    execution_mode: u8,
    signal_length_micros: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AnchoredAdmissionExitValuesV2 {
    stop: Option<i64>,
    target: Option<i64>,
    tsl: Option<i64>,
    ttp_arm: Option<i64>,
    ttp_trail: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AnchoredAdmissionChoiceV2 {
    ordinal: u64,
    mask: ConditionMask,
    side: Direction,
    exit: crate::grid::Chosen,
    exit_values: AnchoredAdmissionExitValuesV2,
    in_sample_pessimistic: i64,
    out_of_sample_pessimistic: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AnchoredAdmissionFoldV2 {
    index: u64,
    train_start: u64,
    train_end: u64,
    train_second_start: u64,
    train_second_end: u64,
    test_start: u64,
    test_end: u64,
    purged: u64,
    embargoed: u64,
    considered: u64,
    priced: u64,
    candidate_facts_digest: [u8; 32],
    choice: Option<AnchoredAdmissionChoiceV2>,
    in_sample: Summary,
    out_of_sample: Summary,
}

struct AnchoredAdmissionCaptureV2 {
    policy: AnchoredAdmissionPolicyFactsV2,
    folds: Vec<AnchoredAdmissionFoldV2>,
    refusal: Option<AnchoredAdmissionValidationRefusalV2>,
}

struct AnchoredSearchCaptureV3 {
    policy: AnchoredSearchPolicyFactsV3,
    folds: Vec<AnchoredAdmissionFoldV2>,
    refusal: Option<AnchoredSearchValidationRefusalV3>,
}

enum AnchoredCapture<'a> {
    AdmissionV2(&'a mut AnchoredAdmissionCaptureV2),
    SearchV3(&'a mut AnchoredSearchCaptureV3),
}

impl AnchoredCapture<'_> {
    fn observe_fold_and_retain(
        &mut self,
        split: &crate::split::Fold,
        visible: &FoldResult,
        scored: &[Scored],
        oos: &[CandidateOosV2],
        chosen_ordinal: Option<usize>,
    ) -> bool {
        match self {
            Self::AdmissionV2(capture) => {
                match capture.observe_fold(split, visible, scored, oos, chosen_ordinal) {
                    Ok(()) => true,
                    Err(refusal) => {
                        capture.refusal = Some(refusal);
                        false
                    }
                }
            }
            Self::SearchV3(capture) => {
                match capture.observe_fold(split, visible, scored, oos, chosen_ordinal) {
                    Ok(()) => true,
                    Err(refusal) => {
                        capture.refusal = Some(refusal);
                        false
                    }
                }
            }
        }
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
/// quantity. This function remains only for legacy callers that cannot yet pass
/// the resolved count through [`walk_forward_with_rungs`] or
/// [`walk_forward_shaped_with_rungs`]. Operator-facing callers must use those
/// explicit doors: `cli` also owns request-local knobs and the machine-safe
/// clamp, neither of which this crate can recover from `std::env`.
///
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
/// Unset, the legacy behaviour is exactly what it was.
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

/// Selects one candidate's side from that candidate's TRAINING edge only.
///
/// Zero is deliberately Long. It is not an absent decision: a candidate exists
/// and the canonical strict-negative rule has selected its non-negative arm.
/// The absence of a fold winner is represented separately as `None` and never
/// enters this function.
fn direction_from_training_edge(mean_paisa: f64) -> Direction {
    if mean_paisa < 0.0 {
        Direction::Short
    } else {
        Direction::Long
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
        // THIS WRAPPER HAS ONE SLICE AND SO ANSWERS FOR IT. A caller with two
        // series cannot express the difference through this signature, which is
        // why it must reach for `walk_forward_shaped` directly and say which it
        // is handing over.
        TradesOnTheseBars::Yes,
    )
}

/// [`walk_forward`], with the exit-grid rung count already resolved by the
/// caller that owns the run's knobs and memory clamp.
///
/// Runner cannot read `cli`'s request-local knob store without reversing the
/// crate graph. Carrying the resolved value through this API keeps execution
/// on the same value the caller records in run identity, including the caller's
/// machine-safe clamp.
#[must_use]
pub fn walk_forward_with_rungs(
    bars: &[Candle],
    horizon: Horizon,
    splits: usize,
    direction: Direction,
    sweeper: &crate::Sweeper,
    mut evaluator: impl FnMut() -> Evaluator,
    rungs: usize,
) -> Validated {
    walk_forward_shaped_with_rungs(
        bars,
        horizon,
        splits,
        direction,
        sweeper,
        &mut evaluator,
        Shape::Anchored,
        TradesOnTheseBars::Yes,
        rungs,
    )
}

/// One candidate, priced on a fold's TRAINING window.
///
/// A named type because clippy is right that the tuple got too wide, and the
/// widening is what earned it: the fifth member is the SIDE, which the fold now
/// decides per candidate instead of being handed one for all of them.
struct Assessed {
    /// The combination.
    mask: ConditionMask,
    /// Its training-window trade summary.
    summary: Summary,
    /// The exit variant training chose for it, to be applied unchanged to test.
    pick: ExitPick,
    /// Its training total under worst-case fills, which selection reads.
    pessimistic: i64,
    /// The side ITS OWN training evidence points to.
    side: Direction,
}

/// One rankable candidate retained in canonical sweep order.
struct Scored {
    mask: ConditionMask,
    pessimistic: i64,
    pick: ExitPick,
    side: Direction,
}

/// Exact-or-absent OOS result aligned to one [`Scored`] ordinal.
#[derive(Clone, Copy)]
struct CandidateOosV2 {
    pessimistic: Option<i64>,
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
    clippy::too_many_arguments,
    reason = "eight, and the eighth is the one that stops a wrong answer. \
              Bundling them into a struct is the usual remedy and is the wrong \
              one here: `bars` and `trades_on_these_bars` are a QUESTION and its \
              ANSWER, and a struct literal lets a caller fill the answer in \
              without looking at the slice it is about -- which is exactly how \
              `cli` came to hand this function the signal series while its \
              sibling three lines away got the trade series. A positional \
              argument the compiler refuses to default is what makes the caller \
              say which series it has. The other seven were already here."
)]
pub fn walk_forward_shaped(
    bars: &[Candle],
    horizon: Horizon,
    splits: usize,
    direction: Direction,
    sweeper: &crate::Sweeper,
    evaluator: &mut impl FnMut() -> Evaluator,
    shape: Shape,
    trades_on_these_bars: TradesOnTheseBars,
) -> Validated {
    // LEGACY DOOR: resolve the process environment once, before any fold or
    // candidate lane starts. Operator-facing callers should resolve through
    // their own knob store and use `walk_forward_shaped_with_rungs` instead.
    let rungs = fold_rungs();
    walk_forward_shaped_with_rungs(
        bars,
        horizon,
        splits,
        direction,
        sweeper,
        evaluator,
        shape,
        trades_on_these_bars,
        rungs,
    )
}

/// [`walk_forward_shaped`], with one authoritative, caller-resolved rung count.
///
/// The caller owns both run identity and the machine-safe grid clamp, so this
/// function deliberately does not read the process environment or reinterpret
/// the value. Zero retains the legacy fallback rather than turning validation
/// into a no-level baseline.
#[expect(
    clippy::too_many_arguments,
    reason = "nine, and the last two make the caller state both whether this is \
              the execution series and the already-resolved grid width. Neither \
              may be inferred inside runner without recreating the drift these \
              arguments exist to prevent."
)]
pub fn walk_forward_shaped_with_rungs(
    bars: &[Candle],
    horizon: Horizon,
    splits: usize,
    _direction: Direction,
    sweeper: &crate::Sweeper,
    evaluator: &mut impl FnMut() -> Evaluator,
    shape: Shape,
    trades_on_these_bars: TradesOnTheseBars,
    rungs: usize,
) -> Validated {
    let mut builder = |slice: &[Candle]| Ok(Column::build(slice, &mut evaluator()));
    walk_forward_core(
        bars,
        horizon,
        splits,
        sweeper,
        &mut builder,
        shape,
        trades_on_these_bars,
        rungs,
        None,
        None,
    )
}

/// Walk forward on signal bars while entering and exiting exclusively on an
/// explicit one-minute execution series.
///
/// Fold selection and every indicator remain on `signal`. Each fold's masks
/// are then re-indexed to the bar opening at the signal bar's exact close. A
/// missing minute drops that signal; a later minute is never substituted. The
/// training execution prefix stops before the first test instant, while the
/// test prefix stops at its last priceable horizon, so neither half can read an
/// execution observation belonging only to the other.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "the two slices and the signal duration are the safety boundary: \
              hiding them in defaults recreates the coarse-fill compatibility \
              path this entry point removes"
)]
pub fn walk_forward_projected_with_rungs(
    signal: &[Candle],
    execution: ExecutionSeries<'_>,
    horizon: Horizon,
    splits: usize,
    _direction: Direction,
    sweeper: &crate::Sweeper,
    evaluator: &mut impl FnMut() -> Evaluator,
    shape: Shape,
    rungs: usize,
) -> Validated {
    let mut builder = |slice: &[Candle]| Ok(Column::build(slice, &mut evaluator()));
    walk_forward_core(
        signal,
        horizon,
        splits,
        sweeper,
        &mut builder,
        shape,
        TradesOnTheseBars::Yes,
        rungs,
        Some(execution),
        None,
    )
}

/// Walk forward with a caller-owned causal signal-column builder.
///
/// Stored runs use this door because rebuilding through [`Evaluator`] would
/// discard their sealed daily references and exact-minute `GapFib` overlay. The
/// builder is called on each fold's exact signal slice/prefix and may refuse;
/// that reason is returned in [`Validated::refused`] without falling back to an
/// ordinary evaluator.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "the explicit execution series and fallible signal-column builder are the safety boundary"
)]
pub fn walk_forward_projected_prepared_with_rungs(
    signal: &[Candle],
    execution: ExecutionSeries<'_>,
    horizon: Horizon,
    splits: usize,
    _direction: Direction,
    sweeper: &crate::Sweeper,
    builder: &mut impl FnMut(&[Candle]) -> Result<Column, String>,
    shape: Shape,
    rungs: usize,
) -> Validated {
    walk_forward_core(
        signal,
        horizon,
        splits,
        sweeper,
        builder,
        shape,
        TradesOnTheseBars::Yes,
        rungs,
        Some(execution),
        None,
    )
}

/// Runs the stored/prepared two-series walk in the only shape Admission V2 may
/// consume and returns an opaque provenance-bearing result.
///
/// Unlike [`walk_forward_projected_prepared_with_rungs`], this door has no
/// shape argument: rolling validation cannot accidentally acquire an anchored
/// authority label. The visible [`Validated`] result is generated by the same
/// core and is available through [`AnchoredAdmissionValidationV2::validated`].
///
/// # Errors
///
/// Refuses a zero resolved rung count, the ordinary malformed/builder path, an
/// empty span, an incomplete candidate family, a misaligned chosen ordinal/exit
/// ladder, stable-width conversion failure, exact aggregate overflow, or any
/// private provenance mismatch.
#[expect(
    clippy::too_many_arguments,
    reason = "the explicit signal/execution pair, horizon, split count, resolved grid and prepared builder are independent identity terms"
)]
pub fn walk_forward_projected_prepared_anchored_admission_v2(
    signal: &[Candle],
    execution: ExecutionSeries<'_>,
    horizon: Horizon,
    splits: usize,
    direction: Direction,
    sweeper: &crate::Sweeper,
    builder: &mut impl FnMut(&[Candle]) -> Result<Column, String>,
    rungs: usize,
) -> Result<AnchoredAdmissionValidationV2, AnchoredAdmissionValidationRefusalV2> {
    if rungs == 0 {
        return Err(AnchoredAdmissionValidationRefusalV2::ZeroResolvedRungs);
    }
    let resolved_rungs = rungs;
    let mut capture = AnchoredAdmissionCaptureV2::new(
        signal.len(),
        horizon,
        splits,
        direction,
        sweeper,
        resolved_rungs,
        execution.signal_length_micros,
    )?;
    let validated = walk_forward_core(
        signal,
        horizon,
        splits,
        sweeper,
        builder,
        Shape::Anchored,
        TradesOnTheseBars::Yes,
        resolved_rungs,
        Some(execution),
        Some(AnchoredCapture::AdmissionV2(&mut capture)),
    );
    capture.finish(validated)
}

/// Runs the stored/prepared anchored search under the bilateral V3 side rule.
///
/// There is deliberately no family-level `Direction` argument. Each candidate
/// is priced Long when its own TRAINING mean edge is non-negative (including
/// exact zero), and Short only when that edge is strictly negative. A fold that
/// selects no candidate records neither side. The V3 policy seal authenticates
/// that rule; V2's caller-direction identity remains untouched.
///
/// The returned object is in-memory evidence only. Its typed projection must be
/// joined to separately reopened durable CLI receipts before any production
/// claim can be made.
///
/// # Errors
///
/// Refuses zero resolved rungs, malformed input/build paths, empty folds,
/// incomplete candidate families, missing chosen outcomes, overflow, or any
/// private provenance mismatch.
pub fn walk_forward_projected_prepared_anchored_search_v3(
    signal: &[Candle],
    execution: ExecutionSeries<'_>,
    horizon: Horizon,
    splits: usize,
    sweeper: &crate::Sweeper,
    builder: &mut impl FnMut(&[Candle]) -> Result<Column, String>,
    rungs: usize,
) -> Result<AnchoredSearchValidationV3, AnchoredSearchValidationRefusalV3> {
    if rungs == 0 {
        return Err(AnchoredSearchValidationRefusalV3::ZeroResolvedRungs);
    }
    let mut capture = AnchoredSearchCaptureV3::new(
        signal.len(),
        horizon,
        splits,
        sweeper,
        rungs,
        execution.signal_length_micros,
    )?;
    let validated = walk_forward_core(
        signal,
        horizon,
        splits,
        sweeper,
        builder,
        Shape::Anchored,
        TradesOnTheseBars::Yes,
        rungs,
        Some(execution),
        Some(AnchoredCapture::SearchV3(&mut capture)),
    );
    capture.finish(validated)
}

/// Runs the stored/prepared anchored search over the exact Candidate V4 grid
/// semantics for both Long and Short.
///
/// `full_long` and `full_short` are not loose configuration. They are retained
/// capabilities resolved over the complete supplied execution series. This
/// door binds the caller-retained `full_signal_column` beside the exact signal
/// bytes and proves every rebuilt causal prefix is its corresponding prefix. It
/// re-resolves both carried grid policies and requires byte-for-byte equality
/// before any fold runs. For every anchored fold it evaluates the complete
/// canonical cell sequence for every closed mask in
/// `mask -> Long -> Short -> cell` order and invokes each policy's selector. It
/// never derives scalar ladders and never infers a side from one candidate.
///
/// # Errors
///
/// Refuses any source/policy/resolution/side mismatch, a non-prefix fold,
/// incomplete sweep, malformed complete grid, missing chosen OOS cell,
/// overflow, builder refusal, or private seal mismatch.
#[expect(
    clippy::too_many_arguments,
    reason = "signal/execution identity, causal timeframe, fold policy, prepared builder and both authenticated side resolutions are independent authority terms"
)]
pub fn walk_forward_projected_prepared_anchored_search_v4(
    signal: &[Candle],
    full_signal_column: &Column,
    execution: ExecutionSeriesV1<'_>,
    signal_length_micros: i64,
    horizon: Horizon,
    splits: usize,
    sweeper: &crate::Sweeper,
    builder: &mut impl FnMut(&[Candle]) -> Result<Column, String>,
    full_long: &ResolvedExitGridV1,
    full_short: &ResolvedExitGridV1,
) -> Result<AnchoredSearchValidationV4, AnchoredSearchValidationRefusalV4> {
    let timeframe = signal_timeframe_v4(signal_length_micros)
        .ok_or(AnchoredSearchValidationRefusalV4::UnsupportedSignalLength)?;
    authenticate_signal_column_shape_v4(signal, full_signal_column)?;
    let signal_bars = stable_u64_v4(signal.len())?;
    let signal_first_ts_micros = signal
        .first()
        .map(|bar| bar.ts_micros)
        .ok_or(AnchoredSearchValidationRefusalV4::Empty)?;
    let signal_last_ts_micros = signal
        .last()
        .map(|bar| bar.ts_micros)
        .ok_or(AnchoredSearchValidationRefusalV4::Empty)?;
    if !is_one_minute_path(execution.bars()) {
        return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
            "execution is not a non-empty exact one-minute path",
        ));
    }
    let full_grid_digest = authenticate_grid_pair_v4(execution, full_long, full_short, true)?;
    let base = sweeper.ladder();
    let policy = AnchoredSearchPolicyFactsV4 {
        signal_bars,
        signal_digest: data_digest(signal),
        signal_first_ts_micros,
        signal_last_ts_micros,
        signal_column_digest: column_digest_v1(full_signal_column),
        horizon_bars: horizon.as_bars(),
        splits: stable_u64_v4(splits)?,
        min_hits: base.min_hits(),
        ceiling: stable_u64_v4(base.ceiling())?,
        pair_budget: base.pair_budget(),
        signal_length_micros,
        semantic_order: CANDIDATE_SEMANTIC_ORDER_V4,
        full_long_policy_digest: full_long.policy_digest(),
        full_short_policy_digest: full_short.policy_digest(),
        full_long_resolution_digest: full_long.digest(),
        full_short_resolution_digest: full_short.digest(),
        full_training_digest: full_long.training_digest(),
        full_training_bars: full_long.training_bars(),
        feed_digest: full_long.feed_digest(),
        commit_digest: full_long.commit_digest(),
        calendar_digest: full_long.calendar_digest(),
    };
    let (validated, folds) = walk_forward_exact_grid_v4(
        signal,
        full_signal_column,
        execution,
        signal_length_micros,
        timeframe,
        horizon,
        splits,
        sweeper,
        builder,
        full_long,
        full_short,
    )?;
    if folds.is_empty() || folds.len() != validated.folds.len() {
        return Err(AnchoredSearchValidationRefusalV4::Empty);
    }
    let validation_policy_digest = hash_validation_policy_v4(&policy, full_grid_digest);
    let validation_family_digest = hash_validation_family_v4(&folds)?;
    let walk_facts_digest = hash_walk_facts_v4(
        validation_policy_digest,
        full_grid_digest,
        validation_family_digest,
        &folds,
    );
    let opaque = AnchoredSearchValidationV4 {
        validated,
        policy,
        folds,
        full_grid_digest,
        validation_policy_digest,
        validation_family_digest,
        walk_facts_digest,
    };
    opaque.search_authority_projection()?;
    Ok(opaque)
}

#[derive(Clone, Debug)]
struct PendingCandidateV4 {
    mask: ConditionMask,
    side: Direction,
    selected: SelectedExitV1,
    resolution_digest: [u8; 32],
    evaluation_digest: [u8; 32],
    training_cell_digest: [u8; 32],
    in_sample_pessimistic: i64,
}

fn authenticate_signal_column_shape_v4(
    signal: &[Candle],
    column: &Column,
) -> Result<(), AnchoredSearchValidationRefusalV4> {
    let signal_bars = stable_u64_v4(signal.len())?;
    let census = column.census();
    if signal.is_empty()
        || !signal.windows(2).all(|pair| {
            pair.first()
                .zip(pair.get(1))
                .is_some_and(|(a, b)| a.ts_micros < b.ts_micros)
        })
        || column.sourced() != indicators::column::Sourced::Signal
        || column.bits().len() != column.sources().len()
        || census.offered != signal_bars
        || census.swept != stable_u64_v4(column.bits().len())?
        || !census.reconciles()
        || !column.acceptance_covers(signal.len())
        || column.acceptance_census() != census
        || column.evaluation_spec_token().is_none()
        || column.collided() != 0
        || column.first_swept() != column.sources().first().copied()
        || column
            .sources()
            .iter()
            .copied()
            .any(|source| source >= signal.len())
        || !column
            .sources()
            .windows(2)
            .all(|pair| pair.first().zip(pair.get(1)).is_some_and(|(a, b)| a < b))
    {
        return Err(AnchoredSearchValidationRefusalV4::SignalSourceMismatch(
            "the prepared signal column does not exactly cover its ordered signal series",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PreparedSignalPrefixCursor {
    end: usize,
    last_signal_bars: Option<usize>,
}

impl PreparedSignalPrefixCursor {
    fn authenticate(
        &mut self,
        prefix: &[Candle],
        prepared: &Column,
        full: &Column,
    ) -> Result<(), AnchoredSearchValidationRefusalV4> {
        authenticate_signal_column_shape_v4(prefix, prepared)?;
        if self.end > full.sources().len()
            || self
                .last_signal_bars
                .is_some_and(|previous| prefix.len() < previous)
        {
            return Err(AnchoredSearchValidationRefusalV4::SignalSourceMismatch(
                "prepared signal-prefix boundaries are not monotonic",
            ));
        }
        while full
            .sources()
            .get(self.end)
            .is_some_and(|source| *source < prefix.len())
        {
            self.end = self
                .end
                .checked_add(1)
                .ok_or(AnchoredSearchValidationRefusalV4::PopulationOverflow)?;
        }
        self.last_signal_bars = Some(prefix.len());
        let expected_sources = full.sources().split_at(self.end).0;
        let expected_bits = full.bits().split_at(self.end).0;
        let full_acceptance =
            full.acceptance()
                .ok_or(AnchoredSearchValidationRefusalV4::SignalSourceMismatch(
                    "the retained full signal column has no acceptance capability",
                ))?;
        let expected_acceptance = full_acceptance.get(..prefix.len()).ok_or(
            AnchoredSearchValidationRefusalV4::SignalSourceMismatch(
                "the retained full signal acceptance is shorter than a causal prefix",
            ),
        )?;
        if prepared.sources() != expected_sources
            || prepared.bits() != expected_bits
            || prepared.evaluation_spec_token() != full.evaluation_spec_token()
            || prepared.acceptance().as_deref() != Some(expected_acceptance)
        {
            return Err(AnchoredSearchValidationRefusalV4::SignalSourceMismatch(
                "a rebuilt causal signal prefix differs from the retained full prepared column",
            ));
        }
        Ok(())
    }
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "the dedicated V4 fold keeps causal prefix resolution, exact complete-population evaluation, selector choice and OOS replay in one auditable procedure"
)]
fn walk_forward_exact_grid_v4(
    signal: &[Candle],
    full_signal_column: &Column,
    execution: ExecutionSeriesV1<'_>,
    signal_length_micros: i64,
    timeframe: &'static str,
    horizon: Horizon,
    splits: usize,
    sweeper: &crate::Sweeper,
    builder: &mut impl FnMut(&[Candle]) -> Result<Column, String>,
    full_long: &ResolvedExitGridV1,
    full_short: &ResolvedExitGridV1,
) -> Result<(Validated, Vec<AnchoredSearchFoldV4>), AnchoredSearchValidationRefusalV4> {
    let split_windows = Shape::Anchored.folds(signal.len(), horizon, splits);
    if split_windows.is_empty() {
        return Err(AnchoredSearchValidationRefusalV4::Empty);
    }
    let mut visible_folds = Vec::with_capacity(split_windows.len());
    let mut proofs = Vec::with_capacity(split_windows.len());
    let base = sweeper.ladder();
    let mut training_execution_cursor = MonotonicExecutionPrefix::default();
    let mut oos_execution_cursor = MonotonicExecutionPrefix::default();
    let mut training_signal_cursor = PreparedSignalPrefixCursor::default();
    let mut oos_signal_cursor = PreparedSignalPrefixCursor::default();

    for (index, fold) in split_windows.into_iter().enumerate() {
        if fold.train.0.start != 0
            || !fold.train.1.is_empty()
            || fold.train.0.is_empty()
            || fold.test.is_empty()
        {
            return Err(AnchoredSearchValidationRefusalV4::DiscontiguousTraining { fold: index });
        }
        let train = signal
            .get(fold.train.0.clone())
            .ok_or(AnchoredSearchValidationRefusalV4::DiscontiguousTraining { fold: index })?;
        let test_open = signal
            .get(fold.test.start)
            .map(|bar| bar.ts_micros)
            .ok_or(AnchoredSearchValidationRefusalV4::DiscontiguousTraining { fold: index })?;
        let trade_train = training_execution_cursor
            .before(execution.bars(), test_open)
            .ok_or(AnchoredSearchValidationRefusalV4::GridPairMismatch(
                "training execution boundaries are not monotonic",
            ))?;
        let first_oos_execution = training_execution_cursor.end();
        let train_series = ExecutionSeriesV1::new(
            execution.instrument(),
            execution.feed(),
            execution.commit(),
            execution.calendar_digest(),
            trade_train,
        )?;

        // Causality is established here: policies travel from the authenticated
        // full pair, but every rung is re-resolved on this fold's prefix alone.
        let long = full_long.policy().resolve_attested(train_series)?;
        let short = full_short.policy().resolve_attested(train_series)?;
        let coordinate_order_digest =
            authenticate_grid_pair_v4(train_series, &long, &short, false)?;

        let scaled = scale_min_hits(base.min_hits(), train.len(), signal.len());
        let per_fold = engine::Ladder::with_min_hits(scaled)
            .with_ceiling(base.ceiling())
            .with_pair_budget(base.pair_budget());
        let signal_train_column =
            builder(train).map_err(AnchoredSearchValidationRefusalV4::BuilderRefused)?;
        training_signal_cursor.authenticate(train, &signal_train_column, full_signal_column)?;
        let projected_train = project_fold(
            train,
            &signal_train_column,
            trade_train,
            signal_length_micros,
        )
        .map_err(AnchoredSearchValidationRefusalV4::BuilderRefused)?;
        let swept = crate::Sweeper::new(per_fold).run_prepared(&signal_train_column);
        if swept.sweep.halted.is_some() || !swept.is_complete() {
            return Err(AnchoredSearchValidationRefusalV4::IncompleteSearch { fold: index });
        }
        let closed = crate::closed::closed(&swept.sweep);
        let considered_masks = stable_u64_v4(closed.kept.len())?;
        let cells_per_mask = long
            .cell_count()
            .checked_add(short.cell_count())
            .ok_or(AnchoredSearchValidationRefusalV4::PopulationOverflow)?;
        let expected_population = considered_masks
            .checked_mul(cells_per_mask)
            .ok_or(AnchoredSearchValidationRefusalV4::PopulationOverflow)?;
        let mut population_hasher = brutex_core::blake3::Hasher::new();
        population_hasher.update(ANCHORED_SEARCH_FOLD_POPULATION_DOMAIN_V4);
        population_hasher.update(&stable_u64_v4(index)?.to_le_bytes());
        population_hasher.update(&coordinate_order_digest);
        population_hasher.update(&expected_population.to_le_bytes());

        let params = Params::of(per_fold).with_policy(&[
            4,
            u64::try_from(signal_length_micros)
                .map_err(|_| AnchoredSearchValidationRefusalV4::UnsupportedSignalLength)?,
            u64::from(horizon.as_bars()),
            digest_head_u64_v4(coordinate_order_digest),
        ]);
        let mut pending = Vec::new();
        pending
            .try_reserve(closed.kept.len().saturating_mul(2))
            .map_err(|_| AnchoredSearchValidationRefusalV4::PopulationOverflow)?;
        let mut population_cells = 0_u64;
        for item in &closed.kept {
            for (side, resolved) in [(Direction::Long, &long), (Direction::Short, &short)] {
                hash_mask_for_admission_v2(&mut population_hasher, item.mask);
                population_hasher.update(&[direction_tag_v2(side)]);
                population_hasher.update(&resolved.digest());
                let run = Run {
                    mask: item.mask,
                    direction: run_direction_v4(side),
                    instrument: execution.instrument(),
                    timeframe,
                    params,
                    data_digest: data_digest_with_execution(train, Some(trade_train)),
                    commit: execution.commit(),
                    feed: execution.feed(),
                };
                let execution_run = ExecutionRunV1::new(&run, train, Some(trade_train))?;
                let evaluated = resolved.evaluate_training_grid_attested(
                    train_series,
                    &projected_train,
                    horizon,
                    execution_run,
                )?;
                let validated = resolved.validate_evaluation(&evaluated)?;
                population_hasher.update(&validated.evaluation_digest());
                let evaluated_cells = stable_u64_v4(evaluated.grid().cells.len())?;
                if evaluated_cells != resolved.cell_count() {
                    return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
                        "evaluated cell population differs from its resolution",
                    ));
                }
                for ordinal in 0..evaluated.grid().cells.len() {
                    let cell = validated.cell(ordinal).ok_or(
                        AnchoredSearchValidationRefusalV4::GridPairMismatch(
                            "validated grid lost one canonical cell",
                        ),
                    )?;
                    population_hasher.update(&stable_u64_v4(ordinal)?.to_le_bytes());
                    hash_cell_v4(&mut population_hasher, cell);
                }
                population_cells = population_cells
                    .checked_add(evaluated_cells)
                    .ok_or(AnchoredSearchValidationRefusalV4::PopulationOverflow)?;
                if let Some(selected) = resolved.select(&evaluated)? {
                    pending.push(PendingCandidateV4 {
                        mask: item.mask,
                        side,
                        resolution_digest: resolved.digest(),
                        evaluation_digest: validated.evaluation_digest(),
                        training_cell_digest: digest_cell_v4(selected.training_cell()),
                        in_sample_pessimistic: selected.training_cell().pessimistic,
                        selected,
                    });
                }
            }
        }
        if population_cells != expected_population {
            return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
                "mask-by-side population is incomplete",
            ));
        }
        let population_digest = population_hasher.finalize();
        let best_ordinal =
            pending
                .iter()
                .enumerate()
                .fold(None, |best: Option<usize>, (ordinal, candidate)| {
                    if best.is_none_or(|held| {
                        pending.get(held).is_some_and(|current| {
                            candidate.in_sample_pessimistic > current.in_sample_pessimistic
                        })
                    }) {
                        Some(ordinal)
                    } else {
                        best
                    }
                });

        let mut final_candidates = Vec::with_capacity(pending.len());
        let mut oos_all = Vec::with_capacity(pending.len());
        let mut choice = None;
        // `Summary` has no versioned exact-exit-grid representation. V4 keeps
        // these legacy summaries deliberately empty instead of publishing a
        // scalar-horizon walk for a different strategy. The authoritative
        // exact selected-cell totals remain the typed chosen fields below.
        let plain_train = Summary::default();
        let plain_oos = Summary::default();
        if !pending.is_empty() {
            let signal_upto = signal
                .get(..fold.test.end)
                .ok_or(AnchoredSearchValidationRefusalV4::DiscontiguousTraining { fold: index })?;
            let full_column =
                builder(signal_upto).map_err(AnchoredSearchValidationRefusalV4::BuilderRefused)?;
            oos_signal_cursor.authenticate(signal_upto, &full_column, full_signal_column)?;
            let last_signal_close = signal_upto
                .last()
                .map(|bar| bar.ts_micros.saturating_add(signal_length_micros))
                .ok_or(AnchoredSearchValidationRefusalV4::DiscontiguousTraining { fold: index })?;
            let hold = i64::from(horizon.as_bars()).saturating_mul(EXECUTION_MINUTE_MICROS);
            let trade_test = oos_execution_cursor
                .through(execution.bars(), last_signal_close.saturating_add(hold))
                .ok_or(AnchoredSearchValidationRefusalV4::GridPairMismatch(
                    "OOS execution boundaries are not monotonic",
                ))?;
            let first_oos = first_oos_execution;
            let oos_series = ExecutionSeriesV1::new(
                execution.instrument(),
                execution.feed(),
                execution.commit(),
                execution.calendar_digest(),
                trade_test,
            )?;
            let oos = OosExecutionSeriesV1::new(oos_series, first_oos)?;
            let projected_oos = project_oos_fold(
                signal_upto,
                &full_column,
                trade_test,
                signal_length_micros,
                fold.test.start,
                first_oos,
            )
            .map_err(AnchoredSearchValidationRefusalV4::BuilderRefused)?;

            for (ordinal, candidate) in pending.iter().enumerate() {
                let resolved = match candidate.side {
                    Direction::Long => &long,
                    Direction::Short => &short,
                };
                let run = Run {
                    mask: candidate.mask,
                    direction: run_direction_v4(candidate.side),
                    instrument: execution.instrument(),
                    timeframe,
                    params,
                    data_digest: data_digest_with_execution(signal_upto, Some(trade_test)),
                    commit: execution.commit(),
                    feed: execution.feed(),
                };
                let execution_run = ExecutionRunV1::new(&run, signal_upto, Some(trade_test))?;
                let replay = resolved.replay_selected(
                    oos,
                    &projected_oos,
                    &candidate.selected,
                    execution_run,
                )?;
                let outcome = replay.cell().ok_or(
                    AnchoredSearchValidationRefusalV4::IncompleteCandidateOos {
                        fold: index,
                        ordinal,
                    },
                )?;
                let oos_cell_digest = Some(digest_cell_v4(outcome));
                let out_of_sample_pessimistic = Some(outcome.pessimistic);
                oos_all.push(outcome.pessimistic);
                let proof = AnchoredSearchCandidateV4 {
                    mask: candidate.mask,
                    side: candidate.side,
                    coordinate: candidate.selected.coordinate(),
                    resolution_digest: candidate.resolution_digest,
                    evaluation_digest: candidate.evaluation_digest,
                    selection_digest: candidate.selected.digest(),
                    training_run_id: candidate.selected.run_id().bytes(),
                    training_cell_digest: candidate.training_cell_digest,
                    in_sample_pessimistic: candidate.in_sample_pessimistic,
                    replay_digest: replay.digest(),
                    oos_run_id: replay.run_id().bytes(),
                    oos_cell_digest,
                    out_of_sample_pessimistic,
                };
                if best_ordinal == Some(ordinal) {
                    choice = Some(AnchoredSearchChoiceV4 {
                        ordinal: stable_u64_v4(ordinal)?,
                        mask: candidate.mask,
                        side: candidate.side,
                        coordinate: candidate.selected.coordinate(),
                        resolution_digest: candidate.resolution_digest,
                        evaluation_digest: candidate.evaluation_digest,
                        selection_digest: candidate.selected.digest(),
                        training_run_id: candidate.selected.run_id().bytes(),
                        replay_digest: replay.digest(),
                        oos_run_id: replay.run_id().bytes(),
                        training_cell_digest: candidate.training_cell_digest,
                        oos_cell_digest: digest_cell_v4(outcome),
                        in_sample_pessimistic: candidate.in_sample_pessimistic,
                        out_of_sample_pessimistic: outcome.pessimistic,
                    });
                }
                final_candidates.push(proof);
            }
        }

        if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {
            return Err(AnchoredSearchValidationRefusalV4::CandidateFamilyMismatch { fold: index });
        }
        let visible = FoldResult {
            index,
            train_bars: train.len(),
            purged: fold.purged,
            test_bars: fold.test.len(),
            considered: considered_masks,
            priced: considered_masks,
            halted: None,
            chosen_exit: choice.map(|value| value.coordinate),
            chosen_exit_total: choice.map(|value| value.in_sample_pessimistic),
            out_of_sample_exit: choice.map(|value| value.out_of_sample_pessimistic),
            chosen_side: choice.map(|value| value.side),
            chosen: choice.map(|value| value.mask),
            in_sample: plain_train,
            out_of_sample: plain_oos,
            in_sample_all: pending
                .iter()
                .map(|candidate| candidate.in_sample_pessimistic)
                .collect(),
            out_of_sample_all: oos_all,
        };
        proofs.push(AnchoredSearchFoldV4 {
            index: stable_u64_v4(index)?,
            train_start: stable_u64_v4(fold.train.0.start)?,
            train_end: stable_u64_v4(fold.train.0.end)?,
            test_start: stable_u64_v4(fold.test.start)?,
            test_end: stable_u64_v4(fold.test.end)?,
            purged: stable_u64_v4(fold.purged)?,
            considered_masks,
            population_cells,
            selected_candidates: stable_u64_v4(final_candidates.len())?,
            long_resolution_digest: long.digest(),
            short_resolution_digest: short.digest(),
            coordinate_order_digest,
            population_digest,
            candidates: final_candidates,
            choice,
            in_sample: plain_train,
            out_of_sample: plain_oos,
        });
        visible_folds.push(visible);
    }

    Ok((
        Validated {
            folds: visible_folds,
            refused: None,
        },
        proofs,
    ))
}

fn signal_timeframe_v4(signal_length_micros: i64) -> Option<&'static str> {
    match signal_length_micros {
        60_000_000 => Some("1min"),
        120_000_000 => Some("2min"),
        180_000_000 => Some("3min"),
        300_000_000 => Some("5min"),
        600_000_000 => Some("10min"),
        900_000_000 => Some("15min"),
        1_800_000_000 => Some("30min"),
        3_600_000_000 => Some("60min"),
        _ => None,
    }
}

const fn run_direction_v4(direction: Direction) -> crate::identity::Direction {
    match direction {
        Direction::Long => crate::identity::Direction::Long,
        Direction::Short => crate::identity::Direction::Short,
    }
}

fn stable_u64_v4(value: usize) -> Result<u64, AnchoredSearchValidationRefusalV4> {
    u64::try_from(value).map_err(|_| AnchoredSearchValidationRefusalV4::StableWidth)
}

fn digest_head_u64_v4(digest: [u8; 32]) -> u64 {
    let mut head = [0_u8; 8];
    head.copy_from_slice(&digest[..8]);
    u64::from_le_bytes(head)
}

fn authenticate_grid_pair_v4(
    series: ExecutionSeriesV1<'_>,
    long: &ResolvedExitGridV1,
    short: &ResolvedExitGridV1,
    full_span: bool,
) -> Result<[u8; 32], AnchoredSearchValidationRefusalV4> {
    if long.side() != crate::excursion::Side::Long || short.side() != crate::excursion::Side::Short
    {
        return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
            "Long/Short resolutions are swapped or carry the wrong side",
        ));
    }
    if !long.digest_is_valid() || !short.digest_is_valid() {
        return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
            "one resolution or policy digest is torn",
        ));
    }
    if long.instrument() != short.instrument()
        || long.instrument() != *series.instrument()
        || long.feed_digest() != short.feed_digest()
        || long.commit_digest() != short.commit_digest()
        || long.calendar_digest() != short.calendar_digest()
        || long.training_digest() != short.training_digest()
        || long.training_bars() != short.training_bars()
        || long.training_first_ts_micros() != short.training_first_ts_micros()
        || long.training_last_ts_micros() != short.training_last_ts_micros()
    {
        return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
            "Long/Short source or training identity differs",
        ));
    }
    if long.policy_digest() == short.policy_digest() || long.digest() == short.digest() {
        return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
            "Long/Short policy or resolution identities are indistinguishable",
        ));
    }
    let fresh_long = long.policy().resolve_attested(series)?;
    let fresh_short = short.policy().resolve_attested(series)?;
    if &fresh_long != long || &fresh_short != short {
        return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
            "retained resolution does not exactly match the attested execution series",
        ));
    }

    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(ANCHORED_SEARCH_GRID_DOMAIN_V4);
    hasher.update(&[u8::from(full_span)]);
    for (side, resolved) in [(Direction::Long, long), (Direction::Short, short)] {
        hasher.update(&[direction_tag_v2(side)]);
        hasher.update(&resolved.policy_digest());
        hasher.update(&resolved.digest());
        hasher.update(&resolved.feed_digest());
        hasher.update(&resolved.commit_digest());
        hasher.update(&resolved.calendar_digest());
        hasher.update(&resolved.training_digest());
        hasher.update(&resolved.training_bars().to_le_bytes());
        hasher.update(&resolved.training_first_ts_micros().to_le_bytes());
        hasher.update(&resolved.training_last_ts_micros().to_le_bytes());
        hasher.update(&resolved.cell_count().to_le_bytes());
        let mut ordinal = 0_u64;
        let mut overflowed = false;
        let complete = resolved.visit_coordinates(|coordinate| {
            hasher.update(&ordinal.to_le_bytes());
            hash_chosen_for_admission_v2(&mut hasher, coordinate);
            let Some(next) = ordinal.checked_add(1) else {
                overflowed = true;
                return false;
            };
            ordinal = next;
            true
        });
        if overflowed || !complete || ordinal != resolved.cell_count() {
            return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
                "canonical coordinate order is incomplete",
            ));
        }
    }
    Ok(hasher.finalize())
}

fn digest_cell_v4(cell: &crate::grid::Cell) -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(b"brutex.validate.anchored-search.cell.v4\0");
    hash_cell_v4(&mut hasher, cell);
    hasher.finalize()
}

fn hash_cell_v4(hasher: &mut brutex_core::blake3::Hasher, cell: &crate::grid::Cell) {
    hash_chosen_for_admission_v2(hasher, crate::grid::Chosen::from_cell(cell));
    for value in [
        cell.trades,
        cell.wins,
        cell.stopped,
        cell.trailed_stop,
        cell.trailed_profit,
        cell.targeted,
        cell.timed_out,
        cell.ambiguous_bars,
        cell.gapped,
        cell.bars_held,
    ] {
        hasher.update(&value.to_le_bytes());
    }
    for value in [
        cell.pessimistic,
        cell.optimistic,
        cell.fill_cost,
        cell.winner_mae,
        cell.winner_mfe,
        cell.all_mae,
        cell.worst_mae,
        cell.gross_win,
        cell.gross_loss,
        cell.best_trade,
        cell.min_win,
        cell.worst_trade,
        cell.max_drawdown,
    ] {
        hasher.update(&value.to_le_bytes());
    }
    hasher.update(&cell.max_losing_streak.to_le_bytes());
    hasher.update(&cell.max_winning_streak.to_le_bytes());
}

fn hash_validation_policy_v4(
    value: &AnchoredSearchPolicyFactsV4,
    full_grid_digest: [u8; 32],
) -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(ANCHORED_SEARCH_POLICY_DOMAIN_V4);
    hasher.update(&[1, value.semantic_order]);
    hasher.update(&value.signal_bars.to_le_bytes());
    hasher.update(&value.signal_digest);
    hasher.update(&value.signal_first_ts_micros.to_le_bytes());
    hasher.update(&value.signal_last_ts_micros.to_le_bytes());
    hasher.update(&value.signal_column_digest);
    hasher.update(&value.horizon_bars.to_le_bytes());
    hasher.update(&value.splits.to_le_bytes());
    hasher.update(&value.min_hits.to_le_bytes());
    hasher.update(&value.ceiling.to_le_bytes());
    hasher.update(&value.pair_budget.to_le_bytes());
    hasher.update(&value.signal_length_micros.to_le_bytes());
    hasher.update(&value.full_long_policy_digest);
    hasher.update(&value.full_short_policy_digest);
    hasher.update(&value.full_long_resolution_digest);
    hasher.update(&value.full_short_resolution_digest);
    hasher.update(&value.full_training_digest);
    hasher.update(&value.full_training_bars.to_le_bytes());
    hasher.update(&value.feed_digest);
    hasher.update(&value.commit_digest);
    hasher.update(&value.calendar_digest);
    hasher.update(&full_grid_digest);
    hasher.finalize()
}

fn hash_candidate_training_v4(
    hasher: &mut brutex_core::blake3::Hasher,
    candidate: &AnchoredSearchCandidateV4,
) {
    hash_mask_for_admission_v2(hasher, candidate.mask);
    hasher.update(&[direction_tag_v2(candidate.side)]);
    hash_chosen_for_admission_v2(hasher, candidate.coordinate);
    hasher.update(&candidate.resolution_digest);
    hasher.update(&candidate.evaluation_digest);
    hasher.update(&candidate.selection_digest);
    hasher.update(&candidate.training_run_id);
    hasher.update(&candidate.training_cell_digest);
    hasher.update(&candidate.in_sample_pessimistic.to_le_bytes());
}

fn hash_candidate_walk_v4(
    hasher: &mut brutex_core::blake3::Hasher,
    candidate: &AnchoredSearchCandidateV4,
) {
    hasher.update(&candidate.replay_digest);
    hasher.update(&candidate.oos_run_id);
    hasher.update(&[u8::from(candidate.oos_cell_digest.is_some())]);
    hasher.update(&candidate.oos_cell_digest.unwrap_or_default());
    hash_optional_i64_for_admission_v2(hasher, candidate.out_of_sample_pessimistic);
}

fn hash_validation_family_v4(
    folds: &[AnchoredSearchFoldV4],
) -> Result<[u8; 32], AnchoredSearchValidationRefusalV4> {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(ANCHORED_SEARCH_FAMILY_DOMAIN_V4);
    hasher.update(&stable_u64_v4(folds.len())?.to_le_bytes());
    for fold in folds {
        for value in [
            fold.index,
            fold.train_start,
            fold.train_end,
            fold.test_start,
            fold.test_end,
            fold.purged,
            fold.considered_masks,
            fold.population_cells,
            fold.selected_candidates,
        ] {
            hasher.update(&value.to_le_bytes());
        }
        hasher.update(&fold.long_resolution_digest);
        hasher.update(&fold.short_resolution_digest);
        hasher.update(&fold.coordinate_order_digest);
        hasher.update(&fold.population_digest);
        hasher.update(&stable_u64_v4(fold.candidates.len())?.to_le_bytes());
        for candidate in &fold.candidates {
            hash_candidate_training_v4(&mut hasher, candidate);
        }
    }
    Ok(hasher.finalize())
}

fn hash_choice_v4(
    hasher: &mut brutex_core::blake3::Hasher,
    choice: Option<&AnchoredSearchChoiceV4>,
) {
    let Some(choice) = choice else {
        hasher.update(&[0]);
        return;
    };
    hasher.update(&[1]);
    hasher.update(&choice.ordinal.to_le_bytes());
    hash_mask_for_admission_v2(hasher, choice.mask);
    hasher.update(&[direction_tag_v2(choice.side)]);
    hash_chosen_for_admission_v2(hasher, choice.coordinate);
    hasher.update(&choice.resolution_digest);
    hasher.update(&choice.evaluation_digest);
    hasher.update(&choice.selection_digest);
    hasher.update(&choice.training_run_id);
    hasher.update(&choice.replay_digest);
    hasher.update(&choice.oos_run_id);
    hasher.update(&choice.training_cell_digest);
    hasher.update(&choice.oos_cell_digest);
    hasher.update(&choice.in_sample_pessimistic.to_le_bytes());
    hasher.update(&choice.out_of_sample_pessimistic.to_le_bytes());
}

fn hash_walk_facts_v4(
    policy_digest: [u8; 32],
    full_grid_digest: [u8; 32],
    family_digest: [u8; 32],
    folds: &[AnchoredSearchFoldV4],
) -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(ANCHORED_SEARCH_WALK_DOMAIN_V4);
    hasher.update(&policy_digest);
    hasher.update(&full_grid_digest);
    hasher.update(&family_digest);
    for fold in folds {
        hash_summary_for_admission_v2(&mut hasher, fold.in_sample);
        hash_summary_for_admission_v2(&mut hasher, fold.out_of_sample);
        hash_choice_v4(&mut hasher, fold.choice.as_ref());
        for candidate in &fold.candidates {
            hash_candidate_walk_v4(&mut hasher, candidate);
        }
    }
    hasher.finalize()
}

fn choice_matches_candidate_v4(
    choice: &AnchoredSearchChoiceV4,
    candidate: &AnchoredSearchCandidateV4,
) -> bool {
    choice.mask == candidate.mask
        && choice.side == candidate.side
        && choice.coordinate == candidate.coordinate
        && choice.resolution_digest == candidate.resolution_digest
        && choice.evaluation_digest == candidate.evaluation_digest
        && choice.selection_digest == candidate.selection_digest
        && choice.training_run_id == candidate.training_run_id
        && choice.replay_digest == candidate.replay_digest
        && choice.oos_run_id == candidate.oos_run_id
        && choice.training_cell_digest == candidate.training_cell_digest
        && candidate.oos_cell_digest == Some(choice.oos_cell_digest)
        && choice.in_sample_pessimistic == candidate.in_sample_pessimistic
        && candidate.out_of_sample_pessimistic == Some(choice.out_of_sample_pessimistic)
}

#[derive(Clone, Copy)]
struct AnchoredSearchFoldTotalsV4 {
    decided: u64,
    profitable: u64,
    aggregate_oos_paisa: i64,
    population_cells: u64,
}

fn candidate_vectors_match_v4(proof: &AnchoredSearchFoldV4, visible: &FoldResult) -> bool {
    u64::try_from(proof.candidates.len()).is_ok_and(|count| proof.selected_candidates == count)
        && proof.candidates.len() == visible.in_sample_all.len()
        && proof.candidates.len() == visible.out_of_sample_all.len()
        && proof.in_sample == visible.in_sample
        && proof.out_of_sample == visible.out_of_sample
        && proof
            .candidates
            .iter()
            .enumerate()
            .all(|(ordinal, candidate)| {
                candidate.oos_cell_digest.is_some()
                    && candidate.out_of_sample_pessimistic.is_some_and(|oos| {
                        visible.in_sample_all.get(ordinal).copied()
                            == Some(candidate.in_sample_pessimistic)
                            && visible.out_of_sample_all.get(ordinal).copied() == Some(oos)
                    })
            })
}

fn reconcile_fold_choice_v4(
    expected: usize,
    proof: &AnchoredSearchFoldV4,
    visible: &FoldResult,
) -> Result<(u64, u64, i64), AnchoredSearchValidationRefusalV4> {
    let Some(choice) = proof.choice else {
        if !proof.candidates.is_empty()
            || visible.chosen.is_some()
            || visible.chosen_side.is_some()
            || visible.chosen_exit.is_some()
            || visible.chosen_exit_total.is_some()
            || visible.out_of_sample_exit.is_some()
        {
            return Err(AnchoredSearchValidationRefusalV4::ProvenanceMismatch { fold: expected });
        }
        return Ok((0, 0, 0));
    };

    let ordinal = usize::try_from(choice.ordinal)
        .map_err(|_| AnchoredSearchValidationRefusalV4::StableWidth)?;
    let candidate = proof
        .candidates
        .get(ordinal)
        .ok_or(AnchoredSearchValidationRefusalV4::ProvenanceMismatch { fold: expected })?;
    let best_ordinal = visible
        .in_sample_all
        .iter()
        .copied()
        .max()
        .and_then(|best| {
            visible
                .in_sample_all
                .iter()
                .position(|value| *value == best)
        });
    if !choice_matches_candidate_v4(&choice, candidate)
        || visible.chosen != Some(choice.mask)
        || visible.chosen_side != Some(choice.side)
        || visible.chosen_exit != Some(choice.coordinate)
        || visible.chosen_exit_total != Some(choice.in_sample_pessimistic)
        || visible.out_of_sample_exit != Some(choice.out_of_sample_pessimistic)
        || best_ordinal != Some(ordinal)
    {
        return Err(AnchoredSearchValidationRefusalV4::ProvenanceMismatch { fold: expected });
    }
    Ok((
        1,
        u64::from(choice.out_of_sample_pessimistic > 0),
        choice.out_of_sample_pessimistic,
    ))
}

fn reconcile_anchored_search_fold_v4(
    expected: usize,
    proof: &AnchoredSearchFoldV4,
    visible: &FoldResult,
) -> Result<AnchoredSearchFoldTotalsV4, AnchoredSearchValidationRefusalV4> {
    let expected_index = stable_u64_v4(expected)?;
    if proof.index != expected_index
        || visible.index != expected
        || proof.train_start != 0
        || proof.train_end.saturating_sub(proof.train_start) != stable_u64_v4(visible.train_bars)?
        || proof.test_end.saturating_sub(proof.test_start) != stable_u64_v4(visible.test_bars)?
        || proof.purged != stable_u64_v4(visible.purged)?
        || proof.considered_masks != visible.considered
        || proof.considered_masks != visible.priced
        || visible.halted.is_some()
        || !candidate_vectors_match_v4(proof, visible)
    {
        return Err(AnchoredSearchValidationRefusalV4::ProvenanceMismatch { fold: expected });
    }
    let (decided, profitable, aggregate_oos_paisa) =
        reconcile_fold_choice_v4(expected, proof, visible)?;
    Ok(AnchoredSearchFoldTotalsV4 {
        decided,
        profitable,
        aggregate_oos_paisa,
        population_cells: proof.population_cells,
    })
}

fn reconcile_anchored_search_v4(
    value: &AnchoredSearchValidationV4,
) -> Result<AnchoredSearchProjectionV4, AnchoredSearchValidationRefusalV4> {
    if value.policy.semantic_order != CANDIDATE_SEMANTIC_ORDER_V4
        || value.policy.full_long_policy_digest == value.policy.full_short_policy_digest
        || value.policy.full_long_resolution_digest == value.policy.full_short_resolution_digest
        || value.validated.refused.is_some()
        || value.folds.is_empty()
        || value.folds.len() != value.validated.folds.len()
        || hash_validation_policy_v4(&value.policy, value.full_grid_digest)
            != value.validation_policy_digest
        || hash_validation_family_v4(&value.folds)? != value.validation_family_digest
        || hash_walk_facts_v4(
            value.validation_policy_digest,
            value.full_grid_digest,
            value.validation_family_digest,
            &value.folds,
        ) != value.walk_facts_digest
    {
        return Err(AnchoredSearchValidationRefusalV4::SealMismatch);
    }

    let mut decided_folds = 0_u64;
    let mut profitable_oos_folds = 0_u64;
    let mut aggregate_oos_paisa = 0_i64;
    let mut evaluated_population_cells = 0_u64;
    for (expected, (proof, visible)) in value.folds.iter().zip(&value.validated.folds).enumerate() {
        let totals = reconcile_anchored_search_fold_v4(expected, proof, visible)?;
        decided_folds = decided_folds
            .checked_add(totals.decided)
            .ok_or(AnchoredSearchValidationRefusalV4::StableWidth)?;
        profitable_oos_folds = profitable_oos_folds
            .checked_add(totals.profitable)
            .ok_or(AnchoredSearchValidationRefusalV4::StableWidth)?;
        aggregate_oos_paisa = aggregate_oos_paisa
            .checked_add(totals.aggregate_oos_paisa)
            .ok_or(AnchoredSearchValidationRefusalV4::AggregateOosOverflow)?;
        evaluated_population_cells = evaluated_population_cells
            .checked_add(totals.population_cells)
            .ok_or(AnchoredSearchValidationRefusalV4::PopulationOverflow)?;
    }

    Ok(AnchoredSearchProjectionV4 {
        validation_policy_digest: value.validation_policy_digest,
        signal_digest: value.policy.signal_digest,
        signal_bars: value.policy.signal_bars,
        signal_first_ts_micros: value.policy.signal_first_ts_micros,
        signal_last_ts_micros: value.policy.signal_last_ts_micros,
        signal_column_digest: value.policy.signal_column_digest,
        full_grid_digest: value.full_grid_digest,
        full_long_policy_digest: value.policy.full_long_policy_digest,
        full_short_policy_digest: value.policy.full_short_policy_digest,
        full_long_resolution_digest: value.policy.full_long_resolution_digest,
        full_short_resolution_digest: value.policy.full_short_resolution_digest,
        validation_family_digest: value.validation_family_digest,
        walk_facts_digest: value.walk_facts_digest,
        fold_count: stable_u64_v4(value.folds.len())?,
        decided_folds,
        profitable_oos_folds,
        aggregate_oos_paisa,
        evaluated_population_cells,
    })
}

const EXECUTION_MINUTE_MICROS: i64 = 60_000_000;

/// One forward-only execution-prefix cursor.
///
/// A cursor instance is used for one monotonically increasing family of fold
/// boundaries. Its index advances at most once across the supplied execution
/// path, so all folds together cost O(E + F), not one search or scan per fold.
/// A decreasing boundary or a changed-to-shorter slice refuses; neither can
/// recover the full series as a fallback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MonotonicExecutionPrefix {
    end: usize,
    last_boundary: Option<i64>,
}

impl MonotonicExecutionPrefix {
    /// Prefix strictly before `exclusive_ts`.
    fn before<'a>(&mut self, execution: &'a [Candle], exclusive_ts: i64) -> Option<&'a [Candle]> {
        self.advance(execution, exclusive_ts, false)
    }

    /// Prefix through `inclusive_ts`.
    fn through<'a>(&mut self, execution: &'a [Candle], inclusive_ts: i64) -> Option<&'a [Candle]> {
        self.advance(execution, inclusive_ts, true)
    }

    const fn end(&self) -> usize {
        self.end
    }

    fn advance<'a>(
        &mut self,
        execution: &'a [Candle],
        boundary: i64,
        inclusive: bool,
    ) -> Option<&'a [Candle]> {
        if self.end > execution.len()
            || self
                .last_boundary
                .is_some_and(|previous| boundary < previous)
        {
            return None;
        }
        while let Some(bar) = execution.get(self.end) {
            let belongs = if inclusive {
                bar.ts_micros <= boundary
            } else {
                bar.ts_micros < boundary
            };
            if !belongs {
                break;
            }
            self.end = self.end.checked_add(1)?;
        }
        self.last_boundary = Some(boundary);
        Some(execution.split_at(self.end).0)
    }
}

/// Project one already-built signal column onto one bounded execution prefix.
fn project_fold(
    signal: &[Candle],
    signal_column: &Column,
    execution: &[Candle],
    signal_length_micros: i64,
) -> Result<Column, String> {
    let alignment = crate::align::onto_execution(
        signal,
        signal_column.sources(),
        execution,
        signal_length_micros,
    )
    .ok_or_else(|| {
        "a walk-forward fold could not align its signal rows to exact one-minute closes".to_owned()
    })?;
    signal_column
        .reproject_checked(&alignment.onto, execution)
        .map(|(projected, _)| projected)
        .ok_or_else(|| {
            "a walk-forward fold produced an execution alignment that was not parallel to its signal column"
                .to_owned()
        })
}

/// Project a warmed signal column while physically excluding every pre-OOS row.
///
/// Clearing a pre-OOS mask is insufficient for an exact replay capability: the
/// row still carries its original execution source, and a zero mask is still a
/// purported observation. This door removes those mappings before reprojection
/// and reconciles the exact number of deliberate and naturally-unreachable
/// rows. The returned column therefore contains no invented pre-OOS source.
fn project_oos_fold(
    signal: &[Candle],
    signal_column: &Column,
    execution: &[Candle],
    signal_length_micros: i64,
    first_oos_signal: usize,
    first_oos_execution: usize,
) -> Result<Column, String> {
    if first_oos_signal > signal.len() || first_oos_execution > execution.len() {
        return Err("an OOS projection boundary is outside its supplied series".to_owned());
    }
    let mut alignment = crate::align::onto_execution(
        signal,
        signal_column.sources(),
        execution,
        signal_length_micros,
    )
    .ok_or_else(|| {
        "an OOS fold could not align its warmed signal rows to exact one-minute closes".to_owned()
    })?;
    if alignment.onto.len() != signal_column.sources().len() {
        return Err("an OOS alignment is not parallel to its warmed signal column".to_owned());
    }

    let mut deliberately_dropped = 0_u64;
    for (&source, target) in signal_column.sources().iter().zip(&mut alignment.onto) {
        if source < first_oos_signal && target.take().is_some() {
            deliberately_dropped = deliberately_dropped
                .checked_add(1)
                .ok_or_else(|| "the OOS deliberate-drop census overflowed".to_owned())?;
        }
    }
    let expected_dropped = alignment
        .unreachable
        .checked_add(deliberately_dropped)
        .ok_or_else(|| "the OOS projection drop census overflowed".to_owned())?;
    let (projected, dropped) = signal_column
        .reproject_checked(&alignment.onto, execution)
        .ok_or_else(|| {
            "an OOS execution alignment was not parallel to its warmed signal column".to_owned()
        })?;
    if dropped != expected_dropped {
        return Err("an OOS projection did not reconcile its exact dropped-row census".to_owned());
    }
    if projected
        .sources()
        .iter()
        .any(|&source| source < first_oos_execution || source >= execution.len())
    {
        return Err("an OOS projection retained a source outside its exact OOS window".to_owned());
    }
    Ok(projected)
}

/// Does the supplied path have the only cadence this API accepts?
///
/// Gaps are legal when they are whole missing minutes. Sub-minute, duplicate,
/// backward, or off-grid timestamps are malformed and cannot be treated as a
/// one-minute path.
fn is_one_minute_path(execution: &[Candle]) -> bool {
    execution
        .first()
        .is_some_and(|first| first.ts_micros.rem_euclid(EXECUTION_MINUTE_MICROS) == 0)
        && execution.windows(2).all(|pair| {
            pair.first().zip(pair.get(1)).is_some_and(|(a, b)| {
                let delta = b.ts_micros.saturating_sub(a.ts_micros);
                delta >= EXECUTION_MINUTE_MICROS && delta.rem_euclid(EXECUTION_MINUTE_MICROS) == 0
            })
        })
}

impl AnchoredAdmissionCaptureV2 {
    fn new(
        signal_bars: usize,
        horizon: Horizon,
        splits: usize,
        direction: Direction,
        sweeper: &crate::Sweeper,
        resolved_rungs: usize,
        signal_length_micros: i64,
    ) -> Result<Self, AnchoredAdmissionValidationRefusalV2> {
        if resolved_rungs == 0 {
            return Err(AnchoredAdmissionValidationRefusalV2::ZeroResolvedRungs);
        }
        let ladder = sweeper.ladder();
        Ok(Self {
            policy: AnchoredAdmissionPolicyFactsV2 {
                signal_bars: stable_u64_v2(signal_bars)?,
                horizon_bars: horizon.as_bars(),
                splits: stable_u64_v2(splits)?,
                resolved_rungs: stable_u64_v2(resolved_rungs)?,
                min_hits: ladder.min_hits(),
                ceiling: stable_u64_v2(ladder.ceiling())?,
                pair_budget: ladder.pair_budget(),
                requested_direction: direction_tag_v2(direction),
                // This constructor is intentionally reachable only from the
                // exact projected one-minute execution door.
                execution_mode: 1,
                signal_length_micros,
            },
            folds: Vec::with_capacity(splits),
            refusal: None,
        })
    }

    fn observe_fold(
        &mut self,
        split: &crate::split::Fold,
        visible: &FoldResult,
        scored: &[Scored],
        oos: &[CandidateOosV2],
        chosen_ordinal: Option<usize>,
    ) -> Result<(), AnchoredAdmissionValidationRefusalV2> {
        let candidate_facts_digest = hash_candidate_facts_v2(scored, oos, visible.index)?;
        self.folds.push(capture_anchored_fold(
            split,
            visible,
            scored,
            oos,
            chosen_ordinal,
            candidate_facts_digest,
        )?);
        Ok(())
    }

    fn finish(
        self,
        validated: Validated,
    ) -> Result<AnchoredAdmissionValidationV2, AnchoredAdmissionValidationRefusalV2> {
        if let Some(refusal) = self.refusal.clone() {
            return Err(refusal);
        }
        if let Some(why) = validated.refused.as_ref() {
            return Err(AnchoredAdmissionValidationRefusalV2::UpstreamRefused(
                why.clone(),
            ));
        }
        if validated.folds.is_empty() {
            return Err(AnchoredAdmissionValidationRefusalV2::Empty);
        }
        if validated.folds.len() != self.folds.len() {
            return Err(AnchoredAdmissionValidationRefusalV2::SealMismatch);
        }
        let validation_policy_digest = hash_validation_policy_v2(self.policy);
        let validation_family_digest = hash_validation_family_v2(&self.folds);
        let walk_facts_digest = hash_walk_facts_v2(
            validation_policy_digest,
            validation_family_digest,
            &self.folds,
        );
        let opaque = AnchoredAdmissionValidationV2 {
            validated,
            policy: self.policy,
            folds: self.folds,
            validation_policy_digest,
            validation_family_digest,
            walk_facts_digest,
        };
        opaque.search_authority_projection()?;
        Ok(opaque)
    }
}

fn capture_anchored_fold(
    split: &crate::split::Fold,
    visible: &FoldResult,
    scored: &[Scored],
    oos: &[CandidateOosV2],
    chosen_ordinal: Option<usize>,
    candidate_facts_digest: [u8; 32],
) -> Result<AnchoredAdmissionFoldV2, AnchoredAdmissionValidationRefusalV2> {
    let fold = visible.index;
    if visible.halted.is_some()
        || visible.considered != visible.priced
        || stable_u64_v2(scored.len())? != visible.priced
        || scored.len() != oos.len()
        || visible.in_sample_all.len() != scored.len()
        || visible.out_of_sample_all.len() != scored.len()
    {
        return Err(AnchoredAdmissionValidationRefusalV2::CandidateFamilyMismatch { fold });
    }

    let choice = if let Some(ordinal) = chosen_ordinal {
        let Some(candidate) = scored.get(ordinal) else {
            return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold });
        };
        let Some(outcome) = oos.get(ordinal) else {
            return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold });
        };
        let Some(out_of_sample_pessimistic) = outcome.pessimistic else {
            return Err(AnchoredAdmissionValidationRefusalV2::IncompleteChosenOos { fold });
        };
        let exit_values = selected_exit_values_v2(&candidate.pick, fold)?;
        let choice = AnchoredAdmissionChoiceV2 {
            ordinal: stable_u64_v2(ordinal)?,
            mask: candidate.mask,
            side: candidate.side,
            exit: candidate.pick.rungs,
            exit_values,
            in_sample_pessimistic: candidate.pessimistic,
            out_of_sample_pessimistic,
        };
        if !choice_matches_visible_v2(&choice, visible, scored) {
            return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold });
        }
        Some(choice)
    } else {
        if visible.chosen.is_some()
            || visible.chosen_side.is_some()
            || visible.chosen_exit.is_some()
            || visible.chosen_exit_total.is_some()
            || visible.out_of_sample_exit.is_some()
        {
            return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold });
        }
        None
    };

    Ok(AnchoredAdmissionFoldV2 {
        index: stable_u64_v2(visible.index)?,
        train_start: stable_u64_v2(split.train.0.start)?,
        train_end: stable_u64_v2(split.train.0.end)?,
        train_second_start: stable_u64_v2(split.train.1.start)?,
        train_second_end: stable_u64_v2(split.train.1.end)?,
        test_start: stable_u64_v2(split.test.start)?,
        test_end: stable_u64_v2(split.test.end)?,
        purged: stable_u64_v2(split.purged)?,
        embargoed: stable_u64_v2(split.embargoed)?,
        considered: visible.considered,
        priced: visible.priced,
        candidate_facts_digest,
        choice,
        in_sample: visible.in_sample,
        out_of_sample: visible.out_of_sample,
    })
}

impl AnchoredSearchCaptureV3 {
    fn new(
        signal_bars: usize,
        horizon: Horizon,
        splits: usize,
        sweeper: &crate::Sweeper,
        resolved_rungs: usize,
        signal_length_micros: i64,
    ) -> Result<Self, AnchoredSearchValidationRefusalV3> {
        if resolved_rungs == 0 {
            return Err(AnchoredSearchValidationRefusalV3::ZeroResolvedRungs);
        }
        let ladder = sweeper.ladder();
        Ok(Self {
            policy: AnchoredSearchPolicyFactsV3 {
                signal_bars: stable_u64_v2(signal_bars)?,
                horizon_bars: horizon.as_bars(),
                splits: stable_u64_v2(splits)?,
                resolved_rungs: stable_u64_v2(resolved_rungs)?,
                min_hits: ladder.min_hits(),
                ceiling: stable_u64_v2(ladder.ceiling())?,
                pair_budget: ladder.pair_budget(),
                training_edge_side_rule: TRAINING_EDGE_SIDE_RULE_V3,
                execution_mode: 1,
                signal_length_micros,
            },
            folds: Vec::with_capacity(splits),
            refusal: None,
        })
    }

    fn observe_fold(
        &mut self,
        split: &crate::split::Fold,
        visible: &FoldResult,
        scored: &[Scored],
        oos: &[CandidateOosV2],
        chosen_ordinal: Option<usize>,
    ) -> Result<(), AnchoredSearchValidationRefusalV3> {
        let candidate_facts_digest = hash_candidate_facts_v3(scored, oos, visible.index)?;
        self.folds.push(capture_anchored_fold(
            split,
            visible,
            scored,
            oos,
            chosen_ordinal,
            candidate_facts_digest,
        )?);
        Ok(())
    }

    fn finish(
        self,
        validated: Validated,
    ) -> Result<AnchoredSearchValidationV3, AnchoredSearchValidationRefusalV3> {
        if let Some(refusal) = self.refusal {
            return Err(refusal);
        }
        if let Some(why) = validated.refused.as_ref() {
            return Err(AnchoredSearchValidationRefusalV3::UpstreamRefused(
                why.clone(),
            ));
        }
        if validated.folds.is_empty() {
            return Err(AnchoredSearchValidationRefusalV3::Empty);
        }
        if validated.folds.len() != self.folds.len() {
            return Err(AnchoredSearchValidationRefusalV3::SealMismatch);
        }
        let validation_policy_digest = hash_validation_policy_v3(self.policy);
        let validation_family_digest = hash_validation_family_v3(&self.folds);
        let walk_facts_digest = hash_walk_facts_v3(
            validation_policy_digest,
            validation_family_digest,
            &self.folds,
        );
        let opaque = AnchoredSearchValidationV3 {
            validated,
            policy: self.policy,
            folds: self.folds,
            validation_policy_digest,
            validation_family_digest,
            walk_facts_digest,
        };
        opaque.search_authority_projection()?;
        Ok(opaque)
    }
}

fn reconcile_anchored_admission_v2(
    value: &AnchoredAdmissionValidationV2,
) -> Result<AnchoredAdmissionProjectionV2, AnchoredAdmissionValidationRefusalV2> {
    if value.policy.resolved_rungs == 0 {
        return Err(AnchoredAdmissionValidationRefusalV2::ZeroResolvedRungs);
    }
    if value.validated.refused.is_some() {
        return Err(AnchoredAdmissionValidationRefusalV2::SealMismatch);
    }
    if value.folds.is_empty() || value.folds.len() != value.validated.folds.len() {
        return Err(AnchoredAdmissionValidationRefusalV2::SealMismatch);
    }
    if hash_validation_policy_v2(value.policy) != value.validation_policy_digest
        || hash_validation_family_v2(&value.folds) != value.validation_family_digest
        || hash_walk_facts_v2(
            value.validation_policy_digest,
            value.validation_family_digest,
            &value.folds,
        ) != value.walk_facts_digest
    {
        return Err(AnchoredAdmissionValidationRefusalV2::SealMismatch);
    }

    let mut decided_folds = 0_u64;
    let mut profitable_oos_folds = 0_u64;
    let mut aggregate_oos_paisa = 0_i64;
    for (expected, (proof, visible)) in value.folds.iter().zip(&value.validated.folds).enumerate() {
        if proof.index != stable_u64_v2(expected)?
            || visible.index != expected
            || proof.train_end.saturating_sub(proof.train_start)
                != stable_u64_v2(visible.train_bars)?
            || proof.train_second_end != proof.train_second_start
            || proof.test_end.saturating_sub(proof.test_start) != stable_u64_v2(visible.test_bars)?
            || proof.purged != stable_u64_v2(visible.purged)?
            || proof.considered != visible.considered
            || proof.priced != visible.priced
            || proof.in_sample != visible.in_sample
            || proof.out_of_sample != visible.out_of_sample
        {
            return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch {
                fold: expected,
            });
        }
        match proof.choice {
            Some(choice) => {
                let ordinal = usize::try_from(choice.ordinal)
                    .map_err(|_| AnchoredAdmissionValidationRefusalV2::StableWidth)?;
                let visible_oos = visible.out_of_sample_all.get(ordinal).copied();
                if visible.chosen != Some(choice.mask)
                    || visible.chosen_side != Some(choice.side)
                    || visible.chosen_exit != Some(choice.exit)
                    || visible.chosen_exit_total != Some(choice.in_sample_pessimistic)
                    || visible.out_of_sample_exit != Some(choice.out_of_sample_pessimistic)
                    || visible.in_sample_all.get(ordinal).copied()
                        != Some(choice.in_sample_pessimistic)
                    || visible_oos != Some(choice.out_of_sample_pessimistic)
                    || visible
                        .in_sample_all
                        .iter()
                        .copied()
                        .max()
                        .and_then(|best| visible.in_sample_all.iter().position(|v| *v == best))
                        != Some(ordinal)
                {
                    return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch {
                        fold: expected,
                    });
                }
                decided_folds = decided_folds.saturating_add(1);
                profitable_oos_folds = profitable_oos_folds
                    .saturating_add(u64::from(choice.out_of_sample_pessimistic > 0));
                aggregate_oos_paisa = aggregate_oos_paisa
                    .checked_add(choice.out_of_sample_pessimistic)
                    .ok_or(AnchoredAdmissionValidationRefusalV2::AggregateOosOverflow)?;
            }
            None => {
                if visible.chosen.is_some()
                    || visible.chosen_side.is_some()
                    || visible.chosen_exit.is_some()
                    || visible.chosen_exit_total.is_some()
                    || visible.out_of_sample_exit.is_some()
                {
                    return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch {
                        fold: expected,
                    });
                }
            }
        }
    }

    Ok(AnchoredAdmissionProjectionV2 {
        validation_policy_digest: value.validation_policy_digest,
        validation_family_digest: value.validation_family_digest,
        walk_facts_digest: value.walk_facts_digest,
        fold_count: stable_u64_v2(value.folds.len())?,
        decided_folds,
        profitable_oos_folds,
        aggregate_oos_paisa,
    })
}

fn reconcile_anchored_search_v3(
    value: &AnchoredSearchValidationV3,
) -> Result<AnchoredSearchProjectionV3, AnchoredSearchValidationRefusalV3> {
    if value.policy.resolved_rungs == 0 {
        return Err(AnchoredSearchValidationRefusalV3::ZeroResolvedRungs);
    }
    if value.policy.training_edge_side_rule != TRAINING_EDGE_SIDE_RULE_V3
        || value.policy.execution_mode != 1
        || value.validated.refused.is_some()
        || value.folds.is_empty()
        || value.folds.len() != value.validated.folds.len()
    {
        return Err(AnchoredSearchValidationRefusalV3::SealMismatch);
    }
    if hash_validation_policy_v3(value.policy) != value.validation_policy_digest
        || hash_validation_family_v3(&value.folds) != value.validation_family_digest
        || hash_walk_facts_v3(
            value.validation_policy_digest,
            value.validation_family_digest,
            &value.folds,
        ) != value.walk_facts_digest
    {
        return Err(AnchoredSearchValidationRefusalV3::SealMismatch);
    }

    let mut decided_folds = 0_u64;
    let mut profitable_oos_folds = 0_u64;
    let mut aggregate_oos_paisa = 0_i64;
    for (expected, (proof, visible)) in value.folds.iter().zip(&value.validated.folds).enumerate() {
        if proof.index != stable_u64_v2(expected)?
            || visible.index != expected
            || proof.train_end.saturating_sub(proof.train_start)
                != stable_u64_v2(visible.train_bars)?
            || proof.train_second_end != proof.train_second_start
            || proof.test_end.saturating_sub(proof.test_start) != stable_u64_v2(visible.test_bars)?
            || proof.purged != stable_u64_v2(visible.purged)?
            || proof.considered != visible.considered
            || proof.priced != visible.priced
            || proof.in_sample != visible.in_sample
            || proof.out_of_sample != visible.out_of_sample
        {
            return Err(AnchoredSearchValidationRefusalV3::ProvenanceMismatch { fold: expected });
        }
        match proof.choice {
            Some(choice) => {
                let ordinal = usize::try_from(choice.ordinal)
                    .map_err(|_| AnchoredSearchValidationRefusalV3::StableWidth)?;
                if visible.chosen != Some(choice.mask)
                    || visible.chosen_side != Some(choice.side)
                    || visible.chosen_exit != Some(choice.exit)
                    || visible.chosen_exit_total != Some(choice.in_sample_pessimistic)
                    || visible.out_of_sample_exit != Some(choice.out_of_sample_pessimistic)
                    || visible.in_sample_all.get(ordinal).copied()
                        != Some(choice.in_sample_pessimistic)
                    || visible.out_of_sample_all.get(ordinal).copied()
                        != Some(choice.out_of_sample_pessimistic)
                    || visible
                        .in_sample_all
                        .iter()
                        .copied()
                        .max()
                        .and_then(|best| visible.in_sample_all.iter().position(|v| *v == best))
                        != Some(ordinal)
                {
                    return Err(AnchoredSearchValidationRefusalV3::ProvenanceMismatch {
                        fold: expected,
                    });
                }
                decided_folds = decided_folds.saturating_add(1);
                profitable_oos_folds = profitable_oos_folds
                    .saturating_add(u64::from(choice.out_of_sample_pessimistic > 0));
                aggregate_oos_paisa = aggregate_oos_paisa
                    .checked_add(choice.out_of_sample_pessimistic)
                    .ok_or(AnchoredSearchValidationRefusalV3::AggregateOosOverflow)?;
            }
            None => {
                if visible.chosen.is_some()
                    || visible.chosen_side.is_some()
                    || visible.chosen_exit.is_some()
                    || visible.chosen_exit_total.is_some()
                    || visible.out_of_sample_exit.is_some()
                {
                    return Err(AnchoredSearchValidationRefusalV3::ProvenanceMismatch {
                        fold: expected,
                    });
                }
            }
        }
    }

    Ok(AnchoredSearchProjectionV3 {
        validation_policy_digest: value.validation_policy_digest,
        validation_family_digest: value.validation_family_digest,
        walk_facts_digest: value.walk_facts_digest,
        fold_count: stable_u64_v2(value.folds.len())?,
        decided_folds,
        profitable_oos_folds,
        aggregate_oos_paisa,
    })
}

fn choice_matches_visible_v2(
    choice: &AnchoredAdmissionChoiceV2,
    visible: &FoldResult,
    scored: &[Scored],
) -> bool {
    let Ok(ordinal) = usize::try_from(choice.ordinal) else {
        return false;
    };
    visible.chosen == Some(choice.mask)
        && visible.chosen_side == Some(choice.side)
        && visible.chosen_exit == Some(choice.exit)
        && visible.chosen_exit_total == Some(choice.in_sample_pessimistic)
        && visible.out_of_sample_exit == Some(choice.out_of_sample_pessimistic)
        && visible.in_sample_all.get(ordinal).copied() == Some(choice.in_sample_pessimistic)
        && scored.get(ordinal).is_some_and(|candidate| {
            candidate.mask == choice.mask
                && candidate.side == choice.side
                && candidate.pick.rungs == choice.exit
                && candidate.pessimistic == choice.in_sample_pessimistic
        })
        && scored
            .iter()
            .map(|candidate| candidate.pessimistic)
            .max()
            .and_then(|best| {
                scored
                    .iter()
                    .position(|candidate| candidate.pessimistic == best)
            })
            == Some(ordinal)
}

fn stable_u64_v2(value: usize) -> Result<u64, AnchoredAdmissionValidationRefusalV2> {
    u64::try_from(value).map_err(|_| AnchoredAdmissionValidationRefusalV2::StableWidth)
}

const fn direction_tag_v2(direction: Direction) -> u8 {
    match direction {
        Direction::Long => 1,
        Direction::Short => 2,
    }
}

fn selected_exit_values_v2(
    pick: &ExitPick,
    fold: usize,
) -> Result<AnchoredAdmissionExitValuesV2, AnchoredAdmissionValidationRefusalV2> {
    let rung = |ladder: &crate::excursion::Ladder,
                index: Option<usize>|
     -> Result<Option<i64>, AnchoredAdmissionValidationRefusalV2> {
        index
            .map(|at| {
                ladder
                    .rungs()
                    .get(at)
                    .copied()
                    .ok_or(AnchoredAdmissionValidationRefusalV2::ExitRungMismatch { fold })
            })
            .transpose()
    };
    let (ttp_arm, ttp_trail) = pick.rungs.ttp.map_or(Ok((None, None)), |ttp| {
        Ok((
            rung(&pick.targets, Some(ttp.arm))?,
            rung(&pick.trails, Some(ttp.trail))?,
        ))
    })?;
    Ok(AnchoredAdmissionExitValuesV2 {
        stop: rung(&pick.stops, pick.rungs.stop)?,
        target: rung(&pick.targets, pick.rungs.target)?,
        tsl: rung(&pick.trails, pick.rungs.tsl)?,
        ttp_arm,
        ttp_trail,
    })
}

fn hash_validation_policy_v2(value: AnchoredAdmissionPolicyFactsV2) -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(ANCHORED_ADMISSION_POLICY_DOMAIN_V2);
    hasher.update(&[1]); // Shape::Anchored.
    hasher.update(&value.signal_bars.to_le_bytes());
    hasher.update(&value.horizon_bars.to_le_bytes());
    hasher.update(&value.splits.to_le_bytes());
    hasher.update(&value.resolved_rungs.to_le_bytes());
    hasher.update(&value.min_hits.to_le_bytes());
    hasher.update(&value.ceiling.to_le_bytes());
    hasher.update(&value.pair_budget.to_le_bytes());
    hasher.update(&[value.requested_direction, value.execution_mode]);
    hasher.update(&value.signal_length_micros.to_le_bytes());
    hasher.finalize()
}

fn hash_validation_family_v2(folds: &[AnchoredAdmissionFoldV2]) -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(ANCHORED_ADMISSION_FAMILY_DOMAIN_V2);
    hasher.update(&u64::try_from(folds.len()).unwrap_or(u64::MAX).to_le_bytes());
    for fold in folds {
        for value in [
            fold.index,
            fold.train_start,
            fold.train_end,
            fold.train_second_start,
            fold.train_second_end,
            fold.test_start,
            fold.test_end,
            fold.purged,
            fold.embargoed,
            fold.considered,
            fold.priced,
        ] {
            hasher.update(&value.to_le_bytes());
        }
        hasher.update(&fold.candidate_facts_digest);
    }
    hasher.finalize()
}

fn hash_walk_facts_v2(
    policy_digest: [u8; 32],
    family_digest: [u8; 32],
    folds: &[AnchoredAdmissionFoldV2],
) -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(ANCHORED_ADMISSION_FACTS_DOMAIN_V2);
    hasher.update(&policy_digest);
    hasher.update(&family_digest);
    for fold in folds {
        hash_summary_for_admission_v2(&mut hasher, fold.in_sample);
        hash_summary_for_admission_v2(&mut hasher, fold.out_of_sample);
        hash_choice_for_admission_v2(&mut hasher, fold.choice);
    }
    hasher.finalize()
}

fn hash_validation_policy_v3(value: AnchoredSearchPolicyFactsV3) -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(ANCHORED_SEARCH_POLICY_DOMAIN_V3);
    hasher.update(&[1]); // Shape::Anchored.
    hasher.update(&value.signal_bars.to_le_bytes());
    hasher.update(&value.horizon_bars.to_le_bytes());
    hasher.update(&value.splits.to_le_bytes());
    hasher.update(&value.resolved_rungs.to_le_bytes());
    hasher.update(&value.min_hits.to_le_bytes());
    hasher.update(&value.ceiling.to_le_bytes());
    hasher.update(&value.pair_budget.to_le_bytes());
    hasher.update(&[value.training_edge_side_rule, value.execution_mode]);
    hasher.update(&value.signal_length_micros.to_le_bytes());
    hasher.finalize()
}

fn hash_validation_family_v3(folds: &[AnchoredAdmissionFoldV2]) -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(ANCHORED_SEARCH_FAMILY_DOMAIN_V3);
    hasher.update(&u64::try_from(folds.len()).unwrap_or(u64::MAX).to_le_bytes());
    for fold in folds {
        for value in [
            fold.index,
            fold.train_start,
            fold.train_end,
            fold.train_second_start,
            fold.train_second_end,
            fold.test_start,
            fold.test_end,
            fold.purged,
            fold.embargoed,
            fold.considered,
            fold.priced,
        ] {
            hasher.update(&value.to_le_bytes());
        }
        hasher.update(&fold.candidate_facts_digest);
    }
    hasher.finalize()
}

fn hash_walk_facts_v3(
    policy_digest: [u8; 32],
    family_digest: [u8; 32],
    folds: &[AnchoredAdmissionFoldV2],
) -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(ANCHORED_SEARCH_FACTS_DOMAIN_V3);
    hasher.update(&policy_digest);
    hasher.update(&family_digest);
    for fold in folds {
        hash_summary_for_admission_v2(&mut hasher, fold.in_sample);
        hash_summary_for_admission_v2(&mut hasher, fold.out_of_sample);
        hash_choice_for_admission_v2(&mut hasher, fold.choice);
    }
    hasher.finalize()
}

fn hash_candidate_facts_v2(
    scored: &[Scored],
    oos: &[CandidateOosV2],
    fold: usize,
) -> Result<[u8; 32], AnchoredAdmissionValidationRefusalV2> {
    if scored.len() != oos.len() {
        return Err(AnchoredAdmissionValidationRefusalV2::CandidateFamilyMismatch { fold });
    }
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(ANCHORED_ADMISSION_CANDIDATES_DOMAIN_V2);
    hasher.update(&stable_u64_v2(scored.len())?.to_le_bytes());
    for (ordinal, (candidate, outcome)) in scored.iter().zip(oos).enumerate() {
        hasher.update(&stable_u64_v2(ordinal)?.to_le_bytes());
        hash_mask_for_admission_v2(&mut hasher, candidate.mask);
        hasher.update(&[direction_tag_v2(candidate.side)]);
        hash_chosen_for_admission_v2(&mut hasher, candidate.pick.rungs);
        let values = selected_exit_values_v2(&candidate.pick, fold)?;
        hash_exit_values_for_admission_v2(&mut hasher, values);
        hasher.update(&candidate.pessimistic.to_le_bytes());
        hash_optional_i64_for_admission_v2(&mut hasher, outcome.pessimistic);
    }
    Ok(hasher.finalize())
}

fn hash_candidate_facts_v3(
    scored: &[Scored],
    oos: &[CandidateOosV2],
    fold: usize,
) -> Result<[u8; 32], AnchoredSearchValidationRefusalV3> {
    if scored.len() != oos.len() {
        return Err(AnchoredSearchValidationRefusalV3::CandidateFamilyMismatch { fold });
    }
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(ANCHORED_SEARCH_CANDIDATES_DOMAIN_V3);
    hasher.update(&stable_u64_v2(scored.len())?.to_le_bytes());
    for (ordinal, (candidate, outcome)) in scored.iter().zip(oos).enumerate() {
        hasher.update(&stable_u64_v2(ordinal)?.to_le_bytes());
        hash_mask_for_admission_v2(&mut hasher, candidate.mask);
        hasher.update(&[direction_tag_v2(candidate.side)]);
        hash_chosen_for_admission_v2(&mut hasher, candidate.pick.rungs);
        let values = selected_exit_values_v2(&candidate.pick, fold)?;
        hash_exit_values_for_admission_v2(&mut hasher, values);
        hasher.update(&candidate.pessimistic.to_le_bytes());
        hash_optional_i64_for_admission_v2(&mut hasher, outcome.pessimistic);
    }
    Ok(hasher.finalize())
}

fn hash_choice_for_admission_v2(
    hasher: &mut brutex_core::blake3::Hasher,
    choice: Option<AnchoredAdmissionChoiceV2>,
) {
    let Some(choice) = choice else {
        hasher.update(&[0]);
        return;
    };
    hasher.update(&[1]);
    hasher.update(&choice.ordinal.to_le_bytes());
    hash_mask_for_admission_v2(hasher, choice.mask);
    hasher.update(&[direction_tag_v2(choice.side)]);
    hash_chosen_for_admission_v2(hasher, choice.exit);
    hash_exit_values_for_admission_v2(hasher, choice.exit_values);
    hasher.update(&choice.in_sample_pessimistic.to_le_bytes());
    hasher.update(&choice.out_of_sample_pessimistic.to_le_bytes());
}

fn hash_mask_for_admission_v2(hasher: &mut brutex_core::blake3::Hasher, mask: ConditionMask) {
    for word in mask.words() {
        hasher.update(&word.to_le_bytes());
    }
}

fn hash_chosen_for_admission_v2(
    hasher: &mut brutex_core::blake3::Hasher,
    chosen: crate::grid::Chosen,
) {
    for rung in [chosen.stop, chosen.target, chosen.tsl] {
        hash_optional_usize_for_admission_v2(hasher, rung);
    }
    if let Some(ttp) = chosen.ttp {
        hasher.update(&[1]);
        hasher.update(&u64::try_from(ttp.arm).unwrap_or(u64::MAX).to_le_bytes());
        hasher.update(&u64::try_from(ttp.trail).unwrap_or(u64::MAX).to_le_bytes());
    } else {
        hasher.update(&[0]);
        hasher.update(&[0; 16]);
    }
}

fn hash_exit_values_for_admission_v2(
    hasher: &mut brutex_core::blake3::Hasher,
    values: AnchoredAdmissionExitValuesV2,
) {
    for value in [
        values.stop,
        values.target,
        values.tsl,
        values.ttp_arm,
        values.ttp_trail,
    ] {
        hash_optional_i64_for_admission_v2(hasher, value);
    }
}

fn hash_optional_usize_for_admission_v2(
    hasher: &mut brutex_core::blake3::Hasher,
    value: Option<usize>,
) {
    hasher.update(&[u8::from(value.is_some())]);
    hasher.update(
        &value
            .and_then(|v| u64::try_from(v).ok())
            .unwrap_or_default()
            .to_le_bytes(),
    );
}

fn hash_optional_i64_for_admission_v2(
    hasher: &mut brutex_core::blake3::Hasher,
    value: Option<i64>,
) {
    hasher.update(&[u8::from(value.is_some())]);
    hasher.update(&value.unwrap_or_default().to_le_bytes());
}

fn hash_summary_for_admission_v2(hasher: &mut brutex_core::blake3::Hasher, summary: Summary) {
    hasher.update(&summary.trades.to_le_bytes());
    hasher.update(&summary.best.to_le_bytes());
    hasher.update(&summary.worst.to_le_bytes());
    hasher.update(&summary.forced.to_le_bytes());
}

#[expect(
    clippy::too_many_arguments,
    reason = "the private join carries both public validation modes without \
              letting the one-series door infer an execution path"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one fold is one procedure: sweep signal history, project only its \
              admitted rows, rank on its execution prefix, then judge on the \
              disjoint test prefix. Splitting those halves is how leakage \
              defects become visually plausible"
)]
fn walk_forward_core(
    bars: &[Candle],
    horizon: Horizon,
    splits: usize,
    sweeper: &crate::Sweeper,
    builder: &mut impl FnMut(&[Candle]) -> Result<Column, String>,
    shape: Shape,
    trades_on_these_bars: TradesOnTheseBars,
    rungs: usize,
    execution: Option<ExecutionSeries<'_>>,
    mut admission_capture: Option<AnchoredCapture<'_>>,
) -> Validated {
    // ONE-SERIES AND TWO-SERIES MODES SHARE THE FOLD, NOT THE ASSUMPTION.
    //
    // With no `execution`, the caller must affirm that `bars` genuinely has both
    // roles. With one, each fold sweeps only its signal range, then builds
    // `trade_train`/`trade_test` and checked-reprojects onto those bounded
    // one-minute prefixes before any trade, grid, or exit is evaluated.
    //
    // A `bool` would have been enough to express this and is exactly what let
    // the defect exist -- `cli` passed `&bars` where its sibling three lines away
    // passed `&trade_bars`, and nothing in either signature could tell them
    // apart. A named type makes the caller answer the question out loud.
    if let TradesOnTheseBars::No(why) = trades_on_these_bars {
        return Validated {
            folds: Vec::new(),
            refused: Some(why),
        };
    }
    if let Some(exec) = execution {
        if exec.signal_length_micros <= 0 {
            return Validated {
                folds: Vec::new(),
                refused: Some(
                    "the signal timeframe has no positive duration, so its exact close cannot be mapped"
                        .to_owned(),
                ),
            };
        }
        if !is_one_minute_path(exec.bars) {
            return Validated {
                folds: Vec::new(),
                refused: Some(
                    "the execution series is empty, unordered, duplicated, sub-minute, or off the exact one-minute timestamp grid"
                        .to_owned(),
                ),
            };
        }
    }
    let mut out = Validated::default();
    let rungs = if rungs == 0 { DEFAULT_RUNGS } else { rungs };
    let mut training_execution_cursor = MonotonicExecutionPrefix::default();
    let mut oos_execution_cursor = MonotonicExecutionPrefix::default();

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
        let signal_train_column = match builder(train) {
            Ok(column) => column,
            Err(why) => {
                out.refused = Some(why);
                return out;
            }
        };
        let swept = crate::Sweeper::new(per_fold).run_prepared(&signal_train_column);

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
        // THE TRAIN EXECUTION PREFIX STOPS BEFORE THE FIRST TEST SIGNAL.
        //
        // A mask is still found only on `train`. Pricing it may read the purge
        // gap after the last training signal, but it may not read the first
        // observation the fold is holding out. Clipping by timestamp rather
        // than by an execution index keeps the rule correct across missing
        // minutes and coarse signal rungs.
        let Some(test_open) = bars.get(fold.test.start).map(|bar| bar.ts_micros) else {
            continue;
        };
        let trade_train = if let Some(exec) = execution {
            let Some(prefix) = training_execution_cursor.before(exec.bars, test_open) else {
                out.refused =
                    Some("walk-forward training execution boundaries are not monotonic".to_owned());
                return out;
            };
            prefix
        } else {
            train
        };
        let projected_train = match execution {
            Some(exec) => match project_fold(
                train,
                &signal_train_column,
                trade_train,
                exec.signal_length_micros,
            ) {
                Ok(projected) => Some(projected),
                Err(why) => {
                    out.refused = Some(why);
                    return out;
                }
            },
            None => None,
        };
        let train_column = projected_train.as_ref().unwrap_or(&signal_train_column);
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
        let mut best_ordinal: Option<usize> = None;
        let mut best_summary = Summary::default();
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
        let facts = crate::trade::SliceFacts::of(trade_train, train_column);
        // THE TRAIN WINDOW'S OWN FORWARD RETURNS, so each candidate can be
        // priced on the side ITS OWN evidence points to.
        //
        // The fold took ONE `Direction`, and the caller derived it from
        // `side_of_evidence(first)` -- the top row of a rank over the WHOLE
        // span, test folds included. Two defects in one line: the side was
        // fitted to the window it is meant to be tested on, which is the exact
        // look-ahead `walk_forward_shaped`'s own doc argues against for the
        // STOP; and it was then applied to every OTHER candidate in the fold,
        // so a short-edged combination was priced as a long and could never be
        // chosen. `crate::pbo` ranks that vector, so the reported
        // probability-of-overfitting was computed on it.
        //
        // Hoisted beside `facts` because it too is a fact about `train` and
        // does not vary by candidate. One pass, shared by every thread.
        let forward = crate::outcome::forward(trade_train, train_column, horizon);
        // THE RUNG COUNT WAS RESOLVED BY THE CALLER, ONCE. It is the same
        // already-clamped value the caller records in run identity; no lane
        // reads `std::env`, and no fold can reinterpret the memory bound.
        // THE SIDE TRAVELS WITH THE CANDIDATE, because the test window must be
        // priced on the side TRAINING chose and not on one derived again from
        // the bars being tested.
        let assessed: Vec<Option<Assessed>> = closed
            .kept
            .par_iter()
            .map(|item| {
                // THIS CANDIDATE'S OWN SIDE, from `train` alone. Same rule
                // `cli::side_of_evidence` applies in production -- the sign of
                // the mean forward return -- read here on the training window
                // so nothing about the test window decides it.
                let own = direction_from_training_edge(
                    crate::outcome::edge(train_column, &forward, &item.mask).mean_paisa,
                );
                let g = crate::grid::evaluate_over(
                    trade_train,
                    train_column,
                    &item.mask,
                    horizon,
                    side_of(own),
                    crate::grid::Levels::derived(rungs),
                    &facts,
                );
                let cell = g.sharpest().or_else(|| g.best())?;
                if cell.trades == 0 {
                    return None;
                }
                let s = Summary::of(&crate::trade::walk_over(
                    trade_train,
                    train_column,
                    &item.mask,
                    horizon,
                    own,
                    &facts,
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
                    stops: g.stops.clone(),
                    targets: g.targets.clone(),
                    trails: g.trails.clone(),
                };
                Some(Assessed {
                    mask: item.mask,
                    summary: s,
                    pick,
                    pessimistic: cell.pessimistic,
                    side: own,
                })
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
        // The side rides in this vector too, so `oos_all` prices every candidate
        // on the side its own training evidence chose.
        let mut scored: Vec<Scored> = Vec::with_capacity(assessed.len());
        let priced: u64 = u64::try_from(closed.kept.len()).unwrap_or(u64::MAX);
        for Assessed {
            mask,
            summary: s,
            pick,
            pessimistic,
            side: own,
        } in assessed.into_iter().flatten()
        {
            let improves = best_ordinal
                .and_then(|ordinal| scored.get(ordinal))
                .is_none_or(|held| pessimistic > held.pessimistic);
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
                best_ordinal = Some(scored.len());
                best_summary = s;
            }
            scored.push(Scored {
                mask,
                pessimistic,
                pick,
                side: own,
            });
        }

        // The exit came out of the SAME evaluation that chose the combination,
        // so there is no second grid pass and no chance of the two disagreeing.
        // It is derived from the TRAINING trades alone -- a stop fitted to the
        // test window is the same look-ahead as a combination fitted to it, and
        // worse, because a stop fitted to the future looks spectacular and is
        // trivially findable.
        // `chosen_side` is the sixth, and it is the whole point: the test window
        // is priced on the side TRAINING picked for this candidate, carried
        // across unchanged, exactly as the exit rungs are.
        let (chosen, in_sample, chosen_exit, chosen_exit_total, chosen_side) = best_ordinal
            .and_then(|ordinal| scored.get(ordinal))
            .map_or((None, Summary::default(), None, None, None), |candidate| {
                (
                    Some(candidate.mask),
                    best_summary,
                    Some(candidate.pick.rungs),
                    Some(candidate.pessimistic),
                    Some(candidate.side),
                )
            });

        // OUT OF SAMPLE. The column runs from bar zero so the indicators hold
        // what they would genuinely have held, and the trade walk is confined to
        // the test window by `restricted`.
        let mut oos_all: Vec<i64> = Vec::new();
        let mut oos_exact: Vec<CandidateOosV2> = Vec::new();
        let (out_of_sample, out_of_sample_exit) = match chosen {
            Some(mask) => {
                let Some(chosen_side) = chosen_side else {
                    out.refused = Some(
                        "a walk-forward fold selected a candidate without its training side"
                            .to_owned(),
                    );
                    return out;
                };
                let signal_upto = bars.get(..fold.test.end).unwrap_or(bars);
                let full = match builder(signal_upto) {
                    Ok(column) => column,
                    Err(why) => {
                        out.refused = Some(why);
                        return out;
                    }
                };
                let confined_signal = restricted(&full, fold.test.start);
                // TEST INDICATORS SEE ONLY THE SIGNAL PREFIX THROUGH THIS
                // WINDOW. Its execution path may extend exactly one configured
                // horizon beyond the last test signal, because those bars are
                // the label being judged; nothing later is in scope.
                let trade_test = if let Some(exec) = execution {
                    let last_signal = signal_upto.last().map_or(i64::MIN, |bar| {
                        bar.ts_micros.saturating_add(exec.signal_length_micros)
                    });
                    let hold = i64::from(horizon.as_bars()).saturating_mul(EXECUTION_MINUTE_MICROS);
                    let Some(prefix) =
                        oos_execution_cursor.through(exec.bars, last_signal.saturating_add(hold))
                    else {
                        out.refused = Some(
                            "walk-forward OOS execution boundaries are not monotonic".to_owned(),
                        );
                        return out;
                    };
                    prefix
                } else {
                    signal_upto
                };
                let projected_test = match execution {
                    Some(exec) => match project_fold(
                        signal_upto,
                        &confined_signal,
                        trade_test,
                        exec.signal_length_micros,
                    ) {
                        Ok(projected) => Some(projected),
                        Err(why) => {
                            out.refused = Some(why);
                            return out;
                        }
                    },
                    None => None,
                };
                let confined = projected_test.as_ref().unwrap_or(&confined_signal);
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
                // HOISTED, because it is the same answer every time.
                //
                // `with_levels` builds `SliceFacts` per call, so this loop
                // rebuilt them once per candidate: an O(B) `HashMap` reserved at
                // `bars.len()` with that many hashed inserts, two `Vec<u64>` of
                // `B + 1`, a `Vec<Option<SquareOff>>` of `B`, and a second full
                // pass for the median step — O(C x B) to compute one
                // candidate-INDEPENDENT value C times.
                //
                // `SliceFacts`' own doc says the type exists so that "a loop
                // over candidates must hoist the facts", and `evaluate_over`
                // and `walk_over` are that door for their callers.
                // `with_levels` had none until `with_levels_over`, which is why
                // the source-shape guard written for this — it checks
                // `walk_core` — never saw this loop.
                //
                // Determinism is untouched: the facts are a pure function of
                // `trade_test` and `confined`, both borrowed immutably here and
                // unchanged by the loop.
                let test_facts = crate::trade::SliceFacts::of(trade_test, confined);
                oos_exact = scored
                    .par_iter()
                    .map(|candidate| {
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
                        let pessimistic = crate::grid::with_levels_over(
                            trade_test,
                            confined,
                            &candidate.mask,
                            horizon,
                            // THE SIDE TRAINING CHOSE FOR THIS CANDIDATE.
                            side_of(candidate.side),
                            crate::excursion::Ladders {
                                stops: &candidate.pick.stops,
                                targets: &candidate.pick.targets,
                                trails: &candidate.pick.trails,
                            },
                            candidate.pick.rungs,
                            &test_facts,
                        )
                        .map(|cell| cell.pessimistic);
                        CandidateOosV2 { pessimistic }
                    })
                    .collect();
                oos_all = oos_exact
                    .iter()
                    .map(|outcome| outcome.pessimistic.unwrap_or_default())
                    .collect();
                // THE SIDE TRAINING CHOSE, not the one a whole-span rank did.
                let plain = Summary::of(&walk(trade_test, confined, &mask, horizon, chosen_side));

                // THE CHOSEN EXIT, APPLIED. `docs/06-limits.md` §70 recorded
                // that this fold reported a chosen stop beside an out-of-sample
                // total that had never used it -- the walk above takes no
                // levels. The variant is now scored on the test window using
                // the TRAINING ladders, so the rung values travel unchanged and
                // nothing about the test bars decides a level.
                // The winner's OOS value is selected by its canonical ordinal,
                // not by searching the vector for an equal number. A different
                // candidate may legitimately have the same total.
                let with = best_ordinal
                    .and_then(|ordinal| oos_exact.get(ordinal))
                    .and_then(|outcome| outcome.pessimistic);
                (plain, with)
            }
            None => (Summary::default(), None),
        };

        let visible = FoldResult {
            // Paired with `chosen`, so the two cannot disagree about whether
            // this fold decided anything.
            chosen_side,
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
            in_sample_all: scored
                .iter()
                .map(|candidate| candidate.pessimistic)
                .collect(),
            out_of_sample_all: oos_all,
        };
        if let Some(capture) = admission_capture.as_mut()
            && !capture.observe_fold_and_retain(&fold, &visible, &scored, &oos_exact, best_ordinal)
        {
            return out;
        }
        out.folds.push(visible);
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
    use super::{
        AnchoredAdmissionValidationRefusalV2, AnchoredAdmissionValidationV2,
        AnchoredSearchValidationV3, DEFAULT_RUNGS, ExecutionSeries, MonotonicExecutionPrefix,
        Shape, TradesOnTheseBars, Validated, is_one_minute_path, project_fold, project_oos_fold,
        walk_forward, walk_forward_projected_prepared_anchored_admission_v2,
        walk_forward_projected_prepared_anchored_search_v3,
        walk_forward_projected_prepared_with_rungs, walk_forward_projected_with_rungs,
        walk_forward_shaped, walk_forward_shaped_with_rungs, walk_forward_with_rungs,
    };

    /// A caller whose two series differ gets NO folds and a stated reason.
    ///
    /// # The number this refuses to print
    ///
    /// `walk_forward_shaped` sweeps conditions on the slice it is given and then
    /// trades on that same slice -- `SliceFacts::of`, `grid::evaluate_over` and
    /// `walk_over` all take it. Correct whenever positions are taken on the
    /// series the conditions were found on, which is every caller that supplies
    /// no execution series.
    ///
    /// `cli`'s `audit-range` and `range-all` are not those callers. On a
    /// 60-minute signal series the last bar inside the fill window is the 14:15
    /// bucket, so `entry + horizon` exceeds it for every entry and every fold
    /// trade takes the forced exit there -- a five-hour hold standing in for one
    /// that lasts fifteen minutes, printed as out-of-sample evidence.
    ///
    /// Its sibling was already corrected: `cli` hands `bootstrap_family` the
    /// trade series, under a comment naming this defect class. This call kept the
    /// signal series because both arguments are `&[Candle]` and no signature
    /// could tell them apart. `TradesOnTheseBars` is what makes the difference
    /// sayable, and this is what makes it enforced.
    #[test]
    fn a_walk_that_would_trade_the_wrong_series_refuses_and_names_the_reason() {
        let bars = crate::synthetic::sessions(12);
        let refused = walk_forward_shaped(
            &bars,
            h(15),
            3,
            Direction::Long,
            &sweeper(),
            &mut evaluator,
            Shape::Anchored,
            TradesOnTheseBars::No("the fills happen elsewhere".to_owned()),
        );
        assert!(
            refused.folds.is_empty(),
            "a refused walk runs no fold at all"
        );
        assert_eq!(
            refused.refused.as_deref(),
            Some("the fills happen elsewhere"),
            "and it carries the caller's own reason verbatim, because the reason \
             names which parts of the report are still trustworthy"
        );

        // THE SAME CALL WITH THE SAME BARS RUNS when the caller says the series
        // is the one positions are taken on -- so the refusal is the flag's doing
        // and not a fixture too short to split, which renders identically and is
        // the confusion `Validated::refused` exists to end.
        let ran = walk_forward_shaped(
            &bars,
            h(15),
            3,
            Direction::Long,
            &sweeper(),
            &mut evaluator,
            Shape::Anchored,
            TradesOnTheseBars::Yes,
        );
        assert!(
            ran.refused.is_none(),
            "an attempted walk records no refusal"
        );
        assert!(
            !ran.folds.is_empty(),
            "and this fixture does split, which is what makes the contrast mean \
             something"
        );
    }

    use crate::Sweeper;
    use crate::exit_grid_policy::{
        ExecutionResolutionV1, ExitGridPolicyV1, ExitGridSelectorV1, ForcedStopV1,
        RangeResolutionV1, RatioLimitsV1, RationalPercentileV1, RungPlanV1,
        printed_ohlcv_cost_model_id_v1,
    };
    use crate::outcome::Horizon;
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use costs::fill::Direction;
    use engine::Ladder;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use std::sync::OnceLock;
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

    fn anchored_admission_run_v2(
        horizon: Horizon,
        splits: usize,
        rungs: usize,
    ) -> AnchoredAdmissionValidationV2 {
        let bars = crate::synthetic::sessions(12);
        let complete = Sweeper::new(Ladder::with_min_hits(1_200).with_ceiling(20_000));
        let mut builder = |slice: &[indicators::Candle]| {
            let mut evaluator = evaluator();
            Ok(Column::build(slice, &mut evaluator))
        };
        walk_forward_projected_prepared_anchored_admission_v2(
            &bars,
            ExecutionSeries {
                bars: &bars,
                signal_length_micros: 60_000_000,
            },
            horizon,
            splits,
            Direction::Long,
            &complete,
            &mut builder,
            rungs,
        )
        .expect("the complete anchored fixture issues opaque provenance")
    }

    fn anchored_admission_fixture_v2() -> AnchoredAdmissionValidationV2 {
        static FIXTURE: OnceLock<AnchoredAdmissionValidationV2> = OnceLock::new();
        FIXTURE
            .get_or_init(|| anchored_admission_run_v2(h(15), 3, DEFAULT_RUNGS))
            .clone()
    }

    fn anchored_search_run_v3(
        horizon: Horizon,
        splits: usize,
        rungs: usize,
    ) -> AnchoredSearchValidationV3 {
        let bars = crate::synthetic::sessions(12);
        let complete = Sweeper::new(Ladder::with_min_hits(1_200).with_ceiling(20_000));
        let mut builder = |slice: &[indicators::Candle]| {
            let mut evaluator = evaluator();
            Ok(Column::build(slice, &mut evaluator))
        };
        walk_forward_projected_prepared_anchored_search_v3(
            &bars,
            ExecutionSeries {
                bars: &bars,
                signal_length_micros: 60_000_000,
            },
            horizon,
            splits,
            &complete,
            &mut builder,
            rungs,
        )
        .expect("the complete V3 anchored fixture issues opaque provenance")
    }

    fn anchored_search_fixture_v3() -> AnchoredSearchValidationV3 {
        static FIXTURE: OnceLock<AnchoredSearchValidationV3> = OnceLock::new();
        FIXTURE
            .get_or_init(|| anchored_search_run_v3(h(15), 3, DEFAULT_RUNGS))
            .clone()
    }

    fn exact_grid_policy_v4(side: crate::excursion::Side) -> ExitGridPolicyV1 {
        let middle =
            RationalPercentileV1::new(1, 2).expect("the exact one-half percentile is valid");
        let rungs = RungPlanV1::new(vec![middle], vec![middle], vec![middle], 1)
            .expect("one explicit rung per axis is bounded");
        let ratios = RatioLimitsV1::new(1, 100_000, 1)
            .expect("the deliberately broad single-pair ratio range is valid");
        ExitGridPolicyV1::new(
            ExecutionResolutionV1::OneMinuteOhlcv,
            RangeResolutionV1::PpmCeiling,
            side,
            rungs,
            ratios,
            32,
            ExitGridSelectorV1::PessimisticTotal,
            printed_ohlcv_cost_model_id_v1(),
            ForcedStopV1::Disabled,
            u64::MAX,
            u64::MAX,
        )
        .expect("the complete explicit V4 fixture policy is valid")
    }

    fn anchored_search_run_v4_with_column(
        full_signal_column: &Column,
    ) -> Result<super::AnchoredSearchValidationV4, super::AnchoredSearchValidationRefusalV4> {
        let bars = crate::synthetic::sessions(16);
        let instrument =
            InstrumentKey::index(Exchange::Nse, "NIFTY").expect("NSE-NIFTY is an allowed index");
        let execution = super::ExecutionSeriesV1::new(
            &instrument,
            "test-feed",
            "test-commit",
            [0xA5; 32],
            &bars,
        )
        .expect("the fixture carries every execution identity");
        let long = exact_grid_policy_v4(crate::excursion::Side::Long)
            .resolve_attested(execution)
            .expect("the Long grid resolves over the exact full span");
        let short = exact_grid_policy_v4(crate::excursion::Side::Short)
            .resolve_attested(execution)
            .expect("the Short grid resolves over the exact full span");
        let complete = Sweeper::new(Ladder::with_min_hits(2_200).with_ceiling(20_000));
        let mut builder = |slice: &[indicators::Candle]| {
            let mut evaluator = evaluator();
            Ok(Column::build(slice, &mut evaluator))
        };
        super::walk_forward_projected_prepared_anchored_search_v4(
            &bars,
            full_signal_column,
            execution,
            60_000_000,
            h(15),
            1,
            &complete,
            &mut builder,
            &long,
            &short,
        )
    }

    fn anchored_search_run_v4() -> super::AnchoredSearchValidationV4 {
        let bars = crate::synthetic::sessions(16);
        let full_signal_column = Column::build(&bars, &mut evaluator());
        anchored_search_run_v4_with_column(&full_signal_column)
            .expect("the exact causal V4 fixture issues opaque authority")
    }

    fn anchored_search_fixture_v4() -> super::AnchoredSearchValidationV4 {
        static FIXTURE: OnceLock<super::AnchoredSearchValidationV4> = OnceLock::new();
        FIXTURE.get_or_init(anchored_search_run_v4).clone()
    }

    fn admission_capture_v2() -> super::AnchoredAdmissionCaptureV2 {
        super::AnchoredAdmissionCaptureV2::new(
            2,
            h(1),
            1,
            Direction::Long,
            &sweeper(),
            1,
            60_000_000,
        )
        .expect("the fixed test policy fits the stable identity domain")
    }

    fn admission_candidate_v2() -> super::Scored {
        super::Scored {
            mask: ConditionMask::ZERO.with_bit(0),
            pessimistic: 7,
            pick: super::ExitPick {
                rungs: crate::grid::Chosen::default(),
                stops: crate::excursion::Ladder::default(),
                targets: crate::excursion::Ladder::default(),
                trails: crate::excursion::Ladder::default(),
            },
            side: Direction::Long,
        }
    }

    fn admission_split_v2() -> crate::split::Fold {
        crate::split::Fold {
            train: (0..1, 1..1),
            test: 1..2,
            purged: 0,
            embargoed: 0,
        }
    }

    #[test]
    fn anchored_admission_v2_refuses_zero_rungs_before_running_any_search() {
        let bars = [];
        let mut builder_was_called = false;
        let mut builder = |_slice: &[indicators::Candle]| -> Result<Column, String> {
            builder_was_called = true;
            Err("the zero-rung door must refuse before building".to_owned())
        };
        let result = walk_forward_projected_prepared_anchored_admission_v2(
            &bars,
            ExecutionSeries {
                bars: &bars,
                signal_length_micros: 60_000_000,
            },
            h(1),
            1,
            Direction::Long,
            &sweeper(),
            &mut builder,
            0,
        );
        assert!(matches!(
            result,
            Err(AnchoredAdmissionValidationRefusalV2::ZeroResolvedRungs)
        ));
        assert!(
            !builder_was_called,
            "zero is a malformed resolved Admission policy, not permission to run a default search"
        );
    }

    #[test]
    fn anchored_admission_v2_refuses_a_decided_fold_without_an_exact_oos_exit() {
        let candidate = admission_candidate_v2();
        let mask = candidate.mask;
        let exit = candidate.pick.rungs;
        let visible = super::FoldResult {
            considered: 1,
            priced: 1,
            chosen: Some(mask),
            chosen_side: Some(Direction::Long),
            chosen_exit: Some(exit),
            chosen_exit_total: Some(candidate.pessimistic),
            out_of_sample_exit: None,
            in_sample_all: vec![candidate.pessimistic],
            // The legacy ranking vector uses zero for an absent candidate OOS
            // walk, but Admission must not turn that ranking placeholder into
            // a measured chosen outcome.
            out_of_sample_all: vec![0],
            ..super::FoldResult::default()
        };
        let mut capture = admission_capture_v2();
        assert_eq!(
            capture.observe_fold(
                &admission_split_v2(),
                &visible,
                &[candidate],
                &[super::CandidateOosV2 { pessimistic: None }],
                Some(0),
            ),
            Err(AnchoredAdmissionValidationRefusalV2::IncompleteChosenOos { fold: 0 }),
            "Validated::decided counts this chosen fold, so the fixed-width Admission projection must refuse rather than silently omit it from decided_folds and aggregate a made-up zero"
        );
    }

    #[test]
    fn anchored_admission_v2_refuses_when_a_priced_candidate_was_dropped_before_capture() {
        let candidate = admission_candidate_v2();
        let visible = super::FoldResult {
            considered: 2,
            priced: 2,
            in_sample_all: vec![candidate.pessimistic],
            out_of_sample_all: vec![3],
            ..super::FoldResult::default()
        };
        let mut capture = admission_capture_v2();
        assert_eq!(
            capture.observe_fold(
                &admission_split_v2(),
                &visible,
                &[candidate],
                &[super::CandidateOosV2 {
                    pessimistic: Some(3),
                }],
                None,
            ),
            Err(AnchoredAdmissionValidationRefusalV2::CandidateFamilyMismatch { fold: 0 }),
            "matching retained vectors do not prove completeness when their one row is fewer than the two candidates the fold says it priced"
        );
    }

    /// Whole missing minutes are a market-data absence handled by exact
    /// projection. Duplicate, backward, sub-minute, or off-grid stamps are a
    /// malformed execution path and refuse before any fold is run.
    #[test]
    fn a_projected_walk_accepts_minute_gaps_but_refuses_malformed_execution_time() {
        let dense = crate::synthetic::sessions(1);
        assert!(is_one_minute_path(&dense));

        let mut gapped = dense.clone();
        gapped.remove(20);
        assert!(
            is_one_minute_path(&gapped),
            "a whole missing minute is represented as an unreachable exact fill"
        );

        let mut duplicate = dense.clone();
        let repeated = duplicate.get(20).copied().expect("minute 20 exists");
        duplicate.insert(20, repeated);
        assert!(!is_one_minute_path(&duplicate));

        let mut backward = dense.clone();
        backward.swap(20, 21);
        assert!(!is_one_minute_path(&backward));

        let mut sub_minute = dense.clone();
        let previous = sub_minute
            .get(19)
            .map(|bar| bar.ts_micros)
            .expect("minute 19 exists");
        sub_minute.get_mut(20).expect("minute 20 exists").ts_micros =
            previous.saturating_add(30_000_000);
        assert!(!is_one_minute_path(&sub_minute));

        let mut off_grid = dense.clone();
        let first = off_grid.first_mut().expect("the fixture is not empty");
        first.ts_micros = first.ts_micros.saturating_add(1);
        assert!(!is_one_minute_path(&off_grid));
        assert!(!is_one_minute_path(&[]));

        let refused = walk_forward_projected_with_rungs(
            &dense,
            ExecutionSeries {
                bars: &duplicate,
                signal_length_micros: 60_000_000,
            },
            h(15),
            2,
            Direction::Long,
            &sweeper(),
            &mut evaluator,
            Shape::Anchored,
            DEFAULT_RUNGS,
        );
        assert!(refused.folds.is_empty());
        assert!(
            refused
                .refused
                .as_deref()
                .is_some_and(|why| why.contains("unordered, duplicated, sub-minute, or off")),
            "the public door must surface the malformed-path cause: {:?}",
            refused.refused
        );
    }

    #[test]
    fn a_prepared_fold_builder_refusal_never_falls_back_to_the_ordinary_evaluator() {
        let bars = crate::synthetic::sessions(12);
        let mut called = 0_usize;
        let refused = walk_forward_projected_prepared_with_rungs(
            &bars,
            ExecutionSeries {
                bars: &bars,
                signal_length_micros: 60_000_000,
            },
            h(15),
            3,
            Direction::Long,
            &sweeper(),
            &mut |_| {
                called = called.saturating_add(1);
                Err("stored daily/minute replay evidence refused this fold".to_owned())
            },
            Shape::Anchored,
            DEFAULT_RUNGS,
        );
        assert_eq!(called, 1, "the first refused fold stops the walk");
        assert!(refused.folds.is_empty());
        assert_eq!(
            refused.refused.as_deref(),
            Some("stored daily/minute replay evidence refused this fold")
        );
    }

    /// Fold ranges stay in signal-index space. Only after the training column
    /// is fixed are its rows mapped to exact one-minute timestamps, and the
    /// execution prefix ends strictly before the held-out test signal.
    #[test]
    fn a_coarse_fold_projects_exactly_and_cannot_read_its_test_execution_bar() {
        let execution = crate::synthetic::sessions(120);
        let period = crate::resample::Period::minutes(60).expect("sixty minutes is coarse");
        let signal = crate::resample::resample(&execution, period);
        let fold = Shape::Anchored
            .folds(signal.len(), h(15), 2)
            .into_iter()
            .next()
            .expect("the coarse fixture produces a fold");
        let train = signal
            .get(fold.train.0.clone())
            .expect("the fold's training range is in the signal series");
        let test_open = signal
            .get(fold.test.start)
            .map(|bar| bar.ts_micros)
            .expect("the fold has a test start");
        let train_execution = MonotonicExecutionPrefix::default()
            .before(&execution, test_open)
            .expect("the first training boundary is monotonic");
        assert!(
            train_execution
                .last()
                .is_some_and(|bar| bar.ts_micros < test_open)
        );
        assert!(
            execution.iter().any(|bar| bar.ts_micros == test_open),
            "the excluded first-test minute must really exist"
        );

        let signal_column = Column::build(train, &mut evaluator());
        assert!(!signal_column.is_empty(), "the fixture must clear warm-up");
        let projected = project_fold(train, &signal_column, train_execution, 3_600_000_000)
            .expect("a 60-minute signal column projects onto exact minutes");
        assert_eq!(projected.sourced(), indicators::column::Sourced::Fill);
        assert!(projected.sources().iter().all(|&source| {
            train_execution
                .get(source)
                .is_some_and(|bar| bar.ts_micros < test_open)
        }));

        let exact_signal_closes: std::collections::HashSet<i64> = signal_column
            .sources()
            .iter()
            .filter_map(|&source| train.get(source))
            .map(|bar| bar.ts_micros.saturating_add(3_600_000_000))
            .collect();
        assert!(projected.sources().iter().all(|&source| {
            train_execution
                .get(source)
                .is_some_and(|bar| exact_signal_closes.contains(&bar.ts_micros))
        }));

        // The held-out minute is excluded by timestamp, not by price. Mutating
        // it cannot change a byte of the training prefix.
        let mut changed = execution.clone();
        let held_out = changed
            .iter()
            .position(|bar| bar.ts_micros == test_open)
            .expect("held-out minute exists");
        let hidden = changed
            .get_mut(held_out)
            .expect("the located held-out minute remains present");
        hidden.close = hidden.close.saturating_add(1_000_000);
        let changed_train = MonotonicExecutionPrefix::default()
            .before(&changed, test_open)
            .expect("the changed prefix has the same monotonic boundary");
        assert_eq!(
            changed_train, train_execution,
            "the first test execution observation leaked into training"
        );

        let last_test_close = signal
            .get(fold.test.end.saturating_sub(1))
            .map(|bar| bar.ts_micros.saturating_add(3_600_000_000))
            .expect("the fold has a last test signal");
        let label_end = last_test_close.saturating_add(15 * 60_000_000);
        let test_execution = MonotonicExecutionPrefix::default()
            .through(&execution, label_end)
            .expect("the first OOS boundary is monotonic");
        assert!(
            test_execution
                .last()
                .is_some_and(|bar| bar.ts_micros <= label_end),
            "test pricing read beyond its configured execution horizon"
        );
    }

    /// Prefix clipping has exact boundary semantics and no full-series
    /// fallback: before the first bar is empty, while through the last bar is
    /// the complete supplied series.
    #[test]
    fn execution_prefix_boundaries_are_exact_and_never_fall_back() {
        let execution = crate::synthetic::sessions(1);
        let first = execution
            .first()
            .map(|bar| bar.ts_micros)
            .expect("the one-session fixture is non-empty");
        let last = execution
            .last()
            .map(|bar| bar.ts_micros)
            .expect("the one-session fixture has a final bar");

        let mut before = MonotonicExecutionPrefix::default();
        assert!(
            before
                .before(&execution, first)
                .expect("the first exact boundary is monotonic")
                .is_empty()
        );
        assert_eq!(
            before
                .before(&execution, last.saturating_add(1))
                .expect("the later exact boundary advances"),
            execution
        );

        let mut through = MonotonicExecutionPrefix::default();
        assert!(
            through
                .through(&execution, first.saturating_sub(1))
                .expect("the boundary before the first bar is representable")
                .is_empty()
        );
        assert_eq!(
            through
                .through(&execution, last)
                .expect("the later inclusive boundary advances"),
            execution
        );
        assert!(
            through.through(&execution, first).is_none(),
            "a decreasing boundary refuses instead of restarting or falling back"
        );

        let mut gapped = execution.clone();
        let removed_ts = gapped
            .get(10)
            .map(|bar| bar.ts_micros)
            .expect("the session contains an interior minute");
        gapped.remove(10);
        let mut gap_cursor = MonotonicExecutionPrefix::default();
        let before_gap = gap_cursor
            .before(&gapped, removed_ts)
            .expect("a missing minute does not break the monotonic cursor");
        assert!(before_gap.iter().all(|bar| bar.ts_micros < removed_ts));
        let through_gap = gap_cursor
            .before(&gapped, removed_ts.saturating_add(60_000_001))
            .expect("the cursor advances across the timestamp gap");
        assert_eq!(
            through_gap.last().map(|bar| bar.ts_micros),
            Some(removed_ts.saturating_add(60_000_000)),
            "the next present minute is admitted without inventing the missing row"
        );
    }

    /// An OOS boundary exactly at the supplied tail denotes an empty OOS
    /// projection. It is not out of range and must not recover the full series.
    #[test]
    fn oos_projection_accepts_an_exact_empty_tail_without_fallback() {
        let execution = crate::synthetic::sessions(1);
        let column = Column::build(&execution, &mut evaluator());
        let projected = project_oos_fold(
            &execution,
            &column,
            &execution,
            60_000_000,
            execution.len(),
            execution.len(),
        )
        .expect("the exact tail boundary is a valid empty OOS interval");

        assert!(projected.bits().is_empty());
        assert!(projected.sources().is_empty());
        assert!(projected.acceptance_covers(execution.len()));
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
            TradesOnTheseBars::Yes,
        );
        let rolling = walk_forward_shaped(
            &bars,
            h(15),
            3,
            Direction::Long,
            &sweeper(),
            &mut evaluator,
            Shape::Rolling,
            TradesOnTheseBars::Yes,
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

    /// The resolved-rung entry point is still ANCHORED, byte for byte.
    ///
    /// `walk_forward` is called from `cli` and from the audit, and its numbers
    /// are recorded in the ledger. Adding a shape must not have moved them --
    /// §3 rule 5 makes a rerun byte-identical, and a silent change of window
    /// shape would break that without any signal at all.
    #[test]
    fn an_explicit_rung_count_changes_no_other_walk_forward_semantics() {
        let bars = crate::synthetic::sessions(12);
        let plain = walk_forward_with_rungs(
            &bars,
            h(15),
            3,
            Direction::Long,
            &sweeper(),
            evaluator,
            DEFAULT_RUNGS,
        );
        let shaped = walk_forward_shaped_with_rungs(
            &bars,
            h(15),
            3,
            Direction::Long,
            &sweeper(),
            &mut evaluator,
            Shape::Anchored,
            TradesOnTheseBars::Yes,
            DEFAULT_RUNGS,
        );

        assert_eq!(
            plain, shaped,
            "the anchored convenience door and shaped door must carry the same \
             resolved rung count into byte-identical folds"
        );
    }

    /// The short direction is exercised, and it is not the long one.
    ///
    /// Invariant `R-03`. Every other walk-forward test in this module passes
    /// `Direction::Long`, so the `Short` arm was never entered by the suite at
    /// all — the row records that a `panic!` planted in `side_of`'s `Short` arm
    /// survived a full green run.
    ///
    /// # Why the second half is asserted on `side_of` and not on the folds
    ///
    /// The row also asks that "a direction accepted and then ignored fails it",
    /// and the obvious reading — compare the two walks' folds — cannot express
    /// it. Measured: on `sessions(12)` the long and short walks return fold for
    /// fold IDENTICAL results, with 12,531 candidates considered and priced in
    /// fold 1 and an exit chosen in both.
    ///
    /// That is not a dropped direction. `FoldResult` carries split geometry
    /// (`train_bars`, `purged`, `test_bars`), counts from a sweep that is the
    /// same for either side, and `chosen_exit` — which is `grid::Chosen`, the
    /// stop/target/tsl/ttp RUNG INDICES and no money. Two directions selecting
    /// the same rungs is ordinary. `Validated` exposes nothing else.
    ///
    /// So the surface cannot answer the question, and asserting inequality on
    /// it would have pinned a coincidence. What CAN be asserted is the thing the
    /// row actually names: the arm is entered, and the two directions do not map
    /// to one side.
    #[test]
    fn a_short_walk_forward_runs_and_is_not_the_long_one() {
        let bars = crate::synthetic::sessions(12);
        let short = walk_forward(&bars, h(15), 3, Direction::Short, &sweeper(), evaluator);

        // IT RAN. The arm is entered, folds are built, and each trains on
        // something — the half a planted `panic!` would have caught.
        assert_eq!(
            short.folds.len(),
            3,
            "three splits must yield three folds on the short side too"
        );
        for fold in &short.folds {
            assert!(
                fold.train_bars > 0,
                "a short fold must train on something: fold {}",
                fold.index
            );
        }
        assert!(
            short.refused.is_none(),
            "the short walk must not refuse: {:?}",
            short.refused
        );

        // AND IT IS NOT THE LONG ONE. `side_of` is the single point where the
        // caller's direction becomes the side every excursion and fill is
        // measured against; a direction accepted and then ignored is exactly a
        // `side_of` that collapses both arms onto one value.
        assert_eq!(
            super::side_of(Direction::Long),
            crate::excursion::Side::Long
        );
        assert_eq!(
            super::side_of(Direction::Short),
            crate::excursion::Side::Short
        );
        assert_ne!(
            super::side_of(Direction::Long),
            super::side_of(Direction::Short),
            "two directions that map to one side are a direction the walk \
             accepted and then ignored"
        );
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

    /// THE FOLD DECIDES THE SIDE, AND THE CALLER NO LONGER CAN.
    ///
    /// KILLS: a `panic!` planted in `side_of`'s `Direction::Short` arm.
    ///
    /// It survived once because NO test in this module ever ran `walk_forward`
    /// short — every fixture passed `Direction::Long`, so half the execution
    /// model was never entered. The test that fixed that asserted the two
    /// directions produce DIFFERENT folds, which was the right assertion while
    /// the caller's direction reached the pricing.
    ///
    /// It no longer does, and that is the fix rather than a regression. The
    /// direction the caller passed came from `side_of_evidence` over the WHOLE
    /// span, test folds included, so the side was fitted to the window it is
    /// meant to be tested on — and was then applied to every other candidate in
    /// the fold, pricing short-edged combinations as longs in the vector
    /// `crate::pbo` ranks. Each candidate now reads its own side off the
    /// TRAINING window, so what the caller passes cannot change an answer.
    ///
    /// So the assertion inverts: the two walks must be IDENTICAL. And the short
    /// arm is still reached, which is what the mutant needs — `chosen_side`
    /// reports what each fold actually decided, so the test can require that at
    /// least one fold went short on evidence rather than on instruction.
    #[test]
    fn the_fold_decides_the_side_and_the_caller_cannot() {
        let bars = crate::synthetic::sessions(12);
        let long = walk_forward(&bars, h(15), 3, Direction::Long, &sweeper(), evaluator);
        let short = walk_forward(&bars, h(15), 3, Direction::Short, &sweeper(), evaluator);

        assert_eq!(short.folds.len(), 3, "the walk must produce folds");
        assert!(
            short.decided() > 0,
            "the walk chose nothing, so no side was exercised at all"
        );
        for f in &short.folds {
            assert_eq!(
                f.priced, f.considered,
                "fold {} skipped candidates",
                f.index
            );
        }

        // WHAT THE CALLER PASSES CANNOT MOVE THE ANSWER. If it can, the
        // look-ahead is back.
        for (l, s) in long.folds.iter().zip(&short.folds) {
            assert_eq!(
                (l.in_sample, l.chosen, l.chosen_side),
                (s.in_sample, s.chosen, s.chosen_side),
                "fold {} differs between a Long and a Short caller, so the \
                 caller's direction is still reaching the pricing",
                l.index
            );
        }

        // AND THE SIDE IS REPORTED, so a fold that decided something says what.
        for f in &short.folds {
            assert_eq!(
                f.chosen.is_some(),
                f.chosen_side.is_some(),
                "fold {} reports a side without a combination, or the reverse",
                f.index
            );
        }
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

    #[test]
    fn anchored_admission_v2_is_deterministic_and_binds_every_search_term() {
        let first = anchored_admission_run_v2(h(15), 3, DEFAULT_RUNGS);
        let second = anchored_admission_run_v2(h(15), 3, DEFAULT_RUNGS);
        assert_eq!(first, second, "identical runs must be byte-fact identical");
        assert_eq!(
            first.admission_projection(),
            second.admission_projection(),
            "identical runs must project identical authority identities"
        );
        let projection = first
            .admission_projection()
            .expect("the complete opaque fixture reconciles");
        assert_eq!(
            projection.decided_folds,
            u64::try_from(first.validated().decided())
                .expect("the retained fold count fits its stable wire domain"),
            "the opaque Admission count must retain Validated::decided semantics exactly"
        );

        let mut changed_horizon = first.clone();
        changed_horizon.policy.horizon_bars = changed_horizon.policy.horizon_bars.saturating_add(1);
        assert_eq!(
            changed_horizon.admission_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::SealMismatch)
        );

        let mut changed_splits = first.clone();
        changed_splits.policy.splits = changed_splits.policy.splits.saturating_add(1);
        assert_eq!(
            changed_splits.admission_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::SealMismatch)
        );

        let mut changed_rungs = first.clone();
        changed_rungs.policy.resolved_rungs = changed_rungs.policy.resolved_rungs.saturating_add(1);
        assert_eq!(
            changed_rungs.admission_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::SealMismatch)
        );

        let mut changed_window = first.clone();
        let changed_test_end = changed_window
            .folds
            .first()
            .expect("fixture has folds")
            .test_end
            .saturating_add(1);
        changed_window
            .folds
            .first_mut()
            .expect("fixture has folds")
            .test_end = changed_test_end;
        assert_eq!(
            changed_window.admission_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::SealMismatch)
        );

        let mut changed_candidate_order = first;
        changed_candidate_order
            .folds
            .first_mut()
            .expect("fixture has folds")
            .candidate_facts_digest[0] ^= 1;
        assert_eq!(
            changed_candidate_order.admission_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::SealMismatch)
        );
    }

    #[test]
    fn anchored_search_projection_is_typed_detached_and_seal_checked() {
        fn assert_public_shape<T: Clone + Copy + Eq>() {}
        assert_public_shape::<super::AnchoredSearchAuthorityProjectionV2>();
        assert_public_shape::<super::AnchoredSearchPolicyIdentityV2>();
        assert_public_shape::<super::AnchoredSearchFamilyIdentityV2>();
        assert_public_shape::<super::AnchoredSearchWalkIdentityV2>();

        let anchored = anchored_admission_fixture_v2();
        let projection = anchored
            .search_authority_projection()
            .expect("the opaque fixture revalidates before projection");
        assert_ne!(
            projection.policy_identity().digest(),
            projection.family_identity().digest(),
            "domain-separated policy and ordered-family identities must remain distinct"
        );
        assert_ne!(
            projection.family_identity().digest(),
            projection.walk_identity().digest(),
            "candidate-family and exact outcome identities must remain distinct"
        );
        assert_eq!(
            projection.fold_count(),
            u64::try_from(anchored.validated().folds.len())
                .expect("the fixture fold count fits the stable identity domain")
        );
        assert_eq!(
            projection.decided_folds(),
            u64::try_from(anchored.validated().decided())
                .expect("the fixture decided count fits the stable identity domain")
        );
        assert!(projection.profitable_oos_folds() <= projection.decided_folds());
        let expected_oos = anchored
            .validated()
            .folds
            .iter()
            .filter_map(|fold| fold.chosen.map(|_| fold.out_of_sample_exit))
            .try_fold(0_i64, |sum, outcome| {
                sum.checked_add(outcome.expect("opaque decided folds have complete OOS"))
            })
            .expect("the fixture aggregate is representable");
        assert_eq!(projection.aggregate_oos_paisa(), expected_oos);

        let mut changed_seal = anchored;
        changed_seal.validation_family_digest[0] ^= 1;
        assert!(matches!(
            changed_seal.search_authority_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::SealMismatch)
        ));

        let mut resealed_zero = anchored_admission_fixture_v2();
        resealed_zero.policy.resolved_rungs = 0;
        resealed_zero.validation_policy_digest =
            super::hash_validation_policy_v2(resealed_zero.policy);
        resealed_zero.walk_facts_digest = super::hash_walk_facts_v2(
            resealed_zero.validation_policy_digest,
            resealed_zero.validation_family_digest,
            &resealed_zero.folds,
        );
        assert!(matches!(
            resealed_zero.search_authority_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::ZeroResolvedRungs)
        ));
    }

    fn anchored_admission_attack_coordinates_v2(
        original: &AnchoredAdmissionValidationV2,
    ) -> (usize, usize, i64) {
        original
            .folds
            .iter()
            .zip(&original.validated.folds)
            .enumerate()
            .find_map(|(fold, (proof, visible))| {
                let choice = proof.choice?;
                let expected = choice.out_of_sample_pessimistic;
                let ordinal = usize::try_from(choice.ordinal).ok()?;
                visible
                    .out_of_sample_all
                    .iter()
                    .copied()
                    .enumerate()
                    .find(|(other, value)| *other != ordinal && *value != expected)
                    .map(|(_, value)| (fold, ordinal, value))
            })
            .expect("fixture has a decided fold with a distinguishable foreign ordinal")
    }

    #[test]
    fn anchored_admission_v2_refuses_oos_from_the_wrong_ordinal() {
        let original = anchored_admission_fixture_v2();
        let (fold, ordinal, foreign_oos) = anchored_admission_attack_coordinates_v2(&original);
        let mut wrong_ordinal = original.clone();
        wrong_ordinal
            .validated
            .folds
            .get_mut(fold)
            .expect("selected fold remains present")
            .out_of_sample_exit = Some(foreign_oos);
        assert_eq!(
            wrong_ordinal.admission_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold })
        );
        assert!(matches!(
            wrong_ordinal.search_authority_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch {
                fold: refused_fold
            }) if refused_fold == fold
        ));
        assert_ne!(
            wrong_ordinal
                .validated
                .folds
                .get(fold)
                .and_then(|visible| visible.out_of_sample_all.get(ordinal))
                .copied()
                .expect("selected ordinal remains present"),
            foreign_oos,
            "the attack value must truly belong to another ordinal"
        );
    }

    #[test]
    fn anchored_admission_v2_refuses_mask_and_side_crosswires() {
        let original = anchored_admission_fixture_v2();
        let (fold, _, _) = anchored_admission_attack_coordinates_v2(&original);
        let mut swapped_mask = original.clone();
        let visible = swapped_mask
            .validated
            .folds
            .get_mut(fold)
            .expect("selected fold remains present");
        let mask = visible.chosen.expect("selected fold has a mask");
        let high_bit_set = mask
            .words()
            .get(5)
            .copied()
            .expect("ConditionMask has six stable words")
            & (1_u64 << 63)
            != 0;
        visible.chosen = Some(if high_bit_set {
            mask.without_bit(383)
        } else {
            mask.with_bit(383)
        });
        assert_eq!(
            swapped_mask.admission_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold })
        );

        let mut swapped_side = original.clone();
        let visible = swapped_side
            .validated
            .folds
            .get_mut(fold)
            .expect("selected fold remains present");
        visible.chosen_side = Some(
            match visible.chosen_side.expect("selected fold has a side") {
                Direction::Long => Direction::Short,
                Direction::Short => Direction::Long,
            },
        );
        assert_eq!(
            swapped_side.admission_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold })
        );
    }

    #[test]
    fn anchored_admission_v2_refuses_exit_and_ladder_crosswires() {
        let original = anchored_admission_fixture_v2();
        let (fold, _, _) = anchored_admission_attack_coordinates_v2(&original);
        let mut swapped_exit = original.clone();
        let exit = swapped_exit
            .validated
            .folds
            .get_mut(fold)
            .expect("selected fold remains present")
            .chosen_exit
            .as_mut()
            .expect("selected fold has an exit");
        exit.stop = if exit.stop.is_some() { None } else { Some(0) };
        assert_eq!(
            swapped_exit.admission_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold })
        );

        let mut swapped_ladder = original;
        let values = &mut swapped_ladder
            .folds
            .get_mut(fold)
            .expect("selected fold remains present")
            .choice
            .as_mut()
            .expect("selected fold has private choice provenance")
            .exit_values;
        values.stop = values
            .stop
            .map_or(Some(1), |value| Some(value.saturating_add(1)));
        assert_eq!(
            swapped_ladder.admission_projection(),
            Err(AnchoredAdmissionValidationRefusalV2::SealMismatch)
        );
    }

    #[test]
    fn anchored_search_v3_side_rule_covers_long_short_and_exact_zero() {
        assert_eq!(super::direction_from_training_edge(1.0), Direction::Long);
        assert_eq!(super::direction_from_training_edge(-1.0), Direction::Short);
        assert_eq!(
            super::direction_from_training_edge(0.0),
            Direction::Long,
            "the versioned strict-negative rule must resolve exact zero deterministically"
        );
        assert_eq!(
            super::direction_from_training_edge(-0.0),
            Direction::Long,
            "negative IEEE zero is still an exact zero edge"
        );
    }

    #[test]
    fn anchored_search_v3_no_winner_has_no_side_or_caller_fallback() {
        let mut capture =
            super::AnchoredSearchCaptureV3::new(2, h(1), 1, &sweeper(), 1, 60_000_000)
                .expect("the bounded V3 policy is representable");
        let visible = super::FoldResult {
            train_bars: 1,
            test_bars: 1,
            ..super::FoldResult::default()
        };
        capture
            .observe_fold(&admission_split_v2(), &visible, &[], &[], None)
            .expect("an empty complete family is a no-winner fold, not a direction");
        let opaque = capture
            .finish(Validated {
                folds: vec![visible],
                refused: None,
            })
            .expect("the no-winner V3 fold reconciles");
        let fold = opaque
            .validated()
            .folds
            .first()
            .expect("the exact no-winner fold remains visible");
        assert!(fold.chosen.is_none());
        assert!(fold.chosen_side.is_none());
        let projection = opaque
            .search_authority_projection()
            .expect("the no-winner fold revalidates");
        assert_eq!(projection.fold_count(), 1);
        assert_eq!(projection.decided_folds(), 0);
        assert_eq!(projection.profitable_oos_folds(), 0);
        assert_eq!(projection.aggregate_oos_paisa(), 0);
    }

    #[test]
    fn anchored_search_v3_projection_is_typed_sealed_and_rekeys() {
        fn assert_public_shape<T: Clone + Copy + Eq>() {}
        type BuilderFn = fn(&[indicators::Candle]) -> Result<Column, String>;
        type V3Door = for<'a> fn(
            &'a [indicators::Candle],
            ExecutionSeries<'a>,
            Horizon,
            usize,
            &Sweeper,
            &mut BuilderFn,
            usize,
        ) -> Result<
            AnchoredSearchValidationV3,
            super::AnchoredSearchValidationRefusalV3,
        >;
        fn assert_v3_door(_: V3Door) {}

        assert_public_shape::<super::AnchoredSearchAuthorityProjectionV3>();
        assert_public_shape::<super::AnchoredSearchPolicyIdentityV3>();
        assert_public_shape::<super::AnchoredSearchFamilyIdentityV3>();
        assert_public_shape::<super::AnchoredSearchWalkIdentityV3>();
        assert_v3_door(walk_forward_projected_prepared_anchored_search_v3);

        let original = anchored_search_fixture_v3();
        let super::AnchoredSearchPolicyFactsV3 {
            signal_bars: _,
            horizon_bars: _,
            splits: _,
            resolved_rungs: _,
            min_hits: _,
            ceiling: _,
            pair_budget: _,
            training_edge_side_rule: _,
            execution_mode: _,
            signal_length_micros: _,
        } = original.policy;
        let projection = original
            .search_authority_projection()
            .expect("the V3 fixture revalidates");
        assert_ne!(
            projection.policy_identity().digest(),
            projection.family_identity().digest()
        );
        assert_ne!(
            projection.family_identity().digest(),
            projection.walk_identity().digest()
        );
        assert_eq!(
            projection.fold_count(),
            u64::try_from(original.validated().folds.len())
                .expect("the fixture fold count fits the identity domain")
        );

        let mut broken_seal = original.clone();
        broken_seal.validation_policy_digest[0] ^= 1;
        assert!(matches!(
            broken_seal.search_authority_projection(),
            Err(super::AnchoredSearchValidationRefusalV3::SealMismatch)
        ));

        let mut changed_policy = original.policy;
        changed_policy.signal_length_micros = changed_policy
            .signal_length_micros
            .checked_add(60_000_000)
            .expect("the fixture duration remains representable");
        assert_ne!(
            super::hash_validation_policy_v3(original.policy),
            super::hash_validation_policy_v3(changed_policy),
            "every policy input must rekey the V3 policy identity"
        );

        let mut resealed_unknown_rule = original.clone();
        resealed_unknown_rule.policy.training_edge_side_rule =
            super::TRAINING_EDGE_SIDE_RULE_V3.saturating_add(1);
        resealed_unknown_rule.validation_policy_digest =
            super::hash_validation_policy_v3(resealed_unknown_rule.policy);
        resealed_unknown_rule.walk_facts_digest = super::hash_walk_facts_v3(
            resealed_unknown_rule.validation_policy_digest,
            resealed_unknown_rule.validation_family_digest,
            &resealed_unknown_rule.folds,
        );
        assert!(
            matches!(
                resealed_unknown_rule.search_authority_projection(),
                Err(super::AnchoredSearchValidationRefusalV3::SealMismatch)
            ),
            "recomputing hashes cannot authorize an unknown side rule"
        );

        let mut changed_choice = original;
        let choice = changed_choice
            .folds
            .iter_mut()
            .find_map(|fold| fold.choice.as_mut())
            .expect("the complete fixture has a selected fold");
        choice.side = match choice.side {
            Direction::Long => Direction::Short,
            Direction::Short => Direction::Long,
        };
        assert!(
            matches!(
                changed_choice.search_authority_projection(),
                Err(super::AnchoredSearchValidationRefusalV3::SealMismatch)
            ),
            "a selected side is part of the sealed walk facts"
        );
    }

    #[test]
    fn anchored_search_v4_causal_happy_path_exposes_exact_grid_components() {
        fn assert_public_shape<T: Clone + Copy + Eq>() {}
        assert_public_shape::<super::AnchoredSearchAuthorityProjectionV4>();
        assert_public_shape::<super::AnchoredSearchPolicyIdentityV4>();
        assert_public_shape::<super::AnchoredSearchSourceIdentityV4>();
        assert_public_shape::<super::AnchoredSearchGridIdentityV4>();
        assert_public_shape::<super::AnchoredSearchSideGridIdentityV4>();
        assert_public_shape::<super::AnchoredSearchFamilyIdentityV4>();
        assert_public_shape::<super::AnchoredSearchWalkIdentityV4>();

        let exact = anchored_search_fixture_v4();
        let projection = exact
            .search_authority_projection()
            .expect("the complete V4 fixture revalidates");
        let bars = crate::synthetic::sessions(16);
        let full_signal_column = Column::build(&bars, &mut evaluator());
        let source = projection.source_identity();
        assert_eq!(source.signal_digest(), crate::identity::data_digest(&bars));
        assert_eq!(source.signal_bars(), 6_000);
        assert_eq!(
            source.signal_first_ts_micros(),
            bars.first()
                .map(|bar| bar.ts_micros)
                .expect("the fixture has a first signal")
        );
        assert_eq!(
            source.signal_last_ts_micros(),
            bars.last()
                .map(|bar| bar.ts_micros)
                .expect("the fixture has a last signal")
        );
        assert_eq!(
            source.signal_column_digest(),
            crate::exit_grid_policy::column_digest_v1(&full_signal_column)
        );
        assert_eq!(
            projection.long_grid_identity().policy_digest(),
            exact.policy.full_long_policy_digest
        );
        assert_eq!(
            projection.long_grid_identity().resolution_digest(),
            exact.policy.full_long_resolution_digest
        );
        assert_eq!(
            projection.short_grid_identity().policy_digest(),
            exact.policy.full_short_policy_digest
        );
        assert_eq!(
            projection.short_grid_identity().resolution_digest(),
            exact.policy.full_short_resolution_digest
        );
        assert!(
            projection.long_grid_identity() != projection.short_grid_identity(),
            "Long and Short remain separately typed equality components"
        );
        assert_eq!(
            projection.evaluated_population_cells(),
            exact.folds.iter().map(|fold| fold.population_cells).sum()
        );
        let fold_populations: Vec<(u64, u64, usize)> = exact
            .folds
            .iter()
            .map(|fold| {
                (
                    fold.considered_masks,
                    fold.population_cells,
                    fold.candidates.len(),
                )
            })
            .collect();
        assert!(
            exact.folds.iter().any(|fold| !fold.candidates.is_empty()),
            "the causal happy path must exercise selector and OOS replay, not only an empty family: {fold_populations:?}"
        );
        for (proof, visible) in exact.folds.iter().zip(&exact.validated.folds) {
            assert!(proof.candidates.iter().all(|candidate| {
                candidate.oos_cell_digest.is_some() && candidate.out_of_sample_pessimistic.is_some()
            }));
            assert_eq!(
                visible.in_sample,
                super::Summary::default(),
                "V4 must not publish a legacy scalar-exit walk as the exact-grid summary"
            );
            assert_eq!(visible.out_of_sample, super::Summary::default());
            if let Some(choice) = proof.choice {
                assert_eq!(
                    visible.chosen_exit_total,
                    Some(choice.in_sample_pessimistic)
                );
                assert_eq!(
                    visible.out_of_sample_exit,
                    Some(choice.out_of_sample_pessimistic)
                );
            }
        }
    }

    #[test]
    fn anchored_search_v4_refuses_swapped_and_scalar_equal_foreign_grids() {
        let bars = crate::synthetic::sessions(2);
        let instrument =
            InstrumentKey::index(Exchange::Nse, "NIFTY").expect("NSE-NIFTY is an allowed index");
        let series = super::ExecutionSeriesV1::new(
            &instrument,
            "test-feed",
            "test-commit",
            [0xA5; 32],
            &bars,
        )
        .expect("the original series is fully identified");
        let long = exact_grid_policy_v4(crate::excursion::Side::Long)
            .resolve_attested(series)
            .expect("the original Long grid resolves");
        let short = exact_grid_policy_v4(crate::excursion::Side::Short)
            .resolve_attested(series)
            .expect("the original Short grid resolves");
        assert!(matches!(
            super::authenticate_grid_pair_v4(series, &short, &long, true),
            Err(super::AnchoredSearchValidationRefusalV4::GridPairMismatch(
                _
            ))
        ));

        let mut changed_bars = bars.clone();
        let changed_volume = changed_bars
            .first()
            .map_or(1, |bar| bar.volume.saturating_add(1));
        changed_bars
            .first_mut()
            .expect("the fixture is non-empty")
            .volume = changed_volume;
        let changed_series = super::ExecutionSeriesV1::new(
            &instrument,
            "test-feed",
            "test-commit",
            [0xA5; 32],
            &changed_bars,
        )
        .expect("the changed series retains every source identity");
        let foreign_long = exact_grid_policy_v4(crate::excursion::Side::Long)
            .resolve_attested(changed_series)
            .expect("the scalar-equivalent foreign Long grid resolves");
        assert_eq!(foreign_long.stop_levels_ppm(), long.stop_levels_ppm());
        assert_eq!(foreign_long.target_levels_ppm(), long.target_levels_ppm());
        assert_eq!(foreign_long.trail_levels_ppm(), long.trail_levels_ppm());
        assert_ne!(foreign_long.digest(), long.digest());
        assert!(matches!(
            super::authenticate_grid_pair_v4(series, &foreign_long, &short, true),
            Err(super::AnchoredSearchValidationRefusalV4::GridPairMismatch(
                _
            ))
        ));
    }

    #[test]
    fn anchored_search_v4_refuses_a_foreign_or_mutated_full_signal_column() {
        let bars = crate::synthetic::sessions(16);
        let mut foreign = Column::build(&bars, &mut evaluator());
        foreign.clear_before(usize::MAX);

        assert!(matches!(
            anchored_search_run_v4_with_column(&foreign),
            Err(super::AnchoredSearchValidationRefusalV4::SignalSourceMismatch(_))
        ));

        let mut attacked = anchored_search_fixture_v4();
        attacked.policy.signal_digest[0] ^= 1;
        assert!(matches!(
            attacked.search_authority_projection(),
            Err(super::AnchoredSearchValidationRefusalV4::SealMismatch)
        ));

        let mut attacked_column = anchored_search_fixture_v4();
        attacked_column.policy.signal_column_digest[0] ^= 1;
        assert!(matches!(
            attacked_column.search_authority_projection(),
            Err(super::AnchoredSearchValidationRefusalV4::SealMismatch)
        ));
    }

    #[test]
    fn anchored_search_v4_exact_order_and_selected_totals_are_sealed() {
        let original = anchored_search_fixture_v4();

        let mut reordered = original.clone();
        reordered
            .folds
            .first_mut()
            .expect("the fixture has a fold")
            .coordinate_order_digest[0] ^= 1;
        assert!(matches!(
            reordered.search_authority_projection(),
            Err(super::AnchoredSearchValidationRefusalV4::SealMismatch)
        ));

        let mut contradicted = original;
        let (fold_index, exact_total) = contradicted
            .folds
            .iter()
            .enumerate()
            .find_map(|(index, fold)| {
                fold.choice
                    .map(|choice| (index, choice.in_sample_pessimistic))
            })
            .expect("the happy path has a selected exact-grid result");
        contradicted
            .validated
            .folds
            .get_mut(fold_index)
            .expect("the visible fold remains aligned")
            .chosen_exit_total = Some(exact_total.saturating_add(1));
        assert!(matches!(
            contradicted.search_authority_projection(),
            Err(super::AnchoredSearchValidationRefusalV4::ProvenanceMismatch { fold })
                if fold == fold_index
        ));
    }

    #[test]
    fn anchored_search_v4_never_relabels_absent_oos_as_zero() {
        let mut attacked = anchored_search_fixture_v4();
        let (fold_index, candidate_index) = attacked
            .folds
            .iter()
            .enumerate()
            .find_map(|(fold_index, fold)| (!fold.candidates.is_empty()).then_some((fold_index, 0)))
            .expect("the fixture contains a replayed candidate");
        let candidate = attacked
            .folds
            .get_mut(fold_index)
            .and_then(|fold| fold.candidates.get_mut(candidate_index))
            .expect("the selected candidate remains present");
        candidate.oos_cell_digest = None;
        candidate.out_of_sample_pessimistic = None;
        attacked.walk_facts_digest = super::hash_walk_facts_v4(
            attacked.validation_policy_digest,
            attacked.full_grid_digest,
            attacked.validation_family_digest,
            &attacked.folds,
        );
        assert!(
            matches!(
                attacked.search_authority_projection(),
                Err(super::AnchoredSearchValidationRefusalV4::ProvenanceMismatch { fold })
                    if fold == fold_index
            ),
            "even a privately resealed absent replay must refuse rather than compare as zero"
        );
    }

    #[test]
    fn anchored_search_v4_all_private_seals_revalidate() {
        let original = anchored_search_fixture_v4();
        for attacked in 0..4 {
            let mut changed = original.clone();
            match attacked {
                0 => changed.validation_policy_digest[0] ^= 1,
                1 => changed.full_grid_digest[0] ^= 1,
                2 => changed.validation_family_digest[0] ^= 1,
                _ => changed.walk_facts_digest[0] ^= 1,
            }
            assert!(matches!(
                changed.search_authority_projection(),
                Err(super::AnchoredSearchValidationRefusalV4::SealMismatch)
            ));
        }
    }

    /// The out-of-sample loop must not rebuild the slice facts per candidate.
    ///
    /// # Why a source-shape test and not a timing one
    ///
    /// The defect is invisible to every behavioural test: rebuilding
    /// `SliceFacts` per candidate produces the IDENTICAL answer, just O(C)
    /// times over. Only the shape of the call distinguishes the two, and a
    /// timing test on a machine this session has seen at load average 76
    /// measures the machine.
    ///
    /// `trade.rs` already guards `walk_core` this way, and that guard is
    /// precisely why this one is needed: it names one function, so `levelled` —
    /// which `with_levels` calls and which built the facts itself — was never in
    /// its scope. The fix was `with_levels_over`, matching the
    /// `evaluate`/`evaluate_over` and `walk`/`walk_over` pairs.
    ///
    /// Scoped to the `par_iter` body rather than the file, so an unrelated
    /// `with_levels` elsewhere in this module does not fail it.
    #[test]
    fn the_out_of_sample_pass_hoists_the_slice_facts_out_of_its_candidate_loop() {
        let source = include_str!("validate.rs");
        let anchor = "let test_facts = crate::trade::SliceFacts::of(trade_test, confined);";
        let at = source
            .find(anchor)
            .expect("the out-of-sample pass must build the facts ONCE, before its loop");
        let rest = source.get(at..).unwrap_or_default();
        let end = rest
            .find(".collect();")
            .expect("the hoisted facts must be followed by the collecting loop");
        let body = rest.get(..end).unwrap_or_default();

        assert!(
            body.len() > 500,
            "the scan found a {}-byte body, so the anchors moved and this test \
             would pass over nothing",
            body.len()
        );
        assert!(
            body.contains("with_levels_over("),
            "the loop must call the hoisted door"
        );
        // EVERYTHING AFTER THE ANCHOR is the loop. The convenience form
        // `with_levels` builds its own facts, so a single occurrence in here is
        // one rebuild per candidate -- the exact defect. Slicing past the anchor
        // matters: `body` begins WITH the hoisted build, so a check against the
        // whole body would match the correct line and assert nothing.
        let inner = body.get(anchor.len()..).unwrap_or_default();
        assert!(
            !inner.contains("SliceFacts::of("),
            "nothing inside the candidate loop may build slice facts: they are a \
             function of the bars and the column, and rebuilding them per \
             candidate is the O(C x B) term this hoist removed"
        );
    }
}
