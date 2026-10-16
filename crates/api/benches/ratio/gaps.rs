//! Gate 8 for `/gaps.json` (R3-06, DB-A30): the gap answer reads one month
//! summary a month asked and no bar, so it costs the same whatever the bars
//! per month, and a span costs at most its months.
//!
//! Timed through `api::server::gaps_answer`, the route's own handler with
//! nothing in front of it, on a Tokio runtime, n sequential requests per row
//! with p50, p99 and max printed (nearest-rank on the sorted samples):
//!
//! | Row | Asserted |
//! |---|---|
//! | one one-second index month of 1,024, 10,240 and 102,400 bars (`MONTH_SIZES`), its summary kept | p99 at 10x and 100x within [`RATIO_CEILING_PERMILLE`] of 1x, and the answer says `kept` and zero bar records read |
//! | the same months with the summary spoiled, so the bars are read | printed, unasserted: it is the slow path a `built` answer names, and it grows with the bars by design |
//! | 24 and 240 one-minute months, each with its summary kept | p99 at 240 within [`SPAN_CEILING_PERMILLE`] (ten times) of 24: linear in the months asked, never in their bars |
//! | a 375-bar commit onto a 1x, 10x and 100x month, and the summary step's own work on the same months | printed both, unasserted: the step's work is a read, a fold of the batch and a write of a few kilobytes |
//!
//! R3-06 owns this file. Each fixture root is a new directory of this run's
//! own; like every row in this bench it is removed when its row is done.

use std::time::Instant;

use api::server::{Site, gaps_answer};
use brutex_core::vendor::Vendor;

use super::{MONTH_SIZES, RATIO_CEILING_PERMILLE, month_file, ratio, refuse};

/// Requests per timed row.
const REQUESTS: usize = 200;

/// A span of ten times the months may cost at most ten times as much.
const SPAN_CEILING_PERMILLE: u128 = 10_000;

/// Commits per timed commit row.
const COMMITS: usize = 100;

/// p50, p99 and max of `samples` in nanoseconds, nearest rank.
fn percentiles(samples: &mut [u128]) -> (u128, u128, u128) {
    samples.sort_unstable();
    let at = |q: usize| {
        let rank = (samples.len() * q).div_ceil(100).max(1);
        samples.get(rank - 1).copied().unwrap_or(0)
    };
    (at(50), at(99), samples.last().copied().unwrap_or(0))
}

/// Times `n` sequential `/gaps.json?{query}` answers over `site`; refuses
/// the row when an answer is not a 200 that contains `must`.
fn timed(
    runtime: &tokio::runtime::Runtime,
    site: &api::server::Loaded,
    query: &str,
    must: &str,
    n: usize,
) -> (u128, u128, u128) {
    let mut samples = Vec::with_capacity(n);
    for _ in 0..n {
        let start = Instant::now();
        let (code, body) = runtime.block_on(gaps_answer(std::sync::Arc::clone(site), query));
        samples.push(start.elapsed().as_nanos());
        if code != 200 || !body.contains(must) {
            refuse(&format!(
                "/gaps.json?{query} answered {code} without {must}: {}",
                body.get(..400).unwrap_or(&body)
            ));
        }
    }
    percentiles(&mut samples)
}

fn print_row(label: &str, n: usize, (p50, p99, max): (u128, u128, u128)) {
    println!(
        "  {label:<46} n={n} p50 {} ms  p99 {} ms  max {} ms",
        ms(p50),
        ms(p99),
        ms(max)
    );
}

/// `ns` as milliseconds to three places, in integers (no float: the
/// workspace forbids float arithmetic).
fn ms(ns: u128) -> String {
    format!("{}.{:03}", ns / 1_000_000, (ns / 1_000) % 1_000)
}

/// A site over `root` with an empty masters directory of its own.
fn site_over(root: &std::path::Path) -> api::server::Loaded {
    let masters = root.join("masters-for-the-bench");
    if let Err(why) = std::fs::create_dir_all(&masters) {
        refuse(&format!("the bench masters directory: {why}"));
    }
    std::sync::Arc::new(Site::load(&masters, root))
}

fn runtime() -> tokio::runtime::Runtime {
    match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(why) => refuse(&format!("a Tokio runtime: {why}")),
    }
}

/// The one-second rows: kept is flat in the bars, spoiled is printed.
fn months_are_flat(runtime: &tokio::runtime::Runtime) -> bool {
    let query = "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&month=2024-06";
    let mut kept = Vec::new();
    let mut roots = Vec::new();
    for &n in &MONTH_SIZES {
        let (file, root) = month_file("gaps", n);
        let bars = file.path().to_path_buf();
        // THE WRITER'S LOCK IS RELEASED, as an import's is between days.
        drop(file);
        let site = site_over(&root);
        let figures = timed(
            runtime,
            &site,
            query,
            r#""summary":"kept","summary_fallback":null,"bar_records_read":0"#,
            REQUESTS,
        );
        print_row(
            &format!("gaps 1s month {n} bars, summary kept"),
            REQUESTS,
            figures,
        );
        kept.push(figures.1);
        roots.push((root, bars, site));
    }
    for ((_, bars, site), n) in roots.iter().zip(MONTH_SIZES) {
        let beside = store::month_summary::path_beside(bars);
        if let Err(why) = std::fs::write(&beside, b"spoiled by the bench") {
            refuse(&format!("spoiling the bench's own summary: {why}"));
        }
        let figures = timed(runtime, site, query, r#""summary":"built""#, REQUESTS / 4);
        print_row(
            &format!("gaps 1s month {n} bars, bars read"),
            REQUESTS / 4,
            figures,
        );
    }
    for (root, _, _) in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    let (Some(&base), Some(&mid), Some(&big)) = (kept.first(), kept.get(1), kept.get(2)) else {
        refuse("three sizes measured");
    };
    let mut ok = ratio(
        &format!("gaps p99 kept {}->{}", MONTH_SIZES[0], MONTH_SIZES[1]),
        base,
        mid,
        RATIO_CEILING_PERMILLE,
    );
    ok &= ratio(
        &format!("gaps p99 kept {}->{}", MONTH_SIZES[0], MONTH_SIZES[2]),
        base,
        big,
        RATIO_CEILING_PERMILLE,
    );
    ok
}

/// `months` one-minute NIFTY months from 2006-01 under a new root, each of
/// 1,100 bars (over the 1,024 a summary needs) from 09:15 IST on its 2nd.
fn minute_months(months: u16) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-api-bench-gaps-span-{months}-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(why) = std::fs::create_dir_all(&root) {
        refuse(&format!("the bench store root would not create: {why}"));
    }
    let Ok(symbol_id) = u32::try_from(brutex_core::universe::fnv1a("NIFTY") & u64::from(u32::MAX))
    else {
        refuse("the low 32 bits of a hash fit 32 bits")
    };
    for index in 0..months {
        let Ok(month) = store::path::YearMonth::new(
            2006 + index / 12,
            u8::try_from(index % 12 + 1).unwrap_or(1),
        ) else {
            refuse("a real month");
        };
        let Ok(path) = store::path::StorePath::new(store::path::PathParts {
            vendor: Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: store::path::Timeframe::MINUTE_1,
            month,
            file: store::path::FileKind::Bars,
        }) else {
            refuse("the bench path is a legal one")
        };
        let (from, _) = month.ist_bounds_micros();
        let open = from + (86_400 + 9 * 3_600 + 15 * 60) * 1_000_000;
        let batch: Vec<store::format::Bar> = (0..1_100_i64)
            .map(|k| store::format::Bar {
                ts_micros: open + k * 60_000_000,
                open: 2_000_000 + k,
                high: 2_000_100 + k,
                low: 1_999_900 + k,
                close: 2_000_050 + k,
                volume: 0,
                open_interest: store::format::OI_NULL,
            })
            .collect();
        let mut file = match store::file::BarFile::open_or_create(&root, path, symbol_id) {
            Ok(file) => file,
            Err(why) => refuse(&format!("the bench month would not open: {why}")),
        };
        if let Err(why) = file.append(&batch) {
            refuse(&format!("the bench month would not fill: {why}"));
        }
    }
    root
}

/// 24 and 240 minute months: the span is linear in its months.
fn spans_are_bounded(runtime: &tokio::runtime::Runtime) -> bool {
    let mut p99 = Vec::new();
    for (months, to) in [(24_u16, "2007-12"), (240, "2025-12")] {
        let root = minute_months(months);
        let site = site_over(&root);
        let query = format!(
            "feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month=2006-01&to={to}"
        );
        let must = format!(r#""summaries":{{"kept":{months},"built":0,"bar_records_read":0}}"#);
        let figures = timed(runtime, &site, &query, &must, REQUESTS);
        print_row(
            &format!("gaps 1min span of {months} months, kept"),
            REQUESTS,
            figures,
        );
        p99.push(figures.1);
        let _ignored = std::fs::remove_dir_all(root);
    }
    let (Some(&base), Some(&big)) = (p99.first(), p99.get(1)) else {
        refuse("two spans measured");
    };
    ratio(
        "gaps p99 span 24->240 months",
        base,
        big,
        SPAN_CEILING_PERMILLE,
    )
}

/// A 375-bar commit onto a month of each size, and the summary step's own
/// work on it, both printed: the commit carries the `.tix`, `.srt` and
/// `.sum` steps; the step row is what the `.sum` one does (read and check
/// the kept summary, fold the batch, encode, write a new file and rename).
fn the_writer_step_is_printed() {
    for &n in &MONTH_SIZES {
        let (mut file, root) = month_file("gaps-commit", n);
        let mut commits = Vec::with_capacity(COMMITS);
        let mut steps = Vec::with_capacity(COMMITS);
        let mut next = n;
        for _ in 0..COMMITS {
            let batch: Vec<store::format::Bar> = (next..next + 375)
                .map(|i| store::format::Bar {
                    ts_micros: super::second(i),
                    open: 2_000_000,
                    high: 2_000_100,
                    low: 1_999_900,
                    close: 2_000_050,
                    volume: 1_000,
                    open_interest: store::format::OI_NULL,
                })
                .collect();
            next += 375;
            let start = Instant::now();
            if let Err(why) = file.append(&batch) {
                refuse(&format!("a bench commit: {why}"));
            }
            commits.push(start.elapsed().as_nanos());
            let header = file.header();
            let start = Instant::now();
            let Ok(mut summary) =
                store::month_summary::read_beside(file.path(), &header, super::bench_month())
            else {
                refuse("the writer kept the month's summary");
            };
            for bar in &batch {
                summary.fold(bar);
            }
            let image = summary.file_image();
            let scratch = root.join("step.sum");
            let renamed = root.join("step-renamed.sum");
            if std::fs::write(&scratch, &image)
                .and_then(|()| std::fs::rename(&scratch, &renamed))
                .is_err()
            {
                refuse("the step's write");
            }
            steps.push(start.elapsed().as_nanos());
        }
        print_row(
            &format!("commit of 375 onto {n} bars"),
            COMMITS,
            percentiles(&mut commits),
        );
        print_row(
            &format!("  .sum step's work at {n} bars"),
            COMMITS,
            percentiles(&mut steps),
        );
        drop(file);
        let _ignored = std::fs::remove_dir_all(root);
    }
}

/// Every `/gaps.json` row. Run by `crates/api/benches/ratio.rs`'s `main`.
pub(crate) fn answers_are_flat() -> bool {
    let runtime = runtime();
    let mut ok = months_are_flat(&runtime);
    ok &= spans_are_bounded(&runtime);
    the_writer_step_is_printed();
    ok
}
