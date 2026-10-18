//! THE ALERT QUEUE'S TEST SEAMS ARE SHUT IN EVERY BUILD BUT ITS OWN UNIT
//! TESTS (GF-1).
//!
//! `pull::alerts` has two seams: `FULL_WRITES`, a write that meets a full
//! disk, and `REPLACEMENT_STOPS_AFTER`, a replacement stopped after a step as
//! a kill would stop it. Each is read by a `#[cfg(test)]` function whose
//! `#[cfg(not(test))]` twin answers `false`. The crate's unit tests compile
//! only the first, so the round's mutation run found both twins mutated to
//! `true` surviving (`seam_says_full`, `seam_stops_after`): no test compiled
//! the answer a production build gives and used it. This binary links the
//! library as `api` and `cli` do, so the twins are what runs here.
//!
//! Every key, text and clock below is invented.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;

use pull::alerts::{self, Names, NewAlert, Put, Replaced, Reserve};

/// An invented clock: 2025-10-09 08:53:20 UTC.
const NOW: u64 = 1_760_000_000;

/// A store root of this test's own, made fresh for each test.
fn root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-alerts-seams-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    ));
    fs::create_dir_all(&root).expect("a scratch store root");
    root
}

fn alert(key: &str) -> NewAlert<'_> {
    NewAlert {
        kind: 6,
        key,
        day: 0,
        run: 0,
        text: "an invented gap of this test",
    }
}

fn written(put: Put) -> (u32, Reserve) {
    match put {
        Put::Written {
            ordinal, reserve, ..
        } => (ordinal, reserve),
        Put::Duplicate { ordinal } => panic!("a fresh key was a duplicate of #{ordinal}"),
    }
}

/// With `seam_says_full` answering `true`, the first write of every put met
/// a full disk, spent the reserve and failed again: no put landed.
#[test]
fn a_put_in_a_production_build_lands_on_its_first_write_and_keeps_its_reserve() {
    let root = root("PUT");
    let names = Names::under(&root);
    for (want, key) in [(1, "GF1-SEAM-A"), (2, "GF1-SEAM-B"), (3, "GF1-SEAM-C")] {
        let put = alerts::put_at(&root, &alert(key), NOW).expect("a put on a disk with room");
        assert_eq!(written(put), (want, Reserve::Held), "{key}");
    }
    assert_eq!(
        fs::metadata(&names.reserve)
            .expect("the reserve is in place")
            .len(),
        alerts::RESERVE_RECORDS * alerts::RECORD_LEN,
        "no write met a full disk, so nothing was given up"
    );
    let page = alerts::window(&root).expect("the queue reads back");
    assert_eq!((page.records, page.alerts.len()), (3, 3));
}

/// With `seam_stops_after` answering `true`, every replacement stopped after
/// its first step with "stopped after step 1 by the test seam".
#[test]
fn a_replacement_in_a_production_build_runs_every_step() {
    let root = root("REPLACE");
    let names = Names::under(&root);
    for key in [
        "GF1-SEAM-1",
        "GF1-SEAM-2",
        "GF1-SEAM-3",
        "GF1-SEAM-4",
        "GF1-SEAM-5",
    ] {
        let _landed = written(alerts::put_at(&root, &alert(key), NOW).expect("a put"));
    }
    // Two failed CRCs at the tail: a queue no writer may append to.
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&names.queue)
        .expect("the queue");
    for ordinal in [4_u64, 5] {
        let at = alerts::offset_of(ordinal) + 60;
        let mut one = [0_u8; 1];
        file.read_exact_at(&mut one, at)
            .expect("a byte of the record");
        file.write_all_at(&[one[0] ^ 0xFF], at)
            .expect("the byte flipped");
    }
    drop(file);
    assert!(
        alerts::put_at(&root, &alert("GF1-SEAM-6"), NOW).is_err(),
        "the premise: the queue is damaged"
    );

    let done = alerts::replace_damaged_at(&root, NOW + 1).expect("every step of the replacement");
    let kept = names.dir.join(format!("alerts.damaged-{}", NOW + 1));
    assert_eq!(
        done,
        Replaced {
            kept: kept.clone(),
            open: Some(3),
            ordinal: 1,
        }
    );
    assert!(kept.is_file(), "the damaged queue is kept");
    let page = alerts::window(&root).expect("the new queue reads");
    assert_eq!((page.records, page.alerts.len()), (1, 1));
    assert_eq!(page.alerts[0].record.kind, alerts::KIND_QUEUE_REPLACED);
}
