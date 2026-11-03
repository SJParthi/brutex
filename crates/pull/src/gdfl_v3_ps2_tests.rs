#![cfg(test)]
//! BASE-COV-PULL-PS2 (fork B): two arms of the v3 ledger check reached by a
//! direct call, since `check` never displays a missing read (it answers the
//! missing manifest and marker by name before) and never meets a path of
//! another month in a ledger the test can build cheaply: a missing capped
//! read says `missing`, and a manifest path maps to an entry only under its
//! own month and a slash. Every name is invented.

use super::*;

/// Each capped-read fault in its own words.
#[test]
fn a_capped_read_says_each_fault_in_its_words() {
    assert_eq!(Capped::Missing.to_string(), "missing");
    assert_eq!(Capped::Over(64).to_string(), "over 64 bytes");
    assert_eq!(
        Capped::Unreadable(std::io::ErrorKind::PermissionDenied).to_string(),
        "unreadable (permission denied)"
    );
}

/// A manifest path maps to a listing entry only under the entry's own month
/// and a slash, every backslash of the entry read as a slash.
#[test]
fn a_manifest_path_maps_only_under_its_own_month_and_a_slash() {
    assert!(maps_to(
        "2019-01",
        "Options\\A.NFO.csv",
        "2019-01/Options/A.NFO.csv"
    ));
    assert!(!maps_to(
        "2019-01",
        "Options\\A.NFO.csv",
        "2019-02/Options/A.NFO.csv"
    ));
    assert!(!maps_to(
        "2019-01",
        "Options\\A.NFO.csv",
        "2019-01Options/A.NFO.csv"
    ));
    assert!(!maps_to("2019-01", "A.csv", "2019-0"));
    assert!(!maps_to("2019-01", "A.csv", ""));
    assert!(!maps_to("2019-01", "A.csv", "2019-01/A.csvx"));
    assert!(!maps_to("2019-01", "A.csv", "2019-01/B.csv"));
    assert!(maps_to("", "A.csv", "/A.csv"), "an empty month and a slash");
}
