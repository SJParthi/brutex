//! The small-month rule's two in-crate proofs (D-4933): the bisection a month
//! under the threshold is answered by costs at most ten bar reads, and a
//! threshold write the disk refuses leaves no index, no bar and a month the
//! next commit indexes.
//!
//! Every bar is invented: one-second stamps in 2031-05 at a price no index
//! printed.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::cell::Cell;
use std::fs;
use std::path::PathBuf;

use brutex_core::vendor::Vendor;

use super::tests::arm_threshold_fault;
use super::{Appended, BarFile, StoreError, TimeLookup, first_at_or_after};
use crate::format::{Bar, OI_NULL};
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use crate::time_index::{INDEX_MIN_ROWS, Why};

const SYMBOL: u32 = 29;
const MICROS: i64 = 1_000_000;

fn bar(ts_micros: i64) -> Bar {
    Bar {
        ts_micros,
        open: 77_777,
        high: 77_787,
        low: 77_767,
        close: 77_780,
        volume: 3,
        open_interest: OI_NULL,
    }
}

#[test]
fn the_bisection_of_the_largest_small_month_reads_ten_bars() {
    // The largest month the rule leaves without an index holds 1,023 bars;
    // `ceil(log2(1_024)) = 10`. EVERY insertion point, and both sides of the
    // month, is asked, and the worst case is exactly the bound: the rule's
    // cost is a number, not a ceiling over one.
    let n = INDEX_MIN_ROWS - 1;
    let mut worst = 0u32;
    for ts in -1..=i64::try_from(n).unwrap() + 1 {
        let probes = Cell::new(0u32);
        let at = first_at_or_after(n, ts, |index: u64| -> Result<Bar, StoreError> {
            assert!(index < n, "probe {index} is past the {n} committed bars");
            probes.set(probes.get() + 1);
            Ok(bar(i64::try_from(index).unwrap()))
        })
        .unwrap();
        assert_eq!(
            at,
            u64::try_from(ts.clamp(0, i64::try_from(n).unwrap())).unwrap()
        );
        assert!(probes.get() <= 10, "ts={ts}: {} probes", probes.get());
        worst = worst.max(probes.get());
    }
    assert_eq!(
        worst, 10,
        "the worst case of a 1,023-bar month is ten reads"
    );
}

struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("brutex-small-month-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn file(&self, kind: FileKind) -> PathBuf {
        path().with_file(kind).to_path_buf(&self.0)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn path() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2031, 5).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// `count` one-second bars from 09:15:00 IST on Monday 5 May 2031, starting
/// `from` seconds in.
fn seconds(from: i64, count: u64) -> Vec<Bar> {
    let open =
        YearMonth::new(2031, 5).unwrap().ist_bounds_micros().0 + (4 * 86_400 + 555 * 60) * MICROS;
    (0..i64::try_from(count).unwrap())
        .map(|k| bar(open + (from + k) * MICROS))
        .collect()
}

#[test]
fn a_full_disk_at_the_threshold_write_leaves_no_index_and_no_bar() {
    let root = Root::new("full");
    let mut writer = BarFile::open_or_create(&root.0, path(), SYMBOL).unwrap();
    let below = INDEX_MIN_ROWS - 1;
    assert!(matches!(
        writer.append(&seconds(0, below)),
        Ok(Appended::Committed { .. })
    ));
    assert!(
        !root.file(FileKind::TimeIndex).exists(),
        "no index below it"
    );

    // THE WRITE THAT CROSSES IS REFUSED after the `.tix` is created and
    // before a byte of it is written: the append fails with the disk's own
    // refusal, the file this append made is taken back, and no bar moved.
    arm_threshold_fault();
    let crossing = seconds(i64::try_from(below).unwrap(), 2);
    assert!(matches!(
        writer.append(&crossing),
        Err(StoreError::DiskFull { .. })
    ));
    assert!(
        !root.file(FileKind::TimeIndex).exists(),
        "the refused threshold write leaves no index behind"
    );
    assert_eq!(writer.header().n_valid, below, "no bar was committed");
    assert!(!root.file(FileKind::SortIndex).exists());

    // THE SAME HANDLE TRIES AGAIN from nothing, and the commit that crosses
    // now writes both indexes whole.
    assert!(matches!(
        writer.append(&crossing),
        Ok(Appended::Committed { n_valid, .. }) if n_valid == INDEX_MIN_ROWS + 1
    ));
    drop(writer);
    let reader = BarFile::open_existing(&root.0, path(), SYMBOL).unwrap();
    assert_eq!(reader.time_lookup(), TimeLookup::Indexed);
    let all = seconds(0, INDEX_MIN_ROWS + 1);
    for (row, bar) in all.iter().enumerate() {
        assert_eq!(
            reader.first_at_or_after(bar.ts_micros),
            Ok(u64::try_from(row).unwrap())
        );
    }
    let srt = crate::sort_index::SortIndex::open(
        &root.file(FileKind::SortIndex),
        &crate::sort_index::Shape::of(YearMonth::new(2031, 5).unwrap(), 1, SYMBOL),
    )
    .unwrap();
    assert_eq!((srt.runs().len(), srt.coverage()), (1, INDEX_MIN_ROWS + 1));
}

#[test]
fn a_month_reopened_under_the_threshold_is_below_it_without_a_file() {
    // The WRITER's own answer: a handle that opened a small month with no
    // `.tix` is bisecting for the rule's reason, and no file was made by the
    // open.
    let root = Root::new("reopen");
    drop(BarFile::open_or_create(&root.0, path(), SYMBOL).unwrap());
    let mut writer = BarFile::open_or_create(&root.0, path(), SYMBOL).unwrap();
    assert_eq!(
        writer.time_lookup(),
        TimeLookup::Bisection(Why::BelowThreshold)
    );
    assert!(matches!(
        writer.append(&seconds(0, 5)),
        Ok(Appended::Committed { .. })
    ));
    assert_eq!(
        writer.time_lookup(),
        TimeLookup::Bisection(Why::BelowThreshold)
    );
    assert!(!root.file(FileKind::TimeIndex).exists());
}
