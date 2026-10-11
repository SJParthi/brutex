//! The census on disk: which file a vendor's census is, how a batch reaches
//! it, how a partition is compacted into its next epoch, how a single file
//! becomes partitions, and how rows are recounted from the bar files
//! themselves. D-3316 (WP-10).
//!
//! [`crate::manifest`] performs no I/O, so every byte that reaches a disk on
//! the census's behalf goes through here or through [`crate::ingest`], whose
//! child module this is: the lock, the capped read, the positional append and
//! the rename are the parent's, reused rather than repeated.
//!
//! # The two shapes a vendor's census can have
//!
//! | Location | Files | Written at | Bound |
//! |---|---|---|---|
//! | [`Location::Single`] | `manifest/<vendor>.man` | version 3 | [`MAX_ENTRIES`], compiled |
//! | [`Location::Partitioned`] | `manifest/v4/<vendor>/<SEGMENT>/<epoch>.man` | version 4 | the header's own capacity |
//!
//! A vendor moves from the first to the second only through [`migrate`]
//! (`cli census-migrate`), never as a side effect of a write. That is the
//! ordering D-3316 item 7 requires (DB-G08): a reader built before version 4
//! reads the single file and nothing else, so a writer that switched on its own
//! would make every month written after the switch invisible to it. The
//! migration runs after the version-4-aware API is live (WP-30, WP-80 step 1).
//!
//! # Epochs (D-3316 items 1, 2 and 4)
//!
//! A partition is a directory of epochs, `000001.man`, `000002.man`, ... The
//! CURRENT epoch is the highest-numbered `<epoch>.man` present; a
//! `<epoch>.man.tmp` is incomplete and never read. Appends go to the current
//! epoch at its own offsets, exactly as to a single file. A compaction writes
//! the next epoch to its `.tmp`, flushes it, renames it into place and flushes
//! the directory. The epoch it replaced is never opened for writing again and
//! never deleted (`CLAUDE.md` §3 rule 8), so no byte of the census's history is
//! rewritten in place, and a crash at any step leaves a whole epoch current.
//!
//! # The up-front refusal (DB-G10)
//!
//! A batch is probed before a row is recorded: how many entries it needs, and
//! how many of its keys are new. A single file without room refuses there and
//! then, naming the file's own `n_valid`, its capacity, the free slots and the
//! batch — not the in-memory counter of whichever row happened to be first past
//! the ceiling, which is what `n_valid is 2097153` was. A partition without
//! room compacts once into its next epoch, and refuses, with the same numbers,
//! only when even one row per key leaves no room for the batch (D-3316 item 5).
//!
//! # What this does not do
//!
//! It does not read a vendor's archive. [`recount`] rebuilds census rows from
//! bar-file HEADERS — `n_valid`, the first and last stamp, two record reads for
//! the closes — so a census refused while its bars landed is caught up without
//! re-reading a single vendor zip (DB-G09, DB-G49).

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::io::{ErrorKind, Read as _, Write as _};
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use crate::manifest::{
    Append, Closes, ENTRY_STRIDE, EPOCH_TMP_SUFFIX, Entry, EntryKey, FIRST_EPOCH, HEADER_LEN, Held,
    LAST_EPOCH, MAX_ENTRIES, Manifest, ManifestError, ManifestHeader, SEGMENTS, epoch_name,
    epoch_path, manifest_path, parse_epoch_name, segment_dir, segment_lock_path, v4_root,
    vendor_dir,
};
use crate::session::Day;

use super::{CensusLock, Unbounded, append_locked, install_census, read_census};

/// The telemetry target every line here is filed under.
const TARGET: &str = "pull.census";

/// The name of the receipt a migration leaves inside `manifest/v4/<vendor>/`.
pub const MIGRATION_RECEIPT: &str = "MIGRATED";

/// The suffix of the directory a migration is staged in, beside the vendor
/// directories under `manifest/v4/`, before it is renamed into place. A reader
/// looks up `manifest/v4/<vendor>/` by the vendor's own name, so a staging
/// directory is never mistaken for a census.
pub const STAGING_SUFFIX: &str = ".staging";

/// Refusals a recount names one by one before it only counts them.
const NAMED: usize = 50;

/// Bytes of one header slot's spacing, for splitting a header region.
const SLOT_SPACING: usize = 16_384;

/// [`HEADER_LEN`] in the width a buffer is measured in.
const HEADER_REGION_LEN: usize = 32_768;

/// Microseconds in one civil day, for the IST bounds of a day.
const DAY_MICROS: i64 = 86_400_000_000;

const _: () = assert!(HEADER_LEN == 32_768);
const _: () = assert!(SLOT_SPACING * 2 == HEADER_REGION_LEN);

// ═══ Where a census is ══════════════════════════════════════════════════════

/// Where one vendor's census is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// Not migrated: the one file at `manifest/<vendor>.man`, versions 1 to 3,
    /// read and written exactly as before version 4 existed.
    Single(PathBuf),
    /// Migrated: `manifest/v4/<vendor>/`, one directory of epochs per segment.
    Partitioned(PathBuf),
}

/// Where `vendor`'s census is.
///
/// Partitioned exactly when `manifest/v4/<vendor>/` is a directory: [`migrate`]
/// creates it by renaming a complete staging directory into place, so it is
/// absent or whole (D-3316 item 7). A symlink, a file or an unmeasurable entry
/// at that name is refused rather than read as "not migrated": guessing would
/// write months into a census no reader of the other shape will look at.
///
/// # Errors
///
/// The path and the host's words, for anything at that name that is not a
/// directory, or that cannot be measured.
pub fn locate(root: &Path, vendor: Vendor) -> Result<Location, String> {
    let dir = vendor_dir(root, vendor);
    match fs::symlink_metadata(&dir) {
        Ok(found) if found.is_dir() => Ok(Location::Partitioned(dir)),
        Ok(_) => Err(format!(
            "{} is not a directory, and it is where {}'s version-4 census lives \
             once migrated; refused rather than guess which census is {}'s (D-3316)",
            dir.display(),
            vendor.as_str(),
            vendor.as_str()
        )),
        // NOTHING IS THERE, OR NOTHING CAN BE: no directory, a file where a
        // directory of the path belongs, or a name past the host's limit. In
        // each case the single file's own read and install fail on the same
        // cause a moment later and name it in the words an operator needs —
        // `a_census_that_cannot_be_measured_stops_the_run` asserts that wording
        // — so this does not pre-empt them with a worse one.
        Err(absent)
            if matches!(
                absent.kind(),
                ErrorKind::NotFound | ErrorKind::NotADirectory | ErrorKind::InvalidFilename
            ) =>
        {
            Ok(Location::Single(manifest_path(root, vendor)))
        }
        Err(why) => Err(format!(
            "{} could not be measured: {why}; refused rather than guess whether \
             {} was migrated (D-3316)",
            dir.display(),
            vendor.as_str()
        )),
    }
}

/// The current epoch of one `(vendor, segment)` partition: the highest-numbered
/// `<epoch>.man` in its directory, or [`None`] when it holds none. D-3316
/// item 2.
///
/// A `.man.tmp` is an epoch being written and is never an answer, and any other
/// name — a lock, a stray file — is not an epoch. O(entries of the directory),
/// which is one per compaction the partition ever had.
///
/// # Errors
///
/// The directory and the host's words, for a directory that exists and cannot
/// be listed.
pub fn current_epoch(root: &Path, vendor: Vendor, segment: Segment) -> Result<Option<u32>, String> {
    let dir = segment_dir(root, vendor, segment);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(absent)
            if matches!(
                absent.kind(),
                ErrorKind::NotFound | ErrorKind::NotADirectory
            ) =>
        {
            return Ok(None);
        }
        Err(why) => return Err(format!("{} could not be listed: {why}", dir.display())),
    };
    let mut newest: Option<u32> = None;
    for entry in entries {
        let entry = entry.map_err(|why| format!("{} could not be listed: {why}", dir.display()))?;
        if let Some(epoch) = entry.file_name().to_str().and_then(parse_epoch_name) {
            newest = Some(newest.map_or(epoch, |seen| seen.max(epoch)));
        }
    }
    Ok(newest)
}

/// One census a batch is written into: a vendor's single file, or one
/// `(vendor, segment)` partition, whose file is its current epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Whose census it is.
    pub vendor: Vendor,
    /// The partition's segment, or [`None`] for a single file.
    pub segment: Option<Segment>,
    /// The store root.
    root: PathBuf,
}

/// A census as one cycle loaded it, under the target's lock.
#[derive(Debug)]
pub struct Loaded {
    /// The census.
    pub census: Manifest,
    /// The epoch it was read from, for a partition; [`None`] for a single file
    /// and for a partition with no epoch yet.
    pub epoch: Option<u32>,
    /// The file the next append goes to: the single file, the current epoch, or
    /// the first epoch a partition with none will have.
    pub path: PathBuf,
    /// Whether that file exists.
    pub exists: bool,
    /// Entries the file held when it was read: what a compaction says it
    /// replaced, even after the census in memory has been compacted or
    /// appended to since.
    pub on_disk: u64,
}

impl Target {
    /// The single file of an unmigrated vendor.
    #[must_use]
    pub fn single(root: &Path, vendor: Vendor) -> Self {
        Self {
            vendor,
            segment: None,
            root: root.to_path_buf(),
        }
    }

    /// One segment of a migrated vendor.
    #[must_use]
    pub fn partition(root: &Path, vendor: Vendor, segment: Segment) -> Self {
        Self {
            vendor,
            segment: Some(segment),
            root: root.to_path_buf(),
        }
    }

    /// What a refusal names this census by: the single file's path, or the
    /// partition's directory of epochs.
    #[must_use]
    pub fn named(&self) -> PathBuf {
        match self.segment {
            None => manifest_path(&self.root, self.vendor),
            Some(segment) => segment_dir(&self.root, self.vendor, segment),
        }
    }

    /// The lock this census is written under: `manifest/<vendor>.man.lock` for a
    /// single file — the one every writer before version 4 takes — and
    /// `manifest/v4/<vendor>/<SEGMENT>.lock` for a partition (D-3316 item 1),
    /// which outlives every epoch the partition will have.
    #[must_use]
    pub fn lock_path(&self) -> PathBuf {
        match self.segment {
            None => manifest_path(&self.root, self.vendor).with_extension("man.lock"),
            Some(segment) => segment_lock_path(&self.root, self.vendor, segment),
        }
    }

    /// Takes this census's lock, for the whole read-modify-write.
    ///
    /// A single file's is taken exactly as before version 4, which makes
    /// `manifest/` on a first ingest; a partition's creates no directory,
    /// because only [`migrate`] may make `manifest/v4/<vendor>/`.
    ///
    /// # Errors
    ///
    /// The lock's refusal, naming it.
    pub(super) fn lock(&self) -> Result<CensusLock, String> {
        match self.segment {
            None => CensusLock::take(&manifest_path(&self.root, self.vendor)),
            Some(_) => CensusLock::take_at(&self.lock_path()),
        }
    }

    /// Refuses a single-file write that lost a race with [`migrate`].
    ///
    /// Asked AFTER the lock is held: [`migrate`] holds the single file's lock
    /// across its rename, so a writer that located the single file before the
    /// rename and took the lock after it finds the partition directory here and
    /// stops, instead of appending to a file no version-4 reader consults.
    ///
    /// # Errors
    ///
    /// The sentence naming the directory the census moved to.
    pub fn still_where_located(&self) -> Result<(), String> {
        if self.segment.is_some() {
            return Ok(());
        }
        match locate(&self.root, self.vendor)? {
            Location::Single(_) => Ok(()),
            Location::Partitioned(dir) => Err(format!(
                "{}'s census was migrated to {} while this run was starting; \
                 nothing was written to the single file. Run again (D-3316)",
                self.vendor.as_str(),
                dir.display()
            )),
        }
    }

    /// The census, read whole, and where its next write goes. Asked under the
    /// lock, so the current epoch cannot move while the cycle holds it.
    ///
    /// # Errors
    ///
    /// The file and the refusal, in one sentence.
    pub fn load(&self) -> Result<Loaded, String> {
        match self.segment {
            None => {
                let path = manifest_path(&self.root, self.vendor);
                let census = read_census(&path, self.vendor)?;
                // AN EPOCH AT THE SINGLE FILE'S NAME is one segment's census
                // where every segment's is read: refused, never appended to as
                // if it held the vendor's whole census (D-3316).
                if let Some(segment) = census.segment() {
                    return Err(format!(
                        "{} holds a version-4 epoch of the {} partition, not a \
                         single-file census; refused rather than read as all of {}'s \
                         census (D-3316)",
                        path.display(),
                        segment.as_str(),
                        self.vendor.as_str()
                    ));
                }
                let exists = fs::symlink_metadata(&path).is_ok();
                Ok(Loaded {
                    on_disk: census.entries(),
                    census,
                    epoch: None,
                    path,
                    exists,
                })
            }
            Some(segment) => match current_epoch(&self.root, self.vendor, segment)? {
                None => Ok(Loaded {
                    census: Manifest::partition_absent(self.vendor, segment),
                    epoch: None,
                    path: epoch_path(&self.root, self.vendor, segment, FIRST_EPOCH),
                    exists: false,
                    on_disk: 0,
                }),
                Some(epoch) => {
                    let path = epoch_path(&self.root, self.vendor, segment, epoch);
                    let census = read_epoch(&path, self.vendor, segment)?;
                    Ok(Loaded {
                        on_disk: census.entries(),
                        census,
                        epoch: Some(epoch),
                        path,
                        exists: true,
                    })
                }
            },
        }
    }

    /// The epoch after `loaded`'s: the first for a partition with none.
    fn next_epoch(loaded: &Loaded) -> u32 {
        loaded
            .epoch
            .map_or(FIRST_EPOCH, |epoch| epoch.saturating_add(1))
    }
}

/// The census one segment's rows go to.
///
/// # Errors
///
/// Whatever [`locate`] refuses.
pub fn target_for(root: &Path, vendor: Vendor, segment: Segment) -> Result<Target, String> {
    Ok(match locate(root, vendor)? {
        Location::Single(_) => Target::single(root, vendor),
        Location::Partitioned(_) => Target::partition(root, vendor, segment),
    })
}

/// The census a run's plan writes to: the single file of an unmigrated vendor
/// whatever the segment says, or the partition of `segment` — which must then
/// be a segment, since it names a directory.
///
/// # Errors
///
/// [`locate`]'s, or a segment the store does not define, for a migrated vendor.
pub fn target_for_plan(root: &Path, vendor: Vendor, segment: &str) -> Result<Target, String> {
    match locate(root, vendor)? {
        Location::Single(_) => Ok(Target::single(root, vendor)),
        Location::Partitioned(_) => {
            let segment = Segment::parse(segment).map_err(|why| {
                format!(
                    "segment {segment:?}: {why}; {}'s census is per segment, so a \
                     plan must name one the store defines (D-3316)",
                    vendor.as_str()
                )
            })?;
            Ok(Target::partition(root, vendor, segment))
        }
    }
}

/// One epoch file of a `(vendor, segment)` partition, read through one handle:
/// the header region first, then the rest, capped by the capacity the header
/// declares.
///
/// **The cap is the file's own** (DB-G08, D-3316 item 3). A reader compiled
/// with a ceiling refuses, by size, the first file a writer built with a larger
/// one grows past it; this one reads the declared capacity and is bound by that
/// and by the file's length. A file at this path whose header is not a
/// partition's is bounded by the single-file ceiling, read whole, and refused
/// by [`Manifest::open_partition`] by name.
///
/// **An epoch that is listed exists**, so an absent file is refused here, never
/// answered with a genesis: epochs are never deleted, and one that vanished
/// between the listing and the read is a fact somebody must look at.
///
/// # Errors
///
/// The path and the refusal: a handle that is not a regular file, a file larger
/// than its header allows, a read that failed, or whatever the census refuses.
pub fn read_epoch(path: &Path, vendor: Vendor, segment: Segment) -> Result<Manifest, String> {
    let named = |why: &dyn std::fmt::Display| format!("{}: {why}", path.display());
    let file = store::open_flags::open_read_no_follow(path).map_err(|why| {
        format!(
            "{} could not be opened: {why}. An epoch is never deleted, so one that \
             cannot be opened is refused rather than read as absent",
            path.display()
        )
    })?;
    let meta = file.metadata().map_err(|why| named(&why))?;
    if !meta.is_file() {
        return Err(named(&Unbounded::NotRegular));
    }
    let mut bytes: Vec<u8> = Vec::with_capacity(HEADER_REGION_LEN);
    let _ = (&file)
        .take(HEADER_LEN)
        .read_to_end(&mut bytes)
        .map_err(|why| named(&why))?;
    let capacity = declared_capacity(&bytes).unwrap_or(MAX_ENTRIES);
    let cap = HEADER_LEN.saturating_add(capacity.saturating_mul(ENTRY_STRIDE));
    if meta.len() > cap {
        return Err(named(&Unbounded::PastCap {
            cap,
            size: meta.len(),
        }));
    }
    let read_so_far = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let _ = (&file)
        .take(cap.saturating_add(1).saturating_sub(read_so_far))
        .read_to_end(&mut bytes)
        .map_err(|why| named(&why))?;
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if size > cap {
        return Err(named(&Unbounded::PastCap { cap, size }));
    }
    Manifest::open_partition(vendor, segment, &bytes).map_err(|why| named(&why))
}

/// The largest capacity any decodable version-4 slot in `region` declares.
///
/// The largest rather than the newest's, because the read cap must admit the
/// generation the loader falls back to as well as the one it tries first. A
/// slot that does not decode bounds nothing. `None` when no slot is a
/// partition's.
fn declared_capacity(region: &[u8]) -> Option<u64> {
    region
        .chunks(SLOT_SPACING)
        .take(2)
        .filter_map(|slot| ManifestHeader::decode(slot).ok())
        .filter(|header| header.segment.is_some())
        .map(|header| header.capacity)
        .max()
}

// ═══ One batch ══════════════════════════════════════════════════════════════

/// What a batch would do to a census, measured before a row is recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fit {
    /// The file's committed entry count.
    pub n_valid: u64,
    /// Its capacity: [`MAX_ENTRIES`] for a single file, the header's for a
    /// partition.
    pub capacity: u64,
    /// `capacity − n_valid`.
    pub free: u64,
    /// Distinct keys it holds.
    pub keys: u64,
    /// Rows offered.
    pub offered: u64,
    /// Entries the batch would append: every offered row that is not already
    /// recorded exactly.
    pub batch: u64,
    /// Of those, keys the census does not hold yet.
    pub new_keys: u64,
}

impl Fit {
    /// Whether the batch fits without a compaction.
    #[must_use]
    pub const fn fits(&self) -> bool {
        self.batch <= self.free
    }
}

/// [`Fit`] of `held` against `census`: one probe per offered row.
///
/// O(offered), with one set bounded by the offered count so a key offered
/// twice is counted as new once.
#[must_use]
pub fn probe(census: &Manifest, held: &[Held]) -> Fit {
    let mut batch = 0u64;
    let mut new_keys = 0u64;
    let mut fresh: HashSet<EntryKey> = HashSet::with_capacity(held.len());
    for one in held {
        match census.held(&one.entry.key) {
            Some(recorded) if recorded == *one => {}
            Some(_) => batch += 1,
            None => {
                batch += 1;
                if fresh.insert(one.entry.key) {
                    new_keys += 1;
                }
            }
        }
    }
    Fit {
        n_valid: census.entries(),
        capacity: census.capacity(),
        free: census.headroom(),
        keys: census.keys(),
        offered: u64::try_from(held.len()).unwrap_or(u64::MAX),
        batch,
        new_keys,
    }
}

/// The sentence a batch that does not fit is refused with (DB-G10, D-3316
/// item 5, D-3317 item 7's `reason=too_many_entries`).
///
/// Every number is the FILE's — its committed `n_valid`, its capacity, its
/// free slots — beside the batch's own size, and it says that nothing was
/// published, which is what all-or-nothing means and what the old line left a
/// reader to guess. The `ManifestError::TooManyEntries` wording is inside it,
/// so a caller matching that refusal's words finds them.
fn refusal(target: &Target, fit: &Fit, remedy: &str) -> String {
    let shape = target.segment.map_or_else(
        || "single file".to_owned(),
        |s| format!("{} partition", s.as_str()),
    );
    let error = ManifestError::TooManyEntries {
        n_valid: fit.n_valid,
        batch: fit.batch,
        limit: fit.capacity,
    };
    format!(
        "census {} ({} {shape}) refused this batch before recording any of it: \
         it holds n_valid {} of a capacity of {} ({} free); the batch needs {} \
         entries ({} new keys) for {} offered rows [{error}]. {remedy} Nothing \
         of the batch was published: it is all-or-nothing, so the free slots \
         ({}) were left unused rather than filled with part of it.",
        target.named().display(),
        target.vendor.as_str(),
        fit.n_valid,
        fit.capacity,
        fit.free,
        fit.batch,
        fit.new_keys,
        fit.offered,
        fit.free,
    )
}

/// How a cycle treats a row the census refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The first refused row refuses the batch and nothing of it is published.
    /// The import's day is this: its rows are one commit or none.
    AllOrNothing,
    /// A refused row is named and skipped, and the rest are published. A
    /// recount is this: one damaged month must not keep every other month
    /// uncounted.
    PerKey,
}

/// What one compaction did: the next epoch installed, the old one kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compaction {
    /// The epoch replaced, or [`None`] when the partition had none (a repair
    /// of an absent census cannot happen, so in practice always `Some`).
    pub from_epoch: Option<u32>,
    /// The epoch installed.
    pub to_epoch: u32,
    /// Entries in the epoch replaced.
    pub from_entries: u64,
    /// Entries in the epoch installed: one per key.
    pub to_entries: u64,
    /// The epoch installed.
    pub path: PathBuf,
    /// Why: `full` (the batch did not fit), `repair` (the census loaded
    /// degraded, and its recovered prefix is published as a new epoch rather
    /// than over the damaged one) or `recount` (a recount publishes its next
    /// epoch whole, [`cycle`]).
    pub why: &'static str,
}

/// What one census cycle — lock, read, record, publish — did to one census.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cycle {
    /// The file published to: the single file or the partition's epoch.
    pub path: PathBuf,
    /// The partition's segment, or [`None`] for a single file.
    pub segment: Option<Segment>,
    /// The partition's epoch after the cycle, or [`None`] for a single file.
    pub epoch: Option<u32>,
    /// Rows offered.
    pub offered: usize,
    /// Rows recorded: new or changed.
    pub recorded: usize,
    /// Rows already recorded exactly, which change nothing.
    pub exact: usize,
    /// Rows the census refused, by key, in [`Mode::PerKey`].
    pub refused: Vec<(EntryKey, String)>,
    /// The compaction, if this cycle compacted.
    pub compacted: Option<Compaction>,
    /// What the load stepped over, if it loaded degraded.
    pub degraded: Option<String>,
    /// The generation after the cycle.
    pub generation: u64,
    /// Committed entries after the cycle.
    pub entries: u64,
    /// Distinct keys after the cycle.
    pub keys: u64,
    /// Rows after the cycle.
    pub rows: u64,
    /// Capacity after the cycle.
    pub capacity: u64,
}

/// One census cycle over one census: take the lock, read, probe, compact if a
/// partition needs it, record, publish. D-3316.
///
/// # How a partition publishes
///
/// [`Mode::AllOrNothing`] — an import day — appends to the current epoch, after
/// installing the compacted next epoch first when the batch does not fit or the
/// epoch loaded degraded. [`Mode::PerKey`] — a recount — publishes its NEXT
/// EPOCH WHOLE whenever anything changed: the newest row per key, by rename,
/// the epoch it replaced kept as it was. A recount offers every month of its
/// span at once, hundreds of thousands of rows for the stopped options import,
/// and an append costs two flushes per entry (`docs/02-store-format.md` §11.6),
/// so whole is one write and two flushes where appending would be two flushes
/// per row. The bytes are the ones a compaction writes, so a reader cannot tell
/// the two apart and needs no rule of its own.
///
/// # Errors
///
/// The lock's, the read's, the up-front refusal ([`refusal`]), the first
/// refused row in [`Mode::AllOrNothing`], or the install's — each naming the
/// census. On any of them nothing of the batch is published; a compaction
/// installed before a row was refused stays installed, and holds exactly the
/// totals the replaced epoch held, so it changes no answer.
pub fn cycle(target: &Target, held: &[Held], mode: Mode) -> Result<Cycle, String> {
    let lock = target.lock()?;
    target.still_where_located()?;
    let mut loaded = target.load()?;
    let degraded = loaded.census.degraded_reason().map(|why| why.to_string());
    let fit = probe(&loaded.census, held);
    let whole = target.segment.is_some() && mode == Mode::PerKey;
    let room = make_room(&lock, target, &mut loaded, &fit, degraded.is_some(), whole)?;
    let mut compacted = room.installed;

    let mut appends: Vec<Append> = Vec::with_capacity(held.len());
    let mut recorded = 0usize;
    let mut exact = 0usize;
    let mut refused: Vec<(EntryKey, String)> = Vec::new();
    for one in held {
        match super::count(&mut loaded.census, *one) {
            Ok(Some(append)) => {
                recorded += 1;
                appends.push(append);
            }
            Ok(None) => exact += 1,
            Err(why) => match mode {
                Mode::AllOrNothing => {
                    return Err(format!(
                        "{}: {why}; nothing of this batch of {} row(s) was published{}",
                        loaded.path.display(),
                        held.len(),
                        compacted.as_ref().map_or_else(String::new, |done| format!(
                            " (epoch {} was installed by a compaction first, and holds \
                             the totals the epoch it replaced held)",
                            epoch_name(done.to_epoch)
                        ))
                    ));
                }
                Mode::PerKey => refused.push((one.entry.key, why)),
            },
        }
    }

    if whole {
        // THE NEXT EPOCH, WHOLE, or nothing at all when nothing changed — so a
        // second recount of the same store writes no byte (`CLAUDE.md` §3
        // rule 5).
        if !appends.is_empty() || room.carried.is_some() {
            let why = room.carried.unwrap_or("recount");
            compacted = Some(install_whole(&lock, target, &mut loaded, why)?);
        }
    } else {
        // A single file keeps its own install rules — repair, upgrade and first
        // write publish the whole image, anything else appends. A partition
        // never renames over an epoch: a repair already went to a new epoch.
        let repairing = target.segment.is_none() && degraded.is_some();
        publish(&lock, target, &loaded, &appends, repairing)?;
    }

    let header = loaded.census.header();
    Ok(Cycle {
        path: loaded.path.clone(),
        segment: target.segment,
        epoch: target.segment.and(
            loaded
                .epoch
                .or_else(|| (!appends.is_empty()).then_some(FIRST_EPOCH)),
        ),
        offered: held.len(),
        recorded,
        exact,
        refused,
        compacted,
        degraded,
        generation: header.generation,
        entries: header.n_valid,
        keys: header.n_keys,
        rows: header.total_rows,
        capacity: loaded.census.capacity(),
    })
}

/// What [`make_room`] did before a row was recorded.
struct Room {
    /// The compacted epoch it installed, for an import day.
    installed: Option<Compaction>,
    /// Why it compacted in memory without installing, for a recount, whose
    /// whole publish installs the compaction with the batch in it.
    carried: Option<&'static str>,
}

/// Refuses a batch the census cannot take, before a row is recorded, and
/// compacts a partition once when the batch does not fit or the epoch loaded
/// degraded (D-3316 item 5). A single file without room refuses: it is never
/// compacted by a write.
///
/// # Errors
///
/// The up-front refusal ([`refusal`]), the compaction's, or the install's.
fn make_room(
    lock: &CensusLock,
    target: &Target,
    loaded: &mut Loaded,
    fit: &Fit,
    degraded: bool,
    whole: bool,
) -> Result<Room, String> {
    let nothing = Room {
        installed: None,
        carried: None,
    };
    if target.segment.is_none() {
        if fit.fits() {
            return Ok(nothing);
        }
        return Err(refusal(
            target,
            fit,
            &format!(
                "A single-file census is never compacted by a write; `cli \
                 census-migrate {}` moves it to per-segment epochs that carry \
                 their own capacity and compact (D-3316).",
                target.vendor.as_str()
            ),
        ));
    }
    // ONE COMPACTION, AND ONLY WHEN IT IS NEEDED: the batch does not fit, or
    // the census loaded degraded and its recovered prefix must be published as
    // a new epoch rather than over the damaged one.
    let why = if !fit.fits() {
        "full"
    } else if degraded {
        "repair"
    } else {
        return Ok(nothing);
    };
    let next = loaded
        .census
        .compacted()
        .map_err(|refused| format!("{}: {refused}", target.named().display()))?;
    if fit.batch > next.headroom() {
        return Err(refusal(
            target,
            fit,
            &format!(
                "Even compacted to one row per key ({} keys, capacity {}, {} \
                 free) it cannot take the batch, so no epoch was installed.",
                next.keys(),
                next.capacity(),
                next.headroom()
            ),
        ));
    }
    if whole {
        loaded.census = next;
        return Ok(Room {
            installed: None,
            carried: Some(why),
        });
    }
    let done = install_next_epoch(lock, target, loaded, &next, why)?;
    *loaded = Loaded {
        census: next,
        epoch: Some(done.to_epoch),
        path: done.path.clone(),
        exists: true,
        on_disk: done.to_entries,
    };
    Ok(Room {
        installed: Some(done),
        carried: None,
    })
}

/// `loaded`'s census, compacted, installed as the next epoch; `loaded` then
/// names that epoch.
///
/// # Errors
///
/// The compaction's or the install's.
fn install_whole(
    lock: &CensusLock,
    target: &Target,
    loaded: &mut Loaded,
    why: &'static str,
) -> Result<Compaction, String> {
    let next = loaded
        .census
        .compacted()
        .map_err(|refused| format!("{}: {refused}", target.named().display()))?;
    let done = install_next_epoch(lock, target, loaded, &next, why)?;
    *loaded = Loaded {
        census: next,
        epoch: Some(done.to_epoch),
        path: done.path.clone(),
        exists: true,
        on_disk: done.to_entries,
    };
    Ok(done)
}

/// Installs the next epoch of `vendor`'s `segment` partition WITHOUT the keys
/// `withdrawn` names, the epoch it replaces kept byte for byte (D-3316 item
/// 4): the census half of the re-key (`crate::gdfl_rekey`, D-3331; WP-12c).
/// A key withdrawn here is a stored series the current decoder files under
/// another key or refuses, so the census stops listing it while its bar files
/// and the replaced epoch keep it as history.
///
/// Nothing is written when the census holds none of the keys, so a second
/// run changes no byte (`CLAUDE.md` §3 rule 5). The partition's lock is held
/// for the whole read, compaction and install, the order every other cycle
/// keeps.
///
/// # Errors
///
/// A census that is a single file (it is never compacted by a write:
/// `cli census-migrate` first), and the lock's, the read's, the compaction's
/// or the install's refusal, each naming the census.
pub fn withdraw(
    root: &Path,
    vendor: Vendor,
    segment: Segment,
    withdrawn: &dyn Fn(&EntryKey) -> bool,
) -> Result<Option<Compaction>, String> {
    let target = target_for(root, vendor, segment)?;
    if target.segment.is_none() {
        return Err(format!(
            "{}'s census is a single file ({}); a key is withdrawn by installing the \
             partition's next epoch, which only a migrated census has. Run `cli \
             census-migrate {}` first (D-3316); nothing was written",
            vendor.as_str(),
            target.named().display(),
            vendor.as_str()
        ));
    }
    let lock = target.lock()?;
    let loaded = target.load()?;
    if !loaded.census.held_keys().any(withdrawn) {
        return Ok(None);
    }
    let next = loaded
        .census
        .compacted_without(withdrawn)
        .map_err(|refused| format!("{}: {refused}", target.named().display()))?;
    install_next_epoch(&lock, &target, &loaded, &next, "rekey").map(Some)
}

/// Publishes what a cycle recorded.
///
/// A single file goes through the parent's [`install_census`]. A partition
/// appends to its current epoch at the epoch's own offsets; a partition with no
/// epoch yet installs its first epoch whole, by rename from a `.tmp`; and
/// nothing at all is written when nothing changed, so a re-run leaves every
/// byte as it was (`CLAUDE.md` §3 rule 5).
///
/// # Errors
///
/// The install's, naming the file.
fn publish(
    lock: &CensusLock,
    target: &Target,
    loaded: &Loaded,
    appends: &[Append],
    repairing: bool,
) -> Result<(), String> {
    match target.segment {
        None => install_census(lock, &loaded.path, &loaded.census, appends, repairing),
        Some(segment) => {
            if appends.is_empty() {
                Ok(())
            } else if loaded.exists {
                append_locked(lock, &loaded.path, appends)
            } else {
                install_epoch(
                    lock,
                    &target.root,
                    target.vendor,
                    segment,
                    Target::next_epoch(loaded),
                    &loaded.census.image(),
                )
                .map(|_| ())
            }
        }
    }
}

/// Installs `next` — a compacted census — as the epoch after `loaded`'s, and
/// says so in the log.
///
/// # Errors
///
/// [`install_epoch`]'s.
fn install_next_epoch(
    lock: &CensusLock,
    target: &Target,
    loaded: &Loaded,
    next: &Manifest,
    why: &'static str,
) -> Result<Compaction, String> {
    let Some(segment) = target.segment else {
        return Err(format!(
            "{}: a single-file census is never compacted by a write (D-3316)",
            target.named().display()
        ));
    };
    let to_epoch = Target::next_epoch(loaded);
    let path = install_epoch(
        lock,
        &target.root,
        target.vendor,
        segment,
        to_epoch,
        &next.image(),
    )?;
    let done = Compaction {
        from_epoch: loaded.epoch,
        to_epoch,
        from_entries: loaded.on_disk,
        to_entries: next.entries(),
        path,
        why,
    };
    note_compacted(target, &done, next);
    Ok(done)
}

/// Writes one whole epoch: `<epoch>.man.tmp`, flushed, renamed to
/// `<epoch>.man`, and the directory flushed. D-3316 items 2 and 4.
///
/// **An epoch is written once.** One that already exists is refused, never
/// replaced: the lock serialises every writer of the partition, so an epoch
/// already at that name means the listing and the disk disagree, and renaming
/// over it would rewrite a generation in place (`CLAUDE.md` §3 rule 8). A
/// `.tmp` left by a crash is never read and is overwritten here.
///
/// # Errors
///
/// The path and the host's words; an epoch number past [`LAST_EPOCH`]; an
/// epoch already present.
fn install_epoch(
    _lock: &CensusLock,
    root: &Path,
    vendor: Vendor,
    segment: Segment,
    epoch: u32,
    image: &[u8],
) -> Result<PathBuf, String> {
    let dir = segment_dir(root, vendor, segment);
    // A range pattern: the epochs six digits can name, a compile-time bound.
    let (FIRST_EPOCH..=LAST_EPOCH) = epoch else {
        return Err(format!(
            "{}: epoch {epoch} is outside {FIRST_EPOCH}..={LAST_EPOCH}, the epochs six \
             digits can name; nothing was written",
            dir.display()
        ));
    };
    let path = epoch_path(root, vendor, segment, epoch);
    if fs::symlink_metadata(&path).is_ok() {
        return Err(format!(
            "{} already exists: an epoch is written once and never replaced, so \
             nothing was written (D-3316)",
            path.display()
        ));
    }
    let tmp = dir.join(format!(
        "{}{EPOCH_TMP_SUFFIX}",
        epoch_name(epoch).trim_end_matches(crate::manifest::MANIFEST_EXTENSION)
    ));
    write_whole(&dir, &tmp, &path, image).map_err(|why| {
        format!(
            "{} could not be installed through {}: {why}",
            path.display(),
            tmp.display()
        )
    })?;
    Ok(path)
}

/// The five calls an epoch install is, apart from the sentence it fails with.
fn write_whole(dir: &Path, tmp: &Path, path: &Path, image: &[u8]) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let mut file = fs::File::create(tmp)?;
    file.write_all(image)?;
    file.sync_all()?;
    drop(file);
    fs::rename(tmp, path)?;
    fs::File::open(dir)?.sync_all()
}

// ═══ One batch, two flushes (WP-16, DB-N25) ═══════════════════════════════

/// Which of a batch's two flushes [`write_batch_with`] is about to issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BatchBarrier {
    /// After every entry of the batch is written, before any header slot.
    Entries,
    /// After the header slots, before the batch is answered.
    Slots,
}

/// Appends a census batch with TWO flushes, however many entries it holds
/// (WP-16, DB-N25): every entry, one barrier, then the newest commit of each
/// header slot the batch wrote — the batch's LAST commit first — and one more
/// barrier.
///
/// # Why, and why the bytes are the bytes one entry at a time leaves
///
/// One flush per entry and one per slot cost an import day of 5,641 option
/// files 11,282 flushes, 12.3 s of its 94.5 s on the external drive
/// (measured on 2020-09-15 into a scratch store, every flush an
/// `F_FULLFSYNC`). Writing the entries one by one, each followed by its own
/// commit, overwrites each header slot again and again; the file ends with
/// every entry at its offset and, in each slot, the LAST commit the batch
/// gave that slot. That is exactly what this writes, so the bytes after the
/// batch are the same bytes (`census_store_tests::
/// a_batch_lands_the_bytes_one_entry_at_a_time_leaves`).
///
/// # Crash safety
///
/// The slot rule is unchanged: no slot names an entry that is not durable,
/// because every entry is flushed before the first slot is written (each
/// commit's [`crate::manifest::Commit::durable_through`] is checked against
/// the batch's end first, and a batch that would break it is refused before
/// a byte is written). The batch's last commit is written first, so a crash
/// between the two slot writes leaves the newest generation naming the whole
/// batch and the other slot an older generation that names a prefix of it,
/// which [`crate::manifest::ManifestHeader::read_region`] reads as it reads
/// any two slots. A crash before the first slot leaves the census as it was.
/// Where one entry at a time could leave a census holding part of a day, this
/// leaves all of it or none of it.
///
/// `landed` sees the file once its last slot is durable, before its
/// modification time is moved past the writes (census-1, D-2766).
///
/// # Errors
///
/// The host's refusal, or a commit whose `durable_through` lies past every
/// entry of the batch.
pub(super) fn write_batch(
    path: &Path,
    appends: &[Append],
    landed: impl FnMut(&fs::File),
) -> std::io::Result<()> {
    write_batch_with(path, appends, landed, &mut |file, _| file.sync_all())
}

/// [`write_batch`], with each of its two flushes issued through `barrier`:
/// the seam a test counts them by, or fails one at.
pub(super) fn write_batch_with(
    path: &Path,
    appends: &[Append],
    mut landed: impl FnMut(&fs::File),
    barrier: &mut dyn FnMut(&fs::File, BatchBarrier) -> std::io::Result<()>,
) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt as _;
    let Some(last) = appends.last() else {
        return Ok(());
    };
    // ONE PASS before a byte is written: the batch's end, and every commit
    // checked against it, so no slot can name an entry the one barrier below
    // does not cover.
    let mut end = 0_u64;
    for append in appends {
        end = end.max(append.offset.saturating_add(ENTRY_STRIDE));
    }
    for append in appends {
        if append.commit.durable_through > end {
            return Err(std::io::Error::other(format!(
                "entry {} would be published through byte {} and the batch ends at {end}; \
                 nothing was written",
                append.ordinal, append.commit.durable_through
            )));
        }
    }
    let file = fs::OpenOptions::new().read(true).write(true).open(path)?;
    for append in appends {
        file.write_all_at(&append.bytes, append.offset)?;
    }
    // THE BARRIER: every entry is durable before any slot may count it.
    barrier(&file, BatchBarrier::Entries)?;
    // THE NEWEST COMMIT OF EACH SLOT, the batch's last first. Two slots
    // (`manifest::SLOT_COUNT`), so at most two writes: the last commit, then
    // the newest commit the batch gave the other slot, found walking back
    // from the end. O(batch), and nothing is allocated.
    file.write_all_at(&last.commit.bytes, last.commit.offset)?;
    for append in appends.iter().rev() {
        if append.commit.offset != last.commit.offset {
            file.write_all_at(&append.commit.bytes, append.commit.offset)?;
            break;
        }
    }
    barrier(&file, BatchBarrier::Slots)?;
    landed(&file);
    let written = file.metadata()?.modified()?;
    let past = written
        .checked_add(std::time::Duration::from_nanos(1))
        .ok_or_else(|| std::io::Error::other("the census's modification time cannot advance"))?;
    file.set_modified(std::time::SystemTime::now().max(past))
}

/// Records a batch of census rows for `vendor` — the door `ingest::record_held`
/// and the GDFL import use. All-or-nothing PER CENSUS.
///
/// A single-file census is one cycle. A partitioned one is one cycle per
/// segment the batch touches, in [`SEGMENTS`] order, each under its own lock;
/// a partition that refuses does not undo one already published, and the
/// refusal says which were.
///
/// # Errors
///
/// Every refused census's sentence, joined, naming the ones that did publish.
pub fn record(root: &Path, vendor: Vendor, held: &[Held]) -> Result<Vec<Cycle>, String> {
    let groups = grouped(root, vendor, held)?;
    let mut done = Vec::with_capacity(groups.len());
    let mut refused: Vec<String> = Vec::new();
    for (target, rows) in &groups {
        match cycle(target, rows, Mode::AllOrNothing) {
            Ok(cycle) => done.push(cycle),
            Err(why) => {
                super::note_census_unpublished(&target.named(), rows.len(), &why);
                refused.push(why);
            }
        }
    }
    if refused.is_empty() {
        return Ok(done);
    }
    let published: Vec<String> = done
        .iter()
        .map(|cycle| cycle.path.display().to_string())
        .collect();
    Err(format!(
        "{}{}",
        refused.join("; "),
        if published.is_empty() {
            String::new()
        } else {
            format!(" (published: {})", published.join(", "))
        }
    ))
}

/// `held` split into the censuses its rows go to: the whole batch for a single
/// file, one group per segment present for a partitioned vendor.
fn grouped(root: &Path, vendor: Vendor, held: &[Held]) -> Result<Vec<(Target, Vec<Held>)>, String> {
    Ok(match locate(root, vendor)? {
        Location::Single(_) => vec![(Target::single(root, vendor), held.to_vec())],
        Location::Partitioned(_) => SEGMENTS
            .into_iter()
            .filter_map(|segment| {
                let rows: Vec<Held> = held
                    .iter()
                    .filter(|one| one.entry.key.segment == segment)
                    .copied()
                    .collect();
                (!rows.is_empty()).then(|| (Target::partition(root, vendor, segment), rows))
            })
            .collect(),
    })
}

// ═══ The run-level door: `ingest::from_members` ═════════════════════════════

/// [`super::count`], with a partition at its capacity compacted in memory and
/// the row retried. The run-level door: `ingest::from_members` records rows as
/// members land, so the batch is not known up front. D-3316 item 5.
///
/// Sets `compacted`, which tells [`install_run`] that the in-memory census no
/// longer matches the current epoch's offsets and goes to the next epoch whole.
///
/// # Errors
///
/// Whatever [`Manifest::record_held`] refuses, or the compaction's refusal —
/// including a compacted partition that still has no room for the row.
pub(super) fn count_growing(
    census: &mut Manifest,
    held: Held,
    compacted: &mut bool,
) -> Result<Option<Append>, String> {
    if census.held(&held.entry.key) == Some(held) {
        return Ok(None);
    }
    match census.record_held(held) {
        Ok(append) => Ok(Some(append)),
        Err(ManifestError::TooManyEntries { .. }) if census.segment().is_some() => {
            *census = census.compacted().map_err(|why| why.to_string())?;
            *compacted = true;
            census
                .record_held(held)
                .map(Some)
                .map_err(|why| why.to_string())
        }
        Err(why) => Err(why.to_string()),
    }
}

/// The run-level install: a partition compacted mid-run, or loaded degraded,
/// goes to its next epoch whole — folded once more so it holds one row per key
/// — and anything else is [`publish`]ed as a cycle publishes it.
///
/// # Errors
///
/// The install's, naming the file.
pub(super) fn install_run(
    lock: &CensusLock,
    target: &Target,
    loaded: &Loaded,
    appends: &[Append],
    repairing: bool,
    compacted: bool,
) -> Result<(), String> {
    if target.segment.is_some() && (compacted || repairing) {
        let whole = loaded
            .census
            .compacted()
            .map_err(|why| format!("{}: {why}", target.named().display()))?;
        let why = if compacted { "full" } else { "repair" };
        let _done = install_next_epoch(lock, target, loaded, &whole, why)?;
        return Ok(());
    }
    publish(lock, target, loaded, appends, repairing)
}

/// One line per compaction, at info: it installs a whole new epoch of the
/// store's counter, and an operator must be able to find which run did it.
fn note_compacted(target: &Target, done: &Compaction, census: &Manifest) {
    let path = done.path.display().to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(TARGET, "census compacted")
            .with("census", telemetry::Value::Str(&path))
            .with("vendor", telemetry::Value::Str(target.vendor.as_str()))
            .with(
                "segment",
                telemetry::Value::Str(target.segment.map_or("all", Segment::as_str)),
            )
            .with("why", telemetry::Value::Str(done.why))
            .with(
                "from_epoch",
                done.from_epoch.map_or(telemetry::Value::Int(-1), |epoch| {
                    telemetry::Value::Uint(u64::from(epoch))
                }),
            )
            .with("to_epoch", telemetry::Value::Uint(u64::from(done.to_epoch)))
            .with("from_entries", telemetry::Value::Uint(done.from_entries))
            .with("to_entries", telemetry::Value::Uint(done.to_entries))
            .with("keys", telemetry::Value::Uint(census.keys()))
            .with("rows", telemetry::Value::Uint(census.total_rows()))
            .with("capacity", telemetry::Value::Uint(census.capacity()))
            .with(
                "generation",
                telemetry::Value::Uint(census.header().generation),
            ),
    );
}

// ═══ Reading a vendor's whole census ════════════════════════════════════════

/// One census file of a vendor, as a reader finds it.
#[derive(Debug)]
pub struct Read {
    /// The partition's segment, or [`None`] for the single file.
    pub segment: Option<Segment>,
    /// The epoch read, for a partition.
    pub epoch: Option<u32>,
    /// The file read.
    pub path: PathBuf,
    /// The census it holds.
    pub census: Manifest,
}

/// Every census file `vendor` has, each loaded — the read a census reader
/// needs (WP-30, D-3316 item 7): only the version-4 partitions once
/// `manifest/v4/<vendor>/` exists, the single file otherwise.
///
/// A partitioned vendor answers its current epoch of each segment that has
/// one, in [`SEGMENTS`] order; a single-file vendor answers its one file, or
/// nothing when there is none. Takes no lock: an epoch other than the current
/// one is never written, and the current one is read exactly as a single file
/// is, through its two header slots.
///
/// # Errors
///
/// [`locate`]'s, or the first file that will not load, naming it.
pub fn read_vendor(root: &Path, vendor: Vendor) -> Result<Vec<Read>, String> {
    match locate(root, vendor)? {
        Location::Single(path) => {
            if fs::symlink_metadata(&path).is_err() {
                return Ok(Vec::new());
            }
            let census = read_census(&path, vendor)?;
            Ok(vec![Read {
                segment: None,
                epoch: None,
                path,
                census,
            }])
        }
        Location::Partitioned(_) => {
            let mut out = Vec::with_capacity(SEGMENTS.len());
            for segment in SEGMENTS {
                if let Some(epoch) = current_epoch(root, vendor, segment)? {
                    let path = epoch_path(root, vendor, segment, epoch);
                    let census = read_epoch(&path, vendor, segment)?;
                    out.push(Read {
                        segment: Some(segment),
                        epoch: Some(epoch),
                        path,
                        census,
                    });
                }
            }
            Ok(out)
        }
    }
}

// ═══ Migration: a single file becomes partitions (D-3316 item 6) ════════════

/// One partition a migration wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// Its segment.
    pub segment: Segment,
    /// Where its first epoch now lives.
    pub path: PathBuf,
    /// Its keys — and entries, because a migrated partition is compacted.
    pub keys: u64,
    /// Its rows.
    pub rows: u64,
    /// Its capacity.
    pub capacity: u64,
    /// Its bytes.
    pub bytes: u64,
}

/// What a migration did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    /// The single file it read, which it left exactly as it was.
    pub legacy: PathBuf,
    /// Whether that file existed.
    pub legacy_present: bool,
    /// Its version, generation, entries, keys and rows.
    pub legacy_counters: (u16, u64, u64, u64, u64),
    /// Its length in bytes.
    pub legacy_bytes: u64,
    /// The vendor directory now in place: `manifest/v4/<vendor>/`.
    pub dir: PathBuf,
    /// Every partition written, one per segment.
    pub partitions: Vec<Written>,
}

/// Moves `vendor`'s single-file census to version-4 partitions, epoch `000001`
/// of each segment. D-3316 item 6.
///
/// # What it does, in order
///
/// 1. Refuses if `manifest/v4/<vendor>/` exists (already migrated) or a
///    staging directory from an interrupted migration is there.
/// 2. Takes the single file's lock — the one every pre-version-4 writer takes —
///    and holds it to the end, so no writer can add to the file mid-migration.
/// 3. Reads the single file, and refuses a census that loaded degraded:
///    migrating a recovered prefix would carry the loss into the new files with
///    nothing to say so.
/// 4. Splits it ([`Manifest::partitions`]): the newest row per key, per
///    segment, every segment — an empty one is an empty epoch at its floor.
/// 5. Checks the split adds up — keys and rows equal the single file's.
/// 6. Writes each partition's `000001.man` and the receipt into
///    `manifest/v4/<vendor>.staging/`, flushes them, reads every epoch back
///    through [`read_epoch`] and checks its counters, then renames the staging
///    directory to `manifest/v4/<vendor>/` and flushes `manifest/v4/`.
///
/// **The single file is never touched.** It stays where it was, byte for byte,
/// as the history it is; a reader built before version 4 keeps reading it,
/// which is why the version-4-aware API must be live first (WP-80 step 1).
///
/// # Errors
///
/// Each refusal above, naming the path; nothing is renamed into place unless
/// every partition round-tripped.
pub fn migrate(root: &Path, vendor: Vendor) -> Result<Migration, String> {
    let dir = vendor_dir(root, vendor);
    let legacy = manifest_path(root, vendor);
    let staging = staging_dir(root, vendor);
    if fs::symlink_metadata(&dir).is_ok() {
        return Err(format!(
            "{} exists: {}'s census is already version 4. Nothing was written",
            dir.display(),
            vendor.as_str()
        ));
    }
    if fs::symlink_metadata(&staging).is_ok() {
        return Err(format!(
            "{} exists: an earlier migration of {} stopped before it was renamed \
             into place. It is not the census and nothing reads it; look at it, \
             remove it, and run again. Nothing was written",
            staging.display(),
            vendor.as_str()
        ));
    }
    let lock = Target::single(root, vendor).lock()?;
    let legacy_meta = fs::symlink_metadata(&legacy).ok();
    let census = read_census(&legacy, vendor)?;
    if let Some(why) = census.degraded_reason() {
        return Err(format!(
            "{} loaded DEGRADED ({why}); migrating it would carry a recovered \
             prefix into the new files silently. Let the next write repair it, \
             then migrate. Nothing was written",
            legacy.display()
        ));
    }
    let parts = census
        .partitions()
        .map_err(|why| format!("{}: {why}", legacy.display()))?;
    let keys: u64 = parts.iter().map(Manifest::keys).sum();
    let rows: u64 = parts.iter().map(Manifest::total_rows).sum();
    if keys != census.keys() || rows != census.total_rows() {
        return Err(format!(
            "{}: the split holds {keys} keys and {rows} rows, the census {} and {}; \
             nothing was written",
            legacy.display(),
            census.keys(),
            census.total_rows()
        ));
    }
    let header = census.header();
    let legacy_bytes = legacy_meta.as_ref().map_or(0, fs::Metadata::len);
    let written = stage(&staging, vendor, &parts)?;
    write_receipt(
        &staging,
        &legacy,
        (
            census.loaded_version(),
            header.generation,
            header.n_valid,
            legacy_bytes,
        ),
        &written,
    )?;
    sync_dir(&staging)?;
    fs::rename(&staging, &dir).map_err(|why| {
        format!(
            "{} could not be renamed to {}: {why}; the staging directory is left \
             for inspection and the single file is untouched",
            staging.display(),
            dir.display()
        )
    })?;
    sync_dir(&v4_root(root))?;
    drop(lock);
    let partitions: Vec<Written> = written
        .into_iter()
        .map(|mut part| {
            part.path = epoch_path(root, vendor, part.segment, FIRST_EPOCH);
            part
        })
        .collect();
    let done = Migration {
        legacy,
        legacy_present: legacy_meta.is_some(),
        legacy_counters: (
            census.loaded_version(),
            header.generation,
            header.n_valid,
            header.n_keys,
            header.total_rows,
        ),
        legacy_bytes,
        dir,
        partitions,
    };
    note_migrated(vendor, &done);
    Ok(done)
}

/// Where a migration is staged: `manifest/v4/<vendor>.staging`.
#[must_use]
pub fn staging_dir(root: &Path, vendor: Vendor) -> PathBuf {
    v4_root(root).join(format!("{}{STAGING_SUFFIX}", vendor.as_str()))
}

/// Flushes a directory, so a rename into or out of it survives a power cut.
fn sync_dir(dir: &Path) -> Result<(), String> {
    fs::File::open(dir)
        .and_then(|handle| handle.sync_all())
        .map_err(|why| format!("{} could not be flushed: {why}", dir.display()))
}

/// Writes every partition's first epoch into the staging directory and reads
/// each back.
fn stage(staging: &Path, vendor: Vendor, parts: &[Manifest]) -> Result<Vec<Written>, String> {
    fs::create_dir_all(staging).map_err(|why| format!("{}: {why}", staging.display()))?;
    let mut written = Vec::with_capacity(parts.len());
    for part in parts {
        let Some(segment) = part.segment() else {
            return Err(format!(
                "{}: a split produced a census with no segment; nothing was renamed \
                 into place",
                staging.display()
            ));
        };
        let dir = staging.join(segment.as_str());
        let path = dir.join(epoch_name(FIRST_EPOCH));
        let image = part.image();
        let wrote = fs::create_dir_all(&dir).and_then(|()| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            file.write_all(&image)?;
            file.sync_all()
        });
        wrote.map_err(|why| format!("{}: {why}", path.display()))?;
        sync_dir(&dir)?;
        let back = read_epoch(&path, vendor, segment)?;
        if back.keys() != part.keys()
            || back.total_rows() != part.total_rows()
            || back.entries() != part.entries()
            || back.capacity() != part.capacity()
            || back.degraded_reason().is_some()
        {
            return Err(format!(
                "{} did not read back as written: {} keys {} rows {} entries \
                 capacity {} against {} {} {} {}; nothing was renamed into place",
                path.display(),
                back.keys(),
                back.total_rows(),
                back.entries(),
                back.capacity(),
                part.keys(),
                part.total_rows(),
                part.entries(),
                part.capacity()
            ));
        }
        written.push(Written {
            segment,
            path,
            keys: part.keys(),
            rows: part.total_rows(),
            capacity: part.capacity(),
            bytes: u64::try_from(image.len()).unwrap_or(u64::MAX),
        });
    }
    Ok(written)
}

/// The migration's receipt: what single file it read, and what it wrote.
///
/// One `key=value` line each, so a reader can tell, with one `stat`, whether a
/// pre-version-4 writer has added to the single file since: its length is on
/// the `legacy_bytes` line. [`legacy_drift`] is that check.
fn write_receipt(
    staging: &Path,
    legacy: &Path,
    (version, generation, entries, bytes): (u16, u64, u64, u64),
    written: &[Written],
) -> Result<(), String> {
    let mut text = format!(
        "format=census-migration-v1\nlegacy={}\nlegacy_version={version}\n\
         legacy_generation={generation}\nlegacy_entries={entries}\nlegacy_bytes={bytes}\n",
        legacy.display()
    );
    for part in written {
        let _ = writeln!(
            text,
            "partition={} epoch={} keys={} rows={} capacity={}",
            part.segment.as_str(),
            epoch_name(FIRST_EPOCH),
            part.keys,
            part.rows,
            part.capacity
        );
    }
    let path = staging.join(MIGRATION_RECEIPT);
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut file| {
            file.write_all(text.as_bytes())?;
            file.sync_all()
        })
        .map_err(|why| format!("{}: {why}", path.display()))
}

/// Whether the single file has changed since `vendor` was migrated, and how.
///
/// `None` for a vendor that is not migrated, or whose single file is the length
/// the receipt recorded. `Some` names both lengths: a pre-version-4 writer is
/// still appending to a file no version-4 reader consults, so those months are
/// in the census nobody reads (DB-G08's other half). One `stat` and one small
/// read.
#[must_use]
pub fn legacy_drift(root: &Path, vendor: Vendor) -> Option<String> {
    let receipt = vendor_dir(root, vendor).join(MIGRATION_RECEIPT);
    let text = fs::read_to_string(&receipt).ok()?;
    let recorded: u64 = text
        .lines()
        .find_map(|line| line.strip_prefix("legacy_bytes="))
        .and_then(|value| value.parse().ok())?;
    let legacy = manifest_path(root, vendor);
    let now = fs::symlink_metadata(&legacy).map_or(0, |meta| meta.len());
    (now != recorded).then(|| {
        format!(
            "{} is {now} bytes; it was {recorded} when {} was migrated, so a writer \
             built before version 4 has written to it since and those months are \
             in no partition",
            legacy.display(),
            vendor.as_str()
        )
    })
}

fn note_migrated(vendor: Vendor, done: &Migration) {
    let dir = done.dir.display().to_string();
    let (version, generation, entries, keys, rows) = done.legacy_counters;
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(TARGET, "census migrated")
            .with("vendor", telemetry::Value::Str(vendor.as_str()))
            .with("dir", telemetry::Value::Str(&dir))
            .with("legacy_version", telemetry::Value::Uint(u64::from(version)))
            .with("legacy_generation", telemetry::Value::Uint(generation))
            .with("legacy_entries", telemetry::Value::Uint(entries))
            .with("keys", telemetry::Value::Uint(keys))
            .with("rows", telemetry::Value::Uint(rows))
            .with(
                "partitions",
                telemetry::Value::Uint(u64::try_from(done.partitions.len()).unwrap_or(0)),
            ),
    );
}

// ═══ Recount: census rows rebuilt from bar-file headers (DB-G09) ════════════

/// The days a recount covers, inclusive, and so the months it walks: every
/// month that overlaps them. `to` of [`None`] is open-ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// The first day.
    pub from: Day,
    /// The last day, or [`None`] for every day from `from` on.
    pub to: Option<Day>,
}

impl Span {
    /// Whether `month` overlaps the span.
    #[must_use]
    pub fn holds(&self, month: YearMonth) -> bool {
        let first = self.from.year_month().ok();
        let last = self.to.and_then(|to| to.year_month().ok());
        first.is_none_or(|first| month >= first) && last.is_none_or(|last| month <= last)
    }

    /// Whether `day` is inside the span.
    #[must_use]
    pub fn contains(&self, day: Day) -> bool {
        day >= self.from && self.to.is_none_or(|to| day <= to)
    }
}

/// What a recount found and did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Recount {
    /// Bar files under `bars/<vendor>/` whose path is a store path.
    pub files_seen: u64,
    /// Of those, the ones in a month the span overlaps.
    pub files_in_range: u64,
    /// In range and holding no bar, so there is no row to count.
    pub files_empty: u64,
    /// In range and unreadable — locked by a writer, damaged — counted.
    pub unreadable: u64,
    /// The first [`NAMED`] of them, with why.
    pub unreadable_named: Vec<(PathBuf, String)>,
    /// Months holding an unreadable file: no day of these is closed.
    pub months_unreadable: Vec<YearMonth>,
    /// Directory entries that are not part of a store path, counted, so a
    /// stray file is visible and never silently read as a month.
    pub not_store_names: u64,
    /// The first [`NAMED`] of them.
    pub not_store_named: Vec<PathBuf>,
    /// Rows built from headers and offered to the census.
    pub offered: u64,
    /// Rows built from headers and NOT offered, because the GDFL re-key
    /// ledger names their key superseded or retired (D-3331; WP-12c,
    /// `crate::gdfl_rekey::withdrawn`): the census stopped listing those
    /// series on purpose, and a recount must not list them again.
    pub withdrawn: u64,
    /// One per census written.
    pub cycles: Vec<Cycle>,
    /// What the import journal closing did, for a vendor that has one.
    pub journal: Option<JournalClose>,
}

impl Recount {
    /// Rows the census refused, across every cycle.
    #[must_use]
    pub fn refused(&self) -> usize {
        self.cycles.iter().map(|cycle| cycle.refused.len()).sum()
    }

    /// Whether everything in range was read and recorded: nothing unreadable,
    /// nothing refused, and no census-refused day of the span left open.
    #[must_use]
    pub fn clean(&self) -> bool {
        self.unreadable == 0
            && self.refused() == 0
            && self
                .journal
                .as_ref()
                .is_none_or(|journal| journal.left_open.is_empty())
    }
}

/// Rebuilds `vendor`'s census rows for every bar file in a month the span
/// overlaps from the files' own headers, records them, and closes the span's
/// census-refused import days. D-3316 item 6, DB-G09.
///
/// # What is read
///
/// Every directory under `bars/<vendor>/` once, and for each `.bin` month in
/// range: its header (`n_valid`, the first and last stamp) and records `0` and
/// `n_valid − 1` for the two closes — **the same four facts the ingest
/// records after an append, read the same way**. No vendor file, no zip, no
/// network.
///
/// # What is written
///
/// The census, through [`cycle`] in [`Mode::PerKey`]: a row already exact
/// changes nothing, a row that grew is recorded, and a row the census refuses
/// (a count that went backwards) is named and skipped. Bar files are opened
/// through [`BarFile::open_existing`] — read-only, never created, under a
/// SHARED lock — so a month a writer holds is refused by name, not read torn.
/// Then, for the GDFL feed, one `recounted` line per day of the span the
/// import journal holds `incomplete` and the census now counts (see
/// [`close_journal`]).
///
/// # Cost
///
/// O(files · log d) for the walk, where `d` is the widest directory: each
/// directory is sorted so two recounts of one store record rows in one order
/// and publish identical bytes (`CLAUDE.md` §3 rule 5). One header read and two
/// record reads per month in range. The census write is one cycle per census it
/// touches.
///
/// # Errors
///
/// [`locate`]'s, a directory under `bars/<vendor>/` that cannot be listed, a
/// census cycle that refuses, or a journal that cannot be read or appended to —
/// each with what was found up to then.
pub fn recount(
    root: &Path,
    vendor: Vendor,
    span: Span,
    commit: Option<&str>,
) -> Result<Recount, String> {
    let mut found = Recount::default();
    let mut held: Vec<Held> = Vec::new();
    let base = root.join(store::path::STORE_ROOT).join(vendor.as_str());
    // Ordered as it is filled, so the receipt lists the months in order with
    // no sort after the walk.
    let mut unreadable_months: BTreeSet<YearMonth> = BTreeSet::new();
    for (exchange_path, exchange_name) in sorted_dir(&base)? {
        let Ok(exchange) = Exchange::parse(&exchange_name) else {
            stray(&mut found, exchange_path);
            continue;
        };
        for (segment_path, segment_name) in sorted_dir(&exchange_path)? {
            let Ok(segment) = Segment::parse(&segment_name) else {
                stray(&mut found, segment_path);
                continue;
            };
            for (symbol_path, symbol_name) in sorted_dir(&segment_path)? {
                let Some(symbol) = Symbol::new(&symbol_name)
                    .ok()
                    .filter(|symbol| symbol.as_str() == symbol_name)
                else {
                    stray(&mut found, symbol_path);
                    continue;
                };
                let at = Place {
                    root,
                    vendor,
                    exchange,
                    segment,
                    symbol,
                };
                let mut walk = Walk {
                    span,
                    found: &mut found,
                    held: &mut held,
                    unreadable_months: &mut unreadable_months,
                };
                walk_symbol(&at, &symbol_path, &mut walk)?;
            }
        }
    }
    // A KEY THE RE-KEY WITHDREW IS NOT COUNTED BACK (D-3331, WP-12c). Its
    // bar files are kept and stay readable, so the walk finds them; the
    // ledger says the series continues under another key, or is refused, and
    // the census stopped listing it in an epoch of its own. Counted, never
    // silently dropped.
    if vendor == Vendor::Gdfl {
        let withdrawn = crate::gdfl_rekey::withdrawn(root)?;
        let before = held.len();
        held.retain(|row| !withdrawn.holds(&row.entry.key));
        found.withdrawn = u64::try_from(before - held.len()).unwrap_or(u64::MAX);
    }
    found.offered = u64::try_from(held.len()).unwrap_or(u64::MAX);
    found.months_unreadable = unreadable_months.into_iter().collect();
    for (target, rows) in &grouped(root, vendor, &held)? {
        if rows.is_empty() {
            continue;
        }
        let done = cycle(target, rows, Mode::PerKey).map_err(|why| {
            format!(
                "{why} (the recount had read {} file(s) in range, {} unreadable; no \
                 import day was closed)",
                found.files_in_range, found.unreadable
            )
        })?;
        found.cycles.push(done);
    }
    if vendor == Vendor::Gdfl {
        let refused_months: HashSet<YearMonth> = found
            .cycles
            .iter()
            .flat_map(|cycle| cycle.refused.iter().map(|(key, _)| key.month))
            .collect();
        let checked = Checked {
            held: &held,
            unreadable: &found.months_unreadable,
            refused: &refused_months,
            strays: found.not_store_names,
        };
        let journal = close_journal(root, span, commit, &checked).map_err(|why| {
            format!(
                "{why} (the census was recounted and published: {} row(s) offered)",
                found.offered
            )
        })?;
        found.journal = Some(journal);
    }
    note_recounted(vendor, span, &found);
    Ok(found)
}

/// One instrument's place in the store, the part of a path above its rung.
struct Place<'a> {
    root: &'a Path,
    vendor: Vendor,
    exchange: Exchange,
    segment: Segment,
    symbol: Symbol,
}

/// What a walk accumulates.
struct Walk<'a> {
    span: Span,
    found: &'a mut Recount,
    held: &'a mut Vec<Held>,
    unreadable_months: &'a mut BTreeSet<YearMonth>,
}

/// Everything below one symbol directory: rungs directly, or contracts then
/// rungs.
fn walk_symbol(at: &Place<'_>, dir: &Path, walk: &mut Walk<'_>) -> Result<(), String> {
    for (path, name) in sorted_dir(dir)? {
        if let Some(timeframe) = timeframe_named(&name) {
            walk_rung(at, None, timeframe, &path, walk)?;
        } else if let Some(contract) = Contract::parse(&name).filter(|c| c.as_str() == name) {
            for (rung_path, rung_name) in sorted_dir(&path)? {
                match timeframe_named(&rung_name) {
                    Some(timeframe) => walk_rung(at, Some(contract), timeframe, &rung_path, walk)?,
                    None => stray(walk.found, rung_path),
                }
            }
        } else {
            stray(walk.found, path);
        }
    }
    Ok(())
}

/// Every month file in one rung directory.
fn walk_rung(
    at: &Place<'_>,
    contract: Option<Contract>,
    timeframe: Timeframe,
    dir: &Path,
    walk: &mut Walk<'_>,
) -> Result<(), String> {
    for (path, name) in sorted_dir(dir)? {
        // A month's siblings — `.crc`, `.lock`, `.tix`, `.ovl` — are expected
        // here and are not strays; only the bars are counted.
        let Some(stem) = name.strip_suffix(FileKind::Bars.extension()) else {
            if !FileKind::ALL
                .iter()
                .any(|kind| name.ends_with(kind.extension()))
            {
                stray(walk.found, path);
            }
            continue;
        };
        let Some(month) = month_named(stem) else {
            stray(walk.found, path);
            continue;
        };
        walk.found.files_seen += 1;
        if !walk.span.holds(month) {
            continue;
        }
        walk.found.files_in_range += 1;
        match header_row(at, contract, timeframe, month) {
            Ok(Some(row)) => walk.held.push(row),
            Ok(None) => walk.found.files_empty += 1,
            Err(why) => {
                walk.found.unreadable += 1;
                let _ = walk.unreadable_months.insert(month);
                if walk.found.unreadable_named.len() < NAMED {
                    walk.found.unreadable_named.push((path, why));
                }
            }
        }
    }
    Ok(())
}

/// The census row one month's bar file describes, from its header, or `None`
/// for a file that holds no bar.
///
/// The same derivation the ingest uses after an append: rows and both stamps
/// from the committed header, the closes from records `0` and `n_valid − 1`,
/// and the symbol id from the symbol's name — the cross-check the store stamped
/// into the header and verifies on open.
fn header_row(
    at: &Place<'_>,
    contract: Option<Contract>,
    timeframe: Timeframe,
    month: YearMonth,
) -> Result<Option<Held>, String> {
    let path = StorePath::new(PathParts {
        vendor: at.vendor,
        exchange: at.exchange.as_str(),
        segment: at.segment.as_str(),
        symbol: at.symbol.as_str(),
        contract,
        timeframe,
        month,
        file: FileKind::Bars,
    })
    .map_err(|why| why.to_string())?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the symbol id is the low half of a 64-bit FNV-1a, exactly as \
                  `ingest::identify` derives it; the narrowing IS the identity"
    )]
    let symbol_id = brutex_core::universe::fnv1a(at.symbol.as_str()) as u32;
    let file = BarFile::open_existing(at.root, path, symbol_id).map_err(|why| why.to_string())?;
    let header = file.header();
    if header.n_valid == 0 {
        return Ok(None);
    }
    let first = file.read_record(0).map_err(|why| why.to_string())?;
    let last = file
        .read_record(header.n_valid - 1)
        .map_err(|why| why.to_string())?;
    let closes = Closes::known(first.close, last.close).map_err(|why| why.to_string())?;
    Ok(Some(Held::new(
        Entry {
            key: EntryKey {
                contract,
                exchange: at.exchange,
                segment: at.segment,
                symbol: at.symbol,
                timeframe,
                month,
            },
            rows: header.n_valid,
            first_ts_micros: header.first_ts_micros,
            last_ts_micros: header.last_ts_micros,
        },
        closes,
    )))
}

/// The rung a directory name is, if it is one.
fn timeframe_named(name: &str) -> Option<Timeframe> {
    Timeframe::KNOWN
        .iter()
        .copied()
        .find(|timeframe| timeframe.as_str() == name)
}

/// The month a `YYYY-MM` stem is, if it is one in the store's own spelling.
fn month_named(stem: &str) -> Option<YearMonth> {
    let (year, month) = stem.split_once('-')?;
    if year.len() != 4 || month.len() != 2 {
        return None;
    }
    let month = YearMonth::new(year.parse().ok()?, month.parse().ok()?).ok()?;
    (month.to_string() == stem).then_some(month)
}

/// Records one directory entry that is not part of a store path.
fn stray(found: &mut Recount, path: PathBuf) {
    found.not_store_names += 1;
    if found.not_store_named.len() < NAMED {
        found.not_store_named.push(path);
    }
}

/// One directory's entries, sorted by name, with names that are not text
/// returned under their lossy rendering so they are counted as strays rather
/// than skipped.
///
/// An absent directory is an empty one: a vendor with no bars has nothing to
/// recount, and that is an answer, not a failure.
fn sorted_dir(dir: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(absent) if absent.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{} could not be listed: {why}", dir.display())),
    };
    let mut out: Vec<(PathBuf, String)> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|why| format!("{} could not be listed: {why}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        out.push((entry.path(), name));
    }
    out.sort_unstable_by(|a, b| a.1.cmp(&b.1));
    Ok(out)
}

fn note_recounted(vendor: Vendor, span: Span, found: &Recount) {
    let from = span.from.to_string();
    let to = span
        .to
        .map_or_else(|| "open".to_owned(), |to| to.to_string());
    let recorded: usize = found.cycles.iter().map(|cycle| cycle.recorded).sum();
    let refused = found.refused();
    let (closed, left_open) = found.journal.as_ref().map_or((0, 0), |journal| {
        (journal.closed.len(), journal.left_open.len())
    });
    let level = if found.clean() {
        telemetry::Level::Info
    } else {
        telemetry::Level::Warn
    };
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(level, TARGET, "census recounted")
            .with("vendor", telemetry::Value::Str(vendor.as_str()))
            .with("from", telemetry::Value::Str(&from))
            .with("to", telemetry::Value::Str(&to))
            .with("files_seen", telemetry::Value::Uint(found.files_seen))
            .with(
                "files_in_range",
                telemetry::Value::Uint(found.files_in_range),
            )
            .with("files_empty", telemetry::Value::Uint(found.files_empty))
            .with("unreadable", telemetry::Value::Uint(found.unreadable))
            .with(
                "not_store_names",
                telemetry::Value::Uint(found.not_store_names),
            )
            .with("offered", telemetry::Value::Uint(found.offered))
            .with(
                "recorded",
                telemetry::Value::Uint(u64::try_from(recorded).unwrap_or(u64::MAX)),
            )
            .with(
                "refused",
                telemetry::Value::Uint(u64::try_from(refused).unwrap_or(u64::MAX)),
            )
            .with(
                "days_closed",
                telemetry::Value::Uint(u64::try_from(closed).unwrap_or(u64::MAX)),
            )
            .with(
                "days_left_open",
                telemetry::Value::Uint(u64::try_from(left_open).unwrap_or(u64::MAX)),
            ),
    );
}

// ═══ The import journal's `recounted` line (D-3317 item 5) ══════════════════

/// The import journal's torn-line mark, as `pull::gdfl_import` writes it
/// (D-2814): a line a crash cut short is closed with it before the next write.
const TORN: &str = " (torn)";

/// The verbs the journal holds since D-3317, each a line's first field.
const JOURNAL_VERBS: [&str; 6] = ["begin", "done", "incomplete", "refused", "run", "recounted"];

/// One day the recount closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Closed {
    /// The journal key: `<kind> <day> <filter>`.
    pub key: String,
    /// The `months=` the line carries: the instrument-months of the day's
    /// segment the census now holds whose stored span includes the day.
    pub months: u64,
    /// The `files=` of the day's `incomplete` line, when it had one.
    pub files: Option<u64>,
}

/// What the journal closing did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JournalClose {
    /// The journal.
    pub path: PathBuf,
    /// The run id the `recounted` lines carry, when any was written.
    pub run: Option<u64>,
    /// Every day closed with a `recounted` line, in key order.
    pub closed: Vec<Closed>,
    /// Census-refused days of the span left `incomplete`, with why.
    pub left_open: Vec<(String, String)>,
    /// Days of the span closed `incomplete` at a stage that is not the census
    /// (D-3317 item 6), which a recount never closes.
    pub other_stage: Vec<String>,
}

/// What the recount established, which decides whether a day closes.
#[derive(Debug, Clone, Copy)]
pub struct Checked<'a> {
    /// Every row the recount built from a bar header.
    pub held: &'a [Held],
    /// Months holding a bar file that could not be read.
    pub unreadable: &'a [YearMonth],
    /// Months holding a row the census refused.
    pub refused: &'a HashSet<YearMonth>,
    /// Directory entries under `bars/<vendor>/` the walk could not name as
    /// part of a store path. Any one of them could be a month a day touched
    /// that the recount did not read, so while there is one no day closes.
    pub strays: u64,
}

/// The state of one journal key after its last line.
#[derive(Debug, Clone, PartialEq, Eq)]
enum KeyState {
    /// Begun and not closed.
    Open,
    /// Closed: done, done with refusals, or recounted.
    Done,
    /// Closed `incomplete`, with that line's fields.
    Incomplete(Vec<String>),
    /// Closed `refused`.
    Refused,
}

/// The value of `name=` among `fields`.
fn field<'a>(fields: &'a [String], name: &str) -> Option<&'a str> {
    fields.iter().find_map(|one| {
        one.strip_prefix(name)
            .and_then(|rest| rest.strip_prefix('='))
    })
}

/// Closes the span's census-refused days of the GDFL import journal with a
/// `recounted` line each, once the census holds every month the day touched.
/// D-3316 item 6, D-3317 item 5.
///
/// # Which days
///
/// A key whose LAST line is `incomplete` and whose day is in the span —
/// `refused`, `begin` (a crashed day) and anything done are never touched. A
/// line that names `stage=` other than `census` is not closed (D-3317 item 6);
/// a line with no stage (every line before D-3317) cannot say why it was
/// incomplete, so the operator's span is what excludes the days refused for any
/// other reason, exactly as D-3317 item 5 states.
///
/// # When a day closes
///
/// Only when the census now holds every month it touched: no file of the day's
/// month was unreadable, no row of it refused, no directory entry under the
/// vendor's bars went unnamed, and at least one instrument-month of the day's
/// own segment (the key's kind: `options` is `FNO`, `indices` `INDEX`,
/// `stocks` `CASH`) spans the day. Otherwise it is left open and named, as is a
/// key whose kind names no segment and a day with no `definition=`: the line
/// it would be closed with must state the bar definition that built it. The
/// day's `files=` is carried beside the months that span it and decides
/// nothing; [`day_verdict`] says why.
///
/// # What is written
///
/// One `run <id> source=recount commit=<sha> started=<unix_s> from=<day>
/// to=<day>` line before the first `recounted` line, so the `run=` each carries
/// names a line of the journal as D-3317 item 2 requires of every closing line.
/// D-3317 item 4 names the import's two sources, `zip` and `tickstore`; a
/// recount reads neither and says so with `source=recount`, a value the loader
/// accepts as it accepts every `name=value` field of a run line. `commit` is
/// the build's stamp, `unrecorded` when the binary carries none. Then one
/// `recounted <key>
/// definition=<n> months=<n> run=<id>` line per day. Nothing is written when no
/// day closes. A torn last line is closed with the mark first (D-2814). The
/// journal is read whole before anything is written, and a line this build does
/// not know refuses the closing with nothing written.
///
/// # Errors
///
/// The journal and the host's words, for a journal that cannot be read or
/// written, or holds a line this build does not know.
pub fn close_journal(
    root: &Path,
    span: Span,
    commit: Option<&str>,
    checked: &Checked<'_>,
) -> Result<JournalClose, String> {
    let Checked {
        held,
        unreadable,
        refused,
        strays,
    } = *checked;
    let path = crate::gdfl_import::journal_path(root);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(absent) if absent.kind() == ErrorKind::NotFound => String::new(),
        Err(why) => return Err(format!("{}: {why}", path.display())),
    };
    let (states, runs) = journal_states(&path, &text)?;

    // Rows per (segment, month), so each day counts only its own month's rows
    // OF ITS OWN KIND: an options day's `files=` are option contracts, and an
    // index or share month spanning the same day is not one of them.
    let mut by_month: Spans = BTreeMap::new();
    for one in held {
        by_month
            .entry((one.entry.key.segment, one.entry.key.month))
            .or_default()
            .push((one.entry.first_ts_micros, one.entry.last_ts_micros));
    }
    // The months no day of may close, and why: an unreadable file outranks a
    // refused row, because a month the recount could not read is one it
    // cannot say anything about.
    let mut blocked: BTreeMap<YearMonth, Blocked> = BTreeMap::new();
    for month in refused {
        let _ = blocked.insert(*month, Blocked::Refused);
    }
    for month in unreadable {
        let _ = blocked.insert(*month, Blocked::Unreadable);
    }
    let mut done = JournalClose {
        path: path.clone(),
        ..JournalClose::default()
    };
    for (key, state) in &states {
        let KeyState::Incomplete(fields) = state else {
            continue;
        };
        let mut words = key.split(' ');
        let (Some(kind), Some(day)) = (words.next(), words.next().and_then(parse_day)) else {
            continue;
        };
        if !span.contains(day) {
            continue;
        }
        if let Some(stage) = field(fields, "stage")
            && stage != "census"
        {
            done.other_stage.push(key.clone());
            continue;
        }
        match day_verdict((kind, day), fields, &by_month, (&blocked, strays)) {
            Ok(closed) => done.closed.push(Closed {
                key: key.clone(),
                ..closed
            }),
            Err(why) => done.left_open.push((key.clone(), why)),
        }
    }
    if done.closed.is_empty() {
        return Ok(done);
    }
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    // `started`, raised by one while an earlier `run` line holds it (D-3317
    // item 4): one ordered walk of the ids at or past it, stopping at the
    // first gap.
    let mut id = started;
    for held in runs.range(started..) {
        if *held != id {
            break;
        }
        id += 1;
    }
    let to = span
        .to
        .map_or_else(|| "open".to_owned(), |to| to.to_string());
    let mut lines = format!(
        "run {id} source=recount commit={} started={started} from={} to={to}\n",
        commit.unwrap_or("unrecorded"),
        span.from
    );
    for closed in &done.closed {
        let definition = states
            .get(&closed.key)
            .and_then(|state| match state {
                KeyState::Incomplete(fields) => field(fields, "definition"),
                _ => None,
            })
            .unwrap_or("0");
        let _ = writeln!(
            lines,
            "recounted {} definition={definition} months={} run={id}",
            closed.key, closed.months
        );
    }
    append_journal(&path, &lines)?;
    done.run = Some(id);
    Ok(done)
}

/// The first and last stamps of every row the recount built, by the segment
/// and month of its key.
type Spans = BTreeMap<(Segment, YearMonth), Vec<(i64, i64)>>;

/// Why no day of a month may close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Blocked {
    /// A bar file of the month could not be read.
    Unreadable,
    /// The census refused a row of the month.
    Refused,
}

/// Whether `day`'s months are all in the census, and the `recounted` fields if
/// they are. `kind` is the journal key's first word, which names the segment
/// the day's files were filed under (`pull::gdfl_import::ImportKind`).
///
/// # What proves a day is counted
///
/// The recount read every bar file of every month its span overlaps, so the
/// census now holds every instrument-month on disk in the day's month — which
/// is every month the day touched, since a day's bars are filed in the day's
/// own month — unless one of them could not be read, was refused, or sits
/// under a name the walk could not attribute. Those three leave the day open,
/// and so does a day no instrument-month of its segment spans at all: a day
/// closed `incomplete` had begun a write, so at least one month must span it.
///
/// # What `files=` is, and why it decides nothing
///
/// The import counts every file it CONVERTED, including one whose rows gave no
/// in-session second, which writes no bar. Measured on 2020-01-01 against the
/// recounted census: 3,633 files, 3,632 option months spanning the day, and
/// the one difference a 396-byte file that wrote no bar that day. So `months`
/// and `files` are both carried on the receipt — `months` on the `recounted`
/// line — and a shortfall is the reader's to see, never a reason to hold a day
/// the census provably counts.
fn day_verdict(
    (kind, day): (&str, Day),
    fields: &[String],
    by_month: &Spans,
    (blocked, strays): (&BTreeMap<YearMonth, Blocked>, u64),
) -> Result<Closed, String> {
    let month = day
        .year_month()
        .map_err(|why| format!("the day names no store month: {why}"))?;
    let segment = crate::gdfl_import::ImportKind::parse(kind)
        .ok()
        .and_then(|kind| Segment::parse(kind.segment()).ok())
        .ok_or_else(|| format!("its kind `{kind}` names no segment the import files under"))?;
    if field(fields, "definition").is_none() {
        return Err("its incomplete line names no bar definition".to_owned());
    }
    if strays > 0 {
        return Err(format!(
            "{strays} directory entr(ies) under the vendor's bars are not store names, so \
             the census may not hold every month the day touched"
        ));
    }
    match blocked.get(&month) {
        Some(Blocked::Unreadable) => {
            return Err(format!(
                "a bar file of {month} could not be read, so the census may not hold \
                 every month the day touched"
            ));
        }
        Some(Blocked::Refused) => {
            return Err(format!(
                "the census refused a row of {month}, so it does not hold every month \
                 the day touched"
            ));
        }
        None => {}
    }
    let (start, end) = ist_day_micros(day);
    let months = by_month.get(&(segment, month)).map_or(0, |spans| {
        spans
            .iter()
            .filter(|(first, last)| *first < end && *last >= start)
            .count()
    });
    let months = u64::try_from(months).unwrap_or(u64::MAX);
    if months == 0 {
        return Err(format!(
            "no {} instrument-month the census holds spans the day, and a day closed \
             incomplete had begun a write",
            segment.as_str()
        ));
    }
    Ok(Closed {
        key: String::new(),
        months,
        files: field(fields, "files").and_then(|text| text.parse::<u64>().ok()),
    })
}

/// The IST bounds of `day`, `[start, end)`, in microseconds since the epoch.
fn ist_day_micros(day: Day) -> (i64, i64) {
    let start =
        i64::from(day.days_from_epoch()) * DAY_MICROS - crate::session::IST_OFFSET_SECS * 1_000_000;
    (start, start + DAY_MICROS)
}

/// A `YYYY-MM-DD` journal day.
fn parse_day(text: &str) -> Option<Day> {
    let mut parts = text.split('-');
    let (Some(y), Some(m), Some(d), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    if y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    let day = Day::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?).ok()?;
    (day.to_string() == text).then_some(day)
}

/// The last state of every key, and every run id, from the journal's whole
/// lines. A torn tail and a line closed with the torn mark are skipped, as the
/// import's loader skips them.
fn journal_states(
    path: &Path,
    text: &str,
) -> Result<(BTreeMap<String, KeyState>, BTreeSet<u64>), String> {
    let mut states: BTreeMap<String, KeyState> = BTreeMap::new();
    let mut runs: BTreeSet<u64> = BTreeSet::new();
    let whole = text.rsplit_once('\n').map_or("", |(whole, _)| whole);
    for line in whole.split('\n').filter(|line| !line.is_empty()) {
        if line.ends_with(TORN) {
            continue;
        }
        let fields: Vec<&str> = line.split(' ').collect();
        let foreign = || {
            format!(
                "{}: not a journal line this build knows: {line:?}; nothing was written",
                path.display()
            )
        };
        match fields.as_slice() {
            ["run", id, ..] => {
                let _ = runs.insert(id.parse().map_err(|_| foreign())?);
            }
            [verb, kind, day, only, rest @ ..] if JOURNAL_VERBS.contains(verb) => {
                let key = format!("{kind} {day} {only}");
                let state = match *verb {
                    "begin" if rest.is_empty() => KeyState::Open,
                    "done" | "recounted" => KeyState::Done,
                    "incomplete" => {
                        KeyState::Incomplete(rest.iter().map(|one| (*one).to_owned()).collect())
                    }
                    "refused" => KeyState::Refused,
                    _ => return Err(foreign()),
                };
                let _ = states.insert(key, state);
            }
            _ => return Err(foreign()),
        }
    }
    Ok((states, runs))
}

/// Appends `lines` to the journal, durably, after closing a torn last line with
/// the mark — the import's own rule (D-2814, D-2826).
fn append_journal(path: &Path, lines: &str) -> Result<(), String> {
    use std::io::Seek as _;
    let refused = |why: std::io::Error| format!("{}: {why}", path.display());
    let mut file = fs::OpenOptions::new()
        .read(true)
        .append(true)
        .open(path)
        .map_err(refused)?;
    let mut text = String::new();
    if file.metadata().map_err(refused)?.len() > 0 {
        let mut last = [0_u8; 1];
        file.seek(std::io::SeekFrom::End(-1)).map_err(refused)?;
        file.read_exact(&mut last).map_err(refused)?;
        if last != *b"\n" {
            text.push_str(TORN);
            text.push('\n');
        }
    }
    text.push_str(lines);
    file.write_all(text.as_bytes()).map_err(refused)?;
    file.sync_all().map_err(refused)
}

/// The arms only this module can reach: the run-level compaction, the epoch
/// install's refusals, the drift check and the migration's refusals.
#[cfg(test)]
#[path = "census_store_tests.rs"]
mod tests;
