//! A resumable grammar batch advances only after its eight-rung campaign.
//! Each batch is a separate statistical comparison; completing batches does
//! not establish a multiple-testing correction over the whole grammar.

use std::fmt::Write as _;
use std::path::Path;

use brutex_core::blake3::Hasher;
use vocab::expression_search::Cursor;

use crate::boolean_grammar_batch::{Batch, Budget};
use crate::search_checkpoint::{Journal, Saved};

const NAMESPACE: &str = "boolean-grammar-v1";
const PLAN: &[u8; 8] = b"BRBGPN01";
const DONE: &[u8; 8] = b"BRBGDN01";
const ENVELOPE: usize = 48;

struct Request<'a> {
    vendor: &'a str,
    symbols: &'a str,
    from: (u16, u8),
    to: (u16, u8),
    initial: Cursor,
    horizon: runner::outcome::Horizon,
    max_points: u64,
    programs: u64,
    nodes: u64,
    root: &'a Path,
}

pub(crate) fn command(args: &[&str], out: &mut String) -> u8 {
    match parse(args).and_then(|request| execute(&request, out)) {
        Ok(()) => crate::OK,
        Err(why) => crate::refuse(out, &why),
    }
}

fn parse<'a>(args: &[&'a str]) -> Result<Request<'a>, String> {
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
        root,
    ] = args
    else {
        return Err("boolean-grammar-campaign-stored requires its 12 explicit arguments".into());
    };
    let month = |year: &str, month: &str| -> Result<(u16, u8), String> {
        let year = year.parse::<u16>().map_err(display)?;
        let month = month.parse::<u8>().map_err(display)?;
        pull::session::Day::new(year, month, 1).map_err(display)?;
        Ok((year, month))
    };
    let from = month(fy, fm)?;
    let to = month(ty, tm)?;
    if from > to {
        return Err("grammar campaign requires an ordered historical month span".into());
    }
    let live = if *bits == "all" {
        runner::live_positions()
    } else {
        if bits.len() > 4096 {
            return Err("grammar alphabet exceeds its input byte admission".into());
        }
        bits.split(',')
            .map(|bit| bit.parse::<u32>().map_err(display))
            .collect::<Result<Vec<_>, _>>()?
    };
    let initial = Cursor::new(&live).map_err(|why| format!("grammar alphabet: {why:?}"))?;
    Ok(Request {
        vendor,
        symbols,
        from,
        to,
        initial,
        horizon: crate::knobs::horizon_count(horizon).ok_or("HORIZON must be positive")?,
        max_points: positive(points)?,
        programs: positive(programs)?,
        nodes: positive(nodes)?,
        root: Path::new(*root),
    })
}

fn positive(source: &str) -> Result<u64, String> {
    source
        .parse::<u64>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| "positive integer required".into())
}

impl Request<'_> {
    fn campaign<'a>(
        &'a self,
        programs: &'a [runner::expression::Expression],
    ) -> crate::boolean_campaign::Request<'a> {
        crate::boolean_campaign::Request {
            rungs: crate::boolean_campaign::RungScope::ALL,
            vendor: self.vendor,
            symbols: self.symbols,
            from: self.from,
            to: self.to,
            programs,
            horizon: self.horizon,
            max_points: self.max_points,
            output: self.root,
            max_rung_jobs: 8,
        }
    }
}

fn execute(request: &Request<'_>, out: &mut String) -> Result<(), String> {
    execute_with(request, out, crate::boolean_campaign::prepare)
}

type PrepareCampaign = fn(
    &crate::boolean_campaign::Request<'_>,
    &mut String,
) -> Result<crate::boolean_campaign::Prepared, String>;

fn execute_with(
    request: &Request<'_>,
    out: &mut String,
    prepare: PrepareCampaign,
) -> Result<(), String> {
    let strict = crate::audited_range_command::StrictConfig::from_env().map_err(display)?;
    let bytes = strict.max_bytes();
    let budget = Budget {
        programs: request.programs,
        nodes: request.nodes,
        bytes: bytes
            .checked_sub((ENVELOPE + 96) as u64)
            .ok_or("grammar journal byte ceiling is too small")?,
    };
    let first = Batch::prepare(request.initial.clone(), 0, 0, budget)?;
    // An initial positive node allowance emits the first leaf. Preparation
    // binds actual source receipts and resolved runtime policy before pricing.
    let probe = prepare(&request.campaign(first.programs()), out)?;
    let descriptor = probe.descriptor_digest();
    let ancestry_bytes = probe.verification_bytes();
    let mut hash = Hasher::new();
    hash.update(b"brutex/boolean-grammar-campaign/v1\0");
    hash.update(&descriptor);
    hash.update(&request.initial.initial_descriptor());
    hash.update(&request.programs.to_le_bytes());
    hash.update(&request.nodes.to_le_bytes());
    let identity = hash.finalize();
    let _ = writeln!(
        out,
        "\nGRAMMAR CAMPAIGN: {}\nEach invocation admits one bounded binary batch across all eight intraday timeframes. Saved cursor progress resumes exactly. Batch statistics are not a correction over the entire grammar; no live trading or profitability assurance.\n",
        crate::identity_hex(&identity)
    );
    let mut journal = Journal::open(request.root, NAMESPACE, identity)?;
    let latest = journal.latest(bytes)?;
    let state = restore(
        &journal,
        latest.as_ref(),
        &request.initial,
        budget,
        strict.max_records(),
        |id, pin, programs| {
            let expected = prepare(&request.campaign(programs), &mut String::new())?;
            recover_campaign(&expected, descriptor, id, pin, programs, |id, pin| {
                crate::boolean_campaign::verify_complete(request.root, id, pin, ancestry_bytes)
            })
        },
    )?;
    probe.require_current()?;
    if state.exhausted {
        out.push_str("FIXED GRAMMAR EXHAUSTED: every recorded batch has a pinned complete eight-rung campaign. This does not establish global statistical admission or later-period acceptance.\n");
        return Ok(());
    }
    admit_checkpoints(
        journal.next_sequence(),
        state.pending.is_some(),
        strict.max_records(),
        budget.bytes,
    )?;
    let previous = latest
        .as_ref()
        .map_or((0, [0; 32]), |saved| (saved.sequence, saved.seal));
    let (batch, plan) = if let Some((saved, batch)) = state.pending {
        (batch, (saved.sequence, saved.seal))
    } else {
        let batch = Batch::prepare(state.cursor, state.work, state.programs, budget)?;
        let raw = plan_record(previous, &batch)?;
        let pin = journal.publish(&raw, bytes)?;
        (batch, pin)
    };
    // Keep all admitted source/receipt guards through the grammar parent
    // acknowledgment, including empty node-only batches. The priced campaign
    // independently admits the same descriptor before any kernel work.
    let campaign = complete_batch(request, &batch, descriptor, out, prepare)?;
    let raw = done_record(plan, campaign);
    let (sequence, pin) = publish_done(&mut journal, &raw, bytes, || probe.require_current())?;
    let _ = writeln!(
        out,
        "\n{}; {} recorded programs, {} grammar choices. Checkpoint {} / {}. No overall percentage is fabricated for this combinatorial search.\n",
        if batch.exhausted() {
            "FIXED GRAMMAR EXHAUSTED"
        } else {
            "PAUSED AT WORK ALLOWANCE — same request resumes the next batch"
        },
        batch.cumulative_programs()?,
        batch.work(),
        sequence,
        crate::identity_hex(&pin)
    );
    Ok(())
}

fn recover_campaign(
    expected: &crate::boolean_campaign::Prepared,
    descriptor: [u8; 32],
    id: [u8; 32],
    pin: [u8; 32],
    programs: &[runner::expression::Expression],
    mut verify: impl FnMut(
        [u8; 32],
        [u8; 32],
    ) -> Result<crate::boolean_campaign::CampaignReceipt, String>,
) -> Result<(), String> {
    if expected.identity() != id || expected.descriptor_digest() != descriptor {
        return Err("grammar completion differs from freshly admitted exact batch sources".into());
    }
    let receipt = verify(id, pin)?;
    require_campaign_binding(
        (receipt.descriptor_digest(), receipt.program_digest()),
        descriptor,
        programs,
    )?;
    expected.require_current()
}

// These projection checks deliberately remain independent of the identity
// authentication that normally implies them. Keeping the expected fields as
// explicit inputs makes the local contract testable without minting authority.
fn require_campaign_binding(
    actual: ([u8; 32], [u8; 32]),
    descriptor: [u8; 32],
    programs: &[runner::expression::Expression],
) -> Result<(), String> {
    if actual.0 != descriptor || actual.1 != crate::boolean_campaign::program_digest(programs) {
        return Err(
            "grammar completion belongs to different programs or source-policy descriptor".into(),
        );
    }
    Ok(())
}

/// Admission charges reserved ordinals, including crash holes. This is
/// conservative for sparse history and ensures the same configured bounds can
/// read every checkpoint we permit this invocation to append.
fn admit_checkpoints(
    next: u64,
    pending: bool,
    max_records: u64,
    memory_bytes: u64,
) -> Result<(), String> {
    // Discovery also counts the permanent owner.lock directory entry.
    let physical = (crate::search_checkpoint::DIRECTORY_LIMIT as u64).saturating_sub(1);
    let ceiling = max_records.min(memory_bytes / 40).min(physical);
    let needed = if pending { 1 } else { 2 };
    if next == 0
        || next
            .checked_add(needed - 1)
            .is_none_or(|last| last > ceiling)
    {
        return Err(format!(
            "grammar checkpoint reservation refused before pricing: next {next}, needed {needed}, retained ordinal ceiling {ceiling} (including interrupted reservations)"
        ));
    }
    Ok(())
}

fn publish_done(
    journal: &mut Journal,
    raw: &[u8],
    bytes: u64,
    mut require_current: impl FnMut() -> Result<(), String>,
) -> Result<(u64, [u8; 32]), String> {
    require_current()?;
    let (sequence, pin) = journal.publish(raw, bytes)?;
    let saved = journal.read(sequence, bytes)?;
    require_acknowledged(&saved, pin, raw)?;
    // A failed final source check leaves the historical checkpoint intact but
    // refuses this invocation. Restart re-admits the exact sources again.
    require_current()?;
    Ok((sequence, pin))
}

fn require_acknowledged(saved: &Saved, pin: [u8; 32], raw: &[u8]) -> Result<(), String> {
    if saved.seal != pin || saved.payload != raw {
        return Err("grammar completion changed before acknowledgment".into());
    }
    Ok(())
}

fn complete_batch(
    request: &Request<'_>,
    batch: &Batch,
    descriptor: [u8; 32],
    out: &mut String,
    prepare: PrepareCampaign,
) -> Result<([u8; 32], [u8; 32]), String> {
    if batch.programs().is_empty() {
        return Ok(([0; 32], [0; 32]));
    }
    let prepared = prepare(&request.campaign(batch.programs()), out)?;
    if prepared.descriptor_digest() != descriptor {
        return Err(
            "grammar campaign source or resolved runtime policy changed before execution".into(),
        );
    }
    let bytes = prepared.verification_bytes();
    let receipt = crate::boolean_campaign::run_prepared(prepared, out)?;
    receipt.require_complete()?;
    crate::boolean_campaign::verify_complete(
        request.root,
        receipt.identity(),
        receipt.pin(),
        bytes,
    )?;
    Ok((receipt.identity(), receipt.pin()))
}

struct Restored {
    cursor: Cursor,
    work: u64,
    programs: u64,
    exhausted: bool,
    pending: Option<(Saved, Batch)>,
}

/// Cold recovery is explicitly O(checkpoints + replayed grammar work + saved
/// campaign evidence). Each allocation and each replay has a physical ceiling.
fn restore(
    journal: &Journal,
    latest: Option<&Saved>,
    initial: &Cursor,
    budget: Budget,
    max_records: u64,
    mut complete: impl FnMut(
        [u8; 32],
        [u8; 32],
        &[runner::expression::Expression],
    ) -> Result<(), String>,
) -> Result<Restored, String> {
    let max_bytes = budget
        .bytes
        .checked_add((ENVELOPE + 96) as u64)
        .ok_or("grammar journal ceiling overflow")?;
    let chain = discover_chain(journal, latest, max_bytes, budget.bytes, max_records)?;
    let mut state = Restored {
        cursor: initial.clone(),
        work: 0,
        programs: 0,
        exhausted: false,
        pending: None,
    };
    for (sequence, pin) in chain.into_iter().rev() {
        let saved = journal.read(sequence, max_bytes)?;
        if saved.seal != pin || state.exhausted {
            return Err("grammar history changed or continued after exhaustion".into());
        }
        match field::<8>(&saved.payload, 0)? {
            magic if magic == *PLAN => {
                if state.pending.is_some() {
                    return Err("grammar advanced past an unfinished batch".into());
                }
                let batch = Batch::decode(
                    saved
                        .payload
                        .get(ENVELOPE..)
                        .ok_or("grammar batch missing")?,
                    budget.bytes,
                    budget.nodes,
                )?;
                if !batch.follows(&state.cursor, state.work, state.programs)
                    || !batch.uses_budget(budget.programs, budget.nodes)
                {
                    return Err("grammar history skips or repeats cursor work".into());
                }
                state.pending = Some((saved, batch));
            }
            magic if magic == *DONE => {
                let (plan, batch) = state
                    .pending
                    .take()
                    .ok_or("grammar completion has no preceding batch")?;
                require_plan_link(&saved, &plan)?;
                let id = field::<32>(&saved.payload, 48)?;
                let pin = field::<32>(&saved.payload, 80)?;
                if batch.programs().is_empty() {
                    if id != [0; 32] || pin != [0; 32] {
                        return Err("empty grammar work unexpectedly names a campaign".into());
                    }
                } else if id == [0; 32] || pin == [0; 32] {
                    return Err("nonempty grammar batch has no campaign completion".into());
                } else {
                    complete(id, pin, batch.programs())?;
                }
                state.cursor = batch.next_cursor();
                state.work = batch.work();
                state.programs = batch.cumulative_programs()?;
                state.exhausted = batch.exhausted();
            }
            _ => return Err("unknown grammar checkpoint record".into()),
        }
    }
    Ok(state)
}

fn require_plan_link(saved: &Saved, plan: &Saved) -> Result<(), String> {
    if saved.payload.len() != 112
        || u64::from_le_bytes(field(&saved.payload, 8)?) != plan.sequence
        || field::<32>(&saved.payload, 16)? != plan.seal
    {
        return Err("grammar completion refers to a different batch".into());
    }
    Ok(())
}

fn discover_chain(
    journal: &Journal,
    latest: Option<&Saved>,
    max_bytes: u64,
    memory_bytes: u64,
    max_records: u64,
) -> Result<Vec<(u64, [u8; 32])>, String> {
    let mut chain = Vec::new();
    let mut link = latest.map(|saved| (saved.sequence, saved.seal));
    while let Some((sequence, pin)) = link {
        if chain.len() as u64 >= max_records
            || (chain.len() as u64)
                .checked_add(1)
                .and_then(|n| n.checked_mul(40))
                .is_none_or(|n| n > memory_bytes)
        {
            return Err("grammar checkpoint history exceeds its record admission".into());
        }
        let saved = journal.read(sequence, max_bytes)?;
        if saved.seal != pin {
            return Err("grammar checkpoint chain pin mismatch".into());
        }
        if chain.len() == chain.capacity() {
            let ceiling = max_records.min(memory_bytes / 40);
            let target = (chain.len() as u64).saturating_mul(2).max(4).min(ceiling);
            chain
                .try_reserve_exact(
                    usize::try_from(target)
                        .map_err(display)?
                        .saturating_sub(chain.len()),
                )
                .map_err(display)?;
        }
        chain.push((sequence, pin));
        let prior = u64::from_le_bytes(field(&saved.payload, 8)?);
        let seal = field::<32>(&saved.payload, 16)?;
        link = match (prior, seal) {
            (0, seal) if seal == [0; 32] => None,
            (prior, seal) if prior > 0 && prior < sequence && seal != [0; 32] => {
                Some((prior, seal))
            }
            _ => return Err("grammar checkpoint predecessor is invalid".into()),
        };
    }
    Ok(chain)
}

fn plan_record(previous: (u64, [u8; 32]), batch: &Batch) -> Result<Vec<u8>, String> {
    let payload = batch.encode()?;
    let mut raw = Vec::new();
    raw.try_reserve_exact(ENVELOPE + payload.len())
        .map_err(display)?;
    raw.extend_from_slice(PLAN);
    raw.extend_from_slice(&previous.0.to_le_bytes());
    raw.extend_from_slice(&previous.1);
    raw.extend_from_slice(&payload);
    Ok(raw)
}

fn done_record(plan: (u64, [u8; 32]), campaign: ([u8; 32], [u8; 32])) -> Vec<u8> {
    DONE.iter()
        .copied()
        .chain(plan.0.to_le_bytes())
        .chain(plan.1)
        .chain(campaign.0)
        .chain(campaign.1)
        .collect()
}

fn field<const N: usize>(raw: &[u8], at: usize) -> Result<[u8; N], String> {
    raw.get(at..at.saturating_add(N))
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| "grammar checkpoint field missing".into())
}

fn display(why: impl std::fmt::Display) -> String {
    why.to_string()
}

#[cfg(test)]
#[path = "boolean_grammar_campaign_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "boolean_grammar_source_tests.rs"]
mod source_tests;
