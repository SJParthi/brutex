//! The o1p99 attack on `crates/engine` (cloud lane `cloud/o1p99-attack-20261110`).
//!
//! `benches/ratio.rs` times C-E-01, C-E-10 and C-E-11 at 10^3 to 10^5 (10^4 to
//! 10^6 bars for C-E-01) and keeps the minimum, or the mean of a long loop,
//! which hides the tail. This file re-times the same production primitives
//! per call at 1, 10^2, 10^4 and 10^6, prints p50 and p99, and fails when the
//! quietest-round p99 at the largest size exceeds the smallest's by more than
//! 3.0x (the shared-CI ceiling). See `o1p99/harness.rs` and
//! `O1-ATTACK-REPORT.md` at the repository root.
//!
//! * `O1P99-ENGINE-01` `Column::support`, cost PER BAR, flat in column length,
//!   gated in floors (a fold of the same rows) so memory bandwidth cancels.
//!   One call is O(bars) by definition; the claim is the per-bar constant.
//! * `O1P99-ENGINE-02` `primitives::offer` re-offering an already offered
//!   position (k=1 duplicate rejection), flat in how many were offered. The
//!   re-offered position walks 64 offered ones rather than repeating one.
//! * `O1P99-ENGINE-03` `primitives::append` into a vector reserved past it,
//!   flat in how many results are held, 64 pushes a sample.

use std::collections::HashSet;
use std::hint::black_box;

#[allow(
    dead_code,
    reason = "the harness is copied into every crate's o1p99 bench and not every bench calls every helper"
)]
#[path = "o1p99/harness.rs"]
mod harness;

use engine::Itemset;
use engine::column::Column;
use harness::{FACTOR_PERMILLE, measure, verdict};
use vocab::ConditionMask;

/// The four sizes every row is read at.
const SIZES: [u64; 4] = [1, 100, 10_000, 1_000_000];

/// Positions the synthetic bars draw from, as `benches/ratio.rs` does.
const DRAWN_FROM: [u32; 8] = [3, 17, 70, 101, 150, 199, 210, 233];

/// `n` bars, each a deterministic subset of [`DRAWN_FROM`].
fn rows(n: u64) -> Vec<ConditionMask> {
    (0..n)
        .map(|i| {
            let mut m = ConditionMask::ZERO;
            for (slot, bit) in (0_u64..).zip(DRAWN_FROM) {
                if (i / (slot + 1) + i * 7) % 3 != 0 {
                    m = m.with_bit(bit);
                }
            }
            m
        })
        .collect()
}

/// `O1P99-ENGINE-01`.
///
/// Per bar, against a floor that streams the same rows: a fold of every
/// row's six words, the least any per-bar scan of `n` rows can cost. 10^6
/// bars are 48 MB and stream from memory, which both legs pay; the gate is on
/// the cost in floors. The raw per-bar figures are printed first.
fn support_per_bar_is_flat_in_column_length(clock: u128) -> bool {
    // Batched so one sample is well over the clock at every size.
    const BATCHES: [u32; 4] = [512, 16, 1, 1];
    let mut cand = ConditionMask::ZERO;
    for bit in DRAWN_FROM.iter().take(3) {
        cand = cand.with_bit(*bit);
    }
    let mut columns: Vec<(Column, Vec<ConditionMask>)> = SIZES
        .iter()
        .map(|&n| {
            let r = rows(n);
            (Column::from_rows(&r), r)
        })
        .collect();
    // A 10^6-bar call is about a millisecond, so fewer samples: 5 x 200.
    let raw = harness::measure_each(
        &mut columns,
        5,
        200,
        &BATCHES,
        clock,
        |_| {},
        |(c, _), _| c.support(black_box(&cand)),
    );
    let floor = harness::measure_each(
        &mut columns,
        5,
        200,
        &BATCHES,
        clock,
        |_| {},
        |(_, r), _| {
            black_box(&*r)
                .iter()
                .fold(0_u64, |acc, m| m.words().iter().fold(acc, |a, w| a ^ w))
        },
    );
    let per_bar = harness::per_unit(&raw, &SIZES);
    let floor_per_bar = harness::per_unit(&floor, &SIZES);
    let _printed = verdict(
        "O1P99-ENGINE-01",
        "Column::support per bar by bars (raw, printed)",
        &SIZES,
        &per_bar,
        u128::MAX,
    );
    let _printed = verdict(
        "O1P99-ENGINE-01",
        "the floor: a fold of the same rows, per bar (printed)",
        &SIZES,
        &floor_per_bar,
        u128::MAX,
    );
    harness::verdict_floors(
        "O1P99-ENGINE-01",
        "Column::support per bar in floors by bars",
        &SIZES,
        &harness::against(&per_bar, &floor_per_bar),
        FACTOR_PERMILLE,
    )
}

/// `O1P99-ENGINE-02`.
fn duplicate_rejection_is_flat_in_offered(clock: u128) -> bool {
    let mut sets: Vec<(HashSet<u32>, Vec<u32>)> = SIZES
        .iter()
        .map(|&n| {
            let width = usize::try_from(n).unwrap_or(0);
            let mut set = HashSet::with_capacity(width);
            for p in 0..u32::try_from(n).unwrap_or(0) {
                engine::primitives::offer(&mut set, p);
            }
            let mut r = harness::Rng::new(n);
            let hot: Vec<u32> = (0..64)
                .map(|_| u32::try_from(r.below(n)).unwrap_or(0))
                .collect();
            (set, hot)
        })
        .collect();
    let tails = measure(
        &mut sets,
        5,
        2_000,
        16,
        clock,
        |_| {},
        |(set, hot), i| {
            let at = usize::try_from(i % 64).unwrap_or(0);
            let p = hot.get(at).copied().unwrap_or(0);
            u64::from(engine::primitives::offer(black_box(set), black_box(p)))
        },
    );
    verdict(
        "O1P99-ENGINE-02",
        "primitives::offer (repeat) by offered",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    )
}

/// `O1P99-ENGINE-03`.
fn append_is_flat_in_held(clock: u128) -> bool {
    let one = Itemset {
        mask: ConditionMask::ZERO.with_bit(3),
        hits: 1,
    };
    let mut held: Vec<(Vec<Itemset>, usize)> = SIZES
        .iter()
        .map(|&n| {
            let n = usize::try_from(n).unwrap_or(0);
            let mut v = Vec::with_capacity(n + 64);
            v.resize(n.saturating_sub(1), one);
            (v, n.saturating_sub(1))
        })
        .collect();
    // 64 pushes a sample: one push is under a nanosecond, far under the clock.
    // The held count moves from n-1 to n+63 inside a sample, and is reset
    // outside the timer before the next one.
    let tails = measure(
        &mut held,
        5,
        2_000,
        64,
        clock,
        |(v, at)| v.truncate(*at),
        |(v, _), _| {
            engine::primitives::append(black_box(v), black_box(one));
            u64::try_from(v.len()).unwrap_or(0)
        },
    );
    verdict(
        "O1P99-ENGINE-03",
        "primitives::append (reserved) by held",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    )
}

/// A distinct mask for every `i`: two words carry `i`, so no two collide.
fn mask_of(i: u64) -> ConditionMask {
    ConditionMask::from_words([i, i.rotate_left(29) ^ 0x5bd1_e995, 0, 0, 0, 1])
}

/// `O1P99-ENGINE-04` (gated) and `O1P99-ENGINE-05` (printed).
///
/// `keep::Best::offer` says "O(1) to refuse" and that its mask set "never
/// grows past its `cap + 1` reservation". A refusal is an insert into the
/// set and a remove from it. ENGINE-04 times one refused offer into a full
/// retention of cap 1, 10^2, 10^4 and 10^5 (gated on p99). ENGINE-05 makes
/// 10^6 refused offers into the full 10^5 retention one after another and
/// prints the slowest and how many cost over 100 us: inserts and removes in
/// a set kept about three quarters full leave deleted slots that only a
/// rehash of the whole set clears, so the steady state is not where the
/// tail is.
fn refused_offer_is_flat(clock: u128) -> bool {
    const CAPS: [u64; 4] = [1, 100, 10_000, 100_000];
    let mut held: Vec<(engine::keep::Best, u64)> = CAPS
        .iter()
        .map(|&cap| {
            let Ok(mut best) =
                engine::keep::Best::try_with_capacity(usize::try_from(cap).unwrap_or(0))
            else {
                println!("BENCH SETUP FAILED — a retention of {cap}");
                std::process::exit(1)
            };
            for i in 0..cap {
                best.offer(Itemset {
                    mask: mask_of(i),
                    hits: 1_000_000 + i,
                });
            }
            (best, cap)
        })
        .collect();
    let tails = measure(
        &mut held,
        5,
        2_000,
        1,
        clock,
        |_| {},
        |(best, next), _| {
            *next = next.saturating_add(1);
            best.offer(black_box(Itemset {
                mask: mask_of(*next),
                hits: 0,
            }));
            best.discarded()
        },
    );
    let _printed = verdict(
        "O1P99-ENGINE-04",
        "keep::Best::offer refused, by cap (raw, printed)",
        &CAPS,
        &tails,
        u128::MAX,
    );
    // The refused mask lands in a random slot of a set of `cap` masks: the
    // memory floor is one random load in the bytes the set and the heap
    // hold (a 48-byte mask in the set, a 56-byte itemset in the heap). The
    // gate is on the cost in those floors.
    let bytes: Vec<u64> = CAPS.iter().map(|c| c.saturating_mul(104)).collect();
    let floor = harness::memory_floor(&bytes, 5, 2_000, clock);
    let _printed = verdict(
        "O1P99-ENGINE-04",
        "memory floor at the set's bytes (printed)",
        &CAPS,
        &floor,
        u128::MAX,
    );
    let ok = harness::verdict_floors(
        "O1P99-ENGINE-04",
        "keep::Best::offer refused, in floors, by cap",
        &CAPS,
        &harness::against(&tails, &floor),
        FACTOR_PERMILLE,
    );
    // ENGINE-05: the long churn on the largest retention, printed only.
    if let Some((best, next)) = held.last_mut() {
        let mut over = 0_u64;
        let mut slowest = 0_u128;
        let mut ns: Vec<u128> = Vec::with_capacity(1_000_000);
        for _ in 0..1_000_000 {
            *next = next.saturating_add(1);
            let start = std::time::Instant::now();
            best.offer(black_box(Itemset {
                mask: mask_of(*next),
                hits: 0,
            }));
            let took = start.elapsed().as_nanos();
            ns.push(took);
            slowest = slowest.max(took);
            if took > 100_000 {
                over += 1;
            }
        }
        ns.sort_unstable();
        let at = |q: usize| {
            ns.get(ns.len().saturating_mul(q) / 1_000)
                .copied()
                .unwrap_or(0)
        };
        println!(
            "O1P99|O1P99-ENGINE-05|keep::Best::offer refused x 10^6 at cap 10^5|p50={} ns|p99={} ns|p999={} ns|max={slowest} ns|calls over 100 us: {over}|held {} discarded {}",
            at(500),
            at(990),
            at(999),
            best.len(),
            best.discarded()
        );
    }
    ok
}

fn main() {
    let clock = harness::clock_ps();
    println!(
        "o1p99 attack — crates/engine, clock {} ns a pair",
        harness::ns(clock)
    );
    let mut ok = true;
    ok &= support_per_bar_is_flat_in_column_length(clock);
    ok &= duplicate_rejection_is_flat_in_offered(clock);
    ok &= append_is_flat_in_held(clock);
    ok &= refused_offer_is_flat(clock);
    if ok {
        println!("every gated o1p99 row held its ceiling");
    } else {
        println!("AN o1p99 ROW GREW PAST ITS CEILING — see the VERDICT lines marked GROWS.");
        std::process::exit(1);
    }
}
