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
//!   root is on, from POSIX `df -Pk` (1024-byte blocks), or the refusal.
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
//!   findings still stand and the first of them by name — or `"recorded":
//!   false` with the reason when no scrub of that feed ever ran. The series a
//!   page shows asks `/scrub.json` for its own findings.
//! * `backup` — the last line of `<store>/ops/backups.log`, the receipt the
//!   runbook's copy procedure appends (`docs/36-ops-runbook.md`), or
//!   `"recorded": false` said loudly. This build cannot see a backup it was
//!   not told about, and says so rather than guess.
//!
//! # Cost
//!
//! One `df` child process, one `lstat` per census file and one directory
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

/// One filesystem, as `df -Pk` reports it, in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Filesystem {
    pub(crate) total: u64,
    pub(crate) used: u64,
    pub(crate) free: u64,
    pub(crate) mount: String,
}

/// The filesystem in POSIX `df -Pk` output: a header line, then one line
/// `<name> <1024-blocks> <used> <available> <capacity%> <mounted on>`. The
/// capacity column is found by its `%`, so a mount point with spaces is kept
/// whole.
///
/// # Errors
///
/// The sentence naming what in the output could not be read.
pub(crate) fn parse_df(text: &str) -> Result<Filesystem, String> {
    let line = text
        .lines()
        .rfind(|line| !line.trim().is_empty())
        .ok_or_else(|| "df printed nothing".to_owned())?;
    let words: Vec<&str> = line.split_whitespace().collect();
    let at = words
        .iter()
        .rposition(|word| word.ends_with('%'))
        .filter(|at| *at >= 4)
        .ok_or_else(|| format!("df line {line:?} has no capacity column"))?;
    let kib = |offset: usize| -> Result<u64, String> {
        words
            .get(at - offset)
            .and_then(|word| word.parse::<u64>().ok())
            .map(|blocks| blocks.saturating_mul(1024))
            .ok_or_else(|| format!("df line {line:?} has no number {offset} before its capacity"))
    };
    Ok(Filesystem {
        total: kib(3)?,
        used: kib(2)?,
        free: kib(1)?,
        mount: words
            .get(at + 1..)
            .map(|rest| rest.join(" "))
            .unwrap_or_default(),
    })
}

/// The filesystem `root` is on, measured now.
fn measure(root: &Path) -> Result<Filesystem, String> {
    let output = std::process::Command::new("/bin/df")
        .arg("-Pk")
        .arg(root)
        .output()
        .map_err(|why| format!("/bin/df could not be run: {why}"))?;
    if !output.status.success() {
        return Err(format!(
            "/bin/df -Pk {} answered {}: {}",
            root.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_df(&String::from_utf8_lossy(&output.stdout))
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
            r#"{{"ledger":{ledger},"recorded":true,"pass":{},"state":{},"totals":{},"standing_from_pass":{from_pass},"standing_findings":{},"truncated":{},"named":["#,
            state.pass,
            render::json_string(if state.complete {
                "complete"
            } else {
                "in progress"
            }),
            state.totals_json(),
            found.standing_count(from_pass),
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
/// whether a copy is dated. A day past its month's length is not caught here
/// (31 for every month): a receipt is written by `date -u`, and this refuses
/// what is no date at all.
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
    let (Some(_), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = fields else {
        return false;
    };
    is(4, b'-')
        && is(7, b'-')
        && is(10, b'T')
        && is(13, b':')
        && bytes.last() == Some(&b'Z')
        && (1..=12).contains(&month)
        && (1..=31).contains(&day)
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
                r#"{{"total_bytes":{},"used_bytes":{},"free_bytes":{},"mount":{},"source":"df -Pk"}}"#,
                fs.total,
                fs.used,
                fs.free,
                render::json_string(&fs.mount)
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

    /// `df -Pk` as macOS and Linux print it, a mount with a space, and the
    /// refusals.
    #[test]
    fn df_output_is_read_by_its_capacity_column() {
        let mac = "Filesystem     1024-blocks       Used  Available Capacity  Mounted on\n\
                   /dev/disk7s1    7811891200 3620421632 4191469568    47%    /Volumes/WD_BLACK\n";
        let fs = parse_df(mac).expect("read");
        assert_eq!(fs.total, 7_811_891_200 * 1024);
        assert_eq!(fs.used, 3_620_421_632 * 1024);
        assert_eq!(fs.free, 4_191_469_568 * 1024);
        assert_eq!(fs.mount, "/Volumes/WD_BLACK");
        let spaced = "Filesystem 1024-blocks Used Available Capacity Mounted on\n\
                      /dev/x 100 40 60 40% /Volumes/My Disk\n";
        assert_eq!(parse_df(spaced).expect("read").mount, "/Volumes/My Disk");
        assert!(parse_df("").is_err());
        assert!(parse_df("Filesystem\n/dev/x 1 2 3 /m\n").is_err());
        assert!(parse_df("h\n/dev/x a 2 3 4% /m\n").is_err());
    }

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
        ] {
            assert!(is_utc_instant(word), "{word}");
        }
        for word in [
            "",
            "Z",
            "2026-10-13T01:00:0Z",
            "2026-10-13t01:00:00Z",
            "2026-00-13T01:00Z",
        ] {
            assert!(!is_utc_instant(word), "{word}");
        }
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
}
