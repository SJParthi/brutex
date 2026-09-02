//! Append-only durable authority for exact-grid Anchored Search V4.
//!
//! One logical append is exactly two fixed-width member records in canonical
//! `NSE-NIFTY`, then `NSE-BANKNIFTY` order followed by one receipt-last
//! Completion record.  V3 has different files, magic, version and domains;
//! this module neither opens nor reinterprets V3 bytes.
//!
//! A structural reopen proves only that the bounded bytes are internally
//! consistent.  The authenticated capability is returned only after those
//! bytes are freshly reopened and compared with both opaque Runner V4
//! projections again.  Every V4 equality component is retained independently:
//! policy; the five-part full signal source; aggregate, Long and Short grid
//! identities; family; walk; and all fixed outcome/population counts.
//!
//! # Cost
//!
//! Open, append, retry, lookup validation and authentication hash explicitly
//! bounded files and are O(file bytes + pair records), not O(1).  Fixed record
//! encode/decode and one prepared-pair comparison are O(1).  The in-memory
//! receipt lookup is average O(1) after bounded validation.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use std::os::unix::fs::OpenOptionsExt as _;

use brutex_core::blake3::Hasher;
use runner::validate::AnchoredSearchAuthorityProjectionV4;

/// Bytes in one fixed V4 member record.
pub(crate) const ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES: usize = 768;
/// Bytes in one receipt-last V4 Completion record.
pub(crate) const ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES: usize = 512;

/// Operator-facing refusal from this boundary.
pub(crate) type AnchoredSearchLineageV4Refusal = String;

const VERSION: u32 = 4;
const MEMBER_MAGIC: [u8; 16] = *b"BTX-SRCHV4-MEM\0\0";
const COMPLETION_MAGIC: [u8; 16] = *b"BTX-SRCHV4-CMP\0\0";
const MEMBER_DOMAIN: u32 = 1;
const COMPLETION_DOMAIN: u32 = 2;
const SEAL_BYTES: usize = 32;
const MEMBER_PAYLOAD_BYTES: usize = ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES - SEAL_BYTES;
const COMPLETION_PAYLOAD_BYTES: usize = ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES - SEAL_BYTES;
const SOURCE_AUTHORITY_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v4-source\0";
const MEMBER_ID_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v4-member\0";
const PAIR_ID_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v4-pair\0";
const ORDERED_MEMBER_BYTES_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v4-ordered-members\0";
const MEMBER_SEAL_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v4-member-seal\0";
const COMPLETION_SEAL_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v4-completion-seal\0";
const FILE_GENERATION_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v4-file\0";
const MEMBER_FILE: &str = "anchored-search-lineage-members-v4.bin";
const COMPLETION_FILE: &str = "anchored-search-lineage-completions-v4.bin";
const LOCK_FILE: &str = "anchored-search-lineage-v4.lock";
const LOCK_FILE_MAX_BYTES: u64 = 0;
const READ_CHUNK_BYTES: usize = 16 * 1_024;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () = assert!(MEMBER_PAYLOAD_BYTES + SEAL_BYTES == ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES);
const _: () =
    assert!(COMPLETION_PAYLOAD_BYTES + SEAL_BYTES == ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES);

/// Explicit nonzero physical ceilings for this ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnchoredSearchLineageV4Bounds {
    pair_records: u64,
    member_bytes: u64,
    completion_bytes: u64,
}

impl AnchoredSearchLineageV4Bounds {
    /// Constructs bounds large enough for `max_pair_records` complete pairs.
    // THE ANNOTATION THAT USED TO BE HERE HAS BEEN EARNED OUT. It read "the
    // authoritative CLI surface will construct explicit Search V4 bounds in
    // Step 4", and `ledger_v6::lineage_bounds` is that surface. `expect` rather
    // than `allow` is what made this self-correcting: the build failed the
    // moment the prediction came true, instead of leaving a stale claim in the
    // tree for a reader to trust.
    pub(crate) fn new(
        max_pair_records: u64,
        max_member_bytes: u64,
        max_completion_bytes: u64,
    ) -> Result<Self, AnchoredSearchLineageV4Refusal> {
        for (name, value) in [
            ("pair records", max_pair_records),
            ("member bytes", max_member_bytes),
            ("Completion bytes", max_completion_bytes),
        ] {
            if value == 0 {
                return Err(format!(
                    "anchored search-lineage V4 {name} bound must be nonzero"
                ));
            }
        }
        let member_records = max_pair_records.checked_mul(2).ok_or_else(|| {
            "anchored search-lineage V4 member-record bound overflowed".to_owned()
        })?;
        let required_members = member_records
            .checked_mul(ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES as u64)
            .ok_or_else(|| "anchored search-lineage V4 member-byte bound overflowed".to_owned())?;
        if max_member_bytes < required_members {
            return Err(format!(
                "anchored search-lineage V4 member-byte bound {max_member_bytes} cannot hold {max_pair_records} pairs ({required_members} bytes)"
            ));
        }
        let required_completions = max_pair_records
            .checked_mul(ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES as u64)
            .ok_or_else(|| {
                "anchored search-lineage V4 Completion-byte bound overflowed".to_owned()
            })?;
        if max_completion_bytes < required_completions {
            return Err(format!(
                "anchored search-lineage V4 Completion-byte bound {max_completion_bytes} cannot hold {max_pair_records} pairs ({required_completions} bytes)"
            ));
        }
        Ok(Self {
            pair_records: max_pair_records,
            member_bytes: max_member_bytes,
            completion_bytes: max_completion_bytes,
        })
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchFamilyV4 {
    Nifty = 1,
    BankNifty = 2,
}

impl SearchFamilyV4 {
    fn decode(value: u8) -> Result<Self, AnchoredSearchLineageV4Refusal> {
        match value {
            1 => Ok(Self::Nifty),
            2 => Ok(Self::BankNifty),
            _ => Err(format!(
                "anchored search-lineage family tag {value} is outside the fixed V4 domain"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MemberRecordV4 {
    block_sequence: u64,
    family: SearchFamilyV4,
    validation_policy_id: [u8; 32],
    signal_digest: [u8; 32],
    signal_bars: u64,
    signal_first_ts_micros: i64,
    signal_last_ts_micros: i64,
    signal_column_digest: [u8; 32],
    aggregate_grid_id: [u8; 32],
    long_policy_id: [u8; 32],
    long_resolution_id: [u8; 32],
    short_policy_id: [u8; 32],
    short_resolution_id: [u8; 32],
    validation_family_id: [u8; 32],
    walk_facts_id: [u8; 32],
    source_authority_id: [u8; 32],
    member_id: [u8; 32],
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
    evaluated_population_cells: u64,
}

impl MemberRecordV4 {
    fn from_projection(
        family: SearchFamilyV4,
        projection: &AnchoredSearchAuthorityProjectionV4,
    ) -> Result<Self, AnchoredSearchLineageV4Refusal> {
        let source = projection.source_identity();
        let long = projection.long_grid_identity();
        let short = projection.short_grid_identity();
        let mut value = Self {
            block_sequence: 0,
            family,
            validation_policy_id: projection.policy_identity().digest(),
            signal_digest: source.signal_digest(),
            signal_bars: source.signal_bars(),
            signal_first_ts_micros: source.signal_first_ts_micros(),
            signal_last_ts_micros: source.signal_last_ts_micros(),
            signal_column_digest: source.signal_column_digest(),
            aggregate_grid_id: projection.grid_identity().digest(),
            long_policy_id: long.policy_digest(),
            long_resolution_id: long.resolution_digest(),
            short_policy_id: short.policy_digest(),
            short_resolution_id: short.resolution_digest(),
            validation_family_id: projection.family_identity().digest(),
            walk_facts_id: projection.walk_identity().digest(),
            source_authority_id: [0; 32],
            member_id: [0; 32],
            fold_count: projection.fold_count(),
            decided_folds: projection.decided_folds(),
            profitable_oos_folds: projection.profitable_oos_folds(),
            aggregate_oos_paisa: projection.aggregate_oos_paisa(),
            evaluated_population_cells: projection.evaluated_population_cells(),
        };
        value.source_authority_id = value.derive_source_authority_id();
        value.member_id = value.derive_member_id();
        value.validate()?;
        Ok(value)
    }

    const fn with_sequence(mut self, sequence: u64) -> Self {
        self.block_sequence = sequence;
        self
    }

    fn validate(self) -> Result<(), AnchoredSearchLineageV4Refusal> {
        for (name, identity) in [
            ("validation policy", self.validation_policy_id),
            ("signal", self.signal_digest),
            ("signal column", self.signal_column_digest),
            ("aggregate grid", self.aggregate_grid_id),
            ("Long policy", self.long_policy_id),
            ("Long resolution", self.long_resolution_id),
            ("Short policy", self.short_policy_id),
            ("Short resolution", self.short_resolution_id),
            ("validation family", self.validation_family_id),
            ("walk facts", self.walk_facts_id),
            ("source authority", self.source_authority_id),
            ("member", self.member_id),
        ] {
            if identity == [0; 32] {
                return Err(format!(
                    "anchored search-lineage V4 {name} identity is zero"
                ));
            }
        }
        if self.signal_bars == 0
            || self.signal_first_ts_micros > self.signal_last_ts_micros
            || (self.signal_bars == 1 && self.signal_first_ts_micros != self.signal_last_ts_micros)
            || (self.signal_bars > 1 && self.signal_first_ts_micros >= self.signal_last_ts_micros)
        {
            return Err("anchored search-lineage V4 signal extent is invalid".to_owned());
        }
        if self.long_policy_id == self.short_policy_id
            || self.long_resolution_id == self.short_resolution_id
        {
            return Err("anchored search-lineage V4 Long and Short grids alias".to_owned());
        }
        if self.fold_count == 0
            || self.decided_folds > self.fold_count
            || self.profitable_oos_folds > self.decided_folds
        {
            return Err(
                "anchored search-lineage V4 fold/population hierarchy is invalid".to_owned(),
            );
        }
        if (self.decided_folds == 0 && self.aggregate_oos_paisa != 0)
            || (self.profitable_oos_folds == 0 && self.aggregate_oos_paisa > 0)
            || (self.profitable_oos_folds == self.decided_folds
                && self.decided_folds > 0
                && self.aggregate_oos_paisa <= 0)
        {
            return Err("anchored search-lineage V4 outcome hierarchy is invalid".to_owned());
        }
        if self.source_authority_id != self.derive_source_authority_id() {
            return Err("anchored search-lineage V4 source-authority identity mismatch".to_owned());
        }
        if self.member_id != self.derive_member_id() {
            return Err("anchored search-lineage V4 member identity mismatch".to_owned());
        }
        Ok(())
    }

    fn derive_source_authority_id(self) -> [u8; 32] {
        hash_slices(
            SOURCE_AUTHORITY_DOMAIN,
            &[
                &self.validation_policy_id,
                &self.signal_digest,
                &self.signal_bars.to_le_bytes(),
                &self.signal_first_ts_micros.to_le_bytes(),
                &self.signal_last_ts_micros.to_le_bytes(),
                &self.signal_column_digest,
                &self.aggregate_grid_id,
                &self.long_policy_id,
                &self.long_resolution_id,
                &self.short_policy_id,
                &self.short_resolution_id,
                &self.validation_family_id,
                &self.walk_facts_id,
                &self.fold_count.to_le_bytes(),
                &self.decided_folds.to_le_bytes(),
                &self.profitable_oos_folds.to_le_bytes(),
                &self.aggregate_oos_paisa.to_le_bytes(),
                &self.evaluated_population_cells.to_le_bytes(),
            ],
        )
    }

    fn derive_member_id(self) -> [u8; 32] {
        hash_slices(
            MEMBER_ID_DOMAIN,
            &[&[self.family as u8], &self.source_authority_id],
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PreparedPairV4 {
    nifty: MemberRecordV4,
    banknifty: MemberRecordV4,
    pair_id: [u8; 32],
}

impl PreparedPairV4 {
    fn from_opaque(
        nifty: &AnchoredSearchAuthorityProjectionV4,
        banknifty: &AnchoredSearchAuthorityProjectionV4,
    ) -> Result<Self, AnchoredSearchLineageV4Refusal> {
        let nifty = MemberRecordV4::from_projection(SearchFamilyV4::Nifty, nifty)?;
        let banknifty = MemberRecordV4::from_projection(SearchFamilyV4::BankNifty, banknifty)?;
        if nifty.source_authority_id == banknifty.source_authority_id {
            return Err(
                "anchored search-lineage V4 NIFTY and BANKNIFTY source authorities alias"
                    .to_owned(),
            );
        }
        Ok(Self {
            pair_id: derive_pair_id(nifty.member_id, banknifty.member_id),
            nifty,
            banknifty,
        })
    }

    const fn members(self, sequence: u64) -> [MemberRecordV4; 2] {
        [
            self.nifty.with_sequence(sequence),
            self.banknifty.with_sequence(sequence),
        ]
    }

    fn completion(
        self,
        sequence: u64,
    ) -> Result<CompletionRecordV4, AnchoredSearchLineageV4Refusal> {
        let members = self.members(sequence);
        let member_bytes = encode_members(&members)?;
        Ok(CompletionRecordV4 {
            block_sequence: sequence,
            pair_id: self.pair_id,
            nifty_member_id: self.nifty.member_id,
            banknifty_member_id: self.banknifty.member_id,
            ordered_member_bytes_digest: hash_slices(ORDERED_MEMBER_BYTES_DOMAIN, &[&member_bytes]),
            total_folds: checked_sum_u64(self.nifty.fold_count, self.banknifty.fold_count, "fold")?,
            total_decided: checked_sum_u64(
                self.nifty.decided_folds,
                self.banknifty.decided_folds,
                "decided",
            )?,
            total_profitable: checked_sum_u64(
                self.nifty.profitable_oos_folds,
                self.banknifty.profitable_oos_folds,
                "profitable",
            )?,
            aggregate_oos_paisa: self
                .nifty
                .aggregate_oos_paisa
                .checked_add(self.banknifty.aggregate_oos_paisa)
                .ok_or_else(|| {
                    "anchored search-lineage V4 paired OOS total overflowed".to_owned()
                })?,
            total_evaluated_population_cells: checked_sum_u64(
                self.nifty.evaluated_population_cells,
                self.banknifty.evaluated_population_cells,
                "evaluated-population",
            )?,
        })
    }
}

fn checked_sum_u64(
    left: u64,
    right: u64,
    name: &str,
) -> Result<u64, AnchoredSearchLineageV4Refusal> {
    left.checked_add(right)
        .ok_or_else(|| format!("anchored search-lineage V4 total {name} count overflowed"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CompletionRecordV4 {
    block_sequence: u64,
    pair_id: [u8; 32],
    nifty_member_id: [u8; 32],
    banknifty_member_id: [u8; 32],
    ordered_member_bytes_digest: [u8; 32],
    total_folds: u64,
    total_decided: u64,
    total_profitable: u64,
    aggregate_oos_paisa: i64,
    total_evaluated_population_cells: u64,
}

/// Public structural proof of one complete canonical V4 pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnchoredSearchLineageV4StructuralReceipt {
    block_sequence: u64,
    pair_id: [u8; 32],
    nifty_member_id: [u8; 32],
    banknifty_member_id: [u8; 32],
    total_folds: u64,
    total_decided: u64,
    total_profitable: u64,
    aggregate_oos_paisa: i64,
    total_evaluated_population_cells: u64,
}

impl AnchoredSearchLineageV4StructuralReceipt {
    pub(crate) const fn block_sequence(self) -> u64 {
        self.block_sequence
    }
    pub(crate) const fn pair_id(self) -> [u8; 32] {
        self.pair_id
    }
    pub(crate) const fn nifty_member_id(self) -> [u8; 32] {
        self.nifty_member_id
    }
    pub(crate) const fn banknifty_member_id(self) -> [u8; 32] {
        self.banknifty_member_id
    }
    pub(crate) const fn total_folds(self) -> u64 {
        self.total_folds
    }
    pub(crate) const fn total_decided(self) -> u64 {
        self.total_decided
    }
    pub(crate) const fn total_profitable(self) -> u64 {
        self.total_profitable
    }
    pub(crate) const fn aggregate_oos_paisa(self) -> i64 {
        self.aggregate_oos_paisa
    }
    pub(crate) const fn total_evaluated_population_cells(self) -> u64 {
        self.total_evaluated_population_cells
    }
}

/// Fresh-reopen authenticated canonical V4 pair.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct AuthenticatedAnchoredSearchLineageV4 {
    receipt: AnchoredSearchLineageV4StructuralReceipt,
    nifty: MemberRecordV4,
    banknifty: MemberRecordV4,
}

impl AuthenticatedAnchoredSearchLineageV4 {
    pub(crate) const fn structural_receipt(self) -> AnchoredSearchLineageV4StructuralReceipt {
        self.receipt
    }
    pub(crate) const fn nifty(self) -> AnchoredSearchLineageMemberAuthorityV4 {
        AnchoredSearchLineageMemberAuthorityV4 { member: self.nifty }
    }
    pub(crate) const fn banknifty(self) -> AnchoredSearchLineageMemberAuthorityV4 {
        AnchoredSearchLineageMemberAuthorityV4 {
            member: self.banknifty,
        }
    }
}

/// Exact durable equality components for one family member.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnchoredSearchLineageMemberAuthorityV4 {
    member: MemberRecordV4,
}

impl AnchoredSearchLineageMemberAuthorityV4 {
    pub(crate) const fn member_id(self) -> [u8; 32] {
        self.member.member_id
    }
    pub(crate) const fn source_authority_id(self) -> [u8; 32] {
        self.member.source_authority_id
    }
    pub(crate) const fn validation_policy_id(self) -> [u8; 32] {
        self.member.validation_policy_id
    }
    pub(crate) const fn signal_digest(self) -> [u8; 32] {
        self.member.signal_digest
    }
    pub(crate) const fn signal_bars(self) -> u64 {
        self.member.signal_bars
    }
    pub(crate) const fn signal_first_ts_micros(self) -> i64 {
        self.member.signal_first_ts_micros
    }
    pub(crate) const fn signal_last_ts_micros(self) -> i64 {
        self.member.signal_last_ts_micros
    }
    pub(crate) const fn signal_column_digest(self) -> [u8; 32] {
        self.member.signal_column_digest
    }
    pub(crate) const fn aggregate_grid_id(self) -> [u8; 32] {
        self.member.aggregate_grid_id
    }
    pub(crate) const fn long_policy_id(self) -> [u8; 32] {
        self.member.long_policy_id
    }
    pub(crate) const fn long_resolution_id(self) -> [u8; 32] {
        self.member.long_resolution_id
    }
    pub(crate) const fn short_policy_id(self) -> [u8; 32] {
        self.member.short_policy_id
    }
    pub(crate) const fn short_resolution_id(self) -> [u8; 32] {
        self.member.short_resolution_id
    }
    pub(crate) const fn validation_family_id(self) -> [u8; 32] {
        self.member.validation_family_id
    }
    pub(crate) const fn walk_facts_id(self) -> [u8; 32] {
        self.member.walk_facts_id
    }
    pub(crate) const fn fold_count(self) -> u64 {
        self.member.fold_count
    }
    pub(crate) const fn decided_folds(self) -> u64 {
        self.member.decided_folds
    }
    pub(crate) const fn profitable_oos_folds(self) -> u64 {
        self.member.profitable_oos_folds
    }
    pub(crate) const fn aggregate_oos_paisa(self) -> i64 {
        self.member.aggregate_oos_paisa
    }
    pub(crate) const fn evaluated_population_cells(self) -> u64 {
        self.member.evaluated_population_cells
    }
}

/// Result of an authenticated append or exact idempotent retry.
pub(crate) enum AnchoredSearchLineageV4AuthenticatedCommit {
    Written(AuthenticatedAnchoredSearchLineageV4),
    Reused(AuthenticatedAnchoredSearchLineageV4),
}

impl AnchoredSearchLineageV4AuthenticatedCommit {
    pub(crate) const fn authority(self) -> AuthenticatedAnchoredSearchLineageV4 {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TrailingPairV4 {
    first_member_record: u64,
    pair: PreparedPairV4,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
impl PlatformIdentity {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MetadataGeneration {
    identity: PlatformIdentity,
    len: u64,
    mode: u32,
    links: u64,
    owner: u32,
    group: u32,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
impl MetadataGeneration {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            identity: PlatformIdentity::of(metadata),
            len: metadata.len(),
            mode: metadata.mode(),
            links: metadata.nlink(),
            owner: metadata.uid(),
            group: metadata.gid(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }
}

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformIdentity {
    len: u64,
}

#[cfg(not(unix))]
impl PlatformIdentity {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            len: metadata.len(),
        }
    }
}

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MetadataGeneration {
    identity: PlatformIdentity,
    len: u64,
    readonly: bool,
    modified: Option<std::time::SystemTime>,
}

#[cfg(not(unix))]
impl MetadataGeneration {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            identity: PlatformIdentity::of(metadata),
            len: metadata.len(),
            readonly: metadata.permissions().readonly(),
            modified: metadata.modified().ok(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    identity: PlatformIdentity,
    len: u64,
    digest: [u8; 32],
}

/// Bounded structural view of the V4 lineage files.
pub(crate) struct AnchoredSearchLineageV4Ledger {
    root: PathBuf,
    root_file: File,
    root_identity: PlatformIdentity,
    lock_path: PathBuf,
    lock_file: File,
    lock_generation: FileGeneration,
    member_path: PathBuf,
    member_file: File,
    member_generation: FileGeneration,
    completion_path: PathBuf,
    completion_file: File,
    completion_generation: FileGeneration,
    bounds: AnchoredSearchLineageV4Bounds,
    writable: bool,
    receipts: HashMap<[u8; 32], AnchoredSearchLineageV4StructuralReceipt>,
    trailing: Option<TrailingPairV4>,
    member_records: u64,
    completion_records: u64,
}

impl AnchoredSearchLineageV4Ledger {
    /// Opens existing V4 files read-only and validates every bounded record.
    pub(crate) fn open_read(
        root: &Path,
        bounds: AnchoredSearchLineageV4Bounds,
    ) -> Result<Self, AnchoredSearchLineageV4Refusal> {
        Self::open(root, bounds, false)
    }

    fn open_write(
        root: &Path,
        bounds: AnchoredSearchLineageV4Bounds,
    ) -> Result<Self, AnchoredSearchLineageV4Refusal> {
        Self::open(root, bounds, true)
    }

    fn open(
        root: &Path,
        bounds: AnchoredSearchLineageV4Bounds,
        writable: bool,
    ) -> Result<Self, AnchoredSearchLineageV4Refusal> {
        let (root, root_file, root_identity) = open_root(root)?;
        let lock_path = root.join(LOCK_FILE);
        let member_path = root.join(MEMBER_FILE);
        let completion_path = root.join(COMPLETION_FILE);
        let (lock_file, lock_created) = open_child(&lock_path, writable)?;
        if writable {
            lock_file.lock().map_err(|why| {
                format!("cannot exclusively lock search-lineage V4 ledger: {why}")
            })?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot shared-lock search-lineage V4 ledger: {why}"))?;
        }
        let opened = (|| {
            let (member_file, member_created) = open_child(&member_path, writable)?;
            let (completion_file, completion_created) = open_child(&completion_path, writable)?;
            if lock_created || member_created || completion_created {
                sync_directory(&root_file, &root)?;
            }
            if named_identity(&root)? != root_identity {
                return Err("anchored search-lineage V4 root changed while opening".to_owned());
            }
            let lock_generation = file_generation(&lock_file, &lock_path, LOCK_FILE_MAX_BYTES)?;
            let member_generation =
                file_generation(&member_file, &member_path, bounds.member_bytes)?;
            let completion_generation =
                file_generation(&completion_file, &completion_path, bounds.completion_bytes)?;
            let mut ledger = Self {
                root,
                root_file,
                root_identity,
                lock_path,
                lock_file: lock_file
                    .try_clone()
                    .map_err(|why| format!("cannot clone held search-lineage V4 lock: {why}"))?,
                lock_generation,
                member_path,
                member_file,
                member_generation,
                completion_path,
                completion_file,
                completion_generation,
                bounds,
                writable,
                receipts: HashMap::new(),
                trailing: None,
                member_records: 0,
                completion_records: 0,
            };
            ledger.scan()?;
            Ok(ledger)
        })();
        let released = lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock search-lineage V4 ledger after open: {why}"));
        combine_with_unlock(opened, released)
    }

    fn scan(&mut self) -> Result<(), AnchoredSearchLineageV4Refusal> {
        let member_records = record_count(
            self.member_generation.len,
            ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES,
            self.bounds.pair_records.saturating_mul(2),
            "member",
        )?;
        let completion_records = record_count(
            self.completion_generation.len,
            ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES,
            self.bounds.pair_records,
            "Completion",
        )?;
        let covered_members = completion_records.checked_mul(2).ok_or_else(|| {
            "anchored search-lineage V4 covered-member count overflowed".to_owned()
        })?;
        if covered_members > member_records {
            return Err(format!(
                "anchored search-lineage V4 is torn: {completion_records} Completions require {covered_members} members but file has {member_records}"
            ));
        }
        self.receipts.clear();
        self.receipts
            .try_reserve(
                usize::try_from(completion_records).map_err(|_| {
                    "search-lineage V4 Completion count does not fit usize".to_owned()
                })?,
            )
            .map_err(|why| format!("cannot reserve search-lineage V4 receipt index: {why}"))?;
        for sequence in 0..completion_records {
            let members = read_member_pair(&mut self.member_file, sequence)?;
            let completion =
                decode_completion(&read_completion_at(&mut self.completion_file, sequence)?)?;
            let receipt = validate_complete_pair(sequence, &members, completion)?;
            if self.receipts.insert(receipt.pair_id, receipt).is_some() {
                return Err(format!(
                    "anchored search-lineage V4 pair {} appears more than once",
                    hex32(receipt.pair_id)
                ));
            }
        }
        let trailing_members = member_records
            .checked_sub(covered_members)
            .ok_or_else(|| "anchored search-lineage V4 trailing-member underflow".to_owned())?;
        self.trailing = match trailing_members {
            0 => None,
            1 => {
                return Err(
                    "anchored search-lineage V4 has a partial trailing pair; history is not rewritten"
                        .to_owned(),
                );
            }
            2 => {
                let members = read_member_pair(&mut self.member_file, completion_records)?;
                let pair = validate_member_pair(completion_records, &members)?;
                if self.receipts.contains_key(&pair.pair_id) {
                    return Err(format!(
                        "anchored search-lineage V4 trailing pair {} duplicates a completed pair",
                        hex32(pair.pair_id)
                    ));
                }
                Some(TrailingPairV4 {
                    first_member_record: covered_members,
                    pair,
                })
            }
            _ => {
                return Err(format!(
                    "anchored search-lineage V4 has {trailing_members} unreceipted member records"
                ));
            }
        };
        self.member_records = member_records;
        self.completion_records = completion_records;
        self.require_unchanged()
    }

    /// Revalidates all bounded generations before and after an indexed lookup.
    pub(crate) fn structural_receipt(
        &self,
        pair_id: &[u8; 32],
    ) -> Result<Option<AnchoredSearchLineageV4StructuralReceipt>, AnchoredSearchLineageV4Refusal>
    {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot shared-lock search-lineage V4 lookup: {why}"))?;
        let result = self.structural_receipt_locked_with_after_lookup(pair_id, || Ok(()));
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock search-lineage V4 lookup: {why}"));
        combine_with_unlock(result, released)
    }

    fn structural_receipt_locked_with_after_lookup(
        &self,
        pair_id: &[u8; 32],
        after_lookup: impl FnOnce() -> Result<(), AnchoredSearchLineageV4Refusal>,
    ) -> Result<Option<AnchoredSearchLineageV4StructuralReceipt>, AnchoredSearchLineageV4Refusal>
    {
        self.require_unchanged()?;
        let value = self.receipts.get(pair_id).copied();
        after_lookup()?;
        self.require_unchanged()?;
        Ok(value)
    }

    fn append(
        &mut self,
        prepared: &PreparedPairV4,
    ) -> Result<(bool, AnchoredSearchLineageV4StructuralReceipt), AnchoredSearchLineageV4Refusal>
    {
        if !self.writable {
            return Err("anchored search-lineage V4 ledger is read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot exclusively lock search-lineage V4 append: {why}"))?;
        let result = self.append_locked(prepared);
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock search-lineage V4 append: {why}"));
        combine_with_unlock(result, released)
    }

    fn append_locked(
        &mut self,
        prepared: &PreparedPairV4,
    ) -> Result<(bool, AnchoredSearchLineageV4StructuralReceipt), AnchoredSearchLineageV4Refusal>
    {
        self.require_unchanged()?;
        if let Some(receipt) = self.receipts.get(&prepared.pair_id).copied() {
            self.require_exact_existing(prepared, receipt)?;
            self.member_file
                .sync_data()
                .map_err(|why| format!("cannot sync reused search-lineage V4 members: {why}"))?;
            self.completion_file
                .sync_data()
                .map_err(|why| format!("cannot sync reused search-lineage V4 Completion: {why}"))?;
            sync_directory(&self.root_file, &self.root)?;
            self.require_unchanged()?;
            return Ok((false, receipt));
        }
        if let Some(trailing) = self.trailing {
            if trailing.first_member_record != self.member_records.saturating_sub(2)
                || trailing.pair != *prepared
            {
                return Err(format!(
                    "anchored search-lineage V4 trailing pair {} is not exact retry {}",
                    hex32(trailing.pair.pair_id),
                    hex32(prepared.pair_id)
                ));
            }
            self.member_file
                .sync_data()
                .map_err(|why| format!("cannot sync retry search-lineage V4 members: {why}"))?;
            self.append_completion(prepared)?;
            let receipt = self
                .receipts
                .get(&prepared.pair_id)
                .copied()
                .ok_or_else(|| "retried search-lineage V4 pair was not indexed".to_owned())?;
            return Ok((true, receipt));
        }
        self.require_append_capacity()?;
        for raw in encode_member_records(&prepared.members(self.completion_records))? {
            append_raw(&mut self.member_file, &raw)?;
        }
        self.member_file
            .sync_data()
            .map_err(|why| format!("cannot sync search-lineage V4 members: {why}"))?;
        self.member_records = self
            .member_records
            .checked_add(2)
            .ok_or_else(|| "search-lineage V4 member count overflowed".to_owned())?;
        self.member_generation = file_generation(
            &self.member_file,
            &self.member_path,
            self.bounds.member_bytes,
        )?;
        self.append_completion(prepared)?;
        let receipt = self
            .receipts
            .get(&prepared.pair_id)
            .copied()
            .ok_or_else(|| "written search-lineage V4 pair was not indexed".to_owned())?;
        Ok((true, receipt))
    }

    fn append_completion(
        &mut self,
        prepared: &PreparedPairV4,
    ) -> Result<(), AnchoredSearchLineageV4Refusal> {
        let completion = prepared.completion(self.completion_records)?;
        append_raw(&mut self.completion_file, &encode_completion(&completion)?)?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync search-lineage V4 Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.completion_records = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "search-lineage V4 Completion count overflowed".to_owned())?;
        self.completion_generation = file_generation(
            &self.completion_file,
            &self.completion_path,
            self.bounds.completion_bytes,
        )?;
        self.scan()
    }

    fn require_exact_existing(
        &mut self,
        prepared: &PreparedPairV4,
        receipt: AnchoredSearchLineageV4StructuralReceipt,
    ) -> Result<(), AnchoredSearchLineageV4Refusal> {
        let members = read_member_pair(&mut self.member_file, receipt.block_sequence)?;
        if members != prepared.members(receipt.block_sequence) {
            return Err(
                "existing search-lineage V4 members differ from opaque preparation".to_owned(),
            );
        }
        let completion = decode_completion(&read_completion_at(
            &mut self.completion_file,
            receipt.block_sequence,
        )?)?;
        if completion != prepared.completion(receipt.block_sequence)? {
            return Err(
                "existing search-lineage V4 Completion differs from opaque preparation".to_owned(),
            );
        }
        Ok(())
    }

    fn require_append_capacity(&self) -> Result<(), AnchoredSearchLineageV4Refusal> {
        let next_pairs = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "search-lineage V4 pair count overflowed".to_owned())?;
        if next_pairs > self.bounds.pair_records {
            return Err(format!(
                "search-lineage V4 append reaches {next_pairs} pairs above explicit maximum {}",
                self.bounds.pair_records
            ));
        }
        let next_members = self
            .member_records
            .checked_add(2)
            .ok_or_else(|| "search-lineage V4 member count overflowed".to_owned())?;
        let next_member_bytes = next_members
            .checked_mul(ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES as u64)
            .ok_or_else(|| "search-lineage V4 member bytes overflowed".to_owned())?;
        if next_member_bytes > self.bounds.member_bytes {
            return Err("search-lineage V4 append exceeds explicit member-byte bound".to_owned());
        }
        let next_completion_bytes = next_pairs
            .checked_mul(ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES as u64)
            .ok_or_else(|| "search-lineage V4 Completion bytes overflowed".to_owned())?;
        if next_completion_bytes > self.bounds.completion_bytes {
            return Err(
                "search-lineage V4 append exceeds explicit Completion-byte bound".to_owned(),
            );
        }
        Ok(())
    }

    fn require_unchanged(&self) -> Result<(), AnchoredSearchLineageV4Refusal> {
        if named_identity(&self.root)? != self.root_identity
            || PlatformIdentity::of(
                &self
                    .root_file
                    .metadata()
                    .map_err(|why| format!("cannot stat held search-lineage V4 root: {why}"))?,
            ) != self.root_identity
        {
            return Err("anchored search-lineage V4 root changed after open".to_owned());
        }
        if file_generation(&self.lock_file, &self.lock_path, LOCK_FILE_MAX_BYTES)?
            != self.lock_generation
        {
            return Err("anchored search-lineage V4 lock generation changed".to_owned());
        }
        if file_generation(
            &self.member_file,
            &self.member_path,
            self.bounds.member_bytes,
        )? != self.member_generation
        {
            return Err("anchored search-lineage V4 member generation changed".to_owned());
        }
        if file_generation(
            &self.completion_file,
            &self.completion_path,
            self.bounds.completion_bytes,
        )? != self.completion_generation
        {
            return Err("anchored search-lineage V4 Completion generation changed".to_owned());
        }
        Ok(())
    }

    fn authenticate(
        &mut self,
        receipt: AnchoredSearchLineageV4StructuralReceipt,
        prepared: &PreparedPairV4,
    ) -> Result<AuthenticatedAnchoredSearchLineageV4, AnchoredSearchLineageV4Refusal> {
        self.require_unchanged()?;
        if self.receipts.get(&prepared.pair_id) != Some(&receipt) {
            return Err(
                "search-lineage V4 receipt is not the freshly reopened indexed pair".to_owned(),
            );
        }
        self.require_exact_existing(prepared, receipt)?;
        self.require_unchanged()?;
        Ok(AuthenticatedAnchoredSearchLineageV4 {
            receipt,
            nifty: prepared.nifty,
            banknifty: prepared.banknifty,
        })
    }
}

/// Persists and freshly reauthenticates one canonical NIFTY/BANKNIFTY pair.
pub(crate) fn persist_anchored_search_lineage_v4(
    root: &Path,
    bounds: AnchoredSearchLineageV4Bounds,
    nifty: &AnchoredSearchAuthorityProjectionV4,
    banknifty: &AnchoredSearchAuthorityProjectionV4,
) -> Result<AnchoredSearchLineageV4AuthenticatedCommit, AnchoredSearchLineageV4Refusal> {
    let prepared = PreparedPairV4::from_opaque(nifty, banknifty)?;
    let mut writer = AnchoredSearchLineageV4Ledger::open_write(root, bounds)?;
    let (written, receipt) = writer.append(&prepared)?;
    drop(writer);
    let mut reopened = AnchoredSearchLineageV4Ledger::open_read(root, bounds)?;
    let observed = reopened
        .structural_receipt(&prepared.pair_id)?
        .ok_or_else(|| "fresh search-lineage V4 reopen omitted the persisted pair".to_owned())?;
    if observed != receipt {
        return Err("fresh search-lineage V4 reopen changed the structural receipt".to_owned());
    }
    let authority = reopened.authenticate(observed, &prepared)?;
    Ok(if written {
        AnchoredSearchLineageV4AuthenticatedCommit::Written(authority)
    } else {
        AnchoredSearchLineageV4AuthenticatedCommit::Reused(authority)
    })
}

fn validate_complete_pair(
    sequence: u64,
    members: &[MemberRecordV4; 2],
    completion: CompletionRecordV4,
) -> Result<AnchoredSearchLineageV4StructuralReceipt, AnchoredSearchLineageV4Refusal> {
    let prepared = validate_member_pair(sequence, members)?;
    if completion != prepared.completion(sequence)? {
        return Err(format!(
            "anchored search-lineage V4 Completion {sequence} does not exactly bind its ordered members"
        ));
    }
    Ok(AnchoredSearchLineageV4StructuralReceipt {
        block_sequence: sequence,
        pair_id: completion.pair_id,
        nifty_member_id: completion.nifty_member_id,
        banknifty_member_id: completion.banknifty_member_id,
        total_folds: completion.total_folds,
        total_decided: completion.total_decided,
        total_profitable: completion.total_profitable,
        aggregate_oos_paisa: completion.aggregate_oos_paisa,
        total_evaluated_population_cells: completion.total_evaluated_population_cells,
    })
}

fn validate_member_pair(
    sequence: u64,
    members: &[MemberRecordV4; 2],
) -> Result<PreparedPairV4, AnchoredSearchLineageV4Refusal> {
    let nifty = members
        .first()
        .copied()
        .ok_or_else(|| "search-lineage V4 pair omitted NIFTY".to_owned())?;
    let banknifty = members
        .get(1)
        .copied()
        .ok_or_else(|| "search-lineage V4 pair omitted BANKNIFTY".to_owned())?;
    if nifty.block_sequence != sequence || banknifty.block_sequence != sequence {
        return Err(format!(
            "search-lineage V4 pair sequence does not match canonical {sequence}"
        ));
    }
    if nifty.family != SearchFamilyV4::Nifty || banknifty.family != SearchFamilyV4::BankNifty {
        return Err("search-lineage V4 members are not canonical NIFTY then BANKNIFTY".to_owned());
    }
    nifty.validate()?;
    banknifty.validate()?;
    if nifty.source_authority_id == banknifty.source_authority_id {
        return Err("search-lineage V4 NIFTY/BANKNIFTY source authorities alias".to_owned());
    }
    Ok(PreparedPairV4 {
        nifty: nifty.with_sequence(0),
        banknifty: banknifty.with_sequence(0),
        pair_id: derive_pair_id(nifty.member_id, banknifty.member_id),
    })
}

fn derive_pair_id(nifty_member_id: [u8; 32], banknifty_member_id: [u8; 32]) -> [u8; 32] {
    hash_slices(PAIR_ID_DOMAIN, &[&nifty_member_id, &banknifty_member_id])
}

fn encode_member_records(
    members: &[MemberRecordV4; 2],
) -> Result<[[u8; ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES]; 2], AnchoredSearchLineageV4Refusal> {
    Ok([
        encode_member(
            members
                .first()
                .ok_or_else(|| "cannot encode absent V4 NIFTY member".to_owned())?,
        )?,
        encode_member(
            members
                .get(1)
                .ok_or_else(|| "cannot encode absent V4 BANKNIFTY member".to_owned())?,
        )?,
    ])
}

fn encode_members(
    members: &[MemberRecordV4; 2],
) -> Result<Vec<u8>, AnchoredSearchLineageV4Refusal> {
    let records = encode_member_records(members)?;
    let capacity = ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES
        .checked_mul(2)
        .ok_or_else(|| "ordered search-lineage V4 member capacity overflowed".to_owned())?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|why| format!("cannot reserve ordered search-lineage V4 bytes: {why}"))?;
    for record in records {
        bytes.extend_from_slice(&record);
    }
    Ok(bytes)
}

fn encode_member(
    value: &MemberRecordV4,
) -> Result<[u8; ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES], AnchoredSearchLineageV4Refusal> {
    value.validate()?;
    let mut raw = [0_u8; ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES];
    let mut writer = FixedWriter::new(
        raw.get_mut(..MEMBER_PAYLOAD_BYTES)
            .ok_or_else(|| "search-lineage V4 member payload range is invalid".to_owned())?,
    );
    writer.write(&MEMBER_MAGIC)?;
    writer.u32(VERSION)?;
    writer.u32(MEMBER_DOMAIN)?;
    writer.u64(value.block_sequence)?;
    writer.u8(value.family as u8)?;
    writer.zeros(7)?;
    writer.write(&value.validation_policy_id)?;
    writer.write(&value.signal_digest)?;
    writer.u64(value.signal_bars)?;
    writer.i64(value.signal_first_ts_micros)?;
    writer.i64(value.signal_last_ts_micros)?;
    for identity in [
        value.signal_column_digest,
        value.aggregate_grid_id,
        value.long_policy_id,
        value.long_resolution_id,
        value.short_policy_id,
        value.short_resolution_id,
        value.validation_family_id,
        value.walk_facts_id,
        value.source_authority_id,
        value.member_id,
    ] {
        writer.write(&identity)?;
    }
    writer.u64(value.fold_count)?;
    writer.u64(value.decided_folds)?;
    writer.u64(value.profitable_oos_folds)?;
    writer.i64(value.aggregate_oos_paisa)?;
    writer.u64(value.evaluated_population_cells)?;
    writer.zeros_to_end()?;
    let seal = hash_slices(
        MEMBER_SEAL_DOMAIN,
        &[raw
            .get(..MEMBER_PAYLOAD_BYTES)
            .ok_or_else(|| "search-lineage V4 member payload is absent".to_owned())?],
    );
    raw.get_mut(MEMBER_PAYLOAD_BYTES..)
        .ok_or_else(|| "search-lineage V4 member seal range is absent".to_owned())?
        .copy_from_slice(&seal);
    Ok(raw)
}

fn decode_member(
    raw: &[u8; ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES],
) -> Result<MemberRecordV4, AnchoredSearchLineageV4Refusal> {
    let payload = raw
        .get(..MEMBER_PAYLOAD_BYTES)
        .ok_or_else(|| "search-lineage V4 member payload is absent".to_owned())?;
    let expected = hash_slices(MEMBER_SEAL_DOMAIN, &[payload]);
    if raw.get(MEMBER_PAYLOAD_BYTES..) != Some(expected.as_slice()) {
        return Err("anchored search-lineage V4 member seal mismatch".to_owned());
    }
    let mut reader = FixedReader::new(payload);
    reader.require(&MEMBER_MAGIC, "V4 member magic")?;
    reader.require_u32(VERSION, "V4 member version")?;
    reader.require_u32(MEMBER_DOMAIN, "V4 member domain")?;
    let block_sequence = reader.u64()?;
    let family = SearchFamilyV4::decode(reader.u8()?)?;
    reader.require_zero(7, "V4 member family reserve")?;
    let value = MemberRecordV4 {
        block_sequence,
        family,
        validation_policy_id: reader.array()?,
        signal_digest: reader.array()?,
        signal_bars: reader.u64()?,
        signal_first_ts_micros: reader.i64()?,
        signal_last_ts_micros: reader.i64()?,
        signal_column_digest: reader.array()?,
        aggregate_grid_id: reader.array()?,
        long_policy_id: reader.array()?,
        long_resolution_id: reader.array()?,
        short_policy_id: reader.array()?,
        short_resolution_id: reader.array()?,
        validation_family_id: reader.array()?,
        walk_facts_id: reader.array()?,
        source_authority_id: reader.array()?,
        member_id: reader.array()?,
        fold_count: reader.u64()?,
        decided_folds: reader.u64()?,
        profitable_oos_folds: reader.u64()?,
        aggregate_oos_paisa: reader.i64()?,
        evaluated_population_cells: reader.u64()?,
    };
    reader.require_zero_to_end("V4 member reserve")?;
    value.validate()?;
    Ok(value)
}

fn encode_completion(
    value: &CompletionRecordV4,
) -> Result<[u8; ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES], AnchoredSearchLineageV4Refusal> {
    validate_completion(*value)?;
    let mut raw = [0_u8; ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES];
    let mut writer = FixedWriter::new(
        raw.get_mut(..COMPLETION_PAYLOAD_BYTES)
            .ok_or_else(|| "search-lineage V4 Completion payload range is invalid".to_owned())?,
    );
    writer.write(&COMPLETION_MAGIC)?;
    writer.u32(VERSION)?;
    writer.u32(COMPLETION_DOMAIN)?;
    writer.u64(value.block_sequence)?;
    for identity in [
        value.pair_id,
        value.nifty_member_id,
        value.banknifty_member_id,
        value.ordered_member_bytes_digest,
    ] {
        writer.write(&identity)?;
    }
    writer.u64(value.total_folds)?;
    writer.u64(value.total_decided)?;
    writer.u64(value.total_profitable)?;
    writer.i64(value.aggregate_oos_paisa)?;
    writer.u64(value.total_evaluated_population_cells)?;
    writer.zeros_to_end()?;
    let seal = hash_slices(
        COMPLETION_SEAL_DOMAIN,
        &[raw
            .get(..COMPLETION_PAYLOAD_BYTES)
            .ok_or_else(|| "search-lineage V4 Completion payload is absent".to_owned())?],
    );
    raw.get_mut(COMPLETION_PAYLOAD_BYTES..)
        .ok_or_else(|| "search-lineage V4 Completion seal range is absent".to_owned())?
        .copy_from_slice(&seal);
    Ok(raw)
}

fn decode_completion(
    raw: &[u8; ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES],
) -> Result<CompletionRecordV4, AnchoredSearchLineageV4Refusal> {
    let payload = raw
        .get(..COMPLETION_PAYLOAD_BYTES)
        .ok_or_else(|| "search-lineage V4 Completion payload is absent".to_owned())?;
    let expected = hash_slices(COMPLETION_SEAL_DOMAIN, &[payload]);
    if raw.get(COMPLETION_PAYLOAD_BYTES..) != Some(expected.as_slice()) {
        return Err("anchored search-lineage V4 Completion seal mismatch".to_owned());
    }
    let mut reader = FixedReader::new(payload);
    reader.require(&COMPLETION_MAGIC, "V4 Completion magic")?;
    reader.require_u32(VERSION, "V4 Completion version")?;
    reader.require_u32(COMPLETION_DOMAIN, "V4 Completion domain")?;
    let value = CompletionRecordV4 {
        block_sequence: reader.u64()?,
        pair_id: reader.array()?,
        nifty_member_id: reader.array()?,
        banknifty_member_id: reader.array()?,
        ordered_member_bytes_digest: reader.array()?,
        total_folds: reader.u64()?,
        total_decided: reader.u64()?,
        total_profitable: reader.u64()?,
        aggregate_oos_paisa: reader.i64()?,
        total_evaluated_population_cells: reader.u64()?,
    };
    reader.require_zero_to_end("V4 Completion reserve")?;
    validate_completion(value)?;
    Ok(value)
}

fn validate_completion(value: CompletionRecordV4) -> Result<(), AnchoredSearchLineageV4Refusal> {
    for (name, identity) in [
        ("pair", value.pair_id),
        ("NIFTY member", value.nifty_member_id),
        ("BANKNIFTY member", value.banknifty_member_id),
        ("ordered-member bytes", value.ordered_member_bytes_digest),
    ] {
        if identity == [0; 32] {
            return Err(format!(
                "search-lineage V4 Completion {name} identity is zero"
            ));
        }
    }
    if value.total_folds == 0
        || value.total_decided > value.total_folds
        || value.total_profitable > value.total_decided
    {
        return Err("search-lineage V4 Completion count hierarchy is invalid".to_owned());
    }
    Ok(())
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    cursor: usize,
}

impl<'a> FixedWriter<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn write(&mut self, value: &[u8]) -> Result<(), AnchoredSearchLineageV4Refusal> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or_else(|| "search-lineage V4 writer cursor overflowed".to_owned())?;
        self.bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "search-lineage V4 fixed record overflowed".to_owned())?
            .copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), AnchoredSearchLineageV4Refusal> {
        self.write(&[value])
    }
    fn u32(&mut self, value: u32) -> Result<(), AnchoredSearchLineageV4Refusal> {
        self.write(&value.to_le_bytes())
    }
    fn u64(&mut self, value: u64) -> Result<(), AnchoredSearchLineageV4Refusal> {
        self.write(&value.to_le_bytes())
    }
    fn i64(&mut self, value: i64) -> Result<(), AnchoredSearchLineageV4Refusal> {
        self.write(&value.to_le_bytes())
    }
    fn zeros(&mut self, count: usize) -> Result<(), AnchoredSearchLineageV4Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "search-lineage V4 zero reserve overflowed".to_owned())?;
        self.bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "search-lineage V4 zero reserve exceeds record".to_owned())?
            .fill(0);
        self.cursor = end;
        Ok(())
    }
    fn zeros_to_end(&mut self) -> Result<(), AnchoredSearchLineageV4Refusal> {
        self.zeros(self.bytes.len().saturating_sub(self.cursor))
    }
}

struct FixedReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> FixedReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }
    fn take(&mut self, count: usize) -> Result<&'a [u8], AnchoredSearchLineageV4Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "search-lineage V4 reader cursor overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "search-lineage V4 fixed record ended early".to_owned())?;
        self.cursor = end;
        Ok(value)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], AnchoredSearchLineageV4Refusal> {
        self.take(N)?
            .try_into()
            .map_err(|_| "search-lineage V4 fixed array width mismatch".to_owned())
    }
    fn u8(&mut self) -> Result<u8, AnchoredSearchLineageV4Refusal> {
        self.take(1)?
            .first()
            .copied()
            .ok_or_else(|| "search-lineage V4 byte is absent".to_owned())
    }
    fn u32(&mut self) -> Result<u32, AnchoredSearchLineageV4Refusal> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, AnchoredSearchLineageV4Refusal> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn i64(&mut self) -> Result<i64, AnchoredSearchLineageV4Refusal> {
        Ok(i64::from_le_bytes(self.array()?))
    }
    fn require(
        &mut self,
        expected: &[u8],
        name: &str,
    ) -> Result<(), AnchoredSearchLineageV4Refusal> {
        if self.take(expected.len())? != expected {
            return Err(format!("search-lineage {name} mismatch"));
        }
        Ok(())
    }
    fn require_u32(
        &mut self,
        expected: u32,
        name: &str,
    ) -> Result<(), AnchoredSearchLineageV4Refusal> {
        let observed = self.u32()?;
        if observed != expected {
            return Err(format!(
                "search-lineage {name} is {observed}, expected {expected}"
            ));
        }
        Ok(())
    }
    fn require_zero(
        &mut self,
        count: usize,
        name: &str,
    ) -> Result<(), AnchoredSearchLineageV4Refusal> {
        if self.take(count)?.iter().any(|value| *value != 0) {
            return Err(format!("search-lineage {name} is nonzero"));
        }
        Ok(())
    }
    fn require_zero_to_end(&mut self, name: &str) -> Result<(), AnchoredSearchLineageV4Refusal> {
        self.require_zero(self.bytes.len().saturating_sub(self.cursor), name)
    }
}

fn read_member_pair(
    file: &mut File,
    pair_sequence: u64,
) -> Result<[MemberRecordV4; 2], AnchoredSearchLineageV4Refusal> {
    let first = pair_sequence
        .checked_mul(2)
        .ok_or_else(|| "search-lineage V4 member offset overflowed".to_owned())?;
    Ok([
        decode_member(&read_fixed_at::<ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES>(
            file, first,
        )?)?,
        decode_member(&read_fixed_at::<ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES>(
            file,
            first
                .checked_add(1)
                .ok_or_else(|| "search-lineage V4 second-member offset overflowed".to_owned())?,
        )?)?,
    ])
}

fn read_completion_at(
    file: &mut File,
    sequence: u64,
) -> Result<[u8; ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES], AnchoredSearchLineageV4Refusal> {
    read_fixed_at(file, sequence)
}

fn read_fixed_at<const N: usize>(
    file: &mut File,
    record: u64,
) -> Result<[u8; N], AnchoredSearchLineageV4Refusal> {
    let width = u64::try_from(N)
        .map_err(|_| "search-lineage V4 record width does not fit u64".to_owned())?;
    let offset = record
        .checked_mul(width)
        .ok_or_else(|| "search-lineage V4 record offset overflowed".to_owned())?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|why| format!("cannot seek search-lineage V4 record {record}: {why}"))?;
    let mut raw = [0_u8; N];
    file.read_exact(&mut raw)
        .map_err(|why| format!("cannot read search-lineage V4 record {record}: {why}"))?;
    Ok(raw)
}

fn record_count(
    len: u64,
    width: usize,
    maximum: u64,
    name: &str,
) -> Result<u64, AnchoredSearchLineageV4Refusal> {
    let width = u64::try_from(width)
        .map_err(|_| format!("search-lineage V4 {name} width does not fit u64"))?;
    if !len.is_multiple_of(width) {
        return Err(format!(
            "search-lineage V4 {name} file is ragged: {len} bytes is not divisible by {width}"
        ));
    }
    let count = len / width;
    if count > maximum {
        return Err(format!(
            "search-lineage V4 {name} file has {count} records above explicit maximum {maximum}"
        ));
    }
    Ok(count)
}

fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), AnchoredSearchLineageV4Refusal> {
    file.seek(SeekFrom::End(0))
        .map_err(|why| format!("cannot seek search-lineage V4 append: {why}"))?;
    file.write_all(raw)
        .map_err(|why| format!("cannot append search-lineage V4 bytes: {why}"))
}

fn open_root(
    root: &Path,
) -> Result<(PathBuf, File, PlatformIdentity), AnchoredSearchLineageV4Refusal> {
    let metadata = std::fs::symlink_metadata(root).map_err(|why| {
        format!(
            "cannot inspect search-lineage V4 root {}: {why}",
            root.display()
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "search-lineage V4 root {} must be a real directory, not a symlink",
            root.display()
        ));
    }
    let canonical = std::fs::canonicalize(root)
        .map_err(|why| format!("cannot canonicalize search-lineage V4 root: {why}"))?;
    let file = File::open(&canonical)
        .map_err(|why| format!("cannot open search-lineage V4 root directory: {why}"))?;
    let held = file
        .metadata()
        .map_err(|why| format!("cannot stat held search-lineage V4 root: {why}"))?;
    if !held.is_dir() || PlatformIdentity::of(&held) != PlatformIdentity::of(&metadata) {
        return Err("search-lineage V4 root changed during open".to_owned());
    }
    Ok((canonical, file, PlatformIdentity::of(&held)))
}

fn named_identity(path: &Path) -> Result<PlatformIdentity, AnchoredSearchLineageV4Refusal> {
    let metadata = std::fs::symlink_metadata(path).map_err(|why| {
        format!(
            "cannot restat search-lineage V4 path {}: {why}",
            path.display()
        )
    })?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "search-lineage V4 path {} became a symlink",
            path.display()
        ));
    }
    Ok(PlatformIdentity::of(&metadata))
}

fn open_child(path: &Path, writable: bool) -> Result<(File, bool), AnchoredSearchLineageV4Refusal> {
    let existed = match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(format!(
                    "search-lineage V4 child {} must be a regular non-symlink file",
                    path.display()
                ));
            }
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            return Err(format!(
                "cannot inspect search-lineage V4 child {}: {error}",
                path.display()
            ));
        }
    };
    if !existed && !writable {
        return Err(format!(
            "search-lineage V4 child {} is absent",
            path.display()
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true).write(writable).create(writable);
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    {
        options.mode(0o600).custom_flags(O_NOFOLLOW_FLAG);
    }
    let file = options.open(path).map_err(|why| {
        format!(
            "cannot open search-lineage V4 child {}: {why}",
            path.display()
        )
    })?;
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat held search-lineage V4 child: {why}"))?;
    if !metadata.is_file() || named_identity(path)? != PlatformIdentity::of(&metadata) {
        return Err(format!(
            "search-lineage V4 child {} changed during open",
            path.display()
        ));
    }
    Ok((file, !existed))
}

fn file_generation(
    file: &File,
    path: &Path,
    maximum: u64,
) -> Result<FileGeneration, AnchoredSearchLineageV4Refusal> {
    file_generation_with_between_hash_action(file, path, maximum, || Ok(()))
}

fn file_generation_with_between_hash_action(
    file: &File,
    path: &Path,
    maximum: u64,
    between_hashes: impl FnOnce() -> Result<(), AnchoredSearchLineageV4Refusal>,
) -> Result<FileGeneration, AnchoredSearchLineageV4Refusal> {
    let before_metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat held search-lineage V4 file: {why}"))?;
    if !before_metadata.is_file() || before_metadata.len() > maximum {
        return Err(format!(
            "search-lineage V4 file {} length {} exceeds explicit maximum {maximum}",
            path.display(),
            before_metadata.len()
        ));
    }
    let before = MetadataGeneration::of(&before_metadata);
    let identity = before.identity;
    if named_identity(path)? != identity {
        return Err(format!(
            "search-lineage V4 file {} was replaced",
            path.display()
        ));
    }
    let digest = hash_held_prefix(file, path, before.len)?;
    between_hashes()?;
    let middle = file.metadata().map_err(|why| {
        format!("cannot restat held search-lineage V4 file after first hash: {why}")
    })?;
    if MetadataGeneration::of(&middle) != before || named_identity(path)? != identity {
        return Err(format!(
            "search-lineage V4 file {} changed during bounded generation hashing",
            path.display()
        ));
    }
    let confirmation = hash_held_prefix(file, path, before.len)?;
    let after = file.metadata().map_err(|why| {
        format!("cannot restat held search-lineage V4 file after second hash: {why}")
    })?;
    if MetadataGeneration::of(&after) != before || named_identity(path)? != identity {
        return Err(format!(
            "search-lineage V4 file {} changed during bounded generation hashing",
            path.display()
        ));
    }
    if confirmation != digest {
        return Err(format!(
            "search-lineage V4 file {} changed between bounded generation hashes",
            path.display()
        ));
    }
    Ok(FileGeneration {
        identity,
        len: before.len,
        digest,
    })
}

fn hash_held_prefix(
    file: &File,
    path: &Path,
    length: u64,
) -> Result<[u8; 32], AnchoredSearchLineageV4Refusal> {
    let mut reader = file
        .try_clone()
        .map_err(|why| format!("cannot clone search-lineage V4 file for hashing: {why}"))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot rewind search-lineage V4 file: {why}"))?;
    hash_exact_prefix(&mut reader, path, length)
}

fn hash_exact_prefix(
    reader: &mut impl std::io::Read,
    path: &Path,
    length: u64,
) -> Result<[u8; 32], AnchoredSearchLineageV4Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(FILE_GENERATION_DOMAIN);
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    let mut remaining = length;
    while remaining != 0 {
        let requested = usize::try_from(remaining.min(READ_CHUNK_BYTES as u64))
            .map_err(|_| "search-lineage V4 bounded hash width does not fit usize".to_owned())?;
        let read = reader
            .read(
                buffer
                    .get_mut(..requested)
                    .ok_or_else(|| "search-lineage V4 bounded hash range is invalid".to_owned())?,
            )
            .map_err(|why| format!("cannot hash search-lineage V4 file: {why}"))?;
        if read == 0 {
            return Err(format!(
                "search-lineage V4 file {} shortened while hashing",
                path.display()
            ));
        }
        hasher.update(
            buffer
                .get(..read)
                .ok_or_else(|| "search-lineage V4 hash chunk range is invalid".to_owned())?,
        );
        remaining = remaining
            .checked_sub(
                u64::try_from(read)
                    .map_err(|_| "search-lineage V4 hash count does not fit u64".to_owned())?,
            )
            .ok_or_else(|| "search-lineage V4 bounded hash count underflowed".to_owned())?;
    }
    Ok(hasher.finalize())
}

fn sync_directory(file: &File, root: &Path) -> Result<(), AnchoredSearchLineageV4Refusal> {
    file.sync_all().map_err(|why| {
        format!(
            "cannot sync search-lineage V4 root {}: {why}",
            root.display()
        )
    })
}

fn hash_slices(domain: &[u8], slices: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    for slice in slices {
        hasher.update(slice);
    }
    hasher.finalize()
}

fn hex32(bytes: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(char::from(
            HEX.get(usize::from(byte >> 4)).copied().unwrap_or(b'?'),
        ));
        output.push(char::from(
            HEX.get(usize::from(byte & 0x0f)).copied().unwrap_or(b'?'),
        ));
    }
    output
}

fn combine_with_unlock<T>(
    result: Result<T, AnchoredSearchLineageV4Refusal>,
    released: Result<(), AnchoredSearchLineageV4Refusal>,
) -> Result<T, AnchoredSearchLineageV4Refusal> {
    match (result, released) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (Ok(_), Err(why)) => Err(why),
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "focused adversarial tests stop on fixture failure and mutate exact fixed bytes"
)]
mod tests {
    use super::*;
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use engine::Ladder;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use runner::Sweeper;
    use runner::exit_grid_policy::{
        ExecutionResolutionV1, ExecutionSeriesV1, ExitGridPolicyV1, ExitGridSelectorV1,
        ForcedStopV1, RangeResolutionV1, RatioLimitsV1, RationalPercentileV1, RungPlanV1,
        printed_ohlcv_cost_model_id_v1,
    };
    use runner::outcome::Horizon;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(label: &str) -> Self {
            let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "brutex-search-lineage-v4-{}-{label}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create isolated V4 search-lineage root");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            if self.0.exists() {
                std::fs::remove_dir_all(&self.0).expect("remove isolated V4 search-lineage root");
            }
        }
    }

    fn bounds() -> AnchoredSearchLineageV4Bounds {
        AnchoredSearchLineageV4Bounds::new(
            8,
            16 * ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES as u64,
            8 * ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES as u64,
        )
        .expect("fixture V4 bounds are valid")
    }

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    fn exact_grid_policy(side: runner::excursion::Side) -> ExitGridPolicyV1 {
        let middle = RationalPercentileV1::new(1, 2).expect("one-half percentile is valid");
        let rungs = RungPlanV1::new(vec![middle], vec![middle], vec![middle], 1)
            .expect("one explicit rung per axis is bounded");
        let ratios =
            RatioLimitsV1::new(1, 100_000, 1).expect("broad single-pair ratio range is valid");
        ExitGridPolicyV1::new(
            ExecutionResolutionV1::OneMinuteOhlcv,
            RangeResolutionV1::PpmCeiling,
            side,
            rungs,
            ratios,
            32,
            ExitGridSelectorV1::PessimisticTotal,
            printed_ohlcv_cost_model_id_v1(),
            ForcedStopV1::Disabled,
            u64::MAX,
            u64::MAX,
        )
        .expect("complete explicit V4 fixture policy is valid")
    }

    fn projection_with_min_hits(
        symbol: &'static str,
        min_hits: u64,
    ) -> AnchoredSearchAuthorityProjectionV4 {
        let bars = runner::synthetic::sessions(16);
        let instrument =
            InstrumentKey::index(Exchange::Nse, symbol).expect("fixture index is in scope");
        let execution = ExecutionSeriesV1::new(
            &instrument,
            "lineage-v4-test-feed",
            "lineage-v4-test-commit",
            [0xa5; 32],
            &bars,
        )
        .expect("fixture carries every execution identity");
        let long = exact_grid_policy(runner::excursion::Side::Long)
            .resolve_attested(execution)
            .expect("Long grid resolves over the exact full span");
        let short = exact_grid_policy(runner::excursion::Side::Short)
            .resolve_attested(execution)
            .expect("Short grid resolves over the exact full span");
        let full_column = Column::build(&bars, &mut evaluator());
        let sweeper = Sweeper::new(Ladder::with_min_hits(min_hits).with_ceiling(20_000));
        let mut builder = |slice: &[indicators::Candle]| {
            let mut evaluator = evaluator();
            Ok(Column::build(slice, &mut evaluator))
        };
        runner::validate::walk_forward_projected_prepared_anchored_search_v4(
            &bars,
            &full_column,
            execution,
            60_000_000,
            Horizon::bars(15).expect("nonzero horizon"),
            1,
            &sweeper,
            &mut builder,
            &long,
            &short,
        )
        .expect("opaque exact-grid V4 fixture runs")
        .search_authority_projection()
        .expect("opaque exact-grid V4 fixture revalidates")
    }

    fn projection(symbol: &'static str) -> AnchoredSearchAuthorityProjectionV4 {
        projection_with_min_hits(symbol, 2_200)
    }

    fn projections() -> (
        AnchoredSearchAuthorityProjectionV4,
        AnchoredSearchAuthorityProjectionV4,
    ) {
        static VALUES: OnceLock<(
            AnchoredSearchAuthorityProjectionV4,
            AnchoredSearchAuthorityProjectionV4,
        )> = OnceLock::new();
        *VALUES.get_or_init(|| (projection("NIFTY"), projection("BANKNIFTY")))
    }

    fn zero_population_projections() -> (
        AnchoredSearchAuthorityProjectionV4,
        AnchoredSearchAuthorityProjectionV4,
    ) {
        static VALUES: OnceLock<(
            AnchoredSearchAuthorityProjectionV4,
            AnchoredSearchAuthorityProjectionV4,
        )> = OnceLock::new();
        *VALUES.get_or_init(|| {
            (
                projection_with_min_hits("NIFTY", u64::MAX),
                projection_with_min_hits("BANKNIFTY", u64::MAX),
            )
        })
    }

    fn initialize(root: &Path) {
        drop(
            AnchoredSearchLineageV4Ledger::open_write(root, bounds())
                .expect("initialize empty V4 search-lineage ledger"),
        );
    }

    fn lengths(root: &Path) -> (u64, u64) {
        (
            std::fs::metadata(root.join(MEMBER_FILE))
                .expect("V4 member metadata")
                .len(),
            std::fs::metadata(root.join(COMPLETION_FILE))
                .expect("V4 Completion metadata")
                .len(),
        )
    }

    fn overwrite_byte(path: &Path, offset: u64) {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("open V4 attack file");
        file.seek(SeekFrom::Start(offset))
            .expect("seek attack byte");
        let mut byte = [0_u8; 1];
        file.read_exact(&mut byte).expect("read attack byte");
        byte[0] ^= 1;
        file.seek(SeekFrom::Start(offset))
            .expect("rewind attack byte");
        file.write_all(&byte).expect("write attack byte");
        file.sync_all().expect("sync attack byte");
    }

    fn assert_refuses<T>(result: Result<T, AnchoredSearchLineageV4Refusal>, needle: &str) {
        let error = result.err().expect("operation must refuse");
        assert!(
            error.contains(needle),
            "expected refusal containing {needle:?}, got {error:?}"
        );
    }

    #[test]
    fn v4_bounds_and_names_are_disjoint_from_v3() {
        assert_refuses(AnchoredSearchLineageV4Bounds::new(0, 1, 1), "nonzero");
        assert_refuses(
            AnchoredSearchLineageV4Bounds::new(
                2,
                3 * ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES as u64,
                2 * ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES as u64,
            ),
            "cannot hold",
        );
        assert_refuses(
            AnchoredSearchLineageV4Bounds::new(
                2,
                4 * ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES as u64,
                ANCHORED_SEARCH_LINEAGE_V4_COMPLETION_BYTES as u64,
            ),
            "cannot hold",
        );
        assert!(!MEMBER_FILE.contains("v3"));
        assert!(!COMPLETION_FILE.contains("v3"));
        assert!(!LOCK_FILE.contains("v3"));
    }

    #[test]
    fn fixed_codec_retains_every_v4_projection_component() {
        let (nifty, banknifty) = projections();
        let prepared = PreparedPairV4::from_opaque(&nifty, &banknifty).expect("prepare V4 pair");
        let members = prepared.members(7);
        let raw = encode_member_records(&members).expect("encode V4 records");
        assert_eq!(raw[0].len(), ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES);
        assert_eq!(raw[1].len(), ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES);
        assert_eq!(decode_member(&raw[0]).expect("decode NIFTY"), members[0]);
        assert_eq!(
            decode_member(&raw[1]).expect("decode BANKNIFTY"),
            members[1]
        );
        assert_eq!(members[0].family, SearchFamilyV4::Nifty);
        assert_eq!(members[1].family, SearchFamilyV4::BankNifty);
        assert_ne!(
            members[0].source_authority_id,
            members[1].source_authority_id
        );
        assert_refuses(PreparedPairV4::from_opaque(&nifty, &nifty), "alias");

        let source = nifty.source_identity();
        let long = nifty.long_grid_identity();
        let short = nifty.short_grid_identity();
        let member = members[0];
        assert_eq!(
            member.validation_policy_id,
            nifty.policy_identity().digest()
        );
        assert_eq!(member.signal_digest, source.signal_digest());
        assert_eq!(member.signal_bars, source.signal_bars());
        assert_eq!(
            member.signal_first_ts_micros,
            source.signal_first_ts_micros()
        );
        assert_eq!(member.signal_last_ts_micros, source.signal_last_ts_micros());
        assert_eq!(member.signal_column_digest, source.signal_column_digest());
        assert_eq!(member.aggregate_grid_id, nifty.grid_identity().digest());
        assert_eq!(member.long_policy_id, long.policy_digest());
        assert_eq!(member.long_resolution_id, long.resolution_digest());
        assert_eq!(member.short_policy_id, short.policy_digest());
        assert_eq!(member.short_resolution_id, short.resolution_digest());
        assert_eq!(
            member.validation_family_id,
            nifty.family_identity().digest()
        );
        assert_eq!(member.walk_facts_id, nifty.walk_identity().digest());
        assert_eq!(member.fold_count, nifty.fold_count());
        assert_eq!(member.decided_folds, nifty.decided_folds());
        assert_eq!(member.profitable_oos_folds, nifty.profitable_oos_folds());
        assert_eq!(member.aggregate_oos_paisa, nifty.aggregate_oos_paisa());
        assert_eq!(
            member.evaluated_population_cells,
            nifty.evaluated_population_cells()
        );

        assert_eq!(&raw[0][..16], &MEMBER_MAGIC);
        assert_eq!(
            u32::from_le_bytes(raw[0][16..20].try_into().expect("version")),
            VERSION
        );
        let completion = prepared.completion(7).expect("derive V4 Completion");
        let completion_raw = encode_completion(&completion).expect("encode V4 Completion");
        assert_eq!(&completion_raw[..16], &COMPLETION_MAGIC);
        assert_eq!(
            decode_completion(&completion_raw).expect("decode Completion"),
            completion
        );

        let mut noncanonical_member = raw[0];
        noncanonical_member[488] = 1;
        let seal = hash_slices(
            MEMBER_SEAL_DOMAIN,
            &[&noncanonical_member[..MEMBER_PAYLOAD_BYTES]],
        );
        noncanonical_member[MEMBER_PAYLOAD_BYTES..].copy_from_slice(&seal);
        assert_refuses(decode_member(&noncanonical_member), "member reserve");

        let mut noncanonical_completion = completion_raw;
        noncanonical_completion[200] = 1;
        let seal = hash_slices(
            COMPLETION_SEAL_DOMAIN,
            &[&noncanonical_completion[..COMPLETION_PAYLOAD_BYTES]],
        );
        noncanonical_completion[COMPLETION_PAYLOAD_BYTES..].copy_from_slice(&seal);
        assert_refuses(
            decode_completion(&noncanonical_completion),
            "Completion reserve",
        );
    }

    #[test]
    fn valid_zero_evaluated_population_roundtrips_and_freshly_reopens() {
        let root = TestRoot::new("zero-evaluated-population");
        let (nifty, banknifty) = zero_population_projections();
        assert!(nifty.fold_count() > 0);
        assert!(banknifty.fold_count() > 0);
        assert_eq!(nifty.evaluated_population_cells(), 0);
        assert_eq!(banknifty.evaluated_population_cells(), 0);

        let prepared =
            PreparedPairV4::from_opaque(&nifty, &banknifty).expect("prepare zero-population pair");
        let members = prepared.members(0);
        for member in members {
            let encoded = encode_member(&member).expect("encode zero-population member");
            assert_eq!(
                decode_member(&encoded).expect("decode zero-population member"),
                member
            );
        }
        let completion = prepared
            .completion(0)
            .expect("derive zero-population Completion");
        assert_eq!(completion.total_evaluated_population_cells, 0);
        let encoded = encode_completion(&completion).expect("encode zero-population Completion");
        assert_eq!(
            decode_completion(&encoded).expect("decode zero-population Completion"),
            completion
        );

        let commit = persist_anchored_search_lineage_v4(root.path(), bounds(), &nifty, &banknifty)
            .expect("persist and freshly reopen zero-population V4 pair");
        let authority = match commit {
            AnchoredSearchLineageV4AuthenticatedCommit::Written(value) => value,
            AnchoredSearchLineageV4AuthenticatedCommit::Reused(_) => {
                panic!("first zero-population append must write")
            }
        };
        assert_eq!(
            authority
                .structural_receipt()
                .total_evaluated_population_cells(),
            0
        );
        assert_eq!(authority.nifty().evaluated_population_cells(), 0);
        assert_eq!(authority.banknifty().evaluated_population_cells(), 0);
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one durability test checks the complete receipt, member lineage, exact reuse, and byte identity"
    )]
    fn fresh_reopen_authenticates_and_exact_retry_is_idempotent() {
        let root = TestRoot::new("persist-reuse");
        let (nifty, banknifty) = projections();
        let first = persist_anchored_search_lineage_v4(root.path(), bounds(), &nifty, &banknifty)
            .expect("write and freshly authenticate V4 pair");
        let authority = match first {
            AnchoredSearchLineageV4AuthenticatedCommit::Written(value) => value,
            AnchoredSearchLineageV4AuthenticatedCommit::Reused(_) => {
                panic!("first V4 append must write")
            }
        };
        let nifty_authority = authority.nifty();
        let banknifty_authority = authority.banknifty();
        let source = nifty.source_identity();
        assert_eq!(nifty_authority.signal_digest(), source.signal_digest());
        assert_eq!(nifty_authority.signal_bars(), source.signal_bars());
        assert_eq!(
            nifty_authority.signal_first_ts_micros(),
            source.signal_first_ts_micros()
        );
        assert_eq!(
            nifty_authority.signal_last_ts_micros(),
            source.signal_last_ts_micros()
        );
        assert_eq!(
            nifty_authority.signal_column_digest(),
            source.signal_column_digest()
        );
        assert_ne!(
            nifty_authority.source_authority_id(),
            banknifty_authority.source_authority_id()
        );
        assert_ne!(nifty_authority.member_id(), banknifty_authority.member_id());
        assert_eq!(
            nifty_authority.validation_policy_id(),
            nifty.policy_identity().digest()
        );
        assert_eq!(
            nifty_authority.aggregate_grid_id(),
            nifty.grid_identity().digest()
        );
        assert_eq!(
            nifty_authority.long_policy_id(),
            nifty.long_grid_identity().policy_digest()
        );
        assert_eq!(
            nifty_authority.long_resolution_id(),
            nifty.long_grid_identity().resolution_digest()
        );
        assert_eq!(
            nifty_authority.short_policy_id(),
            nifty.short_grid_identity().policy_digest()
        );
        assert_eq!(
            nifty_authority.short_resolution_id(),
            nifty.short_grid_identity().resolution_digest()
        );
        assert_eq!(
            nifty_authority.validation_family_id(),
            nifty.family_identity().digest()
        );
        assert_eq!(
            nifty_authority.walk_facts_id(),
            nifty.walk_identity().digest()
        );
        assert_eq!(nifty_authority.fold_count(), nifty.fold_count());
        assert_eq!(nifty_authority.decided_folds(), nifty.decided_folds());
        assert_eq!(
            nifty_authority.profitable_oos_folds(),
            nifty.profitable_oos_folds()
        );
        assert_eq!(
            nifty_authority.aggregate_oos_paisa(),
            nifty.aggregate_oos_paisa()
        );
        assert_eq!(
            nifty_authority.evaluated_population_cells(),
            nifty.evaluated_population_cells()
        );

        let receipt = authority.structural_receipt();
        assert_eq!(receipt.block_sequence(), 0);
        assert_eq!(receipt.nifty_member_id(), nifty_authority.member_id());
        assert_eq!(
            receipt.banknifty_member_id(),
            banknifty_authority.member_id()
        );
        assert_eq!(
            receipt.total_folds(),
            nifty.fold_count() + banknifty.fold_count()
        );
        assert_eq!(
            receipt.total_evaluated_population_cells(),
            nifty.evaluated_population_cells() + banknifty.evaluated_population_cells()
        );
        assert!(receipt.total_decided() <= receipt.total_folds());
        assert!(receipt.total_profitable() <= receipt.total_decided());
        let _signed_total = receipt.aggregate_oos_paisa();

        let before = lengths(root.path());
        let retry = persist_anchored_search_lineage_v4(root.path(), bounds(), &nifty, &banknifty)
            .expect("exact V4 retry reuses");
        let retry_authority = match retry {
            AnchoredSearchLineageV4AuthenticatedCommit::Reused(value) => value,
            AnchoredSearchLineageV4AuthenticatedCommit::Written(_) => {
                panic!("exact V4 retry must not append")
            }
        };
        assert_eq!(retry_authority.structural_receipt(), receipt);
        assert_eq!(lengths(root.path()), before);
        let reader = AnchoredSearchLineageV4Ledger::open_read(root.path(), bounds())
            .expect("fresh public V4 structural reopen");
        assert_eq!(
            reader
                .structural_receipt(&receipt.pair_id())
                .expect("stable V4 receipt lookup"),
            Some(receipt)
        );
    }

    #[test]
    fn full_synced_tail_is_retryable_but_partial_or_foreign_tail_refuses() {
        let (nifty, banknifty) = projections();
        let prepared = PreparedPairV4::from_opaque(&nifty, &banknifty).expect("prepare V4 pair");

        let full = TestRoot::new("full-tail");
        initialize(full.path());
        let mut member = OpenOptions::new()
            .read(true)
            .write(true)
            .open(full.path().join(MEMBER_FILE))
            .expect("open V4 member tail");
        for raw in encode_member_records(&prepared.members(0)).expect("encode complete V4 tail") {
            append_raw(&mut member, &raw).expect("append V4 tail member");
        }
        member.sync_all().expect("sync complete V4 tail");
        drop(member);
        let retry = persist_anchored_search_lineage_v4(full.path(), bounds(), &nifty, &banknifty)
            .expect("complete exact V4 tail");
        assert!(matches!(
            retry,
            AnchoredSearchLineageV4AuthenticatedCommit::Written(_)
        ));
        assert_eq!(
            retry.authority().structural_receipt().pair_id(),
            prepared.pair_id
        );

        let partial = TestRoot::new("partial-tail");
        initialize(partial.path());
        let mut member = OpenOptions::new()
            .read(true)
            .write(true)
            .open(partial.path().join(MEMBER_FILE))
            .expect("open partial V4 tail");
        append_raw(
            &mut member,
            &encode_member(&prepared.members(0)[0]).expect("encode first V4 member"),
        )
        .expect("append one V4 member");
        member.sync_all().expect("sync partial V4 tail");
        drop(member);
        assert_refuses(
            AnchoredSearchLineageV4Ledger::open_read(partial.path(), bounds()),
            "partial trailing pair",
        );

        let foreign = TestRoot::new("foreign-tail");
        initialize(foreign.path());
        let mut member = OpenOptions::new()
            .read(true)
            .write(true)
            .open(foreign.path().join(MEMBER_FILE))
            .expect("open foreign V4 tail");
        for raw in encode_member_records(&prepared.members(0)).expect("encode foreign V4 tail") {
            append_raw(&mut member, &raw).expect("append foreign V4 tail member");
        }
        member.sync_all().expect("sync foreign V4 tail");
        drop(member);
        let mut other_bank = prepared.banknifty;
        other_bank.walk_facts_id[0] ^= 1;
        other_bank.source_authority_id = other_bank.derive_source_authority_id();
        other_bank.member_id = other_bank.derive_member_id();
        let other = PreparedPairV4 {
            nifty: prepared.nifty,
            banknifty: other_bank,
            pair_id: derive_pair_id(prepared.nifty.member_id, other_bank.member_id),
        };
        let mut writer = AnchoredSearchLineageV4Ledger::open_write(foreign.path(), bounds())
            .expect("foreign complete tail is structurally valid");
        assert_refuses(writer.append(&other), "not exact retry");
    }

    #[test]
    fn member_completion_ragged_and_canonical_order_attacks_fail_closed() {
        let (nifty, banknifty) = projections();

        let member_root = TestRoot::new("member-corrupt");
        persist_anchored_search_lineage_v4(member_root.path(), bounds(), &nifty, &banknifty)
            .expect("persist V4 member attack fixture");
        overwrite_byte(&member_root.path().join(MEMBER_FILE), 80);
        assert_refuses(
            AnchoredSearchLineageV4Ledger::open_read(member_root.path(), bounds()),
            "member seal mismatch",
        );

        let completion_root = TestRoot::new("completion-corrupt");
        persist_anchored_search_lineage_v4(completion_root.path(), bounds(), &nifty, &banknifty)
            .expect("persist V4 Completion attack fixture");
        overwrite_byte(&completion_root.path().join(COMPLETION_FILE), 80);
        assert_refuses(
            AnchoredSearchLineageV4Ledger::open_read(completion_root.path(), bounds()),
            "Completion seal mismatch",
        );

        for (label, file_name) in [
            ("ragged-member", MEMBER_FILE),
            ("ragged-completion", COMPLETION_FILE),
        ] {
            let root = TestRoot::new(label);
            initialize(root.path());
            let mut file = OpenOptions::new()
                .append(true)
                .open(root.path().join(file_name))
                .expect("open V4 file for ragged append");
            file.write_all(&[0x5a]).expect("append ragged V4 byte");
            file.sync_all().expect("sync ragged V4 byte");
            drop(file);
            assert_refuses(
                AnchoredSearchLineageV4Ledger::open_read(root.path(), bounds()),
                "ragged",
            );
        }

        let prepared = PreparedPairV4::from_opaque(&nifty, &banknifty).expect("prepare V4 pair");
        let swapped = TestRoot::new("swapped");
        initialize(swapped.path());
        let members = prepared.members(0);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(swapped.path().join(MEMBER_FILE))
            .expect("open V4 members");
        append_raw(
            &mut file,
            &encode_member(&members[1]).expect("encode BANKNIFTY"),
        )
        .expect("append BANKNIFTY first");
        append_raw(
            &mut file,
            &encode_member(&members[0]).expect("encode NIFTY"),
        )
        .expect("append NIFTY second");
        file.sync_all().expect("sync swapped V4 pair");
        drop(file);
        assert_refuses(
            AnchoredSearchLineageV4Ledger::open_read(swapped.path(), bounds()),
            "canonical NIFTY then BANKNIFTY",
        );
    }

    #[test]
    fn stale_same_length_growth_replacement_and_post_lookup_mutation_refuse() {
        let (nifty, banknifty) = projections();

        let same = TestRoot::new("same-length-stale");
        let receipt = persist_anchored_search_lineage_v4(same.path(), bounds(), &nifty, &banknifty)
            .expect("persist same-length V4 fixture")
            .authority()
            .structural_receipt();
        let reader = AnchoredSearchLineageV4Ledger::open_read(same.path(), bounds())
            .expect("open same-length V4 reader");
        overwrite_byte(&same.path().join(MEMBER_FILE), 80);
        assert_refuses(
            reader.structural_receipt(&receipt.pair_id()),
            "member generation changed",
        );

        let growth = TestRoot::new("growth-stale");
        let receipt =
            persist_anchored_search_lineage_v4(growth.path(), bounds(), &nifty, &banknifty)
                .expect("persist growth V4 fixture")
                .authority()
                .structural_receipt();
        let reader = AnchoredSearchLineageV4Ledger::open_read(growth.path(), bounds())
            .expect("open growth V4 reader");
        let mut file = OpenOptions::new()
            .append(true)
            .open(growth.path().join(MEMBER_FILE))
            .expect("open V4 file for external growth");
        file.write_all(&[0x7f]).expect("append foreign V4 byte");
        file.sync_all().expect("sync foreign V4 growth");
        drop(file);
        assert_refuses(
            reader.structural_receipt(&receipt.pair_id()),
            "member generation changed",
        );

        let replaced = TestRoot::new("path-replaced");
        let receipt =
            persist_anchored_search_lineage_v4(replaced.path(), bounds(), &nifty, &banknifty)
                .expect("persist replacement V4 fixture")
                .authority()
                .structural_receipt();
        let reader = AnchoredSearchLineageV4Ledger::open_read(replaced.path(), bounds())
            .expect("open replacement V4 reader");
        let member_path = replaced.path().join(MEMBER_FILE);
        let displaced_path = replaced.path().join("displaced-v4-members.bin");
        std::fs::rename(&member_path, &displaced_path).expect("displace named V4 member file");
        std::fs::copy(&displaced_path, &member_path).expect("replace V4 member with same bytes");
        File::open(&member_path)
            .expect("open V4 replacement")
            .sync_all()
            .expect("sync V4 replacement");
        assert_refuses(
            reader.structural_receipt(&receipt.pair_id()),
            "was replaced",
        );

        let post = TestRoot::new("post-lookup");
        let receipt = persist_anchored_search_lineage_v4(post.path(), bounds(), &nifty, &banknifty)
            .expect("persist post-lookup V4 fixture")
            .authority()
            .structural_receipt();
        let reader = AnchoredSearchLineageV4Ledger::open_read(post.path(), bounds())
            .expect("open post-lookup V4 reader");
        let path = post.path().join(MEMBER_FILE);
        assert_refuses(
            reader.structural_receipt_locked_with_after_lookup(&receipt.pair_id(), || {
                overwrite_byte(&path, 80);
                Ok(())
            }),
            "member generation changed",
        );
    }

    #[test]
    fn resealed_foreign_semantics_are_structural_only_not_authenticated() {
        let root = TestRoot::new("resealed-foreign");
        initialize(root.path());
        let (nifty, banknifty) = projections();
        let original =
            PreparedPairV4::from_opaque(&nifty, &banknifty).expect("prepare real V4 pair");
        let mut forged_nifty = original.nifty;
        forged_nifty.signal_column_digest[0] ^= 1;
        forged_nifty.source_authority_id = forged_nifty.derive_source_authority_id();
        forged_nifty.member_id = forged_nifty.derive_member_id();
        let forged = PreparedPairV4 {
            nifty: forged_nifty,
            banknifty: original.banknifty,
            pair_id: derive_pair_id(forged_nifty.member_id, original.banknifty.member_id),
        };
        let mut members = OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.path().join(MEMBER_FILE))
            .expect("open forged V4 member file");
        for raw in encode_member_records(&forged.members(0)).expect("encode forged V4 members") {
            append_raw(&mut members, &raw).expect("append forged V4 member");
        }
        members.sync_all().expect("sync forged V4 members");
        drop(members);
        let mut completions = OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.path().join(COMPLETION_FILE))
            .expect("open forged V4 Completion file");
        append_raw(
            &mut completions,
            &encode_completion(&forged.completion(0).expect("derive forged V4 Completion"))
                .expect("encode forged V4 Completion"),
        )
        .expect("append forged V4 Completion");
        completions.sync_all().expect("sync forged V4 Completion");
        drop(completions);

        let mut reopened = AnchoredSearchLineageV4Ledger::open_read(root.path(), bounds())
            .expect("structurally reopen forged V4 pair");
        let receipt = reopened
            .structural_receipt(&forged.pair_id)
            .expect("look up forged V4 receipt")
            .expect("forged V4 structural receipt is present");
        assert_refuses(
            reopened.authenticate(receipt, &original),
            "not the freshly reopened indexed pair",
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_root_and_child_are_refused_without_fallback() {
        use std::os::unix::fs::symlink;

        let target = TestRoot::new("symlink-target");
        initialize(target.path());
        let holder = TestRoot::new("symlink-holder");
        let root_link = holder.path().join("root-link");
        symlink(target.path(), &root_link).expect("create V4 root symlink");
        assert_refuses(
            AnchoredSearchLineageV4Ledger::open_read(&root_link, bounds()),
            "real directory",
        );

        let child = TestRoot::new("symlink-child");
        initialize(child.path());
        std::fs::remove_file(child.path().join(MEMBER_FILE)).expect("remove V4 member file");
        symlink(
            target.path().join(MEMBER_FILE),
            child.path().join(MEMBER_FILE),
        )
        .expect("create V4 member symlink");
        assert_refuses(
            AnchoredSearchLineageV4Ledger::open_read(child.path(), bounds()),
            "regular non-symlink",
        );
    }
}
