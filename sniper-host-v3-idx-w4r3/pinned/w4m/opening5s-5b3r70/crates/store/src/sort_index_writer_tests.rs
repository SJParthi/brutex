//! The writer's sort-index step when the disk will not take a run: a sync
//! that fails after the write (a full or failing disk, D-3349, WP-39).
//!
//! The bars are committed before the step runs, so nothing here may refuse
//! them, lose them, or leave bytes a reader could take for a run. Every bar
//! is invented: one-second stamps in 2031-05 at prices no index printed, and
//! every month is past the small-month threshold (D-4933), so it has an index
//! for the disk to refuse.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::path::PathBuf;

use brutex_core::vendor::Vendor;

use super::super::tests::{arm_sync_fault, sync_fault_fires};
use super::super::{Appended, BarFile};
use crate::format::{Bar, OI_NULL};
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use crate::sort_index::{RunKind, Shape, SortIndex, Why};

const SYMBOL: u32 = 23;

struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("brutex-srt-writer-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn file(&self, kind: FileKind) -> PathBuf {
        path(kind).to_path_buf(&self.0)
    }

    fn staging(&self) -> PathBuf {
        self.file(FileKind::SortIndex)
            .with_file_name("2031-05.new.srt")
    }

    fn index(&self) -> Result<SortIndex, Why> {
        SortIndex::open(
            &self.file(FileKind::SortIndex),
            &Shape::of(month(), 1, SYMBOL),
        )
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn month() -> YearMonth {
    YearMonth::new(2031, 5).unwrap()
}

fn path(file: FileKind) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: month(),
        file,
    })
    .unwrap()
}

fn bars(from: i64, to: i64) -> Vec<Bar> {
    let open = month().ist_bounds_micros().0 + (86_400 + 555 * 60) * 1_000_000;
    (from..to)
        .map(|i| {
            let close = 6_660_000 + (i * 7_919) % 401;
            Bar {
                ts_micros: open + i * 1_000_000,
                open: close - 3,
                high: close + 9,
                low: close - 9,
                close,
                volume: i % 11,
                open_interest: OI_NULL,
            }
        })
        .collect()
}

/// **A RUN WHOSE SYNC FAILS IS CUT OFF AGAIN, AND THE BARS STAY COMMITTED.**
/// The run's bytes were written and the disk refused the sync, so they may
/// not be whole: the file is cut back to the end of the last run that was
/// synced, no later run is appended after them, and the next step appends
/// the run again. Before the cut, a walk would have met a run header over a
/// body no sync ever confirmed.
#[test]
fn an_appended_run_whose_sync_fails_is_cut_and_written_again() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("append-sync");
    let mut writer = BarFile::open_or_create(&root.0, path(FileKind::Bars), SYMBOL).unwrap();
    assert!(matches!(
        writer.append(&bars(0, 1_100)),
        Ok(Appended::Committed { .. })
    ));
    assert!(matches!(
        writer.append(&bars(1_100, 1_400)),
        Ok(Appended::Committed { .. })
    ));
    let whole = fs::read(root.file(FileKind::SortIndex)).unwrap();
    let first_run_end = {
        let index = root.index().unwrap();
        assert_eq!(index.runs().len(), 2);
        index.runs()[1].offset
    };
    // THE SECOND RUN NEVER LANDED: the state a crash between the commit and
    // the run leaves.
    fs::OpenOptions::new()
        .write(true)
        .open(root.file(FileKind::SortIndex))
        .unwrap()
        .set_len(first_run_end)
        .unwrap();
    let bars_before = fs::read(root.file(FileKind::Bars)).unwrap();

    arm_sync_fault(0);
    writer.keep_sort_index();
    assert!(!sync_fault_fires(), "the run's own sync met the fault");
    assert_eq!(
        fs::metadata(root.file(FileKind::SortIndex)).unwrap().len(),
        first_run_end,
        "the unsynced run is cut off, not left for a reader"
    );
    let index = root.index().unwrap();
    assert_eq!(index.coverage(), 1_100, "rows 1100..1400 are not claimed");
    assert!(index.torn().is_none(), "and no torn bytes are left");
    drop(index);
    assert_eq!(writer.header().n_valid, 1_400, "the bars stay committed");
    assert_eq!(fs::read(root.file(FileKind::Bars)).unwrap(), bars_before);

    // THE NEXT STEP WRITES THE RUN AGAIN, byte for byte what it was.
    writer.keep_sort_index();
    assert_eq!(fs::read(root.file(FileKind::SortIndex)).unwrap(), whole);
    assert_eq!(root.index().unwrap().coverage(), 1_400);
}

/// **A WHOLE IMAGE WHOSE SYNC FAILS IS NEVER RENAMED INTO PLACE.** The month
/// has no index; the image is written into `<yyyy-mm>.new.srt` and the disk
/// refuses the sync. No `.srt` appears — a reader is told the month has none
/// and reads it — and the next step removes the staging name, writes the image into a new file
/// again and renames it.
#[test]
fn a_whole_image_whose_sync_fails_is_never_renamed_into_place() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("whole-sync");
    let mut writer = BarFile::open_or_create(&root.0, path(FileKind::Bars), SYMBOL).unwrap();
    assert!(matches!(
        writer.append(&bars(0, 1_100)),
        Ok(Appended::Committed { .. })
    ));
    let whole = fs::read(root.file(FileKind::SortIndex)).unwrap();
    fs::remove_file(root.file(FileKind::SortIndex)).unwrap();

    arm_sync_fault(0);
    writer.keep_sort_index();
    assert!(!sync_fault_fires(), "the image's own sync met the fault");
    assert!(matches!(root.index(), Err(Why::Absent)), "nothing renamed");
    assert!(root.staging().exists(), "the unsynced image stays staged");

    // A STAGING FILE LONGER THAN THE IMAGE, as a writer killed mid-write of
    // a larger month would leave: its name is removed; it is never trusted, cut or appended to.
    let mut junk = fs::read(root.staging()).unwrap();
    junk.extend_from_slice(&[0xA5; 4_096]);
    fs::write(root.staging(), &junk).unwrap();
    writer.keep_sort_index();
    assert_eq!(fs::read(root.file(FileKind::SortIndex)).unwrap(), whole);
    assert!(!root.staging().exists(), "renamed, not left");
    let index = root.index().unwrap();
    assert_eq!(index.runs().len(), 1);
    assert_eq!(index.runs()[0].kind, RunKind::Whole);
    assert_eq!(index.coverage(), 1_100);
}
