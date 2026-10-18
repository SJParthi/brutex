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
use store::path::{StorePath, Timeframe, YearMonth};

use super::{
    COLUMN_COUNT, COLUMNS, Carry, FLAG_NAMES, FORMAT, FactorIndex, FactorRefusal, Frac, Grid,
    LAG_BINS, PairAt, Running, Scratch, USAGE, Window, ZMinute, ask, bin, bounds, carried,
    carry_line, command_with, day_kind, file_digest, flag_text, lag_order, lookup_ask, mode,
    newest, parse_carry, parse_day, parse_month, parts, percent, previous, publish, range_bounds,
    rank, round_half_up, schedule, symbol_id,
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
        // Only ever a directory this file made, under the temp dir.
        if self.0.starts_with(std::env::temp_dir())
            && self
                .0
                .to_string_lossy()
                .contains("brutex-cli-feed-reconcile-")
        {
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
        Some(COMMIT),
        &mut said,
    );
    assert_eq!(code, FAILED);
    assert!(
        said.contains("FEED RECONCILE REFUSED: no store here. Nothing was read."),
        "{said}"
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
    let only = rows(&file(&out.0, "2025-03", 2));
    assert_eq!(only.len(), 1);
    assert_eq!(only[0]["symbol"], "M&M");
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
    let (path, version) = publish(&dir.0, month, b"new").unwrap();
    assert_eq!(version, 8);
    assert_eq!(fs::read(&path).unwrap(), b"new");
    assert_eq!(
        fs::read(dir.0.join("feed-reconcile-2025-03.v7.csv")).unwrap(),
        b"kept"
    );
    assert_eq!(newest(&dir.0, month).unwrap().map(|(_, v)| v), Some(8));
    assert_eq!(newest(&dir.0.join("absent"), month).unwrap(), None);
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
        "none:feed-reconcile-2025-02.v1.csv_is_not_format_2"
    );
    assert!(FactorIndex::load(&dir.0, ym(2025, 2)).is_err());
    let none = carried(&dir.0, ym(2025, 5)).unwrap();
    assert_eq!(none.note, "none:no_file_for_2025-04");
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
    assert_eq!(FORMAT, "# brutex feed-reconcile 2");
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
        Some(COMMIT),
        &mut text,
    );
    assert_eq!(code, FAILED);
    assert!(text.contains("is not a store"), "{text}");
}
