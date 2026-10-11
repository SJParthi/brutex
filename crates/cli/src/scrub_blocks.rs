//! `cli scrub-blocks VENDOR [--months N] [--mib-per-s N]` — every committed
//! block of every month the census holds, read whole and checked against its
//! stored checksum, resumably, rate-limited, with a durable ledger of what
//! would not verify (DB-B74, DB-B65; WP-81).
//!
//! # Why
//!
//! `/verify.json`'s scrub (`pull::scrub::one`) reads each month's first and
//! last record. One flipped byte in the middle of a month left it answering
//! "agreed"; the 73 bars of that block were refused only to whoever read that
//! exact block, the event went to the log with no file named, and nothing kept
//! it. This walks the census's months in path order and reads every block
//! (`pull::scrub::blocks`), so a rotted block is found by a pass. The same
//! walk opens every month through the store's own reader door, so a month
//! filed under another name is met and named too
//! (`store::file::StoreError::Misfiled`, DB-B65), and it counts the months no
//! provenance sidecar binds to their path.
//!
//! # Two files per vendor under `<store>/audit/`, both append-only
//!
//! **The ledger, `scrub-blocks.<vendor>.log`** — what was found:
//!
//! ```text
//! run <id> vendor=<v> commit=<sha|unstamped> pass=<n> resume=<path|->
//! bad <path> block=<b> records=<first>-<last> run=<id> why=<sentence>
//! unverifiable <path> run=<id> why=<sentence>
//! misfiled <path> run=<id> why=<sentence>
//! busy <path> run=<id>
//! missing <path> run=<id>
//! ```
//!
//! **The cursor, `scrub-blocks.<vendor>.cursor`** — where the pass stands,
//! and its totals, one short line each:
//!
//! ```text
//! progress run=<id> pass=<n> months=<n> blocks=<n> bad=<n> unverifiable=<n> misfiled=<n> unbound=<n> busy=<n> missing=<n> cursor=<path|->
//! done run=<id> pass=<n> months=<n> blocks=<n> bad=<n> unverifiable=<n> misfiled=<n> unbound=<n> busy=<n> missing=<n>
//! ```
//!
//! `<path>` is the month's `.bin` relative to the store root; `<id>` is the
//! run's start in Unix seconds, or one more than the newest id either file
//! holds when that is not earlier, so no two runs of a vendor share an id
//! whatever the clock says. They are two files because they are read
//! differently: a run resumes from, and the API serves the totals of, the
//! cursor's LAST LINE, which a bounded tail read always holds whatever the
//! pass found — one file for both lost the cursor behind 64 KiB of findings
//! and restarted a pass in silence. And they are per vendor because one
//! shared file resumed a Zerodha run from GDFL's cursor.
//!
//! A line counts once its newline is written. A run killed, or out of disk,
//! in the middle of an append leaves a tail no newline ends; the next run
//! ENDS that tail before it appends anything, as a void line (the fragment,
//! then [`VOID`], then the newline), in the cursor and in the ledger, and
//! records it as a `torn <file> bytes=<n> run=<id>` ledger line. Without that
//! the next line was joined onto the fragment and every reader took the
//! fragment's `months=`, `bad=` and `cursor=` for the pass's: totals nobody
//! counted, and a resume from a path no run reached, skipping every month
//! between in a pass that still ended `done`. No reader reads a void line,
//! and a cursor line is read STRICTLY ([`cursor_line`]): exactly the words
//! this module writes, in its order, each a number — one parser, used by the
//! resume here and by the API's `/scrub.json` and `/disk.json`, so a line two
//! writers' bytes were joined into is no line at all.
//!
//! A run killed between two `progress` lines re-reads the months since the
//! last one, so a `bad` line can appear twice under two run ids; the totals
//! are carried by the cursor and count once. A cursor file that holds bytes
//! and no line this build can read is REFUSED by name: where to resume cannot
//! be told, and starting again in silence is the failure this replaces.
//!
//! Both files must be plain files: a symlink, a FIFO or anything else at
//! either name is refused by name before a byte is read or written.
//!
//! # One run at a time per vendor
//!
//! The run holds an exclusive lock on the cursor file. A second run of the
//! same vendor is refused by name at once; two vendors run side by side.
//!
//! # What it does to a store that is being written
//!
//! Each month is opened through `BarFile::open_existing`: read-only, under
//! the month's SHARED lock, exactly as the API reads it. A month a writer
//! holds is a `busy` line, counted and skipped, never waited for. While this
//! reads a month a writer of that month is refused by the store
//! (`StoreError::Locked`) — for as long as that month's blocks take to READ,
//! and no longer: what the rate limit owes is slept after the month is
//! closed, with no month open (rules audit RA-128; it used to sleep with the
//! month held, so a low `--mib-per-s` held one month for bytes / rate). The
//! hold is O(blocks of one month) and is not timed here on the operator's
//! drive (NOT MEASURED), so a scrub and an import of one vendor can still
//! meet on a month: the scrub counts it `busy`, the importer sees `Locked`
//! and retries; `docs/36-ops-runbook.md` §6.
//!
//! # Cost
//!
//! One census read and one sort of its month paths per run (O(months), a
//! second or two and a few hundred MB at the full options import — an
//! extrapolation from 385,442 months, not a measurement), then O(blocks) per
//! month, every block once per pass: about 437 GB of bars on the operator's
//! store, so a pass is hours at the default 64 MiB/s (extrapolated from the
//! rate, not measured). Bounded per run by `--months`. It writes only its
//! two files: no bar, sidecar or census byte. Those two are served as
//! findings about the store, so the verb is a store writer to `store_write`:
//! a build with no commit stamp is refused before it reads a block.

use std::fmt::Write as _;
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use brutex_core::vendor::Vendor;
use pull::ingest::census_store;
use pull::manifest::Entry;
use store::file::{BarFile, StoreError};
use store::flock::Flock;
use store::path::{FileKind, PathParts, StorePath};

use crate::{FAILED, MISUSED, OK};

/// The directory both files live in, relative to the store root.
pub(crate) const DIR: &str = "audit";

/// Blocks read between two rate checks.
const CHUNK_BLOCKS: u64 = 1_024;

/// Months between two `progress` lines, at most.
const PROGRESS_MONTHS: u64 = 256;

/// Time between two `progress` lines, at most.
const PROGRESS_EVERY: Duration = Duration::from_secs(2);

/// The default read rate, in MiB per second.
const DEFAULT_MIB_PER_S: u64 = 64;

/// The most cursor bytes read to find where to resume. A cursor line is a
/// store path of under 300 bytes and eleven small fields, so this holds
/// several whole lines whatever came before them.
pub(crate) const CURSOR_TAIL_BYTES: u64 = 8 * 1024;

/// Bytes one block read costs: a covered range of at most 73 × 56 bytes and a
/// 4-byte sidecar entry.
const BLOCK_BYTES: u64 = 4_088 + 4;

/// The findings ledger of `vendor`, relative to the store root.
#[must_use]
pub fn ledger_name(vendor: Vendor) -> String {
    format!("{DIR}/scrub-blocks.{}.log", vendor.as_str())
}

/// The cursor file of `vendor`, relative to the store root.
#[must_use]
pub fn cursor_name(vendor: Vendor) -> String {
    format!("{DIR}/scrub-blocks.{}.cursor", vendor.as_str())
}

/// One pass's running totals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Totals {
    /// Months met: read, busy, missing or refused.
    pub(crate) months: u64,
    /// Blocks read whole.
    pub(crate) blocks: u64,
    /// Blocks that did not match their stored checksum.
    pub(crate) bad: u64,
    /// Months that cannot be verified at all, or would not open.
    pub(crate) unverifiable: u64,
    /// Months whose provenance sidecar names another month (DB-B65).
    pub(crate) misfiled: u64,
    /// Months no provenance sidecar binds to their path: written before
    /// D-3334, their only on-disk identity is the symbol's hash (DB-B65).
    pub(crate) unbound: u64,
    /// Months a writer held, skipped.
    pub(crate) busy: u64,
    /// Months the census holds and the disk does not.
    pub(crate) missing: u64,
}

impl Totals {
    /// The eight totals in the order a cursor line writes them
    /// ([`TOTAL_NAMES`]).
    const fn array(self) -> [u64; 8] {
        [
            self.months,
            self.blocks,
            self.bad,
            self.unverifiable,
            self.misfiled,
            self.unbound,
            self.busy,
            self.missing,
        ]
    }

    /// The totals of a cursor line's eight numbers.
    const fn of(
        [
            months,
            blocks,
            bad,
            unverifiable,
            misfiled,
            unbound,
            busy,
            missing,
        ]: [u64; 8],
    ) -> Self {
        Self {
            months,
            blocks,
            bad,
            unverifiable,
            misfiled,
            unbound,
            busy,
            missing,
        }
    }

    /// The totals as a cursor line spells them: [`TOTAL_NAMES`]' order, the
    /// one order [`cursor_line`] reads.
    fn words(self) -> String {
        let mut out = String::new();
        for (name, value) in TOTAL_NAMES.iter().zip(self.array()) {
            if !out.is_empty() {
                out.push(' ');
            }
            let _ = write!(out, "{name}={value}");
        }
        out
    }

    fn plus(self, other: Self) -> Self {
        Self {
            months: self.months.saturating_add(other.months),
            blocks: self.blocks.saturating_add(other.blocks),
            bad: self.bad.saturating_add(other.bad),
            unverifiable: self.unverifiable.saturating_add(other.unverifiable),
            misfiled: self.misfiled.saturating_add(other.misfiled),
            unbound: self.unbound.saturating_add(other.unbound),
            busy: self.busy.saturating_add(other.busy),
            missing: self.missing.saturating_add(other.missing),
        }
    }

    /// Whether the pass found anything an operator must act on.
    const fn faulted(self) -> bool {
        self.bad > 0 || self.unverifiable > 0 || self.misfiled > 0 || self.missing > 0
    }
}

/// The words that end a torn tail: `<fragment> #void`. No reader reads a line
/// that ends in them, and no line this module writes ends in them.
pub(crate) const VOID: &str = " #void";

/// The eight totals of a cursor line, in the order it writes them.
pub const TOTAL_NAMES: [&str; 8] = [
    "months",
    "blocks",
    "bad",
    "unverifiable",
    "misfiled",
    "unbound",
    "busy",
    "missing",
];

/// Whether `line` is the ended tail of a killed run: bytes nobody finished
/// writing, which no reader may take for a line.
#[must_use]
pub(crate) fn is_void(line: &str) -> bool {
    line.ends_with(VOID)
}

/// The whole lines of a tail, oldest first: never the fragment a cut began
/// in, never the fragment after the last newline, never an empty line and
/// never a void one.
#[must_use]
pub fn whole_lines(tail: &str, cut: bool) -> impl DoubleEndedIterator<Item = &str> {
    let whole = tail
        .rfind('\n')
        .map_or("", |end| tail.get(..end).unwrap_or(""));
    let mut lines = whole.split('\n');
    if cut {
        // The first fragment began before the tail did.
        let _partial = lines.next();
    }
    lines.filter(|line| !line.is_empty() && !is_void(line))
}

/// One cursor line, read whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorLine {
    /// `done` (the pass is finished) or `progress`.
    pub done: bool,
    /// The run that wrote it.
    pub run: u64,
    /// The pass it belongs to, never zero.
    pub pass: u64,
    /// The pass's totals so far, in [`TOTAL_NAMES`]' order.
    pub totals: [u64; 8],
    /// A `progress` line's cursor: the last month read, or `None` for `-`
    /// (none yet). Always `None` on a `done` line.
    pub cursor: Option<String>,
}

/// `line` as the cursor line this module writes, or `None`.
///
/// STRICT, and the only reader of a cursor line (the resume here, the API's
/// `/scrub.json` and `/disk.json`): the kind, then `run=`, `pass=`, the eight
/// totals in [`TOTAL_NAMES`]' order, then `cursor=` on a `progress` line and
/// nothing on a `done` one — each present once, in that place, each a decimal
/// number, and not a word more. A reader that took "the first `months=` word"
/// read a killed run's fragment for the pass's totals once the next line was
/// joined onto it; a joined line is two lines' words and is refused here.
#[must_use]
pub fn cursor_line(line: &str) -> Option<CursorLine> {
    if is_void(line) {
        return None;
    }
    let mut words = line.split(' ');
    let done = match words.next()? {
        "done" => true,
        "progress" => false,
        _ => return None,
    };
    let mut number = |key: &str| {
        let digits = words.next()?.strip_prefix(key)?.strip_prefix('=')?;
        // DIGITS ONLY: `u64::from_str` also takes a leading `+`.
        (!digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| digits.parse::<u64>().ok())?
    };
    let run = number("run")?;
    let pass = number("pass").filter(|pass| *pass > 0)?;
    let mut totals = [0_u64; 8];
    for (slot, key) in totals.iter_mut().zip(TOTAL_NAMES) {
        *slot = number(key)?;
    }
    let cursor = if done {
        None
    } else {
        let path = words.next()?.strip_prefix("cursor=")?;
        if path.is_empty() {
            return None;
        }
        (path != "-").then(|| path.to_owned())
    };
    if words.next().is_some() {
        return None;
    }
    Some(CursorLine {
        done,
        run,
        pass,
        totals,
        cursor,
    })
}

/// The run id of a ledger line: the second word of a `run` line, the `run=`
/// word before the reason of any other.
fn run_of(line: &str) -> Option<u64> {
    let (head, _why) = line.split_once(" why=").unwrap_or((line, ""));
    let mut words = head.split(' ');
    if words.next() == Some("run") {
        return words.next()?.parse().ok();
    }
    words.find_map(|word| word.strip_prefix("run=")?.parse().ok())
}

/// Where a run starts: the pass, the path after which it resumes, the pass's
/// totals so far, and the newest run id the cursor names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Resume {
    pub(crate) pass: u64,
    pub(crate) cursor: Option<String>,
    pub(crate) totals: Totals,
    /// The run of the cursor's newest line; zero for an empty cursor.
    pub(crate) newest_run: u64,
}

/// Where the next run starts, from the cursor file's tail.
///
/// Only whole lines are read ([`whole_lines`]): `tail` may begin mid-line (it
/// is the end of a longer file when `cut` is set), may end mid-line (a run
/// killed while writing) and may hold the ended fragments of earlier killed
/// runs, and none of those is a line. The newest whole `progress` line
/// resumes its pass after its cursor with its totals; the newest whole `done`
/// line starts the next pass. An empty file starts pass 1.
///
/// # Errors
///
/// A file that holds bytes and no whole line, or whose newest whole line is
/// not one [`cursor_line`] reads: where to resume cannot be told, and guessing
/// "from the start" would rerun, and recount, a pass in silence.
pub(crate) fn resume_from(tail: &str, cut: bool) -> Result<Resume, String> {
    if tail.is_empty() {
        return Ok(Resume {
            pass: 1,
            ..Resume::default()
        });
    }
    let Some(last) = whole_lines(tail, cut).next_back() else {
        return Err("holds bytes and no whole line".to_owned());
    };
    let Some(read) = cursor_line(last) else {
        return Err(format!(
            "its newest line is no progress or done line this build reads: {last:?}"
        ));
    };
    Ok(if read.done {
        Resume {
            pass: read.pass.saturating_add(1),
            cursor: None,
            totals: Totals::default(),
            newest_run: read.run,
        }
    } else {
        Resume {
            pass: read.pass,
            cursor: read.cursor,
            totals: Totals::of(read.totals),
            newest_run: read.run,
        }
    })
}

/// The last [`CURSOR_TAIL_BYTES`] of `file`, and whether that cut off an
/// earlier part of it.
fn tail_of(file: &mut std::fs::File, path: &Path) -> Result<(String, bool), String> {
    let named = |why: std::io::Error| format!("{}: {why}", path.display());
    let len = file.metadata().map_err(named)?.len();
    let from = len.saturating_sub(CURSOR_TAIL_BYTES);
    file.seek(SeekFrom::Start(from)).map_err(named)?;
    let mut bytes = Vec::new();
    file.take(CURSOR_TAIL_BYTES)
        .read_to_end(&mut bytes)
        .map_err(named)?;
    Ok((String::from_utf8_lossy(&bytes).into_owned(), from > 0))
}

/// One sentence on one line: a reason never breaks a line in two.
fn one_line(text: &str) -> String {
    text.replace(['\n', '\r'], " ")
}

/// One append-only file of lines.
struct Lines {
    file: std::fs::File,
    path: PathBuf,
}

impl Lines {
    /// Opens `relative` under `root` for append and read, creating it and the
    /// audit directory when absent.
    ///
    /// The open never follows a final symlink and never waits on a FIFO, and
    /// what it opened must be a plain file: anything else is refused by name
    /// (a line appended through a link lands in a file this store does not
    /// own; a FIFO takes the lines and keeps none).
    fn open(root: &Path, relative: &str) -> Result<Self, String> {
        use std::os::unix::fs::OpenOptionsExt as _;
        let path = root.join(relative);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .custom_flags(store::open_flags::O_NOFOLLOW_NONBLOCK)
            .open(&path)
            .map_err(|why| {
                format!(
                    "{}: {why} (it must be a plain file, never a link)",
                    path.display()
                )
            })?;
        let plain = file
            .metadata()
            .map_err(|why| format!("{}: {why}", path.display()))?
            .is_file();
        if !plain {
            return Err(format!(
                "{} is not a plain file; nothing was read or written",
                path.display()
            ));
        }
        Ok(Self { file, path })
    }

    /// Ends a tail no newline ends, as a void line, and returns the bytes of
    /// the fragment it ended (zero when the file is empty or ends whole).
    ///
    /// Called under the vendor's run lock, before anything is appended: the
    /// fragment is what a run killed or out of disk mid-append left, and the
    /// next line must start at a line's start, never be joined onto it.
    fn end_torn_tail(&mut self) -> Result<u64, String> {
        let named = |why: std::io::Error| format!("{}: {why}", self.path.display());
        let len = self.file.metadata().map_err(named)?.len();
        // THE FRAGMENT IS WHAT FOLLOWS THE LAST NEWLINE of a bounded tail; a
        // tail with no newline at all is a fragment at least that long.
        let from = len.saturating_sub(CURSOR_TAIL_BYTES);
        self.file.seek(SeekFrom::Start(from)).map_err(named)?;
        let mut tail = Vec::new();
        (&self.file)
            .take(CURSOR_TAIL_BYTES)
            .read_to_end(&mut tail)
            .map_err(named)?;
        let torn = tail
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(tail.len(), |newline| tail.len() - newline - 1);
        if torn == 0 {
            return Ok(0);
        }
        let mut end = Vec::with_capacity(VOID.len() + 1);
        end.extend_from_slice(VOID.as_bytes());
        end.push(b'\n');
        self.file.write_all(&end).map_err(named)?;
        self.file.sync_data().map_err(named)?;
        Ok(u64::try_from(torn).unwrap_or(u64::MAX))
    }

    /// The newest run id among the file's last whole lines, or zero.
    fn newest_run(&mut self) -> Result<u64, String> {
        let (tail, cut) = tail_of(&mut self.file, &self.path)?;
        Ok(whole_lines(&tail, cut)
            .filter_map(run_of)
            .max()
            .unwrap_or(0))
    }

    fn line(&mut self, text: &str) -> Result<(), String> {
        // ONE WRITE PER LINE, newline included, so a line is whole or is the
        // tail of a killed run, which the next run ends as a void line.
        let mut bytes = Vec::with_capacity(text.len() + 2);
        bytes.extend_from_slice(text.as_bytes());
        if is_void(text) {
            // No written line reads as a void one, whatever a reason ended in.
            bytes.push(b'.');
        }
        bytes.push(b'\n');
        self.file
            .write_all(&bytes)
            .map_err(|why| format!("{}: {why}", self.path.display()))
    }

    fn sync(&self) -> Result<(), String> {
        self.file
            .sync_data()
            .map_err(|why| format!("{}: {why}", self.path.display()))
    }
}

/// What one run did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Ran {
    /// The pass this run belongs to.
    pub(crate) pass: u64,
    /// What this run alone met.
    pub(crate) this_run: Totals,
    /// The pass so far, this run included.
    pub(crate) pass_totals: Totals,
    /// Whether the pass reached the census's last month.
    pub(crate) finished_pass: bool,
    /// This run's id.
    pub(crate) id: u64,
    /// Each torn tail this run ended before it appended: the file, relative
    /// to the store root, and the fragment's bytes.
    pub(crate) torn: Vec<(String, u64)>,
    /// Census entries that name no legal store path, so no file of theirs
    /// could be looked for: not read, and said.
    pub(crate) unaddressed: Unaddressed,
}

/// `entry`'s bar file, as the store addresses it, or `None` for an entry that
/// names no legal store path.
fn path_of(vendor: Vendor, entry: &Entry) -> Option<StorePath<'_>> {
    StorePath::new(PathParts {
        vendor,
        exchange: entry.key.exchange.as_str(),
        segment: entry.key.segment.as_str(),
        symbol: entry.key.symbol.as_str(),
        contract: entry.key.contract,
        timeframe: entry.key.timeframe,
        month: entry.key.month,
        file: FileKind::Bars,
    })
    .ok()
}

/// Census entries a run could not address: how many, and the first by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Unaddressed {
    /// How many entries name no legal store path.
    pub(crate) entries: u64,
    /// The first of them, as the census names it.
    pub(crate) first: Option<String>,
}

/// `entries` as (relative path, entry) in path order, and those `path` could
/// not address — COUNTED AND NAMED, never dropped: a census entry that names
/// no legal store path was skipped in silence, so a pass could say `complete`
/// over months it never tried.
fn addressed(
    entries: impl Iterator<Item = Entry>,
    path: impl Fn(&Entry) -> Option<String>,
) -> (Vec<(String, Entry)>, Unaddressed) {
    let mut out = Vec::new();
    let mut unaddressed = Unaddressed::default();
    for entry in entries {
        if let Some(relative) = path(&entry) {
            out.push((relative, entry));
            continue;
        }
        unaddressed.entries += 1;
        if unaddressed.first.is_none() {
            unaddressed.first = Some(format!("{:?}", entry.key));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    (out, unaddressed)
}

/// The census's months of `vendor`, as (relative path, entry), in path order,
/// with the entries that name no store path.
fn months_of(root: &Path, vendor: Vendor) -> Result<(Vec<(String, Entry)>, Unaddressed), String> {
    let censuses = census_store::read_vendor(root, vendor)?;
    Ok(addressed(
        censuses.iter().flat_map(|read| read.census.newest()),
        |entry| {
            path_of(vendor, entry).map(|path| path.to_path_buf(Path::new("")).display().to_string())
        },
    ))
}

/// How a run is bounded.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Bounds<'a> {
    /// At most this many months, or every remaining one.
    pub(crate) months: Option<u64>,
    /// At most this many bytes a second; zero is unthrottled.
    pub(crate) bytes_per_s: u64,
    /// The build's commit, for the run line.
    pub(crate) commit: Option<&'a str>,
}

/// Opens `vendor`'s ledger for a run that holds the run lock: ends its torn
/// tail, chooses the run's id, and writes the `torn` lines and the `run` line.
fn begin_ledger(
    root: &Path,
    vendor: Vendor,
    resume: &Resume,
    torn: &mut Vec<(String, u64)>,
    commit: Option<&str>,
) -> Result<(Lines, u64), String> {
    let mut ledger = Lines::open(root, &ledger_name(vendor))?;
    let torn_ledger = ledger.end_torn_tail()?;
    if torn_ledger > 0 {
        torn.push((ledger_name(vendor), torn_ledger));
    }
    // ONE ID PER RUN OF A VENDOR: the clock's second, or the next after the
    // newest id on file when the clock has not passed it (two runs in one
    // second, or a clock set back). One run holds the lock, so no other run
    // is choosing an id now.
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
        .max(
            resume
                .newest_run
                .max(ledger.newest_run()?)
                .saturating_add(1),
        );
    for (file, bytes) in torn.iter() {
        ledger.line(&format!("torn {file} bytes={bytes} run={id}"))?;
    }
    ledger.line(&format!(
        "run {id} vendor={} commit={} pass={} resume={}",
        vendor.as_str(),
        commit.unwrap_or("unstamped"),
        resume.pass,
        resume.cursor.as_deref().unwrap_or("-")
    ))?;
    Ok((ledger, id))
}

/// The months a run of `vendor` has to read, or the refusal of a vendor that
/// holds none.
///
/// THE CENSUS FIRST, BEFORE EITHER FILE EXISTS. A vendor with no census —
/// never imported, or a name typed for another — was scrubbed "complete" over
/// zero months, and its ledger and cursor were created to say so: `/disk.json`
/// then showed a finished pass of a feed that holds nothing.
fn months_to_scrub(
    root: &Path,
    vendor: Vendor,
) -> Result<(Vec<(String, Entry)>, Unaddressed), String> {
    let (all, unaddressed) = months_of(root, vendor)?;
    if all.is_empty() {
        return Err(format!(
            "no census of {} names a month under {}, so there is nothing to scrub; no \
             ledger and no cursor was written{}",
            vendor.as_str(),
            root.display(),
            unaddressed
                .first
                .as_deref()
                .map_or_else(String::new, |first| format!(
                    " ({} census entr(ies) name no legal store path, the first {first})",
                    unaddressed.entries
                ))
        ));
    }
    Ok((all, unaddressed))
}

/// One run over `vendor`'s months.
///
/// # Errors
///
/// Before a block is read: a census this build may not read, another run of
/// this vendor holding the cursor, a cursor that cannot say where to resume,
/// or either file refusing to open. After: a line that could not be written
/// or flushed — the run stops there, and the next resumes from the last line
/// that was.
pub(crate) fn run(root: &Path, vendor: Vendor, bounds: Bounds<'_>) -> Result<Ran, String> {
    let (all, unaddressed) = months_to_scrub(root, vendor)?;
    let mut cursor_file = Lines::open(root, &cursor_name(vendor))?;
    // THE LOCK IS THE OPEN FILE DESCRIPTION'S, which a duplicate shares: the
    // guard owns one duplicate, the tail is read through another, and the
    // lines are appended through the original.
    let duplicate = || {
        cursor_file
            .file
            .try_clone()
            .map_err(|why| format!("{}: {why}", cursor_file.path.display()))
    };
    let _one_run = Flock::try_lock(duplicate()?, cursor_file.path.clone()).map_err(|_| {
        format!(
            "{} is held by another scrub of {}; one run at a time, nothing was read",
            cursor_file.path.display(),
            vendor.as_str()
        )
    })?;
    // UNDER THE LOCK, BEFORE ANYTHING IS READ OR APPENDED: a tail a killed run
    // left is ended, so it is never the head of this run's first line.
    let mut torn = Vec::new();
    let mut reader = duplicate()?;
    let torn_cursor = cursor_file.end_torn_tail()?;
    if torn_cursor > 0 {
        torn.push((cursor_name(vendor), torn_cursor));
    }
    let (tail, cut) = tail_of(&mut reader, &cursor_file.path)?;
    let resume = resume_from(&tail, cut).map_err(|why| {
        format!(
            "{} {why}; where to resume cannot be told, so nothing was read. Move the \
             file aside to start pass 1 again",
            cursor_file.path.display()
        )
    })?;
    let (mut ledger, id) = begin_ledger(root, vendor, &resume, &mut torn, bounds.commit)?;
    let mut ran = Ran {
        pass: resume.pass,
        pass_totals: resume.totals,
        id,
        torn,
        unaddressed,
        ..Ran::default()
    };
    let started = Instant::now();
    let mut said = Instant::now();
    let mut read_bytes = 0_u64;
    let mut cursor = resume.cursor.clone();
    // THE MONTHS AFTER THE CURSOR, found by one binary search of the sorted
    // paths rather than a comparison per month.
    let from = resume.cursor.as_deref().map_or(0, |after| {
        all.partition_point(|(relative, _)| relative.as_str() <= after)
    });
    let mut finished = true;
    for (relative, entry) in all.iter().skip(from) {
        if bounds
            .months
            .is_some_and(|limit| ran.this_run.months >= limit)
        {
            finished = false;
            break;
        }
        let rate = Rate {
            started,
            bytes_per_s: bounds.bytes_per_s,
        };
        read_bytes = one_month(
            (root, vendor),
            (relative, entry),
            id,
            &mut ran,
            &mut ledger,
            rate,
            read_bytes,
        )?;
        ran.this_run.months += 1;
        cursor = Some(relative.clone());
        if ran.this_run.months.is_multiple_of(PROGRESS_MONTHS) || said.elapsed() >= PROGRESS_EVERY {
            ledger.sync()?;
            cursor_file.line(&progress_line(id, &ran, cursor.as_deref()))?;
            cursor_file.sync()?;
            said = Instant::now();
        }
    }
    ran.finished_pass = finished;
    ran.pass_totals = ran.pass_totals.plus(ran.this_run);
    // THE FINDINGS ARE DURABLE BEFORE THE CURSOR MOVES PAST THEM: a crash
    // between the two re-reads months, it never skips a finding.
    ledger.sync()?;
    if finished {
        cursor_file.line(&format!(
            "done run={id} pass={} {}",
            ran.pass,
            ran.pass_totals.words()
        ))?;
    } else {
        cursor_file.line(&format!(
            "progress run={id} pass={} {} cursor={}",
            ran.pass,
            ran.pass_totals.words(),
            cursor.as_deref().unwrap_or("-")
        ))?;
    }
    cursor_file.sync()?;
    Ok(ran)
}

/// When a run started and how fast it may read.
#[derive(Debug, Clone, Copy)]
struct Rate {
    started: Instant,
    bytes_per_s: u64,
}

/// One month: opened through the store's reader door, every block read, each
/// finding a ledger line. Returns the run's bytes read so far.
fn one_month(
    (root, vendor): (&Path, Vendor),
    (relative, entry): (&str, &Entry),
    id: u64,
    ran: &mut Ran,
    ledger: &mut Lines,
    rate: Rate,
    read_bytes: u64,
) -> Result<u64, String> {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the header cross-check is the low 32 bits of the symbol's FNV-1a, as the \
                  writer folds it"
    )]
    let symbol_id = brutex_core::universe::fnv1a(entry.key.symbol.as_str()) as u32;
    let Some(path) = path_of(vendor, entry) else {
        return Ok(read_bytes);
    };
    match BarFile::open_existing(root, path, symbol_id) {
        Ok(file) => {
            // NO SIDECAR BINDS THIS MONTH TO ITS PATH (DB-B65): counted, so
            // the size of that limit is a number on a page, not a guess. A
            // sidecar naming ANOTHER month never reaches here: the door
            // refused it, below.
            if store::provenance::written_for(root, path)
                == store::provenance::WrittenFor::Unrecorded
            {
                ran.this_run.unbound += 1;
            }
            let read = scrub_month(&file, relative, id, ran, ledger, read_bytes)?;
            // THE MONTH IS CLOSED BEFORE THE RUN SLEEPS (rules audit RA-128):
            // the rate limit used to sleep between chunks WITH the month open,
            // so at a low rate a reader's shared lock refused that month's
            // writer for as long as the sleep took (bytes / rate), far past an
            // importer's 2 s retry. A month is now held only for as long as
            // its blocks take to read; what the rate owes is slept with no
            // month open.
            drop(file);
            throttle(rate.started, read, rate.bytes_per_s);
            Ok(read)
        }
        Err(StoreError::Locked { .. }) => {
            ran.this_run.busy += 1;
            ledger.line(&format!("busy {relative} run={id}"))?;
            Ok(read_bytes)
        }
        Err(StoreError::Missing { .. }) => {
            ran.this_run.missing += 1;
            ledger.line(&format!("missing {relative} run={id}"))?;
            Ok(read_bytes)
        }
        Err(why @ StoreError::Misfiled { .. }) => {
            ran.this_run.misfiled += 1;
            ledger.line(&format!(
                "misfiled {relative} run={id} why={}",
                one_line(&why.to_string())
            ))?;
            Ok(read_bytes)
        }
        Err(why) => {
            ran.this_run.unverifiable += 1;
            ledger.line(&format!(
                "unverifiable {relative} run={id} why={}",
                one_line(&why.to_string())
            ))?;
            Ok(read_bytes)
        }
    }
}

/// Every block of one open month, in chunks of [`CHUNK_BLOCKS`], each bad
/// block and an unverifiable month a ledger line. Returns the run's bytes read
/// so far. It never sleeps: the caller pays the rate after the month is closed.
fn scrub_month(
    file: &BarFile,
    relative: &str,
    id: u64,
    ran: &mut Ran,
    ledger: &mut Lines,
    mut read_bytes: u64,
) -> Result<u64, String> {
    let mut from = 0_u64;
    loop {
        let pass = pull::scrub::blocks(file, from, CHUNK_BLOCKS);
        ran.this_run.blocks += pass.checked;
        read_bytes = read_bytes.saturating_add(pass.checked.saturating_mul(BLOCK_BYTES));
        for bad in &pass.bad {
            ran.this_run.bad += 1;
            ledger.line(&format!(
                "bad {relative} block={} records={}-{} run={id} why={}",
                bad.block,
                bad.first,
                bad.last,
                one_line(&bad.why)
            ))?;
        }
        if let Some(why) = &pass.unverifiable {
            ran.this_run.unverifiable += 1;
            ledger.line(&format!(
                "unverifiable {relative} run={id} why={}",
                one_line(why)
            ))?;
        }
        match pass.next {
            Some(next) => from = next,
            None => return Ok(read_bytes),
        }
    }
}

fn progress_line(id: u64, ran: &Ran, cursor: Option<&str>) -> String {
    format!(
        "progress run={id} pass={} {} cursor={}",
        ran.pass,
        ran.pass_totals.plus(ran.this_run).words(),
        cursor.unwrap_or("-")
    )
}

/// Sleeps as long as reading `read` bytes since `started` ran ahead of
/// `bytes_per_s`. A rate of zero is unthrottled.
fn throttle(started: Instant, read: u64, bytes_per_s: u64) {
    if bytes_per_s == 0 {
        return;
    }
    let micros = u128::from(read).saturating_mul(1_000_000) / u128::from(bytes_per_s);
    let owed = Duration::from_micros(u64::try_from(micros).unwrap_or(u64::MAX));
    if let Some(ahead) = owed.checked_sub(started.elapsed()) {
        std::thread::sleep(ahead);
    }
}

/// The verb's usage, said with every refusal of its arguments.
const TAKES: &str = "scrub-blocks takes VENDOR [--months N] [--mib-per-s N]";

/// The verb, with its store root passed in.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    commit: Option<&str>,
    out: &mut String,
) -> u8 {
    let Some((feed, rest)) = arguments.split_first() else {
        let _ = writeln!(out, "refused: {TAKES}");
        return MISUSED;
    };
    let Some(vendor) = Vendor::ALL.into_iter().find(|v| v.as_str() == *feed) else {
        let _ = writeln!(out, "refused: {feed:?} is not a vendor this store keeps");
        return MISUSED;
    };
    let mut months = None;
    let mut mib = DEFAULT_MIB_PER_S;
    let mut words = rest.iter();
    while let Some(word) = words.next() {
        let value = words.next().and_then(|v| v.parse::<u64>().ok());
        match (*word, value) {
            ("--months", Some(n)) => months = Some(n),
            ("--mib-per-s", Some(n)) => mib = n,
            _ => {
                let _ = writeln!(out, "refused: {word:?} — {TAKES}");
                return MISUSED;
            }
        }
    }
    let bounds = Bounds {
        months,
        bytes_per_s: mib.saturating_mul(1024 * 1024),
        commit,
    };
    match store().and_then(|root| run(&root, vendor, bounds).map(|ran| (root, ran))) {
        Ok((root, ran)) => {
            let _ = writeln!(
                out,
                "SCRUB BLOCKS -- {} pass {}: this run {}; the pass so far {}{}",
                vendor.as_str(),
                ran.pass,
                ran.this_run.words(),
                ran.pass_totals.words(),
                if ran.finished_pass {
                    " -- pass complete"
                } else {
                    " -- run again to resume"
                }
            );
            for (file, bytes) in &ran.torn {
                let _ = writeln!(
                    out,
                    "  torn tail ended     {file}: {bytes} byte(s) of a line a killed run never \
                     finished, ended as a void line and not read"
                );
            }
            if let Some(first) = &ran.unaddressed.first {
                let _ = writeln!(
                    out,
                    "  NOT ADDRESSED       {} census entr(ies) name no legal store path, so no \
                     file of theirs was looked for and this pass did not read them (the first: \
                     {first})",
                    ran.unaddressed.entries
                );
            }
            let _ = writeln!(
                out,
                "  ledger              {}",
                root.join(ledger_name(vendor)).display()
            );
            let _ = writeln!(
                out,
                "  cursor              {}",
                root.join(cursor_name(vendor)).display()
            );
            if ran.this_run.faulted() { FAILED } else { OK }
        }
        Err(why) => {
            let _ = writeln!(out, "SCRUB BLOCKS REFUSED: {why}");
            FAILED
        }
    }
}

/// The verb against the CLI's store root, which an unstamped build is refused
/// (`store_write`): the two files this writes are served as findings about
/// the store, and each run line names the build that read the blocks.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    let store = crate::store_write::root_for("scrub-blocks", arguments);
    command_with(arguments, &store, crate::commit_stamp(), out)
}

#[cfg(test)]
#[path = "scrub_blocks_tests.rs"]
mod tests;
