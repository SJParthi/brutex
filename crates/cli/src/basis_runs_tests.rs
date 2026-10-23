//! `cli basis-runs` end to end: GDFL raw seconds and Zerodha back-adjusted
//! minutes written with the store's own writer, DATA-1's `feed-reconcile`
//! run over them, and this verb folding its files (R4-26: Z8, COR-06,
//! FM-260). Every price is invented (a level near Rs 4,321 scaled by exact
//! factors), every symbol is invented, and no vendor row is copied.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use pull::corporate_actions::{CorporateActions, Kind, Tag};
use pull::reference_data::{Draft, Slot, Standing, Table, load_one, write};
use pull::session::{Day, IST_OFFSET_SECS};
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{Ask, USAGE, ask, command_with, months_in};
use crate::{FAILED, MISUSED, OK};

static NEXT: AtomicU64 = AtomicU64::new(0);
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const BASE: i64 = 432_100;

/// A fresh directory of this test's own under the system temp dir.
struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "brutex-cli-basis-runs-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        // Only ever a directory `Dir::new` made in THIS process: directly
        // under the temp dir, named with this file's prefix and this
        // process's id (gate 23 clause C).
        let under_temp = self.0.parent() == Some(std::env::temp_dir().as_path());
        let pid = format!("-{}-", std::process::id());
        let own = self.0.file_name().is_some_and(|name| {
            let name = name.to_string_lossy();
            name.starts_with("brutex-cli-basis-runs-") && name.contains(pid.as_str())
        });
        if under_temp && own {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// UTC microseconds of an IST wall time.
fn at(on: Day, minute_of_day: i64, second: i64) -> i64 {
    let days = i64::from(on.days_from_epoch());
    (days * 86_400 - IST_OFFSET_SECS + minute_of_day * 60 + second) * 1_000_000
}

fn bar(ts_micros: i64, open: i64, high: i64, low: i64, close: i64, volume: i64) -> Bar {
    Bar {
        ts_micros,
        open,
        high,
        low,
        close,
        volume,
        open_interest: OI_NULL,
    }
}

/// One bar file of the fixture: (symbol, vendor, rung seconds, (year, month)).
type FileKey = (String, Vendor, u32, (u16, u8));

/// Bars per (symbol, vendor, rung seconds, month), kept in stamp order.
#[derive(Default)]
struct Fixture {
    files: BTreeMap<FileKey, Vec<Bar>>,
}

impl Fixture {
    /// Four minutes from 09:15 of `on`: GDFL seconds at :00 and :30, and,
    /// unless `gdfl_only`, Zerodha minutes equal to GDFL's fold times
    /// `num / den` (Zerodha back-adjusts; GDFL is raw).
    fn day(&mut self, symbol: &str, on: Day, (num, den): (i64, i64), gdfl_only: bool) -> &mut Self {
        let month = (on.year(), on.month());
        let (mut seconds, mut minutes) = (Vec::new(), Vec::new());
        for n in 0..4 {
            let minute = 9 * 60 + 15 + n;
            let p = BASE + 2 * n;
            seconds.push(bar(at(on, minute, 0), p, p + 4, p - 2, p + 2, 10));
            seconds.push(bar(at(on, minute, 30), p + 2, p + 6, p, p + 4, 10));
            let k = |x: i64| x * num / den;
            minutes.push(bar(
                at(on, minute, 0),
                k(p),
                k(p + 6),
                k(p - 2),
                k(p + 4),
                20,
            ));
        }
        let key = |vendor, rung: Timeframe| (symbol.to_owned(), vendor, rung.secs(), month);
        self.files
            .entry(key(Vendor::Gdfl, Timeframe::SECOND_1))
            .or_default()
            .extend(seconds);
        if !gdfl_only {
            self.files
                .entry(key(Vendor::Zerodha, Timeframe::MINUTE_1))
                .or_default()
                .extend(minutes);
        }
        self
    }

    fn write(&self, root: &Path) {
        for ((symbol, vendor, secs, (year, month)), bars) in &self.files {
            let path = StorePath::new(PathParts {
                vendor: *vendor,
                exchange: "NSE",
                segment: "CASH",
                symbol,
                contract: None,
                timeframe: Timeframe::from_secs(*secs).unwrap(),
                month: YearMonth::new(*year, *month).unwrap(),
                file: FileKind::Bars,
            })
            .unwrap();
            let id = u32::try_from(brutex_core::universe::fnv1a(symbol) & 0xffff_ffff).unwrap();
            let mut file = BarFile::open_or_create(root, path, id).unwrap();
            file.append(bars).unwrap();
        }
    }
}

/// DATA-1's `feed-reconcile` over `store` for `month`, into `out`.
fn reconcile(store: &Path, out: &Path, month: &str) {
    let root = store.to_path_buf();
    let out_text = out.display().to_string();
    let mut text = String::new();
    let code = crate::feed_reconcile::command_with(
        &[month, "--out", out_text.as_str()],
        &move || Ok(root.clone()),
        None,
        Some(COMMIT),
        &mut text,
    );
    assert_eq!(code, OK, "{text}");
}

fn basis_runs(store: &Path, arguments: &[&str]) -> (u8, String) {
    let root = store.to_path_buf();
    let mut text = String::new();
    let code = command_with(
        arguments,
        &move || Ok(root.clone()),
        Some(day(2026, 10, 22)),
        &mut text,
    );
    (code, text)
}

/// SPLITCO: a 2:1 split on 2025-03-11, five sessions a side (D-7139: a side
/// must hold to be lasting). Zerodha reads half of GDFL before
/// it and equal from it; 2025-03-03 seeds the lag history DATA-1 aligns by.
/// IDLECO: GDFL only, so no stock-day has a basis.
fn split_store(store: &Path) {
    let mut fixture = Fixture::default();
    for d in [3, 4, 5, 6, 7, 10] {
        fixture.day("SPLITCO", day(2025, 3, d), (1, 2), false);
    }
    for d in [11, 12, 13, 14, 17] {
        fixture.day("SPLITCO", day(2025, 3, d), (1, 1), false);
    }
    fixture.day("IDLECO", day(2025, 3, 4), (1, 1), true);
    fixture.write(store);
}

fn dir_text(dir: &Dir) -> String {
    dir.0.display().to_string()
}

// ─────────────────────────────── arguments ───────────────────────────────

#[test]
fn every_misuse_is_refused_by_name_with_the_usage() {
    for (arguments, said) in [
        (&[][..], "--dir DIR is required"),
        (&["--dir"][..], "--dir needs a value"),
        (&["--dir", "a", "--dir", "b"][..], "given twice"),
        (&["--dir", "a", "--from", "2025-13"][..], "2025-13"),
        (
            &["--dir", "a", "--from", "2025-04", "--to", "2025-03"][..],
            "is after --to",
        ),
        (&["--dir", "a", "--ledger"][..], "needs --apply"),
        (
            &["--dir", "a", "--shout"][..],
            "not an argument of basis-runs",
        ),
    ] {
        let (code, text) = basis_runs(Path::new("/nonexistent"), arguments);
        assert_eq!(code, MISUSED, "{arguments:?}: {text}");
        assert!(
            text.contains(said) && text.contains(USAGE),
            "{arguments:?}: {text}"
        );
    }
    assert_eq!(
        ask(&[
            "--dir", "d", "--from", "2025-01", "--to", "2025-02", "--apply", "--ledger"
        ])
        .unwrap(),
        Ask {
            dir: PathBuf::from("d"),
            from: Some(YearMonth::new(2025, 1).unwrap()),
            to: Some(YearMonth::new(2025, 2).unwrap()),
            write: true,
            ledger: true,
        }
    );
}

// ─────────────────────────────── end to end ───────────────────────────────

#[test]
fn a_two_for_one_split_is_ledgered_from_the_vendor_ratio_step() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    split_store(&store.0);
    reconcile(&store.0, &out.0, "2025-03");
    let (code, text) = basis_runs(&store.0, &["--dir", &dir_text(&out), "--apply", "--ledger"]);
    assert_eq!(code, OK, "{text}");
    // ONE STEP, factor 2, from the vendor ratio 1/2 to 1/1.
    assert!(
        text.contains("steps 1, lasting 1 (a ledger row each), basis anomalies 0 (reversed 0, short 0, within_wobble 0) on 0 share(s), within_noise 0\n"),
        "{text}"
    );
    assert!(
        text.contains("step SPLITCO (2025-03-10, 2025-03-11]: ratio 1/2 to 1/1, factor 2/1"),
        "{text}"
    );
    // The share with no common minutes has no run, and is named.
    assert!(text.contains("no_run IDLECO"), "{text}");
    // ONE LEDGER ROW, factor 2, tagged measured: it labels and never excuses.
    let Slot::Store { table, receipt, .. } =
        load_one(&store.0, Table::CorporateActions, CorporateActions::build)
    else {
        panic!("the ledger loads: {text}");
    };
    assert_eq!(receipt.standing, Standing::Measured);
    let rows = table.of("SPLITCO");
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0].ratio.to_string(), "2/1");
    assert_eq!(
        (rows[0].tag, rows[0].kind),
        (Tag::Measured, Kind::Unclassed)
    );
    assert_eq!(rows[0].ex_date, day(2025, 3, 11));
    assert!(!rows[0].excuses() && rows[0].class().is_none());
    assert_eq!(
        table.excuse("SPLITCO", Some(day(2025, 3, 10)), day(2025, 3, 11)),
        None
    );
    // Every measured step now has its row.
    assert!(
        text.contains("check against corporate_actions (measured, 1 rows): 0 finding(s)"),
        "{text}"
    );
}

#[test]
fn a_measured_basis_run_ends_on_the_day_the_ratio_steps() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    split_store(&store.0);
    reconcile(&store.0, &out.0, "2025-03");
    let (code, text) = basis_runs(&store.0, &["--dir", &dir_text(&out), "--apply"]);
    assert_eq!(code, OK, "{text}");
    let runs = load_one(
        &store.0,
        Table::BasisRuns,
        pull::corporate_actions::BasisRuns::build,
    );
    let runs = runs.table().unwrap();
    let share = runs.of("SPLITCO");
    assert_eq!(share.len(), 2, "{share:#?}");
    assert_eq!(share[0].ratio.to_string(), "1/2");
    assert_eq!(share[0].end.word(), "step");
    assert_eq!(
        share[1].first,
        day(2025, 3, 11),
        "the new run starts on the ex-date"
    );
    assert_eq!(
        share[0].last,
        day(2025, 3, 10),
        "the old one ends the session before"
    );
    assert_eq!(share[1].ratio.to_string(), "1/1");
    // The receipt names its source as measured and hashes the files read.
    let Slot::Store { receipt, .. } = load_one(
        &store.0,
        Table::BasisRuns,
        pull::corporate_actions::BasisRuns::build,
    ) else {
        panic!("stored");
    };
    assert!(
        receipt.source.starts_with("measured, not sourced"),
        "{}",
        receipt.source
    );
    assert!(receipt.source_sha256.is_some());
    // No ledger without --ledger: the checker names the step with no row.
    assert!(
        text.contains("not served") || text.contains("no_row SPLITCO"),
        "{text}"
    );
    // A second run over the same files lands nothing new (append-only, idempotent).
    let (code, again) = basis_runs(&store.0, &["--dir", &dir_text(&out), "--apply"]);
    assert_eq!(code, OK, "{again}");
    assert!(
        again.contains("generation 1 already held these bytes"),
        "{again}"
    );
}

#[test]
fn a_dry_run_writes_nothing_and_checks_against_the_stores_table() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    split_store(&store.0);
    reconcile(&store.0, &out.0, "2025-03");
    // An NSE table without SPLITCO's split: the checker flags the step.
    let draft = Draft {
        table: Table::CorporateActions,
        source: "https://invented.example/ca.csv".to_owned(),
        source_sha256: Some("b".repeat(64)),
        fetched: day(2026, 10, 22),
        standing: Standing::Sourced,
        decision: "D-6892".to_owned(),
        rows: vec![vec![
            "OTHERCO".into(),
            "-".into(),
            "2025-03-06".into(),
            "split".into(),
            "2".into(),
            "1".into(),
            "nse".into(),
            "-".into(),
            "invented circular".into(),
        ]],
    };
    write(&store.0, &draft).unwrap();
    let (code, text) = basis_runs(&store.0, &["--dir", &dir_text(&out)]);
    assert_eq!(code, OK, "{text}");
    assert!(
        text.contains("check against corporate_actions (sourced, 1 rows): 1 finding(s)"),
        "{text}"
    );
    assert!(
        text.contains("no_row SPLITCO (2025-03-10, 2025-03-11]"),
        "{text}"
    );
    assert!(text.contains("dry run: nothing written"), "{text}");
    assert!(!Table::BasisRuns.dir(&store.0).exists(), "nothing written");
    // The measured ledger never replaces the NSE file.
    let (code, text) = basis_runs(&store.0, &["--dir", &dir_text(&out), "--apply", "--ledger"]);
    assert_eq!(code, FAILED, "{text}");
    assert!(
        text.contains("LEDGER REFUSED") && text.contains("is sourced"),
        "{text}"
    );
    let table = load_one(&store.0, Table::CorporateActions, CorporateActions::build);
    assert_eq!(
        table.receipt().unwrap().generation,
        1,
        "still the NSE generation"
    );
}

#[test]
fn a_missing_month_is_named_and_a_damaged_file_refuses_the_run() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let mut fixture = Fixture::default();
    for d in [3, 4] {
        fixture.day("SPLITCO", day(2025, 3, d), (1, 2), false);
    }
    for d in [5, 6] {
        fixture.day("SPLITCO", day(2025, 5, d), (1, 2), false);
    }
    fixture.write(&store.0);
    reconcile(&store.0, &out.0, "2025-03");
    reconcile(&store.0, &out.0, "2025-05");
    assert_eq!(
        months_in(&out.0).unwrap(),
        [
            YearMonth::new(2025, 3).unwrap(),
            YearMonth::new(2025, 5).unwrap()
        ]
    );
    let (code, text) = basis_runs(&store.0, &["--dir", &dir_text(&out)]);
    assert_eq!(code, OK, "{text}");
    assert!(
        text.contains("months read 2 (2025-03..2025-05), missing 1: 2025-04"),
        "{text}"
    );
    // A range with no file in it is refused by name.
    let (code, text) = basis_runs(&store.0, &["--dir", &dir_text(&out), "--from", "2025-06"]);
    assert_eq!(code, FAILED, "{text}");
    assert!(text.contains("holds no full feed-reconcile file"), "{text}");
    // A damaged version file (cut in half) refuses the whole run, by name.
    let file = out.0.join("feed-reconcile-2025-05.v1.csv");
    let whole = fs::read(&file).unwrap();
    fs::write(&file, &whole[..whole.len() / 2]).unwrap();
    let (code, text) = basis_runs(&store.0, &["--dir", &dir_text(&out), "--apply"]);
    assert_eq!(code, FAILED, "{text}");
    assert!(
        text.contains("BASIS RUNS REFUSED") && text.contains("Nothing was written"),
        "{text}"
    );
    assert!(!Table::BasisRuns.dir(&store.0).exists());
    // An empty directory is refused, and a missing one.
    let empty = Dir::new("empty");
    assert_eq!(
        basis_runs(&store.0, &["--dir", &dir_text(&empty)]).0,
        FAILED
    );
    assert_eq!(
        basis_runs(&store.0, &["--dir", "/nonexistent/brutex-basis"]).0,
        FAILED
    );
}

#[test]
fn a_ratio_that_alternates_is_named_as_basis_anomalies_and_never_ledgered() {
    // WOBBLECO: Zerodha reads 1/2 of GDFL on one session and 51/100 on the
    // next, for two weeks. Every change is a proven step; none lasts.
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let mut fixture = Fixture::default();
    for (n, d) in [3, 4, 5, 6, 7, 10, 11, 12, 13, 14].into_iter().enumerate() {
        let ratio = if n % 2 == 0 { (1, 2) } else { (51, 100) };
        fixture.day("WOBBLECO", day(2025, 3, d), ratio, false);
    }
    fixture.write(&store.0);
    reconcile(&store.0, &out.0, "2025-03");
    let (code, text) = basis_runs(&store.0, &["--dir", &dir_text(&out), "--apply", "--ledger"]);
    assert_eq!(code, OK, "{text}");
    assert!(
        text.contains("basis_anomaly WOBBLECO (") && text.contains("basis_reversed"),
        "{text}"
    );
    assert!(text.contains(", lasting 0 (a ledger row each)"), "{text}");
    assert!(!text.contains("basis anomalies 0 "), "{text}");
    assert!(text.contains("on 1 share(s)"), "{text}");
    // NO ROW: the measured ledger holds nothing of the share.
    let Slot::Store { table, .. } =
        load_one(&store.0, Table::CorporateActions, CorporateActions::build)
    else {
        panic!("the ledger loads: {text}");
    };
    assert!(
        table.of("WOBBLECO").is_empty(),
        "{:#?}",
        table.of("WOBBLECO")
    );
    assert_eq!(table.rows(), 0);
    // The checker names each anomaly; none is a missing row.
    assert!(text.contains("  basis_anomaly WOBBLECO"), "{text}");
    assert!(!text.contains("no_row WOBBLECO"), "{text}");
}

#[test]
fn a_step_across_a_missing_month_says_its_day_is_unknown_and_a_rerun_lands_nothing() {
    // GAPCO: 1/4 in March, April missing from DIR, 1/2 in May. The step is
    // lasting and ledgered; its window spans April, so its day is unknown.
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let mut fixture = Fixture::default();
    for d in [3, 4, 5, 6, 7, 10] {
        fixture.day("GAPCO", day(2025, 3, d), (1, 4), false);
    }
    for d in [5, 6, 7, 8, 9, 12] {
        fixture.day("GAPCO", day(2025, 5, d), (1, 2), false);
    }
    fixture.write(&store.0);
    reconcile(&store.0, &out.0, "2025-03");
    reconcile(&store.0, &out.0, "2025-05");
    let (code, text) = basis_runs(&store.0, &["--dir", &dir_text(&out), "--apply", "--ledger"]);
    assert_eq!(code, OK, "{text}");
    assert!(text.contains("missing 1: 2025-04"), "{text}");
    let line = text
        .lines()
        .find(|line| line.starts_with("step GAPCO ("))
        .unwrap_or_else(|| panic!("{text}"));
    assert!(
        line.contains("factor 2/1")
            && line.contains("ex-date UNKNOWN inside the window")
            && line.contains("spans month(s) missing from DIR: 2025-04"),
        "{line}"
    );
    let Slot::Store { table, .. } =
        load_one(&store.0, Table::CorporateActions, CorporateActions::build)
    else {
        panic!("the ledger loads: {text}");
    };
    let rows = table.of("GAPCO");
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert!(!rows[0].day_known(), "{rows:#?}");
    assert!(rows[0].after.is_some_and(|after| after < day(2025, 4, 1)));
    // A RERUN ON ANOTHER DAY over the same files renders the same bytes and
    // lands nothing: the receipt keeps the day its source was first read.
    let root = store.0.clone();
    let mut again = String::new();
    let code = command_with(
        &["--dir", &dir_text(&out), "--apply", "--ledger"],
        &move || Ok(root.clone()),
        Some(day(2026, 11, 5)),
        &mut again,
    );
    assert_eq!(code, OK, "{again}");
    assert!(
        again.contains("basis_runs generation 1 already held these bytes"),
        "{again}"
    );
    assert!(
        again.contains("corporate_actions generation 1 already held these bytes"),
        "{again}"
    );
}
