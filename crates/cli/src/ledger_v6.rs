//! The `ledger-v6` verb: the all-rung Step-4 successor route.
//!
//! # What this is, next to `ledger-all`
//!
//! Both verbs share the Candidate pricing kernel. V6 additionally requires
//! strict checksum receipts and physical input limits, binds that authority in
//! its run identity, and retains genuine empty-family V2 evidence. Its strict
//! candidates therefore do not collide with ordinary `ledger-all` candidates.
//! The successor routes are version-separated:
//!
//! | | `ledger-all` | `ledger-v6` |
//! |---|---|---|
//! | statistics | Observation/Statistics **V2** | Statistics **V3** |
//! | admission | **V3** | **V4** |
//! | finalization | **V3** | **V4** |
//! | population | **V5** | **V6** |
//! | execution | **V3** | **V4** |
//! | selection | **V5** | **V6** |
//!
//! They are not two implementations of one thing. V6 carries a fact V5 cannot
//! express: **which families were naturally extinct.** A family whose ladder
//! emptied produced no candidate, and V5 has no way to say that other than by
//! refusing the rung; V6's `PopulationV6CandidateAuthoritiesV1` has `Both`,
//! `Nifty`, `BankNifty` and `None` arms, and Statistics V3 has a producer for
//! each shape. That is why both routes exist and why this verb is not a flag on
//! the other one.
//!
//! Selection V6 consumes the exact Execution V4 capability and retains both
//! terminal family envelopes. It applies the shared ranking and stores actual
//! Top-25/Top-10 prefixes. Chronological portfolio replay is a later authority;
//! a per-rung selection does not itself prove that selected trades never overlap.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::selection_v6::{SelectionV6Bounds, commit_stored_selection_v6};
use runner::outcome::Horizon;
use runner::topn::{RankingPolicyV1, Weights};

use indicators::evaluator::Widths;
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;

use crate::anchored_search_lineage_v4::{
    ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES, ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES,
    AnchoredSearchLineageV4Bounds,
};
use crate::execution_v4::{
    EXECUTION_V4_COMPLETION_BYTES, EXECUTION_V4_DISPOSITION_BYTES, EXECUTION_V4_PARAMETER_BYTES,
    EXECUTION_V4_PERCENTILE_BYTES, ExecutionV4Bounds, ExecutionV4FileBound,
};
use crate::ledger_all::{
    BLOCK_RECORDS, BOOTSTRAP_BLOCK, BOOTSTRAP_DRAWS, BOOTSTRAP_SEED, CEILING_BYTES,
    CEILING_RECORDS, LEDGER_RUNGS, LEDGER_TARGET, LedgerAllRequest, admission_policy,
    candidate_bounds, exit_policy, render_gate_census, run_finished_event, run_refused_event,
    run_started_event, rung_refused_event,
};
use crate::population_admission_v4::PopulationAdmissionV4Bounds;
use crate::population_finalization_v4::PopulationFinalizationV4Bounds;
use crate::population_observations_v1::ObservationAuthorityBoundsV2;
use crate::population_statistics_v2::PopulationStatisticsProcedureV2;
use crate::population_statistics_v3::PopulationStatisticsV3Bounds;
use crate::population_v6::PopulationV6Bounds;
use crate::step3_orchestrator::{
    StoredCandidatePreAdmissionRequestV1, StoredPopulationV6RouteV1,
    commit_stored_population_v6_route, commit_strict_candidate_pre_admission_authority_v1,
};

/// The two charter families, in the order every V6 successor requires them.
///
/// Canonical rather than configurable: `produce_evaluated_population_statistics_v3`
/// refuses anything but NIFTY then BANKNIFTY by name, and the mixed and
/// all-extinct producers refuse a pair that repeats one family. The order is
/// part of the record format, not a preference.
const ROUTE_FAMILIES: [&str; 2] = ["NIFTY", "BANKNIFTY"];

/// The six stage directories one rung writes under.
///
/// Each is `ROOT/<stage>/<rung>`, so no two rungs and no two stages can resolve
/// to one path. The V6 successors admit each root separately and refuse an
/// alias, exactly as the V5 chain does.
const ROUTE_STAGES: [&str; 6] = [
    "observation",
    "statistics",
    "lineage",
    "admission",
    "finalization",
    "population",
];

/// The verb word `ledger-v6` stamps on every event it emits.
pub(crate) const LEDGER_V6_VERB: &str = "ledger-v6";

/// The candidate/pre-admission phase, as the `stage` field spells it.
const CANDIDATE_STAGE: &str = "candidates";

/// The V6 successor route, as the `stage` field spells it.
const ROUTE_STAGE: &str = "population-v6-route";

// WHY THIS VERB REPORTS PER RUNG AND PER FAMILY, AND `ledger-all` DOES NOT.
//
// `crate::ledger_all` drives three successor commits that each walk the eight
// rungs internally, so the finest boundary reachable from that file is a stage.
// This file drives its own loop: eight rungs, and two families inside each. The
// per-family commit is where the sweep actually happens and where a multi-hour
// run spends its time, so it is exactly the boundary an operator watching a
// live page needs.
//
// IT IS STILL NOT A LOOP GATE 17 IS ABOUT. Sixteen iterations, fixed by
// `LEDGER_RUNGS` and `ROUTE_FAMILIES`, both compile-time constants -- the count
// does not move with bars, candidates or grid cells. A whole `ledger-v6` run
// adds one selection event per successful rung; telemetry stays at structural
// boundaries over what may be eighty months of data.

/// One rung's route opening, before its two families are committed.
fn rung_started_event(rung: &str, index: usize) -> telemetry::Event<'_> {
    telemetry::Event::info(LEDGER_TARGET, "rung route started")
        .with("verb", LEDGER_V6_VERB)
        .with("stage", ROUTE_STAGE)
        .with("rung", rung)
        .with("rung_index", index)
        .with("rungs", LEDGER_RUNGS.len())
}

/// One family's Candidate/Pre-Admission authority committed for one rung.
///
/// Sixteen of these in a whole run -- eight rungs by two families -- and each
/// one is the end of a real sweep rather than a step inside one.
fn family_committed_event<'a>(rung: &'a str, underlying: &'a str) -> telemetry::Event<'a> {
    telemetry::Event::info(LEDGER_TARGET, "family candidates committed")
        .with("verb", LEDGER_V6_VERB)
        .with("stage", CANDIDATE_STAGE)
        .with("rung", rung)
        .with("underlying", underlying)
}

/// One family refusing, with the family named beside the reason.
///
/// The family is its own field rather than only a word inside `why`, because
/// the refusal an operator actually meets today is BANKNIFTY having no bars at
/// any feed -- and a log they can filter by `underlying` answers "is it always
/// the same family" without reading prose.
fn family_refused_event<'a>(
    rung: &'a str,
    underlying: &'a str,
    why: &'a str,
) -> telemetry::Event<'a> {
    telemetry::Event::warn(LEDGER_TARGET, "family refused")
        .with("verb", LEDGER_V6_VERB)
        .with("stage", CANDIDATE_STAGE)
        .with("rung", rung)
        .with("underlying", underlying)
        .with("why", why)
}

/// One rung's V6 route committed, carrying the summary the report prints.
///
/// Every field is read from `summary`, which the route already returned from a
/// reauthenticated projection, and `admission` is the same short identity the
/// terminal row prints -- computed once and used twice. Nothing here is
/// calculated in order to be logged.
fn route_committed_event<'a>(
    rung: &'a str,
    summary: &crate::step3_orchestrator::StoredPopulationV6SummaryV1,
    admission: &'a str,
) -> telemetry::Event<'a> {
    telemetry::Event::info(LEDGER_TARGET, "rung route committed")
        .with("verb", LEDGER_V6_VERB)
        .with("stage", ROUTE_STAGE)
        .with("rung", rung)
        .with("decisions", summary.decisions)
        .with("candidates", summary.candidate_count)
        .with("nifty_candidates", summary.nifty_candidates)
        .with("banknifty_candidates", summary.banknifty_candidates)
        .with("nifty_terminal", summary.nifty_terminal)
        .with("banknifty_terminal", summary.banknifty_terminal)
        .with("admission", admission)
}

/// One rung's six stage roots plus its Execution V4 root.
struct RungRoots {
    observation: PathBuf,
    statistics: PathBuf,
    lineage: PathBuf,
    admission: PathBuf,
    finalization: PathBuf,
    population: PathBuf,
    execution: PathBuf,
    selection: PathBuf,
}

impl RungRoots {
    /// Lays out and creates one rung's roots beneath `root`.
    ///
    /// # Errors
    ///
    /// Names the directory that could not be created. The successors require
    /// every one to exist already, so creating them is what makes the verb
    /// runnable rather than a list of `mkdir` instructions.
    fn create(root: &Path, rung: &str) -> Result<Self, String> {
        let mut made = Vec::with_capacity(ROUTE_STAGES.len() + 2);
        for stage in ROUTE_STAGES.into_iter().chain(["execution", "selection"]) {
            let path = root.join(stage).join(rung);
            std::fs::create_dir_all(&path)
                .map_err(|why| format!("cannot create {}: {why}", path.display()))?;
            made.push(path);
        }
        let [
            observation,
            statistics,
            lineage,
            admission,
            finalization,
            population,
            execution,
            selection,
        ]: [PathBuf; 8] = made
            .try_into()
            .map_err(|_| format!("{rung} did not lay out eight stage roots"))?;
        Ok(Self {
            observation,
            statistics,
            lineage,
            admission,
            finalization,
            population,
            execution,
            selection,
        })
    }
}

/// Runs the all-rung V6 successor route and returns the operator's report.
///
/// # What it writes
///
/// Per rung, beneath `request.root`: `observation/`, `statistics/`,
/// `lineage/`, `admission/`, `finalization/`, `population/`, `execution/` and `selection/`
/// ledgers, each reauthenticated before the next stage reads it. The Candidate,
/// Base Evidence and Pre-Admission ledgers are written under the STORE root, as
/// they are for `ledger-all` -- the two verbs share those, and a second run
/// reuses rather than rewrites them.
pub(crate) fn ledger_v6(request: &LedgerAllRequest<'_>) -> String {
    let mut out = String::new();
    out.push_str(crate::STORED_PROVENANCE);
    let _ = writeln!(
        out,
        "\nLEDGER-V6  {} {:04}-{:02}..{:04}-{:02}  support {} ppm  stop ceiling {} points\n\
         Statistics V3 / Admission V4 / Finalization V4 / Population V6 / Execution V4 / Selection V6.",
        request.vendor,
        request.from.0,
        request.from.1,
        request.to.0,
        request.to.1,
        request.support_ppm,
        request.max_points
    );
    // BEFORE THE FIRST REFUSAL, for the reason `ledger_all` states at its own
    // call site: a run that refuses on its vendor word never reaches a rung,
    // and a page with no events at all cannot distinguish that from a run
    // nobody started.
    crate::note(&run_started_event(LEDGER_V6_VERB, request));

    match run_route(request, &mut out) {
        Ok(selections) => {
            let rungs = selections.len();
            let _ = writeln!(
                out,
                "\nCOMMITTED. {rungs} of {} rungs produced a complete Selection V6 authority (written or reused).",
                LEDGER_RUNGS.len()
            );
            crate::note(&run_finished_event(LEDGER_V6_VERB, rungs));
        }
        Err(why) => {
            let _ = writeln!(out, "\nrefused: {why}");
            crate::note(&run_refused_event(LEDGER_V6_VERB, &why));
        }
    }
    out
}

/// The route itself, lifted out so [`ledger_v6`] owns only the report.
///
/// # Errors
///
/// Names the rung and the stage that refused. There is no arm that continues
/// past one: a partially committed rung reported as a success would be the
/// failure wearing a success's clothes `CLAUDE.md` §4 bans.
fn run_route(
    request: &LedgerAllRequest<'_>,
    out: &mut String,
) -> Result<Vec<crate::selection_v6::CommittedStoredSelectionV6>, String> {
    let vendor = crate::parse_vendor(request.vendor)?;
    // Policy depends only on the request and its explicit knobs. Resolve it
    // before sizing loads all eight market spans, so unavailable data cannot
    // hide the complete worksheet for a run whose policy is already missing.
    let (admission, active_gates) = admission_policy(request, LEDGER_V6_VERB)?;
    render_gate_census(out, &active_gates);
    let strict = strict_configuration(out)?;
    let source_root = crate::store_root().map_err(|why| format!("stored source root: {why}"))?;

    let long_exit = exit_policy(runner::excursion::Side::Long)?;
    let short_exit = exit_policy(runner::excursion::Side::Short)?;
    let widths = Widths::pinned().map_err(|why| format!("pinned tolerances: {why}"))?;
    let bounds = candidate_bounds()?;

    let mut committed = Vec::with_capacity(8);
    for (index, rung) in LEDGER_RUNGS.into_iter().enumerate() {
        let (sweeper, sizing_inputs) = crate::step3_orchestrator::strict::size_sweeper(
            &source_root,
            vendor,
            request,
            rung,
            bounds,
            &strict,
        )?;
        crate::note(&rung_started_event(rung, index));
        let roots = RungRoots::create(request.root, rung).inspect_err(|why| {
            crate::note(&rung_refused_event(LEDGER_V6_VERB, ROUTE_STAGE, rung, why));
        })?;

        // The shared Candidate kernel keeps ordinary pricing semantics.
        // Strict identities additionally bind exact receipts and physical
        // limits; only repeated strict requests reuse the same authority.
        let mut families = Vec::with_capacity(ROUTE_FAMILIES.len());
        for underlying in ROUTE_FAMILIES {
            sizing_inputs.require_current()?;
            let committed_family = commit_strict_candidate_pre_admission_authority_v1(
                StoredCandidatePreAdmissionRequestV1 {
                    root: source_root.as_path(),
                    vendor,
                    underlying,
                    rung_name: rung,
                    from: request.from,
                    to: request.to,
                    sweeper: &sweeper,
                    horizon: Horizon::DEFAULT,
                    widths,
                    // ABSENT, and this caller may not derive it --
                    // `vwap::availability_of` reads the whole slice, which is
                    // the look-ahead §3 rule 7 forbids.
                    availability: Availability::Absent,
                    thresholds: Thresholds::CLASSICAL,
                    long_exit_policy: &long_exit,
                    short_exit_policy: &short_exit,
                    bounds,
                },
                &strict,
            )
            .map_err(|why| {
                let refusal = format!("v6 {rung} {underlying} refused: {why}");
                crate::note(&family_refused_event(rung, underlying, &refusal));
                refusal
            })?;
            // ONE EVENT PER FAMILY PER RUNG -- sixteen for the run, and never
            // one per candidate. The sweep that just finished evaluated
            // millions of (bar, mask) pairs and logged none of them.
            crate::note(&family_committed_event(rung, underlying));
            families.push(committed_family);
        }
        let [nifty, banknifty]: [_; 2] = families
            .try_into()
            .map_err(|_| format!("v6 {rung} did not commit exactly two families"))?;

        let committed_route = commit_stored_population_v6_route(
            nifty,
            banknifty,
            &route_for(rung, &roots, &admission)?,
        )
        .map_err(|why| {
            let refusal = format!("v6 {rung} route refused: {why}");
            crate::note(&rung_refused_event(
                LEDGER_V6_VERB,
                ROUTE_STAGE,
                rung,
                &refusal,
            ));
            refusal
        })?;

        let (execution, summary) = committed_route;
        sizing_inputs.require_current()?;
        let selection =
            render_selection(rung, &roots.selection, execution, out).inspect_err(|why| {
                crate::note(&rung_refused_event(
                    LEDGER_V6_VERB,
                    "selection-v6",
                    rung,
                    why,
                ));
            })?;
        sizing_inputs.require_current()?;
        committed.push(selection);
        if committed.len() == 1 {
            let _ = writeln!(
                out,
                "\n  {:<6}  {:>9}  {:>7}  {:>9}  {:>9}  {:>9}  {:>9}",
                "rung", "decisions", "cands", "NIFTY", "BANKNIFTY", "draws/seed", "block"
            );
        }
        render_route_summary(rung, &summary, out);
    }
    Ok(committed)
}

fn strict_configuration(
    out: &mut String,
) -> Result<crate::audited_range_command::StrictConfig, String> {
    let strict =
        crate::audited_range_command::StrictConfig::from_env().map_err(|why| why.to_string())?;
    crate::audited_range_command::validate_runtime(&[]).map_err(|why| why.to_string())?;
    crate::commit_stamp()
        .ok_or("strict institutional route requires a verified clean build before source sizing")?;
    out.push_str("\nSTRICT CHECKSUM INPUTS V1: all signal, daily and exact-minute source receipts are retained through terminal publication; physical bounds are explicit.\n");
    Ok(strict)
}

fn render_route_summary(
    rung: &str,
    summary: &crate::step3_orchestrator::StoredPopulationV6SummaryV1,
    out: &mut String,
) {
    // HOISTED SO IT IS COMPUTED ONCE AND USED TWICE -- the terminal row
    // below and the event beneath it name the same block. A second
    // `short_id` call solely to fill a log field would be the value
    // computed only to be logged that this file refuses.
    let admission_short = short_id(&summary.admission_block);
    let _ = writeln!(
        out,
        "  {:<6}  {:>9}  {:>7}  {:>9}  {:>9}  {:>4}/{:<4}  {:>9}\n         \
             NIFTY {} · BANKNIFTY {}",
        rung,
        summary.decisions,
        summary.candidate_count,
        summary.nifty_candidates,
        summary.banknifty_candidates,
        summary.draws,
        summary.seed,
        summary.block_length,
        summary.nifty_terminal,
        summary.banknifty_terminal,
    );
    // THE IDENTITIES, SHORTENED BUT NOT INVENTED. A durable ledger whose
    // report names no block leaves the operator no way to find the rows it
    // describes; sixteen hex characters locate one by prefix and still fit
    // a terminal line. `ordered` is the one to watch across reruns -- §3
    // rule 5's idempotence means it must not move.
    let _ = writeln!(
        out,
        "         admission {}  statistics {}  ordered {}\n         \
             universes  NIFTY {}  BANKNIFTY {}",
        admission_short,
        short_id(&summary.statistics_authority),
        short_id(&summary.ordered_candidates),
        short_id(&summary.nifty_universe),
        short_id(&summary.banknifty_universe),
    );
    crate::note(&route_committed_event(rung, summary, &admission_short));
}

#[cfg(test)]
pub(crate) fn strict_fixture_selection(
    root: &Path,
    nifty: impl Into<crate::step3_orchestrator::family_v6::StoredFamilyV6>,
    banknifty: impl Into<crate::step3_orchestrator::family_v6::StoredFamilyV6>,
    policy: &runner::admission::AdmissionPolicyV1,
) -> Result<crate::selection_v6::CommittedStoredSelectionV6, String> {
    let roots = RungRoots::create(root, "1min")?;
    let (execution, _) =
        commit_stored_population_v6_route(nifty, banknifty, &route_for("1min", &roots, policy)?)?;
    render_selection("1min", &roots.selection, execution, &mut String::new())
}

/// One rung's six roots and six sets of ceilings, assembled.
///
/// # Why this is its own function
///
/// It is fifty lines of `Bounds::new(...)?` and nothing else, and inside the
/// rung loop it buried the three things that loop actually does: commit two
/// families, run the route, print a row. `clippy::too_many_lines` caught it,
/// and the cap was right — a reader looking for the loop's shape had to scroll
/// past six ledgers' worth of ceilings to find it.
///
/// # Errors
///
/// Names the rung and the ledger whose ceilings were refused.
fn route_for<'a>(
    rung: &str,
    roots: &'a RungRoots,
    policy: &'a runner::admission::AdmissionPolicyV1,
) -> Result<StoredPopulationV6RouteV1<'a>, String> {
    Ok(StoredPopulationV6RouteV1 {
        observation_root: roots.observation.as_path(),
        observation_bounds: ObservationAuthorityBoundsV2::new(CEILING_RECORDS, CEILING_BYTES)
            .map_err(|why| format!("v6 {rung} observation bounds: {why}"))?,
        statistics_root: roots.statistics.as_path(),
        lineage_root: roots.lineage.as_path(),
        admission_root: roots.admission.as_path(),
        finalization_root: roots.finalization.as_path(),
        population_root: roots.population.as_path(),
        execution_root: roots.execution.as_path(),
        statistics_bounds: PopulationStatisticsV3Bounds::new(
            CEILING_RECORDS,
            CEILING_RECORDS,
            CEILING_BYTES,
        )
        .map_err(|why| format!("v6 {rung} statistics bounds: {why}"))?,
        lineage_bounds: lineage_bounds(rung)?,
        admission_bounds: PopulationAdmissionV4Bounds::new(
            CEILING_RECORDS,
            CEILING_RECORDS,
            CEILING_BYTES,
        )
        .map_err(|why| format!("v6 {rung} admission bounds: {why}"))?,
        finalization_bounds: PopulationFinalizationV4Bounds::new(
            CEILING_RECORDS,
            CEILING_RECORDS,
            CEILING_BYTES,
        )
        .map_err(|why| format!("v6 {rung} finalization bounds: {why}"))?,
        population_bounds: PopulationV6Bounds::new(CEILING_RECORDS, CEILING_RECORDS, CEILING_BYTES)
            .map_err(|why| format!("v6 {rung} population bounds: {why}"))?,
        execution_bounds: execution_v4_bounds(rung)?,
        procedure: PopulationStatisticsProcedureV2::new(
            BOOTSTRAP_DRAWS,
            BOOTSTRAP_SEED,
            BOOTSTRAP_BLOCK,
        )
        .map_err(|why| format!("v6 {rung} statistics procedure: {why:?}"))?,
        policy,
    })
}

/// The first eight bytes of a 32-byte identity, in hex.
///
/// # Why a prefix and not the whole thing
///
/// Five full identities is 320 hex characters and wraps every terminal, which
/// makes the line unreadable and so makes the identity useless. Eight bytes is
/// enough to locate a block by prefix in the ledger it came from, and the full
/// value is on disk in the record itself — this is a pointer to the answer, not
/// a substitute for it.
fn short_id(id: &[u8; 32]) -> String {
    let mut hex = String::with_capacity(16);
    for byte in id.iter().take(8) {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn selection_bounds() -> Result<SelectionV6Bounds, String> {
    SelectionV6Bounds::new(
        CEILING_BYTES / crate::selection_v6::SELECTION_V6_BLOCK_BYTES as u64,
        CEILING_BYTES,
    )
}

fn render_selection(
    rung: &str,
    root: &Path,
    execution: crate::execution_v4::CommittedStoredExecutionV4,
    out: &mut String,
) -> Result<crate::selection_v6::CommittedStoredSelectionV6, String> {
    let policy = RankingPolicyV1::new(Weights::equal())
        .map_err(|why| format!("Selection V6 ranking policy: {why:?}"))?;
    let mut selected = commit_stored_selection_v6(root, selection_bounds()?, execution, policy)?;
    let top = selected.top_twenty_five()?;
    let ten = selected.top_ten()?;
    if !top.starts_with(&ten) || ten.len() != top.len().min(10) {
        return Err("Selection V6 Top-10 differs from the actual Top-25 prefix".to_owned());
    }
    let identity = short_id(&selected.identity());
    let action = if selected.was_written() {
        "written"
    } else {
        "reused"
    };
    let _ = writeln!(
        out,
        "\n  {rung} Selection V6 {identity} {action}: {} actual Top-25 rows; {} actual Top-10 rows",
        top.len(),
        ten.len()
    );
    for winner in &top {
        let _ = writeln!(
            out,
            "    {} {:>2} {} {:?} score {} strategy {} disposition {} exit {}",
            if winner.rank < 10 { "Top10" } else { "Top25" },
            winner.rank + 1,
            winner.family,
            winner.ranked.candidate.direction,
            winner.ranked.score,
            short_id(&winner.ranked.candidate.strategy_digest.bytes()),
            short_id(&winner.disposition_id),
            short_id(&winner.selected_exit_digest)
        );
    }
    crate::note(&selection_committed_event(
        rung,
        &identity,
        selected.was_written(),
        [top.len(), ten.len()],
    ));
    Ok(selected)
}

/// Explicit later-period replay command; ordinary `ledger-v6` keeps its args.
pub(crate) fn ledger_v6_replay(
    request: &LedgerAllRequest<'_>,
    oos_from: (u16, u8),
    oos_to: (u16, u8),
) -> String {
    let mut out = String::from(crate::STORED_PROVENANCE);
    let _ = writeln!(
        out,
        "GLOBAL REPLAY V4 — actual Selection V6 prefixes, OOS {}-{:02} through {}-{:02}",
        oos_from.0, oos_from.1, oos_to.0, oos_to.1
    );
    crate::note(&run_started_event("ledger-v6-replay", request));
    match replay_route(request, oos_from, oos_to, &mut out) {
        Ok(audit) => {
            if audit.witnesses == 0 {
                out.push_str("\nNo strategies were selected. This is a complete zero-stream schedule; no OOS market bars or VIX references were loaded, so it does not attest market-data coverage for the requested period.\n");
            }
            let _ = writeln!(
                out,
                "\nCOMMITTED. Global Replay V4 {}: {} selected streams; {} offered entries; {} globally admitted; {} priced money rows; {} admitted without a price.\nPessimistic {} paisa; optimistic {} paisa. Chronological replay is evidence, not profitability or live execution assurance.",
                short_id(&audit.replay_id),
                audit.witnesses,
                audit.counters.offered,
                audit.counters.admitted,
                audit.money_rows,
                audit.admitted_pricing_refused,
                audit.pessimistic_paisa,
                audit.optimistic_paisa
            );
            crate::note(&run_finished_event("ledger-v6-replay", 8));
        }
        Err(why) => {
            let _ = writeln!(out, "\nrefused: {why}");
            crate::note(&run_refused_event("ledger-v6-replay", &why));
        }
    }
    out
}

fn replay_route(
    request: &LedgerAllRequest<'_>,
    from: (u16, u8),
    to: (u16, u8),
    out: &mut String,
) -> Result<crate::global_replay_v4::GlobalReplayV4Audit, String> {
    if from <= request.to {
        return Err(
            "Global Replay V4 OOS civil span must begin strictly after the training month span"
                .to_owned(),
        );
    }
    let loads = candidate_bounds()?;
    let oos = crate::stored_post_training_oos::StoredPostTrainingOosRequestV1::new(
        from,
        to,
        loads.signal_records,
        loads.minute_records,
        loads.daily_records,
    )?;
    let selected: [_; 8] = run_route(request, out)?
        .try_into()
        .map_err(|_| "Global Replay V4 requires all eight canonical Selection V6 authorities")?;
    let selected = crate::all_rung_selection_v6::AllRungSelectionV6::new(selected)?;
    let root = request.root.join("global-replay-v4");
    std::fs::create_dir_all(&root).map_err(|why| why.to_string())?;
    let bounds = crate::global_replay_v4::GlobalReplayV4Bounds::new(
        CEILING_BYTES / crate::global_replay_v4::GLOBAL_REPLAY_V4_RECORD_BYTES,
        CEILING_BYTES,
    )?;
    let replay = crate::global_replay_v4::commit_stored_global_replay_v4(
        &root,
        bounds,
        selected,
        oos,
        &crate::store_root()?,
    )?;
    let audit = replay.audit()?;
    let identity = short_id(&audit.publication_id);
    crate::note(
        &telemetry::Event::info(LEDGER_TARGET, "global replay committed")
            .with("verb", "ledger-v6-replay")
            .with("stage", "global-replay-v4")
            .with("publication", identity.as_str())
            .with("written", replay.was_written())
            .with("witnesses", audit.witnesses)
            .with("entries", audit.candidates)
            .with("admitted", audit.counters.admitted)
            .with("money_rows", audit.money_rows),
    );
    Ok(audit)
}

fn selection_committed_event<'a>(
    rung: &'a str,
    identity: &'a str,
    written: bool,
    counts: [usize; 2],
) -> telemetry::Event<'a> {
    let [top25, top10] = counts;
    telemetry::Event::info(LEDGER_TARGET, "selection committed")
        .with("verb", LEDGER_V6_VERB)
        .with("stage", "selection-v6")
        .with("rung", rung)
        .with("selection", identity)
        .with("written", written)
        .with("top25", top25)
        .with("top10", top10)
}

/// Search V4 lineage ceilings, sized against the record's own strides.
///
/// # Errors
///
/// Names the rung whose ceilings the runner refused.
fn lineage_bounds(rung: &str) -> Result<AnchoredSearchLineageV4Bounds, String> {
    // MEMBER AND COMPLETION BYTES ARE DERIVED, NOT GUESSED. The lineage ledger
    // checks that its byte ceiling can hold its record ceiling at each stride,
    // and `ledger-all`'s first version got that arithmetic wrong in the other
    // direction -- a byte bound that no record count could satisfy. Multiplying
    // the record count by the stride the file actually uses cannot be wrong by
    // a factor nobody notices.
    // TWO MEMBERS TO A PAIR, and the ledger counts PAIRS. A member-byte bound
    // of `pairs * MEMBER_BYTES` is exactly half what the file needs, and the
    // refusal says so in bytes rather than in the word "half":
    // "member-byte bound 12884901888 cannot hold 16777216 pairs
    // (25769803776 bytes)". `the_derived_ceilings_are_accepted_by_their_own_ledgers`
    // is what turned that from an operator's failed run into a failed build.
    let member = CEILING_RECORDS
        .checked_mul(ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES as u64)
        .and_then(|bytes| bytes.checked_mul(ROUTE_FAMILIES.len() as u64))
        .ok_or_else(|| format!("v6 {rung} lineage member-byte bound overflowed"))?;
    let completion = CEILING_RECORDS
        .checked_mul(ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES as u64)
        .ok_or_else(|| format!("v6 {rung} lineage completion-byte bound overflowed"))?;
    AnchoredSearchLineageV4Bounds::new(CEILING_RECORDS, member, completion)
        .map_err(|why| format!("v6 {rung} lineage bounds: {why:?}"))
}

/// Execution V4 file ceilings, each at its own record stride.
///
/// # Errors
///
/// Names the file whose ceilings the runner refused.
fn execution_v4_bounds(rung: &str) -> Result<ExecutionV4Bounds, String> {
    let file = |stride: usize, what: &str| {
        ExecutionV4FileBound::new(CEILING_RECORDS, CEILING_BYTES, stride, what)
            .map_err(|why| format!("v6 {rung} {what} execution file bound: {why:?}"))
    };
    ExecutionV4Bounds::new(
        file(EXECUTION_V4_PARAMETER_BYTES, "parameter")?,
        file(EXECUTION_V4_PERCENTILE_BYTES, "percentile")?,
        file(EXECUTION_V4_DISPOSITION_BYTES, "disposition")?,
        file(EXECUTION_V4_COMPLETION_BYTES, "completion")?,
        BLOCK_RECORDS,
        BLOCK_RECORDS,
    )
    .map_err(|why| format!("v6 {rung} execution bounds: {why:?}"))
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
    use super::{
        LEDGER_V6_VERB, ROUTE_FAMILIES, ROUTE_STAGES, RungRoots, execution_v4_bounds,
        lineage_bounds,
    };
    use crate::ledger_all::tests::{counts, landed, mark, says};

    /// A summary with every count distinct, so a field crossed with another is
    /// a failure rather than a coincidence.
    fn sample_summary() -> crate::step3_orchestrator::StoredPopulationV6SummaryV1 {
        crate::step3_orchestrator::StoredPopulationV6SummaryV1 {
            draws: 1_000,
            seed: 1,
            block_length: 2,
            candidate_count: 512,
            nifty_terminal: "Admitted",
            banknifty_terminal: "Extinct",
            nifty_candidates: 500,
            banknifty_candidates: 12,
            decisions: 37,
            admission_block: [0xab; 32],
            statistics_authority: [0xcd; 32],
            ordered_candidates: [0xef; 32],
            nifty_universe: [0x01; 32],
            banknifty_universe: [0x02; 32],
        }
    }

    /// Every V6 boundary names its verb, its rung and stays inside the ceiling.
    ///
    /// The rung is the field this route exists to carry: `ledger-all` can only
    /// report per stage because its successors walk the eight rungs internally,
    /// and this verb drives its own loop. An event here without a `rung` would
    /// throw away the only thing the finer granularity bought.
    #[test]
    fn every_v6_boundary_names_its_rung_and_stays_inside_the_field_ceiling() {
        let summary = sample_summary();
        let events = [
            super::rung_started_event("5min", 3),
            super::family_committed_event("5min", "NIFTY"),
            super::route_committed_event("5min", &summary, "abababababababab"),
        ];
        for event in &events {
            assert_eq!(event.target(), "cli.ledger", "{}", event.message());
            assert_eq!(event.level(), telemetry::Level::Info, "{}", event.message());
            assert_eq!(
                event.dropped_fields(),
                0,
                "{} outgrew the field ceiling",
                event.message()
            );
            assert!(event.fields().len() <= telemetry::MAX_FIELDS);
            for (name, value) in [
                ("verb", telemetry::Value::Str(LEDGER_V6_VERB)),
                ("rung", telemetry::Value::Str("5min")),
            ] {
                assert!(
                    event
                        .fields()
                        .iter()
                        .any(|&(field, carried)| field == name && carried == value),
                    "{} omitted {name}",
                    event.message()
                );
            }
        }

        // THE ROUTE'S OWN NUMBERS, every one of them read from the summary the
        // route already returned and the report already prints.
        assert_eq!(
            super::route_committed_event("5min", &summary, "abababababababab").fields(),
            [
                ("verb", telemetry::Value::Str("ledger-v6")),
                ("stage", telemetry::Value::Str("population-v6-route")),
                ("rung", telemetry::Value::Str("5min")),
                ("decisions", telemetry::Value::Uint(37)),
                ("candidates", telemetry::Value::Uint(512)),
                ("nifty_candidates", telemetry::Value::Uint(500)),
                ("banknifty_candidates", telemetry::Value::Uint(12)),
                ("nifty_terminal", telemetry::Value::Str("Admitted")),
                ("banknifty_terminal", telemetry::Value::Str("Extinct")),
                ("admission", telemetry::Value::Str("abababababababab")),
            ]
        );
        assert_eq!(
            super::rung_started_event("60min", 7).fields(),
            [
                ("verb", telemetry::Value::Str("ledger-v6")),
                ("stage", telemetry::Value::Str("population-v6-route")),
                ("rung", telemetry::Value::Str("60min")),
                ("rung_index", telemetry::Value::Uint(7)),
                ("rungs", telemetry::Value::Uint(8)),
            ]
        );
    }

    /// A refused family is a warning that names the family and the reason.
    ///
    /// The family is a field of its own because the refusal an operator meets
    /// today is BANKNIFTY having no bars at any feed, and a log they can filter
    /// by `underlying` answers "is it always the same family" without reading
    /// prose.
    #[test]
    fn a_refused_family_is_a_warning_that_names_the_family() {
        let why = "v6 1min BANKNIFTY refused: no stored bars";
        let event = super::family_refused_event("1min", "BANKNIFTY", why);
        assert_eq!(event.target(), "cli.ledger");
        assert_eq!(event.level(), telemetry::Level::Warn);
        assert_eq!(event.dropped_fields(), 0);
        assert_eq!(
            event.fields(),
            [
                ("verb", telemetry::Value::Str("ledger-v6")),
                ("stage", telemetry::Value::Str("candidates")),
                ("rung", telemetry::Value::Str("1min")),
                ("underlying", telemetry::Value::Str("BANKNIFTY")),
                ("why", telemetry::Value::Str(why)),
            ]
        );
    }

    /// The `ledger-v6` run boundaries reach a file, driven through the verb.
    ///
    /// Drives the real verb, for the reason `ledger_all`'s own file-landing
    /// test states: a hand-built event on a sink proves the sink and says
    /// nothing about the call site. An unknown feed refuses inside
    /// `parse_vendor`, which `run_route` calls first, so this exercises the
    /// opening and closing boundaries without opening the operator's store.
    ///
    /// # What this cannot reach, said rather than implied
    ///
    /// The per-rung, per-family and per-route events are emitted only after a
    /// real span has loaded, so no test in this binary drives them: their
    /// fields are proven above and their reach is not. Making them reachable
    /// needs a store fixture on this path, which does not exist today.
    /// `CLAUDE.md` §3 rule 6.
    #[test]
    fn the_ledger_v6_run_boundaries_reach_the_log_file() {
        let request = crate::ledger_all::LedgerAllRequest {
            vendor: "no-such-feed",
            from: (2024, 1),
            to: (2024, 12),
            support_ppm: 200_000,
            max_points: 50,
            root: std::path::Path::new("/nonexistent"),
        };
        let from = mark();
        let report = super::ledger_v6(&request);

        assert!(
            report.contains("refused:"),
            "an unknown feed must refuse rather than sweep: {report}"
        );
        let started = landed(from, "ledger run started");
        assert!(
            started
                .iter()
                .any(|record| says(record, "verb", "ledger-v6")
                    && says(record, "feed", "no-such-feed")
                    && counts(record, "rungs", 8)),
            "the opening boundary must reach the file under this verb's own \
             name, got {started:?}"
        );
        let refused = landed(from, "ledger run refused");
        assert!(
            refused
                .iter()
                .any(|record| record.level == telemetry::Level::Warn
                    && says(record, "verb", "ledger-v6")
                    && says(record, "why", "is not a feed this build knows")),
            "the refusal must reach the file as a warning naming its reason, \
             got {refused:?}"
        );
    }

    fn assert_missing_policy_precedes_market_sizing(replay: bool) {
        let _serial = crate::knobs::serially();
        crate::knobs::clear_all();
        let root = std::env::temp_dir().join(format!(
            "brutex-ledger-v6-policy-preflight-{}-{replay}",
            std::process::id()
        ));
        assert!(!root.exists(), "preflight has no output tree to reuse");
        let request = crate::ledger_all::LedgerAllRequest {
            vendor: "dhan",
            from: (2024, 1),
            to: (2024, 1),
            support_ppm: 200_000,
            max_points: 50,
            root: &root,
        };
        let from = mark();
        let report = if replay {
            super::ledger_v6_replay(&request, (2024, 2), (2024, 2))
        } else {
            super::ledger_v6(&request)
        };
        assert!(
            report.contains("37 of 39 admission gates have no value"),
            "{report}"
        );
        for field in runner::admission::AdmissionFieldV1::ALL {
            if !matches!(
                field,
                runner::admission::AdmissionFieldV1::MaxWorstTradeLossPaisa
                    | runner::admission::AdmissionFieldV1::MinWorstRewardRiskPpm
            ) {
                let knob = format!("BRUTEX_ADMIT_{}=", field.name().to_ascii_uppercase());
                assert!(report.contains(&knob), "worksheet omitted {knob}: {report}");
            }
        }
        assert!(!report.contains("COMMITTED."), "{report}");
        assert!(
            !root.exists(),
            "a refused policy must not create authority roots"
        );
        assert!(landed(from, "admission gates unset").iter().any(|record| {
            says(record, "verb", LEDGER_V6_VERB)
                && counts(record, "missing", 37)
                && counts(record, "gates", 39)
        }));
        assert!(
            !landed(from, "rung support sized").iter().any(|record| says(
                record,
                "verb",
                LEDGER_V6_VERB
            )),
            "no sizing may run"
        );
        assert!(
            !landed(from, "rung refused")
                .iter()
                .any(|record| says(record, "verb", LEDGER_V6_VERB)),
            "no sizing may refuse first"
        );
    }

    #[test]
    fn missing_policy_refuses_before_ledger_v6_market_sizing() {
        assert_missing_policy_precedes_market_sizing(false);
    }

    #[test]
    fn missing_policy_refuses_before_ledger_v6_replay_market_sizing() {
        assert_missing_policy_precedes_market_sizing(true);
    }

    /// The family order is the one every V6 successor demands.
    ///
    /// `produce_evaluated_population_statistics_v3` refuses anything but NIFTY
    /// then BANKNIFTY by name, so a reordering here would not be a preference
    /// change -- it would refuse every rung at the statistics stage.
    #[test]
    fn the_route_asks_for_nifty_then_banknifty() {
        assert_eq!(ROUTE_FAMILIES, ["NIFTY", "BANKNIFTY"]);
    }

    /// Each rung lays out eight distinct roots, none aliasing another.
    #[test]
    fn a_rung_lays_out_eight_disjoint_stage_roots() {
        // Named for this process, for the reason gate 23 clause C gives: the
        // test creates and removes the tree, so a fixed name lets two
        // concurrent runs delete each other's fixtures.
        let temp =
            std::env::temp_dir().join(format!("brutex-ledger-v6-roots-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        let roots = RungRoots::create(&temp, "1min").expect("the rung tree is creatable");

        let mut every = vec![
            roots.observation,
            roots.statistics,
            roots.lineage,
            roots.admission,
            roots.finalization,
            roots.population,
            roots.execution,
            roots.selection,
        ];
        let count = every.len();
        every.sort_unstable();
        every.dedup();
        assert_eq!(count, every.len(), "two stage roots resolved to one path");
        assert_eq!(
            count,
            ROUTE_STAGES.len() + 2,
            "six stages plus execution and selection"
        );
        for path in &every {
            assert!(path.is_dir(), "{} was not created", path.display());
        }
        let _ = std::fs::remove_dir_all(&temp);
    }

    /// The lineage and Execution V4 ceilings are internally consistent.
    ///
    /// Both derive their byte bounds from the record count times the stride the
    /// file actually uses. `ledger-all` shipped the other arithmetic once -- a
    /// byte ceiling no record count could satisfy -- and it refused with a
    /// message about six terabytes that had nothing to do with the operator's
    /// data. This fails the build instead.
    #[test]
    fn the_derived_ceilings_are_accepted_by_their_own_ledgers() {
        lineage_bounds("1min").expect("lineage ceilings are self-consistent");
        execution_v4_bounds("1min").expect("Execution V4 ceilings are self-consistent");
        super::selection_bounds().expect("Selection V6 ceilings are self-consistent");
    }

    #[test]
    fn selection_event_retains_actual_empty_and_short_prefix_counts() {
        for (written, top25, top10) in [(true, 0, 0), (false, 7, 7), (true, 25, 10)] {
            let event = super::selection_committed_event(
                "5min",
                "exact-selection",
                written,
                [top25, top10],
            );
            assert_eq!(event.dropped_fields(), 0);
            assert_eq!(event.target(), "cli.ledger");
            assert_eq!(
                event.fields(),
                [
                    ("verb", telemetry::Value::Str("ledger-v6")),
                    ("stage", telemetry::Value::Str("selection-v6")),
                    ("rung", telemetry::Value::Str("5min")),
                    ("selection", telemetry::Value::Str("exact-selection")),
                    ("written", telemetry::Value::Bool(written)),
                    ("top25", telemetry::Value::Uint(top25 as u64)),
                    ("top10", telemetry::Value::Uint(top10 as u64)),
                ]
            );
        }
    }
}
