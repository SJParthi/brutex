//! `cli feed-reconcile` driven through `command_with` on stores this file
//! builds with the store's own writer. Every price is invented (a level near
//! ₹4,321, scaled by exact factors), every symbol but `M&M` and `GVT&D` is
//! invented, and no vendor row is copied.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use pull::session::{Day, IST_OFFSET_SECS};
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

use super::{
    Basis, COLUMN_COUNT, COLUMNS, Carry, FLAG_NAMES, FORMAT, FactorIndex, FactorPair,
    FactorRefusal, Frac, Grid, LAG_BINS, MAX_REPEAT, PairAt, Report, Running, Scope, Scratch,
    USAGE, Window, ZMinute, ask, bin, bounds, carried, carry_line, command_with, day_kind,
    file_digest, flag_text, lag_order, lookup_ask, mode, newest, newest_of, parse_carry, parse_day,
    parse_frac, parse_month, parts, percent, previous, publish, publish_through, range_bounds,
    rank, round_half_up, schedule, symbol_id, tick_root,
};
use crate::{FAILED, MISUSED, OK};

static NEXT: AtomicU64 = AtomicU64::new(0);
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const BASE: i64 = 432_100;

/// A fresh directory of this test's own under the system temp dir.
struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "brutex-cli-feed-reconcile-{tag}-{}-{}",
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
        // process's id (gate 23 clause C: no temporary path without it).
        let under_temp = self.0.parent() == Some(std::env::temp_dir().as_path());
        let pid = format!("-{}-", std::process::id());
        let own = self.0.file_name().is_some_and(|name| {
            let name = name.to_string_lossy();
            name.starts_with("brutex-cli-feed-reconcile-") && name.contains(pid.as_str())
        });
        if under_temp && own {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).unwrap()
}

fn epoch(year: u16, month: u8, day: u8) -> u32 {
    Day::new(year, month, day).unwrap().days_from_epoch()
}

/// UTC microseconds of an IST wall time.
fn at(year: u16, month: u8, day: u8, minute_of_day: i64, second: i64) -> i64 {
    let days = i64::from(epoch(year, month, day));
    (days * 86_400 - IST_OFFSET_SECS + minute_of_day * 60 + second) * 1_000_000
}

const fn hm(hour: i64, minute: i64) -> i64 {
    hour * 60 + minute
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

/// One month file of a fixture: symbol, vendor, rung seconds, (year, month).
type FileKey = (String, &'static str, u32, (u16, u8));

/// Bars to write, per file, kept in stamp order.
#[derive(Default)]
struct Fixture {
    files: BTreeMap<FileKey, Vec<Bar>>,
}

#[derive(Clone, Copy)]
struct Minutes {
    year: u16,
    month: u8,
    day: u8,
    from: i64,
    count: i64,
}

const fn on(year: u16, month: u8, day: u8, from: i64, count: i64) -> Minutes {
    Minutes {
        year,
        month,
        day,
        from,
        count,
    }
}

fn vendor_of(name: &str) -> Vendor {
    if name == "gdfl" {
        Vendor::Gdfl
    } else {
        Vendor::Zerodha
    }
}

/// A price path that never repeats between neighbours: 100 paisa times a
/// value in −11..=11, consecutive values 7 apart modulo 23.
fn wander(i: i64) -> i64 {
    BASE + 100 * ((i * 7_919).rem_euclid(23) - 11)
}

impl Fixture {
    fn push(
        &mut self,
        vendor: &'static str,
        rung: Timeframe,
        symbol: &str,
        month: (u16, u8),
        bars: Vec<Bar>,
    ) -> &mut Self {
        let file = self
            .files
            .entry((symbol.to_owned(), vendor, rung.secs(), month))
            .or_default();
        file.extend(bars);
        file.sort_by_key(|bar| bar.ts_micros);
        self
    }

    /// GDFL seconds at :00 and :30 of each minute; the minute folds to
    /// open p, high p+6, low p-2, close p+4. `p` is even, so the grid is 2.
    fn gdfl(&mut self, symbol: &str, span: Minutes, volume: i64) -> &mut Self {
        let mut bars = Vec::new();
        for n in 0..span.count {
            let minute = span.from + n;
            let p = BASE + 2 * n;
            let t = |s| at(span.year, span.month, span.day, minute, s);
            bars.push(bar(t(0), p, p + 4, p - 2, p + 2, volume));
            bars.push(bar(t(30), p + 2, p + 6, p, p + 4, volume));
        }
        self.push(
            "gdfl",
            Timeframe::SECOND_1,
            symbol,
            (span.year, span.month),
            bars,
        )
    }

    /// Zerodha minutes equal to the GDFL fold times `num / den`.
    fn minutes(
        &mut self,
        symbol: &str,
        span: Minutes,
        (num, den): (i64, i64),
        volume: i64,
    ) -> &mut Self {
        let bars = (0..span.count)
            .map(|n| {
                let p = BASE + 2 * n;
                let k = |x: i64| x * num / den;
                bar(
                    at(span.year, span.month, span.day, span.from + n, 0),
                    k(p),
                    k(p + 6),
                    k(p - 2),
                    k(p + 4),
                    volume,
                )
            })
            .collect();
        self.push(
            "zerodha",
            Timeframe::MINUTE_1,
            symbol,
            (span.year, span.month),
            bars,
        )
    }

    /// GDFL seconds every 10 s on the [`wander`] path, and Zerodha minutes
    /// whose close is the GDFL close at second 30: Zerodha's clock is 20 to
    /// 29 seconds ahead.
    fn shifted(&mut self, symbol: &str, span: Minutes) -> &mut Self {
        let (mut seconds, mut minutes) = (Vec::new(), Vec::new());
        for n in 0..span.count {
            let minute = span.from + n;
            let t = |s| at(span.year, span.month, span.day, minute, s);
            for k in 0..6 {
                let c = wander(6 * n + k);
                seconds.push(bar(t(10 * k), c, c, c, c, 3));
            }
            let z = wander(6 * n + 3);
            minutes.push(bar(t(0), z - 1, z, z - 1, z, 5));
        }
        let month = (span.year, span.month);
        self.push("gdfl", Timeframe::SECOND_1, symbol, month, seconds)
            .push("zerodha", Timeframe::MINUTE_1, symbol, month, minutes)
    }

    fn day_bar(
        &mut self,
        symbol: &str,
        (year, month, day): (u16, u8, u8),
        volume: i64,
    ) -> &mut Self {
        let ts = at(year, month, day, 0, 0);
        self.push(
            "zerodha",
            Timeframe::DAY_1,
            symbol,
            (year, month),
            vec![bar(ts, BASE, BASE + 100, BASE - 100, BASE + 10, volume)],
        )
    }

    fn write(&self, root: &Path) {
        for ((symbol, vendor, secs, (year, month)), bars) in &self.files {
            let rung = Timeframe::from_secs(*secs).unwrap();
            let path =
                StorePath::new(parts(vendor_of(vendor), symbol, rung, ym(*year, *month))).unwrap();
            let mut file = BarFile::open_or_create(root, path, symbol_id(symbol)).unwrap();
            file.append(bars).unwrap();
        }
    }
}

fn run_with(store: &Path, arguments: &[&str]) -> (u8, String) {
    let root = store.to_path_buf();
    let mut text = String::new();
    let code = command_with(
        arguments,
        &move || Ok(root.clone()),
        None,
        Some(COMMIT),
        &mut text,
    );
    (code, text)
}

/// One run of the verb over `store` for `month` into `out`.
fn reconcile_month(store: &Path, out: &Path, month: &str, extra: &[&str]) -> (u8, String) {
    let out_text = out.display().to_string();
    let mut arguments = vec![month, "--out", out_text.as_str()];
    arguments.extend_from_slice(extra);
    run_with(store, &arguments)
}

fn reconcile(store: &Path, out: &Path, extra: &[&str]) -> (u8, String) {
    reconcile_month(store, out, "2025-03", extra)
}

fn file(out: &Path, month: &str, version: u32) -> String {
    fs::read_to_string(out.join(format!("feed-reconcile-{month}.v{version}.csv"))).unwrap()
}

/// Every data row of a file, as a map from column name to field.
fn rows(text: &str) -> Vec<BTreeMap<&'static str, String>> {
    let names: Vec<&'static str> = COLUMNS.split(',').collect();
    text.lines()
        .filter(|line| !line.starts_with('#') && *line != COLUMNS)
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(
                fields.len(),
                names.len(),
                "every row has every column: {line}"
            );
            names
                .iter()
                .zip(fields)
                .map(|(name, field)| (*name, field.to_owned()))
                .collect()
        })
        .collect()
}

fn row<'a>(
    rows: &'a [BTreeMap<&'static str, String>],
    symbol: &str,
    day: &str,
) -> &'a BTreeMap<&'static str, String> {
    rows.iter()
        .find(|row| row["symbol"] == symbol && row["day"] == day)
        .unwrap_or_else(|| panic!("no row for {symbol} {day}"))
}

fn has_flag(row: &BTreeMap<&'static str, String>, flag: &str) -> bool {
    row["flags"].split('|').any(|f| f == flag)
}

/// Overwrites the WHOLE header region of a month file (both commit slots),
/// so the file is there and no slot of it decodes.
fn damage(path: &Path) {
    let mut bytes = fs::read(path).unwrap();
    let header = usize::try_from(store::format::HEADER_LEN).unwrap();
    assert!(bytes.len() >= header, "a written month holds its header");
    bytes.iter_mut().take(header).for_each(|b| *b = 0xA5);
    fs::write(path, bytes).unwrap();
}

/// A store with one earlier day of a share, so the next day has a lag.
fn seeded<'a>(fixture: &'a mut Fixture, symbol: &str) -> &'a mut Fixture {
    let seed = on(2025, 3, 3, hm(9, 15), 4);
    fixture
        .gdfl(symbol, seed, 10)
        .minutes(symbol, seed, (1, 1), 20)
        .day_bar(symbol, (2025, 3, 3), 80)
}

// ---------------------------------------------------------------- arguments

#[test]
fn a_month_and_a_day_are_written_one_way() {
    assert_eq!(parse_month("2025-03").unwrap(), ym(2025, 3));
    for bad in [
        "2025-3", "2025/03", "202503", "2025-13", "2025-00", "abcd-ef", "", "2025-031", "+025-03",
    ] {
        assert!(parse_month(bad).is_err(), "`{bad}` must be refused");
    }
    assert_eq!(
        parse_day("2025-03-04").unwrap(),
        Day::new(2025, 3, 4).unwrap()
    );
    for bad in [
        "2025-3-04",
        "2025-03-32",
        "2025-02-30",
        "2025/03/04",
        "",
        "2025-03-0x",
    ] {
        assert!(parse_day(bad).is_err(), "`{bad}` must be refused");
    }
}

#[test]
fn every_misuse_is_refused_by_name_with_the_usage() {
    let store = Dir::new("misuse");
    let out = store.0.display().to_string();
    let cases: [(&[&str], &str); 15] = [
        (&[], "name a month"),
        (&["2025-03"], "--out DIR is required"),
        (&["2025-3", "--out", &out], "write it YYYY-MM"),
        (&["2025-03", "--out"], "--out takes a value"),
        (
            &["2025-03", "--out", &out, "--sideways", "1"],
            "`--sideways` is not a flag",
        ),
        (
            &["2025-03", "--out", &out, "--out", &out],
            "--out is named twice",
        ),
        (
            &["2025-03", "--out", &out, "--workers", "0"],
            "a whole number of at least 1",
        ),
        (
            &["2025-03", "--out", &out, "--workers", "many"],
            "a whole number",
        ),
        (
            &["2025-03", "--out", &out, "--symbols", "AB,,CD"],
            "--symbols ``",
        ),
        (
            &["2025-03", "--out", &out, "--symbols", "lower"],
            "--symbols `lower`: write the stored key `LOWER`",
        ),
        (&["factor"], "factor takes SYMBOL YYYY-MM-DD"),
        (&["factor", "SYNA", "2025-03-04"], "--dir DIR is required"),
        (
            &["factor", "syna", "2025-03-04", "--dir", &out],
            "write the stored key",
        ),
        (
            &["factor", "SYNA", "2025-3-4", "--dir", &out],
            "write it YYYY-MM-DD",
        ),
        (
            &[
                "factor",
                "SYNA",
                "2025-03-04",
                "--dir",
                &out,
                "--repeat",
                "0",
            ],
            "a whole number of at least 1",
        ),
    ];
    for (arguments, says) in cases {
        let (code, text) = run_with(&store.0, arguments);
        assert_eq!(code, MISUSED, "{arguments:?}: {text}");
        assert!(
            text.contains(says),
            "{arguments:?} must say `{says}`, said: {text}"
        );
        assert!(text.contains(USAGE), "{arguments:?} prints the usage");
    }
}

#[test]
fn the_arguments_parse_to_what_they_name() {
    let asked = ask(&[
        "2025-03",
        "--symbols",
        "ZZ,M&M,ZZ",
        "--out",
        "/x",
        "--workers",
        "3",
    ])
    .unwrap();
    assert_eq!(asked.month, ym(2025, 3));
    assert_eq!(
        asked.symbols,
        Some(vec!["M&M".to_owned(), "ZZ".to_owned()]),
        "sorted, once each"
    );
    assert_eq!(asked.out, PathBuf::from("/x"));
    assert_eq!(asked.workers, Some(3));
    let generated = ask(&["2025-03", "--out", "/x"]).unwrap();
    assert_eq!(generated.workers, None, "no worker count is typed");
    assert_eq!(generated.symbols, None);
    let lookup = lookup_ask(&["M&M", "2025-03-04", "--dir", "/d", "--repeat", "9"]).unwrap();
    assert_eq!(
        (lookup.symbol.as_str(), lookup.day, lookup.repeat),
        ("M&M", Day::new(2025, 3, 4).unwrap(), 9)
    );
    assert_eq!(lookup.dir, PathBuf::from("/d"));
    // --repeat IS BOUNDED by its timing buffer: the largest count is taken,
    // one more and usize::MAX are refused by name (usize::MAX used to abort
    // the process with a capacity overflow).
    let most = MAX_REPEAT.to_string();
    let taken = lookup_ask(&["M&M", "2025-03-04", "--dir", "/d", "--repeat", &most]).unwrap();
    assert_eq!(taken.repeat, MAX_REPEAT);
    for over in [(MAX_REPEAT + 1).to_string(), usize::MAX.to_string()] {
        let refused = lookup_ask(&["M&M", "2025-03-04", "--dir", "/d", "--repeat", &over])
            .expect_err("a count above the bound is refused");
        assert!(
            refused.contains(&format!("at most {MAX_REPEAT} lookups")),
            "{refused}"
        );
    }
}

#[test]
fn a_missing_out_directory_or_store_refuses_before_anything_is_read() {
    let store = Dir::new("refusals");
    let gone = store.0.join("not-here");
    let gone_text = gone.display().to_string();
    let (code, text) = run_with(&store.0, &["2025-03", "--out", &gone_text]);
    assert_eq!(code, FAILED);
    assert!(
        text.contains("is not an existing directory; it was not created"),
        "{text}"
    );
    assert!(!gone.exists(), "the directory is never created");
    let out = store.0.display().to_string();
    let mut said = String::new();
    let code = command_with(
        &["2025-03", "--out", &out],
        &|| Err("no store here".to_owned()),
        None,
        Some(COMMIT),
        &mut said,
    );
    assert_eq!(code, FAILED);
    assert!(
        said.contains("FEED RECONCILE REFUSED: no store here. Nothing was read."),
        "{said}"
    );
}

/// `--out` MAY SIT IN THE STORE ONLY UNDER ITS AUDIT AREA, and a store root
/// that is not a directory is refused rather than read as a store of no
/// share (which would publish a file of no row).
#[test]
fn the_out_directory_may_sit_in_the_store_only_under_its_audit_area() {
    let store = Dir::new("store");
    let span = on(2025, 3, 3, hm(9, 15), 2);
    Fixture::default()
        .gdfl("SYNA", span, 10)
        .minutes("SYNA", span, (1, 1), 20)
        .write(&store.0);
    let bars = store.0.join("bars");
    assert!(bars.is_dir(), "the writer made the bars tree");
    for inside in [store.0.clone(), bars.clone(), bars.join("..").join("bars")] {
        let before = contents(&inside);
        let (code, text) = reconcile(&store.0, &inside, &[]);
        assert_eq!(code, FAILED, "{}: {text}", inside.display());
        assert!(
            text.contains("outside its audit area `audit/`"),
            "{}: {text}",
            inside.display()
        );
        assert_eq!(contents(&inside), before, "nothing written");
    }
    let audit = store.0.join("audit").join("feed-reconcile");
    fs::create_dir_all(&audit).unwrap();
    let (code, text) = reconcile(&store.0, &audit, &[]);
    assert_eq!(code, OK, "{text}");
    assert!(audit.join("feed-reconcile-2025-03.v1.csv").exists());
    let out = Dir::new("out");
    let absent = store.0.join("no-store-here");
    let (code, text) = reconcile(&absent, &out.0, &[]);
    assert_eq!(code, FAILED, "{text}");
    assert!(
        text.contains("FEED RECONCILE REFUSED: the store `"),
        "{text}"
    );
    assert!(
        contents(&out.0).0.is_empty(),
        "no file of no row is published"
    );
}

/// AN `--out` INSIDE THE TICK STORE IS REFUSED BY NAME (DATA-1 verification
/// round 1, fifth pass: "not tried"). The root `BRUTEX_GDFL_TICKSTORE` names,
/// read once by `command`, is compared on canonical paths like the store's; a
/// root that does not exist holds nothing an existing `--out` can be inside,
/// one that cannot be resolved for another reason refuses, and an unset or
/// empty variable names no root.
#[test]
fn an_out_inside_the_tick_store_is_refused_by_name() {
    let (store, ticks, out) = (Dir::new("store"), Dir::new("ticks"), Dir::new("out"));
    let span = on(2025, 3, 3, hm(9, 15), 2);
    Fixture::default()
        .gdfl("SYNA", span, 10)
        .minutes("SYNA", span, (1, 1), 20)
        .write(&store.0);
    let day = ticks.0.join("cm").join("2025-03-03");
    fs::create_dir_all(&day).unwrap();
    let run = |tick: Option<&Path>, into: &Path| {
        let (root, into) = (store.0.clone(), into.display().to_string());
        let mut text = String::new();
        let code = command_with(
            &["2025-03", "--out", &into],
            &move || Ok(root.clone()),
            tick,
            Some(COMMIT),
            &mut text,
        );
        (code, text)
    };
    for inside in [ticks.0.clone(), day.clone(), day.join("..").join("..")] {
        let before = contents(&inside);
        let (code, text) = run(Some(&ticks.0), &inside);
        assert_eq!(code, FAILED, "{}: {text}", inside.display());
        assert!(
            text.starts_with("FEED RECONCILE REFUSED: --out `")
                && text.contains("lies inside the tick store `")
                && text.contains("(BRUTEX_GDFL_TICKSTORE), which this repository only reads")
                && text.contains("Nothing was read or written."),
            "{}: {text}",
            inside.display()
        );
        assert_eq!(contents(&inside), before, "nothing written");
    }
    let plain = ticks.0.join("plain");
    fs::write(&plain, "not a directory").unwrap();
    let (code, text) = run(Some(&plain.join("under")), &out.0);
    assert_eq!(code, FAILED, "{text}");
    assert!(
        text.contains("cannot be resolved, so --out `") && text.contains("cannot be shown"),
        "{text}"
    );
    assert!(contents(&out.0).0.is_empty(), "nothing written");

    // A tick store elsewhere, one that does not exist, and none named.
    for (tick, version) in [
        (Some(ticks.0.clone()), 1),
        (Some(ticks.0.join("not-here")), 2),
        (None, 3),
    ] {
        let (code, text) = run(tick.as_deref(), &out.0);
        assert_eq!(code, OK, "{tick:?}: {text}");
        assert!(
            out.0
                .join(format!("feed-reconcile-2025-03.v{version}.csv"))
                .exists(),
            "{tick:?}"
        );
    }
    assert_eq!(tick_root(None), None);
    assert_eq!(tick_root(Some(std::ffi::OsString::new())), None, "empty");
    assert_eq!(
        tick_root(Some(std::ffi::OsString::from("/ticks"))),
        Some(PathBuf::from("/ticks"))
    );
}

// ------------------------------------------------------- statuses and pairs

#[test]
fn every_status_comes_from_its_own_shape_and_no_factor_is_invented() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let next = on(2025, 3, 4, hm(9, 15), 3);
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNM")
        .gdfl("SYNM", next, 10)
        .minutes("SYNM", next, (1, 1), 20)
        .day_bar("SYNM", (2025, 3, 4), 60)
        // GDFL only on the 5th; Zerodha only on the 6th.
        .gdfl("SYNM", on(2025, 3, 5, hm(9, 15), 2), 10)
        .minutes("SYNM", on(2025, 3, 6, hm(9, 15), 2), (1, 1), 20)
        // Both feeds on the 7th, in minutes that never meet.
        .gdfl("SYNM", on(2025, 3, 7, hm(10, 0), 2), 10)
        .minutes("SYNM", on(2025, 3, 7, hm(11, 0), 2), (1, 1), 20);
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let first = row(&all, "SYNM", "2025-03-03");
    assert_eq!(first["status"], "not_aligned", "no earlier day, no lag");
    assert_eq!(first["lag_source"], "none");
    assert_eq!(first["median_z"], "", "no factor without an applied lag");
    assert_eq!(first["best_lag"], "0", "the day's own best lag is reported");
    let matched = row(&all, "SYNM", "2025-03-04");
    assert_eq!(matched["status"], "matched");
    assert_eq!(
        (
            matched["applied_lag"].as_str(),
            matched["lag_source"].as_str()
        ),
        ("0", "own")
    );
    assert_eq!(
        (matched["median_z"].as_str(), matched["median_g"].as_str()),
        ("432106", "432106"),
        "three ratios of exactly 1: the lower median is the middle minute"
    );
    assert_eq!(matched["matched_minutes"], "3");
    assert_eq!(matched["high_match"], "3/3");
    assert_eq!(matched["grid_z"], "2");
    assert_eq!(matched["grid_g"], "2");
    assert!(!has_flag(matched, "adjusted"));
    assert_eq!(matched["ex_date"], "unchecked:no_carried_history:close");
    assert_eq!(row(&all, "SYNM", "2025-03-05")["status"], "gdfl_only");
    assert_eq!(row(&all, "SYNM", "2025-03-06")["status"], "zerodha_only");
    let apart = row(&all, "SYNM", "2025-03-07");
    assert_eq!(apart["status"], "no_overlap");
    assert_eq!(apart["median_z"], "", "never 1.0");
    for day in ["2025-03-05", "2025-03-06", "2025-03-07"] {
        assert_eq!(row(&all, "SYNM", day)["median_g"], "", "{day}");
    }
}

#[test]
fn an_adjusted_day_is_an_exact_pair_and_a_disjoint_interval_is_an_ex_date() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let one = on(2025, 3, 4, hm(9, 15), 4);
    let half = on(2025, 3, 5, hm(9, 15), 4);
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNB")
        .gdfl("SYNB", one, 10)
        .minutes("SYNB", one, (1, 1), 20)
        .gdfl("SYNB", half, 10)
        .minutes("SYNB", half, (1, 2), 20)
        .day_bar("SYNB", (2025, 3, 5), 80);
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    assert_eq!(
        row(&all, "SYNB", "2025-03-04")["ex_date"],
        "unchecked:no_carried_history:close"
    );
    let day = row(&all, "SYNB", "2025-03-05");
    assert_eq!(day["status"], "matched");
    // Four ratios of exactly 1/2: the lower median is the second by minute.
    assert_eq!(
        (day["median_z"].as_str(), day["median_g"].as_str()),
        ("216053", "432106")
    );
    assert!(has_flag(day, "adjusted"), "{}", day["flags"]);
    assert!(has_flag(day, "ex_date"), "{}", day["flags"]);
    assert_eq!(day["ex_date"], "yes:unsourced:close");
    assert_eq!(
        day["grid_z"], "1",
        "the series-month grid shrank to 1 paisa"
    );
    assert_eq!(
        row(&all, "SYNB", "2025-03-04")["grid_z"],
        "2",
        "a day's grid is the grid as of that day, never the month's end"
    );
    assert!(text.contains("ex-date days 1 (unsourced)"), "{text}");
    assert!(text.contains("adjusted shares 1"), "{text}");
}

#[test]
fn a_minute_from_another_basis_empties_the_interval_and_is_named() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNC")
        .gdfl("SYNC", on(2025, 3, 4, hm(9, 15), 4), 10)
        .minutes("SYNC", on(2025, 3, 4, hm(9, 15), 2), (1, 1), 20);
    // Minutes 3 and 4 of the 4th on half the basis.
    let half = |price: i64| price / 2;
    let bars: Vec<Bar> = (2..4)
        .map(|n| {
            let p = BASE + 2 * n;
            bar(
                at(2025, 3, 4, hm(9, 15) + n, 0),
                half(p),
                half(p + 6),
                half(p - 2),
                half(p + 4),
                20,
            )
        })
        .collect();
    fixture.push("zerodha", Timeframe::MINUTE_1, "SYNC", (2025, 3), bars);
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let day = row(&all, "SYNC", "2025-03-04");
    assert!(has_flag(day, "factor_not_constant"), "{}", day["flags"]);
    assert_eq!(day["emptied_at"], "09:17");
    assert_eq!(day["emptied_pair"], "216054/432108");
    assert_eq!(day["opposing_pair"], "432106/432106");
    assert_eq!(
        day["interval_lo"], "",
        "an empty intersection has no interval"
    );
    assert!(
        has_flag(day, "range_not_constant"),
        "half the basis cannot share a trade: {}",
        day["flags"]
    );
    assert_eq!(day["range_emptied_at"], "09:17");
    assert_eq!(day["range_lo"], "", "an empty range intersection");
    let index = FactorIndex::load(&out.0, ym(2025, 3)).unwrap();
    match index.factor_of("SYNC", Day::new(2025, 3, 4).unwrap()) {
        Err(FactorRefusal::FactorNotConstant { minute, pair }) => {
            assert_eq!(minute, "09:17");
            assert_eq!((pair.interval, pair.range), (None, None));
            assert_eq!(pair.max, (432_106, 432_106));
        }
        other => panic!("refused as not constant, got {other:?}"),
    }
}

/// THE TWO FEEDS' CLOSES ARE OFTEN DIFFERENT TRADES OF ONE MINUTE (D-5176):
/// a Zerodha close at the low end of the same range GDFL printed empties the
/// close intersection (`factor_not_constant`, the operator's rule, kept) and
/// leaves the range intersection whole, and the reader hands the consumer
/// the pairs and that verdict inside the refusal.
#[test]
fn a_different_trade_of_one_minute_empties_the_closes_and_not_the_ranges() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNK").gdfl("SYNK", on(2025, 3, 4, hm(9, 15), 4), 10);
    // The GDFL fold of minute n: high p+6, low p-2, close p+4. Zerodha
    // closes at p+4 on minutes 0 and 2 and at p on minutes 1 and 3. The
    // intervals are closed: 09:16's upper bound (2B+6)/(2B+10) EQUALS 09:15's
    // lower bound, so the intersection is that one point after 09:16 and
    // empties at 09:17, whose lower bound lies above it.
    let bars: Vec<Bar> = (0..4)
        .map(|n| {
            let p = BASE + 2 * n;
            let close = if n % 2 == 0 { p + 4 } else { p };
            bar(at(2025, 3, 4, hm(9, 15) + n, 0), p, p + 6, p - 2, close, 20)
        })
        .collect();
    fixture.push("zerodha", Timeframe::MINUTE_1, "SYNK", (2025, 3), bars);
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let day = row(&all, "SYNK", "2025-03-04");
    assert_eq!(day["status"], "matched");
    assert!(has_flag(day, "factor_not_constant"), "{}", day["flags"]);
    assert_eq!(day["emptied_at"], "09:17");
    assert!(!has_flag(day, "range_not_constant"), "{}", day["flags"]);
    assert_eq!(day["range_emptied_at"], "");
    assert!(
        !day["range_lo"].is_empty() && !day["range_hi"].is_empty(),
        "the ranges still hold one basis"
    );
    let index = FactorIndex::load(&out.0, ym(2025, 3)).unwrap();
    match index.factor_of("SYNK", Day::new(2025, 3, 4).unwrap()) {
        Err(FactorRefusal::FactorNotConstant { minute, pair }) => {
            assert_eq!(minute, "09:17");
            assert_eq!(pair.interval, None);
            let (low, high) = pair.range.expect("the range intersection held");
            let one = Frac { num: 1, den: 1 };
            assert!(low.le(one) && one.le(high), "{low} .. {high}");
            assert!(
                pair.to_string().contains("interval empty range ["),
                "{pair}"
            );
        }
        other => panic!("refused as not constant, got {other:?}"),
    }
    let dir = out.0.display().to_string();
    let (code, said) = run_with(&store.0, &["factor", "SYNK", "2025-03-04", "--dir", &dir]);
    assert_eq!(code, OK, "{said}");
    assert!(
        said.contains("refused factor_not_constant at 09:17 (median"),
        "{said}"
    );
}

/// Zerodha minutes over the GDFL fold of [`Fixture::gdfl`] times `num/den`,
/// whose close is a DIFFERENT trade of the minute on every other minute
/// (`p` instead of `p+4`), so the close intersection empties and the range
/// intersection holds.
fn other_trades(fixture: &mut Fixture, symbol: &str, span: Minutes, (num, den): (i64, i64)) {
    let k = |x: i64| x * num / den;
    let bars: Vec<Bar> = (0..span.count)
        .map(|n| {
            let p = BASE + 2 * n;
            let close = if n % 2 == 0 { p + 4 } else { p };
            bar(
                at(span.year, span.month, span.day, span.from + n, 0),
                k(p),
                k(p + 6),
                k(p - 2),
                k(close),
                20,
            )
        })
        .collect();
    fixture.push(
        "zerodha",
        Timeframe::MINUTE_1,
        symbol,
        (span.year, span.month),
        bars,
    );
}

/// THE EX-DATE IS JUDGED ON THE RANGE INTERSECTION WHERE THE CLOSES EMPTIED
/// (D-5177). On the real store the close intersection empties on almost
/// every matched day (2025-03: 3,948 of 3,952), so an ex-date judged on it
/// alone never fired. Two days on bases 1 and 1/2, each with closes from
/// different trades: both close intersections empty, both range
/// intersections hold, and they are disjoint.
#[test]
fn an_ex_date_is_judged_on_the_ranges_where_the_closes_emptied() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let (one, half) = (on(2025, 3, 4, hm(9, 15), 4), on(2025, 3, 5, hm(9, 15), 4));
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNX")
        .gdfl("SYNX", one, 10)
        .gdfl("SYNX", half, 10);
    other_trades(&mut fixture, "SYNX", one, (1, 1));
    other_trades(&mut fixture, "SYNX", half, (1, 2));
    // A third day on the same half basis: no ex-date against the second.
    let again = on(2025, 3, 6, hm(9, 15), 4);
    fixture.gdfl("SYNX", again, 10);
    other_trades(&mut fixture, "SYNX", again, (1, 2));
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let first = row(&all, "SYNX", "2025-03-04");
    assert!(has_flag(first, "factor_not_constant"), "{}", first["flags"]);
    assert!(!has_flag(first, "range_not_constant"), "{}", first["flags"]);
    assert_eq!(first["ex_date"], "unchecked:no_carried_history:range");
    let second = row(&all, "SYNX", "2025-03-05");
    assert!(
        has_flag(second, "factor_not_constant"),
        "{}",
        second["flags"]
    );
    assert!(has_flag(second, "adjusted"), "{}", second["flags"]);
    assert!(has_flag(second, "ex_date"), "{}", second["flags"]);
    assert_eq!(second["ex_date"], "yes:unsourced:range");
    let third = row(&all, "SYNX", "2025-03-06");
    assert!(!has_flag(third, "ex_date"), "{}", third["flags"]);
    assert_eq!(third["ex_date"], "no:range");
    assert!(text.contains("ex-date days 1 (unsourced)"), "{text}");
    // A day whose minutes admit no one factor in either intersection is
    // named, and hands the next day nothing.
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNC")
        .gdfl("SYNC", on(2025, 3, 4, hm(9, 15), 4), 10)
        .minutes("SYNC", on(2025, 3, 4, hm(9, 15), 2), (1, 1), 20);
    let bars: Vec<Bar> = (2..4)
        .map(|n| {
            let p = BASE + 2 * n;
            let half = |price: i64| price / 2;
            let stamp = at(2025, 3, 4, hm(9, 15) + n, 0);
            bar(stamp, half(p), half(p + 6), half(p - 2), half(p + 4), 20)
        })
        .collect();
    fixture.push("zerodha", Timeframe::MINUTE_1, "SYNC", (2025, 3), bars);
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let day = row(&all, "SYNC", "2025-03-04");
    assert!(has_flag(day, "range_not_constant"), "{}", day["flags"]);
    assert_eq!(day["ex_date"], "unchecked:no_interval");
}

/// One minute of both feeds: GDFL seconds at :00 (every price `g.0`) and :30
/// (every price `g.1`), so the window's low is `g.0`, its high and close
/// `g.1`; a Zerodha minute of low `z.0`, high `z.1` and close `z.2`.
fn both_feeds(
    fixture: &mut Fixture,
    symbol: &str,
    (year, month, day): (u16, u8, u8),
    minute: i64,
    g: (i64, i64),
    z: (i64, i64, i64),
) {
    let t = |second| at(year, month, day, minute, second);
    fixture
        .push(
            "gdfl",
            Timeframe::SECOND_1,
            symbol,
            (year, month),
            vec![
                bar(t(0), g.0, g.0, g.0, g.0, 5),
                bar(t(30), g.1, g.1, g.1, g.1, 5),
            ],
        )
        .push(
            "zerodha",
            Timeframe::MINUTE_1,
            symbol,
            (year, month),
            vec![bar(t(0), z.0, z.1, z.0, z.2, 10)],
        );
}

/// AN INTERVAL OFF ITS OWN DAY'S MEDIAN IS NO BASIS FOR AN EX-DATE (D-5179).
/// Three days on one unchanged basis, factor 1. The 4th and the 6th are
/// tight: each feed prints the same two trades, so the close intersection
/// holds the ratio 1. On the 5th the closes are different trades (so the
/// close intersection empties, as on almost every real day) and ONE minute is
/// discordant: its Zerodha range lies 90 paisa above GDFL's. That minute
/// narrows the range intersection to about [1 + 8/q, 1 + 82/q], which
/// excludes the day's own median ratio 1 and is disjoint from the 4th's
/// interval. Judged on it, the 5th and then the 6th were ex-dates on a basis
/// that never moved.
#[test]
fn an_interval_off_its_own_median_is_no_basis_for_an_ex_date() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNO");
    for n in 0..4 {
        let q = BASE + 10 * n;
        for day in [4, 6] {
            both_feeds(
                &mut fixture,
                "SYNO",
                (2025, 3, day),
                hm(9, 15) + n,
                (q, q + 2),
                (q, q + 2, q + 2),
            );
        }
        let close = if n % 2 == 0 { q + 40 } else { q - 40 };
        both_feeds(
            &mut fixture,
            "SYNO",
            (2025, 3, 5),
            hm(9, 15) + n,
            (q - 40, q + 40),
            (q - 40, q + 40, close),
        );
    }
    let q = BASE + 40;
    both_feeds(
        &mut fixture,
        "SYNO",
        (2025, 3, 5),
        hm(9, 19),
        (q - 40, q + 40),
        (q + 50, q + 130, q + 130),
    );
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let first = row(&all, "SYNO", "2025-03-04");
    assert_eq!(first["ex_date"], "unchecked:no_carried_history:close");
    let odd = row(&all, "SYNO", "2025-03-05");
    assert_eq!(odd["status"], "matched");
    assert_eq!(
        odd["median_z"], odd["median_g"],
        "the day's own median ratio is 1"
    );
    assert!(has_flag(odd, "factor_not_constant"), "{}", odd["flags"]);
    assert!(!has_flag(odd, "range_not_constant"), "{}", odd["flags"]);
    let low = parse_frac(&odd["range_lo"]).unwrap();
    assert!(
        Frac { num: 1, den: 1 }.lt(low),
        "the discordant minute moved the range intersection off 1: {low}"
    );
    assert_eq!(odd["ex_date"], "unchecked:interval_excludes_median:range");
    assert!(!has_flag(odd, "ex_date"), "{}", odd["flags"]);
    let last = row(&all, "SYNO", "2025-03-06");
    assert_eq!(
        last["ex_date"], "no:close",
        "judged against the 4th, the last day with a basis"
    );
    assert!(!has_flag(last, "ex_date"), "{}", last["flags"]);
    assert!(text.contains("ex-date days 0 (unsourced)"), "{text}");
    let carry = file(&out.0, "2025-03", 1)
        .lines()
        .find(|line| line.starts_with("# carry,SYNO,"))
        .map(str::to_owned)
        .unwrap();
    assert!(
        carry.contains("interval=2025-03-06:"),
        "the 6th hands its own interval on: {carry}"
    );
    // THE CONSUMER READS THE SAME BASIS (D-5179 item 6): the 4th and the 6th
    // have one on their closes, the 5th has none although its range
    // intersection survived, so a consumer refuses it and converts nothing.
    let index = FactorIndex::load(&out.0, ym(2025, 3)).unwrap();
    for day in [4, 6] {
        let pair = index
            .factor_of("SYNO", Day::new(2025, 3, day).unwrap())
            .unwrap();
        assert_eq!(pair.basis().map(|(_, b)| b), Some(Basis::Close), "{pair}");
    }
    let Err(FactorRefusal::FactorNotConstant { pair, .. }) =
        index.factor_of("SYNO", Day::new(2025, 3, 5).unwrap())
    else {
        panic!("the 5th's closes emptied");
    };
    assert!(pair.range.is_some(), "the range intersection survived");
    assert_eq!(pair.basis(), None, "and holds no median: no factor");
}

/// THE BASIS RULE ITSELF (D-5179): the close intersection when it holds the
/// median's ratio, else the range intersection when it does, else none; an
/// emptied close intersection is no answer by itself.
#[test]
fn a_days_basis_is_the_first_intersection_holding_its_own_median() {
    let f = |num, den| Frac { num, den };
    let pair = |interval, range| FactorPair {
        median: (100, 100),
        min: (99, 100),
        max: (101, 100),
        interval,
        range,
        lag: 0,
    };
    let holding = (f(99, 100), f(101, 100));
    let off = (f(102, 100), f(103, 100));
    assert_eq!(
        pair(Some(holding), Some(off)).basis(),
        Some((holding, Basis::Close))
    );
    assert_eq!(
        pair(None, Some(holding)).basis(),
        Some((holding, Basis::Range)),
        "an emptied close intersection falls to the ranges"
    );
    assert_eq!(
        pair(Some(off), Some(holding)).basis(),
        Some((holding, Basis::Range)),
        "a close intersection off the median is passed over"
    );
    assert_eq!(pair(Some(off), Some(off)).basis(), None);
    assert_eq!(pair(None, None).basis(), None);
    // The bounds are closed: a median on an end is held.
    let edge = (f(1, 1), f(2, 1));
    assert_eq!(pair(Some(edge), None).basis(), Some((edge, Basis::Close)));
    assert_eq!(
        (Basis::Close.name(), Basis::Range.name()),
        ("close", "range")
    );
}

// ------------------------------------------------------------------- lags

#[test]
fn the_best_lag_is_found_and_only_applied_from_the_next_day() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    Fixture::default()
        .shifted("SYNL", on(2025, 3, 3, hm(9, 15), 20))
        .shifted("SYNL", on(2025, 3, 4, hm(9, 15), 20))
        .write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let first = row(&all, "SYNL", "2025-03-03");
    assert_eq!(first["status"], "not_aligned");
    assert_eq!(
        first["best_lag"], "-20",
        "seconds 20..29 early: the smallest |L| wins"
    );
    assert_eq!(first["best_lag_share"], "20/20");
    assert_ne!(
        first["lag0_share"], "20/20",
        "lag 0 does not match every minute"
    );
    let second = row(&all, "SYNL", "2025-03-04");
    assert_eq!(
        (
            second["applied_lag"].as_str(),
            second["lag_source"].as_str()
        ),
        ("-20", "own"),
        "day D's lag comes from day D-1"
    );
    assert_eq!(second["status"], "matched");
    assert_eq!(
        second["median_z"], second["median_g"],
        "on one basis at the right lag"
    );
    assert!(
        !has_flag(second, "factor_not_constant"),
        "{}",
        second["flags"]
    );
}

#[test]
fn a_share_without_history_takes_the_pooled_lag_labelled() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let day4 = on(2025, 3, 4, hm(9, 15), 3);
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNP")
        .gdfl("SYNP", day4, 10)
        .minutes("SYNP", day4, (1, 1), 20)
        .gdfl("SYNQ", day4, 10)
        .minutes("SYNQ", day4, (1, 1), 20);
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &["--workers", "2"]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let late = row(&all, "SYNQ", "2025-03-04");
    assert_eq!(
        (late["applied_lag"].as_str(), late["lag_source"].as_str()),
        ("0", "pooled")
    );
    assert_eq!(late["status"], "matched");
    assert_eq!(row(&all, "SYNP", "2025-03-04")["lag_source"], "own");
    let body = file(&out.0, "2025-03", 1);
    assert!(
        body.contains("# pooled,lags=0:3"),
        "three best lags of 0 this month: {body}"
    );
}

#[test]
fn the_next_month_carries_the_lag_the_interval_and_the_open_span() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let feb = on(2025, 2, 28, hm(9, 15), 3);
    let mar = on(2025, 3, 3, hm(9, 15), 4);
    Fixture::default()
        .gdfl("SYND", feb, 10)
        .minutes("SYND", feb, (1, 1), 20)
        .day_bar("SYND", (2025, 2, 28), 57)
        .gdfl("SYND", mar, 10)
        .minutes("SYND", mar, (1, 2), 20)
        .day_bar("SYND", (2025, 3, 3), 76)
        .write(&store.0);
    let (code, text) = reconcile_month(&store.0, &out.0, "2025-02", &[]);
    assert_eq!(code, OK, "{text}");
    let february = file(&out.0, "2025-02", 1);
    let carry = february
        .lines()
        .find(|line| line.starts_with("# carry,SYND,"))
        .unwrap();
    assert!(carry.contains("lags=0:1"), "{carry}");
    assert!(carry.contains("exceed_open=2025-02-28:1"), "{carry}");
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let march = file(&out.0, "2025-03", 1);
    assert!(
        march.contains("previous feed-reconcile-2025-02.v1.csv"),
        "{march}"
    );
    let all = rows(&march);
    let day = row(&all, "SYND", "2025-03-03");
    assert_eq!(
        day["lag_source"], "own",
        "the lag history crossed the month"
    );
    assert_eq!(day["status"], "matched");
    assert_eq!(
        day["ex_date"], "unchecked:no_earlier_interval:close",
        "February's only day had no applied lag, so no interval"
    );
    assert!(has_flag(day, "zerodha_minute_sum_exceeds_day_bar"));
    assert!(
        march.contains("# span,zerodha_minute_sum_exceeds_day_bar,SYND,2025-02-28,2,open"),
        "the run of flagged days continues across the month: {march}"
    );
}

/// A MONTH WITHOUT A SHARE DOES NOT ERASE ITS HISTORY. SYNA trades in
/// January and March and holds no file in February (another share keeps the
/// month running): February's file carries SYNA's January lag and interval
/// unchanged, so March applies SYNA's OWN lag and judges its first day against
/// January's last interval, and a basis change across the gap is an ex-date.
#[test]
fn a_month_without_a_share_carries_its_history_unchanged() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let jan = (on(2025, 1, 2, hm(9, 15), 4), on(2025, 1, 3, hm(9, 15), 4));
    let feb = on(2025, 2, 3, hm(9, 15), 4);
    let mar = on(2025, 3, 3, hm(9, 15), 4);
    let mut fixture = Fixture::default();
    for day in [jan.0, jan.1] {
        fixture
            .gdfl("SYNA", day, 10)
            .minutes("SYNA", day, (1, 1), 20);
    }
    fixture
        .gdfl("SYNB", feb, 10)
        .minutes("SYNB", feb, (1, 1), 20)
        .gdfl("SYNA", mar, 10)
        .minutes("SYNA", mar, (1, 2), 20);
    fixture.write(&store.0);
    for month in ["2025-01", "2025-02", "2025-03"] {
        let (code, text) = reconcile_month(&store.0, &out.0, month, &[]);
        assert_eq!(code, OK, "{month}: {text}");
    }
    let january = file(&out.0, "2025-01", 1);
    let carried = january
        .lines()
        .find(|line| line.starts_with("# carry,SYNA,"))
        .unwrap();
    let february = file(&out.0, "2025-02", 1);
    assert!(
        february.lines().any(|line| line == carried),
        "February carries SYNA's January line unchanged: {february}"
    );
    assert!(
        rows(&february).iter().all(|row| row["symbol"] != "SYNA"),
        "and has no row for it"
    );
    let all = rows(&file(&out.0, "2025-03", 1));
    let day = row(&all, "SYNA", "2025-03-03");
    assert_eq!(
        (day["applied_lag"].as_str(), day["lag_source"].as_str()),
        ("0", "own"),
        "the lag history crossed the empty month"
    );
    assert_eq!(day["ex_date"], "yes:unsourced:close", "{}", day["flags"]);
}

/// Every file name in `dir`, sorted, and the summary log's text.
fn contents(dir: &Path) -> (Vec<String>, String) {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let log = fs::read_to_string(dir.join("feed-reconcile.summary.log")).unwrap_or_default();
    (names, log)
}

/// A `--SYMBOLS` RUN IS NEVER THE MONTH'S AUTHORITY (D-5178). Two
/// directories run February, March and April in full; one of them also runs
/// March for SYNA alone in between. That run is its own `.symbols.` file, so
/// the factor of the share it did not list is answered from the full March
/// exactly as before, and April's bytes are the same in both directories.
/// SYNB moves from basis 1 in February to 1/2 in March and stays there in
/// April, so an April carried from February's interval (what a partial March
/// as the newest version did) would call April 1 an ex-date.
#[test]
fn a_symbols_run_is_never_the_months_authority_and_feeds_no_later_month() {
    let store = Dir::new("store");
    let (plain, probed) = (Dir::new("out"), Dir::new("out"));
    let mut fixture = Fixture::default();
    for (span, basis) in [
        (on(2025, 2, 26, hm(9, 15), 4), (1, 1)),
        (on(2025, 2, 27, hm(9, 15), 4), (1, 1)),
        (on(2025, 3, 3, hm(9, 15), 4), (1, 2)),
        (on(2025, 4, 1, hm(9, 15), 4), (1, 2)),
    ] {
        fixture
            .gdfl("SYNB", span, 10)
            .minutes("SYNB", span, basis, 20)
            .gdfl("SYNA", span, 10)
            .minutes("SYNA", span, (1, 1), 20);
    }
    fixture.write(&store.0);
    for month in ["2025-02", "2025-03"] {
        for out in [&plain, &probed] {
            let (code, text) = reconcile_month(&store.0, &out.0, month, &[]);
            assert_eq!(code, OK, "{month}: {text}");
        }
    }
    let (code, text) = reconcile_month(&store.0, &probed.0, "2025-03", &["--symbols", "SYNA"]);
    assert_eq!(code, OK, "{text}");
    assert!(text.contains("FEED RECONCILE --symbols"), "{text}");
    let partial =
        fs::read_to_string(probed.0.join("feed-reconcile-2025-03.symbols.v1.csv")).unwrap();
    assert!(partial.contains("\n# scope symbols: "), "{partial}");
    assert!(rows(&partial).iter().all(|row| row["symbol"] == "SYNA"));
    assert!(
        !partial.contains("# carry,SYNB,"),
        "a --symbols file repeats no unlisted share's carry"
    );
    assert!(!probed.0.join("feed-reconcile-2025-03.v2.csv").exists());
    assert!(file(&plain.0, "2025-03", 1).contains("\n# scope all: "));
    let march = Day::new(2025, 3, 3).unwrap();
    let before = FactorIndex::load(&plain.0, ym(2025, 3)).unwrap();
    let after = FactorIndex::load(&probed.0, ym(2025, 3)).unwrap();
    assert!(
        before.factor_of("SYNB", march).is_ok(),
        "{:?}",
        before.factor_of("SYNB", march)
    );
    assert_eq!(
        after.factor_of("SYNB", march),
        before.factor_of("SYNB", march),
        "the unlisted share answers exactly as before"
    );
    assert_eq!(after.digest, before.digest, "the same authority");
    for out in [&plain, &probed] {
        let (code, text) = reconcile_month(&store.0, &out.0, "2025-04", &[]);
        assert_eq!(code, OK, "{text}");
    }
    let april = file(&plain.0, "2025-04", 1);
    assert_eq!(
        file(&probed.0, "2025-04", 1),
        april,
        "the next month is byte for byte the run without the --symbols run"
    );
    assert_eq!(
        row(&rows(&april), "SYNB", "2025-04-01")["ex_date"],
        "no:close"
    );
    // A KEY NEITHER FEED HOLDS (a typo) is refused by name and nothing is
    // written: no version, no summary line.
    let held = contents(&probed.0);
    let (code, text) = reconcile_month(&store.0, &probed.0, "2025-03", &["--symbols", "SYNA,SYNZ"]);
    assert_eq!(code, FAILED, "{text}");
    assert!(
        text.contains(
            "--symbols `SYNZ`: neither feed holds a share directory of that key, in 2025-03 \
             or any other month"
        ),
        "the listing is of share directories, never of one month's files: {text}"
    );
    assert!(!text.contains("`SYNA`"), "a held key is not named: {text}");
    assert_eq!(contents(&probed.0), held, "nothing was written");
}

#[test]
fn lag_order_mode_and_bins_break_ties_by_the_smallest_magnitude_then_negative() {
    let order: Vec<i8> = lag_order().take(5).collect();
    assert_eq!(order, vec![0, -1, 1, -2, 2]);
    assert_eq!(lag_order().count(), LAG_BINS);
    assert_eq!((bin(-59), bin(0), bin(59)), (0, 59, 118));
    let mut counts = [0_u32; LAG_BINS];
    assert_eq!(mode(&counts), None);
    counts[bin(3)] = 2;
    counts[bin(-3)] = 2;
    assert_eq!(mode(&counts), Some(-3));
    counts[bin(5)] = 3;
    assert_eq!(mode(&counts), Some(5));
}

// ----------------------------------------------------------------- volumes

#[test]
fn the_short_is_measured_against_what_gdfl_explains_and_left_unjudged() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let day4 = on(2025, 3, 4, hm(9, 15), 4);
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNV")
        // 4 minutes x 2 seconds x 10 inside Zerodha's minutes, 7 in the
        // pre-open and 4 x 2 x 10 in minutes Zerodha omitted.
        .gdfl("SYNV", day4, 10)
        .gdfl("SYNV", on(2025, 3, 4, hm(9, 25), 4), 10)
        .push(
            "gdfl",
            Timeframe::SECOND_1,
            "SYNV",
            (2025, 3),
            vec![bar(at(2025, 3, 4, hm(9, 5), 0), BASE, BASE, BASE, BASE, 7)],
        )
        .minutes("SYNV", day4, (1, 1), 15)
        .day_bar("SYNV", (2025, 3, 4), 200);
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let day = row(&all, "SYNV", "2025-03-04");
    assert_eq!(day["gdfl_day_volume"], "167");
    assert_eq!(day["zerodha_minute_volume"], "60");
    assert_eq!(
        day["explained_volume"], "87",
        "7 pre-open + 80 in omitted minutes"
    );
    assert_eq!(day["short_volume"], "53", "(200 - 60) - 87");
    assert_eq!(day["short_of_day"], "53/200");
    assert_eq!(day["short_judgement"], "unjudged:no_env1_envelope");
    assert_eq!(
        day["explained_volume_zbasis"], "",
        "an unadjusted day has one basis"
    );
    let first = row(&all, "SYNV", "2025-03-03");
    assert_eq!(first["short_judgement"], "not_checked:not_aligned");
}

#[test]
fn the_minute_sum_may_exceed_the_day_bar_only_by_the_rounding_allowance() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNE");
    for (day, num_den, day_volume) in [
        (4, (1, 1), 80),
        (5, (1, 1), 79),
        (6, (1, 2), 77),
        (7, (1, 2), 76),
    ] {
        let span = on(2025, 3, day, hm(9, 15), 4);
        fixture
            .gdfl("SYNE", span, 10)
            .minutes("SYNE", span, num_den, 20)
            .day_bar("SYNE", (2025, 3, day), day_volume);
    }
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let equal = row(&all, "SYNE", "2025-03-04");
    assert_eq!(equal["minute_sum_allowance"], "0:unadjusted");
    assert!(!has_flag(equal, "zerodha_minute_sum_exceeds_day_bar"));
    let over = row(&all, "SYNE", "2025-03-05");
    assert!(
        has_flag(over, "zerodha_minute_sum_exceeds_day_bar"),
        "one share over is over"
    );
    // Adjusted: allowance ceil((4 + 1) / 2) = 3.
    let within = row(&all, "SYNE", "2025-03-06");
    assert_eq!(within["minute_sum_allowance"], "3:adjusted");
    assert!(
        !has_flag(within, "zerodha_minute_sum_exceeds_day_bar"),
        "80 <= 77 + 3"
    );
    assert!(has_flag(
        row(&all, "SYNE", "2025-03-07"),
        "zerodha_minute_sum_exceeds_day_bar"
    ));
    let unknown = row(&all, "SYNE", "2025-03-03");
    assert_eq!(
        unknown["minute_sum_allowance"], "3:unknown",
        "no factor: the wider room"
    );
    let body = file(&out.0, "2025-03", 1);
    assert!(
        body.contains("# span,zerodha_minute_sum_exceeds_day_bar,SYNE,2025-03-05,1\n"),
        "{body}"
    );
    assert!(
        body.contains("# span,zerodha_minute_sum_exceeds_day_bar,SYNE,2025-03-07,1,open"),
        "{body}"
    );
}

#[test]
fn a_session_cut_is_a_continuous_minute_gdfl_traded_after_zerodha_stopped() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNS")
        .gdfl("SYNS", on(2025, 3, 4, hm(13, 58), 4), 10)
        .minutes("SYNS", on(2025, 3, 4, hm(13, 58), 2), (1, 1), 20);
    // From 2026-08-03, 15:15-15:29 is the closing auction of an eligible
    // share: with eligibility not read it is Unverified, never a gap.
    let seed = on(2026, 8, 3, hm(15, 13), 2);
    let aug = on(2026, 8, 4, hm(15, 13), 2);
    fixture
        .gdfl("SYNT", seed, 10)
        .minutes("SYNT", seed, (1, 1), 20)
        .gdfl("SYNT", aug, 10)
        .minutes("SYNT", aug, (1, 1), 20)
        .gdfl("SYNT", on(2026, 8, 4, hm(15, 20), 1), 10);
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let cut = row(&all, "SYNS", "2025-03-04");
    assert!(has_flag(cut, "zerodha_session_cut"), "{}", cut["flags"]);
    assert_eq!(cut["session_cut_minute"], "14:00");
    assert!(file(&out.0, "2025-03", 1).contains("# onset,zerodha_session_cut,SYNS,2025-03-04"));
    let (code, text) = reconcile_month(&store.0, &out.0, "2026-08", &[]);
    assert_eq!(code, OK, "{text}");
    let august = rows(&file(&out.0, "2026-08", 1));
    let auction = row(&august, "SYNT", "2026-08-04");
    assert!(
        auction["day_kind"].starts_with("open"),
        "{}",
        auction["day_kind"]
    );
    assert_eq!(auction["status"], "matched");
    assert!(
        !has_flag(auction, "zerodha_session_cut"),
        "{}",
        auction["flags"]
    );
    assert_eq!(auction["gdfl_unverified_phase_volume"], "20");
}

// ------------------------------------------------------------ the read door

#[test]
fn a_month_that_cannot_be_read_is_read_failed_never_absent() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let span = on(2025, 3, 3, hm(9, 15), 4);
    Fixture::default()
        .gdfl("SYNF", span, 10)
        .minutes("SYNF", span, (1, 1), 20)
        .day_bar("SYNF", (2025, 3, 3), 80)
        .write(&store.0);
    let path = StorePath::new(parts(
        Vendor::Zerodha,
        "SYNF",
        Timeframe::MINUTE_1,
        ym(2025, 3),
    ))
    .unwrap()
    .to_path_buf(&store.0);
    damage(&path);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, FAILED, "an unread month fails the run: {text}");
    assert!(text.contains("FEED RECONCILE INCOMPLETE"), "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let failed = row(&all, "SYNF", "2025-03-03");
    assert!(
        failed["status"].starts_with("read_failed:zerodha_1min:"),
        "{}",
        failed["status"]
    );
    assert_eq!(failed["gdfl_day_volume"], "", "no half-read number");
    assert!(text.contains("1 unread"), "{text}");
    let index = FactorIndex::load(&out.0, ym(2025, 3)).unwrap();
    assert!(matches!(
        index.factor_of("SYNF", Day::new(2025, 3, 3).unwrap()),
        Err(FactorRefusal::ReadFailed(_))
    ));
}

#[test]
fn a_share_with_no_readable_day_still_has_its_failure_row() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    Fixture::default()
        .gdfl("SYNQ", on(2025, 3, 3, hm(9, 15), 2), 10)
        .write(&store.0);
    let path = StorePath::new(parts(
        Vendor::Gdfl,
        "SYNQ",
        Timeframe::SECOND_1,
        ym(2025, 3),
    ))
    .unwrap()
    .to_path_buf(&store.0);
    damage(&path);
    let segment = path.parent().unwrap().parent().unwrap().parent().unwrap();
    fs::create_dir_all(segment.join("bad.name")).unwrap();
    fs::create_dir_all(segment.join("synl")).unwrap();
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, FAILED, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    assert!(row(&all, "SYNQ", "-")["status"].starts_with("read_failed:gdfl_1s:"));
    assert!(row(&all, "bad.name", "-")["status"].starts_with("read_failed:not_a_share_key:"));
    assert_eq!(
        row(&all, "synl", "-")["status"],
        "read_failed:not_a_share_key:write the stored key `SYNL`"
    );
    assert!(text.contains("3 unread"), "{text}");
    let index = FactorIndex::load(&out.0, ym(2025, 3)).unwrap();
    assert!(
        matches!(
            index.factor_of("SYNQ", Day::new(2025, 3, 17).unwrap()),
            Err(FactorRefusal::ReadFailed(_))
        ),
        "a share-level failure answers every day"
    );
}

/// Writes `fixture` into `raw`, then places the sealed form of each of its
/// month files, built from that file's own committed header and records,
/// under `sealed` (which may be `raw` itself); the `.sbar` paths written.
fn seal_into(fixture: &Fixture, raw: &Path, sealed: &Path) -> Vec<PathBuf> {
    fixture.write(raw);
    fixture
        .files
        .iter()
        .map(|((symbol, vendor, secs, (year, month)), bars)| {
            let rung = Timeframe::from_secs(*secs).unwrap();
            let path =
                StorePath::new(parts(vendor_of(vendor), symbol, rung, ym(*year, *month))).unwrap();
            let header = BarFile::open_existing(raw, path, symbol_id(symbol))
                .unwrap()
                .header();
            let bytes = store::sealed::build(path, &header, bars, None, None)
                .unwrap()
                .to_bytes();
            let sbar = path.with_file(FileKind::SealedBars).to_path_buf(sealed);
            fs::create_dir_all(sbar.parent().unwrap()).unwrap();
            fs::write(&sbar, bytes).unwrap();
            sbar
        })
        .collect()
}

/// SC-3's read door (D-5899) reaches this verb through
/// `BarFile::open_existing`: a month held only sealed (after a retirement)
/// gives the raw form's file byte for byte, and a damaged sealed form beside
/// an intact `.bin` is `read_failed` by name, never answered from the `.bin`
/// and never `absent`.
#[test]
fn a_sealed_month_is_read_through_the_door_like_its_raw_form() {
    let (raw, sealed, both) = (Dir::new("raw"), Dir::new("sealed"), Dir::new("both"));
    let mut fixture = Fixture::default();
    for day in [3, 4] {
        // 200 minutes, so 400 GDFL seconds a day: four sealed frames.
        let span = on(2025, 3, day, hm(9, 15), 200);
        fixture
            .gdfl("SYNS", span, 10)
            .minutes("SYNS", span, (2, 1), 20)
            .day_bar("SYNS", (2025, 3, day), 4_000);
    }
    let sbars = seal_into(&fixture, &raw.0, &sealed.0);
    assert_eq!(sbars.len(), 3, "the 1s, 1min and 1day months are sealed");
    let gdfl = StorePath::new(parts(
        Vendor::Gdfl,
        "SYNS",
        Timeframe::SECOND_1,
        ym(2025, 3),
    ))
    .unwrap();
    assert!(
        !gdfl.to_path_buf(&sealed.0).exists(),
        "the sealed root holds no raw form"
    );
    assert_eq!(
        BarFile::open_existing(&sealed.0, gdfl, symbol_id("SYNS"))
            .unwrap()
            .form(),
        store::file::Form::Sealed,
        "the door serves the sealed form"
    );

    let (out_raw, out_sealed) = (Dir::new("out"), Dir::new("out"));
    let (code, text) = reconcile(&raw.0, &out_raw.0, &[]);
    assert_eq!(code, OK, "{text}");
    let (code, text) = reconcile(&sealed.0, &out_sealed.0, &[]);
    assert_eq!(code, OK, "a month held only sealed is read: {text}");
    let from_raw = file(&out_raw.0, "2025-03", 1);
    let (raw_root, sealed_root) = (raw.0.display().to_string(), sealed.0.display().to_string());
    assert!(from_raw.contains(&raw_root), "the header names its store");
    assert_eq!(
        file(&out_sealed.0, "2025-03", 1),
        from_raw.replace(&raw_root, &sealed_root),
        "the sealed form gives the raw form's file byte for byte but the store it names"
    );
    let all = rows(&from_raw);
    let statuses: Vec<&str> = all.iter().map(|row| row["status"].as_str()).collect();
    assert_eq!(
        statuses,
        ["not_aligned", "matched"],
        "two stock-days; the first has no earlier lag to apply (D-5173 item 4): {all:?}"
    );
    assert_eq!(all[1]["gdfl_day_volume"], "4000", "{all:?}");

    // A DAMAGED SEALED FORM BESIDE AN INTACT RAW ONE: a byte of the GDFL
    // month's last frame. The door refuses it and never reads the `.bin`.
    let placed = seal_into(&fixture, &both.0, &both.0);
    let target = gdfl.with_file(FileKind::SealedBars).to_path_buf(&both.0);
    assert!(placed.contains(&target), "{placed:?}");
    let mut bytes = fs::read(&target).unwrap();
    let last = bytes.len() - 3;
    bytes[last] ^= 0x10;
    fs::write(&target, bytes).unwrap();
    assert!(
        gdfl.to_path_buf(&both.0).exists(),
        "the raw form is beside it"
    );
    let out_both = Dir::new("out");
    let (code, text) = reconcile(&both.0, &out_both.0, &[]);
    assert_eq!(code, FAILED, "a damaged sealed month fails the run: {text}");
    assert!(text.contains("FEED RECONCILE INCOMPLETE"), "{text}");
    let damaged = rows(&file(&out_both.0, "2025-03", 1));
    assert_eq!(damaged.len(), 2, "both days keep their row: {damaged:?}");
    for row in &damaged {
        assert!(
            row["status"].starts_with("read_failed:gdfl_1s:") && row["status"].contains("sealed"),
            "named, never absent and never the .bin's answer: {}",
            row["status"]
        );
        assert_eq!(row["gdfl_day_volume"], "", "no half-read number");
    }
}

#[test]
fn names_with_an_ampersand_are_read_by_their_stored_key() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let span = on(2025, 3, 3, hm(9, 15), 3);
    Fixture::default()
        .gdfl("M&M", span, 10)
        .minutes("M&M", span, (1, 1), 20)
        .gdfl("GVT&D", span, 10)
        // A '-' in a key (BAJAJ-AUTO, NAM-INDIA on the real store) is read by
        // its stored key too (handover issue 29).
        .minutes("SYN-X", span, (1, 1), 20)
        .write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    assert_eq!(row(&all, "M&M", "2025-03-03")["status"], "not_aligned");
    assert_eq!(row(&all, "M&M", "2025-03-03")["best_lag"], "0");
    assert_eq!(row(&all, "GVT&D", "2025-03-03")["status"], "gdfl_only");
    assert_eq!(row(&all, "SYN-X", "2025-03-03")["status"], "zerodha_only");
    let (code, text) = reconcile(&store.0, &out.0, &["--symbols", "M&M"]);
    assert_eq!(code, OK, "{text}");
    let partial = fs::read_to_string(out.0.join("feed-reconcile-2025-03.symbols.v1.csv")).unwrap();
    let only = rows(&partial);
    assert_eq!(only.len(), 1);
    assert_eq!(only[0]["symbol"], "M&M");
    assert!(
        !out.0.join("feed-reconcile-2025-03.v2.csv").exists(),
        "a --symbols run is never a full version"
    );
}

#[test]
fn a_second_day_bar_and_faults_are_flagged_never_misfiled() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let span = on(2025, 3, 3, hm(9, 15), 2);
    Fixture::default()
        .gdfl("SYNJ", span, 10)
        .minutes("SYNJ", span, (1, 1), 20)
        .day_bar("SYNJ", (2025, 3, 3), 40)
        .push(
            "zerodha",
            Timeframe::DAY_1,
            "SYNJ",
            (2025, 3),
            vec![bar(at(2025, 3, 3, 0, 1), BASE, BASE, BASE, BASE, 999)],
        )
        .day_bar("SYNJ", (2025, 3, 5), 0)
        .write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let day = row(&all, "SYNJ", "2025-03-03");
    assert_eq!(day["zerodha_day_volume"], "40");
    assert!(has_flag(day, "zerodha_duplicate_day_bar"));
    let zero = row(&all, "SYNJ", "2025-03-05");
    assert!(has_flag(zero, "zero_day_volume") && has_flag(zero, "zerodha_no_minutes"));
    // Bars a writer refuses are fed to the fold directly.
    let mut scratch = Scratch::new();
    scratch.reset(ym(2025, 3));
    let minute = at(2025, 3, 3, hm(9, 15), 0);
    scratch.add_minute(&bar(minute, BASE, BASE, BASE, BASE, 5));
    scratch.add_minute(&bar(minute, BASE, BASE, BASE, BASE, 7));
    scratch.add_minute(&bar(minute + 1_000_000, BASE, BASE, BASE, BASE, 11));
    scratch.add_minute(&bar(minute + 60_000_000, 0, 0, 0, 0, 1));
    scratch.add_minute(&bar(
        at(2025, 4, 1, hm(9, 15), 0),
        BASE,
        BASE,
        BASE,
        BASE,
        1,
    ));
    scratch.add_day(&bar(at(2025, 2, 28, 0, 0), BASE, BASE, BASE, BASE, 1));
    assert_eq!(
        scratch.outside, 2,
        "outside the month: counted, never placed"
    );
    let flags = flag_text(scratch.zdays[2].flags);
    for flag in [
        "zerodha_unaligned_minutes",
        "zerodha_duplicate_minutes",
        "nonpositive_price",
    ] {
        assert!(flags.contains(flag), "{flags}");
    }
    assert_eq!(
        scratch.zdays[2].volume, 24,
        "every stored minute's volume is in the sum"
    );
}

/// THE FLAGS A MATCHED MINUTE CAN CARRY WITHOUT A FACTOR, end to end: a
/// Zerodha series with one distinct print has no grid
/// (`grid_unknown_minutes`), a GDFL close at or below half its own grid has
/// no upper bound (`interval_unbounded_minutes`), and Zerodha minutes with no
/// day bar say so (`zerodha_no_day_bar`). Each day overlaps and still has no
/// factor, so it is never `matched` and the reader refuses it.
#[test]
fn a_minute_with_no_grid_or_no_bound_admits_nothing_and_is_named() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let day4 = on(2025, 3, 4, hm(9, 15), 2);
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNP").gdfl("SYNG", day4, 10);
    // SYNG: Zerodha prints one price only, and has no day bar.
    let flat: Vec<Bar> = (0..2)
        .map(|n| bar(at(2025, 3, 4, hm(9, 15) + n, 0), BASE, BASE, BASE, BASE, 20))
        .collect();
    fixture.push("zerodha", Timeframe::MINUTE_1, "SYNG", (2025, 3), flat);
    // SYNU: GDFL prints 11 and 1 paisa (grid 10) and closes each minute at
    // 1, so 2g = 2 is not above the grid; Zerodha's own grid is 2.
    let (mut seconds, mut minutes) = (Vec::new(), Vec::new());
    for n in 0..2 {
        let t = |s| at(2025, 3, 4, hm(9, 15) + n, s);
        seconds.push(bar(t(0), 11, 11, 11, 11, 4));
        seconds.push(bar(t(30), 1, 1, 1, 1, 4));
        minutes.push(bar(t(0), 100, 104, 98, 102, 8));
    }
    fixture
        .push("gdfl", Timeframe::SECOND_1, "SYNU", (2025, 3), seconds)
        .push("zerodha", Timeframe::MINUTE_1, "SYNU", (2025, 3), minutes)
        .day_bar("SYNU", (2025, 3, 4), 16);
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let all = rows(&file(&out.0, "2025-03", 1));
    let flat = row(&all, "SYNG", "2025-03-04");
    assert_eq!(
        (flat["applied_lag"].as_str(), flat["lag_source"].as_str()),
        ("0", "pooled")
    );
    assert_eq!(flat["status"], "no_overlap", "no minute admits a factor");
    assert_eq!(flat["matched_minutes"], "2", "both minutes met GDFL");
    assert_eq!(flat["grid_z"], "unknown");
    assert_eq!(flat["median_z"], "", "never 1.0");
    assert!(has_flag(flat, "grid_unknown_minutes"), "{}", flat["flags"]);
    assert!(has_flag(flat, "zerodha_no_day_bar"), "{}", flat["flags"]);
    let low = row(&all, "SYNU", "2025-03-04");
    assert_eq!(low["status"], "no_overlap");
    assert_eq!(low["grid_g"], "10");
    assert!(
        has_flag(low, "interval_unbounded_minutes"),
        "{}",
        low["flags"]
    );
    assert!(!has_flag(low, "zerodha_no_day_bar"), "{}", low["flags"]);
    let index = FactorIndex::load(&out.0, ym(2025, 3)).unwrap();
    for symbol in ["SYNG", "SYNU"] {
        assert_eq!(
            index.factor_of(symbol, Day::new(2025, 3, 4).unwrap()),
            Err(FactorRefusal::NoOverlap),
            "{symbol}"
        );
    }
}

// ------------------------------------------------- versions and determinism

#[test]
fn a_rerun_is_a_new_version_with_the_same_bytes_at_any_worker_count() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let mut fixture = Fixture::default();
    for symbol in ["SYNA", "SYNB", "SYNC", "SYND", "SYNE"] {
        seeded(&mut fixture, symbol);
        let span = on(2025, 3, 4, hm(9, 15), 5);
        fixture
            .gdfl(symbol, span, 10)
            .minutes(symbol, span, (1, 2), 20);
    }
    fixture.write(&store.0);
    let (code, first) = reconcile(&store.0, &out.0, &["--workers", "1"]);
    assert_eq!(code, OK, "{first}");
    let one = file(&out.0, "2025-03", 1);
    let (code, second) = reconcile(&store.0, &out.0, &["--workers", "4"]);
    assert_eq!(code, OK, "{second}");
    let (code, third) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{third}");
    assert!(third.contains("FEED RECONCILE v3"), "{third}");
    assert_eq!(
        file(&out.0, "2025-03", 1),
        one,
        "version 1 is never rewritten"
    );
    assert_eq!(file(&out.0, "2025-03", 2), one, "same bytes, 4 workers");
    assert_eq!(
        file(&out.0, "2025-03", 3),
        one,
        "same bytes, generated workers"
    );
    assert!(
        third.contains("rounds 1w/1i/"),
        "generated: the first round is one worker: {third}"
    );
    let log = fs::read_to_string(out.0.join("feed-reconcile.summary.log")).unwrap();
    assert_eq!(log.lines().count(), 3, "one summary line a run");
    assert!(!one.contains("timing"), "the file carries no machine speed");
    let leftovers = fs::read_dir(&out.0)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp-"))
        .count();
    assert_eq!(leftovers, 0, "no temporary is left behind");
}

#[test]
fn publishing_takes_the_next_free_version_and_replaces_nothing() {
    let dir = Dir::new("versions");
    let month = ym(2025, 3);
    fs::write(dir.0.join("feed-reconcile-2025-03.v7.csv"), b"kept").unwrap();
    fs::write(
        dir.0.join("feed-reconcile-2025-03.vx.csv"),
        b"not a version",
    )
    .unwrap();
    let (path, version, cleared) = publish(&dir.0, month, Scope::Full, b"new").unwrap();
    assert_eq!(cleared, None, "a staging name of its own clears nothing");
    assert_eq!(version, 8);
    assert_eq!(fs::read(&path).unwrap(), b"new");
    assert_eq!(
        fs::read(dir.0.join("feed-reconcile-2025-03.v7.csv")).unwrap(),
        b"kept"
    );
    assert_eq!(newest(&dir.0, month).unwrap().map(|(_, v)| v), Some(8));
    assert_eq!(newest(&dir.0.join("absent"), month).unwrap(), None);
    // A `--symbols` file has its own name and numbers, and is never the
    // month's newest version (D-5178).
    let (partial, number, _) = publish(&dir.0, month, Scope::Symbols, b"partial").unwrap();
    assert_eq!(number, 1);
    assert_eq!(
        partial.file_name().unwrap().to_string_lossy(),
        "feed-reconcile-2025-03.symbols.v1.csv"
    );
    assert_eq!(
        newest(&dir.0, month).unwrap().map(|(_, v)| v),
        Some(8),
        "the authority is still the newest FULL version"
    );
    assert_eq!(
        newest_of(&dir.0, month, Scope::Symbols)
            .unwrap()
            .map(|(_, v)| v),
        Some(1)
    );
    let (_, again, _) = publish(&dir.0, month, Scope::Full, b"next").unwrap();
    assert_eq!(again, 9, "a --symbols file takes no full version number");
}

/// A NAME THAT DOES NOT ROUND-TRIP IS NO VERSION (`v+7` and `v007` parse as
/// numbers this verb never writes), and a publish that cannot take a number
/// refuses by name and leaves no temporary behind.
#[test]
fn a_version_name_must_round_trip_and_a_failed_publish_leaves_no_temporary() {
    let dir = Dir::new("versions");
    let month = ym(2025, 3);
    for odd in [
        "feed-reconcile-2025-03.v+7.csv",
        "feed-reconcile-2025-03.v007.csv",
        "feed-reconcile-2025-03.v.csv",
    ] {
        fs::write(dir.0.join(odd), b"odd").unwrap();
    }
    assert_eq!(newest(&dir.0, month).unwrap(), None);
    let (path, version, _) = publish(&dir.0, month, Scope::Full, b"first").unwrap();
    assert_eq!(version, 1);
    assert_eq!(newest(&dir.0, month).unwrap(), Some((path, 1)));
    fs::write(
        dir.0
            .join(format!("feed-reconcile-2025-03.v{}.csv", u32::MAX)),
        b"last",
    )
    .unwrap();
    let before = contents(&dir.0);
    let refused = publish(&dir.0, month, Scope::Full, b"none left").unwrap_err();
    assert!(
        refused.contains("no version number is left for 2025-03"),
        "{refused}"
    );
    assert_eq!(
        contents(&dir.0),
        before,
        "no temporary and no version left behind"
    );
}

/// THE TEMPORARY IS CREATED THROUGH THE STAGING DOOR (gate 28, R3-16; found
/// refused on this file by the repair round of 20 Oct 2026): a second name of
/// another file at the staging name is refused by name and that file keeps
/// every byte; a symbolic link there is refused and left as it is; a one-name
/// leftover is a dead writer's, cleared, its size handed back and said on the
/// screen, and the publish goes ahead.
#[test]
fn the_temporary_is_created_through_the_staging_door() {
    let dir = Dir::new("staging");
    let month = ym(2025, 3);
    let tmp = dir.0.join(".feed-reconcile-2025-03.tmp-1-2");
    let other = dir.0.join("other.csv");
    fs::write(&other, b"kept bytes").unwrap();

    fs::hard_link(&other, &tmp).unwrap();
    let refused = publish_through(&dir.0, month, Scope::Full, b"new", &tmp).unwrap_err();
    assert!(
        refused.starts_with("creating the temporary: staging_linked: ")
            && refused.ends_with("; nothing was published"),
        "{refused}"
    );
    assert_eq!(
        fs::read(&other).unwrap(),
        b"kept bytes",
        "nothing written through"
    );
    assert_eq!(newest(&dir.0, month).unwrap(), None, "no version");
    fs::remove_file(&tmp).unwrap();

    std::os::unix::fs::symlink(&other, &tmp).unwrap();
    let refused = publish_through(&dir.0, month, Scope::Full, b"new", &tmp).unwrap_err();
    assert!(
        refused.starts_with("creating the temporary: staging_not_regular"),
        "{refused}"
    );
    assert!(
        fs::symlink_metadata(&tmp).unwrap().file_type().is_symlink(),
        "left as it is"
    );
    assert_eq!(fs::read(&other).unwrap(), b"kept bytes");
    assert_eq!(newest(&dir.0, month).unwrap(), None, "no version");
    fs::remove_file(&tmp).unwrap();

    fs::write(&tmp, b"half a file").unwrap();
    let (path, version, cleared) =
        publish_through(&dir.0, month, Scope::Full, b"new", &tmp).unwrap();
    assert_eq!((version, cleared), (1, Some(11)), "the leftover's 11 bytes");
    assert_eq!(fs::read(&path).unwrap(), b"new");
    assert!(!tmp.exists(), "the temporary is removed after the link");
    assert_eq!(fs::read(&other).unwrap(), b"kept bytes");

    let report = |cleared| Report {
        file: path.clone(),
        version,
        scope: Scope::Full,
        carry: "feed-reconcile-2025-02.v1.csv".to_owned(),
        summary: "summary".to_owned(),
        timing: "timing".to_owned(),
        failed: false,
        cleared,
    };
    let mut said = String::new();
    assert_eq!(report(Some(11)).print(&mut said), OK);
    assert!(
        said.contains("FEED RECONCILE NOTE: a dead run's temporary of 11 byte(s) sat at"),
        "{said}"
    );
    let mut quiet = String::new();
    assert_eq!(report(None).print(&mut quiet), OK);
    assert!(!quiet.contains("NOTE"), "{quiet}");
}

/// A MONTH NEITHER FEED HOLDS A SHARE OF IS REFUSED rather than published as
/// an authority of no row, and a month run with no previous file says so on
/// the screen and in the summary log, not only in the file's header.
#[test]
fn an_empty_month_is_refused_and_a_missing_carry_is_said_out_loud() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let mut fixture = Fixture::default();
    for span in [on(2025, 3, 3, hm(9, 15), 2), on(2025, 4, 1, hm(9, 15), 2)] {
        fixture
            .gdfl("SYNA", span, 10)
            .minutes("SYNA", span, (1, 1), 20);
    }
    fixture.write(&store.0);
    let (code, text) = reconcile_month(&store.0, &out.0, "2025-05", &[]);
    assert_eq!(code, FAILED, "{text}");
    assert!(
        text.contains("listed shares holds a bar of 2025-05 in either feed"),
        "{text}"
    );
    assert!(contents(&out.0).0.is_empty(), "nothing written");
    // A store with no share directory at all is refused before any read.
    let bare = Dir::new("store");
    let (code, text) = reconcile_month(&bare.0, &out.0, "2025-03", &[]);
    assert_eq!(code, FAILED, "{text}");
    assert!(
        text.contains(
            "neither feed holds any share directory, for 2025-03 or any other month, under"
        ),
        "a share directory holds every month's files, so the refusal is not of one month: \
         {text}"
    );
    assert!(contents(&out.0).0.is_empty(), "nothing written");
    let (code, text) = reconcile_month(&store.0, &out.0, "2025-03", &[]);
    assert_eq!(code, OK, "{text}");
    assert!(
        text.contains("\ncarry none:no_file_for_2025-02\n"),
        "{text}"
    );
    assert!(
        text.contains(
            "FEED RECONCILE NOTE: no carry from the previous month (no_file_for_2025-02)"
        ),
        "{text}"
    );
    assert!(
        contents(&out.0)
            .1
            .contains("| carry none:no_file_for_2025-02 |"),
        "the summary log names it"
    );
    let (code, text) = reconcile_month(&store.0, &out.0, "2025-04", &[]);
    assert_eq!(code, OK, "{text}");
    assert!(
        text.contains("\ncarry feed-reconcile-2025-03.v1.csv\n"),
        "{text}"
    );
    assert!(!text.contains("FEED RECONCILE NOTE"), "{text}");
}

#[test]
fn the_scheduler_doubles_while_throughput_rises_and_keeps_item_order() {
    let (out, rounds) = schedule(10, None, 4, |at, _| (at * 10, 1_000)).unwrap();
    assert_eq!(out, (0..10).map(|n| n * 10).collect::<Vec<_>>());
    assert_eq!(
        rounds.log.first().map(|r| r.0),
        Some(1),
        "starts at one worker"
    );
    assert!(rounds.last <= 4, "never past the ceiling");
    let items: usize = rounds.log.iter().map(|r| r.1).sum();
    assert_eq!(items, 10, "every item once");
    let (fixed, rounds) = schedule(3, Some(9), 8, |at, _| (at, 0)).unwrap();
    assert_eq!(fixed, vec![0, 1, 2]);
    assert_eq!(rounds.log.len(), 1, "a fixed count runs in one round");
    assert_eq!(rounds.log[0].0, 3, "never more threads than items");
    let (none, _) = schedule(0, None, 4, |at, _| (at, 0)).unwrap();
    assert!(none.is_empty());
}

/// THE COUNT DOUBLES WHILE THROUGHPUT RISES AND GOES BACK WHEN IT FALLS.
/// Item `n` reports `256^n` bars for the first 7 items, so each full round's
/// throughput is tens of thousands of times its predecessor's whatever the
/// host's timing (rounds of 1, 2 and 4 workers); the 8 items of the round
/// of 8 report no bar, so that round's throughput falls to 0 and the rest of
/// the run (and `rounds.last`, which a second pass uses) is back at 4.
#[test]
fn the_scheduler_doubles_while_throughput_rises_and_returns_to_the_fastest_count() {
    let bars = |at: usize| -> u64 {
        if at < 7 {
            1_u64 << (8 * u32::try_from(at).unwrap())
        } else {
            0
        }
    };
    let (out, rounds) = schedule(20, None, 8, |at, _| (at, bars(at))).unwrap();
    assert_eq!(out, (0..20).collect::<Vec<_>>(), "item order kept");
    let threads: Vec<usize> = rounds.log.iter().map(|r| r.0).collect();
    let taken: Vec<usize> = rounds.log.iter().map(|r| r.1).collect();
    assert_eq!(
        threads,
        vec![1, 2, 4, 8, 4],
        "doubled three times, then back"
    );
    assert_eq!(taken, vec![1, 2, 4, 8, 5], "every item once");
    assert_eq!(rounds.last, 4, "the fastest count, not the last one tried");
    // At the ceiling with throughput still rising, it stays at the ceiling.
    let (_, capped) = schedule(7, None, 2, |at, _| (at, bars(at))).unwrap();
    let threads: Vec<usize> = capped.log.iter().map(|r| r.0).collect();
    assert_eq!(threads, vec![1, 2, 2]);
    assert_eq!(capped.last, 2);
}

// ------------------------------------------------------------ the reader

#[test]
fn the_factor_reader_answers_every_day_from_the_newest_file() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let next = on(2025, 3, 4, hm(9, 15), 4);
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNR")
        .gdfl("SYNR", next, 10)
        .minutes("SYNR", next, (1, 2), 20)
        .gdfl("SYNR", on(2025, 3, 5, hm(9, 15), 2), 10)
        .minutes("SYNR", on(2025, 3, 7, hm(9, 15), 2), (1, 1), 20)
        .gdfl("SYNR", on(2025, 3, 10, hm(10, 0), 2), 10)
        .minutes("SYNR", on(2025, 3, 10, hm(11, 0), 2), (1, 1), 20);
    fixture.write(&store.0);
    assert!(
        FactorIndex::load(&out.0, ym(2025, 3)).is_err(),
        "no file, no index"
    );
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let index = FactorIndex::load(&out.0, ym(2025, 3)).unwrap();
    let day = |d| Day::new(2025, 3, d).unwrap();
    let pair = index.factor_of("SYNR", day(4)).unwrap();
    assert_eq!(pair.median, (216_053, 432_106));
    assert_eq!(pair.lag, 0);
    let converted = pair.to_gdfl(216_053);
    assert_eq!(
        (converted.num, converted.den),
        (216_053 * 432_106, 216_053),
        "exact: level x g / z, never a float"
    );
    assert_eq!(
        index.factor_of("SYNR", day(3)),
        Err(FactorRefusal::NotAligned)
    );
    assert_eq!(
        index.factor_of("SYNR", day(5)),
        Err(FactorRefusal::OneFeed("gdfl_only".to_owned()))
    );
    assert_eq!(
        index.factor_of("SYNR", day(7)),
        Err(FactorRefusal::OneFeed("zerodha_only".to_owned()))
    );
    assert_eq!(
        index.factor_of("SYNR", day(10)),
        Err(FactorRefusal::NoOverlap)
    );
    assert_eq!(
        index.factor_of("SYNR", day(11)),
        Err(FactorRefusal::NotChecked)
    );
    assert_eq!(
        index.factor_of("NONE", day(4)),
        Err(FactorRefusal::NotChecked)
    );
    assert_eq!(
        index.factor_of("SYNR", Day::new(2025, 4, 4).unwrap()),
        Err(FactorRefusal::NotChecked),
        "another month is never answered from this file"
    );
    let (path, digest) = file_digest(&out.0, ym(2025, 3)).unwrap();
    assert_eq!(digest, brutex_core::blake3::hash(&fs::read(&path).unwrap()));
    assert_eq!(index.digest, digest);
    // The verb form, timed.
    let dir = out.0.display().to_string();
    let (code, said) = run_with(
        &store.0,
        &[
            "factor",
            "SYNR",
            "2025-03-04",
            "--dir",
            &dir,
            "--repeat",
            "200",
        ],
    );
    assert_eq!(code, OK, "{said}");
    assert!(said.contains("factor median 216053/432106"), "{said}");
    assert!(said.contains("(n=200)"), "{said}");
    assert!(
        said.contains(&format!("blake3 {} loaded in ", super::hex(&digest))),
        "the load is timed where it is paid: {said}"
    );
    let (code, said) = run_with(&store.0, &["factor", "SYNR", "2025-03-03", "--dir", &dir]);
    assert_eq!(code, OK, "{said}");
    assert!(said.contains("refused not_aligned"), "{said}");
    let (code, said) = run_with(&store.0, &["factor", "SYNR", "2025-05-03", "--dir", &dir]);
    assert_eq!(code, FAILED, "{said}");
    assert!(said.contains("FACTOR REFUSED"), "{said}");
}

#[test]
fn an_older_format_is_never_read_as_this_one() {
    let dir = Dir::new("format");
    fs::write(
        dir.0.join("feed-reconcile-2025-02.v1.csv"),
        "# brutex feed-reconcile 1\nsymbol,day\n",
    )
    .unwrap();
    let carried_in = carried(&dir.0, ym(2025, 3)).unwrap();
    assert!(carried_in.stocks.is_empty());
    assert_eq!(
        carried_in.note,
        "none:feed-reconcile-2025-02.v1.csv_is_not_format_3"
    );
    assert!(FactorIndex::load(&dir.0, ym(2025, 2)).is_err());
    let none = carried(&dir.0, ym(2025, 5)).unwrap();
    assert_eq!(none.note, "none:no_file_for_2025-04");
}

/// What the reader, the digest and the next month's carry say of `text`
/// published as the only 2025-03 file of a new directory: each one's
/// refusal, or an empty string where it read the file.
fn refusals_of(text: &str) -> [String; 3] {
    let dir = Dir::new("damaged");
    fs::write(dir.0.join("feed-reconcile-2025-03.v1.csv"), text).unwrap();
    [
        FactorIndex::load(&dir.0, ym(2025, 3))
            .err()
            .unwrap_or_default(),
        file_digest(&dir.0, ym(2025, 3)).err().unwrap_or_default(),
        carried(&dir.0, ym(2025, 4)).err().unwrap_or_default(),
    ]
}

/// `text` with the `column` of `symbol`'s row of `day` set to `value`.
fn with_field(text: &str, symbol: &str, day: &str, column: &str, value: &str) -> String {
    let at = COLUMNS.split(',').position(|n| n == column).unwrap();
    let changed: Vec<String> = text
        .lines()
        .map(|line| {
            let mut fields: Vec<&str> = line.split(',').collect();
            if !line.starts_with('#') && fields[0] == symbol && fields[1] == day {
                fields[at] = value;
            }
            fields.join(",")
        })
        .collect();
    let changed = changed.join("\n") + "\n";
    assert_ne!(changed, text, "the row of {symbol} {day} is in the file");
    changed
}

/// A CUT OR DAMAGED FILE IS NEVER READ AS WHOLE (D-5174 item 6). A file cut
/// at a line boundary used to load as if whole and hand the next month a
/// carry without the shares it lost; a 0 in a pair was served as a factor.
/// SYNR and SYNS have rows; SYNT holds only a February file, so March gives
/// it a carry line and no row, and only the summary's share count can miss it.
#[test]
fn a_cut_or_damaged_file_is_refused_by_name() {
    let (store, out) = (Dir::new("store"), Dir::new("out"));
    let next = on(2025, 3, 4, hm(9, 15), 4);
    let mut fixture = Fixture::default();
    seeded(&mut fixture, "SYNR")
        .gdfl("SYNR", next, 10)
        .minutes("SYNR", next, (1, 2), 20)
        .gdfl("SYNR", on(2025, 4, 1, hm(9, 15), 2), 10);
    seeded(&mut fixture, "SYNS").gdfl("SYNT", on(2025, 2, 28, hm(9, 15), 2), 10);
    fixture.write(&store.0);
    let (code, text) = reconcile(&store.0, &out.0, &[]);
    assert_eq!(code, OK, "{text}");
    let whole = file(&out.0, "2025-03", 1);
    assert!(whole.contains("# carry,SYNT,"), "{whole}");
    assert!(
        !rows(&whole).iter().any(|row| row["symbol"] == "SYNT"),
        "SYNT has no row of March: {whole}"
    );
    assert!(
        whole.contains("# summary: feed-reconcile 2025-03: 3 shares ("),
        "{whole}"
    );
    // The whole file: read by all three, and the factor is served.
    assert_eq!(
        refusals_of(&whole),
        [String::new(), String::new(), String::new()]
    );
    let dir = Dir::new("whole");
    fs::write(dir.0.join("feed-reconcile-2025-03.v1.csv"), &whole).unwrap();
    let index = FactorIndex::load(&dir.0, ym(2025, 3)).unwrap();
    let pair = index
        .factor_of("SYNR", Day::new(2025, 3, 4).unwrap())
        .unwrap();
    assert_eq!(pair.median, (216_053, 432_106));

    let lines: Vec<&str> = whole.lines().collect();
    let first_row = lines.iter().position(|line| *line == COLUMNS).unwrap() + 1;
    let joined = |kept: &[&str]| kept.join("\n") + "\n";
    let cuts = [
        ("without its last line", joined(&lines[..lines.len() - 1])),
        ("without its last byte", whole[..whole.len() - 1].to_owned()),
        ("cut among its rows", joined(&lines[..=first_row])),
        ("with a blank line after", format!("{whole}\n")),
        (
            "with another month's summary",
            whole.replace(
                "# summary: feed-reconcile 2025-03:",
                "# summary: feed-reconcile 2025-02:",
            ),
        ),
    ];
    for (what, text) in &cuts {
        for (reader, why) in ["load", "file_digest", "carry"]
            .iter()
            .zip(refusals_of(text))
        {
            assert!(
                why.contains("is cut or damaged") && why.contains("summary"),
                "a file {what}: {reader} said `{why}`"
            );
            assert!(why.contains("D-5174 item 6"), "{why}");
        }
    }
    // A share with rows and no carry line, named.
    let missing_with_rows: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| !line.starts_with("# carry,SYNS,"))
        .collect();
    for why in refusals_of(&joined(missing_with_rows.as_slice())) {
        assert!(
            why.contains("share `SYNS` has rows and no `# carry,` line"),
            "{why}"
        );
    }
    // A share with no row and no carry line: only the count sees it.
    let missing_without_a_row: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| !line.starts_with("# carry,SYNT,"))
        .collect();
    for why in refusals_of(&joined(missing_without_a_row.as_slice())) {
        assert!(
            why.contains("its summary counts 3 shares and it holds 2 carry lines"),
            "{why}"
        );
    }
    // A paisa price below 1 in a pair: the reader of the pairs names it.
    for (column, value) in [("median_g", "0"), ("min_z", "-7"), ("max_g", "0")] {
        let damaged = with_field(&whole, "SYNR", "2025-03-04", column, value);
        let [load, digest, carry] = refusals_of(&damaged);
        assert!(
            load.contains(&format!("column `{column}` holds `{value}`"))
                && load.contains("a paisa price is at least 1"),
            "{load}"
        );
        assert!(load.contains("SYNR 2025-03-04"), "{load}");
        assert_eq!((digest, carry), (String::new(), String::new()));
    }
    // The run that needs a cut file's carry stops and writes nothing.
    let cut = Dir::new("cut");
    fs::write(cut.0.join("feed-reconcile-2025-03.v1.csv"), &cuts[0].1).unwrap();
    let (code, text) = reconcile_month(&store.0, &cut.0, "2025-04", &[]);
    assert_eq!(code, FAILED, "{text}");
    assert!(text.contains("FEED RECONCILE FAILED"), "{text}");
    assert!(text.contains("is cut or damaged"), "{text}");
    assert_eq!(
        fs::read_dir(&cut.0).unwrap().count(),
        1,
        "nothing was written beside the cut file"
    );
    let dir = cut.0.display().to_string();
    let (code, said) = run_with(&store.0, &["factor", "SYNR", "2025-03-04", "--dir", &dir]);
    assert_eq!(code, FAILED, "{said}");
    assert!(said.contains("FACTOR REFUSED"), "{said}");
}

// -------------------------------------------------------------- the parts

#[test]
fn the_grid_is_the_gcd_of_distinct_print_differences_so_far() {
    let mut grid = Grid::default();
    assert_eq!(grid.step(), 0, "no print, no grid");
    assert!(!grid.see(&bar(0, 1_000, 1_000, 1_000, 1_000, 1)));
    assert_eq!(grid.step(), 0, "one distinct print is not a grid");
    grid.see(&bar(0, 1_010, 1_050, 1_000, 1_020, 1));
    assert_eq!(grid.step(), 10);
    grid.see(&bar(0, 1_005, 1_005, 1_005, 1_005, 1));
    assert_eq!(grid.step(), 5, "it only shrinks");
    assert!(
        grid.see(&bar(0, 0, 1_005, -5, 1_005, 1)),
        "a non-positive price is named"
    );
    assert_eq!(grid.step(), 5, "and never seen");
}

#[test]
fn the_allowance_interval_and_the_running_intersection_are_exact() {
    let pair = |minute, z, g, sz, sg| PairAt {
        minute,
        z,
        g,
        sz,
        sg,
    };
    let (low, high) = bounds(pair(0, 100, 100, 2, 2)).unwrap();
    assert_eq!(
        (low, high),
        (Frac { num: 198, den: 202 }, Frac { num: 202, den: 198 })
    );
    assert!(
        bounds(pair(0, 100, 1, 2, 2)).is_none(),
        "2g <= s_g has no upper bound"
    );
    assert!(bounds(pair(0, 0, 100, 2, 2)).is_none());
    // NO PRICE CAN OVERFLOW A CROSS-MULTIPLICATION: above 2^60 paisa a
    // minute admits nothing (counted unbounded), never a panicking worker.
    assert!(bounds(pair(0, i64::MAX, i64::MAX, 2, 2)).is_none());
    assert!(
        bounds(pair(0, 1 << 60, 1 << 60, 2, 2)).is_some(),
        "the ceiling itself is held"
    );
    assert!(bounds(pair(0, (1 << 60) + 1, 100, 2, 2)).is_none());
    assert!(bounds(pair(0, 100, 100, 2, u64::MAX)).is_none());
    let mut running = Running::default();
    assert_eq!(running.interval(), None);
    let first = pair(1, 100, 100, 2, 2);
    let (lo, hi) = bounds(first).unwrap();
    running.admit(first, lo, hi);
    let second = pair(2, 101, 100, 2, 2);
    let (lo, hi) = bounds(second).unwrap();
    running.admit(second, lo, hi);
    assert_eq!(
        running.interval(),
        Some((Frac { num: 200, den: 202 }, Frac { num: 202, den: 198 }))
    );
    let third = pair(3, 50, 100, 1, 2);
    let (lo, hi) = bounds(third).unwrap();
    running.admit(third, lo, hi);
    assert_eq!(running.interval(), None);
    assert_eq!(
        running.emptied,
        Some((third, second)),
        "the emptying minute and the opposing bound's"
    );
    let fourth = pair(4, 100, 100, 2, 2);
    let (lo, hi) = bounds(fourth).unwrap();
    running.admit(fourth, lo, hi);
    assert_eq!(
        running.emptied,
        Some((third, second)),
        "the first emptying is kept"
    );
    // THE RANGE COMPANION (D-5176): the two ranges widened by half a step.
    let minute = ZMinute {
        held: true,
        high: 106,
        low: 98,
        close: 104,
        step: 2,
    };
    let window = Window {
        high: 106,
        low: 98,
        close: 104,
        step: 2,
        volume: 0,
        traded: false,
    };
    assert_eq!(
        range_bounds(minute, window),
        Some((Frac { num: 194, den: 214 }, Frac { num: 214, den: 194 }))
    );
    let floor = Window { low: 1, ..window };
    assert_eq!(
        range_bounds(minute, floor),
        None,
        "2·g_low <= s_g has no bound"
    );
    let unpriced = ZMinute { low: 0, ..minute };
    assert_eq!(range_bounds(unpriced, window), None);
    let huge = ZMinute {
        high: i64::MAX,
        ..minute
    };
    assert_eq!(range_bounds(huge, window), None, "above the ceiling");
    let inverted = Window { high: 97, ..window };
    assert_eq!(range_bounds(minute, inverted), None, "a high below its low");
    assert!(Frac { num: 1, den: 3 }.lt(Frac { num: 1, den: 2 }));
    assert!(Frac { num: 2, den: 4 }.le(Frac { num: 1, den: 2 }));
    assert!(!Frac { num: 2, den: 4 }.lt(Frac { num: 1, den: 2 }));
}

#[test]
fn carry_lines_round_trip_and_bad_ones_are_refused_by_name() {
    let mut lags = [0; LAG_BINS];
    lags[bin(-20)] = 4;
    lags[bin(0)] = 9;
    let carry = Carry {
        lags,
        interval: Some((
            epoch(2025, 3, 31),
            Frac { num: 1, den: 3 },
            Frac { num: 2, den: 5 },
        )),
        exceed_open: Some((epoch(2025, 3, 28), 2)),
        cut_onset: Some(epoch(2025, 3, 4)),
    };
    let line = carry_line("M&M", &carry);
    assert_eq!(
        line,
        "# carry,M&M,lags=-20:4;0:9,interval=2025-03-31:1/3:2/5,exceed_open=2025-03-28:2,\
         cut_onset=2025-03-04"
    );
    assert_eq!(parse_carry(&line).unwrap(), ("M&M".to_owned(), carry));
    let empty = carry_line("SYNA", &Carry::default());
    assert_eq!(parse_carry(&empty).unwrap().1, Carry::default());
    for bad in [
        "# carry,SYNA",
        "# carry,SYNA,lags=99:1,interval=-,exceed_open=-,cut_onset=-",
        "# carry,SYNA,lags=x,interval=-,exceed_open=-,cut_onset=-",
        "# carry,SYNA,lags=,interval=2025-03-31:1/0:1/2,exceed_open=-,cut_onset=-",
        "# carry,SYNA,lags=,interval=-,exceed_open=2025-03-28,cut_onset=-",
        "# carry,SYNA,lag=,interval=-,exceed_open=-,cut_onset=-",
        "# carry,SYNA,lags=,interval=2025-03-31:9223372036854775807000/1:1/2,exceed_open=-,cut_onset=-",
        "# carry,SYNA,lags=,interval=2025-03-31:1/2:1/4611686018427387904,exceed_open=-,cut_onset=-",
    ] {
        assert!(parse_carry(bad).is_err(), "{bad}");
    }
}

#[test]
fn percentages_ranks_rounding_and_flags() {
    assert_eq!(percent(90, 100), Some(-10.0));
    assert_eq!(percent(1, 0), None);
    assert_eq!(rank(&[], 99), 0);
    assert_eq!(rank(&[5], 50), 5);
    let hundred: Vec<u64> = (1..=100).collect();
    assert_eq!((rank(&hundred, 50), rank(&hundred, 99)), (50, 99));
    assert_eq!(
        (
            round_half_up(5, 2),
            round_half_up(4, 3),
            round_half_up(7, 7)
        ),
        (3, 1, 1)
    );
    assert_eq!(flag_text(0), "");
    assert_eq!(flag_text(1 | (1 << 2)), "factor_not_constant|ex_date");
    assert_eq!(previous(ym(2025, 1)), Some(ym(2024, 12)));
    assert_eq!(previous(ym(2025, 3)), Some(ym(2025, 2)));
}

#[test]
fn the_calendar_names_each_day() {
    assert_eq!(day_kind(epoch(2025, 3, 3)), "open_375");
    assert_eq!(day_kind(epoch(2025, 3, 2)), "closed");
    assert_eq!(day_kind(0), "unmeasured");
}

#[test]
fn the_column_header_flags_and_format_agree() {
    assert_eq!(COLUMNS.split(',').count(), COLUMN_COUNT);
    assert!(COLUMNS.starts_with("symbol,day,day_kind,status,"));
    assert!(COLUMNS.ends_with(",flags,source"));
    assert_eq!(FLAG_NAMES.len(), 17);
    let unique: std::collections::HashSet<&str> = FLAG_NAMES.iter().copied().collect();
    assert_eq!(unique.len(), FLAG_NAMES.len());
    assert_eq!(FORMAT, "# brutex feed-reconcile 3");
    assert!(FORMAT.ends_with(&super::FORMAT_NUMBER.to_string()));
}

// ------------------------------------------------------------------- months

/// One census row: `rows` rows of `symbol` in `segment` at `rung` in `month`.
fn census_row(
    segment: brutex_core::instrument::Segment,
    contract: Option<&str>,
    symbol: &str,
    rung: Timeframe,
    month: YearMonth,
    rows: u64,
) -> pull::manifest::Held {
    use pull::manifest::{Closes, Entry, EntryKey, Held};
    Held::new(
        Entry {
            key: EntryKey {
                contract: contract.map(|c| brutex_core::instrument::Contract::parse(c).unwrap()),
                exchange: brutex_core::instrument::Exchange::Nse,
                segment,
                symbol: brutex_core::symbol::Symbol::new(symbol).unwrap(),
                timeframe: rung,
                month,
            },
            rows,
            first_ts_micros: 1_000,
            last_ts_micros: 1_000 + i64::try_from(rows).unwrap(),
        },
        Closes::known(BASE, BASE + 2).unwrap(),
    )
}

/// THE SPAN IS WHAT THE CENSUS HOLDS, NOT A TYPED RANGE (operator 17:15 IST,
/// item 6): every month in which a share holds a GDFL `1s`, Zerodha `1min`
/// or Zerodha `1day` month, oldest first; an index, a contract and another
/// rung add nothing; an empty store spans nothing; a census that will not
/// load refuses by name; `months` takes no argument.
#[test]
fn the_span_is_every_month_the_census_holds_for_a_share() {
    use brutex_core::instrument::Segment;
    use pull::ingest::census_store;
    let store = Dir::new("months");
    let (code, text) = run_with(&store.0, &["months"]);
    assert_eq!(
        (code, text.as_str()),
        (OK, ""),
        "an empty store spans nothing"
    );
    census_store::record(
        &store.0,
        Vendor::Gdfl,
        &[
            census_row(
                Segment::Cash,
                None,
                "M&M",
                Timeframe::SECOND_1,
                ym(2019, 3),
                9,
            ),
            census_row(
                Segment::Index,
                None,
                "NIFTY",
                Timeframe::SECOND_1,
                ym(2017, 5),
                9,
            ),
            census_row(
                Segment::Fno,
                Some("2019-03-28-FUT"),
                "SYNA",
                Timeframe::SECOND_1,
                ym(2017, 6),
                9,
            ),
        ],
    )
    .unwrap();
    census_store::record(
        &store.0,
        Vendor::Zerodha,
        &[
            census_row(
                Segment::Cash,
                None,
                "SYNA",
                Timeframe::DAY_1,
                ym(2018, 1),
                5,
            ),
            census_row(
                Segment::Cash,
                None,
                "SYNB",
                Timeframe::MINUTE_1,
                ym(2018, 2),
                7,
            ),
            census_row(
                Segment::Cash,
                None,
                "SYNB",
                Timeframe::MINUTE_5,
                ym(2017, 7),
                7,
            ),
        ],
    )
    .unwrap();
    assert_eq!(
        super::months_held(&store.0).unwrap(),
        vec![ym(2018, 1), ym(2018, 2), ym(2019, 3)]
    );
    let (code, text) = run_with(&store.0, &["months"]);
    assert_eq!((code, text.as_str()), (OK, "2018-01\n2018-02\n2019-03\n"));
    // A PARTITIONED CENSUS IS READ IN ITS CASH PARTITION: after the GDFL
    // census migrates, a share month recorded there is listed, and one of a
    // contract in the FNO partition is not.
    census_store::migrate(&store.0, Vendor::Gdfl).unwrap();
    census_store::record(
        &store.0,
        Vendor::Gdfl,
        &[
            census_row(
                Segment::Cash,
                None,
                "SYNC",
                Timeframe::SECOND_1,
                ym(2016, 2),
                4,
            ),
            census_row(
                Segment::Fno,
                Some("2016-01-28-FUT"),
                "SYNC",
                Timeframe::SECOND_1,
                ym(2016, 1),
                4,
            ),
        ],
    )
    .unwrap();
    assert!(matches!(
        census_store::locate(&store.0, Vendor::Gdfl).unwrap(),
        census_store::Location::Partitioned(_)
    ));
    let (code, text) = run_with(&store.0, &["months"]);
    assert_eq!(
        (code, text.as_str()),
        (OK, "2016-02\n2018-01\n2018-02\n2019-03\n")
    );
    let (code, text) = run_with(&store.0, &["months", "2018-01"]);
    assert_eq!(code, MISUSED, "{text}");
    assert!(
        text.contains("months takes no argument") && text.contains(USAGE),
        "{text}"
    );
    // A CENSUS THAT WILL NOT LOAD REFUSES; NO DIRECTORY WALK STANDS IN.
    let census = pull::manifest::manifest_path(&store.0, Vendor::Zerodha);
    let mut bytes = fs::read(&census).unwrap();
    bytes.fill(0x5A);
    fs::write(&census, bytes).unwrap();
    let (code, text) = run_with(&store.0, &["months"]);
    assert_eq!(code, FAILED, "{text}");
    assert!(
        text.starts_with("FEED RECONCILE MONTHS REFUSED: the zerodha census:"),
        "{text}"
    );
    let missing = store.0.join("no-such-root");
    let mut text = String::new();
    let code = command_with(
        &["months"],
        &move || Err(format!("`{}` is not a store", missing.display())),
        None,
        Some(COMMIT),
        &mut text,
    );
    assert_eq!(code, FAILED);
    assert!(text.contains("is not a store"), "{text}");
}
