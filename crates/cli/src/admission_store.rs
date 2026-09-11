//! Receipt-last authority for institutional admission decisions.
//!
//! A population row historically carried a caller-authored admission status.
//! That status is useful as an audit claim, but it cannot authorize ranking:
//! the durable bytes did not carry the policy and evidence from which the
//! verdict was computed.  This module adds two append-only sidecars without
//! changing any population or selection byte already on disk.
//!
//! `population-admission-v1.bin` stores one exact evidence/verdict pair beside
//! the population, row and strategy identities it evaluates.
//! `population-admission-completions-v1.bin` is appended only after all of
//! those decisions are durable.  Its receipt carries the exact policy,
//! terminal-status counts and ordered decision digest.  Reopen decodes every
//! canonical admission value and recomputes every verdict under that policy.
//! A decision block without its receipt remains prepared but invisible.
//!
//! # Cost
//!
//! Open is O(all admission decisions + completion receipts) time and builds
//! indexes proportional to the number of populations.  Reconciliation reads
//! every committed decision once more because the receipt-last policy is not
//! known while the decision file is first scanned.  A committed lookup is one
//! average-O(1) hash probe plus one fixed seek/read; a page is O(requested)
//! with a fixed 256-row ceiling.  Committing is O(decisions) time.  None of the
//! open, page, population commit, hashing or persistence work is claimed O(1).
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::hash::Hash;
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use runner::admission::{
    ADMISSION_EVIDENCE_CANONICAL_LEN_V1, ADMISSION_POLICY_CANONICAL_LEN_V1,
    ADMISSION_VERDICT_CANONICAL_LEN_V1, AdmissionDecisionSealV1, AdmissionEvidenceV1,
    AdmissionPolicyV1, AdmissionStatusV1, AdmissionVerdictV1,
};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt as _;

/// Operator-facing refusal from the admission sidecar.
pub type AdmissionStoreRefusal = String;

const DECISION_MAGIC: [u8; 8] = *b"BRUTXAD1";
const COMPLETION_MAGIC: [u8; 8] = *b"BRUTXAC1";
const FORMAT_VERSION_V1: u32 = 1;
const HEADER: u64 = 16;
const HEADER_BYTES: usize = 16;
const DECISION_PAYLOAD_BYTES: usize = 512;
const COMPLETION_PAYLOAD_BYTES: usize = 448;
const ORDERED_DECISION_DOMAIN: &[u8] = b"brutex-population-admission-decisions-v1\0";
const COMPLETION_DIGEST_DOMAIN: &[u8] = b"brutex-population-admission-completion-v1\0";

/// Bytes in one complete admission-decision record.
pub const ADMISSION_DECISION_STRIDE: u64 = 544;
/// [`ADMISSION_DECISION_STRIDE`] as a machine-sized constant.
pub const ADMISSION_DECISION_STRIDE_BYTES: usize = 544;
/// Bytes in one complete admission-completion receipt.
pub const ADMISSION_COMPLETION_STRIDE: u64 = 480;
/// [`ADMISSION_COMPLETION_STRIDE`] as a machine-sized constant.
pub const ADMISSION_COMPLETION_STRIDE_BYTES: usize = 480;
/// Hard ceiling for one decoded admission page.
pub const MAX_ADMISSION_PAGE_ROWS_V1: u64 = 256;

/// One population row's recomputable admission evidence and verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmissionDecisionRecordV1 {
    population_id: [u8; 32],
    row_sequence: u64,
    strategy_digest: [u8; 32],
    row_payload_digest: [u8; 32],
    evidence: AdmissionEvidenceV1,
    verdict: AdmissionVerdictV1,
}

impl AdmissionDecisionRecordV1 {
    /// Binds a policy-computed decision to one exact population row.
    ///
    /// # Errors
    ///
    /// Refuses an all-zero population, strategy or row-payload digest.
    pub fn new(
        population_id: [u8; 32],
        row_sequence: u64,
        strategy_digest: [u8; 32],
        row_payload_digest: [u8; 32],
        decision: &AdmissionDecisionSealV1,
    ) -> Result<Self, AdmissionStoreRefusal> {
        require_nonzero_digest("population identity", &population_id)?;
        require_nonzero_digest("strategy digest", &strategy_digest)?;
        require_nonzero_digest("population row payload digest", &row_payload_digest)?;
        if !decision.recomputes() {
            return Err(
                "admission decision does not recompute from its policy and evidence".into(),
            );
        }
        Ok(Self {
            population_id,
            row_sequence,
            strategy_digest,
            row_payload_digest,
            evidence: decision.evidence(),
            verdict: decision.verdict(),
        })
    }

    /// Population completed by this decision's receipt.
    #[must_use]
    pub const fn population_id(&self) -> [u8; 32] {
        self.population_id
    }

    /// Exact zero-based row sequence within the population.
    #[must_use]
    pub const fn row_sequence(&self) -> u64 {
        self.row_sequence
    }

    /// Exact durable strategy identity.
    #[must_use]
    pub const fn strategy_digest(&self) -> [u8; 32] {
        self.strategy_digest
    }

    /// Digest of the exact canonical population-row payload.
    #[must_use]
    pub const fn row_payload_digest(&self) -> [u8; 32] {
        self.row_payload_digest
    }

    /// Validated evidence stored for policy recomputation.
    #[must_use]
    pub const fn evidence(&self) -> &AdmissionEvidenceV1 {
        &self.evidence
    }

    /// Persisted verdict, authoritative only after receipt reconciliation.
    #[must_use]
    pub const fn verdict(&self) -> &AdmissionVerdictV1 {
        &self.verdict
    }

    /// Fixed V1 bytes including the complete BLAKE3 payload seal.
    ///
    /// # Errors
    ///
    /// Refuses only an internal fixed-layout length mismatch.
    pub fn to_bytes(&self) -> Result<[u8; ADMISSION_DECISION_STRIDE_BYTES], AdmissionStoreRefusal> {
        let payload = self.payload_bytes()?;
        let mut raw = [0_u8; ADMISSION_DECISION_STRIDE_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&payload)?;
        encoder.bytes(&brutex_core::blake3::hash(&payload))?;
        encoder.finish()?;
        Ok(raw)
    }

    /// Decodes and validates one exact fixed V1 decision record.
    ///
    /// The evidence and verdict are individually canonical here.  Their
    /// relationship to the policy is re-authorized only when the receipt-last
    /// completion is reconciled.
    ///
    /// # Errors
    ///
    /// Refuses length, seal, reserved bytes, absent identities, or any nested
    /// canonical admission refusal.
    pub fn from_bytes(raw: &[u8]) -> Result<Self, AdmissionStoreRefusal> {
        require_len(raw, ADMISSION_DECISION_STRIDE_BYTES, "admission decision")?;
        let payload = raw
            .get(..DECISION_PAYLOAD_BYTES)
            .ok_or_else(|| "admission decision payload is absent".to_owned())?;
        let stored_seal = raw
            .get(DECISION_PAYLOAD_BYTES..)
            .ok_or_else(|| "admission decision seal is absent".to_owned())?;
        if stored_seal != brutex_core::blake3::hash(payload) {
            return Err("admission decision failed its complete BLAKE3 seal".into());
        }
        let mut decoder = Decoder::new(payload);
        let population_id = decoder.array::<32>()?;
        let row_sequence = decoder.u64()?;
        let strategy_digest = decoder.array::<32>()?;
        let row_payload_digest = decoder.array::<32>()?;
        let evidence_bytes = decoder.array::<ADMISSION_EVIDENCE_CANONICAL_LEN_V1>()?;
        let verdict_bytes = decoder.array::<ADMISSION_VERDICT_CANONICAL_LEN_V1>()?;
        decoder.zeros(11, "admission decision reserve")?;
        decoder.finish()?;
        require_nonzero_digest("population identity", &population_id)?;
        require_nonzero_digest("strategy digest", &strategy_digest)?;
        require_nonzero_digest("population row payload digest", &row_payload_digest)?;
        let evidence = AdmissionEvidenceV1::from_canonical_bytes(&evidence_bytes)
            .map_err(|why| format!("admission decision evidence is invalid: {why:?}"))?;
        let verdict = AdmissionVerdictV1::from_canonical_bytes(&verdict_bytes)
            .map_err(|why| format!("admission decision verdict is invalid: {why:?}"))?;
        Ok(Self {
            population_id,
            row_sequence,
            strategy_digest,
            row_payload_digest,
            evidence,
            verdict,
        })
    }

    fn payload_bytes(&self) -> Result<[u8; DECISION_PAYLOAD_BYTES], AdmissionStoreRefusal> {
        let mut payload = [0_u8; DECISION_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut payload);
        encoder.bytes(&self.population_id)?;
        encoder.u64(self.row_sequence)?;
        encoder.bytes(&self.strategy_digest)?;
        encoder.bytes(&self.row_payload_digest)?;
        encoder.bytes(&self.evidence.canonical_bytes())?;
        encoder.bytes(&self.verdict.canonical_bytes())?;
        encoder.zeros(11)?;
        encoder.finish()?;
        Ok(payload)
    }
}

/// Receipt-last proof that every decision for one population was authorized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmissionCompletionReceiptV1 {
    population_id: [u8; 32],
    population_v4_completion_digest: [u8; 32],
    decision_count: u64,
    admitted_count: u64,
    rejected_count: u64,
    unmeasured_count: u64,
    refused_count: u64,
    ordered_decision_digest: [u8; 32],
    policy: AdmissionPolicyV1,
}

impl AdmissionCompletionReceiptV1 {
    /// Derives every count and the ordered decision digest from exact records.
    ///
    /// # Errors
    ///
    /// Refuses absent identities, non-contiguous row sequences, a decision for
    /// another population, a duplicate strategy, count overflow, or a verdict
    /// that differs from fresh evaluation under `policy`.
    pub fn derive(
        population_id: [u8; 32],
        population_v4_completion_digest: [u8; 32],
        policy: &AdmissionPolicyV1,
        decisions: &[AdmissionDecisionRecordV1],
    ) -> Result<Self, AdmissionStoreRefusal> {
        require_nonzero_digest("population identity", &population_id)?;
        require_nonzero_digest(
            "population V4 completion digest",
            &population_v4_completion_digest,
        )?;
        let decision_count = u64::try_from(decisions.len())
            .map_err(|_| "admission decision count does not fit u64".to_owned())?;
        let mut counters = StatusCounters::new(decision_count);
        let mut ordered = OrderedDecisionDigest::new(decision_count);
        let mut strategies = HashSet::new();
        strategies.try_reserve(decisions.len()).map_err(|why| {
            format!(
                "admission strategy uniqueness set could not reserve {} slots: {why}",
                decisions.len()
            )
        })?;
        for (index, decision) in decisions.iter().enumerate() {
            let expected_sequence = u64::try_from(index)
                .map_err(|_| "admission decision sequence does not fit u64".to_owned())?;
            validate_decision_binding(decision, population_id, expected_sequence, policy)?;
            if !strategies.insert(decision.strategy_digest) {
                return Err(format!(
                    "population {} repeats strategy {} in its admission block",
                    hex(&population_id),
                    hex(&decision.strategy_digest)
                ));
            }
            let raw = decision.to_bytes()?;
            ordered.push(&raw);
            counters.push(decision.verdict.status())?;
        }
        counters.finish()?;
        Ok(Self {
            population_id,
            population_v4_completion_digest,
            decision_count,
            admitted_count: counters.admitted,
            rejected_count: counters.rejected,
            unmeasured_count: counters.unmeasured,
            refused_count: counters.refused,
            ordered_decision_digest: ordered.finish(),
            policy: *policy,
        })
    }

    /// Population made authoritative by this receipt.
    #[must_use]
    pub const fn population_id(&self) -> [u8; 32] {
        self.population_id
    }

    /// Digest of the exact V4 population completion this admission follows.
    #[must_use]
    pub const fn population_v4_completion_digest(&self) -> [u8; 32] {
        self.population_v4_completion_digest
    }

    /// Number of decision records committed before this receipt.
    #[must_use]
    pub const fn decision_count(&self) -> u64 {
        self.decision_count
    }

    /// Number of admitted decisions.
    #[must_use]
    pub const fn admitted_count(&self) -> u64 {
        self.admitted_count
    }

    /// Number of rejected decisions.
    #[must_use]
    pub const fn rejected_count(&self) -> u64 {
        self.rejected_count
    }

    /// Number of decisions with unmeasured required evidence.
    #[must_use]
    pub const fn unmeasured_count(&self) -> u64 {
        self.unmeasured_count
    }

    /// Number of decisions refused by an upstream authority.
    #[must_use]
    pub const fn refused_count(&self) -> u64 {
        self.refused_count
    }

    /// Digest over exact decision records in row-sequence order.
    #[must_use]
    pub const fn ordered_decision_digest(&self) -> [u8; 32] {
        self.ordered_decision_digest
    }

    /// Exact runtime admission policy used to recompute every verdict.
    #[must_use]
    pub const fn policy(&self) -> &AdmissionPolicyV1 {
        &self.policy
    }

    /// Domain-separated digest Selection V3 can bind without copying policy.
    ///
    /// # Errors
    ///
    /// Refuses an internal fixed-layout length mismatch instead of hashing a
    /// partial completion payload.
    pub fn digest(&self) -> Result<[u8; 32], AdmissionStoreRefusal> {
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(COMPLETION_DIGEST_DOMAIN);
        hasher.update(&self.payload_bytes()?);
        Ok(hasher.finalize())
    }

    /// Fixed V1 bytes including the complete BLAKE3 payload seal.
    ///
    /// # Errors
    ///
    /// Refuses only an internal fixed-layout length mismatch.
    pub fn to_bytes(
        &self,
    ) -> Result<[u8; ADMISSION_COMPLETION_STRIDE_BYTES], AdmissionStoreRefusal> {
        let payload = self.payload_bytes()?;
        let mut raw = [0_u8; ADMISSION_COMPLETION_STRIDE_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&payload)?;
        encoder.bytes(&brutex_core::blake3::hash(&payload))?;
        encoder.finish()?;
        Ok(raw)
    }

    /// Decodes and validates one exact fixed V1 completion receipt.
    ///
    /// # Errors
    ///
    /// Refuses length, seal, reserved bytes, absent identities, count
    /// disagreement, or an invalid canonical policy.
    pub fn from_bytes(raw: &[u8]) -> Result<Self, AdmissionStoreRefusal> {
        require_len(
            raw,
            ADMISSION_COMPLETION_STRIDE_BYTES,
            "admission completion",
        )?;
        let payload = raw
            .get(..COMPLETION_PAYLOAD_BYTES)
            .ok_or_else(|| "admission completion payload is absent".to_owned())?;
        let stored_seal = raw
            .get(COMPLETION_PAYLOAD_BYTES..)
            .ok_or_else(|| "admission completion seal is absent".to_owned())?;
        if stored_seal != brutex_core::blake3::hash(payload) {
            return Err("admission completion failed its complete BLAKE3 seal".into());
        }
        let mut decoder = Decoder::new(payload);
        let population_id = decoder.array::<32>()?;
        let population_v4_completion_digest = decoder.array::<32>()?;
        let decision_count = decoder.u64()?;
        let admitted_count = decoder.u64()?;
        let rejected_count = decoder.u64()?;
        let unmeasured_count = decoder.u64()?;
        let refused_count = decoder.u64()?;
        let ordered_decision_digest = decoder.array::<32>()?;
        let policy_bytes = decoder.array::<ADMISSION_POLICY_CANONICAL_LEN_V1>()?;
        decoder.zeros(2, "admission completion reserve")?;
        decoder.finish()?;
        require_nonzero_digest("population identity", &population_id)?;
        require_nonzero_digest(
            "population V4 completion digest",
            &population_v4_completion_digest,
        )?;
        require_nonzero_digest(
            "ordered admission decision digest",
            &ordered_decision_digest,
        )?;
        let classified = checked_sum(
            &[
                admitted_count,
                rejected_count,
                unmeasured_count,
                refused_count,
            ],
            "admission terminal status counts",
        )?;
        if classified != decision_count {
            return Err(format!(
                "admission completion declares {decision_count} decisions but its terminal status counts sum to {classified}"
            ));
        }
        let policy = AdmissionPolicyV1::from_canonical_bytes(&policy_bytes)
            .map_err(|why| format!("admission completion policy is invalid: {why:?}"))?;
        Ok(Self {
            population_id,
            population_v4_completion_digest,
            decision_count,
            admitted_count,
            rejected_count,
            unmeasured_count,
            refused_count,
            ordered_decision_digest,
            policy,
        })
    }

    fn payload_bytes(&self) -> Result<[u8; COMPLETION_PAYLOAD_BYTES], AdmissionStoreRefusal> {
        let mut payload = [0_u8; COMPLETION_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut payload);
        encoder.bytes(&self.population_id)?;
        encoder.bytes(&self.population_v4_completion_digest)?;
        encoder.u64(self.decision_count)?;
        encoder.u64(self.admitted_count)?;
        encoder.u64(self.rejected_count)?;
        encoder.u64(self.unmeasured_count)?;
        encoder.u64(self.refused_count)?;
        encoder.bytes(&self.ordered_decision_digest)?;
        encoder.bytes(&self.policy.canonical_bytes())?;
        encoder.zeros(2)?;
        encoder.finish()?;
        Ok(payload)
    }
}

/// Whether one commit appended new authority or reused byte-identical history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdmissionCommitOutcomeV1 {
    /// New decisions and/or their receipt were durably appended.
    Appended(AdmissionCompletionReceiptV1),
    /// The exact durable authority already existed and was byte-verified.
    Reused(AdmissionCompletionReceiptV1),
}

impl AdmissionCommitOutcomeV1 {
    /// Receipt proved by either outcome.
    #[must_use]
    pub const fn receipt(&self) -> &AdmissionCompletionReceiptV1 {
        match self {
            Self::Appended(receipt) | Self::Reused(receipt) => receipt,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DecisionBlock {
    first: u64,
    count: u64,
}

#[cfg(any(unix, windows))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    len: u64,
    platform: PlatformGeneration,
}

#[cfg(not(any(unix, windows)))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    len: u64,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration {
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration {
    volume_serial: u32,
    file_index: u64,
    creation_time: u64,
    last_write_time: u64,
}

/// Admission sidecar files and their complete receipt-last indexes.
#[derive(Debug)]
pub struct AdmissionAuthorityLedger {
    decision_file: File,
    completion_file: File,
    writer_lock: File,
    decision_path: PathBuf,
    completion_path: PathBuf,
    blocks: HashMap<[u8; 32], DecisionBlock>,
    completions: HashMap<[u8; 32], AdmissionCompletionReceiptV1>,
    trailing_orphan: Option<[u8; 32]>,
    decision_generation: FileGeneration,
    completion_generation: FileGeneration,
    writable: bool,
}

impl AdmissionAuthorityLedger {
    /// Decision sidecar path beside population results.
    #[must_use]
    pub fn decision_path(root: &Path) -> PathBuf {
        root.join("results").join("population-admission-v1.bin")
    }

    /// Receipt-last completion sidecar path beside population results.
    #[must_use]
    pub fn completion_path(root: &Path) -> PathBuf {
        root.join("results")
            .join("population-admission-completions-v1.bin")
    }

    fn lock_path(root: &Path) -> PathBuf {
        root.join("results").join("population-write.lock")
    }

    /// Opens existing sidecars without creating any path.
    ///
    /// # Errors
    ///
    /// Refuses absent files, invalid headers, ragged tails, corrupt records,
    /// interleaved populations, or any completion that does not re-authorize
    /// its complete decision block.
    pub fn open_read(root: &Path) -> Result<Self, AdmissionStoreRefusal> {
        Self::open_existing(root, None)
    }

    /// Opens existing sidecars only when each file fits `max_bytes`.
    ///
    /// The size gate occurs before either file is scanned.
    ///
    /// # Errors
    ///
    /// Every refusal from [`Self::open_read`], plus either file exceeding the
    /// supplied bound.
    pub fn open_read_bounded(root: &Path, max_bytes: u64) -> Result<Self, AdmissionStoreRefusal> {
        Self::open_existing(root, Some(max_bytes))
    }

    /// Opens for append, creating only absent headers and the shared lock.
    ///
    /// # Errors
    ///
    /// Refuses directory, lock, header, schema, integrity or reconciliation
    /// failures. Existing bytes are never replaced.
    pub fn open(root: &Path) -> Result<Self, AdmissionStoreRefusal> {
        let directory = root.join("results");
        std::fs::create_dir_all(&directory)
            .map_err(|why| format!("the results directory could not be made: {why}"))?;
        let lock_path = Self::lock_path(root);
        let writer_lock = open_or_create(&lock_path)?;
        writer_lock
            .lock()
            .map_err(|why| format!("{} could not be locked: {why}", lock_path.display()))?;
        let opened = (|| {
            let decision_path = Self::decision_path(root);
            let completion_path = Self::completion_path(root);
            let mut decision_file = open_or_create(&decision_path)?;
            let mut completion_file = open_or_create(&completion_path)?;
            ensure_header(
                &mut decision_file,
                &decision_path,
                DECISION_MAGIC,
                FORMAT_VERSION_V1,
            )?;
            ensure_header(
                &mut completion_file,
                &completion_path,
                COMPLETION_MAGIC,
                FORMAT_VERSION_V1,
            )?;
            sync_directory(&directory)?;
            Self::from_files(
                decision_file,
                completion_file,
                writer_lock.try_clone().map_err(|why| {
                    format!(
                        "{} lock handle could not be cloned: {why}",
                        lock_path.display()
                    )
                })?,
                decision_path,
                completion_path,
                None,
                true,
            )
        })();
        release_lock(&writer_lock, &lock_path, opened)
    }

    fn open_existing(root: &Path, max_bytes: Option<u64>) -> Result<Self, AdmissionStoreRefusal> {
        let lock_path = Self::lock_path(root);
        let writer_lock = File::open(&lock_path)
            .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
        writer_lock
            .lock_shared()
            .map_err(|why| format!("{} could not be shared-locked: {why}", lock_path.display()))?;
        let opened = (|| {
            let decision_path = Self::decision_path(root);
            let completion_path = Self::completion_path(root);
            let decision_file = File::open(&decision_path)
                .map_err(|why| format!("{} could not be opened: {why}", decision_path.display()))?;
            let completion_file = File::open(&completion_path).map_err(|why| {
                format!("{} could not be opened: {why}", completion_path.display())
            })?;
            Self::from_files(
                decision_file,
                completion_file,
                writer_lock.try_clone().map_err(|why| {
                    format!(
                        "{} lock handle could not be cloned: {why}",
                        lock_path.display()
                    )
                })?,
                decision_path,
                completion_path,
                max_bytes,
                false,
            )
        })();
        release_lock(&writer_lock, &lock_path, opened)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one ordered open transaction validates both sidecars and their receipt-last relationship before constructing reusable authority"
    )]
    fn from_files(
        mut decision_file: File,
        mut completion_file: File,
        writer_lock: File,
        decision_path: PathBuf,
        completion_path: PathBuf,
        max_bytes: Option<u64>,
        writable: bool,
    ) -> Result<Self, AdmissionStoreRefusal> {
        let decision_len = measured_len(&decision_file, &decision_path)?;
        let completion_len = measured_len(&completion_file, &completion_path)?;
        if let Some(maximum) = max_bytes {
            require_bounded(&decision_path, decision_len, maximum)?;
            require_bounded(&completion_path, completion_len, maximum)?;
        }
        check_header(
            &mut decision_file,
            &decision_path,
            decision_len,
            DECISION_MAGIC,
            FORMAT_VERSION_V1,
            ADMISSION_DECISION_STRIDE,
        )?;
        check_header(
            &mut completion_file,
            &completion_path,
            completion_len,
            COMPLETION_MAGIC,
            FORMAT_VERSION_V1,
            ADMISSION_COMPLETION_STRIDE,
        )?;
        let DecisionScan { blocks, order } =
            scan_decisions(&mut decision_file, &decision_path, decision_len)?;
        let completions = scan_completions(&mut completion_file, &completion_path, completion_len)?;
        reconcile_all(&mut decision_file, &decision_path, &blocks, &completions)?;
        let trailing_orphan = find_trailing_orphan(&order, &completions)?;
        let decision_generation = validated_generation(
            &decision_file,
            &decision_path,
            decision_len,
            "decision file",
        )?;
        let completion_generation = validated_generation(
            &completion_file,
            &completion_path,
            completion_len,
            "completion file",
        )?;
        Ok(Self {
            decision_file,
            completion_file,
            writer_lock,
            decision_path,
            completion_path,
            blocks,
            completions,
            trailing_orphan,
            decision_generation,
            completion_generation,
            writable,
        })
    }

    /// Returns one receipt only while both indexed files remain unchanged.
    ///
    /// # Errors
    ///
    /// Refuses any replacement or same-length mutation after open.
    pub fn completion(
        &self,
        population_id: [u8; 32],
    ) -> Result<Option<AdmissionCompletionReceiptV1>, AdmissionStoreRefusal> {
        self.writer_lock.lock_shared().map_err(|why| {
            format!(
                "{} could not be shared-locked for admission lookup: {why}",
                self.completion_path.display()
            )
        })?;
        let result = self
            .require_generations_unchanged()
            .map(|()| self.completions.get(&population_id).cloned());
        release_locked_result(&self.writer_lock, &self.completion_path, result)
    }

    /// Reads one committed decision by exact zero-based row sequence.
    ///
    /// # Errors
    ///
    /// Refuses stale files, arithmetic overflow, or any newly invalid record.
    pub fn decision(
        &mut self,
        population_id: [u8; 32],
        row_sequence: u64,
    ) -> Result<Option<AdmissionDecisionRecordV1>, AdmissionStoreRefusal> {
        self.writer_lock.lock_shared().map_err(|why| {
            format!(
                "{} could not be shared-locked for admission decision lookup: {why}",
                self.decision_path.display()
            )
        })?;
        let result = (|| {
            self.require_generations_unchanged()?;
            let Some(receipt) = self.completions.get(&population_id) else {
                return Ok(None);
            };
            if row_sequence >= receipt.decision_count {
                return Ok(None);
            }
            let block = self.blocks.get(&population_id).ok_or_else(|| {
                format!(
                    "committed admission population {} has no decision block",
                    hex(&population_id)
                )
            })?;
            read_decision_in_block(
                &mut self.decision_file,
                &self.decision_path,
                *block,
                row_sequence,
            )
            .map(Some)
        })();
        release_locked_result(&self.writer_lock, &self.decision_path, result)
    }

    /// Reads a bounded contiguous page only from a committed population.
    ///
    /// Zero `limit` is valid. A missing population returns `None`.
    ///
    /// # Errors
    ///
    /// Refuses a limit above [`MAX_ADMISSION_PAGE_ROWS_V1`], stale files,
    /// overflow, allocation failure, or any newly invalid record.
    pub fn page(
        &mut self,
        population_id: [u8; 32],
        offset: u64,
        limit: u64,
    ) -> Result<Option<Vec<AdmissionDecisionRecordV1>>, AdmissionStoreRefusal> {
        if limit > MAX_ADMISSION_PAGE_ROWS_V1 {
            return Err(format!(
                "admission page limit {limit} exceeds the fixed {MAX_ADMISSION_PAGE_ROWS_V1}-row ceiling"
            ));
        }
        self.writer_lock.lock_shared().map_err(|why| {
            format!(
                "{} could not be shared-locked for admission page: {why}",
                self.decision_path.display()
            )
        })?;
        let result = (|| {
            self.require_generations_unchanged()?;
            let Some(receipt) = self.completions.get(&population_id) else {
                return Ok(None);
            };
            let available = receipt.decision_count.saturating_sub(offset);
            let count = available.min(limit);
            let capacity = usize::try_from(count)
                .map_err(|_| "bounded admission page count does not fit usize".to_owned())?;
            let mut page = Vec::new();
            page.try_reserve_exact(capacity).map_err(|why| {
                format!("bounded admission page could not reserve {capacity} rows: {why}")
            })?;
            if count == 0 {
                return Ok(Some(page));
            }
            let block = self.blocks.get(&population_id).ok_or_else(|| {
                format!(
                    "committed admission population {} has no decision block",
                    hex(&population_id)
                )
            })?;
            for relative in 0..count {
                let sequence = offset
                    .checked_add(relative)
                    .ok_or_else(|| "admission page sequence overflowed u64".to_owned())?;
                page.push(read_decision_in_block(
                    &mut self.decision_file,
                    &self.decision_path,
                    *block,
                    sequence,
                )?);
            }
            Ok(Some(page))
        })();
        release_locked_result(&self.writer_lock, &self.decision_path, result)
    }

    /// Appends decisions, syncs them, and appends their completion last.
    ///
    /// Exact reruns are byte-verified and reused. A prepared trailing block
    /// from a crash must be completed exactly before another population can be
    /// appended.
    ///
    /// # Errors
    ///
    /// Refuses a read-only handle, stale generation, conflicting rerun,
    /// non-contiguous or forged decisions, an unrelated trailing orphan, or
    /// any write/sync/rollback failure.
    pub fn commit(
        &mut self,
        population_id: [u8; 32],
        population_v4_completion_digest: [u8; 32],
        policy: &AdmissionPolicyV1,
        decisions: &[AdmissionDecisionRecordV1],
    ) -> Result<AdmissionCommitOutcomeV1, AdmissionStoreRefusal> {
        if !self.writable {
            return Err("read-only admission ledger cannot append".into());
        }
        self.writer_lock.lock().map_err(|why| {
            format!(
                "{} could not be locked for admission append: {why}",
                self.decision_path.display()
            )
        })?;
        let result = self.commit_locked(
            population_id,
            population_v4_completion_digest,
            policy,
            decisions,
        );
        release_locked_result(&self.writer_lock, &self.decision_path, result)
    }

    fn commit_locked(
        &mut self,
        population_id: [u8; 32],
        population_v4_completion_digest: [u8; 32],
        policy: &AdmissionPolicyV1,
        decisions: &[AdmissionDecisionRecordV1],
    ) -> Result<AdmissionCommitOutcomeV1, AdmissionStoreRefusal> {
        self.require_generations_unchanged()?;
        let receipt = AdmissionCompletionReceiptV1::derive(
            population_id,
            population_v4_completion_digest,
            policy,
            decisions,
        )?;
        if let Some(held) = self.completions.get(&population_id).cloned() {
            if held != receipt {
                return Err(format!(
                    "population {} already has a different admission completion; append-only history was preserved",
                    hex(&population_id)
                ));
            }
            self.verify_supplied_decisions(population_id, decisions)?;
            return Ok(AdmissionCommitOutcomeV1::Reused(held));
        }
        if let Some(orphan) = self.trailing_orphan {
            if orphan != population_id {
                return Err(format!(
                    "prepared admission population {} must be recovered before population {} can append",
                    hex(&orphan),
                    hex(&population_id)
                ));
            }
            self.verify_supplied_decisions(population_id, decisions)?;
        } else if decisions.is_empty() {
            if self.blocks.contains_key(&population_id) {
                return Err(format!(
                    "population {} has stored admission decisions but the rerun supplied none",
                    hex(&population_id)
                ));
            }
        } else {
            self.append_decisions(population_id, decisions)?;
        }
        self.append_completion(&receipt)?;
        self.completions.insert(population_id, receipt.clone());
        self.trailing_orphan = None;
        Ok(AdmissionCommitOutcomeV1::Appended(receipt))
    }

    fn append_decisions(
        &mut self,
        population_id: [u8; 32],
        decisions: &[AdmissionDecisionRecordV1],
    ) -> Result<(), AdmissionStoreRefusal> {
        let at = self.decision_generation.len;
        self.decision_file
            .seek(SeekFrom::Start(at))
            .map_err(|why| format!("{} append seek failed: {why}", self.decision_path.display()))?;
        for decision in decisions {
            let raw = decision.to_bytes()?;
            if let Err(why) = self.decision_file.write_all(&raw) {
                return Err(rollback_message(
                    &self.decision_file,
                    at,
                    "admission decision block",
                    &why,
                ));
            }
        }
        if let Err(why) = self.decision_file.sync_all() {
            return Err(rollback_message(
                &self.decision_file,
                at,
                "admission decision block sync",
                &why,
            ));
        }
        self.decision_generation = file_generation(&self.decision_file, &self.decision_path)?;
        let count = u64::try_from(decisions.len())
            .map_err(|_| "admission decision count does not fit u64".to_owned())?;
        self.blocks
            .insert(population_id, DecisionBlock { first: at, count });
        self.trailing_orphan = Some(population_id);
        Ok(())
    }

    fn append_completion(
        &mut self,
        receipt: &AdmissionCompletionReceiptV1,
    ) -> Result<(), AdmissionStoreRefusal> {
        let at = self.completion_generation.len;
        let raw = receipt.to_bytes()?;
        self.completion_file
            .seek(SeekFrom::Start(at))
            .and_then(|_| self.completion_file.write_all(&raw))
            .and_then(|()| self.completion_file.sync_all())
            .map_err(|why| {
                rollback_message(
                    &self.completion_file,
                    at,
                    "admission completion receipt",
                    &why,
                )
            })?;
        self.completion_generation = file_generation(&self.completion_file, &self.completion_path)?;
        Ok(())
    }

    fn verify_supplied_decisions(
        &mut self,
        population_id: [u8; 32],
        decisions: &[AdmissionDecisionRecordV1],
    ) -> Result<(), AdmissionStoreRefusal> {
        if decisions.is_empty() {
            if self.blocks.contains_key(&population_id) {
                return Err(format!(
                    "population {} stored a non-empty admission block but the rerun supplied none",
                    hex(&population_id)
                ));
            }
            return Ok(());
        }
        let block = self.blocks.get(&population_id).copied().ok_or_else(|| {
            format!(
                "population {} has no stored admission block to reuse",
                hex(&population_id)
            )
        })?;
        let supplied_count = u64::try_from(decisions.len())
            .map_err(|_| "admission decision count does not fit u64".to_owned())?;
        if block.count != supplied_count {
            return Err(format!(
                "population {} stored {} admission decisions but the rerun supplied {supplied_count}",
                hex(&population_id),
                block.count
            ));
        }
        for (index, supplied) in decisions.iter().enumerate() {
            let sequence = u64::try_from(index)
                .map_err(|_| "admission decision sequence does not fit u64".to_owned())?;
            let held = read_decision_in_block(
                &mut self.decision_file,
                &self.decision_path,
                block,
                sequence,
            )?;
            if held != *supplied {
                return Err(format!(
                    "population {} admission decision {sequence} differs from its prepared bytes",
                    hex(&population_id)
                ));
            }
        }
        Ok(())
    }

    fn require_generations_unchanged(&self) -> Result<(), AdmissionStoreRefusal> {
        require_generation_unchanged(
            self.decision_generation,
            file_generation(&self.decision_file, &self.decision_path)?,
            &self.decision_path,
            "admission decision file",
        )?;
        require_generation_unchanged(
            self.completion_generation,
            file_generation(&self.completion_file, &self.completion_path)?,
            &self.completion_path,
            "admission completion file",
        )
    }
}

fn validate_decision_binding(
    decision: &AdmissionDecisionRecordV1,
    population_id: [u8; 32],
    expected_sequence: u64,
    policy: &AdmissionPolicyV1,
) -> Result<(), AdmissionStoreRefusal> {
    if decision.population_id != population_id {
        return Err(format!(
            "admission decision {expected_sequence} names population {}, expected {}",
            hex(&decision.population_id),
            hex(&population_id)
        ));
    }
    if decision.row_sequence != expected_sequence {
        return Err(format!(
            "admission row sequence {} is not expected contiguous sequence {expected_sequence}",
            decision.row_sequence
        ));
    }
    let policy_bytes = policy.canonical_bytes();
    let evidence_bytes = decision.evidence.canonical_bytes();
    let verdict_bytes = decision.verdict.canonical_bytes();
    AdmissionDecisionSealV1::from_canonical_parts(
        &policy_bytes,
        &evidence_bytes,
        &verdict_bytes,
    )
    .map_err(|why| {
        format!(
            "admission decision {expected_sequence} is not authorized by its completion policy: {why:?}"
        )
    })?;
    Ok(())
}

#[derive(Clone, Copy)]
struct StatusCounters {
    total: u64,
    seen: u64,
    admitted: u64,
    rejected: u64,
    unmeasured: u64,
    refused: u64,
}

impl StatusCounters {
    const fn new(total: u64) -> Self {
        Self {
            total,
            seen: 0,
            admitted: 0,
            rejected: 0,
            unmeasured: 0,
            refused: 0,
        }
    }

    fn push(&mut self, status: AdmissionStatusV1) -> Result<(), AdmissionStoreRefusal> {
        self.seen = self
            .seen
            .checked_add(1)
            .ok_or_else(|| "admission decision counter overflowed u64".to_owned())?;
        let target = match status {
            AdmissionStatusV1::Admitted => &mut self.admitted,
            AdmissionStatusV1::Rejected => &mut self.rejected,
            AdmissionStatusV1::Unmeasured => &mut self.unmeasured,
            AdmissionStatusV1::Refused => &mut self.refused,
        };
        *target = target
            .checked_add(1)
            .ok_or_else(|| "admission status counter overflowed u64".to_owned())?;
        Ok(())
    }

    fn finish(&self) -> Result<(), AdmissionStoreRefusal> {
        if self.seen == self.total {
            Ok(())
        } else {
            Err(format!(
                "admission counter saw {} decisions, expected {}",
                self.seen, self.total
            ))
        }
    }
}

struct OrderedDecisionDigest {
    hasher: brutex_core::blake3::Hasher,
}

impl OrderedDecisionDigest {
    fn new(decision_count: u64) -> Self {
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(ORDERED_DECISION_DOMAIN);
        hasher.update(&decision_count.to_le_bytes());
        Self { hasher }
    }

    fn push(&mut self, raw: &[u8; ADMISSION_DECISION_STRIDE_BYTES]) {
        self.hasher.update(raw);
    }

    fn finish(self) -> [u8; 32] {
        self.hasher.finalize()
    }
}

struct DecisionScan {
    blocks: HashMap<[u8; 32], DecisionBlock>,
    order: Vec<[u8; 32]>,
}

fn scan_decisions(
    file: &mut File,
    path: &Path,
    len: u64,
) -> Result<DecisionScan, AdmissionStoreRefusal> {
    let record_count = len.saturating_sub(HEADER) / ADMISSION_DECISION_STRIDE;
    let capacity = usize::try_from(record_count)
        .map_err(|_| format!("{} decision count does not fit usize", path.display()))?;
    let mut blocks = HashMap::new();
    reserve_map(&mut blocks, capacity, "admission decision block index")?;
    let mut order = Vec::new();
    order
        .try_reserve(capacity)
        .map_err(|why| format!("admission block order could not reserve {capacity}: {why}"))?;
    let mut seen = HashSet::new();
    seen.try_reserve(capacity)
        .map_err(|why| format!("admission population set could not reserve {capacity}: {why}"))?;
    let mut current: Option<([u8; 32], DecisionBlock, HashSet<[u8; 32]>)> = None;
    let mut at = HEADER;
    while at < len {
        let decision = read_decision_at(file, path, at)?;
        let changes_population = current
            .as_ref()
            .is_none_or(|(identity, _, _)| *identity != decision.population_id);
        if changes_population {
            if let Some((identity, block, _)) = current.take() {
                blocks.insert(identity, block);
            }
            if !seen.insert(decision.population_id) {
                return Err(format!(
                    "{} interleaves admission population {}; a block must be contiguous",
                    path.display(),
                    hex(&decision.population_id)
                ));
            }
            order.push(decision.population_id);
            current = Some((
                decision.population_id,
                DecisionBlock {
                    first: at,
                    count: 0,
                },
                HashSet::new(),
            ));
        }
        let (_, block, strategies) = current
            .as_mut()
            .ok_or_else(|| "admission decision scan lost its current block".to_owned())?;
        if decision.row_sequence != block.count {
            return Err(format!(
                "{} admission population {} row sequence {} is not contiguous sequence {}",
                path.display(),
                hex(&decision.population_id),
                decision.row_sequence,
                block.count
            ));
        }
        if !strategies.insert(decision.strategy_digest) {
            return Err(format!(
                "{} admission population {} repeats strategy {}",
                path.display(),
                hex(&decision.population_id),
                hex(&decision.strategy_digest)
            ));
        }
        block.count = block
            .count
            .checked_add(1)
            .ok_or_else(|| "admission decision block count overflowed u64".to_owned())?;
        at = at
            .checked_add(ADMISSION_DECISION_STRIDE)
            .ok_or_else(|| "admission decision scan offset overflowed u64".to_owned())?;
    }
    if let Some((identity, block, _)) = current {
        blocks.insert(identity, block);
    }
    Ok(DecisionScan { blocks, order })
}

fn scan_completions(
    file: &mut File,
    path: &Path,
    len: u64,
) -> Result<HashMap<[u8; 32], AdmissionCompletionReceiptV1>, AdmissionStoreRefusal> {
    let record_count = len.saturating_sub(HEADER) / ADMISSION_COMPLETION_STRIDE;
    let capacity = usize::try_from(record_count)
        .map_err(|_| format!("{} completion count does not fit usize", path.display()))?;
    let mut completions = HashMap::new();
    reserve_map(&mut completions, capacity, "admission completion index")?;
    let mut at = HEADER;
    while at < len {
        let receipt = read_completion_at(file, path, at)?;
        let population_id = receipt.population_id;
        if completions.insert(population_id, receipt).is_some() {
            return Err(format!(
                "{} repeats admission completion for population {}",
                path.display(),
                hex(&population_id)
            ));
        }
        at = at
            .checked_add(ADMISSION_COMPLETION_STRIDE)
            .ok_or_else(|| "admission completion scan offset overflowed u64".to_owned())?;
    }
    Ok(completions)
}

fn reconcile_all(
    file: &mut File,
    path: &Path,
    blocks: &HashMap<[u8; 32], DecisionBlock>,
    completions: &HashMap<[u8; 32], AdmissionCompletionReceiptV1>,
) -> Result<(), AdmissionStoreRefusal> {
    for (population_id, receipt) in completions {
        match (receipt.decision_count, blocks.get(population_id)) {
            (0, None) => validate_empty_receipt(receipt)?,
            (0, Some(_)) => {
                return Err(format!(
                    "population {} has stored admission decisions but its receipt declares zero",
                    hex(population_id)
                ));
            }
            (_, None) => {
                return Err(format!(
                    "population {} has an admission receipt without its decision block",
                    hex(population_id)
                ));
            }
            (_, Some(block)) => validate_receipt_against_block(file, path, *block, receipt)?,
        }
    }
    Ok(())
}

fn validate_empty_receipt(
    receipt: &AdmissionCompletionReceiptV1,
) -> Result<(), AdmissionStoreRefusal> {
    let expected = AdmissionCompletionReceiptV1::derive(
        receipt.population_id,
        receipt.population_v4_completion_digest,
        &receipt.policy,
        &[],
    )?;
    if &expected == receipt {
        Ok(())
    } else {
        Err(format!(
            "empty admission population {} does not match its derived receipt",
            hex(&receipt.population_id)
        ))
    }
}

fn validate_receipt_against_block(
    file: &mut File,
    path: &Path,
    block: DecisionBlock,
    receipt: &AdmissionCompletionReceiptV1,
) -> Result<(), AdmissionStoreRefusal> {
    if block.count != receipt.decision_count {
        return Err(format!(
            "population {} stores {} admission decisions but its receipt declares {}",
            hex(&receipt.population_id),
            block.count,
            receipt.decision_count
        ));
    }
    let mut counters = StatusCounters::new(block.count);
    let mut ordered = OrderedDecisionDigest::new(block.count);
    for sequence in 0..block.count {
        let decision = read_decision_in_block(file, path, block, sequence)?;
        validate_decision_binding(&decision, receipt.population_id, sequence, &receipt.policy)?;
        counters.push(decision.verdict.status())?;
        ordered.push(&decision.to_bytes()?);
    }
    counters.finish()?;
    let expected = (
        counters.admitted,
        counters.rejected,
        counters.unmeasured,
        counters.refused,
        ordered.finish(),
    );
    let stored = (
        receipt.admitted_count,
        receipt.rejected_count,
        receipt.unmeasured_count,
        receipt.refused_count,
        receipt.ordered_decision_digest,
    );
    if expected == stored {
        Ok(())
    } else {
        Err(format!(
            "population {} admission receipt does not match its ordered decisions",
            hex(&receipt.population_id)
        ))
    }
}

fn find_trailing_orphan(
    order: &[[u8; 32]],
    completions: &HashMap<[u8; 32], AdmissionCompletionReceiptV1>,
) -> Result<Option<[u8; 32]>, AdmissionStoreRefusal> {
    let mut orphan = None;
    for (index, population_id) in order.iter().enumerate() {
        if completions.contains_key(population_id) {
            continue;
        }
        if orphan.is_some() || index.saturating_add(1) != order.len() {
            return Err(format!(
                "uncommitted admission population {} is not the sole trailing prepared block",
                hex(population_id)
            ));
        }
        orphan = Some(*population_id);
    }
    Ok(orphan)
}

fn read_decision_in_block(
    file: &mut File,
    path: &Path,
    block: DecisionBlock,
    sequence: u64,
) -> Result<AdmissionDecisionRecordV1, AdmissionStoreRefusal> {
    let relative = sequence
        .checked_mul(ADMISSION_DECISION_STRIDE)
        .ok_or_else(|| "admission decision offset multiplication overflowed u64".to_owned())?;
    let at = block
        .first
        .checked_add(relative)
        .ok_or_else(|| "admission decision offset addition overflowed u64".to_owned())?;
    read_decision_at(file, path, at)
}

fn read_decision_at(
    file: &mut File,
    path: &Path,
    at: u64,
) -> Result<AdmissionDecisionRecordV1, AdmissionStoreRefusal> {
    let mut raw = [0_u8; ADMISSION_DECISION_STRIDE_BYTES];
    file.seek(SeekFrom::Start(at))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| {
            format!(
                "{} admission decision at byte {at} could not be read: {why}",
                path.display()
            )
        })?;
    AdmissionDecisionRecordV1::from_bytes(&raw).map_err(|why| {
        format!(
            "{} admission decision at byte {at} is invalid: {why}",
            path.display()
        )
    })
}

fn read_completion_at(
    file: &mut File,
    path: &Path,
    at: u64,
) -> Result<AdmissionCompletionReceiptV1, AdmissionStoreRefusal> {
    let mut raw = [0_u8; ADMISSION_COMPLETION_STRIDE_BYTES];
    file.seek(SeekFrom::Start(at))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| {
            format!(
                "{} admission completion at byte {at} could not be read: {why}",
                path.display()
            )
        })?;
    AdmissionCompletionReceiptV1::from_bytes(&raw).map_err(|why| {
        format!(
            "{} admission completion at byte {at} is invalid: {why}",
            path.display()
        )
    })
}

fn open_or_create(path: &Path) -> Result<File, AdmissionStoreRefusal> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
}

fn ensure_header(
    file: &mut File,
    path: &Path,
    magic: [u8; 8],
    version: u32,
) -> Result<(), AdmissionStoreRefusal> {
    let len = measured_len(file, path)?;
    if len != 0 {
        return Ok(());
    }
    let mut header = [0_u8; HEADER_BYTES];
    let mut encoder = Encoder::new(&mut header);
    encoder.bytes(&magic)?;
    encoder.u32(version)?;
    encoder.zeros(4)?;
    encoder.finish()?;
    file.write_all(&header)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} could not receive a durable header: {why}",
                path.display()
            )
        })
}

fn check_header(
    file: &mut File,
    path: &Path,
    len: u64,
    magic: [u8; 8],
    expected_version: u32,
    stride: u64,
) -> Result<(), AdmissionStoreRefusal> {
    if len < HEADER {
        return Err(format!(
            "{} has length {len}, shorter than its {HEADER}-byte admission header",
            path.display()
        ));
    }
    let mut header = [0_u8; HEADER_BYTES];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut header))
        .map_err(|why| format!("{} header could not be read: {why}", path.display()))?;
    if header.get(..8) != Some(&magic) {
        return Err(format!(
            "{} has the wrong admission-file magic; no record was trusted",
            path.display()
        ));
    }
    let version = header
        .get(8..12)
        .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
        .map_or(0, u32::from_le_bytes);
    if version != expected_version {
        return Err(format!(
            "{} is admission format version {version}; this path requires version {expected_version}",
            path.display()
        ));
    }
    if header.get(12..16) != Some(&[0_u8; 4]) {
        return Err(format!(
            "{} has non-zero reserved header bytes; their meaning is unknown",
            path.display()
        ));
    }
    if !len.saturating_sub(HEADER).is_multiple_of(stride) {
        return Err(format!(
            "{} has length {len}, not a {HEADER}-byte header plus whole {stride}-byte records; a torn/ragged tail is never ignored",
            path.display()
        ));
    }
    Ok(())
}

fn require_bounded(path: &Path, len: u64, maximum: u64) -> Result<(), AdmissionStoreRefusal> {
    if len <= maximum {
        Ok(())
    } else {
        Err(format!(
            "{} is {len} bytes; this bounded admission reader accepts at most {maximum} before indexing",
            path.display()
        ))
    }
}

fn measured_len(file: &File, path: &Path) -> Result<u64, AdmissionStoreRefusal> {
    file.metadata()
        .map(|metadata| metadata.len())
        .map_err(|why| format!("{} could not be measured: {why}", path.display()))
}

fn validated_generation(
    file: &File,
    path: &Path,
    expected_len: u64,
    kind: &str,
) -> Result<FileGeneration, AdmissionStoreRefusal> {
    let generation = file_generation(file, path)?;
    if generation.len != expected_len {
        return Err(format!(
            "{} {kind} changed length from validated byte {expected_len} to {}",
            path.display(),
            generation.len
        ));
    }
    Ok(generation)
}

fn file_generation(file: &File, path: &Path) -> Result<FileGeneration, AdmissionStoreRefusal> {
    let held = file
        .metadata()
        .map_err(|why| format!("{} open file could not be measured: {why}", path.display()))?;
    let named = std::fs::metadata(path)
        .map_err(|why| format!("{} path could not be measured: {why}", path.display()))?;
    platform_generation(&held, &named, path)
}

#[cfg(unix)]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, AdmissionStoreRefusal> {
    let held = FileGeneration {
        len: held.len(),
        platform: PlatformGeneration {
            device: held.dev(),
            inode: held.ino(),
            modified_seconds: held.mtime(),
            modified_nanoseconds: held.mtime_nsec(),
            changed_seconds: held.ctime(),
            changed_nanoseconds: held.ctime_nsec(),
        },
    };
    let named = FileGeneration {
        len: named.len(),
        platform: PlatformGeneration {
            device: named.dev(),
            inode: named.ino(),
            modified_seconds: named.mtime(),
            modified_nanoseconds: named.mtime_nsec(),
            changed_seconds: named.ctime(),
            changed_nanoseconds: named.ctime_nsec(),
        },
    };
    if (held.platform.device, held.platform.inode) != (named.platform.device, named.platform.inode)
    {
        return Err(format!(
            "{} no longer names the opened admission file; a replacement was refused",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its filesystem generation was measured",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(windows)]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, AdmissionStoreRefusal> {
    fn of(
        metadata: &std::fs::Metadata,
        path: &Path,
    ) -> Result<FileGeneration, AdmissionStoreRefusal> {
        let volume_serial = metadata
            .volume_serial_number()
            .ok_or_else(|| format!("{} has no Windows volume serial", path.display()))?;
        let file_index = metadata
            .file_index()
            .ok_or_else(|| format!("{} has no Windows file index", path.display()))?;
        Ok(FileGeneration {
            len: metadata.len(),
            platform: PlatformGeneration {
                volume_serial,
                file_index,
                creation_time: metadata.creation_time(),
                last_write_time: metadata.last_write_time(),
            },
        })
    }
    let held = of(held, path)?;
    let named = of(named, path)?;
    if (held.platform.volume_serial, held.platform.file_index)
        != (named.platform.volume_serial, named.platform.file_index)
    {
        return Err(format!(
            "{} no longer names the opened admission file; a replacement was refused",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its filesystem generation was measured",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(not(any(unix, windows)))]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, AdmissionStoreRefusal> {
    if held.len() != named.len() {
        return Err(format!(
            "{} changed length while its filesystem generation was measured",
            path.display()
        ));
    }
    Ok(FileGeneration { len: held.len() })
}

#[cfg(any(unix, windows))]
fn require_generation_unchanged(
    expected: FileGeneration,
    observed: FileGeneration,
    path: &Path,
    kind: &str,
) -> Result<(), AdmissionStoreRefusal> {
    if expected == observed {
        Ok(())
    } else {
        Err(format!(
            "{} {kind} kept length {} but its validated filesystem generation changed; reopen before reuse",
            path.display(),
            observed.len
        ))
    }
}

#[cfg(not(any(unix, windows)))]
fn require_generation_unchanged(
    _expected: FileGeneration,
    observed: FileGeneration,
    path: &Path,
    kind: &str,
) -> Result<(), AdmissionStoreRefusal> {
    Err(format!(
        "{} {kind} is {} bytes, but this target has no stable file identity; cached authority fails closed",
        path.display(),
        observed.len
    ))
}

fn sync_directory(path: &Path) -> Result<(), AdmissionStoreRefusal> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|why| {
            format!(
                "{} could not durably confirm admission file names: {why}",
                path.display()
            )
        })
}

fn rollback_message(file: &File, at: u64, subject: &str, why: &std::io::Error) -> String {
    match file.set_len(at) {
        Ok(()) => format!(
            "the {subject} could not be written: {why}. Partial bytes were rolled back to byte {at}"
        ),
        Err(and) => format!(
            "the {subject} could not be written: {why}. Rolling back to byte {at} also failed: {and}; the tail is refused"
        ),
    }
}

fn release_lock<T>(
    lock: &File,
    path: &Path,
    result: Result<T, AdmissionStoreRefusal>,
) -> Result<T, AdmissionStoreRefusal> {
    release_locked_result(lock, path, result)
}

fn release_locked_result<T>(
    lock: &File,
    path: &Path,
    result: Result<T, AdmissionStoreRefusal>,
) -> Result<T, AdmissionStoreRefusal> {
    let released = lock
        .unlock()
        .map_err(|why| format!("{} could not be unlocked: {why}", path.display()));
    match (result, released) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (Ok(_), Err(why)) => Err(why),
    }
}

fn require_nonzero_digest(name: &str, digest: &[u8; 32]) -> Result<(), AdmissionStoreRefusal> {
    if digest.iter().all(|byte| *byte == 0) {
        Err(format!("{name} is absent (all zero); no default exists"))
    } else {
        Ok(())
    }
}

fn checked_sum(values: &[u64], subject: &str) -> Result<u64, AdmissionStoreRefusal> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| format!("{subject} overflowed u64"))
    })
}

fn require_len(raw: &[u8], expected: usize, subject: &str) -> Result<(), AdmissionStoreRefusal> {
    if raw.len() == expected {
        Ok(())
    } else {
        Err(format!(
            "{subject} is {} bytes; V1 requires exactly {expected}",
            raw.len()
        ))
    }
}

fn reserve_map<K: Eq + Hash, V>(
    map: &mut HashMap<K, V>,
    additional: usize,
    subject: &str,
) -> Result<(), AdmissionStoreRefusal> {
    map.try_reserve(additional)
        .map_err(|why| format!("{subject} could not reserve {additional} slots: {why}"))
}

fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

struct Encoder<'a> {
    raw: &'a mut [u8],
    offset: usize,
}

impl<'a> Encoder<'a> {
    const fn new(raw: &'a mut [u8]) -> Self {
        Self { raw, offset: 0 }
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), AdmissionStoreRefusal> {
        let end = self
            .offset
            .checked_add(value.len())
            .ok_or_else(|| "admission encoder offset overflowed usize".to_owned())?;
        let target = self
            .raw
            .get_mut(self.offset..end)
            .ok_or_else(|| "admission encoder exceeded its fixed record".to_owned())?;
        target.copy_from_slice(value);
        self.offset = end;
        Ok(())
    }

    fn u32(&mut self, value: u32) -> Result<(), AdmissionStoreRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), AdmissionStoreRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), AdmissionStoreRefusal> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| "admission encoder reserve overflowed usize".to_owned())?;
        let target = self
            .raw
            .get_mut(self.offset..end)
            .ok_or_else(|| "admission encoder reserve exceeded its fixed record".to_owned())?;
        target.fill(0);
        self.offset = end;
        Ok(())
    }

    fn finish(self) -> Result<(), AdmissionStoreRefusal> {
        if self.offset == self.raw.len() {
            Ok(())
        } else {
            Err(format!(
                "admission encoder wrote {} of {} fixed bytes",
                self.offset,
                self.raw.len()
            ))
        }
    }
}

struct Decoder<'a> {
    raw: &'a [u8],
    offset: usize,
}

impl<'a> Decoder<'a> {
    const fn new(raw: &'a [u8]) -> Self {
        Self { raw, offset: 0 }
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], AdmissionStoreRefusal> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or_else(|| "admission decoder offset overflowed usize".to_owned())?;
        let bytes = self
            .raw
            .get(self.offset..end)
            .ok_or_else(|| "admission decoder exceeded its fixed record".to_owned())?;
        let out = <[u8; N]>::try_from(bytes)
            .map_err(|_| "admission decoder fixed array had the wrong length".to_owned())?;
        self.offset = end;
        Ok(out)
    }

    fn u64(&mut self) -> Result<u64, AdmissionStoreRefusal> {
        self.array::<8>().map(u64::from_le_bytes)
    }

    fn zeros(&mut self, count: usize, subject: &str) -> Result<(), AdmissionStoreRefusal> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or_else(|| "admission decoder reserve overflowed usize".to_owned())?;
        let bytes = self
            .raw
            .get(self.offset..end)
            .ok_or_else(|| "admission decoder reserve exceeded its fixed record".to_owned())?;
        if bytes.iter().any(|byte| *byte != 0) {
            return Err(format!("{subject} contains a non-zero byte"));
        }
        self.offset = end;
        Ok(())
    }

    fn finish(self) -> Result<(), AdmissionStoreRefusal> {
        if self.offset == self.raw.len() {
            Ok(())
        } else {
            Err(format!(
                "admission decoder consumed {} of {} fixed bytes",
                self.offset,
                self.raw.len()
            ))
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::panic,
    reason = "fixed-codec fixtures must fail loudly and mutate exact adversarial byte offsets"
)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::{Seek, SeekFrom, Write};
    use std::sync::atomic::{AtomicU64, Ordering};

    use runner::admission::{
        AdmissionEvidenceV1, AdmissionEvidenceValuesV1, AdmissionPolicyDraftV1, AdmissionPolicyV1,
        CompletenessV1, HypothesisDecisionV1, ObservedI64V1, ObservedU64V1,
    };

    use super::{
        ADMISSION_COMPLETION_STRIDE_BYTES, ADMISSION_DECISION_STRIDE_BYTES,
        AdmissionAuthorityLedger, AdmissionCommitOutcomeV1, AdmissionCompletionReceiptV1,
        AdmissionDecisionRecordV1, COMPLETION_PAYLOAD_BYTES, DECISION_PAYLOAD_BYTES, HEADER,
        MAX_ADMISSION_PAGE_ROWS_V1,
    };

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    fn root(name: &str) -> std::path::PathBuf {
        let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "brutex-admission-store-{}-{name}-{sequence}",
            std::process::id()
        ))
    }

    fn digest(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    fn policy() -> AdmissionPolicyV1 {
        AdmissionPolicyV1::new(AdmissionPolicyDraftV1 {
            min_support_hits: Some(100),
            min_independent_sessions: Some(20),
            min_trades: Some(30),
            max_mae_paisa: Some(500),
            min_worst_reward_risk_ppm: Some(2_000_000),
            min_win_rate_ppm: Some(600_000),
            min_wilson_win_rate_ppm: Some(550_000),
            min_return_drawdown_ppm: Some(3_000_000),
            min_weakest_period_return_paisa: Some(0),
            max_pbo_ppm: Some(100_000),
            max_fwer_p_value_ppm: Some(50_000),
            max_spa_p_value_ppm: Some(50_000),
            min_decided_folds: Some(10),
            max_ambiguous_fill_rate_ppm: Some(100_000),
            max_gap_affected_rate_ppm: Some(100_000),
            max_session_concentration_ppm: Some(300_000),
            max_largest_trade_profit_share_ppm: Some(200_000),
            max_drawdown_paisa: Some(10_000),
            max_worst_trade_loss_paisa: Some(2_000),
            max_losing_trade_rate_ppm: Some(400_000),
            max_losing_trades: Some(20),
            min_pessimistic_profit_paisa: Some(1_000),
            min_winning_trades: Some(30),
            min_average_win_paisa: Some(300),
            max_average_loss_paisa: Some(150),
            min_profit_factor_ppm: Some(2_000_000),
            max_consecutive_losing_streak: Some(3),
            min_consecutive_winning_streak: Some(3),
            min_bootstrap_draws: Some(1_000),
            min_bootstrap_strategies: Some(100),
            min_bootstrap_periods: Some(300),
            min_pbo_contributing_folds: Some(10),
            max_pbo_unrankable_folds: Some(2),
            min_profitable_oos_folds: Some(8),
            min_oos_pessimistic_return_paisa: Some(1_000),
            max_white_reality_p_value_ppm: Some(50_000),
            require_white_reality_rejection: Some(true),
            max_romano_wolf_p_value_ppm: Some(50_000),
            require_romano_wolf_rejection: Some(true),
        })
        .expect("complete admission policy")
    }

    fn truthful_v1_values() -> AdmissionEvidenceValuesV1 {
        AdmissionEvidenceValuesV1 {
            support_hits: ObservedU64V1::Measured(100),
            independent_sessions: ObservedU64V1::Measured(20),
            trades: ObservedU64V1::Measured(50),
            max_mae_paisa: ObservedU64V1::Measured(500),
            worst_reward_risk_ppm: ObservedU64V1::Measured(2_000_000),
            win_rate_ppm: ObservedU64V1::Measured(600_000),
            wilson_win_rate_ppm: ObservedU64V1::Measured(550_000),
            return_drawdown_ppm: ObservedU64V1::Measured(3_000_000),
            weakest_period_return_paisa: ObservedI64V1::Measured(0),
            pbo_ppm: ObservedU64V1::Unmeasured,
            fwer_p_value_ppm: ObservedU64V1::Measured(50_000),
            spa_p_value_ppm: ObservedU64V1::Measured(50_000),
            decided_folds: ObservedU64V1::Measured(10),
            ambiguous_fill_rate_ppm: ObservedU64V1::Measured(100_000),
            gap_affected_rate_ppm: ObservedU64V1::Measured(100_000),
            session_concentration_ppm: ObservedU64V1::Measured(300_000),
            largest_trade_profit_share_ppm: ObservedU64V1::Measured(200_000),
            execution_complete: CompletenessV1::Complete,
            data_complete: CompletenessV1::Complete,
            calendar_complete: CompletenessV1::Complete,
            population_complete: CompletenessV1::Complete,
            drawdown_paisa: ObservedU64V1::Measured(10_000),
            worst_trade_loss_paisa: ObservedU64V1::Measured(2_000),
            losing_trade_rate_ppm: ObservedU64V1::Measured(400_000),
            losing_trades: ObservedU64V1::Measured(20),
            pessimistic_profit_paisa: ObservedI64V1::Measured(1_000),
            winning_trades: ObservedU64V1::Measured(30),
            average_win_paisa: ObservedU64V1::Measured(300),
            average_loss_paisa: ObservedU64V1::Measured(150),
            profit_factor_ppm: ObservedU64V1::Measured(2_000_000),
            consecutive_losing_streak: ObservedU64V1::Measured(3),
            consecutive_winning_streak: ObservedU64V1::Measured(3),
            bootstrap_draws: ObservedU64V1::Measured(1_000),
            bootstrap_strategies: ObservedU64V1::Measured(100),
            bootstrap_periods: ObservedU64V1::Measured(300),
            pbo_contributing_folds: ObservedU64V1::Unmeasured,
            pbo_unrankable_folds: ObservedU64V1::Unmeasured,
            profitable_oos_folds: ObservedU64V1::Measured(8),
            oos_pessimistic_return_paisa: ObservedI64V1::Measured(1_000),
            white_reality_p_value_ppm: ObservedU64V1::Measured(50_000),
            romano_wolf_p_value_ppm: ObservedU64V1::Measured(50_000),
            white_reality_decision: HypothesisDecisionV1::RejectedNull,
            romano_wolf_decision: HypothesisDecisionV1::RejectedNull,
            full_precision_statistics_complete: CompletenessV1::Complete,
        }
    }

    fn decision(population_id: [u8; 32], sequence: u64, status: u8) -> AdmissionDecisionRecordV1 {
        let mut values = truthful_v1_values();
        values.support_hits = match status {
            0 => ObservedU64V1::Measured(100),
            1 => ObservedU64V1::Measured(99),
            2 => ObservedU64V1::Unmeasured,
            _ => ObservedU64V1::Refused,
        };
        let evidence = AdmissionEvidenceV1::new(values).expect("in-domain evidence");
        let policy = policy();
        let sealed = policy.evaluate_sealed(&evidence);
        let sequence_byte = u8::try_from(sequence.saturating_add(20)).expect("small sequence");
        AdmissionDecisionRecordV1::new(
            population_id,
            sequence,
            digest(sequence_byte),
            digest(sequence_byte.saturating_add(40)),
            &sealed,
        )
        .expect("bound admission decision")
    }

    fn four_decisions(population_id: [u8; 32]) -> Vec<AdmissionDecisionRecordV1> {
        (0_u8..4)
            .map(|status| decision(population_id, u64::from(status), status))
            .collect()
    }

    fn reseal(raw: &mut [u8], payload_len: usize) {
        let seal = brutex_core::blake3::hash(&raw[..payload_len]);
        raw[payload_len..].copy_from_slice(&seal);
    }

    fn cleanup(root: &std::path::Path) {
        std::fs::remove_dir_all(root).expect("fixture cleanup");
    }

    #[test]
    fn exact_codecs_bind_truthful_v1_states_and_refuse_reserves_and_seals() {
        let population_id = digest(1);
        let policy = policy();
        let decisions = four_decisions(population_id);
        let receipt =
            AdmissionCompletionReceiptV1::derive(population_id, digest(2), &policy, &decisions)
                .expect("derived receipt");
        assert_eq!(receipt.decision_count(), 4);
        assert_eq!(receipt.admitted_count(), 0);
        assert_eq!(receipt.rejected_count(), 0);
        assert_eq!(receipt.unmeasured_count(), 3);
        assert_eq!(receipt.refused_count(), 1);
        assert_ne!(receipt.digest().expect("completion digest"), [0; 32]);

        let decision_raw = decisions[0].to_bytes().expect("decision bytes");
        assert_eq!(decision_raw.len(), ADMISSION_DECISION_STRIDE_BYTES);
        assert_eq!(
            AdmissionDecisionRecordV1::from_bytes(&decision_raw).expect("decision round trip"),
            decisions[0]
        );
        let receipt_raw = receipt.to_bytes().expect("receipt bytes");
        assert_eq!(receipt_raw.len(), ADMISSION_COMPLETION_STRIDE_BYTES);
        assert_eq!(
            AdmissionCompletionReceiptV1::from_bytes(&receipt_raw).expect("receipt round trip"),
            receipt
        );

        let mut bad_decision_seal = decision_raw;
        bad_decision_seal[DECISION_PAYLOAD_BYTES] ^= 1;
        assert!(
            AdmissionDecisionRecordV1::from_bytes(&bad_decision_seal)
                .expect_err("decision seal mutation")
                .contains("seal")
        );
        let mut decision_reserve = decision_raw;
        decision_reserve[501] = 1;
        reseal(&mut decision_reserve, DECISION_PAYLOAD_BYTES);
        assert!(
            AdmissionDecisionRecordV1::from_bytes(&decision_reserve)
                .expect_err("decision reserve mutation")
                .contains("reserve")
        );

        let mut completion_seal = receipt_raw;
        completion_seal[COMPLETION_PAYLOAD_BYTES] ^= 1;
        assert!(
            AdmissionCompletionReceiptV1::from_bytes(&completion_seal)
                .expect_err("completion seal mutation")
                .contains("seal")
        );
        let mut completion_reserve = receipt_raw;
        completion_reserve[446] = 1;
        reseal(&mut completion_reserve, COMPLETION_PAYLOAD_BYTES);
        assert!(
            AdmissionCompletionReceiptV1::from_bytes(&completion_reserve)
                .expect_err("completion reserve mutation")
                .contains("reserve")
        );
        let mut wrong_count = receipt_raw;
        wrong_count[64..72].copy_from_slice(&5_u64.to_le_bytes());
        reseal(&mut wrong_count, COMPLETION_PAYLOAD_BYTES);
        assert!(
            AdmissionCompletionReceiptV1::from_bytes(&wrong_count)
                .expect_err("count mismatch")
                .contains("sum")
        );
    }

    #[test]
    fn private_counter_kernel_covers_all_four_terminal_statuses() {
        // Durable Admission V1 evidence cannot truthfully produce Admitted or
        // Rejected while its PBO authority is absent.  The fixed counter
        // algebra is therefore covered directly inside this private unit-test
        // module instead of persisting fabricated V1 evidence.
        let mut counters = super::StatusCounters::new(4);
        for status in [
            runner::admission::AdmissionStatusV1::Admitted,
            runner::admission::AdmissionStatusV1::Rejected,
            runner::admission::AdmissionStatusV1::Unmeasured,
            runner::admission::AdmissionStatusV1::Refused,
        ] {
            counters.push(status).expect("bounded status counter");
        }
        counters.finish().expect("all four statuses were counted");
        assert_eq!(counters.admitted, 1);
        assert_eq!(counters.rejected, 1);
        assert_eq!(counters.unmeasured, 1);
        assert_eq!(counters.refused, 1);
    }

    #[test]
    fn receipt_last_commit_reopens_pages_and_reuses_only_exact_bytes() {
        let root = root("commit-reopen");
        let population_id = digest(3);
        let policy = policy();
        let decisions = four_decisions(population_id);
        let mut writer = AdmissionAuthorityLedger::open(&root).expect("open writer");
        let appended = writer
            .commit(population_id, digest(4), &policy, &decisions)
            .expect("append authority");
        assert!(matches!(appended, AdmissionCommitOutcomeV1::Appended(_)));
        let reused = writer
            .commit(population_id, digest(4), &policy, &decisions)
            .expect("exact rerun");
        assert!(matches!(reused, AdmissionCommitOutcomeV1::Reused(_)));
        assert!(
            writer
                .commit(population_id, digest(5), &policy, &decisions)
                .expect_err("changed V4 digest")
                .contains("different admission completion")
        );
        drop(writer);

        let mut reader = AdmissionAuthorityLedger::open_read(&root).expect("reopen reader");
        let completion = reader
            .completion(population_id)
            .expect("completion lookup")
            .expect("committed completion");
        assert_eq!(completion.decision_count(), 4);
        assert_eq!(
            reader
                .decision(population_id, 2)
                .expect("decision lookup")
                .expect("decision exists"),
            decisions[2]
        );
        assert!(
            reader
                .decision(population_id, 4)
                .expect("past-end lookup")
                .is_none()
        );
        assert_eq!(
            reader
                .page(population_id, 1, 2)
                .expect("page lookup")
                .expect("population exists"),
            decisions[1..3]
        );
        assert!(
            reader
                .page(population_id, 0, MAX_ADMISSION_PAGE_ROWS_V1 + 1)
                .expect_err("unbounded page")
                .contains("ceiling")
        );
        assert!(
            AdmissionAuthorityLedger::open_read_bounded(&root, HEADER)
                .expect_err("bounded open")
                .contains("at most")
        );
        drop(reader);
        cleanup(&root);
    }

    #[test]
    fn a_prepared_orphan_is_hidden_and_only_an_exact_retry_can_complete_it() {
        let root = root("orphan");
        let population_id = digest(6);
        let policy = policy();
        let decisions = four_decisions(population_id);
        let ledger = AdmissionAuthorityLedger::open(&root).expect("create files");
        drop(ledger);

        let path = AdmissionAuthorityLedger::decision_path(&root);
        let mut raw_file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open raw decision sidecar");
        for decision in &decisions {
            raw_file
                .write_all(&decision.to_bytes().expect("decision bytes"))
                .expect("write orphan");
        }
        raw_file.sync_all().expect("durable orphan");
        drop(raw_file);

        let mut reopened = AdmissionAuthorityLedger::open(&root).expect("reopen orphan");
        assert!(
            reopened
                .completion(population_id)
                .expect("hidden completion")
                .is_none()
        );
        assert!(
            reopened
                .page(population_id, 0, 4)
                .expect("hidden page")
                .is_none()
        );
        let other_population = digest(7);
        let other_decisions = four_decisions(other_population);
        assert!(
            reopened
                .commit(other_population, digest(8), &policy, &other_decisions)
                .expect_err("unrelated append while orphan exists")
                .contains("must be recovered")
        );
        let mut changed = four_decisions(population_id);
        changed.swap(0, 1);
        assert!(
            reopened
                .commit(population_id, digest(8), &policy, &changed)
                .expect_err("changed orphan retry")
                .contains("row sequence")
        );
        reopened
            .commit(population_id, digest(8), &policy, &decisions)
            .expect("exact orphan recovery");
        assert!(
            reopened
                .completion(population_id)
                .expect("completion after recovery")
                .is_some()
        );
        drop(reopened);
        cleanup(&root);
    }

    #[test]
    fn reopen_refuses_forged_verdicts_ragged_tails_bad_headers_and_reserves() {
        let policy = policy();
        for mutation in 0_u8..4 {
            let root = root("reopen-adversary");
            let population_id = digest(mutation.saturating_add(20));
            let decisions = four_decisions(population_id);
            let mut writer = AdmissionAuthorityLedger::open(&root).expect("writer");
            writer
                .commit(population_id, digest(31), &policy, &decisions)
                .expect("commit fixture");
            drop(writer);
            match mutation {
                0 => {
                    let path = AdmissionAuthorityLedger::decision_path(&root);
                    let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(path)
                        .expect("decision adversary");
                    let rejected = decision(population_id, 0, 1).evidence().canonical_bytes();
                    file.seek(SeekFrom::Start(HEADER + 104))
                        .and_then(|_| file.write_all(&rejected))
                        .expect("forge evidence");
                    let mut raw = [0_u8; ADMISSION_DECISION_STRIDE_BYTES];
                    file.seek(SeekFrom::Start(HEADER))
                        .and_then(|_| std::io::Read::read_exact(&mut file, &mut raw))
                        .expect("read forged record");
                    reseal(&mut raw, DECISION_PAYLOAD_BYTES);
                    file.seek(SeekFrom::Start(HEADER))
                        .and_then(|_| file.write_all(&raw))
                        .and_then(|()| file.sync_all())
                        .expect("reseal forged record");
                    assert!(
                        AdmissionAuthorityLedger::open_read(&root)
                            .expect_err("forged verdict")
                            .contains("not authorized")
                    );
                }
                1 => {
                    let path = AdmissionAuthorityLedger::completion_path(&root);
                    OpenOptions::new()
                        .append(true)
                        .open(path)
                        .and_then(|mut file| file.write_all(&[1]))
                        .expect("ragged completion tail");
                    assert!(
                        AdmissionAuthorityLedger::open_read(&root)
                            .expect_err("ragged completion")
                            .contains("ragged")
                    );
                }
                2 => {
                    let path = AdmissionAuthorityLedger::decision_path(&root);
                    let mut file = OpenOptions::new()
                        .write(true)
                        .open(path)
                        .expect("header adversary");
                    file.seek(SeekFrom::Start(8))
                        .and_then(|_| file.write_all(&2_u32.to_le_bytes()))
                        .expect("wrong version");
                    assert!(
                        AdmissionAuthorityLedger::open_read(&root)
                            .expect_err("bad version")
                            .contains("version 2")
                    );
                }
                _ => {
                    let path = AdmissionAuthorityLedger::completion_path(&root);
                    let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(path)
                        .expect("completion adversary");
                    let mut raw = [0_u8; ADMISSION_COMPLETION_STRIDE_BYTES];
                    file.seek(SeekFrom::Start(HEADER))
                        .and_then(|_| std::io::Read::read_exact(&mut file, &mut raw))
                        .expect("read completion");
                    raw[446] = 1;
                    reseal(&mut raw, COMPLETION_PAYLOAD_BYTES);
                    file.seek(SeekFrom::Start(HEADER))
                        .and_then(|_| file.write_all(&raw))
                        .and_then(|()| file.sync_all())
                        .expect("write reserved completion");
                    assert!(
                        AdmissionAuthorityLedger::open_read(&root)
                            .expect_err("completion reserve")
                            .contains("reserve")
                    );
                }
            }
            cleanup(&root);
        }
    }

    #[test]
    fn stale_handles_refuse_length_and_same_length_generation_changes() {
        let root = root("stale");
        let population_id = digest(40);
        let policy = policy();
        let decisions = four_decisions(population_id);
        let mut stale = AdmissionAuthorityLedger::open(&root).expect("first writer");
        let mut current = AdmissionAuthorityLedger::open(&root).expect("second writer");
        current
            .commit(population_id, digest(41), &policy, &decisions)
            .expect("current commit");
        assert!(
            stale
                .commit(population_id, digest(41), &policy, &decisions)
                .expect_err("stale length")
                .contains("generation changed")
        );
        drop(stale);
        drop(current);

        let reader = AdmissionAuthorityLedger::open_read(&root).expect("fresh reader");
        let path = AdmissionAuthorityLedger::completion_path(&root);
        let mut external = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("same-length mutator");
        external
            .seek(SeekFrom::Start(HEADER))
            .and_then(|_| external.write_all(&[99]))
            .and_then(|()| external.sync_all())
            .expect("same-length mutation");
        assert!(
            reader
                .completion(population_id)
                .expect_err("stale same-length generation")
                .contains("generation changed")
        );
        drop(reader);
        cleanup(&root);
    }
}
