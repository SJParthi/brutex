#![cfg(test)]
//! DATA-PATH FOLLOW-UP, API: the rolling-option walk's receipt keeps one
//! reason per kind of failure, the rule D-3124 gave the pricing reasons
//! (D-3127).
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use super::*;

/// One run's refusal, as `roll_one` words it: the run's label, which carries
/// the strike offset and the chunk's dates, then the cause.
fn refused(offset: usize, cause: &str) -> String {
    format!(
        "NIFTY ATM+{offset} CALL 2025-07-0{} weekly: {cause}",
        offset % 9 + 1
    )
}

/// A run that partly failed: `Ok`, with its failures inside the counts.
fn partly(offset: usize, cause: &str) -> Rolled {
    Rolled {
        rows_read: 375,
        stored: 375,
        failed: 1,
        why: vec![refused(offset, cause)],
        ..Rolled::default()
    }
}

const HELD: &str = "the month file is locked by another writer";
const FULL: &str = "no space left on device (os error 28)";

#[test]
fn a_walk_names_a_new_kind_of_run_failure_after_many_runs_of_one_kind() {
    // ERR ARM: twenty runs refused for one cause, then one for another.
    let mut walk = Rolled::default();
    for offset in 0..20 {
        assert!(!walk.absorb(Err(refused(offset, HELD))));
    }
    assert!(!walk.absorb(Err(refused(20, FULL))));
    assert_eq!(walk.failed, 21, "every refused run is counted");
    assert_eq!(
        walk.why,
        vec![refused(0, HELD), refused(20, FULL)],
        "one sentence per kind, the first of each kept verbatim"
    );

    // OK ARM: the same through runs that partly failed.
    let mut walk = Rolled::default();
    for offset in 0..20 {
        assert!(!walk.absorb(Ok(partly(offset, HELD))));
    }
    assert!(!walk.absorb(Ok(partly(20, FULL))));
    assert_eq!(walk.failed, 21);
    assert_eq!(walk.stored, 21 * 375);
    assert_eq!(walk.why, vec![refused(0, HELD), refused(20, FULL)]);

    // THE CAP STILL HOLDS: more kinds than slots keeps the first five kinds.
    let mut walk = Rolled::default();
    let kinds: Vec<String> = (0..7)
        .map(|kind| format!("cause {}", "x".repeat(kind + 1)))
        .collect();
    for (offset, kind) in kinds.iter().enumerate() {
        walk.absorb(Err(refused(offset, kind)));
        walk.absorb(Ok(partly(offset, kind)));
    }
    assert_eq!(walk.failed, 14);
    assert_eq!(walk.why.len(), pull::pricing::REASONS_KEPT);
    for (offset, (kept, kind)) in walk.why.iter().zip(&kinds).enumerate() {
        assert_eq!(kept, &refused(offset, kind));
    }
}

#[test]
fn a_credential_stop_is_still_counted_and_its_marker_never_kept() {
    let mut walk = Rolled::default();
    let dead = format!(
        "{CREDENTIAL_DEAD}{}",
        refused(1, "the vendor refused the token")
    );
    assert!(
        walk.absorb(Err(dead)),
        "the stop is still reported to the walk"
    );
    assert!(!walk.absorb(Err(refused(2, "the vendor refused the token"))));
    assert_eq!(walk.failed, 2);
    assert_eq!(walk.why, vec![refused(1, "the vendor refused the token")]);
}
