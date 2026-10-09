//! ATTACK ROUND 2, STORE: the D-3140 last-stamp read meets a rotted last
//! record.
//!
//! D-3140 made `append` compare the header's `last_ts_micros` with the stamp
//! of record `n_valid - 1`, read WITHOUT the block verify, and refuse a
//! disagreement as `LastStampDisagrees` — "the header lies". When it is the
//! RECORD that rotted, not the header, that is a named reason and the wrong
//! one: it sends an operator to the header slot while the damage sits in a
//! sealed block the sidecar already proves is damaged. And it ran BEFORE the
//! D-0910 verify of a partially covered tail block, so even the case D-0910
//! names (`BlockChecksum`) answered "header" instead whenever the rot hit the
//! eight stamp bytes. D-3181.
//!
//! Every bit of the last record's stamp, in a partial and in a full tail
//! block. splitmix64-free: the domain is enumerated.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, StoreError};
use store::format::{Bar, FormatError, HEADER_LEN, OI_NULL, RECORD_LEN};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

const MINUTE: i64 = 60_000_000;
const SYMBOL: u32 = 26_000;
/// Records in one V2 checksum block.
const PER_BLOCK: u64 = 73;

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-attack-r2-store-{}-{tag}",
            std::process::id()
        ));
        drop(fs::remove_dir_all(&root));
        fs::create_dir_all(&root).expect("scratch root");
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

fn june() -> YearMonth {
    YearMonth::new(2024, 6).expect("2024-06")
}

fn path() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: june(),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

fn minute(m: i64) -> i64 {
    june().ist_bounds_micros().0 + m * MINUTE
}

fn bar(m: i64) -> Bar {
    let p = 2_400_000 + m * 7;
    Bar {
        ts_micros: minute(m),
        open: p,
        high: p + 10,
        low: p - 10,
        close: p + 1,
        volume: 0,
        open_interest: OI_NULL,
    }
}

fn both(root: &Path) -> (Vec<u8>, Vec<u8>) {
    (
        fs::read(path().to_path_buf(root)).expect("bars"),
        fs::read(path().with_file(FileKind::Checksums).to_path_buf(root)).unwrap_or_default(),
    )
}

/// Byte offset of record `index` in a V2 bar file.
fn offset(index: u64) -> usize {
    usize::try_from(HEADER_LEN + index * RECORD_LEN as u64).expect("fits")
}

/// Seeds `held` minutes, flips one bit of the last record's stamp, and offers
/// the next minute. Answers the append's result and whether the month moved.
fn rot_stamp_and_append(
    tag: &str,
    held: i64,
    byte: usize,
    bit: u8,
) -> (Result<Appended, StoreError>, bool) {
    let scratch = Scratch::new(tag);
    let mut file = BarFile::open_or_create(&scratch.0, path(), SYMBOL).expect("create");
    let bars: Vec<Bar> = (0..held).map(bar).collect();
    file.append(&bars).expect("seed");
    drop(file);
    let at = offset(u64::try_from(held - 1).unwrap());
    let on_disk = path().to_path_buf(&scratch.0);
    let mut bytes = fs::read(&on_disk).expect("bars");
    assert_eq!(
        i64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()),
        minute(held - 1),
        "the premise: record {} starts at byte {at}",
        held - 1
    );
    bytes[at + byte] ^= 1 << bit;
    fs::write(&on_disk, bytes).expect("rot");
    let before = both(&scratch.0);
    let got = match BarFile::open_or_create(&scratch.0, path(), SYMBOL) {
        Ok(mut file) => file.append(&[bar(held)]),
        Err(why) => Err(why),
    };
    (got, both(&scratch.0) != before)
}

/// **A ROTTED LAST STAMP IS NAMED AS ROT, NOT AS A LYING HEADER.** For every
/// one of the 64 bits of the last record's stamp, in a tail block holding one
/// record, a few, and exactly a full block: the append is refused, the month
/// is byte-identical, and the reason is the block checksum the sidecar holds
/// — never `LastStampDisagrees`, whose header is in fact right.
#[test]
fn a_rotted_last_stamp_is_refused_as_a_block_checksum_never_as_a_header_disagreement() {
    let mut tried = 0usize;
    for held in [1_i64, 10, 74, 73, 146] {
        for byte in 0..8 {
            for bit in 0..8u8 {
                tried += 1;
                let (got, moved) =
                    rot_stamp_and_append(&format!("stamp-{held}-{byte}-{bit}"), held, byte, bit);
                assert!(
                    !moved,
                    "held {held}, byte {byte} bit {bit}: the month moved: {got:?}"
                );
                let block = (u64::try_from(held).unwrap() - 1) / PER_BLOCK;
                assert!(
                    matches!(got, Err(StoreError::BlockChecksum { block: b, .. }) if b == block),
                    "held {held}, byte {byte} bit {bit}: a rotted stamp in block {block} \
                     answered {got:?}"
                );
                assert!(
                    !matches!(
                        got,
                        Err(StoreError::Format {
                            source: FormatError::LastStampDisagrees { .. },
                            ..
                        })
                    ),
                    "the header is right; the record rotted"
                );
            }
        }
    }
    assert_eq!(tried, 5 * 64);
}

/// **D-0910 STILL HOLDS FOR ROT OUTSIDE THE STAMP.** A full tail block whose
/// last record rotted in a PRICE is not read by the next append, which
/// commits; the damage stays sealed for a reader to refuse. The D-3181 verify
/// runs only when the stamps disagree.
#[test]
fn rot_outside_the_stamp_of_a_full_tail_block_still_lets_the_next_append_commit() {
    let held = i64::try_from(PER_BLOCK).unwrap();
    for byte in 8..RECORD_LEN {
        let scratch = Scratch::new(&format!("price-{byte}"));
        let mut file = BarFile::open_or_create(&scratch.0, path(), SYMBOL).expect("create");
        let bars: Vec<Bar> = (0..held).map(bar).collect();
        file.append(&bars).expect("seed");
        drop(file);
        let on_disk = path().to_path_buf(&scratch.0);
        let mut bytes = fs::read(&on_disk).expect("bars");
        bytes[offset(PER_BLOCK - 1) + byte] ^= 0b0100_0000;
        fs::write(&on_disk, bytes).expect("rot");
        let mut file = BarFile::open_or_create(&scratch.0, path(), SYMBOL).expect("reopen");
        assert_eq!(
            file.append(&[bar(held)]),
            Ok(Appended::Committed {
                first_index: PER_BLOCK,
                n_valid: PER_BLOCK + 1,
            }),
            "byte {byte}"
        );
    }
}
