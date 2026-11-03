#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail; and one test per edge, each \
              holding every arm of its row."
)]

//! BASE-COV-PULL-PS3: the verifier's edges on disk, each driven through the
//! production path and its answer asserted: a walk that cannot list, a cold
//! run beside months and days that will not read, days outside the 1min
//! months, a peer month busy or damaged, the last day a `Day` can name, a
//! stop inside a series and a series that fails beside a busy one, a host
//! that starts no thread, the window path at 1day and beside an alert or a
//! month that will not write or open, and the merge beside a spool tree, a
//! live lock, a merge log or a folder that refuses, and an interrupted
//! commit. Every bar, body and spool here is invented: the instruments are
//! `ZZ`-named and the prices sit near 11,111.00, which no real print names.

use std::path::{Path, PathBuf};

use super::spool::{self, LogRecord, LogState, SpoolId, SpoolRecord, SpoolWriter};
use super::tests::{
    append_raw, bar_at, day, flip, full_day, index, keep_answer, kite_body, regular, row_of,
    scratch, spooled, the_record, utc_s, whole_day, write_month, write_peer, year_generation,
};
use super::*;
use crate::fetch::{RawRow, RowCause};
use crate::session::Day;

// ------------------------------------------------------------- fixtures --

/// A series of `segment` named `symbol` on NSE.
fn named(segment: Segment, symbol: &str) -> Series {
    Series {
        exchange: Exchange::Nse,
        segment,
        symbol: symbol.to_owned(),
        contract: None,
    }
}

/// The bar file of `series` at `tf` in `month` under `root`.
fn month_path(root: &Path, series: &Series, tf: Timeframe, month: YearMonth) -> PathBuf {
    StorePath::new(series.parts(tf, month))
        .unwrap()
        .to_path_buf(root)
}

/// A day bar of `on`, stamped at IST midnight as Zerodha stamps one.
fn daily_bar(on: Day, price: i64) -> Bar {
    let mut bar = bar_at(on, 0, price);
    bar.ts_micros = utc_s(on, 0) * 1_000_000;
    bar
}

/// Flips one byte of the stored record whose stamp and open are `bar`'s: the
/// month still opens (its header is whole), and every read of the block
/// holding that record fails its sealed CRC (D-1433).
fn damage(path: &Path, bar: &Bar) {
    let bytes = std::fs::read(path).unwrap();
    let mut needle = bar.ts_micros.to_le_bytes().to_vec();
    needle.extend(bar.open.to_le_bytes());
    let at = bytes
        .windows(needle.len())
        .position(|window| window == needle.as_slice())
        .expect("the record is stored");
    flip(path, u64::try_from(at).unwrap() + 8);
}

/// The records of `run`'s spool of (`segment`, `tf`, `month`), in spool order.
fn spool_of(
    root: &Path,
    run: u64,
    segment: Segment,
    tf: Timeframe,
    month: YearMonth,
) -> Vec<SpoolRecord> {
    let id = SpoolId {
        run,
        segment,
        timeframe: tf,
        month,
    };
    spool::read_spool(&id.path(root), &id).unwrap().records
}

/// The one record of `key` on `on` among `records`.
fn record_on<'a>(records: &'a [SpoolRecord], key: &str, on: Day) -> &'a SpoolRecord {
    let found: Vec<&SpoolRecord> = records
        .iter()
        .filter(|record| record.day == on && record.key.text() == key)
        .collect();
    assert_eq!(found.len(), 1, "{key} {on}: {records:?}");
    found[0]
}

/// The (code, reason word) of a spooled record.
fn said(record: &SpoolRecord) -> (Code, &'static str) {
    (
        record.facts.code,
        words::reason_word(record.facts.reason).unwrap_or(""),
    )
}

/// Keeps `offered` as one answer of `key` at `tf` over `window`, as IA-8's
/// keep does after a landing that landed `landed` and refused `refused`.
fn keep_offered(
    root: &Path,
    key: &InstrumentKey,
    (tf, rung): (Timeframe, crate::vendor::Granularity),
    window: (Day, Day),
    offered: response_keep::Offered<'_>,
    (landed, refused): (&[Bar], &[response_keep::RefusedRow]),
) {
    let pack = response_keep::Pack {
        vendor: Vendor::Zerodha,
        key,
        timeframe: tf,
    };
    let landing = response_keep::Landing {
        window: crate::session::Window::new(window.0, window.1).unwrap(),
        first_stamp: landed.first().map_or(0, |b| b.ts_micros / 1_000_000),
        last_stamp: landed.last().map_or(0, |b| b.ts_micros / 1_000_000),
        rows_decoded: u32::try_from(landed.len()).unwrap(),
        rows_landed: u32::try_from(landed.len()).unwrap(),
        rows_refused: u32::try_from(refused.len()).unwrap(),
        refused,
        run: 5,
        rung: rung as u8,
        rules_version: 1,
        door: response_keep::Door::FromWindow,
    };
    response_keep::append(root, pack, offered, &landing).unwrap();
}

/// `ZZIDX`'s instrument key, as its kept answers are filed.
fn zzidx() -> InstrumentKey {
    InstrumentKey::index(Exchange::Nse, "ZZIDX").unwrap()
}

/// Runs `body` in a child of this test binary in which no thread can start:
/// `RUST_MIN_STACK` asks every new thread for a stack of 1 PiB, which no host
/// maps, so `std::thread::Builder::spawn` refuses (`WouldBlock`) and libtest
/// runs the child's one test on its main thread. The environment is
/// inherited, so a coverage run's `LLVM_PROFILE_FILE` reaches the child. The
/// parent asserts the child ran exactly one test and passed.
fn where_no_thread_starts(test: &str, body: impl FnOnce()) {
    const CHILD: &str = "BRUTEX_NO_THREAD_STARTS_CHILD";
    if std::env::var_os(CHILD).is_some() {
        body();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().expect("this binary"))
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .env("RUST_MIN_STACK", (1_u64 << 50).to_string())
        .output()
        .expect("the child test process starts");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let words = format!(
        "child {test} failed or ran no test:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{words}");
    assert!(stdout.contains("1 passed"), "{words}");
}

// ------------------------------------------------------------- the walk --

/// The walk names what it cannot list, and reads past what is no series or
/// no month: a file where a segment's folder belongs refuses the walk with
/// the host's words (never "no series"); a contract folder holding no rung is
/// no series, and one holding a rung is; a rung's months are read by name, a
/// month held in both forms is one month, and a name that is no month, a
/// month that is no month (`2024-13`) and a name that is not text are stray
/// (extremes; APFS refuses to make the last, so on macOS it cannot arise);
/// a file where a rung's folder belongs is refused by its words; a link at
/// the first 1day month's name that points nowhere is a month not held, and
/// a held month with no bar serves no first day.
#[test]
fn the_series_walk_names_what_it_cannot_list() {
    use std::os::unix::ffi::OsStrExt as _;
    let root = scratch("walk-file");
    let segment = root.join("bars/zerodha/NSE/INDEX");
    std::fs::create_dir_all(segment.parent().unwrap()).unwrap();
    std::fs::write(&segment, b"not a folder").unwrap();
    let mut stray = 0;
    let refused = series_held(&root, &mut stray).unwrap_err();
    assert!(
        refused.starts_with(&format!("{}: ", segment.display())),
        "{refused}"
    );

    let root = scratch("walk-contracts");
    let fno = root.join("bars/zerodha/NSE/FNO/ZZIDX");
    std::fs::create_dir_all(fno.join("2024-04-25-FUT")).unwrap();
    std::fs::create_dir_all(fno.join("2024-04-25-2222200-CE").join("1min")).unwrap();
    std::fs::write(fno.join("2024-04-25-2222200-CE/1min/2024-04.bin"), b"").unwrap();
    let mut stray = 0;
    let held = series_held(&root, &mut stray).unwrap();
    assert_eq!(
        held.iter().map(Series::key_text).collect::<Vec<_>>(),
        ["NSE-FNO-ZZIDX-2024-04-25-2222200-CE"]
    );
    assert_eq!(stray, 0);
    // A contract's months are listed under its own folder.
    assert_eq!(
        months_held(&root, &held[0], Timeframe::MINUTE_1, &mut stray).unwrap(),
        [YearMonth::new(2024, 4).unwrap()]
    );

    let root = scratch("walk-months");
    let dir = rung_dir(&root, &index(), Timeframe::MINUTE_1);
    std::fs::create_dir_all(&dir).unwrap();
    for name in [
        "2024-04.bin",
        "2024-05.bin",
        "2024-05.sbar",
        "notamonth.bin",
        "2024-13.sbar",
        "README",
    ] {
        std::fs::write(dir.join(name), b"").unwrap();
    }
    let odd = std::fs::write(
        dir.join(std::ffi::OsStr::from_bytes(b"2024-06\xff.bin")),
        b"",
    )
    .is_ok();
    assert_eq!(odd, !cfg!(target_os = "macos"));
    let mut stray = 0;
    let mut months = months_held(&root, &index(), Timeframe::MINUTE_1, &mut stray).unwrap();
    months.sort_unstable();
    assert_eq!(
        months,
        [
            YearMonth::new(2024, 4).unwrap(),
            YearMonth::new(2024, 5).unwrap()
        ]
    );
    assert_eq!(stray, 2 + u64::from(odd));
    let five = rung_dir(&root, &index(), Timeframe::MINUTE_5);
    std::fs::write(&five, b"not a folder").unwrap();
    let refused = months_held(&root, &index(), Timeframe::MINUTE_5, &mut stray).unwrap_err();
    assert!(
        refused.starts_with(&format!("{}: ", five.display())),
        "{refused}"
    );

    let root = scratch("walk-first");
    let month = regular().year_month().unwrap();
    let first = month_path(&root, &index(), Timeframe::DAY_1, month);
    std::fs::create_dir_all(first.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(root.join("nowhere"), &first).unwrap();
    assert_eq!(first_served_day(&root, &index()), Ok(None));
    std::fs::remove_file(&first).unwrap();
    let series = index();
    let path = StorePath::new(series.parts(Timeframe::DAY_1, month)).unwrap();
    drop(BarFile::open_or_create(&root, path, repair_plan::symbol_id("ZZIDX")).unwrap());
    assert!(first.is_file(), "the month is held, with no bar");
    assert_eq!(first_served_day(&root, &index()), Ok(None));
}

// ------------------------------------------------------------- the cold run --

/// A cold run beside months and days that will not read names every one and
/// reads the rest (a combination of every refusal at once): `ZZIDX`'s 1min,
/// 5min and 1day months open but their first block fails its sealed CRC, so
/// each day read on or before it is a fault by day and rung, its 1min record
/// `store_unreadable (month_refused)` and the rungs that did read recorded;
/// its 2min month is a folder (a refusal at open, said once); a Sunday whose
/// 1min read failed is still recorded, never dropped as a closed day. A
/// Sunday holding a minute (`ZZSUN`) is `extra_in_store`. `ZZFIRST`'s first
/// 1day month will not open, so its first day bar is a fault and its days
/// read `store_unreadable` at 1day; `ZZFILE`'s 1min and 1day folders are
/// files, each named.
#[test]
fn a_cold_run_names_every_month_and_day_it_cannot_read() {
    let root = scratch("cold-unread");
    let (sunday, monday, on) = (day(2024, 4, 21), day(2024, 4, 22), regular());
    let month = on.year_month().unwrap();
    let series = index();
    let minutes = full_day(on, &[]);
    write_month(&root, &series, Timeframe::MINUTE_1, &minutes);
    damage(
        &month_path(&root, &series, Timeframe::MINUTE_1, month),
        &minutes[0],
    );
    let (buckets, _) = refold(&series, on.days_from_epoch(), &minutes, Timeframe::MINUTE_5);
    let five: Vec<Bar> = buckets.iter().map(|b| b.bar).collect();
    write_month(&root, &series, Timeframe::MINUTE_5, &five);
    damage(
        &month_path(&root, &series, Timeframe::MINUTE_5, month),
        &five[0],
    );
    let daily = daily_bar(on, 1_111_100);
    write_month(&root, &series, Timeframe::DAY_1, &[daily]);
    damage(&month_path(&root, &series, Timeframe::DAY_1, month), &daily);
    std::fs::create_dir_all(month_path(&root, &series, Timeframe::MINUTE_2, month)).unwrap();
    let held_sunday = named(Segment::Index, "ZZSUN");
    write_month(
        &root,
        &held_sunday,
        Timeframe::MINUTE_1,
        &[bar_at(sunday, 600, 1_111_100)],
    );
    let unfirst = named(Segment::Index, "ZZFIRST");
    std::fs::create_dir_all(month_path(&root, &unfirst, Timeframe::DAY_1, month)).unwrap();
    let files = named(Segment::Index, "ZZFILE");
    std::fs::create_dir_all(rung_dir(&root, &files, Timeframe::MINUTE_5)).unwrap();
    for rung in [Timeframe::MINUTE_1, Timeframe::DAY_1] {
        std::fs::write(rung_dir(&root, &files, rung), b"not a folder").unwrap();
    }

    let run = 120;
    let writer = SpoolWriter::new(&root, run, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: sunday,
        to: on,
        only: Vec::new(),
    };
    let report = cold_on(&root, &request, &writer, None, 1).unwrap();
    drop(writer);
    let named_by = |prefix: &str, middle: &str| {
        report
            .refused
            .iter()
            .filter(|words| words.starts_with(prefix) && words.contains(middle))
            .count()
    };
    for rung in ["1min", "5min", "1day"] {
        assert_eq!(
            named_by("NSE-INDEX-ZZIDX 2024-04-2", &format!(" {rung}: ")),
            3,
            "{rung} on the 21st, 22nd and 23rd: {:?}",
            report.refused
        );
    }
    assert_eq!(
        named_by("NSE-INDEX-ZZIDX 2024-04 2min: ", "is a directory"),
        1
    );
    assert_eq!(named_by("NSE-INDEX-ZZIDX first day bar: ", "block 0"), 1);
    assert_eq!(
        named_by("NSE-INDEX-ZZFIRST first day bar: ", "is a directory"),
        1
    );
    assert_eq!(
        named_by("NSE-INDEX-ZZFIRST 2024-04 1day: ", "is a directory"),
        1
    );
    for middle in ["first day bar: ", "1min months: ", "1day months: "] {
        assert_eq!(named_by("NSE-INDEX-ZZFILE ", middle), 1, "{middle}");
    }
    assert_eq!(report.refused.len(), 16, "{:?}", report.refused);
    assert!(report.busy.is_empty());
    assert_eq!(
        (
            report.series,
            report.months,
            report.beyond_months,
            report.days
        ),
        (4, 3, 1, 8)
    );

    let minutes_spooled = spool_of(&root, run, Segment::Index, Timeframe::MINUTE_1, month);
    for at in [sunday, monday, on] {
        assert_eq!(
            said(record_on(&minutes_spooled, "NSE-INDEX-ZZIDX", at)),
            (Code::StoreUnreadable, "month_refused"),
            "{at}"
        );
    }
    assert_eq!(
        said(record_on(&minutes_spooled, "NSE-INDEX-ZZSUN", sunday)).0,
        Code::ExtraInStore
    );
    let two = spool_of(&root, run, Segment::Index, Timeframe::MINUTE_2, month);
    assert_eq!(
        said(record_on(&two, "NSE-INDEX-ZZIDX", on)),
        (Code::StoreUnreadable, "month_refused")
    );
    let days = spool_of(&root, run, Segment::Index, Timeframe::DAY_1, month);
    for at in [monday, on] {
        assert_eq!(
            said(record_on(&days, "NSE-INDEX-ZZFIRST", at)),
            (Code::StoreUnreadable, "month_refused"),
            "{at}"
        );
    }
    assert!(
        days.iter().all(|r| r.key.text() == "NSE-INDEX-ZZFIRST"),
        "a 1day month whose days will not read gives no record"
    );
}

/// Outside the 1min months and the calendar of record (March 2016), a day
/// holding no bar at any rung and no kept answer is counted, never judged,
/// but a day holding a derived bar, or a kept 1day answer, is judged: the
/// derived bar `not_compared (calendar_unverified)`, the answered day with no
/// bar the vendor's hole (combinations: beyond the minutes, unattested and
/// held at once). A 1day month before the asked span is never walked.
#[test]
fn a_day_outside_the_minutes_with_a_derived_bar_or_a_kept_day_answer_is_judged() {
    let root = scratch("beyond-held");
    let (tenth, eleventh) = (day(2016, 3, 10), day(2016, 3, 11));
    let month = tenth.year_month().unwrap();
    let old = named(Segment::Index, "ZZOLD");
    write_month(
        &root,
        &old,
        Timeframe::MINUTE_5,
        &[bar_at(tenth, 555, 1_111_100)],
    );
    write_month(
        &root,
        &old,
        Timeframe::DAY_1,
        &[daily_bar(day(2016, 2, 10), 1_111_100)],
    );
    let kept_series = index();
    let daily = daily_bar(tenth, 1_111_100);
    write_month(&root, &kept_series, Timeframe::DAY_1, &[daily]);
    let answered =
        response_keep::Answered::of(crate::fetch::RawWindow::default(), kite_body(&[daily]), 200);
    keep_offered(
        &root,
        &zzidx(),
        (Timeframe::DAY_1, crate::vendor::Granularity::Day1),
        (tenth, eleventh),
        response_keep::Offered::Answer(&answered),
        (&[daily], &[]),
    );
    let run = 125;
    let writer = SpoolWriter::new(&root, run, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: day(2016, 3, 1),
        to: day(2016, 3, 31),
        only: vec!["ZZOLD".to_owned(), "ZZIDX".to_owned()],
    };
    let report = cold_on(&root, &request, &writer, None, 1).unwrap();
    drop(writer);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert_eq!(
        (
            report.months,
            report.beyond_months,
            report.days,
            report.records
        ),
        (2, 2, 3, 3),
        "{report:?}"
    );
    assert_eq!(
        report.unattested_days, 50,
        "ZZOLD's 30 days without its bar, ZZIDX's 12th to 31st"
    );
    let five = spool_of(&root, run, Segment::Index, Timeframe::MINUTE_5, month);
    assert_eq!(
        said(record_on(&five, "NSE-INDEX-ZZOLD", tenth)),
        (Code::NotCompared, "calendar_unverified")
    );
    let days = spool_of(&root, run, Segment::Index, Timeframe::DAY_1, month);
    let matched = record_on(&days, "NSE-INDEX-ZZIDX", tenth);
    assert_eq!((matched.facts.code, matched.receipt), (Code::Match, 1));
    assert_eq!(
        said(record_on(&days, "NSE-INDEX-ZZIDX", eleventh)),
        (Code::VendorHole, "vendor_empty")
    );
    assert!(days.iter().all(|r| r.key.text() == "NSE-INDEX-ZZIDX"));
}

/// A GDFL peer month a writer holds is busy, said apart and never a fault,
/// and so is the series' own first 1day month (twice: the first day bar and
/// the month's read); a peer month that opens but whose seconds fail their
/// sealed CRC is a fault by day. Either way the minute the store lacks reads
/// `peer_unavailable`, never "neither vendor has it".
#[test]
fn a_peer_month_that_is_busy_or_damaged_is_said_apart() {
    let on = regular();
    let month = on.year_month().unwrap();
    let request = ColdRequest {
        from: on,
        to: on,
        only: vec!["ZZIDX".to_owned()],
    };
    let peer_parts = || store::path::PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "ZZIDX",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month,
        file: store::path::FileKind::Bars,
    };
    let peer_unavailable = |root: &Path, run: u64| {
        let minutes = spool_of(root, run, Segment::Index, Timeframe::MINUTE_1, month);
        let record = record_on(&minutes, "NSE-INDEX-ZZIDX", on);
        (
            record.facts.flags & flags::PEER_UNAVAILABLE != 0,
            record.facts.code,
        )
    };

    let root = scratch("peer-busy");
    write_month(&root, &index(), Timeframe::MINUTE_1, &full_day(on, &[600]));
    write_month(
        &root,
        &index(),
        Timeframe::DAY_1,
        &[daily_bar(on, 1_111_100)],
    );
    write_peer(&root, on, &[33_300, 36_060]);
    let series = index();
    let peer_hold = BarFile::open_or_create(
        &root,
        StorePath::new(peer_parts()).unwrap(),
        repair_plan::symbol_id("ZZIDX"),
    )
    .unwrap();
    let day_hold = BarFile::open_or_create(
        &root,
        StorePath::new(series.parts(Timeframe::DAY_1, month)).unwrap(),
        repair_plan::symbol_id("ZZIDX"),
    )
    .unwrap();
    let writer = SpoolWriter::new(&root, 126, Mode::ZerodhaCold, 1);
    let report = cold_on(&root, &request, &writer, None, 1).unwrap();
    drop((peer_hold, day_hold));
    drop(writer);
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    let busy_by = |middle: &str| {
        report
            .busy
            .iter()
            .filter(|words| words.contains(middle) && words.contains("month_busy"))
            .count()
    };
    assert_eq!(
        busy_by("NSE-INDEX-ZZIDX first day bar: "),
        1,
        "{:?}",
        report.busy
    );
    assert_eq!(busy_by("NSE-INDEX-ZZIDX 2024-04 1day: "), 1);
    assert_eq!(busy_by("NSE-INDEX-ZZIDX 2024-04 gdfl 1s peer: "), 1);
    assert_eq!(report.busy.len(), 3);
    assert_eq!(peer_unavailable(&root, 126), (true, Code::NoLedger));

    let root = scratch("peer-damaged");
    write_month(&root, &index(), Timeframe::MINUTE_1, &full_day(on, &[600]));
    write_peer(&root, on, &[33_300, 36_060]);
    let first_second = Bar {
        ts_micros: (utc_s(on, 0) + 33_300) * 1_000_000,
        open: 1_111_100,
        ..Bar::default()
    };
    damage(
        &StorePath::new(peer_parts()).unwrap().to_path_buf(&root),
        &first_second,
    );
    let writer = SpoolWriter::new(&root, 127, Mode::ZerodhaCold, 1);
    let report = cold_on(&root, &request, &writer, None, 1).unwrap();
    drop(writer);
    assert_eq!(report.refused.len(), 1, "{:?}", report.refused);
    assert!(
        report.refused[0].starts_with("NSE-INDEX-ZZIDX 2024-04-23 gdfl 1s peer: "),
        "{:?}",
        report.refused
    );
    assert_eq!(peer_unavailable(&root, 127), (true, Code::NoLedger));
}

/// The last day a `Day` can name, 31 December 9999 (out of the box: the end
/// of the calendar type itself, far past any record): a cold run over its
/// month walks every day to it and stops there, where no next day exists,
/// recording its one bar; a window asked from it into the days past it
/// verifies the one day a `Day` names and passes over the rest, never failing
/// the window.
#[test]
fn the_last_day_a_day_can_name_ends_the_walk_and_cuts_a_window() {
    let last = Day::new(9999, 12, 31).unwrap();
    assert!(last.succ().is_err(), "no day after it");
    let root = scratch("last-day");
    write_month(
        &root,
        &index(),
        Timeframe::DAY_1,
        &[daily_bar(last, 1_111_100)],
    );
    let run = 128;
    let writer = SpoolWriter::new(&root, run, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: Day::new(9999, 12, 1).unwrap(),
        to: last,
        only: vec!["ZZIDX".to_owned()],
    };
    let report = cold_on(&root, &request, &writer, None, 1).unwrap();
    assert_eq!(
        (
            report.months,
            report.beyond_months,
            report.days,
            report.records,
            report.unattested_days
        ),
        (1, 1, 1, 1, 0),
        "{report:?}"
    );
    assert!(!report.stopped);
    let days = spool_of(
        &root,
        run,
        Segment::Index,
        Timeframe::DAY_1,
        last.year_month().unwrap(),
    );
    assert_eq!(
        said(record_on(&days, "NSE-INDEX-ZZIDX", last)).0,
        Code::NoResponseKept
    );

    let series = index();
    let window = WindowInput {
        series: &series,
        rung: Timeframe::MINUTE_1,
        rows: &[],
        asked: (utc_s(last, 0), utc_s(last, 0) + 4 * 86_400 - 1),
        receipt: 1,
        refused: &[],
        rows_refused: 0,
        first_day_bar: None,
        shape: &Shape::default(),
    };
    let wrote = verify_window(&root, &window, &GapLedger::default(), &writer).unwrap();
    assert_eq!(
        wrote.records, 8,
        "1min and seven derived rungs of the last day only"
    );
    let minutes = spool_of(
        &root,
        run,
        Segment::Index,
        Timeframe::MINUTE_1,
        last.year_month().unwrap(),
    );
    assert_eq!(
        said(record_on(&minutes, "NSE-INDEX-ZZIDX", last)),
        (Code::NotCompared, "calendar_past_record")
    );
}

/// A stop seen inside a series, before its first month, ends that series and
/// the run `stopped` with nothing read and nothing spooled; the worker takes
/// no further series.
#[test]
fn a_stop_inside_a_series_ends_the_run_stopped() {
    let root = scratch("stop-inside");
    let on = regular();
    write_month(&root, &index(), Timeframe::MINUTE_1, &full_day(on, &[]));
    let writer = SpoolWriter::new(&root, 129, Mode::ZerodhaCold, 1);
    let stop = StopControl::tripping_at(root.join("verify/zerodha.stop"), 129, 2);
    let request = ColdRequest {
        from: on,
        to: on,
        only: Vec::new(),
    };
    let report = cold_on(&root, &request, &writer, Some(&stop), 1).unwrap();
    assert!(report.stopped);
    assert_eq!(
        (report.series, report.months, report.days, report.records),
        (1, 0, 0, 0)
    );
    assert_eq!(
        stop.checks(),
        2,
        "between series once, inside the series once"
    );
    drop(writer);
    assert!(spool::waiting(&root).unwrap().spools.is_empty());
}

/// A series whose spool append fails ends the run with its words, and the
/// other worker takes no series after the one it holds: on two workers, an
/// index series whose 1min spool name is a link fails at once while a share
/// waits for its first 1day month a writer holds; the run's progress names
/// the one refusal. The refusal is returned whichever worker took which
/// series.
#[test]
fn a_series_that_fails_stops_the_other_worker_at_its_next_series() {
    let root = scratch("cold-fail");
    let on = regular();
    let month = on.year_month().unwrap();
    let failing = named(Segment::Index, "ZZFAIL");
    write_month(&root, &failing, Timeframe::MINUTE_1, &full_day(on, &[]));
    let slow = named(Segment::Cash, "ZZSLOW");
    write_month(&root, &slow, Timeframe::MINUTE_1, &full_day(on, &[]));
    write_month(&root, &slow, Timeframe::DAY_1, &[daily_bar(on, 1_111_100)]);
    let run = 130;
    let planted = SpoolId {
        run,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month,
    }
    .path(&root);
    std::fs::create_dir_all(planted.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(root.join("elsewhere"), &planted).unwrap();
    let hold = BarFile::open_or_create(
        &root,
        StorePath::new(slow.parts(Timeframe::DAY_1, month)).unwrap(),
        repair_plan::symbol_id("ZZSLOW"),
    )
    .unwrap();
    let writer = SpoolWriter::new(&root, run, Mode::ZerodhaCold, 1);
    let progress = crate::gdfl_run::ProgressFile::of_record(
        root.join(PROGRESS_COLD),
        progress_record("zerodha-verify", 1, run, 2, Some((on, on))),
    );
    let request = ColdRequest {
        from: on,
        to: on,
        only: Vec::new(),
    };
    let refused = cold_observed(&root, &request, &writer, None, 2, Some(&progress)).unwrap_err();
    drop(hold);
    assert!(
        refused.contains(&planted.display().to_string()),
        "{refused}"
    );
    assert_eq!(progress.record().refusals_total, 1);
    assert!(planted.is_symlink(), "never followed, never replaced");
}

/// A host on which no thread starts (out of the box: every new thread is
/// asked for a stack no host maps) loses nothing in silence: the worker pool
/// starts none and answers nothing, a cold run refuses by name, and a merge
/// names every spool its workers never merged and leaves it waiting.
#[test]
fn no_worker_that_cannot_start_loses_a_series_or_a_spool() {
    where_no_thread_starts(
        "zerodha_verify::cov_store_tests::no_worker_that_cannot_start_loses_a_series_or_a_spool",
        no_worker_that_cannot_start_loses_a_series_or_a_spool_body,
    );
}

fn no_worker_that_cannot_start_loses_a_series_or_a_spool_body() {
    assert!(
        std::thread::Builder::new().spawn(|| ()).is_err(),
        "no thread starts here"
    );
    assert!(on_workers("zerodha-test", 3, &|| 7_u8).is_empty());

    let root = scratch("no-thread");
    let on = regular();
    write_month(&root, &index(), Timeframe::MINUTE_1, &full_day(on, &[]));
    let writer = SpoolWriter::new(&root, 131, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: on,
        to: on,
        only: Vec::new(),
    };
    assert_eq!(
        cold_on(&root, &request, &writer, None, 2),
        Err("no cold verification worker could start".to_owned())
    );
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-ZZIDX", Code::Match, 1)],
        )
        .unwrap();
    drop(writer);
    let merged = merge_observed_on(&root, None, None, None, 2).unwrap();
    assert_eq!((merged.spools_merged, merged.generations), (0, 0));
    assert_eq!(merged.waiting, 1);
    assert_eq!(merged.faults.len(), 1, "{:?}", merged.faults);
    assert!(
        merged.faults[0].ends_with(
            "was not merged: its merge worker panicked or never started; it waits for the next merge"
        ),
        "{:?}",
        merged.faults
    );
}

// ------------------------------------------------------------ the window --

/// The window path at 1day: one window over two days of one month opens the
/// month once, verifies each day's bar against the answer (the 23rd held and
/// equal, `match`; the 24th answered and not held, `missing_in_store`), and
/// spools 1day records only, never the 1min record a day bar's window does
/// not answer.
#[test]
fn a_daily_window_over_two_days_is_verified_against_its_day_bars() {
    let root = scratch("window-daily");
    let (on, next) = (regular(), day(2024, 4, 24));
    let month = on.year_month().unwrap();
    let held = daily_bar(on, 1_111_100);
    write_month(&root, &index(), Timeframe::DAY_1, &[held]);
    let rows: Vec<RawRow> = [held, daily_bar(next, 1_111_200)]
        .iter()
        .map(row_of)
        .collect();
    let series = index();
    let window = WindowInput {
        series: &series,
        rung: Timeframe::DAY_1,
        rows: &rows,
        asked: (whole_day(on).0, whole_day(next).1),
        receipt: 3,
        refused: &[],
        rows_refused: 0,
        first_day_bar: None,
        shape: &Shape::default(),
    };
    let run = 132;
    let writer = SpoolWriter::new(&root, run, Mode::ZerodhaMerge, 1);
    let wrote = verify_window(&root, &window, &GapLedger::default(), &writer).unwrap();
    drop(writer);
    assert_eq!(wrote.records, 2);
    let days = spool_of(&root, run, Segment::Index, Timeframe::DAY_1, month);
    let first = record_on(&days, "NSE-INDEX-ZZIDX", on);
    assert_eq!((first.facts.code, first.receipt), (Code::Match, 3));
    assert_eq!(
        said(record_on(&days, "NSE-INDEX-ZZIDX", next)),
        (Code::MissingInStore, "unexplained")
    );
    let minute = SpoolId {
        run,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month,
    };
    assert!(
        !minute.path(&root).exists(),
        "no 1min record of a 1day window"
    );
}

/// A window whose answer changed shape still lands its records when the
/// alert cannot be written (a folder at the alert queue's name: an error
/// event, never a failed landing), and a month that will not open (a folder
/// at the 5min month's name) is its rung's `store_unreadable
/// (month_refused)` record with an event, never a lost window (a
/// combination: both at once). A changed shape over a window that names no
/// day (asked before 1970) puts no alert and writes nothing.
#[test]
fn a_window_whose_alert_or_month_will_not_write_or_open_still_lands_its_records() {
    let root = scratch("window-faults");
    let on = regular();
    let month = on.year_month().unwrap();
    let minutes = full_day(on, &[]);
    write_month(&root, &index(), Timeframe::MINUTE_1, &minutes);
    std::fs::create_dir_all(month_path(&root, &index(), Timeframe::MINUTE_5, month)).unwrap();
    std::fs::create_dir_all(root.join("audit/alerts")).unwrap();
    let rows: Vec<RawRow> = minutes.iter().map(row_of).collect();
    let changed = answer_shape("{\"status\":\"x\",\"data\":{\"candles\":[]},\"meta\":1}");
    let series = index();
    let window = WindowInput {
        series: &series,
        rung: Timeframe::MINUTE_1,
        rows: &rows,
        asked: whole_day(on),
        receipt: 1,
        refused: &[],
        rows_refused: 0,
        first_day_bar: None,
        shape: &changed,
    };
    let run = 133;
    let writer = SpoolWriter::new(&root, run, Mode::ZerodhaMerge, 1);
    let wrote = verify_window(&root, &window, &GapLedger::default(), &writer).unwrap();
    assert_eq!(wrote.records, 8);
    let five = spool_of(&root, run, Segment::Index, Timeframe::MINUTE_5, month);
    assert_eq!(
        said(record_on(&five, "NSE-INDEX-ZZIDX", on)),
        (Code::StoreUnreadable, "month_refused")
    );
    let minute_records = spool_of(&root, run, Segment::Index, Timeframe::MINUTE_1, month);
    let minute = record_on(&minute_records, "NSE-INDEX-ZZIDX", on);
    assert_eq!(minute.facts.code, Code::Match);
    assert!(
        minute
            .details
            .iter()
            .any(|d| d.kind == words::DETAIL_VENDOR_SHAPE)
    );
    assert!(
        root.join("audit/alerts").is_dir(),
        "the planted folder is left as it is"
    );

    let root = scratch("window-before-1970");
    let before = WindowInput {
        rows: &[],
        asked: (-3 * 86_400, -2 * 86_400),
        ..window
    };
    let writer = SpoolWriter::new(&root, 134, Mode::ZerodhaMerge, 1);
    let wrote = verify_window(&root, &before, &GapLedger::default(), &writer).unwrap();
    assert_eq!(wrote, spool::Appended::default());
    assert_eq!(crate::alerts::window(&root).unwrap().alerts.len(), 0);
}

// --------------------------------------------------------- kept answers --

/// Every kept answer of a month that will not read is named and never read
/// as an answer, and the rest are read (extremes, one pack of five
/// receipts): a refused answer's head is `refused_answer` for its day; two
/// answers whose shape changed put one alert, not two; a body that no longer
/// decodes is a fault and `body_unreadable`; a receipt whose refused rows
/// fail their CRC is a fault, its answer read without them (so the
/// receipt's count differs, `quality_differs`). A receipt damaged before the
/// last refuses the month's kept answers whole, by name.
#[test]
fn every_kept_answer_that_will_not_read_is_named() {
    use crate::vendor::Granularity;
    let root = scratch("kept-faults");
    let month = regular().year_month().unwrap();
    let (monday, on, wednesday, thursday, friday) = (
        day(2024, 4, 22),
        regular(),
        day(2024, 4, 24),
        day(2024, 4, 25),
        day(2024, 4, 26),
    );
    let key = zzidx();
    let at_1min = (Timeframe::MINUTE_1, Granularity::Minute1);
    let refused_head = response_keep::RefusedAnswer::of(200, "{\"status\":\"error\"}");
    keep_offered(
        &root,
        &key,
        at_1min,
        (monday, monday),
        response_keep::Offered::Refused(&refused_head),
        (&[], &[]),
    );
    for at in [on, wednesday] {
        let bars = full_day(at, &[]);
        let body = kite_body(&bars).replacen("{\"status\"", "{\"meta\":1,\"status\"", 1);
        let answered = response_keep::Answered::of(crate::fetch::RawWindow::default(), body, 200);
        keep_offered(
            &root,
            &key,
            at_1min,
            (at, at),
            response_keep::Offered::Answer(&answered),
            (&bars, &[]),
        );
    }
    let garbled = response_keep::Answered::of(
        crate::fetch::RawWindow::default(),
        "not json".to_owned(),
        200,
    );
    keep_offered(
        &root,
        &key,
        at_1min,
        (thursday, thursday),
        response_keep::Offered::Answer(&garbled),
        (&[], &[]),
    );
    let bars = full_day(friday, &[]);
    let body =
        response_keep::Answered::of(crate::fetch::RawWindow::default(), kite_body(&bars), 200);
    keep_offered(
        &root,
        &key,
        at_1min,
        (friday, friday),
        response_keep::Offered::Answer(&body),
        (
            &bars,
            &[response_keep::RefusedRow {
                local_second: utc_s(friday, 600) + 19_800,
                cause: RowCause::Price,
            }],
        ),
    );
    let pack = response_keep::Pack {
        vendor: Vendor::Zerodha,
        key: &key,
        timeframe: Timeframe::MINUTE_1,
    };
    let names = response_keep::names(&response_keep::pack_dir(&root, pack).unwrap(), month, 0);
    flip(&names.refusals, 0);

    let mut next = 0;
    let (answers, faults) = month_answers(&root, &index(), Timeframe::MINUTE_1, month, &mut next);
    assert_eq!(next, 5, "every receipt numbered");
    assert_eq!(faults.len(), 2, "{faults:?}");
    assert!(
        faults[0].starts_with("NSE-INDEX-ZZIDX 2024-04 1min receipt 3: "),
        "{faults:?}"
    );
    assert!(
        faults[1].starts_with("NSE-INDEX-ZZIDX 2024-04 1min receipt 4 refused rows: "),
        "{faults:?}"
    );
    let only = |at: Day| {
        let held = &answers[&at.days_from_epoch()];
        assert_eq!(held.len(), 1, "{at}");
        held[0].clone()
    };
    let refused = only(monday);
    assert_eq!(
        (refused.kept, refused.not_kept, refused.receipt),
        (false, reason_of("refused_answer"), 1)
    );
    for (at, receipt) in [(on, 2), (wednesday, 3)] {
        let shaped = only(at);
        assert_eq!(
            (shaped.kept, shaped.receipt, shaped.shape.unknown),
            (true, receipt, 1)
        );
        assert_eq!(shaped.bars.len(), 375);
    }
    let unread = only(thursday);
    assert_eq!(
        (unread.kept, unread.not_kept),
        (false, reason_of("body_unreadable"))
    );
    let counted = only(friday);
    assert!(counted.kept);
    assert_eq!(counted.bars.len(), 375, "the refused row is not taken in");
    assert_eq!(counted.quality, Some((1, 0)));
    let alerts = crate::alerts::window(&root).unwrap();
    assert_eq!(
        alerts
            .alerts
            .iter()
            .filter(|a| a.record.kind == VENDOR_SHAPE_CHANGED)
            .count(),
        1,
        "one alert for the month's two changed answers"
    );

    let root = scratch("kept-receipts");
    let minutes = full_day(on, &[]);
    for _ in 0..2 {
        keep_answer(
            &root,
            on,
            kite_body(&minutes),
            &minutes,
            375,
            response_keep::Door::FromWindow,
        );
    }
    let names = response_keep::names(&response_keep::pack_dir(&root, pack).unwrap(), month, 0);
    flip(&names.receipts, 10);
    let mut next = 0;
    let (answers, faults) = month_answers(&root, &index(), Timeframe::MINUTE_1, month, &mut next);
    assert!(answers.is_empty());
    assert_eq!(next, 0);
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert!(
        faults[0].starts_with("NSE-INDEX-ZZIDX 2024-04 1min kept answers: "),
        "{faults:?}"
    );
}

// -------------------------------------------------------------- the merge --

/// A spool tree that will not list (a file at `verify/spool`) refuses the
/// merge by the spool's words and lets the lock go, with or without a
/// progress file; with one, its record ends `refused` by those words.
#[test]
fn a_spool_tree_that_will_not_list_refuses_the_merge_and_its_progress() {
    let root = scratch("merge-unlisted");
    std::fs::create_dir_all(root.join("verify")).unwrap();
    std::fs::write(root.join(spool::SPOOL_DIR), b"not a folder").unwrap();
    let refused = merge_all(&root, None, None).unwrap_err();
    assert!(matches!(refused, MergeError::Spool(_)), "{refused}");
    assert!(refused.to_string().contains(spool::SPOOL_DIR), "{refused}");
    let progress = crate::gdfl_run::ProgressFile::of_record(
        root.join(PROGRESS_MERGE),
        progress_record("verify-merge", 1, 135, 1, None),
    );
    let again = merge_observed(&root, None, None, Some(&progress)).unwrap_err();
    assert_eq!(
        again.to_string(),
        refused.to_string(),
        "the lock was let go"
    );
    let record = progress.record();
    assert_eq!(record.state, crate::gdfl_run::RunState::Refused.code());
    assert_eq!(
        record
            .refusals
            .last()
            .map(|r| r.why.contains(spool::SPOOL_DIR)),
        Some(true)
    );
}

/// A run whose live lock will not open (a folder at its name) keeps its
/// spools waiting, never read as free, and is named each time it is asked:
/// in each of the merge's two passes that would take its spools, and once
/// more when the merge would say whether it is still spooling.
#[test]
fn a_live_lock_that_will_not_open_keeps_its_run_waiting_and_named() {
    let root = scratch("merge-live-folder");
    let writer = SpoolWriter::new(&root, 77, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(regular(), "NSE-INDEX-ZZLIVE", Code::Match, 1)],
        )
        .unwrap();
    drop(writer);
    let lock = spool::live_lock_path(&root, 77);
    std::fs::remove_file(&lock).unwrap();
    std::fs::create_dir(&lock).unwrap();
    let merged = merge_all(&root, None, None).unwrap();
    assert_eq!(
        (merged.spools_merged, merged.generations, merged.waiting),
        (0, 0, 1)
    );
    assert!(merged.live.is_empty(), "not known to be live either");
    assert_eq!(merged.faults.len(), 3, "{:?}", merged.faults);
    for (pass, fault) in merged.faults[..2].iter().enumerate() {
        assert!(
            fault.starts_with("run 77's live lock would not open, so its spools wait: "),
            "pass {pass}: {:?}",
            merged.faults
        );
    }
    assert!(
        merged.faults[2].starts_with(
            "run 77's live lock would not open, so whether it is still spooling is not known: "
        ),
        "{:?}",
        merged.faults
    );
}

/// The merge's own edges, each by name: a merge log damaged before its last
/// record refuses its spool `merge_log_damaged` with an alert of kind 10, nothing merged; a
/// spool's torn tail is ignored and named by offset and length while its
/// whole records merge; a spool holding two exchanges' records of one day
/// commits one generation per exchange; and a spool of a header and no
/// record (out of the box: a run killed before its first append) merges to
/// nothing, its log never written, and is removed.
#[test]
fn the_merge_names_a_damaged_log_and_a_torn_tail_and_splits_exchanges() {
    let on = regular();
    let month = on.year_month().unwrap();
    let id = |run: u64| SpoolId {
        run,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month,
    };

    let root = scratch("merge-log-damaged");
    let writer = SpoolWriter::new(&root, 61, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-ZZLOG", Code::Match, 1)],
        )
        .unwrap();
    drop(writer);
    let log_path = id(61).log_path(&root);
    let mut log = spool::read_log(&log_path).unwrap();
    for record in [
        LogRecord {
            day: 1,
            generation: 0,
            state: LogState::Seen,
            exchange: 0,
        },
        LogRecord {
            day: on.days_from_epoch(),
            generation: 0,
            state: LogState::Intent,
            exchange: verdict::exchange_code(Exchange::Nse),
        },
    ] {
        spool::append_log(&log_path, &mut log, record).unwrap();
    }
    flip(&log_path, 3);
    let merged = merge_all(&root, None, None).unwrap();
    assert_eq!(merged.generations, 0);
    assert_eq!(merged.refused.len(), 1, "{merged:?}");
    assert_eq!(merged.refused[0].0, id(61).path(&root));
    assert!(
        merged.refused[0].1.starts_with("merge_log_damaged"),
        "{:?}",
        merged.refused
    );
    assert_eq!(
        crate::alerts::window(&root)
            .unwrap()
            .alerts
            .iter()
            .filter(|a| a.record.kind == 10)
            .count(),
        1
    );

    let root = scratch("merge-torn");
    let writer = SpoolWriter::new(&root, 62, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-ZZTORN", Code::Match, 1)],
        )
        .unwrap();
    drop(writer);
    let path = id(62).path(&root);
    append_raw(&path, &[0xAB; 70]);
    let merged = merge_all(&root, None, None).unwrap();
    assert_eq!((merged.generations, merged.records), (1, 1));
    assert_eq!(merged.torn, [(path, 224, 70)]);

    let root = scratch("merge-exchanges");
    let writer = SpoolWriter::new(&root, 63, Mode::ZerodhaMerge, 1);
    let on_bse = SpoolRecord {
        exchange: Exchange::Bse,
        ..spooled(on, "BSE-INDEX-ZZTWIN", Code::VendorHole, 1)
    };
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-ZZTWIN", Code::Match, 1), on_bse],
        )
        .unwrap();
    drop(writer);
    let merged = merge_all(&root, None, None).unwrap();
    assert_eq!(
        (merged.spools_merged, merged.generations, merged.records),
        (1, 2, 2)
    );
    assert_eq!(
        the_record(
            &root,
            Segment::Index,
            Timeframe::MINUTE_1,
            on,
            "NSE-INDEX-ZZTWIN"
        )
        .facts
        .code,
        Code::Match
    );

    let root = scratch("merge-empty");
    let path = id(64).path(&root);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, spool::encode_header(&id(64), Mode::ZerodhaMerge, 1)).unwrap();
    let merged = merge_all(&root, None, None).unwrap();
    assert_eq!(
        (merged.spools_merged, merged.generations, merged.waiting),
        (1, 0, 0)
    );
    assert!(merged.refused.is_empty(), "{merged:?}");
    assert!(merged.not_removed.is_empty(), "{merged:?}");
    assert!(!path.exists(), "the merged spool is removed");
    assert!(!id(64).log_path(&root).exists(), "and no log was written");
}

/// A merge that cannot write its log (the spool's folder read-only, its log
/// not yet made) refuses the spool `verdict_refused`, never as damage, and
/// leaves it waiting; one that merged a spool but cannot remove its log (the
/// log made before the folder became read-only) names the log it left and
/// keeps the spool beside it. Run where the permission bits bind (D-0995):
/// as root a read-only folder takes both.
#[test]
fn a_merge_that_cannot_write_its_log_or_remove_it_says_so() {
    crate::support::where_permission_binds(
        "zerodha_verify::cov_store_tests::a_merge_that_cannot_write_its_log_or_remove_it_says_so",
        a_merge_that_cannot_write_its_log_or_remove_it_says_so_body,
    );
}

fn a_merge_that_cannot_write_its_log_or_remove_it_says_so_body() {
    use std::os::unix::fs::PermissionsExt as _;
    let on = regular();
    let id = |run: u64| SpoolId {
        run,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month: on.year_month().unwrap(),
    };
    let spooled_once = |root: &Path, run: u64| {
        let writer = SpoolWriter::new(root, run, Mode::ZerodhaMerge, 1);
        writer
            .append(
                Segment::Index,
                Timeframe::MINUTE_1,
                &[spooled(on, "NSE-INDEX-ZZRO", Code::Match, 1)],
            )
            .unwrap();
    };
    let mode = |path: &Path, bits: u32| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(bits)).unwrap();
    };

    let root = scratch("merge-log-unwritable");
    spooled_once(&root, 65);
    let folder = id(65).path(&root).parent().unwrap().to_path_buf();
    mode(&folder, 0o555);
    let merged = merge_all(&root, None, None).unwrap();
    mode(&folder, 0o755);
    assert_eq!((merged.generations, merged.waiting), (0, 1));
    assert_eq!(merged.refused.len(), 1, "{merged:?}");
    assert!(
        merged.refused[0].1.contains("Permission denied"),
        "{:?}",
        merged.refused
    );
    assert!(!id(65).log_path(&root).exists());
    assert_eq!(
        crate::alerts::window(&root)
            .unwrap()
            .alerts
            .iter()
            .filter(|a| a.record.kind == 11)
            .count(),
        1,
        "verdict_refused, never spool_damaged"
    );

    let root = scratch("merge-log-unremovable");
    spooled_once(&root, 66);
    let log = id(66).log_path(&root);
    std::fs::write(&log, b"").unwrap();
    let folder = log.parent().unwrap().to_path_buf();
    mode(&folder, 0o555);
    let merged = merge_all(&root, None, None).unwrap();
    mode(&folder, 0o755);
    assert_eq!((merged.spools_merged, merged.generations), (1, 1));
    assert_eq!(merged.not_removed.len(), 1, "{merged:?}");
    assert_eq!(merged.not_removed[0].0, log);
    assert!(
        merged.not_removed[0].1.contains("Permission denied"),
        "{:?}",
        merged.not_removed
    );
    assert!(
        id(66).path(&root).exists(),
        "the spool stays beside its log"
    );
}

/// A commit interrupted after its intent is proved only by its own run's
/// image, walking back at most `MAX_WALK_BACK` images, and is otherwise
/// merged again: the day's newer generation written by ANOTHER run is no
/// proof; an image that will not read is no proof (and the commit after it
/// is refused by the verdict store, by name); a run's own image further back
/// than the walk reaches is no proof (the extreme of the walk); and an intent
/// that covered fewer records than the spool now holds for the day is merged
/// again whatever the year index says.
#[test]
fn an_interrupted_commit_is_proved_only_by_its_own_run() {
    fn series(root: &Path) -> verdict::Series<'_> {
        verdict::Series {
            root,
            feed: Vendor::Zerodha,
            exchange: Exchange::Nse,
            segment: Segment::Index,
            timeframe: Timeframe::MINUTE_1,
        }
    }
    let on = regular();
    let nse = verdict::exchange_code(Exchange::Nse);
    let id = |run: u64| SpoolId {
        run,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month: on.year_month().unwrap(),
    };
    // `run`'s spool of `keys` on the day, and a log saying a merge read
    // `seen` records and wrote the day's intent over generation 0.
    let interrupted = |root: &Path, run: u64, keys: &[&str], seen: u32| {
        let writer = SpoolWriter::new(root, run, Mode::ZerodhaMerge, 1);
        let records: Vec<SpoolRecord> = keys
            .iter()
            .map(|key| spooled(on, key, Code::Match, 1))
            .collect();
        writer
            .append(Segment::Index, Timeframe::MINUTE_1, &records)
            .unwrap();
        drop(writer);
        let path = id(run).log_path(root);
        let mut log = spool::read_log(&path).unwrap();
        for record in [
            LogRecord {
                day: seen,
                generation: 0,
                state: LogState::Seen,
                exchange: 0,
            },
            LogRecord {
                day: on.days_from_epoch(),
                generation: 0,
                state: LogState::Intent,
                exchange: nse,
            },
        ] {
            spool::append_log(&path, &mut log, record).unwrap();
        }
    };
    let commit_by = |root: &Path, run: u64, key: &str| {
        let generation = Generation {
            day: store_day(on).unwrap(),
            run,
            written_at: 1,
            verifier: VERIFIER,
            mode: Mode::ZerodhaMerge,
            rule_set: 0,
            origin: [0; 16],
            day_block: DayBlock::of(Code::Match),
            day_details: Vec::new(),
            entries: vec![Entry {
                key: Key::instrument(key).unwrap(),
                facts: Facts::of(Code::VendorHole),
                details: Vec::new(),
            }],
            carry: Carry::Others,
        };
        verdict::commit(&series(root), &generation).unwrap();
    };

    let root = scratch("intent-other-run");
    interrupted(&root, 33, &["NSE-INDEX-ZZOWN"], 1);
    commit_by(&root, 99, "NSE-INDEX-ZZOTHER");
    let merged = merge_all(&root, None, None).unwrap();
    assert_eq!((merged.days_recovered, merged.generations), (0, 1));
    assert_eq!(
        year_generation(&root, Segment::Index, Timeframe::MINUTE_1, on),
        2
    );
    let own = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZOWN",
    );
    assert_eq!((own.run, own.facts.code), (33, Code::Match));
    let carried = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZOTHER",
    );
    assert_eq!(carried.run, 99, "the other run's record carried");

    let root = scratch("intent-image-unread");
    interrupted(&root, 33, &["NSE-INDEX-ZZOWN"], 1);
    commit_by(&root, 99, "NSE-INDEX-ZZOTHER");
    let image = series(&root).image_path(store_day(on).unwrap(), 1);
    std::fs::write(&image, b"an image that no longer reads").unwrap();
    let merged = merge_all(&root, None, None).unwrap();
    assert_eq!((merged.days_recovered, merged.generations), (0, 0));
    assert_eq!(merged.refused.len(), 1, "{merged:?}");
    assert!(
        !merged.refused[0].1.starts_with("spool_damaged"),
        "a refused commit, never damage: {:?}",
        merged.refused
    );
    assert_eq!(merged.waiting, 1);

    let root = scratch("intent-walk-cap");
    interrupted(&root, 33, &["NSE-INDEX-ZZOWN"], 1);
    commit_by(&root, 33, "NSE-INDEX-ZZOWN");
    for _ in 0..MAX_WALK_BACK {
        commit_by(&root, 99, "NSE-INDEX-ZZOTHER");
    }
    let past = MAX_WALK_BACK + 1;
    assert_eq!(
        year_generation(&root, Segment::Index, Timeframe::MINUTE_1, on),
        past
    );
    let merged = merge_all(&root, None, None).unwrap();
    assert_eq!((merged.days_recovered, merged.generations), (0, 1));
    assert_eq!(
        year_generation(&root, Segment::Index, Timeframe::MINUTE_1, on),
        past + 1
    );

    let root = scratch("intent-short");
    interrupted(&root, 34, &["NSE-INDEX-ZZA", "NSE-INDEX-ZZB"], 1);
    commit_by(&root, 34, "NSE-INDEX-ZZA");
    let merged = merge_all(&root, None, None).unwrap();
    assert_eq!(
        (merged.days_recovered, merged.generations, merged.records),
        (0, 1, 2),
        "the intent covered one record of two: merged again, never recovered"
    );
    assert_eq!(
        year_generation(&root, Segment::Index, Timeframe::MINUTE_1, on),
        2
    );
}

/// The merge lock's answers and the merge's last listing, each by a direct
/// call (the seams; their refusals need a file system that refuses `flock`,
/// or a spool tree that changes between a merge's listings): the host's
/// refusal of the lock is `merge_lock_refused` by its words, a lock held
/// elsewhere is `run_busy` naming the lock, and a last listing that failed
/// is a fault by name with the waiting count left unknown (0), never a
/// silent 0.
#[test]
fn the_merge_lock_and_the_last_listing_say_their_refusals() {
    let root = scratch("merge-seams");
    let path = root.join(spool::MERGE_LOCK);
    let Err(refused) = lock_answer(
        &path,
        Err(std::fs::TryLockError::Error(std::io::Error::other(
            "an invented host refusal",
        ))),
    ) else {
        panic!("the host's refusal answered a lock");
    };
    assert_eq!(
        refused.to_string(),
        "merge_lock_refused: an invented host refusal"
    );
    let Err(busy) = lock_answer(&path, Err(std::fs::TryLockError::WouldBlock)) else {
        panic!("a lock held elsewhere answered a lock");
    };
    assert!(matches!(busy, MergeError::RunBusy { .. }), "{busy}");
    assert!(busy.to_string().contains(spool::MERGE_LOCK), "{busy}");

    let mut outcome = MergeOutcome::default();
    note_left(
        &root,
        Err(SpoolError::Io {
            path: root.join(spool::SPOOL_DIR),
            action: "list",
            source: std::io::Error::other("an invented refusal"),
        }),
        &mut outcome,
    );
    assert_eq!(outcome.waiting, 0);
    assert!(outcome.live.is_empty());
    assert_eq!(outcome.faults.len(), 1);
    assert!(
        outcome.faults[0].starts_with(
            "the spool tree could not be listed after the merge, so the spools still waiting are not known: "
        ),
        "{:?}",
        outcome.faults
    );
    assert!(outcome.faults[0].contains("an invented refusal"));

    note_left(&root, Ok(spool::Waiting::default()), &mut outcome);
    assert_eq!((outcome.waiting, outcome.faults.len()), (0, 1));
}
