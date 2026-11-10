//! Findings of the o1p99 attack on `crates/cli` (cloud lane
//! `cloud/o1p99-attack-20261110`, 10 Nov 2026). `O1-ATTACK-REPORT.md` at the
//! repository root has the measurements, the likely cause and a proposed fix.
//!
//! EVERY TEST HERE IS `#[ignore]`d BECAUSE IT FAILS on the code it was
//! written against: it asserts the flat p99 shape a claim states and the
//! code does not have. Run by hand, in the optimised test profile:
//!
//! ```text
//! cargo test --locked -p cli --test o1p99_findings -- --ignored --nocapture
//! ```
//!
//! Un-ignore a test when its fix lands; it is then the regression guard.
//! p99 is read as `benches/o1p99.rs` reads it (`benches/o1p99/harness.rs`).

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

use cli::results::{Record, Results, field};
use harness::{FACTOR_PERMILLE, measure, permille, verdict};

/// The record `benches/ratio.rs` writes, identity `n`.
fn record(n: u64) -> Record {
    let mut identity = [0_u8; 32];
    identity[..8].copy_from_slice(&n.to_le_bytes());
    Record {
        identity,
        finished_micros: 1_787_509_605_880_000,
        mask_words: [0; 6],
        feed: field("zerodha"),
        underlying: field("NIFTY"),
        timeframe: field("15min"),
        from_year: 2019,
        from_month: 12,
        to_year: 2026,
        to_month: 8,
        months_asked: 81,
        months_found: 81,
        bars: 41_572,
        min_hits: 8_314,
        combinations: 1_024_058,
        depth: 16,
        halted: 0,
        trades: 11_209,
        pessimistic: 2_459_160,
        optimistic: 3_649_640,
        worst_trade: -4_694,
        max_drawdown: 29_163,
        winner_mae: 300,
        winner_mfe: 700,
        all_mae: 350,
        exit_rungs: [0; 5],
    }
}

/// FINDING O1P99-F-CLI-01 — `Results::refresh` after another handle appended
/// ONE record costs the whole ledger.
///
/// Claim: `crates/cli/src/results.rs`, `Results::refresh`: "O(new rows),
/// with constant metadata checks when unchanged" (the unchanged half holds:
/// `benches/o1p99.rs` O1P99-CLI-03). When the file grew, `absorb_new_records`
/// re-reads and re-hashes every byte it had indexed before it decodes the
/// new tail (`self.prefix.require_unchanged`, `result_set.rs`), so one new
/// record costs O(ledger bytes). `docs/06-limits.md` (D-1560) admits that
/// cost; the doc on `refresh` and the header of `results.rs` still say
/// O(new rows). Measured on the attack host: quietest-round p99 70.7 us at
/// 1 record, 8.64 ms at 10^4 (122x), about 0.86 us per record held.
#[test]
#[ignore = "o1p99 finding O1P99-F-CLI-01: fails today, refresh after a foreign append re-hashes the whole ledger"]
fn finding_cli_refresh_after_a_foreign_append_costs_the_whole_ledger() {
    const SIZES: [u64; 2] = [1, 100_000];
    let clock = harness::clock_ps();
    let mut held: Vec<(Results, Results, PathBuf, u64)> = SIZES
        .iter()
        .map(|&n| {
            let root = std::env::temp_dir().join(format!(
                "brutex-o1p99-finding-cli-{n}-{}",
                std::process::id()
            ));
            let _ignored = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            let mut writer = Results::open(&root).unwrap();
            for i in 0..n {
                writer.append(&record(i)).unwrap();
            }
            let reader = Results::open_read(&root).unwrap();
            (writer, reader, root, n)
        })
        .collect();
    let tails = measure(
        &mut held,
        4,
        50,
        1,
        clock,
        |(writer, _, _, next)| {
            writer.append(&record(*next)).unwrap();
            *next += 1;
        },
        |(_, reader, _, _), _| u64::from(reader.refresh().is_ok()),
    );
    let flat = verdict(
        "O1P99-F-CLI-01",
        "Results::refresh after one foreign append: 1 record -> 10^5 records",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    );
    for (w, r, root, _) in held {
        drop((w, r));
        let _ignored = std::fs::remove_dir_all(root);
    }
    assert!(
        flat,
        "refreshing one new record costs {} at 10^5 records what it costs at 1",
        harness::times(permille(tails[0].quiet_p99_ps, tails[1].quiet_p99_ps))
    );
}
