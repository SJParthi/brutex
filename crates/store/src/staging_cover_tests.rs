#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: a dead writer's leftover the host will not let the
//! staging door remove is the host's refusal, named for the removal, and the
//! leftover is left as it was.

use std::os::unix::fs::PermissionsExt as _;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

/// **A LEFTOVER THE HOST WILL NOT LET THE DOOR REMOVE IS REFUSED BY THE
/// REMOVAL.** A regular file of one name at the staging name is a dead
/// writer's leftover, and the door removes it before making its own; in a
/// directory closed to writes the removal is refused, named `remove`, and
/// the leftover's bytes are still there. Root removes from a mode-0555
/// directory, so the body runs where the mode bits bind: as `nobody` when
/// this process is root (D-0995).
#[test]
fn a_leftover_the_host_will_not_let_the_door_remove_is_refused_by_the_removal() {
    crate::support::where_permission_binds(
        "staging::cover_tests::a_leftover_the_host_will_not_let_the_door_remove_is_refused_by_the_removal",
        a_leftover_the_host_will_not_let_the_door_remove_is_refused_by_the_removal_body,
    );
}

fn a_leftover_the_host_will_not_let_the_door_remove_is_refused_by_the_removal_body() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "brutex-staging-cover-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    let at = dir.join("2031-05.new.sum");
    fs::write(&at, b"a dead writer's leftover").unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    let refused = create(&at);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let left = fs::read(&at);
    let _cleanup = fs::remove_dir_all(&dir);
    match refused {
        Err(StagingError::Io {
            path,
            action: "remove",
            source,
        }) => {
            assert_eq!(path, at);
            assert_eq!(source.kind(), ErrorKind::PermissionDenied);
        }
        Err(other) => panic!("the removal is refused by name: {other:?}"),
        Ok(_) => panic!("the removal is refused by name, not staged"),
    }
    assert_eq!(left.unwrap(), b"a dead writer's leftover");
}

/// **A LEFTOVER ANOTHER ACTOR REMOVES BETWEEN THE LOOK AND THE REMOVAL IS
/// GONE, NOT A REFUSAL** (D-8561): the door looks at a dead writer's
/// leftover (one name, 24 bytes), and before its own removal another actor
/// removes it (a real change, the race hook's); the removal answers "not
/// found", which leaves the name free as the door wanted, so the file is
/// staged new and the leftover the look saw is reported cleared.
#[test]
fn a_leftover_removed_between_the_look_and_the_removal_is_gone() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "brutex-staging-cover-race-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    let at = dir.join("2031-05.new.sum");
    fs::write(&at, b"a dead writer's leftover").unwrap();
    crate::file::tests::arm_race("staging leftover looked at", |leftover| {
        fs::remove_file(leftover).unwrap();
    });
    let staged = create(&at).expect("a name freed by another actor is staged");
    assert!(!crate::file::tests::race_armed(), "the race ran");
    assert_eq!(staged.cleared, Some(24), "the leftover the look saw");
    assert_eq!(staged.calls, 4, "open, look, removal, open");
    assert_eq!(fs::metadata(&at).unwrap().len(), 0, "a new, empty file");
    drop(staged);
    let _cleanup = fs::remove_dir_all(&dir);
}
