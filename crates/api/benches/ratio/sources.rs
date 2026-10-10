//! Gate 8 for the All sources routes: `/sources/bars.json` and
//! `/sources/day.json` (D-3339, WP-37).
//!
//! # What is timed, and why it is flat
//!
//! A timeline page reads `offset + limit + 1` buckets of each source from one
//! `.tix` seek, and a GDFL fold reads the seconds of those buckets and no
//! more. A day of the day table reads that day's records of each source,
//! located through `.tix`, and is then cached per (generations, instrument,
//! day). Neither may cost more because the month around it holds more.
//!
//! So each fixture month holds a FIXED last day — 2024-06-30: 1,000 GDFL
//! seconds from 09:15, 375 Zerodha minutes and one Zerodha day — and grows
//! only by the GDFL seconds it holds on earlier days: 0, 9,000 and 99,000
//! more, making months of 1,000, 10,000 and 100,000 seconds (`MONTH_SIZES`).
//! Every figure below reads only the last day, and a ratio past the ceiling
//! is a page or a day whose cost tracks the month.
//!
//! Timed:
//!
//! * a 1min timeline page of 10 buckets from 09:15 on the last day — Zerodha's
//!   stored minutes beside GDFL's seconds folded by `pull::fold`;
//! * the same page newest first from 09:25, so the walk runs backwards;
//! * a one-day day table, served from its cache after the first read.
//!
//! **Printed, not asserted:** the first, uncached read of the day, which folds
//! the day's 1,000 seconds and 375 minutes. It is bounded by the day (at most
//! one session of seconds), not by the month, and the line shows it.

use std::path::{Path, PathBuf};
use std::time::Instant;

use api::server::sources::{DayAsk, TimelineAsk, day_table, timeline};
use brutex_core::vendor::Vendor;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe};

use super::{MONTH_SIZES, RATIO_CEILING_PERMILLE, bench_month, cost_ps, ratio, refuse, second};

/// GDFL seconds on the fixed last day.
const LAST_DAY_SECONDS: usize = 1_000;

/// 2024-06-30 00:00 IST, in seconds after 2024-06-01 00:00 IST.
const LAST_DAY_OFFSET: usize = 29 * 86_400;

/// 09:15 IST, in seconds after midnight.
const OPEN: usize = 9 * 3_600 + 15 * 60;

/// One flat bar at `ts` near ₹1,000.
fn flat_bar(ts_micros: i64, i: usize) -> Bar {
    let raw = i64::try_from(i % 1_000).unwrap_or(0);
    Bar {
        ts_micros,
        open: 100_000 + raw,
        high: 100_100 + raw,
        low: 99_900 + raw,
        close: 100_050 + raw,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// Appends `bars` to one NIFTY INDEX month of `vendor` at `rung`.
fn write(root: &Path, vendor: Vendor, rung: Timeframe, bars: &[Bar]) {
    let Ok(path) = StorePath::new(PathParts {
        vendor,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: rung,
        month: bench_month(),
        file: FileKind::Bars,
    }) else {
        refuse("the sources bench path is a legal one")
    };
    let Ok(id) = u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX)) else {
        refuse("the low 32 bits of a hash fit 32 bits")
    };
    let mut file = match store::file::BarFile::open_or_create(root, path, id) {
        Ok(file) => file,
        Err(why) => refuse(&format!("the sources bench month would not open: {why}")),
    };
    if let Err(why) = file.append(bars) {
        refuse(&format!("the sources bench month would not fill: {why}"));
    }
}

/// A store whose GDFL month holds `n` seconds — `n − 1,000` from the month's
/// first instant and 1,000 from 09:15 on its last day — beside Zerodha's
/// fixed last-day minutes and day bar.
fn store_of(n: usize) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-api-bench-sources-{n}-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(why) = std::fs::create_dir_all(&root) {
        refuse(&format!("the sources bench root would not create: {why}"));
    }
    let early = n.saturating_sub(LAST_DAY_SECONDS);
    let seconds: Vec<Bar> = (0..early)
        .map(|i| flat_bar(second(i), i))
        .chain((0..LAST_DAY_SECONDS).map(|i| flat_bar(second(LAST_DAY_OFFSET + OPEN + i), i)))
        .collect();
    write(&root, Vendor::Gdfl, Timeframe::SECOND_1, &seconds);
    let minutes: Vec<Bar> = (0..375)
        .map(|i| flat_bar(second(LAST_DAY_OFFSET + OPEN + 60 * i), i))
        .collect();
    write(&root, Vendor::Zerodha, Timeframe::MINUTE_1, &minutes);
    write(
        &root,
        Vendor::Zerodha,
        Timeframe::DAY_1,
        &[flat_bar(second(LAST_DAY_OFFSET), 0)],
    );
    root
}

/// One timeline page's row count, or a refusal.
fn page(root: &Path, asked: &TimelineAsk) -> usize {
    match timeline(root, asked) {
        Ok(body) => body
            .get("rows")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len),
        Err(why) => refuse(&format!("the timeline refused: {}", why.why)),
    }
}

/// One day table's day count, or a refusal.
fn days(root: &Path, asked: &DayAsk) -> usize {
    match day_table(root, asked) {
        Ok(body) => body
            .get("days")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len),
        Err(why) => refuse(&format!("the day table refused: {}", why.why)),
    }
}

/// The three sizes' costs of `label`, and whether 1x -> 10x and 1x -> 100x
/// stay under the ceiling.
fn flat(label: &str, costs: &[u128]) -> bool {
    let (Some(&base), Some(&mid), Some(&big)) = (costs.first(), costs.get(1), costs.get(2)) else {
        refuse("three sizes measured");
    };
    ratio(
        &format!("{label} {}->{}", MONTH_SIZES[0], MONTH_SIZES[1]),
        base,
        mid,
        RATIO_CEILING_PERMILLE,
    ) & ratio(
        &format!("{label} {}->{}", MONTH_SIZES[0], MONTH_SIZES[2]),
        base,
        big,
        RATIO_CEILING_PERMILLE,
    )
}

/// The timeline pages and the day table at every size, and whether each
/// stayed flat between 1x and 100x. Run by `crates/api/benches/ratio.rs`'s
/// `main`.
pub(crate) fn pages_are_flat() -> bool {
    let roots: Vec<(usize, PathBuf)> = MONTH_SIZES.iter().map(|&n| (n, store_of(n))).collect();
    let instrument = "exchange=NSE&segment=INDEX&symbol=NIFTY";
    let open = second(LAST_DAY_OFFSET + OPEN) / 1_000_000;
    let mut ok = true;
    for (label, query) in [
        (
            "sources page 1min asc",
            format!("{instrument}&rung=1min&from=2024-06-30&dir=asc&limit=10&cursor={open}"),
        ),
        (
            "sources page 1min desc",
            format!(
                "{instrument}&rung=1min&from=2024-06-30&dir=desc&limit=10&cursor={}",
                open + 600
            ),
        ),
    ] {
        let Ok(asked) = TimelineAsk::parse(&query) else {
            refuse("the bench timeline query");
        };
        let mut costs = Vec::with_capacity(roots.len());
        for (_, root) in &roots {
            if page(root, &asked) != 10 {
                refuse("a bench page is ten buckets");
            }
            costs.push(cost_ps(50, || page(std::hint::black_box(root), &asked)));
        }
        ok &= flat(label, &costs);
    }
    let Ok(asked) = DayAsk::parse(&format!("{instrument}&from=2024-06-30")) else {
        refuse("the bench day query");
    };
    let mut costs = Vec::with_capacity(roots.len());
    for (n, root) in &roots {
        let started = Instant::now();
        if days(root, &asked) != 1 {
            refuse("a bench day table is one day");
        }
        println!(
            "  sources day table first read n={n:<7} {:>8} us   (one day folded: bounded by the day, not asserted)",
            started.elapsed().as_micros()
        );
        costs.push(cost_ps(50, || days(std::hint::black_box(root), &asked)));
    }
    ok &= flat("sources day table cached", &costs);
    for (_, root) in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}
