//! Immutable, source-bound index consistency receipts, separate from historical
//! qualification formats. Cold authentication is O(saved bytes); an admitted
//! setting lookup is direct indexing and a page copies at most 256 records.
use crate::candidate_universe::boolean_candidate_v1::persistence::{self, Observation};
use crate::index_consistency::{Evaluation, Outcome, Policy, Session, Week};
use brutex_core::blake3::{Hasher, hash};
use runner::admission::AdmissionStatusV1;
use std::path::Path;

const NAMESPACE: &str = "index-consistency-v1";
const MAGIC: &[u8; 8] = b"BRICST01";
const BINDING_BYTES: usize = 176;

/// Exact qualification coordinate and its authenticated later data authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    /// Original complete coordinate identity.
    pub original: [u8; 32],
    /// Later complete coordinate identity, including the actual exit settings.
    pub later: [u8; 32],
    /// Later family artifact identity.
    pub later_identity: [u8; 32],
    /// Exact later artifact completion pin.
    pub later_completion: [u8; 32],
    /// Strict source identity, including calendar and OHLCV receipts.
    pub source: [u8; 32],
    /// Canonical family index in the parent qualification.
    pub family: u64,
    /// Full coordinate index within that family.
    pub coordinate: u64,
}
impl Binding {
    fn encode(self, out: &mut Vec<u8>) {
        for digest in [
            self.original,
            self.later,
            self.later_identity,
            self.later_completion,
            self.source,
        ] {
            out.extend_from_slice(&digest);
        }
        out.extend_from_slice(&self.family.to_le_bytes());
        out.extend_from_slice(&self.coordinate.to_le_bytes());
    }
    fn decode(raw: &mut Decoder<'_>) -> Result<Self, String> {
        Ok(Self {
            original: raw.array()?,
            later: raw.array()?,
            later_identity: raw.array()?,
            later_completion: raw.array()?,
            source: raw.array()?,
            family: raw.u64()?,
            coordinate: raw.u64()?,
        })
    }
}

/// One immutable assessment. Daily rows include observed zero-trade sessions;
/// absent expected sessions remain missing evidence, never manufactured zeros.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// Exact parent coordinate mapping.
    pub binding: Binding,
    /// Persisted outcome, counters, calendar and source digests.
    pub evaluation: Evaluation,
    /// Independent original training assessment, before later selection evidence.
    pub training: Evaluation,
    /// Independent later-period assessment, with frozen original coordinates.
    pub later: Evaluation,
    /// Prefix length belonging to the original training observations.
    pub training_session_count: usize,
    /// Exact observed daily pessimistic gross totals.
    pub sessions: Vec<Session>,
}
impl Record {
    /// All periods must satisfy the rule; missing intervening expected sessions
    /// in the full span remain refused instead of being filled or skipped.
    #[must_use]
    pub fn outcome(&self) -> Outcome {
        for outcome in [Outcome::Refused, Outcome::Unmeasured, Outcome::Failed] {
            if [&self.evaluation, &self.training, &self.later]
                .iter()
                .any(|value| value.outcome == outcome)
            {
                return outcome;
            }
        }
        self.evaluation.outcome
    }
    /// Intersection with the relevant institutional verdict, including any
    /// stricter search-wide comparison. This never creates a trading approval.
    #[must_use]
    pub fn combined_qualifies(&self, institutional: AdmissionStatusV1) -> bool {
        institutional == AdmissionStatusV1::Admitted
            && matches!(self.outcome(), Outcome::Passed | Outcome::NotApplicable)
    }
}

/// Completion identity for an authenticated consistency artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    /// Content-declared identity tied to the exact parent and policy.
    pub identity: [u8; 32],
    /// Exact completed bytes pin.
    pub completion: [u8; 32],
}

/// Retained saved observations. This type cannot mint producer authority.
pub struct Reader {
    observation: Observation,
    parent: [u8; 32],
    parent_completion: [u8; 32],
    records: Vec<Record>,
}
impl Reader {
    pub(crate) fn open(
        root: &Path,
        parent: [u8; 32],
        pin: [u8; 32],
        policy: Policy,
        max_bytes: u64,
        count: usize,
    ) -> Result<Self, String> {
        let id = identity(parent, pin, policy);
        let (observation, records) = Observation::open(root, NAMESPACE, id, max_bytes, |bytes| {
            decode(bytes, parent, pin, policy, count)
        })?;
        Ok(Self {
            observation,
            parent,
            parent_completion: pin,
            records,
        })
    }
    /// Exact qualification parent identity and completion.
    #[must_use]
    pub const fn parent(&self) -> ([u8; 32], [u8; 32]) {
        (self.parent, self.parent_completion)
    }
    /// Exact consistency completion identity.
    #[must_use]
    pub fn receipt(&self) -> Receipt {
        Receipt {
            identity: self.observation.identity(),
            completion: self.observation.completion_digest(),
        }
    }
    /// Serialized body and completion bytes, charged to the aggregate reader budget.
    #[must_use]
    pub fn admitted_bytes(&self) -> u64 {
        self.observation.body_bytes() + 112
    }
    pub(crate) const fn observation(&self) -> &Observation {
        &self.observation
    }
    /// Recheck exact immutable file generations; no body rescan on warm lookup.
    /// # Errors
    /// Missing, replaced, modified or busy evidence refuses.
    pub fn require_current(&self) -> Result<(), String> {
        self.observation.require_current()
    }
    pub(crate) fn with_current<T>(
        &self,
        project: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        self.observation.with_current(project)
    }
    /// Look up the exact saved setting in constant indexing work.
    /// # Errors
    /// Refuses foreign pins, out-of-range coordinates and changed bytes.
    pub fn record(&self, pin: [u8; 32], index: usize) -> Result<&Record, String> {
        if pin != self.parent_completion {
            return Err("index consistency parent completion differs".into());
        }
        self.require_current()?;
        self.records
            .get(index)
            .ok_or_else(|| "index consistency setting is outside the saved extent".into())
    }
    pub(crate) fn records(&self) -> &[Record] {
        &self.records
    }
}

pub(crate) fn identity(parent: [u8; 32], pin: [u8; 32], policy: Policy) -> [u8; 32] {
    let mut state = Hasher::new();
    state.update(b"brutex-index-consistency-v1\0");
    state.update(&parent);
    state.update(&pin);
    state.update(&policy.canonical_bytes());
    state.finalize()
}

pub(crate) fn produce(
    root: &Path,
    parent: [u8; 32],
    pin: [u8; 32],
    records: Vec<Record>,
    max_bytes: u64,
    verify_parent: impl Fn() -> Result<(), String>,
) -> Result<Reader, String> {
    verify_parent()?;
    // KEY the identity here; JUDGE the set inside the audited region below.
    //
    // `declared_policy` ran here and refused a MIXED or EMPTY set before
    // `sweep_evidence::begin`, so neither produced an attempt row, a Refused
    // terminal, or anything on /logs, while every other refusal in this
    // function still recorded one. The mixed case is restored: `keying_policy`
    // answers only "which content address is this" from the first record, the
    // attempt opens, and `encode`'s `declared_policy` then refuses inside the
    // closure and terminates the attempt as Refused.
    //
    // THE EMPTY CASE IS NOT FIXED AND CANNOT BE, and D-0601 overstated this by
    // naming both. A record set with no evaluations declares no policy, so it
    // has no content address, so there is no identity to file an audit row
    // under -- `begin` takes one. The refusal is correct and the absence of a
    // row is structural, not an oversight. Neither live producer can reach it:
    // `index_stop_qualification_numeric` refuses an empty training family and
    // `boolean_qualification_v1` refuses a zero count, both before this call.
    // D-0602.
    let policy = keying_policy(&records)?;
    let id = identity(parent, pin, policy);
    let attempt =
        crate::sweep_evidence::begin(root, id, crate::sweep_evidence::Operation::IndexConsistency)?;
    let result = (|| {
        let count = records.len();
        let body = encode(parent, pin, &records, max_bytes)?;
        drop(records);
        let pending = persistence::prepare_in_namespace(root, NAMESPACE, id, &body)?;
        verify_parent()?;
        pending.verify_body(hash(&body), body.len() as u64)?;
        pending.finish(id, hash(&body), body.len() as u64)?;
        drop(pending);
        drop(body);
        verify_parent()?;
        Reader::open(root, parent, pin, policy, max_bytes, count)
    })();
    match result {
        Ok(reader) => {
            attempt.finish(crate::sweep_evidence::Completion::Completed)?;
            reader.require_current()?;
            Ok(reader)
        }
        Err(why) => {
            let audit = attempt.finish(crate::sweep_evidence::Completion::Refused);
            Err(audit.err().map_or(why.clone(), |failure| {
                format!("{why}; consistency refusal audit failed: {failure}")
            }))
        }
    }
}

/// The policy a set of records was evaluated under, refusing a mixed set.
///
/// Read from the EVIDENCE rather than pinned to a constant. A constant here
/// was `Policy::V1` at four sites, so the store could only ever hold the one
/// version the day it was written, and pointing the live path at a newer policy
/// failed inside the writer rather than at the caller that chose it. Deriving
/// it keeps every shipped record readable as the version it was actually
/// evaluated under, which is what §3 rule 8 asks for.
/// The policy to address this artifact by, without judging the whole set.
///
/// Deliberately weaker than [`declared_policy`]: it answers "which content
/// address does this belong at" so an attempt can be OPENED, and leaves "is
/// this set coherent" to the strict check inside the audited region.
fn keying_policy(records: &[Record]) -> Result<Policy, String> {
    records
        .first()
        .map(|record| record.evaluation.policy)
        .ok_or_else(|| "index consistency requires at least one evaluated coordinate".into())
}

fn declared_policy(records: &[Record]) -> Result<Policy, String> {
    let mut found: Option<Policy> = None;
    for record in records {
        for evaluation in [&record.evaluation, &record.training, &record.later] {
            if evaluation.week_count != evaluation.weeks.len() as u64
                || *found.get_or_insert(evaluation.policy) != evaluation.policy
            {
                return Err(
                    "index consistency requires one exact policy across every retained evaluation and its weekly rows"
                        .into(),
                );
            }
        }
    }
    found.ok_or_else(|| "index consistency requires at least one evaluated coordinate".into())
}

fn encode(
    parent: [u8; 32],
    pin: [u8; 32],
    records: &[Record],
    max_bytes: u64,
) -> Result<Vec<u8>, String> {
    let policy = declared_policy(records)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&parent);
    bytes.extend_from_slice(&pin);
    bytes.extend_from_slice(&policy.canonical_bytes());
    bytes.extend_from_slice(&(records.len() as u64).to_le_bytes());
    for record in records {
        let evaluations = [&record.evaluation, &record.training, &record.later];
        let week_count = evaluations
            .iter()
            .try_fold(0_usize, |n, value| n.checked_add(value.weeks.len()))
            .ok_or("index consistency week count overflow")?;
        let count = BINDING_BYTES
            .checked_add(Evaluation::BYTE_LEN * 3)
            .and_then(|n| n.checked_add(40))
            .and_then(|n| {
                week_count
                    .checked_mul(Week::BYTE_LEN)
                    .and_then(|m| n.checked_add(m))
            })
            .and_then(|n| {
                record
                    .sessions
                    .len()
                    .checked_mul(Session::BYTE_LEN)
                    .and_then(|m| n.checked_add(m))
            })
            .and_then(|n| n.checked_add(bytes.len()))
            .ok_or("index consistency byte count overflow")?;
        if count as u64 > max_bytes.saturating_sub(112) {
            return Err("index consistency receipt exceeds explicit byte admission".into());
        }
        bytes
            .try_reserve_exact(count - bytes.len())
            .map_err(display)?;
        record.binding.encode(&mut bytes);
        for evaluation in evaluations {
            bytes.extend_from_slice(&evaluation.canonical_bytes());
        }
        for evaluation in evaluations {
            bytes.extend_from_slice(&(evaluation.weeks.len() as u64).to_le_bytes());
        }
        bytes.extend_from_slice(&(record.sessions.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&(record.training_session_count as u64).to_le_bytes());
        for evaluation in evaluations {
            for week in &evaluation.weeks {
                bytes.extend_from_slice(&week.canonical_bytes());
            }
        }
        for session in &record.sessions {
            bytes.extend_from_slice(&session.canonical_bytes());
        }
    }
    // Decoder enforces fixed canonical extents, retained week hashes and exact
    // parent/policy before any receipt may be published.
    decode(&bytes, parent, pin, policy, records.len())?;
    Ok(bytes)
}

fn decode(
    bytes: &[u8],
    parent: [u8; 32],
    pin: [u8; 32],
    policy: Policy,
    count: usize,
) -> Result<Vec<Record>, String> {
    let mut raw = Decoder { bytes, at: 0 };
    if raw.array::<8>()? != *MAGIC
        || raw.array::<32>()? != parent
        || raw.array::<32>()? != pin
        || Policy::decode(raw.take(Policy::BYTE_LEN)?).map_err(display)? != policy
        || raw.usize()? != count
    {
        return Err("index consistency version, parent, policy or coordinate count differs".into());
    }
    let minimum = BINDING_BYTES
        .checked_add(Evaluation::BYTE_LEN * 3)
        .and_then(|n| n.checked_add(40))
        .ok_or("index record size overflow")?;
    if count > (bytes.len() - raw.at) / minimum {
        return Err("index consistency record count exceeds body".into());
    }
    let mut records = Vec::new();
    records.try_reserve_exact(count).map_err(display)?;
    for _ in 0..count {
        let binding = Binding::decode(&mut raw)?;
        let encoded = [
            raw.take(Evaluation::BYTE_LEN)?,
            raw.take(Evaluation::BYTE_LEN)?,
            raw.take(Evaluation::BYTE_LEN)?,
        ];
        let weeks_counts = [raw.usize()?, raw.usize()?, raw.usize()?];
        let weeks_count = weeks_counts
            .iter()
            .try_fold(0_usize, |n, count| n.checked_add(*count))
            .ok_or("index week count overflow")?;
        let days_count = raw.usize()?;
        let training_session_count = raw.usize()?;
        if training_session_count > days_count {
            return Err("index training day extent exceeds full span".into());
        }
        let required = weeks_count
            .checked_mul(Week::BYTE_LEN)
            .and_then(|n| {
                days_count
                    .checked_mul(Session::BYTE_LEN)
                    .and_then(|d| n.checked_add(d))
            })
            .ok_or("index consistency row size overflow")?;
        if required > bytes.len() - raw.at {
            return Err("index consistency day/week extent exceeds body".into());
        }
        let mut evaluations = Vec::new();
        for (count, bytes) in weeks_counts.into_iter().zip(encoded) {
            let mut weeks = Vec::new();
            weeks.try_reserve_exact(count).map_err(display)?;
            for _ in 0..count {
                weeks.push(Week::decode(raw.take(Week::BYTE_LEN)?, policy).map_err(display)?);
            }
            evaluations.push(Evaluation::decode(bytes, Some(&weeks)).map_err(display)?);
        }
        let mut sessions = Vec::new();
        sessions.try_reserve_exact(days_count).map_err(display)?;
        for _ in 0..days_count {
            sessions.push(Session::decode(raw.take(Session::BYTE_LEN)?).map_err(display)?);
        }
        let [evaluation, training, later]: [Evaluation; 3] = evaluations
            .try_into()
            .map_err(|_| "index assessment period count differs")?;
        let training_days = sessions
            .get(..training_session_count)
            .ok_or("index training day extent missing")?;
        let later_days = sessions
            .get(training_session_count..)
            .ok_or("index later day extent missing")?;
        if evaluation.sessions_digest != crate::index_consistency::session_digest(&sessions)
            || training.sessions_digest != crate::index_consistency::session_digest(training_days)
            || later.sessions_digest != crate::index_consistency::session_digest(later_days)
            || training.family != evaluation.family
            || later.family != evaluation.family
            || training.first_day != evaluation.first_day
            || later.last_day != evaluation.last_day
            || training.last_day >= later.first_day
        {
            return Err("index consistency daily rows differ from evaluation digest".into());
        }
        records.push(Record {
            binding,
            evaluation,
            training,
            later,
            training_session_count,
            sessions,
        });
    }
    if raw.at != bytes.len() {
        return Err("index consistency trailing bytes refused".into());
    }
    Ok(records)
}
struct Decoder<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Decoder<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(count)
            .ok_or("index consistency decoder overflow")?;
        let value = self
            .bytes
            .get(self.at..end)
            .ok_or("index consistency field missing")?;
        self.at = end;
        Ok(value)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        self.take(N)?.try_into().map_err(display)
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn usize(&mut self) -> Result<usize, String> {
        usize::try_from(self.u64()?).map_err(display)
    }
}
fn display(value: impl std::fmt::Display) -> String {
    value.to_string()
}

#[cfg(test)]
#[path = "index_consistency_store_tests.rs"]
mod tests;
