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

use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pull::gdfl_archive::Archive;
use pull::gdfl_import::{
    ImportKind, Origin, Report, Run, RunOptions, SourceKind, run_cm_from, run_nfo_from,
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

/// `YYYY-MM-DD` as a day, or why not.
fn day(text: &str) -> Result<Day, String> {
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

/// The root a variable names, or the refusal naming the variable.
fn root(var: &str, env: &dyn Fn(&str) -> Option<OsString>) -> Result<PathBuf, String> {
    match env(var) {
        Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
        _ => Err(format!(
            "{var} is not set; it names where the GDFL data is read from and has no default"
        )),
    }
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
    let parsed = ImportKind::parse(kind)
        .map_err(|why| why.to_string())
        .and_then(|kind| Ok((kind, day(from)?, day(to)?)));
    let (kind, from, to) = match parsed {
        Ok(three) => three,
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
    let retry_refused = only.contains(&RETRY_REFUSED);
    let only: Vec<String> = only
        .iter()
        .filter(|word| **word != RETRY_REFUSED)
        .map(|word| (*word).to_owned())
        .collect();
    let roots = root(var, env).and_then(|data| Ok((data, store()?)));
    let result = roots.and_then(|(data, store)| {
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
        let options = RunOptions {
            origin: Some(&origin),
            retry_refused,
        };
        let report = match (*source, kind) {
            ("tickstore", ImportKind::Options) => {
                run_nfo_from(&NfoTickStore::new(&data), &run, options)
            }
            ("tickstore", _) => run_cm_from(&TickStore::new(&data), &run, options),
            ("zip", ImportKind::Options) => run_nfo_from(&NfoZips::new(&data), &run, options),
            _ => {
                let archive =
                    Archive::open(&data).map_err(|w| format!("{}: {w}", data.display()))?;
                run_cm_from(&archive, &run, options)
            }
        };
        Ok((report, store))
    });
    match result {
        Ok((Ok(report), store)) => {
            render(out, kind, source, &store, &report);
            if report.failures.is_empty() {
                OK
            } else {
                FAILED
            }
        }
        Ok((Err(refusal), store)) => {
            // A census refusal stopped the run part way (D-3317 item 7): what
            // it did before the refused day is on disk, so it is printed too.
            if let Some(done) = refusal.report() {
                render(out, kind, source, &store, done);
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
