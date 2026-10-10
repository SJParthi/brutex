//! `census.rs`'s tests of the per-vendor read (D-3316, D-3327): a census
//! extended by its appended entries is the census a whole read gives; a
//! migrated vendor is read from its version-4 partitions, each bounded by the
//! capacity its own header declares; and a census near its capacity, or behind
//! its import, says so in its own word without calling itself damaged.
//!
//! Every price and row count below is invented.

use super::*;
use pull::manifest::{Closes, Entry, EntryKey, Held, Manifest, epoch_path, segment_dir};

/// A store root of its own, with its `manifest/` directory.
fn root(name: &str) -> PathBuf {
    let dir = crate::scratch::path(&format!("census-tail-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("manifest")).expect("mkdir");
    dir
}

/// One key: an option of NIFTY when `contract` is given, spot otherwise.
fn key(segment: Segment, contract: Option<&str>, month: u8) -> EntryKey {
    EntryKey {
        contract: contract
            .map(|text| brutex_core::instrument::Contract::parse(text).expect("a contract")),
        exchange: Exchange::Nse,
        segment,
        symbol: Symbol::new("NIFTY").expect("a symbol"),
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2019, month).expect("a month"),
    }
}

/// A row of `rows` bars whose last stamp is `last` seconds into 2019, with an
/// invented close.
fn row(key: EntryKey, rows: u64, last: i64) -> Held {
    let base = 1_546_300_800_000_000_i64;
    Held::new(
        Entry {
            key,
            rows,
            first_ts_micros: base,
            last_ts_micros: base + last * 1_000_000,
        },
        Closes::known(7_001, 7_000 + i64::try_from(rows).expect("small")).expect("a close"),
    )
}

/// A census file on disk and the in-memory census the writer keeps beside it.
struct Disk {
    path: PathBuf,
    manifest: Manifest,
}

impl Disk {
    /// GDFL's single file, holding `rows`.
    fn single(root: &Path, rows: &[Held]) -> Self {
        let mut manifest = Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
        for one in rows {
            let _append = manifest.record_held(*one).expect("records");
        }
        let path = manifest_path(root, Vendor::Gdfl);
        std::fs::write(&path, manifest.image()).expect("written");
        Self { path, manifest }
    }

    /// GDFL's `segment` partition as epoch `epoch`, declaring `capacity`.
    fn partition(root: &Path, segment: Segment, epoch: u32, capacity: u64, rows: &[Held]) -> Self {
        let mut manifest =
            Manifest::partition_genesis(Vendor::Gdfl, segment, capacity).expect("a partition");
        for one in rows {
            let _append = manifest.record_held(*one).expect("records");
        }
        std::fs::create_dir_all(segment_dir(root, Vendor::Gdfl, segment)).expect("mkdir");
        let path = epoch_path(root, Vendor::Gdfl, segment, epoch);
        std::fs::write(&path, manifest.image()).expect("written");
        Self { path, manifest }
    }

    /// Appends `rows` the way the writer does: each entry at its own offset,
    /// then the header slot that counts it, into the SAME file.
    fn append(&mut self, rows: &[Held]) {
        use std::os::unix::fs::FileExt as _;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&self.path)
            .expect("open");
        for one in rows {
            let append = self.manifest.record_held(*one).expect("records");
            file.write_all_at(&append.bytes, append.offset)
                .expect("entry");
            file.write_all_at(&append.commit.bytes, append.commit.offset)
                .expect("slot");
        }
    }
}

/// The census's loaded half, or a failure naming what it was instead.
fn held(census: &VendorCensus) -> &Manifests {
    match census.state {
        Census::Held { ref manifest } => manifest,
        Census::Absent | Census::Unreadable { .. } => panic!("not held: {}", census.note()),
    }
}

/// Every counter and every key's newest row agree between two censuses.
fn same_census(extended: &VendorCensus, whole: &VendorCensus) {
    let (a, b) = (held(extended), held(whole));
    assert_eq!(a.generation(), b.generation(), "generation");
    assert_eq!(a.keys(), b.keys(), "keys");
    assert_eq!(a.total_rows(), b.total_rows(), "rows");
    assert_eq!(a.entries(), b.entries(), "entries");
    let sorted = |census: &Manifests| {
        let mut rows = census.newest();
        rows.sort_unstable_by_key(|entry| entry.key);
        rows
    };
    assert_eq!(sorted(a), sorted(b), "the newest row of every key");
    for key in b.held_keys() {
        assert_eq!(a.held(key), b.held(key), "{key:?}");
    }
    assert_eq!(
        a.held_keys().count(),
        b.held_keys().count(),
        "each key once"
    );
    assert_eq!(
        held_entries(std::slice::from_ref(extended)),
        held_entries(std::slice::from_ref(whole))
    );
}

/// **A CENSUS EXTENDED BY ITS TAIL IS THE CENSUS A WHOLE READ GIVES**
/// (DB-059, DB-G06, DB-G79). New keys, a key that grew, and a tail on a tail,
/// each compared field by field with a whole read of the same file; the
/// extended reads decode only the appended entries and never read the file
/// whole.
#[test]
fn a_tail_reaches_the_census_a_whole_read_gives() {
    let dir = root("reaches");
    let ce = |month| key(Segment::Fno, Some("2019-03-28-1150000-CE"), month);
    let pe = |month| key(Segment::Fno, Some("2019-03-28-1150000-PE"), month);
    let spot = |month| key(Segment::Index, None, month);
    let mut disk = Disk::single(&dir, &[row(ce(2), 10, 100), row(spot(2), 20, 100)]);
    let path = disk.path.clone();
    let whole_reads = || manifest_reads::of(&path);
    let first = read_vendor(&dir, Vendor::Gdfl);
    let read_once = whole_reads();

    // UNCHANGED: the census is handed back, nothing decoded, nothing read whole.
    let unchanged = read_vendor_after(&dir, Vendor::Gdfl, Some(&first));
    same_census(&unchanged, &first);
    assert_eq!(
        whole_reads(),
        read_once,
        "an unchanged file is not read whole"
    );

    // A NEW KEY, A KEY THAT GREW, AND ANOTHER NEW KEY.
    disk.append(&[
        row(pe(3), 5, 200),
        row(ce(2), 15, 150),
        row(spot(3), 30, 300),
    ]);
    let extended = read_vendor_after(&dir, Vendor::Gdfl, Some(&first));
    assert_eq!(whole_reads(), read_once, "extended, not read whole");
    assert_eq!(
        manifest_reads::tails_of(&disk.path).last(),
        Some(&3),
        "exactly the three appended entries were decoded"
    );
    let whole = read_vendor(&dir, Vendor::Gdfl);
    same_census(&extended, &whole);
    assert_eq!(
        held(&extended).held(&ce(2)).map(|one| one.entry.rows),
        Some(15)
    );

    // A TAIL ON A TAIL.
    disk.append(&[row(pe(3), 9, 260)]);
    let again = read_vendor_after(&dir, Vendor::Gdfl, Some(&extended));
    let whole_now = whole_reads();
    same_census(&again, &read_vendor(&dir, Vendor::Gdfl));
    assert_eq!(manifest_reads::tails_of(&disk.path).last(), Some(&1));
    assert!(
        whole_now > read_once,
        "the comparison reads were whole reads"
    );
}

/// **THE APPENDS AFTER A GENERATION ARE NAMED, AND A GENERATION THE TAIL DOES
/// NOT HOLD IS REFUSED** (D-3327 item 3): what `/census/facets.json?since=`
/// reads.
#[test]
fn the_tail_names_what_was_appended_after_each_generation() {
    let dir = root("since");
    let ce = key(Segment::Fno, Some("2019-03-28-1150000-CE"), 3);
    let pe = key(Segment::Fno, Some("2019-03-28-1150000-PE"), 3);
    let mut disk = Disk::single(&dir, &[row(ce, 1, 10)]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    let base = held(&first).generation();
    disk.append(&[row(pe, 2, 20), row(ce, 3, 30)]);
    let after = read_vendor_after(&dir, Vendor::Gdfl, Some(&first));
    let part = held(&after).parts().first().expect("one part");
    assert_eq!(part.since_floor(), base);
    let keys = |since| {
        part.changed_since(since)
            .map(|changes| changes.iter().map(|(_, key)| *key).collect::<Vec<_>>())
    };
    assert_eq!(keys(base), Some(vec![pe, ce]), "both appends, in order");
    assert_eq!(keys(base + 1), Some(vec![ce]), "only the second");
    assert_eq!(keys(base + 2), Some(Vec::new()), "already current");
    assert_eq!(keys(base + 3), None, "a generation never published");
    assert_eq!(keys(base - 1), None, "before the census was read whole");
}

/// **A FILE REPLACED BY RENAME, OR ONE WHOSE COUNTERS WENT BACKWARDS, IS READ
/// WHOLE**, never extended: the tail is a shortcut that must reach the same
/// census, and a file that was not only appended to cannot be shown to.
#[test]
fn a_replaced_or_rewritten_file_is_read_whole() {
    let dir = root("replaced");
    let ce = key(Segment::Fno, Some("2019-03-28-1150000-CE"), 3);
    let pe = key(Segment::Fno, Some("2019-03-28-1150000-PE"), 3);
    let disk = Disk::single(&dir, &[row(ce, 1, 10), row(pe, 2, 20)]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    let tails_before = manifest_reads::tails_of(&disk.path).len();

    // ANOTHER INODE, AND ONE MORE ENTRY THAN BEFORE: read whole.
    let mut other = Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
    for one in [row(ce, 4, 40), row(pe, 5, 50), row(ce, 6, 60)] {
        let _append = other.record_held(one).expect("records");
    }
    let tmp = disk.path.with_extension("man.writing");
    std::fs::write(&tmp, other.image()).expect("written");
    std::fs::rename(&tmp, &disk.path).expect("renamed");
    let reads = manifest_reads::of(&disk.path);
    let replaced = read_vendor_after(&dir, Vendor::Gdfl, Some(&first));
    assert_eq!(manifest_reads::of(&disk.path), reads + 1, "read whole");
    assert_eq!(manifest_reads::tails_of(&disk.path).len(), tails_before);
    same_census(&replaced, &read_vendor(&dir, Vendor::Gdfl));
    assert_eq!(held(&replaced).held(&ce).map(|one| one.entry.rows), Some(6));

    // THE SAME INODE, REWRITTEN SHORTER: read whole, never a tail.
    let mut shorter = Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
    let _append = shorter.record_held(row(pe, 7, 70)).expect("records");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&disk.path)
        .expect("open");
    std::io::Write::write_all(&mut &file, &shorter.image()).expect("rewritten");
    drop(file);
    let rewritten = read_vendor_after(&dir, Vendor::Gdfl, Some(&replaced));
    assert_eq!(manifest_reads::tails_of(&disk.path).len(), tails_before);
    same_census(&rewritten, &read_vendor(&dir, Vendor::Gdfl));
    assert_eq!(held(&rewritten).keys(), 1);
}

/// **A MIGRATED VENDOR IS READ FROM ITS PARTITIONS, EACH BOUNDED BY THE
/// CAPACITY ITS OWN HEADER DECLARES** (D-3316 items 3 and 7, DB-G08).
///
/// The FNO epoch here declares four times the single file's compiled
/// `MAX_ENTRIES` and is read; the reader is not bound to the writer's
/// constant. A file longer than its own declared capacity is refused by name,
/// unread. A migrated vendor's old single file is never read beside it.
#[test]
fn a_partition_is_bounded_by_its_own_header_capacity() {
    let dir = root("partition");
    let wide = MAX_ENTRIES * 4;
    let ce = key(Segment::Fno, Some("2019-03-28-1150000-CE"), 3);
    let spot = key(Segment::Index, None, 3);
    let _fno = Disk::partition(&dir, Segment::Fno, 1, wide, &[row(ce, 10, 100)]);
    let _index = Disk::partition(&dir, Segment::Index, 2, 1_048_576, &[row(spot, 20, 100)]);
    // AN OLD SINGLE FILE THAT DISAGREES: never read once migrated.
    let _single = Disk::single(&dir, &[row(ce, 999, 999)]);

    let census = read_vendor(&dir, Vendor::Gdfl);
    let loaded = held(&census);
    assert!(loaded.is_partitioned());
    assert_eq!(
        loaded.held(&ce).map(|one| one.entry.rows),
        Some(10),
        "the partition's, not the single file's"
    );
    assert_eq!(loaded.held(&spot).map(|one| one.entry.rows), Some(20));
    assert_eq!(loaded.keys(), 2);
    let parts: Vec<(Option<Segment>, Option<u32>)> = loaded
        .parts()
        .iter()
        .map(|part| (part.segment(), part.epoch()))
        .collect();
    assert_eq!(
        parts,
        vec![
            (Some(Segment::Index), Some(2)),
            (Some(Segment::Cash), None),
            (Some(Segment::Fno), Some(1)),
        ]
    );
    let room = census.room().expect("a loaded census has room");
    assert_eq!(room.segment, Some(Segment::Index), "the tightest part");
    assert_eq!(room.capacity, 1_048_576);
    assert_eq!(room.headroom, 1_048_575);
    let fno = loaded.part_of(Segment::Fno).expect("an FNO part");
    assert_eq!(
        fno.header().limit(),
        wide,
        "the header's capacity, past MAX_ENTRIES"
    );

    // A FILE LONGER THAN ITS OWN HEADER DECLARES: refused by name, unread.
    let refused_dir = root("partition-over");
    let tight = Disk::partition(&refused_dir, Segment::Fno, 1, 2, &[row(ce, 1, 10)]);
    let mut bytes = std::fs::read(&tight.path).expect("read");
    bytes.resize(bytes.len() + 3 * 128, 0);
    std::fs::write(&tight.path, &bytes).expect("grown");
    let over = read_vendor(&refused_dir, Vendor::Gdfl);
    match over.state {
        Census::Unreadable { ref reason, fault } => {
            assert_eq!(fault, Fault::Refused);
            assert!(reason.contains("declares 2 entries"), "{reason}");
        }
        Census::Absent | Census::Held { .. } => panic!("served past its capacity: {}", over.note()),
    }
}

/// **A PARTITION'S TAIL IS READ AS THE SINGLE FILE'S IS, AND A NEW EPOCH IS
/// READ WHOLE** (D-3316 items 2 and 4): a compaction installs another file,
/// which no tail of the old one may stand in for.
#[test]
fn a_partition_is_extended_by_its_tail_and_a_new_epoch_is_read_whole() {
    let dir = root("partition-tail");
    let ce = key(Segment::Fno, Some("2019-03-28-1150000-CE"), 3);
    let pe = key(Segment::Fno, Some("2019-03-28-1150000-PE"), 3);
    let mut fno = Disk::partition(&dir, Segment::Fno, 1, 64, &[row(ce, 1, 10)]);
    let first = read_vendor(&dir, Vendor::Gdfl);
    fno.append(&[row(pe, 2, 20), row(ce, 3, 30)]);
    let extended = read_vendor_after(&dir, Vendor::Gdfl, Some(&first));
    assert_eq!(manifest_reads::tails_of(&fno.path).last(), Some(&2));
    same_census(&extended, &read_vendor(&dir, Vendor::Gdfl));

    // THE NEXT EPOCH, AS A COMPACTION INSTALLS IT.
    let next = fno.manifest.compacted().expect("compacts");
    let path = epoch_path(&dir, Vendor::Gdfl, Segment::Fno, 2);
    std::fs::write(&path, next.image()).expect("installed");
    let reads = manifest_reads::of(&path);
    let moved = read_vendor_after(&dir, Vendor::Gdfl, Some(&extended));
    assert_eq!(
        manifest_reads::of(&path),
        reads + 1,
        "the new epoch is read whole"
    );
    assert_eq!(
        held(&moved).part_of(Segment::Fno).and_then(Part::epoch),
        Some(2)
    );
    same_census(&moved, &read_vendor(&dir, Vendor::Gdfl));
    assert_eq!(
        held(&moved).total_rows(),
        5,
        "the totals the replaced epoch held"
    );
}

/// **`capped` AND `behind` ARE NAMED IN THEIR OWN WORDS, AND NEITHER IS
/// "DEGRADED"** (D-3327 item 2; DB-G07, DB-G11, DB-G60, DB-G78).
///
/// The census refused every options day from 2019-11-14 while `/store.json`
/// said `held`. Below [`CAPPED_HEADROOM`] free entries the state is `capped`;
/// with census-refused days in the import journal it is `behind`, which wins,
/// and the room is still reported. A single file's capacity is the bound it
/// was written under, `MAX_ENTRIES`.
#[test]
fn capped_and_behind_are_named_and_never_called_degraded() {
    let ce = key(Segment::Fno, Some("2019-03-28-1150000-CE"), 3);
    let dir = root("capped");
    let _fno = Disk::partition(
        &dir,
        Segment::Fno,
        1,
        CAPPED_HEADROOM + 1,
        &[row(ce, 1, 10), row(ce, 2, 20)],
    );
    let census = read_vendor(&dir, Vendor::Gdfl);
    assert_eq!(census.degraded(), None, "near capacity is not damaged");
    let room = census.room().expect("room");
    assert_eq!(room.segment, Some(Segment::Fno));
    assert_eq!(room.headroom, CAPPED_HEADROOM - 1);
    assert_eq!(census.state_word(0), "capped");
    assert_eq!(
        census.state_word(3),
        "behind",
        "behind wins, the room is still on the headers"
    );
    let why = room.capped_reason().expect("a reason");
    assert!(why.contains("capped") && why.contains("FNO"), "{why}");

    let roomy = root("roomy");
    let _single = Disk::single(&roomy, &[row(ce, 1, 10)]);
    let census = read_vendor(&roomy, Vendor::Gdfl);
    let room = census.room().expect("room");
    assert_eq!(room.capacity, MAX_ENTRIES);
    assert_eq!(room.segment, None);
    assert_eq!(room.capped_reason(), None);
    assert_eq!(census.state_word(0), "held");
    assert_eq!(census.state_word(1), "behind");

    let absent = read_vendor(&root("nothing"), Vendor::Gdfl);
    assert_eq!(absent.room(), None);
    assert_eq!(
        absent.state_word(5),
        "absent",
        "nothing loaded is never behind"
    );
}
