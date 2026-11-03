#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

//! BASE-COV-PULL-PS2 (D-7336): the arms of `census_store` its other test
//! modules leave unreached, each driven by the input that reaches it and held
//! to what it says or does. Every fixture is invented: keys, counts, stamps,
//! prices and journal lines.

use std::os::unix::fs::{FileExt as _, MetadataExt as _};
use std::sync::atomic::{AtomicU32, Ordering};

use super::*;
use crate::manifest::{
    Closes, Entry, EntryKey, Held, Manifest, epoch_path, manifest_path, vendor_dir,
};
use crate::session::Day;

/// A scratch store of this test's own, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-census-ps2-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("manifest")).expect("a scratch store");
        Self(root)
    }

    fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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

fn row(n: u32, rows: u64) -> Held {
    row_at(
        n,
        ym(2019, 11),
        rows,
        10,
        10 + i64::try_from(rows).expect("rows"),
    )
}

/// An INDEX row of the same invented month: a key of another segment.
fn index_row(rows: u64) -> Held {
    let mut held = row(90, rows);
    held.entry.key.segment = Segment::Index;
    held.entry.key.contract = None;
    held
}

/// A partition of capacity 2 holding one key written twice: full.
fn full_at_two() -> Manifest {
    let mut census = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 2).expect("genesis");
    census.record_held(row(0, 1)).expect("one");
    census.record_held(row(0, 2)).expect("two");
    census
}

/// Writes `census` as `epoch` of the gdfl FNO partition.
fn put(root: &Path, census: &Manifest, epoch: u32) -> PathBuf {
    let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, epoch);
    fs::create_dir_all(path.parent().expect("a parent dir")).expect("dir");
    fs::write(&path, census.image()).expect("write");
    path
}

/// A gdfl FNO epoch 1 of capacity 4 whose newest commit was cut off, so it
/// loads DEGRADED at its older generation (key 0 alone).
fn degraded_epoch(root: &Path) -> PathBuf {
    let mut census = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 4).expect("genesis");
    census.record_held(row(0, 1)).expect("one");
    let path = put(root, &census, 1);
    let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
    let day = cycle(&target, &[row(1, 1)], Mode::AllOrNothing).expect("an import day appends");
    assert_eq!((day.epoch, day.compacted), (Some(1), None), "an append");
    let appended = fs::read(&path).expect("epoch 1");
    fs::write(&path, &appended[..appended.len() - 128]).expect("torn");
    path
}

fn span_from(year: u16, month: u8, day: u8) -> Span {
    Span {
        from: Day::new(year, month, day).expect("a day"),
        to: None,
    }
}

// ═══ Withdrawal and the epoch install ═══════════════════════════════════════

/// A KEY IS NEVER WITHDRAWN FROM A SINGLE FILE: only a migrated census has
/// epochs to install the withdrawal as, so the single file is refused by name
/// with the verb that migrates it, and no byte moves.
#[test]
fn a_withdrawal_from_a_single_file_census_is_refused_before_a_byte() {
    let scratch = Scratch::new("WD-SINGLE");
    let root = scratch.root();
    let _ = record(root, Vendor::Zerodha, &[row(0, 1)]).expect("a single-file census");
    let single = manifest_path(root, Vendor::Zerodha);
    let before = fs::read(&single).expect("bytes");
    let why = withdraw(root, Vendor::Zerodha, Segment::Fno, &|_| true).expect_err("refused");
    assert!(why.contains("zerodha's census is a single file"), "{why}");
    assert!(why.contains("census-migrate zerodha"), "{why}");
    assert!(why.contains(&single.display().to_string()), "{why}");
    assert_eq!(fs::read(&single).expect("bytes"), before, "nothing written");
    assert!(fs::symlink_metadata(vendor_dir(root, Vendor::Zerodha)).is_err());
}

/// THE NEXT EPOCH OF A SINGLE FILE DOES NOT EXIST, whoever asks for it: the
/// install refuses a single-file target by name and makes no directory.
#[test]
fn a_single_file_target_is_never_given_a_next_epoch() {
    let scratch = Scratch::new("NEXT-SINGLE");
    let root = scratch.root();
    let target = Target::single(root, Vendor::Zerodha);
    let lock = target.lock().expect("the lock");
    let loaded = target.load().expect("an absent census loads empty");
    let next = Manifest::partition_genesis(Vendor::Zerodha, Segment::Fno, 4).expect("genesis");
    let why = install_next_epoch(&lock, &target, &loaded, &next, "full").expect_err("refused");
    assert!(
        why.contains("a single-file census is never compacted by a write"),
        "{why}"
    );
    assert!(why.contains("zerodha.man"), "{why}");
    assert!(fs::symlink_metadata(vendor_dir(root, Vendor::Zerodha)).is_err());
}

/// A RUN THAT LOADED A PARTITION DEGRADED INSTALLS ITS NEXT EPOCH even when
/// nothing filled it: the repair goes to a new epoch, never over the damaged
/// one; and a run that neither filled nor repaired appends nothing at all.
#[test]
fn a_degraded_partition_s_run_install_is_a_repair_epoch() {
    let scratch = Scratch::new("RUN-REPAIR");
    let root = scratch.root();
    let first = put(root, &full_at_two(), 1);
    let before = fs::read(&first).expect("epoch 1");
    let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
    let lock = target.lock().expect("lock");
    let loaded = target.load().expect("loads");
    install_run(&lock, &target, &loaded, &[], false, false).expect("nothing to publish");
    assert_eq!(current_epoch(root, Vendor::Gdfl, Segment::Fno), Ok(Some(1)));
    install_run(&lock, &target, &loaded, &[], true, false).expect("repaired");
    assert_eq!(current_epoch(root, Vendor::Gdfl, Segment::Fno), Ok(Some(2)));
    let next = read_epoch(
        &epoch_path(root, Vendor::Gdfl, Segment::Fno, 2),
        Vendor::Gdfl,
        Segment::Fno,
    )
    .expect("epoch 2");
    assert_eq!(
        (next.keys(), next.entries()),
        (1, 1),
        "folded to one row a key"
    );
    assert_eq!(fs::read(&first).expect("epoch 1"), before, "kept");
}

/// A SEGMENT DIRECTORY THE INSTALL MUST MAKE AND THAT HAS NO PARENT is made
/// alone, and its refusal names it.
#[test]
fn a_segment_directory_with_no_parent_is_made_alone_or_named() {
    let mut never = |_: WholeStep| -> std::io::Result<()> { Ok(()) };
    let why = write_whole_with(
        (Path::new(""), true),
        Path::new("000001.man.tmp"),
        Path::new("000001.man"),
        b"invented",
        &mut never,
    )
    .expect_err("an empty name is no directory");
    assert!(why.contains("could not be made"), "{why}");
}

// ═══ One cycle ═══════════════════════════════════════════════════════════════

/// AN ALL-OR-NOTHING BATCH REFUSED AFTER ITS COMPACTION says the epoch the
/// compaction installed holds the totals of the one it replaced, so nobody
/// reads the new epoch as the batch's.
#[test]
fn an_all_or_nothing_refusal_after_a_compaction_names_the_epoch_it_installed() {
    let scratch = Scratch::new("AON-COMPACT");
    let root = scratch.root();
    let first = put(root, &full_at_two(), 1);
    let before = fs::read(&first).expect("epoch 1");
    let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
    let why = cycle(&target, &[row(0, 1)], Mode::AllOrNothing).expect_err("refused");
    assert!(why.contains("records 1 rows where 2 were already"), "{why}");
    assert!(why.contains("was installed by a compaction first"), "{why}");
    assert!(
        why.contains(&format!("epoch {}", epoch_name(2))),
        "the installed epoch by name: {why}"
    );
    assert!(why.contains("nothing of this batch of 1 row(s)"), "{why}");
    assert_eq!(current_epoch(root, Vendor::Gdfl, Segment::Fno), Ok(Some(2)));
    assert_eq!(fs::read(&first).expect("epoch 1"), before, "kept");
}

/// A RECOUNT OF A DEGRADED PARTITION THAT FINDS NOTHING NEW still installs the
/// repair it carried, whole, as the next epoch.
#[test]
fn a_recount_with_nothing_new_still_installs_the_repair_it_carried() {
    let scratch = Scratch::new("REPAIR-NOTHING-NEW");
    let root = scratch.root();
    let torn = degraded_epoch(root);
    let torn_bytes = fs::read(&torn).expect("torn");
    let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
    let done = cycle(&target, &[row(0, 1)], Mode::PerKey).expect("repairs");
    assert!(done.degraded.is_some(), "it says it loaded degraded");
    assert_eq!((done.recorded, done.exact), (0, 1), "nothing new");
    assert_eq!(done.compacted.as_ref().map(|c| c.why), Some("repair"));
    assert_eq!(done.epoch, Some(2));
    assert_eq!(fs::read(&torn).expect("epoch 1"), torn_bytes, "kept");
    // And now the census is whole: the same recount writes nothing.
    let same = cycle(&target, &[row(0, 1)], Mode::PerKey).expect("exact");
    assert_eq!((same.compacted, same.epoch), (None, Some(2)));
}

/// A BATCH OVER TWO PARTITIONS, ONE REFUSED: the refusal names the partition
/// that did publish, which a refusal never undoes.
#[test]
fn a_refused_partition_names_the_partitions_that_published() {
    let scratch = Scratch::new("TWO-PARTS");
    let root = scratch.root();
    let _ = put(root, &full_at_two(), 1);
    let why = record(root, Vendor::Gdfl, &[index_row(3), row(0, 1)]).expect_err("FNO refuses");
    let index = epoch_path(root, Vendor::Gdfl, Segment::Index, 1);
    assert!(
        why.contains(&format!(" (published: {})", index.display())),
        "{why}"
    );
    assert!(why.contains("records 1 rows where 2 were already"), "{why}");
    assert_eq!(
        current_epoch(root, Vendor::Gdfl, Segment::Index),
        Ok(Some(1))
    );
}

// ═══ The kept census ═════════════════════════════════════════════════════════

fn pace() -> Pace {
    Pace {
        read: CensusRead::Kept,
        entries: 3,
        load_micros: 1,
        total_micros: 2,
    }
}

fn loaded_at(path: PathBuf) -> Loaded {
    Loaded {
        census: Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 2).expect("genesis"),
        epoch: Some(1),
        path,
        exists: true,
        on_disk: 0,
    }
}

/// A CENSUS IS KEPT ONLY WHEN ITS FILE IS A REGULAR FILE IT CAN STAMP: a
/// directory at the name, and a name with nothing at it, are each said by
/// their own word and keep nothing; the last cycle's cost is kept either way.
#[test]
fn a_census_is_kept_only_when_its_file_can_be_stamped() {
    let scratch = Scratch::new("KEPT-PUT");
    let root = scratch.root();
    let target = Target::partition(root, Vendor::Gdfl, Segment::Fno);
    let mut kept = Kept::new();
    assert_eq!(kept.last(), None, "no cycle yet");
    let dir = root.join("a-directory");
    fs::create_dir_all(&dir).expect("dir");
    assert_eq!(
        kept.put(&target, loaded_at(dir), true, pace()),
        Err("not_regular")
    );
    assert_eq!(
        kept.put(&target, loaded_at(root.join("nothing-here")), true, pace()),
        Err("unmeasured")
    );
    assert!(kept.is_empty());
    assert_eq!(kept.last(), Some(pace()), "the cost is said either way");
    assert_eq!(
        kept.put(&target, loaded_at(root.join("nothing-here")), false, pace()),
        Err("published_whole")
    );
    let file = root.join("a-file");
    fs::write(&file, b"invented").expect("file");
    assert_eq!(kept.put(&target, loaded_at(file), true, pace()), Ok(()));
    assert_eq!(kept.len(), 1);
}

/// A KEPT CENSUS WHOSE FILE IS GONE, OR WHOSE HEADER WAS REWRITTEN IN PLACE AT
/// THE SAME LENGTH, is no longer the file: each is said by its own word.
#[test]
fn a_kept_census_whose_file_is_gone_or_whose_header_moved_is_not_current() {
    let scratch = Scratch::new("KEPT-MOVED");
    let root = scratch.root();
    let target = Target::single(root, Vendor::Zerodha);
    let gone = KeptCensus {
        loaded: Loaded {
            census: Manifest::open_image(Vendor::Zerodha, &[]).expect("empty"),
            epoch: None,
            path: root.join("manifest").join("gone.man"),
            exists: true,
            on_disk: 0,
        },
        stamp: Stamp {
            dev: 0,
            ino: 0,
            len: 0,
        },
    };
    assert_eq!(gone.still_current(&target), Ok(Err("unopened")));

    let _ = record(root, Vendor::Zerodha, &[row(0, 1)]).expect("a census file");
    let loaded = target.load().expect("loads");
    let path = loaded.path.clone();
    let stamp = Stamp::of(&fs::symlink_metadata(&path).expect("stat"));
    let held = KeptCensus { loaded, stamp };
    assert_eq!(held.still_current(&target), Ok(Ok(())), "the file it kept");
    let len = fs::metadata(&path).expect("len").len();
    let file = fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("in place");
    file.write_all_at(&vec![0_u8; HEADER_REGION_LEN], 0)
        .expect("the header region rewritten");
    drop(file);
    assert_eq!(fs::metadata(&path).expect("len").len(), len, "same length");
    assert_eq!(held.still_current(&target), Ok(Err("header_moved")));
}

/// A RUN'S MEMORY KEEPS NOTHING A COMPACTION OR A DEGRADED LOAD PUBLISHED
/// WHOLE: the next day reads the census whole once more.
#[test]
fn a_kept_cycle_that_compacted_or_loaded_degraded_keeps_nothing() {
    let scratch = Scratch::new("KEPT-WHOLE");
    let root = scratch.root();
    let _ = put(root, &full_at_two(), 1);
    let mut kept = Kept::new();
    let done = record_kept(&mut kept, root, Vendor::Gdfl, &[row(1, 1)]).expect("compacts");
    assert!(done[0].compacted.is_some() && done[0].degraded.is_none());
    assert!(
        kept.is_empty(),
        "a compacted census is read whole next time"
    );
    assert!(kept.last().is_some());

    let other = Scratch::new("KEPT-DEGRADED");
    let root = other.root();
    let _ = degraded_epoch(root);
    let mut kept = Kept::new();
    let done = record_kept(&mut kept, root, Vendor::Gdfl, &[row(2, 1)]).expect("repairs");
    assert!(done[0].degraded.is_some());
    assert!(kept.is_empty(), "a degraded census is read whole next time");
}

// ═══ Reading a vendor, and the census verbs' hold ═══════════════════════════

/// A PARTITIONED VENDOR ANSWERS ONLY THE SEGMENTS THAT HAVE AN EPOCH, in
/// segment order, and nothing for the others.
#[test]
fn a_partitioned_vendor_reads_only_the_segments_that_have_an_epoch() {
    let scratch = Scratch::new("READ-ONE");
    let root = scratch.root();
    let path = put(root, &full_at_two(), 1);
    let read = read_vendor(root, Vendor::Gdfl).expect("reads");
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].segment, Some(Segment::Fno));
    assert_eq!(read[0].epoch, Some(1));
    assert_eq!(read[0].path, path);
    assert_eq!(read[0].census.keys(), 1);
}

/// A CENSUS VERB OF A VENDOR WHOSE IMPORTS TAKE NO STORE LOCK HOLDS NOTHING,
/// and releasing nothing is nothing.
#[test]
fn a_census_verb_of_a_vendor_without_a_store_lock_holds_nothing() {
    let scratch = Scratch::new("NO-HOLD");
    let root = scratch.root();
    for vendor in [Vendor::Zerodha, Vendor::Dhan] {
        let hold = census_hold(root, vendor, "census-migrate").expect("no hold to take");
        assert!(hold.is_none(), "{vendor:?} takes no store lock");
        release_hold(hold);
    }
    assert!(
        census_hold(root, Vendor::Gdfl, "census-migrate")
            .expect("the gdfl store lock")
            .is_some(),
        "gdfl's census verbs hold the store lock"
    );
}

/// A CENSUS VERB WHOSE HOLD WILL NOT RELEASE SAYS SO AT `error`, naming the
/// run, and its own result stands.
#[test]
fn a_census_hold_that_will_not_release_is_said_at_error() {
    crate::support::where_permission_binds(
        "ingest::census_store::ps2_tests::a_census_hold_that_will_not_release_is_said_at_error",
        hold_release_body,
    );
}

fn hold_release_body() {
    let home = Scratch::new("HOLD-SINK");
    let sink = telemetry::install(
        &telemetry::Config::new(home.root()).with_min_level(telemetry::Level::Trace),
    )
    .expect("the only install in this child");
    let scratch = Scratch::new("HOLD-FAULT");
    let root = scratch.root();
    let hold = census_hold(root, Vendor::Gdfl, "census-migrate")
        .expect("taken")
        .expect("a gdfl hold");
    let run = hold.run;
    // A directory at the run's stop name: the release cannot remove it.
    let stop = hold.locks.stop_path();
    fs::create_dir_all(stop.join("kept")).expect("a directory at the stop name");
    let before = sink.health().next_seq;
    release_hold(Some(hold));
    let said: Vec<telemetry::Record> = telemetry::tail(
        home.root(),
        sink.keep_files(),
        &telemetry::Query::last(telemetry::MAX_LIMIT).from_target(TARGET),
    )
    .records
    .into_iter()
    .filter(|record| record.seq >= before)
    .collect();
    let line = said
        .iter()
        .find(|record| record.message == "census verb lock not released")
        .expect("the release's fault is said");
    assert_eq!(line.level, telemetry::Level::Error);
    assert_eq!(
        line.field("run"),
        Some(&telemetry::OwnedValue::Int(
            i64::try_from(run).expect("a run id")
        ))
    );
    let Some(telemetry::OwnedValue::Str(why)) = line.field("why") else {
        panic!("no why: {line:?}");
    };
    assert!(why.contains(&stop.display().to_string()), "{why}");
    // The store lock itself was released: the next verb takes it.
    assert!(
        census_hold(root, Vendor::Gdfl, "census-migrate")
            .expect("free again")
            .is_some()
    );
}

// ═══ The migration's staging ════════════════════════════════════════════════

/// A SPLIT PART WITH NO SEGMENT IS REFUSED BEFORE A FILE IS STAGED: a
/// partition is a segment's, and nothing is renamed into place.
#[test]
fn a_split_part_without_a_segment_is_refused_before_a_file() {
    let scratch = Scratch::new("STAGE-NOSEG");
    let root = scratch.root();
    let staging = staging_dir(root, Vendor::Gdfl);
    let single = Manifest::open_image(Vendor::Gdfl, &[]).expect("an empty single file");
    let why = stage(&staging, Vendor::Gdfl, &[single]).expect_err("refused");
    assert!(
        why.contains("a split produced a census with no segment"),
        "{why}"
    );
    assert!(why.contains(&staging.display().to_string()), "{why}");
    assert_eq!(fs::read_dir(&staging).expect("made").count(), 0, "no file");
}

// ═══ What the store requires of its reader ══════════════════════════════════

/// A REQUIREMENT UNDER A `manifest` THAT IS A FILE is refused in the host's
/// words with its path: the read-only look meets `ENOTDIR`, which is neither
/// "nothing there" nor a link.
#[test]
fn a_requirement_under_a_file_where_manifest_belongs_is_refused_in_the_host_s_words() {
    let scratch = Scratch::new("REQ-NOTDIR");
    let root = scratch.root();
    fs::remove_dir(root.join("manifest")).expect("the scratch manifest dir");
    fs::write(root.join("manifest"), b"invented: a file, not a directory").expect("file");
    let why = require_feature(root, Vendor::Gdfl, FEATURE_CENSUS_V4).expect_err("refused");
    assert!(
        why.contains(&requires_path(root).display().to_string()),
        "{why}"
    );
    assert!(why.contains("ot a directory"), "the host's words: {why}");
    assert!(!why.contains("symbolic link"), "{why}");
}

// ═══ The seal ════════════════════════════════════════════════════════════════

/// A FILE OF THE STUB'S LENGTH IS A STUB ONLY BY ITS MAGIC AND ITS VERSION:
/// zeros are not, the magic alone is not, the stub is.
#[test]
fn a_file_of_the_stub_s_length_is_a_stub_only_by_its_magic_and_version() {
    let scratch = Scratch::new("STUB-BYTES");
    let at = scratch.root().join("manifest").join("zerodha.man");
    fs::write(&at, vec![0_u8; STUB_LEN]).expect("zeros");
    assert!(!holds_sealed_stub(&at), "zeros are no stub");
    let mut magic_only = vec![0_u8; STUB_LEN];
    magic_only[..8].copy_from_slice(b"BRUTEXMS");
    fs::write(&at, &magic_only).expect("magic");
    assert!(!holds_sealed_stub(&at), "the magic without the version");
    fs::write(&at, sealed_stub(Vendor::Zerodha)).expect("the stub");
    assert!(holds_sealed_stub(&at));
}

/// ONE INODE NUMBER ON TWO DEVICES IS TWO FILES: two mount roots that share
/// an inode number (procfs and sysfs on Linux, two APFS volumes on macOS) are
/// told apart by their device.
#[test]
fn one_inode_number_on_two_devices_is_two_files() {
    let candidates = [
        "/",
        "/proc",
        "/sys",
        "/dev",
        "/dev/shm",
        "/run",
        "/System/Volumes/Data",
        "/System/Volumes/VM",
        "/System/Volumes/Preboot",
    ];
    let found: Vec<(&str, fs::Metadata)> = candidates
        .iter()
        .filter_map(|path| fs::symlink_metadata(path).ok().map(|meta| (*path, meta)))
        .collect();
    let (one, other) = found
        .iter()
        .enumerate()
        .find_map(|(at, (path, meta))| {
            found
                .iter()
                .skip(at + 1)
                .find(|(_, them)| them.ino() == meta.ino() && them.dev() != meta.dev())
                .map(|(_, them)| (*path, them))
        })
        .expect("two mount roots with one inode number on two devices");
    assert!(
        !same_inode(Path::new(one), other),
        "{one}: the same number on another device is another file"
    );
    let itself = fs::symlink_metadata(one).expect("stat");
    assert!(same_inode(Path::new(one), &itself));
}

/// A MIGRATION RECEIPT THAT NAMES NO `legacy_bytes` SEALS NOTHING: what the
/// stub would replace cannot be checked against what the migration read.
#[test]
fn a_seal_whose_receipt_names_no_legacy_bytes_writes_nothing() {
    let scratch = Scratch::new("SEAL-NOBYTES");
    let root = scratch.root();
    let dir = vendor_dir(root, Vendor::Zerodha);
    fs::create_dir_all(&dir).expect("migrated");
    fs::write(dir.join(MIGRATION_RECEIPT), "format=invented\n").expect("receipt");
    let why = seal_legacy(root, Vendor::Zerodha).expect_err("refused");
    assert!(
        why.contains("names no legacy_bytes; nothing was written"),
        "{why}"
    );
    assert!(fs::symlink_metadata(superseded_path(root, Vendor::Zerodha)).is_err());
    assert!(fs::symlink_metadata(manifest_path(root, Vendor::Zerodha)).is_err());
}

/// A DIRECTORY AT THE SINGLE FILE'S NAME IS NO CENSUS TO SEAL, even at the
/// recorded length: refused, nothing linked.
#[test]
fn a_directory_at_the_single_file_s_name_is_not_sealed() {
    let scratch = Scratch::new("SEAL-DIR");
    let root = scratch.root();
    let legacy = manifest_path(root, Vendor::Zerodha);
    fs::create_dir_all(legacy.join("inside")).expect("a directory at the name");
    let len = fs::symlink_metadata(&legacy).expect("stat").len();
    let dir = vendor_dir(root, Vendor::Zerodha);
    fs::create_dir_all(&dir).expect("migrated");
    fs::write(dir.join(MIGRATION_RECEIPT), format!("legacy_bytes={len}\n")).expect("receipt");
    let why = seal_legacy(root, Vendor::Zerodha).expect_err("refused");
    assert!(
        why.contains(&format!(
            "is {len} bytes and not a plain file of the {len} bytes the migration recorded"
        )),
        "{why}"
    );
    assert!(fs::symlink_metadata(superseded_path(root, Vendor::Zerodha)).is_err());
}

/// A STUB AT THE SINGLE FILE'S NAME WITH NO SUPERSEDED COPY, whose length is
/// the receipt's, is the census the receipt recorded: it is kept under the
/// superseded name and the stub is installed.
#[test]
fn a_stub_of_the_recorded_length_with_no_superseded_copy_is_kept_and_sealed() {
    let scratch = Scratch::new("SEAL-STUB-ONLY");
    let root = scratch.root();
    let legacy = manifest_path(root, Vendor::Zerodha);
    fs::write(&legacy, sealed_stub(Vendor::Zerodha)).expect("a stub");
    let dir = vendor_dir(root, Vendor::Zerodha);
    fs::create_dir_all(&dir).expect("migrated");
    fs::write(
        dir.join(MIGRATION_RECEIPT),
        format!("legacy_bytes={STUB_LEN}\n"),
    )
    .expect("receipt");
    let seal = seal_legacy(root, Vendor::Zerodha).expect("sealed");
    assert!(seal.wrote);
    assert_eq!(seal.legacy_bytes, STUB_LEN as u64);
    let superseded = superseded_path(root, Vendor::Zerodha);
    assert_eq!(
        fs::read(&superseded).expect("kept"),
        sealed_stub(Vendor::Zerodha)
    );
    assert!(holds_sealed_stub(&legacy));
}

/// A SEAL MADE BEFORE THE REQUIREMENTS FILE EXISTED GAINS ITS LINE when the
/// seal runs again, and says it wrote; a third run writes nothing.
#[test]
fn a_seal_run_again_writes_the_requirement_it_lacks() {
    let scratch = Scratch::new("SEAL-REQ");
    let root = scratch.root();
    let _ = record(root, Vendor::Zerodha, &[row(0, 1)]).expect("a single-file census");
    let migrated = migrate(root, Vendor::Zerodha).expect("migrated and sealed");
    assert!(matches!(migrated.seal, Some(Ok(_))), "{:?}", migrated.seal);
    let requires = requires_path(root);
    fs::remove_file(&requires).expect("the requirement as an older seal left it");
    let again = seal_legacy(root, Vendor::Zerodha).expect("sealed already");
    assert!(again.wrote, "the requirement was written");
    let text = fs::read_to_string(&requires).expect("requires");
    assert!(
        text.contains(&format!("requires={FEATURE_CENSUS_V4} vendor=zerodha ")),
        "{text}"
    );
    let third = seal_legacy(root, Vendor::Zerodha).expect("nothing to do");
    assert!(!third.wrote);
}

/// DRIFT AFTER A SEAL: a superseded census whose length moved since the seal
/// was written after it, and is named.
#[test]
fn a_superseded_census_that_grew_after_the_seal_is_named() {
    let scratch = Scratch::new("DRIFT-SUPERSEDED");
    let root = scratch.root();
    let dir = vendor_dir(root, Vendor::Zerodha);
    fs::create_dir_all(&dir).expect("migrated");
    fs::write(dir.join(MIGRATION_RECEIPT), "legacy_bytes=10\n").expect("receipt");
    let superseded = superseded_path(root, Vendor::Zerodha);
    fs::write(&superseded, b"invented and longer than ten bytes").expect("grown");
    let drift = legacy_drift(root, Vendor::Zerodha).expect("drift");
    assert!(
        drift.contains("the superseded census was written after the seal"),
        "{drift}"
    );
    assert!(
        drift.contains("it was 10 when zerodha was migrated"),
        "{drift}"
    );
}

/// A SEAL RECEIPT THAT NAMES ITS BYTES BUT NO SUPERSEDED NAME IS TORN, and is
/// read as torn, never as whole.
#[test]
fn a_seal_receipt_without_a_superseded_name_is_torn() {
    let scratch = Scratch::new("RECEIPT-TORN");
    let path = scratch.root().join("SEALED");
    fs::write(
        &path,
        format!(
            "{SEAL_RECEIPT_FORMAT}\nsuperseded=\nsuperseded_bytes=5\nreader_since={V4_READER_SINCE}\n"
        ),
    )
    .expect("receipt");
    assert_eq!(
        seal_receipt_kept_bytes(&path).expect("read"),
        (
            SealReceipt::Torn(
                "it names no whole superseded= and superseded_bytes= line".to_owned()
            ),
            None
        )
    );
    fs::write(
        &path,
        format!(
            "{SEAL_RECEIPT_FORMAT}\nsuperseded=kept.man\nsuperseded_bytes=5\nreader_since={V4_READER_SINCE}\n"
        ),
    )
    .expect("receipt");
    assert_eq!(
        seal_receipt_kept_bytes(&path).expect("read"),
        (SealReceipt::Whole, Some(5))
    );
}

/// A SEAL RECEIPT THAT CANNOT BE RENAMED INTO PLACE IS NAMED, with both names.
#[test]
fn a_seal_receipt_that_cannot_be_renamed_into_place_is_named() {
    let scratch = Scratch::new("RECEIPT-RENAME");
    let path = scratch.root().join("SEALED");
    fs::create_dir_all(path.join("inside")).expect("a directory at the receipt's name");
    let why =
        write_seal_receipt(&path, &scratch.root().join("x.superseded"), 7).expect_err("refused");
    assert!(why.contains("could not be renamed to"), "{why}");
    assert!(why.contains("SEALED.tmp"), "{why}");
    assert!(path.join("inside").is_dir(), "the directory is untouched");
}

// ═══ The recount ═════════════════════════════════════════════════════════════

/// Writes `count` invented one-second NIFTY bars of `month` for `vendor`,
/// and returns the month file's path.
fn bars(root: &Path, vendor: Vendor, month: YearMonth, count: i64) -> PathBuf {
    #[expect(clippy::cast_possible_truncation, reason = "the census's own fold")]
    let symbol_id = brutex_core::universe::fnv1a("NIFTY") as u32;
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    })
    .expect("a store path");
    let open = month.ist_bounds_micros().0 + (2 * 86_400 + 555 * 60) * 1_000_000;
    let mut writer = BarFile::open_or_create(root, path, symbol_id).expect("a month");
    for k in 0..count {
        writer
            .append(&[store::format::Bar {
                ts_micros: open + k * 1_000_000,
                open: 3_100_000_000 + k,
                high: 3_100_000_500 + k,
                low: 3_099_999_500 + k,
                close: 3_100_000_000 + k,
                volume: 1 + k,
                open_interest: store::format::OI_NULL,
            }])
            .expect("a bar lands");
    }
    drop(writer);
    path.to_path_buf(root)
}

/// EVERY NAME UNDER `bars/<vendor>/` THAT IS NOT A STORE PATH IS A STRAY,
/// counted and named up to the cap; a month outside the span is seen and not
/// read; an empty month offers no row; and a vendor with no journal and no
/// re-key closes nothing.
#[test]
fn a_recount_walk_counts_every_stray_and_reads_only_its_span() {
    let scratch = Scratch::new("WALK");
    let root = scratch.root();
    let base = root.join(store::path::STORE_ROOT).join("zerodha");
    let good = bars(root, Vendor::Zerodha, ym(2031, 6), 3);
    let empty = bars(root, Vendor::Zerodha, ym(2031, 5), 0);
    let before_span = bars(root, Vendor::Zerodha, ym(2030, 12), 2);
    let rung = good.parent().expect("the rung").to_path_buf();
    let mut strays = vec![
        base.join("NOT-AN-EXCHANGE"),
        base.join("NSE").join("NOT-A-SEGMENT"),
        base.join("NSE").join("INDEX").join("not a symbol"),
        base.join("NSE")
            .join("INDEX")
            .join("NIFTY")
            .join("not-a-rung"),
        base.join("NSE")
            .join("FNO")
            .join("NIFTY")
            .join("2019-12-26-1000000-CE")
            .join("not-a-rung"),
    ];
    for one in &strays {
        fs::create_dir_all(one).expect("a stray directory");
    }
    for name in ["notes.txt", "2031-1.bin", "203-01.bin", "20310-1.bin"] {
        fs::write(rung.join(name), b"invented").expect("a stray file");
        strays.push(rung.join(name));
    }
    // A month's own sibling kinds are not strays, beside no month at all.
    fs::write(rung.join("2031-07.crc"), b"invented").expect("a sibling");
    let found = recount(root, Vendor::Zerodha, span_from(2031, 1, 1), None).expect("recounts");
    assert_eq!(found.not_store_names, strays.len() as u64, "{found:?}");
    let mut named = found.not_store_named.clone();
    named.sort();
    strays.sort();
    assert_eq!(named, strays);
    assert_eq!(found.files_seen, 3, "three months, one before the span");
    assert_eq!(found.files_in_range, 2);
    assert_eq!(found.files_empty, 1, "{}", empty.display());
    assert_eq!(
        found.offered, 1,
        "the good month alone offers a row: {:?}",
        found.unreadable_named
    );
    assert!(found.journal.is_none(), "zerodha keeps no import journal");
    assert_eq!(found.withdrawn, 0);
    assert!(found.clean(), "strays alone leave a recount clean");
    assert!(before_span.is_file());
}

/// MORE STRAYS THAN THE CAP are all counted and only the first fifty named.
#[test]
fn strays_past_the_cap_are_counted_and_not_named() {
    let scratch = Scratch::new("WALK-CAP");
    let root = scratch.root();
    let base = root.join(store::path::STORE_ROOT).join("zerodha");
    for n in 0..=NAMED {
        fs::create_dir_all(base.join(format!("STRAY-{n:03}"))).expect("a stray");
    }
    let found = recount(root, Vendor::Zerodha, span_from(2031, 1, 1), None).expect("recounts");
    assert_eq!(found.not_store_names, NAMED as u64 + 1);
    assert_eq!(found.not_store_named.len(), NAMED);
    assert_eq!(found.cycles.len(), 0, "nothing offered, nothing written");
}

/// A BARS ROOT THAT IS A FILE CANNOT BE LISTED, and the recount says which.
#[test]
fn a_vendor_bars_directory_that_is_a_file_is_refused_by_name() {
    let scratch = Scratch::new("WALK-FILE");
    let root = scratch.root();
    let base = root.join(store::path::STORE_ROOT);
    fs::create_dir_all(&base).expect("bars");
    fs::write(base.join("zerodha"), b"invented").expect("a file");
    let why = recount(root, Vendor::Zerodha, span_from(2031, 1, 1), None).expect_err("refused");
    assert!(why.contains("could not be listed"), "{why}");
    assert!(why.contains("zerodha"), "{why}");
}

/// A RECOUNT WHOSE CENSUS WILL NOT LOAD says how far it got and that no
/// import day was closed.
#[test]
fn a_recount_whose_census_will_not_load_says_how_far_it_got() {
    let scratch = Scratch::new("WALK-CENSUS");
    let root = scratch.root();
    let _ = bars(root, Vendor::Zerodha, ym(2031, 6), 3);
    let epoch = epoch_path(root, Vendor::Zerodha, Segment::Index, 1);
    fs::create_dir_all(epoch.parent().expect("dir")).expect("dir");
    fs::write(&epoch, b"invented: no census").expect("garbage at an epoch");
    let why = recount(root, Vendor::Zerodha, span_from(2031, 1, 1), None).expect_err("refused");
    assert!(why.contains(&epoch.display().to_string()), "{why}");
    assert!(
        why.contains(
            "(the recount had read 1 file(s) in range, 0 unreadable; no import day was closed)"
        ),
        "{why}"
    );
}

/// A GDFL RECOUNT WHOSE JOURNAL CANNOT BE READ says the census was
/// recounted and published, and how many rows it offered.
#[test]
fn a_gdfl_recount_whose_journal_cannot_be_read_says_the_census_stands() {
    let scratch = Scratch::new("WALK-JOURNAL");
    let root = scratch.root();
    // Bytes that are no text: the runs ledger reads the journal for its ids
    // as bytes, the closing reads it as text.
    let journal = journal_with(root, "");
    fs::write(&journal, b"run 5 source=invented\n\xff\xfe\n").expect("no text");
    let why = recount(root, Vendor::Gdfl, span_from(2031, 1, 1), None).expect_err("refused");
    assert!(why.contains(&journal.display().to_string()), "{why}");
    assert!(
        why.contains("(the census was recounted and published: 0 row(s) offered)"),
        "{why}"
    );
}

/// A RECOUNT IS CLEAN ONLY WITH NO DAMAGED SLOT AND NO REFUSED ROW.
#[test]
fn a_recount_with_a_refused_row_is_not_clean() {
    let scratch = Scratch::new("CLEAN");
    let root = scratch.root();
    let target = Target::single(root, Vendor::Zerodha);
    let refusing = cycle(&target, &[row(0, 2), row(0, 1)], Mode::PerKey).expect("published");
    assert_eq!(refusing.refused.len(), 1);
    let mut found = Recount::default();
    assert!(found.clean());
    found.cycles.push(refusing);
    assert_eq!(found.refused(), 1);
    assert!(!found.clean());
}

// ═══ The journal ═════════════════════════════════════════════════════════════

fn checked<'a>(held: &'a [Held], refused: &'a HashSet<YearMonth>) -> Checked<'a> {
    Checked {
        held,
        unreadable: &[],
        refused,
        strays: 0,
    }
}

fn journal_with(root: &Path, text: &str) -> PathBuf {
    let journal = crate::gdfl_import::journal_path(root);
    fs::create_dir_all(journal.parent().expect("imports")).expect("imports");
    fs::write(&journal, text).expect("journal");
    journal
}

/// A JOURNAL THAT CANNOT BE READ IS REFUSED BY ITS PATH, and one whose line no
/// verb of this build reads is refused naming the line.
#[test]
fn a_journal_that_cannot_be_read_or_holds_a_foreign_line_is_refused() {
    let scratch = Scratch::new("JOURNAL-REFUSED");
    let root = scratch.root();
    let refused = HashSet::new();
    let span = span_from(2019, 11, 1);
    let journal = crate::gdfl_import::journal_path(root);
    fs::create_dir_all(journal.join("inside")).expect("a directory at the journal");
    let why = close_journal(root, span, None, &checked(&[], &refused)).expect_err("refused");
    assert!(why.contains(&journal.display().to_string()), "{why}");

    let other = Scratch::new("JOURNAL-FOREIGN");
    let root = other.root();
    let _ = journal_with(root, "begin options 2019-11-14 * extra\n");
    let why = close_journal(root, span, None, &checked(&[], &refused)).expect_err("refused");
    assert!(why.contains("not a journal line this build knows"), "{why}");
    assert!(why.contains("begin options 2019-11-14 * extra"), "{why}");
}

/// A REFUSED DAY AND AN INCOMPLETE LINE WHOSE DAY IS NO DAY CLOSE NOTHING and
/// write nothing.
#[test]
fn a_refused_day_and_a_line_whose_day_is_no_day_close_nothing() {
    let scratch = Scratch::new("JOURNAL-NOTHING");
    let root = scratch.root();
    let text = "refused options 2019-11-14 * why=invented\n\
                incomplete options 2019-13-45 * definition=2 files=1 failures=1\n";
    let journal = journal_with(root, text);
    let refused = HashSet::new();
    let done =
        close_journal(root, span_from(2019, 11, 1), None, &checked(&[], &refused)).expect("reads");
    assert!(done.closed.is_empty() && done.left_open.is_empty() && done.run.is_none());
    assert_eq!(
        fs::read_to_string(&journal).expect("journal"),
        text,
        "unchanged"
    );
}

/// IST micros of 10:00 on `day` of November 2019.
fn nov(day: i64) -> i64 {
    let (start, _) = ym(2019, 11).ist_bounds_micros();
    start + ((day - 1) * 86_400 + 36_000) * 1_000_000
}

/// A RUN ID HELD FAR AHEAD OF THE CLOCK DOES NOT PUSH THIS RUN'S ID: the walk
/// past `started` stops at the first gap.
#[test]
fn a_run_id_far_ahead_does_not_push_the_recount_s_id() {
    let scratch = Scratch::new("JOURNAL-AHEAD");
    let root = scratch.root();
    let ahead = 9_000_000_000_u64;
    let journal = journal_with(
        root,
        &format!(
            "run {ahead} source=invented\n\
             incomplete options 2019-11-14 * definition=2 files=2 seconds=10 failures=1\n"
        ),
    );
    let held = [
        row_at(0, ym(2019, 11), 5, nov(1), nov(14)),
        row_at(1, ym(2019, 11), 5, nov(14), nov(15)),
    ];
    let refused = HashSet::new();
    let span = Span {
        from: Day::new(2019, 11, 14).expect("a day"),
        to: Some(Day::new(2019, 11, 30).expect("a day")),
    };
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_secs();
    let done = close_journal(root, span, None, &checked(&held, &refused)).expect("closes");
    let run = done.run.expect("a run");
    assert!(run >= before && run < ahead, "{run}");
    let text = fs::read_to_string(&journal).expect("journal");
    assert!(
        text.contains(&format!(
            "recounted options 2019-11-14 * definition=2 months=2 run={run}\n"
        )),
        "{text}"
    );
}

/// A CLOSED DAY WHOSE KEY THE STATES DO NOT HOLD AS INCOMPLETE carries
/// `definition=0`, the line's default.
#[test]
fn a_closed_day_without_an_incomplete_state_carries_definition_zero() {
    let mut states = BTreeMap::new();
    let _ = states.insert("options 2019-11-14 *".to_owned(), KeyState::Done);
    let closed = [Closed {
        key: "options 2019-11-14 *".to_owned(),
        months: 3,
        files: None,
    }];
    let mut lines = String::new();
    recounted_lines(&mut lines, 7, false, &closed, &states);
    assert_eq!(
        lines,
        "recounted options 2019-11-14 * definition=0 months=3 run=7\n"
    );
}

/// A JOURNAL DAY IS `YYYY-MM-DD` EXACTLY.
#[test]
fn a_journal_day_is_spelled_exactly() {
    assert_eq!(parse_day("2019-11-14"), Day::new(2019, 11, 14).ok());
    for wrong in [
        "19-11-14",
        "2019-1-14",
        "2019-11-4",
        "2019-11-14-1",
        "2019-11",
        "2019-02-30",
    ] {
        assert_eq!(parse_day(wrong), None, "{wrong}");
    }
}

/// AN EMPTY JOURNAL TAKES AN APPEND WITH NO TORN MARK: there is no last line
/// to close.
#[test]
fn an_empty_journal_takes_an_append_with_no_torn_mark() {
    let scratch = Scratch::new("JOURNAL-EMPTY");
    let root = scratch.root();
    let journal = journal_with(root, "");
    append_journal(&journal, "run 1 source=invented\n").expect("appends");
    assert_eq!(
        fs::read_to_string(&journal).expect("journal"),
        "run 1 source=invented\n"
    );
    append_journal(&journal, "run 2 source=invented\n").expect("appends");
    assert_eq!(
        fs::read_to_string(&journal).expect("journal"),
        "run 1 source=invented\nrun 2 source=invented\n"
    );
}

// ═══ Between two host calls (the seams, D-7336) ═════════════════════════════

/// A SEGMENT DIRECTORY GONE BETWEEN ITS LOOK AND ITS LISTING holds no epoch,
/// and one replaced by a file there is no directory to list: neither is a
/// refusal, as neither is when the look itself finds it.
#[test]
fn a_segment_directory_gone_between_its_look_and_its_listing_holds_no_epoch() {
    let scratch = Scratch::new("SEAM-LOOKED");
    let dir = scratch.root().join("segment");
    fs::create_dir_all(&dir).expect("dir");
    seam::plant("segment.looked", |at| fs::remove_dir(at).expect("removed"));
    assert_eq!(current_epoch_in(&dir), Ok(None), "gone");
    fs::create_dir_all(&dir).expect("dir");
    seam::plant("segment.looked", |at| {
        fs::remove_dir(at).expect("removed");
        fs::write(at, b"invented").expect("a file at the name");
    });
    assert_eq!(current_epoch_in(&dir), Ok(None), "a file there now");
}

/// AN EPOCH THAT GROWS BETWEEN ITS MEASURE AND ITS READ is refused past its
/// cap, with the bytes the read found: the read is bounded, never trusted to
/// the length the handle had.
#[test]
fn an_epoch_that_grows_between_its_measure_and_its_read_is_refused_past_its_cap() {
    let scratch = Scratch::new("SEAM-GROWS");
    let path = put(scratch.root(), &full_at_two(), 1);
    let cap = fs::metadata(&path).expect("len").len();
    seam::plant("epoch.measured", |at| {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(at)
            .expect("append");
        file.write_all(&[0_u8; 7]).expect("grown");
    });
    let why = read_epoch(&path, Vendor::Gdfl, Segment::Fno).expect_err("refused");
    assert!(
        why.contains(&format!(
            "holds {} bytes, more than the {cap} this reader takes",
            cap + 1
        )),
        "{why}"
    );
}

/// A KEPT CENSUS CUT BETWEEN ITS STAMP AND ITS HEADER READ is not current:
/// its header could not be read.
#[test]
fn a_kept_census_cut_between_its_stamp_and_its_header_read_is_not_current() {
    let scratch = Scratch::new("SEAM-STAMPED");
    let root = scratch.root();
    let target = Target::single(root, Vendor::Zerodha);
    let _ = record(root, Vendor::Zerodha, &[row(0, 1)]).expect("a census file");
    let loaded = target.load().expect("loads");
    let stamp = Stamp::of(&fs::symlink_metadata(&loaded.path).expect("stat"));
    let held = KeptCensus { loaded, stamp };
    seam::plant("kept.stamped", |at| {
        let file = fs::OpenOptions::new().write(true).open(at).expect("open");
        file.set_len(100).expect("cut");
    });
    assert_eq!(held.still_current(&target), Ok(Err("header_unread")));
}

/// A MIGRATION WHOSE RECEIPT CANNOT BE WRITTEN renames nothing into place.
#[test]
fn a_migration_whose_receipt_cannot_be_written_renames_nothing() {
    let scratch = Scratch::new("SEAM-RECEIPT");
    let root = scratch.root();
    let _ = record(root, Vendor::Zerodha, &[row(0, 1)]).expect("a single-file census");
    seam::plant("migrate.staged", |staging| {
        fs::create_dir_all(staging.join(MIGRATION_RECEIPT).join("inside")).expect("in the way");
    });
    let why = migrate(root, Vendor::Zerodha).expect_err("refused");
    assert!(why.contains(MIGRATION_RECEIPT), "{why}");
    assert!(fs::symlink_metadata(vendor_dir(root, Vendor::Zerodha)).is_err());
    assert!(holds_census(root), "the single file is untouched");
}

/// A MIGRATION WHOSE RENAME MEETS A DIRECTORY AT THE VERSION-4 NAME names
/// both, and leaves the staging directory for inspection.
#[test]
fn a_migration_whose_rename_is_refused_leaves_its_staging() {
    let scratch = Scratch::new("SEAM-RENAME");
    let root = scratch.root();
    let _ = record(root, Vendor::Zerodha, &[row(0, 1)]).expect("a single-file census");
    let dir = vendor_dir(root, Vendor::Zerodha);
    seam::plant("migrate.staged", move |_staging| {
        fs::create_dir_all(dir.join("planted")).expect("a directory at the v4 name");
    });
    let why = migrate(root, Vendor::Zerodha).expect_err("refused");
    let staging = staging_dir(root, Vendor::Zerodha);
    assert!(why.contains("could not be renamed to"), "{why}");
    assert!(
        why.contains("the staging directory is left for inspection"),
        "{why}"
    );
    assert!(staging.join(MIGRATION_RECEIPT).is_file(), "staging kept");
    assert!(holds_census(root), "the single file is untouched");
}

/// Whether the zerodha single file still holds a census (not a stub).
fn holds_census(root: &Path) -> bool {
    let single = manifest_path(root, Vendor::Zerodha);
    assert!(single.is_file(), "{} is no file", single.display());
    !holds_sealed_stub(&single)
}

/// A STAGED EPOCH THAT READS BACK OTHERWISE THAN IT WAS WRITTEN is refused
/// with both counts, and nothing is renamed into place.
#[test]
fn a_staged_epoch_that_reads_back_otherwise_is_refused() {
    let scratch = Scratch::new("SEAM-READBACK");
    let root = scratch.root();
    let _ = record(root, Vendor::Zerodha, &[row(0, 1)]).expect("a single-file census");
    let mut other =
        Manifest::partition_genesis(Vendor::Zerodha, Segment::Index, 64).expect("genesis");
    other.record_held(index_row(4)).expect("an index row");
    let image = other.image();
    seam::plant("stage.written", move |path| {
        fs::write(path, &image).expect("another epoch at the staged name");
    });
    let why = migrate(root, Vendor::Zerodha).expect_err("refused");
    assert!(
        why.contains("did not read back as written: 1 keys 4 rows"),
        "{why}"
    );
    assert!(why.contains("nothing was renamed into place"), "{why}");
    assert!(fs::symlink_metadata(vendor_dir(root, Vendor::Zerodha)).is_err());
}

/// A REQUIREMENT ANOTHER WRITER PUT IN FORCE BETWEEN THE READ AND THE APPEND
/// is in force: nothing is written.
#[test]
fn a_requirement_put_in_force_between_the_read_and_the_append_writes_nothing() {
    let scratch = Scratch::new("SEAM-REQUIRES");
    let root = scratch.root();
    let wrote = format!(
        "{REQUIRES_FORMAT}\nrequires={FEATURE_CENSUS_V4} vendor=gdfl since={V4_READER_SINCE} at=1\n"
    );
    let planted = wrote.clone();
    seam::plant("requires.read", move |at| {
        fs::write(at, &planted).expect("another writer's line");
    });
    assert_eq!(
        require_feature(root, Vendor::Gdfl, FEATURE_CENSUS_V4),
        Ok(false)
    );
    assert_eq!(
        fs::read_to_string(requires_path(root)).expect("requires"),
        wrote
    );
}

/// A STUB CUT BETWEEN ITS OPEN AND ITS READ is no stub.
#[test]
fn a_stub_cut_between_its_open_and_its_read_is_no_stub() {
    let scratch = Scratch::new("SEAM-STUB");
    let at = scratch.root().join("manifest").join("zerodha.man");
    fs::write(&at, sealed_stub(Vendor::Zerodha)).expect("the stub");
    seam::plant("stub.opened", |path| {
        let file = fs::OpenOptions::new().write(true).open(path).expect("open");
        // The magic and the version survive the cut: only the short read
        // tells this from a stub.
        file.set_len(10).expect("cut to the magic and the version");
    });
    assert!(!holds_sealed_stub(&at));
}

/// A migrated zerodha store: a receipt naming `bytes` and a single file of
/// that many invented bytes.
fn migrated_with(root: &Path, bytes: usize) -> PathBuf {
    let dir = vendor_dir(root, Vendor::Zerodha);
    fs::create_dir_all(&dir).expect("migrated");
    fs::write(
        dir.join(MIGRATION_RECEIPT),
        format!("legacy_bytes={bytes}\n"),
    )
    .expect("receipt");
    let legacy = manifest_path(root, Vendor::Zerodha);
    fs::write(&legacy, vec![7_u8; bytes]).expect("the single file");
    legacy
}

/// A SINGLE FILE THAT MOVED AFTER ITS DRIFT CHECK is not sealed, and a
/// superseded name another writer took is not linked over.
#[test]
fn a_single_file_that_moved_after_its_drift_check_is_not_sealed() {
    let scratch = Scratch::new("SEAM-SEAL");
    let root = scratch.root();
    let legacy = migrated_with(root, 40);
    seam::plant("seal.drift_checked", |at| {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(at)
            .expect("append");
        file.write_all(b"grown").expect("grown");
    });
    let why = seal_legacy(root, Vendor::Zerodha).expect_err("refused");
    assert!(
        why.contains("is 45 bytes and not a plain file of the 40 bytes the migration recorded"),
        "{why}"
    );
    assert!(fs::symlink_metadata(superseded_path(root, Vendor::Zerodha)).is_err());

    let _ = fs::write(&legacy, vec![7_u8; 40]);
    seam::plant("seal.linking", |superseded| {
        fs::write(superseded, b"another writer's file").expect("taken");
    });
    let why = seal_legacy(root, Vendor::Zerodha).expect_err("refused");
    assert!(why.contains("could not be linked to"), "{why}");
    assert_eq!(
        fs::read(&legacy).expect("legacy"),
        vec![7_u8; 40],
        "untouched"
    );
}

/// A STUB THAT CANNOT BE RENAMED OVER THE SINGLE FILE leaves the census where
/// it is, and says so.
#[test]
fn a_stub_that_cannot_be_renamed_over_the_census_is_named() {
    let scratch = Scratch::new("SEAM-STUB-RENAME");
    let root = scratch.root();
    let _ = migrated_with(root, 40);
    seam::plant("stub.staged", |legacy| {
        fs::remove_file(legacy).expect("the census moved away");
        fs::create_dir_all(legacy.join("inside")).expect("a directory at its name");
    });
    let why = seal_legacy(root, Vendor::Zerodha).expect_err("refused");
    assert!(why.contains("could not be renamed over"), "{why}");
    assert!(why.contains("still holds the census"), "{why}");
}

/// A TORN SEAL RECEIPT GONE BEFORE IT IS MOVED ASIDE is named with the name it
/// was to be kept under; nothing is written.
#[test]
fn a_torn_receipt_gone_before_it_is_moved_aside_is_named() {
    let scratch = Scratch::new("SEAM-ASIDE");
    let path = scratch.root().join("SEALED");
    fs::write(&path, b"torn").expect("a torn receipt");
    seam::plant("receipt.aside", |at| fs::remove_file(at).expect("gone"));
    let why = settle_seal_receipt(&path, &scratch.root().join("kept"), 5).expect_err("refused");
    assert!(why.contains("is no whole seal receipt"), "{why}");
    assert!(why.contains("could not be moved to"), "{why}");
    assert!(why.contains("nothing was written"), "{why}");
    assert!(
        fs::symlink_metadata(&path).is_err(),
        "no receipt was written"
    );
}

/// A MONTH WHOSE HEADER SLOT FAILS ITS CHECKSUM is named slot-damaged, its
/// month unreadable, and it offers no row: the read fell back to an older
/// generation, so the row would be that generation's. The recount is then not
/// clean.
#[test]
fn a_reverted_month_is_named_and_offers_no_row() {
    let scratch = Scratch::new("WALK-REVERTED");
    let root = scratch.root();
    let path = bars(root, Vendor::Zerodha, ym(2031, 6), 3);
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .expect("the month");
    let mut one = [0_u8; 1];
    file.read_exact_at(&mut one, 16_384 + 56).expect("slot 1");
    file.write_all_at(&[one[0] ^ 0x01], 16_384 + 56)
        .expect("one checksum byte flipped");
    drop(file);
    let found = recount(root, Vendor::Zerodha, span_from(2031, 1, 1), None).expect("recounts");
    assert_eq!(found.slot_damaged.len(), 1, "{found:?}");
    assert_eq!(found.slot_damaged[0].0, path);
    assert!(matches!(
        found.slot_damaged[0].1.verdict,
        store::header::MonthVerdict::Reverted { .. }
    ));
    assert_eq!(found.months_unreadable, vec![ym(2031, 6)]);
    assert_eq!((found.unreadable, found.offered), (0, 0));
    assert!(!found.clean(), "a damaged slot is not clean");
}

/// MORE UNREADABLE MONTHS THAN THE CAP are all counted and only the first
/// fifty named, each with why.
#[test]
fn unreadable_months_past_the_cap_are_counted_and_not_named() {
    let scratch = Scratch::new("WALK-UNREADABLE");
    let root = scratch.root();
    let rung = root
        .join(store::path::STORE_ROOT)
        .join("zerodha")
        .join("NSE")
        .join("INDEX")
        .join("NIFTY")
        .join(Timeframe::SECOND_1.as_str());
    fs::create_dir_all(&rung).expect("rung");
    for n in 0..=NAMED {
        let month = ym(
            2031 + u16::try_from(n / 12).expect("a year"),
            1 + u8::try_from(n % 12).expect("a month"),
        );
        fs::write(rung.join(format!("{month}.bin")), b"invented: no bar file").expect("garbage");
    }
    let found = recount(root, Vendor::Zerodha, span_from(2031, 1, 1), None).expect("recounts");
    assert_eq!(found.unreadable, NAMED as u64 + 1);
    assert_eq!(found.unreadable_named.len(), NAMED);
    assert_eq!(found.months_unreadable.len(), NAMED + 1);
    assert!(!found.clean());
}

/// An INDEX epoch of zerodha whose older generation holds the one invented
/// index row at 4 rows, and whose newer commit (a second key) is cut off: it
/// loads DEGRADED with the older generation's counts.
fn degraded_index_image(capacity: u64) -> Vec<u8> {
    let lab = Scratch::new("SEAM-DEGRADED-LAB");
    let mut census =
        Manifest::partition_genesis(Vendor::Zerodha, Segment::Index, capacity).expect("genesis");
    census.record_held(index_row(4)).expect("a row");
    let path = epoch_path(lab.root(), Vendor::Zerodha, Segment::Index, 1);
    fs::create_dir_all(path.parent().expect("dir")).expect("dir");
    fs::write(&path, census.image()).expect("epoch 1");
    let mut other = row_at(91, ym(2019, 12), 1, 10, 11);
    other.entry.key.segment = Segment::Index;
    other.entry.key.contract = None;
    let target = Target::partition(lab.root(), Vendor::Zerodha, Segment::Index);
    let _ = cycle(&target, &[other], Mode::AllOrNothing).expect("an append");
    let mut bytes = fs::read(&path).expect("the epoch");
    bytes.truncate(bytes.len() - 128);
    bytes
}

/// A STAGED EPOCH THAT READS BACK WITH ITS KEYS BUT OTHER ROWS, OTHER ENTRIES
/// OR ANOTHER CAPACITY is refused each time: every count is compared, not the
/// keys alone.
#[test]
fn a_staged_epoch_that_differs_in_any_count_is_refused() {
    let census_of = |root: &Path| {
        let _ = record(root, Vendor::Zerodha, &[index_row(4)]).expect("a single-file census");
        read_single(&manifest_path(root, Vendor::Zerodha), Vendor::Zerodha).expect("loads")
    };
    let probe = Scratch::new("SEAM-COUNTS-PROBE");
    let part = census_of(probe.root())
        .partitions()
        .expect("splits")
        .into_iter()
        .find(|part| part.segment() == Some(Segment::Index))
        .expect("an index part");
    let capacity = part.capacity();
    let index = |rows: &[u64], capacity: u64| {
        let mut census = Manifest::partition_genesis(Vendor::Zerodha, Segment::Index, capacity)
            .expect("genesis");
        for &one in rows {
            census.record_held(index_row(one)).expect("a row");
        }
        census.image()
    };
    for (what, image) in [
        ("rows", index(&[5], capacity)),
        ("entries", index(&[2, 4], capacity)),
        ("capacity", index(&[4], capacity + 2)),
        ("degraded", degraded_index_image(capacity)),
    ] {
        let scratch = Scratch::new("SEAM-COUNTS");
        let root = scratch.root();
        let _ = census_of(root);
        seam::plant("stage.written", move |path| {
            fs::write(path, &image).expect("another epoch at the staged name");
        });
        let why = migrate(root, Vendor::Zerodha).expect_err(what);
        assert!(
            why.contains("did not read back as written"),
            "{what}: {why}"
        );
        assert!(fs::symlink_metadata(vendor_dir(root, Vendor::Zerodha)).is_err());
    }
}
