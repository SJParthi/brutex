//! Wall-clock latency of time-to-row and row lookups — D-2329's measurement.
//!
//! `#[ignore]`d: it writes a 517,500-bar month and times 200,000 lookups per
//! case, so it is run on purpose, in release:
//!
//! ```text
//! cargo test -p store --release --test tix_latency -- --ignored --nocapture
//! ```
//!
//! It asserts nothing about time — a shared host's numbers are not a gate,
//! and C-TIX-01/C-TIX-02 in `benches/ratio.rs` are the gate — and it uses only
//! calls that existed before D-2329 (`open_or_create`, `append`,
//! `open_existing`, `first_at_or_after`, `read_record`), so the same file
//! measures the bisection on the base commit. On a build with the index, the
//! `bisection` rows are the same month with its `.tix` taken away: the legacy
//! path, unchanged code.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::print_stdout
)]

use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

const LOOKUPS: usize = 200_000;
/// 2024-07-01 00:00 IST: a 31-day month.
const JULY_START: i64 = 1_719_772_200_000_000;
const MICROS: i64 = 1_000_000;

fn path(timeframe: Timeframe) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe,
        month: YearMonth::new(2024, 7).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// `days` days (every day when `weekdays` is false) of `per_day` bars
/// `step` seconds apart from 09:15 IST.
fn month(days: i64, per_day: i64, step: i64, weekdays: bool) -> Vec<Bar> {
    let mut out = Vec::new();
    let (mut day, mut taken) = (0, 0);
    while taken < days {
        // 2024-07-01 is a Monday: days 5, 6, 12, 13, ... are the weekend.
        if !weekdays || day % 7 < 5 {
            for k in 0..per_day {
                let ts = JULY_START + day * 86_400 * MICROS + (555 * 60 + k * step) * MICROS;
                out.push(Bar {
                    ts_micros: ts,
                    open: 2_000_000 + k,
                    high: 2_000_100 + k,
                    low: 1_999_900 + k,
                    close: 2_000_050 + k,
                    volume: 10,
                    open_interest: OI_NULL,
                });
            }
            taken += 1;
        }
        day += 1;
    }
    out
}

/// xorshift64*: deterministic, so before and after time the same lookups.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

fn percentiles(mut ns: Vec<u64>) -> (u64, u64, u64) {
    ns.sort_unstable();
    let at = |q: usize| ns[(ns.len() * q / 1000).min(ns.len() - 1)];
    (at(500), at(990), *ns.last().unwrap())
}

fn report(case: &str, ns: Vec<u64>) {
    let (p50, p99, max) = percentiles(ns);
    println!("{case:<58} p50 {p50:>8} ns   p99 {p99:>8} ns   max {max:>9} ns");
}

/// Times `LOOKUPS` calls of `op` on `inputs`, one `Instant` pair per call.
fn time<T: Copy>(inputs: &[T], mut op: impl FnMut(T) -> u64) -> Vec<u64> {
    inputs
        .iter()
        .map(|&input| {
            let start = Instant::now();
            black_box(op(black_box(input)));
            start.elapsed().as_nanos() as u64
        })
        .collect()
}

fn measure(name: &str, timeframe: Timeframe, bars: &[Bar]) {
    let root: PathBuf =
        std::env::temp_dir().join(format!("brutex-tix-latency-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    {
        let mut writer = BarFile::open_or_create(&root, path(timeframe), 7).unwrap();
        for chunk in bars.chunks(22_500) {
            writer.append(chunk).unwrap();
        }
    }
    let n = bars.len() as u64;
    let first = bars[0].ts_micros;
    let last = bars[bars.len() - 1].ts_micros;
    let width = i64::from(timeframe.secs()) * MICROS;
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    // Uniform microseconds over the bars' span: mostly NOT on the grid.
    let anywhere: Vec<i64> = (0..LOOKUPS)
        .map(|_| first + (rng.next() % (last - first + 1) as u64) as i64)
        .collect();
    // The same instants floored to the rung's grid (09:15 is on it).
    let on_grid: Vec<i64> = anywhere
        .iter()
        .map(|&ts| first + (ts - first) / width * width)
        .collect();
    let rows: Vec<u64> = (0..LOOKUPS).map(|_| rng.next() % n).collect();

    let tix = path(timeframe).to_path_buf(&root).with_extension("tix");
    let passes: &[bool] = if tix.exists() {
        &[true, false]
    } else {
        &[false]
    };
    for &with_index in passes {
        if !with_index && tix.exists() {
            fs::remove_file(&tix).unwrap();
        }
        let tag = if with_index { "index" } else { "bisection" };
        let label = |what: &str| format!("{name} ({n} bars) {what} [{tag}]");
        // A freshly reopened handle per case: the first lookup pays the
        // handle's one-time work, and it is inside the sample.
        let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
        report(
            &label("by time, random us"),
            time(&anywhere, |ts| file.first_at_or_after(ts).unwrap()),
        );
        let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
        report(
            &label("by time, on grid"),
            time(&on_grid, |ts| file.first_at_or_after(ts).unwrap()),
        );
        let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
        report(
            &label("by row, random rows (cold block)"),
            time(&rows, |row| file.read_record(row).unwrap().ts_micros as u64),
        );
        // One row again and again: the warm path C-28 times, its block
        // verified once and kept by the handle.
        let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
        let same = vec![rows[0]; LOOKUPS];
        report(
            &label("by row, one row (warm block)"),
            time(&same, |row| file.read_record(row).unwrap().ts_micros as u64),
        );
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
#[ignore = "a measurement, run on purpose in release: see the module doc"]
fn time_and_row_lookup_latency() {
    measure("1min", Timeframe::MINUTE_1, &month(31, 375, 60, false));
    measure("1s", Timeframe::SECOND_1, &month(23, 22_500, 1, true));
}

/// The `.tix` REBUILD, timed at 10^3 to 10^6 one-second bars — D-3302.
///
/// `BarFile::rebuild_index` reads every committed record. A writer pays it at
/// open for a month with records and no index it can confirm, and an
/// `append` pays it when the index entry it resumes from no longer agrees
/// with the header — after an append that failed on the same handle
/// (`index_batch` → `reindex`). Both run the same function; this times the
/// open, which is the one a test can reach without a torn write, by removing
/// the `.tix` and opening a writer. O(`n_valid`) by construction, and that is
/// what the numbers show: it is NOT an O(1) path and is never claimed one.
/// Its cost past 10^6 bars is UNVERIFIED, an extrapolation in
/// `docs/06-limits.md`.
#[test]
#[ignore = "a measurement, run on purpose in release: see the module doc"]
fn index_rebuild_cost_grows_with_the_month() {
    for n in [1_000_i64, 10_000, 100_000, 1_000_000] {
        let root: PathBuf =
            std::env::temp_dir().join(format!("brutex-tix-rebuild-{n}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let bars: Vec<Bar> = (0..n)
            .map(|i| Bar {
                ts_micros: JULY_START + i * MICROS,
                open: 2_000_000,
                high: 2_000_100,
                low: 1_999_900,
                close: 2_000_050,
                volume: 1,
                open_interest: OI_NULL,
            })
            .collect();
        BarFile::open_or_create(&root, path(Timeframe::SECOND_1), 7)
            .unwrap()
            .append(&bars)
            .unwrap();
        let tix = path(Timeframe::SECOND_1)
            .to_path_buf(&root)
            .with_extension("tix");
        let mut ns = Vec::new();
        for _ in 0..7 {
            fs::remove_file(&tix).unwrap();
            let start = Instant::now();
            let writer = BarFile::open_or_create(&root, path(Timeframe::SECOND_1), 7).unwrap();
            ns.push(start.elapsed().as_nanos() as u64);
            assert!(tix.exists(), "the writer open rebuilt no index");
            drop(writer);
        }
        report(&format!("rebuild on writer open, {n} bars"), ns);
        let _ = fs::remove_dir_all(&root);
    }
}
