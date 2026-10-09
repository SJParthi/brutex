//! Census version 4 — one directory of epochs per (vendor, segment), capacity
//! in the header, compaction into the next epoch with the old one kept.
//! D-3316, WP-10.
//!
//! # What each test is the proof of
//!
//! | Finding | Claim | Test |
//! |---|---|---|
//! | DB-G08 | a reader is bound by the file's own capacity, not a compiled one | `a_partition_header_carries_its_capacity_and_segment`, `a_partition_is_bound_by_its_own_capacity_not_the_single_file_ceiling` |
//! | DB-G10 | a batch that does not fit refuses up front, naming the file's numbers | `a_single_file_without_room_refuses_up_front_naming_its_own_numbers`, `a_batch_too_large_even_for_a_compacted_partition_is_refused_whole` |
//! | DB-058, DB-G05 | census size follows keys, not log length | `a_log_compacts_to_its_keys_and_keeps_its_totals`, `a_partition_without_room_compacts_into_its_next_epoch` |
//! | DB-G01, DB-G49 | a migrated vendor keeps counting past the old ceiling | `a_partition_without_room_compacts_into_its_next_epoch`, `the_audit_scale_census_migrates_and_compacts_with_equal_totals` |
//! | D-3316 | a partition holds one segment, and a write goes to its own | `a_partition_refuses_a_row_of_another_segment`, `rows_reach_the_partition_of_their_own_segment` |
//! | D-3316 | a pull run writes its plan's partition, compacting mid-run | `a_pull_run_writes_its_plan_s_partition_and_compacts_mid_run` |
//!
//! Every fixture is synthetic: invented keys, invented counts, invented
//! prices, no vendor row.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "a test that cannot panic cannot fail"
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::csv::Columns;
use pull::fetch::BarRequest;
use pull::ingest::census_store::{self, Location, Mode, Target};
use pull::ingest::{self, Plan};
use pull::manifest::{
    Closes, Entry, EntryKey, FNO_FLOOR, HEADER_LEN, Held, Layout, MAGIC_V2, MAX_CAPACITY,
    MAX_ENTRIES, Manifest, ManifestError, ManifestHeader, SPOT_FLOOR, epoch_capacity, epoch_path,
    manifest_path, segment_dir, vendor_dir,
};
use pull::session::{Day, Window};
use pull::vendor::{PriceScale, TimestampEncoding};
use store::path::{Timeframe, YearMonth};

/// A scratch store of this test's own, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-census-v4-{tag}-{}-{}",
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

/// The `n`th invented option contract-month: distinct for every `n` below
/// 12 · 400,000.
fn option_key(n: u32) -> EntryKey {
    let strike = 1_000_000 + u64::from(n / 12) * 100;
    let contract = Contract::parse(&format!("2019-12-26-{strike}-CE")).expect("a contract");
    EntryKey {
        contract: Some(contract),
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        symbol: Symbol::new("NIFTY").expect("a symbol"),
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2019, u8::try_from(n % 12 + 1).expect("a month")).expect("a month"),
    }
}

/// The `n`th invented spot index month: 96,000 months (1970..=9969) per
/// invented symbol, so every `n` this file asks for is a distinct key.
fn index_key(n: u32) -> EntryKey {
    let months = n % 96_000;
    EntryKey {
        contract: None,
        exchange: Exchange::Nse,
        segment: Segment::Index,
        symbol: Symbol::new(&format!("IDX{}", n / 96_000)).expect("a symbol"),
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(
            1970 + u16::try_from(months / 12).expect("a year"),
            (months % 12 + 1) as u8,
        )
        .expect("a month"),
    }
}

/// A row for `key` holding `rows` rows, its last stamp following the count.
fn row(key: EntryKey, rows: u64) -> Held {
    Held::new(
        Entry {
            key,
            rows,
            first_ts_micros: 1_000,
            last_ts_micros: 1_000 + i64::try_from(rows).expect("rows"),
        },
        Closes::known(100, 100 + i64::try_from(rows).expect("rows")).expect("two prices"),
    )
}

/// A partition of `segment` at `capacity`, every row recorded in order.
fn partition(segment: Segment, capacity: u64, rows: &[Held]) -> Manifest {
    let mut census = Manifest::partition_genesis(Vendor::Gdfl, segment, capacity).expect("genesis");
    for one in rows {
        census.record_held(*one).expect("recorded");
    }
    census
}

/// Writes a census image as `epoch` of its partition, the vendor's directory
/// marking it migrated — the bytes a migration or a compaction leaves.
fn install(root: &Path, segment: Segment, epoch: u32, census: &Manifest) -> PathBuf {
    let path = epoch_path(root, Vendor::Gdfl, segment, epoch);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("the segment dir");
    std::fs::write(&path, census.image()).expect("written");
    path
}

/// The current epoch of a gdfl partition, read as a reader reads it.
fn current(root: &Path, segment: Segment) -> (u32, Manifest) {
    let epoch = census_store::current_epoch(root, Vendor::Gdfl, segment)
        .expect("listed")
        .expect("an epoch");
    let census = census_store::read_epoch(
        &epoch_path(root, Vendor::Gdfl, segment, epoch),
        Vendor::Gdfl,
        segment,
    )
    .expect("reads");
    (epoch, census)
}

/// Re-seals a 64-byte slot after a test has edited it.
fn reseal(mut slot: [u8; 64]) -> [u8; 64] {
    let crc = store::crc::crc32c(&slot[..60]);
    slot[60..64].copy_from_slice(&crc.to_le_bytes());
    slot
}

#[test]
fn a_partition_header_carries_its_capacity_and_segment() {
    let header = ManifestHeader {
        generation: 4,
        n_valid: 3,
        n_keys: 2,
        total_rows: 10,
        ..ManifestHeader::genesis_partition(Vendor::Gdfl, Segment::Fno, 3_000_000).expect("ok")
    };
    let slot = header.image();
    assert_eq!(
        &slot[0..8],
        &MAGIC_V2,
        "the geometry did not change, so neither did the magic"
    );
    assert_eq!(
        &slot[8..10],
        &4u16.to_le_bytes(),
        "the version says the runs hold fields"
    );
    assert_eq!(&slot[10..12], &128u16.to_le_bytes(), "version 3's entry");
    assert_eq!(&slot[12..16], &3_000_000u32.to_le_bytes(), "capacity at 12");
    assert_eq!(slot[56], 3, "FNO's code at 56");
    assert_eq!(&slot[57..60], &[0, 0, 0]);
    let back = ManifestHeader::decode(&slot).expect("decodes");
    assert_eq!(back, header);
    assert_eq!(
        (back.capacity, back.segment),
        (3_000_000, Some(Segment::Fno))
    );
    assert_eq!(
        back.limit(),
        3_000_000,
        "the file's bound, past MAX_ENTRIES"
    );
    assert!(back.limit() > MAX_ENTRIES);
    assert_eq!(back.headroom(), 2_999_997);
    assert_eq!(back.write_layout(), Layout::PARTITION);

    // A single file keeps the image it always had: both runs zero.
    let single = ManifestHeader::genesis(Vendor::Gdfl).image();
    assert!(
        single[12..16]
            .iter()
            .chain(&single[56..60])
            .all(|b| *b == 0)
    );
    assert_eq!(
        ManifestHeader::decode(&single).expect("decodes").segment,
        None
    );
    assert_eq!(
        ManifestHeader::decode(&single).expect("decodes").limit(),
        MAX_ENTRIES
    );

    // Every capacity the 32-bit field carries is admitted, zero and past it
    // are refused at the door; never clamped.
    for capacity in [0, MAX_CAPACITY + 1] {
        assert_eq!(
            ManifestHeader::genesis_partition(Vendor::Gdfl, Segment::Fno, capacity),
            Err(ManifestError::CapacityOutOfRange { capacity })
        );
    }
    let widest = ManifestHeader::genesis_partition(Vendor::Gdfl, Segment::Fno, MAX_CAPACITY)
        .expect("the field's widest value");
    assert_eq!(
        ManifestHeader::decode(&widest.image())
            .expect("decodes")
            .capacity,
        MAX_CAPACITY
    );
    let mut zero = slot;
    zero[12..16].copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(
        ManifestHeader::decode(&reseal(zero)),
        Err(ManifestError::CapacityOutOfRange { capacity: 0 })
    );
    let mut unknown = slot;
    unknown[56] = 9;
    assert_eq!(
        ManifestHeader::decode(&reseal(unknown)),
        Err(ManifestError::UnknownSegmentCode { code: 9 })
    );
    let mut dirty = slot;
    dirty[58] = 1;
    assert_eq!(
        ManifestHeader::decode(&reseal(dirty)),
        Err(ManifestError::ReservedNotZero { offset: 58 })
    );
}

#[test]
fn a_partition_is_bound_by_its_own_capacity_not_the_single_file_ceiling() {
    // A FILE'S BOUND IS THE FILE'S. A partition declaring 3,000,000 admits a
    // counter past the single-file 2,097,152 — which a reader built with that
    // constant refused by size (DB-G08) — and refuses one past its own.
    let declared =
        ManifestHeader::genesis_partition(Vendor::Gdfl, Segment::Fno, 3_000_000).expect("ok");
    let past_single = ManifestHeader {
        n_valid: MAX_ENTRIES + 400_000,
        n_keys: 250_000,
        ..declared
    };
    assert_eq!(past_single.validate(MAX_ENTRIES + 400_000), Ok(()));
    let past_own = ManifestHeader {
        n_valid: 3_000_001,
        ..past_single
    };
    assert_eq!(
        past_own.validate(3_000_001),
        Err(ManifestError::TooManyEntries {
            n_valid: 3_000_001,
            batch: 0,
            limit: 3_000_000
        })
    );
    // And the single file stays bound by its constant whatever its field says.
    let single = ManifestHeader {
        n_valid: MAX_ENTRIES + 1,
        capacity: 3_000_000,
        ..ManifestHeader::genesis(Vendor::Gdfl)
    };
    assert_eq!(
        single.validate(MAX_ENTRIES + 1),
        Err(ManifestError::TooManyEntries {
            n_valid: MAX_ENTRIES + 1,
            batch: 0,
            limit: MAX_ENTRIES
        })
    );

    // At a small capacity the whole path is drivable: four fit, a fifth is
    // refused naming the census's own count and the one entry asked for —
    // never an in-memory counter one past the limit (DB-G10).
    let rows: Vec<Held> = (0..4).map(|n| row(option_key(n), 1)).collect();
    let mut census = partition(Segment::Fno, 4, &rows);
    let refused = census.record_held(row(option_key(4), 1));
    assert_eq!(
        refused,
        Err(ManifestError::TooManyEntries {
            n_valid: 4,
            batch: 1,
            limit: 4
        })
    );
    let said = refused.expect_err("refused").to_string();
    assert!(
        said.contains("n_valid is 4 of a capacity of 4 (0 free)"),
        "{said}"
    );
    assert!(said.contains("a batch of 1 entry does not fit"), "{said}");
    let reread =
        Manifest::open_partition(Vendor::Gdfl, Segment::Fno, &census.image()).expect("reads back");
    assert_eq!(
        (reread.entries(), reread.capacity(), reread.headroom()),
        (4, 4, 0)
    );
}

#[test]
fn a_partition_refuses_a_row_of_another_segment() {
    let mut census = partition(Segment::Fno, 16, &[row(option_key(0), 1)]);
    assert_eq!(
        census.record_held(row(index_key(0), 1)),
        Err(ManifestError::ForeignSegment {
            ordinal: 1,
            partition: Segment::Fno,
            found: Segment::Index
        })
    );
    // At another segment's path, or a single file at a partition's path, the
    // file is refused by name — never appended to under the wrong name.
    let image = census.image();
    assert_eq!(
        Manifest::open_partition(Vendor::Gdfl, Segment::Index, &image).err(),
        Some(ManifestError::PartitionMismatch {
            asked: Segment::Index,
            found: Some(Segment::Fno)
        })
    );
    let mut single = Manifest::open(Vendor::Gdfl, &[], &[]).expect("genesis");
    single.record_held(row(option_key(0), 1)).expect("recorded");
    assert_eq!(
        Manifest::open_partition(Vendor::Gdfl, Segment::Fno, &single.image()).err(),
        Some(ManifestError::PartitionMismatch {
            asked: Segment::Fno,
            found: None
        })
    );
    // An empty epoch is refused, never a genesis: an epoch is installed whole.
    assert!(matches!(
        Manifest::open_partition(Vendor::Gdfl, Segment::Cash, &[]),
        Err(ManifestError::HeaderRegionTooShort { .. })
    ));
    // A partition with no epoch yet is a genesis at its floor.
    let fresh = Manifest::partition_absent(Vendor::Gdfl, Segment::Cash);
    assert_eq!((fresh.entries(), fresh.capacity()), (0, SPOT_FLOOR));
    assert_eq!(fresh.segment(), Some(Segment::Cash));
    assert_eq!(
        Manifest::partition_absent(Vendor::Gdfl, Segment::Fno).capacity(),
        FNO_FLOOR
    );
}

/// THE CENSUS SCALES WITH KEYS. 20,900 entries over 2,420 keys — the audit's
/// 2.09M over 242k, at a hundredth — compact to the 2,420 newest rows, with
/// every count, every row and every close as `held` answered before.
#[test]
fn a_log_compacts_to_its_keys_and_keeps_its_totals() {
    let keys = 2_420u32;
    let mut rows: Vec<Held> = Vec::new();
    // Each key written 8 or 9 times, rows growing, interleaved across keys.
    for pass in 0..9u64 {
        for n in 0..keys {
            if pass < 8 || n % 7 != 0 {
                rows.push(row(option_key(n), pass + 1));
            }
        }
    }
    let census = partition(Segment::Fno, 1 << 15, &rows);
    assert_eq!(census.keys(), u64::from(keys));
    assert!(census.entries() > 20_000);
    assert_eq!(census.superseded(), census.entries() - census.keys());

    let compacted = census.compacted().expect("it compacts");
    assert_eq!(compacted.entries(), u64::from(keys), "one row per key");
    assert_eq!(compacted.keys(), census.keys());
    assert_eq!(compacted.total_rows(), census.total_rows());
    assert_eq!(
        compacted.header().generation,
        census.header().generation + u64::from(keys),
        "generations stay monotonic across epochs"
    );
    assert_eq!(
        compacted.capacity(),
        FNO_FLOOR,
        "the larger of the floor and 2 x keys"
    );
    for n in 0..keys {
        assert_eq!(compacted.held(&option_key(n)), census.held(&option_key(n)));
    }
    // Deterministic: the same census compacts to the same bytes, in key order.
    assert_eq!(
        compacted.image(),
        census.compacted().expect("again").image()
    );
    let order: Vec<EntryKey> = compacted.all().map(|entry| entry.key).collect();
    let mut sorted = order.clone();
    sorted.sort_unstable();
    assert_eq!(order, sorted, "an epoch holds its rows in key order");
    // And it reopens with nothing stepped over.
    let back = Manifest::open_partition(Vendor::Gdfl, Segment::Fno, &compacted.image())
        .expect("it reopens");
    assert_eq!(back.degraded_reason(), None);
    assert_eq!(
        (back.entries(), back.keys(), back.total_rows()),
        (
            compacted.entries(),
            compacted.keys(),
            compacted.total_rows()
        )
    );
    // Capacity: past half the floor, twice the keys.
    assert_eq!(epoch_capacity(Segment::Fno, FNO_FLOOR), Ok(2 * FNO_FLOOR));
    // And a single file never compacts by a write.
    let single = Manifest::open(Vendor::Gdfl, &[], &[]).expect("genesis");
    assert_eq!(
        single.compacted().err(),
        Some(ManifestError::SingleFileNotCompacted)
    );
}

#[test]
fn rows_reach_the_partition_of_their_own_segment() {
    let scratch = Scratch::new("SEGMENTS");
    let root = scratch.root();
    // Migrated, empty: every segment gets its first epoch, empty.
    let done = census_store::migrate(root, Vendor::Gdfl).expect("migrates an empty store");
    assert!(!done.legacy_present);
    assert_eq!(done.partitions.len(), 3);
    assert_eq!(
        census_store::locate(root, Vendor::Gdfl).expect("it is located"),
        Location::Partitioned(vendor_dir(root, Vendor::Gdfl))
    );
    let batch = [
        row(option_key(0), 3),
        row(index_key(0), 5),
        row(option_key(1), 7),
    ];
    let cycles = census_store::record(root, Vendor::Gdfl, &batch).expect("records");
    assert_eq!(cycles.len(), 2, "one cycle per segment touched");
    assert!(cycles.iter().all(|cycle| cycle.epoch == Some(1)));
    let (epoch, fno) = current(root, Segment::Fno);
    assert_eq!(
        (epoch, fno.keys(), fno.total_rows(), fno.capacity()),
        (1, 2, 10, FNO_FLOOR)
    );
    let (_, index) = current(root, Segment::Index);
    assert_eq!(
        (index.keys(), index.total_rows(), index.capacity()),
        (1, 5, SPOT_FLOOR)
    );
    let (_, cash) = current(root, Segment::Cash);
    assert_eq!(
        cash.keys(),
        0,
        "an empty segment is an empty epoch, not a missing one"
    );
    assert!(
        !manifest_path(root, Vendor::Gdfl).exists(),
        "no single file is made"
    );

    // A Zerodha write touches nothing of GDFL's: not a file, not a lock.
    let fno_path = epoch_path(root, Vendor::Gdfl, Segment::Fno, 1);
    let gdfl_before = std::fs::read(&fno_path).expect("bytes");
    let listing = |dir: &Path| {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .expect("listed")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    };
    let gdfl_names = listing(&vendor_dir(root, Vendor::Gdfl));
    assert!(census_store::record(root, Vendor::Zerodha, &[row(index_key(1), 2)]).is_ok());
    assert_eq!(std::fs::read(&fno_path).expect("bytes"), gdfl_before);
    assert_eq!(listing(&vendor_dir(root, Vendor::Gdfl)), gdfl_names);
    assert!(
        manifest_path(root, Vendor::Zerodha).exists(),
        "zerodha stays a single file"
    );

    // `read_vendor` answers every partition's current epoch, in segment order.
    let all = census_store::read_vendor(root, Vendor::Gdfl).expect("reads");
    let segments: Vec<Option<Segment>> = all.iter().map(|read| read.segment).collect();
    assert_eq!(
        segments,
        vec![
            Some(Segment::Index),
            Some(Segment::Cash),
            Some(Segment::Fno)
        ]
    );
    assert!(all.iter().all(|read| read.epoch == Some(1)));
    let single = census_store::read_vendor(root, Vendor::Zerodha).expect("reads");
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].segment, None);
    assert!(
        census_store::read_vendor(root, Vendor::Dhan)
            .expect("reads")
            .is_empty()
    );
}

/// DB-058 / DB-G01 / DB-G05, ON DISK. A partition at its capacity takes the
/// next batch by compacting into its NEXT EPOCH: the new epoch holds one row
/// per key and the batch, the counters agree, and the epoch it replaced is
/// kept byte for byte and never written again.
#[test]
fn a_partition_without_room_compacts_into_its_next_epoch() {
    let scratch = Scratch::new("COMPACT");
    let root = scratch.root();
    // Capacity 8, full: 8 entries over 3 keys (5 superseded).
    let rows = [
        row(option_key(0), 1),
        row(option_key(1), 1),
        row(option_key(2), 1),
        row(option_key(0), 2),
        row(option_key(1), 2),
        row(option_key(0), 3),
        row(option_key(1), 3),
        row(option_key(0), 4),
    ];
    let full = partition(Segment::Fno, 8, &rows);
    assert_eq!((full.entries(), full.keys(), full.headroom()), (8, 3, 0));
    let first = install(root, Segment::Fno, 1, &full);
    let before = std::fs::read(&first).expect("bytes");

    // Two updates and one new key: 3 more entries against 0 free.
    let batch = [
        row(option_key(0), 9),
        row(option_key(2), 9),
        row(option_key(3), 9),
    ];
    let target = census_store::target_for(root, Vendor::Gdfl, Segment::Fno).expect("a target");
    let done = census_store::cycle(&target, &batch, Mode::AllOrNothing).expect("it compacts");
    assert_eq!(done.recorded, 3);
    let compaction = done.compacted.clone().expect("it compacted");
    assert_eq!((compaction.from_epoch, compaction.to_epoch), (Some(1), 2));
    assert_eq!((compaction.from_entries, compaction.to_entries), (8, 3));
    assert_eq!(
        compaction.path,
        epoch_path(root, Vendor::Gdfl, Segment::Fno, 2)
    );
    assert_eq!(
        std::fs::read(&first).expect("epoch 1"),
        before,
        "kept byte for byte"
    );

    let (epoch, now) = current(root, Segment::Fno);
    assert_eq!(epoch, 2);
    assert_eq!(
        (now.entries(), now.keys()),
        (6, 4),
        "3 compacted rows, then the batch"
    );
    assert_eq!(now.total_rows(), 9 + 3 + 9 + 9);
    assert_eq!(now.held(&option_key(0)).map(|h| h.entry.rows), Some(9));
    assert_eq!(now.held(&option_key(1)).map(|h| h.entry.rows), Some(3));
    assert_eq!(now.capacity(), FNO_FLOOR, "grown to the segment's floor");
    assert!(now.header().generation > full.header().generation);
    assert_eq!(done.epoch, Some(2));
    assert_eq!((done.entries, done.keys, done.rows), (6, 4, 30));

    // Room now, so the next batch is a positional append to epoch 2, and
    // epoch 1 is still exactly as it was.
    let next = census_store::cycle(&target, &[row(option_key(4), 1)], Mode::AllOrNothing)
        .expect("it appends");
    assert!(next.compacted.is_none(), "room now, so a positional append");
    assert_eq!(next.epoch, Some(2));
    assert_eq!(std::fs::read(&first).expect("epoch 1"), before);
    // A re-offer of what is held changes no byte (CLAUDE.md §3 rule 5).
    let second = std::fs::read(epoch_path(root, Vendor::Gdfl, Segment::Fno, 2)).expect("bytes");
    let again =
        census_store::cycle(&target, &[row(option_key(4), 1)], Mode::AllOrNothing).expect("exact");
    assert_eq!((again.exact, again.recorded), (1, 0));
    assert_eq!(
        std::fs::read(epoch_path(root, Vendor::Gdfl, Segment::Fno, 2)).expect("bytes"),
        second
    );
    // A row that goes backwards refuses the whole batch, naming the file.
    let why = census_store::cycle(
        &target,
        &[row(option_key(5), 1), row(option_key(4), 0)],
        Mode::AllOrNothing,
    )
    .expect_err("refused");
    assert!(
        why.contains("nothing of this batch of 2 row(s) was published"),
        "{why}"
    );
    assert_eq!(
        std::fs::read(epoch_path(root, Vendor::Gdfl, Segment::Fno, 2)).expect("bytes"),
        second
    );
}

/// D-3316 ITEM 5 AT A REAL FLOOR. An INDEX partition at capacity 1, full, is
/// offered 1,048,576 new keys: compacted, its next epoch would hold one key at
/// the floor of 1,048,576 — one slot short. It refuses before recording a row,
/// naming the file's numbers, and installs no epoch.
#[test]
fn a_batch_too_large_even_for_a_compacted_partition_is_refused_whole() {
    let scratch = Scratch::new("TOOBIG");
    let root = scratch.root();
    let full = partition(Segment::Index, 1, &[row(index_key(0), 1)]);
    let first = install(root, Segment::Index, 1, &full);
    let batch: Vec<Held> = (1..=u32::try_from(SPOT_FLOOR).expect("fits"))
        .map(|n| row(index_key(n), 1))
        .collect();
    let target = Target::partition(root, Vendor::Gdfl, Segment::Index);
    let why = census_store::cycle(&target, &batch, Mode::AllOrNothing).expect_err("refused");
    for says in [
        "refused this batch before recording any of it",
        "n_valid 1 of a capacity of 1 (0 free)",
        "the batch needs 1048576 entries (1048576 new keys)",
        "Even compacted to one row per key (1 keys, capacity 1048576, 1048575 free)",
        "no epoch was installed",
        "Nothing of the batch was published",
    ] {
        assert!(why.contains(says), "{says:?} missing from: {why}");
    }
    assert_eq!(
        census_store::current_epoch(root, Vendor::Gdfl, Segment::Index),
        Ok(Some(1))
    );
    assert_eq!(std::fs::read(&first).expect("bytes"), full.image());
}

/// DB-G10, MEASURED AT THE REAL CEILING. A single file with three free slots
/// is offered five new rows: it refuses before recording any, names its OWN
/// `n_valid`, its capacity, the free slots and the batch, says nothing was
/// published, and leaves the file byte for byte.
///
/// This builds a 268 MB file because the single-file ceiling is compiled and
/// nothing smaller reaches it; that cost is the argument for version 4.
#[test]
fn a_single_file_without_room_refuses_up_front_naming_its_own_numbers() {
    let scratch = Scratch::new("CEILING");
    let root = scratch.root();
    let n = MAX_ENTRIES - 3;
    // The bytes directly, without a writer's index: header, then entries.
    let header = ManifestHeader {
        generation: n,
        n_valid: n,
        n_keys: n,
        total_rows: n,
        ..ManifestHeader::genesis(Vendor::Gdfl)
    };
    let mut file = vec![0u8; HEADER_LEN as usize];
    let slot = header.commit().expect("a commit");
    let at = usize::try_from(slot.offset).expect("an offset in range");
    file[at..at + 64].copy_from_slice(&slot.bytes);
    file.reserve(usize::try_from(n).expect("n in range") * 128);
    for k in 0..u32::try_from(n).expect("n in range") {
        file.extend_from_slice(&row(index_key(k), 1).image());
    }
    let path = manifest_path(root, Vendor::Gdfl);
    std::fs::write(&path, &file).expect("written");
    drop(file);
    let before = std::fs::metadata(&path).expect("its metadata").len();

    let batch: Vec<Held> = (0..5)
        .map(|k| row(index_key(u32::try_from(n).expect("n in range") + k), 1))
        .collect();
    let why = pull::ingest::record_held(root, Vendor::Gdfl, &batch).expect("refused");
    for says in [
        "refused this batch before recording any of it",
        "n_valid 2097149 of a capacity of 2097152 (3 free)",
        "the batch needs 5 entries (5 new keys) for 5 offered rows",
        "a batch of 5 entries does not fit",
        "Nothing of the batch was published",
        "cli census-migrate gdfl",
    ] {
        assert!(why.contains(says), "{says:?} missing from: {why}");
    }
    assert!(
        !why.contains(&(MAX_ENTRIES + 1).to_string()),
        "never an in-memory counter: {why}"
    );
    assert_eq!(
        std::fs::metadata(&path).expect("its metadata").len(),
        before
    );
    assert!(matches!(
        census_store::locate(root, Vendor::Gdfl),
        Ok(Location::Single(_))
    ));
}

// ═══ A pull run against a migrated vendor ═══════════════════════════════════

/// One invented minute series: the regular session of 2022-10-03, at a price
/// no real index has quoted (`11111.00` and up by a paisa a minute).
fn minute_body() -> String {
    let mut body = String::new();
    for minute in 0..375u32 {
        let clock = 9 * 60 + 15 + minute;
        let _ = writeln!(
            body,
            "20221003,{:02}:{:02}:00,11111.{:02},0,0",
            clock / 60,
            clock % 60,
            minute % 100
        );
    }
    body
}

fn minute_request() -> BarRequest {
    BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Equity,
        window: Window::new(
            Day::new(2022, 10, 3).expect("a day"),
            Day::new(2022, 10, 3).expect("a day"),
        )
        .expect("a window"),
        granularity: pull::vendor::Granularity::Minute1,
    }
}

fn minute_plan(request: &BarRequest) -> Plan<'_> {
    Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: Columns::TrueDataIndex,
        request,
        encoding: TimestampEncoding::EpochSecondsUtc,
        scale: PriceScale::Paisa,
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
    }
}

/// A folder of invented members, one per name.
fn archive(root: &Path, names: &[&str]) -> PathBuf {
    let dir = root.join("ARCHIVE");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("an archive");
    for name in names {
        std::fs::write(dir.join(format!("{name}.csv")), minute_body()).expect("a member");
    }
    dir
}

/// A PULL RUN WRITES ITS PLAN'S PARTITION (D-3316 item 1). Against a migrated
/// vendor, a run of index members appends to the INDEX epoch and opens no
/// other segment's census; against a full INDEX epoch it compacts in memory and
/// installs the next epoch whole, keeping the old one; a re-run changes nothing.
#[test]
fn a_pull_run_writes_its_plan_s_partition_and_compacts_mid_run() {
    let scratch = Scratch::new("PULL");
    let root = scratch.root();
    let store = root.join("STORE");
    std::fs::create_dir_all(&store).expect("a store");
    let request = minute_request();
    let done = census_store::migrate(&store, Vendor::Gdfl).expect("migrated, empty");
    assert_eq!(done.partitions.len(), 3);
    let fno_before =
        std::fs::read(epoch_path(&store, Vendor::Gdfl, Segment::Fno, 1)).expect("the FNO epoch");

    let run = ingest::from_dir(&archive(root, &["NIFTY"]), &store, minute_plan(&request))
        .expect("the folder reads");
    assert!(run.failures.is_empty(), "{:?}", run.failures);
    assert!(run.counted > 0, "the member's rungs are counted");
    let (epoch, index) = current(&store, Segment::Index);
    assert_eq!(epoch, 1, "a positional append to the first epoch");
    assert_eq!(index.keys(), u64::try_from(run.counted).expect("counted"));
    assert_eq!(
        std::fs::read(epoch_path(&store, Vendor::Gdfl, Segment::Fno, 1)).expect("the FNO epoch"),
        fno_before,
        "an INDEX run never writes the FNO census"
    );
    assert!(!manifest_path(&store, Vendor::Gdfl).exists());

    // A re-run of the same folder changes no byte of the census.
    let first = std::fs::read(epoch_path(&store, Vendor::Gdfl, Segment::Index, 1)).expect("bytes");
    let again = ingest::from_dir(&archive(root, &["NIFTY"]), &store, minute_plan(&request))
        .expect("the folder reads");
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert_eq!(
        std::fs::read(epoch_path(&store, Vendor::Gdfl, Segment::Index, 1)).expect("bytes"),
        first
    );

    // Now a FULL index epoch: rewrite epoch 2 at a capacity of exactly what
    // it holds, as a writer might have left it, and run a new member.
    let (_, held) = current(&store, Segment::Index);
    let rows: Vec<Held> = held
        .held_keys()
        .copied()
        .map(|key| held.held(&key).expect("held"))
        .collect();
    let mut tight = Manifest::partition_genesis(
        Vendor::Gdfl,
        Segment::Index,
        u64::try_from(rows.len()).expect("rows"),
    )
    .expect("genesis");
    for one in &rows {
        tight.record_held(*one).expect("recorded");
    }
    let tight_path = epoch_path(&store, Vendor::Gdfl, Segment::Index, 2);
    std::fs::write(&tight_path, tight.image()).expect("a full epoch 2");
    let tight_bytes = std::fs::read(&tight_path).expect("bytes");
    let grown = ingest::from_dir(
        &archive(root, &["BANKNIFTY"]),
        &store,
        minute_plan(&request),
    )
    .expect("the folder reads");
    assert!(grown.failures.is_empty(), "{:?}", grown.failures);
    let (epoch, index) = current(&store, Segment::Index);
    assert_eq!(epoch, 3, "compacted mid-run into the next epoch");
    assert_eq!(
        index.keys(),
        u64::try_from(rows.len() + grown.counted).expect("keys")
    );
    assert_eq!(index.entries(), index.keys(), "one row per key");
    assert_eq!(index.capacity(), SPOT_FLOOR);
    assert_eq!(
        std::fs::read(&tight_path).expect("epoch 2"),
        tight_bytes,
        "kept"
    );
    assert!(
        segment_dir(&store, Vendor::Gdfl, Segment::Index)
            .join("000003.man")
            .exists()
    );
}

/// THE AUDIT'S SCALE, IN FULL (D-3316 verification, WP-80 step 1 in
/// miniature). A version-3 single file of 2,093,049 entries over 242,341 keys
/// — the shape `gdfl.man` had on 6 Oct 2026 — migrates to epoch `000001` of
/// each segment with equal keys and rows, and the single file is untouched.
/// Then a partition log of 2,200,000 entries, past the single-file ceiling, is
/// read back by its own capacity, compacted to its keys, and reopened with
/// equal totals.
///
/// Ignored in the ordinary run for its memory, not its correctness; run it
/// with `cargo test -p pull --test census_v4 --release -- --ignored --nocapture`.
#[test]
#[ignore = "builds a 268 MB single file and a 282 MB epoch, and peaks near 2 GB"]
fn the_audit_scale_census_migrates_and_compacts_with_equal_totals() {
    let scratch = Scratch::new("SCALE");
    let root = scratch.root();
    let keys = 242_341u32;
    let entries = 2_093_049u32;
    let index_keys = 2_341u32;

    // The single file, built as the import built it: one entry per write.
    let started = std::time::Instant::now();
    let mut single = Manifest::open(Vendor::Gdfl, &[], &[]).expect("genesis");
    for i in 0..entries {
        let n = i % keys;
        let pass = u64::from(i / keys) + 1;
        let key = if n < index_keys {
            index_key(n)
        } else {
            option_key(n - index_keys)
        };
        single.record_held(row(key, pass)).expect("recorded");
    }
    assert_eq!(
        (single.entries(), single.keys()),
        (u64::from(entries), u64::from(keys))
    );
    let legacy = manifest_path(root, Vendor::Gdfl);
    std::fs::write(&legacy, single.image()).expect("written");
    let legacy_bytes = std::fs::read(&legacy).expect("bytes");
    let built = started.elapsed();

    let started = std::time::Instant::now();
    let done = census_store::migrate(root, Vendor::Gdfl).expect("it migrates");
    let migrated = started.elapsed();
    assert_eq!(
        std::fs::read(&legacy).expect("bytes"),
        legacy_bytes,
        "untouched"
    );
    let (_, fno) = current(root, Segment::Fno);
    let (_, index) = current(root, Segment::Index);
    let (_, cash) = current(root, Segment::Cash);
    assert_eq!(fno.keys() + index.keys() + cash.keys(), single.keys());
    assert_eq!(
        fno.total_rows() + index.total_rows() + cash.total_rows(),
        single.total_rows()
    );
    assert_eq!(fno.entries(), fno.keys(), "migrated one row per key");
    assert_eq!(fno.capacity(), FNO_FLOOR);
    assert_eq!(fno.headroom(), FNO_FLOOR - fno.keys());

    // A partition log past the single-file ceiling, then compacted.
    let log_keys = 242_000u32;
    let log_entries = 2_200_000u32;
    let mut census =
        Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, FNO_FLOOR).expect("genesis");
    for i in 0..log_entries {
        census
            .record_held(row(option_key(i % log_keys), u64::from(i / log_keys) + 1))
            .expect("recorded");
    }
    assert!(
        census.entries() > MAX_ENTRIES,
        "past the single-file ceiling"
    );
    let path = install(root, Segment::Fno, 2, &census);
    let started = std::time::Instant::now();
    let loaded = census_store::read_epoch(&path, Vendor::Gdfl, Segment::Fno).expect("reads");
    let read = started.elapsed();
    assert_eq!(
        (loaded.entries(), loaded.keys(), loaded.total_rows()),
        (census.entries(), census.keys(), census.total_rows())
    );
    let started = std::time::Instant::now();
    let compacted = loaded.compacted().expect("it compacts");
    let compact = started.elapsed();
    assert_eq!(compacted.entries(), u64::from(log_keys));
    assert_eq!(compacted.total_rows(), census.total_rows());
    let path = install(root, Segment::Fno, 3, &compacted);
    let started = std::time::Instant::now();
    let reopened = census_store::read_epoch(&path, Vendor::Gdfl, Segment::Fno).expect("reads");
    let reread = started.elapsed();
    assert_eq!(
        (reopened.entries(), reopened.keys(), reopened.total_rows()),
        (u64::from(log_keys), census.keys(), census.total_rows())
    );
    println!(
        "scale: single {entries} entries / {keys} keys built in {built:?}, migrated in \
         {migrated:?} to FNO {} INDEX {} CASH {} keys ({} bytes FNO); log {log_entries} / \
         {log_keys} read in {read:?}, compacted in {compact:?}, reread in {reread:?}; \
         file {} -> {} bytes",
        fno.keys(),
        index.keys(),
        cash.keys(),
        done.partitions
            .iter()
            .find(|part| part.segment == Segment::Fno)
            .map_or(0, |part| part.bytes),
        HEADER_LEN + u64::from(log_entries) * 128,
        std::fs::metadata(&path).expect("its metadata").len()
    );
}
