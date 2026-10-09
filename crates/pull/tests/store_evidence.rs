//! READ-ONLY MEASUREMENTS OF WP-13b AGAINST A STORE THAT HOLDS REAL MINUTES.
//!
//! Ignored by default: it needs a store root, named by the environment
//! variable `BRUTEX_EVIDENCE_STORE`, and it reads only. Each case opens one
//! stored Zerodha one-minute month with `BarFile::open_existing` (which
//! creates nothing, not even a lock), replays its bars as a Zerodha minute
//! answer through `pull::ingest::from_window` into a SCRATCH store under the
//! temp directory, and compares what this build derives and records there
//! with what the given store holds. Nothing is written beside the given
//! store, and the scratch store is removed afterwards; no row is printed,
//! only counts.
//!
//! Run: `BRUTEX_EVIDENCE_STORE=<root> cargo test -p pull --test store_evidence
//! -- --ignored --nocapture`.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::session::{Day, IST_OFFSET_SECS, Window};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

fn store_root() -> PathBuf {
    PathBuf::from(
        std::env::var_os("BRUTEX_EVIDENCE_STORE")
            .expect("BRUTEX_EVIDENCE_STORE names the store root to read"),
    )
}

/// Every bar of one stored month, or none when the month file is absent.
fn read_month(
    root: &Path,
    segment: &str,
    symbol: &str,
    tf: Timeframe,
    month: YearMonth,
) -> Vec<store::format::Bar> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe: tf,
        month,
        file: FileKind::Bars,
    })
    .unwrap();
    let id = u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX)).unwrap();
    let Ok(file) = store::file::BarFile::open_existing(root, path, id) else {
        return Vec::new();
    };
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

/// Bars of `bars` on `day`.
fn on_day(day: Day, bars: &[store::format::Bar]) -> usize {
    let midnight = (i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS) * 1_000_000;
    bars.iter()
        .filter(|b| (midnight..midnight + 86_400_000_000).contains(&b.ts_micros))
        .count()
}

/// What replaying one stored minute month through this build gives.
struct Replay {
    scratch: PathBuf,
    done: pull::ingest::Ingested,
    ledger: String,
}

impl Drop for Replay {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir_all(&self.scratch);
    }
}

fn replay(root: &Path, segment: &str, symbol: &str, month: YearMonth) -> Replay {
    let minutes = read_month(root, segment, symbol, Timeframe::MINUTE_1, month);
    assert!(
        !minutes.is_empty(),
        "{segment} {symbol} {month}: no stored minutes"
    );
    let rows: Vec<pull::fetch::RawRow> = minutes
        .iter()
        .map(|b| pull::fetch::RawRow {
            timestamp: b.ts_micros.div_euclid(1_000_000),
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
            open_interest: None,
        })
        .collect();
    let first = Day::new(month.year(), month.month(), 1).unwrap();
    let request = pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: if segment == "INDEX" {
            pull::vendor::Listing::Index
        } else {
            pull::vendor::Listing::Equity
        },
        window: Window::new(first, first.end_of_month()).unwrap(),
        granularity: pull::vendor::Granularity::Minute1,
    };
    let plan = pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::Gdfl,
        request: &request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment,
        contract: None,
    };
    let scratch = std::env::temp_dir().join(format!(
        "brutex-EVIDENCE-{symbol}-{month}-{}",
        std::process::id()
    ));
    let _fresh = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    let done = pull::ingest::from_window(
        &pull::fetch::RawWindow { rows },
        symbol,
        "STORE EVIDENCE",
        &scratch,
        plan,
    );
    let ledger = fs::read_to_string(scratch.join("audit/zerodha.gaps")).unwrap_or_default();
    Replay {
        scratch,
        done,
        ledger,
    }
}

fn derive_line<'a>(ledger: &'a str, rung: &str) -> &'a str {
    ledger
        .lines()
        .find(|line| line.starts_with("derive ") && line.contains(&format!(" {rung} ")))
        .unwrap_or("(no derive line)")
}

/// DB-013 / D-3321: the disaster-recovery Saturday derives at every rung.
#[test]
#[ignore = "reads the store named by BRUTEX_EVIDENCE_STORE"]
fn banknifty_2024_03_02_derives_from_its_stored_minutes() {
    let root = store_root();
    let month = YearMonth::new(2024, 3).unwrap();
    let saturday = Day::new(2024, 3, 2).unwrap();
    let stored_minutes = read_month(&root, "INDEX", "BANKNIFTY", Timeframe::MINUTE_1, month);
    let replay = replay(&root, "INDEX", "BANKNIFTY", month);
    eprintln!(
        "BANKNIFTY 2024-03-02 1min stored: {}",
        on_day(saturday, &stored_minutes)
    );
    for tf in [
        Timeframe::MINUTE_2,
        Timeframe::MINUTE_5,
        Timeframe::MINUTE_15,
        Timeframe::MINUTE_60,
    ] {
        let stored = read_month(&root, "INDEX", "BANKNIFTY", tf, month);
        let derived = read_month(&replay.scratch, "INDEX", "BANKNIFTY", tf, month);
        eprintln!(
            "BANKNIFTY 2024-03 {}: stored {} bars ({} on 03-02); this build derives {} ({} on 03-02)",
            tf.as_str(),
            stored.len(),
            on_day(saturday, &stored),
            derived.len(),
            on_day(saturday, &derived)
        );
    }
    eprintln!("ledger: {}", derive_line(&replay.ledger, "5min"));
    let derived = read_month(
        &replay.scratch,
        "INDEX",
        "BANKNIFTY",
        Timeframe::MINUTE_5,
        month,
    );
    assert_eq!(on_day(saturday, &stored_minutes), 105);
    assert_eq!(on_day(saturday, &derived), 21);
}

/// DB-G12 / DB-G35 / D-3346: NIFTY 2026-09 derives every session, and the
/// month's line says so.
#[test]
#[ignore = "reads the store named by BRUTEX_EVIDENCE_STORE"]
fn nifty_2026_09_derives_every_session() {
    let root = store_root();
    let month = YearMonth::new(2026, 9).unwrap();
    let replay = replay(&root, "INDEX", "NIFTY", month);
    for tf in [Timeframe::MINUTE_5, Timeframe::MINUTE_60] {
        let stored = read_month(&root, "INDEX", "NIFTY", tf, month);
        let derived = read_month(&replay.scratch, "INDEX", "NIFTY", tf, month);
        eprintln!(
            "NIFTY 2026-09 {}: stored {} bars, last close {}; this build derives {}, last close {}",
            tf.as_str(),
            stored.len(),
            stored.last().map_or(0, |b| b.close),
            derived.len(),
            derived.last().map_or(0, |b| b.close)
        );
    }
    eprintln!("ledger: {}", derive_line(&replay.ledger, "5min"));
    eprintln!("failures: {}", replay.done.failures.len());
    for failure in replay.done.failures.iter().take(5) {
        eprintln!("  {}", failure.why.chars().take(240).collect::<String>());
    }
    assert!(derive_line(&replay.ledger, "5min").contains(" days=21 "));
}

/// DB-012 / DB-G33 / DB-G37: what a stored month around a Muhurat is
/// missing, as the gap ledger names it day by day.
#[test]
#[ignore = "reads the store named by BRUTEX_EVIDENCE_STORE"]
fn the_gap_ledger_names_each_short_day_of_a_stored_month() {
    let root = store_root();
    for (symbol, year, month) in [("NIFTY", 2020, 11), ("NIFTY", 2024, 11), ("NIFTY", 2021, 2)] {
        let ym = YearMonth::new(year, month).unwrap();
        let replay = replay(&root, "INDEX", symbol, ym);
        let gaps: Vec<&str> = replay
            .ledger
            .lines()
            .filter(|line| line.starts_with("gap "))
            .collect();
        eprintln!(
            "{symbol} {ym}: {} gap line(s), {} receipt failure(s)",
            gaps.len(),
            replay.done.failures.len()
        );
        for line in &gaps {
            eprintln!("  {line}");
        }
        eprintln!("  {}", derive_line(&replay.ledger, "5min"));
    }
}

/// DB-G36 / D-3321: a share's no-trade minutes fold into partial buckets.
#[test]
#[ignore = "reads the store named by BRUTEX_EVIDENCE_STORE"]
fn an_illiquid_share_folds_its_no_trade_minutes_into_partial_buckets() {
    let root = store_root();
    let month = YearMonth::new(2023, 2).unwrap();
    let replay = replay(&root, "CASH", "360ONE", month);
    for tf in [Timeframe::MINUTE_5, Timeframe::MINUTE_60] {
        let stored = read_month(&root, "CASH", "360ONE", tf, month);
        let derived = read_month(&replay.scratch, "CASH", "360ONE", tf, month);
        eprintln!(
            "360ONE 2023-02 {}: stored {} bars; this build derives {}",
            tf.as_str(),
            stored.len(),
            derived.len()
        );
        eprintln!("  {}", derive_line(&replay.ledger, tf.as_str()));
    }
    let gaps = replay
        .ledger
        .lines()
        .filter(|l| l.starts_with("gap "))
        .count();
    eprintln!(
        "360ONE 2023-02: {gaps} gap line(s); failures {}",
        replay.done.failures.len()
    );
    let stored = read_month(&root, "CASH", "360ONE", Timeframe::MINUTE_60, month);
    let derived = read_month(
        &replay.scratch,
        "CASH",
        "360ONE",
        Timeframe::MINUTE_60,
        month,
    );
    assert!(derived.len() > stored.len());
}

/// The cost of the derive fold this build runs per rung-month, measured on a
/// stored month: `derive_minutes` (the aggregated form the derive calls)
/// against `complete_minutes_with_calendar` (the detailed form it called),
/// 200 calls each at five minutes. Run in release for a number worth quoting.
#[test]
#[ignore = "reads the store named by BRUTEX_EVIDENCE_STORE"]
fn the_derive_fold_costs_per_month() {
    let root = store_root();
    for (segment, symbol, year, month) in [("INDEX", "NIFTY", 2026, 9), ("CASH", "360ONE", 2023, 2)]
    {
        let ym = YearMonth::new(year, month).unwrap();
        let minutes = read_month(&root, segment, symbol, Timeframe::MINUTE_1, ym);
        let venue = if segment == "INDEX" {
            pull::vendor::Venue::NseIndex
        } else {
            pull::vendor::Venue::NseCash
        };
        let bucket = pull::fold::Bucket::of_secs(300).unwrap();
        let runtime = pull::calendar::Runtime::default();
        let mut aggregated = Vec::with_capacity(200);
        let mut detailed = Vec::with_capacity(200);
        for _ in 0..200 {
            let start = std::time::Instant::now();
            let derived =
                pull::fold::derive_minutes(&minutes, bucket, venue, None, runtime).unwrap();
            aggregated.push(start.elapsed());
            std::hint::black_box(derived);
            let start = std::time::Instant::now();
            let folded =
                pull::fold::complete_minutes_with_calendar(&minutes, bucket, venue, None, runtime)
                    .unwrap();
            detailed.push(start.elapsed());
            std::hint::black_box(folded);
        }
        aggregated.sort_unstable();
        detailed.sort_unstable();
        eprintln!(
            "{symbol} {ym} 5min fold of {} minutes, n=200: derive_minutes p50 {:?} p99 {:?}; complete_minutes_with_calendar p50 {:?} p99 {:?}",
            minutes.len(),
            aggregated[99],
            aggregated[197],
            detailed[99],
            detailed[197]
        );
        assert!(!minutes.is_empty());
    }
}
