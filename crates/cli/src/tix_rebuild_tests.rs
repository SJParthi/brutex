//! `cli tix-rebuild` on invented stores: misuse, the receipt, the exit, the
//! lock, a month no index can describe, and the CM index writer sealing a
//! `.tix` on every new month. D-3340, WP-15; DB-064, DB-G81.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, TimeLookup};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::time_index::Why;

use super::{USAGE, command_with};
use crate::{FAILED, MISUSED, OK};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A store root that removes itself.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-tix-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    /// Runs the verb against this root.
    fn run(&self, arguments: &[&str]) -> (u8, String) {
        let mut out = String::new();
        let root = self.0.clone();
        let code = command_with(arguments, &move || Ok(root.clone()), &mut out);
        (code, out)
    }

    fn tix(&self, timeframe: Timeframe, month: YearMonth) -> PathBuf {
        path(timeframe, month)
            .with_file(FileKind::TimeIndex)
            .to_path_buf(&self.0)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).unwrap()
}

fn path(timeframe: Timeframe, month: YearMonth) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe,
        month,
        file: FileKind::Bars,
    })
    .unwrap()
}

/// NIFTY's header symbol id, as every writer derives it.
fn nifty_id() -> u32 {
    let [a, b, c, d, ..] = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
    u32::from_le_bytes([a, b, c, d])
}

/// One invented bar at `ts_micros`, at a level NIFTY never printed.
fn bar(ts_micros: i64) -> Bar {
    Bar {
        ts_micros,
        open: 424_200,
        high: 424_300,
        low: 424_100,
        close: 424_250,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// 09:15 IST on day `day` (0-based) of `month`, plus `secs`.
fn at(month: YearMonth, day: i64, secs: i64) -> i64 {
    month.ist_bounds_micros().0 + (day * 86_400 + 555 * 60 + secs) * 1_000_000
}

/// Writes one session of `count` bars `step` seconds apart on day 2 of
/// `month`, through the writer, and optionally takes its `.tix` away — the
/// shape a month written before D-2329 has.
fn month_of(
    root: &Root,
    timeframe: Timeframe,
    month: YearMonth,
    count: i64,
    step: i64,
    keep_tix: bool,
) {
    let bars: Vec<Bar> = (0..count).map(|k| bar(at(month, 2, k * step))).collect();
    let mut writer = BarFile::open_or_create(&root.0, path(timeframe, month), nifty_id()).unwrap();
    assert!(matches!(
        writer.append(&bars),
        Ok(Appended::Committed { .. })
    ));
    drop(writer);
    if !keep_tix {
        fs::remove_file(root.tix(timeframe, month)).unwrap();
    }
}

fn reader(root: &Root, timeframe: Timeframe, month: YearMonth) -> BarFile {
    BarFile::open_existing(&root.0, path(timeframe, month), nifty_id()).unwrap()
}

/// Every file under `dir`, recursively, sorted.
fn tree(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let Ok(entries) = fs::read_dir(&at) else {
            continue;
        };
        for entry in entries {
            let entry = entry.unwrap().path();
            if entry.is_dir() {
                stack.push(entry.clone());
            }
            out.push(entry);
        }
    }
    out.sort();
    out
}

#[test]
fn misuse_is_refused_by_name_before_the_store_is_asked() {
    let cases: &[(&[&str], &str)] = &[
        (&[], "five arguments"),
        (
            &["gdfl", "NSE/INDEX/NIFTY", "1s", "2019-01"],
            "five arguments",
        ),
        (
            &[
                "gdfl",
                "NSE/INDEX/NIFTY",
                "1s",
                "2019-01",
                "2019-02",
                "--force",
            ],
            "five arguments",
        ),
        (
            &["nofeed", "NSE/INDEX/NIFTY", "1s", "2019-01", "2019-02"],
            "is not a feed this build knows",
        ),
        (
            &["gdfl", "NSE/NIFTY", "1s", "2019-01", "2019-02"],
            "is not EXCHANGE/SEGMENT/SYMBOL",
        ),
        (
            &[
                "gdfl",
                "NSE/INDEX/NIFTY/2019-03-28-1150000-CE",
                "1s",
                "2019-01",
                "2019-02",
            ],
            "is not EXCHANGE/SEGMENT/SYMBOL",
        ),
        (
            &["gdfl", "NSE//NIFTY", "1s", "2019-01", "2019-02"],
            "has an empty segment",
        ),
        (
            &["gdfl", "NSE/INDEX/NIFTY", "1sec", "2019-01", "2019-02"],
            "is not a rung this store carries",
        ),
        (
            &["gdfl", "NSE/INDEX/NIFTY", "1s", "2019-1", "2019-02"],
            "FROM `2019-1` is not a YYYY-MM month",
        ),
        (
            &["gdfl", "NSE/INDEX/NIFTY", "1s", "2019-01", "201902"],
            "TO `201902` is not a YYYY-MM month",
        ),
        (
            &["gdfl", "NSE/INDEX/NIFTY", "1s", "2019-13", "2019-02"],
            "FROM `2019-13`",
        ),
        (
            &["gdfl", "NSE/INDEX/NIFTY", "1s", "2019-03", "2019-02"],
            "runs backwards",
        ),
        (
            &["gdfl", "NSE/INDEX/NI FTY", "1s", "2019-01", "2019-02"],
            "is not a store path",
        ),
    ];
    for (arguments, says) in cases {
        let asked = Cell::new(false);
        let store = || {
            asked.set(true);
            Err("the store was asked".to_owned())
        };
        let mut out = String::new();
        assert_eq!(
            command_with(arguments, &store, &mut out),
            MISUSED,
            "{arguments:?}: {out}"
        );
        assert!(out.starts_with("refused: "), "{out}");
        assert!(
            out.contains(says),
            "{arguments:?}: wanted {says:?} in {out}"
        );
        assert!(out.contains(USAGE), "{out}");
        assert!(!asked.get(), "{arguments:?}: the store root was asked for");
    }
}

#[test]
fn a_store_root_that_cannot_be_proven_fails_and_touches_nothing() {
    let mut out = String::new();
    let code = command_with(
        &["gdfl", "NSE/INDEX/NIFTY", "1s", "2019-01", "2019-02"],
        &|| Err("store root `/nowhere` could not be canonicalized".to_owned()),
        &mut out,
    );
    assert_eq!(code, FAILED);
    assert_eq!(
        out,
        "TIX REBUILD REFUSED: store root `/nowhere` could not be canonicalized. Nothing was read or written.\n"
    );
}

#[test]
fn the_named_months_are_built_kept_or_named_and_the_exit_says_whether_every_one_is_indexed() {
    let tf = Timeframe::SECOND_1;
    let root = Root::new();
    // 2019-01 and 2019-02 as an import before D-2329 left them, 2019-04 with
    // its index, 2019-03 never held.
    month_of(&root, tf, ym(2019, 1), 22_500, 1, false);
    month_of(&root, tf, ym(2019, 2), 3_000, 7, false);
    month_of(&root, tf, ym(2019, 4), 500, 1, true);
    let held_tix = fs::read(root.tix(tf, ym(2019, 4))).unwrap();
    let before = tree(&root.0);

    // The dry run says what a reader sees and writes nothing.
    let span = ["gdfl", "NSE/INDEX/NIFTY", "1s", "2019-01", "2019-04"];
    let (code, out) = root.run(&[&span[..], &["--dry-run"]].concat());
    assert_eq!(code, FAILED, "a month in the span is not held: {out}");
    let absent = Why::Absent.to_string();
    let expected = format!(
        "TIX REBUILD (dry run: nothing is written) — gdfl NSE/INDEX/NIFTY 1s, 2019-01..2019-04: 4 month(s) under {}\n\
         \x20 2019-01  would build       22500 bars  a reader bisects: {absent}\n\
         \x20 2019-02  would build        3000 bars  a reader bisects: {absent}\n\
         \x20 2019-03  NOT HELD      no such month; nothing was created\n\
         \x20 2019-04  indexed             500 bars\n\
         Indexed 1, would build 2, not held 1, refused 0. Nothing was written.\n",
        root.0.display()
    );
    assert_eq!(out, expected);
    assert_eq!(tree(&root.0), before, "a dry run writes nothing");

    // The rebuild: two built, one kept, one not held — and so FAILED.
    let (code, out) = root.run(&span);
    assert_eq!(code, FAILED, "{out}");
    let expected = format!(
        "TIX REBUILD — gdfl NSE/INDEX/NIFTY 1s, 2019-01..2019-04: 4 month(s) under {}\n\
         \x20 2019-01  built             22500 bars  because {absent}; a reader now goes through the index\n\
         \x20 2019-02  built              3000 bars  because {absent}; a reader now goes through the index\n\
         \x20 2019-03  NOT HELD      no such month; nothing was created\n\
         \x20 2019-04  kept                500 bars  already indexed; nothing written\n\
         Built 2, kept 1, unindexable 0, not indexed 0, not held 1, refused 0: NOT every month in the \
         span is indexed; rerun once the cause is gone (a rerun keeps what is built)\n",
        root.0.display()
    );
    assert_eq!(out, expected);
    let mut grown = before.clone();
    grown.extend([root.tix(tf, ym(2019, 1)), root.tix(tf, ym(2019, 2))]);
    grown.sort();
    assert_eq!(
        tree(&root.0),
        grown,
        "two .tix and nothing else; nothing for 2019-03"
    );
    assert_eq!(fs::read(root.tix(tf, ym(2019, 4))).unwrap(), held_tix);
    for month in [ym(2019, 1), ym(2019, 2), ym(2019, 4)] {
        assert_eq!(reader(&root, tf, month).time_lookup(), TimeLookup::Indexed);
    }
    // A probe answered through the new index is the definition.
    let january = reader(&root, tf, ym(2019, 1));
    assert_eq!(
        january.first_at_or_after(at(ym(2019, 1), 2, 300) + 1),
        Ok(301)
    );
    drop(january);

    // The held months alone: every one kept, and OK.
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2019-01", "2019-02"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("  2019-01  kept "), "{out}");
    assert!(out.contains("  2019-02  kept "), "{out}");
    assert!(
        out.ends_with(
            "Built 0, kept 2, unindexable 0, not indexed 0, not held 0, refused 0: every month in \
             the span is indexed\n"
        ),
        "{out}"
    );
    // And a dry run over them is clean too.
    let (code, out) = root.run(&[
        "gdfl",
        "NSE/INDEX/NIFTY",
        "1s",
        "2019-01",
        "2019-02",
        "--dry-run",
    ]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.ends_with("Indexed 2, would build 0, not held 0, refused 0. Nothing was written.\n")
    );
}

#[test]
fn a_month_another_process_holds_is_refused_and_the_rest_are_built() {
    let tf = Timeframe::MINUTE_1;
    let root = Root::new();
    month_of(&root, tf, ym(2019, 1), 375, 60, false);
    month_of(&root, tf, ym(2019, 2), 375, 60, false);
    let span = ["gdfl", "NSE/INDEX/NIFTY", "1min", "2019-01", "2019-02"];

    // A reader mid-request holds February shared.
    let held = reader(&root, tf, ym(2019, 2));
    let (code, out) = root.run(&span);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("  2019-01  built "), "{out}");
    assert!(
        out.contains(&format!(
            "  2019-02  REFUSED       {} is held by another process (a writer, or a reader \
             mid-request) and is not waited for; nothing was written to it\n",
            path(tf, ym(2019, 2))
                .with_file(FileKind::Lock)
                .to_path_buf(&root.0)
                .display()
        )),
        "{out}"
    );
    assert!(!root.tix(tf, ym(2019, 2)).exists());
    drop(held);

    // Released: January is kept, February built, and the run is clean.
    let (code, out) = root.run(&span);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("  2019-01  kept "), "{out}");
    assert!(out.contains("  2019-02  built "), "{out}");
}

#[test]
fn a_month_no_index_can_describe_fails_the_run_and_keeps_no_index() {
    // Two daily bars on one IST day (D-2330): named, left without a `.tix`.
    let tf = Timeframe::DAY_1;
    let root = Root::new();
    let month = ym(2019, 1);
    let mut writer = BarFile::open_or_create(&root.0, path(tf, month), nifty_id()).unwrap();
    assert!(writer.append(&[bar(at(month, 2, 0))]).is_ok());
    assert!(writer.append(&[bar(at(month, 2, 3_600))]).is_ok());
    drop(writer);
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1day", "2019-01", "2019-01"]);
    assert_eq!(code, FAILED, "{out}");
    let shared = Why::SharedSlot {
        index: 1,
        ts_micros: at(month, 2, 3_600),
    };
    assert!(
        out.contains(&format!(
            "  2019-01  UNINDEXABLE           2 bars  no .tix is kept and lookups bisect: {shared}\n"
        )),
        "{out}"
    );
    assert!(!root.tix(tf, month).exists());
}

#[test]
fn the_cm_index_writer_seals_a_tix_on_every_new_month() {
    // D-3340: the GDFL capital-market index import writes through the store's
    // writer door, which seals a `.tix` on every month it opens. Driven here
    // through the import runtime itself, over two days in two months, with
    // invented ticks; then every bar file the import wrote, at every rung,
    // has an index a reader goes through, and the verb finds nothing to do.
    use pull::gdfl_import::{DayRead, ImportKind, Run, Tick, TickFile, drive};
    use pull::session::Day;

    let root = Root::new();
    let run = Run {
        kind: ImportKind::Indices,
        from: Day::new(2019, 1, 31).unwrap(),
        to: Day::new(2019, 2, 1).unwrap(),
        only: &[],
        store_root: &root.0,
    };
    let report = drive(&run, |_day, sink| {
        let ticks = (0..3_214u32)
            .map(|k| Tick {
                sod: 33_300 + k * 7,
                ltp: 424_200 + i64::from(k % 50),
                ltq: 0,
                oi: None,
            })
            .collect();
        sink(TickFile {
            symbol: "NIFTY".to_owned(),
            contract: None,
            name: "NIFTY 50.NSE_IDX.csv".to_owned(),
            ticks,
        });
        DayRead {
            held: true,
            ..DayRead::default()
        }
    })
    .unwrap();
    assert!(report.seconds_committed > 0, "{report:?}");

    let bins: Vec<PathBuf> = tree(&root.0.join("bars"))
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "bin"))
        .collect();
    let one_second: Vec<&PathBuf> = bins
        .iter()
        .filter(|path| path.parent().is_some_and(|dir| dir.ends_with("1s")))
        .collect();
    assert_eq!(
        one_second.len(),
        2,
        "one new month each side of the boundary: {bins:?}"
    );
    for bin in &bins {
        assert!(
            bin.with_extension("tix").is_file(),
            "{} was written without its .tix",
            bin.display()
        );
    }
    for month in [ym(2019, 1), ym(2019, 2)] {
        assert_eq!(
            reader(&root, Timeframe::SECOND_1, month).time_lookup(),
            TimeLookup::Indexed
        );
    }
    let (code, out) = root.run(&["gdfl", "NSE/INDEX/NIFTY", "1s", "2019-01", "2019-02"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("  2019-01  kept "), "{out}");
    assert!(out.contains("  2019-02  kept "), "{out}");
}
