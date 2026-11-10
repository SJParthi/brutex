//! Findings of the o1p99 attack on `crates/store` (cloud lane
//! `cloud/o1p99-attack-20261110`, 10 Nov 2026). `O1-ATTACK-REPORT.md` at the
//! repository root has the measurements, the likely cause and a proposed fix.
//!
//! EVERY TEST HERE IS `#[ignore]`d BECAUSE IT FAILS on the code it was
//! written against: each asserts the flat p99 shape a claim states and the
//! code does not have. Run them by hand, in the optimised test profile:
//!
//! ```text
//! cargo test --locked -p store --test o1p99_findings -- --ignored --nocapture
//! ```
//!
//! Un-ignore a test when its fix lands; it is then the regression guard.
//! p99 is read as `benches/o1p99.rs` reads it (`benches/o1p99/harness.rs`):
//! interleaved rounds, the quietest round's p99, against a 3.0x ceiling.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::PathBuf;

#[allow(
    dead_code,
    reason = "the bench harness is shared with the findings and not every helper is called here"
)]
#[path = "../benches/o1p99/harness.rs"]
mod harness;

use brutex_core::vendor::Vendor;
use harness::{FACTOR_PERMILLE, Rng, measure, permille, verdict};
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// 2024-06-01 00:00 IST in epoch microseconds.
const JUNE_2024_IST_START: i64 = 1_717_180_200_000_000;

/// The symbol id every file here is created with.
const SYMBOL_ID: u32 = 7;

/// The one-second June 2024 NIFTY month.
fn month_path() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2024, 6).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// The bar at second `i` of the month.
fn bar(i: u64) -> Bar {
    let raw = i64::try_from(i).unwrap();
    Bar {
        ts_micros: JUNE_2024_IST_START + raw * 1_000_000,
        open: 2_000_000 + raw,
        high: 2_000_100 + raw,
        low: 1_999_900 + raw,
        close: 2_000_050 + raw,
        volume: 1_000,
        open_interest: 0,
    }
}

/// A fresh root holding one month of `n` bars, written in one batch.
fn loaded(name: &str, n: u64) -> (BarFile, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "brutex-o1p99-finding-{name}-{n}-{}",
        std::process::id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let mut file = BarFile::open_or_create(&root, month_path(), SYMBOL_ID).unwrap();
    let batch: Vec<Bar> = (0..n).map(bar).collect();
    file.append(&batch).unwrap();
    (file, root)
}

/// FINDING O1P99-F-STORE-01 — time to row is not flat below the `.tix`
/// threshold.
///
/// Claim: C-TIX-01, "Time to row — `BarFile::first_at_or_after` through the
/// `.tix` index — costs the same whatever the file holds"
/// (`docs/04-invariants.md`), and `first_at_or_after`'s doc
/// (`crates/store/src/file.rs`). A month under
/// `time_index::INDEX_MIN_ROWS` (1,024) bars has no `.tix` by the
/// small-month rule (D-4933) and bisects: up to ten record reads, most of
/// them in a different checksum block, so each verifies a block. Measured
/// on the attack host: quietest-round p99 27.4 us at 1,023 bars against
/// 208 ns at 1,024 (about 130x) and 28 ns at 1 bar. C-TIX-01's bench starts
/// at 1,024 bars, so it never times this side.
#[test]
#[ignore = "o1p99 finding O1P99-F-STORE-01: fails today, time to row bisects below 1,024 bars"]
fn finding_store_time_to_row_is_not_flat_below_the_index_threshold() {
    const SIZES: [u64; 3] = [1_024, 100, 1_023];
    let clock = harness::clock_ps();
    let mut months: Vec<(BarFile, PathBuf, u64, Rng)> = SIZES
        .iter()
        .map(|&n| {
            let (writer, root) = loaded("tix", n);
            drop(writer);
            let reader = BarFile::open_existing(&root, month_path(), SYMBOL_ID).unwrap();
            (reader, root, n, Rng::new(n))
        })
        .collect();
    let tails = measure(
        &mut months,
        5,
        2_000,
        1,
        clock,
        |_| {},
        |(f, _, n, r), _| {
            let s = i64::try_from(r.below(*n)).unwrap();
            f.first_at_or_after(JUNE_2024_IST_START + s * 1_000_000)
                .unwrap()
        },
    );
    let flat = verdict(
        "O1P99-F-STORE-01",
        "first_at_or_after random stamp: 1,024 (indexed) -> 100, 1,023 (bisected)",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    );
    for (f, root, _, _) in months {
        drop(f);
        let _ignored = std::fs::remove_dir_all(root);
    }
    assert!(
        flat,
        "time to row on a month under 1,024 bars costs {} the indexed month's p99",
        harness::times(permille(tails[0].quiet_p99_ps, tails[2].quiet_p99_ps))
    );
}

/// FINDING O1P99-F-STORE-02 — a one-bar append's p99 grows with the month.
///
/// Claim: C-07, "Sealing one block costs the same at 1x, 10x and 100x the
/// file's record count", and `CLAUDE.md` section 3 rule 4's constant
/// per-operation cost. C-07 times the seal step alone. The whole
/// `BarFile::append` also keeps the `.srt` sort index, and
/// `crates/store/src/sort_index_writer.rs` (module doc, "Cost") rewrites the
/// month WHOLE, O(n log n), whenever the next run would reach
/// `sort_index::COMPACT_AT` (32) runs. One-bar appends onto a 10^6-bar month
/// therefore stall for about half a second once every 32 appends: measured
/// quietest-round p99 534 ms against 0.68 ms on a 1-bar month (787x).
#[test]
#[ignore = "o1p99 finding O1P99-F-STORE-02: fails today, every 32nd append rewrites the sort index whole"]
fn finding_store_one_bar_append_p99_grows_with_the_month() {
    const SIZES: [u64; 2] = [1, 1_000_000];
    let clock = harness::clock_ps();
    let mut writers: Vec<(BarFile, PathBuf, u64)> = SIZES
        .iter()
        .map(|&n| {
            let (w, root) = loaded("append", n);
            (w, root, n)
        })
        .collect();
    // 64 appends a round, so every round crosses at least one compaction.
    let tails = measure(
        &mut writers,
        4,
        64,
        1,
        clock,
        |_| {},
        |(w, _, next), _| {
            let b = bar(*next);
            *next += 1;
            u64::from(w.append(&[b]).is_ok())
        },
    );
    let flat = verdict(
        "O1P99-F-STORE-02",
        "append one bar: 1 bar held -> 10^6 bars held",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    );
    for (w, root, _) in writers {
        drop(w);
        let _ignored = std::fs::remove_dir_all(root);
    }
    assert!(
        flat,
        "a one-bar append onto a 10^6-bar month has p99 {} the 1-bar month's",
        harness::times(permille(tails[0].quiet_p99_ps, tails[1].quiet_p99_ps))
    );
}
