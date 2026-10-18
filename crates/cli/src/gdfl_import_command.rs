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
    self, KindEnd, KindPlan, Kinds, LogState, Plan, Process, ProgressFile, Request, RunRefusal,
    SourceCache, SourceWord,
};
use pull::session::Day;

use crate::{FAILED, MISUSED, OK};

/// The door around the importer (AF-01).
#[path = "gdfl_import_guard.rs"]
mod guard;

pub(crate) use pull::gdfl_run::{CM_ARCHIVE_VAR, NFO_ARCHIVES_VAR, TICKSTORE_VAR};
/// The flag that reads again the days closed done with refusals (D-3317).
pub(crate) const RETRY_REFUSED: &str = "--retry-refused";
/// The flag that narrows a capital-market run to what D-3333 adds.
pub(crate) const STORED: &str = "--stored";
/// The flag that names the run's snapshot (D-4902): `--snapshot NAME`.
pub(crate) const SNAPSHOT: &str = "--snapshot";
/// How many workers read and file one day's files (WP-16): optional, 1 to
/// `pull::gdfl_import::Workers::MAX`; absent is the import's default for
/// this host's cores (`Workers::for_host`). The bytes a run leaves do not
/// depend on it.
pub(crate) const WORKERS_VAR: &str = "BRUTEX_GDFL_WORKERS";
/// When a day's writes are made durable (WP-16, D-3353): optional, `day`
/// (the measured default), `day-each-file` or `each-write`. The bytes a run
/// leaves do not depend on it.
pub(crate) const DURABILITY_VAR: &str = "BRUTEX_GDFL_DURABILITY";

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
fn flags_of(words: &[&str]) -> Result<(bool, bool, Option<String>, Vec<String>), String> {
    let mut symbols = Vec::with_capacity(words.len());
    let (mut retry, mut stored, mut snapshot) = (false, false, None);
    let mut rest = words.iter();
    while let Some(word) = rest.next() {
        match *word {
            RETRY_REFUSED => retry = true,
            STORED => stored = true,
            SNAPSHOT => {
                if snapshot.is_some() {
                    return Err(format!(
                        "{SNAPSHOT} is given twice; a run has one snapshot name"
                    ));
                }
                match rest.next() {
                    Some(name) if !name.starts_with('-') => snapshot = Some((*name).to_owned()),
                    Some(flag) => {
                        return Err(format!(
                            "{SNAPSHOT} takes a NAME after it; {} is a flag, not a name",
                            shown_word(flag)
                        ));
                    }
                    None => return Err(format!("{SNAPSHOT} takes a NAME after it")),
                }
            }
            flag if flag.starts_with('-') => {
                return Err(format!(
                    "{} is not a flag of gdfl-import; its flags are {RETRY_REFUSED}, {STORED} \
                     and {SNAPSHOT} NAME",
                    shown_word(flag)
                ));
            }
            symbol => symbols.push(symbol.to_owned()),
        }
    }
    Ok((retry, stored, snapshot, symbols))
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
    let (retry_refused, stored, snapshot, only) = flags_of(rest)?;
    Ok(Request {
        only,
        stored,
        retry_refused,
        workers,
        durability,
        snapshot,
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
            let _ = writeln!(
                out,
                "GDFL IMPORT REFUSED: {}",
                guard::plain(&refused.to_string())
            );
            return code;
        }
    };
    let Some(kind) = plan.kinds.first() else {
        let _ = writeln!(out, "GDFL IMPORT REFUSED: the plan holds no kind");
        return FAILED;
    };
    // THE DOOR (AF-01, D-4977; FM-060, FM-064, FM-086, FM-290): refused
    // here, before the importer writes a journal line, or held until the
    // run returns.
    let door = match guard::Door::open(guard::Opening {
        kind: kind.kind,
        var: kind.source.var(),
        store: &store,
        origin: &kind.origin,
        only: &kind.only,
        args,
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
    let end = gdfl_run::execute_kind(&plan, kind, Some(&told));
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
