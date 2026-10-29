//! Declared, resumable single-stop Boolean search over one index and selected
//! intraday timeframes. A complete batch contains every program and both sides;
//! a bounded invocation is never described as an exhausted grammar.
use crate::boolean_campaign::RungScope;
use crate::boolean_grammar_batch::{Batch, Budget};
use crate::index_stop::{Context, Limits, Loaded};
use crate::index_stop_qualification as qualification;
use crate::population_statistics_v2::PopulationStatisticsProcedureV2;
use crate::search_checkpoint::Journal;
use brutex_core::blake3::hash;
use runner::admission::AdmissionPolicyV1;
use std::fmt::Write as _;
use std::path::Path;
use vocab::expression_search::Cursor;

#[path = "index_stop_search_checkpoint.rs"]
mod checkpoint;
use checkpoint::{Frame, Link};
const NAMESPACE: &str = "index-stop-search-v1";

#[path = "index_stop_search_reader.rs"]
pub mod reader;

#[path = "index_stop_launch.rs"]
mod launch;
pub use launch::{COMMAND, Launch, LaunchInput, configuration, prepare};

#[path = "index_stop_search_progress.rs"]
mod live;
pub use live::{Observation, RungStage};

/// Resolved, immutable statistical and physical policy for one declaration.
pub struct Configuration {
    /// Complete source integrity and summed source-record admission.
    pub strict: crate::audited_range_command::StrictConfig,
    /// Existing institutional acceptance criteria; no stop-rule substitution.
    pub policy: AdmissionPolicyV1,
    /// Explicit resampling procedure, fixed before any results are inspected.
    pub procedure: PopulationStatisticsProcedureV2,
    /// Complete per-timeframe candidate body and output-row admission.
    pub capture: Limits,
    /// Full institutional computation admission per timeframe.
    pub qualification: qualification::Bounds,
    /// Maximum concurrently evaluated timeframes, capped by actual CPU parallelism.
    pub workers: usize,
    /// Aggregate acknowledged checkpoint read admission, independent of search identity.
    pub history_bytes: u64,
    /// Independent current limit used by the original-source browser reader.
    /// Source preflight and actual catalog publication both recheck this bound.
    pub source_observation_bytes: u64,
    /// Current cold grammar and numerical replay work admission, independent of search identity.
    pub replay_nodes: u64,
}

/// Exact immutable research declaration. One index keeps its per-index daily
/// requirement distinct; a second index is a separately named research search.
pub struct Request<'a> {
    /// Server-owned source and evidence store.
    pub root: &'a Path,
    /// Exact stored source feed.
    pub feed: &'a str,
    /// Exactly NSE-NIFTY or NSE-BANKNIFTY.
    pub index: &'a str,
    /// Inclusive original training months.
    pub training: ((u16, u8), (u16, u8)),
    /// Inclusive wholly later evaluation months; no outcomes choose this split.
    pub later: ((u16, u8), (u16, u8)),
    /// Canonical selected physical timeframe slots.
    pub rungs: RungScope,
    /// Exact declared live expression alphabet; full alphabet is not narrowed.
    pub alphabet: &'a [u32],
    /// Fixed programs per immutable batch, across all selected timeframes.
    pub batch_programs: u64,
    /// Fixed syntax-node work allowance per batch; pauses preserve the cursor.
    pub node_allowance: u64,
    /// Batches allowed in this invocation, not a grammar-depth or completion rule.
    pub batch_allowance: u64,
    /// Complete predeclared policy and resource configuration.
    pub configuration: &'a Configuration,
}

/// Saved evidence returned by the exact current operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    /// Immutable source/policy/search declaration identity.
    pub identity: [u8; 32],
    /// Acknowledged complete batches; running children do not increase this.
    pub completed_batches: u64,
    /// Exact grammar exhaustion, never inferred from inactivity.
    pub exhausted: bool,
    /// Per-rung qualification links for the latest complete batch.
    pub qualifications: [Option<([u8; 32], [u8; 32])>; 8],
    /// Complete canonical programs in acknowledged batches, excluding active work.
    pub completed_programs: u64,
    /// Syntax-node work recorded by acknowledged complete batches.
    pub completed_work: u64,
    /// Zero-based reserved batch currently running, absent between batches.
    pub current_batch: Option<u64>,
    /// Actual last boundary per selected physical timeframe, not a percentage.
    pub rung_stages: [Option<RungStage>; 8],
    /// Most recent complete nonempty batch verified in the acknowledged ancestry.
    /// Retained while the current reservation is pending, including after restart.
    pub latest_saved: Option<CompletedBatch>,
}

/// Pinned complete-batch navigation, separate from current worker stage snapshots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletedBatch {
    /// Zero-based acknowledged batch ordinal.
    pub batch: u64,
    /// Exactly the selected timeframe qualifications for this whole batch.
    pub qualifications: [Option<([u8; 32], [u8; 32])>; 8],
}

struct Sources {
    rung: usize,
    training: Loaded,
    later: Loaded,
}
struct PreparedSources<'a> {
    sources: &'a Sources,
    training: Context<'a>,
    later: Context<'a>,
    training_context: crate::index_stop::source_context::Committed,
    later_context: crate::index_stop::source_context::Committed,
}

/// Execute checked real OHLCV, with parallel independent timeframes, exact
/// stopped/resumed grammar state and durable candidate/qualification links.
///
/// # Errors
/// Invalid declaration, dirty build, absent/changed sources, unrepresentable
/// budgets, execution/statistical admission or any durable observation failure.
pub fn execute(
    request: &Request<'_>,
    observe: &mut dyn FnMut(Progress) -> Result<(), String>,
) -> Result<String, String> {
    crate::commit_stamp().ok_or("single-stop search requires a clean build identity")?;
    execute_with(request, observe, Loaded::load)
}

/// Execute with source preparation and exact per-timeframe boundary observations.
/// No search identity is claimed until its source-bound declaration exists.
///
/// # Errors
/// Every native execution refusal, or an observer failure that stops further stages.
pub fn execute_observed(
    request: &Request<'_>,
    observe: &mut dyn FnMut(Observation) -> Result<(), String>,
) -> Result<String, String> {
    crate::commit_stamp().ok_or("single-stop search requires a clean build identity")?;
    execute_observed_with(request, observe, Loaded::load)
}

fn execute_with(
    request: &Request<'_>,
    observe: &mut dyn FnMut(Progress) -> Result<(), String>,
    load: fn(crate::index_stop::Request<'_>) -> Result<Loaded, String>,
) -> Result<String, String> {
    execute_observed_with(
        request,
        &mut |next| match next {
            Observation::Search(progress) => observe(progress),
            Observation::Preparing { .. } => Ok(()),
        },
        load,
    )
}

fn execute_observed_with(
    request: &Request<'_>,
    observe: &mut dyn FnMut(Observation) -> Result<(), String>,
    load: fn(crate::index_stop::Request<'_>) -> Result<Loaded, String>,
) -> Result<String, String> {
    validate(request)?;
    let sources = load_sources(request, load, observe)?;
    let prepared = prepare_sources(request, &sources, observe)?;
    let declaration = declaration(request, &prepared)?;
    let identity = hash(&declaration);
    let bytes = request.configuration.capture.bytes;
    let observation = checkpoint::ReadBudget {
        bytes: request.configuration.history_bytes,
        records: request.configuration.capture.records,
        nodes: request.configuration.replay_nodes,
    };
    let budget = Budget {
        programs: request.batch_programs,
        nodes: request.node_allowance,
        bytes: bytes / 4,
    };
    let (mut journal, mut frame) = open_checkpoint(request, &prepared, &declaration, observation)?;
    observe(Observation::Search(progress(
        identity,
        &frame,
        request.rungs,
    )))?;
    // ONE POOL OF EVERY CORE, AND `lanes` IS A JOB COUNT, NOT A THREAD COUNT.
    //
    // This pool used to be `lanes` threads wide and ran `prepared.par_iter()`,
    // so when every lane was busy with a timeframe the statistics nested inside
    // it had no thread to spread onto, and the cores above `lanes` sat idle.
    // `live::parallel` now runs exactly `lanes` long-lived jobs on a pool of
    // `cores` threads: the `workers` cap still bounds how many timeframes are
    // in flight -- each needs gigabytes -- while nested parallel work reaches
    // every core.
    let cores = std::thread::available_parallelism().map_err(display)?.get();
    let lanes = request.configuration.workers.min(prepared.len()).min(cores);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(cores)
        .thread_name(|index| format!("index-stop-{index}"))
        .build()
        .map_err(display)?;
    let mut report = String::from(crate::STORED_PROVENANCE);
    let _ = writeln!(
        report,
        "\nSingle-stop search {}: {} on {:?}; {} parallel timeframe workers. Both long/short and both printed fill readings; criteria use pessimistic gross results. Exit is the completed signal-candle stop or 15:10 IST, with no target/trailing/horizon grid.\n",
        crate::identity_hex(&identity),
        request.index,
        request.rungs.labels(),
        lanes
    );
    for _ in 0..request.batch_allowance {
        if frame.exhausted {
            break;
        }
        let pending = if frame.pending {
            frame
        } else {
            let batch = Batch::prepare(frame.cursor.clone(), frame.work, frame.programs, budget)?;
            let mut next = Frame::pending(&frame, batch)?;
            next.publish(&mut journal, bytes, observation, request.node_allowance)?;
            next
        };
        let mut current = progress(identity, &pending, request.rungs);
        observe(Observation::Search(current.clone()))?;
        let batch = pending
            .batch
            .as_ref()
            .ok_or("single-stop pending checkpoint lacks its declared programs")?;
        let results = if batch.programs().is_empty() {
            Vec::new()
        } else {
            live::parallel(
                request,
                &prepared,
                &pool,
                lanes,
                batch.programs(),
                &mut current,
                observe,
            )?
        };
        let links = completed_links(&prepared, results)?;
        for source in &sources {
            source.training.require_current()?;
            source.later.require_current()?;
        }
        frame = Frame::done(pending, &links)?;
        frame.publish(&mut journal, bytes, observation, request.node_allowance)?;
        observe(Observation::Search(progress(
            identity,
            &frame,
            request.rungs,
        )))?;
        let _ = writeln!(
            report,
            "Batch {} saved. {} cumulative programs; grammar exhausted: {}.",
            frame.completed_batches, frame.programs, frame.exhausted
        );
    }
    let _ = writeln!(
        report,
        "\nInvocation finished with {} acknowledged batches. Search exhausted: {}. Each program has exactly two direction settings per selected timeframe, each with pessimistic and optimistic fills. Passing these research checks is not a future-profit guarantee.\n",
        frame.completed_batches, frame.exhausted
    );
    Ok(report)
}

fn completed_links(
    prepared: &[PreparedSources<'_>],
    results: Vec<Result<Link, String>>,
) -> Result<[Option<Link>; 8], String> {
    let mut links = [None; 8];
    let mut failures = Vec::new();
    for (source, result) in prepared.iter().zip(results) {
        match result {
            Ok(link) => {
                *links
                    .get_mut(source.sources.rung)
                    .ok_or("single-stop physical rung index")? = Some(link);
            }
            Err(why) => failures.push(format!(
                "{}: {why}",
                crate::ledger_all::LEDGER_RUNGS
                    .get(source.sources.rung)
                    .ok_or("single-stop rung label")?
            )),
        }
    }
    if !failures.is_empty() {
        return Err(format!(
            "Single-stop batch remains pending; saved successful children are retained and exact retry reuses them. {}",
            failures.join("; ")
        ));
    }
    Ok(links)
}

fn prepare_sources<'a>(
    request: &Request<'_>,
    sources: &'a [Sources],
    observe: &mut dyn FnMut(Observation) -> Result<(), String>,
) -> Result<Vec<PreparedSources<'a>>, String> {
    let context_limit = admit_source_contexts(request, sources)?;
    sources
        .iter()
        .map(|source| {
            live::preparing(observe, source.rung, RungStage::Preparing)?;
            let result = (|| {
                let training = source.training.prepare(request.configuration.capture)?;
                let later = source.later.prepare(request.configuration.capture)?;
                let training_context = crate::index_stop::source_context::publish(
                    request.root,
                    &source.training,
                    &training,
                    context_limit,
                    request.configuration.source_observation_bytes,
                )?;
                let later_context = crate::index_stop::source_context::publish(
                    request.root,
                    &source.later,
                    &later,
                    context_limit,
                    request.configuration.source_observation_bytes,
                )?;
                Ok(PreparedSources {
                    sources: source,
                    training,
                    later,
                    training_context,
                    later_context,
                })
            })();
            if result.is_err() {
                live::preparing(observe, source.rung, RungStage::Refused)?;
            }
            result
        })
        .collect()
}

fn admit_source_contexts(request: &Request<'_>, sources: &[Sources]) -> Result<u64, String> {
    let mut total = 0_u64;
    let mut largest = 0_u64;
    for source in sources {
        for loaded in [&source.training, &source.later] {
            crate::index_stop::source_context::require_minimum_observation(
                loaded,
                request.batch_programs,
                request.configuration.source_observation_bytes,
            )?;
            let bytes = crate::index_stop::source_context::encoded_bytes(loaded)?;
            total = total
                .checked_add(bytes)
                .ok_or("source snapshot aggregate bytes overflow")?;
            largest = largest.max(bytes);
        }
    }
    // Every selected batch can publish two fixed context relations per rung.
    // This invocation allowance is physical admission, not a grammar limit.
    let links = (sources.len() as u64)
        .checked_mul(2)
        .and_then(|count| count.checked_mul(request.batch_allowance))
        .and_then(|count| count.checked_mul(crate::index_stop::source_context::CATALOG_LINK_BYTES))
        .ok_or("source relation aggregate bytes overflow")?;
    total = total
        .checked_add(links)
        .ok_or("source snapshot aggregate bytes overflow")?;
    if total > request.configuration.history_bytes {
        return Err("complete selected source snapshots and catalog relations exceed history byte admission before publication".into());
    }
    // Serialization and receipt re-read overlap; both complete buffers must fit
    // before publishing any source. Archived bodies are then released.
    if largest
        .checked_mul(2)
        .is_none_or(|bytes| bytes > request.configuration.qualification.memory_bytes)
    {
        return Err("source serialization and verification buffers exceed memory admission before publication".into());
    }
    Ok(largest)
}

fn open_checkpoint(
    request: &Request<'_>,
    prepared: &[PreparedSources<'_>],
    declaration: &[u8],
    observation: checkpoint::ReadBudget,
) -> Result<(Journal, Frame), String> {
    let initial = Cursor::new(request.alphabet).map_err(debug)?;
    let verification = checkpoint::Verification::new(request, prepared)?;
    let mut journal = Journal::open(request.root, NAMESPACE, hash(declaration))?;
    let current = checkpoint::recover(
        &journal,
        declaration,
        request.root,
        request.configuration.history_bytes,
        request.configuration.capture.records,
        request.configuration.replay_nodes,
        request.rungs.mask(),
        &initial,
        request.batch_programs,
        request.node_allowance,
        &verification,
    )?;
    let frame = if let Some(frame) = current {
        frame
    } else {
        let mut first = Frame::declaration(declaration.to_vec(), initial, request.rungs.mask());
        first.publish(
            &mut journal,
            request.configuration.capture.bytes,
            observation,
            request.node_allowance,
        )?;
        first
    };
    Ok((journal, frame))
}

fn run_rung(
    request: &Request<'_>,
    source: &PreparedSources<'_>,
    search: [u8; 32],
    batch: u64,
    programs: &[runner::expression::Expression],
    boundary: &live::Boundary<'_>,
) -> Result<Link, String> {
    let config = request.configuration;
    let allocation = pricing_allocation(config, batch, source.sources.rung)?;
    boundary.send(RungStage::Training)?;
    let training = crate::index_stop::produce_catalog_with_context(
        request.root,
        &source.sources.training,
        &source.training,
        programs,
        source.sources.training.days(),
        config.capture,
        &source.training_context,
    )?;
    let later_days = month_days(request.later)?;
    boundary.send(RungStage::Later)?;
    let later = crate::index_stop::produce_catalog_with_context(
        request.root,
        &source.sources.later,
        &source.later,
        programs,
        later_days,
        config.capture,
        &source.later_context,
    )?;
    boundary.send(RungStage::Institutional)?;
    let measured = qualification::produce(qualification::Request {
        root: request.root,
        search_identity: search,
        training: &training,
        later: &later,
        policy: &config.policy,
        procedure: config.procedure,
        allocation,
        bounds: config.qualification,
    })?;
    measured.require_current()?;
    Ok(Link {
        identity: measured.identity(),
        pin: measured.completion_digest(),
    })
}

fn pricing_allocation(
    config: &Configuration,
    batch: u64,
    rung: usize,
) -> Result<runner::search_allocation_v1::Allocation, String> {
    allocation_for(&config.policy, config.procedure, batch, rung)
}

fn allocation_for(
    policy: &AdmissionPolicyV1,
    procedure: PopulationStatisticsProcedureV2,
    batch: u64,
    rung: usize,
) -> Result<runner::search_allocation_v1::Allocation, String> {
    let values = policy.values();
    let alpha = values
        .max_fwer_p_value_ppm
        .min(values.max_spa_p_value_ppm)
        .min(values.max_white_reality_p_value_ppm)
        .min(values.max_romano_wolf_p_value_ppm);
    let allocation =
        runner::search_allocation_v1::allocate(batch, u64::try_from(rung).map_err(display)?, alpha)
            .map_err(debug)?;
    let draws = procedure.draws();
    allocation.require_draws(draws).map_err(|why| {
        let label=crate::ledger_all::LEDGER_RUNGS.get(rung).copied().unwrap_or("unavailable");
        let minimum=allocation.minimum_draws().map_or_else(||"unattainable at zero alpha".into(),|count|count.to_string());
        format!("Single-stop batch {batch}, timeframe {label}: configured bootstrap draws {draws}, required minimum {minimum}; no candidate pricing started ({why:?})")
    })?;
    Ok(allocation)
}

/// Refuse an unaffordable `batch_programs` BEFORE any source is read.
///
/// # The check this replaces could never be true
///
/// Admission read `batch_programs > config.capture.programs`, and
/// `capture.programs` is ASSIGNED from `batch_programs` -- so the guard was
/// `x > x`. The real bound lives in `boolean_grammar_batch::Batch::prepare`,
/// which refuses unless `batch_programs * ENCODED_LEN + HEADER <= bytes / 4`.
///
/// Because the vacuous guard looked like the bound, nothing enforced the real
/// one at admission: the launch page reported ready, the POST returned 202, and
/// the worker then loaded every selected timeframe's training AND later sources
/// and published their context archives before refusing. Every retry repeated
/// the whole load, and the message named neither the setting that was too large
/// nor the one that bounds it -- nor that the effective ceiling is a QUARTER of
/// the named variable, so an operator who raised it to the exact figure in the
/// message was still refused. D-0601.
fn batch_admission(batch_programs: u64, capture_bytes: u64) -> Result<(), String> {
    let admitted = capture_bytes / 4;
    let needed = batch_programs
        .checked_mul(runner::expression::ENCODED_LEN as u64)
        .and_then(|n| n.checked_add(crate::boolean_grammar_batch::HEADER as u64))
        .ok_or("single-stop batch program capacity overflows its byte admission")?;
    if needed > admitted {
        let affordable = admitted.saturating_sub(crate::boolean_grammar_batch::HEADER as u64)
            / runner::expression::ENCODED_LEN as u64;
        return Err(format!(
            "single-stop batch_programs {batch_programs} needs {needed} bytes but only {admitted} are admitted, which is ONE QUARTER of BRUTEX_CHECKSUM_MAX_BYTES. Either lower batch_programs to at most {affordable}, or raise BRUTEX_CHECKSUM_MAX_BYTES to at least {}",
            needed.saturating_mul(4)
        ));
    }
    Ok(())
}

fn validate(request: &Request<'_>) -> Result<(), String> {
    let config = request.configuration;
    if !matches!(request.index, "NSE-NIFTY" | "NSE-BANKNIFTY")
        || request.training.0 > request.training.1
        || request.later.0 > request.later.1
        || request.later.0 <= request.training.1
        || request.batch_programs == 0
        // Vacuous on the production path -- `index_stop_launch::request` assigns
        // `capture.programs` FROM `batch_programs`, so this reads `x > x` there.
        // Retained because it is not vacuous for a caller that supplies the two
        // independently, and because removing it would weaken that caller for no
        // gain. What it never did was bound the BYTES, which is `batch_admission`
        // below. D-0601.
        || request.batch_programs > config.capture.programs
        || request.node_allowance == 0
        || request.node_allowance > config.capture.records
        || request.batch_allowance == 0
        || config.workers == 0
        || !request.root.is_absolute()
        || !request.root.is_dir()
    {
        return Err(
            "single-stop search declaration, period order or physical admission refused".into(),
        );
    }
    batch_admission(request.batch_programs, config.capture.bytes)?;
    crate::parse_vendor(request.feed)?;
    month_days(request.training)?;
    month_days(request.later)?;
    Cursor::new(request.alphabet).map_err(debug)?;
    Ok(())
}
fn load_sources(
    request: &Request<'_>,
    load: fn(crate::index_stop::Request<'_>) -> Result<Loaded, String>,
    observe: &mut dyn FnMut(Observation) -> Result<(), String>,
) -> Result<Vec<Sources>, String> {
    request
        .rungs
        .labels()
        .iter()
        .map(|rung| {
            let rung_index = crate::ledger_all::LEDGER_RUNGS
                .iter()
                .position(|value| value == rung)
                .ok_or("single-stop physical timeframe mapping")?;
            let input = crate::index_stop::Request {
                store: request.root,
                vendor: request.feed,
                underlying: request.index,
                rung,
                from: request.training.0,
                to: request.training.1,
                strict: &request.configuration.strict,
            };
            live::preparing(observe, rung_index, RungStage::Preparing)?;
            let result = (|| {
                let training = load(input)?;
                let later = load(crate::index_stop::Request {
                    to: request.later.1,
                    ..input
                })?;
                Ok(Sources {
                    rung: rung_index,
                    training,
                    later,
                })
            })();
            if result.is_err() {
                live::preparing(observe, rung_index, RungStage::Refused)?;
            }
            result
        })
        .collect()
}
fn month_days(span: ((u16, u8), (u16, u8))) -> Result<(i64, i64), String> {
    crate::candidate_universe::requested_span_days(crate::population::RequestedSpanIdentityV1::new(
        span.0.0, span.0.1, span.1.0, span.1.1,
    )?)
}
fn declaration(request: &Request<'_>, sources: &[PreparedSources<'_>]) -> Result<Vec<u8>, String> {
    let legacy = legacy_declaration(request, sources)?;
    let length = 16_usize
        .checked_add(legacy.len())
        .and_then(|value| value.checked_add(runner::admission::ADMISSION_POLICY_CANONICAL_LEN_V1))
        .and_then(|value| {
            sources
                .len()
                .checked_mul(128)
                .and_then(|extra| value.checked_add(extra))
        })
        .ok_or("source-pinned declaration size overflow")?;
    if length as u64 > request.configuration.capture.bytes {
        return Err("source-pinned declaration exceeds byte admission".into());
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length).map_err(display)?;
    bytes.extend_from_slice(b"BRISSD02");
    bytes.extend_from_slice(&(legacy.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&legacy);
    bytes.extend_from_slice(&request.configuration.policy.canonical_bytes());
    for source in sources {
        for context in [&source.training_context, &source.later_context] {
            context.require_current()?;
            let link = context.link();
            bytes.extend_from_slice(&link.identity);
            bytes.extend_from_slice(&link.completion);
        }
    }
    Ok(bytes)
}

fn legacy_declaration(
    request: &Request<'_>,
    sources: &[PreparedSources<'_>],
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"BRISSD01");
    for text in [request.index, request.feed] {
        bytes.extend_from_slice(&u64::try_from(text.len()).map_err(display)?.to_le_bytes());
        bytes.extend_from_slice(text.as_bytes());
    }
    bytes.extend_from_slice(&[request.rungs.mask()]);
    for span in [request.training, request.later] {
        for day in [month_days(span)?.0, month_days(span)?.1] {
            bytes.extend_from_slice(&day.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&Cursor::new(request.alphabet).map_err(debug)?.encode());
    bytes.extend_from_slice(&request.configuration.policy.digest());
    bytes.extend_from_slice(&runner::signal_candle_stop::Policy::V1.digest());
    bytes.extend_from_slice(&crate::index_consistency::INDEX_STOP.digest());
    let config = request.configuration;
    for value in [
        request.batch_programs,
        request.node_allowance,
        config.capture.programs,
        config.capture.records,
        config.capture.bytes,
        config.procedure.draws(),
        config.procedure.seed(),
        config.procedure.block_length(),
        config.qualification.candidates,
        config.qualification.bootstrap_work,
        config.qualification.split_work,
        config.qualification.memory_bytes,
        config.qualification.bytes,
        config.strict.max_bytes(),
        config.strict.max_records(),
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for source in sources {
        bytes.extend_from_slice(&source.training.source_id());
        bytes.extend_from_slice(&source.later.source_id());
    }
    // Invocation allowances and scheduling do not change the fixed search or
    // replenish its error budget; every answer-changing bound above is fixed.
    Ok(bytes)
}
fn progress(identity: [u8; 32], frame: &Frame, rungs: RungScope) -> Progress {
    Progress {
        identity,
        completed_batches: frame.completed_batches,
        exhausted: frame.exhausted,
        qualifications: frame
            .links
            .map(|link| link.map(|value| (value.identity, value.pin))),
        completed_programs: frame.programs,
        completed_work: frame.work,
        current_batch: frame.pending.then_some(frame.completed_batches),
        rung_stages: std::array::from_fn(|rung| {
            if frame.pending && rungs.contains(rung) {
                Some(RungStage::Preparing)
            } else if frame.links.get(rung).is_some_and(Option::is_some) {
                Some(RungStage::Saved)
            } else {
                None
            }
        }),
        latest_saved: frame.latest_saved.map(|(batch, links)| CompletedBatch {
            batch,
            qualifications: links.map(|link| link.map(|value| (value.identity, value.pin))),
        }),
    }
}
fn display(value: impl std::fmt::Display) -> String {
    value.to_string()
}
fn debug(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}

#[cfg(test)]
#[path = "index_stop_search_tests.rs"]
mod tests;
