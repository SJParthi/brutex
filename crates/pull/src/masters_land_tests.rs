#![cfg(test)]
//! BASE-COV-PULL PS3 (D-7337): every refusal and uncertain state of
//! [`land`]'s write path driven to its named answer, on synthetic masters in
//! scratch folders: the masters folder that cannot be made, a lock that cannot
//! be opened, a temporary that cannot be opened, a write, a sync or a rename
//! the host refuses (planted, because no portable host refuses them on
//! demand), a cleanup that refuses or finds nothing, a folder whose sync is
//! refused after the rename (the uncertain state), and an unchanged master
//! whose touch is refused (the full replacement runs instead). Every byte is
//! invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::fmt::Write as _;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

use super::*;

/// A fresh scratch folder of this process, named by `tag`.
fn scratch(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("brutex-masters-land-{tag}-{}", std::process::id()));
    let _gone = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A source with no vendor and a plain CSV shape: only the shape guards apply.
fn plain_source(file: &'static str) -> Source {
    Source {
        shape: Shape::Csv,
        vendor: None,
        file,
        url: "https://zq.invalid/master.csv",
        mirrors: &[],
        prime: None,
        needs_token: false,
    }
}

/// A synthetic CSV past the byte floor, every row naming `mark`.
fn body(mark: &str) -> String {
    let mut out = String::from("zq_name,zq_value\n");
    let mut n = 0_u32;
    while out.len() <= MIN_BODY_BYTES {
        let _ = writeln!(out, "ZZIDX {mark} {n},7");
        n += 1;
    }
    out
}

fn temporary_of(dir: &Path, source: &Source) -> PathBuf {
    dir.join(format!(".{}.partial", source.file))
}

fn refused(landed: Landed) -> String {
    let Landed::Refused(why) = landed else {
        panic!("refused, got {landed:?}");
    };
    why
}

/// A write the host refuses after the temporary opened: refused by name, the
/// temporary removed, and the master never written.
#[test]
fn a_refused_write_is_named_and_leaves_no_partial_and_no_master() {
    let dir = scratch("write");
    let source = plain_source("zq-write.csv");
    inject::plant(
        &temporary_of(&dir, &source),
        Step::Write,
        std::io::ErrorKind::Other,
    );
    let why = refused(land(&dir, &source, &body("A")));
    assert!(why.contains("could not be written"), "{why}");
    assert!(why.contains("refused by the test's plant"), "{why}");
    assert!(!why.contains("; cleanup of"), "the cleanup worked: {why}");
    assert!(!temporary_of(&dir, &source).exists());
    assert!(!path_of(&dir, &source).exists());
}

/// A sync the host refuses: refused by name, nothing left, nothing published.
#[test]
fn a_refused_sync_is_named_and_publishes_nothing() {
    let dir = scratch("sync");
    let source = plain_source("zq-sync.csv");
    inject::plant(
        &temporary_of(&dir, &source),
        Step::Sync,
        std::io::ErrorKind::Other,
    );
    let why = refused(land(&dir, &source, &body("A")));
    assert!(why.contains("could not be synced"), "{why}");
    assert!(!temporary_of(&dir, &source).exists());
    assert!(!path_of(&dir, &source).exists());
}

/// A rename the host refuses over a held master: refused naming both paths,
/// the temporary removed, and the held master untouched, byte for byte.
#[test]
fn a_refused_rename_keeps_the_held_master_whole() {
    let dir = scratch("rename");
    let source = plain_source("zq-rename.csv");
    let first = body("A");
    assert!(land(&dir, &source, &first).is_written());
    inject::plant(
        &temporary_of(&dir, &source),
        Step::Rename,
        std::io::ErrorKind::Other,
    );
    let why = refused(land(&dir, &source, &body("B")));
    assert!(why.contains("could not replace"), "{why}");
    assert!(
        why.contains(&path_of(&dir, &source).display().to_string()),
        "{why}"
    );
    assert!(!temporary_of(&dir, &source).exists());
    assert_eq!(
        std::fs::read_to_string(path_of(&dir, &source)).unwrap(),
        first
    );
}

/// Two failures at once: the write refused AND its cleanup refused. Both are
/// named, the cleanup's after the write's, and neither hides the other.
#[test]
fn a_cleanup_that_also_fails_is_named_after_the_failure_it_followed() {
    let dir = scratch("cleanup");
    let source = plain_source("zq-cleanup.csv");
    let temporary = temporary_of(&dir, &source);
    inject::plant(&temporary, Step::Write, std::io::ErrorKind::Other);
    inject::plant(
        &temporary,
        Step::Remove,
        std::io::ErrorKind::PermissionDenied,
    );
    let why = refused(land(&dir, &source, &body("A")));
    let wrote = why.find("could not be written").unwrap();
    let cleanup = why.find("; cleanup of").unwrap();
    assert!(wrote < cleanup, "{why}");
    assert!(
        why.contains(&format!("cleanup of {} also failed", temporary.display())),
        "{why}"
    );
    assert!(!path_of(&dir, &source).exists());
}

/// Two failures at once where the second finds nothing: a sync refused and a
/// cleanup that finds the temporary already gone. Gone is what cleanup was
/// for, so only the sync is named.
#[test]
fn a_cleanup_that_finds_nothing_names_only_the_failure() {
    let dir = scratch("cleanup-gone");
    let source = plain_source("zq-gone.csv");
    let temporary = temporary_of(&dir, &source);
    inject::plant(&temporary, Step::Sync, std::io::ErrorKind::Other);
    inject::plant(&temporary, Step::Remove, std::io::ErrorKind::NotFound);
    let why = refused(land(&dir, &source, &body("A")));
    assert!(why.contains("could not be synced"), "{why}");
    assert!(!why.contains("; cleanup of"), "{why}");
    assert!(!path_of(&dir, &source).exists());
}

/// A masters folder that cannot be made (a file holds its parent's name) is
/// refused by name before any lock or byte.
#[test]
fn a_masters_folder_that_cannot_be_made_is_refused_by_name() {
    let root = scratch("no-dir");
    let blocker = root.join("blocker");
    std::fs::write(&blocker, b"a file, not a folder").unwrap();
    let dir = blocker.join("masters");
    let why = refused(land(&dir, &plain_source("zq-nodir.csv"), &body("A")));
    assert!(why.contains("the masters directory"), "{why}");
    assert!(why.contains("cannot be created"), "{why}");
}

/// A folder at the lock's name: the lock cannot be opened, so nothing is
/// compared or written.
#[test]
fn a_lock_that_cannot_be_opened_refuses_the_landing() {
    let dir = scratch("lock");
    let source = plain_source("zq-lock.csv");
    std::fs::create_dir_all(lock_path_of(&dir, &source)).unwrap();
    let why = refused(land(&dir, &source, &body("A")));
    assert!(why.contains("could not be opened for locking"), "{why}");
    assert!(!path_of(&dir, &source).exists());
}

/// A folder at the temporary's name: the temporary cannot be opened, so the
/// replacement is refused and the held master stays.
#[test]
fn a_temporary_that_cannot_be_opened_refuses_the_replacement() {
    let dir = scratch("partial-dir");
    let source = plain_source("zq-partial.csv");
    let first = body("A");
    assert!(land(&dir, &source, &first).is_written());
    std::fs::create_dir_all(temporary_of(&dir, &source)).unwrap();
    let why = refused(land(&dir, &source, &body("B")));
    assert!(why.contains("could not be opened"), "{why}");
    assert_eq!(
        std::fs::read_to_string(path_of(&dir, &source)).unwrap(),
        first
    );
}

/// The rename lands and then the folder's own sync is refused (the folder may
/// be entered and written, not read): the bytes are visible and the answer is
/// the uncertain state, never `Written` and never `Refused`. Run where the
/// permission bits bind (D-0995): as root a folder of mode 0o300 opens anyway.
#[test]
fn a_folder_that_will_not_sync_after_the_rename_is_uncertain() {
    crate::support::where_permission_binds(
        "masters::land_tests::a_folder_that_will_not_sync_after_the_rename_is_uncertain",
        a_folder_that_will_not_sync_after_the_rename_is_uncertain_body,
    );
}

fn a_folder_that_will_not_sync_after_the_rename_is_uncertain_body() {
    let dir = scratch("uncertain");
    let source = plain_source("zq-uncertain.csv");
    let fresh = body("A");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o300)).unwrap();
    let landed = land(&dir, &source, &fresh);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let Landed::Uncertain {
        bytes,
        changed,
        why,
    } = landed
    else {
        panic!("uncertain, got {landed:?}");
    };
    assert_eq!(bytes, fresh.len());
    assert!(changed, "nothing was held before");
    assert!(why.contains("could not be synced"), "{why}");
    assert!(why.contains("UNVERIFIED"), "{why}");
    assert_eq!(
        std::fs::read_to_string(path_of(&dir, &source)).unwrap(),
        fresh
    );
}

/// Two extremes at once: the same bytes again (nothing to replace) AND a
/// master the writer may not open to touch (mode 0o444). The touch is refused,
/// so the full replacement runs instead: a new inode, the same bytes, and
/// `changed` still false. Run where the permission bits bind (D-0995).
#[test]
fn an_unchanged_master_that_cannot_be_touched_is_replaced_whole() {
    crate::support::where_permission_binds(
        "masters::land_tests::an_unchanged_master_that_cannot_be_touched_is_replaced_whole",
        an_unchanged_master_that_cannot_be_touched_is_replaced_whole_body,
    );
}

fn an_unchanged_master_that_cannot_be_touched_is_replaced_whole_body() {
    let dir = scratch("untouchable");
    let source = plain_source("zq-untouchable.csv");
    let same = body("A");
    assert!(land(&dir, &source, &same).is_written());
    let target = path_of(&dir, &source);
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444)).unwrap();
    let before = std::fs::metadata(&target).unwrap().ino();
    let landed = land(&dir, &source, &same);
    let Landed::Written { changed, bytes } = landed else {
        panic!("written, got {landed:?}");
    };
    assert!(!changed, "the same bytes are no change");
    assert_eq!(bytes, same.len());
    assert_ne!(
        std::fs::metadata(&target).unwrap().ino(),
        before,
        "the touch was refused, so the master was replaced whole"
    );
    assert_eq!(std::fs::read_to_string(&target).unwrap(), same);
}

/// A body opening with `[` (a JSON array, full of commas) is refused on its
/// first byte, like `{` and `<`.
#[test]
fn a_body_opening_with_a_json_array_is_refused_on_its_first_byte() {
    let dir = scratch("array");
    let source = plain_source("zq-array.csv");
    let mut array = String::from("[\"ZZIDX ONE\",\"ZZIDX TWO\"");
    while array.len() <= MIN_BODY_BYTES {
        array.push_str(",\"ZZIDX MORE\"");
    }
    array.push(']');
    let why = refused(land(&dir, &source, &array));
    assert!(why.contains("JSON or markup"), "{why}");
    assert!(why.contains("opening with `[`"), "{why}");
    assert!(!path_of(&dir, &source).exists());
}

/// Out of the box: an index list whose every category is a list, one element
/// of which is a number. No category is skipped, the number is, every name
/// still converts, and the skip is still said: one `Warn` counting no
/// category and one element, in the category that held it. Run in a process
/// of its own (`where_permission_binds`), the one sink install there.
#[test]
fn a_number_among_the_names_is_skipped_said_and_the_names_convert() {
    crate::support::where_permission_binds(
        "masters::land_tests::a_number_among_the_names_is_skipped_said_and_the_names_convert",
        a_number_among_the_names_is_skipped_said_and_the_names_convert_body,
    );
}

fn a_number_among_the_names_is_skipped_said_and_the_names_convert_body() {
    let home = scratch("index-skip-sink");
    let sink =
        telemetry::install(&telemetry::Config::new(&home).with_min_level(telemetry::Level::Trace))
            .expect("the only install in this process");
    let csv = nse_index_csv(r#"{"ZZ Broad":["ZZIDX ONE",7,"ZZIDX TWO"]}"#).unwrap();
    assert_eq!(
        csv,
        "index_name,category\nZZIDX ONE,ZZ Broad\nZZIDX TWO,ZZ Broad\n"
    );
    let records = telemetry::tail(
        &home,
        sink.keep_files(),
        &telemetry::Query::last(telemetry::MAX_LIMIT).from_target("pull.masters"),
    )
    .records;
    let said: Vec<_> = records
        .iter()
        .filter(|record| record.message == "index entries skipped")
        .map(|record| {
            (
                record
                    .field("categories")
                    .and_then(telemetry::OwnedValue::as_u64),
                record
                    .field("elements")
                    .and_then(telemetry::OwnedValue::as_u64),
                record
                    .field("category")
                    .and_then(telemetry::OwnedValue::as_str)
                    .map(str::to_owned),
            )
        })
        .collect();
    assert_eq!(said, [(Some(0), Some(1), Some("ZZ Broad".to_owned()))]);
}

/// Two sources with refusals planted at once do not cross: the second
/// source's landing passes the first's plant by, and each source meets only
/// its own (the sync of one, the rename of the other). A lookup that ignored
/// the path would hand the second source the first one's sync.
#[test]
fn plants_for_two_sources_at_once_meet_only_their_own() {
    let dir = scratch("two-sources");
    let (one, two) = (plain_source("zq-one.csv"), plain_source("zq-two.csv"));
    inject::plant(
        &temporary_of(&dir, &one),
        Step::Sync,
        std::io::ErrorKind::Other,
    );
    inject::plant(
        &temporary_of(&dir, &two),
        Step::Rename,
        std::io::ErrorKind::Other,
    );
    let second = refused(land(&dir, &two, &body("B")));
    assert!(second.contains("could not replace"), "{second}");
    assert!(second.contains("zq-two.csv"), "{second}");
    let first = refused(land(&dir, &one, &body("A")));
    assert!(first.contains("could not be synced"), "{first}");
    assert!(first.contains("zq-one.csv"), "{first}");
    assert!(!path_of(&dir, &one).exists());
    assert!(!path_of(&dir, &two).exists());
}
