#![cfg(test)]
//! Tests of the GDFL import runtime. Every value is invented at run time;
//! no vendor row is quoted (`gdfl_cm::tests::fixtures_are_built_not_pasted`).

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe};

use super::*;
use crate::gdfl_archive::Archive;
use crate::gdfl_fixtures::{Method, bts, csv, mon, put, row, scratch, zip};
use crate::gdfl_nfo::{NfoTickStore, NfoZips};
use crate::gdfl_tickstore::TickStore;

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// Three regular sessions and the Saturday after them.
fn d1() -> Day {
    day(2024, 4, 1)
}
fn d2() -> Day {
    day(2024, 4, 2)
}
fn d3() -> Day {
    day(2024, 4, 3)
}

const NINE_FIFTEEN: u32 = 9 * 3_600 + 15 * 60;
const CLOSE: u32 = 15 * 3_600 + 30 * 60;

fn tick(sod: u32, ltp: i64, ltq: u64) -> Tick {
    Tick {
        sod,
        ltp,
        ltq,
        oi: None,
    }
}

// ── the bar: row rule, placement, fold ─────────────────────────────────────

#[test]
fn an_index_second_uses_every_row_and_carries_no_volume() {
    let ticks = [
        tick(NINE_FIFTEEN, 1_000, 0),
        tick(NINE_FIFTEEN, 1_030, 0),
        tick(NINE_FIFTEEN, 990, 0),
        tick(NINE_FIFTEEN + 1, 1_010, 7),
    ];
    let got = convert(ImportKind::Indices, d1(), &ticks).unwrap();
    assert_eq!(got.seconds.len(), 2);
    let first = got.seconds[0];
    assert_eq!(
        (first.open, first.high, first.low, first.close),
        (1_000, 1_030, 990, 990)
    );
    assert_eq!(first.volume, 0, "index volume is 0, never null");
    assert_eq!(
        got.seconds[1].volume, 0,
        "even where a row states a quantity"
    );
    assert_eq!(first.open_interest, OI_NULL);
    // 2024-04-01 09:15:00 IST is 03:45:00 UTC.
    assert_eq!(first.ts_micros, 1_711_943_100 * 1_000_000);
    assert_eq!(got.placement.ltq_zero_dropped, 0);
}

#[test]
fn a_stock_or_option_second_uses_traded_rows_only_and_sums_their_quantity() {
    let ticks = [
        tick(NINE_FIFTEEN, 500, 0),
        tick(NINE_FIFTEEN, 510, 10),
        tick(NINE_FIFTEEN, 505, 0),
        tick(NINE_FIFTEEN, 520, 5),
        tick(NINE_FIFTEEN, 495, 1),
        tick(NINE_FIFTEEN + 3, 600, 0),
    ];
    for kind in [ImportKind::Stocks, ImportKind::Options] {
        let got = convert(kind, d1(), &ticks).unwrap();
        assert_eq!(
            got.seconds.len(),
            1,
            "a second with no traded row has no bar"
        );
        let bar = got.seconds[0];
        assert_eq!(
            (bar.open, bar.high, bar.low, bar.close),
            (510, 520, 495, 495)
        );
        assert_eq!(bar.volume, 16);
        assert_eq!(got.placement.ltq_zero_dropped, 3);
        assert_eq!(got.placement.rows, 6);
    }
}

#[test]
fn an_option_second_carries_the_last_open_interest_of_its_traded_rows() {
    let with = |sod, ltp, ltq, oi| Tick {
        sod,
        ltp,
        ltq,
        oi: Some(oi),
    };
    let got = convert(
        ImportKind::Options,
        d1(),
        &[
            with(NINE_FIFTEEN, 5, 1, 100),
            with(NINE_FIFTEEN, 6, 2, 0),
            with(NINE_FIFTEEN, 7, 0, 999),
        ],
    )
    .unwrap();
    assert_eq!(
        got.seconds[0].open_interest, 0,
        "zero is zero, the last traded row's"
    );
}

#[test]
fn a_late_row_is_deferred_to_the_next_in_order_second_never_earlier() {
    let ticks = [
        tick(NINE_FIFTEEN, 100, 0),
        tick(NINE_FIFTEEN + 5, 105, 0),
        // Two late rows, back-stepping 3 s and 5 s.
        tick(NINE_FIFTEEN + 2, 999, 0),
        tick(NINE_FIFTEEN, 1, 0),
        tick(NINE_FIFTEEN + 7, 107, 0),
    ];
    let got = convert(ImportKind::Indices, d1(), &ticks).unwrap();
    let secs: Vec<i64> = got.seconds.iter().map(|b| b.ts_micros).collect();
    let at = |s: u32| micros_at(d1(), s);
    assert_eq!(
        secs,
        vec![at(NINE_FIFTEEN), at(NINE_FIFTEEN + 5), at(NINE_FIFTEEN + 7)]
    );
    let landed = got.seconds[2];
    assert_eq!(
        (landed.open, landed.high, landed.low, landed.close),
        (999, 999, 1, 107)
    );
    assert_eq!(got.placement.late_rows, 2);
    assert_eq!(got.placement.max_back_s, 5);
    assert_eq!(got.placement.late_unresolved, 0);
    // The 09:15:05 second is untouched by the deferred prices.
    assert_eq!(got.seconds[1].high, 105);
}

#[test]
fn a_late_row_with_nothing_after_it_is_dropped_and_counted() {
    let ticks = [
        tick(NINE_FIFTEEN + 10, 100, 0),
        tick(NINE_FIFTEEN, 1, 0),
        tick(NINE_FIFTEEN + 1, 2, 0),
    ];
    let got = convert(ImportKind::Indices, d1(), &ticks).unwrap();
    assert_eq!(got.seconds.len(), 1);
    assert_eq!(got.placement.late_unresolved, 2);
    assert_eq!(got.placement.max_back_s, 10);
}

#[test]
fn a_utc_stamped_row_is_deferred_like_any_late_row() {
    let ticks = [
        tick(NINE_FIFTEEN + 19_800, 100, 0),
        tick(NINE_FIFTEEN, 7, 0),
        tick(NINE_FIFTEEN + 19_801, 101, 0),
    ];
    let got = convert(ImportKind::Indices, d1(), &ticks).unwrap();
    assert_eq!(got.placement.max_back_s, 19_800);
    assert_eq!(got.seconds[1].low, 7, "carried to the next in-order second");
}

#[test]
fn duplicate_stamps_halts_and_frozen_runs_are_kept_as_printed() {
    let mut ticks = vec![tick(NINE_FIFTEEN, 100, 0); 50];
    // A halt: nothing for ten minutes, then a frozen run of one price.
    for s in 0..30 {
        ticks.push(tick(NINE_FIFTEEN + 600 + s, 100, 0));
    }
    let got = convert(ImportKind::Indices, d1(), &ticks).unwrap();
    assert_eq!(
        got.seconds.len(),
        31,
        "one bar for fifty rows of one second, none for the halt"
    );
    assert!(got.seconds.iter().all(|b| b.open == 100 && b.close == 100));
}

#[test]
fn empty_and_untraded_files_make_no_bar_and_no_refusal() {
    assert!(
        convert(ImportKind::Stocks, d1(), &[])
            .unwrap()
            .seconds
            .is_empty()
    );
    let untraded = [tick(NINE_FIFTEEN, 1, 0), tick(NINE_FIFTEEN + 1, 1, 0)];
    let got = convert(ImportKind::Options, d1(), &untraded).unwrap();
    assert!(got.seconds.is_empty());
    assert_eq!(got.placement.ltq_zero_dropped, 2);
}

#[test]
fn a_quantity_past_i64_is_refused_never_saturated() {
    let big = u64::try_from(i64::MAX).unwrap() + 1;
    assert_eq!(
        convert(ImportKind::Stocks, d1(), &[tick(NINE_FIFTEEN, 1, big)]),
        Err(ImportRefusal::VolumeOverflow { sod: NINE_FIFTEEN })
    );
    let max = u64::try_from(i64::MAX).unwrap();
    let one = convert(ImportKind::Stocks, d1(), &[tick(NINE_FIFTEEN, 1, max)]).unwrap();
    assert_eq!(one.seconds[0].volume, i64::MAX);
    assert_eq!(
        convert(
            ImportKind::Stocks,
            d1(),
            &[tick(NINE_FIFTEEN + 9, 1, max), tick(NINE_FIFTEEN + 9, 1, 1)]
        ),
        Err(ImportRefusal::VolumeOverflow {
            sod: NINE_FIFTEEN + 9
        })
    );
}

#[test]
fn a_negative_price_reaches_the_fold_and_is_refused_by_it() {
    // No reader hands one on (both refuse below one tick); the fold's own
    // refusals still name themselves here.
    let got = convert(ImportKind::Indices, d1(), &[tick(NINE_FIFTEEN, -5, 0)]);
    assert!(got.is_ok(), "the fold folds what it is given: {got:?}");
    assert_eq!(second_of(micros_at(d1(), 77), d1()), 77);
    assert_eq!(second_of(i64::MIN, d1()), 0);
}

#[test]
fn kinds_name_their_words_segments_and_venues() {
    for kind in ImportKind::ALL {
        assert_eq!(ImportKind::parse(kind.as_str()), Ok(kind));
    }
    assert_eq!(
        ImportKind::parse("futures"),
        Err(ImportRefusal::KindUnknown {
            word: "futures".to_owned()
        })
    );
    assert_eq!(
        ImportKind::ALL.map(ImportKind::segment),
        ["INDEX", "CASH", "FNO"]
    );
    assert_eq!(
        ImportKind::ALL.map(ImportKind::listing),
        [Listing::Index, Listing::Equity, Listing::Derivative]
    );
    assert_eq!(
        ImportKind::ALL.map(ImportKind::every_row_counts),
        [true, false, false]
    );
}

#[test]
fn every_refusal_says_what_it_is() {
    let all = [
        ImportRefusal::KindUnknown { word: "w".into() },
        ImportRefusal::RangeBackwards {
            from: d2(),
            to: d1(),
        },
        ImportRefusal::VolumeOverflow { sod: 3 },
        ImportRefusal::Fold { why: "f".into() },
        ImportRefusal::Journal { why: "j".into() },
    ];
    let texts: Vec<String> = all.iter().map(ToString::to_string).collect();
    let unique: std::collections::HashSet<&String> = texts.iter().collect();
    assert_eq!(unique.len(), texts.len());
    assert!(texts[1].contains("2024-04-02"));
}

// ── a world of invented days, in both sources ──────────────────────────────

fn ddmmyyyy(d: Day) -> String {
    format!("{:02}{:02}{:04}", d.day(), d.month(), d.year())
}

/// One invented index file: pre-open rows, three rows in one second, a late
/// row, session rows and post-close rows (so the reader's end check holds).
fn index_file(stem: &str, d: Day, base: u32) -> Vec<u8> {
    let p = |v: u32| format!("{}.{:02}", base + v / 100, v % 100);
    let mut rows = vec![row(stem, d, NINE_FIFTEEN - 300, &p(1), 0, 0)];
    rows.push(row(stem, d, NINE_FIFTEEN, &p(5), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN, &p(55), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN, &p(15), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN + 2, &p(20), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN + 1, &p(30), 0, 0));
    for s in (60..22_500).step_by(997) {
        rows.push(row(stem, d, NINE_FIFTEEN + s, &p(s % 500), 0, 0));
    }
    rows.push(row(stem, d, CLOSE + 60, &p(7), 0, 0));
    csv(&rows)
}

/// One invented stock file: traded and untraded rows.
fn stock_file(stem: &str, d: Day) -> Vec<u8> {
    let mut rows = Vec::new();
    for s in (0..22_500).step_by(1_201) {
        rows.push(row(stem, d, NINE_FIFTEEN + s, "731.40", 0, 0));
        rows.push(row(
            stem,
            d,
            NINE_FIFTEEN + s,
            &format!("73{}.05", s % 10),
            u64::from(s % 7 + 1),
            0,
        ));
    }
    csv(&rows)
}

/// One invented option file.
fn option_file(ticker: &str, d: Day, ltp: &str) -> Vec<u8> {
    let stem = format!("{ticker}.NFO");
    let mut rows = Vec::new();
    for s in (0..22_500).step_by(2_003) {
        rows.push(row(
            &stem,
            d,
            NINE_FIFTEEN + s,
            ltp,
            u64::from(s % 3) * 25,
            1_000 + u64::from(s),
        ));
    }
    csv(&rows)
}

/// The capital-market entries of one tree on one day.
fn cm_entries(kind: CmKind, d: Day) -> Vec<(String, Vec<u8>)> {
    let folder = crate::gdfl_cm::day_folder_name(kind, d);
    match kind {
        CmKind::Indices => vec![
            (format!("{folder}/"), Vec::new()),
            (
                format!("{folder}/NIFTY 50.NSE_IDX.csv"),
                index_file("NIFTY 50.NSE_IDX", d, 22_000),
            ),
            (
                format!("{folder}/NIFTY BANK.NSE_IDX.csv"),
                index_file("NIFTY BANK.NSE_IDX", d, 47_000),
            ),
            (
                format!("{folder}/INDIA VIX.NSE_IDX.csv"),
                index_file("INDIA VIX.NSE_IDX", d, 13),
            ),
        ],
        CmKind::Stocks => vec![
            (
                format!("{folder}/RELIANCE.NSE.csv"),
                stock_file("RELIANCE.NSE", d),
            ),
            (format!("{folder}/SBIN.NSE.csv"), stock_file("SBIN.NSE", d)),
            (
                format!("{folder}/RELIANCE.BE.NSE.csv"),
                stock_file("RELIANCE.BE.NSE", d),
            ),
        ],
    }
}

/// The options entries of one day.
fn nfo_entries(d: Day) -> Vec<(String, Vec<u8>)> {
    let folder = crate::gdfl_nfo::day_folder_name(d);
    [
        "NIFTY04APR2422000CE",
        "NIFTY04APR2422000PE",
        "BANKNIFTY10APR2447000CE",
    ]
    .iter()
    .map(|t| {
        (
            format!("{folder}\\Options\\{t}.NFO.csv"),
            option_file(t, d, "101.5"),
        )
    })
    .collect()
}

fn as_refs(entries: &[(String, Vec<u8>)]) -> Vec<(&str, &[u8])> {
    entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect()
}

/// Writes `days` of every tree into a tick store and into zips under `root`.
fn world(root: &Path, days: &[Day]) {
    let mut cm_outer: Vec<(String, Vec<u8>)> = Vec::new();
    let mut nfo_outer: BTreeMap<u16, Vec<(String, Vec<u8>)>> = BTreeMap::new();
    for &d in days {
        let month = format!("{}_{:04}", mon(d), d.year());
        for (kind, tree, prefix) in [
            (CmKind::Indices, "INDICES", "GFDLCM_INDICES_TICK_"),
            (CmKind::Stocks, "STOCKS", "GFDLCM_STOCK_TICK_"),
        ] {
            let entries = cm_entries(kind, d);
            let rel = format!("{tree}/{:04}/{month}/{prefix}{}", d.year(), ddmmyyyy(d));
            put(root, &format!("ts/cm/{rel}.bts"), &bts(&as_refs(&entries)));
            let methods: Vec<(&str, &[u8], Method)> = entries
                .iter()
                .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Deflated))
                .collect();
            cm_outer.push((format!("{rel}.zip"), zip(&methods)));
        }
        let entries = nfo_entries(d);
        let rel = format!("{:04}/{month}/GFDLNFO_TICK_{}", d.year(), ddmmyyyy(d));
        put(
            root,
            &format!("ts/options/{rel}.bts"),
            &bts(&as_refs(&entries)),
        );
        let methods: Vec<(&str, &[u8], Method)> = entries
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Deflated))
            .collect();
        nfo_outer.entry(d.year()).or_default().push((
            format!("{month}/GFDLNFO_TICK_{}.zip", ddmmyyyy(d)),
            zip(&methods),
        ));
    }
    let stored: Vec<(&str, &[u8], Method)> = cm_outer
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
        .collect();
    put(root, "zips/cm.zip", &zip(&stored));
    for (year, inner) in nfo_outer {
        let stored: Vec<(&str, &[u8], Method)> = inner
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
            .collect();
        put(root, &format!("zips/options/{year:04}.zip"), &zip(&stored));
    }
}

/// Which source a run reads.
#[derive(Clone, Copy)]
enum Src {
    Store,
    Zips,
}

fn import(root: &Path, src: Src, kind: ImportKind, from: Day, to: Day, store: &Path) -> Report {
    import_only(root, src, kind, from, to, store, &[])
}

fn import_only(
    root: &Path,
    src: Src,
    kind: ImportKind,
    from: Day,
    to: Day,
    store: &Path,
    only: &[String],
) -> Report {
    let run = Run {
        kind,
        from,
        to,
        only,
        store_root: store,
    };
    match (src, kind) {
        (Src::Store, ImportKind::Options) => run_nfo(&NfoTickStore::new(&root.join("ts")), &run),
        (Src::Zips, ImportKind::Options) => {
            run_nfo(&NfoZips::new(&root.join("zips/options")), &run)
        }
        (Src::Store, _) => run_cm(&TickStore::new(&root.join("ts")), &run),
        (Src::Zips, _) => run_cm(&Archive::open(&root.join("zips/cm.zip")).unwrap(), &run),
    }
    .unwrap()
}

/// Every file under `dir`, relative path to bytes.
fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(
                    path.strip_prefix(dir).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                );
            }
        }
    }
    out
}

/// Every one-second bar of one instrument-month.
fn bars(store: &Path, segment: &str, symbol: &str, contract: Option<Contract>) -> Vec<Bar> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract,
        timeframe: Timeframe::SECOND_1,
        month: d1().year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    #[expect(clippy::cast_possible_truncation, reason = "the store's own id fold")]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    let file = BarFile::open_existing(store, path, id).unwrap();
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

fn nifty_ce() -> Contract {
    Contract::parse("2024-04-04-2200000-CE").unwrap()
}

#[test]
fn every_kind_lands_at_one_second_and_both_sources_leave_the_same_store() {
    let root = scratch("import-both");
    world(&root, &[d1()]);
    let (a, b) = (root.join("A"), root.join("B"));
    for kind in ImportKind::ALL {
        let ra = import(&root, Src::Store, kind, d1(), d1(), &a);
        let rb = import(&root, Src::Zips, kind, d1(), d1(), &b);
        assert!(ra.failures.is_empty(), "{kind:?}: {:?}", ra.failures);
        assert_eq!(ra, rb, "{kind:?}: the two sources report the same run");
        assert_eq!(ra.days_imported, 1);
        assert!(
            ra.seconds > 0 && ra.seconds_committed == ra.seconds,
            "{kind:?}: {ra:?}"
        );
    }
    assert_eq!(
        tree(&a),
        tree(&b),
        "byte for byte, census and journal included"
    );
    // Only the one-second rung exists, for every kind (D-2802).
    let files = tree(&a);
    for path in files.keys() {
        let text = path.to_string_lossy();
        if text.starts_with("bars/") {
            assert!(text.contains("/1s/"), "{text}");
        }
    }
    // The import files through the store's one writer, so every one-second
    // month it lands carries the `.tix` time index that writer keeps on each
    // append (D-2329): no side door leaves a month the index does not cover.
    let months: Vec<_> = files
        .keys()
        .filter(|p| p.starts_with("bars") && p.to_string_lossy().ends_with(".bin"))
        .collect();
    assert!(!months.is_empty(), "the import landed no bar month");
    for bin in months {
        let tix = bin.with_extension("tix");
        assert!(
            files.get(&tix).is_some_and(|bytes| !bytes.is_empty()),
            "{} has no time index beside it",
            bin.display()
        );
    }
    let nifty = bars(&a, "INDEX", "NIFTY", None);
    // Pre-open and post-close rows are not session bars.
    assert_eq!(nifty[0].ts_micros, micros_at(d1(), NINE_FIFTEEN));
    assert!(
        nifty
            .iter()
            .all(|b| b.volume == 0 && b.open_interest == OI_NULL)
    );
    // The late 09:15:01 row joined the 09:15:02 second; no 09:15:01 bar.
    assert_eq!(nifty[1].ts_micros, micros_at(d1(), NINE_FIFTEEN + 2));
    assert_eq!(
        (nifty[0].open, nifty[0].high, nifty[0].low),
        (2_200_005, 2_200_055, 2_200_005)
    );
    let ce = bars(&a, "FNO", "NIFTY", Some(nifty_ce()));
    assert!(ce.iter().all(|b| b.volume > 0 && b.open_interest >= 1_000));
    assert!(
        bars(&a, "CASH", "RELIANCE", None)
            .iter()
            .all(|b| b.volume > 0)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_rerun_writes_nothing_and_a_new_day_appends() {
    let root = scratch("import-rerun");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    let first = import(&root, Src::Store, ImportKind::Options, d1(), d1(), &store);
    let before = tree(&store);
    let again = import(&root, Src::Store, ImportKind::Options, d1(), d1(), &store);
    assert_eq!(again.days_skipped, 1);
    assert_eq!(again.seconds, 0);
    assert_eq!(tree(&store), before, "byte for byte");
    let held = bars(&store, "FNO", "NIFTY", Some(nifty_ce())).len();
    let next = import(&root, Src::Store, ImportKind::Options, d1(), d2(), &store);
    assert_eq!((next.days_skipped, next.days_imported), (1, 1));
    let now = bars(&store, "FNO", "NIFTY", Some(nifty_ce()));
    assert!(now.len() > held, "the second day appended");
    assert!(
        now.windows(2).all(|w| w[0].ts_micros < w[1].ts_micros),
        "strictly increasing"
    );
    assert_eq!(first.seconds + next.seconds, first.seconds * 2);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_crashed_run_is_resumed_without_a_duplicate() {
    let root = scratch("import-crash");
    world(&root, &[d1()]);
    let clean = root.join("clean");
    import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &clean);
    // The same run, crashed after its bars landed and before its census and
    // its `done` line: the journal holds only `begin`, the census is gone,
    // and one bar file carries a torn tail past its committed records.
    let crashed = root.join("crashed");
    import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &crashed);
    let journal = journal_path(&crashed);
    let text = std::fs::read_to_string(&journal).unwrap();
    let begin = text.lines().next().unwrap().to_owned();
    assert!(begin.starts_with("begin indices 2024-04-01 *"), "{begin}");
    std::fs::write(&journal, format!("{begin}\n")).unwrap();
    std::fs::remove_dir_all(crashed.join("manifest")).unwrap();
    let bin = tree(&crashed)
        .into_keys()
        .find(|p| p.to_string_lossy().ends_with(".bin"))
        .unwrap();
    let mut torn = std::fs::read(crashed.join(&bin)).unwrap();
    torn.extend_from_slice(&[0xAB; 37]);
    std::fs::write(crashed.join(&bin), torn).unwrap();
    let resumed = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &crashed);
    assert_eq!(resumed.resumed, vec![d1()]);
    assert_eq!(resumed.seconds_committed, 0, "nothing written twice");
    assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
    assert_eq!(
        bars(&crashed, "INDEX", "NIFTY", None),
        bars(&clean, "INDEX", "NIFTY", None)
    );
    let census = |s: &Path| std::fs::read(s.join("manifest/gdfl.man")).unwrap();
    assert_eq!(
        census(&crashed),
        census(&clean),
        "the census is re-counted whole"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_torn_journal_line_is_ignored_and_a_foreign_one_refuses_the_run() {
    let root = scratch("import-journal");
    world(&root, &[d1()]);
    let store = root.join("S");
    put(
        &store,
        "imports/gdfl.journal",
        b"done indices 2024-04-01 * files=2\ndone ind",
    );
    let got = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    assert_eq!(got.days_skipped, 1);
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(
        text.ends_with("done ind (torn)\n"),
        "the torn line is closed with its mark, not rewritten (D-2814)"
    );
    put(&store, "imports/gdfl.journal", b"something else\n");
    let run = Run {
        kind: ImportKind::Indices,
        from: d1(),
        to: d1(),
        only: &[],
        store_root: &store,
    };
    assert!(matches!(
        run_cm(&TickStore::new(&root.join("ts")), &run),
        Err(ImportRefusal::Journal { .. })
    ));
    // A store root that is a file: the journal cannot be read.
    let file_root = put(&root, "plain", b"x");
    let run = Run {
        store_root: &file_root,
        ..run
    };
    assert!(matches!(
        run_cm(&TickStore::new(&root.join("ts")), &run),
        Err(ImportRefusal::Journal { .. })
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_journal_that_cannot_be_written_fails_the_day_by_name() {
    let root = scratch("import-journal-write");
    world(&root, &[d1()]);
    let store = root.join("S");
    // A journal that reads and will not take a line.
    let journal = put(&store, "imports/gdfl.journal", b"");
    let mut perms = std::fs::metadata(&journal).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o444);
    std::fs::set_permissions(&journal, perms).unwrap();
    let got = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    assert_eq!(got.seconds, 0, "nothing is written before its begin line");
    assert!(
        got.failures
            .iter()
            .any(|f| f.instrument.ends_with("journal")),
        "{:?}",
        got.failures
    );
    // A clean day that wrote nothing still records `done`, and that line
    // failing is named too.
    let folder = crate::gdfl_nfo::day_folder_name(d1());
    let untraded = csv(&[row(
        "NIFTY04APR2422000CE.NFO",
        d1(),
        NINE_FIFTEEN,
        "0",
        0,
        5,
    )]);
    let entries = vec![(
        format!("{folder}\\Options\\NIFTY04APR2422000CE.NFO.csv"),
        untraded,
    )];
    put(
        &root,
        "ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let quiet = import(&root, Src::Store, ImportKind::Options, d1(), d1(), &store);
    assert_eq!(quiet.files, 1);
    assert!(
        quiet
            .failures
            .iter()
            .any(|f| f.instrument.ends_with("journal")),
        "{:?}",
        quiet.failures
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn closed_unmeasured_special_and_missing_days_are_each_named() {
    let root = scratch("import-days");
    world(&root, &[d1()]);
    let store = root.join("S");
    // 2024-04-06/07 are a weekend; 2024-04-02 is a trading day nobody wrote.
    let got = import(
        &root,
        Src::Store,
        ImportKind::Stocks,
        d1(),
        day(2024, 4, 7),
        &store,
    );
    assert_eq!(got.days_imported, 1);
    assert_eq!(got.days_closed, 2);
    assert_eq!(
        got.days_missing,
        vec![d2(), d3(), day(2024, 4, 4), day(2024, 4, 5)]
    );
    // The disaster-recovery Saturday is a special session, never a regular day.
    let special = import(
        &root,
        Src::Zips,
        ImportKind::Indices,
        day(2024, 3, 2),
        day(2024, 3, 2),
        &store,
    );
    assert_eq!(special.days_refused, 1);
    assert!(special.failures[0].instrument.ends_with("calendar"));
    let unmeasured = import(
        &root,
        Src::Zips,
        ImportKind::Options,
        day(2011, 1, 3),
        day(2011, 1, 3),
        &store,
    );
    assert_eq!(unmeasured.days_refused, 1);
    let run = Run {
        kind: ImportKind::Indices,
        from: d2(),
        to: d1(),
        only: &[],
        store_root: &store,
    };
    assert_eq!(
        run_cm(&TickStore::new(&root.join("ts")), &run).unwrap_err(),
        ImportRefusal::RangeBackwards {
            from: d2(),
            to: d1()
        }
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_day_older_than_what_the_month_holds_is_refused_by_the_store() {
    let root = scratch("import-order");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &store);
    let late = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    assert!(!late.failures.is_empty(), "never inserted behind the tail");
    assert_eq!(late.seconds_committed, 0);
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(text.contains("incomplete indices 2024-04-01"), "{text}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_corrupt_file_is_refused_and_its_neighbours_still_land() {
    let root = scratch("import-corrupt");
    world(&root, &[d1()]);
    // Flip a byte inside the tick store's first options block.
    let path = root.join("ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[12] ^= 0xFF;
    std::fs::write(&path, bytes).unwrap();
    let store = root.join("S");
    let got = import(&root, Src::Store, ImportKind::Options, d1(), d1(), &store);
    assert_eq!(got.files_refused, 1);
    assert_eq!(got.files, 2, "the other two contracts landed");
    assert!(got.failures[0].instrument.contains("NIFTY04APR2422000CE"));
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(
        text.contains("incomplete options"),
        "a day with a refusal is retried"
    );
    // An unreadable listing is one named failure for the day.
    std::fs::write(&path, b"not a day file").unwrap();
    let again = import(
        &root,
        Src::Store,
        ImportKind::Options,
        d1(),
        d1(),
        &root.join("T"),
    );
    assert!(again.failures[0].instrument.ends_with("listing"));
    std::fs::write(
        root.join("ts/cm/INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_01042024.bts"),
        b"no",
    )
    .unwrap();
    let cm = import(
        &root,
        Src::Store,
        ImportKind::Indices,
        d1(),
        d1(),
        &root.join("U"),
    );
    assert!(cm.failures[0].instrument.ends_with("listing"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_reader_refusal_in_a_capital_market_file_is_named() {
    let root = scratch("import-cm-refusal");
    world(&root, &[d1()]);
    // A stock file with no traded session row: the reader refuses it.
    let folder = crate::gdfl_cm::day_folder_name(CmKind::Stocks, d1());
    let dead = csv(&[row("SBIN.NSE", d1(), NINE_FIFTEEN, "1.00", 0, 0)]);
    let entries = vec![
        (
            format!("{folder}/RELIANCE.NSE.csv"),
            stock_file("RELIANCE.NSE", d1()),
        ),
        (format!("{folder}/SBIN.NSE.csv"), dead),
    ];
    put(
        &root,
        "ts/cm/STOCKS/2024/APR_2024/GFDLCM_STOCK_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let got = import(
        &root,
        Src::Store,
        ImportKind::Stocks,
        d1(),
        d1(),
        &root.join("S"),
    );
    assert_eq!((got.files, got.files_refused), (1, 1));
    assert!(got.failures[0].instrument.contains("SBIN.NSE.csv"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn two_tickers_naming_one_contract_are_both_refused() {
    let root = scratch("import-ambiguous");
    // Before the switch an index monthly can be named in both forms: the
    // day form `NIFTY27DEC1810500CE` and the monthly `NIFTY18DEC10500CE`
    // (expiry Thu 27 Dec 2018) name one contract.
    let d = day(2018, 12, 3);
    let folder = crate::gdfl_nfo::day_folder_name(d);
    let entries: Vec<(String, Vec<u8>)> = [
        "NIFTY27DEC1810500CE",
        "NIFTY18DEC10500CE",
        "NIFTY18DEC10600CE",
    ]
    .iter()
    .map(|t| {
        (
            format!("{folder}\\Options\\{t}.NFO.csv"),
            option_file(t, d, "5"),
        )
    })
    .collect();
    put(
        &root,
        "ts/options/2018/DEC_2018/GFDLNFO_TICK_03122018.bts",
        &bts(&as_refs(&entries)),
    );
    let got = import(
        &root,
        Src::Store,
        ImportKind::Options,
        d,
        d,
        &root.join("S"),
    );
    assert_eq!((got.files, got.files_refused), (1, 2), "{:?}", got.failures);
    assert!(got.failures.iter().all(|f| f.why.contains("same contract")));
    // A filter that excludes them refuses nothing.
    let none = import_only(
        &root,
        Src::Store,
        ImportKind::Options,
        d,
        d,
        &root.join("T"),
        &["BANKNIFTY".to_owned()],
    );
    assert!(none.failures.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_filter_takes_only_the_named_instruments_and_keys_the_journal() {
    let root = scratch("import-only");
    world(&root, &[d1()]);
    let store = root.join("S");
    let only = ["BANKNIFTY".to_owned(), "BANKNIFTY".to_owned()];
    let got = import_only(
        &root,
        Src::Store,
        ImportKind::Options,
        d1(),
        d1(),
        &store,
        &only,
    );
    assert_eq!((got.files, got.files_skipped), (1, 2));
    let spot = import_only(
        &root,
        Src::Zips,
        ImportKind::Indices,
        d1(),
        d1(),
        &store,
        &["NIFTY".to_owned()],
    );
    assert_eq!(spot.files, 1);
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(
        text.contains("done options 2024-04-01 BANKNIFTY "),
        "{text}"
    );
    assert!(text.contains("done indices 2024-04-01 NIFTY "), "{text}");
    // An undecodable ticker is refused when it could be one of the filter's.
    let folder = crate::gdfl_nfo::day_folder_name(d1());
    let entries = vec![
        (
            format!("{folder}\\Options\\NIFTYJUNK.NFO.csv"),
            option_file("NIFTYJUNK", d1(), "5"),
        ),
        (
            format!("{folder}\\Options\\ACCJUNK.NFO.csv"),
            option_file("ACCJUNK", d1(), "5"),
        ),
        (format!("{folder}\\Futures\\NIFTY-I.NFO.csv"), Vec::new()),
    ];
    put(
        &root,
        "ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let junk = import_only(
        &root,
        Src::Store,
        ImportKind::Options,
        d1(),
        d1(),
        &root.join("T"),
        &["NIFTY".to_owned()],
    );
    assert_eq!((junk.files_refused, junk.files_skipped), (1, 2));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_source_of_the_wrong_kind_is_refused_by_name() {
    let store = scratch("import-kind");
    let run = Run {
        kind: ImportKind::Options,
        from: d1(),
        to: d1(),
        only: &[],
        store_root: &store,
    };
    assert!(matches!(
        run_cm(&TickStore::new(&store), &run),
        Err(ImportRefusal::KindUnknown { .. })
    ));
    let spot = Run {
        kind: ImportKind::Stocks,
        ..run
    };
    assert!(matches!(
        run_nfo(&NfoTickStore::new(&store), &spot),
        Err(ImportRefusal::KindUnknown { .. })
    ));
    std::fs::remove_dir_all(store).unwrap();
}

#[test]
fn a_census_that_will_not_publish_fails_the_day_by_name() {
    let root = scratch("import-census");
    world(&root, &[d1()]);
    let store = root.join("S");
    // A directory where the census file should be.
    std::fs::create_dir_all(store.join("manifest/gdfl.man")).unwrap();
    let got = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    assert!(
        got.failures
            .iter()
            .any(|f| f.instrument.ends_with("census")),
        "{:?}",
        got.failures
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_gdfl_bar_is_folded_into_no_coarser_rung_on_any_ingest_door() {
    assert!(!crate::ingest::derives_for(Vendor::Gdfl));
    assert!(crate::ingest::derives_for(Vendor::Zerodha));
    assert_eq!(
        crate::ingest::derived_count_for(Vendor::Gdfl, None, Timeframe::MINUTE_1),
        0
    );
    assert_eq!(
        crate::ingest::derived_count_for(Vendor::Zerodha, None, Timeframe::MINUTE_1),
        crate::ingest::derived_count(Timeframe::MINUTE_1)
    );
    // Through `from_members` at one second: one file, no failure, no rung.
    let store = scratch("import-derive");
    let window = Window::new(d1(), d1()).unwrap();
    let request = BarRequest {
        instrument_id: String::new(),
        listing: Listing::Index,
        window,
        granularity: Granularity::Second1,
    };
    let secs =
        i64::from(d1().days_from_epoch()) * 86_400 + i64::from(NINE_FIFTEEN) - IST_OFFSET_SECS;
    let member = crate::archive::Member {
        path: PathBuf::from("invented"),
        instrument: "NIFTY".to_owned(),
        rows: (0..120)
            .map(|s| crate::fetch::RawRow {
                timestamp: secs + s,
                open: 100,
                high: 100,
                low: 100,
                close: 100,
                volume: 0,
                open_interest: None,
            })
            .collect(),
    };
    let done =
        crate::ingest::from_members(&[member], &store, plan(ImportKind::Indices, &request, None));
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.derived_files, 0);
    let rungs: Vec<String> = tree(&store)
        .into_keys()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|p| p.starts_with("bars/") && p.ends_with(".bin"))
        .collect();
    assert_eq!(rungs.len(), 1, "{rungs:?}");
    assert!(rungs[0].contains("/1s/"));
    std::fs::remove_dir_all(store).unwrap();
}

// ── properties of the one-second build ─────────────────────────────────────

/// A seeded generator, so every run tests the same sequences.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// The rule written the slow, obvious way: keep (every row for an index,
/// traded rows otherwise), place each late row at the next in-order stamp,
/// drop late rows with none after them, then per placed second: first,
/// max, min, last, sum, last OI.
fn reference(kind: ImportKind, day: Day, ticks: &[Tick]) -> Vec<Bar> {
    // The clock is every row; the bar is the kept rows (D-2812).
    let kept = ticks;
    let mut placed: Vec<(u32, Tick)> = Vec::new();
    for (i, t) in kept.iter().enumerate() {
        let max_before = kept[..i].iter().map(|x| x.sod).max();
        let late = max_before.is_some_and(|m| t.sod < m);
        let second = if late {
            // The next in-order row after it.
            let mut running = max_before.unwrap();
            let mut found = None;
            for x in &kept[i + 1..] {
                if x.sod >= running {
                    found = Some(x.sod);
                    break;
                }
                running = running.max(x.sod);
            }
            found
        } else {
            Some(t.sod)
        };
        if let Some(s) = second.filter(|_| kind.every_row_counts() || t.ltq > 0) {
            placed.push((s, *t));
        }
    }
    let mut out: Vec<Bar> = Vec::new();
    for (s, t) in placed {
        let volume = if kind.every_row_counts() {
            0
        } else {
            i64::try_from(t.ltq).unwrap()
        };
        let oi = t.oi.unwrap_or(OI_NULL);
        match out.last_mut() {
            Some(bar) if bar.ts_micros == micros_at(day, s) => {
                bar.high = bar.high.max(t.ltp);
                bar.low = bar.low.min(t.ltp);
                bar.close = t.ltp;
                bar.volume += volume;
                if oi != OI_NULL {
                    bar.open_interest = oi;
                }
            }
            _ => out.push(Bar {
                ts_micros: micros_at(day, s),
                open: t.ltp,
                high: t.ltp,
                low: t.ltp,
                close: t.ltp,
                volume,
                open_interest: oi,
            }),
        }
    }
    out
}

#[test]
fn the_one_second_build_equals_the_rule_written_the_slow_way() {
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    for case in 0..2_000 {
        let kind = ImportKind::ALL[usize::try_from(rng.below(3)).unwrap()];
        let n = rng.below(120);
        let mut sod = NINE_FIFTEEN - 5 + u32::try_from(rng.below(10)).unwrap();
        let mut ticks = Vec::new();
        for _ in 0..n {
            match rng.below(10) {
                // Back-steps of every size, duplicates and forward jumps.
                0 => sod = sod.saturating_sub(u32::try_from(rng.below(40)).unwrap()),
                1 | 2 => {}
                3 => sod += u32::try_from(rng.below(600)).unwrap(),
                _ => sod += 1,
            }
            let ltq = if rng.below(3) == 0 {
                0
            } else {
                rng.below(1_000)
            };
            let oi =
                (kind == ImportKind::Options).then(|| i64::try_from(rng.below(1_000_000)).unwrap());
            ticks.push(Tick {
                sod,
                ltp: 5 + i64::try_from(rng.below(100_000)).unwrap(),
                ltq,
                oi,
            });
        }
        let got = convert(kind, d1(), &ticks).unwrap();
        assert_eq!(
            got.seconds,
            reference(kind, d1(), &ticks),
            "case {case} {kind:?}"
        );
        assert!(
            got.seconds
                .windows(2)
                .all(|w| w[0].ts_micros < w[1].ts_micros),
            "strictly increasing"
        );
        let p = got.placement;
        assert_eq!(p.rows, ticks.len());
        assert!(p.ltq_zero_dropped + p.late_unresolved <= p.rows);
    }
}

#[test]
fn the_session_edges_keep_0915_00_and_1529_59_and_count_what_is_outside() {
    let root = scratch("import-edges");
    let d = d1();
    let folder = crate::gdfl_nfo::day_folder_name(d);
    let stem = "NIFTY04APR2422000CE.NFO";
    let rows = csv(&[
        row(stem, d, NINE_FIFTEEN - 1, "1", 5, 1),
        row(stem, d, NINE_FIFTEEN, "2", 5, 2),
        row(stem, d, CLOSE - 1, "3", 5, 3),
        row(stem, d, CLOSE, "4", 5, 4),
    ]);
    let entries = vec![(
        format!("{folder}\\Options\\NIFTY04APR2422000CE.NFO.csv"),
        rows,
    )];
    put(
        &root,
        "ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let store = root.join("S");
    let got = import(&root, Src::Store, ImportKind::Options, d, d, &store);
    assert_eq!(
        (got.seconds_committed, got.outside_session),
        (2, 2),
        "{got:?}"
    );
    let held = bars(&store, "FNO", "NIFTY", Some(nifty_ce()));
    assert_eq!(held[0].ts_micros, micros_at(d, NINE_FIFTEEN));
    assert_eq!(held[1].ts_micros, micros_at(d, CLOSE - 1));
    assert_eq!(
        (held[0].close, held[1].close, held[1].open_interest),
        (200, 300, 3)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_fold_refusal_other_than_volume_is_named_in_the_folds_words() {
    let why = crate::fold::FoldError::OutOfOrder {
        at: 1,
        previous: 2,
        found: 1,
    };
    assert!(
        matches!(fold_refusal(why, d1()), ImportRefusal::Fold { why } if why.contains("OutOfOrder"))
    );
}

#[test]
fn an_incomplete_day_is_closed_in_the_journal_and_retried() {
    let root = scratch("import-incomplete");
    world(&root, &[d1()]);
    let store = root.join("S");
    put(
        &store,
        "imports/gdfl.journal",
        b"begin indices 2024-04-01 *\nincomplete indices 2024-04-01 * files=1\n",
    );
    let got = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    assert!(
        got.resumed.is_empty(),
        "an incomplete day is closed, not crashed"
    );
    assert_eq!(got.days_imported, 1, "and imported again");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_quantity_past_i64_in_a_real_file_refuses_that_file_by_name() {
    let root = scratch("import-huge");
    let folder = crate::gdfl_cm::day_folder_name(CmKind::Stocks, d1());
    let huge = csv(&[row(
        "SBIN.NSE",
        d1(),
        NINE_FIFTEEN,
        "1.00",
        9_223_372_036_854_775_808,
        0,
    )]);
    let entries = vec![
        (
            format!("{folder}/RELIANCE.NSE.csv"),
            stock_file("RELIANCE.NSE", d1()),
        ),
        (format!("{folder}/SBIN.NSE.csv"), huge),
        // Two names for one ticker: refused once, the twin skipped.
        (
            format!("{folder}/SBIN.NSE.CSV"),
            stock_file("SBIN.NSE", d1()),
        ),
    ];
    put(
        &root,
        "ts/cm/STOCKS/2024/APR_2024/GFDLCM_STOCK_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let got = import(
        &root,
        Src::Store,
        ImportKind::Stocks,
        d1(),
        d1(),
        &root.join("S"),
    );
    assert_eq!(got.files, 1, "{:?}", got.failures);
    assert_eq!(got.files_refused, 1, "{:?}", got.failures);
    // Options: a trading day the source does not hold.
    let none = import(
        &root,
        Src::Store,
        ImportKind::Options,
        d1(),
        d1(),
        &root.join("T"),
    );
    assert_eq!(none.days_missing, vec![d1()]);
    // The huge quantity, through the options reader into the converter.
    let ofolder = crate::gdfl_nfo::day_folder_name(d1());
    let t = "NIFTY04APR2422000CE";
    let entries = vec![(
        format!("{ofolder}\\Options\\{t}.NFO.csv"),
        csv(&[row(
            &format!("{t}.NFO"),
            d1(),
            NINE_FIFTEEN,
            "5",
            9_223_372_036_854_775_808,
            1,
        )]),
    )];
    put(
        &root,
        "ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let big = import(
        &root,
        Src::Store,
        ImportKind::Options,
        d1(),
        d1(),
        &root.join("U"),
    );
    assert_eq!((big.files, big.files_refused), (0, 1), "{:?}", big.failures);
    assert!(big.failures[0].why.contains("does not fit an i64"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_done_line_states_the_days_exact_counts() {
    let root = scratch("import-done-line");
    world(&root, &[d1()]);
    let store = root.join("S");
    let got = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    let want = format!(
        "done indices 2024-04-01 * files={} seconds={} failures=0 late={} late_unresolved={} max_back_s={}",
        got.files, got.seconds, got.late_rows, got.late_unresolved, got.max_back_s
    );
    assert!(text.contains(&want), "{text} lacks {want}");
    assert_eq!(
        (got.files, got.late_rows),
        (2, 2),
        "two files, one late row each"
    );
    std::fs::remove_dir_all(root).unwrap();
}
