//! `cli gdfl-dayflags KIND FROM TO [--lineage]`: the GDFL day ledger of days
//! already imported (D-3335 §1; WP-11b; findings DB-026, DB-029, DB-030,
//! DB-056, DB-G96), and with `--lineage` the corporate-action lineage of
//! their options (D-3331, D-3343; DB-G71, DB-G72, DB-G77).
//!
//! # Why it exists
//!
//! The import writes a day's quality fields as it files the day, so every day
//! imported before WP-11b has none: 30 May 2019, 2021-05-21, 2022-03-07 and
//! the October 2020 UTC bursts were all imported before the ledger existed.
//! This verb reads the vendor's files of those days again, READ-ONLY, and
//! writes their `flags ... src=rescan` lines (WP-80 step 6).
//!
//! # What it reads and what it writes
//!
//! It reads the vendor's zips, as D-3335 names them: `KIND` is `indices` or
//! `stocks` from `BRUTEX_GDFL_CM_ARCHIVE` (the capital-market outer zip), or
//! `options` from `BRUTEX_GDFL_NFO_ARCHIVES` (the directory of yearly zips),
//! each with no default (`CLAUDE.md` §8's rule), and the import journal under
//! the CLI's store root, `BRUTEX_STORE`. A day is read only when the journal
//! records it as imported (`done`, `recounted` or `incomplete` under any
//! filter); every other day of the range is counted, never read.
//!
//! It writes `<store>/imports/gdfl.dayflags` and nothing else: no bar, no
//! census, no journal line, no auction sidecar (`pull::gdfl_import::rescan_cm`
//! reads a share's closing-auction prints and files none). With `--lineage`,
//! an options run also appends to `<store>/imports/gdfl.lineage` and
//! `<store>/imports/gdfl.lots`. A line the ledger already holds, `src`
//! aside, is not written again, so a second run over the same days writes
//! nothing.
//!
//! A capital-market run reads under the same scope `cli gdfl-import` files
//! (D-3333): every applied index row, every share an option underlying
//! names, closing-auction eligibility from `<store>/session-masters`.
//!
//! # Exit code
//!
//! `FAILED` when any day or file was refused or a ledger would not take its
//! lines, each named; `MISUSED` for arguments that are not the verb's.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use pull::gdfl_archive::Archive;
use pull::gdfl_import::{ImportKind, Rescan, rescan_cm, rescan_nfo};
use pull::gdfl_nfo::NfoZips;
use pull::session::Day;

use crate::gdfl_import_command::{CM_ARCHIVE_VAR, NFO_ARCHIVES_VAR, day, root};
use crate::{FAILED, MISUSED, OK};
use pull::gdfl_run::scope::of as scope_of;

/// The flag that writes the options lineage too (D-3331, D-3343).
pub(crate) const LINEAGE: &str = "--lineage";

/// The verb's shape, for every refusal of its arguments.
pub(crate) const USAGE: &str = "gdfl-dayflags takes KIND FROM TO [--lineage]: KIND is indices, \
     stocks or options, FROM and TO are YYYY-MM-DD, and --lineage is for options only";

/// What the arguments ask: the kind, the range and whether to write the
/// lineage.
fn parsed(args: &[&str]) -> Result<(ImportKind, Day, Day, bool), String> {
    let [kind, from, to, rest @ ..] = args else {
        return Err(USAGE.to_owned());
    };
    let kind = ImportKind::parse(kind).map_err(|why| why.to_string())?;
    let (from, to) = (day(from)?, day(to)?);
    let lineage = match rest {
        [] => false,
        [flag] if *flag == LINEAGE => true,
        _ => return Err(USAGE.to_owned()),
    };
    if lineage && kind != ImportKind::Options {
        return Err(format!(
            "{LINEAGE} is for options: the lineage is the option contracts' (D-3331)"
        ));
    }
    if from > to {
        return Err(format!("the range {from}..{to} runs backwards"));
    }
    Ok((kind, from, to, lineage))
}

/// The verb, with its environment and store root passed in so every arm is
/// driven by a test without touching the process environment.
pub(crate) fn command_with(
    args: &[&str],
    env: &dyn Fn(&str) -> Option<OsString>,
    store: &dyn Fn() -> Result<PathBuf, String>,
    out: &mut String,
) -> u8 {
    let (kind, from, to, lineage) = match parsed(args) {
        Ok(asked) => asked,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            return MISUSED;
        }
    };
    let var = if kind == ImportKind::Options {
        NFO_ARCHIVES_VAR
    } else {
        CM_ARCHIVE_VAR
    };
    let result = root(var, env)
        .and_then(|data| Ok((data, store()?)))
        .and_then(|(data, store)| {
            let done = rescan(kind, (from, to), &data, &store, lineage)?;
            Ok((done, store))
        });
    match result {
        Ok((Ok(done), store)) => {
            render(out, kind, (from, to), &store, &done);
            if done.failures.is_empty() { OK } else { FAILED }
        }
        Ok((Err(refusal), _)) => {
            let _ = writeln!(out, "GDFL DAYFLAGS REFUSED: {refusal}");
            FAILED
        }
        Err(why) => {
            let _ = writeln!(out, "GDFL DAYFLAGS REFUSED: {why}");
            FAILED
        }
    }
}

/// The rescan of `kind` over `range` from the zips at `data` into the
/// ledgers under `store`.
///
/// # Errors
///
/// The outer `Err` names an archive or a scope that cannot be opened; the
/// inner one is the rescan's own run-level refusal.
fn rescan(
    kind: ImportKind,
    range: (Day, Day),
    data: &Path,
    store: &Path,
    lineage: bool,
) -> Result<Result<Rescan, pull::gdfl_import::ImportRefusal>, String> {
    if kind == ImportKind::Options {
        return Ok(rescan_nfo(&NfoZips::new(data), range, store, lineage));
    }
    let (scope, _) = scope_of(kind, store)?;
    let archive = Archive::open(data)
        .map_err(|why| format!("{}: {why}", data.display()))?
        .with_scope(Arc::new(scope));
    Ok(rescan_cm(&archive, kind, range, store))
}

/// The rescan's receipt.
fn render(out: &mut String, kind: ImportKind, (from, to): (Day, Day), store: &Path, done: &Rescan) {
    let _ = writeln!(
        out,
        "GDFL DAYFLAGS -- {} {from}..{to}, read-only from the vendor's zips, into {}",
        kind.as_str(),
        pull::gdfl_import::quality::dayflags_path(store).display()
    );
    let _ = writeln!(out, "  days read           {}", done.days_read);
    let _ = writeln!(out, "  days not imported   {}", done.days_not_imported);
    let missing: Vec<String> = done.days_missing.iter().map(ToString::to_string).collect();
    let _ = writeln!(
        out,
        "  days not in source  {} {}",
        done.days_missing.len(),
        missing.join(" ")
    );
    let _ = writeln!(out, "  files read          {}", done.files);
    let _ = writeln!(out, "  files refused       {}", done.files_refused);
    let _ = writeln!(out, "  flags lines written {}", done.flags_lines);
    let _ = writeln!(out, "  flags unchanged     {}", done.flags_unchanged);
    if kind == ImportKind::Options {
        let _ = writeln!(out, "  lineage records     {}", done.lineage_lines);
        let _ = writeln!(out, "  lot records         {}", done.lot_lines);
        let _ = writeln!(out, "  adjustments found   {}", done.adjustments);
    }
    let _ = writeln!(out, "  failures            {}", done.failures.len());
    for failure in &done.failures {
        let _ = writeln!(out, "    {}: {}", failure.instrument, failure.why);
    }
}

/// The process environment, read at the edge.
fn process_env(var: &str) -> Option<OsString> {
    std::env::var_os(var)
}

/// The verb as dispatched: the process environment and the CLI's store root.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    // AN UNSTAMPED BUILD DOES NOT WRITE THE DAY LEDGER (DB-B71, WP-81).
    let store = crate::store_write::root_for("gdfl-dayflags", arguments);
    command_with(arguments, &process_env, &store, out)
}

#[cfg(test)]
#[path = "gdfl_dayflags_tests.rs"]
mod tests;
