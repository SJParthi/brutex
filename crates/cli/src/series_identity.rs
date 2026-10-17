//! `cli series-identity` — whose bars a stored series holds. WP-13d; findings
//! DB-B07, DB-B08, DB-B10, DB-B22, DB-B23, DB-B60, DB-B64, DB-B66. R4-17:
//! RT-DATA-02 b, Z6, the PIIND quarantine plan.
//!
//! # Usage
//!
//! ```text
//! cli series-identity INSTRUMENT [--segment INDEX|CASH] [--feed FEED]
//! cli series-identity ALL [--segment INDEX|CASH] [--feed FEED]
//! ```
//!
//! `--segment` is `INDEX` unless named; `--feed` is `zerodha` unless named.
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
//! GDFL minute to compare it is `no_gdfl_witness`. A price at or below zero is
//! `zero_price`; a bar before its listing `before_listing`.
//!
//! **It writes one file**, and no bar: the QUARANTINE PLAN, under
//! `<store>/audit/series-identity/`, naming per (key, rung, month, measured
//! cause) the action, the count, and either the whole month or each stamp.
//! Its name carries the BLAKE3 of its content, so a rerun over the same store
//! finds it already there and writes nothing (`CLAUDE.md` §3 rule 5). A CASH
//! scan also prints each share's in-session jump distribution (p50, p99,
//! p999, max): the measurement `JUMP_BPS` is to be recorded from.
//!
//! # Exit
//!
//! [`OK`] when no stored bar would be refused and every month opened;
//! [`FAILED`] otherwise — any in-session jump among them; [`MISUSED`] for
//! arguments that name no series.

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
pub(crate) const USAGE: &str =
    "cli series-identity INSTRUMENT|ALL [--segment INDEX|CASH] [--feed FEED]";

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
const PLAN_HEAD: &str = "brutex series-identity quarantine plan v1";

/// What the arguments name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ask {
    /// `None` for `ALL`.
    instrument: Option<String>,
    segment: &'static str,
    feed: Vendor,
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
    let (mut segment, mut feed) = ("INDEX", Vendor::Zerodha);
    let mut rest = flags.iter();
    while let Some(flag) = rest.next() {
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
            other => return Err(format!("`{other}` is not a flag of this verb")),
        }
    }
    Ok(Ask {
        instrument,
        segment,
        feed,
    })
}

/// Runs the verb against the store root `store` names, so every arm is driven
/// by a test without the process environment.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
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
            let _ = writeln!(out, "SERIES IDENTITY REFUSED: {why}. Nothing was read.");
            return FAILED;
        }
    };
    match &ask.instrument {
        Some(symbol) => one(&root, &ask, symbol, out),
        None => all(&root, &ask, out),
    }
}

/// `cli series-identity`: see the module header.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    command_with(arguments, &crate::store_root, out)
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
    /// GDFL holds no minute to compare it with.
    NoGdflWitness,
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
        }
    }

    /// What the plan asks for a bar of this cause: `quarantine`, or `label`
    /// for a minute GDFL vouches for and for a suspect one nothing accuses.
    const fn action(self, refused: bool) -> &'static str {
        match self {
            Self::BasisMismatch => "label",
            Self::NoGdflWitness if !refused => "label",
            _ => "quarantine",
        }
    }

    const ALL: [Self; 6] = [
        Self::ZeroPrice,
        Self::BeforeListing,
        Self::BeforeFirstDayBar,
        Self::BasisMismatch,
        Self::ForeignLevel,
        Self::NoGdflWitness,
    ];
}

/// GDFL's 1s bars of one share, folded to minutes, one month at a time.
struct Witness<'a> {
    root: &'a Path,
    ask: &'a Ask,
    symbol: &'a str,
    months: BTreeMap<YearMonth, BTreeMap<i64, Bar>>,
}

impl Witness<'_> {
    /// GDFL's folded minute stamped `ts_micros` of `month`, if GDFL holds one.
    fn minute(&mut self, month: YearMonth, ts_micros: i64) -> Option<Bar> {
        let (root, ask, symbol) = (self.root, self.ask, self.symbol);
        self.months
            .entry(month)
            .or_insert_with(|| gdfl_minutes(root, ask, symbol, month))
            .get(&ts_micros)
            .copied()
    }
}

/// Every GDFL 1s bar of `symbol` in `month`, folded to minutes by
/// `pull::fold`. A month GDFL lacks, or cannot be read or folded, witnesses
/// nothing.
fn gdfl_minutes(root: &Path, ask: &Ask, symbol: &str, month: YearMonth) -> BTreeMap<i64, Bar> {
    let id = pull::ingest::repair_plan::symbol_id(symbol);
    StorePath::new(parts(ask, Vendor::Gdfl, symbol, Timeframe::SECOND_1, month))
        .ok()
        .and_then(|path| store::file::BarFile::open_existing(root, path, id).ok())
        .and_then(|file| {
            (0..file.header().n_valid)
                .map(|i| file.read_record(i).ok())
                .collect::<Option<Vec<Bar>>>()
        })
        .and_then(|seconds| {
            pull::fold::fold_from_bars(
                &seconds,
                pull::fold::Bucket::MINUTE,
                pull::fold::Bucket::SECOND,
            )
            .ok()
        })
        .map(|minutes| {
            minutes
                .into_iter()
                .map(|bar| (bar.ts_micros, bar))
                .collect()
        })
        .unwrap_or_default()
}

/// The measured cause of one flagged bar.
fn measure(flag: &FlaggedBar, witness: &mut Witness<'_>) -> Measured {
    match flag.code {
        "non_positive_price" => Measured::ZeroPrice,
        "before_listing" => Measured::BeforeListing,
        "before_first_day_bar" => Measured::BeforeFirstDayBar,
        _ => match witness.minute(flag.month, flag.bar.ts_micros) {
            Some(gdfl) if within(flag.bar.close, gdfl.close) => Measured::BasisMismatch,
            Some(_) => Measured::ForeignLevel,
            None => Measured::NoGdflWitness,
        },
    }
}

/// Whether `close` lies within [`WITNESS_TOLERANCE_BPS`] of `witness`.
fn within(close: i64, witness: i64) -> bool {
    witness > 0
        && (i128::from(close) - i128::from(witness)).abs() * 10_000
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
    /// Every flagged bar of `scan` measured and added.
    fn add(
        &mut self,
        root: &Path,
        ask: &Ask,
        symbol: &str,
        scan: &SeriesScan,
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
        counts
    }

    /// The plan's text: the head, then one tab-separated row per key.
    fn text(&self, ask: &Ask, scope: &str) -> String {
        let mut text = format!(
            "{PLAN_HEAD}\nfeed\t{}\nsegment\t{}\nscope\t{scope}\njump_bps\t{JUMP_BPS}\tPROVISIONAL, UNVERIFIED until measured\nwitness_tolerance_bps\t{WITNESS_TOLERANCE_BPS}\nbars_written\t0\nkey\trung\tmonth\tscope\tcause\taction\tbars\ttimestamps_utc_micros\n",
            ask.feed.as_str(),
            ask.segment
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
    /// again. Nothing is written when nothing was flagged.
    fn write(&self, root: &Path, ask: &Ask, scope: &str) -> Result<Option<PathBuf>, String> {
        if self.rows.is_empty() {
            return Ok(None);
        }
        let text = self.text(ask, scope);
        let digest = brutex_core::blake3::hash(text.as_bytes());
        let hex = digest.iter().take(8).fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        });
        let dir = PLAN_DIR
            .iter()
            .fold(root.to_path_buf(), |dir, part| dir.join(part));
        let name = format!("{}-{}-{scope}-{hex}.plan", ask.feed.as_str(), ask.segment);
        let path = dir.join(&name);
        if path.is_file() {
            return Ok(Some(path));
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
        Ok(Some(path))
    }
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

fn plan_line(out: &mut String, written: Result<Option<PathBuf>, String>) -> bool {
    match written {
        Ok(Some(path)) => {
            let _ = writeln!(
                out,
                "  QUARANTINE PLAN (no bar written): {}",
                path.display()
            );
            true
        }
        Ok(None) => {
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
         (PROVISIONAL: UNVERIFIED until this distribution is measured on the store)",
        sorted.len(),
        percentile(&sorted, 500),
        percentile(&sorted, 990),
        percentile(&sorted, 999),
        sorted.last().copied().unwrap_or(0),
    );
}

fn one(root: &Path, ask: &Ask, symbol: &str, out: &mut String) -> u8 {
    let Some(scan) = scan(root, ask, symbol) else {
        return MISUSED;
    };
    let sum = total(&scan);
    let _ = writeln!(
        out,
        "SERIES IDENTITY (read-only of bars; writes only its quarantine plan) — {} NSE-{}-{symbol} under {}",
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
    let mut plan = Plan::default();
    let counts = plan.add(root, ask, symbol, &scan);
    let _ = writeln!(out, "  measured causes: {}", counts_text(&counts));
    let written = plan_line(out, plan.write(root, ask, symbol));
    let months = refused_months(&scan);
    let days = day_refused(&scan);
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

fn all(root: &Path, ask: &Ask, out: &mut String) -> u8 {
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
    let symbols: std::collections::BTreeSet<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
        .filter(|name| brutex_core::symbol::Symbol::new(name).is_ok())
        .collect();
    let _ = writeln!(
        out,
        "SERIES IDENTITY, EVERY SERIES (read-only of bars) — {} NSE/{}: {} series under {}",
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
    let mut days_refused = 0_u64;
    for symbol in &symbols {
        let Some(scan) = scan(root, ask, symbol) else {
            continue;
        };
        let day_bad = day_refused(&scan);
        let _ = plan.add(root, ask, symbol, &scan);
        days_refused += day_bad;
        bad |= day_bad > 0;
        if scan.minute_months.is_empty() {
            no_minutes.push(format!("{symbol} (1day {})", span(&scan.day_months)));
            continue;
        }
        bad |= !refused_months(&scan).is_empty() || !scan.unread.is_empty();
        sum.absorb(series_row(out, ask, symbol, &scan));
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
    bad |= !plan_line(out, plan.write(root, ask, "ALL"));
    if bad { FAILED } else { OK }
}

#[cfg(test)]
#[path = "series_identity_tests.rs"]
mod tests;
