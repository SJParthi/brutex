//! Bounded overview: acknowledged snapshot plus fixed child receipts only.
use super::{Link, MAX_CHECKPOINTS, MAX_SNAPSHOT, NAMESPACE, Rung, Snapshot, State, Status, codec};
use std::io::Read as _;
use std::path::{Path, PathBuf};

/// Saved campaign overview. Child bodies and current raw files are not opened.
pub struct Reader {
    root: PathBuf,
    pub(super) state: State,
    sequence: u64,
    pin: [u8; 32],
    owner_active: bool,
    max_bytes: u64,
}
impl Reader {
    /// Exact selected intraday scope; physical row indices remain unchanged.
    #[must_use]
    pub const fn rungs(&self) -> super::RungScope {
        self.state.rungs
    }
    /// Read the exact campaign's latest acknowledged snapshot and child receipts.
    /// # Errors
    /// Missing, corrupt, foreign, busy child receipt or exhausted byte budget.
    pub fn open(root: &Path, identity: [u8; 32], max_bytes: u64) -> Result<Self, String> {
        let snapshot = Snapshot::open(root, NAMESPACE, identity)?.ok_or("campaign not found")?;
        let sequence = snapshot
            .latest
            .ok_or("campaign has no acknowledged snapshot")?;
        if sequence >= MAX_CHECKPOINTS || snapshot.interrupted >= MAX_CHECKPOINTS {
            return Err("campaign history exceeds bound".into());
        }
        let saved = snapshot.read(sequence, max_bytes.min(MAX_SNAPSHOT))?;
        let state = codec::decode(&saved.payload)?;
        if state.identity != identity || state.previous.is_some_and(|p| p.0 >= sequence) {
            return Err("campaign identity/predecessor differs".into());
        }
        let remaining = max_bytes
            .checked_sub(saved.payload.len() as u64 + 96)
            .ok_or("campaign snapshot exceeds byte admission")?;
        check_receipts(root, &state, remaining)?;
        Ok(Self {
            root: root.to_path_buf(),
            state,
            sequence,
            pin: saved.seal,
            owner_active: snapshot.writer_observed,
            max_bytes,
        })
    }
    /// Exact full campaign identity.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.state.identity
    }
    /// Required latest snapshot pin.
    #[must_use]
    pub const fn pin(&self) -> [u8; 32] {
        self.pin
    }
    /// Immutable checkpoint sequence.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Conflicting campaign owner observed at the latest successful read.
    #[must_use]
    pub const fn owner_active(&self) -> bool {
        self.owner_active
    }
    /// Recorded campaign state, not process liveness.
    #[must_use]
    pub const fn status(&self) -> Status {
        self.state.status
    }
    /// All eight canonical timeframe rows.
    #[must_use]
    pub fn rows(&self) -> &[Rung] {
        &self.state.rows
    }
    /// Inclusive requested first month.
    #[must_use]
    pub const fn from(&self) -> (u16, u8) {
        self.state.from
    }
    /// Inclusive requested final month.
    #[must_use]
    pub const fn to(&self) -> (u16, u8) {
        self.state.to
    }
    /// Exact one-minute execution horizon.
    #[must_use]
    pub const fn horizon(&self) -> u32 {
        self.state.horizon
    }
    /// Number of supplied fixed-wire programs.
    #[must_use]
    pub const fn program_count(&self) -> u64 {
        self.state.program_count
    }
    /// Exact ordered program digest.
    #[must_use]
    pub const fn program_digest(&self) -> [u8; 32] {
        self.state.programs
    }
    /// Shared source and policy preparation identity.
    #[must_use]
    pub const fn descriptor_digest(&self) -> [u8; 32] {
        self.state.descriptor
    }
    /// Recheck this exact latest snapshot and every recorded completion receipt.
    /// # Errors
    /// Refuses changed latest snapshot or changed/missing child receipts.
    pub fn require_current(&self) -> Result<(), String> {
        let fresh = Self::open(&self.root, self.identity(), self.max_bytes)?;
        if fresh.sequence != self.sequence || fresh.pin != self.pin {
            return Err("campaign snapshot changed; refresh explicitly".into());
        }
        Ok(())
    }
}
fn check_receipts(root: &Path, state: &State, mut remaining: u64) -> Result<(), String> {
    for row in &state.rows {
        for catalog in &row.catalogs {
            if let Some(completion) = catalog.completion {
                check_receipt(
                    root,
                    "boolean-candidates-v1",
                    Link {
                        identity: catalog.expected,
                        completion,
                    },
                    &mut remaining,
                )?;
            }
        }
        for (namespace, link) in [
            ("boolean-statistics-v1", row.statistics),
            ("boolean-admission-v1", row.admission),
        ] {
            if let Some(link) = link {
                check_receipt(root, namespace, link, &mut remaining)?;
            }
        }
    }
    Ok(())
}
fn check_receipt(
    root: &Path,
    namespace: &str,
    link: Link,
    remaining: &mut u64,
) -> Result<(), String> {
    *remaining = remaining
        .checked_sub(112)
        .ok_or("campaign child receipt byte budget exhausted")?;
    let directory = root
        .join(namespace)
        .join(crate::identity_hex(&link.identity));
    let owner_path = directory.join("owner.lock");
    let owner = crate::readonly_file::open(&owner_path).map_err(|why| why.to_string())?;
    let owner_before = crate::result_set::file_generation(&owner, &owner_path)?;
    if owner.metadata().map_err(|why| why.to_string())?.len() != 0 {
        return Err("campaign child owner is not empty".into());
    }
    owner
        .try_lock_shared()
        .map_err(|why| format!("campaign child publication is busy: {why}"))?;
    let path = directory.join("complete.bin");
    let mut file = crate::readonly_file::open(&path).map_err(|why| why.to_string())?;
    let before = crate::result_set::file_generation(&file, &path)?;
    if file.metadata().map_err(|why| why.to_string())?.len() != 112 {
        return Err("campaign child receipt width differs".into());
    }
    let mut raw = [0_u8; 112];
    file.read_exact(&mut raw).map_err(|why| why.to_string())?;
    if raw.get(..8) != Some(b"BRBLCM01")
        || raw.get(8..40) != Some(link.identity.as_slice())
        || raw.get(80..112)
            != Some(brutex_core::blake3::hash(raw.get(..80).ok_or("receipt prefix")?).as_slice())
        || brutex_core::blake3::hash(&raw) != link.completion
    {
        return Err("campaign child completion receipt differs".into());
    }
    crate::result_set::require_generation_unchanged(
        before,
        crate::result_set::file_generation(&file, &path)?,
        &path,
    )?;
    crate::result_set::require_generation_unchanged(
        owner_before,
        crate::result_set::file_generation(&owner, &owner_path)?,
        &owner_path,
    )?;
    owner.unlock().map_err(|why| why.to_string())
}
