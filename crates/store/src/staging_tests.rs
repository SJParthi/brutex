//! Unit tests of `store::staging`: every row of the module's table, each on
//! a directory the test owns. Every byte written is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::io::Write as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{StagingError, create};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A new, empty directory of this test's own.
fn dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-staging-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_free_name_is_created_empty_and_nothing_is_cleared() {
    let dir = dir("free");
    let path = dir.join("a.tmp");
    let mut staged = create(&path).unwrap();
    assert_eq!(staged.cleared, None);
    assert_eq!(fs::metadata(&path).unwrap().len(), 0);
    staged.file.write_all(b"synthetic").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"synthetic");
}

#[test]
fn a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported() {
    let dir = dir("leftover");
    let path = dir.join("a.tmp");
    fs::write(&path, b"half a file").unwrap();
    let before = fs::metadata(&path).unwrap().ino();
    let staged = create(&path).unwrap();
    assert_eq!(staged.cleared, Some(11), "the leftover's bytes are named");
    let after = fs::metadata(&path).unwrap();
    assert_eq!(after.len(), 0, "the new file is empty");
    assert_ne!(after.ino(), before, "it is a new file, not the old one cut");
}

#[test]
fn a_hard_link_at_a_staging_name_is_refused_and_the_file_behind_it_is_untouched() {
    let dir = dir("linked");
    let victim = dir.join("victim.bin");
    let body = b"bytes that must survive".to_vec();
    fs::write(&victim, &body).unwrap();
    let path = dir.join("a.tmp");
    fs::hard_link(&victim, &path).unwrap();

    let refusal = create(&path).unwrap_err();
    assert_eq!(refusal.word(), "staging_linked");
    assert!(
        matches!(&refusal, StagingError::Linked { links: 2, path: p } if *p == path),
        "{refusal}"
    );
    assert_eq!(refusal.path(), path);
    assert_eq!(fs::read(&victim).unwrap(), body, "not one byte changed");
    assert_eq!(
        fs::metadata(&victim).unwrap().nlink(),
        2,
        "the planted name is left for the operator to see"
    );
}

#[test]
fn a_symbolic_link_or_a_directory_at_a_staging_name_is_refused_and_left() {
    let dir = dir("special");
    let victim = dir.join("victim.bin");
    fs::write(&victim, b"kept").unwrap();
    let link = dir.join("link.tmp");
    std::os::unix::fs::symlink(&victim, &link).unwrap();
    let refusal = create(&link).unwrap_err();
    assert_eq!(refusal.word(), "staging_not_regular", "{refusal}");
    assert_eq!(fs::read(&victim).unwrap(), b"kept");
    assert!(fs::symlink_metadata(&link).unwrap().is_symlink());

    let dangling = dir.join("dangling.tmp");
    std::os::unix::fs::symlink(dir.join("nowhere"), &dangling).unwrap();
    assert_eq!(
        create(&dangling).unwrap_err().word(),
        "staging_not_regular",
        "a link to nothing is not created through"
    );
    assert!(!dir.join("nowhere").exists());

    let folder = dir.join("folder.tmp");
    fs::create_dir(&folder).unwrap();
    assert_eq!(create(&folder).unwrap_err().word(), "staging_not_regular");
    assert!(folder.is_dir());
}

#[test]
fn a_missing_directory_is_the_hosts_refusal_by_name() {
    let dir = dir("nodir");
    let path = dir.join("absent").join("a.tmp");
    let refusal = create(&path).unwrap_err();
    assert_eq!(refusal.word(), "io");
    assert!(
        matches!(
            &refusal,
            StagingError::Io {
                action: "create",
                ..
            }
        ),
        "{refusal}"
    );
    assert!(std::error::Error::source(&refusal).is_some());
}
