//! Durable paired authority for Runner's opaque anchored-search projection.
//!
//! One logical block contains exactly two fixed records in canonical order:
//! NIFTY first, BANKNIFTY second.  Their data bytes are appended and synced
//! before one fixed Completion record is appended and synced.  Public reopen
//! proves only structural consistency.  The crate-private authority is minted
//! only after a fresh reopen reproduces the exact pair prepared directly from
//! two [`runner::validate::AnchoredSearchAuthorityProjectionV2`] values.
//!
//! This module deliberately does not bind a feed, stored-bar receipt,
//! Candidate Universe, Statistics, Admission decision, Selection result, or
//! Finalization identity.  In particular, the positional NIFTY/BANKNIFTY
//! labels are not proof of instrument provenance; that remains a later typed
//! join.
//!
//! # Cost
//!
//! Open, append, retry, reuse, lookup validation and authentication each hash
//! or scan explicitly bounded files and are O(file bytes + pair records), not
//! O(1).  After that validation, the in-memory pair lookup is average O(1) and
//! uses O(completed pairs) space.  Record encode/decode and one pair comparison
//! are fixed-width O(1).
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
use std::os::unix::fs::OpenOptionsExt as _;

use brutex_core::blake3::Hasher;
use runner::validate::AnchoredSearchAuthorityProjectionV2;

/// Bytes in one fixed anchored-search member record.
pub(crate) const ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES: usize = 512;
/// Bytes in one receipt-last anchored-search Completion record.
pub(crate) const ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES: usize = 512;

/// Operator-facing refusal from the paired search-lineage boundary.
pub(crate) type AnchoredSearchLineageV2Refusal = String;

const VERSION: u32 = 2;
const MEMBER_MAGIC: [u8; 16] = *b"BTX-SRCHV2-MEM\0\0";
const COMPLETION_MAGIC: [u8; 16] = *b"BTX-SRCHV2-CMP\0\0";
const MEMBER_DOMAIN: u32 = 1;
const COMPLETION_DOMAIN: u32 = 2;
const SEAL_BYTES: usize = 32;
const MEMBER_PAYLOAD_BYTES: usize = ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES - SEAL_BYTES;
const COMPLETION_PAYLOAD_BYTES: usize = ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES - SEAL_BYTES;
const SOURCE_AUTHORITY_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v2-source\0";
const MEMBER_ID_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v2-member\0";
const PAIR_ID_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v2-pair\0";
const ORDERED_MEMBER_BYTES_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v2-ordered-members\0";
const MEMBER_SEAL_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v2-member-seal\0";
const COMPLETION_SEAL_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v2-completion-seal\0";
const FILE_GENERATION_DOMAIN: &[u8] = b"brutex-anchored-search-lineage-v2-file\0";
const MEMBER_FILE: &str = "anchored-search-lineage-members-v2.bin";
const COMPLETION_FILE: &str = "anchored-search-lineage-completions-v2.bin";
const LOCK_FILE: &str = "anchored-search-lineage-v2.lock";
const READ_CHUNK_BYTES: usize = 16 * 1_024;

#[cfg(any(target_os = "android", target_os = "linux"))]
const O_NOFOLLOW_FLAG: i32 = 0x20_000;
#[cfg(target_os = "macos")]
const O_NOFOLLOW_FLAG: i32 = 0x100;

const _: () = assert!(MEMBER_PAYLOAD_BYTES + SEAL_BYTES == ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES);
const _: () =
    assert!(COMPLETION_PAYLOAD_BYTES + SEAL_BYTES == ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES);

/// Explicit physical ceilings for the paired search-lineage ledger.
///
/// There is intentionally no `Default`; every caller states all three bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnchoredSearchLineageV2Bounds {
    pair_records: u64,
    member_bytes: u64,
    completion_bytes: u64,
}

impl AnchoredSearchLineageV2Bounds {
    /// Constructs nonzero ceilings that can hold the stated number of pairs.
    ///
    /// # Errors
    ///
    /// Refuses zero, multiplication overflow, or byte ceilings smaller than
    /// the exact fixed records implied by `max_pair_records`.
    pub(crate) fn new(
        max_pair_records: u64,
        max_member_bytes: u64,
        max_completion_bytes: u64,
    ) -> Result<Self, AnchoredSearchLineageV2Refusal> {
        for (name, value) in [
            ("pair records", max_pair_records),
            ("member bytes", max_member_bytes),
            ("Completion bytes", max_completion_bytes),
        ] {
            if value == 0 {
                return Err(format!(
                    "anchored search-lineage {name} bound must be nonzero"
                ));
            }
        }
        let member_records = max_pair_records
            .checked_mul(2)
            .ok_or_else(|| "anchored search-lineage member-record bound overflowed".to_owned())?;
        let required_member_bytes = member_records
            .checked_mul(ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES as u64)
            .ok_or_else(|| "anchored search-lineage member-byte bound overflowed".to_owned())?;
        if max_member_bytes < required_member_bytes {
            return Err(format!(
                "anchored search-lineage member-byte bound {max_member_bytes} cannot hold {max_pair_records} pairs ({required_member_bytes} bytes)"
            ));
        }
        let required_completion_bytes = max_pair_records
            .checked_mul(ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES as u64)
            .ok_or_else(|| "anchored search-lineage Completion-byte bound overflowed".to_owned())?;
        if max_completion_bytes < required_completion_bytes {
            return Err(format!(
                "anchored search-lineage Completion-byte bound {max_completion_bytes} cannot hold {max_pair_records} pairs ({required_completion_bytes} bytes)"
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
enum SearchFamilyV2 {
    Nifty = 1,
    BankNifty = 2,
}

impl SearchFamilyV2 {
    fn decode(value: u8) -> Result<Self, AnchoredSearchLineageV2Refusal> {
        match value {
            1 => Ok(Self::Nifty),
            2 => Ok(Self::BankNifty),
            _ => Err(format!(
                "anchored search-lineage family tag {value} is outside the fixed V2 domain"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MemberRecordV2 {
    block_sequence: u64,
    family: SearchFamilyV2,
    validation_policy_id: [u8; 32],
    validation_family_id: [u8; 32],
    walk_facts_id: [u8; 32],
    source_authority_id: [u8; 32],
    member_id: [u8; 32],
    fold_count: u64,
    decided_folds: u64,
    profitable_oos_folds: u64,
    aggregate_oos_paisa: i64,
}

impl MemberRecordV2 {
    fn from_projection(
        family: SearchFamilyV2,
        projection: &AnchoredSearchAuthorityProjectionV2,
    ) -> Result<Self, AnchoredSearchLineageV2Refusal> {
        let mut value = Self {
            block_sequence: 0,
            family,
            validation_policy_id: projection.policy_identity().digest(),
            validation_family_id: projection.family_identity().digest(),
            walk_facts_id: projection.walk_identity().digest(),
            source_authority_id: [0; 32],
            member_id: [0; 32],
            fold_count: projection.fold_count(),
            decided_folds: projection.decided_folds(),
            profitable_oos_folds: projection.profitable_oos_folds(),
            aggregate_oos_paisa: projection.aggregate_oos_paisa(),
        };
        value.source_authority_id = value.derive_source_authority_id();
        value.member_id = value.derive_member_id();
        value.validate()?;
        Ok(value)
    }

    fn with_sequence(mut self, sequence: u64) -> Self {
        self.block_sequence = sequence;
        self
    }

    fn validate(self) -> Result<(), AnchoredSearchLineageV2Refusal> {
        for (name, value) in [
            ("validation policy", self.validation_policy_id),
            ("validation family", self.validation_family_id),
            ("walk facts", self.walk_facts_id),
            ("source authority", self.source_authority_id),
            ("member", self.member_id),
        ] {
            if value == [0; 32] {
                return Err(format!("anchored search-lineage {name} identity is zero"));
            }
        }
        if self.fold_count == 0
            || self.decided_folds > self.fold_count
            || self.profitable_oos_folds > self.decided_folds
        {
            return Err("anchored search-lineage fold hierarchy is invalid".to_owned());
        }
        if (self.decided_folds == 0 && self.aggregate_oos_paisa != 0)
            || (self.profitable_oos_folds == 0 && self.aggregate_oos_paisa > 0)
            || (self.profitable_oos_folds == self.decided_folds
                && self.decided_folds > 0
                && self.aggregate_oos_paisa <= 0)
        {
            return Err("anchored search-lineage outcome hierarchy is invalid".to_owned());
        }
        if self.source_authority_id != self.derive_source_authority_id() {
            return Err("anchored search-lineage source-authority identity mismatch".to_owned());
        }
        if self.member_id != self.derive_member_id() {
            return Err("anchored search-lineage member identity mismatch".to_owned());
        }
        Ok(())
    }

    fn derive_source_authority_id(self) -> [u8; 32] {
        hash_slices(
            SOURCE_AUTHORITY_DOMAIN,
            &[
                &self.validation_policy_id,
                &self.validation_family_id,
                &self.walk_facts_id,
                &self.fold_count.to_le_bytes(),
                &self.decided_folds.to_le_bytes(),
                &self.profitable_oos_folds.to_le_bytes(),
                &self.aggregate_oos_paisa.to_le_bytes(),
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
struct PreparedPairV2 {
    nifty: MemberRecordV2,
    banknifty: MemberRecordV2,
    pair_id: [u8; 32],
}

impl PreparedPairV2 {
    fn from_opaque(
        nifty: &AnchoredSearchAuthorityProjectionV2,
        banknifty: &AnchoredSearchAuthorityProjectionV2,
    ) -> Result<Self, AnchoredSearchLineageV2Refusal> {
        let nifty = MemberRecordV2::from_projection(SearchFamilyV2::Nifty, nifty)?;
        let banknifty = MemberRecordV2::from_projection(SearchFamilyV2::BankNifty, banknifty)?;
        if nifty.source_authority_id == banknifty.source_authority_id {
            return Err(
                "anchored search-lineage NIFTY and BANKNIFTY source authorities alias".to_owned(),
            );
        }
        let pair_id = derive_pair_id(nifty.member_id, banknifty.member_id);
        Ok(Self {
            nifty,
            banknifty,
            pair_id,
        })
    }

    fn members(self, block_sequence: u64) -> [MemberRecordV2; 2] {
        [
            self.nifty.with_sequence(block_sequence),
            self.banknifty.with_sequence(block_sequence),
        ]
    }

    fn completion(
        self,
        block_sequence: u64,
    ) -> Result<CompletionRecordV2, AnchoredSearchLineageV2Refusal> {
        let members = self.members(block_sequence);
        let member_bytes = encode_members(&members)?;
        let total_folds = self
            .nifty
            .fold_count
            .checked_add(self.banknifty.fold_count)
            .ok_or_else(|| "anchored search-lineage total fold count overflowed".to_owned())?;
        let total_decided = self
            .nifty
            .decided_folds
            .checked_add(self.banknifty.decided_folds)
            .ok_or_else(|| "anchored search-lineage total decided count overflowed".to_owned())?;
        let total_profitable = self
            .nifty
            .profitable_oos_folds
            .checked_add(self.banknifty.profitable_oos_folds)
            .ok_or_else(|| {
                "anchored search-lineage total profitable count overflowed".to_owned()
            })?;
        let aggregate_oos_paisa = self
            .nifty
            .aggregate_oos_paisa
            .checked_add(self.banknifty.aggregate_oos_paisa)
            .ok_or_else(|| "anchored search-lineage paired OOS total overflowed".to_owned())?;
        Ok(CompletionRecordV2 {
            block_sequence,
            pair_id: self.pair_id,
            nifty_member_id: self.nifty.member_id,
            banknifty_member_id: self.banknifty.member_id,
            ordered_member_bytes_digest: hash_slices(ORDERED_MEMBER_BYTES_DOMAIN, &[&member_bytes]),
            total_folds,
            total_decided,
            total_profitable,
            aggregate_oos_paisa,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CompletionRecordV2 {
    block_sequence: u64,
    pair_id: [u8; 32],
    nifty_member_id: [u8; 32],
    banknifty_member_id: [u8; 32],
    ordered_member_bytes_digest: [u8; 32],
    total_folds: u64,
    total_decided: u64,
    total_profitable: u64,
    aggregate_oos_paisa: i64,
}

/// Public structural proof of one completed paired record.
///
/// The receipt proves self-consistent bytes only.  It is not an authenticated
/// Runner, instrument, feed, stored-data, Admission, or execution capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnchoredSearchLineageV2StructuralReceipt {
    block_sequence: u64,
    pair_id: [u8; 32],
    nifty_member_id: [u8; 32],
    banknifty_member_id: [u8; 32],
    total_folds: u64,
    total_decided: u64,
    total_profitable: u64,
    aggregate_oos_paisa: i64,
}

impl AnchoredSearchLineageV2StructuralReceipt {
    /// Stable identity of the canonical NIFTY-first pair.
    #[must_use]
    pub(crate) const fn pair_id(self) -> [u8; 32] {
        self.pair_id
    }

    /// Physical append sequence, excluded from semantic pair identity.
    #[must_use]
    pub(crate) const fn block_sequence(self) -> u64 {
        self.block_sequence
    }

    /// Structural NIFTY member identity.
    #[must_use]
    pub(crate) const fn nifty_member_id(self) -> [u8; 32] {
        self.nifty_member_id
    }

    /// Structural BANKNIFTY member identity.
    #[must_use]
    pub(crate) const fn banknifty_member_id(self) -> [u8; 32] {
        self.banknifty_member_id
    }

    /// Checked sum of the two exact fold counts.
    #[must_use]
    pub(crate) const fn total_folds(self) -> u64 {
        self.total_folds
    }

    /// Checked sum of decided folds.
    #[must_use]
    pub(crate) const fn total_decided(self) -> u64 {
        self.total_decided
    }

    /// Checked sum of profitable decided folds.
    #[must_use]
    pub(crate) const fn total_profitable(self) -> u64 {
        self.total_profitable
    }

    /// Checked signed sum of both exact chosen-OOS totals.
    #[must_use]
    pub(crate) const fn aggregate_oos_paisa(self) -> i64 {
        self.aggregate_oos_paisa
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct AuthenticatedAnchoredSearchLineageV2 {
    receipt: AnchoredSearchLineageV2StructuralReceipt,
    nifty: MemberRecordV2,
    banknifty: MemberRecordV2,
}

impl AuthenticatedAnchoredSearchLineageV2 {
    pub(crate) const fn structural_receipt(self) -> AnchoredSearchLineageV2StructuralReceipt {
        self.receipt
    }

    pub(crate) const fn nifty_source_authority_id(self) -> [u8; 32] {
        self.nifty.source_authority_id
    }

    pub(crate) const fn banknifty_source_authority_id(self) -> [u8; 32] {
        self.banknifty.source_authority_id
    }

    pub(crate) const fn nifty(self) -> AnchoredSearchLineageMemberAuthorityV2 {
        AnchoredSearchLineageMemberAuthorityV2 { member: self.nifty }
    }

    pub(crate) const fn banknifty(self) -> AnchoredSearchLineageMemberAuthorityV2 {
        AnchoredSearchLineageMemberAuthorityV2 {
            member: self.banknifty,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnchoredSearchLineageMemberAuthorityV2 {
    member: MemberRecordV2,
}

impl AnchoredSearchLineageMemberAuthorityV2 {
    /// Receipt-last identity of this exact reopened family member.
    ///
    /// This is the durable Search-Lineage member identity. It is deliberately
    /// distinct from Runner's source-authority identity below; a later
    /// Admission join must retain and compare both instead of treating two
    /// differently domain-separated digests as aliases.
    pub(crate) const fn member_id(self) -> [u8; 32] {
        self.member.member_id
    }

    pub(crate) const fn validation_policy_id(self) -> [u8; 32] {
        self.member.validation_policy_id
    }

    pub(crate) const fn validation_family_id(self) -> [u8; 32] {
        self.member.validation_family_id
    }

    pub(crate) const fn walk_facts_id(self) -> [u8; 32] {
        self.member.walk_facts_id
    }

    pub(crate) const fn source_authority_id(self) -> [u8; 32] {
        self.member.source_authority_id
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
}

pub(crate) enum AnchoredSearchLineageV2AuthenticatedCommit {
    Written(AuthenticatedAnchoredSearchLineageV2),
    Reused(AuthenticatedAnchoredSearchLineageV2),
}

impl AnchoredSearchLineageV2AuthenticatedCommit {
    pub(crate) const fn authority(self) -> AuthenticatedAnchoredSearchLineageV2 {
        match self {
            Self::Written(value) | Self::Reused(value) => value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TrailingPairV2 {
    first_member_record: u64,
    pair: PreparedPairV2,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    identity: PlatformIdentity,
    len: u64,
    digest: [u8; 32],
}

/// Bounded public structural view of the paired lineage files.
pub(crate) struct AnchoredSearchLineageV2Ledger {
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
    bounds: AnchoredSearchLineageV2Bounds,
    writable: bool,
    receipts: HashMap<[u8; 32], AnchoredSearchLineageV2StructuralReceipt>,
    trailing: Option<TrailingPairV2>,
    member_records: u64,
    completion_records: u64,
}

impl AnchoredSearchLineageV2Ledger {
    /// Opens existing files read-only and validates every bounded record.
    ///
    /// # Errors
    ///
    /// Refuses absent/symlink/non-directory roots, missing/symlink/non-regular
    /// children, exceeded bounds, ragged/torn/corrupt/reordered/duplicate bytes,
    /// lock failure, or any pathname/generation replacement during open.
    pub(crate) fn open_read(
        root: &Path,
        bounds: AnchoredSearchLineageV2Bounds,
    ) -> Result<Self, AnchoredSearchLineageV2Refusal> {
        Self::open(root, bounds, false)
    }

    fn open_write(
        root: &Path,
        bounds: AnchoredSearchLineageV2Bounds,
    ) -> Result<Self, AnchoredSearchLineageV2Refusal> {
        Self::open(root, bounds, true)
    }

    fn open(
        root: &Path,
        bounds: AnchoredSearchLineageV2Bounds,
        writable: bool,
    ) -> Result<Self, AnchoredSearchLineageV2Refusal> {
        let (root, root_file, root_identity) = open_root(root)?;
        let lock_path = root.join(LOCK_FILE);
        let member_path = root.join(MEMBER_FILE);
        let completion_path = root.join(COMPLETION_FILE);
        let (lock_file, lock_created) = open_child(&lock_path, writable)?;
        if writable {
            lock_file
                .lock()
                .map_err(|why| format!("cannot exclusively lock search-lineage ledger: {why}"))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot shared-lock search-lineage ledger: {why}"))?;
        }
        let opened = (|| {
            let (member_file, member_created) = open_child(&member_path, writable)?;
            let (completion_file, completion_created) = open_child(&completion_path, writable)?;
            if lock_created || member_created || completion_created {
                sync_directory(&root_file, &root)?;
            }
            if named_identity(&root)? != root_identity {
                return Err("anchored search-lineage root changed while opening files".to_owned());
            }
            let lock_generation = file_generation(&lock_file, &lock_path, bounds.completion_bytes)?;
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
                    .map_err(|why| format!("cannot clone held search-lineage lock: {why}"))?,
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
            .map_err(|why| format!("cannot unlock search-lineage ledger after open: {why}"));
        combine_with_unlock(opened, released)
    }

    fn scan(&mut self) -> Result<(), AnchoredSearchLineageV2Refusal> {
        let member_records = record_count(
            self.member_generation.len,
            ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES,
            self.bounds.pair_records.saturating_mul(2),
            "member",
        )?;
        let completion_records = record_count(
            self.completion_generation.len,
            ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES,
            self.bounds.pair_records,
            "Completion",
        )?;
        let covered_members = completion_records
            .checked_mul(2)
            .ok_or_else(|| "anchored search-lineage covered-member count overflowed".to_owned())?;
        if covered_members > member_records {
            return Err(format!(
                "anchored search-lineage is torn: {completion_records} Completions require {covered_members} members but file has {member_records}"
            ));
        }
        self.receipts.clear();
        self.receipts
            .try_reserve(
                usize::try_from(completion_records)
                    .map_err(|_| "search-lineage Completion count does not fit usize".to_owned())?,
            )
            .map_err(|why| format!("cannot reserve search-lineage receipt index: {why}"))?;
        let mut sequence = 0_u64;
        while sequence < completion_records {
            let members = read_member_pair(&mut self.member_file, sequence)?;
            let completion =
                decode_completion(&read_completion_at(&mut self.completion_file, sequence)?)?;
            let receipt = validate_complete_pair(sequence, &members, completion)?;
            if self.receipts.insert(receipt.pair_id, receipt).is_some() {
                return Err(format!(
                    "anchored search-lineage pair {} appears more than once",
                    hex32(receipt.pair_id)
                ));
            }
            sequence = sequence
                .checked_add(1)
                .ok_or_else(|| "search-lineage Completion scan overflowed".to_owned())?;
        }
        let trailing_members = member_records
            .checked_sub(covered_members)
            .ok_or_else(|| "search-lineage trailing-member underflow".to_owned())?;
        self.trailing = match trailing_members {
            0 => None,
            1 => {
                return Err(
                    "anchored search-lineage has a partial trailing pair; history is not rewritten"
                        .to_owned(),
                );
            }
            2 => {
                let members = read_member_pair(&mut self.member_file, completion_records)?;
                let pair = validate_member_pair(completion_records, &members)?;
                if self.receipts.contains_key(&pair.pair_id) {
                    return Err(format!(
                        "anchored search-lineage trailing pair {} duplicates a completed pair",
                        hex32(pair.pair_id)
                    ));
                }
                Some(TrailingPairV2 {
                    first_member_record: covered_members,
                    pair,
                })
            }
            _ => {
                return Err(format!(
                    "anchored search-lineage has {trailing_members} unreceipted member records"
                ));
            }
        };
        self.member_records = member_records;
        self.completion_records = completion_records;
        self.require_unchanged()
    }

    /// Revalidates both bounded generations and performs an indexed lookup.
    ///
    /// Generation validation is O(file bytes); only the final hash lookup is
    /// average O(1).
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Refuses lock failure or any root/path/file-generation change since open.
    pub(crate) fn structural_receipt(
        &self,
        pair_id: &[u8; 32],
    ) -> Result<Option<AnchoredSearchLineageV2StructuralReceipt>, AnchoredSearchLineageV2Refusal>
    {
        self.lock_file
            .lock_shared()
            .map_err(|why| format!("cannot shared-lock search-lineage lookup: {why}"))?;
        let result = self
            .require_unchanged()
            .map(|()| self.receipts.get(pair_id).copied());
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock search-lineage lookup: {why}"));
        combine_with_unlock(result, released)
    }

    fn append(
        &mut self,
        prepared: &PreparedPairV2,
    ) -> Result<(bool, AnchoredSearchLineageV2StructuralReceipt), AnchoredSearchLineageV2Refusal>
    {
        if !self.writable {
            return Err("anchored search-lineage ledger is read-only".to_owned());
        }
        self.lock_file
            .lock()
            .map_err(|why| format!("cannot exclusively lock search-lineage append: {why}"))?;
        let result = self.append_locked(prepared);
        let released = self
            .lock_file
            .unlock()
            .map_err(|why| format!("cannot unlock search-lineage append: {why}"));
        combine_with_unlock(result, released)
    }

    fn append_locked(
        &mut self,
        prepared: &PreparedPairV2,
    ) -> Result<(bool, AnchoredSearchLineageV2StructuralReceipt), AnchoredSearchLineageV2Refusal>
    {
        self.require_unchanged()?;
        if let Some(receipt) = self.receipts.get(&prepared.pair_id).copied() {
            self.require_exact_existing(prepared, receipt)?;
            self.member_file
                .sync_data()
                .map_err(|why| format!("cannot sync reused search-lineage members: {why}"))?;
            self.completion_file
                .sync_data()
                .map_err(|why| format!("cannot sync reused search-lineage Completion: {why}"))?;
            sync_directory(&self.root_file, &self.root)?;
            self.require_unchanged()?;
            return Ok((false, receipt));
        }
        if let Some(trailing) = self.trailing {
            if trailing.first_member_record != self.member_records.saturating_sub(2)
                || trailing.pair != *prepared
            {
                return Err(format!(
                    "anchored search-lineage trailing pair {} is not exact retry {}",
                    hex32(trailing.pair.pair_id),
                    hex32(prepared.pair_id)
                ));
            }
            self.member_file
                .sync_data()
                .map_err(|why| format!("cannot sync retry search-lineage members: {why}"))?;
            self.append_completion(prepared)?;
            let receipt = self
                .receipts
                .get(&prepared.pair_id)
                .copied()
                .ok_or_else(|| "retried search-lineage pair was not indexed".to_owned())?;
            return Ok((true, receipt));
        }
        self.require_append_capacity()?;
        let members = prepared.members(self.completion_records);
        for raw in encode_member_records(&members)? {
            append_raw(&mut self.member_file, &raw)?;
        }
        self.member_file
            .sync_data()
            .map_err(|why| format!("cannot sync search-lineage members: {why}"))?;
        self.member_records = self
            .member_records
            .checked_add(2)
            .ok_or_else(|| "search-lineage member count overflowed".to_owned())?;
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
            .ok_or_else(|| "written search-lineage pair was not indexed".to_owned())?;
        Ok((true, receipt))
    }

    fn append_completion(
        &mut self,
        prepared: &PreparedPairV2,
    ) -> Result<(), AnchoredSearchLineageV2Refusal> {
        let completion = prepared.completion(self.completion_records)?;
        append_raw(&mut self.completion_file, &encode_completion(&completion)?)?;
        self.completion_file
            .sync_data()
            .map_err(|why| format!("cannot sync search-lineage Completion: {why}"))?;
        sync_directory(&self.root_file, &self.root)?;
        self.completion_records = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "search-lineage Completion count overflowed".to_owned())?;
        self.completion_generation = file_generation(
            &self.completion_file,
            &self.completion_path,
            self.bounds.completion_bytes,
        )?;
        self.scan()
    }

    fn require_exact_existing(
        &mut self,
        prepared: &PreparedPairV2,
        receipt: AnchoredSearchLineageV2StructuralReceipt,
    ) -> Result<(), AnchoredSearchLineageV2Refusal> {
        let members = read_member_pair(&mut self.member_file, receipt.block_sequence)?;
        if members != prepared.members(receipt.block_sequence) {
            return Err(
                "existing search-lineage members differ from opaque preparation".to_owned(),
            );
        }
        let completion = decode_completion(&read_completion_at(
            &mut self.completion_file,
            receipt.block_sequence,
        )?)?;
        if completion != prepared.completion(receipt.block_sequence)? {
            return Err(
                "existing search-lineage Completion differs from opaque preparation".to_owned(),
            );
        }
        Ok(())
    }

    fn require_append_capacity(&self) -> Result<(), AnchoredSearchLineageV2Refusal> {
        let next_pairs = self
            .completion_records
            .checked_add(1)
            .ok_or_else(|| "search-lineage pair count overflowed".to_owned())?;
        if next_pairs > self.bounds.pair_records {
            return Err(format!(
                "search-lineage append reaches {next_pairs} pairs above explicit maximum {}",
                self.bounds.pair_records
            ));
        }
        let next_members = self
            .member_records
            .checked_add(2)
            .ok_or_else(|| "search-lineage member count overflowed".to_owned())?;
        let next_member_bytes = next_members
            .checked_mul(ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES as u64)
            .ok_or_else(|| "search-lineage member bytes overflowed".to_owned())?;
        if next_member_bytes > self.bounds.member_bytes {
            return Err("search-lineage append exceeds explicit member-byte bound".to_owned());
        }
        let next_completion_bytes = next_pairs
            .checked_mul(ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES as u64)
            .ok_or_else(|| "search-lineage Completion bytes overflowed".to_owned())?;
        if next_completion_bytes > self.bounds.completion_bytes {
            return Err("search-lineage append exceeds explicit Completion-byte bound".to_owned());
        }
        Ok(())
    }

    fn require_unchanged(&self) -> Result<(), AnchoredSearchLineageV2Refusal> {
        if named_identity(&self.root)? != self.root_identity
            || PlatformIdentity::of(
                &self
                    .root_file
                    .metadata()
                    .map_err(|why| format!("cannot stat held search-lineage root: {why}"))?,
            ) != self.root_identity
        {
            return Err("anchored search-lineage root changed after open".to_owned());
        }
        if file_generation(
            &self.lock_file,
            &self.lock_path,
            self.bounds.completion_bytes,
        )? != self.lock_generation
        {
            return Err("anchored search-lineage lock generation changed".to_owned());
        }
        if file_generation(
            &self.member_file,
            &self.member_path,
            self.bounds.member_bytes,
        )? != self.member_generation
        {
            return Err("anchored search-lineage member generation changed".to_owned());
        }
        if file_generation(
            &self.completion_file,
            &self.completion_path,
            self.bounds.completion_bytes,
        )? != self.completion_generation
        {
            return Err("anchored search-lineage Completion generation changed".to_owned());
        }
        Ok(())
    }

    fn authenticate(
        &mut self,
        receipt: AnchoredSearchLineageV2StructuralReceipt,
        prepared: &PreparedPairV2,
    ) -> Result<AuthenticatedAnchoredSearchLineageV2, AnchoredSearchLineageV2Refusal> {
        self.require_unchanged()?;
        if self.receipts.get(&prepared.pair_id) != Some(&receipt) {
            return Err(
                "search-lineage receipt is not the freshly reopened indexed pair".to_owned(),
            );
        }
        self.require_exact_existing(prepared, receipt)?;
        self.require_unchanged()?;
        Ok(AuthenticatedAnchoredSearchLineageV2 {
            receipt,
            nifty: prepared.nifty,
            banknifty: prepared.banknifty,
        })
    }
}

/// Persists one canonical pair from opaque Runner projections.
///
/// The writer is dropped, both files are freshly reopened read-only, and the
/// exact bytes are compared again with the opaque-derived preparation before
/// this function returns the crate-private authenticated capability.
///
/// # Errors
///
/// Returns the first opaque-projection hierarchy, cross-family alias, path,
/// bound, lock, durability, codec, retry, fresh-reopen, or authentication
/// refusal.  No fallback or partial authority is returned.
pub(crate) fn persist_anchored_search_lineage_v2(
    root: &Path,
    bounds: AnchoredSearchLineageV2Bounds,
    nifty: &AnchoredSearchAuthorityProjectionV2,
    banknifty: &AnchoredSearchAuthorityProjectionV2,
) -> Result<AnchoredSearchLineageV2AuthenticatedCommit, AnchoredSearchLineageV2Refusal> {
    let prepared = PreparedPairV2::from_opaque(nifty, banknifty)?;
    let mut writer = AnchoredSearchLineageV2Ledger::open_write(root, bounds)?;
    let (written, receipt) = writer.append(&prepared)?;
    drop(writer);
    let mut reopened = AnchoredSearchLineageV2Ledger::open_read(root, bounds)?;
    let observed = reopened
        .structural_receipt(&prepared.pair_id)?
        .ok_or_else(|| "fresh search-lineage reopen omitted the persisted pair".to_owned())?;
    if observed != receipt {
        return Err("fresh search-lineage reopen changed the structural receipt".to_owned());
    }
    let authority = reopened.authenticate(observed, &prepared)?;
    Ok(if written {
        AnchoredSearchLineageV2AuthenticatedCommit::Written(authority)
    } else {
        AnchoredSearchLineageV2AuthenticatedCommit::Reused(authority)
    })
}

fn validate_complete_pair(
    sequence: u64,
    members: &[MemberRecordV2; 2],
    completion: CompletionRecordV2,
) -> Result<AnchoredSearchLineageV2StructuralReceipt, AnchoredSearchLineageV2Refusal> {
    let prepared = validate_member_pair(sequence, members)?;
    let expected = prepared.completion(sequence)?;
    if completion != expected {
        return Err(format!(
            "anchored search-lineage Completion {sequence} does not exactly bind its ordered members"
        ));
    }
    Ok(AnchoredSearchLineageV2StructuralReceipt {
        block_sequence: sequence,
        pair_id: completion.pair_id,
        nifty_member_id: completion.nifty_member_id,
        banknifty_member_id: completion.banknifty_member_id,
        total_folds: completion.total_folds,
        total_decided: completion.total_decided,
        total_profitable: completion.total_profitable,
        aggregate_oos_paisa: completion.aggregate_oos_paisa,
    })
}

fn validate_member_pair(
    sequence: u64,
    members: &[MemberRecordV2; 2],
) -> Result<PreparedPairV2, AnchoredSearchLineageV2Refusal> {
    let nifty = members
        .first()
        .copied()
        .ok_or_else(|| "search-lineage pair omitted NIFTY".to_owned())?;
    let banknifty = members
        .get(1)
        .copied()
        .ok_or_else(|| "search-lineage pair omitted BANKNIFTY".to_owned())?;
    if nifty.block_sequence != sequence || banknifty.block_sequence != sequence {
        return Err(format!(
            "search-lineage pair sequence does not match canonical {sequence}"
        ));
    }
    if nifty.family != SearchFamilyV2::Nifty || banknifty.family != SearchFamilyV2::BankNifty {
        return Err("search-lineage members are not canonical NIFTY then BANKNIFTY".to_owned());
    }
    nifty.validate()?;
    banknifty.validate()?;
    if nifty.source_authority_id == banknifty.source_authority_id {
        return Err("search-lineage NIFTY/BANKNIFTY source authorities alias".to_owned());
    }
    Ok(PreparedPairV2 {
        // A block sequence is physical append position, not semantic source
        // authority.  Normalizing it here makes a fully synced crash tail at
        // any later sequence exactly retryable against an opaque preparation,
        // whose pre-append sequence is necessarily zero.
        nifty: nifty.with_sequence(0),
        banknifty: banknifty.with_sequence(0),
        pair_id: derive_pair_id(nifty.member_id, banknifty.member_id),
    })
}

fn derive_pair_id(nifty_member_id: [u8; 32], banknifty_member_id: [u8; 32]) -> [u8; 32] {
    hash_slices(PAIR_ID_DOMAIN, &[&nifty_member_id, &banknifty_member_id])
}

fn encode_member_records(
    members: &[MemberRecordV2; 2],
) -> Result<[[u8; ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES]; 2], AnchoredSearchLineageV2Refusal> {
    Ok([
        encode_member(
            members
                .first()
                .ok_or_else(|| "cannot encode absent NIFTY member".to_owned())?,
        )?,
        encode_member(
            members
                .get(1)
                .ok_or_else(|| "cannot encode absent BANKNIFTY member".to_owned())?,
        )?,
    ])
}

fn encode_members(
    members: &[MemberRecordV2; 2],
) -> Result<Vec<u8>, AnchoredSearchLineageV2Refusal> {
    let records = encode_member_records(members)?;
    let capacity = ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES
        .checked_mul(2)
        .ok_or_else(|| "ordered search-lineage member capacity overflowed".to_owned())?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|why| format!("cannot reserve ordered search-lineage bytes: {why}"))?;
    for record in records {
        bytes.extend_from_slice(&record);
    }
    Ok(bytes)
}

fn encode_member(
    value: &MemberRecordV2,
) -> Result<[u8; ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES], AnchoredSearchLineageV2Refusal> {
    value.validate()?;
    let mut raw = [0_u8; ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES];
    let mut writer = FixedWriter::new(
        raw.get_mut(..MEMBER_PAYLOAD_BYTES)
            .ok_or_else(|| "search-lineage member payload range is invalid".to_owned())?,
    );
    writer.write(&MEMBER_MAGIC)?;
    writer.u32(VERSION)?;
    writer.u32(MEMBER_DOMAIN)?;
    writer.u64(value.block_sequence)?;
    writer.u8(value.family as u8)?;
    writer.zeros(7)?;
    for identity in [
        value.validation_policy_id,
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
    writer.zeros_to_end()?;
    let seal = hash_slices(
        MEMBER_SEAL_DOMAIN,
        &[raw
            .get(..MEMBER_PAYLOAD_BYTES)
            .ok_or_else(|| "search-lineage member payload is absent".to_owned())?],
    );
    raw.get_mut(MEMBER_PAYLOAD_BYTES..)
        .ok_or_else(|| "search-lineage member seal range is absent".to_owned())?
        .copy_from_slice(&seal);
    Ok(raw)
}

fn decode_member(
    raw: &[u8; ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES],
) -> Result<MemberRecordV2, AnchoredSearchLineageV2Refusal> {
    let payload = raw
        .get(..MEMBER_PAYLOAD_BYTES)
        .ok_or_else(|| "search-lineage member payload is absent".to_owned())?;
    let expected = hash_slices(MEMBER_SEAL_DOMAIN, &[payload]);
    if raw.get(MEMBER_PAYLOAD_BYTES..) != Some(expected.as_slice()) {
        return Err("anchored search-lineage member seal mismatch".to_owned());
    }
    let mut reader = FixedReader::new(payload);
    reader.require(&MEMBER_MAGIC, "member magic")?;
    reader.require_u32(VERSION, "member version")?;
    reader.require_u32(MEMBER_DOMAIN, "member domain")?;
    let value = MemberRecordV2 {
        block_sequence: reader.u64()?,
        family: SearchFamilyV2::decode(reader.u8()?)?,
        validation_policy_id: {
            reader.require_zero(7, "member family reserve")?;
            reader.array()?
        },
        validation_family_id: reader.array()?,
        walk_facts_id: reader.array()?,
        source_authority_id: reader.array()?,
        member_id: reader.array()?,
        fold_count: reader.u64()?,
        decided_folds: reader.u64()?,
        profitable_oos_folds: reader.u64()?,
        aggregate_oos_paisa: reader.i64()?,
    };
    reader.require_zero_to_end("member reserve")?;
    value.validate()?;
    Ok(value)
}

fn encode_completion(
    value: &CompletionRecordV2,
) -> Result<[u8; ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES], AnchoredSearchLineageV2Refusal> {
    let mut raw = [0_u8; ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES];
    let mut writer = FixedWriter::new(
        raw.get_mut(..COMPLETION_PAYLOAD_BYTES)
            .ok_or_else(|| "search-lineage Completion payload range is invalid".to_owned())?,
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
    writer.zeros_to_end()?;
    let seal = hash_slices(
        COMPLETION_SEAL_DOMAIN,
        &[raw
            .get(..COMPLETION_PAYLOAD_BYTES)
            .ok_or_else(|| "search-lineage Completion payload is absent".to_owned())?],
    );
    raw.get_mut(COMPLETION_PAYLOAD_BYTES..)
        .ok_or_else(|| "search-lineage Completion seal range is absent".to_owned())?
        .copy_from_slice(&seal);
    Ok(raw)
}

fn decode_completion(
    raw: &[u8; ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES],
) -> Result<CompletionRecordV2, AnchoredSearchLineageV2Refusal> {
    let payload = raw
        .get(..COMPLETION_PAYLOAD_BYTES)
        .ok_or_else(|| "search-lineage Completion payload is absent".to_owned())?;
    let expected = hash_slices(COMPLETION_SEAL_DOMAIN, &[payload]);
    if raw.get(COMPLETION_PAYLOAD_BYTES..) != Some(expected.as_slice()) {
        return Err("anchored search-lineage Completion seal mismatch".to_owned());
    }
    let mut reader = FixedReader::new(payload);
    reader.require(&COMPLETION_MAGIC, "Completion magic")?;
    reader.require_u32(VERSION, "Completion version")?;
    reader.require_u32(COMPLETION_DOMAIN, "Completion domain")?;
    let value = CompletionRecordV2 {
        block_sequence: reader.u64()?,
        pair_id: reader.array()?,
        nifty_member_id: reader.array()?,
        banknifty_member_id: reader.array()?,
        ordered_member_bytes_digest: reader.array()?,
        total_folds: reader.u64()?,
        total_decided: reader.u64()?,
        total_profitable: reader.u64()?,
        aggregate_oos_paisa: reader.i64()?,
    };
    reader.require_zero_to_end("Completion reserve")?;
    for (name, identity) in [
        ("pair", value.pair_id),
        ("NIFTY member", value.nifty_member_id),
        ("BANKNIFTY member", value.banknifty_member_id),
        ("ordered-member bytes", value.ordered_member_bytes_digest),
    ] {
        if identity == [0; 32] {
            return Err(format!("search-lineage Completion {name} identity is zero"));
        }
    }
    if value.total_folds == 0
        || value.total_decided > value.total_folds
        || value.total_profitable > value.total_decided
    {
        return Err("search-lineage Completion count hierarchy is invalid".to_owned());
    }
    Ok(value)
}

struct FixedWriter<'a> {
    bytes: &'a mut [u8],
    cursor: usize,
}

impl<'a> FixedWriter<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn write(&mut self, value: &[u8]) -> Result<(), AnchoredSearchLineageV2Refusal> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or_else(|| "search-lineage writer cursor overflowed".to_owned())?;
        self.bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "search-lineage fixed record overflowed".to_owned())?
            .copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), AnchoredSearchLineageV2Refusal> {
        self.write(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), AnchoredSearchLineageV2Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), AnchoredSearchLineageV2Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), AnchoredSearchLineageV2Refusal> {
        self.write(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), AnchoredSearchLineageV2Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "search-lineage zero reserve overflowed".to_owned())?;
        let target = self
            .bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "search-lineage zero reserve exceeds record".to_owned())?;
        target.fill(0);
        self.cursor = end;
        Ok(())
    }

    fn zeros_to_end(&mut self) -> Result<(), AnchoredSearchLineageV2Refusal> {
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

    fn take(&mut self, count: usize) -> Result<&'a [u8], AnchoredSearchLineageV2Refusal> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "search-lineage reader cursor overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "search-lineage fixed record ended early".to_owned())?;
        self.cursor = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], AnchoredSearchLineageV2Refusal> {
        self.take(N)?
            .try_into()
            .map_err(|_| "search-lineage fixed array width mismatch".to_owned())
    }

    fn u8(&mut self) -> Result<u8, AnchoredSearchLineageV2Refusal> {
        self.take(1)?
            .first()
            .copied()
            .ok_or_else(|| "search-lineage byte is absent".to_owned())
    }

    fn u32(&mut self) -> Result<u32, AnchoredSearchLineageV2Refusal> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, AnchoredSearchLineageV2Refusal> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn i64(&mut self) -> Result<i64, AnchoredSearchLineageV2Refusal> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    fn require(
        &mut self,
        expected: &[u8],
        name: &str,
    ) -> Result<(), AnchoredSearchLineageV2Refusal> {
        if self.take(expected.len())? != expected {
            return Err(format!("search-lineage {name} mismatch"));
        }
        Ok(())
    }

    fn require_u32(
        &mut self,
        expected: u32,
        name: &str,
    ) -> Result<(), AnchoredSearchLineageV2Refusal> {
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
    ) -> Result<(), AnchoredSearchLineageV2Refusal> {
        if self.take(count)?.iter().any(|value| *value != 0) {
            return Err(format!("search-lineage {name} is nonzero"));
        }
        Ok(())
    }

    fn require_zero_to_end(&mut self, name: &str) -> Result<(), AnchoredSearchLineageV2Refusal> {
        self.require_zero(self.bytes.len().saturating_sub(self.cursor), name)
    }
}

fn read_member_pair(
    file: &mut File,
    pair_sequence: u64,
) -> Result<[MemberRecordV2; 2], AnchoredSearchLineageV2Refusal> {
    let first = pair_sequence
        .checked_mul(2)
        .ok_or_else(|| "search-lineage member offset overflowed".to_owned())?;
    Ok([
        decode_member(&read_fixed_at::<ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES>(
            file, first,
        )?)?,
        decode_member(&read_fixed_at::<ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES>(
            file,
            first
                .checked_add(1)
                .ok_or_else(|| "search-lineage second-member offset overflowed".to_owned())?,
        )?)?,
    ])
}

fn read_completion_at(
    file: &mut File,
    sequence: u64,
) -> Result<[u8; ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES], AnchoredSearchLineageV2Refusal> {
    read_fixed_at(file, sequence)
}

fn read_fixed_at<const N: usize>(
    file: &mut File,
    record: u64,
) -> Result<[u8; N], AnchoredSearchLineageV2Refusal> {
    let width =
        u64::try_from(N).map_err(|_| "search-lineage record width does not fit u64".to_owned())?;
    let offset = record
        .checked_mul(width)
        .ok_or_else(|| "search-lineage record offset overflowed".to_owned())?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|why| format!("cannot seek search-lineage record {record}: {why}"))?;
    let mut raw = [0_u8; N];
    file.read_exact(&mut raw)
        .map_err(|why| format!("cannot read search-lineage record {record}: {why}"))?;
    Ok(raw)
}

fn record_count(
    len: u64,
    width: usize,
    maximum: u64,
    name: &str,
) -> Result<u64, AnchoredSearchLineageV2Refusal> {
    let width = u64::try_from(width)
        .map_err(|_| format!("search-lineage {name} width does not fit u64"))?;
    if !len.is_multiple_of(width) {
        return Err(format!(
            "search-lineage {name} file is ragged: {len} bytes is not divisible by {width}"
        ));
    }
    let count = len / width;
    if count > maximum {
        return Err(format!(
            "search-lineage {name} file has {count} records above explicit maximum {maximum}"
        ));
    }
    Ok(count)
}

fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), AnchoredSearchLineageV2Refusal> {
    file.seek(SeekFrom::End(0))
        .map_err(|why| format!("cannot seek search-lineage append: {why}"))?;
    file.write_all(raw)
        .map_err(|why| format!("cannot append search-lineage bytes: {why}"))
}

fn open_root(
    root: &Path,
) -> Result<(PathBuf, File, PlatformIdentity), AnchoredSearchLineageV2Refusal> {
    let metadata = std::fs::symlink_metadata(root).map_err(|why| {
        format!(
            "cannot inspect search-lineage root {}: {why}",
            root.display()
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "search-lineage root {} must be a real directory, not a symlink",
            root.display()
        ));
    }
    let canonical = std::fs::canonicalize(root)
        .map_err(|why| format!("cannot canonicalize search-lineage root: {why}"))?;
    let file = File::open(&canonical)
        .map_err(|why| format!("cannot open search-lineage root directory: {why}"))?;
    let held = file
        .metadata()
        .map_err(|why| format!("cannot stat held search-lineage root: {why}"))?;
    if !held.is_dir() || PlatformIdentity::of(&held) != PlatformIdentity::of(&metadata) {
        return Err("search-lineage root changed during open".to_owned());
    }
    Ok((canonical, file, PlatformIdentity::of(&held)))
}

fn named_identity(path: &Path) -> Result<PlatformIdentity, AnchoredSearchLineageV2Refusal> {
    let metadata = std::fs::symlink_metadata(path).map_err(|why| {
        format!(
            "cannot restat search-lineage path {}: {why}",
            path.display()
        )
    })?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "search-lineage path {} became a symlink",
            path.display()
        ));
    }
    Ok(PlatformIdentity::of(&metadata))
}

fn open_child(path: &Path, writable: bool) -> Result<(File, bool), AnchoredSearchLineageV2Refusal> {
    let existed = match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(format!(
                    "search-lineage child {} must be a regular non-symlink file",
                    path.display()
                ));
            }
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            return Err(format!(
                "cannot inspect search-lineage child {}: {error}",
                path.display()
            ));
        }
    };
    if !existed && !writable {
        return Err(format!("search-lineage child {} is absent", path.display()));
    }
    let mut options = OpenOptions::new();
    options.read(true).write(writable).create(writable);
    #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
    {
        options.mode(0o600).custom_flags(O_NOFOLLOW_FLAG);
    }
    let file = options
        .open(path)
        .map_err(|why| format!("cannot open search-lineage child {}: {why}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|why| format!("cannot stat held search-lineage child: {why}"))?;
    if !metadata.is_file() || named_identity(path)? != PlatformIdentity::of(&metadata) {
        return Err(format!(
            "search-lineage child {} changed during open",
            path.display()
        ));
    }
    Ok((file, !existed))
}

fn file_generation(
    file: &File,
    path: &Path,
    maximum: u64,
) -> Result<FileGeneration, AnchoredSearchLineageV2Refusal> {
    let before = file
        .metadata()
        .map_err(|why| format!("cannot stat held search-lineage file: {why}"))?;
    if !before.is_file() || before.len() > maximum {
        return Err(format!(
            "search-lineage file {} length {} exceeds explicit maximum {maximum}",
            path.display(),
            before.len()
        ));
    }
    let identity = PlatformIdentity::of(&before);
    if named_identity(path)? != identity {
        return Err(format!(
            "search-lineage file {} was replaced",
            path.display()
        ));
    }
    let mut reader = file
        .try_clone()
        .map_err(|why| format!("cannot clone search-lineage file for hashing: {why}"))?;
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|why| format!("cannot rewind search-lineage file: {why}"))?;
    let digest = hash_exact_prefix(&mut reader, path, before.len())?;
    let after = file
        .metadata()
        .map_err(|why| format!("cannot restat held search-lineage file: {why}"))?;
    if PlatformIdentity::of(&after) != identity
        || after.len() != before.len()
        || named_identity(path)? != identity
    {
        return Err(format!(
            "search-lineage file {} changed while hashing",
            path.display()
        ));
    }
    Ok(FileGeneration {
        identity,
        len: before.len(),
        digest,
    })
}

fn hash_exact_prefix(
    reader: &mut impl std::io::Read,
    path: &Path,
    length: u64,
) -> Result<[u8; 32], AnchoredSearchLineageV2Refusal> {
    let mut hasher = Hasher::new();
    hasher.update(FILE_GENERATION_DOMAIN);
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    let mut remaining = length;
    while remaining != 0 {
        let requested = usize::try_from(remaining.min(READ_CHUNK_BYTES as u64))
            .map_err(|_| "search-lineage bounded hash width does not fit usize".to_owned())?;
        let read = reader
            .read(
                buffer
                    .get_mut(..requested)
                    .ok_or_else(|| "search-lineage bounded hash range is invalid".to_owned())?,
            )
            .map_err(|why| format!("cannot hash search-lineage file: {why}"))?;
        if read == 0 {
            return Err(format!(
                "search-lineage file {} shortened while hashing",
                path.display()
            ));
        }
        hasher.update(
            buffer
                .get(..read)
                .ok_or_else(|| "search-lineage hash chunk range is invalid".to_owned())?,
        );
        remaining = remaining
            .checked_sub(
                u64::try_from(read)
                    .map_err(|_| "search-lineage hash count does not fit u64".to_owned())?,
            )
            .ok_or_else(|| "search-lineage bounded hash count underflowed".to_owned())?;
    }
    Ok(hasher.finalize())
}

fn sync_directory(file: &File, root: &Path) -> Result<(), AnchoredSearchLineageV2Refusal> {
    file.sync_all()
        .map_err(|why| format!("cannot sync search-lineage root {}: {why}", root.display()))
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
    result: Result<T, AnchoredSearchLineageV2Refusal>,
    released: Result<(), AnchoredSearchLineageV2Refusal>,
) -> Result<T, AnchoredSearchLineageV2Refusal> {
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
    use costs::fill::Direction;
    use engine::Ladder;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use runner::Sweeper;
    use runner::outcome::Horizon;
    use runner::validate::{DEFAULT_RUNGS, ExecutionSeries};
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    struct NeverEofReader {
        bytes_read: u64,
    }

    impl std::io::Read for NeverEofReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            buffer.fill(0x5a);
            let read = u64::try_from(buffer.len())
                .map_err(|_| std::io::Error::other("fixture read width does not fit u64"))?;
            self.bytes_read = self
                .bytes_read
                .checked_add(read)
                .ok_or_else(|| std::io::Error::other("fixture read count overflowed"))?;
            Ok(buffer.len())
        }
    }

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(label: &str) -> Self {
            let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "brutex-search-lineage-v2-{}-{label}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir(&path).expect("create isolated search-lineage root");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            if self.0.exists() {
                std::fs::remove_dir_all(&self.0).expect("remove isolated search-lineage root");
            }
        }
    }

    fn bounds() -> AnchoredSearchLineageV2Bounds {
        AnchoredSearchLineageV2Bounds::new(
            8,
            16 * ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES as u64,
            8 * ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES as u64,
        )
        .expect("fixture bounds are valid")
    }

    #[test]
    fn generation_hash_stops_at_the_captured_length_even_when_reader_never_reaches_eof() {
        let chunk = u64::try_from(READ_CHUNK_BYTES).expect("read chunk width fits u64");
        let captured_length = chunk
            .checked_mul(2)
            .and_then(|value| value.checked_add(7))
            .expect("fixture captured length fits u64");
        let mut reader = NeverEofReader { bytes_read: 0 };

        let digest = hash_exact_prefix(
            &mut reader,
            Path::new("never-eof-search-lineage-fixture"),
            captured_length,
        )
        .expect("bounded hashing completes without waiting for EOF");

        assert_ne!(digest, [0; 32]);
        assert_eq!(reader.bytes_read, captured_length);
    }

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    fn projection(horizon: u32, direction: Direction) -> AnchoredSearchAuthorityProjectionV2 {
        let bars = runner::synthetic::sessions(12);
        let sweeper = Sweeper::new(Ladder::with_min_hits(1_200).with_ceiling(20_000));
        let mut builder = |slice: &[indicators::Candle]| {
            let mut evaluator = evaluator();
            Ok(Column::build(slice, &mut evaluator))
        };
        runner::validate::walk_forward_projected_prepared_anchored_admission_v2(
            &bars,
            ExecutionSeries {
                bars: &bars,
                signal_length_micros: 60_000_000,
            },
            Horizon::bars(horizon).expect("nonzero horizon"),
            3,
            direction,
            &sweeper,
            &mut builder,
            DEFAULT_RUNGS,
        )
        .expect("opaque anchored fixture runs")
        .search_authority_projection()
        .expect("opaque anchored fixture revalidates")
    }

    fn projections() -> (
        AnchoredSearchAuthorityProjectionV2,
        AnchoredSearchAuthorityProjectionV2,
    ) {
        static PROJECTIONS: OnceLock<(
            AnchoredSearchAuthorityProjectionV2,
            AnchoredSearchAuthorityProjectionV2,
        )> = OnceLock::new();
        *PROJECTIONS.get_or_init(|| {
            (
                projection(15, Direction::Long),
                projection(16, Direction::Short),
            )
        })
    }

    fn initialize(root: &Path) {
        drop(
            AnchoredSearchLineageV2Ledger::open_write(root, bounds())
                .expect("initialize empty search-lineage ledger"),
        );
    }

    fn lengths(root: &Path) -> (u64, u64) {
        (
            std::fs::metadata(root.join(MEMBER_FILE))
                .expect("member metadata")
                .len(),
            std::fs::metadata(root.join(COMPLETION_FILE))
                .expect("Completion metadata")
                .len(),
        )
    }

    fn overwrite_byte(path: &Path, offset: u64) {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("open attack file");
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

    fn assert_refuses<T>(result: Result<T, AnchoredSearchLineageV2Refusal>, needle: &str) {
        let error = result.err().expect("operation must refuse");
        assert!(
            error.contains(needle),
            "expected refusal containing {needle:?}, got {error:?}"
        );
    }

    #[test]
    fn explicit_bounds_cover_pair_member_and_completion_axes() {
        assert_refuses(AnchoredSearchLineageV2Bounds::new(0, 1, 1), "nonzero");
        assert_refuses(
            AnchoredSearchLineageV2Bounds::new(
                2,
                3 * ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES as u64,
                2 * ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES as u64,
            ),
            "cannot hold",
        );
        assert_refuses(
            AnchoredSearchLineageV2Bounds::new(
                2,
                4 * ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES as u64,
                ANCHORED_SEARCH_LINEAGE_V2_COMPLETION_BYTES as u64,
            ),
            "cannot hold",
        );
    }

    #[test]
    fn opaque_projections_make_fixed_canonical_non_aliasing_records() {
        let (nifty, banknifty) = projections();
        let prepared = PreparedPairV2::from_opaque(&nifty, &banknifty).expect("prepare pair");
        let members = prepared.members(7);
        let raw = encode_member_records(&members).expect("encode fixed records");
        assert_eq!(raw[0].len(), ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES);
        assert_eq!(raw[1].len(), ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES);
        assert_eq!(decode_member(&raw[0]).expect("decode NIFTY"), members[0]);
        assert_eq!(
            decode_member(&raw[1]).expect("decode BANKNIFTY"),
            members[1]
        );
        assert_eq!(members[0].family, SearchFamilyV2::Nifty);
        assert_eq!(members[1].family, SearchFamilyV2::BankNifty);
        assert_ne!(
            members[0].source_authority_id,
            members[1].source_authority_id
        );
        assert_refuses(PreparedPairV2::from_opaque(&nifty, &nifty), "alias");
    }

    #[test]
    fn fresh_reopen_authenticates_then_exact_retry_reuses_without_growth() {
        let root = TestRoot::new("persist-reuse");
        let (nifty, banknifty) = projections();
        let first = persist_anchored_search_lineage_v2(root.path(), bounds(), &nifty, &banknifty)
            .expect("write and freshly authenticate");
        let first_authority = match first {
            AnchoredSearchLineageV2AuthenticatedCommit::Written(value) => value,
            AnchoredSearchLineageV2AuthenticatedCommit::Reused(_) => {
                panic!("first append must write")
            }
        };
        assert_ne!(
            first_authority.nifty_source_authority_id(),
            first_authority.banknifty_source_authority_id()
        );
        let nifty_authority = first_authority.nifty();
        let banknifty_authority = first_authority.banknifty();
        assert_eq!(
            nifty_authority.validation_policy_id(),
            nifty.policy_identity().digest()
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
            banknifty_authority.validation_policy_id(),
            banknifty.policy_identity().digest()
        );
        assert_eq!(
            banknifty_authority.validation_family_id(),
            banknifty.family_identity().digest()
        );
        assert_eq!(
            banknifty_authority.walk_facts_id(),
            banknifty.walk_identity().digest()
        );
        assert_ne!(
            nifty_authority.source_authority_id(),
            banknifty_authority.source_authority_id()
        );
        assert_eq!(banknifty_authority.fold_count(), banknifty.fold_count());
        assert_eq!(
            banknifty_authority.decided_folds(),
            banknifty.decided_folds()
        );
        assert_eq!(
            banknifty_authority.profitable_oos_folds(),
            banknifty.profitable_oos_folds()
        );
        assert_eq!(
            banknifty_authority.aggregate_oos_paisa(),
            banknifty.aggregate_oos_paisa()
        );
        let first_receipt = first_authority.structural_receipt();
        assert_eq!(first_receipt.block_sequence(), 0);
        assert_ne!(first_receipt.nifty_member_id(), [0; 32]);
        assert_ne!(first_receipt.banknifty_member_id(), [0; 32]);
        assert_eq!(nifty_authority.member_id(), first_receipt.nifty_member_id());
        assert_eq!(
            banknifty_authority.member_id(),
            first_receipt.banknifty_member_id()
        );
        assert!(first_receipt.total_folds() > 0);
        assert!(first_receipt.total_decided() <= first_receipt.total_folds());
        assert!(first_receipt.total_profitable() <= first_receipt.total_decided());
        let _signed_oos_sum = first_receipt.aggregate_oos_paisa();
        let before = lengths(root.path());
        let retry = persist_anchored_search_lineage_v2(root.path(), bounds(), &nifty, &banknifty)
            .expect("exact retry reuses");
        let retry_receipt = match retry {
            AnchoredSearchLineageV2AuthenticatedCommit::Reused(value) => value.structural_receipt(),
            AnchoredSearchLineageV2AuthenticatedCommit::Written(_) => {
                panic!("exact retry must reuse")
            }
        };
        assert_eq!(retry_receipt, first_authority.structural_receipt());
        assert_eq!(lengths(root.path()), before, "reuse must append no bytes");
        let reopened = AnchoredSearchLineageV2Ledger::open_read(root.path(), bounds())
            .expect("public structural reopen");
        assert_eq!(
            reopened
                .structural_receipt(&retry_receipt.pair_id())
                .expect("stable structural lookup"),
            Some(retry_receipt)
        );
    }

    #[test]
    fn unsupported_candidate_statistics_and_final_fields_have_no_canonical_slot() {
        let (nifty, banknifty) = projections();
        let prepared = PreparedPairV2::from_opaque(&nifty, &banknifty).expect("prepare pair");
        let member = prepared.members(0)[0];

        // Exhaustive destructuring is a compile-shape lock: adding any field
        // requires this proof to be revisited.  The only semantic fields are
        // exactly the Runner-owned identities and outcome facts below.
        let MemberRecordV2 {
            block_sequence: _,
            family: _,
            validation_policy_id: _,
            validation_family_id: _,
            walk_facts_id: _,
            source_authority_id: _,
            member_id: _,
            fold_count: _,
            decided_folds: _,
            profitable_oos_folds: _,
            aggregate_oos_paisa: _,
        } = member;
        let PreparedPairV2 {
            nifty: _,
            banknifty: _,
            pair_id: _,
        } = prepared;

        let mut raw = encode_member(&member).expect("encode NIFTY member");

        // Offset 232 starts the reserve after every fact the opaque Runner
        // projection actually owns. Candidate/considered/priced/scored counts,
        // population-search/ranking/source/finalization IDs, Statistics IDs and
        // final Admission IDs have no encoder parameter or record slot. Even a
        // correctly resealed attempted extension is rejected as noncanonical.
        raw[232] = 1;
        let seal = hash_slices(MEMBER_SEAL_DOMAIN, &[&raw[..MEMBER_PAYLOAD_BYTES]]);
        raw[MEMBER_PAYLOAD_BYTES..].copy_from_slice(&seal);
        assert_refuses(decode_member(&raw), "member reserve");
    }

    #[test]
    fn full_synced_member_tail_is_exactly_retryable_but_partial_tail_refuses() {
        let (nifty, banknifty) = projections();
        let prepared = PreparedPairV2::from_opaque(&nifty, &banknifty).expect("prepare pair");

        let full = TestRoot::new("full-tail");
        initialize(full.path());
        let mut member = OpenOptions::new()
            .read(true)
            .write(true)
            .open(full.path().join(MEMBER_FILE))
            .expect("open member tail");
        for raw in encode_member_records(&prepared.members(0)).expect("encode tail") {
            append_raw(&mut member, &raw).expect("append tail member");
        }
        member.sync_all().expect("sync full member tail");
        drop(member);
        let retry = persist_anchored_search_lineage_v2(full.path(), bounds(), &nifty, &banknifty)
            .expect("complete exact full tail");
        assert!(matches!(
            retry,
            AnchoredSearchLineageV2AuthenticatedCommit::Written(_)
        ));
        assert_eq!(
            retry.authority().structural_receipt().pair_id(),
            prepared.pair_id
        );

        let later = TestRoot::new("later-full-tail");
        persist_anchored_search_lineage_v2(later.path(), bounds(), &nifty, &banknifty)
            .expect("commit earlier pair");
        let later_nifty = projection(17, Direction::Long);
        let later_banknifty = projection(18, Direction::Short);
        let later_prepared = PreparedPairV2::from_opaque(&later_nifty, &later_banknifty)
            .expect("prepare later pair");
        let mut later_member = OpenOptions::new()
            .read(true)
            .write(true)
            .open(later.path().join(MEMBER_FILE))
            .expect("open later member tail");
        for raw in
            encode_member_records(&later_prepared.members(1)).expect("encode later member tail")
        {
            append_raw(&mut later_member, &raw).expect("append later tail member");
        }
        later_member
            .sync_all()
            .expect("sync later full member tail");
        drop(later_member);
        let later_retry = persist_anchored_search_lineage_v2(
            later.path(),
            bounds(),
            &later_nifty,
            &later_banknifty,
        )
        .expect("complete exact full tail at nonzero sequence");
        assert!(matches!(
            later_retry,
            AnchoredSearchLineageV2AuthenticatedCommit::Written(_)
        ));
        assert_eq!(
            later_retry
                .authority()
                .structural_receipt()
                .block_sequence(),
            1
        );

        let partial = TestRoot::new("partial-tail");
        initialize(partial.path());
        let mut member = OpenOptions::new()
            .read(true)
            .write(true)
            .open(partial.path().join(MEMBER_FILE))
            .expect("open partial member tail");
        append_raw(
            &mut member,
            &encode_member(&prepared.members(0)[0]).expect("encode first member"),
        )
        .expect("append one member");
        member.sync_all().expect("sync partial member tail");
        drop(member);
        assert_refuses(
            AnchoredSearchLineageV2Ledger::open_read(partial.path(), bounds()),
            "partial trailing pair",
        );
    }

    #[test]
    fn member_and_completion_corruption_fail_closed() {
        let (nifty, banknifty) = projections();
        let member_root = TestRoot::new("member-corrupt");
        persist_anchored_search_lineage_v2(member_root.path(), bounds(), &nifty, &banknifty)
            .expect("persist member attack fixture");
        overwrite_byte(&member_root.path().join(MEMBER_FILE), 80);
        assert_refuses(
            AnchoredSearchLineageV2Ledger::open_read(member_root.path(), bounds()),
            "member seal mismatch",
        );

        let completion_root = TestRoot::new("completion-corrupt");
        persist_anchored_search_lineage_v2(completion_root.path(), bounds(), &nifty, &banknifty)
            .expect("persist Completion attack fixture");
        overwrite_byte(&completion_root.path().join(COMPLETION_FILE), 80);
        assert_refuses(
            AnchoredSearchLineageV2Ledger::open_read(completion_root.path(), bounds()),
            "Completion seal mismatch",
        );
    }

    #[test]
    fn stale_handles_refuse_same_length_growth_and_path_replacement() {
        let (nifty, banknifty) = projections();

        let same_length = TestRoot::new("same-length-stale");
        let same_receipt =
            persist_anchored_search_lineage_v2(same_length.path(), bounds(), &nifty, &banknifty)
                .expect("persist same-length fixture")
                .authority()
                .structural_receipt();
        let same_reader = AnchoredSearchLineageV2Ledger::open_read(same_length.path(), bounds())
            .expect("open same-length reader");
        overwrite_byte(&same_length.path().join(MEMBER_FILE), 80);
        assert_refuses(
            same_reader.structural_receipt(&same_receipt.pair_id()),
            "member generation changed",
        );

        let growth = TestRoot::new("growth-stale");
        let growth_receipt =
            persist_anchored_search_lineage_v2(growth.path(), bounds(), &nifty, &banknifty)
                .expect("persist growth fixture")
                .authority()
                .structural_receipt();
        let growth_reader = AnchoredSearchLineageV2Ledger::open_read(growth.path(), bounds())
            .expect("open growth reader");
        let mut grown = OpenOptions::new()
            .append(true)
            .open(growth.path().join(MEMBER_FILE))
            .expect("open member file for external growth");
        grown.write_all(&[0x7f]).expect("append external byte");
        grown.sync_all().expect("sync external growth");
        drop(grown);
        assert_refuses(
            growth_reader.structural_receipt(&growth_receipt.pair_id()),
            "member generation changed",
        );

        let replaced = TestRoot::new("path-replaced");
        let replaced_receipt =
            persist_anchored_search_lineage_v2(replaced.path(), bounds(), &nifty, &banknifty)
                .expect("persist replacement fixture")
                .authority()
                .structural_receipt();
        let replaced_reader = AnchoredSearchLineageV2Ledger::open_read(replaced.path(), bounds())
            .expect("open replacement reader");
        let member_path = replaced.path().join(MEMBER_FILE);
        let displaced_path = replaced.path().join("displaced-members.bin");
        std::fs::rename(&member_path, &displaced_path).expect("displace named member file");
        std::fs::copy(&displaced_path, &member_path).expect("replace member with same bytes");
        File::open(&member_path)
            .expect("open replacement")
            .sync_all()
            .expect("sync replacement");
        assert_refuses(
            replaced_reader.structural_receipt(&replaced_receipt.pair_id()),
            "was replaced",
        );
    }

    #[test]
    fn resealed_semantic_tamper_remains_structural_and_cannot_mint_runner_authority() {
        let root = TestRoot::new("resealed-semantic-tamper");
        initialize(root.path());
        let (nifty, banknifty) = projections();
        let original = PreparedPairV2::from_opaque(&nifty, &banknifty).expect("prepare real pair");

        // Model a writer that knows every public byte rule but does not possess
        // a matching opaque Runner projection.  It can create a structurally
        // self-consistent foreign ledger row, never an authenticated authority.
        let mut forged_nifty = original.nifty;
        forged_nifty.validation_policy_id[0] ^= 1;
        forged_nifty.source_authority_id = forged_nifty.derive_source_authority_id();
        forged_nifty.member_id = forged_nifty.derive_member_id();
        let forged = PreparedPairV2 {
            nifty: forged_nifty,
            banknifty: original.banknifty,
            pair_id: derive_pair_id(forged_nifty.member_id, original.banknifty.member_id),
        };
        assert_ne!(forged.pair_id, original.pair_id);

        let mut members = OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.path().join(MEMBER_FILE))
            .expect("open forged member file");
        for raw in encode_member_records(&forged.members(0)).expect("encode forged members") {
            append_raw(&mut members, &raw).expect("append forged member");
        }
        members.sync_all().expect("sync forged members");
        drop(members);
        let mut completions = OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.path().join(COMPLETION_FILE))
            .expect("open forged Completion file");
        append_raw(
            &mut completions,
            &encode_completion(&forged.completion(0).expect("derive forged Completion"))
                .expect("encode forged Completion"),
        )
        .expect("append forged Completion");
        completions.sync_all().expect("sync forged Completion");
        drop(completions);

        let mut reopened = AnchoredSearchLineageV2Ledger::open_read(root.path(), bounds())
            .expect("structurally reopen forged pair");
        let forged_receipt = reopened
            .structural_receipt(&forged.pair_id)
            .expect("look up forged structural receipt")
            .expect("forged structural receipt is present");
        assert_refuses(
            reopened.authenticate(forged_receipt, &original),
            "not the freshly reopened indexed pair",
        );
    }

    #[test]
    fn swapped_family_order_and_foreign_retry_fail_closed() {
        let (nifty, banknifty) = projections();
        let prepared = PreparedPairV2::from_opaque(&nifty, &banknifty).expect("prepare pair");
        let root = TestRoot::new("swapped");
        initialize(root.path());
        let members = prepared.members(0);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.path().join(MEMBER_FILE))
            .expect("open members");
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
        file.sync_all().expect("sync swapped pair");
        drop(file);
        assert_refuses(
            AnchoredSearchLineageV2Ledger::open_read(root.path(), bounds()),
            "canonical NIFTY then BANKNIFTY",
        );

        let foreign = TestRoot::new("foreign-retry");
        initialize(foreign.path());
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(foreign.path().join(MEMBER_FILE))
            .expect("open foreign members");
        for raw in encode_member_records(&prepared.members(0)).expect("encode original tail") {
            append_raw(&mut file, &raw).expect("append original tail");
        }
        file.sync_all().expect("sync original tail");
        drop(file);
        let other_bank = projection(17, Direction::Short);
        assert_refuses(
            persist_anchored_search_lineage_v2(foreign.path(), bounds(), &nifty, &other_bank),
            "not exact retry",
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_root_and_child_are_refused_without_fallback() {
        use std::os::unix::fs::symlink;

        let target = TestRoot::new("symlink-target");
        initialize(target.path());
        let link_holder = TestRoot::new("symlink-holder");
        let root_link = link_holder.path().join("root-link");
        symlink(target.path(), &root_link).expect("create root symlink");
        assert_refuses(
            AnchoredSearchLineageV2Ledger::open_read(&root_link, bounds()),
            "real directory",
        );

        let child = TestRoot::new("symlink-child");
        initialize(child.path());
        std::fs::remove_file(child.path().join(MEMBER_FILE)).expect("remove member file");
        symlink(
            target.path().join(MEMBER_FILE),
            child.path().join(MEMBER_FILE),
        )
        .expect("create member symlink");
        assert_refuses(
            AnchoredSearchLineageV2Ledger::open_read(child.path(), bounds()),
            "regular non-symlink",
        );
    }
}
