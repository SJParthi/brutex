//! The full-block pass (DB-B74, WP-81) on owned months of invented bars.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use std::fs;
use store::format::{Bar, OI_NULL};
use store::path::{Timeframe, YearMonth};

/// An owned store with one NIFTY index minute month of `n` invented bars.
fn month(tag: &str, n: usize) -> (std::path::PathBuf, StorePath<'static>) {
    let root = std::env::temp_dir().join(format!(
        "brutex-scrub-blocks-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let path = StorePath::new(PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2025, 5).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    let start = i64::from(
        crate::session::Day::new(2025, 5, 1)
            .unwrap()
            .days_from_epoch(),
    ) * 86_400_000_000;
    let mut writer = BarFile::open_or_create(&root, path, 7).unwrap();
    let rows: Vec<Bar> = (0..n)
        .map(|i| {
            let nth = i64::try_from(i).unwrap();
            Bar {
                ts_micros: start + nth * 60_000_000,
                open: 222_222 + nth,
                high: 222_322 + nth,
                low: 222_122 + nth,
                close: 222_222 + nth,
                volume: 1,
                open_interest: OI_NULL,
            }
        })
        .collect();
    writer.append(&rows).unwrap();
    (root, path)
}

/// Flips one byte inside record `index`.
fn rot(root: &std::path::Path, path: StorePath, index: u64) {
    let file = BarFile::open_existing(root, path, 7).unwrap();
    let at = usize::try_from(file.layout().offset_of(index).unwrap()).unwrap();
    let bin = file.path().to_path_buf();
    drop(file);
    let mut bytes = fs::read(&bin).unwrap();
    bytes[at + 8] ^= 0x5a;
    fs::write(&bin, bytes).unwrap();
}

/// **ONE ROTTED BLOCK IN THE MIDDLE IS FOUND BY A PASS (DB-B74).** The
/// first-and-last scrub agrees with the damaged month; the block pass names
/// the one damaged block, its records, and nothing else; a bounded pass
/// resumes where it stopped and the two halves find the same block.
#[test]
fn a_rotted_middle_block_is_found_by_a_pass_and_missed_by_the_ends_scrub() {
    let (root, path) = month("rot", 730);
    rot(&root, path, 300);
    let file = BarFile::open_existing(&root, path, 7).unwrap();
    assert_eq!(file.read_record(0).map(|bar| bar.close), Ok(222_222));
    assert!(file.read_record(729).is_ok(), "the ends read");

    let whole = blocks(&file, 0, u64::MAX);
    assert_eq!((whole.blocks, whole.checked, whole.next), (10, 10, None));
    assert_eq!(whole.bad.len(), 1, "{:?}", whole.bad);
    let bad = &whole.bad[0];
    assert_eq!((bad.block, bad.first, bad.last), (4, 292, 364));
    assert!(bad.why.contains("block 4"), "{}", bad.why);
    assert_eq!(whole.unverifiable, None);

    let first = blocks(&file, 0, 4);
    assert_eq!((first.checked, first.next), (4, Some(4)));
    assert!(first.bad.is_empty());
    let rest = blocks(&file, 4, 100);
    assert_eq!((rest.checked, rest.next), (6, None));
    assert_eq!(rest.bad, whole.bad, "a resumed pass finds the same block");
    drop(file);
    let _ = fs::remove_dir_all(&root);
}

/// A clean month has no bad block; a month whose `.crc` is gone is
/// unverifiable, said once, not a bad block per block.
#[test]
fn a_clean_month_passes_and_a_lost_sidecar_is_unverifiable() {
    let (root, path) = month("crc", 150);
    let file = BarFile::open_existing(&root, path, 7).unwrap();
    let clean = blocks(&file, 0, u64::MAX);
    assert_eq!((clean.blocks, clean.checked, clean.bad.len()), (3, 3, 0));
    let bin = file.path().to_path_buf();
    drop(file);
    fs::remove_file(bin.with_extension("crc")).unwrap();
    let file = BarFile::open_existing(&root, path, 7).unwrap();
    let lost = blocks(&file, 0, u64::MAX);
    assert!(lost.bad.is_empty());
    assert!(
        lost.unverifiable
            .as_deref()
            .is_some_and(|why| why.contains("declares block checksums")),
        "{lost:?}"
    );
    assert_eq!(lost.checked, 0);
    drop(file);
    let _ = fs::remove_dir_all(&root);
}
