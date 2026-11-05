#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: the write door's time-index steps and its overlap
//! diagnosis where no other test of this module reached them. A daily month
//! that reaches the small-month threshold with two bars in one IST day, and
//! a header that claims a later last stamp than its last record, through
//! the doors; the index steps' own refusals and their answers for a handle
//! that keeps no index, by direct call against each step's own contract
//! (the admission refuses those batches first on every door). Every stamp
//! and price is invented: 2031 months at a level no index printed.

use std::fs;
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use super::{
    Appended, BarFile, Conflict, Offered, Reindexed, SealedOffer, StoreError, TimeLookup, TixState,
    index_refused,
};
use crate::format::{Bar, OI_NULL};
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use crate::time_index::{Entry, INDEX_MIN_ROWS, Why};

const SYMBOL: u32 = 43;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// 09:15 IST on 2031-02-03, epoch microseconds.
const OPEN: i64 = 1_927_856_700_000_000;

/// 09:15 IST on 2031-03-01: the month after.
const MARCH_OPEN: i64 = 1_930_103_100_000_000;

const MINUTE: i64 = 60_000_000;

/// A store root of this test's own, cleared first and removed after.
struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-file-index-cover-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn at(&self, timeframe: Timeframe, file: FileKind) -> PathBuf {
        path(timeframe, file).to_path_buf(&self.0)
    }

    fn writer(&self, timeframe: Timeframe) -> BarFile {
        BarFile::open_or_create(&self.0, path(timeframe, FileKind::Bars), SYMBOL).unwrap()
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn feb() -> YearMonth {
    YearMonth::new(2031, 2).unwrap()
}

/// A Groww NIFTY month of 2031-02 at `timeframe`.
fn path(timeframe: Timeframe, file: FileKind) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe,
        month: feb(),
        file,
    })
    .unwrap()
}

/// A bar at `ts_micros`, at an invented level.
const fn bar(ts_micros: i64) -> Bar {
    Bar {
        ts_micros,
        open: 910_125,
        high: 910_750,
        low: 909_700,
        close: 910_275,
        volume: 3,
        open_interest: OI_NULL,
    }
}

/// `count` one-minute bars from minute `from` after [`OPEN`].
fn minutes(from: i64, count: i64) -> Vec<Bar> {
    (from..from + count)
        .map(|k| bar(OPEN + k * MINUTE))
        .collect()
}

/// Every file name under `dir`, recursively.
fn names_under(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let at = entry.unwrap().path();
        if at.is_dir() {
            out.extend(names_under(&at));
        } else {
            out.push(at.to_string_lossy().into_owned());
        }
    }
    out.sort();
    out
}

/// **A DAILY MONTH THAT REACHES THE SMALL-MONTH THRESHOLD WITH TWO BARS IN
/// ONE IST DAY COMMITS EVERY BAR AND KEEPS NO INDEX, SAYING WHY.** The
/// daily rung admits any whole second (D-0915), so 1,024 bars fit one day;
/// the commit that brings the month to [`INDEX_MIN_ROWS`] would write its
/// `.tix` whole (D-4933), and the second bar of the day shares the first
/// one's slot. The threshold step names that bar, writes no file, and the
/// commit lands; the next writer open meets the same bars and says the same.
#[test]
fn a_daily_month_reaching_the_threshold_with_two_bars_in_one_day_commits_unindexed() {
    let root = Root::new("daily-threshold");
    let tf = Timeframe::DAY_1;
    let tix = root.at(tf, FileKind::TimeIndex);
    let mut writer = root.writer(tf);
    assert!(matches!(
        writer.append(&[bar(OPEN)]),
        Ok(Appended::Committed { n_valid: 1, .. })
    ));
    assert_eq!(
        writer.time_lookup(),
        TimeLookup::Bisection(Why::BelowThreshold)
    );
    let rest: Vec<Bar> = (1..1_024).map(|k| bar(OPEN + k * 1_000_000)).collect();
    assert_eq!(u64::try_from(rest.len()).unwrap() + 1, INDEX_MIN_ROWS);
    let shared = Why::SharedSlot {
        index: 1,
        ts_micros: OPEN + 1_000_000,
    };
    assert!(
        matches!(
            writer.append(&rest),
            Ok(Appended::Committed { n_valid: 1_024, .. })
        ),
        "the commit lands"
    );
    assert!(!tix.exists(), "no index is left claiming the month");
    assert_eq!(writer.time_lookup(), TimeLookup::Bisection(shared.clone()));
    for (ts, row) in [
        (OPEN - 1, 0),
        (OPEN, 0),
        (OPEN + 1, 1),
        (OPEN + 1_023_000_000, 1_023),
    ] {
        assert_eq!(writer.first_at_or_after(ts), Ok(row), "ts {ts}");
    }
    drop(writer);
    let indexed = BarFile::index_existing(&root.0, path(tf, FileKind::Bars), SYMBOL).unwrap();
    assert_eq!(
        (indexed.n_valid, indexed.outcome),
        (1_024, Reindexed::Unindexable(shared))
    );
    assert!(!tix.exists());
}

/// **THE THRESHOLD STEP REFUSES A BATCH BAR OFF ITS RUNG'S GRID OR OUTSIDE
/// THE MONTH BY ITS PLACE IN THE BATCH, AND WRITES NOTHING** (its contract:
/// "refused as `index_batch` refuses it, before any byte is written"). The
/// admission refuses both first on every door, so the step is asked
/// directly, on a small month under the threshold.
#[test]
fn the_threshold_step_refuses_an_off_grid_or_outside_bar_by_its_batch_offset() {
    let root = Root::new("threshold-refusals");
    let tf = Timeframe::MINUTE_1;
    let mut writer = root.writer(tf);
    writer.append(&minutes(0, 10)).unwrap();
    let before = names_under(&root.0);
    let off_grid = OPEN + 11 * MINUTE + 30_000_000;
    assert_eq!(
        writer.index_at_threshold(&[bar(OPEN + 10 * MINUTE), bar(off_grid)]),
        Err(StoreError::OffGrid {
            at: 1,
            ts_micros: off_grid,
            timeframe_secs: 60,
        })
    );
    assert_eq!(
        writer.index_at_threshold(&[bar(MARCH_OPEN)]),
        Err(StoreError::OutsideMonth {
            at: 0,
            ts_micros: MARCH_OPEN,
            month: feb(),
        })
    );
    assert_eq!(names_under(&root.0), before, "nothing was written");
    assert_eq!(
        writer.time_lookup(),
        TimeLookup::Bisection(Why::BelowThreshold),
        "the month is as it was"
    );
}

/// **THE APPEND'S INDEX STEP REFUSES A BATCH BAR OFF ITS RUNG'S GRID OR
/// OUTSIDE THE MONTH BY ITS PLACE IN THE BATCH, AND WRITES NOTHING** (its
/// contract names both refusals "for a bar the admission already refuses,
/// should one ever reach here"), asked directly of an indexed month; the
/// index still serves, and the next admitted append keeps it in step.
#[test]
fn the_index_step_refuses_an_off_grid_or_outside_bar_by_its_batch_offset() {
    let root = Root::new("index-refusals");
    let tf = Timeframe::MINUTE_1;
    let tix = root.at(tf, FileKind::TimeIndex);
    let mut writer = root.writer(tf);
    writer.append(&minutes(0, 1_100)).unwrap();
    assert_eq!(writer.time_lookup(), TimeLookup::Indexed);
    let index = fs::read(&tix).unwrap();
    let off_grid = OPEN + 1_101 * MINUTE + 1_000_000;
    assert_eq!(
        writer.index_batch(&[bar(OPEN + 1_100 * MINUTE), bar(off_grid)]),
        Err(StoreError::OffGrid {
            at: 1,
            ts_micros: off_grid,
            timeframe_secs: 60,
        })
    );
    assert_eq!(
        writer.index_batch(&[bar(MARCH_OPEN)]),
        Err(StoreError::OutsideMonth {
            at: 0,
            ts_micros: MARCH_OPEN,
            month: feb(),
        })
    );
    assert_eq!(fs::read(&tix).unwrap(), index, "the index is untouched");
    assert_eq!(writer.time_lookup(), TimeLookup::Indexed);
    assert!(matches!(
        writer.append(&minutes(1_100, 5)),
        Ok(Appended::Committed { n_valid: 1_105, .. })
    ));
    drop(writer);
    let indexed = BarFile::index_existing(&root.0, path(tf, FileKind::Bars), SYMBOL).unwrap();
    assert_eq!((indexed.n_valid, indexed.outcome), (1_105, Reindexed::Kept));
}

/// **AN INDEX FAILURE AN APPEND CANNOT ROUTE AROUND IS REFUSED NAMING THE
/// `.tix` AND THE INDEX'S OWN SENTENCE**; a refused read inside it is the
/// host's refusal itself, unwrapped.
#[test]
fn an_index_failure_is_refused_naming_the_index_and_its_reason() {
    let at = Path::new("/brutex-cover/bars/groww/NSE/INDEX/NIFTY/1m/2031-02.tix");
    let reasons = [
        Why::Absent,
        Why::Header("the geometry is another month's"),
        Why::Entry { bucket: 9 },
        Why::Stale,
        Why::Outside {
            index: 4,
            ts_micros: MARCH_OPEN,
        },
        Why::OffGrid {
            index: 5,
            ts_micros: OPEN + 1,
        },
        Why::SharedSlot {
            index: 6,
            ts_micros: OPEN,
        },
        Why::NotBars,
        Why::BelowThreshold,
    ];
    for why in reasons {
        let refusal = index_refused(at, &why);
        assert_eq!(
            refusal,
            StoreError::TimeIndex {
                path: at.to_path_buf(),
                reason: why.to_string(),
            },
            "{why:?}"
        );
        assert!(refusal.to_string().contains(&why.to_string()), "{refusal}");
    }
    let host = StoreError::Denied {
        path: at.to_path_buf(),
        action: super::Action::Read,
    };
    assert_eq!(index_refused(at, &Why::Unreadable(host.clone())), host);
}

/// **A HANDLE THAT KEEPS NO INDEX HAS NOTHING TO REPLACE, CROSS, RETIRE,
/// REBUILD OR WRITE**: an overlay's writer (only bar files carry a `.tix`)
/// asked each index step directly answers done, writes no file anywhere,
/// and still bisects for the reason it always gave.
#[test]
fn a_handle_that_keeps_no_index_has_no_index_step_to_take() {
    let root = Root::new("no-index");
    let overlay = path(Timeframe::MINUTE_1, FileKind::Overlay);
    let mut file = BarFile::open_or_create(&root.0, overlay, SYMBOL).unwrap();
    assert!(file.tix.is_none());
    let before = names_under(&root.0);
    file.set_tix_state(TixState::Bisecting(Why::Stale));
    assert!(file.tix.is_none(), "nothing to replace");
    assert_eq!(file.index_at_threshold(&minutes(0, 3)), Ok(None));
    assert_eq!(file.retire_index(Why::Stale), Ok(()));
    assert_eq!(file.reindex(&Why::Stale), Ok(()));
    assert_eq!(file.write_entries(0, &[Entry::EMPTY]), Ok(()));
    assert_eq!(names_under(&root.0), before, "nothing was written");
    assert_eq!(file.time_lookup(), TimeLookup::Bisection(Why::NotBars));
}

/// **A BAR MONTH WHOSE INDEX IS NOT READY TAKES NO ENTRIES**: a small month
/// under the threshold has no `.tix` (D-4933), and the entry step asked
/// directly writes none and creates none.
#[test]
fn a_month_with_no_ready_index_takes_no_entries() {
    let root = Root::new("not-ready");
    let tf = Timeframe::MINUTE_1;
    let mut writer = root.writer(tf);
    writer.append(&minutes(0, 4)).unwrap();
    let before = names_under(&root.0);
    assert_eq!(writer.write_entries(0, &[Entry::EMPTY]), Ok(()));
    assert_eq!(names_under(&root.0), before);
    assert!(!root.at(tf, FileKind::TimeIndex).exists());
    assert_eq!(
        writer.time_lookup(),
        TimeLookup::Bisection(Why::BelowThreshold)
    );
}

/// **A MONTH WITH NOTHING COMMITTED HAS NO OVERLAP TO VERIFY**: the suffix
/// step answers `None` before it reads anything (both doors ask it only
/// after `advance` refused a batch that does not follow, which an empty
/// month never does).
#[test]
fn an_empty_month_has_no_verified_suffix() {
    let root = Root::new("empty-suffix");
    let writer = root.writer(Timeframe::MINUTE_1);
    assert_eq!(writer.header().n_valid, 0);
    let batch = minutes(0, 3);
    assert_eq!(writer.suffix_that_follows(&batch), Ok(None));
}

/// **A HEADER THAT CLAIMS A LATER LAST STAMP THAN ITS LAST RECORD: A BAR
/// BETWEEN THE TWO IS NAMED AS NOT HELD, BY BOTH DOORS, AND NOTHING IS
/// WRITTEN.** The slot is re-committed one generation on with only its last
/// stamp moved five minutes past the last record (every record and checksum
/// as written), so the month opens; a bar three minutes past the last
/// record does not follow the header, is held nowhere, and is located past
/// the last record, which the diagnosis names `NotHeld` at its place.
#[test]
fn a_bar_past_the_last_record_but_inside_the_claimed_range_is_not_held() {
    let root = Root::new("claimed-past");
    let tf = Timeframe::MINUTE_1;
    let bars_path = root.at(tf, FileKind::Bars);
    let mut writer = root.writer(tf);
    writer.append(&minutes(0, 10)).unwrap();
    let held = writer.header();
    drop(writer);
    let claimed = super::Header {
        generation: held.generation + 1,
        last_ts_micros: held.last_ts_micros + 5 * MINUTE,
        ..held
    };
    let commit = claimed.commit().unwrap();
    let file = fs::OpenOptions::new().write(true).open(&bars_path).unwrap();
    file.write_all_at(&commit.bytes, commit.offset).unwrap();
    drop(file);
    let between = held.last_ts_micros + 3 * MINUTE;
    let mut writer = root.writer(tf);
    assert_eq!(writer.header().last_ts_micros, claimed.last_ts_micros);
    let before = fs::read(&bars_path).unwrap();
    assert_eq!(
        writer.append(&[bar(between)]),
        Err(StoreError::OverlapDisagrees {
            path: bars_path.clone(),
            at: 0,
            ts_micros: between,
            conflict: Conflict::NotHeld,
        })
    );
    assert_eq!(
        writer.offer(&[bar(between)]),
        Ok(Offered::Unheld {
            at: 0,
            ts_micros: between,
            offer: SealedOffer::Disagrees(Conflict::NotHeld),
        })
    );
    assert_eq!(fs::read(&bars_path).unwrap(), before, "nothing was written");
    assert_eq!(writer.header().n_valid, 10);
}
