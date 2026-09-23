//! Shared projection-only byte authentication and temporary publication leases.
use super::{display, known_namespace, read_held};
use brutex_core::blake3::hash;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use store::flock::Flock;

/// Authenticated saved bytes; this type grants no successor-authoring authority.
///
/// The body and receipt stay shared-locked for this value's whole life, and
/// both are released by an explicit unlock when it drops, never by closing a
/// descriptor: a duplicate left in a child another thread spawned would
/// otherwise keep the publisher out after every reader is gone (D-0693).
pub(crate) struct Observation {
    directory: PathBuf,
    /// A duplicate of the owner lock's descriptor, unlocked; each projection
    /// takes and releases its lease on it.
    owner: File,
    /// Where `owner` is, for the lease's refusal to release.
    owner_path: PathBuf,
    body: Flock<File>,
    receipt: Flock<File>,
    generations: [crate::result_set::FileGeneration; 3],
    identity: [u8; 32],
    completion: [u8; 32],
    bytes: u64,
    projecting: AtomicBool,
}
impl Observation {
    pub(crate) fn open<T>(
        root: &Path,
        namespace: &str,
        identity: [u8; 32],
        max_bytes: u64,
        decode: impl FnOnce(&[u8]) -> Result<T, String>,
    ) -> Result<(Self, T), String> {
        known_namespace(namespace)?;
        let payload_limit = max_bytes
            .checked_sub(112)
            .ok_or("Boolean read ceiling excludes receipt")?;
        let directory = root.join(namespace).join(crate::identity_hex(&identity));
        // The same file excludes the publisher through receipt/directory fsync.
        // Every refusal from here to the release below unlocks the owner
        // through the guard's explicit unlock, never by close.
        let owner_path = directory.join("owner.lock");
        let (owner_lock, owner_generation, _) = read_held(&owner_path, 0)?;
        let owner = owner_lock.try_clone().map_err(display)?;
        let (receipt, receipt_generation, raw) = read_held(&directory.join("complete.bin"), 112)?;
        if raw.len() != 112
            || field::<8>(&raw, 0)? != *b"BRBLCM01"
            || field::<32>(&raw, 8)? != identity
        {
            return Err("Boolean receipt identity or size mismatch".to_owned());
        }
        let payload = field::<32>(&raw, 40)?;
        let bytes = u64::from_le_bytes(field(&raw, 72)?);
        if field::<32>(&raw, 80)? != hash(raw.get(..80).ok_or("Boolean receipt payload missing")?)
            || bytes > payload_limit
        {
            return Err("Boolean receipt seal or byte admission refused".to_owned());
        }
        let (body, body_generation, raw_body) = read_held(&directory.join("body.bin"), bytes)?;
        if raw_body.len() as u64 != bytes || hash(&raw_body) != payload {
            return Err("Boolean body differs from receipt".to_owned());
        }
        let decoded = decode(&raw_body)?;
        let held = Self {
            directory,
            owner,
            owner_path,
            body,
            receipt,
            generations: [owner_generation, body_generation, receipt_generation],
            identity,
            completion: hash(&raw),
            bytes,
            projecting: AtomicBool::new(false),
        };
        held.check_generations()?;
        owner_lock.release().map_err(|u| display(u.why))?;
        Ok((held, decoded))
    }
    pub(crate) const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    pub(crate) const fn completion_digest(&self) -> [u8; 32] {
        self.completion
    }
    pub(crate) const fn body_bytes(&self) -> u64 {
        self.bytes
    }
    pub(crate) fn require_current(&self) -> Result<(), String> {
        self.with_current(|| Ok(()))
    }
    /// One whole projection holds the lease; an idle cached handle does not.
    /// Nested or concurrent entry on this same descriptor refuses before it
    /// can acquire or release the outer projection's operating-system lock.
    pub(crate) fn with_current<T>(
        &self,
        project: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let lease = ReadLease::acquire(self)?;
        self.check_generations()?;
        let result = project()?;
        self.check_generations()?;
        lease.lease.release().map_err(|u| display(u.why))?;
        Ok(result)
    }
    /// Maximum temporary reference/lease storage, including the caller's input
    /// reference vector. Allocator overhead is not a process RSS guarantee.
    pub(crate) fn projection_scratch_bytes(count: usize) -> Result<u64, String> {
        count
            .checked_mul(2 * std::mem::size_of::<&Self>() + std::mem::size_of::<ReadLease<'_>>())
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or_else(|| "compound observation lease scratch size overflow".into())
    }
    /// Hold one publication lease for every distinct full artifact path. A path
    /// alias must carry the same receipt and all original file generations;
    /// conflicting authorities never become an arbitrary chosen observation.
    /// This is iterative, O(n log n) ordering plus O(n) generation checks. The
    /// callback must use direct decoded getters, never nested lease operations.
    pub(crate) fn with_current_many<T>(
        observations: &[&Self],
        max_scratch_bytes: u64,
        project: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        if Self::projection_scratch_bytes(observations.len())? > max_scratch_bytes {
            return Err("compound observation leases exceed scratch admission".into());
        }
        let mut ordered = Vec::new();
        ordered
            .try_reserve_exact(observations.len())
            .map_err(display)?;
        ordered.extend_from_slice(observations);
        ordered.sort_unstable_by(|a, b| a.directory.cmp(&b.directory));
        for (left, right) in ordered.iter().zip(ordered.iter().skip(1)) {
            if left.directory == right.directory
                && (left.identity != right.identity
                    || left.completion != right.completion
                    || left.bytes != right.bytes
                    || left.generations != right.generations)
            {
                return Err(
                    "duplicate artifact path carries conflicting receipt or generations".into(),
                );
            }
        }
        ordered.dedup_by(|left, right| left.directory == right.directory);
        let mut leases = Vec::new();
        leases.try_reserve_exact(ordered.len()).map_err(display)?;
        for observation in ordered {
            leases.push(ReadLease::acquire(observation)?);
        }
        // Recheck aliases as well: their separately opened descriptors must
        // still name the same immutable files while all owners are excluded.
        for observation in observations {
            observation.check_generations()?;
        }
        let result = project()?;
        for observation in observations {
            observation.check_generations()?;
        }
        for lease in leases {
            lease.lease.release().map_err(|u| display(u.why))?;
        }
        Ok(result)
    }
    fn check_generations(&self) -> Result<(), String> {
        for ((file, name), generation) in [
            (&self.owner, "owner.lock"),
            (&*self.body, "body.bin"),
            (&*self.receipt, "complete.bin"),
        ]
        .into_iter()
        .zip(self.generations)
        {
            let path = self.directory.join(name);
            crate::result_set::require_generation_unchanged(
                generation,
                crate::result_set::file_generation(file, &path)?,
                &path,
            )?;
        }
        Ok(())
    }
}

fn field<const N: usize>(raw: &[u8], at: usize) -> Result<[u8; N], String> {
    raw.get(at..at.checked_add(N).ok_or("Boolean receipt field overflow")?)
        .ok_or("Boolean receipt field missing")?
        .try_into()
        .map_err(display)
}

/// One projection's shared lease on the owner lock.
///
/// THE FIELD ORDER IS LOAD-BEARING. Fields drop in declaration order, so the
/// lease is unlocked before the nesting flag clears: a second projection on
/// the same descriptor can never take a lease this one is about to unlock.
/// The unlock is explicit on every path — the guard's own `release`, called
/// by name where a projection succeeds, and its `Drop` on every refusal — and
/// a refused one is reported, never discarded (D-0693). The success path
/// moves the guard out and releases it; the nesting flag clears when the rest
/// of the lease drops, after that. There is no `release` wrapper here: its
/// only body would be the guard's release, and replacing that with `Ok(())`
/// would still unlock through `Drop`, which no test can tell apart.
struct ReadLease<'a> {
    lease: Flock<&'a File, &'a Path>,
    _projecting: Projecting<'a>,
}
/// The observation's nesting flag, cleared when this drops.
struct Projecting<'a>(&'a AtomicBool);
impl Drop for Projecting<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
impl<'a> ReadLease<'a> {
    fn acquire(observation: &'a Observation) -> Result<Self, String> {
        observation
            .projecting
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(
                |_| "nested or concurrent observation projection on the same descriptor is refused",
            )?;
        let projecting = Projecting(&observation.projecting);
        let lease = Flock::try_lock_shared(&observation.owner, observation.owner_path.as_path())
            .map_err(|why| format!("Boolean publication is busy or cannot be locked: {why}"))?;
        Ok(Self {
            lease,
            _projecting: projecting,
        })
    }
}

#[cfg(test)]
#[path = "boolean_observation_file_tests.rs"]
mod tests;
