//! `cli gdfl-stop (KIND | --run ID)`: asks the live GDFL run of a kind, or
//! the run with an id, to stop at its next file boundary (IA-2; D-4827,
//! D-4853, D-4893; test IA-GR-21).
//!
//! It writes one stop file beside the lock that holds the run's claim
//! (`pull::gdfl_run::stop::request_stop`): synced and renamed before this
//! verb answers, so a stop it said it wrote survives a crash. It works on a
//! run the api's child holds as on one a terminal holds; it never signals a
//! process and never takes the run's lock. The run reads the file before each
//! file it takes, closes the day it was in by D-4853's rules, releases and
//! removes the file. A second stop of the same run keeps the first asker.
//!
//! **Never refused for an unstamped build** (D-4894, by name): a stop must
//! always be possible, and the one file it writes changes no bar, no census
//! and no journal line (`store_write`'s classification lists it among the
//! verbs that do not write the store, with this reason).
//!
//! Exit: `0` the stop file was written (or a first asker's already names the
//! run); `1` no live run matches, its claim is being written, or the file
//! could not be written; `2` a misused verb.

use std::fmt::Write as _;
use std::path::PathBuf;

use pull::gdfl_run::stop::{self, StopTarget};

use crate::{FAILED, MISUSED, OK};

/// The kinds a stop may name.
const KINDS: [&str; 3] = ["indices", "stocks", "options"];

/// `cli gdfl-stop`, against the CLI's store root (`BRUTEX_STORE`).
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    command_with(
        arguments,
        &crate::store_root,
        pull::gdfl_run::clock_now(),
        out,
    )
}

/// The verb with its store root and clock passed in.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    now: u64,
    out: &mut String,
) -> u8 {
    let target = match arguments {
        [kind] if KINDS.contains(kind) => StopTarget::Kind(kind),
        ["--run", word] => match word.parse::<u64>() {
            Ok(id) if id.to_string() == *word => StopTarget::Run(id),
            _ => {
                let _ = writeln!(out, "refused: --run takes a run id (digits), not {word:?}");
                return MISUSED;
            }
        },
        _ => {
            let _ = writeln!(
                out,
                "refused: gdfl-stop takes KIND (indices, stocks or options) or --run ID"
            );
            return MISUSED;
        }
    };
    let store = match store() {
        Ok(store) => store,
        Err(why) => {
            let _ = writeln!(out, "GDFL STOP REFUSED: {why}");
            return FAILED;
        }
    };
    match stop::request_stop(&store, target, "cli", now) {
        Ok(answer) => {
            let _ = writeln!(
                out,
                "GDFL STOP REQUESTED: run {} ({}, held by {}) stops at its next file boundary",
                answer.run, answer.kind, answer.held_by
            );
            let _ = writeln!(
                out,
                "  stop file           {} (asked by {} at {} UNIX s)",
                pull::gdfl_run::locks::stop_name(&answer.lock),
                answer.request.by,
                answer.request.requested
            );
            OK
        }
        Err(refused) => {
            let _ = writeln!(out, "GDFL STOP REFUSED: {refused}");
            FAILED
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        reason = "a test that cannot panic cannot fail"
    )]

    use super::*;
    use pull::gdfl_run::locks::Claim;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("brutex-gdfl-stop-{tag}-{}", std::process::id()));
        let _fresh = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn claim(kind: &str) -> Claim {
        Claim {
            run: 0,
            mode: "import".to_owned(),
            kind: kind.to_owned(),
            batch: String::new(),
            source: "tickstore".to_owned(),
            from: "2024-04-01".to_owned(),
            to: "2024-04-01".to_owned(),
            filter: "*".to_owned(),
            verify: "off".to_owned(),
            restate: false,
            repair: "0".to_owned(),
            workers: 1,
            auto_resume: false,
            pid: 4242,
            host: "-".to_owned(),
            exe: "-".to_owned(),
            by: "api".to_owned(),
            started: 0,
            resumes: 0,
            resume_day: "-".to_owned(),
            legacy_ack: false,
            unstamped: false,
            request: "-".to_owned(),
        }
    }

    /// **IA-GR-21's verb: `cli gdfl-stop` stops a run by kind or by id,
    /// whoever holds it, and names a stop with no live run.** The run's own
    /// stop sees the file at its next check; a misused verb exits 2.
    #[test]
    fn gdfl_stop_names_the_run_it_stopped_and_refuses_one_not_live() {
        let store = scratch("verb");
        let root = || Ok(store.clone());
        let mut out = String::new();
        assert_eq!(
            command_with(&["options"], &root, 1_790_812_800, &mut out),
            FAILED
        );
        assert!(out.contains("GDFL STOP REFUSED: run_not_live"), "{out}");

        let claimed =
            pull::gdfl_run::claim_with(&store, claim("options"), 0, None, 1_790_812_800).unwrap();
        let mut out = String::new();
        assert_eq!(
            command_with(&["options"], &root, 1_790_812_801, &mut out),
            OK,
            "{out}"
        );
        assert!(
            out.contains(&format!("run {} (options, held by api)", claimed.run))
                && out.contains("imports/gdfl.stop (asked by cli at 1790812801"),
            "{out}"
        );
        assert!(claimed.stop.requested());
        let mut out = String::new();
        let by_id = claimed.run.to_string();
        assert_eq!(
            command_with(&["--run", &by_id], &root, 1_790_812_809, &mut out),
            OK
        );
        assert!(
            out.contains("at 1790812801"),
            "the first asker is kept: {out}"
        );
        claimed.release().unwrap();

        for misuse in [
            &[][..],
            &["futures"][..],
            &["--run", "x1"][..],
            &["--run", "01"][..],
        ] {
            let mut out = String::new();
            assert_eq!(
                command_with(misuse, &root, 0, &mut out),
                MISUSED,
                "{misuse:?}"
            );
        }
        let mut out = String::new();
        let gone = || Err("BRUTEX_STORE is not set".to_owned());
        assert_eq!(command_with(&["indices"], &gone, 0, &mut out), FAILED);
        assert!(out.starts_with("GDFL STOP REFUSED: BRUTEX_STORE"), "{out}");
    }
}
