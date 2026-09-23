//! Read-only acknowledged search prefixes. Cold work grows with the admitted
//! checkpoint chain and full selected-timeframe families; no winner-only read
//! is allowed to stand in for that complete scope.
use super::{
    AdmissionPolicyV1, Cursor, NAMESPACE, PopulationStatisticsProcedureV2, allocation_for,
    checkpoint, debug, display, hash, qualification,
};
use crate::candidate_universe::boolean_candidate_v1::persistence::Observation;
use crate::search_checkpoint::{Saved, Snapshot};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use vocab::expression_search::CURSOR_BYTES;

/// Exact immutable artifact identity and completion pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    /// Content-addressed artifact identity.
    pub identity: [u8; 32],
    /// Acknowledged completion digest.
    pub completion: [u8; 32],
}
/// Native prepared source and inclusive measurement bounds.
pub type Source = qualification::ExpectedSource;

/// Canonically decoded declaration; old hash-only policies stay unavailable
/// until an independently verified child proves exact, untightened equality.
pub struct Declaration {
    version: u8,
    identity: [u8; 32],
    index: String,
    feed: String,
    scope: u8,
    training: (i64, i64),
    later: (i64, i64),
    initial: Cursor,
    policy_digest: [u8; 32],
    policy: Option<AdmissionPolicyV1>,
    batch_programs: u64,
    node_allowance: u64,
    procedure: PopulationStatisticsProcedureV2,
    bounds: qualification::Bounds,
    sources: [Option<(Source, Source)>; 8],
    contexts: [Option<(Link, Link)>; 8],
}
impl Declaration {
    /// Decode exactly one V1 body or V2 provenance envelope, without defaults.
    /// # Errors
    /// Refuses unknown versions, malformed fields, changed policies and tails.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let mut outer = Decode::new(bytes);
        match outer.take(8)? {
            b"BRISSD01" => Self::legacy(bytes),
            b"BRISSD02" => {
                let length = outer.word()?;
                let mut value = Self::legacy(outer.take_word(length)?)?;
                value.version = 2;
                value.identity = hash(bytes);
                let policy = AdmissionPolicyV1::from_canonical_bytes(
                    outer.take(runner::admission::ADMISSION_POLICY_CANONICAL_LEN_V1)?,
                )
                .map_err(debug)?;
                if policy.digest() != value.policy_digest {
                    return Err("search declaration base policy differs from its digest".into());
                }
                value.policy = Some(policy);
                for (rung, slot) in value.contexts.iter_mut().enumerate() {
                    if value.scope & (1 << rung) != 0 {
                        *slot = Some((outer.link()?, outer.link()?));
                    }
                }
                outer.finish()?;
                Ok(value)
            }
            _ => Err("single-stop search declaration version is unsupported".into()),
        }
    }
    fn legacy(bytes: &[u8]) -> Result<Self, String> {
        let mut raw = Decode::new(bytes);
        if raw.take(8)? != b"BRISSD01" {
            return Err("single-stop inner declaration must be V1".into());
        }
        let index = raw.text()?;
        if !matches!(index.as_str(), "NSE-NIFTY" | "NSE-BANKNIFTY") {
            return Err("single-stop declaration index is outside its research surface".into());
        }
        let feed = raw.text()?;
        crate::parse_vendor(&feed)?;
        let scope = raw.array::<1>()?[0];
        let training = (raw.day()?, raw.day()?);
        let later = (raw.day()?, raw.day()?);
        if scope == 0 || training.0 > training.1 || later.0 > later.1 || training.1 >= later.0 {
            return Err("single-stop declaration timeframe or chronological bounds refused".into());
        }
        let initial = Cursor::decode(&raw.array::<CURSOR_BYTES>()?).map_err(debug)?;
        if initial.initial_descriptor() != initial.encode() {
            return Err("search declaration cursor is not an initial alphabet descriptor".into());
        }
        let policy_digest = raw.array()?;
        if policy_digest == [0; 32]
            || raw.array::<32>()? != runner::signal_candle_stop::Policy::V1.digest()
            || raw.array::<32>()? != crate::index_consistency::INDEX_STOP.digest()
        {
            return Err("single-stop declared execution or daily policy differs".into());
        }
        let mut values = [0; 15];
        for value in &mut values {
            *value = raw.word()?;
        }
        if values
            .iter()
            .enumerate()
            .any(|(n, value)| n != 6 && *value == 0)
            || values[0] > values[2]
            || values[1] > values[3]
        {
            return Err("single-stop declared numerical or physical bounds refused".into());
        }
        let procedure = PopulationStatisticsProcedureV2::new(values[5], values[6], values[7])?;
        let bounds = qualification::Bounds {
            candidates: values[8],
            bootstrap_work: values[9],
            split_work: values[10],
            memory_bytes: values[11],
            bytes: values[12],
        };
        let mut sources = [None; 8];
        for (rung, slot) in sources.iter_mut().enumerate() {
            if scope & (1 << rung) != 0 {
                let before = raw.array()?;
                let after = raw.array()?;
                if before == [0; 32] || after == [0; 32] || before == after {
                    return Err("single-stop declaration source identities refused".into());
                }
                *slot = Some((
                    Source {
                        identity: before,
                        first_day: training.0,
                        last_day: training.1,
                    },
                    Source {
                        identity: after,
                        first_day: later.0,
                        last_day: later.1,
                    },
                ));
            }
        }
        raw.finish()?;
        Ok(Self {
            version: 1,
            identity: hash(bytes),
            index,
            feed,
            scope,
            training,
            later,
            initial,
            policy_digest,
            policy: None,
            batch_programs: values[0],
            node_allowance: values[1],
            procedure,
            bounds,
            sources,
            contexts: [None; 8],
        })
    }
    /// Declaration format version, never inferred from current configuration.
    #[must_use]
    pub const fn version(&self) -> u8 {
        self.version
    }
    /// Hash of the exact outer declaration bytes.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    /// Declared research index.
    #[must_use]
    pub fn index(&self) -> &str {
        &self.index
    }
    /// Declared stored feed.
    #[must_use]
    pub fn feed(&self) -> &str {
        &self.feed
    }
    /// Physical selected-timeframe bit mask.
    #[must_use]
    pub const fn scope(&self) -> u8 {
        self.scope
    }
    /// Inclusive training output days.
    #[must_use]
    pub const fn training_days(&self) -> (i64, i64) {
        self.training
    }
    /// Inclusive later output days.
    #[must_use]
    pub const fn later_days(&self) -> (i64, i64) {
        self.later
    }
    /// Exact selected timeframe source and measurement bounds.
    #[must_use]
    pub fn source(&self, rung: usize, later: bool) -> Option<Source> {
        self.sources
            .get(rung)
            .copied()
            .flatten()
            .map(|pair| if later { pair.1 } else { pair.0 })
    }
    /// Exact source companion link, absent in legacy declarations.
    #[must_use]
    pub fn source_context(&self, rung: usize, later: bool) -> Option<Link> {
        self.contexts
            .get(rung)
            .copied()
            .flatten()
            .map(|pair| if later { pair.1 } else { pair.0 })
    }
    /// Original pre-allocation policy, present only when explicitly stored.
    #[must_use]
    pub const fn policy(&self) -> Option<AdmissionPolicyV1> {
        self.policy
    }
    fn expected<'a>(
        &self,
        policy: &AdmissionPolicyV1,
        batch: u64,
        rung: usize,
        programs: &'a [runner::expression::Expression],
    ) -> Result<qualification::ExpectedSearchSlot<'a>, String> {
        let allocation = allocation_for(policy, self.procedure, batch, rung)?;
        Ok(qualification::ExpectedSearchSlot {
            search: self.identity,
            training: self
                .source(rung, false)
                .ok_or("declared training source absent")?,
            later: self
                .source(rung, true)
                .ok_or("declared later source absent")?,
            policy: policy.with_search_probability_ceiling(allocation.alpha_ppm()),
            procedure: self.procedure,
            allocation,
            bounds: self.bounds,
            programs,
        })
    }
}

struct Decode<'a> {
    raw: &'a [u8],
    at: usize,
}
impl<'a> Decode<'a> {
    const fn new(raw: &'a [u8]) -> Self {
        Self { raw, at: 0 }
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(count)
            .ok_or("declaration length overflow")?;
        let value = self
            .raw
            .get(self.at..end)
            .ok_or("search declaration is truncated")?;
        self.at = end;
        Ok(value)
    }
    fn take_word(&mut self, count: u64) -> Result<&'a [u8], String> {
        self.take(usize::try_from(count).map_err(display)?)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        self.take(N)?.try_into().map_err(display)
    }
    fn word(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn day(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.array()?))
    }
    fn text(&mut self) -> Result<String, String> {
        let length = self.word()?;
        if length > 256 {
            return Err("search declaration text exceeds its bounded vocabulary".into());
        }
        std::str::from_utf8(self.take_word(length)?)
            .map(str::to_owned)
            .map_err(display)
    }
    fn link(&mut self) -> Result<Link, String> {
        let value = Link {
            identity: self.array()?,
            completion: self.array()?,
        };
        if value.identity == [0; 32] || value.completion == [0; 32] {
            return Err("source companion link is not fully pinned".into());
        }
        Ok(value)
    }
    fn finish(self) -> Result<(), String> {
        if self.at == self.raw.len() {
            Ok(())
        } else {
            Err("search declaration has trailing bytes".into())
        }
    }
}

/// Independent whole-prefix read bounds; they never alter stored policies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// Saved evidence, checkpoint, ranking and compound-lease scratch allowance.
    pub bytes: u64,
    /// Aggregate checkpoint plus candidate row allowance.
    pub records: u64,
    /// Cumulative canonical grammar replay allowance.
    pub grammar_nodes: u64,
    /// Cumulative required bootstrap-work allowance over all child families.
    pub bootstrap_work: u64,
    /// Cumulative required training-split allowance over all child families.
    pub split_work: u64,
}
/// One acknowledged selected-timeframe family in its original batch ordinal.
pub struct Family {
    /// Zero-based acknowledged batch ordinal.
    pub batch: u64,
    /// Independently verified original, later and institutional evidence.
    pub reader: qualification::Reader,
}
/// Fully verified immutable prefix for one selected timeframe. The read itself
/// does not create, resume or repair any checkpoint.
pub struct Reader {
    declaration: Declaration,
    sequence: u64,
    pin: [u8; 32],
    completed: u64,
    programs: u64,
    work: u64,
    pending: bool,
    exhausted: bool,
    interrupted: u64,
    writer_observed: bool,
    families: Vec<Family>,
    guards: Vec<Guard>,
    contexts: Vec<crate::index_stop::source_context::ContextBinding>,
    bytes: u64,
    grammar_nodes: u64,
    bootstrap_work: u64,
    split_work: u64,
    projection_scratch: u64,
    projecting: AtomicBool,
}
impl Reader {
    /// UNVERIFIED performance: no named cost test or measured latency bound is established here.
    /// Discover and authenticate the current acknowledged marker without
    /// redoing complete grammar or numerical replay. Directory discovery and
    /// this one payload read are bounded cold work, not a constant-time index.
    /// # Errors
    /// Refuses missing, malformed, unacknowledged or over-budget checkpoints.
    pub fn latest_checkpoint(
        root: &Path,
        identity: [u8; 32],
        max_bytes: u64,
    ) -> Result<(u64, [u8; 32]), String> {
        let snapshot = Snapshot::open(root, NAMESPACE, identity)?
            .ok_or("no saved checkpoint exists for this search")?;
        let sequence = snapshot
            .latest
            .ok_or("search has no acknowledged checkpoint")?;
        let saved = snapshot.read(sequence, max_bytes)?;
        Ok((sequence, saved.seal))
    }
    /// Admit one complete acknowledged prefix. A pin requires both sequence
    /// and digest; an unpinned request discovers the latest saved checkpoint.
    /// # Errors
    /// Refuses foreign ancestry, unavailable original policies, changed files,
    /// incomplete families and cumulative physical or replay admission failures.
    #[expect(
        clippy::too_many_lines,
        reason = "ordered cold admission accounts every retained ancestor and whole family before exposing a cumulative result"
    )]
    pub fn open(
        root: &Path,
        identity: [u8; 32],
        rung: usize,
        pin: Option<(u64, [u8; 32])>,
        budget: Budget,
    ) -> Result<Self, String> {
        if identity == [0; 32]
            || rung >= 8
            || [
                budget.bytes,
                budget.records,
                budget.grammar_nodes,
                budget.bootstrap_work,
                budget.split_work,
            ]
            .contains(&0)
        {
            return Err(
                "cumulative ranking requires an exact identity, timeframe and positive read bounds"
                    .into(),
            );
        }
        let snapshot = match pin {
            Some((sequence, digest)) if digest != [0; 32] => {
                Snapshot::open_prefix(root, NAMESPACE, identity, sequence)?
            }
            Some(_) => return Err("cumulative checkpoint completion pin is empty".into()),
            None => Snapshot::open(root, NAMESPACE, identity)?,
        }
        .ok_or("no saved checkpoint exists for this search")?;
        let sequence = snapshot
            .latest
            .ok_or("search has no acknowledged checkpoint")?;
        if pin.is_some_and(|value| value.0 != sequence) {
            return Err("requested checkpoint is not acknowledged".into());
        }
        let mut guards = Vec::new();
        let mut saved = guarded_read(
            root,
            identity,
            &snapshot,
            sequence,
            budget.bytes,
            &mut guards,
        )?;
        if pin.is_some_and(|value| value.1 != saved.seal) {
            return Err("requested checkpoint completion pin differs".into());
        }
        let seal = saved.seal;
        let declaration = Declaration::decode(declaration_bytes(&saved)?)?;
        let raw = declaration_bytes(&saved)?.to_vec();
        if declaration.identity != identity || declaration.scope & (1 << rung) == 0 {
            return Err("saved declaration identity or selected timeframe differs".into());
        }
        let mut history = Vec::new();
        let (mut charged, mut nodes) = (0_u64, 0_u64);
        loop {
            charge(
                &mut charged,
                u64::try_from(saved.payload.len())
                    .map_err(display)?
                    .checked_add(96)
                    .ok_or("checkpoint byte extent overflow")?,
                budget.bytes,
                "cumulative checkpoint bytes",
            )?;
            if u64::try_from(history.len()).map_err(display)? >= budget.records {
                return Err("cumulative checkpoint record admission exceeded".into());
            }
            let frame = checkpoint::Frame::decode(
                &saved,
                &raw,
                budget.bytes,
                budget.grammar_nodes - nodes,
                declaration.scope,
            )?;
            if let Some(batch) = &frame.batch {
                if !batch.uses_budget(declaration.batch_programs, declaration.node_allowance) {
                    return Err("saved batch differs from declared syntax work allowance".into());
                }
                charge(
                    &mut nodes,
                    declaration.node_allowance,
                    budget.grammar_nodes,
                    "cumulative grammar replay",
                )?;
            } else if frame.cursor.encode() != declaration.initial.encode() {
                return Err("saved initial cursor differs from declared alphabet".into());
            }
            let previous = frame.previous;
            history.try_reserve(1).map_err(display)?;
            history.push(frame);
            let Some((sequence, pin)) = previous else {
                break;
            };
            saved = guarded_read(
                root,
                identity,
                &snapshot,
                sequence,
                budget.bytes,
                &mut guards,
            )?;
            if saved.seal != pin {
                return Err("saved checkpoint ancestor pin changed".into());
            }
        }
        if u64::try_from(history.len()).map_err(display)? != snapshot.acknowledged {
            return Err("saved chain omitted acknowledged checkpoints".into());
        }
        history.reverse();
        let mut previous = None;
        for frame in &history {
            checkpoint::transition(previous, frame)?;
            previous = Some(frame);
        }
        let latest = history.last().ok_or("saved checkpoint chain is empty")?;
        let (completed, programs, work, pending, exhausted) = (
            latest.completed_batches,
            latest.programs,
            latest.work,
            latest.pending,
            latest.exhausted,
        );
        let mut families = Vec::new();
        let mut contexts = Vec::new();
        let (mut bootstrap, mut splits) = (0_u64, 0_u64);
        let mut records = u64::try_from(history.len()).map_err(display)?;
        let mut base_policy = declaration.policy;
        for frame in &history {
            if frame.pending {
                continue;
            }
            let Some(link) = frame.links.get(rung).copied().flatten() else {
                continue;
            };
            let batch = frame
                .completed_batches
                .checked_sub(1)
                .ok_or("completed child batch ordinal missing")?;
            let grammar = frame
                .batch
                .as_ref()
                .ok_or("completed child lacks its grammar reservation")?;
            let count = u64::try_from(grammar.programs().len())
                .map_err(display)?
                .checked_mul(2)
                .ok_or("candidate count overflow")?;
            charge(
                &mut records,
                count,
                budget.records,
                "cumulative candidate records",
            )?;
            let replay = qualification::ReplayBounds {
                bootstrap_work: budget
                    .bootstrap_work
                    .checked_sub(bootstrap)
                    .ok_or("cumulative bootstrap allowance exhausted")?,
                split_work: budget
                    .split_work
                    .checked_sub(splits)
                    .ok_or("cumulative split allowance exhausted")?,
                memory_bytes: budget
                    .bytes
                    .checked_sub(charged)
                    .ok_or("retained evidence exceeds memory admission")?,
            };
            // The reader applies the remaining independent allowance to its
            // exact required-work formula before any numerical replay. Saved
            // policy and procedure fields remain unchanged.
            let reader = if let Some(policy) = base_policy {
                qualification::Reader::open_search_slot_bounded(
                    root,
                    link.identity,
                    link.pin,
                    budget.bytes - charged,
                    budget.records,
                    &declaration.expected(&policy, batch, rung, grammar.programs())?,
                    replay,
                )?
            } else {
                let reader = qualification::Reader::open_with_replay_bounds(
                    root,
                    link.identity,
                    budget.bytes - charged,
                    budget.records,
                    replay,
                )?;
                if reader.policy().digest() != declaration.policy_digest {
                    return Err("Legacy search stores only the original policy digest; its tightened child cannot prove that original policy. Cumulative ranking is unavailable; individual saved comparisons remain inspectable.".into());
                }
                base_policy = Some(reader.policy());
                reader.require_search_slot(
                    link.pin,
                    &declaration.expected(&reader.policy(), batch, rung, grammar.programs())?,
                )?;
                reader
            };
            let timeframe = crate::EVERY_RUNG
                .get(rung)
                .ok_or("declared timeframe missing")?;
            for source in [reader.training(), reader.later()] {
                let row = source
                    .records()
                    .first()
                    .ok_or("declared complete family is empty")?;
                if row.family().instrument().to_string() != declaration.index
                    || row.timeframe() != *timeframe
                {
                    return Err(
                        "saved child family differs from the declared index or timeframe".into(),
                    );
                }
            }
            charge(
                &mut charged,
                reader.admitted_bytes(),
                budget.bytes,
                "cumulative retained evidence bytes",
            )?;
            let work = reader.replay_work()?;
            charge(
                &mut bootstrap,
                work.bootstrap_work,
                budget.bootstrap_work,
                "cumulative required bootstrap work",
            )?;
            charge(
                &mut splits,
                work.split_work,
                budget.split_work,
                "cumulative required split work",
            )?;
            if declaration.version == 2 {
                for (later, source) in [(false, reader.training()), (true, reader.later())] {
                    let context = declaration
                        .source_context(rung, later)
                        .ok_or("V2 declared source companion missing")?;
                    let binding = crate::index_stop::source_context::open_catalog_context(
                        root,
                        source.identity(),
                        source.completion_digest(),
                        crate::index_stop::source_context::Link {
                            identity: context.identity,
                            completion: context.completion,
                        },
                        budget
                            .bytes
                            .checked_sub(charged)
                            .ok_or("source companion byte admission exhausted")?,
                    )?;
                    charge(
                        &mut charged,
                        binding.admitted_bytes(),
                        budget.bytes,
                        "cumulative source companion bytes",
                    )?;
                    contexts.try_reserve(1).map_err(display)?;
                    contexts.push(binding);
                }
            }
            families.try_reserve(1).map_err(display)?;
            families.push(Family { batch, reader });
        }
        let checkpoints = u64::try_from(history.len()).map_err(display)?;
        if records.checked_sub(checkpoints) != programs.checked_mul(2) {
            return Err(
                "acknowledged program count differs from complete direction-paired families".into(),
            );
        }
        let observation_count = families
            .len()
            .checked_mul(4)
            .and_then(|count| count.checked_add(contexts.len()))
            .ok_or("cumulative observation lease count overflow")?;
        let projection_scratch = Observation::projection_scratch_bytes(observation_count)?;
        if charged
            .checked_add(projection_scratch)
            .is_none_or(|bytes| bytes > budget.bytes)
        {
            return Err(
                "cumulative observations and compound lease scratch exceed read admission".into(),
            );
        }
        let value = Self {
            declaration,
            sequence,
            pin: seal,
            completed,
            programs,
            work,
            pending,
            exhausted,
            interrupted: snapshot.interrupted,
            writer_observed: snapshot.writer_observed,
            families,
            guards,
            contexts,
            bytes: charged,
            grammar_nodes: nodes,
            bootstrap_work: bootstrap,
            split_work: splits,
            projection_scratch,
            projecting: AtomicBool::new(false),
        };
        value.require_current()?;
        Ok(value)
    }
    /// Verify retained generations without rescanning or reranking history.
    /// This costs O(checkpoints + families), not constant total filesystem work.
    /// # Errors
    /// Refuses any changed or unavailable retained ancestor generation.
    pub fn require_current(&self) -> Result<(), String> {
        if self.projecting.load(Ordering::Acquire) {
            return Err(
                "nested cumulative generation check inside a compound projection is refused".into(),
            );
        }
        self.check_checkpoints()?;
        for family in &self.families {
            family.reader.require_current()?;
        }
        for context in &self.contexts {
            context.require_current()?;
        }
        Ok(())
    }
    fn check_checkpoints(&self) -> Result<(), String> {
        for guard in &self.guards {
            guard.current()?;
        }
        Ok(())
    }
    /// Hold all saved-family and source-relation publication leases for a
    /// complete cumulative rank or page projection. The search journal owner
    /// remains unlocked so newer batches can append beside this pinned prefix.
    /// The callback uses direct getters only, without nested reader leases.
    /// Ordering is O(families log families), followed by bounded ancestor checks.
    /// # Errors
    /// Conflicting aliases, busy or changed artifacts, scratch allocation,
    /// checkpoint generation changes, nested access or a refused projection.
    pub fn with_current<T>(
        &self,
        project: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let _projection = Projection::enter(&self.projecting)?;
        let count = self
            .families
            .len()
            .checked_mul(4)
            .and_then(|count| count.checked_add(self.contexts.len()))
            .ok_or("cumulative observation lease count overflow")?;
        if Observation::projection_scratch_bytes(count)? > self.projection_scratch {
            return Err("cumulative observation lease scratch changed after admission".into());
        }
        let mut observations = Vec::new();
        observations.try_reserve_exact(count).map_err(display)?;
        for family in &self.families {
            observations.extend(family.reader.observations());
        }
        observations.extend(
            self.contexts
                .iter()
                .map(crate::index_stop::source_context::ContextBinding::observation),
        );
        Observation::with_current_many(&observations, self.projection_scratch, || {
            self.check_checkpoints()?;
            let result = project()?;
            self.check_checkpoints()?;
            Ok(result)
        })
    }
    /// Accounted temporary references and compound leases, separate from saved
    /// serialized bytes. This excludes allocator overhead and is not an RSS cap.
    #[must_use]
    pub const fn projection_scratch_bytes(&self) -> u64 {
        self.projection_scratch
    }
    /// Exact declaration used for this prefix.
    #[must_use]
    pub const fn declaration(&self) -> &Declaration {
        &self.declaration
    }
    /// Exact acknowledged prefix sequence and digest.
    #[must_use]
    pub const fn checkpoint(&self) -> (u64, [u8; 32]) {
        (self.sequence, self.pin)
    }
    /// Completed whole batches, including empty grammar batches.
    #[must_use]
    pub const fn completed_batches(&self) -> u64 {
        self.completed
    }
    /// Acknowledged emitted canonical programs across batches.
    #[must_use]
    pub const fn completed_programs(&self) -> u64 {
        self.programs
    }
    /// Acknowledged grammar work counter.
    #[must_use]
    pub const fn completed_work(&self) -> u64 {
        self.work
    }
    /// This checkpoint retains a next reservation whose results are excluded.
    #[must_use]
    pub const fn pending(&self) -> bool {
        self.pending
    }
    /// Exact grammar exhaustion marker; no elapsed-time inference.
    #[must_use]
    pub const fn exhausted(&self) -> bool {
        self.exhausted
    }
    /// Incomplete checkpoint directory reservations observed during discovery.
    #[must_use]
    pub const fn interrupted(&self) -> u64 {
        self.interrupted
    }
    /// Writer lock state observed during discovery, not current process status.
    #[must_use]
    pub const fn writer_observed(&self) -> bool {
        self.writer_observed
    }
    /// Complete requested-timeframe families in original batch order.
    #[must_use]
    pub fn families(&self) -> &[Family] {
        &self.families
    }
    /// Total retained serialized ancestor bytes charged at admission.
    #[must_use]
    pub const fn admitted_bytes(&self) -> u64 {
        self.bytes
    }
    /// Charged grammar and exact required bootstrap/split admission work.
    #[must_use]
    pub const fn charged_work(&self) -> (u64, u64, u64) {
        (self.grammar_nodes, self.bootstrap_work, self.split_work)
    }
}
struct Projection<'a>(&'a AtomicBool);
impl<'a> Projection<'a> {
    fn enter(active: &'a AtomicBool) -> Result<Self, String> {
        active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "nested or concurrent cumulative projection is refused")?;
        Ok(Self(active))
    }
}
impl Drop for Projection<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
fn charge(total: &mut u64, add: u64, maximum: u64, label: &str) -> Result<(), String> {
    *total = total
        .checked_add(add)
        .filter(|value| *value <= maximum)
        .ok_or_else(|| {
            format!("{label} exceeds independent read admission; no partial ranking returned")
        })?;
    Ok(())
}
fn declaration_bytes(saved: &Saved) -> Result<&[u8], String> {
    // Locate the single canonical declaration, then Frame::decode validates
    // the complete checkpoint protocol and all of its surrounding fields.
    let length = u64::from_le_bytes(
        saved
            .payload
            .get(80..88)
            .ok_or("checkpoint declaration length missing")?
            .try_into()
            .map_err(display)?,
    );
    let start: usize = 96 + CURSOR_BYTES + 8 * 64;
    let end = start
        .checked_add(usize::try_from(length).map_err(display)?)
        .ok_or("checkpoint declaration extent overflow")?;
    saved
        .payload
        .get(start..end)
        .ok_or_else(|| "checkpoint declaration truncated".into())
}
struct Guard {
    file: File,
    path: PathBuf,
    generation: crate::result_set::FileGeneration,
}
impl Guard {
    fn open(path: PathBuf) -> Result<Self, String> {
        let file = crate::readonly_file::open(&path).map_err(display)?;
        let generation = crate::result_set::file_generation(&file, &path)?;
        Ok(Self {
            file,
            path,
            generation,
        })
    }
    fn current(&self) -> Result<(), String> {
        crate::result_set::require_generation_unchanged(
            self.generation,
            crate::result_set::file_generation(&self.file, &self.path)?,
            &self.path,
        )
    }
}
fn guarded_read(
    root: &Path,
    identity: [u8; 32],
    snapshot: &Snapshot,
    sequence: u64,
    bytes: u64,
    guards: &mut Vec<Guard>,
) -> Result<Saved, String> {
    use std::fmt::Write as _;
    let identity = identity.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    });
    let directory = root
        .join(NAMESPACE)
        .join(identity)
        .join(format!("{sequence:016x}"));
    let payload = Guard::open(directory.join("payload"))?;
    let complete = Guard::open(directory.join("complete"))?;
    let saved = snapshot.read(sequence, bytes)?;
    payload.current()?;
    complete.current()?;
    guards.try_reserve(2).map_err(display)?;
    guards.push(payload);
    guards.push(complete);
    Ok(saved)
}

#[cfg(test)]
#[path = "index_stop_search_reader_tests.rs"]
mod tests;
