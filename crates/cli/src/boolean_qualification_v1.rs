//! Fixed-training later-window qualification over a predeclared finite family.
//! The original training and later OOS formats remain unchanged. No observation
//! read from disk can create this source-bound successor capability.
use super::super::super::{BooleanCoordinateV1, oos::CommittedBooleanOosV1};
use super::{CommittedBooleanAdmissionV1, base_values, display, persistence, probability};
use crate::population_statistics_v2::{PopulationStatisticsProcedureV2, wilson_lower_bits};
use crate::sweep_evidence::{Completion, Operation};
use brutex_core::blake3::{Hasher, hash};
use runner::admission::research_projection::{
    ResearchAdmissionProjectionV1, hypothesis_decision, wilson_ppm,
};
use runner::admission::{AdmissionPolicyV1, CompletenessV1, ObservedI64V1, ObservedU64V1};
use runner::bootstrap::{spa_receipt_v1, white_reality_check_receipt_v1};
use runner::bootstrap_zero_v2::{self as zero, Classification};
use runner::family_allocation_v1::Allocation;
use std::path::{Path, PathBuf};

#[path = "boolean_qualification_reader.rs"]
pub(crate) mod reader;
#[path = "boolean_qualification_wire.rs"]
mod wire;

pub(crate) const NAMESPACE: &str = "boolean-qualification-v1";

/// Explicit fixed numerical and output admission, bound into the run identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Bounds {
    pub candidates: u64,
    pub observations: u64,
    pub work: u64,
    pub memory: u64,
    pub bytes: u64,
}
impl Bounds {
    fn words(self) -> [u64; 5] {
        [
            self.candidates,
            self.observations,
            self.work,
            self.memory,
            self.bytes,
        ]
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Request<'a> {
    pub root: &'a Path,
    pub training: &'a CommittedBooleanAdmissionV1,
    pub later: &'a [CommittedBooleanOosV1<'a>],
    pub policy: AdmissionPolicyV1,
    pub procedure: PopulationStatisticsProcedureV2,
    /// Live predeclared plan; decoding saved observations cannot mint this value.
    pub plan: &'a crate::boolean_qualification_plan::Plan,
    pub rung: usize,
    pub bounds: Bounds,
}

#[derive(Clone)]
struct Row {
    original: [u8; 32],
    later: [u8; 32],
    family: usize,
    coordinate: usize,
    projection: ResearchAdmissionProjectionV1,
    /// classification, exact numerator, denominator; optional original shared
    /// strategy/rank/statistic/exceedances/initial/adjusted facts occupy8 words.
    romano: [u64; 12],
    fold: Option<Vec<u8>>,
}

#[derive(Clone)]
struct Manifest {
    identity: [u8; 32],
    original: [u8; 32],
    original_pin: [u8; 32],
    scope: [u8; 32],
    unit: [u8; 32],
    policy: AdmissionPolicyV1,
    procedure: [u64; 3],
    allocation: Allocation,
    bounds: Bounds,
    folds: usize,
    count: usize,
    periods: usize,
    family_digest: [u8; 32],
    shared_digest: Option<[u8; 32]>,
    /// statistic bits, p bits, numerator, denominator, exceedances for White/SPA.
    family_tests: [[u64; 5]; 2],
    later: Vec<([u8; 32], [u8; 32], usize)>,
    plan: Vec<u8>,
}

pub(crate) struct Committed<'a> {
    training: &'a CommittedBooleanAdmissionV1,
    later: &'a [CommittedBooleanOosV1<'a>],
    identity: [u8; 32],
    pin: [u8; 32],
    payload: [u8; 32],
    bytes: u64,
    directory: PathBuf,
    rows: Vec<Row>,
    consistency: Option<crate::index_consistency_store::Reader>,
}
impl Committed<'_> {
    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    pub(crate) const fn pin(&self) -> [u8; 32] {
        self.pin
    }
    pub(crate) fn count(&self) -> usize {
        self.rows.len()
    }
    pub(crate) fn counts(&self) -> [u64; 4] {
        let mut counts = [0; 4];
        for row in &self.rows {
            let count = match row.projection.verdict().status() {
                runner::admission::AdmissionStatusV1::Admitted => &mut counts[0],
                runner::admission::AdmissionStatusV1::Rejected => &mut counts[1],
                runner::admission::AdmissionStatusV1::Unmeasured => &mut counts[2],
                runner::admission::AdmissionStatusV1::Refused => &mut counts[3],
            };
            *count += 1;
        }
        counts
    }
    pub(crate) fn require_current(&self) -> Result<(), String> {
        current(self.training, self.later)?;
        persistence::verify(
            &self.directory,
            self.identity,
            self.payload,
            self.bytes,
            self.pin,
        )?;
        if let Some(consistency) = &self.consistency {
            consistency.require_current()?;
        }
        current(self.training, self.later)
    }
}

pub(crate) fn produce<'a>(request: &Request<'a>) -> Result<Committed<'a>, String> {
    current(request.training, request.later)?;
    let mut manifest = prepare(request)?;
    let attempt = crate::sweep_evidence::begin(
        request.root,
        manifest.identity,
        Operation::BooleanQualification,
    )?;
    let prepared = measure(request, &mut manifest)
        .and_then(|rows| wire::encode(&manifest, &rows).map(|body| (rows, body)));
    let (rows, body) = match prepared {
        Ok(result) => result,
        Err(why) => return Err(super::super::super::refuse(attempt, why)),
    };
    let pending = match persistence::prepare_in_namespace(
        request.root,
        NAMESPACE,
        manifest.identity,
        &body,
    ) {
        Ok(value) => value,
        Err(why) => return Err(super::super::super::refuse(attempt, why)),
    };
    let payload = hash(&body);
    let bytes = body.len() as u64;
    let pin = crate::finish_stored_month(
        attempt,
        Completion::Completed,
        || {
            current(request.training, request.later)?;
            pending.verify_body(payload, bytes)
        },
        || pending.finish(manifest.identity, payload, bytes),
    )?;
    let mut result = Committed {
        training: request.training,
        later: request.later,
        identity: manifest.identity,
        pin,
        payload,
        bytes,
        directory: pending.directory().to_path_buf(),
        rows,
        consistency: None,
    };
    result.require_current()?;
    if request.plan.index_policy().is_some() {
        let records = consistency_records(request, &result.rows)?;
        result.consistency = Some(crate::index_consistency_store::produce(
            request.root,
            result.identity,
            result.pin,
            records,
            request.bounds.bytes,
            || result.require_current(),
        )?);
    }
    result.require_current()?;
    Ok(result)
}

fn consistency_records(
    request: &Request<'_>,
    rows: &[Row],
) -> Result<Vec<crate::index_consistency_store::Record>, String> {
    use crate::index_consistency::Session;
    use crate::index_consistency_store::{Binding, Record};
    let requested = request.plan.requested()?;
    let original_span = request.plan.training_requested()?;
    let mut records = Vec::new();
    let mut admitted = (rows.len() as u64)
        .checked_mul(2 * std::mem::size_of::<Record>() as u64)
        .ok_or("consistency allocation overflow")?;
    if admitted > request.bounds.memory {
        return Err("index consistency settings exceed admitted working memory".into());
    }
    records.try_reserve_exact(rows.len()).map_err(display)?;
    for row in rows {
        let later = request
            .later
            .get(row.family)
            .ok_or("consistency later family absent")?;
        let coordinate = later
            .rows()
            .get(row.coordinate)
            .ok_or("consistency later coordinate absent")?;
        let original = request
            .training
            .statistics()
            .sources()
            .get(row.family)
            .ok_or("consistency original family absent")?;
        let before = original
            .rows()
            .get(row.coordinate)
            .ok_or("consistency original coordinate absent")?;
        let family = original.family();
        if coordinate.identity() != row.later {
            return Err("consistency coordinate identity differs".into());
        }
        // Admit both the retained daily observations and their wire copy before
        // allocating; the shared output budget remains a separate ceiling.
        let days = requested
            .last_day()
            .checked_sub(original_span.first_day())
            .and_then(|n| n.checked_add(1))
            .and_then(|n| u64::try_from(n).ok())
            .ok_or("consistency civil span overflow")?;
        let count = coordinate
            .periods()
            .len()
            .checked_add(before.periods().len())
            .ok_or("consistency session count overflow")?;
        admitted = admitted
            .checked_add(consistency_memory(count, days)?)
            .ok_or("consistency memory admission overflow")?;
        if admitted > request.bounds.memory {
            return Err("index consistency exceeds admitted working memory".into());
        }
        let training_session_count = before.periods().len();
        let mut sessions = Vec::new();
        sessions.try_reserve_exact(count).map_err(display)?;
        sessions.extend(
            before
                .periods()
                .iter()
                .chain(coordinate.periods())
                .map(|day| Session {
                    day: day.day(),
                    pessimistic_paisa: day.return_paisa(),
                    trades: day.trades(),
                }),
        );
        let [evaluation, training, later_evaluation] = consistency_assessments(
            family,
            [
                original_span.first_day(),
                original_span.last_day(),
                requested.first_day(),
                requested.last_day(),
            ],
            &sessions,
            training_session_count,
        )?;
        records.push(Record {
            binding: Binding {
                original: row.original,
                later: row.later,
                later_identity: later.identity(),
                later_completion: later.completion_digest(),
                source: later.source_identity(),
                family: row.family as u64,
                coordinate: row.coordinate as u64,
            },
            evaluation,
            training,
            later: later_evaluation,
            training_session_count,
            sessions,
        });
    }
    Ok(records)
}

fn consistency_assessments(
    family: runner::research_family::ResearchFamilyV1,
    [first, training_last, later_first, last]: [i64; 4],
    sessions: &[crate::index_consistency::Session],
    split: usize,
) -> Result<[crate::index_consistency::Evaluation; 3], String> {
    use crate::index_consistency::{Policy, evaluate};
    let training = sessions
        .get(..split)
        .ok_or("index training day extent missing")?;
    let later = sessions
        .get(split..)
        .ok_or("index later day extent missing")?;
    Ok([
        evaluate(Policy::V1, family, first, last, sessions, true),
        evaluate(Policy::V1, family, first, training_last, training, true),
        evaluate(Policy::V1, family, later_first, last, later, true),
    ])
}

fn consistency_memory(count: usize, days: u64) -> Result<u64, String> {
    use crate::index_consistency::{Evaluation, Session, Week};
    let sessions =
        (count as u64).checked_mul((std::mem::size_of::<Session>() + Session::BYTE_LEN) as u64);
    let weeks = days.div_ceil(7).checked_add(1).and_then(|count| {
        count.checked_mul(3 * (std::mem::size_of::<Week>() + Week::BYTE_LEN) as u64)
    });
    sessions
        .and_then(|n| weeks.and_then(|weeks| n.checked_add(weeks)))
        .and_then(|n| n.checked_add(3 * Evaluation::BYTE_LEN as u64 + 216))
        .and_then(|n| n.checked_mul(2))
        .ok_or_else(|| "index consistency peak memory estimate overflow".into())
}

fn current(
    training: &CommittedBooleanAdmissionV1,
    later: &[CommittedBooleanOosV1<'_>],
) -> Result<(), String> {
    training.require_current()?;
    for source in later {
        source.require_current()?;
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "complete fixed manifest keeps each admitted input beside its validation"
)]
fn prepare(request: &Request<'_>) -> Result<Manifest, String> {
    let original = request.training.statistics().sources();
    let count = request.training.rows().len();
    let allocation = request.plan.allocation(request.rung)?;
    let scope = request.plan.descriptor();
    let unit = *request
        .plan
        .units()
        .get(request.rung)
        .ok_or("qualification rung missing")?;
    let fold_count = request.plan.windows().len();
    let numerical = request.plan.numerical_bounds()?;
    let limits = request.plan.source_limits();
    if request.training.policy() != request.policy
        || request.policy.digest() != request.plan.policy_digest()
        || request.procedure != request.plan.procedure()
        || request.bounds.work != numerical.max_work
        || request.bounds.memory != numerical.max_bytes
        || request.bounds.bytes != limits[0]
        || request.bounds.candidates != limits[1]
        || request.bounds.observations != limits[1]
        || fold_count == 0
        || original.is_empty()
        || request.later.len() != original.len()
        || allocation.batches() != 1
        || allocation.rungs() != 8
        || request
            .policy
            .values()
            .max_fwer_p_value_ppm
            .min(request.policy.values().max_romano_wolf_p_value_ppm)
            != allocation.alpha_ppm()
    {
        return Err(
            "qualification requires a complete predeclared eight-rung research scope".into(),
        );
    }
    let original_digest = crate::boolean_qualification_plan::ordered_identities(
        original
            .iter()
            .map(super::super::super::CommittedBooleanFamilyV1::identity),
    );
    let later_digest = crate::boolean_qualification_plan::ordered_identities(
        request
            .later
            .iter()
            .map(CommittedBooleanOosV1::source_identity),
    );
    let keys: Vec<_> = request
        .training
        .statistics()
        .families()
        .iter()
        .map(|family| family.instrument())
        .collect();
    let declared_scope =
        runner::research_family::ResearchScopeV1::new(&keys, keys.len()).map_err(display)?;
    if original_digest != request.plan.expected_catalogs(request.rung)?
        || later_digest != request.plan.expected_sources(request.rung)?
        || declared_scope.digest() != request.plan.scope_digest()
    {
        return Err("qualification original/later sources differ from the predeclared rung".into());
    }
    let periods = request
        .later
        .first()
        .ok_or("later family missing")?
        .sessions();
    if periods.len() < 2 || count == 0 || count as u64 > request.bounds.candidates {
        return Err(
            "qualification requires the complete candidate and later-period population".into(),
        );
    }
    let observations = (count as u64)
        .checked_mul(periods.len() as u64)
        .ok_or("qualification observation overflow")?;
    if observations > request.bounds.observations
        || observations
            .checked_mul(8)
            .is_none_or(|n| n > request.bounds.memory)
    {
        return Err("qualification complete session matrix exceeds admission".into());
    }
    let mut links = Vec::new();
    links.try_reserve_exact(original.len()).map_err(display)?;
    for (first, later) in original.iter().zip(request.later) {
        if first.identity() != later.training_identity()
            || first.completion_digest() != later.training_completion_digest()
            || first.rows().len() != later.rows().len()
            || later.sessions() != periods
            || later.fold_projections().len() != later.rows().len()
        {
            return Err(
                "qualification later families are incomplete or not session aligned".into(),
            );
        }
        for ((a, b), proof) in first
            .rows()
            .iter()
            .zip(later.rows())
            .zip(later.fold_projections())
        {
            if a.program_index() != b.program_index()
                || a.side() != b.side()
                || a.ordinal() != b.ordinal()
            {
                return Err(
                    "qualification later coordinates differ from the original full catalog".into(),
                );
            }
            if a.selected_exit().is_some() != proof.is_some() {
                return Err(
                    "qualification selected coordinate is missing its declared later proof".into(),
                );
            }
        }
        links.push((
            later.identity(),
            later.completion_digest(),
            later.rows().len(),
        ));
    }
    let mut result = Manifest {
        identity: [0; 32],
        original: request.training.identity(),
        original_pin: request.training.completion_digest(),
        scope,
        unit,
        policy: request.policy,
        procedure: [
            request.procedure.draws(),
            request.procedure.seed(),
            request.procedure.block_length(),
        ],
        allocation,
        bounds: request.bounds,
        folds: fold_count,
        count,
        periods: periods.len(),
        family_digest: [0; 32],
        shared_digest: None,
        family_tests: [[0; 5]; 2],
        later: links,
        plan: request.plan.canonical_bytes()?,
    };
    if wire::required_bytes(&result)? > request.bounds.bytes {
        return Err("qualification complete output exceeds admission before statistics".into());
    }
    result.identity = identity(&result);
    Ok(result)
}

fn identity(manifest: &Manifest) -> [u8; 32] {
    let mut hash = Hasher::new();
    hash.update(b"brutex-boolean-fixed-training-qualification-v1\0");
    hash.update(&brutex_core::blake3::hash(&manifest.plan));
    for digest in [
        manifest.original,
        manifest.original_pin,
        manifest.scope,
        manifest.unit,
        manifest.policy.digest(),
        manifest.allocation.digest(),
    ] {
        hash.update(&digest);
    }
    for word in manifest
        .procedure
        .into_iter()
        .chain(manifest.bounds.words())
        .chain([
            manifest.folds as u64,
            manifest.count as u64,
            manifest.periods as u64,
            manifest.later.len() as u64,
        ])
    {
        hash.update(&word.to_le_bytes());
    }
    for (id, pin, count) in &manifest.later {
        hash.update(id);
        hash.update(pin);
        hash.update(&(*count as u64).to_le_bytes());
    }
    hash.finalize()
}

#[expect(
    clippy::too_many_lines,
    reason = "single complete-family numeric pass retains original, later, exact statistics and typed fold correspondence"
)]
fn measure(request: &Request<'_>, manifest: &mut Manifest) -> Result<Vec<Row>, String> {
    let mut returns = Vec::new();
    returns.try_reserve_exact(manifest.count).map_err(display)?;
    for source in request.later {
        for row in source.rows() {
            let mut values = Vec::new();
            values
                .try_reserve_exact(manifest.periods)
                .map_err(display)?;
            if row.periods().len() != manifest.periods {
                return Err("qualification coordinate periods incomplete".into());
            }
            for (period, day) in row.periods().iter().zip(source.sessions()) {
                if period.day() != *day {
                    return Err("qualification session order differs".into());
                }
                values.push(period.return_paisa());
            }
            returns.push(values);
        }
    }
    if returns.len() != manifest.count {
        return Err("qualification omitted a coordinate".into());
    }
    let [draws, seed, block] = manifest.procedure;
    let draws = usize::try_from(draws).map_err(display)?;
    let block = usize::try_from(block).map_err(display)?;
    let romano = zero::evaluate(
        &returns,
        draws,
        seed,
        block,
        zero::Bounds {
            max_work: request.bounds.work,
            max_bytes: request.bounds.memory,
        },
    )
    .map_err(|why| format!("qualification zero-conservative Romano-Wolf refused: {why:?}"))?;
    let white = white_reality_check_receipt_v1(&returns, draws, seed, block)
        .ok_or("qualification later White procedure refused")?;
    let spa = spa_receipt_v1(&returns, draws, seed, block)
        .ok_or("qualification later SPA procedure refused")?;
    manifest.family_digest = romano.family_digest();
    manifest.shared_digest = romano.shared_digest();
    manifest.family_tests = [
        [
            white.statistic_bits(),
            white.p_value_bits(),
            white.exact_p_value().numerator() as u64,
            white.exact_p_value().denominator() as u64,
            white.matched_or_exceeded() as u64,
        ],
        [
            spa.statistic_bits(),
            spa.p_value_bits(),
            spa.exact_p_value().numerator() as u64,
            spa.exact_p_value().denominator() as u64,
            spa.matched_or_exceeded() as u64,
        ],
    ];
    drop(returns);
    let mut rows = Vec::new();
    rows.try_reserve_exact(manifest.count).map_err(display)?;
    for (family, later) in request.later.iter().enumerate() {
        let original = request
            .training
            .statistics()
            .sources()
            .get(family)
            .ok_or("qualification original family missing")?;
        for (coordinate, (before, after)) in original.rows().iter().zip(later.rows()).enumerate() {
            let index = rows.len();
            let old = request
                .training
                .rows()
                .get(index)
                .ok_or("qualification original row missing")?;
            if before.identity() != old.identity() || old.source_index() != index {
                return Err("qualification original order differs".into());
            }
            let numeric = romano
                .candidate(index)
                .ok_or("qualification statistical coordinate missing")?;
            let facts = numeric_words(*numeric);
            let fold = later
                .fold_projections()
                .get(coordinate)
                .ok_or("qualification fold position missing")?;
            if before.selected_exit().is_some() != fold.is_some() {
                return Err("qualification fold authority availability differs from the original selected coordinate".into());
            }
            let fold = fold
                .as_ref()
                .map(|proof| {
                    if proof.training_run_id() != before.run_id()
                        || proof.later_run_id() != after.run_id()
                        || proof.ordinal() != before.ordinal()
                        || proof.folds().len() != manifest.folds
                        || before
                            .selected_exit()
                            .is_none_or(|selected| selected.digest() != proof.selected_digest())
                        || proof
                            .folds()
                            .iter()
                            .zip(request.plan.windows())
                            .any(|(fold, window)| fold.window != *window)
                        || proof.execution_refusal_bits() != after.execution_refusal_bits()
                        || proof.aggregate_oos_paisa()? != after.cell().pessimistic
                    {
                        return Err(
                            "qualification typed fold proof differs from original/later execution"
                                .into(),
                        );
                    }
                    proof.canonical_bytes(request.bounds.bytes)
                })
                .transpose()?;
            let projection = project(
                manifest,
                &old.values(),
                before,
                after,
                facts,
                fold.as_deref(),
            )?;
            rows.push(Row {
                original: before.identity(),
                later: after.identity(),
                family,
                coordinate,
                projection,
                romano: facts,
                fold,
            });
        }
    }
    current(request.training, request.later)?;
    Ok(rows)
}

fn numeric_words(candidate: zero::Candidate) -> [u64; 12] {
    let p = candidate.p_value();
    let mut result = [0; 12];
    result[0] = match candidate.classification() {
        Classification::MeasuredPositive => 1,
        Classification::ConservativeZero => 2,
        Classification::ConservativeNonpositive => 3,
    };
    result[1] = p.numerator();
    result[2] = p.denominator();
    if let Some(shared) = candidate.shared() {
        result[3] = 1;
        result[4..].copy_from_slice(&[
            shared.strategy() as u64,
            shared.stepdown_rank() as u64,
            shared.observed_statistic().to_bits(),
            shared.strict_exceedances() as u64,
            shared.initial_p_value().numerator() as u64,
            shared.initial_p_value().denominator() as u64,
            shared.adjusted_p_value().numerator() as u64,
            shared.adjusted_p_value().denominator() as u64,
        ]);
    }
    result
}

fn scaled(
    manifest: &Manifest,
    numerator: u64,
    denominator: u64,
) -> Result<runner::admission::AdmissionExactProbabilityV2, String> {
    let exact = manifest
        .allocation
        .scaled_probability(numerator, denominator)
        .map_err(|why| format!("qualification exact allocation refused: {why:?}"))?;
    probability(
        u64::try_from(exact.numerator()).map_err(display)?,
        u64::try_from(exact.denominator()).map_err(display)?,
    )
}

fn upper_ppm(exact: runner::admission::AdmissionExactProbabilityV2) -> Result<u64, String> {
    u64::try_from(
        (u128::from(exact.numerator()) * 1_000_000).div_ceil(u128::from(exact.denominator())),
    )
    .map_err(display)
}

fn project(
    manifest: &Manifest,
    old: &runner::admission::AdmissionEvidenceValuesV1,
    before: &BooleanCoordinateV1,
    later: &BooleanCoordinateV1,
    facts: [u64; 12],
    fold: Option<&[u8]>,
) -> Result<ResearchAdmissionProjectionV1, String> {
    let mut values = base_values(later)?;
    let cell = later.cell();
    values.wilson_win_rate_ppm = ObservedU64V1::Measured(
        wilson_ppm(
            cell.wins,
            cell.trades,
            wilson_lower_bits(cell.wins, cell.trades),
        )
        .ok_or("qualification Wilson projection refused")?,
    );
    // PBO describes the original training search. Later folds are separately
    // predeclared fixed-training checks; they never masquerade as retraining.
    values.pbo_ppm = old.pbo_ppm;
    values.pbo_contributing_folds = old.pbo_contributing_folds;
    values.pbo_unrankable_folds = old.pbo_unrankable_folds;
    let adjusted = scaled(manifest, facts[1], facts[2])?;
    values.fwer_p_value_ppm = ObservedU64V1::Measured(upper_ppm(adjusted)?);
    values.romano_wolf_p_value_ppm = ObservedU64V1::Measured(upper_ppm(adjusted)?);
    values.romano_wolf_decision = hypothesis_decision(adjusted);
    let white = scaled(
        manifest,
        manifest.family_tests[0][2],
        manifest.family_tests[0][3],
    )?;
    values.white_reality_p_value_ppm = ObservedU64V1::Measured(upper_ppm(white)?);
    values.white_reality_decision = hypothesis_decision(white);
    values.spa_p_value_ppm = ObservedU64V1::Measured(upper_ppm(scaled(
        manifest,
        manifest.family_tests[1][2],
        manifest.family_tests[1][3],
    )?)?);
    values.bootstrap_draws = ObservedU64V1::Measured(manifest.procedure[0]);
    values.bootstrap_strategies = ObservedU64V1::Measured(manifest.count as u64);
    values.bootstrap_periods = ObservedU64V1::Measured(manifest.periods as u64);
    values.full_precision_statistics_complete = CompletenessV1::Complete;
    if before.execution_refusal_bits().bits() != 0 {
        values.execution_complete = CompletenessV1::Refused;
    }
    if let Some(raw) = fold {
        let observed = runner::exit_grid_policy::expression_execution::later_period::validation::ObservedFixedTrainingFoldsV1::decode(raw, manifest.bounds.bytes, manifest.folds)?;
        values.decided_folds = ObservedU64V1::Measured(observed.decided_folds());
        values.profitable_oos_folds = ObservedU64V1::Measured(observed.profitable_oos_folds());
        values.oos_pessimistic_return_paisa =
            ObservedI64V1::Measured(observed.aggregate_oos_paisa()?);
    }
    manifest
        .policy
        .evaluate_research_projection(values)
        .map_err(|why| format!("qualification complete policy projection refused: {why:?}"))
}

#[cfg(test)]
#[path = "boolean_qualification_tests.rs"]
mod tests;
