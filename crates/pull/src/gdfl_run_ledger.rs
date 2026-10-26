//! `pull::gdfl_run::ledger`: the store-wide runs ledger (IA-2; D-4847,
//! amended by D-4880; round 4, SF-12; design §5.3).
//!
//! One append-only file, `<store>/audit/runs.ledger`, of fixed 128-byte
//! ASCII records, one per run of either feed and any mode. A run's id is
//! given here and nowhere else, so an id names one run in `/logs.json?run=`,
//! a receipt and a verdict, whichever feed it belongs to, and is never given
//! twice, whatever the clock does and whether or not the run ever writes a
//! journal line.
//!
//! # The record (`docs/02-store-format.md` §37)
//!
//! | bytes | holds |
//! |---|---|
//! | 0..114 | `<id> <mode> <kind> <by> <pid> <started>`, then ` resumes=<id>` when the run resumes one (RA-230), padded with spaces |
//! | 114..126 | `crc=` and 8 lowercase hex digits: CRC-32C over bytes 0..114 |
//! | 126 | a space |
//! | 127 | a newline |
//!
//! # The allocator: O(1) per run (measured by
//! `pull::gdfl_run::ia2_tests::ia2_timings`, C-GR-08)
//!
//! Under `audit/runs.ledger.lock` (taken within [`super::locks::LOCK_WAIT`],
//! D-4906): it reads the last whole record at L = ⌊len ÷ 128⌋ × 128 − 128
//! (one positional read of 128 bytes) and the claims' tail (one positional
//! read of at most [`CLAIMS_TAIL`] bytes), takes `id = max(now, last + 1)`
//! where `last` is the newer of the ledger's last record and the claims' last
//! line whose CRC verifies, writes its record at ⌊len ÷ 128⌋ × 128, over any
//! torn tail, and syncs; then appends the id's claims line (one 1-byte read
//! of the claims' tail, one write) and syncs again (D-6269).
//!
//! A crash's leftovers (round 4, SF-12): bytes past the last whole record are
//! a torn append and are written over; a last whole record whose CRC fails is
//! most likely an append that never finished its sync, whose id was never
//! used, but may be a record a run was given and then lost to bit rot (IA-2
//! repair 2, D-5901): the record before it is read (a second 128-byte read),
//! `last` is taken from it, the new id is raised past the largest id the
//! claims and the GDFL journal hold (below), and the new record is written
//! at L over the bad one. A ledger shorter than one record, or whose only
//! record fails its CRC, is unborn and is seeded again. TWO bad records at
//! the tail cannot come from one interrupted append:
//! [`RunsLedgerFault::Damaged`] names the offset, an error event and an
//! alert (kind `runs_ledger_damaged`, keyed per offset) are put, and no run
//! of either feed starts until the operator has looked: never skipped, never
//! repaired silently.
//!
//! **Every id is written down twice** (IA-10c repair 2, D-6269): after its
//! record is synced and before the id is handed out, still under the lock,
//! one line `run <id> <mode> <kind> <by> <pid> <started>` (then
//! ` resumes=<id>`, then the record's own ` crc=<8 hex>`) is appended to
//! [`CLAIMS`] and synced. A run of any feed and mode is in it whether or not
//! it ever writes a journal line, so a lost last record is raised past from
//! it: before, only the GDFL journal was read, and a Zerodha landing's lost
//! id, which no journal line holds, was given again to the next landing under
//! a clock at or behind it. A claims line that cannot be written refuses the
//! allocation (`runs_ledger_io`); the id's record stays in the ledger,
//! unused, and later ids go past it.
//!
//! **A ledger behind its claims is raised past them** (IA-10c repair 3,
//! D-6269): the claims line is synced after its record, so the claims' last
//! line never names a later id than the ledger's last record unless the
//! ledger lost records WHOLE (a drive that dropped a flushed append, a
//! ledger put back from an older copy), which its own bytes cannot show.
//! Every allocation therefore reads the claims' tail, and a later id there
//! is the `last` the new id goes past, said at `warn` with an alert (kind
//! `runs_ledger_damaged`, keyed per claimed id). A claims line is trusted
//! only when its CRC verifies (the record is rebuilt from it and read as a
//! record is), so a rotted line can neither lift every later id past what a
//! receipt holds nor hide one; whole lines at the tail that fail it are said
//! at `warn`. Claims that are not a regular file (a link, a pipe, a
//! directory) refuse the allocation `runs_ledger_io` before anything is
//! written, and are never waited on.
//!
//! **Seeding** (L-IA-23): the first record of an unborn ledger is
//! `<n> seed - - 0 <now>`, where `n` is the largest `run` id the GDFL journal
//! and the claims (their lines whose CRC verifies) hold, one pass over each,
//! once per store, so no id a journal line or a claims line already carries
//! is given again.
//!
//! **A clock that cannot be right is refused** (RA-292, RA-326):
//! `clock_implausible` when today is before the calendar's first day, more
//! than a year past the last day the calendar's sources cover, or more than
//! a day before the newest record's start. One start under a clock in 2040
//! would otherwise lift every later id for ever.

use std::fmt::Write as _;
use std::fs::File;
use std::io::ErrorKind;
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};

use store::flock::Flock;

use super::locks::{self, LockFault, TARGET};

/// The ledger, beneath the store root.
pub const LEDGER: &str = "audit/runs.ledger";
/// Its lock.
pub const LEDGER_LOCK: &str = "audit/runs.ledger.lock";
/// Its claims: one `run <id> ...` line per id given, appended under
/// [`LEDGER_LOCK`] after the id's record is synced (D-6269).
pub const CLAIMS: &str = "audit/runs.claims";
/// One record's bytes.
pub const RECORD: u64 = 128;
/// The claims' tail every allocation reads (IA-10c repair 3, D-6269): four
/// records' worth. A claims line is at most 131 bytes and its line break
/// (`run `, a body of at most 114 bytes, ` crc=` and 8 hex digits), so a
/// torn last fragment and a first line the read cuts into still leave one
/// whole line inside it.
pub const CLAIMS_TAIL: u64 = 4 * RECORD;
/// The bytes the CRC covers.
pub const BODY: usize = 114;
/// One day, in seconds: how far behind the newest record a clock may be.
pub const CLOCK_BEHIND_S: u64 = 86_400;
/// How far past the last day the calendar's sources cover a clock may be.
pub const CLOCK_AHEAD_DAYS: i64 = 366;

/// The modes a record may name (design §5.3, round 6).
pub const MODES: [&str; 15] = [
    "seed",
    "import",
    "verify",
    "deferred",
    "completeness",
    "census-migrate",
    "census-recount",
    "dayflags",
    "pull",
    "pull-spot",
    "pull-fno",
    "pull-repair",
    "zerodha-repair",
    "zerodha-verify",
    "verify-merge",
];

/// The kinds a record may name.
pub const KINDS: [&str; 6] = ["indices", "stocks", "options", "census", "zerodha", "-"];

/// Who a record may name.
pub const BY: [&str; 3] = ["cli", "api", "-"];

/// One record, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The run's id.
    pub id: u64,
    /// A word of [`MODES`].
    pub mode: String,
    /// A word of [`KINDS`].
    pub kind: String,
    /// A word of [`BY`].
    pub by: String,
    /// The process.
    pub pid: u32,
    /// When it started, UNIX seconds.
    pub started: u64,
    /// The run it resumes, or 0.
    pub resumes: u64,
}

impl Entry {
    /// The record's 128 bytes.
    ///
    /// # Errors
    ///
    /// A word outside its list, or a body past 114 bytes.
    pub fn record(&self) -> Result<[u8; 128], String> {
        if !MODES.contains(&self.mode.as_str())
            || !KINDS.contains(&self.kind.as_str())
            || !BY.contains(&self.by.as_str())
        {
            return Err(format!(
                "mode {:?}, kind {:?} or by {:?} is not a word the ledger holds",
                self.mode, self.kind, self.by
            ));
        }
        let mut body = format!(
            "{} {} {} {} {} {}",
            self.id, self.mode, self.kind, self.by, self.pid, self.started
        );
        if self.resumes > 0 {
            let _ = write!(body, " resumes={}", self.resumes);
        }
        if body.len() > BODY {
            return Err(format!(
                "a record body of {} bytes is past {BODY}",
                body.len()
            ));
        }
        let mut out = [b' '; 128];
        for (slot, byte) in out.iter_mut().zip(body.bytes()) {
            *slot = byte;
        }
        let crc = format!(
            "crc={:08x}",
            store::crc::crc32c(out.get(..BODY).unwrap_or_default())
        );
        for (slot, byte) in out.iter_mut().skip(BODY).zip(crc.bytes()) {
            *slot = byte;
        }
        if let Some(last) = out.last_mut() {
            *last = b'\n';
        }
        Ok(out)
    }

    /// Reads one record; `None` when its CRC fails or its body is not one.
    #[must_use]
    pub fn of(record: &[u8]) -> Option<Self> {
        if record.len() != 128 || record.last() != Some(&b'\n') || record.get(126) != Some(&b' ') {
            return None;
        }
        let body = record.get(..BODY)?;
        let crc = std::str::from_utf8(record.get(BODY..126)?).ok()?;
        if crc != format!("crc={:08x}", store::crc::crc32c(body)) {
            return None;
        }
        let text = std::str::from_utf8(body).ok()?.trim_end_matches(' ');
        let words: Vec<&str> = text.split(' ').collect();
        let (fixed, rest) = words.split_at_checked(6)?;
        let [id, mode, kind, by, pid, started] = fixed else {
            return None;
        };
        let resumes = match rest {
            [] => 0,
            [one] => one.strip_prefix("resumes=")?.parse().ok()?,
            _ => return None,
        };
        let entry = Self {
            id: id.parse().ok()?,
            mode: (*mode).to_owned(),
            kind: (*kind).to_owned(),
            by: (*by).to_owned(),
            pid: pid.parse().ok()?,
            started: started.parse().ok()?,
            resumes,
        };
        (MODES.contains(mode) && KINDS.contains(kind) && BY.contains(by)).then_some(entry)
    }
}

/// Why no id was given (design §5.8). Codes are never renamed or reused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunsLedgerFault {
    /// Two records at the tail fail their CRC (round 4, SF-12).
    Damaged {
        /// The offset of the first bad record.
        offset: u64,
    },
    /// The clock cannot be right (RA-292, RA-326).
    ClockImplausible {
        /// The clock, UNIX seconds.
        now: u64,
        /// Why.
        why: String,
    },
    /// Its lock could not be taken: stalled past `LOCK_WAIT`, replaced, or
    /// refused.
    Lock(LockFault),
    /// The ledger could not be read or written.
    Io {
        /// Why.
        why: String,
    },
    /// The entry is not one the ledger holds.
    Entry {
        /// Why.
        why: String,
    },
}

impl RunsLedgerFault {
    /// The code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Damaged { .. } => "runs_ledger_damaged",
            Self::ClockImplausible { .. } => "clock_implausible",
            Self::Lock(fault) => fault.code(),
            Self::Io { .. } => "runs_ledger_io",
            Self::Entry { .. } => "runs_ledger_entry",
        }
    }
}

impl core::fmt::Display for RunsLedgerFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Damaged { offset } => write!(
                f,
                "runs_ledger_damaged: {LEDGER} has two records at its tail whose CRC fails, the \
                 first at byte {offset}; one interrupted append cannot leave two, so no run \
                 starts until the operator has looked (never skipped, never repaired silently)"
            ),
            Self::ClockImplausible { now, why } => write!(
                f,
                "clock_implausible: the clock says {now} (UNIX s), {why}; no run id is given \
                 under it, so a wrong clock cannot lift every later id for ever"
            ),
            Self::Lock(fault) => fault.fmt(f),
            Self::Io { why } => write!(f, "runs_ledger_io: {LEDGER}: {why}"),
            Self::Entry { why } => write!(f, "runs_ledger_entry: {why}"),
        }
    }
}

/// The ledger's lock, held: ids are allocated under it, and a GDFL run
/// writes its claim before it is dropped (D-4880).
#[derive(Debug)]
pub struct LedgerHold {
    store: PathBuf,
    _lock: Flock<File>,
}

impl LedgerHold {
    /// Takes the ledger's lock of `store` within `wait` (D-4906).
    ///
    /// # Errors
    ///
    /// [`RunsLedgerFault::Lock`].
    pub fn take(store: &Path, wait: std::time::Duration) -> Result<Self, RunsLedgerFault> {
        let lock = locks::take_within(store, LEDGER_LOCK, wait).map_err(RunsLedgerFault::Lock)?;
        Ok(Self {
            store: store.to_path_buf(),
            _lock: lock,
        })
    }

    /// Allocates the next id for `entry` (its `id` ignored, its `started`
    /// the clock), writes its record and syncs it, then appends its claims
    /// line and syncs that (D-6269). O(1): one or two 128-byte reads of the
    /// ledger and one read of at most [`CLAIMS_TAIL`] bytes of the claims'
    /// tail, one write and one sync of the record, one 1-byte read, one write
    /// and one sync of the claims line (seeding once per store, and one pass
    /// over the journal and the claims when a damaged last record is written
    /// over, which is rare and said at `warn`: O(journal + claims) then,
    /// bounded by L-IA-23's seed); measured by
    /// `pull::gdfl_run::ia2_tests::ia2_timings` (C-GR-08).
    ///
    /// # Errors
    ///
    /// Every [`RunsLedgerFault`]; on [`RunsLedgerFault::Damaged`] an error
    /// event and an alert are put first.
    pub fn allocate(&mut self, entry: &Entry) -> Result<u64, RunsLedgerFault> {
        let path = self.store.join(LEDGER);
        let io = |why: std::io::Error| RunsLedgerFault::Io {
            why: why.to_string(),
        };
        let file = locks::open_lock_file(&self.store, LEDGER).map_err(RunsLedgerFault::Lock)?;
        let len = file.metadata().map_err(io)?.len();
        let whole = len / RECORD;
        let read = |at: u64| -> Result<Option<Entry>, RunsLedgerFault> {
            let mut buf = [0_u8; 128];
            match file.read_exact_at(&mut buf, at) {
                Ok(()) => Ok(Entry::of(&buf)),
                Err(why) if why.kind() == ErrorKind::UnexpectedEof => Ok(None),
                Err(why) => Err(io(why)),
            }
        };
        // (offset to write at, the last record's id and start, or the claims'
        // last line's when that is later; `None` when the ledger is unborn and
        // is seeded at 0; and, when a damaged last record is written over, the
        // largest run id the journal and the claims hold).
        let (mut at, last, used) = if whole == 0 {
            (0, None, None)
        } else {
            let tail = (whole - 1) * RECORD;
            match read(tail)? {
                // A HEALTHY LAST RECORD IS NOT ASSUMED THE LAST ID GIVEN (IA-10c
                // repair 3, D-6269): a ledger that lost records whole (a drive
                // that dropped a flushed append, a ledger put back from an
                // older copy) ends at a record that verifies, and only its
                // claims, synced after each record, still name the ids after
                // it. Their tail is read every time (one bounded read), and
                // the newer of the two is the last id the new one goes past.
                Some(last) => (
                    whole * RECORD,
                    Some(newest(&self.store, &path, last)?),
                    None,
                ),
                None if whole == 1 => (0, None, None),
                None => {
                    let Some(before) = read(tail - RECORD)? else {
                        let offset = tail - RECORD;
                        note_damaged(&self.store, &path, offset);
                        return Err(RunsLedgerFault::Damaged { offset });
                    };
                    // A DAMAGED LAST RECORD IS NOT ASSUMED UNUSED (IA-2
                    // repair 2; D-4880, D-5901): an append whose sync never
                    // finished is the likely cause, but a record a run was
                    // given and then lost to bit rot reads the same. Its id
                    // is never parsed from the bad bytes; the claims, which
                    // hold every id the ledger has given (D-6269), and the
                    // journal, which holds every id a GDFL run wrote a line
                    // under before the claims existed, are read once and the
                    // new id is raised past both, so a clock at or behind
                    // the lost record cannot hand that id out again, whichever
                    // feed's run it was.
                    let floor = Floor::read(&self.store)?;
                    note_written_over(&path, tail, floor);
                    (tail, Some(before), Some(floor.largest()))
                }
            }
        };
        let now = entry.started;
        clock_check(now, last.as_ref())?;
        let mut bytes = Vec::with_capacity(256);
        let last_id = if let Some(last) = last {
            last.id
        } else {
            // AN UNBORN LEDGER IS SEEDED PAST EVERY ID GIVEN BEFORE IT
            // (L-IA-23; D-6269): one its only record held and lost, one of a
            // ledger the store was put back without, are in the claims.
            let seed = Entry {
                id: Floor::read(&self.store)?.largest(),
                mode: "seed".to_owned(),
                kind: "-".to_owned(),
                by: "-".to_owned(),
                pid: 0,
                started: now,
                resumes: 0,
            };
            bytes.extend_from_slice(
                &seed
                    .record()
                    .map_err(|why| RunsLedgerFault::Entry { why })?,
            );
            at = 0;
            seed.id
        };
        // THE LAST ID THERE IS IS NEVER HANDED OUT TWICE (IA-2 repair 1): a
        // ledger whose last id is `u64::MAX` has no later one, and refuses by
        // name rather than repeat it.
        let Some(next) = last_id.checked_add(1) else {
            return Err(RunsLedgerFault::Entry {
                why: format!(
                    "the runs ledger's last id is {last_id}, the largest an id can be: no later \
                     id exists, so none was allocated and nothing was written"
                ),
            });
        };
        let past_journal = match used {
            None => 0,
            Some(used) => used.checked_add(1).ok_or_else(|| RunsLedgerFault::Entry {
                why: format!(
                    "the journal's largest run id is {used}, the largest an id can be: no \
                     later id exists, so none was allocated and nothing was written"
                ),
            })?,
        };
        let id = now.max(next).max(past_journal);
        let mine = Entry {
            id,
            ..entry.clone()
        };
        let record = mine
            .record()
            .map_err(|why| RunsLedgerFault::Entry { why })?;
        bytes.extend_from_slice(&record);
        file.write_all_at(&bytes, at).map_err(io)?;
        file.sync_all().map_err(io)?;
        // EVERY ID IS WRITTEN DOWN TWICE BEFORE IT IS GIVEN (IA-10c repair 2,
        // D-6269): its record, synced, and then its claims line, synced, both
        // under this lock. A run that writes no journal line of its own (a
        // Zerodha landing, a cold verification, a merge) left nothing a lost
        // last record could be raised past, and the next id under a clock at
        // or behind it was that run's id again.
        append_claim(&self.store, &record, id)?;
        Ok(id)
    }
}

/// What an id is raised past when the ledger cannot say its last id (an
/// unborn ledger's seed, a damaged last record written over): the largest
/// `run` id of the GDFL journal (L-IA-23) and of the claims (D-6269), each
/// read once.
#[derive(Debug, Clone, Copy)]
struct Floor {
    /// The GDFL journal's largest `run` id, or 0.
    journal: u64,
    /// The claims' largest id, or 0.
    claims: u64,
}

impl Floor {
    /// Reads both, one pass each.
    fn read(store: &Path) -> Result<Self, RunsLedgerFault> {
        let io = |why: String| RunsLedgerFault::Io { why };
        Ok(Self {
            journal: largest_journal_run(store).map_err(io)?,
            claims: largest_claimed_run(store).map_err(io)?,
        })
    }

    /// The larger of the two.
    fn largest(self) -> u64 {
        self.journal.max(self.claims)
    }
}

/// The claims line of `record`: its body, `run ` first, its padding
/// dropped, then its own CRC field (`run <id> <mode> <kind> <by> <pid>
/// <started>`, then ` resumes=<id>`, then ` crc=<8 hex>`; the CRC since
/// IA-10c repair 3), so a line is verified as its record is ([`claimed`]).
fn claim_line(record: &[u8; 128]) -> String {
    let body = String::from_utf8_lossy(record.get(..BODY).unwrap_or_default());
    let crc = String::from_utf8_lossy(record.get(BODY..126).unwrap_or_default());
    format!("run {} {crc}", body.trim_end_matches(' '))
}

/// The bytes a claims line ends with past its body: ` crc=` and 8 hex digits.
const CLAIM_CRC: usize = 13;

/// The record the claims line `line` (no line break) was written from, when
/// its CRC verifies: the record is rebuilt (the body padded again, then its
/// CRC field, a space and a line break) and read by [`Entry::of`], so a line
/// is trusted exactly when a record would be. `None` for a torn, rotted or
/// foreign line: its id is never read, so a rotted digit can neither lift
/// every later id nor hide one (IA-10c repair 3, D-6269).
fn claimed(line: &[u8]) -> Option<Entry> {
    let rest = line.strip_prefix(b"run ")?;
    let (body, crc) = rest.split_at_checked(rest.len().checked_sub(CLAIM_CRC)?)?;
    let crc = crc.strip_prefix(b" ")?;
    if body.len() > BODY {
        return None;
    }
    let mut record = [b' '; 128];
    for (slot, byte) in record.iter_mut().zip(body) {
        *slot = *byte;
    }
    for (slot, byte) in record.iter_mut().skip(BODY).zip(crc) {
        *slot = *byte;
    }
    if let Some(last) = record.last_mut() {
        *last = b'\n';
    }
    Entry::of(&record)
}

/// What the claims' tail says (IA-10c repair 3, D-6269).
#[derive(Debug, Default)]
struct ClaimsTail {
    /// The record of its last whole line whose CRC verifies.
    last: Option<Entry>,
    /// The whole lines after that one, or in the whole tail when none
    /// verifies, whose CRC fails.
    unverified: usize,
}

/// Reads the last [`CLAIMS_TAIL`] bytes of the claims of `store`: one open
/// (non-blocking, never through a link, a regular file only) and one
/// positional read, whatever the claims' length. The bytes after the last
/// line break are a fragment a crash tore, whose id was never handed out
/// (the next append closes it, and says so); the first line is passed over
/// when the read began inside it.
///
/// # Errors
///
/// Claims that exist and are not a regular file, or cannot be read.
fn claims_tail(store: &Path) -> Result<ClaimsTail, String> {
    let path = store.join(CLAIMS);
    let Some(file) = open_regular(&path, true)? else {
        return Ok(ClaimsTail::default());
    };
    let named = |why: std::io::Error| format!("{}: {why}", path.display());
    let len = file.metadata().map_err(named)?.len();
    let from = len.saturating_sub(CLAIMS_TAIL);
    let mut bytes = vec![0_u8; usize::try_from(len.saturating_sub(from)).unwrap_or(0)];
    file.read_exact_at(&mut bytes, from).map_err(named)?;
    let mut lines: Vec<&[u8]> = bytes.split(|&b| b == b'\n').collect();
    let _torn_or_empty = lines.pop();
    let whole = lines.iter().skip(usize::from(from > 0));
    let mut tail = ClaimsTail::default();
    for line in whole.rev() {
        if let Some(entry) = claimed(line) {
            tail.last = Some(entry);
            break;
        }
        tail.unverified = tail.unverified.saturating_add(1);
    }
    Ok(tail)
}

/// The newer of the ledger's healthy last record `last` and the claims'
/// last line whose CRC verifies (IA-10c repair 3, D-6269): the claims line
/// is synced after its record, so a later id in the claims means the ledger
/// at `path` lost records whole, said at `warn` with an alert; whole lines at
/// the claims' tail that fail their CRC are said at `warn` and never read.
///
/// # Errors
///
/// [`RunsLedgerFault::Io`]: claims that are not a regular file or cannot be
/// read refuse the allocation before anything is written.
fn newest(store: &Path, path: &Path, last: Entry) -> Result<Entry, RunsLedgerFault> {
    let tail = claims_tail(store).map_err(|why| RunsLedgerFault::Io { why })?;
    if tail.unverified > 0 {
        note_claims_damaged(store, tail.unverified);
    }
    Ok(match tail.last {
        Some(claimed) if claimed.id > last.id => {
            note_behind_claims(store, path, &last, &claimed);
            claimed
        }
        _ => last,
    })
}

/// Appends run `id`'s claims line to [`CLAIMS`] and syncs it, under the
/// ledger's lock, which every caller holds (D-6269), through the text
/// ledgers' one append (`audit_ledger::append_path`): a regular file only,
/// a torn last line closed first.
///
/// # Errors
///
/// [`RunsLedgerFault::Io`] naming the claims file: the id's record is
/// written and synced, and the id is never handed out; the next allocation
/// goes past it as past any record.
fn append_claim(store: &Path, record: &[u8; 128], id: u64) -> Result<(), RunsLedgerFault> {
    crate::audit_ledger::append_path(&store.join(CLAIMS), &[claim_line(record)]).map_err(|why| {
        RunsLedgerFault::Io {
            why: format!(
                "its claims line was not written ({why}); run {id}'s record is in the ledger \
                 and the id was not handed out"
            ),
        }
    })
}

/// Takes the ledger's lock, allocates one id for `entry` and releases: for a
/// run that writes no claim (a Zerodha pull run, a verify-merge).
///
/// # Errors
///
/// What [`LedgerHold::take`] and [`LedgerHold::allocate`] refuse.
pub fn allocate(store: &Path, entry: &Entry) -> Result<u64, RunsLedgerFault> {
    LedgerHold::take(store, locks::LOCK_WAIT)?.allocate(entry)
}

/// Refuses a clock that cannot be right: before the calendar's first day,
/// more than [`CLOCK_AHEAD_DAYS`] past the last day its sources cover, or
/// more than [`CLOCK_BEHIND_S`] before `last`'s start.
///
/// # Errors
///
/// [`RunsLedgerFault::ClockImplausible`], with an error event.
pub fn clock_check(now: u64, last: Option<&Entry>) -> Result<(), RunsLedgerFault> {
    let today = i64::try_from(now / 86_400).unwrap_or(i64::MAX);
    let why = if today < crate::calendar::FIRST_DAY {
        Some(format!(
            "before the calendar's first day ({} days since 1970)",
            crate::calendar::FIRST_DAY
        ))
    } else if today > crate::calendar::SOURCED_LAST_DAY + CLOCK_AHEAD_DAYS {
        Some(format!(
            "more than {CLOCK_AHEAD_DAYS} days past the last day the calendar's sources cover \
             ({} days since 1970)",
            crate::calendar::SOURCED_LAST_DAY
        ))
    } else {
        last.filter(|last| now.saturating_add(CLOCK_BEHIND_S) < last.started)
            .map(|last| {
                format!(
                    "more than a day before the newest run's start ({}, run {})",
                    last.started, last.id
                )
            })
    };
    match why {
        None => Ok(()),
        Some(why) => {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::error(TARGET, "clock implausible")
                    .with("now", now)
                    .with("why", why.as_str()),
            );
            Err(RunsLedgerFault::ClockImplausible { now, why })
        }
    }
}

/// Says, at `warn`, that the ledger at `path` had one damaged last record at
/// byte `offset`, which this allocation writes over, and the journal's and
/// the claims' largest run ids the new id is raised past (IA-2 repair 2;
/// D-5901, D-6269): one bad record is a torn or rotted append, named rather
/// than silently replaced.
fn note_written_over(path: &Path, offset: u64, floor: Floor) {
    let shown = path.display().to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(TARGET, "runs ledger last record written over")
            .with("ledger", shown.as_str())
            .with("offset", offset)
            .with("journal_run", floor.journal)
            .with("claims_run", floor.claims),
    );
}

/// Says, at `warn` and with an alert (kind `runs_ledger_damaged`, keyed per
/// claimed id, so the event puts one), that the ledger at `path` ends at run
/// `last` while its claims name the later run `claimed` (IA-10c repair 3,
/// D-6269): records the ledger had synced were lost whole, and the new id is
/// raised past `claimed`, never repaired silently and never refused, since
/// the claims still hold every id given.
fn note_behind_claims(store: &Path, path: &Path, last: &Entry, claimed: &Entry) {
    let shown = path.display().to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(TARGET, "runs ledger behind its claims")
            .with("ledger", shown.as_str())
            .with("ledger_run", last.id)
            .with("claims_run", claimed.id),
    );
    let key = format!("runs_ledger_behind_claims:{}", claimed.id);
    let text = format!(
        "ledger ends at run {}; its claims name run {}",
        last.id, claimed.id
    );
    let alert = crate::alerts::NewAlert {
        kind: 8,
        key: &key,
        day: 0,
        run: claimed.id,
        text: &text,
    };
    if let Err(why) = crate::alerts::put(store, &alert) {
        let why = why.to_string();
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::error(TARGET, "runs ledger alert not put")
                .with("key", key.as_str())
                .with("why", why.as_str()),
        );
    }
}

/// Says, at `warn`, that `lines` whole lines at the tail of the claims of
/// `store` fail their CRC (IA-10c repair 3, D-6269): a rotted or foreign
/// line, whose id is never read; the ledger's own last record stands.
fn note_claims_damaged(store: &Path, lines: usize) {
    let shown = store.join(CLAIMS).display().to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn(TARGET, "runs claims tail damaged")
            .with("claims", shown.as_str())
            .with("lines", u64::try_from(lines).unwrap_or(u64::MAX)),
    );
}

/// The error event and the alert of a damaged ledger, keyed per offset so a
/// second refusal of the same damage puts nothing more.
fn note_damaged(store: &Path, path: &Path, offset: u64) {
    let shown = path.display().to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error(TARGET, "runs ledger damaged")
            .with("ledger", shown.as_str())
            .with("offset", offset),
    );
    let key = format!("runs_ledger_damaged:{offset}");
    let text = format!("two bad records at byte {offset}");
    let alert = crate::alerts::NewAlert {
        kind: 8,
        key: &key,
        day: 0,
        run: 0,
        text: &text,
    };
    if let Err(why) = crate::alerts::put(store, &alert) {
        let why = why.to_string();
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::error(TARGET, "runs ledger alert not put")
                .with("offset", offset)
                .with("why", why.as_str()),
        );
    }
}

/// The largest `run` id the GDFL journal of `store` holds, or 0: one pass
/// over the journal, once per store (L-IA-23). A line that does not parse (a
/// torn fragment, a rotted digit run past `u64`) names no id.
///
/// # Errors
///
/// A journal that exists and is not a regular file (a pipe is refused at
/// once, never waited on), or cannot be read.
pub fn largest_journal_run(store: &Path) -> Result<u64, String> {
    // The journal's own writer follows a link at its name, so this reader
    // does too; a pipe or a directory there is refused.
    largest_line(&crate::gdfl_import::journal_path(store), false, |line| {
        let id = line.strip_prefix(b"run ")?.split(|&b| b == b' ').next()?;
        std::str::from_utf8(id).ok()?.parse::<u64>().ok()
    })
}

/// The largest id the claims of `store` hold ([`CLAIMS`], D-6269), or 0:
/// one pass, taken only when the ledger cannot say its last id. Only a line
/// whose CRC verifies names an id ([`claimed`], repair round 3).
///
/// # Errors
///
/// Claims that exist and are not a regular file (a link, a pipe, a
/// directory), or cannot be read.
pub fn largest_claimed_run(store: &Path) -> Result<u64, String> {
    largest_line(&store.join(CLAIMS), true, |line| {
        claimed(line).map(|entry| entry.id)
    })
}

/// The largest id `id_of` reads off the lines of the text file at `path`, or
/// 0 when it names none or the file is not there; read through
/// [`open_regular`], so a pipe planted at the name never holds the ledger's
/// lock waiting for a writer (the second agent's repair-round-3 probe P4c).
fn largest_line(
    path: &Path,
    no_link: bool,
    id_of: impl Fn(&[u8]) -> Option<u64>,
) -> Result<u64, String> {
    use std::io::Read as _;
    let Some(mut file) = open_regular(path, no_link)? else {
        return Ok(0);
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(bytes
        .split(|&b| b == b'\n')
        .filter_map(id_of)
        .max()
        .unwrap_or(0))
}

/// Opens the text file at `path` to read the ids it holds, `None` when it is
/// not there (IA-10c repair 3): non-blocking, so a pipe at the name opens at
/// once and is refused by the handle's own `fstat` rather than waited on
/// under the ledger's lock; never through a link when `no_link` (the claims,
/// whose one writer refuses a link too).
///
/// # Errors
///
/// A link (when `no_link`), something other than a regular file, or an open
/// that fails for any other reason, each naming the path.
fn open_regular(path: &Path, no_link: bool) -> Result<Option<File>, String> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let flags = if no_link {
        store::open_flags::O_NOFOLLOW_NONBLOCK
    } else {
        store::open_flags::O_NONBLOCK
    };
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)
    {
        Ok(file) => file,
        Err(why) if why.kind() == ErrorKind::NotFound => return Ok(None),
        Err(why) => {
            return Err(format!(
                "{}: {why}; its ids are read only from a regular file{}",
                path.display(),
                if no_link {
                    ", never through a link"
                } else {
                    ""
                }
            ));
        }
    };
    let regular = file
        .metadata()
        .map_err(|why| format!("{}: {why}", path.display()))?
        .is_file();
    if regular {
        Ok(Some(file))
    } else {
        Err(format!(
            "{}: not a regular file, so no id is read from it and nothing is written",
            path.display()
        ))
    }
}

/// Every whole record of the ledger of `store`, for a reader (the runs page,
/// a test): `Err` names the offset of a record that does not verify.
///
/// # Errors
///
/// The ledger cannot be read.
pub fn read_all(store: &Path) -> Result<Vec<Result<Entry, u64>>, String> {
    let path = store.join(LEDGER);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(why) if why.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{}: {why}", path.display())),
    };
    Ok(bytes
        .chunks_exact(128)
        .zip((0_u64..).step_by(128))
        .map(|(record, at)| Entry::of(record).ok_or(at))
        .collect())
}
