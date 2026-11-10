//! Findings of the o1p99 attack on `crates/costs` (cloud lane
//! `cloud/o1p99-attack-20261110`, 10 Nov 2026). `O1-ATTACK-REPORT.md` at the
//! repository root has the measurements, the likely cause and a proposed fix.
//!
//! EVERY TEST HERE IS `#[ignore]`d BECAUSE IT FAILS on the code it was
//! written against. Run by hand, in the optimised test profile:
//!
//! ```text
//! cargo test --locked -p costs --test o1p99_findings -- --ignored --nocapture
//! ```
//!
//! Un-ignore a test when its fix lands; it is then the regression guard.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[allow(
    dead_code,
    reason = "the bench harness is shared with the findings and not every helper is called here"
)]
#[path = "../benches/o1p99/harness.rs"]
mod harness;

use costs::day::TradeDay;
use costs::lot::{LotTable, MAX_LOT_TEXT_BYTES, compiled_rows};
use harness::{FACTOR_PERMILLE, measure, permille, verdict};

/// FINDING O1P99-F-COSTS-01 — `LotTable::lot_on` costs the length of the
/// caller's key.
///
/// Claim: `crates/costs/src/lot.rs`, `LotTable::lot_on`, "# Cost: One hash
/// probe and one binary search of at most `MAX_LOT_ROWS_PER_UNDERLYING`
/// rows: bounded, whatever the table holds". The probe (`rows_of`) hashes
/// the whole `underlying` with the map's keyed hash, and a miss copies it into
/// `LotAbsence::NoRow` (`underlying.to_owned()`), before anything compares
/// its length with `MAX_LOT_TEXT_BYTES` (2,048), the longest underlying a
/// row may carry. `core::universe::MemberIndex::position` refuses an
/// over-long key before hashing for exactly this reason (D-0033 class).
/// Measured on the attack host: quietest-round p99 62 ns at a 1-byte key,
/// 2.55 us at 10^4 bytes and 500 us at 10^6 (8,063x).
#[test]
#[ignore = "o1p99 finding O1P99-F-COSTS-01: fails today, lot_on hashes and copies an unbounded key"]
fn finding_costs_lot_on_costs_the_length_of_its_key() {
    let table = LotTable::from_rows(compiled_rows()).unwrap();
    let day = TradeDay::new(2024, 12, 10).unwrap();
    // The longest key a row may hold, then a key 500 times past it.
    let sizes: [u64; 2] = [u64::try_from(MAX_LOT_TEXT_BYTES).unwrap(), 1_000_000];
    let clock = harness::clock_ps();
    let mut keys: Vec<String> = sizes
        .iter()
        .map(|&n| "X".repeat(usize::try_from(n).unwrap()))
        .collect();
    let tails = measure(
        &mut keys,
        5,
        400,
        1,
        clock,
        |_| {},
        |k, _| u64::from(table.lot_on(k.as_str(), day).is_ok()),
    );
    let flat = verdict(
        "O1P99-F-COSTS-01",
        "LotTable::lot_on miss: 2,048-byte key -> 10^6-byte key",
        &sizes,
        &tails,
        FACTOR_PERMILLE,
    );
    assert!(
        flat,
        "a 10^6-byte key costs {} what the longest key a row may hold costs",
        harness::times(permille(tails[0].quiet_p99_ps, tails[1].quiet_p99_ps))
    );
}
