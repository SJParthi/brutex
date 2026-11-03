//! BASE-COV-PULL-PS2 (D-7336): THE ALERT QUEUE'S WALK SEAM AS EVERY BUILD BUT
//! ITS OWN UNIT TESTS COMPILES IT.
//!
//! `View::read` calls `seam_between_reads`, whose `#[cfg(not(test))]` twin
//! does nothing; the crate's unit tests compile only the seam, so this binary,
//! which links the library as `api` and `cli` do, is the one that runs the
//! twin. A settled page reads the records after it through that read, and an
//! acknowledgement among them takes its alert off the page.
//!
//! Every key, text and clock below is invented.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::PathBuf;

use pull::alerts::{self, NewAlert, Put, Scope};

/// An invented clock: 2025-10-09 08:53:20 UTC.
const NOW: u64 = 1_760_000_000;

/// A store root of this test's own, made fresh for each test.
fn root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-alerts-ps2-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    ));
    fs::create_dir_all(&root).expect("a scratch store root");
    root
}

#[test]
fn a_settled_page_reads_the_records_after_it_through_the_shipped_seam() {
    let root = root("SETTLED");
    for (at, key) in (0_u64..).zip(["k1", "k2", "k3"]) {
        let alert = NewAlert {
            kind: 6,
            key,
            day: 18_682,
            run: 4_242,
            text: "invented",
        };
        let put = alerts::put_at(&root, &alert, NOW + at).expect("a put");
        assert!(matches!(put, Put::Written { .. }), "{put:?}");
    }
    let open = alerts::page_settled(&root, 2).expect("a settled page");
    assert_eq!(
        open.alerts
            .iter()
            .map(|alert| alert.ordinal)
            .collect::<Vec<_>>(),
        [1],
        "the page before ordinal 2 holds alert 1, open"
    );
    let acked = alerts::ack_at(&root, 1, "cli", Scope::Whole, NOW + 9).expect("acknowledged");
    assert_eq!(acked.ordinal, 1);
    let settled = alerts::page_settled(&root, 2).expect("a settled page");
    assert!(
        settled.alerts.is_empty(),
        "a later record acknowledges alert 1: {:?}",
        settled.alerts
    );
    let _ = fs::remove_dir_all(&root);
}
