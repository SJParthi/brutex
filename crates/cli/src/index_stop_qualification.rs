//! UNVERIFIED performance: no named cost test or measured latency bound is established here.
//! Source-bound qualification of the native signal-candle-stop policy.
//!
//! Every row in a complete program/side catalog enters the numerical family.
//! Training CSCV is separated from fixed later-day validation, and the original,
//! later and joined calendar spans must all pass the saved index consistency
//! policy. A warm row lookup checks fixed ancestry generations and indexes the
//! retained rows. Initial numerical work, authentication and output are bounded
//! and proportional to their inputs; they are not whole-operation O(1).

use crate::candidate_universe::boolean_candidate_v1::persistence::{self, Observation};
use crate::index_consistency_store::{self as daily, Record as DailyRecord};
use crate::index_stop;
use crate::index_stop_store::{self as candidates, Record as Candidate};
use crate::population_statistics_v2::PopulationStatisticsProcedureV2;
use crate::sweep_evidence::{Completion, Operation};
use brutex_core::blake3::{Hasher, hash};
use runner::admission::AdmissionPolicyV1;
use runner::admission::research_projection::ResearchAdmissionProjectionV1;
use runner::search_allocation_v1::Allocation;
use runner::signal_candle_stop::{Evaluation, Policy};
use std::path::Path;

#[path = "index_stop_qualification_codec.rs"]
mod codec;
#[path = "index_stop_qualification_numeric.rs"]
mod numeric;

/// Separate immutable namespace; no legacy grid record is reinterpreted.
pub const NAMESPACE: &str = "index-stop-qualification-v1";

/// Explicit admission for this complete numerical family and saved ancestry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// Complete program/side settings; never a winner-only subset.
    pub candidates: u64,
    /// Maximum bootstrap work, including the complete family.
    pub bootstrap_work: u64,
    /// Maximum training CSCV candidate/period/split work.
    pub split_work: u64,
    /// Maximum estimated numeric/source payload, including canonical masks,
    /// saved/reproduced splits and score scratch; not total process RSS.
    pub memory_bytes: u64,
    /// Aggregate bytes of both candidate ancestors and both qualification files.
    pub bytes: u64,
}
impl Bounds {
    const fn words(self) -> [u64; 5] {
        [
            self.candidates,
            self.bootstrap_work,
            self.split_work,
            self.memory_bytes,
            self.bytes,
        ]
    }
}

/// Independent current limits for reproducing saved numerical evidence. These
/// limits do not change its identity, policy, probabilities or recorded values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayBounds {
    /// Sum of candidate × later period × (draws + 1) work for three procedures.
    pub bootstrap_work: u64,
    /// Training candidate × period × canonical split work.
    pub split_work: u64,
    /// Estimated numeric buffers and retained observations; not a process RSS
    /// or allocator-overhead guarantee.
    pub memory_bytes: u64,
}

/// Required native admission work derived from exact saved family geometry.
/// These units match the existing numerical admission formulas, not CPU time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayWork {
    /// All three procedures: candidate × later period × (draws + 1) × 3.
    pub bootstrap_work: u64,
    /// Canonical split count × candidate count × training period count.
    pub split_work: u64,
}
impl ReplayBounds {
    fn validate(self) -> Result<(), String> {
        if [self.bootstrap_work, self.split_work, self.memory_bytes].contains(&0) {
            return Err("single-stop independent numerical replay limits must be positive".into());
        }
        Ok(())
    }
    fn limit(self, facts: &Facts) -> Result<Facts, String> {
        self.validate()?;
        let mut limited = *facts;
        limited.bounds.bootstrap_work = facts.bounds.bootstrap_work.min(self.bootstrap_work);
        limited.bounds.split_work = facts.bounds.split_work.min(self.split_work);
        limited.bounds.memory_bytes = facts.bounds.memory_bytes.min(self.memory_bytes);
        Ok(limited)
    }
}

/// Native source and measurement window taken from the caller's immutable
/// search declaration, independently of a child's self-reported ancestry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedSource {
    /// Exact prepared native source identity.
    pub identity: [u8; 32],
    /// Inclusive first measured IST civil day.
    pub first_day: i64,
    /// Inclusive last measured IST civil day.
    pub last_day: i64,
}

/// Complete expected checkpoint slot. A matching search hash in an otherwise
/// valid child is only a reference; every answer-changing field must also agree.
pub struct ExpectedSearchSlot<'a> {
    /// Hash of the independently recovered declaration.
    pub search: [u8; 32],
    /// Exact training source and output days.
    pub training: ExpectedSource,
    /// Exact original-first later source and later output days.
    pub later: ExpectedSource,
    /// Effective institutional policy after the declaration's shared ceiling.
    pub policy: AdmissionPolicyV1,
    /// Declared draws, seed and block length.
    pub procedure: PopulationStatisticsProcedureV2,
    /// Full allocation, including its original shared alpha.
    pub allocation: Allocation,
    /// Declared complete-family production admission.
    pub bounds: Bounds,
    /// Exact complete canonical programs in this reserved batch.
    pub programs: &'a [runner::expression::Expression],
}
impl ExpectedSearchSlot<'_> {
    fn check_facts(&self, facts: &Facts) -> Result<(), String> {
        if facts.search != self.search
            || facts.policy != self.policy
            || facts.procedure != self.procedure
            || facts.allocation != self.allocation
            || facts.bounds != self.bounds
            || self.programs.len().checked_mul(2) != Some(facts.count)
        {
            return Err("single-stop checkpoint child differs from its declared policy, procedure, allocation, bounds or program extent".into());
        }
        Ok(())
    }
    fn check_sources(
        &self,
        training: &[candidates::Record],
        later: &[candidates::Record],
    ) -> Result<(), String> {
        for (rows, expected) in [(training, self.training), (later, self.later)] {
            if self.programs.len().checked_mul(2) != Some(rows.len())
                || rows.iter().any(|row| {
                    row.source_id() != expected.identity
                        || row.first_day() != expected.first_day
                        || row.last_day() != expected.last_day
                })
                || self
                    .programs
                    .iter()
                    .zip(rows.chunks_exact(2))
                    .any(|(program, pair)| pair.iter().any(|row| row.program() != program))
            {
                return Err("single-stop checkpoint child differs from its declared native source, measurement window or complete expression".into());
            }
        }
        Ok(())
    }
}

/// Exact source receipts and predeclared search allocation, before measurement.
#[derive(Clone, Copy)]
pub struct Request<'a> {
    /// Server-owned append-only market/evidence root.
    pub root: &'a Path,
    /// Immutable grammar/source/order plan; retries reuse its allocation.
    pub search_identity: [u8; 32],
    /// Independently frozen training-only source and native executions.
    pub training: &'a index_stop::Committed,
    /// Same coordinates, later output days and original-first causal history.
    pub later: &'a index_stop::Committed,
    /// Existing complete institutional policy, with no weakened threshold.
    pub policy: &'a AdmissionPolicyV1,
    /// Fixed numerical draws, seed and block procedure.
    pub procedure: PopulationStatisticsProcedureV2,
    /// Exact ordinal/physical-rung allocation in the immutable search.
    pub allocation: Allocation,
    /// Complete-family physical admission.
    pub bounds: Bounds,
}

/// Exact immutable candidate ancestor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    /// Parent artifact identity.
    pub identity: [u8; 32],
    /// Exact completed parent bytes.
    pub completion: [u8; 32],
}

/// One fixed later civil-day window, with original coordinates unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fold {
    /// Inclusive first requested IST day.
    pub first_day: i64,
    /// Inclusive last requested IST day.
    pub last_day: i64,
    /// Actually observed execution days in this window.
    pub observed_days: u64,
    /// Completed round trips in this window.
    pub trades: u64,
    /// Strictly profitable pessimistic round trips.
    pub wins: u64,
    /// Exact summed pessimistic gross paisa.
    pub pessimistic_paisa: i64,
    /// Refused paths in this window.
    pub refused: u64,
    /// Complete calendar, data and priceable execution evidence.
    pub decided: bool,
}

/// Native row bindings and detached comparison arithmetic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// Exact original native run identity, including its complete expression.
    pub original_run: [u8; 32],
    /// Exact later native run identity, including the later-day window.
    pub later_run: [u8; 32],
    /// Complete original observation bytes digest.
    pub original_digest: [u8; 32],
    /// Complete later observation bytes digest.
    pub later_digest: [u8; 32],
    /// Policy projection, which alone is never trading approval.
    pub projection: ResearchAdmissionProjectionV1,
    /// Exact IEEE754 Wilson lower-bound source bits.
    pub wilson_bits: u64,
    /// Complete conservative/adjusted Romano-Wolf source values.
    pub romano: [u64; 12],
    /// Every fixed later window, including zero-trade and refused windows.
    pub folds: Vec<Fold>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Split {
    train: u64,
    test: u64,
    bottom_half: bool,
    rankable: bool,
    scores: [u8; 32],
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Statistics {
    white: [u64; 5],
    spa: [u64; 5],
    family: [u8; 32],
    shared: Option<[u8; 32]>,
    layout: Option<[u64; 4]>,
    layout_digest: [u8; 32],
    splits: Vec<Split>,
}
#[derive(Clone, Copy)]
struct Facts {
    search: [u8; 32],
    training: Link,
    later: Link,
    policy: AdmissionPolicyV1,
    procedure: PopulationStatisticsProcedureV2,
    allocation: Allocation,
    bounds: Bounds,
    count: usize,
}
struct Body {
    facts: Facts,
    statistics: Statistics,
    rows: Vec<Row>,
}

/// Receipt-last qualification retaining exact read-only native ancestry.
pub struct Reader {
    observation: Observation,
    training: candidates::Reader,
    later: candidates::Reader,
    daily: daily::Reader,
    body: Body,
}
impl Reader {
    /// Cold, bounded authentication and numerical reconciliation. Never called
    /// implicitly by a warm per-row lookup.
    /// # Errors
    /// Missing/corrupt/foreign evidence, changed sources or insufficient bounds.
    pub fn open(
        root: &Path,
        identity: [u8; 32],
        max_bytes: u64,
        max_records: u64,
    ) -> Result<Self, String> {
        Self::open_bounded(root, identity, max_bytes, max_records, None, None)
    }
    /// Cold read with independent current numerical work/memory admission.
    /// A saved receipt can never enlarge these bounds. No partial projection is
    /// returned when a complete replay does not fit.
    /// # Errors
    /// Invalid current bounds, unauthenticated ancestry or over-budget replay.
    pub fn open_with_replay_bounds(
        root: &Path,
        identity: [u8; 32],
        max_bytes: u64,
        max_records: u64,
        replay: ReplayBounds,
    ) -> Result<Self, String> {
        replay.validate()?;
        Self::open_bounded(root, identity, max_bytes, max_records, Some(replay), None)
    }
    /// Open an exact acknowledged search child with its independently decoded
    /// declaration checked before numerical replay. The returned reader is the
    /// same authenticated object used by cumulative comparison pages.
    /// # Errors
    /// Foreign completion, declaration, sources, procedures or replay admission.
    pub fn open_search_slot_bounded(
        root: &Path,
        identity: [u8; 32],
        pin: [u8; 32],
        max_bytes: u64,
        max_records: u64,
        expected: &ExpectedSearchSlot<'_>,
        replay: ReplayBounds,
    ) -> Result<Self, String> {
        replay.validate()?;
        let reader = Self::open_bounded(
            root,
            identity,
            max_bytes,
            max_records,
            Some(replay),
            Some(expected),
        )?;
        reader.require_search_slot(pin, expected)?;
        Ok(reader)
    }
    /// Reconcile a retained reader against the exact parent declaration. This
    /// supports legacy declarations only when their base-policy digest can be
    /// matched exactly; it never substitutes the current runtime policy.
    /// # Errors
    /// A changed pin, source, policy, program list or measurement window.
    pub fn require_search_slot(
        &self,
        pin: [u8; 32],
        expected: &ExpectedSearchSlot<'_>,
    ) -> Result<(), String> {
        if self.completion_digest() != pin {
            return Err("single-stop checkpoint qualification pin differs".into());
        }
        expected.check_facts(&self.body.facts)?;
        expected.check_sources(self.training.records(), self.later.records())?;
        self.require_current()
    }
    fn open_bounded(
        root: &Path,
        identity: [u8; 32],
        max_bytes: u64,
        max_records: u64,
        replay: Option<ReplayBounds>,
        expected: Option<&ExpectedSearchSlot<'_>>,
    ) -> Result<Self, String> {
        let (observation, body) =
            Observation::open(root, NAMESPACE, identity, max_bytes, |bytes| {
                codec::decode(bytes, identity, max_records)
            })?;
        if let Some(expected) = expected {
            expected.check_facts(&body.facts)?;
        }
        let remaining = max_bytes
            .min(body.facts.bounds.bytes)
            .checked_sub(observation.body_bytes())
            .and_then(|n| n.checked_sub(112))
            .ok_or("single-stop qualification body exhausts read admission")?;
        let training =
            candidates::Reader::open(root, body.facts.training.identity, remaining, max_records)?;
        let remaining = remaining
            .checked_sub(training.admitted_bytes())
            .ok_or("training ancestor exhausts read admission")?;
        let later =
            candidates::Reader::open(root, body.facts.later.identity, remaining, max_records)?;
        let remaining = remaining
            .checked_sub(later.admitted_bytes())
            .ok_or("later ancestor exhausts read admission")?;
        if training.completion_digest() != body.facts.training.completion
            || later.completion_digest() != body.facts.later.completion
        {
            return Err("single-stop qualification candidate pin differs".into());
        }
        if let Some(expected) = expected {
            expected.check_sources(training.records(), later.records())?;
        }
        let daily = daily::Reader::open(
            root,
            identity,
            observation.completion_digest(),
            crate::index_consistency::INDEX_STOP,
            remaining,
            body.facts.count,
        )?;
        let reader = Self {
            observation,
            training,
            later,
            daily,
            body,
        };
        reader.check(replay)?;
        reader.require_current()?;
        Ok(reader)
    }
    /// Exact immutable qualification identity.
    #[must_use]
    pub fn identity(&self) -> [u8; 32] {
        self.observation.identity()
    }
    /// Exact completed main-body pin; mandatory daily ancestry is derived from it.
    #[must_use]
    pub fn completion_digest(&self) -> [u8; 32] {
        self.observation.completion_digest()
    }
    /// Native training candidate receipt retained by this reader.
    #[must_use]
    pub const fn training(&self) -> &candidates::Reader {
        &self.training
    }
    /// Native later candidate receipt retained by this reader.
    #[must_use]
    pub const fn later(&self) -> &candidates::Reader {
        &self.later
    }
    /// Saved complete institutional policy.
    #[must_use]
    pub const fn policy(&self) -> AdmissionPolicyV1 {
        self.body.facts.policy
    }
    /// Original immutable search plan identity.
    #[must_use]
    pub const fn search_identity(&self) -> [u8; 32] {
        self.body.facts.search
    }
    /// Exact predeclared ordinal and physical rung allocation.
    #[must_use]
    pub const fn allocation(&self) -> Allocation {
        self.body.facts.allocation
    }
    /// Complete canonical program/side rows, including all failures.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.body.rows
    }
    /// Retained daily rows for use inside `with_current`; no nested lease.
    #[must_use]
    pub fn consistencies(&self) -> &[DailyRecord] {
        self.daily.records()
    }
    /// Exact saved day/week receipt.
    #[must_use]
    pub fn consistency_receipt(&self) -> daily::Receipt {
        self.daily.receipt()
    }
    /// Aggregate authenticated main, daily and native-parent bytes. The cold
    /// reader proved this sum fits both current and recorded byte admission.
    #[must_use]
    pub fn admitted_bytes(&self) -> u64 {
        self.observation.body_bytes()
            + 112
            + self.training.admitted_bytes()
            + self.later.admitted_bytes()
            + self.daily.admitted_bytes()
    }
    /// Required replay work from the exact retained native counts. Callers
    /// retain the reader's completion pin and check its generations separately.
    /// # Errors
    /// Missing source geometry or unrepresentable complete work refuses.
    pub fn replay_work(&self) -> Result<ReplayWork, String> {
        numeric::required_work(
            self.training.records(),
            self.later.records(),
            &self.body.facts,
        )
    }
    /// Fixed ancestry generation checks; no daily or bootstrap replay.
    /// # Errors
    /// Replaced, missing, modified or concurrently written evidence.
    pub fn require_current(&self) -> Result<(), String> {
        self.observation.require_current()?;
        self.training.require_current()?;
        self.later.require_current()?;
        self.daily.require_current()?;
        self.observation.require_current()
    }
    /// Hold immutable parent publication leases for one bounded API projection.
    /// The closure uses direct `rows`, `consistencies` and candidate `records`
    /// getters; it must not call methods that acquire these same leases again.
    /// # Errors
    /// Changed evidence, concurrent publication or a refused projection.
    pub fn with_current<T>(
        &self,
        project: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        self.observation.with_current(|| {
            self.training
                .with_current(|| self.later.with_current(|| self.daily.with_current(project)))
        })
    }
    pub(crate) const fn observations(&self) -> [&Observation; 4] {
        [
            &self.observation,
            self.training.observation(),
            self.later.observation(),
            self.daily.observation(),
        ]
    }
    /// Direct pinned lookup after cold authentication.
    /// # Errors
    /// Foreign pin, stale ancestry or an index outside the saved extent.
    pub fn row(&self, pin: [u8; 32], index: usize) -> Result<&Row, String> {
        if pin != self.completion_digest() {
            return Err("single-stop qualification pin differs".into());
        }
        self.require_current()?;
        self.body
            .rows
            .get(index)
            .ok_or_else(|| "single-stop qualification setting outside saved extent".into())
    }
    /// Direct saved three-period assessment, without recalculating any day.
    /// # Errors
    /// Foreign pin, stale ancestry or an invalid setting coordinate.
    pub fn consistency(&self, pin: [u8; 32], index: usize) -> Result<&DailyRecord, String> {
        self.row(pin, index)?;
        self.daily.record(pin, index)
    }
    /// Combined institutional and all-three-period admission for selection.
    /// # Errors
    /// Any missing, foreign or changed required evidence.
    pub fn combined_qualifies(&self, pin: [u8; 32], index: usize) -> Result<bool, String> {
        let row = self.row(pin, index)?;
        Ok(self
            .daily
            .record(pin, index)?
            .combined_qualifies(row.projection.verdict().status()))
    }
    fn check(&self, replay: Option<ReplayBounds>) -> Result<(), String> {
        let limited = replay
            .map(|limits| limits.limit(&self.body.facts))
            .transpose()?;
        let facts = limited.as_ref().unwrap_or(&self.body.facts);
        numeric::validate_saved_layout(self.training.records(), &self.body.statistics)?;
        numeric::validate_family(self.training.records(), self.later.records(), facts)?;
        // Independent limits admit the exact geometry before numerical work.
        // The original bounds are also part of the saved zero-bootstrap digest;
        // they select no work or buffer size and must not be rewritten on read.
        let measured = numeric::measure(
            self.training.records(),
            self.later.records(),
            &self.body.facts,
        )?;
        if measured.0 != self.body.statistics || measured.1 != self.body.rows {
            return Err(
                "single-stop saved numerical evidence does not reproduce from exact native parents"
                    .into(),
            );
        }
        let expected = numeric::daily(
            self.training.records(),
            self.later.records(),
            &self.body.facts,
        )?;
        for (index, expected) in expected.iter().enumerate() {
            if self.daily.record(self.completion_digest(), index)? != expected {
                return Err("single-stop daily evidence differs from exact native parents".into());
            }
        }
        Ok(())
    }
}

/// Successful publication with native source guards still checked at return.
pub struct Committed<'a> {
    reader: Reader,
    training: &'a index_stop::Committed,
    later: &'a index_stop::Committed,
}
impl Committed<'_> {
    /// Exact saved result identity.
    #[must_use]
    pub fn identity(&self) -> [u8; 32] {
        self.reader.identity()
    }
    /// Exact saved result completion pin.
    #[must_use]
    pub fn completion_digest(&self) -> [u8; 32] {
        self.reader.completion_digest()
    }
    /// Direct reader of every native row, decision and consistency receipt.
    #[must_use]
    pub const fn reader(&self) -> &Reader {
        &self.reader
    }
    /// Current exact saved evidence, including both candidate ancestors.
    /// # Errors
    /// Any source file generation or mandatory receipt differs.
    pub fn require_current(&self) -> Result<(), String> {
        self.training.require_current()?;
        self.later.require_current()?;
        self.reader.require_current()
    }
}

/// Measure and persist the complete native family; never select winners first.
/// # Errors
/// Mismatched program/side/window/source, unavailable statistics, physical bound,
/// persistence failure or missing mandatory daily assessment.
pub fn produce(request: Request<'_>) -> Result<Committed<'_>, String> {
    current(&request)?;
    let facts = Facts {
        search: request.search_identity,
        training: Link {
            identity: request.training.identity(),
            completion: request.training.completion_digest(),
        },
        later: Link {
            identity: request.later.identity(),
            completion: request.later.completion_digest(),
        },
        policy: request
            .policy
            .with_search_probability_ceiling(request.allocation.alpha_ppm()),
        procedure: request.procedure,
        allocation: request.allocation,
        bounds: request.bounds,
        count: request.training.evaluations().len(),
    };
    numeric::validate_family(
        request.training.evaluations(),
        request.later.evaluations(),
        &facts,
    )?;
    let identity = identity(&facts);
    let attempt =
        crate::sweep_evidence::begin(request.root, identity, Operation::IndexStopQualification)?;
    let result = publish(&request, &facts, identity);
    match result {
        Ok(reader) => {
            current(&request)?;
            attempt.finish(Completion::Completed)?;
            Ok(Committed {
                reader,
                training: request.training,
                later: request.later,
            })
        }
        Err(why) => {
            attempt.finish(Completion::Refused)?;
            Err(why)
        }
    }
}
fn publish(request: &Request<'_>, facts: &Facts, identity: [u8; 32]) -> Result<Reader, String> {
    let (statistics, rows) = numeric::measure(
        request.training.evaluations(),
        request.later.evaluations(),
        facts,
    )?;
    let records = numeric::daily(
        request.training.evaluations(),
        request.later.evaluations(),
        facts,
    )?;
    let body = codec::encode(
        identity,
        &Body {
            facts: *facts,
            statistics,
            rows,
        },
    )?;
    let bytes = u64::try_from(body.len()).map_err(display)?;
    let pending = persistence::prepare_in_namespace(request.root, NAMESPACE, identity, &body)?;
    let digest = hash(&body);
    current(request)?;
    pending.verify_body(digest, bytes)?;
    let pin = pending.finish(identity, digest, bytes)?;
    drop(body);
    drop(pending);
    daily::produce(
        request.root,
        identity,
        pin,
        records,
        request.bounds.bytes,
        || current(request),
    )?;
    Reader::open(
        request.root,
        identity,
        request.bounds.bytes,
        request.bounds.bytes,
    )
}
fn current(request: &Request<'_>) -> Result<(), String> {
    request.training.require_current()?;
    request.later.require_current()?;
    request.training.require_current()
}
fn identity(facts: &Facts) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(b"brutex-index-stop-qualification-v1\0");
    for id in [
        facts.search,
        facts.training.identity,
        facts.training.completion,
        facts.later.identity,
        facts.later.completion,
    ] {
        h.update(&id);
    }
    h.update(&Policy::V1.canonical_bytes());
    h.update(&facts.policy.canonical_bytes());
    h.update(&crate::index_consistency::INDEX_STOP.canonical_bytes());
    h.update(&facts.allocation.digest());
    for word in facts.bounds.words().into_iter().chain([
        facts.procedure.draws(),
        facts.procedure.seed(),
        facts.procedure.block_length(),
    ]) {
        h.update(&word.to_le_bytes());
    }
    h.finalize()
}
fn display(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Cold checkpoint verification of one exact qualification completion.
/// # Errors
/// Missing, corrupt, changed or foreign ancestry and numerical evidence.
pub fn verify_receipt(
    root: &Path,
    id: [u8; 32],
    pin: [u8; 32],
    max_bytes: u64,
    max_records: u64,
) -> Result<(), String> {
    let reader = Reader::open(root, id, max_bytes, max_records)?;
    if reader.completion_digest() != pin {
        return Err("single-stop checkpoint qualification pin differs".into());
    }
    reader.require_current()
}

/// Cold verification also binding search order, physical rung and full programs.
/// # Errors
/// A valid but foreign child, omitted/reordered programs or changed ancestry.
#[expect(
    clippy::too_many_arguments,
    reason = "all immutable checkpoint slot facts are explicit rather than inferred from a child"
)]
pub fn verify_search_slot(
    root: &Path,
    id: [u8; 32],
    pin: [u8; 32],
    max_bytes: u64,
    max_records: u64,
    search: [u8; 32],
    batch: u64,
    rung: u64,
    programs: &[runner::expression::Expression],
) -> Result<(), String> {
    let reader = Reader::open(root, id, max_bytes, max_records)?;
    if reader.completion_digest() != pin
        || reader.search_identity() != search
        || reader.allocation().batch() != batch
        || reader.allocation().rung() != rung
        || programs.len().checked_mul(2) != Some(reader.rows().len())
    {
        return Err(
            "single-stop checkpoint belongs to another search, slot or program extent".into(),
        );
    }
    for (program, pair) in programs
        .iter()
        .zip(reader.training().records().chunks_exact(2))
    {
        if pair.iter().any(|row| row.program() != program) {
            return Err("single-stop checkpoint complete expression differs".into());
        }
    }
    reader.require_current()
}

/// Verify one recovered checkpoint against its independently known declaration
/// and current numerical read limits. Declaration and native-source equality are
/// checked before any saved numerical procedure is replayed.
/// # Errors
/// Foreign child facts, changed ancestry, or an independently refused cold replay.
pub fn verify_search_slot_bounded(
    root: &Path,
    id: [u8; 32],
    pin: [u8; 32],
    max_bytes: u64,
    max_records: u64,
    expected: &ExpectedSearchSlot<'_>,
    replay: ReplayBounds,
) -> Result<(), String> {
    replay.validate()?;
    let reader = Reader::open_bounded(
        root,
        id,
        max_bytes,
        max_records,
        Some(replay),
        Some(expected),
    )?;
    if reader.completion_digest() != pin {
        return Err("single-stop checkpoint qualification pin differs".into());
    }
    reader.require_current()
}

trait Snapshot {
    fn run(&self) -> [u8; 32];
    fn digest(&self) -> [u8; 32];
    fn source(&self) -> [u8; 32];
    fn family(&self) -> runner::research_family::ResearchFamilyV1;
    fn direction(&self) -> runner::identity::Direction;
    fn timeframe(&self) -> &'static str;
    fn first(&self) -> i64;
    fn last(&self) -> i64;
    fn program(&self) -> &runner::expression::Expression;
    fn truth(&self) -> runner::expression::Summary;
    fn metrics(&self) -> runner::signal_candle_stop::Metrics;
    fn trades(&self) -> &[runner::signal_candle_stop::Trade];
    fn events(&self) -> &[runner::signal_candle_stop::Event];
    fn periods(&self) -> &[runner::signal_candle_stop::Period];
}
macro_rules! snapshot {
    ($ty:ty,$digest:ident) => {
        impl Snapshot for $ty {
            fn run(&self) -> [u8; 32] {
                self.run_id()
            }
            fn digest(&self) -> [u8; 32] {
                self.$digest()
            }
            fn source(&self) -> [u8; 32] {
                self.source_id()
            }
            fn family(&self) -> runner::research_family::ResearchFamilyV1 {
                self.family()
            }
            fn direction(&self) -> runner::identity::Direction {
                self.direction()
            }
            fn timeframe(&self) -> &'static str {
                self.timeframe()
            }
            fn first(&self) -> i64 {
                self.first_day()
            }
            fn last(&self) -> i64 {
                self.last_day()
            }
            fn program(&self) -> &runner::expression::Expression {
                self.program()
            }
            fn truth(&self) -> runner::expression::Summary {
                self.truth()
            }
            fn metrics(&self) -> runner::signal_candle_stop::Metrics {
                self.metrics()
            }
            fn trades(&self) -> &[runner::signal_candle_stop::Trade] {
                self.trades()
            }
            fn events(&self) -> &[runner::signal_candle_stop::Event] {
                self.events()
            }
            fn periods(&self) -> &[runner::signal_candle_stop::Period] {
                self.periods()
            }
        }
    };
}
snapshot!(Evaluation, digest);
snapshot!(Candidate, evaluation_digest);

#[cfg(test)]
#[path = "index_stop_qualification_tests.rs"]
mod tests;
