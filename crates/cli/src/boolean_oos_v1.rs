//! Separate later-period comparison of every original program and frozen exit.
//! Completion proves a measured comparison, never admission or portfolio selection.
use super::{
    BooleanCoordinateV1, Bounds, Chosen, CommittedBooleanFamilyV1, Completion, Direction,
    ExecutionSeriesV1, Expression, ExpressionExecutionRunV1, Hasher, Horizon, Operation, Params,
    Path, PathBuf, Remaining, Request, RetainedSource, Run, Sessions, Side, Source, StrictConfig,
    catalog_words, cohort_digest, display, grid_context, hash, load_source, persistence,
    prepare_column, reference, refuse, validate_request,
};
use runner::exit_grid_policy::expression_execution::later_period::EvaluatedExpressionOosV1;
use runner::exit_grid_policy::expression_execution::later_period::validation::{
    BoundFixedTrainingFoldsV1, FixedTrainingFoldPlanV1, FixedTrainingFoldProjectionV1,
    FixedTrainingFoldV1, LaterSessionWindowV1,
};

#[path = "boolean_oos_reader.rs"]
pub(crate) mod reader;

pub(super) struct TrainingContext {
    store: PathBuf,
    vendor: brutex_core::vendor::Vendor,
    rung: String,
    underlying: String,
    from: (u16, u8),
    to: (u16, u8),
    horizon: Horizon,
    commit: String,
    widths: indicators::evaluator::Widths,
    thresholds: indicators::pattern::Thresholds,
}
impl TrainingContext {
    pub(super) fn new(request: &Request<'_>, commit: &str) -> Self {
        Self {
            store: request.store.to_path_buf(),
            vendor: request.vendor,
            rung: request.rung.to_owned(),
            underlying: request.underlying.to_owned(),
            from: request.from,
            to: request.to,
            horizon: request.horizon,
            commit: commit.to_owned(),
            widths: request.widths,
            thresholds: request.thresholds,
        }
    }
}
/// Later dates and physical ceilings only: training defines the program/grid policy.
#[derive(Clone, Copy)]
pub(crate) struct LaterRequest<'a> {
    pub output: &'a Path,
    pub inputs: &'a StrictConfig,
    pub from: (u16, u8),
    pub to: (u16, u8),
    pub bounds: Bounds,
}
/// A declared full-month partition and additional proof allocation ceilings.
/// These are fixed-training later windows, not prefix-retrained anchored folds.
#[derive(Clone, Copy)]
pub(crate) struct ValidationRequest<'a> {
    pub requested: LaterSessionWindowV1,
    pub windows: &'a [LaterSessionWindowV1],
    pub max_folds: usize,
    /// One current program/side's bar mapping and session-count buffers.
    pub mapping_bytes: u64,
    /// Retained coordinate options/fold records plus one current fold plan.
    pub projection_bytes: u64,
}
impl ValidationRequest<'_> {
    fn validate(
        self,
        training: &CommittedBooleanFamilyV1,
        later: LaterRequest<'_>,
    ) -> Result<(), String> {
        let first = pull::session::Day::new(later.from.0, later.from.1, 1).map_err(display)?;
        let last = pull::session::Day::new(later.to.0, later.to.1, 1)
            .map_err(display)?
            .end_of_month();
        if self.requested.first_day() != i64::from(first.days_from_epoch())
            || self.requested.last_day() != i64::from(last.days_from_epoch())
        {
            return Err("Boolean validation dates differ from complete later month request".into());
        }
        if self.windows.is_empty() || self.windows.len() > self.max_folds {
            return Err("Boolean validation fold count exceeds physical ceiling".into());
        }
        let selected = training
            .rows
            .iter()
            .filter(|row| row.selected.is_some())
            .count();
        let bytes = selected
            .checked_mul(self.windows.len())
            .and_then(|count| count.checked_mul(size_of::<FixedTrainingFoldV1>()))
            .and_then(|folds| {
                training
                    .rows
                    .len()
                    .checked_mul(size_of::<Option<FixedTrainingFoldProjectionV1>>())
                    .and_then(|slots| slots.checked_add(folds))
            })
            .and_then(|retained| {
                self.windows
                    .len()
                    .checked_mul(size_of::<LaterSessionWindowV1>())
                    .and_then(|windows| windows.checked_add(size_of::<FixedTrainingFoldPlanV1>()))
                    .and_then(|plan| retained.checked_add(plan))
            })
            .ok_or("Boolean validation retained projection size overflow")?;
        if u64::try_from(bytes).map_err(display)? > self.projection_bytes {
            return Err("Boolean validation retained projections exceed physical ceiling".into());
        }
        Ok(())
    }
}
/// Only this live producer can retain original and later strict source authorities.
pub(crate) struct CommittedBooleanOosV1<'a> {
    training: &'a CommittedBooleanFamilyV1,
    source: RetainedSource,
    source_identity: [u8; 32],
    identity: [u8; 32],
    completion: [u8; 32],
    payload: [u8; 32],
    bytes: u64,
    directory: PathBuf,
    rows: Vec<BooleanCoordinateV1>,
    sessions: Vec<i64>,
    fold_projections: Vec<Option<FixedTrainingFoldProjectionV1>>,
}
impl CommittedBooleanOosV1<'_> {
    pub(crate) fn require_current(&self) -> Result<(), String> {
        self.training.require_current()?;
        self.source.guard.require_current()?;
        persistence::verify(
            &self.directory,
            self.identity,
            self.payload,
            self.bytes,
            self.completion,
        )?;
        self.training.require_current()?;
        self.source.guard.require_current()
    }
    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    pub(crate) const fn completion_digest(&self) -> [u8; 32] {
        self.completion
    }
    pub(crate) const fn training_identity(&self) -> [u8; 32] {
        self.training.identity
    }
    pub(crate) const fn training_completion_digest(&self) -> [u8; 32] {
        self.training.completion
    }
    pub(crate) const fn source_identity(&self) -> [u8; 32] {
        self.source_identity
    }
    pub(crate) fn rows(&self) -> &[BooleanCoordinateV1] {
        &self.rows
    }
    pub(crate) fn sessions(&self) -> &[i64] {
        &self.sessions
    }
    /// Aligned with every original coordinate. Missing original selection or an
    /// ordinary V1 comparison has no fold authority. These proofs are not part
    /// of the V1 receipt: the caller must persist their distinct plan/bytes.
    pub(crate) fn fold_projections(&self) -> &[Option<FixedTrainingFoldProjectionV1>] {
        &self.fold_projections
    }
}
/// Execute only with the exact clean build that captured the training authority.
pub(crate) fn produce<'a>(
    training: &'a CommittedBooleanFamilyV1,
    later: LaterRequest<'_>,
) -> Result<CommittedBooleanOosV1<'a>, String> {
    let commit =
        crate::commit_stamp().ok_or("Boolean later comparison requires a clean build identity")?;
    produce_identified(training, later, commit)
}
/// Compute the existing V1 comparison and additional original-exit fold proof
/// in the same materialization pass. The caller records the declared validation
/// request before invoking this path and persists its separate proof identity.
pub(crate) fn produce_validated<'a>(
    training: &'a CommittedBooleanFamilyV1,
    later: LaterRequest<'_>,
    validation: ValidationRequest<'_>,
) -> Result<CommittedBooleanOosV1<'a>, String> {
    let commit =
        crate::commit_stamp().ok_or("Boolean later comparison requires a clean build identity")?;
    produce_with_validation_identified(training, later, Some(validation), commit)
}
fn request<'a>(
    training: &'a CommittedBooleanFamilyV1,
    later: LaterRequest<'a>,
    commit: &str,
) -> Result<Request<'a>, String> {
    let context = &training.training_context;
    if commit != context.commit || later.from > later.to || later.from <= context.to {
        return Err(
            "Boolean later comparison requires matching build and distinct strictly later months"
                .into(),
        );
    }
    for (year, month) in [later.from, later.to] {
        pull::session::Day::new(year, month, 1).map_err(display)?;
    }
    let [long, short] = &training.resolutions;
    let request = Request {
        store: &context.store,
        output: later.output,
        vendor: context.vendor,
        underlying: &context.underlying,
        rung: &context.rung,
        from: later.from,
        to: later.to,
        horizon: context.horizon,
        programs: &training.programs,
        long: long.policy(),
        short: short.policy(),
        inputs: later.inputs,
        bounds: later.bounds,
        widths: context.widths,
        thresholds: context.thresholds,
    };
    validate_request(&request)?;
    if training.anchors.len()
        != training
            .programs
            .len()
            .checked_mul(2)
            .ok_or("Boolean anchor count overflow")?
    {
        return Err("Boolean training anchors incomplete".into());
    }
    Ok(request)
}
fn produce_identified<'a>(
    training: &'a CommittedBooleanFamilyV1,
    later: LaterRequest<'_>,
    commit: &str,
) -> Result<CommittedBooleanOosV1<'a>, String> {
    produce_with_validation_identified(training, later, None, commit)
}
fn produce_with_validation_identified<'a>(
    training: &'a CommittedBooleanFamilyV1,
    later: LaterRequest<'_>,
    validation: Option<ValidationRequest<'_>>,
    commit: &str,
) -> Result<CommittedBooleanOosV1<'a>, String> {
    training.require_current()?;
    let request = request(training, later, commit)?;
    if let Some(validation) = validation {
        validation.validate(training, later)?;
    }
    let source = load_source(&request)?;
    if source.family != training.family() {
        return Err("Boolean later family differs from training".into());
    }
    let identity = identity(training, &request, &source, commit)?;
    let attempt = crate::sweep_evidence::begin(later.output, identity, Operation::BooleanOos)?;
    let computed = compute(training, &request, &source, identity, validation, commit);
    let Computed {
        rows,
        sessions,
        fold_projections,
    } = match computed {
        Ok(value) => value,
        Err(why) => return Err(refuse(attempt, why)),
    };
    let encoded = reader::encode(training, &source, &request, identity, &sessions, &rows);
    let body = match encoded {
        Ok(value) => value,
        Err(why) => return Err(refuse(attempt, why)),
    };
    let pending =
        match persistence::prepare_in_namespace(later.output, "boolean-oos-v1", identity, &body) {
            Ok(value) => value,
            Err(why) => return Err(refuse(attempt, why)),
        };
    let payload = hash(&body);
    let bytes = body.len() as u64;
    let completion = crate::finish_stored_month(
        attempt,
        Completion::Completed,
        || {
            training.require_current()?;
            source.guard.require_current()?;
            pending.verify_body(payload, bytes)
        },
        || pending.finish(identity, payload, bytes),
    )?;
    Ok(CommittedBooleanOosV1 {
        training,
        source: RetainedSource {
            guard: source.guard,
            family: source.family,
        },
        source_identity: source.identity,
        identity,
        completion,
        payload,
        bytes,
        directory: pending.directory().to_path_buf(),
        rows,
        sessions,
        fold_projections,
    })
}
fn identity(
    training: &CommittedBooleanFamilyV1,
    request: &Request<'_>,
    source: &Source,
    commit: &str,
) -> Result<[u8; 32], String> {
    let mut h = Hasher::new();
    h.update(b"brutex.boolean-later-period.v1\0");
    for value in [
        training.identity,
        training.completion,
        training.cohort,
        source.identity,
        cohort_digest(request, commit)?,
    ] {
        h.update(&value);
    }
    for anchor in &training.anchors {
        h.update(&anchor.digest());
    }
    for bound in [
        u64::from(training.training_context.from.0),
        u64::from(training.training_context.from.1),
        u64::from(training.training_context.to.0),
        u64::from(training.training_context.to.1),
    ] {
        h.update(&bound.to_le_bytes());
    }
    Ok(runner::identity::identity(&Run {
        mask: vocab::ConditionMask::ZERO,
        direction: Direction::Undirected,
        instrument: &source.data.signal.key,
        timeframe: request.rung,
        params: Params {
            min_hits: 0,
            ceiling: request.bounds.coordinates,
            pair_budget: request.bounds.programs,
            policy: 1,
        },
        data_digest: h.finalize(),
        commit,
        feed: request.vendor.as_str(),
    })
    .bytes())
}
struct Computed {
    rows: Vec<BooleanCoordinateV1>,
    sessions: Vec<i64>,
    fold_projections: Vec<Option<FixedTrainingFoldProjectionV1>>,
}
fn prepare_rows(
    training: &CommittedBooleanFamilyV1,
    request: &Request<'_>,
    sessions: usize,
) -> Result<(Computed, Remaining), String> {
    let base = persistence::base_size(training.programs.len(), sessions)?
        .checked_add(grid_context::size(&training.resolutions)?)
        .and_then(|value| value.checked_add(reader::HEADER as u64))
        .ok_or("Boolean later descriptor overflow")?;
    let minimum = persistence::row_size(sessions, 0)?
        .checked_mul(training.rows.len() as u64)
        .and_then(|value| value.checked_add(base))
        .ok_or("Boolean later population size overflow")?;
    let options = training
        .rows
        .len()
        .checked_mul(size_of::<Option<FixedTrainingFoldProjectionV1>>())
        .ok_or("Boolean later projection slots overflow")?;
    if minimum > request.bounds.bytes
        || options as u64 > request.bounds.bytes
        || training.rows.len() as u64 > request.bounds.coordinates
    {
        return Err(
            "Boolean later complete population exceeds physical ceiling before replay".into(),
        );
    }
    let mut computed = Computed {
        rows: Vec::new(),
        sessions: Vec::new(),
        fold_projections: Vec::new(),
    };
    computed
        .rows
        .try_reserve_exact(training.rows.len())
        .map_err(display)?;
    computed
        .fold_projections
        .try_reserve_exact(training.rows.len())
        .map_err(display)?;
    Ok((
        computed,
        Remaining {
            trades: request.bounds.trades,
            bytes: request.bounds.bytes - base,
        },
    ))
}
fn compute(
    training: &CommittedBooleanFamilyV1,
    request: &Request<'_>,
    source: &Source,
    identity: [u8; 32],
    validation: Option<ValidationRequest<'_>>,
    commit: &str,
) -> Result<Computed, String> {
    let (column, sessions) = prepare_column(request, source)?;
    let execution = execution(source);
    let series = ExecutionSeriesV1::new(
        &source.data.signal.key,
        request.vendor.as_str(),
        commit,
        crate::stored::calendar_policy_digest_v2(),
        execution,
    )
    .map_err(display)?;
    let (mut computed, mut remaining) = prepare_rows(training, request, sessions.days.len())?;
    for (group, anchor) in training.anchors.iter().enumerate() {
        let program = training
            .programs
            .get(group / 2)
            .ok_or("Boolean later training program absent")?;
        let resolved = training
            .resolutions
            .get(group % 2)
            .ok_or("Boolean later resolution absent")?;
        if program != anchor.program() || anchor.resolution_digest() != resolved.digest() {
            return Err("Boolean later training anchor crosswire".into());
        }
        if training.rows.get(computed.rows.len()).map(|row| row.run)
            != Some(anchor.run_id().bytes())
        {
            return Err("Boolean later anchor differs from original directional run".into());
        }
        let run = execution_run(request, source, identity, commit, program, resolved.side())?;
        let attempt = crate::sweep_evidence::begin(
            request.output,
            run.run_id().bytes(),
            Operation::Expression,
        )?;
        let produced = (|| {
            let plan = validation
                .map(|validation| {
                    FixedTrainingFoldPlanV1::new(
                        resolved,
                        anchor,
                        validation.requested,
                        validation.windows,
                        validation.max_folds,
                    )
                })
                .transpose()?;
            let evaluated = resolved.evaluate_expression_oos(anchor, series, &column, &run)?;
            let bound = plan
                .as_ref()
                .zip(validation)
                .map(|(plan, validation)| plan.bind(&evaluated, validation.mapping_bytes))
                .transpose()?;
            append_rows(
                training,
                identity,
                group,
                &evaluated,
                &sessions,
                execution,
                bound.as_ref(),
                &mut remaining,
                &mut computed,
            )?;
            training.require_current()?;
            source.guard.require_current()
        })();
        match produced {
            Ok(()) => attempt.finish(Completion::Completed)?,
            Err(why) => return Err(refuse(attempt, why)),
        }
    }
    if computed.rows.len() != training.rows.len()
        || computed.fold_projections.len() != training.rows.len()
    {
        return Err("Boolean later population omitted original coordinates".into());
    }
    computed.sessions = sessions.days;
    Ok(computed)
}
fn execution(source: &Source) -> &[indicators::Candle] {
    source
        .data
        .execution
        .as_ref()
        .map_or(source.data.signal.bars.as_slice(), |span| {
            span.bars.as_slice()
        })
}
fn execution_run(
    request: &Request<'_>,
    source: &Source,
    identity: [u8; 32],
    commit: &str,
    program: &Expression,
    side: Side,
) -> Result<ExpressionExecutionRunV1, String> {
    let data_digest = runner::identity::data_digest_with_daily_reference(
        &source.data.signal.bars,
        &source.data.exact_minute.bars,
        reference(source),
    )
    .map_err(|why| format!("Boolean later daily identity: {why:?}"))?;
    ExpressionExecutionRunV1::new_with_daily_reference(
        &Run {
            mask: program.referenced(),
            direction: if side == Side::Long {
                Direction::Long
            } else {
                Direction::Short
            },
            instrument: &source.data.signal.key,
            timeframe: request.rung,
            params: Params {
                min_hits: 0,
                ceiling: request.bounds.coordinates,
                pair_budget: request.bounds.programs,
                policy: 1,
            }
            .with_policy(&catalog_words(identity)),
            data_digest,
            commit,
            feed: request.vendor.as_str(),
        },
        program,
        &source.data.signal.bars,
        &source.data.exact_minute.bars,
        execution(source),
        reference(source),
    )
}
#[expect(
    clippy::too_many_arguments,
    reason = "one complete later grid retains original coordinate and strict source authorities"
)]
fn append_rows(
    training: &CommittedBooleanFamilyV1,
    identity: [u8; 32],
    group: usize,
    evaluated: &EvaluatedExpressionOosV1<'_>,
    sessions: &Sessions,
    bars: &[indicators::Candle],
    validation: Option<&BoundFixedTrainingFoldsV1<'_>>,
    remaining: &mut Remaining,
    computed: &mut Computed,
) -> Result<(), String> {
    for (ordinal, cell) in evaluated.grid().cells.iter().enumerate() {
        let original = training
            .rows
            .get(computed.rows.len())
            .ok_or("Boolean original coordinate absent")?;
        if original.program_index != group / 2
            || usize::from(original.side == Side::Short) != group % 2
            || original.ordinal != ordinal as u64
            || Chosen::from_cell(&original.cell) != Chosen::from_cell(cell)
        {
            return Err("Boolean later coordinate differs from frozen original".into());
        }
        let bytes = persistence::row_size(sessions.days.len(), cell.trades)?;
        if cell.trades > remaining.trades || bytes > remaining.bytes {
            return Err(format!(
                "Boolean later exact trade evidence exceeds physical ceiling: group={group} ordinal={ordinal} needed_trades={} needed_bytes={bytes} remaining_trades={} remaining_bytes={}",
                cell.trades, remaining.trades, remaining.bytes
            ));
        }
        let (trades, projection) = match validation.zip(original.selected.as_ref()) {
            Some((bound, selected)) => {
                let (trades, projection) = bound.materialize_coordinate(selected, ordinal)?;
                (trades, Some(projection))
            }
            None => (evaluated.materialize(ordinal)?, None),
        };
        let periods = sessions.observe(bars, cell, &trades)?;
        let mut h = Hasher::new();
        h.update(b"brutex.boolean-later-coordinate.v1\0");
        for value in [identity, original.identity, evaluated.digest()] {
            h.update(&value);
        }
        h.update(&(ordinal as u64).to_le_bytes());
        computed.rows.push(BooleanCoordinateV1 {
            identity: h.finalize(),
            program_index: group / 2,
            run: evaluated.run_id().bytes(),
            side: original.side,
            ordinal: ordinal as u64,
            cell: *cell,
            refusal: evaluated
                .refusal_bits(ordinal)
                .ok_or("Boolean later refusal comparison absent")?,
            periods,
            trades,
            selected: None,
            summary: evaluated.summary(),
            support_sessions: evaluated.support_sessions(),
        });
        computed.fold_projections.push(projection);
        remaining.trades -= cell.trades;
        remaining.bytes -= bytes;
    }
    Ok(())
}
#[cfg(test)]
pub(crate) fn source_identity_fixture(
    training: &CommittedBooleanFamilyV1,
    later: LaterRequest<'_>,
) -> Result<[u8; 32], String> {
    training.require_current()?;
    let request = request(training, later, "generated-boolean-candidate-fixture")?;
    let source = load_source(&request)?;
    source.guard.require_current()?;
    training.require_current()?;
    Ok(source.identity)
}
#[cfg(test)]
pub(crate) fn produce_fixture<'a>(
    training: &'a CommittedBooleanFamilyV1,
    later: LaterRequest<'_>,
    validation: Option<ValidationRequest<'_>>,
) -> Result<CommittedBooleanOosV1<'a>, String> {
    produce_with_validation_identified(
        training,
        later,
        validation,
        "generated-boolean-candidate-fixture",
    )
}
#[cfg(test)]
pub(crate) fn produce_campaign_fixture<'a>(
    training: &'a CommittedBooleanFamilyV1,
    later: LaterRequest<'_>,
    validation: ValidationRequest<'_>,
) -> Result<CommittedBooleanOosV1<'a>, String> {
    produce_with_validation_identified(
        training,
        later,
        Some(validation),
        "generated-boolean-campaign-fixture",
    )
}
#[cfg(test)]
#[path = "boolean_oos_tests.rs"]
mod tests;
