//! Strict, source-bound NIFTY/BANKNIFTY single-stop research.
//!
//! This is an additive executor, not a selected exit-grid capability. Source
//! admission and indicator preparation are linear in history; the retained
//! prepared source is reused across expressions. Every program/direction starts
//! an audited attempt before evaluation. Exact observation bytes are durable
//! before those attempts can report completion.

use crate::audited_range_command::StrictConfig;
use crate::audited_stored::{RangeData, RangeGuard, RangeInputs, RangeRequest};
use crate::candidate_universe::boolean_candidate_v1::persistence;
use crate::sweep_evidence::{Attempt, Completion, Operation};
use brutex_core::blake3::{Hasher, hash};
use brutex_core::vendor::Vendor;
use indicators::column::Column;
use runner::expression::Expression;
use runner::identity::{DailyReferenceBinding, Direction, Params, ReferenceIntegrity};
use runner::signal_candle_stop::{Evaluation, Policy, Prepared, Source};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[path = "index_stop_source_context.rs"]
pub mod source_context;

/// Program/direction attempts begun, and later finished, per shared barrier.
/// A group shares its journal and reservation-directory barriers; each
/// attempt keeps its own reservation, lifecycle and identity barriers.
///
/// Measured 2026-09-12 on a generated 64-program catalog, release build, on a
/// drive shared with a live sweep: 27.0 barriers per attempt before grouping;
/// 10.0, 7.0, 6.5, 6.3 and 6.2 at groups of 1, 4, 8, 16 and 32. Sixteen keeps
/// all but 0.3 of the 21 barriers grouping can remove, while a refused group
/// leaves at most fifteen begun but unevaluated attempts, each recorded Refused.
const ATTEMPT_GROUP: usize = 16;

#[cfg(test)]
std::thread_local! {
    static ATTEMPT_GROUP_OVERRIDE: std::cell::Cell<Option<usize>> =
        const { std::cell::Cell::new(None) };
}

fn attempt_group() -> usize {
    #[cfg(test)]
    if let Some(group) = ATTEMPT_GROUP_OVERRIDE.with(std::cell::Cell::get) {
        return group.max(1);
    }
    ATTEMPT_GROUP
}

/// Exact physical capture limits; none changes the trading rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Complete canonical programs admitted in this one batch.
    pub programs: u64,
    /// Total signal events, trades and observed days across the full batch.
    pub records: u64,
    /// Complete serialized candidate body, including both directions.
    pub bytes: u64,
}

/// One exact original-first source span. Later measurement may select a suffix
/// of this span while retaining its causal indicator history.
#[derive(Clone, Copy)]
pub struct Request<'a> {
    /// Server-owned market store.
    pub store: &'a Path,
    /// Canonical source feed.
    pub vendor: &'a str,
    /// Exactly NSE-NIFTY or NSE-BANKNIFTY.
    pub underlying: &'a str,
    /// One supported intraday timeframe.
    pub rung: &'a str,
    /// Inclusive source starting month.
    pub from: (u16, u8),
    /// Inclusive source ending month.
    pub to: (u16, u8),
    /// Explicit checksum receipt and source resource admission.
    pub strict: &'a StrictConfig,
}

/// Strict source ownership, including the original causal signal column.
/// Possessing it does not imply any strategy passed institutional checks.
pub struct Loaded {
    // Reference annotations use the same store chosen by the native loader.
    // This path is not source/run identity and is never serialized as evidence.
    reference_store: PathBuf,
    data: RangeData,
    guard: Arc<RangeGuard>,
    column: Column,
    vendor: Vendor,
    rung: String,
    commit: String,
    source_binding: [u8; 32],
    first_day: i64,
    last_day: i64,
}

/// Native execution context tied to the exact admitted loader and limits.
/// Callers cannot substitute a source that merely has similarly shaped bars.
pub struct Context<'a> {
    native: Prepared<'a>,
    origin: &'a Loaded,
    source_binding: [u8; 32],
    limits: Limits,
}
impl Context<'_> {
    /// Exact native source/evaluator identity, with immutable loaded history.
    #[must_use]
    pub fn source_id(&self) -> [u8; 32] {
        self.native.source_id()
    }
}

impl Loaded {
    /// Load complete actual OHLCV with strict checksums and the shared calendar.
    /// No vendor fetch, generated price, nearest close or partial-month fallback.
    ///
    /// # Errors
    /// Invalid scope/build, incomplete or changed source/receipt/calendar,
    /// allocation/resource admission or causal evaluator refusal.
    pub fn load(request: Request<'_>) -> Result<Self, String> {
        let commit = crate::commit_stamp()
            .ok_or("single-stop research requires a verified clean build identity")?;
        Self::load_identified(request, commit)
    }

    fn load_identified(request: Request<'_>, commit: &str) -> Result<Self, String> {
        validate_scope(request.underlying, request.rung)?;
        let vendor = crate::parse_vendor(request.vendor)?;
        let underlying = request
            .underlying
            .strip_prefix("NSE-")
            .ok_or("single-stop canonical NSE instrument prefix is missing")?;
        let (data, guard) = RangeInputs::load(RangeRequest {
            store_root: request.store,
            vendor,
            underlying,
            rung: request.rung,
            from: request.from,
            to: request.to,
            receipt_root: request.strict.receipt_root(),
            max_bytes: request.strict.max_bytes(),
            max_records: request.strict.max_records(),
        })?
        .into_parts();
        let span = crate::population::RequestedSpanIdentityV1::new(
            request.from.0,
            request.from.1,
            request.to.0,
            request.to.1,
        )?;
        let (first, last) = crate::candidate_universe::requested_span_days(span)?;
        let first_day = first;
        let last_day = last;
        let mut binding = Hasher::new();
        binding.update(b"brutex-index-stop-strict-source-v1\0");
        binding.update(&Policy::V1.canonical_bytes());
        binding.update(&request.strict.max_bytes().to_le_bytes());
        binding.update(&request.strict.max_records().to_le_bytes());
        binding.update(&first_day.to_le_bytes());
        binding.update(&last_day.to_le_bytes());
        let source_binding = guard.bind_digest(binding.finalize());
        let preparation_id = hash(
            &[
                b"brutex-index-stop-preparation-v1\0".as_slice(),
                &source_binding,
                commit.as_bytes(),
                request.rung.as_bytes(),
            ]
            .concat(),
        );
        let attempt =
            crate::sweep_evidence::begin(request.store, preparation_id, Operation::Preparation)?;
        let column = prepare_column(&data, request.rung, span, first, last);
        let column = match column {
            Ok(column) => column,
            Err(why) => return Err(refuse(attempt, why)),
        };
        if let Err(why) = guard.require_current() {
            return Err(refuse(attempt, why));
        }
        attempt.finish(Completion::Completed)?;
        Ok(Self {
            reference_store: request.store.to_path_buf(),
            data,
            guard: Arc::new(guard),
            column,
            vendor,
            rung: request.rung.to_owned(),
            commit: commit.to_owned(),
            source_binding,
            first_day,
            last_day,
        })
    }

    /// Retained source and checksum generations, checked without decoding again.
    /// # Errors
    /// Changed or unavailable sources and receipt links refuse.
    pub fn require_current(&self) -> Result<(), String> {
        self.guard.require_current()
    }

    /// Inclusive requested source days, including expected-but-missing dates.
    #[must_use]
    pub const fn days(&self) -> (i64, i64) {
        (self.first_day, self.last_day)
    }

    /// Exact checksum, source-policy and requested-span binding.
    #[must_use]
    pub const fn source_binding(&self) -> [u8; 32] {
        self.source_binding
    }

    /// Construct one reusable native executor for this admitted source.
    ///
    /// # Errors
    /// Refuses changed sources, bad physical bounds, foreign source alignment or
    /// inconsistent reference/calendars. There is no exit-grid or horizon input.
    pub fn prepare(&self, limits: Limits) -> Result<Context<'_>, String> {
        validate_limits(limits)?;
        self.require_current()?;
        let execution = self
            .data
            .execution
            .as_ref()
            .map_or(self.data.signal.bars.as_slice(), |span| {
                span.bars.as_slice()
            });
        let prepared = Prepared::new(
            Policy::V1,
            Source {
                series: runner::exit_grid_policy::ExecutionSeriesV1::new(
                    &self.data.signal.key,
                    self.vendor.as_str(),
                    &self.commit,
                    crate::stored::calendar_policy_digest_v2(),
                    execution,
                )
                .map_err(display)?,
                signal_bars: &self.data.signal.bars,
                signal_column: &self.column,
                minute_context: &self.data.exact_minute.bars,
                daily_reference: self.reference(),
                timeframe: &self.rung,
                first_day: self.first_day,
                last_day: self.last_day,
                params: Params {
                    min_hits: 0,
                    ceiling: limits.records,
                    pair_budget: limits.programs,
                    policy: 1,
                },
            },
        )
        .map_err(display)?;
        self.require_current()?;
        Ok(Context {
            native: prepared,
            origin: self,
            source_binding: self.source_binding,
            limits,
        })
    }

    fn reference(&self) -> DailyReferenceBinding<'_> {
        DailyReferenceBinding {
            daily_bars: &self.data.daily.bars,
            eligibility: &self.data.daily.eligibility,
            schema: crate::stored::DAILY_REFERENCE_SCHEMA,
            eligibility_policy: crate::stored::DAILY_ELIGIBILITY_POLICY,
            gap_overlay_policy: crate::stored::EXACT_MINUTE_GAP_POLICY,
            excluded_ist_days: &indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS,
            daily_integrity: ReferenceIntegrity::ChecksumReceiptV1(self.source_binding),
            minute_integrity: ReferenceIntegrity::ChecksumReceiptV1(self.source_binding),
            swept_series_calendar_policy: crate::stored::SWEPT_SERIES_CALENDAR_POLICY,
        }
    }
}

/// Durable complete catalog, retaining native execution authority for the
/// statistical adapter and a separately decoded, read-only receipt for pages.
pub struct Committed {
    guard: Arc<RangeGuard>,
    reader: crate::index_stop_store::Reader,
    evaluations: Vec<Evaluation>,
    publication: crate::index_stop_vix::Publication,
}
impl Committed {
    /// Exact immutable catalog identity.
    #[must_use]
    pub fn identity(&self) -> [u8; 32] {
        self.reader.identity()
    }
    /// Exact completed observation bytes pin.
    #[must_use]
    pub fn completion_digest(&self) -> [u8; 32] {
        self.reader.completion_digest()
    }
    /// Sealed executions in canonical program then long/short order.
    #[must_use]
    pub fn evaluations(&self) -> &[Evaluation] {
        &self.evaluations
    }
    /// Retained source and full saved-publication generations remain current.
    /// # Errors
    /// Any source, checksum, candidate or VIX publication receipt change refuses.
    pub fn require_current(&self) -> Result<(), String> {
        self.with_publication_current(|| Ok(()))
    }
    fn with_publication_current<T>(
        &self,
        project: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        self.publication.with_current(&self.reader, || {
            self.guard.require_current()?;
            let result = project()?;
            self.guard.require_current()?;
            Ok(result)
        })
    }
}

/// Persist both directions and both fill readings for the complete supplied
/// catalog and fixed measurement days. This does not claim grammar exhaustion.
///
/// # Errors
/// Duplicate/empty programs, changed source, invalid calendar window, exact
/// arithmetic, capture admission or any persistence/terminal error refuses.
pub fn produce_catalog(
    root: &Path,
    loaded: &Loaded,
    context: &Context<'_>,
    programs: &[Expression],
    days: (i64, i64),
    limits: Limits,
) -> Result<Committed, String> {
    produce_catalog_inner(root, loaded, context, programs, days, limits, None)
}

pub(crate) fn produce_catalog_with_context(
    root: &Path,
    loaded: &Loaded,
    context: &Context<'_>,
    programs: &[Expression],
    days: (i64, i64),
    limits: Limits,
    original: &source_context::Committed,
) -> Result<Committed, String> {
    original.require_current()?;
    produce_catalog_inner(
        root,
        loaded,
        context,
        programs,
        days,
        limits,
        Some(original),
    )
}

fn produce_catalog_inner(
    root: &Path,
    loaded: &Loaded,
    context: &Context<'_>,
    programs: &[Expression],
    days: (i64, i64),
    limits: Limits,
    original: Option<&source_context::Committed>,
) -> Result<Committed, String> {
    validate_catalog(programs, limits)?;
    validate_catalog_context(loaded, context, days, limits)?;
    let prepared = &context.native;
    let (legacy, run_ids) = catalog_identity(loaded, prepared, programs, days, limits)?;
    let identity = original.map_or(legacy, |value| {
        source_context::contextual_catalog(legacy, value.link())
    });
    let catalog = crate::sweep_evidence::begin(root, identity, Operation::IndexStop)?;
    let mut attempts = Vec::new();
    let result: Result<Committed, String> = (|| {
        let evaluations = evaluate_grouped(
            root,
            prepared,
            programs,
            &run_ids,
            days,
            limits,
            &mut attempts,
        )?;
        if let Some(original) = original {
            original
                .require_catalog_bytes(crate::index_stop_store::encoded_bytes(&evaluations)?)?;
        }
        let body = crate::index_stop_store::encode(identity, &evaluations, limits.bytes)?;
        let bytes = u64::try_from(body.len()).map_err(display)?;
        let payload = hash(&body);
        let pending =
            persistence::prepare_in_namespace(root, "index-stop-candidates-v1", identity, &body)?;
        loaded.require_current()?;
        pending.verify_body(payload, bytes)?;
        pending.finish(identity, payload, bytes)?;
        drop(pending);
        let reader =
            crate::index_stop_store::Reader::open(root, identity, limits.bytes, limits.records)?;
        loaded.require_current()?;
        reader.require_current()?;
        if let Some(original) = original {
            // The exact source relation becomes durable before either the
            // native run attempts or the whole catalog reports completion.
            source_context::bind_catalog(root, &reader, original)?;
        }
        let vix = publish_vix(root, loaded, &reader, limits)?;
        // Annotation failures retain existing candidate/source bytes but cannot
        // report a complete publication. Typed unavailable reference months are
        // saved visibly and do not alter any native result or qualification.
        vix.with_current(|_| {
            // A refusal drops the undrained rest, each with its Refused terminal.
            let mut pending = attempts.drain(..);
            loop {
                let group: Vec<Attempt> = pending.by_ref().take(attempt_group()).collect();
                if group.is_empty() {
                    return Ok(());
                }
                crate::sweep_evidence::finish_many(group, Completion::Completed)?;
            }
        })?;
        Ok(Committed {
            guard: Arc::clone(&loaded.guard),
            reader,
            evaluations,
            publication: vix.into_publication(),
        })
    })();
    match result {
        Ok(committed) => finish_catalog(committed, catalog),
        Err(mut why) => {
            for attempt in attempts {
                if let Err(terminal) = attempt.finish(Completion::Refused) {
                    let _ = write!(why, "; single-stop terminal also refused: {terminal}");
                }
            }
            Err(refuse(catalog, why))
        }
    }
}

/// Evaluate every program and direction in canonical order. A whole group of
/// attempts is durably begun before any of its evaluations; when a start is
/// refused, the durable prefix still evaluates first, so the refusal returned
/// is the one sequential starts would have met first.
fn evaluate_grouped(
    root: &Path,
    prepared: &Prepared<'_>,
    programs: &[Expression],
    run_ids: &[[u8; 32]],
    days: (i64, i64),
    limits: Limits,
    attempts: &mut Vec<Attempt>,
) -> Result<Vec<Evaluation>, String> {
    let count = programs
        .len()
        .checked_mul(2)
        .ok_or("single-stop direction count overflow")?;
    let mut evaluations = Vec::new();
    evaluations.try_reserve_exact(count).map_err(display)?;
    attempts.try_reserve_exact(count).map_err(display)?;
    let mut records = 0_u64;
    let mut jobs = programs
        .iter()
        .flat_map(|program| [Direction::Long, Direction::Short].map(|side| (program, side)));
    for identities in run_ids.chunks(attempt_group()) {
        let before = attempts.len();
        let started =
            crate::sweep_evidence::begin_many(root, identities, Operation::IndexStop, attempts);
        let admitted = attempts.len() - before;
        for (&run_id, (program, side)) in identities.iter().zip(jobs.by_ref()).take(admitted) {
            let remaining = limits
                .records
                .checked_sub(records)
                .ok_or("single-stop evidence count overflow")?;
            let evaluation = prepared
                .evaluate_days(program, side, remaining, days.0, days.1)
                .map_err(display)?;
            if evaluation.run_id() != run_id {
                return Err("single-stop execution identity differs from its durable start".into());
            }
            let added = evaluation
                .events()
                .len()
                .checked_add(evaluation.trades().len())
                .and_then(|value| value.checked_add(evaluation.periods().len()))
                .and_then(|value| u64::try_from(value).ok())
                .ok_or("single-stop complete row count overflow")?;
            records = records
                .checked_add(added)
                .filter(|value| *value <= limits.records)
                .ok_or("single-stop complete evidence exceeds aggregate record admission")?;
            evaluations.push(evaluation);
        }
        started?;
    }
    Ok(evaluations)
}

fn validate_catalog_context(
    loaded: &Loaded,
    context: &Context<'_>,
    days: (i64, i64),
    limits: Limits,
) -> Result<(), String> {
    if !std::ptr::eq(context.origin, loaded)
        || context.source_binding != loaded.source_binding
        || context.limits != limits
    {
        return Err(
            "single-stop native context differs from the admitted source or capture limits".into(),
        );
    }
    if days.0 > days.1 || days.0 < loaded.first_day || days.1 > loaded.last_day {
        return Err(
            "single-stop measurement window is outside admitted original-first history".into(),
        );
    }
    loaded.require_current()
}

fn finish_catalog(committed: Committed, catalog: Attempt) -> Result<Committed, String> {
    // A missing or changed reference publication refuses before the outer
    // terminal write. Both owners remain excluded throughout that write.
    committed.with_publication_current(|| catalog.finish(Completion::Completed))?;
    Ok(committed)
}

fn publish_vix(
    root: &Path,
    loaded: &Loaded,
    reader: &crate::index_stop_store::Reader,
    limits: Limits,
) -> Result<crate::index_stop_vix::Reader, String> {
    let observation_bytes = source_context::observation_budget()?;
    crate::index_stop_vix::publish(
        root,
        &loaded.reference_store,
        loaded.vendor,
        reader,
        crate::index_stop_vix::Bounds {
            bytes: observation_bytes,
            records: limits.records.min(observation_bytes / 96),
            memory_bytes: observation_bytes,
            page_records: 256,
        },
    )
}

/// The catalog identity and the exact run identities it binds, in canonical
/// program then long/short order, computed once for both uses.
fn catalog_identity(
    loaded: &Loaded,
    prepared: &Prepared<'_>,
    programs: &[Expression],
    days: (i64, i64),
    limits: Limits,
) -> Result<([u8; 32], Vec<[u8; 32]>), String> {
    let runs = run_ids(prepared, programs, days)?;
    let identity = catalog_identity_of(loaded.source_binding, &runs, days, limits);
    Ok((identity, runs))
}

fn catalog_identity_for(
    source_binding: [u8; 32],
    prepared: &Prepared<'_>,
    programs: &[Expression],
    days: (i64, i64),
    limits: Limits,
) -> Result<[u8; 32], String> {
    let runs = run_ids(prepared, programs, days)?;
    Ok(catalog_identity_of(source_binding, &runs, days, limits))
}

fn run_ids(
    prepared: &Prepared<'_>,
    programs: &[Expression],
    days: (i64, i64),
) -> Result<Vec<[u8; 32]>, String> {
    let mut runs = Vec::with_capacity(programs.len().saturating_mul(2));
    for program in programs {
        for side in [Direction::Long, Direction::Short] {
            runs.push(
                prepared
                    .run_id_days(program, side, days.0, days.1)
                    .map_err(display)?,
            );
        }
    }
    Ok(runs)
}

fn catalog_identity_of(
    source_binding: [u8; 32],
    runs: &[[u8; 32]],
    days: (i64, i64),
    limits: Limits,
) -> [u8; 32] {
    let mut identity = Hasher::new();
    identity.update(b"brutex-index-stop-catalog-v1\0");
    identity.update(&source_binding);
    identity.update(&Policy::V1.canonical_bytes());
    for limit in [limits.programs, limits.records, limits.bytes] {
        identity.update(&limit.to_le_bytes());
    }
    for day in [days.0, days.1] {
        identity.update(&day.to_le_bytes());
    }
    for run in runs {
        identity.update(run);
    }
    identity.finalize()
}

fn validate_limits(limits: Limits) -> Result<(), String> {
    if [limits.programs, limits.records, limits.bytes].contains(&0)
        || limits
            .programs
            .checked_mul(2)
            .is_none_or(|value| value > limits.records)
    {
        return Err(
            "single-stop physical limits must admit complete programs and both directions".into(),
        );
    }
    Ok(())
}
fn validate_catalog(programs: &[Expression], limits: Limits) -> Result<(), String> {
    validate_limits(limits)?;
    if programs.is_empty() || u64::try_from(programs.len()).map_err(display)? > limits.programs {
        return Err("single-stop catalog is empty or exceeds declared program admission".into());
    }
    let mut seen = std::collections::HashSet::new();
    seen.try_reserve(programs.len()).map_err(display)?;
    for program in programs {
        if !seen.insert(program.encode()) {
            return Err("single-stop catalog contains a duplicate canonical program".into());
        }
    }
    Ok(())
}
fn validate_scope(underlying: &str, rung: &str) -> Result<(), String> {
    if !matches!(underlying, "NSE-NIFTY" | "NSE-BANKNIFTY")
        || !crate::ledger_all::LEDGER_RUNGS.contains(&rung)
    {
        return Err(
            "single-stop research is only NSE-NIFTY/NSE-BANKNIFTY on the eight intraday timeframes"
                .into(),
        );
    }
    Ok(())
}
fn prepare_column(
    data: &RangeData,
    rung: &str,
    span: crate::population::RequestedSpanIdentityV1,
    first: i64,
    last: i64,
) -> Result<Column, String> {
    let execution = data
        .execution
        .as_ref()
        .map_or(data.signal.bars.as_slice(), |value| value.bars.as_slice());
    let seconds =
        u32::try_from(crate::stored::rung_length_micros(rung)? / 1_000_000).map_err(display)?;
    let signal_calendar =
        crate::stored::calendar_receipt_v2_for_bars(&data.signal.bars, seconds, first, last)?
            .require_complete()?;
    let execution_calendar =
        crate::stored::calendar_receipt_v2_for_bars(execution, 60, first, last)?
            .require_complete()?;
    let coverage = crate::candidate_universe::CandidateCalendarCoverageV1::from_complete(
        seconds,
        span,
        signal_calendar,
        execution_calendar,
    )?;
    crate::candidate_universe::require_exact_calendar(
        "single-stop signal",
        &data.signal.bars,
        seconds,
        coverage,
        signal_calendar,
    )?;
    crate::candidate_universe::require_exact_calendar(
        "single-stop execution",
        execution,
        60,
        coverage,
        execution_calendar,
    )?;
    let (column, _) = crate::candidate_universe::build_candidate_columns(
        &data.signal.bars,
        &data.daily.references,
        &data.exact_minute.bars,
        execution,
        seconds,
        &crate::candidate_universe::CandidateEvaluationInputsV1 {
            widths: indicators::evaluator::Widths::pinned()
                .map_err(|why| format!("single-stop evaluator widths: {why:?}"))?,
            availability: crate::stored::vwap_availability(&data.signal.key),
            thresholds: indicators::pattern::Thresholds::CLASSICAL,
        },
    )?;
    Ok(column)
}
fn refuse(attempt: Attempt, why: String) -> String {
    match attempt.finish(Completion::Refused) {
        Ok(()) => why,
        Err(terminal) => format!("{why}; single-stop terminal also refused: {terminal}"),
    }
}
fn display(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
#[path = "index_stop_tests.rs"]
pub(crate) mod tests;
