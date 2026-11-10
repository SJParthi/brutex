//! The o1p99 attack on `crates/core` (cloud lane `cloud/o1p99-attack-20261110`).
//!
//! `benches/ratio.rs` times each claim as the MINIMUM of a few trials at a
//! few sizes. A minimum discards exactly the samples a p99 is made of, so this
//! file re-times the same public functions per call, reads p50 and p99 at
//! 1, 10^2, 10^4 and 10^6 of the size each claim says it is flat in, and fails
//! when the quietest-round p99 at the largest size exceeds the smallest's by
//! more than the factor each row states (3.0x, the shared-CI ceiling, unless
//! the row says otherwise). See `o1p99/harness.rs` for how p99 is read and
//! `O1-ATTACK-REPORT.md` at the repository root for the results.
//!
//! Rows (each prints `O1P99|<id>|...` lines and one `VERDICT` line):
//!
//! * `O1P99-CORE-01` `vendor::decode_master_row`, flat in the width of one
//!   vendor field (C-09's claim, at four widths and per call).
//! * `O1P99-CORE-02` `universe::of_equity`, flat in the length of its
//!   argument (the `SYMBOL_CAPACITY` guard in `MemberIndex::position`).
//! * `O1P99-CORE-03` `symbol::Symbol::new`, flat in the length of its argument.
//! * `O1P99-CORE-04` `universe::FNO_INDEX.position`, the worst miss an
//!   adversary can choose against the best one: printed, never gated, because
//!   the claim is a probe BOUND (`<= 12`), not equal cost for every key.

use std::hint::black_box;

#[allow(
    dead_code,
    reason = "the harness is copied into every crate's o1p99 bench and not every bench calls every helper"
)]
#[path = "o1p99/harness.rs"]
mod harness;

use brutex_core::symbol::Symbol;
use brutex_core::universe::{FNO_INDEX, FNO_UNDERLYINGS, fnv1a, of_equity};
use brutex_core::vendor::{MasterRow, Vendor, decode_master_row};
use harness::{FACTOR_PERMILLE, Rng, measure, verdict};

/// The four sizes every row is read at.
const SIZES: [u64; 4] = [1, 100, 10_000, 1_000_000];

/// Rounds and samples per round per size: 5 x 2,000 = 10,000 samples a size.
const ROUNDS: usize = 5;
/// Samples per round per size.
const PER_ROUND: usize = 2_000;

/// A string of `n` bytes that starts like a real symbol and runs on in `X`.
fn token(prefix: &str, n: u64) -> String {
    let n = usize::try_from(n).unwrap_or(usize::MAX);
    let mut s: String = prefix.chars().take(n).collect();
    while s.len() < n {
        s.push('X');
    }
    s
}

/// The C-09 row with `trading_symbol` replaced.
fn row(trading_symbol: &str) -> MasterRow<'_> {
    MasterRow {
        vendor_id: "1333",
        exchange: "NSE",
        segment: "CASH",
        underlying: "RELIANCE",
        trading_symbol,
        instrument_type: "EQ",
        listing_class: "EQ",
        isin: "INE002A01018",
        expiry: "",
        strike_rupees: "",
        option_side: "",
    }
}

/// `O1P99-CORE-01`.
fn decode_is_flat_in_field_width(clock: u128) -> bool {
    let mut fields: Vec<String> = SIZES
        .iter()
        .map(|&n| token("NIFTYNXT50-Aug2026-101500-CE", n))
        .collect();
    let tails = measure(
        &mut fields,
        ROUNDS,
        PER_ROUND,
        16,
        clock,
        |_| {},
        |f, _| u64::from(decode_master_row(Vendor::Groww, black_box(row(f))).is_ok()),
    );
    verdict(
        "O1P99-CORE-01",
        "decode_master_row by trading_symbol bytes",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    )
}

/// `O1P99-CORE-02`.
fn of_equity_is_flat_in_argument_length(clock: u128) -> bool {
    let mut args: Vec<String> = SIZES.iter().map(|&n| token("RELIANCE", n)).collect();
    let tails = measure(
        &mut args,
        ROUNDS,
        PER_ROUND,
        16,
        clock,
        |_| {},
        |a, _| u64::from(of_equity(black_box(a.as_str())).bits()),
    );
    verdict(
        "O1P99-CORE-02",
        "of_equity by argument bytes",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    )
}

/// `O1P99-CORE-03`.
fn symbol_new_is_flat_in_argument_length(clock: u128) -> bool {
    let mut args: Vec<String> = SIZES.iter().map(|&n| token("RELIANCE", n)).collect();
    let tails = measure(
        &mut args,
        ROUNDS,
        PER_ROUND,
        16,
        clock,
        |_| {},
        |a, _| u64::from(Symbol::new(black_box(a.as_str())).is_ok()),
    );
    verdict(
        "O1P99-CORE-03",
        "Symbol::new by argument bytes",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    )
}

/// The `FNO_INDEX` layout rebuilt from the public member list and the public
/// hash: 1,024 slots, linear probing, members inserted in list order, exactly
/// as `MemberIndex::build` documents it. Returns the occupied flags.
fn fno_layout() -> Vec<bool> {
    const SLOTS: usize = 1024;
    let mut used = vec![false; SLOTS];
    for m in FNO_UNDERLYINGS {
        let mut at = usize::try_from(fnv1a(m) & (SLOTS as u64 - 1)).unwrap_or(0);
        while used.get(at).copied().unwrap_or(false) {
            at = (at + 1) & (SLOTS - 1);
        }
        if let Some(slot) = used.get_mut(at) {
            *slot = true;
        }
    }
    used
}

/// A valid-looking 8-byte token no table holds whose home slot is `home`.
fn miss_token_homed_at(home: usize) -> Option<String> {
    let mut r = Rng::new(0x0c0e_0400);
    for _ in 0..1_000_000 {
        let n = r.next_u64();
        let t = format!("ZQ{:06}", n % 1_000_000);
        if usize::try_from(fnv1a(&t) & 1023).ok() == Some(home)
            && !FNO_UNDERLYINGS.contains(&t.as_str())
        {
            return Some(t);
        }
    }
    None
}

/// `O1P99-CORE-04`: printed, never gated.
fn fno_worst_miss_against_best(clock: u128) {
    let used = fno_layout();
    // The longest run of occupied slots, wrapping, and where it starts.
    let (mut best_start, mut best_len) = (0, 0);
    for start in 0..used.len() {
        let mut len = 0;
        while len < used.len()
            && used
                .get((start + len) % used.len())
                .copied()
                .unwrap_or(false)
        {
            len += 1;
        }
        if len > best_len {
            best_len = len;
            best_start = start;
        }
    }
    let empty = used.iter().position(|u| !u).unwrap_or(0);
    let (Some(worst), Some(best)) = (miss_token_homed_at(best_start), miss_token_homed_at(empty))
    else {
        println!("O1P99|O1P99-CORE-04|could not find the attack tokens -- UNMEASURED");
        return;
    };
    println!(
        "O1P99|O1P99-CORE-04|longest occupied run in FNO_INDEX: {best_len} slots from slot {best_start}; worst miss token {worst:?} probes {} slots, best miss token {best:?} probes 1",
        best_len + 1
    );
    let mut keys = vec![best, worst];
    let labels = [1_u64, u64::try_from(best_len + 1).unwrap_or(0)];
    let tails = measure(
        &mut keys,
        ROUNDS,
        PER_ROUND,
        16,
        clock,
        |_| {},
        |k, _| u64::from(FNO_INDEX.position(black_box(k.as_str())).is_some()),
    );
    // Sizes here are probe counts, not a population: the claim is a bound.
    let _bounded = verdict(
        "O1P99-CORE-04",
        "FNO_INDEX miss: 1 probe -> worst run (printed, not gated)",
        &labels,
        &tails,
        u128::MAX,
    );
}

fn main() {
    let clock = harness::clock_ps();
    println!(
        "o1p99 attack — crates/core, clock {} ns a pair",
        harness::ns(clock)
    );
    let mut ok = true;
    ok &= decode_is_flat_in_field_width(clock);
    ok &= of_equity_is_flat_in_argument_length(clock);
    ok &= symbol_new_is_flat_in_argument_length(clock);
    fno_worst_miss_against_best(clock);
    if ok {
        println!("every gated o1p99 row held its ceiling");
    } else {
        println!("AN o1p99 ROW GREW PAST ITS CEILING — see the VERDICT lines marked GROWS.");
        std::process::exit(1);
    }
}
