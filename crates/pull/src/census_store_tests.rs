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
    Checked, FEATURE_CENSUS_V4, Location, MIGRATION_RECEIPT, Mode, REQUIRES_CAP, REQUIRES_FORMAT,
    REQUIRES_VOID, SEAL_RECEIPT, SEALED_VERSION, Span, Target, V4_READER_SINCE, close_journal,
    count_growing, current_epoch, cycle, holds_sealed_stub, install_epoch, install_run,
    legacy_drift, locate, migrate, parse_day, read_epoch, reader_refusal, record, require_feature,
    requires_path, seal_legacy, sealed_stub, staging_dir, superseded_path, target_for,
    target_for_plan,
};
use crate::manifest::{
    Closes, Entry, EntryKey, FNO_FLOOR, Held, Layout, Manifest, ManifestError, ManifestHeader,
    epoch_path, manifest_path, segment_dir, vendor_dir,
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
    // THE SINGLE FILE IS SEALED AS IT IS SUPERSEDED (DB-B68): its bytes kept
    // under the superseded name, its own name a stub old readers refuse.
    assert!(
        done.seal.as_ref().is_some_and(Result::is_ok),
        "{:?}",
        done.seal
    );
    assert_eq!(
        std::fs::read(superseded_path(root, Vendor::Gdfl)).expect("kept"),
        bytes
    );
    assert_eq!(
        std::fs::read(&legacy).expect("stub"),
        sealed_stub(Vendor::Gdfl)
    );
    // A writer that is not this build replaces the stub: named.
    let mut grown = bytes.clone();
    grown.extend_from_slice(&[0u8; 128]);
    std::fs::write(&legacy, &grown).expect("it appended");
    let drift = legacy_drift(root, Vendor::Gdfl).expect("drift is named");
    assert!(drift.contains("no longer holds the seal"), "{drift}");
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

// ═══ One batch, three flushes (WP-16, DB-N25) ═════════════════════════════

/// `bytes` laid over `image` at `at`, the image grown with zeros to hold them.
fn lay(image: &mut Vec<u8>, at: u64, bytes: &[u8]) {
    let at = usize::try_from(at).expect("an offset");
    if image.len() < at + bytes.len() {
        image.resize(at + bytes.len(), 0);
    }
    image[at..at + bytes.len()].copy_from_slice(bytes);
}

/// A census file already holding three rows, the appends a day of `new` rows
/// (fresh keys from 100, and key 1 restated) asks of it, and the census those
/// are counted into. `both`: the three rows were appended one at a time, so
/// BOTH header slots are populated, as after any earlier batch; otherwise the
/// file was written whole as an image (a first import, a recount, a
/// compaction), so it has ONE.
fn a_day_on(new: u32, both: bool) -> (Vec<u8>, Vec<crate::manifest::Append>, Manifest) {
    let mut census = Manifest::open_image(Vendor::Gdfl, &[]).expect("a genesis census");
    let mut file = Vec::new();
    for n in 0..3 {
        let append = census
            .record_held(row(n, 40 + u64::from(n)))
            .expect("a held row");
        if n == 0 || !both {
            file = census.image();
        } else {
            lay(&mut file, append.offset, &append.bytes);
            lay(&mut file, append.commit.offset, &append.commit.bytes);
        }
    }
    let mut appends = Vec::new();
    for n in 0..new {
        let held = if n == 1 { row(1, 99) } else { row(100 + n, 7) };
        appends.push(census.record_held(held).expect("an append"));
    }
    (file, appends, census)
}

/// [`a_day_on`] a census written whole as an image.
fn a_day_of(new: u32) -> (Vec<u8>, Vec<crate::manifest::Append>, Manifest) {
    a_day_on(new, false)
}

/// Where each header slot starts, and how long a slot write is.
fn slot_ranges() -> [std::ops::Range<usize>; 2] {
    let stride = usize::try_from(crate::manifest::HEADER_LEN / crate::manifest::SLOT_COUNT)
        .expect("a slot stride");
    [0, stride].map(|at| at..at + crate::manifest::IMAGE_LEN)
}

/// Which header slots differ between two images of one census file.
fn slots_changed(was: &[u8], now: &[u8]) -> [bool; 2] {
    slot_ranges().map(|slot| was[slot.clone()] != now[slot])
}

/// The header an image of a census file loads at, or why it does not load.
fn loads(image: &[u8]) -> Result<crate::manifest::ManifestHeader, String> {
    Manifest::open_image(Vendor::Gdfl, image)
        .map(|census| census.header())
        .map_err(|why| why.to_string())
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

/// A BATCH LANDS THE BYTES ONE ENTRY AT A TIME LEAVES (WP-16, DB-N25): one
/// entry, one slot; two, seven and forty, both slots; on a census with one
/// slot populated and with both; and the census each loads is the census the
/// batch was counted into.
#[test]
fn a_batch_lands_the_bytes_one_entry_at_a_time_leaves() {
    let scratch = Scratch::new("batch-bytes");
    for both in [false, true] {
        for new in [1_u32, 2, 7, 40] {
            let (before, appends, census) = a_day_on(new, both);
            let reference = scratch.root().join(format!("reference-{both}-{new}.man"));
            let batched = scratch.root().join(format!("batched-{both}-{new}.man"));
            std::fs::write(&reference, &before).expect("reference");
            std::fs::write(&batched, &before).expect("batched");
            one_at_a_time(&reference, &appends);
            super::write_batch(&batched, &appends, |_| {}).expect("the batch lands");
            let (want, got) = (
                std::fs::read(&reference).expect("reference"),
                std::fs::read(&batched).expect("batched"),
            );
            assert_ne!(want, before, "{both} {new}: the day changed the census");
            assert_eq!(
                got, want,
                "{both} {new}: the batch's bytes are the reference's"
            );
            let loaded = Manifest::open_image(Vendor::Gdfl, &got).expect("it loads");
            assert_eq!(loaded.header(), census.header(), "{both} {new}");
            assert_eq!(loaded.entries(), 3 + u64::from(new), "{both} {new}");
        }
    }
}

/// A BATCH IS THREE FLUSHES HOWEVER LONG IT IS, AND ONE SLOT WRITE AT A TIME
/// (repair 2): the entries, with no slot touched; then the slot the batch's
/// first commit belongs in, which is NOT the slot holding the newest
/// generation; and only then the other. A batch of one commit has one slot
/// write, so two flushes. It was two per entry. An empty batch opens nothing.
#[test]
fn a_batch_is_three_flushes_and_one_slot_write_at_a_time() {
    use super::BatchBarrier::{Entries, NewerSlot, OlderSlot};
    let scratch = Scratch::new("batch-flushes");
    for both in [false, true] {
        for new in [1_u32, 2, 7, 40] {
            let (before, appends, _) = a_day_on(new, both);
            let newest = loads(&before).expect("the census before the batch");
            let newest_slot = usize::try_from(newest.generation % 2).expect("0 or 1");
            let path = scratch.root().join(format!("census-{both}-{new}.man"));
            std::fs::write(&path, &before).expect("census");
            let mut seen = Vec::new();
            let mut landed = 0;
            super::write_batch_with(&path, &appends, |_| landed += 1, &mut |file, step| {
                let now = std::fs::read(&path).expect("census");
                seen.push((step, slots_changed(&before, &now)));
                file.sync_all()
            })
            .expect("the batch lands");
            let mut older_only = [true, true];
            older_only[newest_slot] = false;
            let mut want = vec![(Entries, [false, false]), (OlderSlot, older_only)];
            if new > 1 {
                want.push((NewerSlot, [true, true]));
            }
            assert_eq!(
                seen, want,
                "{both} {new}: the flushes, and the slots each found changed"
            );
            assert_eq!(landed, 1, "{both} {new}: seen once, after the last flush");
        }
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

/// A TORN WRITE INSIDE A BATCH NEVER LEAVES THE CENSUS BEHIND (repair 2; a
/// power cut, which no kill can emulate). What the drive holds for certain is
/// the file as each barrier found it; a cut between two barriers may have
/// landed ANY PART of what was written between them. For batches of 1, 2, 3,
/// 40 and 41 commits, on a census with both slots populated and on one
/// written whole as an image, every such tear (a first byte, half, all but
/// the last byte of each write in flight, alone and together) must load, at
/// a generation the batch or the census before it committed, and never below
/// the one the drive held at the barrier before.
///
/// Before the repair the batch's last commit went first, onto the slot
/// holding the newest generation whenever the batch was even, with the other
/// slot's write in flight beside it: torn, the census loaded one generation
/// behind, or not at all.
#[test]
fn a_torn_write_inside_a_batch_never_leaves_the_census_behind() {
    let scratch = Scratch::new("batch-torn");
    let mut behind = Vec::new();
    let mut together = Vec::new();
    let mut tears = 0_u32;
    for both in [true, false] {
        for new in [1_u32, 2, 3, 40, 41] {
            let (before, appends, census) = a_day_on(new, both);
            let path = scratch.root().join(format!("census-{both}-{new}.man"));
            std::fs::write(&path, &before).expect("census");
            let mut durable = vec![before.clone()];
            super::write_batch_with(&path, &appends, |_| {}, &mut |_, _| {
                durable.push(std::fs::read(&path).expect("census"));
                Ok(())
            })
            .expect("the batch lands");
            let mut committed = vec![loads(&before).expect("the census before the batch")];
            committed.extend(appends.iter().map(|append| append.commit.header));
            let entries =
                usize::try_from(crate::manifest::HEADER_LEN).expect("a length")..usize::MAX;
            for pair in durable.windows(2) {
                let (was, now) = (&pair[0], &pair[1]);
                let held = loads(was).expect("what a barrier found loads");
                // The writes in flight between the two barriers.
                let mut flying: Vec<std::ops::Range<usize>> = slot_ranges()
                    .into_iter()
                    .filter(|slot| was[slot.clone()] != now[slot.clone()])
                    .collect();
                if flying.len() > 1 {
                    together.push((both, new, "two slot writes"));
                }
                let tail = entries.start..now.len();
                if was.get(tail.clone()) != now.get(tail.clone()) {
                    if !flying.is_empty() {
                        together.push((both, new, "entries and a slot"));
                    }
                    flying.push(tail);
                }
                assert!(!flying.is_empty(), "{both} {new}: a barrier over no write");
                // Each write torn alone and, when more than one is in flight,
                // all of them torn together; cut after its first byte, at its
                // half and before its last byte.
                let mut cases: Vec<Vec<std::ops::Range<usize>>> =
                    flying.iter().cloned().map(|one| vec![one]).collect();
                if flying.len() > 1 {
                    cases.push(flying.clone());
                }
                for case in &cases {
                    for cut in ["first byte", "half", "all but the last byte"] {
                        let mut torn = was.clone();
                        for write in case {
                            let part = match cut {
                                "first byte" => 1,
                                "half" => write.len() / 2,
                                _ => write.len() - 1,
                            };
                            let at = u64::try_from(write.start).expect("an offset");
                            lay(&mut torn, at, &now[write.start..write.start + part]);
                        }
                        tears += 1;
                        let ok = loads(&torn).is_ok_and(|header| {
                            header.generation >= held.generation && committed.contains(&header)
                        });
                        if !ok {
                            let loaded =
                                loads(&torn).map(|header| (header.n_valid, header.generation));
                            behind.push((both, new, case.clone(), cut, loaded, held.generation));
                        }
                    }
                }
            }
            assert_eq!(
                durable.last().map(|image| loads(image)),
                Some(Ok(census.header())),
                "{both} {new}: the last barrier holds the whole batch"
            );
        }
    }
    let torn_behind: Vec<String> = behind
        .iter()
        .map(|(both, new, case, cut, loaded, held)| {
            format!(
                "({both}, {new}) {case:?} cut at its {cut}: held generation {held}, loads as \
                 (entries, generation) {loaded:?}"
            )
        })
        .collect();
    assert!(
        together.is_empty() && torn_behind.is_empty(),
        "by (both slots populated, commits): writes in flight together between two barriers \
         = {together:?}; and a torn write that left the census behind where the drive held \
         it, or unreadable = {torn_behind:#?}"
    );
    assert_eq!(tears, 84, "every tear was tried");
}

/// A FLUSH THE HOST REFUSES STOPS THE BATCH AT A WHOLE GENERATION, and the
/// caller is never told it landed. Refused after the entries: no slot names
/// one, the census loads as it was. Refused after the first slot: the other
/// slot is never written, and the census loads at that slot's commit.
/// Refused after the second: every byte is written, and the refusal is still
/// the answer.
#[test]
fn a_refused_flush_inside_a_batch_stops_it_at_a_whole_generation() {
    use super::BatchBarrier::{Entries, NewerSlot, OlderSlot};
    let scratch = Scratch::new("batch-refused");
    for new in [7_u32, 8] {
        let (before, appends, census) = a_day_on(new, true);
        let was = loads(&before).expect("the census before the batch");
        let whole = census.header();
        let older = appends[0].commit.offset;
        let in_older = appends
            .iter()
            .rev()
            .find(|append| append.commit.offset == older)
            .expect("the first commit's slot")
            .commit
            .header;
        for (at, loaded, slots) in [
            (Entries, was, [false, false]),
            (OlderSlot, in_older, [true, false]),
            (NewerSlot, whole, [true, true]),
        ] {
            let path = scratch.root().join(format!("census-{new}-{at:?}.man"));
            std::fs::write(&path, &before).expect("census");
            let mut landed = 0;
            let refused =
                super::write_batch_with(&path, &appends, |_| landed += 1, &mut |_, step| {
                    if step == at {
                        Err(std::io::Error::other("the drive went away"))
                    } else {
                        Ok(())
                    }
                })
                .expect_err("a flush was refused");
            assert!(refused.to_string().contains("went away"), "{refused}");
            assert_eq!(landed, 0, "{new} {at:?}: never reported as landed");
            let now = std::fs::read(&path).expect("census");
            assert_eq!(loads(&now), Ok(loaded), "{new} {at:?}");
            let mut changed = slots_changed(&before, &now);
            if was.generation.is_multiple_of(2) {
                changed.reverse();
            }
            assert_eq!(
                changed, slots,
                "{new} {at:?}: (older slot, newer slot) written"
            );
        }
        // The batch is odd or even; the first slot's commit is its last or
        // the one before it.
        let short = whole.generation - in_older.generation;
        assert_eq!(short, u64::from(new.is_multiple_of(2)), "{new}");
    }
}

/// A COMMIT PAST THE BATCH IS REFUSED BEFORE A BYTE IS WRITTEN: a slot may
/// never name an entry the batch's entries barrier did not cover.
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

// ═══ The seal (DB-B68, WP-81) ═══════════════════════════════════════════════

/// **THE STUB IS NO GEOMETRY ANY READER KNOWS.** Every reader resolves a
/// version through `Layout::for_version` and refuses an unknown one by number
/// — the reader a rollback starts (`tgt/app5`, built at b72371d8) has the same
/// dispatch — so the stub is refused as `unknown manifest version 65535`, and
/// a census at its name is never read, as held or as empty.
#[test]
fn the_seal_version_is_no_geometry_any_reader_knows() {
    assert_eq!(
        Layout::for_version(SEALED_VERSION),
        Err(ManifestError::UnknownVersion(u16::MAX))
    );
    let stub = sealed_stub(Vendor::Gdfl);
    assert_eq!(stub.len(), 32_768, "one whole header region");
    assert_eq!(
        ManifestHeader::decode(&stub[..64]),
        Err(ManifestError::UnknownVersion(u16::MAX))
    );
    assert_eq!(
        ManifestHeader::decode(&stub[16_384..16_384 + 64]),
        Err(ManifestError::UnknownVersion(u16::MAX)),
        "the second slot too, so no generation survives"
    );
    let refused = Manifest::open_image(Vendor::Gdfl, &stub).expect_err("refused");
    assert_eq!(refused.to_string(), "unknown manifest version 65535");
    let text = String::from_utf8_lossy(&stub);
    assert!(text.contains("SEALED (DB-B68)") && text.contains(V4_READER_SINCE));
}

/// A store migrated BEFORE the seal existed — the operator's GDFL census of
/// 10 Oct 2026 — is put back to that state here: the single file at its name,
/// no superseded file, no seal receipt.
fn unsealed(root: &Path) -> Vec<u8> {
    let legacy = manifest_path(root, Vendor::Gdfl);
    let kept = superseded_path(root, Vendor::Gdfl);
    let bytes = std::fs::read(&kept).expect("the superseded census");
    std::fs::remove_file(&legacy).expect("the stub removed");
    std::fs::rename(&kept, &legacy).expect("the census back at its name");
    std::fs::remove_file(vendor_dir(root, Vendor::Gdfl).join(SEAL_RECEIPT)).expect("no receipt");
    assert_eq!(legacy_drift(root, Vendor::Gdfl), None);
    bytes
}

/// **AN ALREADY-MIGRATED VENDOR IS SEALED ONCE, AND A RERUN ONLY CONFIRMS
/// IT.** The census's bytes are the superseded file's, the name holds the
/// stub, the receipt names the kept file and the first version-4 reader, and
/// nothing drifts.
#[test]
fn an_already_migrated_vendor_is_sealed_once_and_a_rerun_confirms_it() {
    let scratch = Scratch::new("SEAL");
    let root = scratch.root();
    assert!(record(root, Vendor::Gdfl, &[row(0, 1)]).is_ok());
    let done = migrate(root, Vendor::Gdfl).expect("it migrates");
    assert!(done.seal.as_ref().is_some_and(Result::is_ok));
    let bytes = unsealed(root);

    let seal = seal_legacy(root, Vendor::Gdfl).expect("sealed");
    assert!(seal.wrote);
    assert_eq!(seal.legacy_bytes, u64::try_from(bytes.len()).expect("len"));
    assert_eq!(std::fs::read(&seal.superseded).expect("kept"), bytes);
    assert!(holds_sealed_stub(&seal.legacy));
    let receipt = std::fs::read_to_string(vendor_dir(root, Vendor::Gdfl).join(SEAL_RECEIPT))
        .expect("the seal receipt");
    assert!(
        receipt.contains(&format!("superseded_bytes={}", bytes.len())),
        "{receipt}"
    );
    assert!(
        receipt.contains(&format!("reader_since={V4_READER_SINCE}")),
        "{receipt}"
    );
    assert_eq!(legacy_drift(root, Vendor::Gdfl), None);
    // The partitions read as before: the seal touched no epoch.
    assert!(matches!(
        locate(root, Vendor::Gdfl),
        Ok(Location::Partitioned(_))
    ));

    let again = seal_legacy(root, Vendor::Gdfl).expect("confirmed");
    assert!(!again.wrote, "nothing written the second time");
    assert_eq!(std::fs::read(&again.superseded).expect("kept"), bytes);
}

/// **THE SEAL RECEIPT IS READ, NEVER TRUSTED FOR ITS NAME, AND ITS NAME NEVER
/// HOLDS HALF OF ONE (DB-B68).** The receipt was created at its final name
/// and then written; a crash between the two left an empty `SEALED`, and
/// every later seal run answered "already sealed, nothing was written" over
/// zero bytes — the file a deploy reads `reader_since` from.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "every shape a crash leaves, in one store"
)]
fn a_seal_receipt_no_seal_finished_writing_is_kept_aside_and_written_whole() {
    use super::{SEAL_RECEIPT_FORMAT, SealReceipt, seal_receipt};
    let scratch = Scratch::new("SEALRECEIPT");
    let root = scratch.root();
    assert!(record(root, Vendor::Gdfl, &[row(0, 1)]).is_ok());
    let done = migrate(root, Vendor::Gdfl).expect("it migrates");
    assert!(done.seal.as_ref().is_some_and(Result::is_ok));
    let dir = vendor_dir(root, Vendor::Gdfl);
    let at = dir.join(SEAL_RECEIPT);
    let whole = std::fs::read_to_string(&at).expect("the receipt");
    assert!(
        whole.starts_with(&format!("{SEAL_RECEIPT_FORMAT}\n")),
        "{whole}"
    );
    assert_eq!(seal_receipt(&at), Ok(SealReceipt::Whole));
    assert!(
        !dir.join("SEALED.tmp").exists(),
        "the staged receipt was renamed, not left"
    );
    let names = |dir: &Path| {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .expect("listed")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| name.starts_with("SEALED"))
            .collect();
        names.sort();
        names
    };
    assert_eq!(names(&dir), ["SEALED"]);

    // EVERY SHAPE A CRASH MID-WRITE LEAVES: empty; cut inside the first
    // line; cut before reader_since; cut inside reader_since (no newline);
    // a reader_since that is no commit.
    let cut_at = |word: &str| {
        whole
            .split_at(whole.find(word).expect("present"))
            .0
            .to_owned()
    };
    let torn = [
        String::new(),
        "format=census-se".to_owned(),
        cut_at("stub_version="),
        whole.trim_end_matches('\n').to_owned(),
        whole.replace(V4_READER_SINCE, "not-a-commit"),
        whole.replace("superseded_bytes=", "superseded_bytes=x"),
    ];
    for (n, bytes) in torn.iter().enumerate() {
        std::fs::write(&at, bytes).expect("torn");
        assert!(
            matches!(seal_receipt(&at), Ok(SealReceipt::Torn(_))),
            "{bytes:?}: {:?}",
            seal_receipt(&at)
        );
        let seal = seal_legacy(root, Vendor::Gdfl).expect("confirmed and repaired");
        assert!(seal.wrote, "{bytes:?}: a whole receipt was written");
        let aside = seal.torn_receipt.expect("the torn bytes are kept");
        assert_eq!(
            std::fs::read_to_string(&aside).expect("kept"),
            *bytes,
            "byte for byte"
        );
        assert_eq!(std::fs::read_to_string(&at).expect("whole"), whole);
        assert_eq!(seal_receipt(&at), Ok(SealReceipt::Whole));
        assert_eq!(names(&dir).len(), n + 2, "one kept file per torn receipt");
        let again = seal_legacy(root, Vendor::Gdfl).expect("confirmed");
        assert!(!again.wrote && again.torn_receipt.is_none());
    }

    // A STAGED RECEIPT A CRASH LEFT is not the receipt and does not stop the
    // next run.
    std::fs::remove_file(&at).expect("removed");
    std::fs::write(dir.join("SEALED.tmp"), "format=census-se").expect("staged");
    assert_eq!(seal_receipt(&at), Ok(SealReceipt::Absent));
    let seal = seal_legacy(root, Vendor::Gdfl).expect("written");
    assert!(seal.wrote && seal.torn_receipt.is_none());
    assert_eq!(std::fs::read_to_string(&at).expect("whole"), whole);
    assert!(!dir.join("SEALED.tmp").exists());

    // A LATER BUILD'S RECEIPT IS NOT THIS BUILD'S TO REPLACE: refused by
    // name, left byte for byte.
    let foreign = whole.replace(SEAL_RECEIPT_FORMAT, "format=census-seal-v2");
    std::fs::write(&at, &foreign).expect("foreign");
    assert_eq!(
        seal_receipt(&at),
        Ok(SealReceipt::Foreign("census-seal-v2".to_owned()))
    );
    let why = seal_legacy(root, Vendor::Gdfl).expect_err("refused");
    assert!(
        why.contains("SEALED")
            && why.contains("census-seal-v2")
            && why.contains("nothing was written"),
        "{why}"
    );
    assert_eq!(std::fs::read_to_string(&at).expect("left"), foreign);

    // A NAME THAT IS NO REGULAR FILE is refused by name, at once.
    std::fs::remove_file(&at).expect("removed");
    std::fs::create_dir(&at).expect("a directory");
    let why = seal_legacy(root, Vendor::Gdfl).expect_err("refused");
    assert!(
        why.contains("SEALED") && why.contains("not a regular file"),
        "{why}"
    );
    std::fs::remove_dir(&at).expect("removed");
    std::os::unix::fs::symlink(dir.join(MIGRATION_RECEIPT), &at).expect("a link");
    let why = seal_legacy(root, Vendor::Gdfl).expect_err("refused");
    assert!(why.contains("SEALED"), "{why}");
}

/// **A SEAL REFUSES WHAT IT WOULD HIDE, AND FINISHES WHAT A CRASH LEFT.**
/// Months a pre-version-4 writer appended since migration are refused by name,
/// nothing written; a vendor not migrated has nothing to seal; a seal stopped
/// between its link and its rename is completed by the next run, and that
/// halfway state is no drift.
#[test]
fn a_seal_refuses_drift_and_an_unmigrated_vendor_and_finishes_an_interrupted_one() {
    let scratch = Scratch::new("SEAL-REFUSE");
    let root = scratch.root();
    let why = seal_legacy(root, Vendor::Gdfl).expect_err("not migrated");
    assert!(why.contains("is not migrated"), "{why}");
    assert!(record(root, Vendor::Gdfl, &[row(0, 1)]).is_ok());
    assert!(migrate(root, Vendor::Gdfl).is_ok());
    let bytes = unsealed(root);
    let legacy = manifest_path(root, Vendor::Gdfl);
    let kept = superseded_path(root, Vendor::Gdfl);

    let mut grown = bytes.clone();
    grown.extend_from_slice(&[0_u8; 128]);
    std::fs::write(&legacy, &grown).expect("an old writer appended");
    let why = seal_legacy(root, Vendor::Gdfl).expect_err("drift refused");
    assert!(why.contains("Count those months first"), "{why}");
    assert!(!kept.exists(), "nothing was written");
    std::fs::write(&legacy, &bytes).expect("as migrated");

    std::fs::hard_link(&legacy, &kept).expect("a seal stopped after its link");
    assert_eq!(
        legacy_drift(root, Vendor::Gdfl),
        None,
        "halfway is no write"
    );
    let seal = seal_legacy(root, Vendor::Gdfl).expect("finished");
    assert!(seal.wrote);
    assert!(holds_sealed_stub(&legacy));
    assert_eq!(std::fs::read(&kept).expect("kept"), bytes);
    assert!(
        vendor_dir(root, Vendor::Gdfl)
            .join(MIGRATION_RECEIPT)
            .exists()
    );

    // A superseded file of the right length that is NOT this census is
    // refused rather than guessed at.
    let other = Scratch::new("SEAL-OTHER");
    let root = other.root();
    assert!(record(root, Vendor::Gdfl, &[row(0, 1)]).is_ok());
    assert!(migrate(root, Vendor::Gdfl).is_ok());
    let bytes = unsealed(root);
    std::fs::write(superseded_path(root, Vendor::Gdfl), &bytes).expect("a copy, not a link");
    let why = seal_legacy(root, Vendor::Gdfl).expect_err("refused");
    assert!(why.contains("refused rather than guess"), "{why}");
}

// ═══ What the store requires of its reader (DB-B68, WP-81) ══════════════════

/// **A MIGRATION SAYS WHAT ITS CENSUS REQUIRES, ONCE, AND THIS BUILD HAS IT.**
/// The requirements file is absent before any format change; the migration's
/// seal writes the format line and one `census-v4` line for the vendor; a
/// second seal and a second request write nothing; and the census is located
/// as before.
#[test]
fn a_migration_records_the_reader_feature_its_census_requires() {
    let scratch = Scratch::new("REQ");
    let root = scratch.root();
    let at = requires_path(root);
    assert_eq!(at, root.join("manifest/REQUIRES"));
    assert_eq!(reader_refusal(root, Vendor::Gdfl), None, "no file, no rule");
    assert!(record(root, Vendor::Gdfl, &[row(0, 1)]).is_ok());
    assert!(migrate(root, Vendor::Gdfl).is_ok());
    let text = std::fs::read_to_string(&at).expect("written by the seal");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(lines[0], REQUIRES_FORMAT);
    assert!(
        lines[1].starts_with(&format!(
            "requires={FEATURE_CENSUS_V4} vendor=gdfl since={V4_READER_SINCE} at="
        )),
        "{text}"
    );
    assert!(text.ends_with('\n'));
    assert_eq!(
        reader_refusal(root, Vendor::Gdfl),
        None,
        "this build has it"
    );
    assert!(matches!(
        locate(root, Vendor::Gdfl),
        Ok(Location::Partitioned(_))
    ));
    assert_eq!(
        require_feature(root, Vendor::Gdfl, FEATURE_CENSUS_V4),
        Ok(false),
        "already in force"
    );
    assert!(!seal_legacy(root, Vendor::Gdfl).expect("confirmed").wrote);
    assert_eq!(std::fs::read_to_string(&at).expect("unchanged"), text);
    // Another vendor's requirement is its own line.
    assert_eq!(
        require_feature(root, Vendor::Zerodha, FEATURE_CENSUS_V4),
        Ok(true)
    );
    assert_eq!(
        std::fs::read_to_string(&at).expect("grown").lines().count(),
        3
    );
}

/// **A CENSUS THAT NEEDS WHAT THIS BUILD LACKS IS REFUSED BY NAME, FOR THE
/// VENDOR IT BINDS AND NO OTHER (DB-B68).** The failure a rollback met: a
/// build read the shape it knew and served it as held. A line naming a
/// feature this build does not have refuses `locate` for that vendor, naming
/// the file, the line and the feature; `vendor=*` binds every vendor; a
/// vendor the line does not name is located as before.
#[test]
fn a_census_requiring_an_unknown_feature_is_refused_for_the_vendor_it_binds() {
    let scratch = Scratch::new("REQ-UNKNOWN");
    let root = scratch.root();
    assert!(record(root, Vendor::Gdfl, &[row(0, 1)]).is_ok());
    assert!(migrate(root, Vendor::Gdfl).is_ok());
    let at = requires_path(root);
    let mut text = std::fs::read_to_string(&at).expect("the seal's lines");
    text.push_str("requires=census-v5 vendor=gdfl since=ffff at=1\n");
    std::fs::write(&at, &text).expect("a later migration's line");

    let why = reader_refusal(root, Vendor::Gdfl).expect("refused");
    assert!(
        why.contains("manifest/REQUIRES")
            && why.contains("line 3")
            && why.contains("\"census-v5\"")
            && why.contains("this build does not have")
            && why.contains("DB-B68"),
        "{why}"
    );
    assert_eq!(locate(root, Vendor::Gdfl), Err(why.clone()));
    assert!(
        record(root, Vendor::Gdfl, &[row(1, 1)]).is_err(),
        "and no writer of this build adds to a census it cannot read"
    );
    assert_eq!(reader_refusal(root, Vendor::Zerodha), None);
    assert!(matches!(
        locate(root, Vendor::Zerodha),
        Ok(Location::Single(_))
    ));

    text.push_str("requires=bars-v4 vendor=* since=ffff at=2\n");
    std::fs::write(&at, &text).expect("a store-wide line");
    let every = reader_refusal(root, Vendor::Zerodha).expect("every vendor");
    assert!(
        every.contains("line 4") && every.contains("\"bars-v4\""),
        "{every}"
    );
}

/// **EVERY EXTREME OF THE FILE.** Empty; only an unfinished tail; a tail a
/// writer voids and follows with its own line; duplicates; another format; a
/// line of no known shape; a file too long to be one; and something at the
/// name that is not a file. What cannot be told is refused, never read as
/// "requires nothing"; an append that did not finish is never a rule.
#[test]
fn the_requirements_file_is_read_exactly_or_refused() {
    let scratch = Scratch::new("REQ-EDGE");
    let root = scratch.root();
    let at = requires_path(root);
    let say = |text: &str| {
        std::fs::write(&at, text).expect("written");
        reader_refusal(root, Vendor::Gdfl)
    };
    assert_eq!(say(""), None, "empty");
    assert_eq!(say("\n\n"), None, "blank lines");
    assert_eq!(
        say(&format!(
            "{REQUIRES_FORMAT}\nrequires=census-v4 vendor=gdfl\nrequires=census-v4 vendor=gdfl\n"
        )),
        None,
        "a duplicate is the same rule twice"
    );

    // AN APPEND THAT DID NOT FINISH IS NOT A RULE, whatever it would have said.
    assert_eq!(
        say(&format!("{REQUIRES_FORMAT}\nrequires=census-v9 vendor=gd")),
        None,
        "no newline, not in force"
    );
    // THE NEXT WRITER VOIDS IT AND WRITES ITS OWN LINE AFTER, by append alone.
    assert_eq!(
        require_feature(root, Vendor::Gdfl, FEATURE_CENSUS_V4),
        Ok(true)
    );
    let text = std::fs::read_to_string(&at).expect("grown");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[1],
        format!("requires=census-v9 vendor=gd{REQUIRES_VOID}"),
        "{text}"
    );
    assert!(
        lines[2].starts_with("requires=census-v4 vendor=gdfl since="),
        "{text}"
    );
    assert_eq!(lines.len(), 3, "the format line was already there: {text}");
    assert_eq!(reader_refusal(root, Vendor::Gdfl), None);

    let format = say("format=store-requires-v2\n").expect("another format");
    assert!(
        format.contains("declares format \"store-requires-v2\""),
        "{format}"
    );
    for shapeless in ["census-v4 gdfl\n", "requires=census-v4\n", "vendor=gdfl\n"] {
        let why = say(shapeless).expect("no known shape");
        assert!(
            why.contains("is no requirement this build can read"),
            "{why}"
        );
    }
    let long = "\n".repeat(usize::try_from(REQUIRES_CAP).expect("small") + 1);
    let why = say(&long).expect("too long");
    assert!(why.contains("is longer than 65536 bytes"), "{why}");

    std::fs::remove_file(&at).expect("removed");
    std::fs::create_dir(&at).expect("a directory at the name");
    let why = reader_refusal(root, Vendor::Gdfl).expect("not a file");
    assert!(why.contains("is not a regular file"), "{why}");
    assert!(!why.contains("  "), "one sentence, no run of spaces: {why}");
    assert!(
        require_feature(root, Vendor::Gdfl, FEATURE_CENSUS_V4).is_err(),
        "and a writer says so too"
    );

    // A STORE ROOT NO HOST CAN NAME HOLDS NO REQUIREMENTS FILE, so it requires
    // nothing here: the census's own read then refuses the root in its own
    // words (`a_census_that_cannot_be_measured_stops_the_run`), which a
    // "could not be opened" from this file used to pre-empt.
    assert_eq!(
        reader_refusal(&root.join("A".repeat(500)), Vendor::Gdfl),
        None,
        "a name past the host's limit is no requirements file"
    );
    assert_eq!(
        reader_refusal(&elsewhere_file(root).join("under-a-file"), Vendor::Gdfl),
        None,
        "a file where a directory belongs is no requirements file"
    );

    // A LINK AT THE NAME IS NOT FOLLOWED to a file that requires nothing: the
    // rule a store states is its own file's, never one a link points at.
    std::fs::remove_dir(&at).expect("removed");
    let elsewhere = root.join("elsewhere");
    std::fs::write(&elsewhere, format!("{REQUIRES_FORMAT}\n")).expect("written");
    std::os::unix::fs::symlink(&elsewhere, &at).expect("linked");
    let why = reader_refusal(root, Vendor::Gdfl).expect("a link is refused");
    assert!(why.contains("could not be opened"), "{why}");

    // A FIFO AT THE NAME IS REFUSED AT ONCE, never waited on: before this the
    // open blocked until a writer came, and a census load hung with it.
    // `mkfifo` is the external program, run from a test only.
    std::fs::remove_file(&at).expect("removed");
    let made = std::process::Command::new("mkfifo")
        .arg(&at)
        .status()
        .expect("mkfifo ran");
    assert!(made.success(), "mkfifo made the pipe");
    let asked = std::time::Instant::now();
    let why = reader_refusal(root, Vendor::Gdfl).expect("a pipe is refused");
    assert!(why.contains("is not a regular file"), "{why}");
    assert!(
        asked.elapsed() < std::time::Duration::from_secs(5),
        "answered without waiting for a writer"
    );
}

/// A regular file under `root`, for a path that runs THROUGH a file.
fn elsewhere_file(root: &Path) -> PathBuf {
    let at = root.join("a-file-not-a-directory");
    std::fs::write(&at, b"x").expect("written");
    at
}
