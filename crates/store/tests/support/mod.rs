//! Test-only: run one test where the permission bits bind, even as root.
//!
//! # Why
//!
//! A test that closes a file or a directory with `chmod` and expects the host
//! to refuse proves nothing as root. Root bypasses the mode bits
//! (`CAP_DAC_OVERRIDE`), so the call the test expects to fail succeeds, the
//! refusal under test never runs, and the test fails on its own premise. CI and
//! the operator's machine run as an ordinary user; a cloud container runs as
//! root. Skipping there would leave the refusal unexercised on exactly that
//! host, and `CLAUDE.md` §4 bans a test that asserts nothing. D-0995.
//!
//! # How
//!
//! [`where_permission_binds`] re-runs the named test, alone, in a child of this
//! test binary. As root the child is given uid 65534 (`nobody`): `setuid` away
//! from root drops every capability, so the mode bits decide again and the SAME
//! production refusal runs under the SAME assertions an ordinary user sees. As
//! anyone else the child keeps the parent's own uid. Either way the same lines
//! run, so nothing here is reachable only as root.
//!
//! The child sees the marker variable and runs the body itself. The parent
//! asserts the child passed AND that the harness ran exactly one test, because
//! `--exact` with a name that matches nothing exits zero having run nothing.
//! The environment is inherited, so a coverage run's `LLVM_PROFILE_FILE`
//! reaches the child and the body's lines are counted.

/// Runs `body` in a process whose permission bits are enforced.
///
/// `test` is the name the harness knows the caller by: the bare function name
/// in an integration test, the full module path in a unit test.
///
/// # Panics
///
/// When the child cannot start, fails, or did not run exactly one test; and,
/// inside the child, whenever `body` does.
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
