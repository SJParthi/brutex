//! The `ledger-all` verb: the durable all-rung Step-3 run.
//!
//! # What this connects, and why it had no caller
//!
//! [`crate::all_rung_population_v5`] and [`crate::all_rung_selection_v5`] hold a
//! complete three-stage pipeline -- Population V5, Execution V3, Selection V5 --
//! that commits a DURABLE, reauthenticated ledger for all eight intraday rungs.
//! It is not a second copy of `range-all`. `range-all` sweeps and prints; this
//! writes admission decisions, statistics, lineage and selection rows to disk
//! with receipts, and refuses to continue if any retained root stops
//! authenticating mid-run.
//!
//! Until this module the three entry points had **zero callers, tests
//! included**. That is not the same as untested -- the two files carry fourteen
//! tests -- but every one of them exercises a helper, so the composed chain had
//! never run. The absence was deliberate and it is recorded in the tree: the
//! `expect(dead_code)` on `AnchoredSearchLineageV4Bounds::new` says, in the
//! author's own words, *"the authoritative CLI surface will construct explicit
//! Search V4 bounds in Step 4"*. This module is that surface.
//!
//! # The policy is the operator's, not this file's
//!
//! The chain needs three policy objects, and a census of the workspace found
//! that **every construction of all three lives inside a `#[cfg(test)]`
//! module** -- twenty-five `AdmissionPolicyV1`, fourteen `ExitGridPolicyV1`,
//! every `RankingPolicyV1`. There is no production policy anywhere to reuse,
//! and `AdmissionPolicyDraftV1` alone carries thirty-nine gates that decide
//! what the engine is willing to trade.
//!
//! Copying a fixture's numbers into an operator verb would be the invention
//! `CLAUDE.md` §3 rule 1 forbids, dressed as wiring. So this module takes the
//! rule of construction instead:
//!
//! * A gate whose value the operator has **stated** is set, and
//!   [`ActiveGate`] records the sentence it came from.
//! * A gate whose value is **derived from this run's own arguments** is set,
//!   and the derivation is named.
//! * Every other gate is read from `BRUTEX_ADMIT_<GATE>`, and when one is unset
//!   the run refuses with [`unset_gate_worksheet`] -- every missing gate at
//!   once, each beside the variable that would answer it.
//!
//! # There is no "off"
//!
//! `AdmissionPolicyDraftV1`'s fields are `Option`, which reads like a gate can
//! be disabled by leaving it `None`. **It cannot.** `AdmissionPolicyV1::new`
//! calls `required` on all thirty-nine and refuses an absent one by name, so a
//! run needs every number. This module found that out the way it should be
//! found out -- by running the verb against the real store and reading the
//! refusal -- and the first draft of this file, which set two gates and left
//! thirty-seven `None` believing they were off, could never have constructed a
//! policy at all.
//!
//! Two gates have values today: the `MAX_POINTS` ceiling and the stated
//! three-times reward-to-risk floor. The other thirty-seven are questions only
//! the operator can answer, and until they do this verb refuses rather than
//! guessing -- which is what §3 rule 6 asks of a bound that cannot be met.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use indicators::evaluator::Widths;
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::Sweeper;
use runner::admission::{AdmissionPolicyDraftV1, AdmissionPolicyV1};
use runner::excursion::Side;
use runner::exit_grid_policy::{
    ExecutionResolutionV1, ExitGridPolicyV1, ExitGridSelectorV1, ForcedStopV1, RangeResolutionV1,
    RatioLimitsV1, RationalPercentileV1, RungPlanV1, printed_ohlcv_cost_model_id_v1,
};
use runner::outcome::Horizon;
use runner::topn::{RankingPolicyV1, Weights};

use crate::all_rung_population_v5::{
    AllRungStoredExecutionV3Request, AllRungStoredPopulationV5Request,
    commit_all_rung_stored_execution_v3, commit_all_rung_stored_population_v5,
};
use crate::all_rung_selection_v5::{
    AllRungSelectionV5Request, commit_all_rung_stored_selection_v5,
};
use crate::anchored_search_lineage_v4::AnchoredSearchLineageV4Bounds;
use crate::candidate_universe::CandidateUniverseBoundsV1;
use crate::execution_v3::{ExecutionV3Bounds, ExecutionV3FileBound};
use crate::population_admission_v3::AdmissionV3Bounds;
use crate::population_finalization_v3::PopulationFinalizationV3Bounds;
use crate::population_observations_v1::ObservationAuthorityBoundsV1;
use crate::population_statistics_v2::{
    PopulationStatisticsProcedureV2, PopulationStatisticsV2Bounds,
};
use crate::population_v5::PopulationV5Bounds;
use crate::pre_admission_data::PreAdmissionDataBoundsV1;
use crate::selection_v5::SelectionV5Bounds;
use crate::step3_orchestrator::StoredCandidatePreAdmissionBoundsV1;
use crate::stored::StoredSpanLoadBoundV1;

/// The eight intraday rungs this verb commits, tightest first.
///
/// Eight and not nine: `1day` is stored and never swept, so it is not a rung of
/// the ladder this pipeline walks. The list is duplicated from
/// `all_rung_population_v5`'s own canonical order deliberately -- that one is
/// private to its module, and
/// [`the_rung_order_matches_the_population_chains_own`] fails the build if the
/// two ever disagree, which is cheaper than making the constant public.
pub(crate) const LEDGER_RUNGS: [&str; 8] = [
    "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min",
];

/// One paisa is a hundredth of a rupee, and one NIFTY point is one rupee of
/// index level.
///
/// # Why this constant exists rather than a bare `* 100`
///
/// A stop expressed in POINTS and a stop expressed in PAISA differ by exactly
/// this factor, and the two have been mixed up in this workspace before: a
/// points figure handed to a paisa field ran a stop ladder a hundred times
/// tighter than asked, which does not crash and does not look wrong in a
/// report -- it just silently rejects every trade that would have survived.
/// Naming the conversion gives the mistake somewhere to be caught.
const PAISA_PER_POINT: u64 = 100;

/// A gate this run applies, and the sentence that decided its value.
///
/// The provenance string is not decoration. `CLAUDE.md` §3 rule 1 asks every
/// number to be traceable, and a threshold printed without its source is
/// indistinguishable from one somebody guessed.
pub(crate) struct ActiveGate {
    name: &'static str,
    value: String,
    because: &'static str,
}

/// Everything `ledger-all` needs that the operator names on the command line.
pub(crate) struct LedgerAllRequest<'a> {
    pub(crate) vendor: &'a str,
    pub(crate) from: (u16, u8),
    pub(crate) to: (u16, u8),
    pub(crate) support_ppm: u64,
    pub(crate) max_points: u64,
    pub(crate) root: &'a Path,
}

/// The canonical tree this verb lays out beneath the operator's root.
///
/// # Why the operator names one path and not eighteen
///
/// The chain admits eighteen distinct directories -- one source, one authority
/// parent, eight rung authorities, eight Execution roots and eight Selection
/// roots -- and it refuses if any two of them physically alias. Asking an
/// operator to type eighteen paths correctly is asking for the one typo that
/// makes two rungs share a directory, which the chain would catch but only
/// after a partial write.
///
/// Deriving them from one root cannot produce that collision: the layout is a
/// tree, so two leaves differ by construction. Execution and Selection roots
/// additionally end in their own rung word, because their admission checks the
/// final path component against the field name it was passed as.
pub(crate) struct LedgerTree {
    pub(crate) authority: PathBuf,
    pub(crate) execution: [PathBuf; 8],
    pub(crate) selection: [PathBuf; 8],
}

impl LedgerTree {
    /// Lays out and CREATES the canonical tree beneath `root`.
    ///
    /// # Errors
    ///
    /// Refuses if any directory cannot be created. The chain's own admission
    /// requires every one of these to exist already, so creating them here is
    /// not a convenience -- it is the difference between a verb an operator can
    /// run and one that refuses until they have run `mkdir` eighteen times.
    pub(crate) fn create(root: &Path) -> Result<Self, String> {
        let authority = root.join("authority");
        let execution_parent = root.join("execution");
        let selection_parent = root.join("selection");

        let mut execution = Vec::with_capacity(LEDGER_RUNGS.len());
        let mut selection = Vec::with_capacity(LEDGER_RUNGS.len());
        for rung in LEDGER_RUNGS {
            let authority_rung = authority.join(rung);
            let execution_rung = execution_parent.join(rung);
            let selection_rung = selection_parent.join(rung);
            for path in [&authority_rung, &execution_rung, &selection_rung] {
                std::fs::create_dir_all(path)
                    .map_err(|why| format!("cannot create {}: {why}", path.display()))?;
            }
            execution.push(execution_rung);
            selection.push(selection_rung);
        }

        let execution: [PathBuf; 8] = execution
            .try_into()
            .map_err(|_| "the ledger tree did not lay out eight Execution roots".to_owned())?;
        let selection: [PathBuf; 8] = selection
            .try_into()
            .map_err(|_| "the ledger tree did not lay out eight Selection roots".to_owned())?;
        Ok(Self {
            authority,
            execution,
            selection,
        })
    }
}

/// The reward-to-risk floor, in hundredths, that the operator's own rule sets.
///
/// The rule as stated is *"min(win) >= 3x max(loss)"* -- the SMALLEST win
/// against the LARGEST loss, which is a stricter question than a mean-to-mean
/// ratio and deliberately so. `RatioLimitsV1` is denominated in hundredths, so
/// three times is 300.
const STATED_REWARD_RISK_HUNDREDTHS: i64 = 300;

/// The same floor expressed in ppm, for the admission gate that wants it there.
const STATED_REWARD_RISK_PPM: u64 = 3_000_000;

/// How many exit cells one side may price.
///
/// Sixteen thousand against the five-step ladder's 1,089, so the guard bounds a
/// runaway rather than the ladder written beside it.
const EXIT_CELL_CEILING: u64 = 16_384;

/// Builds the exit-grid policy for one side from the operator's stated ceiling.
///
/// # What is stated and what is shape
///
/// The 50-point ceiling and the 3x ratio floor are the operator's, and they go
/// in verbatim. The LADDER -- five percentile steps on each of stop, target and
/// trail -- is not a threshold but a resolution: it decides how finely the grid
/// is searched, not what passes. Five steps on three axes is 125 cells, well
/// inside `max_cells`, and the number is printed in the gate census so it is
/// visible rather than buried.
///
/// # Errors
///
/// Refuses if the percentile ladder, ratio limits or the policy itself reject
/// the values -- each of which names which term it objected to.
pub(crate) fn exit_policy(side: Side) -> Result<ExitGridPolicyV1, String> {
    let mut ladder = Vec::with_capacity(5);
    for step in 1..=5_u32 {
        ladder.push(
            RationalPercentileV1::new(step, 5)
                .map_err(|why| format!("exit ladder step {step}/5 refused: {why:?}"))?,
        );
    }
    ExitGridPolicyV1::new(
        ExecutionResolutionV1::OneMinuteOhlcv,
        RangeResolutionV1::PpmCeiling,
        side,
        RungPlanV1::new(ladder.clone(), ladder.clone(), ladder, 5)
            .map_err(|why| format!("exit rung plan refused: {why:?}"))?,
        // The UPPER ratio bound is left at the widest the type allows. A ceiling
        // on reward-to-risk would discard the best cells in the grid, and no
        // operator rule names one -- only the floor was stated.
        RatioLimitsV1::new(STATED_REWARD_RISK_HUNDREDTHS, i64::MAX, u64::MAX)
            .map_err(|why| format!("exit ratio limits refused: {why:?}"))?,
        // A COMPUTE GUARD, not a policy. It caps how many exit cells one side
        // may price, and the LADDER above is what decides which cells those
        // are. Set to a thousand it refused the ladder it was written beside --
        // `CellLimitExceeded { needed: 1089, max: 1000 }` -- which is a policy
        // silently trimmed by a resource bound, exactly backwards. Sized so the
        // five-step ladder fits with room, and a ladder that outgrows THIS
        // should raise it deliberately rather than lose cells to it.
        EXIT_CELL_CEILING,
        ExitGridSelectorV1::GuaranteedFloor,
        printed_ohlcv_cost_model_id_v1(),
        ForcedStopV1::Disabled,
        u64::MAX,
        u64::MAX,
    )
    .map_err(|why| format!("exit grid policy refused: {why:?}"))
}

/// Resolves one gate per call and remembers the ones nobody answered.
///
/// # Why a collector and not a chain of `?`
///
/// Returning on the first missing gate would name one, the operator would set
/// it, and the next run would name the second -- thirty-seven runs to discover
/// thirty-seven numbers. Collecting every miss lets one refusal print the whole
/// worksheet.
struct GateResolver {
    missing: Vec<&'static str>,
    active: Vec<ActiveGate>,
}

impl GateResolver {
    const fn new() -> Self {
        Self {
            missing: Vec::new(),
            active: Vec::new(),
        }
    }

    /// The knob a gate is read from: `min_support_hits` -> `BRUTEX_ADMIT_...`.
    fn knob_of(name: &str) -> String {
        format!("BRUTEX_ADMIT_{}", name.to_ascii_uppercase())
    }

    /// One gate, from its knob or from a value the operator already stated.
    ///
    /// `stated` is not a default. It is a value with a SOURCE, and `because`
    /// carries that source into the report. A gate with no stated value and no
    /// knob is recorded as missing rather than filled in.
    fn gate<T: std::str::FromStr + std::fmt::Display>(
        &mut self,
        name: &'static str,
        stated: Option<(T, &'static str)>,
    ) -> Option<T> {
        if let Some(raw) = crate::knobs::var(&Self::knob_of(name)) {
            if let Ok(parsed) = raw.trim().parse::<T>() {
                self.active.push(ActiveGate {
                    name,
                    value: parsed.to_string(),
                    because: "set by its BRUTEX_ADMIT_ knob",
                });
                return Some(parsed);
            }
            // A knob that is SET and unparseable is a miss, not a fallback to
            // the stated value. Silently ignoring a typo would apply a
            // threshold the operator did not ask for while their own value sat
            // in the environment unread -- the fallback that hides a failure
            // `CLAUDE.md` §4 bans.
            self.missing.push(name);
            return None;
        }
        let Some((value, because)) = stated else {
            self.missing.push(name);
            return None;
        };
        self.active.push(ActiveGate {
            name,
            value: value.to_string(),
            because,
        });
        Some(value)
    }
}

/// The admission policy, built from what the operator has actually said.
///
/// # Every gate is required, and that is the runner's rule, not this file's
///
/// `AdmissionPolicyDraftV1`'s fields are `Option`, which reads like "off when
/// `None`". They are not: `AdmissionPolicyV1::new` calls `required` on all
/// thirty-nine and refuses an absent one by name. There is no partial policy
/// and no gate that can be left unanswered -- a run needs all thirty-nine
/// numbers, and thirty-seven of them are nowhere in this repository, its
/// documents or its decision ledger.
///
/// So this reads each from `BRUTEX_ADMIT_<GATE>` and, when any is unset,
/// refuses with the whole worksheet rather than the first name the runner
/// happened to check.
///
/// `verb` is stamped on the two events this resolution emits -- one per run,
/// never one per gate -- so that a log filtered to `cli.ledger` says which verb
/// asked and whether the thirty-nine were answered.
///
/// # Errors
///
/// Names every unresolved gate and the knob that would answer it.
pub(crate) fn admission_policy(
    request: &LedgerAllRequest<'_>,
    verb: &str,
) -> Result<(AdmissionPolicyV1, Vec<ActiveGate>), String> {
    let max_loss_paisa = request
        .max_points
        .checked_mul(PAISA_PER_POINT)
        .ok_or_else(|| format!("MAX_POINTS {} overflows paisa", request.max_points))?;

    let mut r = GateResolver::new();
    let draft = AdmissionPolicyDraftV1 {
        // THE TWO THE OPERATOR HAS STATED. Both carry the sentence they came
        // from, so a reader of the report can check the number against the rule
        // rather than taking it on trust.
        max_worst_trade_loss_paisa: r.gate(
            "max_worst_trade_loss_paisa",
            Some((
                max_loss_paisa,
                "MAX_POINTS argument, at 100 paisa per index point",
            )),
        ),
        min_worst_reward_risk_ppm: r.gate(
            "min_worst_reward_risk_ppm",
            Some((
                STATED_REWARD_RISK_PPM,
                "stated rule: smallest win at least three times the largest loss",
            )),
        ),
        // AND THE THIRTY-SEVEN NOBODY HAS. Each is read from its own knob and
        // named in the refusal when it is not set.
        min_support_hits: r.gate("min_support_hits", None),
        min_independent_sessions: r.gate("min_independent_sessions", None),
        min_trades: r.gate("min_trades", None),
        max_mae_paisa: r.gate("max_mae_paisa", None),
        min_win_rate_ppm: r.gate("min_win_rate_ppm", None),
        min_wilson_win_rate_ppm: r.gate("min_wilson_win_rate_ppm", None),
        min_return_drawdown_ppm: r.gate("min_return_drawdown_ppm", None),
        min_weakest_period_return_paisa: r.gate("min_weakest_period_return_paisa", None),
        max_pbo_ppm: r.gate("max_pbo_ppm", None),
        max_fwer_p_value_ppm: r.gate("max_fwer_p_value_ppm", None),
        max_spa_p_value_ppm: r.gate("max_spa_p_value_ppm", None),
        min_decided_folds: r.gate("min_decided_folds", None),
        max_ambiguous_fill_rate_ppm: r.gate("max_ambiguous_fill_rate_ppm", None),
        max_gap_affected_rate_ppm: r.gate("max_gap_affected_rate_ppm", None),
        max_session_concentration_ppm: r.gate("max_session_concentration_ppm", None),
        max_largest_trade_profit_share_ppm: r.gate("max_largest_trade_profit_share_ppm", None),
        max_drawdown_paisa: r.gate("max_drawdown_paisa", None),
        max_losing_trade_rate_ppm: r.gate("max_losing_trade_rate_ppm", None),
        max_losing_trades: r.gate("max_losing_trades", None),
        min_pessimistic_profit_paisa: r.gate("min_pessimistic_profit_paisa", None),
        min_winning_trades: r.gate("min_winning_trades", None),
        min_average_win_paisa: r.gate("min_average_win_paisa", None),
        max_average_loss_paisa: r.gate("max_average_loss_paisa", None),
        min_profit_factor_ppm: r.gate("min_profit_factor_ppm", None),
        max_consecutive_losing_streak: r.gate("max_consecutive_losing_streak", None),
        min_consecutive_winning_streak: r.gate("min_consecutive_winning_streak", None),
        min_bootstrap_draws: r.gate("min_bootstrap_draws", None),
        min_bootstrap_strategies: r.gate("min_bootstrap_strategies", None),
        min_bootstrap_periods: r.gate("min_bootstrap_periods", None),
        min_pbo_contributing_folds: r.gate("min_pbo_contributing_folds", None),
        max_pbo_unrankable_folds: r.gate("max_pbo_unrankable_folds", None),
        min_profitable_oos_folds: r.gate("min_profitable_oos_folds", None),
        min_oos_pessimistic_return_paisa: r.gate("min_oos_pessimistic_return_paisa", None),
        max_white_reality_p_value_ppm: r.gate("max_white_reality_p_value_ppm", None),
        require_white_reality_rejection: r.gate("require_white_reality_rejection", None),
        max_romano_wolf_p_value_ppm: r.gate("max_romano_wolf_p_value_ppm", None),
        require_romano_wolf_rejection: r.gate("require_romano_wolf_rejection", None),
    };

    if !r.missing.is_empty() {
        // ONE EVENT FOR THE WHOLE WORKSHEET, not one per unanswered gate. The
        // resolution above walks thirty-nine gates; logging inside that walk
        // would be the per-member shape gate 17 exists to refuse, and the count
        // is the fact an operator acts on.
        crate::note(&gates_refused_event(verb, r.missing.len()));
        return Err(unset_gate_worksheet(&r));
    }
    let policy = AdmissionPolicyV1::new(draft).map_err(|why| {
        let refusal = format!("admission policy refused: {why:?}");
        crate::note(&stage_refused_event(verb, "admission-policy", &refusal));
        refusal
    })?;
    crate::note(&gates_resolved_event(verb, r.active.len()));
    Ok((policy, r.active))
}

/// The refusal an operator gets when a gate has no value.
///
/// # Why this is long
///
/// It is the shortest thing that is not a lie. The alternative -- refusing with
/// `MinSupportHits Absent`, which is what the runner says on its own -- is true
/// and useless: it names one of thirty-seven and gives no way to answer it.
fn unset_gate_worksheet(resolver: &GateResolver) -> String {
    let mut why = String::new();
    let _ = writeln!(
        why,
        "{} of {} admission gates have no value, and the runner requires ALL of them.",
        resolver.missing.len(),
        ALL_GATES.len()
    );
    why.push_str(
        "\nThese are thresholds that decide what this engine is willing to trade. Not one\n\
         of them is named in this repository, in docs/, or in the decision ledger -- every\n\
         construction of an admission policy in the workspace is a test fixture. Choosing\n\
         them here would be inventing a trading policy and printing it as though somebody\n\
         had decided it.\n\n\
         Set each as an environment variable and run again:\n\n",
    );
    for name in &resolver.missing {
        let _ = writeln!(why, "  {}=", GateResolver::knob_of(name));
    }
    if !resolver.active.is_empty() {
        why.push_str("\nAlready answered:\n");
        for gate in &resolver.active {
            let _ = writeln!(
                why,
                "  {:<36} {:>12}   {}",
                gate.name, gate.value, gate.because
            );
        }
    }
    why
}

/// Every gate the runner requires, in declaration order.
///
/// Used to size the worksheet -- "31 of 39" means something only if the 39 is
/// the runner's own count. It is a literal list because Rust cannot enumerate a
/// struct's fields at runtime, and
/// [`the_gate_census_names_every_gate_the_draft_carries`] pins the count
/// against the draft this module fills in.
const ALL_GATES: [&str; 39] = [
    "min_support_hits",
    "min_independent_sessions",
    "min_trades",
    "max_mae_paisa",
    "min_worst_reward_risk_ppm",
    "min_win_rate_ppm",
    "min_wilson_win_rate_ppm",
    "min_return_drawdown_ppm",
    "min_weakest_period_return_paisa",
    "max_pbo_ppm",
    "max_fwer_p_value_ppm",
    "max_spa_p_value_ppm",
    "min_decided_folds",
    "max_ambiguous_fill_rate_ppm",
    "max_gap_affected_rate_ppm",
    "max_session_concentration_ppm",
    "max_largest_trade_profit_share_ppm",
    "max_drawdown_paisa",
    "max_worst_trade_loss_paisa",
    "max_losing_trade_rate_ppm",
    "max_losing_trades",
    "min_pessimistic_profit_paisa",
    "min_winning_trades",
    "min_average_win_paisa",
    "max_average_loss_paisa",
    "min_profit_factor_ppm",
    "max_consecutive_losing_streak",
    "min_consecutive_winning_streak",
    "min_bootstrap_draws",
    "min_bootstrap_strategies",
    "min_bootstrap_periods",
    "min_pbo_contributing_folds",
    "max_pbo_unrankable_folds",
    "min_profitable_oos_folds",
    "min_oos_pessimistic_return_paisa",
    "max_white_reality_p_value_ppm",
    "require_white_reality_rejection",
    "max_romano_wolf_p_value_ppm",
    "require_romano_wolf_rejection",
];

/// Prints every gate this run applies and where its value came from.
///
/// Reached only once all thirty-nine resolve, so there is no "not applied" half
/// to print -- an unresolved gate refuses the run in
/// [`unset_gate_worksheet`] instead. What this shows is PROVENANCE: two values
/// carry the sentence that decided them and the rest name the knob they were
/// read from, so nothing in the policy is a number without a source.
pub(crate) fn render_gate_census(out: &mut String, active: &[ActiveGate]) {
    let _ = writeln!(
        out,
        "\nADMISSION GATES -- all {} applied, and where each value came from",
        ALL_GATES.len()
    );
    for gate in active {
        let _ = writeln!(
            out,
            "  {:<36} {:>14}   {}",
            gate.name, gate.value, gate.because
        );
    }
}

/// The target every `ledger-all` and `ledger-v6` event carries.
///
/// # Why one target, and why it begins with `cli.`
///
/// `/backtest/run.json` filters the tail on the `cli` prefix and `/logs` walks
/// the CLI's own directory, so a target under `cli.` reaches both surfaces with
/// no change on the server side. It is not `cli.audit`: that target already
/// means `range-all`'s rung lifecycle, and a second meaning on one target would
/// hand an operator's filter two unrelated runs interleaved.
pub(crate) const LEDGER_TARGET: &str = "cli.ledger";

/// The verb word `ledger-all` stamps on every event it emits.
pub(crate) const LEDGER_ALL_VERB: &str = "ledger-all";

/// The Population V5 stage, as the `stage` field spells it.
const POPULATION_STAGE: &str = "population-v5";
/// The Execution V3 stage, as the `stage` field spells it.
const EXECUTION_STAGE: &str = "execution-v3";
/// The Selection V5 stage, as the `stage` field spells it.
const SELECTION_STAGE: &str = "selection-v5";
/// The support-sizing step, as [`rung_refused_event`]'s `stage` field spells it.
const SIZING_STAGE: &str = "support-sizing";

// WHERE THESE EVENTS ARE ALLOWED TO BE, AND WHY THIS IS THE AFFORDABLE PLACE.
//
// CI gate 17 forbids `telemetry::` outright in `vocab`, `engine`, `indicators`
// and `runner`, because those hold the loops: the sweep evaluates
// `(bits & mask) == mask` per (bar, combination) and the exit grid prices per
// cell. Its rule is not "each call is cheap" -- a filtered emit genuinely is --
// it is "the innermost loop calls nothing at all", because a billion O(1) calls
// is still a billion calls.
//
// `cli` is not on that list and must never be added to one, and the reason is
// structural rather than a favour: **no loop in this file iterates a bar, a
// candidate or a grid cell.** Every loop here is over a FIXED, CHARTER-SIZED
// list -- eight rungs, two families, thirty-nine gates -- so the event count of
// a whole `ledger-all` run is seventeen and of a whole `ledger-v6` run is
// forty-three, whether the span is one month or eighty. That is the
// "per k-level, per instrument, per run" granularity gate 17's own comment
// prescribes as affordable, and it is bounded by constants in this file rather
// than by the operator's data.
//
// WHAT THIS DELIBERATELY CANNOT SEE, stated rather than left for a reader to
// discover. `commit_all_rung_stored_population_v5`, its Execution V3 successor
// and its Selection V5 successor each walk all eight rungs INTERNALLY and
// return one committed value. So `ledger-all` reports three stage boundaries,
// not twenty-four: a per-rung event inside those stages would have to be
// emitted from `all_rung_population_v5.rs`, which is not this file. `ledger-v6`
// drives its own rung loop and therefore does report per rung and per family.
// `CLAUDE.md` §3 rule 6: the coarser half is named, not implied.

/// The event that opens a run, before anything is loaded or created.
///
/// Carries the whole request because a run is found in a log by its span and
/// its feed, and an operator reading `/logs` has no other handle on it.
pub(crate) fn run_started_event<'a>(
    verb: &'a str,
    request: &'a LedgerAllRequest<'a>,
) -> telemetry::Event<'a> {
    telemetry::Event::info(LEDGER_TARGET, "ledger run started")
        .with("verb", verb)
        .with("feed", request.vendor)
        .with("from_year", u64::from(request.from.0))
        .with("from_month", u64::from(request.from.1))
        .with("to_year", u64::from(request.to.0))
        .with("to_month", u64::from(request.to.1))
        .with("support_ppm", request.support_ppm)
        .with("max_points", request.max_points)
        .with("rungs", LEDGER_RUNGS.len())
}

/// The event that closes a run that committed.
///
/// `written` is the count the report already prints -- blocks written rather
/// than byte-identically reused -- so a rerun that is idempotent under §3 rule 5
/// is visible in the log as a zero without anybody opening the ledger.
pub(crate) fn run_finished_event(verb: &str, written: usize) -> telemetry::Event<'_> {
    telemetry::Event::info(LEDGER_TARGET, "ledger run finished")
        .with("verb", verb)
        .with("written_rungs", written)
        .with("rungs", LEDGER_RUNGS.len())
}

/// The event that closes a run that refused, carrying the reason.
///
/// A `Warn` and not an `Info`, so that the default `Info` floor still keeps it
/// and a filter for trouble finds it. `CLAUDE.md` §4 bans a failure that is
/// invisible, and a verb whose only refusal channel is a terminal nobody
/// attached is exactly that.
pub(crate) fn run_refused_event<'a>(verb: &'a str, why: &'a str) -> telemetry::Event<'a> {
    telemetry::Event::warn(LEDGER_TARGET, "ledger run refused")
        .with("verb", verb)
        .with("why", why)
}

/// One stage of the chain opening.
pub(crate) fn stage_started_event<'a>(verb: &'a str, stage: &'a str) -> telemetry::Event<'a> {
    telemetry::Event::info(LEDGER_TARGET, "stage started")
        .with("verb", verb)
        .with("stage", stage)
        .with("rungs", LEDGER_RUNGS.len())
}

/// One stage of the chain closing, with the blocks it actually wrote.
pub(crate) fn stage_finished_event<'a>(
    verb: &'a str,
    stage: &'a str,
    written: usize,
) -> telemetry::Event<'a> {
    telemetry::Event::info(LEDGER_TARGET, "stage finished")
        .with("verb", verb)
        .with("stage", stage)
        .with("written_rungs", written)
        .with("rungs", LEDGER_RUNGS.len())
}

/// One stage refusing, named, so the log says WHICH stage stopped the run.
pub(crate) fn stage_refused_event<'a>(
    verb: &'a str,
    stage: &'a str,
    why: &'a str,
) -> telemetry::Event<'a> {
    telemetry::Event::warn(LEDGER_TARGET, "stage refused")
        .with("verb", verb)
        .with("stage", stage)
        .with("why", why)
}

/// One rung's support threshold, resolved against that rung's own bar count.
///
/// Every number here was computed to BUILD the sweeper and is read rather than
/// derived for the log: `bars` is the span this rung loaded, `min_hits` is what
/// [`crate::min_hits_for`] returned from it, and `support_ppm` is the
/// operator's own argument. An operator watching a long run learns from this
/// line both that the rung's span loaded and what threshold it will be swept at
/// -- the two facts that decide whether the answer will be empty.
pub(crate) fn rung_sized_event<'a>(
    verb: &'a str,
    rung: &'a str,
    bars: usize,
    min_hits: u64,
    support_ppm: u64,
) -> telemetry::Event<'a> {
    telemetry::Event::info(LEDGER_TARGET, "rung support sized")
        .with("verb", verb)
        .with("stage", SIZING_STAGE)
        .with("rung", rung)
        .with("bars", bars)
        .with("min_hits", min_hits)
        .with("support_ppm", support_ppm)
}

/// One rung refusing at a named step, carrying the reason it gave.
pub(crate) fn rung_refused_event<'a>(
    verb: &'a str,
    stage: &'a str,
    rung: &'a str,
    why: &'a str,
) -> telemetry::Event<'a> {
    telemetry::Event::warn(LEDGER_TARGET, "rung refused")
        .with("verb", verb)
        .with("stage", stage)
        .with("rung", rung)
        .with("why", why)
}

/// Every admission gate answered, and the run may proceed.
pub(crate) fn gates_resolved_event(verb: &str, resolved: usize) -> telemetry::Event<'_> {
    telemetry::Event::info(LEDGER_TARGET, "admission gates resolved")
        .with("verb", verb)
        .with("resolved", resolved)
        .with("gates", ALL_GATES.len())
}

/// Gates with no value, counted, so the refusal is in the log and not only in
/// the worksheet the terminal printed.
///
/// The whole worksheet is deliberately NOT a field: it is thirty-seven lines
/// and would be cut at [`telemetry::MAX_STR_VALUE_BYTES`], leaving a truncated
/// list that looks complete. The count is exact and the terminal report carries
/// the names.
pub(crate) fn gates_refused_event(verb: &str, missing: usize) -> telemetry::Event<'_> {
    telemetry::Event::warn(LEDGER_TARGET, "admission gates unset")
        .with("verb", verb)
        .with("missing", missing)
        .with("gates", ALL_GATES.len())
}

/// Runs the durable all-rung Step-3 chain and returns the operator's report.
///
/// # What it writes
///
/// Beneath `request.root`: an `authority/` tree carrying the Candidate,
/// Pre-Admission, Observation, Statistics, Search-lineage, Admission and
/// Finalization ledgers per rung, an `execution/` tree carrying Execution V3
/// parameters, percentiles, dispositions and completions, and a `selection/`
/// tree carrying the ranked Top-25 rows. Every one is reauthenticated before
/// the next stage reads it.
///
/// # Cost
///
/// One span load per rung to size that rung's support threshold -- eight loads
/// for the whole run, outside any loop over bars or candidates. `CLAUDE.md` §3
/// rule 4 bounds five per-operation costs and this is none of them.
pub(crate) fn ledger_all(request: &LedgerAllRequest<'_>) -> String {
    let mut out = String::new();
    out.push_str(crate::STORED_PROVENANCE);
    let _ = writeln!(
        out,
        "\nLEDGER-ALL  {} {:04}-{:02}..{:04}-{:02}  support {} ppm  stop ceiling {} points",
        request.vendor,
        request.from.0,
        request.from.1,
        request.to.0,
        request.to.1,
        request.support_ppm,
        request.max_points
    );
    // THE FIRST EVENT IS BEFORE THE FIRST REFUSAL, ON PURPOSE. A run that
    // refuses on its vendor word never reaches a stage, and an operator whose
    // page showed nothing at all could not tell that from a run that never
    // started. One event here means every run appears in the log.
    crate::note(&run_started_event(LEDGER_ALL_VERB, request));

    match run_chain(request, &mut out) {
        Ok(written) => {
            let _ = writeln!(
                out,
                "\nCOMMITTED. {written} of {} rung Selection blocks were written rather than \
                 byte-identically reused.",
                LEDGER_RUNGS.len()
            );
            crate::note(&run_finished_event(LEDGER_ALL_VERB, written));
        }
        Err(why) => {
            let _ = writeln!(out, "\nrefused: {why}");
            crate::note(&run_refused_event(LEDGER_ALL_VERB, &why));
        }
    }
    out
}

/// The chain itself, lifted out so [`ledger_all`] owns only the report.
///
/// # Errors
///
/// Every refusal is named by the stage that raised it and carries the rung and
/// family it stopped on. There is no arm that continues past a refusal: a
/// partially committed ledger that reported success would be the failure
/// wearing a success's clothes that `CLAUDE.md` §4 bans outright.
fn run_chain(request: &LedgerAllRequest<'_>, out: &mut String) -> Result<usize, String> {
    let vendor = crate::parse_vendor(request.vendor)?;
    let source_root = crate::store_root().map_err(|why| format!("stored source root: {why}"))?;
    let tree = LedgerTree::create(request.root)?;

    let sweepers = build_sweepers(&source_root, vendor, request, LEDGER_ALL_VERB)?;
    let (admission, active_gates) = admission_policy(request, LEDGER_ALL_VERB)?;
    render_gate_census(out, &active_gates);

    let ranking = RankingPolicyV1::new(Weights::equal())
        .map_err(|why| format!("ranking policy refused: {why:?}"))?;

    crate::note(&stage_started_event(LEDGER_ALL_VERB, POPULATION_STAGE));
    let population = commit_population(&Stage1 {
        source_root: source_root.as_path(),
        tree: &tree,
        vendor,
        request,
        sweepers: &sweepers,
        admission: &admission,
    })
    .inspect_err(|why| {
        crate::note(&stage_refused_event(LEDGER_ALL_VERB, POPULATION_STAGE, why));
    })?;
    // WRITTEN VERSUS REUSED, AT EVERY STAGE AND NOT JUST THE LAST. A rerun over
    // an unchanged span should write nothing at all -- that is what §3 rule 5's
    // idempotence means on disk -- and reporting only the Selection count hid
    // the two stages where a spurious rewrite would actually show up first.
    let population_written = population.written_rung_count();
    crate::note(&stage_finished_event(
        LEDGER_ALL_VERB,
        POPULATION_STAGE,
        population_written,
    ));

    crate::note(&stage_started_event(LEDGER_ALL_VERB, EXECUTION_STAGE));
    let execution = commit_execution(population, &tree).inspect_err(|why| {
        crate::note(&stage_refused_event(LEDGER_ALL_VERB, EXECUTION_STAGE, why));
    })?;
    let execution_written = execution.written_rung_count();
    crate::note(&stage_finished_event(
        LEDGER_ALL_VERB,
        EXECUTION_STAGE,
        execution_written,
    ));

    crate::note(&stage_started_event(LEDGER_ALL_VERB, SELECTION_STAGE));
    let selection = commit_selection(execution, &tree, ranking).inspect_err(|why| {
        crate::note(&stage_refused_event(LEDGER_ALL_VERB, SELECTION_STAGE, why));
    })?;
    let selection_written = selection.written_rung_count();
    crate::note(&stage_finished_event(
        LEDGER_ALL_VERB,
        SELECTION_STAGE,
        selection_written,
    ));
    let _ = writeln!(
        out,
        "\nBLOCKS WRITTEN RATHER THAN BYTE-IDENTICALLY REUSED, of {} rungs each\n  \
         Population V5 {population_written}   Execution V3 {execution_written}   \
         Selection V5 {selection_written}",
        LEDGER_RUNGS.len()
    );
    render_winners(out, selection)?;
    Ok(selection_written)
}

/// Prints the ranked winners the run actually selected.
///
/// # Why the report ends here and not at "committed"
///
/// The chain's whole output is two hundred ranked rows -- eight rungs by
/// twenty-five ranks -- and until this function they were written to disk and
/// shown to nobody. A verb that says `COMMITTED` and nothing else asks the
/// operator to go and decode a ledger to find out what it decided, which is the
/// same as not answering.
///
/// Ten and not twenty-five: `visit_canonical` yields all two hundred, and the
/// full set is on disk for anything that wants it. A terminal report that runs
/// to two hundred rows is one nobody reads, and the Top-10 of each rung is the
/// prefix the selector itself treats as the answer.
///
/// # Errors
///
/// Refuses if the retained topology stops authenticating mid-visit -- the rows
/// are read under the same reauthentication every other stage uses, so a
/// partially-read set is a refusal rather than a short table.
fn render_winners(
    out: &mut String,
    selection: crate::all_rung_selection_v5::CommittedStoredAllRungSelectionV5,
) -> Result<(), String> {
    out.push_str(
        "\nTOP 10 BY RUNG -- paisa unless a column says ppm; profit is the PESSIMISTIC fill\n",
    );
    let mut current = String::new();
    selection.into_successor_set()?.visit_canonical(|winner| {
        let row = winner.row();
        let rank = row.rank();
        // THE SELECTOR'S OWN CONSTANT, not a literal ten. `all_rung_selection_v5`
        // already defines what a Top-10 prefix is and its
        // `require_exact_prefix` enforces it; a second 10 written here would be
        // a copy that can drift from the thing it claims to show.
        if u64::from(rank) > crate::all_rung_selection_v5::TOP_TEN_U64 {
            return Ok(());
        }
        let rung = row.rung_seconds();
        if current != rung.to_string() {
            current = rung.to_string();
            let _ = writeln!(
                out,
                "\n  {rung}s\n    {:>4}  {:>9}  {:>6}  {:>8}  {:>8}  {:>7}  {:>6}",
                "rank", "profit", "win%", "worstLoss", "drawdown", "avgWin", "R:R"
            );
        }
        let m = row.metrics();
        let _ = writeln!(
            out,
            "    {:>4}  {:>9}  {:>5}.{}  {:>8}  {:>8}  {:>7}  {:>6}",
            rank,
            m.pessimistic_profit,
            m.win_rate_ppm / 10_000,
            (m.win_rate_ppm / 1_000) % 10,
            m.worst_loss,
            m.drawdown,
            m.average_win,
            // ABSENT, NOT ZERO. `reward_to_risk_ppm` is `None` when there is no
            // losing trade to divide by, and printing that as 0.00 would read
            // as the worst possible ratio when it is the best possible one.
            m.reward_to_risk_ppm.map_or_else(
                || "none".to_owned(),
                |ppm| format!("{}.{:02}", ppm / 1_000_000, (ppm / 10_000) % 100)
            ),
        );
        Ok(())
    })
}

/// Everything the Population V5 stage borrows for the length of its commit.
///
/// # Why a struct and not six parameters
///
/// `clippy::too_many_arguments` is not the reason -- the reason is that five of
/// the six are borrows that must all outlive the request, and gathering them
/// into one value makes that a single lifetime the compiler checks rather than
/// six the reader has to line up by eye.
struct Stage1<'a> {
    source_root: &'a Path,
    tree: &'a LedgerTree,
    vendor: Vendor,
    request: &'a LedgerAllRequest<'a>,
    sweepers: &'a [Sweeper; 8],
    admission: &'a AdmissionPolicyV1,
}

/// Commits the Population V5 stage for all eight rungs.
///
/// # Errors
///
/// Names the stage, the rung and the family that refused. This is where a run
/// against today's store stops: the chain commits NIFTY and then BANKNIFTY for
/// every rung, and the store holds no BANKNIFTY bars at any feed -- so the
/// refusal an operator will actually see reads
/// `all-rung 1min BANKNIFTY refused: ...`. That is the honest answer, and it
/// arrives after the NIFTY half of the first rung has genuinely been computed.
fn commit_population(
    stage: &Stage1<'_>,
) -> Result<crate::all_rung_population_v5::CommittedStoredAllRungPopulationV5, String> {
    let request = stage.request;
    let long_exit = exit_policy(Side::Long)?;
    let short_exit = exit_policy(Side::Short)?;
    let [
        one_minute_sweeper,
        two_minute_sweeper,
        three_minute_sweeper,
        five_minute_sweeper,
        ten_minute_sweeper,
        fifteen_minute_sweeper,
        thirty_minute_sweeper,
        sixty_minute_sweeper,
    ] = stage.sweepers;

    commit_all_rung_stored_population_v5(&AllRungStoredPopulationV5Request {
        source_root: stage.source_root,
        authority_root: stage.tree.authority.as_path(),
        vendor: stage.vendor,
        from: request.from,
        to: request.to,
        one_minute_sweeper,
        two_minute_sweeper,
        three_minute_sweeper,
        five_minute_sweeper,
        ten_minute_sweeper,
        fifteen_minute_sweeper,
        thirty_minute_sweeper,
        sixty_minute_sweeper,
        horizon: Horizon::DEFAULT,
        widths: Widths::pinned().map_err(|why| format!("pinned tolerances: {why}"))?,
        // ABSENT, and this caller may not derive it: `vwap::availability_of`
        // reads the whole slice, so deriving it here would make a mask at bar 0
        // depend on bar N -- the look-ahead §3 rule 7 forbids.
        availability: Availability::Absent,
        thresholds: Thresholds::CLASSICAL,
        long_exit_policy: &long_exit,
        short_exit_policy: &short_exit,
        candidate_bounds: candidate_bounds()?,
        observation_bounds: ObservationAuthorityBoundsV1::new(CEILING_RECORDS, CEILING_BYTES)
            .map_err(|why| format!("observation bounds: {why}"))?,
        statistics_bounds: PopulationStatisticsV2Bounds::new(
            CEILING_RECORDS,
            CEILING_RECORDS,
            CEILING_RECORDS,
            CEILING_RECORDS,
            CEILING_BYTES,
        )
        .map_err(|why| format!("statistics bounds: {why:?}"))?,
        statistics_procedure: PopulationStatisticsProcedureV2::new(
            BOOTSTRAP_DRAWS,
            BOOTSTRAP_SEED,
            BOOTSTRAP_BLOCK,
        )
        .map_err(|why| format!("statistics procedure: {why:?}"))?,
        search_bounds: AnchoredSearchLineageV4Bounds::new(
            CEILING_RECORDS,
            CEILING_BYTES,
            CEILING_BYTES,
        )
        .map_err(|why| format!("search lineage bounds: {why:?}"))?,
        admission_bounds: AdmissionV3Bounds::new(
            CEILING_RECORDS,
            CEILING_BYTES,
            CEILING_RECORDS,
            CEILING_BYTES,
            BLOCK_RECORDS,
        )
        .map_err(|why| format!("admission bounds: {why:?}"))?,
        admission_policy: stage.admission,
        finalization_bounds: PopulationFinalizationV3Bounds::new(
            CEILING_RECORDS,
            CEILING_BYTES,
            CEILING_RECORDS,
            CEILING_BYTES,
            BLOCK_RECORDS,
        )
        .map_err(|why| format!("finalization bounds: {why:?}"))?,
        population_bounds: PopulationV5Bounds::new(
            CEILING_RECORDS,
            CEILING_BYTES,
            CEILING_RECORDS,
            CEILING_BYTES,
            BLOCK_RECORDS,
        )
        .map_err(|why| format!("population bounds: {why:?}"))?,
    })
}

/// Commits the Execution V3 stage for all eight rungs.
///
/// # Errors
///
/// Names the rung whose Execution root or bounds refused. Every rung shares one
/// set of ceilings and each writes to its own root, whose final path component
/// the chain checks against the field name it arrived as -- so a tree that
/// somehow put `30min` bytes under `3min` refuses before the first write.
fn commit_execution(
    population: crate::all_rung_population_v5::CommittedStoredAllRungPopulationV5,
    tree: &LedgerTree,
) -> Result<crate::all_rung_population_v5::CommittedStoredAllRungExecutionV3, String> {
    let bounds = execution_bounds()?;
    let [m1, m2, m3, m5, m10, m15, m30, m60] = &tree.execution;
    commit_all_rung_stored_execution_v3(
        population,
        &AllRungStoredExecutionV3Request {
            one_minute_root: m1.as_path(),
            one_minute_bounds: bounds,
            two_minute_root: m2.as_path(),
            two_minute_bounds: bounds,
            three_minute_root: m3.as_path(),
            three_minute_bounds: bounds,
            five_minute_root: m5.as_path(),
            five_minute_bounds: bounds,
            ten_minute_root: m10.as_path(),
            ten_minute_bounds: bounds,
            fifteen_minute_root: m15.as_path(),
            fifteen_minute_bounds: bounds,
            thirty_minute_root: m30.as_path(),
            thirty_minute_bounds: bounds,
            sixty_minute_root: m60.as_path(),
            sixty_minute_bounds: bounds,
        },
    )
}

/// Commits the Selection V5 stage for all eight rungs.
///
/// # Errors
///
/// Names the rung whose Selection root, bounds or Top-25/Top-10 prefix join
/// refused.
fn commit_selection(
    execution: crate::all_rung_population_v5::CommittedStoredAllRungExecutionV3,
    tree: &LedgerTree,
    ranking: RankingPolicyV1,
) -> Result<crate::all_rung_selection_v5::CommittedStoredAllRungSelectionV5, String> {
    let selection_bounds = SelectionV5Bounds::new(
        CEILING_RECORDS,
        CEILING_BYTES,
        CEILING_RECORDS,
        CEILING_BYTES,
    )
    .map_err(|why| format!("selection bounds: {why:?}"))?;
    let [m1, m2, m3, m5, m10, m15, m30, m60] = &tree.selection;
    commit_all_rung_stored_selection_v5(
        execution,
        &AllRungSelectionV5Request {
            one_minute_root: m1.as_path(),
            one_minute_bounds: selection_bounds,
            two_minute_root: m2.as_path(),
            two_minute_bounds: selection_bounds,
            three_minute_root: m3.as_path(),
            three_minute_bounds: selection_bounds,
            five_minute_root: m5.as_path(),
            five_minute_bounds: selection_bounds,
            ten_minute_root: m10.as_path(),
            ten_minute_bounds: selection_bounds,
            fifteen_minute_root: m15.as_path(),
            fifteen_minute_bounds: selection_bounds,
            thirty_minute_root: m30.as_path(),
            thirty_minute_bounds: selection_bounds,
            sixty_minute_root: m60.as_path(),
            sixty_minute_bounds: selection_bounds,
            ranking_policy: ranking,
        },
    )
}

/// Ledger record and byte ceilings.
///
/// # Why one number and not a measured one per ledger
///
/// These are RESOURCE ceilings, not thresholds: their only job is to refuse a
/// file that has grown past what this process is willing to map. Nothing about
/// the answer changes with their value, so a ceiling derived from the data
/// would be precision that means nothing. What WOULD matter is one set too low
/// -- a run that refuses halfway -- so they are set high and named here rather
/// than tuned per ledger and forgotten.
///
/// # The two must agree, and the first draft's did not
///
/// Every ledger checks that its byte ceiling can hold its record ceiling at
/// that ledger's own stride. At 2^32 records the widest of them -- Search V4
/// lineage, at 1,536 bytes a pair -- needs six terabytes, and the byte ceiling
/// was one. The run refused with an arithmetic complaint that had nothing to do
/// with the operator's data:
///
/// > member-byte bound 1099511627776 cannot hold 4294967296 pairs
///
/// 2^24 records is sixteen million per ledger per rung, which is far more than
/// a span of this store can produce, and at the widest stride it needs 25 GiB
/// against a 64 GiB ceiling. The pair is now coherent for every ledger rather
/// than for most of them.
pub(crate) const CEILING_RECORDS: u64 = 1 << 24;
/// The byte ceiling that pairs with [`CEILING_RECORDS`].
///
/// 64 GiB. Chosen so the widest ledger's `records * stride` fits with room to
/// spare -- see [`CEILING_RECORDS`] for the arithmetic that has to hold.
pub(crate) const CEILING_BYTES: u64 = 1 << 36;
/// Records per written block, for the ledgers that block their writes.
pub(crate) const BLOCK_RECORDS: u64 = 4_096;

/// Stationary-bootstrap draws for the Statistics V2 procedure.
///
/// # Honest limit
///
/// This is a RESOLUTION, not a threshold: more draws narrow the confidence
/// interval and none of them decide what passes, because every gate that would
/// read a bootstrap p-value is answered by its own knob. It is named here so that
/// when one of those gates IS turned on, the number it depends on is visible
/// rather than buried.
pub(crate) const BOOTSTRAP_DRAWS: u64 = 1_000;
/// The deterministic bootstrap seed. Zero is a valid explicit seed; this is not.
pub(crate) const BOOTSTRAP_SEED: u64 = 1;
/// Block length for the stationary bootstrap, in periods.
pub(crate) const BOOTSTRAP_BLOCK: u64 = 2;

/// Candidate, Pre-Admission and stored-load ceilings.
///
/// # Errors
///
/// Refuses if any ceiling is rejected as zero or too small for one record.
pub(crate) fn candidate_bounds() -> Result<StoredCandidatePreAdmissionBoundsV1, String> {
    let span = |what: &str| {
        StoredSpanLoadBoundV1::new(CEILING_RECORDS)
            .map_err(|why| format!("{what} stored-load bound: {why:?}"))
    };
    Ok(StoredCandidatePreAdmissionBoundsV1 {
        signal_records: span("signal")?,
        minute_records: span("minute")?,
        daily_records: span("daily")?,
        candidate: CandidateUniverseBoundsV1::new(CEILING_RECORDS, CEILING_RECORDS)
            .map_err(|why| format!("candidate universe bounds: {why:?}"))?,
        pre_admission: PreAdmissionDataBoundsV1::new(CEILING_RECORDS, CEILING_BYTES)
            .map_err(|why| format!("pre-admission bounds: {why:?}"))?,
    })
}

/// Execution V3 file ceilings, shared by all eight rungs.
///
/// # Errors
///
/// Refuses if the runner rejects a file bound or the composed bounds.
fn execution_bounds() -> Result<ExecutionV3Bounds, String> {
    // EACH FILE'S OWN STRIDE, not a shared one. `ExecutionV3FileBound` checks
    // that the byte ceiling can actually hold the record ceiling at that
    // stride, so passing one stride for all four would either over-admit three
    // files or refuse a legal one -- and the refusal names the file, which is
    // why each carries its own word.
    let file = |records: u64, stride: usize, what: &str| {
        ExecutionV3FileBound::new(records, CEILING_BYTES, stride, what)
            .map_err(|why| format!("{what} execution file bound: {why:?}"))
    };
    ExecutionV3Bounds::new(
        file(
            CEILING_RECORDS,
            crate::execution_v3::EXECUTION_V3_PARAMETER_BYTES,
            "parameter",
        )?,
        file(
            CEILING_RECORDS,
            crate::execution_v3::EXECUTION_V3_PERCENTILE_BYTES,
            "percentile",
        )?,
        file(
            CEILING_RECORDS,
            crate::execution_v3::EXECUTION_V3_DISPOSITION_BYTES,
            "disposition",
        )?,
        file(
            CEILING_RECORDS,
            crate::execution_v3::EXECUTION_V3_COMPLETION_BYTES,
            "completion",
        )?,
        BLOCK_RECORDS,
        BLOCK_RECORDS,
    )
    .map_err(|why| format!("execution bounds: {why:?}"))
}

/// One sweeper per rung, each with a support threshold from its OWN bar count.
///
/// # Why not one shared threshold
///
/// The span holds roughly 623,000 one-minute bars and roughly 10,000
/// sixty-minute ones. A hit count that is 20% support on the coarse rung is a
/// third of a percent on the fine one, so a single `min_hits` across eight
/// rungs is eight different questions wearing one number. Expressing the
/// operator's support in ppm and resolving it against each rung's own bars asks
/// the same question eight times.
///
/// # What it logs, and why the loop may
///
/// One event per rung, eight for the whole run, each at the moment that rung's
/// span has loaded and its threshold is known. The loop is over
/// [`LEDGER_RUNGS`] -- a fixed eight -- and not over the bars it just counted,
/// so the event count does not move with the operator's span. Gate 17's rule is
/// about the loop over bars, and there is none here.
///
/// # Errors
///
/// Refuses if a rung's span cannot be loaded or its ladder rejected. A rung
/// with no stored bars is named, not skipped: a silent skip would produce a
/// seven-rung answer in an eight-rung report.
pub(crate) fn build_sweepers(
    root: &Path,
    vendor: Vendor,
    request: &LedgerAllRequest<'_>,
    verb: &str,
) -> Result<[Sweeper; 8], String> {
    let mut sweepers = Vec::with_capacity(LEDGER_RUNGS.len());
    for rung in LEDGER_RUNGS {
        // SIZED ON NIFTY'S BARS, and the pair is why that is not a narrowing.
        // Both families are swept at the same threshold, and the two share a
        // calendar and a session length, so either one answers "how many bars
        // does this rung hold over this span". NIFTY is the one the store
        // actually has.
        let span = crate::stored::load_span(root, vendor, "NIFTY", rung, request.from, request.to)
            .map_err(|why| {
                let first = why.lines().next().unwrap_or("").to_owned();
                let refusal = format!("{rung} span refused: {first}");
                crate::note(&rung_refused_event(verb, SIZING_STAGE, rung, &refusal));
                refusal
            })?;
        let min_hits = crate::min_hits_for(span.bars.len(), request.support_ppm);
        let ladder = crate::ladder_for(min_hits).map_err(|why| {
            let refusal = format!("{rung} ladder: {why}");
            crate::note(&rung_refused_event(verb, SIZING_STAGE, rung, &refusal));
            refusal
        })?;
        crate::note(&rung_sized_event(
            verb,
            rung,
            span.bars.len(),
            min_hits,
            request.support_ppm,
        ));
        sweepers.push(Sweeper::new(ladder));
    }
    // AN ARRAY, so the caller destructures instead of indexing. Eight named
    // sweepers reached by `sweepers[0]`..`sweepers[7]` is eight chances to
    // hand the sixty-minute ladder to the one-minute rung, and neither the
    // compiler nor a reader would catch it.
    sweepers
        .try_into()
        .map_err(|_| "the sweeper phase did not build eight ladders".to_owned())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
pub(crate) mod tests {
    use super::{
        ALL_GATES, LEDGER_ALL_VERB, LEDGER_RUNGS, LEDGER_TARGET, LedgerAllRequest, LedgerTree,
        PAISA_PER_POINT,
    };

    /// The one telemetry sink this crate's test binary installs.
    ///
    /// # Why install-or-adopt, and why it is shared with `ledger_v6`
    ///
    /// `telemetry::install` refuses a second call by design -- two sinks on one
    /// directory would each keep their own byte count and each would roll the
    /// other's file away -- so a test binary gets exactly ONE sink and every
    /// test that wants to observe a production emit has to assert against that
    /// one. This is that single point, and `crate::ledger_v6`'s tests call it
    /// rather than opening a second: whichever test arrives first creates it
    /// and the rest read it back.
    ///
    /// The directory is cleared once, under a [`std::sync::Once`], and never
    /// per call: a second thread clearing it after the first had opened its
    /// file would unlink the inode out from under a live descriptor, and every
    /// record written afterwards would go somewhere no reader can find.
    ///
    /// The floor is left at the default `Info`. Every event these two verbs
    /// emit is `Info` or `Warn`, so no assertion here is secretly an assertion
    /// about the floor.
    pub(crate) fn sink() -> &'static telemetry::Sink {
        static PREPARED: std::sync::Once = std::sync::Once::new();
        let dir =
            std::env::temp_dir().join(format!("brutex-cli-ledger-events-{}", std::process::id()));
        PREPARED.call_once(|| {
            let _ignored = std::fs::remove_dir_all(&dir);
        });
        match telemetry::install(&telemetry::Config::new(&dir)) {
            Ok(installed) => installed,
            Err(_refused) => telemetry::global()
                .expect("install either created the sink or named the one already there"),
        }
    }

    /// The sequence number the next record written to [`sink`] will carry.
    ///
    /// Taken immediately before a production call, so a record found afterwards
    /// is one THIS call wrote rather than one a concurrent test left behind.
    pub(crate) fn mark() -> u64 {
        sink().health().next_seq
    }

    /// Every `cli.ledger` record from sequence `from` onward carrying `message`.
    ///
    /// Read back through [`telemetry::tail`] -- the shipped reader `/logs`
    /// renders from -- rather than by parsing the file here, so a test cannot
    /// pass against bytes the real reader would refuse. Filtered on `seq` and
    /// not on time, for the reason `api::emitted` measured: the clock is read
    /// before the lock is taken, so two parallel tests can be ordered one way
    /// by their timestamps and the other way in the file.
    pub(crate) fn landed(from: u64, message: &str) -> Vec<telemetry::Record> {
        let sink = sink();
        let dir = sink
            .path()
            .parent()
            .expect("the sink writes its file inside a directory")
            .to_path_buf();
        let query = telemetry::Query::last(telemetry::MAX_LIMIT).from_target(LEDGER_TARGET);
        telemetry::tail(&dir, sink.keep_files(), &query)
            .records
            .into_iter()
            .filter(|record| record.seq >= from && record.message == message)
            .collect()
    }

    /// Whether a string field on a landed record carries `needle`.
    ///
    /// A `contains` rather than an equality because the sink cuts a string
    /// value at [`telemetry::MAX_STR_VALUE_BYTES`] and says so; every needle
    /// used here sits in the leading component, which no cut can remove.
    pub(crate) fn says(record: &telemetry::Record, key: &str, needle: &str) -> bool {
        record
            .field(key)
            .and_then(telemetry::OwnedValue::as_str)
            .is_some_and(|got| got.contains(needle))
    }

    /// Whether an integer field on a landed record carries exactly `want`.
    pub(crate) fn counts(record: &telemetry::Record, key: &str, want: u64) -> bool {
        record.field(key).and_then(telemetry::OwnedValue::as_u64) == Some(want)
    }

    /// A request that refuses before it can touch the store.
    ///
    /// `run_chain` and `run_route` both parse the vendor word FIRST, so an
    /// unknown feed refuses before `store_root`, before any directory is
    /// created and before a single bar is read. That is what makes the two
    /// run-boundary tests below drivable at all: they exercise the real verb
    /// end to end and never open the operator's store.
    fn unreachable_feed_request() -> LedgerAllRequest<'static> {
        LedgerAllRequest {
            vendor: "no-such-feed",
            from: (2024, 1),
            to: (2024, 12),
            support_ppm: 200_000,
            max_points: 50,
            root: std::path::Path::new("/nonexistent"),
        }
    }

    /// Every boundary event names its verb, its target and stays whole.
    ///
    /// `dropped_fields` is asserted rather than assumed: `telemetry::Event`
    /// COUNTS a thirteenth field instead of keeping it, so an event that grew
    /// past the ceiling would still emit and would silently be missing the
    /// field an operator filters on.
    #[test]
    fn every_ledger_boundary_is_targeted_typed_and_inside_the_field_ceiling() {
        let request = LedgerAllRequest {
            vendor: "dhan",
            from: (2024, 1),
            to: (2024, 12),
            support_ppm: 200_000,
            max_points: 50,
            root: std::path::Path::new("/nonexistent"),
        };
        let events = [
            super::run_started_event(LEDGER_ALL_VERB, &request),
            super::run_finished_event(LEDGER_ALL_VERB, 8),
            super::stage_started_event(LEDGER_ALL_VERB, super::POPULATION_STAGE),
            super::stage_finished_event(LEDGER_ALL_VERB, super::EXECUTION_STAGE, 3),
            super::rung_sized_event(LEDGER_ALL_VERB, "15min", 41_000, 8_200, 200_000),
            super::gates_resolved_event(LEDGER_ALL_VERB, ALL_GATES.len()),
        ];
        for event in &events {
            assert_eq!(event.target(), LEDGER_TARGET, "{}", event.message());
            assert_eq!(
                event.level(),
                telemetry::Level::Info,
                "{} is progress, not trouble",
                event.message()
            );
            assert_eq!(
                event.dropped_fields(),
                0,
                "{} outgrew the field ceiling",
                event.message()
            );
            assert!(event.fields().len() <= telemetry::MAX_FIELDS);
            assert!(
                event.fields().iter().any(|&(name, value)| name == "verb"
                    && value == telemetry::Value::Str(LEDGER_ALL_VERB)),
                "{} must say which verb emitted it",
                event.message()
            );
        }

        // THE WHOLE SPAN, FIELD FOR FIELD. A run is found in a log by its feed
        // and its months; an opening event missing one of them is an event an
        // operator cannot correlate to the run they are watching.
        assert_eq!(
            super::run_started_event(LEDGER_ALL_VERB, &request).fields(),
            [
                ("verb", telemetry::Value::Str("ledger-all")),
                ("feed", telemetry::Value::Str("dhan")),
                ("from_year", telemetry::Value::Uint(2024)),
                ("from_month", telemetry::Value::Uint(1)),
                ("to_year", telemetry::Value::Uint(2024)),
                ("to_month", telemetry::Value::Uint(12)),
                ("support_ppm", telemetry::Value::Uint(200_000)),
                ("max_points", telemetry::Value::Uint(50)),
                ("rungs", telemetry::Value::Uint(8)),
            ]
        );
        // AND THE ONE AN OPERATOR WATCHES DURING A LONG RUN: the rung, the bars
        // that rung actually loaded, and the threshold those bars resolved to.
        assert_eq!(
            super::rung_sized_event(LEDGER_ALL_VERB, "15min", 41_000, 8_200, 200_000).fields(),
            [
                ("verb", telemetry::Value::Str("ledger-all")),
                ("stage", telemetry::Value::Str("support-sizing")),
                ("rung", telemetry::Value::Str("15min")),
                ("bars", telemetry::Value::Uint(41_000)),
                ("min_hits", telemetry::Value::Uint(8_200)),
                ("support_ppm", telemetry::Value::Uint(200_000)),
            ]
        );
        assert_eq!(
            super::stage_finished_event(LEDGER_ALL_VERB, super::SELECTION_STAGE, 3).fields(),
            [
                ("verb", telemetry::Value::Str("ledger-all")),
                ("stage", telemetry::Value::Str("selection-v5")),
                ("written_rungs", telemetry::Value::Uint(3)),
                ("rungs", telemetry::Value::Uint(8)),
            ]
        );
    }

    /// A refusal is a warning that carries its reason, at every boundary.
    ///
    /// `CLAUDE.md` §4 bans a failure that is invisible. A refused stage that
    /// emitted at `Info`, or emitted without `why`, would be in the file and
    /// still useless: an operator filtering for trouble would not see it, and
    /// one who did could not tell what stopped the run.
    #[test]
    fn every_refused_boundary_is_a_warning_that_names_its_reason() {
        let why = "1min span refused: no stored bars for BANKNIFTY";
        let refusals = [
            super::run_refused_event(LEDGER_ALL_VERB, why),
            super::stage_refused_event(LEDGER_ALL_VERB, super::POPULATION_STAGE, why),
            super::rung_refused_event(LEDGER_ALL_VERB, super::SIZING_STAGE, "1min", why),
        ];
        for event in &refusals {
            assert_eq!(event.target(), LEDGER_TARGET);
            assert_eq!(
                event.level(),
                telemetry::Level::Warn,
                "{} must survive a filter for trouble",
                event.message()
            );
            assert!(
                event
                    .fields()
                    .iter()
                    .any(|&(name, value)| name == "why" && value == telemetry::Value::Str(why)),
                "{} must carry the reason it refused",
                event.message()
            );
            assert_eq!(event.dropped_fields(), 0);
        }
        assert_eq!(
            super::rung_refused_event(LEDGER_ALL_VERB, super::SIZING_STAGE, "1min", why).fields(),
            [
                ("verb", telemetry::Value::Str("ledger-all")),
                ("stage", telemetry::Value::Str("support-sizing")),
                ("rung", telemetry::Value::Str("1min")),
                ("why", telemetry::Value::Str(why)),
            ]
        );
        // THE COUNT AND NOT THE WORKSHEET. Thirty-seven names would be cut at
        // the string ceiling and would leave a truncated list looking complete.
        let gates = super::gates_refused_event(LEDGER_ALL_VERB, 37);
        assert_eq!(gates.level(), telemetry::Level::Warn);
        assert_eq!(
            gates.fields(),
            [
                ("verb", telemetry::Value::Str("ledger-all")),
                ("missing", telemetry::Value::Uint(37)),
                ("gates", telemetry::Value::Uint(39)),
            ]
        );
    }

    /// The `ledger-all` run boundaries reach a file, driven through the verb.
    ///
    /// # Why this drives `ledger_all` rather than building an event
    ///
    /// A hand-built event emitted onto a local sink proves the sink works and
    /// says nothing about the call site. `telemetry::emit` answers
    /// `NotInstalled` when nothing is installed and `crate::note` discards that
    /// answer, so before this test the emit calls could have been deleted
    /// wholesale and every other assertion in this file would still have
    /// passed. This drives the real verb and reads the real file.
    #[test]
    fn the_ledger_all_run_boundaries_reach_the_log_file() {
        let request = unreachable_feed_request();
        let from = mark();
        let report = super::ledger_all(&request);

        assert!(
            report.contains("refused:"),
            "an unknown feed must refuse rather than sweep: {report}"
        );
        let started = landed(from, "ledger run started");
        assert!(
            started
                .iter()
                .any(|record| says(record, "verb", "ledger-all")
                    && says(record, "feed", "no-such-feed")
                    && counts(record, "support_ppm", 200_000)),
            "the opening boundary must reach the file before the first refusal, \
             got {started:?}"
        );
        let refused = landed(from, "ledger run refused");
        assert!(
            refused
                .iter()
                .any(|record| record.level == telemetry::Level::Warn
                    && says(record, "verb", "ledger-all")
                    && says(record, "why", "is not a feed this build knows")),
            "the refusal must reach the file as a warning naming its reason, \
             got {refused:?}"
        );
    }

    /// The unset-gate refusal reaches a file, driven through the resolution.
    ///
    /// This is the refusal an operator meets on their first `ledger-all` run,
    /// and until now it existed only as terminal text.
    #[test]
    fn the_unset_gate_refusal_reaches_the_log_file() {
        let request = LedgerAllRequest {
            vendor: "dhan",
            from: (2024, 1),
            to: (2024, 1),
            support_ppm: 200_000,
            max_points: 50,
            root: std::path::Path::new("/nonexistent"),
        };
        let from = mark();
        let why = super::admission_policy(&request, LEDGER_ALL_VERB)
            .err()
            .expect("thirty-seven gates have no value, so this cannot build a policy");
        assert!(
            why.contains("37 of 39 admission gates have no value"),
            "{why}"
        );

        let records = landed(from, "admission gates unset");
        assert!(
            records
                .iter()
                .any(|record| record.level == telemetry::Level::Warn
                    && says(record, "verb", "ledger-all")
                    && counts(record, "missing", 37)
                    && counts(record, "gates", 39)),
            "the gate refusal must reach the file with its exact count, got {records:?}"
        );
    }

    /// The rung list here and the one the population chain walks are the same
    /// eight words in the same order.
    ///
    /// The chain's own constant is private, so this pins the copy against the
    /// public ladder every other command uses. If a ninth rung is ever swept,
    /// this fails before a report can claim eight.
    #[test]
    fn the_rung_order_matches_the_population_chains_own() {
        assert_eq!(
            LEDGER_RUNGS.to_vec(),
            crate::EVERY_RUNG
                .iter()
                .copied()
                .filter(|rung| *rung != "1day")
                .collect::<Vec<_>>(),
            "the ledger rungs must be the swept ladder without the daily rung"
        );
    }

    /// Every gate the runner offers is named in `ALL_GATES`.
    ///
    /// `admission_policy` fills thirty-nine fields and `ALL_GATES` lists thirty-nine
    /// names. Rust cannot check that they are the SAME thirty-nine, so this
    /// pins the count and the module doc explains why a mismatch matters: a
    /// gate missing from the list is one the census would silently not report
    /// as off.
    #[test]
    fn the_gate_census_names_every_gate_the_draft_carries() {
        assert_eq!(ALL_GATES.len(), 39, "the runner offers thirty-nine gates");
        let mut sorted = ALL_GATES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ALL_GATES.len(), "a gate is named twice");
    }

    /// A blank draft turns nothing on.
    #[test]
    fn an_unanswered_gate_refuses_with_its_own_knob_named() {
        let request = LedgerAllRequest {
            vendor: "dhan",
            from: (2024, 1),
            to: (2024, 1),
            support_ppm: 200_000,
            max_points: 50,
            root: std::path::Path::new("/nonexistent"),
        };
        let why = super::admission_policy(&request, LEDGER_ALL_VERB)
            .err()
            .expect("thirty-seven gates have no value, so this cannot build a policy");

        // THE COUNT, so a gate quietly gaining a default is a failure here.
        assert!(
            why.contains("37 of 39 admission gates have no value"),
            "the refusal must say how many are unanswered, got:\n{why}"
        );
        // A KNOB PER MISSING GATE, spelled exactly as the operator must set it.
        assert!(
            why.contains("BRUTEX_ADMIT_MIN_SUPPORT_HITS="),
            "the refusal must name the variable that answers each gate, got:\n{why}"
        );
        assert!(
            why.contains("BRUTEX_ADMIT_REQUIRE_ROMANO_WOLF_REJECTION="),
            "the last gate must be named too, got:\n{why}"
        );
        // AND THE TWO THAT ARE ANSWERED, with the sentence that decided them.
        assert!(
            why.contains("max_worst_trade_loss_paisa") && why.contains("5000"),
            "fifty points is five thousand paisa and the report must show it, got:\n{why}"
        );
    }

    /// A gate set through its knob is read, and stops being missing.
    #[test]
    fn a_knob_answers_its_gate() {
        // The knob layer reads the process environment, so this asserts the
        // NAME rather than setting it: a test that mutated the environment
        // would race every other test in this binary.
        assert_eq!(
            super::GateResolver::knob_of("min_support_hits"),
            "BRUTEX_ADMIT_MIN_SUPPORT_HITS"
        );
        assert_eq!(
            super::GateResolver::knob_of("require_romano_wolf_rejection"),
            "BRUTEX_ADMIT_REQUIRE_ROMANO_WOLF_REJECTION"
        );
    }

    /// Fifty points is five thousand paisa, not fifty.
    ///
    /// The conversion has been got wrong in this workspace before, in the
    /// direction that makes a stop a hundred times tighter than asked -- which
    /// rejects every trade and looks like a strategy that simply never fires.
    #[test]
    fn a_stop_in_points_becomes_paisa_at_a_hundred_to_one() {
        assert_eq!(50 * PAISA_PER_POINT, 5_000);
    }

    /// The tree lays out eighteen distinct directories, none aliasing another.
    #[test]
    fn the_tree_lays_out_eight_disjoint_roots_per_stage() {
        // THE PROCESS ID IS IN THE NAME, and gate 23 clause C is right to
        // insist. A fixed `/tmp` name is one directory shared by every
        // concurrent run of this suite, and this test CREATES and then DELETES
        // the tree it names -- so two runs would delete each other's fixtures
        // and fail for a reason neither of them contains.
        let temp =
            std::env::temp_dir().join(format!("brutex-ledger-all-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        let tree = LedgerTree::create(&temp).expect("the tree is creatable under a temp root");

        let mut every: Vec<_> = tree
            .execution
            .iter()
            .chain(tree.selection.iter())
            .cloned()
            .collect();
        every.push(tree.authority.clone());
        let count = every.len();
        every.sort_unstable();
        every.dedup();
        assert_eq!(count, every.len(), "two stage roots resolved to one path");
        assert_eq!(count, 17, "eight Execution, eight Selection, one authority");

        for rung in LEDGER_RUNGS {
            assert!(
                tree.authority.join(rung).is_dir(),
                "{rung} authority root was not created"
            );
        }
        let _ = std::fs::remove_dir_all(&temp);
    }
}
