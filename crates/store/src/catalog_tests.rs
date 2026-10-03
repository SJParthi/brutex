#![cfg(test)]
#![allow(clippy::expect_used, clippy::panic)]
//! Unit proofs for the catalog walk's refusal buckets, beside the code they
//! pin so that `cargo mutants` over the library's own tests reaches them.
//! The walk-level proofs on real temporary trees are in
//! `crates/store/tests/catalog.rs`. D-0765 to D-0768.

use super::{CatalogError, Census, Holdings, admit, bars_present, classify, parse_month, walk};
use std::path::{Path, PathBuf};

/// A scratch tree that removes itself when the test that owns it ends, so a
/// run leaves nothing under the system temporary root (found by a review:
/// these trees were removed only before a test, never after).
struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A private directory under the system temporary root, named for its test.
fn scratch(name: &str) -> Scratch {
    let mut root = std::env::temp_dir();
    root.push(format!("brutex-catalog-unit-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch root is creatable");
    Scratch(root)
}

/// An empty file at `root/bars/<rel>`, parents created.
fn put(root: &Path, rel: &str) {
    let full = root.join("bars").join(rel);
    std::fs::create_dir_all(full.parent().expect("a relative path has a parent"))
        .expect("parents are creatable");
    std::fs::write(&full, b"").expect("the file is writable");
}

#[test]
fn each_counting_helper_counts_one_seen_entry_in_its_own_bucket() {
    let mut census = Census::default();
    census.count_unreadable();
    assert_eq!((census.seen, census.unreadable, census.linked), (1, 1, 0));
    census.count_linked();
    assert_eq!((census.seen, census.unreadable, census.linked), (2, 1, 1));
    assert!(census.reconciles(), "{census:?}");
}

#[test]
fn every_new_bucket_is_a_reconciliation_part() {
    for census in [
        Census {
            seen: 1,
            unreadable: 1,
            ..Census::default()
        },
        Census {
            seen: 1,
            linked: 1,
            ..Census::default()
        },
        Census {
            seen: 1,
            non_utf8: 1,
            ..Census::default()
        },
        Census {
            seen: 1,
            not_regular: 1,
            ..Census::default()
        },
    ] {
        assert!(census.reconciles(), "{census:?}");
        let short = Census { seen: 2, ..census };
        assert!(!short.reconciles(), "{short:?}");
    }
}

#[test]
fn a_link_back_to_an_ancestor_is_one_linked_entry() {
    let root = scratch("loop");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    std::os::unix::fs::symlink("../..", root.join("bars/groww/NSE/back"))
        .expect("a symlink is creatable");
    let out = walk(&root).expect("the walk runs");
    let expected = Census {
        seen: 2,
        spot: 1,
        linked: 1,
        ..Census::default()
    };
    assert_eq!(out.census, expected);
    assert_eq!(out.held.len(), 1);
}

/// Runs `body` in a child of this test binary whose permission bits bind:
/// as root the child is uid 65534 (`nobody`), whose `setuid` away from root
/// drops `CAP_DAC_OVERRIDE`, so a mode-000 directory refuses it exactly as it
/// refuses an ordinary user. D-0995 moved the integration tests to
/// `tests/support`'s copy of this and missed these two, so they failed on every
/// root container (audit-20261003 hunt-store-7, D-1526). `test` is the full
/// module path the harness knows the caller by.
fn where_permission_binds(test: &str, body: impl FnOnce()) {
    use std::os::unix::fs::MetadataExt as _;
    use std::os::unix::process::CommandExt as _;
    const CHILD: &str = "BRUTEX_PERMISSION_BINDS_CHILD";
    const NOBODY: u32 = 65_534;
    if std::env::var_os(CHILD).is_some() {
        body();
        return;
    }
    // This process's effective uid, as the owner of a file it just made.
    let probe = scratch("uid-probe");
    let uid = std::fs::metadata(&probe.0)
        .expect("the probe's status")
        .uid();
    drop(probe);
    let uid = Some(uid).filter(|&uid| uid != 0).unwrap_or(NOBODY);
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
    let said = format!("child {test} (uid {uid}) failed or ran no test:\n{stdout}\n{stderr}");
    assert!(ran, "{said}");
}

#[test]
fn a_locked_directory_is_one_unreadable_entry() {
    where_permission_binds(
        "catalog::catalog_tests::a_locked_directory_is_one_unreadable_entry",
        a_locked_directory_is_one_unreadable_entry_body,
    );
}

/// The test above, run where the mode bits bind (D-1526).
fn a_locked_directory_is_one_unreadable_entry_body() {
    use std::os::unix::fs::PermissionsExt;
    let root = scratch("locked");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    put(&root, "groww/NSE/INDEX/BANKNIFTY/1min/2026-08.bin");
    let locked = root.join("bars/groww/NSE/INDEX/BANKNIFTY");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))
        .expect("the fixture directory can be locked");
    let refused = std::fs::read_dir(&locked).is_err();
    let out = walk(&root);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))
        .expect("the fixture directory can be unlocked");
    assert!(
        refused,
        "a process that ignores permissions cannot run this test"
    );
    let out = out.expect("one locked branch does not fail the walk");
    let expected = Census {
        seen: 2,
        spot: 1,
        unreadable: 1,
        ..Census::default()
    };
    assert_eq!(out.census, expected);
}

#[test]
fn a_signed_or_non_digit_month_field_does_not_parse() {
    for stem in ["2026-+8", "+026-08", "2026- 8", "2026-0x"] {
        assert!(parse_month(stem).is_err(), "{stem} must not parse");
    }
    let month = parse_month("2026-08").expect("the canonical spelling parses");
    assert_eq!(month.to_string(), "2026-08");
}

#[test]
fn a_non_utf8_component_is_counted_and_never_dropped() {
    use std::os::unix::ffi::OsStrExt;
    let bars = Path::new("/s/bars");
    let path = bars
        .join(std::ffi::OsStr::from_bytes(b"\xff"))
        .join("groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    let mut out = Holdings::default();
    classify(bars, &path, &mut out);
    let expected = Census {
        non_utf8: 1,
        ..Census::default()
    };
    assert_eq!(out.census, expected, "{:?}", out.held);
    assert!(out.held.is_empty());

    let mut control = Holdings::default();
    classify(
        bars,
        &bars.join("groww/NSE/INDEX/NIFTY/1min/2026-08.bin"),
        &mut control,
    );
    assert_eq!(
        control.census.spot, 1,
        "the six-level path without that component is a spot month"
    );
}

#[test]
fn only_an_absent_bars_is_the_empty_store_and_every_other_non_directory_is_refused() {
    where_permission_binds(
        "catalog::catalog_tests::only_an_absent_bars_is_the_empty_store_and_every_other_non_directory_is_refused",
        only_an_absent_bars_is_the_empty_store_and_every_other_non_directory_is_refused_body,
    );
}

/// The test above, run where the mode bits bind (D-1526).
fn only_an_absent_bars_is_the_empty_store_and_every_other_non_directory_is_refused_body() {
    use std::os::unix::fs::PermissionsExt;
    let root = scratch("probe");
    let bars = root.join("bars");
    assert_eq!(bars_present(&bars), Ok(false), "nothing there: empty store");

    std::fs::write(&bars, b"x").expect("writable");
    assert_eq!(
        bars_present(&bars),
        Err(CatalogError::BarsUnreadable {
            because: "it is not a directory".to_owned()
        })
    );
    std::fs::remove_file(&bars).expect("removable");

    std::os::unix::fs::symlink(root.join("unmounted"), &bars).expect("a symlink is creatable");
    let Err(CatalogError::BarsUnreadable { because }) = bars_present(&bars) else {
        panic!("a dangling `bars` link is refused");
    };
    assert!(because.contains("os error 2"), "{because}");
    std::fs::remove_file(&bars).expect("removable");

    std::fs::create_dir(&bars).expect("creatable");
    assert_eq!(bars_present(&bars), Ok(true));

    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o000))
        .expect("the fixture root can be locked");
    let locked = bars_present(&bars);
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755))
        .expect("the fixture root can be unlocked");
    let Err(CatalogError::BarsUnreadable { because }) = locked else {
        panic!("an unsearchable root is refused, not empty: {locked:?}");
    };
    assert!(because.contains("os error 13"), "{because}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The type of `/dev/null`, a character device: an entry that is neither a
/// regular file, a directory nor a symbolic link, without a FIFO or socket
/// fixture.
fn device_kind() -> std::fs::FileType {
    std::fs::symlink_metadata("/dev/null")
        .expect("/dev/null exists")
        .file_type()
}

#[test]
fn an_entry_the_os_could_not_read_is_one_unreadable_entry() {
    let bars = Path::new("/s/bars");
    let mut stack = Vec::new();
    let mut out = Holdings::default();
    admit(
        Err(std::io::Error::other("injected entry failure")),
        bars,
        &mut stack,
        &mut out,
    );
    let expected = Census {
        seen: 1,
        unreadable: 1,
        ..Census::default()
    };
    assert_eq!(out.census, expected);
    assert!(stack.is_empty() && out.held.is_empty());
}

#[test]
fn an_entry_that_is_not_a_regular_file_is_never_classified_as_a_month() {
    let kind = device_kind();
    assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
    let bars = Path::new("/s/bars");
    let mut stack = Vec::new();
    let mut out = Holdings::default();
    admit(
        Ok((kind, bars.join("groww/NSE/INDEX/NIFTY/1min/2026-09.bin"))),
        bars,
        &mut stack,
        &mut out,
    );
    let expected = Census {
        seen: 1,
        not_regular: 1,
        ..Census::default()
    };
    assert_eq!(out.census, expected, "{:?}", out.held);
    assert!(stack.is_empty() && out.held.is_empty());
}

#[test]
fn each_entry_kind_goes_to_exactly_one_place() {
    let root = scratch("kinds");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    let link = root.join("link");
    std::os::unix::fs::symlink(&root, &link).expect("a symlink is creatable");
    let kind_of = |p: &Path| {
        std::fs::symlink_metadata(p)
            .expect("the fixture exists")
            .file_type()
    };
    let bars = root.join("bars");
    let month = bars.join("groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    let dir = bars.join("groww");
    let mut stack = Vec::new();
    let mut out = Holdings::default();

    admit(
        Ok((kind_of(&dir), dir.clone())),
        &bars,
        &mut stack,
        &mut out,
    );
    assert_eq!(stack, vec![dir], "a directory is descended, not counted");
    assert_eq!(out.census, Census::default());

    admit(
        Ok((kind_of(&link), link.clone())),
        &bars,
        &mut stack,
        &mut out,
    );
    assert_eq!(stack.len(), 1, "a link is never descended");
    assert_eq!((out.census.seen, out.census.linked), (1, 1));

    admit(Ok((kind_of(&month), month)), &bars, &mut stack, &mut out);
    assert_eq!((out.census.seen, out.census.spot), (2, 1));
    assert_eq!(out.held.len(), 1);
    assert!(out.census.reconciles(), "{:?}", out.census);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_unoffered_report_names_every_unwalked_bucket_and_is_silent_otherwise() {
    let quiet = Census {
        seen: 3,
        spot: 1,
        with_contract: 1,
        wrong_depth: 1,
        ..Census::default()
    };
    assert_eq!(
        quiet.unoffered_report(),
        "",
        "a filed-in-full census prints nothing"
    );

    let all = Census {
        seen: 10,
        unreadable: 1,
        linked: 2,
        non_utf8: 3,
        not_regular: 4,
        ..Census::default()
    };
    assert_eq!(
        all.unoffered_report(),
        "NOT OFFERED: below bars/ the catalog could not read 1 director(ies) or entr(ies), \
         did not follow 2 symbolic link(s), and found 3 path(s) that are not UTF-8 and 4 \
         entr(ies) that are not a regular file. None of them was offered to any sweep; what \
         an unreadable directory or a link holds is unknown.\n"
    );
    for one in [
        Census {
            seen: 1,
            unreadable: 1,
            ..Census::default()
        },
        Census {
            seen: 1,
            linked: 1,
            ..Census::default()
        },
        Census {
            seen: 1,
            non_utf8: 1,
            ..Census::default()
        },
        Census {
            seen: 1,
            not_regular: 1,
            ..Census::default()
        },
    ] {
        let text = one.unoffered_report();
        assert!(text.starts_with("NOT OFFERED: "), "{one:?}: {text}");
        assert!(!text.contains("DOES NOT RECONCILE"), "{one:?}: {text}");
    }

    let lost = Census {
        seen: 5,
        spot: 3,
        ..Census::default()
    };
    assert_eq!(
        lost.unoffered_report(),
        "CATALOG CENSUS DOES NOT RECONCILE: 5 entries seen, 3 filed. An entry has been lost, \
         which is the silent shortfall CLAUDE.md §4 bans.\n"
    );
}

#[test]
fn a_scratch_tree_is_removed_when_its_test_ends() {
    let root = scratch("self-removing");
    put(&root, "groww/NSE/INDEX/NIFTY/1min/2026-08.bin");
    let path = root.to_path_buf();
    assert!(path.join("bars").is_dir(), "premise: the tree was built");
    drop(root);
    assert!(
        std::fs::symlink_metadata(&path).is_err(),
        "{} was left behind",
        path.display()
    );
}
