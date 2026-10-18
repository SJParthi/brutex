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
//! (one positional read of 128 bytes), takes `id = max(now, last + 1)`,
//! writes its record at ⌊len ÷ 128⌋ × 128, over any torn tail, and syncs.
//!
//! A crash's leftovers (round 4, SF-12): bytes past the last whole record are
//! a torn append and are written over; a last whole record whose CRC fails is
//! an append that never finished its sync, so its id was never used: the
//! record before it is read (a second 128-byte read), `last` is taken from
//! it, and the new record is written at L over the bad one. A ledger shorter
//! than one record, or whose only record fails its CRC, is unborn and is
//! seeded again. TWO bad records at the tail cannot come from one interrupted
//! append: [`RunsLedgerFault::Damaged`] names the offset, an error event and
//! an alert (kind `runs_ledger_damaged`, keyed per offset) are put, and no
//! run of either feed starts until the operator has looked: never skipped,
//! never repaired silently.
//!
//! **Seeding** (L-IA-23): the first record of an unborn ledger is
//! `<n> seed - - 0 <now>`, where `n` is the largest `run` id the GDFL journal
//! holds (one pass over the journal, once per store), so no id a journal line
//! already carries is given again.
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
/// One record's bytes.
pub const RECORD: u64 = 128;
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
    /// the clock), writes its record and syncs it. O(1): one or two 128-byte
    /// reads and one write (seeding once per store); measured by
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
        // (offset to write at, the last record's id and start; `None` when
        // the ledger is unborn and is seeded at 0).
        let (mut at, last) = if whole == 0 {
            (0, None)
        } else {
            let tail = (whole - 1) * RECORD;
            match read(tail)? {
                Some(last) => (whole * RECORD, Some(last)),
                None if whole == 1 => (0, None),
                None => {
                    let Some(before) = read(tail - RECORD)? else {
                        let offset = tail - RECORD;
                        note_damaged(&self.store, &path, offset);
                        return Err(RunsLedgerFault::Damaged { offset });
                    };
                    (tail, Some(before))
                }
            }
        };
        let now = entry.started;
        clock_check(now, last.as_ref())?;
        let mut bytes = Vec::with_capacity(256);
        let last_id = if let Some(last) = last {
            last.id
        } else {
            let seed = Entry {
                id: largest_journal_run(&self.store).map_err(|why| RunsLedgerFault::Io { why })?,
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
        let id = now.max(last_id.saturating_add(1));
        let mine = Entry {
            id,
            ..entry.clone()
        };
        bytes.extend_from_slice(
            &mine
                .record()
                .map_err(|why| RunsLedgerFault::Entry { why })?,
        );
        file.write_all_at(&bytes, at).map_err(io)?;
        file.sync_all().map_err(io)?;
        Ok(id)
    }
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
/// over the journal, once per store (L-IA-23).
///
/// # Errors
///
/// A journal that exists and cannot be read.
pub fn largest_journal_run(store: &Path) -> Result<u64, String> {
    let path = crate::gdfl_import::journal_path(store);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(why) if why.kind() == ErrorKind::NotFound => return Ok(0),
        Err(why) => return Err(format!("{}: {why}", path.display())),
    };
    Ok(bytes
        .split(|&b| b == b'\n')
        .filter_map(|line| line.strip_prefix(b"run "))
        .filter_map(|rest| {
            let id = rest.split(|&b| b == b' ').next()?;
            std::str::from_utf8(id).ok()?.parse::<u64>().ok()
        })
        .max()
        .unwrap_or(0))
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
