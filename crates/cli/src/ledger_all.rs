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
/// # Errors
///
/// Names every unresolved gate and the knob that would answer it.
pub(crate) fn admission_policy(
    request: &LedgerAllRequest<'_>,
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
        return Err(unset_gate_worksheet(&r));
    }
    let policy = AdmissionPolicyV1::new(draft)
        .map_err(|why| format!("admission policy refused: {why:?}"))?;
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

    match run_chain(request, &mut out) {
        Ok(written) => {
            let _ = writeln!(
                out,
                "\nCOMMITTED. {written} of {} rung Selection blocks were written rather than \
                 byte-identically reused.",
                LEDGER_RUNGS.len()
            );
        }
        Err(why) => {
            let _ = writeln!(out, "\nrefused: {why}");
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

    let sweepers = build_sweepers(&source_root, vendor, request)?;
    let (admission, active_gates) = admission_policy(request)?;
    render_gate_census(out, &active_gates);

    let ranking = RankingPolicyV1::new(Weights::equal())
        .map_err(|why| format!("ranking policy refused: {why:?}"))?;

    let population = commit_population(&Stage1 {
        source_root: source_root.as_path(),
        tree: &tree,
        vendor,
        request,
        sweepers: &sweepers,
        admission: &admission,
    })?;
    let execution = commit_execution(population, &tree)?;
    let selection = commit_selection(execution, &tree, ranking)?;
    Ok(selection.written_rung_count())
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
/// # Errors
///
/// Refuses if a rung's span cannot be loaded or its ladder rejected. A rung
/// with no stored bars is named, not skipped: a silent skip would produce a
/// seven-rung answer in an eight-rung report.
pub(crate) fn build_sweepers(
    root: &Path,
    vendor: Vendor,
    request: &LedgerAllRequest<'_>,
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
                format!("{rung} span refused: {first}")
            })?;
        let min_hits = crate::min_hits_for(span.bars.len(), request.support_ppm);
        let ladder = crate::ladder_for(min_hits).map_err(|why| format!("{rung} ladder: {why}"))?;
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
mod tests {
    use super::{ALL_GATES, LEDGER_RUNGS, LedgerAllRequest, LedgerTree, PAISA_PER_POINT};

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
        let why = super::admission_policy(&request)
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
        let temp = std::env::temp_dir().join("brutex-ledger-all-tree-test");
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
