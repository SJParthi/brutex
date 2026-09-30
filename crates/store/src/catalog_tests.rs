#![cfg(test)]
#![allow(clippy::expect_used, clippy::panic)]
//! Unit proofs for the catalog walk's refusal buckets, beside the code they
//! pin so that `cargo mutants` over the library's own tests reaches them.
//! The walk-level proofs on real temporary trees are in
//! `crates/store/tests/catalog.rs`. D-0765 to D-0768.

use super::{Census, Holdings, classify, parse_month, walk};
use std::path::{Path, PathBuf};

/// A private directory under the system temporary root, named for its test.
fn scratch(name: &str) -> PathBuf {
    let mut root = std::env::temp_dir();
    root.push(format!("brutex-catalog-unit-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch root is creatable");
    root
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

#[test]
fn a_locked_directory_is_one_unreadable_entry() {
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
