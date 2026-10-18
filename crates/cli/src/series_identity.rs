//! `cli series-identity` — whose bars a stored series holds. WP-13d; findings
//! DB-B07, DB-B08, DB-B10, DB-B22, DB-B23, DB-B60, DB-B64, DB-B66. R4-17:
//! RT-DATA-02 b, Z6, the PIIND quarantine plan.
//!
//! # Usage
//!
//! ```text
//! cli series-identity INSTRUMENT [--segment INDEX|CASH] [--feed FEED] [--dry-run]
//! cli series-identity ALL [--segment INDEX|CASH] [--feed FEED] [--workers N] [--dry-run]
//! ```
//!
//! `--segment` is `INDEX` unless named; `--feed` is `zerodha` unless named;
//! `--dry-run` names the plan it would write and writes nothing.
//! `ALL` scans the series in parallel, one series per worker at a time, on
//! as many workers as the machine reports (`--workers` names fewer, never
//! more than one per series); the table, the counts and the plan are the same
//! bytes for every worker count, because each series is judged alone and the
//! results are joined in name order.
//!
//! # What it does
//!
//! **It writes no bar.** Every stored bar of the series is judged by the rules
//! the landing door applies to a broker's answer
//! (`pull::ingest::identity_screen`): each 1day bar by the day-rung arm (a
//! price at or below zero, a CASH day before its master's listing day), each
//! 1min bar against the series' own stored 1day bars, its listing and, for a
//! CASH share, the minute kept before it in the same continuous run (an
//! in-session jump beyond `pull::series_identity::JUMP_BPS`). Months already on
//! disk hold such bars until a generation without them is published, so this
//! verb prints, per month, how many, and each (day, cause).
//!
//! Each flagged bar is then given a MEASURED cause ([`Measured`]): a minute
//! the day rung or its run accuses is compared with GDFL's 1s bars of the same
//! share folded to that minute (`pull::fold`, the one fold authority). Within
//! [`WITNESS_TOLERANCE_BPS`] of GDFL's close it is `basis_mismatch` — the
//! vendor's minute is GDFL's, and its day bar is what was adjusted apart: a
//! label, never quarantined. Matching neither it is `foreign_level`. With no
//! GDFL minute to compare — GDFL holds no such month, or the month holds no
//! such minute — it is `no_gdfl_witness`. A GDFL month that is there and will
//! not open, read or fold is NOT absence: its bars are `gdfl_unread`, the plan
//! asks `remeasure` for them, and the month is named `NOT READ` and fails the
//! run (Z6 repair 1). A price at or below zero is `zero_price`; a bar before
//! its listing `before_listing`.
//!
//! **It writes one file**, and no bar: the QUARANTINE PLAN, under
//! `<store>/audit/series-identity/`, naming the commit that measured it and,
//! per (key, rung, month, measured cause), the action, the count, and either
//! the whole month or each stamp. Its name carries the BLAKE3 of its content,
//! so a rerun of the same build over the same store finds it already there
//! and writes nothing (`CLAUDE.md` §3 rule 5). The verb is therefore a store
//! writer (`crate::store_write::WRITERS`): an unstamped build is refused
//! before anything is read unless `BRUTEX_ALLOW_UNSTAMPED_WRITE=1`, and a
//! store whose `bars` is a link is refused by name (DB-B71, RA-298).
//! `--dry-run` is its read-only form: the same scan and report, the plan's
//! path named, nothing written, so a shadow can be scanned. A CASH scan also
//! prints each share's in-session jump distribution (p50, p99, p999, max):
//! the measurement `JUMP_BPS` is to be recorded from.
//!
//! # Exit
//!
//! [`OK`] when no stored bar would be refused and every month of both rungs
//! and of the GDFL witness opened; [`FAILED`] otherwise — any in-session jump
//! among them; [`MISUSED`] for arguments that name no series.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::ingest::identity_screen::{FlaggedBar, SeriesScan, scan_series};
use pull::series_identity::{JUMP_BPS, Tally};
use pull::session::Day;
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str = "cli series-identity INSTRUMENT|ALL [--segment INDEX|CASH] [--feed FEED] [--workers N] [--dry-run]";

/// How far a vendor minute's close may lie from GDFL's folded minute's close,
/// in basis points of GDFL's, and still be GDFL's minute.
///
/// # Why 100
///
/// Chosen, not measured, and stated so: two vendors' minutes of one share are
/// folded from different tick feeds and their closes may differ by a tick or
/// two, while an accused minute that is another price basis differs by the
/// basis factor — PIIND's interleave by a factor above nine. 100 bps sits far
/// from both. It decides a LABEL only: `basis_mismatch` against
/// `foreign_level`; no landing rule reads it.
pub(crate) const WITNESS_TOLERANCE_BPS: i64 = 100;

/// The plan file's directory under the store root.
const PLAN_DIR: [&str; 2] = ["audit", "series-identity"];

/// The plan file's first line: its format and version.
///
/// `v2` (R4-17 repair 1) adds the `commit` line and the `gdfl_unread` cause
/// with its `remeasure` action. `v1` was never on `feat/db-fix`: its plans
/// exist only in test shadows, and a `v2` reader does not read them as `v2`.
const PLAN_HEAD: &str = "brutex series-identity quarantine plan v2";

/// What the arguments name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    /// `None` for `ALL`.
    instrument: Option<String>,
    segment: &'static str,
    feed: Vendor,
    /// `--workers`: at most this many series scanned at once by `ALL`.
    workers: Option<usize>,
    /// `--dry-run`: the plan is named, never written.
    dry_run: bool,
}

fn ask(arguments: &[&str]) -> Result<Ask, String> {
    let [first, flags @ ..] = arguments else {
        return Err("name an instrument, or ALL".to_owned());
    };
    let instrument = if *first == "ALL" {
        None
    } else {
        brutex_core::symbol::Symbol::new(first).map_err(|why| format!("`{first}`: {why}"))?;
        Some((*first).to_owned())
    };
    let (mut segment, mut feed, mut workers, mut dry_run) = ("INDEX", Vendor::Zerodha, None, false);
    let mut rest = flags.iter();
    while let Some(flag) = rest.next() {
        if *flag == "--dry-run" {
            dry_run = true;
            continue;
        }
        let value = rest.next().ok_or_else(|| format!("{flag} takes a value"))?;
        match *flag {
            "--segment" => {
                segment = match *value {
                    "INDEX" => "INDEX",
                    "CASH" => "CASH",
                    other => return Err(format!("--segment `{other}`: INDEX or CASH")),
                };
            }
            "--feed" => {
                feed = Vendor::ALL
                    .into_iter()
                    .find(|vendor| vendor.as_str() == *value)
                    .ok_or_else(|| format!("--feed `{value}`: not a feed this store names"))?;
            }
            "--workers" => {
                workers = Some(
                    value
                        .parse::<usize>()
                        .ok()
                        .filter(|count| *count > 0)
                        .ok_or_else(|| format!("--workers `{value}`: a whole number above zero"))?,
                );
            }
            other => return Err(format!("`{other}` is not a flag of this verb")),
        }
    }
    Ok(Ask {
        instrument,
        segment,
        feed,
        workers,
        dry_run,
    })
}

/// How many series `ALL` scans at once: `--workers` when named, else every
/// core the machine reports, and never more than one per series.
fn worker_count(asked: Option<usize>, series: usize) -> usize {
    asked
        .unwrap_or_else(|| {
            std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
        })
        .min(series)
        .max(1)
}

/// Runs the verb against the store root `store` names, as the build stamped
/// `commit` (`None`: unstamped), so every arm is driven by a test without the
/// process environment. `store` is the writer's gate: the binary hands
/// [`crate::store_write::root_for`], which answers an unstamped writing
/// invocation's refusal instead of the root.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    commit: Option<&str>,
    out: &mut String,
) -> u8 {
    let ask = match ask(arguments) {
        Ok(ask) => ask,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}\nusage: {USAGE}");
            return MISUSED;
        }
    };
    let root = match store() {
        Ok(root) => root,
        Err(why) => {
            let _ = writeln!(
                out,
                "SERIES IDENTITY REFUSED: {why}. Nothing was read and no plan was written."
            );
            return FAILED;
        }
    };
    let run = Run {
        root: &root,
        ask: &ask,
        commit: commit.unwrap_or(UNSTAMPED),
    };
    match &ask.instrument {
        Some(symbol) => one(&run, symbol, out),
        None => all(&run, out),
    }
}

/// `cli series-identity`: see the module header. A STORE WRITER: the root
/// comes through the writer's gate, and the plan names this build's commit.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    command_with(
        arguments,
        &crate::store_write::root_for("series-identity", arguments),
        crate::commit_stamp(),
        out,
    )
}

/// What a plan written by a build with no commit stamp names as its commit:
/// written only under `BRUTEX_ALLOW_UNSTAMPED_WRITE=1`, never disguised.
const UNSTAMPED: &str = "unstamped";

/// One invocation: its root, its arguments and the commit its plan names.
struct Run<'a> {
    root: &'a Path,
    ask: &'a Ask,
    commit: &'a str,
}

/// The series' path parts in `month` and `timeframe`, on `feed`.
fn parts<'a>(
    ask: &'a Ask,
    feed: Vendor,
    symbol: &'a str,
    timeframe: Timeframe,
    month: YearMonth,
) -> PathParts<'a> {
    PathParts {
        vendor: feed,
        exchange: "NSE",
        segment: ask.segment,
        symbol,
        contract: None,
        timeframe,
        month,
        file: FileKind::Bars,
    }
}

fn scan(root: &Path, ask: &Ask, symbol: &str) -> Option<SeriesScan> {
    let month = YearMonth::new(2000, 1).ok()?;
    Some(scan_series(
        root,
        parts(ask, ask.feed, symbol, Timeframe::MINUTE_1, month),
    ))
}

fn total(scan: &SeriesScan) -> Tally {
    let mut sum = Tally::default();
    for (_, tally) in &scan.months {
        let mut counts = tally.clone();
        counts.flagged.clear();
        sum.absorb(counts);
    }
    sum
}

/// Day bars the day-rung arm refuses, across every day month.
fn day_refused(scan: &SeriesScan) -> u64 {
    scan.day_verdicts
        .iter()
        .map(|(_, tally)| tally.refused())
        .sum()
}

fn span(months: &[YearMonth]) -> String {
    match (months.first(), months.last()) {
        (Some(first), Some(last)) => format!("{} months {first}..{last}", months.len()),
        _ => "none".to_owned(),
    }
}

fn day_text(day: i64) -> String {
    u32::try_from(day)
        .ok()
        .and_then(|days| Day::from_days(days).ok())
        .map_or_else(|| format!("IST day {day}"), |day| day.to_string())
}

/// Months holding at least one minute a landing refuses.
fn refused_months(scan: &SeriesScan) -> Vec<YearMonth> {
    scan.months
        .iter()
        .filter(|(_, tally)| tally.refused() > 0)
        .map(|(month, _)| *month)
        .collect()
}

/// A flagged bar's cause as measured against a second vendor (Z6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Measured {
    /// A price at or below zero.
    ZeroPrice,
    /// A bar before its master's listing day.
    BeforeListing,
    /// A minute before the series' first served day bar, where no master
    /// states its listing (WP-13d's rule).
    BeforeFirstDayBar,
    /// Within [`WITNESS_TOLERANCE_BPS`] of GDFL's folded minute: the vendor's
    /// minute is right and its day bar was adjusted apart. A label only.
    BasisMismatch,
    /// Matches neither its own served day nor GDFL's minute.
    ForeignLevel,
    /// GDFL holds no minute to compare it with: no such month, or no such
    /// minute in it.
    NoGdflWitness,
    /// GDFL's month is there and would not open, read or fold: the bar was
    /// not measured, and nothing may be decided about it until it is.
    GdflUnread,
}

impl Measured {
    /// The code the plan and the report carry.
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::ZeroPrice => "zero_price",
            Self::BeforeListing => "before_listing",
            Self::BeforeFirstDayBar => "before_first_day_bar",
            Self::BasisMismatch => "basis_mismatch",
            Self::ForeignLevel => "foreign_level",
            Self::NoGdflWitness => "no_gdfl_witness",
            Self::GdflUnread => "gdfl_unread",
        }
    }

    /// What the plan asks for a bar of this cause: `quarantine`, or `label`
    /// for a minute GDFL vouches for, for a suspect one nothing accuses, and
    /// for an index's zero, which a landing holds (D-4992); `remeasure` for a
    /// bar whose witness would not open, which is neither until it is read.
    const fn action(self, refused: bool) -> &'static str {
        match self {
            Self::GdflUnread => "remeasure",
            Self::BasisMismatch => "label",
            Self::NoGdflWitness | Self::ZeroPrice if !refused => "label",
            _ => "quarantine",
        }
    }

    const ALL: [Self; 7] = [
        Self::ZeroPrice,
        Self::BeforeListing,
        Self::BeforeFirstDayBar,
        Self::BasisMismatch,
        Self::ForeignLevel,
        Self::NoGdflWitness,
        Self::GdflUnread,
    ];
}

/// GDFL's 1s bars of one share, folded to minutes, one month at a time.
struct Witness<'a> {
    root: &'a Path,
    ask: &'a Ask,
    symbol: &'a str,
    /// Each month asked for: its folded minutes, or why it would not open,
    /// read or fold.
    months: BTreeMap<YearMonth, Result<BTreeMap<i64, Bar>, String>>,
}

impl Witness<'_> {
    /// GDFL's folded minute stamped `ts_micros` of `month`: `Ok(None)` when
    /// GDFL holds no such month or minute, `Err` when its month is there and
    /// could not be read — never the one taken for the other.
    fn minute(&mut self, month: YearMonth, ts_micros: i64) -> Result<Option<Bar>, ()> {
        let (root, ask, symbol) = (self.root, self.ask, self.symbol);
        match self
            .months
            .entry(month)
            .or_insert_with(|| gdfl_minutes(root, ask, symbol, month))
        {
            Ok(minutes) => Ok(minutes.get(&ts_micros).copied()),
            Err(_) => Err(()),
        }
    }

    /// Every GDFL month that was asked for and would not open, read or fold,
    /// ascending, each with why: the scan's `NOT READ` lines.
    fn unread(&self) -> Vec<(YearMonth, String)> {
        self.months
            .iter()
            .filter_map(|(month, read)| {
                read.as_ref()
                    .err()
                    .map(|why| (*month, format!("gdfl 1s witness: {why}")))
            })
            .collect()
    }
}

/// Every GDFL 1s bar of `symbol` in `month`, folded to minutes by
/// `pull::fold`. ONLY a month GDFL lacks (`StoreError::Missing`) witnesses
/// nothing; a month that is there and will not open, read or fold answers
/// why, so it is named and never measured as absence (Z6 repair 1).
fn gdfl_minutes(
    root: &Path,
    ask: &Ask,
    symbol: &str,
    month: YearMonth,
) -> Result<BTreeMap<i64, Bar>, String> {
    let id = pull::ingest::repair_plan::symbol_id(symbol);
    let path = StorePath::new(parts(ask, Vendor::Gdfl, symbol, Timeframe::SECOND_1, month))
        .map_err(|why| why.to_string())?;
    let file = match store::file::BarFile::open_existing(root, path, id) {
        Ok(file) => file,
        Err(store::file::StoreError::Missing { .. }) => return Ok(BTreeMap::new()),
        Err(why) => return Err(why.to_string()),
    };
    let seconds = (0..file.header().n_valid)
        .map(|i| file.read_record(i).map_err(|why| why.to_string()))
        .collect::<Result<Vec<Bar>, String>>()?;
    let minutes = pull::fold::fold_from_bars(
        &seconds,
        pull::fold::Bucket::MINUTE,
        pull::fold::Bucket::SECOND,
    )
    .map_err(|why| format!("fold to minutes: {why}"))?;
    Ok(minutes
        .into_iter()
        .map(|bar| (bar.ts_micros, bar))
        .collect())
}

/// The measured cause of one flagged bar.
fn measure(flag: &FlaggedBar, witness: &mut Witness<'_>) -> Measured {
    match flag.code {
        "non_positive_price" => Measured::ZeroPrice,
        "before_listing" => Measured::BeforeListing,
        "before_first_day_bar" => Measured::BeforeFirstDayBar,
        _ => match witness.minute(flag.month, flag.bar.ts_micros) {
            Ok(Some(gdfl)) if within(flag.bar.close, gdfl.close) => Measured::BasisMismatch,
            Ok(Some(_)) => Measured::ForeignLevel,
            Ok(None) => Measured::NoGdflWitness,
            Err(()) => Measured::GdflUnread,
        },
    }
}

/// Whether `close` lies within [`WITNESS_TOLERANCE_BPS`] of `witness`. A
/// witness at or below zero vouches for nothing: no positive close lies
/// within a non-positive bound, and a close at or below zero was measured
/// `zero_price` before it reached here.
fn within(close: i64, witness: i64) -> bool {
    (i128::from(close) - i128::from(witness)).abs() * 10_000
        <= i128::from(witness) * i128::from(WITNESS_TOLERANCE_BPS)
}

/// One plan row's key: (series key, rung, month, measured cause, action).
type PlanKey = (String, &'static str, YearMonth, Measured, &'static str);

/// The plan's rows and every measured cause's count.
#[derive(Default)]
struct Plan {
    rows: BTreeMap<PlanKey, Vec<i64>>,
    /// Bars per (series key, rung, month), so a row covering all of them is
    /// a whole month.
    held: BTreeMap<(String, &'static str, YearMonth), u64>,
    counts: BTreeMap<Measured, u64>,
}

impl Plan {
    /// Every flagged bar of `scan` measured and added. Each GDFL month that
    /// was there and would not open, read or fold is appended to
    /// `scan.unread` (`gdfl 1s witness: ...`), so it is printed `NOT READ`
    /// and fails the run like a month of either rung; its bars are counted,
    /// as `gdfl_unread`.
    fn add(
        &mut self,
        root: &Path,
        ask: &Ask,
        symbol: &str,
        scan: &mut SeriesScan,
    ) -> BTreeMap<Measured, u64> {
        let key = format!("NSE-{}-{symbol}", ask.segment);
        for (rung, verdicts) in [("1day", &scan.day_verdicts), ("1min", &scan.months)] {
            for (month, tally) in verdicts {
                self.held
                    .insert((key.clone(), rung, *month), tally.minutes());
            }
        }
        let mut witness = Witness {
            root,
            ask,
            symbol,
            months: BTreeMap::new(),
        };
        let mut counts = BTreeMap::new();
        for flag in &scan.flagged {
            let measured = measure(flag, &mut witness);
            *counts.entry(measured).or_insert(0) += 1;
            *self.counts.entry(measured).or_insert(0) += 1;
            let rung = if flag.timeframe == Timeframe::DAY_1 {
                "1day"
            } else {
                "1min"
            };
            self.rows
                .entry((
                    key.clone(),
                    rung,
                    flag.month,
                    measured,
                    measured.action(flag.refused),
                ))
                .or_default()
                .push(flag.bar.ts_micros);
        }
        scan.unread.extend(witness.unread());
        counts
    }

    /// Another series' plan joined into this one. Every row and every held
    /// count is keyed by its series, so two series never share a key.
    fn absorb(&mut self, other: Self) {
        self.rows.extend(other.rows);
        self.held.extend(other.held);
        for (measured, count) in other.counts {
            *self.counts.entry(measured).or_insert(0) += count;
        }
    }

    /// The plan's text: the head, then one tab-separated row per key.
    fn text(&self, run: &Run<'_>, scope: &str) -> String {
        let mut text = format!(
            "{PLAN_HEAD}\ncommit\t{}\nfeed\t{}\nsegment\t{}\nscope\t{scope}\njump_bps\t{JUMP_BPS}\tmeasured 2026-10-17, docs/06-limits.md R4-17\nwitness_tolerance_bps\t{WITNESS_TOLERANCE_BPS}\nbars_written\t0\nkey\trung\tmonth\tscope\tcause\taction\tbars\ttimestamps_utc_micros\n",
            run.commit,
            run.ask.feed.as_str(),
            run.ask.segment
        );
        for ((key, rung, month, measured, action), stamps) in &self.rows {
            let held = self
                .held
                .get(&(key.clone(), *rung, *month))
                .copied()
                .unwrap_or(0);
            let count = stamps.len() as u64;
            let (scope, listed) = if count == held {
                ("whole_month", String::new())
            } else {
                (
                    "timestamps",
                    stamps
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                )
            };
            let _ = writeln!(
                text,
                "{key}\t{rung}\t{month}\t{scope}\t{}\t{action}\t{count}\t{listed}",
                measured.code()
            );
        }
        text
    }

    /// Write the plan under the store's audit directory, named by its own
    /// digest, and answer its path; a plan already there is not written
    /// again. Nothing is written when nothing was flagged, and nothing under
    /// `--dry-run`, which answers the path it would have written.
    ///
    /// The bytes are synced before the name appears and the directory after
    /// it does, so a reader never sees a part-written plan and a plan the
    /// verb named survives a power cut.
    fn write(&self, run: &Run<'_>, scope: &str) -> Result<Written, String> {
        if self.rows.is_empty() {
            return Ok(Written::Nothing);
        }
        let text = self.text(run, scope);
        let digest = brutex_core::blake3::hash(text.as_bytes());
        let hex = digest.iter().take(8).fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        });
        let dir = PLAN_DIR
            .iter()
            .fold(run.root.to_path_buf(), |dir, part| dir.join(part));
        let name = format!(
            "{}-{}-{scope}-{hex}.plan",
            run.ask.feed.as_str(),
            run.ask.segment
        );
        let path = dir.join(&name);
        if run.ask.dry_run {
            return Ok(Written::DryRun(path));
        }
        if path.is_file() {
            return Ok(Written::Plan(path));
        }
        std::fs::create_dir_all(&dir).map_err(|why| format!("{}: {why}", dir.display()))?;
        // ONE STAGING NAME PER PROCESS, the writer's own (`store::staging`).
        let staging = dir.join(format!(".series-identity.{}.staging", std::process::id()));
        let mut staged = store::staging::create(&staging).map_err(|why| why.to_string())?;
        staged
            .file
            .write_all(text.as_bytes())
            .and_then(|()| staged.file.sync_all())
            .and_then(|()| std::fs::rename(&staging, &path))
            .map_err(|why| format!("{}: {why}", path.display()))?;
        std::fs::File::open(&dir)
            .and_then(|handle| handle.sync_all())
            .map_err(|why| format!("{}: sync after the rename: {why}", dir.display()))?;
        Ok(Written::Plan(path))
    }
}

/// What [`Plan::write`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Written {
    /// Nothing was flagged: no plan.
    Nothing,
    /// The plan is at this path: written now, or already there.
    Plan(PathBuf),
    /// `--dry-run`: the path the plan would have, nothing written.
    DryRun(PathBuf),
}

/// The nearest-rank percentile `per_mille / 1000` of ascending `sorted`.
fn percentile(sorted: &[i64], per_mille: usize) -> i64 {
    let rank = (sorted.len() * per_mille).div_ceil(1_000).max(1);
    sorted.get(rank - 1).copied().unwrap_or(0)
}

/// A share's in-session jump distribution, ascending.
fn jumps(scan: &SeriesScan) -> Vec<i64> {
    let mut sorted = scan.jumps.clone();
    sorted.sort_unstable();
    sorted
}

fn counts_text(counts: &BTreeMap<Measured, u64>) -> String {
    Measured::ALL
        .iter()
        .map(|m| format!("{} {}", m.code(), counts.get(m).copied().unwrap_or(0)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The plan's outcome in the words the run's event carries.
fn plan_word(written: &Result<Written, String>) -> String {
    match written {
        Ok(Written::Plan(path)) => path.display().to_string(),
        Ok(Written::DryRun(path)) => format!("dry run, not written: {}", path.display()),
        Ok(Written::Nothing) => "none: nothing flagged".to_owned(),
        Err(why) => format!("NOT WRITTEN: {why}"),
    }
}

/// The target of the one event a run leaves on `/logs`.
const EVENT_TARGET: &str = "cli.series_identity";

/// The run's one `cli.series_identity` event: what was scanned, its exit,
/// every measured cause's count, how many months would not open and where
/// the plan is. Once per run, after every bar was judged, never in a loop
/// (gate 17); `warn` for a run that fails.
fn run_event(
    run: &Run<'_>,
    scope: &str,
    code: u8,
    plan: &str,
    counts: &BTreeMap<Measured, u64>,
    unread: usize,
) {
    let causes = counts_text(counts);
    let event = if code == OK {
        telemetry::Event::info(EVENT_TARGET, "series identity scanned: nothing refused")
    } else {
        telemetry::Event::warn(
            EVENT_TARGET,
            "series identity scanned: bars a landing refuses, or a month that would not open",
        )
    };
    crate::note(
        &event
            .with("feed", telemetry::Value::Str(run.ask.feed.as_str()))
            .with("segment", telemetry::Value::Str(run.ask.segment))
            .with("scope", telemetry::Value::Str(scope))
            .with("exit", telemetry::Value::Uint(u64::from(code)))
            .with("commit", telemetry::Value::Str(run.commit))
            .with("causes", telemetry::Value::Str(&causes))
            .with(
                "unread_months",
                telemetry::Value::Uint(u64::try_from(unread).unwrap_or(u64::MAX)),
            )
            .with("plan", telemetry::Value::Str(plan)),
    );
}

fn plan_line(out: &mut String, written: Result<Written, String>) -> bool {
    match written {
        Ok(Written::Plan(path)) => {
            let _ = writeln!(
                out,
                "  QUARANTINE PLAN (no bar written): {}",
                path.display()
            );
            true
        }
        Ok(Written::DryRun(path)) => {
            let _ = writeln!(
                out,
                "  QUARANTINE PLAN NOT WRITTEN (--dry-run; nothing written): would be {}",
                path.display()
            );
            true
        }
        Ok(Written::Nothing) => {
            let _ = writeln!(out, "  no quarantine plan: nothing flagged");
            true
        }
        Err(why) => {
            let _ = writeln!(out, "  QUARANTINE PLAN NOT WRITTEN: {why}");
            false
        }
    }
}

/// Each month of either rung that holds a refused or suspect bar, and its
/// (day, cause) groups.
fn verdicts(out: &mut String, scan: &SeriesScan) {
    for (month, tally) in &scan.day_verdicts {
        if tally.refused() == 0 {
            continue;
        }
        let _ = writeln!(
            out,
            "  1day {month}: {} of {} day bars refused",
            tally.refused(),
            tally.minutes()
        );
        for group in &tally.flagged {
            let _ = writeln!(
                out,
                "    {} {} {} day bar(s)",
                day_text(group.day),
                group.code,
                group.minutes
            );
        }
    }
    for (month, tally) in &scan.months {
        if tally.refused() + tally.suspect_level == 0 {
            continue;
        }
        let _ = writeln!(
            out,
            "  {month}: {} of {} minutes refused, {} suspect, {} in-session jump(s)",
            tally.refused(),
            tally.minutes(),
            tally.suspect_level,
            tally.in_session_jump
        );
        for group in &tally.flagged {
            let _ = writeln!(
                out,
                "    {} {} {} minute(s)",
                day_text(group.day),
                group.code,
                group.minutes
            );
        }
    }
}

/// A CASH share's in-session jump distribution: the measurement
/// `JUMP_BPS` is to be recorded from.
fn jump_line(out: &mut String, scan: &SeriesScan, tripped: u64) {
    let sorted = jumps(scan);
    let _ = writeln!(
        out,
        "  in-session jumps: {} judged; p50 {} p99 {} p999 {} max {} bps; {tripped} beyond JUMP_BPS {JUMP_BPS} \
         (measured on the store 2026-10-17: docs/06-limits.md R4-17)",
        sorted.len(),
        percentile(&sorted, 500),
        percentile(&sorted, 990),
        percentile(&sorted, 999),
        sorted.last().copied().unwrap_or(0),
    );
}

/// What the run writes, for its heading.
const fn writes_what(ask: &Ask) -> &'static str {
    if ask.dry_run {
        "--dry-run: writes nothing"
    } else {
        "writes only its quarantine plan"
    }
}

fn one(run: &Run<'_>, symbol: &str, out: &mut String) -> u8 {
    let (root, ask) = (run.root, run.ask);
    let Some(mut scan) = scan(root, ask, symbol) else {
        return MISUSED;
    };
    // MEASURED FIRST, so a GDFL month that would not open is among the
    // NOT READ lines below and fails the run (Z6 repair 1).
    let mut plan = Plan::default();
    let counts = plan.add(root, ask, symbol, &mut scan);
    let sum = total(&scan);
    let _ = writeln!(
        out,
        "SERIES IDENTITY (read-only of bars; {}) — {} NSE-{}-{symbol} under {}",
        writes_what(ask),
        ask.feed.as_str(),
        ask.segment,
        root.display()
    );
    let _ = writeln!(
        out,
        "  1day: {}; first served day {}",
        span(&scan.day_months),
        scan.first_day.map_or_else(|| "none".to_owned(), day_text)
    );
    let _ = writeln!(out, "  1min: {}", span(&scan.minute_months));
    let _ = writeln!(
        out,
        "  listing: {}; listed on {}",
        scan.listing,
        scan.listed_on
            .map_or_else(|| "unknown".to_owned(), day_text)
    );
    let _ = writeln!(
        out,
        "  minutes {}: held {}, before_first_day_bar {}, foreign_level {}, suspect_level {}, \
         unjudged {} (no_day_rung {}, no_day_bar {}, day_bar_unusable {}), non_positive_price {}, \
         before_listing {}, in_session_jump {}",
        sum.minutes(),
        sum.held,
        sum.before_first_day_bar,
        sum.foreign_level,
        sum.suspect_level,
        sum.unchecked(),
        sum.no_day_rung,
        sum.no_day_bar,
        sum.day_bar_unusable,
        sum.non_positive_price,
        sum.before_listing,
        sum.in_session_jump
    );
    verdicts(out, &scan);
    if ask.segment == "CASH" {
        jump_line(out, &scan, sum.in_session_jump);
    }
    for (month, why) in &scan.unread {
        let _ = writeln!(out, "  {month}: NOT READ: {why}");
    }
    let _ = writeln!(out, "  measured causes: {}", counts_text(&counts));
    let written = plan.write(run, symbol);
    let plan_said = plan_word(&written);
    let written = plan_line(out, written);
    let code = one_verdict(out, &scan, &sum, written);
    run_event(run, symbol, code, &plan_said, &counts, scan.unread.len());
    code
}

/// The closing lines of one series' report, and its exit.
fn one_verdict(out: &mut String, scan: &SeriesScan, sum: &Tally, written: bool) -> u8 {
    let months = refused_months(scan);
    let days = day_refused(scan);
    if months.is_empty() && days == 0 && scan.unread.is_empty() && written {
        let _ = writeln!(out, "  no stored bar would be refused at landing");
        return OK;
    }
    if !months.is_empty() {
        let _ = writeln!(
            out,
            "  QUARANTINE NEEDED: {} month(s) hold {} minute(s) a landing refuses today: {}",
            months.len(),
            sum.refused(),
            months
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    if days > 0 {
        let _ = writeln!(
            out,
            "  QUARANTINE NEEDED: {days} day bar(s) a landing refuses today"
        );
    }
    FAILED
}

/// One series' row of the `ALL` table; answers its minute counts.
fn series_row(out: &mut String, ask: &Ask, symbol: &str, scan: &SeriesScan) -> Tally {
    let from = |months: &[YearMonth]| {
        months
            .first()
            .map_or_else(|| "none".to_owned(), ToString::to_string)
    };
    let row = total(scan);
    let sorted = jumps(scan);
    let measured = |per_mille: usize| {
        if ask.segment == "CASH" {
            percentile(&sorted, per_mille).to_string()
        } else {
            "-".to_owned()
        }
    };
    let _ = writeln!(
        out,
        "  {symbol:<20}  {:>9}  {:>20}  {:>13}  {:>13}  {:>8}  {:>14}  {:>9}  {:>9}  {:>5}  {:>8}  {:>9}  {:>8}  {:>11}  {}{}",
        row.minutes(),
        row.before_first_day_bar,
        row.foreign_level,
        row.suspect_level,
        row.unchecked(),
        refused_months(scan).len(),
        from(&scan.day_months),
        from(&scan.minute_months),
        row.in_session_jump,
        measured(990),
        measured(999),
        measured(1_000),
        day_refused(scan),
        scan.listing,
        if scan.unread.is_empty() {
            String::new()
        } else {
            format!("  NOT READ: {} month(s)", scan.unread.len())
        }
    );
    row
}

/// One series of the `ALL` table, judged alone on a worker.
struct SeriesDone {
    /// Its table row and minute counts, or, with no minute rung, how it is
    /// named among the series that have none.
    row: Result<(String, Tally), String>,
    /// Day bars the day-rung arm refuses.
    day_bad: u64,
    /// Whether it fails the run: a refused bar or a month that would not open.
    bad: bool,
    /// Its share of the quarantine plan, measured.
    plan: Plan,
    /// Months of either rung or of its GDFL witness that would not open.
    unread: usize,
}

/// `symbol` scanned, measured and written up for the `ALL` table. Reads
/// bars only; the plan is joined and written by the caller.
fn series_done(root: &Path, ask: &Ask, symbol: &str) -> Option<SeriesDone> {
    let mut scan = scan(root, ask, symbol)?;
    let day_bad = day_refused(&scan);
    let mut plan = Plan::default();
    let _ = plan.add(root, ask, symbol, &mut scan);
    // A MONTH THAT WOULD NOT OPEN FAILS THE RUN on either rung or of the
    // GDFL witness, a series with no minute rung included: its day months
    // were judged too.
    let bad = day_bad > 0 || !scan.unread.is_empty() || !refused_months(&scan).is_empty();
    if scan.minute_months.is_empty() {
        let unread = if scan.unread.is_empty() {
            String::new()
        } else {
            format!(", NOT READ: {} month(s)", scan.unread.len())
        };
        return Some(SeriesDone {
            row: Err(format!(
                "{symbol} (1day {}{unread})",
                span(&scan.day_months)
            )),
            day_bad,
            bad,
            plan,
            unread: scan.unread.len(),
        });
    }
    let mut row = String::new();
    let tally = series_row(&mut row, ask, symbol, &scan);
    Some(SeriesDone {
        row: Ok((row, tally)),
        day_bad,
        bad,
        plan,
        unread: scan.unread.len(),
    })
}

/// Every series of `symbols` judged by [`series_done`] on `workers` threads,
/// each taking the next unjudged series, and answered in `symbols`' order: the
/// same answer for every worker count. A worker that panics re-raises its
/// panic here; nothing is answered for a run that lost a series.
fn scan_every(root: &Path, ask: &Ask, symbols: &[String], workers: usize) -> Vec<SeriesDone> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let next = AtomicUsize::new(0);
    let mut slots: Vec<Option<SeriesDone>> = std::iter::repeat_with(|| None)
        .take(symbols.len())
        .collect();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(symbol) = symbols.get(index) else {
                            break done;
                        };
                        done.push((index, series_done(root, ask, symbol)));
                    }
                })
            })
            .collect();
        for handle in handles {
            let done = handle
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            for (index, series) in done {
                if let Some(slot) = slots.get_mut(index) {
                    *slot = series;
                }
            }
        }
    });
    slots.into_iter().flatten().collect()
}

fn all(run: &Run<'_>, out: &mut String) -> u8 {
    let (root, ask) = (run.root, run.ask);
    let dir = root
        .join("bars")
        .join(ask.feed.as_str())
        .join("NSE")
        .join(ask.segment);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(why) => {
            let _ = writeln!(
                out,
                "SERIES IDENTITY REFUSED: cannot list {}: {why}",
                dir.display()
            );
            return FAILED;
        }
    };
    // IN NAME ORDER, so two runs print the same table.
    let symbols: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .filter(|name| brutex_core::symbol::Symbol::new(name).is_ok())
        .collect::<std::collections::BTreeSet<String>>()
        .into_iter()
        .collect();
    let workers = worker_count(ask.workers, symbols.len());
    let _ = writeln!(
        out,
        "SERIES IDENTITY, EVERY SERIES (read-only of bars; {}) — {} NSE/{}: {} series under {}",
        writes_what(ask),
        ask.feed.as_str(),
        ask.segment,
        symbols.len(),
        root.display()
    );
    let _ = writeln!(
        out,
        "  instrument              minutes  before_first_day_bar  foreign_level  suspect_level  unjudged  refused-months  1day-from  1min-from  jumps  jump-p99  jump-p999  jump-max  day-refused  listing"
    );
    let (mut sum, mut bad, mut no_minutes) = (Tally::default(), false, Vec::new());
    let mut plan = Plan::default();
    let (mut days_refused, mut unread) = (0_u64, 0_usize);
    for done in scan_every(root, ask, &symbols, workers) {
        days_refused += done.day_bad;
        unread += done.unread;
        bad |= done.bad;
        plan.absorb(done.plan);
        match done.row {
            Ok((row, tally)) => {
                out.push_str(&row);
                sum.absorb(tally);
            }
            Err(named) => no_minutes.push(named),
        }
    }
    let _ = writeln!(
        out,
        "  TOTAL: minutes {}, before_first_day_bar {}, foreign_level {}, suspect_level {}, unjudged {}, \
         non_positive_price {}, before_listing {}, in_session_jump {}, day bars refused {days_refused}",
        sum.minutes(),
        sum.before_first_day_bar,
        sum.foreign_level,
        sum.suspect_level,
        sum.unchecked(),
        sum.non_positive_price,
        sum.before_listing,
        sum.in_session_jump
    );
    let _ = writeln!(
        out,
        "  DAY RUNG AND NO MINUTE RUNG ({}): {}",
        no_minutes.len(),
        if no_minutes.is_empty() {
            "none".to_owned()
        } else {
            no_minutes.join(", ")
        }
    );
    let _ = writeln!(out, "  measured causes: {}", counts_text(&plan.counts));
    let written = plan.write(run, "ALL");
    let plan_said = plan_word(&written);
    bad |= !plan_line(out, written);
    let code = if bad { FAILED } else { OK };
    run_event(run, "ALL", code, &plan_said, &plan.counts, unread);
    code
}

#[cfg(test)]
#[path = "series_identity_tests.rs"]
mod tests;
