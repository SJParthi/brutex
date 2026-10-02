//! The write boundary refuses a bar the file's own PATH says cannot be in it.
//!
//! A bar file is addressed by `…/<timeframe>/<yyyy-mm>.bin`, and until D-0915
//! both segments were metadata only: a `2024-06` one-minute file accepted a
//! 2030 bar, an `i64::MIN` bar, and a 09:15:30 bar. Each of those then sat at
//! the end of an append-only, strictly increasing file, so the FIRST such bar
//! permanently blocked every later legitimate append to the month
//! (findings ET-bars-candles-store-2 and ET-bars-candles-store-3).
//!
//! The month is the IST calendar month, because that is how
//! `pull::ingest::months_in` assigns a bar to a file: `IstMoment` of the bar's
//! whole seconds, `.day().year_month()`. A UTC month would put the first five
//! and a half hours of every IST month in the previous file.
//!
//! Session membership (09:15 to the close, trading days only) is NOT checked
//! here: the store holds no exchange calendar and may not depend on `pull`,
//! which does. The last test pins that limit so it is a stated fact rather
//! than an omission. `docs/06-limits.md`, D-0915.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use store::file::{Appended, BarFile, StoreError};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut root = std::env::temp_dir();
        root.push(format!(
            "brutex-admission-{}-{tag}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("scratch root");
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.root));
    }
}

const SECOND: i64 = 1_000_000;
const MINUTE: i64 = 60 * SECOND;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
/// 2024-06-01 00:00:00 IST, the first instant of the IST month `2024-06`.
/// `1_717_180_200` seconds: 19,875 days from 1970-01-01 at UTC midnight, less
/// the 19,800-second IST offset.
const JUNE_START: i64 = 1_717_180_200 * SECOND;
/// 2024-07-01 00:00:00 IST, the first instant that is NOT `2024-06`.
const JULY_START: i64 = JUNE_START + 30 * DAY;
/// 2024-06-03 09:15 IST, the constant `tests/write.rs` uses.
const T0: i64 = JUNE_START + 2 * DAY + 9 * HOUR + 15 * MINUTE;
const SYMBOL: u32 = 26_000;

fn bar_at(ts_micros: i64) -> Bar {
    Bar {
        ts_micros,
        open: 2_345_600,
        high: 2_345_900,
        low: 2_345_100,
        close: 2_345_700,
        volume: 1_000,
        open_interest: OI_NULL,
    }
}

fn june() -> YearMonth {
    YearMonth::new(2024, 6).expect("2024-06")
}

fn path(timeframe: Timeframe) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe,
        month: june(),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

fn open(root: &Path, timeframe: Timeframe) -> BarFile {
    BarFile::open_or_create(root, path(timeframe), SYMBOL).expect("the month opens")
}

fn image(root: &Path, timeframe: Timeframe) -> Vec<u8> {
    fs::read(path(timeframe).to_path_buf(root)).expect("the bar file")
}

fn outside(at: u64, ts_micros: i64) -> StoreError {
    StoreError::OutsideMonth {
        at,
        ts_micros,
        month: june(),
    }
}

fn off_grid(at: u64, ts_micros: i64, timeframe_secs: u32) -> StoreError {
    StoreError::OffGrid {
        at,
        ts_micros,
        timeframe_secs,
    }
}

#[test]
fn the_ist_month_bounds_are_the_ones_pull_assigns() {
    assert_eq!(june().ist_bounds_micros(), (JUNE_START, JULY_START));
    // THE EPOCH MONTH STARTS BEFORE THE EPOCH: 1970-01-01 00:00 IST is
    // 1969-12-31 18:30 UTC, `-19_800` seconds.
    let epoch = YearMonth::new(1970, 1).unwrap().ist_bounds_micros();
    assert_eq!(epoch, (-19_800 * SECOND, (31 * 86_400 - 19_800) * SECOND));
    // December rolls into the NEXT year's January, with no gap and no overlap.
    let december = YearMonth::new(2024, 12).unwrap().ist_bounds_micros();
    let january = YearMonth::new(2025, 1).unwrap().ist_bounds_micros();
    assert_eq!(december.1, january.0);
    assert_eq!(december.1 - december.0, 31 * DAY);
    // Leap years, including both century rules.
    let days = |year: u16, month: u8| {
        let (from, until) = YearMonth::new(year, month).unwrap().ist_bounds_micros();
        (until - from) / DAY
    };
    assert_eq!(days(2024, 2), 29);
    assert_eq!(days(2023, 2), 28);
    assert_eq!(days(2000, 2), 29);
    assert_eq!(days(2100, 2), 28);
    assert_eq!(days(2024, 4), 30);
    // THE LAST REPRESENTABLE MONTH: 10000-01-01 00:00 IST, no overflow.
    let last = YearMonth::new(9999, 12).unwrap().ist_bounds_micros();
    assert_eq!(last.1, (2_932_897 * 86_400 - 19_800) * SECOND);
    assert_eq!(last.1 - last.0, 31 * DAY);
}

#[test]
fn the_first_and_last_instant_of_the_ist_month_are_admitted() {
    let scratch = Scratch::new("edges");
    let mut file = open(scratch.root(), Timeframe::MINUTE_1);
    assert_eq!(
        file.append(&[bar_at(JUNE_START), bar_at(JULY_START - MINUTE)]),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 2
        })
    );
}

#[test]
fn a_bar_one_step_outside_the_ist_month_is_refused_on_either_side() {
    let scratch = Scratch::new("outside");
    let mut file = open(scratch.root(), Timeframe::MINUTE_1);
    let before = image(scratch.root(), Timeframe::MINUTE_1);
    // 2024-05-31 23:59 IST: in May under IST, in June under UTC.
    assert_eq!(
        file.append(&[bar_at(JUNE_START - MINUTE)]),
        Err(outside(0, JUNE_START - MINUTE))
    );
    // 2024-07-01 00:00 IST: in July under IST, still June under UTC.
    assert_eq!(
        file.append(&[bar_at(T0), bar_at(JULY_START)]),
        Err(outside(1, JULY_START))
    );
    // The finding's own case: a 2030 bar in a 2024-06 file.
    let year_2030 = 1_893_456_000 * SECOND;
    assert_eq!(
        file.append(&[bar_at(year_2030)]),
        Err(outside(0, year_2030))
    );
    assert_eq!(image(scratch.root(), Timeframe::MINUTE_1), before);
    assert_eq!(file.records(), 0);
}

#[test]
fn sentinel_timestamps_are_refused_and_do_not_block_the_month() {
    let scratch = Scratch::new("sentinel");
    let mut file = open(scratch.root(), Timeframe::MINUTE_1);
    assert_eq!(file.append(&[bar_at(i64::MIN)]), Err(outside(0, i64::MIN)));
    assert_eq!(
        file.append(&[bar_at(T0), bar_at(i64::MAX)]),
        Err(outside(1, i64::MAX))
    );
    assert_eq!(file.append(&[bar_at(0)]), Err(outside(0, 0)));
    // Nothing was committed, so the month still takes its real bars. Before
    // D-0915 the `i64::MAX` bar would have committed and refused this forever.
    assert_eq!(
        file.append(&[bar_at(T0), bar_at(T0 + MINUTE)]),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 2
        })
    );
}

#[test]
fn a_one_minute_file_refuses_a_stamp_off_its_grid() {
    let scratch = Scratch::new("grid1");
    let mut file = open(scratch.root(), Timeframe::MINUTE_1);
    let before = image(scratch.root(), Timeframe::MINUTE_1);
    assert_eq!(
        file.append(&[bar_at(T0 + 30 * SECOND)]),
        Err(off_grid(0, T0 + 30 * SECOND, 60))
    );
    assert_eq!(
        file.append(&[bar_at(T0), bar_at(T0 + MINUTE), bar_at(T0 + 2 * MINUTE + 1)]),
        Err(off_grid(2, T0 + 2 * MINUTE + 1, 60))
    );
    assert_eq!(file.append(&[bar_at(T0 - 1)]), Err(off_grid(0, T0 - 1, 60)));
    assert_eq!(image(scratch.root(), Timeframe::MINUTE_1), before);
}

#[test]
fn an_open_anchored_rung_admits_its_own_edges_and_nothing_between() {
    // 30 minutes does not divide the 555 minutes from IST midnight to the open,
    // so `pull::fold` anchors it at 09:15: 09:15, 09:45, … A midnight- or
    // epoch-anchored grid would put its edges at :00 and :30 instead.
    let scratch = Scratch::new("grid30");
    let mut file = open(scratch.root(), Timeframe::MINUTE_30);
    assert_eq!(
        file.append(&[bar_at(T0 + 15 * MINUTE)]),
        Err(off_grid(0, T0 + 15 * MINUTE, 1_800))
    );
    assert_eq!(
        file.append(&[bar_at(T0), bar_at(T0 + 30 * MINUTE)]),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 2
        })
    );
    let scratch = Scratch::new("grid60");
    let mut hour = open(scratch.root(), Timeframe::MINUTE_60);
    assert_eq!(
        hour.append(&[bar_at(T0 + 45 * MINUTE)]),
        Err(off_grid(0, T0 + 45 * MINUTE, 3_600))
    );
    assert_eq!(
        hour.append(&[bar_at(T0), bar_at(T0 + HOUR)]),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 2
        })
    );
}

#[test]
fn a_one_second_file_admits_every_whole_second_and_no_fraction() {
    let scratch = Scratch::new("grid1s");
    let mut file = open(scratch.root(), Timeframe::SECOND_1);
    assert_eq!(
        file.append(&[bar_at(T0 + SECOND / 2)]),
        Err(off_grid(0, T0 + SECOND / 2, 1))
    );
    assert_eq!(
        file.append(&[bar_at(T0 + 7 * SECOND)]),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 1
        })
    );
}

#[test]
fn a_daily_file_admits_every_vendor_stamp_convention_but_no_fraction() {
    // `pull::session::Window::verdict`: "vendors stamp it at midnight, at the
    // open or at the close". The store refuses none of the three, and refuses
    // a sub-second stamp, which no vendor sends.
    let scratch = Scratch::new("grid1day");
    let mut file = open(scratch.root(), Timeframe::DAY_1);
    assert_eq!(
        file.append(&[bar_at(JUNE_START + 3 * DAY + 1)]),
        Err(off_grid(0, JUNE_START + 3 * DAY + 1, 86_400))
    );
    let midnight = JUNE_START + 3 * DAY;
    let open = JUNE_START + 4 * DAY + 9 * HOUR + 15 * MINUTE;
    let close = JUNE_START + 5 * DAY + 15 * HOUR + 30 * MINUTE;
    assert_eq!(
        file.append(&[bar_at(midnight), bar_at(open), bar_at(close)]),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 3
        })
    );
}

#[test]
fn session_membership_is_not_the_stores_check_and_a_three_am_bar_is_admitted() {
    // THE STATED LIMIT, PINNED. 03:00 IST is outside every NSE session, and it
    // is on the minute grid and inside the month, so the store admits it.
    // Session filtering belongs to `pull` (`Window::verdict`), which holds the
    // venue hours; the store holds no calendar and may not depend on `pull`.
    // If the store ever gains that check, this test is the one to turn round.
    let scratch = Scratch::new("session");
    let mut file = open(scratch.root(), Timeframe::MINUTE_1);
    let three_am = JUNE_START + 3 * DAY + 3 * HOUR;
    assert_eq!(
        file.append(&[bar_at(three_am)]),
        Ok(Appended::Committed {
            first_index: 0,
            n_valid: 1
        })
    );
}

#[test]
fn the_refusals_name_the_bar_the_stamp_and_the_rule() {
    assert_eq!(
        outside(3, JULY_START).to_string(),
        format!(
            "batch record 3 is stamped {JULY_START}, outside the IST month 2024-06 \
             its file is named for"
        )
    );
    assert_eq!(
        off_grid(2, T0 + 1, 60).to_string(),
        format!(
            "batch record 2 is stamped {}, off the 60-second grid its file is named for",
            T0 + 1
        )
    );
}
