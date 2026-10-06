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

/// What [`init_or_heal_header`] did to a writer's file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeaderInit {
    /// The file already held more than an interrupted header; nothing was
    /// written. The caller's own header check decides whether it is this
    /// ledger.
    Present,
    /// The file was empty, or held only an interrupted header that was cut,
    /// and this call wrote `header` and passed its barrier. A file this call
    /// made non-empty needs its directory synced by the caller (conc11-1).
    Written,
}

/// Initialises a writer's fixed header, or heals one that never passed its
/// barrier, ONE way for every cli ledger (conc5-1, D-2644). Call ONLY from a
/// writer holding the ledger's exclusive lock; readers keep refusing.
///
/// - A file of `1..=header.len()` bytes that are ALL ZERO is an interrupted
///   genesis: a power cut on a filesystem that extends the size before the
///   data, or a header whose barrier failed and whose page was then evicted.
///   No record can follow a header that never passed its barrier, so the
///   file names nothing; it is cut to zero and reported, as the store's own
///   `is_interrupted_genesis` does (D-1520, D-1521).
/// - A strict non-empty prefix of `header` is cut the same way
///   ([`heal_torn_header`]).
/// - An empty file then gets `header`, through [`write_at_end`] and
///   [`sync_or_roll_back`]: a failed write or barrier cuts the file back to
///   zero bytes, and a failed barrier is remembered for this path
///   ([`remember_failed_barrier`]). The header was left in place before, so
///   after eviction it read back as zeros at full length and every later
///   open, the writer's included, refused the empty ledger for good.
///
/// # Errors
///
/// Names a measurement, read, truncation, write or barrier failure.
pub(crate) fn init_or_heal_header(
    file: &mut File,
    path: &Path,
    header: &[u8],
    sync: impl FnOnce(&File) -> std::io::Result<()>,
) -> Result<HeaderInit, String> {
    heal_zeroed_header(file, path, header.len())?;
    heal_torn_header(file, path, header)?;
    let len = file
        .metadata()
        .map_err(|why| format!("{} could not be measured: {why}", path.display()))?
        .len();
    if len != 0 {
        return Ok(HeaderInit::Present);
    }
    write_at_end(file, &path.display(), 0, header, std::io::Write::write_all)?;
    sync_or_roll_back(file, path, 0, sync)?;
    Ok(HeaderInit::Written)
}

/// The healing half of [`init_or_heal_header`], for a writer whose own header
/// write keeps a ledger-specific order (a read-back before the barrier):
/// an all-zero file of at most `header.len()` bytes, or a strict prefix of
/// `header`, is cut to nothing and reported (conc5-1, D-2644). Call ONLY
/// from a writer holding the ledger's exclusive lock.
///
/// # Errors
///
/// Names a measurement, read, truncation or barrier failure.
pub(crate) fn heal_interrupted_header(
    file: &File,
    path: &Path,
    header: &[u8],
) -> Result<(), String> {
    heal_zeroed_header(file, path, header.len())?;
    heal_torn_header(file, path, header).map(drop)
}

/// The sixteen-byte header the results, frontier and result-set ledgers
/// write: the magic, the little-endian version, and four zero bytes.
pub(crate) fn sixteen_byte_header(magic: [u8; 8], version: u32) -> [u8; 16] {
    let mut header = [0_u8; 16];
    for (slot, byte) in header.iter_mut().zip(magic_and_version(magic, version)) {
        *slot = byte;
    }
    header
}

/// Cuts a file of `1..=header_len` bytes that are all zero back to nothing,
/// durably, and reports it. Anything else is left alone.
fn heal_zeroed_header(file: &File, path: &Path, header_len: usize) -> Result<(), String> {
    let found = file
        .metadata()
        .map_err(|why| format!("{} could not be measured: {why}", path.display()))?
        .len();
    let Some(len) = usize::try_from(found)
        .ok()
        .filter(|&len| len > 0 && len <= header_len)
    else {
        return Ok(());
    };
    let mut leading = vec![0_u8; len];
    let mut reader = file;
    reader
        .seek(SeekFrom::Start(0))
        .and_then(|_| std::io::Read::read_exact(&mut reader, &mut leading))
        .map_err(|why| format!("{} could not be read: {why}", path.display()))?;
    if leading.iter().any(|&byte| byte != 0) {
        return Ok(());
    }
    file.set_len(0)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} holds {found} zero bytes where a header that never passed its barrier was; cutting it back to 0 bytes failed: {why}",
                path.display()
            )
        })?;
    crate::note(
        &telemetry::Event::warn("cli.ledger", "zero-filled ledger header reinitialised")
            .with("path", path.display().to_string().as_str())
            .with("found", found)
            .with("kept", 0_u64)
            .with(
                "reason",
                "an all-zero header was never acknowledged, so no record can follow it",
            ),
    );
    Ok(())
}

/// Syncs the directory holding `path`, so a file a writer just created and
/// initialised keeps its name across a power cut (conc11-1, D-2645).
///
/// # Errors
///
/// Names the directory and the host's refusal.
pub(crate) fn sync_parent_directory(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory to sync", path.display()))?;
    File::open(parent)
        .and_then(|directory| sync_all_hooked(&directory, parent))
        .map_err(|why| {
            format!(
                "the directory {} holding {} could not be synced: {why}",
                parent.display(),
                path.display()
            )
        })
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

    fn take(subject: &dyn std::fmt::Display, sync: bool) -> Option<Kind> {
        let subject = subject.to_string();
        ARMED.with(|armed| {
            let mut armed = armed.borrow_mut();
            let (name, kind, skip) = armed.as_mut()?;
            if !subject.contains(name.as_str()) || matches!(kind, Kind::Sync) != sync {
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
        match take(subject, false) {
            Some(Kind::Write { keep }) => Some(keep),
            _ => None,
        }
    }

    pub(super) fn take_sync(subject: &dyn std::fmt::Display) -> bool {
        take(subject, true).is_some()
    }
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

    const HEADER16: [u8; 16] = *b"CONC51HDR\x01\x00\x00\x00\x00\x00\x07";

    fn init(path: &Path) -> Result<HeaderInit, String> {
        init_or_heal_header(&mut open(path), path, &HEADER16, File::sync_all)
    }

    /// conc5-1 (D-2644): every all-zero file of 1..=header bytes, every strict
    /// prefix of the header and an empty file are re-initialised by the
    /// writer; the exact header, a zero file one byte longer than the header,
    /// a non-zero foreign file, and a header followed by records are left for
    /// the caller's own check. On the old code no helper existed and every
    /// writer refused a full-length zero header as a wrong magic for good.
    #[test]
    fn a_zero_filled_full_header_is_reinitialised_by_the_writer() {
        let path = scratch("conc5-1-zero");
        for len in 0..=HEADER16.len() {
            std::fs::write(&path, vec![0_u8; len]).expect("zero fixture");
            assert_eq!(init(&path), Ok(HeaderInit::Written), "{len} zero bytes");
            assert_eq!(contents(&path), HEADER16.to_vec(), "{len} zero bytes");
        }
        for len in 1..HEADER16.len() {
            std::fs::write(&path, &HEADER16[..len]).expect("prefix fixture");
            assert_eq!(init(&path), Ok(HeaderInit::Written), "{len}-byte prefix");
            assert_eq!(contents(&path), HEADER16.to_vec());
        }
        let kept: [Vec<u8>; 5] = [
            HEADER16.to_vec(),
            vec![0_u8; HEADER16.len() + 1],
            b"FOREIGNHEADER123".to_vec(),
            [&HEADER16[..], &[9_u8; 8][..]].concat(),
            {
                let mut torn = vec![0_u8; HEADER16.len()];
                torn[15] = 1;
                torn
            },
        ];
        for bytes in kept {
            std::fs::write(&path, &bytes).expect("kept fixture");
            assert_eq!(init(&path), Ok(HeaderInit::Present), "{bytes:?}");
            assert_eq!(contents(&path), bytes, "left for the caller's own check");
        }
        // A reader never calls the helper, and the zero header is still
        // refused by name by a magic check: the healing is the writer's.
        assert_eq!(sixteen_byte_header(*b"BRUTEXRS", 3)[..12], magic_and_version(*b"BRUTEXRS", 3));
        assert_eq!(sixteen_byte_header(*b"BRUTEXRS", 3)[12..], [0_u8; 4]);
    }

    /// conc5-1 (D-2644): a header whose barrier fails is cut back to zero
    /// bytes and remembered, a header whose write fails part way is cut back
    /// too, and the next writer open writes it again. On the old code the
    /// header bytes were left in place and the failure was not remembered.
    #[test]
    fn a_failed_header_barrier_is_cut_and_remembered() {
        use fault::{Armed, Kind};
        let path = scratch("conc5-1-barrier");
        {
            let _armed = Armed::arm("conc5-1-barrier", Kind::Sync);
            let refusal = init(&path).expect_err("the barrier fails");
            assert!(refusal.contains("injected sync fault"), "{refusal}");
            assert!(!Armed::pending());
        }
        assert_eq!(contents(&path), Vec::<u8>::new(), "cut back to nothing");
        assert!(refuse_after_failed_barrier(&path).is_err(), "remembered");
        assert_eq!(init(&path), Ok(HeaderInit::Written), "the next open rewrites");
        assert_eq!(contents(&path), HEADER16.to_vec());

        let path = scratch("conc5-1-write");
        for keep in [0, 1, HEADER16.len() - 1] {
            {
                let _armed = Armed::arm("conc5-1-write", Kind::Write { keep });
                assert!(init(&path).is_err(), "a short write at {keep} refuses");
            }
            assert_eq!(contents(&path), Vec::<u8>::new(), "cut back at {keep}");
        }
        assert_eq!(init(&path), Ok(HeaderInit::Written));
        assert_eq!(init(&path), Ok(HeaderInit::Present), "and is then present");
    }

    /// conc11-1 (D-2645): the directory barrier is a real barrier on the
    /// directory holding the file, and a failure is named.
    #[test]
    fn a_created_files_directory_is_synced_and_a_failure_named() {
        use fault::{Armed, Kind};
        let path = scratch("conc11-1-directory");
        sync_parent_directory(&path).expect("the directory syncs");
        let _armed = Armed::arm("conc11-1-directory", Kind::Sync);
        let refusal = sync_parent_directory(&path).expect_err("the barrier fails");
        assert!(refusal.contains("could not be synced"), "{refusal}");
        assert!(!Armed::pending());
        assert!(sync_parent_directory(Path::new("")).is_err(), "no parent");
    }

    /// conc11-1 (D-2645): the two store-root ledgers D-1903 missed sync the
    /// root after a header this writer initialised, through the shared
    /// header rule. Measured on the source, as ZL-09 is, because a directory
    /// entry's durability cannot be observed without a power cut. On the old
    /// code neither file named a directory barrier.
    #[test]
    fn candidate_universe_and_pre_admission_sync_the_root_after_initialising() {
        for (name, source, function) in [
            (
                "candidate_universe.rs",
                include_str!("candidate_universe.rs"),
                "fn ensure_header(",
            ),
            (
                "pre_admission_data.rs",
                include_str!("pre_admission_data.rs"),
                "fn init_header(",
            ),
        ] {
            let body = source
                .split_once(function)
                .expect("the header door exists")
                .1;
            let body = body.split_once("\n}\n").map_or(body, |(head, _)| head);
            let init = body
                .find("init_or_heal_header(")
                .expect("the shared header rule is called");
            let synced = body
                .find("sync_parent_directory(path)")
                .expect("the root barrier is called");
            assert!(init < synced, "{name}: the root is synced after the header");
        }
        let pre = include_str!("pre_admission_data.rs");
        for door in ["fn ensure_header(", "fn ensure_header_v2("] {
            let body = pre.split_once(door).expect("both doors").1;
            let body = body.split_once("\n}\n").map_or(body, |(head, _)| head);
            assert!(body.contains("init_header(file, path,"), "{door}: {body}");
        }
    }
}
