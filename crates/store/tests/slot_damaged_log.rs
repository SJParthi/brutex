//! `store.header.slot_damaged` — the line a read beside a damaged header slot
//! writes, read back off the disk (R4-09a, D-6240).
//!
//! One test in its own binary, because `telemetry::install` is once per
//! process: every line in the sink below was written by the calls this file
//! makes, so the count is exact. Invented bars only — closes of seven rupees
//! for an index that has never traded there.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, FLAG_CHECKSUMS, HEADER_LEN, OI_NULL, SLOT_LEN};
use store::header::{Header, MonthVerdict, SlotDamage, SlotState};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use telemetry::OwnedValue;

/// Byte offset of the slot checksum within a slot.
const OFF_CRC: usize = 56;
/// 2024-06-03 09:15 IST, in microseconds.
const T0: i64 = 1_717_386_300_000_000;
/// The symbol id the month is opened under.
const SYMBOL: u32 = 26_000;

/// A fresh scratch directory of this test's own.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "brutex-slot-damaged-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    ));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// The NIFTY spot one-second month of June 2024, on the GDFL feed.
fn month() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2024, 6).expect("a month"),
        file: FileKind::Bars,
    })
    .expect("a store path")
}

/// `count` invented one-second bars from second `from` of the session.
fn bars(from: i64, count: i64) -> Vec<Bar> {
    (from..from + count)
        .map(|n| Bar {
            ts_micros: T0 + n * 1_000_000,
            open: 700 + n,
            high: 710 + n,
            low: 690 + n,
            close: 700 + n,
            volume: 0,
            open_interest: OI_NULL,
        })
        .collect()
}

/// A two-slot header region: genesis, then one commit of 10 records.
fn two_slots() -> Vec<u8> {
    let mut region = vec![0u8; 32_768];
    let genesis = Header::genesis(SYMBOL, 1, FLAG_CHECKSUMS);
    let next = genesis.advance(10, T0, T0 + 9_000_000).expect("fits");
    for commit in [
        genesis.commit().expect("commits"),
        next.commit().expect("commits"),
    ] {
        let at = usize::try_from(commit.offset).expect("in range");
        region[at..at + SLOT_LEN].copy_from_slice(&commit.bytes);
    }
    region
}

fn uint(record: &telemetry::Record, key: &str) -> u64 {
    match record.field(key) {
        Some(OwnedValue::Int(value)) => u64::try_from(*value).expect("non-negative"),
        Some(OwnedValue::Uint(value)) => *value,
        other => panic!("{key}: {other:?}"),
    }
}

fn text<'r>(record: &'r telemetry::Record, key: &str) -> &'r str {
    match record.field(key) {
        Some(OwnedValue::Str(value)) => value,
        other => panic!("{key}: {other:?}"),
    }
}

/// A month whose NEWEST slot fails its checksum opens at the older generation
/// — every reader's bytes unchanged — and that open writes exactly one
/// `store.header.slot_damaged` line at `Error`, naming the path, the slot, the
/// reason and the generation read. A clean read and a first-commit read write
/// none. On `feat/db-fix` at 371b1a506 the damaged open wrote nothing at all.
#[test]
fn a_read_beside_a_damaged_newest_slot_writes_one_error_line_and_a_clean_read_none() {
    let log = scratch("log");
    let root = scratch("root");
    let sink =
        telemetry::install(&telemetry::Config::new(&log).with_min_level(telemetry::Level::Trace))
            .expect("nothing else in this binary installs a sink");

    // 1. Clean, and first-commit: silent.
    let clean = two_slots();
    assert_eq!(
        Header::read_region(&clean, HEADER_LEN + 560).map(|h| h.generation),
        Ok(1)
    );
    let mut first = vec![0u8; 32_768];
    first[..SLOT_LEN].copy_from_slice(&clean[..SLOT_LEN]);
    assert_eq!(
        Header::read_region(&first, HEADER_LEN).map(|h| h.generation),
        Ok(0)
    );

    // 2. The region alone, newest slot's checksum flipped: one line, no path.
    let mut flipped = clean.clone();
    flipped[16_384 + OFF_CRC] ^= 0x80;
    assert_eq!(
        Header::read_region(&flipped, HEADER_LEN + 560).map(|h| h.generation),
        Ok(0)
    );

    // 3. A real month through the store's own writer and reader.
    let path = month();
    let file_path: PathBuf = {
        let mut file = BarFile::open_or_create(&root, path, SYMBOL).expect("open");
        let _ = file.append(&bars(0, 6)).expect("append 1");
        let _ = file.append(&bars(6, 4)).expect("append 2");
        assert_eq!(file.header().generation, 2);
        file.path().to_path_buf()
    };
    let before = BarFile::open_existing(&root, path, SYMBOL).expect("whole");
    let whole = before.slot_audit();
    assert!(whole.is_some_and(|audit| audit.is_clean()), "{whole:?}");
    assert_eq!(before.header().n_valid, 10);
    drop(before);
    flip_byte(&file_path, OFF_CRC);
    let after = BarFile::open_existing(&root, path, SYMBOL).expect("opens, as before");
    assert_eq!(
        after.header().n_valid,
        6,
        "the reader's bytes are unchanged: generation 1's six records"
    );
    assert_eq!(after.header().generation, 1);
    let audit = after
        .slot_audit()
        .expect("a raw month has a slot audit; only the sealed form has none");
    assert!(matches!(
        audit.slots[0],
        SlotState::Damaged {
            reason: SlotDamage::Refused(store::format::FormatError::SlotChecksum { .. }),
            generation: None
        }
    ));
    assert!(matches!(
        audit.verdict,
        MonthVerdict::Reverted {
            chosen_generation: 1,
            damaged_slot: 0,
            damaged_generation: None,
            ..
        }
    ));
    drop(after);

    let found = telemetry::tail(&log, sink.keep_files(), &telemetry::Query::last(256));
    let lines: Vec<&telemetry::Record> = found
        .records
        .iter()
        .filter(|record| record.target == "store.header.slot_damaged")
        .collect();
    assert_eq!(
        lines.len(),
        2,
        "one per damaged read, none for the clean and first-commit reads: {:?}",
        found
            .records
            .iter()
            .map(|r| (r.target.clone(), r.message.clone()))
            .collect::<Vec<_>>()
    );
    for line in &lines {
        names_a_crc_revert(line);
    }
    // Told apart by what they carry, not by the order the tail hands them
    // back in: the region read has no path to name, the month open has one.
    let (region_lines, opened): (Vec<&telemetry::Record>, Vec<&telemetry::Record>) = lines
        .iter()
        .copied()
        .partition(|line| line.field("path").is_none());
    assert_eq!((region_lines.len(), opened.len()), (1, 1), "{lines:?}");
    let (region_line, opened) = (region_lines[0], opened[0]);
    // The region read: slot 1, generation 0, no path to name.
    assert_eq!(uint(region_line, "slot"), 1);
    assert_eq!(uint(region_line, "chosen_generation"), 0);
    // The month opened: slot 0, generation 1, and its path.
    assert_eq!(uint(opened, "slot"), 0);
    assert_eq!(uint(opened, "chosen_generation"), 1);
    assert_eq!(text(opened, "path"), file_path.display().to_string());
    assert_eq!(
        uint(opened, "file_len"),
        std::fs::metadata(&file_path).expect("the month").len()
    );
    let _ = std::fs::remove_dir_all(&log);
    let _ = std::fs::remove_dir_all(&root);
}

/// One `store.header.slot_damaged` line naming a checksum-failed slot: `Error`,
/// verdict `reverted`, reason `crc`, and no damaged generation, because a slot
/// whose checksum fails has none the format vouches for (D-6241).
fn names_a_crc_revert(line: &telemetry::Record) {
    assert_eq!(line.level, telemetry::Level::Error);
    assert_eq!(line.message, "header slot damaged");
    assert_eq!(text(line, "verdict"), "reverted");
    assert_eq!(text(line, "reason"), "crc");
    assert!(text(line, "why").starts_with("crc: "), "{line:?}");
    assert!(line.field("damaged_generation").is_none(), "{line:?}");
}

/// Flips one byte of a file in place: the damage under test, made by the test.
fn flip_byte(path: &Path, at: usize) {
    let mut bytes = std::fs::read(path).expect("the month");
    bytes[at] ^= 0x01;
    std::fs::write(path, bytes).expect("written back");
}
