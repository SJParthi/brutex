//! The tail of a fixed-stride ledger: how an append that failed is undone, and
//! how the next writer treats bytes a killed append left past the last whole
//! record (D-1900, D-1901, D-1902).
//!
//! # One rule, in one place
//!
//! Every fixed-stride ledger in this crate used to carry its own copy of the
//! rollback, and roughly half carried none: a `write_all` that failed part way
//! through a record (ENOSPC, EDQUOT, EIO) left a ragged tail, and every later
//! open, read-only included, refused the whole ledger over bytes that were
//! never a record. The copies that did exist disagreed on whether a failed
//! durability barrier rolls back. This module is the one answer.
//!
//! - [`append`] / [`append_with`]: one write at the end of the file. A write
//!   error truncates the file back to the length it had before the write, and
//!   the truncation is itself made durable.
//! - [`start`] then [`write_at_end`]... then [`sync_or_roll_back`]: a block of
//!   several writes made durable by one barrier. Any failure, the barrier's
//!   included, truncates back to the length before the block's first write.
//! - [`heal_torn_tail`]: a WRITER, under its exclusive lock, cuts bytes past
//!   the last whole record and says so with an event and a returned note.
//!   Readers never call it; they keep refusing.
//!
//! # Why a failed barrier rolls back now
//!
//! `results.rs` argued the opposite: a complete record whose `sync_all` failed
//! "will very likely reach the platter", so discarding it was the larger harm.
//! On Linux that is false. A writeback error is reported to one `fsync` per
//! open file description and the failed pages are then marked CLEAN while
//! their contents stay readable in the page cache, so a second `fsync` returns
//! `Ok` without writing them. Keeping the bytes meant a later lookup in the
//! same boot found them, a second barrier "confirmed" them, and a durable
//! marker was then written over bytes the device never held (resources-1,
//! store1-2, pop1-4). The bytes are therefore cut, under the same lock that
//! wrote them, before anyone can read them, and the path is remembered as
//! having failed a barrier in this process: [`refuse_after_failed_barrier`]
//! lets a caller refuse to vouch for it again.
//!
//! # When a torn tail may be cut
//!
//! A record is acknowledged only after its whole stride was written and
//! synced, so a SUB-record tail cannot be any acknowledged record: whatever
//! put it there was interrupted before its own barrier returned. It is cut
//! only by a writer holding the ledger's exclusive lock, because an unlocked
//! measurement can land inside a live append (cli2-3) and cutting that would
//! destroy a write in progress. A whole record that fails its seal is NOT a
//! torn tail and is never cut here: it may be damaged history.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{Seek as _, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

/// Paths whose durability barrier failed in this process (rule 3 of D-1900).
static FAILED_BARRIERS: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

/// What [`heal_torn_tail`] cut, for the caller to report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TornTail {
    /// The length kept: the header plus every whole record.
    pub(crate) kept: u64,
    /// The length found, which ended part way through a record.
    pub(crate) found: u64,
}

/// A header's first twelve bytes: the eight-byte magic and the little-endian
/// version. [`heal_torn_tail`] cuts only a file that carries exactly these,
/// so a legacy version with another stride is never cut against this one.
pub(crate) fn magic_and_version(magic: [u8; 8], version: u32) -> [u8; 12] {
    let mut leading = [0_u8; 12];
    leading[..8].copy_from_slice(&magic);
    leading[8..].copy_from_slice(&version.to_le_bytes());
    leading
}

/// What a refusal names: a path's display, or a ledger's own label.
pub(crate) type Subject<'a> = &'a dyn std::fmt::Display;

/// Truncates `file` back to `end`, makes the cut durable, and composes the
/// refusal for `failure`. If the rollback itself fails, both failures are
/// named, because the tail is then still there.
pub(crate) fn roll_back(file: &File, subject: Subject<'_>, end: u64, failure: &str) -> String {
    match file.set_len(end).and_then(|()| file.sync_all()) {
        Ok(()) => format!(
            "{failure}; {subject} was truncated back to {end} bytes, so it still ends on a whole record"
        ),
        Err(rollback) => format!(
            "{failure}; rolling {subject} back to {end} bytes ALSO failed: {rollback}. The file may end mid-record until a writer cuts it back"
        ),
    }
}

/// The current end of `file`, where the next append will land.
pub(crate) fn start(file: &mut File, subject: Subject<'_>) -> Result<u64, String> {
    file.seek(SeekFrom::End(0))
        .map_err(|why| format!("cannot seek to the end of {subject}: {why}"))
}

/// Writes `bytes` at the end through `write`; on an error, rolls the file back
/// to `block_start` (the length before the first write of this block).
pub(crate) fn write_at_end(
    file: &mut File,
    subject: Subject<'_>,
    block_start: u64,
    bytes: &[u8],
    write: impl FnOnce(&mut File, &[u8]) -> std::io::Result<()>,
) -> Result<(), String> {
    #[cfg(test)]
    if let Some(keep) = fault::take_write(subject) {
        let kept = bytes.get(..keep).unwrap_or(bytes);
        let written = file
            .seek(SeekFrom::End(0))
            .and_then(|_| std::io::Write::write_all(file, kept))
            .and_then(|()| Err(std::io::Error::other("injected write fault")));
        return written.map_err(|why| {
            roll_back(
                file,
                subject,
                block_start,
                &format!("cannot append to {subject}: {why}"),
            )
        });
    }
    let written = file.seek(SeekFrom::End(0)).and_then(|_| write(file, bytes));
    written.map_err(|why| {
        roll_back(
            file,
            subject,
            block_start,
            &format!("cannot append to {subject}: {why}"),
        )
    })
}

/// Appends every encoded record as one block, then makes the block durable
/// through `sync`. A failed encode, write or barrier cuts the WHOLE block, so
/// no whole-record orphan of a block whose barrier never returned is left
/// for a retry to find in the page cache and "confirm".
pub(crate) fn append_block<B: AsRef<[u8]>>(
    file: &mut File,
    path: &Path,
    records: impl IntoIterator<Item = Result<B, String>>,
    sync: impl FnOnce(&File) -> std::io::Result<()>,
) -> Result<u64, String> {
    let block = start(file, &path.display())?;
    for record in records {
        let bytes = record.map_err(|why| roll_back(file, &path.display(), block, &why))?;
        write_at_end(
            file,
            &path.display(),
            block,
            bytes.as_ref(),
            std::io::Write::write_all,
        )?;
    }
    sync_or_roll_back(file, path, block, sync)?;
    Ok(block)
}

/// The barrier for everything written since `block_start`. A failure rolls
/// the file back to `block_start` and marks `path` as having failed a barrier
/// in this process.
pub(crate) fn sync_or_roll_back(
    file: &File,
    path: &Path,
    block_start: u64,
    sync: impl FnOnce(&File) -> std::io::Result<()>,
) -> Result<(), String> {
    #[cfg(test)]
    let sync = |file: &File| {
        if fault::take_sync(&path.display()) {
            return Err(std::io::Error::other("injected sync fault"));
        }
        sync(file)
    };
    sync(file).map_err(|why| {
        remember_failed_barrier(path);
        roll_back(
            file,
            &path.display(),
            block_start,
            &format!(
                "{} could not be made durable: {why}. A second barrier cannot prove these bytes reached the device",
                path.display()
            ),
        )
    })
}

/// `sync_all` through the same test fault hook as [`sync_or_roll_back`], for
/// a caller whose rollback is not a truncation (a created file withdrawn by
/// name).
///
/// # Errors
///
/// Whatever the barrier returned.
pub(crate) fn sync_all_hooked(file: &File, path: &Path) -> std::io::Result<()> {
    // Read only by the test fault hook below.
    #[cfg(not(test))]
    let _ = path;
    #[cfg(test)]
    if fault::take_sync(&path.display()) {
        return Err(std::io::Error::other("injected sync fault"));
    }
    file.sync_all()
}

/// [`sync_or_roll_back`] through `sync_all`.
pub(crate) fn sync_all_or_roll_back(
    file: &File,
    path: &Path,
    block_start: u64,
) -> Result<(), String> {
    sync_or_roll_back(file, path, block_start, File::sync_all)
}

/// Records that a barrier on `path` failed in this process.
pub(crate) fn remember_failed_barrier(path: &Path) {
    FAILED_BARRIERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(path.to_path_buf());
}

/// Refuses to vouch for `path` once a barrier on it failed in this process.
///
/// # Errors
///
/// Names the path when this process has seen a barrier on it fail.
pub(crate) fn refuse_after_failed_barrier(path: &Path) -> Result<(), String> {
    if FAILED_BARRIERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains(path)
    {
        return Err(format!(
            "a durability barrier on {} already failed in this process; durability is unknown, and a second barrier cannot confirm it. Rerun after the device is healthy",
            path.display()
        ));
    }
    Ok(())
}

/// Cuts a sub-record tail past `header + k·stride`, makes the cut durable, and
/// reports it. Call ONLY from a writer holding the ledger's exclusive lock.
///
/// `magic` is the file's leading bytes (empty for a headerless file). A file
/// that does not begin with them is not this ledger, and nothing is cut: the
/// caller's own header check then refuses it by name.
///
/// Returns `None` and changes nothing when the file is no longer than
/// `header` (not this module's question), does not carry `magic`, or already
/// ends on a whole record.
///
/// # Errors
///
/// Names a measurement, read, truncation or barrier failure.
pub(crate) fn heal_torn_tail(
    file: &File,
    path: &Path,
    header: u64,
    stride: u64,
    magic: &[u8],
) -> Result<Option<TornTail>, String> {
    let found = file
        .metadata()
        .map_err(|why| format!("{} could not be measured: {why}", path.display()))?
        .len();
    if stride == 0 || found <= header {
        return Ok(None);
    }
    let spare = (found - header) % stride;
    if spare == 0 {
        return Ok(None);
    }
    if !magic.is_empty() {
        let mut leading = vec![0_u8; magic.len()];
        let mut reader = file;
        reader
            .seek(SeekFrom::Start(0))
            .and_then(|_| std::io::Read::read_exact(&mut reader, &mut leading))
            .map_err(|why| format!("{} could not be read: {why}", path.display()))?;
        if leading != magic {
            return Ok(None);
        }
    }
    let kept = found - spare;
    file.set_len(kept)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} ends {spare} bytes into a record, left by an interrupted append; cutting it back to {kept} bytes failed: {why}",
                path.display()
            )
        })?;
    crate::note(
        &telemetry::Event::warn("cli.ledger", "torn ledger tail truncated")
            .with("path", path.display().to_string().as_str())
            .with("found", found)
            .with("kept", kept)
            .with(
                "reason",
                "bytes past the last whole record were never acknowledged",
            ),
    );
    Ok(Some(TornTail { kept, found }))
}

/// The writer's whole repair of what a killed process left in a headered,
/// fixed-stride ledger (rnew-1, D-4460). Call ONLY from a writer holding the
/// ledger's exclusive lock, before its header check.
///
/// A non-empty strict prefix of `header` is cut to nothing ([`heal_torn_header`];
/// the caller then writes the header whole, exactly as for a new file), and a
/// sub-record tail past `header.len() + k·stride` of a file that begins with
/// `magic` is cut ([`heal_torn_tail`]). Each cut is synced and logged. A whole
/// record is never cut, and anything else stays for the caller to refuse.
///
/// # Errors
///
/// Names a measurement, read, truncation or barrier failure.
pub(crate) fn heal_header_and_tail(
    file: &File,
    path: &Path,
    header: &[u8],
    stride: u64,
    magic: &[u8],
) -> Result<(), String> {
    heal_torn_header(file, path, header)?;
    let header_len = u64::try_from(header.len())
        .map_err(|_| format!("{} header length does not fit u64", path.display()))?;
    heal_torn_tail(file, path, header_len, stride, magic).map(drop)
}

/// Discards a receipt-less trailing block that is not the caller's exact
/// retry: cuts `file` back to `at` (where the block began), makes the cut
/// durable, and reports it. Call ONLY from a writer holding the ledger's
/// exclusive lock, after the scan proved everything before `at` complete.
///
/// A block without its receipt was never acknowledged to anyone, so it is
/// scratch (D-1905, pop2-4). Refusing every other identity because of it
/// wedged the ledger until an operator cut the file by hand.
///
/// # Errors
///
/// Names a truncation or barrier failure; the tail is then still there.
pub(crate) fn discard_orphan(
    file: &File,
    path: &Path,
    at: u64,
    orphan: &str,
) -> Result<(), String> {
    let found = file
        .metadata()
        .map_err(|why| format!("{} could not be measured: {why}", path.display()))?
        .len();
    file.set_len(at)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} ends with a receipt-less block of {orphan}; cutting it back to {at} bytes failed: {why}",
                path.display()
            )
        })?;
    crate::note(
        &telemetry::Event::warn("cli.ledger", "receipt-less orphan block discarded")
            .with("path", path.display().to_string().as_str())
            .with("found", found)
            .with("kept", at)
            .with("orphan", orphan)
            .with(
                "reason",
                "a block without its receipt was never acknowledged and is not this exact retry",
            ),
    );
    Ok(())
}

/// Cuts a file that is a strict, non-empty prefix of its own `header` back to
/// zero bytes, makes the cut durable, and reports it. Call ONLY from a writer
/// holding the ledger's exclusive lock.
///
/// A writer writes a header and passes its barrier before any record can be
/// acknowledged, so a file shorter than its header names nothing. A file
/// whose bytes differ from the header's prefix is not this ledger and is left
/// for the caller's own check to refuse.
///
/// # Errors
///
/// Names a measurement, read, truncation or barrier failure.
pub(crate) fn heal_torn_header(
    file: &File,
    path: &Path,
    header: &[u8],
) -> Result<Option<TornTail>, String> {
    let found = file
        .metadata()
        .map_err(|why| format!("{} could not be measured: {why}", path.display()))?
        .len();
    let Some(expected) = usize::try_from(found)
        .ok()
        .filter(|&len| len > 0)
        .and_then(|len| header.get(..len))
        .filter(|prefix| prefix.len() < header.len())
    else {
        return Ok(None);
    };
    let mut leading = vec![0_u8; expected.len()];
    let mut reader = file;
    reader
        .seek(SeekFrom::Start(0))
        .and_then(|_| std::io::Read::read_exact(&mut reader, &mut leading))
        .map_err(|why| format!("{} could not be read: {why}", path.display()))?;
    if leading != expected {
        return Ok(None);
    }
    file.set_len(0)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} holds {found} bytes of a header that never passed its barrier; cutting it back to 0 bytes failed: {why}",
                path.display()
            )
        })?;
    crate::note(
        &telemetry::Event::warn("cli.ledger", "torn ledger header truncated")
            .with("path", path.display().to_string().as_str())
            .with("found", found)
            .with("kept", 0_u64)
            .with(
                "reason",
                "a header shorter than its own length was never acknowledged",
            ),
    );
    Ok(Some(TornTail { kept: 0, found }))
}

/// A thread-local fault a test arms to make the NEXT write or barrier whose
/// subject names `name` fail, so a ledger's own commit path is exercised
/// against a short write or a failed barrier rather than a copy of it.
#[cfg(test)]
pub(crate) mod fault {
    use std::cell::RefCell;

    /// What the armed fault does.
    #[derive(Clone, Copy, Debug)]
    pub(crate) enum Kind {
        /// The write lands `keep` bytes, then fails.
        Write {
            /// Bytes written before the failure.
            keep: usize,
        },
        /// The barrier fails.
        Sync,
        /// The barrier on a DIRECTORY fails (sobs-12, D-4461).
        DirectorySync,
    }

    std::thread_local! {
        static ARMED: RefCell<Option<(String, Kind, usize)>> = const { RefCell::new(None) };
    }

    /// The armed fault; disarmed when dropped.
    pub(crate) struct Armed;

    impl Armed {
        /// Arms one fault for the next matching write or barrier on this thread.
        pub(crate) fn arm(name: &str, kind: Kind) -> Self {
            Self::arm_after(name, kind, 0)
        }

        /// Arms one fault that lets `skip` matching calls pass first.
        pub(crate) fn arm_after(name: &str, kind: Kind, skip: usize) -> Self {
            ARMED.with(|armed| *armed.borrow_mut() = Some((name.to_owned(), kind, skip)));
            Self
        }

        /// Whether the fault is still armed (it fires once).
        pub(crate) fn pending() -> bool {
            ARMED.with(|armed| armed.borrow().is_some())
        }
    }

    impl Drop for Armed {
        fn drop(&mut self) {
            ARMED.with(|armed| *armed.borrow_mut() = None);
        }
    }

    fn take(subject: &dyn std::fmt::Display, wanted: fn(&Kind) -> bool) -> Option<Kind> {
        let subject = subject.to_string();
        ARMED.with(|armed| {
            let mut armed = armed.borrow_mut();
            let (name, kind, skip) = armed.as_mut()?;
            if !subject.contains(name.as_str()) || !wanted(kind) {
                return None;
            }
            if *skip > 0 {
                *skip -= 1;
                return None;
            }
            armed.take().map(|(_, kind, _)| kind)
        })
    }

    pub(super) fn take_write(subject: &dyn std::fmt::Display) -> Option<usize> {
        match take(subject, |kind| matches!(kind, Kind::Write { .. })) {
            Some(Kind::Write { keep }) => Some(keep),
            _ => None,
        }
    }

    pub(super) fn take_sync(subject: &dyn std::fmt::Display) -> bool {
        take(subject, |kind| matches!(kind, Kind::Sync)).is_some()
    }

    pub(super) fn take_directory_sync(subject: &dyn std::fmt::Display) -> bool {
        take(subject, |kind| matches!(kind, Kind::DirectorySync)).is_some()
    }
}

/// Test support (rnew-1, D-4460): the attack a process killed mid-append
/// mounts on a ledger.
#[cfg(test)]
#[allow(
    clippy::panic,
    reason = "a test helper that cannot fail its caller's test is no test at all"
)]
pub(crate) mod attack {
    use std::path::Path;

    /// The tail lengths every writer's heal is attacked with: the shortest, a
    /// middle and the longest sub-record tail a killed append can leave.
    /// Every length in between takes the same branch of
    /// [`super::heal_torn_tail`]: `spare` is nonzero and below `stride`.
    pub(crate) fn torn_lengths(stride: u64) -> [u64; 3] {
        [1, stride / 2, stride - 1]
    }

    /// Runs the attack against each of `files` and each of [`torn_lengths`].
    /// For every case it appends that many bytes to that file, then requires
    ///
    /// * a reader to refuse the ragged file (readers never cut),
    /// * a writer open to succeed and cut the file back to exactly its length
    ///   before the tear, so no whole record is dropped,
    /// * exactly one `torn ledger tail truncated` event naming that file, and
    /// * the reader then to see exactly the committed state `read` saw before.
    ///
    /// `read` opens read-only and summarises what it sees; `write` opens the
    /// writer and drops it.
    pub(crate) fn torn_tails(
        files: &[(&Path, u64)],
        read: &mut dyn FnMut() -> Result<String, String>,
        write: &mut dyn FnMut() -> Result<(), String>,
    ) {
        let committed = read().unwrap_or_else(|why| panic!("the committed fixture reads: {why}"));
        for &(path, stride) in files {
            for torn in torn_lengths(stride) {
                let before = std::fs::metadata(path)
                    .unwrap_or_else(|why| panic!("{} measures: {why}", path.display()))
                    .len();
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .open(path)
                    .unwrap_or_else(|why| panic!("{} opens to tear: {why}", path.display()));
                let tail = vec![0x5a_u8; usize::try_from(torn).unwrap_or(usize::MAX)];
                std::io::Write::write_all(&mut file, &tail)
                    .and_then(|()| file.sync_all())
                    .unwrap_or_else(|why| panic!("{} tears: {why}", path.display()));
                drop(file);
                assert!(
                    read().is_err(),
                    "a reader refuses {} with a {torn}-byte torn tail",
                    path.display()
                );
                drop(crate::noted::take());
                write().unwrap_or_else(|why| {
                    panic!(
                        "the writer heals {} with a {torn}-byte torn tail: {why}",
                        path.display()
                    )
                });
                let after = std::fs::metadata(path)
                    .unwrap_or_else(|why| panic!("{} measures: {why}", path.display()))
                    .len();
                assert_eq!(
                    after,
                    before,
                    "{} is cut back to its last whole record, and no further",
                    path.display()
                );
                let noted = crate::noted::take();
                let shown = path.display().to_string();
                assert_eq!(
                    noted
                        .iter()
                        .filter(|line| line.contains("torn ledger tail truncated")
                            && line.contains(shown.as_str()))
                        .count(),
                    1,
                    "one logged cut for {shown}: {noted:#?}"
                );
                assert_eq!(
                    read().unwrap_or_else(|why| panic!("the healed ledger reads: {why}")),
                    committed,
                    "the healed ledger holds exactly its committed history"
                );
            }
        }
    }
}

/// Makes `path`'s directory entry durable: one barrier on its parent
/// directory (sobs-12, D-4461). A file's own `sync_all` does not make the name
/// that reaches it durable, so a power cut right after a new ledger's first
/// synced record could lose the whole file. Called once, when a writer
/// creates the file, never per record.
///
/// # Errors
///
/// Names the directory and the failure.
pub(crate) fn sync_parent(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    sync_directory(parent)
}

/// Creates `dir` and every missing ancestor, then makes each entry it
/// created durable by one barrier on that entry's parent (sobs-12,
/// D-4461). Every barrier completes before this returns, so their order does
/// not change what is durable. A directory that already existed costs one
/// `try_exists` and no barrier.
///
/// # Errors
///
/// Names the directory that could not be measured, made or made durable.
pub(crate) fn create_dir_all_durable(dir: &Path) -> Result<(), String> {
    let mut missing = Vec::new();
    let mut at = Some(dir);
    while let Some(path) = at.filter(|path| !path.as_os_str().is_empty()) {
        if path
            .try_exists()
            .map_err(|why| format!("{} could not be measured: {why}", path.display()))?
        {
            break;
        }
        missing.push(path);
        at = path.parent();
    }
    std::fs::create_dir_all(dir)
        .map_err(|why| format!("the directory {} could not be made: {why}", dir.display()))?;
    for created in &missing {
        sync_parent(created)?;
    }
    Ok(())
}

/// One barrier on a directory, through the test fault hook.
fn sync_directory(directory: &Path) -> Result<(), String> {
    #[cfg(test)]
    if fault::take_directory_sync(&directory.display()) {
        return Err(format!(
            "the directory {} could not be made durable: injected directory sync fault",
            directory.display()
        ));
    }
    File::open(directory)
        .and_then(|opened| opened.sync_all())
        .map_err(|why| {
            format!(
                "the directory {} could not be made durable: {why}",
                directory.display()
            )
        })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "scratch-file fixtures fail loudly and cut exact byte prefixes"
)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;
    use std::io::Read as _;
    use std::io::Write as _;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "brutex-fixed-tail-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");
        dir.join("ledger.bin")
    }

    fn open(path: &Path) -> File {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .expect("open scratch ledger")
    }

    fn append_with(
        file: &mut File,
        subject: Subject<'_>,
        bytes: &[u8],
        write: impl FnOnce(&mut File, &[u8]) -> std::io::Result<()>,
    ) -> Result<u64, String> {
        let end = start(file, subject)?;
        write_at_end(file, subject, end, bytes, write)?;
        Ok(end)
    }

    fn append(file: &mut File, subject: Subject<'_>, bytes: &[u8]) -> Result<u64, String> {
        append_with(file, subject, bytes, std::io::Write::write_all)
    }

    fn contents(path: &Path) -> Vec<u8> {
        let mut bytes = Vec::new();
        File::open(path)
            .and_then(|mut file| file.read_to_end(&mut bytes))
            .expect("read scratch ledger");
        bytes
    }

    #[test]
    fn a_partial_write_error_is_cut_back_to_the_last_whole_record() {
        let path = scratch("partial");
        let mut file = open(&path);
        append(&mut file, &path.display(), &[1; 8]).expect("first record");
        let refusal = append_with(&mut file, &path.display(), &[2; 8], |file, bytes| {
            file.write_all(&bytes[..3])?;
            Err(std::io::Error::other("injected ENOSPC"))
        })
        .expect_err("the injected write fails");
        assert!(refusal.contains("injected ENOSPC"), "{refusal}");
        assert!(refusal.contains("truncated back to 8 bytes"), "{refusal}");
        assert_eq!(contents(&path), vec![1; 8]);
        assert_eq!(append(&mut file, &path.display(), &[3; 8]), Ok(8));
    }

    #[test]
    fn a_failed_barrier_cuts_the_whole_block_and_is_never_confirmed_later() {
        let path = scratch("barrier");
        let mut file = open(&path);
        append(&mut file, &path.display(), &[1; 8]).expect("first record");
        let block = start(&mut file, &path.display()).expect("block start");
        for _ in 0..3 {
            write_at_end(&mut file, &path.display(), block, &[7; 8], |file, bytes| {
                file.write_all(bytes)
            })
            .expect("block record");
        }
        let refusal = sync_or_roll_back(&file, &path, block, |_| {
            Err(std::io::Error::other("injected EIO"))
        })
        .expect_err("the injected barrier fails");
        assert!(refusal.contains("injected EIO"), "{refusal}");
        assert_eq!(contents(&path), vec![1; 8], "the whole block is cut");
        let later = refuse_after_failed_barrier(&path).expect_err("never confirmed");
        assert!(later.contains("already failed in this process"), "{later}");
        let other = scratch("barrier-other");
        assert_eq!(refuse_after_failed_barrier(&other), Ok(()));
    }

    #[test]
    fn a_write_error_mid_block_cuts_every_earlier_record_of_that_block() {
        let path = scratch("mid-block");
        let mut file = open(&path);
        append(&mut file, &path.display(), &[1; 4]).expect("committed record");
        let block = start(&mut file, &path.display()).expect("block start");
        write_at_end(&mut file, &path.display(), block, &[2; 4], |file, bytes| {
            file.write_all(bytes)
        })
        .expect("first block record");
        let refusal = write_at_end(&mut file, &path.display(), block, &[3; 4], |file, bytes| {
            file.write_all(&bytes[..1])?;
            Err(std::io::Error::other("injected short write"))
        })
        .expect_err("the second write fails");
        assert!(refusal.contains("truncated back to 4 bytes"), "{refusal}");
        assert_eq!(contents(&path), vec![1; 4]);
    }

    #[test]
    fn a_torn_tail_is_cut_to_the_last_whole_record_and_reported() {
        let path = scratch("torn");
        let mut file = open(&path);
        file.write_all(&[9; 16 + 3 * 10 + 4]).expect("torn fixture");
        let healed = heal_torn_tail(&file, &path, 16, 10, &[]).expect("heal");
        assert_eq!(
            healed,
            Some(TornTail {
                kept: 46,
                found: 50
            })
        );
        assert_eq!(contents(&path).len(), 46);
        assert_eq!(heal_torn_tail(&file, &path, 16, 10, &[]), Ok(None));
    }

    #[test]
    fn a_whole_or_short_file_is_left_alone() {
        let path = scratch("whole");
        let mut file = open(&path);
        file.write_all(&[5; 12]).expect("short fixture");
        assert_eq!(heal_torn_tail(&file, &path, 16, 10, &[]), Ok(None));
        assert_eq!(heal_torn_tail(&file, &path, 12, 0, &[]), Ok(None));
        file.write_all(&[5; 14]).expect("whole fixture");
        assert_eq!(heal_torn_tail(&file, &path, 16, 10, &[]), Ok(None));
        assert_eq!(contents(&path).len(), 26);
        file.write_all(&[5; 3]).expect("torn fixture");
        assert_eq!(heal_torn_tail(&file, &path, 16, 10, b"NOTTHIS"), Ok(None));
        assert_eq!(contents(&path).len(), 29, "a foreign file is never cut");
        assert_eq!(
            heal_torn_tail(&file, &path, 16, 10, &[5; 4]),
            Ok(Some(TornTail {
                kept: 26,
                found: 29
            }))
        );
    }

    #[test]
    fn a_strict_prefix_of_the_header_is_cut_to_nothing_and_anything_else_is_left() {
        let header = b"HEADER01";
        let path = scratch("torn-header");
        let mut file = open(&path);
        assert_eq!(heal_torn_header(&file, &path, header), Ok(None), "empty");
        file.write_all(b"HEA").expect("torn header fixture");
        assert_eq!(
            heal_torn_header(&file, &path, header),
            Ok(Some(TornTail { kept: 0, found: 3 }))
        );
        assert_eq!(contents(&path), Vec::<u8>::new());
        file.rewind().expect("rewind");
        file.write_all(b"HEX").expect("foreign fixture");
        assert_eq!(heal_torn_header(&file, &path, header), Ok(None), "foreign");
        file.set_len(0).and_then(|()| file.rewind()).expect("reset");
        file.write_all(header).expect("whole header");
        assert_eq!(heal_torn_header(&file, &path, header), Ok(None), "whole");
        assert_eq!(contents(&path), header.to_vec());
    }

    #[test]
    fn an_armed_fault_fires_once_on_the_named_subject_only() {
        let path = scratch("fault");
        let mut file = open(&path);
        let armed = fault::Armed::arm("ledger.bin", fault::Kind::Write { keep: 2 });
        let other = Path::new("elsewhere.bin");
        assert!(fault::take_write(&other.display()).is_none());
        let refusal = append(&mut file, &path.display(), &[4; 6]).expect_err("fires");
        assert!(refusal.contains("injected write fault"), "{refusal}");
        assert!(!fault::Armed::pending(), "a fault fires once");
        assert_eq!(contents(&path), Vec::<u8>::new());
        append(&mut file, &path.display(), &[4; 6]).expect("the next write lands");
        drop(armed);
        let _sync = fault::Armed::arm("ledger.bin", fault::Kind::Sync);
        let refusal = sync_all_or_roll_back(&file, &path, 0).expect_err("fires");
        assert!(refusal.contains("injected sync fault"), "{refusal}");
        assert_eq!(contents(&path), Vec::<u8>::new());
    }

    #[test]
    fn a_rollback_that_cannot_run_names_both_failures() {
        let path = scratch("readonly");
        std::fs::write(&path, [1; 4]).expect("fixture");
        let file = File::open(&path).expect("read-only handle");
        let refusal = roll_back(&file, &path.display(), 0, "injected write failure");
        assert!(refusal.contains("injected write failure"), "{refusal}");
        assert!(refusal.contains("ALSO failed"), "{refusal}");
        assert_eq!(contents(&path), vec![1; 4]);
    }

    /// sobs-12, D-4461: every directory a writer creates is made durable by a
    /// barrier on its parent, a failed barrier is refused by name, and a
    /// directory that already exists takes no barrier at all.
    #[test]
    fn created_directories_are_made_durable_and_a_failed_barrier_is_named() {
        let base = scratch("durable-dirs")
            .parent()
            .expect("scratch parent")
            .to_path_buf();
        let outer = base.join("a");
        let deep = outer.join("b");
        let named = |path: &Path| path.display().to_string();

        let armed = fault::Armed::arm(&named(&outer), fault::Kind::DirectorySync);
        let refusal = create_dir_all_durable(&deep).expect_err("the inner barrier fails");
        assert!(
            refusal.contains("injected directory sync fault") && refusal.contains(&named(&outer)),
            "{refusal}"
        );
        assert!(!fault::Armed::pending(), "the barrier on `a` was reached");
        drop(armed);
        assert!(deep.is_dir(), "the chain was made before its barriers");

        let fresh = base.join("c").join("d");
        let armed = fault::Armed::arm(&named(&base), fault::Kind::DirectorySync);
        let refusal = create_dir_all_durable(&fresh).expect_err("a barrier in the chain fails");
        assert!(refusal.contains(&named(&base)), "{refusal}");
        drop(armed);

        let armed = fault::Armed::arm(&named(&base), fault::Kind::DirectorySync);
        create_dir_all_durable(&deep).expect("an existing chain needs nothing");
        assert!(
            fault::Armed::pending(),
            "an existing directory takes no barrier"
        );
        drop(armed);
        create_dir_all_durable(&base.join("e").join("f")).expect("real barriers succeed");

        let file = deep.join("ledger.bin");
        let armed = fault::Armed::arm(&named(&deep), fault::Kind::DirectorySync);
        let refusal = sync_parent(&file).expect_err("the parent's barrier fails");
        assert!(
            refusal.contains("injected directory sync fault") && refusal.contains(&named(&deep)),
            "{refusal}"
        );
        drop(armed);
        sync_parent(&file).expect("a real barrier succeeds");
        sync_parent(Path::new("bare.bin")).expect("a bare name's parent is `.`");
        let refusal =
            sync_parent(&base.join("missing").join("x.bin")).expect_err("no such directory");
        assert!(refusal.contains("could not be made durable"), "{refusal}");

        // A directory barrier fault is not a file write or barrier fault.
        let _armed = fault::Armed::arm("ledger.bin", fault::Kind::DirectorySync);
        assert!(fault::take_write(&file.display()).is_none());
        assert!(!fault::take_sync(&file.display()));
        assert!(fault::Armed::pending());
    }

    /// rnew-1, D-4460: the attack helper itself cuts, logs and keeps history
    /// on a plain headerless file.
    #[test]
    fn the_torn_tail_attack_holds_on_a_plain_fixed_stride_file() {
        let path = scratch("attack");
        std::fs::write(&path, [7_u8; 3 * 10]).expect("three whole records");
        let reads = |path: &Path| -> Result<String, String> {
            let bytes = std::fs::read(path).map_err(|why| why.to_string())?;
            if bytes.len() % 10 == 0 {
                Ok(format!("{} records", bytes.len() / 10))
            } else {
                Err("ragged".to_owned())
            }
        };
        attack::torn_tails(&[(&path, 10)], &mut || reads(&path), &mut || {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .map_err(|why| why.to_string())?;
            heal_torn_tail(&file, &path, 0, 10, &[]).map(drop)
        });
        assert_eq!(attack::torn_lengths(4_096), [1, 2_048, 4_095]);
        assert_eq!(contents(&path).len(), 30);
    }
}
