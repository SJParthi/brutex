//! Immutable original-first single-stop grammar transitions. A partial write
//! never advances the cursor; acknowledged predecessors are verified by pin.
use super::{
    AdmissionPolicyV1, Batch, Cursor, Journal, Path, PopulationStatisticsProcedureV2,
    PreparedSources, Request, allocation_for, debug, display, hash, month_days, qualification,
};
use crate::search_checkpoint::Saved;
use vocab::expression_search::CURSOR_BYTES;

#[cfg(test)]
use super::{Budget, NAMESPACE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Link {
    pub identity: [u8; 32],
    pub pin: [u8; 32],
}

#[derive(Clone, Copy)]
pub(super) struct ReadBudget {
    pub bytes: u64,
    pub records: u64,
    pub nodes: u64,
}

pub(super) struct Verification {
    sources: [Option<(qualification::ExpectedSource, qualification::ExpectedSource)>; 8],
    policy: AdmissionPolicyV1,
    procedure: PopulationStatisticsProcedureV2,
    bounds: qualification::Bounds,
    replay: qualification::ReplayBounds,
}
impl Verification {
    pub(super) fn new(
        request: &Request<'_>,
        prepared: &[PreparedSources<'_>],
    ) -> Result<Self, String> {
        let mut sources = [None; 8];
        let training = month_days(request.training)?;
        let later = month_days(request.later)?;
        for source in prepared {
            *sources
                .get_mut(source.sources.rung)
                .ok_or("single-stop checkpoint source rung")? = Some((
                qualification::ExpectedSource {
                    identity: source.training.source_id(),
                    first_day: training.0,
                    last_day: training.1,
                },
                qualification::ExpectedSource {
                    identity: source.later.source_id(),
                    first_day: later.0,
                    last_day: later.1,
                },
            ));
        }
        let config = request.configuration;
        Ok(Self {
            sources,
            policy: config.policy,
            procedure: config.procedure,
            bounds: config.qualification,
            replay: qualification::ReplayBounds {
                bootstrap_work: config.replay_nodes,
                split_work: config.replay_nodes,
                memory_bytes: config.history_bytes,
            },
        })
    }
    fn slot<'a>(
        &self,
        search: [u8; 32],
        batch: u64,
        rung: usize,
        programs: &'a [runner::expression::Expression],
    ) -> Result<qualification::ExpectedSearchSlot<'a>, String> {
        let (training, later) = self
            .sources
            .get(rung)
            .copied()
            .flatten()
            .ok_or("single-stop checkpoint has no declared source for its timeframe")?;
        let allocation = allocation_for(&self.policy, self.procedure, batch, rung)?;
        Ok(qualification::ExpectedSearchSlot {
            search,
            training,
            later,
            policy: self
                .policy
                .with_search_probability_ceiling(allocation.alpha_ppm()),
            procedure: self.procedure,
            allocation,
            bounds: self.bounds,
            programs,
        })
    }
}

pub(super) struct Frame {
    pub(super) declaration: Vec<u8>,
    pub(super) scope: u8,
    pub(super) previous: Option<(u64, [u8; 32])>,
    pub location: Option<(u64, [u8; 32])>,
    pub pending: bool,
    pub completed_batches: u64,
    pub cursor: Cursor,
    pub work: u64,
    pub programs: u64,
    pub exhausted: bool,
    pub batch: Option<Batch>,
    pub links: [Option<Link>; 8],
    /// Reconstructed from acknowledged ancestors; never encoded as a second authority.
    pub latest_saved: Option<(u64, [Option<Link>; 8])>,
    retained_bytes: u64,
    replayed_nodes: u64,
}
impl Frame {
    pub(super) fn declaration(declaration: Vec<u8>, cursor: Cursor, scope: u8) -> Self {
        Self {
            declaration,
            scope,
            previous: None,
            location: None,
            pending: false,
            completed_batches: 0,
            cursor,
            work: 0,
            programs: 0,
            exhausted: false,
            batch: None,
            links: [None; 8],
            latest_saved: None,
            retained_bytes: 0,
            replayed_nodes: 0,
        }
    }
    pub(super) fn pending(previous: &Self, batch: Batch) -> Result<Self, String> {
        if previous.pending
            || previous.exhausted
            || previous.location.is_none()
            || !batch.follows(&previous.cursor, previous.work, previous.programs)
        {
            return Err(
                "single-stop grammar reservation does not follow its acknowledged cursor".into(),
            );
        }
        Ok(Self {
            declaration: previous.declaration.clone(),
            scope: previous.scope,
            previous: previous.location,
            location: None,
            pending: true,
            completed_batches: previous.completed_batches,
            cursor: previous.cursor.clone(),
            work: previous.work,
            programs: previous.programs,
            exhausted: false,
            batch: Some(batch),
            links: [None; 8],
            retained_bytes: previous.retained_bytes,
            replayed_nodes: previous.replayed_nodes,
            latest_saved: previous.latest_saved,
        })
    }
    pub(super) fn done(previous: Self, links: &[Option<Link>; 8]) -> Result<Self, String> {
        if !previous.pending || previous.location.is_none() {
            return Err("single-stop completion lacks its acknowledged reservation".into());
        }
        let batch = previous
            .batch
            .ok_or("single-stop pending grammar body missing")?;
        let has_programs = !batch.programs().is_empty();
        for (rung, link) in links.iter().enumerate() {
            if link.is_some() != (has_programs && (previous.scope & (1 << rung)) != 0)
                || link.is_some_and(|value| value.identity == [0; 32] || value.pin == [0; 32])
            {
                return Err(
                    "single-stop completion does not cover exactly its selected timeframes".into(),
                );
            }
        }
        let latest_saved = if has_programs {
            Some((previous.completed_batches, *links))
        } else {
            previous.latest_saved
        };
        Ok(Self {
            declaration: previous.declaration,
            scope: previous.scope,
            previous: previous.location,
            location: None,
            pending: false,
            completed_batches: previous
                .completed_batches
                .checked_add(1)
                .ok_or("single-stop batch counter overflow")?,
            cursor: batch.next_cursor(),
            work: batch.work(),
            programs: batch.cumulative_programs()?,
            exhausted: batch.exhausted(),
            batch: Some(batch),
            links: *links,
            retained_bytes: previous.retained_bytes,
            replayed_nodes: previous.replayed_nodes,
            latest_saved,
        })
    }
    pub(super) fn publish(
        &mut self,
        journal: &mut Journal,
        body_bytes: u64,
        budget: ReadBudget,
        node_allowance: u64,
    ) -> Result<(), String> {
        if journal.acknowledged() >= budget.records {
            return Err("single-stop checkpoint would exceed cold history record admission; no acknowledgment written".into());
        }
        let body = self.encode()?;
        let retained = self.retained_bytes.checked_add(u64::try_from(body.len()).map_err(display)?)
            .and_then(|n| n.checked_add(96)).filter(|n| *n <= budget.bytes)
            .ok_or("single-stop checkpoint history would exceed observation admission; increase its read budget to resume the same search")?;
        let replayed = self.replayed_nodes.checked_add(if self.batch.is_some() { node_allowance } else { 0 })
            .filter(|n| *n <= budget.nodes)
            .ok_or("single-stop checkpoint would exceed cold replay admission; increase its read budget to resume the same search")?;
        self.location = Some(journal.publish(&body, body_bytes)?);
        self.retained_bytes = retained;
        self.replayed_nodes = replayed;
        Ok(())
    }
    pub(super) fn encode(&self) -> Result<Vec<u8>, String> {
        let batch = self
            .batch
            .as_ref()
            .map(Batch::encode)
            .transpose()?
            .unwrap_or_default();
        let mut out = Vec::new();
        let size = 640_usize
            .checked_add(CURSOR_BYTES)
            .and_then(|n| n.checked_add(self.declaration.len()))
            .and_then(|n| n.checked_add(batch.len()))
            .ok_or("single-stop checkpoint byte overflow")?;
        out.try_reserve_exact(size).map_err(display)?;
        out.extend_from_slice(b"BRISCP01");
        out.extend_from_slice(&[
            u8::from(self.pending),
            u8::from(self.exhausted),
            self.scope,
            u8::from(self.previous.is_some()),
            0,
            0,
            0,
            0,
        ]);
        let (sequence, pin) = self.previous.unwrap_or((0, [0; 32]));
        word(&mut out, sequence);
        out.extend_from_slice(&pin);
        for value in [
            self.completed_batches,
            self.work,
            self.programs,
            u64::try_from(self.declaration.len()).map_err(display)?,
            u64::try_from(batch.len()).map_err(display)?,
        ] {
            word(&mut out, value);
        }
        out.extend_from_slice(&self.cursor.encode());
        for link in self.links {
            let link = link.unwrap_or(Link {
                identity: [0; 32],
                pin: [0; 32],
            });
            out.extend_from_slice(&link.identity);
            out.extend_from_slice(&link.pin);
        }
        out.extend_from_slice(&self.declaration);
        out.extend_from_slice(&batch);
        Ok(out)
    }
    pub(super) fn decode(
        saved: &Saved,
        declaration: &[u8],
        bytes: u64,
        nodes: u64,
        scope: u8,
    ) -> Result<Self, String> {
        if u64::try_from(saved.payload.len()).map_err(display)? > bytes {
            return Err("single-stop checkpoint byte admission refused".into());
        }
        let mut raw = Decode {
            raw: &saved.payload,
            at: 0,
        };
        if raw.array::<8>()? != *b"BRISCP01" {
            return Err("single-stop checkpoint format differs".into());
        }
        let flags = raw.array::<8>()?;
        if flags[0] > 1
            || flags[1] > 1
            || flags[2] != scope
            || scope == 0
            || flags[3] > 1
            || flags[4..] != [0; 4]
        {
            return Err("single-stop checkpoint scope or flags refused".into());
        }
        let sequence = raw.word()?;
        let pin = raw.array()?;
        let previous = if flags[3] == 1 {
            if sequence >= saved.sequence || pin == [0; 32] {
                return Err("single-stop checkpoint predecessor is not earlier and pinned".into());
            }
            Some((sequence, pin))
        } else {
            if sequence != 0 || pin != [0; 32] {
                return Err("single-stop absent predecessor carries hidden values".into());
            }
            None
        };
        let completed_batches = raw.word()?;
        let work = raw.word()?;
        let programs = raw.word()?;
        let declaration_bytes = raw.word()?;
        let batch_bytes = raw.word()?;
        let cursor = Cursor::decode(&raw.array::<CURSOR_BYTES>()?).map_err(debug)?;
        let mut links = [None; 8];
        for slot in &mut links {
            let identity = raw.array()?;
            let pin = raw.array()?;
            if (identity == [0; 32]) != (pin == [0; 32]) {
                return Err("single-stop child has only half an identity/pin pair".into());
            }
            *slot = (identity != [0; 32]).then_some(Link { identity, pin });
        }
        if raw.take(declaration_bytes)? != declaration {
            return Err("single-stop source/search declaration changed".into());
        }
        let batch = if batch_bytes == 0 {
            None
        } else {
            Some(Batch::decode(raw.take(batch_bytes)?, bytes, nodes)?)
        };
        if raw.at != raw.raw.len() {
            return Err("single-stop checkpoint has trailing bytes".into());
        }
        Ok(Self {
            declaration: declaration.to_vec(),
            scope,
            previous,
            location: Some((saved.sequence, saved.seal)),
            pending: flags[0] == 1,
            exhausted: flags[1] == 1,
            completed_batches,
            cursor,
            work,
            programs,
            batch,
            links,
            retained_bytes: 0,
            replayed_nodes: 0,
            latest_saved: None,
        })
    }
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one ordered recovery verifies the complete journal chain and independent declared slot facts before retaining a checkpoint"
)]
pub(super) fn recover(
    journal: &Journal,
    declaration: &[u8],
    root: &Path,
    bytes: u64,
    records: u64,
    nodes: u64,
    scope: u8,
    initial: &Cursor,
    batch_programs: u64,
    node_allowance: u64,
    verification: &Verification,
) -> Result<Option<Frame>, String> {
    if verification
        .sources
        .iter()
        .enumerate()
        .any(|(rung, source)| source.is_some() != (scope & (1 << rung) != 0))
    {
        return Err(
            "single-stop checkpoint source verification differs from declared timeframe scope"
                .into(),
        );
    }
    let Some(mut saved) = journal.latest(bytes)? else {
        return Ok(None);
    };
    let mut history = Vec::new();
    let mut charged = 0_u64;
    let mut remaining_nodes = nodes;
    loop {
        charged = charged
            .checked_add(u64::try_from(saved.payload.len()).map_err(display)?)
            .and_then(|n| n.checked_add(96))
            .filter(|n| *n <= bytes)
            .ok_or("single-stop full checkpoint history exceeds cold byte admission")?;
        if u64::try_from(history.len()).map_err(display)? >= records {
            return Err("single-stop acknowledged history exceeds cold record admission".into());
        }
        let frame = Frame::decode(&saved, declaration, bytes, remaining_nodes, scope)?;
        if frame.batch.is_some() {
            if !frame
                .batch
                .as_ref()
                .is_some_and(|batch| batch.uses_budget(batch_programs, node_allowance))
            {
                return Err(
                    "single-stop recovered batch uses different declared work limits".into(),
                );
            }
            remaining_nodes = remaining_nodes
                .checked_sub(node_allowance)
                .ok_or("single-stop cumulative cold grammar replay exceeds node admission")?;
        } else if frame.cursor.encode() != initial.encode() {
            return Err("single-stop initial cursor differs from its declared alphabet".into());
        }
        let prior = frame.previous;
        history.push(frame);
        let Some((sequence, pin)) = prior else {
            break;
        };
        saved = journal.read(sequence, bytes)?;
        if saved.seal != pin {
            return Err("single-stop acknowledged predecessor pin changed".into());
        }
    }
    if u64::try_from(history.len()).map_err(display)? != journal.acknowledged() {
        return Err("single-stop chain omitted acknowledged checkpoints".into());
    }
    history.reverse();
    let mut previous: Option<Frame> = None;
    for mut frame in history {
        transition(previous.as_ref(), &frame)?;
        if !frame.pending {
            for (rung, link) in frame.links.iter().enumerate() {
                if let Some(link) = link {
                    let batch = frame
                        .batch
                        .as_ref()
                        .ok_or("single-stop completed child without grammar")?;
                    let ordinal = frame
                        .completed_batches
                        .checked_sub(1)
                        .ok_or("single-stop completed ordinal")?;
                    let expected =
                        verification.slot(hash(declaration), ordinal, rung, batch.programs())?;
                    qualification::verify_search_slot_bounded(
                        root,
                        link.identity,
                        link.pin,
                        bytes,
                        records,
                        &expected,
                        verification.replay,
                    )?;
                }
            }
        }
        frame.latest_saved = if frame.links.iter().any(Option::is_some) {
            Some((
                frame
                    .completed_batches
                    .checked_sub(1)
                    .ok_or("single-stop saved batch ordinal")?,
                frame.links,
            ))
        } else {
            previous.as_ref().and_then(|value| value.latest_saved)
        };
        previous = Some(frame);
    }
    if let Some(frame) = previous.as_mut() {
        frame.retained_bytes = charged;
        frame.replayed_nodes = nodes - remaining_nodes;
    }
    Ok(previous)
}
pub(super) fn transition(prior: Option<&Frame>, next: &Frame) -> Result<(), String> {
    let Some(prior) = prior else {
        if next.previous.is_some()
            || next.pending
            || next.exhausted
            || next.completed_batches != 0
            || next.work != 0
            || next.programs != 0
            || next.batch.is_some()
            || next.links.iter().any(Option::is_some)
            || next.cursor.initial_descriptor() != next.cursor.encode()
        {
            return Err("single-stop first checkpoint is not an empty initial declaration".into());
        }
        return Ok(());
    };
    if next.previous != prior.location
        || next.declaration != prior.declaration
        || next.scope != prior.scope
        || prior.exhausted
    {
        return Err("single-stop checkpoint ancestry or terminal transition differs".into());
    }
    let batch = next
        .batch
        .as_ref()
        .ok_or("single-stop continuation omitted its grammar body")?;
    if next.pending {
        if prior.pending
            || next.completed_batches != prior.completed_batches
            || next.work != prior.work
            || next.programs != prior.programs
            || next.exhausted
            || next.cursor.encode() != prior.cursor.encode()
            || next.links.iter().any(Option::is_some)
            || !batch.follows(&prior.cursor, prior.work, prior.programs)
        {
            return Err(
                "single-stop reservation changed or skipped its acknowledged cursor".into(),
            );
        }
    } else {
        let original = prior
            .batch
            .as_ref()
            .ok_or("single-stop completion has no previous batch")?;
        if !prior.pending
            || batch.encode()? != original.encode()?
            || Some(next.completed_batches) != prior.completed_batches.checked_add(1)
            || next.work != batch.work()
            || next.programs != batch.cumulative_programs()?
            || next.cursor.encode() != batch.next_cursor().encode()
            || next.exhausted != batch.exhausted()
        {
            return Err("single-stop completion changed the reserved programs or counters".into());
        }
        for (rung, link) in next.links.iter().enumerate() {
            if link.is_some() != (!batch.programs().is_empty() && next.scope & (1 << rung) != 0) {
                return Err(
                    "single-stop completion has missing or foreign timeframe evidence".into(),
                );
            }
        }
    }
    Ok(())
}
fn word(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}
struct Decode<'a> {
    raw: &'a [u8],
    at: usize,
}
impl<'a> Decode<'a> {
    fn take(&mut self, bytes: u64) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(usize::try_from(bytes).map_err(display)?)
            .ok_or("single-stop checkpoint offset overflow")?;
        let value = self
            .raw
            .get(self.at..end)
            .ok_or("single-stop checkpoint body is truncated")?;
        self.at = end;
        Ok(value)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        self.take(u64::try_from(N).map_err(display)?)?
            .try_into()
            .map_err(|_| "single-stop checkpoint field width differs".into())
    }
    fn word(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.array()?))
    }
}

#[cfg(test)]
#[path = "index_stop_search_checkpoint_tests.rs"]
mod tests;
