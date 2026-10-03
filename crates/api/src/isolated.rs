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

/// The permission-bits rerun, D-0995: as root a test that `chmod`s a fixture
/// shut proves nothing, because root bypasses the mode bits. The named test is
/// re-run in a child as uid 65534 when this process is root, and as its own uid
/// otherwise, so the same refusal and the same assertions run on every host.
/// `crates/store/tests/support/mod.rs` carries the full reasoning.
///
/// Runs `body` in a process whose permission bits are enforced.
///
/// `test` is the name the harness knows the caller by: the bare function name
/// in an integration test, the full module path in a unit test.
///
/// # Panics
///
/// When the child cannot start, fails, or did not run exactly one test; and,
/// inside the child, whenever `body` does.
#[cfg(unix)]
#[allow(
    clippy::expect_used,
    reason = "a fixture that cannot start its child must fail the test loudly"
)]
pub(crate) fn where_permission_binds(test: &str, body: impl FnOnce()) {
    use std::os::unix::process::CommandExt as _;
    const CHILD: &str = "BRUTEX_PERMISSION_BINDS_CHILD";
    const NOBODY: u32 = 65_534;
    if std::env::var_os(CHILD).is_some() {
        body();
        return;
    }
    let uid = Some(effective_uid())
        .filter(|&uid| uid != 0)
        .unwrap_or(NOBODY);
    let output = std::process::Command::new(std::env::current_exe().expect("this binary"))
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .uid(uid)
        .output()
        .expect("the child test process starts");
    let (stdout, stderr) = (
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let ran = output.status.success() && stdout.contains("1 passed");
    // Formatted unconditionally, so no line is reached only when the child fails.
    let said = format!("child {test} (uid {uid}) failed or ran no test:\n{stdout}\n{stderr}");
    assert!(ran, "{said}");
}

/// This process's effective uid, read as the owner of a file it just made.
///
/// `std` has no `geteuid` and every crate here denies `unsafe`, so the kernel
/// is asked the portable way: a new file belongs to the uid that created it.
#[cfg(unix)]
#[allow(
    clippy::expect_used,
    reason = "a probe that cannot be made must fail the test loudly"
)]
fn effective_uid() -> u32 {
    use std::os::unix::fs::MetadataExt as _;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let probe =
        std::env::temp_dir().join(format!("brutex-uid-probe-{}-{serial}", std::process::id()));
    let file = std::fs::File::create_new(&probe).expect("a probe file");
    let uid = file.metadata().expect("the probe's own status").uid();
    drop(file);
    let _removed = std::fs::remove_file(&probe);
    uid
}
