//! `cli basis-runs` — per share, the maximal runs of one Zerodha/GDFL close
//! ratio, folded from DATA-1's per stock-day factor, and the measured
//! corporate-action ledger their steps make (R4-26: Z8, COR-06, FM-260).
//!
//! # Usage
//!
//! ```text
//! cli basis-runs --dir DIR [--from YYYY-MM] [--to YYYY-MM] [--apply] [--ledger]
//! ```
//!
//! # One owner of the factor
//!
//! The ratio of a stock-day is DATA-1's and nobody else's: this verb reads
//! the newest FULL `feed-reconcile` version file of each month in `DIR`
//! through [`crate::feed_reconcile::FactorIndex`] and takes each day's
//! interval from `FactorPair::basis` (D-5179), the same rule DATA-1's own
//! `ex_date` column uses. It measures no price itself and reads no bar. A
//! day with no basis (no overlap, one feed, a day the file does not hold, an
//! interval off its own median) is counted by its reason and never folded.
//!
//! # What it folds
//!
//! Per share, days ascending across the months read,
//! `pull::corporate_actions::Folder`: a run ends `step` where the next
//! day's interval lies more than 0.2% from the last day's (D-3339's
//! `basis_step`, the threshold `/sources` judges by), else grows while its
//! band (its days' intervals, each widened by 0.2%, intersected) is not
//! empty and ends `drift` where it empties; a month missing from `DIR` is named and
//! leaves a gap, never a break nor a guess. A share with no basis day has no
//! run and is named. The window, first month to last (asked, or held when a
//! flag is left out), is at most 1,200 months and a wider one is refused by
//! name before any file is read (D-7197).
//!
//! # What it writes
//!
//! Nothing, without `--apply`: it prints the runs' summary, every step and
//! the checker's findings against the store's corporate-action table. With
//! `--apply`, generation `n + 1` of `<store>/reference/v1/basis_runs`
//! through R3-30's writer (append-only: a hard link that refuses an existing
//! name; the same bytes again land nothing), its receipt naming the files
//! read and the SHA-256 of their list (each file's BLAKE3 in it). With
//! `--ledger` as well, the measured `corporate_actions` generation the steps
//! make: refused when the store's newest corporate-action table is
//! `sourced` (an NSE file stands; the measured ledger never replaces it) or
//! is refused (it must be looked at first). Both are `measured`: a measured
//! row labels and never excuses.
//!
//! # Only a LASTING step is a row (D-7195, D-7196)
//!
//! Every step is printed with its verdict from `pull::corporate_actions::judge`,
//! the one owner of the rule: `step` (lasting, a ledger row), `step_within_noise`
//! (not proven) or `basis_anomaly` with its reason (`basis_reversed`,
//! `basis_short`, `basis_within_wobble`). Anomalies are counted on the summary
//! line and in one `warn` event on `/logs`, and never ledgered. A lasting step
//! whose window holds a session with no basis says its ex-date is UNKNOWN
//! inside the window and names the months of it missing from `DIR`. A run
//! ending `step` whose factor terms pass 10^18 makes no step: it is counted
//! (`steps_skipped`) on its own line and in that event, never dropped silently.
//!
//! With `--ledger`, the ledger is drawn from the basis-runs generation THIS
//! run wrote or found holding its bytes, read back at that generation's own
//! name (`pull::reference_data::load_written`), never from whichever
//! generation is newest when it looks: a second writer landing between the
//! two writes cannot lend the ledger its runs.
//!
//! A receipt keeps the day its exact source bytes were first read: a rerun on
//! another day over the same files renders the same bytes and lands nothing.
//!
//! # Cost
//!
//! One load of each month's version file (DATA-1's, O(its rows)), then per
//! share-day one hash lookup (DATA-1's `FactorIndex::factor_of`) and one
//! fold step of fixed work: O(shares x days) for the whole history, one
//! pass, memory O(shares) for the folders plus one month's index at a time.
//! The write is one file of O(runs) rows. Measured once, not gated: DATA-1's
//! full run (105 months, 940,143 stock-days) folds in 2.2 s on the operator's
//! machine (`docs/06-limits.md` L-R426-02).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pull::corporate_actions::{
    self, Anomaly, BasisDay, BasisRuns, CorporateActions, Finding, Folder, Judged, Ratio, Run,
    RunEnd, Step, Verdict,
};
use pull::reference_data::{self, Draft, Slot, Standing, Table};
use pull::session::Day;
use store::path::YearMonth;

use crate::feed_reconcile::{FactorIndex, FactorRefusal};
use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str =
    "cli basis-runs --dir DIR [--from YYYY-MM] [--to YYYY-MM] [--apply] [--ledger]";

/// The first line of every report.
pub const FORMAT: &str = "# brutex basis-runs 1";

/// The most months one run reads, and the widest window it walks: 2000-01
/// to 2099-12 (D-7197). A wider `--from`/`--to`, or a directory whose
/// first and last months lie further apart, is refused by name.
const MAX_MONTHS: usize = 1_200;

/// What the operator asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ask {
    /// The directory holding DATA-1's feed-reconcile version files.
    pub(crate) dir: PathBuf,
    /// The first month read, when bounded.
    pub(crate) from: Option<YearMonth>,
    /// The last month read, when bounded.
    pub(crate) to: Option<YearMonth>,
    /// Write the basis-run generation.
    pub(crate) write: bool,
    /// Write the measured corporate-action generation too.
    pub(crate) ledger: bool,
}

fn month(text: &str) -> Result<YearMonth, String> {
    crate::feed_reconcile::parse_month(text)
}

/// Parses the arguments.
///
/// # Errors
///
/// A missing `--dir`, an unknown or repeated flag, a flag without its value,
/// a month that is not `YYYY-MM`, `--from` after `--to`, a `--from`/`--to`
/// window wider than [`MAX_MONTHS`], or `--ledger` without `--apply`; each
/// by name.
pub(crate) fn ask(arguments: &[&str]) -> Result<Ask, String> {
    let mut dir = None;
    let (mut from, mut to) = (None, None);
    let (mut write, mut ledger) = (false, false);
    let mut rest = arguments.iter();
    while let Some(flag) = rest.next() {
        let mut value = |name: &str| {
            rest.next()
                .copied()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match *flag {
            "--dir" if dir.is_none() => dir = Some(PathBuf::from(value("--dir")?)),
            "--from" if from.is_none() => from = Some(month(value("--from")?)?),
            "--to" if to.is_none() => to = Some(month(value("--to")?)?),
            "--apply" if !write => write = true,
            "--ledger" if !ledger => ledger = true,
            "--dir" | "--from" | "--to" | "--apply" | "--ledger" => {
                return Err(format!("{flag} is given twice"));
            }
            other => return Err(format!("{other:?} is not an argument of basis-runs")),
        }
    }
    let dir = dir.ok_or_else(|| "--dir DIR is required".to_owned())?;
    if let (Some(from), Some(to)) = (from, to)
        && from > to
    {
        return Err(format!("--from {from} is after --to {to}"));
    }
    if let (Some(from), Some(to)) = (from, to) {
        window_months(from, to).map_err(|why| format!("--from {from} --to {to}: {why}"))?;
    }
    if ledger && !write {
        return Err("--ledger writes the measured ledger and needs --apply".to_owned());
    }
    Ok(Ask {
        dir,
        from,
        to,
        write,
        ledger,
    })
}

/// Every month `dir` holds a full version file of, ascending. A
/// `--symbols` file is never a month's authority (D-5178) and is not read.
///
/// # Errors
///
/// A directory that cannot be listed, or more than [`MAX_MONTHS`] months.
pub(crate) fn months_in(dir: &Path) -> Result<Vec<YearMonth>, String> {
    let entries =
        std::fs::read_dir(dir).map_err(|why| format!("listing `{}`: {why}", dir.display()))?;
    let mut months = std::collections::BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|why| format!("listing `{}`: {why}", dir.display()))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(rest) = name.strip_prefix("feed-reconcile-") else {
            continue;
        };
        let (Some(stamp), Some(tail)) = (rest.get(..7), rest.get(7..)) else {
            continue;
        };
        // ONLY A NAME DATA-1 WRITES (`feed_reconcile::newest_of`'s rule):
        // `.v<N>.csv` with a number that round-trips; a `.symbols.v` file
        // or a `.v007.csv` is not a month's full file.
        let full = tail
            .strip_prefix(".v")
            .and_then(|rest| rest.strip_suffix(".csv"))
            .and_then(|n| n.parse::<u32>().ok().filter(|v| v.to_string() == n));
        if full.is_none() {
            continue;
        }
        if let Ok(month) = month(stamp) {
            months.insert(month);
        }
        if months.len() > MAX_MONTHS {
            return Err(format!(
                "`{}` holds more than {MAX_MONTHS} months",
                dir.display()
            ));
        }
    }
    Ok(months.into_iter().collect())
}

/// The month after `month`, if the store can name it.
fn next_month(month: YearMonth) -> Option<YearMonth> {
    let (year, number) = if month.month() == 12 {
        (month.year().checked_add(1)?, 1)
    } else {
        (month.year(), month.month().checked_add(1)?)
    };
    YearMonth::new(year, number).ok()
}

/// `month` numbered from January of year 0, so that the distance between
/// two months is one subtraction. At most 9999 x 12 + 11. O(1).
fn ordinal(month: YearMonth) -> usize {
    usize::from(month.year()) * 12 + usize::from(month.month().saturating_sub(1))
}

/// How many months `first..=last` holds. O(1).
///
/// # Errors
///
/// `first` after `last`, or a window of more than [`MAX_MONTHS`] months,
/// which no run walks; each by name.
fn window_months(first: YearMonth, last: YearMonth) -> Result<usize, String> {
    let months = ordinal(last)
        .checked_sub(ordinal(first))
        .map(|apart| apart + 1)
        .ok_or_else(|| format!("{first} is after {last}"))?;
    if months > MAX_MONTHS {
        return Err(format!(
            "the window {first}..{last} spans {months} months, more than the {MAX_MONTHS} one \
             run reads; name a narrower --from/--to"
        ));
    }
    Ok(months)
}

/// The months of a run's window `DIR` holds no full file of, with one
/// prefix count per month walked: `before[i]` is how many of them lie
/// before the window's month `i`. A step finds the missing months inside
/// its own window in two reads of `before`, never a search (CI gate 11
/// rule 1, D-7197); the window is at most [`MAX_MONTHS`] months, so the
/// counts are at most 1,201 words.
#[derive(Debug, Default)]
pub(crate) struct Gaps {
    /// The window's first month, offset 0 of `before`; `None` before any
    /// window is walked.
    first: Option<YearMonth>,
    /// The months missing, ascending.
    months: Vec<YearMonth>,
    /// `before[i]`: how many of `months` lie before the window's month `i`.
    /// One entry a month walked, and one more.
    before: Vec<usize>,
}

impl Gaps {
    /// The record of a window starting at `first` and `months` months long,
    /// before any month of it is walked.
    fn new(first: YearMonth, months: usize) -> Self {
        let mut before = Vec::with_capacity(months.saturating_add(1));
        before.push(0);
        Self {
            first: Some(first),
            months: Vec::new(),
            before,
        }
    }

    /// The window's next month, in order: held, or missing from `DIR`.
    fn walk(&mut self, month: YearMonth, held: bool) {
        if !held {
            self.months.push(month);
        }
        self.before.push(self.months.len());
    }

    /// Every month missing, ascending.
    pub(crate) fn months(&self) -> &[YearMonth] {
        &self.months
    }

    /// How many missing months come before the month numbered `at`. A month
    /// before the window counts none and one after it counts all, which is
    /// exact: no month outside the window is missing. O(1).
    fn count_before(&self, at: usize) -> usize {
        let Some(first) = self.first else {
            return 0;
        };
        let last = self.before.len().saturating_sub(1);
        let offset = at.saturating_sub(ordinal(first)).min(last);
        self.before.get(offset).copied().unwrap_or(0)
    }

    /// The missing months strictly after `from` and strictly before `to`.
    /// Two reads of the counts, then the slice. O(1).
    pub(crate) fn between(&self, from: YearMonth, to: YearMonth) -> &[YearMonth] {
        let start = self.count_before(ordinal(from) + 1);
        let end = self.count_before(ordinal(to));
        self.months.get(start..end).unwrap_or(&[])
    }
}

/// Why a stock-day was not folded, as the report counts it.
fn refusal_word(why: &FactorRefusal) -> &'static str {
    match why {
        FactorRefusal::NoOverlap => "no_overlap",
        FactorRefusal::FactorNotConstant { .. } => "no_basis",
        FactorRefusal::ReadFailed(_) => "read_failed",
        FactorRefusal::NotChecked => "not_checked",
        FactorRefusal::NotAligned => "not_aligned",
        FactorRefusal::OneFeed(_) => "one_feed",
    }
}

/// One run's fold over the months read.
#[derive(Debug)]
pub(crate) struct Folded {
    /// The months read, each with its file and that file's BLAKE3.
    pub(crate) read: Vec<(YearMonth, PathBuf, [u8; 32])>,
    /// The months in range `DIR` holds no full file of.
    pub(crate) missing: Gaps,
    /// Every run, by share then first day.
    pub(crate) runs: Vec<Run>,
    /// Shares a file held with no basis day at all.
    pub(crate) no_run: Vec<String>,
    /// Shares whose days could not be folded, with why.
    pub(crate) refused: Vec<(String, String)>,
    /// Stock-days not folded, by reason.
    pub(crate) skipped: BTreeMap<&'static str, u64>,
    /// Stock-days folded.
    pub(crate) stock_days: u64,
}

/// One share's days of `month` from `index`, pushed onto its `folder`;
/// a day with no basis is counted in `skipped` by its reason. Returns the
/// days folded, or why the folder refused one (the share is then refused).
fn fold_month(
    index: &FactorIndex,
    month: YearMonth,
    symbol: &str,
    folder: &mut Folder,
    skipped: &mut BTreeMap<&'static str, u64>,
) -> Result<u64, String> {
    let mut days = 0_u64;
    for d in 1..=31_u8 {
        let Ok(day) = Day::new(month.year(), month.month(), d) else {
            break;
        };
        let pair = match index.factor_of(symbol, day) {
            Ok(pair) => pair,
            Err(FactorRefusal::FactorNotConstant { pair, .. }) => *pair,
            Err(why) => {
                *skipped.entry(refusal_word(&why)).or_default() += 1;
                continue;
            }
        };
        let Some(((low, high), _)) = pair.basis() else {
            *skipped.entry("no_basis").or_default() += 1;
            continue;
        };
        let (Some(low), Some(high)) = (
            Ratio::of_signed(low.num, low.den),
            Ratio::of_signed(high.num, high.den),
        ) else {
            *skipped
                .entry("interval_not_positive_or_too_large")
                .or_default() += 1;
            continue;
        };
        folder
            .push(BasisDay { day, low, high })
            .map_err(|why| why.to_string())?;
        days += 1;
    }
    Ok(days)
}

/// Folds every month of `ask` in `ask.dir`.
///
/// # Errors
///
/// A directory that cannot be listed, no month in range, a window (the
/// asked or held months, first to last) wider than [`MAX_MONTHS`], or a
/// version file DATA-1's reader refuses (cut, damaged, of another format);
/// each by name.
pub(crate) fn fold(ask: &Ask) -> Result<Folded, String> {
    let held = months_in(&ask.dir)?;
    let in_range: Vec<YearMonth> = held
        .iter()
        .copied()
        .filter(|m| ask.from.is_none_or(|from| *m >= from) && ask.to.is_none_or(|to| *m <= to))
        .collect();
    let (Some(first), Some(last)) = (
        ask.from.or_else(|| in_range.first().copied()),
        ask.to.or_else(|| in_range.last().copied()),
    ) else {
        return Err(format!(
            "`{}` holds no full feed-reconcile file in the range asked",
            ask.dir.display()
        ));
    };
    if in_range.is_empty() {
        return Err(format!(
            "`{}` holds no full feed-reconcile file from {first} to {last}",
            ask.dir.display()
        ));
    }
    // BOUNDED BEFORE ANYTHING IS READ: the walk below is one step a month
    // of the window, and a window is at most MAX_MONTHS long (D-7197).
    let span = window_months(first, last)?;
    let mut folders: BTreeMap<String, Folder> = BTreeMap::new();
    let mut seen_days: BTreeMap<String, u64> = BTreeMap::new();
    let mut refused: BTreeMap<String, String> = BTreeMap::new();
    let mut skipped: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut days_folded = 0_u64;
    let mut read = Vec::new();
    let mut missing = Gaps::new(first, span);
    // The months held, walked beside the calendar: one compare a month.
    let mut held_months = in_range.iter().copied().peekable();
    let mut at = Some(first);
    while let Some(month) = at.filter(|m| *m <= last) {
        at = next_month(month);
        let held = held_months.next_if_eq(&month).is_some();
        missing.walk(month, held);
        if !held {
            continue;
        }
        let index = FactorIndex::load(&ask.dir, month)?;
        read.push((month, index.file.clone(), index.digest));
        // IN THE FILE'S OWN ORDER: each share folds alone into its own
        // folder, so the order changes nothing the report or the file says.
        for symbol in index.symbols() {
            let seen = seen_days.entry(symbol.to_owned()).or_insert(0);
            if refused.contains_key(symbol) {
                continue;
            }
            let share = folders
                .entry(symbol.to_owned())
                .or_insert_with(|| Folder::new(symbol));
            match fold_month(&index, month, symbol, share, &mut skipped) {
                Ok(days) => {
                    days_folded += days;
                    *seen += days;
                }
                Err(why) => {
                    refused.insert(symbol.to_owned(), why);
                }
            }
        }
    }
    let mut runs = Vec::new();
    for (symbol, folder) in folders {
        if refused.contains_key(&symbol) {
            continue;
        }
        runs.extend(folder.finish());
    }
    let no_run = seen_days
        .into_iter()
        .filter(|(symbol, days)| *days == 0 && !refused.contains_key(symbol))
        .map(|(symbol, _)| symbol)
        .collect();
    Ok(Folded {
        read,
        missing,
        runs,
        no_run,
        refused: refused.into_iter().collect(),
        skipped,
        stock_days: days_folded,
    })
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _infallible = write!(out, "{byte:02x}");
    }
    out
}

/// The list of files read, one line each (`<month> <file name> <blake3>`):
/// the bytes the receipt's `source_sha256` hashes.
fn manifest(folded: &Folded) -> String {
    let mut text = String::new();
    for (month, file, digest) in &folded.read {
        let name = file
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let _ = writeln!(text, "{month} {name} {}", hex(digest));
    }
    text
}

/// The basis-run table `folded` makes, as a draft.
pub(crate) fn runs_draft(ask: &Ask, folded: &Folded, fetched: Day) -> Draft {
    let (first, last) = (
        folded
            .read
            .first()
            .map(|(m, ..)| m.to_string())
            .unwrap_or_default(),
        folded
            .read
            .last()
            .map(|(m, ..)| m.to_string())
            .unwrap_or_default(),
    );
    let mut source = format!(
        "measured, not sourced: cli basis-runs over DATA-1's feed-reconcile files in {} ({} months \
         {first}..{last}, {} missing",
        ask.dir.display(),
        folded.read.len(),
        folded.missing.months().len(),
    );
    if let Some(gap) = folded.missing.months().first() {
        let _ = write!(source, ", the first {gap}");
    }
    source.push(')');
    // A receipt value is one line of at most 2,048 bytes: a very long
    // directory name is cut at a character boundary, and the manifest's
    // hash still binds every file read.
    while source.len() > reference_data::MAX_FIELD_BYTES {
        source.pop();
    }
    Draft {
        table: Table::BasisRuns,
        source,
        source_sha256: Some(reference_data::sha256_of(manifest(folded).as_bytes())),
        fetched,
        standing: Standing::Measured,
        decision: corporate_actions::DECISION.to_owned(),
        rows: folded.runs.iter().map(Run::fields).collect(),
    }
}

/// Today in IST, from the clock.
fn today() -> Option<Day> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    pull::session::IstMoment::from_epoch_secs(i64::try_from(secs).ok()?)
        .ok()
        .map(pull::session::IstMoment::day)
}

/// `cli basis-runs`: see the module header.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    let store = crate::store_write::root_for("basis-runs", arguments);
    command_with(arguments, &store, today(), out)
}

/// The verb against the store root `store` names and the day `today`, so
/// every arm is driven by a test without the process environment.
#[expect(
    clippy::too_many_lines,
    reason = "one report printed top to bottom in the order the operator reads it"
)]
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    today: Option<Day>,
    out: &mut String,
) -> u8 {
    let ask = match ask(arguments) {
        Ok(ask) => ask,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}\nusage: {USAGE}");
            return MISUSED;
        }
    };
    let folded = match fold(&ask) {
        Ok(folded) => folded,
        Err(why) => {
            let _ = writeln!(out, "BASIS RUNS REFUSED: {why}. Nothing was written.");
            return FAILED;
        }
    };
    let _ = writeln!(out, "{FORMAT}");
    let _ = writeln!(
        out,
        "months read {} ({}..{}), missing {}{}",
        folded.read.len(),
        folded
            .read
            .first()
            .map(|(m, ..)| m.to_string())
            .unwrap_or_default(),
        folded
            .read
            .last()
            .map(|(m, ..)| m.to_string())
            .unwrap_or_default(),
        folded.missing.months().len(),
        if folded.missing.months().is_empty() {
            String::new()
        } else {
            format!(
                ": {}",
                folded
                    .missing
                    .months()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        }
    );
    let count = |end: RunEnd| folded.runs.iter().filter(|run| run.end == end).count();
    // The runs come share by share (the folders are keyed by symbol).
    let shares = folded.runs.chunk_by(|a, b| a.symbol == b.symbol).count();
    let _ = writeln!(
        out,
        "stock-days folded {}, not folded {} ({}); shares with runs {shares}, runs {} (step {}, \
         drift {}, open {})",
        folded.stock_days,
        folded.skipped.values().sum::<u64>(),
        folded
            .skipped
            .iter()
            .map(|(why, n)| format!("{why} {n}"))
            .collect::<Vec<_>>()
            .join(", "),
        folded.runs.len(),
        count(RunEnd::Step),
        count(RunEnd::Drift),
        count(RunEnd::Open),
    );
    for symbol in &folded.no_run {
        let _ = writeln!(
            out,
            "no_run {symbol}: no stock-day with a basis in the months read"
        );
    }
    for (symbol, why) in &folded.refused {
        let _ = writeln!(out, "refused {symbol}: {why}");
    }
    let tally = report_steps(out, &folded);
    let Some(today) = today else {
        let _ = writeln!(
            out,
            "BASIS RUNS FAILED: the clock gives no IST day. Nothing was written."
        );
        return FAILED;
    };
    if tally.anomalies() > 0 || tally.steps_skipped > 0 {
        // NAMED, COUNTED AND ON /logs: proven steps the data does not show
        // to last are never ledgered, and never dropped without a word.
        let count = |n: usize| u64::try_from(n).unwrap_or(u64::MAX);
        crate::note(
            &telemetry::Event::warn(
                "cli.basis_runs",
                "basis anomalies: proven ratio steps that are not a lasting change of basis; \
                 named, never ledgered",
            )
            .with("anomalies", count(tally.anomalies()))
            .with("reversed", count(tally.reversed))
            .with("short", count(tally.short))
            .with("within_wobble", count(tally.within_wobble))
            .with("shares", count(tally.shares_with_anomalies))
            .with("steps", count(tally.steps))
            .with("lasting", count(tally.lasting))
            .with("steps_skipped", count(tally.steps_skipped)),
        );
    }
    let root = match store() {
        Ok(root) => root,
        Err(why) => {
            let _ = writeln!(
                out,
                "store: {why}; the runs were not written and nothing was checked against a table"
            );
            return if ask.write { FAILED } else { OK };
        }
    };
    let mut draft = runs_draft(&ask, &folded, today);
    // IDEMPOTENT ACROSS DAYS: the same source bytes (the same manifest of
    // the same files) keep the day they were first read, so a rerun on
    // another day renders the same bytes and lands nothing.
    if let Some(held) = reference_data::load_one(&root, Table::BasisRuns, BasisRuns::build)
        .receipt()
        .filter(|held| held.source == draft.source && held.source_sha256 == draft.source_sha256)
    {
        draft.fetched = held.fetched;
    }
    let fetched = draft.fetched;
    let actions = reference_data::load_one(&root, Table::CorporateActions, CorporateActions::build);
    if !ask.write {
        report_check(out, &draft, &actions);
        let _ = writeln!(out, "dry run: nothing written (add --apply)");
        return OK;
    }
    let written = match reference_data::write(&root, &draft) {
        Ok(written) => written,
        Err(why) => {
            let _ = writeln!(out, "BASIS RUNS FAILED: {why}. Nothing was written.");
            return FAILED;
        }
    };
    let _ = writeln!(
        out,
        "basis_runs generation {} {} sha256 {} at {}",
        written.generation,
        if written.landed {
            "landed"
        } else {
            "already held these bytes"
        },
        written.sha256,
        written.path.display()
    );
    crate::note(
        &telemetry::Event::info("cli.basis_runs", "basis runs written")
            .with("generation", written.generation)
            .with("landed", written.landed)
            .with("runs", u64::try_from(folded.runs.len()).unwrap_or(u64::MAX))
            .with("sha256", written.sha256.as_str()),
    );
    if !ask.ledger {
        report_check(out, &draft, &actions);
        return OK;
    }
    // THE MEASURED LEDGER NEVER REPLACES AN NSE FILE, and never lands over
    // a table that is refused: that table is looked at first.
    match &actions {
        Slot::Store { receipt, .. } if receipt.standing == Standing::Sourced => {
            let _ = writeln!(
                out,
                "LEDGER REFUSED: the store's corporate_actions generation {} is sourced ({}); the \
                 measured ledger never replaces it. The checker below names every step it lacks.",
                receipt.generation, receipt.source
            );
            report_check(out, &draft, &actions);
            return FAILED;
        }
        Slot::Refused(why) if why.kind != reference_data::RefusalKind::Absent => {
            let _ = writeln!(out, "LEDGER REFUSED: {why}");
            return FAILED;
        }
        _ => {}
    }
    // THE RUNS THIS RUN WROTE, read back at their own generation's name:
    // never the newest generation, which a second writer may have landed
    // since, so the ledger always names the runs it was drawn from.
    let runs = match reference_data::load_written(&written, BasisRuns::build) {
        Ok((_, table)) => table,
        Err(why) => {
            let _ = writeln!(
                out,
                "LEDGER REFUSED: the basis runs generation {} just written does not load back: \
                 {why}",
                written.generation
            );
            return FAILED;
        }
    };
    let ledger =
        corporate_actions::measured_draft(&runs, written.generation, &written.sha256, fetched);
    match reference_data::write(&root, &ledger) {
        Ok(landed) => {
            let _ = writeln!(
                out,
                "corporate_actions generation {} {} ({} measured rows) sha256 {}",
                landed.generation,
                if landed.landed {
                    "landed"
                } else {
                    "already held these bytes"
                },
                ledger.rows.len(),
                landed.sha256
            );
            crate::note(
                &telemetry::Event::info("cli.basis_runs", "measured ledger written")
                    .with("generation", landed.generation)
                    .with("landed", landed.landed)
                    .with("rows", u64::try_from(ledger.rows.len()).unwrap_or(u64::MAX)),
            );
        }
        Err(why) => {
            let _ = writeln!(out, "LEDGER FAILED: {why}");
            return FAILED;
        }
    }
    let reloaded =
        reference_data::load_one(&root, Table::CorporateActions, CorporateActions::build);
    report_check(out, &draft, &reloaded);
    OK
}

/// What the steps of one fold came to, by verdict (`pull`'s `judge`, the
/// one owner of the lasting rule, D-7195).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tally {
    /// Every step.
    pub(crate) steps: usize,
    /// Lasting: a ledger row each.
    pub(crate) lasting: usize,
    /// Not proven: within the measurement's noise.
    pub(crate) unproven: usize,
    /// Proven, and the ratio crosses back over it within 90 days.
    pub(crate) reversed: usize,
    /// Proven, with a side that does not hold its basis 5 days.
    pub(crate) short: usize,
    /// Proven, inside the share's own wobble.
    pub(crate) within_wobble: usize,
    /// Shares with at least one anomaly.
    pub(crate) shares_with_anomalies: usize,
    /// Runs ending `step` that make no step: their factor terms pass
    /// 10^18 (`pull::corporate_actions::steps_skipped`), so no verdict.
    pub(crate) steps_skipped: usize,
}

impl Tally {
    /// Every basis anomaly.
    pub(crate) const fn anomalies(&self) -> usize {
        self.reversed + self.short + self.within_wobble
    }
}

/// The month `day` lies in.
fn month_of(day: Day) -> Option<YearMonth> {
    YearMonth::new(day.year(), day.month()).ok()
}

/// For a step whose window `(after, ex_date]` holds a session with no
/// basis: says the day is unknown, and names every month of the window
/// missing from `DIR`. Empty for a step that knows its day. O(1) to find
/// the months ([`Gaps::between`]), then one word per month named.
fn window_note(folded: &Folded, step: &Step) -> String {
    if step.day_known() {
        return String::new();
    }
    let mut note =
        String::from("; ex-date UNKNOWN inside the window: a session with no basis lies between");
    if let (Some(from), Some(to)) = (month_of(step.after), month_of(step.ex_date)) {
        let gaps = folded.missing.between(from, to);
        if !gaps.is_empty() {
            let _ = write!(
                note,
                ", and it spans month(s) missing from DIR: {}",
                gaps.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
    }
    note
}

/// Prints every step of `folded` with its verdict and the summary line, and
/// answers the tally. O(steps).
fn report_steps(out: &mut String, folded: &Folded) -> Tally {
    let mut tally = Tally::default();
    for runs in folded.runs.chunk_by(|a, b| a.symbol == b.symbol) {
        let mut anomalous = false;
        tally.steps_skipped += corporate_actions::steps_skipped(runs);
        for Judged { step, verdict, .. } in corporate_actions::judge(runs) {
            tally.steps += 1;
            let (word, why) = match verdict {
                Verdict::Lasting => {
                    tally.lasting += 1;
                    ("step", String::new())
                }
                Verdict::Unproven => {
                    tally.unproven += 1;
                    ("step_within_noise", String::new())
                }
                Verdict::Anomaly(anomaly) => {
                    anomalous = true;
                    match anomaly {
                        Anomaly::Reversed { .. } => tally.reversed += 1,
                        Anomaly::Short { .. } => tally.short += 1,
                        Anomaly::WithinWobble { .. } => tally.within_wobble += 1,
                    }
                    ("basis_anomaly", format!("; {anomaly}"))
                }
            };
            let _ = writeln!(
                out,
                "{word} {} ({}, {}]: ratio {} to {}, factor {} in [{}, {}]{why}{}",
                step.symbol,
                step.after,
                step.ex_date,
                step.from,
                step.to,
                step.factor,
                step.low,
                step.high,
                window_note(folded, &step)
            );
        }
        if anomalous {
            tally.shares_with_anomalies += 1;
        }
    }
    let _ = writeln!(
        out,
        "steps {}, lasting {} (a ledger row each), basis anomalies {} (reversed {}, short {}, \
         within_wobble {}) on {} share(s), within_noise {}",
        tally.steps,
        tally.lasting,
        tally.anomalies(),
        tally.reversed,
        tally.short,
        tally.within_wobble,
        tally.shares_with_anomalies,
        tally.unproven
    );
    if tally.steps_skipped > 0 {
        // NEVER DROPPED WITHOUT A WORD: a run that ends `step` and makes no
        // step has no verdict, so it is counted here and on /logs.
        let _ = writeln!(
            out,
            "steps_skipped {}: run(s) ending step whose factor terms pass 10^18 make no step and \
             no verdict",
            tally.steps_skipped
        );
    }
    tally
}

/// The checker's findings for the runs `draft` holds against `actions`.
fn report_check(out: &mut String, draft: &Draft, actions: &Slot<CorporateActions>) {
    let parsed = reference_data::render(draft, 0).and_then(|text| {
        reference_data::parse(Table::BasisRuns, Some(0), Path::new("."), text.as_bytes())
            .map_err(|why| why.to_string())
    });
    let runs = match parsed
        .and_then(|parsed| BasisRuns::build(&parsed, Path::new(".")).map_err(|why| why.to_string()))
    {
        Ok(runs) => runs,
        Err(why) => {
            let _ = writeln!(out, "check: the runs do not build as a table: {why}");
            return;
        }
    };
    let table = match actions.table() {
        Ok(table) => table,
        Err(why) => {
            let _ = writeln!(
                out,
                "check: not run, the corporate-action table is not served: {why}"
            );
            return;
        }
    };
    let found: Vec<Finding> = corporate_actions::check(&runs, table);
    let _ = writeln!(
        out,
        "check against corporate_actions ({}, {} rows): {} finding(s)",
        table.standing().word(),
        table.rows(),
        found.len()
    );
    for finding in &found {
        let _ = writeln!(out, "  {finding}");
    }
}

#[cfg(test)]
#[path = "basis_runs_tests.rs"]
mod tests;
