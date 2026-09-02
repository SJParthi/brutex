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
//! * A gate whose value is **derived from this run's own arguments or data** is
//!   set, and the derivation is named.
//! * Every other gate is `None` -- **off**, not defaulted -- and
//!   [`render_gate_census`] prints each one by name.
//!
//! `None` here is the ABSENCE of a threshold, which is exactly what §3 rule 6
//! asks a report to admit to. A constant would be somebody's guess compiled
//! into the binary and invisible in the output; a named disabled gate is a
//! question the operator can answer with an argument.

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
struct ActiveGate {
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
struct LedgerTree {
    authority: PathBuf,
    execution: [PathBuf; 8],
    selection: [PathBuf; 8],
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
    fn create(root: &Path) -> Result<Self, String> {
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
fn exit_policy(side: Side) -> Result<ExitGridPolicyV1, String> {
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
        1_000,
        ExitGridSelectorV1::GuaranteedFloor,
        printed_ohlcv_cost_model_id_v1(),
        ForcedStopV1::Disabled,
        u64::MAX,
        u64::MAX,
    )
    .map_err(|why| format!("exit grid policy refused: {why:?}"))
}

/// The admission policy, and the census of what it does NOT check.
///
/// Returns the policy beside the gates it actually applies, so the caller can
/// print both. Thirty-nine gates exist; this sets the four the operator's own
/// rules and this run's arguments determine, and leaves thirty-five off.
///
/// # Errors
///
/// Refuses if the runner rejects the draft.
fn admission_policy(
    request: &LedgerAllRequest<'_>,
) -> Result<(AdmissionPolicyV1, Vec<ActiveGate>), String> {
    let max_loss_paisa = request
        .max_points
        .checked_mul(PAISA_PER_POINT)
        .ok_or_else(|| format!("MAX_POINTS {} overflows paisa", request.max_points))?;

    let active = vec![
        ActiveGate {
            name: "max_worst_trade_loss_paisa",
            value: max_loss_paisa.to_string(),
            because: "MAX_POINTS argument, converted at 100 paisa per index point",
        },
        ActiveGate {
            name: "min_worst_reward_risk_ppm",
            value: STATED_REWARD_RISK_PPM.to_string(),
            because: "the stated rule: smallest win at least three times the largest loss",
        },
    ];

    let policy = AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
        max_worst_trade_loss_paisa: Some(max_loss_paisa),
        min_worst_reward_risk_ppm: Some(STATED_REWARD_RISK_PPM),
        ..blank_draft()
    })
    .map_err(|why| format!("admission policy refused: {why:?}"))?;
    Ok((policy, active))
}

/// A draft with every gate off.
///
/// # Why this is spelled out and not `Default::default()`
///
/// `AdmissionPolicyDraftV1` deliberately has no `Default`, and that is the
/// right call: a defaulted trading policy is a policy nobody chose. Writing the
/// thirty-nine `None`s by hand keeps that property while still letting this
/// module say "all off except these" in one place, and a gate added to the
/// runner becomes a compile error here rather than a silently-off gate.
fn blank_draft() -> AdmissionPolicyDraftV1 {
    AdmissionPolicyDraftV1 {
        min_support_hits: None,
        min_independent_sessions: None,
        min_trades: None,
        max_mae_paisa: None,
        min_worst_reward_risk_ppm: None,
        min_win_rate_ppm: None,
        min_wilson_win_rate_ppm: None,
        min_return_drawdown_ppm: None,
        min_weakest_period_return_paisa: None,
        max_pbo_ppm: None,
        max_fwer_p_value_ppm: None,
        max_spa_p_value_ppm: None,
        min_decided_folds: None,
        max_ambiguous_fill_rate_ppm: None,
        max_gap_affected_rate_ppm: None,
        max_session_concentration_ppm: None,
        max_largest_trade_profit_share_ppm: None,
        max_drawdown_paisa: None,
        max_worst_trade_loss_paisa: None,
        max_losing_trade_rate_ppm: None,
        max_losing_trades: None,
        min_pessimistic_profit_paisa: None,
        min_winning_trades: None,
        min_average_win_paisa: None,
        max_average_loss_paisa: None,
        min_profit_factor_ppm: None,
        max_consecutive_losing_streak: None,
        min_consecutive_winning_streak: None,
        min_bootstrap_draws: None,
        min_bootstrap_strategies: None,
        min_bootstrap_periods: None,
        min_pbo_contributing_folds: None,
        max_pbo_unrankable_folds: None,
        min_profitable_oos_folds: None,
        min_oos_pessimistic_return_paisa: None,
        max_white_reality_p_value_ppm: None,
        require_white_reality_rejection: None,
        max_romano_wolf_p_value_ppm: None,
        require_romano_wolf_rejection: None,
    }
}

/// Every gate name the runner offers, in declaration order.
///
/// Used only to print what is OFF. It is a literal list rather than something
/// derived, because Rust cannot enumerate a struct's fields at runtime -- and
/// the same test that pins [`blank_draft`] pins this beside it.
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

/// Prints which gates this run applies and which it does not.
///
/// Both halves matter. A report that listed only the active gates would read as
/// a policy; listing the thirty-five that are off is what stops it being
/// mistaken for one.
fn render_gate_census(out: &mut String, active: &[ActiveGate]) {
    out.push_str("\nADMISSION GATES APPLIED BY THIS RUN\n");
    for gate in active {
        let _ = writeln!(
            out,
            "  {:<34} {:>14}   {}",
            gate.name, gate.value, gate.because
        );
    }

    let off: Vec<&str> = ALL_GATES
        .iter()
        .copied()
        .filter(|name| !active.iter().any(|gate| gate.name == *name))
        .collect();
    let _ = writeln!(
        out,
        "\nADMISSION GATES NOT APPLIED -- {} of {}",
        off.len(),
        ALL_GATES.len()
    );
    out.push_str(
        "  These are OFF, not defaulted. Nothing in this repository or in any\n\
         \x20 instruction on record names a value for them, and inventing one would\n\
         \x20 make this report claim a standard it never checked.\n",
    );
    for chunk in off.chunks(3) {
        let _ = writeln!(out, "    {}", chunk.join(", "));
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
const CEILING_RECORDS: u64 = 1 << 32;
/// The byte ceiling that pairs with [`CEILING_RECORDS`].
const CEILING_BYTES: u64 = 1 << 40;
/// Records per written block, for the ledgers that block their writes.
const BLOCK_RECORDS: u64 = 4_096;

/// Stationary-bootstrap draws for the Statistics V2 procedure.
///
/// # Honest limit
///
/// This is a RESOLUTION, not a threshold: more draws narrow the confidence
/// interval and none of them decide what passes, because every gate that would
/// read a bootstrap p-value is off in [`blank_draft`]. It is named here so that
/// when one of those gates IS turned on, the number it depends on is visible
/// rather than buried.
const BOOTSTRAP_DRAWS: u64 = 1_000;
/// The deterministic bootstrap seed. Zero is a valid explicit seed; this is not.
const BOOTSTRAP_SEED: u64 = 1;
/// Block length for the stationary bootstrap, in periods.
const BOOTSTRAP_BLOCK: u64 = 2;

/// Candidate, Pre-Admission and stored-load ceilings.
///
/// # Errors
///
/// Refuses if any ceiling is rejected as zero or too small for one record.
fn candidate_bounds() -> Result<StoredCandidatePreAdmissionBoundsV1, String> {
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
fn build_sweepers(
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
    use super::{ALL_GATES, LEDGER_RUNGS, LedgerTree, PAISA_PER_POINT, blank_draft};

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
    /// `blank_draft` sets thirty-nine fields and `ALL_GATES` lists thirty-nine
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
    fn a_blank_draft_applies_no_gate_at_all() {
        let draft = blank_draft();
        assert!(draft.min_support_hits.is_none());
        assert!(draft.max_worst_trade_loss_paisa.is_none());
        assert!(draft.min_worst_reward_risk_ppm.is_none());
        assert!(draft.require_romano_wolf_rejection.is_none());
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
