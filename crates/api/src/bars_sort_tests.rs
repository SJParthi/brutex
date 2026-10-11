//! A value page from the sort indexes is the brute-force sort of the same
//! kept rows, page for page, and reads no bar outside the page. D-3349,
//! WP-39; DB-N14, DB-N15.
//!
//! Every bar is invented: one-minute stamps in 2031, at prices no market
//! printed, written through the store's own writer — which keeps each
//! month's `.srt` as it appends, one run per append.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::PathBuf;

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::sort_index::Column;

use super::super::{
    Cancel, Filter, SeriesKey, SortKey, Window, WindowQuery, window, window_cancellable,
};

const MICROS: i64 = 1_000_000;
const CE: &str = "2031-03-27-1000000-CE";
const PE: &str = "2031-03-27-1000000-PE";

/// A store root that removes itself.
struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-sort-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The three series every fixture holds: two option contracts on one
/// underlying, written at the SAME instants so ties across series are real,
/// and the index, whose open interest is no number and whose volume is none.
fn keys() -> [SeriesKey; 3] {
    [
        SeriesKey::new("NSE", "FNO", "NIFTY", Contract::parse(CE)),
        SeriesKey::new("NSE", "FNO", "NIFTY", Contract::parse(PE)),
        SeriesKey::new("NSE", "INDEX", "NIFTY", None),
    ]
}

fn months() -> [YearMonth; 3] {
    [
        YearMonth::new(2031, 1).unwrap(),
        YearMonth::new(2031, 2).unwrap(),
        YearMonth::new(2031, 3).unwrap(),
    ]
}

fn path(key: &SeriesKey, month: YearMonth, file: FileKind) -> StorePath<'_> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: &key.exchange,
        segment: &key.segment,
        symbol: &key.symbol,
        contract: key.contract,
        timeframe: Timeframe::MINUTE_1,
        month,
        file,
    })
    .unwrap()
}

/// Bar `i` of a month for series `s`: three sessions of 120 minutes from
/// 09:15 IST on the 2nd, 3rd and 4th; closes that repeat often, volumes that
/// tie, open interest that is null on every sixth bar.
fn bar(month: YearMonth, s: usize, i: i64) -> Bar {
    let day = 1 + i / 120;
    let ts = month.ist_bounds_micros().0 + (day * 86_400 + 555 * 60 + (i % 120) * 60) * MICROS;
    let seed = i64::try_from(s).unwrap() * 7 + i64::from(month.month());
    let close = 8_880_000 + ((i * 37 + seed * 11) % 53) * 5;
    let index = s == 2;
    Bar {
        ts_micros: ts,
        open: close - (i % 3),
        high: close + 20 + (i % 7),
        low: close - 20 - (i % 5),
        close,
        volume: if index { 0 } else { (i * 13 + seed) % 7 },
        open_interest: if index || i % 6 == 0 {
            OI_NULL
        } else {
            40_000 + (i * 5 + seed) % 9
        },
    }
}

/// Bars per month.
const PER_MONTH: i64 = 360;

/// Writes every series' months through the store's writer, in three
/// appends a month — three sort runs a month.
fn write(root: &Root) {
    for key in &keys() {
        let s = keys().iter().position(|k| k == key).unwrap();
        for month in months() {
            let symbol_id =
                u32::try_from(brutex_core::universe::fnv1a(&key.symbol) & u64::from(u32::MAX))
                    .unwrap();
            let mut writer = store::file::BarFile::open_or_create(
                &root.0,
                path(key, month, FileKind::Bars),
                symbol_id,
            )
            .unwrap();
            let rows: Vec<Bar> = (0..PER_MONTH).map(|i| bar(month, s, i)).collect();
            for chunk in rows.chunks(130) {
                writer.append(chunk).unwrap();
            }
        }
    }
}

/// Every bar of `series`, with its series' place in the request.
fn every(series: &[usize]) -> Vec<(u32, Bar)> {
    let mut out = Vec::new();
    for (place, &s) in series.iter().enumerate() {
        for month in months() {
            for i in 0..PER_MONTH {
                out.push((u32::try_from(place).unwrap(), bar(month, s, i)));
            }
        }
    }
    out
}

/// The brute-force order of `rows` by `column`: nulls last in both
/// directions, then the value, then the instant, then the series.
fn brute(rows: &[(u32, Bar)], series: &[usize], column: Column, desc: bool) -> Vec<(i64, u32)> {
    let mut keyed: Vec<(bool, i64, i64, u32)> = rows
        .iter()
        .map(|(place, bar)| {
            let v = column.of(bar);
            let derivative = series[usize::try_from(*place).unwrap()] != 2;
            let null = column == Column::OpenInterest && (!derivative || v == OI_NULL);
            let key = if null {
                0
            } else if desc {
                -v
            } else {
                v
            };
            (null, key, bar.ts_micros, *place)
        })
        .collect();
    keyed.sort_unstable();
    keyed.into_iter().map(|(_, _, ts, s)| (ts, s)).collect()
}

fn sort_of(column: Column) -> SortKey {
    match column {
        Column::Open => SortKey::Open,
        Column::High => SortKey::High,
        Column::Low => SortKey::Low,
        Column::Close => SortKey::Close,
        Column::Volume => SortKey::Volume,
        Column::OpenInterest => SortKey::OpenInterest,
    }
}

fn query(
    series: &[usize],
    column: Column,
    desc: bool,
    offset: usize,
    filter: Filter,
) -> WindowQuery {
    WindowQuery {
        vendor: Vendor::Gdfl,
        timeframe: Timeframe::MINUTE_1,
        series: series.iter().map(|&s| keys()[s].clone()).collect(),
        from: months()[0],
        to: months()[2],
        sort: sort_of(column),
        desc,
        offset: Some(offset),
        limit: 37,
        want_extremes: false,
        at: None,
        filter,
        lookback: true,
        coverage: false,
    }
}

fn served(answer: &Window) -> Vec<(i64, u32)> {
    answer
        .bars
        .iter()
        .map(|row| (row.bar.ts_micros, u32::try_from(row.series).unwrap()))
        .collect()
}

/// Checks every column, both directions and the first, a middle, the last
/// and a past-the-end page of `series` against the brute force, keeping only
/// the rows `keep` keeps; returns the rows the answers read.
fn check(root: &Root, series: &[usize], filter: Filter, keep: &dyn Fn(&Bar) -> bool) -> u64 {
    let rows: Vec<(u32, Bar)> = every(series).into_iter().filter(|(_, b)| keep(b)).collect();
    let total = rows.len();
    let mut read = 0;
    for column in store::sort_index::Column::ALL {
        for desc in [false, true] {
            let truth = brute(&rows, series, column, desc);
            for offset in [0, total / 2 + 3, total.saturating_sub(37), total + 5] {
                let answer = window(&root.0, &query(series, column, desc, offset, filter))
                    .unwrap_or_else(|why| panic!("{series:?} {column:?} {desc}: {why}"));
                assert_eq!(answer.total, total as u64);
                let want: Vec<(i64, u32)> = truth.iter().skip(offset).take(37).copied().collect();
                assert_eq!(
                    served(&answer),
                    want,
                    "series {series:?} column {} desc {desc} offset {offset}",
                    column.as_str()
                );
                assert!(answer.faults.is_empty(), "{:?}", answer.faults);
                read += answer.scan_rows;
            }
        }
    }
    read
}

/// **EVERY VALUE PAGE IS THE BRUTE-FORCE SORT, AND NO BAR OUTSIDE THE PAGE
/// IS READ.** One series, two series tied at every instant, and the index
/// (whose open interest is all null): six columns, both directions, the
/// first, middle, last and past-the-end pages. `scan_rows` — the bars read
/// beyond the page — is zero for every one; the stream this replaces read
/// all of them (DB-N14).
#[test]
fn a_value_page_from_the_indexes_is_the_brute_force_page_and_reads_no_other_bar() {
    let root = Root::new("brute");
    write(&root);
    for series in [&[0][..], &[0, 1], &[2], &[1, 2, 0]] {
        let read = check(&root, series, Filter::default(), &|_| true);
        assert_eq!(read, 0, "{series:?}: every row came from an index");
    }
    let answer = window(
        &root.0,
        &query(&[0], Column::Close, true, 0, Filter::default()),
    )
    .unwrap();
    let report = answer.sort_index.expect("a value page reports its index");
    assert_eq!(report.indexed_rows, 3 * 360);
    assert_eq!(
        report.indexed_runs, 9,
        "three appends a month, three months"
    );
    assert_eq!(report.scanned_rows, 0);
    assert!(report.build.is_empty());
}

/// **A DAY BOUND IS SERVED FROM THE INDEX, NOT BY READING THE EDGE MONTHS.**
/// It cuts runs in the edge months (two a series here); their kept rows are
/// the runs' own entries stamped inside the bound, so the page is still the
/// brute force at every offset and no bar is read to order it (D-3349). Before,
/// those rows were read, and a deep page over them refused `sort_too_deep`.
#[test]
fn a_day_bound_is_served_from_the_cut_runs_entries() {
    let root = Root::new("days");
    write(&root);
    let from = months()[0].ist_bounds_micros().0 + 2 * 86_400 * MICROS;
    let to = months()[2].ist_bounds_micros().0 + 3 * 86_400 * MICROS;
    let filter = Filter {
        from: Some(from),
        to: Some(to),
        tod: None,
    };
    let inside = |b: &Bar| b.ts_micros >= from && b.ts_micros < to;
    let read = check(&root, &[0, 1], filter, &inside);
    assert_eq!(read, 0, "no bar is read to order a day-bounded page");
    check(&root, &[2, 0], filter, &inside);
    let answer = window(&root.0, &query(&[0], Column::Volume, false, 0, filter)).unwrap();
    let report = answer.sort_index.unwrap();
    assert_eq!(report.cut_runs, 2, "{report:?}");
    assert_eq!(report.scanned_rows, 0, "{report:?}");
    assert_eq!(report.scanned_months, 0, "{report:?}");
    assert!(report.build.is_empty());
    assert_eq!(report.indexed_rows, answer.total);

    // THE EXTREMES of the kept rows: the whole runs' summaries and the cut
    // runs' kept rows folded once, then held.
    let mut asked = query(&[0, 1], Column::Close, true, 0, filter);
    asked.want_extremes = true;
    let first = window(&root.0, &asked).unwrap();
    assert!(first.scan_rows > 0, "the cut stretches are folded once");
    let again = window(&root.0, &asked).unwrap();
    assert_eq!(again.scan_rows, 0, "and held after");
    assert_eq!(again.extremes, first.extremes);
    let rows: Vec<(u32, Bar)> = every(&[0, 1])
        .into_iter()
        .filter(|(_, b)| inside(b))
        .collect();
    let got = first.extremes.expect("extremes");
    let close_max = rows
        .iter()
        .map(|(s, b)| (b.close, std::cmp::Reverse((b.ts_micros, *s))))
        .max()
        .unwrap();
    let held = got.close_max.unwrap();
    assert_eq!(
        (held.v, held.t, u32::try_from(held.series).unwrap()),
        (close_max.0, (close_max.1).0.0, (close_max.1).0.1)
    );
    let range_max = rows
        .iter()
        .map(|(s, b)| (b.high - b.low, std::cmp::Reverse((b.ts_micros, *s))))
        .max()
        .unwrap();
    let held = got.range_max.unwrap();
    assert_eq!(
        (held.v, held.t, u32::try_from(held.series).unwrap()),
        (range_max.0, (range_max.1).0.0, (range_max.1).0.1)
    );
    // The time-ordered page's extremes are the same.
    asked.sort = SortKey::Ts;
    let by_time = window(&root.0, &asked).unwrap();
    assert_eq!(by_time.extremes, first.extremes);
    assert_eq!(by_time.scan_rows, 0, "held from the value page's fold");
}

/// A cut run whose block fails its checksum is not served from its entries:
/// its kept rows are read, the page is still the brute force, and the answer
/// says why. Nothing is served from a damaged index.
#[test]
fn a_cut_run_whose_entries_would_not_serve_is_read_and_named() {
    let root = Root::new("days-damaged");
    write(&root);
    let [ce, ..] = keys();
    let jan = path(&ce, months()[0], FileKind::SortIndex).to_path_buf(&root.0);
    let shape = store::sort_index::Shape::of(
        months()[0],
        60,
        u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)).unwrap(),
    );
    let index = store::sort_index::SortIndex::open(&jan, &shape).unwrap();
    let run = index.runs()[0];
    let at = run.offset + run.column(Column::Close).entries_at() + 3;
    drop(index);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&jan)
        .unwrap();
    let mut byte = [0u8; 1];
    std::os::unix::fs::FileExt::read_exact_at(&file, &mut byte, at).unwrap();
    byte[0] ^= 0x20;
    std::os::unix::fs::FileExt::write_all_at(&file, &byte, at).unwrap();
    drop(file);
    let from = months()[0].ist_bounds_micros().0 + 2 * 86_400 * MICROS;
    let filter = Filter {
        from: Some(from),
        to: None,
        tod: None,
    };
    for desc in [false, true] {
        let rows: Vec<(u32, Bar)> = every(&[0])
            .into_iter()
            .filter(|(_, b)| b.ts_micros >= from)
            .collect();
        let truth = brute(&rows, &[0], Column::Close, desc);
        let answer = window(&root.0, &query(&[0], Column::Close, desc, 0, filter)).unwrap();
        let want: Vec<(i64, u32)> = truth.iter().take(37).copied().collect();
        assert_eq!(served(&answer), want);
        let report = answer.sort_index.unwrap();
        assert_eq!(
            report.scanned_rows, 10,
            "rows 120..130 of January: {report:?}"
        );
        assert!(
            report.scanned_why[0].contains("would not serve")
                && report.scanned_why[0].contains("checksum"),
            "{report:?}"
        );
        assert_eq!(
            report.build,
            ["cli srt-build gdfl NSE/FNO/NIFTY/2031-03-27-1000000-CE 1min 2031-01 2031-01"],
            "a rebuild writes the damaged run again"
        );
    }
    // Another column of the same run is whole and still served.
    let answer = window(&root.0, &query(&[0], Column::Open, false, 0, filter)).unwrap();
    assert_eq!(answer.sort_index.unwrap().scanned_rows, 0);
}

/// A time-of-day bound keeps windows no run is cut to: every kept row is
/// read, named, and the page is the brute force.
#[test]
fn a_time_of_day_bound_reads_its_kept_rows_and_says_why() {
    let root = Root::new("tod");
    write(&root);
    let filter = Filter {
        from: None,
        to: None,
        tod: Some((9 * 3_600 + 40 * 60, 10 * 3_600 + 5 * 60)),
    };
    let keep = |b: &Bar| {
        let tod = (b.ts_micros / MICROS + 19_800).rem_euclid(86_400);
        (9 * 3_600 + 40 * 60..=10 * 3_600 + 5 * 60).contains(&tod)
    };
    check(&root, &[0, 2], filter, &keep);
    let answer = window(&root.0, &query(&[0], Column::High, true, 0, filter)).unwrap();
    let report = answer.sort_index.unwrap();
    assert_eq!(report.indexed_rows, 0);
    assert_eq!(report.scanned_rows, answer.total);
    assert!(report.scanned_why[0].contains("time-of-day"), "{report:?}");
}

/// **EVERY PAGE OF A READ ORDER WITHIN `MAX_READ_HELD` IS SERVED, AND PAST IT
/// A PAGE NEAR EITHER END.** Under a time-of-day bound every kept row is read.
/// Up to `MAX_READ_HELD` of them are ordered whole, so the middle page is the
/// brute force too (D-3349); `sort_too_deep` refused it before. Past that
/// bound (lowered here on this thread), a page past `MAX_SORT_DEPTH` from the
/// front but within it from the end keeps the order's last rows and is still
/// the brute force, in both directions; a page deep from both ends is refused
/// by name. 202,500 invented one-second bars kept.
#[test]
fn a_page_near_the_end_of_a_read_order_keeps_the_last_rows() {
    let root = Root::new("tail");
    let key = SeriesKey::new("NSE", "INDEX", "TAILTEST", None);
    let month = months()[0];
    let symbol_id =
        u32::try_from(brutex_core::universe::fnv1a("TAILTEST") & u64::from(u32::MAX)).unwrap();
    let address = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "TAILTEST",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    })
    .unwrap();
    let mut writer = store::file::BarFile::open_or_create(&root.0, address, symbol_id).unwrap();
    let mut rows = Vec::new();
    for day in 1..=9_i64 {
        let bars: Vec<Bar> = (0..24_000_i64)
            .map(|i| {
                let n = day * 24_000 + i;
                let close = 7_770_000 + (n * 7_919) % 10_007;
                Bar {
                    ts_micros: month.ist_bounds_micros().0 + (day * 86_400 + 555 * 60 + i) * MICROS,
                    open: close,
                    high: close + 5,
                    low: close - 5,
                    close,
                    volume: 0,
                    open_interest: OI_NULL,
                }
            })
            .collect();
        writer.append(&bars).unwrap();
        rows.extend(bars);
    }
    drop(writer);
    let tod = (9 * 3_600 + 15 * 60, 15 * 3_600 + 30 * 60);
    let filter = Filter {
        from: None,
        to: None,
        tod: Some(tod),
    };
    let kept: Vec<(u32, Bar)> = rows
        .into_iter()
        .filter(|b| {
            let at = u32::try_from((b.ts_micros / MICROS + 19_800).rem_euclid(86_400)).unwrap();
            at >= tod.0 && at <= tod.1
        })
        .map(|b| (0, b))
        .collect();
    let total = kept.len();
    assert!(total > 2 * super::super::MAX_SORT_DEPTH + 37, "{total}");
    let asked = |desc: bool, offset: usize| WindowQuery {
        vendor: Vendor::Gdfl,
        timeframe: Timeframe::SECOND_1,
        series: vec![key.clone()],
        from: month,
        to: month,
        sort: SortKey::Close,
        desc,
        offset: Some(offset),
        limit: 37,
        want_extremes: false,
        at: None,
        filter,
        lookback: true,
        coverage: false,
    };
    for held in [super::MAX_READ_HELD, 150_000] {
        super::READ_HELD.with(|bound| bound.set(held));
        for desc in [false, true] {
            let truth = brute(&kept, &[0], Column::Close, desc);
            let mut offsets = vec![99_000, total - 99_000, total - 1_000, total - 37];
            if held > total as u64 {
                offsets.push(total / 2);
            }
            for offset in offsets {
                let answer = window(&root.0, &asked(desc, offset))
                    .unwrap_or_else(|why| panic!("{held} {desc} {offset}: {why}"));
                let want: Vec<(i64, u32)> = truth.iter().skip(offset).take(37).copied().collect();
                assert_eq!(
                    served(&answer),
                    want,
                    "held {held} desc {desc} offset {offset}"
                );
            }
            if held < total as u64 {
                let why =
                    window(&root.0, &asked(desc, total / 2)).expect_err("deep from both ends");
                assert!(why.starts_with("sort_too_deep"), "{why}");
            }
        }
    }
    super::READ_HELD.with(|bound| bound.set(super::MAX_READ_HELD));
}

/// A month with no `.srt`, and a month whose last run is gone (a writer that
/// died after its commit): their rows are read, the page is the brute force,
/// and the answer names the command that indexes them.
#[test]
fn a_month_without_its_index_is_read_named_and_given_its_build_command() {
    let root = Root::new("absent");
    write(&root);
    let [ce, ..] = keys();
    std::fs::remove_file(path(&ce, months()[1], FileKind::SortIndex).to_path_buf(&root.0)).unwrap();
    let march = path(&ce, months()[2], FileKind::SortIndex).to_path_buf(&root.0);
    let index = store::sort_index::SortIndex::open(
        &march,
        &store::sort_index::Shape::of(
            months()[2],
            60,
            u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)).unwrap(),
        ),
    )
    .unwrap();
    let cut = index.runs()[2].offset;
    drop(index);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&march)
        .unwrap()
        .set_len(cut)
        .unwrap();
    let read = check(&root, &[0, 1], Filter::default(), &|_| true);
    assert!(read > 0);
    let answer = window(
        &root.0,
        &query(&[0], Column::Close, false, 0, Filter::default()),
    )
    .unwrap();
    let report = answer.sort_index.unwrap();
    assert_eq!(report.scanned_rows, 360 + (360 - 260), "{report:?}");
    assert_eq!(report.scanned_months, 2);
    assert!(report.scanned_why[0].contains("2031-02") && report.scanned_why[0].contains("no .srt"));
    assert!(report.scanned_why[1].contains("2031-03") && report.scanned_why[1].contains("0..260"));
    assert_eq!(
        report.build,
        ["cli srt-build gdfl NSE/FNO/NIFTY/2031-03-27-1000000-CE 1min 2031-02 2031-03"]
    );
}

/// **A MONTH WHOSE INDEX FAILS WHILE IT IS READ IS SET ASIDE, NOT SERVED AND
/// NOT FATAL.** A block of February's first run fails its checksum (the walk
/// does not read an earlier run whole): the page is asked again with
/// February's rows read, is the brute force, names the month and the command
/// that rebuilds it — and so does the next request, without meeting the
/// damage again.
#[test]
fn a_month_whose_index_fails_when_read_is_set_aside_and_read() {
    let root = Root::new("broken");
    write(&root);
    let [ce, ..] = keys();
    let feb = path(&ce, months()[1], FileKind::SortIndex).to_path_buf(&root.0);
    let shape = store::sort_index::Shape::of(
        months()[1],
        60,
        u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)).unwrap(),
    );
    let index = store::sort_index::SortIndex::open(&feb, &shape).unwrap();
    assert_eq!(index.runs().len(), 3);
    let run = index.runs()[0];
    let at = run.offset + run.column(Column::Close).entries_at() + 9;
    drop(index);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&feb)
        .unwrap();
    let mut byte = [0u8; 1];
    std::os::unix::fs::FileExt::read_exact_at(&file, &mut byte, at).unwrap();
    byte[0] ^= 0x04;
    std::os::unix::fs::FileExt::write_all_at(&file, &byte, at).unwrap();
    drop(file);
    let rows = every(&[0]);
    for round in 0..2 {
        for desc in [false, true] {
            let truth = brute(&rows, &[0], Column::Close, desc);
            for offset in [0, 500, 1_043] {
                let answer = window(
                    &root.0,
                    &query(&[0], Column::Close, desc, offset, Filter::default()),
                )
                .unwrap_or_else(|why| panic!("round {round}: {why}"));
                let want: Vec<(i64, u32)> = truth.iter().skip(offset).take(37).copied().collect();
                assert_eq!(
                    served(&answer),
                    want,
                    "round {round} desc {desc} offset {offset}"
                );
                let report = answer.sort_index.unwrap();
                assert_eq!(report.scanned_rows, 360, "February is read: {report:?}");
                assert!(
                    report.scanned_why[0].contains("2031-02")
                        && report.scanned_why[0].contains("failed when it was read")
                        && report.scanned_why[0].contains("checksum"),
                    "{report:?}"
                );
                assert_eq!(
                    report.build,
                    ["cli srt-build gdfl NSE/FNO/NIFTY/2031-03-27-1000000-CE 1min 2031-02 2031-02"]
                );
            }
        }
    }
    // Another column of February is not the damaged one, and the month is set
    // aside whole until its index changes: read, and named.
    let answer = window(
        &root.0,
        &query(&[0], Column::Open, false, 0, Filter::default()),
    )
    .unwrap();
    assert_eq!(answer.sort_index.unwrap().scanned_rows, 360);
}

/// **THE EXTREMES OF AN INDEXED WINDOW COST ONE HEADER PER RUN.** The
/// time-ordered page with `extremes=1` folds no bar when every month is
/// indexed — it read every kept row before (DB-N14's `extremes=1`) — and the
/// extremes are the brute-force ones, ties to the earlier bar.
#[test]
fn indexed_extremes_read_no_bar_and_match_the_fold() {
    let root = Root::new("extremes");
    write(&root);
    for series in [&[0, 1][..], &[2]] {
        let mut asked = query(series, Column::Close, true, 0, Filter::default());
        asked.sort = SortKey::Ts;
        asked.want_extremes = true;
        let answer = window(&root.0, &asked).unwrap();
        assert_eq!(answer.scan_rows, 0, "{series:?}: no bar folded");
        let got = answer.extremes.expect("extremes");
        let rows = every(series);
        let first = |pick: &dyn Fn(&Bar) -> i64, larger: bool| {
            rows.iter()
                .map(|(s, b)| (pick(b), b.ts_micros, *s))
                .reduce(|held, now| {
                    let better = if larger {
                        now.0 > held.0
                    } else {
                        now.0 < held.0
                    };
                    if better || (now.0 == held.0 && (now.1, now.2) < (held.1, held.2)) {
                        now
                    } else {
                        held
                    }
                })
                .unwrap()
        };
        let check = |held: Option<super::super::Extreme>, want: (i64, i64, u32)| {
            let held = held.expect("an extreme");
            assert_eq!(
                (held.v, held.t, held.series),
                (want.0, want.1, want.2 as usize)
            );
        };
        check(got.close_max, first(&|b| b.close, true));
        check(got.close_min, first(&|b| b.close, false));
        check(got.high_max, first(&|b| b.high, true));
        check(got.low_min, first(&|b| b.low, false));
        check(got.range_max, first(&|b| b.high - b.low, true));
        if series == [2] {
            assert!(got.volume_max.is_none(), "an index trades no volume");
            assert_eq!(got.volume, 0);
        } else {
            check(got.volume_max, first(&|b| b.volume, true));
        }
        // The same answer from the sorted path.
        let mut sorted = asked.clone();
        sorted.sort = SortKey::Volume;
        let again = window(&root.0, &sorted).unwrap();
        assert_eq!(again.extremes, Some(got));
        assert_eq!(again.scan_rows, 0);
    }
}

/// A request whose client went away stops by name before reading.
#[test]
fn a_cancelled_window_stops_by_name() {
    let root = Root::new("cancel");
    write(&root);
    let (cancel, guard) = Cancel::pair();
    drop(guard);
    assert!(cancel.is_cancelled());
    let why = window_cancellable(
        &root.0,
        &query(&[0], Column::Close, false, 0, Filter::default()),
        &cancel,
    )
    .expect_err("cancelled");
    assert!(why.starts_with("cancelled"), "{why}");
    let (kept, guard) = Cancel::pair();
    guard.disarm();
    assert!(!kept.is_cancelled(), "a disarmed guard cancels nothing");
    assert!(
        window_cancellable(
            &root.0,
            &query(&[0], Column::Close, false, 0, Filter::default()),
            &kept
        )
        .is_ok()
    );
    assert!(!Cancel::never().is_cancelled());
}

/// **A SPARSE RANGE WRITES ONE LINE A REQUEST, COUNTING ITS HOLES** (DB-N17).
/// An index series held in January and December only, asked over two years:
/// the ten months between are holes, the twelve before its first file lie
/// outside its life. One `api.bars` line names the count and the first and
/// last hole, and no absent month writes a `read refused` line of its own —
/// one did, on every request, so a sparse 240-month window wrote 143 of them.
#[test]
fn a_sparse_range_writes_one_line_counting_its_holes() {
    let root = Root::new("holes");
    let key = SeriesKey::new("NSE", "INDEX", "HOLETEST", None);
    let symbol_id =
        u32::try_from(brutex_core::universe::fnv1a("HOLETEST") & u64::from(u32::MAX)).unwrap();
    for month in [
        YearMonth::new(2031, 1).unwrap(),
        YearMonth::new(2031, 12).unwrap(),
    ] {
        let address = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "HOLETEST",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month,
            file: FileKind::Bars,
        })
        .unwrap();
        let mut writer = store::file::BarFile::open_or_create(&root.0, address, symbol_id).unwrap();
        writer
            .append(&[bar(month, 2, 0), bar(month, 2, 1)])
            .unwrap();
    }
    let asked = WindowQuery {
        vendor: Vendor::Gdfl,
        timeframe: Timeframe::MINUTE_1,
        series: vec![key],
        from: YearMonth::new(2030, 1).unwrap(),
        to: YearMonth::new(2031, 12).unwrap(),
        sort: SortKey::Ts,
        desc: true,
        offset: Some(0),
        limit: 5,
        want_extremes: true,
        at: None,
        filter: Filter::default(),
        lookback: true,
        coverage: false,
    };
    let from = crate::emitted::mark();
    for _ in 0..3 {
        let answer = window(&root.0, &asked).unwrap();
        assert_eq!(answer.total, 4);
        assert_eq!(
            answer.missing_months.len(),
            10,
            "{:?}",
            answer.missing_months
        );
    }
    let mine = |message: &str| -> Vec<telemetry::Record> {
        crate::emitted::landed(from, "api.bars", message)
            .into_iter()
            .filter(|line| {
                crate::emitted::says(line, "key", "HOLETEST")
                    || crate::emitted::says(line, "symbol", "HOLETEST")
            })
            .collect()
    };
    let holes = mine("window range holds months not stored");
    assert_eq!(holes.len(), 3, "one line a request: {holes:?}");
    for line in &holes {
        assert!(crate::emitted::counts(line, "months", 10), "{line:?}");
        assert!(crate::emitted::says(line, "first", "2031-02"), "{line:?}");
        assert!(crate::emitted::says(line, "last", "2031-11"), "{line:?}");
    }
    assert!(
        mine("read refused").is_empty(),
        "no absent month is opened or named alone"
    );
}

/// One invented option contract no other fixture writes, on a one-second
/// rung, for the edge cases below.
const EDGE: &str = "2031-06-26-1234500-CE";

fn edge_key() -> SeriesKey {
    SeriesKey::new("NSE", "FNO", "EDGECASE", Contract::parse(EDGE))
}

/// Writes `rows` as one month of the edge contract, one append per chunk of
/// `chunk` rows.
fn write_edge(root: &Root, month: YearMonth, rows: &[Bar], chunk: usize) {
    let key = edge_key();
    let address = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: &key.exchange,
        segment: &key.segment,
        symbol: &key.symbol,
        contract: key.contract,
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    })
    .unwrap();
    let symbol_id =
        u32::try_from(brutex_core::universe::fnv1a(&key.symbol) & u64::from(u32::MAX)).unwrap();
    let mut writer = store::file::BarFile::open_or_create(&root.0, address, symbol_id).unwrap();
    for batch in rows.chunks(chunk.max(1)) {
        writer.append(batch).unwrap();
    }
}

fn edge_query(
    (from, to): (YearMonth, YearMonth),
    column: Column,
    desc: bool,
    offset: usize,
    limit: usize,
) -> WindowQuery {
    WindowQuery {
        vendor: Vendor::Gdfl,
        timeframe: Timeframe::SECOND_1,
        series: vec![edge_key()],
        from,
        to,
        sort: sort_of(column),
        desc,
        offset: Some(offset),
        limit,
        want_extremes: true,
        at: None,
        filter: Filter::default(),
        lookback: true,
        coverage: false,
    }
}

/// Every column, both directions and each of `offsets` of the edge contract
/// over `span` against the brute force of `rows`; returns the rows read.
fn check_edge(
    root: &Root,
    span: (YearMonth, YearMonth),
    rows: &[Bar],
    offsets: &[usize],
    limit: usize,
) -> u64 {
    let placed: Vec<(u32, Bar)> = rows.iter().map(|bar| (0, *bar)).collect();
    let mut read = 0;
    for column in Column::ALL {
        for desc in [false, true] {
            let truth = brute(&placed, &[0], column, desc);
            for &offset in offsets {
                let answer = window(&root.0, &edge_query(span, column, desc, offset, limit))
                    .unwrap_or_else(|why| panic!("{} {desc} {offset}: {why}", column.as_str()));
                assert_eq!(answer.total, rows.len() as u64);
                let want: Vec<(i64, u32)> =
                    truth.iter().skip(offset).take(limit).copied().collect();
                assert_eq!(
                    served(&answer),
                    want,
                    "column {} desc {desc} offset {offset}",
                    column.as_str()
                );
                assert!(answer.faults.is_empty(), "{:?}", answer.faults);
                read += answer.scan_rows;
            }
        }
    }
    read
}

/// **ONE BAR, NO BAR, AND AN OFFSET PAST EVERYTHING.** A series of a single
/// bar pages as itself at offset 0 and as nothing past it, at the smallest
/// and the largest limit; a range the series holds nothing in answers an
/// empty page with a zero total, not a refusal; and an offset no store
/// reaches is an empty page, not an overflow.
#[test]
fn a_single_bar_an_empty_range_and_an_offset_past_everything_are_answered() {
    let root = Root::new("single");
    let june = YearMonth::new(2031, 6).unwrap();
    let only = Bar {
        ts_micros: june.ist_bounds_micros().0 + (86_400 + 555 * 60) * MICROS,
        open: 4_440_100,
        high: 4_440_900,
        low: 4_440_050,
        close: 4_440_500,
        volume: 75,
        open_interest: 1_800,
    };
    write_edge(&root, june, &[only], 1);
    for limit in [1, super::super::MAX_WINDOW_LIMIT] {
        let read = check_edge(
            &root,
            (june, june),
            &[only],
            &[0, 1, usize::MAX / 2, usize::MAX],
            limit,
        );
        assert_eq!(read, 0, "limit {limit}: one bar is served from its index");
    }
    let answer = window(
        &root.0,
        &edge_query((june, june), Column::Close, true, 0, 50),
    )
    .unwrap();
    let got = answer.extremes.expect("the extremes of one bar");
    assert_eq!(
        got.close_max.map(|x| (x.v, x.t)),
        Some((4_440_500, only.ts_micros))
    );
    assert_eq!(
        got.close_min.map(|x| (x.v, x.t)),
        Some((4_440_500, only.ts_micros))
    );
    assert_eq!(got.range, 850);

    // NOTHING HELD IN THE RANGE: the year before the contract's first month.
    let (from, to) = (
        YearMonth::new(2030, 1).unwrap(),
        YearMonth::new(2030, 12).unwrap(),
    );
    for column in Column::ALL {
        for desc in [false, true] {
            let answer = window(&root.0, &edge_query((from, to), column, desc, 0, 50))
                .unwrap_or_else(|why| panic!("{} {desc}: {why}", column.as_str()));
            assert_eq!((answer.total, answer.bars.len()), (0, 0));
            assert_eq!(answer.scan_rows, 0);
            let report = answer.sort_index.expect("a value page reports its index");
            assert_eq!((report.indexed_rows, report.scanned_rows), (0, 0));
        }
    }
}

/// **VALUES AT THE ENDS OF `i64`, AND COLUMNS TOO WIDE FOR FOUR BYTES.** A
/// volume and an open interest of `i64::MAX`, a high whose span passes four
/// bytes, zeros, nulls and many ties: every page is the brute force in both
/// directions (descending keys are the bitwise complement, so the largest
/// value is the smallest key), and no bar outside the page is read.
#[test]
fn values_at_the_ends_of_i64_and_wide_columns_page_like_the_brute_force() {
    let root = Root::new("wide");
    let june = YearMonth::new(2031, 6).unwrap();
    let open = june.ist_bounds_micros().0 + (2 * 86_400 + 555 * 60) * MICROS;
    let rows: Vec<Bar> = (0..900_i64)
        .map(|i| {
            let close = 5_550_000 + (i * 7_919) % 211;
            Bar {
                ts_micros: open + i * MICROS,
                open: close - i % 4,
                // One bar in seven prints a high sixty million rupees up: the
                // column's span no longer fits four bytes.
                high: close + if i % 7 == 0 { 6_000_000_000 + i } else { 30 },
                low: close - 30,
                close,
                volume: match i % 5 {
                    0 => 0,
                    1 => i64::MAX,
                    2 => i64::MAX - 1,
                    _ => 5_000_000_000 + i % 3,
                },
                open_interest: match i % 6 {
                    0 => OI_NULL,
                    1 => i64::MAX,
                    2 => 0,
                    _ => 9_000_000_000_000 + i % 4,
                },
            }
        })
        .collect();
    write_edge(&root, june, &rows, 400);
    let srt = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "FNO",
        symbol: "EDGECASE",
        contract: Contract::parse(EDGE),
        timeframe: Timeframe::SECOND_1,
        month: june,
        file: FileKind::SortIndex,
    })
    .unwrap()
    .to_path_buf(&root.0);
    let index = store::sort_index::SortIndex::open(
        &srt,
        &store::sort_index::Shape::of(
            june,
            1,
            u32::try_from(brutex_core::universe::fnv1a("EDGECASE") & u64::from(u32::MAX)).unwrap(),
        ),
    )
    .unwrap();
    let widths: Vec<u8> = Column::ALL
        .iter()
        .map(|&column| index.runs()[0].column(column).width)
        .collect();
    assert_eq!(
        widths,
        [8, 12, 8, 8, 12, 12],
        "high, volume and open interest are written wide"
    );
    drop(index);
    let total = rows.len();
    let read = check_edge(
        &root,
        (june, june),
        &rows,
        &[0, 1, total / 2, total - 37, total - 1, total],
        37,
    );
    assert_eq!(read, 0);
}

/// **A SPECIAL SESSION AND THE EDGES OF THE MONTH KEEP THEIR PLACES.** Bars
/// at the month's first second (00:00:00 IST on the 1st), in an evening
/// session on a Sunday, and at its last second: each is a second of the
/// month an entry can name, and each pages where the brute force puts it —
/// no stamp is assumed to lie in the ordinary session.
#[test]
fn a_special_session_and_the_edges_of_the_month_keep_their_places() {
    let root = Root::new("special");
    let june = YearMonth::new(2031, 6).unwrap();
    let (lo, hi) = june.ist_bounds_micros();
    // 1 June 2031 is a Sunday: 18:15 to 18:20 IST that evening.
    let evening = lo + (18 * 3_600 + 15 * 60) * MICROS;
    let ordinary = lo + (86_400 + 555 * 60) * MICROS;
    let mut stamps = vec![lo, lo + MICROS];
    stamps.extend((0..300).map(|i| evening + i * MICROS));
    stamps.extend((0..300).map(|i| ordinary + i * MICROS));
    stamps.extend([hi - 2 * MICROS, hi - MICROS]);
    let rows: Vec<Bar> = stamps
        .iter()
        .enumerate()
        .map(|(i, &ts)| {
            let i = i64::try_from(i).unwrap();
            // The edges tie with rows inside the sessions, so time decides.
            let close = 3_330_000 + (i * 37) % 23;
            Bar {
                ts_micros: ts,
                open: close,
                high: close + 15,
                low: close - 15,
                close,
                volume: i % 3,
                open_interest: 500 + i % 2,
            }
        })
        .collect();
    write_edge(&root, june, &rows, 151);
    let total = rows.len();
    let read = check_edge(
        &root,
        (june, june),
        &rows,
        &[0, total / 3, total - 37, total - 1],
        37,
    );
    assert_eq!(read, 0);
    // The month's first and last seconds are served with their own stamps.
    let first = window(
        &root.0,
        &edge_query((june, june), Column::Close, false, 0, total),
    )
    .unwrap();
    let stamps_served: std::collections::BTreeSet<i64> =
        first.bars.iter().map(|row| row.bar.ts_micros).collect();
    assert!(stamps_served.contains(&lo) && stamps_served.contains(&(hi - MICROS)));
    assert_eq!(stamps_served.len(), total);
}

/// **AN INDEX BUILT FROM ANOTHER CONTRACT'S BARS IS NEVER SERVED AS THIS
/// ONE'S.** The two contracts trade the same instants, so the other's `.srt`
/// — same underlying, month, rung, row count and end stamps — binds. Every
/// row a page serves is still read from this contract's bars and checked
/// against the entry that named it: a row whose value disagrees is named in
/// `faults` with the command that rebuilds the index, and is not on the
/// page. No value the bars do not hold is ever served.
#[test]
fn an_index_built_from_another_contracts_bars_is_never_served_as_this_ones() {
    let root = Root::new("foreign");
    write(&root);
    let [ce, pe, _] = keys();
    let feb = months()[1];
    std::fs::copy(
        path(&pe, feb, FileKind::SortIndex).to_path_buf(&root.0),
        path(&ce, feb, FileKind::SortIndex).to_path_buf(&root.0),
    )
    .unwrap();
    let own: std::collections::HashMap<i64, Bar> = (0..PER_MONTH)
        .map(|i| bar(feb, 0, i))
        .map(|bar| (bar.ts_micros, bar))
        .collect();
    let mut asked = query(&[0], Column::Close, false, 0, Filter::default());
    (asked.from, asked.to, asked.limit) = (feb, feb, 360);
    let answer = window(&root.0, &asked).unwrap();
    assert_eq!(answer.total, 360, "the total is the bars', not the index's");
    let report = answer
        .sort_index
        .clone()
        .expect("a value page reports its index");
    assert_eq!(
        report.indexed_rows, 360,
        "the foreign index binds: {report:?}"
    );
    for row in &answer.bars {
        assert_eq!(
            own.get(&row.bar.ts_micros),
            Some(&row.bar),
            "a served row is this contract's own bar"
        );
    }
    assert_eq!(
        answer.bars.len() + answer.faults.len(),
        360,
        "every place of the page is a checked row or a named fault"
    );
    assert!(
        !answer.faults.is_empty(),
        "the two contracts' closes differ"
    );
    assert!(
        answer
            .faults
            .iter()
            .all(|fault| fault.contains("the sort index names")
                && fault.contains(
                    "cli srt-build gdfl NSE/FNO/NIFTY/2031-03-27-1000000-CE 1min 2031-02 2031-02"
                )),
        "{:?}",
        answer.faults
    );
}

/// **AN INDEX FROM ANOTHER MONTH OR ANOTHER SYMBOL IS REFUSED, AND THE MONTH
/// IS READ.** January's `.srt` under February's name, then another symbol's
/// February under this one's: each fails the header's binding, so the
/// month's rows are read, the page is the brute force, and the answer names
/// the month, why, and the command that rebuilds it.
#[test]
fn an_index_renamed_from_another_month_or_symbol_is_refused_and_the_month_read() {
    let root = Root::new("renamed");
    write(&root);
    let [ce, ..] = keys();
    let (jan, feb) = (months()[0], months()[1]);
    let other = edge_key();
    let stranger: Vec<Bar> = (0..PER_MONTH).map(|i| bar(feb, 0, i)).collect();
    write_edge(&root, feb, &stranger, 130);
    let other_symbol = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: &other.exchange,
        segment: &other.segment,
        symbol: &other.symbol,
        contract: other.contract,
        timeframe: Timeframe::SECOND_1,
        month: feb,
        file: FileKind::SortIndex,
    })
    .unwrap()
    .to_path_buf(&root.0);
    for foreign in [
        path(&ce, jan, FileKind::SortIndex).to_path_buf(&root.0),
        other_symbol,
    ] {
        std::fs::copy(
            &foreign,
            path(&ce, feb, FileKind::SortIndex).to_path_buf(&root.0),
        )
        .unwrap();
        let read = check(&root, &[0], Filter::default(), &|_| true);
        assert!(read > 0, "{}: February is read", foreign.display());
        let answer = window(
            &root.0,
            &query(&[0], Column::Close, false, 0, Filter::default()),
        )
        .unwrap();
        let report = answer.sort_index.unwrap();
        assert_eq!(report.scanned_rows, 360, "{report:?}");
        assert_eq!(report.indexed_rows, 720, "{report:?}");
        assert!(
            report.scanned_why[0].contains("2031-02")
                && report.scanned_why[0].contains("is not this bar file's"),
            "{report:?}"
        );
        assert_eq!(
            report.build,
            ["cli srt-build gdfl NSE/FNO/NIFTY/2031-03-27-1000000-CE 1min 2031-02 2031-02"]
        );
    }
}

/// **CONCURRENT REQUESTS SHARE THE MEMOS AND EACH IS THE BRUTE FORCE.** Eight
/// threads page the same store at once — the walks, fence tables and cut
/// entries are process-wide — over whole months and under a day bound, and
/// every page of every thread is the brute-force page.
#[test]
fn concurrent_value_pages_are_each_the_brute_force() {
    let root = Root::new("concurrent");
    write(&root);
    let from = months()[0].ist_bounds_micros().0 + 2 * 86_400 * MICROS;
    let to = months()[2].ist_bounds_micros().0 + 3 * 86_400 * MICROS;
    let bound = Filter {
        from: Some(from),
        to: Some(to),
        tod: None,
    };
    std::thread::scope(|scope| {
        for worker in 0..8 {
            let root = &root;
            scope.spawn(move || {
                let series: &[usize] = if worker % 2 == 0 { &[0, 1] } else { &[1, 2, 0] };
                if worker % 4 < 2 {
                    check(root, series, Filter::default(), &|_| true);
                } else {
                    check(root, series, bound, &|b: &Bar| {
                        b.ts_micros >= from && b.ts_micros < to
                    });
                }
            });
        }
    });
}
