//! Mutation-survivor tests for `telemetry::sink`'s public surface.
//!
//! Each test here kills a mutant `cargo mutants` left alive in
//! `crates/telemetry/src/sink.rs` (cloud session 2026-10-16, report
//! `reports/cloud-mut-store-20261016.md`). Its doc comment names the mutant
//! and the behaviour that mutant breaks; the assertion is that behaviour.
//! Private items are proved in `sink.rs`'s own `cloud_mutant_tests` module.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::path::Path;

use telemetry::{FileTarget, Target as _};

/// **A FILE TARGET'S `sync` IS THE HOST'S FSYNC, ERRORS INCLUDED.**
///
/// Kills `replace <impl Target for FileTarget>::sync -> std::io::Result<()>
/// with Ok(())`: a sync that answers `Ok` without asking the host reports
/// every event durable when nothing was made so. No regular file on a working
/// disk refuses `fsync`, so the refusal is the host's own: Linux answers
/// `fsync` on a character device with `EINVAL` ("fd is bound to a special
/// file which does not support synchronization", fsync(2)), and `/dev/null`
/// opens for appending without being created or changed. The control beside
/// it is a regular file, whose sync succeeds, so the refusal is the device's
/// and not the target's.
#[cfg(target_os = "linux")]
#[test]
fn a_file_target_passes_the_hosts_fsync_refusal_through() {
    const EINVAL: i32 = 22;
    let device = FileTarget::open(Path::new("/dev/null")).expect("/dev/null opens to append");
    let refused = device
        .sync()
        .expect_err("fsync of a character device is refused by the host");
    assert_eq!(refused.raw_os_error(), Some(EINVAL), "{refused}");

    let dir = std::env::temp_dir().join(format!(
        "brutex-telemetry-cloud-sync-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let mut regular = FileTarget::open(&dir.join("events.ndjson")).expect("a regular file");
    regular.append(b"{}\n").expect("one line lands");
    regular.sync().expect("a regular file syncs");
    let _ignored = std::fs::remove_dir_all(&dir);
}

/// **THE AUDIT STREAM ROLLS AT THE DECIDED 64 MIB AND HOLDS AT MOST 1 GIB.**
///
/// Kills both `replace * with +` mutants of
/// `AUDIT_MAX_FILE_BYTES = 64 * 1024 * 1024` (to 1,048,640 and to 66,560
/// bytes): either would roll the durable
/// stream thousands of times sooner than D-3341 item 3 decided and shrink the
/// history its 16 files hold from 1 GiB to under 17 MiB, with nothing saying
/// so. The bound and the ceiling the constants' own documentation states are
/// asserted as numbers.
#[test]
fn the_audit_stream_is_bounded_at_the_decided_64_mib_and_1_gib() {
    assert_eq!(telemetry::AUDIT_MAX_FILE_BYTES, 67_108_864, "64 MiB");
    assert_eq!(telemetry::AUDIT_KEEP_FILES, 16);
    assert_eq!(
        telemetry::AUDIT_MAX_FILE_BYTES * u64::from(telemetry::AUDIT_KEEP_FILES),
        1_073_741_824,
        "1 GiB while rotation works"
    );
}
