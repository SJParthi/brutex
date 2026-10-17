//! `cli msum-build` on invented stores: misuse, a month written before
//! D-4980 summarised and a rerun that writes nothing, `--dry-run`, the lock,
//! a month not held, the small-month rule, and a `.sum` name that is not a
//! regular file. R3-06, D-4980.
//!
//! Every bar is invented (2033, prices no index printed). Nothing is removed:
//! a summary a test needs gone is renamed aside, and every root is new.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::month_summary::{MonthSummary, path_beside, read_beside};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{USAGE, command_with};
use crate::{FAILED, MISUSED, OK};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Root(PathBuf);

fn symbol_id() -> u32 {
    u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)).unwrap()
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).unwrap()
}

fn path(month: YearMonth) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    })
    .unwrap()
}

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-msum-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn run(&self, arguments: &[&str]) -> (u8, String) {
        let mut out = String::new();
        let root = self.0.clone();
        let code = command_with(arguments, &move || Ok(root.clone()), &mut out);
        (code, out)
    }

    /// The month's `.sum`, beside the bar file a reader opens.
    fn sum(&self, month: YearMonth) -> PathBuf {
        let file = BarFile::open_existing(&self.0, path(month), symbol_id()).unwrap();
        path_beside(file.path())
    }

    /// An invented month of `count` one-second bars from 09:15 on its second
    /// day. When `before` it is a month written before D-4980: its writer's
    /// `.sum` is renamed aside, so none is kept beside it.
    fn month(&self, month: YearMonth, count: i64, before: bool) {
        let mut writer = BarFile::open_or_create(&self.0, path(month), symbol_id()).unwrap();
        let open = month.ist_bounds_micros().0 + (86_400 + 555 * 60) * 1_000_000;
        let rows: Vec<Bar> = (0..count)
            .map(|k| Bar {
                ts_micros: open + k * 1_000_000,
                open: 4_321_000 + k % 19,
                high: 4_321_050 + k % 19,
                low: 4_320_950 + k % 19,
                close: 4_321_010 + (k * 7) % 13,
                volume: k % 5,
                open_interest: OI_NULL,
            })
            .collect();
        writer.append(&rows).unwrap();
        drop(writer);
        let sum = self.sum(month);
        if before && sum.exists() {
            fs::rename(&sum, sum.with_extension("aside")).unwrap();
        }
    }

    /// The summary a reader finds, which must be what a full read builds.
    fn found_and_true(&self, month: YearMonth) {
        let file = BarFile::open_existing(&self.0, path(month), symbol_id()).unwrap();
        let header = file.header();
        let rows: Vec<Bar> = (0..header.n_valid)
            .map(|row| file.read_record(row).unwrap())
            .collect();
        assert_eq!(
            read_beside(file.path(), &header, month).unwrap(),
            MonthSummary::build(&header, month, &rows)
        );
    }
}

#[test]
fn misuse_is_refused_before_anything_is_opened() {
    let root = Root::new();
    for arguments in [
        &["gdfl", "NSE/INDEX/NIFTY", "1s", "2033-01"][..],
        &["nofeed", "NSE/INDEX/NIFTY", "1s", "2033-01", "2033-01"],
        &["gdfl", "NSE/INDEX", "1s", "2033-01", "2033-01"],
        &["gdfl", "NSE/INDEX/NIFTY", "1s", "2033-1", "2033-01"],
    ] {
        let (code, out) = root.run(arguments);
        assert_eq!(code, MISUSED, "{arguments:?}: {out}");
        assert!(out.contains(USAGE), "{out}");
    }
    let (_, out) = root.run(&["gdfl"]);
    assert!(out.contains("msum-build takes five arguments"), "{out}");
    assert_eq!(fs::read_dir(&root.0).unwrap().count(), 0, "nothing created");
}

/// A month written before D-4980 is summarised whole, a reader then finds
/// exactly what a full read builds, and a rerun writes nothing.
#[test]
fn a_month_without_a_summary_is_built_and_a_rerun_writes_nothing() {
    let root = Root::new();
    root.month(ym(2033, 3), 1_500, true);
    root.month(ym(2033, 4), 1_500, false);
    assert!(!root.sum(ym(2033, 3)).exists());
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2033-03", "2033-04"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("2033-03  built") && out.contains("because absent"),
        "{out}"
    );
    assert!(out.contains("2033-04  kept"), "{out}");
    assert!(out.contains("Built 1, kept 1,"), "{out}");
    root.found_and_true(ym(2033, 3));
    root.found_and_true(ym(2033, 4));
    let image = fs::read(root.sum(ym(2033, 3))).unwrap();
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2033-03", "2033-04"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("Built 0, kept 2,"), "{out}");
    assert_eq!(fs::read(root.sum(ym(2033, 3))).unwrap(), image);
}

/// `--dry-run` says what the run would do and writes nothing; a month not
/// held fails the run and nothing is created for it; a small month is small.
#[test]
fn a_dry_run_writes_nothing_and_every_line_is_named() {
    let root = Root::new();
    root.month(ym(2033, 5), 1_500, true);
    root.month(ym(2033, 6), 400, true);
    let (code, out) = root.run(&[
        "gdfl",
        "NSE/INDEX/NIFTY",
        "1s",
        "2033-05",
        "2033-07",
        "--dry-run",
    ]);
    assert_eq!(code, FAILED, "2033-07 is not held: {out}");
    assert!(
        out.contains("2033-05  would build") && out.contains("absent"),
        "{out}"
    );
    assert!(out.contains("2033-06  small"), "{out}");
    assert!(out.contains("2033-07  NOT HELD"), "{out}");
    assert!(!root.sum(ym(2033, 5)).exists(), "the dry run wrote nothing");
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2033-05", "2033-06"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("2033-06  small"), "{out}");
    assert!(
        !root.sum(ym(2033, 6)).exists(),
        "a small month is given none"
    );
    let (code, out) = root.run(&[
        "gdfl",
        "NSE/INDEX/NIFTY",
        "1s",
        "2033-05",
        "2033-05",
        "--dry-run",
    ]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("2033-05  summarized"), "{out}");
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2033-07", "2033-07"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("NOT HELD"), "{out}");
    assert!(BarFile::open_existing(&root.0, path(ym(2033, 7)), symbol_id()).is_err());
}

/// A month a writer holds is refused by the store's lock and left as it was;
/// a `.sum` name that is a directory is refused by the dry run and the run
/// alike, and written through by neither.
#[test]
fn a_held_month_and_a_name_that_is_not_a_file_are_refused_and_left_alone() {
    let root = Root::new();
    root.month(ym(2033, 8), 1_500, true);
    // THE NAME FIRST: a reader may not open a month a writer holds.
    let held = root.sum(ym(2033, 8));
    let writer = BarFile::open_or_create(&root.0, path(ym(2033, 8)), symbol_id()).unwrap();
    let kept = fs::read(&held).unwrap();
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2033-08", "2033-08"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("REFUSED") && out.contains("held by another process"),
        "{out}"
    );
    assert_eq!(fs::read(&held).unwrap(), kept);
    drop(writer);
    root.month(ym(2033, 9), 1_500, true);
    let sum = root.sum(ym(2033, 9));
    fs::create_dir(&sum).unwrap();
    for dry in [true, false] {
        let mut arguments = vec!["gdfl", "NSE/INDEX/NIFTY", "1s", "2033-09", "2033-09"];
        if dry {
            arguments.push("--dry-run");
        }
        let (code, out) = root.run(&arguments);
        assert_eq!(code, FAILED, "dry {dry}: {out}");
        assert!(out.contains("2033-09  REFUSED"), "dry {dry}: {out}");
        assert!(fs::metadata(&sum).unwrap().is_dir(), "left as it was");
        assert!(
            !sum.with_extension("new.sum").exists(),
            "dry {dry}: refused before a staging file was made"
        );
    }
}
