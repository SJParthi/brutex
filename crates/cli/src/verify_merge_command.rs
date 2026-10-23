//! `cli verify-merge` — merge every Zerodha verification spool waiting into
//! the verdict store, in a process of its own (IA-9; D-4881, D-4921).
//!
//! # Usage
//!
//! ```text
//! cli verify-merge [--retry SPOOL]
//! cli verify-merge --list
//! ```
//!
//! # What it does
//!
//! Takes `<store>/verify/merge.lock` (a second merger exits `run_busy` at
//! once, nothing read), a run id from the store-wide runs ledger (mode
//! `verify-merge`), and merges each spool under `verify/spool/` into one
//! verdict generation per (segment, timeframe, day) it touches, through
//! `pull::zerodha_verify::merge_all`; then lists the spool tree once more and
//! merges what arrived meanwhile. A merged spool and its merge log are
//! removed; a damaged one is refused by name, once, with an alert, and is not
//! tried again until `--retry SPOOL` names it. A stop file
//! (`verify/merge.stop`) is read between days.
//!
//! **This is where every Zerodha merge runs**: the api starts this verb as a
//! child process when a landing request that spooled ends, and a few seconds
//! after it serves (never on its start path); the operator runs it with the
//! api down. Never inside the api (round 4, SF-13).
//!
//! `--list` writes nothing: it prints the spools waiting and the stray names
//! in the tree.
//!
//! # Logs
//!
//! A merge logs to `<store>/logs/verify/merge`, the directory its lock makes
//! it alone in (`telemetry::run_dirs::VERIFY_MERGE`, D-4862, D-4897).
//!
//! # Exit
//!
//! [`OK`] when every spool waiting was merged or none waited; [`FAILED`]
//! for `run_busy`, a refused spool, a stop, or a store that would not take
//! the run; [`MISUSED`] for arguments it does not take.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pull::gdfl_run::ledger;
use pull::gdfl_run::stop::StopControl;
use pull::zerodha_verify::{self, MergeError, MergeOutcome, spool};

use crate::{FAILED, MISUSED, OK};

/// The verb's shape, printed with every misuse.
pub(crate) const USAGE: &str = "cli verify-merge [--retry SPOOL] | cli verify-merge --list";

/// The stop file a merge reads between days.
pub const STOP_FILE: &str = "verify/merge.stop";

/// What the arguments ask.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Ask {
    /// List what waits; write nothing.
    List,
    /// Merge, retrying one refused spool when named.
    Merge(Option<PathBuf>),
}

fn ask(arguments: &[&str]) -> Result<Ask, String> {
    match arguments {
        [] => Ok(Ask::Merge(None)),
        ["--list"] => Ok(Ask::List),
        ["--retry", path] if !path.is_empty() => Ok(Ask::Merge(Some(PathBuf::from(path)))),
        _ => Err(format!(
            "verify-merge takes no argument, `--list`, or `--retry SPOOL`; it was given `{}`",
            crate::stored::clipped(&arguments.join(" "))
        )),
    }
}

/// A run id from the store-wide runs ledger (D-4880), for `mode`.
///
/// # Errors
///
/// The ledger's refusal, in its words.
pub(crate) fn run_id(root: &Path, mode: &str) -> Result<u64, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    ledger::allocate(
        root,
        &ledger::Entry {
            id: 0,
            mode: mode.to_owned(),
            kind: "zerodha".to_owned(),
            by: "cli".to_owned(),
            pid: std::process::id(),
            started: now,
            resumes: 0,
        },
    )
    .map_err(|why| why.to_string())
}

/// The merge's outcome as plain lines, one table.
pub(crate) fn render(outcome: &MergeOutcome, out: &mut String) {
    let _ = writeln!(
        out,
        "  {:<24} {:>10}",
        "spools merged", outcome.spools_merged
    );
    let _ = writeln!(
        out,
        "  {:<24} {:>10}",
        "generations committed", outcome.generations
    );
    let _ = writeln!(out, "  {:<24} {:>10}", "records committed", outcome.records);
    let _ = writeln!(
        out,
        "  {:<24} {:>10}",
        "days already merged", outcome.days_already
    );
    let _ = writeln!(
        out,
        "  {:<24} {:>10}",
        "days recovered", outcome.days_recovered
    );
    let _ = writeln!(
        out,
        "  {:<24} {:>10}",
        "spools refused now",
        outcome.refused.len()
    );
    let _ = writeln!(
        out,
        "  {:<24} {:>10}",
        "refused earlier",
        outcome.refused_earlier.len()
    );
    let _ = writeln!(
        out,
        "  {:<24} {:>10}",
        "torn tails ignored",
        outcome.torn.len()
    );
    let _ = writeln!(out, "  {:<24} {:>10}", "stray names", outcome.stray);
    let _ = writeln!(
        out,
        "  {:<24} {:>10}",
        "spools still waiting", outcome.waiting
    );
    let _ = writeln!(
        out,
        "  {:<24} {:>10}",
        "runs still spooling",
        outcome.live.len()
    );
    for (run, spools, holder) in &outcome.live {
        let _ = writeln!(
            out,
            "  LIVE run {run} is still spooling: its {spools} spool(s) were not read and wait for its end ({holder})"
        );
    }
    for (path, why) in &outcome.refused {
        let _ = writeln!(out, "  REFUSED {}: {why}", path.display());
    }
    for path in &outcome.refused_earlier {
        let _ = writeln!(
            out,
            "  REFUSED EARLIER {} (cli verify-merge --retry {} reads it again)",
            path.display(),
            path.display()
        );
    }
    for (path, at, bytes) in &outcome.torn {
        let _ = writeln!(
            out,
            "  TORN TAIL {} at byte {at}, {bytes} byte(s) ignored",
            path.display()
        );
    }
    for (path, why) in &outcome.not_removed {
        let _ = writeln!(out, "  NOT REMOVED {}: {why}", path.display());
    }
    for fault in &outcome.faults {
        let _ = writeln!(out, "  FAULT {fault}");
    }
    if outcome.stopped {
        let _ = writeln!(
            out,
            "  STOPPED: a stop was asked; the spools left wait for the next merge"
        );
    }
}

/// [`command`] over a named store root, so every arm is driven by a test.
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
            let _ = writeln!(
                out,
                "VERIFY-MERGE REFUSED: {why}. Nothing was read or written."
            );
            return FAILED;
        }
    };
    match ask {
        Ask::List => list(&root, out),
        Ask::Merge(retry) => merge(&root, retry.as_deref(), out),
    }
}

/// `cli verify-merge`: see the module header.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    let store = crate::store_write::root_for("verify-merge", arguments);
    command_with(arguments, &store, out)
}

fn list(root: &Path, out: &mut String) -> u8 {
    match spool::waiting(root) {
        Ok(waiting) => {
            let _ = writeln!(
                out,
                "VERIFY-MERGE: {} spool(s) waiting",
                waiting.spools.len()
            );
            for id in &waiting.spools {
                let _ = writeln!(out, "  {}", id.path(root).display());
            }
            for path in &waiting.stray {
                let _ = writeln!(out, "  stray (never read): {}", path.display());
            }
            OK
        }
        Err(why) => {
            let _ = writeln!(out, "VERIFY-MERGE REFUSED: {why}");
            FAILED
        }
    }
}

/// A progress file that could not be written, said (the run goes on: the
/// record is a view of the run, never its record, D-4904).
pub(crate) fn progress_said(progress: &pull::gdfl_run::ProgressFile, out: &mut String) {
    if let Some(fault) = progress.fault() {
        let _ = writeln!(out, "  progress file not written: {fault}");
    }
}

fn merge(root: &Path, retry: Option<&Path>, out: &mut String) -> u8 {
    // THE LOCK BEFORE THE RUN'S NUMBER: a merger that finds another one
    // running exits `run_busy` without taking a runs-ledger id.
    let lock = match zerodha_verify::merge_lock(root) {
        Ok(lock) => lock,
        Err(MergeError::RunBusy { holder }) => {
            let _ = writeln!(
                out,
                "VERIFY-MERGE run_busy: another merger holds {} ({holder}). The spools wait for it or the next trigger.",
                spool::MERGE_LOCK
            );
            return FAILED;
        }
        Err(why) => {
            let _ = writeln!(out, "VERIFY-MERGE REFUSED: {why}");
            return FAILED;
        }
    };
    let run = match run_id(root, "verify-merge") {
        Ok(run) => run,
        Err(why) => {
            let _ = writeln!(
                out,
                "VERIFY-MERGE REFUSED: the runs ledger gave no run id: {why}. Nothing was merged."
            );
            return FAILED;
        }
    };
    let stop = StopControl::new(root.join(STOP_FILE), run);
    let progress = pull::gdfl_run::ProgressFile::of_record(
        root.join(zerodha_verify::PROGRESS_MERGE),
        zerodha_verify::progress_record("verify-merge", 1, run, 1, None),
    );
    let merged = zerodha_verify::merge_under(
        lock,
        root,
        (retry, Some(&stop), Some(&progress)),
        zerodha_verify::merge_workers(),
    );
    progress_said(&progress, out);
    match merged {
        Ok(outcome) => {
            let _ = writeln!(out, "VERIFY-MERGE run {run}");
            render(&outcome, out);
            // A LIVE RUN'S SPOOLS WAIT BY RULE, named above; any other spool
            // left waiting is not merged and fails the run.
            if outcome.refused.is_empty()
                && outcome.faults.is_empty()
                && !outcome.stopped
                && outcome.waiting_not_live() == 0
            {
                OK
            } else {
                FAILED
            }
        }
        Err(why) => {
            let _ = writeln!(out, "VERIFY-MERGE REFUSED: {why}");
            FAILED
        }
    }
}

/// The sink of a verification run: `verify-merge` logs to
/// `logs/verify/merge`, `zerodha-verify` to `logs/verify/zerodha`, the
/// directory each one's lock makes it alone in (D-4862, D-4897). `None` for
/// any other verb, which keeps the default sink.
#[must_use]
pub fn install_run_log(args: &[OsString]) -> Option<String> {
    let dir = match args.first().and_then(|verb| verb.to_str()) {
        Some("verify-merge") => telemetry::run_dirs::VERIFY_MERGE,
        Some("zerodha-verify") => telemetry::run_dirs::VERIFY_ZERODHA,
        _ => return None,
    };
    let store = crate::store_root().ok()?;
    let at = dir.beneath(&store);
    Some(match telemetry::install(&telemetry::Config::new(&at)) {
        Ok(_) => format!(
            "events -> {}\n  This run logs to its own directory (D-4862); /logs reads it.",
            at.display()
        ),
        Err(why) => format!("{}: {why}", crate::UNLOGGED),
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "the same exception every test module in this workspace takes"
    )]

    use super::*;
    use brutex_core::instrument::{Exchange, Segment};
    use pull::session::Day;
    use pull::zerodha_verify::spool::{SpoolRecord, SpoolWriter};
    use store::path::Timeframe;
    use store::verdict::{Code, Facts, Key, Mode};

    fn scratch(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "brutex-ia9-cli-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos())
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn verify_merge_takes_only_its_own_arguments() {
        assert_eq!(ask(&[]), Ok(Ask::Merge(None)));
        assert_eq!(ask(&["--list"]), Ok(Ask::List));
        assert_eq!(
            ask(&["--retry", "x.vsp"]),
            Ok(Ask::Merge(Some(PathBuf::from("x.vsp"))))
        );
        for wrong in [
            &["--retry"][..],
            &["--retry", ""],
            &["--list", "--list"],
            &["merge"],
        ] {
            let mut out = String::new();
            let code = command_with(wrong, &|| panic!("never opened"), &mut out);
            assert_eq!(code, MISUSED, "{wrong:?}");
            assert!(out.contains("usage: cli verify-merge"), "{out}");
        }
        let mut out = String::new();
        let code = command_with(&[], &|| Err("no store".to_owned()), &mut out);
        assert_eq!(code, FAILED);
        assert!(out.contains("Nothing was read or written"));
    }

    #[test]
    fn verify_merge_merges_lists_and_refuses_a_busy_lock() {
        let root = scratch("merge");
        let on = Day::new(2024, 4, 23).unwrap();
        let writer = SpoolWriter::new(&root, 900, Mode::ZerodhaMerge, 1);
        writer
            .append(
                Segment::Index,
                Timeframe::MINUTE_1,
                &[SpoolRecord {
                    exchange: Exchange::Nse,
                    day: on,
                    key: Key::instrument("NSE-INDEX-ZZIDX").unwrap(),
                    facts: Facts::of(Code::Match),
                    receipt: 1,
                    details: Vec::new(),
                }],
            )
            .unwrap();
        let store = || Ok(root.clone());
        let mut out = String::new();
        assert_eq!(command_with(&["--list"], &store, &mut out), OK);
        assert!(out.contains("1 spool(s) waiting"), "{out}");
        // A merger holding the lock: this one exits run_busy, nothing merged,
        // and it takes no runs-ledger id (the next id is the one after the
        // id taken just before it).
        let file = pull::gdfl_run::locks::open_lock_file(&root, spool::MERGE_LOCK).unwrap();
        let lock = store::flock::Flock::try_lock(file, root.join(spool::MERGE_LOCK)).unwrap();
        let before = run_id(&root, "verify-merge").unwrap();
        let mut out = String::new();
        assert_eq!(command_with(&[], &store, &mut out), FAILED);
        assert!(out.contains("run_busy"), "{out}");
        // The lock and its holder are two phrases, never run together
        // ("verify/merge.lockheld by": the second check, IA-9 repair 2).
        assert!(out.contains("holds verify/merge.lock (held by "), "{out}");
        assert_eq!(run_id(&root, "verify-merge").unwrap(), before + 1, "{out}");
        drop(lock);
        // The run that spooled is still live: its spool is named and waits,
        // and that is no failure of the merge (IA-9 repair 1).
        let mut out = String::new();
        assert_eq!(command_with(&[], &store, &mut out), OK, "{out}");
        assert!(out.contains("LIVE run 900 is still spooling"), "{out}");
        let live_row = |n: usize| format!("{:<24} {n:>10}", "runs still spooling");
        assert!(out.contains(&live_row(1)), "{out}");
        drop(writer);
        let mut out = String::new();
        assert_eq!(command_with(&[], &store, &mut out), OK, "{out}");
        assert!(out.contains(&live_row(0)), "{out}");
        assert!(out.contains("generations committed"), "{out}");
        assert!(out.contains("         1"), "{out}");
        let mut out = String::new();
        assert_eq!(command_with(&["--list"], &store, &mut out), OK);
        assert!(out.contains("0 spool(s) waiting"), "{out}");
    }
}
