//! `tick-to-second-check --out` named RELATIVE to the process's current
//! directory, through the binary itself (verify-DATA-2-r1, D-5187).
//!
//! A bare relative name is the one form a unit test cannot reach: it needs the
//! process to stand inside the store, and a test thread that moved the test
//! binary's current directory would move every other test's with it. So the
//! binary runs here as the operator's shell runs it, `cd <root> && cli ...`,
//! once standing in the store and once inside the tick store. Both ran in full
//! before and left their directory, lock, segments and versions there.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The binary under test, as cargo built it beside this test.
fn bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_BIN_EXE_cli"));
    p.set_extension(std::env::consts::EXE_EXTENSION);
    p
}

/// A fresh directory of this process's own under the host's temporary root.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-t2s-out-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs `tick-to-second-check 2025 --out <out>` standing in `cwd`, with the
/// store at `store` and, when given, the tick store at `ticks`.
fn run(cwd: &Path, store: &Path, ticks: Option<&Path>, out: &str, logs: &Path) -> (i32, String) {
    let mut command = Command::new(bin());
    command
        .current_dir(cwd)
        .env("BRUTEX_STORE", store)
        .env("BRUTEX_LOG_DIR", logs)
        .env_remove(pull::gdfl_run::TICKSTORE_VAR)
        .args(["tick-to-second-check", "2025", "--out", out]);
    if let Some(ticks) = ticks {
        command.env(pull::gdfl_run::TICKSTORE_VAR, ticks);
    }
    let done = command.output().expect("the binary runs");
    (
        done.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&done.stdout).into_owned(),
    )
}

/// Every name directly under `dir`, sorted.
fn names(dir: &Path) -> Vec<String> {
    let mut all: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    all.sort_unstable();
    all
}

#[test]
fn a_bare_relative_out_standing_in_the_store_or_the_tick_store_is_refused_before_anything_is_made()
{
    let root = scratch("bare");
    let store = root.join("store");
    let ticks = root.join("ticks");
    let logs = root.join("logs");
    std::fs::create_dir_all(store.join("bars")).unwrap();
    std::fs::create_dir_all(ticks.join("cm")).unwrap();
    let store_before = names(&store);
    let cm_before = names(&ticks.join("cm"));

    // (a) Standing in the store: `--out reports`.
    let (code, text) = run(&store, &store, None, "reports", &logs);
    assert_eq!(code, i32::from(cli::FAILED), "{text}");
    assert!(text.contains("lies inside the store"), "{text}");
    assert_eq!(names(&store), store_before, "nothing was made in the store");

    // `./reports` and `bars/../reports` from the same place.
    for asked in ["./reports", "bars/../reports", "nope/../reports"] {
        let (code, text) = run(&store, &store, None, asked, &logs);
        assert_eq!(code, i32::from(cli::FAILED), "{asked}: {text}");
        assert!(text.contains("lies inside the store"), "{asked}: {text}");
        assert_eq!(names(&store), store_before, "{asked}: nothing was made");
    }

    // (c) Standing in the tick store's `cm`: `--out relt`.
    let (code, text) = run(&ticks.join("cm"), &store, Some(&ticks), "relt", &logs);
    assert_eq!(code, i32::from(cli::FAILED), "{text}");
    assert!(text.contains("lies inside the tick store"), "{text}");
    assert_eq!(
        names(&ticks.join("cm")),
        cm_before,
        "nothing was made in the tick store"
    );

    // The same bare name standing outside both roots is a run, not a refusal:
    // the refusal is about where, never about the form.
    let outside = root.join("work");
    std::fs::create_dir_all(&outside).unwrap();
    let (_, text) = run(&outside, &store, Some(&ticks), "reports", &logs);
    assert!(!text.contains("lies inside"), "{text}");
    assert!(outside.join("reports").is_dir(), "{text}");
}
