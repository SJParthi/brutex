//! `/disk.json` — the free space under the store, what the store holds, what
//! the census keeps forever, and whether an independent copy is recorded
//! (DB-B73, DB-B59; WP-81).
//!
//! # The failure this surfaces
//!
//! One drive holds the purchased vendor archives and the whole store. Time
//! Machine reported `No destinations configured` and listed the store as
//! excluded; no snapshot preceded the census migration, recount and import
//! writes of 10 Oct 2026; and no route or page said how much room was left,
//! how much the store already held, or that nothing was copied anywhere. A
//! drive failure or a bad bulk write would have been found by its absence.
//!
//! # What it answers
//!
//! * `filesystem` — total, used and free bytes of the filesystem the store
//!   root is on and where it is mounted, from one `statfs(2)` on macOS
//!   (D-5241) or one `statvfs(2)` elsewhere, in this process (D-5240: it was
//!   a `/bin/df -Pk` child), or the refusal naming the root — one that does
//!   not exist, is no directory, or whose filesystem answers counts no
//!   filesystem can have is not measured. `used_bytes` is the fragments not
//!   free; on APFS those are the container's, every volume in it, not the
//!   volume's own figure `df` printed (`docs/06-limits.md` L-RUST1-1). No
//!   page reads `used_bytes`.
//! * `feeds` — per vendor with a census: its format, its months and rows,
//!   the bar bytes those rows occupy at the bar file's own geometry (a LOWER
//!   BOUND: sidecars and time indexes are not counted), and every census file
//!   with its role — `current`, `older-epoch`, `superseded`, `sealed-stub` or
//!   `single` — and bytes.
//! * `census_kept_bytes` — what keeping every older epoch and every superseded
//!   single file costs today: the census's retention is "keep everything"
//!   (`CLAUDE.md` §3 rule 8, `census_store` "never deleted"), and this is the
//!   number that rule spends (DB-B59).
//! * each feed's `scrub` — where its full-block scrub stands (`cli
//!   scrub-blocks`, DB-B74): the pass, whether it is complete, its totals of
//!   months, blocks, bad, unverifiable, misfiled and unbound months, how many
//!   findings still stand (`standing_findings`, which counts the ones KEPT
//!   for a month a pass met busy and did not read) and the first of them by
//!   name, how many months the standing passes did not read
//!   (`standing_busy`) — or `"recorded": false` with the reason when no
//!   scrub of that feed ever ran. The series a
//!   page shows asks `/scrub.json` for its own findings.
//! * `backup` — the last line of `<store>/ops/backups.log`, the receipt the
//!   runbook's copy procedure appends (`docs/36-ops-runbook.md`), or
//!   `"recorded": false` said loudly. This build cannot see a backup it was
//!   not told about, and says so rather than guess.
//!
//! # Cost
//!
//! No child process. On macOS one `statfs(2)`; elsewhere one `statvfs(2)` and
//! one `stat` per directory from the store root up to its mount point, bounded
//! by the path's depth. Then one `lstat` per census file and one directory
//! listing per migrated segment — O(epochs), which compaction alone grows, a
//! handful today — a 4 KiB tail read of the receipt log, and per feed one
//! 8 KiB tail read of its scrub cursor (the findings index behind it is
//! rebuilt only when its ledger moves, `scrub_route::with_index`). The answer is
//! kept for [`DISK_TTL`] per store root, so a page polling it costs one
//! `Mutex` read between measurements.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use brutex_core::vendor::Vendor;
use pull::ingest::census_store::{self, MIGRATION_RECEIPT as MIGRATION_RECEIPT_NAME};
use pull::manifest::{SEGMENTS, manifest_path, parse_epoch_name, segment_dir, vendor_dir};

use super::{Loaded, Site, vendor_census};
use crate::render;

/// How long one measurement is served before the next is taken.
pub(crate) const DISK_TTL: Duration = Duration::from_secs(30);

/// The receipt log the runbook's copy procedure appends one line to per
/// completed copy, relative to the store root.
pub(crate) const BACKUP_RECEIPTS: &str = "ops/backups.log";

/// The most bytes of the receipt log read: its last line is the answer.
const RECEIPT_TAIL_BYTES: u64 = 4_096;

/// The most standing scrub findings named per feed in one answer.
const MAX_NAMED_BAD: usize = 16;

/// The last answer, for one store root, and when it was measured.
static LAST: Mutex<Option<(PathBuf, Instant, String)>> = Mutex::new(None);

/// The call `/disk.json`'s `filesystem.source` names: the one
/// [`counts_and_mount`] makes on this platform (D-5241).
#[cfg(target_vendor = "apple")]
const SOURCE: &str = "statfs";

/// The call `/disk.json`'s `filesystem.source` names: the one
/// [`counts_and_mount`] makes on this platform (D-5240).
#[cfg(not(target_vendor = "apple"))]
const SOURCE: &str = "statvfs";

/// One filesystem, as the filesystem itself reports it, in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Filesystem {
    pub(crate) total: u64,
    pub(crate) used: u64,
    pub(crate) free: u64,
    pub(crate) mount: String,
}

/// What a filesystem says of itself, in its own fragments, before any of it
/// becomes a byte: the fragment size, the fragments it has, the fragments
/// free, and the fragments an unprivileged writer such as this server may
/// still take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Counts {
    fragment: u64,
    blocks: u64,
    free: u64,
    available: u64,
}

impl Counts {
    /// `(total, used, free)` in bytes: `total` is every fragment; `used` is
    /// every fragment less the free ones, so the fragments a filesystem keeps
    /// for its superuser count as used, as `df` counts them; `free` is what an
    /// unprivileged writer may take.
    ///
    /// # Errors
    ///
    /// AN ANSWER NO FILESYSTEM CAN GIVE IS REFUSED BY NAME (D-5241), never
    /// served as a number: a fragment of 0 bytes (every count would read 0),
    /// more free or available fragments than it has (used would wrap or be
    /// clamped to 0), or more bytes than 64 bits hold. Each was a
    /// `saturating_` call that answered a wrong number quietly.
    fn in_bytes(self, root: &Path) -> Result<(u64, u64, u64), String> {
        let refuse = |what: &str| {
            format!(
                "{} cannot be measured: its filesystem answered {what} ({self:?})",
                root.display()
            )
        };
        if self.fragment == 0 {
            return Err(refuse("a fragment of 0 bytes"));
        }
        let used = self
            .blocks
            .checked_sub(self.free)
            .ok_or_else(|| refuse("more free fragments than it has"))?;
        if self.available > self.blocks {
            return Err(refuse("more available fragments than it has"));
        }
        // `used` and `available` are at most `blocks`, so only `blocks` can
        // overflow; all three are checked alike rather than argued about.
        let bytes = |fragments: u64| {
            fragments
                .checked_mul(self.fragment)
                .ok_or_else(|| refuse("more bytes than 64 bits hold"))
        };
        Ok((bytes(self.blocks)?, bytes(used)?, bytes(self.available)?))
    }
}

/// The sentence naming `root` when `call` refused it.
fn refused(root: &Path, call: &str, why: rustix::io::Errno) -> String {
    if why == rustix::io::Errno::NOENT {
        format!(
            "{} does not exist, so no filesystem is measured for it",
            root.display()
        )
    } else {
        format!(
            "{} cannot be measured: {call} answered {why}",
            root.display()
        )
    }
}

/// `root`'s metadata when it is a directory. A FILE IS NO STORE ROOT: `df`
/// measured the filesystem around one, which answered a question nobody asked
/// about the store.
fn directory(root: &Path) -> Result<std::fs::Metadata, String> {
    std::fs::metadata(root)
        .ok()
        .filter(std::fs::Metadata::is_dir)
        .ok_or_else(|| {
            format!(
                "{} is not a directory, so it is no store root whose filesystem is measured",
                root.display()
            )
        })
}

/// The filesystem `root` is on, measured now and in this process.
///
/// NO CHILD PROCESS (D-5240). This was `/bin/df -Pk <root>`, the one program
/// outside the browser opener that shipped code started;
/// `crates/vocab/tests/no_runtime_spawn.rs` now refuses another. `df -Pk`
/// printed the same counts in 1024-byte blocks, so on a filesystem whose
/// fragment is a multiple of 1 KiB these are the bytes it gave, and on any
/// other they are exact where it rounded — except macOS's Used on APFS, which
/// is the volume's own allocation and not these counts (D-5241).
///
/// # Errors
///
/// A sentence naming `root`: it does not exist, it cannot be measured (the
/// call's own error, such as a path through a regular file, or an answer no
/// filesystem can give), it is not a directory, or its mount point cannot be
/// found.
fn measure(root: &Path) -> Result<Filesystem, String> {
    let (counts, mount) = counts_and_mount(root)?;
    let (total, used, free) = counts.in_bytes(root)?;
    Ok(Filesystem {
        total,
        used,
        free,
        mount,
    })
}

/// ON macOS ONE `statfs(2)` (D-5241): `f_bsize`, `f_blocks`, `f_bfree`,
/// `f_bavail` and `f_mntonname`, whose total, available bytes and mount are
/// what `/bin/df -Pk` prints (RUST1-09). ITS USED IS NOT `df`'S ON APFS: there
/// `f_blocks` and `f_bfree` are the container's, shared by every volume in it,
/// so `used_bytes` is the container's allocation, while `df` prints the
/// volume's own, which `statfs(2)` does not carry (`docs/06-limits.md`
/// L-RUST1-1). Not `statvfs(2)`: Darwin's `fsblkcnt_t` is 32 bits wide
/// (`sys/_types.h`, `docs/00-charter.md`), so its block counts cannot hold a
/// volume of 2^32 fragments or more (16 TiB at 4 KiB), while `statfs`'s are
/// 64-bit. Not a walk upward for the mount: APFS firmlinks put `/Users` and
/// `/private/var` on the data volume mounted at `/System/Volumes/Data`, so the
/// highest directory on the device is not where it is mounted.
///
/// # Errors
///
/// A sentence naming `root` when `statfs(2)` refuses it, when it is no
/// directory, or when the mount point it names is not absolute.
#[cfg(target_vendor = "apple")]
fn counts_and_mount(root: &Path) -> Result<(Counts, String), String> {
    let fs = rustix::fs::statfs(root).map_err(|why| refused(root, "statfs", why))?;
    directory(root)?;
    let name: Vec<u8> = fs
        .f_mntonname
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| u8::from_ne_bytes(c.to_ne_bytes()))
        .collect();
    let mount = String::from_utf8_lossy(&name).into_owned();
    if !mount.starts_with('/') {
        return Err(format!(
            "{} has no mount point to name: statfs answered {mount:?}",
            root.display()
        ));
    }
    Ok((
        Counts {
            fragment: u64::from(fs.f_bsize),
            blocks: fs.f_blocks,
            free: fs.f_bfree,
            available: fs.f_bavail,
        },
        mount,
    ))
}

/// ELSEWHERE ONE `statvfs(2)` gives the counts, each in `f_frsize`-byte
/// fragments, and the mount point is the highest directory above the
/// canonical root still on its device, as coreutils' `find_mount_point` finds
/// it — one `stat` per directory between them, bounded by the path's depth. A
/// bind mount of the same device higher up is not told apart, as that walk
/// does not tell it apart.
///
/// # Errors
///
/// A sentence naming `root` when `statvfs(2)` refuses it, when it is no
/// directory, or when it cannot be made canonical.
#[cfg(not(target_vendor = "apple"))]
fn counts_and_mount(root: &Path) -> Result<(Counts, String), String> {
    use std::os::unix::fs::MetadataExt as _;
    let vfs = rustix::fs::statvfs(root).map_err(|why| refused(root, "statvfs", why))?;
    let meta = directory(root)?;
    let canonical = std::fs::canonicalize(root).map_err(|why| {
        format!(
            "{} has no mount point to name: it cannot be made canonical ({why})",
            root.display()
        )
    })?;
    let mut mount = canonical.as_path();
    while let Some(parent) = mount.parent()
        && std::fs::metadata(parent).is_ok_and(|above| above.dev() == meta.dev())
    {
        mount = parent;
    }
    Ok((
        Counts {
            fragment: vfs.f_frsize,
            blocks: vfs.f_blocks,
            free: vfs.f_bfree,
            available: vfs.f_bavail,
        },
        mount.display().to_string(),
    ))
}

/// One census file and what it is to the census.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CensusFile {
    pub(crate) path: PathBuf,
    pub(crate) bytes: u64,
    pub(crate) role: &'static str,
}

impl CensusFile {
    /// Whether keeping it is retention rather than the census in use.
    fn kept(&self) -> bool {
        matches!(self.role, "older-epoch" | "superseded")
    }
}

/// Every census file `vendor` has under `root`, each with its role.
pub(crate) fn census_files(root: &Path, vendor: Vendor) -> Vec<CensusFile> {
    let mut out = Vec::new();
    let migrated = std::fs::symlink_metadata(vendor_dir(root, vendor)).is_ok_and(|m| m.is_dir());
    let superseded = census_store::superseded_path(root, vendor);
    let sealed = std::fs::symlink_metadata(&superseded).is_ok();
    let single = manifest_path(root, vendor);
    if let Ok(meta) = std::fs::symlink_metadata(&single) {
        out.push(CensusFile {
            bytes: meta.len(),
            role: match (migrated, sealed) {
                (false, _) => "single",
                (true, true) => "sealed-stub",
                // MIGRATED AND NOT SEALED: still the whole v3 census at the
                // name a pre-version-4 reader opens (DB-B68).
                (true, false) => "superseded",
            },
            path: single,
        });
    }
    if let Ok(meta) = std::fs::symlink_metadata(&superseded) {
        out.push(CensusFile {
            path: superseded,
            bytes: meta.len(),
            role: "superseded",
        });
    }
    if !migrated {
        return out;
    }
    for segment in SEGMENTS {
        let dir = segment_dir(root, vendor, segment);
        let Ok(listing) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut epochs: Vec<(u32, PathBuf, u64)> = listing
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let epoch = parse_epoch_name(entry.file_name().to_str()?)?;
                let bytes = entry.metadata().ok()?.len();
                Some((epoch, entry.path(), bytes))
            })
            .collect();
        epochs.sort_unstable_by_key(|(epoch, _, _)| *epoch);
        let current = epochs.last().map(|(epoch, _, _)| *epoch);
        for (epoch, path, bytes) in epochs {
            out.push(CensusFile {
                path,
                bytes,
                role: if Some(epoch) == current {
                    "current"
                } else {
                    "older-epoch"
                },
            });
        }
    }
    let receipt = vendor_dir(root, vendor).join(MIGRATION_RECEIPT_NAME);
    if let Ok(meta) = std::fs::symlink_metadata(&receipt) {
        out.push(CensusFile {
            path: receipt,
            bytes: meta.len(),
            role: "receipt",
        });
    }
    out
}

/// Where `vendor`'s full-block scrub stands, as JSON: its pass, whether it
/// is complete, its totals, how many findings still stand and the first
/// [`MAX_NAMED_BAD`] of them — or that no scrub of it ever ran (DB-B74).
pub(crate) fn scrub_json(root: &Path, vendor: Vendor) -> String {
    let ledger = render::json_string(
        &root
            .join(cli::scrub_ledger_name(vendor))
            .display()
            .to_string(),
    );
    let state = match super::scrub_route::standing(root, vendor) {
        Ok(state) => state,
        Err(why) => {
            return format!(
                r#"{{"ledger":{ledger},"recorded":false,"why":{}}}"#,
                render::json_string(&why)
            );
        }
    };
    let from_pass = state.standing_from();
    // READ UNDER THE INDEX'S LOCK, AND BOUNDED: one sum and the first
    // `MAX_NAMED_BAD` standing findings, kept in this order as they are
    // indexed — never a walk or a sort of the ledger's findings.
    let rendered = super::scrub_route::with_index(root, vendor, |found| {
        let mut out = format!(
            r#"{{"ledger":{ledger},"recorded":true,"pass":{},"state":{},"totals":{},"standing_from_pass":{from_pass},"standing_findings":{},"standing_busy":{},"truncated":{},"named":["#,
            state.pass,
            render::json_string(if state.complete {
                "complete"
            } else {
                "in progress"
            }),
            state.totals_json(),
            found.standing_count(from_pass),
            found.busy_count(from_pass),
            found.truncated
        );
        for (n, (series, finding)) in found.named(from_pass, MAX_NAMED_BAD).iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                r#"{{"series":{},"finding":"#,
                render::json_string(series)
            );
            super::scrub_route::finding_json(&mut out, finding);
            out.push('}');
        }
        out.push_str("]}");
        out
    });
    rendered.unwrap_or_else(|why| {
        format!(
            r#"{{"ledger":{ledger},"recorded":false,"why":{}}}"#,
            render::json_string(&why)
        )
    })
}

/// The longest receipt line served, in bytes: a timestamp, a path and three
/// numbers. A longer line is not a receipt the runbook's procedure writes.
const MAX_RECEIPT_LINE: usize = 1_024;

/// The newest whole receipt of the receipt log, read from its last
/// [`RECEIPT_TAIL_BYTES`], or why there is none.
///
/// A receipt is a WHOLE line (ended by its newline) of at most
/// [`MAX_RECEIPT_LINE`] bytes that carries `copied=<when>` and `dest=<where>`,
/// each with a value — the two words `docs/36-ops-runbook.md` §4 writes first.
/// Any other newest line is NOT a recorded backup: before this, whatever the
/// file's last line was — a note, half a line, a shell error — was served as
/// "the last copy", which is the one page a person reads before trusting that
/// a copy exists.
///
/// The log is opened without following a link or waiting on a FIFO and must
/// be a regular file (`scrub_route::open_plain`).
///
/// # Errors
///
/// A sentence: no log, one that cannot be read or is no regular file, one
/// with no whole line, or one whose newest whole line is no receipt.
pub(crate) fn last_backup(root: &Path) -> Result<String, String> {
    let path = root.join(BACKUP_RECEIPTS);
    let (tail, cut) = super::scrub_route::tail_of(&path, RECEIPT_TAIL_BYTES).map_err(|why| {
        if why.kind() == std::io::ErrorKind::NotFound {
            format!(
                "{} cannot be read ({why}): no copy of this store is recorded. One drive \
                     holds the raw vendor data and the store; the runbook \
                     (docs/36-ops-runbook.md) says how to make and record one",
                path.display()
            )
        } else {
            format!(
                "{} cannot be read ({why}): whether a copy of this store is recorded \
                     cannot be told",
                path.display()
            )
        }
    })?;
    // WHOLE LINES ONLY: what follows the last newline is a line nobody
    // finished, and a tail cut from a longer log begins inside one.
    let whole = tail
        .rfind('\n')
        .map_or("", |end| tail.get(..end).unwrap_or(""));
    let mut lines = whole.split('\n');
    if cut {
        let _fragment = lines.next();
    }
    let line = lines
        .rfind(|line| !line.trim().is_empty())
        .ok_or_else(|| format!("{} holds no whole receipt line", path.display()))?;
    let value = |key: &str| {
        line.split(' ')
            .find_map(|word| word.strip_prefix(key)?.strip_prefix('='))
            .filter(|value| !value.is_empty())
    };
    // `copied=` IS A UTC INSTANT, NOT ANY WORD: `copied=banana` was served as
    // a recorded copy, and only the page refused to date it.
    if line.len() > MAX_RECEIPT_LINE
        || !value("copied").is_some_and(is_utc_instant)
        || value("dest").is_none()
    {
        return Err(format!(
            "{}'s newest line is an unreadable receipt, so no copy is taken as recorded: a \
             receipt is one line of at most {MAX_RECEIPT_LINE} bytes carrying \
             copied=<YYYY-MM-DDTHH:MM[:SS]Z> and dest=<where> (docs/36-ops-runbook.md); it \
             reads {:?}",
            path.display(),
            crate::fault_log::capped(line.to_owned())
        ));
    }
    Ok(line.to_owned())
}

/// Whether `word` is a UTC instant as a receipt writes it:
/// `YYYY-MM-DDTHH:MM[:SS]Z`, each field in its range. The shape the page's
/// own reader takes (`web/src/lib/disk.js`), so the two cannot disagree about
/// whether a copy is dated. A day is in ITS month's length, leap years
/// counted: `2026-02-31` is no date, and a receipt that carries it is not a
/// recorded copy (it was taken as one, and the page then dated it).
fn is_utc_instant(word: &str) -> bool {
    let bytes = word.as_bytes();
    let number = |from: usize, len: usize| -> Option<u32> {
        let digits = bytes.get(from..from + len)?;
        digits
            .iter()
            .all(u8::is_ascii_digit)
            .then(|| digits.iter().fold(0, |n, d| n * 10 + u32::from(d - b'0')))
    };
    let is = |at: usize, byte: u8| bytes.get(at) == Some(&byte);
    let seconds = match bytes.len() {
        17 => Some(0),
        20 if is(16, b':') => number(17, 2),
        _ => None,
    };
    let fields = (
        number(0, 4),
        number(5, 2),
        number(8, 2),
        number(11, 2),
        number(14, 2),
        seconds,
    );
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = fields
    else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 31,
    };
    is(4, b'-')
        && is(7, b'-')
        && is(10, b'T')
        && is(13, b':')
        && bytes.last() == Some(&b'Z')
        && (1..=12).contains(&month)
        && (1..=days).contains(&day)
        && hour < 24
        && minute < 60
        && second < 60
}

/// One feed's object in `feeds`, written to `out`; returns its kept bytes.
fn feed_json(
    out: &mut String,
    root: &Path,
    vendor: Vendor,
    files: &[CensusFile],
    site: Option<&Site>,
) -> u64 {
    let kept: u64 = files.iter().filter(|f| f.kept()).map(|f| f.bytes).sum();
    let counters = site.and_then(|site| {
        let (_stamps, now) = vendor_census(site, vendor);
        now.census().and_then(crate::census::VendorCensus::counters)
    });
    let (months, rows) = counters.map_or((None, None), |(m, r, _)| (Some(m), Some(r)));
    // THE BAR BYTES THE ROWS OCCUPY AT THE FILE'S OWN GEOMETRY: a header
    // region per month and a fixed stride per bar. A lower bound — the
    // `.crc`, `.tix`, `.prv` and `.lock` siblings are not counted, and no
    // directory is walked to count them.
    let bar_bytes = months.zip(rows).map(|(months, rows)| {
        months
            .saturating_mul(store::format::HEADER_LEN)
            .saturating_add(rows.saturating_mul(store::format::RECORD_STRIDE))
    });
    let number = |n: Option<u64>| n.map_or_else(|| "null".to_owned(), |n| n.to_string());
    let _ = write!(
        out,
        r#"{{"feed":{},"months":{},"rows":{},"bar_bytes_lower_bound":{},"census_kept_bytes":{kept},"census_files":["#,
        render::json_string(vendor.as_str()),
        number(months),
        number(rows),
        number(bar_bytes),
    );
    for (n, file) in files.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"path":{},"bytes":{},"role":{}}}"#,
            render::json_string(&file.path.display().to_string()),
            file.bytes,
            render::json_string(file.role)
        );
    }
    out.push_str(r#"],"scrub":"#);
    out.push_str(&scrub_json(root, vendor));
    out.push('}');
    kept
}

/// The whole answer, measured now.
pub(crate) fn measured(root: &Path, site: Option<&Site>) -> String {
    let mut out = String::from(r#"{"schema":1,"store":"#);
    out.push_str(&render::json_string(&root.display().to_string()));
    out.push_str(r#","filesystem":"#);
    match measure(root) {
        Ok(fs) => {
            let _ = write!(
                out,
                r#"{{"total_bytes":{},"used_bytes":{},"free_bytes":{},"mount":{},"source":"{}"}}"#,
                fs.total,
                fs.used,
                fs.free,
                render::json_string(&fs.mount),
                SOURCE
            );
        }
        Err(why) => {
            let _ = write!(out, r#"{{"error":{}}}"#, render::json_string(&why));
        }
    }
    out.push_str(r#","feeds":["#);
    let mut kept_total = 0_u64;
    let mut first = true;
    for vendor in Vendor::ALL {
        let files = census_files(root, vendor);
        if files.is_empty() {
            continue;
        }
        if !first {
            out.push(',');
        }
        first = false;
        kept_total = kept_total.saturating_add(feed_json(&mut out, root, vendor, &files, site));
    }
    let _ = write!(
        out,
        r#"],"census_kept_bytes":{kept_total},"retention":{},"backup":"#,
        render::json_string(
            "every census epoch and every superseded single file is kept and never deleted \
             (CLAUDE.md §3 rule 8); census_kept_bytes is what that costs today, and each \
             compaction adds one epoch of the partition's size (DB-B59)"
        )
    );
    match last_backup(root) {
        Ok(line) => {
            let _ = write!(
                out,
                r#"{{"recorded":true,"last":{}}}"#,
                render::json_string(&line)
            );
        }
        Err(why) => {
            let _ = write!(
                out,
                r#"{{"recorded":false,"why":{}}}"#,
                render::json_string(&why)
            );
        }
    }
    let _ = write!(
        out,
        r#","projection":null,"projection_why":{},"measured_ttl_secs":{}}}"#,
        render::json_string(
            "UNVERIFIED: this server does not know how much vendor data remains to be \
             imported, so it projects nothing; free_bytes against the runbook's growth \
             table is the operator's comparison"
        ),
        DISK_TTL.as_secs()
    );
    out
}

/// [`measured`], kept for [`DISK_TTL`] per store root.
fn answer(site: &Site) -> String {
    let root = site.store_root.clone();
    if let Some((held_root, at, body)) =
        LAST.lock().unwrap_or_else(PoisonError::into_inner).as_ref()
        && *held_root == root
        && at.elapsed() < DISK_TTL
    {
        return body.clone();
    }
    let body = measured(&root, Some(site));
    *LAST.lock().unwrap_or_else(PoisonError::into_inner) =
        Some((root, Instant::now(), body.clone()));
    body
}

/// `GET /disk.json`. See the module header.
pub(super) async fn disk_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 2],
    String,
) {
    let headers = [
        (
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        ),
        (axum::http::header::CACHE_CONTROL, "no-store"),
    ];
    match crate::detail::run_ops_read(move || answer(&site)).await {
        Ok(body) => (axum::http::StatusCode::OK, headers, body),
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                "a disk measurement",
                crate::detail::MAX_OPS_READ_CONCURRENT,
                &why,
            );
            (status, headers, body)
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;

    /// The census files of a sealed vendor carry their roles, the kept bytes
    /// are the superseded file and the older epoch, and a missing receipt
    /// log is said, not hidden (DB-B59, DB-B73).
    #[test]
    fn census_files_roles_kept_bytes_and_the_backup_receipt() {
        let root = crate::scratch::path("disk-route");
        let vendor = Vendor::Gdfl;
        let fno = segment_dir(&root, vendor, brutex_core::instrument::Segment::Fno);
        std::fs::create_dir_all(&fno).unwrap();
        std::fs::write(fno.join("000001.man"), vec![0_u8; 100]).unwrap();
        std::fs::write(fno.join("000002.man"), vec![0_u8; 300]).unwrap();
        std::fs::write(fno.join("000003.man.tmp"), vec![0_u8; 7]).unwrap();
        std::fs::write(
            vendor_dir(&root, vendor).join(MIGRATION_RECEIPT_NAME),
            b"legacy_bytes=50\n",
        )
        .unwrap();
        std::fs::write(manifest_path(&root, vendor), vec![0_u8; 50]).unwrap();
        let files = census_files(&root, vendor);
        let role = |name: &str| {
            files
                .iter()
                .find(|f| f.path.ends_with(name))
                .map(|f| (f.role, f.bytes))
        };
        assert_eq!(
            role("gdfl.man"),
            Some(("superseded", 50)),
            "unsealed: {files:?}"
        );
        assert_eq!(role("000001.man"), Some(("older-epoch", 100)));
        assert_eq!(role("000002.man"), Some(("current", 300)));
        assert_eq!(role("000003.man.tmp"), None, "a staged epoch is no epoch");
        assert_eq!(role("MIGRATED"), Some(("receipt", 16)));
        let kept: u64 = files.iter().filter(|f| f.kept()).map(|f| f.bytes).sum();
        assert_eq!(kept, 150);

        let body = measured(&root, None);
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        assert_eq!(parsed["census_kept_bytes"], 150);
        assert_eq!(parsed["backup"]["recorded"], false);
        assert!(
            parsed["backup"]["why"]
                .as_str()
                .unwrap()
                .contains("no copy of this store is recorded")
        );
        assert!(parsed["projection"].is_null());
        assert!(
            parsed["filesystem"]["free_bytes"].as_u64().is_some()
                || parsed["filesystem"]["error"].is_string(),
            "{body}"
        );

        std::fs::create_dir_all(root.join("ops")).unwrap();
        std::fs::write(
            root.join(BACKUP_RECEIPTS),
            "copied=2026-10-10T01:00:00Z dest=/Volumes/B bytes=1\n\
             copied=2026-10-11T01:00:00Z dest=/Volumes/B bytes=2\n\n",
        )
        .unwrap();
        assert_eq!(
            last_backup(&root).expect("recorded"),
            "copied=2026-10-11T01:00:00Z dest=/Volumes/B bytes=2"
        );

        // EACH FEED'S SCRUB (DB-B74): never run, said; run, its pass, its
        // totals and what still stands.
        let none: serde_json::Value =
            serde_json::from_str(&scrub_json(&root, vendor)).expect("JSON");
        assert_eq!(none["recorded"], false);
        assert!(
            none["why"]
                .as_str()
                .unwrap()
                .contains("cli scrub-blocks gdfl")
        );
        std::fs::create_dir_all(root.join("audit")).unwrap();
        std::fs::write(
            root.join(cli::scrub_ledger_name(vendor)),
            "run 1 vendor=gdfl commit=unstamped pass=1 resume=-\n\
             bad bars/gdfl/NSE/INDEX/X/1s/2020-07.bin block=4 records=292-364 run=1 why=checksum\n",
        )
        .unwrap();
        std::fs::write(
            root.join(cli::scrub_cursor_name(vendor)),
            "progress run=1 pass=1 months=9 blocks=90 bad=1 unverifiable=0 misfiled=0 unbound=9 busy=0 missing=0 cursor=bars/x.bin\n",
        )
        .unwrap();
        let some: serde_json::Value =
            serde_json::from_str(&scrub_json(&root, vendor)).expect("JSON");
        assert_eq!(some["recorded"], true);
        assert_eq!(some["state"], "in progress");
        assert_eq!(some["totals"]["bad"], 1);
        assert_eq!(some["totals"]["unbound"], 9);
        assert_eq!(some["standing_findings"], 1);
        assert_eq!(some["named"][0]["series"], "bars/gdfl/NSE/INDEX/X/1s");
        assert_eq!(some["named"][0]["finding"]["month"], "2020-07");
        assert_eq!(some["named"][0]["finding"]["block"], 4);
        // And the whole answer carries it under its feed.
        let body: serde_json::Value = serde_json::from_str(&measured(&root, None)).expect("JSON");
        assert_eq!(body["feeds"][0]["feed"], "gdfl");
        assert_eq!(body["feeds"][0]["scrub"]["totals"]["bad"], 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **ONLY A WHOLE RECEIPT, IN A REGULAR FILE, IS A RECORDED BACKUP
    /// (DB-B73).**
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one table of every shape a receipt log's last line takes"
    )]
    fn only_a_whole_receipt_in_a_regular_file_is_a_recorded_backup() {
        let root = crate::scratch::path("disk-route-receipt");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("ops")).unwrap();
        // ONLY A WHOLE RECEIPT IS A RECORDED BACKUP. Any last line used to be
        // served as "the last copy".
        let good = "copied=2026-10-11T01:00:00Z dest=/Volumes/B bytes=2\n";
        for (log, said) in [
            (
                format!("{good}rsync: write failed: No space left on device\n"),
                "unreadable receipt",
            ),
            (
                format!("{good}copied=2026-10-12T01:00:00Z\n"),
                "unreadable receipt",
            ),
            (
                format!("{good}copied= dest=/Volumes/B\n"),
                "unreadable receipt",
            ),
            (format!("{good}dest=/Volumes/B\n"), "unreadable receipt"),
            // `copied=` THAT IS NO UTC INSTANT IS NO RECEIPT: these were
            // answered `recorded: true`.
            (
                format!("{good}copied=banana dest=/Volumes/B\n"),
                "unreadable receipt",
            ),
            (
                format!("{good}copied=2026-13-01T01:00:00Z dest=/Volumes/B\n"),
                "unreadable receipt",
            ),
            (
                format!("{good}copied=2026-10-12T24:00:00Z dest=/Volumes/B\n"),
                "unreadable receipt",
            ),
            (
                format!("{good}copied=2026-10-12T01:00:00 dest=/Volumes/B\n"),
                "unreadable receipt",
            ),
            (
                format!("{good}copied=2026-10-12 dest=/Volumes/B\n"),
                "unreadable receipt",
            ),
            (
                format!(
                    "{good}copied=x dest=/Volumes/B pad={}\n",
                    "9".repeat(MAX_RECEIPT_LINE)
                ),
                "unreadable receipt",
            ),
            (
                "copied=2026-10-12T01:00:00Z dest=/Volumes/B".to_owned(),
                "no whole receipt line",
            ),
            ("\n\n".to_owned(), "no whole receipt line"),
            (String::new(), "no whole receipt line"),
        ] {
            std::fs::write(root.join(BACKUP_RECEIPTS), &log).unwrap();
            let why = last_backup(&root).expect_err("no receipt");
            assert!(
                why.contains(said) && why.contains("ops/backups.log"),
                "{log:?}: {why}"
            );
            assert!(why.len() < 1_200, "the line is capped: {}", why.len());
        }
        // A line torn after a whole receipt does not hide the receipt, and is
        // not read as one.
        std::fs::write(
            root.join(BACKUP_RECEIPTS),
            format!("{good}copied=2026-10-13T01:00:00Z dest=/Volumes/C"),
        )
        .unwrap();
        assert_eq!(last_backup(&root).expect("the whole one"), good.trim_end());
        // MINUTES WITHOUT SECONDS is the other shape the page reads.
        for word in [
            "2026-10-13T01:00Z",
            "2026-02-28T23:59:59Z",
            "0001-01-01T00:00:00Z",
            // The last day of each month length, and of a leap February.
            "2026-01-31T00:00Z",
            "2026-04-30T00:00Z",
            "2024-02-29T00:00Z",
            "2000-02-29T00:00Z",
        ] {
            assert!(is_utc_instant(word), "{word}");
        }
        for word in [
            "",
            "Z",
            "2026-10-13T01:00:0Z",
            "2026-10-13t01:00:00Z",
            "2026-00-13T01:00Z",
            // A DAY ITS MONTH DOES NOT HAVE is no date: `2026-02-31` was
            // taken as a recorded copy and the page dated it.
            "2026-02-31T01:00:00Z",
            "2026-02-29T01:00:00Z",
            "1900-02-29T01:00:00Z",
            "2026-04-31T01:00Z",
            "2026-01-32T01:00Z",
            "2026-01-00T01:00Z",
        ] {
            assert!(!is_utc_instant(word), "{word}");
        }
        // ... and a receipt that carries one is not a recorded backup.
        std::fs::write(
            root.join(BACKUP_RECEIPTS),
            "copied=2026-02-31T01:00:00Z dest=/Volumes/B bytes=2\n",
        )
        .unwrap();
        assert!(last_backup(&root).is_err(), "no such day");
        std::fs::write(root.join(BACKUP_RECEIPTS), good).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&measured(&root, None)).expect("JSON");
        assert_eq!(parsed["backup"]["recorded"], true);
        assert_eq!(parsed["backup"]["last"], good.trim_end());

        // A LOG THAT IS NO REGULAR FILE IS REFUSED AT ONCE, BY NAME. A FIFO
        // there was opened with a plain blocking open: /disk.json never
        // answered and each request kept a store-read slot until every bar
        // seek answered 429.
        for kind in ["fifo", "symlink", "directory"] {
            let at = root.join(BACKUP_RECEIPTS);
            let _ = std::fs::remove_file(&at);
            match kind {
                "fifo" => {
                    // `mkfifo` is the external program, run from a test only.
                    let made = std::process::Command::new("mkfifo")
                        .arg(&at)
                        .status()
                        .expect("mkfifo ran");
                    assert!(made.success());
                }
                "symlink" => {
                    let target = root.join("ops/elsewhere.log");
                    std::fs::write(&target, good).unwrap();
                    std::os::unix::fs::symlink(&target, &at).unwrap();
                }
                _ => std::fs::create_dir(&at).unwrap(),
            }
            let asked = Instant::now();
            let why = last_backup(&root).expect_err("refused");
            assert!(asked.elapsed() < Duration::from_secs(5), "{kind}: no wait");
            assert!(
                why.contains("ops/backups.log") && why.contains("cannot be told"),
                "{kind}: {why}"
            );
            assert!(
                !why.contains("no copy of this store is recorded"),
                "{kind}: an unreadable log is not the absence of one: {why}"
            );
            let parsed: serde_json::Value =
                serde_json::from_str(&measured(&root, None)).expect("JSON");
            assert_eq!(parsed["backup"]["recorded"], false, "{kind}");
            if kind == "directory" {
                std::fs::remove_dir(&at).unwrap();
            } else {
                std::fs::remove_file(&at).unwrap();
            }
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The counts the filesystem answers for `dir`, read apart from
    /// [`measure`] through the same call it makes: fragment size, fragments,
    /// free fragments, and fragments an unprivileged writer may take. On
    /// macOS that call is `statfs(2)` (D-5241).
    #[cfg(target_vendor = "apple")]
    fn counts_of(dir: &Path) -> (u64, u64, u64, u64) {
        let fs = rustix::fs::statfs(dir).expect("statfs");
        (u64::from(fs.f_bsize), fs.f_blocks, fs.f_bfree, fs.f_bavail)
    }

    /// As above; off macOS the call is `statvfs(2)`, in `f_frsize` fragments.
    #[cfg(not(target_vendor = "apple"))]
    fn counts_of(dir: &Path) -> (u64, u64, u64, u64) {
        let vfs = rustix::fs::statvfs(dir).expect("statvfs");
        (vfs.f_frsize, vfs.f_blocks, vfs.f_bfree, vfs.f_bavail)
    }

    /// The filesystem id `statvfs(2)` answers for `dir`: a second call,
    /// independent of the one [`measure`] reads on macOS, that says whether
    /// two paths are on one filesystem.
    fn fsid_of(dir: &Path) -> u64 {
        rustix::fs::statvfs(dir).expect("statvfs").f_fsid
    }

    /// **THE NUMBERS ARE WHAT THE FILESYSTEM RETURNS (D-5240, D-5241)**, on a
    /// temporary directory: total is fragments, used is fragments less free
    /// ones, free is what an unprivileged writer may take, each times the
    /// fragment size, from `statfs(2)` on macOS and `statvfs(2)` elsewhere; the
    /// mount point is on the directory's own filesystem, and off macOS is the
    /// highest directory above it that still is.
    #[test]
    fn the_filesystem_numbers_are_what_the_call_returns() {
        let dir = crate::scratch::path("disk-route-statvfs");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // ONLY A QUIET READ IS COMPARED, AND A RACE IS RETRIED: another process
        // may write between two reads, so a round counts when the counts read
        // on each side of it agree AND the measurement equals them. A round
        // can be quiet on both sides and still race — on the operator's APFS
        // boot volume on 17 Oct 2026 one round's `used` was one fragment off,
        // the free count having moved and moved back while `measure` read —
        // so a quiet round that disagrees is retried rather than failed. A
        // wrong formula disagrees on every round, so it cannot reach three;
        // the formula on counts where free and available differ is pinned by
        // `counts_no_filesystem_can_have_are_refused_by_name`.
        let mut agreed = 0;
        for _ in 0..200 {
            let before = counts_of(&dir);
            let fs = measure(&dir).expect("measured");
            if counts_of(&dir) != before {
                continue;
            }
            let (fragment, blocks, free, available) = before;
            if (fs.total, fs.used, fs.free)
                == (
                    blocks * fragment,
                    (blocks - free) * fragment,
                    available * fragment,
                )
            {
                agreed += 1;
                if agreed == 3 {
                    break;
                }
            }
        }
        assert_eq!(
            agreed, 3,
            "no three quiet reads agreed with the measurement"
        );

        let fs = measure(&dir).expect("measured");
        let mount = Path::new(&fs.mount);
        assert!(mount.is_absolute(), "{}", fs.mount);
        assert_eq!(
            fsid_of(mount),
            fsid_of(&dir),
            "{} is not on the filesystem of {}",
            fs.mount,
            dir.display()
        );
        #[cfg(not(target_vendor = "apple"))]
        {
            use std::os::unix::fs::MetadataExt as _;
            let device = std::fs::metadata(&dir).unwrap().dev();
            assert!(
                std::fs::canonicalize(&dir).unwrap().starts_with(mount),
                "{} is not above {}",
                fs.mount,
                dir.display()
            );
            assert_eq!(std::fs::metadata(mount).unwrap().dev(), device);
            if let Some(parent) = mount.parent() {
                assert_ne!(
                    std::fs::metadata(parent).unwrap().dev(),
                    device,
                    "{} is not the highest directory on its device",
                    fs.mount
                );
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **COUNTS NO FILESYSTEM CAN HAVE ARE REFUSED BY NAME (D-5241)**, never
    /// turned into a number: a fragment of 0 bytes, more free or more
    /// available fragments than there are, and more bytes than 64 bits hold.
    /// Each refusal names the root, the impossibility and the counts. Before
    /// D-5241 each of these was a `saturating_` call that answered 0 bytes, a
    /// used figure clamped to 0, or `u64::MAX`, as if measured.
    #[test]
    fn counts_no_filesystem_can_have_are_refused_by_name() {
        let root = Path::new("/invented/store-root");
        let counts = |fragment, blocks, free, available| Counts {
            fragment,
            blocks,
            free,
            available,
        };
        // An ordinary answer is exact, and the edges that ARE possible pass:
        // a full filesystem, an empty one, and the largest that fits.
        assert_eq!(
            counts(4_096, 1_000, 300, 250).in_bytes(root),
            Ok((4_096_000, 2_867_200, 1_024_000))
        );
        assert_eq!(counts(512, 10, 0, 0).in_bytes(root), Ok((5_120, 5_120, 0)));
        assert_eq!(
            counts(4_096, 10, 10, 10).in_bytes(root),
            Ok((40_960, 0, 40_960))
        );
        let most = u64::MAX / 4_096;
        assert_eq!(
            counts(4_096, most, 0, 0).in_bytes(root),
            Ok((most * 4_096, most * 4_096, 0))
        );
        for (bad, says) in [
            (counts(0, 1_000, 300, 250), "a fragment of 0 bytes"),
            (
                counts(4_096, 1_000, 1_001, 250),
                "more free fragments than it has",
            ),
            (
                counts(4_096, 1_000, 300, 1_001),
                "more available fragments than it has",
            ),
            (
                counts(4_096, most + 1, 0, 0),
                "more bytes than 64 bits hold",
            ),
            (counts(u64::MAX, 2, 0, 0), "more bytes than 64 bits hold"),
        ] {
            let why = bad.in_bytes(root).expect_err("refused");
            assert!(
                why.starts_with(
                    "/invented/store-root cannot be measured: its filesystem answered "
                ) && why.contains(says)
                    && why.contains(&format!("{bad:?}")),
                "{why}"
            );
        }
    }

    /// `/bin/df -Pk <dir>` as the route ran it before D-5240: total, used and
    /// available in bytes, and the mount point. Test code may start a program;
    /// shipped code may not (`crates/vocab/tests/no_runtime_spawn.rs`). `-P`
    /// is POSIX's portable format, the same columns from BSD and GNU `df`.
    fn df_of(dir: &Path) -> (u64, u64, u64, String) {
        let out = std::process::Command::new("/bin/df")
            .arg("-Pk")
            .arg(dir)
            .output()
            .expect("/bin/df runs");
        assert!(out.status.success(), "{out:?}");
        let text = String::from_utf8(out.stdout).expect("UTF-8");
        let line = text.lines().nth(1).expect("a filesystem line");
        let words: Vec<&str> = line.split_whitespace().collect();
        let at = words
            .iter()
            .rposition(|word| word.ends_with('%'))
            .expect("a capacity column");
        let kib = |offset: usize| words[at - offset].parse::<u64>().expect("a count") * 1_024;
        (kib(3), kib(2), kib(1), words[at + 1..].join(" "))
    }

    /// The filesystem type `statfs(2)` names for `dir`, such as `apfs`.
    #[cfg(target_vendor = "apple")]
    fn fstype_of(dir: &Path) -> String {
        let fs = rustix::fs::statfs(dir).expect("statfs");
        let name: Vec<u8> = fs
            .f_fstypename
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| u8::from_ne_bytes(c.to_ne_bytes()))
            .collect();
        String::from_utf8(name).expect("UTF-8")
    }

    /// **ON macOS THE TOTAL, THE FREE BYTES AND THE MOUNT ARE WHAT `/bin/df
    /// -Pk` PRINTED, AND `used_bytes` IS NAMED FOR WHAT IT IS (D-5241)**, on
    /// two volumes where they exist: the temporary directory (on the boot
    /// disk, APFS, behind the `/private` firmlink: `stat` gives `/`,
    /// `/private` and `/System/Volumes/Data` one device on the operator's
    /// machine, so a walk upward would name `/` where `df` names
    /// `/System/Volumes/Data`) and this crate's own directory (on the
    /// operator's machine, the external drive).
    ///
    /// `used_bytes` is `f_blocks − f_bfree`. On a filesystem that is not APFS
    /// that is `df`'s Used column too, and is held equal. ON APFS IT IS NOT:
    /// several volumes share one container, `f_blocks` and `f_bfree` are the
    /// container's, and `df` prints the volume's own allocation there, which
    /// no field of `statfs(2)` carries (measured on 17 Oct 2026, boot volume:
    /// 425,294,667,776 bytes against `df`'s 334,431,264,768; how `df` reads
    /// it was not read, UNVERIFIED). There `used_bytes` is held at least
    /// `df`'s figure, and each pair is printed. Only a quiet round counts, and
    /// a quiet round whose total or free bytes still moved is retried, as in
    /// `the_filesystem_numbers_are_what_the_call_returns`; the mount never
    /// moves and is held equal on every quiet round.
    #[cfg(target_vendor = "apple")]
    #[test]
    fn on_macos_total_free_and_mount_are_what_bin_df_printed() {
        let dir = crate::scratch::path("disk-route-df");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for root in [dir.as_path(), manifest.as_path()] {
            let (fragment, ..) = counts_of(root);
            assert_eq!(
                fragment % 1_024,
                0,
                "df -Pk rounds a {fragment}-byte fragment"
            );
            let apfs = fstype_of(root) == "apfs";
            let mut agreed = 0;
            for _ in 0..200 {
                let before = counts_of(root);
                let (total, used, free, mount) = df_of(root);
                let fs = measure(root).expect("measured");
                if counts_of(root) != before {
                    continue;
                }
                assert_eq!(fs.mount, mount, "{}", root.display());
                if (fs.total, fs.free) != (total, free) {
                    continue;
                }
                if apfs {
                    assert!(
                        fs.used >= used,
                        "{}: used_bytes {} is below df's {used}",
                        root.display(),
                        fs.used
                    );
                } else {
                    assert_eq!(fs.used, used, "{}", root.display());
                }
                println!(
                    "RUST1 df root={} apfs={apfs} total={total} free={free} mount={mount} \
                     used_bytes={} df_used={used}",
                    root.display(),
                    fs.used
                );
                agreed += 1;
                if agreed == 3 {
                    break;
                }
            }
            assert_eq!(
                agreed,
                3,
                "{}: no three quiet rounds agreed with df",
                root.display()
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **A MISSING STORE ROOT IS REFUSED BY NAME (D-5240)**, by `measure` and
    /// as the whole answer's `filesystem.error`, never measured as the
    /// filesystem around it.
    #[test]
    fn a_missing_root_is_refused_by_name() {
        let dir = crate::scratch::path("disk-route-missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let absent = dir.join("absent");
        let named = absent.display().to_string();
        let why = measure(&absent).expect_err("refused");
        assert!(
            why.contains(&named) && why.contains("does not exist"),
            "{why}"
        );
        let parsed: serde_json::Value =
            serde_json::from_str(&measured(&absent, None)).expect("JSON");
        assert_eq!(parsed["filesystem"]["error"], why.as_str());
        assert!(parsed["filesystem"].get("free_bytes").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **A STORE ROOT THAT IS NO DIRECTORY IS REFUSED (D-5240)**: a regular
    /// file is not measured as the filesystem it sits on.
    #[test]
    fn a_root_that_is_no_directory_is_refused() {
        let dir = crate::scratch::path("disk-route-file");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("store-root-that-is-a-file");
        std::fs::write(&file, b"x").unwrap();
        let named = file.display().to_string();
        let why = measure(&file).expect_err("refused");
        assert!(
            why.contains(&named) && why.contains("is not a directory"),
            "{why}"
        );
        let parsed: serde_json::Value = serde_json::from_str(&measured(&file, None)).expect("JSON");
        assert_eq!(parsed["filesystem"]["error"], why.as_str());
        // A path THROUGH a file is refused with the call's own error.
        let below = file.join("below");
        let why = measure(&below).expect_err("refused");
        assert!(
            why.contains(&below.display().to_string()) && why.contains("cannot be measured"),
            "{why}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **THE ROUTE'S JSON SHAPE IS UNCHANGED (D-5240)**: the same top-level
    /// keys, and the filesystem object's five in the same order with the same
    /// types. Only `source` changes, to name the call now made: `statfs` on
    /// macOS, `statvfs` elsewhere.
    #[test]
    fn the_disk_json_shape_is_unchanged() {
        let root = crate::scratch::path("disk-route-shape");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let body = measured(&root, None);
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("JSON");
        let keys = |value: &serde_json::Value| {
            let mut keys: Vec<String> =
                value.as_object().expect("object").keys().cloned().collect();
            keys.sort();
            keys
        };
        assert_eq!(
            keys(&parsed),
            [
                "backup",
                "census_kept_bytes",
                "feeds",
                "filesystem",
                "measured_ttl_secs",
                "projection",
                "projection_why",
                "retention",
                "schema",
                "store"
            ]
        );
        assert!(body.starts_with(r#"{"schema":1,"store":"#), "{body}");
        let fs = &parsed["filesystem"];
        assert_eq!(
            keys(fs),
            ["free_bytes", "mount", "source", "total_bytes", "used_bytes"]
        );
        let number = |key: &str| fs[key].as_u64().unwrap_or_else(|| panic!("{key}: {fs}"));
        let mount = fs["mount"].as_str().expect("mount");
        let text = format!(
            r#""filesystem":{{"total_bytes":{},"used_bytes":{},"free_bytes":{},"mount":{},"source":"{}"}},"feeds":["#,
            number("total_bytes"),
            number("used_bytes"),
            number("free_bytes"),
            render::json_string(mount),
            SOURCE
        );
        assert!(body.contains(&text), "{text}\nis not in\n{body}");
        // `source` names the call this platform makes, spelled out here rather
        // than read back from the constant the route writes (D-5241).
        #[cfg(target_vendor = "apple")]
        assert_eq!(fs["source"], "statfs");
        #[cfg(not(target_vendor = "apple"))]
        assert_eq!(fs["source"], "statvfs");
        assert!(number("used_bytes") <= number("total_bytes"));
        assert!(number("free_bytes") <= number("total_bytes"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// `(p50, p99, max)` of `ns` by nearest rank, in nanoseconds.
    fn ranks(mut ns: Vec<u128>) -> (u128, u128, u128) {
        ns.sort_unstable();
        let n = ns.len();
        let rank = |p: usize| ns[(n * p).div_ceil(100) - 1];
        (rank(50), rank(99), ns[n - 1])
    }

    /// **THE CALL AT ITS p99 COSTS LESS THAN THE `df` CHILD IT REPLACED AT
    /// ITS p50 (D-5242)**, measured, not argued: on the temporary directory
    /// and on this crate's own directory (on the operator's machine, the
    /// external drive), 50 warm-up rounds and then n = 1,000 interleaved
    /// rounds of the child (`/bin/df -Pk`, started here as the route started
    /// it before D-5240), [`measure`] and the whole uncached [`measured`];
    /// then the `GET /disk.json` handler through the operations pool, its
    /// cache cleared before every call (a miss) and warm (a hit), n = 1,000
    /// each after 50 warm-up calls. Every figure is printed as one `RUST1`
    /// line (p50, p99, max by nearest rank). A measurement, so it is run by
    /// hand: `cargo test -p api --lib -- --ignored --nocapture
    /// --test-threads=1 disk_route::tests::the_call_at_its_p99`.
    #[test]
    #[ignore = "a timing measurement, run by hand on one test thread"]
    fn the_call_at_its_p99_costs_less_than_the_df_child_at_its_p50() {
        let tmp = crate::scratch::path("disk-route-timing");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("masters")).unwrap();
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for (name, root) in [("tmp", tmp.as_path()), ("crate-dir", manifest.as_path())] {
            for _ in 0..50 {
                let _ = std::hint::black_box((df_of(root), measure(root), measured(root, None)));
            }
            let (mut child, mut call, mut whole) = (Vec::new(), Vec::new(), Vec::new());
            for _ in 0..1_000 {
                let t = Instant::now();
                let _ = std::hint::black_box(df_of(root));
                child.push(t.elapsed().as_nanos());
                let t = Instant::now();
                let _ = std::hint::black_box(measure(root).expect("measured"));
                call.push(t.elapsed().as_nanos());
                let t = Instant::now();
                let _ = std::hint::black_box(measured(root, None));
                whole.push(t.elapsed().as_nanos());
            }
            let (child, call, whole) = (ranks(child), ranks(call), ranks(whole));
            for (what, (p50, p99, max)) in [
                ("df-child", child),
                ("measure", call),
                ("measured-uncached", whole),
            ] {
                println!(
                    "RUST1 root={name} {what} n=1000 p50_ns={p50} p99_ns={p99} max_ns={max} ({})",
                    root.display()
                );
            }
            assert!(
                call.1 < child.0,
                "{name}: {SOURCE} p99 {} ns is not below the df child's p50 {} ns",
                call.1,
                child.0
            );
        }
        let site = Loaded::new(Site::load(&tmp.join("masters"), &tmp));
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();
        for (what, miss) in [("route-miss", true), ("route-hit", false)] {
            let mut ns = Vec::with_capacity(1_000);
            for k in 0..1_050 {
                if miss {
                    *LAST.lock().unwrap_or_else(PoisonError::into_inner) = None;
                }
                let t = Instant::now();
                let (status, _, body) =
                    runtime.block_on(disk_json(axum::extract::State(Loaded::clone(&site))));
                let took = t.elapsed().as_nanos();
                assert_eq!(status, axum::http::StatusCode::OK, "{body}");
                let _ = std::hint::black_box(body);
                if k >= 50 {
                    ns.push(took);
                }
            }
            let (p50, p99, max) = ranks(ns);
            println!("RUST1 {what} n=1000 p50_ns={p50} p99_ns={p99} max_ns={max}");
        }
        *LAST.lock().unwrap_or_else(PoisonError::into_inner) = None;
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
