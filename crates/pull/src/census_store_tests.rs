//! The arms of `census_store` only its own module can reach: the run-level
//! compaction, the epoch install's refusals, the lock-free listing, the drift
//! check, the migration's refusals and the journal closing. D-3316, D-3317.
//!
//! Every fixture is invented: keys, counts, stamps and journal lines.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use store::path::{Timeframe, YearMonth};

use super::{
    Checked, Location, Mode, Span, Target, close_journal, count_growing, current_epoch, cycle,
    install_epoch, install_run, legacy_drift, locate, migrate, parse_day, read_epoch, record,
    staging_dir, target_for, target_for_plan,
};
use crate::manifest::{
    Closes, Entry, EntryKey, FNO_FLOOR, Held, Manifest, epoch_path, manifest_path, segment_dir,
    vendor_dir,
};
use crate::session::Day;

/// A scratch store of this test's own, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-census-store-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos())
        ));
        std::fs::create_dir_all(root.join("manifest")).expect("a scratch store");
        Self(root)
    }

    fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).expect("a month")
}

fn key(n: u32, month: YearMonth) -> EntryKey {
    EntryKey {
        contract: Contract::parse(&format!("2019-12-26-{}-CE", 1_000_000 + n * 100)),
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        symbol: Symbol::new("NIFTY").expect("a symbol"),
        timeframe: Timeframe::SECOND_1,
        month,
    }
}

fn row(n: u32, rows: u64) -> Held {
    row_at(
        n,
        ym(2019, 11),
        rows,
        10,
        10 + i64::try_from(rows).expect("rows"),
    )
}

fn row_at(n: u32, month: YearMonth, rows: u64, first: i64, last: i64) -> Held {
    Held::new(
        Entry {
            key: key(n, month),
            rows,
            first_ts_micros: first,
            last_ts_micros: last,
        },
        Closes::UNKNOWN,
    )
}

/// A partition of capacity 2 holding one key written twice — full.
fn full_at_two() -> Manifest {
    let mut census = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 2).expect("genesis");
    census.record_held(row(0, 1)).expect("one");
    census.record_held(row(0, 2)).expect("two");
    census
}

/// Writes `census` as `epoch` of the gdfl FNO partition, the vendor marked
/// migrated by its directory.
fn put(root: &Path, census: &Manifest, epoch: u32) -> PathBuf {
    let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, epoch);
    std::fs::create_dir_all(path.parent().expect("a parent dir")).expect("dir");
    std::fs::write(&path, census.image()).expect("write");
    path
}

/// A ROW THAT MEETS A FULL PARTITION MID-RUN compacts it in memory and is
/// recorded; the install then publishes the next epoch whole and the epoch it
/// replaced keeps every byte. A single file at its ceiling is never compacted.
#[test]
fn a_run_that_meets_a_full_partition_installs_its_next_epoch() {
    let scratch = Scratch::new("MIDRUN");
    let root = scratch.root();
    let full = full_at_two();
    let first = put(root, &full, 1);
    let before = std::fs::read(&first).expect("bytes");

    let target = target_for(root, Vendor::Gdfl, Segment::Fno).expect("a partition target");
    let lock = target.lock().expect("the partition lock");
    let mut loaded = target.load().expect("it loads");
    assert_eq!((loaded.epoch, loaded.exists), (Some(1), true));
    let mut compacted = false;
    let first_row = count_growing(&mut loaded.census, row(1, 5), &mut compacted).expect("recorded");
    assert!(first_row.is_some());
    assert!(compacted, "a compaction happened, so the appends are void");
    assert_eq!(
        loaded.census.capacity(),
        FNO_FLOOR,
        "the floor, not twice two keys"
    );
    assert_eq!((loaded.census.entries(), loaded.census.keys()), (2, 2));
    // An exact row is no change at all, compaction or not.
    assert_eq!(
        count_growing(&mut loaded.census, row(1, 5), &mut compacted).expect("exact"),
        None
    );
    install_run(&lock, &target, &loaded, &[], false, true).expect("installed");
    drop(lock);
    assert_eq!(
        std::fs::read(&first).expect("epoch 1"),
        before,
        "never rewritten"
    );
    assert_eq!(current_epoch(root, Vendor::Gdfl, Segment::Fno), Ok(Some(2)));
    let now = read_epoch(
        &epoch_path(root, Vendor::Gdfl, Segment::Fno, 2),
        Vendor::Gdfl,
        Segment::Fno,
    )
    .expect("epoch 2 reads");
    assert_eq!((now.entries(), now.keys(), now.total_rows()), (2, 2, 7));
    assert!(now.header().generation > full.header().generation);

    // A single file at its ceiling refuses, and nothing compacts it.
    let mut single = Manifest::open(Vendor::Gdfl, &[], &[]).expect("genesis");
    single.record_held(row(0, 1)).expect("one");
    let mut never = false;
    assert!(count_growing(&mut single, row(0, 0), &mut never).is_err());
    assert!(!never);
}

/// AN EPOCH IS WRITTEN ONCE: one already at its name is refused, and so is an
/// epoch number six digits cannot spell; a `.tmp` and a stray name are never
/// the current epoch.
#[test]
fn an_epoch_is_written_once_and_only_whole_epochs_are_current() {
    let scratch = Scratch::new("EPOCHS");
    let root = scratch.root();
    let census = full_at_two();
    let path = put(root, &census, 1);
    let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
    let lock = target.lock().expect("the lock");
    let why = install_epoch(&lock, root, Vendor::Gdfl, Segment::Fno, 1, &census.image())
        .expect_err("refused");
    assert!(why.contains("written once"), "{why}");
    for out_of_range in [0, 1_000_000] {
        let why = install_epoch(
            &lock,
            root,
            Vendor::Gdfl,
            Segment::Fno,
            out_of_range,
            &census.image(),
        )
        .expect_err("refused");
        assert!(why.contains("outside 1..=999999"), "{why}");
    }
    drop(lock);
    let dir = segment_dir(root, Vendor::Gdfl, Segment::Fno);
    std::fs::write(dir.join("000009.man.tmp"), b"half an epoch").expect("a torn tmp");
    std::fs::write(dir.join("notes.txt"), b"a stray").expect("a stray");
    std::fs::write(dir.join("7.man"), b"not six digits").expect("a misnamed file");
    assert_eq!(current_epoch(root, Vendor::Gdfl, Segment::Fno), Ok(Some(1)));
    assert_eq!(current_epoch(root, Vendor::Gdfl, Segment::Cash), Ok(None));
    assert_eq!(std::fs::read(&path).expect("bytes"), census.image());
    // A cycle over the partition ignores both and appends to epoch 1.
    let done = cycle(&target, &[row(0, 2)], Mode::AllOrNothing).expect("exact");
    assert_eq!((done.exact, done.recorded, done.epoch), (1, 0, Some(1)));
}

/// An epoch that is listed must be readable: an absent one, a directory at its
/// name and one longer than its own header allows are each refused by name.
#[test]
fn an_epoch_that_cannot_be_read_is_refused_never_read_as_absent() {
    let scratch = Scratch::new("READ");
    let root = scratch.root();
    let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, 3);
    let why = read_epoch(&path, Vendor::Gdfl, Segment::Fno).expect_err("absent");
    assert!(why.contains("never deleted"), "{why}");
    std::fs::create_dir_all(&path).expect("a directory at the epoch's name");
    let why = read_epoch(&path, Vendor::Gdfl, Segment::Fno).expect_err("a directory");
    assert!(why.contains("not a regular file"), "{why}");
    std::fs::remove_dir(&path).expect("removed");
    let census = full_at_two();
    let mut grown = census.image();
    grown.extend_from_slice(&[0u8; 128]);
    std::fs::write(&path, &grown).expect("one entry past its capacity of two");
    let why = read_epoch(&path, Vendor::Gdfl, Segment::Fno).expect_err("too long");
    assert!(why.contains("more than the"), "{why}");
    // At another segment's directory it is refused as that partition's.
    let cash = epoch_path(root, Vendor::Gdfl, Segment::Cash, 1);
    std::fs::create_dir_all(cash.parent().expect("parent")).expect("dir");
    std::fs::write(&cash, census.image()).expect("written");
    let why = read_epoch(&cash, Vendor::Gdfl, Segment::Cash).expect_err("mismatch");
    assert!(why.contains("holds FNO, not CASH"), "{why}");
}

/// A FULL PARTITION compacts into its next epoch at the segment's floor and
/// takes the batch there; a degraded partition is repaired into a new epoch,
/// never over the damaged one. (The refusal of a batch that does not fit even
/// compacted is `census_v4`'s, at the INDEX floor.)
#[test]
fn a_full_partition_compacts_and_a_degraded_one_moves_on() {
    let scratch = Scratch::new("REFUSE");
    let root = scratch.root();
    // A partition of capacity 2 holding two distinct keys: compacting frees
    // no entry, but the epoch it writes takes the segment's floor.
    let mut two = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 2).expect("genesis");
    two.record_held(row(0, 1)).expect("one");
    two.record_held(row(1, 1)).expect("two");
    put(root, &two, 1);
    let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
    // The compacted epoch takes the floor, so a batch fits after it.
    let done = cycle(&target, &[row(2, 1)], Mode::AllOrNothing).expect("compacts");
    let compaction = done.compacted.expect("a compaction");
    assert_eq!(compaction.why, "full");
    assert_eq!((compaction.from_epoch, compaction.to_epoch), (Some(1), 2));
    assert_eq!(done.epoch, Some(2));
    assert_eq!((done.entries, done.keys), (3, 3));

    // A torn last commit: the newest slot counts an entry the region does not
    // hold, so the epoch loads DEGRADED at the older generation.
    let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, 2);
    let bytes = std::fs::read(&path).expect("bytes");
    std::fs::write(&path, &bytes[..bytes.len() - 128]).expect("torn");
    let torn = std::fs::read(&path).expect("torn bytes");
    let repaired = cycle(&target, &[row(3, 1)], Mode::AllOrNothing).expect("repairs");
    assert!(repaired.degraded.is_some(), "it says it loaded degraded");
    assert_eq!(repaired.compacted.as_ref().map(|c| c.why), Some("repair"));
    assert_eq!(repaired.epoch, Some(3));
    assert_eq!(
        std::fs::read(&path).expect("epoch 2"),
        torn,
        "the damaged epoch is kept"
    );
}

/// A RECOUNT PUBLISHES ITS NEXT EPOCH WHOLE (D-3316): a full partition's
/// compaction is carried in memory and installed once, with the batch in it; a
/// row the census refuses is named and skipped while the rest are published; a
/// recount with nothing new installs nothing; and a degraded epoch is repaired
/// into a new one. No epoch it replaces is ever written again.
#[test]
fn a_recount_publishes_its_next_epoch_whole() {
    let scratch = Scratch::new("WHOLE");
    let root = scratch.root();
    // Key 0 written twice, at a capacity of two: full.
    let first = put(root, &full_at_two(), 1);
    let before = std::fs::read(&first).expect("bytes");
    let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);

    // A new key, a grown key, and the same key's count going backwards.
    let done = cycle(&target, &[row(1, 4), row(0, 3), row(0, 1)], Mode::PerKey).expect("published");
    assert_eq!((done.recorded, done.exact), (2, 0));
    assert_eq!(done.refused.len(), 1);
    assert_eq!(done.refused[0].0, key(0, ym(2019, 11)));
    assert!(
        done.refused[0]
            .1
            .contains("records 1 rows where 3 were already"),
        "{}",
        done.refused[0].1
    );
    let compaction = done.compacted.as_ref().expect("one epoch installed");
    assert_eq!(compaction.why, "full", "the carried compaction names why");
    assert_eq!((compaction.from_epoch, compaction.to_epoch), (Some(1), 2));
    assert_eq!((compaction.from_entries, compaction.to_entries), (2, 2));
    assert_eq!(done.epoch, Some(2));
    assert_eq!((done.entries, done.keys, done.rows), (2, 2, 7));
    assert_eq!(done.capacity, FNO_FLOOR);
    assert_eq!(std::fs::read(&first).expect("epoch 1"), before, "kept");
    assert_eq!(current_epoch(root, Vendor::Gdfl, Segment::Fno), Ok(Some(2)));

    // Something new and room for it: still whole, as a recount.
    let grown = cycle(&target, &[row(2, 1)], Mode::PerKey).expect("published");
    assert_eq!(grown.compacted.as_ref().map(|c| c.why), Some("recount"));
    assert_eq!(grown.epoch, Some(3));
    assert_eq!((grown.entries, grown.keys), (3, 3));

    // Nothing new: nothing installed, nothing written.
    let epoch3 = epoch_path(root, Vendor::Gdfl, Segment::Fno, 3);
    let bytes3 = std::fs::read(&epoch3).expect("epoch 3");
    let same = cycle(&target, &[row(2, 1)], Mode::PerKey).expect("exact");
    assert_eq!((same.exact, same.recorded, same.compacted), (1, 0, None));
    assert_eq!(same.epoch, Some(3));
    assert_eq!(current_epoch(root, Vendor::Gdfl, Segment::Fno), Ok(Some(3)));
    assert_eq!(std::fs::read(&epoch3).expect("epoch 3"), bytes3);

    // A degraded epoch: the repair is carried, and installed with the batch.
    // An import day appends one entry to epoch 3, then the file is cut by
    // that entry, so its newest slot counts an entry its region does not hold
    // and it loads at the older generation.
    let day = cycle(&target, &[row(5, 1)], Mode::AllOrNothing).expect("an import day appends");
    assert_eq!(
        (day.epoch, day.compacted),
        (Some(3), None),
        "an import appends"
    );
    let appended = std::fs::read(&epoch3).expect("epoch 3");
    std::fs::write(&epoch3, &appended[..appended.len() - 128]).expect("torn");
    let torn = std::fs::read(&epoch3).expect("torn bytes");
    let repaired = cycle(&target, &[row(4, 1)], Mode::PerKey).expect("repairs");
    assert!(repaired.degraded.is_some(), "it says it loaded degraded");
    assert_eq!(repaired.compacted.as_ref().map(|c| c.why), Some("repair"));
    assert_eq!(repaired.epoch, Some(4));
    assert_eq!(std::fs::read(&epoch3).expect("epoch 3"), torn, "kept");
}

/// A MIGRATION refuses a staging directory it did not leave clean and a single
/// file that loaded degraded; once migrated, drift in the single file is named.
#[test]
fn a_migration_refuses_what_it_cannot_carry_and_names_later_drift() {
    let scratch = Scratch::new("MIGRATE");
    let root = scratch.root();
    assert_eq!(
        record(root, Vendor::Gdfl, &[row(0, 1)]).map(|c| c.len()),
        Ok(1)
    );
    assert_eq!(
        record(root, Vendor::Gdfl, &[row(1, 1)]).map(|c| c.len()),
        Ok(1)
    );
    let legacy = manifest_path(root, Vendor::Gdfl);

    let staging = staging_dir(root, Vendor::Gdfl);
    std::fs::create_dir_all(&staging).expect("a staging dir");
    let why = migrate(root, Vendor::Gdfl).expect_err("refused");
    assert!(why.contains("stopped before it was renamed"), "{why}");
    std::fs::remove_dir_all(&staging).expect("cleared away");

    let bytes = std::fs::read(&legacy).expect("bytes");
    std::fs::write(&legacy, &bytes[..bytes.len() - 128]).expect("torn off");
    let why = migrate(root, Vendor::Gdfl).expect_err("refused");
    assert!(why.contains("loaded DEGRADED"), "{why}");
    std::fs::write(&legacy, &bytes).expect("whole again");

    let done = migrate(root, Vendor::Gdfl).expect("it migrates");
    assert_eq!(
        done.partitions.len(),
        3,
        "one epoch per segment, empty or not"
    );
    assert!(!staging.exists(), "the staging directory became the census");
    let receipt =
        std::fs::read_to_string(vendor_dir(root, Vendor::Gdfl).join("MIGRATED")).expect("receipt");
    assert!(
        receipt.contains(&format!("legacy_bytes={}", bytes.len())),
        "{receipt}"
    );
    assert!(
        receipt.contains("partition=FNO epoch=000001.man keys=2 rows=2"),
        "{receipt}"
    );
    assert_eq!(legacy_drift(root, Vendor::Gdfl), None, "unchanged since");
    let mut grown = bytes.clone();
    grown.extend_from_slice(&[0u8; 128]);
    std::fs::write(&legacy, &grown).expect("it appended");
    let drift = legacy_drift(root, Vendor::Gdfl).expect("drift is named");
    assert!(drift.contains("written to it since"), "{drift}");
    assert_eq!(legacy_drift(root, Vendor::Zerodha), None, "not migrated");
    let again = migrate(root, Vendor::Gdfl).expect_err("refused");
    assert!(again.contains("already version 4"), "{again}");
}

/// WHERE A CENSUS IS: the single file until `manifest/v4/<vendor>/` exists; a
/// plan's segment matters only once it does; anything else at that name is
/// refused, not guessed; a single-file writer that lost the race stops.
#[test]
fn a_census_is_located_by_its_vendor_directory_alone() {
    let scratch = Scratch::new("LOCATE");
    let root = scratch.root();
    assert_eq!(
        locate(root, Vendor::Gdfl),
        Ok(Location::Single(manifest_path(root, Vendor::Gdfl)))
    );
    let single = target_for_plan(root, Vendor::Gdfl, "NOT-A-SEGMENT").expect("the single file");
    assert_eq!(
        single.segment, None,
        "an unmigrated vendor ignores the segment"
    );
    assert_eq!(
        single.lock_path(),
        root.join("manifest").join("gdfl.man.lock")
    );
    let dir = vendor_dir(root, Vendor::Gdfl);
    std::fs::create_dir_all(dir.parent().expect("v4")).expect("v4");
    std::fs::write(&dir, b"a file").expect("a file");
    let why = locate(root, Vendor::Gdfl).expect_err("refused");
    assert!(why.contains("is not a directory"), "{why}");
    std::fs::remove_file(&dir).expect("removed");
    std::fs::create_dir_all(&dir).expect("dir");
    let why = target_for_plan(root, Vendor::Gdfl, "NOT-A-SEGMENT").expect_err("refused");
    assert!(why.contains("per segment"), "{why}");
    let fno = target_for_plan(root, Vendor::Gdfl, "FNO").expect("the FNO partition");
    assert_eq!(fno.named(), segment_dir(root, Vendor::Gdfl, Segment::Fno));
    assert_eq!(fno.lock_path(), dir.join("FNO.lock"));
    assert_eq!(fno.still_where_located(), Ok(()));
    let raced = Target::single(root, Vendor::Gdfl);
    let why = raced.still_where_located().expect_err("refused");
    assert!(why.contains("was migrated"), "{why}");
    // A partition's lock creates no directory, so it cannot migrate a vendor.
    let _guard = Target::partition(root, Vendor::Zerodha, Segment::Index).lock();
    assert!(!vendor_dir(root, Vendor::Zerodha).exists());
}

/// AN EPOCH AT THE SINGLE FILE'S NAME is refused by a single-file write: it is
/// one segment's census where every segment's is read, and appending to it as
/// the vendor's whole census would hide every other segment.
#[test]
fn an_epoch_at_the_single_file_s_name_is_refused() {
    let scratch = Scratch::new("MISPLACED");
    let root = scratch.root();
    let path = manifest_path(root, Vendor::Gdfl);
    std::fs::create_dir_all(path.parent().expect("manifest")).expect("dir");
    std::fs::write(&path, full_at_two().image()).expect("an epoch, misplaced");
    let why = Target::single(root, Vendor::Gdfl)
        .load()
        .expect_err("refused");
    assert!(
        why.contains("holds a version-4 epoch of the FNO partition"),
        "{why}"
    );
    let why = record(root, Vendor::Gdfl, &[row(1, 1)]).expect_err("nothing recorded");
    assert!(why.contains("not a single-file census"), "{why}");
    assert_eq!(
        std::fs::read(&path).expect("bytes"),
        full_at_two().image(),
        "left as it was"
    );
}

/// A journal of invented lines: three census-refused days, a vendor-refused
/// day outside the span, a crashed day, a done day, and a torn tail. The 15th
/// converted four files and only three months span it: a file that wrote no
/// bar that day spans no month, which is why `files=` decides nothing.
const JOURNAL: &str = "\
done options 2019-11-13 * definition=2 files=2 seconds=10 failures=0
begin options 2019-11-14 *
incomplete options 2019-11-14 * definition=2 files=2 seconds=10 failures=1
begin options 2019-11-15 *
incomplete options 2019-11-15 * definition=2 files=4 seconds=10 failures=1
begin options 2019-11-18 *
incomplete options 2019-11-18 * files=1 seconds=10 failures=1
begin options 2019-11-19 *
incomplete options 2019-11-19 * definition=2 files=1 stage=vendor reason=malformed_row failures=1
incomplete options 2018-11-06 * definition=2 files=1 seconds=10 failures=1
begin options 2019-11-20 *
incomplete options 2019-11-20 * definition=2 files=1 failures=1
incomplete swaps 2019-11-22 * definition=2 files=1 failures=1
begin options 2019-11-21 *
done opt";

/// IST micros of 10:00 on `day` of November 2019.
fn nov(day: i64) -> i64 {
    let (start, _) = ym(2019, 11).ist_bounds_micros();
    start + ((day - 1) * 86_400 + 36_000) * 1_000_000
}

/// THE JOURNAL CLOSING (D-3317 item 5): only the span's `incomplete` days
/// whose months the census now holds close, each with a `recounted` line after
/// one `run` line; the others are named and left open; the torn tail is
/// closed first; and a second closing finds nothing left to do.
#[test]
fn the_recount_closes_only_the_census_refused_days_it_can_prove() {
    let scratch = Scratch::new("JOURNAL");
    let root = scratch.root();
    let journal = root.join("imports").join("gdfl.journal");
    std::fs::create_dir_all(journal.parent().expect("imports")).expect("imports");
    std::fs::write(&journal, JOURNAL).expect("journal");
    // Rows the recount found: two option months spanning the 14th, three the
    // 15th (one of them also the 18th), none the 20th — where only an INDEX
    // month spans the day, and an options day's files are not index months.
    // The 15th closes with three months for its four files.
    let mut index_month = row_at(9, ym(2019, 11), 5, nov(1), nov(29));
    index_month.entry.key.segment = Segment::Index;
    index_month.entry.key.contract = None;
    let held = [
        row_at(0, ym(2019, 11), 5, nov(1), nov(14)),
        row_at(1, ym(2019, 11), 5, nov(14), nov(15)),
        row_at(2, ym(2019, 11), 5, nov(15), nov(18)),
        row_at(3, ym(2019, 11), 5, nov(15), nov(15)),
        index_month,
    ];
    let span = Span {
        from: Day::new(2019, 11, 14).expect("a day"),
        to: Some(Day::new(2019, 11, 30).expect("a day")),
    };
    let refused: HashSet<YearMonth> = HashSet::new();
    let checked = Checked {
        held: &held,
        unreadable: &[],
        refused: &refused,
        strays: 0,
    };
    let done = close_journal(root, span, Some("0123abcd"), &checked).expect("closes");
    let keys: Vec<&str> = done.closed.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, ["options 2019-11-14 *", "options 2019-11-15 *"]);
    assert_eq!(
        done.closed
            .iter()
            .map(|c| (c.months, c.files))
            .collect::<Vec<_>>(),
        [(2, Some(2)), (3, Some(4))]
    );
    assert_eq!(done.other_stage, ["options 2019-11-19 *"]);
    let open: Vec<&str> = done.left_open.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        open,
        [
            "options 2019-11-18 *",
            "options 2019-11-20 *",
            "swaps 2019-11-22 *"
        ]
    );
    assert!(done.left_open[0].1.contains("no bar definition"));
    let why = &done.left_open[1].1;
    assert!(
        why.contains("no FNO instrument-month the census holds spans the day"),
        "{why}"
    );
    assert!(
        done.left_open[2]
            .1
            .contains("its kind `swaps` names no segment"),
        "{:?}",
        done.left_open[2]
    );
    let run = done.run.expect("a run id");

    let text = std::fs::read_to_string(&journal).expect("journal");
    let tail = text
        .strip_prefix(JOURNAL)
        .expect("appended, never rewritten");
    assert_eq!(
        tail,
        format!(
            " (torn)\nrun {run} source=recount commit=0123abcd started={} from=2019-11-14 \
             to=2019-11-30\nrecounted options 2019-11-14 * definition=2 months=2 run={run}\n\
             recounted options 2019-11-15 * definition=2 months=3 run={run}\n",
            tail.split("started=")
                .nth(1)
                .and_then(|rest| rest.split(' ').next())
                .expect("a start stamp")
        )
    );

    // Twice: both days are done now, so nothing is written.
    let again = close_journal(root, span, None, &checked).expect("closes nothing");
    assert!(again.closed.is_empty() && again.run.is_none());
    assert_eq!(std::fs::read_to_string(&journal).expect("journal"), text);
}

/// A JOURNAL CLOSING THAT CANNOT PROVE A MONTH, OR READ A LINE, WRITES NOTHING:
/// a month holding an unreadable bar file or a refused row closes none of its
/// days, and a line this build does not know refuses the whole closing.
#[test]
fn a_closing_that_cannot_prove_a_month_or_read_a_line_writes_nothing() {
    let scratch = Scratch::new("JOURNAL-BLOCKED");
    let root = scratch.root();
    let journal = root.join("imports").join("gdfl.journal");
    std::fs::create_dir_all(journal.parent().expect("imports")).expect("imports");
    let held = [
        row_at(0, ym(2019, 11), 5, nov(1), nov(14)),
        row_at(1, ym(2019, 11), 5, nov(14), nov(15)),
    ];
    let span = Span {
        from: Day::new(2019, 11, 14).expect("a day"),
        to: Some(Day::new(2019, 11, 30).expect("a day")),
    };
    let refused: HashSet<YearMonth> = HashSet::new();
    let checked = Checked {
        held: &held,
        unreadable: &[],
        refused: &refused,
        strays: 0,
    };
    // A month with an unreadable file or a refused row closes nothing, and
    // neither does a walk that met a name it could not attribute.
    std::fs::write(&journal, JOURNAL.replace("\ndone opt", "\n")).expect("reset");
    let unreadable = [ym(2019, 11)];
    let blocked = Checked {
        held: &held,
        unreadable: &unreadable,
        refused: &refused,
        strays: 0,
    };
    let none = close_journal(root, span, None, &blocked).expect("closes nothing");
    assert!(none.closed.is_empty());
    assert!(
        none.left_open
            .iter()
            .any(|(_, why)| why.contains("could not be read"))
    );
    let refused_month: HashSet<YearMonth> = [ym(2019, 11)].into_iter().collect();
    let blocked = Checked {
        held: &held,
        unreadable: &[],
        refused: &refused_month,
        strays: 0,
    };
    let none = close_journal(root, span, None, &blocked).expect("closes nothing");
    assert!(
        none.left_open
            .iter()
            .any(|(_, why)| why.contains("census refused a row"))
    );
    let blocked = Checked {
        held: &held,
        unreadable: &[],
        refused: &refused,
        strays: 2,
    };
    let none = close_journal(root, span, None, &blocked).expect("closes nothing");
    assert!(none.closed.is_empty());
    assert!(
        none.left_open
            .iter()
            .any(|(_, why)| why.contains("2 directory entr(ies) under the vendor's bars"))
    );
    assert_eq!(
        std::fs::read_to_string(&journal).expect("journal"),
        JOURNAL.replace("\ndone opt", "\n"),
        "nothing written while any day is blocked"
    );

    // A line this build does not know refuses the closing, writing nothing.
    std::fs::write(&journal, "forgotten options 2019-11-14 *\n").expect("foreign");
    let why = close_journal(root, span, None, &checked).expect_err("refused");
    assert!(why.contains("not a journal line this build knows"), "{why}");
    assert_eq!(
        std::fs::read_to_string(&journal).expect("journal"),
        "forgotten options 2019-11-14 *\n"
    );
}

/// A journal day is `YYYY-MM-DD` exactly, and a span holds the months its days
/// overlap.
#[test]
fn journal_days_and_spans_read_the_store_s_own_spelling() {
    assert_eq!(parse_day("2019-11-14"), Day::new(2019, 11, 14).ok());
    for bad in ["2019-11-4", "2019-11", "2019-11-14-1", "2019-02-30", "x"] {
        assert_eq!(parse_day(bad), None, "{bad}");
    }
    let span = Span {
        from: Day::new(2019, 11, 14).expect("a day"),
        to: Some(Day::new(2020, 9, 15).expect("a day")),
    };
    assert!(span.holds(ym(2019, 11)) && span.holds(ym(2020, 9)));
    assert!(!span.holds(ym(2019, 10)) && !span.holds(ym(2020, 10)));
    assert!(span.contains(Day::new(2019, 11, 14).expect("a day")));
    assert!(!span.contains(Day::new(2019, 11, 13).expect("a day")));
    let open = Span { to: None, ..span };
    assert!(open.holds(ym(2026, 9)) && open.contains(Day::new(2026, 9, 30).expect("a day")));
}

// ═══ One batch, two flushes (WP-16, DB-N25) ═══════════════════════════════

/// A census already holding three rows, and the appends a day of `new` rows
/// (fresh keys from 100, and key 1 restated) asks of it, with the image they
/// are appended to.
fn a_day_of(new: u32) -> (Vec<u8>, Vec<crate::manifest::Append>, Manifest) {
    let mut census = Manifest::open_image(Vendor::Gdfl, &[]).expect("a genesis census");
    for n in 0..3 {
        census
            .record_held(row(n, 40 + u64::from(n)))
            .expect("a held row");
    }
    let before = census.image();
    let mut appends = Vec::new();
    for n in 0..new {
        let held = if n == 1 { row(1, 99) } else { row(100 + n, 7) };
        appends.push(census.record_held(held).expect("an append"));
    }
    (before, appends, census)
}

/// The writer this replaced, one entry at a time, each followed by its own
/// commit — kept here as the reference the batch's bytes are held to.
fn one_at_a_time(path: &Path, appends: &[crate::manifest::Append]) {
    use std::os::unix::fs::FileExt as _;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .expect("the census");
    for append in appends {
        file.write_all_at(&append.bytes, append.offset)
            .expect("the entry");
        file.write_all_at(&append.commit.bytes, append.commit.offset)
            .expect("the slot");
    }
}

/// A BATCH LANDS THE BYTES ONE ENTRY AT A TIME LEAVES (WP-16, DB-N25): seven
/// entries, both slots written; one entry, one slot; and the census each loads
/// is the census the batch was counted into.
#[test]
fn a_batch_lands_the_bytes_one_entry_at_a_time_leaves() {
    let scratch = Scratch::new("batch-bytes");
    for new in [1_u32, 2, 7] {
        let (before, appends, census) = a_day_of(new);
        let reference = scratch.root().join(format!("reference-{new}.man"));
        let batched = scratch.root().join(format!("batched-{new}.man"));
        std::fs::write(&reference, &before).expect("reference");
        std::fs::write(&batched, &before).expect("batched");
        one_at_a_time(&reference, &appends);
        super::write_batch(&batched, &appends, |_| {}).expect("the batch lands");
        let (want, got) = (
            std::fs::read(&reference).expect("reference"),
            std::fs::read(&batched).expect("batched"),
        );
        assert_ne!(want, before, "{new}: the day changed the census");
        assert_eq!(got, want, "{new}: the batch's bytes are the reference's");
        let loaded = Manifest::open_image(Vendor::Gdfl, &got).expect("it loads");
        assert_eq!(loaded.header(), census.header(), "{new}");
        assert_eq!(loaded.entries(), 3 + u64::from(new), "{new}");
    }
}

/// A BATCH IS TWO FLUSHES HOWEVER LONG IT IS, entries first and slots second;
/// it was two per entry. And an empty batch opens nothing.
#[test]
fn a_batch_is_two_flushes_however_long() {
    let scratch = Scratch::new("batch-flushes");
    for new in [1_u32, 7, 40] {
        let (before, appends, _) = a_day_of(new);
        let path = scratch.root().join(format!("census-{new}.man"));
        std::fs::write(&path, &before).expect("census");
        let mut seen = Vec::new();
        super::write_batch_with(&path, &appends, |_| {}, &mut |file, step| {
            seen.push(step);
            file.sync_all()
        })
        .expect("the batch lands");
        assert_eq!(
            seen,
            [super::BatchBarrier::Entries, super::BatchBarrier::Slots],
            "{new} entries"
        );
    }
    let absent = scratch.root().join("never.man");
    let mut flushed = 0;
    super::write_batch_with(&absent, &[], |_| {}, &mut |_, _| {
        flushed += 1;
        Ok(())
    })
    .expect("nothing to write");
    assert_eq!(flushed, 0);
    assert!(!absent.exists(), "an empty batch creates nothing");
}

/// A CRASH IS ALL OF THE DAY OR NONE OF IT: entries written and their flush
/// failed leaves no slot naming them, so the census loads as it was; the
/// batch's last commit alone, the other slot not yet rewritten, already names
/// the whole batch.
#[test]
fn a_crash_inside_a_batch_leaves_all_of_it_or_none() {
    use std::os::unix::fs::FileExt as _;
    let scratch = Scratch::new("batch-crash");
    let (before, appends, census) = a_day_of(7);
    let path = scratch.root().join("census.man");
    std::fs::write(&path, &before).expect("census");
    let refused = super::write_batch_with(&path, &appends, |_| {}, &mut |_, step| match step {
        super::BatchBarrier::Entries => Err(std::io::Error::other("the drive went away")),
        super::BatchBarrier::Slots => Ok(()),
    })
    .expect_err("the first flush failed");
    assert!(refused.to_string().contains("went away"), "{refused}");
    let loaded = Manifest::open_image(Vendor::Gdfl, &std::fs::read(&path).expect("census"))
        .expect("it loads");
    assert_eq!(loaded.entries(), 3, "no slot names an entry of the batch");

    let last = appends.last().expect("a last append");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("census");
    file.write_all_at(&last.commit.bytes, last.commit.offset)
        .expect("the newest slot alone");
    let loaded = Manifest::open_image(Vendor::Gdfl, &std::fs::read(&path).expect("census"))
        .expect("it loads");
    assert_eq!(
        loaded.header(),
        census.header(),
        "the newest slot names all of it"
    );
}

/// A COMMIT PAST THE BATCH IS REFUSED BEFORE A BYTE IS WRITTEN: a slot may
/// never name an entry the batch's one barrier did not cover.
#[test]
fn a_commit_past_the_batch_is_refused_before_a_byte() {
    let scratch = Scratch::new("batch-past");
    let (before, mut appends, _) = a_day_of(3);
    let path = scratch.root().join("census.man");
    std::fs::write(&path, &before).expect("census");
    if let Some(first) = appends.first_mut() {
        first.commit.durable_through = u64::MAX;
    }
    let why = super::write_batch(&path, &appends, |_| {}).expect_err("refused");
    assert!(why.to_string().contains("nothing was written"), "{why}");
    assert_eq!(std::fs::read(&path).expect("census"), before);
}
