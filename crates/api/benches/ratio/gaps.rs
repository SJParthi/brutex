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
//! | one one-second index month of 1,024, 10,240 and 102,400 bars (`MONTH_SIZES`), its summary kept, each of `MONTH_ROUNDS` rounds asking every size once (the size asked first rotating) | p99 at 10x and 100x within [`RATIO_CEILING_PERMILLE`] of 1x, and the answer says `kept` and zero bar records read |
//! | the same months with the summary spoiled, so the bars are read | printed, unasserted: it is the slow path a `built` answer names, and it grows with the bars by design |
//! | 240 one-minute months 2006-01..2025-12 with their summaries kept, asked whole and as the ten 24-month tiles that cover them, each round asking the whole once and every tile once (`SPAN_ROUNDS` rounds, order alternating) | the whole's p50 within [`SPAN_MEDIAN_CEILING_PERMILLE`] (one and a half times) of the median of each round's ten tile times summed, and its p99 within [`SPAN_CEILING_PERMILLE`] (twice) of their p99: linear in the months asked, never in their bars, comparing the same months over the same exposure to load (`spans_are_bounded`) |
//! | a 375-bar commit onto a 1x, 10x and 100x month, and the summary step's own work on the same months | printed both, unasserted: the step's work is a read, a fold of the batch and a write of a few kilobytes |
//! | the gap classification of one request over 1, 10 and 100 NSE cash months (`CLASSIFY_MONTHS`) as `/gaps.json` makes it: the share's not-owed key built once (one listing probe of a receipted master that holds it), then `pull::gaps::classify_cash` once a month with that key; `CLASSIFY_ROUNDS` rounds, each timing every size once, the size first rotating (R4-19) | p99 PER DAY at 10x and 100x within [`RATIO_CEILING_PERMILLE`] of 1x |
//! | `pull::gaps::not_owed` alone, once for every day of the same three spans, per call | p99 per call at 10x and 100x within [`RATIO_CEILING_PERMILLE`] of 1x: constant compares and a span the key already holds, whatever the span |
//!
//! R3-06 owns this file; R4-19 added the classification rows. Each fixture
//! root is a new directory of this run's own; like every row in this bench it
//! is removed when its row is done.

use std::time::Instant;

use api::server::{Site, gaps_answer};
use brutex_core::vendor::Vendor;

use super::{MONTH_SIZES, RATIO_CEILING_PERMILLE, month_file, ratio, refuse};

/// Requests per timed row.
const REQUESTS: usize = 200;

/// The whole span's MEDIAN may be at most one and a half times the median of
/// its ten tiles asked one by one in the same round. Linear in the months,
/// it costs no more than they do (one fixed cost against ten); a term in the
/// months squared makes it up to ten times more, at every quantile. The half
/// is headroom for one answer touching ten times the memory. A median is far
/// from where a machine stall decides the order statistic, so this is the
/// row's sharp edge: measured 0.938x to 1.004x under load 16 to 80.
const SPAN_MEDIAN_CEILING_PERMILLE: u128 = 1_500;

/// The whole span's p99 may be at most twice the p99 of its rounds' tile
/// sums. The same linearity, with stated headroom for the tail: a p99 over
/// 600 rounds is the sixth-largest sample, and whether a stall lands in the
/// sixth-largest whole or the sixth-largest sum is a coin the load tosses. On
/// one like-with-like build it read 0.981x, 1.014x and 0.980x on a machine at
/// load 16 to 30 and 1.283x at load 50 to 80 with the medians at 0.993x,
/// 1.004x, 0.998x and 0.938x. A term in the months squared is still ten times.
const SPAN_CEILING_PERMILLE: u128 = 2_000;

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

/// Rounds of the one-second kept rows: each asks every month size once.
///
/// Three times [`REQUESTS`] for the reason [`SPAN_ROUNDS`] is: the gate
/// compares p99s, and at 200 samples a p99 is the second-largest, so two
/// stalls of the machine decide it. At 600 it is the sixth-largest.
const MONTH_ROUNDS: usize = 600;

/// The one-second rows: kept is flat in the bars, spoiled is printed.
///
/// **INTERLEAVED, round by round** (R3-06 repair 1, D-4980). The first form
/// timed all of one size's answers and then all of the next, so a stall of
/// the machine landed in one size's row alone: on a merged tree under load
/// 16-37 the 10,240-bar row printed p99 46.6 ms against 5.8 ms on either
/// side of it (8.1x, BREACH) while its p50 sat at 3.6 ms against 3.3 ms, a
/// breach made of load, not of bars. Now every round asks each size once,
/// the size asked first rotating, so each sample of every size is exposed to
/// the same machine over the same wall time and a stall is as likely to land
/// in any of them. The ceiling is unchanged: a kept answer that read the
/// bars would still grow with them, and the answer's own words (`kept`, zero
/// bar records read) are asserted on every sample. Proof: this bench, run by
/// `crates/api/benches/ratio.rs`, which fails on a breach.
fn months_are_flat(runtime: &tokio::runtime::Runtime) -> bool {
    let query = "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&month=2024-06";
    let must = r#""summary":"kept","summary_fallback":null,"bar_records_read":0"#;
    let mut roots = Vec::new();
    for &n in &MONTH_SIZES {
        let (file, root) = month_file("gaps", n);
        let bars = file.path().to_path_buf();
        // THE WRITER'S LOCK IS RELEASED, as an import's is between days.
        drop(file);
        let site = site_over(&root);
        roots.push((root, bars, site));
    }
    let mut kept_ns = vec![Vec::with_capacity(MONTH_ROUNDS); roots.len()];
    for round in 0..MONTH_ROUNDS {
        for step in 0..roots.len() {
            let at = (round + step) % roots.len();
            let (Some((_, _, site)), Some(samples)) = (roots.get(at), kept_ns.get_mut(at)) else {
                refuse("every size has a site and a sample list");
            };
            samples.push(once(runtime, site, query, must));
        }
    }
    let mut kept = Vec::new();
    for (samples, n) in kept_ns.iter_mut().zip(MONTH_SIZES) {
        let figures = percentiles(samples);
        print_row(
            &format!("gaps 1s month {n} bars, summary kept"),
            MONTH_ROUNDS,
            figures,
        );
        kept.push(figures.1);
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
/// gate holds the whole's median to [`SPAN_MEDIAN_CEILING_PERMILLE`] of the
/// median of those per-round sums and its p99 to [`SPAN_CEILING_PERMILLE`] of
/// their p99. A route linear in its months answers the whole for no more than
/// its parts (one fixed cost against ten); one with a term in the months
/// squared answers it for up to ten times more at both quantiles, so the
/// ceilings still catch what the first row was for, whatever kind each month
/// is.
///
/// **Third, the tail's ceiling had no stated headroom.** Like with like, the
/// p99 ratio still moved with the load (0.98x to 1.28x on one build) while
/// the medians held at 0.94x to 1.00x: both p99s are the sixth-largest of
/// 600, and which side a stall lands in at that rank is chance. So the median
/// carries the sharp ceiling and the p99 has twice, both asserted.
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
    // NANOSECONDS INTO THE PICOSECONDS `ratio` PRINTS: the first row handed
    // it nanoseconds, so its line read 1,000 times too small.
    //
    // BOTH QUANTILES ARE ASSERTED, and both are evaluated, so a breach of
    // either is printed whatever the other says (R3-06 repair 1): the median
    // is the sharp edge on how close to "the sum of its parts" the whole is,
    // and the p99 holds the tail to stated headroom.
    let median = ratio(
        &format!("gaps p50 span {SPAN_MONTHS} vs its tiles summed"),
        summed.0.saturating_mul(1_000),
        figures.0.saturating_mul(1_000),
        SPAN_MEDIAN_CEILING_PERMILLE,
    );
    let tail = ratio(
        &format!("gaps p99 span {SPAN_MONTHS} vs its tiles summed"),
        summed.1.saturating_mul(1_000),
        figures.1.saturating_mul(1_000),
        SPAN_CEILING_PERMILLE,
    );
    median && tail
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

/// Months of the classification rows: 1x, 10x and 100x, each a run of the
/// whole months the cash door's own table measures ([`measured_months`]),
/// so all three spans hold the same kind of day.
const CLASSIFY_MONTHS: [u16; 3] = [1, 10, 100];

/// Rounds of the classification rows: each times every size once.
const CLASSIFY_ROUNDS: usize = 200;

/// A gzip member of one stored (uncompressed) deflate block: `api` has no
/// compressor, and the master reader only needs a valid member (the helper
/// `dayflags_rules`' tests use).
fn stored_gzip(text: &[u8]) -> Vec<u8> {
    let mut crc = !0u32;
    for &byte in text {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    let Ok(len) = u16::try_from(text.len()) else {
        refuse("the bench master fits one stored block");
    };
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff, 1];
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(text);
    out.extend_from_slice(&(!crc).to_le_bytes());
    out.extend_from_slice(&u32::from(len).to_le_bytes());
    out
}

/// A listing table loaded from one receipted master under a new root of this
/// run's own, holding FORCEMOT with an invented listing day (2005-01-03).
fn bench_listing() -> (std::path::PathBuf, pull::listing::Listing) {
    let root = std::env::temp_dir().join(format!(
        "brutex-api-bench-gaps-listing-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    let masters = root.join(pull::listing::MASTER_DIR);
    let (Ok(epoch), Ok(listed), Ok(master_day)) = (
        pull::session::Day::new(1980, 1, 1),
        pull::session::Day::new(2005, 1, 3),
        pull::session::Day::new(2026, 8, 3),
    ) else {
        refuse("the bench's dates are real days");
    };
    let Some(isin) = brutex_core::universe::nse_isin("FORCEMOT") else {
        refuse("the universe holds FORCEMOT's ISIN");
    };
    let seconds = u64::from(listed.days_from_epoch() - epoch.days_from_epoch()) * 86_400;
    let csv = format!(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn,ListgDt,RmvlDt,RadmssnDt\n\
         9300,FORCEMOT,EQ,{},1,{seconds},0,0\n",
        isin.as_str()
    );
    if let Err(why) =
        pull::cash_session_cache::install(&masters, master_day, &stored_gzip(csv.as_bytes()))
    {
        refuse(&format!("the bench master would not install: {why}"));
    }
    let table = pull::listing::Listing::load(&masters);
    if table.in_use().is_none() {
        refuse("the bench master did not check");
    }
    (root, table)
}

/// The first and last day of every whole month the cash door's own table
/// measures (`pull::calendar::FIRST_DAY ..= LAST_DAY`) before closing-auction
/// eligibility must be asked for (`pull::vendor::cash_auction_eligibility_required`):
/// each of their days is a sized session or a closed day.
///
/// **Why not any run of months.** The first run of these rows took 100
/// months from 2018-01: the 1x and 10x spans then lay wholly before the
/// table (2019-12-02), every day `unmeasured` and one whole-day run, while
/// the 100x span was mostly sessions walked minute by minute, and the 1x to
/// 100x ratio read 9.04x for a difference of day KIND, not of span length.
fn measured_months() -> Vec<(i64, i64)> {
    let (Ok(first), Ok(last)) = (
        u32::try_from(pull::calendar::FIRST_DAY).map(pull::session::Day::from_days),
        u32::try_from(pull::calendar::LAST_DAY).map(pull::session::Day::from_days),
    ) else {
        refuse("the table's bounds are days");
    };
    let (Ok(first), Ok(last)) = (first, last) else {
        refuse("the table's bounds are real days");
    };
    let mut out = Vec::new();
    let (mut year, mut month) = (first.year(), first.month());
    loop {
        let Ok(start) = pull::session::Day::new(year, month, 1) else {
            refuse("a real month");
        };
        let end = start.end_of_month();
        if end > last || pull::vendor::cash_auction_eligibility_required(end) {
            break;
        }
        if start >= first {
            out.push((
                i64::from(start.days_from_epoch()),
                i64::from(end.days_from_epoch()),
            ));
        }
        (year, month) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
    }
    if out.len() < 12 {
        refuse("the table measures at least a year of whole months");
    }
    out
}

/// `months` of [`measured_months`] in order from the first, starting again
/// at the first when they run out: a request's months, each one the cash
/// door classifies in full, with the same mix of sessions and closed days
/// whatever the count.
fn classify_span(months: u16) -> Vec<(i64, i64)> {
    let measured = measured_months();
    measured
        .iter()
        .copied()
        .cycle()
        .take(usize::from(months))
        .collect()
}

/// One request's classification over `span`, in nanoseconds: the key built
/// once, then one `classify_cash` a month, as `audit_span` makes it.
fn classify_once(table: &pull::listing::Listing, span: &[(i64, i64)]) -> u128 {
    let start = Instant::now();
    let key = pull::gaps::Key::series("NSE", "CASH", "FORCEMOT", Some(table));
    let mut lost = 0_u32;
    for &(first, last) in span {
        let ledger = pull::gaps::classify_cash(&[], first, last, None, &key);
        lost = lost.saturating_add(std::hint::black_box(ledger.lost_minutes()));
    }
    std::hint::black_box(lost);
    start.elapsed().as_nanos()
}

/// `not_owed` alone, once for every day of `span`, in nanoseconds.
fn probe_once(key: &pull::gaps::Key<'_>, span: &[(i64, i64)]) -> u128 {
    let start = Instant::now();
    let mut excused = 0_u32;
    for &(first, last) in span {
        for day in first..=last {
            if pull::gaps::not_owed(std::hint::black_box(key), std::hint::black_box(day)).is_some()
            {
                excused = excused.saturating_add(1);
            }
        }
    }
    std::hint::black_box(excused);
    start.elapsed().as_nanos()
}

/// **The gap classification costs the same per day at 1x, 10x and 100x
/// months (R4-19).** One request's classification as `/gaps.json` makes it
/// ([`classify_once`]), and the not-owed probe alone ([`probe_once`]), each
/// sample divided by its days; interleaved round by round like the rows
/// above, so a stall of the machine is as likely to land in any size. The
/// spans are whole months the cash door measures ([`classify_span`]), so the
/// 100x span holds FORCEMOT's withdrawal (2023-10-26..=2024-02-13) and every
/// not-owed branch of a session is walked.
fn classification_is_flat() -> bool {
    let (root, table) = bench_listing();
    let spans: Vec<Vec<(i64, i64)>> = CLASSIFY_MONTHS.iter().map(|&m| classify_span(m)).collect();
    let days: Vec<u128> = spans
        .iter()
        .map(|span| {
            span.iter()
                .map(|&(first, last)| u128::try_from(last - first + 1).unwrap_or(1))
                .sum::<u128>()
                .max(1)
        })
        .collect();
    let key = pull::gaps::Key::series("NSE", "CASH", "FORCEMOT", Some(&table));
    if key.span.is_none() {
        refuse("the bench key holds the master's span");
    }
    let mut classified = vec![Vec::with_capacity(CLASSIFY_ROUNDS); spans.len()];
    let mut probed = vec![Vec::with_capacity(CLASSIFY_ROUNDS); spans.len()];
    for round in 0..CLASSIFY_ROUNDS {
        for step in 0..spans.len() {
            let at = (round + step) % spans.len();
            let (Some(span), Some(&n), Some(class), Some(probe)) = (
                spans.get(at),
                days.get(at),
                classified.get_mut(at),
                probed.get_mut(at),
            ) else {
                refuse("every size has a span and two sample lists");
            };
            // PICOSECONDS PER DAY, as `ratio` reads them.
            class.push(classify_once(&table, span).saturating_mul(1_000) / n);
            probe.push(probe_once(&key, span).saturating_mul(1_000) / n);
        }
    }
    let _ignored = std::fs::remove_dir_all(root);
    let mut ok = true;
    for (label, samples) in [
        ("gap classification per day", &mut classified),
        ("not_owed per call", &mut probed),
    ] {
        let mut p99s = Vec::new();
        for (sample, months) in samples.iter_mut().zip(CLASSIFY_MONTHS) {
            let (p50, p99, max) = percentiles(sample);
            let row = format!("{label}, {months} month(s)");
            println!("  {row:<46} n={CLASSIFY_ROUNDS} p50 {p50} ps  p99 {p99} ps  max {max} ps");
            p99s.push(p99);
        }
        let (Some(&base), Some(&mid), Some(&big)) = (p99s.first(), p99s.get(1), p99s.get(2)) else {
            refuse("three sizes measured");
        };
        ok &= ratio(
            &format!("{label} p99 1->10 months"),
            base,
            mid,
            RATIO_CEILING_PERMILLE,
        );
        ok &= ratio(
            &format!("{label} p99 1->100 months"),
            base,
            big,
            RATIO_CEILING_PERMILLE,
        );
    }
    ok
}

/// Every `/gaps.json` row. Run by `crates/api/benches/ratio.rs`'s `main`.
pub(crate) fn answers_are_flat() -> bool {
    let runtime = runtime();
    let mut ok = months_are_flat(&runtime);
    ok &= spans_are_bounded(&runtime);
    the_writer_step_is_printed();
    ok &= classification_is_flat();
    ok
}
