//! The tick-store writer, in the workspace and in pure Rust (E2E-02): one
//! vendor day zip becomes one `BRTXTS01` day file (`.bts`) under the tick
//! store's root, verified entry by entry against the zip before the day is
//! held.
//!
//! # What it writes, and what it does not change
//!
//! Version 1 of the format exactly as `tickstore-data/FORMAT.md` freezes it
//! (§3 container, index, footer and CRC; §4 block kinds and the columnar
//! transform): the bytes a reader sees are the format's, so
//! [`crate::gdfl_tickstore`] reads a day file from this writer and one from
//! the separate tool alike. Only the zstd encoder differs. The tool links C
//! zstd and writes level 9; that binding is what `CLAUDE.md` §2 forbids, so
//! nothing of it is ported. Every frame here is written by `ruzstd` 0.8.2,
//! the workspace's one zstd crate (D-2801), at its `Fastest` level, the only
//! compressing level that crate implements (`Default`, `Better` and `Best`
//! are `unimplemented!()` in its `encoding/frame_compressor.rs`). Any valid
//! frame decodes in any decoder, so the file is the format's; it is NOT the
//! tool's bytes, and identity is therefore defined on the decoded entries
//! (name, size, CRC-32, DOS time and rebuilt bytes), never on `.bts` bytes
//! (FM-249). [`WRITER`] names this writer on every log line and event.
//!
//! No memory mapping (a mapped file raises a signal when it is cut short
//! under the reader, `CLAUDE.md` §4, FM-250): the zip and the day file are
//! read with positional reads through [`crate::gdfl_archive::ReadAt`]. No
//! `libc`: the lock is `std::fs::File::try_lock`. No new dependency.
//!
//! # The protocol (E2E-02 item 2)
//!
//! 1. The zip's own name must be a canonical day zip name, and the day file
//!    goes exactly where the readers open it ([`DayIdentity::path_under`]):
//!    a conflict copy such as `… (1).zip` is refused by name rather than
//!    written under a name no reader opens (FM-024).
//! 2. The zip is read through [`crate::gdfl_archive`], the one zip authority
//!    (FM-238), and every entry is checked before anything is written: names
//!    UTF-8 and unique, the inner day folder the file name states, each
//!    member inflated and held to its central size and CRC-32. A bad CRC is
//!    refused and NOTHING is written.
//! 3. Every member is encoded in memory, in parallel, and each block is
//!    rebuilt through [`crate::gdfl_tickstore::rebuild`] and compared before
//!    it is kept.
//! 4. The tick store's root must already be a directory: a mistyped root is
//!    refused `root_missing`, never made, since a root no reader opens would
//!    hold days nobody finds. The day file is written to
//!    `<day>.bts.tmp.<run-id>`, created through `store::staging::create`, the
//!    workspace's one door for a temporary (R3-16, gate 28), fsynced, and
//!    then EVERY entry is read back from that file through
//!    [`crate::gdfl_tickstore`] and compared byte for byte with the zip's
//!    member, read again. Only then is the temporary renamed to `<day>.bts`
//!    and the directory fsynced: only then is the day held. A year or month
//!    folder made for the day has its parent fsynced first, so a power loss
//!    cannot take back the folder of a day a log line called written.
//! 5. A `<day>.bts.tmp*` left by a crash, a full disk or a failed verify is
//!    refused BY NAME and LEFT: this code removes no file by choice (the
//!    operator's deletion list owns that, R4-06). The staging door clears a
//!    one-name regular file at the name it is handed; the leftover check runs
//!    first, under the root's lock, so that path is reached only when a file
//!    appears at this run's own name between the check and the create, and
//!    then the run stops `temp_raced`, naming the bytes the door cleared
//!    (D-4967). A held day is never rewritten: a rerun verifies it and writes
//!    nothing; a held day that no longer matches its zip is refused with the
//!    old file untouched, and a zip that no longer reads back as its own
//!    records state is refused `zip_corrupt_on_verify`, never blamed on the
//!    held day.
//! 6. The zip and a held day file are opened through `store::safe_open`, and
//!    the lock and the log only after a look that does not follow a link: a
//!    link, a pipe, a socket, a device or a directory at any of those names
//!    is refused `not_regular_file`, never followed and never waited on.
//!
//! Every outcome, written, held or refused, is one line of the store's
//! append-only [`LOG_NAME`] (fsynced) and one `pull.tickstore.write` event,
//! each naming the writer, the commit of the code that ran (`unstamped` when
//! the caller has none) and the run.
//!
//! # Cost
//!
//! O(the day's bytes) to write and to verify, by design: every byte is read,
//! encoded, written and read back once. Memory is the day's compressed
//! blocks plus one member per worker, bounded by [`DAY_BLOCKS_CAP`] and
//! [`MEMBER_CAP`], each refused by name past its bound.

use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use ruzstd::encoding::{CompressionLevel, compress_to_vec};

use crate::gdfl_archive::{ReadAt, ZipEntry, crc32, member_bytes, zip_entries};
use crate::gdfl_cm::{CmKind, CmRefusal, folder_day};
use crate::gdfl_nfo::{FOLDER_PREFIX as NFO_FOLDER_PREFIX, NfoTickStore};
use crate::gdfl_tickstore::{IndexEntry, TickStore, read_index, rebuild};
use crate::session::Day;
use store::safe_open::{Found, ReadRefusal};

/// This writer, named on every log line and event: which code and which
/// encoder produced a day file (FM-249). The separate tool's files are
/// `C zstd level 9`; a store holding both lists both.
pub const WRITER: &str = "pull::tickstore_write/1 ruzstd-0.8.2 Fastest";

/// The append-only log of every outcome, one line each, in the tick store's
/// root: the day file, the writer, the run, the counts, the verdict.
pub const LOG_NAME: &str = "tickstore-write.log";

/// The commit named on a log line and an event when the caller has none.
pub const UNSTAMPED: &str = "unstamped";

/// The root's lock file, the same name the separate tool locks, so the two
/// writers never run on one root at once. It is opened, never removed.
pub const LOCK_NAME: &str = ".lock";

/// The largest member encoded, 2 GiB: one member and its columns are held in
/// memory by each worker, so a bigger one is refused by name before it is
/// read. The largest member of the three measured real days is far below it
/// (`docs/06-limits.md`).
pub const MEMBER_CAP: u64 = 2 << 30;

/// The most compressed block bytes one day may hold in memory before its file
/// is written, 4 GiB: about 28 times the largest day file the tool wrote for
/// September 2026 (151,027,298 bytes, `GFDLNFO_TICK_29092026.bts`).
pub const DAY_BLOCKS_CAP: u64 = 4 << 30;

/// The first eight bytes of a day file (§3).
const MAGIC: &[u8; 8] = b"BRTXTS01";
/// The last eight bytes of a day file (§3).
const FOOTER_MAGIC: &[u8; 8] = b"BRTXTSE1";
/// What follows a day file's name in its temporary's name.
const TMP_SUFFIX: &str = ".bts.tmp";

/// Block kinds (§3).
const KIND_DIR: u8 = 0;
/// A columnar CSV block (§4).
const KIND_COLUMNAR: u8 = 1;
/// A raw block: one zstd frame of the bytes (§4).
const KIND_RAW: u8 = 2;

/// Shape bits of a numeric field (§4): the decimals, the sign, the time.
const SHAPE_NEG: u8 = 0x10;
/// `HH:MM:SS`.
const SHAPE_TIME: u8 = 0x20;

/// Which vendor tree a day zip belongs to (§2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tree {
    /// `GFDLNFO_TICK_<ddmmyyyy>`: options.
    Options,
    /// `GFDLCM_STOCK_TICK_<ddmmyyyy>` or `GFDLCM_INDICES_TICK_<ddmmyyyy>`.
    Cm(CmKind),
}

/// The day a canonical day zip name states, and where its day file lies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayIdentity {
    /// The tree.
    pub tree: Tree,
    /// The trade day.
    pub day: Day,
    /// The day folder's name, the zip's stem: `GFDLNFO_TICK_29092026`.
    pub stem: String,
}

impl DayIdentity {
    /// The identity a day zip's FILE NAME states: exactly a tree's day folder
    /// for a real date plus `.zip`, and nothing else.
    ///
    /// # Errors
    ///
    /// [`Refusal::NameNotCanonical`] for any other name, a conflict copy
    /// (`… (1).zip`) and a `.ZIP` among them.
    pub fn of_zip_name(name: &str) -> Result<Self, Refusal> {
        let refuse = || Refusal::NameNotCanonical {
            name: name.to_owned(),
        };
        let stem = name.strip_suffix(".zip").ok_or_else(refuse)?;
        let (tree, day) = if let Some(date) = stem.strip_prefix(NFO_FOLDER_PREFIX) {
            (Tree::Options, ddmmyyyy(date).ok_or_else(refuse)?)
        } else {
            let (kind, day) = folder_day(stem).map_err(|_| refuse())?;
            (Tree::Cm(kind), day)
        };
        let identity = Self {
            tree,
            day,
            stem: stem.to_owned(),
        };
        // The readers' own folder name for that day must be the stem itself:
        // `GFDLNFO_TICK_1092026x` or a padded date never reaches here.
        if identity.canonical_stem() == stem {
            Ok(identity)
        } else {
            Err(refuse())
        }
    }

    /// The readers' day folder name for this tree and day.
    fn canonical_stem(&self) -> String {
        match self.tree {
            Tree::Options => crate::gdfl_nfo::day_folder_name(self.day),
            Tree::Cm(kind) => crate::gdfl_cm::day_folder_name(kind, self.day),
        }
    }

    /// Where this day's file lies under the tick store's `root`: the path
    /// the readers themselves open ([`TickStore::day_path`],
    /// [`NfoTickStore::day_path`]), so there is one path rule, theirs.
    #[must_use]
    pub fn path_under(&self, root: &Path) -> PathBuf {
        match self.tree {
            Tree::Options => NfoTickStore::new(root).day_path(self.day),
            Tree::Cm(kind) => TickStore::new(root).day_path(kind, self.day),
        }
    }
}

/// `ddmmyyyy` as a real date.
fn ddmmyyyy(text: &str) -> Option<Day> {
    let &[d1, d2, m1, m2, y1, y2, y3, y4] = text.as_bytes() else {
        return None;
    };
    let digit = |b: u8| b.is_ascii_digit().then(|| b - b'0');
    let dd = digit(d1)? * 10 + digit(d2)?;
    let mm = digit(m1)? * 10 + digit(m2)?;
    let yyyy = u16::from(digit(y1)?) * 1_000
        + u16::from(digit(y2)?) * 100
        + u16::from(digit(y3)?) * 10
        + u16::from(digit(y4)?);
    Day::new(yyyy, mm, dd).ok()
}

/// Why a day was not written or not held. Every variant is named by
/// [`Refusal::code`] on its log line and event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The zip's file name is not a canonical day zip name.
    NameNotCanonical {
        /// The name.
        name: String,
    },
    /// The run id is not 1 to 64 of `[A-Za-z0-9_-]`.
    RunIdInvalid {
        /// The id.
        run_id: String,
    },
    /// The commit stamp is not 1 to 64 of `[A-Za-z0-9_-]`: it could not be
    /// one field of a log line.
    CommitInvalid {
        /// The stamp.
        commit: String,
    },
    /// The tick store's root is not an existing directory. The writer never
    /// makes a root: a mistyped one would hold days no reader opens.
    RootMissing {
        /// The root.
        root: PathBuf,
        /// What is there instead, or the host's refusal.
        what: String,
    },
    /// A name the writer opens (the zip, a held day file, the lock or the
    /// log) is not a regular file. It is never followed or waited on, and it
    /// is left as it is.
    NotRegularFile {
        /// The name.
        path: PathBuf,
        /// What is there.
        found: Found,
    },
    /// A folder between the tick store's root and the day file is a symbolic
    /// link. The root may be one (where the operator keeps the store is the
    /// operator's to place, as `store::safe_open::refuse_linked_store_root`
    /// says), but a folder below it never is written through: a test's
    /// shadow whose `options` or `cm` is a link would write the real store.
    LinkedFolder {
        /// The link.
        path: PathBuf,
    },
    /// The zip could not be opened or its length read.
    ZipUnavailable {
        /// The zip.
        path: PathBuf,
        /// The error's kind.
        kind: std::io::ErrorKind,
    },
    /// The zip reader refused the zip.
    ZipRefused {
        /// The reader's refusal.
        why: CmRefusal,
    },
    /// An entry name that is not UTF-8: the workspace's readers decode names
    /// lossily, so two such names could not be told apart (FM-248).
    NameNotUtf8 {
        /// The raw name, hex.
        raw_hex: String,
    },
    /// An entry name longer than the index's `u16` length.
    NameTooLong {
        /// Its length.
        len: usize,
    },
    /// One name twice in the zip: the index's name lookup needs unique names.
    DuplicateEntry {
        /// The name.
        name: String,
    },
    /// The zip holds no entry in the day folder its file name states, or an
    /// entry in another day's folder.
    InnerFolderMismatch {
        /// The folder the file name states.
        stem: String,
        /// The first folder that disagrees, or nothing found.
        found: String,
    },
    /// The zip holds its day folder's directory entry and no file in it: a
    /// day with no file is not held.
    NoFileInDay {
        /// The day folder.
        stem: String,
    },
    /// More entries than the footer's `u32` count.
    TooManyEntries {
        /// How many.
        entries: usize,
    },
    /// A member over [`MEMBER_CAP`], refused before it is read.
    MemberTooLarge {
        /// The member.
        name: String,
        /// Its stated size.
        size: u64,
    },
    /// A member that does not read back as its central record states: a bad
    /// CRC-32, a wrong length or a stream that does not inflate.
    MemberCorrupt {
        /// The member.
        name: String,
        /// What was wrong.
        why: String,
    },
    /// The encoded block did not rebuild to the member's bytes, or the
    /// encoder failed.
    EncodeFailed {
        /// The member.
        name: String,
        /// What was wrong.
        why: String,
    },
    /// The day's compressed blocks pass [`DAY_BLOCKS_CAP`].
    DayTooLarge {
        /// The bytes reached.
        bytes: u64,
    },
    /// Another writer holds the root's lock.
    DayLocked {
        /// The lock file.
        lock: PathBuf,
    },
    /// The lock file could not be opened or locked.
    LockFailed {
        /// The lock file.
        lock: PathBuf,
        /// The error's kind.
        kind: std::io::ErrorKind,
    },
    /// A `<day>.bts.tmp*` from an earlier run is there. It is left as it is
    /// for the operator's deletion list; the day is not written until it is
    /// gone.
    StaleTemp {
        /// Each leftover's name and length.
        leftovers: Vec<(String, u64)>,
    },
    /// The held day file does not match the zip; it is left as it is.
    HeldDayDiffers {
        /// The day file.
        path: PathBuf,
        /// The first difference.
        why: String,
    },
    /// A member of the zip did not read back as the zip's own record states
    /// while a held day was verified against it: the zip is the damaged
    /// side, and the held day file is left as it is and not blamed.
    ZipCorruptOnVerify {
        /// The held day file.
        path: PathBuf,
        /// The zip reader's refusal.
        why: String,
    },
    /// The disk filled while the temporary was written; it is left, named.
    DiskFull {
        /// The temporary.
        temp: PathBuf,
        /// Bytes written before the disk filled.
        written: u64,
    },
    /// `store::staging::create` refused the temporary's name: a second name
    /// of another file, a name that is not a regular file, or a name another
    /// writer took again. Nothing was written through it.
    TempNotCreated {
        /// The temporary's name.
        temp: PathBuf,
        /// The door's word (`staging_linked`, `staging_not_regular`,
        /// `staging_contended`).
        word: &'static str,
        /// The door's sentence.
        why: String,
    },
    /// A file appeared at this run's own temporary name after the leftover
    /// check, under the root's lock, and `store::staging::create` removed
    /// that name to create the temporary. The run stops: this writer never
    /// removes a file by choice, and says so when the door did.
    TempRaced {
        /// The temporary, left empty for the operator.
        temp: PathBuf,
        /// Bytes the removed name held.
        cleared: u64,
    },
    /// Any other failure creating, writing or syncing a file.
    WriteFailed {
        /// The file.
        path: PathBuf,
        /// What was being done.
        step: &'static str,
        /// The error's kind.
        kind: std::io::ErrorKind,
    },
    /// The written temporary did not read back as the zip; it is left, named,
    /// and the day is not held.
    VerifyFailed {
        /// The temporary.
        temp: PathBuf,
        /// The first difference.
        why: String,
    },
}

impl Refusal {
    /// The refusal's code on a log line and an event.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NameNotCanonical { .. } => "name_not_canonical",
            Self::RunIdInvalid { .. } => "run_id_invalid",
            Self::CommitInvalid { .. } => "commit_invalid",
            Self::RootMissing { .. } => "root_missing",
            Self::NotRegularFile { .. } => "not_regular_file",
            Self::LinkedFolder { .. } => "linked_folder",
            Self::ZipUnavailable { .. } => "zip_unavailable",
            Self::ZipRefused { .. } => "zip_refused",
            Self::NameNotUtf8 { .. } => "name_not_utf8",
            Self::NameTooLong { .. } => "name_too_long",
            Self::DuplicateEntry { .. } => "duplicate_entry",
            Self::InnerFolderMismatch { .. } => "inner_folder_mismatch",
            Self::NoFileInDay { .. } => "no_file_in_day",
            Self::TooManyEntries { .. } => "too_many_entries",
            Self::MemberTooLarge { .. } => "member_too_large",
            Self::MemberCorrupt { .. } => "member_corrupt",
            Self::EncodeFailed { .. } => "encode_failed",
            Self::DayTooLarge { .. } => "day_too_large",
            Self::DayLocked { .. } => "day_locked",
            Self::LockFailed { .. } => "lock_failed",
            Self::StaleTemp { .. } => "stale_temp",
            Self::HeldDayDiffers { .. } => "held_day_differs",
            Self::ZipCorruptOnVerify { .. } => "zip_corrupt_on_verify",
            Self::DiskFull { .. } => "disk_full",
            Self::TempNotCreated { .. } => "temp_not_created",
            Self::TempRaced { .. } => "temp_raced",
            Self::WriteFailed { .. } => "write_failed",
            Self::VerifyFailed { .. } => "verify_failed",
        }
    }
}

impl core::fmt::Display for Refusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let code = self.code();
        match self {
            Self::NameNotCanonical { name } => write!(
                f,
                "{code}: `{name}` is not GFDLNFO_TICK_, GFDLCM_STOCK_TICK_ or \
                 GFDLCM_INDICES_TICK_ plus a real ddmmyyyy and `.zip`"
            ),
            Self::RunIdInvalid { run_id: value } | Self::CommitInvalid { commit: value } => {
                write!(f, "{code}: `{value}` is not 1 to 64 of A-Z a-z 0-9 _ -")
            }
            Self::RootMissing { .. }
            | Self::NotRegularFile { .. }
            | Self::LinkedFolder { .. }
            | Self::ZipCorruptOnVerify { .. }
            | Self::TempRaced { .. } => self.door_words(code, f),
            Self::ZipUnavailable { path, kind } => {
                write!(f, "{code}: {} could not be read ({kind})", path.display())
            }
            Self::ZipRefused { why } => write!(f, "{code}: {why}"),
            Self::NameNotUtf8 { raw_hex } => write!(f, "{code}: raw={raw_hex}"),
            Self::NameTooLong { len } => write!(f, "{code}: {len} bytes"),
            Self::DuplicateEntry { name } => write!(f, "{code}: `{name}` twice"),
            Self::InnerFolderMismatch { stem, found } => {
                write!(
                    f,
                    "{code}: the name states `{stem}`, the zip holds `{found}`"
                )
            }
            Self::NoFileInDay { stem } => {
                write!(
                    f,
                    "{code}: the zip holds the folder `{stem}` and no file in it"
                )
            }
            Self::TooManyEntries { entries } => write!(f, "{code}: {entries}"),
            Self::MemberTooLarge { name, size } => {
                write!(f, "{code}: `{name}` states {size} bytes, over {MEMBER_CAP}")
            }
            Self::MemberCorrupt { name, why } | Self::EncodeFailed { name, why } => {
                write!(f, "{code}: `{name}`: {why}")
            }
            Self::DayTooLarge { bytes } => {
                write!(f, "{code}: {bytes} compressed bytes, over {DAY_BLOCKS_CAP}")
            }
            Self::DayLocked { lock } => {
                write!(f, "{code}: another writer holds {}", lock.display())
            }
            Self::LockFailed { lock, kind } => {
                write!(f, "{code}: {} ({kind})", lock.display())
            }
            Self::StaleTemp { leftovers } => {
                write!(f, "{code}: left for the operator, not removed:")?;
                for (name, len) in leftovers {
                    write!(f, " {name} ({len} bytes)")?;
                }
                Ok(())
            }
            Self::HeldDayDiffers { path, why } => write!(
                f,
                "{code}: {} does not match its zip ({why}); it is left as it is",
                path.display()
            ),
            Self::TempNotCreated { why, .. } => write!(f, "{code}: {why}"),
            Self::DiskFull { temp, written } => write!(
                f,
                "{code}: stage=compress file={} bytes={written}; the temporary is left",
                temp.display()
            ),
            Self::WriteFailed { path, step, kind } => {
                write!(f, "{code}: {step} {} ({kind})", path.display())
            }
            Self::VerifyFailed { temp, why } => write!(
                f,
                "{code}: {} ({why}); the temporary is left and the day is not held",
                temp.display()
            ),
        }
    }
}

impl Refusal {
    /// The words of the refusals about the names the writer opens and the
    /// door it creates through: a free method rather than arms, so the
    /// `Display` stays under `clippy::too_many_lines`.
    fn door_words(&self, code: &str, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RootMissing { root, what } => write!(
                f,
                "{code}: {} is not an existing directory ({what}); the writer never \
                 makes a tick store's root",
                root.display()
            ),
            Self::NotRegularFile { path, found } => write!(
                f,
                "{code} ({}): {} is {}, not a regular file; it is neither followed nor \
                 waited on, and it is left as it is",
                found.word(),
                path.display(),
                found.sentence()
            ),
            Self::LinkedFolder { path } => write!(
                f,
                "{code}: {} is a symbolic link; no folder below a tick store's root is \
                 written through one",
                path.display()
            ),
            Self::ZipCorruptOnVerify { path, why } => write!(
                f,
                "{code}: the zip does not read back as its own records state ({why}); \
                 the held day {} is left as it is and not blamed",
                path.display()
            ),
            Self::TempRaced { temp, cleared } => write!(
                f,
                "{code}: a file of {cleared} bytes appeared at {} after the leftover \
                 check, under the root's lock, and the staging door removed that name; \
                 the run stops and its empty temporary is left for the operator",
                temp.display()
            ),
            _ => write!(f, "{code}"),
        }
    }
}

impl core::error::Error for Refusal {}

/// What a written or held day holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayReport {
    /// The day file.
    pub path: PathBuf,
    /// Entries, directories included.
    pub entries: u64,
    /// Entries stored columnar.
    pub columnar: u64,
    /// Files stored raw: not a regular CSV (FM-057).
    pub raw_fallbacks: u64,
    /// Data rows of the columnar entries.
    pub rows: u64,
    /// The members' uncompressed bytes.
    pub csv_bytes: u64,
    /// The zip's length.
    pub zip_bytes: u64,
    /// The day file's length.
    pub stored_bytes: u64,
    /// Entries outside the day folder, kept as they are.
    pub outside_folder: u64,
}

/// A day's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The day file was written, read back, verified and published now.
    Written(DayReport),
    /// The day file was already there and verified against the zip: nothing
    /// was written.
    Held(DayReport),
}

impl Outcome {
    /// What the day holds.
    #[must_use]
    pub const fn report(&self) -> &DayReport {
        match self {
            Self::Written(report) | Self::Held(report) => report,
        }
    }

    /// `written` or `held`.
    #[must_use]
    pub const fn verdict(&self) -> &'static str {
        match self {
            Self::Written(_) => "written",
            Self::Held(_) => "held",
        }
    }
}

/// Fault hooks for the extreme cases a test cannot provoke: a disk that fills
/// after `full_after` bytes, and a temporary flipped after it is written and
/// before it is verified (a torn or bit-rotted write).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Faults {
    /// Writing past this many bytes of the temporary fails `StorageFull`.
    pub(crate) full_after: Option<u64>,
    /// The temporary's byte at this offset is inverted before the verify.
    pub(crate) flip_before_verify: Option<u64>,
    /// A file put at the temporary's name after the leftover check and before
    /// the create: the race the staging door's clear path answers.
    pub(crate) plant_before_create: Option<Plant>,
}

/// What [`Faults::plant_before_create`] puts at the temporary's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the race hook is set by the tests alone")
)]
pub(crate) enum Plant {
    /// A regular file with one name, of this many bytes.
    File(u64),
    /// A second name of the file `<root>/planted`.
    HardLink,
}

/// How one day is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteOptions {
    /// The run's id, in the temporary's name and on the log line.
    run_id: String,
    /// Encoding and verifying threads.
    workers: usize,
    /// The commit of the code that writes, named on every log line and
    /// event; [`UNSTAMPED`] when the caller has none.
    commit: Option<String>,
    /// Test-only faults.
    faults: Faults,
}

impl WriteOptions {
    /// A run named `run_id`, with one worker per available core up to eight.
    #[must_use]
    pub fn new(run_id: &str) -> Self {
        let cores = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        Self {
            run_id: run_id.to_owned(),
            workers: cores.clamp(1, 8),
            commit: None,
            faults: Faults::default(),
        }
    }

    /// The same, naming `commit` as the code that writes.
    #[must_use]
    pub fn with_commit(self, commit: &str) -> Self {
        Self {
            commit: Some(commit.to_owned()),
            ..self
        }
    }

    /// The commit named on the log line and the event.
    #[must_use]
    pub fn commit(&self) -> &str {
        self.commit.as_deref().unwrap_or(UNSTAMPED)
    }

    /// The same, with `workers` threads (at least one).
    #[must_use]
    pub fn with_workers(self, workers: usize) -> Self {
        Self {
            workers: workers.max(1),
            ..self
        }
    }

    /// The same, with `faults`.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_faults(self, faults: Faults) -> Self {
        Self { faults, ..self }
    }

    /// The run id.
    #[must_use]
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
}

/// Whether `run_id` may be part of a file name: 1 to 64 of `[A-Za-z0-9_-]`.
fn run_id_ok(run_id: &str) -> bool {
    (1..=64).contains(&run_id.len())
        && run_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

// ── the encoder (FORMAT.md §4) ──────────────────────────────────────────────

/// One zstd frame of `bytes` at `ruzstd`'s `Fastest` level. A panic inside
/// the encoder is caught and named, never let kill the process.
fn zstd(bytes: &[u8]) -> Result<Vec<u8>, String> {
    std::panic::catch_unwind(|| compress_to_vec(bytes, CompressionLevel::Fastest))
        .map_err(|_| "the zstd encoder panicked".to_owned())
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// `usize` as `u64`: lossless on every target this builds for.
fn wide(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// One member encoded: its block, its kind and its data rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoded {
    /// The block (§4), its first byte the kind.
    pub block: Vec<u8>,
    /// 1 columnar or 2 raw.
    pub kind: u8,
    /// Data rows, 0 for a raw block.
    pub rows: u64,
}

/// `data` encoded as §4 states: columnar when it is a perfectly regular CSV,
/// raw otherwise.
///
/// # Errors
///
/// The encoder's failure, named.
pub fn encode(data: &[u8]) -> Result<Encoded, String> {
    if let Some((block, rows)) = columnar(data)? {
        return Ok(Encoded {
            block,
            kind: KIND_COLUMNAR,
            rows,
        });
    }
    let mut block = vec![KIND_RAW];
    block.extend_from_slice(&zstd(data)?);
    Ok(Encoded {
        block,
        kind: KIND_RAW,
        rows: 0,
    })
}

/// The columnar block of `data` (§4, kind 1), or `None` when `data` is not a
/// CSV whose every line has the header's field count and one terminator:
/// empty, mixed `\r\n` and `\n`, a stray `\r`, a ragged row.
fn columnar(data: &[u8]) -> Result<Option<(Vec<u8>, u64)>, String> {
    if data.is_empty() {
        return Ok(None);
    }
    let mut lines: Vec<&[u8]> = data.split(|&b| b == b'\n').collect();
    let trailing = lines.last().is_some_and(|last| last.is_empty());
    if trailing {
        lines.pop();
    }
    let count = lines.len();
    let crlf = lines.first().and_then(|first| first.last()) == Some(&b'\r');
    for (at, line) in lines.iter_mut().enumerate() {
        let terminated = at + 1 < count || trailing;
        if crlf && terminated {
            match line.split_last() {
                Some((b'\r', body)) => *line = body,
                _ => return Ok(None),
            }
        }
        if line.contains(&b'\r') {
            return Ok(None);
        }
    }
    let Some((&header, body)) = lines.split_first() else {
        return Ok(None);
    };
    let Ok(header_len) = u32::try_from(header.len()) else {
        return Ok(None);
    };
    let ncols = header.split(|&b| b == b',').count();
    let Ok(ncols32) = u32::try_from(ncols) else {
        return Ok(None);
    };
    let mut columns: Vec<Vec<u8>> = vec![Vec::new(); ncols];
    for (row, line) in body.iter().enumerate() {
        let mut fields = line.split(|&b| b == b',');
        for column in &mut columns {
            let Some(field) = fields.next() else {
                return Ok(None);
            };
            if row > 0 {
                column.push(b'\n');
            }
            column.extend_from_slice(field);
        }
        if fields.next().is_some() {
            return Ok(None);
        }
    }
    let rows = wide(body.len());
    let mut block = vec![KIND_COLUMNAR, u8::from(crlf), u8::from(trailing)];
    put_u32(&mut block, header_len);
    block.extend_from_slice(header);
    put_u32(&mut block, ncols32);
    put_u64(&mut block, rows);
    for text in &columns {
        // Each representation is compressed and the smallest kept, as the
        // tool's trial does; ties keep the text, then the plain numbers.
        let mut best = (0_u8, wide(text.len()), zstd(text)?);
        if let Some(parsed) = parse_column(text, body.len()) {
            for delta in [false, true] {
                let payload = numeric_payload(&parsed, delta);
                let frame = zstd(&payload)?;
                if frame.len() < best.2.len() {
                    best = (1, wide(payload.len()), frame);
                }
            }
        }
        let (tag, payload_len, frame) = best;
        block.push(tag);
        put_u64(&mut block, payload_len);
        put_u64(&mut block, wide(text.len()));
        put_u64(&mut block, wide(frame.len()));
        block.extend_from_slice(&frame);
    }
    Ok(Some((block, rows)))
}

/// A column every field of which is a number the shape byte re-renders byte
/// for byte: its shapes and values.
struct Parsed {
    shapes: Vec<u8>,
    values: Vec<i64>,
}

/// The column `text` (`rows` fields joined by `\n`) as numbers, or `None`
/// when any field does not re-render exactly (§4: "a column is only stored
/// as numeric if re-rendering every field reproduces it byte for byte").
fn parse_column(text: &[u8], rows: usize) -> Option<Parsed> {
    if rows == 0 {
        return None;
    }
    let mut shapes = Vec::with_capacity(rows);
    let mut values = Vec::with_capacity(rows);
    let mut again = Vec::with_capacity(24);
    for field in text.split(|&b| b == b'\n') {
        let (shape, value) = parse_field(field)?;
        again.clear();
        render(&mut again, shape, value);
        if again != field {
            return None;
        }
        shapes.push(shape);
        values.push(value);
    }
    (shapes.len() == rows).then_some(Parsed { shapes, values })
}

/// One field as `(shape, value)`: `HH:MM:SS`, or `-?D+(.D+)?` with at most
/// seventeen characters after the sign and fifteen decimals.
fn parse_field(field: &[u8]) -> Option<(u8, i64)> {
    if let &[h1, h2, b':', m1, m2, b':', s1, s2] = field {
        let two = |a: u8, b: u8| {
            (a.is_ascii_digit() && b.is_ascii_digit())
                .then(|| i64::from(a - b'0') * 10 + i64::from(b - b'0'))
        };
        return Some((
            SHAPE_TIME,
            two(h1, h2)? * 3_600 + two(m1, m2)? * 60 + two(s1, s2)?,
        ));
    }
    let (negative, body) = match field.split_first() {
        Some((b'-', rest)) => (true, rest),
        _ => (false, field),
    };
    if body.is_empty() || body.len() > 17 {
        return None;
    }
    let mut value: i64 = 0;
    let mut decimals: Option<u8> = None;
    for &b in body {
        if b == b'.' {
            if decimals.is_some() {
                return None;
            }
            decimals = Some(0);
        } else if b.is_ascii_digit() {
            value = value * 10 + i64::from(b - b'0');
            if let Some(d) = decimals.as_mut() {
                *d += 1;
            }
        } else {
            return None;
        }
    }
    let d = decimals.unwrap_or(0);
    if decimals == Some(0) || d > 15 {
        return None;
    }
    let shape = d | if negative { SHAPE_NEG } else { 0 };
    Some((shape, if negative { -value } else { value }))
}

/// One field's text from its shape and value, as §4 renders it.
fn render(out: &mut Vec<u8>, shape: u8, value: i64) {
    if shape & SHAPE_TIME != 0 {
        // A parsed time is never negative, and its hours are at most 99
        // whenever its minutes and seconds are below 60; any other field
        // re-renders differently in those places and is kept as text, so the
        // hours are written as their last two digits.
        let secs = value.unsigned_abs();
        let two = |out: &mut Vec<u8>, n: u64| {
            out.push(b'0' + u8::try_from(n / 10 % 10).unwrap_or_default());
            out.push(b'0' + u8::try_from(n % 10).unwrap_or_default());
        };
        two(out, secs / 3_600);
        out.push(b':');
        two(out, secs / 60 % 60);
        out.push(b':');
        two(out, secs % 60);
        return;
    }
    if shape & SHAPE_NEG != 0 {
        out.push(b'-');
    }
    let d = usize::from(shape & 0x0F);
    let mut digits = [b'0'; 24];
    let mut n = value.unsigned_abs();
    let mut len = 0;
    for slot in digits.iter_mut().rev() {
        *slot = b'0' + u8::try_from(n % 10).unwrap_or_default();
        n /= 10;
        len += 1;
        if n == 0 && len > d {
            break;
        }
    }
    let start = digits.len() - len;
    let shown = digits.get(start..).unwrap_or_default();
    let split = shown.len() - d;
    out.extend_from_slice(shown.get(..split).unwrap_or_default());
    if d > 0 {
        out.push(b'.');
        out.extend_from_slice(shown.get(split..).unwrap_or_default());
    }
}

/// A numeric column's payload (§4): `shape[rows]`, `delta`, `w`, then `w`
/// byte planes of the zigzag values (their differences when `delta`).
fn numeric_payload(parsed: &Parsed, delta: bool) -> Vec<u8> {
    let mut previous: i64 = 0;
    let zigzag: Vec<u64> = parsed
        .values
        .iter()
        .map(|&value| {
            let d = if delta {
                value.wrapping_sub(previous)
            } else {
                value
            };
            previous = value;
            u64::from_ne_bytes(((d << 1) ^ (d >> 63)).to_ne_bytes())
        })
        .collect();
    let max = zigzag.iter().copied().max().unwrap_or(0);
    let width = (64 - max.leading_zeros()).div_ceil(8);
    let rows = parsed.shapes.len();
    let mut out = Vec::with_capacity(rows * (1 + usize::try_from(width).unwrap_or(8)) + 2);
    out.extend_from_slice(&parsed.shapes);
    out.push(u8::from(delta));
    out.push(u8::try_from(width).unwrap_or(8));
    for k in 0..width {
        out.extend(
            zigzag
                .iter()
                .map(|&x| u8::try_from((x >> (8 * k)) & 0xFF).unwrap_or_default()),
        );
    }
    out
}

// ── one day ────────────────────────────────────────────────────────────────

/// One index entry with its block, as written.
struct Item {
    kind: u8,
    block: Vec<u8>,
    rows: u64,
}

/// The zip's entries, checked before anything is read: names UTF-8, short
/// enough and unique, the day folder the file name states, no member past
/// [`MEMBER_CAP`].
fn preflight(identity: &DayIdentity, entries: &[ZipEntry]) -> Result<u64, Refusal> {
    if u32::try_from(entries.len()).is_err() {
        return Err(Refusal::TooManyEntries {
            entries: entries.len(),
        });
    }
    let mut seen = std::collections::HashSet::with_capacity(entries.len());
    let (mut inside, mut outside) = (0_u64, 0_u64);
    let mut folder_only = false;
    for entry in entries {
        if std::str::from_utf8(&entry.raw_name).is_err() {
            return Err(Refusal::NameNotUtf8 {
                raw_hex: entry.raw_name.iter().fold(String::new(), |mut hex, b| {
                    let _infallible = write!(hex, "{b:02x}");
                    hex
                }),
            });
        }
        if u16::try_from(entry.raw_name.len()).is_err() {
            return Err(Refusal::NameTooLong {
                len: entry.raw_name.len(),
            });
        }
        if !seen.insert(entry.raw_name.as_slice()) {
            return Err(Refusal::DuplicateEntry {
                name: entry.name.clone(),
            });
        }
        if entry.len > MEMBER_CAP {
            return Err(Refusal::MemberTooLarge {
                name: entry.name.clone(),
                size: entry.len,
            });
        }
        let first = entry.name.split(['/', '\\']).next().unwrap_or_default();
        if first == identity.stem {
            // A directory entry is not a file of the day: a zip of only its
            // folder is refused `no_file_in_day`, not held with 0 rows.
            if is_directory(entry) {
                folder_only = true;
            } else {
                inside += 1;
            }
        } else if DayIdentity::of_zip_name(&format!("{first}.zip")).is_ok() {
            return Err(Refusal::InnerFolderMismatch {
                stem: identity.stem.clone(),
                found: first.to_owned(),
            });
        } else {
            outside += 1;
        }
    }
    if inside == 0 && folder_only {
        return Err(Refusal::NoFileInDay {
            stem: identity.stem.clone(),
        });
    }
    if inside == 0 {
        return Err(Refusal::InnerFolderMismatch {
            stem: identity.stem.clone(),
            found: entries
                .first()
                .map(|entry| entry.name.clone())
                .unwrap_or_default(),
        });
    }
    Ok(outside)
}

/// Whether an entry is a directory entry with no bytes, stored as kind 0.
fn is_directory(entry: &ZipEntry) -> bool {
    entry.len == 0 && (entry.name.ends_with('/') || entry.name.ends_with('\\'))
}

/// A member's bytes read from the zip and held to its central size and
/// CRC-32.
fn member<B: ReadAt>(zip: &B, entry: &ZipEntry) -> Result<Vec<u8>, Refusal> {
    let corrupt = |why: String| Refusal::MemberCorrupt {
        name: entry.name.clone(),
        why,
    };
    let bytes = member_bytes(zip, entry.locator, &entry.name, entry.len)
        .map_err(|why| corrupt(why.to_string()))?;
    if wide(bytes.len()) != entry.len {
        return Err(corrupt(format!(
            "{} bytes, the record states {}",
            bytes.len(),
            entry.len
        )));
    }
    if crc32(&bytes) != entry.crc32 {
        return Err(corrupt(format!(
            "CRC-32 {:08x}, the record states {:08x}",
            crc32(&bytes),
            entry.crc32
        )));
    }
    Ok(bytes)
}

/// One member read, encoded and rebuilt from its block before it is kept.
fn item<B: ReadAt>(zip: &B, entry: &ZipEntry) -> Result<Item, Refusal> {
    if is_directory(entry) {
        return Ok(Item {
            kind: KIND_DIR,
            block: Vec::new(),
            rows: 0,
        });
    }
    let data = member(zip, entry)?;
    let failed = |why: String| Refusal::EncodeFailed {
        name: entry.name.clone(),
        why,
    };
    let encoded = encode(&data).map_err(failed)?;
    let probe = IndexEntry {
        name: entry.name.clone(),
        kind: encoded.kind,
        off: 0,
        len: wide(encoded.block.len()),
        size: entry.len,
        crc: entry.crc32,
        rows: encoded.rows,
        dos_time: entry.dos_time,
    };
    let back = rebuild(&encoded.block, &probe).map_err(|why| failed(why.to_string()))?;
    if back != data {
        return Err(failed(
            "the block does not rebuild to the member".to_owned(),
        ));
    }
    Ok(Item {
        kind: encoded.kind,
        block: encoded.block,
        rows: encoded.rows,
    })
}

/// `job(i)` for every `i` below `count` on `workers` threads, the results in
/// index order; the first refusal stops the rest.
fn parallel<T: Send>(
    count: usize,
    workers: usize,
    job: &(dyn Fn(usize) -> Result<T, Refusal> + Sync),
) -> Result<Vec<T>, Refusal> {
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let mut done: Vec<(usize, Result<T, Refusal>)> = Vec::with_capacity(count);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers.min(count.max(1)))
            .map(|_| {
                scope.spawn(|| {
                    let mut mine = Vec::new();
                    while !stop.load(Ordering::Relaxed) {
                        let at = next.fetch_add(1, Ordering::Relaxed);
                        if at >= count {
                            break;
                        }
                        let result = job(at);
                        if result.is_err() {
                            stop.store(true, Ordering::Relaxed);
                        }
                        mine.push((at, result));
                    }
                    mine
                })
            })
            .collect();
        for handle in handles {
            match handle.join() {
                Ok(mine) => done.extend(mine),
                Err(_) => done.push((
                    0,
                    Err(Refusal::EncodeFailed {
                        name: String::new(),
                        why: "a worker panicked".to_owned(),
                    }),
                )),
            }
        }
    });
    // Indices are handed out in increasing order and every one handed out
    // yields one result, so in index order the results run unbroken from 0:
    // up to the first refusal when one stopped the rest, to `count` when none
    // did.
    done.sort_by_key(|(at, _)| *at);
    done.into_iter().map(|(_, result)| result).collect()
}

/// The index (§3) of `items` written after the magic in order.
fn index_bytes(entries: &[ZipEntry], items: &[Item]) -> (Vec<u8>, u64) {
    let mut index = Vec::new();
    let mut off = wide(MAGIC.len());
    for (entry, item) in entries.iter().zip(items) {
        let len = wide(item.block.len());
        index.extend_from_slice(
            &u16::try_from(entry.raw_name.len())
                .unwrap_or(u16::MAX)
                .to_le_bytes(),
        );
        index.extend_from_slice(&entry.raw_name);
        index.push(item.kind);
        put_u64(&mut index, off);
        put_u64(&mut index, len);
        put_u64(&mut index, entry.len);
        put_u32(&mut index, entry.crc32);
        put_u64(&mut index, item.rows);
        put_u32(&mut index, entry.dos_time);
        off += len;
    }
    (index, off)
}

/// The footer (§3): `index_off`, `index_len`, `index_raw_len`, `index_crc`,
/// `n`, `BRTXTSE1`.
fn footer(index_off: u64, frame_len: u64, index: &[u8], entries: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(40);
    put_u64(&mut out, index_off);
    put_u64(&mut out, frame_len);
    put_u64(&mut out, wide(index.len()));
    put_u32(&mut out, crc32(index));
    put_u32(&mut out, u32::try_from(entries).unwrap_or(u32::MAX));
    out.extend_from_slice(FOOTER_MAGIC);
    out
}

/// A writer that fails `StorageFull` past `limit` bytes: the disk-full hook.
struct Limited<'a> {
    file: &'a File,
    written: u64,
    limit: Option<u64>,
}

impl Write for Limited<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let room = self
            .limit
            .map_or(u64::MAX, |limit| limit.saturating_sub(self.written));
        if room == 0 {
            return Err(std::io::ErrorKind::StorageFull.into());
        }
        let take = buf
            .get(..usize::try_from(room).unwrap_or(usize::MAX).min(buf.len()))
            .unwrap_or_default();
        let mut handle: &File = self.file;
        let n = handle.write(take)?;
        self.written += wide(n);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut handle: &File = self.file;
        handle.flush()
    }
}

/// The temporary written: magic, blocks, index frame, footer; then fsynced.
fn write_temp(
    temp: &Path,
    entries: &[ZipEntry],
    items: &[Item],
    faults: Faults,
) -> Result<u64, Refusal> {
    let failed = |step: &'static str, why: &std::io::Error| Refusal::WriteFailed {
        path: temp.to_path_buf(),
        step,
        kind: why.kind(),
    };
    // The workspace's one door for a temporary (R3-16, gate 28): it opens
    // nothing already at the name. A one-name file there is cleared by the
    // door, which this writer never chooses: the leftover check ran under the
    // lock, so that is a race, and the run stops naming it (D-4967).
    let staged = store::staging::create(temp).map_err(|why| match why {
        store::staging::StagingError::Io { source, .. } => failed("create", &source),
        other => Refusal::TempNotCreated {
            temp: temp.to_path_buf(),
            word: other.word(),
            why: other.to_string(),
        },
    })?;
    if let Some(cleared) = staged.cleared {
        return Err(Refusal::TempRaced {
            temp: temp.to_path_buf(),
            cleared,
        });
    }
    let file = staged.file;
    let (index, index_off) = index_bytes(entries, items);
    let frame = zstd(&index).map_err(|why| Refusal::EncodeFailed {
        name: "the index".to_owned(),
        why,
    })?;
    let tail = footer(index_off, wide(frame.len()), &index, entries.len());
    let mut out = Limited {
        file: &file,
        written: 0,
        limit: faults.full_after,
    };
    let pieces = std::iter::once(MAGIC.as_slice())
        .chain(items.iter().map(|item| item.block.as_slice()))
        .chain([frame.as_slice(), tail.as_slice()]);
    for piece in pieces {
        if let Err(why) = out.write_all(piece) {
            return Err(if why.kind() == std::io::ErrorKind::StorageFull {
                Refusal::DiskFull {
                    temp: temp.to_path_buf(),
                    written: out.written,
                }
            } else {
                failed("write", &why)
            });
        }
    }
    file.sync_all().map_err(|why| failed("fsync", &why))?;
    Ok(out.written)
}

/// The day file at `bts` read back through [`crate::gdfl_tickstore`] and held
/// to the zip entry by entry: the same count, each name byte for byte, size,
/// CRC-32, DOS time and kind, every block inside the blocks area, after the
/// one before it and not overlapping it (FM-248), and every rebuilt file
/// equal byte for byte to the zip's member, read again. Writer-blind: a day
/// file of the separate tool is held to the same rules.
///
/// # Errors
///
/// The first difference, as text.
pub fn verify_file<B: ReadAt + Sync>(
    bts: &Path,
    zip: &B,
    entries: &[ZipEntry],
    workers: usize,
) -> Result<DayReport, String> {
    let (file, len) = store::safe_open::open_read_len(bts).map_err(|why| format!("open: {why}"))?;
    verify_open(&file, len, bts, zip, entries, workers).map_err(|why| match why {
        Mismatch::File(why) => why,
        Mismatch::Zip(why) => format!("the zip: {why}"),
    })
}

/// Why a day file did not verify against its zip.
#[derive(Debug)]
enum Mismatch {
    /// The day file differs from the zip.
    File(String),
    /// A member of the zip did not read back as the zip's own record
    /// states: the zip is the damaged side.
    Zip(String),
}

/// [`verify_file`] on a day file already open, of length `len`.
fn verify_open<B: ReadAt + Sync>(
    file: &File,
    len: u64,
    bts: &Path,
    zip: &B,
    entries: &[ZipEntry],
    workers: usize,
) -> Result<DayReport, Mismatch> {
    let index = read_index(file, len).map_err(|why| Mismatch::File(why.to_string()))?;
    if index.len() != entries.len() {
        return Err(Mismatch::File(format!(
            "{} entries, the zip holds {}",
            index.len(),
            entries.len()
        )));
    }
    let mut next = wide(MAGIC.len());
    let mut report = DayReport {
        path: bts.to_path_buf(),
        entries: wide(index.len()),
        columnar: 0,
        raw_fallbacks: 0,
        rows: 0,
        csv_bytes: 0,
        zip_bytes: 0,
        stored_bytes: len,
        outside_folder: 0,
    };
    for (stored, entry) in index.iter().zip(entries) {
        let same = stored.name.as_bytes() == entry.raw_name.as_slice()
            && stored.size == entry.len
            && stored.crc == entry.crc32
            && stored.dos_time == entry.dos_time;
        if !same {
            return Err(Mismatch::File(format!(
                "`{}`: name, size, CRC-32 or DOS time is not the zip's",
                entry.name
            )));
        }
        if (stored.kind == KIND_DIR) != is_directory(entry) {
            return Err(Mismatch::File(format!(
                "`{}`: kind {} for this entry",
                entry.name, stored.kind
            )));
        }
        if stored.kind != KIND_DIR {
            if stored.off < next {
                return Err(Mismatch::File(format!("`{}`: block_overlap", entry.name)));
            }
            next = stored.off.saturating_add(stored.len);
        }
        report.csv_bytes += stored.size;
        report.rows += stored.rows;
        match stored.kind {
            KIND_COLUMNAR => report.columnar += 1,
            KIND_RAW => report.raw_fallbacks += 1,
            _ => {}
        }
    }
    let index = &index;
    parallel(entries.len(), workers, &|at| {
        let (Some(stored), Some(entry)) = (index.get(at), entries.get(at)) else {
            return Err(verify_refusal("an entry went missing".to_owned()));
        };
        if stored.kind == KIND_DIR {
            return Ok(());
        }
        let got = rebuild(file, stored)
            .map_err(|why| verify_refusal(format!("`{}`: {why}", entry.name)))?;
        // The zip's own refusal passes through as itself: the zip is then
        // the damaged side, not the day file.
        let want = member(zip, entry)?;
        if got == want {
            Ok(())
        } else {
            Err(verify_refusal(format!("`{}`: bytes differ", entry.name)))
        }
    })
    .map_err(|why| match why {
        Refusal::VerifyFailed { why, .. } => Mismatch::File(why),
        corrupt @ Refusal::MemberCorrupt { .. } => Mismatch::Zip(corrupt.to_string()),
        other => Mismatch::File(other.to_string()),
    })?;
    Ok(report)
}

/// A verify difference, carried through [`parallel`].
const fn verify_refusal(why: String) -> Refusal {
    Refusal::VerifyFailed {
        temp: PathBuf::new(),
        why,
    }
}

/// Every `<stem>.bts.tmp*` in the day's folder `dir`, with its length.
fn leftovers(dir: &Path, stem: &str) -> Result<Vec<(String, u64)>, Refusal> {
    let prefix = format!("{stem}{TMP_SUFFIX}");
    let listing = std::fs::read_dir(dir).map_err(|why| Refusal::WriteFailed {
        path: dir.to_path_buf(),
        step: "list",
        kind: why.kind(),
    })?;
    let mut found = Vec::new();
    for item in listing.flatten() {
        let name = item.file_name().to_string_lossy().into_owned();
        if name.starts_with(&prefix) {
            let len = item.metadata().map_or(0, |meta| meta.len());
            found.push((name, len));
        }
    }
    found.sort();
    Ok(found)
}

/// One line of [`LOG_NAME`], appended; its failure is returned, never
/// swallowed.
fn append_log(root: &Path, line: &str) -> Result<(), String> {
    let path = root.join(LOG_NAME);
    if let Some(found) = not_regular(&path) {
        return Err(format!(
            "{} is {} ({}), not a regular file; it is left as it is",
            path.display(),
            found.sentence(),
            found.word()
        ));
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|why| format!("{}: {}", path.display(), why.kind()))?;
    file.write_all(format!("{line}\n").as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|why| format!("{}: {}", path.display(), why.kind()))
}

/// What is at `path` when something is there and it is not a regular file,
/// looked at without following a link; `None` for nothing or a regular file.
fn not_regular(path: &Path) -> Option<Found> {
    std::fs::symlink_metadata(path)
        .ok()
        .and_then(|meta| Found::of(meta.file_type()))
}

/// `value` as one unquoted field of a log line when it is in the run id's
/// alphabet, else `invalid` (the refusal's words carry it).
fn token(value: &str) -> &str {
    if run_id_ok(value) { value } else { "invalid" }
}

/// `text` as one quoted field of a log line: a path with a space stays one
/// field.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace(['"', '\n'], "'"))
}

/// What one run did to one day, logged and emitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The outcome or the refusal.
    pub result: Result<Outcome, Refusal>,
    /// Milliseconds the day took.
    pub millis: u64,
    /// Why the log line could not be appended, if it could not.
    pub log_error: Option<String>,
}

/// One day zip written into the tick store under `root`, by the protocol the
/// module states, then logged to [`LOG_NAME`] and emitted as a
/// `pull.tickstore.write` event, whatever the outcome.
#[must_use]
pub fn write_day(zip: &Path, root: &Path, options: &WriteOptions) -> Run {
    let started = Instant::now();
    let result = write_day_inner(zip, root, options);
    let millis = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let zip_name = zip.display().to_string();
    let zip_field = quoted(&zip_name);
    let commit = token(options.commit());
    let run = token(&options.run_id);
    let line = match &result {
        Ok(outcome) => {
            let r = outcome.report();
            format!(
                "v=1 verdict={} day={} zip={zip_field} writer=\"{WRITER}\" commit={commit} \
                 run={run} entries={} columnar={} raw_fallbacks={} rows={} csv_bytes={} \
                 zip_bytes={} stored_bytes={} outside_folder={} millis={millis}",
                outcome.verdict(),
                quoted(&r.path.display().to_string()),
                r.entries,
                r.columnar,
                r.raw_fallbacks,
                r.rows,
                r.csv_bytes,
                r.zip_bytes,
                r.stored_bytes,
                r.outside_folder,
            )
        }
        Err(why) => format!(
            "v=1 verdict=refused code={} zip={zip_field} writer=\"{WRITER}\" commit={commit} \
             run={run} millis={millis} why={}",
            why.code(),
            quoted(&why.to_string()),
        ),
    };
    // The root is never made here either: a root that is not there has no
    // log, and the caller is told so (`log_error`).
    let log_error = append_log(root, &line).err();
    emit(&result, &zip_name, options, millis);
    Run {
        result,
        millis,
        log_error,
    }
}

/// The `pull.tickstore.write` event for one day.
fn emit(result: &Result<Outcome, Refusal>, zip: &str, options: &WriteOptions, millis: u64) {
    let words = match result {
        Ok(outcome) => outcome.report().path.display().to_string(),
        Err(why) => why.to_string(),
    };
    let _dropped_when_filtered = telemetry::emit(&event_of(result, zip, options, millis, &words));
}

/// The `pull.tickstore.write` event: `words` is the day file's path when
/// the day was written or held, the refusal's words when it was refused.
fn event_of<'a>(
    result: &'a Result<Outcome, Refusal>,
    zip: &'a str,
    options: &'a WriteOptions,
    millis: u64,
    words: &'a str,
) -> telemetry::Event<'a> {
    match result {
        Ok(outcome) => {
            let r = outcome.report();
            telemetry::Event::info("pull.tickstore.write", outcome.verdict())
                .with("day", telemetry::Value::Str(words))
                .with("stored_bytes", telemetry::Value::Uint(r.stored_bytes))
                .with("csv_bytes", telemetry::Value::Uint(r.csv_bytes))
                .with("entries", telemetry::Value::Uint(r.entries))
                .with("raw_fallbacks", telemetry::Value::Uint(r.raw_fallbacks))
                .with("zip", telemetry::Value::Str(zip))
                .with("writer", telemetry::Value::Str(WRITER))
                .with("commit", telemetry::Value::Str(options.commit()))
                .with("run", telemetry::Value::Str(&options.run_id))
                .with("millis", telemetry::Value::Uint(millis))
        }
        Err(why) => telemetry::Event::warn("pull.tickstore.write", "refused")
            .with("code", telemetry::Value::Str(why.code()))
            .with("why", telemetry::Value::Str(words))
            .with("zip", telemetry::Value::Str(zip))
            .with("writer", telemetry::Value::Str(WRITER))
            .with("commit", telemetry::Value::Str(options.commit()))
            .with("run", telemetry::Value::Str(&options.run_id))
            .with("millis", telemetry::Value::Uint(millis)),
    }
}

/// The protocol, without the log line and the event.
fn write_day_inner(zip: &Path, root: &Path, options: &WriteOptions) -> Result<Outcome, Refusal> {
    if !run_id_ok(&options.run_id) {
        return Err(Refusal::RunIdInvalid {
            run_id: options.run_id.clone(),
        });
    }
    if !run_id_ok(options.commit()) {
        return Err(Refusal::CommitInvalid {
            commit: options.commit().to_owned(),
        });
    }
    let name = zip
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let identity = DayIdentity::of_zip_name(&name)?;
    // Through the store's one reading door (R3-16): a pipe at the zip's name
    // is refused, never waited on; a link is never followed.
    let (source, zip_len) = store::safe_open::open_read_len(zip).map_err(|why| {
        opened_refusal(&why, |kind| Refusal::ZipUnavailable {
            path: zip.to_path_buf(),
            kind,
        })
    })?;
    let entries = zip_entries(&source, 0, zip_len).map_err(|why| Refusal::ZipRefused { why })?;
    let outside_folder = preflight(&identity, &entries)?;
    let final_path = identity.path_under(root);
    let dir = final_path.parent().unwrap_or(root).to_path_buf();
    match std::fs::metadata(root) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            return Err(Refusal::RootMissing {
                root: root.to_path_buf(),
                what: "not a directory".to_owned(),
            });
        }
        Err(why) => {
            return Err(Refusal::RootMissing {
                root: root.to_path_buf(),
                what: why.kind().to_string(),
            });
        }
    }
    let lock = lock_root(root)?;
    refuse_linked_folders(root, &dir)?;
    let finish = |mut report: DayReport| {
        report.zip_bytes = zip_len;
        report.outside_folder = outside_folder;
        report
    };
    match store::safe_open::open_read_len(&final_path) {
        Ok((held, len)) => {
            return verify_open(&held, len, &final_path, &source, &entries, options.workers)
                .map(|report| Outcome::Held(finish(report)))
                .map_err(|why| match why {
                    Mismatch::File(why) => Refusal::HeldDayDiffers {
                        path: final_path.clone(),
                        why,
                    },
                    Mismatch::Zip(why) => Refusal::ZipCorruptOnVerify {
                        path: final_path.clone(),
                        why,
                    },
                });
        }
        Err(why) if why.is_missing() => {}
        Err(why) => {
            return Err(opened_refusal(&why, |kind| Refusal::WriteFailed {
                path: final_path.clone(),
                step: "open the held day",
                kind,
            }));
        }
    }
    if dir.exists() {
        let left = leftovers(&dir, &identity.stem)?;
        if !left.is_empty() {
            return Err(Refusal::StaleTemp { leftovers: left });
        }
    }
    let items = encode_all(&source, &entries, options.workers)?;
    // The day's folder is made only now: a refusal above writes nothing.
    make_dirs(&dir)?;
    let temp = dir.join(format!("{}{TMP_SUFFIX}.{}", identity.stem, options.run_id));
    if let Some(plant) = options.faults.plant_before_create {
        plant_at(&temp, root, plant)?;
    }
    write_temp(&temp, &entries, &items, options.faults)?;
    drop(items);
    if let Some(at) = options.faults.flip_before_verify {
        flip(&temp, at)?;
    }
    let report = verify_file(&temp, &source, &entries, options.workers).map_err(|why| {
        Refusal::VerifyFailed {
            temp: temp.clone(),
            why,
        }
    })?;
    publish(&temp, &final_path, &dir)?;
    drop(lock);
    let mut report = finish(report);
    report.path = final_path;
    Ok(Outcome::Written(report))
}

/// The root's [`LOCK_NAME`], opened (never removed) and locked for this run;
/// released when the returned handle is dropped or the process dies.
fn lock_root(root: &Path) -> Result<File, Refusal> {
    let lock_path = root.join(LOCK_NAME);
    if let Some(found) = not_regular(&lock_path) {
        return Err(Refusal::NotRegularFile {
            path: lock_path,
            found,
        });
    }
    let lock = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&lock_path)
        .map_err(|why| Refusal::LockFailed {
            lock: lock_path.clone(),
            kind: why.kind(),
        })?;
    match lock.try_lock() {
        Ok(()) => Ok(lock),
        Err(std::fs::TryLockError::WouldBlock) => Err(Refusal::DayLocked { lock: lock_path }),
        Err(std::fs::TryLockError::Error(why)) => Err(Refusal::LockFailed {
            lock: lock_path,
            kind: why.kind(),
        }),
    }
}

/// A refusal of `store::safe_open` as this writer's: what is at the name
/// when it is not a regular file, else `other` with the host's error kind.
fn opened_refusal(why: &ReadRefusal, other: impl FnOnce(std::io::ErrorKind) -> Refusal) -> Refusal {
    if let Some(found) = why.found() {
        return Refusal::NotRegularFile {
            path: why.path().to_path_buf(),
            found,
        };
    }
    other(match why {
        ReadRefusal::Io { source, .. } => source.kind(),
        _ => std::io::ErrorKind::Other,
    })
}

/// Every folder from `dir` up to, and not including, `root` that exists is
/// a folder and not a symbolic link (R3-16's rule for a store, RA-298, on
/// the tick store): at most one look per level, four at most for any day.
fn refuse_linked_folders(root: &Path, dir: &Path) -> Result<(), Refusal> {
    for level in dir.ancestors().take_while(|level| *level != root) {
        if std::fs::symlink_metadata(level).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err(Refusal::LinkedFolder {
                path: level.to_path_buf(),
            });
        }
    }
    Ok(())
}

/// `dir` made with every missing folder above it, and the folder holding
/// each new one fsynced, outermost first, so a power loss cannot take back a
/// folder, and the day in it, that a log line already called written. The
/// folders fsynced are returned. One look per missing level of the path,
/// which stops at the root (it is never made).
fn make_dirs(dir: &Path) -> Result<Vec<PathBuf>, Refusal> {
    let mut missing = Vec::new();
    let mut at = Some(dir);
    while let Some(here) = at {
        match std::fs::symlink_metadata(here) {
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                missing.push(here);
                at = here.parent();
            }
            _ => break,
        }
    }
    std::fs::create_dir_all(dir).map_err(|why| Refusal::WriteFailed {
        path: dir.to_path_buf(),
        step: "create the day's folder",
        kind: why.kind(),
    })?;
    let mut synced = Vec::with_capacity(missing.len());
    for made in missing.iter().rev() {
        let parent = made.parent().unwrap_or(made);
        File::open(parent)
            .and_then(|handle| handle.sync_all())
            .map_err(|why| Refusal::WriteFailed {
                path: parent.to_path_buf(),
                step: "fsync a new folder's parent",
                kind: why.kind(),
            })?;
        synced.push(parent.to_path_buf());
    }
    Ok(synced)
}

/// The race hook: `plant` put at `temp` (test-only by its caller's flag).
fn plant_at(temp: &Path, root: &Path, plant: Plant) -> Result<(), Refusal> {
    let failed = |why: std::io::Error| Refusal::WriteFailed {
        path: temp.to_path_buf(),
        step: "race hook",
        kind: why.kind(),
    };
    match plant {
        Plant::File(len) => std::fs::write(temp, vec![b'x'; usize::try_from(len).unwrap_or(0)]),
        Plant::HardLink => {
            let other = root.join("planted");
            std::fs::write(&other, b"another file").and_then(|()| std::fs::hard_link(&other, temp))
        }
    }
    .map_err(failed)
}

/// The verified temporary renamed to the day's name and the folder fsynced:
/// the moment the day is held.
fn publish(temp: &Path, final_path: &Path, dir: &Path) -> Result<(), Refusal> {
    std::fs::rename(temp, final_path).map_err(|why| Refusal::WriteFailed {
        path: temp.to_path_buf(),
        step: "rename",
        kind: why.kind(),
    })?;
    File::open(dir)
        .and_then(|handle| handle.sync_all())
        .map_err(|why| Refusal::WriteFailed {
            path: dir.to_path_buf(),
            step: "fsync the folder",
            kind: why.kind(),
        })
}

/// Every member encoded on the run's workers, held to [`DAY_BLOCKS_CAP`].
fn encode_all<B: ReadAt + Sync>(
    zip: &B,
    entries: &[ZipEntry],
    workers: usize,
) -> Result<Vec<Item>, Refusal> {
    let held = std::sync::atomic::AtomicU64::new(0);
    parallel(entries.len(), workers, &|at| {
        let Some(entry) = entries.get(at) else {
            return Err(Refusal::EncodeFailed {
                name: String::new(),
                why: "an entry went missing".to_owned(),
            });
        };
        let built = item(zip, entry)?;
        let bytes =
            held.fetch_add(wide(built.block.len()), Ordering::Relaxed) + wide(built.block.len());
        if bytes > DAY_BLOCKS_CAP {
            return Err(Refusal::DayTooLarge { bytes });
        }
        Ok(built)
    })
}

/// The fault hook: the byte at `at` of `temp` inverted, in place, as a torn
/// or rotted write would leave it. Test-only by its caller's flag.
fn flip(temp: &Path, at: u64) -> Result<(), Refusal> {
    let failed = |why: std::io::Error| Refusal::WriteFailed {
        path: temp.to_path_buf(),
        step: "fault hook",
        kind: why.kind(),
    };
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(temp)
        .map_err(failed)?;
    let mut byte = [0_u8; 1];
    std::os::unix::fs::FileExt::read_exact_at(&file, &mut byte, at).map_err(failed)?;
    let byte = byte.map(|b| !b);
    std::os::unix::fs::FileExt::write_all_at(&file, &byte, at).map_err(failed)
}

#[cfg(test)]
#[path = "tickstore_write_tests.rs"]
mod tests;
