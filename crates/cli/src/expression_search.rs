//! Resumable historical traversal of the complete fixed expression language.
//! Each accepted candidate has its own full identity and exact saved signal
//! rows. Work budgets pause traversal; they never declare grammar exhaustion.

#[path = "expression_search_reader.rs"]
pub mod reader;

use crate::search_checkpoint::{Journal, Saved};
use runner::expression::{Expression, Summary};
use runner::identity::{Direction, Params, Run};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use vocab::expression_search::{CURSOR_BYTES, Cursor, Step};

const PAYLOAD_BYTES: usize = 8 + CURSOR_BYTES + 32 + 40 + 32 + 8 + 32 + 8;
const CHECKPOINT_MAX: u64 = 8192;
const HISTORY_LIMIT: u64 = 1_000_000;
const SIGNAL_FILE_MAX: u64 = 64 * 1024 * 1024;

struct Args<'a> {
    feed: &'a str,
    underlying: &'a str,
    rung: &'a str,
    year: u16,
    month: u8,
    live: Vec<u32>,
    min_hits: u64,
    candidates: u64,
    nodes: u64,
    pricing: Option<crate::expression_pricing::Plan>,
}

impl<'a> Args<'a> {
    fn parse(args: &[&'a str]) -> Result<Self, String> {
        let [
            feed,
            underlying,
            rung,
            year,
            month,
            bits,
            hits,
            candidates,
            nodes,
        ] = args
        else {
            return Err("expression-search-stored requires VENDOR UNDERLYING RUNG YEAR MONTH BITS MIN_HITS CANDIDATES NODES".to_owned());
        };
        let positive = |value: &str, name: &str| {
            value
                .parse::<u64>()
                .ok()
                .filter(|n| *n != 0)
                .ok_or_else(|| format!("{name} must be a positive integer"))
        };
        let year = year.parse().map_err(|_| "YEAR must be an integer")?;
        let month = month
            .parse::<u8>()
            .ok()
            .filter(|m| (1..=12).contains(m))
            .ok_or("MONTH must be 1..=12")?;
        let live = if *bits == "all" {
            runner::live_positions()
        } else {
            bits.split(',')
                .map(|s| {
                    s.parse::<u32>()
                        .map_err(|_| "BITS must be all or comma-separated live IDs".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        Cursor::new(&live)
            .map_err(|why| format!("BITS must be sorted, unique, live IDs: {why:?}"))?;
        Ok(Self {
            feed,
            underlying,
            rung,
            year,
            month,
            live,
            min_hits: positive(hits, "MIN_HITS")?,
            candidates: positive(candidates, "CANDIDATES")?,
            nodes: positive(nodes, "NODES")?,
            pricing: None,
        })
    }
}

pub(crate) fn stored(args: &[&str], out: &mut String) -> u8 {
    match Args::parse(args).and_then(|args| run(&args)) {
        Ok(report) => {
            out.push_str(&report);
            crate::OK
        }
        Err(why) => crate::refuse(out, &why),
    }
}

pub(crate) fn priced(args: &[&str], out: &mut String) -> u8 {
    let result = (|| {
        let mut parsed = Args::parse(args.get(..9).ok_or("missing search arguments")?)?;
        parsed.pricing = Some(crate::expression_pricing::Plan::parse(
            args.get(9..).ok_or("missing pricing arguments")?,
        )?);
        run(&parsed)
    })();
    match result {
        Ok(report) => {
            out.push_str(&report);
            crate::OK
        }
        Err(why) => crate::refuse(out, &why),
    }
}

struct Inputs {
    loaded: crate::stored::Loaded,
    daily: crate::stored::DailyContext,
    minute: crate::stored::ExactMinuteContext,
    execution: Option<crate::stored::Loaded>,
}
impl Inputs {
    fn load(root: &Path, args: &Args<'_>) -> Result<Self, String> {
        let vendor = crate::parse_vendor(args.feed)?;
        let loaded = crate::stored::load(
            root,
            vendor,
            args.underlying,
            args.rung,
            args.year,
            args.month,
        )?;
        let span = ((args.year, args.month), (args.year, args.month));
        let daily =
            crate::stored::load_daily_context(root, vendor, args.underlying, span, &loaded.bars)?;
        let minute = crate::stored::load_exact_minute_context(
            root,
            vendor,
            args.underlying,
            span,
            &loaded.bars,
        )?;
        let execution = if args.pricing.is_some() && args.rung != crate::EXECUTION_RUNG {
            Some(crate::stored::load(
                root,
                vendor,
                args.underlying,
                crate::EXECUTION_RUNG,
                args.year,
                args.month,
            )?)
        } else {
            None
        };
        Ok(Self {
            loaded,
            daily,
            minute,
            execution,
        })
    }
    fn execution_bars(&self, args: &Args<'_>) -> Result<&[indicators::Candle], String> {
        if args.rung == crate::EXECUTION_RUNG {
            Ok(&self.loaded.bars)
        } else {
            self.execution
                .as_ref()
                .map(|e| e.bars.as_slice())
                .ok_or_else(|| {
                    "priced expression requires its exact one-minute execution slice".to_owned()
                })
        }
    }
    fn verify_pricing(
        &self,
        args: &Args<'_>,
    ) -> Result<Option<crate::expression_pricing::Verify>, String> {
        args.pricing
            .map(|plan| {
                Ok(crate::expression_pricing::Verify {
                    plan,
                    execution_digest: runner::identity::data_digest(self.execution_bars(args)?),
                })
            })
            .transpose()
    }
    fn prepare_pricing(
        &self,
        args: &Args<'_>,
        column: &indicators::column::Column,
    ) -> Result<Option<crate::expression_pricing::Prepared>, String> {
        let Some(plan) = args.pricing else {
            return Ok(None);
        };
        let length = crate::stored::rung_length_micros(args.rung)?;
        let (bars, projected, execution_note) = crate::project_onto_execution(
            &self.loaded.bars,
            column,
            self.execution.as_ref().map(|e| crate::Execution {
                bars: &e.bars,
                signal_length_micros: length,
            }),
            args.rung == crate::EXECUTION_RUNG,
            plan.horizon,
        )?;
        Ok(Some(crate::expression_pricing::Prepared::new(
            bars,
            projected,
            plan,
            execution_note,
        )))
    }
}

fn run(args: &Args<'_>) -> Result<String, String> {
    crate::swept_rung(args.rung)?;
    let commit = crate::commit_stamp()
        .ok_or("expression search requires a clean verified build before historical computation")?;
    let root = crate::store_root()?;
    let inputs = Inputs::load(&root, args)?;
    let Inputs {
        loaded,
        daily,
        minute,
        ..
    } = &inputs;
    let cursor = Cursor::new(&args.live).map_err(debug)?;
    let run = Run {
        mask: cursor.alphabet(),
        direction: Direction::Undirected,
        instrument: &loaded.key,
        timeframe: loaded.timeframe,
        params: args.pricing.map_or(
            Params::of(engine::Ladder::with_min_hits(args.min_hits)),
            |plan| {
                Params::of(engine::Ladder::with_min_hits(args.min_hits)).with_policy(&plan.words())
            },
        ),
        data_digest: if args.pricing.is_some() {
            crate::stored_executed_digest(
                &loaded.bars,
                minute,
                daily,
                inputs.execution_bars(args)?,
            )?
        } else {
            crate::stored_anchored_digest(&loaded.bars, minute, daily)?
        },
        commit,
        feed: loaded.vendor.as_str(),
    };
    let identity = runner::expression::search_identity(&run, &cursor);
    let attempt = crate::sweep_evidence::begin(
        &root,
        identity,
        crate::sweep_evidence::Operation::ExpressionSearch,
    )?;
    let mut journal = Journal::open(&root, "expression-search-v1", identity)?;
    let saved = journal.latest(CHECKPOINT_MAX)?;
    let pricing_proof = inputs.verify_pricing(args)?;
    verify_history(&root, &journal, saved.as_ref(), &run, pricing_proof)?;
    let state = saved
        .as_ref()
        .map_or_else(|| Ok(State::new(cursor)), |s| State::decode(&s.payload))?;
    let column = crate::stored_anchored_column(
        &loaded.bars,
        daily,
        minute,
        crate::stored::rung_length_micros(args.rung)?,
        crate::stored::vwap_availability(&loaded.key),
    )?;
    if !column.census().reconciles() || column.census().refused() != 0 || column.bits().is_empty() {
        return Err(
            "expression search requires a nonempty, warm, reconciled signal column".to_owned(),
        );
    }
    let previous = saved
        .as_ref()
        .map_or((0, [0; 32]), |s| (s.sequence, s.seal));
    let pricing = inputs.prepare_pricing(args, &column)?;
    let state = execute(
        &root,
        &run,
        &column,
        &loaded.bars,
        &mut journal,
        state,
        previous,
        (args.candidates, args.nodes),
        pricing.as_ref(),
    )?;
    // A checkpoint publication alone cannot attest to child files saved earlier
    // in this invocation. Reopen the complete acknowledged history before the
    // outer attempt can publish even a truthful work-budget halt.
    verify_history(
        &root,
        &journal,
        journal.latest(CHECKPOINT_MAX)?.as_ref(),
        &run,
        pricing_proof,
    )?;
    let status = if state.exhausted {
        crate::sweep_evidence::Completion::Completed
    } else {
        crate::sweep_evidence::Completion::Halted
    };
    attempt.finish(status)?;
    Ok(render(
        args,
        &state,
        identity,
        journal.interrupted(),
        pricing.as_ref(),
    ))
}

fn render(
    args: &Args<'_>,
    state: &State,
    identity: [u8; 32],
    interrupted: u64,
    pricing: Option<&crate::expression_pricing::Prepared>,
) -> String {
    let mut out = String::from(crate::STORED_PROVENANCE);
    let _ = writeln!(
        out,
        "EXPRESSION SEARCH V1\n  identity: {}\n  status: {}\n  evaluated expressions: {}\n  expressions meeting support {}: {}\n  grammar work: {}\n  signal rows saved: {}\n  interrupted/uncommitted checkpoint reservations observed: {}\nRun the same command again to continue; CANDIDATES and NODES are work budgets, not a depth limit. Complete syntax enumeration may be enormous.",
        hex(&identity),
        if state.exhausted {
            "entire fixed grammar exhausted"
        } else {
            "PAUSED at a work budget; grammar NOT exhausted"
        },
        state.candidates,
        args.min_hits,
        state.qualifying,
        state.work,
        state.rows,
        interrupted
    );
    if let Some(plan) = args.pricing {
        if let Some(pricing) = pricing {
            out.push_str(&pricing.execution_note);
        }
        out.push_str("Cost-excluded, unvalidated research; printed-fill amounts are not net trading profits.\n");
        let _ = writeln!(
            out,
            "PRICED EXPRESSION RESEARCH\n  Both directions of every evaluated expression have exact saved selected-cell trades or an explicit no-cell outcome.\n  Hold: {} execution minutes; grid rungs: {}; step: {} ppm.\n  Applied cell policy: {:?}\n  MIN_HITS reports signal support; it does not skip pricing or establish institutional admission.",
            plan.horizon.as_bars(),
            plan.rungs,
            plan.step,
            plan.rules
        );
    } else {
        out.push_str("Signal research only: no priced trades or institutional approval.\n");
    }
    out
}

#[derive(Clone)]
struct Candidate {
    identity: [u8; 32],
    attempt: u64,
    summary: Summary,
}

struct State {
    cursor: Cursor,
    work: u64,
    candidates: u64,
    qualifying: u64,
    rows: u64,
    previous: (u64, [u8; 32]),
    last: Option<Candidate>,
    exhausted: bool,
}

impl State {
    fn new(cursor: Cursor) -> Self {
        Self {
            cursor,
            work: 0,
            candidates: 0,
            qualifying: 0,
            rows: 0,
            previous: (0, [0; 32]),
            last: None,
            exhausted: false,
        }
    }
    fn encode(&self) -> [u8; PAYLOAD_BYTES] {
        let mut bytes = [0; PAYLOAD_BYTES];
        let candidate = self.last.clone().unwrap_or(Candidate {
            identity: [0; 32],
            attempt: 0,
            summary: Summary::default(),
        });
        let numbers = [self.work, self.candidates, self.qualifying, self.rows];
        let counts = [
            candidate.summary.evaluated,
            candidate.summary.hits,
            candidate.summary.misses,
            candidate.summary.unknown,
        ];
        let content = b"BTXESR01"
            .iter()
            .copied()
            .chain(self.cursor.encode())
            .chain(numbers.into_iter().flat_map(u64::to_le_bytes))
            .chain(self.previous.0.to_le_bytes())
            .chain(self.previous.1)
            .chain(candidate.identity)
            .chain(candidate.attempt.to_le_bytes())
            .chain(counts.into_iter().flat_map(u64::to_le_bytes))
            .chain([
                u8::from(self.last.is_some()),
                u8::from(self.exhausted),
                0,
                0,
                0,
                0,
                0,
                0,
            ]);
        for (slot, byte) in bytes.iter_mut().zip(content) {
            *slot = byte;
        }
        bytes
    }
    fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != PAYLOAD_BYTES || bytes.get(..8) != Some(b"BTXESR01") {
            return Err("expression checkpoint format mismatch".to_owned());
        }
        let cursor = Cursor::decode(
            bytes
                .get(8..8 + CURSOR_BYTES)
                .and_then(|s| s.try_into().ok())
                .ok_or("cursor width mismatch")?,
        )
        .map_err(debug)?;
        let mut at = 8 + CURSOR_BYTES;
        let number = |at: &mut usize| -> Result<u64, String> {
            let n = bytes
                .get(*at..*at + 8)
                .and_then(|s| <[u8; 8]>::try_from(s).ok())
                .map(u64::from_le_bytes)
                .ok_or("checkpoint number missing")?;
            *at += 8;
            Ok(n)
        };
        let work = number(&mut at)?;
        let candidates = number(&mut at)?;
        let qualifying = number(&mut at)?;
        let rows = number(&mut at)?;
        let sequence = number(&mut at)?;
        let digest = bytes
            .get(at..at + 32)
            .and_then(|s| s.try_into().ok())
            .ok_or("previous checkpoint seal missing")?;
        at += 32;
        let identity = bytes
            .get(at..at + 32)
            .and_then(|s| s.try_into().ok())
            .ok_or("candidate identity missing")?;
        at += 32;
        let attempt = number(&mut at)?;
        let summary = Summary {
            evaluated: number(&mut at)?,
            hits: number(&mut at)?,
            misses: number(&mut at)?,
            unknown: number(&mut at)?,
        };
        let flags = bytes.get(at..).ok_or("checkpoint flags missing")?;
        if flags.len() != 8
            || flags.iter().skip(2).any(|&b| b != 0)
            || !matches!(flags.first(), Some(0 | 1))
            || !matches!(flags.get(1), Some(0 | 1))
            || qualifying > candidates
            || !summary.reconciles()
        {
            return Err("invalid expression checkpoint counters/flags".to_owned());
        }
        let present = flags.first() == Some(&1);
        if (present && (attempt == 0 || summary.evaluated == 0 || candidates == 0))
            || (!present && (identity != [0; 32] || attempt != 0 || summary != Summary::default()))
            || (sequence == 0 && digest != [0; 32])
        {
            return Err("invalid expression candidate reference".to_owned());
        }
        let exhausted = flags.get(1) == Some(&1);
        let mut checked = cursor.clone();
        if exhausted != matches!(checked.advance(0, &mut 0).map_err(debug)?, Step::Exhausted) {
            return Err("expression exhaustion flag disagrees with cursor".to_owned());
        }
        Ok(Self {
            cursor,
            work,
            candidates,
            qualifying,
            rows,
            previous: (sequence, digest),
            last: present.then_some(Candidate {
                identity,
                attempt,
                summary,
            }),
            exhausted,
        })
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "one prepared search transaction keeps exact data, run identity and persistence explicit"
)]
fn execute(
    root: &Path,
    run: &Run<'_>,
    column: &indicators::column::Column,
    bars: &[indicators::Candle],
    journal: &mut Journal,
    mut state: State,
    mut previous: (u64, [u8; 32]),
    budgets: (u64, u64),
    pricing: Option<&crate::expression_pricing::Prepared>,
) -> Result<State, String> {
    let start_count = state.candidates;
    let start_work = state.work;
    while !state.exhausted
        && state.candidates - start_count < budgets.0
        && state.work - start_work < budgets.1
    {
        let remaining = budgets.1 - (state.work - start_work);
        match state
            .cursor
            .advance(remaining.min(4096), &mut state.work)
            .map_err(debug)?
        {
            Step::Candidate(expression) => {
                let candidate = evaluate_candidate(root, run, column, bars, &expression, pricing)?;
                state.candidates = add(state.candidates, 1)?;
                state.rows = add(state.rows, candidate.summary.evaluated)?;
                state.qualifying = add(
                    state.qualifying,
                    u64::from(candidate.summary.hits >= run.params.min_hits),
                )?;
                state.last = Some(candidate);
                state.previous = previous;
                previous = journal.publish(&state.encode(), CHECKPOINT_MAX)?;
                state.last = None;
            }
            Step::Paused => {
                state.previous = previous;
                state.last = None;
                previous = journal.publish(&state.encode(), CHECKPOINT_MAX)?;
            }
            Step::Exhausted => state.exhausted = true,
        }
    }
    state.previous = previous;
    state.last = None;
    journal.publish(&state.encode(), CHECKPOINT_MAX)?;
    Ok(state)
}

fn candidate_identity(run: &Run<'_>, expression: &Expression) -> [u8; 32] {
    let candidate = Run {
        mask: expression.referenced(),
        direction: run.direction,
        instrument: run.instrument,
        timeframe: run.timeframe,
        params: run.params,
        data_digest: run.data_digest,
        commit: run.commit,
        feed: run.feed,
    };
    runner::expression::identity(&candidate, expression)
}

fn evaluate_candidate(
    root: &Path,
    run: &Run<'_>,
    column: &indicators::column::Column,
    bars: &[indicators::Candle],
    expression: &Expression,
    pricing: Option<&crate::expression_pricing::Prepared>,
) -> Result<Candidate, String> {
    if u64::try_from(column.bits().len())
        .map_err(debug)?
        .checked_mul(56)
        .and_then(|n| n.checked_add(3561))
        .is_none_or(|n| n > SIGNAL_FILE_MAX)
    {
        return Err(
            "one expression signal file exceeds its 64 MiB admission; no candidate was evaluated"
                .to_owned(),
        );
    }
    let identity = candidate_identity(run, expression);
    let attempt =
        crate::sweep_evidence::begin(root, identity, crate::sweep_evidence::Operation::Expression)?;
    let directory =
        crate::expression::attempt_directory(root, identity, attempt.token()).map_err(debug)?;
    let mut writer = crate::expression::EvidenceWriter::begin(&directory, identity, expression)
        .map_err(debug)?;
    let summary = runner::expression::evaluate(column, expression, |source, truth| {
        let bar = bars.get(source).ok_or_else(|| {
            std::io::Error::other("expression source outside exact historical slice")
        })?;
        writer.row(source, bar.ts_micros, truth)
    })
    .map_err(debug)?;
    writer.finish(summary).map_err(debug)?;
    if let Some(pricing) = pricing {
        pricing.capture(root, &attempt, expression)?;
    }
    let token = attempt.token();
    attempt.finish(crate::sweep_evidence::Completion::Completed)?;
    Ok(Candidate {
        identity,
        attempt: token,
        summary,
    })
}

fn verify_history(
    root: &Path,
    journal: &Journal,
    latest: Option<&Saved>,
    run: &Run<'_>,
    pricing: Option<crate::expression_pricing::Verify>,
) -> Result<(), String> {
    let Some(latest) = latest else {
        return Ok(());
    };
    let mut current = State::decode(&latest.payload)?;
    let mut sequence = latest.sequence;
    let alphabet = run.mask;
    for _ in 0..HISTORY_LIMIT {
        if current.cursor.alphabet() != alphabet {
            return Err("checkpoint search alphabet differs from run".to_owned());
        }
        if let Some(candidate) = &current.last {
            let evidence = crate::sweep_evidence::read_attempt(
                root,
                candidate.identity,
                candidate.attempt,
                SIGNAL_FILE_MAX,
            )?
            .ok_or("expression candidate has no durable attempt")?;
            if evidence.operation != crate::sweep_evidence::Operation::Expression
                || evidence.completion != crate::sweep_evidence::Completion::Completed
            {
                return Err("expression candidate lacks completed expression authority".to_owned());
            }
            let path = candidate_path(root, candidate);
            let (identity, expression, summary) =
                crate::expression::read_saved(&path, SIGNAL_FILE_MAX, |_| Ok(())).map_err(debug)?;
            if identity != candidate.identity
                || candidate_identity(run, &expression) != identity
                || summary != candidate.summary
            {
                return Err(
                    "saved candidate evidence differs from its search checkpoint".to_owned(),
                );
            }
            if let Some(pricing) = pricing {
                pricing.candidate(root, identity, candidate.attempt, &expression)?;
            }
        }
        if current.previous.0 == 0 {
            let increment = u64::from(current.last.is_some());
            if current.candidates != increment
                || current.rows != current.last.as_ref().map_or(0, |c| c.summary.evaluated)
                || current.qualifying
                    != u64::from(
                        current
                            .last
                            .as_ref()
                            .is_some_and(|c| c.summary.hits >= run.params.min_hits),
                    )
            {
                return Err("initial expression checkpoint counters mismatch".to_owned());
            }
            let initial =
                State::new(Cursor::decode(&current.cursor.initial_descriptor()).map_err(debug)?);
            verify_transition(&initial, &current, run)?;
            return Ok(());
        }
        if current.previous.0 >= sequence {
            return Err("cyclic expression checkpoint history".to_owned());
        }
        let prior = journal.read(current.previous.0, CHECKPOINT_MAX)?;
        if prior.seal != current.previous.1 {
            return Err("expression checkpoint predecessor seal changed".to_owned());
        }
        let before = State::decode(&prior.payload)?;
        if current.work < before.work
            || current.candidates != add(before.candidates, u64::from(current.last.is_some()))?
            || current.rows
                != add(
                    before.rows,
                    current.last.as_ref().map_or(0, |c| c.summary.evaluated),
                )?
            || current.qualifying
                != add(
                    before.qualifying,
                    u64::from(
                        current
                            .last
                            .as_ref()
                            .is_some_and(|c| c.summary.hits >= run.params.min_hits),
                    ),
                )?
        {
            return Err("expression checkpoint history does not reconcile".to_owned());
        }
        verify_transition(&before, &current, run)?;
        current = before;
        sequence = prior.sequence;
    }
    Err("expression checkpoint history admission limit exceeded".to_owned())
}

fn verify_transition(before: &State, current: &State, run: &Run<'_>) -> Result<(), String> {
    let delta = current
        .work
        .checked_sub(before.work)
        .filter(|&n| n <= 4096)
        .ok_or("expression checkpoint work transition exceeds its exact admission")?;
    let mut cursor = before.cursor.clone();
    let mut work = before.work;
    let step = cursor.advance(delta, &mut work).map_err(debug)?;
    let expected_candidate = match &step {
        Step::Candidate(expression) => Some(candidate_identity(run, expression)),
        Step::Paused | Step::Exhausted => None,
    };
    if work != current.work
        || cursor.encode() != current.cursor.encode()
        || expected_candidate != current.last.as_ref().map(|candidate| candidate.identity)
        || matches!(step, Step::Exhausted) != current.exhausted
    {
        return Err(
            "expression checkpoint does not prove the exact next grammar transition".to_owned(),
        );
    }
    Ok(())
}

fn candidate_path(root: &Path, candidate: &Candidate) -> PathBuf {
    root.join("expression-v1")
        .join(hex(&candidate.identity))
        .join(candidate.attempt.to_string())
        .join("expression-v1.rows")
}
fn add(a: u64, b: u64) -> Result<u64, String> {
    a.checked_add(b)
        .ok_or_else(|| "expression search counter overflow".to_owned())
}
fn debug(why: impl std::fmt::Debug) -> String {
    format!("{why:?}")
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

#[cfg(test)]
#[path = "expression_search_tests.rs"]
mod tests;
