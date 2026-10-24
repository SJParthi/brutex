//! `pull::gdfl_run::preflight`: the refusals IA-3 left (IA-3b, D-7000 to
//! D-7009; the design's D-4802, D-4819, D-4850, D-4864 and the critic's
//! D-4908), run once per run inside [`super::plan`], after the scope is
//! resolved and before any lock, claim or journal line.
//!
//! # What it checks, in order
//!
//! Per kind, of its source:
//!
//! 1. **The year zips** (`nfo_zips`, FM-060's remainder, design §8 A9, A11,
//!    A23): every year of the range has exactly `<yyyy>.zip`
//!    ([`RunRefusal::YearZipMissing`], with at most [`NEAR_MISS_MAX`] names
//!    that look like it), readable by one positional byte of its end record
//!    ([`RunRefusal::SourceUnreadable`]), and its end record states at least
//!    one entry ([`RunRefusal::YearZipEmpty`]).
//! 2. **The tick store's writer lock** (`tickstore`, RA-323): its root's
//!    [`crate::tickstore_write::LOCK_NAME`] probed shared and non-blocking,
//!    opened without create ([`RunRefusal::SourceBusy`]).
//! 3. **Where the source begins** (FM-133): the span of the day table the
//!    source already builds, and the asked calendar-open days outside it
//!    counted `outside_source`, never as holes.
//!
//! Then of the store, once:
//!
//! 4. **The legacy cutover** (D-4864, D-4908, IA-GR-20):
//!    [`RunRefusal::LegacyRunMayBeLive`] when no claim lock file exists and the
//!    journal holds a key of the kind whose last line is `begin`.
//! 5. **The store the run writes** (RA-314, RA-340, RA-306), always after 4:
//!    [`RunRefusal::StoreReadOnly`], [`RunRefusal::StoreNotLocal`],
//!    [`RunRefusal::OpenFilesShort`].
//!
//! Then per kind, of the census and the first planned day:
//!
//! 6. **Census room** (D-4802, D-4850, IA-GR-15, IA-GR-26, FM-096):
//!    [`RunRefusal::CensusRoomShort`] and
//!    [`RunRefusal::CensusCompactionNoDisk`] from two header slot reads and
//!    the first planned day's listing.
//! 7. **The tick store's format digest** (D-4819): not a refusal; an
//!    unpinned digest is warned and the run line carries
//!    `format_digest_unpinned=1`.
//!
//! # What it does not do
//!
//! A root that is no source of its kind at all (no `<yyyy>.zip`, no
//! `FORMAT.md`, no day zip of its tree) is AF-01's `source_root_unknown`,
//! said by the cli's door right after the plan (`gdfl_import_guard.rs`): this
//! module names what is wrong inside a root that is one, and never rebuilds
//! that refusal. Nothing here caches across runs, watches a folder or runs on
//! a timer (D-6900).
//!
//! # The one identity of a source file (item 4, RA-304, RA-323)
//!
//! [`SourceIdentity`] is the one identity a source file is judged by: its
//! length, inode, modification and change times, never its device (which
//! changed on a remount, CC-2). [`Watch`] takes it when a file's listing is
//! read, through the one handle every later read uses, and compares the
//! handle's and the path's identities with it after each read: a file still
//! being copied, rewritten in place or replaced by rename refuses the day
//! read from it [`CmRefusal::SourceChanging`], which never closes a day.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use brutex_core::instrument::Segment;
use brutex_core::vendor::Vendor;
use store::format::SLOT_STRIDE;
use telemetry::run_dirs::GdflLayout;

use super::{KindPlan, Process, Request, RunRefusal, SourceId, locks};
use crate::disk_projection::{Floor, FloorRefusal, Space, floor_check};
use crate::gdfl_archive::{Archive, stated_entries};
use crate::gdfl_cm::{CmKind, CmRefusal, CmSource, DayListing};
use crate::gdfl_import::{ImportKind, Origin, SourceKind};
use crate::gdfl_nfo::{NfoDay, NfoSource, NfoTickStore, NfoZips, decode_ticker};
use crate::gdfl_tickstore::TickStore;
use crate::ingest::census_store;
use crate::manifest::{self, ManifestHeader, epoch_capacity};
use crate::session::Day;

// ═══ The identity of a source file ══════════════════════════════════════════

/// A source file's identity (IA-3b item 4): what two reads of it must agree
/// on for them to be reads of one file. Without the device, which a remount
/// changes for the same bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceIdentity {
    /// Its length in bytes.
    pub len: u64,
    /// Its inode.
    pub ino: u64,
    /// Its modification time, seconds and nanoseconds since the epoch.
    pub mtime: (i64, i64),
    /// Its change time (metadata or contents), seconds and nanoseconds.
    pub ctime: (i64, i64),
}

impl SourceIdentity {
    /// The identity `meta` states.
    #[must_use]
    pub fn of(meta: &std::fs::Metadata) -> Self {
        use std::os::unix::fs::MetadataExt as _;
        Self {
            len: meta.len(),
            ino: meta.ino(),
            mtime: (meta.mtime(), meta.mtime_nsec()),
            ctime: (meta.ctime(), meta.ctime_nsec()),
        }
    }
}

impl core::fmt::Display for SourceIdentity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "len={} ino={} mtime={}.{:09} ctime={}.{:09}",
            self.len, self.ino, self.mtime.0, self.mtime.1, self.ctime.0, self.ctime.1
        )
    }
}

/// Why a [`Watch`] could not say its file is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchFault {
    /// The handle or the path names another identity than the listing's.
    Changed(CmRefusal),
    /// The handle or the path could not be measured: the reader's own
    /// "unavailable" refusal, so the run's loss probe asks the root (D-4852).
    Unreachable(std::io::ErrorKind),
}

/// One source file, from the read of its listing to its last read (IA-3b
/// item 4): its path, the one handle every read goes through, and the
/// identity the handle had when the listing was read. Two stats a check:
/// the handle's and the path's.
#[derive(Debug)]
pub struct Watch {
    path: PathBuf,
    handle: Arc<std::fs::File>,
    listed: SourceIdentity,
}

impl Watch {
    /// Watches `handle`, opened at `path`, from now: one `fstat`.
    ///
    /// # Errors
    ///
    /// The host's refusal of the `fstat`.
    pub fn new(path: &Path, handle: Arc<std::fs::File>) -> std::io::Result<Self> {
        let listed = SourceIdentity::of(&handle.metadata()?);
        Ok(Self {
            path: path.to_path_buf(),
            handle,
            listed,
        })
    }

    /// The identity taken when the listing was read.
    #[must_use]
    pub const fn listed(&self) -> SourceIdentity {
        self.listed
    }

    /// The watched file's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the file is still the one listed: the handle's identity (a
    /// file grown or rewritten in place) and the path's (a file replaced by
    /// rename, even with its length and modification time kept) both equal
    /// the listing's. Two stats, O(1).
    ///
    /// # Errors
    ///
    /// [`WatchFault::Changed`] naming the path and both identities, and
    /// [`WatchFault::Unreachable`] when either stat is refused.
    pub fn check(&self) -> Result<(), WatchFault> {
        let unreachable = |why: std::io::Error| WatchFault::Unreachable(why.kind());
        let held = SourceIdentity::of(&self.handle.metadata().map_err(unreachable)?);
        let named = SourceIdentity::of(&std::fs::metadata(&self.path).map_err(unreachable)?);
        // The path's identity is what the next reader would read, so it is
        // the one named when it moved (a rename names the new inode); the
        // handle's alone moves when the listed file changed under its name
        // without the name moving (its link count dropped, say).
        let after = if named == self.listed { held } else { named };
        if after == self.listed {
            return Ok(());
        }
        Err(WatchFault::Changed(CmRefusal::SourceChanging {
            path: self.path.display().to_string(),
            before: self.listed.to_string(),
            after: after.to_string(),
        }))
    }
}


// ═══ The constants ══════════════════════════════════════════════════════════

/// The most names beside a missing year zip that
/// [`RunRefusal::YearZipMissing`] names (design §5.2: "≤4 names").
pub const NEAR_MISS_MAX: usize = 4;

/// The two sha256 digests of the tick store's `FORMAT.md` the charter
/// records (`docs/00-charter.md`, the GDFL section): format version 1 as
/// frozen on 2026-10-04, and its §6 as revised the same day. Both are quoted
/// whole in `gdfl_tickstore.rs`'s module doc; a test holds the three copies to
/// one another (D-4819).
pub const PINNED_FORMAT_DIGESTS: [&str; 2] = [
    "6edc62a772b257a2aa1b45c0785a23936d9bff47b87648bbb1d9a84ab7063259",
    "d21139f55789315c5a8280432f8074ea613f17efe8d1a838b325d193adec5be9",
];

/// The fix a store that takes no write names, the one sentence IA-16's
/// `store_not_writable` (FM-129) and [`RunRefusal::StoreReadOnly`] both end in.
pub const NOT_WRITABLE_FIX: &str = "clear the file's immutable flag (`chflags nouchg <file>`; \
     `ls -lO` shows it), remove a restrictive ACL (`chmod -N <file>`; `ls -le` shows it), or \
     remount the volume read-write, then resume";

/// The descriptors one worker of an import holds at once beyond the run's
/// own (L-IA-35, IA-GR-08): CITED, the design's count of one month handle
/// (its pointer, `.bin`, `.lock`, `.crc` and `.tix`), a worker filing one
/// month at a time. Checked against a measurement, never typed alone:
/// `the_descriptors_a_run_holds_are_measured_at_one_and_four_workers`
/// (`gdfl_run_preflight_tests.rs`) samples the process's descriptors
/// through an invented 400-file options day and fails when a run holds more
/// than [`files_needed`] says. Sampled on 24 Oct 2026 (Linux, three runs): 32
/// at one and at four workers, 36 to 37 at twelve, beyond the 3 held before
/// (`docs/06-limits.md` L-IA3B-04). A sample is a lower bound of the peak,
/// which is why the per-worker count is the structural one.
pub const FILES_PER_WORKER: u64 = 5;

/// The descriptors an import run holds beside its workers' (the journal, the
/// census, the source's handles, the store's directories): MEASURED, the
/// one-worker peak of the sample above, so it already holds one worker's
/// handles. The cli's own (its event sink, the claim lock, the runs ledger,
/// the progress record) are not in that sample: NOT MEASURED, NEEDS THE MAC
/// M7, named in `docs/06-limits.md` L-IA3B-04.
pub const FILES_PER_RUN: u64 = 32;

/// What a run of `workers` holds open at once (RA-306, L-IA-35): the run's
/// own plus each worker's, as measured.
#[must_use]
pub const fn files_needed(workers: usize) -> u64 {
    let workers = workers as u64;
    FILES_PER_RUN.saturating_add(FILES_PER_WORKER.saturating_mul(workers))
}

// ═══ The format digest (item 9) ═════════════════════════════════════════════

/// Whether `origin` is a tick store whose `FORMAT.md` digest was measured
/// and is neither digest the charter records (D-4819): the run line then
/// carries `format_digest_unpinned=1` and the cli warns. Never a refusal: the
/// magic decides whether a day file is read.
#[must_use]
pub fn unpinned(origin: &Origin) -> bool {
    let [frozen, revised] = PINNED_FORMAT_DIGESTS;
    origin.source == SourceKind::TickStore && origin.digest.is_some() && {
        let hex = origin.digest_hex();
        hex != frozen && hex != revised
    }
}

/// What the preflight says of a source's `FORMAT.md` digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    /// The source is not a tick store.
    NotATickStore,
    /// The digest could not be measured (`Origin::fault` names why).
    Unmeasured,
    /// One of the charter's two.
    Pinned,
    /// Another digest, in hex: warned, and the run line marked.
    Unpinned(String),
}

impl Format {
    /// The format of `origin`.
    #[must_use]
    pub fn of(origin: &Origin) -> Self {
        if origin.source != SourceKind::TickStore {
            Self::NotATickStore
        } else if origin.digest.is_none() {
            Self::Unmeasured
        } else if unpinned(origin) {
            Self::Unpinned(origin.digest_hex())
        } else {
            Self::Pinned
        }
    }

    /// The word the plan line says it by.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::NotATickStore => "-",
            Self::Unmeasured => "unmeasured",
            Self::Pinned => "pinned",
            Self::Unpinned(_) => "unpinned",
        }
    }
}

// ═══ The host (item 7's seam) ═══════════════════════════════════════════════

/// The store volume as its own flags state them (item 7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    /// Mounted read-only.
    pub read_only: bool,
    /// Local (`Some(true)`), not local (`Some(false)`), or not measurable on
    /// this host (`None`, named in the plan line and `docs/06-limits.md`).
    pub local: Option<bool>,
    /// The filesystem as the volume names itself.
    pub fs: String,
}

/// What the preflight asks the host: one seam, so a test can answer what a
/// developer's machine never does (a read-only or network volume, a soft
/// limit of 256).
pub trait Host {
    /// The volume `path` is on.
    ///
    /// # Errors
    ///
    /// The sentence naming the call that refused.
    fn volume(&self, path: &Path) -> Result<Volume, String>;

    /// The process's soft open-file limit; `None` when unlimited.
    fn open_files(&self) -> Option<u64>;

    /// The free space of the volume `path` is on.
    ///
    /// # Errors
    ///
    /// The sentence naming the call that refused.
    fn space(&self, path: &Path) -> Result<Space, String>;
}

/// The host this process runs on: `statfs(2)` on macOS, `statvfs(2)` and
/// `statfs(2)` elsewhere, `getrlimit(2)`, all through `rustix`'s safe
/// wrappers (no child process, no shell, no interpreter; D-7004).
#[derive(Debug, Clone, Copy, Default)]
pub struct Os;

impl Host for Os {
    fn volume(&self, path: &Path) -> Result<Volume, String> {
        volume_of(path)
    }

    fn open_files(&self) -> Option<u64> {
        rustix::process::getrlimit(rustix::process::Resource::Nofile).current
    }

    fn space(&self, path: &Path) -> Result<Space, String> {
        crate::gdfl_import::volume_space(path)
    }
}

/// The volume `path` is on, from ONE `statfs(2)`: `MNT_RDONLY` and
/// `MNT_LOCAL` in `f_flags`, and `f_fstypename` (Apple's `<sys/mount.h>`; the
/// values as `libc` 0.2.189 binds them, `src/unix/bsd/mod.rs` and
/// `src/unix/bsd/apple/mod.rs`; D-7004).
#[cfg(target_vendor = "apple")]
fn volume_of(path: &Path) -> Result<Volume, String> {
    const MNT_RDONLY: u32 = 0x0000_0001;
    const MNT_LOCAL: u32 = 0x0000_1000;
    let fs = rustix::fs::statfs(path)
        .map_err(|why| format!("statfs({}) refused: {why}", path.display()))?;
    let name: String = fs
        .f_fstypename
        .iter()
        .map(|c| u8::from_ne_bytes(c.to_ne_bytes()))
        .take_while(|byte| *byte != 0)
        .map(char::from)
        .collect();
    Ok(Volume {
        read_only: fs.f_flags & MNT_RDONLY != 0,
        local: Some(fs.f_flags & MNT_LOCAL != 0),
        fs: name,
    })
}

/// The volume `path` is on: read-only from `statvfs(2)`'s `ST_RDONLY`
/// (POSIX); local only where an exact read exists, Linux's `statfs(2)`
/// `f_type` against `NFS_SUPER_MAGIC` (`rustix::fs`), so an NFS store is named
/// and every other filesystem's locality is `None`, NOT DONE by name in
/// `docs/06-limits.md` rather than guessed from a list of magics (D-7004).
#[cfg(not(target_vendor = "apple"))]
fn volume_of(path: &Path) -> Result<Volume, String> {
    let vfs = rustix::fs::statvfs(path)
        .map_err(|why| format!("statvfs({}) refused: {why}", path.display()))?;
    let read_only = vfs
        .f_flag
        .contains(rustix::fs::StatVfsMountFlags::RDONLY);
    #[cfg(any(target_os = "linux", target_os = "android"))]
    let (local, fs) = {
        let fs = rustix::fs::statfs(path)
            .map_err(|why| format!("statfs({}) refused: {why}", path.display()))?;
        if fs.f_type == rustix::fs::NFS_SUPER_MAGIC {
            (Some(false), "nfs".to_owned())
        } else {
            (None, format!("f_type {:#x}", fs.f_type))
        }
    };
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    let (local, fs) = (None, "unmeasured on this host".to_owned());
    Ok(Volume {
        read_only,
        local,
        fs,
    })
}

// ═══ What the preflight found ═══════════════════════════════════════════════

/// The census's room for a kind's first planned day (item 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Room {
    /// Measured from the census header (two slot reads) and the first
    /// planned day's listing.
    Measured {
        /// `v3` or `v4`.
        layout: &'static str,
        /// The entries free now.
        free_now: u64,
        /// The entries free after the one compaction a v4 partition makes
        /// (v3: `free_now`).
        free_after: u64,
        /// The first planned day's wanted files.
        need_first_day: u64,
        /// Whether only the compaction gives the room (v4).
        compaction: bool,
        /// The first planned day the room would not reach, when the range
        /// may need more than it: an EXTRAPOLATION FROM THE FIRST DAY (every
        /// later day assumed to want as many files), never a measurement.
        projected_stop: Option<Day>,
    },
    /// Not measured, and why: no planned day the source holds, or the first
    /// one's listing refused (that day's refusal is the run's, by name).
    Unmeasured(String),
}

/// What the preflight found of one kind, beside what it refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// The kind.
    pub kind: ImportKind,
    /// The source's first and last day (item 8); `None` when it could not
    /// be opened or holds none.
    pub span: Option<(Day, Day)>,
    /// The asked calendar-open days outside that span (item 8): days the
    /// source does not begin or end at, never holes.
    pub outside_source: u32,
    /// The first planned day: the first calendar-open day of the range the
    /// source holds.
    pub first_day: Option<Day>,
    /// The census's room for it (item 5).
    pub room: Room,
    /// The format digest (item 9).
    pub format: Format,
    /// The store's volume (item 7), or why it could not be measured.
    pub volume: Result<Volume, String>,
    /// The soft open-file limit (`None`: unlimited) and what the run needs.
    pub open_files: (Option<u64>, u64),
    /// Whether the operator acknowledged a legacy open key (item 6).
    pub legacy_ack: bool,
}

impl Checked {
    /// The plan's one line for this kind, as the cli says it.
    #[must_use]
    pub fn line(&self) -> String {
        let span = self
            .span
            .map_or_else(|| "unmeasured".to_owned(), |(a, b)| format!("{a}..{b}"));
        let first = self
            .first_day
            .map_or_else(|| "-".to_owned(), |day| day.to_string());
        let room = match &self.room {
            Room::Measured {
                layout,
                free_now,
                free_after,
                need_first_day,
                compaction,
                projected_stop,
            } => format!(
                "census={layout} free_now={free_now} free_after={free_after} \
                 need_first_day={need_first_day} compaction={} census.projected_stop={}",
                u8::from(*compaction),
                projected_stop.map_or_else(
                    || "-".to_owned(),
                    |day| format!("{day} (extrapolation from the first day)")
                )
            ),
            Room::Unmeasured(why) => format!("census=unmeasured ({why})"),
        };
        let volume = match &self.volume {
            Ok(volume) => format!(
                "store_fs={} store_read_only={} store_local={}",
                volume.fs,
                u8::from(volume.read_only),
                volume
                    .local
                    .map_or("unmeasured", |local| if local { "1" } else { "0" })
            ),
            Err(why) => format!("store_volume=unmeasured ({why})"),
        };
        let (have, need) = self.open_files;
        let have = have.map_or_else(|| "unlimited".to_owned(), |have| have.to_string());
        format!(
            "preflight {}: source_span={span} outside_source={} first_day={first} {room} \
             format={} {volume} open_files={have} need={need} legacy_ack={}",
            self.kind.as_str(),
            self.outside_source,
            self.format.word(),
            u8::from(self.legacy_ack),
        )
    }
}

// ═══ The checks ═════════════════════════════════════════════════════════════

/// Every check of this module for `kinds` of `request` in `process`, on this
/// host: [`check_with`] with [`Os`]. The one call [`super::plan`] makes.
///
/// # Errors
///
/// The first refusal, in the module doc's order.
pub fn run(
    request: &Request,
    process: &Process<'_>,
    kinds: &[KindPlan],
) -> Result<Vec<Checked>, RunRefusal> {
    check_with(request, process, kinds, &Os)
}

/// Every check of this module, asking `host` what the host is. One event
/// either way at [`super::TARGET`]: `preflight passed` at `info`, with each
/// kind's line, or `preflight refused` at `error`, with the code and the
/// refusal's words, so a refusal is on `/logs` as well as on the cli's
/// `GDFL IMPORT REFUSED` line.
///
/// # Errors
///
/// The first refusal, in the module doc's order.
pub fn check_with(
    request: &Request,
    process: &Process<'_>,
    kinds: &[KindPlan],
    host: &dyn Host,
) -> Result<Vec<Checked>, RunRefusal> {
    let started = std::time::Instant::now();
    let checked = checks(request, process, kinds, host);
    let micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let _dropped_when_filtered = match &checked {
        Ok(found) => {
            let lines: Vec<String> = found.iter().map(Checked::line).collect();
            telemetry::emit(
                &telemetry::Event::info(super::TARGET, "preflight passed")
                    .with("kinds", found.len())
                    .with("micros", micros)
                    .with("checked", lines.join("; ").as_str()),
            )
        }
        Err(refused) => {
            let words = refused.to_string();
            telemetry::emit(
                &telemetry::Event::error(super::TARGET, "preflight refused")
                    .with("code", refused.code())
                    .with("micros", micros)
                    .with("why", words.as_str()),
            )
        }
    };
    checked
}

/// The checks in order: each kind's source, then the store once (the
/// legacy cutover before the volume and the limit), then each kind's census
/// room, span and digest.
fn checks(
    request: &Request,
    process: &Process<'_>,
    kinds: &[KindPlan],
    host: &dyn Host,
) -> Result<Vec<Checked>, RunRefusal> {
    let range = (request.from, request.to);
    for kind in kinds {
        match kind.source {
            SourceId::NfoZips => year_zips(&kind.root, range)?,
            SourceId::TickStore => writer_lock(&kind.root)?,
            SourceId::CmZip => {}
        }
    }
    let store = process.store_root;
    let layout = locks::layout_at(store)?;
    for kind in kinds {
        legacy(store, layout, kind.kind, request.legacy_ack)?;
    }
    let volume = host.volume(store);
    let need = files_needed(request.workers.get());
    let have = host.open_files();
    for kind in kinds {
        writable(store, layout, kind.kind, volume.as_ref().ok())?;
    }
    if let Ok(found) = &volume
        && found.local == Some(false)
    {
        return Err(RunRefusal::StoreNotLocal {
            root: store.to_path_buf(),
            fs: found.fs.clone(),
        });
    }
    if let Some(have) = have
        && have < need
    {
        return Err(RunRefusal::OpenFilesShort { have, need });
    }
    let mut found = Vec::with_capacity(kinds.len());
    for kind in kinds {
        let looked = look(kind, range);
        let room = room(store, layout, kind.kind, &looked, request, host)?;
        found.push(Checked {
            kind: kind.kind,
            span: looked.span,
            outside_source: outside(looked.span, range),
            first_day: looked.first.as_ref().map(|first| first.0),
            room,
            format: Format::of(&kind.origin),
            volume: volume.clone(),
            open_files: (have, need),
            legacy_ack: request.legacy_ack,
        });
    }
    Ok(found)
}

// ── 1, 2: the year zips ─────────────────────────────────────────────────────

/// `<yyyy>.zip`'s year, for exactly that name.
fn year_of(name: &str) -> Option<u16> {
    let digits = name.strip_suffix(".zip")?;
    (digits.len() == 4 && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}

/// Every year of `range` has exactly `<yyyy>.zip` in `root`, a regular file
/// one positional byte of whose end record reads, whose end record states at
/// least one entry (items 1 and 2). One listing of the directory (O(names in
/// it), once per run), then per year one open, one byte and the end record
/// (O(1) each). A root that cannot be listed or holds no `<yyyy>.zip` at all
/// is AF-01's `source_root_unknown`, said by the door: nothing here.
fn year_zips(root: &Path, (from, to): (Day, Day)) -> Result<(), RunRefusal> {
    let Ok(listing) = std::fs::read_dir(root) else {
        return Ok(());
    };
    let mut years: BTreeMap<u16, PathBuf> = BTreeMap::new();
    let mut others: BTreeSet<String> = BTreeSet::new();
    for entry in listing {
        let Ok(entry) = entry else {
            return Ok(());
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        match year_of(&name) {
            Some(year) if std::fs::metadata(entry.path()).is_ok_and(|meta| meta.is_file()) => {
                let _first = years.insert(year, entry.path());
            }
            _ => {
                let _new = others.insert(name);
            }
        }
    }
    if years.is_empty() {
        return Ok(());
    }
    for year in from.year()..=to.year() {
        let Some(path) = years.get(&year) else {
            let digits = format!("{year:04}");
            let near_miss = others
                .iter()
                .filter(|name| {
                    let lower = name.to_ascii_lowercase();
                    lower.starts_with(&digits) && lower.ends_with(".zip")
                })
                .take(NEAR_MISS_MAX)
                .cloned()
                .collect();
            return Err(RunRefusal::YearZipMissing { year, near_miss });
        };
        year_zip(year, path)?;
    }
    Ok(())
}

/// One year zip: opened, its last byte (the end record's) read by position,
/// and its end record's entry count asked ([`stated_entries`]). A zip whose
/// end record cannot be found is left to the day reader, which refuses it per
/// day by name (`archive_malformed`): a copy still being written has none.
fn year_zip(year: u16, path: &Path) -> Result<(), RunRefusal> {
    let unreadable = || RunRefusal::SourceUnreadable {
        path: path.to_path_buf(),
        mode: mode_of(path),
    };
    let file = std::fs::File::open(path).map_err(|_| unreadable())?;
    let len = file.metadata().map_err(|_| unreadable())?.len();
    let Some(last) = len.checked_sub(1) else {
        return Err(RunRefusal::YearZipEmpty { year });
    };
    let mut byte = [0_u8; 1];
    std::os::unix::fs::FileExt::read_exact_at(&file, &mut byte, last).map_err(|_| unreadable())?;
    match stated_entries(&file, len) {
        Ok(0) => Err(RunRefusal::YearZipEmpty { year }),
        Ok(_) | Err(_) => Ok(()),
    }
}

/// `path`'s permission bits in octal, or why they could not be read.
fn mode_of(path: &Path) -> String {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::metadata(path).map_or_else(
        |why| format!("unmeasured ({why})"),
        |meta| format!("{:04o}", meta.permissions().mode() & 0o7777),
    )
}

// ── 4: the tick store's writer lock ─────────────────────────────────────────

/// The tick store's root lock, probed shared and non-blocking through a
/// handle opened read-only, so the probe never creates the file (item 4).
/// Absent or not openable: nothing this process can see holds it. Held
/// exclusively: [`RunRefusal::SourceBusy`]. A probe the host refuses is
/// refused too, since nothing then says the writer is idle.
fn writer_lock(root: &Path) -> Result<(), RunRefusal> {
    let path = root.join(crate::tickstore_write::LOCK_NAME);
    let Ok(file) = std::fs::File::open(&path) else {
        return Ok(());
    };
    let busy = |holder: String| RunRefusal::SourceBusy {
        root: root.to_path_buf(),
        holder,
    };
    match file.try_lock_shared() {
        Ok(()) => {
            let _released = file.unlock();
            Ok(())
        }
        Err(std::fs::TryLockError::WouldBlock) => Err(busy(format!(
            "a writer holds {} exclusively: it is replacing day files",
            path.display()
        ))),
        Err(std::fs::TryLockError::Error(why)) => Err(busy(format!(
            "{} could not be probed ({why}), so no writer is known to be idle",
            path.display()
        ))),
    }
}

// ── 6: the legacy cutover ───────────────────────────────────────────────────

/// D-4864 as D-4908 defines it: a kind HAS CLAIMED on this store when
/// `imports/gdfl.lock` or `imports/gdfl.<kind>.lock` exists (one `metadata`
/// each, never an open with create, since a probe that created one would
/// erase the evidence it reads). Only when neither exists is the journal read
/// (O(its lines), once, the refusal path) for a key whose last line is
/// `begin`: of the kind on a v4 store, where kinds run apart, and of any kind
/// on v3, where they share one claim (D-7003). Acknowledged by
/// `--legacy-run-not-live`, which the claim records.
fn legacy(store: &Path, layout: GdflLayout, kind: ImportKind, ack: bool) -> Result<(), RunRefusal> {
    if ack {
        return Ok(());
    }
    let present = |name: &str| {
        std::fs::symlink_metadata(store.join(name))
            .map_or_else(|why| why.kind() != std::io::ErrorKind::NotFound, |_| true)
    };
    if present(locks::STORE_LOCK) || present(&locks::kind_lock(kind.as_str())) {
        return Ok(());
    }
    let Ok(reader) = super::JournalReader::open(store) else {
        // The run reads the journal again before any day and refuses it by
        // name; there is nothing here to decide on.
        return Ok(());
    };
    let prefix = format!("{} ", kind.as_str());
    let wanted = |key: &str| layout == GdflLayout::V3 || key.starts_with(&prefix);
    match reader.journal.first_open(&wanted) {
        Some(key) => Err(RunRefusal::LegacyRunMayBeLive {
            key: key.to_owned(),
        }),
        None => Ok(()),
    }
}

// ── 7: the store the run writes ─────────────────────────────────────────────

/// The store takes writes (item 7, RA-314): the kind's claim lock file,
/// when it exists, opened for writing WITHOUT create (EROFS or EACCES is
/// [`RunRefusal::StoreReadOnly`]); when it does not, the volume's own
/// read-only flag. Never a file created on the store to test it.
fn writable(
    store: &Path,
    layout: GdflLayout,
    kind: ImportKind,
    volume: Option<&Volume>,
) -> Result<(), RunRefusal> {
    let refused = || RunRefusal::StoreReadOnly {
        root: store.to_path_buf(),
    };
    let lock = store.join(locks::claim_lock(kind.as_str(), layout));
    match std::fs::OpenOptions::new().write(true).open(&lock) {
        Ok(_) => Ok(()),
        Err(why)
            if matches!(
                why.kind(),
                std::io::ErrorKind::ReadOnlyFilesystem | std::io::ErrorKind::PermissionDenied
            ) =>
        {
            Err(refused())
        }
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => match volume {
            Some(volume) if volume.read_only => Err(refused()),
            _ => Ok(()),
        },
        // Anything else at the lock's name (a directory, a link) is the
        // claim's to refuse by name a moment later.
        Err(_) => Ok(()),
    }
}

// ── 8: where the source begins ──────────────────────────────────────────────

/// Whether the calendar the GDFL gate reads by says the exchange traded on
/// `day` (D-3319).
fn trading(day: Day) -> bool {
    matches!(
        crate::calendar::sourced_kind_of(i64::from(day.days_from_epoch())),
        crate::calendar::DayKind::Open(_) | crate::calendar::DayKind::OpenLengthUnmeasured
    )
}

/// The calendar-open days of `range` outside `span` (FM-133): before its
/// first day and after its last, counted, never holes. O(days outside), at
/// most the range's.
fn outside(span: Option<(Day, Day)>, (from, to): (Day, Day)) -> u32 {
    let Some((first, last)) = span else {
        return 0;
    };
    let mut count = 0_u32;
    let mut day = from;
    while day <= to {
        if (day < first || day > last) && trading(day) {
            count = count.saturating_add(1);
        }
        if day >= first && day <= last {
            // Jump the span: nothing inside it is outside.
            let Ok(next) = last.succ() else {
                break;
            };
            day = next;
            continue;
        }
        let Ok(next) = day.succ() else {
            break;
        };
        day = next;
    }
    count
}

/// What the preflight read of a kind's source: its span, and the first
/// planned day with its wanted files (`None` when its listing refused).
#[derive(Debug, Default)]
struct Looked {
    span: Option<(Day, Day)>,
    first: Option<(Day, Option<u64>)>,
}

/// The capital-market tree of `kind` (options never reaches it).
const fn cm_tree(kind: ImportKind) -> CmKind {
    if matches!(kind, ImportKind::Indices) {
        CmKind::Indices
    } else {
        CmKind::Stocks
    }
}

/// `kind`'s source opened once for its span and its first planned day's
/// listing (items 5 and 8). A source that cannot be opened is looked at no
/// further: the run opens it again and refuses it by name.
fn look(kind: &KindPlan, range: (Day, Day)) -> Looked {
    let only = &kind.only;
    match (kind.source, kind.kind) {
        (SourceId::CmZip, cm) => Archive::open(&kind.root).map_or_else(
            |_| Looked::default(),
            |archive| {
                let archive = archive.with_scope(Arc::clone(&kind.scope));
                let tree = cm_tree(cm);
                let span = archive.span(tree);
                Looked {
                    span,
                    first: first_day(span, range, &|day| {
                        archive
                            .day(tree, day)
                            .map(|listing| listing.map(|listing| cm_wanted(&listing, only)))
                    }),
                }
            },
        ),
        (SourceId::TickStore, ImportKind::Options) => {
            let store = NfoTickStore::new(&kind.root);
            let span = store.span();
            Looked {
                span,
                first: first_day(span, range, &|day| {
                    store
                        .day(day)
                        .map(|listing| listing.map(|listing| nfo_wanted(&listing, only)))
                }),
            }
        }
        (SourceId::TickStore, cm) => {
            let store = TickStore::new(&kind.root).with_scope(Arc::clone(&kind.scope));
            let tree = cm_tree(cm);
            let span = store.span(tree);
            Looked {
                span,
                first: first_day(span, range, &|day| {
                    store
                        .day(tree, day)
                        .map(|listing| listing.map(|listing| cm_wanted(&listing, only)))
                }),
            }
        }
        (SourceId::NfoZips, _) => {
            let zips = NfoZips::new(&kind.root);
            let span = zips.span().ok().flatten();
            Looked {
                span,
                first: first_day(span, range, &|day| {
                    zips.day(day)
                        .map(|listing| listing.map(|listing| nfo_wanted(&listing, only)))
                }),
            }
        }
    }
}

/// The first calendar-open day of `range` inside `span` the source holds,
/// with its wanted files: one probe a day from the later of the range's and
/// the span's first day (a day table's O(1) probe, or a day file's open)
/// until one is held, so normally one listing read. A listing that refuses
/// ends the search with `None` files: that day's refusal is the run's.
fn first_day(
    span: Option<(Day, Day)>,
    (from, to): (Day, Day),
    listing: &dyn Fn(Day) -> Result<Option<u64>, CmRefusal>,
) -> Option<(Day, Option<u64>)> {
    let (first, last) = span?;
    let end = to.min(last);
    let mut day = from.max(first);
    while day <= end {
        if trading(day) {
            match listing(day) {
                Ok(Some(wanted)) => return Some((day, Some(wanted))),
                Ok(None) => {}
                Err(_) => return Some((day, None)),
            }
        }
        day = day.succ().ok()?;
    }
    None
}

/// The files a capital-market day files: every entry less those that name
/// no ticker of the scope; under a filter, at most one a name.
fn cm_wanted<L>(listing: &DayListing<L>, only: &[String]) -> u64 {
    let wanted = listing
        .entries()
        .len()
        .saturating_sub(listing.unresolved());
    let wanted = if only.is_empty() {
        wanted
    } else {
        wanted.min(only.len())
    };
    u64::try_from(wanted).unwrap_or(u64::MAX)
}

/// The files an options day files: every option file of the day folder;
/// under a filter, those whose ticker decodes to a named underlying.
fn nfo_wanted<L>(listing: &NfoDay<L>, only: &[String]) -> u64 {
    let wanted = if only.is_empty() {
        listing.entries().len().saturating_sub(listing.skipped())
    } else {
        let names: BTreeSet<&str> = only.iter().map(String::as_str).collect();
        listing
            .entries()
            .iter()
            .filter_map(|file| listing.ticker_of(file))
            .filter_map(|ticker| decode_ticker(ticker, listing.day()).ok())
            .filter(|ticker| names.contains(ticker.underlying.as_str()))
            .count()
    };
    u64::try_from(wanted).unwrap_or(u64::MAX)
}

// ── 5: census room ──────────────────────────────────────────────────────────

/// The census segment a kind files into.
const fn segment_of(kind: ImportKind) -> Segment {
    match kind {
        ImportKind::Indices => Segment::Index,
        ImportKind::Stocks => Segment::Cash,
        ImportKind::Options => Segment::Fno,
    }
}

/// The census header of `kind`'s file or partition: its newest slot that
/// decodes and whose entry count the file's length holds (the writer's own
/// fall-back order), from TWO slot reads (`manifest.rs`'s
/// `ManifestHeader`, D-4850). `None` for a census not yet written (no file,
/// or a v4 partition with no epoch).
///
/// # Errors
///
/// [`RunRefusal::CensusUnreadable`] for a census that exists and holds no
/// such slot, or cannot be read.
fn census_header(
    store: &Path,
    layout: GdflLayout,
    kind: ImportKind,
) -> Result<Option<ManifestHeader>, RunRefusal> {
    let unreadable = |why: String| RunRefusal::CensusUnreadable { why };
    let path = match layout {
        GdflLayout::V3 => manifest::manifest_path(store, Vendor::Gdfl),
        GdflLayout::V4 => {
            let segment = segment_of(kind);
            match census_store::current_epoch(store, Vendor::Gdfl, segment).map_err(unreadable)? {
                Some(epoch) => manifest::epoch_path(store, Vendor::Gdfl, segment, epoch),
                None => return Ok(None),
            }
        }
    };
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(unreadable(format!("{}: {why}", path.display()))),
    };
    let len = file
        .metadata()
        .map_err(|why| unreadable(format!("{}: {why}", path.display())))?
        .len();
    let slot = |at: u64| {
        let mut image = [0_u8; manifest::IMAGE_LEN];
        std::os::unix::fs::FileExt::read_exact_at(&file, &mut image, at)
            .ok()
            .and_then(|()| ManifestHeader::decode(&image).ok())
            .filter(|header| header.generation % manifest::SLOT_COUNT == at / SLOT_STRIDE)
            .filter(|header| {
                len.checked_sub(manifest::HEADER_LEN)
                    .is_some_and(|room| header.n_valid <= room / u64::from(header.entry_stride.max(1)))
            })
    };
    match (slot(0), slot(SLOT_STRIDE)) {
        (Some(a), Some(b)) => Ok(Some(if a.generation >= b.generation { a } else { b })),
        (Some(one), None) | (None, Some(one)) => Ok(Some(one)),
        (None, None) => Err(unreadable(format!(
            "{}: neither header slot decodes to a generation its file's {len} bytes hold",
            path.display()
        ))),
    }
}

/// The census room for `kind`'s first planned day (item 5): `free_now` is
/// the header's headroom (capacity − `n_valid`; v3 `MAX_ENTRIES` −
/// `n_valid`); `free_after` is `epoch_capacity(segment, n_keys)` − `n_keys`
/// by the writer's own function on v4, and `free_now` on v3; refused when the
/// first day wants more than both. When only the compaction gives the room,
/// the new epoch's bytes (`HEADER_LEN` + `ENTRY_STRIDE` × its capacity, the
/// writer's constants) are put to R4-07's `floor_check` at the run's own
/// floor, the one the run checks before each day (D-7002). O(1): two slot
/// reads, arithmetic; the projected stop is O(days in range).
fn room(
    store: &Path,
    layout: GdflLayout,
    kind: ImportKind,
    looked: &Looked,
    request: &Request,
    host: &dyn Host,
) -> Result<Room, RunRefusal> {
    let Some((first_day, wanted)) = looked.first else {
        return Ok(Room::Unmeasured(
            "no day of the range the source holds".to_owned(),
        ));
    };
    let Some(need) = wanted else {
        return Ok(Room::Unmeasured(format!(
            "the listing of {first_day}, the first planned day, refused"
        )));
    };
    let segment = segment_of(kind);
    let header = census_header(store, layout, kind)?;
    let (n_valid, n_keys, limit) = match (header, layout) {
        (Some(header), _) => (header.n_valid, header.n_keys, header.limit()),
        (None, GdflLayout::V3) => (0, 0, manifest::MAX_ENTRIES),
        (None, GdflLayout::V4) => (0, 0, epoch_capacity(segment, 0).unwrap_or(0)),
    };
    let free_now = limit.saturating_sub(n_valid);
    let (free_after, compacted) = match layout {
        GdflLayout::V3 => (free_now, None),
        GdflLayout::V4 => epoch_capacity(segment, n_keys).map_or((0, None), |capacity| {
            (capacity.saturating_sub(n_keys), Some(capacity))
        }),
    };
    let word = locks::layout_word(layout);
    if need > free_now.max(free_after) {
        return Err(RunRefusal::CensusRoomShort {
            layout: word,
            free_now,
            free_after,
            need_first_day: need,
        });
    }
    let compaction = need > free_now;
    if compaction && let Some(capacity) = compacted {
        let need_bytes = manifest::ENTRY_STRIDE
            .saturating_mul(capacity)
            .saturating_add(manifest::HEADER_LEN);
        if let Ok(space) = host.space(store)
            && let Err(refused) = floor_check(space, need_bytes, Floor::Bytes(request.free_floor))
        {
            let floor = match refused {
                FloorRefusal::DiskFloor { floor_bytes, .. }
                | FloorRefusal::NeedOverflows { floor_bytes, .. } => floor_bytes,
                FloorRefusal::ProjectionUnknown { .. } | FloorRefusal::SpaceUnknown { .. } => {
                    request.free_floor
                }
            };
            return Err(RunRefusal::CensusCompactionNoDisk {
                need_bytes,
                free_bytes: space.free,
                floor,
            });
        }
    }
    Ok(Room::Measured {
        layout: word,
        free_now,
        free_after,
        need_first_day: need,
        compaction,
        projected_stop: projected_stop(
            first_day,
            (request.to, looked.span),
            free_now.max(free_after),
            need,
        ),
    })
}

/// The first planned day the room would not reach, if every planned day
/// wanted `need` files as the first does: an EXTRAPOLATION FROM THE FIRST
/// DAY (D-4802), `None` when the range fits or wants nothing. O(days in
/// range), once per run.
fn projected_stop(
    first: Day,
    (to, span): (Day, Option<(Day, Day)>),
    room: u64,
    need: u64,
) -> Option<Day> {
    let fit = room.checked_div(need)?;
    let end = span.map_or(to, |(_, last)| to.min(last));
    let mut planned = 0_u64;
    let mut day = first;
    while day <= end {
        if trading(day) {
            if planned == fit {
                return Some(day);
            }
            planned += 1;
        }
        day = day.succ().ok()?;
    }
    None
}

// ── The sentences ───────────────────────────────────────────────────────────

/// The words of this module's refusals after their code (the rest of
/// [`RunRefusal`]'s Display); any other refusal says its code alone.
///
/// # Errors
///
/// The formatter's.
pub fn say(refusal: &RunRefusal, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match refusal {
        RunRefusal::YearZipMissing { year, near_miss } if near_miss.is_empty() => write!(
            f,
            "the options root holds no {year:04}.zip, so no day of {year} can be read; \
             nothing was read or written"
        ),
        RunRefusal::YearZipMissing { year, near_miss } => write!(
            f,
            "the options root holds no {year:04}.zip, so no day of {year} can be read, but \
             holds {}: rename the year's zip to {year:04}.zip exactly; nothing was read or \
             written",
            near_miss.join(", ")
        ),
        RunRefusal::SourceUnreadable { path, mode } => write!(
            f,
            "{} (mode {mode}) cannot be read by this process; give it read access, then run \
             again; nothing was read or written",
            path.display()
        ),
        RunRefusal::YearZipEmpty { year } => write!(
            f,
            "{year:04}.zip holds no entry, so every day of {year} would read as one the source \
             lacks; replace it with the year's zip; nothing was read or written"
        ),
        RunRefusal::SourceBusy { root, holder } => write!(
            f,
            "{}: {holder}; no day is read under a writer; run again once it is done",
            root.display()
        ),
        RunRefusal::CensusRoomShort {
            layout,
            free_now,
            free_after,
            need_first_day,
        } => write!(
            f,
            "the {layout} census has room for {free_now} entries now and {free_after} after a \
             compaction, and the first planned day wants {need_first_day}: no day was read or \
             written (D-4802, D-4850)"
        ),
        RunRefusal::CensusCompactionNoDisk {
            need_bytes,
            free_bytes,
            floor,
        } => write!(
            f,
            "only a census compaction makes room, and its new epoch of {need_bytes} bytes does \
             not fit in {free_bytes} free above the floor of {floor}; free space first; \
             nothing was read or written"
        ),
        RunRefusal::LegacyRunMayBeLive { key } => write!(
            f,
            "no claim lock file has ever been written on this store, and the journal's key \
             {key} ends in begin: an import by a build before the locks may still be running \
             it. Make sure none is, then run again with --legacy-run-not-live (recorded in the \
             claim); nothing was read or written"
        ),
        RunRefusal::StoreReadOnly { root } => write!(
            f,
            "{} takes no write: the store does not take writes (permission or a read-only \
             volume), so no run starts: {NOT_WRITABLE_FIX}",
            root.display()
        ),
        RunRefusal::StoreNotLocal { root, fs } => write!(
            f,
            "{} is on a {fs} volume that is not local: a file lock there does not exclude \
             another machine, so no run starts; move the store to a local drive",
            root.display()
        ),
        RunRefusal::OpenFilesShort { have, need } => write!(
            f,
            "this process may hold {have} open files and the run's workers need {need} at once \
             (L-IA-35): raise the soft limit in the shell that runs it (`ulimit -n {need}`, or \
             `launchctl limit maxfiles {need} unlimited` on macOS, which a fresh Terminal \
             reads), or run fewer workers (BRUTEX_GDFL_WORKERS); nothing was read or written"
        ),
        other => f.write_str(other.code()),
    }
}

#[cfg(test)]
#[path = "gdfl_run_preflight_tests.rs"]
mod tests;
