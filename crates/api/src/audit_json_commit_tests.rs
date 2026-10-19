//! `committed_at` of a partitioned census (AH-04, g-ui-dom-vs-json-4, D-5880).
//!
//! `/audit` stat'ed `manifest/v4/gdfl` — the DIRECTORY a migrated vendor's
//! census lives in — and printed its mtime as the store's last commit, four
//! and a half days stale while every segment's epoch had been written since.
//! Every fixture here is an empty census written by this build's own writer;
//! no vendor row is involved.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use super::{Committed, committed_at};
use brutex_core::instrument::Segment;
use brutex_core::vendor::Vendor;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// A store root no other test names, never removed: each test makes a fresh
/// one under its own name and leaves it where it is.
fn root(name: &str) -> PathBuf {
    let dir = crate::scratch::path(&format!("auditcommit-{name}"));
    std::fs::create_dir_all(dir.join("manifest")).expect("mkdir");
    dir
}

/// A fixed moment the fixtures' mtimes are set relative to.
const T0: u64 = 1_790_000_000;

fn at(secs: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
}

/// Sets `path`'s mtime (a file or a directory) to `secs`.
fn touch(path: &Path, secs: u64) {
    let file = std::fs::File::open(path).expect("open to set its mtime");
    file.set_times(std::fs::FileTimes::new().set_modified(at(secs)))
        .expect("set the mtime");
}

/// One empty partition per `(segment, epoch, mtime)`, written by this build's
/// own writer, then the vendor directory's mtime set to `dir_secs` LAST, so it
/// is older than every file under it — the shape the real store had.
fn partitioned(root: &Path, parts: &[(Segment, u32, u64)], dir_secs: u64) {
    for &(segment, epoch, secs) in parts {
        let manifest = pull::manifest::Manifest::partition_genesis(Vendor::Gdfl, segment, 4_096)
            .expect("a partition");
        std::fs::create_dir_all(pull::manifest::segment_dir(root, Vendor::Gdfl, segment))
            .expect("mkdir");
        let path = pull::manifest::epoch_path(root, Vendor::Gdfl, segment, epoch);
        std::fs::write(&path, manifest.image()).expect("an epoch on disk");
        touch(&path, secs);
    }
    touch(&pull::manifest::vendor_dir(root, Vendor::Gdfl), dir_secs);
}

fn read(root: &Path) -> crate::census::VendorCensus {
    crate::census::read_vendor(root, Vendor::Gdfl)
}

/// `committed_from` as JSON.
fn from_json(committed: &Committed) -> serde_json::Value {
    serde_json::from_str(&committed.json()).expect("committed_from is JSON")
}

#[test]
fn a_partitioned_census_reports_its_newest_epoch_file() {
    let root = root("newest");
    // FNO's CURRENT epoch is 2; its epoch 1 is older by number and is given
    // the newest mtime of all, so an answer that read every file, rather than
    // the current epoch of each segment, would say T0 + 900.
    partitioned(
        &root,
        &[
            (Segment::Index, 1, T0 + 100),
            (Segment::Cash, 1, T0 + 300),
            (Segment::Fno, 1, T0 + 900),
            (Segment::Fno, 2, T0 + 200),
        ],
        T0,
    );
    let census = read(&root);
    assert!(
        matches!(census.state, crate::census::Census::Held { ref manifest } if manifest.is_partitioned()),
        "{:?}",
        census.state
    );
    // THE DEFECT, MEASURED ON THE FIXTURE: the directory's own mtime is the
    // oldest stamp here, and it is what `/audit` printed.
    let dir = std::fs::metadata(&census.path)
        .expect("the directory")
        .modified()
        .expect("mtime");
    assert_eq!(crate::ingest::epoch_secs(dir), i64::try_from(T0).unwrap());

    let committed = committed_at(&census);
    assert_eq!(
        committed.at,
        Some(i64::try_from(T0 + 300).unwrap()),
        "the newest CURRENT epoch"
    );
    assert_eq!(committed.basis, "newest census file");
    let json = from_json(&committed);
    assert_eq!(json["complete"], true);
    let files = json["files"].as_array().expect("files");
    assert_eq!(files.len(), 3, "one stat per segment: {json}");
    let fno = files
        .iter()
        .find(|file| file["segment"] == "FNO")
        .expect("FNO");
    assert_eq!(fno["epoch"], "000002");
    assert_eq!(fno["at"], T0 + 200);
    assert_eq!(fno["state"], "read");
    assert!(
        files
            .iter()
            .all(|file| Path::new(file["path"].as_str().unwrap())
                .extension()
                .is_some_and(|ext| ext == "man")),
        "{json}"
    );
    assert_eq!(json["note"], serde_json::Value::Null);
}

#[test]
fn a_single_file_census_is_still_its_one_files_mtime() {
    let root = root("single");
    let manifest =
        pull::manifest::Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
    let path = pull::manifest::manifest_path(&root, Vendor::Gdfl);
    std::fs::write(&path, manifest.image()).expect("written");
    touch(&path, T0 + 42);
    let census = read(&root);
    let committed = committed_at(&census);
    assert_eq!(committed.at, Some(i64::try_from(T0 + 42).unwrap()));
    assert_eq!(committed.basis, "manifest file");
    let json = from_json(&committed);
    assert_eq!(json["files"].as_array().map(Vec::len), Some(1));
    assert_eq!(json["files"][0]["segment"], serde_json::Value::Null);
    assert_eq!(json["complete"], true);
}

#[test]
fn an_epoch_that_will_not_stat_is_named_and_the_rest_is_a_lower_bound() {
    let root = root("unread");
    partitioned(
        &root,
        &[
            (Segment::Index, 1, T0 + 100),
            (Segment::Cash, 1, T0 + 700),
            (Segment::Fno, 1, T0 + 200),
        ],
        T0,
    );
    let census = read(&root);
    // The census read CASH's epoch; it is moved aside (never deleted) before
    // the stat, as a file the reader listed and then lost would be.
    let cash = pull::manifest::epoch_path(&root, Vendor::Gdfl, Segment::Cash, 1);
    std::fs::rename(&cash, cash.with_extension("man.aside")).expect("moved aside");
    let committed = committed_at(&census);
    assert_eq!(
        committed.at,
        Some(i64::try_from(T0 + 200).unwrap()),
        "the newest of the files that stat'ed, and never a zero for the one that did not"
    );
    let json = from_json(&committed);
    assert_eq!(json["complete"], false);
    let unread = json["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["state"] == "unread")
        .expect("the unread epoch is named");
    assert_eq!(unread["segment"], "CASH");
    assert_eq!(unread["at"], serde_json::Value::Null);
    assert!(
        unread["why"].as_str().unwrap().contains("never deleted"),
        "{json}"
    );
    let note = json["note"].as_str().expect("a note");
    assert!(
        note.contains("CASH") && note.contains("lower bound"),
        "{note}"
    );
}

#[test]
fn a_segment_with_no_epoch_contributes_nothing_and_says_so() {
    let root = root("noepoch");
    partitioned(
        &root,
        &[(Segment::Index, 1, T0 + 100), (Segment::Cash, 3, T0 + 50)],
        T0,
    );
    let census = read(&root);
    let committed = committed_at(&census);
    assert_eq!(committed.at, Some(i64::try_from(T0 + 100).unwrap()));
    let json = from_json(&committed);
    assert_eq!(json["complete"], true);
    let fno = json["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["segment"] == "FNO")
        .expect("FNO is named");
    assert_eq!(fno["state"], "no epoch");
    assert_eq!(fno["epoch"], serde_json::Value::Null);
    assert_eq!(fno["at"], serde_json::Value::Null);
}

#[test]
fn a_migrated_vendor_with_no_epoch_at_all_has_committed_nothing() {
    let root = root("empty-v4");
    std::fs::create_dir_all(pull::manifest::vendor_dir(&root, Vendor::Gdfl)).expect("mkdir");
    let committed = committed_at(&read(&root));
    assert_eq!(committed.at, None, "never the directory's mtime");
    let note = from_json(&committed)["note"]
        .as_str()
        .expect("a note")
        .to_owned();
    assert!(note.contains("no segment holds an epoch"), "{note}");
}

#[test]
fn a_partitioned_census_that_did_not_load_is_not_dated_by_its_directory() {
    let root = root("refused");
    partitioned(&root, &[(Segment::Index, 1, T0 + 100)], T0);
    // A CASH epoch that does not decode makes the whole census unreadable.
    std::fs::create_dir_all(pull::manifest::segment_dir(
        &root,
        Vendor::Gdfl,
        Segment::Cash,
    ))
    .expect("mkdir");
    std::fs::write(
        pull::manifest::epoch_path(&root, Vendor::Gdfl, Segment::Cash, 1),
        b"not a census",
    )
    .expect("written");
    let census = read(&root);
    assert!(
        matches!(census.state, crate::census::Census::Unreadable { .. }),
        "{:?}",
        census.state
    );
    let committed = committed_at(&census);
    assert_eq!(committed.at, None);
    assert_eq!(committed.basis, "none");
    let note = from_json(&committed)["note"]
        .as_str()
        .expect("a note")
        .to_owned();
    assert!(
        note.contains("is a directory") && note.contains("not a commit"),
        "{note}"
    );
}

#[test]
fn an_epoch_newer_than_the_clock_is_reported_as_it_is() {
    let root = root("skew");
    let ahead = crate::ingest::epoch_secs(SystemTime::now()) + 86_400;
    let ahead = u64::try_from(ahead).unwrap();
    partitioned(
        &root,
        &[(Segment::Index, 1, ahead), (Segment::Cash, 1, T0)],
        T0,
    );
    let committed = committed_at(&read(&root));
    // Shown as is; the page subtracts it from the server's own `at` and says
    // the file is ahead of the clock, rather than this route clamping it.
    assert_eq!(committed.at, Some(i64::try_from(ahead).unwrap()));
}

#[test]
fn the_audit_answer_carries_the_newest_epoch_and_what_it_stands_on() {
    let root = root("route");
    partitioned(
        &root,
        &[
            (Segment::Index, 1, T0 + 100),
            (Segment::Cash, 1, T0 + 300),
            (Segment::Fno, 1, T0 + 200),
        ],
        T0,
    );
    let masters = crate::scratch::path("auditcommit-route-masters");
    std::fs::create_dir_all(&masters).expect("mkdir");
    let site = crate::server::Site::load(&masters, &root);
    let json = super::body(
        &site,
        Vendor::Gdfl,
        0,
        at(T0 + 1_000),
        Ok(pull::session::Day::new(2026, 8, 7).expect("a day")),
    );
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON");
    let store = &parsed["store"];
    assert_eq!(store["committed_at"], T0 + 300, "{store}");
    assert_eq!(store["committed_from"]["basis"], "newest census file");
    assert_eq!(
        store["committed_from"]["files"].as_array().map(Vec::len),
        Some(3)
    );
}
