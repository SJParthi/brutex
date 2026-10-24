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
//! # The door (AF-01, D-4977)
//!
//! Before the importer may write a journal line, [`guard`] refuses a range
//! that runs backwards, a source root that is not what its source needs
//! (`source_root_unknown`) and a second run of the same kind (`run_busy`,
//! the kind's lock beneath `imports/`), and reads what the last run of the
//! same key read from `imports/gdfl.sources`. After it returns, a run that
//! read no file while it owed days (`no_file_read`), a changed zip with days
//! skipped unread (`source_changed_since_last_run`) and a root that is gone
//! (`store_lost`, `source_lost`) each end it `FAILED`, one exit line each.
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

//!
//! # One authority (IA-1, D-4800)
//!
//! This verb keeps only its argument and environment parsing. What a run
//! files, which reader opens which source, every refusal before the lock and
//! the kinds run in order are `pull::gdfl_run`'s, which the api's import
//! shares (IA-10a): a refusal here and there is one code. The run logs to
//! its own directory, `logs/gdfl/<kind>` on a version-4 census and
//! `logs/gdfl/store` on version 3 ([`install_run_log`], D-4862), and refuses
//! `log_unavailable` before any lock when that sink could not be installed
//! (D-4903): a store-writing run never works unlogged. It writes the
//! progress record `imports/gdfl.<kind>.progress` (D-4904) and says a line a
//! day on stderr through [`say_on_stderr`], the one declared writer.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pull::gdfl_import::{Durability, ImportKind, ImportRefusal, Observer, Report, Workers};
use pull::gdfl_run::{
    self, By, ClaimAsk, KindEnd, KindPlan, Kinds, LogState, Plan, Process, ProgressFile, Request,
    RunRefusal, SourceCache, SourceWord,
};
use pull::session::Day;

use crate::{FAILED, MISUSED, OK};

/// The door around the importer (AF-01).
#[path = "gdfl_import_guard.rs"]
mod guard;
/// The import's run lock, which `cli upkeep`'s tests take to stand for a
/// live import (R3-17, D-6270 amendment 1: the upkeep itself takes none).
#[cfg(test)]
pub(crate) use guard::{Holder, RunLock};

pub(crate) use pull::gdfl_run::{CM_ARCHIVE_VAR, NFO_ARCHIVES_VAR, TICKSTORE_VAR};
/// The flag that reads again the days closed done with refusals (D-3317).
pub(crate) const RETRY_REFUSED: &str = "--retry-refused";
/// The flag that narrows a capital-market run to what D-3333 adds.
pub(crate) const STORED: &str = "--stored";
/// The flag that names the run's snapshot (D-4902): `--snapshot NAME`.
pub(crate) const SNAPSHOT: &str = "--snapshot";
/// The flag that carries the operator's repair reason (IA-16, D-4889):
/// `--repair-reason TEXT`, 1 to 200 bytes on one line.
pub(crate) const REPAIR_REASON: &str = "--repair-reason";
/// The flag that accepts a repair in a month still being traded (IA-16,
/// D-4876 (2), D-6900 keep item 3).
pub(crate) const ACCEPT_OPEN_MONTH: &str = "--accept-open-month";
/// How many workers read and file one day's files (WP-16): optional, 1 to
/// `pull::gdfl_import::Workers::MAX`; absent is the import's default for
/// this host's cores (`Workers::for_host`). The bytes a run leaves do not
/// depend on it.
pub(crate) const WORKERS_VAR: &str = "BRUTEX_GDFL_WORKERS";
/// When a day's writes are made durable (WP-16, D-3353): optional, `day`
/// (the measured default), `day-each-file` or `each-write`. The bytes a run
/// leaves do not depend on it.
pub(crate) const DURABILITY_VAR: &str = "BRUTEX_GDFL_DURABILITY";
/// The free-space floor in bytes the run checks before its first day and
/// before each day it reads (IA-16, CONC-031, D-6900 keep item 5): optional,
/// a count of bytes; absent is `pull::gdfl_import::DEFAULT_FREE_FLOOR`, and
/// `0` checks nothing. Read here and carried in the request (D-4901).
pub(crate) const FREE_FLOOR_VAR: &str = "BRUTEX_GDFL_FREE_FLOOR";

/// The words after `TO`: whether `--retry-refused` and `--stored` were
/// given (each a flag anywhere among them, never a symbol), the snapshot
/// `--snapshot` names, and the symbols.
///
/// # Errors
///
/// A `--snapshot` with no NAME after it or with a flag where its NAME goes
/// (`--snapshot --stored` once dropped `--stored` without a word), a second
/// `--snapshot`, and a word beginning with `-` that is not one of the
/// verb's flags (never filed as a symbol) (D-5831).
fn flags_of(words: &[&str]) -> Result<Flags, String> {
    let mut flags = Flags {
        symbols: Vec::with_capacity(words.len()),
        ..Flags::default()
    };
    let mut rest = words.iter();
    while let Some(word) = rest.next() {
        match *word {
            RETRY_REFUSED => flags.retry = true,
            STORED => flags.stored = true,
            ACCEPT_OPEN_MONTH => flags.accept_open_month = true,
            SNAPSHOT => {
                flags.snapshot = Some(value_of(
                    SNAPSHOT,
                    "NAME",
                    flags.snapshot.is_some(),
                    rest.next(),
                )?);
            }
            REPAIR_REASON => {
                flags.repair_reason = Some(value_of(
                    REPAIR_REASON,
                    "TEXT",
                    flags.repair_reason.is_some(),
                    rest.next(),
                )?);
            }
            flag if flag.starts_with('-') => {
                return Err(format!(
                    "{} is not a flag of gdfl-import; its flags are {RETRY_REFUSED}, {STORED}, \
                     {SNAPSHOT} NAME, {REPAIR_REASON} TEXT and {ACCEPT_OPEN_MONTH}",
                    shown_word(flag)
                ));
            }
            symbol => flags.symbols.push(symbol.to_owned()),
        }
    }
    Ok(flags)
}

/// The word after a flag that takes one (`--snapshot NAME`,
/// `--repair-reason TEXT`): refused when the flag is given twice, when
/// nothing follows, or when a flag stands where the value goes.
fn value_of(flag: &str, what: &str, again: bool, next: Option<&&str>) -> Result<String, String> {
    if again {
        return Err(format!("{flag} is given twice; a run has one {what}"));
    }
    match next {
        Some(value) if !value.starts_with('-') => Ok((*value).to_owned()),
        Some(other) => Err(format!(
            "{flag} takes a {what} after it; {} is a flag, not a {what}",
            shown_word(other)
        )),
        None => Err(format!("{flag} takes a {what} after it")),
    }
}

/// What the words after `TO` ask ([`flags_of`]).
#[derive(Debug, Default)]
struct Flags {
    retry: bool,
    stored: bool,
    snapshot: Option<String>,
    /// `--repair-reason TEXT` (IA-16, D-4889, critic X-1 (b)): the one run
    /// that reads the repairable hole days of its range and hands a month
    /// the store refused to the back-fill door. Judged by
    /// `pull::ingest::repair_reason_ok`, the rule both feeds use.
    repair_reason: Option<String>,
    /// `--accept-open-month` (IA-16, D-4876 (2), D-6900 keep item 3).
    accept_open_month: bool,
    symbols: Vec<String>,
}

/// A word of the command line as a refusal shows it: debug-quoted, so a
/// control character is visible, and at most 64 characters.
fn shown_word(word: &str) -> String {
    let mut shown = format!("{:?}", word.chars().take(64).collect::<String>());
    if word.chars().nth(64).is_some() {
        shown.push_str("...");
    }
    shown
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

/// The floor `BRUTEX_GDFL_FREE_FLOOR` asks for, or the import's default
/// when it is not set; a value that is not a count of bytes is refused by
/// name, never read as the default.
pub(crate) fn free_floor_of(env: &dyn Fn(&str) -> Option<OsString>) -> Result<u64, String> {
    let Some(value) = env(FREE_FLOOR_VAR) else {
        return Ok(pull::gdfl_import::DEFAULT_FREE_FLOOR);
    };
    value
        .to_str()
        .filter(|text| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|text| text.parse::<u64>().ok())
        .ok_or_else(|| {
            format!(
                "{FREE_FLOOR_VAR}={} is not a count of bytes",
                value.display()
            )
        })
}

/// The root a variable names, or the refusal naming the variable
/// (`pull::gdfl_run::source_root`'s words, one authority).
pub(crate) fn root(var: &str, env: &dyn Fn(&str) -> Option<OsString>) -> Result<PathBuf, String> {
    let known = [TICKSTORE_VAR, CM_ARCHIVE_VAR, NFO_ARCHIVES_VAR]
        .into_iter()
        .find(|known| *known == var)
        .ok_or_else(|| format!("{var} is not a GDFL source variable"))?;
    gdfl_run::source_root(known, env).map_err(|why| {
        why.to_string()
            .strip_prefix("source_unset: ")
            .map_or_else(|| why.to_string(), str::to_owned)
    })
}

/// The request the verb's words and the environment ask for (the cli's
/// whole share of a run: argument and environment parsing). The workers and
/// the durability are read from the environment HERE and travel in the
/// request; the run reads no run parameter from the environment (D-4901).
///
/// # Errors
///
/// The words a misuse shows, for a kind, a day, a source, a worker count, a
/// durability or a flag that is not one.
fn asked(
    (kind, source, from, to, rest): (&str, &str, &str, &str, &[&str]),
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Result<Request, String> {
    let kind = ImportKind::parse(kind).map_err(|why| why.to_string())?;
    let (from, to) = (day(from)?, day(to)?);
    let source = SourceWord::parse(source).ok_or_else(|| {
        format!(
            "{} is not a source; the sources are tickstore and zip",
            guard::plain(&format!("{source:?}"))
        )
    })?;
    let (workers, durability) = (workers_of(env)?, durability_of(env)?);
    let free_floor = free_floor_of(env)?;
    let flags = flags_of(rest)?;
    Ok(Request {
        only: flags.symbols,
        stored: flags.stored,
        retry_refused: flags.retry,
        workers,
        durability,
        snapshot: flags.snapshot,
        repair_reason: flags.repair_reason,
        accept_open_month: flags.accept_open_month,
        free_floor,
        ..Request::new(Kinds::one(kind), source, (from, to))
    })
}

pub(crate) use crate::store_write::{ALLOW_UNSTAMPED_VAR, unstamped_write_refusal};

/// What a run of this process says as it goes: a line a day on the declared
/// stderr writer, and the progress record (D-4904), both told by the one
/// run on its coordinating thread.
struct Told<'a> {
    say: &'a (dyn Fn(&str) + Sync),
    progress: Option<ProgressFile>,
    run: Mutex<u64>,
}

impl std::fmt::Debug for Told<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Told")
            .field("progress", &self.progress)
            .finish_non_exhaustive()
    }
}

impl Observer for Told<'_> {
    fn begun(&self, kind: ImportKind, run: u64) {
        *self
            .run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = run;
        (self.say)(&format!("gdfl-import {}: run {run} begun", kind.as_str()));
        if let Some(progress) = &self.progress {
            progress.begun(kind, run);
        }
    }

    fn day_begun(&self, kind: ImportKind, day: Day, at: (usize, usize)) {
        if let Some(progress) = &self.progress {
            progress.day_begun(kind, day, at);
        }
    }

    fn day_passed(&self, kind: ImportKind, day: Day, report: &Report) {
        (self.say)(&format!(
            "gdfl-import {} {day}: imported {} done before {} not held {} closed {} refused {}; \
             files {} failures {}",
            kind.as_str(),
            report.days_imported,
            report.days_skipped,
            report.days_missing.len(),
            report.days_closed,
            report.days_refused + report.closed_refused.len(),
            report.files,
            report.failures_total,
        ));
        if let Some(progress) = &self.progress {
            progress.day_passed(kind, day, report);
        }
    }

    fn finished(&self, kind: ImportKind, end: Result<&Report, &ImportRefusal>) {
        let run = *self
            .run
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let how = end.map_or_else(
            |refusal| format!("refused {}", refusal.code()),
            |_| "finished".to_owned(),
        );
        (self.say)(&format!("gdfl-import {}: run {run} {how}", kind.as_str()));
        if let Some(progress) = &self.progress {
            progress.finished(kind, end);
            if let Some(fault) = progress.fault() {
                (self.say)(&format!(
                    "gdfl-import {}: the progress record was not written: {fault}",
                    kind.as_str()
                ));
            }
        }
    }
}

/// The progress file a run of `kind` writes on `store`, when the store's
/// census layout can be read (else the door refuses the run by name).
fn progress_of(store: &Path, plan: &Plan, kind: &KindPlan) -> Option<ProgressFile> {
    let layout = gdfl_run::layout_of(store).ok()?;
    let byte = match layout {
        telemetry::run_dirs::GdflLayout::V3 => 3,
        telemetry::run_dirs::GdflLayout::V4 => 4,
    };
    Some(ProgressFile::new(
        gdfl_run::progress_path(store, kind.kind, layout),
        plan,
        kind,
        byte,
    ))
}

/// Where a run reads its environment: a variable's value, or `None`.
type Env<'a> = &'a dyn Fn(&str) -> Option<OsString>;
/// Where a run finds its store root, proved, or why not.
type StoreRoot<'a> = &'a dyn Fn() -> Result<PathBuf, String>;

/// The verb, with its environment, store root, event sink and stderr passed
/// in so every arm is driven by a test without touching the process.
pub(crate) fn command_in(
    args: &[&str],
    (env, store): (Env<'_>, StoreRoot<'_>),
    (log, say): (&LogState, &(dyn Fn(&str) + Sync)),
    out: &mut String,
) -> u8 {
    let [kind, source, from, to, rest @ ..] = args else {
        let _ = writeln!(
            out,
            "refused: gdfl-import takes KIND SOURCE FROM TO [SYMBOL ...]"
        );
        return MISUSED;
    };
    let request = match asked((kind, source, from, to, rest), env) {
        Ok(request) => request,
        Err(why) => {
            let _ = writeln!(out, "refused: {}", guard::plain(&why));
            return MISUSED;
        }
    };
    // A MISUSE IS REFUSED BEFORE THE LOCK AND ANY ROOT (AF-01, GD-09), by the
    // one authority's code (IA-GR-13): `--stored` with options, a range that
    // runs backwards, an empty SYMBOL, an oversized filter, a bad snapshot.
    if let Some(misuse) = request.misuse() {
        let _ = writeln!(out, "refused: {}", guard::plain(&misuse.to_string()));
        return MISUSED;
    }
    // A STORE WHOSE `bars` IS A LINK IS NOT WRITTEN (R3-16, RA-298): asked
    // once, by name, before the run reads a day (`store_write::writable`).
    let store = match store().and_then(|store| {
        crate::store_write::writable(&store)?;
        Ok(store)
    }) {
        Ok(store) => store,
        Err(why) => {
            let _ = writeln!(out, "GDFL IMPORT REFUSED: {}", guard::plain(&why));
            return FAILED;
        }
    };
    // EVERY REFUSAL BEFORE THE LOCK, IN THE AUTHORITY'S ORDER (IA-1): the
    // sink (D-4903), the build's stamp (DB-B71, D-4894), the source root and
    // the scope (D-3333).
    let unstamped =
        unstamped_write_refusal(crate::commit_stamp(), env(ALLOW_UNSTAMPED_VAR).as_deref());
    let process = Process {
        store_root: &store,
        commit: crate::commit_stamp(),
        log,
        unstamped: unstamped.as_deref(),
    };
    let plan = match gdfl_run::plan(request, &process, env, &mut SourceCache::default()) {
        Ok(plan) => plan,
        Err(refused) => {
            let code = if refused.misuse() { MISUSED } else { FAILED };
            // A HELD LOCK FOUND BEFORE THE SINK (IA-2 repair 1; FM-064) is
            // said in the door's own words: the holder's kind, pid, start,
            // run and by, exactly as a refusal at the lock says them.
            let said = match &refused {
                RunRefusal::RunBusy { .. } => guard::refused_finding(&store, &refused).to_string(),
                other => other.to_string(),
            };
            let _ = writeln!(out, "GDFL IMPORT REFUSED: {}", guard::plain(&said));
            return code;
        }
    };
    let Some(kind) = plan.kinds.first() else {
        let _ = writeln!(out, "GDFL IMPORT REFUSED: the plan holds no kind");
        return FAILED;
    };
    // THE DOOR (AF-01, D-4977; FM-060, FM-064, FM-086, FM-290): refused
    // here, before the importer writes a journal line, or held until the
    // run returns. Its lock is the authority's claim (IA-2, D-4804): the
    // locks by census layout, the id from the runs ledger, a claim that
    // says `by=cli auto_resume=0` (D-4804), and the request sidecar when
    // the run names a filter or a snapshot (D-4907).
    let claim = gdfl_run::claim_template(
        &plan,
        kind,
        &ClaimAsk {
            by: By::Cli,
            pid: std::process::id(),
            host: "",
            resumes: 0,
            resume_day: None,
            auto_resume: false,
            legacy_ack: false,
            unstamped: crate::commit_stamp().is_none(),
            retry: pull::gdfl_run::locks::PROBE_RETRY,
        },
    );
    let sidecar = (!plan.request.only.is_empty() || plan.request.snapshot.is_some())
        .then(|| plan.request.to_line().ok())
        .flatten();
    let door = match guard::Door::open(guard::Opening {
        kind: kind.kind,
        var: kind.source.var(),
        store: &store,
        origin: &kind.origin,
        only: &kind.only,
        args,
        claim: Some(&claim),
        sidecar: sidecar.as_deref(),
    }) {
        Ok(door) => door,
        Err(why) => {
            let _ = writeln!(
                out,
                "GDFL IMPORT REFUSED: {}",
                guard::plain(&why.to_string())
            );
            return FAILED;
        }
    };
    let told = Told {
        say,
        progress: progress_of(&store, &plan, kind),
        run: Mutex::new(0),
    };
    // Run under the claim: its id is the runs ledger's, and `cli gdfl-stop`
    // stops it at the next file boundary (IA-2, D-4827).
    let end = match door.claimed() {
        Some(claimed) => gdfl_run::execute_claimed(&plan, kind, Some(&told), claimed),
        None => gdfl_run::execute_kind(&plan, kind, Some(&told)),
    };
    receipt((kind, source, &store, &plan.request), end, door, out)
}

/// The run's receipt and the door's close, by how the run ended.
fn receipt(
    (kind, source, store, request): (&KindPlan, &str, &Path, &Request),
    end: KindEnd,
    door: guard::Door,
    out: &mut String,
) -> u8 {
    let how = (request.workers, request.durability);
    match end {
        KindEnd::Report(report) => {
            let report: Report = *report;
            render(out, kind.kind, source, store, &report);
            render_how(out, how);
            if let Some(words) = &kind.scope_words {
                let _ = writeln!(out, "  scope               {}", guard::plain(words));
            }
            let failed = report.failures_total > 0;
            let door_failed = door.close(Some(&report), failed.then_some("failed"), out);
            if failed || door_failed { FAILED } else { OK }
        }
        KindEnd::Refused(refusal) => {
            // A census refusal stopped the run part way (D-3317 item 7): what
            // it did before the refused day is on disk, so it is printed too.
            if let Some(done) = refusal.report() {
                render(out, kind.kind, source, store, done);
                render_how(out, how);
            }
            let _ = writeln!(
                out,
                "GDFL IMPORT REFUSED: {}",
                guard::plain(&refusal.to_string())
            );
            let _ = door.close(refusal.report(), Some("refused"), out);
            FAILED
        }
        KindEnd::Unopened(refused) => {
            let _ = writeln!(
                out,
                "GDFL IMPORT REFUSED: {}",
                guard::plain(&refused_words(&refused))
            );
            let _ = door.close(None, Some("refused"), out);
            FAILED
        }
    }
}

/// An unopened source's words, as the receipt said them before IA-1: the
/// path and why, without a code in front.
fn refused_words(refused: &RunRefusal) -> String {
    match refused {
        RunRefusal::SourceUnopened { path, why } => format!("{}: {why}", path.display()),
        other => other.to_string(),
    }
}

/// THE CLI'S ONE DECLARED STDERR WRITER (round 5, SF-7; CI gate 23 clause
/// A2): a run's line a day, and nothing else of this verb, reaches stderr
/// here. A failed write is ignored by design and never panics: Rust ignores
/// `SIGPIPE`, so a print to a closed pipe would panic and a release build
/// abort (`eprintln!` is never used). Each line is also in the run's events
/// and its progress record, so a stderr that goes nowhere loses nothing.
pub(crate) fn say_on_stderr(line: &str) {
    use std::io::Write as _;
    let mut err = std::io::stderr().lock();
    let _closed = err
        .write_all(line.as_bytes())
        .and_then(|()| err.write_all(b"\n"))
        .and_then(|()| err.flush());
}

/// WHERE A RUN PROCESS LOGS (D-4862, D-4903; IA-1): for `gdfl-import KIND`,
/// installs this process's one event sink in the run's own directory,
/// `<store>/logs/gdfl/<kind>` on a version-4 census and `logs/gdfl/store` on
/// version 3, which the kind's lock makes it alone in, and answers the line
/// `main` prints. `None` for every other verb, for a word that is not a
/// kind (the verb refuses it as a misuse, logged where every verb logs), and
/// for a store whose census layout cannot be read (logged to `logs/cli`; the
/// run's door then refuses it by name, never a guessed layout): those keep
/// [`crate::install_log`].
///
/// A `BRUTEX_LOG_DIR` naming any other directory is not followed: the run
/// would log where no reader of its evidence looks (`/imports.json`, the day
/// band and `/logs` read the run directories, IA-11r), so the sink is not
/// installed, the answer says why, and the run then refuses
/// `log_unavailable` before any lock (D-4903).
#[must_use]
pub fn install_run_log(args: &[OsString]) -> Option<String> {
    let [verb, kind, ..] = args else {
        return None;
    };
    if verb.to_str() != Some("gdfl-import") {
        return None;
    }
    let kind = ImportKind::parse(kind.to_str()?).ok()?;
    let store = crate::store_root().ok()?;
    let dir = gdfl_run::run_dir(&store, kind).ok()?.beneath(&store);
    Some(run_log_answer(
        std::env::var_os("BRUTEX_LOG_DIR"),
        &dir,
        &|dir| telemetry::install(&telemetry::Config::new(dir)).map(|_| ()),
    ))
}

/// [`install_run_log`]'s answer for a run whose directory is `dir`, with the
/// sink installed by `install`.
fn run_log_answer(
    explicit: Option<OsString>,
    dir: &Path,
    install: &dyn Fn(&Path) -> Result<(), String>,
) -> String {
    if let Some(named) = explicit
        && resolved(Path::new(&named)) != resolved(dir)
    {
        return format!(
            "{}: BRUTEX_LOG_DIR names \"{}\", but a gdfl-import run logs only to {}, the \
             directory its kind's lock makes it alone in and every reader of its evidence \
             reads (D-4862). Unset BRUTEX_LOG_DIR.",
            crate::UNLOGGED,
            named.display(),
            dir.display()
        );
    }
    match install(dir) {
        Ok(()) => format!(
            "events -> {}\n  This run logs to its own directory (D-4862); /logs, /imports.json \
             and the day band read it.",
            dir.display()
        ),
        Err(why) => format!("{}: {why}", crate::UNLOGGED),
    }
}

/// `path` with its longest existing ancestor made canonical and the rest
/// appended as written, so two spellings of one directory compare equal
/// even before it exists: the store root is canonical
/// (`cli::existing_store_root`), and `/var` is `/private/var` on macOS, as
/// is a volume reached through a link. A path with no existing ancestor, or
/// a `..` past the last existing one, stays as written, so it can only
/// compare unequal and be refused by name, never be taken for the run's
/// directory. One `canonicalize` per component at most.
fn resolved(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut at = path;
    loop {
        if let Ok(real) = std::fs::canonicalize(at) {
            return rest.iter().rev().fold(real, |whole, part| whole.join(part));
        }
        match (at.parent(), at.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_owned());
                at = parent;
            }
            _ => return path.to_path_buf(),
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

/// How many hole days a receipt names per reason before it says how many
/// more there are.
const HOLE_DAYS_NAMED: usize = 5;

/// A receipt's hole days by reason (IA-16, D-6900 keep item 2): `0`, or the
/// count, then each reason in first-seen order with its count, whether a
/// repair-reason run reads it, and its first [`HOLE_DAYS_NAMED`] days.
/// O(holes in the run's range).
fn holes(days: &[(Day, String)]) -> String {
    if days.is_empty() {
        return "0".to_owned();
    }
    let mut reasons: Vec<(&str, Vec<Day>)> = Vec::new();
    for (day, reason) in days {
        match reasons.iter_mut().find(|(one, _)| *one == reason.as_str()) {
            Some((_, list)) => list.push(*day),
            None => reasons.push((reason.as_str(), vec![*day])),
        }
    }
    let mut text = days.len().to_string();
    for (reason, list) in &reasons {
        let named: Vec<String> = list
            .iter()
            .take(HOLE_DAYS_NAMED)
            .map(ToString::to_string)
            .collect();
        let more = list.len().saturating_sub(HOLE_DAYS_NAMED);
        let _ = write!(
            text,
            "; {reason} {} ({}): {}{}",
            list.len(),
            if pull::gdfl_import::hole_is_repairable(reason) {
                "a run with a repair reason reads it"
            } else {
                "no repair fills it"
            },
            named.join(" "),
            if more > 0 {
                format!(" and {more} more")
            } else {
                String::new()
            }
        );
    }
    text
}

/// The store's standing holes by kind and reason (IA-16, D-6900 keep item
/// 2), from the journal the run loaded: `0`, or the count, then each
/// `kind reason=n`.
fn standing(holes: &[(String, String, usize)]) -> String {
    if holes.is_empty() {
        return "0 in the whole store".to_owned();
    }
    let total: usize = holes.iter().map(|(_, _, n)| n).sum();
    let each: Vec<String> = holes
        .iter()
        .map(|(kind, reason, n)| format!("{kind} {reason}={n}"))
        .collect();
    format!("{total} in the whole store: {}", each.join(", "))
}

/// THE HOLES, ON THE ONE PER-RUN SURFACE THERE IS (IA-16, D-6900 keep item
/// 2): a hole `admit` passed over is never a day already done.
fn render_holes(out: &mut String, report: &Report) {
    let _ = writeln!(
        out,
        "  holes passed        {}",
        holes(&report.holes_in_range)
    );
    let _ = writeln!(out, "  holes made          {}", holes(&report.holes_made));
    let _ = writeln!(
        out,
        "  standing holes      {}",
        standing(&report.standing_holes)
    );
}

/// The run's receipt.
fn render(out: &mut String, kind: ImportKind, source: &str, store: &Path, report: &Report) {
    let _ = writeln!(
        out,
        "GDFL IMPORT -- {} from {source} into {}",
        kind.as_str(),
        guard::shown(store)
    );
    let _ = writeln!(
        out,
        "  rung                1s only (D-2802: no coarser rung is derived from gdfl)"
    );
    let _ = writeln!(out, "  days imported       {}", report.days_imported);
    let _ = writeln!(out, "  days already done   {}", report.days_skipped);
    render_holes(out, report);
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
    // EVERY FAILURE COUNTED, THE NEWEST 1,024 LISTED (D-4839): the count is
    // the run's; what is not listed is said, with each stage and code's count.
    let _ = writeln!(out, "  failures            {}", report.failures_total);
    if report.failures_unlisted() > 0 {
        let _ = writeln!(
            out,
            "    ({} more not listed; the newest {} are; each was logged as it \
             happened, and the journal keeps each day's cause codes, not each failure)",
            report.failures_unlisted(),
            report.failures.len()
        );
        for ((stage, code), count) in &report.failures_by_cause {
            let _ = writeln!(out, "    {stage}/{code}: {count}");
        }
    }
    for failure in &report.failures {
        let _ = writeln!(
            out,
            "    {}: {}",
            guard::plain(&failure.instrument),
            guard::plain(&failure.why)
        );
    }
}

/// The verb as dispatched: the process environment, the CLI's store root,
/// this process's event sink and the declared stderr writer.
pub(crate) fn command(args: &[&str], out: &mut String) -> u8 {
    let log = LogState::of_process(
        "no event sink is installed in this process; the line above the report says why",
    );
    command_in(
        args,
        (&process_env, &cli_store),
        (&log, &say_on_stderr),
        out,
    )
}

/// [`command_in`] with a sink that is installed and a stderr that hears
/// nothing: the tests' door to every arm after the log.
#[cfg(test)]
pub(crate) fn command_with(
    args: &[&str],
    env: &dyn Fn(&str) -> Option<OsString>,
    store: &dyn Fn() -> Result<PathBuf, String>,
    out: &mut String,
) -> u8 {
    let log = LogState::Installed(PathBuf::from("logs/test"));
    command_in(args, (env, store), (&log, &|_| {}), out)
}

#[cfg(test)]
#[path = "gdfl_import_command_tests.rs"]
mod tests;

/// THE CLOUD MUTATION PASS of 2026-10-16 (`docs/06-limits.md`, "The cli
/// import verbs"): each test pins the behaviour a mutant of this file broke
/// while every other test stayed green, or runs a line no test reached. No
/// vendor byte: the one archive here is built at run time from invented
/// values.
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
    /// through the day and a row after the close, at an invented level
    /// (`4242.xx`) chosen only to be plainly synthetic. Nothing here claims
    /// what NIFTY printed on any day.
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
        // And the verb prints exactly what it was asked for, right after the
        // receipt's counts. The tick store is one the door accepts (AF-01: a
        // `FORMAT.md` at its root) holding no day, so the run owes the day,
        // reads no file and ends `FAILED` `no_file_read` (the guard's
        // `verdict`, as `tests::a_run_that_read_no_file_never_exits_ok`): the
        // receipt and how it filed still come first, the failure last. (The
        // cloud pass ran this over a tick store that did not exist and
        // expected `OK`; the door added since refuses that root by name.)
        let root = scratch("how");
        let store = root.join("store");
        std::fs::create_dir_all(&store).unwrap();
        let ts = root.join("ts");
        super::tests::tick_store(&ts);
        let env = env_of(&[
            (TICKSTORE_VAR, ts.into_os_string()),
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
        assert_eq!(code, FAILED, "{out}");
        let how = out
            .find(&format!(
                "  failures            0\n  workers             2 ({WORKERS_VAR}, 1 to {})\n  \
                 durability          each-write ({DURABILITY_VAR}; D-3353)\n",
                Workers::MAX
            ))
            .unwrap_or_else(|| panic!("the receipt's counts, then how it filed: {out}"));
        let failed = out
            .find("GDFL IMPORT FAILED: no_file_read: the run owed 1 day")
            .unwrap_or_else(|| panic!("the run that read no file is named: {out}"));
        assert!(how < failed, "how the run filed, before its failure: {out}");
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

    /// **A SOURCE ROOT IS READ FROM THE VARIABLE ASKED FOR, AND ONLY FROM A
    /// GDFL SOURCE VARIABLE** (`cli gdfl-dayflags` reads its roots here):
    /// each of the three is read as it is set, never another's value; one
    /// that is unset is refused in the run authority's words without its
    /// code; any other variable is refused by name, never read.
    #[test]
    fn a_source_root_is_read_from_the_variable_asked_for_and_no_other() {
        let env = env_of(&[
            (TICKSTORE_VAR, OsString::from("/synthetic/ts")),
            (NFO_ARCHIVES_VAR, OsString::from("/synthetic/years")),
            ("BRUTEX_STORE", OsString::from("/synthetic/store")),
        ]);
        assert_eq!(
            root(TICKSTORE_VAR, &env),
            Ok(PathBuf::from("/synthetic/ts"))
        );
        assert_eq!(
            root(NFO_ARCHIVES_VAR, &env),
            Ok(PathBuf::from("/synthetic/years"))
        );
        assert_eq!(
            root(CM_ARCHIVE_VAR, &env),
            Err(format!(
                "{CM_ARCHIVE_VAR} is not set; it names where the GDFL data is read from and has \
                 no default"
            ))
        );
        assert_eq!(
            root("BRUTEX_STORE", &env),
            Err("BRUTEX_STORE is not a GDFL source variable".to_owned())
        );
    }

    /// A run's observer shows its progress record and nothing it cannot
    /// print: the writer it says lines through is left out by name.
    #[test]
    fn the_runs_observer_debugs_as_its_progress_record() {
        let told = Told {
            say: &|_| {},
            progress: None,
            run: Mutex::new(7),
        };
        assert_eq!(format!("{told:?}"), "Told { progress: None, .. }");
    }

    /// **A REFUSED WORD IS SHOWN DEBUG-QUOTED AND CUT AT 64 CHARACTERS**
    /// (D-5831): the word that is not a flag, and the flag where
    /// `--snapshot`'s NAME belongs, are each named as the refusal shows them,
    /// so a control character is visible and a long word cannot flood the
    /// receipt. The tests of `flags_of` named only the sentence; this pins
    /// the word in it (a survivor of this merge's own mutation run, 22 Oct
    /// 2026).
    #[test]
    fn a_refused_word_is_shown_quoted_and_cut_at_64_characters() {
        assert_eq!(shown_word("--stord"), "\"--stord\"");
        assert_eq!(shown_word("-\u{7}x"), "\"-\\u{7}x\"");
        let exactly = format!("-{}", "y".repeat(63));
        assert_eq!(shown_word(&exactly), format!("{exactly:?}"));
        let longer = format!("{exactly}z");
        assert_eq!(shown_word(&longer), format!("{exactly:?}..."));
        // The word as the refusal shows it, first; the list of flags after it
        // is the flag tests' to pin (it grew with `--repair-reason` and
        // `--accept-open-month` on `feat/db-fix`).
        let refused = flags_of(&["NIFTY", "--stord"]).expect_err("a word that is not a flag");
        assert!(
            refused.starts_with("\"--stord\" is not a flag of gdfl-import; its flags are "),
            "{refused}"
        );
        assert!(refused.contains(RETRY_REFUSED), "{refused}");
        // Where `--snapshot`'s NAME belongs: the word, cut, then the
        // sentence's end (whose wording is the flag tests' to pin).
        let refused = flags_of(&[SNAPSHOT, longer.as_str()]).expect_err("a flag is not a name");
        assert!(
            refused.starts_with(&format!(
                "{SNAPSHOT} takes a NAME after it; {exactly:?}... is a flag, not a "
            )),
            "{refused}"
        );
    }

    /// An options tree that cannot be listed is named, never read as one
    /// that holds no underlying; and an underlying whose name is not text
    /// is refused by name.
    ///
    /// **Where the host can hold such a name.** APFS and HFS+ refuse a
    /// directory name that is not UTF-8 (`EILSEQ`, "Illegal byte sequence"),
    /// so on macOS no listing can return one and the refusal arm is
    /// unreachable through the filesystem. There the test proves the host
    /// refused the name rather than skipping silently; on Linux it proves
    /// the arm.
    ///
    /// **Where the code now lives.** The scope moved into the one run
    /// authority (IA-1, D-4800): the listing is
    /// `pull::gdfl_run::scope::option_underlyings`, which this verb's run
    /// reads, so this test pins the verb's scope from here exactly as
    /// `the_stored_flag_narrows_a_capital_market_run_to_what_d3333_adds`
    /// does; the guard's mutant is now in `crates/pull/src/gdfl_run_scope.rs`
    /// (`docs/06-limits.md`, "The cli import verbs").
    #[test]
    fn an_options_tree_that_cannot_be_listed_or_named_is_refused() {
        use pull::gdfl_run::scope::{
            option_underlyings as stored_option_underlyings, options_tree,
        };
        let root = scratch("tree");
        let tree = options_tree(&root);
        std::fs::create_dir_all(tree.parent().unwrap()).unwrap();
        std::fs::write(&tree, b"a file where the tree belongs").unwrap();
        let why = stored_option_underlyings(&root).expect_err("a file is not a tree");
        assert!(why.starts_with(&tree.display().to_string()), "{why}");
        std::fs::remove_file(&tree).unwrap();

        let odd = std::ffi::OsStr::from_bytes(b"NIFTY\xff");
        match std::fs::create_dir_all(tree.join(odd)) {
            Ok(()) => {
                let why = stored_option_underlyings(&root).expect_err("a name that is not text");
                assert!(why.contains("is not a symbol"), "{why}");
            }
            Err(host) => {
                assert_eq!(
                    std::env::consts::OS,
                    "macos",
                    "only a macOS volume may refuse a non-UTF-8 name here: {host}"
                );
                assert!(
                    host.to_string().contains("Illegal byte sequence"),
                    "the host refused the name for its encoding: {host}"
                );
                assert_eq!(
                    stored_option_underlyings(&root).unwrap(),
                    Vec::<String>::new(),
                    "a refused name leaves the tree empty"
                );
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    /// **AN UNOPENED SOURCE IS SAID BY ITS PATH AND WHY, WITHOUT ITS CODE**
    /// (the receipt's words before IA-1); every other refusal keeps the
    /// authority's own words, code first. (Survivors of this merge's mutation
    /// run of 22 Oct 2026: `refused_words` to an empty or an invented
    /// string.)
    ///
    /// Pinned on the function, not through the verb: the plan opens the
    /// capital-market archive before the door (`source_root_unknown`, as the
    /// last case here shows), so the receipt's `Unopened` arm is reached only
    /// by an archive that changes between the plan's open and the run's
    /// (`docs/06-limits.md`, "The cli import verbs").
    #[test]
    fn an_unopened_source_is_said_by_its_path_and_why_without_its_code() {
        let unopened = RunRefusal::SourceUnopened {
            path: PathBuf::from("/synthetic/cm.zip"),
            why: "not a zip archive".to_owned(),
        };
        assert_eq!(
            refused_words(&unopened),
            "/synthetic/cm.zip: not a zip archive"
        );
        assert_eq!(
            unopened.to_string(),
            "source_unopened: /synthetic/cm.zip: not a zip archive",
            "the code the receipt leaves out"
        );
        let unset = RunRefusal::SourceUnset {
            var: CM_ARCHIVE_VAR,
        };
        assert_eq!(refused_words(&unset), unset.to_string());
        assert!(
            refused_words(&unset).starts_with("source_unset: "),
            "{unset}"
        );

        // Through the verb, an archive of invented bytes is refused by the
        // plan, before the door and any receipt, in the authority's words.
        let root = scratch("unopened");
        let store = root.join("store");
        std::fs::create_dir_all(&store).unwrap();
        let cm = root.join("cm.zip");
        std::fs::write(&cm, b"invented bytes, not a zip archive").unwrap();
        let env = env_of(&[(CM_ARCHIVE_VAR, cm.into_os_string())]);
        let mut out = String::new();
        let code = command_with(
            &["indices", "zip", "2024-04-01", "2024-04-01"],
            &env,
            &|| Ok(store.clone()),
            &mut out,
        );
        assert_eq!(code, FAILED, "{out}");
        assert!(
            out.starts_with("GDFL IMPORT REFUSED: source_root_unknown: "),
            "refused before the door, with no receipt: {out}"
        );
        assert!(!store.join("imports").exists(), "no lock was taken: {out}");
        std::fs::remove_dir_all(root).unwrap();
    }

    /// **A RUN UNDER A FILTER OR A SNAPSHOT FILES ITS REQUEST SIDECAR; A RUN
    /// UNDER NEITHER FILES NONE** (D-4907): the sidecar is beside the run's
    /// lock while the run is begun, and gone when it is released. Read from
    /// the run's own `begun` line, the moment its claim is written.
    /// (Survivors of this merge's mutation run of 22 Oct 2026: the sidecar's
    /// condition with its `!` deleted, and with `||` read as `&&`.)
    #[test]
    fn a_request_sidecar_is_filed_exactly_when_the_run_names_a_filter_or_a_snapshot() {
        let root = scratch("sidecar");
        let ts = root.join("ts");
        super::tests::tick_store(&ts);
        let env = env_of(&[(TICKSTORE_VAR, ts.into_os_string())]);
        let sidecars = |imports: &Path| -> usize {
            std::fs::read_dir(imports).map_or(0, |entries| {
                entries
                    .filter(|entry| {
                        entry.as_ref().is_ok_and(|entry| {
                            entry.file_name().to_string_lossy().ends_with(".request")
                        })
                    })
                    .count()
            })
        };
        let cases: [(&[&str], usize); 4] = [
            (&[], 0),
            (&["NIFTY"], 1),
            (&[SNAPSHOT, "pre-1"], 1),
            (&["NIFTY", SNAPSHOT, "pre-1"], 1),
        ];
        for (n, (tail, filed)) in cases.into_iter().enumerate() {
            let store = root.join(format!("store-{n}"));
            std::fs::create_dir_all(&store).unwrap();
            let imports = store.join("imports");
            let seen = Mutex::new(Vec::<usize>::new());
            let say = |line: &str| {
                if line.ends_with(" begun") {
                    seen.lock().unwrap().push(sidecars(&imports));
                }
            };
            let mut args = vec!["indices", "tickstore", "2024-04-01", "2024-04-01"];
            args.extend_from_slice(tail);
            let log = LogState::Installed(store.join("logs").join("gdfl").join("store"));
            let mut out = String::new();
            let _ = command_in(&args, (&env, &|| Ok(store.clone())), (&log, &say), &mut out);
            assert_eq!(
                *seen.lock().unwrap(),
                vec![filed],
                "{args:?}: one begun line, and the sidecars beside the lock then: {out}"
            );
            assert_eq!(sidecars(&imports), 0, "released with the run: {out}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Runs the test named `test` again in a child process with `env` set
    /// and every inherited `BRUTEX_` variable removed, and answers its
    /// stderr. The child must prove it ran: `--exact` on a name that matches
    /// nothing exits zero, so its own `1 passed` line is required.
    fn child_of(test: &str, env: &[(&str, &std::ffi::OsStr)]) -> String {
        let test = test.split_once("::").map_or(test, |(_, path)| path);
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child.args(["--exact", test, "--test-threads=1"]);
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("BRUTEX_") {
                child.env_remove(name);
            }
        }
        for (name, value) in env {
            child.env(name, value);
        }
        let output = child.output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(output.status.success(), "{stdout}{stderr}");
        assert!(
            stdout.contains("test result: ok. 1 passed;"),
            "the child ran exactly this one test: {stdout}"
        );
        stderr
    }

    /// **THE ONE STDERR WRITER SAYS ITS LINE AND A NEWLINE** (round 5, SF-7):
    /// in a child process, whose stderr the parent reads, the line arrives
    /// whole and ended. (A survivor of this merge's mutation run of 22 Oct
    /// 2026: `say_on_stderr` replaced by `()`, which no in-process test can
    /// see.)
    #[test]
    fn the_one_stderr_writer_says_its_line_and_a_newline() {
        const CHILD: &str = "BRUTEX_GDFL_CLOUD_STDERR_CHILD";
        const LINE: &str = "gdfl-import synthetic: a line said on stderr";
        if std::env::var_os(CHILD).is_some() {
            say_on_stderr(LINE);
            return;
        }
        let stderr = child_of(
            concat!(
                module_path!(),
                "::the_one_stderr_writer_says_its_line_and_a_newline"
            ),
            &[(CHILD, std::ffi::OsStr::new("1"))],
        );
        assert!(stderr.contains(&format!("{LINE}\n")), "{stderr:?}");
    }

    /// **ONLY A `gdfl-import KIND` RUN IS ANSWERED WHERE IT LOGS** (D-4862,
    /// D-4903): another verb, a missing kind and a word that is not a kind
    /// are `None`; a run whose `BRUTEX_LOG_DIR` names another directory is
    /// told its events are not recorded, naming both directories, and no
    /// sink is installed there. In a child process, because the answer reads
    /// the process's own `BRUTEX_STORE` and `BRUTEX_LOG_DIR`. (Survivors of
    /// this merge's mutation run of 22 Oct 2026: `install_run_log` to `None`,
    /// and its verb check read as `==`.)
    #[test]
    fn only_a_gdfl_import_run_is_answered_where_it_logs() {
        const CHILD: &str = "BRUTEX_GDFL_CLOUD_RUN_LOG_CHILD";
        if let Some(elsewhere) = std::env::var_os(CHILD) {
            let answer = |args: &[&str]| {
                install_run_log(&args.iter().map(OsString::from).collect::<Vec<_>>())
            };
            assert_eq!(answer(&["srt-build", "options"]), None);
            assert_eq!(answer(&["gdfl-import"]), None);
            assert_eq!(answer(&["gdfl-import", "not-a-kind"]), None);
            let told = answer(&["gdfl-import", "options"]).expect("a gdfl-import run is answered");
            assert!(told.starts_with(crate::UNLOGGED), "{told}");
            assert!(
                told.contains(&format!(
                    "BRUTEX_LOG_DIR names \"{}\"",
                    Path::new(&elsewhere).display()
                )),
                "{told}"
            );
            assert!(
                told.contains("/logs/gdfl/"),
                "the run's own directory: {told}"
            );
            return;
        }
        let root = scratch("runlog");
        let store = root.join("store");
        std::fs::create_dir_all(&store).unwrap();
        let elsewhere = root.join("elsewhere");
        let _ = child_of(
            concat!(
                module_path!(),
                "::only_a_gdfl_import_run_is_answered_where_it_logs"
            ),
            &[
                (CHILD, elsewhere.as_os_str()),
                ("BRUTEX_STORE", store.as_os_str()),
                ("BRUTEX_LOG_DIR", elsewhere.as_os_str()),
            ],
        );
        assert!(!elsewhere.exists(), "nothing is made where it would log");
        assert!(
            !store.join("logs").exists(),
            "no sink is installed for a run told it is unlogged"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// **THE REQUEST CARRIES EVERY WORD AND VARIABLE IT WAS ASKED** (D-4901):
    /// the filter, each flag, the snapshot and the repair reason from the
    /// words after `TO`; the workers, the durability and the free-space
    /// floor from the environment (IA-16, D-6900 keep item 5); with none of
    /// them, the request's own defaults. (A survivor of this merge's
    /// mutation run of 23 Oct 2026: the free-space floor dropped from the
    /// request, so a run checked the default floor whatever it was asked.)
    #[test]
    fn the_request_carries_every_word_and_variable_it_was_asked() {
        let env = env_of(&[
            (WORKERS_VAR, OsString::from("2")),
            (DURABILITY_VAR, OsString::from("each-write")),
            (FREE_FLOOR_VAR, OsString::from("4242")),
        ]);
        let words = [
            RETRY_REFUSED,
            STORED,
            ACCEPT_OPEN_MONTH,
            SNAPSHOT,
            "pre-1",
            REPAIR_REASON,
            "synthetic-reason",
            "NIFTY",
            "BANKNIFTY",
        ];
        let request = asked(
            (
                "indices",
                "tickstore",
                "2024-04-01",
                "2024-04-02",
                &words[..],
            ),
            &env,
        )
        .unwrap();
        assert_eq!(request.only, ["NIFTY", "BANKNIFTY"]);
        assert!(request.stored, "{request:?}");
        assert!(request.retry_refused, "{request:?}");
        assert!(request.accept_open_month, "{request:?}");
        assert_eq!(request.snapshot.as_deref(), Some("pre-1"));
        assert_eq!(request.repair_reason.as_deref(), Some("synthetic-reason"));
        assert_eq!(request.workers, Workers::new(2).unwrap());
        assert_eq!(request.durability, Durability::parse("each-write").unwrap());
        assert_eq!(request.free_floor, 4242);
        assert_eq!(
            (request.from, request.to),
            (Day::new(2024, 4, 1).unwrap(), Day::new(2024, 4, 2).unwrap())
        );

        let bare = asked(
            ("indices", "tickstore", "2024-04-01", "2024-04-01", &[]),
            &|_: &str| None,
        )
        .unwrap();
        assert_eq!(bare.free_floor, pull::gdfl_import::DEFAULT_FREE_FLOOR);
        assert_ne!(
            bare.free_floor, request.free_floor,
            "the asked floor is not the default"
        );
        assert!(bare.only.is_empty(), "{bare:?}");
        assert!(
            !bare.stored && !bare.retry_refused && !bare.accept_open_month,
            "{bare:?}"
        );
        assert_eq!((bare.snapshot, bare.repair_reason), (None, None));
    }

    /// **THE FILTER IS RESERVED ONCE, FOR EVERY WORD AFTER `TO`**: the list
    /// of symbols is sized before the words are read, so a filter of up to
    /// `ONLY_MAX` names is filed without regrowing, whatever share of the
    /// words are flags. (A survivor of this merge's mutation run of 23 Oct
    /// 2026: the reservation deleted, which changes no answer and only the
    /// allocation.)
    #[test]
    fn the_filter_is_reserved_once_for_every_word_after_to() {
        let words = [
            RETRY_REFUSED,
            STORED,
            ACCEPT_OPEN_MONTH,
            SNAPSHOT,
            "pre-1",
            "NIFTY",
        ];
        let flags = flags_of(&words).unwrap();
        assert_eq!(flags.symbols, ["NIFTY"]);
        assert!(
            flags.symbols.capacity() >= words.len(),
            "reserved {} for {} words",
            flags.symbols.capacity(),
            words.len()
        );
    }
}
