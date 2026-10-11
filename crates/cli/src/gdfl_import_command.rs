//! `cli gdfl-import KIND SOURCE FROM TO [SYMBOL ...]`: GDFL one-second bars
//! into the store, through `pull::gdfl_import`, the one runtime every kind
//! shares (D-2802 to D-2806).
//!
//! # Where the data is read from (D-2804)
//!
//! `SOURCE` is `tickstore` or `zip`, and each names its own root by an
//! environment variable with NO default and no fallback (`CLAUDE.md` §8's
//! rule for configuration; a missing variable refuses and names itself):
//!
//! | Source | Kind | Variable | Names |
//! |---|---|---|---|
//! | `tickstore` | every kind | `BRUTEX_GDFL_TICKSTORE` | the tick store's root |
//! | `zip` | `indices`, `stocks` | `BRUTEX_GDFL_CM_ARCHIVE` | the vendor's capital-market outer zip |
//! | `zip` | `options` | `BRUTEX_GDFL_NFO_ARCHIVES` | the directory of the yearly options zips |
//!
//! One source per run, never both: a refusal from the chosen source is the
//! run's answer. The bars go to the CLI's store root (`BRUTEX_STORE`), which
//! `main` proves exists before anything is dispatched.
//!
//! # What it prints
//!
//! The run's counts and every failure by name; the exit code is `FAILED` when
//! any failure was named, so a script cannot read a partial import as clean.
//! A census refusal stops the run: the receipt of what it did before the
//! refused day is printed, then `GDFL IMPORT REFUSED` naming the census's
//! room, the day's batch and the days imported before it (D-3317 item 7).
//! Every run measures its archive into a `pull::gdfl_import::Origin` and
//! stamps this binary's commit on it, so "run started" and the journal's `run`
//! line say where the bars came from (D-3317 item 4; DB-G56).
//!
//! `--retry-refused`, anywhere after `TO`, reads again every day of the range
//! closed done with refusals (D-3317 item 2); it is never a symbol.
//!
//! # What a capital-market run files (D-3333)
//!
//! An `indices` or `stocks` run files the scope `pull::gdfl_cm::CmScope::stored`
//! names, stored and never swept beside the swept surface: every other
//! applied index row of `core::alias` (FINNIFTY's `NIFTY FIN SERVICE` under
//! `NIFTYFINSERVICE`, and the rest; `INDIA VIX` under `INDIAVIX`, reference
//! only), every share symbol of an applied symbol change (an old name filed
//! under the newest symbol its chain reaches, for the days before each
//! change), and every share underlying the options store holds, read once
//! from `<store>/bars/gdfl/NSE/FNO/` (D-3333 item 2: "widened from
//! `FNO_UNDERLYINGS` to every stored option underlying"). A `stocks` run reads
//! closing-auction eligibility from `<store>/session-masters` (D-3320).
//!
//! `--stored`, anywhere after `TO`, narrows the run to the series D-3333 adds
//! and the swept shares an old name is filed under, as the run's filter: a
//! filter has its own journal key, so days already done for the swept
//! surface are read again for those series only (WP-80 step 4). It is never
//! a symbol.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use pull::gdfl_archive::Archive;
use pull::gdfl_cm::CmScope;
use pull::gdfl_import::{
    Durability, ImportKind, Origin, Report, Run, RunOptions, SourceKind, Workers, run_cm_from,
    run_nfo_from,
};
use pull::gdfl_nfo::{NfoTickStore, NfoZips};
use pull::gdfl_tickstore::TickStore;
use pull::session::Day;

use crate::{FAILED, MISUSED, OK};

/// The tick store's root.
pub(crate) const TICKSTORE_VAR: &str = "BRUTEX_GDFL_TICKSTORE";
/// The vendor's capital-market outer zip.
pub(crate) const CM_ARCHIVE_VAR: &str = "BRUTEX_GDFL_CM_ARCHIVE";
/// The directory of the vendor's yearly options zips.
pub(crate) const NFO_ARCHIVES_VAR: &str = "BRUTEX_GDFL_NFO_ARCHIVES";
/// The flag that reads again the days closed done with refusals (D-3317).
pub(crate) const RETRY_REFUSED: &str = "--retry-refused";
/// The flag that narrows a capital-market run to what D-3333 adds.
pub(crate) const STORED: &str = "--stored";
/// How many workers read and file one day's files (WP-16): optional, 1 to
/// `pull::gdfl_import::Workers::MAX`; absent is the import's default for
/// this host's cores (`Workers::for_host`). The bytes a run leaves do not
/// depend on it.
pub(crate) const WORKERS_VAR: &str = "BRUTEX_GDFL_WORKERS";
/// When a day's writes are made durable (WP-16, D-3353): optional, `day`
/// (the measured default), `day-each-file` or `each-write`. The bytes a run
/// leaves do not depend on it.
pub(crate) const DURABILITY_VAR: &str = "BRUTEX_GDFL_DURABILITY";

/// The store's GDFL options tree, whose underlyings a share run files.
fn options_tree(store: &Path) -> PathBuf {
    store.join("bars").join("gdfl").join("NSE").join("FNO")
}

/// Every underlying the options store holds, sorted: one read of the options
/// tree's directory, O(underlyings). A tree that does not exist yet holds
/// none.
///
/// # Errors
///
/// A tree that cannot be listed, or an entry whose name is not text, named.
fn stored_option_underlyings(store: &Path) -> Result<Vec<String>, String> {
    let tree = options_tree(store);
    let entries = match std::fs::read_dir(&tree) {
        Ok(entries) => entries,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{}: {why}", tree.display())),
    };
    let mut names = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|why| format!("{}: {why}", tree.display()))?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|name| format!("{}: {} is not a symbol", tree.display(), name.display()))?;
        names.insert(name);
    }
    Ok(names.into_iter().collect())
}

/// The scope of a capital-market run of `kind` (D-3333), and how many
/// option underlyings it read.
///
/// # Errors
///
/// What [`stored_option_underlyings`] and `CmScope::stored` refuse.
pub(crate) fn scope_of(kind: ImportKind, store: &Path) -> Result<(CmScope, usize), String> {
    match kind {
        ImportKind::Stocks => {
            let underlyings = stored_option_underlyings(store)?;
            let scope = CmScope::stored(&underlyings)?.with_masters(store.join("session-masters"));
            Ok((scope, underlyings.len()))
        }
        _ => Ok((CmScope::stored(std::iter::empty::<&str>())?, 0)),
    }
}

/// The filter `--stored` stands for: every store symbol the scope files that
/// the swept surface alone does not, sorted. An index row's store key; a
/// stored share's own symbol, and the newest symbol its chain of changes
/// reaches, which is where its old name's days are filed.
fn stored_filter(kind: ImportKind, scope: &CmScope) -> Vec<String> {
    let mut names = BTreeSet::new();
    match kind {
        ImportKind::Indices => {
            for row in scope.indices() {
                names.insert(row.store_key.to_owned());
            }
        }
        _ => {
            for share in scope.shares() {
                names.insert(share.to_string());
                names.insert(brutex_core::alias::LINEAGE.current(share).to_owned());
            }
        }
    }
    names.into_iter().collect()
}

/// WHAT A RUN FILES (D-3333), and the receipt's sentence saying so: the
/// swept surface alone for an options run; for a capital-market run the
/// stored scope (the other index rows, the old names and every share
/// underlying the options store holds), `--stored` narrowing `only` to what
/// it adds, under a filter of its own.
///
/// # Errors
///
/// What [`scope_of`] refuses.
fn scope_for(
    kind: ImportKind,
    store: &Path,
    stored_only: bool,
    only: &mut Vec<String>,
) -> Result<(Arc<CmScope>, Option<String>), String> {
    if kind == ImportKind::Options {
        return Ok((Arc::new(CmScope::swept()), None));
    }
    let (scope, underlyings) = scope_of(kind, store)?;
    if stored_only {
        only.extend(stored_filter(kind, &scope));
    }
    let words = format!(
        "stored (D-3333): {} index rows and {} shares beyond the F&O list; {} option \
         underlyings read from {}",
        scope.indices().len(),
        scope.shares().len(),
        underlyings,
        options_tree(store).display()
    );
    Ok((Arc::new(scope), Some(words)))
}

/// The words after `TO`: whether `--retry-refused` and `--stored` were
/// given (each a flag anywhere among them, never a symbol), and the symbols.
fn flags_of(words: &[&str]) -> (bool, bool, Vec<String>) {
    let symbols = words
        .iter()
        .filter(|word| **word != RETRY_REFUSED && **word != STORED)
        .map(|word| (*word).to_owned())
        .collect();
    (
        words.contains(&RETRY_REFUSED),
        words.contains(&STORED),
        symbols,
    )
}

/// `YYYY-MM-DD` as a day, or why not.
pub(crate) fn day(text: &str) -> Result<Day, String> {
    let parts: Vec<&str> = text.split('-').collect();
    let number = |part: &str, width: usize| {
        (part.len() == width && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse::<u16>().ok())
            .flatten()
    };
    let parsed = match parts.as_slice() {
        [y, m, d] => number(y, 4).zip(number(m, 2)).zip(number(d, 2)),
        _ => None,
    };
    let ((y, m), d) = parsed.ok_or_else(|| format!("{text:?} is not a YYYY-MM-DD day"))?;
    let (m, d) = (u8::try_from(m).unwrap_or(0), u8::try_from(d).unwrap_or(0));
    Day::new(y, m, d).map_err(|why| format!("{text:?}: {why}"))
}

/// The workers `BRUTEX_GDFL_WORKERS` asks for, or the import's default when
/// it is not set; a value that is not a count from 1 to the bound is refused
/// by name, never read as the default.
pub(crate) fn workers_of(env: &dyn Fn(&str) -> Option<OsString>) -> Result<Workers, String> {
    let Some(value) = env(WORKERS_VAR) else {
        return Ok(Workers::for_host());
    };
    value
        .to_str()
        .and_then(|text| text.parse::<usize>().ok())
        .ok_or_else(|| {
            format!(
                "{WORKERS_VAR}={} is not a count of workers",
                value.display()
            )
        })
        .and_then(Workers::new)
        .map_err(|why| format!("{WORKERS_VAR}: {why}"))
}

/// The durability `BRUTEX_GDFL_DURABILITY` names, or the import's default
/// when it is not set; any other word is refused by name, never read as the
/// default (WP-16, D-3353).
pub(crate) fn durability_of(env: &dyn Fn(&str) -> Option<OsString>) -> Result<Durability, String> {
    let Some(value) = env(DURABILITY_VAR) else {
        return Ok(Durability::DEFAULT);
    };
    value
        .to_str()
        .ok_or_else(|| format!("{} is not text", value.display()))
        .and_then(Durability::parse)
        .map_err(|why| format!("{DURABILITY_VAR}: {why}"))
}

/// The run's options: the origin it measured, whether it reads again the
/// days done with refusals, its workers and when a day is made durable
/// (WP-16).
const fn options_of(
    origin: &Origin,
    retry_refused: bool,
    (workers, durability): (Workers, Durability),
) -> RunOptions<'_> {
    RunOptions {
        origin: Some(origin),
        retry_refused,
        workers,
        durability,
    }
}

/// The root a variable names, or the refusal naming the variable.
pub(crate) fn root(var: &str, env: &dyn Fn(&str) -> Option<OsString>) -> Result<PathBuf, String> {
    match env(var) {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => Err(format!(
            "{var} is not set; it names where the GDFL data is read from and has no default"
        )),
    }
}

/// What the verb's first words and the environment ask for: the kind, the
/// first and last day, and how the run files its days (WP-16).
///
/// # Errors
///
/// The words a refusal shows, for a kind, a day, a worker count or a
/// durability that is not one.
fn asked(
    (kind, from, to): (&str, &str, &str),
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Result<(ImportKind, Day, Day, (Workers, Durability)), String> {
    let kind = ImportKind::parse(kind).map_err(|why| why.to_string())?;
    Ok((
        kind,
        day(from)?,
        day(to)?,
        (workers_of(env)?, durability_of(env)?),
    ))
}

/// The verb, with its environment and store root passed in so every arm is
/// driven by a test without touching the process environment.
pub(crate) fn command_with(
    args: &[&str],
    env: &dyn Fn(&str) -> Option<OsString>,
    store: &dyn Fn() -> Result<PathBuf, String>,
    out: &mut String,
) -> u8 {
    let [kind, source, from, to, only @ ..] = args else {
        let _ = writeln!(
            out,
            "refused: gdfl-import takes KIND SOURCE FROM TO [SYMBOL ...]"
        );
        return MISUSED;
    };
    let (kind, from, to, workers) = match asked((kind, from, to), env) {
        Ok(four) => four,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            return MISUSED;
        }
    };
    let var = match (*source, kind) {
        ("tickstore", _) => TICKSTORE_VAR,
        ("zip", ImportKind::Options) => NFO_ARCHIVES_VAR,
        ("zip", _) => CM_ARCHIVE_VAR,
        (other, _) => {
            let _ = writeln!(
                out,
                "refused: {other:?} is not a source; the sources are tickstore and zip"
            );
            return MISUSED;
        }
    };
    let (retry_refused, stored_only, mut only) = flags_of(only);
    if stored_only && kind == ImportKind::Options {
        let _ = writeln!(
            out,
            "refused: {STORED} narrows a capital-market run; an options run files no stored series"
        );
        return MISUSED;
    }
    let roots = root(var, env).and_then(|data| Ok((data, store()?)));
    let result = roots.and_then(|(data, store)| {
        let (scope, scope_words) = scope_for(kind, &store, stored_only, &mut only)?;
        let run = Run {
            kind,
            from,
            to,
            only: &only,
            store_root: &store,
        };
        // WHERE THE DATA CAME FROM AND WHAT BUILT IT (D-3317 item 4; DB-G56):
        // the source, the archive's path, bytes and identity, and this
        // binary's commit, in "run started" and the journal's `run` line.
        let origin_kind = if *source == "tickstore" {
            SourceKind::TickStore
        } else {
            SourceKind::Zip
        };
        let origin = Origin::measure(origin_kind, &data, crate::commit_stamp());
        let options = options_of(&origin, retry_refused, workers);
        let report = match (*source, kind) {
            ("tickstore", ImportKind::Options) => {
                run_nfo_from(&NfoTickStore::new(&data), &run, options)
            }
            ("tickstore", _) => run_cm_from(
                &TickStore::new(&data).with_scope(Arc::clone(&scope)),
                &run,
                options,
            ),
            ("zip", ImportKind::Options) => run_nfo_from(&NfoZips::new(&data), &run, options),
            _ => {
                let archive = Archive::open(&data)
                    .map_err(|w| format!("{}: {w}", data.display()))?
                    .with_scope(Arc::clone(&scope));
                run_cm_from(&archive, &run, options)
            }
        };
        Ok((report, store, scope_words))
    });
    match result {
        Ok((Ok(report), store, scope_words)) => {
            render(out, kind, source, &store, &report);
            render_how(out, workers);
            if let Some(words) = scope_words {
                let _ = writeln!(out, "  scope               {words}");
            }
            if report.failures.is_empty() {
                OK
            } else {
                FAILED
            }
        }
        Ok((Err(refusal), store, _)) => {
            // A census refusal stopped the run part way (D-3317 item 7): what
            // it did before the refused day is on disk, so it is printed too.
            if let Some(done) = refusal.report() {
                render(out, kind, source, &store, done);
                render_how(out, workers);
            }
            let _ = writeln!(out, "GDFL IMPORT REFUSED: {refusal}");
            FAILED
        }
        Err(why) => {
            let _ = writeln!(out, "GDFL IMPORT REFUSED: {why}");
            FAILED
        }
    }
}

/// How the run filed its days (WP-16): its workers and when each day was
/// made durable, each with the variable that sets it.
fn render_how(out: &mut String, (workers, durability): (Workers, Durability)) {
    let _ = writeln!(
        out,
        "  workers             {} ({WORKERS_VAR}, 1 to {})",
        workers.get(),
        Workers::MAX
    );
    let _ = writeln!(
        out,
        "  durability          {} ({DURABILITY_VAR}; D-3353)",
        durability.as_str()
    );
}

/// The process environment, read at the edge.
fn process_env(var: &str) -> Option<OsString> {
    std::env::var_os(var)
}

/// The CLI's store root, proved as every other verb proves it.
fn cli_store() -> Result<PathBuf, String> {
    crate::store_root()
}

/// The run's receipt.
fn render(out: &mut String, kind: ImportKind, source: &str, store: &Path, report: &Report) {
    let _ = writeln!(
        out,
        "GDFL IMPORT -- {} from {source} into {}",
        kind.as_str(),
        store.display()
    );
    let _ = writeln!(
        out,
        "  rung                1s only (D-2802: no coarser rung is derived from gdfl)"
    );
    let _ = writeln!(out, "  days imported       {}", report.days_imported);
    let _ = writeln!(out, "  days already done   {}", report.days_skipped);
    let _ = writeln!(out, "  days closed         {}", report.days_closed);
    let _ = writeln!(out, "  days refused        {}", report.days_refused);
    let days = |list: &[Day]| {
        list.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    };
    let _ = writeln!(
        out,
        "  days not in source  {} {}",
        report.days_missing.len(),
        days(&report.days_missing)
    );
    let _ = writeln!(
        out,
        "  days resumed        {} {}",
        report.resumed.len(),
        days(&report.resumed)
    );
    let _ = writeln!(
        out,
        "  days done, refused  {} {}",
        report.closed_with_refusals.len(),
        days(&report.closed_with_refusals)
    );
    let _ = writeln!(
        out,
        "  days incomplete     {} {}",
        report.closed_incomplete.len(),
        days(&report.closed_incomplete)
    );
    let _ = writeln!(
        out,
        "  days all refused    {} {}",
        report.closed_refused.len(),
        days(&report.closed_refused)
    );
    let _ = writeln!(out, "  files read          {}", report.files);
    let _ = writeln!(out, "  files not wanted    {}", report.files_skipped);
    let _ = writeln!(out, "  files refused       {}", report.files_refused);
    let _ = writeln!(out, "  rows                {}", report.rows);
    let _ = writeln!(out, "  rows LTQ=0 dropped  {}", report.ltq_zero_dropped);
    let _ = writeln!(out, "  late rows deferred  {}", report.late_rows);
    let _ = writeln!(out, "  late rows dropped   {}", report.late_unresolved);
    let _ = writeln!(out, "  seconds offered     {}", report.seconds);
    let _ = writeln!(out, "  seconds written     {}", report.seconds_committed);
    // A DAY INSIDE WHAT ITS MONTH HOLDS (D-3350; DB-N26): each such month is
    // a new generation, journaled in `<store>/audit/gdfl.repairs`.
    let _ = writeln!(
        out,
        "  months repaired     {} (a new generation each; audit/gdfl.repairs)",
        report.months_repaired
    );
    let _ = writeln!(out, "  outside session     {}", report.outside_session);
    let _ = writeln!(out, "  failures            {}", report.failures.len());
    for failure in &report.failures {
        let _ = writeln!(out, "    {}: {}", failure.instrument, failure.why);
    }
}

/// The verb as dispatched: the process environment and the CLI's store root.
pub(crate) fn command(args: &[&str], out: &mut String) -> u8 {
    command_with(args, &process_env, &cli_store, out)
}

#[cfg(test)]
#[path = "gdfl_import_command_tests.rs"]
mod tests;
