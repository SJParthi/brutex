//! The write path: a month's bar file, opened, appended to, and read back.
//!
//! Every other module in this crate is arithmetic over bytes somebody else
//! holds. This one is the only place the crate touches the operating system,
//! and it exists because the crate whose entire job is bytes on disk could not
//! put a byte on disk: an audit found no `std::fs` and no `File::` anywhere
//! under `crates/store/src`.
//!
//! # The five calls, and what is durable after each
//!
//! Durability is stated per call rather than left to the reader, because
//! "after which call are the bars safe" is the only question that matters
//! after a power loss.
//!
//! | Call | Durable when it returns `Ok` |
//! |---|---|
//! | [`BarFile::open_or_create`] on a **new** month | the whole 32768-byte header region, and the file's *name* in its directory — both `fsync`ed, the directory one included |
//! | [`BarFile::open_or_create`] on an **existing** month | nothing new; it only reads |
//! | [`BarFile::open_existing`] | nothing, ever — it creates no directory, no bar file and no lock, and a month that is absent is [`StoreError::Missing`] naming the path |
//! | [`BarFile::append`] returning [`Appended::Committed`] | every appended record **and** the header slot that publishes them, in that order, with an `fsync` after each |
//! | [`BarFile::append`] returning [`Appended::AlreadyPresent`] | nothing was written; the month already held every offered bar, byte for byte, at the index the answer names |
//! | [`BarFile::read_record`] | nothing; it writes nothing |
//!
//! The order inside [`BarFile::append`] is `docs/02-store-format.md` §5, and
//! the two `fsync`s are not decoration. The header slot published before its
//! records are durable is the *likely* reordering, not the exotic one — the
//! header page is re-dirtied on every commit and is therefore the hottest
//! writeback candidate in the file. [`crate::header::Commit::durable_through`]
//! is the offset the first `fsync` must cover, and it is exactly the end of the
//! records this commit publishes: `store::write::the_committed_length_is_exactly_durable_through`
//! asserts the file's length against it after every commit of a ladder.
//!
//! # No writable mapping, and why the ban is load-bearing here
//!
//! `CLAUDE.md` §4 bans a writable memory mapping and this module is where that
//! ban is either honoured or broken. A mapping that runs out of space raises
//! `SIGBUS`, which is delivered asynchronously and cannot be caught by any
//! construct in any language — so the process dies mid-write and the operator
//! gets a core file instead of a refusal. Every write here is an ordinary
//! positional write syscall, which **returns** `ENOSPC` as a value, and that
//! value is turned into [`StoreError::DiskFull`] and handed back.
//!
//! # One writer per month, enforced by an advisory lock
//!
//! [`crate::header::Header::commit`]'s crash argument is conditioned on
//! exactly one writer, and `docs/02-store-format.md` §9 names the mechanism:
//! an advisory lock on the [`crate::path::FileKind::Lock`] sibling, taken
//! before the bar file is even measured and held for the life of the
//! [`BarFile`]. A second [`BarFile`] on the same month is refused by name with
//! [`StoreError::Locked`] rather than allowed to interleave commits.
//!
//! [`BarFile::open_existing`] takes that lock **shared** instead, so any number
//! of readers coexist and no reader blocks another, while a writer holding it
//! exclusively still refuses them all. A reader also never *creates* the lock
//! file: a bar file with no lock beside it has had no writer since it was
//! written, so there is nothing to wait for, and conjuring the file into being
//! to hold a lock on it would be the very write that door exists to avoid.
//!
//! **What the lock does not protect against, stated rather than implied away:**
//!
//! * A process that opens the `.bin` directly and writes to it. The lock is
//!   advisory: it constrains writers that ask, which is every writer that goes
//!   through this type and no other.
//! * A network filesystem whose lock is a fiction. `flock` over NFS is
//!   emulated, and over some mounts it is a no-op that reports success.
//!   `docs/02-store-format.md` §9 already says to keep the store on local disk.
//! * A symlink at any path component. [`crate::path::StorePath`] guarantees a
//!   **lexical** property only, and this module resolves nothing: it opens the
//!   rendered path with an ordinary `open`, so `bars/groww` linked at
//!   `bars/dhan` sends one vendor's writes into another's file with the lock
//!   held on the wrong month. Closing it needs `openat` with `O_NOFOLLOW` per
//!   component.
//! * A crash. The lock is released by the kernel when the process dies, which
//!   is the property a lock *file* created with `O_EXCL` would not have — that
//!   was the alternative, and it was rejected because a crashed writer would
//!   leave a stale lock that only an operator could clear.
//!
//! # This build writes files with no block checksums, on purpose
//!
//! [`crate::format::FLAG_CHECKSUMS`] is clear in every file this module
//! creates, and the [`crate::path::FileKind::Checksums`] sidecar is not
//! created. `docs/02-store-format.md` §6 provides for exactly that state: a
//! file whose flag is clear carries no sidecar and a verification request
//! against it is **refused** rather than answered, which
//! [`crate::block::verify`] already does by returning
//! [`crate::format::FormatError::ChecksumsAbsent`].
//!
//! It is a hole and it is named as one. What it costs: a lost write to the
//! record extent is undetectable, because an all-zero record is a legal flat
//! bar. What closing it needs is *not* more code here — it is a sidecar entry
//! that says which record count it covers. The tail block's checksum domain is
//! a function of `n_valid` ([`crate::layout::Layout::covered_byte_range`]), so
//! a writer that re-seals the tail block on every commit has a window between
//! the sidecar write and the header commit in which a crash leaves an entry
//! computed over a record count the header does not yet claim — and the next
//! reader reports a healthy file as corrupt. Inventing an entry format to close
//! that is a `docs/02-store-format.md` change with a decision entry behind it,
//! not something to slip into a writer.

use std::fmt;
use std::fs::{self, File, TryLockError};
use std::io::{self, ErrorKind};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

use crate::format::{Bar, FormatError, HEADER_LEN, MAX_SLOT_COUNT, Row, SLOT_STRIDE};
use crate::header::Header;
use crate::layout::Layout;
use crate::path::{FileKind, StorePath};

/// The largest header region the format family can declare, as a length.
///
/// Written as a literal in both widths rather than converted, the way every
/// other paired constant in this crate is: a fallible conversion here would
/// carry a failure arm no input could reach. The assertions keep the two
/// honest, and they are stated against the **family** bound rather than
/// against version 2, so a version with fewer slots still reads correctly and
/// a version with more is a compile error here.
const REGION_LEN: usize = 32_768;

const _: () = assert!(REGION_LEN == 32_768);
const _: () = assert!(MAX_SLOT_COUNT * SLOT_STRIDE == 32_768);
const _: () = assert!(HEADER_LEN <= 32_768);

/// [`REGION_LEN`] in the width an offset is measured in.
const REGION_LEN_U64: u64 = 32_768;

const _: () = assert!(REGION_LEN_U64 == 32_768);

/// Which syscall refused, so an error names the operation and not only the
/// path.
///
/// "Permission denied" against a month file is three different bugs depending
/// on whether it was the directory, the lock or the records that refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Action {
    /// Creating the directory tree the month lives in.
    CreateDir,
    /// Opening a file.
    Open,
    /// Taking the month's advisory lock.
    Lock,
    /// Reading a file's length.
    Measure,
    /// Reading bytes at an offset.
    Read,
    /// Writing bytes at an offset.
    Write,
    /// Flushing to stable storage.
    Sync,
}

impl Action {
    /// The word this action is named by in a refusal.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CreateDir => "creating the directory",
            Self::Open => "opening",
            Self::Lock => "locking",
            Self::Measure => "measuring",
            Self::Read => "reading",
            Self::Write => "writing",
            Self::Sync => "syncing",
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Why a bar file could not be opened, appended to or read.
///
/// Every variant names the path, the operation, or both. A refusal that says
/// only "I/O error" sends an operator to `strace`, which is where a "just retry
/// it" habit comes from.
///
/// The host's own error is reduced to its [`ErrorKind`] and its `errno` rather
/// than kept: those two are `Copy` and comparable, so a test asserts an exact
/// value instead of matching a message the platform is free to reword.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StoreError {
    /// The path names a sibling that is not the bar records.
    ///
    /// Refused rather than silently redirected to `.bin`: a caller that asked
    /// for the overlay and got the records back would corrupt the one it meant
    /// to write.
    NotABarPath {
        /// The sibling that was asked for.
        found: FileKind,
    },
    /// Another writer holds this month's advisory lock.
    Locked {
        /// The lock file.
        path: PathBuf,
    },
    /// The disk filled. `ENOSPC`.
    ///
    /// This is the error a writable mapping could not have returned — it would
    /// have raised `SIGBUS` instead and killed the process mid-write. See the
    /// module documentation.
    DiskFull {
        /// The file being written.
        path: PathBuf,
        /// Which operation refused.
        action: Action,
    },
    /// The host refused permission. `EACCES`.
    Denied {
        /// The file or directory.
        path: PathBuf,
        /// Which operation refused.
        action: Action,
    },
    /// The filesystem is mounted read-only. `EROFS`.
    ReadOnly {
        /// The file or directory.
        path: PathBuf,
        /// Which operation refused.
        action: Action,
    },
    /// A directory sits where the file belongs. `EISDIR`.
    IsADirectory {
        /// The path that is a directory.
        path: PathBuf,
        /// Which operation refused.
        action: Action,
    },
    /// A file sits where a directory component belongs. `ENOTDIR`.
    NotADirectory {
        /// The path whose parent is not a directory.
        path: PathBuf,
        /// Which operation refused.
        action: Action,
    },
    /// The path is not there. `ENOENT`.
    Missing {
        /// The path that does not exist.
        path: PathBuf,
        /// Which operation refused.
        action: Action,
    },
    /// The host refused for a reason this module does not classify.
    ///
    /// Carries the kind and the raw `errno` rather than collapsing to "I/O
    /// error", so an unclassified failure is still diagnosable and the
    /// classifier can be widened by whoever meets one.
    Io {
        /// The file or directory.
        path: PathBuf,
        /// Which operation refused.
        action: Action,
        /// The host's classification.
        kind: ErrorKind,
        /// The raw `errno`, when the host supplied one.
        code: Option<i32>,
    },
    /// A write accepted fewer bytes than it was offered and then stopped
    /// accepting any.
    ///
    /// A *partial* write is not this: it is ordinary, and the loop resumes at
    /// the exact offset the host stopped at. This is the case where the host
    /// takes zero bytes twice — there is no offset to resume from and
    /// retrying forever would hang.
    ShortWrite {
        /// The file being written.
        path: PathBuf,
        /// Where the stall happened.
        offset: u64,
        /// Bytes still owed at that offset.
        asked: usize,
        /// Bytes the host had accepted before it stalled.
        wrote: usize,
    },
    /// A read returned fewer bytes than the file's own geometry promised.
    ///
    /// The file was truncated under an open handle: the header says the
    /// records are there and the bytes are not.
    ShortRead {
        /// The file being read.
        path: PathBuf,
        /// Where the read stopped.
        offset: u64,
        /// Bytes still owed at that offset.
        asked: usize,
        /// Bytes the host had returned before it ran out.
        read: usize,
    },
    /// The file's length is not the header region plus a whole number of
    /// records.
    ///
    /// **NO DOOR IN THIS MODULE CONSTRUCTS THIS ANY MORE, AND THAT IS THE
    /// POINT.** It was returned for a remainder anywhere in the file, which
    /// bricked a month for bytes no commit claimed: an append interrupted
    /// before its header slot was written leaves a partial record *past*
    /// `offset_of(n_valid)`, and refusing there cost every committed bar in
    /// the month to protect a tail nothing reads. `BarFile::validated` now
    /// measures the committed extent instead and reports the remainder through
    /// the log — see the comment there, and `docs/04-invariants.md` S-07.
    ///
    /// The other half — a file SHORTER than its own counter claims — is still
    /// refused, as [`StoreError::Format`] carrying
    /// [`FormatError::CounterExceedsFile`]. Usually that happens one layer up,
    /// in [`crate::header::Header::validate`], which is where it was always
    /// caught first; when the header search answers it by walking back to an
    /// older slot instead, `BarFile::validated` catches it, because the
    /// walk-back turns a counter that is ahead of the bytes into one that is
    /// behind them and the two mean opposite things.
    ///
    /// The variant is kept rather than deleted because deleting a public
    /// variant is an API change that wants a `docs/05-decisions.md` entry
    /// behind it, and because the refusal-rendering test in
    /// `crates/store/tests/write.rs` still builds one and asserts its sentence.
    /// Removing it is a separate change, not a side effect of this one.
    RaggedTail {
        /// The bar file.
        path: PathBuf,
        /// Its length.
        len: u64,
        /// Bytes past the last whole record.
        extra: u64,
    },
    /// The header disagrees with the bytes, or with itself.
    ///
    /// Carries [`FormatError`], which names which disagreement it was:
    /// [`FormatError::CounterExceedsFile`] for a header claiming more records
    /// than the file can hold, [`FormatError::NoValidHeader`] when every slot
    /// is damaged, [`FormatError::TimestampsOutOfOrder`] for a batch that does
    /// not follow what is committed, and so on.
    Format {
        /// The bar file.
        path: PathBuf,
        /// What the bytes said.
        source: FormatError,
    },
    /// The file was written for a different instrument.
    ///
    /// `symbol_id` is a cross-check, never the index — this is the check.
    SymbolMismatch {
        /// The bar file.
        path: PathBuf,
        /// What the header carries.
        stored: u32,
        /// What the caller asked for.
        asked: u32,
    },
    /// The file was written at a different bar length.
    TimeframeMismatch {
        /// The bar file.
        path: PathBuf,
        /// Seconds per bar the header carries.
        stored: u32,
        /// Seconds per bar the path names.
        asked: u32,
    },
    /// A record index at or past the commit counter.
    ///
    /// Bytes past `n_valid` are not "empty", they are **not there**: they may
    /// be a half-written batch from a crash, and returning them as a bar would
    /// hand a sweep data nobody committed.
    NotCommitted {
        /// The index asked for.
        index: u64,
        /// How many records are committed.
        n_valid: u64,
    },
    /// An append with no records in it.
    ///
    /// Refused rather than treated as a no-op: publishing nothing still
    /// advances the generation and rewrites a header slot, so a caller looping
    /// over empty days would rewrite the header once per day for no bars. A day
    /// with no bars is a census outcome, not a commit.
    EmptyBatch,
    /// A batch whose own timestamps do not strictly increase.
    ///
    /// Refused at the write boundary, because afterwards it is well-formed
    /// bytes that no checksum can catch.
    BatchNotOrdered {
        /// Index within the batch of the bar that did not follow.
        at: u64,
        /// The timestamp before it.
        previous: i64,
        /// The timestamp that did not follow.
        next: i64,
    },
    /// A batch holding a bar whose VOLUME or OPEN INTEREST cannot have
    /// happened.
    ///
    /// Separate from [`Self::ImpossibleBar`] so the message names the field an
    /// operator has to go and look at. See [`Bar::counts_are_sane`].
    ImpossibleCount {
        /// Index within the batch.
        at: u64,
        /// The volume as offered.
        volume: i64,
        /// The open interest as offered. [`OI_NULL`] is legal and means absent.
        open_interest: i64,
    },
    /// A batch holding a bar whose OHLC cannot have happened.
    ///
    /// [`Bar::ohlc_is_sane`] exists "so an impossible bar never reaches the
    /// disk in the first place"; this is the call site that makes that true.
    ImpossibleBar {
        /// Index within the batch.
        at: u64,
    },
}

/// The [`StoreError::ImpossibleCount`] sentence.
///
/// Lifted out of `Display::fmt` only to keep that match under the workspace's
/// 100-line ceiling; it carries no logic of its own.
fn write_impossible_count(
    f: &mut fmt::Formatter<'_>,
    at: u64,
    volume: i64,
    open_interest: i64,
) -> fmt::Result {
    write!(
        f,
        "batch record {at} has an impossible count: volume {volume}, open \
         interest {open_interest}. A count is never negative — zero means \
         zero, and the only legal negative is the open-interest null sentinel"
    )
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotABarPath { found } => {
                write!(
                    f,
                    "path names the {} sibling, not the records",
                    found.extension()
                )
            }
            Self::Locked { path } => {
                write!(f, "another writer holds {}", path.display())
            }
            Self::DiskFull { path, action } => {
                write!(f, "disk full {action} {}", path.display())
            }
            Self::Denied { path, action } => {
                write!(f, "permission denied {action} {}", path.display())
            }
            Self::ReadOnly { path, action } => {
                write!(f, "read-only filesystem {action} {}", path.display())
            }
            Self::IsADirectory { path, action } => {
                write!(f, "{} is a directory, {action} it", path.display())
            }
            Self::NotADirectory { path, action } => {
                write!(
                    f,
                    "a path component of {} is not a directory, {action} it",
                    path.display()
                )
            }
            Self::Missing { path, action } => {
                write!(f, "{} does not exist, {action} it", path.display())
            }
            Self::Io {
                path,
                action,
                kind,
                code,
            } => write!(
                f,
                "{action} {} failed: {kind:?} (errno {code:?})",
                path.display()
            ),
            Self::ShortWrite {
                path,
                offset,
                asked,
                wrote,
            } => write!(
                f,
                "{} stalled at offset {offset} with {asked} bytes still owed after {wrote} accepted",
                path.display()
            ),
            Self::ShortRead {
                path,
                offset,
                asked,
                read,
            } => write!(
                f,
                "{} ended at offset {offset} with {asked} bytes owed after {read}",
                path.display()
            ),
            Self::RaggedTail { path, len, extra } => write!(
                f,
                "{} is {len} bytes, {extra} past the last whole record",
                path.display()
            ),
            Self::Format { path, source } => write!(f, "{}: {source}", path.display()),
            Self::SymbolMismatch {
                path,
                stored,
                asked,
            } => write!(f, "{} holds symbol {stored}, not {asked}", path.display()),
            Self::TimeframeMismatch {
                path,
                stored,
                asked,
            } => write!(
                f,
                "{} holds {stored}-second bars, not {asked}",
                path.display()
            ),
            Self::NotCommitted { index, n_valid } => {
                write!(f, "record {index} is past the {n_valid} committed")
            }
            Self::EmptyBatch => f.write_str("an append with no records in it"),
            Self::BatchNotOrdered { at, previous, next } => {
                write!(f, "batch record {at}: {next} does not follow {previous}")
            }
            Self::ImpossibleCount {
                at,
                volume,
                open_interest,
            } => write_impossible_count(f, *at, *volume, *open_interest),
            Self::ImpossibleBar { at } => {
                write!(f, "batch record {at} has impossible OHLC")
            }
        }
    }
}

impl std::error::Error for StoreError {}

/// What an append did.
///
/// Two outcomes rather than one, because "we wrote it" and "it was already
/// there" are different facts and a caller counting bars needs to tell them
/// apart. Collapsing them into `Ok(())` is the shape of a silent no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Appended {
    /// The batch was written and published.
    Committed {
        /// The index the first record of the batch landed at.
        first_index: u64,
        /// The commit counter after the append.
        n_valid: u64,
    },
    /// The month already held every bar of the batch, byte for byte, starting
    /// at `first_index`. Nothing was written and no generation was spent.
    ///
    /// This is what makes a re-run of the same pull safe: the second append
    /// leaves the file byte-identical, which
    /// `store::write::re_appending_the_same_batch_leaves_the_file_byte_identical`
    /// proves by checksumming the whole file either side.
    ///
    /// **`first_index` is where the bars actually are, not `n_valid - count`.**
    /// It used to be the second of those, because the check could only ever
    /// answer for the file's tail; a re-pull of a window in the middle of a
    /// month is now answered at the index it really occupies.
    AlreadyPresent {
        /// The index the first record of the batch already sits at.
        first_index: u64,
        /// The commit counter, unchanged.
        n_valid: u64,
    },
}

/// One month's bar file, open for reading and appending.
///
/// Holds the month's advisory lock for as long as it lives. Dropping it closes
/// the records and then releases the lock, in that order — the fields are
/// declared in that order and Rust drops them in declaration order.
#[derive(Debug)]
pub struct BarFile {
    /// The records.
    bars: File,
    /// Where they are, for every refusal this type can produce.
    bars_path: PathBuf,
    /// The geometry, taken from the file's own `format_version`.
    layout: Layout,
    /// The committed state, re-read from the file on open.
    header: Header,
    /// The advisory lock, held for its **drop** and never read again.
    ///
    /// Underscored because that is what it is: closing this descriptor is what
    /// releases the month, so the field is live for its whole life and is
    /// touched exactly once, by the compiler-generated drop. Declared last so
    /// the records are closed before the lock is released.
    _lock: Option<File>,
}

/// The only geometry a `.ovl` file may have.
///
/// A one-row table rather than a bare `Layout`, so it goes through the same
/// `Layout::resolve` the bar path uses — which is what makes a `.ovl` carrying
/// any other version number a NAMED refusal rather than a silent mis-read.
const OVERLAY_TABLE: &[Layout] = &[Layout::OVERLAY];

/// The one geometry a `.grk` file can resolve to.
const GREEKS_TABLE: &[Layout] = &[Layout::GREEKS];

/// Whether this kind of file holds records at a declared geometry.
///
/// `.crc` and `.lock` do not: opening either through [`BarFile`] would read its
/// first bytes as a header and its rest as records at whatever stride that
/// header happened to name.
const fn holds_records(kind: FileKind) -> bool {
    matches!(kind, FileKind::Bars | FileKind::Overlay | FileKind::Greeks)
}

/// The geometry a file of this kind is BORN at.
///
/// Decided once, from the kind, so creation and reopen cannot disagree — a file
/// born at the overlay's geometry and reopened against the bar table reports
/// `UnknownVersion(9)`, which is the resolver being right and the caller having
/// handed it the wrong table. `Layout::CURRENT` for anything that is not a
/// sidecar, which after [`holds_records`] can only be [`FileKind::Bars`].
const fn geometry_of(kind: FileKind) -> Layout {
    match kind {
        FileKind::Overlay => Layout::OVERLAY,
        FileKind::Greeks => Layout::GREEKS,
        _ => Layout::CURRENT,
    }
}

/// The versions a file of this kind may resolve against.
///
/// A sidecar gets a one-row table; a bar file gets every bar version this build
/// can read. See [`geometry_of`] on why the two answers must be derived from
/// the same input.
const fn table_of(kind: FileKind) -> &'static [Layout] {
    match kind {
        FileKind::Overlay => OVERLAY_TABLE,
        FileKind::Greeks => GREEKS_TABLE,
        _ => Layout::KNOWN,
    }
}

impl BarFile {
    /// Opens a month's bar file, creating it and its directory if absent.
    ///
    /// A file that does not exist, and one that exists with zero bytes, are
    /// the same thing to this function: both get a fresh 32768-byte header
    /// region. A zero-byte file is what a crash between `open` and the first
    /// write leaves behind, and it can hold no record by definition, so
    /// initialising it loses nothing. Any other length is read, never
    /// overwritten.
    ///
    /// The header is always **read back from the file** before this returns,
    /// including on the create path, so the caller never holds a header that
    /// only the process believes in.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotABarPath`] for a path naming a sibling other than the
    /// records. [`StoreError::Locked`] when another writer holds the month.
    /// [`StoreError::Denied`], [`StoreError::ReadOnly`],
    /// [`StoreError::IsADirectory`], [`StoreError::NotADirectory`],
    /// [`StoreError::DiskFull`] or [`StoreError::Io`] from the host.
    /// [`StoreError::Format`] for a header that disagrees with the file's
    /// length — [`FormatError::CounterExceedsFile`] is the file that is
    /// *shorter* than its counter claims, including when an older slot would
    /// otherwise have covered for it — or is damaged in every slot, and
    /// [`StoreError::SymbolMismatch`] or [`StoreError::TimeframeMismatch`] for
    /// a file written for something else.
    ///
    /// **Not [`StoreError::RaggedTail`].** Bytes past the committed extent are
    /// an interrupted append, and this door opens the month and logs them
    /// rather than refusing it; see `Self::validated`.
    pub fn open_or_create(
        root: &Path,
        path: StorePath<'_>,
        symbol_id: u32,
    ) -> Result<Self, StoreError> {
        // BARS OR A SIDECAR BESIDE THEM, and nothing else. A `.crc` or a
        // `.lock` opened here would be read as records at whichever geometry
        // its first bytes happened to name.
        //
        // Asked of `FileKind` rather than spelled as a chain of `!=`. There are
        // three record kinds now and the chain had to be edited in two places
        // to add the third — two places that could have disagreed.
        if !holds_records(path.file()) {
            return Err(StoreError::NotABarPath { found: path.file() });
        }
        let timeframe_secs = path.timeframe().secs();
        // THE GEOMETRY THIS FILE IS BORN AT AND RESOLVED AGAINST, decided once
        // from the file kind so creation and reopen cannot disagree.
        let born = geometry_of(path.file());
        let bars_path = path.to_path_buf(root);
        let lock_path = path.with_file(FileKind::Lock).to_path_buf(root);

        // A rendered store path always has six parent components, so the
        // fallback is unreachable; it is written as one anyway because the
        // alternative is a refusal arm no input can produce. Reaching it would
        // create the root and then fail loudly on the open below.
        let dir = bars_path.parent().unwrap_or(root).to_path_buf();
        fault(fs::create_dir_all(&dir), &dir, Action::CreateDir)?;

        // The lock is taken before the bar file is opened, let alone measured.
        // Everything below this line assumes exactly one writer.
        let lock = fault(open_rw(&lock_path), &lock_path, Action::Open)?;
        if let Err(refusal) = lock.try_lock() {
            return Err(lock_fault(&lock_path, refusal));
        }

        let bars = fault(open_rw(&bars_path), &bars_path, Action::Open)?;
        let mut len = fault(bars.metadata(), &bars_path, Action::Measure)?.len();

        // `len == 0` WAS THE REPAIR CONDITION, AND IT DID NOT COVER THE CRASH.
        //
        // `initialise` is two writes with nothing between them: a 32,768-byte
        // zero fill, then the 64-byte header, then one sync. A process that
        // dies inside that window leaves a file of 1..=32,768 bytes with no
        // committed header. On the next open `len != 0`, so the repair was
        // skipped, `validated` found no header slot, and the month refused to
        // open — FOREVER. Nothing could rewrite it, because §3 rule 8 says
        // nothing may.
        //
        // And the likeliest crash point is the worst one: after the fill and
        // before the header is exactly `REGION_LEN` bytes, the size a healthy
        // empty month also has.
        //
        // The condition is now "every byte this file has is zero, and it has no
        // more than the region". That is provably an interrupted `initialise`
        // and provably holds no data:
        //
        //   * records live PAST `REGION_LEN`, so a file this short has none;
        //   * a committed header is never all zeros — `Header::commit` writes a
        //     magic and a CRC — so all-zero means no header was ever committed;
        //   * therefore re-running `initialise` destroys nothing, and refusing
        //     instead would strand a month that holds nothing.
        //
        // Deliberately NOT "no valid header ⇒ re-initialise". A file with real
        // records and a corrupted header must still refuse loudly: that one has
        // something to lose, and §3 rule 8 outranks getting it open.
        //
        // `len == 0` is subsumed rather than kept beside this — an empty file
        // is the all-zero case with nothing in it, and two conditions that must
        // agree are two conditions that can drift.
        if len <= REGION_LEN_U64 {
            let mut head = vec![0u8; usize::try_from(len).unwrap_or(REGION_LEN)];
            read_fully(&bars, &bars_path, 0, &mut head)?;
            if head.iter().all(|&byte| byte == 0) {
                initialise(&bars, &bars_path, symbol_id, timeframe_secs, born)?;
                fsync_dir(&dir)?;
                len = fault(bars.metadata(), &bars_path, Action::Measure)?.len();
            }
        }

        // THE SAME DOOR AS `open_existing`, AND NOW ACTUALLY THE SAME CODE.
        //
        // `open_existing`'s doc says of these two: "both call [`Self::validated`]
        // so they cannot drift into disagreeing about what a well-formed month
        // is." That sentence was FALSE. `validated` had exactly one caller —
        // `open_existing` — and this function carried its own copy of the four
        // checks, twenty-eight lines that happened to be byte-for-byte identical.
        //
        // Nothing was wrong with the values, and that is precisely why it was
        // worth fixing: the comment asserted a guarantee that no mechanism
        // enforced, so the two agreed only for as long as nobody edited one of
        // them. A promise kept by coincidence is the shape `CLAUDE.md` §4 calls
        // a fallback that hides a failure — it reads as safe and refuses
        // nothing. The duplicate is gone and the sentence is now true by
        // construction rather than by inspection.
        Self::validated(
            bars,
            bars_path,
            Some(lock),
            len,
            symbol_id,
            timeframe_secs,
            // THE SAME ANSWER CREATION USED, from the same function.
            table_of(path.file()),
        )
    }

    /// Open a month that already exists, **without creating anything**.
    ///
    /// # Why this exists
    ///
    /// [`Self::open_or_create`] was this module's only door, so every reader
    /// went through the writer's path and got the writer's side effects. A
    /// read-only HTTP page — `GET /bars` — therefore *created* whatever tuple a
    /// caller typed into the query string: six directories, a 32 KiB bar file
    /// with a freshly initialised header, and a `.lock` beside it. Then it
    /// rendered "this month is empty" about the file the request had just made.
    /// Reproduced live against a symbol that has never existed.
    ///
    /// The disk cost was the smaller half. The load-bearing damage was that
    /// `open_or_create` had erased the distinction the page was trying to
    /// report: "there is no such month" and "there is a month and it holds
    /// nothing" arrived as the same answer, so the missing-file arm was
    /// unreachable and the honest message could never be shown. That is
    /// `CLAUDE.md` §4 — a fallback that hides a failure — reached by accident
    /// rather than by design.
    ///
    /// # What it does differently
    ///
    /// No `create_dir_all`, no `create(true)`, no `initialise`. A month that is
    /// not there returns [`StoreError::Missing`] naming the path it looked for,
    /// which is what the caller wanted to say all along. The lock is taken
    /// **shared**, so any number of readers coexist and a reader never blocks
    /// another reader — but a writer holding the exclusive lock still refuses
    /// them, which is the property that matters.
    ///
    /// Every validation after the open is the same one `open_or_create`
    /// performs, and both call `Self::validated` so they cannot drift into
    /// disagreeing about what a well-formed month is.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotABarPath`] for a non-bar path, [`StoreError::Missing`]
    /// when the month does not exist, [`StoreError::Locked`] when a writer holds
    /// it, and whatever `Self::validated` refuses.
    pub fn open_existing(
        root: &Path,
        path: StorePath<'_>,
        symbol_id: u32,
    ) -> Result<Self, StoreError> {
        // BARS OR A SIDECAR BESIDE THEM, and nothing else. A `.crc` or a
        // `.lock` opened here would be read as records at whichever geometry
        // its first bytes happened to name.
        //
        // Asked of `FileKind` rather than spelled as a chain of `!=`. There are
        // three record kinds now and the chain had to be edited in two places
        // to add the third — two places that could have disagreed.
        if !holds_records(path.file()) {
            return Err(StoreError::NotABarPath { found: path.file() });
        }
        let bars_path = path.to_path_buf(root);
        let lock_path = path.with_file(FileKind::Lock).to_path_buf(root);

        // Read-only and no `create`, so a missing month is `ENOENT` and
        // `classify` turns it into `Missing` naming this exact path.
        let bars = fault(File::open(&bars_path), &bars_path, Action::Open)?;

        // The lock is opened read-only and never created. A bar file with no
        // lock beside it has had no writer since it was made, so there is
        // nothing to wait for and `None` is the honest answer; inventing the
        // file to hold a lock on would be the very write this function exists
        // to avoid.
        let lock = match File::open(&lock_path) {
            Ok(handle) => {
                if let Err(refusal) = handle.try_lock_shared() {
                    return Err(lock_fault(&lock_path, refusal));
                }
                Some(handle)
            }
            Err(why) if why.kind() == io::ErrorKind::NotFound => None,
            Err(why) => return Err(classify(&lock_path, Action::Open, &why)),
        };

        let len = fault(bars.metadata(), &bars_path, Action::Measure)?.len();
        Self::validated(
            bars,
            bars_path,
            lock,
            len,
            symbol_id,
            path.timeframe().secs(),
            // WHICH TABLE, DECIDED BY THE FILE KIND rather than assumed, and by
            // the same function the creating door uses.
            table_of(path.file()),
        )
    }

    /// The checks both doors perform, once.
    ///
    /// Split out so the read-only and read-write paths cannot drift about what
    /// a well-formed month is — a reader that accepted a file the writer would
    /// have refused is a reader that serves bytes nobody agreed were bars.
    fn validated(
        bars: File,
        bars_path: PathBuf,
        lock: Option<File>,
        len: u64,
        symbol_id: u32,
        timeframe_secs: u32,
        // WHICH GEOMETRIES THIS FILE MAY BE. Passed rather than looked up,
        // because the answer differs by file KIND: a `.bar` may be any version
        // in `Layout::KNOWN`, and a `.ovl` may only be `Layout::OVERLAY`.
        //
        // Handing the bar table to an overlay would be the dangerous
        // direction: the header region is the SAME SHAPE in both, so a 24-byte
        // file opened at a 56-byte geometry validates its header, passes its
        // CRC, and then reads every field from the wrong offset. The table is a
        // parameter so that cannot happen by omission.
        table: &[Layout],
    ) -> Result<Self, StoreError> {
        let (header, claimed) = read_header(&bars, &bars_path, len)?;
        let layout = refused(Layout::resolve(table, header.format_version), &bars_path)?;

        // A TORN TAIL PAST THE COUNTER BRICKED THE WHOLE MONTH, PERMANENTLY.
        //
        // This measured `layout.ragged_tail_bytes(len)` — the raggedness of the
        // WHOLE FILE — and refused any non-zero remainder. But the remainder
        // that reaches this line is not the file's, it is the part of the file
        // NO COMMIT COVERS. `read_header` has already put every slot through
        // `Header::validate`, which refuses `n_valid > capacity_for(len)` as
        // `FormatError::CounterExceedsFile` and otherwise walks back to an
        // older generation — so by here the bytes the counter claims are all
        // present, and everything past `offset_of(n_valid)` is an append that
        // died before its header slot was written.
        //
        // Those bytes are not data, and they are not in anything's way: the
        // next append writes at exactly `offset_of(n_valid)` and overwrites
        // them. Refusing cost the entire month — every committed bar in it —
        // to protect a tail no reader can reach, and `CLAUDE.md` §3 rule 8
        // means nothing in this repository may rewrite the file to clear it.
        // One interrupted pull and the month was unopenable by every process,
        // forever. `docs/04-invariants.md` S-07 and `docs/02-store-format.md`
        // §7 both promise the opposite: discard the remainder, log the byte
        // count, continue.
        //
        // The old refusal's own justification has expired. It read "this crate
        // has no logging sink to be loud through", which was true when it was
        // written; `crates/store` has depended on `telemetry` since D-0075, and
        // `note_tail_past_the_commit` is the loud half `CLAUDE.md` §4 asks for.
        //
        // WHAT THIS DOES NOT DO, stated rather than implied: it does not
        // truncate. §7's pseudocode does, and the module doc above argues
        // against performing a destructive write on an operator's file at open
        // time. Ignoring the bytes and naming them is the same outcome for
        // every reader — nothing at or past `n_valid` is readable, which is
        // S-03 — without the write. It also leaves the remainder there across
        // reopens, so the line repeats once per open until an append covers it.
        //
        // AND THE `len < committed_end` ARM IS BACK, BECAUSE THE FALLBACK
        // LAUNDERED IT. This comment used to say that arm was unreachable —
        // "refused one layer up as `CounterExceedsFile`" — and that was true of
        // the newest *commit* and false of the *file*. When the newest slot
        // fails `Header::validate`, `Header::read_region` does not refuse: it
        // walks back to an older slot and returns that one. Commit four
        // records, cut the file 39 bytes short of the fourth, and generation 1
        // is rejected for `CounterExceedsFile`, generation 0 comes back with
        // `n_valid == 0`, and 185 bytes — three whole records a commit
        // published, plus a torn fourth — arrive on this line looking exactly
        // like an interrupted append. Accepting them demotes three committed
        // bars to scratch that the next append overwrites, which is `CLAUDE.md`
        // §4's fallback that hides a failure, one layer removed.
        //
        // `claimed` — the largest counter any slot of the region still decodes
        // as — is what separates the two, and it is the only thing that can.
        // Both files are 32,953 bytes and both are 17 bytes ragged, so no
        // function of the length can tell them apart:
        //
        //   * `claimed == n_valid`: no header slot ever published these bytes.
        //     An append died before its commit. Accept, and log them.
        //   * `claimed > n_valid`: a slot DID publish records this counter
        //     drops, and their bytes are still on the disk. The counter is
        //     ahead of the file, not behind it. Refuse.
        //
        // `store::write::a_counter_behind_its_bytes_opens_and_a_counter_ahead_of_them_is_refused`
        // asserts the two side by side so a later edit cannot collapse them
        // again, which is what both halves of this line's history did in turn.
        //
        // `CounterExceedsFile` and not `RaggedTail`, deliberately: what is
        // wrong is the 39 bytes MISSING below the commit, not the 17 that
        // happen to be extra above the last whole record. Truncate to a record
        // boundary instead and `RaggedTail`'s own `extra` is zero while the
        // three records are just as lost — the length-shaped refusal cannot
        // even state this condition, which is half of why it was the wrong one.
        //
        // WHAT THIS DOES NOT FIX, stated rather than implied: a truncation that
        // lands exactly on an older commit's extent. Cut the file to
        // `offset_of` of the generation the fallback returns and there is
        // nothing left over to notice — `discarded` is zero and the bytes are
        // byte-for-byte what a header published before its records became
        // durable looks like. Those two files are the same file, so no rule
        // over these bytes can separate them, and the recovery is the half
        // worth keeping: `store::write::a_header_that_outran_its_file_falls_back_one_generation`
        // is that file and it still opens. Catching it needs the block
        // checksums this build does not write, which the module doc names.
        //
        // The subtraction is written through `capacity_for` and
        // `ragged_tail_bytes` rather than `offset_of`, because those two are
        // infallible and `offset_of` is not: with `n_valid <= capacity_for(len)`
        // already proven, its overflow arm is unreachable too, and a `?` on it
        // would smuggle one back in. The identity is
        //   len − offset_of(n_valid) = (capacity − n_valid)·stride + ragged
        // and `store::file::the_discarded_count_is_the_bytes_past_the_counter`
        // pins it against `offset_of` directly.
        let discarded = bytes_past_the_counter(layout, len, header.n_valid);

        // A SLOT CLAIMING MORE BARS THAN THE HEADER WE FELL BACK TO IS A LOSS,
        // WHETHER OR NOT THERE IS A RAGGED TAIL. This guard used to sit INSIDE
        // the `discarded != 0` arm below, and the case it could not see is the
        // one that loses data in silence: a truncation landing EXACTLY on an
        // older commit's record extent leaves `discarded == 0` by construction,
        // so the check was never evaluated and the file opened clean one
        // generation short. `SLOT_COUNT == 2`, so the older commit is always
        // the immediately preceding append -- the vulnerable length is the file
        // as it stood one append ago, which is exactly where a lost tail
        // naturally lands.
        //
        // THE SEPARATING VALUE WAS ALREADY IN A LOCAL. `claimed` is
        // `highest_claim`'s maximum `n_valid` over every slot that still
        // decodes -- magic, version, stride and a CRC over all 64 bytes -- and
        // a tail truncation cannot reach offset 0, so the slot naming the lost
        // bars survives to prove they existed. The comment above this block
        // reasoned that catching this needed block checksums the build does not
        // write; it did not, it needed this `if` one level out.
        //
        // IT CANNOT FIRE ON AN HONEST INTERRUPTED APPEND. `append` writes the
        // records and `sync_all`s them BEFORE writing any slot that names them,
        // and a torn slot fails its CRC and does not decode at all -- so in
        // the benign case `claimed == header.n_valid` and this is silent.
        if claimed > header.n_valid {
            return Err(StoreError::Format {
                path: bars_path,
                source: FormatError::CounterExceedsFile,
            });
        }
        if discarded != 0 {
            note_tail_past_the_commit(&bars_path, len, header.n_valid, discarded);
        }

        if header.symbol_id != symbol_id {
            return Err(StoreError::SymbolMismatch {
                path: bars_path,
                stored: header.symbol_id,
                asked: symbol_id,
            });
        }
        if header.timeframe_secs != timeframe_secs {
            return Err(StoreError::TimeframeMismatch {
                path: bars_path,
                stored: header.timeframe_secs,
                asked: timeframe_secs,
            });
        }
        Ok(Self {
            bars,
            bars_path,
            layout,
            header,
            _lock: lock,
        })
    }

    /// The committed header, as the file's own bytes describe it.
    #[must_use]
    pub const fn header(&self) -> Header {
        self.header
    }

    /// The geometry this file's version defines.
    #[must_use]
    pub const fn layout(&self) -> Layout {
        self.layout
    }

    /// How many records are committed.
    #[must_use]
    pub const fn records(&self) -> u64 {
        self.header.n_valid
    }

    /// The bar file's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.bars_path
    }

    /// Appends a batch and publishes it.
    ///
    /// The sequence is `docs/02-store-format.md` §5: the records are written
    /// at `header_len + n_valid · stride`, flushed, and only then is the
    /// 64-byte header slot written and flushed. Nothing is written at all
    /// until the batch has been surveyed and the next header computed, so a
    /// refusal leaves the file byte-identical.
    ///
    /// **Re-appending bars the month already holds is a no-op, not a
    /// duplicate — wherever in the month they sit.** A batch whose first
    /// timestamp does not follow the committed range is located by that
    /// timestamp and compared, record by record, against the records already
    /// there: identical means [`Appended::AlreadyPresent`] and no write at
    /// all, different means a refusal. A re-pull of the same day is therefore
    /// safe and leaves the same bytes, which is `CLAUDE.md` §3 rule 5.
    ///
    /// The location used to be assumed rather than found — the batch was
    /// compared against the file's last `count` records and nothing else — so
    /// re-offering a day from the *middle* of a month was refused as a
    /// conflict, which is the ordinary shape of a re-pull of one day inside a
    /// month already backfilled.
    ///
    /// # Errors
    ///
    /// [`StoreError::EmptyBatch`], [`StoreError::BatchNotOrdered`] or
    /// [`StoreError::ImpossibleBar`] before anything is written.
    /// [`StoreError::Format`] carrying
    /// [`FormatError::TimestampsOutOfOrder`] for a batch that overlaps the
    /// committed range with different bars, or
    /// [`FormatError::CounterOverflow`] and
    /// [`FormatError::GenerationExhausted`] at the counters' ends. Anything
    /// the host refuses: [`StoreError::DiskFull`], [`StoreError::ShortWrite`],
    /// [`StoreError::Denied`], [`StoreError::Io`].
    pub fn append<R: Row>(&mut self, batch: &[R]) -> Result<Appended, StoreError> {
        // THE FILE'S STRIDE AND THE RECORD'S WIDTH MUST AGREE. Writing a
        // 24-byte record into a 56-byte geometry lays every field at the wrong
        // offset and the CRC would still pass, because the bytes written are
        // the bytes read back. Refused here, once, before any of them move.
        if u64::try_from(R::LEN) != Ok(self.layout.record_stride()) {
            return Err(StoreError::NotABarPath {
                found: FileKind::Overlay,
            });
        }
        let (first_ts, last_ts) = survey(batch)?;
        let count = len_u64(batch.len());

        // `Header::advance` is the single authority on whether a batch follows
        // what is committed. Asking it first, rather than repeating its
        // comparison here, is what keeps the two from drifting apart — a
        // duplicate of that condition would also be a branch no test could
        // tell from the original.
        let next = match self.header.advance(count, first_ts, last_ts) {
            Ok(next) => next,
            Err(source) => {
                // ONLY A TIMESTAMP DISAGREEMENT CAN BE AN OVERLAP, and asking
                // that first is what stops this function recursing forever.
                //
                // `advance` refuses for three reasons, and this doc block names
                // all three: timestamps out of order, `CounterOverflow`, and
                // `GenerationExhausted`. The two counter refusals are TERMINAL —
                // they say the file cannot take another batch at all, and they
                // say nothing whatever about these bars' timestamps.
                //
                // The recovery below did not ask. For a batch that legitimately
                // FOLLOWS what is held, `suffix_that_follows` finds an empty
                // overlap and hands back the WHOLE batch, and `self.append` was
                // then called with byte-identical input — same header, same
                // bars, same refusal, forever. Not a hang either: it is a stack
                // overflow, which aborts the process, and an abort cannot be
                // caught, reported, or turned into the named error the doc
                // above promises. `store::fault::the_counter_and_the_generation
                // _refuse_to_wrap` asserts that named error and reaches it by
                // another path, so nothing failed while this one recursed.
                if !matches!(source, FormatError::TimestampsOutOfOrder { .. }) {
                    return Err(StoreError::Format {
                        path: self.bars_path.clone(),
                        source,
                    });
                }

                // It does not follow. Three things it could be, and only the
                // third is a conflict.
                //
                // ONE: exactly what is already there, byte for byte — a re-run
                // of the same pull. Write nothing.
                //
                // THE BATCH IS LOCATED, NOT ASSUMED TO BE THE TAIL. This asked
                // `tail_matches`, which compared the batch against the LAST
                // `count` committed records and nothing else. A day re-pulled
                // from the middle of a month therefore compared 2026-08-04's
                // bars against 2026-08-29's, found them different, and the
                // month refused the whole window as a vendor restating history
                // — for bars it already held, byte for byte. Only a re-pull
                // that happened to end at the file's last record could answer.
                let located = already_stored(batch, first_ts, self.header.n_valid, |index| {
                    self.read_row::<R>(index)
                })?;
                if let Some(first_index) = located {
                    return Ok(Appended::AlreadyPresent {
                        first_index,
                        n_valid: self.header.n_valid,
                    });
                }

                // TWO: a PARTIAL overlap whose overlapping part matches. This
                // is the normal shape of a resumed backfill and it was being
                // refused outright:
                //
                //   held   2026-08-04, 375 bars
                //   offered 2026-08-04..=2026-08-06, 1,260 bars
                //   -> 375 duplicates, 885 new, and all 1,260 refused
                //
                // Measured on a real Groww pull: 1,260 rows read, 0 stored.
                // `CLAUDE.md` §3 rule 5 promises reruns are safe, and across
                // the ~11,200 requests of the stated backfill a run that cannot
                // resume after one interruption is a run that cannot finish.
                //
                // THE OVERLAP IS VERIFIED, NOT ASSUMED. Every offered bar at or
                // before `last_ts_micros` must equal the record already stored
                // at its timestamp. A bar the file does not hold, or holds
                // differently, falls through to the refusal below — dropping it
                // silently is the fallback §4 bans, and "earlier than the last
                // held" is NOT the same claim as "already held".
                if let Some(suffix) = self.suffix_that_follows(batch)? {
                    return self.append(suffix);
                }

                // THREE: a genuine conflict. The vendor restated history, or
                // bars arrived out of order. Refused by name.
                return Err(StoreError::Format {
                    path: self.bars_path.clone(),
                    source,
                });
            }
        };
        let commit = refused(next.commit(), &self.bars_path)?;
        let first_index = self.header.n_valid;
        let at = refused(self.layout.offset_of(first_index), &self.bars_path)?;

        let mut image = Vec::with_capacity(batch.len().saturating_mul(R::LEN));
        for row in batch {
            row.write_into(&mut image);
        }

        // Step 1 and step 3 of §5: the records, then the barrier. Every byte
        // below `commit.durable_through` is on stable storage when the second
        // write is issued.
        write_fully(&self.bars, &self.bars_path, at, &image)?;
        fault(self.bars.sync_all(), &self.bars_path, Action::Sync)?;

        // Step 4 and step 5: one write of one self-checked 64-byte unit, into
        // the slot that does not hold the previous commit.
        write_fully(&self.bars, &self.bars_path, commit.offset, &commit.bytes)?;
        fault(self.bars.sync_all(), &self.bars_path, Action::Sync)?;

        self.header = commit.header;
        // THE WRITE ITSELF, ONCE IT IS DURABLE — after the second `sync_all`,
        // never before. An event emitted ahead of the commit would claim a
        // durability the file does not have yet, and a crash between the two
        // would leave a log asserting bars that are not there. That is a worse
        // failure than silence, which is what this path had.
        //
        // `Debug`: this is the per-member write path, so a normal run at the
        // `Info` floor pays one relaxed atomic load and writes nothing.
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::debug("store.append", "committed")
                .with(
                    "file",
                    telemetry::Value::Str(&self.bars_path.display().to_string()),
                )
                .with("bars", telemetry::Value::Uint(count))
                .with("first_index", telemetry::Value::Uint(first_index))
                .with("n_valid", telemetry::Value::Uint(self.header.n_valid))
                .with("generation", telemetry::Value::Uint(self.header.generation)),
        );
        Ok(Appended::Committed {
            first_index,
            n_valid: self.header.n_valid,
        })
    }

    /// Reads one record by index, in O(1).
    ///
    /// One multiply, one add, one positional read of [`RECORD_LEN`] bytes. No
    /// scan, no directory walk, no index file — the address *is* the index,
    /// which is the whole reason the format exists, and the work does not move
    /// with the number of records in the file.
    /// `store::write::a_record_reads_back_at_its_computed_offset` pins the
    /// offset against [`Layout::offset_of`] at both ends of a 375-record file
    /// and across two checksum blocks.
    ///
    /// **Read that bound precisely.** The *operation* is constant; the read
    /// syscall underneath it is not free and its latency is the device's, not
    /// this crate's. Nothing here measures the device, and no bench in this
    /// repository times a syscall — `crates/store/benches/ratio.rs` measures
    /// the arithmetic and the checksum. The syscall cost is UNVERIFIED.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotCommitted`] for an index at or past the commit
    /// counter. [`StoreError::ShortRead`] when the file was truncated under
    /// this handle, and anything the host refuses.
    pub fn read_record(&self, index: u64) -> Result<Bar, StoreError> {
        if index >= self.header.n_valid {
            return Err(StoreError::NotCommitted {
                index,
                n_valid: self.header.n_valid,
            });
        }
        self.read_row::<Bar>(index)
    }

    /// One record of whatever kind this file holds.
    ///
    /// # Why the width comes from the RECORD and the offset from the FILE
    ///
    /// `offset_of` uses the file's own stride, read from its header; the buffer
    /// uses the record type's width. If those disagree the read is short or
    /// long, and `read_fully` refuses rather than serving a partial record —
    /// which is the check that stops a 24-byte overlay being decoded at a
    /// 56-byte geometry after its header validated.
    ///
    /// # Errors
    ///
    /// [`StoreError`] for an unreadable offset or a short read.
    fn read_row<W: Row>(&self, index: u64) -> Result<W, StoreError> {
        let at = refused(self.layout.offset_of(index), &self.bars_path)?;
        let mut image = vec![0u8; W::LEN];
        read_fully(&self.bars, &self.bars_path, at, &mut image)?;
        refused(W::read_from(&image), &self.bars_path)
    }

    /// The part of `batch` that follows what is committed, when the part that
    /// does not follow is **already stored, byte for byte**.
    ///
    /// `None` when the overlap cannot be verified — a bar at a timestamp the
    /// file does not hold, or one whose values differ from the record there.
    /// The caller refuses in that case, because silently dropping a bar the
    /// store never had is the failure `CLAUDE.md` §4 forbids: "earlier than the
    /// last held" is not the same claim as "already held", and a bar arriving
    /// out of order is missing data rather than a duplicate.
    ///
    /// Returns `None` for an empty suffix too. A batch wholly inside what is
    /// held is [`Appended::AlreadyPresent`]'s job, which the caller has already
    /// tried; reaching here with nothing left means the overlap did not verify.
    ///
    /// # Cost
    ///
    /// One record read per OVERLAPPING bar. Bounded by the batch, never by the
    /// file: the partition point comes from `last_ts_micros`, a header field.
    /// A batch with no overlap reads nothing.
    fn suffix_that_follows<'b, R: Row>(
        &self,
        batch: &'b [R],
    ) -> Result<Option<&'b [R]>, StoreError> {
        if self.header.n_valid == 0 {
            return Ok(None);
        }
        let held_through = self.header.last_ts_micros;

        // `survey` has already proven the batch is strictly increasing, so the
        // first bar past the held range is the partition and everything before
        // it is the overlap.
        let split = batch.partition_point(|row| row.stamp() <= held_through);
        let (overlap, suffix) = batch.split_at(split);
        if suffix.is_empty() {
            return Ok(None);
        }

        // EVERY overlapping bar must be the one already stored. The stored
        // records are strictly increasing too, so the overlap ends at
        // `n_valid` and begins that many records back.
        //
        // THIS ANCHOR IS STILL THE TAIL, AND DELIBERATELY. The check above it
        // no longer is — `already_stored` locates the batch by timestamp —
        // but the two are not the same question, and this one cannot be wrong
        // in the accepting direction: a bar carries its own `ts_micros`, so a
        // comparison that matches at some index proves that index holds that
        // timestamp. Anchoring wrongly can only make the comparison FAIL, and
        // a failure here is a refusal, never a silent drop. What it costs is
        // an overlap that is not a contiguous run ending at the last held bar
        // — a vendor that skipped a bar inside the held range — which is
        // refused rather than resumed. Locating it by timestamp would refuse
        // it too, at the gap instead of at the anchor, so the bisection buys
        // nothing here and is not spent.
        let Some(start) = self.header.n_valid.checked_sub(len_u64(overlap.len())) else {
            return Ok(None);
        };
        for (offset, bar) in overlap.iter().enumerate() {
            let stored = self.read_row::<R>(start.saturating_add(len_u64(offset)))?;
            if stored != *bar {
                return Ok(None);
            }
        }
        Ok(Some(suffix))
    }
}

/// Checks a batch and returns its first and last timestamps.
///
/// Every refusal here happens before a byte is written, which is the point:
/// `docs/02-store-format.md` §9 puts range validation "at the ingest boundary,
/// before a byte is written", and this is that boundary for the two properties
/// the bytes themselves cannot carry — an impossible bar and a batch out of
/// order are both well-formed records afterwards.
fn survey<R: Row>(batch: &[R]) -> Result<(i64, i64), StoreError> {
    let Some(first) = batch.first() else {
        return Err(StoreError::EmptyBatch);
    };
    let mut previous: Option<i64> = None;
    for (offset, bar) in batch.iter().enumerate() {
        if !bar.is_sane() {
            return Err(StoreError::ImpossibleBar {
                at: len_u64(offset),
            });
        }
        // AND THE COUNTS, WHICH THIS LOOP DID NOT ASK ABOUT.
        //
        // `ohlc_is_sane` is named for the four prices and checks exactly those.
        // So `volume: -1` walked past here, was appended, checksummed and
        // recorded as good — the same silent write D-0143 closed on the price
        // side, one field over. Named as its own fault rather than folded into
        // `ImpossibleBar`, because "impossible OHLC" sent to an operator
        // holding a bad volume is a wrong diagnosis, and §4 requires the reason.
        if let Some((volume, open_interest)) = bar.bad_counts() {
            return Err(StoreError::ImpossibleCount {
                at: len_u64(offset),
                volume,
                open_interest,
            });
        }
        if let Some(earlier) = previous
            && bar.stamp() <= earlier
        {
            return Err(StoreError::BatchNotOrdered {
                at: len_u64(offset),
                previous: earlier,
                next: bar.stamp(),
            });
        }
        previous = Some(bar.stamp());
    }
    // The batch is non-empty and strictly increasing, so the running
    // timestamp is the last one.
    Ok((first.stamp(), previous.unwrap_or(first.stamp())))
}

/// Where `batch` already sits among `n_valid` committed records, when the file
/// holds **every** bar of it, byte for byte, in one run.
///
/// `None` when the file does not hold them there — a timestamp it has no
/// record for, a bar whose values differ from the record at that timestamp, or
/// a batch that runs past the last committed record. Each of those is the
/// caller's problem to refuse or to resume from; answering "already present"
/// for any of them is the fallback `CLAUDE.md` §4 bans.
///
/// `first_ts` is the batch's first timestamp, which [`survey`] has already
/// computed and already proven is the smallest. Taking it as an argument
/// rather than re-reading `batch[0]` is what keeps this function free of an
/// empty-batch arm that [`survey`] makes unreachable.
///
/// # Cost, stated exactly
///
/// `O(log n_valid)` reads to locate the run, then one read per offered bar —
/// so it is bounded by the batch plus a bisection, never by the month. It runs
/// only when a batch overlaps the committed range, which is the re-pull case;
/// an ordinary forward append never enters it.
///
/// **This is NOT an O(1) path and does not claim to be.** `CLAUDE.md` §3 rule
/// 4's constant-cost list is bar lookup, condition lookup, mask evaluation,
/// duplicate rejection and result append — the per-bar sweep path — and
/// `docs/07-o1-architecture.md` layer 4 bans a search on it, membership above
/// all. This is the ingest boundary: it runs once per offered batch, off the
/// sweep entirely, and the alternative it replaced was not O(1) either — it
/// read `count` records unconditionally and answered the wrong question. The
/// bisection is written as a loop over [`BarFile::read_record`] rather than
/// spelled `binary_search`, because there is no slice to call that on: the
/// records are on disk.
///
/// # Reader
///
/// `read` is [`BarFile::read_record`] at the call site. It is a parameter so
/// this function can be driven from a table in memory — the same reason
/// [`Positional`] exists in this module, and the reason
/// `store::file::locating_a_batch_costs_a_bisection_and_not_a_scan` can assert
/// the read COUNT rather than assert the cost in a comment.
fn already_stored<W: Row, F>(
    batch: &[W],
    first_ts: i64,
    n_valid: u64,
    read: F,
) -> Result<Option<u64>, StoreError>
where
    F: Fn(u64) -> Result<W, StoreError>,
{
    let at = first_at_or_after(n_valid, first_ts, &read)?;

    // THE RUN MUST LIE WHOLLY INSIDE WHAT IS COMMITTED. Without this the
    // comparison below would ask for a record past the counter and get
    // `StoreError::NotCommitted` back — turning a batch that merely EXTENDS
    // the month, which is the resume the caller handles next, into a hard
    // refusal naming an index nobody asked about.
    //
    // Saturating rather than checked, for `len_u64`'s reason: a sum that
    // saturates is `u64::MAX`, which exceeds every real `n_valid` and lands on
    // the `None` below — a refusal by the ordinary door instead of an arm no
    // input can reach.
    let end = at.saturating_add(len_u64(batch.len()));
    if end > n_valid {
        return Ok(None);
    }

    for (offset, bar) in batch.iter().enumerate() {
        if read(at.saturating_add(len_u64(offset)))? != *bar {
            return Ok(None);
        }
    }
    Ok(Some(at))
}

/// The first of `n_valid` committed records whose timestamp is at or after
/// `ts`, or `n_valid` when every one of them is older.
///
/// A bisection, because the records are strictly increasing in `ts_micros` —
/// that is what [`survey`] enforces at the write boundary and what
/// [`Header::advance`] enforces between batches, so it is a property of the
/// file and not an assumption made here.
///
/// The answer is an INSERTION POINT and is not claimed to hold `ts`: a caller
/// that needs that compares the record there, which [`already_stored`] does as
/// part of the comparison it was going to make anyway. Confirming it here
/// would be one more read for an answer the caller already computes.
///
/// # Errors
///
/// Whatever `read` refuses. A truncated file answers
/// [`StoreError::ShortRead`], which is the honest outcome: the bytes this
/// question is about are gone, and "not present" would be a claim about bytes
/// nobody can see.
fn first_at_or_after<W: Row, F>(n_valid: u64, ts: i64, read: F) -> Result<u64, StoreError>
where
    F: Fn(u64) -> Result<W, StoreError>,
{
    let mut low = 0u64;
    let mut high = n_valid;
    while low < high {
        // `low + (high - low) / 2` rather than `(low + high) / 2`: the second
        // overflows for a counter past half of `u64`, and `overflow-checks` is
        // on in both profiles, so that is a panic and not a wrong answer.
        // Saturating spellings on both, which cannot bite — `low < high`
        // makes the subtraction exact and the sum is below `high`.
        let mid = low.saturating_add(high.saturating_sub(low) / 2);
        if read(mid)?.stamp() < ts {
            // `mid < high`, so this cannot pass `n_valid` and cannot wrap.
            low = mid.saturating_add(1);
        } else {
            high = mid;
        }
    }
    Ok(low)
}

/// Writes a fresh header region: zeros, then the genesis commit.
///
/// The zeros are written rather than left as a hole, so the whole region is
/// allocated before the first record is. A file that cannot fit its own header
/// says so now, with [`StoreError::DiskFull`], rather than on the commit after
/// the first successful append.
fn initialise(
    dst: &File,
    path: &Path,
    symbol_id: u32,
    timeframe_secs: u32,
    // WHICH GEOMETRY THIS FILE IS BORN AT. A `.ovl` created at the bar's
    // geometry would accept 24-byte records at 56-byte offsets, and the header
    // would validate and the CRC would pass because the bytes written are the
    // bytes read.
    layout: Layout,
) -> Result<(), StoreError> {
    let genesis = Header::genesis_at(layout, symbol_id, timeframe_secs, 0);
    let commit = refused(genesis.commit(), path)?;
    let region = vec![0u8; REGION_LEN];
    write_fully(dst, path, 0, &region)?;
    write_fully(dst, path, commit.offset, &commit.bytes)?;
    fault(dst.sync_all(), path, Action::Sync)
}

/// Reads the header region: the committed header, and the largest record count
/// any intact slot in it claims.
///
/// Reads at most [`REGION_LEN`] bytes — never the whole file. A longer read
/// would let record bytes audition as header slots, which is the reason
/// [`Header::read_region`] bounds itself at [`MAX_SLOT_COUNT`] positions.
///
/// # Why the second number exists
///
/// [`Header::read_region`] does not always return the newest commit. When the
/// newest slot fails [`Header::validate`] it walks back to an older one and
/// returns **that**, which is the right recovery for a header that became
/// durable before the records it counts — `crate::header`'s crash table, row
/// four. The counter that comes back is therefore not always the largest one
/// the file's own bytes claim, and the gap between the two is the only thing
/// that separates an interrupted append from a truncation. The comparison and
/// the refusal live in `BarFile::validated`, next to the acceptance they are
/// the other half of; this function only carries the number over.
///
/// It costs a second decode of at most two 64-byte slots, once per open. It is
/// not a second *search*: no generation ordering, no slot-position rule and no
/// fallback are reproduced here — see [`highest_claim`].
fn read_header(src: &File, path: &Path, len: u64) -> Result<(Header, u64), StoreError> {
    let want = len.min(REGION_LEN_U64);
    let mut region = vec![0u8; usize::try_from(want).unwrap_or(REGION_LEN)];
    read_fully(src, path, 0, &mut region)?;
    let header = refused(Header::read_region(&region, len), path)?;
    Ok((header, highest_claim(&region)))
}

/// The largest `n_valid` any slot of the region still decodes as.
///
/// "Decodes" is the whole test, and it is not a weak one: the magic, a version
/// this build knows, the stride that version defines, and the slot's own CRC
/// over all sixty-four of its bytes. A slot that passes those was written by a
/// commit, so the counter it carries is evidence that records up to it were
/// once **published** — whatever a reader ends up trusting afterwards.
///
/// **Slot position is deliberately not checked here, and that direction is the
/// safe one.** [`Header::read_region`] discards a decodable slot sitting where
/// its own generation does not put it; this counts its claim anyway. The
/// consequence is that a file whose header slots are shuffled is refused rather
/// than opened — it is a corrupt file on either reading — and the alternative
/// is a second copy of `generation % slot_count` in this module, which is the
/// duplicated-check drift `BarFile::validated`'s own history is a monument to.
///
/// Zero when nothing decodes, and that is not a fallback that hides anything:
/// the only caller compares this against a counter it already holds, and a
/// region where no slot decodes never yields one — [`Header::read_region`]
/// refuses first, above.
///
/// The walk is bounded by arithmetic rather than by a `take`: the region was
/// read at [`REGION_LEN`] bytes at most and slots are [`SLOT_STRIDE`] apart, so
/// there are at most [`MAX_SLOT_COUNT`] chunks to begin with.
fn highest_claim(region: &[u8]) -> u64 {
    let stride = usize::try_from(SLOT_STRIDE).unwrap_or(REGION_LEN);
    region
        .chunks(stride)
        .filter_map(|slot| Header::decode(slot).ok())
        .map(|header| header.n_valid)
        .max()
        .unwrap_or(0)
}

/// Bytes the file holds past the extent the commit counter covers.
///
/// Exactly `len - layout.offset_of(n_valid)` for every file that got past
/// [`Header::validate`], written as two infallible calls instead: the whole
/// records the bytes can hold but the counter does not claim, plus the partial
/// record at the very end. See `BarFile::validated` for why the fallible
/// spelling was not taken.
///
/// Saturating on both arms, and neither is a disguised refusal. `n_valid`
/// above the capacity is the file `Header::validate` already refused as
/// [`FormatError::CounterExceedsFile`]; were it ever reached here it would
/// report only the partial record, which is a wrong number and not a wrong
/// decision, and the decision is made by the refusal upstream.
fn bytes_past_the_counter(layout: Layout, len: u64, n_valid: u64) -> u64 {
    layout
        .capacity_for(len)
        .saturating_sub(n_valid)
        .saturating_mul(layout.record_stride())
        .saturating_add(layout.ragged_tail_bytes(len))
}

/// An interrupted append still sitting past the counter, on the rolling log.
///
/// # What was invisible
///
/// Nothing, because the month did not open at all: this condition used to be
/// [`StoreError::RaggedTail`], and the operator's month was gone. Now the file
/// opens and the bytes are named — the path, the length, the counter that
/// bounds what is readable, and how many bytes lie past it — because
/// `docs/02-store-format.md` §7 says the discarded count is logged and
/// `CLAUDE.md` §4 admits degrading loudly, never quietly.
///
/// # Why `Warn` and not `Error`
///
/// The same argument [`crate::header`]'s fall-back reporter makes: nothing
/// failed. Every bar this file returns is real and was committed, and the
/// remainder was never published to anybody. It is worth a line and not worth
/// a refusal.
///
/// # What it costs
///
/// One event per **open of a file that has one**, never per record and never
/// on the ordinary path, where `discarded` is zero and this is not called. It
/// repeats on every reopen until an append covers those bytes, because nothing
/// here rewrites the file to make it stop.
fn note_tail_past_the_commit(path: &Path, len: u64, n_valid: u64, discarded: u64) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("store.open", "bytes past the commit counter")
            .with("file", telemetry::Value::Str(&path.display().to_string()))
            .with("file_len", telemetry::Value::Uint(len))
            .with("n_valid", telemetry::Value::Uint(n_valid))
            .with("discarded", telemetry::Value::Uint(discarded)),
    );
}

/// Flushes a directory, so a newly created file's **name** is durable.
///
/// Without it the bars can be on stable storage inside a file the directory
/// does not yet mention after a crash, which is the same as not having them.
fn fsync_dir(dir: &Path) -> Result<(), StoreError> {
    let handle = fault(File::open(dir), dir, Action::Open)?;
    fault(handle.sync_all(), dir, Action::Sync)
}

/// Opens for reading and writing, creating but never truncating.
fn open_rw(path: &Path) -> io::Result<File> {
    File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

/// A file, seen as the two positional calls this module makes of it.
///
/// It exists so the failure arms below can be **exercised**. `ENOSPC`,
/// `EROFS`, a partial write and a write that accepts nothing are all things a
/// developer's disk will not do on request, and a refusal path that no test
/// enters is a refusal path that is wrong the first time it runs. The
/// implementation for [`File`] is the two syscalls and nothing else.
trait Positional {
    /// One `pwrite`. Returns how many bytes the host accepted.
    fn put(&self, offset: u64, buf: &[u8]) -> io::Result<usize>;
    /// One `pread`. Returns how many bytes the host returned.
    fn get(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize>;
}

impl Positional for File {
    fn put(&self, offset: u64, buf: &[u8]) -> io::Result<usize> {
        FileExt::write_at(self, buf, offset)
    }

    fn get(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        FileExt::read_at(self, buf, offset)
    }
}

/// Writes every byte, resuming a partial write at the offset it stopped at.
///
/// A short write is ordinary POSIX behaviour and the answer to it is to
/// continue, not to fail. The failure is a write that accepts **zero** bytes:
/// there is no progress to resume from, and looping on it would hang, which is
/// the one failure a test suite cannot report.
fn write_fully<P: Positional>(
    dst: &P,
    path: &Path,
    offset: u64,
    bytes: &[u8],
) -> Result<(), StoreError> {
    let mut done = 0usize;
    while let Some(rest) = bytes.get(done..).filter(|rest| !rest.is_empty()) {
        // Saturating rather than checked: `offset` came from the geometry,
        // which already refused an overflow, and a saturated offset is one no
        // host accepts a write at — so it becomes a loud refusal by the arm
        // below rather than a wrong write, and there is no arm here that no
        // input can reach.
        let at = offset.saturating_add(len_u64(done));
        match dst.put(at, rest) {
            Ok(0) => {
                return Err(StoreError::ShortWrite {
                    path: path.to_path_buf(),
                    offset: at,
                    asked: rest.len(),
                    wrote: done,
                });
            }
            Ok(taken) => done = done.saturating_add(taken),
            Err(refusal) if refusal.kind() == ErrorKind::Interrupted => {}
            Err(refusal) => return Err(classify(path, Action::Write, &refusal)),
        }
    }
    Ok(())
}

/// Reads every byte, resuming a short read at the offset it stopped at.
///
/// A read that returns zero is end of file: the caller asked for bytes the
/// file's own header promised and the file does not have them, which means it
/// was truncated under this handle.
fn read_fully<P: Positional>(
    src: &P,
    path: &Path,
    offset: u64,
    buf: &mut [u8],
) -> Result<(), StoreError> {
    let mut done = 0usize;
    while let Some(rest) = buf.get_mut(done..).filter(|rest| !rest.is_empty()) {
        let at = offset.saturating_add(len_u64(done));
        let owed = rest.len();
        match src.get(at, rest) {
            Ok(0) => {
                return Err(StoreError::ShortRead {
                    path: path.to_path_buf(),
                    offset: at,
                    asked: owed,
                    read: done,
                });
            }
            Ok(got) => done = done.saturating_add(got),
            Err(refusal) if refusal.kind() == ErrorKind::Interrupted => {}
            Err(refusal) => return Err(classify(path, Action::Read, &refusal)),
        }
    }
    Ok(())
}

/// The one place an [`io::Error`] becomes a [`StoreError`].
///
/// One function rather than a closure at every call site: a closure per site
/// is a refusal arm per site, and most of them are arms the host will never
/// take on a developer's machine. Here there is one arm per *kind*, and the
/// tests below drive all of them.
fn classify(path: &Path, action: Action, refusal: &io::Error) -> StoreError {
    let path = path.to_path_buf();
    match refusal.kind() {
        ErrorKind::StorageFull => StoreError::DiskFull { path, action },
        ErrorKind::PermissionDenied => StoreError::Denied { path, action },
        ErrorKind::ReadOnlyFilesystem => StoreError::ReadOnly { path, action },
        ErrorKind::IsADirectory => StoreError::IsADirectory { path, action },
        ErrorKind::NotADirectory => StoreError::NotADirectory { path, action },
        ErrorKind::NotFound => StoreError::Missing { path, action },
        kind => StoreError::Io {
            path,
            action,
            kind,
            code: refusal.raw_os_error(),
        },
    }
}

/// A failed lock attempt, named.
///
/// `WouldBlock` is the whole point of the lock and is not an I/O error; every
/// other reason is.
fn lock_fault(path: &Path, refusal: TryLockError) -> StoreError {
    match refusal {
        TryLockError::WouldBlock => StoreError::Locked {
            path: path.to_path_buf(),
        },
        TryLockError::Error(host) => classify(path, Action::Lock, &host),
    }
}

/// Lifts an [`io::Result`] into this module's errors.
fn fault<T>(result: io::Result<T>, path: &Path, action: Action) -> Result<T, StoreError> {
    match result {
        Ok(value) => Ok(value),
        Err(refusal) => Err(classify(path, action, &refusal)),
    }
}

/// Lifts a [`FormatError`] into this module's errors, naming the file.
fn refused<T>(result: Result<T, FormatError>, path: &Path) -> Result<T, StoreError> {
    match result {
        Ok(value) => Ok(value),
        Err(source) => Err(StoreError::Format {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// A length as a `u64`, without a cast this workspace denies.
///
/// The saturating answer is only reachable on a target where `usize` is wider
/// than 64 bits, and there it is a length no file has — every caller either
/// compares it against a real length or offers it to the host, both of which
/// refuse loudly. Same argument as [`crate::block`]'s byte count.
fn len_u64(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::{
        Action, Appended, Bar, BarFile, FormatError, Layout, Positional, StoreError,
        already_stored, bytes_past_the_counter, classify, first_at_or_after, initialise, len_u64,
        lock_fault, open_rw, read_fully, write_fully,
    };
    use crate::format::OI_NULL;
    use std::fs::TryLockError;
    use std::io::{self, ErrorKind};
    use std::path::{Path, PathBuf};

    /// What the fake host should do on the next call.
    #[derive(Debug, Clone, Copy)]
    enum Step {
        /// Accept or return this many bytes.
        Take(usize),
        /// Fail with this kind.
        Fail(ErrorKind),
    }

    /// A host that does exactly what a test tells it to.
    ///
    /// This is a fake and it is used **only** where the real condition cannot
    /// be produced: no developer disk fills on request, no filesystem becomes
    /// read-only on request, and no regular file accepts zero bytes on
    /// request. Every condition that *can* be made — a directory where a file
    /// belongs, a read-only directory, a truncated file, a second writer — is
    /// made for real in `crates/store/tests/write.rs`.
    struct Script {
        steps: RefCell<Vec<Step>>,
        seen: RefCell<Vec<(u64, usize)>>,
    }

    impl Script {
        fn new(steps: &[Step]) -> Self {
            Self {
                steps: RefCell::new(steps.iter().rev().copied().collect()),
                seen: RefCell::new(Vec::new()),
            }
        }

        fn next(&self, offset: u64, len: usize) -> io::Result<usize> {
            self.seen.borrow_mut().push((offset, len));
            match self.steps.borrow_mut().pop() {
                Some(Step::Take(n)) => Ok(n.min(len)),
                Some(Step::Fail(kind)) => Err(io::Error::from(kind)),
                None => Ok(len),
            }
        }
    }

    impl Positional for Script {
        fn put(&self, offset: u64, buf: &[u8]) -> io::Result<usize> {
            self.next(offset, buf.len())
        }

        fn get(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
            self.next(offset, buf.len())
        }
    }

    fn path() -> &'static Path {
        Path::new("/tmp/brutex-store-fake.bin")
    }

    #[test]
    fn a_partial_write_resumes_at_the_offset_it_stopped_at() {
        // Two scripted partials, and then a host that takes whatever is left:
        // the loop must not depend on how the remainder is handed back.
        let host = Script::new(&[Step::Take(3), Step::Take(2)]);
        assert_eq!(write_fully(&host, path(), 100, &[7u8; 10]), Ok(()));
        assert_eq!(
            *host.seen.borrow(),
            vec![(100, 10), (103, 7), (105, 5)],
            "each resume starts where the last one stopped"
        );
    }

    #[test]
    fn a_write_that_accepts_nothing_is_refused_rather_than_retried_forever() {
        let host = Script::new(&[Step::Take(4), Step::Take(0)]);
        assert_eq!(
            write_fully(&host, path(), 0, &[1u8; 9]),
            Err(StoreError::ShortWrite {
                path: path().to_path_buf(),
                offset: 4,
                asked: 5,
                wrote: 4,
            })
        );
    }

    #[test]
    fn an_interrupted_write_is_retried_at_the_same_offset() {
        let host = Script::new(&[Step::Fail(ErrorKind::Interrupted), Step::Take(6)]);
        assert_eq!(write_fully(&host, path(), 8, &[2u8; 6]), Ok(()));
        assert_eq!(*host.seen.borrow(), vec![(8, 6), (8, 6)]);
    }

    #[test]
    fn a_full_disk_mid_write_is_returned_and_never_signalled() {
        // The whole argument for banning a writable mapping, as a value: an
        // ordinary write RETURNS this. A mapping would have raised SIGBUS.
        let host = Script::new(&[Step::Take(2), Step::Fail(ErrorKind::StorageFull)]);
        assert_eq!(
            write_fully(&host, path(), 32_768, &[3u8; 56]),
            Err(StoreError::DiskFull {
                path: path().to_path_buf(),
                action: Action::Write,
            })
        );
    }

    #[test]
    fn an_empty_write_touches_the_host_at_all() {
        let host = Script::new(&[]);
        assert_eq!(write_fully(&host, path(), 0, &[]), Ok(()));
        assert!(
            host.seen.borrow().is_empty(),
            "nothing to write means no syscall"
        );
    }

    #[test]
    fn a_short_read_resumes_and_a_read_of_nothing_is_end_of_file() {
        let host = Script::new(&[Step::Take(2), Step::Take(3)]);
        let mut buf = [0u8; 5];
        assert_eq!(read_fully(&host, path(), 16, &mut buf), Ok(()));
        assert_eq!(*host.seen.borrow(), vec![(16, 5), (18, 3)]);

        let host = Script::new(&[Step::Take(2), Step::Take(0)]);
        let mut buf = [0u8; 56];
        assert_eq!(
            read_fully(&host, path(), 32_768, &mut buf),
            Err(StoreError::ShortRead {
                path: path().to_path_buf(),
                offset: 32_770,
                asked: 54,
                read: 2,
            })
        );
    }

    #[test]
    fn an_interrupted_read_is_retried_and_a_refused_read_is_classified() {
        let host = Script::new(&[Step::Fail(ErrorKind::Interrupted), Step::Take(4)]);
        let mut buf = [0u8; 4];
        assert_eq!(read_fully(&host, path(), 0, &mut buf), Ok(()));

        let host = Script::new(&[Step::Fail(ErrorKind::PermissionDenied)]);
        let mut buf = [0u8; 4];
        assert_eq!(
            read_fully(&host, path(), 0, &mut buf),
            Err(StoreError::Denied {
                path: path().to_path_buf(),
                action: Action::Read,
            })
        );
    }

    #[test]
    fn every_classified_kind_gets_its_own_name() {
        // Two of these — a full disk and a read-only mount — cannot be
        // produced on a developer's machine without privileged mounting, so
        // this is where the mapping is checked. It proves the classifier, not
        // the kernel: `crates/store/tests/write.rs` produces the other five
        // for real.
        let cases = [
            (
                ErrorKind::StorageFull,
                StoreError::DiskFull {
                    path: path().to_path_buf(),
                    action: Action::Write,
                },
            ),
            (
                ErrorKind::PermissionDenied,
                StoreError::Denied {
                    path: path().to_path_buf(),
                    action: Action::Write,
                },
            ),
            (
                ErrorKind::ReadOnlyFilesystem,
                StoreError::ReadOnly {
                    path: path().to_path_buf(),
                    action: Action::Write,
                },
            ),
            (
                ErrorKind::IsADirectory,
                StoreError::IsADirectory {
                    path: path().to_path_buf(),
                    action: Action::Write,
                },
            ),
            (
                ErrorKind::NotADirectory,
                StoreError::NotADirectory {
                    path: path().to_path_buf(),
                    action: Action::Write,
                },
            ),
            (
                ErrorKind::NotFound,
                StoreError::Missing {
                    path: path().to_path_buf(),
                    action: Action::Write,
                },
            ),
        ];
        for (kind, want) in cases {
            assert_eq!(
                classify(path(), Action::Write, &io::Error::from(kind)),
                want,
                "{kind:?}"
            );
        }

        // Anything unclassified keeps the kind and the errno rather than
        // collapsing to "I/O error".
        let other = io::Error::from_raw_os_error(9);
        assert_eq!(
            classify(path(), Action::Sync, &other),
            StoreError::Io {
                path: path().to_path_buf(),
                action: Action::Sync,
                kind: other.kind(),
                code: Some(9),
            }
        );
    }

    #[test]
    fn a_lock_that_would_block_is_a_held_month_and_nothing_else_is() {
        assert_eq!(
            lock_fault(path(), TryLockError::WouldBlock),
            StoreError::Locked {
                path: path().to_path_buf(),
            }
        );
        assert_eq!(
            lock_fault(
                path(),
                TryLockError::Error(io::Error::from(ErrorKind::PermissionDenied))
            ),
            StoreError::Denied {
                path: path().to_path_buf(),
                action: Action::Lock,
            }
        );
    }

    #[test]
    fn a_length_crosses_widths_without_a_cast() {
        assert_eq!(len_u64(0), 0);
        assert_eq!(len_u64(56), 56);
        assert_eq!(len_u64(usize::MAX), u64::MAX);
    }

    // =======================================================================
    // The two doors' shared check, and the duplicate check behind `append`
    //
    // Two of the five below touch a REAL filesystem, unlike everything above,
    // and the reason is the one the `Script` fake's own comment gives: an
    // interrupted append and a month re-pulled from its middle are conditions
    // that CAN be made, so they are made rather than simulated. They go
    // through `BarFile::validated` — the door `open_or_create` and
    // `open_existing` both funnel into — with no directory tree and no
    // advisory lock, because neither is what is under test here and rendering
    // a `StorePath` to get one would put the path module in the failure
    // surface of a question about bytes.
    //
    // The other three take a reader as a parameter and never open anything.
    // That is the same trade `Positional` makes above: a refusal arm no test
    // enters is a refusal arm that is wrong the first time it runs, and "the
    // record at index 3 is gone" is not a state a developer's disk produces on
    // request.
    // =======================================================================

    /// The symbol id every month below is written and reopened under.
    const SYMBOL: u32 = 26_000;

    /// One minute, in microseconds.
    const MINUTE: i64 = 60_000_000;

    /// The open of the first one-minute bar of 2024-06-03, in microseconds.
    const T0: i64 = 1_717_386_300_000_000;

    /// The `index`-th one-minute bar of the session, in paisa.
    fn bar(index: i64) -> Bar {
        Bar {
            ts_micros: T0 + index * MINUTE,
            open: 2_345_600 + index,
            high: 2_345_900 + index,
            low: 2_345_100 + index,
            close: 2_345_700 + index,
            volume: 1_000 + index,
            open_interest: OI_NULL,
        }
    }

    /// A temporary month no other live process will name.
    ///
    /// The process id is not a random number and is not meant to be: it is
    /// unique among *live* processes, which is exactly the set that can
    /// collide. The tag separates the tests in this binary, which `cargo test`
    /// runs in parallel threads over one temp directory.
    fn scratch(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "brutex-store-file-{tag}-{}.bin",
            std::process::id()
        ))
    }

    /// A fresh month, initialised and open.
    fn month(tag: &str) -> (PathBuf, BarFile) {
        let path = scratch(tag);
        let _ignored = std::fs::remove_file(&path);
        let bars = open_rw(&path).expect("a temp path opens");
        initialise(&bars, &path, SYMBOL, 60, Layout::CURRENT)
            .expect("a fresh header region is written");
        (path.clone(), reopen(&path).expect("and reads back"))
    }

    /// Reopens a month at whatever length it now has.
    fn reopen(path: &Path) -> Result<BarFile, StoreError> {
        let bars = open_rw(path).expect("the month is there");
        let len = bars.metadata().expect("a length").len();
        BarFile::validated(
            bars,
            path.to_path_buf(),
            None,
            len,
            SYMBOL,
            60,
            Layout::KNOWN,
        )
    }

    #[test]
    fn the_discarded_count_is_the_bytes_past_the_counter() {
        // The identity `bytes_past_the_counter` is written to satisfy, against
        // the `offset_of` it deliberately does not call. Both ends of every
        // record and the whole of a checksum block, so a remainder that is a
        // whole number of records is not confused with a torn one.
        let v2 = Layout::V2;
        for n_valid in [0u64, 1, 72, 73, 74, 375] {
            let committed_end = v2.offset_of(n_valid).expect("an offset inside u64");
            for extra in [0u64, 1, 17, 55, 56, 57, 112, 4_088] {
                assert_eq!(
                    bytes_past_the_counter(v2, committed_end + extra, n_valid),
                    extra,
                    "n_valid {n_valid}, {extra} bytes past its end"
                );
            }
        }
    }

    #[test]
    fn a_torn_tail_past_the_commit_counter_opens_the_month_rather_than_bricking_it() {
        // THIS TEST WRITES TO A LOG IT DOES NOT OWN, AND IT IS NOT ALONE.
        //
        // The four production calls below reach two emit sites — two commits
        // are two `store.append` records, two reopens of a file with a torn
        // tail are two `store.open` records — and they go to whatever sink the
        // *process* has installed, because `telemetry::install` writes a
        // `OnceLock` and `cargo test` runs this crate's unit tests as one
        // process on N threads.
        //
        // The only thing that installs a sink in this binary is
        // `crate::emits`, whose test asserts that the file holds EXACTLY one
        // record per emit site in the crate. Running beside it, these four
        // records landed inside that count: five consecutive runs gave 12, 8,
        // 11, 12 and 12 records against six sites, and `--test-threads=1` gave
        // six every time.
        //
        // Neither test is wrong about the crate. They are two tests sharing one
        // global, and `hold_the_sink` makes their windows disjoint. **Any
        // future test in this binary that reaches a `telemetry::emit` must take
        // it too** — otherwise that count goes back to depending on the
        // scheduler, and it fails on a machine with a different core count
        // rather than on a defect.
        let _sink_is_mine = crate::emits::hold_the_sink();

        let (path, mut file) = month("torn");
        assert_eq!(
            file.append(&[bar(0), bar(1), bar(2)]),
            Ok(Appended::Committed {
                first_index: 0,
                n_valid: 3,
            })
        );
        drop(file);

        // 17 bytes of a fourth record whose header slot never got written —
        // the crash `docs/02-store-format.md` §7 describes. This used to be
        // `StoreError::RaggedTail`, and the three committed bars below were
        // unreachable to every process from then on, permanently: §3 rule 8
        // forbids rewriting the file to clear it.
        let mut bytes = std::fs::read(&path).expect("the month reads");
        bytes.extend_from_slice(&[9u8; 17]);
        std::fs::write(&path, &bytes).expect("the month writes");
        let len = u64::try_from(bytes.len()).expect("a length fits u64");

        let reopened = reopen(&path).expect("a month is not lost to bytes no commit claims");
        assert_eq!(reopened.records(), 3, "the counter is untouched");
        assert_eq!(reopened.read_record(2), Ok(bar(2)), "and so are the bars");
        assert_eq!(
            bytes_past_the_counter(reopened.layout(), len, 3),
            17,
            "and the discarded count the log carries is the torn remainder"
        );

        drop(reopened);

        // The next append lands at `offset_of(n_valid)` and covers them, which
        // is the whole reason ignoring them is safe rather than merely quiet.
        let mut writer = reopen(&path).expect("reopens");
        assert_eq!(
            writer.append(&[bar(3), bar(4)]),
            Ok(Appended::Committed {
                first_index: 3,
                n_valid: 5,
            })
        );
        assert_eq!(writer.read_record(3), Ok(bar(3)), "over the torn bytes");
        drop(writer);
        let grown = std::fs::metadata(&path).expect("a length").len();
        assert_eq!(
            bytes_past_the_counter(Layout::V2, grown, 5),
            0,
            "and nothing is left past the counter"
        );
        let _ignored = std::fs::remove_file(&path);
    }

    #[test]
    fn a_re_pull_from_the_middle_of_a_month_is_already_present_not_a_conflict() {
        // Two of the appends below commit, so two `store.append` records go to
        // the process-wide sink `crate::emits` installs and counts. See the
        // torn-tail test above for what that cost and why the lock is the fix;
        // the three `AlreadyPresent` appends emit nothing, which is itself a
        // silence that test asserts.
        let _sink_is_mine = crate::emits::hold_the_sink();

        let (path, mut file) = month("middle");
        let held: Vec<Bar> = (0..20).map(bar).collect();
        assert_eq!(
            file.append(&held),
            Ok(Appended::Committed {
                first_index: 0,
                n_valid: 20,
            })
        );

        // FIVE BARS FROM THE MIDDLE OF THE MONTH. Refused before this change,
        // as `TimestampsOutOfOrder`: the check compared them against records
        // 15..=19 — the file's tail — found them different, and reported a
        // vendor restating history for bars the file already held.
        assert_eq!(
            file.append(&held[5..10]),
            Ok(Appended::AlreadyPresent {
                first_index: 5,
                n_valid: 20,
            }),
            "a day re-pulled from inside a backfilled month"
        );
        // The tail still answers, and still names its own index.
        assert_eq!(
            file.append(&held[15..]),
            Ok(Appended::AlreadyPresent {
                first_index: 15,
                n_valid: 20,
            })
        );
        // So does the whole month, offered again.
        assert_eq!(
            file.append(&held),
            Ok(Appended::AlreadyPresent {
                first_index: 0,
                n_valid: 20,
            })
        );
        assert_eq!(file.records(), 20, "and not one of the three wrote a byte");

        // A BAR THE MONTH HOLDS DIFFERENTLY IS STILL A CONFLICT, from the
        // middle exactly as from the tail. Locating the batch is not the same
        // as trusting it.
        let mut altered = held[5..10].to_vec();
        altered[2].close += 1;
        assert_eq!(
            file.append(&altered),
            Err(StoreError::Format {
                path: path.clone(),
                source: FormatError::TimestampsOutOfOrder {
                    previous: bar(19).ts_micros,
                    next: bar(5).ts_micros,
                },
            }),
            "a vendor restating history is refused, not absorbed"
        );

        // And a batch that starts inside the month and runs past its end is a
        // resume: the duplicate check declines it — it is not WHOLLY held —
        // and only the part that follows is written.
        let resumed: Vec<Bar> = (15..25).map(bar).collect();
        assert_eq!(
            file.append(&resumed),
            Ok(Appended::Committed {
                first_index: 20,
                n_valid: 25,
            }),
            "five duplicates and five new bars, and only the five landed"
        );
        let _ignored = std::fs::remove_file(&path);
    }

    #[test]
    fn locating_a_batch_costs_a_bisection_and_not_a_scan() {
        // The cost claim in `already_stored`'s doc, asserted as a number
        // rather than written down. A scan for record 900 of 1,024 would read
        // 901 records; eleven is the ceiling of log2(1024) plus the step that
        // closes an interval of one.
        let held: Vec<Bar> = (0..1_024).map(bar).collect();
        let reads = Cell::new(0u32);
        let read = |index: u64| -> Result<Bar, StoreError> {
            reads.set(reads.get() + 1);
            Ok(held[usize::try_from(index).expect("an index fits a usize")])
        };

        assert_eq!(first_at_or_after(1_024, bar(900).ts_micros, read), Ok(900));
        assert!(
            reads.get() <= 11,
            "1,024 records is at most eleven probes, not {}",
            reads.get()
        );

        // The three edges, each of which `already_stored` reads differently.
        assert_eq!(first_at_or_after(1_024, bar(0).ts_micros, read), Ok(0));
        assert_eq!(
            first_at_or_after(1_024, bar(1_023).ts_micros + 1, read),
            Ok(1_024),
            "every held bar is older: the insertion point is past the end"
        );
        assert_eq!(
            first_at_or_after(1_024, bar(500).ts_micros + 1, read),
            Ok(501),
            "a timestamp between two records is not claimed to be either"
        );

        reads.set(0);
        assert_eq!(first_at_or_after(0, T0, read), Ok(0));
        assert_eq!(reads.get(), 0, "an empty month is not probed at all");
    }

    #[test]
    fn a_read_that_refuses_mid_comparison_comes_back_out_rather_than_answering_no() {
        // "The month does not hold these" and "I could not look" are different
        // answers, and only the first lets `append` fall through to a write.
        // Record 3 is the one the bisection does not touch on the way to
        // record 1 — it probes 2, 1, 0 — so this refusal can only come from
        // the comparison loop.
        let held: Vec<Bar> = (0..4).map(bar).collect();
        let read = |index: u64| -> Result<Bar, StoreError> {
            if index == 3 {
                return Err(StoreError::NotCommitted {
                    index: 3,
                    n_valid: 4,
                });
            }
            Ok(held[usize::try_from(index).expect("an index fits a usize")])
        };

        assert_eq!(
            already_stored(&held[1..4], bar(1).ts_micros, 4, read),
            Err(StoreError::NotCommitted {
                index: 3,
                n_valid: 4,
            }),
            "the read's refusal travels, and is not flattened into None"
        );
    }
}
