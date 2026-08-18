//! The binary itself, run once per command, so `main` is measured.
//!
//! `cargo test` replaces `main` with its own harness and never enters it, so
//! without this file the one call in `main.rs` is a line no test reaches — the
//! exact gap `crates/api/tests/binary.rs` exists to close for the other binary.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

use std::process::Command;

/// The binary under test, as cargo built it beside this test.
fn bin() -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_BIN_EXE_cli"));
    p.set_extension(std::env::consts::EXE_EXTENSION);
    p
}

#[test]
fn a_sweep_runs_end_to_end_and_says_its_bars_were_generated() {
    let out = Command::new(bin())
        .args(["sweep", "2", "50"])
        .output()
        .expect("the binary runs");
    assert!(out.status.success(), "a valid sweep exits zero");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("THESE BARS ARE GENERATED"),
        "a report that does not declare its provenance is the one thing this \
         binary must never print:\n{text}"
    );
    assert!(
        text.contains("bars"),
        "the report body rendered, not just the banner:\n{text}"
    );
}

#[test]
fn the_threshold_search_runs_end_to_end() {
    let out = Command::new(bin())
        .args(["auto", "2"])
        .output()
        .expect("the binary runs");
    assert!(out.status.success(), "a valid auto exits zero");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("THESE BARS ARE GENERATED"),
        "provenance travels"
    );
}

#[test]
fn a_word_this_build_does_not_know_is_refused_by_name() {
    let out = Command::new(bin())
        .args(["backtest"])
        .output()
        .expect("the binary runs");
    assert_eq!(
        out.status.code(),
        Some(i32::from(cli::MISUSED)),
        "an unknown command is a misuse, not a failure"
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("backtest"),
        "the refusal names the word it objected to:\n{text}"
    );
    assert!(text.contains("usage:"), "and prints the usage:\n{text}");
}

#[test]
fn no_command_at_all_is_refused_rather_than_defaulted() {
    let out = Command::new(bin()).output().expect("the binary runs");
    assert_eq!(out.status.code(), Some(i32::from(cli::MISUSED)));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("no command given"),
        "silence is not a command, and guessing one would be the fallback \
         CLAUDE.md section 4 bans:\n{text}"
    );
}
