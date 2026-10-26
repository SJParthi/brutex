//! Where a `cli basis-runs` run that stops says why, measured on the BINARY
//! (R4-26 repair 4; check 4's follow-up "failure reasons only on stdout").
//!
//! A refusal or a failure of `cli basis-runs` was printed and nothing else:
//! the run's log held the lifecycle's `command finished` with its exit code
//! and no reason. `main` installs the process's one event sink before any
//! command runs, so only the binary can show that the reason now reaches the
//! log. Every store here is a fresh temporary directory, and every input two
//! EMPTY files named for months: no vendor byte.

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
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-cli-basis-runs-logs-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
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

/// A store (canonical, as the cli makes its root: on macOS the temporary
/// directory is `/var/...` for `/private/var/...`) and an input directory
/// whose first and last months lie 96,360 months apart: two empty files, so
/// the run is refused by its window bound (D-7197) before either is read.
fn fixture(tag: &str) -> (PathBuf, PathBuf) {
    let root = std::fs::canonicalize(scratch(tag)).unwrap();
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let input = root.join("in");
    std::fs::create_dir_all(&input).unwrap();
    for month in ["1970-01", "9999-12"] {
        std::fs::write(input.join(format!("feed-reconcile-{month}.v1.csv")), b"").unwrap();
    }
    (store, input)
}

fn basis_runs(store: &Path, arguments: &[&str]) -> (Option<i32>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_cli"))
        .arg("basis-runs")
        .args(arguments)
        .env("BRUTEX_STORE", store)
        .env("BRUTEX_ALLOW_UNSTAMPED_WRITE", "1")
        .env_remove("BRUTEX_LOG_DIR")
        .output()
        .unwrap();
    (
        output.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

/// The `cli.basis_runs` records of `store`'s cli log with `message`.
fn said(store: &Path, message: &str) -> Vec<telemetry::Record> {
    records(&store.join("logs").join("cli"))
        .into_iter()
        .filter(|record| record.target == "cli.basis_runs" && record.message == message)
        .collect()
}

#[test]
fn a_refused_run_says_why_on_its_log_as_well_as_on_stdout() {
    let (store, input) = fixture("refused");
    let (code, text) = basis_runs(&store, &["--dir", input.to_str().unwrap(), "--apply"]);
    assert_eq!(code, Some(1), "{text}");
    let line = "BASIS RUNS REFUSED: the window 1970-01..9999-12 spans 96360 months, more than \
                the 1200 one run reads; name a narrower --from/--to. Nothing was written.";
    assert!(text.contains(line), "{text}");
    let refused = said(&store, "basis runs refused");
    assert_eq!(
        refused.len(),
        1,
        "{:?}",
        records(&store.join("logs").join("cli"))
    );
    assert_eq!(refused[0].level, telemetry::Level::Error);
    assert_eq!(
        refused[0]
            .field("why")
            .and_then(telemetry::OwnedValue::as_str),
        Some(line),
        "the log carries the line stdout carries"
    );
    // Nothing was written: no table generation under reference/.
    assert!(!store.join("reference").exists(), "nothing written");
}

#[test]
fn a_misused_run_says_why_on_its_log_as_a_warning() {
    let (store, input) = fixture("misused");
    let (code, text) = basis_runs(
        &store,
        &[
            "--dir",
            input.to_str().unwrap(),
            "--from",
            "1970-01",
            "--to",
            "9999-12",
        ],
    );
    assert_eq!(code, Some(2), "{text}");
    assert!(text.contains("usage: "), "{text}");
    let misused = said(&store, "basis runs misused");
    assert_eq!(
        misused.len(),
        1,
        "{:?}",
        records(&store.join("logs").join("cli"))
    );
    assert_eq!(misused[0].level, telemetry::Level::Warn);
    let why = misused[0]
        .field("why")
        .and_then(telemetry::OwnedValue::as_str)
        .unwrap();
    assert!(
        why.starts_with("refused: --from 1970-01 --to 9999-12: the window")
            && !why.contains("usage"),
        "the reason, not the usage text: {why}"
    );
}
