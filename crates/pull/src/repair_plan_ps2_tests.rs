#![cfg(test)]
//! BASE-COV-PULL-PS2 (3 Nov 2026): the repair plan's arms Route B found
//! unreached on START (2e7a0b6d3b56). Every bar is invented: prices of 5,000.00
//! rupees on an index no exchange lists.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::*;
use crate::session::{Day, IST_OFFSET_SECS, Window};

fn scratch(tag: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let root = std::env::temp_dir().join(format!(
        "brutex-ps2-plan-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _fresh = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn parts(month: YearMonth) -> PathParts<'static> {
    PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "PSPLANIDX",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    }
}

fn bar(on: Day, minute: i64) -> Bar {
    let secs = i64::from(on.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60;
    Bar {
        ts_micros: secs * 1_000_000,
        open: 500_000,
        high: 500_010,
        low: 499_990,
        close: 500_001,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// The month file of `parts`, born with `bars` (none: an empty month).
fn month_file(root: &Path, parts: PathParts<'_>, bars: &[Bar]) -> BarFile {
    let mut file = BarFile::open_or_create(
        root,
        StorePath::new(parts).unwrap(),
        symbol_id(parts.symbol),
    )
    .unwrap();
    if !bars.is_empty() {
        file.append(bars).unwrap();
    }
    file
}

fn november() -> YearMonth {
    YearMonth::new(2020, 11).unwrap()
}

/// PS2-RP-01: the more demanding of two fills wins whichever side it is on;
/// two equal fills give that fill.
#[test]
fn the_more_demanding_fill_wins_from_either_side() {
    use Missing::{Absent, BeforeFirst, Front, Interior, Tail};
    assert_eq!(
        Interior.worst(Front),
        Interior,
        "the less demanding offered"
    );
    assert_eq!(Front.worst(Interior), Interior);
    assert_eq!(Tail.worst(BeforeFirst), Tail);
    assert_eq!(Absent.worst(Absent), Absent);
    assert_eq!(BeforeFirst.worst(Absent), BeforeFirst);
}

/// PS2-RP-02: a special session the calendar of record lists with no window
/// (the 2026-11-08 Muhurat, `calendar::SOURCED_SPECIAL`, timings to be
/// notified) owes no sized windows, for any segment; at the minute rung a held
/// bar on it is counted held and its length is left unjudged, never short and
/// never missing.
#[test]
fn a_session_of_no_recorded_length_is_held_and_left_unjudged() {
    let muhurat = day(2026, 11, 8);
    assert_eq!(
        exchange_kind_of(i64::from(muhurat.days_from_epoch())),
        DayKind::OpenLengthUnmeasured
    );
    for segment in ["INDEX", "CASH", "FNO"] {
        assert_eq!(
            owed(segment, muhurat.days_from_epoch()),
            Owes::Unsized,
            "{segment}"
        );
    }
    let root = scratch("muhurat");
    let month = YearMonth::new(2026, 11).unwrap();
    drop(month_file(&root, parts(month), &[bar(muhurat, 1_110)]));
    let months = plan(&root, parts(month), Window::new(muhurat, muhurat).unwrap());
    let [planned] = months.as_slice() else {
        panic!("{months:?}");
    };
    assert_eq!((planned.sessions, planned.held), (1, 1));
    assert_eq!(planned.length_unjudged, [muhurat]);
    assert!(planned.short.is_empty() && planned.missing.is_empty());
    assert_eq!(planned.refused, None);
    let _cleanup = std::fs::remove_dir_all(&root);
}

/// PS2-RP-03: a series of another exchange is planned with no listing asked;
/// parts no store path can hold are refused in every month the window
/// touches, by the path's own words, and hold no month.
#[test]
fn another_exchange_plans_unlisted_and_unpathed_parts_are_refused() {
    let root = scratch("bse");
    let bse = PathParts {
        exchange: "BSE",
        ..parts(november())
    };
    let window = Window::new(day(2020, 11, 2), day(2020, 12, 3)).unwrap();
    let months = plan(&root, bse, window);
    assert_eq!(months.len(), 2);
    assert!(months.iter().all(|m| m.refused.is_none() && m.bars == 0));
    assert_eq!(
        months[0].missing.len(),
        usize::try_from(months[0].sessions).unwrap()
    );
    let unpathed = PathParts {
        segment: "Index",
        ..parts(november())
    };
    assert!(held_months(&root, unpathed).is_empty());
    let months = plan(&root, unpathed, window);
    assert_eq!(months.len(), 2);
    for month in &months {
        let why = month.refused.as_deref().unwrap();
        assert_eq!(
            why,
            StorePath::new(PathParts {
                month: month.month,
                ..unpathed
            })
            .unwrap_err()
            .to_string()
        );
        assert_eq!(month.sessions, 0);
    }
    let _cleanup = std::fs::remove_dir_all(&root);
}

/// PS2-RP-04: only `<yyyy>-<mm>.bin` names a held month: a year or a month of
/// another width, letters, a month past 12, another extension and a stray
/// name are read past.
#[test]
fn only_a_four_and_two_digit_bin_name_is_a_held_month() {
    let root = scratch("names");
    drop(month_file(
        &root,
        parts(november()),
        &[bar(day(2020, 11, 27), 0)],
    ));
    let dir = StorePath::new(parts(november()))
        .unwrap()
        .to_path_buf(&root)
        .parent()
        .unwrap()
        .to_path_buf();
    for stray in [
        "2020-1.bin",
        "20201-11.bin",
        "202-11.bin",
        "20x0-12.bin",
        "2020-1x.bin",
        "2020-13.bin",
        "2020-12.txt",
        "notes",
    ] {
        std::fs::write(dir.join(stray), b"invented").unwrap();
    }
    assert_eq!(held_months(&root, parts(november())), [november()]);
    let _cleanup = std::fs::remove_dir_all(&root);
}

/// PS2-RP-05: an empty month file (born, no bar) holds no span: every session
/// of the window is missing as absent, an ordinary pull's, never a repair's.
#[test]
fn an_empty_month_file_holds_no_span_and_owes_its_sessions_absent() {
    let root = scratch("empty");
    drop(month_file(&root, parts(november()), &[]));
    let window = Window::new(day(2020, 11, 2), day(2020, 11, 6)).unwrap();
    let [month] = plan(&root, parts(november()), window).try_into().unwrap();
    assert_eq!((month.bars, month.span, month.held), (0, None, 0));
    assert_eq!(month.sessions, 5);
    assert!(month.missing.iter().all(|(_, why)| *why == Missing::Absent));
    assert!(!month.needs_repair());
    let _cleanup = std::fs::remove_dir_all(&root);
}

/// PS2-RP-06: a month whose bar file is not one (a directory at its name) is
/// refused by the store's own words and judged no further.
#[test]
fn a_month_whose_file_the_store_refuses_is_refused_and_unjudged() {
    let root = scratch("dir");
    let path = StorePath::new(parts(november()))
        .unwrap()
        .to_path_buf(&root);
    std::fs::create_dir_all(&path).unwrap();
    let window = Window::new(day(2020, 11, 2), day(2020, 11, 6)).unwrap();
    let [month] = plan(&root, parts(november()), window).try_into().unwrap();
    let why = month.refused.as_deref().unwrap();
    assert!(why.contains(&path.display().to_string()), "{why}");
    assert_eq!((month.sessions, month.held, month.bars), (0, 0, 0));
    assert!(month.missing.is_empty());
    let _cleanup = std::fs::remove_dir_all(&root);
}

/// PS2-RP-07: days no calendar of record attests are named unattested and
/// owed nothing: no session counted, nothing missing.
#[test]
fn days_no_calendar_attests_are_unattested_and_owed_nothing() {
    let root = scratch("unattested");
    let january = Window::new(day(2030, 1, 1), day(2030, 1, 3)).unwrap();
    let [month] = plan(&root, parts(YearMonth::new(2030, 1).unwrap()), january)
        .try_into()
        .unwrap();
    assert_eq!(
        month.unattested,
        [day(2030, 1, 1), day(2030, 1, 2), day(2030, 1, 3)]
    );
    assert_eq!(month.sessions, 0);
    assert!(month.missing.is_empty());
    let _cleanup = std::fs::remove_dir_all(&root);
}

/// PS2-RP-08: a window that owes no minute (empty, or `to` not after `from`)
/// is passed over; one that owes none at all judges nothing. The window
/// beside it is judged as alone.
#[test]
fn an_empty_window_is_passed_over_and_none_judges_nothing() {
    let root = scratch("judge");
    let on = day(2020, 11, 27);
    let file = month_file(&root, parts(november()), &[bar(on, 555)]);
    let bars = file.header().n_valid;
    let number = on.days_from_epoch();
    assert_eq!(
        judge(&file, bars, number, ([(600, 600), (0, 0)], 2)),
        Ok(None)
    );
    assert_eq!(
        judge(&file, bars, number, ([(700, 650), (0, 0)], 1)),
        Ok(None)
    );
    assert_eq!(judge(&file, bars, number, ([(0, 0), (0, 0)], 0)), Ok(None));
    let alone = judge(&file, bars, number, ([(555, 930), (0, 0)], 1)).unwrap();
    let beside = judge(&file, bars, number, ([(600, 600), (555, 930)], 2)).unwrap();
    assert_eq!(beside, alone);
    let found = beside.unwrap();
    assert_eq!((found.owed, found.held), (375, 1));
    drop(file);
    let _cleanup = std::fs::remove_dir_all(&root);
}

/// A full invented session of minute bars on `on`: 09:15 to 15:29.
fn session_of(on: Day) -> Vec<Bar> {
    (555..930).map(|minute| bar(on, minute)).collect()
}

/// Cuts a month file to its header, so every record read past it fails.
fn cut_to_header(at: &Path) {
    std::fs::OpenOptions::new()
        .write(true)
        .open(at)
        .unwrap()
        .set_len(32_768)
        .unwrap();
}

/// PS2 (D-7336): A MONTH WHOSE ROWS FAIL TO READ AFTER IT OPENED is refused at
/// its first session, by that day, and judged no further.
#[test]
fn a_month_whose_rows_fail_to_read_after_it_opened_is_refused_at_its_first_session() {
    let root = scratch("cut-opened");
    drop(month_file(
        &root,
        parts(november()),
        &session_of(day(2020, 11, 2)),
    ));
    seam::plant("repair.opened", cut_to_header);
    let window = Window::new(day(2020, 11, 2), day(2020, 11, 6)).unwrap();
    let [month] = plan(&root, parts(november()), window).try_into().unwrap();
    let why = month.refused.as_deref().unwrap();
    assert!(why.starts_with("2020-11-02: "), "{why}");
    assert_eq!(
        (month.sessions, month.held),
        (1, 0),
        "stopped at its first session"
    );
    let _cleanup = std::fs::remove_dir_all(&root);
}

/// PS2 (D-7336): A DAY WHOSE WINDOWS FAIL TO READ AFTER IT WAS COUNTED is
/// refused by that day: the judge never answers for bars it could not read.
#[test]
fn a_day_whose_windows_fail_to_read_after_it_was_counted_is_refused_by_its_judge() {
    let root = scratch("cut-counted");
    drop(month_file(
        &root,
        parts(november()),
        &session_of(day(2020, 11, 2)),
    ));
    seam::plant("repair.counted", cut_to_header);
    let window = Window::new(day(2020, 11, 2), day(2020, 11, 6)).unwrap();
    let [month] = plan(&root, parts(november()), window).try_into().unwrap();
    let why = month.refused.as_deref().unwrap();
    assert!(why.starts_with("2020-11-02: "), "{why}");
    assert_eq!(
        (month.sessions, month.held),
        (1, 1),
        "counted, then not judged"
    );
    assert!(month.short.is_empty());
    let _cleanup = std::fs::remove_dir_all(&root);
}
