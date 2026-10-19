//! Where a `cli gdfl-import` run logs, measured on the BINARY (IA-1;
//! D-4862, D-4903; FM-065, RA-118, the leftover DB-A41).
//!
//! `main` installs the process's one event sink before any command runs, so
//! only the binary can show which directory a run logs to, that two kinds at
//! once both log, and that a run whose sink is refused refuses before any
//! lock. Every store here is a fresh temporary directory, every source an
//! empty tick store (a `FORMAT.md` and no day file): no vendor byte.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-cli-gdfl-run-logs-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A store whose census is version 4 (`manifest/v4/gdfl/`), so each kind
/// has its own lock and log directory, and an empty tick store beside it.
fn fixture(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    // Canonical, as the cli makes the store root (`cli::existing_store_root`):
    // on macOS the temporary directory is `/var/...` for `/private/var/...`.
    let root = std::fs::canonicalize(scratch(tag)).unwrap();
    let store = root.join("store");
    std::fs::create_dir_all(store.join("manifest").join("v4").join("gdfl")).unwrap();
    let ts = root.join("ts");
    std::fs::create_dir_all(&ts).unwrap();
    std::fs::write(ts.join("FORMAT.md"), b"# synthetic tick store (cli test)\n").unwrap();
    (root, store, ts)
}

/// `cli gdfl-import KIND tickstore FROM TO` on `store`, with no inherited
/// log directory, as a child not yet waited for.
fn import(kind: &str, store: &Path, ts: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cli"));
    command
        .args(["gdfl-import", kind, "tickstore", "2024-04-01", "2024-04-05"])
        .env("BRUTEX_STORE", store)
        .env("BRUTEX_GDFL_TICKSTORE", ts)
        .env("BRUTEX_ALLOW_UNSTAMPED_WRITE", "1")
        .env("BRUTEX_GDFL_WORKERS", "1")
        .env_remove("BRUTEX_LOG_DIR");
    command
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Every record of `dir`'s current events file.
fn records(dir: &Path) -> Vec<telemetry::Record> {
    let bytes = std::fs::read(dir.join("events.ndjson")).unwrap_or_default();
    bytes
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| telemetry::Record::decode(line).unwrap())
        .collect()
}

/// **TWO KINDS AT ONCE BOTH LOG, EACH IN ITS OWN DIRECTORY** (IA-GR-24's
/// half, FM-065, RA-118): before IA-1 every cli run logged to `logs/cli`
/// under one lock, so the second of two kinds said "events are NOT being
/// recorded" and ran unlogged. Now each run's events are in
/// `logs/gdfl/<kind>`, and **every** importer line of a run, per day and per
/// refusal, carries the run's id as its record's `run` (DB-A41), so
/// `/logs.json?run=<id>` finds more than its start and finish.
#[test]
fn two_cli_kinds_at_once_both_log() {
    let (root, store, ts) = fixture("both");
    let indices = import("indices", &store, &ts).output();
    let stocks = import("stocks", &store, &ts).output();
    let both = std::thread::scope(|scope| {
        let a = scope.spawn(|| import("indices", &store, &ts).output().unwrap());
        let b = scope.spawn(|| import("stocks", &store, &ts).output().unwrap());
        (a.join().unwrap(), b.join().unwrap())
    });
    for (kind, run) in [("indices", &both.0), ("stocks", &both.1)] {
        let said = text(run);
        assert!(!said.contains("NOT being recorded"), "{kind}: {said}");
        let dir = store.join("logs").join("gdfl").join(kind);
        assert!(
            said.contains(&format!("events -> {}", dir.display())),
            "{kind}: {said}"
        );
        // A line a day on stderr, through the declared writer.
        assert!(
            String::from_utf8_lossy(&run.stderr)
                .contains(&format!("gdfl-import {kind} 2024-04-05:")),
            "{kind}: {said}"
        );
        let all = records(&dir);
        let started: Vec<&telemetry::Record> = all
            .iter()
            .filter(|r| r.target == "gdfl.import" && r.message == "run started")
            .collect();
        assert!(started.len() >= 2, "{kind}: {started:?}");
        for start in &started {
            assert_eq!(
                start.field("kind").and_then(telemetry::OwnedValue::as_str),
                Some(kind)
            );
            // The run's id is the record's key, not only a field of it.
            let id = start
                .field("run")
                .and_then(telemetry::OwnedValue::as_u64)
                .unwrap();
            assert_eq!(start.run, id, "{start:?}");
        }
        let importer: Vec<&telemetry::Record> =
            all.iter().filter(|r| r.target == "gdfl.import").collect();
        let per_day = importer
            .iter()
            .filter(|r| r.message == "the source holds no file of this trading day")
            .count();
        assert!(per_day >= 5, "{kind}: per-day lines {per_day}");
        for record in &importer {
            assert_ne!(
                record.run, 0,
                "{kind}: an importer line with no run: {record:?}"
            );
            if let Some(id) = record.field("run").and_then(telemetry::OwnedValue::as_u64) {
                assert_eq!(record.run, id, "{record:?}");
            }
        }
        // No run of a kind wrote anything into `logs/cli`.
        assert!(
            !store
                .join("logs")
                .join("cli")
                .join("events.ndjson")
                .exists(),
            "{kind}"
        );
        // The progress record, beside the kind's lock (D-4904).
        let progress = pull::gdfl_run::Progress::read(
            &store.join("imports").join(format!("gdfl.{kind}.progress")),
        )
        .unwrap();
        assert_eq!(progress.state, pull::gdfl_run::RunState::Finished.code());
        assert_eq!((progress.day_total, progress.days_not_held), (5, 5));
    }
    assert!(indices.is_ok() && stocks.is_ok());
    std::fs::remove_dir_all(root).unwrap();
}

/// **A RUN WHOSE SINK IS REFUSED REFUSES BEFORE ANY LOCK** (IA-GR-44,
/// D-4903): with the kind's log directory held by another process, the run
/// says why it has no log and refuses `log_unavailable`, exit 1, with no
/// journal, no lock file and no progress record written.
#[test]
fn a_run_whose_sink_is_held_refuses_before_any_lock() {
    let (root, store, ts) = fixture("held");
    let dir = store.join("logs").join("gdfl").join("indices");
    std::fs::create_dir_all(&dir).unwrap();
    let holder = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("events.lock"))
        .unwrap();
    holder.try_lock().unwrap();
    let run = import("indices", &store, &ts).output().unwrap();
    let said = text(&run);
    assert_eq!(run.status.code(), Some(1), "{said}");
    assert!(said.contains("events are NOT being recorded"), "{said}");
    assert!(
        said.contains("GDFL IMPORT REFUSED: log_unavailable: "),
        "{said}"
    );
    let imports = store.join("imports");
    for name in ["gdfl.journal", "gdfl.indices.lock", "gdfl.indices.progress"] {
        assert!(!imports.join(name).exists(), "{name}: {said}");
    }
    drop(holder);
    // Released, the same run logs and runs.
    let again = import("indices", &store, &ts).output().unwrap();
    assert!(
        !text(&again).contains("log_unavailable"),
        "{}",
        text(&again)
    );
    assert!(imports.join("gdfl.journal").exists());
    std::fs::remove_dir_all(root).unwrap();
}

/// **A LOG DIRECTORY OUTSIDE THE RUN DIRECTORIES IS REFUSED AT START**
/// (FM-065's CONC-070): a `BRUTEX_LOG_DIR` naming anywhere but the run's
/// own directory would put the run's evidence where no reader looks, so the
/// sink is not installed there and the run refuses `log_unavailable`; one
/// naming the run's own directory is the run's.
#[test]
fn a_log_directory_outside_the_run_directories_is_refused_at_start() {
    let (root, store, ts) = fixture("elsewhere");
    let elsewhere = root.join("cli-batch");
    let run = import("indices", &store, &ts)
        .env("BRUTEX_LOG_DIR", &elsewhere)
        .output()
        .unwrap();
    let said = text(&run);
    assert_eq!(run.status.code(), Some(1), "{said}");
    assert!(said.contains("Unset BRUTEX_LOG_DIR"), "{said}");
    assert!(
        said.contains("GDFL IMPORT REFUSED: log_unavailable: "),
        "{said}"
    );
    assert!(!elsewhere.exists(), "nothing was logged there");
    assert!(!store.join("imports").join("gdfl.journal").exists());
    // The run's own directory, named through a link to the store: one
    // directory, two spellings, so it is the run's and nothing is refused.
    let own = store.join("logs").join("gdfl").join("indices");
    let link = root.join("store-link");
    std::os::unix::fs::symlink(&store, &link).unwrap();
    let named = link.join("logs").join("gdfl").join("indices");
    let run = import("indices", &store, &ts)
        .env("BRUTEX_LOG_DIR", &named)
        .output()
        .unwrap();
    assert!(!text(&run).contains("log_unavailable"), "{}", text(&run));
    assert!(own.join("events.ndjson").exists());
    std::fs::remove_dir_all(root).unwrap();
}

/// **A SECOND PRESS ON A HELD LOCK IS REFUSED NAMING THE HOLDER** (IA-2
/// repair 1; FM-064, IA-GR-03), measured on the BINARY, two processes: this
/// test holds a run's claim through the one authority
/// (`pull::gdfl_run::claim_with`) and that run's log directory, as a live
/// `gdfl-import` does. A second `cli gdfl-import` that needs the same lock
/// (v4: the same kind; v3: any kind) finds the sink taken before it tries
/// any lock, and is refused `run_busy` naming the holder's kind, pid, start,
/// run and by, exit 1, never `log_unavailable` alone; it writes no journal,
/// and the holder keeps its lock.
#[test]
fn a_second_press_on_a_held_lock_is_refused_naming_the_holder() {
    for (layout, held, pressed, log) in [
        ("v4", "options", "options", "options"),
        ("v3", "options", "indices", "store"),
    ] {
        let (root, store, ts) = fixture(&format!("busy-{layout}"));
        if layout == "v3" {
            std::fs::remove_dir_all(store.join("manifest")).unwrap();
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claim = pull::gdfl_run::locks::Claim {
            run: 0,
            mode: "import".to_owned(),
            kind: held.to_owned(),
            batch: String::new(),
            source: "tickstore".to_owned(),
            from: "2024-04-01".to_owned(),
            to: "2024-04-05".to_owned(),
            filter: "*".to_owned(),
            verify: "off".to_owned(),
            restate: false,
            repair: "0".to_owned(),
            workers: 1,
            auto_resume: false,
            pid: std::process::id(),
            host: "-".to_owned(),
            exe: "-".to_owned(),
            by: "cli".to_owned(),
            started: 0,
            resumes: 0,
            resume_day: "-".to_owned(),
            legacy_ack: false,
            unstamped: false,
            request: "-".to_owned(),
        };
        let holder = pull::gdfl_run::claim_with(&store, claim, 0, None, now).unwrap();
        let dir = store.join("logs").join("gdfl").join(log);
        std::fs::create_dir_all(&dir).unwrap();
        let sink = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join("events.lock"))
            .unwrap();
        sink.try_lock().unwrap();
        let journal = store.join("imports").join("gdfl.journal");
        let journal_before = std::fs::read(&journal).ok();

        let run = import(pressed, &store, &ts).output().unwrap();
        let said = text(&run);
        assert_eq!(run.status.code(), Some(1), "{layout}: {said}");
        let lock = holder.locks.claim_name().to_owned();
        let named = format!(
            "GDFL IMPORT REFUSED: run_busy: another {held} run holds {lock}: pid {} started {} \
             (UNIX s), run {}, by cli",
            std::process::id(),
            holder.claim.started,
            holder.run
        );
        assert!(said.contains(&named), "{layout}: {said}");
        assert!(
            !said.contains("REFUSED: log_unavailable"),
            "{layout}: {said}"
        );
        assert_eq!(
            std::fs::read(&journal).ok(),
            journal_before,
            "{layout}: no journal line"
        );
        assert!(
            holder.locks.still_held(),
            "{layout}: the holder keeps its lock"
        );
        drop(sink);
        holder.release().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
