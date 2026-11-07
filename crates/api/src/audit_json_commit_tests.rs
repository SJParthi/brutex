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

/// The current epoch `1` of `segment`, moved aside (never deleted), as a file
/// the census listed and then lost would be.
fn aside(root: &Path, segment: Segment) {
    let path = pull::manifest::epoch_path(root, Vendor::Gdfl, segment, 1);
    std::fs::rename(&path, path.with_extension("man.aside")).expect("moved aside");
}

/// `/audit.json`'s store block for GDFL over the store at `root`.
fn store_of(root: &Path, name: &str) -> serde_json::Value {
    let masters = crate::scratch::path(&format!("auditcommit-{name}-masters"));
    std::fs::create_dir_all(&masters).expect("mkdir");
    let site = crate::server::Site::load(&masters, root);
    let json = super::body(
        &site,
        Vendor::Gdfl,
        0,
        at(T0 + 1_000),
        Ok(pull::session::Day::new(2026, 8, 7).expect("a day")),
    );
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON");
    parsed["store"].clone()
}

/// `text` as the GDFL import journal of the store at `root`.
fn import_journal(root: &Path, text: &str) {
    let path = pull::gdfl_import::journal_path(root);
    std::fs::create_dir_all(path.parent().expect("imports/")).expect("mkdir");
    std::fs::write(&path, text).expect("a journal");
}

/// One day the census refused for options: one day behind, from that day.
const CENSUS_REFUSED: &str = "\
    begin options 2019-11-15 *\n\
    incomplete options 2019-11-15 * definition=2 files=9 seconds=99 failures=1 late=0 late_unresolved=0 max_back_s=0 stage=census reason=too_many_entries run=1791300000\n\
";

/// **NO EPOCH THAT STATS LEAVES NO COMMIT TIME, AND THE DAYS BEHIND ARE SAID
/// ONLY OVER AN IMPORT JOURNAL THAT READS** (BASE-COV-API repair 3, D-8661).
/// `committed_at` over partitions whose every current epoch will not stat,
/// and the store block of `/audit.json` over GDFL import journals.
/// Extremes: the one epoch of a census whose two other segments hold none,
/// moved aside (`1 of 3`, no commit time can be stated: a segment with no
/// epoch dates nothing); all three epochs of a three-part census moved
/// aside (`3 of 3`, no time); two of three moved aside (the third stat'ed, a
/// lower bound); an import journal with one census-refused day (one day
/// behind, from that day); a day refused at the vendor alone (no day, no first
/// day); no journal at all (`0`, never `null`); a journal under a regular file
/// (`null`: a journal that did not read is not zero days); a folder where the
/// journal goes (`null` too); and, at once, every epoch moved aside under a
/// journal that will not read (no commit time and no day count in one
/// answer), and every epoch moved aside beside a census-refused day (no
/// commit time, the first day still said); out of the box, an epoch replaced
/// by a symbolic link to itself (unread with the stat's own error, never
/// "gone").
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "every census and journal shape beside the others the same store block is read against"
)]
fn no_epoch_that_stats_leaves_no_commit_time_and_days_behind_need_a_journal_that_reads() {
    // THE ONE EPOCH OF A CENSUS WHOSE TWO OTHER SEGMENTS HOLD NONE.
    let one = root("allaside-one");
    partitioned(&one, &[(Segment::Index, 1, T0 + 100)], T0);
    let census = read(&one);
    aside(&one, Segment::Index);
    let committed = committed_at(&census);
    assert_eq!(committed.at, None, "no epoch stat'ed");
    assert_eq!(committed.basis, "newest census file");
    let json = from_json(&committed);
    assert_eq!(json["complete"], false, "{json}");
    let note = json["note"].as_str().expect("a note");
    assert!(
        note.starts_with("1 of 3 segment epoch(s) could not be stat'ed (INDEX)"),
        "{note}"
    );
    assert!(note.ends_with("no commit time can be stated"), "{note}");

    // EVERY EPOCH OF THREE, THEN TWO OF THREE.
    let three = root("allaside-three");
    let segments = [Segment::Index, Segment::Cash, Segment::Fno];
    partitioned(
        &three,
        &[
            (Segment::Index, 1, T0 + 100),
            (Segment::Cash, 1, T0 + 300),
            (Segment::Fno, 1, T0 + 200),
        ],
        T0,
    );
    let census = read(&three);
    for segment in segments {
        aside(&three, segment);
    }
    let note = from_json(&committed_at(&census))["note"]
        .as_str()
        .expect("a note")
        .to_owned();
    assert!(note.starts_with("3 of 3 segment epoch(s)"), "{note}");
    assert!(note.contains("INDEX, CASH, FNO"), "{note}");
    assert!(note.ends_with("no commit time can be stated"), "{note}");
    let two = root("aside-two-of-three");
    partitioned(
        &two,
        &[
            (Segment::Index, 1, T0 + 100),
            (Segment::Cash, 1, T0 + 300),
            (Segment::Fno, 1, T0 + 200),
        ],
        T0,
    );
    let census = read(&two);
    aside(&two, Segment::Index);
    aside(&two, Segment::Cash);
    let committed = committed_at(&census);
    assert_eq!(committed.at, Some(i64::try_from(T0 + 200).unwrap()));
    let note = from_json(&committed)["note"]
        .as_str()
        .expect("a note")
        .to_owned();
    assert!(note.starts_with("2 of 3 segment epoch(s)"), "{note}");
    assert!(note.ends_with("not the last commit"), "{note}");

    // OUT OF THE BOX: AN EPOCH THAT IS A LINK TO ITSELF.
    let looped = root("epoch-loop");
    partitioned(&looped, &[(Segment::Index, 1, T0 + 100)], T0);
    let census = read(&looped);
    aside(&looped, Segment::Index);
    let path = pull::manifest::epoch_path(&looped, Vendor::Gdfl, Segment::Index, 1);
    std::os::unix::fs::symlink(&path, &path).expect("a link to itself");
    let json = from_json(&committed_at(&census));
    let file = &json["files"][0];
    assert_eq!(file["state"], "unread", "{json}");
    let why = file["why"].as_str().expect("a reason");
    assert!(
        !why.contains("never deleted"),
        "the stat's own error: {why}"
    );
    assert!(
        json["note"]
            .as_str()
            .is_some_and(|note| note.ends_with("no commit time can be stated")),
        "{json}"
    );

    // THE DAYS BEHIND, OVER THE IMPORT JOURNAL.
    let refused = root("behind-census-day");
    partitioned(&refused, &[(Segment::Index, 1, T0 + 100)], T0);
    import_journal(&refused, CENSUS_REFUSED);
    let store = store_of(&refused, "behind-census-day");
    assert_eq!(store["behind_days"], 1, "{store}");
    assert_eq!(store["behind_from"], "2019-11-15", "{store}");
    let vendor = root("behind-vendor-day");
    partitioned(&vendor, &[(Segment::Index, 1, T0 + 100)], T0);
    import_journal(
        &vendor,
        "\
            begin indices 2022-03-07 *\n\
            refused indices 2022-03-07 * definition=2 stage=vendor reason=price_refused failures=2 run=1791300000\n\
        ",
    );
    let store = store_of(&vendor, "behind-vendor-day");
    assert_eq!(store["behind_days"], 0, "a vendor refusal: {store}");
    assert_eq!(store["behind_from"], serde_json::Value::Null, "{store}");
    let unjournaled = root("behind-no-journal");
    partitioned(&unjournaled, &[(Segment::Index, 1, T0 + 100)], T0);
    let store = store_of(&unjournaled, "behind-no-journal");
    assert_eq!(store["behind_days"], 0, "no journal is no day: {store}");
    let under_file = root("behind-journal-under-file");
    partitioned(&under_file, &[(Segment::Index, 1, T0 + 100)], T0);
    let parent = pull::gdfl_import::journal_path(&under_file)
        .parent()
        .expect("imports/")
        .to_path_buf();
    std::fs::write(&parent, b"a file where the journal's folder goes").expect("a file");
    let store = store_of(&under_file, "behind-journal-under-file");
    assert_eq!(
        store["behind_days"],
        serde_json::Value::Null,
        "unread, not zero: {store}"
    );
    assert_eq!(store["behind_from"], serde_json::Value::Null, "{store}");
    let folder = root("behind-journal-folder");
    partitioned(&folder, &[(Segment::Index, 1, T0 + 100)], T0);
    std::fs::create_dir_all(pull::gdfl_import::journal_path(&folder)).expect("a folder");
    let store = store_of(&folder, "behind-journal-folder");
    assert_eq!(store["behind_days"], serde_json::Value::Null, "{store}");

    // AT ONCE: NO EPOCH STATS, UNDER AN UNREADABLE JOURNAL AND BESIDE A
    // CENSUS-REFUSED DAY.
    std::fs::create_dir_all(pull::gdfl_import::journal_path(&one)).expect("a folder");
    let store = store_of(&one, "allaside-one");
    assert_eq!(store["committed_at"], serde_json::Value::Null, "{store}");
    assert_eq!(store["behind_days"], serde_json::Value::Null, "{store}");
    import_journal(&three, CENSUS_REFUSED);
    let store = store_of(&three, "allaside-three");
    assert_eq!(store["committed_at"], serde_json::Value::Null, "{store}");
    assert_eq!(store["behind_days"], 1, "{store}");
    assert_eq!(store["behind_from"], "2019-11-15", "{store}");
}
