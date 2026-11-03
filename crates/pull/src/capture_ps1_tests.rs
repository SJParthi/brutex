//! BASE-COV-PULL-PS1 (D-7335): a capture name the host will not make, and an
//! unreadable body whose folder cannot be made. Every body, URL and reason
//! below is invented.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

/// **A NAME THE HOST WILL NOT MAKE IS REFUSED IN THE HOST'S WORDS**, never
/// retried under the next attempt's name as if it already existed: a prefix
/// of 300 bytes puts every candidate past the 255-byte file name limit of
/// both hosts this crate runs on, and the refusal says the file cannot be
/// created, with nothing written. Extremes at once: the longest prefix beside
/// the shortest body, on the first of the sixteen attempts.
#[test]
fn a_name_the_host_will_not_make_is_refused_in_its_words() {
    let dir = crate::gdfl_fixtures::scratch("ps1-capture-long");
    let prefix = "Z".repeat(300);
    let why = write_new_capture_with_stamp(&dir, &prefix, b"x", 7).unwrap_err();
    assert!(why.contains(" cannot be created — "), "{why}");
    assert!(!why.contains("exclusive names"), "{why}");
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        0,
        "nothing written"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// **AN UNREADABLE BODY WHOSE FOLDER CANNOT BE MADE IS NOTED AND NOT KEPT**:
/// with a file where `captures/` goes, `record_unreadable` answers `None`
/// (never an error the failing pull would have to act on) and the slot it
/// took is spent. Run in a process of its own: the per-feed budget is a
/// process-wide counter that other tests in this binary assert on.
#[test]
fn an_unreadable_body_whose_folder_cannot_be_made_is_noted_and_not_kept() {
    crate::support::where_permission_binds(
        "capture::ps1_tests::an_unreadable_body_whose_folder_cannot_be_made_is_noted_and_not_kept",
        an_unreadable_body_whose_folder_cannot_be_made_is_noted_and_not_kept_body,
    );
}

fn an_unreadable_body_whose_folder_cannot_be_made_is_noted_and_not_kept_body() {
    let root = crate::gdfl_fixtures::scratch("ps1-capture-blocked");
    std::fs::write(root.join("captures"), b"a file where the folder goes").unwrap();
    assert_eq!(unread_kept(Feed::Dhan), 0);
    let kept = record_unreadable(
        &root,
        Feed::Dhan,
        "https://zq.invalid/v2",
        "{\"invented\":true}",
        "an invented reason",
    );
    assert_eq!(kept, None);
    assert_eq!(unread_kept(Feed::Dhan), 1, "the slot was taken first");
    assert!(root.join("captures").is_file(), "the file was left alone");
    std::fs::remove_dir_all(root).unwrap();
}

/// **A CAPTURE THE HOST WILL NOT WRITE IS REMOVED AND NAMED IN THE HOST'S
/// WORDS** (the write seam, D-7335): a write refused leaves no file and says
/// why; a refused write whose file is already gone says the same and no
/// second fault; and a refused write whose name now holds a folder, which
/// the cleanup cannot unlink on either host, says both faults. Extremes at
/// once: three refusals at three stamps in one folder, and a cleanup that
/// meets the one thing it cannot remove.
#[test]
fn a_capture_the_host_will_not_write_is_removed_and_named() {
    let dir = crate::gdfl_fixtures::scratch("ps1-capture-unwritten");
    let refused = || Err(std::io::Error::other("an invented refusal"));
    let why = write_new_capture_using(&dir, "zq", b"x", 1, |_, _, _| refused()).unwrap_err();
    assert!(
        why.ends_with(" could not be written and synced — an invented refusal"),
        "{why}"
    );
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0, "removed");
    let why = write_new_capture_using(&dir, "zq", b"x", 2, |_, path, _| {
        std::fs::remove_file(path)?;
        refused()
    })
    .unwrap_err();
    assert!(
        why.ends_with(" could not be written and synced — an invented refusal"),
        "{why}"
    );
    let why = write_new_capture_using(&dir, "zq", b"x", 3, |_, path, _| {
        std::fs::remove_file(path)?;
        std::fs::create_dir(path)?;
        refused()
    })
    .unwrap_err();
    assert!(
        why.contains(
            " could not be written and synced — an invented refusal; its incomplete file \
             also could not be removed — "
        ),
        "{why}"
    );
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        1,
        "the folder stands"
    );
    std::fs::remove_dir_all(dir).unwrap();
}
