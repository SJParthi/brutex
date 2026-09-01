//! Durable receipt-last authority for same-pass Base Evidence V2.
//!
//! Records are the fixed 1,024-byte Phase-A records.  A separate fixed
//! completion is appended only after the contiguous record block is synced.
//! Opening scans and validates the bounded ledger once; after that, completion
//! lookup and a fixed-offset record lookup are O(1) in record count.  Opening,
//! append, exact retry comparison and fresh-reopen comparison are O(records).

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;

use brutex_core::blake3::Hasher;
use runner::admission::AdmissionEvidenceValuesV1;

use super::super::{CandidateUniverseReceiptV1, CandidateUniverseReopenAuditV1};
use super::{
    BASE_EVIDENCE_RECORD_BYTES_V2, BaseEvidenceRecordV2, PreparedBaseEvidenceV2,
    base_policy_digest_v2,
};
use crate::population::{InstrumentFamilyV1, RequestedSpanIdentityV1};

const HEADER_BYTES: usize = 64;
const HEADER_BYTES_U64: u64 = 64;
const COMPLETION_BYTES: usize = 512;
const COMPLETION_PAYLOAD_BYTES: usize = COMPLETION_BYTES - 32;
const COMPLETION_BYTES_U64: u64 = 512;
const RECORD_BYTES_U64: u64 = 1_024;
const HEADER_VERSION: u32 = 2;
const RECORD_KIND: u32 = 1;
const COMPLETION_KIND: u32 = 2;
const RECORD_MAGIC: [u8; 16] = *b"BTX-BASE-ROW-V2\0";
const COMPLETION_MAGIC: [u8; 16] = *b"BTX-BASE-CMP-V2\0";
const COMPLETION_RECORD_MAGIC: [u8; 16] = *b"BTX-BASE-DONEV2\0";
const HEADER_DOMAIN: &[u8] = b"brutex-base-evidence-v2-ledger-header\0";
const COMPLETION_ID_DOMAIN: &[u8] = b"brutex-base-evidence-v2-completion-id\0";
const COMPLETION_SEAL_DOMAIN: &[u8] = b"brutex-base-evidence-v2-completion-seal\0";
const PAIR_ID_DOMAIN: &[u8] = b"brutex-base-evidence-v2-nifty-banknifty-pair\0";
const RECORD_FILE: &str = "base-evidence-records-v2.bin";
const COMPLETION_FILE: &str = "base-evidence-completions-v2.bin";
const LOCK_FILE: &str = "base-evidence-write-v2.lock";

const _: () = assert!(BASE_EVIDENCE_RECORD_BYTES_V2 == 1_024);
const _: () = assert!(COMPLETION_PAYLOAD_BYTES + 32 == COMPLETION_BYTES);

/// Typed fail-closed refusal at the durable Base Evidence boundary.
#[derive(Debug)]
pub enum BaseEvidenceLedgerRefusalV2 {
    /// One explicit ledger ceiling was zero.
    InvalidBound(&'static str),
    /// A physical or requested count exceeded its explicit ceiling.
    BoundExceeded {
        /// Bounded resource.
        resource: &'static str,
        /// Observed count.
        actual: u64,
        /// Explicit ceiling.
        maximum: u64,
    },
    /// Fixed bytes, order, identity or retry semantics disagreed.
    Reconciliation(String),
    /// A fixed-stride record was malformed, corrupt or noncanonical.
    Codec(String),
    /// A filesystem operation failed or a named file was replaced.
    Io(String),
    /// A checked offset/count operation overflowed.
    Arithmetic(&'static str),
    /// A bounded index/page allocation failed.
    Allocation(String),
}

impl core::fmt::Display for BaseEvidenceLedgerRefusalV2 {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidBound(name) => {
                write!(formatter, "Base Evidence ledger {name} bound is zero")
            }
            Self::BoundExceeded {
                resource,
                actual,
                maximum,
            } => write!(
                formatter,
                "Base Evidence ledger {resource} count {actual} exceeds explicit maximum {maximum}"
            ),
            Self::Reconciliation(why) => write!(formatter, "Base Evidence ledger refused: {why}"),
            Self::Codec(why) => write!(formatter, "Base Evidence ledger codec refused: {why}"),
            Self::Io(why) => write!(formatter, "Base Evidence ledger I/O refused: {why}"),
            Self::Arithmetic(name) => write!(formatter, "Base Evidence ledger {name} overflowed"),
            Self::Allocation(why) => {
                write!(formatter, "Base Evidence ledger allocation refused: {why}")
            }
        }
    }
}

/// Explicit physical ceilings for one durable Base Evidence ledger.
///
/// There is intentionally no `Default`; the operator must bind both axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaseEvidenceLedgerBoundsV2 {
    records: u64,
    completions: u64,
}

impl BaseEvidenceLedgerBoundsV2 {
    /// Creates nonzero fixed-record and completion ceilings.
    ///
    /// # Errors
    ///
    /// Refuses either zero ceiling because an unbounded or unusable ledger is
    /// not an admitted durable authority.
    pub fn new(records: u64, completions: u64) -> Result<Self, BaseEvidenceLedgerRefusalV2> {
        if records == 0 {
            return Err(BaseEvidenceLedgerRefusalV2::InvalidBound("records"));
        }
        if completions == 0 {
            return Err(BaseEvidenceLedgerRefusalV2::InvalidBound("completions"));
        }
        Ok(Self {
            records,
            completions,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CompletionV2 {
    completion_id: [u8; 32],
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    candidate_ordered_row_digest: [u8; 32],
    base_policy_digest: [u8; 32],
    ordered_base_record_digest: [u8; 32],
    feed_digest: [u8; 32],
    source_commit_digest: [u8; 32],
    vocabulary_digest: [u8; 32],
    evaluation_policy_digest: [u8; 32],
    calendar_policy_digest: [u8; 32],
    daily_reference_policy_digest: [u8; 32],
    first_record: u64,
    candidate_row_count: u64,
    base_record_count: u64,
    family: InstrumentFamilyV1,
    rung_seconds: u32,
    horizon_bars: u32,
    requested_span: RequestedSpanIdentityV1,
}

impl CompletionV2 {
    fn from_prepared(
        candidate: &CandidateUniverseReceiptV1,
        prepared: &PreparedBaseEvidenceV2,
        first_record: u64,
    ) -> Result<Self, BaseEvidenceLedgerRefusalV2> {
        prepared
            .validate_candidate_receipt(candidate)
            .map_err(|why| BaseEvidenceLedgerRefusalV2::Reconciliation(why.to_string()))?;
        let identities = candidate.identities();
        let mut value = Self {
            completion_id: [0; 32],
            candidate_universe_id: candidate.universe_id(),
            candidate_completion_digest: candidate.content_digest(),
            candidate_ordered_row_digest: candidate.ordered_row_digest(),
            base_policy_digest: base_policy_digest_v2(),
            ordered_base_record_digest: prepared.ordered_record_digest,
            feed_digest: identities.feed_digest(),
            source_commit_digest: identities.source_commit_digest(),
            vocabulary_digest: identities.vocabulary_digest(),
            evaluation_policy_digest: identities.evaluation_policy_digest(),
            calendar_policy_digest: identities.calendar_policy_digest(),
            daily_reference_policy_digest: identities.daily_reference_policy_digest(),
            first_record,
            candidate_row_count: candidate.row_count(),
            base_record_count: u64::try_from(prepared.records.len())
                .map_err(|_| BaseEvidenceLedgerRefusalV2::Arithmetic("prepared record count"))?,
            family: candidate.family(),
            rung_seconds: candidate.rung_seconds(),
            horizon_bars: candidate.horizon_bars(),
            requested_span: candidate.requested_span(),
        };
        value.completion_id = value.derive_id();
        value.validate()?;
        Ok(value)
    }

    fn derive_id(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(COMPLETION_ID_DOMAIN);
        for digest in [
            self.candidate_universe_id,
            self.candidate_completion_digest,
            self.candidate_ordered_row_digest,
            self.base_policy_digest,
            self.ordered_base_record_digest,
            self.feed_digest,
            self.source_commit_digest,
            self.vocabulary_digest,
            self.evaluation_policy_digest,
            self.calendar_policy_digest,
            self.daily_reference_policy_digest,
        ] {
            hasher.update(&digest);
        }
        hasher.update(&self.candidate_row_count.to_le_bytes());
        hasher.update(&self.base_record_count.to_le_bytes());
        hasher.update(&[family_byte(self.family)]);
        hasher.update(&self.rung_seconds.to_le_bytes());
        hasher.update(&self.horizon_bars.to_le_bytes());
        hasher.update(&self.requested_span.canonical_bytes());
        hasher.finalize()
    }

    fn validate(&self) -> Result<(), BaseEvidenceLedgerRefusalV2> {
        for (name, value) in [
            ("completion", self.completion_id),
            ("Candidate universe", self.candidate_universe_id),
            ("Candidate completion", self.candidate_completion_digest),
            ("Candidate ordered rows", self.candidate_ordered_row_digest),
            ("Base policy", self.base_policy_digest),
            ("ordered Base records", self.ordered_base_record_digest),
            ("feed", self.feed_digest),
            ("source commit", self.source_commit_digest),
            ("vocabulary", self.vocabulary_digest),
            ("evaluation policy", self.evaluation_policy_digest),
            ("calendar policy", self.calendar_policy_digest),
            ("daily reference policy", self.daily_reference_policy_digest),
        ] {
            require_nonzero(name, value)?;
        }
        if self.base_policy_digest != base_policy_digest_v2() {
            return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
                "completion names a foreign Base Evidence policy".to_owned(),
            ));
        }
        if self.candidate_row_count != self.base_record_count {
            return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(format!(
                "Candidate count {} differs from Base record count {}",
                self.candidate_row_count, self.base_record_count
            )));
        }
        if self.rung_seconds == 0 || self.horizon_bars == 0 {
            return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
                "completion rung or horizon is zero".to_owned(),
            ));
        }
        if self.completion_id != self.derive_id() {
            return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
                "completion identity does not reproduce".to_owned(),
            ));
        }
        Ok(())
    }

    fn encode(&self) -> Result<[u8; COMPLETION_BYTES], BaseEvidenceLedgerRefusalV2> {
        self.validate()?;
        let mut raw = [0_u8; COMPLETION_BYTES];
        raw[..16].copy_from_slice(&COMPLETION_RECORD_MAGIC);
        put_u32(&mut raw, 16, HEADER_VERSION)?;
        put_u32(&mut raw, 20, 0)?;
        for (offset, value) in [
            (24, self.completion_id),
            (56, self.candidate_universe_id),
            (88, self.candidate_completion_digest),
            (120, self.candidate_ordered_row_digest),
            (152, self.base_policy_digest),
            (184, self.ordered_base_record_digest),
            (216, self.feed_digest),
            (248, self.source_commit_digest),
            (280, self.vocabulary_digest),
            (312, self.evaluation_policy_digest),
            (344, self.calendar_policy_digest),
            (376, self.daily_reference_policy_digest),
        ] {
            put_bytes(&mut raw, offset, &value)?;
        }
        put_u64(&mut raw, 408, self.first_record)?;
        put_u64(&mut raw, 416, self.candidate_row_count)?;
        put_u64(&mut raw, 424, self.base_record_count)?;
        raw[432] = family_byte(self.family);
        put_u32(&mut raw, 436, self.rung_seconds)?;
        put_u32(&mut raw, 440, self.horizon_bars)?;
        put_bytes(&mut raw, 444, &self.requested_span.canonical_bytes())?;
        let seal = digest_domain(COMPLETION_SEAL_DOMAIN, &raw[..COMPLETION_PAYLOAD_BYTES]);
        raw[COMPLETION_PAYLOAD_BYTES..].copy_from_slice(&seal);
        Ok(raw)
    }

    fn decode(raw: &[u8]) -> Result<Self, BaseEvidenceLedgerRefusalV2> {
        if raw.len() != COMPLETION_BYTES {
            return Err(BaseEvidenceLedgerRefusalV2::Codec(format!(
                "completion has {} bytes, not {COMPLETION_BYTES}",
                raw.len()
            )));
        }
        if raw.get(..16) != Some(COMPLETION_RECORD_MAGIC.as_slice()) {
            return Err(BaseEvidenceLedgerRefusalV2::Codec(
                "completion magic is unknown".to_owned(),
            ));
        }
        if get_u32(raw, 16)? != HEADER_VERSION {
            return Err(BaseEvidenceLedgerRefusalV2::Codec(
                "completion version is unknown".to_owned(),
            ));
        }
        require_zero(raw, 20, 4, "completion header reserve")?;
        require_zero(raw, 433, 3, "completion family reserve")?;
        require_zero(raw, 464, 16, "completion tail reserve")?;
        let payload = raw.get(..COMPLETION_PAYLOAD_BYTES).ok_or_else(|| {
            BaseEvidenceLedgerRefusalV2::Codec("completion payload is absent".to_owned())
        })?;
        let stored_seal = raw.get(COMPLETION_PAYLOAD_BYTES..).ok_or_else(|| {
            BaseEvidenceLedgerRefusalV2::Codec("completion seal is absent".to_owned())
        })?;
        let expected = digest_domain(COMPLETION_SEAL_DOMAIN, payload);
        if stored_seal != expected {
            return Err(BaseEvidenceLedgerRefusalV2::Codec(
                "completion seal does not match its payload".to_owned(),
            ));
        }
        let span = decode_span(raw.get(444..464).ok_or_else(|| {
            BaseEvidenceLedgerRefusalV2::Codec("completion span is absent".to_owned())
        })?)?;
        let value = Self {
            completion_id: get_32(raw, 24)?,
            candidate_universe_id: get_32(raw, 56)?,
            candidate_completion_digest: get_32(raw, 88)?,
            candidate_ordered_row_digest: get_32(raw, 120)?,
            base_policy_digest: get_32(raw, 152)?,
            ordered_base_record_digest: get_32(raw, 184)?,
            feed_digest: get_32(raw, 216)?,
            source_commit_digest: get_32(raw, 248)?,
            vocabulary_digest: get_32(raw, 280)?,
            evaluation_policy_digest: get_32(raw, 312)?,
            calendar_policy_digest: get_32(raw, 344)?,
            daily_reference_policy_digest: get_32(raw, 376)?,
            first_record: get_u64(raw, 408)?,
            candidate_row_count: get_u64(raw, 416)?,
            base_record_count: get_u64(raw, 424)?,
            family: family_from_byte(*raw.get(432).ok_or_else(|| {
                BaseEvidenceLedgerRefusalV2::Codec("completion family is absent".to_owned())
            })?)?,
            rung_seconds: get_u32(raw, 436)?,
            horizon_bars: get_u32(raw, 440)?,
            requested_span: span,
        };
        value.validate()?;
        Ok(value)
    }
}

/// Freshly reopened, fixed-stride Base Evidence completion authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaseEvidenceReopenAuditV2 {
    completion: CompletionV2,
}

impl BaseEvidenceReopenAuditV2 {
    /// Durable Base completion identity.
    #[must_use]
    pub const fn completion_id(&self) -> [u8; 32] {
        self.completion.completion_id
    }

    /// Candidate universe authenticated by this Base completion.
    #[must_use]
    pub const fn candidate_universe_id(&self) -> [u8; 32] {
        self.completion.candidate_universe_id
    }

    /// Exact Candidate receipt/content digest authenticated by this block.
    #[must_use]
    pub(crate) const fn candidate_completion_digest(&self) -> [u8; 32] {
        self.completion.candidate_completion_digest
    }

    /// Exact ordered Candidate-row digest authenticated by this block.
    #[must_use]
    pub(crate) const fn candidate_ordered_row_digest(&self) -> [u8; 32] {
        self.completion.candidate_ordered_row_digest
    }

    /// Number of fixed Base records in the contiguous block.
    #[must_use]
    pub const fn record_count(&self) -> u64 {
        self.completion.base_record_count
    }

    /// Swept family authenticated by the Candidate completion.
    #[must_use]
    pub const fn family(&self) -> InstrumentFamilyV1 {
        self.completion.family
    }
}

/// Immutable authenticated projection of one fixed-offset Base record.
///
/// It carries only decoded facts needed by the successor admission boundary;
/// no caller can use it to author or mutate a record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseEvidenceRecordProjectionV2 {
    evidence_id: [u8; 32],
    candidate_universe_id: [u8; 32],
    candidate_sequence: u64,
    candidate_semantic_id: [u8; 32],
    candidate_row_digest: [u8; 32],
    trade_rows_digest: [u8; 32],
    admission_values: AdmissionEvidenceValuesV1,
}

impl BaseEvidenceRecordProjectionV2 {
    /// Canonical same-pass evidence identity.
    #[must_use]
    pub const fn evidence_id(&self) -> [u8; 32] {
        self.evidence_id
    }

    /// Candidate universe authenticated by the durable completion.
    #[must_use]
    pub const fn candidate_universe_id(&self) -> [u8; 32] {
        self.candidate_universe_id
    }

    /// Canonical Candidate physical/logical order.
    #[must_use]
    pub const fn candidate_sequence(&self) -> u64 {
        self.candidate_sequence
    }

    /// Candidate semantic identity; this does not replace the row digest.
    #[must_use]
    pub const fn candidate_semantic_id(&self) -> [u8; 32] {
        self.candidate_semantic_id
    }

    /// Digest of the exact fixed Candidate row bytes.
    #[must_use]
    pub const fn candidate_row_digest(&self) -> [u8; 32] {
        self.candidate_row_digest
    }

    /// Digest of the exact same-pass authoritative `TradeRow` sequence.
    #[must_use]
    pub const fn trade_rows_digest(&self) -> [u8; 32] {
        self.trade_rows_digest
    }

    /// Exact 27 Base fields with all 17 Statistics/walk fields unmeasured.
    #[must_use]
    pub const fn admission_values(&self) -> &AdmissionEvidenceValuesV1 {
        &self.admission_values
    }
}

/// Bounded read-only Base Evidence authority with generation-checked O(1)
/// completion lookup and fixed-offset record reads after its opening scan.
pub struct BaseEvidenceLedgerReaderV2 {
    ledger: LedgerV2,
}

impl BaseEvidenceLedgerReaderV2 {
    /// Opens and validates every bounded completion/record block once.
    ///
    /// # Errors
    ///
    /// Refuses an absent/replaced root, malformed header, ragged file,
    /// exceeded bound, corrupt record/completion or inconsistent authority.
    pub fn open(
        root: &Path,
        bounds: BaseEvidenceLedgerBoundsV2,
    ) -> Result<Self, BaseEvidenceLedgerRefusalV2> {
        Ok(Self {
            ledger: LedgerV2::open_read(root, bounds)?,
        })
    }

    /// Returns the completed authority for one exact Candidate universe.
    ///
    /// # Errors
    ///
    /// Refuses if any held ledger file changed or its admitted path now names
    /// a different file.
    pub fn audit(
        &mut self,
        universe_id: [u8; 32],
    ) -> Result<Option<BaseEvidenceReopenAuditV2>, BaseEvidenceLedgerRefusalV2> {
        self.ledger.audit(universe_id)
    }

    /// Reads one Candidate-sequence record at a checked fixed offset.
    ///
    /// # Errors
    ///
    /// Refuses a stale/swapped audit, an out-of-range sequence, changed file
    /// generation, corrupt record or record/completion identity mismatch.
    pub fn record(
        &mut self,
        audit: &BaseEvidenceReopenAuditV2,
        sequence: u64,
    ) -> Result<BaseEvidenceRecordProjectionV2, BaseEvidenceLedgerRefusalV2> {
        self.ledger.record_projection(audit, sequence)
    }

    /// Consumes this already-scanned reader and binds it to one opaque,
    /// canonical NIFTY-then-BANKNIFTY Base authority.
    ///
    /// This is crate-private because the paired capability is a production
    /// join, not a general ledger-query surface.  No bytes, family labels or
    /// digests are accepted from the caller.  The opening scan already
    /// authenticated both complete record blocks; binding performs no reopen
    /// and no second block hash.
    pub(crate) fn bind_pair(
        mut self,
        authority: &PairedBaseEvidenceAuthorityV2,
    ) -> Result<PairedBaseEvidenceReaderV2, BaseEvidenceLedgerRefusalV2> {
        let authority = *authority;
        self.ledger
            .with_shared_lock(|ledger| require_exact_pair_in_ledger(ledger, &authority))?;
        let nifty_count = authority.nifty.record_count();
        let candidate_count = nifty_count
            .checked_add(authority.banknifty.record_count())
            .ok_or(BaseEvidenceLedgerRefusalV2::Arithmetic(
                "paired Base candidate count",
            ))?;
        Ok(PairedBaseEvidenceReaderV2 {
            ledger: self.ledger,
            authority,
            nifty_count,
            candidate_count,
        })
    }
}

/// One exact Base record projected through the canonical two-family ordinal.
///
/// The wrapper retains the opaque pair identity and derives both family labels
/// from the completed Base blocks.  A successor can compare this projection to
/// its Statistics candidate without accepting a caller-authored family,
/// family ordinal, Candidate digest or evidence digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PairedBaseEvidenceRecordProjectionV2 {
    pair_id: [u8; 32],
    global_sequence: u64,
    family: InstrumentFamilyV1,
    family_sequence: u64,
    record: BaseEvidenceRecordProjectionV2,
}

impl PairedBaseEvidenceRecordProjectionV2 {
    /// Exact NIFTY-then-BANKNIFTY pair that authenticated this record.
    #[must_use]
    pub(crate) const fn pair_id(&self) -> [u8; 32] {
        self.pair_id
    }

    /// Canonical global ordinal: every NIFTY candidate, then BANKNIFTY.
    #[must_use]
    pub(crate) const fn global_sequence(&self) -> u64 {
        self.global_sequence
    }

    /// Family derived from the retained pair and global ordinal.
    #[must_use]
    pub(crate) const fn family(&self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Candidate ordinal inside the derived family.
    #[must_use]
    pub(crate) const fn family_sequence(&self) -> u64 {
        self.family_sequence
    }

    /// Exact decoded Base record; its constructor and fields remain private.
    #[must_use]
    pub(crate) const fn record(&self) -> &BaseEvidenceRecordProjectionV2 {
        &self.record
    }
}

/// One held, bounded Base ledger bound to the exact two-family authority.
///
/// Opening and binding validate the complete bounded ledger once.  Thereafter
/// `candidate` performs arithmetic family mapping, one hash-table authority
/// lookup and one fixed-offset record read.  It does not reopen or rehash a
/// record block per candidate.  Filesystem locking, metadata validation and
/// the physical read are intentionally not described as constant latency.
pub(crate) struct PairedBaseEvidenceReaderV2 {
    ledger: LedgerV2,
    authority: PairedBaseEvidenceAuthorityV2,
    nifty_count: u64,
    candidate_count: u64,
}

impl PairedBaseEvidenceReaderV2 {
    /// Opaque paired authority retained by this reader.
    #[must_use]
    pub(crate) const fn authority(&self) -> &PairedBaseEvidenceAuthorityV2 {
        &self.authority
    }

    /// Exact total NIFTY-plus-BANKNIFTY candidate count.
    #[must_use]
    pub(crate) const fn candidate_count(&self) -> u64 {
        self.candidate_count
    }

    /// Reads one canonical NIFTY-first global candidate ordinal.
    ///
    /// # Errors
    ///
    /// Refuses an out-of-range ordinal, replaced root or child, generation
    /// change before/during the read, foreign pair, corrupt record or any
    /// record/completion source mismatch.
    pub(crate) fn candidate(
        &mut self,
        global_sequence: u64,
    ) -> Result<PairedBaseEvidenceRecordProjectionV2, BaseEvidenceLedgerRefusalV2> {
        let (audit, family, family_sequence) = paired_ordinal_source(
            &self.authority,
            self.nifty_count,
            self.candidate_count,
            global_sequence,
        )?;
        let record = self
            .ledger
            .with_shared_lock(|ledger| ledger.record_projection_locked(&audit, family_sequence))?;
        if record.candidate_universe_id() != audit.candidate_universe_id()
            || record.candidate_sequence() != family_sequence
        {
            return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
                "paired Base projection escaped its canonical family ordinal".to_owned(),
            ));
        }
        Ok(PairedBaseEvidenceRecordProjectionV2 {
            pair_id: self.authority.pair_id,
            global_sequence,
            family,
            family_sequence,
            record,
        })
    }
}

/// Exact result of append+sync+fresh read-only reopen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BaseEvidenceProductionCommitV2 {
    /// New record bytes and then a new completion were synced.
    Written(BaseEvidenceReopenAuditV2),
    /// Every existing record and completion byte matched the exact retry.
    Reused(BaseEvidenceReopenAuditV2),
}

impl BaseEvidenceProductionCommitV2 {
    /// Freshly reopened durable authority from either exact branch.
    pub(crate) const fn audit(self) -> BaseEvidenceReopenAuditV2 {
        match self {
            Self::Written(audit) | Self::Reused(audit) => audit,
        }
    }
}

/// Canonical NIFTY-then-BANKNIFTY Base Evidence authority pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PairedBaseEvidenceAuthorityV2 {
    pair_id: [u8; 32],
    nifty: BaseEvidenceReopenAuditV2,
    banknifty: BaseEvidenceReopenAuditV2,
}

impl PairedBaseEvidenceAuthorityV2 {
    /// Domain-separated identity of the ordered two-family authority.
    #[must_use]
    pub(crate) const fn pair_id(&self) -> [u8; 32] {
        self.pair_id
    }

    /// Exact NIFTY Base completion retained for fixed-offset projection.
    #[must_use]
    pub(crate) const fn nifty_audit(&self) -> &BaseEvidenceReopenAuditV2 {
        &self.nifty
    }

    /// Exact BANKNIFTY Base completion retained for fixed-offset projection.
    #[must_use]
    pub(crate) const fn banknifty_audit(&self) -> &BaseEvidenceReopenAuditV2 {
        &self.banknifty
    }
}

/// Requires exactly NIFTY then BANKNIFTY under one shared cohort policy.
pub(crate) fn pair_base_evidence_authority_v2(
    nifty: &BaseEvidenceReopenAuditV2,
    banknifty: &BaseEvidenceReopenAuditV2,
) -> Result<PairedBaseEvidenceAuthorityV2, BaseEvidenceLedgerRefusalV2> {
    let left = &nifty.completion;
    let right = &banknifty.completion;
    if left.family != InstrumentFamilyV1::Nifty || right.family != InstrumentFamilyV1::BankNifty {
        return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
            "Base Evidence pair must be ordered NIFTY then BANKNIFTY".to_owned(),
        ));
    }
    if left.candidate_universe_id == right.candidate_universe_id {
        return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
            "Base Evidence pair repeats one Candidate universe".to_owned(),
        ));
    }
    if left.rung_seconds != right.rung_seconds
        || left.horizon_bars != right.horizon_bars
        || left.requested_span != right.requested_span
        || left.feed_digest != right.feed_digest
        || left.source_commit_digest != right.source_commit_digest
        || left.vocabulary_digest != right.vocabulary_digest
        || left.evaluation_policy_digest != right.evaluation_policy_digest
        || left.calendar_policy_digest != right.calendar_policy_digest
        || left.daily_reference_policy_digest != right.daily_reference_policy_digest
        || left.base_policy_digest != right.base_policy_digest
    {
        return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
            "NIFTY/BANKNIFTY Base Evidence completions do not share one exact cohort policy"
                .to_owned(),
        ));
    }
    let mut hasher = Hasher::new();
    hasher.update(PAIR_ID_DOMAIN);
    hasher.update(&left.completion_id);
    hasher.update(&right.completion_id);
    let pair_id = hasher.finalize();
    require_nonzero("paired Base Evidence", pair_id)?;
    Ok(PairedBaseEvidenceAuthorityV2 {
        pair_id,
        nifty: *nifty,
        banknifty: *banknifty,
    })
}

fn require_exact_pair_in_ledger(
    ledger: &LedgerV2,
    authority: &PairedBaseEvidenceAuthorityV2,
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    let nifty = ledger
        .audits
        .get(&authority.nifty.candidate_universe_id())
        .copied()
        .ok_or_else(|| {
            BaseEvidenceLedgerRefusalV2::Reconciliation(
                "paired Base NIFTY completion is absent from the held ledger".to_owned(),
            )
        })?;
    let banknifty = ledger
        .audits
        .get(&authority.banknifty.candidate_universe_id())
        .copied()
        .ok_or_else(|| {
            BaseEvidenceLedgerRefusalV2::Reconciliation(
                "paired Base BANKNIFTY completion is absent from the held ledger".to_owned(),
            )
        })?;
    if nifty != authority.nifty || banknifty != authority.banknifty {
        return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
            "paired Base authority differs from the held ledger generations".to_owned(),
        ));
    }
    if nifty.family() != InstrumentFamilyV1::Nifty
        || banknifty.family() != InstrumentFamilyV1::BankNifty
    {
        return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
            "paired Base held completions are not canonical NIFTY then BANKNIFTY".to_owned(),
        ));
    }
    Ok(())
}

fn paired_ordinal_source(
    authority: &PairedBaseEvidenceAuthorityV2,
    nifty_count: u64,
    candidate_count: u64,
    global_sequence: u64,
) -> Result<(BaseEvidenceReopenAuditV2, InstrumentFamilyV1, u64), BaseEvidenceLedgerRefusalV2> {
    if global_sequence < nifty_count {
        return Ok((authority.nifty, InstrumentFamilyV1::Nifty, global_sequence));
    }
    if global_sequence < candidate_count {
        let family_sequence = global_sequence.checked_sub(nifty_count).ok_or(
            BaseEvidenceLedgerRefusalV2::Arithmetic("BANKNIFTY Base family ordinal"),
        )?;
        return Ok((
            authority.banknifty,
            InstrumentFamilyV1::BankNifty,
            family_sequence,
        ));
    }
    Err(BaseEvidenceLedgerRefusalV2::BoundExceeded {
        resource: "paired candidate ordinal",
        actual: global_sequence,
        maximum: candidate_count.saturating_sub(1),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OrphanV2 {
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    candidate_ordered_row_digest: [u8; 32],
    base_policy_digest: [u8; 32],
    first_record: u64,
    record_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGenerationV2 {
    len: u64,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    modified_seconds: i64,
    #[cfg(unix)]
    modified_nanoseconds: i64,
    #[cfg(unix)]
    changed_seconds: i64,
    #[cfg(unix)]
    changed_nanoseconds: i64,
    #[cfg(not(unix))]
    modified: Option<std::time::SystemTime>,
}

struct LedgerV2 {
    root: PathBuf,
    root_file: File,
    root_generation: FileGenerationV2,
    lock_path: PathBuf,
    record_path: PathBuf,
    completion_path: PathBuf,
    lock_file: File,
    record_file: File,
    completion_file: File,
    bounds: BaseEvidenceLedgerBoundsV2,
    audits: HashMap<[u8; 32], BaseEvidenceReopenAuditV2>,
    orphan: Option<OrphanV2>,
    total_records: u64,
    lock_generation: FileGenerationV2,
    record_generation: FileGenerationV2,
    completion_generation: FileGenerationV2,
    writable: bool,
}

impl LedgerV2 {
    fn open(
        root: &Path,
        bounds: BaseEvidenceLedgerBoundsV2,
    ) -> Result<Self, BaseEvidenceLedgerRefusalV2> {
        Self::open_inner(root, bounds, true)
    }

    fn open_read(
        root: &Path,
        bounds: BaseEvidenceLedgerBoundsV2,
    ) -> Result<Self, BaseEvidenceLedgerRefusalV2> {
        Self::open_inner(root, bounds, false)
    }

    fn open_inner(
        root: &Path,
        bounds: BaseEvidenceLedgerBoundsV2,
        writable: bool,
    ) -> Result<Self, BaseEvidenceLedgerRefusalV2> {
        let root = admit_root(root)?;
        let root_file =
            File::open(&root).map_err(|why| io_error("hold Base ledger root", &root, &why))?;
        if !root_file
            .metadata()
            .map_err(|why| io_error("stat held Base ledger root", &root, &why))?
            .is_dir()
        {
            return Err(BaseEvidenceLedgerRefusalV2::Io(format!(
                "Base ledger root {} is no longer a directory",
                root.display()
            )));
        }
        let lock_path = root.join(LOCK_FILE);
        let record_path = root.join(RECORD_FILE);
        let completion_path = root.join(COMPLETION_FILE);
        let lock_file = open_file(&lock_path, writable, writable)?;
        if writable {
            lock_file
                .lock()
                .map_err(|why| io_error("take exclusive open lock", &lock_path, &why))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| io_error("take shared open lock", &lock_path, &why))?;
        }
        let opened = (|| {
            let mut record_file = open_file(&record_path, writable, writable)?;
            let mut completion_file = open_file(&completion_path, writable, writable)?;
            if writable {
                ensure_header(
                    &mut record_file,
                    RECORD_MAGIC,
                    RECORD_KIND,
                    RECORD_BYTES_U64,
                    &record_path,
                )?;
                ensure_header(
                    &mut completion_file,
                    COMPLETION_MAGIC,
                    COMPLETION_KIND,
                    COMPLETION_BYTES_U64,
                    &completion_path,
                )?;
            } else {
                verify_header(
                    &mut record_file,
                    RECORD_MAGIC,
                    RECORD_KIND,
                    RECORD_BYTES_U64,
                    &record_path,
                )?;
                verify_header(
                    &mut completion_file,
                    COMPLETION_MAGIC,
                    COMPLETION_KIND,
                    COMPLETION_BYTES_U64,
                    &completion_path,
                )?;
            }
            let lock_generation = file_generation(&lock_file, &lock_path)?;
            let record_generation = file_generation(&record_file, &record_path)?;
            let completion_generation = file_generation(&completion_file, &completion_path)?;
            let root_generation = file_generation(&root_file, &root)?;
            let mut ledger = Self {
                root,
                root_file,
                root_generation,
                lock_path: lock_path.clone(),
                record_path,
                completion_path,
                lock_file: lock_file.try_clone().map_err(|why| {
                    BaseEvidenceLedgerRefusalV2::Io(format!("cannot clone Base writer lock: {why}"))
                })?,
                record_file,
                completion_file,
                bounds,
                audits: HashMap::new(),
                orphan: None,
                total_records: 0,
                lock_generation,
                record_generation,
                completion_generation,
                writable,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let unlocked = lock_file
            .unlock()
            .map_err(|why| io_error("release open lock", &lock_path, &why));
        match (opened, unlocked) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn scan(&mut self) -> Result<(), BaseEvidenceLedgerRefusalV2> {
        verify_header(
            &mut self.record_file,
            RECORD_MAGIC,
            RECORD_KIND,
            RECORD_BYTES_U64,
            &self.record_path,
        )?;
        verify_header(
            &mut self.completion_file,
            COMPLETION_MAGIC,
            COMPLETION_KIND,
            COMPLETION_BYTES_U64,
            &self.completion_path,
        )?;
        let records = record_count(
            &self.record_file,
            RECORD_BYTES_U64,
            self.bounds.records,
            "records",
        )?;
        let completions = record_count(
            &self.completion_file,
            COMPLETION_BYTES_U64,
            self.bounds.completions,
            "completions",
        )?;
        let capacity = usize::try_from(completions)
            .map_err(|_| BaseEvidenceLedgerRefusalV2::Arithmetic("completion index capacity"))?;
        self.audits.clear();
        self.audits.try_reserve(capacity).map_err(|why| {
            BaseEvidenceLedgerRefusalV2::Allocation(format!("completion index: {why}"))
        })?;
        let mut committed = 0_u64;
        for physical in 0..completions {
            let completion = read_completion(&mut self.completion_file, physical)?;
            if completion.first_record != committed {
                return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(format!(
                    "completion {} begins at {}, not contiguous cursor {committed}",
                    hex32(completion.completion_id),
                    completion.first_record
                )));
            }
            let end = committed.checked_add(completion.base_record_count).ok_or(
                BaseEvidenceLedgerRefusalV2::Arithmetic("committed record cursor"),
            )?;
            if end > records {
                return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(format!(
                    "completion {} reaches {end}, beyond physical record count {records}",
                    hex32(completion.completion_id)
                )));
            }
            validate_block(&mut self.record_file, &completion)?;
            let audit = BaseEvidenceReopenAuditV2 { completion };
            if self
                .audits
                .insert(completion.candidate_universe_id, audit)
                .is_some()
            {
                return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
                    "one Candidate universe has more than one Base completion".to_owned(),
                ));
            }
            committed = end;
        }
        self.orphan = scan_orphan(&mut self.record_file, committed, records)?;
        self.total_records = records;
        self.record_generation = file_generation(&self.record_file, &self.record_path)?;
        self.completion_generation = file_generation(&self.completion_file, &self.completion_path)?;
        Ok(())
    }

    fn audit(
        &mut self,
        universe_id: [u8; 32],
    ) -> Result<Option<BaseEvidenceReopenAuditV2>, BaseEvidenceLedgerRefusalV2> {
        self.with_shared_lock(|ledger| Ok(ledger.audits.get(&universe_id).copied()))
    }

    fn record_projection(
        &mut self,
        audit: &BaseEvidenceReopenAuditV2,
        sequence: u64,
    ) -> Result<BaseEvidenceRecordProjectionV2, BaseEvidenceLedgerRefusalV2> {
        self.with_shared_lock(|ledger| ledger.record_projection_locked(audit, sequence))
    }

    fn record_projection_locked(
        &mut self,
        audit: &BaseEvidenceReopenAuditV2,
        sequence: u64,
    ) -> Result<BaseEvidenceRecordProjectionV2, BaseEvidenceLedgerRefusalV2> {
        let current = self
            .audits
            .get(&audit.candidate_universe_id())
            .copied()
            .ok_or_else(|| {
                BaseEvidenceLedgerRefusalV2::Reconciliation(
                    "record audit names no complete Base Candidate universe".to_owned(),
                )
            })?;
        if current != *audit {
            return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
                "record audit was swapped or is stale".to_owned(),
            ));
        }
        if sequence >= audit.record_count() {
            return Err(BaseEvidenceLedgerRefusalV2::BoundExceeded {
                resource: "record sequence",
                actual: sequence,
                maximum: audit.record_count().saturating_sub(1),
            });
        }
        let physical = audit.completion.first_record.checked_add(sequence).ok_or(
            BaseEvidenceLedgerRefusalV2::Arithmetic("record projection offset"),
        )?;
        let record =
            BaseEvidenceRecordV2::decode(&read_record_bytes(&mut self.record_file, physical)?)
                .map_err(|why| BaseEvidenceLedgerRefusalV2::Codec(why.to_string()))?;
        require_record_source(&record, &audit.completion, sequence)?;
        Ok(BaseEvidenceRecordProjectionV2 {
            evidence_id: record.evidence_id,
            candidate_universe_id: record.draft.subject.candidate_universe_id,
            candidate_sequence: record.draft.subject.candidate_sequence,
            candidate_semantic_id: record.draft.subject.candidate_semantic_id,
            candidate_row_digest: record.draft.subject.candidate_row_digest,
            trade_rows_digest: record.draft.trade_rows_digest,
            admission_values: record
                .admission_values()
                .map_err(|why| BaseEvidenceLedgerRefusalV2::Reconciliation(why.to_string()))?,
        })
    }

    fn with_shared_lock<T>(
        &mut self,
        action: impl FnOnce(&mut Self) -> Result<T, BaseEvidenceLedgerRefusalV2>,
    ) -> Result<T, BaseEvidenceLedgerRefusalV2> {
        self.lock_file
            .lock_shared()
            .map_err(|why| io_error("take record read lock", &self.lock_path, &why))?;
        let result = (|| {
            self.require_unchanged()?;
            let value = action(self)?;
            self.require_unchanged()?;
            Ok(value)
        })();
        let unlocked = self
            .lock_file
            .unlock()
            .map_err(|why| io_error("release record read lock", &self.lock_path, &why));
        match (result, unlocked) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append(
        &mut self,
        candidate: &CandidateUniverseReopenAuditV1,
        prepared: &PreparedBaseEvidenceV2,
    ) -> Result<BaseEvidenceProductionCommitV2, BaseEvidenceLedgerRefusalV2> {
        if !self.writable {
            return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
                "Base Evidence ledger was opened read-only".to_owned(),
            ));
        }
        self.lock_file
            .lock()
            .map_err(|why| io_error("take append lock", &self.lock_path, &why))?;
        let result = self.append_locked(candidate, prepared);
        let unlocked = self
            .lock_file
            .unlock()
            .map_err(|why| io_error("release append lock", &self.lock_path, &why));
        match (result, unlocked) {
            (Ok(commit), Ok(())) => Ok(commit),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_locked(
        &mut self,
        candidate: &CandidateUniverseReopenAuditV1,
        prepared: &PreparedBaseEvidenceV2,
    ) -> Result<BaseEvidenceProductionCommitV2, BaseEvidenceLedgerRefusalV2> {
        self.require_unchanged()?;
        let receipt = candidate.receipt();
        prepared
            .validate_candidate_receipt(&receipt)
            .map_err(|why| BaseEvidenceLedgerRefusalV2::Reconciliation(why.to_string()))?;
        let offered = u64::try_from(prepared.records.len())
            .map_err(|_| BaseEvidenceLedgerRefusalV2::Arithmetic("offered record count"))?;
        require_bound("append records", offered, self.bounds.records)?;
        if let Some(existing) = self.audits.get(&receipt.universe_id()).copied() {
            let expected =
                CompletionV2::from_prepared(&receipt, prepared, existing.completion.first_record)?;
            if existing.completion != expected {
                return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
                    "Candidate universe is complete with different Base bytes".to_owned(),
                ));
            }
            compare_prepared(&mut self.record_file, expected.first_record, prepared)?;
            self.record_file
                .sync_data()
                .map_err(|why| io_error("re-sync reused Base records", &self.record_path, &why))?;
            self.completion_file.sync_data().map_err(|why| {
                io_error(
                    "re-sync reused Base completion",
                    &self.completion_path,
                    &why,
                )
            })?;
            sync_directory(&self.root_file, &self.root)?;
            self.record_generation = file_generation(&self.record_file, &self.record_path)?;
            self.completion_generation =
                file_generation(&self.completion_file, &self.completion_path)?;
            return Ok(BaseEvidenceProductionCommitV2::Reused(existing));
        }
        require_bound(
            "completion",
            u64::try_from(self.audits.len())
                .map_err(|_| BaseEvidenceLedgerRefusalV2::Arithmetic("completion index size"))?
                .checked_add(1)
                .ok_or(BaseEvidenceLedgerRefusalV2::Arithmetic("completion count"))?,
            self.bounds.completions,
        )?;
        let (first_record, prefix) = match self.orphan {
            Some(orphan) => {
                if orphan.candidate_universe_id != receipt.universe_id()
                    || orphan.candidate_completion_digest != receipt.content_digest()
                    || orphan.candidate_ordered_row_digest != receipt.ordered_row_digest()
                    || orphan.base_policy_digest != base_policy_digest_v2()
                {
                    return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
                        "physical Base tail belongs to another source; no fallback may hide it"
                            .to_owned(),
                    ));
                }
                if orphan.record_count > offered {
                    return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
                        "physical Base orphan is longer than the exact retry".to_owned(),
                    ));
                }
                compare_prepared_prefix(
                    &mut self.record_file,
                    orphan.first_record,
                    prepared,
                    orphan.record_count,
                )?;
                (orphan.first_record, orphan.record_count)
            }
            None => (self.total_records, 0),
        };
        let completion = CompletionV2::from_prepared(&receipt, prepared, first_record)?;
        let new_total =
            first_record
                .checked_add(offered)
                .ok_or(BaseEvidenceLedgerRefusalV2::Arithmetic(
                    "record append total",
                ))?;
        require_bound("records", new_total, self.bounds.records)?;
        append_prepared_records(&mut self.record_file, prepared, prefix)?;
        self.record_file
            .sync_data()
            .map_err(|why| io_error("sync Base records", &self.record_path, &why))?;
        self.total_records = new_total;
        self.orphan = Some(orphan_from_completion(&completion));
        self.record_generation = file_generation(&self.record_file, &self.record_path)?;
        append_completion(&mut self.completion_file, &completion)?;
        self.completion_file
            .sync_data()
            .map_err(|why| io_error("sync Base completion", &self.completion_path, &why))?;
        sync_directory(&self.root_file, &self.root)?;
        let audit = BaseEvidenceReopenAuditV2 { completion };
        self.audits.insert(receipt.universe_id(), audit);
        self.orphan = None;
        self.completion_generation = file_generation(&self.completion_file, &self.completion_path)?;
        Ok(BaseEvidenceProductionCommitV2::Written(audit))
    }

    fn require_unchanged(&self) -> Result<(), BaseEvidenceLedgerRefusalV2> {
        require_generation(self.root_generation, &self.root_file, &self.root)?;
        require_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
        require_generation(self.record_generation, &self.record_file, &self.record_path)?;
        require_generation(
            self.completion_generation,
            &self.completion_file,
            &self.completion_path,
        )
    }
}

/// Appends one authenticated Base family and returns only after exact fresh reopen.
pub(crate) fn append_and_reopen_base_evidence_v2(
    root: &Path,
    bounds: BaseEvidenceLedgerBoundsV2,
    candidate: &CandidateUniverseReopenAuditV1,
    prepared: &PreparedBaseEvidenceV2,
) -> Result<BaseEvidenceProductionCommitV2, BaseEvidenceLedgerRefusalV2> {
    let mut ledger = LedgerV2::open(root, bounds)?;
    let commit = ledger.append(candidate, prepared)?;
    let expected = commit.audit();
    drop(ledger);
    let mut reopened = LedgerV2::open_read(root, bounds)?;
    let audit = reopened
        .audit(expected.candidate_universe_id())?
        .ok_or_else(|| {
            BaseEvidenceLedgerRefusalV2::Reconciliation(
                "Base completion disappeared after receipt-last append".to_owned(),
            )
        })?;
    if audit != expected {
        return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
            "freshly reopened Base completion differs from the exact append".to_owned(),
        ));
    }
    compare_prepared(
        &mut reopened.record_file,
        audit.completion.first_record,
        prepared,
    )?;
    Ok(match commit {
        BaseEvidenceProductionCommitV2::Written(_) => {
            BaseEvidenceProductionCommitV2::Written(audit)
        }
        BaseEvidenceProductionCommitV2::Reused(_) => BaseEvidenceProductionCommitV2::Reused(audit),
    })
}

fn validate_block(
    file: &mut File,
    completion: &CompletionV2,
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    let mut ordered = ordered_hasher(completion);
    for logical in 0..completion.base_record_count {
        let physical = completion.first_record.checked_add(logical).ok_or(
            BaseEvidenceLedgerRefusalV2::Arithmetic("physical record index"),
        )?;
        let bytes = read_record_bytes(file, physical)?;
        let value = BaseEvidenceRecordV2::decode(&bytes)
            .map_err(|why| BaseEvidenceLedgerRefusalV2::Codec(why.to_string()))?;
        require_record_source(&value, completion, logical)?;
        ordered.update(&bytes);
    }
    if ordered.finalize() != completion.ordered_base_record_digest {
        return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
            "completion ordered Base record digest does not match its block".to_owned(),
        ));
    }
    Ok(())
}

fn ordered_hasher(completion: &CompletionV2) -> Hasher {
    let mut ordered = Hasher::new();
    ordered.update(super::ORDERED_RECORDS_DOMAIN);
    ordered.update(&completion.candidate_universe_id);
    ordered.update(&completion.candidate_completion_digest);
    ordered.update(&completion.candidate_ordered_row_digest);
    ordered.update(&completion.base_record_count.to_le_bytes());
    ordered
}

fn require_record_source(
    value: &BaseEvidenceRecordV2,
    completion: &CompletionV2,
    logical: u64,
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    if value.draft.subject.candidate_universe_id != completion.candidate_universe_id
        || value.candidate_completion_digest != completion.candidate_completion_digest
        || value.candidate_ordered_row_digest != completion.candidate_ordered_row_digest
        || value.base_policy_digest != completion.base_policy_digest
        || value.draft.subject.candidate_sequence != logical
    {
        return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
            "Base record escaped its completion source or canonical sequence".to_owned(),
        ));
    }
    Ok(())
}

fn scan_orphan(
    file: &mut File,
    committed: u64,
    total: u64,
) -> Result<Option<OrphanV2>, BaseEvidenceLedgerRefusalV2> {
    if committed == total {
        return Ok(None);
    }
    let first = BaseEvidenceRecordV2::decode(&read_record_bytes(file, committed)?)
        .map_err(|why| BaseEvidenceLedgerRefusalV2::Codec(why.to_string()))?;
    if first.draft.subject.candidate_sequence != 0 {
        return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
            "Base orphan does not begin at Candidate sequence zero".to_owned(),
        ));
    }
    let orphan = OrphanV2 {
        candidate_universe_id: first.draft.subject.candidate_universe_id,
        candidate_completion_digest: first.candidate_completion_digest,
        candidate_ordered_row_digest: first.candidate_ordered_row_digest,
        base_policy_digest: first.base_policy_digest,
        first_record: committed,
        record_count: total - committed,
    };
    for logical in 0..orphan.record_count {
        let physical =
            committed
                .checked_add(logical)
                .ok_or(BaseEvidenceLedgerRefusalV2::Arithmetic(
                    "orphan physical index",
                ))?;
        let value = BaseEvidenceRecordV2::decode(&read_record_bytes(file, physical)?)
            .map_err(|why| BaseEvidenceLedgerRefusalV2::Codec(why.to_string()))?;
        if value.draft.subject.candidate_universe_id != orphan.candidate_universe_id
            || value.candidate_completion_digest != orphan.candidate_completion_digest
            || value.candidate_ordered_row_digest != orphan.candidate_ordered_row_digest
            || value.base_policy_digest != orphan.base_policy_digest
            || value.draft.subject.candidate_sequence != logical
        {
            return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
                "Base tail contains swapped sources or noncontiguous sequence".to_owned(),
            ));
        }
    }
    Ok(Some(orphan))
}

const fn orphan_from_completion(completion: &CompletionV2) -> OrphanV2 {
    OrphanV2 {
        candidate_universe_id: completion.candidate_universe_id,
        candidate_completion_digest: completion.candidate_completion_digest,
        candidate_ordered_row_digest: completion.candidate_ordered_row_digest,
        base_policy_digest: completion.base_policy_digest,
        first_record: completion.first_record,
        record_count: completion.base_record_count,
    }
}

fn compare_prepared(
    file: &mut File,
    first_record: u64,
    prepared: &PreparedBaseEvidenceV2,
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    compare_prepared_prefix(
        file,
        first_record,
        prepared,
        u64::try_from(prepared.records.len())
            .map_err(|_| BaseEvidenceLedgerRefusalV2::Arithmetic("prepared comparison count"))?,
    )
}

fn compare_prepared_prefix(
    file: &mut File,
    first_record: u64,
    prepared: &PreparedBaseEvidenceV2,
    count: u64,
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    for logical in 0..count {
        let logical_usize = usize::try_from(logical)
            .map_err(|_| BaseEvidenceLedgerRefusalV2::Arithmetic("prepared logical index"))?;
        let expected = prepared
            .records
            .get(logical_usize)
            .ok_or_else(|| {
                BaseEvidenceLedgerRefusalV2::Reconciliation(
                    "prepared Base retry lost a required record".to_owned(),
                )
            })?
            .encode()
            .map_err(|why| BaseEvidenceLedgerRefusalV2::Codec(why.to_string()))?;
        let physical =
            first_record
                .checked_add(logical)
                .ok_or(BaseEvidenceLedgerRefusalV2::Arithmetic(
                    "comparison physical index",
                ))?;
        if read_record_bytes(file, physical)? != expected {
            return Err(BaseEvidenceLedgerRefusalV2::Reconciliation(
                "stored Base record differs from the exact prepared retry".to_owned(),
            ));
        }
    }
    Ok(())
}

fn append_prepared_records(
    file: &mut File,
    prepared: &PreparedBaseEvidenceV2,
    prefix: u64,
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    let start = usize::try_from(prefix)
        .map_err(|_| BaseEvidenceLedgerRefusalV2::Arithmetic("append prefix"))?;
    let original = file
        .metadata()
        .map_err(|why| {
            BaseEvidenceLedgerRefusalV2::Io(format!(
                "cannot stat Base records before append: {why}"
            ))
        })?
        .len();
    file.seek(SeekFrom::End(0)).map_err(|why| {
        BaseEvidenceLedgerRefusalV2::Io(format!("cannot seek Base record append: {why}"))
    })?;
    let result = (|| {
        for record in prepared.records.get(start..).ok_or_else(|| {
            BaseEvidenceLedgerRefusalV2::Reconciliation(
                "append prefix exceeds prepared records".to_owned(),
            )
        })? {
            let bytes = record
                .encode()
                .map_err(|why| BaseEvidenceLedgerRefusalV2::Codec(why.to_string()))?;
            file.write_all(&bytes).map_err(|why| {
                BaseEvidenceLedgerRefusalV2::Io(format!("cannot append Base record: {why}"))
            })?;
        }
        Ok(())
    })();
    if let Err(write_why) = result {
        return match file.set_len(original) {
            Ok(()) => Err(write_why),
            Err(rollback_why) => Err(BaseEvidenceLedgerRefusalV2::Io(format!(
                "{write_why}; rollback to {original} bytes failed: {rollback_why}; file is poisoned"
            ))),
        };
    }
    Ok(())
}

fn append_completion(
    file: &mut File,
    completion: &CompletionV2,
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    let original = file
        .metadata()
        .map_err(|why| {
            BaseEvidenceLedgerRefusalV2::Io(format!(
                "cannot stat Base completion before append: {why}"
            ))
        })?
        .len();
    file.seek(SeekFrom::End(0)).map_err(|why| {
        BaseEvidenceLedgerRefusalV2::Io(format!("cannot seek Base completion append: {why}"))
    })?;
    let bytes = completion.encode()?;
    if let Err(why) = file.write_all(&bytes) {
        return match file.set_len(original) {
            Ok(()) => Err(BaseEvidenceLedgerRefusalV2::Io(format!(
                "cannot append Base completion: {why}"
            ))),
            Err(rollback_why) => Err(BaseEvidenceLedgerRefusalV2::Io(format!(
                "cannot append Base completion: {why}; rollback to {original} bytes failed: {rollback_why}; file is poisoned"
            ))),
        };
    }
    Ok(())
}

fn read_record_bytes(
    file: &mut File,
    index: u64,
) -> Result<[u8; BASE_EVIDENCE_RECORD_BYTES_V2], BaseEvidenceLedgerRefusalV2> {
    let offset = fixed_offset(index, RECORD_BYTES_U64)?;
    let mut raw = [0_u8; BASE_EVIDENCE_RECORD_BYTES_V2];
    file.seek(SeekFrom::Start(offset))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| {
            BaseEvidenceLedgerRefusalV2::Io(format!("cannot read Base record {index}: {why}"))
        })?;
    Ok(raw)
}

fn read_completion(
    file: &mut File,
    index: u64,
) -> Result<CompletionV2, BaseEvidenceLedgerRefusalV2> {
    let offset = fixed_offset(index, COMPLETION_BYTES_U64)?;
    let mut raw = [0_u8; COMPLETION_BYTES];
    file.seek(SeekFrom::Start(offset))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| {
            BaseEvidenceLedgerRefusalV2::Io(format!("cannot read Base completion {index}: {why}"))
        })?;
    CompletionV2::decode(&raw)
}

fn fixed_offset(index: u64, stride: u64) -> Result<u64, BaseEvidenceLedgerRefusalV2> {
    index
        .checked_mul(stride)
        .and_then(|bytes| HEADER_BYTES_U64.checked_add(bytes))
        .ok_or(BaseEvidenceLedgerRefusalV2::Arithmetic(
            "fixed record offset",
        ))
}

fn record_count(
    file: &File,
    stride: u64,
    maximum: u64,
    resource: &'static str,
) -> Result<u64, BaseEvidenceLedgerRefusalV2> {
    let len = file
        .metadata()
        .map_err(|why| {
            BaseEvidenceLedgerRefusalV2::Io(format!("cannot stat Base {resource} file: {why}"))
        })?
        .len();
    let body = len.checked_sub(HEADER_BYTES_U64).ok_or_else(|| {
        BaseEvidenceLedgerRefusalV2::Codec(format!(
            "Base {resource} file is shorter than its header"
        ))
    })?;
    if body % stride != 0 {
        return Err(BaseEvidenceLedgerRefusalV2::Codec(format!(
            "Base {resource} file has ragged {body}-byte body for {stride}-byte stride"
        )));
    }
    let count = body / stride;
    require_bound(resource, count, maximum)?;
    Ok(count)
}

fn header_bytes(magic: [u8; 16], kind: u32, stride: u64) -> [u8; HEADER_BYTES] {
    let mut raw = [0_u8; HEADER_BYTES];
    raw[..16].copy_from_slice(&magic);
    raw[16..20].copy_from_slice(&HEADER_VERSION.to_le_bytes());
    raw[20..24].copy_from_slice(&kind.to_le_bytes());
    raw[24..32].copy_from_slice(&stride.to_le_bytes());
    let seal = digest_domain(HEADER_DOMAIN, &raw[..32]);
    raw[32..].copy_from_slice(&seal);
    raw
}

fn ensure_header(
    file: &mut File,
    magic: [u8; 16],
    kind: u32,
    stride: u64,
    path: &Path,
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    if file
        .metadata()
        .map_err(|why| io_error("stat", path, &why))?
        .len()
        == 0
    {
        file.seek(SeekFrom::Start(0))
            .and_then(|_| file.write_all(&header_bytes(magic, kind, stride)))
            .and_then(|()| file.sync_data())
            .map_err(|why| io_error("initialize header", path, &why))?;
    }
    verify_header(file, magic, kind, stride, path)
}

fn verify_header(
    file: &mut File,
    magic: [u8; 16],
    kind: u32,
    stride: u64,
    path: &Path,
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    if file
        .metadata()
        .map_err(|why| io_error("stat", path, &why))?
        .len()
        < HEADER_BYTES_U64
    {
        return Err(BaseEvidenceLedgerRefusalV2::Codec(format!(
            "{} is shorter than the fixed Base header",
            path.display()
        )));
    }
    let mut raw = [0_u8; HEADER_BYTES];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| io_error("read header", path, &why))?;
    if raw[..16] != magic
        || get_u32(&raw, 16)? != HEADER_VERSION
        || get_u32(&raw, 20)? != kind
        || get_u64(&raw, 24)? != stride
        || raw[32..] != digest_domain(HEADER_DOMAIN, &raw[..32])
    {
        return Err(BaseEvidenceLedgerRefusalV2::Codec(format!(
            "{} has a foreign or corrupt Base header",
            path.display()
        )));
    }
    Ok(())
}

fn admit_root(root: &Path) -> Result<PathBuf, BaseEvidenceLedgerRefusalV2> {
    let metadata = std::fs::metadata(root).map_err(|why| {
        BaseEvidenceLedgerRefusalV2::Io(format!(
            "Base ledger root {} must already exist: {why}",
            root.display()
        ))
    })?;
    if !metadata.is_dir() {
        return Err(BaseEvidenceLedgerRefusalV2::Io(format!(
            "Base ledger root {} is not a directory",
            root.display()
        )));
    }
    root.canonicalize().map_err(|why| {
        BaseEvidenceLedgerRefusalV2::Io(format!(
            "Base ledger root {} cannot be admitted canonically: {why}",
            root.display()
        ))
    })
}

fn open_file(
    path: &Path,
    writable: bool,
    create: bool,
) -> Result<File, BaseEvidenceLedgerRefusalV2> {
    OpenOptions::new()
        .read(true)
        .write(writable)
        .create(create)
        .open(path)
        .map_err(|why| io_error("open", path, &why))
}

fn sync_directory(root_file: &File, root: &Path) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    root_file.sync_all().map_err(|why| {
        BaseEvidenceLedgerRefusalV2::Io(format!(
            "cannot sync held Base ledger directory {}: {why}",
            root.display()
        ))
    })
}

fn file_generation(
    file: &File,
    path: &Path,
) -> Result<FileGenerationV2, BaseEvidenceLedgerRefusalV2> {
    let held = file
        .metadata()
        .map_err(|why| io_error("stat open file", path, &why))?;
    let named = std::fs::metadata(path).map_err(|why| io_error("stat named file", path, &why))?;
    let held = generation_of(&held);
    let named = generation_of(&named);
    #[cfg(unix)]
    if (held.device, held.inode) != (named.device, named.inode) {
        return Err(BaseEvidenceLedgerRefusalV2::Io(format!(
            "{} no longer names the opened Base file",
            path.display()
        )));
    }
    if held != named {
        return Err(BaseEvidenceLedgerRefusalV2::Io(format!(
            "{} changed while its Base generation was measured",
            path.display()
        )));
    }
    Ok(held)
}

#[cfg(unix)]
fn generation_of(metadata: &std::fs::Metadata) -> FileGenerationV2 {
    FileGenerationV2 {
        len: metadata.len(),
        device: metadata.dev(),
        inode: metadata.ino(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

#[cfg(not(unix))]
fn generation_of(metadata: &std::fs::Metadata) -> FileGenerationV2 {
    FileGenerationV2 {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    }
}

fn require_generation(
    expected: FileGenerationV2,
    file: &File,
    path: &Path,
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    if file_generation(file, path)? == expected {
        Ok(())
    } else {
        Err(BaseEvidenceLedgerRefusalV2::Io(format!(
            "{} changed since open; cached Base authority is stale",
            path.display()
        )))
    }
}

fn decode_span(raw: &[u8]) -> Result<RequestedSpanIdentityV1, BaseEvidenceLedgerRefusalV2> {
    if raw.len() != 20 || get_u32(raw, 0)? != 1 {
        return Err(BaseEvidenceLedgerRefusalV2::Codec(
            "completion requested-span version is unknown".to_owned(),
        ));
    }
    RequestedSpanIdentityV1::new(
        u16::try_from(get_u32(raw, 4)?).map_err(|_| {
            BaseEvidenceLedgerRefusalV2::Codec("span from-year exceeds u16".to_owned())
        })?,
        u8::try_from(get_u32(raw, 8)?).map_err(|_| {
            BaseEvidenceLedgerRefusalV2::Codec("span from-month exceeds u8".to_owned())
        })?,
        u16::try_from(get_u32(raw, 12)?).map_err(|_| {
            BaseEvidenceLedgerRefusalV2::Codec("span to-year exceeds u16".to_owned())
        })?,
        u8::try_from(get_u32(raw, 16)?).map_err(|_| {
            BaseEvidenceLedgerRefusalV2::Codec("span to-month exceeds u8".to_owned())
        })?,
    )
    .map_err(BaseEvidenceLedgerRefusalV2::Codec)
}

const fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn family_from_byte(value: u8) -> Result<InstrumentFamilyV1, BaseEvidenceLedgerRefusalV2> {
    match value {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(BaseEvidenceLedgerRefusalV2::Codec(format!(
            "completion family byte {value} is unknown"
        ))),
    }
}

fn require_bound(
    resource: &'static str,
    actual: u64,
    maximum: u64,
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(BaseEvidenceLedgerRefusalV2::BoundExceeded {
            resource,
            actual,
            maximum,
        })
    }
}

fn require_nonzero(name: &str, value: [u8; 32]) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    if value == [0; 32] {
        Err(BaseEvidenceLedgerRefusalV2::Reconciliation(format!(
            "{name} identity is all zero"
        )))
    } else {
        Ok(())
    }
}

fn put_bytes(
    raw: &mut [u8],
    offset: usize,
    value: &[u8],
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    raw.get_mut(offset..offset.saturating_add(value.len()))
        .ok_or_else(|| {
            BaseEvidenceLedgerRefusalV2::Codec(format!(
                "encode field at {offset} exceeds fixed record"
            ))
        })?
        .copy_from_slice(value);
    Ok(())
}

fn put_u32(raw: &mut [u8], offset: usize, value: u32) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    put_bytes(raw, offset, &value.to_le_bytes())
}

fn put_u64(raw: &mut [u8], offset: usize, value: u64) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    put_bytes(raw, offset, &value.to_le_bytes())
}

fn get_u32(raw: &[u8], offset: usize) -> Result<u32, BaseEvidenceLedgerRefusalV2> {
    let value: [u8; 4] = raw
        .get(offset..offset.saturating_add(4))
        .ok_or_else(|| BaseEvidenceLedgerRefusalV2::Codec(format!("decode lacks u32 at {offset}")))?
        .try_into()
        .map_err(|_| BaseEvidenceLedgerRefusalV2::Codec("u32 width changed".to_owned()))?;
    Ok(u32::from_le_bytes(value))
}

fn get_u64(raw: &[u8], offset: usize) -> Result<u64, BaseEvidenceLedgerRefusalV2> {
    let value: [u8; 8] = raw
        .get(offset..offset.saturating_add(8))
        .ok_or_else(|| BaseEvidenceLedgerRefusalV2::Codec(format!("decode lacks u64 at {offset}")))?
        .try_into()
        .map_err(|_| BaseEvidenceLedgerRefusalV2::Codec("u64 width changed".to_owned()))?;
    Ok(u64::from_le_bytes(value))
}

fn get_32(raw: &[u8], offset: usize) -> Result<[u8; 32], BaseEvidenceLedgerRefusalV2> {
    raw.get(offset..offset.saturating_add(32))
        .ok_or_else(|| {
            BaseEvidenceLedgerRefusalV2::Codec(format!("decode lacks digest at {offset}"))
        })?
        .try_into()
        .map_err(|_| BaseEvidenceLedgerRefusalV2::Codec("digest width changed".to_owned()))
}

fn require_zero(
    raw: &[u8],
    offset: usize,
    len: usize,
    name: &str,
) -> Result<(), BaseEvidenceLedgerRefusalV2> {
    if raw
        .get(offset..offset.saturating_add(len))
        .is_some_and(|bytes| bytes.iter().all(|byte| *byte == 0))
    {
        Ok(())
    } else {
        Err(BaseEvidenceLedgerRefusalV2::Codec(format!(
            "{name} is nonzero or absent"
        )))
    }
}

fn digest_domain(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(bytes);
    hasher.finalize()
}

fn io_error(action: &str, path: &Path, why: &std::io::Error) -> BaseEvidenceLedgerRefusalV2 {
    BaseEvidenceLedgerRefusalV2::Io(format!("cannot {action} {}: {why}", path.display()))
}

fn hex32(bytes: [u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(64);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "adversarial ledger fixtures keep each asserted setup invariant explicit"
)]
mod projection_tests {
    use super::super::{
        BaseEvidenceBoundsV2, BaseEvidenceDraftV2, BaseEvidenceFamilySourceV2, CandidateSubjectV2,
        MaskSupportEvidenceV2,
    };
    use super::*;
    use runner::grid::Cell;

    fn record(tag: u8, family: InstrumentFamilyV1) -> BaseEvidenceRecordV2 {
        let family_byte = match family {
            InstrumentFamilyV1::Nifty => 1,
            InstrumentFamilyV1::BankNifty => 2,
        };
        let source = BaseEvidenceFamilySourceV2 {
            universe_id: [tag; 32],
            completion_digest: [tag.saturating_add(1); 32],
            ordered_row_digest: [tag.saturating_add(2); 32],
            row_count: 1,
        };
        let draft = BaseEvidenceDraftV2::measure(
            CandidateSubjectV2 {
                candidate_universe_id: source.universe_id,
                candidate_sequence: 0,
                candidate_semantic_id: [tag.saturating_add(3); 32],
                candidate_row_digest: [tag.saturating_add(4); 32],
                execution_run_id: [tag.saturating_add(5); 32],
                evaluated_grid_digest: [tag.saturating_add(6); 32],
                cell_ordinal: 0,
                family: family_byte,
                direction: 1,
                rung_seconds: 60,
                horizon_bars: 1,
            },
            MaskSupportEvidenceV2 {
                mask_words: [1, 0, 0, 0, 0, 0],
                support_hits: 1,
                independent_sessions: 1,
            },
            &Cell::default(),
            &[],
            BaseEvidenceBoundsV2::new(2, 2, 2, 2).expect("fixture bounds are nonzero"),
        )
        .expect("empty exact trade sequence still forms canonical Base evidence");
        BaseEvidenceRecordV2::seal(source, &draft).expect("fixture Base record seals")
    }

    fn completion(
        record: &BaseEvidenceRecordV2,
        family: InstrumentFamilyV1,
        first_record: u64,
    ) -> CompletionV2 {
        let mut completion = CompletionV2 {
            completion_id: [1; 32],
            candidate_universe_id: record.draft.subject.candidate_universe_id,
            candidate_completion_digest: record.candidate_completion_digest,
            candidate_ordered_row_digest: record.candidate_ordered_row_digest,
            base_policy_digest: record.base_policy_digest,
            ordered_base_record_digest: [1; 32],
            feed_digest: [31; 32],
            source_commit_digest: [32; 32],
            vocabulary_digest: [33; 32],
            evaluation_policy_digest: [34; 32],
            calendar_policy_digest: [35; 32],
            daily_reference_policy_digest: [36; 32],
            first_record,
            candidate_row_count: 1,
            base_record_count: 1,
            family,
            rung_seconds: 60,
            horizon_bars: 1,
            requested_span: RequestedSpanIdentityV1::new(2025, 1, 2025, 1)
                .expect("fixture requested span is canonical"),
        };
        let raw = record.encode().expect("fixture record encodes");
        let mut ordered = ordered_hasher(&completion);
        ordered.update(&raw);
        completion.ordered_base_record_digest = ordered.finalize();
        completion.completion_id = completion.derive_id();
        completion.validate().expect("fixture completion validates");
        completion
    }

    fn paired_fixture(root: &Path) -> PairedBaseEvidenceAuthorityV2 {
        let bounds = BaseEvidenceLedgerBoundsV2::new(4, 4).expect("fixture bounds are nonzero");
        drop(LedgerV2::open(root, bounds).expect("empty fixture ledger initializes"));
        let nifty_record = record(41, InstrumentFamilyV1::Nifty);
        let banknifty_record = record(51, InstrumentFamilyV1::BankNifty);
        let nifty_completion = completion(&nifty_record, InstrumentFamilyV1::Nifty, 0);
        let banknifty_completion = completion(&banknifty_record, InstrumentFamilyV1::BankNifty, 1);

        let mut records = OpenOptions::new()
            .append(true)
            .open(root.join(RECORD_FILE))
            .expect("fixture record file opens");
        records
            .write_all(&nifty_record.encode().expect("NIFTY record encodes"))
            .expect("NIFTY record appends");
        records
            .write_all(&banknifty_record.encode().expect("BANKNIFTY record encodes"))
            .expect("BANKNIFTY record appends");
        records.sync_data().expect("fixture records sync");

        let mut completions = OpenOptions::new()
            .append(true)
            .open(root.join(COMPLETION_FILE))
            .expect("fixture completion file opens");
        completions
            .write_all(&nifty_completion.encode().expect("NIFTY completion encodes"))
            .expect("NIFTY completion appends");
        completions
            .write_all(
                &banknifty_completion
                    .encode()
                    .expect("BANKNIFTY completion encodes"),
            )
            .expect("BANKNIFTY completion appends");
        completions.sync_data().expect("fixture completions sync");

        pair_base_evidence_authority_v2(
            &BaseEvidenceReopenAuditV2 {
                completion: nifty_completion,
            },
            &BaseEvidenceReopenAuditV2 {
                completion: banknifty_completion,
            },
        )
        .expect("fixture pair is canonical")
    }

    #[test]
    fn paired_reader_projects_exact_nifty_then_banknifty_ordinals_without_reopen() {
        let root = TestRoot::new();
        let pair = paired_fixture(root.ledger());
        let bounds = BaseEvidenceLedgerBoundsV2::new(4, 4).expect("fixture bounds are nonzero");
        let reader = BaseEvidenceLedgerReaderV2::open(root.ledger(), bounds)
            .expect("bounded ledger opens once");
        let mut paired = reader
            .bind_pair(&pair)
            .expect("opaque pair binds to the held ledger");
        assert_eq!(paired.candidate_count(), 2);
        assert_eq!(paired.authority(), &pair);

        let nifty = paired.candidate(0).expect("global ordinal zero is NIFTY");
        assert_eq!(nifty.pair_id(), pair.pair_id());
        assert_eq!(nifty.global_sequence(), 0);
        assert_eq!(nifty.family(), InstrumentFamilyV1::Nifty);
        assert_eq!(nifty.family_sequence(), 0);
        assert_eq!(nifty.record().candidate_universe_id(), [41; 32]);
        assert_eq!(nifty.record().candidate_semantic_id(), [44; 32]);

        let banknifty = paired
            .candidate(1)
            .expect("first ordinal after NIFTY is BANKNIFTY zero");
        assert_eq!(banknifty.global_sequence(), 1);
        assert_eq!(banknifty.family(), InstrumentFamilyV1::BankNifty);
        assert_eq!(banknifty.family_sequence(), 0);
        assert_eq!(banknifty.record().candidate_universe_id(), [51; 32]);
        assert_eq!(banknifty.record().candidate_semantic_id(), [54; 32]);
        assert!(matches!(
            paired.candidate(2),
            Err(BaseEvidenceLedgerRefusalV2::BoundExceeded {
                resource: "paired candidate ordinal",
                actual: 2,
                maximum: 1,
            })
        ));
    }

    #[test]
    fn read_scope_refuses_post_action_generation_change_and_replaced_root() {
        let root = TestRoot::new();
        let bounds = BaseEvidenceLedgerBoundsV2::new(4, 4).expect("fixture bounds are nonzero");
        let mut ledger = LedgerV2::open(root.ledger(), bounds).expect("fixture ledger opens");
        let completion_path = ledger.completion_path.clone();
        let changed = ledger
            .with_shared_lock(|_| {
                OpenOptions::new()
                    .append(true)
                    .open(&completion_path)
                    .and_then(|mut file| file.write_all(&[0]))
                    .map_err(|why| {
                        io_error("mutate completion during read test", &completion_path, &why)
                    })?;
                Ok(())
            })
            .expect_err("post-action generation validation must observe a concurrent mutation");
        assert!(changed.to_string().contains("changed"));
        drop(ledger);

        let replace = TestRoot::new();
        let held = LedgerV2::open(replace.ledger(), bounds).expect("replacement fixture opens");
        let displaced = replace.parent.join("displaced-ledger");
        std::fs::rename(replace.ledger(), &displaced).expect("held root is displaced");
        std::fs::create_dir(replace.ledger()).expect("replacement root is created");
        let refused = held
            .require_unchanged()
            .expect_err("same pathname at another inode must not retain authority");
        assert!(refused.to_string().contains("no longer names"));
    }

    struct TestRoot {
        parent: PathBuf,
        ledger: PathBuf,
    }

    impl TestRoot {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let nth = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let parent = std::env::temp_dir().join(format!(
                "brutex-base-projection-{}-{nth}",
                std::process::id()
            ));
            let ledger = parent.join("ledger");
            std::fs::create_dir_all(&ledger).expect("fixture ledger root is writable");
            Self { parent, ledger }
        }

        fn ledger(&self) -> &Path {
            &self.ledger
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.parent);
        }
    }
}
