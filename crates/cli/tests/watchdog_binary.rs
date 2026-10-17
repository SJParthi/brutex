//! The `cli` binary runs `watchdog` and `launch-agent` with the store drive
//! away, and still refuses every other verb there (R3-32, D-4996).
//!
//! `main` proves the store root before any verb runs. The watchdog must run
//! exactly when the drive is unmounted, because that is one of the outages it
//! reports, so its two verbs are the only ones `main` lets past. Every
//! directory here is a scratch directory and the "app" is a port nobody
//! listens on.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

use std::path::PathBuf;
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-cli-watchdog-bin-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cli"))
}

#[test]
fn the_watchdog_runs_with_the_store_drive_away() {
    let dir = scratch("away");
    let missing = dir.join("Volumes").join("DRIVE").join("store");
    let state = dir.join("state");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let url = format!("http://127.0.0.1:{port}/health");
    let run = || {
        cli()
            .env("BRUTEX_STORE", &missing)
            .env("BRUTEX_LOG_DIR", dir.join("events"))
            .args(["watchdog", &url, "--state-dir"])
            .arg(&state)
            .arg("--store")
            .arg(&missing)
            .args(["--notify", "record"])
            .output()
            .unwrap()
    };
    let first = run();
    let shown = String::from_utf8_lossy(&first.stdout);
    assert_eq!(first.status.code(), Some(1), "no answer exits 1: {shown}");
    assert!(shown.contains("gave no answer"), "{shown}");
    assert!(shown.contains("connection refused"), "{shown}");
    assert!(
        !shown.contains("events ->")
            && !String::from_utf8_lossy(&first.stderr).contains("events ->"),
        "a once-a-minute probe prints no banner: {shown}"
    );
    assert!(
        !shown.contains("store root"),
        "the preflight was skipped: {shown}"
    );
    let second = run();
    let shown = String::from_utf8_lossy(&second.stdout);
    assert!(shown.contains("2 in a row; down"), "{shown}");
    let log = std::fs::read_to_string(state.join("watchdog.log")).unwrap();
    assert!(log.contains(" down url="), "{log}");
    assert!(log.contains("store=absent"), "{log}");
    assert!(!missing.exists(), "the store root was not created");
    let pages = std::fs::read_dir(state.join("notices")).unwrap().count();
    assert_eq!(pages, 1);
}

#[test]
fn launch_agent_prints_with_the_store_drive_away_and_other_verbs_still_refuse() {
    let dir = scratch("agent");
    let missing = dir.join("Volumes").join("DRIVE").join("store");
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    let out = cli()
        .env("BRUTEX_STORE", &missing)
        .env("BRUTEX_LOG_DIR", dir.join("events"))
        .args([
            "launch-agent",
            "app",
            env!("CARGO_BIN_EXE_cli"),
            "127.0.0.1:8080",
            "--store",
        ])
        .arg(&missing)
        .arg("--log-dir")
        .arg(&logs)
        .output()
        .unwrap();
    let shown = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{shown}");
    assert!(
        shown.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"),
        "stdout is the agent file and nothing else: {shown}"
    );
    assert!(shown.ends_with("</plist>\n"), "{shown}");
    assert!(shown.contains("<key>PathState</key>"), "{shown}");
    let told = String::from_utf8_lossy(&out.stderr);
    assert!(
        told.contains("events"),
        "where events went is said on stderr: {told}"
    );
    assert!(!missing.exists());
    // Every other verb is still refused before dispatch when the root is away.
    let refused = cli()
        .env("BRUTEX_STORE", &missing)
        .env("BRUTEX_LOG_DIR", dir.join("events"))
        .args(["alerts"])
        .output()
        .unwrap();
    let shown = String::from_utf8_lossy(&refused.stdout);
    assert_eq!(refused.status.code(), Some(1), "{shown}");
    assert!(shown.starts_with("refused: store root"), "{shown}");
}
