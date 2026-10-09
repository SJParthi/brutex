//! sobs-1, D-4464: a panic under `cli::panic_log` leaves ONE event in the log,
//! with its message and location, and neither a closed stderr nor a missing
//! sink turns the panic into an abort.
//!
//! A panic hook and a log sink are both process-wide, so the panic is raised in
//! a CHILD: this same test binary, re-run on this one test with an environment
//! variable that makes it the child. The parent closes the child's stderr
//! before it starts and reads the child's log directory afterwards.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

use std::process::{Command, Stdio};

/// Set in the child: the log directory it writes to, or `nosink`.
const CHILD: &str = "BRUTEX_PANIC_HOOK_CHILD";
const NAME: &str = "a_panic_is_logged_once_and_a_closed_stderr_or_missing_sink_never_aborts";

#[test]
fn a_panic_is_logged_once_and_a_closed_stderr_or_missing_sink_never_aborts() {
    if let Some(mode) = std::env::var_os(CHILD) {
        if mode != "nosink" {
            let _sink = telemetry::install(&telemetry::Config::new(std::path::Path::new(&mode)))
                .expect("the child's sink installs");
        }
        cli::panic_log::install("cli.test");
        panic!("deliberate panic under the hook");
    }

    let logs = std::env::temp_dir().join(format!("brutex-cli-panic-hook-{}", std::process::id()));
    let _stale = std::fs::remove_dir_all(&logs);
    for mode in [logs.as_os_str(), std::ffi::OsStr::new("nosink")] {
        let (reader, writer) = std::io::pipe().expect("a pipe");
        drop(reader);
        let out = Command::new(std::env::current_exe().expect("this test binary"))
            .args(["--exact", NAME, "--nocapture", "--test-threads", "1"])
            .env(CHILD, mode)
            .stdout(Stdio::piped())
            .stderr(writer)
            .output()
            .expect("the child runs");
        let said = String::from_utf8_lossy(&out.stdout);
        // 101 is the harness reporting a failed test. An abort -- the hook or
        // the default hook panicking on the closed stderr -- has no code.
        assert_eq!(
            out.status.code(),
            Some(101),
            "the panic unwound to the harness ({mode:?}):\n{said}"
        );
        assert!(said.contains("FAILED"), "{said}");
    }

    let found = telemetry::tail(
        &logs,
        telemetry::Config::new(&logs).keep_files,
        &telemetry::Query::last(16).from_target("cli.test"),
    );
    assert_eq!(
        found.records.len(),
        1,
        "one panic, one event: {:?}",
        found.records
    );
    let event = found.records.first().expect("the panic event");
    assert_eq!(event.message, "process panicked");
    assert_eq!(event.level, telemetry::Level::Error);
    let text = |name: &str| {
        event
            .field(name)
            .and_then(telemetry::OwnedValue::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    assert_eq!(text("message"), "deliberate panic under the hook");
    assert!(
        text("location").contains("panic_hook.rs:"),
        "{:?}",
        event.fields
    );
    assert!(
        event
            .field("pid")
            .and_then(telemetry::OwnedValue::as_u64)
            .is_some_and(|pid| pid != u64::from(std::process::id())),
        "the child's own pid: {:?}",
        event.fields
    );
    let _cleaned = std::fs::remove_dir_all(&logs);
}
