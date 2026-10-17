#![cfg(test)]
//! `cli results` prints one distinct line per distinct run (repro-5).
//!
//! Invented ledger rows only: no row of the operator's ledger is copied.
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use crate::results::{Record, field};
use std::collections::HashSet;

/// 2026-10-15 10:00:00 IST, in microseconds since the epoch.
const FINISHED: i64 = 1_792_038_600_000_000;

/// The finding's repeated row: zerodha NIFTY 1min, nothing kept, nothing
/// traded, figures identical whatever the month.
fn empty_month(id: u8, year: u16, month: u8, finished: i64) -> Record {
    Record {
        identity: [id; 32],
        finished_micros: finished,
        feed: field("zerodha"),
        underlying: field("NIFTY"),
        timeframe: field("1min"),
        from_year: year,
        from_month: month,
        to_year: year,
        to_month: month,
        months_asked: 1,
        months_found: 1,
        bars: 7_875,
        min_hits: 99_999,
        combinations: 0,
        depth: 0,
        halted: 0,
        trades: 0,
        pessimistic: 0,
        optimistic: 0,
        worst_trade: 0,
        max_drawdown: 0,
        winner_mae: 0,
        winner_mfe: 0,
        all_mae: 0,
        exit_rungs: [-1; 5],
        mask_words: [0; 6],
    }
}

fn lines(rows: &[Record]) -> Vec<String> {
    let mut table = String::new();
    crate::results_table(&mut table, rows);
    table.lines().skip(1).map(str::to_owned).collect()
}

/// Forty months of one instrument with identical figures printed forty
/// identical lines; a stale and a current row for one month printed two. Now
/// every distinct run is a distinct line, and each names its instrument, its
/// span, its bars, its identity's prefix and its finish time in IST.
#[test]
fn distinct_runs_print_distinct_result_lines() {
    let months: Vec<Record> = (0..40_u8)
        .map(|i| empty_month(i, 2023 + u16::from(i / 12), 1 + i % 12, FINISHED))
        .collect();
    let printed = lines(&months);
    assert_eq!(printed.len(), 40);
    let distinct: HashSet<&String> = printed.iter().collect();
    assert_eq!(distinct.len(), 40, "{printed:#?}");
    let first = &printed[0];
    for needle in [
        "NIFTY",
        "2023-01",
        "7,875",
        "000000000000",
        "2026-10-15 10:00:00",
    ] {
        assert!(first.contains(needle), "{needle}: {first}");
    }

    // The stale pair: one month, one figure set, two identities.
    let stale = empty_month(7, 2024, 1, FINISHED);
    let current = empty_month(8, 2024, 1, FINISHED);
    let pair = lines(&[stale, current]);
    assert_ne!(pair[0], pair[1], "{pair:#?}");

    // One identity filed twice at different times (a rerun): two lines.
    let rerun = Record {
        finished_micros: FINISHED + 1_000_000,
        ..stale
    };
    let again = lines(&[stale, rerun]);
    assert_ne!(again[0], again[1], "{again:#?}");

    // A span names both ends.
    let span = Record {
        to_month: 6,
        months_asked: 6,
        months_found: 6,
        ..stale
    };
    assert!(lines(&[span])[0].contains("2024-01..2024-06"));
}

/// The BEST COMPLETE RUN line names the run, not only its span: the
/// identity's prefix and the finish time in IST.
#[test]
fn the_best_complete_line_names_its_identity_and_finish_time() {
    let best = Record {
        trades: 3,
        pessimistic: 500,
        optimistic: 600,
        ..empty_month(0xab, 2024, 2, FINISHED)
    };
    let line = crate::best_complete_line(Some(&best));
    assert!(
        line.contains("BEST COMPLETE RUN: zerodha NIFTY 1min 2024-02..2024-02"),
        "{line}"
    );
    assert!(
        line.contains("run abababababab finished 2026-10-15 10:00:00 IST"),
        "{line}"
    );
}

/// A finish stamp the calendar cannot place is printed raw, never dropped.
#[test]
fn an_unplaceable_finish_time_is_printed_raw() {
    assert_eq!(crate::finished_ist(i64::MIN), format!("{} us", i64::MIN));
    // An unset (zero) or pre-epoch stamp is not 1970-01-01 05:30:00 IST: it
    // was never a finish time, and printing it as one dates a damaged row.
    assert_eq!(crate::finished_ist(0), "0 us");
    assert_eq!(crate::finished_ist(-1), "-1 us");
}

/// Per-row cost of the listing's line, measured: `results_table` over one
/// row at a time for 1,000 distinct invented rows. Run with
/// `cargo test -p cli --locked --lib results_line_tests -- --ignored --nocapture`.
#[test]
#[ignore = "a timing measurement, printed for docs/06-limits.md; not a gate"]
fn results_line_cost_per_row() {
    let rows: Vec<Record> = (0..1_000_u32)
        .map(|i| {
            let mut row = empty_month(
                0,
                2018 + u16::try_from(i / 12 % 9).expect("year"),
                1 + u8::try_from(i % 12).expect("month"),
                FINISHED + i64::from(i),
            );
            row.identity[..4].copy_from_slice(&i.to_le_bytes());
            row
        })
        .collect();
    let mut took: Vec<u128> = rows
        .iter()
        .map(|row| {
            let mut table = String::new();
            let start = std::time::Instant::now();
            crate::results_table(&mut table, std::slice::from_ref(row));
            let spent = start.elapsed().as_nanos();
            assert_eq!(table.lines().count(), 2);
            spent
        })
        .collect();
    took.sort_unstable();
    let at = |q: usize| took[(took.len() * q / 100).min(took.len() - 1)];
    println!(
        "results_table one row: n={} p50={}ns p99={}ns max={}ns",
        took.len(),
        at(50),
        at(99),
        took[took.len() - 1]
    );
}
