#![cfg(test)]
//! R4-22: the written series rule (G3, D-7310) and the stem hand-overs NSE
//! dated (G2, D-7311), through the scope, the day listing and the import.
//!
//! Every value is invented at run time and no vendor row is quoted
//! (`gdfl_cm::tests::fixtures_are_built_not_pasted` walks this file): the
//! share names start `ZZ` where the test needs no real symbol, and the prices
//! are far from any real one.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::Path;
use std::sync::Arc;

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::Bar;
use store::path::{PathParts, StorePath, Timeframe};

use super::*;
use crate::gdfl_archive::day_zip_name;
use crate::gdfl_fixtures::{bts, csv, put, row, scratch};
use crate::gdfl_import::{ImportKind, Report, Run, Stage, cm_cause, journal_path, run_cm};
use crate::gdfl_tickstore::TickStore;

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn at(h: u32, m: u32, s: u32) -> u32 {
    h * 3_600 + m * 60 + s
}

/// The invented share every rule test files.
const SHARE: &str = "ZZSERIES";

/// Every series code the rule is asked about: NSE's legend's codes for
/// shares, SME shares, debt, government paper, units, warrants, block deals
/// and buybacks, a code no source names (`T0`), `EQ` spelled as a suffix (the
/// plain stem is EQ; the suffix was never seen), a lower-case code and one
/// no list holds.
const REFUSED_CODES: [&str; 27] = [
    "BL", "T0", "N1", "N9", "NA", "Y1", "YZ", "Z1", "ZZ", "GS", "TB", "SG", "SM", "ST", "SZ", "IV",
    "ID", "RR", "RT", "MF", "E1", "X1", "W1", "BO", "EQ", "Q9X", "bl",
];

/// `n` invented rows of `stem` on `d`, one a minute from 09:20.
fn rows_of(stem: &str, d: Day, n: u32, base: u32) -> Vec<u8> {
    let rows: Vec<String> = (0..n)
        .map(|i| {
            row(
                stem,
                d,
                at(9, 20, 0) + i * 60,
                &format!("{}.{:02}", base + i, i % 100),
                1 + u64::from(i),
                0,
            )
        })
        .collect();
    csv(&rows)
}

/// Writes one tick-store stocks day holding `files` (names in the day
/// folder).
fn stocks_day(root: &Path, d: Day, files: &[(String, Vec<u8>)]) {
    let folder = day_folder_name(CmKind::Stocks, d);
    let entries: Vec<(String, Vec<u8>)> = files
        .iter()
        .map(|(name, bytes)| (format!("{folder}/{name}"), bytes.clone()))
        .collect();
    let refs: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let zip_name = day_zip_name(CmKind::Stocks, d);
    let stem = zip_name.strip_suffix(".zip").unwrap();
    put(root, &format!("ts/cm/{stem}.bts"), &bts(&refs));
}

/// One stocks run over the tick store under `root`, filing `scope`.
fn import(
    root: &Path,
    store: &Path,
    (from, to): (Day, Day),
    scope: CmScope,
    only: &[String],
) -> Report {
    let run = Run {
        kind: ImportKind::Stocks,
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

/// Every one-second bar of a cash `symbol` in `d`'s month, or `None`.
fn held(store: &Path, symbol: &str, d: Day) -> Option<Vec<Bar>> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "CASH",
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

/// **G3: EVERY SERIES CODE IS STORED OR REFUSED BY A WRITTEN RULE.** Under
/// the stored scope a share's own stem (EQ) and its `BE` and `BZ` files are
/// filed under the share's key; every other code, seen before or not, is
/// refused by name as `series_not_stored:<code>`, its cause a vendor rule
/// that closes the day, and never filed. Under the swept scope no series is
/// filed and none is refused: WP-12f's skip. Before R4-22 every refused code
/// here was an unnamed skip.
#[test]
fn every_series_code_is_stored_or_refused_by_a_written_rule() {
    let d = day(2024, 4, 2);
    let stored = CmScope::stored([SHARE]).unwrap();
    let swept = CmScope::swept();
    let key = InstrumentKey::cash(Exchange::Nse, SHARE).unwrap();
    for stem in [
        format!("{SHARE}.NSE"),
        format!("{SHARE}.BE.NSE"),
        format!("{SHARE}.BZ.NSE"),
    ] {
        assert_eq!(
            stored.filing(CmKind::Stocks, &stem, d),
            Ok(Some(key)),
            "{stem}"
        );
        assert_eq!(
            stored.series_refusal(CmKind::Stocks, &stem, d),
            None,
            "{stem}"
        );
    }
    for code in REFUSED_CODES {
        let stem = format!("{SHARE}.{code}.NSE");
        assert_eq!(
            stored.filing(CmKind::Stocks, &stem, d),
            Ok(None),
            "{stem} is never filed"
        );
        let why = stored.series_refusal(CmKind::Stocks, &stem, d).unwrap();
        assert_eq!(
            why,
            CmRefusal::SeriesNotStored {
                code: code.into(),
                share: SHARE.into()
            }
        );
        assert_eq!(why.filed_as(), Some(SHARE), "the filter asks the share");
        assert!(
            why.to_string()
                .starts_with(&format!("series_not_stored:{code}: ")),
            "{why}"
        );
        let cause = cm_cause(&why);
        assert_eq!(
            (cause.stage, cause.code, cause.closes),
            (Stage::Vendor, "series_not_stored", true)
        );
        assert_eq!(
            swept.series_refusal(CmKind::Stocks, &stem, d),
            None,
            "swept: a skip"
        );
        assert_eq!(swept.filing(CmKind::Stocks, &stem, d), Ok(None));
    }
    // An index stem, or a stem of the index tree, has no series.
    assert_eq!(
        stored.series_refusal(CmKind::Indices, "NIFTY 50.NSE_IDX", d),
        None
    );
    assert_eq!(
        stored.series_refusal(CmKind::Indices, &format!("{SHARE}.BL.NSE"), d),
        None
    );
}

/// The garbage edges of a series name: no code, a code with a byte no code
/// holds, a code past [`MAX_SERIES_CODE`], a share no scope files, a share
/// one byte past a symbol, and a share at exactly a symbol's width. Only the
/// last is a refused series; every other is no series name at all and stays
/// a counted skip. O(1): the name's length is refused before it is read.
#[test]
fn a_series_name_is_read_only_inside_its_bounds() {
    let d = day(2024, 4, 2);
    let widest = "ZZABCDEFGHIJKLMNOPQRSTUV";
    assert_eq!(widest.len(), SYMBOL_CAPACITY);
    let scope = CmScope::stored([SHARE, widest]).unwrap();
    let long_code = "B".repeat(MAX_SERIES_CODE + 1);
    for stem in [
        format!("{SHARE}..NSE"),
        format!("{SHARE}.B-L.NSE"),
        format!("{SHARE}.{long_code}.NSE"),
        "ZZNOTFILED.BL.NSE".to_owned(),
        format!("{widest}W.BL.NSE"),
        format!("{SHARE}.BL.NSE_IDX"),
        ".BL.NSE".to_owned(),
    ] {
        assert_eq!(
            scope.series_refusal(CmKind::Stocks, &stem, d),
            None,
            "{stem}"
        );
    }
    let edge = "B".repeat(MAX_SERIES_CODE);
    assert!(matches!(
        scope.series_refusal(CmKind::Stocks, &format!("{SHARE}.{edge}.NSE"), d),
        Some(CmRefusal::SeriesNotStored { code, .. }) if *code == *edge
    ));
    assert!(matches!(
        scope.series_refusal(CmKind::Stocks, &format!("{widest}.BL.NSE"), d),
        Some(CmRefusal::SeriesNotStored { share, .. }) if &*share == widest
    ));
}

/// A series file of an OLD symbol names the share's key that day: before
/// its change the newest symbol, after it (the stem no longer live) still
/// the newest symbol, so a filtered run for the new symbol is the one that
/// sees the refusal.
#[test]
fn an_old_symbols_series_file_names_the_newest_key() {
    let scope = CmScope::stored(std::iter::empty::<&str>()).unwrap();
    for d in [day(2022, 7, 12), day(2022, 7, 14)] {
        let why = scope
            .series_refusal(CmKind::Stocks, "RUCHI.BL.NSE", d)
            .unwrap();
        assert_eq!(why.filed_as(), Some("PATANJALI"), "{d}");
    }
}

/// **G3 END TO END: A `BL` DAY FILE IS REFUSED `series_not_stored`.** A day
/// holding a share's own file and its block-deal file files the own file's
/// bars, refuses the `BL` file by name, and closes done with refusals; a run
/// filtered to another share neither reads nor refuses it. Before R4-22 the
/// `BL` file was a silent skip and the day closed plain `done`.
#[test]
fn a_bl_day_file_is_refused_series_not_stored() {
    let root = scratch("r422-bl");
    let d = day(2024, 4, 2);
    stocks_day(
        &root,
        d,
        &[
            (
                format!("{SHARE}.NSE.csv"),
                rows_of(&format!("{SHARE}.NSE"), d, 12, 4_321),
            ),
            (
                format!("{SHARE}.BL.NSE.csv"),
                rows_of(&format!("{SHARE}.BL.NSE"), d, 3, 4_400),
            ),
            (
                "ZZOTHER.NSE.csv".to_owned(),
                rows_of("ZZOTHER.NSE", d, 5, 4_500),
            ),
        ],
    );
    let scope = || CmScope::stored([SHARE, "ZZOTHER"]).unwrap();
    let store = root.join("S");
    let got = import(&root, &store, (d, d), scope(), &[]);
    assert_eq!((got.files, got.files_refused), (2, 1), "{:?}", got.failures);
    assert_eq!(got.failures.len(), 1);
    assert!(
        got.failures[0].why.contains("series_not_stored:BL"),
        "{:?}",
        got.failures
    );
    assert!(
        got.failures[0]
            .instrument
            .contains(&format!("{SHARE}.BL.NSE.csv")),
        "{:?}",
        got.failures
    );
    assert_eq!(
        got.closed_with_refusals,
        vec![d],
        "a written rule closes the day"
    );
    assert_eq!(
        held(&store, SHARE, d).unwrap().len(),
        12,
        "only the own file's bars"
    );
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(journal.contains("series_not_stored"), "{journal}");
    // A run filtered to the other share never sees the block-deal file.
    let filtered = root.join("F");
    let only = vec!["ZZOTHER".to_owned()];
    let got = import(&root, &filtered, (d, d), scope(), &only);
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.files, 1);
    std::fs::remove_dir_all(root).unwrap();
}

/// Extremes at once: one share's day with NO own file, a `BE` file (filed
/// under the share), a `BL` and a `T0` file (each refused by name) and a
/// never-seen code (refused by name). The `BE` file's bars are the share's
/// day; three refusals are named, one per file; nothing of the refused files
/// lands.
#[test]
fn a_day_of_one_trade_for_trade_file_and_three_refused_series_files_the_be_alone() {
    let root = scratch("r422-mixed");
    let d = day(2024, 4, 3);
    let stem = |code: &str| format!("{SHARE}.{code}.NSE");
    stocks_day(
        &root,
        d,
        &[
            (
                format!("{}.csv", stem("BE")),
                rows_of(&stem("BE"), d, 7, 4_321),
            ),
            (
                format!("{}.csv", stem("BL")),
                rows_of(&stem("BL"), d, 2, 4_600),
            ),
            (
                format!("{}.csv", stem("T0")),
                rows_of(&stem("T0"), d, 4, 4_700),
            ),
            (
                format!("{}.csv", stem("Q7")),
                rows_of(&stem("Q7"), d, 1, 4_800),
            ),
        ],
    );
    let store = root.join("S");
    let got = import(
        &root,
        &store,
        (d, d),
        CmScope::stored([SHARE]).unwrap(),
        &[],
    );
    assert_eq!((got.files, got.files_refused), (1, 3), "{:?}", got.failures);
    let mut codes: Vec<&str> = got
        .failures
        .iter()
        .map(|f| {
            ["BL", "T0", "Q7"]
                .into_iter()
                .find(|code| f.why.contains(&format!("series_not_stored:{code}")))
                .unwrap_or_else(|| panic!("{f:?}"))
        })
        .collect();
    codes.sort_unstable();
    assert_eq!(codes, ["BL", "Q7", "T0"]);
    assert_eq!(
        held(&store, SHARE, d).unwrap().len(),
        7,
        "the BE file is the share's day"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A stem resolving to two files of one key on one day: the share's own file
/// and its `BE` file. Both are refused by name as twins and neither is filed
/// (WP-12f's rule, kept): the series rule never picks one.
#[test]
fn a_day_holding_a_plain_and_a_be_file_of_one_share_refuses_both() {
    let root = scratch("r422-twins");
    let d = day(2024, 4, 4);
    stocks_day(
        &root,
        d,
        &[
            (
                format!("{SHARE}.NSE.csv"),
                rows_of(&format!("{SHARE}.NSE"), d, 5, 4_321),
            ),
            (
                format!("{SHARE}.BE.NSE.csv"),
                rows_of(&format!("{SHARE}.BE.NSE"), d, 5, 4_321),
            ),
        ],
    );
    let store = root.join("S");
    let got = import(
        &root,
        &store,
        (d, d),
        CmScope::stored([SHARE]).unwrap(),
        &[],
    );
    assert_eq!(got.files, 0, "{:?}", got.failures);
    assert!(
        got.failures
            .iter()
            .any(|f| f.why.contains("two names for one ticker")),
        "{:?}",
        got.failures
    );
    assert!(held(&store, SHARE, d).is_none());
    std::fs::remove_dir_all(root).unwrap();
}

/// **G2 END TO END: AN OLD STEM BEFORE ITS CHANGE IS FILED UNDER THE NEWEST
/// SYMBOL, FOR RUCHI -> PATANJALI.** On 2022-07-12 the vendor's `RUCHI.NSE`
/// (and a `RUCHI.BE.NSE` spell on another day) lands under `PATANJALI`; on
/// 2022-07-13 `PATANJALI.NSE` lands under the same key and a `RUCHI.NSE`
/// file that day is refused by name (`symbol_not_in_use`). One series, one
/// key, across the hand-over. Before R4-22 no row named the change, so
/// `RUCHI` was its own (unstored) key and its days never reached
/// `PATANJALI`.
#[test]
fn an_old_stem_before_its_change_is_filed_under_the_newest_symbol_for_ruchi_patanjali() {
    let root = scratch("r422-ruchi");
    let (before, spell, after) = (day(2022, 7, 12), day(2022, 7, 11), day(2022, 7, 13));
    stocks_day(
        &root,
        before,
        &[(
            "RUCHI.NSE.csv".to_owned(),
            rows_of("RUCHI.NSE", before, 6, 7_701),
        )],
    );
    stocks_day(
        &root,
        spell,
        &[(
            "RUCHI.BE.NSE.csv".to_owned(),
            rows_of("RUCHI.BE.NSE", spell, 4, 7_702),
        )],
    );
    stocks_day(
        &root,
        after,
        &[
            (
                "PATANJALI.NSE.csv".to_owned(),
                rows_of("PATANJALI.NSE", after, 9, 7_703),
            ),
            (
                "RUCHI.NSE.csv".to_owned(),
                rows_of("RUCHI.NSE", after, 2, 7_704),
            ),
        ],
    );
    let store = root.join("S");
    let scope = || CmScope::stored(std::iter::empty::<&str>()).unwrap();
    let got = import(&root, &store, (spell, after), scope(), &[]);
    assert_eq!((got.files, got.files_refused), (3, 1), "{:?}", got.failures);
    assert!(
        got.failures[0]
            .why
            .contains("RUCHI was not the exchange's symbol that day"),
        "{:?}",
        got.failures
    );
    assert_eq!(
        held(&store, "PATANJALI", before).unwrap().len(),
        6 + 4 + 9,
        "one key across the hand-over"
    );
    assert!(
        held(&store, "RUCHI", before).is_none(),
        "never a second key"
    );
    // A run filtered to the NEW symbol reads the old stem's days too.
    let filtered = root.join("F");
    let only = vec!["PATANJALI".to_owned()];
    let got = import(&root, &filtered, (before, before), scope(), &only);
    assert_eq!(got.files, 1, "{:?}", got.failures);
    assert_eq!(held(&filtered, "PATANJALI", before).unwrap().len(), 6);
    std::fs::remove_dir_all(root).unwrap();
}

/// The five other hand-overs file their old stem the same way, on the day
/// before each change, through the scope's filing.
#[test]
fn every_dated_handover_files_the_old_stem_under_the_new_symbol_the_day_before() {
    let scope = CmScope::stored(std::iter::empty::<&str>()).unwrap();
    for (old, new, (y, m, dd)) in [
        ("ANGELBRKG", "ANGELONE", (2021, 11, 10)),
        ("RNAM", "NAM-INDIA", (2020, 1, 22)),
        ("MINDAIND", "UNOMINDA", (2022, 8, 4)),
        ("GET&D", "GVT&D", (2024, 11, 4)),
        ("RUCHI", "PATANJALI", (2022, 7, 12)),
        ("BAJAJTEMPO", "FORCEMOT", (2019, 8, 16)),
        ("ADANIGAS", "ATGL", (2021, 1, 12)),
        ("MAGMA", "POONAWALLA", (2021, 8, 4)),
        ("TWL", "TITAGARH", (2023, 6, 16)),
    ] {
        let d = day(y, m, dd);
        let key = InstrumentKey::cash(Exchange::Nse, new).unwrap();
        assert_eq!(
            scope.filing(CmKind::Stocks, &format!("{old}.NSE"), d),
            Ok(Some(key)),
            "{old} on {d}"
        );
        assert_eq!(
            scope.filing(CmKind::Stocks, &format!("{old}.BE.NSE"), d),
            Ok(Some(key)),
            "{old}.BE on {d}"
        );
        assert_eq!(
            scope.stem_for(&key, d),
            Some((CmKind::Stocks, format!("{old}.NSE")))
        );
    }
}

/// The cost of the written rule, MEASURED (p50, p99 and max over n = 200
/// calls each, printed for the report) and bounded by its shape: one split,
/// two compares, one probe. Every answer is checked, a hit, a share's own
/// series, a share no scope files and a name past the bound in turn, so the
/// timing loop is also a correctness loop.
#[test]
fn the_series_rule_costs_one_split_and_one_probe_measured() {
    let d = day(2024, 4, 2);
    let scope = CmScope::stored([SHARE]).unwrap();
    let long = format!("{}.BL.NSE", "Z".repeat(SYMBOL_CAPACITY + 1));
    let cases: [(String, bool); 4] = [
        (format!("{SHARE}.BL.NSE"), true),
        (format!("{SHARE}.BE.NSE"), false),
        ("ZZNOTFILED.N1.NSE".to_owned(), false),
        (long, false),
    ];
    let mut took: Vec<u128> = Vec::with_capacity(200);
    for n in 0..200 {
        let (stem, refused) = &cases[n % cases.len()];
        let started = std::time::Instant::now();
        let got = scope.series_refusal(CmKind::Stocks, stem, d);
        took.push(started.elapsed().as_nanos());
        assert_eq!(got.is_some(), *refused, "{stem}");
    }
    took.sort_unstable();
    eprintln!(
        "series_refusal ns, n=200: p50 {} p99 {} max {}",
        took[100], took[197], took[199]
    );
}
