//! The gap ledger for months already stored: one `gap` line per session the
//! store holds short, from the store and the calendar, with no vendor call.
//! R3-18; findings DB-A12, DB-G46, CMP-19; failure-matrix row FM-166.
//!
//! # Why it exists
//!
//! `<store>/audit/<feed>.gaps` (D-3335) is written by a pull as each member
//! lands (`request_minutes`). Every month landed before that ledger existed —
//! and every line a rolling log or a cut-off run lost (DB-G46) — has none, so a
//! stored hole such as NIFTY 2019-09-23 15:15–15:28 read on /db as "no
//! ledger" with no reason at all. This writes those lines from what the store
//! holds.
//!
//! # What it writes
//!
//! For every held month of a series' `1min` rung inside the asked window, the
//! repair plan ([`super::repair_plan`]) names each session that holds no bar
//! and each that holds fewer of its owed minutes than the calendar owes, with
//! what can fill it. One line is written per such session whose fill is
//! `front` or `interior` (only a versioned repair fills it, and a repair writes
//! its own `repaired` line that supersedes this one), or `tail` in a month
//! before the series' newest held month (a stored hole at a month's end):
//!
//! ```text
//! gap NSE-INDEX-NIFTY 2019-09-23 v=1 rung=1min owed=375 held=361 missing=14 first_missing=15:15 runs=1 longest=14 longest_at=15:15 why=stored_short run=0 fill=interior source=store
//! ```
//!
//! D-3335's fields in D-3335's order, counted by the same counter the vendor
//! audit uses; `why=stored_short` says the store was read, not the vendor, so
//! no vendor reason is claimed; `fill=` and `source=` follow, which every
//! reader of the ledger ignores by name.
//!
//! **Never written**, each counted in [`Tally`] and printed:
//!
//! - a session the ledger already holds a line for: that line's reason stands,
//!   because the last line of a key wins and a store-found line would hide the
//!   vendor's reason ([`Tally::already`]);
//! - a `before_first` session: a listing, the start of the feed's history or a
//!   hole, which the bars cannot tell apart ([`Tally::before_first`]);
//! - a `tail` session of the series' NEWEST held month: the frontier an
//!   ordinary pull extends, and a pull that lands a whole day writes no line,
//!   so a line there would outlive its hole ([`Tally::frontier`]);
//! - a session whose owed minutes no record states, or a day no calendar
//!   attests ([`Tally::length_unjudged`], [`Tally::unattested`]).
//!
//! # Idempotent, and never over another writer's line
//!
//! The ledger's `(key, day)` pairs are read first ([`ledgered`]), and the
//! byte where that read stopped is kept. Each series is then written under
//! the ledger's exclusive lock, which every writer of the file takes
//! (`request_minutes::append_named_picking`): the lines appended since that
//! byte — by a pull, a repair or a second `--apply` — are read under the lock
//! first, and every line whose `(key, day)` they now name is dropped before
//! the append ([`Tally::raced`]). So a store-found line never follows another
//! writer's line for the same key and day, two concurrent runs write each
//! line once, and a second run finds every line the first wrote and writes
//! nothing. Bars, census and sidecars are never opened for writing.
//!
//! # Cost
//!
//! One pass over the ledger (O(its lines), once per run); one listing of each
//! series' rung directory; per held month the plan's constant lookups per
//! day; per short session one read of at most its owed minutes (375 on a
//! regular day). O(days held + short minutes), never a vendor call. One append
//! and one sync per series written.

use std::collections::HashSet;
use std::io::BufRead;
use std::path::Path;

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::path::{PathParts, StorePath, Timeframe, YearMonth};

use super::repair_plan::{self, Missing, Owes};
use super::request_minutes;
use crate::session::{Day, Window};

/// The `(key, day)` pairs the ledger already holds a `gap` line for, and
/// where the read that found them stopped.
#[derive(Debug, Clone, Default)]
pub struct Have {
    pairs: HashSet<(String, u32)>,
    /// Lines that are not a well-formed `gap` or another record kind, by
    /// line number: named, never silently skipped.
    pub faults: Vec<u64>,
    /// Bytes of the ledger read, up to the end of its last whole line: the
    /// next read under the lock starts here.
    read_to: u64,
    /// Whole lines read, so a fault found later keeps its line number.
    lines: u64,
}

impl Have {
    /// Whether the ledger holds a line for `key` on `day`. O(1) expected: one
    /// hash probe. Proof: `crates/pull/tests/zerodha_gaps.rs`
    /// (`a_ledger_probe_is_flat_in_the_ledgers_lines`, 790 against 79,000 lines).
    #[must_use]
    pub fn holds(&self, key: &str, day: Day) -> bool {
        self.pairs
            .contains(&(key.to_owned(), day.days_from_epoch()))
    }

    /// How many `(key, day)` pairs it holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether it holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

/// `<store>/audit/<vendor>.gaps`.
#[must_use]
pub fn ledger_path(root: &Path, vendor: Vendor) -> std::path::PathBuf {
    root.join("audit").join(format!("{}.gaps", vendor.as_str()))
}

/// A day as `YYYY-MM-DD`, or `None`.
fn parse_day(word: &str) -> Option<Day> {
    let mut parts = word.split('-');
    let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return None;
    }
    Day::new(year.parse().ok()?, month.parse().ok()?, day.parse().ok()?).ok()
}

/// Reads the ledger once: every `(key, day)` a `gap` line names. A ledger
/// that is not there holds none. Opened through the store's one read door
/// (`store::safe_open::open_read`, R3-16): a link, a pipe or a directory at
/// the name is refused by name, never followed or waited on.
///
/// # Errors
///
/// The host's words for a ledger that exists and cannot be read, or what is
/// at the name when it is not a regular file.
pub fn ledgered(root: &Path, vendor: Vendor) -> Result<Have, String> {
    let path = ledger_path(root, vendor);
    let file = match store::safe_open::open_read(&path) {
        Ok(file) => file,
        Err(why) if why.is_missing() => return Ok(Have::default()),
        Err(why) => return Err(format!("{}: {why}", path.display())),
    };
    let mut have = Have::default();
    scan(&mut have, std::io::BufReader::new(file))
        .map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(have)
}

/// Every line `reader` holds, read into `have` from where its last read
/// stopped. A last line with no line break — a writer killed mid-line — is
/// read for its `(key, day)` (so nothing is written over it) but not counted
/// as read: the next scan reads it again, closed. A line that is not UTF-8
/// is a fault, named by its number.
fn scan(have: &mut Have, mut reader: impl BufRead) -> std::io::Result<()> {
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        let taken = reader.read_until(b'\n', &mut bytes)?;
        if taken == 0 {
            return Ok(());
        }
        let whole = bytes.last() == Some(&b'\n');
        let number = have.lines.saturating_add(1);
        let text = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
        match std::str::from_utf8(text) {
            Ok(line) if line.starts_with("gap ") => {
                let mut words = line.split(' ').skip(1);
                match (words.next(), words.next().and_then(parse_day)) {
                    (Some(key), Some(day)) if !key.is_empty() && !key.contains('=') => {
                        have.pairs.insert((key.to_owned(), day.days_from_epoch()));
                    }
                    _ => fault(have, number),
                }
            }
            Ok(_) => {}
            Err(_) => fault(have, number),
        }
        if whole {
            have.lines = number;
            have.read_to = have
                .read_to
                .saturating_add(u64::try_from(taken).unwrap_or(u64::MAX));
        }
    }
}

/// `number` named as a fault once, however often a torn last line is read.
fn fault(have: &mut Have, number: u64) {
    if have.faults.last() != Some(&number) {
        have.faults.push(number);
    }
}

/// What one series' stored months owe the ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tally {
    /// Held months read.
    pub months: u64,
    /// Sessions of record inside them.
    pub sessions: u64,
    /// Sessions holding no bar or short of their owed minutes.
    pub short: u64,
    /// Of those, the ones the ledger already holds a line for.
    pub already: u64,
    /// Lines found but not written because another writer — a pull, a
    /// repair, another run — appended a line for the same `(key, day)` after
    /// this run read the ledger: read under the ledger's lock, so its reason
    /// stands.
    pub raced: u64,
    /// Of those, `before_first`: never written.
    pub before_first: u64,
    /// Of those, `tail` sessions of the newest held month: never written.
    pub frontier: u64,
    /// Held sessions whose owed minutes no record states.
    pub length_unjudged: u64,
    /// Days no calendar of record attests.
    pub unattested: u64,
    /// Minutes missing on the sessions a line is written for.
    pub minutes: u64,
    /// Months that could not be read, each with why: nothing about them is
    /// claimed.
    pub refused: Vec<String>,
}

/// One series' lines, not yet written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Series {
    /// The ledger key, `<exchange>-<segment>-<symbol>`.
    pub key: String,
    /// The lines, oldest day first.
    pub lines: Vec<String>,
    /// The days they are for, in the same order.
    pub days: Vec<Day>,
    /// The counts.
    pub tally: Tally,
}

/// The lines `parts`' series owes the ledger for its held `1min` months that
/// touch `window`, none written. `parts.month` and `parts.timeframe` are
/// ignored: every held month of the minute rung is read.
#[must_use]
pub fn series(root: &Path, parts: PathParts<'_>, window: Window, have: &Have) -> Series {
    let parts = PathParts {
        timeframe: Timeframe::MINUTE_1,
        ..parts
    };
    let key = request_minutes::ledger_key(parts.exchange, parts.segment, parts.symbol);
    let held = repair_plan::held_months(root, parts);
    let symbol_id = repair_plan::symbol_id(parts.symbol);
    let mut out = Series {
        key,
        ..Series::default()
    };
    let (Some(&first), Some(&newest)) = (held.first(), held.last()) else {
        return out;
    };
    for &month in &held {
        let Some((from, to)) = clip(month, window) else {
            continue;
        };
        let parts = PathParts { month, ..parts };
        let plan = repair_plan::plan_month(root, parts, symbol_id, (from, to, month == first));
        out.tally.months += 1;
        if let Some(why) = &plan.refused {
            out.tally.refused.push(format!("{month}: {why}"));
            continue;
        }
        out.tally.sessions += u64::from(plan.sessions);
        out.tally.length_unjudged += u64::try_from(plan.length_unjudged.len()).unwrap_or(u64::MAX);
        out.tally.unattested += u64::try_from(plan.unattested.len()).unwrap_or(u64::MAX);
        let mut wanted: Vec<(Day, Missing)> = plan
            .missing
            .iter()
            .copied()
            .chain(plan.short.iter().map(|short| (short.day, short.fill)))
            .collect();
        wanted.sort_unstable_by_key(|(day, _)| day.days_from_epoch());
        if let Err(why) = month_lines(
            root,
            parts,
            symbol_id,
            (&wanted, month == newest),
            have,
            &mut out,
        ) {
            out.tally.refused.push(format!("{month}: {why}"));
        }
    }
    out
}

/// `month` clipped to `window`, or `None` when they do not meet.
fn clip(month: YearMonth, window: Window) -> Option<(Day, Day)> {
    let first = Day::new(month.year(), month.month(), 1).ok()?;
    let (from, to) = (
        first.max(window.from()),
        first.end_of_month().min(window.to()),
    );
    (from <= to).then_some((from, to))
}

/// The lines of one held month's `wanted` sessions, appended to `out`.
fn month_lines(
    root: &Path,
    parts: PathParts<'_>,
    symbol_id: u32,
    (wanted, newest): (&[(Day, Missing)], bool),
    have: &Have,
    out: &mut Series,
) -> Result<(), String> {
    let mut file: Option<BarFile> = None;
    for &(day, fill) in wanted {
        out.tally.short += 1;
        match fill {
            Missing::BeforeFirst => {
                out.tally.before_first += 1;
                continue;
            }
            Missing::Tail | Missing::Absent if newest => {
                out.tally.frontier += 1;
                continue;
            }
            Missing::Absent | Missing::Front | Missing::Interior | Missing::Tail => {}
        }
        if have.holds(&out.key, day) {
            out.tally.already += 1;
            continue;
        }
        let Owes::Windows((windows, count)) =
            repair_plan::owed(parts.segment, day.days_from_epoch())
        else {
            out.tally.length_unjudged += 1;
            continue;
        };
        let windows = windows.get(..count).unwrap_or(&[]);
        if file.is_none() {
            let path = StorePath::new(parts).map_err(|why| why.to_string())?;
            file =
                Some(BarFile::open_existing(root, path, symbol_id).map_err(|why| why.to_string())?);
        }
        let Some(open) = file.as_ref() else {
            continue;
        };
        let stamps = held_stamps(open, day, windows)?;
        if let Some((mut line, missing)) =
            request_minutes::stored_line(&out.key, day, windows, stamps.into_iter(), fill.code())
        {
            // A SHARE'S MISSING MINUTE MAY BE ONE NOBODY TRADED (D-3321): said
            // on the line, never decided from the store alone.
            if repair_plan::absence_may_be_no_trade(parts.segment) {
                line.push_str(" absence=may_be_no_trade");
            }
            out.tally.minutes = out.tally.minutes.saturating_add(u64::from(missing));
            out.lines.push(line);
            out.days.push(day);
        }
    }
    Ok(())
}

/// The stamps of the bars `file` holds inside `day`'s owed `windows`, UTC
/// micros, ascending: two lookups per window and at most its owed minutes
/// read.
fn held_stamps(file: &BarFile, day: Day, windows: &[(u32, u32)]) -> Result<Vec<i64>, String> {
    let bars = file.header().n_valid;
    let base = repair_plan::day_base(day.days_from_epoch());
    let mut stamps = Vec::new();
    for &(from, to) in windows {
        let (start, end) = (
            base + i64::from(from) * 60_000_000,
            base + i64::from(to) * 60_000_000,
        );
        let (records, first) = repair_plan::count(file, bars, start, end)?;
        for index in first..first.saturating_add(records) {
            stamps.push(
                file.read_record(index)
                    .map_err(|why| why.to_string())?
                    .ts_micros,
            );
        }
    }
    Ok(stamps)
}

/// Appends `series`' lines to the ledger in one write and one sync, records
/// them in `have` so a second call writes nothing, and says so in one
/// `pull.gap_backfill` event. Answers how many lines were written.
///
/// UNDER THE LEDGER'S EXCLUSIVE LOCK, the lines appended since `have`'s read
/// stopped are read first and every line whose `(key, day)` they name is
/// dropped ([`Tally::raced`]): the other writer's line stands, and is never
/// followed by a store-found one for the same day.
///
/// # Errors
///
/// The ledger's own refusal: a directory, open, lock, read, write or sync
/// that failed. Nothing is written then.
pub fn write(
    root: &Path,
    vendor: Vendor,
    series: &mut Series,
    have: &mut Have,
) -> Result<u64, String> {
    let mut kept_days: Vec<Day> = Vec::new();
    let mut raced = 0_u64;
    let written = request_minutes::append_named_picking(root, vendor, "gaps", |ledger| {
        use std::io::{Seek as _, SeekFrom};
        ledger
            .seek(SeekFrom::Start(have.read_to))
            .and_then(|_| scan(have, std::io::BufReader::new(&mut *ledger)))
            .map_err(|why| format!("{}: {why}", ledger_path(root, vendor).display()))?;
        let mut kept = Vec::with_capacity(series.lines.len());
        for (line, &day) in series.lines.iter().zip(&series.days) {
            if have.holds(&series.key, day) {
                raced += 1;
            } else {
                kept.push(line.clone());
                kept_days.push(day);
            }
        }
        Ok(std::borrow::Cow::Owned(kept))
    });
    series.tally.raced += raced;
    let lines = u64::try_from(kept_days.len()).unwrap_or(u64::MAX);
    let event = match &written {
        Ok(_) => telemetry::Event::info("pull.gap_backfill", "stored-month gap lines written"),
        Err(_) => {
            telemetry::Event::error("pull.gap_backfill", "stored-month gap lines NOT written")
        }
    };
    let reason = written.as_ref().err().cloned().unwrap_or_default();
    let _dropped_when_filtered = telemetry::emit(
        &event
            .with("feed", telemetry::Value::Str(vendor.as_str()))
            .with("instrument", telemetry::Value::Str(&series.key))
            .with("lines", telemetry::Value::Uint(lines))
            .with("raced", telemetry::Value::Uint(raced))
            .with("months", telemetry::Value::Uint(series.tally.months))
            .with("minutes", telemetry::Value::Uint(series.tally.minutes))
            .with("reason", telemetry::Value::Str(&reason)),
    );
    let (end, closed) = written?;
    for day in &kept_days {
        have.pairs
            .insert((series.key.clone(), day.days_from_epoch()));
    }
    // EVERY BYTE UP TO THE END IS NOW READ OR WRITTEN BY THIS RUN, AND WHOLE
    // LINES: the append ended in a line break, and a torn last line it closed
    // is one more line.
    if lines > 0 {
        have.lines = have
            .lines
            .saturating_add(lines)
            .saturating_add(u64::from(closed));
        have.read_to = end;
    }
    Ok(lines)
}
