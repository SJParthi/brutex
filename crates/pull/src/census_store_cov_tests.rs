#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): what the census store's locators answer
//! when the host refuses to measure or to list a directory (a folder whose
//! bits forbid it, run where the permission bits bind, D-0995): refused by
//! name with the host's own kind, never read as absent; and a segment name
//! that is a file is refused as invalid data, no host call having failed.

#![allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]

use std::os::unix::fs::PermissionsExt as _;

use super::*;

#[test]
fn a_census_directory_the_host_will_not_measure_or_list_is_refused_by_its_kind() {
    crate::support::where_permission_binds(
        "ingest::census_store::cov_tests::a_census_directory_the_host_will_not_measure_or_list_is_refused_by_its_kind",
        body,
    );
}

fn body() {
    let root = crate::gdfl_fixtures::scratch("census-cov-perm");
    let shut = |dir: &Path, mode: u32| {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode)).unwrap();
    };
    // A segment the host will not list, and a name beneath it the host will
    // not measure.
    let segment = root.join("segment");
    std::fs::create_dir_all(&segment).unwrap();
    shut(&segment, 0o000);
    let unlisted = current_epoch_in(&segment).unwrap_err();
    assert_eq!(unlisted.kind(), ErrorKind::PermissionDenied);
    assert!(
        unlisted.to_string().contains("could not be listed"),
        "{unlisted}"
    );
    let unmeasured = current_epoch_in(&segment.join("inner")).unwrap_err();
    assert_eq!(unmeasured.kind(), ErrorKind::PermissionDenied);
    assert!(
        unmeasured.to_string().contains("could not be measured"),
        "{unmeasured}"
    );
    shut(&segment, 0o755);
    // A file at a segment's name: refused for what it is.
    let file = root.join("a-file");
    std::fs::write(&file, b"invented").unwrap();
    assert_eq!(
        current_epoch_in(&file).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
    // A store whose version-4 folder will not be searched: where the vendor's
    // census lives cannot be measured, so neither layout is guessed.
    let store = root.join("store");
    let v4 = store.join("manifest").join("v4");
    std::fs::create_dir_all(&v4).unwrap();
    shut(&v4, 0o000);
    let why = locate(&store, Vendor::Gdfl).unwrap_err();
    assert!(why.contains("could not be measured"), "{why}");
    assert!(why.contains("refused rather than guess"), "{why}");
    shut(&v4, 0o755);
}
