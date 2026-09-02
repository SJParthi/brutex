//! The `ledger-v6` verb: the all-rung Step-4 successor route.
//!
//! # What this is, next to `ledger-all`
//!
//! Both verbs run the same phase one -- one Candidate/Pre-Admission commit per
//! family per rung, which is where the sweep actually happens -- and then take
//! different successor routes over the same committed candidates:
//!
//! | | `ledger-all` | `ledger-v6` |
//! |---|---|---|
//! | statistics | Observation/Statistics **V2** | Statistics **V3** |
//! | admission | **V3** | **V4** |
//! | finalization | **V3** | **V4** |
//! | population | **V5** | **V6** |
//! | execution | **V3** | **V4** |
//! | selection | **V5** | none yet |
//!
//! They are not two implementations of one thing. V6 carries a fact V5 cannot
//! express: **which families were naturally extinct.** A family whose ladder
//! emptied produced no candidate, and V5 has no way to say that other than by
//! refusing the rung; V6's `PopulationV6CandidateAuthoritiesV1` has `Both`,
//! `Nifty`, `BankNifty` and `None` arms, and Statistics V3 has a producer for
//! each shape. That is why both routes exist and why this verb is not a flag on
//! the other one.
//!
//! # Why it stops at Execution V4
//!
//! There is no Selection V6. `selection_v5` is the newest selector in the tree
//! and it consumes an Execution **V3** authority, so the V6 route terminates at
//! its Execution V4 commit rather than being wired into a selector that cannot
//! read it. Saying so here is cheaper than a caller discovering it: the ledgers
//! this verb writes are complete and durable, and ranking them is Step 5.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use runner::outcome::Horizon;

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
    CEILING_RECORDS, LEDGER_RUNGS, LedgerAllRequest, admission_policy, build_sweepers,
    candidate_bounds, exit_policy, render_gate_census,
};
use crate::population_admission_v4::PopulationAdmissionV4Bounds;
use crate::population_finalization_v4::PopulationFinalizationV4Bounds;
use crate::population_observations_v1::ObservationAuthorityBoundsV2;
use crate::population_statistics_v2::PopulationStatisticsProcedureV2;
use crate::population_statistics_v3::PopulationStatisticsV3Bounds;
use crate::population_v6::PopulationV6Bounds;
use crate::step3_orchestrator::{
    StoredCandidatePreAdmissionRequestV1, StoredPopulationV6RouteV1,
    commit_stored_candidate_pre_admission_authority_v1, commit_stored_population_v6_route,
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

/// One rung's six stage roots plus its Execution V4 root.
struct RungRoots {
    observation: PathBuf,
    statistics: PathBuf,
    lineage: PathBuf,
    admission: PathBuf,
    finalization: PathBuf,
    population: PathBuf,
    execution: PathBuf,
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
        let mut made = Vec::with_capacity(ROUTE_STAGES.len() + 1);
        for stage in ROUTE_STAGES.into_iter().chain(std::iter::once("execution")) {
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
        ]: [PathBuf; 7] = made
            .try_into()
            .map_err(|_| format!("{rung} did not lay out seven stage roots"))?;
        Ok(Self {
            observation,
            statistics,
            lineage,
            admission,
            finalization,
            population,
            execution,
        })
    }
}

/// Runs the all-rung V6 successor route and returns the operator's report.
///
/// # What it writes
///
/// Per rung, beneath `request.root`: `observation/`, `statistics/`,
/// `lineage/`, `admission/`, `finalization/`, `population/` and `execution/`
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
         Statistics V3 / Admission V4 / Finalization V4 / Population V6 / Execution V4.",
        request.vendor,
        request.from.0,
        request.from.1,
        request.to.0,
        request.to.1,
        request.support_ppm,
        request.max_points
    );

    match run_route(request, &mut out) {
        Ok(rungs) => {
            let _ = writeln!(
                out,
                "\nCOMMITTED. {rungs} of {} rungs wrote a complete Execution V4 authority.",
                LEDGER_RUNGS.len()
            );
        }
        Err(why) => {
            let _ = writeln!(out, "\nrefused: {why}");
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
fn run_route(request: &LedgerAllRequest<'_>, out: &mut String) -> Result<usize, String> {
    let vendor = crate::parse_vendor(request.vendor)?;
    let source_root = crate::store_root().map_err(|why| format!("stored source root: {why}"))?;

    let sweepers = build_sweepers(&source_root, vendor, request)?;
    let (admission, active_gates) = admission_policy(request)?;
    render_gate_census(out, &active_gates);

    let long_exit = exit_policy(runner::excursion::Side::Long)?;
    let short_exit = exit_policy(runner::excursion::Side::Short)?;
    let widths = Widths::pinned().map_err(|why| format!("pinned tolerances: {why}"))?;
    let bounds = candidate_bounds()?;

    let mut committed = 0_usize;
    for (rung, sweeper) in LEDGER_RUNGS.into_iter().zip(sweepers.iter()) {
        let roots = RungRoots::create(request.root, rung)?;

        // PHASE ONE, PER FAMILY. Identical to what `ledger-all` runs, and
        // deliberately so: the candidate ledger is keyed by universe identity
        // and its append is idempotent, so running both verbs over one span
        // sweeps once and the second reuses.
        let mut families = Vec::with_capacity(ROUTE_FAMILIES.len());
        for underlying in ROUTE_FAMILIES {
            let committed_family = commit_stored_candidate_pre_admission_authority_v1(
                StoredCandidatePreAdmissionRequestV1 {
                    root: source_root.as_path(),
                    vendor,
                    underlying,
                    rung_name: rung,
                    from: request.from,
                    to: request.to,
                    sweeper,
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
            )
            .map_err(|why| format!("v6 {rung} {underlying} refused: {why}"))?;
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
        .map_err(|why| format!("v6 {rung} route refused: {why}"))?;

        committed = committed.saturating_add(1);
        if committed == 1 {
            let _ = writeln!(
                out,
                "\n  {:<6}  {:>9}  {:>7}  {:>9}  {:>9}  {:>9}  {:>9}",
                "rung", "decisions", "cands", "NIFTY", "BANKNIFTY", "draws/seed", "block"
            );
        }
        let (_execution, summary) = committed_route;
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
            short_id(&summary.admission_block),
            short_id(&summary.statistics_authority),
            short_id(&summary.ordered_candidates),
            short_id(&summary.nifty_universe),
            short_id(&summary.banknifty_universe),
        );
    }
    Ok(committed)
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
    use super::{ROUTE_FAMILIES, ROUTE_STAGES, RungRoots, execution_v4_bounds, lineage_bounds};

    /// The family order is the one every V6 successor demands.
    ///
    /// `produce_evaluated_population_statistics_v3` refuses anything but NIFTY
    /// then BANKNIFTY by name, so a reordering here would not be a preference
    /// change -- it would refuse every rung at the statistics stage.
    #[test]
    fn the_route_asks_for_nifty_then_banknifty() {
        assert_eq!(ROUTE_FAMILIES, ["NIFTY", "BANKNIFTY"]);
    }

    /// Each rung lays out seven distinct roots, none aliasing another.
    #[test]
    fn a_rung_lays_out_seven_disjoint_stage_roots() {
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
        ];
        let count = every.len();
        every.sort_unstable();
        every.dedup();
        assert_eq!(count, every.len(), "two stage roots resolved to one path");
        assert_eq!(count, ROUTE_STAGES.len() + 1, "six stages plus execution");
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
    }
}
