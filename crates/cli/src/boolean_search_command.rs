//! Automatic bounded continuation of the fixed grammar with one declared error budget.
use crate::boolean_campaign::{self as campaign, Prepared as Campaign};
use crate::boolean_catalog_command::prepared::{Input, Prepared};
use crate::boolean_grammar_batch::Batch;
use crate::boolean_qualification_plan::Plan;
use crate::boolean_search_reader::{Reader, transition};
use crate::boolean_search_record::{NAMESPACE, Record, Spec, Summary, debug, display};
use crate::search_checkpoint::Journal;
use std::fmt::Write as _;
use std::path::Path;
use vocab::expression_search::Cursor;

#[path = "boolean_search_launch.rs"]
pub mod launch;

struct Request<'a> {
    rungs: campaign::RungScope,
    input: Input<'a>,
    initial: Cursor,
    later_from: (u16, u8),
    later_to: (u16, u8),
    programs: u64,
    nodes: u64,
    batches: u64,
}
pub(crate) fn command(args: &[&str], out: &mut String) -> u8 {
    match parse(args).and_then(|request| execute(&request, out)) {
        Ok(()) => crate::OK,
        Err(why) => crate::refuse(out, &why),
    }
}
fn parse<'a>(args: &[&'a str]) -> Result<Request<'a>, String> {
    let (args, rungs) = match args {
        [arguments @ .., selected] if arguments.len() == 17 => {
            if selected.len() > 64 {
                return Err("qualified search timeframe list exceeds input admission".into());
            }
            (
                arguments,
                campaign::RungScope::new(&selected.split(',').collect::<Vec<_>>())?,
            )
        }
        _ => (args, campaign::RungScope::ALL),
    };
    let [
        vendor,
        symbols,
        fy,
        fm,
        ty,
        tm,
        bits,
        horizon,
        points,
        programs,
        nodes,
        batches,
        output,
        lfy,
        lfm,
        lty,
        ltm,
    ] = args
    else {
        return Err("boolean-qualified-search-stored requires 17 explicit arguments: VENDOR SYMBOLS FY FM TY TM BITS HORIZON MAX_POINTS BATCH_PROGRAMS NODE_ALLOWANCE BATCH_ALLOWANCE OUTPUT LATER_FY LATER_FM LATER_TY LATER_TM [TIMEFRAMES comma-separated; omitted means all eight]".into());
    };
    let from = month(fy, fm)?;
    let to = month(ty, tm)?;
    let later_from = month(lfy, lfm)?;
    let later_to = month(lty, ltm)?;
    if from > to || later_from > later_to || later_from <= to {
        return Err("qualified search requires ordered wholly later month spans".into());
    }
    let alphabet = if *bits == "all" {
        runner::live_positions()
    } else {
        if bits.len() > 4096 {
            return Err("qualified search alphabet exceeds input admission".into());
        }
        bits.split(',')
            .map(|bit| bit.parse::<u32>().map_err(display))
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(Request {
        rungs,
        input: Input {
            vendor,
            symbols,
            from,
            to,
            horizon: crate::knobs::horizon_count(horizon)
                .ok_or("search horizon must be positive")?,
            max_points: positive(points)?,
            output: Path::new(*output),
        },
        initial: Cursor::new(&alphabet).map_err(debug)?,
        later_from,
        later_to,
        programs: positive(programs)?,
        nodes: positive(nodes)?,
        batches: positive(batches)?,
    })
}
fn month(year: &str, month: &str) -> Result<(u16, u8), String> {
    let year = year.parse().map_err(display)?;
    let month = month.parse().map_err(display)?;
    pull::session::Day::new(year, month, 1).map_err(display)?;
    Ok((year, month))
}
fn positive(raw: &str) -> Result<u64, String> {
    raw.parse::<u64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == raw)
        .ok_or_else(|| "search allowance must be a canonical positive integer".into())
}
impl Request<'_> {
    fn prepare(
        &self,
        programs: &[runner::expression::Expression],
        later: bool,
        out: &mut String,
        prepare: fn(&campaign::Request<'_>, &mut String) -> Result<Campaign, String>,
    ) -> Result<Campaign, String> {
        prepare(
            &campaign::Request {
                rungs: self.rungs,
                vendor: self.input.vendor,
                symbols: self.input.symbols,
                from: if later {
                    self.later_from
                } else {
                    self.input.from
                },
                to: if later { self.later_to } else { self.input.to },
                programs,
                horizon: self.input.horizon,
                max_points: self.input.max_points,
                output: self.input.output,
                max_rung_jobs: self.rungs.count(),
            },
            out,
        )
    }
}

fn execute(request: &Request<'_>, out: &mut String) -> Result<(), String> {
    execute_with(request, out, Prepared::new, campaign::prepare)
}

fn execute_with(
    request: &Request<'_>,
    out: &mut String,
    make_input: fn(Input<'_>, &mut String) -> Result<Prepared, String>,
    make_campaign: fn(&campaign::Request<'_>, &mut String) -> Result<Campaign, String>,
) -> Result<(), String> {
    execute_observed(request, out, make_input, make_campaign, None, &mut |_| {
        Ok(())
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "one ordered declaration and receipt-last search transaction"
)]
fn execute_observed(
    request: &Request<'_>,
    out: &mut String,
    make_input: fn(Input<'_>, &mut String) -> Result<Prepared, String>,
    make_campaign: fn(&campaign::Request<'_>, &mut String) -> Result<Campaign, String>,
    expected: Option<[u8; 32]>,
    notify: &mut dyn FnMut(launch::Progress) -> Result<(), String>,
) -> Result<(), String> {
    let input = make_input(request.input, out)?;
    if let Some(expected) = expected
        && launch::fingerprint(&input) != expected
    {
        return Err("Boolean launch configuration changed after admission; no search ran".into());
    }
    let bytes = input.strict.max_bytes();
    let records = input.strict.max_records();
    if request.nodes > records {
        return Err("search node allowance exceeds configured replay-work admission".into());
    }
    let first = Batch::prepare(
        request.initial.clone(),
        0,
        0,
        crate::boolean_grammar_batch::Budget {
            programs: request.programs,
            nodes: request.nodes,
            bytes: bytes / 4,
        },
    )?;
    let training = request.prepare(first.programs(), false, out, make_campaign)?;
    let later = request.prepare(first.programs(), true, out, make_campaign)?;
    if expected.is_some()
        && (training.preparation_policy_digest() != input.policy_digest()
            || later.preparation_policy_digest() != input.policy_digest())
    {
        return Err("Boolean launch policy changed during source preparation".into());
    }
    let values = input.policy().values();
    let spec = Spec {
        rungs: request.rungs,
        projection_rule: crate::boolean_search_record::ProjectionRule::IndexConsistencyV4,
        sources: [training.descriptor_digest(), later.descriptor_digest()],
        policy: input.policy().digest(),
        initial: request.initial.initial_descriptor(),
        programs: request.programs,
        nodes: request.nodes,
        bytes,
        records,
        alpha: values
            .max_fwer_p_value_ppm
            .min(values.max_romano_wolf_p_value_ppm),
    };
    spec.validate()?;
    let identity = spec.identity();
    let observe = bytes
        .checked_mul(4)
        .ok_or("search observation ceiling overflow")?;
    let mut journal = Journal::open(request.input.output, NAMESPACE, identity)?;
    let mut latest = if journal.latest(bytes)?.is_some() {
        let reader = Reader::open(request.input.output, identity, observe, records)?;
        if reader.spec() != &spec {
            return Err("qualified search declaration changed".into());
        }
        // Cold restart rechecks every earlier result before any completed work is skipped.
        for batch in 0..reader.completed_batches() {
            reader.verify_batch(batch as u64)?;
        }
        Some(Record::decode(
            &journal
                .latest(bytes)?
                .ok_or("search latest record disappeared")?
                .payload,
            bytes,
        )?)
    } else {
        None
    };
    training.require_current()?;
    later.require_current()?;
    if latest.is_some() {
        notify(launch::Progress {
            identity,
            exhausted: None,
            completed_batches: None,
        })?;
    }
    let _ = writeln!(
        out,
        "\nQUALIFIED GRAMMAR SEARCH {}\nOne immutable countable testing allowance covers this declared grammar across selected intraday rungs {:?}. Eight statistical units remain reserved; unselected units are unspent. Failed, empty and retried batches never recycle it. Every result remains cost-excluded research; bootstrap validity assumptions remain.\nProgress: /backtest?boolean_qualified_search={}\n| Batch | State | Programs | Choices |\n|---:|---|---:|---:|",
        crate::identity_hex(&identity),
        request.rungs.labels(),
        crate::identity_hex(&identity)
    );
    for _ in 0..request.batches {
        if latest
            .as_ref()
            .is_some_and(|r| r.phase == 1 && r.batch.exhausted())
        {
            break;
        }
        admit(
            &journal,
            latest.as_ref().is_some_and(|r| r.phase != 1),
            &spec,
        )?;
        let mut record = match latest.take() {
            Some(old) if old.phase != 1 => old,
            old => {
                let (cursor, work, count, ordinal) = if let Some(old) = &old {
                    (
                        old.batch.next_cursor(),
                        old.batch.work(),
                        old.batch.cumulative_programs()?,
                        old.ordinal
                            .checked_add(1)
                            .ok_or("search batch ordinal exhausted")?,
                    )
                } else {
                    (request.initial.clone(), 0, 0, 0)
                };
                let batch = Batch::prepare(cursor, work, count, spec.budget())?;
                let plan = if batch.programs().is_empty() {
                    Vec::new()
                } else {
                    let before = request.prepare(batch.programs(), false, out, make_campaign)?;
                    let after = request.prepare(batch.programs(), true, out, make_campaign)?;
                    require_sources(&spec, &before, &after)?;
                    Plan::new(&before, &after)?.canonical_bytes()?
                };
                let record = Record {
                    previous: journal.latest(bytes)?.map(|s| (s.sequence, s.seal)),
                    spec: spec.clone(),
                    ordinal,
                    phase: 0,
                    batch,
                    plan,
                    campaign: Summary::EMPTY.child,
                    summaries: [Summary::EMPTY; 8],
                    reason: String::new(),
                };
                transition(old.as_ref(), &record)?;
                // This acknowledged declaration precedes any later pricing/statistics.
                admit_history(&journal, &record, 2, request.input.output, observe)?;
                training.require_current()?;
                later.require_current()?;
                journal.publish(&record.encode()?, bytes)?;
                // Publish the browser link only after the declaration is durably readable.
                notify(launch::Progress {
                    identity,
                    exhausted: None,
                    completed_batches: None,
                })?;
                record
            }
        };
        admit_history(&journal, &record, 1, request.input.output, observe)?;
        let work = (|| {
            if !record.batch.programs().is_empty() {
                let before = request.prepare(record.batch.programs(), false, out, make_campaign)?;
                let after = request.prepare(record.batch.programs(), true, out, make_campaign)?;
                require_sources(&spec, &before, &after)?;
                if Plan::new(&before, &after)?.canonical_bytes()? != record.plan {
                    return Err("search batch pre-work qualification plan changed".into());
                }
                let link = crate::boolean_qualified_command::run_prepared(
                    &input,
                    &before,
                    &after,
                    record.batch.programs(),
                    out,
                )?;
                let campaign = crate::boolean_evidence::QualifiedCampaign::open(
                    request.input.output,
                    link.identity,
                    observe,
                )?;
                let plan = record.observed_plan()?;
                if campaign.pin() != link.pin
                    || campaign.descriptor() != plan.descriptor()
                    || campaign.rungs() != plan.rungs()
                {
                    return Err("qualified search campaign binding changed".into());
                }
                for (rung, ((slot, unit), summary)) in campaign
                    .slots()
                    .iter()
                    .zip(plan.units())
                    .zip(record.summaries.iter_mut())
                    .enumerate()
                {
                    if !spec.rungs.contains(rung) {
                        continue;
                    }
                    let child = slot
                        .complete
                        .ok_or("qualified search child is unfinished")?;
                    let source = crate::boolean_evidence::Qualification::open(
                        request.input.output,
                        child.identity,
                        observe
                            .checked_sub(campaign.admitted_bytes())
                            .ok_or("search ancestry admission exhausted")?,
                    )?;
                    if source.completion_digest() != child.pin
                        || source.scope() != plan.descriptor()
                        || source.unit() != *unit
                        || source.allocation() != plan.allocation(rung)?
                    {
                        return Err(
                            "qualified search child belongs to another declared source/rung".into(),
                        );
                    }
                    let allocation = runner::search_allocation_v1::allocate(
                        record.ordinal,
                        rung as u64,
                        spec.alpha,
                    )
                    .map_err(debug)?;
                    *summary = crate::boolean_search_projection::summarize(
                        &source,
                        allocation,
                        spec.projection_rule,
                    )?;
                }
                record.campaign = link;
                before.require_current()?;
                after.require_current()?;
                campaign.require_current()?;
            }
            training.require_current()?;
            later.require_current()?;
            record.phase = 1;
            record.reason.clear();
            record.previous = journal.latest(bytes)?.map(|s| (s.sequence, s.seal));
            record.validate()?;
            if !record.batch.programs().is_empty() {
                for rung in spec.rungs.indices() {
                    drop(crate::boolean_search_reader::open_rung(
                        request.input.output,
                        &record,
                        rung,
                        observe,
                    )?);
                }
            }
            // Before a new completion, reconcile previously finished children again.
            let prior = Reader::open(request.input.output, identity, observe, records)?;
            for batch in 0..prior.completed_batches() {
                prior.verify_batch(batch as u64)?;
            }
            training.require_current()?;
            later.require_current()?;
            journal.publish(&record.encode()?, bytes)?;
            let saved = Reader::open(request.input.output, identity, observe, records)?;
            for batch in 0..saved.completed_batches() {
                saved.verify_batch(batch as u64)?;
            }
            training.require_current()?;
            later.require_current()?;
            Ok::<(), String>(())
        })();
        if let Err(why) = work {
            return Err(settle_failure(&mut journal, &mut record, bytes, &why));
        }
        let _ = writeln!(
            out,
            "| {} | Saved and verified | {} | {} |",
            record.ordinal,
            record.batch.cumulative_programs()?,
            record.batch.work()
        );
        latest = Some(record);
    }
    let saved = Reader::open(request.input.output, identity, observe, records)?;
    training.require_current()?;
    later.require_current()?;
    // Outcome is read from the acknowledged journal, never inferred from return prose.
    let observed = launch::Progress {
        identity,
        exhausted: Some(saved.exhausted()),
        completed_batches: Some(saved.completed_batches() as u64),
    };
    notify(observed)?;
    let _ = writeln!(
        out,
        "{}; {} completed batches, checkpoint {} / {}. No total completion percentage is invented.\nDashboard evidence root: {}; BRUTEX_BOOLEAN_OBSERVATION_BYTES={observe}",
        if saved.exhausted() && request.rungs != campaign::RungScope::ALL {
            "FIXED GRAMMAR EXHAUSTED FOR SELECTED TIMEFRAMES"
        } else if saved.exhausted() {
            "FIXED GRAMMAR EXHAUSTED"
        } else {
            "PAUSED AT INVOCATION WORK ALLOWANCE; identical request resumes automatically"
        },
        saved.completed_batches(),
        saved.sequence(),
        crate::identity_hex(&saved.pin()),
        request.input.output.display()
    );
    let _ = writeln!(
        out,
        "BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES={records}; saved history must fit this independent replay-work allowance."
    );
    Ok(())
}

fn settle_failure(journal: &mut Journal, record: &mut Record, bytes: u64, why: &str) -> String {
    let settlement = (|| {
        let persisted = journal
            .latest(bytes)?
            .ok_or("search reservation missing after work")?;
        let old = Record::decode(&persisted.payload, bytes)?;
        // Never append a refusal over an already acknowledged completion.
        if old.phase == 1 {
            return Ok::<_, String>("completion remains recorded; subsequent verification failed");
        }
        record.phase = 2;
        record.campaign = Summary::EMPTY.child;
        record.summaries = [Summary::EMPTY; 8];
        record.previous = Some((persisted.sequence, persisted.seal));
        record.reason = bounded_reason(why);
        journal.publish(&record.encode()?, bytes)?;
        Ok("refusal recorded; the same reservation and allowance remain saved")
    })();
    match settlement {
        Ok(state) => format!(
            "qualified search batch {} failed: {why}; {state}",
            record.ordinal
        ),
        Err(audit) => format!(
            "qualified search batch {} failed: {why}; terminal audit could not be recorded: {audit}",
            record.ordinal
        ),
    }
}

fn require_sources(spec: &Spec, training: &Campaign, later: &Campaign) -> Result<(), String> {
    if spec.sources != [training.descriptor_digest(), later.descriptor_digest()]
        || spec.policy != training.policy().digest()
        || spec.policy != later.policy().digest()
    {
        return Err("qualified search source or research policy changed".into());
    }
    training.require_current()?;
    later.require_current()
}
fn bounded_reason(why: &str) -> String {
    let mut length = why.len().min(1024);
    while !why.is_char_boundary(length) {
        length -= 1;
    }
    if length == 0 {
        "unspecified search failure".into()
    } else {
        why[..length].to_owned()
    }
}
fn admit(journal: &Journal, pending: bool, spec: &Spec) -> Result<(), String> {
    let needed = if pending { 1 } else { 2 };
    let ceiling = spec
        .records
        .min((crate::search_checkpoint::DIRECTORY_LIMIT - 1) as u64);
    if journal
        .next_sequence()
        .checked_add(needed - 1)
        .is_none_or(|last| last > ceiling)
    {
        return Err(
            "search checkpoint capacity reached before pricing; saved state remains resumable"
                .into(),
        );
    }
    Ok(())
}

fn admit_history(
    journal: &Journal,
    record: &Record,
    needed: u64,
    root: &Path,
    observe: u64,
) -> Result<(), String> {
    let count = journal
        .acknowledged()
        .checked_add(needed)
        .and_then(|n| n.checked_add(1))
        .ok_or("search replay reservation overflow")?;
    if count
        .checked_mul(record.spec.nodes)
        .is_none_or(|n| n > record.spec.records)
    {
        return Err("search complete-history replay allowance reached before pricing; no allocation was recycled".into());
    }
    let past = if journal.acknowledged() == 0 {
        0
    } else {
        Reader::open(root, record.spec.identity(), observe, record.spec.records)?.admitted_bytes()
    };
    let payload = record.encode()?.len() as u64;
    let each = payload
        .checked_add(
            (96 + size_of::<crate::search_checkpoint::Saved>() + size_of::<usize>()) as u64,
        )
        .ok_or("search byte reservation overflow")?;
    if each
        .checked_mul(needed)
        .and_then(|n| past.checked_add(n))
        .is_none_or(|n| n > observe / 2)
    {
        return Err(
            "search complete-history bytes reached before pricing; saved progress remains intact"
                .into(),
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "boolean_search_integration_tests.rs"]
mod integration_tests;

#[cfg(test)]
#[path = "boolean_search_command_tests.rs"]
mod tests;
