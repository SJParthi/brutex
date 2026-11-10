//! The o1p99 attack on `crates/vocab` (cloud lane `cloud/o1p99-attack-20261110`).
//!
//! `benches/ratio.rs` times `ConditionMask::hits` as the minimum of a few
//! trials. A minimum discards exactly the samples a p99 is made of, so this
//! file re-times the two lookups the sweep and the grammar make per token and
//! per bar, per call, with p50 and p99 printed, and fails when the
//! quietest-round p99 at the largest size exceeds the smallest's by more than
//! 3.0x (the shared-CI ceiling). See `o1p99/harness.rs` and
//! `O1-ATTACK-REPORT.md` at the repository root.
//!
//! * `O1P99-VOCAB-01` `table::index_of` (condition lookup), flat in the token's
//!   length: 1, 10^2, 10^4 and 10^6 bytes, each starting like a real name.
//! * `O1P99-VOCAB-02` `ConditionMask::hits` (mask evaluation), flat in what the
//!   candidate requires: k = 1, 100 and 384 set positions, the last being the
//!   whole representation. 384 is the largest k a mask can carry, so this row
//!   has no 10^4 or 10^6 size.

use std::hint::black_box;

#[allow(
    dead_code,
    reason = "the harness is copied into every crate's o1p99 bench and not every bench calls every helper"
)]
#[path = "o1p99/harness.rs"]
mod harness;

use harness::{FACTOR_PERMILLE, measure, verdict};
use vocab::ConditionMask;

/// Rounds per size.
const ROUNDS: usize = 5;
/// Samples per round per size.
const PER_ROUND: usize = 2_000;

/// `O1P99-VOCAB-01`.
fn index_of_is_flat_in_token_length(clock: u128) -> bool {
    const SIZES: [u64; 4] = [1, 100, 10_000, 1_000_000];
    let first = vocab::table::name(0).unwrap_or("x");
    let mut tokens: Vec<String> = SIZES
        .iter()
        .map(|&n| {
            let n = usize::try_from(n).unwrap_or(usize::MAX);
            let mut s: String = first.chars().take(n).collect();
            while s.len() < n {
                s.push('x');
            }
            s
        })
        .collect();
    let tails = measure(
        &mut tokens,
        ROUNDS,
        PER_ROUND,
        16,
        clock,
        |_| {},
        |t, _| u64::from(vocab::table::index_of(black_box(t.as_str())).unwrap_or(u16::MAX)),
    );
    verdict(
        "O1P99-VOCAB-01",
        "table::index_of by token bytes",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    )
}

/// `O1P99-VOCAB-02`.
fn hits_is_flat_in_candidate_width(clock: u128) -> bool {
    const SIZES: [u64; 3] = [1, 100, 384];
    let mut all = ConditionMask::ZERO;
    for b in 0..ConditionMask::BITS {
        all = all.with_bit(b);
    }
    let mut pairs: Vec<(ConditionMask, ConditionMask)> = SIZES
        .iter()
        .map(|&k| {
            let mut cand = ConditionMask::ZERO;
            for b in 0..u32::try_from(k).unwrap_or(0) {
                cand = cand.with_bit(b);
            }
            (all, cand)
        })
        .collect();
    // 64 calls a sample: one `hits` is about a nanosecond, under the clock.
    let tails = measure(
        &mut pairs,
        ROUNDS,
        PER_ROUND,
        64,
        clock,
        |_| {},
        |(bar, cand), _| u64::from(black_box(&*bar).hits(black_box(&*cand))),
    );
    verdict(
        "O1P99-VOCAB-02",
        "ConditionMask::hits by candidate popcount",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    )
}

fn main() {
    let clock = harness::clock_ps();
    println!(
        "o1p99 attack — crates/vocab, clock {} ns a pair",
        harness::ns(clock)
    );
    let mut ok = true;
    ok &= index_of_is_flat_in_token_length(clock);
    ok &= hits_is_flat_in_candidate_width(clock);
    if ok {
        println!("every gated o1p99 row held its ceiling");
    } else {
        println!("AN o1p99 ROW GREW PAST ITS CEILING — see the VERDICT lines marked GROWS.");
        std::process::exit(1);
    }
}
