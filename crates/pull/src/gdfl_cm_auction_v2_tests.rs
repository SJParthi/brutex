//! AG-04 (store-3): the `.auc` sidecar's format version 2. A flipped bit in
//! any field, two records swapped, a header byte bent, a torn tail and a
//! garbage file are each refused by name with the record's ordinal; a
//! version-1 file is read and labelled unchecked, and an append to one stays
//! version 1; a new file syncs its directory; a failed write is undone; a
//! reader beside an append never sees a torn tail.
//!
//! Every value is invented at run time and no vendor row is quoted: the
//! prices are chosen so they cannot be mistaken for real ones.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::{Path, PathBuf};

use brutex_core::instrument::{Exchange, InstrumentKey};

use super::*;
use crate::gdfl_cm::CmRefusal;
use crate::gdfl_fixtures::scratch;

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn at(h: u32, m: u32, s: u32) -> u32 {
    h * 3_600 + m * 60 + s
}

fn key() -> InstrumentKey {
    InstrumentKey::cash(Exchange::Nse, "SYNTHAUC").unwrap()
}

/// An invented print: ₹4,446.65-ish prices no share of this name ever had.
fn print(sod: u32, price: i64, quantity: u64) -> Print {
    Print {
        sod,
        price,
        quantity,
        rows: 1,
        line: 2,
    }
}

/// Two prints of one day: the match and a straggler a second later.
fn two() -> [Print; 2] {
    [
        print(at(15, 29, 3), 444_665, 98_765),
        print(at(15, 29, 4), 444_665, 1_234),
    ]
}

/// A fresh store with `day`'s `prints` appended, and the sidecar's path.
fn filed(tag: &str, on: Day, prints: &[Print]) -> (PathBuf, PathBuf) {
    let root = scratch(tag);
    assert_eq!(
        append(&root, &key(), on, prints).unwrap().written,
        prints.len()
    );
    let path = path(&root, &key(), on).unwrap();
    (root, path)
}

/// A version-1 file of `records`, as the module wrote before AG-04.
fn version_1(records: &[Record]) -> Vec<u8> {
    let mut bytes = header_of(VERSION_1).to_vec();
    for record in records {
        bytes.extend_from_slice(&record.to_bytes());
    }
    bytes
}

/// The refusal `parse` gives `bytes`, which it must give.
fn refused(bytes: &[u8]) -> Refused {
    match parse(bytes) {
        Ok(sidecar) => panic!(
            "{} records were served from bent bytes",
            sidecar.records.len()
        ),
        Err(refused) => refused,
    }
}

fn sidecar_why(result: Result<Appended, CmRefusal>) -> String {
    match result {
        Err(CmRefusal::AuctionSidecar { why }) => why,
        other => panic!("not a sidecar refusal: {other:?}"),
    }
}

#[test]
fn a_flipped_bit_in_price_quantity_or_time_is_refused_by_name() {
    let on = day(2026, 9, 23);
    let (_root, path) = filed("ag04-flip", on, &two());
    let good = std::fs::read(&path).unwrap();
    let sidecar = parse(&good).unwrap();
    assert_eq!(sidecar.version, VERSION);
    assert!(sidecar.checked());
    assert_eq!(sidecar.unchecked_why(), None);
    assert_eq!(sidecar.records.len(), 2);
    // Record 1 begins at byte 16 + 32. Each field, and the checksum itself.
    let record_1 = HEADER_LEN + RECORD_LEN;
    for (field, offset) in [
        ("time", 0),
        ("time, high byte", 7),
        ("price", 8),
        ("quantity", 16),
        ("rows", 24),
        ("checksum", 28),
    ] {
        for bit in [0_u8, 7] {
            let mut bent = good.clone();
            bent[record_1 + offset] ^= 1 << bit;
            let refusal = refused(&bent);
            assert_eq!(refusal.ordinal, Some(1), "{field} bit {bit}: {refusal}");
            assert!(
                refusal.why.contains("record 1") && refusal.why.contains("checksum"),
                "{field}: {refusal}"
            );
            // The reader names the file too.
            std::fs::write(&path, &bent).unwrap();
            let read_refusal = read(&path).unwrap_err();
            assert_eq!(read_refusal.ordinal, Some(1));
            assert!(read_refusal.why.contains(&path.display().to_string()));
        }
    }
    // A flip in record 0 names record 0.
    let mut bent = good.clone();
    bent[HEADER_LEN + 9] ^= 0x10;
    assert_eq!(refused(&bent).ordinal, Some(0));
    std::fs::write(&path, &good).unwrap();
    assert_eq!(read(&path).unwrap().unwrap(), sidecar, "restored, served");
}

#[test]
fn two_swapped_records_are_refused() {
    let on = day(2026, 9, 23);
    let (_root, path) = filed("ag04-swap", on, &two());
    let good = std::fs::read(&path).unwrap();
    let (r0, r1) = (HEADER_LEN, HEADER_LEN + RECORD_LEN);
    let mut swapped = good.clone();
    swapped[r0..r1].copy_from_slice(&good[r1..r1 + RECORD_LEN]);
    swapped[r1..r1 + RECORD_LEN].copy_from_slice(&good[r0..r1]);
    let refusal = refused(&swapped);
    assert_eq!(refusal.ordinal, Some(0), "{refusal}");
    assert!(
        refusal.why.contains("moved from another position"),
        "{refusal}"
    );
    // One record copied over another is refused at the copy's position.
    let mut copied = good.clone();
    copied[r1..r1 + RECORD_LEN].copy_from_slice(&good[r0..r1]);
    assert_eq!(refused(&copied).ordinal, Some(1));
    // The same record encoded at another ordinal is other bytes.
    let record = Record::of(on, &two()[0]);
    assert_ne!(record.encode(VERSION, 0), record.encode(VERSION, 1));
    assert_eq!(record.encode(VERSION_1, 0), record.encode(VERSION_1, 9));
}

#[test]
fn a_flipped_header_byte_is_refused() {
    let on = day(2026, 9, 23);
    let (_root, path) = filed("ag04-head", on, &two());
    let good = std::fs::read(&path).unwrap();
    for byte in 0..HEADER_LEN {
        let mut bent = good.clone();
        bent[byte] ^= 0x01;
        let refusal = refused(&bent);
        assert_eq!(refusal.ordinal, None, "header byte {byte}: {refusal}");
    }
    let mut v3 = header_of(3).to_vec();
    v3.extend_from_slice(&good[HEADER_LEN..]);
    assert!(refused(&v3).why.contains("format version 3"));
    // The version-2 header is not the version-1 one with a new number: its
    // last four bytes are the checksum of the first twelve.
    assert_eq!(&good[..HEADER_LEN], &header());
    assert_eq!(
        u32::from_le_bytes(good[12..16].try_into().unwrap()),
        store::crc::crc32c(&good[..12])
    );
}

#[test]
fn a_version_1_file_is_read_and_labelled_unchecked() {
    let on = day(2026, 9, 23);
    let records: Vec<Record> = two().iter().map(|p| Record::of(on, p)).collect();
    let bytes = version_1(&records);
    let sidecar = parse(&bytes).unwrap();
    assert_eq!(sidecar.version, VERSION_1);
    assert_eq!(sidecar.records, records);
    assert!(!sidecar.checked());
    assert!(
        sidecar
            .unchecked_why()
            .unwrap()
            .contains("version 1 carries no checksum")
    );
    // THE HONEST LIMIT: a flipped price in version 1 is served as written.
    let mut bent = bytes.clone();
    bent[HEADER_LEN + RECORD_LEN + 8] ^= 0x01;
    let served = parse(&bent).unwrap();
    assert_ne!(served.records[1].price_paisa, records[1].price_paisa);
    assert!(!served.checked(), "and it is labelled so");
    // Its shape is still checked: a reserved field set is refused by name.
    let mut reserved = bytes.clone();
    reserved[HEADER_LEN + RECORD_LEN + 31] = 1;
    let refusal = refused(&reserved);
    assert_eq!(refusal.ordinal, Some(1));
    assert!(refusal.why.contains("reserved"), "{refusal}");
    assert_eq!(
        Record::from_bytes(&reserved[HEADER_LEN..HEADER_LEN + 32]),
        Some(records[0])
    );
    assert_eq!(Record::from_bytes(&reserved[HEADER_LEN + 32..]), None);
}

#[test]
fn an_append_to_a_version_1_file_stays_version_1() {
    let (d1, d2) = (day(2026, 9, 23), day(2026, 9, 24));
    let root = scratch("ag04-v1-append");
    let path = path(&root, &key(), d1).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let held = version_1(&[Record::of(d1, &two()[0])]);
    std::fs::write(&path, &held).unwrap();
    assert_eq!(append(&root, &key(), d2, &two()).unwrap().written, 2);
    let after = std::fs::read(&path).unwrap();
    assert_eq!(&after[..held.len()], &held[..], "no held byte rewritten");
    assert_eq!(after.len(), held.len() + 2 * RECORD_LEN);
    let sidecar = parse(&after).unwrap();
    assert_eq!(sidecar.version, VERSION_1, "no mixed file");
    assert_eq!(sidecar.records.len(), 3);
    for record in after[HEADER_LEN..].chunks(RECORD_LEN) {
        assert_eq!(&record[28..], &[0; 4], "version-1 records, reserved zero");
    }
    // Offered again: nothing written, the file unchanged.
    assert_eq!(append(&root, &key(), d2, &two()).unwrap().written, 0);
    assert_eq!(std::fs::read(&path).unwrap(), after);
}

#[test]
fn a_new_auc_file_syncs_its_directory() {
    let (d1, d2) = (day(2026, 9, 23), day(2026, 9, 24));
    let root = scratch("ag04-dirsync");
    seam::SYNCED.with(|synced| synced.borrow_mut().clear());
    append(&root, &key(), d1, &two()).unwrap();
    let path = path(&root, &key(), d1).unwrap();
    let synced = seam::SYNCED.with(|synced| synced.borrow().clone());
    let dir = path.parent().unwrap().to_path_buf();
    assert_eq!(synced.first(), Some(&dir), "the file's own directory first");
    // Every directory the append made is synced in its parent: down from
    // `bars` under the scratch root.
    for made in [dir.parent().unwrap(), &root.join("bars")] {
        assert!(
            synced.iter().any(|d| d == made),
            "{} not synced: {synced:?}",
            made.display()
        );
    }
    // An append to a file that exists syncs no directory.
    seam::SYNCED.with(|synced| synced.borrow_mut().clear());
    append(&root, &key(), d2, &two()).unwrap();
    assert!(seam::SYNCED.with(|synced| synced.borrow().is_empty()));
}

#[test]
fn a_header_only_file_and_one_record_are_read() {
    for version in [VERSION_1, VERSION] {
        let sidecar = parse(&header_of(version)).unwrap();
        assert_eq!((sidecar.version, sidecar.records.len()), (version, 0));
    }
    // An append to a header-only version-2 file starts at ordinal 0.
    let on = day(2026, 9, 23);
    let root = scratch("ag04-header-only");
    let path = path(&root, &key(), on).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, header()).unwrap();
    assert_eq!(append(&root, &key(), on, &two()[..1]).unwrap().written, 1);
    let sidecar = read(&path).unwrap().unwrap();
    assert_eq!(sidecar.records, vec![Record::of(on, &two()[0])]);
    assert_eq!(
        read(&root.join("absent.auc")).unwrap(),
        None,
        "no file, no sidecar"
    );
}

#[test]
fn a_days_maximum_prints_and_extreme_values_round_trip() {
    let on = day(2026, 9, 23);
    // Every second of the band [15:15:00, 15:35:00) twice, at the field
    // extremes: 2,400 prints, far past any day measured.
    let mut prints = Vec::new();
    for sod in at(15, 15, 0)..at(15, 35, 0) {
        prints.push(Print {
            sod,
            price: i64::MAX,
            quantity: u64::MAX,
            rows: u32::MAX,
            line: 2,
        });
        prints.push(print(sod, 1, 1));
    }
    let (root, path) = filed("ag04-max", on, &prints);
    let sidecar = read(&path).unwrap().unwrap();
    assert_eq!(sidecar.records.len(), prints.len());
    assert_eq!(sidecar.records[0].price_paisa, i64::MAX);
    assert_eq!(sidecar.records[0].quantity, u64::MAX);
    assert_eq!(sidecar.records[0].rows, u32::MAX);
    assert_eq!(append(&root, &key(), on, &prints).unwrap().written, 0);
}

#[test]
fn two_prints_in_one_second_are_both_kept_in_file_order() {
    let on = day(2026, 9, 23);
    let same = [
        print(at(15, 29, 3), 444_665, 7),
        print(at(15, 29, 3), 444_670, 9),
        print(at(15, 29, 2), 444_660, 11),
    ];
    let (_root, path) = filed("ag04-same-second", on, &same);
    let sidecar = read(&path).unwrap().unwrap();
    let got: Vec<(i64, u64)> = sidecar
        .records
        .iter()
        .map(|r| (r.price_paisa, r.quantity))
        .collect();
    assert_eq!(got, vec![(444_665, 7), (444_670, 9), (444_660, 11)]);
}

#[test]
fn an_out_of_order_record_is_refused_and_the_file_unchanged() {
    let (d1, d2) = (day(2026, 9, 23), day(2026, 9, 24));
    let (root, path) = filed("ag04-order", d2, &two());
    let before = std::fs::read(&path).unwrap();
    assert!(sidecar_why(append(&root, &key(), d1, &two())).contains("never an insertion"));
    assert_eq!(std::fs::read(&path).unwrap(), before, "unchanged");
    // A file whose day goes backwards, every checksum good, is refused at
    // the record that goes back.
    let mut bytes = header().to_vec();
    bytes.extend_from_slice(&Record::of(d2, &two()[0]).encode(VERSION, 0));
    bytes.extend_from_slice(&Record::of(d1, &two()[0]).encode(VERSION, 1));
    let refusal = refused(&bytes);
    assert_eq!(refusal.ordinal, Some(1));
    assert!(refusal.why.contains("out of order"), "{refusal}");
    std::fs::write(&path, &bytes).unwrap();
    assert!(sidecar_why(append(&root, &key(), day(2026, 9, 25), &two())).contains("out of order"));
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "never repaired");
}

#[test]
fn a_torn_tail_or_a_truncated_header_is_refused_by_ordinal() {
    let on = day(2026, 9, 23);
    let (root, path) = filed("ag04-torn", on, &two());
    let good = std::fs::read(&path).unwrap();
    // Killed mid-append: a part of record 2.
    let mut torn = good.clone();
    torn.extend_from_slice(&Record::of(on, &two()[0]).encode(VERSION, 2)[..17]);
    let refusal = refused(&torn);
    assert_eq!(refusal.ordinal, Some(2), "{refusal}");
    assert!(
        refusal
            .why
            .contains("record 2 is torn (17 of its 32 bytes)"),
        "{refusal}"
    );
    std::fs::write(&path, &torn).unwrap();
    assert!(sidecar_why(append(&root, &key(), day(2026, 9, 24), &two())).contains("torn"));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        torn,
        "refused, never repaired"
    );
    // A whole-length tail of zeros (a size that reached the disk before its
    // bytes did) fails its checksum at its ordinal.
    let mut zeros = good.clone();
    zeros.extend_from_slice(&[0; RECORD_LEN]);
    assert_eq!(refused(&zeros).ordinal, Some(2));
    // A truncated header, and an empty file.
    let short = refused(&good[..10]);
    assert!(short.why.contains("truncated header"), "{short}");
    assert!(refused(&[]).why.contains("holds no header"));
}

#[test]
fn a_garbage_file_at_the_auc_path_is_refused_by_name_and_kept() {
    let on = day(2026, 9, 23);
    let root = scratch("ag04-garbage");
    let path = path(&root, &key(), on).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let garbage: Vec<u8> = (0_u8..=255).cycle().take(100).collect();
    std::fs::write(&path, &garbage).unwrap();
    let refusal = read(&path).unwrap_err();
    assert!(refusal.why.contains("magic"), "{refusal}");
    assert!(refusal.why.contains(&path.display().to_string()));
    assert!(sidecar_why(append(&root, &key(), on, &two())).contains("magic"));
    assert_eq!(std::fs::read(&path).unwrap(), garbage);
}

#[test]
fn a_reader_beside_a_concurrent_append_sees_whole_days_only() {
    let root = scratch("ag04-concurrent");
    let first = day(2026, 9, 1);
    let path = path(&root, &key(), first).unwrap();
    let prints = [
        print(at(15, 29, 1), 333_331, 3),
        print(at(15, 29, 2), 333_332, 5),
        print(at(15, 29, 3), 333_333, 7),
    ];
    let writer = {
        let root = root.clone();
        std::thread::spawn(move || {
            for d in 1..=28 {
                append(&root, &key(), day(2026, 9, d), &prints).unwrap();
            }
        })
    };
    let mut seen = 0;
    let mut reads = 0_u32;
    while !writer.is_finished() || reads == 0 {
        match read(&path) {
            Ok(None) => {}
            Ok(Some(sidecar)) => {
                let n = sidecar.records.len();
                assert_eq!(n % prints.len(), 0, "a part of a day was seen: {n}");
                assert!(n >= seen, "a read went back: {n} after {seen}");
                seen = n;
            }
            // THE ONE WINDOW: the first append has made the file and not yet
            // taken its lock. Named, and only before any record exists.
            Err(refusal) if seen == 0 && refusal.why.contains("holds no header") => {}
            Err(refusal) => panic!("a reader beside an append was refused: {refusal}"),
        }
        reads += 1;
    }
    writer.join().unwrap();
    assert_eq!(read(&path).unwrap().unwrap().records.len(), 28 * 3);
}

#[test]
fn a_full_disk_on_create_or_append_leaves_nothing_half_written() {
    let (d1, d2) = (day(2026, 9, 23), day(2026, 9, 24));
    let root = scratch("ag04-full");
    let path = path(&root, &key(), d1).unwrap();
    // ON CREATE: the write stops ten bytes into the header.
    seam::FAIL_AFTER.with(|fail| fail.set(Some(10)));
    let why = sidecar_why(append(&root, &key(), d1, &two()));
    assert!(why.contains("no space left on device"), "{why}");
    assert!(why.contains("nothing is half-written"), "{why}");
    assert_eq!(std::fs::read(&path).unwrap(), Vec::<u8>::new());
    assert!(read(&path).unwrap_err().why.contains("holds no header"));
    // The retry writes the whole file, version 2.
    assert_eq!(append(&root, &key(), d1, &two()).unwrap().written, 2);
    let held = std::fs::read(&path).unwrap();
    assert_eq!(parse(&held).unwrap().version, VERSION);
    // ON APPEND: the write stops forty bytes into the next day's records.
    seam::FAIL_AFTER.with(|fail| fail.set(Some(40)));
    let why = sidecar_why(append(&root, &key(), d2, &two()));
    assert!(
        why.contains(&format!("its {} bytes again", held.len())),
        "{why}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), held, "byte for byte");
    assert_eq!(append(&root, &key(), d2, &two()).unwrap().written, 2);
    assert_eq!(read(&path).unwrap().unwrap().records.len(), 4);
}

#[test]
fn a_lock_held_past_the_wait_is_refused_by_name() {
    let on = day(2026, 9, 23);
    let (_root, path) = filed("ag04-lock", on, &two());
    let holder = std::fs::File::open(&path).unwrap();
    let held = Flock::try_lock(&holder, path.clone()).unwrap();
    let mine = std::fs::File::open(&path).unwrap();
    let why = hold(&mine, &path, Hold::Shared, Duration::from_millis(5)).unwrap_err();
    assert!(why.contains("past 5 ms"), "{why}");
    held.release().unwrap();
    assert!(hold(&mine, &path, Hold::Shared, Duration::from_millis(5)).is_ok());
}

/// Every day of one month in one file: the records' IST days, as `parse`
/// orders them, are the days appended.
#[test]
fn a_records_ist_day_is_the_day_it_was_filed_on() {
    let on = day(2026, 9, 30);
    let record = Record::of(on, &print(at(23, 59, 59), 9, 9));
    assert_eq!(ist_day(record.t_micros), i64::from(on.days_from_epoch()));
    let record = Record::of(on, &print(0, 9, 9));
    assert_eq!(ist_day(record.t_micros), i64::from(on.days_from_epoch()));
    assert!(Path::new(&path(Path::new("/s"), &key(), on).unwrap()).ends_with("1s/2026-09.auc"));
}
