//! Test-only: run ONE test of this binary again, in a child process whose
//! environment the test chooses.
//!
//! # Why a child process
//!
//! Some routes read their configuration from the process environment and take
//! no argument a test could pass instead: `trades_json` and `frontier_json`
//! find their store through `server::store_dir`, which reads `BRUTEX_STORE`,
//! and the sweep routes read `BRUTEX_SCREEN_BUDGET_MS` through `cli::knobs`.
//! A test may not set its own process's environment: `set_var` is `unsafe`
//! under edition 2024, this crate denies `unsafe`, and the environment is
//! shared by every test running beside it. A child process owns its own.
//!
//! # Why the child must PROVE it ran
//!
//! `--exact` with a name that matches nothing is not an error to the test
//! harness: it prints `0 passed` and exits zero. A parent that only checked the
//! exit status would then pass on a child that tested nothing -- the test that
//! asserts nothing `CLAUDE.md` §4 bans. So [`rerun`] requires the harness's own
//! `1 passed` line, and each caller also asserts on a line its child printed.

/// Runs `test` alone in a child of this test binary with `env` added to the
/// inherited environment, and returns the child's standard output.
///
/// The environment is inherited rather than cleared so that a coverage run's
/// `LLVM_PROFILE_FILE` reaches the child and its lines are counted.
///
/// # Panics
///
/// When the binary cannot be re-run, the child fails, or the harness did not
/// run exactly one test.
#[allow(
    clippy::expect_used,
    reason = "a fixture that cannot start its child must fail the test loudly"
)]
pub(crate) fn rerun(test: &str, env: &[(&str, &std::ffi::OsStr)]) -> String {
    let output = std::process::Command::new(std::env::current_exe().expect("this test binary"))
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .envs(env.iter().copied())
        .output()
        .expect("the child test process starts");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "child {test} failed:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("1 passed"),
        "child {test} did not run exactly one test:\n{stdout}\n{stderr}"
    );
    stdout
}
