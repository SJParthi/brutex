//! Shared immutable checkpoints for the AND and Boolean search callers.
//! A directory reservation precedes payload creation. Only a separately synced
//! completion marker makes its fully verified payload resumable. Cold discovery
//! scans bounded directory entries; publishing does not rescan history.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use store::flock::Flock;

const MAGIC: &[u8; 8] = b"BTXCHK01";
const HEADER: usize = 64;
pub(crate) const DIRECTORY_LIMIT: usize = 1_000_000;

/// The namespace entry ceiling discovery admits: `DIRECTORY_LIMIT`, except
/// that a test on its own thread may lower it so the limit is reachable with
/// real directories rather than a million of them.
fn directory_limit() -> usize {
    #[cfg(test)]
    if let Some(limit) = tests::LIMIT.with(std::cell::Cell::get) {
        return limit;
    }
    DIRECTORY_LIMIT
}

/// An exclusively owned search checkpoint namespace.
///
/// Every advisory lock this type takes is released by an explicit unlock,
/// never by closing its descriptor: a descriptor duplicated into a child
/// another thread spawned would otherwise keep it, and the next shared read of
/// the same file would refuse as busy (D-0693).
pub(crate) struct Journal {
    directory: PathBuf,
    identity: [u8; 32],
    /// The namespace's exclusive owner lock, released when the journal drops.
    owner: Flock<File>,
    next: u64,
    latest: Option<u64>,
    interrupted: u64,
    acknowledged: u64,
    /// Namespace entries (owner.lock plus every reservation) counted by the
    /// opening discovery and advanced by each reservation this writer makes,
    /// so a publication can refuse before discovery would.
    entries: usize,
    poisoned: bool,
}

/// One fully verified checkpoint; payload format belongs to its versioned caller.
pub(crate) struct Saved {
    pub sequence: u64,
    pub payload: Vec<u8>,
    pub seal: [u8; 32],
}

/// A read-only observed checkpoint namespace; it never owns or writes the search.
pub(crate) struct Snapshot {
    directory: PathBuf,
    identity: [u8; 32],
    pub latest: Option<u64>,
    pub interrupted: u64,
    /// Complete markers observed by the same bounded discovery; holes are separate.
    pub acknowledged: u64,
    /// A conflicting owner lock was observed during this snapshot only.
    pub writer_observed: bool,
}
impl Snapshot {
    pub(crate) fn open(
        root: &Path,
        format: &str,
        identity: [u8; 32],
    ) -> Result<Option<Self>, String> {
        Self::open_through(root, format, identity, None)
    }
    /// Observe only completion markers at or before a pinned immutable
    /// generation. Newer appends cannot enlarge this historical read scope.
    pub(crate) fn open_prefix(
        root: &Path,
        format: &str,
        identity: [u8; 32],
        through: u64,
    ) -> Result<Option<Self>, String> {
        if through == 0 {
            return Err("checkpoint prefix sequence must be positive".into());
        }
        Self::open_through(root, format, identity, Some(through))
    }
    fn open_through(
        root: &Path,
        format: &str,
        identity: [u8; 32],
        through: Option<u64>,
    ) -> Result<Option<Self>, String> {
        if !matches!(
            format,
            "and-checkpoint-v1"
                | "and-checkpoint-v2"
                | "expression-search-v1"
                | "boolean-campaign-v1"
                | "boolean-grammar-v1"
                | "boolean-qualified-campaign-v1"
                | "boolean-qualified-search-v1"
                | "index-stop-search-v1"
        ) {
            return Err("unknown checkpoint format namespace".to_owned());
        }
        let directory = root.join(format).join(hex(&identity));
        match fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(_) => return Err("checkpoint namespace is not a directory".to_owned()),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(why) => return Err(error(why)),
        }
        let owner_path = directory.join("owner.lock");
        if !fs::symlink_metadata(&owner_path)
            .map_err(error)?
            .file_type()
            .is_file()
        {
            return Err("checkpoint owner is not a regular file".to_owned());
        }
        let owner = crate::readonly_file::open(&owner_path).map_err(error)?;
        let before = crate::result_set::file_generation(&owner, &owner_path)?;
        let writer_observed = match owner.try_lock_shared() {
            Ok(()) => {
                owner.unlock().map_err(error)?;
                false
            }
            Err(std::fs::TryLockError::WouldBlock) => true,
            Err(std::fs::TryLockError::Error(why)) => return Err(error(why)),
        };
        let after = crate::result_set::file_generation(&owner, &owner_path)?;
        crate::result_set::require_generation_unchanged(before, after, &owner_path)?;
        let (_, latest, interrupted, acknowledged, _) = discover_through(&directory, through)?;
        Ok(Some(Self {
            directory,
            identity,
            latest,
            interrupted,
            acknowledged,
            writer_observed,
        }))
    }
    pub(crate) fn read(&self, sequence: u64, max_bytes: u64) -> Result<Saved, String> {
        read_saved(&self.directory, self.identity, sequence, max_bytes)
    }
}

impl Journal {
    pub(crate) fn open(root: &Path, format: &str, identity: [u8; 32]) -> Result<Self, String> {
        if !matches!(
            format,
            "and-checkpoint-v1"
                | "and-checkpoint-v2"
                | "expression-search-v1"
                | "boolean-campaign-v1"
                | "boolean-grammar-v1"
                | "boolean-qualified-campaign-v1"
                | "boolean-qualified-search-v1"
                | "index-stop-search-v1"
        ) {
            return Err("unknown checkpoint format namespace".to_owned());
        }
        let base = root.join(format);
        durable_directory(root, &base)?;
        let directory = base.join(hex(&identity));
        durable_directory(&base, &directory)?;
        let owner_path = directory.join("owner.lock");
        let owner =
            Flock::try_lock(open_owner(&owner_path)?, owner_path.clone()).map_err(|why| {
                format!("this exact search is already owned or cannot be locked: {why}")
            })?;
        File::open(&directory)
            .map_err(error)?
            .sync_all()
            .map_err(error)?;
        let (next, latest, interrupted, acknowledged, entries) = discover(&directory)?;
        Ok(Self {
            directory,
            identity,
            owner,
            next,
            latest,
            interrupted,
            acknowledged,
            entries,
            poisoned: false,
        })
    }

    pub(crate) const fn interrupted(&self) -> u64 {
        self.interrupted
    }

    pub(crate) const fn next_sequence(&self) -> u64 {
        self.next
    }

    /// Number of completion markers admitted by the existing cold discovery,
    /// incremented only after an acknowledged publication. Reserved holes do
    /// not count, and consumers can require their retained chain to cover all.
    pub(crate) const fn acknowledged(&self) -> u64 {
        self.acknowledged
    }

    pub(crate) fn latest(&self, max_bytes: u64) -> Result<Option<Saved>, String> {
        self.latest
            .map(|sequence| self.read(sequence, max_bytes))
            .transpose()
    }

    pub(crate) fn read(&self, sequence: u64, max_bytes: u64) -> Result<Saved, String> {
        read_saved(&self.directory, self.identity, sequence, max_bytes)
    }

    pub(crate) fn publish(
        &mut self,
        payload: &[u8],
        max_bytes: u64,
    ) -> Result<(u64, [u8; 32]), String> {
        if self.poisoned {
            return Err(
                "checkpoint writer remains refused after a prior publication failure".to_owned(),
            );
        }
        let result = self.publish_inner(payload, max_bytes);
        self.poisoned = result.is_err();
        result
    }

    fn publish_inner(&mut self, payload: &[u8], max_bytes: u64) -> Result<(u64, [u8; 32]), String> {
        let acknowledged = self
            .acknowledged
            .checked_add(1)
            .ok_or("checkpoint acknowledgment counter exhausted")?;
        crate::result_set::file_generation(&self.owner, &self.directory.join("owner.lock"))?;
        let length = u64::try_from(payload.len()).map_err(error)?;
        if length.checked_add(96).is_none_or(|n| n > max_bytes) {
            return Err("checkpoint publication exceeds its byte admission".to_owned());
        }
        // Discovery admits at most `directory_limit()` entries. A reservation
        // that would make one more is refused here, before it exists, so no
        // acknowledged checkpoint can leave the namespace unreopenable (D-1740).
        let entries = self
            .entries
            .checked_add(1)
            .filter(|entries| *entries <= directory_limit())
            .ok_or("checkpoint namespace reached its directory admission limit")?;
        let sequence = self.next;
        self.next = self
            .next
            .checked_add(1)
            .ok_or("checkpoint sequence exhausted")?;
        let directory = self.directory.join(format!("{sequence:016x}"));
        fs::create_dir(&directory).map_err(error)?;
        self.entries = entries;
        File::open(&self.directory)
            .map_err(error)?
            .sync_all()
            .map_err(error)?;
        let path = directory.join("payload");
        let mut raw = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(error)?;
        let mut file = Flock::lock(&mut raw, path.as_path()).map_err(error)?;
        #[cfg(test)]
        tests::payload_locked(&file);
        let header = header_of(self.identity, sequence, length);
        let mut hash = brutex_core::blake3::Hasher::new();
        hash.update(&header);
        hash.update(payload);
        let seal = hash.finalize();
        file.write_all(&header)
            .and_then(|()| file.write_all(payload))
            .and_then(|()| file.write_all(&seal))
            .and_then(|()| file.sync_all())
            .map_err(error)?;
        verify_acknowledged(&mut file, &path, &header, payload, seal)?;
        File::open(&directory)
            .map_err(error)?
            .sync_all()
            .map_err(error)?;
        // RELEASED BEFORE THE MARKER EXISTS (locks-3, D-1913). The payload is
        // whole, verified and durable here, and the marker is what makes it
        // discoverable. Held until after the marker, every reader that found
        // the new sequence in that window was refused "checkpoint payload is
        // busy" for bytes already published, through two fsyncs and a full
        // re-read. Released by name, not by closing: a duplicate a spawned
        // child still carried kept a closed descriptor's lock alive (D-0693).
        // A refused release leaves `latest` and `acknowledged` unadvanced and
        // poisons the writer through `publish`.
        file.release().map_err(|u| u.to_string())?;
        publish_marker(&directory, seal)?;
        #[cfg(test)]
        tests::marker_visible();
        verify_acknowledged(&mut raw, &path, &header, payload, seal)?;
        if regular_bytes(&directory.join("complete"), 32)? != seal {
            return Err("checkpoint marker changed before acknowledgment".to_owned());
        }
        // Only this acknowledged publication updates the cached latest position.
        // A later reopen validates the marker and all bytes again.
        self.latest = Some(sequence);
        self.acknowledged = acknowledged;
        Ok((sequence, seal))
    }
}

/// Make `complete` appear whole or not at all (D-1740). The seal is written
/// and synced under a temporary name and only then renamed into place, then
/// the reservation directory is synced. A kill before the rename leaves at
/// most `complete.tmp`, which discovery never reads, so the reservation is an
/// interrupted one and the previous checkpoint stays the resume point. A
/// failure this process sees removes the temporary file and refuses.
fn publish_marker(directory: &Path, seal: [u8; 32]) -> Result<(), String> {
    let temporary = directory.join("complete.tmp");
    let written = (|| {
        let mut marker = File::create_new(&temporary)?;
        #[cfg(test)]
        tests::marker_created(directory.parent().unwrap_or(directory));
        marker.write_all(&seal)?;
        marker.sync_all()?;
        #[cfg(test)]
        tests::marker_staged(&temporary)?;
        fs::rename(&temporary, directory.join("complete"))
    })();
    if let Err(why) = written {
        return Err(match fs::remove_file(&temporary) {
            Ok(()) => format!("checkpoint marker was not published: {why}"),
            Err(left) if left.kind() == std::io::ErrorKind::NotFound => {
                format!("checkpoint marker was not published: {why}")
            }
            Err(left) => format!(
                "checkpoint marker was not published: {why}; its temporary file also could not be removed: {left}"
            ),
        });
    }
    File::open(directory)
        .map_err(error)?
        .sync_all()
        .map_err(error)
}

fn verify_acknowledged(
    file: &mut File,
    path: &Path,
    header: &[u8; HEADER],
    payload: &[u8],
    seal: [u8; 32],
) -> Result<(), String> {
    let before = crate::result_set::file_generation(file, path)?;
    file.seek(SeekFrom::Start(0)).map_err(error)?;
    let mut actual = [0; HEADER];
    file.read_exact(&mut actual).map_err(error)?;
    if &actual != header {
        return Err("acknowledged checkpoint header changed".to_owned());
    }
    let mut buffer = [0; 8192];
    for chunk in payload.chunks(8192) {
        let target = buffer
            .get_mut(..chunk.len())
            .ok_or("checkpoint verification buffer mismatch")?;
        file.read_exact(target).map_err(error)?;
        if target != chunk {
            return Err("acknowledged checkpoint payload changed".to_owned());
        }
    }
    let mut tail = [0; 32];
    file.read_exact(&mut tail).map_err(error)?;
    if tail != seal
        || file.metadata().map_err(error)?.len()
            != u64::try_from(HEADER + payload.len() + 32).map_err(error)?
    {
        return Err("acknowledged checkpoint tail changed".to_owned());
    }
    let after = crate::result_set::file_generation(file, path)?;
    crate::result_set::require_generation_unchanged(before, after, path)
}

fn open_owner(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(store::open_flags::O_NOFOLLOW);
    }
    // create_new never follows an existing symlink, including a dangling one.
    // The existing-file door deliberately has no create flag, so refusal can
    // never create a symlink target between metadata admission and open.
    match options.create_new(true).open(path) {
        Ok(file) => Ok(file),
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
            if !fs::symlink_metadata(path)
                .map_err(error)?
                .file_type()
                .is_file()
            {
                return Err("checkpoint owner is not a regular file".to_owned());
            }
            let file = options.create_new(false).open(path).map_err(error)?;
            crate::result_set::file_generation(&file, path)?;
            Ok(file)
        }
        Err(why) => Err(error(why)),
    }
}

fn regular_bytes(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(error)?;
    if !metadata.file_type().is_file() || metadata.len() > limit {
        return Err("checkpoint marker type or size refused".to_owned());
    }
    let mut file = crate::readonly_file::open(path).map_err(error)?;
    let before = crate::result_set::file_generation(&file, path)?;
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if u64::try_from(bytes.len()).map_err(error)? != metadata.len() {
        return Err("checkpoint marker length changed".to_owned());
    }
    let after = crate::result_set::file_generation(&file, path)?;
    crate::result_set::require_generation_unchanged(before, after, path)?;
    Ok(bytes)
}

fn durable_directory(parent: &Path, path: &Path) -> Result<(), String> {
    match fs::create_dir(path) {
        Ok(()) => File::open(parent).map_err(error)?.sync_all().map_err(error),
        Err(why)
            if why.kind() == std::io::ErrorKind::AlreadyExists
                && fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_dir()) =>
        {
            Ok(())
        }
        Err(why) => Err(error(why)),
    }
}

fn header_of(identity: [u8; 32], sequence: u64, length: u64) -> [u8; HEADER] {
    let mut bytes = [0; HEADER];
    for (slot, byte) in bytes.iter_mut().zip(
        MAGIC
            .iter()
            .copied()
            .chain(identity)
            .chain(sequence.to_le_bytes())
            .chain(length.to_le_bytes()),
    ) {
        *slot = byte;
    }
    bytes
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}
fn error(why: impl std::fmt::Display) -> String {
    why.to_string()
}

#[cfg(test)]
#[path = "search_checkpoint_tests.rs"]
pub(crate) mod tests;

/// Finder's `.DS_Store` and the `._<name>` `AppleDouble` files a copy to a
/// non-HFS volume writes: operating-system litter, never a reservation.
fn is_os_litter(name: &str) -> bool {
    name == ".DS_Store" || name.starts_with("._")
}

/// `(next, latest, interrupted, acknowledged, entries)`.
type Discovered = (u64, Option<u64>, u64, u64, usize);

fn discover(directory: &Path) -> Result<Discovered, String> {
    discover_through(directory, None)
}
fn discover_through(directory: &Path, through: Option<u64>) -> Result<Discovered, String> {
    let mut entries = 0_usize;
    let mut next = 1;
    let mut latest = None;
    let mut interrupted = 0_u64;
    let mut acknowledged = 0_u64;
    for (index, entry) in fs::read_dir(directory).map_err(error)?.enumerate() {
        if index == directory_limit() {
            return Err("checkpoint directory admission limit exceeded".to_owned());
        }
        entries = index + 1;
        let entry = entry.map_err(error)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "nontext checkpoint filename".to_owned())?;
        if name == "owner.lock" {
            continue;
        }
        // MACOS LITTER IS NOT A RESERVATION, AND A STRANGER IS NAMED. Finder
        // writes `.DS_Store` into a folder it opens, and a copy to a non-HFS
        // volume writes `._<name>` beside each file; either one refused every
        // start, resume and dashboard read of this search with a sentence that
        // named no file (CE-34, D-1769). Those two, as plain files, are passed
        // over; anything else still refuses, now by its name.
        if is_os_litter(&name) && entry.file_type().map_err(error)?.is_file() {
            continue;
        }
        let sequence = u64::from_str_radix(&name, 16).map_err(|_| {
            format!(
                "invalid checkpoint reservation name {name:?} in {}",
                directory.display()
            )
        })?;
        if sequence == 0
            || name != format!("{sequence:016x}")
            || !entry.file_type().map_err(error)?.is_dir()
        {
            return Err(format!(
                "invalid checkpoint reservation type or sequence {name:?} in {}",
                directory.display()
            ));
        }
        next = next.max(
            sequence
                .checked_add(1)
                .ok_or("checkpoint sequence exhausted")?,
        );
        if through.is_some_and(|ceiling| sequence > ceiling) {
            continue;
        }
        match fs::symlink_metadata(entry.path().join("complete")) {
            // A SHORT MARKER IS A TORN CREATE, not an acknowledgment (CE-3,
            // D-1909): the empty marker the pre-D-1740 protocol left when it was
            // killed between creating `complete` and writing its seal, or any
            // width short of 32 bytes. Counted as interrupted, so `latest` is
            // the newest WHOLE checkpoint. A whole-width marker that disagrees
            // with its payload is still refused by `read`.
            Ok(metadata) if metadata.file_type().is_file() && metadata.len() < 32 => {
                interrupted = interrupted
                    .checked_add(1)
                    .ok_or("checkpoint interruption counter exhausted")?;
            }
            Ok(metadata) if metadata.file_type().is_file() => {
                acknowledged = acknowledged
                    .checked_add(1)
                    .ok_or("checkpoint acknowledgment counter exhausted")?;
                latest = Some(latest.map_or(sequence, |old: u64| old.max(sequence)));
            }
            Ok(_) => {
                return Err("checkpoint completion marker is not a regular file".to_owned());
            }
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                interrupted = interrupted
                    .checked_add(1)
                    .ok_or("checkpoint interruption counter exhausted")?;
            }
            Err(why) => return Err(error(why)),
        }
    }

    Ok((next, latest, interrupted, acknowledged, entries))
}
fn read_saved(
    base: &Path,
    identity: [u8; 32],
    sequence: u64,
    max_bytes: u64,
) -> Result<Saved, String> {
    let directory = base.join(format!("{sequence:016x}"));
    let marker = directory.join("complete");
    let expected: [u8; 32] = regular_bytes(&marker, 32)?
        .try_into()
        .map_err(|_| "checkpoint marker width mismatch".to_owned())?;
    let path = directory.join("payload");
    let metadata = fs::symlink_metadata(&path).map_err(error)?;
    if !metadata.file_type().is_file() || metadata.len() > max_bytes {
        return Err("checkpoint payload exceeds its type or byte admission".to_owned());
    }
    let mut file = Flock::try_lock_shared(
        crate::readonly_file::open(&path).map_err(error)?,
        path.as_path(),
    )
    .map_err(|why| format!("checkpoint payload is busy or cannot be read: {why}"))?;
    let before = crate::result_set::file_generation(&file, &path)?;
    let mut header = [0; HEADER];
    file.read_exact(&mut header).map_err(error)?;
    let length = header
        .get(48..56)
        .and_then(|s| <[u8; 8]>::try_from(s).ok())
        .map(u64::from_le_bytes)
        .ok_or("checkpoint header length missing")?;
    if header != header_of(identity, sequence, length)
        || length.checked_add(96) != Some(metadata.len())
    {
        return Err("checkpoint identity, sequence or exact length mismatch".to_owned());
    }
    let count = usize::try_from(length).map_err(error)?;
    let mut payload = Vec::new();
    payload.try_reserve_exact(count).map_err(error)?;
    payload.resize(count, 0);
    file.read_exact(&mut payload).map_err(error)?;
    let mut sealed = [0; 32];
    file.read_exact(&mut sealed).map_err(error)?;
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(&header);
    hash.update(&payload);
    if hash.finalize() != sealed || sealed != expected {
        return Err("checkpoint seal mismatch".to_owned());
    }
    let after = crate::result_set::file_generation(&file, &path)?;
    crate::result_set::require_generation_unchanged(before, after, &path)?;
    if regular_bytes(&marker, 32)? != expected {
        return Err("checkpoint completion marker changed".to_owned());
    }
    file.release().map_err(|u| u.to_string())?;
    Ok(Saved {
        sequence,
        payload,
        seal: sealed,
    })
}
