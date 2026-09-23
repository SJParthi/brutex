//! Shared projection-only byte authentication and temporary publication leases.
use super::{display, known_namespace, read_held};
use brutex_core::blake3::hash;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Authenticated saved bytes; this type grants no successor-authoring authority.
pub(crate) struct Observation {
    directory: PathBuf,
    owner: File,
    body: File,
    receipt: File,
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
        let (owner, owner_generation, _) = read_held(&directory.join("owner.lock"), 0)?;
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
            body,
            receipt,
            generations: [owner_generation, body_generation, receipt_generation],
            identity,
            completion: hash(&raw),
            bytes,
            projecting: AtomicBool::new(false),
        };
        held.check_generations()?;
        held.owner.unlock().map_err(display)?;
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
        lease.release()?;
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
            lease.release()?;
        }
        Ok(result)
    }
    fn check_generations(&self) -> Result<(), String> {
        for ((file, name), generation) in [
            (&self.owner, "owner.lock"),
            (&self.body, "body.bin"),
            (&self.receipt, "complete.bin"),
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

struct ReadLease<'a> {
    owner: &'a File,
    projecting: &'a AtomicBool,
    held: bool,
}
impl<'a> ReadLease<'a> {
    fn acquire(observation: &'a Observation) -> Result<Self, String> {
        observation
            .projecting
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(
                |_| "nested or concurrent observation projection on the same descriptor is refused",
            )?;
        if let Err(why) = observation.owner.try_lock_shared() {
            observation.projecting.store(false, Ordering::Release);
            return Err(format!(
                "Boolean publication is busy or cannot be locked: {why}"
            ));
        }
        Ok(Self {
            owner: &observation.owner,
            projecting: &observation.projecting,
            held: true,
        })
    }
    fn release(mut self) -> Result<(), String> {
        self.owner.unlock().map_err(display)?;
        self.held = false;
        self.projecting.store(false, Ordering::Release);
        Ok(())
    }
}
impl Drop for ReadLease<'_> {
    fn drop(&mut self) {
        if self.held {
            let _ = self.owner.unlock();
            self.projecting.store(false, Ordering::Release);
        }
    }
}

#[cfg(test)]
#[path = "boolean_observation_file_tests.rs"]
mod tests;
