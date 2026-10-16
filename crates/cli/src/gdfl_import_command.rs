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

pub(crate) use crate::store_write::{ALLOW_UNSTAMPED_VAR, unstamped_write_refusal};

/// The data root `var` names, once this binary may write at all: an
/// unstamped build is refused by [`unstamped_write_refusal`] before any root
/// is read (DB-B71).
fn stamped_root(var: &str, env: &dyn Fn(&str) -> Option<OsString>) -> Result<PathBuf, String> {
    if let Some(why) =
        unstamped_write_refusal(crate::commit_stamp(), env(ALLOW_UNSTAMPED_VAR).as_deref())
    {
        return Err(why);
    }
    root(var, env)
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
    // A STORE WHOSE `bars` IS A LINK IS NOT WRITTEN (R3-16, RA-298): asked
    // once, by name, before the run reads a day (`store_write::writable`).
    let roots = stamped_root(var, env).and_then(|data| {
        let store = store()?;
        crate::store_write::writable(&store)?;
        Ok((data, store))
    });
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
    // WHAT THE LINEAGE WROTE (D-3355; DB-B13): an event the run detected is
    // in the receipt, not only in the ledger. A capital-market run says 0.
    let _ = writeln!(
        out,
        "  adjustments found   {} (ex-date events; imports/gdfl.lineage)",
        report.adjustments
    );
    let _ = writeln!(out, "  lineage records     {}", report.lineage_lines);
    let _ = writeln!(out, "  lot records         {}", report.lot_lines);
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

/// THE CLOUD MUTATION PASS (`reports/cloud-mut-cli-20261016.md`): each test
/// pins the behaviour a mutant of this file broke while every other test
/// stayed green, or runs a line no test reached. No vendor byte: the one
/// archive here is built at run time from invented values.
#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]
mod cloud_mutant_tests {
    use std::fmt::Write as _;
    use std::os::unix::ffi::OsStrExt as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    use pull::gdfl_cm::CmKind;

    use super::*;

    /// A fresh directory unique to this test and call.
    fn scratch(tag: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "brutex-cli-gdfl-cloud-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// `pairs` as an environment, with the allowance a test build needs to
    /// write: a test binary built from a dirty tree carries no commit stamp.
    fn env_of(pairs: &[(&str, OsString)]) -> impl Fn(&str) -> Option<OsString> + use<> {
        let mut pairs: Vec<(String, OsString)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect();
        pairs.push((ALLOW_UNSTAMPED_VAR.to_owned(), OsString::from("1")));
        move |var| pairs.iter().find(|(k, _)| k == var).map(|(_, v)| v.clone())
    }

    /// A plain zip of `members`, every one stored (method 0).
    fn zip_of(members: &[(&str, &[u8])]) -> Vec<u8> {
        let le16 = |out: &mut Vec<u8>, v: usize| {
            out.extend_from_slice(&u16::try_from(v).unwrap().to_le_bytes());
        };
        let le32 = |out: &mut Vec<u8>, v: usize| {
            out.extend_from_slice(&u32::try_from(v).unwrap().to_le_bytes());
        };
        let (mut out, mut central) = (Vec::new(), Vec::new());
        for (name, data) in members {
            let (at, crc) = (out.len(), pull::gdfl_archive::crc32(data));
            le32(&mut out, 0x0403_4b50);
            le16(&mut out, 20);
            le16(&mut out, 0);
            le16(&mut out, 0);
            le32(&mut out, 0);
            out.extend_from_slice(&crc.to_le_bytes());
            le32(&mut out, data.len());
            le32(&mut out, data.len());
            le16(&mut out, name.len());
            le16(&mut out, 0);
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);
            le32(&mut central, 0x0201_4b50);
            le16(&mut central, 20);
            le16(&mut central, 20);
            le16(&mut central, 0);
            le16(&mut central, 0);
            le32(&mut central, 0);
            central.extend_from_slice(&crc.to_le_bytes());
            le32(&mut central, data.len());
            le32(&mut central, data.len());
            le16(&mut central, name.len());
            for _ in 0..4 {
                le16(&mut central, 0);
            }
            le32(&mut central, 0);
            le32(&mut central, at);
            central.extend_from_slice(name.as_bytes());
        }
        let at = out.len();
        out.extend_from_slice(&central);
        le32(&mut out, 0x0605_4b50);
        le16(&mut out, 0);
        le16(&mut out, 0);
        le16(&mut out, members.len());
        le16(&mut out, members.len());
        le32(&mut out, central.len());
        le32(&mut out, at);
        le16(&mut out, 0);
        out
    }

    /// An invented NIFTY index file of `day`: a pre-open row, session rows
    /// through the day and a row after the close, at a level NIFTY never
    /// printed.
    fn index_csv(day: Day) -> Vec<u8> {
        let open = 9 * 3_600 + 15 * 60;
        let close = 15 * 3_600 + 30 * 60;
        let mut seconds = vec![open - 300];
        seconds.extend((0..22_500).step_by(997).map(|s| open + s));
        seconds.push(close + 60);
        let mut text = String::from(pull::gdfl_cm::HEADER_OPEN_INTEREST);
        for (k, sod) in seconds.into_iter().enumerate() {
            let _ = write!(
                text,
                "\r\nNIFTY 50.NSE_IDX,{:02}/{:02}/{:04},{:02}:{:02}:{:02},4242.{:02},0,0,0,0,0,0",
                day.day(),
                day.month(),
                day.year(),
                sod / 3_600,
                sod / 60 % 60,
                sod % 60,
                k % 100
            );
        }
        text.push_str("\r\n");
        text.into_bytes()
    }

    /// A capital-market outer archive holding `day`'s index zip, whose one
    /// member is the invented NIFTY file.
    fn cm_archive(day: Day) -> Vec<u8> {
        let folder = pull::gdfl_cm::day_folder_name(CmKind::Indices, day);
        let csv = index_csv(day);
        let inner = zip_of(&[(&format!("{folder}/NIFTY 50.NSE_IDX.csv"), &csv)]);
        zip_of(&[(
            &pull::gdfl_archive::day_zip_name(CmKind::Indices, day),
            &inner,
        )])
    }

    /// **A CENSUS REFUSAL PRINTS WHAT THE RUN DID BEFORE IT, THEN THE
    /// REFUSAL** (D-3317 item 7): the run's receipt and how it filed its
    /// days come first, the refusal last, exit FAILED. The census here is a
    /// directory, which the census cannot read.
    #[test]
    fn a_census_refusal_prints_the_receipt_and_how_it_filed_then_the_refusal() {
        let root = scratch("census");
        let store = root.join("store");
        std::fs::create_dir_all(store.join("manifest").join("gdfl.man")).unwrap();
        let cm = root.join("cm.zip");
        std::fs::write(&cm, cm_archive(Day::new(2024, 4, 1).unwrap())).unwrap();
        let env = env_of(&[
            (CM_ARCHIVE_VAR, cm.into_os_string()),
            (WORKERS_VAR, OsString::from("1")),
            (DURABILITY_VAR, OsString::from("each-write")),
        ]);
        let mut out = String::new();
        let code = command_with(
            &["indices", "zip", "2024-04-01", "2024-04-01", "NIFTY"],
            &env,
            &|| Ok(store.clone()),
            &mut out,
        );
        assert_eq!(code, FAILED, "{out}");
        let at = |part: &str| {
            out.find(part)
                .unwrap_or_else(|| panic!("{part:?} in {out}"))
        };
        let receipt = at(&format!(
            "GDFL IMPORT -- indices from zip into {}\n",
            store.display()
        ));
        let workers = at(&format!(
            "  workers             1 ({WORKERS_VAR}, 1 to {})\n",
            Workers::MAX
        ));
        let durability = at(&format!(
            "  durability          each-write ({DURABILITY_VAR}; D-3353)\n"
        ));
        let refused = at("GDFL IMPORT REFUSED: ");
        assert!(
            [receipt, workers, durability, refused].is_sorted(),
            "the receipt, its workers and durability, then the refusal: {out}"
        );
        assert!(
            out.get(refused..)
                .is_some_and(|tail| tail.contains("census")),
            "{out}"
        );
        assert!(
            !out.contains("  scope  "),
            "a refused run prints no scope: {out}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// The receipt says how the run filed its days: the workers and the
    /// durability it was given, each beside the variable that sets it.
    #[test]
    fn the_receipt_names_the_workers_and_durability_it_filed_with() {
        let mut out = String::new();
        render_how(
            &mut out,
            (
                Workers::new(3).unwrap(),
                Durability::parse("day-each-file").unwrap(),
            ),
        );
        assert_eq!(
            out,
            format!(
                "  workers             3 ({WORKERS_VAR}, 1 to {})\n  durability          \
                 day-each-file ({DURABILITY_VAR}; D-3353)\n",
                Workers::MAX
            )
        );
        // And the verb prints exactly what it was asked for, after the
        // receipt, on a clean run.
        let root = scratch("how");
        let store = root.join("store");
        std::fs::create_dir_all(&store).unwrap();
        let env = env_of(&[
            (TICKSTORE_VAR, root.join("ts").into_os_string()),
            (WORKERS_VAR, OsString::from("2")),
            (DURABILITY_VAR, OsString::from("each-write")),
        ]);
        let mut out = String::new();
        let code = command_with(
            &["options", "tickstore", "2024-04-01", "2024-04-01", "NIFTY"],
            &env,
            &|| Ok(store.clone()),
            &mut out,
        );
        assert_eq!(code, OK, "{out}");
        assert!(
            out.contains(&format!(
                "  failures            0\n  workers             2 ({WORKERS_VAR}, 1 to {})\n  \
                 durability          each-write ({DURABILITY_VAR}; D-3353)\n",
                Workers::MAX
            )),
            "{out}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// The process environment is read as it is: a variable this test
    /// process holds comes back with its value.
    #[test]
    fn the_process_environment_is_read_as_it_is() {
        let (name, value) = std::env::vars_os()
            .find_map(|(name, value)| {
                let name = name.into_string().ok()?;
                (!value.is_empty()).then_some((name, value))
            })
            .expect("a test process carries a set, non-empty variable");
        assert_eq!(process_env(&name), Some(value));
    }

    /// The store root is the CLI's own, proved as every verb proves it.
    #[test]
    fn the_store_root_is_the_clis_own() {
        assert_eq!(cli_store(), crate::store_root());
    }

    /// An options tree that cannot be listed is named, never read as one
    /// that holds no underlying; and an underlying whose name is not text
    /// is refused by name.
    #[test]
    fn an_options_tree_that_cannot_be_listed_or_named_is_refused() {
        let root = scratch("tree");
        let tree = options_tree(&root);
        std::fs::create_dir_all(tree.parent().unwrap()).unwrap();
        std::fs::write(&tree, b"a file where the tree belongs").unwrap();
        let why = stored_option_underlyings(&root).expect_err("a file is not a tree");
        assert!(why.starts_with(&tree.display().to_string()), "{why}");
        std::fs::remove_file(&tree).unwrap();

        let odd = std::ffi::OsStr::from_bytes(b"NIFTY\xff");
        std::fs::create_dir_all(tree.join(odd)).unwrap();
        let why = stored_option_underlyings(&root).expect_err("a name that is not text");
        assert!(why.contains("is not a symbol"), "{why}");
        std::fs::remove_dir_all(root).unwrap();
    }
}
