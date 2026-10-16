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
//! | 240 one-minute months 2006-01..2025-12 with their summaries kept, asked whole and as the ten 24-month tiles that cover them, each round asking the whole once and every tile once (`SPAN_ROUNDS` rounds, order alternating) | the whole's p99 within [`SPAN_CEILING_PERMILLE`] (one and a half times) of the p99 of each round's ten tile times summed: linear in the months asked, never in their bars, comparing the same months over the same exposure to load (`spans_are_bounded`) |
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

/// The whole span may cost at most one and a half times its ten tiles asked
/// one by one in the same round. Linear in the months, it costs no more than
/// they do (one fixed cost against ten); a term in the months squared makes
/// it up to ten times more. The half is headroom for one answer touching ten
/// times the memory, and for two p99s that are each an order statistic.
const SPAN_CEILING_PERMILLE: u128 = 1_500;

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

/// Times one `/gaps.json?{query}` answer over `site`, in nanoseconds;
/// refuses the row when the answer is not a 200 that contains `must`.
fn once(
    runtime: &tokio::runtime::Runtime,
    site: &api::server::Loaded,
    query: &str,
    must: &str,
) -> u128 {
    let start = Instant::now();
    let (code, body) = runtime.block_on(gaps_answer(std::sync::Arc::clone(site), query));
    let took = start.elapsed().as_nanos();
    if code != 200 || !body.contains(must) {
        refuse(&format!(
            "/gaps.json?{query} answered {code} without {must}: {}",
            body.get(..400).unwrap_or(&body)
        ));
    }
    took
}

/// Times `n` sequential `/gaps.json?{query}` answers over `site` ([`once`]).
fn timed(
    runtime: &tokio::runtime::Runtime,
    site: &api::server::Loaded,
    query: &str,
    must: &str,
    n: usize,
) -> (u128, u128, u128) {
    let mut samples: Vec<u128> = (0..n).map(|_| once(runtime, site, query, must)).collect();
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

/// Months the span row asks whole, from 2006-01.
const SPAN_MONTHS: u16 = 240;

/// Months in each of the tiles that cover the span, asked one by one.
const TILE_MONTHS: u16 = 24;

/// Rounds of the span row: each asks the whole once and every tile once.
///
/// Three times [`REQUESTS`] because the gate compares two p99s and each is
/// an order statistic: at 200 samples it is the second-largest, and in one
/// quiet run the ten tiles, which cost the same, printed p99s from 10.3 to
/// 17.2 ms. At 600 it is the sixth-largest of each.
const SPAN_ROUNDS: usize = 600;

/// The 240-month span against the ten 24-month tiles it is made of, LIKE
/// WITH LIKE in the months AND in the time each sample is exposed to load
/// (R3-06 repair 1, D-4980).
///
/// **The first row compared different kinds of month.** It timed
/// 2006-01..2007-12 against 2006-01..2025-12 and held the second to ten
/// times the first. The calendar of record (D-3336) vouches for days from
/// `pull::calendar::SOURCED_FIRST_DAY` (2018-01-01): every one of the 24
/// base months was unmeasured days, which the audit counts and passes over,
/// and 96 of the 240 were measured days, which it classifies minute by
/// minute. The tiles printed below are that difference, measured. A ceiling of
/// ten on ten times the months left no margin for it: the row breached at
/// 11.1x and 17.6x on one build and passed at 0.9x under load.
///
/// **The second compared different exposures.** It asked the same months
/// whole and as ten tiles, but held the whole's p99 to the SUM of the ten
/// tiles' own p99s. A tile answer lasts a tenth as long, so a stall of the
/// machine lands in a tenth as many of them and their p99s stay quiet while
/// the whole's does not: under load 68-120 the whole's p99 was 1.50x the
/// summed tile p99s with the medians at 0.99x, a breach made of load.
///
/// Now each round asks the whole once and the ten tiles once, in an order
/// that alternates round by round, and the round's TILE SAMPLE is the sum of
/// its ten answers' times: one sample of the same months and about the same
/// wall time as the whole's, so a stall is as likely to land in either. The
/// gate holds the whole's p99 to [`SPAN_CEILING_PERMILLE`] of the p99 of
/// those per-round sums. A route linear in its months answers the whole for
/// no more than its parts (one fixed cost against ten); one with a term in
/// the months squared answers it for up to ten times more, so the ceiling
/// still catches what the first row was for, whatever kind each month is.
fn spans_are_bounded(runtime: &tokio::runtime::Runtime) -> bool {
    let root = minute_months(SPAN_MONTHS);
    let site = site_over(&root);
    let named = |index: u16| format!("{:04}-{:02}", 2006 + index / 12, index % 12 + 1);
    let ask = |from: u16, months: u16| {
        (
            named(from),
            format!(
                "feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1min&month={}&to={}",
                named(from),
                named(from + months - 1)
            ),
            format!(r#""summaries":{{"kept":{months},"built":0,"bar_records_read":0}}"#),
        )
    };
    let whole = ask(0, SPAN_MONTHS);
    let tiles: Vec<_> = (0..SPAN_MONTHS / TILE_MONTHS)
        .map(|tile| ask(tile * TILE_MONTHS, TILE_MONTHS))
        .collect();
    let mut whole_ns = Vec::with_capacity(SPAN_ROUNDS);
    let mut summed_ns = Vec::with_capacity(SPAN_ROUNDS);
    let mut tile_ns = vec![Vec::with_capacity(SPAN_ROUNDS); tiles.len()];
    for round in 0..SPAN_ROUNDS {
        let mut ask_tiles = || {
            let mut summed = 0_u128;
            for ((_, query, must), samples) in tiles.iter().zip(&mut tile_ns) {
                let took = once(runtime, &site, query, must);
                samples.push(took);
                summed = summed.saturating_add(took);
            }
            summed
        };
        // ALTERNATED, so neither side always runs on the other's warm cache.
        if round % 2 == 0 {
            whole_ns.push(once(runtime, &site, &whole.1, &whole.2));
            summed_ns.push(ask_tiles());
        } else {
            summed_ns.push(ask_tiles());
            whole_ns.push(once(runtime, &site, &whole.1, &whole.2));
        }
    }
    let _ignored = std::fs::remove_dir_all(root);
    for ((from, _, _), samples) in tiles.iter().zip(&mut tile_ns) {
        print_row(
            &format!("gaps 1min {TILE_MONTHS} months from {from}, kept"),
            SPAN_ROUNDS,
            percentiles(samples),
        );
    }
    let summed = percentiles(&mut summed_ns);
    print_row(
        &format!("the {} tiles summed in each round", tiles.len()),
        SPAN_ROUNDS,
        summed,
    );
    let figures = percentiles(&mut whole_ns);
    print_row(
        &format!("gaps 1min span of {SPAN_MONTHS} months, kept"),
        SPAN_ROUNDS,
        figures,
    );
    // THE MEDIANS TOO, printed and not asserted: the p99 ratio is the gate,
    // and the p50 one says how close to "the sum of its parts" the whole is.
    if let Some(permille) = figures.0.saturating_mul(1_000).checked_div(summed.0) {
        println!(
            "  {:<46} ratio {}.{:03}x  (printed, unasserted)",
            format!("gaps p50 span {SPAN_MONTHS} vs its tiles summed"),
            permille / 1_000,
            permille % 1_000
        );
    }
    // NANOSECONDS INTO THE PICOSECONDS `ratio` PRINTS: the first row handed
    // it nanoseconds, so its line read 1,000 times too small.
    ratio(
        &format!("gaps p99 span {SPAN_MONTHS} vs its tiles summed"),
        summed.1.saturating_mul(1_000),
        figures.1.saturating_mul(1_000),
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
