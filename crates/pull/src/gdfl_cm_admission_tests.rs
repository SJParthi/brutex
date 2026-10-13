#![cfg(test)]
//! WP-12a: GDFL capital-market admission. Per-row refusal (D-3318; DB-024),
//! special sessions read inside their windows (D-3319; DB-072, DB-G55,
//! DB-G69, DB-G87), the other indices, INDIA VIX and old names stored and
//! never swept (D-3333; DB-069, DB-070, DB-071, DB-G26, DB-G53), the
//! closing auction's print kept apart from trades (D-3320; DB-034, DB-G42),
//! and same-second dual values counted (D-3335; DB-G96).
//!
//! Every value is invented at run time and no vendor row is quoted
//! (`gdfl_cm::tests::fixtures_are_built_not_pasted` walks this file): the
//! levels and prices are chosen so they cannot be mistaken for real ones.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::Bar;
use store::path::{PathParts, StorePath, Timeframe};

use super::*;
use crate::gdfl_archive::day_zip_name;
use crate::gdfl_fixtures::{bts, csv, hms, put, row, scratch};
use crate::gdfl_import::{ImportKind, Report, Run, journal_path, run_cm};
use crate::gdfl_tickstore::TickStore;

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn at(h: u32, m: u32, s: u32) -> u32 {
    h * 3_600 + m * 60 + s
}

/// An invented row with its own bid and ask.
fn quoted(stem: &str, d: Day, sod: u32, ltp: &str, quotes: (&str, &str), ltq: u64) -> String {
    format!(
        "{stem},{:02}/{:02}/{:04},{},{ltp},{},7,{},9,{ltq},0",
        d.day(),
        d.month(),
        d.year(),
        hms(sod),
        quotes.0,
        quotes.1
    )
}

/// An invented index level: far above any real one, two places.
fn level(n: u32) -> String {
    format!("{}.{:02}", 77_000 + n / 100, n % 100)
}

/// `n` index rows, one a second from `from`, of `stem`.
fn index_rows(stem: &str, d: Day, from: u32, n: u32) -> Vec<String> {
    (0..n)
        .map(|i| row(stem, d, from + i, &level(i * 7 % 900), 0, 0))
        .collect()
}

fn expect_of(kind: CmKind, stem: &str, d: Day) -> Expect<'_> {
    Expect { kind, stem, day: d }
}

/// Writes one tick-store day of `kind` holding `files` (names in the day
/// folder).
fn ts_day(root: &Path, kind: CmKind, d: Day, files: &[(String, Vec<u8>)]) {
    let folder = day_folder_name(kind, d);
    let entries: Vec<(String, Vec<u8>)> = files
        .iter()
        .map(|(name, bytes)| (format!("{folder}/{name}"), bytes.clone()))
        .collect();
    let refs: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let zip_name = day_zip_name(kind, d);
    let stem = zip_name.strip_suffix(".zip").unwrap();
    put(root, &format!("ts/cm/{stem}.bts"), &bts(&refs));
}

/// One run of `kind` over the tick store under `root`, filing `scope`.
fn import(
    root: &Path,
    store: &Path,
    (kind, from, to): (ImportKind, Day, Day),
    scope: CmScope,
    only: &[String],
) -> Report {
    let run = Run {
        kind,
        from,
        to,
        only,
        store_root: store,
    };
    run_cm(
        &TickStore::new(&root.join("ts")).with_scope(Arc::new(scope)),
        &run,
    )
    .unwrap()
}

/// Every one-second bar of `segment`/`symbol` in `d`'s month, or `None`.
fn held(store: &Path, segment: &str, symbol: &str, d: Day) -> Option<Vec<Bar>> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: d.year_month().unwrap(),
        file: store::path::FileKind::Bars,
    })
    .unwrap();
    #[expect(clippy::cast_possible_truncation, reason = "the store's own id fold")]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    let file = BarFile::open_existing(store, path, id).ok()?;
    Some(
        (0..file.header().n_valid)
            .map(|i| file.read_record(i).unwrap())
            .collect(),
    )
}

/// The second of the IST day a bar opens at.
fn sod_of(bar: &Bar, d: Day) -> u32 {
    let secs = bar.ts_micros / 1_000_000 + 19_800 - i64::from(d.days_from_epoch()) * 86_400;
    u32::try_from(secs).unwrap()
}

// ── D-3318: a bad row is refused by name, and the rest of its file read ──

/// DB-024's shape on 2022-03-07, invented: four zero-LTP rows (two in the
/// session, two after it) used to refuse the whole `NIFTY 50` file, and
/// with it every NIFTY option bar's spot that day. Now each is refused by
/// name and counted, and every other row is read.
#[test]
fn zero_ltp_rows_are_refused_by_name_and_the_index_day_is_read() {
    let d = day(2022, 3, 7);
    let stem = "NIFTY 50.NSE_IDX";
    let mut rows = vec![row(stem, d, at(9, 8, 1), &level(1), 0, 0)];
    rows.extend(index_rows(stem, d, at(9, 15, 0), 120));
    rows.push(row(stem, d, at(10, 25, 41), "0", 0, 0));
    rows.push(row(stem, d, at(10, 25, 41), "0.00", 0, 0));
    rows.extend(index_rows(stem, d, at(10, 25, 42), 30));
    rows.push(row(stem, d, at(15, 31, 59), &level(3), 0, 0));
    rows.push(row(stem, d, at(22, 27, 5), "0", 0, 0));
    rows.push(row(stem, d, at(22, 27, 5), "0", 0, 0));
    let bytes = csv(&rows);
    let file = decode(&bytes, &expect_of(CmKind::Indices, stem, d)).unwrap();
    assert_eq!(file.rows.len(), 1 + 120 + 30 + 1, "every other row read");
    assert_eq!(
        (
            file.refused.zero_ltp,
            file.refused.off_grid,
            file.refused.malformed
        ),
        (4, 0, 0)
    );
    assert_eq!(file.refused.total(), 4);
    let named: Vec<(u32, Option<u32>, RowFault)> = file
        .refused
        .named
        .iter()
        .map(|r| (r.line, r.sod, r.fault))
        .collect();
    assert_eq!(
        named,
        vec![
            (123, Some(at(10, 25, 41)), RowFault::ZeroLtp),
            (124, Some(at(10, 25, 41)), RowFault::ZeroLtp),
            (156, Some(at(22, 27, 5)), RowFault::ZeroLtp),
            (157, Some(at(22, 27, 5)), RowFault::ZeroLtp),
        ]
    );
    assert_eq!(
        file.refused.named_text(),
        "123:zero_ltp 124:zero_ltp 156:zero_ltp 157:zero_ltp"
    );
    // The largest stamp is a KEPT row's: the refused 22:27:05 rows raise
    // nothing.
    assert_eq!(file.max_sod, at(15, 31, 59));

    // Through the import: the day closes done, its 150 in-session seconds
    // are filed, and nothing is refused.
    let root = scratch("wp12a-zero-ltp");
    ts_day(&root, CmKind::Indices, d, &[(format!("{stem}.csv"), bytes)]);
    let store = root.join("S");
    let got = import(
        &root,
        &store,
        (ImportKind::Indices, d, d),
        CmScope::swept(),
        &[],
    );
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!((got.days_imported, got.files, got.files_refused), (1, 1, 0));
    assert!(got.closed_with_refusals.is_empty() && got.closed_refused.is_empty());
    let bars = held(&store, "INDEX", "NIFTY", d).unwrap();
    assert_eq!(bars.len(), 150);
    assert!(bars.iter().all(|b| b.close > 0));
    assert!(
        std::fs::read_to_string(journal_path(&store))
            .unwrap()
            .contains("done indices 2022-03-07 * ")
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// D-3318's bound: a file is refused whole only past BOTH 64 refused rows
/// and one in every 1,000 of its rows; at or under either it is read.
#[test]
fn too_many_bad_rows_refuse_the_file_whole_only_past_both_bounds() {
    let d = day(2024, 4, 1);
    let stem = "NIFTY 50.NSE_IDX";
    // Good rows cycle through the session's 22,500 seconds, so a file of
    // 70,000 of them holds no stamp past midnight.
    let file = |good: u32, bad: u32| {
        let mut rows: Vec<String> = (0..good)
            .map(|i| {
                row(
                    stem,
                    d,
                    at(9, 15, 0) + i % 22_500,
                    &level(i * 7 % 900),
                    0,
                    0,
                )
            })
            .collect();
        for i in 0..bad {
            rows.push(row(stem, d, at(9, 15, 0) + i % 60, "0.001", 0, 0));
        }
        rows.push(row(stem, d, at(15, 30, 0), &level(9), 0, 0));
        decode(&csv(&rows), &expect_of(CmKind::Indices, stem, d))
    };
    let at_bound = file(10, MAX_BAD_ROWS).unwrap();
    assert_eq!(at_bound.refused.off_grid, MAX_BAD_ROWS);
    assert_eq!(
        file(10, MAX_BAD_ROWS + 1),
        Err(CmRefusal::TooManyBadRows {
            refused: 65,
            rows: 76
        })
    );
    // 65 of 70,066 is under one in 1,000: read.
    assert_eq!(file(70_000, 65).unwrap().refused.total(), 65);
    // 71 of 70,072 is over it: refused.
    assert_eq!(
        file(70_000, 71),
        Err(CmRefusal::TooManyBadRows {
            refused: 71,
            rows: 70_072
        })
    );
    // Only the first `NAMED_ROWS` are named; every one is counted.
    assert_eq!(at_bound.refused.named.len(), NAMED_ROWS);
    assert!(
        CmRefusal::TooManyBadRows {
            refused: 65,
            rows: 76
        }
        .to_string()
        .contains("65 of 76 rows refused")
    );
}

/// A row of another file (Ticker, Date) still refuses the whole file: it is
/// not a bad row of this one (D-3318 keeps these whole).
#[test]
fn a_row_of_another_file_still_refuses_the_whole_file() {
    let d = day(2024, 4, 1);
    let stem = "NIFTY 50.NSE_IDX";
    let mut rows = index_rows(stem, d, at(9, 15, 0), 5);
    rows.push(row("NIFTY BANK.NSE_IDX", d, at(9, 16, 0), &level(1), 0, 0));
    rows.push(row(stem, d, at(15, 30, 0), &level(2), 0, 0));
    assert_eq!(
        decode(&csv(&rows), &expect_of(CmKind::Indices, stem, d)),
        Err(CmRefusal::TickerMismatch { line: 7 })
    );
    let mut rows = index_rows(stem, d, at(9, 15, 0), 5);
    rows.push(row(stem, day(2024, 4, 2), at(9, 16, 0), &level(1), 0, 0));
    assert_eq!(
        decode(&csv(&rows), &expect_of(CmKind::Indices, stem, d)),
        Err(CmRefusal::DateMismatch { line: 7 })
    );
}

// ── D-3319: special sessions, read inside their windows ──────────────────

/// The disaster-recovery drill's two windows decide what is "in session"
/// and where an index file must reach, never 09:15 to 15:30: a file ending
/// at 12:30:00 is whole, one ending at 12:29:59 was cut.
#[test]
fn a_drill_day_file_is_judged_against_its_two_windows() {
    let d = day(2024, 3, 2);
    let stem = "NIFTY 50.NSE_IDX";
    let rules = Rules {
        session: session_of(d).unwrap(),
        ..Rules::regular()
    };
    let file = |last: u32| {
        let mut rows = vec![row(stem, d, at(9, 10, 0), &level(1), 0, 0)];
        rows.extend(index_rows(stem, d, at(9, 20, 0), 3));
        rows.extend(index_rows(stem, d, at(10, 30, 0), 2));
        rows.extend(index_rows(stem, d, at(11, 45, 0), 4));
        rows.push(row(stem, d, last, &level(5), 0, 0));
        decode_with(&csv(&rows), &expect_of(CmKind::Indices, stem, d), &rules)
    };
    let whole = file(at(12, 30, 0)).unwrap();
    assert_eq!(
        (whole.rows_pre, whole.rows_in_session, whole.rows_post),
        (1, 7, 3),
        "the 10:30 break and 12:30 are outside both windows"
    );
    assert_eq!(
        file(at(12, 29, 59)),
        Err(CmRefusal::IndexEndsInSession {
            max_sod: at(12, 29, 59)
        })
    );
    // Under the regular rules the same whole file ends inside 09:15-15:30.
    let mut rows = index_rows(stem, d, at(9, 20, 0), 3);
    rows.push(row(stem, d, at(12, 30, 0), &level(5), 0, 0));
    assert!(matches!(
        decode(&csv(&rows), &expect_of(CmKind::Indices, stem, d)),
        Err(CmRefusal::IndexEndsInSession { .. })
    ));
}

/// End to end (DB-072, DB-G55): the 2024-03-02 drill and the 2025-10-21
/// Muhurat are imported, and only the seconds inside their recorded windows
/// are filed. Before D-3319 each day was refused whole, `calendar_not_regular`.
#[test]
fn special_session_days_are_imported_inside_their_windows() {
    let root = scratch("wp12a-special");
    let drill = day(2024, 3, 2);
    let muhurat = day(2025, 10, 21);
    let n50 = "NIFTY 50.NSE_IDX";
    let mut drill_rows = vec![row(n50, drill, at(9, 8, 0), &level(1), 0, 0)];
    drill_rows.extend(index_rows(n50, drill, at(9, 20, 0), 10));
    drill_rows.extend(index_rows(n50, drill, at(10, 30, 0), 5));
    drill_rows.extend(index_rows(n50, drill, at(11, 45, 0), 10));
    drill_rows.extend(index_rows(n50, drill, at(12, 31, 0), 2));
    ts_day(
        &root,
        CmKind::Indices,
        drill,
        &[(format!("{n50}.csv"), csv(&drill_rows))],
    );
    let mut hour = index_rows(n50, muhurat, at(13, 40, 0), 3);
    hour.extend(index_rows(n50, muhurat, at(13, 45, 0), 12));
    hour.extend(index_rows(n50, muhurat, at(14, 50, 0), 4));
    ts_day(
        &root,
        CmKind::Indices,
        muhurat,
        &[(format!("{n50}.csv"), csv(&hour))],
    );
    let store = root.join("S");
    for (d, filed) in [(drill, 20), (muhurat, 12)] {
        let got = import(
            &root,
            &store,
            (ImportKind::Indices, d, d),
            CmScope::swept(),
            &[],
        );
        assert!(got.failures.is_empty(), "{d}: {:?}", got.failures);
        assert_eq!((got.days_imported, got.days_refused), (1, 0), "{d}");
        let bars: Vec<Bar> = held(&store, "INDEX", "NIFTY", d)
            .unwrap()
            .into_iter()
            .filter(|b| {
                let secs = b.ts_micros / 1_000_000 + 19_800;
                secs.div_euclid(86_400) == i64::from(d.days_from_epoch())
            })
            .collect();
        assert_eq!(bars.len(), filed, "{d}");
        let session = session_of(d).unwrap();
        for bar in &bars {
            let minute = u16::try_from(sod_of(bar, d) / 60).unwrap();
            assert!(session.expects(minute), "{d}: a bar at minute {minute}");
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// End to end (DB-G69, DB-G87): the 2018-11-07 Muhurat, whose window the
/// calendar of record holds (17:30-18:29) and the store's session authority
/// keeps (WP-13b), is imported inside it: the day closes done and only the
/// window's traded seconds are filed. Before D-3319 it was refused whole,
/// `calendar_not_regular` (`OpenLengthUnmeasured`).
#[test]
fn a_recorded_muhurat_is_imported_inside_its_window() {
    let root = scratch("wp12a-muhurat");
    let d = day(2018, 11, 7);
    let stem = "RELIANCE.NSE";
    let mut rows = vec![row(stem, d, at(17, 20, 0), "4321.10", 1, 0)];
    for i in 0..30 {
        rows.push(row(stem, d, at(17, 30, 0) + i * 61, "4321.15", 3, 0));
    }
    rows.push(row(stem, d, at(18, 45, 0), "4321.20", 2, 0));
    ts_day(
        &root,
        CmKind::Stocks,
        d,
        &[(format!("{stem}.csv"), csv(&rows))],
    );
    let store = root.join("S");
    let got = import(
        &root,
        &store,
        (ImportKind::Stocks, d, d),
        CmScope::swept(),
        &[],
    );
    let window = crate::calendar::special_session(i64::from(d.days_from_epoch()))
        .unwrap()
        .session;
    assert!(landed(d, window), "WP-13b keeps the recorded window");
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!((got.days_imported, got.days_refused), (1, 0));
    let bars = held(&store, "CASH", "RELIANCE", d).unwrap();
    assert_eq!(bars.len(), 30);
    for bar in &bars {
        let minute = u16::try_from(sod_of(bar, d) / 60).unwrap();
        assert!(window.expects(minute), "a bar at minute {minute}");
    }
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(journal.contains("done stocks 2018-11-07 * "), "{journal}");
    std::fs::remove_dir_all(root).unwrap();
}

/// End to end (DB-G55, DB-G69): an options day of a special session is
/// admitted by the same day gate and lands only inside the session's
/// recorded windows, through the store's one session authority. Before
/// D-3319 the 2021-02-24 outage day and the 2018-11-07 Muhurat were refused
/// whole for options too (`calendar_not_regular`).
#[test]
fn an_options_day_of_a_special_session_lands_inside_its_windows() {
    use brutex_core::instrument::Contract;
    for (d, ticker, contract) in [
        (
            day(2021, 2, 24),
            "NIFTY25FEB2177000CE",
            "2021-02-25-7700000-CE",
        ),
        (
            day(2018, 11, 7),
            "NIFTY29NOV1877000CE",
            "2018-11-29-7700000-CE",
        ),
    ] {
        let root = scratch("wp12a-nfo-special");
        let folder = crate::gdfl_nfo::day_folder_name(d);
        let stem = format!("{ticker}.NFO");
        // One traded row every 97 s across the whole day, 08:00 to 21:00,
        // so rows fall before, inside, between and after the windows.
        let rows: Vec<String> = (0..483)
            .map(|i| {
                row(
                    &stem,
                    d,
                    at(8, 0, 0) + i * 97,
                    "4.35",
                    75,
                    1_000 + u64::from(i),
                )
            })
            .collect();
        let entry = format!("{folder}\\Options\\{stem}.csv");
        let bytes = csv(&rows);
        let month = format!("{}_{:04}", crate::gdfl_fixtures::mon(d), d.year());
        put(
            &root,
            &format!(
                "ts/options/{:04}/{month}/GFDLNFO_TICK_{:02}{:02}{:04}.bts",
                d.year(),
                d.day(),
                d.month(),
                d.year()
            ),
            &bts(&[(entry.as_str(), bytes.as_slice())]),
        );
        let store = root.join("S");
        let run = Run {
            kind: ImportKind::Options,
            from: d,
            to: d,
            only: &[],
            store_root: &store,
        };
        let got = crate::gdfl_import::run_nfo(
            &crate::gdfl_nfo::NfoTickStore::new(&root.join("ts")),
            &run,
        )
        .unwrap();
        assert!(got.failures.is_empty(), "{d}: {:?}", got.failures);
        assert_eq!(
            (got.days_imported, got.days_refused, got.files),
            (1, 0, 1),
            "{d}"
        );
        let session = session_of(d).unwrap();
        let path = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "FNO",
            symbol: "NIFTY",
            contract: Some(Contract::parse(contract).unwrap()),
            timeframe: Timeframe::SECOND_1,
            month: d.year_month().unwrap(),
            file: store::path::FileKind::Bars,
        })
        .unwrap();
        #[expect(clippy::cast_possible_truncation, reason = "the store's own id fold")]
        let id = brutex_core::universe::fnv1a("NIFTY") as u32;
        let file = BarFile::open_existing(&store, path, id).unwrap();
        let bars: Vec<Bar> = (0..file.header().n_valid)
            .map(|i| file.read_record(i).unwrap())
            .collect();
        let inside = (0..483_u32)
            .filter(|i| session.expects(u16::try_from((at(8, 0, 0) + i * 97) / 60).unwrap()))
            .count();
        assert!(inside > 0, "{d}");
        assert_eq!(
            bars.len(),
            inside,
            "{d}: every in-window row, and only those"
        );
        for bar in &bars {
            let minute = u16::try_from(sod_of(bar, d) / 60).unwrap();
            assert!(session.expects(minute), "{d}: a bar at minute {minute}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// The real options tree's underlying names (shapes only: an ampersand, a
/// hyphen, the index underlyings) all build a stored scope; the index
/// underlyings are left to the index import and the shares are filed.
#[test]
fn a_stored_scope_takes_every_shape_of_option_underlying_name() {
    let names = [
        "BAJAJ-AUTO",
        "L&TFH",
        "M&M",
        "M&MFIN",
        "MCDOWELL-N",
        "HDFC",
        "MINDTREE",
        "NIFTY",
        "BANKNIFTY",
        "NIFTYIT",
        "NIFTYCPSE",
    ];
    let scope = CmScope::stored(names).unwrap();
    for share in ["L&TFH", "MCDOWELL-N", "HDFC", "MINDTREE"] {
        assert!(scope.shares().iter().any(|s| &**s == share), "{share}");
    }
    for index in ["NIFTY", "BANKNIFTY", "NIFTYIT", "NIFTYCPSE"] {
        assert!(!scope.shares().iter().any(|s| &**s == index), "{index}");
    }
    let d = day(2020, 6, 3);
    let cash = |s: &str| InstrumentKey::cash(Exchange::Nse, s).unwrap();
    for (stem, key) in [
        ("L&TFH.NSE", "LTF"),
        ("MCDOWELL-N.NSE", "UNITDSPR"),
        ("BAJAJ-AUTO.NSE", "BAJAJ-AUTO"),
        ("M&M.NSE", "M&M"),
        ("MINDTREE.NSE", "MINDTREE"),
    ] {
        assert_eq!(
            scope.filing(CmKind::Stocks, stem, d),
            Ok(Some(cash(key))),
            "{stem}"
        );
    }
}

// ── D-3333: the other indices, INDIA VIX and old names ─────────────────

/// DB-070, DB-G26, DB-G53: every applied index row of `core::alias` is
/// filed under its store key by the stored scope; the swept two keep their
/// keys; nothing else of the index tree is filed; and none of it is swept.
#[test]
fn the_stored_scope_files_every_alias_index_under_its_store_key() {
    let stored = CmScope::stored(std::iter::empty::<&str>()).unwrap();
    let swept = CmScope::swept();
    let d = day(2024, 8, 6);
    for (stem, key) in [
        ("NIFTY FIN SERVICE", "NIFTYFINSERVICE"),
        ("NIFTY MID SELECT", "NIFTYMIDSELECT"),
        ("NIFTY NEXT 50", "NIFTYNEXT50"),
        ("NIFTY IT", "NIFTYIT"),
        ("NIFTY FPI 150", "NIFTYFPI150"),
        ("NIFTY CPSE", "NIFTYCPSE"),
        ("INDIA VIX", "INDIAVIX"),
    ] {
        let stem = format!("{stem}.NSE_IDX");
        let want = InstrumentKey::index(Exchange::Nse, key).unwrap();
        assert_eq!(
            stored.filing(CmKind::Indices, &stem, d),
            Ok(Some(want)),
            "{stem}"
        );
        assert!(!want.is_sweepable(), "{key} is stored, never swept");
        assert_eq!(swept.filing(CmKind::Indices, &stem, d), Ok(None), "{stem}");
        assert_eq!(stored.filing(CmKind::Stocks, &stem, d), Ok(None), "{stem}");
        assert_eq!(
            stored.stem_for(&want, d),
            Some((CmKind::Indices, stem.clone()))
        );
        let rule = stored.price_rule(&want);
        assert_eq!(rule == PriceRule::HalfUp, key == "INDIAVIX", "{key}");
    }
    assert_eq!(stored.indices().len(), 7);
    for (stem, key) in [("NIFTY 50", "NIFTY"), ("NIFTY BANK", "BANKNIFTY")] {
        let want = InstrumentKey::index(Exchange::Nse, key).unwrap();
        for scope in [&stored, &swept] {
            assert_eq!(
                scope.filing(CmKind::Indices, &format!("{stem}.NSE_IDX"), d),
                Ok(Some(want))
            );
            assert_eq!(scope.price_rule(&want), PriceRule::Exact);
        }
    }
    for other in [
        "NIFTY 100.NSE_IDX",
        "nifty it.NSE_IDX",
        "NIFTY IT",
        "NIFTY IT.NSE",
    ] {
        assert_eq!(
            stored.filing(CmKind::Indices, other, d),
            Ok(None),
            "{other}"
        );
    }
}

/// DB-069, DB-071: a share stem is filed under the newest symbol its chain
/// of applied changes reaches, for the days before each change; a stem the
/// exchange was not using that day is refused by name; a stored option
/// underlying with no change keeps its own key; and the swept scope files
/// none of the stored shares.
#[test]
fn old_names_are_filed_under_the_newest_symbol_before_each_change() {
    let stored = CmScope::stored(["TATAMOTORS", "HDFC", "LTI", "PVR", "NIFTYIT", "NIFTY"]).unwrap();
    let swept = CmScope::swept();
    let cash = |s: &str| InstrumentKey::cash(Exchange::Nse, s).unwrap();
    for (stem, d, key) in [
        ("TATAMOTORS.NSE", day(2022, 6, 15), "TMPV"),
        ("TATAMOTORS.NSE", day(2025, 10, 23), "TMPV"),
        ("TMPV.NSE", day(2025, 10, 24), "TMPV"),
        ("ZOMATO.NSE", day(2024, 1, 2), "ETERNAL"),
        ("ETERNAL.NSE", day(2025, 4, 9), "ETERNAL"),
        ("LTI.NSE", day(2021, 3, 1), "LTM"),
        ("LTIM.NSE", day(2023, 3, 1), "LTM"),
        ("LTM.NSE", day(2026, 3, 2), "LTM"),
        ("MOTHERSUMI.NSE", day(2022, 6, 8), "MOTHERSON"),
        ("HDFC.NSE", day(2022, 6, 15), "HDFC"),
        ("PVR.NSE", day(2022, 6, 15), "PVRINOX"),
        ("RELIANCE.NSE", day(2022, 6, 15), "RELIANCE"),
    ] {
        assert_eq!(
            stored.filing(CmKind::Stocks, stem, d),
            Ok(Some(cash(key))),
            "{stem} {d}"
        );
    }
    for (stem, d, after) in [
        ("TATAMOTORS.NSE", day(2025, 10, 24), true),
        ("TMPV.NSE", day(2025, 10, 23), false),
        ("LTI.NSE", day(2023, 3, 1), true),
        ("LTM.NSE", day(2023, 3, 1), false),
    ] {
        match stored.filing(CmKind::Stocks, stem, d) {
            Err(why @ CmRefusal::SymbolNotInUse { stale }) => {
                assert_eq!(matches!(stale, Stale::AfterChange(_)), after, "{stem}");
                assert!(why.to_string().contains("was not the exchange's symbol"));
                assert_eq!(
                    why.filed_as(),
                    Some(alias::LINEAGE.current(stem.trim_end_matches(".NSE")))
                );
            }
            other => panic!("{stem} {d}: {other:?}"),
        }
    }
    // The swept scope is today's surface: no old name, no stored share.
    for stem in ["TATAMOTORS.NSE", "HDFC.NSE", "LTI.NSE", "ZOMATO.NSE"] {
        assert_eq!(
            swept.filing(CmKind::Stocks, stem, day(2022, 6, 15)),
            Ok(None),
            "{stem}"
        );
    }
    // An index underlying is the index import's, and a series file or a name
    // off the scope is no share's.
    for stem in [
        "NIFTYIT.NSE",
        "NIFTY.NSE",
        "HDFC.SM.NSE",
        "ACC.BE.NSE",
        "ACC.NSE",
        "hdfc.NSE",
    ] {
        assert_eq!(
            stored.filing(CmKind::Stocks, stem, day(2022, 6, 15)),
            Ok(None),
            "{stem}"
        );
    }
    assert!(stored.shares().iter().any(|s| &**s == "HDFC"));
    assert!(!stored.shares().iter().any(|s| &**s == "NIFTYIT"));
    assert!(CmRefusal::NotSwept.filed_as().is_none());
}

/// `stem_for` inverts `filing` on every day of a chain: a key's file is the
/// stem the exchange used that day, and filing that stem gives the key back.
#[test]
fn stem_for_inverts_filing_on_every_day_of_a_chain() {
    let stored = CmScope::stored(["HDFC", "TATAMOTORS"]).unwrap();
    let cash = |s: &str| InstrumentKey::cash(Exchange::Nse, s).unwrap();
    for (key, d, stem) in [
        ("TMPV", day(2022, 6, 15), "TATAMOTORS.NSE"),
        ("TMPV", day(2025, 10, 24), "TMPV.NSE"),
        ("LTM", day(2021, 3, 1), "LTI.NSE"),
        ("LTM", day(2023, 3, 1), "LTIM.NSE"),
        ("LTM", day(2026, 3, 2), "LTM.NSE"),
        ("HDFC", day(2022, 6, 15), "HDFC.NSE"),
        ("RELIANCE", day(2022, 6, 15), "RELIANCE.NSE"),
    ] {
        let got = stored.stem_for(&cash(key), d);
        assert_eq!(got, Some((CmKind::Stocks, stem.to_owned())), "{key} {d}");
        assert_eq!(
            stored.filing(CmKind::Stocks, stem, d),
            Ok(Some(cash(key))),
            "{key} {d}"
        );
    }
    // A key the scope does not file has no stem; the swept scope answers the
    // swept surface's own inverse.
    assert_eq!(stored.stem_for(&cash("ACC"), day(2022, 6, 15)), None);
    assert_eq!(
        CmScope::swept().stem_for(&cash("TMPV"), day(2022, 6, 15)),
        stem_of(&cash("TMPV"))
    );
    let bse = InstrumentKey::cash(Exchange::Bse, "RELIANCE").unwrap();
    assert_eq!(stored.stem_for(&bse, day(2022, 6, 15)), None);
}

/// A name the store cannot file under its own spelling is refused by name
/// before any run begins.
#[test]
fn a_scope_refuses_a_name_the_store_cannot_file() {
    for bad in ["hdfc", "", "M&MFIN.BE", "ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFG"] {
        let why = CmScope::stored([bad]).unwrap_err();
        assert!(why.contains("not a share symbol"), "{bad}: {why}");
    }
    let scope = CmScope::stored(["HDFC", "HDFC", "RELIANCE"]).unwrap();
    assert_eq!(
        scope.shares().iter().filter(|s| &***s == "HDFC").count(),
        1,
        "named once"
    );
    assert!(
        !scope.shares().iter().any(|s| &**s == "RELIANCE"),
        "a swept share keeps its swept slot"
    );
    assert!(scope.is_stored() && !CmScope::swept().is_stored());
    let with = scope.with_masters(PathBuf::from("/m"));
    assert_eq!(with.masters(), Some(Path::new("/m")));
}

/// A stored listing files stored stems past the ticker map (twins refused as
/// for a swept one) and counts every other name.
#[test]
fn a_stored_listing_files_stored_stems_and_counts_the_rest() {
    let scope = Arc::new(CmScope::stored(["HDFC"]).unwrap());
    let d = day(2022, 6, 15);
    let mut listing: DayListing<()> = DayListing::scoped(CmKind::Stocks, d, Arc::clone(&scope));
    let folder = listing.folder().to_owned();
    for name in [
        "HDFC.NSE.csv",
        "TATAMOTORS.NSE.csv",
        "TATAMOTORS.NSE.CSV",
        "RELIANCE.NSE.csv",
        "ACC.NSE.csv",
        "ACC.BE.NSE.csv",
        "HDFC.SM.NSE.csv",
        "SUZLON.BE.NSE.csv",
    ] {
        listing.push(&format!("{folder}/{name}"), 0, 0, ());
    }
    assert_eq!(
        listing.unresolved(),
        3,
        "ACC, its series file, and a series that is no trade-for-trade one"
    );
    // A trade-for-trade spell's file sits in its share's slot (WP-12f,
    // DB-B06) and is filed under the share's key.
    assert_eq!(
        listing
            .locate("SUZLON.NSE")
            .map(|f| f.map(|(file, _)| file.name().to_owned())),
        Ok(Some("SUZLON.BE.NSE.csv".to_owned()))
    );
    assert_eq!(
        listing.filing("SUZLON.BE.NSE"),
        listing.filing("SUZLON.NSE")
    );
    assert!(listing.filing("SUZLON.BE.NSE").unwrap().is_some());
    assert_eq!(listing.filing("HDFC.SM.NSE"), Ok(None));
    assert_eq!(
        listing.member("SUZLON.BE.NSE").as_deref(),
        Some("NSE-CASH-SUZLON.BE")
    );
    assert_eq!(
        listing
            .locate("HDFC.NSE")
            .map(|f| f.map(|(file, _)| file.name().to_owned())),
        Ok(Some("HDFC.NSE.csv".to_owned()))
    );
    assert_eq!(
        listing.locate("TATAMOTORS.NSE").map(|f| f.is_some()),
        Err(CmRefusal::AmbiguousCaseTwins)
    );
    assert_eq!(listing.locate("ACC.NSE").map(|f| f.is_some()), Ok(false));
    assert!(listing.locate("RELIANCE.NSE").unwrap().is_some());
    assert_eq!(
        listing.filing("HDFC.NSE"),
        Ok(Some(InstrumentKey::cash(Exchange::Nse, "HDFC").unwrap()))
    );
    assert_eq!(scope.extra_slots(CmKind::Indices), 7);
    // HDFC's extra slot is its place among the stored shares (after the
    // symbol-change rows' names, which the scope adds first).
    let hdfc = scope.shares().iter().position(|s| &**s == "HDFC");
    assert!(hdfc.is_some());
    assert_eq!(scope.extra_slot(CmKind::Stocks, "HDFC.NSE"), hdfc);
    assert_eq!(scope.extra_slot(CmKind::Stocks, "HDFC"), None);
    let long = format!("{}.NSE", "A".repeat(SYMBOL_CAPACITY + 1));
    assert_eq!(scope.extra_slot(CmKind::Stocks, &long), None);
    assert_eq!(scope.filing(CmKind::Stocks, &long, d), Ok(None));
    // An index listing of the swept scope still counts INDIA VIX.
    let mut swept: DayListing<()> = DayListing::new(CmKind::Indices, d);
    let folder = swept.folder().to_owned();
    swept.push(&format!("{folder}/INDIA VIX.NSE_IDX.csv"), 0, 0, ());
    assert_eq!(swept.unresolved(), 1);
    assert_eq!(swept.scope(), &CmScope::swept());
}

/// INDIA VIX's four-decimal LTP is snapped half-up once under its rule
/// (`CLAUDE.md` §7) and counted; a two-place LTP is not; a value that
/// snaps below one tick is refused.
#[test]
fn india_vix_is_snapped_half_up_once_and_counted() {
    let d = day(2024, 4, 1);
    let stem = "INDIA VIX.NSE_IDX";
    let rules = Rules {
        price: PriceRule::HalfUp,
        ..Rules::regular()
    };
    let rows = [
        row(stem, d, at(9, 15, 0), "61.4175", 0, 0),
        row(stem, d, at(9, 15, 1), "61.4125", 0, 0),
        row(stem, d, at(9, 15, 2), "61.40", 0, 0),
        row(stem, d, at(9, 15, 3), "0.0049", 0, 0),
        row(stem, d, at(15, 30, 0), "61.4", 0, 0),
    ];
    let file = decode_with(&csv(&rows), &expect_of(CmKind::Indices, stem, d), &rules).unwrap();
    let ltps: Vec<i64> = file.rows.iter().map(|r| r.ltp).collect();
    assert_eq!(ltps, vec![6_142, 6_141, 6_140, 6_140]);
    assert_eq!(file.rows_snapped, 2);
    assert_eq!(file.refused.off_grid, 1, "0.0049 snaps to zero paisa");
}

/// End to end: a stored index run files NIFTY, NIFTY FIN SERVICE and INDIA
/// VIX (snapped) under their store keys and skips an index off the table; a
/// swept run of the same day files NIFTY alone; `--stored`'s filter reads the
/// stored ones only.
#[test]
fn a_stored_index_run_files_the_alias_indices_and_india_vix() {
    let root = scratch("wp12a-indices");
    let d = day(2024, 4, 1);
    let file = |stem: &str, base: &str| {
        let mut rows: Vec<String> = (0..20)
            .map(|i| {
                row(
                    stem,
                    d,
                    at(9, 15, 0) + i,
                    &format!("{base}{:02}", i % 100),
                    0,
                    0,
                )
            })
            .collect();
        rows.push(row(stem, d, at(15, 31, 0), &format!("{base}00"), 0, 0));
        csv(&rows)
    };
    ts_day(
        &root,
        CmKind::Indices,
        d,
        &[
            (
                "NIFTY 50.NSE_IDX.csv".to_owned(),
                file("NIFTY 50.NSE_IDX", "77001."),
            ),
            (
                "NIFTY FIN SERVICE.NSE_IDX.csv".to_owned(),
                file("NIFTY FIN SERVICE.NSE_IDX", "88002."),
            ),
            (
                "INDIA VIX.NSE_IDX.csv".to_owned(),
                file("INDIA VIX.NSE_IDX", "61.41"),
            ),
            (
                "NIFTY 100.NSE_IDX.csv".to_owned(),
                file("NIFTY 100.NSE_IDX", "99003."),
            ),
        ],
    );
    let stored_scope = || CmScope::stored(std::iter::empty::<&str>()).unwrap();
    let a = root.join("A");
    let got = import(&root, &a, (ImportKind::Indices, d, d), stored_scope(), &[]);
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!((got.files, got.files_skipped), (3, 1));
    assert_eq!(held(&a, "INDEX", "NIFTY", d).unwrap().len(), 20);
    assert_eq!(held(&a, "INDEX", "NIFTYFINSERVICE", d).unwrap().len(), 20);
    let vix = held(&a, "INDEX", "INDIAVIX", d).unwrap();
    assert_eq!(vix.len(), 20);
    // `61.4100` .. `61.4119`: every value snapped half-up to 61.41 or 61.42.
    assert!(vix.iter().all(|b| b.close == 6_141 || b.close == 6_142));
    assert!(held(&a, "INDEX", "NIFTY100", d).is_none());
    let b = root.join("B");
    let swept = import(
        &root,
        &b,
        (ImportKind::Indices, d, d),
        CmScope::swept(),
        &[],
    );
    assert_eq!((swept.files, swept.files_skipped), (1, 3));
    assert!(held(&b, "INDEX", "NIFTYFINSERVICE", d).is_none());
    // A run filtered to the stored keys reads them alone, under its own
    // journal key, on a store that already holds the swept day.
    let only = vec!["INDIAVIX".to_owned(), "NIFTYFINSERVICE".to_owned()];
    let again = import(
        &root,
        &b,
        (ImportKind::Indices, d, d),
        stored_scope(),
        &only,
    );
    assert_eq!((again.days_skipped, again.files), (0, 2));
    assert_eq!(held(&b, "INDEX", "INDIAVIX", d).unwrap().len(), 20);
    std::fs::remove_dir_all(root).unwrap();
}

/// End to end (DB-069, DB-071): on 2022-06-08 a stored share run files
/// TATAMOTORS's file under TMPV, MOTHERSUMI's (renamed the next day) under
/// MOTHERSON and HDFC's under HDFC; refuses by name a TMPV file the exchange
/// could not have printed that day; skips a share off the scope. A run
/// filtered to HDFC skips the refused stem instead of refusing it.
#[test]
fn a_stored_share_run_files_old_names_under_their_current_keys() {
    let root = scratch("wp12a-shares");
    let d = day(2022, 6, 8);
    let file = |stem: &str| {
        let rows: Vec<String> = (0..12)
            .map(|i| {
                row(
                    stem,
                    d,
                    at(9, 20, 0) + i * 30,
                    &format!("432{i}.05"),
                    1 + u64::from(i),
                    0,
                )
            })
            .collect();
        csv(&rows)
    };
    let names = ["TATAMOTORS", "MOTHERSUMI", "HDFC", "TMPV", "ACC"];
    let files: Vec<(String, Vec<u8>)> = names
        .iter()
        .map(|n| (format!("{n}.NSE.csv"), file(&format!("{n}.NSE"))))
        .collect();
    ts_day(&root, CmKind::Stocks, d, &files);
    let scope = || CmScope::stored(["HDFC", "TATAMOTORS", "MOTHERSUMI"]).unwrap();
    let store = root.join("S");
    let got = import(&root, &store, (ImportKind::Stocks, d, d), scope(), &[]);
    assert_eq!((got.files, got.files_refused, got.files_skipped), (3, 1, 1));
    assert_eq!(got.failures.len(), 1);
    assert!(got.failures[0].why.contains("TMPV"), "{:?}", got.failures);
    assert_eq!(got.closed_with_refusals, vec![d], "the vendor's own bytes");
    for symbol in ["TMPV", "MOTHERSON", "HDFC"] {
        assert_eq!(
            held(&store, "CASH", symbol, d).unwrap().len(),
            12,
            "{symbol}"
        );
    }
    for symbol in ["TATAMOTORS", "MOTHERSUMI", "ACC"] {
        assert!(held(&store, "CASH", symbol, d).is_none(), "{symbol}");
    }
    let filtered = root.join("F");
    let only = vec!["HDFC".to_owned()];
    let got = import(&root, &filtered, (ImportKind::Stocks, d, d), scope(), &only);
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!((got.files, got.files_skipped), (1, 4));
    std::fs::remove_dir_all(root).unwrap();
}

// ── D-3335: same-second dual values, counted (DB-G96) ───────────────────

/// Three in-session seconds whose two rows span more than 10 bps of the
/// lower value are counted, with the widest and its second; a narrower
/// spread, a pre-open spread and a stock file count nothing; the rows are
/// all kept as written.
#[test]
fn same_second_dual_values_are_counted_never_altered() {
    let d = day(2019, 3, 28);
    let stem = "NIFTY 50.NSE_IDX";
    let mut rows = vec![
        row(stem, d, at(9, 10, 0), "70000.00", 0, 0),
        row(stem, d, at(9, 10, 0), "70500.00", 0, 0),
    ];
    rows.extend(index_rows(stem, d, at(9, 15, 0), 10));
    for (i, (low, high)) in [
        ("70000.20", "70154.95"),
        ("70000.75", "70170.05"),
        ("70001.50", "70100.20"),
    ]
    .iter()
    .enumerate()
    {
        let sod = at(13, 30, 29) + u32::try_from(i).unwrap();
        rows.push(row(stem, d, sod, low, 0, 0));
        rows.push(row(stem, d, sod, high, 0, 0));
    }
    // 6.9 bps: not counted.
    rows.push(row(stem, d, at(13, 31, 0), "70000.00", 0, 0));
    rows.push(row(stem, d, at(13, 31, 0), "70048.30", 0, 0));
    rows.push(row(stem, d, at(15, 30, 0), "70000.00", 0, 0));
    let file = decode(&csv(&rows), &expect_of(CmKind::Indices, stem, d)).unwrap();
    assert_eq!(file.rows.len(), rows.len(), "nothing dropped");
    assert_eq!(
        file.dual_spread,
        DualSpread {
            seconds: 3,
            max_bps: 24,
            max_at: Some(at(13, 30, 30)),
        }
    );
    let stock = "SBIN.NSE";
    let rows = [
        row(stock, d, at(10, 0, 0), "4000.00", 5, 0),
        row(stock, d, at(10, 0, 0), "4100.00", 5, 0),
    ];
    let file = decode(&csv(&rows), &expect_of(CmKind::Stocks, stock, d)).unwrap();
    assert_eq!(file.dual_spread, DualSpread::default());
}

// ── D-3320: the closing auction's print, kept apart from trades ──────────

/// A stock file of 2026-09-24, invented: continuous trades to 15:14, one
/// trade at 15:20 with quotes, an auction print at 15:28:07 stated twice,
/// and a post-close row with zero quotes.
fn cas_file(stem: &str, d: Day) -> Vec<u8> {
    let mut rows: Vec<String> = (0..10)
        .map(|i| {
            quoted(
                stem,
                d,
                at(15, 5, 0) + i * 50,
                &format!("4444.{i}5"),
                ("4444.00", "4444.90"),
                3,
            )
        })
        .collect();
    rows.push(quoted(
        stem,
        d,
        at(15, 20, 0),
        "4444.55",
        ("4444.50", "4444.60"),
        2,
    ));
    rows.push(quoted(
        stem,
        d,
        at(15, 28, 7),
        "4446.65",
        ("0", "0.00"),
        98_765,
    ));
    rows.push(quoted(
        stem,
        d,
        at(15, 28, 7),
        "4446.65",
        ("0", "0.00"),
        98_765,
    ));
    rows.push(quoted(stem, d, at(15, 28, 7), "4446.70", ("0", "0"), 11));
    rows.push(quoted(stem, d, at(15, 29, 0), "4446.65", ("0", "0"), 0));
    rows.push(quoted(stem, d, at(15, 52, 0), "4446.65", ("0", "0"), 7));
    csv(&rows)
}

/// An eligible share's candidates are diverted to auction prints (identical
/// consecutive ones merged, never summed); an ineligible share's are rows;
/// an unknown eligibility refuses the file by name; and a day before
/// 2026-08-03 asks nothing.
#[test]
fn an_eligible_shares_auction_rows_are_prints_never_trades() {
    let d = day(2026, 9, 24);
    let stem = "RELIANCE.NSE";
    let bytes = cas_file(stem, d);
    let expect = expect_of(CmKind::Stocks, stem, d);
    let with = |auction: Auction| {
        decode_with(
            &bytes,
            &expect,
            &Rules {
                auction,
                ..Rules::regular()
            },
        )
    };
    let eligible = with(Auction::Eligible).unwrap();
    assert_eq!(eligible.rows.len(), 13, "16 rows, three of them prints");
    assert!(
        eligible.rows.iter().all(|r| r.sod != at(15, 28, 7)),
        "no print is a row"
    );
    assert_eq!(
        eligible.auction,
        vec![
            auction::Print {
                sod: at(15, 28, 7),
                price: 444_665,
                quantity: 98_765,
                rows: 2,
                line: 13,
            },
            auction::Print {
                sod: at(15, 28, 7),
                price: 444_670,
                quantity: 11,
                rows: 1,
                line: 15,
            },
        ]
    );
    // The zero-LTQ row in the band and the post-close row stay rows.
    assert!(eligible.rows.iter().any(|r| r.sod == at(15, 29, 0)));
    assert!(eligible.rows.iter().any(|r| r.sod == at(15, 52, 0)));
    for auction in [Auction::NotEligible, Auction::NotAsked] {
        let file = with(auction).unwrap();
        assert_eq!(file.rows.len(), 16);
        assert!(file.auction.is_empty());
    }
    assert_eq!(
        with(Auction::Unknown("no master".to_owned())),
        Err(CmRefusal::AuctionEligibilityUnknown {
            why: "no master".to_owned()
        })
    );
    // A file with no candidate row never asks the eligibility.
    let quiet = csv(&[quoted(
        stem,
        d,
        at(10, 0, 0),
        "4444.05",
        ("4444.00", "4444.10"),
        3,
    )]);
    assert!(
        decode_with(
            &quiet,
            &expect,
            &Rules {
                auction: Auction::Unknown("no master".to_owned()),
                ..Rules::regular()
            }
        )
        .is_ok()
    );
    // A file whose one session trade is the auction is still a file.
    let only_print = csv(&[quoted(stem, d, at(15, 28, 7), "4446.65", ("0", "0"), 5)]);
    let file = decode_with(
        &only_print,
        &expect,
        &Rules {
            auction: Auction::Eligible,
            ..Rules::regular()
        },
    )
    .unwrap();
    assert_eq!((file.rows.len(), file.auction.len()), (0, 1));
}

/// SC-CLOSE-5: the auction's band runs through its order-matching phase,
/// `[15:15:00, 15:35:00)` (charter §9, the phases of the dated notice §7
/// cites), not to 15:30:00. A zero-book traded row of an eligible share at
/// 15:30:00..15:34:59 is a print (the vendor stamped 9 of 145 measured closing
/// prints 15:30:00..15:30:02, and they were lost); 15:35:00 and the
/// post-close session's zero-book rows (15:50 on) stay rows, never prints; a
/// row with a book in the band stays a row; an ineligible share keeps every
/// row; an unknown eligibility refuses a file whose only candidate is at
/// 15:30:01. Every price is invented.
#[test]
fn the_auction_band_runs_through_order_matching_and_stops_before_post_close() {
    let d = day(2026, 8, 20);
    let stem = "RELIANCE.NSE";
    let zero = ("0", "0.00");
    let mut rows: Vec<String> = (0..6)
        .map(|i| {
            quoted(
                stem,
                d,
                at(15, 5, 0) + i * 60,
                "4444.15",
                ("4444.10", "4444.20"),
                3,
            )
        })
        .collect();
    rows.push(quoted(stem, d, at(15, 29, 6), "4446.65", zero, 101));
    rows.push(quoted(stem, d, at(15, 30, 0), "4446.65", zero, 102));
    rows.push(quoted(stem, d, at(15, 30, 1), "4446.65", zero, 103));
    rows.push(quoted(
        stem,
        d,
        at(15, 30, 5),
        "4446.60",
        ("4446.55", "4446.70"),
        9,
    ));
    rows.push(quoted(stem, d, at(15, 31, 46), "4446.65", zero, 104));
    rows.push(quoted(stem, d, at(15, 34, 59), "4446.65", zero, 105));
    rows.push(quoted(stem, d, at(15, 35, 0), "4446.65", zero, 106));
    rows.push(quoted(stem, d, at(15, 58, 42), "4446.65", zero, 107));
    let bytes = csv(&rows);
    let expect = expect_of(CmKind::Stocks, stem, d);
    let with = |auction: Auction, bytes: &[u8]| {
        decode_with(
            bytes,
            &expect,
            &Rules {
                auction,
                ..Rules::regular()
            },
        )
    };
    let eligible = with(Auction::Eligible, &bytes).unwrap();
    let prints: Vec<(u32, u64)> = eligible
        .auction
        .iter()
        .map(|p| (p.sod, p.quantity))
        .collect();
    assert_eq!(
        prints,
        vec![
            (at(15, 29, 6), 101),
            (at(15, 30, 0), 102),
            (at(15, 30, 1), 103),
            (at(15, 31, 46), 104),
            (at(15, 34, 59), 105),
        ],
        "every zero-book traded row in [15:15:00, 15:35:00) is a print"
    );
    let kept: Vec<u32> = eligible.rows.iter().map(|r| r.sod).collect();
    for sod in [at(15, 30, 5), at(15, 35, 0), at(15, 58, 42)] {
        assert!(kept.contains(&sod), "{sod} stays a row, never a print");
    }
    assert_eq!(eligible.rows.len() + eligible.auction.len(), rows.len());
    for auction in [Auction::NotEligible, Auction::NotAsked] {
        let file = with(auction, &bytes).unwrap();
        assert_eq!((file.rows.len(), file.auction.len()), (rows.len(), 0));
    }
    let late_only = csv(&[
        quoted(stem, d, at(15, 5, 0), "4444.15", ("4444.10", "4444.20"), 3),
        quoted(stem, d, at(15, 30, 1), "4446.65", zero, 103),
    ]);
    assert_eq!(
        with(Auction::Unknown("no master".to_owned()), &late_only),
        Err(CmRefusal::AuctionEligibilityUnknown {
            why: "no master".to_owned()
        }),
        "a candidate at 15:30:01 asks the eligibility like any other"
    );
    let post_only = csv(&[
        quoted(stem, d, at(15, 5, 0), "4444.15", ("4444.10", "4444.20"), 3),
        quoted(stem, d, at(15, 35, 0), "4446.65", zero, 106),
    ]);
    assert!(
        with(Auction::Unknown("no master".to_owned()), &post_only).is_ok(),
        "a zero-book row from 15:35:00 is no candidate and asks nothing"
    );
}

/// The day, stem and quotes of the SC-CLOSE-6 shapes. Every price is invented.
const BAND_STEM: &str = "HDFCBANK.NSE";
const BAND_ZERO: (&str, &str) = ("0", "0.00");
const BAND_BOOK: (&str, &str) = ("4446.60", "4446.70");

fn band_day() -> Day {
    day(2026, 9, 24)
}

/// Lines 2..=7 of every shape: six continuous trades before 15:15.
fn band_base() -> Vec<String> {
    (0..6)
        .map(|i| {
            quoted(
                BAND_STEM,
                band_day(),
                at(15, 5, 0) + i * 60,
                "4444.15",
                ("4444.10", "4444.20"),
                3,
            )
        })
        .collect()
}

/// One row of each `(second, price, quotes, quantity)` after the base.
fn band_file(tail: &[(u32, &str, (&str, &str), u64)]) -> Vec<u8> {
    let mut rows = band_base();
    for &(sod, ltp, quotes, ltq) in tail {
        rows.push(quoted(BAND_STEM, band_day(), sod, ltp, quotes, ltq));
    }
    csv(&rows)
}

fn band_decode(bytes: &[u8], auction: Auction) -> Result<CmFile, CmRefusal> {
    decode_with(
        bytes,
        &expect_of(CmKind::Stocks, BAND_STEM, band_day()),
        &Rules {
            auction,
            ..Rules::regular()
        },
    )
}

/// `(second, price, quantity, rows, line)` of each print.
fn band_prints(file: &CmFile) -> Vec<(u32, i64, u64, u32, u32)> {
    file.auction
        .iter()
        .map(|p| (p.sod, p.price, p.quantity, p.rows, p.line))
        .collect()
}

/// A booked match alone, between a 15:15 straggler and a post-close row
/// (40 of 133 measured share-sessions had no zero-book row in the band).
fn booked_alone() -> Vec<u8> {
    band_file(&[
        (at(15, 15, 2), "4444.25", BAND_BOOK, 4),
        (at(15, 29, 5), "4446.65", BAND_BOOK, 703),
        (at(15, 52, 0), "4446.65", BAND_BOOK, 9),
    ])
}

/// One match stated booked, then zero-book twice, then booked after 15:30,
/// then a booked row at 15:35:00 (HDFCBANK 2026-09-24's shape, widened).
fn booked_stated() -> Vec<u8> {
    band_file(&[
        (at(15, 28, 3), "4446.65", BAND_BOOK, 383),
        (at(15, 28, 7), "4446.65", BAND_ZERO, 383),
        (at(15, 28, 7), "4446.65", BAND_ZERO, 383),
        (at(15, 31, 46), "4446.65", BAND_BOOK, 383),
        (at(15, 35, 0), "4446.65", BAND_BOOK, 5),
    ])
}

/// SC-CLOSE-6: the auction's match is stamped in `[15:28:00, 15:35:00)` at
/// one price (charter §9: a random close from 15:28, one equilibrium price,
/// matching to 15:35), and the vendor states it with a book beside it as
/// often as without. When an eligible file's match band trades at one price,
/// its booked rows are prints too, merged with the zero-book ones in file
/// order (identical consecutive rows merged, never summed), and the row
/// counts follow them out; 15:35:00 and the post-close session stay rows.
#[test]
fn a_booked_match_at_the_one_band_price_is_a_print() {
    let file = band_decode(&booked_alone(), Auction::Eligible).unwrap();
    assert_eq!(
        band_prints(&file),
        vec![(at(15, 29, 5), 444_665, 703, 1, 9)]
    );
    assert_eq!(file.rows.len(), 8, "nine rows, one of them the print");
    assert!(file.rows.iter().all(|r| r.sod != at(15, 29, 5)));
    assert_eq!((file.rows_in_session, file.rows_post), (7, 1));
    assert_eq!(file.max_sod, at(15, 52, 0));
    assert_eq!(file.auction_band_unattributed, 0);
    let file = band_decode(&booked_alone(), Auction::NotEligible).unwrap();
    assert_eq!((file.rows.len(), file.auction.len()), (9, 0));
    assert_eq!((file.rows_in_session, file.rows_post), (8, 1));

    let file = band_decode(&booked_stated(), Auction::Eligible).unwrap();
    assert_eq!(
        band_prints(&file),
        vec![
            (at(15, 28, 3), 444_665, 383, 1, 8),
            (at(15, 28, 7), 444_665, 383, 2, 9),
            (at(15, 31, 46), 444_665, 383, 1, 11),
        ]
    );
    let kept: Vec<u32> = file.rows.iter().map(|r| r.sod).collect();
    assert_eq!(kept.len(), 7);
    assert_eq!(kept.last(), Some(&at(15, 35, 0)), "15:35:00 stays a row");
    assert_eq!((file.rows_in_session, file.rows_post), (6, 1));
}

/// SC-CLOSE-6, the other shapes: a band at two prices (RELIANCE
/// 2026-08-31's) claims no booked row and counts them, its zero-book row
/// still a print (D-3320 item 3); a file whose one trade is a booked match
/// after 15:30 is one print for an eligible share and has no session row
/// otherwise; before 2026-08-03 nothing is asked; and an unknown eligibility
/// is asked only by a zero-book candidate, as before.
#[test]
fn two_band_prices_claim_no_booked_row_and_the_edges_hold() {
    let two = band_file(&[
        (at(15, 20, 7), "4445.00", BAND_BOOK, 2),
        (at(15, 28, 45), "4445.50", BAND_BOOK, 3),
        (at(15, 29, 52), "4446.65", BAND_BOOK, 900),
        (at(15, 29, 53), "4446.65", BAND_ZERO, 900),
    ]);
    let file = band_decode(&two, Auction::Eligible).unwrap();
    assert_eq!(
        band_prints(&file),
        vec![(at(15, 29, 53), 444_665, 900, 1, 11)]
    );
    assert_eq!(file.auction_band_unattributed, 2);
    for sod in [at(15, 20, 7), at(15, 28, 45), at(15, 29, 52)] {
        assert!(file.rows.iter().any(|r| r.sod == sod), "{sod} stays a row");
    }
    assert_eq!(file.rows.len(), 9);

    let late = csv(&[quoted(
        BAND_STEM,
        band_day(),
        at(15, 31, 0),
        "4446.65",
        BAND_BOOK,
        77,
    )]);
    let file = band_decode(&late, Auction::Eligible).unwrap();
    assert_eq!(band_prints(&file), vec![(at(15, 31, 0), 444_665, 77, 1, 2)]);
    assert_eq!((file.rows.len(), file.rows_post), (0, 0));
    assert_eq!(
        band_decode(&late, Auction::NotEligible),
        Err(CmRefusal::NoSessionRows)
    );

    let file = band_decode(&booked_stated(), Auction::NotAsked).unwrap();
    assert_eq!((file.rows.len(), file.auction.len()), (11, 0));
}

/// SC-CLOSE-6 under an unknown eligibility: a booked match at one band price
/// is the auction's print if the share is eligible and a trade if not, so the
/// file is refused by name, never folded on a guess (it was kept, its print
/// a trade, before); a zero-book candidate refuses as before; a band at two
/// prices keeps its booked rows as rows whatever the flag, so nothing is
/// guessed and the file is kept; a file with no traded row in the band, or
/// none in it at all, asks nothing.
#[test]
fn an_unknown_eligibility_refuses_a_one_price_booked_match_and_keeps_two_prices() {
    let unknown = || Auction::Unknown("no master".to_owned());
    let refused = Err(CmRefusal::AuctionEligibilityUnknown {
        why: "no master".to_owned(),
    });
    assert_eq!(band_decode(&booked_alone(), unknown()), refused);
    assert_eq!(band_decode(&booked_stated(), unknown()), refused);
    let two_booked = band_file(&[
        (at(15, 28, 45), "4445.50", BAND_BOOK, 3),
        (at(15, 29, 52), "4446.65", BAND_BOOK, 900),
    ]);
    let file = band_decode(&two_booked, unknown()).unwrap();
    assert_eq!((file.rows.len(), file.auction.len()), (8, 0));
    assert_eq!(file.auction_band_unattributed, 0);
    let file = band_decode(&two_booked, Auction::Eligible).unwrap();
    assert_eq!((file.rows.len(), file.auction.len()), (8, 0));
    assert_eq!(file.auction_band_unattributed, 2);
    let before_band = band_file(&[(at(15, 27, 59), "4446.65", BAND_BOOK, 9)]);
    let file = band_decode(&before_band, unknown()).unwrap();
    assert_eq!((file.rows.len(), file.auction.len()), (7, 0));
    let quote_only = band_file(&[(at(15, 29, 0), "4446.65", BAND_BOOK, 0)]);
    assert!(band_decode(&quote_only, unknown()).is_ok());
}

/// A booked print leaves the count its row was put in, by the one rule that
/// put it there: on a session whose only window opens after the match band
/// (a special session, invented), the band's row was counted before the
/// session, so the print takes it out of `rows_pre`, never out of
/// `rows_post` (which it never entered), and counts as usable.
#[test]
fn a_booked_print_leaves_the_count_its_row_was_put_in() {
    let late_session = crate::calendar::Session {
        windows: [
            crate::calendar::Window {
                from: 18 * 60,
                to: 18 * 60 + 59,
            },
            crate::calendar::Window { from: 0, to: 0 },
        ],
        count: 1,
    };
    let rules = |auction: Auction| Rules {
        session: late_session,
        auction,
        ..Rules::regular()
    };
    let bytes = csv(&[
        quoted(
            BAND_STEM,
            band_day(),
            at(15, 29, 5),
            "4446.65",
            BAND_BOOK,
            7,
        ),
        quoted(BAND_STEM, band_day(), at(18, 5, 0), "4450.05", BAND_BOOK, 2),
        quoted(
            BAND_STEM,
            band_day(),
            at(19, 30, 0),
            "4450.05",
            BAND_BOOK,
            1,
        ),
    ]);
    let expect = expect_of(CmKind::Stocks, BAND_STEM, band_day());
    let file = decode_with(&bytes, &expect, &rules(Auction::NotEligible)).unwrap();
    assert_eq!(
        (file.rows_pre, file.rows_in_session, file.rows_post),
        (1, 1, 1)
    );
    let file = decode_with(&bytes, &expect, &rules(Auction::Eligible)).unwrap();
    assert_eq!(band_prints(&file), vec![(at(15, 29, 5), 444_665, 7, 1, 2)]);
    assert_eq!(
        (file.rows_pre, file.rows_in_session, file.rows_post),
        (0, 1, 1),
        "the print leaves rows_pre, where its row was counted"
    );
    assert_eq!(file.rows.len(), 2);
    assert_eq!(file.max_sod, at(19, 30, 0));
}

/// The match band costs O(1) a row and one pass a file (SC-CLOSE-6),
/// measured: a 30,000-row eligible share-day with a booked match decoded 200
/// times as eligible and 200 times as ineligible, interleaved; p50, p99 and
/// max of each printed, and the eligible decode's median held within twice
/// the ineligible one's plus a millisecond. A timing, so run on demand:
/// `cargo test -p pull --lib -- --ignored the_match_band_costs --nocapture`.
/// This test is the proof of the claim it states:
/// `pull::gdfl_cm::admission_tests::the_match_band_costs_one_pass_a_file`.
#[test]
#[ignore = "timing; run on demand with --ignored"]
fn the_match_band_costs_one_pass_a_file() {
    let mut rows: Vec<String> = (0..30_000_u32)
        .map(|i| {
            quoted(
                BAND_STEM,
                band_day(),
                at(9, 15, 0) + i * 21_599 / 30_000,
                "4444.15",
                ("4444.10", "4444.20"),
                3,
            )
        })
        .collect();
    for (sod, quotes) in [
        (at(15, 28, 3), BAND_BOOK),
        (at(15, 28, 7), BAND_ZERO),
        (at(15, 30, 1), BAND_BOOK),
    ] {
        rows.push(quoted(BAND_STEM, band_day(), sod, "4446.65", quotes, 383));
    }
    rows.push(quoted(
        BAND_STEM,
        band_day(),
        at(15, 52, 0),
        "4446.65",
        BAND_BOOK,
        9,
    ));
    let bytes = csv(&rows);
    let mut eligible = Vec::with_capacity(200);
    let mut ineligible = Vec::with_capacity(200);
    for _ in 0..200 {
        let start = std::time::Instant::now();
        let file = band_decode(&bytes, Auction::Eligible).unwrap();
        eligible.push(start.elapsed());
        assert_eq!(file.auction.len(), 3);
        let start = std::time::Instant::now();
        let file = band_decode(&bytes, Auction::NotEligible).unwrap();
        ineligible.push(start.elapsed());
        assert_eq!(file.rows.len(), rows.len());
    }
    let stats = |runs: &mut [std::time::Duration]| {
        runs.sort_unstable();
        (
            runs[runs.len() / 2],
            runs[runs.len() * 99 / 100],
            runs[runs.len() - 1],
        )
    };
    let (e50, e99, emax) = stats(&mut eligible);
    let (n50, n99, nmax) = stats(&mut ineligible);
    println!(
        "decode of {} rows, n=200 each: eligible p50={e50:?} p99={e99:?} max={emax:?}; \
         ineligible p50={n50:?} p99={n99:?} max={nmax:?}",
        rows.len()
    );
    assert!(
        e50 <= n50 * 2 + std::time::Duration::from_millis(1),
        "the band's settling is one pass: {e50:?} against {n50:?}"
    );
}

/// The `.auc` sidecar's bytes, its append-only and idempotent rules, and its
/// refusals, on a scratch store.
#[test]
fn the_auction_sidecar_is_append_only_and_idempotent() {
    let root = scratch("wp12a-auc");
    let key = InstrumentKey::cash(Exchange::Nse, "RELIANCE").unwrap();
    let (d1, d2) = (day(2026, 9, 23), day(2026, 9, 24));
    let print = |sod: u32, price: i64, quantity: u64| auction::Print {
        sod,
        price,
        quantity,
        rows: 1,
        line: 2,
    };
    let path = auction::path(&root, &key, d1).unwrap();
    assert!(
        path.ends_with("bars/gdfl/NSE/CASH/RELIANCE/1s/2026-09.auc"),
        "{}",
        path.display()
    );
    let empty = auction::append(&root, &key, d1, &[]).unwrap();
    assert_eq!((empty.offered, empty.written), (0, 0));
    assert!(!path.exists(), "nothing offered, nothing made");
    let one = [print(at(15, 28, 7), 444_665, 98_765)];
    assert_eq!(auction::append(&root, &key, d1, &one).unwrap().written, 1);
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes.len(), auction::HEADER_LEN + auction::RECORD_LEN);
    assert_eq!(&bytes[..8], b"BRTXAUC1");
    assert_eq!(&bytes[8..12], &[1, 0, 32, 0]);
    let records = auction::parse(&bytes).unwrap();
    assert_eq!(records, vec![auction::Record::of(d1, &one[0])]);
    // 15:28:07 IST on 2026-09-23 is 09:58:07 UTC.
    let midnight_utc = i64::from(d1.days_from_epoch()) * 86_400;
    assert_eq!(
        records[0].t_micros,
        (midnight_utc + i64::from(at(9, 58, 7))) * 1_000_000
    );
    // The same day again writes nothing; a different one for that day, or
    // an earlier day after a later one, is refused.
    assert_eq!(auction::append(&root, &key, d1, &one).unwrap().written, 0);
    let other = [print(at(15, 28, 8), 444_665, 1)];
    assert!(matches!(
        auction::append(&root, &key, d1, &other),
        Err(CmRefusal::AuctionSidecar { why }) if why.contains("new generation")
    ));
    assert_eq!(auction::append(&root, &key, d2, &other).unwrap().written, 1);
    assert!(matches!(
        auction::append(&root, &key, day(2026, 9, 22), &one),
        Err(CmRefusal::AuctionSidecar { why }) if why.contains("never an insertion")
    ));
    assert_eq!(
        auction::parse(&std::fs::read(&path).unwrap())
            .unwrap()
            .len(),
        2
    );
    // A torn tail or a foreign header is refused, never repaired.
    let mut torn = std::fs::read(&path).unwrap();
    torn.push(0);
    std::fs::write(&path, &torn).unwrap();
    assert!(matches!(
        auction::append(&root, &key, day(2026, 9, 25), &one),
        Err(CmRefusal::AuctionSidecar { why }) if why.contains("whole number")
    ));
    assert!(auction::parse(b"BRTXAUC2\x01\x00\x20\x00\x00\x00\x00\x00").is_err());
    assert!(auction::parse(b"short").is_err());
    let mut bent = auction::header().to_vec();
    bent.extend_from_slice(&[0_u8; 28]);
    bent.extend_from_slice(&[1, 0, 0, 0]);
    assert!(auction::parse(&bent).is_err(), "a reserved field set");
    std::fs::remove_dir_all(root).unwrap();
}

/// A gzip of `text`, as an NSE security master arrives.
fn gzip(text: &str) -> Vec<u8> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(text.as_bytes()).unwrap();
    enc.finish().unwrap()
}

/// End to end (DB-034, DB-G42): with the day's security master flagging
/// the share eligible, the import files its trades as bars and its auction
/// print in the month's sidecar, never as a bar; a rerun changes nothing.
/// With no master the file is refused by name and the day stays open.
#[test]
fn an_eligible_shares_print_is_filed_in_the_sidecar_never_as_a_bar() {
    let root = scratch("wp12a-cas");
    let d = day(2026, 9, 24);
    let stem = "RELIANCE.NSE";
    ts_day(
        &root,
        CmKind::Stocks,
        d,
        &[(format!("{stem}.csv"), cas_file(stem, d))],
    );
    let masters = root.join("masters");
    let isin = share_isin("RELIANCE").unwrap();
    let master = format!(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n2885,RELIANCE,EQ,{isin},1\n"
    );
    crate::cash_session_cache::install(&masters, d, &gzip(&master)).unwrap();
    let scope = || {
        CmScope::stored(std::iter::empty::<&str>())
            .unwrap()
            .with_masters(masters.clone())
    };
    let store = root.join("S");
    let got = import(&root, &store, (ImportKind::Stocks, d, d), scope(), &[]);
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    let bars = held(&store, "CASH", "RELIANCE", d).unwrap();
    assert!(bars.iter().all(|b| sod_of(b, d) != at(15, 28, 7)));
    assert!(bars.iter().any(|b| sod_of(b, d) == at(15, 20, 0)));
    let key = InstrumentKey::cash(Exchange::Nse, "RELIANCE").unwrap();
    let sidecar = auction::path(&store, &key, d).unwrap();
    let records = auction::parse(&std::fs::read(&sidecar).unwrap()).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(
        (records[0].price_paisa, records[0].quantity, records[0].rows),
        (444_665, 98_765, 2)
    );
    let before = std::fs::read(&sidecar).unwrap();
    let again = import(
        &root,
        &store,
        (ImportKind::Stocks, d, d),
        scope(),
        &["RELIANCE".to_owned()],
    );
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert_eq!(std::fs::read(&sidecar).unwrap(), before, "idempotent");
    // No master: the candidate rows are neither prints nor trades.
    let unmastered = root.join("B");
    let got = import(
        &root,
        &unmastered,
        (ImportKind::Stocks, d, d),
        CmScope::stored(std::iter::empty::<&str>()).unwrap(),
        &[],
    );
    assert_eq!(got.files_refused, 1);
    assert!(
        got.failures[0].why.contains("eligibility"),
        "{:?}",
        got.failures
    );
    assert_eq!(got.closed_refused, vec![d], "no file read clean");
    assert!(held(&unmastered, "CASH", "RELIANCE", d).is_none());
    std::fs::remove_dir_all(root).unwrap();
}

/// Every new refusal says what it refused.
#[test]
fn every_new_refusal_says_what_it_refused() {
    let change = alias::SYMBOL_CHANGES[0];
    for (why, words) in [
        (
            CmRefusal::SymbolNotInUse {
                stale: Stale::BeforeChange(change),
            },
            "replaced",
        ),
        (
            CmRefusal::CalendarWindowNotLanded {
                kind: DayKind::OpenLengthUnmeasured,
            },
            "keeps no bar",
        ),
        (
            CmRefusal::AuctionSidecar {
                why: "x".to_owned(),
            },
            "sidecar",
        ),
    ] {
        assert!(why.to_string().contains(words), "{why}");
    }
}
