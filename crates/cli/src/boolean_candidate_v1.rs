//! Complete program-aware candidate observations in a separate immutable namespace.
//! A finite explicitly requested catalog is not proof of exhaustive grammar search.
//! Every program, direction and resolved coordinate is retained before statistics.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use brutex_core::blake3::{Hasher, hash};
use indicators::column::Column;
use runner::excursion::Side;
use runner::exit_grid_policy::expression_execution::later_period::ExpressionTrainingAnchorV1;
use runner::exit_grid_policy::expression_execution::{
    ExpressionExecutionRunV1, SelectedExpressionExitV1,
};
use runner::exit_grid_policy::research_resolution::ResearchResolvedExitGridV1;
use runner::exit_grid_policy::{ExecutionRefusalBitsV1, ExecutionSeriesV1, ExitGridPolicyV1};
use runner::expression::Expression;
use runner::grid::{Cell, Chosen, TradeRow};
use runner::identity::{DailyReferenceBinding, Direction, Params, ReferenceIntegrity, Run};
use runner::outcome::Horizon;
use runner::research_family::ResearchFamilyV1;

use crate::audited_range_command::StrictConfig;
use crate::audited_stored::{RangeData, RangeGuard, RangeInputs, RangeRequest};
use crate::sweep_evidence::{Attempt, Completion, Operation};

#[path = "boolean_candidate_persistence.rs"]
pub(crate) mod persistence;

#[path = "boolean_statistics_v1.rs"]
pub(crate) mod statistics;

#[path = "boolean_candidate_reader.rs"]
pub mod reader;

#[path = "boolean_candidate_grid.rs"]
pub mod grid_context;

#[path = "boolean_oos_v1.rs"]
pub(crate) mod oos;

/// Explicit physical ceilings for this complete catalog; never selection limits.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bounds {
    pub programs: u64,
    pub coordinates: u64,
    pub trades: u64,
    pub bytes: u64,
}

/// Caller supplies actual policies; there are no invented financial defaults.
#[derive(Clone, Copy)]
pub(crate) struct Request<'a> {
    pub store: &'a Path,
    pub output: &'a Path,
    pub vendor: brutex_core::vendor::Vendor,
    pub underlying: &'a str,
    pub rung: &'a str,
    pub from: (u16, u8),
    pub to: (u16, u8),
    pub horizon: Horizon,
    pub programs: &'a [Expression],
    pub long: &'a ExitGridPolicyV1,
    pub short: &'a ExitGridPolicyV1,
    pub inputs: &'a StrictConfig,
    pub bounds: Bounds,
    pub widths: indicators::evaluator::Widths,
    pub thresholds: indicators::pattern::Thresholds,
}

/// One explicit accepted session, including sessions with no trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BooleanSessionV1 {
    day: i64,
    return_paisa: i64,
    trades: u64,
    wins: u64,
}
impl BooleanSessionV1 {
    pub(crate) const fn day(&self) -> i64 {
        self.day
    }
    pub(crate) const fn return_paisa(&self) -> i64 {
        self.return_paisa
    }
    pub(crate) const fn trades(&self) -> u64 {
        self.trades
    }
    pub(crate) const fn wins(&self) -> u64 {
        self.wins
    }
}

/// Exact complete-grid candidate, before any statistical admission.
#[derive(Clone, Debug)]
pub(crate) struct BooleanCoordinateV1 {
    identity: [u8; 32],
    program_index: usize,
    run: [u8; 32],
    side: Side,
    ordinal: u64,
    cell: Cell,
    refusal: ExecutionRefusalBitsV1,
    periods: Vec<BooleanSessionV1>,
    trades: Vec<TradeRow>,
    selected: Option<SelectedExpressionExitV1>,
    summary: runner::expression::Summary,
    support_sessions: u64,
}
impl BooleanCoordinateV1 {
    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    pub(crate) const fn program_index(&self) -> usize {
        self.program_index
    }
    pub(crate) const fn run_id(&self) -> [u8; 32] {
        self.run
    }
    pub(crate) const fn side(&self) -> Side {
        self.side
    }
    pub(crate) const fn ordinal(&self) -> u64 {
        self.ordinal
    }
    pub(crate) const fn cell(&self) -> &Cell {
        &self.cell
    }
    pub(crate) const fn execution_refusal_bits(&self) -> ExecutionRefusalBitsV1 {
        self.refusal
    }
    pub(crate) fn periods(&self) -> &[BooleanSessionV1] {
        &self.periods
    }
    pub(crate) fn trades(&self) -> &[TradeRow] {
        &self.trades
    }
    pub(crate) const fn summary(&self) -> runner::expression::Summary {
        self.summary
    }
    pub(crate) const fn support_sessions(&self) -> u64 {
        self.support_sessions
    }
    pub(crate) fn selected_exit(&self) -> Option<&SelectedExpressionExitV1> {
        self.selected.as_ref()
    }
}

struct Source {
    data: RangeData,
    guard: RangeGuard,
    family: ResearchFamilyV1,
    identity: [u8; 32],
}

struct RetainedSource {
    guard: RangeGuard,
    family: ResearchFamilyV1,
}

/// Only the producer can mint this authority. Reopened bytes alone are a view.
pub(crate) struct CommittedBooleanFamilyV1 {
    source: Arc<RetainedSource>,
    identity: [u8; 32],
    completion: [u8; 32],
    directory: PathBuf,
    programs: Vec<Expression>,
    sessions: Vec<i64>,
    rows: Vec<BooleanCoordinateV1>,
    payload: [u8; 32],
    bytes: u64,
    cohort: [u8; 32],
    resolutions: [ResearchResolvedExitGridV1; 2],
    training_context: oos::TrainingContext,
    anchors: Vec<ExpressionTrainingAnchorV1>,
}
impl CommittedBooleanFamilyV1 {
    pub(crate) fn require_current(&self) -> Result<(), String> {
        self.source.guard.require_current()?;
        for row in &self.rows {
            let resolved = self
                .resolutions
                .get(usize::from(row.side == Side::Short))
                .ok_or("Boolean retained resolution missing")?;
            if resolved.family() != self.source.family || resolved.side() != row.side {
                return Err("Boolean retained resolution family crosswire".to_owned());
            }
            let program = self
                .programs
                .get(row.program_index)
                .ok_or("Boolean retained program index changed")?;
            match row.selected_exit() {
                Some(selected)
                    if row.refusal.is_empty()
                        && selected.run_id().bytes() == row.run
                        && selected.program() == program
                        && selected.coordinate() == Chosen::from_cell(&row.cell)
                        && selected.training_cell() == &row.cell => {}
                None if !row.refusal.is_empty() => {}
                _ => {
                    return Err(
                        "Boolean retained selected capability differs from complete candidate"
                            .to_owned(),
                    );
                }
            }
        }
        persistence::verify(
            &self.directory,
            self.identity,
            self.payload,
            self.bytes,
            self.completion,
        )?;
        self.source.guard.require_current()
    }
    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    pub(crate) const fn completion_digest(&self) -> [u8; 32] {
        self.completion
    }
    pub(crate) fn family(&self) -> ResearchFamilyV1 {
        self.source.family
    }
    pub(crate) fn sessions(&self) -> &[i64] {
        &self.sessions
    }
    pub(crate) fn rows(&self) -> &[BooleanCoordinateV1] {
        &self.rows
    }
    pub(crate) fn programs(&self) -> &[Expression] {
        &self.programs
    }
    pub(crate) const fn cohort_digest(&self) -> [u8; 32] {
        self.cohort
    }
}

/// Execute stored inputs only under this binary's recorded clean provenance.
pub(crate) fn produce(request: Request<'_>) -> Result<CommittedBooleanFamilyV1, String> {
    let commit = crate::commit_stamp()
        .ok_or("Boolean candidate production requires a clean build identity")?;
    produce_identified(request, commit)
}

/// Exact source and expected catalog identity, with no indicator or price kernel.
/// Retains the same source/receipt guards after discarding decoded bar buffers.
pub(crate) struct Fingerprint {
    pub(crate) identity: [u8; 32],
    pub(crate) descriptor: [u8; 32],
    pub(crate) source: [u8; 32],
    guard: RangeGuard,
}

impl Fingerprint {
    pub(crate) fn require_current(&self) -> Result<(), String> {
        self.guard.require_current()
    }
}

pub(crate) fn fingerprint(request: &Request<'_>, commit: &str) -> Result<Fingerprint, String> {
    validate_request(request)?;
    let source = load_source(request)?;
    let identity = catalog_identity(request, &source, commit, cohort_digest(request, commit)?);
    let mut common = *request;
    common.programs = &[];
    common.bounds.programs = 0;
    let mut descriptor = Hasher::new();
    descriptor.update(b"brutex-boolean-campaign-source-v1\0");
    descriptor.update(&source.family.encode());
    descriptor.update(&source.identity);
    descriptor.update(&cohort_digest(&common, commit)?);
    source.guard.require_current()?;
    Ok(Fingerprint {
        identity,
        descriptor: descriptor.finalize(),
        source: source.identity,
        guard: source.guard,
    })
}

#[cfg(test)]
pub(crate) fn produce_campaign_fixture(
    request: Request<'_>,
) -> Result<CommittedBooleanFamilyV1, String> {
    produce_identified(request, "generated-boolean-campaign-fixture")
}

fn produce_identified(
    request: Request<'_>,
    commit: &str,
) -> Result<CommittedBooleanFamilyV1, String> {
    validate_request(&request)?;
    let source = load_source(&request)?;
    let cohort = cohort_digest(&request, commit)?;
    let identity = catalog_identity(&request, &source, commit, cohort);
    let attempt =
        crate::sweep_evidence::begin(request.output, identity, Operation::BooleanCandidates)?;
    let produced = compute(&request, &source, identity, commit);
    let (programs, sessions, rows, resolutions, anchors) = match produced {
        Ok(value) => value,
        Err(why) => return Err(refuse(attempt, why)),
    };
    let body = persistence::encode(
        (identity, cohort),
        source.family,
        &programs,
        &sessions,
        &rows,
        (&resolutions, request.horizon),
        request.bounds.bytes,
    );
    let body = match body {
        Ok(body) => body,
        Err(why) => return Err(refuse(attempt, why)),
    };
    let pending = match persistence::prepare(request.output, identity, &body) {
        Ok(value) => value,
        Err(why) => return Err(refuse(attempt, why)),
    };
    let payload = hash(&body);
    let bytes = body.len() as u64;
    #[cfg(test)]
    commit_fault(pending.directory());
    let completion = crate::finish_stored_month(
        attempt,
        Completion::Completed,
        || {
            source.guard.require_current()?;
            pending.verify_body(payload, bytes)
        },
        || pending.finish(identity, payload, bytes),
    )?;
    let committed = CommittedBooleanFamilyV1 {
        source: Arc::new(RetainedSource {
            guard: source.guard,
            family: source.family,
        }),
        identity,
        completion,
        directory: pending.directory().to_path_buf(),
        programs,
        sessions,
        rows,
        payload,
        bytes,
        cohort,
        resolutions,
        training_context: oos::TrainingContext::new(&request, commit),
        anchors,
    };
    committed.require_current()?;
    Ok(committed)
}

fn refuse(attempt: Attempt, why: String) -> String {
    match attempt.finish(Completion::Refused) {
        Ok(()) => why,
        Err(terminal) => format!("{why}; Boolean candidate terminal also refused: {terminal}"),
    }
}

fn validate_request(request: &Request<'_>) -> Result<(), String> {
    if [
        request.bounds.programs,
        request.bounds.coordinates,
        request.bounds.trades,
        request.bounds.bytes,
    ]
    .contains(&0)
        || request.programs.is_empty()
        || request.programs.len() as u64 > request.bounds.programs
        || request.horizon.as_bars() == 0
        || request.long.side() != Side::Long
        || request.short.side() != Side::Short
    {
        return Err("Boolean candidate catalog/policies/physical limits are invalid".to_owned());
    }
    let mut identities = std::collections::HashSet::new();
    identities
        .try_reserve(request.programs.len())
        .map_err(display)?;
    for program in request.programs {
        if !identities.insert(hash(&program.encode())) {
            return Err("Boolean candidate catalog contains a duplicate exact program".to_owned());
        }
    }
    let coordinates = request
        .long
        .max_cells()
        .checked_add(request.short.max_cells())
        .and_then(|per| per.checked_mul(request.programs.len() as u64))
        .ok_or("Boolean candidate cell admission overflow")?;
    if coordinates > request.bounds.coordinates {
        return Err(
            "Boolean candidate complete grid admission exceeds coordinate ceiling".to_owned(),
        );
    }
    Ok(())
}

fn load_source(request: &Request<'_>) -> Result<Source, String> {
    let (data, guard) = RangeInputs::load(RangeRequest {
        store_root: request.store,
        vendor: request.vendor,
        underlying: request.underlying,
        rung: request.rung,
        from: request.from,
        to: request.to,
        receipt_root: request.inputs.receipt_root(),
        max_bytes: request.inputs.max_bytes(),
        max_records: request.inputs.max_records(),
    })?
    .into_parts();
    let family = ResearchFamilyV1::new(data.signal.key).map_err(display)?;
    let mut binding = Hasher::new();
    binding.update(b"brutex-boolean-strict-input-policy-v1\0");
    binding.update(&request.inputs.max_bytes().to_le_bytes());
    binding.update(&request.inputs.max_records().to_le_bytes());
    let identity = guard.bind_digest(binding.finalize());
    guard.require_current()?;
    Ok(Source {
        data,
        guard,
        family,
        identity,
    })
}

fn reference(source: &Source) -> DailyReferenceBinding<'_> {
    DailyReferenceBinding {
        daily_bars: &source.data.daily.bars,
        eligibility: &source.data.daily.eligibility,
        schema: crate::stored::DAILY_REFERENCE_SCHEMA,
        eligibility_policy: crate::stored::DAILY_ELIGIBILITY_POLICY,
        gap_overlay_policy: crate::stored::EXACT_MINUTE_GAP_POLICY,
        excluded_ist_days: &indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS,
        daily_integrity: ReferenceIntegrity::ChecksumReceiptV1(source.identity),
        minute_integrity: ReferenceIntegrity::ChecksumReceiptV1(source.identity),
        swept_series_calendar_policy: crate::stored::SWEPT_SERIES_CALENDAR_POLICY,
    }
}

fn catalog_identity(
    request: &Request<'_>,
    source: &Source,
    commit: &str,
    cohort: [u8; 32],
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(b"brutex-boolean-candidate-catalog-v1\0");
    hasher.update(&source.family.encode());
    hasher.update(&source.identity);
    hasher.update(&cohort);
    hasher.update(&request.long.digest());
    hasher.update(&request.short.digest());
    hasher.update(&request.horizon.as_bars().to_le_bytes());
    hasher.update(&[request.from.1, request.to.1]);
    hasher.update(&request.from.0.to_le_bytes());
    hasher.update(&request.to.0.to_le_bytes());
    for limit in [
        request.bounds.programs,
        request.bounds.coordinates,
        request.bounds.trades,
        request.bounds.bytes,
    ] {
        hasher.update(&limit.to_le_bytes());
    }
    for program in request.programs {
        hasher.update(&program.encode());
    }
    // The full 32-byte catalog and strict source identity live in the data term.
    runner::identity::identity(&Run {
        mask: vocab::ConditionMask::default(),
        direction: Direction::Undirected,
        instrument: &source.data.signal.key,
        timeframe: request.rung,
        params: Params {
            min_hits: 0,
            ceiling: request.bounds.coordinates,
            pair_budget: request.bounds.programs,
            policy: 1,
        },
        data_digest: hasher.finalize(),
        commit,
        feed: request.vendor.as_str(),
    })
    .bytes()
}

fn cohort_digest(request: &Request<'_>, commit: &str) -> Result<[u8; 32], String> {
    let mut hasher = Hasher::new();
    hasher.update(b"brutex-boolean-candidate-common-policy-v1\0");
    for text in [request.rung, request.vendor.as_str(), commit] {
        hasher.update(&(text.len() as u64).to_le_bytes());
        hasher.update(text.as_bytes());
    }
    // Bind both kind-derived evaluator modes; the actual mode remains a family
    // fact. No cash family acquires the index's absent-volume interpretation.
    for availability in [
        indicators::vwap::Availability::Absent,
        indicators::vwap::Availability::Present,
    ] {
        let mut evaluator =
            indicators::evaluator::Evaluator::new(request.widths, availability, request.thresholds);
        let column = Column::build(&[], &mut evaluator);
        let token = column
            .evaluation_spec_token()
            .ok_or("Boolean evaluator specification absent")?;
        hasher.update(token.fingerprint_v1().as_bytes());
    }
    hasher.update(&crate::stored::calendar_policy_digest_v2());
    hasher.update(&vocab::VOCAB_VERSION.to_le_bytes());
    hasher.update(&request.long.digest());
    hasher.update(&request.short.digest());
    hasher.update(&request.horizon.as_bars().to_le_bytes());
    hasher.update(&[request.from.1, request.to.1]);
    hasher.update(&request.from.0.to_le_bytes());
    hasher.update(&request.to.0.to_le_bytes());
    for limit in [
        request.bounds.programs,
        request.bounds.coordinates,
        request.bounds.trades,
        request.bounds.bytes,
        request.inputs.max_bytes(),
        request.inputs.max_records(),
    ] {
        hasher.update(&limit.to_le_bytes());
    }
    for program in request.programs {
        hasher.update(&program.encode());
    }
    Ok(hasher.finalize())
}

type Computed = (
    Vec<Expression>,
    Vec<i64>,
    Vec<BooleanCoordinateV1>,
    [ResearchResolvedExitGridV1; 2],
    Vec<ExpressionTrainingAnchorV1>,
);

fn prepare_column(request: &Request<'_>, source: &Source) -> Result<(Column, Sessions), String> {
    source.guard.require_current()?;
    let data = &source.data;
    let execution = data
        .execution
        .as_ref()
        .map_or(data.signal.bars.as_slice(), |span| span.bars.as_slice());
    let span = super::RequestedSpanIdentityV1::new(
        request.from.0,
        request.from.1,
        request.to.0,
        request.to.1,
    )?;
    let (first, last) = super::requested_span_days(span)?;
    let rung = u32::try_from(crate::stored::rung_length_micros(request.rung)? / 1_000_000)
        .map_err(display)?;
    let signal_calendar =
        crate::stored::calendar_receipt_v2_for_bars(&data.signal.bars, rung, first, last)?
            .require_complete()?;
    let execution_calendar =
        crate::stored::calendar_receipt_v2_for_bars(execution, 60, first, last)?
            .require_complete()?;
    let coverage = super::CandidateCalendarCoverageV1::from_complete(
        rung,
        span,
        signal_calendar,
        execution_calendar,
    )?;
    super::require_exact_calendar(
        "Boolean signal",
        &data.signal.bars,
        rung,
        coverage,
        signal_calendar,
    )?;
    super::require_exact_calendar(
        "Boolean execution",
        execution,
        60,
        coverage,
        execution_calendar,
    )?;
    let evaluation = super::CandidateEvaluationInputsV1 {
        widths: request.widths,
        availability: crate::stored::vwap_availability(&data.signal.key),
        thresholds: request.thresholds,
    };
    let (_, column) = super::build_candidate_columns(
        &data.signal.bars,
        &data.daily.references,
        &data.exact_minute.bars,
        execution,
        rung,
        &evaluation,
    )?;
    let session_index = Sessions::new(execution, &column, first, last)?;
    Ok((column, session_index))
}

fn compute(
    request: &Request<'_>,
    source: &Source,
    catalog: [u8; 32],
    commit: &str,
) -> Result<Computed, String> {
    let (column, session_index) = prepare_column(request, source)?;
    let data = &source.data;
    let execution = data
        .execution
        .as_ref()
        .map_or(data.signal.bars.as_slice(), |span| span.bars.as_slice());
    let series = ExecutionSeriesV1::new(
        &data.signal.key,
        request.vendor.as_str(),
        commit,
        crate::stored::calendar_policy_digest_v2(),
        execution,
    )
    .map_err(display)?;
    let resolutions = [
        request
            .long
            .resolve_research_attested(series)
            .map_err(display)?,
        request
            .short
            .resolve_research_attested(series)
            .map_err(display)?,
    ];
    let mut rows = Vec::new();
    let count = resolutions
        .iter()
        .try_fold(0_u64, |count, resolved| {
            count
                .checked_add(resolved.cell_count())
                .ok_or("Boolean coordinate count overflow")
        })?
        .checked_mul(request.programs.len() as u64)
        .ok_or("Boolean catalog coordinate count overflow")?;
    let base = persistence::base_size(request.programs.len(), session_index.days.len())?
        .checked_add(grid_context::size(&resolutions)?)
        .ok_or("Boolean complete descriptor size overflow")?;
    let minimum = persistence::row_size(session_index.days.len(), 0)?
        .checked_mul(count)
        .and_then(|size| size.checked_add(base))
        .ok_or("Boolean minimum evidence size overflow")?;
    if minimum > request.bounds.bytes {
        return Err(
            "Boolean complete coordinate/session evidence exceeds byte ceiling before replay"
                .to_owned(),
        );
    }
    rows.try_reserve_exact(usize::try_from(count).map_err(display)?)
        .map_err(display)?;
    let mut remaining = Remaining {
        trades: request.bounds.trades,
        bytes: request.bounds.bytes - base,
    };
    let mut anchors = Vec::new();
    anchors
        .try_reserve_exact(
            request
                .programs
                .len()
                .checked_mul(2)
                .ok_or("Boolean anchor count overflow")?,
        )
        .map_err(display)?;
    for (program_index, program) in request.programs.iter().enumerate() {
        for resolved in &resolutions {
            let (mut produced, anchor) = produce_side(
                request,
                source,
                catalog,
                commit,
                program_index,
                program,
                resolved,
                &column,
                series,
                &session_index,
                &mut remaining,
            )?;
            rows.append(&mut produced);
            anchors.push(anchor);
        }
    }
    if rows.len() as u64 != count {
        return Err("Boolean complete coordinate count differs".to_owned());
    }
    source.guard.require_current()?;
    let mut programs = Vec::new();
    programs
        .try_reserve_exact(request.programs.len())
        .map_err(display)?;
    programs.extend_from_slice(request.programs);
    Ok((programs, session_index.days, rows, resolutions, anchors))
}

#[expect(
    clippy::too_many_arguments,
    reason = "one exact program/side transaction retains all source and physical authorities"
)]
fn produce_side(
    request: &Request<'_>,
    source: &Source,
    catalog: [u8; 32],
    commit: &str,
    program_index: usize,
    program: &Expression,
    resolved: &ResearchResolvedExitGridV1,
    column: &Column,
    series: ExecutionSeriesV1<'_>,
    sessions: &Sessions,
    remaining: &mut Remaining,
) -> Result<(Vec<BooleanCoordinateV1>, ExpressionTrainingAnchorV1), String> {
    let data_digest = runner::identity::data_digest_with_daily_reference(
        &source.data.signal.bars,
        &source.data.exact_minute.bars,
        reference(source),
    )
    .map_err(|why| format!("Boolean daily identity refused: {why:?}"))?;
    let run = Run {
        mask: program.referenced(),
        direction: match resolved.side() {
            Side::Long => Direction::Long,
            Side::Short => Direction::Short,
        },
        instrument: &source.data.signal.key,
        timeframe: request.rung,
        params: Params {
            min_hits: 0,
            ceiling: request.bounds.coordinates,
            pair_budget: request.bounds.programs,
            policy: 1,
        }
        .with_policy(&catalog_words(catalog)),
        data_digest,
        commit,
        feed: request.vendor.as_str(),
    };
    let run = ExpressionExecutionRunV1::new_with_daily_reference(
        &run,
        program,
        &source.data.signal.bars,
        &source.data.exact_minute.bars,
        series.bars(),
        reference(source),
    )?;
    let attempt =
        crate::sweep_evidence::begin(request.output, run.run_id().bytes(), Operation::Expression)?;
    let produced = (|| {
        let attested = resolved
            .attest_training(series, column, request.horizon)
            .map_err(display)?;
        let evaluated = resolved.evaluate_expression_with_attested(&attested, &run)?;
        let valid = resolved.validate_expression_evaluation(&evaluated)?;
        let mut rows = Vec::new();
        rows.try_reserve_exact(evaluated.grid().cells.len())
            .map_err(display)?;
        for (ordinal, cell) in evaluated.grid().cells.iter().enumerate() {
            let bytes = persistence::row_size(sessions.days.len(), cell.trades)?;
            if cell.trades > remaining.trades || bytes > remaining.bytes {
                return Err(format!(
                    "Boolean exact trade evidence exceeds physical ceiling: program_index={program_index} side={:?} ordinal={ordinal} needed_trades={} needed_bytes={bytes} remaining_trades={} remaining_bytes={}",
                    resolved.side(),
                    cell.trades,
                    remaining.trades,
                    remaining.bytes
                ));
            }
            let disposition =
                resolved.classify_expression_coordinate(&valid, Chosen::from_cell(cell))?;
            let trades = resolved.materialize_expression_coordinate(&attested, &valid, ordinal)?;
            if trades.len() as u64 != cell.trades {
                return Err("Boolean materialized trade count changed".to_owned());
            }
            remaining.trades -= cell.trades;
            remaining.bytes -= bytes;
            let periods = sessions.observe(series.bars(), cell, &trades)?;
            let mut identity = Hasher::new();
            identity.update(b"brutex-boolean-candidate-coordinate-v1\0");
            identity.update(&catalog);
            identity.update(&source.family.encode());
            identity.update(&disposition.digest());
            rows.push(BooleanCoordinateV1 {
                identity: identity.finalize(),
                program_index,
                run: run.run_id().bytes(),
                side: resolved.side(),
                ordinal: ordinal as u64,
                cell: *cell,
                refusal: disposition.refusal_bits(),
                periods,
                trades,
                selected: disposition.selected().cloned(),
                summary: evaluated.summary(),
                support_sessions: evaluated.support_sessions(),
            });
        }
        source.guard.require_current()?;
        Ok((rows, valid.later_period_anchor()))
    })();
    match produced {
        Ok(rows) => {
            attempt.finish(Completion::Completed)?;
            Ok(rows)
        }
        Err(why) => Err(refuse(attempt, why)),
    }
}

struct Remaining {
    trades: u64,
    bytes: u64,
}

fn catalog_words(digest: [u8; 32]) -> [u64; 4] {
    let mut words = [0; 4];
    for (word, bytes) in words.iter_mut().zip(digest.chunks_exact(8)) {
        let mut raw = [0; 8];
        raw.copy_from_slice(bytes);
        *word = u64::from_le_bytes(raw);
    }
    words
}

struct Sessions {
    days: Vec<i64>,
    first: i64,
    positions: Vec<Option<usize>>,
}
impl Sessions {
    fn new(
        bars: &[indicators::Candle],
        column: &Column,
        first: i64,
        last: i64,
    ) -> Result<Self, String> {
        let count = last
            .checked_sub(first)
            .and_then(|v| v.checked_add(1))
            .ok_or("Boolean session span overflow")?;
        let count = usize::try_from(count).map_err(display)?;
        let mut positions = Vec::new();
        positions.try_reserve_exact(count).map_err(display)?;
        positions.resize(count, None);
        let mut days = Vec::new();
        days.try_reserve_exact(count).map_err(display)?;
        for (index, bar) in bars.iter().enumerate() {
            if !column.accepts(index) {
                continue;
            }
            let day = indicators::ist_day(bar.ts_micros);
            let offset = usize::try_from(
                day.checked_sub(first)
                    .ok_or("Boolean session day underflow")?,
            )
            .map_err(display)?;
            let slot = positions
                .get_mut(offset)
                .ok_or("Boolean accepted day outside calendar")?;
            if slot.is_none() {
                *slot = Some(days.len());
                days.push(day);
            }
        }
        if days.is_empty() {
            return Err("Boolean observations have no accepted execution sessions".to_owned());
        }
        Ok(Self {
            days,
            first,
            positions,
        })
    }
    fn observe(
        &self,
        bars: &[indicators::Candle],
        cell: &Cell,
        trades: &[TradeRow],
    ) -> Result<Vec<BooleanSessionV1>, String> {
        let mut periods = Vec::new();
        periods
            .try_reserve_exact(self.days.len())
            .map_err(display)?;
        periods.extend(self.days.iter().map(|day| BooleanSessionV1 {
            day: *day,
            return_paisa: 0,
            trades: 0,
            wins: 0,
        }));
        for trade in trades {
            let entry = bars
                .get(trade.entry_bar)
                .ok_or("Boolean trade entry outside source")?;
            let exit = bars
                .get(trade.exit_bar)
                .ok_or("Boolean trade exit outside source")?;
            let day = indicators::ist_day(trade.exit_micros);
            if trade.entry_bar > trade.exit_bar
                || entry.ts_micros != trade.entry_micros
                || exit.ts_micros != trade.exit_micros
                || indicators::ist_day(trade.entry_micros) != day
            {
                return Err("Boolean observation trade source crosswire".to_owned());
            }
            let offset = usize::try_from(
                day.checked_sub(self.first)
                    .ok_or("Boolean trade day underflow")?,
            )
            .map_err(display)?;
            let position = self
                .positions
                .get(offset)
                .copied()
                .flatten()
                .ok_or("Boolean trade day was not accepted")?;
            let period = periods
                .get_mut(position)
                .ok_or("Boolean session position missing")?;
            period.return_paisa = period
                .return_paisa
                .checked_add(trade.worst)
                .ok_or("Boolean session return overflow")?;
            period.trades = period
                .trades
                .checked_add(1)
                .ok_or("Boolean session count overflow")?;
            period.wins = period
                .wins
                .checked_add(u64::from(trade.worst > 0))
                .ok_or("Boolean session wins overflow")?;
        }
        let mut actual = (0_i64, 0_u64, 0_u64);
        for period in &periods {
            actual.0 = actual
                .0
                .checked_add(period.return_paisa)
                .ok_or("Boolean total return overflow")?;
            actual.1 = actual
                .1
                .checked_add(period.trades)
                .ok_or("Boolean total trades overflow")?;
            actual.2 = actual
                .2
                .checked_add(period.wins)
                .ok_or("Boolean total wins overflow")?;
        }
        if actual != (cell.pessimistic, cell.trades, cell.wins) {
            return Err("Boolean session observations do not reconcile with exact cell".to_owned());
        }
        Ok(periods)
    }
}

fn display(why: impl std::fmt::Display) -> String {
    why.to_string()
}

#[cfg(test)]
type CommitHook = Box<dyn FnOnce(&Path)>;
#[cfg(test)]
std::thread_local! {static COMMIT_HOOK:std::cell::RefCell<Option<CommitHook>>=const{std::cell::RefCell::new(None)};}
#[cfg(test)]
struct CommitFault;
#[cfg(test)]
impl CommitFault {
    fn install(hook: impl FnOnce(&Path) + 'static) -> Self {
        COMMIT_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
        Self
    }
}
#[cfg(test)]
impl Drop for CommitFault {
    fn drop(&mut self) {
        COMMIT_HOOK.with(|slot| {
            slot.borrow_mut().take();
        });
    }
}
#[cfg(test)]
fn commit_fault(path: &Path) {
    if let Some(hook) = COMMIT_HOOK.with(|slot| slot.borrow_mut().take()) {
        hook(path);
    }
}

#[cfg(test)]
#[path = "boolean_candidate_tests.rs"]
pub(crate) mod tests;
