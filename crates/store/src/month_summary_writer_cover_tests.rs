#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: the writer's month-summary step for every kept file it
//! must not extend, and the existing-month door's quiet answers and its
//! refusals of a name it must not write through. Each case is checked
//! against a CONTROL month written the same way in a root of its own: a
//! summary built whole is byte for byte the control's. Every bar is invented.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use brutex_core::vendor::Vendor;

use super::*;
use crate::format::OI_NULL;
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

const SYMBOL: u32 = 0x1A57;
const MICROS: i64 = 1_000_000;

/// A store root of this test's own, removed when the test ends.
struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let at = std::env::temp_dir().join(format!(
            "brutex-msum-writer-cover-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _stale = fs::remove_dir_all(&at);
        fs::create_dir_all(&at).unwrap();
        Self(at)
    }

    fn sum(&self) -> PathBuf {
        path_beside(&month().to_path_buf(&self.0))
    }

    fn dir(&self) -> PathBuf {
        month().to_path_buf(&self.0).parent().unwrap().to_path_buf()
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _cleanup = fs::remove_dir_all(&self.0);
    }
}

fn month() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "QMSUMW",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2031, 5).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// `count` invented one-second bars from 09:15:00 IST on Monday 5 May 2031,
/// starting `from` seconds in.
fn bars(from: i64, count: i64) -> Vec<Bar> {
    let open =
        YearMonth::new(2031, 5).unwrap().ist_bounds_micros().0 + (4 * 86_400 + 555 * 60) * MICROS;
    (from..from + count)
        .map(|k| Bar {
            ts_micros: open + k * MICROS,
            open: 3_300_000 + k % 50,
            high: 3_300_100,
            low: 3_299_900,
            close: 3_300_000 + (k * 3) % 70,
            volume: 1 + k % 9,
            open_interest: OI_NULL,
        })
        .collect()
}

/// A month of `commits`, one append each, under `root`.
fn write(root: &Root, commits: &[Vec<Bar>]) {
    let mut writer = BarFile::open_or_create(&root.0, month(), SYMBOL).unwrap();
    for batch in commits {
        writer.append(batch).unwrap();
    }
}

/// A summary of `rows` as the kept file would hold it at `generation`.
fn summary_of(symbol: u32, rows: &[Bar], generation: u64) -> MonthSummary {
    let mut summary = MonthSummary::new(symbol, 1, YearMonth::new(2031, 5).unwrap());
    for row in rows {
        summary.fold(row);
    }
    summary.generation = generation;
    summary
}

/// Puts `summary` where the month keeps its summary, opens the writer (its
/// step runs at the open) and answers the kept bytes after it.
fn after_open_with(root: &Root, summary: &MonthSummary) -> Vec<u8> {
    fs::write(root.sum(), summary.file_image()).unwrap();
    drop(BarFile::open_or_create(&root.0, month(), SYMBOL).unwrap());
    fs::read(root.sum()).unwrap()
}

/// **A KEPT SUMMARY OF ANOTHER SYMBOL IS BUILT WHOLE, NEVER EXTENDED.** Its
/// rows are these very bars, so an extension would look plausible; it is
/// another symbol's, so the writer builds the month's own, byte for byte the
/// control's.
#[test]
fn a_kept_summary_of_another_symbol_is_built_whole_never_extended() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let commits = [bars(0, 1_100), bars(1_100, 100)];
    let control = Root::new("foreign-control");
    write(&control, &commits);
    let expected = fs::read(control.sum()).unwrap();
    let root = Root::new("foreign");
    write(&root, &commits);
    let foreign = summary_of(SYMBOL + 1, &bars(0, 1_100), 1);
    assert_ne!(foreign.file_image(), expected);
    assert_eq!(after_open_with(&root, &foreign), expected);
}

/// **A STALE SUMMARY THAT IS NO PREFIX OF THESE BARS IS BUILT WHOLE.** The
/// month is 1,100 bars then 100 more (generation 2). A kept summary is
/// extended only when it is an older generation of these very rows; each
/// way of failing that, one at a time, builds the summary whole: a newer
/// generation, no rows, more rows than the month, a fold count that is not
/// its rows, another first stamp, and a last stamp the month does not hold
/// at that row. Every outcome is the control's bytes. The one true prefix
/// is extended to the same bytes (the control of the control).
#[test]
fn a_stale_summary_that_is_no_prefix_of_these_bars_is_built_whole() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let commits = [bars(0, 1_100), bars(1_100, 100)];
    let control = Root::new("stale-control");
    write(&control, &commits);
    let expected = fs::read(control.sum()).unwrap();
    let root = Root::new("stale");
    write(&root, &commits);
    let head = bars(0, 1_100);

    let newer = summary_of(SYMBOL, &head, 3);
    let empty = summary_of(SYMBOL, &[], 1);
    let longer = summary_of(SYMBOL, &bars(0, 1_300), 1);
    let mut miscounted = summary_of(SYMBOL, &head, 1);
    miscounted.counts.folded -= 1;
    let shifted = summary_of(SYMBOL, &bars(1, 1_100), 1);
    let mut late_end = head.clone();
    late_end[1_099].ts_micros += 7 * MICROS;
    let unheld_end = summary_of(SYMBOL, &late_end, 1);
    let cases = [
        ("a newer generation", newer),
        ("no rows", empty),
        ("more rows than the month", longer),
        ("a fold count that is not its rows", miscounted),
        ("another first stamp", shifted),
        ("a last stamp the month does not hold there", unheld_end),
    ];
    for (why, summary) in &cases {
        assert_ne!(summary.file_image(), expected, "{why}: the case differs");
        assert_eq!(after_open_with(&root, summary), expected, "{why}");
    }
    // The true prefix: extended, and the extension is the same summary.
    let prefix = summary_of(SYMBOL, &head, 1);
    assert_eq!(after_open_with(&root, &prefix), expected, "the true prefix");
}

/// **THE EXISTING-MONTH DOOR ANSWERS AN EMPTY MONTH AND A SMALL ONE WITHOUT
/// WRITING.** A month that commits no rows is `Empty` and a month under the
/// small-month rule with no summary is `BelowThreshold`; neither leaves a
/// `.sum`. The same door builds a dense month's missing summary whole.
#[test]
fn the_existing_month_door_answers_an_empty_and_a_small_month_without_writing() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let empty = Root::new("door-empty");
    write(&empty, &[]);
    assert_eq!(
        BarFile::month_summary_existing(&empty.0, month(), SYMBOL),
        Ok(Summarized::Empty)
    );
    assert!(!empty.sum().exists());
    let small = Root::new("door-small");
    write(&small, &[bars(0, 10)]);
    assert_eq!(
        BarFile::month_summary_existing(&small.0, month(), SYMBOL),
        Ok(Summarized::BelowThreshold)
    );
    assert!(!small.sum().exists());
    let dense = Root::new("door-dense");
    write(&dense, &[bars(0, 1_100)]);
    fs::remove_file(dense.sum()).unwrap();
    assert!(matches!(
        BarFile::month_summary_existing(&dense.0, month(), SYMBOL),
        Ok(Summarized::Built(_))
    ));
    assert!(dense.sum().is_file());
}

/// **A NAME HELD BY A DIRECTORY IS LEFT AS IT IS AND THE WRITE REFUSED.**
/// At the summary's own name the step refuses before any staging file is
/// made; at the staging name the staging is refused. Either way the
/// directory is still there, and nothing was written through it.
#[test]
fn a_name_held_by_a_directory_is_left_and_the_summary_write_refused() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("held-name");
    write(&root, &[bars(0, 1_100)]);
    fs::remove_file(root.sum()).unwrap();
    fs::create_dir(root.sum()).unwrap();
    let at_name = BarFile::month_summary_existing(&root.0, month(), SYMBOL);
    assert_eq!(
        at_name,
        Err(StoreError::Io {
            path: root.sum(),
            action: Action::Write,
            kind: std::io::ErrorKind::AlreadyExists,
            code: None,
        })
    );
    assert!(root.sum().is_dir());
    let staging = root.dir().join("2031-05.new.sum");
    assert!(!staging.exists(), "no staging file was made");
    fs::remove_dir(root.sum()).unwrap();
    fs::create_dir(&staging).unwrap();
    let at_staging = BarFile::month_summary_existing(&root.0, month(), SYMBOL);
    assert_eq!(
        at_staging,
        Err(StoreError::Io {
            path: staging.clone(),
            action: Action::Open,
            kind: std::io::ErrorKind::AlreadyExists,
            code: None,
        })
    );
    assert!(staging.is_dir() && !root.sum().exists());
}

/// **A MONTH DIRECTORY THE HOST WILL NOT WRITE REFUSES THE STAGING BY ITS
/// KIND.** The host's own refusal of the staging create is named for the
/// staging file. Root writes into a mode-0555 directory, so the body runs
/// where the mode bits bind: as `nobody` when this process is root (D-0995).
#[test]
fn a_month_directory_the_host_will_not_write_refuses_the_staging_by_its_kind() {
    crate::support::where_permission_binds(
        "file::summary_writer::cover_tests::a_month_directory_the_host_will_not_write_refuses_the_staging_by_its_kind",
        a_month_directory_the_host_will_not_write_refuses_the_staging_by_its_kind_body,
    );
}

fn a_month_directory_the_host_will_not_write_refuses_the_staging_by_its_kind_body() {
    use std::os::unix::fs::PermissionsExt as _;
    let _sink_is_mine = crate::emits::hold_the_sink();
    let root = Root::new("closed-dir");
    write(&root, &[bars(0, 1_100)]);
    fs::remove_file(root.sum()).unwrap();
    fs::set_permissions(root.dir(), fs::Permissions::from_mode(0o555)).unwrap();
    let refused = BarFile::month_summary_existing(&root.0, month(), SYMBOL);
    fs::set_permissions(root.dir(), fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        refused,
        Err(StoreError::Denied {
            path: root.dir().join("2031-05.new.sum"),
            action: Action::Open,
        })
    );
    assert!(!root.sum().exists());
}
