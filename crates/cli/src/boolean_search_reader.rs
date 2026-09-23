//! Read-only full-history search observation with separately authenticated details.
use crate::boolean_evidence::{Qualification, QualifiedCampaign};
use crate::boolean_search_projection::{RungReader, summarize};
use crate::boolean_search_record::{NAMESPACE, Record, Spec, Summary, display};
use crate::search_checkpoint::{Saved, Snapshot};
use std::path::{Path, PathBuf};

/// Authenticated acknowledged search history. Overview does not attest child bodies.
pub struct Reader {
    root: PathBuf,
    snapshot: Snapshot,
    identity: [u8; 32],
    history: Vec<Saved>,
    completed: Vec<usize>,
    latest: Record,
    bytes: u64,
    max_bytes: u64,
    max_replay_nodes: u64,
    replay_nodes: u64,
}
impl Reader {
    /// Read every acknowledged predecessor under an aggregate admission.
    /// # Errors
    /// Missing, torn, forked, changed or oversized history refuses.
    pub fn open(
        root: &Path,
        identity: [u8; 32],
        max_bytes: u64,
        max_replay_nodes: u64,
    ) -> Result<Self, String> {
        if max_replay_nodes == 0 {
            return Err("search replay node admission must be positive".into());
        }
        let snapshot =
            Snapshot::open(root, NAMESPACE, identity)?.ok_or("qualified search is not recorded")?;
        let sequence = snapshot
            .latest
            .ok_or("qualified search has no acknowledged reservation")?;
        let mut saved = snapshot.read(sequence, max_bytes)?;
        let mut history = Vec::new();
        let mut charged = 0_u64;
        loop {
            charged = charged
                .checked_add(saved.payload.len() as u64)
                .and_then(|n| n.checked_add((96 + size_of::<Saved>() + size_of::<usize>()) as u64))
                .ok_or("search history byte overflow")?;
            // Keep half the admission for decoded cursor/plan buffers and detail work.
            if charged > max_bytes / 2 {
                return Err("qualified search complete history exceeds byte admission".into());
            }
            let spec = Record::declaration(&saved.payload, max_bytes)?;
            if spec.identity() != identity || history.len() as u64 >= spec.records {
                return Err("search declaration identity or history extent differs".into());
            }
            let previous = Record::predecessor(&saved.payload)?;
            if previous.is_some_and(|(n, _)| n >= saved.sequence) {
                return Err("search predecessor is not strictly earlier".into());
            }
            if history.len() == history.capacity() {
                let ceiling = usize::try_from(
                    spec.records
                        .min(max_bytes / ((size_of::<Saved>() + 96) as u64)),
                )
                .map_err(display)?;
                let desired = history.len().saturating_mul(2).max(1).min(ceiling);
                history
                    .try_reserve_exact(desired.saturating_sub(history.len()))
                    .map_err(display)?;
            }
            history.push(saved);
            match previous {
                None => break,
                Some((n, pin)) => {
                    saved = snapshot.read(n, max_bytes)?;
                    if saved.seal != pin {
                        return Err("search predecessor pin differs".into());
                    }
                }
            }
        }
        if history.len() as u64 != snapshot.acknowledged {
            return Err("search chain omits acknowledged history".into());
        }
        history.reverse();
        let mut completed = Vec::new();
        completed
            .try_reserve_exact(history.len())
            .map_err(display)?;
        let mut previous: Option<Record> = None;
        let mut remaining_nodes = max_replay_nodes;
        for (index, saved) in history.iter().enumerate() {
            let record = Record::decode_bounded(&saved.payload, max_bytes, remaining_nodes)?;
            remaining_nodes = remaining_nodes
                .checked_sub(record.spec.nodes)
                .ok_or("search replay node admission exhausted")?;
            transition(previous.as_ref(), &record)?;
            if record.phase == 1 {
                completed.push(index);
            }
            previous = Some(record);
        }
        let latest = previous.ok_or("search history is empty")?;
        let reader = Self {
            root: root.to_owned(),
            snapshot,
            identity,
            history,
            completed,
            latest,
            bytes: charged,
            max_bytes,
            max_replay_nodes,
            replay_nodes: max_replay_nodes - remaining_nodes,
        };
        reader.require_current()?;
        Ok(reader)
    }
    /// Exact declared search identity, independent of invocation work allowance.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    /// Latest pinned checkpoint, used to refuse mixed-page observations.
    #[must_use]
    pub fn pin(&self) -> [u8; 32] {
        self.history.last().map_or([0; 32], |s| s.seal)
    }
    /// Latest acknowledged checkpoint ordinal, not elapsed-time progress.
    #[must_use]
    pub fn sequence(&self) -> u64 {
        self.history.last().map_or(0, |s| s.sequence)
    }
    /// Zero-based current declared batch, including failed reservations.
    #[must_use]
    pub const fn batch(&self) -> u64 {
        self.latest.ordinal
    }
    /// Saved phase: reserved, complete or refused.
    #[must_use]
    pub const fn phase(&self) -> u64 {
        self.latest.phase
    }
    /// Saved bounded refusal reason.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.latest.reason
    }
    /// Number of completed grammar batches, including explicit node-only batches.
    #[must_use]
    pub fn completed_batches(&self) -> usize {
        self.completed.len()
    }
    /// Actual grammar exhaustion is distinct from a work allowance pause.
    #[must_use]
    pub fn exhausted(&self) -> bool {
        self.latest.phase == 1 && self.latest.batch.exhausted()
    }
    /// Number of grammar node choices acknowledged through the current batch.
    #[must_use]
    pub fn work(&self) -> u64 {
        self.latest.batch.work()
    }
    /// Number of declared grammar programs through the current batch.
    /// # Errors
    /// Refuses a corrupt cumulative count.
    pub fn programs(&self) -> Result<u64, String> {
        self.latest.batch.cumulative_programs()
    }
    /// All latest batch rung summaries; unfinished/empty batches have zero links.
    #[must_use]
    pub const fn summaries(&self) -> &[Summary; 8] {
        &self.latest.summaries
    }
    /// Exact selected timeframes; unselected physical summaries remain empty.
    #[must_use]
    pub const fn rungs(&self) -> crate::boolean_campaign::RungScope {
        self.latest.spec.rungs
    }
    /// Expected campaign address from the acknowledged plan, not proof it has started.
    /// This permits opening its independently authenticated per-timeframe progress.
    /// # Errors
    /// Refuses an invalid retained qualification plan.
    pub fn planned_campaign(&self) -> Result<Option<[u8; 32]>, String> {
        if self.latest.batch.programs().is_empty() {
            return Ok(None);
        }
        let plan = self.latest.observed_plan()?;
        Ok(Some(crate::boolean_qualified_journal::identity_for_rungs(
            plan.descriptor(),
            *plan.units(),
            plan.rungs(),
        )))
    }
    /// Shared initial error budget in parts per million.
    #[must_use]
    pub const fn alpha_ppm(&self) -> u64 {
        self.latest.spec.alpha
    }
    /// Owner lock observed at snapshot time, not a heartbeat/liveness certificate.
    #[must_use]
    pub const fn owner_observed(&self) -> bool {
        self.snapshot.writer_observed
    }
    /// Retained serialized journal/index admission; not total process memory.
    #[must_use]
    pub const fn admitted_bytes(&self) -> u64 {
        self.bytes
    }
    /// Declared node allowances charged by full-history replay, without claiming elapsed CPU time.
    #[must_use]
    pub const fn replay_nodes(&self) -> u64 {
        self.replay_nodes
    }
    pub(crate) fn spec(&self) -> &Spec {
        &self.latest.spec
    }
    /// Arithmetic version bound into this search's immutable declaration.
    #[must_use]
    pub fn projection_version(&self) -> u8 {
        self.latest.spec.projection_rule.version()
    }
    /// Recheck every retained journal record. New appended checkpoints are permitted.
    /// # Errors
    /// Replaced or changed acknowledged records refuse.
    pub fn require_current(&self) -> Result<(), String> {
        for old in &self.history {
            let now = self.snapshot.read(old.sequence, self.max_bytes)?;
            if now.seal != old.seal || now.payload != old.payload {
                return Err("qualified search history changed".into());
            }
        }
        Ok(())
    }
    /// Authenticate a completed batch's entire eight-child campaign and projections.
    /// This is used by restart before it skips completed work.
    /// # Errors
    /// Any lost, foreign or changed child, source-plan or comparison refuses.
    pub fn verify_batch(&self, batch: u64) -> Result<(), String> {
        let record = self.completed_record(batch)?;
        if record.batch.programs().is_empty() {
            return self.require_current();
        }
        let campaign =
            QualifiedCampaign::open(&self.root, record.campaign.identity, self.remaining()?)?;
        let plan = record.observed_plan()?;
        if campaign.pin() != record.campaign.pin
            || campaign.descriptor() != plan.descriptor()
            || campaign.rungs() != plan.rungs()
        {
            return Err("search batch campaign pin or plan differs".into());
        }
        for (rung, ((slot, summary), unit)) in campaign
            .slots()
            .iter()
            .zip(&record.summaries)
            .zip(plan.units())
            .enumerate()
        {
            if !record.spec.rungs.contains(rung) {
                continue;
            }
            if slot.complete != Some(summary.child) || slot.unit != *unit {
                return Err("search batch campaign child mapping differs".into());
            }
            let allowance = self
                .remaining()?
                .checked_sub(campaign.admitted_bytes())
                .ok_or("search child ancestry exceeds remaining admission")?;
            drop(open_rung(&self.root, &record, rung, allowance)?);
        }
        campaign.require_current()?;
        self.require_current()
    }
    /// Open a completed batch/timeframe with its complete saved projection verified.
    /// # Errors
    /// Wrong parent pin, unavailable batch/rung or inconsistent linked evidence refuses.
    pub fn rung(&self, pin: [u8; 32], batch: u64, rung: usize) -> Result<RungReader<'_>, String> {
        if rung >= 8 {
            return Err("search timeframe outside declared eight".into());
        }
        if !self.rungs().contains(rung) {
            return Err("requested timeframe is outside this saved search selection".into());
        }
        if pin != self.pin() {
            return Err("search checkpoint pin differs; no replacement page returned".into());
        }
        let record = self.completed_record(batch)?;
        let campaign =
            QualifiedCampaign::open(&self.root, record.campaign.identity, self.remaining()?)?;
        let plan = record.observed_plan()?;
        if campaign.pin() != record.campaign.pin
            || campaign.descriptor() != plan.descriptor()
            || campaign.rungs() != plan.rungs()
            || campaign
                .slots()
                .iter()
                .zip(&record.summaries)
                .zip(plan.units())
                .enumerate()
                .any(|(index, ((slot, summary), unit))| {
                    slot.unit != *unit
                        || if record.spec.rungs.contains(index) {
                            slot.complete != Some(summary.child)
                        } else {
                            slot.complete.is_some()
                        }
                })
        {
            return Err("search detail campaign or declared child mapping differs".into());
        }
        let allowance = self
            .remaining()?
            .checked_sub(campaign.admitted_bytes())
            .ok_or("search detail ancestry admission exhausted")?;
        let mut result = open_rung(&self.root, &record, rung, allowance)?;
        campaign.require_current()?;
        self.require_current()?;
        result.parent = Some(self);
        result.campaign = Some(campaign);
        Ok(result)
    }
    fn remaining(&self) -> Result<u64, String> {
        self.max_bytes
            .checked_sub(self.bytes)
            .ok_or_else(|| "search observation admission exhausted".into())
    }
    fn completed_record(&self, batch: u64) -> Result<Record, String> {
        let position = *self
            .completed
            .get(usize::try_from(batch).map_err(display)?)
            .ok_or("search batch is not completed")?;
        let saved = self
            .history
            .get(position)
            .ok_or("search completed batch index absent")?;
        let record = Record::decode_bounded(
            &saved.payload,
            self.max_bytes,
            self.max_replay_nodes
                .checked_sub(self.replay_nodes)
                .ok_or("search detail replay admission exhausted")?,
        )?;
        if record.ordinal != batch || record.phase != 1 {
            return Err("search completed batch mapping differs".into());
        }
        Ok(record)
    }
}

pub(crate) fn open_rung<'a>(
    root: &Path,
    record: &Record,
    rung: usize,
    bytes: u64,
) -> Result<RungReader<'a>, String> {
    if rung >= 8 {
        return Err("search timeframe outside declared eight".into());
    }
    if !record.spec.rungs.contains(rung) {
        return Err("requested timeframe is outside this saved search selection".into());
    }
    let summary = *record
        .summaries
        .get(rung)
        .ok_or("search timeframe outside declared eight")?;
    if record.phase != 1 || record.batch.programs().is_empty() {
        return Err("search batch has no completed qualification rows".into());
    }
    let source = Qualification::open(root, summary.child.identity, bytes)?;
    let plan = record.observed_plan()?;
    let allocation =
        runner::search_allocation_v1::allocate(record.ordinal, rung as u64, record.spec.alpha)
            .map_err(super::boolean_search_record::debug)?;
    if source.completion_digest() != summary.child.pin
        || source.scope() != plan.descriptor()
        || Some(&source.unit()) != plan.units().get(rung)
        || source.allocation() != plan.allocation(rung)?
        || summarize(&source, allocation, record.spec.projection_rule)? != summary
    {
        return Err("search complete projection, allocation or child plan differs".into());
    }
    Ok(RungReader {
        parent: None,
        campaign: None,
        batch_replay_nodes: record.spec.nodes,
        source,
        allocation,
        summary,
        projection_rule: record.spec.projection_rule,
    })
}

#[cfg(test)]
#[path = "boolean_search_reader_tests.rs"]
mod tests;

pub(crate) fn transition(before: Option<&Record>, after: &Record) -> Result<(), String> {
    after.validate()?;
    match before {
        None => {
            if after.previous.is_some()
                || after.ordinal != 0
                || after.phase != 0
                || !after.batch.follows(&after.spec.cursor()?, 0, 0)
            {
                return Err("search first record is not its initial pre-work reservation".into());
            }
        }
        Some(before) => {
            if before.spec != after.spec {
                return Err("search changed its immutable declaration".into());
            }
            if before.phase == 1 {
                if before.batch.exhausted()
                    || after.phase != 0
                    || before.ordinal.checked_add(1) != Some(after.ordinal)
                    || !after.batch.follows(
                        &before.batch.next_cursor(),
                        before.batch.work(),
                        before.batch.cumulative_programs()?,
                    )
                {
                    return Err(
                        "search skipped, repeated or continued exhausted grammar work".into(),
                    );
                }
            } else if after.ordinal != before.ordinal
                || after.binding()? != before.binding()?
                || after.phase == 0
            {
                return Err("search reassigned an unfinished reservation".into());
            }
        }
    }
    Ok(())
}
