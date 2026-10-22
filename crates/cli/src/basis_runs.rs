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
//! run and is named.
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
    self, BasisDay, BasisRuns, CorporateActions, Finding, Folder, Ratio, Run, RunEnd,
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

/// The most months one run reads: 2000-01 to 2099-12.
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
/// a month that is not `YYYY-MM`, `--from` after `--to`, or `--ledger`
/// without `--apply`; each by name.
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
    pub(crate) missing: Vec<YearMonth>,
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
/// A directory that cannot be listed, no month in range, or a version file
/// DATA-1's reader refuses (cut, damaged, of another format); each by name.
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
    let mut folders: BTreeMap<String, Folder> = BTreeMap::new();
    let mut seen_days: BTreeMap<String, u64> = BTreeMap::new();
    let mut refused: BTreeMap<String, String> = BTreeMap::new();
    let mut skipped: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut days_folded = 0_u64;
    let mut read = Vec::new();
    let mut missing = Vec::new();
    // The months held, walked beside the calendar: one compare a month.
    let mut held_months = in_range.iter().copied().peekable();
    let mut at = Some(first);
    while let Some(month) = at.filter(|m| *m <= last) {
        at = next_month(month);
        if held_months.next_if_eq(&month).is_none() {
            missing.push(month);
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
        folded.missing.len(),
    );
    if let Some(gap) = folded.missing.first() {
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
        folded.missing.len(),
        if folded.missing.is_empty() {
            String::new()
        } else {
            format!(
                ": {}",
                folded
                    .missing
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
    let (mut steps, mut proven) = (0_usize, 0_usize);
    for runs in folded.runs.chunk_by(|a, b| a.symbol == b.symbol) {
        for step in corporate_actions::steps_of(runs) {
            steps += 1;
            // PROVEN: the factor's interval excludes one, so the step makes a
            // ledger row; otherwise it ends a run and makes none.
            let word = if step.proven() {
                proven += 1;
                "step"
            } else {
                "step_within_noise"
            };
            let _ = writeln!(
                out,
                "{word} {} ({}, {}]: ratio {} to {}, factor {} in [{}, {}]",
                step.symbol,
                step.after,
                step.ex_date,
                step.from,
                step.to,
                step.factor,
                step.low,
                step.high
            );
        }
    }
    let _ = writeln!(
        out,
        "steps {steps}, proven {proven} (a ledger row each), within_noise {}",
        steps - proven
    );
    let Some(fetched) = today else {
        let _ = writeln!(
            out,
            "BASIS RUNS FAILED: the clock gives no IST day. Nothing was written."
        );
        return FAILED;
    };
    let draft = runs_draft(&ask, &folded, fetched);
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
    let runs = match reference_data::load_one(&root, Table::BasisRuns, BasisRuns::build) {
        Slot::Store { table, .. } => table,
        other => {
            let _ = writeln!(
                out,
                "LEDGER REFUSED: the basis runs just written do not load back: {:?}",
                other.table().err()
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
